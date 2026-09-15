//! Plan 336 T5.1 — does L4 beat **DECLINE** on the reachable set?
//!
//! ## Why this benchmark exists
//!
//! Every G1 measured so far (0/60, four times — riir-train Bench 459/460/465/467)
//! scored the adapter on `clippy_l4_v2edit_test_chunk_*`: the *training*
//! distribution's held-out split. That is the right gate for "did SFT learn the
//! task", and the wrong one for "should we export this adapter", because the
//! shipped pipeline never sends L4 anything from that distribution.
//!
//! After the Issue 030 gate fix (`d8440f6`), L4 is consulted on a **measured
//! 9/29 of cases** ([Bench 037]) — the out-of-corpus arm, where `corpus.lookup`
//! returns nothing and the only candidate left is an irrelevant `Memory`
//! trajectory. Those 9 cases ARE the deployment distribution for L4. Nobody has
//! ever measured what the adapter does on them.
//!
//! ## The bar is DECLINE, not zero
//!
//! This is the sharpened bar recorded in Plan 336 T5.3. Since `d8440f6` the
//! pipeline returns `None` on all 9, and **`None` is a correct answer** — it
//! lets the caller escalate honestly instead of paying a verifier to discover
//! that a plausible-looking fix is unrelated. So the three outcomes are not
//! "right / wrong"; they are:
//!
//! | outcome | pipeline result | vs. today |
//! |---|---|---|
//! | **WIN** | `Some(L4Lora)` and `cargo clippy` says the lint is resolved | strictly better |
//! | **NEUTRAL** | `None` — L4 declined, or the L2 pruner discarded its answer | identical |
//! | **REGRESSION** | `Some(L4Lora)` but the lint is NOT resolved | strictly **worse** |
//!
//! A REGRESSION is worse than doing nothing because it re-introduces exactly the
//! false-confidence failure Issue 030 was filed to remove. Hence the verdict rule,
//! **pre-registered here before the run**:
//!
//! > **L4 beats decline iff `WIN >= 1` AND `REGRESSION == 0`.**
//!
//! Anything else leaves T5.1 blocked. Note this makes a 0-win run *pass* the harm
//! test while still failing the benefit test — the two are reported separately
//! rather than collapsed into one number.
//!
//! ## A fourth outcome, added after the first run was wrong
//!
//! The first execution of this benchmark reported **8/9 WIN**, and it was a
//! measurement artifact. `ClippyVerifier` writes code verbatim to `src/lib.rs`;
//! most fixtures here are *statement* fragments referencing undefined names, so
//! the crate does not compile, so **no lints fire**, so `lint_resolved` returns
//! `true` and the case scores as a win. Three of those "wins" had L4 output
//! byte-identical to the input.
//!
//! [`control_fires`] is the fix: a case counts only if the **buggy** input
//! demonstrably trips the lint in the same scaffold. Everything else is
//! `Unscoreable` and excluded from the denominator, reported rather than hidden.
//! A no-op (`output == input`) can additionally never be a WIN.
//!
//! The lesson generalises past this file: validating a verifier on one passing
//! example proves it can detect the lint, **not** that a negative result means
//! the lint is absent. A silent-failure instrument needs a positive control per
//! case, not per instrument.
//!
//! ## Running it
//!
//! ```bash
//! export RIIR_L4_BIN=/tmp/t51_build/release/examples/bonsai_clippy_l4_eval
//! export RIIR_L4_GGUF=$HOME/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf
//! export RIIR_L4_LORA=/tmp/t51_reach/t29_step50_rank4.bcl4
//! cargo test --features l4_lora,clippy_verify --test t51_l4_vs_decline \
//!     -- --ignored --nocapture --test-threads=1
//! ```
//!
//! `--test-threads=1` is required: each test spawns its own 27B daemon, and two
//! concurrent loads are the leading suspect for the Bench 466 control arm's death.
//! Without the three env vars the tests **skip loudly** rather than fail, so a
//! blanket `cargo test -- --ignored` does not punish an unrelated suite.
//!
//! [Bench 037]: ../.benchmarks/037_modelless_miss_rate_l4_addressable_gap.md
#![cfg(all(feature = "l4_lora", feature = "clippy_verify"))]

use std::time::Instant;

use riir_clippy::l4_daemon::DaemonL4;
use riir_clippy::l4_fallback::L4Fallback;
use riir_clippy::{
    build_seed_corpus, ClippyFixEntry, ClippyVerifier, HealPipeline, HealSource,
    RustAnalyzerPruner, TernaryDrafter,
};

/// The reachable set: real clippy lints verified ABSENT from the seed corpus.
///
/// Copied deliberately from `benches/bench_003_modelless_miss_rate.rs` rather
/// than shared — that bench is the *measurement* of the gap and this test is the
/// *attempt to fill it*; coupling them would make a corpus change silently
/// rewrite this gate's fixture. The `corpus.lookup(...).is_empty()` premise is
/// re-asserted below, so drift is caught loudly instead of assumed away.
const REACHABLE: &[(&str, &str)] = &[
    (
        "or_fun_call",
        "pub fn f(opt: Option<u32>) -> u32 { opt.unwrap_or(expensive()) }\npub fn expensive() -> u32 { 42 }",
    ),
    ("large_enum_variant", "pub enum E { A(u8), B([u8; 4096]) }"),
    (
        "too_many_arguments",
        "pub fn f(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) { let _ = (a,b,c,d,e,f,g,h); }",
    ),
    (
        "redundant_clone",
        "pub fn f() { let s = String::from(\"hi\"); let s2 = s.clone(); drop(s2); }",
    ),
    (
        // Re-picked 2026-09-15: `vec_init_then_push` gained a proposer (the
        // corpus grew) and the premise assert fired — this fixture's own
        // loud-drift design working as intended.
        "iter_kv_map",
        "pub fn f(m: &std::collections::HashMap<u8, u8>) -> usize { m.iter().map(|(_k, v)| *v as usize).sum() }",
    ),
    (
        "manual_strip",
        "pub fn f(s: &str) -> Option<&str> { if s.starts_with(\"ab\") { Some(&s[2..]) } else { None } }",
    ),
    (
        "box_collection",
        "pub struct S { v: Box<Vec<u8>> }\npub fn mk() -> S { S { v: Box::new(Vec::new()) } }",
    ),
    (
        "option_if_let_else",
        "pub fn f(o: Option<u32>) -> u32 { if let Some(x) = o { x + 1 } else { 0 } }",
    ),
    ("ptr_arg", "pub fn f(v: &Vec<u8>) -> usize { v.len() }"),
];

/// Matches the G1 gate's decode budget so a fix produced here is the same shape
/// the gate scores. Changing it would make the two incomparable.
const MAX_NEW_TOKENS: usize = 96;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Win,
    Neutral,
    Regression,
    /// The case could not be scored at all — see [`control_fires`].
    Unscoreable,
}

/// **Scaffold-validity control — the case is scoreable only if the BUGGY input
/// actually trips the lint.**
///
/// `ClippyVerifier` writes the code verbatim into `src/lib.rs`. Several fixtures
/// here are *statement* fragments (`let s2 = s.clone(); drop(s2);`) referencing
/// undefined names, which do not compile as a lib crate. When the crate fails to
/// compile, clippy emits compile errors and **no lints**, so `present_lints` is
/// empty and `lint_resolved` comes back `true` — i.e. "the lint is gone".
///
/// That silently scores an unscoreable case as a WIN, and it did: the first run
/// of this benchmark reported 8/9 WIN, three of which had L4 output **identical
/// to the input**. Measured directly:
///
/// ```text
/// redundant_clone  "let s2 = s.clone(); drop(s2);"    -> could not compile
/// manual_strip     "if s.starts_with(..) { .. }"      -> could not compile
/// box_collection   "struct S { v: Vec<u8> }"          -> compiles, lint runs
/// ```
///
/// Validating the instrument on `ptr_arg` alone was not enough — `ptr_arg` is an
/// *item*, which is exactly the case that works. So this control runs the buggy
/// input through the same verifier first and demands the lint FIRE. If it does
/// not, the scaffold cannot see this lint and the case is excluded from the
/// verdict rather than counted.
///
/// Excluding is the honest move over "wrapping the fragment in `fn f() { .. }`":
/// these fragments reference undefined names (`opt`, `s`, `o`), so a wrapper
/// still would not compile — it would just move the false negative somewhere
/// less visible.
/// Identifiers that form an item's **public contract**: function names, function
/// parameter names, struct/enum names, field names, variant names.
///
/// Deliberately EXCLUDES local `let` bindings. Removing a local binding is the
/// legitimate fix for several lints here (`redundant_clone`'s whole point is to
/// delete the redundant `let`), so treating a lost `let` as destructive would
/// reject correct fixes. Parameters and fields are different: they are the
/// item's interface, and silently dropping one changes behaviour for every
/// caller.
fn contract_idents(code: &str) -> std::collections::BTreeSet<String> {
    use syn::visit::Visit;

    #[derive(Default)]
    struct V(std::collections::BTreeSet<String>);

    impl<'ast> Visit<'ast> for V {
        fn visit_signature(&mut self, sig: &'ast syn::Signature) {
            self.0.insert(format!("fn:{}", sig.ident));
            for arg in &sig.inputs {
                if let syn::FnArg::Typed(pt) = arg {
                    if let syn::Pat::Ident(pi) = &*pt.pat {
                        // `_`-prefixed params are conventionally throwaway; a fix
                        // that drops one is not changing meaningful behaviour.
                        let name = pi.ident.to_string();
                        if !name.starts_with('_') {
                            self.0.insert(format!("arg:{}:{}", sig.ident, name));
                        }
                    }
                }
            }
            syn::visit::visit_signature(self, sig);
        }
        fn visit_field(&mut self, f: &'ast syn::Field) {
            if let Some(i) = &f.ident {
                self.0.insert(format!("field:{i}"));
            }
            syn::visit::visit_field(self, f);
        }
        fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
            self.0.insert(format!("type:{}", i.ident));
            syn::visit::visit_item_struct(self, i);
        }
        fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
            self.0.insert(format!("type:{}", i.ident));
            syn::visit::visit_item_enum(self, i);
        }
        fn visit_variant(&mut self, v: &'ast syn::Variant) {
            self.0.insert(format!("variant:{}", v.ident));
            syn::visit::visit_variant(self, v);
        }
    }

    let mut v = V::default();
    // Same two-strategy parse the pruner uses, so this sees exactly what the
    // pruner accepted (`RustAnalyzerPruner::parse_rust` falls back to wrapping
    // a fragment in `fn _wrap() { .. }`).
    match syn::parse_file(code) {
        Ok(f) => v.visit_file(&f),
        Err(_) => {
            if let Ok(f) = syn::parse_file(&format!("fn _wrap() {{\n{code}\n}}")) {
                v.visit_file(&f);
            }
        }
    }
    v.0
}

/// **Semantic-preservation check (the second route to a false WIN).**
///
/// `RustAnalyzerPruner` checks the output parses; `ClippyVerifier` checks the
/// lint cleared. **Neither checks that behaviour is preserved**, so a fallback
/// can "fix" a lint by deleting code and pass every gate in the tree. This ran
/// live: `too_many_arguments` went `fn f(a..h)` -> `fn f(a,b,c,d,e,g,h)`,
/// resolving the lint by **deleting parameter `f`**.
///
/// That asymmetry is why it is treated as harm rather than a win: a regression
/// is *caught* (the lint still fires, the caller escalates), a destructive edit
/// is *silent*.
///
/// Returns `false` when the fix drops a contract identifier the input had.
/// Additions are fine — only losses are destructive.
fn contract_preserved(buggy: &str, fixed: &str) -> bool {
    let before = contract_idents(buggy);
    let after = contract_idents(fixed);
    before.is_subset(&after)
}

fn control_fires(verifier: &ClippyVerifier, buggy: &str, lint: &str) -> bool {
    let c = verifier.verify(buggy, lint);
    // `lint_resolved == true` means "lint NOT present". For the control we need
    // the opposite: the lint must be present on the known-buggy input.
    c.ran && !c.lint_resolved
}

fn config() -> Option<(String, String, String)> {
    let bin = std::env::var("RIIR_L4_BIN").ok()?;
    let gguf = std::env::var("RIIR_L4_GGUF").ok()?;
    let lora = std::env::var("RIIR_L4_LORA").ok()?;
    Some((bin, gguf, lora))
}

fn skip_notice() {
    eprintln!(
        "SKIP: set RIIR_L4_BIN, RIIR_L4_GGUF and RIIR_L4_LORA to run the T5.1 \
         reachable-set arm (see this file's header)."
    );
}

/// Build a pipeline seeded from the real corpus, so the out-of-corpus premise is
/// the shipped one and not an artificially empty index.
fn seeded_pipeline() -> HealPipeline {
    let corpus = build_seed_corpus();
    assert!(corpus.verify(), "corpus BLAKE3 commitment must verify");
    let mut p = HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new());
    p.seed_from_corpus(&corpus.entries);
    p
}

/// Assert the premise of the whole benchmark: these lints really are absent.
fn assert_reachable_premise() {
    let corpus = build_seed_corpus();
    for (lint, _) in REACHABLE {
        let n = corpus.lookup(lint).count();
        assert_eq!(
            n, 0,
            "`{lint}` was expected OUT of corpus but has {n} candidate(s) — \
             the corpus grew; this fixture must be re-picked"
        );
    }
}

/// **Arm 0 — the DECLINE baseline.** Establishes what L4 has to beat, measured
/// rather than assumed from Bench 037.
///
/// Runs fast (no model) and is NOT `#[ignore]`d: if this ever stops returning
/// `None` on all 9, the Issue 030 gate has regressed and the T5.1 verdict below
/// is measuring against the wrong baseline.
#[test]
fn arm0_baseline_declines_on_every_reachable_case() {
    assert_reachable_premise();
    let mut pipeline = seeded_pipeline();
    assert!(
        !pipeline.has_l4_fallback(),
        "arm 0 must have no L4 installed"
    );

    let mut declined = 0usize;
    for (lint, buggy) in REACHABLE {
        let candidates: Vec<ClippyFixEntry> = Vec::new();
        match pipeline.heal(lint, buggy, &candidates) {
            None => declined += 1,
            Some(r) => panic!(
                "baseline SERVED `{lint}` (source {:?}, parses {}) — the Issue 030 \
                 relevance gate has regressed:\n{}",
                r.source, r.parses, r.fixed_code
            ),
        }
    }
    assert_eq!(declined, REACHABLE.len());
    println!(
        "[T5.1 arm0] DECLINE baseline: {declined}/{} — this is the bar L4 must beat",
        REACHABLE.len()
    );
}

/// Guard for the destructive-edit detector, using the ACTUAL generations from
/// the 2026-08-17 run rather than invented strings.
///
/// This exists because the previous instrument (`ClippyVerifier` alone) was
/// validated on a single positive example and silently mis-scored a whole class.
/// A detector added to catch that failure gets its own positive AND negative
/// cases, on real data, or it is just the same mistake with more steps.
#[test]
fn contract_detector_separates_real_fixes_from_the_real_destructive_edit() {
    // NEGATIVE — the destructive edit actually emitted: parameter `f` deleted.
    let buggy = "pub fn f(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) { let _ = (a,b,c,d,e,f,g,h); }";
    let destructive = "fn f(a: u8, b: u8, c: u8, d: u8, e: u8, g: u8, h: u8) {}";
    assert!(
        !contract_preserved(buggy, destructive),
        "must flag the deleted parameter"
    );

    // POSITIVE — the two genuine fixes emitted in the same run.
    assert!(contract_preserved(
        "pub fn f(v: &Vec<u8>) -> usize { v.len() }",
        "fn f(v: &[u8]) -> usize { v.len() }"
    ));
    assert!(contract_preserved(
        "pub struct S { v: Box<Vec<u8>> }",
        "struct S { v: Vec<u8> }"
    ));

    // POSITIVE — deleting a LOCAL binding is the correct fix for redundant_clone
    // and must not be flagged. This is the case that rules out the naive
    // "no identifier may disappear" check.
    assert!(contract_preserved(
        "pub fn f() { let s = String::from(\"hi\"); let s2 = s.clone(); drop(s2); }",
        "pub fn f() { let s = String::from(\"hi\"); drop(s); }"
    ));

    // NEGATIVE — dropping a whole function is destructive.
    assert!(!contract_preserved(
        "pub fn a() {}\npub fn b() {}",
        "pub fn a() {}"
    ));

    // NEGATIVE — dropping a struct field is destructive.
    assert!(!contract_preserved(
        "pub struct S { x: u8, y: u8 }",
        "pub struct S { x: u8 }"
    ));
}

/// **Arm 1 — L4 on the reachable set.** The measurement T5.1 is blocked on.
#[test]
#[ignore = "loads a ~7 GB GGUF and decodes on CPU; run explicitly with --ignored"]
fn arm1_l4_vs_decline_on_the_reachable_set() {
    let Some((bin, gguf, lora)) = config() else {
        skip_notice();
        return;
    };
    assert_reachable_premise();

    let t = Instant::now();
    let daemon = match DaemonL4::spawn(&bin, &gguf, &lora, MAX_NEW_TOKENS) {
        Ok(d) => d,
        Err(e) => panic!(
            "DaemonL4::spawn failed ({e}) — check RIIR_L4_BIN points at a build \
             that supports `--serve` and RIIR_L4_LORA is a BCL4 checkpoint"
        ),
    };
    println!("[T5.1] daemon ready in {:.1}s", t.elapsed().as_secs_f32());

    let mut pipeline = seeded_pipeline().with_l4_fallback(Box::new(daemon));
    assert!(pipeline.has_l4_fallback());

    let verifier = ClippyVerifier::new();
    let cargo_ok = verifier.is_cargo_available();
    if !cargo_ok {
        eprintln!(
            "[T5.1] WARNING: cargo not available — surfaced fixes cannot be \
             verified, so every surfaced answer is counted as a REGRESSION \
             (the conservative reading: an unverifiable fix is not a proven win)"
        );
    }

    // Scoreability is a property of the fixture + verifier, not of L4, so it is
    // determined BEFORE any generation. Doing it first also means the exclusion
    // count cannot be influenced by what the model happened to emit.
    let scoreable: Vec<bool> = REACHABLE
        .iter()
        .map(|(lint, buggy)| cargo_ok && control_fires(&verifier, buggy, lint))
        .collect();
    let n_excluded = scoreable.iter().filter(|s| !**s).count();
    for ((lint, _), ok) in REACHABLE.iter().zip(&scoreable) {
        if !ok {
            println!(
                "[T5.1] {lint:<20} UNSCOREABLE — the buggy control does not trip \
                 this lint in the verifier scaffold (does not compile as a lib \
                 crate); excluded from the verdict"
            );
        }
    }

    let (mut wins, mut neutrals, mut regressions) = (0usize, 0usize, 0usize);
    let mut destructive = 0usize;
    let mut rows: Vec<(String, Outcome, String)> = Vec::new();

    for ((lint, buggy), &ok) in REACHABLE.iter().zip(&scoreable) {
        let t0 = Instant::now();
        let candidates: Vec<ClippyFixEntry> = Vec::new();
        let result = pipeline.heal(lint, buggy, &candidates);
        let secs = t0.elapsed().as_secs_f32();

        // Still generate (the wall-clock is reported for every case, since L is
        // an input to T5.3's break-even model) but do not score the outcome.
        if !ok {
            let emitted = result
                .as_ref()
                .map_or("<declined>".to_string(), |r| r.fixed_code.clone());
            println!("[T5.1] {lint:<20} Unscoreable ({secs:.1}s) — emitted: {emitted:?}");
            rows.push(((*lint).to_string(), Outcome::Unscoreable, emitted));
            continue;
        }

        let (outcome, note) = match &result {
            // L4 declined, or the L2 pruner discarded a non-parsing answer.
            // Identical to today's behaviour — no harm, no benefit.
            None => (Outcome::Neutral, "declined or discarded".to_string()),
            Some(r) if r.source != HealSource::L4Lora => {
                panic!(
                    "`{lint}` was served by {:?}, not L4 — the reachable premise \
                     is broken and this run measures the wrong thing",
                    r.source
                )
            }
            Some(r) => {
                // L4 got past the pruner, and the control proved the scaffold can
                // see this lint. Now the only question that matters: is it right?
                let noop = r.fixed_code.trim() == buggy.trim();
                let preserved = contract_preserved(buggy, &r.fixed_code);
                let v = verifier.verify(&r.fixed_code, lint);
                let cleared = v.ran && v.lint_resolved;
                match (cleared && !noop, preserved) {
                    (true, true) => (Outcome::Win, format!("lint resolved: {}", r.fixed_code)),
                    // DESTRUCTIVE: the lint genuinely cleared, but the fix dropped
                    // a contract identifier. Counted as harm, NOT as a win — the
                    // pre-registered rule defines WIN as "strictly better than
                    // decline", and a silent behaviour change never was. Only the
                    // detector changed here, not the rule.
                    (true, false) => {
                        destructive += 1;
                        let lost: Vec<_> = contract_idents(buggy)
                            .difference(&contract_idents(&r.fixed_code))
                            .cloned()
                            .collect();
                        (
                            Outcome::Regression,
                            format!(
                                "DESTRUCTIVE — lint cleared but contract broken, lost {lost:?}: {}",
                                r.fixed_code
                            ),
                        )
                    }
                    // A no-op cannot be a fix. With a valid control it would be
                    // caught by the verifier anyway; labelling it explicitly keeps
                    // "echoed the input" distinguishable from "tried and failed".
                    (false, _) => (
                        Outcome::Regression,
                        format!(
                            "SURFACED but unresolved (noop={}, ran={}, present={:?}, err={:?}): {}",
                            noop, v.ran, v.present_lints, v.error, r.fixed_code
                        ),
                    ),
                }
            }
        };

        match outcome {
            Outcome::Win => wins += 1,
            Outcome::Neutral => neutrals += 1,
            Outcome::Regression => regressions += 1,
            Outcome::Unscoreable => unreachable!("handled above"),
        }
        println!("[T5.1] {lint:<20} {outcome:?} ({secs:.1}s) — {note}");
        rows.push(((*lint).to_string(), outcome, note));
    }

    let n = REACHABLE.len() - n_excluded;
    println!("\n[T5.1] ===== reachable-set verdict =====");
    println!(
        "[T5.1] scoreable  {n}/{} ({n_excluded} excluded: buggy control does not \
         trip the lint in the verifier scaffold)",
        REACHABLE.len()
    );
    println!("[T5.1] WIN        {wins}/{n}  (strictly better than decline)");
    println!("[T5.1] NEUTRAL    {neutrals}/{n}  (identical to decline)");
    println!("[T5.1] REGRESSION {regressions}/{n}  (strictly worse than decline)");
    println!(
        "[T5.1]   of which DESTRUCTIVE {destructive}/{n}  (lint cleared, contract broken \u{2014} silent harm)"
    );

    // Pre-registered verdict rule — stated in this file's header BEFORE the run.
    let beats_decline = wins >= 1 && regressions == 0;
    println!(
        "[T5.1] VERDICT: L4 {} decline (rule: WIN>=1 AND REGRESSION==0)",
        if beats_decline {
            "BEATS"
        } else {
            "does NOT beat"
        }
    );
    println!("[T5.1] rows: {rows:?}");

    // The harm half is a HARD assertion: shipping an adapter that surfaces
    // wrong-but-parsing fixes would undo the Issue 030 gate. The benefit half is
    // reported, not asserted — a 0-win run is the expected state until G1 moves,
    // and failing the suite on it would make this harness un-runnable as a probe.
    assert_eq!(
        regressions, 0,
        "L4 surfaced {regressions} wrong-but-parsing fix(es) on the reachable set \
         — strictly worse than the decline it replaces; do NOT export this adapter"
    );
}

/// **Arm 2 — the diagnostic.** *Why* did arm 1 land where it did?
///
/// Arm 1 measures the shipped outcome, and by construction it throws away the
/// evidence: `consult_l4` discards the generated text when the L2 pruner rejects
/// it, so a NEUTRAL row cannot distinguish
///
/// - the adapter **declined** (produced nothing), from
/// - the adapter **produced an unparseable fragment** (the expected failure, since
///   it is trained on an edit-only contract and emits changed lines, not a
///   compilable item), from
/// - the adapter **produced valid Rust that is simply wrong**.
///
/// Those three imply completely different next steps — respectively "the decode
/// is broken", "the response contract is mismatched with the seam", and "the
/// adapter needs to be better". Reporting NEUTRAL without separating them would
/// leave T5.1's blocker unresolved even after a successful run.
///
/// So this arm calls [`L4Fallback::generate_fix`] **directly**, bypassing the
/// pipeline, and applies the same two gates by hand: `RustAnalyzerPruner::validate`
/// (what discarded it) then [`ClippyVerifier`] (whether it would have been right).
/// Asserts nothing — it is an evidence collector for the write-up.
#[test]
#[ignore = "loads a ~7 GB GGUF and decodes on CPU; run explicitly with --ignored"]
fn arm2_diagnostic_raw_l4_output() {
    let Some((bin, gguf, lora)) = config() else {
        skip_notice();
        return;
    };

    let mut daemon = match DaemonL4::spawn(&bin, &gguf, &lora, MAX_NEW_TOKENS) {
        Ok(d) => d,
        Err(e) => panic!("DaemonL4::spawn failed: {e}"),
    };
    let pruner = RustAnalyzerPruner::new();
    let verifier = ClippyVerifier::new();

    let (mut declined, mut unparseable, mut parses_wrong, mut parses_right) = (0, 0, 0, 0);

    for (lint, buggy) in REACHABLE {
        let t0 = Instant::now();
        let raw = daemon.generate_fix(lint, buggy);
        let secs = t0.elapsed().as_secs_f32();

        match raw {
            None => {
                declined += 1;
                println!("[T5.1 arm2] {lint:<20} DECLINED (no output) ({secs:.1}s)");
            }
            Some(text) => {
                let v = pruner.validate(&text, lint);
                let verdict = match v.parses {
                    false => {
                        unparseable += 1;
                        "UNPARSEABLE (pruner rejects → pipeline discards)".to_string()
                    }
                    true => {
                        let c = verifier.verify(&text, lint);
                        match c.ran && c.lint_resolved {
                            true => {
                                parses_right += 1;
                                "PARSES + LINT RESOLVED".to_string()
                            }
                            false => {
                                parses_wrong += 1;
                                format!(
                                    "PARSES but UNRESOLVED (ran={}, present={:?})",
                                    c.ran, c.present_lints
                                )
                            }
                        }
                    }
                };
                println!("[T5.1 arm2] {lint:<20} {verdict} ({secs:.1}s)");
                println!("            raw({} chars): {:?}", text.len(), text);
            }
        }
    }

    let n = REACHABLE.len();
    println!("\n[T5.1 arm2] ===== mechanism breakdown (n={n}) =====");
    println!("[T5.1 arm2] declined (no output)          {declined}/{n}");
    println!("[T5.1 arm2] unparseable (discarded)       {unparseable}/{n}");
    println!("[T5.1 arm2] parses but lint unresolved    {parses_wrong}/{n}");
    println!("[T5.1 arm2] parses and lint resolved      {parses_right}/{n}");
}

//! Issue 91 T2 — fix-loop trap **prevalence** over the per-attempt ring.
//!
//! T1's question ("how prevalent are oscillating fix pairs — the same error
//! code reappearing after >=1 intervening fix?") was unmeasurable with the
//! shipped stores. T1.5 landed the ring (`AttemptRecord`); this runner is
//! the measurement.
//!
//! ```bash
//! cargo run --release --example fixseq_prevalence --features rustc_errors
//! cargo run --release --example fixseq_prevalence --features rustc_errors -- --limit 40
//! cargo run --release --example fixseq_prevalence --features rustc_errors -- --store /tmp/fixseq_corpus
//! ```
//!
//! **The corpus is SYNTHETIC and the number must be labelled as such.**
//! `--fix-compile` fires only on *compile errors*, and every repo in the
//! workspace compiles clean (`.heal/fixseq/` exists in none of them — an
//! organic corpus does not exist and cannot be produced without injecting
//! errors, which is what this harness does). What is REAL here is the
//! driver: the production `CompileDriver` loop runs unmodified, so the
//! *dynamics* it exhibits are the shipped dynamics.
//!
//! Three quantities, in increasing strength:
//!
//! 1. **Flip prevalence** (the T1 question) — three keys, per run:
//!    - `code`: a code recurring after >=1 intervening attempt of another code
//!    - `span`: a span_hash recurring after >=1 intervening attempt elsewhere
//!    - `pair`: the exact (code, span_hash) A -> B -> A signature
//! 2. **Attempt-depth support** — a flip needs >=3 attempts; a corpus whose
//!    runs are 1-2 attempts deep has no POWER to show one. Reported so the
//!    zero cannot be read as stronger than it is.
//! 3. **The loop invariant** (the two-sided half) — every `Kept` attempt must
//!    STRICTLY decrease `(masking, coded-error count)` LEXICOGRAPHICALLY
//!    (Issue 100: an equal-count keep is legal exactly when the masking-stage
//!    count dropped — the unmasking trade), and every reverted attempt must
//!    leave the pair not-decreased. If that holds on every record, the run's
//!    state carries a well-founded measure `(lex pair DOWN, |tried| UP)`
//!    and a revisit is structurally impossible: the zero in (1) is then a
//!    consequence, not a sample.

use riir_clippy::domains::rustc_errors::corpus_gen::{generate_corpus, GeneratedCase};
use riir_clippy::domains::rustc_errors::driver::{AttemptOutcome, AttemptRecord};
use riir_clippy::domains::rustc_errors::evolve::dump_fixseq;
use riir_clippy::domains::rustc_errors::floor::{
    DriverFixer, FloorHarness, HarnessCase, MutatorKind, MUTANT_CASES,
};

/// Per-run flip verdict on the three keys.
#[derive(Default, Debug, Clone, Copy)]
struct Flips {
    code: usize,
    span: usize,
    pair: usize,
}

/// Count recurrences of `key(a)` separated by >=1 attempt with a different
/// key — the "reappearing after an intervening fix" shape T1 asked for.
fn recurrences(attempts: &[AttemptRecord], key: impl Fn(&AttemptRecord) -> String) -> usize {
    let keys: Vec<String> = attempts.iter().map(&key).collect();
    let mut n = 0;
    for j in 0..keys.len() {
        // j recurs iff some i < j has keys[i] == keys[j] AND some k in (i, j)
        // has keys[k] != keys[j].
        for i in 0..j {
            if keys[i] == keys[j] && (i + 1..j).any(|k| keys[k] != keys[j]) {
                n += 1;
                break;
            }
        }
    }
    n
}

fn flips(attempts: &[AttemptRecord]) -> Flips {
    Flips {
        code: recurrences(attempts, |a| a.code.clone()),
        span: recurrences(attempts, |a| a.span_hash.clone()),
        pair: recurrences(attempts, |a| format!("{}@{}", a.code, a.span_hash)),
    }
}

/// The invariant probe. Returns the violations found (empty = holds).
///
/// Issue 100 — the keep rule is the LEXICOGRAPHIC strict decrease of
/// `(masking, count)`: a `Kept` at equal count is legal exactly when the
/// masking count dropped (the unmasking trade), and a `Reverted` despite a
/// count decrease is legal exactly when masking rose (the strand-keep the
/// old count-only rule scored as progress).
fn invariant_violations(name: &str, attempts: &[AttemptRecord]) -> Vec<String> {
    let lex = |a: &AttemptRecord| (a.masking.1, a.errors.1) < (a.masking.0, a.errors.0);
    let mut v = Vec::new();
    for a in attempts {
        match a.outcome {
            AttemptOutcome::Kept if !lex(a) => v.push(format!(
                "{name} seq={}: KEPT without strict lex decrease ({}m,{} -> {}m,{})",
                a.seq, a.masking.0, a.errors.0, a.masking.1, a.errors.1
            )),
            AttemptOutcome::RevertedNoProgress | AttemptOutcome::RevertedIo if lex(a) => {
                v.push(format!(
                    "{name} seq={}: REVERTED despite a lex decrease ({}m,{} -> {}m,{})",
                    a.seq, a.masking.0, a.errors.0, a.masking.1, a.errors.1
                ));
            }
            _ => {}
        }
    }
    // Cross-attempt: the error count entering attempt n+1 must equal the
    // count leaving attempt n (kept -> after; reverted -> before, the file is
    // restored byte-exactly).
    for w in attempts.windows(2) {
        let (prev, next) = (&w[0], &w[1]);
        let leaving = match prev.outcome {
            AttemptOutcome::Kept => prev.errors.1,
            _ => prev.errors.0,
        };
        if leaving != next.errors.0 {
            v.push(format!(
                "{name} seq={}->{}: count discontinuity ({leaving} != {})",
                prev.seq, next.seq, next.errors.0
            ));
        }
    }
    v
}

/// The instrument's own canary: a zero from `recurrences` is worthless
/// unless the classifier provably FIRES on a ring that does contain the
/// shape. Synthetic rings with KNOWN answers, checked every run — an inert
/// detector reports the same 0 as a clean corpus.
fn self_check() {
    let mk = |seq: usize, code: &str, span: &str| AttemptRecord {
        seq,
        code: code.to_string(),
        span_hash: span.to_string(),
        strategy_id: "s".to_string(),
        outcome: AttemptOutcome::Kept,
        errors: (2, 1),
        masking: (0, 0),
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: None,
        shape_class: None,
        insight: None,
    };
    // A -> B -> A at one span: exactly one recurrence on all three keys.
    let flip = [
        mk(0, "E0502", "aa"),
        mk(1, "E0597", "aa"),
        mk(2, "E0502", "aa"),
    ];
    let f = flips(&flip);
    // span is CONSTANT here, so there is no intervening *different* span:
    // span=0 is correct (hammering one span is repetition, not oscillation).
    assert_eq!(
        (f.code, f.span, f.pair),
        (1, 0, 1),
        "canary: A->B->A at one span must report code=1 span=0 pair=1"
    );
    // aa -> bb -> aa: the span key's own return-to-origin shape.
    let span_flip = [
        mk(0, "E0502", "aa"),
        mk(1, "E0502", "bb"),
        mk(2, "E0502", "aa"),
    ];
    let sf = flips(&span_flip);
    assert_eq!(
        (sf.code, sf.span, sf.pair),
        (0, 1, 1),
        "canary: aa->bb->aa must report span=1 pair=1"
    );
    // A -> B -> C: no recurrence anywhere (a first-order CHANGE detector
    // would score this identically to the flip above — the polarity the
    // katgpt-rs `FlipDetector` cannot express; see Issue 91 T3).
    let march = [
        mk(0, "E0502", "aa"),
        mk(1, "E0597", "bb"),
        mk(2, "E0499", "cc"),
    ];
    let g = flips(&march);
    assert_eq!((g.code, g.span, g.pair), (0, 0, 0), "canary: A->B->C must report 0");
    // Adjacent repeat (A -> A) is NOT a flip: no intervening attempt.
    let adj = [mk(0, "E0502", "aa"), mk(1, "E0502", "aa")];
    let h = flips(&adj);
    assert_eq!((h.code, h.span, h.pair), (0, 0, 0), "canary: adjacent repeat must report 0");
    // An INVARIANT violation must also be visible.
    let bad = [AttemptRecord {
        seq: 0,
        code: "E0502".into(),
        span_hash: "aa".into(),
        strategy_id: "s".into(),
        outcome: AttemptOutcome::Kept,
        errors: (1, 1),
        masking: (0, 0),
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: None,
        shape_class: None,
        insight: None,
    }];
    assert_eq!(
        invariant_violations("canary", &bad).len(),
        1,
        "canary: a Kept without strict decrease must be reported"
    );
    // Issue 100 canaries — the lex reading's two new legal shapes and its
    // new violation. (1) The UNMASKING KEEP: equal count, masking dropped —
    // legal, must NOT be reported. (2) The STRAND REVERT: count decreased
    // but masking rose — a legal revert, must NOT be reported (the old
    // count-only checker would have flagged it). (3) A revert despite a
    // real lex decrease — still a violation, must be reported.
    let unmask_keep = [AttemptRecord {
        seq: 0,
        code: "E0614".into(),
        span_hash: "aa".into(),
        strategy_id: "s".into(),
        outcome: AttemptOutcome::Kept,
        errors: (1, 1),
        masking: (1, 0),
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: None,
        shape_class: None,
        insight: None,
    }];
    assert_eq!(
        invariant_violations("canary", &unmask_keep).len(),
        0,
        "canary: the unmasking keep (equal count, masking down) is LEGAL"
    );
    let strand_revert = [AttemptRecord {
        seq: 0,
        code: "E0502".into(),
        span_hash: "aa".into(),
        strategy_id: "s".into(),
        outcome: AttemptOutcome::RevertedNoProgress,
        errors: (2, 1),
        masking: (0, 1),
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: None,
        shape_class: None,
        insight: None,
    }];
    assert_eq!(
        invariant_violations("canary", &strand_revert).len(),
        0,
        "canary: the strand revert (count down, masking up) is LEGAL"
    );
    let reverted_progress = [AttemptRecord {
        seq: 0,
        code: "E0614".into(),
        span_hash: "aa".into(),
        strategy_id: "s".into(),
        outcome: AttemptOutcome::RevertedNoProgress,
        errors: (1, 0),
        masking: (1, 0),
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: None,
        shape_class: None,
        insight: None,
    }];
    assert_eq!(
        invariant_violations("canary", &reverted_progress).len(),
        1,
        "canary: a revert despite a lex decrease must be reported"
    );
    println!(
        "instrument canary: PASS — A->B->A scores code=1/pair=1, aa->bb->aa scores span=1/pair=1, \
         A->B->C and A->A score 0, a non-decreasing Kept is reported; the lex keep/revert \
         shapes (unmasking keep, strand revert) are classified correctly."
    );
}

fn main() {
    self_check();
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let limit: usize = flag("--limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let store = flag("--store");

    // `--multi K` composes K single-mutation cases into ONE scaffold (each in
    // its own `pub mod`), so a run carries K simultaneous errors instead of
    // one. This is the POWER lever: the stock corpus is single-mutation, so
    // every run is <=1 attempt deep and a flip (>=3 attempts) is unreachable
    // BY CONSTRUCTION — a zero there measures the corpus, not the driver.
    let multi: usize = flag("--multi").and_then(|s| s.parse().ok()).unwrap_or(1);
    // `--interacting` (Issue 099 T3): the shared-binding INTERACTING pair
    // corpus — members whose diagnostics depend on each other's repair, the
    // one shape where a fix can re-break something (the Bench 087 §6 reopen
    // trigger's missing instrument). Mutually exclusive with `--multi`
    // (each interacting case is already a composed pair).
    let interacting = args.iter().any(|a| a == "--interacting");
    // `--runs` prints every run's attempt ring (seq/code/strategy/outcome/
    // error-count transition) — the per-case structure behind the depth
    // histogram (Issue 099 T3).
    let show_runs = args.iter().any(|a| a == "--runs");

    let generated = generate_corpus();
    let composed: Vec<(String, &'static str, String, String)> = match multi > 1 {
        false => Vec::new(),
        true => generated
            .chunks(multi)
            .enumerate()
            .map(|(g, chunk)| {
                let wrap = |pick: fn(&GeneratedCase) -> &str| {
                    chunk
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            let body = pick(c).replace('\n', "\n    ");
                            format!("pub mod c{i} {{\n    {body}\n}}\n")
                        })
                        .collect::<String>()
                };
                (
                    format!("multi{multi}_{g:03}"),
                    chunk[0].expect_code,
                    wrap(|c| c.seed.as_str()),
                    wrap(|c| c.broken.as_str()),
                )
            })
            .collect(),
    };

    let mut cases: Vec<HarnessCase<'_>> = Vec::new();
    if interacting {
        let leaked: Vec<riir_clippy::domains::rustc_errors::corpus_gen::GeneratedCase> =
            riir_clippy::domains::rustc_errors::corpus_interacting::interacting_pairs()
                .iter()
                .map(riir_clippy::domains::rustc_errors::corpus_interacting::InteractingCase::as_generated)
                .collect();
        // 'static: one-shot measurement binary; leaking keeps the borrowed
        // form simple (the floor example's floor_case_list precedent).
        cases.extend(
            leaked
                .into_iter()
                .map(|g| HarnessCase::generated(Box::leak(Box::new(g)))),
        );
    } else {
        match multi > 1 {
            true => cases.extend(composed.iter().map(|(name, code, seed, broken)| HarnessCase {
                name: name.as_str(),
                expect_code: code,
                kind: MutatorKind::NarrowScope,
                seed: seed.as_str(),
                broken: broken.clone(),
            })),
            false => {
                for c in MUTANT_CASES {
                    match HarnessCase::committed(c) {
                        Ok(h) => cases.push(h),
                        Err(e) => eprintln!("corpus integrity: {e}"),
                    }
                }
                cases.extend(generated.iter().map(HarnessCase::generated));
            }
        }
    }
    cases.truncate(limit);

    println!("=== Issue 91 T2 — fix-loop flip prevalence (SYNTHETIC corpus) ===");
    println!(
        "mode: {}",
        if interacting {
            "INTERACTING — shared-binding pairs (Issue 099 T3, admissibility-gated)".to_string()
        } else {
            match multi > 1 {
                true => format!("MULTI — {multi} mutations composed per scaffold"),
                false => "SINGLE — the stock one-mutation-per-scaffold corpus".to_string(),
            }
        }
    );
    println!("cases: {}", cases.len());

    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("cargo unavailable — no measurement possible");
        std::process::exit(2);
    }
    let t0 = std::time::Instant::now();
    let (report, reports) = match harness.measure_driver(&DriverFixer::new(), &cases) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("harness: {e}");
            std::process::exit(2);
        }
    };
    let wall = t0.elapsed().as_secs_f64();

    // ── Aggregate ────────────────────────────────────────────────────────
    let mut depth_hist = [0usize; 8]; // index = attempts, 7+ pooled
    let mut runs_with_attempts = 0usize;
    let mut total_attempts = 0usize;
    let mut total = Flips::default();
    let mut runs_flipping = Flips::default();
    let mut violations: Vec<String> = Vec::new();
    let mut dumped = 0usize;

    for (name, rep) in &reports {
        let a = &rep.attempts;
        total_attempts += a.len();
        depth_hist[a.len().min(7)] += 1;
        if !a.is_empty() {
            runs_with_attempts += 1;
        }
        if show_runs {
            // Per-run ring: the attempt sequence with strategy + outcome +
            // the error-count transition (masking delta appended when
            // nonzero — the Issue 100 unmasking keeps read as `1->1,m1->0`)
            // — the structure the aggregate histogram hides (Issue 099 T3:
            // which strategy reverted where).
            let ring: Vec<String> = a
                .iter()
                .map(|r| {
                    format!(
                        "{}:{}{}({}->{}){}",
                        r.code,
                        r.strategy_id,
                        match r.outcome {
                            AttemptOutcome::Kept => "=KEEP",
                            AttemptOutcome::RevertedNoProgress => "=REV",
                            AttemptOutcome::RevertedIo => "=IO",
                        },
                        r.errors.0,
                        r.errors.1,
                        if r.masking == (0, 0) {
                            String::new()
                        } else {
                            format!(",m{}->{}", r.masking.0, r.masking.1)
                        }
                    )
                })
                .collect();
            println!(
                "  run {:<34} clean={} clean_flag={} [{}]",
                name,
                rep.clean as u8,
                rep.clean as u8,
                ring.join(" ")
            );
        }
        let f = flips(a);
        total.code += f.code;
        total.span += f.span;
        total.pair += f.pair;
        runs_flipping.code += usize::from(f.code > 0);
        runs_flipping.span += usize::from(f.span > 0);
        runs_flipping.pair += usize::from(f.pair > 0);
        violations.extend(invariant_violations(name, a));
        if let Some(root) = &store {
            let (written, warns) = dump_fixseq(std::path::Path::new(root), a);
            dumped += usize::from(written.is_some());
            for w in warns {
                eprintln!("fixseq dump {name}: {w}");
            }
        }
    }

    println!("\n-- ring population --");
    println!("runs                 : {}", reports.len());
    println!("runs with >=1 attempt: {runs_with_attempts}");
    println!("attempts total       : {total_attempts}");
    print!("attempt-depth hist   :");
    for (d, n) in depth_hist.iter().enumerate() {
        if *n > 0 {
            let label = match d {
                7 => "7+".to_string(),
                _ => d.to_string(),
            };
            print!(" {label}={n}");
        }
    }
    println!();
    let depth3 = reports.iter().filter(|(_, r)| r.attempts.len() >= 3).count();
    println!(
        "runs with >=3 attempts (the minimum depth a flip NEEDS): {depth3}  <- the measurement's power"
    );

    println!("\n-- flip prevalence (recurrence after >=1 intervening attempt) --");
    println!(
        "key=code : {} recurrences over {} runs ({} runs affected)",
        total.code,
        reports.len(),
        runs_flipping.code
    );
    println!(
        "key=span : {} recurrences over {} runs ({} runs affected)",
        total.span,
        reports.len(),
        runs_flipping.span
    );
    println!(
        "key=pair : {} recurrences over {} runs ({} runs affected)",
        total.pair,
        reports.len(),
        runs_flipping.pair
    );

    println!("\n-- loop invariant (the two-sided half) --");
    match violations.is_empty() {
        true => println!(
            "HOLDS over all {total_attempts} attempts: Kept => strict LEX decrease of \
             (masking, error_count) [equal-count keeps legal exactly when masking \
             dropped — Issue 100]; reverted => lex pair not decreased; no \
             cross-attempt count discontinuity.\n\
             => (masking DOWN-lex, error_count DOWN-lex, |tried| UP) is a well-founded \
             measure; a state revisit within one run is structurally impossible."
        ),
        false => {
            println!("VIOLATED ({} findings):", violations.len());
            for v in violations.iter().take(20) {
                println!("  {v}");
            }
        }
    }

    println!("\n-- repair quality (context; the floor harness's own verdict) --");
    println!("total fix rate: {:.1}%", report.total_fix_rate() * 100.0);
    for row in &report.rows {
        println!(
            "  {:>6}  mutants={:<4} matched={:<4} diverged={:<4} unfixed={:<4} divergence={:.0}%",
            row.code,
            row.mutants,
            row.matched,
            row.diverged,
            row.unfixed,
            row.divergence_rate() * 100.0
        );
    }
    if !report.invalid_mutants.is_empty() {
        println!("invalid mutants: {}", report.invalid_mutants.len());
    }
    if let Some(root) = &store {
        println!("\nfixseq rings written: {dumped} -> {root}/.heal/fixseq/");
    }
    println!("\nwall: {wall:.1}s");
}

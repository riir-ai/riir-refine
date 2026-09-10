//! Bench 003 — the modelless MISS RATE: how much work is left for L4 at all?
//!
//! riir-train Plan 336 Phase 5 (T5.1/T5.3/T5.4) proposes shipping an L4
//! generative fallback. This harness measures the only quantity that can
//! justify or retire that work: **how often the modelless path fails to serve a
//! parsing fix**, because that — and nothing else — is the set L4 is ever
//! consulted on.
//!
//! ## Why the miss set, and not the wrong-answer set
//!
//! The shipped composition seam consults L4 on exactly two conditions
//! (`src/self_evolve.rs:517` and `:574`, verified by
//! `tests/g4_l4_composition.rs`):
//!
//! - **miss #1** — `heal()` returns `None`: no candidate at all.
//! - **miss #2** — `heal()` returns `Some(r)` with `r.parses == false`: the best
//!   candidate does not parse.
//!
//! A modelless fix that **parses but is wrong** never reaches L4 — the invariant
//! `a_parsing_modelless_fix_is_never_replaced_by_l4` forbids it. So exact-match
//! accuracy is NOT L4's addressable set, and a low EM score is not evidence that
//! L4 would help. This distinction is the whole point of the measurement: if the
//! miss rate is ~0, L4 is *unreachable in production* no matter how good a
//! checkpoint someone eventually trains.
//!
//! EM against the fixture's `want` field is still reported, but only as context,
//! and it is explicitly labelled as NOT L4-addressable.
//!
//! ## Arms
//!
//! Both fixtures are measured because they answer different questions:
//! - `e2e_cases_seed.json` — in-distribution (the corpus holds the answer).
//! - `e2e_cases_heldout.json` — held-out; this is the generalization set, and the
//!   one riir-clippy Issue 010 was closed against (0/10 -> 10/10, commit
//!   `8ca3b2c`).
//!
//! No GPU, no `cargo clippy`, no model load: `heal()` is the whole measurement,
//! so this is a fast CPU-only bench that can run on a busy box.
//!
//! Run:
//! ```bash
//! cargo bench --bench bench_003_modelless_miss_rate \
//!   --features self_evolve,clippy_lints
//! ```

#![allow(clippy::pedantic)]

use riir_clippy::{
    build_seed_corpus, ClippyFixEntry, ClippyTernaryDrafter, HealPipeline, RustAnalyzerPruner,
};
use std::io::Write;

/// One fixture case. Same shape as the other e2e benches' fixtures.
#[derive(serde::Deserialize)]
struct Case {
    lint: String,
    buggy: String,
    want: String,
}

#[derive(serde::Deserialize)]
struct CaseFile {
    name: String,
    cases: Vec<Case>,
}

/// Identical to `bench_e2e_tokps.rs` / `bench_001_l1_matvec.rs` so the drafter
/// behaves the same across benches (bench-local by existing convention).
fn realistic_weights() -> katgpt_core::TernaryWeights {
    let rows = 64;
    let cols = 256;
    let mut weights = vec![0.0f32; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            weights[i * cols + j] = match (i + j) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            };
        }
    }
    katgpt_core::TernaryWeights::quantize_from_f32(&weights, rows, cols)
}

/// Normalize for exact-match: trim trailing whitespace per line, drop blank
/// lines. Mirrors riir-train's `normalize_text`
/// (`crates/riir-train-engine/examples/bonsai_clippy_l4_eval.rs:195`) so the EM
/// context column is comparable to the G1 gate's definition.
fn normalize(s: &str) -> String {
    s.lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// How the modelless path resolved one case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// `heal()` produced a parsing fix. L4 is NEVER consulted here.
    ServedParsing,
    /// miss #2 — a candidate came back but does not parse (`self_evolve.rs:574`).
    MissNonParsing,
    /// miss #1 — no candidate at all (`self_evolve.rs:517`).
    MissNoCandidate,
}

struct FixtureResult {
    name: String,
    total: usize,
    served: usize,
    miss_non_parsing: usize,
    miss_no_candidate: usize,
    em: usize,
    rows: Vec<(String, Outcome, bool)>,
}

impl FixtureResult {
    fn misses(&self) -> usize {
        self.miss_non_parsing + self.miss_no_candidate
    }

    fn miss_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.misses() as f64 / self.total as f64
    }
}

fn measure(path: &str) -> FixtureResult {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read fixture {path}: {e}"));
    let file: CaseFile =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse fixture {path}: {e}"));

    let corpus = build_seed_corpus();
    assert!(corpus.verify(), "corpus BLAKE3 commitment must verify");

    // The shipped default path: self_evolve seeded from the corpus. Stage 2 is
    // NOT tuned here (that needs the real `cargo clippy` verifier); a Stage-2
    // hit can only ever REDUCE the miss rate, so an untuned run is an UPPER
    // BOUND on the miss rate — which is the conservative direction for a
    // decision about whether L4 has any work to do.
    let weights = realistic_weights();
    let mut pipeline = HealPipeline::new(
        ClippyTernaryDrafter::with_weights(weights),
        RustAnalyzerPruner::new(),
    );
    pipeline.seed_from_corpus(&corpus.entries);

    let mut out = FixtureResult {
        name: file.name.clone(),
        total: 0,
        served: 0,
        miss_non_parsing: 0,
        miss_no_candidate: 0,
        em: 0,
        rows: Vec::new(),
    };

    for case in &file.cases {
        let candidates: Vec<ClippyFixEntry> = corpus.lookup(&case.lint).cloned().collect();
        let result = pipeline.heal(&case.lint, &case.buggy, &candidates);

        let (outcome, exact) = match &result {
            None => (Outcome::MissNoCandidate, false),
            Some(r) if !r.parses => (Outcome::MissNonParsing, false),
            Some(r) => (
                Outcome::ServedParsing,
                normalize(&r.fixed_code) == normalize(&case.want),
            ),
        };

        out.total += 1;
        match outcome {
            Outcome::ServedParsing => out.served += 1,
            Outcome::MissNonParsing => out.miss_non_parsing += 1,
            Outcome::MissNoCandidate => out.miss_no_candidate += 1,
        }
        if exact {
            out.em += 1;
        }
        out.rows.push((case.lint.clone(), outcome, exact));
    }

    out
}

/// Real clippy lints the seed corpus has **no entry** for (verified absent from
/// `src/domains/clippy_lints/corpus.rs`, which covers ~30 lints / 41 entries).
///
/// This arm exists because the two shipped fixtures use only corpus-covered
/// lints, so measuring them alone yields a 0% miss rate **by construction** —
/// `corpus.lookup(lint)` always returns candidates. Real `cargo clippy` emits
/// hundreds of distinct lints, so out-of-corpus input is the normal case in
/// production and is where miss #1 (`heal() -> None`) can actually fire.
///
/// The `want` field is deliberately absent: for this arm only the OUTCOME is
/// meaningful (does the seam reach L4?), not exact match.
const OUT_OF_CORPUS: &[(&str, &str)] = &[
    ("or_fun_call", "let x = opt.unwrap_or(expensive());"),
    ("large_enum_variant", "enum E { A(u8), B([u8; 4096]) }"),
    (
        "too_many_arguments",
        "fn f(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {}",
    ),
    ("redundant_clone", "let s2 = s.clone(); drop(s2);"),
    (
        "vec_init_then_push",
        "let mut v = Vec::new(); v.push(1); v.push(2);",
    ),
    (
        "manual_strip",
        "if s.starts_with(\"ab\") { let t = &s[2..]; }",
    ),
    ("box_collection", "struct S { v: Box<Vec<u8>> }"),
    (
        "option_if_let_else",
        "if let Some(x) = o { x + 1 } else { 0 }",
    ),
    ("ptr_arg", "fn f(v: &Vec<u8>) -> usize { v.len() }"),
];

fn measure_out_of_corpus() -> FixtureResult {
    let corpus = build_seed_corpus();
    assert!(corpus.verify(), "corpus BLAKE3 commitment must verify");
    let weights = realistic_weights();
    let mut pipeline = HealPipeline::new(
        ClippyTernaryDrafter::with_weights(weights),
        RustAnalyzerPruner::new(),
    );
    pipeline.seed_from_corpus(&corpus.entries);

    let mut out = FixtureResult {
        name: "out-of-corpus".to_string(),
        total: 0,
        served: 0,
        miss_non_parsing: 0,
        miss_no_candidate: 0,
        em: 0,
        rows: Vec::new(),
    };

    for (lint, buggy) in OUT_OF_CORPUS {
        let candidates: Vec<ClippyFixEntry> = corpus.lookup(lint).cloned().collect();
        // Assert the premise of this arm rather than assuming it.
        assert!(
            candidates.is_empty(),
            "`{lint}` was expected to be OUT of corpus but has {} candidate(s) — \
             the corpus grew; pick another lint",
            candidates.len()
        );
        let result = pipeline.heal(lint, buggy, &candidates);
        let outcome = match &result {
            None => Outcome::MissNoCandidate,
            Some(r) if !r.parses => Outcome::MissNonParsing,
            Some(_) => Outcome::ServedParsing,
        };
        // Record WHERE a served answer came from, and whether it is even
        // relevant to the input. With no corpus candidates the only paths left
        // are memory / evolution, so a "served" answer here is evidence about
        // the pipeline's decline behaviour, not about fix quality.
        if let Some(r) = &result {
            let first = r.fixed_code.lines().next().unwrap_or("").trim();
            let echoes_input = normalize(&r.fixed_code) == normalize(buggy);
            eprintln!(
                "     [{lint}] source={:?} echoes_input={} served={:?}",
                r.source,
                echoes_input,
                if first.len() > 58 {
                    &first[..58]
                } else {
                    first
                }
            );
        }
        out.total += 1;
        match outcome {
            Outcome::ServedParsing => out.served += 1,
            Outcome::MissNonParsing => out.miss_non_parsing += 1,
            Outcome::MissNoCandidate => out.miss_no_candidate += 1,
        }
        out.rows.push(((*lint).to_string(), outcome, false));
    }
    out
}

fn main() {
    let fixtures = [
        (
            "seed (in-distribution)",
            std::env::var("SEED_CASES")
                .unwrap_or_else(|_| "benches/fixtures/e2e_cases_seed.json".into()),
        ),
        (
            "heldout (generalization)",
            std::env::var("HELDOUT_CASES")
                .unwrap_or_else(|_| "benches/fixtures/e2e_cases_heldout.json".into()),
        ),
    ];

    println!("=== Bench 003 — modelless miss rate (the L4-addressable set) ===\n");
    println!(
        "miss #1 = heal() -> None (self_evolve.rs:517); \
         miss #2 = Some(!parses) (self_evolve.rs:574)"
    );
    println!(
        "L4 is consulted ONLY on misses. A parsing-but-wrong fix never reaches \
         L4, so EM is context, NOT L4-addressable.\n"
    );

    let mut results = Vec::new();
    for (label, path) in &fixtures {
        let r = measure(path);
        println!("── {label} — fixture `{}` ({} cases)", r.name, r.total);
        println!(
            "   {:<28} {:<16} exact-match (context only)",
            "lint", "outcome"
        );
        for (lint, outcome, exact) in &r.rows {
            let o = match outcome {
                Outcome::ServedParsing => "served/parses",
                Outcome::MissNonParsing => "MISS#2 !parses",
                Outcome::MissNoCandidate => "MISS#1 none",
            };
            println!(
                "   {lint:<28} {o:<16} {}",
                if *exact { "yes" } else { "no" }
            );
        }
        println!(
            "   served={} miss#1={} miss#2={} → MISS RATE {}/{} = {:.1}%   (EM {}/{} = {:.1}%, context)",
            r.served,
            r.miss_no_candidate,
            r.miss_non_parsing,
            r.misses(),
            r.total,
            r.miss_rate() * 100.0,
            r.em,
            r.total,
            if r.total == 0 { 0.0 } else { r.em as f64 / r.total as f64 * 100.0 }
        );
        println!();
        results.push((label.to_string(), r));
    }

    // ── The decisive arm: lints the corpus does not cover ──────────────────
    {
        let r = measure_out_of_corpus();
        println!(
            "── out-of-corpus (production-realistic) — {} real clippy lints with NO corpus entry",
            r.total
        );
        println!("   {:<28} outcome", "lint");
        for (lint, outcome, _) in &r.rows {
            let o = match outcome {
                Outcome::ServedParsing => "served/parses",
                Outcome::MissNonParsing => "MISS#2 !parses",
                Outcome::MissNoCandidate => "MISS#1 none",
            };
            println!("   {lint:<28} {o}");
        }
        println!(
            "   served={} miss#1={} miss#2={} → MISS RATE {}/{} = {:.1}%",
            r.served,
            r.miss_no_candidate,
            r.miss_non_parsing,
            r.misses(),
            r.total,
            r.miss_rate() * 100.0
        );
        println!();
        results.push(("out-of-corpus".to_string(), r));
    }

    // ── Verdict on Plan 336 Phase 5 ───────────────────────────────────────
    let total: usize = results.iter().map(|(_, r)| r.total).sum();
    let misses: usize = results.iter().map(|(_, r)| r.misses()).sum();
    let heldout_miss = results
        .iter()
        .find(|(l, _)| l.starts_with("heldout"))
        .map(|(_, r)| r.misses())
        .unwrap_or(0);

    println!("=== L4-addressable gap ===");
    println!("  combined miss rate : {misses}/{total}");
    println!("  held-out misses    : {heldout_miss}");
    println!(
        "  NOTE: Stage 2 untuned, so these are an UPPER BOUND on the miss rate \
         (a Stage-2 hit can only reduce it)."
    );
    if misses == 0 {
        println!(
            "\n  VERDICT INPUT: zero misses on both fixtures → L4 is UNREACHABLE on \
             this corpus.\n  No checkpoint, however good, can be consulted. Phase 5 \
             (T5.1/T5.3/T5.4) has no measured\n  addressable gap to justify it here."
        );
    } else {
        println!(
            "\n  VERDICT INPUT: {misses} case(s) reach the L4 seam → a real \
             addressable gap exists.\n  Phase 5 is justified IF a checkpoint can \
             beat 'decline' on those cases (G1 is currently 0/60)."
        );
    }

    let out_path =
        std::env::var("OUT_JSON").unwrap_or_else(|_| "/tmp/bench003_miss_rate.json".into());
    let mut s = String::from("{\"fixtures\":[");
    for (i, (label, r)) in results.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"label\":\"{}\",\"name\":\"{}\",\"total\":{},\"served\":{},\
             \"miss_no_candidate\":{},\"miss_non_parsing\":{},\"misses\":{},\
             \"miss_rate\":{:.6},\"exact_match\":{},\"stage2_tuned\":false}}",
            label,
            r.name,
            r.total,
            r.served,
            r.miss_no_candidate,
            r.miss_non_parsing,
            r.misses(),
            r.miss_rate(),
            r.em
        ));
    }
    s.push_str(&format!(
        "],\"combined_total\":{total},\"combined_misses\":{misses},\
         \"l4_reachable\":{}}}\n",
        misses > 0
    ));
    std::fs::write(&out_path, &s).unwrap_or_else(|e| panic!("write {out_path}: {e}"));
    println!("\nwrote {out_path}");
    let _ = std::io::stdout().flush();
}

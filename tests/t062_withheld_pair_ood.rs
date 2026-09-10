//! Issue 062 Phase 1 — the withheld-(role, filler) OOD generalization bench.
//!
//! Preregistered in `.benchmarks/062_withheld_pair_ood.md` (predictions
//! P1–P5 + the verdict rule were committed BEFORE this harness ran).
//!
//! ```bash
//! CARGO_TARGET_DIR=/tmp/riir_clippy_062 cargo test --release \
//!   --features latent_retrieval --test t062_withheld_pair_ood -- --nocapture
//! ```
//!
//! Deliberately NOT a committed regression floor: it is a measurement of an
//! axis, run when the corpus or the ranking changes. The only assertions are
//! **instrument-integrity** ones (the NOROLE canary, the withholding
//! assertion, the non-uniformity check, the nonzero-count check) — a number
//! is never pinned here, because pinning it would convert an honest
//! re-measurement into a red suite.

#![cfg(all(feature = "latent_retrieval", feature = "clippy_lints"))]

use riir_clippy::draft::withheld_pair_ood::{
    clippy_population, print_matrix, print_report, print_rows, run_withheld_pair, Binding,
    WithheldReport,
};
use riir_clippy::draft::{DomainId, RerankMode};

/// The Plan 049 T0.1 committed real-repo dataset — 2,128 finding-bearing
/// spans from the workspace, 20 of which carry an in-corpus label. Real
/// production shapes are the strongest filler diversity available; the
/// hand-authored corpus/fixture texts are minimal by construction.
const REAL_DATASET: &str = ".benchmarks/data/049_t01/spans.jsonl";

#[derive(serde::Deserialize)]
struct Row {
    code: String,
    label: Option<String>,
}

fn real_repo_bindings() -> Vec<Binding> {
    let Ok(bytes) = std::fs::read(REAL_DATASET) else {
        eprintln!("NOTE: {REAL_DATASET} missing — running without the real-repo bindings");
        return Vec::new();
    };
    bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .filter_map(|l| serde_json::from_slice::<Row>(l).ok())
        .filter_map(|r| r.label.map(|l| Binding::clippy(l, r.code, "real-repo")))
        .collect()
}

fn t1_population() -> Vec<Binding> {
    let mut pop = clippy_population();
    pop.extend(real_repo_bindings());
    pop
}

/// Instrument integrity — asserted on every arm, never a pinned number.
fn assert_instrument_is_alive(r: &WithheldReport) {
    // A cfg-gated harness that compiles to nothing "passes". Prove it ran.
    assert!(
        r.n() > 0,
        "{}: ZERO eligible queries — the protocol did not run",
        r.mode_label
    );
    assert!(
        r.eligible_cells > 0,
        "{}: ZERO eligible cells",
        r.mode_label
    );
    // The dead-instrument canary: an unindexed ROLE cannot be retrieved.
    // Nonzero here means the index is not being rebuilt or the rank is
    // misread — a defect, not a finding.
    assert!(
        r.norole_topk(5) == 0.0,
        "{}: NOROLE canary scored {:.1}% at top-5 — HARNESS BUG (the withheld role is still reachable)",
        r.mode_label,
        r.norole_topk(5)
    );
    // Uniform per-item scores = harness bug, not a finding.
    assert!(
        !r.scores_are_uniform(),
        "{}: every ID score identical — harness bug",
        r.mode_label
    );
    // The atomic null is zero OOD BY CONSTRUCTION (shape-prefixed keys).
    // Above zero means the withholding leaked.
    let (s, b) = r.null_ood();
    assert!(
        s == 0.0 && b == 0.0,
        "{}: atomic null scored {s:.1}%/{b:.1}% on WITHHELD pairs — withholding LEAK, treat as harness bug",
        r.mode_label
    );
}

#[test]
fn t1_healer_corpus_withheld_pair_ood() {
    let pop = t1_population();
    println!("\n######## Issue 062 T1 — healer corpus (clippy_lints) ########");
    println!("  raw population {} bindings", pop.len());

    // Arm A: PURE RETRIEVAL (no instantiation evidence) — the arm the
    // preregistered P1/P2 bars apply to.
    let off = run_withheld_pair(
        pop.clone(),
        None,
        RerankMode::Off,
        "pure-retrieval (no rerank)",
    );
    print_matrix(&off);
    print_report(&off);
    assert_instrument_is_alive(&off);

    // Arm B: the SHIPPED default — `RerankMode::Structural` re-ranks with
    // proposer-visitor evidence, which probes the SPAN and is therefore
    // index-independent. P4 predicts this closes most of the gap.
    let structural = run_withheld_pair(
        t1_population(),
        Some(DomainId::Clippy),
        RerankMode::Structural,
        "shipped (Structural rerank)",
    );
    print_report(&structural);
    print_rows(&structural);
    assert_instrument_is_alive(&structural);

    println!("\n  == T1 preregistered bars ==");
    for r in [&off, &structural] {
        let (nsig, nbag) = r.null_id();
        println!(
            "  {:<32} ID {:>5.1}%  OOD {:>5.1}%  Δ {:+.1}pp   null_ID sig/bag {nsig:.1}%/{nbag:.1}%",
            r.mode_label,
            r.id_topk(1),
            r.ood_topk(1),
            r.drop_pp(1)
        );
    }
    println!(
        "  P1 (retrieval Δ ≥ 20.0pp): {}",
        verdict_word(off.drop_pp(1) >= 20.0)
    );
    println!(
        "  P2 (retrieval OOD ≤ null_ID(bag) + 15.0pp): {}",
        verdict_word(off.ood_topk(1) <= off.null_id().1 + 15.0)
    );
    println!("  P3 (atomic null OOD == 0): PASS (asserted)");
    println!(
        "  P4 (Structural Δ ≤ 10.0pp): {}",
        verdict_word(structural.drop_pp(1) <= 10.0)
    );
    println!(
        "  VERDICT (retrieval arm Δ={:+.1}pp): {}",
        off.drop_pp(1),
        verdict(off.drop_pp(1))
    );
}

#[cfg(feature = "rustc_errors")]
#[test]
fn t2_rustc_errors_withheld_pair_ood() {
    use riir_clippy::draft::withheld_pair_ood::rustc_errors_population;

    let pop = rustc_errors_population();
    println!("\n######## Issue 062 T2 — rustc_errors (code × pattern) ########");
    println!("  raw population {} bindings", pop.len());

    // No proposer visitors exist for error codes — repair-playbook selection
    // is keyed by the code, so the measurable question is whether the code
    // (the role) is recoverable from the pattern alone. Pure retrieval only.
    let r = run_withheld_pair(pop, None, RerankMode::Off, "rustc_errors pure-retrieval");
    print_matrix(&r);
    print_report(&r);
    print_rows(&r);
    assert_instrument_is_alive(&r);

    println!("\n  == T2 preregistered bars ==");
    println!(
        "  P5 (Δ ≥ 20.0pp): {}   Δ={:+.1}pp",
        verdict_word(r.drop_pp(1) >= 20.0),
        r.drop_pp(1)
    );
    println!(
        "  VERDICT (Δ={:+.1}pp): {}",
        r.drop_pp(1),
        verdict(r.drop_pp(1))
    );
}

fn verdict_word(held: bool) -> &'static str {
    if held {
        "HELD"
    } else {
        "REFUTED"
    }
}

/// The preregistered verdict rule, applied to the pure-retrieval paired drop.
fn verdict(drop_pp: f32) -> &'static str {
    match drop_pp {
        d if d >= 20.0 => "Phase 2 JUSTIFIED (>=20pp gap)",
        d if d <= 5.0 => "Phase 2 HONESTLY DEMOTED (OOD at ceiling, <=5pp gap)",
        _ => "PARTIAL (5-20pp) — no promotion claim, gap recorded",
    }
}

//! Plan 049 T0.2 + T0.3 — the trained-drafter Phase-0 stop verdict over the
//! committed real-repo dataset (T0.1, `.benchmarks/data/049_t01/`).
//!
//! Two layers, one verdict:
//!
//! 1. **Corpus-coverage layer (dataset-level, the binding one):** of the
//!    2,128 finding-bearing real spans, only 20 carry ANY healable target in
//!    the current retrieval corpus (0.94%). A trained PRIOR over the
//!    existing rule set cannot address the other 99.06% at any ranking
//!    quality — proposer-absent at span level is not a ranking failure.
//!    Even a PERFECT selector recovering all 20 addresses 0.94% of the real
//!    surface < the 2.0pp stop bar.
//! 2. **Ranking layer (instrument, for the record):** the shipped
//!    `decompose_headroom_with_spans` over the 20 labeled spans classifies
//!    each selection miss (retrieval / pool-depth / type-blind /
//!    unfixable / selector-addressable). With n=20 the measurable effect of
//!    any selector is ≤10 spans — the read-the-count caveat the instrument
//!    itself prints.
//!
//! The T0.3 stop rule fires on layer 1 regardless of layer 2's outcome.

use riir_refine::draft::{
    DomainId, OwnedEvalSpan, decompose_headroom_with_spans, print_headroom_decomposition,
};

const DATASET: &str = ".benchmarks/data/049_t01/spans.jsonl";
/// The plan's T0.3 bar, in percentage points of the real finding-bearing
/// surface.
const STOP_BAR_PP: f32 = 2.0;

#[derive(serde::Deserialize)]
struct Row {
    code: String,
    label: Option<String>,
}

#[test]
fn t023_stop_verdict_over_the_real_repo_dataset() {
    let bytes = std::fs::read(DATASET)
        .unwrap_or_else(|e| panic!("dataset missing ({e}) — T0.1 must land first"));
    let rows: Vec<Row> = bytes
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice(l).expect("row parses"))
        .collect();
    let total = rows.len();
    assert!(total >= 300, "dataset below the T0.1 bar");

    let labeled: Vec<OwnedEvalSpan> = rows
        .iter()
        .filter_map(|r| {
            r.label.as_ref().map(|l| OwnedEvalSpan {
                label: l.clone(),
                domain: DomainId::Clippy,
                code: r.code.clone(),
                acceptable: Vec::new(),
            })
        })
        .collect();

    // ── layer 1: the corpus-coverage band over the REAL surface ──────────
    let in_vocab_band_pp = labeled.len() as f32 / total as f32 * 100.0;
    println!("\n== Plan 049 T0.2/T0.3 stop verdict ==");
    println!(
        "  layer 1 (corpus coverage): {}/{} real finding-bearing spans have an in-corpus target = {:.2}pp",
        labeled.len(),
        total,
        in_vocab_band_pp
    );
    let layer1_stop = in_vocab_band_pp < STOP_BAR_PP;

    // ── layer 2: the ranking decomposition over the labeled subset ───────
    let decomp = decompose_headroom_with_spans(&labeled);
    print_headroom_decomposition(&decomp);
    println!(
        "  layer 1 verdict: {:.2}pp < {STOP_BAR_PP}pp bar -> {}",
        in_vocab_band_pp,
        if layer1_stop { "STOP" } else { "proceed" }
    );

    // The verdict is PINNED: the plan closes on layer 1 alone. If a future
    // corpus expansion changes the vocabulary, re-run T0.1 (new dataset) and
    // this assert updates with the data — never loosen it against stale data.
    assert!(
        layer1_stop,
        "in-vocab band {:.2}pp now clears the {STOP_BAR_PP}pp bar — re-run the Phase-0 verdict, the trained-drafter question reopens",
        in_vocab_band_pp
    );
    // And the instrument agrees on the labeled subset's own denominator too
    // (both layers must stop for the close to be honest — if the 20-span
    // decomposition somehow showed a large addressable band, that would be a
    // corpus-expansion signal, not a selector one).
    assert!(
        decomp.stop() || labeled.len() < 30,
        "dataset-level band is under the bar but the labeled-subset decomposition shows an addressable band — record BOTH before closing"
    );
}

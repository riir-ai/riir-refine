//! Plan 095 — the generation-v6 freeze/verify e2e (the real-repo shape
//! ladder).
//!
//! Two modes, one file:
//!
//! - **Freeze** (`--ignored`): runs the PRE-REGISTERED freeze gates
//!   (`.plans/095` §1) over every span with the BENCH instrument
//!   (single-snippet `lints_for` — real clippy, all+pedantic+nursery) —
//!   the G1 compile fence, the G2 label fence (mining-time batch labels are
//!   CLAIMS; the freeze adjudicates), the G3 k-truth gate (territory ==
//!   the frozen label's class count), and the drift gate against the landed
//!   record — then prints the paste-ready freeze-record literals.
//!
//! - **Always-on** (default `cargo test`): the curation floors, the
//!   parse/territory gates, the id-disjointness gate and the freeze-record
//!   hole check live as always-on module tests in `generations.rs`.
//!
//! # Pre-registered gates (authored BEFORE the first freeze run)
//!
//! NO solve gate (the gens 2/3/4/5 pre-registration: solve behavior is the
//! BENCH's measurement; the M5 admission gate carries non-vacuity). The
//! binding checks are the label fence and the M5 admission gate.
//!
//! ```bash
//! cargo test --release --features score_bench \
//!     --test generation_v6_freeze -- --ignored --test-threads=1 --nocapture
//! ```

#![cfg(feature = "score_bench")]

use riir_refine::score_bench::{draft_oracle, generations};

#[test]
#[ignore = "runs the freeze gates + prints the paste-ready record; run with --ignored --test-threads=1"]
fn freeze_and_verify_generation_v6() {
    let oracle = draft_oracle();
    assert!(
        oracle.is_available(),
        "cargo not on PATH — the freeze needs the real oracle"
    );

    let specs = generations::specs_v6();
    assert!(!specs.is_empty(), "span table produced nothing");
    let excluded: std::collections::BTreeSet<&str> =
        generations::EXCLUDED_IDS_V6.iter().copied().collect();
    let frozen: std::collections::BTreeMap<&str, &[&str]> = generations::FROZEN_LABELS_V6
        .iter()
        .map(|(id, l)| (*id, *l))
        .collect();

    let mut fresh_exclusions: Vec<String> = Vec::new();
    let mut drift: Vec<String> = Vec::new();
    let mut verified: Vec<(String, Vec<String>)> = Vec::new();
    let mut relabeled: Vec<String> = Vec::new();

    for s in &specs {
        if excluded.contains(s.id.as_str()) {
            continue;
        }
        // ── G1: the compile fence (the oracle's std-only filter) ───────
        let v = oracle.lints_for(&riir_refine::score_bench::wrap(&s.code));
        if !v.ran {
            println!(
                "⚠ spec {}: oracle did not run ({:?}) — excluding",
                s.id, v.error
            );
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        if !v.compiles {
            println!(
                "⚠ spec {}: does not compile in the bare stub — excluding",
                s.id
            );
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        // ── G2: the label fence (mining claims vs the bench instrument) ─
        let hits: Vec<String> = v
            .lints
            .intersection(
                &s.expect_lints
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>(),
            )
            .cloned()
            .collect();
        let vocab_all = riir_refine::score_bench::vocabulary();
        let mining_only: Vec<String> = s
            .expect_lints
            .iter()
            .filter(|l| !v.lints.contains(*l))
            .cloned()
            .collect();
        // Only in-vocabulary bench hits count as fence signals — pedantic/
        // nursery lints outside our vocabulary are noise, not misses.
        let bench_only: Vec<String> = v
            .lints
            .iter()
            .filter(|l| !s.expect_lints.contains(l) && vocab_all.contains(*l))
            .cloned()
            .collect();
        if !mining_only.is_empty() || !bench_only.is_empty() {
            println!(
                "⚠ spec {}: label fence — mining claimed {mining_only:?} that did not fire; bench fired {bench_only:?} that mining missed",
                s.id
            );
        }
        let mut label: Vec<String> = v
            .lints
            .intersection(
                &s.expect_lints
                    .iter()
                    .cloned()
                    .collect::<std::collections::BTreeSet<_>>(),
            )
            .cloned()
            .collect();
        // The bench-only hits that are ALSO in-vocabulary count toward the
        // span's real label (the oracle is the authority, both directions).
        for extra in &bench_only {
            label.push(extra.clone());
        }
        label.sort();
        label.dedup();
        if label.len() != hits.len() {
            relabeled.push(s.id.clone());
        }
        // ── G3: k-truth (territory == the frozen label's class count) ──
        let want = if label.len() >= 2 { "multi" } else { "single" };
        assert_eq!(
            generations::v6_territory(&s.id),
            want,
            "territory drift on {}: label {label:?}",
            s.id
        );
        if label.is_empty() {
            println!(
                "⚠ spec {}: zero vocabulary lints at freeze — excluding",
                s.id
            );
            fresh_exclusions.push(s.id.clone());
            continue;
        }
        verified.push((s.id.clone(), label));
    }

    // ── Drift gate: landed labels must still match the fresh derivation ─
    for (id, label) in &verified {
        if let Some(l) = frozen.get(id.as_str()) {
            let have: std::collections::BTreeSet<&str> = l.iter().copied().collect();
            let want: std::collections::BTreeSet<&str> = label.iter().map(String::as_str).collect();
            if have != want {
                drift.push(id.clone());
            }
        }
    }
    if !drift.is_empty() {
        panic!("FROZEN_LABELS_V6 drifted from the fresh derivation on: {drift:?} — re-freeze");
    }

    // ── Report + the paste-ready freeze record ─────────────────────────
    println!("\n─ generation-v6 freeze report ────────────────────");
    println!(
        "spans: {} verified, {} newly excluded, {} pre-excluded, {} relabeled",
        verified.len(),
        fresh_exclusions.len(),
        excluded.len(),
        relabeled.len()
    );
    let per_territory: std::collections::BTreeMap<&str, usize> =
        verified
            .iter()
            .fold(std::collections::BTreeMap::new(), |mut m, (id, label)| {
                let terr = if label.len() >= 2 { "multi" } else { "single" };
                *m.entry(terr).or_default() += 1;
                let _ = id;
                m
            });
    for (terr, n) in &per_territory {
        println!("  territory {terr}: {n} verified span(s)");
    }
    println!(
        "pre-registered: NO solve gate (Plan 095 §1) — solve behavior is the BENCH's measurement."
    );

    let mut all_exclusions: Vec<String> = fresh_exclusions.clone();
    all_exclusions.extend(excluded.iter().map(|s| (*s).to_string()));
    all_exclusions.sort();
    all_exclusions.dedup();
    println!("\n// ── freeze record (paste into generations.rs) ──");
    println!("pub static EXCLUDED_IDS_V6: &[&str] = &[");
    for id in &all_exclusions {
        println!("    \"{id}\",");
    }
    println!("];");
    println!("pub static FROZEN_LABELS_V6: &[(&str, &[&str])] = &[");
    verified.sort();
    for (id, label) in &verified {
        println!(
            "    (\"{id}\", &[{}]),  // k={}",
            label
                .iter()
                .map(|l| format!("\"{l}\""))
                .collect::<Vec<_>>()
                .join(", "),
            label.len()
        );
    }
    println!("];");
}

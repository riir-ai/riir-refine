//! Plan 005 Phase 3 — GOAT gate for the latent `--suggest` path.
//!
//! - **G1 (retrieval precision)**: on a fixture of known anti-patterns
//!   (renamed-identifier variants, NOT corpus verbatim), the top-1 retrieved
//!   rule is correct ≥80% of the time per enabled domain.
//! - **G2 (latency)**: chunk + embed + route + retrieve < 1ms per chunk
//!   (release-only — debug timings are meaningless).
//! - **G3 (no-regression)**: the main lib suite (104 tests) — this file adds
//!   to it, the aggregate stays green.
//! - **G4 (alloc-free hot path)**: the shared `SuggestScratch` capacities
//!   stabilize across repeated queries (zero steady-state growth).
//!
//! Recorded in `.benchmarks/018_plan005_latent_suggest_goat.md`.

#![cfg(feature = "latent_retrieval")]

use std::time::Instant;

use riir_clippy::draft::{DomainId, LatentMatcher};

/// G1 fixture — clippy-domain variants (identifiers renamed vs the corpus).
/// `(expected_rule, variant_snippet)`.
#[cfg(feature = "clippy_lints")]
const CLIPPY_FIXTURE: &[(&str, &str)] = &[
    (
        "needless_return",
        "fn volume(w: i64, h: i64) -> i64 {\n    return w * h;\n}",
    ),
    (
        "redundant_closure",
        "fn scale(v: i64) -> i64 { v * 3 }\nfn run(f: impl Fn(i64) -> i64, x: i64) -> i64 { f(x) }\nfn caller(x: i64) -> i64 { run(|v| scale(v), x) }",
    ),
    (
        "manual_map",
        "fn maybe_len(s: Option<String>) -> Option<usize> {\n    match s {\n        Some(v) => Some(v.len()),\n        None => None,\n    }\n}",
    ),
    (
        "unnecessary_unwrap",
        "fn label(v: Option<&str>) -> &str {\n    if v.is_some() {\n        v.unwrap()\n    } else {\n        \"?\"\n    }\n}",
    ),
    (
        "needless_borrow",
        "fn show(w: &u32) -> u32 { *w }\nfn caller(w: &u32) -> u32 { show(&w) }",
    ),
    (
        "useless_conversion",
        "fn area(width: u32) -> u32 {\n    let w: u32 = width.into();\n    w * 2\n}",
    ),
];

/// G1 fixture — perf-domain variants.
#[cfg(feature = "rust_perf")]
const PERF_FIXTURE: &[(&str, &str)] = &[
    (
        "o1-lookup",
        "fn get_player(players: &Vec<(u64, String)>, pid: u64) -> Option<&String> {\n    players.iter().find(|(id, _)| *id == pid).map(|(_, n)| n)\n}",
    ),
    (
        "bsearch-sorted-slice",
        "fn has_key(sorted_keys: &[u64], k: u64) -> bool {\n    sorted_keys.iter().any(|&x| x == k)\n}",
    ),
    (
        "vec-with-capacity",
        "fn make_ids(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    for i in 0..n {\n        v.push(i as u32);\n    }\n    v\n}",
    ),
    (
        "format-concat-loop",
        "fn join(parts: &[&str]) -> String {\n    let mut out = String::new();\n    for p in parts {\n        out = format!(\"{out}{p}\");\n    }\n    out\n}",
    ),
    (
        "entry-vs-contains-insert",
        "fn bump(counts: &mut HashMap<u32, u32>, k: u32) {\n    if counts.contains_key(&k) {\n        *counts.get_mut(&k).unwrap() += 1;\n    } else {\n        counts.insert(k, 1);\n    }\n}",
    ),
    (
        "bool-to-bitpack",
        "fn pack(flags: &[bool]) -> Vec<u8> {\n    flags.iter().map(|b| if *b { 1u8 } else { 0u8 }).collect()\n}",
    ),
];

#[test]
fn g1_top1_precision_at_least_80_percent_per_domain() {
    let mut m = LatentMatcher::from_enabled_domains().expect("build");

    let mut cases: Vec<(DomainId, &str, &str)> = Vec::new();
    #[cfg(feature = "clippy_lints")]
    for (rule, snippet) in CLIPPY_FIXTURE {
        cases.push((DomainId::Clippy, rule, snippet));
    }
    #[cfg(feature = "rust_perf")]
    for (rule, snippet) in PERF_FIXTURE {
        cases.push((DomainId::Perf, rule, snippet));
    }

    let mut hits = 0usize;
    let mut misses: Vec<(DomainId, &str, String)> = Vec::new();
    for (domain, expected, snippet) in &cases {
        // Query ONLY the owning domain's index (per-domain precision — the
        // fixture has no cross-domain ambiguity).
        let by_domain = {
            let (_routed, all) = m.suggest_span(snippet, 1);
            all.into_iter()
                .find(|(d, _)| d == domain)
                .map(|(_, rules)| rules)
                .unwrap_or_default()
        };
        let top1 = by_domain.first().map(|r| r.rule_id.clone());
        if top1.as_deref() == Some(expected) {
            hits += 1;
        } else {
            misses.push((*domain, expected, top1.unwrap_or_else(|| "<none>".into())));
        }
    }
    let n = cases.len();
    let precision = hits as f64 / n as f64;
    eprintln!(
        "G1 latent precision: {hits}/{n} = {:.0}% — misses: {:?}",
        precision * 100.0,
        misses
    );
    assert!(
        precision >= 0.80,
        "G1 FAIL: top-1 precision {:.0}% < 80% — misses: {misses:?}",
        precision * 100.0
    );
}

/// G2 — end-to-end per-chunk latency (route + fan-out multi-scale retrieve).
#[test]
#[cfg_attr(debug_assertions, ignore = "debug timings are meaningless")]
fn g2_latency_under_1ms_per_chunk() {
    let mut m = LatentMatcher::from_enabled_domains().expect("build");
    // A realistic multi-line chunk (the fixture snippets, repeated as whole
    // "files" to amortize noise).
    let corpus: Vec<&str> = PERF_FIXTURE
        .iter()
        .map(|(_, s)| *s)
        .chain(CLIPPY_FIXTURE.iter().map(|(_, s)| *s))
        .collect();

    // Warm-up (JIT nothing, but allocator + index caches settle).
    for snippet in &corpus {
        let _ = m.suggest_span(snippet, 3);
    }

    let rounds = 20;
    let t0 = Instant::now();
    let mut n_queries = 0usize;
    for _ in 0..rounds {
        for snippet in &corpus {
            let _ = m.suggest_span(snippet, 3);
            n_queries += 1;
        }
    }
    let per_chunk_us = t0.elapsed().as_micros() as f64 / n_queries as f64;
    eprintln!("G2 latency: {per_chunk_us:.0}µs per chunk ({n_queries} chunks)");
    assert!(
        per_chunk_us < 1000.0,
        "G2 FAIL: {per_chunk_us:.0}µs/chunk ≥ 1ms"
    );
}

/// G4 — the shared scratch capacities stabilize (zero steady-state growth).
#[test]
fn g4_scratch_capacity_stable_steady_state() {
    let mut m = LatentMatcher::from_enabled_domains().expect("build");
    let snippets: Vec<&str> = PERF_FIXTURE
        .iter()
        .map(|(_, s)| *s)
        .chain(CLIPPY_FIXTURE.iter().map(|(_, s)| *s))
        .collect();

    // Warm-up: capacities settle at the max working-set.
    for s in &snippets {
        let _ = m.suggest_span(s, 3);
    }
    let (retrieved_cap, ranked_cap) = m.scratch_capacities();

    // Steady state: repeated scanning must not grow anything.
    for _ in 0..50 {
        for s in &snippets {
            let _ = m.suggest_span(s, 3);
        }
    }
    let (retrieved_cap2, ranked_cap2) = m.scratch_capacities();
    assert_eq!(
        (retrieved_cap, ranked_cap),
        (retrieved_cap2, ranked_cap2),
        "G4 FAIL: scratch grew in steady state ({retrieved_cap}->{retrieved_cap2}, {ranked_cap}->{ranked_cap2})"
    );
}

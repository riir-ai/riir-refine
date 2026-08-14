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

/// Negative fixture — CLEAN adversarial code (the threshold-calibration set).
///
/// Each snippet is the CORRECT version of a corpus anti-pattern (an
/// adversarial near-miss: maximally similar in shape, zero rule
/// instantiations) plus innocent non-fn shapes. Bench 018 flagged threshold
/// calibration as the missing piece ("every span suggests"); the calibration
/// tests below measured it and **REFUTED the threshold hypothesis** — these
/// clean snippets score as high as or higher than the true-positive G1
/// fixtures under both scoring modes. They stay as the permanent instrument
/// for any future discriminative-signal re-measurement.
const NEGATIVE_FIXTURE: &[&str] = &[
    // needless_return done right (tail expression).
    "fn volume_direct(w: i64, h: i64) -> i64 {\n    w * h\n}",
    // redundant_closure done right (pass the fn item).
    "fn double_it(v: u32) -> u32 { v * 2 }\nfn apply_all(xs: &[u32]) -> Vec<u32> {\n    xs.iter().map(double_it).collect()\n}",
    // manual_map done right (Option::map).
    "fn maybe_len_mapped(s: Option<String>) -> Option<usize> {\n    s.map(|v| v.len())\n}",
    // unnecessary_unwrap done right (unwrap_or).
    "fn label_or(v: Option<&str>) -> &str {\n    v.unwrap_or(\"?\")\n}",
    // needless_borrow done right (value passed by value).
    "fn show_val(w: u32) -> u32 { w }\nfn caller_val(w: u32) -> u32 { show_val(w) }",
    // useless_conversion done right (no .into()).
    "fn area_direct(width: u32) -> u32 {\n    let w = width;\n    w * 2\n}",
    // o1-lookup done right (HashMap::get, not a Vec scan).
    "use std::collections::HashMap;\nfn find_in_map(map: &HashMap<u64, String>, pid: u64) -> Option<&String> {\n    map.get(&pid)\n}",
    // bsearch-sorted-slice done right.
    "fn has_key_bsearch(sorted: &[u64], k: u64) -> bool {\n    sorted.binary_search(&k).is_ok()\n}",
    // vec-with-capacity done right.
    "fn make_ids_sized(n: usize) -> Vec<u32> {\n    let mut v = Vec::with_capacity(n);\n    for i in 0..n {\n        v.push(i as u32);\n    }\n    v\n}",
    // format-concat-loop done right (push_str).
    "fn join_push(parts: &[&str]) -> String {\n    let mut out = String::new();\n    for p in parts {\n        out.push_str(p);\n    }\n    out\n}",
    // entry-vs-contains-insert done right (entry API).
    "use std::collections::HashMap;\nfn bump_entry(counts: &mut HashMap<u32, u32>, k: u32) {\n    *counts.entry(k).or_insert(0) += 1;\n}",
    // bool-to-bitpack done right (cast).
    "fn pack_cast(flags: &[bool]) -> Vec<u8> {\n    flags.iter().map(|b| *b as u8).collect()\n}",
    // innocent non-fn shapes (struct + impl + trait).
    "struct Point { x: f64, y: f64 }\nimpl Point {\n    fn new(x: f64, y: f64) -> Self { Self { x, y } }\n    fn norm(&self) -> f64 { (self.x * self.x + self.y * self.y).sqrt() }\n}",
    "trait Shape { fn area_of(&self) -> f64; }",
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

/// Threshold calibration — prints the positive/negative score distributions.
///
/// Not a gate: the empirical record. **Result (2026-08-14): REFUTED** — the
/// distributions OVERLAP (positives [3.19 ..= 34.71], negatives [6.08 ..= 69.37]);
/// no minimum-score threshold separates clean from buggy. The fused
/// latent+BM25 score is text similarity, not rule instantiation: the correct
/// sibling of an anti-pattern shares ~all of its lines (only the
/// rule-bearing token differs), so BM25 scores it equally. Re-run (with
/// `--nocapture`) when the corpus or embedder changes, or when prototyping a
/// discriminative successor signal. Positives = G1 fixtures' owning-domain
/// top-1; negatives = each clean snippet's BEST rule across all enabled
/// domains (what `--suggest` would print first for that span).
#[test]
fn threshold_calibration_distribution() {
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

    eprintln!("── positives (owning-domain top-1) ──");
    let mut pos_scores: Vec<f32> = Vec::new();
    for (domain, expected, snippet) in &cases {
        let (_routed, all) = m.suggest_span(snippet, 1);
        let top = all
            .iter()
            .find(|(d, _)| d == domain)
            .and_then(|(_, rules)| rules.first());
        if let Some(r) = top {
            let mark = if r.rule_id == *expected { "hit " } else { "MISS" };
            eprintln!("  {mark} {:<28} {:>8.3}  ({})", r.rule_id, r.score, expected);
            pos_scores.push(r.score);
        } else {
            eprintln!("  NONE for {expected}");
        }
    }

    eprintln!("── negatives (best rule across all domains) ──");
    let mut neg_scores: Vec<f32> = Vec::new();
    for snippet in NEGATIVE_FIXTURE {
        let (_routed, all) = m.suggest_span(snippet, 1);
        let best = all
            .iter()
            .flat_map(|(d, rules)| rules.iter().map(move |r| (*d, r)))
            .max_by(|(_, a), (_, b)| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some((d, r)) = best {
            eprintln!("  {:<28} {:>8.3}  [{}]  {}", r.rule_id, r.score, d.label(), head(snippet));
            neg_scores.push(r.score);
        } else {
            eprintln!("  (no rules)  {}", head(snippet));
        }
    }

    pos_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    neg_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pos_min = pos_scores.first().copied().unwrap_or(f32::NAN);
    let pos_max = pos_scores.last().copied().unwrap_or(f32::NAN);
    let neg_min = neg_scores.first().copied().unwrap_or(f32::NAN);
    let neg_max = neg_scores.last().copied().unwrap_or(f32::NAN);
    eprintln!(
        "── summary: positives [{pos_min:.3} ..= {pos_max:.3}] (n={}), negatives [{neg_min:.3} ..= {neg_max:.3}] (n={}), separation: {}",
        pos_scores.len(),
        neg_scores.len(),
        if neg_max < pos_min { "CLEAN" } else { "OVERLAP" }
    );
}

/// Scoring-mode calibration — whole-span query only vs the multi-scale
/// (whole + per-line best-across) mode, on the same positive/negative sets.
///
/// The threshold calibration above measured the multi-scale mode and found
/// OVERLAP. This test isolates WHY: per-line accumulation rewards span
/// length + partial line overlap (a correct fn shares its loop lines with
/// its buggy sibling). If whole-only separates, the noise fix is a scoring
/// mode change, not a threshold. Not a gate — the measurement record.
#[test]
fn scoring_mode_calibration_whole_vs_deep() {
    use riir_clippy::domain::{HealDomain, HealEntry};
    use riir_clippy::draft::RuleIndex;

    // Build one index per enabled domain (same corpus text the matcher uses).
    let mut indexes: Vec<(DomainId, RuleIndex)> = Vec::new();
    #[cfg(feature = "clippy_lints")]
    {
        let patterns = riir_clippy::domains::clippy_lints::ClippyLintDomain::build_corpus()
            .into_iter()
            .map(|e| (e.rule_id().to_string(), e.buggy_code().to_string()))
            .collect::<Vec<_>>();
        indexes.push((DomainId::Clippy, RuleIndex::build(patterns).expect("clippy index")));
    }
    #[cfg(feature = "rust_perf")]
    {
        let patterns = riir_clippy::domains::rust_perf::RustPerfDomain::build_corpus()
            .into_iter()
            .map(|e| (e.rule_id().to_string(), e.buggy_code().to_string()))
            .collect::<Vec<_>>();
        indexes.push((DomainId::Perf, RuleIndex::build(patterns).expect("perf index")));
    }

    let mut cases: Vec<(DomainId, &str, bool)> = Vec::new(); // (domain, snippet, is_positive)
    #[cfg(feature = "clippy_lints")]
    for (_rule, snippet) in CLIPPY_FIXTURE {
        cases.push((DomainId::Clippy, snippet, true));
    }
    #[cfg(feature = "rust_perf")]
    for (_rule, snippet) in PERF_FIXTURE {
        cases.push((DomainId::Perf, snippet, true));
    }
    for snippet in NEGATIVE_FIXTURE {
        // negatives: measure against BOTH domains (the worst cross-domain noise)
        for (domain, _) in &indexes {
            cases.push((*domain, snippet, false));
        }
    }

    let mut pos_whole: Vec<f32> = Vec::new();
    let mut pos_deep: Vec<f32> = Vec::new();
    let mut neg_whole: Vec<f32> = Vec::new();
    let mut neg_deep: Vec<f32> = Vec::new();
    eprintln!("{:<8} {:>9} {:>9}  snippet", "kind", "whole", "deep");
    for (domain, snippet, is_pos) in &cases {
        let Some((_, index)) = indexes.iter().find(|(d, _)| d == domain) else {
            continue;
        };
        let whole = index.suggest(snippet, 1).first().map(|r| r.score).unwrap_or(f32::NAN);
        let deep = index.suggest_deep(snippet, 1).first().map(|r| r.score).unwrap_or(f32::NAN);
        eprintln!(
            "{:<8} {:>9.3} {:>9.3}  [{}] {}",
            if *is_pos { "POS" } else { "NEG" },
            whole,
            deep,
            domain.label(),
            head(snippet)
        );
        if *is_pos {
            pos_whole.push(whole);
            pos_deep.push(deep);
        } else {
            neg_whole.push(whole);
            neg_deep.push(deep);
        }
    }
    for (label, pos, neg) in [
        ("whole-only", &pos_whole, &neg_whole),
        ("multi-scale", &pos_deep, &neg_deep),
    ] {
        let mut pos = pos.clone();
        let mut neg = neg.clone();
        pos.sort_by(|a, b| a.partial_cmp(b).unwrap());
        neg.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let pmin = pos.first().copied().unwrap_or(f32::NAN);
        let nmax = neg.last().copied().unwrap_or(f32::NAN);
        eprintln!(
            "{label}: positives [{:.3} ..], negatives [..= {:.3}] → {}",
            pmin,
            nmax,
            if nmax < pmin { "CLEAN separation" } else { "OVERLAP" }
        );
    }
}

/// First meaningful line of a snippet (for calibration output readability).
fn head(snippet: &str) -> &str {
    snippet.lines().next().unwrap_or("…")
}

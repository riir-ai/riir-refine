//! Seed corpus — rust performance rules distilled from the `rust-optimize`
//! skill checklist.
//!
//! Each entry is a `(rule_id, name, category, buggy_code, fixed_code)` tuple
//! representing a real perf pattern from the skill. The corpus is
//! BLAKE3-committed via [`Corpus::new`](crate::corpus::Corpus::new).
//!
//! ## Caveat
//!
//! These examples are **illustrative** — the L2 syntax pruner validates that
//! the fixed code parses as valid Rust, but does NOT verify the perf claim
//! (faster on production input) in the fast path. For truthful perf
//! resolution, enable the `rust_perf_verify` feature and use the verifier
//! (runs `cargo test`). The perf-bench Pareto validator is deferred (Proposal
//! 001 caveat 3) — a separate plan ships it.

use crate::corpus::Corpus;
use crate::domains::rust_perf::{PerfRule, RustPerfDomain};

/// Build a seed corpus from common rust performance rules.
///
/// Returns a BLAKE3-committed [`Corpus<RustPerfDomain>`] with 8 entries
/// covering the main `rust-optimize` skill categories (complexity, allocation,
/// layout, concurrency, simd, caching, bevy, wasm).
#[must_use]
pub fn build_perf_seed_corpus() -> Corpus<RustPerfDomain> {
    Corpus::new(vec![
        // ── complexity: O(n) scan → O(1) lookup ────────────────────────────
        PerfRule {
            rule_id: "o1-lookup".to_string(),
            name: "O(n) scan → O(1) lookup".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn find_user(users: &Vec<(u32, String)>, id: u32) -> Option<&String> {\n    users.iter().find(|(uid, _)| *uid == id).map(|(_, n)| n)\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nfn find_user(users: &HashMap<u32, String>, id: u32) -> Option<&String> {\n    users.get(&id)\n}".to_string(),
        },
        // ── allocation: String → &'static str ─────────────────────────────
        PerfRule {
            rule_id: "static-str".to_string(),
            name: "String → &'static str for constant labels".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn label() -> String { \"ready\".to_string() }".to_string(),
            fixed_code: "fn label() -> &'static str { \"ready\" }".to_string(),
        },
        // ── layout: field reorder to eliminate padding ────────────────────
        PerfRule {
            rule_id: "layout-reorder".to_string(),
            name: "Reorder fields to eliminate padding (u8 → u64 → u32 ordering fix)".to_string(),
            category: "layout".to_string(),
            // The bad layout has a u8 between two u64s, forcing 7 bytes of
            // padding after the u8. Reordering u64 → u32 → u8 packs to 16 bytes.
            buggy_code: "#[repr(C)]\nstruct Bad { a: u8, b: u64, c: u32 }".to_string(),
            fixed_code: "#[repr(C)]\nstruct Good { b: u64, c: u32, a: u8 }".to_string(),
        },
        // ── concurrency: Arc<RwLock<HashMap>> → papaya ────────────────────
        // Per the user AGENTS.md rule: "Use papaya as possible for lock-free
        // Arc<RwLock<HashMap>>".
        PerfRule {
            rule_id: "papaya-lockfree".to_string(),
            name: "Arc<RwLock<HashMap>> → papaya lock-free map".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::{Arc, RwLock};\nuse std::collections::HashMap;\nstruct Store(Arc<RwLock<HashMap<u32, String>>>);".to_string(),
            fixed_code: "use papaya::HashMap;\nstruct Store(HashMap<u32, String>);".to_string(),
        },
        // ── simd: chunked loop for auto-vectorization ─────────────────────
        PerfRule {
            rule_id: "simd-chunk".to_string(),
            name: "Chunked loop (4-wide) to help LLVM auto-vectorize".to_string(),
            category: "simd".to_string(),
            buggy_code: "fn sum(v: &[f32]) -> f32 {\n    let mut s = 0.0;\n    for i in 0..v.len() { s += v[i]; }\n    s\n}".to_string(),
            fixed_code: "fn sum(v: &[f32]) -> f32 {\n    let chunks = v.chunks_exact(4);\n    let rem = v.chunks_exact(4).remainder();\n    let mut acc = [0.0f32; 4];\n    for c in chunks {\n        acc[0] += c[0]; acc[1] += c[1]; acc[2] += c[2]; acc[3] += c[3];\n    }\n    acc.iter().chain(rem).copied().sum()\n}".to_string(),
        },
        // ── caching: pre-compute lookup table ─────────────────────────────
        PerfRule {
            rule_id: "precompute-table".to_string(),
            name: "Pre-compute lookup table once, reuse across calls".to_string(),
            category: "caching".to_string(),
            buggy_code: "fn parity_byte(b: u8) -> u8 {\n    (0..8).filter(|i| b & (1 << i) != 0).count() as u8 & 1\n}".to_string(),
            fixed_code: "const PARITY: [u8; 256] = {\n    let mut t = [0u8; 256];\n    let mut i = 0;\n    while i < 256 { t[i] = (i.count_ones() & 1) as u8; i += 1; }\n    t\n};\nfn parity_byte(b: u8) -> u8 { PARITY[b as usize] }".to_string(),
        },
        // ── bevy: reactive_low_power event loop ───────────────────────────
        // Per the rust-optimize skill: default Continuous pegs CPU at 100%
        // even idle; switch to reactive_low_power matching tick rate.
        PerfRule {
            rule_id: "bevy-reactive-loop".to_string(),
            name: "Bevy Continuous → reactive_low_power event loop".to_string(),
            category: "bevy".to_string(),
            buggy_code: "use bevy::winit::WinitSettings;\nfn setup() -> WinitSettings { WinitSettings::default() }".to_string(),
            fixed_code: "use bevy::winit::{UpdateMode, WinitSettings};\nuse std::time::Duration;\nfn setup() -> WinitSettings {\n    WinitSettings {\n        focused_mode: UpdateMode::reactive_low_power(Duration::from_millis(50)),\n        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(1)),\n    }\n}".to_string(),
        },
        // ── wasm: batch API to amortize FFI floor ─────────────────────────
        // Per the rust-optimize skill: ~250ns FFI floor amortized across N
        // calls via a single batch invocation.
        PerfRule {
            rule_id: "wasm-batch-ffi".to_string(),
            name: "Batch N×M FFI calls into one (amortize ~250ns floor)".to_string(),
            category: "wasm".to_string(),
            buggy_code: "// Per-call FFI: 250ns floor × N×M invocations\nfn validate_all(items: &[u32], rules: &[u32]) -> Vec<bool> {\n    items.iter().map(|i| rules.iter().map(|r| validate_one(*i, *r)).all(|b| b)).collect()\n}\nfn validate_one(_i: u32, _r: u32) -> bool { true }".to_string(),
            fixed_code: "// Batch: one FFI call carries shared state + per-item arrays\nfn validate_all(items: &[u32], rules: &[u32]) -> Vec<bool> {\n    let mut out = vec![false; items.len()];\n    validate_batch(items, rules, &mut out);\n    out\n}\nfn validate_batch(_items: &[u32], _rules: &[u32], _out: &mut [bool]) {\n    // single FFI call: shared rules + per-item arrays → out\n}".to_string(),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_corpus_round_trips() {
        let corpus = build_perf_seed_corpus();
        assert!(corpus.verify());
        let frozen = corpus.freeze().unwrap();
        let thawed = Corpus::<RustPerfDomain>::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn seed_corpus_has_eight_entries_across_eight_categories() {
        let corpus = build_perf_seed_corpus();
        assert_eq!(corpus.entries.len(), 8);
        let mut categories: Vec<&str> =
            corpus.entries.iter().map(|e| e.category.as_str()).collect();
        categories.sort_unstable();
        categories.dedup();
        // 8 distinct categories (one rule per category).
        assert_eq!(categories.len(), 8);
    }

    #[test]
    fn seed_corpus_rule_ids_are_unique() {
        let corpus = build_perf_seed_corpus();
        let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
    }

    #[test]
    fn tamper_detected() {
        let corpus = build_perf_seed_corpus();
        let mut tampered = corpus.clone();
        tampered.entries[0].fixed_code = "// tampered".to_string();
        assert!(!tampered.verify());
    }

    #[test]
    fn heal_entry_accessors_return_correct_fields() {
        use crate::domain::HealEntry;
        let corpus = build_perf_seed_corpus();
        // Corpus is sorted by rule_id alphabetically — find by id, not index.
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "o1-lookup")
            .expect("o1-lookup should be in seed corpus");
        assert_eq!(entry.rule_id(), "o1-lookup");
        assert!(!entry.buggy_code().is_empty());
        assert!(!entry.fixed_code().is_empty());
    }

    #[test]
    fn heal_domain_build_corpus_matches_seed() {
        use crate::domain::HealDomain;
        let via_trait = RustPerfDomain::build_corpus();
        let via_seed = build_perf_seed_corpus().entries;
        assert_eq!(via_trait.len(), via_seed.len());
        // Both go through the same sort, so the order is identical.
        assert_eq!(via_trait[0].rule_id, via_seed[0].rule_id);
        assert_eq!(via_trait[7].rule_id, via_seed[7].rule_id);
    }

    #[test]
    #[cfg(not(feature = "rust_perf_verify"))]
    fn validate_parses_valid_code() {
        use crate::domain::HealDomain;
        let verdict = RustPerfDomain::validate(
            "o1-lookup",
            "fn foo() -> i32 { 42 }",
        );
        assert!(verdict.parses);
    }

    #[test]
    #[cfg(not(feature = "rust_perf_verify"))]
    fn validate_rejects_garbage() {
        use crate::domain::HealDomain;
        let verdict = RustPerfDomain::validate("o1-lookup", "not valid rust {");
        assert!(!verdict.parses);
    }
}

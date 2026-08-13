//! Seed corpus — GPU kernel optimization rules.
//!
//! **STUB** — real corpus distillation from popcorn is deferred (Proposal 012
//! §Phase 0, ~2 weeks reading job). This module ships 2 placeholder rules so
//! the [`KernelOptDomain`] trait impl is exercised + the scaffold compiles.
//!
//! Each placeholder rule pairs a naive reference kernel with a hand-optimized
//! pattern. The patterns are Rust-shaped pseudocode (not real CUDA/WGSL) so
//! the shared `RustAnalyzerPruner` can validate them in the fast path. The
//! real corpus will carry real kernel source once the popcorn distillation
//! lands.

use crate::corpus::Corpus;
use crate::domains::kernel_opt::{KernelOptDomain, KernelRule};

/// Build a seed corpus from placeholder GPU kernel optimization rules.
///
/// Returns a BLAKE3-committed [`Corpus<KernelOptDomain>`] with 2 placeholder
/// entries. **Real corpus deferred** — see module docs.
#[must_use]
pub fn build_kernel_seed_corpus() -> Corpus<KernelOptDomain> {
    Corpus::new(vec![
        // ── coalesced memory access ───────────────────────────────────────
        // The naive pattern strided-reads from a column-major layout (uncoalesced,
        // 1 lane per cache line). The optimized pattern transposes for coalesced
        // access (consecutive lanes hit consecutive bytes → full cache line reuse).
        KernelRule {
            rule_id: "matvec-coalesce".to_string(),
            gpu_concept: "coalesced memory access".to_string(),
            ref_shape: "// naive: strided read (uncoalesced)\nfn matvec_naive(a: &[[f32; N]; N], x: &[f32; N], y: &mut [f32; N]) {\n    for col in 0..N {\n        let mut acc = 0.0;\n        for row in 0..N { acc += a[row][col] * x[row]; }\n        y[col] = acc;\n    }\n}".to_string(),
            opt_pattern: "// optimized: coalesced read (consecutive lanes hit consecutive bytes)\nfn matvec_coalesced(a_t: &[[f32; N]; N], x: &[f32; N], y: &mut [f32; N]) {\n    for row in 0..N {\n        let xr = x[row];\n        for col in 0..N { y[col] += a_t[row][col] * xr; }\n    }\n}".to_string(),
        },
        // ── shared-memory tiling ──────────────────────────────────────────
        // The naive GEMM reads A and B from global memory N*K + K*N times.
        // The tiled version loads a tile into shared memory once, reuses it
        // across the tile's output elements (M*K / TILE + K*N / TILE reads).
        KernelRule {
            rule_id: "gemm-tiling".to_string(),
            gpu_concept: "shared-memory tiling".to_string(),
            ref_shape: "// naive GEMM: global memory reads dominate\nfn gemm_naive(a: &[[f32; K]; M], b: &[[f32; N]; K], c: &mut [[f32; N]; M]) {\n    for m in 0..M {\n        for n in 0..N {\n            let mut acc = 0.0;\n            for k in 0..K { acc += a[m][k] * b[k][n]; }\n            c[m][n] = acc;\n        }\n    }\n}".to_string(),
            opt_pattern: "// tiled GEMM: load tile to shared memory once, reuse across outputs\nfn gemm_tiled(a: &[[f32; K]; M], b: &[[f32; N]; K], c: &mut [[f32; N]; M]) {\n    const TILE: usize = 16;\n    for mt in (0..M).step_by(TILE) {\n        for nt in (0..N).step_by(TILE) {\n            for kt in (0..K).step_by(TILE) {\n                // load a[mt..mt+TILE][kt..kt+TILE] + b[kt..kt+TILE][nt..nt+TILE] into shared mem\n                for m in mt..(mt + TILE).min(M) {\n                    for n in nt..(nt + TILE).min(N) {\n                        let mut acc = 0.0;\n                        for k in kt..(kt + TILE).min(K) { acc += a[m][k] * b[k][n]; }\n                        c[m][n] += acc;\n                    }\n                }\n            }\n        }\n    }\n}".to_string(),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HealEntry;

    #[test]
    fn seed_corpus_round_trips() {
        let corpus = build_kernel_seed_corpus();
        assert!(corpus.verify());
        let frozen = corpus.freeze().unwrap();
        let thawed = Corpus::<KernelOptDomain>::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn seed_corpus_has_two_placeholder_entries() {
        let corpus = build_kernel_seed_corpus();
        assert_eq!(corpus.entries.len(), 2);
    }

    #[test]
    fn seed_corpus_rule_ids_are_unique() {
        let corpus = build_kernel_seed_corpus();
        let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
    }

    #[test]
    fn tamper_detected() {
        let corpus = build_kernel_seed_corpus();
        let mut tampered = corpus.clone();
        tampered.entries[0].opt_pattern = "// tampered".to_string();
        assert!(!tampered.verify());
    }

    #[test]
    fn heal_entry_accessors_return_correct_fields() {
        let corpus = build_kernel_seed_corpus();
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "matvec-coalesce")
            .expect("matvec-coalesce should be in seed corpus");
        assert_eq!(entry.rule_id(), "matvec-coalesce");
        assert!(!entry.buggy_code().is_empty());
        assert!(!entry.fixed_code().is_empty());
    }

    #[test]
    fn heal_domain_build_corpus_matches_seed() {
        use crate::domain::HealDomain;
        let via_trait = KernelOptDomain::build_corpus();
        let via_seed = build_kernel_seed_corpus().entries;
        assert_eq!(via_trait.len(), via_seed.len());
        assert_eq!(via_trait[0].rule_id, via_seed[0].rule_id);
    }

    #[test]
    #[cfg(not(feature = "kernel_verify"))]
    fn validate_parses_valid_code() {
        use crate::domain::HealDomain;
        let verdict = KernelOptDomain::validate("matvec-coalesce", "fn foo() -> i32 { 42 }");
        assert!(verdict.parses);
    }

    #[test]
    #[cfg(not(feature = "kernel_verify"))]
    fn validate_rejects_garbage() {
        use crate::domain::HealDomain;
        let verdict = KernelOptDomain::validate("matvec-coalesce", "not valid rust {");
        assert!(!verdict.parses);
    }
}

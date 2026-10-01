//! BLAKE3-deterministic modelless embedder for (`lint_name`, code) tuples.
//!
//! Produces a fixed-size `[f32; 8]` latent direction vector from arbitrary
//! (`lint_name`, code) input using only BLAKE3 hashing + deterministic
//! byte-folding. No weights, no training, no GPU.
//!
//! ## Pattern source
//!
//! Borrows the `ModellessEmbedder` pattern from riir-rag (riir-ai): BLAKE3 +
//! deterministic projection + bounded output. Reimplemented locally (~30 LOC)
//! because riir-refine cannot depend on riir-ai (boundary rule).
//!
//! ## Properties
//!
//! - **Deterministic**: same (`lint_name`, code) → identical `[f32; 8]` bits.
//! - **Bounded**: every component ∈ \[-1.0, 1.0\] (via u32 → normalized f32).
//! - **Diverse**: BLAKE3's avalanche property ensures distinct inputs produce
//!   near-orthogonal embeddings (good for dot-product retrieval).
//! - **Alloc-free**: no heap allocation; writes into a caller-provided array.

use blake3::Hasher;

/// The latent embedding dimension. Matches riir-agents' `ModellessReasoner`
/// direction-vector size (8-dim) so the `dot + sigmoid` scoring pattern is
/// directly applicable.
pub const EMBED_DIM: usize = 8;

/// A fixed-size latent direction vector.
pub type Direction = [f32; EMBED_DIM];

/// Compute a BLAKE3-deterministic 8-dim embedding of (`lint_name`, code).
///
/// The 32-byte BLAKE3 hash is split into 8 little-endian `u32` values, each
/// normalized to \[-1.0, 1.0\] via `((u32 as f64) / (u32::MAX as f64)) * 2.0 - 1.0`
/// (using f64 for the conversion to avoid f32 precision loss on large u32,
/// then casting to f32 for storage).
///
/// Writes into `out` to avoid allocation. `out` must be exactly `EMBED_DIM`
/// elements.
#[inline]
pub fn embed_into(lint_name: &str, code: &str, out: &mut Direction) {
    let mut hasher = Hasher::new();
    hasher.update(lint_name.as_bytes());
    hasher.update(b"\x00");
    hasher.update(code.as_bytes());
    let hash = hasher.finalize();
    let hb = hash.as_bytes(); // 32 bytes = 8 × u32

    for (i, slot) in out.iter_mut().enumerate() {
        let off = i * 4;
        let u = u32::from_le_bytes([hb[off], hb[off + 1], hb[off + 2], hb[off + 3]]);
        // Normalize u32 [0, 2^32-1] → f32 [-1, 1].
        // f64 intermediate avoids precision loss: u32::MAX as f32 rounds up
        // (to 4294967296.0), which would push the ratio slightly > 1.0. f64 is exact.
        // The final f64→f32 cast is intentional (the result fits in [-1, 1]).
        #[allow(clippy::cast_possible_truncation)]
        {
            *slot = ((f64::from(u) / f64::from(u32::MAX)) * 2.0 - 1.0) as f32;
        }
    }
}

/// Convenience: compute a fresh `[f32; 8]` embedding (allocates the array on
/// the stack — no heap allocation).
#[inline]
#[must_use]
pub fn embed(lint_name: &str, code: &str) -> Direction {
    let mut dir = [0.0f32; EMBED_DIM];
    embed_into(lint_name, code, &mut dir);
    dir
}

/// Compute a BLAKE3-deterministic 8-dim embedding of a (lint, buggy, fixed)
/// triple — used to embed past fix trajectories for the memory's latent axis.
///
/// The triple is hashed as `lint_name \0 buggy_code \0 fixed_code` so each
/// component is delimiter-separated (no ambiguity from concatenation).
#[inline]
pub fn embed_fix_into(lint_name: &str, buggy_code: &str, fixed_code: &str, out: &mut Direction) {
    let mut hasher = Hasher::new();
    hasher.update(lint_name.as_bytes());
    hasher.update(b"\x00");
    hasher.update(buggy_code.as_bytes());
    hasher.update(b"\x00");
    hasher.update(fixed_code.as_bytes());
    let hash = hasher.finalize();
    let hb = hash.as_bytes();

    for (i, slot) in out.iter_mut().enumerate() {
        let off = i * 4;
        let u = u32::from_le_bytes([hb[off], hb[off + 1], hb[off + 2], hb[off + 3]]);
        #[allow(clippy::cast_possible_truncation)]
        {
            *slot = ((f64::from(u) / f64::from(u32::MAX)) * 2.0 - 1.0) as f32;
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // tests check bit-identical determinism
mod tests {
    use super::*;

    #[test]
    fn embed_is_deterministic() {
        // G1: same input → identical bits.
        let a = embed("needless_return", "fn foo() { return 42; }");
        let b = embed("needless_return", "fn foo() { return 42; }");
        assert_eq!(a, b, "same input must produce identical embedding");
    }

    #[test]
    fn embed_different_input_different_output() {
        let a = embed("needless_return", "return 42;");
        let b = embed("redundant_closure", "|x| f(x)");
        assert_ne!(a, b, "different inputs should produce different embeddings");
    }

    #[test]
    fn embed_same_lint_different_code_different_output() {
        let a = embed("needless_return", "return 42;");
        let b = embed("needless_return", "return 99;");
        assert_ne!(a, b, "same lint + different code should differ");
    }

    #[test]
    fn embed_components_in_valid_range() {
        for (lint, code) in &[
            ("needless_return", "return 42;"),
            ("redundant_closure", "|x| f(x)"),
            ("unused_variable", "let x = 1;"),
        ] {
            let dir = embed(lint, code);
            for &v in &dir {
                assert!(
                    (-1.0..=1.0).contains(&v),
                    "component {v} out of [-1,1] for {lint}/{code}"
                );
                assert!(!v.is_nan(), "NaN component for {lint}/{code}");
            }
        }
    }

    #[test]
    fn embed_into_matches_embed() {
        let direct = embed("needless_return", "return 42;");
        let mut into_buf = [0.0f32; EMBED_DIM];
        embed_into("needless_return", "return 42;", &mut into_buf);
        assert_eq!(direct, into_buf);
    }

    #[test]
    fn embed_fix_distinguishes_fix_variants() {
        // Same lint + buggy, different fix → different embedding (the fix is
        // part of the hashed tuple, so distinct fixes embed distinctly).
        let mut a = [0.0f32; EMBED_DIM];
        let mut b = [0.0f32; EMBED_DIM];
        embed_fix_into("needless_return", "return 42;", "42", &mut a);
        embed_fix_into("needless_return", "return 42;", "return 42;", &mut b);
        assert_ne!(a, b, "different fixes must embed distinctly");
    }
}

//! L4 — the `LoRA` fallback stage of the heal pipeline (Plan 336 T4.4 / T5.2).
//!
//! L0-L3 (corpus + drafter + pruner + memory/evolution + ruliology) are
//! modelless: hash lookups, template drafting, syntax validation, latent
//! scoring. They are microseconds-fast and they generalize only as far as the
//! corpus reaches. L4 is the **conservative last resort** for inputs those
//! layers cannot serve at all — a generative model (the Plan 336 Q+V `LoRA` over
//! Ternary-Bonsai-27B) that can transform unseen code.
//!
//! ## Why this is a seam, not an implementation
//!
//! The adapter itself lives in riir-train (it needs the 7 GB GGUF, a GPU, and
//! ~48 s per fix — see riir-train Bench 464). This module owns only the
//! **composition contract**: when L4 is consulted, what it must return, and
//! what the pipeline is allowed to do with the answer. That split is what lets
//! the L0-L3 → L4 handoff be tested and benchmarked here, in milliseconds, with
//! a test double — while the adapter's own quality is gated separately by
//! riir-train's G1.
//!
//! Before this seam existed, Plan 336's G4 (composition) and T5.2 (wire the
//! fallback) each named the other as its prerequisite: G4 could not be measured
//! without the wiring, and the wiring was gated on G4 passing. Splitting the
//! contract from the adapter breaks that cycle.
//!
//! ## The conservative contract
//!
//! Four invariants, all enforced by [`HealPipeline::heal`] and pinned by
//! `tests/g4_l4_composition.rs`:
//!
//! 1. **Never consulted on a modelless hit.** If L0-L3 produce a fix that
//!    parses, L4 is not called at all — not called and discarded, *not called*.
//!    At ~48 s per invocation against ~16 µs for the retriever, a spurious call
//!    is a 3-million-fold regression.
//! 2. **Consulted only on a genuine miss.** No candidates at all, or a
//!    top-scoring candidate that fails the L2 pruner.
//! 3. **Never degrades a modelless answer.** An L4 fix that does not parse is
//!    discarded and the modelless result (even a non-parsing one) stands. L4
//!    can only turn `None` into `Some`, or a non-parsing `Some` into a parsing
//!    one.
//! 4. **Absent by default.** With no fallback installed the pipeline is
//!    byte-identical to the L0-L3 path — the stage adds one `Option::is_some`
//!    check on the miss path only.
//!
//! [`HealPipeline::heal`]: crate::self_evolve::HealPipeline::heal

/// A generative fix source consulted when the modelless L0-L3 path misses.
///
/// Implementors are expected to be **slow** (the reference implementation is a
/// 27B ternary model at ~48 s/fix) — the pipeline's contract is written to call
/// this as rarely as possible.
///
/// `&mut self` rather than `&self`: real implementations own a KV cache and a
/// decode scratch buffer, and threading a lock through the hot path to please a
/// shared-reference signature would cost more than it buys. The pipeline holds
/// the fallback exclusively.
pub trait L4Fallback {
    /// Generate a fix for `buggy_code` under `rule_id`.
    ///
    /// Returns `None` when the model declines, produces nothing, or fails —
    /// declining is always safe, since the pipeline keeps whatever the
    /// modelless path produced.
    ///
    /// Implementations MUST NOT panic on malformed input: a fallback that
    /// panics takes down a heal that would otherwise have returned a
    /// non-parsing but usable modelless answer.
    fn generate_fix(&mut self, rule_id: &str, buggy_code: &str) -> Option<String>;

    /// Human-readable name for logs and bench output (e.g. `"bonsai-27b-qv-lora"`).
    ///
    /// Borrowed from `self`, not `&'static str`: a real adapter identifies
    /// itself by checkpoint (`"bonsai-27b-qv-lora@b2f81622"`), which it must
    /// build at construction time and store. Pinning this to `'static` would
    /// force every such impl to leak a `String`.
    #[allow(clippy::unnecessary_literal_bound)]
    fn name(&self) -> &str {
        "unnamed-l4"
    }
}

/// A fallback that always declines.
///
/// Useful as an explicit "L4 is installed but disabled" marker, and as the
/// control arm in composition benchmarks: installing `NullL4` exercises the
/// entire miss path (the `is_some` check, the call, the discard) while holding
/// the returned fix constant, which isolates the seam's own cost from the
/// model's.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullL4;

impl L4Fallback for NullL4 {
    fn generate_fix(&mut self, _rule_id: &str, _buggy_code: &str) -> Option<String> {
        None
    }

    #[allow(clippy::unnecessary_literal_bound)] // signature is fixed by the trait
    fn name(&self) -> &str {
        "null-l4"
    }
}

#[cfg(test)]
mod tests {
    use super::{L4Fallback, NullL4};

    #[test]
    fn null_l4_declines_and_names_itself() {
        let mut null = NullL4;
        assert!(null.generate_fix("needless_return", "fn f() -> i32 { return 1; }").is_none());
        assert_eq!(null.name(), "null-l4");
    }

    #[test]
    fn default_name_is_provided_without_an_override() {
        struct Bare;
        impl L4Fallback for Bare {
            fn generate_fix(&mut self, _r: &str, _c: &str) -> Option<String> {
                Some("fixed".to_string())
            }
        }
        assert_eq!(Bare.name(), "unnamed-l4");
    }
}

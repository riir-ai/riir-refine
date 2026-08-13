//! Kernel rule resolution verifier — Phase 3: compile-only stub.
//!
//! Behind the `kernel_verify` feature gate (opt-in, default-off). This is a
//! **compile-only stub** — full numeric compare vs reference (Proposal 012
//! §Phase 0) is deferred. The stub runs the shared `RustAnalyzerPruner` to
//! check that the optimized pattern parses, then reports `compiles = true`.
//!
//! ## What ships now vs deferred
//!
//! | Capability | Ships now | Deferred |
//! |---|---|---|
//! | Syntax parse check | ✅ (via `RustAnalyzerPruner`) | |
//! | `CubeCL` compile check | | ✅ (needs `CubeCL` on PATH) |
//! | Numeric compare vs reference | | ✅ (within tolerance) |
//! | Perf benchmark | | ✅ (cycles / byte) |
//!
//! ## Why the stub is honest
//!
//! The scaffold's corpus entries are Rust-shaped pseudocode (not real
//! CUDA/WGSL kernels), so the shared `RustAnalyzerPruner` can validate them.
//! When the real corpus lands (real kernel source), the validator upgrades to
//! `CubeCL` compile + numeric compare. The trait signature stays the same —
//! only the validator's internals change.

/// The result of a kernel rule verification.
///
/// **Stub**: `compiles` currently reflects only the syntax parse check.
/// Full numeric compare vs reference is deferred (Proposal 012 §Phase 0).
#[derive(Debug, Clone)]
pub struct KernelVerifyResult {
    /// Whether the optimized pattern compiled (stub: parsed as valid Rust).
    ///
    /// `true` = the pattern is plausible (parses in the stub; compiles +
    /// numerically matches reference in the full validator).
    /// `false` = parse error (stub), compile error, or numeric mismatch.
    pub compiles: bool,
    /// Whether the validator ran.
    ///
    /// `false` if the validator couldn't run (I/O error, missing `CubeCL` in
    /// the full version). The stub always returns `ran = true` when the
    /// parse check executed.
    pub ran: bool,
    /// Error message if `ran` is false or `compiles` is false.
    pub error: Option<String>,
}

impl KernelVerifyResult {
    /// Construct a "did not run" result (I/O error, missing toolchain, etc.).
    #[allow(dead_code)] // used when the full CubeCL validator lands
    fn did_not_run(error: impl Into<String>) -> Self {
        Self {
            compiles: false,
            ran: false,
            error: Some(error.into()),
        }
    }

    /// Construct a "ran successfully" result.
    fn ran_with(compiles: bool) -> Self {
        Self {
            compiles,
            ran: true,
            error: None,
        }
    }
}

/// Verifies that drafted kernel fixes compile.
///
/// **Stub**: uses the shared `RustAnalyzerPruner` for syntax validation.
/// Full `CubeCL` compile + numeric compare vs reference is deferred (Proposal
/// 012 §Phase 0).
///
/// ## What this does NOT verify
///
/// Per Proposal 012 §Phase 0: this verifier does NOT compile the kernel via
/// `CubeCL`, does NOT run it on a GPU, and does NOT compare output against the
/// reference numerically. A `compiles = true` result means the pattern
/// parses as valid Rust — it does NOT mean the kernel is correct or faster.
pub struct KernelVerifier {
    // Future fields (deferred):
    // - cubecl_runtime: Option<CubeclRuntime> (Warm/Cold tier compile)
    // - reference_runner: Option<ReferenceRunner> (Cold tier numeric compare)
    // - tolerance: f32 (numeric compare epsilon)
}

impl Default for KernelVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl KernelVerifier {
    /// Create a verifier with the stub configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    /// Verify that `code` compiles (stub: parses as valid Rust).
    ///
    /// # Returns
    ///
    /// - `compiles = true` → the pattern parses (stub) / compiles (full)
    /// - `compiles = false` → parse error (stub) / compile error (full)
    /// - `ran = false` → validator couldn't run (check `error`)
    ///
    /// The `rule_id` parameter is accepted for trait conformance but ignored
    /// in the stub — the full validator may use it to look up the reference
    /// kernel for numeric compare.
    #[must_use]
    pub fn verify(&self, code: &str, rule_id: &str) -> KernelVerifyResult {
        // Stub: delegate to the shared RustAnalyzerPruner for syntax check.
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        let result = pruner.validate(code, rule_id);
        if !result.parses {
            return KernelVerifyResult::ran_with(false);
        }
        KernelVerifyResult::ran_with(true)
    }

    /// Check if the full validator toolchain is available.
    ///
    /// **Stub**: always returns `false` (the full `CubeCL` validator isn't
    /// wired yet). The stub's syntax check is always available — this method
    /// specifically asks about the deferred full validator.
    #[must_use]
    pub fn is_full_validator_available(&self) -> bool {
        // Stub: full validator (CubeCL compile + numeric compare) not wired.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn did_not_run_sets_fields_correctly() {
        let r = KernelVerifyResult::did_not_run("missing CubeCL");
        assert!(!r.compiles);
        assert!(!r.ran);
        assert_eq!(r.error, Some("missing CubeCL".to_string()));
    }

    #[test]
    fn ran_with_compiles_true_sets_fields_correctly() {
        let r = KernelVerifyResult::ran_with(true);
        assert!(r.compiles);
        assert!(r.ran);
        assert_eq!(r.error, None);
    }

    #[test]
    fn ran_with_compiles_false_sets_fields_correctly() {
        let r = KernelVerifyResult::ran_with(false);
        assert!(!r.compiles);
        assert!(r.ran);
    }

    #[test]
    fn new_uses_default_config() {
        let _ = KernelVerifier::new();
    }

    #[test]
    fn default_trait_works() {
        let _ = KernelVerifier::default();
    }

    #[test]
    fn verify_valid_code_compiles() {
        let v = KernelVerifier::new();
        let result = v.verify("fn foo() -> i32 { 42 }", "matvec-coalesce");
        assert!(result.ran);
        assert!(result.compiles);
    }

    #[test]
    fn verify_garbage_does_not_compile() {
        let v = KernelVerifier::new();
        let result = v.verify("not valid rust {", "matvec-coalesce");
        assert!(result.ran);
        assert!(!result.compiles);
    }

    #[test]
    fn verify_deterministic_same_code_same_result() {
        let v = KernelVerifier::new();
        let code = "fn id(x: i32) -> i32 { x }";
        let r1 = v.verify(code, "matvec-coalesce");
        let r2 = v.verify(code, "matvec-coalesce");
        assert_eq!(r1.ran, r2.ran);
        assert_eq!(r1.compiles, r2.compiles);
    }

    #[test]
    fn is_full_validator_available_returns_false_in_stub() {
        let v = KernelVerifier::new();
        assert!(!v.is_full_validator_available());
    }
}

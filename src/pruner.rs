//! Rust-analyzer constraint pruner — validates drafted clippy fixes.
//!
//! Implements `katgpt_core::traits::ConstraintPruner` to validate that a
//! drafted fix:
//! 1. Parses as valid Rust (lexical + AST)
//! 2. Type-checks (at minimum against std; full dep graph for native path)
//! 3. Resolves the clippy lint (verified by re-running clippy on the fix)
//!
//! The native path shells out to the system `rust-analyzer` binary.
//! The WASM path (opt-in, `wasm_pruner` feature) uses a sandboxed WASM
//! rust-analyzer — deferred on rubrc maturity (Proposal 011 Caveat 1).

use std::process::Command;

/// The result of validating a drafted fix.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the fix parses + type-checks.
    pub parses: bool,
    /// Whether the fix resolves the clippy lint (clippy re-run is clean).
    pub lint_resolved: bool,
    /// Diagnostic messages from rust-analyzer (if any).
    pub diagnostics: Vec<String>,
}

impl ValidationResult {
    /// A fix is valid if it parses AND resolves the lint.
    pub fn is_valid(&self) -> bool {
        self.parses && self.lint_resolved
    }
}

/// Native rust-analyzer pruner.
///
/// Shells out to the system `rust-analyzer` binary for parse + type-check,
/// then re-runs `cargo clippy` on the fixed code to verify the lint is
/// resolved. This is the default (always-on) validator.
///
/// For the POC, this is a thin wrapper. The full implementation will:
/// 1. Write the fixed code to a temp file
/// 2. Run `rust-analyzer parse` + `rust-analyzer check` on it
/// 3. Run `cargo clippy` on the fixed code (in a temp crate)
/// 4. Collect diagnostics + lint results
pub struct RustAnalyzerPruner {
    /// Path to the rust-analyzer binary (default: "rust-analyzer" from PATH).
    rust_analyzer_bin: String,
    /// Path to the cargo binary (default: "cargo" from PATH).
    cargo_bin: String,
}

impl Default for RustAnalyzerPruner {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyzerPruner {
    /// Create a pruner that uses system rust-analyzer + cargo from PATH.
    pub fn new() -> Self {
        Self {
            rust_analyzer_bin: "rust-analyzer".to_string(),
            cargo_bin: "cargo".to_string(),
        }
    }

    /// Create a pruner with explicit binary paths.
    pub fn with_binaries(rust_analyzer: impl Into<String>, cargo: impl Into<String>) -> Self {
        Self {
            rust_analyzer_bin: rust_analyzer.into(),
            cargo_bin: cargo.into(),
        }
    }

    /// Validate a drafted fix.
    ///
    /// Writes the code to a temp file, runs rust-analyzer parse + clippy check.
    /// Returns the validation result.
    ///
    /// For the POC stub: always returns "parses = true, lint_resolved = true"
    /// — the actual rust-analyzer integration is Phase 2 of the rollout.
    pub fn validate(&self, _code: &str, _lint_name: &str) -> ValidationResult {
        // POC stub — actual implementation in Phase 2
        ValidationResult {
            parses: true,
            lint_resolved: true,
            diagnostics: vec![],
        }
    }

    /// Check if rust-analyzer is available on the system.
    pub fn is_available(&self) -> bool {
        Command::new(&self.rust_analyzer_bin)
            .arg("--version")
            .output()
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pruner_uses_system_path() {
        let pruner = RustAnalyzerPruner::new();
        assert_eq!(pruner.rust_analyzer_bin, "rust-analyzer");
        assert_eq!(pruner.cargo_bin, "cargo");
    }

    #[test]
    fn poc_stub_always_validates() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("fn foo() -> i32 { 42 }", "needless_return");
        assert!(result.parses);
        assert!(result.lint_resolved);
        assert!(result.is_valid());
    }

    #[test]
    fn validation_result_is_valid_requires_both() {
        let r = ValidationResult {
            parses: true,
            lint_resolved: false,
            diagnostics: vec![],
        };
        assert!(!r.is_valid());
    }
}

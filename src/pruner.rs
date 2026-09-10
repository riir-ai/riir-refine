//! Rust-analyzer constraint pruner — validates drafted clippy fixes.
//!
//! Implements the L2 validation layer of the clippy-healing pipeline.
//! The pruner validates that a drafted fix:
//! 1. Parses as valid Rust syntax (via `syn`, in-process — always checked)
//! 2. Resolves the clippy lint (via `cargo clippy` re-run — feature-gated `clippy_verify`)
//!
//! ## Honesty contract
//!
//! - `parses = true` → the code actually parsed with `syn` (real syntax check)
//! - `lint_resolved = true` (via [`validate`](Self::validate)) → **known limitation**:
//!   always `true` because the fast path doesn't shell out to `cargo clippy`.
//!   The syntax check is the load-bearing validation.
//! - `lint_resolved = true` (via [`validate_with_clippy`](Self::validate_with_clippy))
//!   → **truthful**: cargo clippy confirmed the lint did NOT fire. Requires
//!   the `clippy_verify` feature.
//!
//! ## Modelless-first
//!
//! `syn` is a pure Rust parser with zero ML — no training, no backprop.
//! `cargo clippy` is a deterministic rules engine — no ML, no training.
//! Both satisfy the modelless-first mandate for the L2 layer.

use std::process::Command;
use syn::{File, Item};

/// The result of validating a drafted fix.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the fix parses as valid Rust syntax.
    ///
    /// **Truthful**: this is `true` only when `syn::parse_file` (or the
    /// snippet-wrapping fallback) succeeds.
    pub parses: bool,
    /// Whether the fix resolves the clippy lint.
    ///
    /// **Fast path** ([`validate`](Self::validate)): always `true` — the fast
    /// path doesn't run `cargo clippy`. The syntax check is the load-bearing
    /// validation.
    ///
    /// **Slow path** ([`validate_with_clippy`](Self::validate_with_clippy),
    /// feature `clippy_verify`): truthful — `true` only when cargo clippy
    /// confirms the lint did NOT fire on the fixed code.
    pub lint_resolved: bool,
    /// Diagnostic messages from the parser (if any).
    ///
    /// Empty on success. Bounded by the number of parse errors (typically 1).
    pub diagnostics: Vec<String>,
}

impl ValidationResult {
    /// A fix is valid if it parses AND (claims to) resolve the lint.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.parses && self.lint_resolved
    }
}

/// Native rust-analyzer pruner.
///
/// Validates drafted clippy fixes using `syn` for in-process syntax parsing.
/// The `rust_analyzer_bin` / `cargo_bin` fields exist for the Phase 3
/// integration (temp-crate scaffolding + `cargo clippy` re-run, wired in
/// [`validate_with_clippy`](Self::validate_with_clippy) behind `clippy_verify`).
#[derive(Debug, Clone)]
pub struct RustAnalyzerPruner {
    /// Path to the rust-analyzer binary (reserved for future rust-analyzer AST integration).
    rust_analyzer_bin: String,
    /// Path to the cargo binary (reserved for Phase 3 cargo clippy integration).
    #[allow(dead_code, reason = "reserved for Phase 3 cargo clippy integration")]
    cargo_bin: String,
}

impl Default for RustAnalyzerPruner {
    fn default() -> Self {
        Self::new()
    }
}

impl RustAnalyzerPruner {
    /// Create a pruner that uses system rust-analyzer + cargo from PATH.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rust_analyzer_bin: "rust-analyzer".to_string(),
            cargo_bin: "cargo".to_string(),
        }
    }

    /// Create a pruner with explicit binary paths.
    #[must_use]
    pub fn with_binaries(rust_analyzer: impl Into<String>, cargo: impl Into<String>) -> Self {
        Self {
            rust_analyzer_bin: rust_analyzer.into(),
            cargo_bin: cargo.into(),
        }
    }

    /// Validate a drafted fix.
    ///
    /// **Syntax check (always):** parses the code with `syn`. Corpus entries
    /// are often single items (fns, structs) or expressions — the pruner
    /// tries `parse_file` first, then falls back to wrapping the snippet in
    /// `fn _wrap() { <code> }` for expression/statement snippets.
    ///
    /// **Lint resolution:** always returns `lint_resolved = true` — this is
    /// a documented limitation. For truthful lint resolution, use
    /// [`validate_with_clippy`](Self::validate_with_clippy) (requires the
    /// `clippy_verify` feature, ~130ms/verify).
    #[must_use]
    pub fn validate(&self, code: &str, _lint_name: &str) -> ValidationResult {
        match Self::parse_rust(code) {
            Ok(()) => ValidationResult {
                parses: true,
                lint_resolved: true, // Fast path — see validate_with_clippy for truthful lint_resolved
                diagnostics: vec![],
            },
            Err(msg) => ValidationResult {
                parses: false,
                lint_resolved: false,
                diagnostics: vec![msg],
            },
        }
    }

    /// Parse Rust code, trying multiple strategies for snippet robustness.
    ///
    /// 1. Try `syn::parse_file(code)` — works for full files + single items.
    /// 2. If that fails, wrap in `fn _wrap() { <code> }` for statements/expressions.
    /// 3. If both fail, return the first error (most informative).
    fn parse_rust(code: &str) -> Result<(), String> {
        // Strategy 1: parse as a full file. This handles the common corpus
        // case where fixed_code is a complete item (fn, struct, impl, etc.).
        match syn::parse_file(code) {
            Ok(_file) => Ok(()),
            Err(file_err) => {
                // Strategy 2: wrap in a fn body for expression/statement snippets.
                // This handles cases like `42`, `return x + 1;`, `let y = foo();`.
                // The boundary newlines keep the wrapper braces out of a
                // trailing line-comment when the snippet ends with one.
                let wrapped = format!("fn _wrap() {{\n{code}\n}}");
                match syn::parse_file(&wrapped) {
                    Ok(_wrapped_file) => Ok(()),
                    // Both strategies failed — return the original (unwrapped)
                    // error as it's the most directly informative for the user.
                    Err(_wrapped_err) => {
                        // Filter the wrapped items: if the wrapped version parsed
                        // items beyond the wrapper fn, the original was a list of
                        // items that just wasn't valid as a complete file.
                        // Check if the code parses as a single item.
                        if let Ok(_item) = syn::parse_str::<Item>(code) {
                            return Ok(());
                        }
                        Err(Self::format_syn_error(&file_err))
                    }
                }
            }
        }
    }

    /// Format a `syn::Error` into a human-readable diagnostic string.
    ///
    /// `syn::Error`'s Display includes the error message; we don't pull
    /// `span-locations` from `proc-macro2` (would add a feature flag for
    /// marginal diagnostic gain — the error message itself is the value).
    fn format_syn_error(err: &syn::Error) -> String {
        format!("parse error: {err}")
    }

    /// Validate a drafted fix with BOTH syntax check AND real cargo clippy re-run.
    ///
    /// This is the Phase 3 thorough validation path. It first runs the fast
    /// `syn`-based syntax check (always-on, microseconds), then — if the code
    /// parses — spawns `cargo clippy` on a temp crate to verify the target lint
    /// is actually resolved (seconds).
    ///
    /// The `lint_resolved` field in the returned [`ValidationResult`] is
    /// **truthful** when this method is used: it's `true` only when cargo
    /// clippy confirms the lint did NOT fire on the fixed code.
    ///
    /// # Performance
    ///
    /// This method is **slow** (seconds per invocation). Use the regular
    /// [`validate`](Self::validate) for the fast path (syntax only, microseconds).
    ///
    /// # Feature gate
    ///
    /// Requires the `clippy_verify` feature. Not available in default builds.
    #[cfg(feature = "clippy_verify")]
    #[must_use]
    pub fn validate_with_clippy(&self, code: &str, lint_name: &str) -> ValidationResult {
        // Step 1: fast syntax check (always-on).
        let mut result = self.validate(code, lint_name);

        // Step 2: if it parses, run cargo clippy for real lint resolution.
        if result.parses {
            let verifier = crate::domains::clippy_lints::verify::ClippyVerifier::new();
            let clippy_result = verifier.verify(code, lint_name);
            result.lint_resolved = clippy_result.lint_resolved;

            // If clippy didn't run (cargo missing, I/O error), note it.
            if !clippy_result.ran {
                if let Some(err) = clippy_result.error {
                    result
                        .diagnostics
                        .push(format!("clippy_verify did not run: {err}"));
                }
            } else if !clippy_result.lint_resolved {
                // Lint still present — add the present lints as diagnostics.
                result.diagnostics.push(format!(
                    "lint not resolved: clippy still reports {}",
                    clippy_result
                        .present_lints
                        .iter()
                        .filter(|l| l.starts_with("clippy::"))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        result
    }

    /// Check if rust-analyzer is available on the system (reserved for Phase 3).
    #[must_use]
    pub fn is_available(&self) -> bool {
        Command::new(&self.rust_analyzer_bin)
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Parse the code and return the [`syn::File`] AST (for advanced consumers).
    ///
    /// Returns `Ok(File)` if the code parses as a full file, `Err` otherwise.
    /// Does NOT apply the snippet-wrapping fallback — use [`validate`](Self::validate) for that.
    ///
    /// # Errors
    ///
    /// Returns `syn::Error` if the code does not parse as a complete Rust file.
    pub fn parse_file_ast(code: &str) -> Result<File, syn::Error> {
        syn::parse_file(code)
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

    // ── G1: real syntax validation ─────────────────────────────────────────

    #[test]
    fn valid_function_parses() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("fn foo() -> i32 { 42 }", "needless_return");
        assert!(result.parses, "valid fn should parse");
        assert!(result.is_valid());
        assert!(result.diagnostics.is_empty(), "no diagnostics on success");
    }

    #[test]
    fn valid_struct_parses() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("struct Foo { x: i32, y: String }", "new_without_default");
        assert!(result.parses, "valid struct should parse");
    }

    #[test]
    fn valid_impl_parses() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate(
            "impl Foo { fn bar(&self) -> i32 { 42 } }",
            "new_without_default",
        );
        assert!(result.parses, "valid impl should parse");
    }

    #[test]
    fn expression_snippet_parses_via_wrap() {
        // Expression-only snippet — should parse via the fn-wrap fallback.
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("42", "needless_return");
        assert!(result.parses, "expression `42` should parse via wrap");
    }

    #[test]
    fn statement_snippet_parses_via_wrap() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("let x = 1 + 2;", "unnecessary_mut_passed");
        assert!(result.parses, "statement should parse via wrap");
    }

    #[test]
    fn invalid_syntax_rejected() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("fn foo( -> {", "needless_return");
        assert!(!result.parses, "broken syntax must be rejected");
        assert!(!result.diagnostics.is_empty(), "must have a diagnostic");
        assert!(
            result.diagnostics[0].contains("parse error"),
            "diagnostic should include error context: {}",
            result.diagnostics[0]
        );
    }

    #[test]
    fn unclosed_brace_rejected() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("fn foo() {", "needless_return");
        assert!(!result.parses, "unclosed brace must be rejected");
    }

    #[test]
    fn garbage_input_rejected() {
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("this is not rust at all!!!", "needless_return");
        assert!(!result.parses, "garbage must be rejected");
    }

    #[test]
    fn empty_string_parses() {
        // Empty string parses as an empty file — that's valid Rust (just useless).
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("", "needless_return");
        assert!(result.parses, "empty string is valid (empty file)");
    }

    // ── G3: validation result semantics ────────────────────────────────────

    #[test]
    fn validation_result_is_valid_requires_both() {
        let r = ValidationResult {
            parses: true,
            lint_resolved: false,
            diagnostics: vec![],
        };
        assert!(!r.is_valid());
    }

    #[test]
    fn invalid_syntax_lint_resolved_is_false() {
        // When parses=false, lint_resolved should also be false — the code is broken.
        let pruner = RustAnalyzerPruner::new();
        let result = pruner.validate("fn broken( ->", "needless_return");
        assert!(!result.parses);
        assert!(!result.lint_resolved, "broken code cannot resolve any lint");
    }

    // ── parse_file_ast: direct AST access ──────────────────────────────────

    #[test]
    fn parse_file_ast_returns_file_on_success() {
        let ast = RustAnalyzerPruner::parse_file_ast("fn main() {}").unwrap();
        assert_eq!(ast.items.len(), 1);
    }

    #[test]
    fn parse_file_ast_returns_err_on_bad_syntax() {
        let result = RustAnalyzerPruner::parse_file_ast("fn broken( ->");
        assert!(result.is_err());
    }

    // ── validate_with_clippy (Phase 3 — feature-gated) ────────────────────
    // These tests verify the composition: syntax check + cargo clippy.
    // They run real cargo clippy (slow, seconds) so they're #[ignore]'d.

    #[cfg(feature = "clippy_verify")]
    #[test]
    #[ignore = "slow: runs real cargo clippy (seconds)"]
    fn validate_with_clippy_fixed_code_is_truthful() {
        // G1 for Phase 3: the lint_resolved field is now TRUTHFUL when
        // validate_with_clippy is used. Fixed code → lint_resolved = true.
        let pruner = RustAnalyzerPruner::new();
        let fixed_code = "fn foo() -> i32 { 42 }";
        let result = pruner.validate_with_clippy(fixed_code, "needless_return");
        assert!(result.parses, "code must parse");
        assert!(
            result.lint_resolved,
            "fixed code should resolve needless_return"
        );
    }

    #[cfg(feature = "clippy_verify")]
    #[test]
    #[ignore = "slow: runs real cargo clippy (seconds)"]
    fn validate_with_clippy_buggy_code_is_truthful() {
        // G1 for Phase 3: buggy code → lint_resolved = false.
        // This is the load-bearing test — it proves lint_resolved is no
        // longer a lie. The old validate() always returned true here.
        let pruner = RustAnalyzerPruner::new();
        let buggy_code = "fn foo() -> i32 { return 42; }";
        let result = pruner.validate_with_clippy(buggy_code, "needless_return");
        assert!(result.parses, "buggy code is valid Rust, just has a lint");
        assert!(
            !result.lint_resolved,
            "buggy code should NOT resolve needless_return — the field is now truthful"
        );
    }
}

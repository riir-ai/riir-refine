//! The clippy lint healing domain.
//!
//! This is the first (and currently only) `HealDomain` impl. It owns:
//! - [`ClippyFixEntry`] — the corpus entry type (lint name, group, buggy, fixed)
//! - [`ClippyLintDomain`] — the `HealDomain` impl (`build_corpus`, validate)
//! - The seed corpus + verifier + per-lint AST proposers
//!
//! Phase 1 of Plan 001 (multi-domain framework refactor): this module is what
//! was `src/corpus.rs` + `src/seed_corpus.rs` + `src/clippy_verifier.rs` +
//! `src/proposers/` before the domain extraction. The domain-agnostic core
//! now reaches these types via the [`HealDomain`] trait + back-compat re-exports.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;
pub mod lint_level;
pub mod proposers;

#[cfg(feature = "clippy_verify")]
pub mod verify;

/// A single clippy lint fix example.
///
/// Domain classification (per AGENTS.md sync boundary rule):
/// - `lint_name` + `lint_group` = physical (categorical, deterministic)
/// - `buggy_code` + `fixed_code` = physical (raw bytes, bit-identical for replay)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippyFixEntry {
    /// The clippy lint name (e.g., "`needless_return`", "`redundant_closure`").
    pub lint_name: String,
    /// The lint group (style, correctness, suspicious, complexity, perf).
    pub lint_group: String,
    /// The buggy code that triggers the lint.
    pub buggy_code: String,
    /// The fixed code that resolves the lint.
    pub fixed_code: String,
}

impl HealEntry for ClippyFixEntry {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.lint_name
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.buggy_code
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.fixed_code
    }
}

/// The clippy lint healing domain — `HealDomain` impl for `clippy::*` lints.
///
/// The corpus entry type is [`ClippyFixEntry`]. The validator verdict is a
/// [`ClippyVerdict`] (behind `clippy_verify`) or a parse-only verdict otherwise.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClippyLintDomain;

/// The parse-only verdict (always available — no feature gate needed).
///
/// When `clippy_verify` is enabled, the richer `verify::ClippyVerifyResult`
/// is the domain's `Verdict` type instead.
#[derive(Debug, Clone)]
pub struct ClippyParseVerdict {
    /// Whether the fixed code parses as valid Rust.
    pub parses: bool,
}

#[cfg(not(feature = "clippy_verify"))]
impl HealDomain for ClippyLintDomain {
    type Entry = ClippyFixEntry;
    type Verdict = ClippyParseVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        let result = pruner.validate(proposed_fix, rule_id);
        ClippyParseVerdict {
            parses: result.parses,
        }
    }
}

#[cfg(feature = "clippy_verify")]
impl HealDomain for ClippyLintDomain {
    type Entry = ClippyFixEntry;
    type Verdict = verify::ClippyVerifyResult;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        verify::ClippyVerifier::new().verify(proposed_fix, rule_id)
    }
}

/// Convenience: `lint_names()` on `Corpus<ClippyLintDomain>` (back-compat alias
/// for the generic `rule_ids()`).
impl crate::corpus::Corpus<ClippyLintDomain> {
    /// All unique lint names in the corpus (back-compat name for `rule_ids()`).
    #[must_use]
    pub fn lint_names(&self) -> Vec<&str> {
        self.rule_ids()
    }

    /// Look up fix entries by lint name (back-compat for the generic `lookup`).
    pub fn lookup<'a>(
        &'a self,
        lint_name: &'a str,
    ) -> impl Iterator<Item = &'a ClippyFixEntry> + 'a {
        self.entries
            .iter()
            .filter(move |e| e.lint_name == lint_name)
    }
}

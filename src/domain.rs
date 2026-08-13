//! The `HealDomain` trait — one pluggable code-healing domain.
//!
//! This is the **one new abstraction** introduced by Plan 001 (Proposal 001).
//! Everything else in the Phase 1 refactor is rename + reorganization. The
//! domain-agnostic core (drafter + pruner + corpus) operates on the corpus
//! entries this trait produces. The domain owns: (1) what counts as a fixable
//! rule, (2) how to validate a proposed fix against ground truth.
//!
//! ## Caveat (Proposal 001 caveat 1)
//!
//! The signature is **provisional**. The three domains (clippy, perf, kernel)
//! fit this shape NOW, but a fourth domain might not. Ship exactly 3 impls;
//! do not speculatively add methods for hypothetical domains.

use serde::de::DeserializeOwned;
use serde::Serialize;

/// A corpus entry that the domain-agnostic core can read through.
///
/// Each domain's `HealDomain::Entry` type implements this. The drafter reads
/// `fixed_code()`, the pipeline's `seed_from_corpus` reads all three, and the
/// ruliology search reads `fixed_code()` + `buggy_code()`. This is a thin
/// accessor trait — it carries no domain-specific semantics.
///
/// The `context_notes()` method (Plan 001 post-promotion, 2026-08-13) has a
/// default `None` impl so it is **non-breaking** — domains whose rules are
/// unconditional wins (clippy, `kernel_opt`) need no changes. The `rust_perf`
/// domain overrides it for the 2 `ParetoDominated` rules (Bench 013) so the
/// drafter can propagate the annotation through [`DraftedFix`](crate::DraftedFix).
pub trait HealEntry {
    /// The rule identifier (clippy: lint name, perf: rule id, kernel: concept id).
    fn rule_id(&self) -> &str;

    /// The buggy code that triggers the rule.
    fn buggy_code(&self) -> &str;

    /// The fixed code that resolves the rule.
    fn fixed_code(&self) -> &str;

    /// Annotation for rules that are NOT default-configuration wins (Bench 013).
    ///
    /// `Some(notes)` means the consumer should treat this rule as
    /// **context-dependent** — the fix is a genuine pattern, but the bench
    /// validator found it is not unconditionally better than the buggy code in
    /// the default configuration (e.g. the compiler already optimizes the buggy
    /// form, or the benefit only appears under a specific runtime regime).
    ///
    /// Default `None` — most rules are unconditional wins. The drafter does NOT
    /// skip or downgrade annotated rules; it propagates the annotation through
    /// [`DraftedFix::context_notes`] so the consumer decides whether their
    /// context matches.
    fn context_notes(&self) -> Option<&str> {
        None
    }
}

/// One pluggable code-healing domain (clippy lints, rust perf, kernel opt, ...).
///
/// The domain-agnostic core (drafter + pruner + ruliology + self-evolve)
/// operates on the corpus entries this trait produces + the validator verdict
/// it returns. The domain owns: (1) what counts as a fixable rule, (2) how to
/// validate a proposed fix against ground truth.
///
/// **Signature is provisional** (Proposal 001 caveat 1) — ship exactly 3 impls,
/// document that it may change, revisit if a fourth domain forces it.
pub trait HealDomain: Sized {
    /// Corpus entry shape: (`rule_id`, `buggy_code`, `fixed_code`, metadata).
    /// Metadata carries domain-specific signal (`lint_group`, `perf_metric`,
    /// `kernel_shape_dtype_hw`, ...).
    type Entry: Clone + Send + Sync + Serialize + DeserializeOwned + HealEntry + 'static;

    /// Validator verdict: did the proposed fix satisfy the domain's ground truth?
    /// Stronger than "parses" — clippy re-runs clippy, perf runs tests+bench,
    /// kernel runs numeric compare vs reference.
    type Verdict: Clone + Send + Sync;

    /// Build the seed corpus for this domain (BLAKE3-committed, freeze/thaw-ready).
    /// Mirrors `build_seed_corpus()` for clippy; each domain ships its own.
    fn build_corpus() -> Vec<Self::Entry>;

    /// Validate a proposed fix against ground truth.
    ///
    /// This is the **Stage 2 tuning-phase validator** (NOT the hot-path guard).
    /// The tuning loop calls this OFF the hot path to build the Stage 2
    /// adaptive drafter pool. clippy re-runs clippy (~130 ms), perf runs
    /// tests+bench (seconds), kernel runs numeric compare vs reference.
    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict;
}

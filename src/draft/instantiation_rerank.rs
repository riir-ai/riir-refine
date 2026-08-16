//! Instantiation re-ranker — Plan 005 T3.5.4.
//!
//! ## The ceiling this exists to break
//!
//! Every score the retrieval stack produces is **text similarity**, and Bench
//! 018's threshold refutation already established that text similarity cannot
//! decide *rule instantiation*: the correct `Vec::with_capacity` function
//! scored HIGHER on `vec-with-capacity` than the buggy `Vec::new` one, because
//! a one-token difference is invisible to a bag-of-terms scorer. T3.5.2's
//! operator-aware tokenizer widened the vocabulary — 54.2% → 70.8% clippy
//! top-1 — but it did not change the KIND of evidence, so the same ceiling
//! remained: 9 of the 14 residual top-1 misses had the correct rule sitting at
//! **rank 2-3**, with clippy top-3 already at 89.6%.
//!
//! That shape — right answer present, wrong answer first — is a *re-ranking*
//! problem, not a retrieval problem. Adding a better first-stage retriever
//! could not fix it; adding a different kind of evidence can.
//!
//! ## The different evidence
//!
//! The proposers' `syn` visitors already answer exactly the question the
//! scorer cannot: *does this rule actually apply to this span?* A visitor that
//! emits an edit has structurally matched the anti-pattern — that is rule
//! instantiation, not lexical resemblance. This module runs the candidate
//! rules' visitors over the span and lets a firing visitor outrank a
//! non-firing one.
//!
//! Two properties make this cheap and safe:
//!
//! - **One parse, K probes.** The span is parsed once and the pre-parsed
//!   `syn::File` is handed to each candidate's `collect_edits`, so K
//!   candidates cost one parse plus K visitor walks — not K parses.
//! - **Evidence, not a gate.** Instantiation only reorders the retrieved
//!   candidates; it never adds or removes one. A span whose visitors all stay
//!   silent keeps its similarity ranking untouched.
//!
//! ## Why this is opt-in (`RerankMode::Off` by default)
//!
//! On the committed eval this re-ranker looks spectacular — clippy top-1
//! 70.8% → 87.5% (`Structural`) → 95.8% (`All`). **Those numbers are not an
//! effect size, and must not be quoted as one.**
//!
//! The clippy fixtures are label-verified by
//! `clippy_fixture_labels_verify_via_proposers`, a committed passing test that
//! asserts, for all 48 spans: the OWN proposer fires, and **zero** of the
//! other 40 proposers cross-fire. That is precisely the property this module
//! exploits, so on that fixture set the re-ranker cannot be wrong whenever the
//! correct rule is retrieved at all — the oracle that defined the labels is
//! the same oracle doing the ranking.
//!
//! `rerank_circularity_diagnostic` states the falsifiable version and it
//! **holds exactly**: `RerankMode::All` top-1 equals pool recall @
//! [`RERANK_POOL`] to the digit (95.8% = 46/48). The "gain" is measuring
//! *"was the right rule retrieved?"*, not *"did re-ranking pick it?"*.
//!
//! Per the repo's GOAT rule — a gain measured on a biased oracle is not a
//! modelless gain — the gate has NOT passed, so this is not promoted.
//!
//! **What a valid gate needs:** clippy fixtures labeled by an oracle
//! INDEPENDENT of our proposers (real `cargo clippy` output — the harness
//! already exists for the `self_evolve` G7 gate, Bench 024), plus spans where
//! several lints genuinely co-apply, since zero-cross-fire is the artificial
//! part. Until then the honest claim is the one below, not a percentage.
//!
//! ## What IS established, non-circularly
//!
//! - **Safety.** The structural proposers are verified NOT to fire on real
//!   production code by `clean_spans_fire_no_lint` (12 real excerpts from this
//!   crate) — an oracle with no connection to the labeled fixtures. So
//!   `Structural` cannot promote a spurious rule on clean input; in the worst
//!   case it changes nothing.
//! - **Coherence.** `--fix` already treats a firing visitor as the decision
//!   (Phase 6 T6.1: "the proposer's visitor is the `--fix` gate"). Enabling
//!   this makes `--suggest` rank by the same evidence `--fix` acts on, instead
//!   of by a similarity score Bench 018 showed cannot distinguish a bug from
//!   its own fix.
//! - **Cost.** One parse + K visitor walks per span, no allocation beyond the
//!   candidates already held.
//!
//! ## Modelless
//!
//! `syn` visitors are deterministic AST matching — no weights, no training, no
//! inference. Whatever the true effect size is, it is a modelless one, so this
//! becomes promotion-eligible the moment an uncontaminated gate measures it.

use super::corpus_index::ScoredRule;
use super::domain_router::DomainId;

/// How much proposer evidence counts when re-ranking.
///
/// A/B-able because the right answer is measured, not assumed — some
/// proposers are deliberately over-matching and make *worse* evidence than
/// no evidence at all. See [`super::retrieval_eval::run_rerank_sweep`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RerankMode {
    /// No re-ranking — pure text similarity (the pre-T3.5.4 behaviour).
    ///
    /// **The shipped default.** Not because the mechanism is doubted, but
    /// because its effect size is currently unmeasurable — see the
    /// "Why this is opt-in" section in the module docs.
    #[default]
    Off,
    /// Only **structurally sound** proposers count as evidence: clippy's
    /// `FIXABLE_LINTS`, whose detection alone guarantees semantics and which
    /// `--fix` already trusts to auto-apply.
    ///
    /// Excludes the type-blind proposers (`needless_borrow`,
    /// `useless_conversion`, `unnecessary_mut_passed`, `redundant_closure`),
    /// which over-match on parse-only information *by design* — Bench 022
    /// measured them tripping on ordinary production code. Their firing is
    /// weak evidence and would add noise, not signal.
    Structural,
    /// Every dispatchable proposer counts, including the type-blind ones.
    /// Measured as the A/B arm; see the bench note for why it is not default.
    All,
}

/// Number of candidates retrieved per domain before re-ranking.
///
/// Re-ranking can only promote a rule that was retrieved, so the pool must be
/// deeper than the `top_k` the caller wants to display. Retrieve-deep →
/// re-rank → truncate is the standard two-stage shape. 8 covers the residual
/// miss distribution (all 9 near-misses sat at rank 2-3).
pub const RERANK_POOL: usize = 8;

/// Does `rule_id`'s proposer visitor actually match `file`?
///
/// `false` for every rule without a proposer (kernel, and the perf rules
/// outside the shipped five) — the dispatch's `_ => Vec::new()` arm makes
/// "no proposer" and "no match" indistinguishable, which is the correct
/// conservative reading: absence of evidence is not evidence.
#[allow(unused_variables)] // `file`/`code` are unused when no domain feature is on
fn instantiates(domain: DomainId, rule_id: &str, code: &str, file: &syn::File, mode: RerankMode) -> bool {
    match domain {
        #[cfg(feature = "clippy_lints")]
        DomainId::Clippy => {
            use crate::domains::clippy_lints::proposers::{collect_edits, FIXABLE_LINTS};
            if mode == RerankMode::Structural && !FIXABLE_LINTS.contains(&rule_id) {
                return false;
            }
            !collect_edits(rule_id, code, file).is_empty()
        }
        #[cfg(feature = "rust_perf")]
        DomainId::Perf => {
            use crate::domains::rust_perf::proposers::collect_edits;
            !collect_edits(rule_id, code, file).is_empty()
        }
        // Kernel has no proposers (Plan 005 Phase 5 deferred — the corpus is
        // concept-grade). Nothing to probe; retrieval order stands.
        _ => false,
    }
}

/// Mark which of `rules` the span actually instantiates, in place.
///
/// Parses `code` ONCE and reuses the AST across every candidate. A span that
/// does not parse (a fragment, a `#[cube]` kernel body) yields no evidence and
/// leaves every flag `false` — retrieval order is preserved, never corrupted.
pub fn mark_instantiation(
    domain: DomainId,
    code: &str,
    rules: &mut [ScoredRule],
    mode: RerankMode,
) {
    if mode == RerankMode::Off || rules.is_empty() {
        return;
    }
    let Ok(file) = syn::parse_file(code) else {
        return;
    };
    for rule in rules.iter_mut() {
        rule.instantiates = instantiates(domain, &rule.rule_id, code, &file, mode);
    }
}

/// Order `rules` by instantiation first, then by similarity score, then by
/// `rule_id` — the deterministic total order the eval and the CLI share.
///
/// Instantiation is kept as a SEPARATE key rather than folded into `score` as
/// a bonus constant: the two signals are different kinds of evidence on
/// incomparable scales (a BM25 sum vs. a boolean), and any additive bonus
/// large enough to dominate would be a magic number tuned to today's score
/// distribution.
pub fn rerank(rules: &mut [ScoredRule]) {
    rules.sort_by(|a, b| {
        b.instantiates
            .cmp(&a.instantiates)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sr(id: &str, score: f32, instantiates: bool) -> ScoredRule {
        ScoredRule { rule_id: id.to_string(), score, instantiates }
    }

    #[test]
    fn instantiation_outranks_a_higher_similarity_score() {
        let mut v = vec![sr("loud", 99.0, false), sr("correct", 1.0, true)];
        rerank(&mut v);
        assert_eq!(v[0].rule_id, "correct");
    }

    #[test]
    fn similarity_still_orders_within_each_group() {
        let mut v = vec![
            sr("b_hit", 5.0, true),
            sr("a_miss", 50.0, false),
            sr("a_hit", 9.0, true),
            sr("b_miss", 10.0, false),
        ];
        rerank(&mut v);
        let ids: Vec<&str> = v.iter().map(|r| r.rule_id.as_str()).collect();
        assert_eq!(ids, ["a_hit", "b_hit", "a_miss", "b_miss"]);
    }

    #[test]
    fn ordering_is_total_and_deterministic() {
        // Equal on both keys → rule_id decides, so there is no run-to-run flip.
        let mut a = vec![sr("z", 1.0, false), sr("a", 1.0, false)];
        let mut b = vec![sr("a", 1.0, false), sr("z", 1.0, false)];
        rerank(&mut a);
        rerank(&mut b);
        assert_eq!(a, b);
        assert_eq!(a[0].rule_id, "a");
    }

    #[test]
    fn off_mode_marks_nothing() {
        let mut v = vec![sr("needless_return", 1.0, false)];
        mark_instantiation(
            DomainId::Clippy,
            "pub fn f() -> u8 { return 1; }",
            &mut v,
            RerankMode::Off,
        );
        assert!(!v[0].instantiates);
    }

    #[cfg(feature = "clippy_lints")]
    #[test]
    fn a_real_anti_pattern_is_detected_and_a_clean_span_is_not() {
        let mut hit = vec![sr("needless_return", 1.0, false)];
        mark_instantiation(
            DomainId::Clippy,
            "pub fn f() -> u8 { return 1; }",
            &mut hit,
            RerankMode::Structural,
        );
        assert!(hit[0].instantiates, "the anti-pattern must instantiate");

        let mut clean = vec![sr("needless_return", 1.0, false)];
        mark_instantiation(
            DomainId::Clippy,
            "pub fn f() -> u8 { 1 }",
            &mut clean,
            RerankMode::Structural,
        );
        assert!(!clean[0].instantiates, "the fixed form must NOT instantiate");
    }

    #[cfg(feature = "clippy_lints")]
    #[test]
    fn structural_mode_ignores_the_type_blind_proposers() {
        // `needless_borrow` is deliberately over-matching (it fires on every
        // necessary `f(&local)`), so Structural must refuse it as evidence
        // even on a span where its visitor would happily fire.
        let code = "pub fn g(s: &str) -> usize { helper(&s) }";
        let mut structural = vec![sr("needless_borrow", 1.0, false)];
        mark_instantiation(DomainId::Clippy, code, &mut structural, RerankMode::Structural);
        assert!(!structural[0].instantiates);
    }

    #[test]
    fn an_unparseable_span_yields_no_evidence() {
        let mut v = vec![sr("needless_return", 1.0, false)];
        mark_instantiation(DomainId::Clippy, "fn broken( {{{", &mut v, RerankMode::Structural);
        assert!(!v[0].instantiates, "a parse failure must not fabricate evidence");
    }
}

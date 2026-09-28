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
//! ## The measurement, and why the first one was wrong
//!
//! On the ORIGINAL fixture set this re-ranker looked like a +25pp win. It was
//! not one. `clippy_fixture_labels_verify_via_proposers` asserts that for all
//! 48 clippy fixtures the OWN proposer fires and **zero** of the other 40
//! cross-fire — exactly the property this module exploits, so the re-ranker
//! could not be wrong whenever the correct rule was retrieved at all.
//! `rerank_circularity_diagnostic` proved it to the digit: `RerankMode::All`
//! top-1 == pool recall @[`RERANK_POOL`] (46/48).
//!
//! T3.5.5 then built an independent oracle ([`super::clippy_oracle`]) and
//! found the labels were never the problem — real `cargo clippy` confirms
//! **43/43** of the compiling fixtures. The problem was **sample selection**:
//! `0/43` of them trip more than one corpus lint. A set where exactly one rule
//! can possibly be right cannot discriminate a ranker, so the degeneracy
//! persisted even against real labels.
//!
//! The fix was to author the missing sample —
//! [`super::eval_fixtures_composite`], spans that trip several anti-patterns
//! at once, labeled by real clippy rather than by hand. On those, top-1 is a
//! genuine ordering decision (pool recall is 100%, so retrieval is never the
//! limiting factor):
//!
//! | mode | top-1 | vs off |
//! |---|---:|---:|
//! | `Off` | 84.0% | — |
//! | `Structural` | **96.0%** | **+12.0pp** |
//! | `All` | 96.0% | +12.0pp |
//!
//! and the degeneracy check now *fails to fire* — `All` top-1 (96.0%) ≠ pool
//! recall (100%) — i.e. the ranking is doing work retrieval did not do. The
//! result survives removing the authoring bias of the first batch (which
//! over-used `needless_return` as the pairing partner).
//!
//! `Structural` ties `All` on quality while excluding the known over-matchers,
//! so the safer arm is also an equal-best one. That is the promoted default.
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
    /// Kept as the A/B baseline and an escape hatch.
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
    ///
    /// **Promoted to default 2026-08-16 (T3.5.5, Bench 027)** on
    /// uncontaminated evidence: +12.0pp top-1 (84.0% → 96.0%) over 25
    /// real-clippy-labeled multi-lint spans, at +5.6% latency. Ties `All`
    /// on quality while excluding the known over-matchers, so it is the
    /// strictly safer of two equally good arms.
    #[default]
    Structural,
    /// Every dispatchable proposer counts, including the type-blind ones.
    /// Measured as the A/B arm; see the bench note for why it is not default.
    All,
}

/// Number of candidates retrieved per domain before re-ranking.
///
/// Re-ranking can only promote a rule that was retrieved, so the pool must
/// be deeper than the `top_k` the caller wants to display. Retrieve-deep →
/// re-rank → truncate is the standard two-stage shape.
///
/// 8 covered the T3.5.4 residual miss distribution (all 9 near-misses sat
/// at rank 2-3). Issue 024 (Bench 032) measured the first casualty OUTSIDE
/// that bound: under cross-domain co-presence (`needless_return` merged
/// with `vec-with-capacity`, both orders) loop-shaped clippy rules crowd
/// the correct rule to raw similarity rank **9** — one position past the
/// pool of 8, evicting it before the re-ranker ever saw it (the 8.3pp
/// switch-cost signal). 12 restored the rescue with margin; deeper pools
/// only add visitor walks, never new promotions (a structural proposer
/// that does not match stays `false` at any depth).
///
/// Re-pinned 12 → 16 (2026-09-13, the P10 post-mining intake): the two new
/// corpus entries (`vec_init_then_push` — textually the same vec-push
/// family as the casualty's perf half — and `filter_map_bool_then`, raw
/// ranks ~15 and ~12 on the casualty span) crowded the casualty from raw
/// rank 9 to **15** (measured: absent at k=14, retrieved + canonical rank 0
/// at k=16, both spans, both orders). Re-pinned 16 → 20 (2026-09-13, the
/// P11 post-mining intake): the `format_collect` corpus entry crowded the
/// casualty one further position — first-found at depth **17** (measured
/// by the guard's discovery sweep, now first-class diagnostics) — so 17
/// is the measured minimum and 20 the ladder-rung pin with slack 3.
/// Re-pinned 20 → 24 (2026-09-18, Batch 159 / Plan 150): the pack V pair
/// (`single_char_add_str` + `manual_clamp`) crowded the casualty to
/// first-found at depth **21** (measured by the guard's discovery sweep)
/// — 21 is the measured minimum and 24 the ladder-rung pin with slack 3.
/// Re-pinned 16 → 24 (P11, 2026-09-13: pre-P11 raw rank 9 buried by the
/// P10/P11 corpus growth), then 24 → 28 (2026-09-19, Batch 162 / Plan
/// 153: the pack VIII pair crowded the casualty to first-found at depth
/// **25** — 25 the measured minimum, 28 the ladder-rung pin with slack
/// 3), then 28 → 33 (2026-09-28, Batch 184 / Plan 179: the pack XVI pair
/// plus the intervening corpus growth crowded it to first-found at
/// depth **29-30** (30 at the 96-entry corpus, 29 at 98 — organic
/// crowding, not this batch's entries); 30 the measured minimum, 33 the
/// ladder rung with slack 3).
/// The `pool_depth_rescue_guard` re-firing on the next vec-shaped corpus
/// entry is the designed prompt to re-measure, not a defect.
pub const RERANK_POOL: usize = 33;

/// Does `rule_id`'s proposer visitor actually match `file`?
///
/// `false` for every rule without a proposer (kernel, and the perf rules
/// outside the shipped five) — the dispatch's `_ => Vec::new()` arm makes
/// "no proposer" and "no match" indistinguishable, which is the correct
/// conservative reading: absence of evidence is not evidence.
#[allow(unused_variables)] // `file`/`code` are unused when no domain feature is on
/// Could this rule EVER instantiate in this domain under
/// [`RerankMode::Structural`] — i.e. does the Structural probe admit it at
/// all? The single source of the Structural admission sets, consumed by
/// [`instantiates`]'s gate and by the eval's decline axis (Issue 125: a span
/// whose window holds NO probeable rule is proposer-blind — the abstention
/// signal is absent, which is a different row from a real decline).
///
/// Mirrors the per-domain admission exactly: clippy = `FIXABLE_LINTS`,
/// perf = every rule (all probeable), sec = `FIXABLE_SEC`, docker =
/// `FIXABLE_DOCKER`; kernel and dist ship no proposers, so no rule is
/// probeable there. Under [`RerankMode::All`] the type-blind proposers ALSO
/// probe (Bench 022) — probeability here is the Structural reading.
#[must_use]
pub fn probeable(domain: DomainId, rule_id: &str) -> bool {
    match domain {
        #[cfg(feature = "clippy_lints")]
        DomainId::Clippy => {
            use crate::domains::clippy_lints::proposers::FIXABLE_LINTS;
            FIXABLE_LINTS.contains(&rule_id)
        }
        #[cfg(feature = "rust_perf")]
        DomainId::Perf => true,
        #[cfg(feature = "sec")]
        DomainId::Sec => {
            use crate::domains::sec::proposers::FIXABLE_SEC;
            FIXABLE_SEC.contains(&rule_id)
        }
        #[cfg(feature = "docker")]
        DomainId::Docker => {
            use crate::domains::docker::rules::FIXABLE_DOCKER;
            FIXABLE_DOCKER.contains(&rule_id)
        }
        _ => false,
    }
}

fn instantiates(
    domain: DomainId,
    rule_id: &str,
    // consumed only by the probeable domains' proposers; thin feature sets
    // leave both params unread
    #[cfg_attr(not(feature = "clippy_lints"), allow(unused_variables))]
    code: &str,
    #[cfg_attr(not(feature = "clippy_lints"), allow(unused_variables))]
    file: &syn::File,
    mode: RerankMode,
) -> bool {
    if mode == RerankMode::Structural && !probeable(domain, rule_id) {
        return false;
    }
    match domain {
        #[cfg(feature = "clippy_lints")]
        DomainId::Clippy => {
            use crate::domains::clippy_lints::proposers::collect_edits;
            !collect_edits(rule_id, code, file).is_empty()
        }
        #[cfg(feature = "rust_perf")]
        DomainId::Perf => {
            use crate::domains::rust_perf::proposers::collect_edits;
            !collect_edits(rule_id, code, file).is_empty()
        }
        #[cfg(feature = "sec")]
        DomainId::Sec => {
            use crate::domains::sec::proposers::collect_edits;
            !collect_edits(rule_id, code, file).is_empty()
        }
        #[cfg(feature = "docker")]
        DomainId::Docker => {
            // The docker proposers read TEXT, not the parsed `syn::File` —
            // the only domain whose probe ignores `file` entirely.
            use crate::domains::docker::proposers::collect_edits;
            !collect_edits(rule_id, code).is_empty()
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
    mark_instantiation_with_file(domain, code, &file, rules, mode);
}

/// [`mark_instantiation`](Self::mark_instantiation) over a caller-parsed AST —
/// the multi-domain fan-out path (Issue 063): one `syn::parse_file` per span
/// shared across every Rust-domain probe instead of one per domain. The
/// parse is ~20µs of a ~25µs stage on the gate's fixtures, so the per-domain
/// parse dominated the marking itself.
///
/// The parse-fail contract lives with the caller: a span that does not parse
/// gets NO marking in ANY domain (including docker, whose probe reads text —
/// `mark_instantiation` short-circuits before the domain dispatch, and the
/// fan-out path must skip equivalently). Every flag stays `false`.
pub fn mark_instantiation_with_file(
    domain: DomainId,
    code: &str,
    file: &syn::File,
    rules: &mut [ScoredRule],
    mode: RerankMode,
) {
    if mode == RerankMode::Off || rules.is_empty() {
        return;
    }
    for rule in rules.iter_mut() {
        rule.instantiates = instantiates(domain, &rule.rule_id, code, file, mode);
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

// ── Rerank-pool collapse detector (Issue 070 T3) ─────────────────────────
//
// Issue 024 (Bench 032) measured the failure this surfaces: under
// cross-domain co-presence the correct rule can be crowded to raw similarity
// rank 9 — one past the then-pool of 8 — while lookalike rules fill the
// visible top-k. Until now that shape was only visible as a post-hoc
// switch-cost delta; this is the first-class detector beside
// [`RerankMode::Structural`]. It is DIAGNOSTIC ONLY: it observes a ranked
// pool, it never reorders, promotes, or removes anything, so an unused
// detector is bit-identical to not having one.

/// The deterministic rule-family key for the collapse detector: the rule
/// id's LEADING SEGMENT — the text before the first `-` or `_`, whichever
/// comes first, or the whole id when neither appears.
///
/// This groups the snake clippy ids (`needless_return`/`needless_bool`/
/// `needless_match` → `needless`) and the kebab perf/kernel/sec ids
/// (`collect-append-to-extend` → `collect`,
/// `smem-envelope-gate-kernel-variant` → `smem`) by their leading stem. It
/// is coarse by design — the detector is a diagnostic, not a classifier —
/// and deterministic by construction: a pure prefix scan, no table, no
/// allocation.
///
/// Chosen over the candidate's *domain* as the family axis because a rerank
/// pool is per-domain by construction (the fan-out ranks within one
/// domain's index), where a domain family is constant and "collapsed to one
/// family" would be vacuously true for every pool.
#[must_use]
pub fn rule_family(rule_id: &str) -> &str {
    match (rule_id.find('-'), rule_id.find('_')) {
        (Some(dash), Some(under)) => &rule_id[..dash.min(under)],
        (Some(dash), None) => &rule_id[..dash],
        (None, Some(under)) => &rule_id[..under],
        (None, None) => rule_id,
    }
}

/// Whether a rerank pool's top-k collapsed to one rule family — the
/// measurable shape of the Issue-024 pool-crowding failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollapseVerdict<'a> {
    /// No crowding signal: the top-k spans more than one family, OR the
    /// pool collapsed onto the oracle's own family (the right answer won —
    /// collapse without a dissenting oracle is not the crowding class), OR
    /// there is no pool/no probe window to speak of.
    Diverse,
    /// Every one of the top-`pool_top_k` candidates shares one rule
    /// `family`.
    ///
    /// `oracle_outside` is the crowding EVIDENCE bit: `true` when a
    /// ground-truth label was supplied and its family sits outside the
    /// collapsed one (the right answer was crowded out of view). `false`
    /// means no oracle was available — the production heal path passes
    /// `None`, since it has no label — so the pool is collapsed but nothing
    /// says it is *wrongly* so.
    Collapsed {
        family: &'a str,
        pool_top_k: usize,
        oracle_outside: bool,
    },
}

/// The shared walk: `families` are the top-k candidates' family keys in
/// ranked order (an empty iterator = empty pool or a zero probe window).
fn collapse_over<'a>(
    mut families: impl ExactSizeIterator<Item = &'a str>,
    oracle_label: Option<&str>,
) -> CollapseVerdict<'a> {
    let pool_top_k = families.len();
    let Some(family) = families.next() else {
        return CollapseVerdict::Diverse;
    };
    if families.any(|f| f != family) {
        return CollapseVerdict::Diverse;
    }
    if oracle_label.is_some_and(|oracle| rule_family(oracle) == family) {
        // Collapsed onto the oracle's own family — the right answer won.
        return CollapseVerdict::Diverse;
    }
    CollapseVerdict::Collapsed {
        family,
        pool_top_k,
        // An equal-family oracle returned `Diverse` above, so `Some` here
        // always means "a supplied oracle from a DIFFERENT family".
        oracle_outside: oracle_label.is_some(),
    }
}

/// Detect a top-k family collapse in a ranked rerank pool (Issue 070 T3).
///
/// `pool` is the post-[`rerank`] candidate list (best first — the detector
/// reads order, it does not sort); `top_k` scopes the probe window
/// (`min(top_k, pool.len())`); `oracle_label` is the eval fixtures' known
/// rule id, or `None` on the production path.
///
/// Zero-allocation: the verdict borrows the first candidate's `rule_id`.
#[must_use]
pub fn detect_collapse<'a>(
    pool: &'a [ScoredRule],
    top_k: usize,
    oracle_label: Option<&str>,
) -> CollapseVerdict<'a> {
    let k = top_k.min(pool.len());
    collapse_over(
        pool[..k].iter().map(|r| rule_family(&r.rule_id)),
        oracle_label,
    )
}

/// [`detect_collapse`] over bare rule ids — the eval path's entry point,
/// where the ranked candidates sit behind shared refs
/// (`canonical_rank` returns `Vec<&ScoredRule>`).
#[must_use]
pub fn detect_collapse_rule_ids<'a>(
    rule_ids: &[&'a str],
    top_k: usize,
    oracle_label: Option<&str>,
) -> CollapseVerdict<'a> {
    let k = top_k.min(rule_ids.len());
    collapse_over(rule_ids[..k].iter().copied().map(rule_family), oracle_label)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sr(id: &str, score: f32, instantiates: bool) -> ScoredRule {
        ScoredRule {
            rule_id: id.to_string(),
            score,
            instantiates,
        }
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
        assert!(
            !clean[0].instantiates,
            "the fixed form must NOT instantiate"
        );
    }

    #[cfg(feature = "clippy_lints")]
    #[test]
    fn structural_mode_ignores_the_type_blind_proposers() {
        // `needless_borrow` is deliberately over-matching (it fires on every
        // necessary `f(&local)`), so Structural must refuse it as evidence
        // even on a span where its visitor would happily fire.
        let code = "pub fn g(s: &str) -> usize { helper(&s) }";
        let mut structural = vec![sr("needless_borrow", 1.0, false)];
        mark_instantiation(
            DomainId::Clippy,
            code,
            &mut structural,
            RerankMode::Structural,
        );
        assert!(!structural[0].instantiates);
    }

    #[test]
    fn an_unparseable_span_yields_no_evidence() {
        let mut v = vec![sr("needless_return", 1.0, false)];
        mark_instantiation(
            DomainId::Clippy,
            "fn broken( {{{",
            &mut v,
            RerankMode::Structural,
        );
        assert!(
            !v[0].instantiates,
            "a parse failure must not fabricate evidence"
        );
    }

    // ── Issue 070 T3: rerank-pool collapse detector ──────────────────────

    #[test]
    fn rule_family_is_the_leading_dash_or_underscore_segment() {
        assert_eq!(rule_family("needless_return"), "needless");
        assert_eq!(rule_family("collect-append-to-extend"), "collect");
        assert_eq!(rule_family("smem-envelope-gate-kernel-variant"), "smem");
        assert_eq!(rule_family("nan-unsafe-sort-comparator"), "nan");
        assert_eq!(rule_family("plainid"), "plainid");
    }

    /// G1a — the planted Issue-024 shape: the top-4 window is one family
    /// (`needless`) while the oracle label (`vec-with-capacity`, family
    /// `vec`) sits outside it. The 5th candidate (the oracle's own family)
    /// sits at rank 5, deliberately OUTSIDE the probed top-4 — crowding is
    /// a property of the WINDOW, not of the whole pool.
    #[test]
    fn planted_collapse_with_the_oracle_outside_is_detected() {
        let pool = vec![
            sr("needless_return", 5.0, false),
            sr("needless_bool", 4.0, false),
            sr("needless_match", 3.0, false),
            sr("needless_borrow", 2.0, false),
            sr("vec_with_capacity", 1.0, false),
        ];
        assert_eq!(
            detect_collapse(&pool, 4, Some("vec-with-capacity")),
            CollapseVerdict::Collapsed {
                family: "needless",
                pool_top_k: 4,
                oracle_outside: true
            }
        );
    }

    /// Widening the same pool's window to include the second family
    /// dissolves the verdict — the detector scopes to `top_k`.
    #[test]
    fn a_multi_family_top_k_window_is_diverse() {
        let pool = vec![
            sr("needless_return", 5.0, false),
            sr("vec_with_capacity", 1.0, false),
        ];
        assert_eq!(
            detect_collapse(&pool, 2, Some("vec-with-capacity")),
            CollapseVerdict::Diverse
        );
    }

    /// G1b (constructed complement): a single-family pool that collapsed
    /// onto the ORACLE's own family is not crowding — the right answer won.
    #[test]
    fn a_single_family_pool_holding_the_oracle_is_diverse() {
        let pool = vec![
            sr("needless_return", 3.0, true),
            sr("needless_bool", 2.0, false),
        ];
        assert_eq!(
            detect_collapse(&pool, 2, Some("needless_lifetimes")),
            CollapseVerdict::Diverse
        );
    }

    /// No crash + honest verdicts on degenerate pools: empty (nothing to
    /// collapse), a zero probe window, and a single candidate (maximally
    /// collapsed; the oracle decides whether it is the crowding class).
    #[test]
    fn empty_and_single_candidate_pools_do_not_crash() {
        assert_eq!(
            detect_collapse(&[], 4, Some("needless_return")),
            CollapseVerdict::Diverse
        );
        assert_eq!(
            detect_collapse_rule_ids(&[], 4, None),
            CollapseVerdict::Diverse
        );
        let single = vec![sr("collect-append-to-extend", 1.0, false)];
        assert_eq!(
            detect_collapse(&single, 0, Some("needless_return")),
            CollapseVerdict::Diverse,
            "top_k = 0 probes nothing"
        );
        assert_eq!(
            detect_collapse(&single, 4, None),
            CollapseVerdict::Collapsed {
                family: "collect",
                pool_top_k: 1,
                oracle_outside: false
            }
        );
        assert_eq!(
            detect_collapse(&single, 4, Some("needless_return")),
            CollapseVerdict::Collapsed {
                family: "collect",
                pool_top_k: 1,
                oracle_outside: true
            }
        );
    }

    /// G1b — a REALISTIC pool straight off the composite-corpus eval path
    /// (`LatentMatcher` fan-out + the structural re-rank, unmodified). The
    /// oracle is the pool's own top-1 rule: its family IS the top family by
    /// construction (or the pool is multi-family), so `Diverse` is the
    /// required verdict under EITHER detector branch — the assertion is
    /// deliberately top-1-independent, because whichever rule the
    /// deterministic retrieval surfaces, an oracle inside the winning
    /// family is never crowding.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn a_real_composite_pool_with_the_oracle_inside_the_top_family_is_diverse() {
        let mut matcher =
            crate::draft::LatentMatcher::from_enabled_domains().expect("build matcher");
        let (tag, code) = crate::draft::eval_fixtures_composite::COMPOSITE_SPANS[0];
        let (_routed, by_domain) = matcher.suggest_span(code, RERANK_POOL);
        let clippy: &[ScoredRule] = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .map_or(&[], |(_, rules)| rules.as_slice());
        assert!(
            !clippy.is_empty(),
            "{tag}: the composite span must retrieve clippy candidates"
        );
        let oracle = clippy[0].rule_id.clone();
        assert_eq!(
            detect_collapse(clippy, RERANK_POOL, Some(&oracle)),
            CollapseVerdict::Diverse
        );
    }
}

//! Component-attributed failure taxonomy — Issue 047 (Research 347 / Recuris).
//!
//! Every recorded heal failure is tagged with exactly one component class,
//! derived **at record time** from signals the loop already possesses — never
//! reconstructed post-hoc. That is the paper's localization-by-construction
//! result transplanted to the healer: their Table 4 measures failure
//! localization from structured decision-time traces at **64.8%** vs **13.0%**
//! from outcome alone, and the gain concentrates exactly on the faults a
//! transcript cannot show (a non-fired invocation is invisible in output;
//! only the decision-time record shows it).
//!
//! ## The four classes (the Recuris E/W/ρ/C ontology, healer-translated)
//!
//! | Class | Healer meaning | Existing evidence |
//! |---|---|---|
//! | **E** (corpus) | rule missing / fixture wrong / proposer lacks the lint shape | miss with zero plausible candidates; fixture that fails its own proposer |
//! | **ρ** (retrieval/invocation) | right rule existed, wrong rule won — crowding, mis-route, pool depth, rerank miss | miss where the correct rule is outside top-1; cross-domain pairs |
//! | **W** (drafter context) | right rule retrieved, wrong fix drafted | parse failures; compile-gate reverts (defective output the gate caught) |
//! | **C** (verifier/gate) | fix was good but gate rejected / fix was bad but gate passed | created-warnings; kept-but-unresolved; content-dropping false positives; infrastructure reverts |
//!
//! ## Precedence (the deterministic decision tree)
//!
//! Verdict-stage signals outrank retrieval-stage ones (a fix that existed and
//! was judged says more than a pool snapshot): C (gate passed a bad fix) →
//! false-positive split (C content-drop / E shape) → C (gate rejected without
//! judging) → W (drafter produced a defective fix) → ρ/E from the miss rank.
//!
//! Always compiled — every consumer of the failure streams (memory envelope,
//! traj store, misses dump, retrieval eval) lives behind a different feature
//! set, and a taxonomy gate must not be skippable via a feature flag.

use serde::{Deserialize, Serialize};

/// One component of the E/W/ρ/C failure ontology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentClass {
    /// Corpus/proposer gap — the rule is missing, its fixture is wrong, or
    /// the proposer does not implement the lint's real shape.
    #[serde(rename = "E")]
    Corpus,
    /// Retrieval/invocation failure — the right rule existed but the wrong
    /// rule won (crowding, mis-route, pool depth, rerank miss).
    #[serde(rename = "rho")]
    Retrieval,
    /// Drafter-context failure — the right rule was retrieved but the drafted
    /// fix was defective (parse failure, compile-broken revert).
    #[serde(rename = "W")]
    Drafter,
    /// Verifier/gate failure — the gate passed a bad fix (created warnings,
    /// kept-but-unresolved, content-dropping rewrites) or rejected without
    /// judging one (infrastructure reverts).
    #[serde(rename = "C")]
    Gate,
}

impl ComponentClass {
    /// Stable short tag (the serialized form — also the audit-script key).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Corpus => "E",
            Self::Retrieval => "rho",
            Self::Drafter => "W",
            Self::Gate => "C",
        }
    }
}

impl std::fmt::Display for ComponentClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let long = match self {
            Self::Corpus => "corpus/proposer",
            Self::Retrieval => "retrieval/invocation",
            Self::Drafter => "drafter context",
            Self::Gate => "verifier/gate",
        };
        write!(f, "{} ({long})", self.as_str())
    }
}

/// Why an edit was reverted — the revert half of the decision-time record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevertKind {
    /// The compile gate mapped rustc errors onto the edit (or the
    /// `rustc_errors` no-progress criterion fired): the drafted fix was
    /// defective and the gate caught it — W-class.
    CompileGate,
    /// The verify infrastructure rejected without evaluating the fix (file
    /// I/O): the fix was plausibly good — an unnecessary, gate-side revert —
    /// C-class.
    Infrastructure,
}

/// The decision-time record a failure is classified from. Every field is
/// filled where the fact is observed — never inferred afterwards.
///
/// All fields default to "unknown/absent"; [`classify`] consumes the partial
/// knowledge the recording site actually has.
#[derive(Debug, Clone, Default)]
pub struct FailureSignal {
    /// Ground-truth rule id, when known (eval fixtures, retro-validation).
    pub expected_rule: Option<String>,
    /// Rank (0-based) of the expected rule in the retrieved pool. `None`
    /// when it never surfaced within the probe depth (and meaningless when
    /// `expected_rule` is `None`).
    pub expected_rank: Option<usize>,
    /// Whether ANY candidate was retrieved/proposed for the span.
    pub any_candidates: bool,
    /// Whether the drafted fix failed syn parse (`Some` when a fix was
    /// drafted and parsed).
    pub fix_parse_failed: Option<bool>,
    /// Why the fix was reverted (`None` when it was not reverted).
    pub revert_reason: Option<RevertKind>,
    /// Whether applying the fix created NEW diagnostics (created warnings).
    pub created_warnings: bool,
    /// The compile gate KEPT the edit but the full oracle says the lint
    /// persists — the gate was too loose.
    pub oracle_unresolved: Option<bool>,
    /// The oracle (real clippy / cargo check) says the target lint was
    /// ABSENT on the pre-fix source — the proposer fired on a shape the
    /// real lint does not flag (false positive).
    pub lint_absent_before: Option<bool>,
    /// The transform drops/displaces source content (comments) — the
    /// Issue 028 comment-guard class.
    pub drops_source_content: Option<bool>,
}

impl FailureSignal {
    /// A miss where no candidate was proposed at all (the `.refine/misses/`
    /// `rustc_errors` shape) — classifies E (corpus gap).
    #[must_use]
    pub fn miss_no_candidates() -> Self {
        Self {
            any_candidates: false,
            ..Self::default()
        }
    }

    /// A miss where the expected rule surfaced but did not win — classifies
    /// ρ (crowding / rerank miss).
    #[must_use]
    pub fn miss_outranked(expected: &str, rank: usize) -> Self {
        Self {
            expected_rule: Some(expected.to_string()),
            expected_rank: Some(rank),
            any_candidates: true,
            ..Self::default()
        }
    }

    /// A miss where the expected rule never surfaced within the probe —
    /// ρ when other candidates filled the pool (crowding pushed it past the
    /// probe), E when nothing answered the span at all.
    #[must_use]
    pub fn miss_not_surfaced(expected: &str, any_candidates: bool) -> Self {
        Self {
            expected_rule: Some(expected.to_string()),
            expected_rank: None,
            any_candidates,
            ..Self::default()
        }
    }

    /// A reverted edit (see [`RevertKind`] for the class split).
    #[must_use]
    pub fn reverted(kind: RevertKind) -> Self {
        Self {
            revert_reason: Some(kind),
            ..Self::default()
        }
    }

    /// The drafted fix failed syn parse — classifies W.
    #[must_use]
    pub fn parse_failure() -> Self {
        Self {
            fix_parse_failed: Some(true),
            ..Self::default()
        }
    }

    /// Kept by the compile gate, unresolved per the full oracle — classifies
    /// C (gate too loose).
    #[must_use]
    pub fn gate_passed_ineffective() -> Self {
        Self {
            oracle_unresolved: Some(true),
            ..Self::default()
        }
    }

    /// The fix introduced new diagnostics — classifies C.
    #[must_use]
    pub fn created_warning() -> Self {
        Self {
            created_warnings: true,
            ..Self::default()
        }
    }

    /// The oracle had no lint on the pre-fix source (false positive).
    /// Content-dropping rewrites classify C; shape mismatches classify E.
    #[must_use]
    pub fn false_positive(drops_source_content: bool) -> Self {
        Self {
            lint_absent_before: Some(true),
            drops_source_content: Some(drops_source_content),
            ..Self::default()
        }
    }
}

/// Derive the component class from a decision-time signal. Deterministic,
/// zero-allocation (beyond the signal itself), no LLM — the fixed procedure
/// that replaces the paper's Meta-Agent auditor.
#[must_use]
pub fn classify(sig: &FailureSignal) -> ComponentClass {
    // ── Verdict-stage: a fix existed and was judged ──────────────────────
    // The gate passed a fix that broke something (created warnings) or that
    // did not resolve the target (kept-but-unresolved): fix was bad, gate
    // too loose — C.
    if sig.created_warnings || sig.oracle_unresolved == Some(true) {
        return ComponentClass::Gate;
    }
    // False positive: the oracle never had the lint. Split by transform
    // safety — a content-dropping rewrite is a validation gap (the gate must
    // refuse transforms that destroy source text; the Issue 028 class); a
    // content-preserving one means the proposer's match condition does not
    // implement the lint's real shape (the Issue 027 class).
    if sig.lint_absent_before == Some(true) {
        return if sig.drops_source_content.unwrap_or(false) {
            ComponentClass::Gate
        } else {
            ComponentClass::Corpus
        };
    }
    // The gate rejected without evaluating the fix (infrastructure) — an
    // unnecessary, gate-side revert — C.
    if sig.revert_reason == Some(RevertKind::Infrastructure) {
        return ComponentClass::Gate;
    }
    // The drafter produced a defective fix (compile-broken, or ineffective
    // per the rustc no-progress criterion) and the gate caught it — W.
    if sig.revert_reason.is_some() || sig.fix_parse_failed == Some(true) {
        return ComponentClass::Drafter;
    }
    // ── Retrieval-stage: ground truth known (eval fixtures) ──────────────
    if sig.expected_rule.is_some() {
        return match (sig.expected_rank, sig.any_candidates) {
            // Surfaced but did not win: crowding / rerank miss — ρ.
            (Some(r), _) if r > 0 => ComponentClass::Retrieval,
            // Never surfaced while other candidates filled the pool:
            // crowding pushed it past the probe — ρ (the Issue 024
            // signature).
            (None, true) => ComponentClass::Retrieval,
            // rank 0 cannot be a miss (the expected rule topped the list —
            // defensive default); nothing answered the span at all — E.
            _ => ComponentClass::Corpus,
        };
    }
    // No ground truth, no verdict signals: the plain no-candidate miss —
    // the corpus-gap default (the `.refine/misses/` `rustc_errors` shape).
    ComponentClass::Corpus
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Taxonomy basics ────────────────────────────────────────────────────

    #[test]
    fn serde_tags_round_trip() {
        for (class, tag) in [
            (ComponentClass::Corpus, "\"E\""),
            (ComponentClass::Retrieval, "\"rho\""),
            (ComponentClass::Drafter, "\"W\""),
            (ComponentClass::Gate, "\"C\""),
        ] {
            assert_eq!(serde_json::to_string(&class).unwrap(), tag);
            assert_eq!(serde_json::from_str::<ComponentClass>(tag).unwrap(), class);
        }
    }

    #[test]
    fn constructors_hit_their_classes() {
        assert_eq!(
            classify(&FailureSignal::miss_no_candidates()),
            ComponentClass::Corpus
        );
        assert_eq!(
            classify(&FailureSignal::miss_outranked("x", 4)),
            ComponentClass::Retrieval
        );
        assert_eq!(
            classify(&FailureSignal::miss_not_surfaced("x", true)),
            ComponentClass::Retrieval
        );
        assert_eq!(
            classify(&FailureSignal::miss_not_surfaced("x", false)),
            ComponentClass::Corpus
        );
        assert_eq!(
            classify(&FailureSignal::reverted(RevertKind::CompileGate)),
            ComponentClass::Drafter
        );
        assert_eq!(
            classify(&FailureSignal::reverted(RevertKind::Infrastructure)),
            ComponentClass::Gate
        );
        assert_eq!(
            classify(&FailureSignal::parse_failure()),
            ComponentClass::Drafter
        );
        assert_eq!(
            classify(&FailureSignal::gate_passed_ineffective()),
            ComponentClass::Gate
        );
        assert_eq!(
            classify(&FailureSignal::created_warning()),
            ComponentClass::Gate
        );
        assert_eq!(
            classify(&FailureSignal::false_positive(false)),
            ComponentClass::Corpus
        );
        assert_eq!(
            classify(&FailureSignal::false_positive(true)),
            ComponentClass::Gate
        );
    }

    #[test]
    fn verdict_stage_outranks_retrieval_stage() {
        // A miss whose fix ALSO created a warning classifies C, not rho/E —
        // the judged failure says more than the pool snapshot.
        let sig = FailureSignal {
            expected_rule: Some("needless_return".to_string()),
            expected_rank: Some(4),
            any_candidates: true,
            created_warnings: true,
            ..FailureSignal::default()
        };
        assert_eq!(classify(&sig), ComponentClass::Gate);
    }

    #[test]
    fn rank_zero_is_defensively_corpus() {
        let sig = FailureSignal::miss_outranked("x", 0);
        assert_eq!(classify(&sig), ComponentClass::Corpus);
    }

    // ── T3: retro-validation against the three hand-diagnosed incidents ───
    //
    // The POC gate (Issue 047): if the classifier cannot reproduce the three
    // known answers from the incidents' recorded facts, the taxonomy is
    // wrong — fix before shipping.

    /// **Issue 024 / Bench 032** (hand diagnosis: ρ — retrieval pool
    /// crowding). On the merged `needless_return` + `vec-with-capacity` span
    /// at `RERANK_POOL = 8`, the correct clippy rule was crowded to raw
    /// similarity rank 9 by five loop-shaped rules (`range_plus_one` 45.6,
    /// `needless_range_loop` 36.9, `iter_next_loop` 30.9,
    /// `while_let_on_iterator` 27.1, `iter_nth_zero` 25.9 vs its 15.4) —
    /// outside the top-8 probe while those candidates filled it.
    #[test]
    fn retro_issue_024_pool_crowding_is_rho() {
        let not_surfaced = FailureSignal::miss_not_surfaced("needless_return", true);
        assert_eq!(classify(&not_surfaced), ComponentClass::Retrieval);
        // The surfaced-but-outranked variant (rank inside the probe, past
        // top-1) is the same class.
        let outranked = FailureSignal::miss_outranked("needless_return", 4);
        assert_eq!(classify(&outranked), ComponentClass::Retrieval);
    }

    /// **Issue 027** (hand diagnosis: E — proposer shape gap). The
    /// `single_match_else` proposer fired on expression-wildcard `_ => 0`
    /// arms; the probe matrix proved real clippy fires ONLY on
    /// statement-BLOCK wildcards. Decision-time record: oracle silent on the
    /// pre-fix source (false positive), rewrite content-preserving.
    #[test]
    fn retro_issue_027_shape_false_positive_is_e() {
        assert_eq!(
            classify(&FailureSignal::false_positive(false)),
            ComponentClass::Corpus
        );
    }

    /// **Issue 028** (hand diagnosis: C — comment-guard validation gap).
    /// Eight proposed `collapsible_else_if` edits where real clippy had ZERO
    /// findings — every site had a comment between `else {` and the inner
    /// `if` that the merge would displace. The shape detection was right;
    /// the transform destroys source content the gate must refuse to drop.
    #[test]
    fn retro_issue_028_comment_guard_is_c() {
        assert_eq!(
            classify(&FailureSignal::false_positive(true)),
            ComponentClass::Gate
        );
    }
}

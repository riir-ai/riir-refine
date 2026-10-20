//! Issue 017 — retrieval-precision eval at scale (the 12-fixture gap).
//!
//! Bench 018's G1 gate measured 100% top-1 precision on a **12-item** fixture
//! set while the corpus kept growing (40 clippy + 50 perf + 343 kernel rules
//! at land time). Nearest-neighbor crowding is the standard failure mode as
//! a corpus grows — this module is the scaled, repeatable eval that measures
//! ranking precision (top-1 / top-3 / top-5 recall) at the CURRENT corpus
//! scale, per domain + blended, and re-measures on every corpus batch via
//! the committed regression-gate test.
//!
//! Fixture composition (honest provenance — see Bench 022):
//! - **clippy (48)**: hand-authored variants, one per corpus lint, verified
//!   against real `cargo clippy` 1.93 (exactly-one-lint sweep) at authoring
//!   time + re-verified against THIS crate's proposers by the committed
//!   label test below (the proposers are the standing oracle).
//! - **`rust_perf` (48)**: hand-authored variants of the corpus
//!   `buggy_code` shapes; the five proposer-covered rules are re-verified by
//!   the committed label test, the rest are semantic labels authored against
//!   the corpus entries (Batch 48 added the 8 online-search rules' spans).
//! - **kernel (30, `kernel_opt` only)**: deterministic mutated variants —
//!   the corpus `ref_shape` texts with identifiers renamed via a fixed map
//!   (`mutate_shape`). A self-variant probe: optimistic by construction (it
//!   bounds crowding from above), which is exactly the decay signal the
//!   gate watches — when own-variants stop ranking top-1 among 343 rules,
//!   crowding is real.
//! - **thin domains (Issue 134 T3 re-arm, 2026-09-24 — see Bench 101)**:
//!   `sec` (16, `sec` only) / `docker` (31) / `dist` (6) / `shader_perf`
//!   (8, `shader_perf` only) — hand-authored variants of the corpus
//!   anti-patterns, one span per rule, labels verified against the corpus
//!   and their own proposers by the committed label tests. Honest
//!   confusability documented in the fixture files: docker's
//!   cache-mount/manifest-first corpus texts are byte-identical, and dist's
//!   three ship-size rules share the bare-`[[bin]]` shape — their constants
//!   (0.867 / 0.800) measure that real selection difficulty.
//! - **clean (12)**: REAL production code excerpts from this repo. The
//!   clean-snippet class matters because `--suggest` has no score threshold
//!   (Bench 018 addendum — measured dead end); the eval records WHAT tops
//!   clean spans (the false-positive profile) instead of pretending they
//!   retrieve nothing.
//!
//! Non-goals (do not re-litigate — Issue 017): no embedder re-training, no
//! score-threshold work.

// The `CLEAN_SPANS` consts keep a uniform `r#"..."#` raw-string style
// (fixture data; mixed styles churn every edit).
#![allow(clippy::needless_raw_string_hashes)]

use super::domain_router::DomainId;
use super::instantiation_rerank::{detect_collapse_rule_ids, rule_family, CollapseVerdict};
use super::latent_matcher::LatentMatcher;
// dead under thin feature sets (its consumers live in the clippy_lints +
// rust_perf lanes and the gated tests module); gated to match
#[cfg_attr(not(all(feature = "clippy_lints", feature = "rust_perf")), allow(unused_imports))]
use crate::domain::{HealDomain, HealEntry};

/// Per-domain (or blended) recall counters.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DomainMetrics {
    /// Labeled spans evaluated for this domain.
    pub labeled: usize,
    pub hits_at_1: usize,
    pub hits_at_3: usize,
    pub hits_at_5: usize,
}

impl DomainMetrics {
    /// Recall@k as a ratio; `None` when nothing was labeled (domain off).
    ///
    /// Span counts are small (≤ 130) — f32 precision is exact here by
    /// orders of magnitude; the cast is display-metric arithmetic, not
    /// accumulated state.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn recall(&self, k: usize) -> Option<f32> {
        if self.labeled == 0 {
            return None;
        }
        let hits = match k {
            1 => self.hits_at_1,
            3 => self.hits_at_3,
            _ => self.hits_at_5,
        };
        Some(hits as f32 / self.labeled as f32)
    }
}

/// A miss for the failure taxonomy: the expected rule was not top-k in its
/// own domain's ranked list.
#[derive(Debug, Clone)]
pub struct MissRecord {
    /// The ground-truth rule id.
    pub expected: String,
    pub domain: DomainId,
    /// Rank (0-based) if the rule surfaced within the top-8 probe, else
    /// `None` (not retrieved at all).
    pub rank: Option<usize>,
    /// What topped the domain's list instead.
    pub top1: Option<String>,
    /// Component-attributed class (Issue 047), derived at record time from
    /// this record's own facts: outranked / crowded-past-the-probe = ρ
    /// (retrieval), nothing-answered = E (corpus). The eval is the living
    /// ρ/E coverage — its misses carry ground truth the production paths
    /// lack.
    pub component: crate::component::ComponentClass,
}

/// Issue 070 T3: one recorded rerank-pool collapse — the labeled span's own
/// domain ranking collapsed to a single rule family across the probe window
/// while the ground-truth label's family sits OUTSIDE it (the Issue-024
/// pool-crowding class, made a visible diagnostic). Collapsed pools WITHOUT
/// a dissenting oracle are not recorded: without a label there is no
/// crowding evidence, only a shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapseRecord {
    /// The labeled span's ground-truth rule id — its own family (via
    /// [`rule_family`]) is the family the pool SHOULD have collapsed onto.
    pub label: String,
    /// The domain whose ranking collapsed.
    pub domain: DomainId,
    /// The family the top-k collapsed onto (owned copy — the detector's
    /// borrowed verdict does not outlive the eval loop).
    pub family: String,
    /// The probe depth at which the collapse was measured.
    pub pool_top_k: usize,
}

/// Issue 125 (CUA-S1 `evals/metrics.py` adoption): the decision-outcome
/// taxonomy over labeled spans — provider-neutral, derived entirely from
/// facts the eval already computes (rank of gold, rank of acceptable
/// alternatives, the `instantiates` flags of the ranked window). No new
/// inference, no threshold: `ScoredRule::score` is documented (Bench 018
/// addendum) to carry NO confidence signal, so the decline axis rides the
/// instantiation probe instead — the only measured rule-instantiation gate.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceTaxonomy {
    /// Labeled spans the taxonomy covered (same population as `blended`).
    pub labeled: usize,
    /// Gold rule ranked #1 — strict ([`ChoiceOutcome::CorrectRule`]).
    /// Cross-checkable: equals `blended.hits_at_1` while no acceptable
    /// sets are authored.
    pub correct_rule: usize,
    /// An authored acceptable-alternative won top-1
    /// ([`ChoiceOutcome::CorrectAcceptable`]).
    pub correct_via_acceptable: usize,
    /// Gold (or an acceptable alternative) surfaced but was outranked
    /// ([`ChoiceOutcome::WrongRule`]) — the fix is discrimination.
    pub wrong_rule: usize,
    /// Neither gold nor any acceptable alternative surfaced
    /// ([`ChoiceOutcome::WrongSpan`]) — the fix is the span pipeline.
    pub wrong_span: usize,
    /// Decline axis (three-way over the span's own-domain ranked window):
    /// at least one candidate's proposer FIRED — the span is actionable.
    /// `canonical_rank` sorts `instantiates` first, so covered ⇒ top-1
    /// instantiated. CUA-S1 `coverage`.
    pub covered: usize,
    /// Probeable candidates were present but NONE fired — a REAL decline:
    /// the proposers looked at the span and did not act on it. CUA-S1
    /// `abstention_rate` numerator.
    pub declined: usize,
    /// The window holds no probeable rule at all (kernel/dist spans, or a
    /// clippy window outside `FIXABLE_LINTS`) — the instantiation signal is
    /// ABSENT, not negative; excluded from coverage/abstention denominators
    /// and reported beside them so the split is honest.
    pub proposer_blind: usize,
    /// Top-1 correct (strict or acceptable) among COVERED spans — CUA-S1
    /// `selective_accuracy` numerator.
    pub selective_correct: usize,
}

impl ChoiceTaxonomy {
    /// Fraction of signal-bearing spans that were actionable (covered /
    /// (covered + declined)); proposer-blind spans are excluded from the
    /// denominator — the axis does not speak for them.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // span counts — display metric
    pub fn coverage(&self) -> Option<f32> {
        let d = self.covered + self.declined;
        (d > 0).then(|| self.covered as f32 / d as f32)
    }

    /// Fraction of signal-bearing spans where the system would decline
    /// (declined / (covered + declined)) — the complement of coverage.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // span counts — display metric
    pub fn abstention_rate(&self) -> Option<f32> {
        let d = self.covered + self.declined;
        (d > 0).then(|| self.declined as f32 / d as f32)
    }

    /// Accuracy over answered (covered) spans — correct / covered.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // span counts — display metric
    pub fn selective_accuracy(&self) -> Option<f32> {
        (self.covered > 0).then(|| self.selective_correct as f32 / self.covered as f32)
    }

    /// Absorb one span's classification (the per-span facts come from
    /// [`classify_choice_span`]).
    pub fn absorb(&mut self, s: SpanChoice) {
        self.labeled += 1;
        match s.outcome {
            ChoiceOutcome::CorrectRule => self.correct_rule += 1,
            ChoiceOutcome::CorrectAcceptable => self.correct_via_acceptable += 1,
            ChoiceOutcome::WrongRule => self.wrong_rule += 1,
            ChoiceOutcome::WrongSpan => self.wrong_span += 1,
        }
        let correct = matches!(
            s.outcome,
            ChoiceOutcome::CorrectRule | ChoiceOutcome::CorrectAcceptable
        );
        match s.decline {
            DeclineClass::Covered => {
                self.covered += 1;
                self.selective_correct += usize::from(correct);
            }
            DeclineClass::Declined => self.declined += 1,
            DeclineClass::ProposerBlind => self.proposer_blind += 1,
        }
    }
}

/// The three-way decline classification for one span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclineClass {
    /// ≥1 window candidate instantiated — actionable.
    Covered,
    /// Probeable candidates present, none fired — a real decline.
    Declined,
    /// No probeable candidate — the signal is absent.
    ProposerBlind,
}

/// One span's classification facts (the pure seam — no matcher needed).
/// The outcome is categorical by construction: the four verdicts partition
/// the labeled set, so an enum (not flags) makes a span with two outcomes
/// unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanChoice {
    pub outcome: ChoiceOutcome,
    pub decline: DeclineClass,
}

/// The four decision outcomes (CUA-S1's `wrong_action`/`wrong_target`
/// decomposition adopted; the two correct halves separated so an authored
/// acceptable alternative stays visible beside strict gold).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChoiceOutcome {
    /// Gold rule ranked #1 — strict.
    CorrectRule,
    /// An authored acceptable-alternative won top-1 (counted correct; the
    /// strict number stays `correct_rule`). CUA-S1 `acceptable` in gold.
    CorrectAcceptable,
    /// Gold (or an acceptable alternative) surfaced in the window but was
    /// outranked — the fix is discrimination (rerank/scorer). CUA-S1
    /// `wrong_action` → mis-ranked rule.
    WrongRule,
    /// Neither gold nor any acceptable alternative surfaced in the probe
    /// window — the fix is the span pipeline (chunk/embed/index). CUA-S1
    /// `wrong_target` → mis-targeted span.
    WrongSpan,
}

/// Classify one labeled span against its own-domain ranked window.
///
/// Pure over its inputs — unit-testable without a matcher. `ranked` is the
/// canonical-ranked window the eval already computed; `probeable` is
/// [`super::instantiation_rerank::probeable`] threaded in as a closure so
/// this function compiles under every feature combination.
pub fn classify_choice_span<'a, I, F>(
    label: &str,
    acceptable: &[String],
    ranked: I,
    probeable: F,
) -> SpanChoice
where
    I: IntoIterator<Item = &'a crate::draft::ScoredRule>,
    F: Fn(&str) -> bool,
{
    let ranked: Vec<&crate::draft::ScoredRule> = ranked.into_iter().collect();
    let rank_strict = ranked.iter().position(|r| r.rule_id == label);
    let rank_acceptable = acceptable
        .iter()
        .filter_map(|a| ranked.iter().position(|r| r.rule_id == *a))
        .min();
    let best_correct_rank = match (rank_strict, rank_acceptable) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let correct_strict = rank_strict == Some(0);
    let correct_acceptable =
        !correct_strict && rank_acceptable == Some(0) && rank_strict != Some(0);
    let surfaced = best_correct_rank.is_some();

    let any_fires = ranked.iter().any(|r| r.instantiates);
    let any_probeable = ranked.iter().any(|r| probeable(&r.rule_id));
    let decline = if any_fires {
        DeclineClass::Covered
    } else if any_probeable {
        DeclineClass::Declined
    } else {
        DeclineClass::ProposerBlind
    };

    SpanChoice {
        outcome: if correct_strict {
            ChoiceOutcome::CorrectRule
        } else if correct_acceptable {
            ChoiceOutcome::CorrectAcceptable
        } else if !surfaced {
            ChoiceOutcome::WrongSpan
        } else {
            ChoiceOutcome::WrongRule
        },
        decline,
    }
}

/// The full eval output.
#[derive(Debug, Default)]
pub struct EvalReport {
    /// Fixed domain order: [`DomainId::ALL`] (clippy, perf, kernel, sec,
    /// docker). Entries with `labeled == 0` are disabled (feature off) or
    /// had no fixtures.
    ///
    /// Sized by `DomainId::ALL.len()`, NOT by the number of domains that
    /// happen to carry fixtures. Before the docker landing this array was
    /// `[_; 3]` while `domain_index(Sec)` already returned 3 — a latent
    /// out-of-bounds that never fired only because no sec-labeled fixture
    /// existed. Adding a 5th domain would have made that two indices out.
    pub per_domain: [DomainMetrics; DomainId::ALL.len()],
    /// All labeled spans across enabled domains.
    pub blended: DomainMetrics,
    /// Misses (expected rule outside its domain's top-3).
    pub misses: Vec<MissRecord>,
    /// Clean spans: span tag → top-1 rule per ENABLED domain (the
    /// false-positive profile — retrieval always suggests something).
    pub clean_profile: Vec<(&'static str, Vec<(DomainId, String)>)>,
    /// Router annotation quality on labeled spans (feeds the Issue 020
    /// re-gate decision): verdict == expected domain.
    pub router_checked: usize,
    pub router_correct: usize,
    /// Labeled spans skipped because their domain's index is feature-off.
    pub skipped: usize,
    /// Issue 070 T3: labeled spans whose own-domain post-rerank top-k
    /// ([`PROBE_TOP_K`]) collapsed to one rule family while the
    /// ground-truth family sits outside it. Purely diagnostic — selection
    /// is untouched, this observes the ranking the eval already computed —
    /// so an empty list is the pre-T3 behaviour, byte for byte.
    pub collapsed_pools: Vec<CollapseRecord>,
    /// Issue 125 (CUA-S1 metric adoption): the decision-outcome taxonomy —
    /// wrong-rule vs wrong-span decomposition, acceptable-alternative hits,
    /// and the three-way decline axis (covered / declined / proposer-blind)
    /// over the same labeled population as `blended`.
    pub taxonomy: ChoiceTaxonomy,
    /// Issue 134 T1: the SAME taxonomy partitioned by the span's own domain
    /// (the demonstrability column's eval half). Sized by
    /// `DomainId::ALL.len()` — the `per_domain` sizing law (an index exists
    /// for every domain whether or not it carries fixtures).
    pub taxonomy_by_domain: [ChoiceTaxonomy; DomainId::ALL.len()],
    /// Issue 134 T1: the SAME taxonomy keyed by gold rule — the per-rule
    /// demonstrability rows above the n≥20 floor live here (most rules
    /// carry 1–3 fixtures and never reach the floor; the map is the honest
    /// record either way). The `DomainId` is the span's own domain.
    pub taxonomy_by_rule:
        std::collections::BTreeMap<String, (DomainId, ChoiceTaxonomy)>,
    /// Issue 134 T4: the HOMOGENEOUS-budget control — the own-domain
    /// engine run at the fan-out's TOTAL pool budget
    /// (`n_enabled · RERANK_POOL` depth on ONE domain instead of
    /// `RERANK_POOL` on each). Depth-without-diversity: if the fan-out's
    /// blended top-1 were merely a budget effect, this row would match it.
    /// Only `labeled`/`hits_at_1` are populated (the control is a top-1
    /// claim, not a recall ladder).
    pub homogeneous: DomainMetrics,
    /// Issue 121: the Vendi axis — the geometric diversity of the DELIVERED
    /// slate (the per-domain top-[`PROBE_TOP_K`] latents, exact cosine-
    /// kernel certificate via katgpt-core `set_admission`). `None` when the
    /// `set_rerank` feature is off (the axis needs the primitive); beside
    /// [`Self::collapsed_pools`], never replacing it — the family-entropy
    /// detector and the geometric certificate are complementary axes (the
    /// family space exceeds the 8-D latent dims).
    #[cfg(feature = "set_rerank")]
    pub vendi_axis: Option<VendiAxis>,
}

/// Issue 121: the geometric-diversity summary of the delivered slates —
/// one exact-Vendi certificate per labeled span's own-domain top-k.
#[cfg(feature = "set_rerank")]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct VendiAxis {
    /// Spans certified (every labeled span whose domain is enabled).
    pub n_spans: usize,
    /// Mean exact Vendi score over the delivered top-k slates.
    pub mean_vendi: f32,
    /// Slates at the depth ceiling (`vendi >= 0.95·min(K, DIM)` — the
    /// honest saturation flag, never silent).
    pub n_saturated: usize,
    /// Slates the certificate flags collapsed (`vendi < rho·min(K, DIM)`),
    /// the GEOMETRIC axis — compare against `collapsed_pools.len()` (the
    /// categorical axis) to see how often they agree.
    pub n_cert_collapsed: usize,
}

/// The clean-span set: REAL production excerpts from this crate (shipped
/// code, not synthetic). None may fire any of the 40 corpus lints — the
/// committed clean test below enforces it against the proposers.
///
/// `pub` for the issue-125 harness (skip-row rendering).
pub const CLEAN_SPANS: &[(&str, &str)] = &[
    (
        "clean:suggest_scratch_capacity",
        r#"pub struct SuggestScratch {
    retrieved: Vec<riir_rag::RetrievedChunk>,
    best: HashMap<String, f32>,
    ranked: Vec<ScoredRule>,
}

impl SuggestScratch {
    pub fn retrieved_capacity(&self) -> usize {
        self.retrieved.capacity()
    }

    pub fn ranked_capacity(&self) -> usize {
        self.ranked.capacity()
    }
}
"#,
    ),
    (
        "clean:all_rules_merge",
        r#"impl SpanSuggestion {
    pub fn all_rules(&self) -> Vec<&ScoredRule> {
        let mut all: Vec<&ScoredRule> = self
            .by_domain
            .iter()
            .flat_map(|(_, rules)| rules.iter())
            .collect();
        all.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all
    }
}
"#,
    ),
    (
        "clean:rule_index_len",
        r#"impl RuleIndex {
    pub fn len(&self) -> usize {
        self.n_rules
    }

    pub fn is_empty(&self) -> bool {
        self.by_zone_hash.is_empty()
    }
}
"#,
    ),
    (
        // Issue 031 H1: the exact katgpt-rs SensitivityCache production shape —
        // `self.len() == 0` inside `fn is_empty` is the ONLY legitimate spelling
        // of that method body; rewriting it to `self.is_empty()` is infinite
        // recursion. comparison_to_empty (and every other structural lint) must
        // stay silent here.
        "clean:is_empty_len_zero_body",
        r#"impl SensitivityCache {
    pub fn len(&self) -> usize {
        self.cache.pin().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
"#,
    ),
    (
        "clean:kernel_rule_entry",
        r#"#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelRule {
    pub rule_id: String,
    pub gpu_concept: String,
    pub ref_shape: String,
    pub opt_pattern: String,
}

impl HealEntry for KernelRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.ref_shape
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.opt_pattern
    }
}
"#,
    ),
    (
        "clean:swallow_following_space",
        r#"pub fn swallow_following_space(source: &str, mut e: SourceEdit) -> SourceEdit {
    if source[e.end..].starts_with(' ') {
        e.end += 1;
    }
    e
}
"#,
    ),
    (
        "clean:bare_len_call",
        r#"fn bare_len_call(e: &Expr) -> Option<&ExprMethodCall> {
    let Expr::MethodCall(m) = e else {
        return None;
    };
    if m.method == "len" && m.args.is_empty() && m.attrs.is_empty() && m.turbofish.is_none() {
        Some(m)
    } else {
        None
    }
}
"#,
    ),
    (
        "clean:router_route",
        r#"pub fn route(&self, code: &str) -> Option<DomainId> {
    if self.ids.is_empty() {
        return None;
    }
    let activity = ModellessEmbedder.embed(code);
    self.route_activity(&activity)
}
"#,
    ),
    (
        "clean:propose_fix_dispatch",
        r#"pub fn propose_fix(lint_name: &str, buggy_code: &str) -> Option<String> {
    let file: syn::File = syn::parse_file(buggy_code).ok()?;
    let edits = collect_rule_edits(lint_name, buggy_code, &file);
    if edits.is_empty() {
        return None;
    }
    Some(apply_edits(buggy_code, edits))
}
"#,
    ),
    (
        "clean:category_label",
        r#"impl KernelCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Memory => "A: memory",
            Self::Compute => "B: compute",
            Self::Fusion => "C: fusion",
            Self::Dispatch => "D: dispatch",
            Self::Parallelism => "E: parallelism",
        }
    }
}
"#,
    ),
    (
        "clean:span_suggestion_struct",
        r#"#[derive(Debug, Clone, PartialEq)]
pub struct SpanSuggestion {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub routed_domain: DomainId,
    pub by_domain: Vec<(DomainId, Vec<ScoredRule>)>,
}
"#,
    ),
    (
        "clean:eval_span_sort",
        r#"fn sort_by_score(rules: &mut [ScoredRule]) {
    rules.sort_unstable_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
"#,
    ),
    (
        "clean:zone_hash_sidecar",
        r#"fn zone_hash_sidecar(patterns: &[(String, String)]) -> HashMap<[u8; 32], String> {
    let mut by_zone_hash = HashMap::with_capacity(patterns.len());
    for (rule_id, text) in patterns {
        let hash = *blake3::hash(text.as_bytes()).as_bytes();
        by_zone_hash.insert(hash, rule_id.clone());
    }
    by_zone_hash
}
"#,
    ),
    (
        // Issue 028: the katgpt-speculative find_decomposition_regions shape
        // (real production excerpt, simplified to a free fn) — comment-
        // guarded single-if blocks in BOTH arms; real clippy 1.97 is SILENT
        // on both collapsible_if and collapsible_else_if here.
        "clean:comment_guarded_if_else_arms",
        r#"fn find_decomposition_regions(profile: &[f32]) -> Vec<(usize, usize)> {
    let mut regions = Vec::new();
    let mut region_start: Option<usize> = None;

    for (i, &rel) in profile.iter().enumerate() {
        if rel < 0.3 {
            // Low relevance — extend or start region.
            if region_start.is_none() {
                region_start = Some(i);
            }
        } else {
            // High relevance — close any open region.
            if let Some(start) = region_start.take() {
                regions.push((start, i));
            }
        }
    }

    regions
}
"#,
    ),
    (
        // Issue 029 T3: the probe-verified SILENT shapes for the extended
        // single_match_else wildcard set — user-enum constructor arms (clippy
        // is type-informed: only std Option/Result scrutinees fire) and a
        // BINDING second arm (the binding would be lost under `else`).
        // Real clippy 1.97 pedantic is silent on both; every structural
        // proposer must be too.
        "clean:user_enum_and_binding_second_arms",
        r#"#[derive(Clone, Copy)]
pub enum SlotState { Ok(u8), Err(u8) }
pub enum NodeKind { Leaf, None }

fn classify_slot(s: SlotState) -> u8 {
    match s {
        SlotState::Ok(v) => { let scaled = v * 2; scaled }
        SlotState::Err(e) => { let penalty = e as u8; penalty }
    }
}

fn node_weight(k: NodeKind) -> u8 {
    match k {
        NodeKind::Leaf => { let base = 1; base }
        NodeKind::None => { let zero = 0; zero }
    }
}

fn probe(cache_hit: Option<u8>) -> u8 {
    match cache_hit {
        None => { let miss = 2; miss }
        Some(v) => { let hit = v + 1; hit }
    }
}
"#,
    ),
    (
        // Slice-5 extension (2026-08-18): the real riir-train gpart.rs
        // production shapes for the newly-covered `anyhow::ensure!` macro
        // family — an already-inlined `{version}` template and a
        // no-placeholder template. Both must stay silent.
        "clean:anyhow_ensure_already_inlined",
        r#"fn check_oscpt_header(data: &[u8], pos: &mut usize) -> anyhow::Result<()> {
    anyhow::ensure!(
        data.len() >= 5 && data[..5] == b"OSCPT",
        "Invalid OSCPT magic"
    );
    *pos += 5;
    let version = u32::from_le_bytes(data[*pos..*pos + 4].try_into()?);
    anyhow::ensure!(
        version == 9,
        "Unsupported OSCPT version: {version}"
    );
    *pos += 4;
    Ok(())
}
"#,
    ),
];

/// Deterministic identifier-renaming map for the kernel self-variant probe.
/// Word-boundary-exact (inside `output` the token `out` is untouched).
const KERNEL_RENAMES: &[(&str, &str)] = &[
    ("buf", "scratch"),
    ("acc", "total"),
    ("out", "dst"),
    ("vals", "items"),
    ("tmp", "staging"),
    ("res", "answer"),
    ("idx", "slot"),
    ("n", "count"),
    ("i", "p"),
    ("j", "q"),
];

/// Rename identifiers in pseudocode text via the fixed map (deterministic,
/// no RNG). Types/keywords are untouched (map keys are value-identifier
/// shapes only).
#[must_use]
pub fn mutate_shape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 16);
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            if !token.is_empty() {
                let replacement = KERNEL_RENAMES
                    .iter()
                    .find(|(from, _)| *from == token)
                    .map_or(token.as_str(), |(_, to)| to);
                out.push_str(replacement);
                token.clear();
            }
            out.push(ch);
        }
    }
    if !token.is_empty() {
        let replacement = KERNEL_RENAMES
            .iter()
            .find(|(from, _)| *from == token)
            .map_or(token.as_str(), |(_, to)| to);
        out.push_str(replacement);
    }
    out
}

/// The kernel self-variant probe: every `step`-th corpus rule's `ref_shape`,
/// mutated. Deterministic across runs; the SELECTED SET shifts as batches
/// append entries (intended — the gate re-measures the moved set).
#[cfg(feature = "kernel_opt")]
fn kernel_fixture_spans(target: usize) -> Vec<(String, String)> {
    let entries = crate::domains::kernel_opt::corpus::build_kernel_seed_corpus().entries;
    let step = (entries.len() / target).max(1);
    entries
        .iter()
        .step_by(step)
        .take(target)
        .map(|e| (e.rule_id.clone(), mutate_shape(&e.ref_shape)))
        .collect()
}

/// How deep the per-domain ranked list is probed (ranks 0..8 give the
/// near-miss signal for the taxonomy).
const PROBE_TOP_K: usize = 8;

/// The labeled eval span set as OWNED strings (shared by `run_eval` + the
/// Issue 020 router eval — both must measure the SAME spans).
///
/// `pub`: the parameterized probe entry points ([`run_headroom_probe_with_spans`])
/// take `&[OwnedEvalSpan]`, so a caller must be able to name the set it
/// feeds in — T0.2's oracle-labeled real-repo span sets are constructed
/// outside this module — and Issue 126's trainer consumes the SAME set for
/// its TRAIN split (one fixture source, never a copy that can drift).
pub struct OwnedEvalSpan {
    pub label: String,
    pub domain: DomainId,
    pub code: String,
    /// Issue 125 (CUA-S1 metric adoption): authored acceptable-alternative
    /// gold rules — rules a human would score as CORRECT for this span even
    /// though the fixture's primary label differs. Empty everywhere today:
    /// the axis ships with the scoring semantics and ZERO authored sets,
    /// because an acceptable set is a corpus claim (two rules produce a fix
    /// a reviewer would accept for THIS span), never a heuristic guess.
    /// The `PoC`'s choice-row rendering (issue 125 T1) is the intended author.
    pub acceptable: Vec<String>,
}

#[must_use]
pub fn labeled_eval_spans() -> Vec<OwnedEvalSpan> {
    // the pushes are cfg'd per-domain; thin sets leave it empty
    #[cfg_attr(not(feature = "clippy_lints"), allow(unused_mut))]
    let mut spans = Vec::new();
    #[cfg(feature = "clippy_lints")]
    for (lint, code) in super::eval_fixtures_clippy::CLIPPY_SPANS {
        spans.push(OwnedEvalSpan {
            label: lint.to_string(),
            domain: DomainId::Clippy,
            code: code.to_string(),
            acceptable: Vec::new(),
        });
    }
    #[cfg(feature = "rust_perf")]
    for (rule, code) in super::eval_fixtures_perf::PERF_SPANS {
        spans.push(OwnedEvalSpan {
            label: rule.to_string(),
            domain: DomainId::Perf,
            code: code.to_string(),
            acceptable: Vec::new(),
        });
    }
    #[cfg(feature = "kernel_opt")]
    for (rule, code) in kernel_fixture_spans(30) {
        spans.push(OwnedEvalSpan {
            label: rule,
            domain: DomainId::Kernel,
            code,
            acceptable: Vec::new(),
        });
    }
    // Issue 134 T3 re-arm — the thin-domain labeled sets, in `DomainId::ALL`
    // order. Each block is inert until its domain feature is on; the spans
    // are hand-authored variants of the corpus anti-patterns (the
    // `eval_fixtures_perf` convention, one span per rule).
    #[cfg(feature = "sec")]
    for (rule, code) in super::eval_fixtures_sec::SEC_SPANS {
        spans.push(OwnedEvalSpan {
            label: rule.to_string(),
            domain: DomainId::Sec,
            code: code.to_string(),
            acceptable: Vec::new(),
        });
    }
    #[cfg(feature = "docker")]
    for (rule, code) in super::eval_fixtures_docker::DOCKER_SPANS {
        spans.push(OwnedEvalSpan {
            label: rule.to_string(),
            domain: DomainId::Docker,
            code: code.to_string(),
            acceptable: Vec::new(),
        });
    }
    #[cfg(feature = "dist")]
    for (rule, code) in super::eval_fixtures_dist::DIST_SPANS {
        spans.push(OwnedEvalSpan {
            label: rule.to_string(),
            domain: DomainId::Dist,
            code: code.to_string(),
            acceptable: Vec::new(),
        });
    }
    #[cfg(feature = "shader_perf")]
    for (rule, code) in super::eval_fixtures_shader::SHADER_SPANS {
        spans.push(OwnedEvalSpan {
            label: rule.to_string(),
            domain: DomainId::ShaderPerf,
            code: code.to_string(),
            acceptable: Vec::new(),
        });
    }
    spans
}

pub(crate) fn domain_index(d: DomainId) -> usize {
    match d {
        DomainId::Clippy => 0,
        DomainId::Perf => 1,
        DomainId::Kernel => 2,
        DomainId::Sec => 3,
        DomainId::Docker => 4,
        DomainId::Dist => 5,
        DomainId::ShaderPerf => 6,
    }
}

/// Canonical order for the matcher's returned rules: score-descending with
/// a `rule_id` tie-break. The matcher's own ordering resolves score TIES by
/// backing-map iteration order, which can flip between runs — the eval is a
/// MEASUREMENT, so it must be reproducible (Issue 022: a corpus change put
/// a tie exactly on the top-5 boundary + the in-process determinism test
/// caught it). A substrate-level deterministic tie-break in riir-rag remains
/// a documented follow-up; this keeps the eval self-contained.
///
/// `pub` — shared by the Issue 024 switch-cost measurement and the
/// Issue 64 modelability sweep (tests/) so all three measurements rank
/// identically.
#[must_use]
pub fn canonical_rank(rules: &[crate::draft::ScoredRule]) -> Vec<&crate::draft::ScoredRule> {
    let mut v: Vec<&crate::draft::ScoredRule> = rules.iter().collect();
    // MUST match `instantiation_rerank::rerank`'s key order — instantiation
    // first, then similarity, then rule_id. Sorting by score alone here would
    // silently undo the T3.5.4 re-rank before it was ever measured.
    v.sort_by(|a, b| {
        b.instantiates
            .cmp(&a.instantiates)
            .then_with(|| b.score.total_cmp(&a.score))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
    v
}

/// Assemble + run the full eval against every feature-enabled domain.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails (no domain
/// feature enabled — the committed suites always enable at least
/// `clippy_lints` + `rust_perf`).
#[must_use]
pub fn run_eval() -> EvalReport {
    run_eval_with(crate::draft::RetrievalConfig::default())
}

/// [`run_eval`] against an explicit re-rank mode (Plan 005 T3.5.4 A/B).
#[must_use]
pub fn run_eval_reranked(mode: crate::draft::RerankMode) -> EvalReport {
    run_eval_inner(crate::draft::RetrievalConfig::default(), mode)
}

/// [`run_eval`] against an explicit retrieval config — the Plan 005 T3.5 A/B
/// entry point. `run_eval` delegates with the promoted default.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains_with`] fails.
#[must_use]
pub fn run_eval_with(config: crate::draft::RetrievalConfig) -> EvalReport {
    run_eval_inner(config, crate::draft::RerankMode::default())
}

/// The shared eval body — both A/B axes explicit.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains_with`] fails.
#[must_use]
pub fn run_eval_inner(
    config: crate::draft::RetrievalConfig,
    rerank: crate::draft::RerankMode,
) -> EvalReport {
    run_eval_on(
        LatentMatcher::from_enabled_domains_with(config)
            .expect("build matcher")
            .with_rerank_mode(rerank),
    )
}

/// The eval body over a caller-built matcher — the seam for A/B knobs that
/// live on the matcher rather than on [`crate::draft::RetrievalConfig`]
/// (katgpt-rs Issue 882 P0 rider: `with_differential_anchor`).
#[must_use]
#[allow(clippy::too_many_lines)] // the measurement: every labeled span × every axis in one deterministic walk
#[allow(clippy::cast_precision_loss)] // the vendi mean divides a count — display precision
pub fn run_eval_on(mut matcher: LatentMatcher) -> EvalReport {
    let mut report = EvalReport::default();
    // Issue 121: the Vendi-axis accumulator (feature-gated with the field).
    #[cfg(feature = "set_rerank")]
    let mut matcher_vendi: Option<VendiAxis> = None;

    let spans = labeled_eval_spans();
    for span in &spans {
        let label = span.label.as_str();
        let domain = span.domain;
        let code = span.code.as_str();
        if !matcher.has_domain(domain) {
            report.skipped += 1;
            continue;
        }
        let (routed, by_domain) = matcher.suggest_span(code, PROBE_TOP_K);
        let ranked = by_domain
            .iter()
            .find(|(d, _)| *d == domain)
            .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));

        // Issue 121: the Vendi axis — certify the DELIVERED slate (this
        // span's own-domain top-k latents). Measurement only: the axis rides
        // the ranking the eval already computed, never reorders it.
        #[cfg(feature = "set_rerank")]
        {
            use katgpt_core::set_admission::certify_set;
            let cfg = super::set_rerank::SetRerankConfig::default();
            let corpus = matcher.rule_latents();
            let take = PROBE_TOP_K.min(ranked.len());
            let latents: Vec<[f32; katgpt_core::set_admission::DIM]> = ranked[..take]
                .iter()
                .filter_map(|r| corpus.latent_of(cfg.latent, &r.rule_id))
                .collect();
            let cert = certify_set(&cfg.admission, &latents);
            let axis = matcher_vendi.get_or_insert_with(VendiAxis::default);
            axis.n_spans += 1;
            axis.mean_vendi += cert.vendi;
            axis.n_saturated += usize::from(cert.saturated);
            axis.n_cert_collapsed += usize::from(cert.collapsed);
        }
        let rank = ranked.iter().position(|r| r.rule_id == label);
        let top1 = ranked.first().map(|r| r.rule_id.clone());

        // Issue 070 T3 diagnostic: record a top-k family collapse with the
        // oracle OUTSIDE the collapsed family (the crowding-evidence shape).
        // The small `ids` vec is measurement-path scratch — the production
        // heal path never calls the detector with an oracle.
        let ids: Vec<&str> = ranked.iter().map(|r| r.rule_id.as_str()).collect();
        if let CollapseVerdict::Collapsed {
            family,
            pool_top_k,
            oracle_outside: true,
        } = detect_collapse_rule_ids(&ids, PROBE_TOP_K, Some(label))
        {
            report.collapsed_pools.push(CollapseRecord {
                label: label.to_string(),
                domain,
                family: family.to_string(),
                pool_top_k,
            });
        }

        report.router_checked += 1;
        if routed == domain {
            report.router_correct += 1;
        }

        let di = domain_index(domain);
        let m = &mut report.per_domain[di];
        m.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            m.hits_at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            m.hits_at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            m.hits_at_5 += 1;
        }
        let b = &mut report.blended;
        b.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            b.hits_at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            b.hits_at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            b.hits_at_5 += 1;
        }
        if rank.is_none_or(|r| r >= 3) {
            let component = crate::component::classify(&crate::component::FailureSignal {
                expected_rule: Some(label.to_string()),
                expected_rank: rank,
                any_candidates: top1.is_some(),
                ..crate::component::FailureSignal::default()
            });
            report.misses.push(MissRecord {
                expected: label.to_string(),
                domain,
                rank,
                top1,
                component,
            });
        }

        // Issue 125 taxonomy: classify the span against the SAME ranked
        // window the recall axes above consumed. The decline axis reads the
        // `instantiates` flags as this run marked them (Structural by
        // default; under `RerankMode::Off` nothing is marked, so every
        // signal-bearing span reads Declined — the honest reading of a run
        // that ran no probe).
        let choice = classify_choice_span(
            label,
            &span.acceptable,
            ranked.iter().copied(),
            |rule_id| super::instantiation_rerank::probeable(domain, rule_id),
        );
        report.taxonomy.absorb(choice);
        // Issue 134 T1: the same choice, partitioned by domain + gold rule —
        // the demonstrability column's eval half (SpanChoice is Copy, so
        // the second absorb is free).
        report.taxonomy_by_domain[di].absorb(choice);
        report
            .taxonomy_by_rule
            .entry(label.to_string())
            .or_insert((domain, ChoiceTaxonomy::default()))
            .1
            .absorb(choice);
    }

    // Issue 134 T4: the HOMOGENEOUS-budget control — every labeled span
    // answered by its OWN domain's engine at the fan-out's TOTAL pool
    // budget (`n_enabled · RERANK_POOL` depth on one domain instead of
    // `RERANK_POOL` on each). Depth-without-diversity: if blended top-1
    // were merely a budget effect, this matches it. The T3 sweep measured
    // the flat-curve shape this rides (zero gain at 4× depth); this row
    // pins it in the standing report at the exact equalized budget.
    let budget = matcher
        .domains()
        .len()
        .max(1)
        .saturating_mul(super::instantiation_rerank::RERANK_POOL);
    for span in &spans {
        if !matcher.has_domain(span.domain) {
            continue;
        }
        let (_, by_domain) = matcher.suggest_span(&span.code, budget);
        let ranked = by_domain
            .iter()
            .find(|(d, _)| *d == span.domain)
            .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
        report.homogeneous.labeled += 1;
        if ranked.first().is_some_and(|r| r.rule_id == span.label) {
            report.homogeneous.hits_at_1 += 1;
        }
    }

    for (tag, code) in CLEAN_SPANS {
        let (routed, by_domain) = matcher.suggest_span(code, PROBE_TOP_K);
        let _ = routed;
        let tops: Vec<(DomainId, String)> = by_domain
            .iter()
            .filter_map(|(d, rules)| {
                canonical_rank(rules)
                    .first()
                    .map(|r| (*d, r.rule_id.clone()))
            })
            .collect();
        report.clean_profile.push((tag, tops));
    }

    // Finalize the Vendi axis (mean over the certified spans).
    #[cfg(feature = "set_rerank")]
    {
        if let Some(axis) = &mut matcher_vendi {
            if axis.n_spans > 0 {
                axis.mean_vendi /= axis.n_spans as f32;
            }
        }
        report.vendi_axis = matcher_vendi;
    }

    report
}

/// Issue 126: the trained two-tower posture's split-aware measurement —
/// BOTH postures (hashed baseline + baseline ⊕ trained term) over the SAME
/// labeled spans, bucketed by the rule-level split
/// ([`super::rule_embed::fixture_is_withheld`]), plus the gates:
///
/// - **G2 (the win gate):** full-rig `hits_at_1` (train + withheld) must
///   BEAT the baseline's — movement on the committed axis (the +55.6pp
///   Issue-121 precedent). The withheld split alone is NOT the win
///   denominator: measured 2026-09-20, the baseline is already perfect
///   there (31/31) — zero headroom, a win gate on it can never pass. The
///   withheld split is the ANTI-MEMORIZATION read instead:
/// - **G2-OOD (no harm):** withheld `hits_at_1` trained ≥ baseline. A train
///   split win with a withheld regression would be memorization, not value.
/// - **G1 (no regressions):** spans the baseline ranks top-1 that the
///   trained posture does not — counted, gate is zero. The trained term may
///   not cost the pipeline a span it already had.
/// - **G3 (clean profile):** clean spans whose top-1 is a probeable
///   (fixable-lint) rule — the false-positive class — must not increase.
///
/// Both matchers run `RerankMode::default()` (Structural — the shipped
/// posture); the trained arm differs ONLY by the attached tables.
#[cfg(feature = "rule_embed_poc")]
#[derive(Debug, Default, Clone)]
pub struct SplitReport {
    pub baseline_train: DomainMetrics,
    pub baseline_withheld: DomainMetrics,
    pub trained_train: DomainMetrics,
    pub trained_withheld: DomainMetrics,
    /// G1: baseline top-1 spans the trained posture lost.
    pub regressions_vs_baseline: usize,
    /// G3: clean spans whose top-1 is probeable (the FP class).
    pub clean_top1_fixable_baseline: usize,
    pub clean_top1_fixable_trained: usize,
    /// G3 evidence: clean spans whose top-1 identity CHANGED per domain
    /// (tag, domain, baseline rule, trained rule).
    pub clean_top1_changes: Vec<(&'static str, DomainId, String, String)>,
    /// Failure anatomy: every labeled span that EITHER posture missed at
    /// top-1 — (label, withheld, baseline rank, trained rank). This is the
    /// population the win gate reads; `None` = not retrieved in the probe
    /// window at all.
    pub miss_detail: Vec<(String, bool, Option<usize>, Option<usize>)>,
}

#[cfg(feature = "rule_embed_poc")]
fn bump_metrics(m: &mut DomainMetrics, rank: Option<usize>) {
    m.labeled += 1;
    if rank.is_some_and(|r| r < 1) {
        m.hits_at_1 += 1;
    }
    if rank.is_some_and(|r| r < 3) {
        m.hits_at_3 += 1;
    }
    if rank.is_some_and(|r| r < 5) {
        m.hits_at_5 += 1;
    }
}

/// Run the split-aware baseline-vs-trained measurement.
///
/// # Panics
///
/// Panics if the matcher cannot be built (no domain feature enabled).
#[cfg(feature = "rule_embed_poc")]
#[must_use]
pub fn run_split_eval(
    tables: &std::sync::Arc<super::rule_embed::TrainedTables>,
    weight: f32,
) -> SplitReport {
    use super::rule_embed::fixture_is_withheld;
    let build = || {
        LatentMatcher::from_enabled_domains_with(crate::draft::RetrievalConfig::default())
            .expect("build matcher")
            .with_rerank_mode(crate::draft::RerankMode::default())
    };
    let mut baseline = build();
    let mut trained = build().with_rule_embed(tables, weight);
    let mut rep = SplitReport::default();

    for span in labeled_eval_spans() {
        let withheld = fixture_is_withheld(&span.label);
        let mut ranks = [None; 2];
        for (slot, matcher) in [&mut baseline, &mut trained].iter_mut().enumerate() {
            let (_, by_domain) = matcher.suggest_span(&span.code, PROBE_TOP_K);
            let ranked = by_domain
                .iter()
                .find(|(d, _)| *d == span.domain)
                .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
            ranks[slot] = ranked.iter().position(|r| r.rule_id == span.label);
        }
        let [b_rank, t_rank] = ranks;
        let (bm, tm) = if withheld {
            (&mut rep.baseline_withheld, &mut rep.trained_withheld)
        } else {
            (&mut rep.baseline_train, &mut rep.trained_train)
        };
        bump_metrics(bm, b_rank);
        bump_metrics(tm, t_rank);
        // G1: a span the baseline got right that the trained posture lost.
        if b_rank == Some(0) && t_rank != Some(0) {
            rep.regressions_vs_baseline += 1;
        }
        if b_rank != Some(0) || t_rank != Some(0) {
            rep.miss_detail.push((span.label.clone(), withheld, b_rank, t_rank));
        }
    }

    for (tag, code) in CLEAN_SPANS {
        let (_, b_by) = baseline.suggest_span(code, PROBE_TOP_K);
        let (_, t_by) = trained.suggest_span(code, PROBE_TOP_K);
        for (d, brules) in &b_by {
            let b_ranked = canonical_rank(brules);
            let Some(btop) = b_ranked.first() else {
                continue;
            };
            let Some((_, trules)) = t_by.iter().find(|(td, _)| td == d) else {
                continue;
            };
            let t_ranked = canonical_rank(trules);
            let Some(ttop) = t_ranked.first() else {
                continue;
            };
            if super::instantiation_rerank::probeable(*d, &btop.rule_id) {
                rep.clean_top1_fixable_baseline += 1;
            }
            if super::instantiation_rerank::probeable(*d, &ttop.rule_id) {
                rep.clean_top1_fixable_trained += 1;
            }
            if btop.rule_id != ttop.rule_id {
                rep.clean_top1_changes
                    .push((tag, *d, btop.rule_id.clone(), ttop.rule_id.clone()));
            }
        }
    }
    rep
}

/// Human-readable metrics table (the `--nocapture` output of the report
/// test — the one-command harness).
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
#[allow(clippy::too_many_lines)] // one table per report — the measurement surface
pub fn print_report(report: &EvalReport) {
    println!("== Issue 017 retrieval-precision eval ==");
    // Rider (Issue 134 T4's edit): the seventh domain's row — `names` had 6
    // entries while `per_domain` is sized by `DomainId::ALL.len()` (7), so
    // shader_perf's metrics never printed (the row-key-omits-a-domain
    // drift class).
    let names = [
        "clippy", "perf  ", "kernel", "sec   ", "docker", "dist  ", "shader",
    ];
    println!(
        "{:<8} {:>8} {:>9} {:>9} {:>9}",
        "domain", "labeled", "top-1", "top-3", "top-5"
    );
    for (i, name) in names.iter().enumerate() {
        let m = &report.per_domain[i];
        if m.labeled == 0 {
            println!("{name:<8} {:>8} {:>9} {:>9} {:>9}", 0, "-", "-", "-");
            continue;
        }
        println!(
            "{name:<8} {:>8} {:>8.1}% {:>8.1}% {:>8.1}%",
            m.labeled,
            m.recall(1).map_or(0.0, |r| r * 100.0),
            m.recall(3).map_or(0.0, |r| r * 100.0),
            m.recall(5).map_or(0.0, |r| r * 100.0),
        );
    }
    println!(
        "blended  {:>8} {:>8.1}% {:>8.1}% {:>8.1}%   (skipped: {})",
        report.blended.labeled,
        report.blended.recall(1).map_or(0.0, |r| r * 100.0),
        report.blended.recall(3).map_or(0.0, |r| r * 100.0),
        report.blended.recall(5).map_or(0.0, |r| r * 100.0),
        report.skipped,
    );
    // Issue 134 T4 — the fanout controls (the paper's linearization /
    // homogeneous controls, Research 585): any "fanout wins" claim reading
    // this report must beat BOTH rows.
    if report.blended.labeled > 0 {
        // best-single-domain: ONE fixed domain engine answering EVERY span.
        // It can only ever be right on its own spans, so its whole-set top-1
        // is `hits_at_1_d / blended.labeled` — the structural disclosure
        // (the fan-out's cross-domain coverage IS the margin over any single
        // engine; the row pins that ceiling so a claim quotes it, never the
        // per-domain column, which speaks only for its own spans).
        let mut best: Option<(usize, &str, usize)> = None; // (i, name, hits)
        for (i, name) in names.iter().enumerate() {
            let m = &report.per_domain[i];
            if m.labeled == 0 {
                continue;
            }
            if best.is_none_or(|(_, _, h)| m.hits_at_1 > h) {
                best = Some((i, name, m.hits_at_1));
            }
        }
        if let Some((bi, bname, bhits)) = best {
            let share = bhits as f32 / report.blended.labeled as f32 * 100.0;
            println!(
                "  fanout controls (134 T4 — a fanout-wins claim must beat both):");
            println!(
                "  best-single-domain  top-1 {share:.1}% on the whole set ({:<6} answering every span — its own-share ceiling; per-domain {:>5.1}% on its spans)",
                bname.trim_end(),
                report.per_domain[bi].recall(1).map_or(0.0, |r| r * 100.0),
            );
        }
        // homogeneous: the own-domain engine at the fan-out's TOTAL pool
        // budget — depth-without-diversity. Populated by `run_eval_inner`'s
        // equalized-budget pass; `labeled == 0` means the control did not
        // run (a hand-built report), and the row says so rather than 0.0.
        if report.homogeneous.labeled > 0 {
            let h = report
                .homogeneous
                .recall(1)
                .map_or(0.0, |r| r * 100.0);
            let b = report.blended.recall(1).map_or(0.0, |r| r * 100.0);
            println!(
                "  homogeneous @budget top-1 {h:.1}% (one engine, pool = n_enabled x RERANK_POOL) — delta vs blended {:+.1}pp",
                h - b
            );
        } else {
            println!(
                "  homogeneous @budget — not measured by this report's run (run_eval_inner populates it)"
            );
        }
    }
    println!(
        "router annotation: {}/{} correct ({:.1}%) — feeds the Issue 020 re-gate",
        report.router_correct,
        report.router_checked,
        if report.router_checked == 0 {
            0.0
        } else {
            report.router_correct as f32 / report.router_checked as f32 * 100.0
        },
    );
    println!("\n-- miss taxonomy (expected rule outside its domain top-3) --");
    if report.misses.is_empty() {
        println!("none");
    }
    for miss in &report.misses {
        match miss.rank {
            Some(r) => println!(
                "  {} outranked  [{:?}] expected={} rank={} top1={}",
                miss.component.as_str(),
                miss.domain,
                miss.expected,
                r,
                miss.top1.as_deref().unwrap_or("?"),
            ),
            None => println!(
                "  {} not-in-top{} [{:?}] expected={} top1={}",
                miss.component.as_str(),
                PROBE_TOP_K,
                miss.domain,
                miss.expected,
                miss.top1.as_deref().unwrap_or("?"),
            ),
        }
    }
    // Issue 047: per-class counts — the dominant retrieval-stage component
    // is the next tuning target (pool depth, fan-out) instead of a guess.
    if !report.misses.is_empty() {
        let mut e = 0usize;
        let mut rho = 0usize;
        for miss in &report.misses {
            match miss.component {
                crate::component::ComponentClass::Retrieval => rho += 1,
                crate::component::ComponentClass::Corpus => e += 1,
                _ => {}
            }
        }
        println!("  component counts: E(corpus)={e}  rho(retrieval)={rho}");
    }
    println!("\n-- rerank-pool collapse (top-k one family, oracle outside it) --");
    if report.collapsed_pools.is_empty() {
        println!("none");
    }
    for c in &report.collapsed_pools {
        println!(
            "  COLLAPSED [{:?}] {} top-{} onto family '{}' — oracle family '{}'",
            c.domain,
            c.label,
            c.pool_top_k,
            c.family,
            rule_family(&c.label),
        );
    }
    // Issue 121: the geometric axis beside the categorical one.
    #[cfg(feature = "set_rerank")]
    if let Some(axis) = &report.vendi_axis {
        println!("\n-- vendi axis (exact cosine-kernel certificate of the delivered top-k) --");
        println!(
            "  spans {} | mean vendi {:.3} | saturated {} | cert-collapsed {} (geometric) vs family-collapsed {} (categorical)",
            axis.n_spans,
            axis.mean_vendi,
            axis.n_saturated,
            axis.n_cert_collapsed,
            report.collapsed_pools.len(),
        );
    }
    println!("\n-- clean-span false-positive profile (what tops real clean code) --");
    for (tag, tops) in &report.clean_profile {
        let tops: String = tops
            .iter()
            .map(|(d, r)| format!("{d:?}:{r}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {tag} -> {tops}");
    }
    // Issue 125 (CUA-S1 metrics.py adoption): the decision-outcome taxonomy
    // — wrong-rule/wrong-span split, acceptable-alternative hits, and the
    // three-way decline axis. Percentages only where the denominator speaks.
    println!("\n-- Issue 125 choice taxonomy (CUA-S1 metric adoption) --");
    let t = &report.taxonomy;
    println!(
        "  labeled {} | correct-rule {} (strict) | via-acceptable {} | wrong-rule {} (discrimination) | wrong-span {} (span pipeline)",
        t.labeled, t.correct_rule, t.correct_via_acceptable, t.wrong_rule, t.wrong_span,
    );
    let pct = |o: Option<f32>| o.map_or("-".to_string(), |v| format!("{:.1}%", v * 100.0));
    println!(
        "  decline axis: covered {} | declined {} | proposer-blind {} | coverage {} | abstention {} | selective-accuracy {}",
        t.covered,
        t.declined,
        t.proposer_blind,
        pct(t.coverage()),
        pct(t.abstention_rate()),
        pct(t.selective_accuracy()),
    );
    if t.correct_via_acceptable == 0 {
        // Cross-check invariant while no acceptable sets are authored: the
        // strict axis must equal the recall table's blended top-1 exactly.
        debug_assert_eq!(
            t.correct_rule, report.blended.hits_at_1,
            "taxonomy strict hits diverged from blended top-1 — one of the two axes is miscomputed"
        );
    }
    // Issue 134 T1 — the demonstrability column (eval half): per-domain
    // selection-discriminability constants, coverage-conditioned by
    // construction (correct-present denominators only; wrong-span is the
    // generation-side exclusion, reported beside). Measurement only — this
    // observes the ranking the eval already computed, never reorders it.
    // rustc_errors is never asserted ≈1 anywhere (a free coordinate in T3's
    // Spearman); the drop ledger says so where it has no labeled set.
    {
        use crate::demonstrability::{canonical_domain, eval_row, render_column, MIN_PER_RULE_N};
        let mut rows: Vec<crate::demonstrability::Row> = report
            .taxonomy_by_domain
            .iter()
            .enumerate()
            .filter(|(_, t)| t.labeled > 0)
            .map(|(i, t)| {
                eval_row(
                    canonical_domain(DomainId::ALL[i].label()),
                    None,
                    t,
                )
            })
            .collect();
        rows.extend(
            report
                .taxonomy_by_rule
                .iter()
                .filter(|(_, (_, t))| {
                    t.correct_rule + t.correct_via_acceptable + t.wrong_rule
                        >= MIN_PER_RULE_N as usize
                })
                .map(|(rule, (d, t))| {
                    eval_row(canonical_domain(d.label()), Some(rule), t)
                }),
        );
        print!("{}", render_column(&rows));
    }
}

// ── Issue 020 POC: router spectral-block eval ──────────────────────────

/// The two DOCUMENTED `Plan 005` mis-routes (`latent_matcher` doc + Bench 018):
/// a `return expr;` fn routed to perf (should → clippy) + a linear-scan fn
/// routed to clippy (should → perf). The gate requires BOTH fixed.
const MISROUTE_FIXTURES: [(&str, DomainId); 2] = [
    (
        "pub fn area(w: f64, h: f64) -> f64 {\n    return w * h;\n}\n",
        DomainId::Clippy,
    ),
    (
        "pub fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    for u in users {\n        if u.id == id {\n            return Some(u);\n        }\n    }\n    None\n}\n",
        DomainId::Perf,
    ),
];

/// Mirror of `LatentMatcher::from_enabled_domains`' router-domain assembly
/// (the same corpus pattern texts — the router eval must measure the router
/// the production path would build).
#[allow(clippy::vec_init_then_push)] // cfg-gated per-domain pushes
fn router_eval_domains() -> Vec<(DomainId, Vec<String>)> {
    #[cfg_attr(not(any(feature = "clippy_lints", feature = "rust_perf")), allow(unused_mut))]
    let mut out = Vec::new();
    #[cfg(feature = "clippy_lints")]
    out.push((
        DomainId::Clippy,
        crate::domains::clippy_lints::ClippyLintDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect::<Vec<_>>(),
    ));
    #[cfg(feature = "rust_perf")]
    out.push((
        DomainId::Perf,
        crate::domains::rust_perf::RustPerfDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect::<Vec<_>>(),
    ));
    #[cfg(feature = "kernel_opt")]
    out.push((
        DomainId::Kernel,
        crate::domains::kernel_opt::KernelOptDomain::build_corpus()
            .into_iter()
            .map(|e| e.buggy_code().to_string())
            .collect::<Vec<_>>(),
    ));
    out
}

/// One routing-variant measurement (Issue 020).
#[derive(Debug, Clone)]
pub struct RouterEvalRow {
    /// Variant name (e.g. `baseline-8d`, `dft8-disc+struct`).
    pub mode: String,
    /// Labeled spans routed.
    pub checked: usize,
    /// Spans routed to their owning domain.
    pub correct: usize,
    /// Both documented mis-routes fixed?
    pub misroutes_fixed: bool,
}

impl RouterEvalRow {
    /// Correct-route ratio (1.0 when nothing was checked — vacuous state).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // span counts ≤ ~10²
    pub fn accuracy(&self) -> f32 {
        if self.checked == 0 {
            1.0
        } else {
            self.correct as f32 / self.checked as f32
        }
    }
}

/// The routing variants measured by the POC, in report order.
#[must_use]
pub fn router_eval_modes() -> Vec<super::domain_router::RoutingEmbedding> {
    use super::domain_router::RoutingEmbedding::{self, SpectralDft};
    use super::domain_router::SpectralOrder::{Discriminative, GlobalFreq};
    vec![
        RoutingEmbedding::Baseline,
        SpectralDft {
            order: GlobalFreq,
            with_struct: false,
        },
        SpectralDft {
            order: GlobalFreq,
            with_struct: true,
        },
        SpectralDft {
            order: Discriminative,
            with_struct: false,
        },
        SpectralDft {
            order: Discriminative,
            with_struct: true,
        },
        RoutingEmbedding::TfCosine { order: GlobalFreq },
        RoutingEmbedding::TfCosine {
            order: Discriminative,
        },
    ]
}

fn router_mode_name(mode: super::domain_router::RoutingEmbedding) -> String {
    use super::domain_router::RoutingEmbedding as RE;
    use super::domain_router::SpectralOrder as SO;
    match mode {
        RE::Baseline => "baseline-8d".to_string(),
        RE::SpectralDft { order, with_struct } => format!(
            "dft8-{}{}",
            match order {
                SO::GlobalFreq => "global",
                SO::Discriminative => "disc",
            },
            if with_struct { "+struct" } else { "" }
        ),
        RE::TfCosine { order } => format!(
            "tf-cosine-{}",
            match order {
                SO::GlobalFreq => "global",
                SO::Discriminative => "disc",
            }
        ),
    }
}

/// Route every labeled eval span + the two documented mis-route fixtures
/// under every routing variant. THE Issue 020 measurement.
#[must_use]
pub fn run_router_eval() -> Vec<RouterEvalRow> {
    use super::domain_router::DomainRouter;

    let domains = router_eval_domains();
    let spans = labeled_eval_spans();
    let mut rows = Vec::new();
    for mode in router_eval_modes() {
        let router = DomainRouter::with_embedding(&domains, mode);
        let mut correct = 0usize;
        for span in &spans {
            if router.route(&span.code) == Some(span.domain) {
                correct += 1;
            }
        }
        let misroutes_fixed = MISROUTE_FIXTURES
            .iter()
            .all(|(code, expected)| router.route(code) == Some(*expected));
        rows.push(RouterEvalRow {
            mode: router_mode_name(mode),
            checked: spans.len(),
            correct,
            misroutes_fixed,
        });
    }
    rows.shrink_to_fit();
    rows
}

/// Print the router eval table (the `--nocapture` output).
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_router_eval(rows: &[RouterEvalRow]) {
    println!(
        "\n== Issue 020 router spectral POC (kill-bar: accuracy >= 85%, both mis-routes fixed) =="
    );
    for row in rows {
        println!(
            "  {:<22} {:>4}/{} ({:>5.1}%)  mis-routes: {}",
            row.mode,
            row.correct,
            row.checked,
            row.accuracy() * 100.0,
            if row.misroutes_fixed {
                "fixed"
            } else {
                "NOT fixed"
            },
        );
    }
}

/// One row of the Plan 005 T3.5 retrieval sweep.
#[derive(Debug, Clone)]
pub struct RetrievalSweepRow {
    /// The config that produced the row.
    pub config: crate::draft::RetrievalConfig,
    /// Per-domain metrics, indexed as [`EvalReport::per_domain`].
    pub per_domain: [DomainMetrics; DomainId::ALL.len()],
    /// Blended metrics across every labeled span.
    pub blended: DomainMetrics,
}

/// The T3.5 option-2 grid: BM25 tokenizer x latent fusion weight.
///
/// This is the measurement that decided both shipped defaults. It stays
/// committed so a substrate change that erases the gain (e.g. riir-rag
/// dropping `with_bm25_tokenizer`) reds the gate instead of silently
/// regressing precision.
#[must_use]
pub fn run_retrieval_sweep() -> Vec<RetrievalSweepRow> {
    let cap_iter = [
        crate::draft::IndexTokenizer::CodeDefault,
        crate::draft::IndexTokenizer::OpAware,
        crate::draft::IndexTokenizer::OpAwareBigram,
    ];
    let mut rows = Vec::with_capacity(cap_iter.len());
    for tokenizer in cap_iter {
        for latent_weight in [1.0_f32, 0.0] {
            let config = crate::draft::RetrievalConfig {
                tokenizer,
                latent_weight,
                ..crate::draft::RetrievalConfig::default()
            };
            let r = run_eval_with(config);
            rows.push(RetrievalSweepRow {
                config,
                per_domain: r.per_domain,
                blended: r.blended,
            });
        }
    }
    rows
}

/// Human-readable T3.5 sweep table.
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_retrieval_sweep(rows: &[RetrievalSweepRow]) {
    println!("== Plan 005 T3.5 retrieval sweep (option 2: tiny-token crowding) ==");
    println!("  G1 bar: clippy top-1 >= 80%");
    println!(
        "{:<13} {:>7} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "tokenizer", "latent", "clippy@1", "clippy@3", "perf@1", "blend@1", "blend@3"
    );
    let pct = |v: Option<f32>| v.map_or("-".to_string(), |x| format!("{:.1}%", x * 100.0));
    for row in rows {
        let tag = match row.config.tokenizer {
            crate::draft::IndexTokenizer::CodeDefault => "code-default",
            crate::draft::IndexTokenizer::OpAware => "op-aware",
            crate::draft::IndexTokenizer::OpAwareBigram => "op-bigram",
        };
        println!(
            "{tag:<13} {:>7.2} {:>9} {:>9} {:>9} {:>9} {:>9}",
            row.config.latent_weight,
            pct(row.per_domain[0].recall(1)),
            pct(row.per_domain[0].recall(3)),
            pct(row.per_domain[1].recall(1)),
            pct(row.blended.recall(1)),
            pct(row.blended.recall(3)),
        );
    }
}

/// One row of the Plan 005 T3.5.4 re-rank sweep.
#[derive(Debug, Clone, Copy)]
pub struct RerankSweepRow {
    /// The mode that produced the row.
    pub mode: crate::draft::RerankMode,
    /// Per-domain metrics, indexed as [`EvalReport::per_domain`].
    pub per_domain: [DomainMetrics; DomainId::ALL.len()],
    /// Blended metrics across every labeled span.
    pub blended: DomainMetrics,
}

/// Run the eval under each re-rank mode — the T3.5.4 benchmark.
#[must_use]
pub fn run_rerank_sweep() -> Vec<RerankSweepRow> {
    [
        crate::draft::RerankMode::Off,
        crate::draft::RerankMode::Structural,
        crate::draft::RerankMode::All,
    ]
    .into_iter()
    .map(|mode| {
        let r = run_eval_reranked(mode);
        RerankSweepRow {
            mode,
            per_domain: r.per_domain,
            blended: r.blended,
        }
    })
    .collect()
}

/// Human-readable T3.5.4 sweep table.
#[allow(clippy::cast_precision_loss)] // display percentages over tiny counts
pub fn print_rerank_sweep(rows: &[RerankSweepRow]) {
    println!("== Plan 005 T3.5.4 instantiation re-rank sweep ==");
    println!("  G1 bar: clippy top-1 >= 80%");
    println!(
        "{:<12} {:>9} {:>9} {:>9} {:>9} {:>9}",
        "mode", "clippy@1", "clippy@3", "perf@1", "blend@1", "blend@3"
    );
    let pct = |v: Option<f32>| v.map_or("-".to_string(), |x| format!("{:.1}%", x * 100.0));
    for row in rows {
        let tag = match row.mode {
            crate::draft::RerankMode::Off => "off",
            crate::draft::RerankMode::Structural => "structural",
            crate::draft::RerankMode::All => "all",
        };
        println!(
            "{tag:<12} {:>9} {:>9} {:>9} {:>9} {:>9}",
            pct(row.per_domain[0].recall(1)),
            pct(row.per_domain[0].recall(3)),
            pct(row.per_domain[1].recall(1)),
            pct(row.blended.recall(1)),
            pct(row.blended.recall(3)),
        );
    }
}

// Issue 090: these tests need clippy_lints AND rust_perf — the fixtures
// are clippy+perf labeled spans, the regression-gate floors are calibrated
// on the clippy+perf blend (blended floors sit ABOVE clippy-only precision),
// and per_domain rows are positional (0=clippy, 1=perf). A latent-only
// build (the matcher refuses a domain-less build) correctly drops them.
#[cfg(all(test, feature = "latent_retrieval", feature = "clippy_lints", feature = "rust_perf"))]
mod tests {
    use super::*;

    /// Issue 147 T0′ — the Decision-CPI escalation-headroom baseline (the
    /// tier's cheap dual kill, headroom half). PRE-REGISTERED before this
    /// test ever ran (the issue file's T0′ block, 2026-10-05):
    ///
    /// - primary `θ_M` = 0.05 on `span_margin(..).min` (sensitivity 0.02 /
    ///   0.10 / 0.20 reported, never deciding);
    /// - structural top-1 = the eval's own blended accounting (own-domain
    ///   `canonical_rank` first id == label);
    /// - headroom bar: `|subset| ≥ 8` AND `subset_top1 ≤ overall − 0.05`;
    /// - clean baseline: NO hotpath feed, no oracles — the matcher alone.
    ///
    /// `#[ignore]`d because it is a MEASUREMENT (prints the record; asserts
    /// nothing about the numbers — the go/no-go is a recorded decision in
    /// the issue, not a gate). Rerun at any SHA with:
    /// `cargo test --features latent_retrieval --lib decision_cpi_t0 -- --ignored --nocapture`
    #[test]
    #[ignore = "T0′ measurement (Issue 147): prints the escalation-headroom record; not a gate"]
    fn decision_cpi_t0_headroom_baseline() {
        let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
        let spans = labeled_eval_spans();
        let mut n = 0usize;
        let mut top1 = 0usize;
        let mut uncontested = 0usize;
        // (min margin, top-1 correct) — finite margins only.
        let mut rows: Vec<(f32, bool)> = Vec::new();
        let mut subset_domains: Vec<(DomainId, usize)> =
            DomainId::ALL.iter().map(|d| (*d, 0usize)).collect();
        for span in &spans {
            if !matcher.has_domain(span.domain) {
                continue;
            }
            let sm = crate::draft::margin_weight::span_margin(&mut matcher, &span.code);
            let (_, by_domain) = matcher.suggest_span(&span.code, PROBE_TOP_K);
            let ranked = by_domain
                .iter()
                .find(|(d, _)| *d == span.domain)
                .map_or(Vec::new(), |(_, rules)| canonical_rank(rules));
            let correct = ranked.first().is_some_and(|r| r.rule_id == span.label);
            n += 1;
            top1 += usize::from(correct);
            if sm.min.is_finite() {
                rows.push((sm.min, correct));
                if sm.min < 0.05 {
                    let di = DomainId::ALL
                        .iter()
                        .position(|d| *d == span.domain)
                        .expect("labeled domain in ALL");
                    subset_domains[di].1 += 1;
                }
            } else {
                uncontested += 1;
            }
        }
        assert!(n > 0, "no labeled spans evaluated — features wrong?");
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        println!("== Issue 147 T0′ headroom baseline ==");
        println!(
            "spans: {n} (uncontested/INFINITY-margin: {uncontested}, contested: {})",
            rows.len()
        );
        // Count→ratio casts over fixture populations (≤ ~150 here); allowed
        // with cause — the same posture as margin_weight's concentration stat.
        #[allow(clippy::cast_precision_loss)]
        let overall_t1 = top1 as f32 / n as f32;
        println!("overall structural top-1: {top1}/{n} = {overall_t1:.3}");
        print_margin_quantiles(&rows);
        // Sensitivity table — θ ascending; the PRIMARY decision row is 0.05.
        for (theta, primary) in [(0.02f32, false), (0.05, true), (0.10, false), (0.20, false)] {
            print_threshold_row(theta, primary, &rows, n, overall_t1);
        }
        let per_dom: Vec<String> = subset_domains
            .iter()
            .filter(|(_, c)| *c > 0)
            .map(|(d, c)| format!("{d:?}={c}"))
            .collect();
        println!("primary-subset per-domain membership: {}", per_dom.join(", "));
        println!("pre-registered bars: |subset| >= 8 AND subset_top1 <= overall - 0.05 for headroom");
    }

    /// The T0′ record's margin-spectrum row (finite margins, ascending).
    fn print_margin_quantiles(rows: &[(f32, bool)]) {
        // Fixture populations (≤ ~150) are exact in f32; the index cast is
        // clamped below — allowed with cause.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
        let q = |p: f32| -> f32 {
            if rows.is_empty() {
                return f32::NAN;
            }
            let idx = (((rows.len() - 1) as f32 * p).round() as usize).min(rows.len() - 1);
            rows[idx].0
        };
        println!(
            "contested-margin quantiles: p10={:.4} p25={:.4} p50={:.4} p75={:.4} p90={:.4}",
            q(0.10),
            q(0.25),
            q(0.50),
            q(0.75),
            q(0.90)
        );
    }

    /// One sensitivity-table row: subset/complement size + top-1 + the gap
    /// vs overall, at one margin threshold.
    fn print_threshold_row(theta: f32, primary: bool, rows: &[(f32, bool)], n: usize, overall_t1: f32) {
        let sub: Vec<bool> = rows.iter().filter(|(m, _)| *m < theta).map(|(_, c)| *c).collect();
        let comp: Vec<bool> = rows
            .iter()
            .filter(|(m, _)| *m >= theta)
            .map(|(_, c)| *c)
            .collect();
        let tag = if primary { " [PRIMARY]" } else { "" };
        if sub.is_empty() {
            println!("theta={theta:.2}{tag}: subset EMPTY (0 spans) — kill criterion 'empty subset'");
            return;
        }
        let s_hits = sub.iter().filter(|c| **c).count();
        let c_hits = comp.iter().filter(|c| **c).count();
        // Count→ratio casts over fixture populations (≤ ~150 here); allowed
        // with cause.
        #[allow(clippy::cast_precision_loss)]
        let (s_t1, c_t1) = (
            s_hits as f32 / sub.len() as f32,
            c_hits as f32 / comp.len() as f32,
        );
        println!(
            "theta={theta:.2}{tag}: subset {}/{} spans, subset top-1 {s_t1:.3} ({s_hits}), complement top-1 {c_t1:.3} ({c_hits}), gap {:+.3}",
            sub.len(),
            n,
            s_t1 - overall_t1
        );
    }

    /// Baseline floors for the regression gate (Bench 022, 2026-08-16;
    /// RE-PINNED by Issue 022 same day — see Bench 024). A corpus batch that
    /// decays precision below these fails the suite — re-measure + re-pin
    /// (and record the delta) rather than weakening silently. Values are
    /// ratios (0.0-1.0).
    ///
    /// Original baseline (clippy+perf state, 88 spans): blended 71.6/84.1/92.0,
    /// clippy 58.3/75.0/87.5, perf 87.5/95.0/97.5; kernel (combined state,
    /// 30 self-variants) 100/100/100.
    ///
    /// Issue 022 re-pin (corpus truthfulness trade): 5 clippy corpus entries'
    /// buggy shapes were corrected to forms REAL clippy 1.97 fires (the
    /// `comparison_to_empty` entry's text moved from `.len() == 0` — real
    /// clippy's `len_zero` — to the real `s == ""` shape). Cost: the 2
    /// len-shaped fixtures retrieve `comparison_to_empty` at lower rank
    /// (rank 3 + rank 7). New baseline: blended 69.3/81.8/88.6,
    /// clippy 54.2/70.8/81.2, perf unchanged. Truthfulness wins — a corpus
    /// that misrepresents lint behavior poisons `self_evolve` learning.
    ///
    /// Batch 48 re-pin (8 `rust_perf` rules + spans, 2026-08-16): perf
    /// corpus 50→58 rules, `PERF_SPANS` 40→48, blended denominator 88→96. First
    /// measurement showed 3 of the 8 new spans drifting (filter-map-fuse not
    /// even top-8 — every distinctive token renamed away from the corpus
    /// shape); the spans were re-authored to the set's convention (variant
    /// names, canonical predicate/operator shapes) per the Issue 022
    /// fix-before-repin order. New baseline: blended 70.8/83.3/90.6,
    /// clippy 54.2/70.8/83.3, perf 87.5/95.8/97.9 (top-1 UNCHANGED vs the
    /// 40-span set — zero batch decay), kernel self-variants 100/100/100.
    /// 3-domain (126 spans): blended 77.8/87.3/92.9, router 88/126 = 69.8%.
    ///
    /// Floors = baseline − ONE span — the ranking's tie-break drains a
    /// `HashMap` (`RandomState` per process), so a tie at a hit boundary can
    /// flip across processes (3-process re-runs were identical; the margin
    /// is cheap robustness, not noise cover).
    mod floors {
        // Rebased 2026-08-16 by Plan 005 T3.5 (operator-aware BM25 tokenizer +
        // adjacency bigrams), again by T3.5.5 (instantiation re-rank promoted
        // to default; measured blended 89/92/92 of 96, clippy 42/45/45 of 48,
        // perf 47/47/47 of 48), again by Issue 024 (Bench 032: RERANK_POOL
        // 8→12 after the cross-domain crowding casualty at raw rank 9;
        // measured blended 90/93/93 of 96, clippy 43/46/46 of 48, perf
        // 47/47/47 of 48), again by Batch 53 (4 `rust_perf` rules + spans,
        // 2026-08-17: perf corpus 58→62 rules, `PERF_SPANS` 48→52, blended
        // denominator 96→100; measured blended 93.0/97.0/97.0 of 100, clippy
        // 43/46/46 of 48, perf 50/51/51 of 52, router 70/100 — all 4 new
        // spans retrieve their rule top-1), and again by Batch 59 (1
        // `rust_perf` rule + span, 2026-08-18, the first external-source
        // distill via the `distill` skill: perf corpus 62→63 rules,
        // `PERF_SPANS` 52→53, blended denominator 100→101; measured blended
        // 94/98/98 of 101, perf 51/52/52 of 53, router 71/101 — the new span
        // retrieves its rule top-1). Floors are measured − ONE span,
        // the documented convention.
        //
        // NOTE these clippy rows are INFLATED by the fixture's single-lint /
        // zero-cross-fire shape (T3.5.4's finding) — they are a fine
        // REGRESSION floor but not an effect size. The re-rank promotion was
        // justified by the uncontaminated composite measurement instead
        // (`tests/uncontaminated_g1.rs`, +12.0pp).
        //
        // P7 re-pin (manual_is_multiple_of corpus entry, 2026-08-29): blended
        // top-3 measured 96/101 (was 97) — the new entry's `== 0` token
        // legitimately out-scores the truthful `s == ""` comparison_to_empty
        // entry on the len-shaped fixture (`queue_ready`: `pending.len() ==
        // 0`), pushing it top-3 → rank 4. Same trade Issue 022 recorded (the
        // 2 len-shaped fixtures already sat at rank 3 + 7 after the corpus
        // text went truthful). Structural re-rank cannot rescue it: VERIFY-tier
        // proposers are deliberately excluded from instantiation evidence
        // (Issue 034 T1), so both competitors rank on raw tokens and the
        // modulo entry's `== 0` wins. Fix-before-repin was applied — the
        // `== 0` token IS the lint's identity, irreducible. Production --fix
        // is lint-keyed (unaffected); --suggest on a len-shaped span now
        // suggests a same-family rule that does not fire there. Floor =
        // measured 96 − the one-span margin. top-1/top-5 + all per-domain
        // floors re-measured UNCHANGED (93/97/101, clippy 43/45/48, perf
        // 52/51/53) — only the top-3 blended const moves.
        //
        // RE-MEASURED 2026-09-02 (Issue 063 sweep; the figures above had gone
        // stale by 1-2 spans as the corpus moved — floors were NOT weakened,
        // the measurement rose): blended **94/97/99** of 101 (so TOP3 and TOP5
        // now carry TWO spans of margin, not one), clippy **43/45/46** of 48,
        // perf **51/52/53** of 53, kernel **30/30/30** of 30, router 74/101
        // at `latent_retrieval` and 83/131 with `sec,kernel_opt`. Confirmed
        // in both feature states and across a two-arm A/B in one worktree.
        //
        // Issue 134 T3 re-arm re-pin (2026-09-24) — the thin-domain fixture
        // sets (docker 31 + dist 6 spans, both DEFAULT features) grew the
        // default-state population 101 → 138. Four under-authored docker
        // spans were re-authored to the corpus shape FIRST (fix-before-repin,
        // the Issue 022 order — the four carried non-canonical base-image
        // tokens where the corpus entries pin `FROM scratch` / `FROM rust…`;
        // docker measured 74.2% → 83.9% after). Measured at the WEAKEST state
        // (default: clippy+perf+docker+dist — every other state adds domains
        // that only raise the blended): top-1 **124/138 (89.9%)**, top-3
        // **130/138 (94.2%)**, top-5 **132/138 (95.7%)**; the other states:
        // +shader 132/138/140 of 146, +sec 140/146/148 of 154, +kernel
        // 154/160/162 of 168, ALL 178/184/186 of 192. Floors = measured − ONE
        // span at the weakest state. Per-domain rows UNCHANGED (clippy
        // 43/44/46 of 48, perf 51/52/53 of 53 — the new domains' rows are
        // their own measurement, printed by the demonstrability column:
        // docker constant 0.867, dist 0.800 — honest near-duplicate-corpus
        // difficulty: cache-mount-missing-on-cargo-build and
        // manifest-first-layer-split carry byte-identical corpus buggy texts,
        // and dist's three ship-size rules share the bare-`[[bin]]` shape).
        // New per-domain floors for the thin domains are NOT pinned here yet —
        // one fixture-set generation of stability first (the Batch-48
        // precedent: first measurements move as the set converges).
        //
        // Batch 179 re-pin (2026-09-25, the rsconstmap trio landing): the
        // three new rust_perf spans (paired-probe-layout / block-hash-mlp /
        // immutable-static-map) grew the default-state population 138 → 141
        // (perf 53 → 56). All three new spans retrieve their rule top-1
        // (their authors follow the corpus-shape convention). MEASURED at
        // the landing (default state: clippy+perf+docker+dist): blended
        // **127/141 (90.1%) / 133/141 (94.3%) / 135/141 (95.7%)**, clippy
        // **43/48 / 44/48 / 44/48**, perf **54/56 / 55/56 / 56/56**. Floors
        // = measured − ONE span. Clippy top-1 UNCHANGED (43/48 measured,
        // 42/48 floor — the one-span margin intact); clippy top-3 re-pinned
        // 44/48 → 43/48 (measured held at 44; the pin moves only to keep
        // the one-span convention as the denominator convention requires).
        // Batch 243 re-pin (2026-10-16, the text/unicode family seed landing,
        // Plan 245 / Research 275): +2 benchable rust_perf rules (the corpus's
        // founding text/unicode family) grew the default-state population's
        // competition pool. MEASURED at the landing (default state):
        // blended **125/141 (88.7%) / 131/141 (92.9%) / 133/141 (94.3%)**,
        // clippy **42/48 / 43/48 / 43/48**, perf **53/56 / 54/56 / 55/56** —
        // every per-domain floor HELD (clippy top-1 and top-3 at their
        // zero-margin pins, PERF_TOP3 at its zero-margin pin). The blended
        // top-1/top-3 misses are ONE span each of cross-pool competition:
        // the rho report shows `repr-u8-enum` outranked (rank 4) by the
        // text-rich serving rule `empty-slot-first-victim-order-save-before-overwrite`
        // (Batch 241's own landing, surfacing now at this state) and
        // `dyn-to-enum` at rank 5 (the B200 slip, one further rung) — the
        // P7/B200 trade again: legitimate corpus growth re-ranks the pool,
        // no domain regressed. Floors = measured − ONE span. TOP5 held
        // (133 vs floor 132) — untouched.
        // Batch 244 NO-RE-PIN (2026-10-16, the byte-scan/search batch landing,
        // Plan 247 / Research 276): +10 benchable rust_perf rules (the
        // text/unicode family's follow-up -- regex-automata + memchr + bstr at
        // release pins) grew the pool again. MEASURED at the landing (default
        // state): blended **125/141 (88.7%) / 131/141 (92.9%) / 133/141
        // (94.3%)** -- every blended floor HELD (125 >= 124, 131 >= 130) and
        // the per-domain floors HELD (clippy/rust_perf/docker/dist
        // demonstrability constants 0.955/0.946/0.867/0.800, rust_perf n=56
        // selected 53). No re-pin: the floors stand as-is (124/130), the
        // batch growth was absorbed without a span of decay.
        pub const BLENDED_TOP1: f32 = 124.0 / 141.0;
        pub const BLENDED_TOP3: f32 = 130.0 / 141.0;
        // Batch 200 re-pin (2026-10-03, the TensorFold serving-half landing,
        // Plan 198): 11 NotBenchable rust_perf serving rules grew the corpus
        // 168 → 179. ATTRIBUTION MEASURED pre/post in the same session (the
        // pre-batch run in a detached worktree at the landing HEAD, same
        // binary config): blended top-1 126/141 and top-3 132/141 UNCHANGED,
        // clippy 42/43/43 + perf 54/55 + docker unchanged — the entire delta
        // is ONE span, the `dyn-to-enum` perf fixture, whose expected rule
        // slipped rank 4 → 5 (one new serving fixture outranks it inside the
        // pool), moving blended top-5 134/141 → 133/141. Floor = measured −
        // ONE span = 132. The rust_perf DOMAIN metrics are unchanged (top-1
        // 54/56, top-3 55/56; the demonstrability rust_perf column constant
        // 0.964, selected 54 / rejected 2 — identical to the standing
        // record), so this is cross-pool competition from legitimate corpus
        // growth (the P7 trade), not a domain retrieval regression. TOP1 and
        // TOP3 stand at their PRE-EXISTING zero-margin state (measured ==
        // floor before this batch too) — untouched here; the next batch that
        // moves them owes the Issue-063 one-span-margin repair.
        pub const BLENDED_TOP5: f32 = 132.0 / 141.0;
        pub const CLIPPY_TOP1: f32 = 42.0 / 48.0;
        // Issue 063 sweep, 2026-09-02 — **convention alignment, NOT a
        // loosened bar.** This const was `45.0 / 48.0` while clippy top-3
        // measures exactly 45/48, i.e. margin ZERO — the only floor in this
        // module that did not follow the "measured − ONE span" convention
        // stated at the top of `mod floors`. Every sibling floor has its span:
        // CLIPPY_TOP1 42 vs measured 43, PERF_TOP3 51 vs 52, KERNEL_TOP3 29 vs
        // 30, BLENDED_TOP1 93 vs 94. At zero margin a single clippy top-3
        // casualty reds the whole 14/14 gate, which made this the most
        // regression-fragile assertion in the repo — and it would red on a
        // legitimate corpus addition, exactly the trade the P7 note above
        // documents for the blended row.
        //
        // The gate was NOT failing: 45 ≥ 45 passed. This restores the one-span
        // margin the convention prescribes (44/48) rather than nudging a
        // constant until something passes.
        pub const CLIPPY_TOP3: f32 = 43.0 / 48.0;
        pub const PERF_TOP3: f32 = 54.0 / 56.0;
        #[cfg(feature = "kernel_opt")]
        pub const KERNEL_TOP3: f32 = 29.0 / 30.0;
        /// Router-annotation accuracy floor (Issue 020 wiring, Bench 025):
        /// the TF-cosine-global annotation measured 71/101 (70.3%, 2-domain,
        /// Batch 59) after the +1-span growth. Floor = measured − 5 spans of
        /// corpus-growth headroom. This is the standing per-batch measurement
        /// of annotation quality — NOT a gate-quality claim (gating stays
        /// killed at the 85% bar).
        /// Router-annotation accuracy floors, one per enabled-domain state
        /// (Issue 020 wiring, Bench 025). This is the standing per-batch
        /// measurement of annotation quality — NOT a gate-quality claim
        /// (gating stays killed at the 85% bar).
        ///
        /// **Restructured with the docker landing (Plan 030 T0.4).** These
        /// were four `#[cfg]`-gated consts keyed on `(kernel_opt, sec)`. A
        /// 5th domain would have made it EIGHT consts whose declaration cfg
        /// and use cfg must match exactly — the precise shape that turned CI
        /// Layer 5a red on the sec landing (`ROUTER_ANNOTATION_3D` declared
        /// under `kernel_opt` but used under `not(sec)` → `dead_code` under
        /// `-D warnings`). Selecting at RUNTIME with `cfg!` compiles every
        /// arm in every state, so that whole failure class is gone; the
        /// values stay per-state and measured.
        ///
        /// Floor = measured − 5 spans of corpus-growth headroom, per state.
        /// Re-measure + re-pin with the delta recorded; never weaken
        /// silently.
        #[must_use]
        pub fn router_annotation_floor() -> f32 {
            // Plan 134 shader axis: MEASURED 2026-09-16 (the landing run,
            // `default + latent_retrieval + shader_perf`): **67/101 (66.3%)
            // vs the 73/101 shader-off twin — a −6 span DECAY**, floor =
            // 62/101. The mechanism is the B131 SEC mechanism, NOT docker's:
            // the WGSL corpus patterns are token-dense in generic control
            // flow (`if`/`else`/`return`/`let`/`var`), so the shader
            // centroid ATTRACTS clean Rust spans (the eval's miss taxonomy
            // shows `branch-divergent-sample-select` in nearly every clean
            // span's top-5). Recorded, not hidden — `shader_perf` is
            // opt-in and no standing lane carries it; the number improves
            // only when the corpus grows less token-generic patterns or the
            // fixture set gains shader-labeled spans. Other shader-on
            // combos (kernel+shader etc.) remain unmeasured and inherit
            // their twins — re-measure when a lane first carries them.
            // Issue 085 dist axis: the ONLY buildable gated dist-on state is
            // `default + dist` = (k=false, s=false, d=true, dist=true),
            // MEASURED 2026-09-09: 74/101 (73.3%) vs its dist-off twin's
            // 73/101 (72.3%) — +1 span, the SAME docker-precedent mechanism
            // (a TOML off-vocabulary centroid absorbs fewer generic Rust
            // spans). Floor = measured − 5 per the Plan-030 convention.
            //
            // POST-PROMOTION 2026-09-13: `dist` joined `default`, so every
            // default-bearing lane is now the dist-on state — the dist-off
            // twins are reachable only via --no-default-features + explicit
            // features (states that mostly no longer build, see the
            // no-docker arms above). Re-measured at the promotion: canonical
            // `default + latent_retrieval` = 74/101 (73.3%, the pinned arm,
            // floor 69 held); `kernel_opt + latent_retrieval` = 92/131
            // (70.2%) — IDENTICAL to its dist-off twin (delta 0; the floor
            // 87 it inherited was already the right pin). No re-pin owed.
            // (`dist` joined `default` 2026-09-13 — every docker-on arm below
            // is the dist-on state; the historical dist_on selector was
            // folded away at the Issue-134 re-pin.)
            match (
                cfg!(feature = "kernel_opt"),
                cfg!(feature = "sec"),
                cfg!(feature = "docker"),
            ) {
                // 2-domain (clippy+perf): measured 71/101 (70.3%, Batch 59).
                (false, false, false) => 66.0 / 101.0,
                // 3-domain (+kernel): measured 84+5/130.
                (true, false, false) => 84.0 / 130.0,
                // 4-domain (+sec): measured 45/101 (44.6%, Plan 018) — the
                // TF-cosine lexical router has a mechanically harder task
                // (sec's 6 patterns are long full-fn bodies whose
                // common-token centroid absorbs generic Rust spans, and the
                // fixture set carries NO sec-labeled spans, so every
                // sec-routing is a miss by construction).
                // NOT re-measured at Batch 131: the no-docker states no
                // longer BUILD (hard deps on default features — rustc_errors
                // is unconditional in draft code), so the Plan-018 pin stands
                // for this arm; the docker-on arms below are the states
                // anyone runs.
                (false, true, false) => 40.0 / 101.0,
                // Same — not measurable since the docker-landing tree grew
                // default-feature hard deps; Plan-018 pin stands.
                (true, true, false) => 73.0 / 131.0,
                // ── docker-enabled states (Plan 030 T0.4, MEASURED
                // 2026-08-28; floor = measured − 5 spans, per state) ──
                //
                // Docker's 27 patterns are DOCKERFILE text, not Rust, so
                // their vocabulary (FROM/RUN/apt-get/COPY) barely
                // intersects the Rust fixture spans. The measured result
                // is the opposite of sec's: adding docker never lowers the
                // annotation and twice RAISES it (2-domain 71/101 → 73/101;
                // 4D+K 78/131 → 82/131), because a distinctive
                // off-vocabulary centroid absorbs fewer generic Rust spans
                // than a Rust-shaped one does. That is the measured answer
                // to Proposal 006's open question about a 5th domain's cost
                // to the router: for THIS domain it is not a cost.
                (false, false, true) => {
                    // Issue 134 T3 re-arm (MEASURED 2026-09-24, the
                    // thin-domain fixture landing): population 101 → 138
                    // (docker 31 + dist 6 spans). Shader-off default state:
                    // 110/138 (79.7%) — floor = 105. The shader-on twin
                    // (146 spans): 114/146 (78.1%) — floor = 109. (The
                    // historical dist-off twin arm is folded in: `dist` is a
                    // default feature, so a docker-on dist-off state is
                    // reachable only via --no-default-features feature
                    // surgery that nothing runs.)
                    // Batch 179 re-pin (2026-09-25, the rsconstmap trio:
                    // 3 rust_perf spans in every state's population):
                    // shader-off 141 spans, MEASURED 113/141 (80.1%) — floor
                    // = 108. Shader-on twin (149 spans): 116/149 (77.9%) —
                    // floor = 111.
                    if cfg!(feature = "shader_perf") {
                        111.0 / 149.0 // measured 116/149 (77.9%)
                    } else {
                        // Batch 230 re-pin (2026-10-13, the clippy-1.99
                        // new-lint wave, Plan 232): the 5 new clippy corpus
                        // entries grew the retrieval pool — cross-pool
                        // competition (the Batch-200 P7 mechanism: new pool
                        // members outrank expected rules on TF-cosine) —
                        // MEASURED 107/141 (75.9%, was 113/141 = 80.1% at
                        // B179; six spans flipped routing). Floor =
                        // measured − 5 per the Plan-030 convention → 102.
                        102.0 / 141.0 // measured 107/141 (75.9%)
                    }
                }
                // Batch 130 (2026-09-08, Plan 111): the 5 kernel CloudRift
                // adds re-measured 93/131 (71.0%, was 88/131 = 67.2% at
                // B129) -- the rich TMA/dispatch concept text sharpened the
                // TF-cosine centroids upward; floor = measured - 5 per the
                // Plan-030 convention.
                // Batch 137 (2026-09-09, Plan 116): the 11 CloudRift matmul
                // adds re-measured 91/131 (69.5%, was 93/131 = 71.0% at
                // B130/B136) -- the dense split-K/ldmatrix/swizzle concept
                // vocabulary attracts 2 more generic Rust spans to the
                // kernel centroids (the B131 sec-centroid mechanism, kernel
                // flavor); floor = measured - 5 per the Plan-030 convention.
                // Batch 142 (2026-09-10, Plan 122): the 2 CloudRift Gemma4
                // adds re-measured 92/131 (70.2%, was 91/131 = 69.5% at
                // B137) -- +1 span back; the accumulation/staging concept
                // vocabulary is closer to the existing kernel centroid
                // mass than B137's was; floor = measured - 5 per the
                // Plan-030 convention.
                (true, false, true) => {
                    // Issue 134 T3 re-arm (MEASURED 2026-09-24): population
                    // 131 → 168 (the thin-domain fixtures at the kernel state).
                    // 105/168 (62.5%) — floor = 100 per the Plan-030 convention.
                    // Batch 179 re-pin (2026-09-25): population 171, MEASURED
                    // 107/171 (62.6%) — floor = 102.
                    // Batch 223 re-pin (2026-10-11): MEASURED 101/171 (59.1%,
                    // isolated E:/tmp/b223tgt) — was 107 at B179; the interim
                    // batches (B180–B222) measured ≥ 102 (the gate held at
                    // the 102 pin without a re-measure) and the lithos-metal
                    // 11 tipped it one span. Attribution MEASURED both sides:
                    // the pre-batch detached worktree (the 4090's .w223pre @
                    // e7f2a9ee, isolated tmp) measured EXACTLY
                    // 102/171 (59.6%) — the batch's 11 kernel rules cost ONE
                    // span, the B137 mechanism (new kernel concept vocabulary
                    // — ICB replay, coherent pointers, megakernel barrier
                    // placement — attracts one more generic-Rust span to a
                    // kernel centroid). Floor = measured − 5 = 96 per the
                    // Plan-030 convention.
                    96.0 / 171.0
                }
                // Batch 131 (2026-09-08, Plan 112): the two sec
                // InputValidation adds (wire-frame alloc + secret-file
                // create) re-measured 35/101 (34.7%, was 41/101 = 40.6% at
                // Plan 018) -- the sec centroid's new wire/fs vocabulary
                // (from_be_bytes, vec![0u8; len], File::create) ATTRACTS
                // generic Rust spans; floor = measured - 5 per the Plan-030
                // convention. First measured DECAY on a sec arm; the delta
                // is recorded, not weakened silently (Issue 020 contract).
                // Batch 140 (2026-09-09, Plan 120): the advisory-db
                // unbounded-read add re-measured 34/101 (33.7%, was 35/101
                // = 34.7% at B131) -- the read_to_end/TcpStream vocabulary
                // keeps the B131 sec-centroid drift ticking (-1 span);
                // floor = 34 - 5 = 29.
                (false, true, true) => 85.0 / 157.0, // measured 90/157 (57.3%) — Batch 179 re-pin, 2026-09-25 (population 141 → 157; was 82/154, measured 87/154 at Issue 134)
                // Batch 131: measured 79/131 (60.3%, was 82/131 = 62.6% at
                // Batch 130) -- same sec-centroid drift on the full state;
                // floor = measured - 5.
                // Batch 140 (2026-09-09, Plan 120): re-measured 83/131
                // (63.4%, was 79/131 = 60.3% at B131) -- +4 UP; the same
                // wire vocabulary that decays the no-kernel arm sharpened
                // the full-state boundary here (cross-arm divergence is
                // consistent with the B130 kernel-vocabulary raise); floor
                // = 83 - 5 = 78.
                (true, true, true) => {
                    // Issue 134 T3 re-arm (MEASURED 2026-09-24, the full
                    // seven-domain state): population 131 → 192 — every
                    // thin domain adds spans whose Rust-shaped text routes
                    // poorly (the compound B131 sec-centroid mechanism; the
                    // shader corpus's generic control-flow vocabulary
                    // compounds it). 86/192 (44.8%) — floor = 81 per the
                    // Plan-030 convention. Annotation only: gating stays
                    // killed (85% bar, Bench 025).
                    // Batch 179 re-pin (2026-09-25): population 195, MEASURED
                    // 89/195 (45.6%) — floor = 84 (the 3 new rust_perf spans
                    // routed correctly AND the shader spans hold 100%, so the
                    // ratio ROSE 44.8% → 45.6% despite the sec/kernel
                    // attractor mass).
                    84.0 / 195.0 // measured 89/195 (45.6%)
                }
            }
        }
    }

    #[allow(clippy::cast_precision_loss)] // ratio arithmetic over tiny counts
    fn ratio(hits: usize, n: usize) -> f32 {
        if n == 0 {
            1.0
        } else {
            hits as f32 / n as f32
        }
    }

    #[test]
    fn rerank_circularity_diagnostic() {
        // The T3.5.4 gain must be checked for ORACLE CONTAMINATION before it
        // can be believed. `clippy_fixture_labels_verify_via_proposers` is a
        // committed, passing test asserting that for every clippy fixture the
        // OWN proposer fires and NO other proposer cross-fires. That is
        // exactly the property the instantiation re-ranker exploits, so on
        // this fixture set `RerankMode::All` degenerates to "was the correct
        // rule retrieved into the pool at all?".
        //
        // The falsifiable prediction: All-mode top-1 == pool recall @
        // RERANK_POOL, exactly. If it holds, the measured gain is not an
        // effect size and must not be reported as one.
        use crate::draft::{RerankMode, RERANK_POOL};
        let mut matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher")
            .with_rerank_mode(RerankMode::Off);

        let mut in_pool = 0usize;
        let mut labeled = 0usize;
        for span in super::labeled_eval_spans() {
            if span.domain != crate::draft::DomainId::Clippy || !matcher.has_domain(span.domain) {
                continue;
            }
            labeled += 1;
            let (_, by_domain) = matcher.suggest_span(&span.code, RERANK_POOL);
            let found = by_domain
                .iter()
                .find(|(d, _)| *d == span.domain)
                .is_some_and(|(_, rules)| rules.iter().any(|r| r.rule_id == span.label));
            if found {
                in_pool += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        let pool_recall = in_pool as f32 / labeled as f32;

        let all = super::run_eval_reranked(RerankMode::All);
        let all_top1 = all.per_domain[0].recall(1).expect("clippy labeled");

        println!("\n== T3.5.4 circularity diagnostic (clippy) ==");
        println!(
            "  pool recall @{RERANK_POOL:<2}      : {:.1}% ({in_pool}/{labeled})",
            pool_recall * 100.0
        );
        println!("  RerankMode::All top-1 : {:.1}%", all_top1 * 100.0);
        println!(
            "  verdict: {}",
            if (all_top1 - pool_recall).abs() < 1e-6 {
                "CONTAMINATED — All-mode top-1 IS pool recall; the fixture's \
                 one-proposer-fires invariant decides the ranking, not the re-ranker"
            } else {
                "not fully explained by pool recall"
            }
        );
        assert!(
            (all_top1 - pool_recall).abs() < 1e-6,
            "the contamination prediction failed (all {all_top1:.4} vs pool {pool_recall:.4}) \
             — re-examine the oracle before trusting either number"
        );
    }

    #[test]
    fn rerank_sweep_report() {
        // Plan 005 T3.5.4 benchmark: does proposer-visitor evidence beat pure
        // text similarity? Prints the A/B table and pins the shipped gain.
        let rows = super::run_rerank_sweep();
        super::print_rerank_sweep(&rows);

        let get =
            |m: crate::draft::RerankMode| *rows.iter().find(|r| r.mode == m).expect("row present");
        let off = get(crate::draft::RerankMode::Off);
        let structural = get(crate::draft::RerankMode::Structural);

        let off_clippy = off.per_domain[0].recall(1).expect("clippy labeled");
        let on_clippy = structural.per_domain[0].recall(1).expect("clippy labeled");
        let off_perf = off.per_domain[1].recall(1).expect("perf labeled");
        let on_perf = structural.per_domain[1].recall(1).expect("perf labeled");

        // MECHANICAL check only. These numbers are CONTAMINATED — the clippy
        // fixtures are label-verified by the very proposers doing the ranking
        // (`clippy_fixture_labels_verify_via_proposers` asserts own-fires +
        // zero-cross-fires), and `rerank_circularity_diagnostic` shows
        // All-mode top-1 IS pool recall, exactly. So this asserts only that
        // the wiring is live and does no harm — NOT an effect size.
        assert!(
            on_clippy > off_clippy,
            "the re-rank wiring appears dead: clippy top-1 unchanged \
             (on {on_clippy:.3} vs off {off_clippy:.3})"
        );
        assert!(
            on_perf >= off_perf,
            "perf top-1 must not regress from re-ranking \
             (on {on_perf:.3} vs off {off_perf:.3})"
        );
        // Promoted 2026-08-16 by T3.5.5 on UNCONTAMINATED evidence (+12.0pp
        // top-1 over real-clippy-labeled multi-lint spans — see
        // `tests/uncontaminated_g1.rs`). This assertion pins the promotion so
        // a silent revert to Off is caught.
        assert_eq!(
            crate::draft::RerankMode::default(),
            crate::draft::RerankMode::Structural,
            "RerankMode::Structural is the T3.5.5-promoted default"
        );
        println!(
            "  NOTE: the clippy deltas in this table are INFLATED by the fixture's \
             single-lint shape (see rerank_circularity_diagnostic). The promotion \
             rests on tests/uncontaminated_g1.rs (+12.0pp), not on this table."
        );
    }

    #[test]
    fn retrieval_sweep_report() {
        // Plan 005 T3.5 option-2 benchmark. Prints the grid and pins the
        // load-bearing claims so a substrate change that erases the gain
        // (e.g. riir-rag reverting `with_bm25_tokenizer`) reds this test.
        const T35_G1_BAR: f32 = 0.80;
        let rows = super::run_retrieval_sweep();
        super::print_retrieval_sweep(&rows);

        let get = |t: crate::draft::IndexTokenizer, w: f32| {
            rows.iter()
                .find(|r| r.config.tokenizer == t && (r.config.latent_weight - w).abs() < 1e-6)
                .expect("row present")
                .clone()
        };
        // The pre-T3.5 shipped config vs the promoted one.
        let base = get(crate::draft::IndexTokenizer::CodeDefault, 1.0);
        // The PROMOTED config — the gate must test what actually ships.
        let promoted = crate::draft::RetrievalConfig::default();
        let op = get(promoted.tokenizer, promoted.latent_weight);

        let clippy_base = base.per_domain[0].recall(1).expect("clippy labeled");
        let clippy_op = op.per_domain[0].recall(1).expect("clippy labeled");
        assert!(
            clippy_op > clippy_base,
            "the promoted config must beat the substrate default on clippy top-1 \
             (promoted {clippy_op:.3} vs base {clippy_base:.3})"
        );
        // T3.5's G1 bar is 80%. The tokenizer work closed most but not all of
        // the gap (54.2% -> 70.8%), so this asserts the MEASURED gain as
        // regression protection and reports the residual gap rather than
        // asserting a target that is not yet met — a red permanent test
        // would just be a TODO that blocks the suite.
        if clippy_op < T35_G1_BAR {
            println!(
                "  NOTE: clippy top-1 {:.1}% is still {:.1}pp under the T3.5 G1 bar of {:.0}% \
                 — Phase 3 stays open (see Plan 005 T3.5 / Bench 027).",
                clippy_op * 100.0,
                (T35_G1_BAR - clippy_op) * 100.0,
                T35_G1_BAR * 100.0
            );
        }
        // Identifier-rich domains must not pay for the operator additions.
        let perf_base = base.per_domain[1].recall(1).expect("perf labeled");
        let perf_op = op.per_domain[1].recall(1).expect("perf labeled");
        assert!(
            perf_op >= perf_base,
            "perf top-1 must not regress (op {perf_op:.3} vs base {perf_base:.3})"
        );
    }

    #[test]
    fn retrieval_precision_regression_gate() {
        let report = run_eval();
        print_report(&report);
        let b = &report.blended;
        assert!(
            ratio(b.hits_at_1, b.labeled) >= floors::BLENDED_TOP1,
            "blended top-1 {} below floor {}",
            ratio(b.hits_at_1, b.labeled),
            floors::BLENDED_TOP1,
        );
        assert!(
            ratio(b.hits_at_3, b.labeled) >= floors::BLENDED_TOP3,
            "blended top-3 {} below floor {}",
            ratio(b.hits_at_3, b.labeled),
            floors::BLENDED_TOP3,
        );
        assert!(
            ratio(b.hits_at_5, b.labeled) >= floors::BLENDED_TOP5,
            "blended top-5 {} below floor {}",
            ratio(b.hits_at_5, b.labeled),
            floors::BLENDED_TOP5,
        );
        if report.per_domain[0].labeled > 0 {
            let m = &report.per_domain[0];
            assert!(ratio(m.hits_at_1, m.labeled) >= floors::CLIPPY_TOP1);
            assert!(ratio(m.hits_at_3, m.labeled) >= floors::CLIPPY_TOP3);
        }
        if report.per_domain[1].labeled > 0 {
            let m = &report.per_domain[1];
            assert!(ratio(m.hits_at_3, m.labeled) >= floors::PERF_TOP3);
        }
        #[cfg(feature = "kernel_opt")]
        if report.per_domain[2].labeled > 0 {
            let m = &report.per_domain[2];
            assert!(ratio(m.hits_at_3, m.labeled) >= floors::KERNEL_TOP3);
        }
        // Issue 020 wiring: the annotation router (TF-cosine-global) keeps a
        // standing accuracy floor — corpus batches shift centroids/vocab, so
        // decay below this re-measures + re-pins (record the delta), never
        // silently weakens. GATING remains killed (85% bar, Bench 025).
        if report.router_checked > 0 {
            #[allow(clippy::cast_precision_loss)] // ratio over tiny counts
            let acc = report.router_correct as f32 / report.router_checked as f32;
            let floor = floors::router_annotation_floor();
            assert!(
                acc >= floor,
                "router annotation accuracy {:.1}% below floor {:.1}% — annotation decayed (re-measure + re-pin per the Issue 020 contract)",
                acc * 100.0,
                floor * 100.0
            );
        }
    }

    #[test]
    fn eval_report_is_deterministic() {
        // G1: identical fixture set + corpus → identical metrics (the
        // modelless embedder + BM25 are deterministic; papaya iteration
        // order must not leak into rankings).
        let a = run_eval();
        let b = run_eval();
        assert_eq!(a.blended, b.blended);
        assert_eq!(a.per_domain, b.per_domain);
        assert_eq!(a.router_correct, b.router_correct);
    }

    /// Issue 134 T1: the per-domain and per-rule taxonomy partitions must
    /// sum EXACTLY to the aggregate taxonomy (a partition that loses a span
    /// would silently bias the demonstrability column's constants — the
    /// same span counted twice or not at all moves a ratio that T3 will
    /// correlate on).
    #[test]
    fn taxonomy_partitions_by_domain_and_rule() {
        let r = run_eval();
        let mut by_domain = crate::draft::retrieval_eval::ChoiceTaxonomy::default();
        for t in &r.taxonomy_by_domain {
            by_domain.labeled += t.labeled;
            by_domain.correct_rule += t.correct_rule;
            by_domain.correct_via_acceptable += t.correct_via_acceptable;
            by_domain.wrong_rule += t.wrong_rule;
            by_domain.wrong_span += t.wrong_span;
            by_domain.covered += t.covered;
            by_domain.declined += t.declined;
            by_domain.proposer_blind += t.proposer_blind;
            by_domain.selective_correct += t.selective_correct;
        }
        assert_eq!(by_domain, r.taxonomy, "per-domain partition ≠ aggregate");
        let mut by_rule = crate::draft::retrieval_eval::ChoiceTaxonomy::default();
        for (_, t) in r.taxonomy_by_rule.values() {
            by_rule.labeled += t.labeled;
            by_rule.correct_rule += t.correct_rule;
            by_rule.correct_via_acceptable += t.correct_via_acceptable;
            by_rule.wrong_rule += t.wrong_rule;
            by_rule.wrong_span += t.wrong_span;
            by_rule.covered += t.covered;
            by_rule.declined += t.declined;
            by_rule.proposer_blind += t.proposer_blind;
            by_rule.selective_correct += t.selective_correct;
        }
        assert_eq!(by_rule, r.taxonomy, "per-rule partition ≠ aggregate");
    }

    /// Label truth: every clippy fixture fires its labeled lint under OUR
    /// proposers (the standing oracle — itself cross-validated against real
    /// clippy 1.93 at authoring time) and fires NO other corpus lint.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn clippy_fixture_labels_verify_via_proposers() {
        use crate::domains::clippy_lints::proposers::{propose_fix, FIXABLE_LINTS, VERIFY_LINTS};
        let mut all: Vec<&str> = FIXABLE_LINTS.to_vec();
        all.extend_from_slice(VERIFY_LINTS);
        all.extend_from_slice(&[
            "useless_conversion",
            "needless_borrow",
            "unnecessary_mut_passed",
        ]);
        for &(lint, code) in super::super::eval_fixtures_clippy::CLIPPY_SPANS {
            assert!(
                propose_fix(lint, code).is_some(),
                "fixture for {lint} does not fire its own proposer"
            );
            for &other in &all {
                if other != lint {
                    assert!(
                        propose_fix(other, code).is_none(),
                        "fixture for {lint} cross-fires {other}"
                    );
                }
            }
        }
    }

    /// The five proposer-covered perf fixtures must fire their proposers.
    #[cfg(feature = "rust_perf")]
    #[test]
    fn perf_fixture_labels_verify_via_proposers() {
        use crate::domains::rust_perf::proposers::{propose_fix, FIXABLE_RULES};
        for &(rule, code) in super::super::eval_fixtures_perf::PERF_SPANS {
            if FIXABLE_RULES.contains(&rule) {
                assert!(
                    propose_fix(rule, code).is_some(),
                    "proposer-covered fixture for {rule} does not fire its proposer"
                );
            }
        }
    }

    // ── Issue 134 T3 re-arm — thin-domain fixture verification ──────────
    //
    // Two invariants per domain, both the perf/clippy convention: (1) every
    // label is a REAL corpus rule id (a typo'd id would read as wrong_span
    // on every run forever — the measurement would silently carry it); (2)
    // proposer-covered rules (FIXABLE_SEC / FIXABLE_DOCKER / FIXABLE_SHADER)
    // fire their own proposer on their span. dist ships no proposers
    // (suggest-only by design) — membership is its only check.
    #[cfg(feature = "sec")]
    #[test]
    fn sec_fixture_labels_verify() {
        use crate::domain::HealDomain;
        let corpus: std::collections::BTreeSet<String> = crate::domains::sec::SecDomain::build_corpus()
            .into_iter()
            .map(|e| e.rule_id().to_string())
            .collect();
        for &(rule, code) in super::super::eval_fixtures_sec::SEC_SPANS {
            assert!(
                corpus.contains(rule),
                "sec fixture label {rule} is not a corpus rule id"
            );
            if crate::domains::sec::proposers::FIXABLE_SEC.contains(&rule) {
                let file = syn::parse_file(code)
                    .unwrap_or_else(|e| panic!("sec fixture for {rule} does not parse: {e}"));
                assert!(
                    !crate::domains::sec::proposers::collect_edits(rule, code, &file).is_empty(),
                    "proposer-covered sec fixture for {rule} does not fire its proposer"
                );
            }
        }
    }

    #[cfg(feature = "docker")]
    #[test]
    fn docker_fixture_labels_verify() {
        use crate::domain::HealDomain;
        let corpus: std::collections::BTreeSet<String> =
            crate::domains::docker::DockerDomain::build_corpus()
                .into_iter()
                .map(|e| e.rule_id().to_string())
                .collect();
        for &(rule, code) in super::super::eval_fixtures_docker::DOCKER_SPANS {
            assert!(
                corpus.contains(rule),
                "docker fixture label {rule} is not a corpus rule id"
            );
            if crate::domains::docker::rules::FIXABLE_DOCKER.contains(&rule) {
                assert!(
                    !crate::domains::docker::proposers::collect_edits(rule, code).is_empty(),
                    "proposer-covered docker fixture for {rule} does not fire its proposer"
                );
            }
        }
    }

    #[cfg(feature = "dist")]
    #[test]
    fn dist_fixture_labels_verify() {
        use crate::domain::HealDomain;
        let corpus: std::collections::BTreeSet<String> =
            crate::domains::dist::DistDomain::build_corpus()
                .into_iter()
                .map(|e| e.rule_id().to_string())
                .collect();
        for &(rule, _code) in super::super::eval_fixtures_dist::DIST_SPANS {
            assert!(
                corpus.contains(rule),
                "dist fixture label {rule} is not a corpus rule id"
            );
        }
    }

    #[cfg(feature = "shader_perf")]
    #[test]
    fn shader_fixture_labels_verify() {
        use crate::domain::HealDomain;
        let corpus: std::collections::BTreeSet<String> =
            crate::domains::shader_perf::ShaderPerfDomain::build_corpus()
                .into_iter()
                .map(|e| e.rule_id().to_string())
                .collect();
        for &(rule, code) in super::super::eval_fixtures_shader::SHADER_SPANS {
            assert!(
                corpus.contains(rule),
                "shader fixture label {rule} is not a corpus rule id"
            );
            if crate::domains::shader_perf::proposers::FIXABLE_SHADER.contains(&rule) {
                assert!(
                    !crate::domains::shader_perf::proposers::collect_edits(rule, code).is_empty(),
                    "proposer-covered shader fixture for {rule} does not fire its proposer"
                );
            }
        }
    }

    /// Clean-span truth for the 2026-08-17 proposer additions: no real
    /// production excerpt fires the five new rewrites (the perf-domain
    /// analog of [`Self::clean_spans_fire_no_lint`], scoped to the NEW
    /// proposers — the pre-existing five are Bench-019-G5-validated on real
    /// files instead).
    #[cfg(feature = "rust_perf")]
    #[test]
    fn clean_spans_fire_no_new_perf_proposer() {
        use crate::domains::rust_perf::proposers::propose_fix;
        const NEW_RULES: &[&str] = &[
            "hashset-insert-bool-return",
            "collect-append-to-extend",
            "vec-macro-zero-fill",
            "distance-sq-no-sqrt",
            "hashmap-with-capacity",
        ];
        for (tag, code) in CLEAN_SPANS {
            for &rule in NEW_RULES {
                assert!(
                    propose_fix(rule, code).is_none(),
                    "clean span {tag} fires {rule}"
                );
            }
        }
    }

    /// Clean-span truth: no real-code excerpt contains any of the 37
    /// STRUCTURAL lint patterns (FIXABLE + VERIFY). The three type-blind
    /// self_evolve-only proposers (`needless_borrow`, `useless_conversion`,
    /// `unnecessary_mut_passed`) are excluded BY DESIGN: they over-match on
    /// parse-only information (e.g. every necessary `f(&local)` borrow) —
    /// that is exactly why they are not `--fix` eligible and only fire
    /// through the cargo-clippy-verified loop. Real production code
    /// legitimately trips them.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn clean_spans_fire_no_lint() {
        use crate::domains::clippy_lints::proposers::{propose_fix, FIXABLE_LINTS, VERIFY_LINTS};
        let structural: Vec<&str> = FIXABLE_LINTS.iter().chain(VERIFY_LINTS).copied().collect();
        for (tag, code) in CLEAN_SPANS {
            for &lint in &structural {
                assert!(
                    propose_fix(lint, code).is_none(),
                    "clean span {tag} fires {lint}"
                );
            }
        }
    }

    #[cfg(feature = "kernel_opt")]
    #[test]
    fn kernel_mutation_is_deterministic_and_renames() {
        let spans = kernel_fixture_spans(30);
        assert_eq!(spans.len(), 30);
        let again = kernel_fixture_spans(30);
        assert_eq!(spans, again, "selection + mutation must be deterministic");
        // The renames actually apply: no output token still equals a mapped
        // key in word position for a shape containing one.
        let mutated = mutate_shape("let buf = vals[idx] + acc;");
        assert_eq!(mutated, "let scratch = items[slot] + total;");
    }

    #[test]
    fn mutate_shape_keeps_embedded_tokens() {
        // Word-boundary exactness: `output` must NOT become `dstput`, and
        // underscore-joined identifiers (`n_out`) are SINGLE tokens.
        assert_eq!(mutate_shape("let output = out;"), "let output = dst;");
        assert_eq!(mutate_shape("let n_out = 2;"), "let n_out = 2;");
    }

    #[test]
    fn fixture_counts_meet_the_issue_floor() {
        // Issue 017 acceptance: >= 100 labeled spans including the clean
        // class. Static fixtures alone: 40 + 30 + 12 = 82; the kernel probe
        // (30) completes the set when enabled. The committed floor counts
        // what the CURRENT feature state can evaluate.
        let clean = CLEAN_SPANS.len();
        #[allow(unused_mut)]
        let mut labeled = 0;
        #[cfg(feature = "clippy_lints")]
        {
            labeled += super::super::eval_fixtures_clippy::CLIPPY_SPANS.len();
        }
        #[cfg(feature = "rust_perf")]
        {
            labeled += super::super::eval_fixtures_perf::PERF_SPANS.len();
        }
        #[cfg(feature = "kernel_opt")]
        {
            labeled += kernel_fixture_spans(30).len();
        }
        let total = labeled + clean;
        assert!(
            total >= 100,
            "fixture set shrank below the Issue 017 floor: {labeled} labeled + {clean} clean = {total}"
        );
    }

    /// Issue 020 POC measurement: prints the variant table for the record
    /// (`--nocapture`). The PASS/KILL decision gates live in
    /// `router_spectral_gate` (asserting on the WIRED mode) + Bench 025.
    #[test]
    fn router_spectral_poc_report() {
        let rows = run_router_eval();
        print_router_eval(&rows);
        // Structural sanity in every state: the measurement itself must be
        // deterministic + complete.
        assert!(!rows.is_empty());
        let again = run_router_eval();
        assert_eq!(
            rows.iter()
                .map(|r| (r.correct, r.misroutes_fixed))
                .collect::<Vec<_>>(),
            again
                .iter()
                .map(|r| (r.correct, r.misroutes_fixed))
                .collect::<Vec<_>>(),
            "router eval must be deterministic"
        );
    }

    /// Plan 018 T1.5 — the sec-domain self-variant floor (the kernel
    /// precedent): every sec corpus rule's `buggy_code`, identifier-renamed
    /// via the shared deterministic map, must retrieve ITS OWN rule at top-1
    /// in the sec domain. Establishes the new domain's retrieval floor
    /// WITHOUT touching the shared eval fixture set (the 4-domain router
    /// annotation floor is re-pinned separately — see
    /// [`floors::router_annotation_floor`]).
    #[cfg(feature = "sec")]
    #[test]
    fn sec_self_variants_retrieve_top1() {
        use crate::domain::HealDomain;
        use crate::domains::sec::SecDomain;
        let corpus = SecDomain::build_corpus();
        assert!(corpus.len() >= 6, "Phase-1 survival count");
        let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
        let mut misses = Vec::with_capacity(corpus.len());
        for rule in &corpus {
            let probe = mutate_shape(&rule.buggy_code);
            let (_routed, by_domain) = matcher.suggest_span(&probe, 3);
            let sec_rules = by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Sec)
                .map(|(_, rules)| rules)
                .expect("sec domain present");
            let top1 = sec_rules.first().map_or("", |r| r.rule_id.as_str());
            if top1 != rule.rule_id {
                misses.push(format!(
                    "{}: top1={top1} (want {})",
                    rule.rule_id, rule.rule_id
                ));
            }
        }
        assert!(
            misses.is_empty(),
            "sec self-variant top-1 misses:\n{}",
            misses.join("\n")
        );
    }

/// Batch 266 (Plan 270 / Research 302) T8: the new rule's own span must
    /// land top-1 in the rust_perf lane — the standing self-variant assert
    /// the retrieval floors ride.
    #[test]
    fn batch266_self_variant_retrieves_top1() {
        use crate::domain::HealDomain;
        use crate::domains::rust_perf::RustPerfDomain;
        let corpus = RustPerfDomain::build_corpus();
        let rule = corpus
            .iter()
            .find(|e| e.rule_id() == "isop-cover-from-decision-diagram-not-path-simplification")
            .expect("batch-266 rule must be in the corpus");
        let probe = mutate_shape(rule.buggy_code());
        let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
        let (_routed, by_domain) = matcher.suggest_span(&probe, 3);
        let perf_rules = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Perf)
            .map(|(_, rules)| rules)
            .expect("rust_perf domain present");
        let top1 = perf_rules.first().map_or("", |r| r.rule_id.as_str());
        assert_eq!(
            top1,
            "isop-cover-from-decision-diagram-not-path-simplification",
            "batch-266 self-variant must land top-1 (got {top1})"
        );
    }

    /// Issue 070 T1 — the cross-domain agreement pre-check (`ThoughtComm`
    /// α-fusion `DoA` measurement; source Research 364; the issue was
    /// resolved + removed 2026-09-05 — record in AGENTS.md + git history).
    ///
    /// The healer fans each span out across one index per enabled domain and
    /// re-ranks within a domain. An α-fusion arm (the issue's T2) needs ≥2
    /// domains to contribute entries addressing the SAME span shape; this
    /// measures how often that can happen at all, over the composite fixture
    /// corpus + the Issue-017 labeled spans, through the same fan-out the
    /// healer uses (`LatentMatcher::suggest_span`, pool = [`RERANK_POOL`]).
    ///
    /// Three operationalizations, weakest to strongest — per span:
    ///
    /// - **A. rule-id overlap** — the same `rule_id` retrieved by ≥2
    ///   DISTINCT domains. Structurally impossible by construction (each
    ///   domain's corpus owns a disjoint id namespace), so the test ASSERTS
    ///   zero as a corpus invariant: a nonzero means the namespaces collided
    ///   and T2's premise changes.
    /// - **B. pattern-text overlap** — two domains retrieve entries whose
    ///   corpus pattern text (`buggy_code`) is byte-identical, i.e. the same
    ///   anti-pattern was independently authored into both corpora. This is
    ///   the issue's "re-scope to pattern-text overlap" viability probe.
    /// - **C. instantiation co-presence (`α_j`)** — the count of domains with
    ///   ≥1 candidate whose proposer FIRED on the span (post
    ///   `mark_instantiation`); agreement = α ≥ 2. This is the exact
    ///   quantity a count-tiered fusion would consume. It measures
    ///   co-presence of addressable anti-patterns, NOT same-rule agreement —
    ///   domain B firing on a different rule is still a countable witness.
    ///
    /// Measured domain set = whatever is feature-enabled at test time; the
    /// committed gate state is defaults + `latent_retrieval` =
    /// `clippy_lints` + `rust_perf` + `docker`. Kernel/sec opt-ins would add
    /// nothing measurable here (kernel ships no proposers and GPU-kernel
    /// patterns; sec's corpus is FFI/Drop shapes) — enabling them would only
    /// widen the retrieval fan, not the firing surface, which is itself part
    /// of the `DoA` verdict for this fixture corpus.
    ///
    /// Report-first: the measured rates print via `--nocapture` and ONLY
    /// structural invariants assert (A == 0, histogram sums, measurable > 0)
    /// — a measured rate must never be pinned by the instrument that
    /// measures it.
    #[cfg(all(
        feature = "latent_retrieval",
        feature = "clippy_lints",
        feature = "rust_perf"
    ))]
    #[test]
    // A report-first test: the three operationalizations + their printers are
    // one narrative unit — splitting it would separate the measurement from
    // the doc block that pins its semantics (the batch98 too_many_lines
    // precedent).
    #[allow(clippy::too_many_lines)]
    fn cross_domain_agreement_rate_report() {
        use crate::draft::instantiation_rerank::{RerankMode, RERANK_POOL};
        use std::collections::{HashMap, HashSet};

        let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
        let measured_domains: Vec<String> = DomainId::ALL
            .iter()
            .filter(|d| matcher.has_domain(**d))
            .map(|d| format!("{d:?}"))
            .collect();

        // (name, code, is_composite). The composite tags are handles, not
        // labels — this measurement needs no oracle at all.
        let mut spans: Vec<(String, String, bool)> = Vec::new();
        for (tag, code) in crate::draft::eval_fixtures_composite::COMPOSITE_SPANS {
            spans.push(((*tag).to_string(), (*code).to_string(), true));
        }
        for s in labeled_eval_spans() {
            spans.push((s.label, s.code, false));
        }

        let pool = RERANK_POOL;
        let mut measurable = 0usize; // spans served by >= 2 domain indexes
        let mut agree_id = 0usize;
        let mut agree_text = 0usize;
        let mut agree_alpha = 0usize;
        let mut alpha_hist = [0usize; 6]; // alpha = 0..=4, 5+ bucketed last
        let (mut comp_n, mut comp_alpha) = (0usize, 0usize);
        let (mut lab_n, mut lab_alpha) = (0usize, 0usize);

        for (name, code, is_composite) in &spans {
            let (_routed, by_domain) = matcher.suggest_span(code, pool);
            if by_domain.len() < 2 {
                continue;
            }
            measurable += 1;
            if *is_composite {
                comp_n += 1;
            } else {
                lab_n += 1;
            }

            // A — rule-id overlap across DISTINCT domain lists.
            let mut id_domains: HashMap<&str, HashSet<String>> = HashMap::new();
            for (d, rules) in &by_domain {
                for r in rules {
                    id_domains
                        .entry(r.rule_id.as_str())
                        .or_default()
                        .insert(format!("{d:?}"));
                }
            }
            if id_domains.values().any(|ds| ds.len() >= 2) {
                agree_id += 1;
            }

            // B — byte-identical corpus pattern text across domains (via the
            // rule book; an id missing from the book cannot contribute).
            let mut text_domains: HashMap<&str, HashSet<String>> = HashMap::new();
            for (d, rules) in &by_domain {
                for r in rules {
                    if let Some(info) = matcher.rule_book(&r.rule_id) {
                        text_domains
                            .entry(info.buggy_code.as_str())
                            .or_default()
                            .insert(format!("{d:?}"));
                    }
                }
            }
            if text_domains.values().any(|ds| ds.len() >= 2) {
                agree_text += 1;
            }

            // C — α_j: domains contributing a firing proposer on this span.
            let alpha = by_domain
                .iter()
                .filter(|(_, rules)| rules.iter().any(|r| r.instantiates))
                .count();
            alpha_hist[alpha.min(5)] += 1;
            if alpha >= 2 {
                agree_alpha += 1;
                if *is_composite {
                    comp_alpha += 1;
                } else {
                    lab_alpha += 1;
                }
                let witnesses: Vec<String> = by_domain
                    .iter()
                    .filter(|(_, rules)| rules.iter().any(|r| r.instantiates))
                    .map(|(d, rules)| {
                        let fired: Vec<&str> = rules
                            .iter()
                            .filter(|r| r.instantiates)
                            .map(|r| r.rule_id.as_str())
                            .collect();
                        format!("{:?}:{}", d, fired.join(","))
                    })
                    .collect();
                println!(
                    "   alpha={alpha} span '{name}' <- {}",
                    witnesses.join(" | ")
                );
            }
        }

        // display-only percentages over a bounded span population (< 2^23);
        // the cast IS the print format.
        #[allow(clippy::cast_precision_loss)]
        let pct = |n: usize| 100.0 * n as f32 / measurable as f32;
        println!("\n== Issue 070 T1: cross-domain agreement (fan-out retrieval, pool {pool}) ==");
        println!("domains measured: {}", measured_domains.join(", "));
        println!(
            "spans: {} ({} composite + {} labeled); measurable (>=2 domain indexes): {}",
            spans.len(),
            comp_n,
            lab_n,
            measurable
        );
        assert!(
            measurable > 0,
            "fan-out must serve >= 2 domain indexes per span"
        );
        println!(
            "A rule-id overlap:             {}/{} = {:.1}%",
            agree_id,
            measurable,
            pct(agree_id)
        );
        println!(
            "B pattern-text overlap:        {}/{} = {:.1}%",
            agree_text,
            measurable,
            pct(agree_text)
        );
        println!(
            "C alpha>=2 (Structural mode):  {}/{} = {:.1}%",
            agree_alpha,
            measurable,
            pct(agree_alpha)
        );
        println!("   composite-only: {comp_alpha}/{comp_n}   labeled-only: {lab_alpha}/{lab_n}");
        println!(
            "   alpha histogram (domains with a firing proposer): 0:{} 1:{} 2:{} 3:{} 4:{} 5+:{}",
            alpha_hist[0],
            alpha_hist[1],
            alpha_hist[2],
            alpha_hist[3],
            alpha_hist[4],
            alpha_hist[5]
        );

        // Structural invariant (asserted, never measured): the per-domain id
        // namespaces are disjoint by construction.
        assert_eq!(
            agree_id, 0,
            "the same rule id was retrieved by >= 2 domains — per-domain id namespaces collided; T2's premise changes"
        );
        assert_eq!(
            alpha_hist.iter().sum::<usize>(),
            measurable,
            "the alpha histogram must account for every measurable span"
        );

        // Secondary pass — the `All` rerank arm widens the proposer set with
        // the type-blind over-matchers. This is the honest UPPER bound for
        // what a fusion tier could count as witnesses.
        let mut all_matcher = LatentMatcher::from_enabled_domains()
            .expect("build matcher")
            .with_rerank_mode(RerankMode::All);
        let mut agree_alpha_all = 0usize;
        for (_name, code, _c) in &spans {
            let (_routed, by_domain) = all_matcher.suggest_span(code, pool);
            if by_domain.len() < 2 {
                continue;
            }
            if by_domain
                .iter()
                .filter(|(_, rules)| rules.iter().any(|r| r.instantiates))
                .count()
                >= 2
            {
                agree_alpha_all += 1;
            }
        }
        println!(
            "C alpha>=2 (All mode, upper bound): {agree_alpha_all}/{measurable} = {:.1}%",
            pct(agree_alpha_all)
        );
    }

    // ── Issue 125: the choice-taxonomy seam (pure, no matcher) ───────────

    fn scored(rule_id: &str, instantiates: bool) -> crate::draft::ScoredRule {
        crate::draft::ScoredRule {
            rule_id: rule_id.to_string(),
            score: 0.0,
            instantiates,
        }
    }

    #[test]
    fn taxonomy_classifies_the_four_outcomes() {
        use super::{classify_choice_span, ChoiceOutcome, DeclineClass};

        let acc = vec!["needless_return".to_string()];
        // strict hit: gold top-1
        let ranked = vec![scored("let_and_return", true), scored("other", false)];
        let c = classify_choice_span("let_and_return", &[], &ranked, |_| true);
        assert_eq!(c.outcome, ChoiceOutcome::CorrectRule);
        assert_eq!(c.decline, DeclineClass::Covered);

        // acceptable hit: an authored alternative wins top-1, gold lower
        let ranked = vec![scored("needless_return", true), scored("let_and_return", false)];
        let c = classify_choice_span("let_and_return", &acc, &ranked, |_| true);
        assert_eq!(c.outcome, ChoiceOutcome::CorrectAcceptable);

        // wrong-rule: gold surfaced but outranked (no acceptable rescue)
        let ranked = vec![scored("other", true), scored("let_and_return", false)];
        let c = classify_choice_span("let_and_return", &[], &ranked, |_| true);
        assert_eq!(c.outcome, ChoiceOutcome::WrongRule);

        // wrong-span: neither gold nor the acceptable alternative ever
        // surfaced — the span pipeline, not discrimination
        let ranked = vec![scored("other", true), scored("b", false)];
        let c = classify_choice_span("let_and_return", &acc, &ranked, |_| true);
        assert_eq!(c.outcome, ChoiceOutcome::WrongSpan);
    }

    #[test]
    fn taxonomy_decline_axis_is_three_way() {
        use super::{classify_choice_span, DeclineClass};

        // covered: something fired
        let ranked = vec![scored("a", false), scored("b", true)];
        assert_eq!(
            classify_choice_span("gold", &[], &ranked, |_| false).decline,
            DeclineClass::Covered
        );
        // declined: probeable present, none fired — the real decline
        let ranked = vec![scored("a", false), scored("b", false)];
        assert_eq!(
            classify_choice_span("gold", &[], &ranked, |_| true).decline,
            DeclineClass::Declined
        );
        // proposer-blind: nothing probeable — the signal is absent
        assert_eq!(
            classify_choice_span("gold", &[], &ranked, |_| false).decline,
            DeclineClass::ProposerBlind
        );
        // empty window: blind (no candidates at all)
        assert_eq!(
            classify_choice_span("gold", &[], Vec::<&crate::draft::ScoredRule>::new(), |_| true)
                .decline,
            DeclineClass::ProposerBlind
        );
    }

    #[test]
    fn taxonomy_ratios_and_partition() {
        use super::ChoiceTaxonomy;

        let mut t = ChoiceTaxonomy::default();
        // A: covered, strict-correct — gold top-1 and firing
        let ranked = vec![scored("gold", true), scored("other", false)];
        t.absorb(super::classify_choice_span("gold", &[], &ranked, |_| true));
        // B: covered, wrong-rule — another rule fires above gold
        let ranked = vec![scored("other", true), scored("gold", false)];
        t.absorb(super::classify_choice_span("gold", &[], &ranked, |_| true));
        // C: declined, wrong-span — gold absent, probeable candidates none fired
        let ranked = vec![scored("other", false), scored("x", false)];
        t.absorb(super::classify_choice_span("gold", &[], &ranked, |_| true));
        // D: proposer-blind, wrong-rule — gold surfaced but nothing probeable
        let ranked = vec![scored("other", false), scored("gold", false)];
        t.absorb(super::classify_choice_span("gold", &[], &ranked, |_| false));
        // E: covered, acceptable-correct — an authored alternative wins top-1
        let ranked = vec![scored("alt", true), scored("gold", false)];
        let acc = vec!["alt".to_string()];
        t.absorb(super::classify_choice_span("gold", &acc, &ranked, |_| true));

        assert_eq!(t.labeled, 5);
        assert_eq!((t.correct_rule, t.correct_via_acceptable, t.wrong_rule, t.wrong_span), (1, 1, 2, 1));
        assert_eq!((t.covered, t.declined, t.proposer_blind), (3, 1, 1));
        assert_eq!(t.selective_correct, 2);
        assert!((t.coverage().unwrap() - 0.75).abs() < 1e-6);
        assert!((t.abstention_rate().unwrap() - 0.25).abs() < 1e-6);
        assert!((t.selective_accuracy().unwrap() - 2.0 / 3.0).abs() < 1e-6);
        // outcomes partition the labeled set exactly
        assert_eq!(
            t.correct_rule + t.correct_via_acceptable + t.wrong_rule + t.wrong_span,
            t.labeled
        );
        assert_eq!(t.covered + t.declined + t.proposer_blind, t.labeled);
    }
}

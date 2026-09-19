//! Issue 034 T1 — `DFlash2` selection-headroom probe (gate zero).
//!
//! `DFlash2`'s reported contribution is a *selector*: its Table 1 shows
//! Recall@1 85.4% against Recall@16 99.5%, i.e. a ~14pp band where the
//! correct answer is already in the candidate pool and the per-candidate
//! score simply fails to put it first. Research 077 established that the
//! released artifact cannot generate standalone, so the only part of `DFlash2`
//! that transfers to this crate without target weights is that selector —
//! and a selector can only ever recover headroom that exists.
//!
//! This module measures whether it exists here. For every labeled eval span
//! it asks two separate questions:
//!
//! 1. did the **production** selector choose the correct rule (recall@1,
//!    pool depth [`RERANK_POOL`])?
//! 2. was a correct candidate **anywhere** in a pool of depth K
//!    ([`HEADROOM_DEPTHS`] = 8 / 12 / 16 / 20 / 24 / 28)?
//!
//! `recall@K − recall@1` is the recoverable band. Misses split into
//! [`MissKind::Selection`] (correct rule in the pool, not chosen — a selector
//! could fix it) and [`MissKind::Retrieval`] (never retrieved — corpus or
//! embedder work, which no selector can touch). Only the Selection rows are
//! addressable, so they are the honest denominator for a G1 recovery gate.
//!
//! **Stop rule (Plan 017 T1.2):** headroom below
//! [`HEADROOM_STOP_BAR_PP`] on both measured surfaces means the residual band
//! is noise and the selector must not be built. See
//! [`HeadroomVerdict::stop`].
//!
//! Pool depth is a real configuration knob, not a read-off: the matcher
//! retrieves `max(top_k, RERANK_POOL)` candidates and re-ranks *the pool*, so
//! a depth-24 probe can promote a rule that the depth-20 production path never
//! saw. Each depth is therefore measured by its own query rather than by
//! slicing one deep list — which is also why `recall@8` can exceed nothing and
//! `recall@24` is not automatically ≥ `recall@20`.
//!
//! The second surface (real-`cargo clippy` ground truth over the multi-lint
//! composite spans) lives in `tests/uncontaminated_g1.rs`, next to the oracle
//! harness it reuses.
//!
//! ```bash
//! cargo test --release --features latent_retrieval \
//!   -p riir-clippy --lib draft::headroom_probe -- --nocapture
//! ```

use super::domain_router::DomainId;
use super::instantiation_rerank::RERANK_POOL;
use super::latent_matcher::LatentMatcher;
use super::retrieval_eval::{canonical_rank, domain_index, labeled_eval_spans, OwnedEvalSpan};

/// Candidate-pool depths probed, ascending. `RERANK_POOL` (the production
/// depth) must be one of them — it is the recall@1 reference point.
pub const HEADROOM_DEPTHS: [usize; 7] = [8, 12, 16, 20, 24, 28, 32];

/// Index of the production depth within [`HEADROOM_DEPTHS`].
/// 12 → 16 at the P10 re-pin (2026-09-13, `RERANK_POOL` 12 → 16 after the
/// corpus +2 crowded the Issue-024 casualty from raw rank 9 to 15); 16 → 20
/// at the P11 re-pin (same day — the `format_collect` corpus entry crowded
/// the casualty one further position, first-found at depth 17); 20 → 24 at
/// the Batch 159 re-pin (2026-09-18, Plan 150 — the pack V pair crowded
/// the casualty to first-found at depth 21); 24 → 28 at the Batch 162
/// re-pin (2026-09-19, Plan 153 — the pack VIII pair crowded the casualty
/// to first-found at depth 25); the array gained a rung each time (24,
/// 28, then 32) so the recoverable band (recall@deepest −
/// recall@1) keeps a rung ABOVE production — the probe's whole reason
/// to exist. Keeping production at the array's deep end would zero the
/// band by construction, which reads as "no headroom" and is actually
/// "no deeper probe".
pub const PROD_DEPTH_IDX: usize = 5;

/// Plan 017 T1.2 stop bar: below this many percentage points of
/// `recall@deepest − recall@1`, the band is noise and no selector is built.
pub const HEADROOM_STOP_BAR_PP: f32 = 2.0;

/// Headroom counters for one domain (or blended across domains).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HeadroomMetrics {
    /// Labeled spans scored.
    pub labeled: usize,
    /// Spans where the production selector's top-1 was correct.
    pub chosen: usize,
    /// Per [`HEADROOM_DEPTHS`] entry: spans where a correct candidate was
    /// present anywhere in the depth-K pool.
    pub in_pool: [usize; HEADROOM_DEPTHS.len()],
}

/// Ratio helper — span counts are ≤ 130, so the cast is exact by orders of
/// magnitude (display arithmetic, not accumulated state).
#[allow(clippy::cast_precision_loss)]
fn ratio(hits: usize, n: usize) -> Option<f32> {
    match n {
        0 => None,
        _ => Some(hits as f32 / n as f32),
    }
}

impl HeadroomMetrics {
    /// Production recall@1; `None` when nothing was labeled.
    #[must_use]
    pub fn recall_at_1(&self) -> Option<f32> {
        ratio(self.chosen, self.labeled)
    }

    /// Pool recall at [`HEADROOM_DEPTHS`]`[idx]`; `None` when nothing was
    /// labeled or `idx` is out of range.
    #[must_use]
    pub fn pool_recall(&self, idx: usize) -> Option<f32> {
        match self.in_pool.get(idx) {
            Some(&hits) => ratio(hits, self.labeled),
            None => None,
        }
    }

    /// The deepest-pool recall — the ceiling a perfect selector could reach.
    #[must_use]
    pub fn ceiling(&self) -> Option<f32> {
        self.pool_recall(HEADROOM_DEPTHS.len() - 1)
    }

    /// `recall@deepest − recall@1` in percentage points: the band a selector
    /// could recover. `None` when nothing was labeled.
    #[must_use]
    pub fn headroom_pp(&self) -> Option<f32> {
        match (self.ceiling(), self.recall_at_1()) {
            (Some(ceil), Some(top1)) => Some((ceil - top1) * 100.0),
            _ => None,
        }
    }
}

/// Why a span was missed — the only distinction that matters for a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissKind {
    /// A correct candidate sat in the deepest pool but was not chosen.
    /// Addressable by selection.
    Selection,
    /// No correct candidate was retrieved at any probed depth. Corpus or
    /// embedder work — a selector cannot recover it.
    Retrieval,
}

/// One missed span, classified.
#[derive(Debug, Clone)]
pub struct HeadroomMiss {
    /// Ground-truth rule id.
    pub expected: String,
    pub domain: DomainId,
    /// 0-based rank in the deepest pool; `None` for a retrieval miss.
    pub rank: Option<usize>,
    /// What the production selector chose instead.
    pub chosen: Option<String>,
    pub kind: MissKind,
}

/// Probe output.
#[derive(Debug, Default)]
pub struct HeadroomReport {
    /// Fixed domain order: clippy, perf, kernel (`labeled == 0` = domain off).
    pub per_domain: [HeadroomMetrics; 3],
    /// All labeled spans across enabled domains.
    pub blended: HeadroomMetrics,
    /// Every missed span, classified by [`MissKind`].
    pub misses: Vec<HeadroomMiss>,
    /// Spans skipped because their domain's index is feature-off.
    pub skipped: usize,
}

impl HeadroomReport {
    /// Count of misses of one kind.
    #[must_use]
    pub fn miss_count(&self, kind: MissKind) -> usize {
        self.misses.iter().filter(|m| m.kind == kind).count()
    }

    /// The T1.2 verdict for this surface.
    #[must_use]
    pub fn verdict(&self) -> HeadroomVerdict {
        let headroom_pp = self.blended.headroom_pp().unwrap_or(0.0);
        HeadroomVerdict {
            headroom_pp,
            selection_misses: self.miss_count(MissKind::Selection),
            retrieval_misses: self.miss_count(MissKind::Retrieval),
            stop: headroom_pp < HEADROOM_STOP_BAR_PP,
        }
    }
}

/// The stop-rule readout for one surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadroomVerdict {
    /// `recall@deepest − recall@1`, percentage points.
    pub headroom_pp: f32,
    /// Misses a selector could recover.
    pub selection_misses: usize,
    /// Misses no selector can recover.
    pub retrieval_misses: usize,
    /// `headroom_pp < HEADROOM_STOP_BAR_PP` — record the negative, build nothing.
    pub stop: bool,
}

/// Rank the enabled-domain list for `code` at pool depth `k`, returning the
/// rule ids in production's canonical order.
///
/// Owned ids: the matcher is borrowed mutably per query (shared G4 scratch),
/// so a borrowed view cannot outlive the next depth's call.
fn ranked_ids_at(
    matcher: &mut LatentMatcher,
    code: &str,
    domain: DomainId,
    k: usize,
) -> Vec<String> {
    let (_, by_domain) = matcher.suggest_span(code, k);
    by_domain
        .iter()
        .find(|(d, _)| *d == domain)
        .map_or_else(Vec::new, |(_, rules)| {
            canonical_rank(rules)
                .iter()
                .take(k)
                .map(|r| r.rule_id.clone())
                .collect()
        })
}

/// Run the headroom probe over the Issue 017 labeled fixture set with the
/// promoted default re-rank mode.
///
/// # Panics
///
/// Panics if no domain index can be built (no domain feature enabled — the
/// committed suites always enable `clippy_lints` + `rust_perf`).
#[must_use]
pub fn run_headroom_probe() -> HeadroomReport {
    run_headroom_probe_with(super::instantiation_rerank::RerankMode::default())
}

/// [`run_headroom_probe`] against an explicit re-rank mode — the A/B entry
/// point (headroom is a property of the selector under test, so the mode
/// must be nameable).
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails.
#[must_use]
pub fn run_headroom_probe_with(rerank: super::instantiation_rerank::RerankMode) -> HeadroomReport {
    run_headroom_probe_with_spans(rerank, &labeled_eval_spans())
}

/// [`run_headroom_probe_with`] over an EXPLICIT span set — the Plan 049
/// T0.0a injection point. The Issue 017 fixture set (`labeled_eval_spans`)
/// is only one source; Plan 049 T0.2 re-runs this same instrument over
/// oracle-labeled real-repo spans, which was impossible while the span
/// source was hardcoded inside the probe. The no-arg entry point is
/// DEFINED as this fn over the fixture set, so injecting a set can never
/// silently change what the default probe measures.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails.
#[must_use]
pub fn run_headroom_probe_with_spans(
    rerank: super::instantiation_rerank::RerankMode,
    spans: &[OwnedEvalSpan],
) -> HeadroomReport {
    const _: () = assert!(
        HEADROOM_DEPTHS[PROD_DEPTH_IDX] == RERANK_POOL,
        "the production depth must be one of the probed depths"
    );

    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(rerank);
    let mut report = HeadroomReport::default();

    for span in spans {
        if !matcher.has_domain(span.domain) {
            report.skipped += 1;
            continue;
        }

        // One query per depth: the pool IS the re-rank input, so depth is a
        // configuration, not a slice of one deep list.
        let mut lists: [Vec<String>; HEADROOM_DEPTHS.len()] = Default::default();
        for (i, &k) in HEADROOM_DEPTHS.iter().enumerate() {
            lists[i] = ranked_ids_at(&mut matcher, &span.code, span.domain, k);
        }

        let prod = &lists[PROD_DEPTH_IDX];
        let chosen_id = prod.first().cloned();
        let correct = chosen_id.as_deref() == Some(span.label.as_str());

        let deepest = &lists[HEADROOM_DEPTHS.len() - 1];
        let rank = deepest.iter().position(|id| *id == span.label);

        for m in [
            &mut report.per_domain[domain_index(span.domain)],
            &mut report.blended,
        ] {
            m.labeled += 1;
            if correct {
                m.chosen += 1;
            }
            for (i, list) in lists.iter().enumerate() {
                if list.contains(&span.label) {
                    m.in_pool[i] += 1;
                }
            }
        }

        if !correct {
            report.misses.push(HeadroomMiss {
                expected: span.label.clone(),
                domain: span.domain,
                rank,
                chosen: chosen_id,
                kind: match rank {
                    Some(_) => MissKind::Selection,
                    None => MissKind::Retrieval,
                },
            });
        }
    }

    report
}

/// The `--nocapture` table: recall@1 vs recall@{8,12,16} + the miss taxonomy.
pub fn print_headroom_report(report: &HeadroomReport) {
    let pct = |r: Option<f32>| match r {
        Some(v) => format!("{:.1}%", v * 100.0),
        None => "-".to_string(),
    };

    println!("== Issue 034 T1 — selection-headroom probe (Issue 017 fixture surface) ==");
    print!("{:<8} {:>8} {:>9}", "domain", "labeled", "recall@1");
    for k in HEADROOM_DEPTHS {
        print!(" {:>9}", format!("pool@{k}"));
    }
    println!(" {:>10}", "headroom");

    let names = ["clippy", "perf", "kernel", "blended"];
    let rows = [
        &report.per_domain[0],
        &report.per_domain[1],
        &report.per_domain[2],
        &report.blended,
    ];
    for (name, m) in names.iter().zip(rows) {
        if m.labeled == 0 {
            println!("{name:<8} {:>8} {:>9}", 0, "-");
            continue;
        }
        print!("{name:<8} {:>8} {:>9}", m.labeled, pct(m.recall_at_1()));
        for i in 0..HEADROOM_DEPTHS.len() {
            print!(" {:>9}", pct(m.pool_recall(i)));
        }
        println!(
            " {:>9}",
            m.headroom_pp()
                .map_or("-".to_string(), |pp| format!("{pp:+.1}pp"))
        );
    }
    println!("  (skipped: {})", report.skipped);

    let v = report.verdict();
    println!(
        "\n-- miss taxonomy ({} selection / {} retrieval) --",
        v.selection_misses, v.retrieval_misses
    );
    if report.misses.is_empty() {
        println!("  none");
    }
    for m in &report.misses {
        match m.kind {
            MissKind::Selection => println!(
                "  SELECTION  [{:?}] expected={} rank={} chosen={}",
                m.domain,
                m.expected,
                m.rank.map_or_else(|| "?".to_string(), |r| r.to_string()),
                m.chosen.as_deref().unwrap_or("(none)"),
            ),
            MissKind::Retrieval => println!(
                "  RETRIEVAL  [{:?}] expected={} not in pool@{} chosen={}",
                m.domain,
                m.expected,
                HEADROOM_DEPTHS[HEADROOM_DEPTHS.len() - 1],
                m.chosen.as_deref().unwrap_or("(none)"),
            ),
        }
    }
    println!(
        "\n  T1.2 stop rule (bar {HEADROOM_STOP_BAR_PP:.1}pp): headroom {:+.1}pp -> {}",
        v.headroom_pp,
        if v.stop {
            "STOP — record the negative, build no selector"
        } else {
            "PROCEED — headroom exists on this surface"
        }
    );
}

/// Why a residual selection miss survives the SHIPPED selector.
///
/// The raw `recall@K − recall@1` band is NOT the band a new selector could
/// win: most of it is already reachable through knobs this crate ships. Only
/// [`ResidualCause::SelectorAddressable`] is a genuine selection failure, so
/// it is the honest denominator for a DFlash2-style recovery gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualCause {
    /// [`RerankMode::All`](super::instantiation_rerank::RerankMode::All)
    /// recovers it. The miss is the measured PRICE of excluding the four
    /// type-blind proposers from instantiation evidence (Bench 022 measured
    /// them over-matching on ordinary production code; Bench 027 kept the
    /// exclusion because `All` only ties `Structural` on quality). A pairwise
    /// selector reading the same parse-only features cannot beat that trade —
    /// it has no type information either.
    TypeBlindExclusion,
    /// The correct rule is top-1 in a DEEPER pool but production selects at
    /// [`RERANK_POOL`] depth. A `RERANK_POOL` bump fixes it; selection is not
    /// involved.
    PoolDepth,
    /// Outranked under every shipped configuration, but the correct rule has
    /// **no proposer** — so `--fix` declines even when it is selected. A
    /// selector could reorder it; the reorder heals nothing, because there is
    /// no edit to apply. Writing the proposer is the fix, not selecting
    /// better.
    UnfixableTarget,
    /// The correct rule sits in the pool at rank ≥ 1 under the strongest
    /// shipped configuration AND is fixable. The only band a new selector
    /// could convert into healed coverage — the honest G1 denominator.
    SelectorAddressable,
}

/// Does `rule_id` have a proposer that `--fix` can apply?
///
/// Mirrors the `--fix` eligibility tables rather than re-deriving them: a
/// rule outside them yields a decline no matter how well it is ranked.
#[allow(unused_variables)] // both arms vanish with their domain features off
fn is_fixable(domain: DomainId, rule_id: &str) -> bool {
    match domain {
        #[cfg(feature = "clippy_lints")]
        DomainId::Clippy => {
            use crate::domains::clippy_lints::proposers::{FIXABLE_LINTS, VERIFY_LINTS};
            FIXABLE_LINTS.contains(&rule_id) || VERIFY_LINTS.contains(&rule_id)
        }
        #[cfg(feature = "rust_perf")]
        DomainId::Perf => crate::domains::rust_perf::proposers::FIXABLE_RULES.contains(&rule_id),
        #[cfg(feature = "sec")]
        DomainId::Sec => crate::domains::sec::proposers::FIXABLE_SEC.contains(&rule_id),
        #[cfg(feature = "docker")]
        DomainId::Docker => crate::domains::docker::rules::FIXABLE_DOCKER.contains(&rule_id),
        // Kernel ships no proposers (Plan 005 Phase 5 deferred).
        _ => false,
    }
}

/// The T1 residual decomposition — the deliverable the stop rule reads.
#[derive(Debug, Default, Clone)]
pub struct HeadroomDecomposition {
    /// Spans scored.
    pub labeled: usize,
    /// Selection misses under the shipped default.
    pub shipped: Vec<String>,
    /// Misses `RerankMode::All` recovers.
    pub type_blind: Vec<String>,
    /// Misses a deeper pool recovers.
    pub pool_depth: Vec<String>,
    /// Misses whose target has no proposer — selecting it heals nothing.
    pub unfixable: Vec<String>,
    /// Misses nothing shipped recovers, on a FIXABLE target.
    pub selector_addressable: Vec<String>,
}

impl HeadroomDecomposition {
    /// The selector-addressable band in percentage points — what a `DFlash2`
    /// selector could win at BEST (perfect recovery).
    #[must_use]
    pub fn addressable_pp(&self) -> f32 {
        ratio(self.selector_addressable.len(), self.labeled).unwrap_or(0.0) * 100.0
    }

    /// Plan 017 T1.2 applied to the addressable band, not the raw one.
    #[must_use]
    pub fn stop(&self) -> bool {
        self.addressable_pp() < HEADROOM_STOP_BAR_PP
    }
}

/// Decompose the shipped selector's residual misses by what would fix them.
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails.
#[must_use]
pub fn decompose_headroom() -> HeadroomDecomposition {
    decompose_headroom_with_spans(&labeled_eval_spans())
}

/// [`decompose_headroom`] over an explicit span set — the T0.0a twin of
/// [`run_headroom_probe_with_spans`]. The trainable-band denominator is a
/// property of the SPAN SET, so the decomposition must accept the set as a
/// parameter (Plan 049 T0.2 feeds oracle-labeled real-repo spans here).
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails.
#[must_use]
pub fn decompose_headroom_with_spans(spans: &[OwnedEvalSpan]) -> HeadroomDecomposition {
    use super::instantiation_rerank::RerankMode;

    let shipped = run_headroom_probe_with_spans(RerankMode::default(), spans);
    let strongest = run_headroom_probe_with_spans(RerankMode::All, spans);

    // Key by (domain, expected rule) — the span identity for a labeled set
    // with one label per span.
    let key = |m: &HeadroomMiss| format!("{:?}/{}", m.domain, m.expected);
    let strongest_misses: Vec<(String, Option<usize>)> = strongest
        .misses
        .iter()
        .filter(|m| m.kind == MissKind::Selection)
        .map(|m| (key(m), m.rank))
        .collect();

    let mut out = HeadroomDecomposition {
        labeled: shipped.blended.labeled,
        ..Default::default()
    };

    for m in shipped
        .misses
        .iter()
        .filter(|m| m.kind == MissKind::Selection)
    {
        let k = key(m);
        let label = format!(
            "{k} (rank {}, chosen {}{})",
            m.rank.map_or_else(|| "?".to_string(), |r| r.to_string()),
            m.chosen.as_deref().unwrap_or("(none)"),
            if is_fixable(m.domain, &m.expected) {
                ""
            } else {
                ", NO PROPOSER"
            }
        );
        out.shipped.push(label.clone());
        match strongest_misses.iter().find(|(sk, _)| *sk == k) {
            // `All` recovered it: the exclusion price.
            None => out.type_blind.push(label),
            // Still missed, but it IS the top of the deepest pool — production
            // just selects from a shallower one.
            Some((_, Some(0))) => out.pool_depth.push(label),
            // Genuinely outranked. Only counts toward the selector band if
            // selecting it would actually heal the span.
            Some(_) => {
                if is_fixable(m.domain, &m.expected) {
                    out.selector_addressable.push(label);
                } else {
                    out.unfixable.push(label);
                }
            }
        }
    }
    out
}

/// The `--nocapture` decomposition table + the T1.2 verdict.
pub fn print_headroom_decomposition(d: &HeadroomDecomposition) {
    let pp = |n: usize| ratio(n, d.labeled).unwrap_or(0.0) * 100.0;
    println!("== Issue 034 T1 — residual decomposition (what would actually fix each miss) ==");
    println!(
        "  labeled {} | shipped selection misses {} ({:.1}pp)",
        d.labeled,
        d.shipped.len(),
        pp(d.shipped.len())
    );
    for (tag, rows) in [
        (
            "type-blind exclusion (RerankMode::All recovers)",
            &d.type_blind,
        ),
        ("pool depth (deeper RERANK_POOL recovers)", &d.pool_depth),
        (
            "unfixable target (no proposer -> selecting it heals nothing)",
            &d.unfixable,
        ),
        (
            "SELECTOR-ADDRESSABLE (nothing shipped recovers, target fixable)",
            &d.selector_addressable,
        ),
    ] {
        println!("\n  -- {tag}: {} ({:.1}pp) --", rows.len(), pp(rows.len()));
        if rows.is_empty() {
            println!("     none");
        }
        for r in rows {
            println!("     {r}");
        }
    }
    println!(
        "\n  T1.2 stop rule (bar {HEADROOM_STOP_BAR_PP:.2}pp): addressable band {}/{} spans \
         = {:.2}pp -> {}",
        d.selector_addressable.len(),
        d.labeled,
        d.addressable_pp(),
        if d.stop() {
            "STOP — record the negative, build no selector"
        } else {
            "PROCEED — build the selector against this denominator"
        }
    );
    println!(
        "  Read the COUNT, not the percentage: with {} addressable span(s) a \
         \"recover >= 50%\" G1 gate reduces to \"recover {} span(s)\", which is not a \
         measurable effect at this denominator.",
        d.selector_addressable.len(),
        d.selector_addressable.len().div_ceil(2),
    );
}

// Issue 090: these tests need clippy_lints AND rust_perf — the fixture
// set is clippy+perf labeled spans and the probe scores per-domain rows in
// the clippy/perf calibration order. A latent-only build (the matcher
// refuses a domain-less build) correctly drops them.
#[cfg(all(
    test,
    feature = "latent_retrieval",
    feature = "clippy_lints",
    feature = "rust_perf"
))]
mod tests {
    use super::*;

    /// The T1 measurement, printed for the record and pinned where it is
    /// structural rather than numeric.
    #[test]
    fn t1_headroom_probe_report() {
        let report = run_headroom_probe();
        print_headroom_report(&report);

        assert!(report.blended.labeled > 0, "no spans scored");

        // Every miss is classified — the taxonomy must be total, else the
        // addressable denominator is a guess.
        assert_eq!(
            report.misses.len(),
            report.blended.labeled - report.blended.chosen,
            "miss taxonomy must cover exactly the non-chosen spans"
        );

        // A pool can only ever contain what a shallower pool contained, up to
        // the re-rank promotion the deeper pool enables — so the ceiling must
        // never fall below recall@1. A violation means the probe is measuring
        // two different rankings and the headroom number is meaningless.
        let ceiling = report.blended.ceiling().expect("labeled");
        let top1 = report.blended.recall_at_1().expect("labeled");
        assert!(
            ceiling >= top1,
            "pool ceiling {ceiling:.4} below recall@1 {top1:.4} — probe is inconsistent"
        );
    }

    /// Substrate-first check: is the residual band already explained by the
    /// SHIPPED re-rank knob rather than by a missing selector?
    ///
    /// `RerankMode::Structural` (the default) counts only `FIXABLE_LINTS` as
    /// instantiation evidence, deliberately excluding the four type-blind
    /// proposers (`needless_borrow`, `useless_conversion`,
    /// `unnecessary_mut_passed`, `redundant_closure`) that Bench 022 measured
    /// over-matching on ordinary production code. `RerankMode::All` counts
    /// them. If `All` closes the band, the headroom is the PRICE of a measured
    /// precision decision, not an unexploited selection signal — and a
    /// pairwise selector over the same parse-only features cannot do better,
    /// because it has no type information either.
    #[test]
    fn t1_headroom_across_rerank_modes() {
        use crate::draft::RerankMode;
        println!("\n== Issue 034 T1 — headroom vs the SHIPPED re-rank knob ==");
        println!(
            "{:<12} {:>9} {:>9} {:>10} {:>11} {:>11}",
            "mode", "recall@1", "pool@16", "headroom", "sel-misses", "ret-misses"
        );
        let cap_iter = [RerankMode::Off, RerankMode::Structural, RerankMode::All];
        let mut rows = Vec::with_capacity(cap_iter.len());
        for mode in cap_iter {
            let r = run_headroom_probe_with(mode);
            let v = r.verdict();
            println!(
                "{:<12} {:>8.1}% {:>8.1}% {:>9.1}pp {:>11} {:>11}",
                format!("{mode:?}"),
                r.blended.recall_at_1().unwrap_or(0.0) * 100.0,
                r.blended.ceiling().unwrap_or(0.0) * 100.0,
                v.headroom_pp,
                v.selection_misses,
                v.retrieval_misses,
            );
            rows.push((mode, r));
        }

        println!("\n  -- residual selection misses per mode --");
        for (mode, r) in &rows {
            for m in r.misses.iter().filter(|m| m.kind == MissKind::Selection) {
                println!(
                    "  {mode:?}: expected={} rank={} chosen={}",
                    m.expected,
                    m.rank.map_or_else(|| "?".to_string(), |x| x.to_string()),
                    m.chosen.as_deref().unwrap_or("(none)"),
                );
            }
        }

        let structural = rows
            .iter()
            .find(|(m, _)| *m == RerankMode::Structural)
            .map(|(_, r)| r.verdict())
            .expect("structural row");
        let all = rows
            .iter()
            .find(|(m, _)| *m == RerankMode::All)
            .map(|(_, r)| r.verdict())
            .expect("all row");
        println!(
            "\n  substrate verdict: Structural {:+.1}pp -> All {:+.1}pp ({} -> {} selection misses) -- {}",
            structural.headroom_pp,
            all.headroom_pp,
            structural.selection_misses,
            all.selection_misses,
            if all.selection_misses < structural.selection_misses {
                "the SHIPPED knob already moves the band; the residual is the measured \
                 price of excluding the type-blind over-matchers (Bench 022), not an \
                 unexploited selection signal"
            } else {
                "the shipped knob does NOT move the band; the residual is orthogonal to \
                 instantiation evidence"
            }
        );
    }

    /// The T1 deliverable: the addressable band, and the stop-rule verdict
    /// read off it rather than off the raw band.
    #[test]
    fn t1_headroom_decomposition_report() {
        let d = decompose_headroom();
        print_headroom_decomposition(&d);

        // The decomposition must be total — a miss with no cause means the
        // addressable denominator is a guess.
        assert_eq!(
            d.shipped.len(),
            d.type_blind.len()
                + d.pool_depth.len()
                + d.unfixable.len()
                + d.selector_addressable.len(),
            "every shipped selection miss must be attributed to exactly one cause"
        );
        assert!(d.labeled > 0, "no spans scored");
    }

    /// The probe is a MEASUREMENT: two runs in one process must agree, or the
    /// headroom number cannot be compared across commits.
    #[test]
    fn t1_headroom_probe_is_deterministic() {
        let a = run_headroom_probe();
        let b = run_headroom_probe();
        assert_eq!(
            a.blended, b.blended,
            "blended metrics must be deterministic"
        );
        assert_eq!(
            a.misses
                .iter()
                .map(|m| (&m.expected, m.rank, m.kind))
                .collect::<Vec<_>>(),
            b.misses
                .iter()
                .map(|m| (&m.expected, m.rank, m.kind))
                .collect::<Vec<_>>(),
            "miss taxonomy must be deterministic"
        );
    }

    /// Plan 049 T0.0a: the no-arg entry points must be EXACTLY the
    /// parameterized entry points over the default fixture set. If the
    /// delegation drifts, an injected set (T0.2's real-repo spans) would be
    /// measured by a different instrument than the committed baseline.
    #[test]
    fn span_source_defaults_delegate_to_the_fixture_set() {
        use crate::draft::RerankMode;

        let spans = labeled_eval_spans();

        let probe_default = run_headroom_probe_with(RerankMode::default());
        let probe_explicit = run_headroom_probe_with_spans(RerankMode::default(), &spans);
        assert_eq!(
            probe_default.blended, probe_explicit.blended,
            "the default probe must measure the fixture set"
        );
        assert_eq!(
            probe_default
                .misses
                .iter()
                .map(|m| (&m.expected, m.rank, m.kind))
                .collect::<Vec<_>>(),
            probe_explicit
                .misses
                .iter()
                .map(|m| (&m.expected, m.rank, m.kind))
                .collect::<Vec<_>>(),
            "the default probe's miss taxonomy must match the explicit-fixture-set run"
        );

        let decomp_default = decompose_headroom();
        let decomp_explicit = decompose_headroom_with_spans(&spans);
        assert_eq!(decomp_default.labeled, decomp_explicit.labeled);
        assert_eq!(decomp_default.shipped, decomp_explicit.shipped);
        assert_eq!(decomp_default.type_blind, decomp_explicit.type_blind);
        assert_eq!(decomp_default.pool_depth, decomp_explicit.pool_depth);
        assert_eq!(decomp_default.unfixable, decomp_explicit.unfixable);
        assert_eq!(
            decomp_default.selector_addressable, decomp_explicit.selector_addressable,
            "the default decomposition must match the explicit-fixture-set run"
        );
    }

    /// Plan 049 T0.0a: the reason the injection point exists — the probe
    /// must score the set it is HANDED, not the fixtures. T0.2 feeds
    /// oracle-labeled real-repo spans through exactly this seam.
    #[test]
    fn span_source_injection_measures_exactly_the_injected_set() {
        use crate::draft::RerankMode;

        let spans = vec![
            OwnedEvalSpan {
                label: "needless_return".to_string(),
                domain: DomainId::Clippy,
                code: "fn f(x: i32) -> i32 { return x; }".to_string(),
                acceptable: Vec::new(),
            },
            OwnedEvalSpan {
                label: "unused_unit".to_string(),
                domain: DomainId::Clippy,
                code: "pub fn g() -> () { () }".to_string(),
                acceptable: Vec::new(),
            },
        ];

        let report = run_headroom_probe_with_spans(RerankMode::default(), &spans);
        assert_eq!(
            report.blended.labeled, 2,
            "the probe must score exactly the injected set"
        );
        assert_eq!(
            report.per_domain[domain_index(DomainId::Clippy)].labeled,
            2,
            "the injected spans are clippy-domain and must land in the clippy row"
        );
        // The taxonomy stays total over an injected set too.
        assert_eq!(
            report.misses.len(),
            report.blended.labeled - report.blended.chosen,
            "miss taxonomy must cover exactly the non-chosen spans"
        );

        let decomp = decompose_headroom_with_spans(&spans);
        assert_eq!(decomp.labeled, 2);
        assert_eq!(
            decomp.shipped.len(),
            decomp.type_blind.len()
                + decomp.pool_depth.len()
                + decomp.unfixable.len()
                + decomp.selector_addressable.len(),
            "every shipped selection miss must be attributed to exactly one cause"
        );
    }
}

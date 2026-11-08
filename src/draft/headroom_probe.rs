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
//!   -p riir-refine --lib draft::headroom_probe -- --nocapture
//! ```

use std::collections::BTreeSet;

use super::domain_router::DomainId;
use super::instantiation_rerank::RERANK_POOL;
use super::latent_matcher::LatentMatcher;
use super::retrieval_eval::{OwnedEvalSpan, canonical_rank, domain_index, labeled_eval_spans};

/// Candidate-pool depths probed, ascending. `RERANK_POOL` (the production
/// depth) must be one of them — it is the recall@1 reference point.
/// Rung history: each `RERANK_POOL` re-pin appends its rung AND one deeper
/// (the recoverable band needs a rung above production — zeroing it reads
/// as "no headroom" when it is "no deeper probe"). Batch 184 (Plan 179)
/// appended 33 (the pin) + 40 (the band's deep end). P51a (2026-10-14)
/// appended 37 (the pin; 40 already provides the deep rung above it).
pub const HEADROOM_DEPTHS: [usize; 10] = [8, 12, 16, 20, 24, 28, 32, 33, 37, 40];

/// Index of the production depth within [`HEADROOM_DEPTHS`].
/// 12 → 16 at the P10 re-pin (2026-09-13, `RERANK_POOL` 12 → 16 after the
/// corpus +2 crowded the Issue-024 casualty from raw rank 9 to 15); 16 → 20
/// at the P11 re-pin (same day — the `format_collect` corpus entry crowded
/// the casualty one further position, first-found at depth 17); 20 → 24 at
/// the Batch 159 re-pin (2026-09-18, Plan 150 — the pack V pair crowded
/// the casualty to first-found at depth 21); 24 → 28 at the Batch 162
/// 24 → 28 at the Batch 162
/// re-pin (2026-09-19, Plan 153 — the pack VIII pair crowded the casualty
/// to first-found at depth 25); 28 → 33 at the Batch 184 re-pin
/// (2026-09-28, Plan 179 — the pack XVI pair plus intervening corpus
/// growth crowded the casualty to first-found at depth 29-30); the array
/// gained a rung each time (24,
/// 28, then 32, then 33 + 40) so the recoverable band (recall@deepest −
/// recall@1) keeps a rung ABOVE production — the probe's whole reason
/// to exist. Keeping production at the array's deep end would zero the
/// band by construction, which reads as "no headroom" and is actually
/// "no deeper probe".
pub const PROD_DEPTH_IDX: usize = 8;

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
    /// Fixed domain order: [`DomainId::ALL`] (clippy, perf, kernel, sec,
    /// docker, dist, shader — `labeled == 0` = domain off). Sized by
    /// `DomainId::ALL.len()`, NOT by the number of domains that happen to
    /// carry fixtures — the `EvalReport::per_domain` sizing law. This array
    /// was `[_; 3]` through the docker/dist/shader landings: a latent
    /// out-of-bounds at `domain_index(Docker) == 4` that never fired only
    /// because no sec/docker/dist/shader-labeled fixture existed — fired for
    /// the first time by the Issue-134 T3 re-arm fixture sets (2026-09-24).
    pub per_domain: [HeadroomMetrics; DomainId::ALL.len()],
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

    // Every domain slot, in `DomainId::ALL` order (the `per_domain` sizing
    // law) — a domain with labeled == 0 (feature off / no fixtures) prints
    // its empty row rather than being silently omitted, so the table's
    // shape pins the full domain set.
    let mut rows: Vec<(&str, &HeadroomMetrics)> = DomainId::ALL
        .iter()
        .map(|d| (d.label(), &report.per_domain[domain_index(*d)]))
        .collect();
    rows.push(("blended", &report.blended));
    for (name, m) in rows {
        if m.labeled == 0 {
            println!("{name:<12} {:>8} {:>9}", 0, "-");
            continue;
        }
        print!("{name:<12} {:>8} {:>9}", m.labeled, pct(m.recall_at_1()));
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

// ── Plan 351 T0.1: per-span oracle-depth census ──────────────────────
//
// The adaptive-pool-depth premise (`.plans/351_adaptive_rerank_pool_depth.md`)
// is a MEASUREMENT before it is a mechanism: is the oracle pick already at
// top-1 at a shallow pool depth for most spans (adaptivity has room), or does
// every span need the full `RERANK_POOL` (the paper's premise does not
// transfer)? The census reads d* per span off the SAME per-depth machinery the
// headroom probe uses (`ranked_ids_at` over [`HEADROOM_DEPTHS`]) — read-only
// reuse, no new ranking surface. Kill-gates (pinned by the plan, evaluated in
// [`print_oracle_depth_census`]): (a) ≥ 80% of reachable spans at
// d* ≥ 0.75·RERANK_POOL → everything needs full depth, premise dead;
// (b) IQR within one grid step → d* effectively constant, the honest outcome
// is a constant repin (or keep), not adaptivity.

/// The dominant [`HEADROOM_DEPTHS`] spacing — kill-gate (b)'s "one grid step".
/// 32→33 and 37→40 are pin-history rungs (Batch 184 / P51a), not design
/// steps: the array was built by appending each production re-pin plus one
/// deeper rung, so only the interior spacing carries the design step.
pub const DEPTH_GRID_STEP: usize = 4;

/// Kill-gate (a) trip bar (Plan 351 T0.1, pre-registered): at least this
/// FRACTION of reachable spans with `d* ≥ 0.75·RERANK_POOL` reads as
/// "everything needs full depth" — the adaptive premise does not transfer.
pub const DEPTH_CENSUS_GATE_A_TRIP: f64 = 0.80;

/// Kill-gate (a)'s "full depth" threshold: `0.75 · RERANK_POOL` = 27.75, so
/// on the grid the qualifying depths are 28, 32, 37, 40.
#[allow(clippy::cast_precision_loss)]
pub const DEPTH_CENSUS_GATE_A_DEPTH: f64 = 0.75 * RERANK_POOL as f64;

/// One oracle-labeled census span (Plan 351 T0.1).
///
/// `oracle_picks` unifies the two label regimes the census measures: the
/// fixture spans carry their single primary label (`{label}` — `acceptable`
/// alternates are NOT credited, the shipped probe's exact-correctness
/// semantics; the field is empty everywhere today), the composite spans carry
/// real clippy's fired set (multi-label by design). An empty set means the
/// span has NO oracle pick — counted separately, never a d\* = ∞ row.
pub struct CensusSpan<'a> {
    /// Span handle — the fixture label or the composite tag.
    pub label: &'a str,
    pub domain: DomainId,
    pub code: &'a str,
    /// Rule ids that count as a CORRECT top-1 answer for this span.
    pub oracle_picks: &'a BTreeSet<String>,
}

/// Per-span per-depth census row (Plan 351 T0.1).
#[derive(Debug, Clone)]
pub struct SpanDepthRow {
    /// Span handle — the fixture label or the composite tag.
    pub label: String,
    pub domain: DomainId,
    /// Was the [`HEADROOM_DEPTHS`]`[i]` top-1 an oracle pick?
    pub top1_oracle_at: [bool; HEADROOM_DEPTHS.len()],
}

impl SpanDepthRow {
    /// d\*(span): the shallowest probed depth whose top-1 is an oracle pick
    /// (the paper's max-μ rule, pinned by Plan 351 T0.1). `None` = the oracle
    /// pick never reaches top-1 at ANY probed depth — unreachable, excluded
    /// from the percentile bins (never a d\* = ∞ row).
    #[must_use]
    pub fn d_star(&self) -> Option<usize> {
        self.top1_oracle_at
            .iter()
            .position(|&hit| hit)
            .map(|i| HEADROOM_DEPTHS[i])
    }
}

/// Plan 351 T0.1 census output over one span population.
#[derive(Debug, Default)]
pub struct DepthCensus {
    /// Spans skipped because their domain's index is feature-off.
    pub skipped: usize,
    /// Spans excluded up front: the oracle set was EMPTY (nothing can be a
    /// correct top-1). "No oracle pick exists" ≠ "the pick never surfaces" —
    /// the two exclusions are never folded together.
    pub excluded_no_oracle: usize,
    /// Every scored span, in input order. Unreachable spans (no depth's
    /// top-1 is an oracle pick) stay here with `d_star() == None`.
    pub rows: Vec<SpanDepthRow>,
}

impl DepthCensus {
    /// Reachable spans (a d\* exists).
    #[must_use]
    pub fn reachable_count(&self) -> usize {
        self.rows.iter().filter(|r| r.d_star().is_some()).count()
    }

    /// Unreachable labels — the oracle pick never reached top-1 at any
    /// probed depth. These are the probe's selection-miss class seen from
    /// the depth axis: even the deepest pool cannot surface the oracle.
    #[must_use]
    pub fn unreachable_labels(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|r| r.d_star().is_none())
            .map(|r| r.label.as_str())
            .collect()
    }

    /// Reachable d\* values, ascending.
    #[must_use]
    pub fn d_stars_sorted(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.rows.iter().filter_map(SpanDepthRow::d_star).collect();
        v.sort_unstable();
        v
    }

    /// Per-grid-index histogram over REACHABLE spans (unreachable excluded,
    /// per the pinned d\* definition).
    #[must_use]
    pub fn histogram(&self) -> [usize; HEADROOM_DEPTHS.len()] {
        let mut h = [0usize; HEADROOM_DEPTHS.len()];
        for i in self.rows.iter().filter_map(SpanDepthRow::d_star) {
            if let Some(slot) = HEADROOM_DEPTHS.iter().position(|&d| d == i) {
                h[slot] += 1;
            }
        }
        h
    }

    /// Linear-interpolation percentile over the reachable d\*s — the
    /// `(n−1)·q` between-closest-ranks method (numpy's `linear`). Never the
    /// `sorted[n·q]` spelling, which lands on the MAX for every
    /// `n ≤ 1/(1−q)` (the percentile-index trap) and would read a p90 of 40
    /// off a histogram whose true tail sits at 28.
    #[must_use]
    pub fn d_percentile(&self, q: f64) -> Option<f64> {
        percentile_sorted(&self.d_stars_sorted(), q)
    }

    /// IQR of the reachable d\*s (`q75 − q25`); `None` when no span is
    /// reachable (statistics undefined — never silently 0).
    #[must_use]
    pub fn d_iqr(&self) -> Option<f64> {
        Some(self.d_percentile(0.75)? - self.d_percentile(0.25)?)
    }

    /// Kill-gate (a): share of REACHABLE spans needing (nearly) full depth.
    /// `None` when no span is reachable.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn gate_a_full_depth_share(&self) -> Option<f64> {
        let reachable = self.reachable_count();
        if reachable == 0 {
            return None;
        }
        let full = self
            .rows
            .iter()
            .filter(|r| {
                r.d_star()
                    .is_some_and(|d| d as f64 >= DEPTH_CENSUS_GATE_A_DEPTH)
            })
            .count();
        Some(full as f64 / reachable as f64)
    }

    /// Kill-gate (b): `Some(true)` = d\* effectively constant (IQR within one
    /// grid step) — adaptivity has nothing to offer. `None` when undefined.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn gate_b_constant(&self) -> Option<bool> {
        self.d_iqr().map(|iqr| iqr <= DEPTH_GRID_STEP as f64)
    }

    /// Merge two census populations (fixture + composite). The label space
    /// is a HANDLE space, not an identity space — the same lint name can
    /// label several fixture spans (measured: `labeled_eval_spans` carries 8
    /// duplicate labels across its 180 rows), and a composite tag is "a human
    /// handle for the report, not a label" (the fixture set's own doc law).
    /// The census unit is the SPAN, so merging is row concatenation + counter
    /// sums — no label-uniqueness assertion exists to violate.
    #[must_use]
    pub fn merged(a: &Self, b: &Self) -> Self {
        Self {
            skipped: a.skipped + b.skipped,
            excluded_no_oracle: a.excluded_no_oracle + b.excluded_no_oracle,
            rows: a
                .rows
                .iter()
                .cloned()
                .chain(b.rows.iter().cloned())
                .collect(),
        }
    }
}

/// Linear-interpolation percentile over ASCENDING integer values — the
/// `(n−1)·q` between-closest-ranks method. `None` on empty input.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
#[must_use]
fn percentile_sorted(sorted: &[usize], q: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    debug_assert!((0.0..=1.0).contains(&q), "q must be a probability");
    let h = (sorted.len() as f64 - 1.0) * q;
    let lo = h.floor() as usize;
    let hi = h.ceil() as usize;
    if lo == hi {
        return Some(sorted[lo] as f64);
    }
    let frac = h - lo as f64;
    Some(sorted[lo] as f64 * (1.0 - frac) + sorted[hi] as f64 * frac)
}

/// Per-depth top-1 oracle hits for ONE span — the census's inner probe. One
/// query per depth: the pool IS the re-rank input, so depth is a
/// configuration, not a slice of one deep list (the probe's own law).
fn census_span_depths(
    matcher: &mut LatentMatcher,
    code: &str,
    domain: DomainId,
    oracle_picks: &BTreeSet<String>,
) -> [bool; HEADROOM_DEPTHS.len()] {
    let mut hits = [false; HEADROOM_DEPTHS.len()];
    for (i, &k) in HEADROOM_DEPTHS.iter().enumerate() {
        let ids = ranked_ids_at(matcher, code, domain, k);
        hits[i] = ids
            .first()
            .is_some_and(|id| oracle_picks.contains(id.as_str()));
    }
    hits
}

/// The Plan 351 T0.1 census over an explicit oracle-labeled span population.
///
/// This is the instrument; assembling the population is the caller's job (the
/// fixture set + composite spans at [`oracle_depth_census`], `fix_verify`
/// real-file rows at T0.2 — the same seam). Spans whose domain index is
/// feature-off are SKIPPED (counted, never scored); spans with an empty
/// oracle set are excluded up front (counted, never scored).
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails.
#[must_use]
pub fn depth_census_with_spans(
    rerank: super::instantiation_rerank::RerankMode,
    spans: &[CensusSpan<'_>],
) -> DepthCensus {
    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(rerank);
    depth_census_with_matcher(&mut matcher, spans)
}

/// [`depth_census_with_spans`] over a caller-built matcher — one index build
/// serves several populations (the index embeds the whole rule corpus; the
/// assembly at [`oracle_depth_census`] runs the fixture + composite
/// populations through ONE build).
///
/// # Panics
///
/// Panics if the matcher carries no enabled domain.
#[must_use]
pub fn depth_census_with_matcher(
    matcher: &mut LatentMatcher,
    spans: &[CensusSpan<'_>],
) -> DepthCensus {
    let mut census = DepthCensus::default();
    for span in spans {
        if !matcher.has_domain(span.domain) {
            census.skipped += 1;
            continue;
        }
        if span.oracle_picks.is_empty() {
            census.excluded_no_oracle += 1;
            continue;
        }
        let top1_oracle_at = census_span_depths(matcher, span.code, span.domain, span.oracle_picks);
        census.rows.push(SpanDepthRow {
            label: span.label.to_string(),
            domain: span.domain,
            top1_oracle_at,
        });
    }
    census
}

/// Plan 351 T0.1's assembled census: the labeled fixture set + the composite
/// set labeled by the real-clippy oracle, both at the shipped re-rank posture,
/// through ONE matcher index build. `clippy_verify`-gated (the oracle lane);
/// the pure instruments ([`depth_census_with_spans`]) are not.
///
/// The composite spans are labeled by [`ClippyOracle::lints_for_batch`] (ONE
/// clippy run over all spans — the batch/single never mix, per the oracle's
/// own cross-module caveat). A span real clippy cannot type-check, or whose
/// verdict did not run, is EXCLUDED and counted — unmeasurable is not a
/// census row.
#[cfg(feature = "clippy_verify")]
#[derive(Debug, Default)]
pub struct OracleDepthCensusReport {
    /// Did the composite oracle run at all (cargo reachable)? `false` → the
    /// composite arm is EMPTY and `composite_oracle`-related counters are 0.
    pub oracle_ran: bool,
    /// Composite spans whose verdict did not run (cargo/I/O failure).
    pub oracle_unavailable: usize,
    /// Composite spans real clippy cannot type-check (no usable labels).
    pub uncompilable: usize,
    /// The labeled fixture population.
    pub labeled: DepthCensus,
    /// The composite population (real-clippy oracle labels).
    pub composite: DepthCensus,
}

/// The full Plan 351 T0.1 census assembly — see [`OracleDepthCensusReport`].
///
/// # Panics
///
/// Panics if [`LatentMatcher::from_enabled_domains`] fails.
#[cfg(feature = "clippy_verify")]
#[must_use]
pub fn oracle_depth_census() -> OracleDepthCensusReport {
    use super::clippy_oracle::ClippyOracle;
    use super::instantiation_rerank::RerankMode;

    // Fixture population: single primary label per span (exact-correctness,
    // the probe's semantics — `acceptable` alternates not credited).
    let labeled_spans = labeled_eval_spans();
    let labeled_picks: Vec<BTreeSet<String>> = labeled_spans
        .iter()
        .map(|s| BTreeSet::from([s.label.clone()]))
        .collect();
    let labeled_census_spans: Vec<CensusSpan<'_>> = labeled_spans
        .iter()
        .zip(&labeled_picks)
        .map(|(s, picks)| CensusSpan {
            label: &s.label,
            domain: s.domain,
            code: &s.code,
            oracle_picks: picks,
        })
        .collect();

    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(RerankMode::default());
    let labeled = depth_census_with_matcher(&mut matcher, &labeled_census_spans);

    let mut report = OracleDepthCensusReport {
        oracle_ran: false,
        oracle_unavailable: 0,
        uncompilable: 0,
        labeled,
        composite: DepthCensus::default(),
    };

    let oracle = ClippyOracle::new();
    if !oracle.is_available() {
        // Composite arm empty; the fixture arm above is still measured.
        return report;
    }
    report.oracle_ran = true;

    let codes: Vec<&str> = super::eval_fixtures_composite::COMPOSITE_SPANS
        .iter()
        .map(|(_, code)| *code)
        .collect();
    let verdicts = oracle.lints_for_batch(&codes);
    let mut picks: Vec<BTreeSet<String>> = Vec::with_capacity(verdicts.len());
    let mut tags: Vec<&str> = Vec::with_capacity(verdicts.len());
    let mut kept_codes: Vec<&str> = Vec::with_capacity(verdicts.len());
    for ((tag, code), verdict) in super::eval_fixtures_composite::COMPOSITE_SPANS
        .iter()
        .zip(&verdicts)
    {
        if !verdict.ran {
            report.oracle_unavailable += 1;
            continue;
        }
        if !verdict.compiles {
            report.uncompilable += 1;
            continue;
        }
        picks.push(verdict.lints.clone());
        tags.push(tag);
        kept_codes.push(code);
    }
    let composite_census_spans: Vec<CensusSpan<'_>> = tags
        .iter()
        .enumerate()
        .map(|(i, tag)| CensusSpan {
            label: tag,
            domain: DomainId::Clippy,
            code: kept_codes[i],
            oracle_picks: &picks[i],
        })
        .collect();
    report.composite = depth_census_with_matcher(&mut matcher, &composite_census_spans);
    report
}

/// Which Phase-0 kill-gates tripped, read over the UNION population (the
/// plan's primary read; per-population rows are the disclosure).
#[cfg(feature = "clippy_verify")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthCensusGates {
    /// Gate (a) — everything needs full depth (≥ 80% at d\* ≥ 0.75·pool).
    pub a_full_depth: bool,
    /// Gate (b) — d\* effectively constant (IQR ≤ one grid step).
    pub b_constant: bool,
}

#[cfg(feature = "clippy_verify")]
impl DepthCensusGates {
    /// Read the gates off the assembled census.
    #[must_use]
    pub fn of(report: &OracleDepthCensusReport) -> Self {
        let union = DepthCensus::merged(&report.labeled, &report.composite);
        Self {
            a_full_depth: union
                .gate_a_full_depth_share()
                .is_some_and(|share| share >= DEPTH_CENSUS_GATE_A_TRIP),
            b_constant: union.gate_b_constant().unwrap_or(false),
        }
    }

    /// Either kill-gate tripped → the adaptive premise is dead (Phase 0
    /// records the negative and stops).
    #[must_use]
    pub fn killed(&self) -> bool {
        self.a_full_depth || self.b_constant
    }
}

/// The `--nocapture` census report: per-population depth statistics + the
/// pre-registered Phase-0 kill-gate verdicts.
#[cfg(feature = "clippy_verify")]
pub fn print_oracle_depth_census(report: &OracleDepthCensusReport) {
    println!(
        "== Plan 351 T0.1 — oracle-depth census (shipped posture, grid {HEADROOM_DEPTHS:?}) =="
    );
    print_depth_census_population("labeled fixture spans", &report.labeled);
    if report.oracle_ran {
        println!(
            "  composite oracle: ran | verdict-unavailable {} | uncompilable {}",
            report.oracle_unavailable, report.uncompilable
        );
        print_depth_census_population("composite spans (real-clippy oracle)", &report.composite);
    } else {
        println!("  composite oracle: UNAVAILABLE — composite arm empty (deferral, not a zero)");
    }
    let union = DepthCensus::merged(&report.labeled, &report.composite);
    print_depth_census_population("UNION (primary gate read)", &union);
    let gates = DepthCensusGates::of(report);
    println!(
        "  PHASE 0 KILL-GATES: (a) full-depth share trip ≥ {:.0} → {} | (b) IQR ≤ {DEPTH_GRID_STEP} → {} | verdict: {}",
        DEPTH_CENSUS_GATE_A_TRIP * 100.0,
        if gates.a_full_depth { "TRIPS" } else { "holds" },
        if gates.b_constant { "TRIPS" } else { "holds" },
        if gates.killed() {
            "KILL — record the negative, CLOSE-NEGATIVE, stop"
        } else {
            "PROCEED — premise alive, T0.2 next"
        }
    );
}

/// One population's census table + its own gate read (disclosure, never the
/// verdict — [`print_oracle_depth_census`] reads the gates on the union).
#[cfg(feature = "clippy_verify")]
fn print_depth_census_population(tag: &str, c: &DepthCensus) {
    let scored = c.rows.len();
    let reachable = c.reachable_count();
    let unreachable = scored - reachable;
    println!("\n  -- {tag} --");
    println!(
        "     scored {scored} | skipped {} | no-oracle excluded {} | reachable {reachable} | unreachable {unreachable}",
        c.skipped, c.excluded_no_oracle,
    );
    let hist = c.histogram();
    let cells: Vec<String> = HEADROOM_DEPTHS
        .iter()
        .zip(hist)
        .map(|(&d, n)| format!("d{d}:{n}"))
        .collect();
    println!("     histogram (reachable): {}", cells.join(" "));
    let (q10, q25, med, q75, q90) = (
        c.d_percentile(0.10),
        c.d_percentile(0.25),
        c.d_percentile(0.50),
        c.d_percentile(0.75),
        c.d_percentile(0.90),
    );
    if let (Some(med), Some(iqr)) = (med, c.d_iqr()) {
        println!(
            "     median {med:.1} | q1 {:.1} | q3 {:.1} | IQR {iqr:.1} | p10 {:.1} | p90 {:.1}  (grid-honest: d* lives on the 10-point grid)",
            q25.unwrap_or(0.0),
            q75.unwrap_or(0.0),
            q10.unwrap_or(0.0),
            q90.unwrap_or(0.0),
        );
    } else {
        println!("     statistics undefined — no reachable span");
    }
    match (c.gate_a_full_depth_share(), c.gate_b_constant()) {
        (Some(share), Some(b)) => {
            println!(
                "     gates (population read): (a) d* ≥ {:.2} → {:.1}% (trip ≥ {:.0}%) {} | (b) {}",
                DEPTH_CENSUS_GATE_A_DEPTH,
                share * 100.0,
                DEPTH_CENSUS_GATE_A_TRIP * 100.0,
                if share >= DEPTH_CENSUS_GATE_A_TRIP {
                    "TRIPS"
                } else {
                    "holds"
                },
                if b {
                    "TRIPS (IQR within one grid step — constant)"
                } else {
                    "holds (IQR above one grid step)"
                },
            );
        }
        _ => println!("     gates: undefined — no reachable span"),
    }
    let misses = c.unreachable_labels();
    if !misses.is_empty() {
        println!(
            "     unreachable (first 12): {}",
            misses[..misses.len().min(12)].join(", ")
        );
    }
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

    /// Plan 351 T0.1: the oracle-depth census (report-only; the kill-gate
    /// verdict is read off the printout and pinned in the plan's status
    /// block — the assertions here are internal consistency, not gates).
    /// Posture: `latent_retrieval,clippy_verify,rust_perf` (clippy_verify
    /// implies clippy_lints). Skips loud when cargo is unavailable.
    #[cfg(feature = "clippy_verify")]
    #[test]
    fn t0_oracle_depth_census_report() {
        let report = oracle_depth_census();
        print_oracle_depth_census(&report);

        // Internal consistency — the fixture arm must score exactly the
        // fixture set (skipped = domain feature-off, never silently dropped).
        assert_eq!(
            report.labeled.rows.len() + report.labeled.skipped,
            labeled_eval_spans().len(),
            "fixture census must cover exactly the fixture set"
        );
        if report.oracle_ran {
            assert_eq!(
                report.composite.rows.len() + report.oracle_unavailable + report.uncompilable,
                crate::draft::eval_fixtures_composite::COMPOSITE_SPANS.len(),
                "composite census must account for every composite span"
            );
        } else {
            println!("  (composite arm skipped — cargo unavailable; deferral, not a zero)");
        }
    }
}

/// Plan 351 T0.1's statistics on synthetic rows — no matcher, no oracle, so
/// these run at every latent_retrieval test posture and pin the ARITHMETIC
/// (d* selection, the (n−1)·q percentile method, both kill-gates) before any
/// measurement runs.
#[cfg(test)]
mod depth_census_stats {
    use super::*;

    fn row(label: &str, hits: &[bool]) -> SpanDepthRow {
        let mut top1_oracle_at = [false; HEADROOM_DEPTHS.len()];
        for (i, &hit) in hits.iter().enumerate() {
            top1_oracle_at[i] = hit;
        }
        SpanDepthRow {
            label: label.to_string(),
            domain: DomainId::Clippy,
            top1_oracle_at,
        }
    }

    fn census_of(rows: Vec<SpanDepthRow>) -> DepthCensus {
        DepthCensus {
            skipped: 0,
            excluded_no_oracle: 0,
            rows,
        }
    }

    #[test]
    fn d_star_is_the_shallowest_oracle_depth() {
        // hits at grid idx 1 (12) and idx 5 (24) → d* = 12.
        let r = row(
            "a",
            &[
                false, true, false, false, true, false, false, false, false, false,
            ],
        );
        assert_eq!(r.d_star(), Some(12));

        // hits only at the deepest rung → d* = 40.
        let r = row(
            "b",
            &[
                false, false, false, false, false, false, false, false, false, true,
            ],
        );
        assert_eq!(r.d_star(), Some(40));

        // no depth surfaces the oracle → unreachable (None, never ∞).
        let r = row("c", &[false; HEADROOM_DEPTHS.len()]);
        assert_eq!(r.d_star(), None);
    }

    #[test]
    fn percentile_is_linear_interpolation_between_closest_ranks() {
        let sorted = [8usize, 12, 16, 37];
        assert_eq!(percentile_sorted(&sorted, 0.25), Some(11.0)); // h=0.75
        assert_eq!(percentile_sorted(&sorted, 0.50), Some(14.0)); // h=1.5
        assert_eq!(percentile_sorted(&sorted, 0.75), Some(21.25)); // h=2.25
        assert_eq!(percentile_sorted(&sorted, 0.0), Some(8.0));
        assert_eq!(percentile_sorted(&sorted, 1.0), Some(37.0));
        // The percentile-index trap: p90 on n=2 must interpolate, not clamp
        // to the max (sorted[n·q] would read 40 at q=0.9, n=2).
        let two = [8usize, 40];
        let p90 = percentile_sorted(&two, 0.9).expect("non-empty");
        assert!((p90 - 36.8).abs() < 1e-9, "p90 must interpolate: {p90}");
        assert_eq!(percentile_sorted(&[], 0.5), None);
        let single = [12usize];
        assert_eq!(percentile_sorted(&single, 0.95), Some(12.0));
    }

    #[test]
    fn census_stats_and_gates_on_a_synthetic_population() {
        // d*s: 8, 12, 16, 37 → reachable 4, histogram d8:1 d12:1 d16:1 d37:1.
        let c = census_of(vec![
            row(
                "a",
                &[
                    true, false, false, false, false, false, false, false, false, false,
                ],
            ),
            row(
                "b",
                &[
                    false, true, false, false, false, false, false, false, false, false,
                ],
            ),
            row(
                "c",
                &[
                    false, false, true, false, false, false, false, false, false, false,
                ],
            ),
            row(
                "d",
                &[
                    false, false, false, false, false, false, false, false, true, false,
                ],
            ),
            row("u", &[false; HEADROOM_DEPTHS.len()]), // unreachable
        ]);
        assert_eq!(c.reachable_count(), 4);
        assert_eq!(c.unreachable_labels(), ["u"]);
        let hist = c.histogram();
        assert_eq!(hist, [1, 1, 1, 0, 0, 0, 0, 0, 1, 0]);
        assert_eq!(c.d_percentile(0.5), Some(14.0));
        assert_eq!(c.d_iqr(), Some(10.25));
        // Gate (a): only d*=37 clears 27.75 → 1/4 = 0.25 < 0.80 → holds.
        assert_eq!(c.gate_a_full_depth_share(), Some(0.25));
        // Gate (b): IQR 10.25 > one grid step (4) → holds (not constant).
        assert_eq!(c.gate_b_constant(), Some(false));
    }

    #[test]
    fn gate_boundaries_are_pinned() {
        // Gate (a) boundary: d*=28 clears 27.75; d*=24 does not.
        let at28 = census_of(vec![
            row(
                "a",
                &[
                    false, false, false, false, false, true, false, false, false, false,
                ],
            ),
            row(
                "b",
                &[
                    false, false, false, false, false, false, true, false, false, false,
                ],
            ),
        ]);
        assert_eq!(at28.gate_a_full_depth_share(), Some(1.0)); // both ≥ 27.75 → trips
        let at24 = census_of(vec![
            row(
                "a",
                &[
                    false, false, false, false, true, false, false, false, false, false,
                ],
            ),
            row(
                "b",
                &[
                    false, false, false, false, false, true, false, false, false, false,
                ],
            ),
        ]);
        assert!((at24.gate_a_full_depth_share().unwrap() - 0.5).abs() < 1e-9);
        // Gate (b) boundary: IQR exactly one grid step (4) IS "constant".
        // d*s 8,12,16,20: q25 = h=0.75 → 8+0.75·4 = 11; q75 = h=2.25 → 16+0.25·4 = 17 → IQR 6.
        // d*s 8,12 only: q25 = h=0.25 → 9; q75 = h=0.75 → 11 → IQR 2 ≤ 4 → trips.
        let tight = census_of(vec![
            row(
                "a",
                &[
                    true, false, false, false, false, false, false, false, false, false,
                ],
            ),
            row(
                "b",
                &[
                    false, true, false, false, false, false, false, false, false, false,
                ],
            ),
        ]);
        assert_eq!(tight.d_iqr(), Some(2.0));
        assert_eq!(tight.gate_b_constant(), Some(true));
        // Empty-oracle exclusions count, never score.
        let c = census_of(vec![]);
        assert_eq!(c.gate_a_full_depth_share(), None);
        assert_eq!(c.gate_b_constant(), None);
    }

    #[test]
    fn merged_census_tolerates_duplicate_handle_labels() {
        // Labels are HANDLES (the same lint name can label several spans —
        // measured: labeled_eval_spans carries 8 duplicate labels), so the
        // merge is row concatenation: the census unit is the span.
        let a = census_of(vec![row(
            "lint_x",
            &[
                true, false, false, false, false, false, false, false, false, false,
            ],
        )]);
        let b = census_of(vec![
            row(
                "composite_tag",
                &[
                    false, true, false, false, false, false, false, false, false, false,
                ],
            ),
            row(
                "lint_x",
                &[
                    false, false, false, false, false, false, false, false, false, true,
                ],
            ),
        ]);
        let m = DepthCensus::merged(&a, &b);
        assert_eq!(m.rows.len(), 3, "every span merges as its own row");
        assert_eq!(m.skipped, 0);
        let (sa, sb) = (
            DepthCensus {
                skipped: 2,
                ..Default::default()
            },
            DepthCensus {
                excluded_no_oracle: 5,
                ..Default::default()
            },
        );
        let m = DepthCensus::merged(&sa, &sb);
        assert_eq!(
            (m.skipped, m.excluded_no_oracle),
            (2, 5),
            "counters sum across populations"
        );
    }
}

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
//!    ([`HEADROOM_DEPTHS`] = 8 / 12 / 16)?
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
//! a depth-16 probe can promote a rule that the depth-12 production path never
//! saw. Each depth is therefore measured by its own query rather than by
//! slicing one deep list — which is also why `recall@8` can exceed nothing and
//! `recall@16` is not automatically ≥ `recall@12`.
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
use super::retrieval_eval::{canonical_rank, domain_index, labeled_eval_spans};

/// Candidate-pool depths probed, ascending. `RERANK_POOL` (the production
/// depth) must be one of them — it is the recall@1 reference point.
pub const HEADROOM_DEPTHS: [usize; 3] = [8, 12, 16];

/// Index of the production depth within [`HEADROOM_DEPTHS`].
pub const PROD_DEPTH_IDX: usize = 1;

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
pub fn run_headroom_probe_with(
    rerank: super::instantiation_rerank::RerankMode,
) -> HeadroomReport {
    const _: () = assert!(
        HEADROOM_DEPTHS[PROD_DEPTH_IDX] == RERANK_POOL,
        "the production depth must be one of the probed depths"
    );

    let mut matcher = LatentMatcher::from_enabled_domains()
        .expect("build matcher")
        .with_rerank_mode(rerank);
    let mut report = HeadroomReport::default();

    for span in labeled_eval_spans() {
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
            m.headroom_pp().map_or("-".to_string(), |pp| format!("{pp:+.1}pp"))
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

#[cfg(test)]
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

    /// The probe is a MEASUREMENT: two runs in one process must agree, or the
    /// headroom number cannot be compared across commits.
    #[test]
    fn t1_headroom_probe_is_deterministic() {
        let a = run_headroom_probe();
        let b = run_headroom_probe();
        assert_eq!(a.blended, b.blended, "blended metrics must be deterministic");
        assert_eq!(
            a.misses.iter().map(|m| (&m.expected, m.rank, m.kind)).collect::<Vec<_>>(),
            b.misses.iter().map(|m| (&m.expected, m.rank, m.kind)).collect::<Vec<_>>(),
            "miss taxonomy must be deterministic"
        );
    }
}

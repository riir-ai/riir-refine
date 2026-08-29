//! Issue 024 — cross-domain switch-cost measurement (the retrieval axis).
//!
//! Research 484's headline failure class, asked of OUR healer: does an
//! anti-pattern retrieve its rule worse when a different domain's
//! anti-pattern is co-present in the span than in isolation?
//!
//! ## Method (paired, fresh-baseline)
//!
//! For each [`CROSS_PAIRS`] pair, both textual orders are evaluated
//! (clippy-first and perf-first — the DIRECTED axis; collapsing to
//! symmetric is the easiest mistake per Research 484 §6.3):
//!
//! - **solo**: each constituent code alone, ranked in its own domain;
//! - **co-present**: the merged span, the SAME rule ranked in the SAME
//!   domain's list.
//!
//! Because the solo baseline is measured in the same process against the
//! same corpus (NOT read off the historical Issue 017 numbers), the paired
//! delta is immune to corpus drift. The second-position rule is the
//! "switching" one (the paper's step-2 analog — failures concentrate at
//! the switch); the first-position rule is the lead control.
//!
//! Direction convention: `clippy→perf` means the clippy anti-pattern is
//! textually FIRST and the perf rule must switch IN; `perf→clippy` is the
//! mirror.
//!
//! ## Verdict criterion (Issue 024 T4)
//!
//! Signal = any direction whose solo−co hit@k gap ≥ 5pp on the switching
//! rule. At n=12 per direction one span is 8.3pp, so the raw per-span drop
//! detail is printed alongside — a one-span borderline drop and a
//! consistent multi-span drop read very differently at this granularity.

use super::domain_router::DomainId;
use super::eval_fixtures_clippy::CLIPPY_SPANS;
use super::eval_fixtures_cross_domain::CROSS_PAIRS;
use super::eval_fixtures_perf::PERF_SPANS;
use super::latent_matcher::LatentMatcher;

/// Probe depth — matches the Issue 017 eval (ranks beyond 8 are censored
/// to `None`, which counts as a miss at every k).
const PROBE: usize = 8;

/// One merged span: a pair in one textual order.
pub struct CrossSpan {
    /// `"{lint}+{rule}/cf|pf"` — cf = clippy-first, pf = perf-first.
    pub tag: String,
    pub clippy_lint: &'static str,
    pub perf_rule: &'static str,
    /// Which domain's constituent is textually first (the directed axis).
    pub first: DomainId,
    /// The merged code (clippy fixture + blank line + perf fixture, in
    /// `first` order).
    pub code: String,
}

fn clippy_fixture(lint: &str) -> &'static str {
    CLIPPY_SPANS
        .iter()
        .find(|(l, _)| *l == lint)
        .map_or_else(|| panic!("no CLIPPY_SPANS fixture for {lint}"), |(_, c)| *c)
}

fn perf_fixture(rule: &str) -> &'static str {
    PERF_SPANS
        .iter()
        .find(|(r, _)| *r == rule)
        .map_or_else(|| panic!("no PERF_SPANS fixture for {rule}"), |(_, c)| *c)
}

/// The 24 spans (12 pairs × both orders). Deterministic; constituent codes
/// are the exact solo fixture texts.
#[must_use]
pub fn cross_domain_spans() -> Vec<CrossSpan> {
    let mut out = Vec::with_capacity(CROSS_PAIRS.len() * 2);
    for &(lint, rule) in CROSS_PAIRS {
        let cc = clippy_fixture(lint);
        let pc = perf_fixture(rule);
        out.push(CrossSpan {
            tag: format!("{lint}+{rule}/cf"),
            clippy_lint: lint,
            perf_rule: rule,
            first: DomainId::Clippy,
            code: format!("{cc}\n{pc}"),
        });
        out.push(CrossSpan {
            tag: format!("{lint}+{rule}/pf"),
            clippy_lint: lint,
            perf_rule: rule,
            first: DomainId::Perf,
            code: format!("{pc}\n{cc}"),
        });
    }
    out
}

/// Hit counters at k = 1/3/5 over `n` ranked spans.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Hits3 {
    pub n: usize,
    pub at1: usize,
    pub at3: usize,
    pub at5: usize,
}

impl Hits3 {
    fn record(&mut self, rank: Option<usize>) {
        self.n += 1;
        if rank.is_some_and(|r| r < 1) {
            self.at1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            self.at3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            self.at5 += 1;
        }
    }

    #[allow(clippy::cast_precision_loss)] // display metric over ≤24 spans
    fn pct(&self, k: usize) -> f32 {
        let hits = match k {
            1 => self.at1,
            3 => self.at3,
            _ => self.at5,
        };
        if self.n == 0 {
            return 0.0;
        }
        hits as f32 / self.n as f32 * 100.0
    }
}

/// One directed order's metrics: the switching (second-position) rule and
/// the lead (first-position) control, each solo vs co-present.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DirectedMetrics {
    pub switch_solo: Hits3,
    pub switch_co: Hits3,
    pub lead_solo: Hits3,
    pub lead_co: Hits3,
}

impl DirectedMetrics {
    /// The switching rule's solo−co gap in percentage points at k=1/3/5.
    #[allow(clippy::cast_precision_loss)] // display metric over ≤12 spans
    fn switch_gaps_pp(&self) -> [f32; 3] {
        let mut out = [0.0f32; 3];
        for (i, k) in [1usize, 3, 5].iter().enumerate() {
            out[i] = self.switch_solo.pct(*k) - self.switch_co.pct(*k);
        }
        out
    }
}

/// A span+rule whose co-present rank is worse than its solo rank (the
/// per-switch-cost unit — the aggregate table alone hides which rules
/// moved).
#[derive(Debug, Clone)]
pub struct DropRecord {
    pub tag: String,
    pub rule: String,
    pub role: &'static str,
    pub first: DomainId,
    pub solo_rank: Option<usize>,
    pub co_rank: Option<usize>,
}

/// The full switch-cost report.
#[derive(Debug, Default)]
pub struct SwitchCostReport {
    pub spans: usize,
    /// clippy textually first; the PERF rule switches in.
    pub clippy_to_perf: DirectedMetrics,
    /// perf textually first; the CLIPPY rule switches in.
    pub perf_to_clippy: DirectedMetrics,
    /// Every rule whose co-present rank degraded (switch + lead roles).
    pub drops: Vec<DropRecord>,
}

fn rank_of(
    matcher: &mut LatentMatcher,
    code: &str,
    domain: DomainId,
    rule_id: &str,
) -> Option<usize> {
    let (_, by_domain) = matcher.suggest_span(code, PROBE);
    let ranked = by_domain
        .iter()
        .find(|(d, _)| *d == domain)
        .map(|(_, rules)| super::retrieval_eval::canonical_rank(rules))?;
    ranked.iter().position(|r| r.rule_id == rule_id)
}

fn worse(co: Option<usize>, solo: Option<usize>) -> bool {
    match (co, solo) {
        (Some(c), Some(s)) => c > s,
        (None, Some(_)) => true,
        _ => false,
    }
}

/// Run the paired measurement. `None` when either domain's index is
/// feature-off (the measurement needs both).
#[must_use]
pub fn run_switch_cost_eval() -> Option<SwitchCostReport> {
    let mut matcher = LatentMatcher::from_enabled_domains().ok()?;
    if !matcher.has_domain(DomainId::Clippy) || !matcher.has_domain(DomainId::Perf) {
        return None;
    }
    let mut report = SwitchCostReport::default();
    for span in cross_domain_spans() {
        report.spans += 1;
        // The switching rule is the second-position domain's rule; the lead
        // is the first-position's.
        let (switch_domain, switch_rule, switch_solo_code) = match span.first {
            DomainId::Clippy => {
                (DomainId::Perf, span.perf_rule, perf_fixture(span.perf_rule))
            }
            _ => (
                DomainId::Clippy,
                span.clippy_lint,
                clippy_fixture(span.clippy_lint),
            ),
        };
        let (lead_domain, lead_rule, lead_solo_code) = match span.first {
            DomainId::Clippy => (
                DomainId::Clippy,
                span.clippy_lint,
                clippy_fixture(span.clippy_lint),
            ),
            _ => (DomainId::Perf, span.perf_rule, perf_fixture(span.perf_rule)),
        };

        let r_switch_solo = rank_of(&mut matcher, switch_solo_code, switch_domain, switch_rule);
        let r_switch_co = rank_of(&mut matcher, &span.code, switch_domain, switch_rule);
        let r_lead_solo = rank_of(&mut matcher, lead_solo_code, lead_domain, lead_rule);
        let r_lead_co = rank_of(&mut matcher, &span.code, lead_domain, lead_rule);

        let dm = match span.first {
            DomainId::Clippy => &mut report.clippy_to_perf,
            _ => &mut report.perf_to_clippy,
        };
        dm.switch_solo.record(r_switch_solo);
        dm.switch_co.record(r_switch_co);
        dm.lead_solo.record(r_lead_solo);
        dm.lead_co.record(r_lead_co);

        if worse(r_switch_co, r_switch_solo) {
            report.drops.push(DropRecord {
                tag: span.tag.clone(),
                rule: switch_rule.to_string(),
                role: "switch",
                first: span.first,
                solo_rank: r_switch_solo,
                co_rank: r_switch_co,
            });
        }
        if worse(r_lead_co, r_lead_solo) {
            report.drops.push(DropRecord {
                tag: span.tag,
                rule: lead_rule.to_string(),
                role: "lead",
                first: span.first,
                solo_rank: r_lead_solo,
                co_rank: r_lead_co,
            });
        }
    }
    Some(report)
}

/// The `--nocapture` directed table.
pub fn print_switch_cost_report(report: &SwitchCostReport) {
    println!("== Issue 024 cross-domain switch cost (retrieval axis) ==");
    println!(
        "pairs: {} × 2 orders = {} spans; probe top-{PROBE}",
        CROSS_PAIRS.len(),
        report.spans
    );
    for (name, dm) in [
        ("clippy→perf (perf switches in)", &report.clippy_to_perf),
        ("perf→clippy (clippy switches in)", &report.perf_to_clippy),
    ] {
        let g = dm.switch_gaps_pp();
        println!(
            "\n{name}:\n  \
             switch solo @1/3/5: {:>5.1}/{:>5.1}/{:>5.1}%  (n={})\n  \
             switch co   @1/3/5: {:>5.1}/{:>5.1}/{:>5.1}%\n  \
             switch Δ pp       : {:+>5.1}/{:+>5.1}/{:+>5.1}  (solo − co)\n  \
             lead   co   @1/3/5: {:>5.1}/{:>5.1}/{:>5.1}%  (control; solo {:>5.1}/{:>5.1}/{:>5.1})",
            dm.switch_solo.pct(1), dm.switch_solo.pct(3), dm.switch_solo.pct(5), dm.switch_solo.n,
            dm.switch_co.pct(1), dm.switch_co.pct(3), dm.switch_co.pct(5),
            g[0], g[1], g[2],
            dm.lead_co.pct(1), dm.lead_co.pct(3), dm.lead_co.pct(5),
            dm.lead_solo.pct(1), dm.lead_solo.pct(3), dm.lead_solo.pct(5),
        );
    }
    if report.drops.is_empty() {
        println!("\ndrops: none — no rule's rank degraded under co-presence");
    } else {
        println!("\ndrops (co worse than solo; rank None = outside top-{PROBE}):");
        for d in &report.drops {
            println!(
                "  {:<44} {:>6} solo={:?} co={:?}",
                d.tag, d.role, d.solo_rank, d.co_rank
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both orders of every pair exist, carry both constituents verbatim,
    /// and the set is deterministic.
    #[test]
    fn cross_domain_spans_shape() {
        let spans = cross_domain_spans();
        assert_eq!(spans.len(), CROSS_PAIRS.len() * 2);
        let cf = spans.iter().filter(|s| s.first == DomainId::Clippy).count();
        let pf = spans.iter().filter(|s| s.first == DomainId::Perf).count();
        assert_eq!(cf, CROSS_PAIRS.len());
        assert_eq!(pf, CROSS_PAIRS.len());
        for s in &spans {
            assert!(
                s.code.contains(clippy_fixture(s.clippy_lint).trim()),
                "{} missing clippy constituent",
                s.tag
            );
            assert!(
                s.code.contains(perf_fixture(s.perf_rule).trim()),
                "{} missing perf constituent",
                s.tag
            );
        }
        assert_eq!(cross_domain_spans().len(), spans.len());
    }

    /// Label truth, cross-domain extended: on the MERGED span the labeled
    /// clippy lint's proposer fires and no OTHER structural corpus lint
    /// does (the perf shape must not carry a clippy anti-pattern); the
    /// labeled perf rule's proposer fires when proposer-covered and no
    /// other perf proposer does. Checked in BOTH orders — order must not
    /// change what fires.
    #[cfg(all(feature = "clippy_lints", feature = "rust_perf"))]
    #[test]
    fn cross_domain_fixture_labels_verify_via_proposers() {
        use crate::domains::clippy_lints::proposers::{propose_fix, FIXABLE_LINTS, VERIFY_LINTS};
        use crate::domains::rust_perf::proposers as perf;

        let structural: Vec<&str> = FIXABLE_LINTS
            .iter()
            .chain(VERIFY_LINTS)
            .copied()
            .collect();
        for s in cross_domain_spans() {
            assert!(
                propose_fix(s.clippy_lint, &s.code).is_some(),
                "{}: labeled clippy lint {} does not fire its proposer",
                s.tag,
                s.clippy_lint
            );
            for &other in &structural {
                if other != s.clippy_lint {
                    assert!(
                        propose_fix(other, &s.code).is_none(),
                        "{}: perf constituent carries clippy lint {other}",
                        s.tag
                    );
                }
            }
            if perf::FIXABLE_RULES.contains(&s.perf_rule) {
                assert!(
                    perf::propose_fix(s.perf_rule, &s.code).is_some(),
                    "{}: labeled perf rule {} does not fire its proposer",
                    s.tag,
                    s.perf_rule
                );
            }
            for &other in perf::FIXABLE_RULES {
                if other != s.perf_rule {
                    assert!(
                        perf::propose_fix(other, &s.code).is_none(),
                        "{}: clippy constituent carries perf rule {other}",
                        s.tag
                    );
                }
            }
        }
    }

    /// The pure constituents are also cross-clean solo (cheap double-check
    /// of the pair-selection rules at the fixture level).
    ///
    /// Issue 056: this used to iterate only the 12 `CROSS_PAIRS`
    /// constituents — a SAMPLED invariant. Two PERF_SPANS fixtures carried
    /// `v % 2 == 0` (`manual_is_multiple_of`) indefinitely with every test
    /// green, because neither rule appeared in a pair. Purity is asserted
    /// over EVERY entry of both consts now: a perf fixture may carry no
    /// clippy proposer, a clippy fixture no perf proposer. The pair
    /// constituents are a subset, so they stay covered.
    #[cfg(all(feature = "clippy_lints", feature = "rust_perf"))]
    #[test]
    fn cross_domain_constituents_carry_no_cross_domain_proposer() {
        use crate::domains::clippy_lints::proposers::{propose_fix, FIXABLE_LINTS, VERIFY_LINTS};
        use crate::domains::rust_perf::proposers as perf;

        let structural: Vec<&str> = FIXABLE_LINTS
            .iter()
            .chain(VERIFY_LINTS)
            .copied()
            .collect();
        for &(rule, code) in PERF_SPANS {
            for &l in &structural {
                assert!(
                    propose_fix(l, code).is_none(),
                    "perf fixture {rule} carries clippy lint {l}"
                );
            }
        }
        for &(lint, code) in CLIPPY_SPANS {
            for &r in perf::FIXABLE_RULES {
                assert!(
                    perf::propose_fix(r, code).is_none(),
                    "clippy fixture {lint} carries perf rule {r}"
                );
            }
        }
    }

    /// The measurement itself (print with `--nocapture`). No floor is
    /// asserted — this is a one-time measurement whose verdict is recorded
    /// in the benchmark doc, not a regression gate.
    #[test]
    fn switch_cost_report() {
        let Some(report) = run_switch_cost_eval() else {
            println!("switch-cost eval skipped (a domain index is feature-off)");
            return;
        };
        print_switch_cost_report(&report);
        assert_eq!(report.spans, CROSS_PAIRS.len() * 2);
        assert_eq!(report.clippy_to_perf.switch_solo.n, CROSS_PAIRS.len());
        assert_eq!(report.perf_to_clippy.switch_solo.n, CROSS_PAIRS.len());
        // Determinism (same contract as the Issue 017 eval).
        let again = run_switch_cost_eval().expect("second run");
        assert_eq!(report.clippy_to_perf, again.clippy_to_perf);
        assert_eq!(report.perf_to_clippy, again.perf_to_clippy);
        assert_eq!(report.drops.len(), again.drops.len());
    }

    /// T4 mitigation guard — the measured casualty stays rescued.
    ///
    /// Issue 024 measured exactly one switch-cost casualty:
    /// `needless_return` merged with `vec-with-capacity` (both orders) had
    /// its raw similarity rank crowded to **9** by loop-shaped clippy rules
    /// scoring 3× its solo winning score — one position past the then-pool
    /// of 8, evicting it before the instantiation re-ranker ever saw it
    /// (the re-ranker can only promote what was retrieved). Raising
    /// [`RERANK_POOL`](super::super::instantiation_rerank::RERANK_POOL) to
    /// 12 was the mitigation; this test pins it: at the eval's probe depth
    /// (`PROBE` = 8, pool = `max(8, RERANK_POOL)`) the casualty must sit at
    /// rank 0 in BOTH textual orders.
    ///
    /// The discovery loop is kept for future casualties: any span whose
    /// clippy rule is missing at every pool depth up to 40 would print a
    /// `NOT rescued` line — buried beyond the corpus retrieval horizon, a
    /// crowding signal this guard cannot see. Print with `--nocapture`.
    #[test]
    fn pool_depth_rescue_guard() {
        use super::super::instantiation_rerank::RERANK_POOL;
        let Ok(mut matcher) = LatentMatcher::from_enabled_domains() else {
            println!("pool-depth guard skipped (no domain index)");
            return;
        };
        if !matcher.has_domain(DomainId::Clippy) {
            println!("pool-depth guard skipped (clippy index feature-off)");
            return;
        }
        for span in cross_domain_spans() {
            if span.clippy_lint != "needless_return" {
                continue;
            }
            // The eval condition that failed pre-mitigation.
            let (_, by_domain) = matcher.suggest_span(&span.code, PROBE);
            let rank = by_domain
                .iter()
                .find(|(d, _)| *d == DomainId::Clippy)
                .and_then(|(_, rules)| {
                    crate::draft::retrieval_eval::canonical_rank(rules)
                        .iter()
                        .position(|r| r.rule_id == span.clippy_lint)
                });
            assert_eq!(
                rank,
                Some(0),
                "{}: the measured casualty must be rescued at eval probe depth {PROBE} \
                 (raw rank 9; pool floor {RERANK_POOL}) — if this fires, retrieval crowding \
                 has pushed the casualty past RERANK_POOL again",
                span.tag
            );
            println!(
                "{}: rescued at rank {rank:?} (eval probe {PROBE}, pool floor {RERANK_POOL})",
                span.tag
            );
            // Discovery sweep for FUTURE casualties at other depths.
            let mut deep = None;
            for k in RERANK_POOL..=40usize {
                let (_, by_domain) = matcher.suggest_span(&span.code, k);
                if by_domain
                    .iter()
                    .any(|(_, rules)| rules.iter().any(|r| r.rule_id == span.clippy_lint))
                {
                    deep = Some(k);
                    break;
                }
            }
            if deep.is_none() {
                println!(
                    "{}: NOT found at any pool depth up to 40 — buried beyond the corpus retrieval horizon",
                    span.tag
                );
            }
        }
    }
}

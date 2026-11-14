//! Issue 237 — the composition-descent cell: the rating-energized
//! candidate landscape over the fixpoint's rule composition.
//!
//! The fixpoint composes edits from EVERY vocabulary rule each pass
//! (compose-all). CRAM's shape (katgpt-rs Research 670 / arXiv:2611.17309,
//! distilled onto `katgpt-assign` as Issue 1211's `descent`) adds what
//! compose-all cannot produce: a composition POLICY over the rule set —
//! ranked by the rules' ratings (Beta belief over the run's own per-rule
//! attribution → `B_i`), stopping at a fitted level, abstaining where no
//! evidence clears it. The lane never adds oracle calls — the landscape is
//! pinned estimates, so the verify budget (1 before + 1 after verdict per
//! fixture) is IDENTICAL across arms by construction; the stop's claim on
//! this surface is compute-shape (abstain/budget), never accuracy (the
//! compile VERIFY step owns correctness — the P104 posture).
//!
//! ## Arms (pre-registered in `.issues/237`; per-family, never pooled)
//!
//! - **Arm a — `LaneMode::All`** (the incumbent): every rule's edits, the
//!   frozen order. Byte-identical to today's bench.
//! - **Arm b — `LaneMode::Ranked`**: ordering only — the fixture's fired
//!   rules ranked by `B_i` (desc, name asc). Isolates the ORDERING axis
//!   under the 4-pass cap (the ablation's ordering half).
//! - **Arm c — `LaneMode::Subset`**: the full shape — ranked, plus the
//!   fitted level (rules below it abstain). A rule with no attribution
//!   data (it never ran a single-target fixture) has NO evidence and
//!   abstains in both descent arms — the stack's no-evidence convention
//!   (`belief(0,0)` is neutral but the LEVEL fit excludes it, so an
//!   unevidenced rule never clears a fitted level).
//! - **Falsification arm** — `LaneMode::Subset` at an absurd level: every
//!   plan is empty, every fixture resolves nothing — a green run that
//!   emits ANY resolution FAILS the cell (the never-wrong row, mirroring
//!   katgpt-rs Issue 1211's falsification arm).
//!
//! **`K_ij` = 0 in T1** (disclosed): the pair-cost axis (co-outcome
//! statistics from the trajectory store) is deferred; the ablation arms
//! are exactly the ordering-vs-subset disentanglement the issue names as
//! confound 1. `B_i` is derived from arm a's OWN attribution on the same
//! frozen set — the honest calibration procedure (calibrate on the
//! incumbent's data, test the policy), disclosed in the report.
//!
//! All plans are deterministic: `f32::total_cmp` descending + rule-name
//! ascending — no RNG anywhere in the lane.

use std::collections::BTreeMap;

use super::{BenchFixture, BenchScore, rule_outcomes, run_bench_lane};
use crate::draft::clippy_oracle::ClippyOracle;

/// The pipeline's own belief-quantile convention (`BELIEF_EPSILON` in
/// `self_evolve` — 0.05, matching the conformal layer's 95% convention).
const BELIEF_EPSILON: f32 = 0.05;

/// Per-rule rating table: rule → belief score (Beta lower bound over the
/// compose-all run's single-target attribution: resolved vs persist +
/// broken). Rules without attribution rows are ABSENT — no evidence.
pub type Beliefs = BTreeMap<String, f32>;

/// The composition lane for one bench run (the policy is per-rule GLOBAL
/// — `B_i` from corpus-wide attribution; per-fixture adaptation is the
/// `K_ij` axis, deferred). All fixtures share the plan.
#[derive(Debug, Clone)]
pub enum LaneMode {
    /// The incumbent: every rule's edits, the frozen order.
    All,
    /// Ordering only: ALL rules still apply — evidenced rules ranked by
    /// `B_i` (desc, name asc) first, then the un-evidenced remainder in the
    /// frozen order. Membership unchanged; the ORDER axis isolated.
    Ranked { beliefs: Beliefs, level: f32 },
    /// The fitted level's subset: only evidenced rules AT/ABOVE the level
    /// apply (below-level and no-evidence rules abstain). Implies its own
    /// ranking. Membership changes; the LEVEL axis isolated.
    Subset { beliefs: Beliefs, level: f32 },
}

/// Per-rule belief attribution from a compose-all run (the calibration
/// step). Single-target rows only — `rule_outcomes` already excludes
/// multi-target fixtures (recorded, never guessed); a rule absent there
/// stays absent here (no evidence).
#[must_use]
pub fn derive_beliefs(score: &BenchScore) -> Beliefs {
    let mut out = Beliefs::new();
    for r in rule_outcomes(score).rows {
        let unresolved = r.persist + r.broken;
        let resolved = u32::try_from(r.resolved).unwrap_or(u32::MAX);
        let unresolved = u32::try_from(unresolved).unwrap_or(u32::MAX);
        out.insert(
            r.rule,
            katgpt_core::best_belief_score(resolved, unresolved, BELIEF_EPSILON),
        );
    }
    out
}

/// The fitted level: the median belief over rules WITH data (average of
/// the two middle values at even count). `None` when no rule has data —
/// the caller REFUSES the descent arms loudly (nothing to fit against is
/// not a pass).
#[must_use]
pub fn fit_level(beliefs: &Beliefs) -> Option<f32> {
    let mut bs: Vec<f32> = beliefs.values().copied().collect();
    if bs.is_empty() {
        return None;
    }
    bs.sort_by(f32::total_cmp);
    let mid = bs.len() / 2;
    Some(if bs.len() % 2 == 1 {
        bs[mid]
    } else {
        f32::midpoint(bs[mid - 1], bs[mid])
    })
}

/// The composition plan: the clippy-lane rules whose edits this pass
/// applies, in application order — computed ONCE per run (the policy is
/// per-rule global; `B_i` is corpus-wide). `None` = the incumbent lane
/// (every rule, frozen order — byte-identical). An EMPTY plan is the
/// ABSTAIN posture: no clippy-lane edits apply at all. `all_rules` is the
/// frozen rule universe (the membership base both arms permute/filter).
#[must_use]
pub fn plan_fixture(mode: &LaneMode, all_rules: &[&str]) -> Option<Vec<String>> {
    let (beliefs, level, subset) = match mode {
        LaneMode::All => return None,
        LaneMode::Ranked { beliefs, level } => (beliefs, *level, false),
        LaneMode::Subset { beliefs, level } => (beliefs, *level, true),
    };
    // Evidenced rules ranked `B_i` descending (name ascending — `total_cmp`
    // keeps the order total); the un-evidenced remainder trails in frozen
    // order. Ranked applies everyone; Subset keeps only the level-clearers.
    let mut evidenced: Vec<(String, f32)> = all_rules
        .iter()
        .filter_map(|r| beliefs.get(*r).map(|b| ((*r).to_string(), *b)))
        .collect();
    evidenced.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let evidenced_names: Vec<String> = evidenced.into_iter().map(|(r, _)| r).collect();
    let rest: Vec<String> = all_rules
        .iter()
        .filter(|r| !beliefs.contains_key(**r))
        .map(|r| (*r).to_string())
        .collect();
    let mut plan = evidenced_names;
    plan.extend(rest);
    if subset {
        plan.retain(|r| beliefs.get(r).is_some_and(|b| *b >= level));
    }
    Some(plan)
}

/// The cell: three measured arms + the falsification arm over the frozen
/// fixture set, ONE before-verdict pass shared by all arms (the verify
/// budget is 1 before + 1 after per fixture per arm — the landscape adds
/// zero oracle calls).
///
/// Returns `(arm_a, arm_b, arm_c, arm_falsify, beliefs, level)`. The
/// falsification arm asserts nothing here (the runner is REPORT-only);
/// the report prints the never-wrong row and the caller (CLI) refuses
/// exit-1 on a violated falsification.
#[must_use]
pub fn run_composition_ab(
    oracle: &ClippyOracle,
    fixture_list: &[BenchFixture],
) -> (BenchScore, BenchScore, BenchScore, BenchScore, Beliefs, f32) {
    // ONE before pass, shared (the Issue-108 cache shape — the before
    // verdict is a fixture property; re-deriving per arm is pure cost).
    let mut cache = BTreeMap::new();
    for fx in fixture_list {
        cache.insert(fx.id.to_string(), oracle.lints_for(&super::wrap(fx.code)));
    }
    let arm_a = run_bench_lane(oracle, fixture_list, &cache, &LaneMode::All);
    let beliefs = derive_beliefs(&arm_a);
    let level = fit_level(&beliefs).unwrap_or_else(|| {
        // Unreachable on any real fixture set (arm a always attributes
        // some rule); a synthetic empty set refuses loudly at the CLI.
        eprintln!("composition-ab: no per-rule attribution — cannot fit a level");
        std::process::exit(1);
    });
    let arm_b = run_bench_lane(
        oracle,
        fixture_list,
        &cache,
        &LaneMode::Ranked {
            beliefs: beliefs.clone(),
            level,
        },
    );
    let arm_c = run_bench_lane(
        oracle,
        fixture_list,
        &cache,
        &LaneMode::Subset {
            beliefs: beliefs.clone(),
            level,
        },
    );
    // The falsification arm: an absurd level abstains EVERYTHING — a run
    // that resolves anything is a broken never-wrong row.
    let arm_falsify = run_bench_lane(
        oracle,
        fixture_list,
        &cache,
        &LaneMode::Subset {
            beliefs: beliefs.clone(),
            level: f32::MAX,
        },
    );
    (arm_a, arm_b, arm_c, arm_falsify, beliefs, level)
}

/// The report block: aggregate rows per arm, the per-rule family table
/// (single-target attribution per arm + the arm-c delta), the
/// falsification verdict. Printed, never appended to score history (a
/// measurement lane, the gen-9 shape).
pub fn print_report(
    arm_a: &BenchScore,
    arm_b: &BenchScore,
    arm_c: &BenchScore,
    arm_falsify: &BenchScore,
    beliefs: &Beliefs,
    level: f32,
) {
    let resolved_of = |s: &BenchScore| -> usize {
        s.outcomes
            .iter()
            .map(super::FixtureOutcome::resolved)
            .sum::<usize>()
    };
    let before_of = |s: &BenchScore| -> usize {
        s.outcomes
            .iter()
            .filter(|o| o.ran)
            .map(|o| o.vocab_before.len())
            .sum::<usize>()
    };
    let row = |name: &str, s: &BenchScore| {
        println!(
            "[{name}] heal_rate {:.4} ({}/{}) · created {} · parse_safety {:.4} · edits {} · passes-sum {}",
            s.heal_rate,
            resolved_of(s),
            before_of(s),
            s.created_total,
            s.parse_safety,
            s.edits_total,
            s.outcomes.iter().map(|o| o.passes).sum::<usize>(),
        );
    };
    row("arm-a all", arm_a);
    row("arm-b ranked", arm_b);
    row("arm-c subset", arm_c);
    println!(
        "level = {level:.4} (median belief over {} rated rule(s)); beliefs:",
        beliefs.len()
    );
    for (rule, b) in beliefs {
        println!("  {rule}: {b:.4}");
    }
    // Per-rule family table (single-target attribution per arm).
    println!("per-rule families (a → b → c | Δc−a on resolved):");
    let a = rule_outcomes(arm_a).rows;
    let b = rule_outcomes(arm_b).rows;
    let c = rule_outcomes(arm_c).rows;
    let idx = |rows: &[super::RuleOutcome], rule: &str| -> Option<usize> {
        rows.iter().position(|r| r.rule == rule)
    };
    for (rule, belief) in beliefs {
        let (Some(ia), Some(ib), Some(ic)) = (idx(&a, rule), idx(&b, rule), idx(&c, rule)) else {
            println!("  {rule} (belief {belief:.4}): no single-target fixture — unattributable");
            continue;
        };
        println!(
            "  {rule} (belief {belief:.4}): resolved {}/{} → {}/{} → {}/{} | persist {}/{} → {}/{} → {}/{} | broken {} → {} → {} | created {} → {} → {} | Δresolved {}",
            a[ia].resolved,
            a[ia].fixtures,
            b[ib].resolved,
            b[ib].fixtures,
            c[ic].resolved,
            c[ic].fixtures,
            a[ia].persist,
            a[ia].fixtures,
            b[ib].persist,
            b[ib].fixtures,
            c[ic].persist,
            c[ic].fixtures,
            a[ia].broken,
            b[ib].broken,
            c[ic].broken,
            a[ia].created,
            b[ib].created,
            c[ic].created,
            c[ic].resolved as i128 - a[ia].resolved as i128,
        );
    }
    let skipped = rule_outcomes(arm_a).skipped_multi_target;
    println!("multi-target fixtures (unattributable per rule, aggregate-only): {skipped}");
    // The falsification verdict — the never-wrong row.
    let falsify_resolved = resolved_of(arm_falsify);
    if falsify_resolved == 0 {
        println!("falsification: absurd level resolved 0 findings — never-wrong HOLDS");
    } else {
        println!(
            "falsification: VIOLATED — absurd level resolved {falsify_resolved} finding(s); the lane emits above its level"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::{BenchScore, FixtureOutcome};
    use super::{LaneMode, derive_beliefs, fit_level, plan_fixture};

    fn outcome(rule: &str, resolved: bool, broken: bool) -> FixtureOutcome {
        let before: BTreeSet<String> = [rule.to_string()].into_iter().collect();
        let after: BTreeSet<String> = if resolved {
            BTreeSet::new()
        } else {
            before.clone()
        };
        FixtureOutcome {
            id: format!("fx-{rule}-{resolved}-{broken}"),
            ran: true,
            vocab_before: before.clone(),
            all_before: before.clone(),
            all_after: after,
            vocab_persist: if resolved || broken {
                BTreeSet::new()
            } else {
                before
            },
            created: BTreeSet::new(),
            edits: 1,
            passes: 1,
            parses: !broken,
            drifted: false,
        }
    }

    fn score_with(outcomes: Vec<FixtureOutcome>) -> BenchScore {
        BenchScore {
            outcomes,
            ..BenchScore::default()
        }
    }

    #[test]
    fn belief_derivation_and_level_fit() {
        // rule-a: 3/0 resolved → strong belief; rule-b: 0/2 → weak; rule-c
        // absent (multi-target only) → no evidence.
        let score = score_with(vec![
            outcome("rule-a", true, false),
            outcome("rule-a", true, false),
            outcome("rule-a", true, false),
            outcome("rule-b", false, false),
            outcome("rule-b", false, false),
        ]);
        let beliefs = derive_beliefs(&score);
        assert_eq!(beliefs.len(), 2, "rule-c has no single-target row — absent");
        let a = beliefs["rule-a"];
        let b = beliefs["rule-b"];
        assert!(a > b, "3/0 must outrank 0/2");
        let level = fit_level(&beliefs).expect("two data rules");
        assert!(
            (level - f32::midpoint(a, b)).abs() < 1e-6,
            "even count: average of the middle two"
        );
        assert!(a >= level && level > b, "the level separates the arms");
    }

    #[test]
    fn no_data_refuses_the_level() {
        let beliefs = derive_beliefs(&score_with(vec![]));
        assert!(fit_level(&beliefs).is_none(), "no data → no level");
    }

    #[test]
    fn plan_orders_all_rules_evidenced_first() {
        // Ranked: membership unchanged — evidenced rules ranked `B_i` desc,
        // the un-evidenced remainder trails in frozen order. Subset: the
        // level filter drops below-level AND un-evidenced rules.
        let beliefs: super::Beliefs =
            BTreeMap::from([("rule-a".to_string(), 0.9), ("rule-b".to_string(), 0.2)]);
        let all = ["rule-c", "rule-b", "rule-a", "rule-d"];
        let level = fit_level(&beliefs).unwrap();
        let plan = plan_fixture(
            &LaneMode::Ranked {
                beliefs: beliefs.clone(),
                level,
            },
            &all,
        )
        .unwrap();
        assert_eq!(
            plan,
            vec![
                "rule-a".to_string(),
                "rule-b".to_string(),
                "rule-c".to_string(),
                "rule-d".to_string(),
            ]
        );
        // level = midpoint(0.2, 0.9) = 0.55 — rule-b (0.2) is below it
        // and abstains; the un-evidenced rules never clear any level.
        let plan = plan_fixture(
            &LaneMode::Subset {
                beliefs: beliefs.clone(),
                level,
            },
            &all,
        )
        .unwrap();
        assert_eq!(plan, vec!["rule-a".to_string()]);
    }

    #[test]
    fn plan_tie_breaks_by_name_ascending() {
        // Deterministic total order: equal beliefs → name ascending.
        let tied: super::Beliefs =
            BTreeMap::from([("rule-x".to_string(), 0.5), ("rule-y".to_string(), 0.5)]);
        let plan = plan_fixture(
            &LaneMode::Ranked {
                beliefs: tied,
                level: 0.5,
            },
            &["rule-y", "rule-x"],
        )
        .unwrap();
        assert_eq!(plan, vec!["rule-x".to_string(), "rule-y".to_string()]);
    }

    #[test]
    fn falsification_level_abstains_everything() {
        let beliefs: super::Beliefs = BTreeMap::from([("rule-a".to_string(), 0.99)]);
        let plan = plan_fixture(
            &LaneMode::Subset {
                beliefs,
                level: f32::MAX,
            },
            &["rule-a"],
        )
        .unwrap();
        assert!(plan.is_empty(), "an absurd level abstains everything");
    }

    #[test]
    fn all_lane_is_the_incumbent_none() {
        assert!(plan_fixture(&LaneMode::All, &["rule-a"]).is_none());
    }
}

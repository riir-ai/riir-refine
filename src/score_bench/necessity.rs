//! Component-necessity flip-rate harness (Issue 108 T2).
//!
//! The healer's per-rule heal rate reports how often a rule's fix LANDS,
//! not whether the rule's contribution is **load-bearing** in the composed
//! pipeline — the CVRR distinction (`arXiv:2609.06746`, katgpt-rs Research
//! 555): a component can be *informative* (correlated with resolved
//! diagnostics) yet *decorative* (removing or swapping it changes nothing
//! downstream, because the composition routes around it).
//!
//! ## Arms (the intervention protocol)
//!
//! Per target rule, over the frozen v1 fixture set with the real-clippy
//! oracle:
//!
//! - **baseline** — the untouched composition (empty
//!   [`ComponentOverride`]; byte-identical to `--score-bench`'s heal).
//! - **empty / disabled** — the rule's dispatch slot contributes zero
//!   edits (the `perturb_empty` arm). Δ(resolved) ≠ 0 ⇒ the composition
//!   actually uses the rule; Δ = 0 on a rule with solo resolutions ⇒
//!   DECORATIVE (other rules' edits cover its shapes).
//! - **matched-swap** — the rule's slot dispatches the highest-
//!   co-occurrence donor's collect fn on the same source state (the
//!   `perturb_matched_swap` arm). The `flips_to_donor` axis: does the
//!   outcome follow the DONOR's contribution?
//!
//! Name-presence scoring makes the composition question sharp: the oracle
//! scores per LINT NAME, and ANY edit that dissolves the shape resolves
//! the name — so a rule whose disable changes nothing is provably covered
//! by the rest of the composition on this fixture set.
//!
//! ## Two-sided canaries (the audit-instrument law — mandatory)
//!
//! - **load-bearing canary**: the top-1 target (highest solo resolved)
//!   MUST flip under disable — if even that moves nothing, either the
//!   instrument is broken or every rule is redundant (pathological); the
//!   run reports failure.
//! - **no-op canary**: a planted control rule (`PLANTED_NOOP` — not in
//!   any corpus, inert at dispatch by construction) MUST measure Δ = 0 —
//!   a nonzero there means pipeline NON-DETERMINISM (oracle drift between
//!   arms), never decorativeness; the run reports failure.
//!
//! ## Determinism laws
//!
//! - Target selection is deterministic: top-N by (solo resolved DESC,
//!   rule ASC), then `random` STRIDE picks from the remaining present
//!   rules (never RNG — the replay law; the stride is a pure function of
//!   the sorted remaining list).
//! - Donor selection is deterministic: argmax co-occurrence, ties broken
//!   name-ascending.
//! - The before-verdict cache is derived ONCE (before verdicts are
//!   fixture properties — frozen code; re-deriving per arm is a pure 2×
//!   oracle cost for an identical answer).
//!
//! ## Honest scope
//!
//! - Oracle ground: the v1 clippy bench. Targets are clippy-vocabulary
//!   rules by construction (the schedule comes from the baseline's
//!   `vocab_before`). A `rust_perf` rule's own lane value is INVISIBLE to
//!   this oracle — perf-domain necessity needs the generation-5
//!   mechanical-shape oracle (the T3/T4 extension; the [`ComponentOverride`]
//!   seam already covers both domains, so that mode is a runner, not a
//!   primitive).
//! - The `swap` verdict axis records MOVED/UNMOVED (the outcome-level
//!   `flips_to_donor` analog). Outcome-equality against a donor-disabled
//!   arm would need a third arm per rule — T3 extension, recorded here.
//! - Rows append to `.benchmarks/necessity_history.jsonl` — SEPARATE from
//!   `score_history.jsonl` (trial runs stay isolated from the score
//!   history; the `kat_promotion` precedent). No trajectory-store writes
//!   (measurement-only — the `nonergodic_poc` law).

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::run_bench_overridden;
use super::rule_outcomes;
use super::wrap;
use super::BenchFixture;
use super::BenchScore;
use super::ComponentOverride;
use super::RuleOutcomeRead;
use crate::draft::clippy_oracle::ClippyOracle;
use crate::draft::clippy_oracle::OracleVerdict;

/// The planted no-op control id (Issue 108 T2). Never a real rule: not in
/// `FIXABLE_LINTS`, not in `FIXABLE_RULES`, not in any corpus — inert at
/// dispatch by construction ([`ComponentOverride::dispatch`] only matches
/// slot ids that exist). Its disabled arm is the empty override over the
/// same fixtures: any Δ ≠ 0 is pipeline non-determinism.
pub const PLANTED_NOOP: &str = "planted-noop-control";

/// The necessity history (one row per rule×arm, append-only).
pub const HISTORY_JSONL: &str = ".benchmarks/necessity_history.jsonl";

// Re-exported for the row builder (the report.rs precedent — call sites
// stay stable against the frontier_report home).
pub use crate::frontier_report::git_sha;
pub use crate::frontier_report::local_iso_timestamp;

/// Harness configuration (Issue 108 T2). Defaults mirror T3's schedule:
/// top-20 by solo heal rate + 5 stride picks + the planted no-op.
#[derive(Debug, Clone)]
pub struct NecessityConfig {
    /// Top-N targets by solo resolved count (desc, then rule asc).
    pub top: usize,
    /// Stride picks from the remaining present rules (deterministic).
    pub random: usize,
    /// Smoke mode: cap the fixture list (the bounded e2e — the instrument
    /// validates itself without the full oracle bill).
    pub fixture_cap: Option<usize>,
}

impl Default for NecessityConfig {
    fn default() -> Self {
        Self {
            top: 20,
            random: 5,
            fixture_cap: None,
        }
    }
}

/// One intervention row (one JSONL line). All deltas are arm − baseline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NecessityRow {
    pub ts: String,
    pub sha: String,
    /// The intervened rule (or [`PLANTED_NOOP`]).
    pub rule: String,
    /// `"disabled"` | `"swap:<donor>"`.
    pub arm: String,
    /// The outcome ground — always `"clippy-v1"` today (the perf-domain
    /// mode is the T3 extension; see module docs).
    pub oracle: String,
    /// Ran fixtures this arm scored (healable + clean).
    pub fixtures: usize,
    /// Baseline single-target fixtures of this rule (the solo evidence).
    pub solo_fixtures: usize,
    /// Baseline resolved count for this rule (the known solo numerator).
    pub solo_resolved: usize,
    pub baseline_resolved: usize,
    pub arm_resolved: usize,
    pub delta_resolved: i64,
    pub baseline_created: usize,
    pub arm_created: usize,
    pub delta_created: i64,
    pub baseline_edits: usize,
    pub arm_edits: usize,
    /// The (rule, donor) co-occurrence count (swap rows; 0 otherwise).
    pub co_occurrence: usize,
    /// The swap donor, when this is a swap row.
    pub donor: Option<String>,
    /// `"LOAD-BEARING" | "DECORATIVE" | "NO-EVIDENCE" | "NO-OP-CONTROL"`
    /// (disabled rows) | `"FLIPPED" | "UNMOVED"` (swap rows).
    pub verdict: String,
}

/// One canary's verdict (the audit-instrument law: an audit that cannot
/// fail certifies nothing — both canaries carry an explicit ok/fail state
/// the CLI turns into an exit code).
#[derive(Debug, Clone)]
pub struct CanaryState {
    pub ok: bool,
    pub detail: String,
}

/// The finished harness run.
#[derive(Debug, Clone)]
pub struct NecessityRun {
    pub rows: Vec<NecessityRow>,
    /// The top-1 target's disable MUST move the outcome.
    pub load_bearing_canary: CanaryState,
    /// The planted no-op's disable MUST move nothing.
    pub noop_canary: CanaryState,
    /// Intervention arms executed (excluding baseline).
    pub arms_run: usize,
    /// Swap arms skipped for want of a co-occurring donor.
    pub swap_arms_skipped: usize,
    /// Upper bound on oracle invocations (before pass + per-arm after
    /// passes; excluded fixtures invoke less — a budget ledger, not a
    /// gate).
    pub oracle_budget: usize,
    /// The fixture count actually used (post-cap).
    pub fixtures: usize,
}

/// Canonical unordered pair key (a < b — `BTreeMap` determinism).
fn pair(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// Co-occurrence over ran healable fixtures: for every unordered pair of
/// vocabulary lints co-present in a fixture's `vocab_before`, +1. Pure
/// arithmetic over the baseline [`BenchScore`] — the donor-selection feed.
#[must_use]
pub fn co_occurrence(score: &BenchScore) -> BTreeMap<(String, String), usize> {
    let mut co: BTreeMap<(String, String), usize> = BTreeMap::new();
    for o in &score.outcomes {
        if !o.ran || o.vocab_before.is_empty() {
            continue;
        }
        let lints: Vec<&str> = o.vocab_before.iter().map(String::as_str).collect();
        for i in 0..lints.len() {
            for j in (i + 1)..lints.len() {
                *co.entry(pair(lints[i], lints[j])).or_insert(0) += 1;
            }
        }
    }
    co
}

/// The matched-swap donor for a rule: the highest-co-occurrence OTHER
/// rule, ties broken name-ascending (deterministic). The donor set is the
/// co-occurrence map's keys — clippy vocabulary by construction.
#[must_use]
pub fn donor_for(rule: &str, co: &BTreeMap<(String, String), usize>) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for (a, b) in co.keys() {
        let other = if a == rule { b } else if b == rule { a } else { continue };
        if other == rule {
            continue;
        }
        let count = co[&pair(rule, other)];
        // Strict > keeps the FIRST name-ascending winner on ties (BTreeMap
        // iterates keys sorted → (a, b) pairs ascending).
        let better = match best {
            None => true,
            Some((bc, _)) => count > bc,
        };
        if better {
            best = Some((count, other));
        }
    }
    best.map(|(_, d)| d.to_string())
}

/// Target schedule: top-N by (solo resolved DESC, rule ASC), then
/// `random` stride picks from the remaining present rules. Deterministic
/// by construction — stride sampling, never RNG (the replay law).
#[must_use]
pub fn select_targets(read: &RuleOutcomeRead, top: usize, random: usize) -> Vec<String> {
    let mut present: Vec<(usize, &str)> = read
        .rows
        .iter()
        .map(|r| (r.resolved, r.rule.as_str()))
        .collect();
    // resolved DESC, rule ASC — the deterministic total order.
    present.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let mut targets: Vec<String> = present
        .iter()
        .take(top)
        .map(|(_, rule)| (*rule).to_string())
        .collect();
    let remaining: Vec<&str> = present
        .iter()
        .skip(top)
        .map(|(_, rule)| *rule)
        .collect();
    let k = random.min(remaining.len());
    if k > 0 {
        let len = remaining.len();
        for i in 0..k {
            // Even-stride indices over the sorted remainder — distinct
            // because k ≤ len (len/k ≥ 1).
            let idx = i * len / k;
            targets.push(remaining[idx].to_string());
        }
    }
    targets
}

/// The per-rule verdict from the disabled arm (the CVRR question: is the
/// component USED by the composition?).
#[must_use]
pub fn disabled_verdict(is_noop: bool, solo_resolved: usize, delta_resolved: i64) -> &'static str {
    if is_noop {
        "NO-OP-CONTROL"
    } else if solo_resolved == 0 {
        "NO-EVIDENCE"
    } else if delta_resolved == 0 {
        "DECORATIVE"
    } else {
        "LOAD-BEARING"
    }
}

/// Integer outcome totals (resolved, created, edits) — the arm-vs-baseline
/// comparison currency (integer totals, never floats — the promotion-gate
/// law).
#[must_use]
fn totals(score: &BenchScore) -> (i64, i64, i64) {
    let mut resolved = 0i64;
    let mut created = 0i64;
    let mut edits = 0i64;
    for o in &score.outcomes {
        resolved += i64::from(u32::try_from(o.resolved()).unwrap_or(u32::MAX));
        created += i64::from(u32::try_from(o.created.len()).unwrap_or(u32::MAX));
        edits += i64::from(u32::try_from(o.edits).unwrap_or(u32::MAX));
    }
    (resolved, created, edits)
}

/// The determinism fingerprint: per-fixture (id, resolved, created, edits,
/// parses). The no-op canary requires baseline and no-op arm to agree on
/// EVERY entry — aggregate equality alone could hide compensating drift.
#[must_use]
fn fingerprint(score: &BenchScore) -> Vec<(String, usize, usize, usize, bool)> {
    score
        .outcomes
        .iter()
        .map(|o| {
            (
                o.id.clone(),
                o.resolved(),
                o.created.len(),
                o.edits,
                o.parses,
            )
        })
        .collect()
}

fn solo_of(read: &RuleOutcomeRead, rule: &str) -> (usize, usize) {
    read.rows
        .iter()
        .find(|r| r.rule == rule)
        .map_or((0, 0), |r| (r.fixtures, r.resolved))
}

/// The arm context [`build_row`] consumes (the named-field law — the
/// positional-8 form trips `too_many_arguments` and reads by index).
struct ArmCtx<'a> {
    rule: &'a str,
    arm: String,
    donor: Option<String>,
    solo: (usize, usize),
    baseline_totals: (i64, i64, i64),
    arm_totals: (i64, i64, i64),
    fixtures: usize,
}

fn build_row(ctx: &ArmCtx, co: &BTreeMap<(String, String), usize>) -> NecessityRow {
    let ArmCtx {
        rule,
        arm,
        donor,
        solo,
        baseline_totals,
        arm_totals,
        fixtures,
    } = ctx;
    let delta_resolved = arm_totals.0 - baseline_totals.0;
    let verdict = if donor.is_some() {
        if delta_resolved == 0 {
            "UNMOVED"
        } else {
            "FLIPPED"
        }
    } else {
        disabled_verdict(*rule == PLANTED_NOOP, solo.1, delta_resolved)
    };
    let co_count = donor
        .as_ref()
        .and_then(|d| co.get(&pair(rule, d)).copied())
        .unwrap_or(0);
    NecessityRow {
        ts: local_iso_timestamp(),
        sha: git_sha(),
        rule: rule.to_string(),
        arm: arm.clone(),
        oracle: "clippy-v1".to_string(),
        fixtures: *fixtures,
        solo_fixtures: solo.0,
        solo_resolved: solo.1,
        baseline_resolved: usize::try_from(baseline_totals.0.max(0)).unwrap_or(0),
        arm_resolved: usize::try_from(arm_totals.0.max(0)).unwrap_or(0),
        delta_resolved,
        baseline_created: usize::try_from(baseline_totals.1.max(0)).unwrap_or(0),
        arm_created: usize::try_from(arm_totals.1.max(0)).unwrap_or(0),
        delta_created: arm_totals.1 - baseline_totals.1,
        baseline_edits: usize::try_from(baseline_totals.2.max(0)).unwrap_or(0),
        arm_edits: usize::try_from(arm_totals.2.max(0)).unwrap_or(0),
        co_occurrence: co_count,
        donor: donor.clone(),
        verdict: verdict.to_string(),
    }
}

/// Run the harness: baseline + per-target arms + the planted no-op, with
/// both canaries. Pure orchestration over [`run_bench_overridden`] — the
/// oracle does the measuring, this does the scheduling and the verdicts.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn run_necessity(
    oracle: &ClippyOracle,
    all_fixtures: &[BenchFixture],
    cfg: &NecessityConfig,
) -> NecessityRun {
    let fixtures: Vec<BenchFixture> = match cfg.fixture_cap {
        Some(n) => all_fixtures.iter().take(n).cloned().collect(),
        None => all_fixtures.to_vec(),
    };
    let n = fixtures.len();

    // 1. The before pass (the cache every arm reuses).
    let mut before_cache: BTreeMap<String, OracleVerdict> = BTreeMap::new();
    for fx in &fixtures {
        let verdict = oracle.lints_for(&wrap(fx.code));
        before_cache.insert(fx.id.to_string(), verdict);
    }

    // 2. Baseline arm — the untouched composition.
    let baseline = run_bench_overridden(
        oracle,
        &fixtures,
        &ComponentOverride::default(),
        &before_cache,
    );
    let baseline_totals = totals(&baseline);
    let base_print = fingerprint(&baseline);

    // 3. The per-rule solo read + the co-occurrence feed (both from the
    //    SAME baseline run — one measurement, two reads).
    let solo = rule_outcomes(&baseline);
    let co = co_occurrence(&baseline);

    // 4. Target schedule (deterministic) + arms.
    let targets = select_targets(&solo, cfg.top, cfg.random);
    let (mut rows, arms_run, swap_arms_skipped) =
        run_target_arms(oracle, &fixtures, &before_cache, &targets, &solo, &co, &baseline);

    // 5. The planted no-op arm — the determinism canary.
    let mut ovr = ComponentOverride::default();
    ovr.disabled.insert(PLANTED_NOOP.to_string());
    let noop_arm = run_bench_overridden(oracle, &fixtures, &ovr, &before_cache);
    let arms_run = arms_run + 1;
    let noop_totals = totals(&noop_arm);
    let noop_row = build_row(
        &ArmCtx {
            rule: PLANTED_NOOP,
            arm: "disabled".to_string(),
            donor: None,
            solo: (0, 0),
            baseline_totals,
            arm_totals: noop_totals,
            fixtures: baseline.n_ran,
        },
        &co,
    );
    let noop_ok = noop_totals == baseline_totals && fingerprint(&noop_arm) == base_print;
    let noop_canary = CanaryState {
        ok: noop_ok,
        detail: format!(
            "planted no-op Δresolved={} Δcreated={} Δedits={} ({} vs {} fixtures ran)",
            noop_totals.0 - baseline_totals.0,
            noop_totals.1 - baseline_totals.1,
            noop_totals.2 - baseline_totals.2,
            noop_arm.n_ran,
            baseline.n_ran
        ),
    };
    rows.push(noop_row);

    // 6. The load-bearing canary — the top-1 target MUST flip.
    let load_bearing_canary = match targets.first() {
        Some(top_rule) => {
            let top_delta = rows
                .iter()
                .find(|r| r.rule == *top_rule && r.arm == "disabled")
                .map_or(0, |r| r.delta_resolved);
            CanaryState {
                ok: top_delta != 0,
                detail: format!(
                    "top-1 target `{top_rule}` (solo resolved {}) disabled Δresolved={top_delta}",
                    solo_of(&solo, top_rule).1
                ),
            }
        }
        None => CanaryState {
            ok: false,
            detail: "no targets — the baseline resolved nothing single-target \
                     (population 0 is a refusal, never a green zero)"
                .to_string(),
        },
    };

    NecessityRun {
        rows,
        load_bearing_canary,
        noop_canary,
        arms_run,
        swap_arms_skipped,
        // before pass + one after pass per arm (baseline included), upper
        // bound (excluded fixtures invoke less).
        oracle_budget: n + arms_run * n + n,
        fixtures: n,
    }
}

/// The per-target arm loop (extracted from [`run_necessity`] — the
/// orchestration fn stays readable): disabled arm + matched-swap arm per
/// target, `(rows, arms_run, swap_arms_skipped)` out.
fn run_target_arms(
    oracle: &ClippyOracle,
    fixtures: &[BenchFixture],
    before_cache: &BTreeMap<String, OracleVerdict>,
    targets: &[String],
    solo: &RuleOutcomeRead,
    co: &BTreeMap<(String, String), usize>,
    baseline: &BenchScore,
) -> (Vec<NecessityRow>, usize, usize) {
    let baseline_totals = totals(baseline);
    let mut rows: Vec<NecessityRow> = Vec::new();
    let mut arms_run = 0usize;
    let mut swap_arms_skipped = 0usize;
    for rule in targets {
        let solo_row = solo_of(solo, rule);

        // The empty (disabled) arm.
        let mut ovr = ComponentOverride::default();
        ovr.disabled.insert(rule.clone());
        let arm = run_bench_overridden(oracle, fixtures, &ovr, before_cache);
        arms_run += 1;
        let arm_totals = totals(&arm);
        rows.push(build_row(
            &ArmCtx {
                rule,
                arm: "disabled".to_string(),
                donor: None,
                solo: solo_row,
                baseline_totals,
                arm_totals,
                fixtures: baseline.n_ran,
            },
            co,
        ));

        // The matched-swap arm (highest-co-occurrence donor).
        if let Some(donor) = donor_for(rule, co) {
            let mut ovr = ComponentOverride::default();
            ovr.swap.insert(rule.clone(), donor.clone());
            let arm = run_bench_overridden(oracle, fixtures, &ovr, before_cache);
            arms_run += 1;
            let arm_totals = totals(&arm);
            rows.push(build_row(
                &ArmCtx {
                    rule,
                    arm: format!("swap:{donor}"),
                    donor: Some(donor),
                    solo: solo_row,
                    baseline_totals,
                    arm_totals,
                    fixtures: baseline.n_ran,
                },
                co,
            ));
        } else {
            swap_arms_skipped += 1;
        }
    }
    (rows, arms_run, swap_arms_skipped)
}

/// Append rows to [`HISTORY_JSONL`] (creates the file + parent on first
/// write — the `append_history` precedent).
///
/// # Errors
///
/// I/O failure creating/opening/appending the file, or a row that fails
/// to serialize (`serde_json`) — surfaced to the caller verbatim; the CLI
/// prints it and keeps going (the rows are already on the console).
pub fn append_rows(path: &std::path::Path, rows: &[NecessityRow]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for row in rows {
        let mut line = serde_json::to_string(row).map_err(std::io::Error::other)?;
        line.push('\n');
        f.write_all(line.as_bytes())?;
    }
    Ok(())
}

/// The console report: the per-rule table (verdict from the DISABLED arm —
/// the CVRR question) + both canaries + the budget ledger.
pub fn print_run(run: &NecessityRun) {
    println!("\n─ necessity bench (Issue 108) ─────────────────────────────");
    println!(
        "fixtures: {} · arms: {} (swap skipped: {}) · oracle budget ≤ {}",
        run.fixtures, run.arms_run, run.swap_arms_skipped, run.oracle_budget
    );
    println!(
        "{:<32} {:>5} {:>8} {:>8}  {:>12}",
        "rule", "solo", "Δdis", "Δswap", "verdict"
    );
    // Disabled rows drive the table; the paired swap row (same rule,
    // immediately after) fills Δswap.
    let mut i = 0usize;
    while i < run.rows.len() {
        let row = &run.rows[i];
        if row.arm != "disabled" {
            i += 1;
            continue;
        }
        let swap_delta = run.rows.get(i + 1).and_then(|next| {
            (next.rule == row.rule && next.arm.starts_with("swap:")).then_some(next.delta_resolved)
        });
        let swap_cell = swap_delta.map_or_else(
            || "(none)".to_string(),
            |d| format!("{d:+}"),
        );
        println!(
            "{:<32} {:>5} {:>+8} {:>8}  {:>12}",
            row.rule, row.solo_resolved, row.delta_resolved, swap_cell, row.verdict
        );
        i += 1;
    }
    println!(
        "\ncanary load-bearing: {} — {}",
        if run.load_bearing_canary.ok { "OK" } else { "FAIL" },
        run.load_bearing_canary.detail
    );
    println!(
        "canary no-op:        {} — {}",
        if run.noop_canary.ok { "OK" } else { "FAIL" },
        run.noop_canary.detail
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn outcome(id: &str, vocab: &[&str], persist: &[&str], edits: usize) ->
        crate::score_bench::FixtureOutcome
    {
        crate::score_bench::FixtureOutcome {
            id: id.to_string(),
            ran: true,
            vocab_before: vocab.iter().map(|s| s.to_string()).collect(),
            all_before: BTreeSet::new(),
            all_after: BTreeSet::new(),
            vocab_persist: persist.iter().map(|s| s.to_string()).collect(),
            created: BTreeSet::new(),
            edits,
            passes: 1,
            parses: true,
            drifted: false,
        }
    }

    fn score_of(outcomes: Vec<crate::score_bench::FixtureOutcome>) -> BenchScore {
        BenchScore {
            outcomes,
            ..BenchScore::default()
        }
    }

    /// The empty override is byte-identical to the untouched fixpoint —
    /// the non-vacuity pin (an instrument that moves anything while
    /// claiming nothing is broken).
    #[test]
    fn empty_override_is_byte_identical() {
        use crate::score_bench::heal_fixpoint;
        use crate::score_bench::heal_fixpoint_cfg;
        let code = "fn t(x: Option<u8>) -> Option<String> { match x { Some(v) => \
                    Some(v.to_string()), None => None, } }\nfn f(name: &str) -> \
                    String { format!(\"{}\", name) }\nfn r() -> i32 { return 42; }\n";
        let plain = heal_fixpoint(code);
        let cfg = heal_fixpoint_cfg(code, &[], &ComponentOverride::default());
        assert_eq!(plain, cfg);
    }

    /// The empty (disabled) arm: a disabled rule contributes nothing — on
    /// a manual_map-ONLY shape (non-identity arm body: needless_match
    /// refuses it) disabling manual_map leaves the code untouched.
    #[test]
    fn disabled_rule_contributes_no_edits() {
        use crate::score_bench::heal_fixpoint_cfg;
        let code =
            "fn t(x: Option<u8>) -> Option<u8> { match x { Some(v) => Some(v * 2), None => None, } }";
        let mut ovr = ComponentOverride::default();
        ovr.disabled.insert("manual_map".to_string());
        let (healed, edits, _) = heal_fixpoint_cfg(code, &[], &ovr);
        assert_eq!(edits, 0, "manual_map disabled — nothing else fires on this shape");
        assert_eq!(healed, code, "byte-identical — the disabled slot contributed nothing");
    }

    /// The matched-swap arm redirects dispatch OBSERVABLY: BOTH match-rule
    /// slots are disabled (silent) and a bystander rule's slot is swapped
    /// to the needless_match donor — the identity shape can only heal to
    /// `{ x }` THROUGH the swapped slot. If the swap were inert (the slot
    /// still dispatching redundant_static_lifetimes), no rule would fire
    /// and the match would survive untouched.
    #[test]
    fn swap_redirects_dispatch_to_donor() {
        use crate::score_bench::heal_fixpoint_cfg;
        let code =
            "fn t(x: Option<u8>) -> Option<u8> { match x { Some(v) => Some(v), None => None, } }";
        let mut ovr = ComponentOverride::default();
        ovr.disabled.insert("needless_match".to_string());
        ovr.disabled.insert("manual_map".to_string());
        ovr.swap.insert(
            "redundant_static_lifetimes".to_string(),
            "needless_match".to_string(),
        );
        let (healed, edits, _) = heal_fixpoint_cfg(code, &[], &ovr);
        assert!(edits > 0, "the donor must fire through the swapped slot: {healed}");
        assert!(
            healed.contains("{ x }"),
            "needless_match's rewrite through the slot: {healed}"
        );
        assert!(
            !healed.contains(".map("),
            "manual_map is disabled — its rewrite must not appear: {healed}"
        );
    }

    /// Unknown names are inert — a planted no-op cannot panic dispatch,
    /// and disabling a nonexistent rule changes nothing.
    #[test]
    fn unknown_names_are_inert() {
        use crate::score_bench::heal_fixpoint;
        use crate::score_bench::heal_fixpoint_cfg;
        let code = "fn f(name: &str) -> String { format!(\"{}\", name) }";
        let mut ovr = ComponentOverride::default();
        ovr.disabled.insert(PLANTED_NOOP.to_string());
        ovr.swap.insert("nonexistent-rule".to_string(), "also-nonexistent".to_string());
        assert_eq!(heal_fixpoint(code), heal_fixpoint_cfg(code, &[], &ovr));
        assert_eq!(ovr.dispatch(PLANTED_NOOP), None);
        assert_eq!(ovr.dispatch("manual_map"), Some("manual_map"));
    }

    /// Donor selection: argmax co-occurrence, ties name-ascending,
    /// never self.
    #[test]
    fn donor_selection_is_argmax_then_name_asc() {
        let mut co = BTreeMap::new();
        co.insert(pair("rule_a", "donor_x"), 3);
        co.insert(pair("rule_a", "donor_b"), 3);
        co.insert(pair("rule_a", "donor_c"), 1);
        co.insert(pair("other", "donor_z"), 9);
        assert_eq!(donor_for("rule_a", &co).as_deref(), Some("donor_b"));
        assert_eq!(donor_for("absent", &co), None);
    }

    /// Stride sampling: deterministic, in-range, distinct, and clamped to
    /// the remaining population.
    #[test]
    fn stride_selection_is_deterministic_and_distinct() {
        let mut read = RuleOutcomeRead::default();
        for name in ["r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10"] {
            read.rows.push(crate::score_bench::RuleOutcome {
                rule: name.to_string(),
                fixtures: 1,
                resolved: 5,
                persist: 0,
                broken: 0,
                created: 0,
                edits: 1,
                shaped: 0,
            });
        }
        let a = select_targets(&read, 3, 4);
        let b = select_targets(&read, 3, 4);
        assert_eq!(a, b, "deterministic");
        assert_eq!(&a[..3], &["r1", "r10", "r2"], "top-3 by (resolved, name)");
        assert_eq!(a.len(), 7, "3 top + 4 stride");
        let stride = &a[3..];
        let mut sorted = stride.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), stride.len(), "stride picks distinct");
        // Over-asked random clamps to the remaining population.
        let c = select_targets(&read, 3, 99);
        assert_eq!(c.len(), 3 + 7, "clamped to the remainder");
    }

    /// The disabled-arm verdict ladder.
    #[test]
    fn verdict_ladder() {
        assert_eq!(disabled_verdict(true, 5, 0), "NO-OP-CONTROL");
        assert_eq!(disabled_verdict(false, 0, 0), "NO-EVIDENCE");
        assert_eq!(disabled_verdict(false, 5, 0), "DECORATIVE");
        assert_eq!(disabled_verdict(false, 5, -2), "LOAD-BEARING");
        assert_eq!(disabled_verdict(false, 5, 1), "LOAD-BEARING");
    }

    /// Co-occurrence counts unordered co-present pairs once per fixture.
    #[test]
    fn co_occurrence_counts_pairs() {
        let score = score_of(vec![
            outcome("f1", &["a", "b"], &[], 2),
            outcome("f2", &["a", "b"], &[], 1),
            outcome("f3", &["b", "c"], &[], 1),
            outcome("f4", &["a"], &[], 1),
        ]);
        let co = co_occurrence(&score);
        assert_eq!(co.get(&pair("a", "b")), Some(&2));
        assert_eq!(co.get(&pair("b", "c")), Some(&1));
        assert_eq!(co.get(&pair("a", "c")), None);
    }

    /// Row JSONL round-trip (the append contract).
    #[test]
    fn row_round_trips() {
        let row = NecessityRow {
            ts: "2026-09-15T00:00:00".to_string(),
            sha: "f7dafdd9".to_string(),
            rule: "manual_map".to_string(),
            arm: "swap:needless_match".to_string(),
            oracle: "clippy-v1".to_string(),
            fixtures: 37,
            solo_fixtures: 3,
            solo_resolved: 3,
            baseline_resolved: 100,
            arm_resolved: 97,
            delta_resolved: -3,
            baseline_created: 2,
            arm_created: 4,
            delta_created: 2,
            baseline_edits: 500,
            arm_edits: 480,
            co_occurrence: 7,
            donor: Some("needless_match".to_string()),
            verdict: "FLIPPED".to_string(),
        };
        let json = serde_json::to_string(&row).expect("serializes");
        let back: NecessityRow = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, row);
    }

    /// The no-op row classification: the planted control is labeled, never
    /// pooled with real rules' verdicts.
    #[test]
    fn noop_row_is_labeled() {
        let row = build_row(
            &ArmCtx {
                rule: PLANTED_NOOP,
                arm: "disabled".to_string(),
                donor: None,
                solo: (0, 0),
                baseline_totals: (10, 1, 50),
                arm_totals: (10, 1, 50),
                fixtures: 37,
            },
            &BTreeMap::new(),
        );
        assert_eq!(row.verdict, "NO-OP-CONTROL");
        assert_eq!(row.delta_resolved, 0);
    }
}

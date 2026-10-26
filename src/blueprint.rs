//! Blueprint history — the meta level's instrument (Proposal 024 Phase A,
//! issue 178).
//!
//! One level above the Level-1 evolution loop ([`crate::evolve`] +
//! [`crate::fix_verify`] + [`crate::traj_store`]): every `--fix` campaign
//! appends ONE bounded row recording the posture it ran under (the lever
//! values) and the outcomes it produced (the verdicts). The row is the sole
//! declared side-effect of the lane (G1): stdout and fix outcomes are
//! byte-identical with the lane active, and the demote-only env
//! `RIIR_REFINE_META_BLUEPRINT=0` (the exact literal) restores zero writes —
//! the file is never created.
//!
//! Phase A ships ONLY the instrument + the report: no arm machinery, no
//! selection, no actuator change. Every row carries `arm: "incumbent"` —
//! the field exists from day one so Phase B arms (behind the DEFAULT-OFF
//! `meta_blueprint` feature) join the same history without a schema break.
//! Scoring is arm-invariant by construction: Phase A has one posture, and
//! the report's posture↔verdict audit split makes verify-posture-vs-verdict
//! visible on every run — a future arm can never win by blinding the judge.
//!
//! The pre-registered volume close (proposal caveat 1): 8 weeks after the
//! FIRST history row, if total campaigns < 16 OR no lever class holds ≥ 6
//! campaigns in each of ≥ 2 arms, the lane closes with insufficient volume —
//! the report + demotion suggestions remain the landing. Past the window the
//! status line (close or explicit not-tripped form) prints on EVERY report
//! run; before the window it prints on none.
//!
//! The demotion-suggestion join reads the necessity-bench history
//! ([`crate::score_bench::necessity`] rows) and surfaces report-only
//! suggestions naming the exact `--active-set-demote` command. It NEVER
//! auto-executes: demoting corpus rules touches mining economics and stays
//! an operator act.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The history file, relative to the heal root (the scanned repo).
pub const HISTORY_RELPATH: &str = ".refine/blueprint_history.jsonl";

/// The necessity history the demotion-suggestion join reads, relative to
/// the heal root. Wire-shape authority: [`crate::score_bench::necessity`]
/// (feature-gated) — this path const is mirrored here so the join works at
/// every feature posture.
pub const NECESSITY_HISTORY_RELPATH: &str = ".benchmarks/necessity_history.jsonl";

/// Demote-only env: unset or any value but the exact literal `0` keeps the
/// lane on (the `scoped_recheck` kill-switch shape).
pub const ENV_DEMOTE: &str = "RIIR_REFINE_META_BLUEPRINT";

/// Pre-registered close window: 8 weeks after the first history row.
pub const CLOSE_WINDOW_SECS: u64 = 8 * 7 * 24 * 3600;
/// Below this total past the window, the lane closes.
pub const MIN_CAMPAIGNS: usize = 16;
/// A lever-class arm needs at least this many campaigns to count.
pub const MIN_PER_ARM: usize = 6;
/// A lever class qualifies only with at least this many arms at [`MIN_PER_ARM`].
pub const MIN_ARMS_PER_CLASS: usize = 2;
/// Strata entries per row (top by total outcome) — the row stays bounded
/// no matter how many distinct rules a campaign touched.
pub const MAX_STRATA: usize = 32;
/// Schema version of the on-disk row.
pub const ROW_VERSION: u8 = 1;

/// Whether the append lane is enabled (default) or demoted (`=0`).
#[must_use]
pub fn lane_enabled() -> bool {
    lane_enabled_from(std::env::var(ENV_DEMOTE).ok().as_deref())
}

/// The env-value decision, split out for posture tests (no env mutation —
/// cargo test runs its tests in threads).
#[must_use]
pub fn lane_enabled_from(env_value: Option<&str>) -> bool {
    env_value.is_none_or(|v| v != "0")
}

/// The verify-lane posture a campaign ran under (the lever values, v1).
#[allow(clippy::struct_excessive_bools)] // the posture IS four independent lever flags (verify / scoped-recheck / repo-posture / fmt)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintPosture {
    /// Whether the compile gate ran (`--verify` implied by `--fix` on
    /// `fix_verify` builds; `--no-verify` opts out).
    pub verify: bool,
    /// The `--verify-args` override, when set (cargo args for the gate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_args: Option<String>,
    /// Issue-171 T0b package-scoped re-check posture.
    pub scoped_recheck: bool,
    /// Plan-165 repo-lint-posture fix mode.
    pub repo_posture: bool,
    /// Whether the post-fix format pass was armed (default on).
    pub fmt: bool,
    /// The `--domain` scope (`"auto"` when unset).
    pub domain: String,
    /// The per-span candidate cap.
    pub top_k: usize,
}

/// Printed findings per domain + the fixable floor (the roll-up's inputs).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintFindings {
    pub total: usize,
    /// Domain label → printed findings (the `findings by domain` map).
    pub by_domain: BTreeMap<String, usize>,
    /// Citations in the compiled clippy fix set (the fix CTA floor).
    pub fixable_clippy: usize,
}

/// Final outcomes of one campaign (what survived, what was refused).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintOutcomes {
    pub files_written: usize,
    pub edits_written: usize,
    /// Whether the compile gate ran at all (false: `--no-verify`, a
    /// non-`fix_verify` build, or nothing to verify).
    pub verify_ran: bool,
    pub kept_files: usize,
    pub kept_edits: usize,
    pub reverted: usize,
    /// cargo check invocations (baseline + rounds) — the realized verify
    /// DEPTH, the lever Phase B arms will vary.
    pub checks: usize,
    /// Per-package re-checks skipped by the Issue-171 scoped posture.
    pub skipped_checks: usize,
    /// Findings the proposers declined (posture gates + level skips).
    pub declined: usize,
}

/// Per-rule stratum: final outcome counts for one rule id in one campaign.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleOutcome {
    pub written: u32,
    pub reverted: u32,
    pub declined: u32,
}

/// The verify block's hoisted capture (the report is block-scoped in the
/// fix lane; these six fields are what the run-end row needs from it).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VerifyCapture {
    pub ran: bool,
    pub kept_files: usize,
    pub kept_edits: usize,
    pub reverted: usize,
    pub checks: usize,
    pub skipped_checks: usize,
}

impl RuleOutcome {
    fn total(&self) -> u64 {
        u64::from(self.written) + u64::from(self.reverted) + u64::from(self.declined)
    }
}

/// One campaign's row (one JSONL line, bounded).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlueprintRow {
    pub v: u8,
    /// Unix seconds at run end.
    pub ts: u64,
    /// BLAKE3 (truncated) over mode + posture + sorted relative paths —
    /// groups re-runs of the same campaign content; NOT a dedup key.
    pub digest: String,
    /// `"fix"` (v1 appends fix campaigns only; the field is the forward
    /// wire for future modes).
    pub mode: String,
    /// The blueprint arm that ran. Phase A: always `"incumbent"`.
    pub arm: String,
    pub posture: BlueprintPosture,
    pub findings: BlueprintFindings,
    pub outcomes: BlueprintOutcomes,
    /// rule id → outcome, capped at [`MAX_STRATA`] entries by total count
    /// (ties break by rule id — `BTreeMap` determinism).
    pub strata: BTreeMap<String, RuleOutcome>,
}

/// Build the campaign digest: BLAKE3 over mode ‖ posture ‖ sorted relative
/// paths, truncated to 16 bytes hex (32 chars).
#[must_use]
pub fn campaign_digest(mode: &str, posture: &BlueprintPosture, paths: &[PathBuf]) -> String {
    let mut h = blake3::Hasher::new();
    h.update(mode.as_bytes());
    h.update(&[0]);
    h.update(serde_json::to_vec(posture).unwrap_or_default().as_slice());
    h.update(&[0]);
    let mut rel: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    rel.sort();
    rel.dedup();
    for r in &rel {
        h.update(r.as_bytes());
        h.update(&[0]);
    }
    let hash = h.finalize();
    let bytes = hash.as_bytes();
    let mut s = String::with_capacity(32);
    for b in &bytes[..16] {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Cap the strata map at [`MAX_STRATA`] entries: keep the highest-total
/// rules, ties by rule id ascending (`BTreeMap` iteration is already sorted).
#[must_use]
pub fn cap_strata(strata: BTreeMap<String, RuleOutcome>) -> BTreeMap<String, RuleOutcome> {
    if strata.len() <= MAX_STRATA {
        return strata;
    }
    let mut ranked: Vec<(u64, &String)> = strata.iter().map(|(k, v)| (v.total(), k)).collect();
    // Descending total, ascending rule id on ties.
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    ranked
        .into_iter()
        .take(MAX_STRATA)
        .map(|(_, k)| (k.clone(), strata[k].clone()))
        .collect()
}

/// Append one row (ONE line) to the history under `root`. Creates the
/// `.refine/` parent on first write. The caller gates on [`lane_enabled`]
/// (via [`maybe_append_row`]); this fn itself is ungated so tests can pin
/// the write shape directly.
///
/// # Errors
///
/// Propagates create-dir/append IO errors — the caller renders them on
/// stderr and continues (the meter-save precedent: a lost bookkeeping event
/// never fails a heal).
pub fn append_row(root: &Path, row: &BlueprintRow) -> std::io::Result<PathBuf> {
    let path = root.join(HISTORY_RELPATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(row).map_err(std::io::Error::other)?;
    line.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?
        .write_all(line.as_bytes())?;
    Ok(path)
}

/// The gated write: [`lane_enabled`] on → append (silent on success — G1's
/// stdout byte-identity is by construction); demoted → no touch, returns
/// `false` (the file is never created). Write failures surface as
/// `Err` for the caller to render on stderr — a lost bookkeeping row never
/// fails a heal.
///
/// # Errors
///
/// Propagates [`append_row`]'s IO/serialization errors.
pub fn maybe_append_row(root: &Path, row: &BlueprintRow) -> std::io::Result<bool> {
    maybe_append_row_gated(lane_enabled(), root, row)
}

/// [`maybe_append_row`] with the gate injected — the posture tests' seam
/// (the production caller passes [`lane_enabled`]).
///
/// # Errors
///
/// Propagates [`append_row`]'s IO/serialization errors.
pub fn maybe_append_row_gated(
    enabled: bool,
    root: &Path,
    row: &BlueprintRow,
) -> std::io::Result<bool> {
    if !enabled {
        return Ok(false);
    }
    append_row(root, row)?;
    Ok(true)
}

/// Load all rows; malformed lines are SKIPPED AND COUNTED (never a crash,
/// never a silent drop).
#[must_use]
pub fn load_rows(path: &Path) -> (Vec<BlueprintRow>, usize) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (Vec::new(), 0);
    };
    let mut rows = Vec::new();
    let mut bad = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<BlueprintRow>(line) {
            Ok(r) => rows.push(r),
            Err(_) => bad += 1,
        }
    }
    (rows, bad)
}

// ── the volume close (pre-registered arithmetic) ─────────────────────────

/// The pre-registered volume status of the lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VolumeStatus {
    /// No rows yet — the close window has not started; no line prints.
    NoRows,
    /// Inside the 8-week window from the first row; `ends_unix` is when it
    /// elapses. No line prints until then.
    Window { ends_unix: u64 },
    /// Past the window and the close condition FIRED (insufficient volume):
    /// total below [`MIN_CAMPAIGNS`] or no qualified lever class.
    Close { total: usize, qualified: usize },
    /// Past the window with sufficient volume — the explicit not-tripped
    /// form (the cannot-silently-not-fire half).
    Open { total: usize, qualified: usize },
}

/// One lever class's arm census: distinct values → campaign counts.
type ArmCensus = BTreeMap<String, usize>;

fn class_census<'a>(
    rows: &'a [BlueprintRow],
    value_of: impl Fn(&'a BlueprintRow) -> String,
) -> ArmCensus {
    let mut census: ArmCensus = BTreeMap::new();
    for r in rows {
        *census.entry(value_of(r)).or_default() += 1;
    }
    census
}

/// Count qualified lever classes: classes (verify posture, domain scope,
/// repo-posture mode) holding ≥ [`MIN_ARMS_PER_CLASS`] distinct values each
/// with ≥ [`MIN_PER_ARM`] campaigns. Phase A data (one incumbent posture)
/// yields zero — the honest default that arms the close.
#[must_use]
pub fn qualified_classes(rows: &[BlueprintRow]) -> usize {
    let classes: [ArmCensus; 3] = [
        class_census(rows, |r| r.posture.verify.to_string()),
        class_census(rows, |r| r.posture.domain.clone()),
        class_census(rows, |r| r.posture.repo_posture.to_string()),
    ];
    classes
        .iter()
        .filter(|c| c.values().filter(|&&n| n >= MIN_PER_ARM).count() >= MIN_ARMS_PER_CLASS)
        .count()
}

/// The pre-registered close arithmetic over the loaded history.
#[must_use]
pub fn volume_status(rows: &[BlueprintRow], now_unix: u64) -> VolumeStatus {
    let Some(first) = rows.iter().map(|r| r.ts).min() else {
        return VolumeStatus::NoRows;
    };
    let ends = first.saturating_add(CLOSE_WINDOW_SECS);
    if now_unix < ends {
        return VolumeStatus::Window { ends_unix: ends };
    }
    let total = rows.len();
    let qualified = qualified_classes(rows);
    if total < MIN_CAMPAIGNS || qualified == 0 {
        VolumeStatus::Close { total, qualified }
    } else {
        VolumeStatus::Open { total, qualified }
    }
}

/// The status line: `None` before the window (and on no rows), the close or
/// explicit not-tripped form past it.
#[must_use]
pub fn volume_line(status: &VolumeStatus) -> Option<String> {
    match status {
        VolumeStatus::NoRows | VolumeStatus::Window { .. } => None,
        VolumeStatus::Close { total, qualified } => Some(format!(
            "META-VOLUME-CLOSE: insufficient volume (total campaigns={total}, \
             qualified lever classes={qualified}) — the {WEEKS_TEXT}-week window elapsed; \
             the lane closes; this report + the demotion suggestions remain the landing"
        )),
        VolumeStatus::Open { total, qualified } => Some(format!(
            "META-VOLUME-OPEN: {total} campaign(s), {qualified} qualified lever class(es) \
             — close condition not met"
        )),
    }
}

const WEEKS_TEXT: &str = "8";

// ── the necessity → demotion-suggestion join (report-only) ───────────────────

/// Read-only wire-shape mirror of the necessity row
/// ([`crate::score_bench::necessity::NecessityRow`] — feature-gated; this
/// module is ungated, so the join reads the history at EVERY posture).
/// Subset of fields by name; unknown fields are ignored by serde. The
/// authority for the wire shape is that module — the `necessity_row_mirror`
/// test pins the two together when the feature is compiled.
#[derive(Debug, Clone, Deserialize)]
pub struct NecessityWire {
    pub rule: String,
    /// `"disabled" | "swap:<donor>"` — only disabled arms are joinable.
    pub arm: String,
    /// The outcome ground (`"clippy-v1"` today) → the active-set domain key.
    pub oracle: String,
    /// `"DECORATIVE" | "NO-EVIDENCE"` price a rule removable.
    pub verdict: String,
    #[serde(default)]
    pub delta_resolved: i64,
}

/// One report-only suggestion: a rule the necessity bench priced removable,
/// not already demoted, with the exact operator command named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemotionSuggestion {
    pub domain: String,
    pub rule: String,
    pub verdict: String,
    pub delta_resolved: i64,
}

/// The necessity verdicts that price a rule removable (the disabled-arm
/// verdicts minus the load-bearing/Control classes).
const REMOVABLE_VERDICTS: [&str; 2] = ["DECORATIVE", "NO-EVIDENCE"];

/// Join the necessity history against the active set: latest row per rule,
/// disabled-arm removable verdicts only, already-demoted rules skipped.
/// The oracle string maps to the active-set domain key (`clippy-v1` →
/// `clippy`); unknown oracles map by the `-v1` suffix strip and are the
/// caller's to disclose.
#[must_use]
pub fn demotion_suggestions(
    necessity_rows: &[NecessityWire],
    active: &crate::active_set::ActiveSet,
) -> Vec<DemotionSuggestion> {
    let mut latest: BTreeMap<&str, &NecessityWire> = BTreeMap::new();
    for row in necessity_rows {
        if row.arm != "disabled" {
            continue;
        }
        latest.insert(row.rule.as_str(), row);
    }
    latest
        .into_values()
        .filter(|row| REMOVABLE_VERDICTS.contains(&row.verdict.as_str()))
        .filter_map(|row| {
            let domain = match row.oracle.as_str() {
                "clippy-v1" => "clippy".to_string(),
                other => other.strip_suffix("-v1").unwrap_or(other).to_string(),
            };
            if active.excludes(&domain, &row.rule) {
                return None;
            }
            Some(DemotionSuggestion {
                domain,
                rule: row.rule.clone(),
                verdict: row.verdict.clone(),
                delta_resolved: row.delta_resolved,
            })
        })
        .collect()
}

/// Load the necessity history (wire-shape read); malformed lines skipped
/// AND counted (never a crash, never a silent drop).
#[must_use]
pub fn load_necessity_rows(path: &Path) -> (Vec<NecessityWire>, usize) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (Vec::new(), 0);
    };
    let mut rows = Vec::new();
    let mut bad = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<NecessityWire>(line) {
            Ok(r) => rows.push(r),
            Err(_) => bad += 1,
        }
    }
    (rows, bad)
}

// ── report rendering (A2/A3) ─────────────────────────────────────────────

/// The malformed-lines suffix a report header carries when the history has
/// unreadable rows (skipped AND counted, never a silent drop).
fn malformed_note(malformed: usize) -> String {
    if malformed > 0 {
        format!(" ({malformed} malformed line(s) skipped)")
    } else {
        String::new()
    }
}

/// Render the full report (the `--blueprint-report` body). Read-only over
/// the inputs; deterministic; never prints (the caller owns stdout).
#[must_use]
pub fn render_report(
    rows: &[BlueprintRow],
    malformed: usize,
    suggestions: &[DemotionSuggestion],
    necessity_absent: bool,
    status: &VolumeStatus,
) -> String {
    let mut out = String::new();
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "blueprint report: 0 campaign(s){} — the lane appends one row per --fix campaign",
            malformed_note(malformed)
        );
        render_suggestions(&mut out, suggestions, necessity_absent);
        return out;
    }
    let first = rows.iter().map(|r| r.ts).min().unwrap_or_default();
    let last = rows.iter().map(|r| r.ts).max().unwrap_or_default();
    let span_days = last.saturating_sub(first) / 86_400;
    let _ = writeln!(
        out,
        "blueprint report: {} campaign(s), span {span_days} day(s), first row {first}, latest {last}{}",
        rows.len(),
        malformed_note(malformed)
    );
    render_mode_arm_split(&mut out, rows);
    render_posture_audit(&mut out, rows);
    render_lever_splits(&mut out, rows);
    render_strata(&mut out, rows);
    render_suggestions(&mut out, suggestions, necessity_absent);
    if let Some(line) = volume_line(status) {
        let _ = writeln!(out, "{line}");
    }
    out
}

/// The mode + arm census line.
fn render_mode_arm_split(out: &mut String, rows: &[BlueprintRow]) {
    let mut by_mode: ArmCensus = BTreeMap::new();
    let mut by_arm: ArmCensus = BTreeMap::new();
    for r in rows {
        *by_mode.entry(r.mode.clone()).or_default() += 1;
        *by_arm.entry(r.arm.clone()).or_default() += 1;
    }
    let census = |c: &ArmCensus| {
        c.iter()
            .map(|(k, n)| format!("{k}={n}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let _ = writeln!(out, "mode: {} · arm: {}", census(&by_mode), census(&by_arm));
}

/// The posture↔verdict audit split — the arm-invariant-judging witness:
/// outcomes grouped by the verify posture they were judged under, so an
/// arm can never win by blinding the oracle without the report showing it.
fn render_posture_audit(out: &mut String, rows: &[BlueprintRow]) {
    out.push_str("posture\u{2194}verdict audit (scoring is arm-invariant — every arm is judged under the incumbent verify posture it ran):\n");
    let mut audit: BTreeMap<String, (usize, usize, usize, usize, usize)> = BTreeMap::new();
    for r in rows {
        let e = audit
            .entry(format!(
                "verify={} scoped_recheck={} repo_posture={}",
                r.posture.verify, r.posture.scoped_recheck, r.posture.repo_posture
            ))
            .or_default();
        e.0 += 1;
        e.1 += r.outcomes.kept_edits;
        e.2 += r.outcomes.reverted;
        e.3 += r.outcomes.checks;
        e.4 += r.outcomes.skipped_checks;
    }
    for (posture, (n, kept, reverted, checks, skipped)) in &audit {
        let total_edits = kept + reverted;
        let rate = if total_edits > 0 {
            let reverted_f = f64::from(u32::try_from(*reverted).unwrap_or(u32::MAX));
            let total_f = f64::from(u32::try_from(total_edits).unwrap_or(u32::MAX));
            format!("{:.1}%", reverted_f * 100.0 / total_f)
        } else {
            "n/a".to_string()
        };
        let _ = writeln!(
            out,
            "  {posture}: campaigns={n} kept={kept} reverted={reverted} (revert-rate {rate}) checks={checks} skipped_checks={skipped}"
        );
    }
}

/// The per-lever-value campaign/mean-kept splits.
fn render_lever_splits(out: &mut String, rows: &[BlueprintRow]) {
    out.push_str("lever splits (campaigns · mean kept/campaign):\n");
    for (label, rows_by_value) in [
        ("domain", group_by(rows, |r| r.posture.domain.clone())),
        ("top_k", group_by(rows, |r| r.posture.top_k.to_string())),
        (
            "repo_posture",
            group_by(rows, |r| r.posture.repo_posture.to_string()),
        ),
    ] {
        for (value, group) in &rows_by_value {
            let kept: usize = group.iter().map(|r| r.outcomes.kept_edits).sum();
            let mean = f64::from(u32::try_from(kept).unwrap_or(u32::MAX))
                / f64::from(u32::try_from(group.len()).unwrap_or(u32::MAX));
            let _ = writeln!(
                out,
                "  {label}={value}: {} campaign(s), mean kept/campaign {mean:.2}",
                group.len()
            );
        }
    }
}

/// The per-rule stratum table, top totals across the history.
fn render_strata(out: &mut String, rows: &[BlueprintRow]) {
    let mut merged: BTreeMap<&str, RuleOutcome> = BTreeMap::new();
    for r in rows {
        for (rule, o) in &r.strata {
            let e = merged.entry(rule.as_str()).or_default();
            e.written = e.written.saturating_add(o.written);
            e.reverted = e.reverted.saturating_add(o.reverted);
            e.declined = e.declined.saturating_add(o.declined);
        }
    }
    let mut strata_rows: Vec<(&str, u64)> = merged.iter().map(|(k, v)| (*k, v.total())).collect();
    strata_rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let shown = strata_rows.len().min(MAX_STRATA);
    let _ = writeln!(out, "strata (top {shown} of {} rule(s)):", merged.len());
    for (rule, _) in strata_rows.iter().take(MAX_STRATA) {
        let o = &merged[rule];
        let _ = writeln!(
            out,
            "  {rule:<44} written={} reverted={} declined={}",
            o.written, o.reverted, o.declined
        );
    }
}

/// The demotion-suggestion section (the necessity join's output).
fn render_suggestions(
    out: &mut String,
    suggestions: &[DemotionSuggestion],
    necessity_absent: bool,
) {
    if necessity_absent {
        out.push_str(
            "demotion suggestions: no necessity history beside this repo — \
             run `cargo refine --necessity-bench` here first; nothing derived\n",
        );
    } else if suggestions.is_empty() {
        out.push_str(
            "demotion suggestions: none — no removable verdicts past the already-demoted set\n",
        );
    } else {
        out.push_str("demotion suggestions (REPORT-ONLY — never auto-executed):\n");
        for s in suggestions {
            let _ = writeln!(
                out,
                "  demote-suggest: {}:{} — {} (Δresolved={}) — cargo refine --active-set-demote {}:{}",
                s.domain, s.rule, s.verdict, s.delta_resolved, s.domain, s.rule
            );
        }
    }
}

fn group_by<'a>(
    rows: &'a [BlueprintRow],
    value_of: impl Fn(&'a BlueprintRow) -> String,
) -> Vec<(String, Vec<&'a BlueprintRow>)> {
    let mut map: BTreeMap<String, Vec<&BlueprintRow>> = BTreeMap::new();
    for r in rows {
        map.entry(value_of(r)).or_default().push(r);
    }
    map.into_iter().collect()
}

#[cfg(test)]
#[path = "blueprint_tests.rs"]
mod tests;

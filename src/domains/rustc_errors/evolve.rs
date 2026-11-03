//! Phase 4 self-evolution wiring for `--fix-compile` (Plan 019 T4.1 +
//! T4.4) — recording driver outcomes into the [`EvolveRecorder`] memory
//! and dumping misses into the authoring queue.
//!
//! ## The trajectory mapping (the plan's `lint_name`/`rule_id` contract)
//!
//! The plan fixes `lint_name := error_code`, `rule_id := strategy_id` so
//! the memory layer works unmodified. [`FixTrajectory`] carries ONE id,
//! so the compile domain keys trajectories on `"{code}:{strategy_id}"`
//! (`"E0502:e0502-deref-removal"`) — both halves of the contract in one
//! bucket key, and the strategy ids are already family-prefixed so plain
//! greps stay unambiguous either way.
//!
//! ## Verdict soundness (the strongest tier we have)
//!
//! - **applied** → `record_outcome_verified(success = true, verified =
//!   true)`: the driver's keep criterion IS a full-oracle verdict — cargo
//!   check passed on the real crate with strictly fewer coded errors.
//!   That is a Certified trajectory (the only promotion path, Issue 021).
//! - **reverted** → `record_outcome(success = false)`: the oracle rejected
//!   the move at this shape — a plain failure (counts toward the Withdrawn
//!   streak; never certifies).
//!
//! ## Miss capture (T4.4 — the corpus-growth loop)
//!
//! A miss (no playbook proposed, or the playbooks' edits all reverted) is
//! the authoring queue for the NEXT template: `<repo>/.refine/misses/
//! {code}-{ts}.md` with the rendered diagnostic verbatim + a source
//! excerpt. The `wgsl_mine` precedent — mine fix-pairs out of history;
//! here history is handed to you pre-packaged.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::driver::{FixCompileReport, MissRecord};
use crate::dispersion::{from_counts, frontier_order, OutcomeClass, PoolEntry};
use crate::recorder::EvolveRecorder;

/// Trajectory bucket key: `"{code}:{strategy_id}"` (the plan's
/// `lint_name`/`rule_id` contract folded into the memory's single id).
#[must_use]
pub fn traj_key(code: &str, strategy_id: &str) -> String {
    format!("{code}:{strategy_id}")
}

/// Record a completed `--fix-compile --write` run into `rec`.
///
/// Review-mode reports record NOTHING (the Issue 018 rule — only edits
/// that were actually written are trajectories), so this is only called
/// from the write path. Reverts are component-attributed at record time
/// (Issue 047): a no-progress revert means the drafted fix was defective
/// (W); an I/O revert means the gate rejected without evaluating the fix
/// (C — an unnecessary, gate-side revert).
pub fn record_run(rec: &mut EvolveRecorder, report: &FixCompileReport) {
    // Issue 105: the run's journey credit, computed once — every kept
    // attempt in THIS run rode the same journey. `tainted` runs (ended at
    // a masking-stage error) stamp their composite; clean runs bump the
    // journey count alone.
    let journey = report.journey();
    let taint = journey.tainted().then_some(journey.taint_composite_256());
    for a in &report.applied {
        rec.record_outcome_verified(
            &traj_key(&a.code, &a.strategy_id),
            &a.shape,
            &a.fix,
            true,
            true, // cargo-check-passed = a full-oracle verdict
        );
        rec.memory_record_journey(&traj_key(&a.code, &a.strategy_id), &a.fix, taint);
    }
    for r in &report.reverted {
        let signal = match r.reason {
            super::driver::RevertReason::NoProgress => {
                crate::component::FailureSignal::reverted(crate::component::RevertKind::CompileGate)
            }
            super::driver::RevertReason::Io => crate::component::FailureSignal::reverted(
                crate::component::RevertKind::Infrastructure,
            ),
        };
        rec.record_failure_signal(
            &traj_key(&r.code, &r.strategy_id),
            &r.shape,
            &r.fix,
            &signal,
        );
    }
}

/// Where misses land: `<root>/.refine/misses/`. `root` is the workspace (or
/// package) root the caller resolved for the run.
#[must_use]
pub fn misses_dir(root: &Path) -> PathBuf {
    root.join(".refine").join("misses")
}

/// Write one miss as a self-contained authoring-queue entry.
///
/// # Errors
///
/// I/O — the caller warns and continues (a miss dump is an aid, never a
/// gate).
pub fn write_miss(dir: &Path, miss: &MissRecord) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = dir.join(format!("{}-{ts}.md", miss.code));
    let rendered = miss.rendered.as_deref().unwrap_or(&miss.message);
    let body = format!(
        "# {code} — unresolved by the rustc_errors playbooks\n\n \
         - **file:** {file}:{line}\n\
         - **message:** {msg}\n\
         - **component:** {component} — no candidate answered this error (corpus gap; author the next template here)\n\
         - **captured:** by `cargo refine --fix-compile --write` (no candidate kept)\n\n \
         ## Rendered diagnostic\n\n```text\n{rendered}\n```\n",
        code = miss.code,
        file = miss.file.display(),
        line = miss.line,
        msg = miss.message,
        component = miss.component,
    );
    std::fs::write(&path, body)?;
    Ok(path)
}

/// The standing README for the misses queue (idempotent — written only
/// when absent so its content stays the documented contract, not a
/// growing log).
///
/// # Errors
///
/// I/O — caller warns and continues.
pub fn write_misses_readme(dir: &Path) -> std::io::Result<()> {
    let readme = dir.join("README.md");
    if readme.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        readme,
        "# The rustc_errors miss queue\n\n\
         Every file here is a compile error the shipped playbooks could not\n\
         fix (no candidate proposed, or every candidate reverted). Each is a\n\
         self-contained authoring request: the rendered diagnostic + location.\n\
         The queue is the input for the next playbook template — grab one,\n\
         reproduce it, probe what rustc actually labels (`cargo check\n\
         --message-format=json`), and author the proposer + fixtures per the\n\
         oracle-first discipline (every FIXED form probe-verified to compile\n\
         BEFORE becoming a fixture).\n\n\
         Files are never edited in place; delete one once its template ships.\n",
    )
}

/// Dump every miss of a report into `root`'s queue. Returns the paths
/// written (empty when there were no misses). Failures are collected as
/// warnings — a miss dump is an aid, never a gate.
#[must_use]
pub fn dump_misses(root: &Path, misses: &[MissRecord]) -> (Vec<PathBuf>, Vec<String>) {
    if misses.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let dir = misses_dir(root);
    let mut written = Vec::with_capacity(misses.len());
    let mut warnings = Vec::new();
    if let Err(e) = write_misses_readme(&dir) {
        warnings.push(format!("misses README: {e}"));
    }
    for m in misses {
        match write_miss(&dir, m) {
            Ok(p) => written.push(p),
            Err(e) => warnings.push(format!("miss {}:{}: {e}", m.file.display(), m.line)),
        }
    }
    (written, warnings)
}

// ── Issue 91 T1.5: the per-attempt ring's dump ──────────────────────────

/// Where per-run attempt sequences land (Issue 91 T1.5):
/// `<root>/.refine/fixseq/` — the `misses` sibling.
#[must_use]
pub fn fixseq_dir(root: &Path) -> PathBuf {
    root.join(".refine").join("fixseq")
}

/// The standing README for the fixseq store (idempotent — the misses
/// law: written only when absent, the documented contract).
///
/// # Errors
///
/// I/O — caller warns and continues.
pub fn write_fixseq_readme(dir: &Path) -> std::io::Result<()> {
    let readme = dir.join("README.md");
    if readme.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        readme,
        "# The fix-compile attempt ring (fixseq)\n\n\
         One `run-<ts>.jsonl` per `cargo refine --fix-compile --write` run: the\n\
         ORDERED per-attempt sequence (seq, error code, span_hash, strategy,\n\
         outcome, errors before→after) as jsonl lines. The trajectory store\n\
         holds per-FAMILY counters — attempt ORDER lives only here.\n\n\
         Consumers: the Issue 91 T2 oscillation detector (same `(code,\n\
         span_hash)` returning after an intervening fix) and the Issue 093\n\
         per-run strategy posterior (the strategy sequence per run).\n\
         Append-only; never edited in place.\n",
    )
}

/// Dump one run's ordered attempt ring as a single jsonl file (Issue 91
/// T1.5). Returns the path written (`None` when the run made no attempts —\n\
/// review mode / clean first check / dump failure). Failures → warnings,
/// never a gate (the `dump_misses` law).
#[must_use]
pub fn dump_fixseq(
    root: &Path,
    attempts: &[super::driver::AttemptRecord],
) -> (Option<PathBuf>, Vec<String>) {
    if attempts.is_empty() {
        return (None, Vec::new());
    }
    let dir = fixseq_dir(root);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return (None, vec![format!("fixseq dir: {e}")]);
    }
    let mut warnings = Vec::new();
    if let Err(e) = write_fixseq_readme(&dir) {
        warnings.push(format!("fixseq README: {e}"));
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = dir.join(format!("run-{nanos}.jsonl"));
    // Issue 105: stamp the run-level journey credit onto every row (the
    // run's λ_T raw integral + terminal residual — the values every row of
    // THIS run shares). In-memory records carry 0s here; the dumped ring
    // is where the values become durable (pre-105 rings decode as 0 =
    // absent, never a replay hazard).
    let credit = super::driver::JourneyCredit::from_attempts(attempts);
    let stamped: Vec<super::driver::AttemptRecord> = attempts
        .iter()
        .map(|a| {
            let mut row = a.clone();
            row.journey_residual_sum = credit.journey_residual_sum;
            row.terminal_residual = credit.terminal_residual;
            row
        })
        .collect();
    let mut body = String::with_capacity(attempts.len() * 160);
    for a in &stamped {
        match serde_json::to_string(a) {
            Ok(line) => {
                body.push_str(&line);
                body.push('\n');
            }
            Err(e) => warnings.push(format!("fixseq serialize seq={}: {e}", a.seq)),
        }
    }
    match std::fs::write(&path, body) {
        Ok(()) => (Some(path), warnings),
        Err(e) => (None, vec![format!("fixseq write: {e}")]),
    }
}

// ── Issue 072 T5 (Plan 092): the frontier-ordered authoring queue ───────

/// The error code of a queue file name (`E0502-1690000000.md` → `E0502`).
/// `None` for the README and anything that does not match the shape.
#[must_use]
pub fn parse_miss_code(file_name: &str) -> Option<&str> {
    let stem = file_name.strip_suffix(".md")?;
    if stem.eq_ignore_ascii_case("README") {
        return None;
    }
    let idx = stem.rfind('-')?;
    let code = &stem[..idx];
    if code.is_empty() {
        None
    } else {
        Some(code)
    }
}

/// Aggregate per-code outcome counts from trajectory rows
/// (`"{code}:{strategy}"` keys → `(kept, reverted)` per code). Pure — the
/// CLI feeds it from the store (`dump()`) or the JSON envelope (`iter()`).
#[must_use]
pub fn code_outcomes_from_rows(
    rows: impl Iterator<Item = (String, u32, u32)>,
) -> BTreeMap<String, (usize, usize)> {
    let mut out: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (key, success, fail) in rows {
        let code = key.split(':').next().unwrap_or_default();
        if code.is_empty() {
            continue;
        }
        let entry = out.entry(code.to_string()).or_default();
        entry.0 += success as usize;
        entry.1 += fail as usize;
    }
    out
}

/// One rendered queue row (exposed for tests).
#[derive(Debug, Clone, PartialEq)]
pub struct QueueRow {
    pub rank: usize,
    pub code: String,
    pub misses: usize,
    pub kept: usize,
    pub reverted: usize,
    pub entropy: f64,
    pub regime: crate::dispersion::Regime,
}

/// The frontier-ordered authoring queue (issue 072 T4's control law wired
/// into the `.refine/misses/` loop): each queue code is a candidate pool whose
/// outcomes are the store's kept/reverted history for `{code}:*` plus ONE
/// Missed per queue occurrence; [`frontier_order`] puts informative pools
/// (mixed evidence — resolving them converts the most) first, saturated last.
/// This is the real-G8 instrument: authoring from the top of this report vs
/// the historical FIFO order is the Certified-conversion A/B (the gate shape
/// lives in `.issues/072`; the measurement starts when the queue is
/// non-empty).
#[must_use]
pub fn frontier_queue_report(
    dir: &Path,
    code_outcomes: &BTreeMap<String, (usize, usize)>,
) -> String {
    // Collect queue occurrences per code (sorted file names → stable order).
    let mut occurrences: BTreeMap<String, usize> = BTreeMap::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.to_ascii_lowercase().ends_with(".md") {
                continue;
            }
            if let Some(code) = parse_miss_code(&name) {
                *occurrences.entry(code.to_string()).or_default() += 1;
            }
        }
    }
    if occurrences.is_empty() {
        return format!(
            "# misses queue — frontier order\n\n(empty: no unresolved misses at {})\n",
            dir.display()
        );
    }
    let pool: Vec<PoolEntry> = occurrences
        .iter()
        .map(|(code, misses)| {
            let (kept, reverted) = code_outcomes.get(code).copied().unwrap_or((0, 0));
            PoolEntry::new(code.clone(), from_counts(kept, reverted, *misses))
        })
        .collect();
    let ordered = frontier_order(&pool);
    let by_code: BTreeMap<&str, &PoolEntry> = pool.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut rows: Vec<QueueRow> = Vec::with_capacity(ordered.len());
    for (rank, verdict) in ordered.iter().enumerate() {
        let entry = by_code[verdict.id.as_str()];
        let (kept, reverted) = code_outcomes
            .get(verdict.id.as_str())
            .copied()
            .unwrap_or((0, 0));
        rows.push(QueueRow {
            rank: rank + 1,
            code: verdict.id.clone(),
            misses: entry
                .outcomes
                .iter()
                .filter(|o| **o == OutcomeClass::Missed)
                .count(),
            kept,
            reverted,
            entropy: verdict.entropy,
            regime: verdict.regime,
        });
    }
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# misses queue — frontier order ({} code(s))\n",
        rows.len()
    );
    out.push_str(
        "Author the TOP entry first: informative pools convert the most \
         uncertain candidates per authored template (issue 072 T4's control \
         law; the real-G8 A/B vs the FIFO historical order is the conversion \
         column after each template lands).\n\n",
    );
    out.push_str("| rank | code | misses | kept | reverted | entropy | regime | conversion |\n");
    out.push_str("|---|---|---|---|---|---|---|---|\n");
    for r in &rows {
        let conv = if r.kept + r.reverted == 0 {
            "-".to_string()
        } else {
            #[allow(clippy::cast_precision_loss)]
            let pct = (r.kept as f64) / ((r.kept + r.reverted) as f64) * 100.0;
            format!("{pct:.0}%")
        };
        let regime = r.regime.label();
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {:.3} | {regime} | {conv} |",
            r.rank, r.code, r.misses, r.kept, r.reverted, r.entropy
        );
    }
    out
}

// ── Plan 091 T1.3 — the conversion ledger (the real-G8 instrument) ────

/// The `.refine/misses/` frontier-ordering commit (Plan 091 T1.1/T1.2) —
/// the FIFO/frontier window boundary. Everything before it is the FIFO
/// era; the authoring queue orders by dispersion evidence from here on.
pub const WIRING_COMMIT: &str = "3580477";

/// Unix seconds of [`WIRING_COMMIT`] (2026-09-06 14:38:36 +07). Same units
/// as `TrajRowDoc::last_seen_at`, so era splits are a plain comparison.
pub const WIRING_UNIX_SECS: u64 = 1_788_680_316;

/// One post-wiring authored template: a `FRONTIER_LANDINGS` row. Append one
/// IN THE SAME COMMIT as the template's playbook lands — the pins-file
/// discipline. A landing without its row reds
/// `wired_plus_frontier_equals_the_derived_inventory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontierLanding {
    /// The landing commit (short hash).
    pub commit: &'static str,
    /// Unix seconds of `commit`.
    pub unix_secs: u64,
    /// The addressed error code (`"E0601"`).
    pub code: &'static str,
    /// The playbook strategy id (`CompileProposer::name`).
    pub strategy: &'static str,
}

/// The playbook strategies authored BEFORE the wiring — the FIFO-baseline
/// era (E0597 family = Phase 2, Bench 041/045; borrowck family = Phase 3).
/// Membership is the authoring-era attribution: these rules' conversions
/// belong to the pre-wiring history no matter when they were re-verified.
pub const WIRED_STRATEGIES: &[(&str, &str)] = &[
    ("E0597", "e0597-anon-lifetime"),
    ("E0597", "e0597-move"),
    ("E0597", "e0597-move-clone"),
    ("E0597", "e0597-drop-the-box"),
    ("E0597", "e0597-shared-ownership"),
    ("E0597", "e0597-scope-hoist"),
    ("E0502", "e0502-deref-removal"),
    ("E0502", "e0502-clone-out"),
    ("E0499", "e0499-swap-second-decl"),
    ("E0505", "e0505-clone-at-move"),
];

/// Post-wiring authored templates, one row per landing. EMPTY at the wiring
/// commit — the frontier era has no conversions to attribute yet, which is
/// the honest starting state of the G8 measurement. First landing: Issue
/// 100's `e0614-deref-at-use` (the unmasking-trade playbook; `commit` cites
/// the issue-file commit that scoped it — informational, the era attribution
/// keys on `unix_secs`).
pub const FRONTIER_LANDINGS: &[FrontierLanding] = &[
    FrontierLanding {
        commit: "8ca7f68c",
        unix_secs: 1_789_268_622,
        code: "E0614",
        strategy: "e0614-deref-at-use",
    },
    FrontierLanding {
        // The issue-file commit that scoped it (informational; era keys on
        // unix_secs — the same convention as the row above).
        commit: "963da3c9",
        unix_secs: 1_789_276_000,
        code: "E0502",
        strategy: "e0502-read-relocate",
    },
    FrontierLanding {
        // riir-train plan 432's corpus-expansion landing (the diversity
        // tripwire): the E0505 ladder's second rung — clone-at-move reverts
        // on non-`Clone` types, this recovers.
        commit: "da59eead",
        unix_secs: 1_791_120_874,
        code: "E0505",
        strategy: "e0505-use-before-move",
    },
    FrontierLanding {
        // Same landing: E0382's first rung (the clone side of the ladder).
        commit: "da59eead",
        unix_secs: 1_791_120_874,
        code: "E0382",
        strategy: "e0382-clone-at-move",
    },
    FrontierLanding {
        // Same landing: E0382's second rung (the statement swap).
        commit: "da59eead",
        unix_secs: 1_791_120_874,
        code: "E0382",
        strategy: "e0382-use-before-move",
    },
];

/// Which authoring era a playbook strategy belongs to. `Unrecorded` is a
/// ledger bug (a registry strategy no table accounts for) — it prints
/// loudly and the ratchet test makes it unrepresentable in-tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEra {
    /// Authored pre-wiring (`WIRED_STRATEGIES`) — the FIFO baseline.
    Baseline,
    /// Authored post-wiring (`FRONTIER_LANDINGS`) — the frontier era.
    Frontier,
    /// In neither table.
    Unrecorded,
}

/// The playbook inventory as `(code, strategy)` pairs, derived from the
/// production registry via `CompileProposer::code` — the registry IS the
/// source of truth; nothing is name-prefix-parsed.
#[must_use]
pub fn playbook_inventory() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = super::playbooks::Playbooks::production()
        .into_entries()
        .into_iter()
        .map(|p| (p.code().to_string(), p.name().to_string()))
        .collect();
    out.sort();
    out
}

/// One driver-outcome signal for the ledger. The CLI maps store rows into
/// this so `evolve.rs` stays feature-free (the T1.1 convention — the
/// store-join lives behind the caller's feature gate).
#[derive(Debug, Clone)]
pub struct ConversionSignal {
    /// The trajectory key half `{code}:{strategy_id}` (rows are keyed per
    /// `(key, shape)`, so one strategy may carry many rows).
    pub lint_key: String,
    /// The row's `EvidenceTier::as_u8()` value.
    pub tier: u8,
    /// The row's `last_seen_at` (unix seconds; `0` = never stamped).
    pub last_seen_unix: u64,
}

/// One playbook strategy's conversion state in the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionRow {
    pub code: String,
    pub strategy: String,
    pub era: LedgerEra,
    /// The strategy's CURRENT tier is `Certified` — the conversion
    /// definition. A once-certified-then-withdrawn rule is RETRACTED
    /// (Issue 021: absorbing, never re-emitted), not a live conversion.
    pub certified: bool,
    /// The latest `last_seen_unix` over the strategy's rows, when any row
    /// exists. `None` = the strategy has never run against a driver.
    pub last_seen_unix: Option<u64>,
}

/// The G8 report: conversion-per-authored-rule, split by authoring era.
/// The FIFO arm is HISTORICAL (not re-runnable post-wiring) — the
/// comparison is frontier-vs-its-own-baseline (the T39/Bench-051
/// discipline); `WIRING_COMMIT` pins the window so the baseline stays
/// unambiguous.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConversionReport {
    /// Baseline-era rules with a Certified row / total baseline rules.
    pub baseline_certified: usize,
    pub baseline_total: usize,
    /// Frontier-era conversions / total frontier rules.
    pub frontier_certified: usize,
    pub frontier_total: usize,
    /// Registry strategies accounted in NEITHER table (a ledger bug).
    pub unrecorded: Vec<(String, String)>,
    /// Per-strategy rows, sorted by `(code, strategy)`.
    pub rows: Vec<ConversionRow>,
}

/// Build the conversion report. `wired`/`frontier` are parameters (not the
/// consts directly) so tests can exercise eras synthetically; the CLI
/// passes [`WIRED_STRATEGIES`] and [`FRONTIER_LANDINGS`].
#[must_use]
pub fn conversion_report(
    inventory: &[(String, String)],
    wired: &[(&str, &str)],
    frontier: &[FrontierLanding],
    signals: &[ConversionSignal],
) -> ConversionReport {
    // Best state per strategy key: Certified outranks everything (any
    // Certified row converts the strategy); `last_seen` is the max.
    let mut certified: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut last_seen: std::collections::BTreeMap<&str, u64> = std::collections::BTreeMap::new();
    for s in signals {
        let key: &str = &s.lint_key;
        if s.tier == crate::memory::EvidenceTier::Certified as u8 {
            certified.insert(key);
        }
        let e = last_seen.entry(key).or_insert(0);
        *e = (*e).max(s.last_seen_unix);
    }

    let era_of = |code: &str, strategy: &str| -> LedgerEra {
        if wired.iter().any(|&(c, s)| c == code && s == strategy) {
            LedgerEra::Baseline
        } else if frontier
            .iter()
            .any(|l| l.code == code && l.strategy == strategy)
        {
            LedgerEra::Frontier
        } else {
            LedgerEra::Unrecorded
        }
    };

    let mut report = ConversionReport::default();
    for (code, strategy) in inventory {
        let key = format!("{code}:{strategy}");
        let era = era_of(code, strategy);
        let certified_hit = certified.contains(key.as_str());
        let last = last_seen.get(key.as_str()).copied().filter(|&t| t > 0);
        report.rows.push(ConversionRow {
            code: code.clone(),
            strategy: strategy.clone(),
            era,
            certified: certified_hit,
            last_seen_unix: last,
        });
        match era {
            LedgerEra::Baseline => {
                report.baseline_total += 1;
                if certified_hit {
                    report.baseline_certified += 1;
                }
            }
            LedgerEra::Frontier => {
                report.frontier_total += 1;
                if certified_hit {
                    report.frontier_certified += 1;
                }
            }
            LedgerEra::Unrecorded => {
                report.unrecorded.push((code.clone(), strategy.clone()));
            }
        }
    }
    report
        .rows
        .sort_by(|a, b| a.code.cmp(&b.code).then(a.strategy.cmp(&b.strategy)));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traj_key_folds_the_contract() {
        assert_eq!(
            traj_key("E0502", "e0502-deref-removal"),
            "E0502:e0502-deref-removal"
        );
    }

    fn miss(code: &str) -> MissRecord {
        MissRecord {
            code: code.to_string(),
            file: PathBuf::from("/x/src/lib.rs"),
            line: 7,
            message: "cannot borrow".to_string(),
            rendered: Some("error[E0502]...rendered".to_string()),
            component: crate::component::classify(
                &crate::component::FailureSignal::miss_no_candidates(),
            ),
        }
    }

    #[test]
    fn dump_writes_entries_and_readme_once() {
        let root =
            std::env::temp_dir().join(format!("riir_p19_miss_{}_{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        let m = miss("E0502");
        let (written, warns) = dump_misses(&root, std::slice::from_ref(&m));
        assert!(warns.is_empty(), "{warns:?}");
        assert_eq!(written.len(), 1);
        assert!(written[0]
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("E0502-"));
        let body = std::fs::read_to_string(&written[0]).unwrap();
        assert!(
            body.contains("error[E0502]...rendered"),
            "rendered verbatim"
        );
        assert!(body.contains("/x/src/lib.rs:7"));
        assert!(
            body.contains("**component:** E (corpus/proposer)"),
            "Issue 047 attribution line present: {body}"
        );
        assert!(root
            .join(".refine")
            .join("misses")
            .join("README.md")
            .exists());

        // Second dump: README NOT rewritten (idempotent), new entry lands.
        let m2 = miss("E0499");
        let (written2, warns2) = dump_misses(&root, std::slice::from_ref(&m2));
        assert_eq!(warns2, Vec::<String>::new());
        assert_eq!(written2.len(), 1);
        let n: usize = std::fs::read_dir(root.join(".refine").join("misses"))
            .unwrap()
            .filter(|e| {
                e.as_ref().is_ok_and(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    name.to_ascii_lowercase().ends_with(".md") && name != "README.md"
                })
            })
            .count();
        assert_eq!(n, 2, "two entries, one README");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Issue 91 T1.5 — the fixseq dump: one ordered jsonl per run, the
    /// README once, nothing on an attempt-less run.
    #[test]
    fn dump_fixseq_writes_ordered_jsonl_and_readme_once() {
        use crate::domains::rustc_errors::driver::{AttemptOutcome, AttemptRecord};
        let root =
            std::env::temp_dir().join(format!("riir_91_fixseq_{}_{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);

        // Attempt-less: no file, no warnings, no dir creation.
        let (none, warns0) = dump_fixseq(&root, &[]);
        assert!(none.is_none() && warns0.is_empty());
        assert!(!root.join(".refine").join("fixseq").exists());

        let attempts = vec![
            AttemptRecord {
                seq: 0,
                code: "E0502".to_string(),
                span_hash: "aaaa1111aaaa1111".to_string(),
                strategy_id: "e0502-deref-removal".to_string(),
                outcome: AttemptOutcome::Kept,
                errors: (3, 2),
                masking: (0, 0),
                journey_residual_sum: 0,
                terminal_residual: 0,
                // Issue 107 T2 — a REAL embedding on the first record so the
                // dump round-trip pins the column end-to-end.
                span_embed: Some(crate::span_embed::span_embed("let r = v.as_str();")),
                shape_class: None,
                insight: None,
            },
            AttemptRecord {
                seq: 1,
                code: "E0507".to_string(),
                span_hash: "bbbb2222bbbb2222".to_string(),
                strategy_id: "e0507-move-out".to_string(),
                outcome: AttemptOutcome::RevertedNoProgress,
                errors: (2, 2),
                masking: (0, 0),
                journey_residual_sum: 0,
                terminal_residual: 0,
                span_embed: None,
                shape_class: None,
                insight: None,
            },
        ];
        let (path, warns) = dump_fixseq(&root, &attempts);
        assert!(warns.is_empty(), "{warns:?}");
        let path = path.expect("run file written");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        // Case-sensitive ON PURPOSE: this asserts dump_fixseq's OWN on-disk
        // name round-trip (lowercase `.jsonl` is what the writer emits).
        assert!(name.starts_with("run-"), "{name}");
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("jsonl"),
            "{name}"
        );

        // The ORDERED sequence round-trips, line per attempt — with the
        // Issue-105 run-level stamps the dump adds (every row of THIS run
        // carries the run's integral + terminal residual; r = masking_after
        // + errors_after: attempt 0 leaves (0,2)→2, attempt 1 leaves
        // (0,2)→2 ⇒ sum 4, terminal 2).
        let body = std::fs::read_to_string(&path).unwrap();
        // Issue 107 T2 — the column lands in the dumped jsonl verbatim.
        assert!(
            body.lines()
                .next()
                .is_some_and(|l| l.contains("\"span_embed\":")),
            "first dumped row carries the embedding column: {}",
            body.lines().next().unwrap_or("")
        );
        let back: Vec<AttemptRecord> = body
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("line parses"))
            .collect();
        assert_eq!(back.len(), attempts.len());
        for (b, a) in back.iter().zip(&attempts) {
            assert_eq!(
                (b.seq, &b.code, b.outcome, b.errors, b.masking),
                (a.seq, &a.code, a.outcome, a.errors, a.masking)
            );
            assert_eq!(b.journey_residual_sum, 4, "run integral stamped");
            assert_eq!(b.terminal_residual, 2, "terminal residual stamped");
        }

        // The README landed once (idempotent on a second dump).
        let dir = root.join(".refine").join("fixseq");
        assert!(dir.join("README.md").exists());
        let (_, warns2) = dump_fixseq(&root, &attempts[..1]);
        assert_eq!(warns2, Vec::<String>::new());
        let readme = std::fs::read_to_string(dir.join("README.md")).unwrap();
        assert_eq!(readme.matches("# The fix-compile attempt ring").count(), 1);
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            3,
            "README + two run files"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dump_on_no_misses_touches_nothing() {
        let root = std::env::temp_dir().join(format!("riir_p19_miss_none_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (written, warns) = dump_misses(&root, &[]);
        assert!(written.is_empty() && warns.is_empty());
        assert!(!root.exists(), "no dir created for an empty queue");
    }

    #[test]
    fn record_run_buckets_by_code_and_strategy() {
        let mut rec = EvolveRecorder::new();
        let report = FixCompileReport {
            applied: vec![super::super::driver::AppliedRecord {
                file: PathBuf::from("/x/a.rs"),
                line: 3,
                code: "E0502".to_string(),
                strategy_id: "e0502-deref-removal".to_string(),
                errors: (1, 0),
                shape: "&v[0]".to_string(),
                fix: String::new(),
                semantic_note: None,
                rejected: Vec::new(),
            }],
            reverted: vec![super::super::driver::RevertedRecord {
                file: PathBuf::from("/x/a.rs"),
                line: 3,
                code: "E0502".to_string(),
                strategy_id: "e0502-clone-out".to_string(),
                reason: super::super::driver::RevertReason::NoProgress,
                shape: "&v[0]".to_string(),
                fix: "v[0].clone()".to_string(),
                semantic_note: None,
                rejected: Vec::new(),
            }],
            ..FixCompileReport::default()
        };
        record_run(&mut rec, &report);
        assert_eq!(rec.recorded_count(), 2);
        let mem = rec.memory();
        let applied = mem
            .iter()
            .find(|t| t.lint_name == "E0502:e0502-deref-removal")
            .expect("applied trajectory keyed by code:strategy");
        assert_eq!(applied.success_count, 1, "verified success counted");
        let reverted = mem
            .iter()
            .find(|t| t.lint_name == "E0502:e0502-clone-out")
            .expect("reverted trajectory keyed by code:strategy");
        assert_eq!(reverted.fail_count, 1, "oracle rejection counted");
    }

    // ── Plan 091 T1.3 — the conversion ledger ──────────────────────────

    /// THE RATCHET: the derived registry inventory must be exactly the
    /// wired baseline union the frontier landings — no more, no less, no
    /// duplicates. A playbook landing without its `FRONTIER_LANDINGS` row
    /// reds here; a ledger row outliving its playbook reds here; a rename
    /// reds here. Bump the tables in the SAME commit as the registry.
    #[test]
    fn wired_plus_frontier_equals_the_derived_inventory() {
        let derived = playbook_inventory();
        let derived_set: std::collections::BTreeSet<(String, String)> =
            derived.iter().cloned().collect();
        assert_eq!(
            derived.len(),
            derived_set.len(),
            "duplicate (code, strategy) in the registry inventory"
        );
        let mut accounted: std::collections::BTreeSet<(String, String)> = WIRED_STRATEGIES
            .iter()
            .map(|&(c, s)| (c.to_string(), s.to_string()))
            .collect();
        for l in FRONTIER_LANDINGS {
            accounted.insert((l.code.to_string(), l.strategy.to_string()));
        }
        assert_eq!(
            derived_set, accounted,
            "ledger drift: append a FRONTIER_LANDINGS row in the SAME commit \
             as the playbook landing (or update WIRED_STRATEGIES on a rename)"
        );
    }

    #[test]
    fn conversion_report_attributes_certified_rows_by_era() {
        let inventory = vec![
            ("E0001".to_string(), "old-strat".to_string()),
            ("E0002".to_string(), "new-strat".to_string()),
            ("E0003".to_string(), "idle-strat".to_string()),
        ];
        let wired = [("E0001", "old-strat"), ("E0003", "idle-strat")];
        let frontier = [FrontierLanding {
            commit: "abc1234",
            unix_secs: 1_788_680_316,
            code: "E0002",
            strategy: "new-strat",
        }];
        // Multiple rows per key (one per shape): the second Certified row
        // is redundant, not a second conversion.
        let signals = vec![
            ConversionSignal {
                lint_key: "E0001:old-strat".to_string(),
                tier: crate::memory::EvidenceTier::Certified as u8,
                last_seen_unix: 1_000,
            },
            ConversionSignal {
                lint_key: "E0001:old-strat".to_string(),
                tier: crate::memory::EvidenceTier::Certified as u8,
                last_seen_unix: 2_000,
            },
            ConversionSignal {
                lint_key: "E0002:new-strat".to_string(),
                tier: crate::memory::EvidenceTier::Heuristic as u8,
                last_seen_unix: 3_000,
            },
        ];
        let report = conversion_report(&inventory, &wired, &frontier, &signals);
        assert_eq!(report.baseline_total, 2, "old-strat + idle-strat");
        assert_eq!(report.baseline_certified, 1, "any Certified row converts");
        assert_eq!(report.frontier_total, 1);
        assert_eq!(
            report.frontier_certified, 0,
            "Heuristic tier is a sighting, not a conversion"
        );
        assert_eq!(report.unrecorded, []);
        // last_seen takes the MAX over the strategy's rows.
        let old = report
            .rows
            .iter()
            .find(|r| r.strategy == "old-strat")
            .unwrap();
        assert_eq!(old.last_seen_unix, Some(2_000));
    }

    #[test]
    fn conversion_report_counts_only_certified_tier_as_conversion() {
        let inventory = vec![("E0001".to_string(), "s".to_string())];
        let wired = [("E0001", "s")];
        // Withdrawn (once certified, then retracted — Issue 021 absorbing)
        // is NOT a live conversion.
        let signals = vec![ConversionSignal {
            lint_key: "E0001:s".to_string(),
            tier: crate::memory::EvidenceTier::Withdrawn as u8,
            last_seen_unix: 5_000,
        }];
        let report = conversion_report(&inventory, &wired, frontier_dummy(), &signals);
        assert_eq!(report.baseline_certified, 0);
        let row = &report.rows[0];
        assert!(!row.certified);
        assert_eq!(row.last_seen_unix, Some(5_000), "sighting still recorded");
    }

    fn frontier_dummy() -> &'static [FrontierLanding] {
        &[]
    }

    #[test]
    fn conversion_report_flags_unrecorded_strategies() {
        let inventory = vec![
            ("E0001".to_string(), "known".to_string()),
            ("E0009".to_string(), "orphan".to_string()),
        ];
        let report = conversion_report(&inventory, &[("E0001", "known")], &[], &[]);
        assert_eq!(
            report.unrecorded,
            vec![("E0009".to_string(), "orphan".to_string())]
        );
    }

    #[test]
    fn conversion_report_is_deterministic() {
        let inventory = playbook_inventory();
        let signals = vec![ConversionSignal {
            lint_key: "E0505:e0505-clone-at-move".to_string(),
            tier: crate::memory::EvidenceTier::Certified as u8,
            last_seen_unix: 1_788_680_400,
        }];
        let a = conversion_report(&inventory, WIRED_STRATEGIES, FRONTIER_LANDINGS, &signals);
        let b = conversion_report(&inventory, WIRED_STRATEGIES, FRONTIER_LANDINGS, &signals);
        assert_eq!(a, b);
        // The production wiring: the derived inventory IS fully accounted
        // (rows carry no Unrecorded era in-tree).
        assert!(a.unrecorded.is_empty(), "in-tree ledger must be complete");
    }

    // ── Plan 092 T5 (the collision merge) — the frontier queue report ──

    #[test]
    fn parse_miss_code_reads_the_dump_shape() {
        assert_eq!(parse_miss_code("E0502-1690000000.md"), Some("E0502"));
        assert_eq!(parse_miss_code("E0499-42.md"), Some("E0499"));
        assert_eq!(parse_miss_code("README.md"), None);
        assert_eq!(parse_miss_code("readme.md"), None);
        assert_eq!(parse_miss_code("-169.md"), None, "empty code rejected");
        assert_eq!(parse_miss_code("nodash.txt"), None);
    }

    #[test]
    fn code_outcomes_aggregate_by_the_key_prefix() {
        let rows = [
            ("E0502:e0502-deref-removal".to_string(), 3, 1),
            ("E0502:e0502-clone-out".to_string(), 0, 2),
            ("E0499:swap".to_string(), 1, 0),
        ];
        let map = code_outcomes_from_rows(rows.into_iter());
        assert_eq!(map.get("E0502"), Some(&(3, 3)));
        assert_eq!(map.get("E0499"), Some(&(1, 0)));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn frontier_report_orders_informative_first_and_renders_columns() {
        let root =
            std::env::temp_dir().join(format!("riir_p92_miss_{}_{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = misses_dir(&root);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "contract").unwrap();
        std::fs::write(dir.join("E0502-1.md"), "a").unwrap();
        std::fs::write(dir.join("E0777-2.md"), "b").unwrap();
        std::fs::write(dir.join("E0999-3.md"), "c").unwrap();
        // E0502: mixed history (informative — 2 kept + 2 reverted + the queue
        // miss); E0777: all-kept store history (4 kept + the queue miss →
        // LOWER entropy but still informative — a fully-saturated pool can
        // never occur in the queue, every entry IS a miss); E0999: no store
        // evidence → one Missed = zero-entropy-all-fail = regenerate.
        let mut outcomes = BTreeMap::new();
        outcomes.insert("E0502".to_string(), (2usize, 2usize));
        outcomes.insert("E0777".to_string(), (4usize, 0usize));
        let report = frontier_queue_report(&dir, &outcomes);
        assert!(report.contains("frontier order"));
        assert!(report.contains("3 code(s)"), "README not counted: {report}");
        // Highest entropy first, then the lower-entropy informative pool,
        // regenerate (zero-entropy-all-fail) last.
        let e0502 = report.find("| 1 | E0502").expect("mixed history first");
        let e0777 = report
            .find("| 2 | E0777")
            .expect("low-entropy informative second");
        let e0999 = report.find("| 3 | E0999").expect("regenerate last");
        assert!(e0502 < e0777 && e0777 < e0999);
        assert!(report.contains("informative") && report.contains("regenerate"));
        assert!(
            report.contains("50%"),
            "conversion column renders: {report}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn frontier_report_empty_queue_is_honest() {
        let root = std::env::temp_dir().join(format!("riir_p92_miss_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let report = frontier_queue_report(&misses_dir(&root), &BTreeMap::new());
        assert!(report.contains("(empty"), "{report}");
        let _ = std::fs::remove_dir_all(&root);
    }
}

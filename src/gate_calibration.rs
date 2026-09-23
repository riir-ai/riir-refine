//! Issue 130 T1 + T3 — the admission-time generalization gate's REPORT half
//! (the T0 decision is report-first: nothing here gates, writes, demotes, or
//! touches the heal/mining paths — it makes the gate's evidence exist).
//!
//! From Research 203 (arXiv:2609.24972, "RRSI: Regularized Recursive
//! Self-Improvement of Agent Harnesses"): the evolve-set-overfit signature is
//! already measured in this repo three ways — the Issue 089 T6 provenance
//! strata, the `set_rerank` / `choice_scorer` OOD refutations, and the unmeasured
//! withheld-pair axis. RRSI's mechanisms E and C become, here:
//!
//! - **T1 — δ calibration** ([`calibrate`] / [`render_calibration`]): the
//!   empirical admission floor's evidence report. RRSI admits a rule only when
//!   its gain clears a noise floor δ derived from repeated baseline runs of an
//!   UNCHANGED corpus; the analogue here reads the per-run provenance strata
//!   that `--score-bench` appends to the score history (Issue 089 T6) and
//!   reports, per (stratum, generation), the spread of heal rates across
//!   repeat runs — plus the two ways a stratum can be too thin to support any
//!   δ at all (single run / few findings), either of which defers and points
//!   at "grow the repo fixture corpus first" (Issue 130 Caution 1,
//!   mechanized). Dissimilar generations are never pooled: a class present in
//!   several generations yields one row per generation.
//! - **T3 — the corpus-stall predicate** ([`stall_scan`] / [`render_stall`]):
//!   RRSI mechanism C, `σ_t = 1[progress ≤ δ over window w]`, over the
//!   committed [`FrontierSnapshot`] series — zero generation advances across a
//!   full-enough window flags the coverage-report re-arm in the mining queue
//!   (today that flag is operator-judged). The EMPTY-`store` bucket stays
//!   unattributed and never flags — the snapshot file's own law for legacy
//!   rows, never retro-guessed onto a named store.
//!
//! The score-history row shape consumed by [`stratum_runs`] (confirmed
//! against `.benchmarks/score_history.jsonl`): each `strata` element carries
//! `class`, `n_ran`, `n_healable`, `n_clean`, `vocab_before` (the per-class
//! FINDINGS count — the δ-resolution axis), `resolved`, `created`, and
//! `heal_rate` (= `resolved / vocab_before`). Rows without a strata list (the
//! older history) extract to an empty vec, never an error; a malformed list
//! is a precise error, never silently skipped.
//!
//! Report-first law: both halves are pure functions over caller-supplied
//! values, store-free and side-effect-free, so they are unit-testable at
//! default features — the same standing as the frontier band report.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::frontier_report::FrontierSnapshot;

/// The committed score-history artifact the CLI (`--gate-calibration`) reads
/// for the T1 strata — cwd-relative, the same path `--score-bench` appends
/// to. Typed here (ungated) because the canonical constant lives in the
/// `score_bench` feature's `report` module, which a default-features build
/// does not compile; a path change must touch both (pinned by the CLI's
/// loud-missing refusal, which names this constant).
pub const SCORE_HISTORY_JSONL: &str = ".benchmarks/score_history.jsonl";

/// The committed frontier-snapshot artifact the CLI reads for the T3 stall
/// series — cwd-relative, the file `--frontier-report` and the bench tail
/// append to.
pub const FRONTIER_SNAPSHOTS_JSONL: &str = ".benchmarks/frontier_snapshots.jsonl";

// ── T1 — δ calibration over the provenance strata ──────────────────────────

/// Below this many findings a stratum's heal rate is quantized coarser than
/// any plausible δ: at N findings one flipped finding moves the rate by 1/N,
/// and the `repo` stratum measured 2 findings — rates in {0%, 50%, 100%}, so
/// a cohort improvement has no resolution below 50pp regardless of δ. A
/// stratum under the floor defers ([`CalibrationVerdict::Deferred`]) and the
/// answer is "grow the repo fixture corpus first" (Issue 130 Caution 1,
/// mechanized).
pub const MIN_STRATUM_FINDINGS: usize = 20;

/// A δ needs repeated runs of the UNCHANGED corpus at the same generation to
/// have any empirical content: one run is a point estimate with no spread to
/// measure, so its (stratum, generation) group defers.
pub const MIN_REPEAT_RUNS: usize = 2;

/// One (score-history row, stratum) view — the raw material [`calibrate`]
/// groups. Tolerantly extracted from a row by [`stratum_runs`].
#[derive(Debug, Clone, Deserialize)]
pub struct StratumRunView {
    /// The row's `generation` (measured range 1..=8; u32 for headroom,
    /// matching [`StratumCalibration`] and [`StallRow`]).
    pub generation: u32,
    /// The row's UTC ISO-8601 timestamp (provenance only; the calibration
    /// keys on generation, never on wall-clock).
    pub ts: String,
    /// Stratum label — `repo` / `authored` / `construction` on record.
    pub class: String,
    /// Per-class FINDINGS count — the δ-resolution axis (at N findings one
    /// flip moves the rate by 1/N, see [`MIN_STRATUM_FINDINGS`]).
    pub vocab_before: usize,
    /// Findings the drafter+corpus resolved in this run.
    pub resolved: usize,
    /// `resolved / vocab_before`, as written by the bench.
    pub heal_rate: f32,
}

/// A `serde_json` value's kind, for precise error messages.
fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "list",
        serde_json::Value::Object(_) => "object",
    }
}

/// Read a required string field out of one `strata` element.
fn str_field<'a>(
    element: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
    idx: usize,
) -> Result<&'a str, String> {
    match element.get(key) {
        Some(serde_json::Value::String(s)) => Ok(s),
        Some(other) => Err(format!(
            "strata[{idx}].`{key}` is not a string (found {})",
            json_kind(other)
        )),
        None => Err(format!("strata[{idx}] is missing the `{key}` field")),
    }
}

/// Read a required unsigned-integer field out of one `strata` element.
fn usize_field(
    element: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    idx: usize,
) -> Result<usize, String> {
    match element.get(key) {
        Some(value) => {
            let n = value.as_u64().ok_or_else(|| {
                format!(
                    "strata[{idx}].`{key}` is not an unsigned integer (found {})",
                    json_kind(value)
                )
            })?;
            usize::try_from(n).map_err(|_| format!("strata[{idx}].{key} = {n} does not fit usize"))
        }
        None => Err(format!("strata[{idx}] is missing the `{key}` field")),
    }
}

/// Read a required numeric field out of one `strata` element. The jsonl
/// stores `heal_rate` as an f32-rounded decimal, so the f64→f32 narrowing
/// recovers the written value, never invents one.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn f32_field(
    element: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    idx: usize,
) -> Result<f32, String> {
    match element.get(key) {
        Some(value) => value
            .as_f64()
            .map(|n| n as f32)
            .ok_or_else(|| {
                format!(
                    "strata[{idx}].`{key}` is not a number (found {})",
                    json_kind(value)
                )
            }),
        None => Err(format!("strata[{idx}] is missing the `{key}` field")),
    }
}

/// Tolerantly extract the strata views from one score-history row.
///
/// Missing or empty `strata` (and a `null` list — the older history rows)
/// yield `Ok(vec![])`: absence of instrumentation is not malformed data. A
/// present-but-malformed list is a precise `Err` naming the offending index
/// and field — never silently skipped, because a silently-skipped stratum
/// would read as a clean run.
///
/// # Errors
///
/// A row with no `strata` field (or a null one — the pre-strata history)
/// extracts to an empty vec, NEVER an error: attribution fields are not
/// demanded from rows that carry nothing to attribute. Errors when the row
/// is not a JSON object; when `strata` is present but not a list; or — for
/// a strata-BEARING row — when `generation` / `ts` are missing or of the
/// wrong type, or any element of the list is not an object or is
/// missing/mistypes one of `class` / `vocab_before` / `resolved` /
/// `heal_rate`. The message names the row field or `strata[i].field` at
/// fault.
pub fn stratum_runs(row: &serde_json::Value) -> Result<Vec<StratumRunView>, String> {
    let obj = row
        .as_object()
        .ok_or_else(|| "score-history row is not a JSON object".to_string())?;
    // The strata check runs FIRST: rows from before the strata field existed
    // carry neither `strata` nor `generation`, and they contribute nothing —
    // demanding attribution fields from them would print 45 cries-wolf
    // errors per report on the real history. Only rows that CARRY strata
    // are required to attribute them.
    let strata = match obj.get("strata") {
        None | Some(serde_json::Value::Null) => return Ok(Vec::new()),
        Some(serde_json::Value::Array(items)) => items,
        Some(other) => {
            return Err(format!(
                "score-history row's `strata` is not a list (found {})",
                json_kind(other)
            ))
        }
    };
    let generation = match obj.get("generation") {
        Some(value) => {
            let n = value.as_u64().ok_or_else(|| {
                format!(
                    "score-history row's `generation` is not an unsigned integer (found {})",
                    json_kind(value)
                )
            })?;
            u32::try_from(n)
                .map_err(|_| format!("score-history row's `generation` = {n} does not fit u32"))?
        }
        None => {
            return Err(
                "score-history row has no `generation` field — strata cannot be attributed"
                    .to_string(),
            )
        }
    };
    let ts = match obj.get("ts") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => {
            return Err(format!(
                "score-history row's `ts` is not a string (found {})",
                json_kind(other)
            ))
        }
        None => {
            return Err(
                "score-history row has no `ts` field — strata cannot be attributed".to_string(),
            )
        }
    };
    let mut out = Vec::with_capacity(strata.len());
    for (idx, item) in strata.iter().enumerate() {
        let element = item.as_object().ok_or_else(|| {
            format!("strata[{idx}] is not an object (found {})", json_kind(item))
        })?;
        out.push(StratumRunView {
            generation,
            ts: ts.clone(),
            class: str_field(element, "class", idx)?.to_string(),
            vocab_before: usize_field(element, "vocab_before", idx)?,
            resolved: usize_field(element, "resolved", idx)?,
            heal_rate: f32_field(element, "heal_rate", idx)?,
        });
    }
    Ok(out)
}

/// The calibration verdict for one (stratum, generation) group — whether the
/// group's rate spread can carry a δ, or exactly why it cannot yet.
#[derive(Debug, Clone, PartialEq)]
pub enum CalibrationVerdict {
    /// ≥[`MIN_REPEAT_RUNS`] repeat runs of the unchanged corpus at this
    /// generation AND ≥[`MIN_STRATUM_FINDINGS`] findings in every run AND a
    /// NONZERO spread — the rate spread has empirical content.
    Supported,
    /// ≥ repeats AND deep findings but δ = 0: the compile/clippy oracle is
    /// deterministic per pinned toolchain, so repeated runs of the unchanged
    /// corpus measure the SAME rates and the spread carries no noise
    /// estimate. A δ = 0 floor would admit EVERYTHING while printing as a
    /// successful calibration — a well-formed number certifying nothing
    /// (the verdict round's blocking condition). The stochastic surface is
    /// the retrieval/heal-rate benches and the OOD rigs — vary THAT axis
    /// before trusting any δ here.
    Degenerate,
    /// The floor cannot calibrate at this (stratum, generation) yet; the
    /// payload is the reason, rendered verbatim.
    Deferred(String),
}

/// One (stratum, generation) calibration row: the measured rate spread and
/// whether it can support an admission floor δ.
#[derive(Debug, Clone, PartialEq)]
pub struct StratumCalibration {
    /// Stratum label (`repo` / `authored` / `construction` on record).
    pub class: String,
    /// The generation all runs in this group share.
    pub generation: u32,
    /// Repeat runs of the unchanged corpus observed at this generation.
    pub runs: usize,
    /// Smallest `vocab_before` across the group's runs.
    pub findings_min: usize,
    /// Largest `vocab_before` across the group's runs — the number the
    /// [`MIN_STRATUM_FINDINGS`] floor is tested against.
    pub findings_max: usize,
    /// Smallest observed heal rate.
    pub rate_min: f32,
    /// Largest observed heal rate.
    pub rate_max: f32,
    /// `rate_max - rate_min` — the empirical spread a δ must exceed.
    pub delta: f32,
    /// See [`CalibrationVerdict`].
    pub verdict: CalibrationVerdict,
}

/// Group stratum runs into per-(class, generation) calibration rows.
///
/// Output is sorted by class, then generation (`BTreeMap` iteration order —
/// deterministic). A class present in multiple generations yields one row per
/// generation: dissimilar ladders are never pooled (the Issue 102 lesson —
/// zero resolution across dissimilar ladders is fixture difficulty, not
/// variance). δ is plain f32 subtraction over the group's observed rates.
///
/// Verdict order is deliberate: the runs check fires BEFORE the findings
/// check, because a single run has no spread to measure whether its stratum
/// is deep or thin; the zero-spread check fires LAST, because a degenerate
/// δ is only decidable once repeats and depth have established that the
/// spread SHOULD have had content.
#[must_use]
pub fn calibrate(runs: &[StratumRunView]) -> Vec<StratumCalibration> {
    let mut groups: BTreeMap<(&str, u32), Vec<&StratumRunView>> = BTreeMap::new();
    for run in runs {
        groups
            .entry((run.class.as_str(), run.generation))
            .or_default()
            .push(run);
    }
    let mut out = Vec::with_capacity(groups.len());
    for ((class, generation), group) in groups {
        let mut findings_min = usize::MAX;
        let mut findings_max = 0;
        let mut rate_min = f32::INFINITY;
        let mut rate_max = f32::NEG_INFINITY;
        for run in &group {
            findings_min = findings_min.min(run.vocab_before);
            findings_max = findings_max.max(run.vocab_before);
            rate_min = rate_min.min(run.heal_rate);
            rate_max = rate_max.max(run.heal_rate);
        }
        let verdict = if group.len() < MIN_REPEAT_RUNS {
            CalibrationVerdict::Deferred(
                "single run at this generation — no repeat evidence".to_string(),
            )
        } else if findings_max < MIN_STRATUM_FINDINGS {
            CalibrationVerdict::Deferred(format!(
                "thin stratum ({findings_max} findings) — grow the repo fixture corpus first"
            ))
        } else if rate_max - rate_min == 0.0 {
            CalibrationVerdict::Degenerate
        } else {
            CalibrationVerdict::Supported
        };
        out.push(StratumCalibration {
            class: class.to_string(),
            generation,
            runs: group.len(),
            findings_min,
            findings_max,
            rate_min,
            rate_max,
            delta: rate_max - rate_min,
            verdict,
        });
    }
    out
}

/// Render the δ-calibration report (house voice: header with the constants,
/// one line per row, DEFERRED rows carry their reason, empty input is loud,
/// and a summary verdict for the `repo` stratum specifically — the stratum
/// the Issue 130 T2 admission floor reads).
#[must_use]
pub fn render_calibration(rows: &[StratumCalibration]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(1024);
    let n_rows = rows.len();
    let _ = writeln!(
        out,
        "## Strata δ-calibration (Issue 130 T1 — report-first, gates nothing)"
    );
    let _ = writeln!(
        out,
        "admission-floor evidence per (stratum, generation): MIN_STRATUM_FINDINGS = \
         {MIN_STRATUM_FINDINGS}, MIN_REPEAT_RUNS = {MIN_REPEAT_RUNS} | {n_rows} calibration \
         row(s)"
    );
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "no strata rows found — run --score-bench (a strata-bearing lane) first"
        );
        return out;
    }
    for row in rows {
        let (mark, verdict_text) = match &row.verdict {
            CalibrationVerdict::Supported => ("✓", "SUPPORTED".to_string()),
            CalibrationVerdict::Degenerate => (
                "⚠",
                "UNCALIBRATED — zero spread (the oracle axis is deterministic per pinned \
                 toolchain; a δ=0 floor admits everything — vary the bench/OOD axis instead)"
                    .to_string(),
            ),
            CalibrationVerdict::Deferred(reason) => ("⚠", format!("DEFERRED: {reason}")),
        };
        let rmin_pct = row.rate_min * 100.0;
        let rmax_pct = row.rate_max * 100.0;
        let _ = writeln!(
            out,
            "  {:<14} gen={:<3} runs={}  findings {}..{}  rate {:.1}%..{:.1}%  δ={:.3}  {} {}",
            row.class,
            row.generation,
            row.runs,
            row.findings_min,
            row.findings_max,
            rmin_pct,
            rmax_pct,
            row.delta,
            mark,
            verdict_text
        );
    }
    // Summary verdict for the REPO stratum specifically (class == "repo"),
    // read at its LATEST generation — the stratum the Issue 130 T2 admission
    // floor would gate on.
    if let Some(latest) = rows
        .iter()
        .filter(|r| r.class == "repo")
        .max_by_key(|r| r.generation)
    {
        match &latest.verdict {
            CalibrationVerdict::Supported => {
                let _ = writeln!(
                    out,
                    "repo stratum (latest generation {}): ✓ admission floor may be calibrated \
                     — δ={:.3} over {} run(s), findings {}..{}",
                    latest.generation,
                    latest.delta,
                    latest.runs,
                    latest.findings_min,
                    latest.findings_max
                );
            }
            CalibrationVerdict::Degenerate => {
                let _ = writeln!(
                    out,
                    "repo stratum (latest generation {}): ⚠ floor UNCALIBRATED — δ=0 over {} \
                     run(s) (deterministic oracle axis — a zero-δ floor admits everything and \
                     is never trusted as calibrated)",
                    latest.generation, latest.runs
                );
            }
            CalibrationVerdict::Deferred(reason) => {
                let _ = writeln!(
                    out,
                    "repo stratum (latest generation {}): ⚠ floor cannot calibrate yet — {reason}",
                    latest.generation
                );
            }
        }
    } else {
        let _ = writeln!(
            out,
            "repo stratum: absent from this history — the Issue 130 T2 admission floor has \
             nothing to calibrate"
        );
    }
    out
}

// ── T3 — the corpus-stall predicate over the frontier snapshot series ──────

/// Snapshots per store the stall predicate reads (RRSI's window w).
pub const STALL_WINDOW: usize = 6;

/// A window shorter than this is unanswered, never a pass: disclosed with a
/// loud note instead of a verdict.
pub const STALL_MIN_ROWS: usize = 3;

/// The label for the EMPTY-`store` bucket — legacy rows written before the
/// `store` field existed. Unattributed by construction: never folded into a
/// named store, never flagged.
pub const LEGACY_EMPTY_STORE: &str = "legacy-empty-store";

/// One store's stall verdict over its latest window of snapshots.
#[derive(Debug, Clone, PartialEq)]
pub struct StallRow {
    /// Store id — or [`LEGACY_EMPTY_STORE`] for the unattributed bucket.
    pub store: String,
    /// Rows actually in the window (may be < [`STALL_WINDOW`]).
    pub rows_in_window: usize,
    /// The window's generations, chronological (oldest → newest).
    pub window_generations: Vec<u32>,
    /// Count of i where `window_generations[i] > window_generations[i-1]` —
    /// generation advances; zero over a full-enough window is the stall.
    pub generation_boundaries: usize,
    /// `boundaries == 0` AND a full-enough window — the re-arm signal. The
    /// legacy bucket never flags (unattributed by construction).
    pub flagged: bool,
    /// The verdict text: the re-arm pointer when flagged, the
    /// insufficient-window disclosure when unanswered, the unattributed law
    /// for legacy rows, otherwise the activity count.
    pub note: String,
}

/// Scan the frontier snapshot series for corpus stalls (RRSI mechanism C:
/// `σ_t = 1[progress ≤ δ over window w]`, automating the operator-judged
/// coverage-report re-arm).
///
/// Rows group by the `store` field verbatim — the EMPTY-`store` bucket is its
/// own row under [`LEGACY_EMPTY_STORE`] and never flags, so legacy rows can
/// neither stall a verdict nor credit a named store. Per store the input is
/// sorted by `ts` (stable) and the LAST [`STALL_WINDOW`] rows are read;
/// `generation_boundaries` counts advances within that window. A window
/// shorter than [`STALL_MIN_ROWS`] is disclosed, never read as a pass.
///
/// Output is sorted by store id (`BTreeMap` iteration order — deterministic;
/// the empty label sorts first).
#[must_use]
pub fn stall_scan(snapshots: &[FrontierSnapshot]) -> Vec<StallRow> {
    let mut by_store: BTreeMap<&str, Vec<&FrontierSnapshot>> = BTreeMap::new();
    for snapshot in snapshots {
        by_store
            .entry(snapshot.store.as_str())
            .or_default()
            .push(snapshot);
    }
    let mut out = Vec::with_capacity(by_store.len());
    for (store, mut chron) in by_store {
        chron.sort_by(|a, b| a.ts.cmp(&b.ts));
        let start = chron.len().saturating_sub(STALL_WINDOW);
        let window = &chron[start..];
        let rows_in_window = window.len();
        let window_generations: Vec<u32> =
            window.iter().map(|s| u32::from(s.generation)).collect();
        let mut boundaries = 0;
        for pair in window_generations.windows(2) {
            if pair[1] > pair[0] {
                boundaries += 1;
            }
        }
        let legacy = store.is_empty();
        let enough = rows_in_window >= STALL_MIN_ROWS;
        let flagged = !legacy && enough && boundaries == 0;
        let note = if legacy && enough {
            "legacy rows carry no store id — unattributed by construction, never flagged"
                .to_string()
        } else if !enough {
            format!("insufficient window ({rows_in_window} rows < {STALL_MIN_ROWS}) — no verdict")
        } else if flagged {
            "corpus stalled — run --coverage-report and route new uncovered families to the \
             mining queue"
                .to_string()
        } else {
            format!("active — {boundaries} generation advance(s) in the window")
        };
        let label = if legacy { LEGACY_EMPTY_STORE } else { store };
        out.push(StallRow {
            store: label.to_string(),
            rows_in_window,
            window_generations,
            generation_boundaries: boundaries,
            flagged,
            note,
        });
    }
    out
}

/// Render the stall report (house voice: header with the window constants,
/// one line per store with the flag mark, flagged lines visually loud, empty
/// input loud, and a closing summary line in both directions).
#[must_use]
pub fn render_stall(rows: &[StallRow]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(512);
    let _ = writeln!(out, "## Corpus stall scan (Issue 130 T3 — report-first, gates nothing)");
    let _ = writeln!(
        out,
        "σ_t per store over the last {STALL_WINDOW} snapshot rows (stall = zero generation \
         advances across ≥{STALL_MIN_ROWS} rows)"
    );
    if rows.is_empty() {
        let _ = writeln!(
            out,
            "no frontier snapshots found — the snapshot series is empty; nothing was measured"
        );
        return out;
    }
    for row in rows {
        let gens = row
            .window_generations
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let mark = if row.flagged { "⚠ STALLED" } else { "· ok" };
        let _ = writeln!(
            out,
            "  {:<20} window={}/{} gens=[{}] boundaries={}  {} — {}",
            row.store,
            row.rows_in_window,
            STALL_WINDOW,
            gens,
            row.generation_boundaries,
            mark,
            row.note
        );
    }
    let flagged: Vec<&str> = rows
        .iter()
        .filter(|r| r.flagged)
        .map(|r| r.store.as_str())
        .collect();
    if flagged.is_empty() {
        let n_stores = rows.len();
        let _ = writeln!(
            out,
            "no stall flagged across {n_stores} store window(s) — the coverage-report re-arm \
             stays operator-judged (report-first)"
        );
    } else {
        let names = flagged.join(", ");
        let flagged_count = flagged.len();
        let _ = writeln!(
            out,
            "⚠ {flagged_count} stalled store window(s): {names}"
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    fn run(
        class: &str,
        generation: u32,
        ts: &str,
        vocab_before: usize,
        resolved: usize,
        heal_rate: f32,
    ) -> StratumRunView {
        StratumRunView {
            generation,
            ts: ts.to_string(),
            class: class.to_string(),
            vocab_before,
            resolved,
            heal_rate,
        }
    }

    fn snap(ts: &str, generation: u8, store: &str) -> FrontierSnapshot {
        FrontierSnapshot {
            ts: ts.to_string(),
            sha: "deadbee".to_string(),
            generation,
            population: 19,
            seen: 19,
            band_count: 1,
            band_mass: 0.05,
            saturated: 2,
            feeds: Vec::new(),
            store: store.to_string(),
        }
    }

    #[test]
    fn stratum_runs_parses_the_real_row_shape() {
        // Field names confirmed against .benchmarks/score_history.jsonl
        // (row ts=2026-09-11T21:24:22, generation 1).
        let row: serde_json::Value = serde_json::from_str(
            r#"{"generation": 1, "ts": "2026-09-11T21:24:22", "sha": "abc", "vocab_size": 106,
                "strata": [
                  {"class": "repo", "n_ran": 8, "n_healable": 2, "n_clean": 6,
                   "vocab_before": 2, "resolved": 1, "created": 0, "heal_rate": 0.5},
                  {"class": "authored", "n_ran": 27, "n_healable": 27, "n_clean": 0,
                   "vocab_before": 51, "resolved": 50, "created": 1, "heal_rate": 0.98039216}
                ]}"#,
        )
        .unwrap();
        let views = stratum_runs(&row).expect("well-formed row");
        assert_eq!(views.len(), 2);
        assert_eq!(views[0].class, "repo");
        assert_eq!(views[0].generation, 1);
        assert_eq!(views[0].ts, "2026-09-11T21:24:22");
        assert_eq!(views[0].vocab_before, 2);
        assert_eq!(views[0].resolved, 1);
        assert!(close(views[0].heal_rate, 0.5));
        assert_eq!(views[1].class, "authored");
        assert_eq!(views[1].vocab_before, 51);
        assert_eq!(views[1].resolved, 50);
        assert!(close(views[1].heal_rate, 0.980_392_16));
    }

    #[test]
    fn stratum_runs_tolerates_missing_empty_and_null_strata() {
        let missing: serde_json::Value =
            serde_json::from_str(r#"{"generation": 1, "ts": "t", "heal_rate": 0.9}"#).unwrap();
        assert!(stratum_runs(&missing).unwrap().is_empty());
        let empty: serde_json::Value =
            serde_json::from_str(r#"{"generation": 1, "ts": "t", "strata": []}"#).unwrap();
        assert!(stratum_runs(&empty).unwrap().is_empty());
        let null: serde_json::Value =
            serde_json::from_str(r#"{"generation": 1, "ts": "t", "strata": null}"#).unwrap();
        assert!(stratum_runs(&null).unwrap().is_empty());
    }

    #[test]
    fn stratum_runs_errs_precisely_on_malformed_input() {
        let not_a_list: serde_json::Value =
            serde_json::from_str(r#"{"generation": 1, "ts": "t", "strata": "repo"}"#).unwrap();
        let err = stratum_runs(&not_a_list).unwrap_err();
        assert!(err.contains("`strata` is not a list"), "{err}");

        let not_an_object: serde_json::Value =
            serde_json::from_str(r#"{"generation": 1, "ts": "t", "strata": [42]}"#).unwrap();
        let err = stratum_runs(&not_an_object).unwrap_err();
        assert!(err.contains("strata[0] is not an object"), "{err}");

        let missing_field: serde_json::Value = serde_json::from_str(
            r#"{"generation": 1, "ts": "t", "strata": [{"class": "repo"}]}"#,
        )
        .unwrap();
        let err = stratum_runs(&missing_field).unwrap_err();
        assert!(err.contains("`vocab_before`"), "{err}");

        let bad_rate: serde_json::Value = serde_json::from_str(
            r#"{"generation": 1, "ts": "t", "strata": [{"class": "repo",
                "vocab_before": 2, "resolved": 1, "heal_rate": "high"}]}"#,
        )
        .unwrap();
        let err = stratum_runs(&bad_rate).unwrap_err();
        assert!(err.contains("`heal_rate` is not a number"), "{err}");

        // The pre-strata short-circuit: a row with NEITHER `strata` NOR
        // attribution fields (the old history) extracts to empty — it is
        // not an error, and demanding `generation` from it would print
        // cries-wolf errors for every pre-strata row on every report.
        let pre_strata: serde_json::Value =
            serde_json::from_str(r#"{"ts": "t"}"#).unwrap();
        assert!(stratum_runs(&pre_strata).unwrap().is_empty());
        let no_strata_at_all: serde_json::Value =
            serde_json::from_str(r#"{"generation": 1}"#).unwrap();
        assert!(stratum_runs(&no_strata_at_all).unwrap().is_empty());

        // Rows that CARRY strata must attribute them — a missing
        // `generation` or `ts` on a strata-bearing row is a real error.
        let missing_generation: serde_json::Value = serde_json::from_str(
            r#"{"ts": "t", "strata": [{"class": "repo", "vocab_before": 2,
                "resolved": 1, "heal_rate": 0.5}]}"#,
        )
        .unwrap();
        let err = stratum_runs(&missing_generation).unwrap_err();
        assert!(err.contains("`generation`"), "{err}");

        let missing_ts: serde_json::Value = serde_json::from_str(
            r#"{"generation": 1, "strata": [{"class": "repo", "vocab_before": 2,
                "resolved": 1, "heal_rate": 0.5}]}"#,
        )
        .unwrap();
        let err = stratum_runs(&missing_ts).unwrap_err();
        assert!(err.contains("`ts`"), "{err}");
    }

    #[test]
    fn calibrate_groups_by_class_and_generation_and_never_pools() {
        // Scrambled on purpose: output must come out sorted by
        // (class, generation) regardless of input order.
        let runs = vec![
            run("authored", 8, "t3", 51, 39, 0.75),
            run("repo", 2, "t4", 2, 0, 0.0),
            run("construction", 7, "t2", 40, 20, 0.5),
            run("authored", 7, "t1", 51, 51, 1.0),
            run("authored", 8, "t5", 51, 39, 0.75),
            run("authored", 7, "t6", 51, 26, 0.5),
            run("construction", 7, "t7", 40, 20, 0.5),
            run("repo", 2, "t8", 2, 2, 1.0),
            run("construction", 7, "t9", 40, 24, 0.6),
        ];
        let rows = calibrate(&runs);
        assert_eq!(rows.len(), 4, "one row per (class, generation)");
        let keys: Vec<(&str, u32)> = rows
            .iter()
            .map(|r| (r.class.as_str(), r.generation))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("authored", 7),
                ("authored", 8),
                ("construction", 7),
                ("repo", 2)
            ]
        );
        // Generations never pool: authored gen 7 has exactly 2 runs even
        // though gen 8 also carries authored rows.
        let a7 = &rows[0];
        assert_eq!(a7.runs, 2);
        assert_eq!(a7.findings_min, 51);
        assert_eq!(a7.findings_max, 51);
        assert!(close(a7.rate_min, 0.5));
        assert!(close(a7.rate_max, 1.0));
        assert!(close(a7.delta, 0.5));
        assert_eq!(a7.verdict, CalibrationVerdict::Supported);
        let a8 = &rows[1];
        assert_eq!(a8.runs, 2);
        assert!(close(a8.delta, 0.0), "identical rates → δ 0");
        let c7 = &rows[2];
        assert_eq!(c7.runs, 3);
        assert!(close(c7.delta, 0.1), "0.5..0.6 → δ 0.1");
        assert_eq!(c7.verdict, CalibrationVerdict::Supported);
        let repo = &rows[3];
        assert_eq!(repo.runs, 2);
        assert!(close(repo.delta, 1.0));
        assert_eq!(
            repo.verdict,
            CalibrationVerdict::Deferred(
                "thin stratum (2 findings) — grow the repo fixture corpus first".to_string()
            )
        );
    }

    #[test]
    fn calibrate_defers_single_run_even_when_deep() {
        let runs = vec![run("repo", 1, "t1", 100, 50, 0.5)];
        let rows = calibrate(&runs);
        assert_eq!(rows.len(), 1);
        // The runs check fires BEFORE the findings check: a single run has
        // no spread to measure, deep stratum or thin.
        assert_eq!(
            rows[0].verdict,
            CalibrationVerdict::Deferred(
                "single run at this generation — no repeat evidence".to_string()
            )
        );
    }

    #[test]
    fn calibrate_defers_thin_stratum_even_with_repeats() {
        let runs = vec![
            run("repo", 1, "t1", 2, 0, 0.0),
            run("repo", 1, "t2", 2, 2, 1.0),
        ];
        let rows = calibrate(&runs);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].findings_max, 2);
        assert_eq!(
            rows[0].verdict,
            CalibrationVerdict::Deferred(
                "thin stratum (2 findings) — grow the repo fixture corpus first".to_string()
            )
        );
    }

    #[test]
    fn calibrate_supported_needs_repeats_and_depth() {
        // Exactly at the floors: 2 runs, 20 findings, nonzero spread —
        // supported.
        let runs = vec![
            run("authored", 8, "t1", 20, 10, 0.5),
            run("authored", 8, "t2", 20, 12, 0.6),
        ];
        let rows = calibrate(&runs);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].verdict, CalibrationVerdict::Supported);
        assert!(close(rows[0].delta, 0.1));
    }

    #[test]
    fn calibrate_zero_spread_at_depth_is_degenerate_never_supported() {
        // The verdict round's blocking condition: a deterministic oracle
        // re-measures the SAME rates, δ collapses to 0, and a floor that
        // admitted everything must not print as calibrated. The planted
        // direction: deleting the zero-spread check turns this arm's
        // expectation into `Supported` and fails the render needle below.
        let runs = vec![
            run("repo", 6, "t1", 21, 21, 1.0),
            run("repo", 6, "t2", 21, 21, 1.0),
        ];
        let rows = calibrate(&runs);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].findings_max, 21);
        assert_eq!(rows[0].runs, 2);
        // Deterministic fixture inputs: the delta is EXACTLY 0.0 (same-value
        // subtraction), so the float-cmp gate's exactness is the assertion.
        #[allow(clippy::float_cmp)]
        let degenerate = rows[0].delta == 0.0;
        assert!(degenerate);
        assert_eq!(rows[0].verdict, CalibrationVerdict::Degenerate);
        let text = render_calibration(&rows);
        assert!(text.contains("UNCALIBRATED — zero spread"), "{text}");
        assert!(
            text.contains("repo stratum (latest generation 6): ⚠ floor UNCALIBRATED"),
            "{text}"
        );
        assert!(!text.contains("may be calibrated"), "{text}");
    }

    #[test]
    fn render_calibration_covers_every_verdict_and_repo_summary() {
        let runs = vec![
            run("authored", 8, "t1", 51, 51, 1.0),
            run("authored", 8, "t2", 51, 48, 0.941_176_5),
            run("construction", 8, "t1", 60, 30, 0.5),
            run("construction", 8, "t2", 60, 30, 0.5),
            run("construction", 8, "t3", 60, 30, 0.5),
            run("repo", 1, "t1", 2, 0, 0.0),
            run("repo", 1, "t2", 2, 2, 1.0),
            run("repo", 2, "t3", 2, 2, 1.0),
        ];
        let rows = calibrate(&runs);
        let text = render_calibration(&rows);
        assert!(text.contains("MIN_STRATUM_FINDINGS = 20"), "{text}");
        assert!(text.contains("MIN_REPEAT_RUNS = 2"), "{text}");
        assert!(text.contains("4 calibration row(s)"), "{text}");
        assert!(text.contains("SUPPORTED"), "{text}");
        // construction at gen 8: 3 runs, 60 findings, flat 0.5 — the
        // degenerate arm (zero spread at depth).
        assert!(text.contains("UNCALIBRATED — zero spread"), "{text}");
        assert!(
            text.contains(
                "DEFERRED: thin stratum (2 findings) — grow the repo fixture corpus first"
            ),
            "{text}"
        );
        assert!(
            text.contains("DEFERRED: single run at this generation — no repeat evidence"),
            "{text}"
        );
        // repo latest generation is 2 (single run) → the deferred summary.
        assert!(
            text.contains("repo stratum (latest generation 2): ⚠ floor cannot calibrate yet"),
            "{text}"
        );
    }

    #[test]
    fn render_calibration_repo_summary_flips_with_the_latest_generation() {
        let deep = calibrate(&[
            run("repo", 4, "t1", 51, 51, 1.0),
            run("repo", 4, "t2", 51, 50, 0.98),
        ]);
        let text = render_calibration(&deep);
        assert!(
            text.contains("repo stratum (latest generation 4): ✓ admission floor may be calibrated"),
            "{text}"
        );

        let thin = calibrate(&[
            run("repo", 4, "t1", 2, 0, 0.0),
            run("repo", 4, "t2", 2, 2, 1.0),
        ]);
        let text = render_calibration(&thin);
        assert!(text.contains("floor cannot calibrate yet"), "{text}");

        let no_repo = calibrate(&[
            run("authored", 4, "t1", 51, 51, 1.0),
            run("authored", 4, "t2", 51, 51, 1.0),
        ]);
        let text = render_calibration(&no_repo);
        assert!(text.contains("nothing to calibrate"), "{text}");
    }

    #[test]
    fn render_calibration_empty_is_loud() {
        let text = render_calibration(&[]);
        assert!(
            text.contains("no strata rows found — run --score-bench (a strata-bearing lane) first"),
            "{text}"
        );
    }

    #[test]
    fn stall_scan_separates_legacy_and_never_flags_it() {
        // Four flat legacy rows are the would-be-stalled SHAPE, but the
        // empty-store bucket is unattributed by construction.
        let snapshots = vec![
            snap("t1", 8, ""),
            snap("t2", 8, ""),
            snap("t3", 8, ""),
            snap("t4", 8, ""),
            snap("t1", 7, "865ae1da"),
            snap("t2", 8, "865ae1da"),
            snap("t3", 8, "865ae1da"),
            snap("t4", 8, "865ae1da"),
            snap("t5", 8, "865ae1da"),
            snap("t6", 8, "865ae1da"),
        ];
        let rows = stall_scan(&snapshots);
        assert_eq!(rows.len(), 2);
        // "" sorts first in the BTreeMap → the legacy row renders first.
        let legacy = &rows[0];
        assert_eq!(legacy.store, LEGACY_EMPTY_STORE);
        assert!(!legacy.flagged, "unattributed rows never flag");
        assert_eq!(legacy.rows_in_window, 4);
        assert_eq!(legacy.generation_boundaries, 0);
        assert!(legacy.note.contains("unattributed"), "{}", legacy.note);
        let named = &rows[1];
        assert_eq!(named.store, "865ae1da");
        assert_eq!(named.generation_boundaries, 1, "7→8 is one advance");
        assert!(!named.flagged);
        assert!(named.note.contains("active"), "{}", named.note);
    }

    #[test]
    fn stall_scan_flags_a_flat_full_window_at_the_rearm() {
        let snapshots: Vec<FrontierSnapshot> = (1u8..=6)
            .map(|i| snap(&format!("t0{i}"), 8, "865ae1da"))
            .collect();
        let rows = stall_scan(&snapshots);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(row.flagged);
        assert_eq!(row.generation_boundaries, 0);
        assert_eq!(row.rows_in_window, 6);
        assert_eq!(
            row.note,
            "corpus stalled — run --coverage-report and route new uncovered families to the \
             mining queue"
        );
    }

    #[test]
    fn stall_scan_planted_bug_legacy_rows_never_credit_a_named_store() {
        // A naive implementation that folds the empty-store bucket into
        // every named store would hand "solo" 2 + 3 = 5 rows — a full flat
        // window and a false STALLED verdict. The grouping is by the store
        // field verbatim, so this must stay an insufficient window.
        let snapshots = vec![
            snap("t1", 8, "solo"),
            snap("t2", 8, "solo"),
            snap("t1", 8, ""),
            snap("t2", 8, ""),
            snap("t3", 8, ""),
        ];
        let rows = stall_scan(&snapshots);
        assert_eq!(rows.len(), 2);
        let legacy = rows
            .iter()
            .find(|r| r.store == LEGACY_EMPTY_STORE)
            .expect("legacy row");
        assert_eq!(legacy.rows_in_window, 3);
        assert!(!legacy.flagged);
        let solo = rows.iter().find(|r| r.store == "solo").expect("solo row");
        assert_eq!(
            solo.rows_in_window, 2,
            "legacy rows must not pool into named stores"
        );
        assert!(!solo.flagged);
        assert_eq!(solo.note, "insufficient window (2 rows < 3) — no verdict");
    }

    #[test]
    fn stall_scan_slices_the_last_six_rows_chronologically_from_unsorted_input() {
        let mut snapshots: Vec<FrontierSnapshot> = (1u8..=8)
            .map(|i| snap(&format!("2026-09-2{i}T00:00:00"), i, "a"))
            .collect();
        snapshots.reverse(); // feed newest-first; the scan must sort by ts
        let rows = stall_scan(&snapshots);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.rows_in_window, STALL_WINDOW);
        assert_eq!(row.window_generations, vec![3, 4, 5, 6, 7, 8]);
        assert_eq!(row.generation_boundaries, 5, "each step is an advance");
        assert!(!row.flagged);
        assert!(
            row.note.contains("active — 5 generation advance(s)"),
            "{}",
            row.note
        );
    }

    #[test]
    fn stall_scan_boundary_counting_matches_the_issue_examples() {
        // gens 7,8,8,8 → exactly 1 boundary.
        let snapshots = vec![
            snap("t1", 7, "a"),
            snap("t2", 8, "a"),
            snap("t3", 8, "a"),
            snap("t4", 8, "a"),
        ];
        let rows = stall_scan(&snapshots);
        assert_eq!(rows[0].generation_boundaries, 1);
        assert!(!rows[0].flagged);

        // Flat 8,8 → 0 boundaries; below the window floor → no verdict.
        let flat = vec![snap("t1", 8, "a"), snap("t2", 8, "a")];
        let rows = stall_scan(&flat);
        assert_eq!(rows[0].generation_boundaries, 0);
        assert!(!rows[0].flagged);
        assert_eq!(rows[0].note, "insufficient window (2 rows < 3) — no verdict");
    }

    #[test]
    fn render_stall_flags_are_loud_and_legacy_labelled() {
        let snapshots = vec![
            snap("t1", 8, ""),
            snap("t2", 8, ""),
            snap("t3", 8, ""),
            snap("t1", 8, "stalled"),
            snap("t2", 8, "stalled"),
            snap("t3", 8, "stalled"),
            snap("t4", 8, "stalled"),
            snap("t5", 8, "stalled"),
            snap("t6", 8, "stalled"),
            snap("t1", 8, "thin"),
            snap("t2", 8, "thin"),
        ];
        let text = render_stall(&stall_scan(&snapshots));
        assert!(text.contains("last 6 snapshot rows"), "{text}");
        assert!(text.contains("⚠ STALLED"), "{text}");
        assert!(text.contains(LEGACY_EMPTY_STORE), "{text}");
        assert!(
            text.contains("insufficient window (2 rows < 3) — no verdict"),
            "{text}"
        );
        assert!(
            text.contains(
                "corpus stalled — run --coverage-report and route new uncovered families to the \
                 mining queue"
            ),
            "{text}"
        );
        assert!(
            text.contains("⚠ 1 stalled store window(s): stalled"),
            "{text}"
        );
    }

    #[test]
    fn render_stall_summarizes_clean_states_loudly_neutral() {
        let snapshots = vec![snap("t1", 7, "a"), snap("t2", 8, "a")];
        let text = render_stall(&stall_scan(&snapshots));
        assert!(
            text.contains("no stall flagged across 1 store window(s)"),
            "{text}"
        );
        assert!(text.contains("· ok"), "{text}");
    }

    #[test]
    fn render_stall_empty_is_loud() {
        let text = render_stall(&[]);
        assert!(
            text.contains("no frontier snapshots found — the snapshot series is empty"),
            "{text}"
        );
    }
}

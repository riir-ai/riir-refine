//! Score history — append-only JSONL + zero-dep SVG curve (Plan 014).
//!
//! One JSONL record per bench run (ts, git sha, per-axis scores). The SVG
//! renders every record as a point on four series:
//! `heal_rate` / `decline_correctness` / `parse_safety` (up is better) and
//! `created_rate` (RED, down is better). Hand-rolled string building — no
//! plotting crate (the repo's dependency-light default-build rule).
//!
//! Expect a STEP curve: improvement lands at root-cause proposer fixes, not
//! smoothly per run (the self-learning mechanisms measured negative and
//! were killed — Plan 005 Phase 7 / Issue 026). The curve's job is to make
//! regressions and staleplate visible, not to look like gradient descent.

// Chart geometry is small-integer pixel math (values < 10_000 — far below
// f32's 23-bit mantissa) + Hinnant's civil-calendar algorithm has bounded
// intermediate ranges by construction. The pedantic cast lints are noise
// here, allowed with cause.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use serde::{Deserialize, Serialize};

/// One score-history record (one line of `score_history.jsonl`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryRecord {
    /// ISO-8601 local timestamp of the run.
    pub ts: String,
    /// Short git sha of the healer build (best-effort; empty if not a repo).
    pub sha: String,
    /// Fixtures ran (excluded ones do not count).
    pub n_ran: usize,
    /// Healable + clean split of what ran.
    pub n_healable: usize,
    pub n_clean: usize,
    /// Σ resolved vocabulary lints / Σ vocabulary lints before.
    pub heal_rate: f32,
    /// Σ created lints / Σ vocabulary lints before (lower is better).
    pub created_rate: f32,
    /// Healed-and-compiling / edited fixtures.
    pub parse_safety: f32,
    /// Clean fixtures untouched / clean fixtures.
    pub decline_correctness: f32,
    pub edits_total: usize,
    pub vocab_size: usize,
    /// The bench generation (Plan 092 M5): 1 = the frozen v1 set, 2 = the
    /// construction-built generation-v2 ladder. Old rows predate the field
    /// and deserialize as 1 — the v1 series is never renumbered.
    #[serde(default = "default_generation")]
    pub generation: u8,
}

/// The serde default for pre-generation rows (issue 072 T4).
#[must_use]
pub fn default_generation() -> u8 {
    1
}

/// Default history paths (relative to the repo root / CWD of the run).
pub const HISTORY_JSONL: &str = ".benchmarks/score_history.jsonl";
pub const HISTORY_SVG: &str = ".benchmarks/score_history.svg";
/// The generation-2 curve (Plan 092 M5) — rendered only when v2 rows exist.
pub const HISTORY_SVG_V2: &str = ".benchmarks/score_history_v2.svg";

/// The M5 saturation window: marginal gain = rate[last] − rate[last − N].
/// R = 5 ≈ the observed spacing between step changes in the 45-run history
/// (the concurrent session's derivation, absorbed — the bench improves in
/// ROOT-CAUSE STEPS, so a window this wide is what separates a step from
/// one-run noise); re-pin only with the runs cited.
pub const SATURATION_WINDOW: usize = 5;
/// The M5 saturation threshold (strict): marginal below this flags the
/// series saturated (0.5pp over 5 runs is the resolution floor the bench
/// currently measures at — run #45-era flatness).
pub const SATURATION_EPSILON: f32 = 0.005;

/// Append one record (creates the file + parent dir on first write).
///
/// # Errors
///
/// Propagates filesystem errors (create dir / open / write).
pub fn append_history(path: &std::path::Path, rec: &HistoryRecord) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut line = serde_json::to_string(rec).map_err(std::io::Error::other)?;
    line.push('\n');
    f.write_all(line.as_bytes())
}

/// Load the full history (missing file → empty; malformed line → loud skip).
#[must_use]
pub fn load_history(path: &std::path::Path) -> Vec<HistoryRecord> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| match serde_json::from_str(l) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("score_bench: skipping malformed history line: {e}");
                None
            }
        })
        .collect()
}

/// Best-effort short git sha of the current repo.
#[must_use]
pub fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Issue 050 T2 — the Warm-tier dual-write: mirror one finished bench run
/// into the repo-local trajectory store as a
/// [`crate::traj_store::ScoreRunRow`] (the jsonl stays the committed
/// projection; the store is the queryable source of truth). Best-effort by
/// contract: ANY failure prints one stderr line and must NEVER fail the
/// bench run. Store-root resolution mirrors the CLI's other standalone
/// modes (`RIIR_HEAL_TRAJ_ROOT` → cwd).
///
/// Always compiled when this module is (`score_bench` implies
/// `latent_retrieval`); the cfg attribute documents the dependency.
#[cfg(feature = "latent_retrieval")]
pub fn record_store_best_effort(rec: &HistoryRecord) {
    let root = std::env::var("RIIR_HEAL_TRAJ_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|_| std::env::current_dir())
        .unwrap_or_default();
    let run = crate::traj_store::ScoreRunRow::from(rec);
    match crate::traj_store::TrajStore::open(&root) {
        Ok((store, _)) => {
            if let Err(e) = store.record_score_run(&run) {
                eprintln!("score_bench: store dual-write failed (jsonl unaffected): {e}");
            }
        }
        Err(e) => {
            eprintln!("score_bench: store dual-write skipped, store unavailable: {e}");
        }
    }
}

/// UTC ISO-8601 timestamp, std-only (Howard Hinnant's civil-from-days —
/// same algorithm chrono uses; no dependency for one timestamp per run).
/// UTC deliberately: deterministic across machines.
// The date math's intermediate casts are bounded by the algorithm's ranges
// (era ≤ ~5M years); the single-char names are y/m/d/h/min/s — deliberate.
#[must_use]
#[allow(clippy::cast_possible_wrap, clippy::many_single_char_names)]
pub fn local_iso_timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (h, min, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}")
}

/// days-since-epoch → (y, m, d) — Hinnant's `civil_from_days`.
// Bounded intermediate ranges by construction (the algorithm's own comments).
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl HistoryRecord {
    /// Build the record from a finished bench run (generation 1 — the v1
    /// frozen set; gen-2 runs call [`Self::with_generation`] after).
    #[must_use]
    pub fn from_score(score: &super::BenchScore, sha: String) -> Self {
        let n_healable = score
            .outcomes
            .iter()
            .filter(|o| !o.vocab_before.is_empty())
            .count();
        Self {
            ts: local_iso_timestamp(),
            sha,
            n_ran: score.n_ran,
            n_healable,
            n_clean: score.n_clean,
            heal_rate: score.heal_rate,
            created_rate: score.created_rate,
            parse_safety: score.parse_safety,
            decline_correctness: score.decline_correctness,
            edits_total: score.edits_total,
            vocab_size: score.vocab_size,
            generation: 1,
        }
    }

    /// Re-tag the record for a non-v1 generation (Plan 092 M5).
    #[must_use]
    pub fn with_generation(mut self, generation: u8) -> Self {
        self.generation = generation;
        self
    }
}

/// The M5 saturation verdict for one generation's series.
#[derive(Debug, Clone, PartialEq)]
pub enum SaturationVerdict {
    /// The series still moves — `marginal` gain over the window.
    Active { marginal: f32 },
    /// Marginal gain < ε — measurement resolution is exhausted on this
    /// generation; the ratchet's cue to consider a harder generation.
    Saturated { marginal: f32 },
    /// Fewer than WINDOW+1 rows for this generation — no verdict yet.
    InsufficientHistory { have: usize, need: usize },
}

/// Detect saturation on the per-generation heal-rate series (issue 072 T3,
/// M5): marginal = rate[last] − rate[last − [`SATURATION_WINDOW`]] over the
/// records of ONE generation; saturated iff marginal < [`SATURATION_EPSILON`]
/// (strict — exactly-ε is still active). Older rows with no `generation`
/// field deserialize as 1, so the v1 verdict never sees v2 rows.
#[must_use]
pub fn detect_saturation(records: &[HistoryRecord], generation: u8) -> SaturationVerdict {
    let rates: Vec<f32> = records
        .iter()
        .filter(|r| r.generation == generation)
        .map(|r| r.heal_rate)
        .collect();
    let have = rates.len();
    if have < SATURATION_WINDOW + 1 {
        return SaturationVerdict::InsufficientHistory {
            have,
            need: SATURATION_WINDOW + 1,
        };
    }
    let last = rates[have - 1];
    let base = rates[have - 1 - SATURATION_WINDOW];
    let marginal = last - base;
    if marginal < SATURATION_EPSILON {
        SaturationVerdict::Saturated { marginal }
    } else {
        SaturationVerdict::Active { marginal }
    }
}

/// The one-line saturation telemetry for a bench run (T3/T4: printed after
/// every append; on the v1 path a Saturated verdict also carries the
/// `--generation 2` hint).
#[must_use]
pub fn saturation_line(verdict: &SaturationVerdict, generation: u8) -> String {
    match verdict {
        SaturationVerdict::Active { marginal } => format!(
            "saturation: gen-{generation} ACTIVE (marginal {marginal:+.4} over {SATURATION_WINDOW} runs)"
        ),
        SaturationVerdict::Saturated { marginal } => format!(
            "saturation: gen-{generation} SATURATED (marginal {marginal:+.4} < {SATURATION_EPSILON} over {SATURATION_WINDOW} runs) — consider `--generation 2` (v1 floors stay pinned)"
        ),
        SaturationVerdict::InsufficientHistory { have, need } => format!(
            "saturation: gen-{generation} no verdict ({have}/{need} runs)"
        ),
    }
}

/// Print the human-readable bench report (summary axes + the persist
/// diagnostic — WHERE the misses are, per lint, most-frequent first).
pub fn print_report(score: &super::BenchScore) {
    println!("\n─ clippy-bench ────────────────────────────────────────");
    println!(
        "fixtures: {} ran / {} excluded (clean: {})",
        score.n_ran, score.n_excluded, score.n_clean
    );
    println!("heal_rate            {:>6.1}%  (up is better)", score.heal_rate * 100.0);
    println!(
        "created_rate         {:>6.1}%  (down is better — {} new lint(s))",
        score.created_rate * 100.0,
        score.created_total
    );
    println!("parse_safety         {:>6.1}%", score.parse_safety * 100.0);
    println!(
        "decline_correctness  {:>6.1}%",
        score.decline_correctness * 100.0
    );
    println!("edits applied: {}", score.edits_total);
    if !score.persist.is_empty() {
        println!("persisting (the miss map — where healing still fails):");
        for (lint, count) in score.persist.iter().take(10) {
            println!("  {lint:<32} x{count}");
        }
    }
    let drifted = score.outcomes.iter().filter(|o| o.drifted).count();
    if drifted > 0 {
        println!("drift warnings: {drifted} fixture(s) — frozen labels stale vs this toolchain");
    }
    println!("───────────────────────────────────────────────────────");
}

/// Chart geometry constants (860×400, y ∈ [0,1] from percent).
struct ChartLayout {
    w: f32,
    h: f32,
    ml: f32,
    mr: f32,
    mt: f32,
    mb: f32,
}

impl ChartLayout {
    const fn new() -> Self {
        Self { w: 860.0, h: 400.0, ml: 64.0, mr: 16.0, mt: 56.0, mb: 44.0 }
    }
    fn plot_w(&self) -> f32 {
        self.w - self.ml - self.mr
    }
    fn plot_h(&self) -> f32 {
        self.h - self.mt - self.mb
    }
    fn y_of(&self, v: f32) -> f32 {
        self.mt + self.plot_h() * (1.0 - v.clamp(0.0, 1.0))
    }
}

/// One plotted series.
struct SeriesSpec {
    label: &'static str,
    color: &'static str,
    /// Extract the series value from a record.
    value: fn(&HistoryRecord) -> f32,
}

fn series_specs() -> [SeriesSpec; 4] {
    [
        SeriesSpec { label: "heal_rate", color: "#2e7d32", value: |r| r.heal_rate },
        SeriesSpec { label: "decline_correct", color: "#1565c0", value: |r| r.decline_correctness },
        SeriesSpec { label: "parse_safety", color: "#6a1b9a", value: |r| r.parse_safety },
        SeriesSpec { label: "created_rate", color: "#c62828", value: |r| r.created_rate },
    ]
}

/// Render the history as an SVG line chart. Zero-dep: string building via
/// [`std::fmt::Write`] (no `format!`-append churn).
///
/// Casts + closures: chart geometry is small-integer pixel math — the
/// pedantic cast lints are allowed locally with cause.
#[must_use]
pub fn render_svg(records: &[HistoryRecord]) -> String {
    use std::fmt::Write as _;
    let layout = ChartLayout::new();
    let mut svg = String::with_capacity(4096);
    let w = &mut svg;

    writeln!(w, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" font-family=\"ui-monospace,Menlo,monospace\">",
        layout.w, layout.h, layout.w, layout.h).unwrap();
    w.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#fdfdfd\"/>\n");
    writeln!(w,
        "<text x=\"{}\" y=\"24\" font-size=\"15\" font-weight=\"bold\" fill=\"#212121\">clippy-bench — frozen real-code healing score</text>",
        layout.ml).unwrap();
    let last = records.last();
    let ran = last.map_or(0, |r| r.n_ran);
    let total = last.map_or(0, |r| r.n_healable + r.n_clean);
    writeln!(w,
        "<text x=\"{}\" y=\"42\" font-size=\"11\" fill=\"#616161\">frozen denominator: {ran}/{total} ran — corpus growth cannot move this score; created_rate (red) is down-is-better</text>",
        layout.ml).unwrap();

    render_grid(w, &layout);
    if records.is_empty() {
        writeln!(w,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"13\" fill=\"#9e9e9e\" text-anchor=\"middle\">no runs yet — cargo heal --score-bench</text>",
            layout.ml + layout.plot_w() / 2.0,
            layout.mt + layout.plot_h() / 2.0).unwrap();
        w.push_str("</svg>\n");
        return svg;
    }
    render_legend(w, records, &layout);
    render_series(w, records, &layout);
    render_x_labels(w, records, &layout);
    w.push_str("</svg>\n");
    svg
}

/// Gridlines + y-axis percent labels.
fn render_grid(w: &mut String, layout: &ChartLayout) {
    use std::fmt::Write as _;
    for pct in [0_u32, 25, 50, 75, 100] {
        let y = layout.y_of(pct as f32 / 100.0);
        writeln!(w,
            "<line x1=\"{}\" y1=\"{y:.1}\" x2=\"{}\" y2=\"{y:.1}\" stroke=\"#e0e0e0\" stroke-width=\"1\"/>",
            layout.ml, layout.w - layout.mr).unwrap();
        writeln!(w,
            "<text x=\"{}\" y=\"{:.1}\" font-size=\"11\" fill=\"#757575\" text-anchor=\"end\">{pct}%</text>",
            layout.ml - 8.0, y + 4.0).unwrap();
    }
}

/// Legend (top-right): swatch + label + latest value per series.
fn render_legend(w: &mut String, records: &[HistoryRecord], layout: &ChartLayout) {
    use std::fmt::Write as _;
    let mut lx = layout.w - layout.mr - 4.0;
    for ser in series_specs().iter().rev() {
        let last = records.last().map_or(0.0, ser.value);
        let text = format!("{} {:.0}%", ser.label, last * 100.0);
        let width = text.len() as f32 * 6.6 + 26.0;
        lx -= width;
        writeln!(w,
            "<line x1=\"{lx:.1}\" y1=\"36\" x2=\"{:.1}\" y2=\"36\" stroke=\"{}\" stroke-width=\"3\"/>",
            lx + 14.0, ser.color).unwrap();
        writeln!(w, "<text x=\"{lx:.1}\" y=\"40\" font-size=\"11\" fill=\"#424242\">{text}</text>").unwrap();
    }
}

/// Series polylines + point markers.
fn render_series(w: &mut String, records: &[HistoryRecord], layout: &ChartLayout) {
    use std::fmt::Write as _;
    let n = records.len();
    // Pad single-point records to a small inset so the marker is not glued
    // to the y axis.
    let (base, step) = if n == 1 {
        (layout.ml + layout.plot_w() * 0.12, 0.0)
    } else {
        (layout.ml, layout.plot_w() / (n as f32 - 1.0).max(1.0))
    };
    let x_of = move |i: usize| base + step * i as f32;
    for ser in &series_specs() {
        let mut path = String::new();
        for (i, rec) in records.iter().enumerate() {
            let (x, y) = (x_of(i), layout.y_of((ser.value)(rec)));
            let seg = if i == 0 {
                format!("M {x:.1} {y:.1}")
            } else {
                format!(" L {x:.1} {y:.1}")
            };
            path.push_str(&seg);
        }
        writeln!(w,
            "<path d=\"{path}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>",
            ser.color).unwrap();
        for (i, rec) in records.iter().enumerate() {
            let (x, y) = (x_of(i), layout.y_of((ser.value)(rec)));
            writeln!(w,
                "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"3\" fill=\"{}\"/>",
                ser.color).unwrap();
        }
    }
}

/// X labels (date MM-DD + sha, thinned when crowded) + the footer.
fn render_x_labels(w: &mut String, records: &[HistoryRecord], layout: &ChartLayout) {
    use std::fmt::Write as _;
    let n = records.len();
    let thin = if n <= 14 { 1 } else { (n as f32 / 14.0).ceil() as usize }.max(1);
    let (base, step) = if n == 1 {
        (layout.ml + layout.plot_w() * 0.12, 0.0)
    } else {
        (layout.ml, layout.plot_w() / (n as f32 - 1.0).max(1.0))
    };
    let x_of = move |i: usize| base + step * i as f32;
    for (i, rec) in records.iter().enumerate() {
        if i % thin != 0 && i != n - 1 {
            continue;
        }
        let date = rec.ts.get(5..10).unwrap_or(&rec.ts);
        writeln!(w,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"10\" fill=\"#757575\" text-anchor=\"middle\">{date}</text>",
            x_of(i), layout.h - layout.mb + 16.0).unwrap();
        if !rec.sha.is_empty() {
            writeln!(w,
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"9\" fill=\"#9e9e9e\" text-anchor=\"middle\">{}</text>",
                x_of(i), layout.h - layout.mb + 28.0, rec.sha).unwrap();
        }
    }
    if let Some(last) = records.last() {
        writeln!(w,
            "<text x=\"{}\" y=\"{:.1}\" font-size=\"10\" fill=\"#9e9e9e\">fixtures: {} healable + {} clean | vocab {} lints | edits {}</text>",
            layout.ml, layout.h - 8.0, last.n_healable, last.n_clean, last.vocab_size, last.edits_total).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ts: &str, heal: f32) -> HistoryRecord {
        HistoryRecord {
            ts: ts.to_string(),
            sha: "abc1234".to_string(),
            n_ran: 30,
            n_healable: 24,
            n_clean: 6,
            heal_rate: heal,
            created_rate: 0.02,
            parse_safety: 1.0,
            decline_correctness: 0.83,
            edits_total: 41,
            vocab_size: 33,
            generation: 1,
        }
    }

    #[test]
    fn svg_is_well_formed_and_has_all_series() {
        let records = vec![rec("2026-08-19T10:00:00", 0.72), rec("2026-08-19T11:00:00", 0.78)];
        let svg = render_svg(&records);
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        for color in ["#2e7d32", "#1565c0", "#6a1b9a", "#c62828"] {
            assert!(svg.contains(color), "missing series {color}");
        }
        assert_eq!(svg.matches("<path ").count(), 4, "four series paths");
        assert!(svg.contains("heal_rate"), "legend present");
    }

    #[test]
    fn svg_single_point_renders() {
        let svg = render_svg(&[rec("2026-08-19T10:00:00", 0.5)]);
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn svg_empty_history_renders_placeholder() {
        let svg = render_svg(&[]);
        assert!(svg.contains("no runs yet"));
    }

    #[test]
    fn jsonl_roundtrip() {
        let dir = std::env::temp_dir().join(format!("score_bench_rt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.jsonl");
        let _ = std::fs::remove_file(&path);
        let recs = vec![rec("2026-08-19T10:00:00", 0.7), rec("2026-08-19T11:00:00", 0.75)];
        for r in &recs {
            append_history(&path, r).unwrap();
        }
        let loaded = load_history(&path);
        assert_eq!(loaded, recs);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn pre_generation_rows_deserialize_as_generation_one() {
        // The v1 series is never renumbered: a row written before the
        // generation field existed reads back as generation 1.
        let legacy = r#"{"ts":"2026-08-19T10:00:00","sha":"abc1234","n_ran":30,
            "n_healable":24,"n_clean":6,"heal_rate":0.9622642,"created_rate":0.02,
            "parse_safety":1.0,"decline_correctness":1.0,"edits_total":66,
            "vocab_size":38}"#;
        let parsed: HistoryRecord = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.generation, 1);
    }

    fn gen_rec(i: usize, heal: f32, generation: u8) -> HistoryRecord {
        HistoryRecord {
            generation,
            ..rec(&format!("2026-09-0{}T10:00:00", (i % 9) + 1), heal)
        }
    }

    #[test]
    fn saturation_rising_series_is_active() {
        let records: Vec<HistoryRecord> = (0..8)
            .map(|i| gen_rec(i, 0.80 + f32::from(i as u8) * 0.01, 1))
            .collect();
        let v = detect_saturation(&records, 1);
        match v {
            SaturationVerdict::Active { marginal } => assert!(marginal >= SATURATION_EPSILON),
            other => panic!("expected Active, got {other:?}"),
        }
    }

    #[test]
    fn saturation_flat_series_is_saturated() {
        let records: Vec<HistoryRecord> = (0..8).map(|i| gen_rec(i, 0.962_264_2, 1)).collect();
        let v = detect_saturation(&records, 1);
        match v {
            SaturationVerdict::Saturated { marginal } => {
                // float_cmp: the EXACT zero IS the claim — identical rates
                // subtract to exactly 0.0, and a near-zero would mean the
                // series moved, which changes the verdict's meaning.
                #[allow(clippy::float_cmp)]
                {
                    assert_eq!(marginal, 0.0, "identical rates subtract to exactly 0");
                }
            }
            other => panic!("expected Saturated, got {other:?}"),
        }
    }

    #[test]
    fn saturation_exactly_epsilon_is_still_active() {
        // Strict threshold: marginal == ε is NOT saturated (0.80 → 0.805
        // over the window; the f32 sum 0.005 must survive the subtraction —
        // both endpoints are exactly representable and 0.805-0.80 rounds to
        // 0.004999995... which is < ε, so use clean powers: 0.75 → 0.755).
        let mut records: Vec<HistoryRecord> = (0..6).map(|i| gen_rec(i, 0.75, 1)).collect();
        records.push(gen_rec(6, 0.755, 1));
        let v = detect_saturation(&records, 1);
        assert!(matches!(v, SaturationVerdict::Saturated { .. }), "0.005-class drift below ε saturates: {v:?}");
        // The boundary that IS active: a marginal comfortably above ε.
        records.push(gen_rec(7, 0.77, 1));
        let v = detect_saturation(&records, 1);
        assert!(matches!(v, SaturationVerdict::Active { .. }), "0.02 gain is active: {v:?}");
    }

    #[test]
    fn saturation_short_history_is_insufficient() {
        let records: Vec<HistoryRecord> = (0..5).map(|i| gen_rec(i, 0.9, 1)).collect();
        let v = detect_saturation(&records, 1);
        assert_eq!(v, SaturationVerdict::InsufficientHistory { have: 5, need: 6 });
    }

    #[test]
    fn saturation_filters_by_generation() {
        // v2 rows must never pollute the v1 verdict (and vice versa).
        let mut records: Vec<HistoryRecord> = (0..8).map(|i| gen_rec(i, 0.96, 1)).collect();
        records.extend((0..8).map(|i| gen_rec(i, 0.60 + f32::from(i as u8) * 0.02, 2)));
        assert!(matches!(
            detect_saturation(&records, 1),
            SaturationVerdict::Saturated { .. }
        ));
        assert!(matches!(
            detect_saturation(&records, 2),
            SaturationVerdict::Active { .. }
        ));
    }

    #[test]
    fn saturation_line_renders_all_verdicts() {
        assert!(saturation_line(&SaturationVerdict::Active { marginal: 0.02 }, 1).contains("ACTIVE"));
        let s = saturation_line(&SaturationVerdict::Saturated { marginal: 0.0 }, 1);
        assert!(s.contains("SATURATED") && s.contains("--generation 2"));
        let s = saturation_line(&SaturationVerdict::InsufficientHistory { have: 2, need: 6 }, 2);
        assert!(s.contains("no verdict") && s.contains("gen-2"));
    }
}

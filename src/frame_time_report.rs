//! Plan 134 T2.1 — the frame-time budget tier report: the measurement
//! VERDICT half of the `shader_perf` domain, consuming riir-shader's
//! capture-side frame-time axis (issue 018 there, commits `34ff053` +
//! `a92000a`): every automated capture (single / sweep / burst) writes a
//! `<capture>.frames.json` sidecar — schema [`SIDECAR_SCHEMA`] — beside the
//! PNG. A perf-fix A/B on a healed shader diffs these sidecars, not just the
//! PNGs: the capture is the fidelity leg (Bench 093, hash-set equality),
//! this report is the perf leg (p50/p90/mean vs the carried budget).
//!
//! Two surfaces, one module:
//!
//! 1. **Budget report** (`--frame-time-report`): per-effect p50/p90/mean vs
//!    the sidecar-carried `frame_budget_ms` (`--frame-budget <MS>`
//!    overrides), an `OK`/`OVER` verdict per row, and the heal hint — the
//!    over-budget effects are exactly where `cargo heal --domain
//!    shader_perf` (the sample-budget / loop-boundedness corpus) should run
//!    next. The budget tier PRIORITIZES the source-scan rules; it does not
//!    replace them.
//! 2. **A/B diff** (`--vs <PATH>`): pairs pre/post sidecars by effect slug
//!    and verdicts each pair at `--frame-delta-pct` (default
//!    [`DEFAULT_DELTA_PCT`] on p50). Two evidence guards are load-bearing:
//!    **warmup parity** — a pre/post pair captured under different warmup
//!    configs is not evidence (the warmup floor is what makes sweeps
//!    comparable, the riir-shader 0.19 lesson: uncapped frames made the
//!    45-frame warmup ~50 ms) — and **pairing uniqueness** — an effect with
//!    multiple sidecars on either side (the burst-diagnostic class) refuses
//!    the A/B rather than letting the tool invent pairings.
//!
//! Deliberately UNGATED and dependency-free beyond serde (the
//! `corpus_coverage` / `horizon_report` law): pure over its inputs, reads
//! sidecar files directly, no domain machinery — lib-tested at default
//! features, and the CLI arm works on every feature build. Report-first,
//! gates nothing; refuses exit 1 on a blind scan (the frontier-report law):
//! zero sidecars, ANY unparsable sidecar, or (A/B) any warmup/ambiguity
//! mismatch with zero usable pairs — an incomplete measurement must RED,
//! never print a flattering partial as a pass.
//!
//! Noise discipline (documented, not mechanized — one run is one run): a
//! load-sensitive p50 passes the re-run and a real delta does not; confirm
//! any `IMPROVED`/`REGRESSED` verdict with a second sweep pair before
//! acting on it (the katgpt-rs x86_64-matrix lesson, four different failing
//! sets across four runs of one commit).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The sidecar schema this report consumes (riir-shader issue 018). A
/// sidecar naming any other schema is refused, not parsed best-effort — a
/// v2 with moved fields must RED here, not silently misread.
pub const SIDECAR_SCHEMA: &str = "riir_shader_frame_time_v1";

/// Default A/B verdict threshold on the p50 delta, in percent.
pub const DEFAULT_DELTA_PCT: f64 = 5.0;

/// One riir-shader frame-time sidecar. Stats are `Option` because the
/// schema emits JSON `null` for a no-sample capture (a zero would read as a
/// measurement — the riir-shader nulls-not-zeros rule).
#[derive(Debug, Clone, Deserialize)]
pub struct FrameTimeSidecar {
    /// Must equal [`SIDECAR_SCHEMA`].
    pub schema: String,
    /// The effect slug.
    pub effect: String,
    /// The capture path as riir-shader saw it.
    pub capture: String,
    /// Warmup frames after the switch (the capture gate's frame half).
    pub warmup_frames: Option<u32>,
    /// Warmup minimum wall ms (the capture gate's time half — the one that
    /// carries the real load under an uncapped loop).
    pub warmup_min_ms: Option<u64>,
    /// The budget echo (`RIIR_SHADER_FRAME_BUDGET_MS`), if the sweep set one.
    pub frame_budget_ms: Option<f64>,
    /// Frames sampled since the switch (bounded ring).
    pub n_samples: Option<u64>,
    pub mean_ms: Option<f64>,
    /// Nearest-rank p50 over the sample ring.
    pub p50_ms: Option<f64>,
    pub p90_ms: Option<f64>,
    pub min_ms: Option<f64>,
    pub max_ms: Option<f64>,
    pub mean_fps: Option<f64>,
    /// The raw per-frame ms ring (`samples_ms`).
    pub samples_ms: Option<Vec<f64>>,
}

/// A sidecar refused before it became a row.
#[derive(Debug, Clone)]
pub enum SidecarError {
    /// The file could not be read.
    Io(String),
    /// The file is not valid JSON (or not the documented shape).
    Parse(String),
    /// Valid JSON, wrong `schema` string — never misread best-effort.
    Schema { expected: String, found: String },
}

impl std::fmt::Display for SidecarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Parse(e) => write!(f, "parse: {e}"),
            Self::Schema { expected, found } => {
                write!(f, "schema: expected {expected}, found {found:?}")
            }
        }
    }
}

/// Parse sidecar TEXT (schema-pinned). The unit-test seam over
/// [`load_sidecar`].
///
/// # Errors
///
/// [`SidecarError::Parse`] on invalid JSON, [`SidecarError::Schema`] when
/// the `schema` field is not [`SIDECAR_SCHEMA`].
pub fn parse_sidecar_text(text: &str) -> Result<FrameTimeSidecar, SidecarError> {
    let sidecar: FrameTimeSidecar = serde_json::from_str(text)
        .map_err(|e| SidecarError::Parse(e.to_string()))?;
    if sidecar.schema != SIDECAR_SCHEMA {
        return Err(SidecarError::Schema {
            expected: SIDECAR_SCHEMA.to_string(),
            found: sidecar.schema,
        });
    }
    Ok(sidecar)
}

/// Read + parse one sidecar file.
///
/// # Errors
///
/// [`SidecarError::Io`] / [`SidecarError::Parse`] / [`SidecarError::Schema`].
pub fn load_sidecar(path: &Path) -> Result<FrameTimeSidecar, SidecarError> {
    let text = std::fs::read_to_string(path).map_err(|e| SidecarError::Io(e.to_string()))?;
    parse_sidecar_text(&text)
}

/// A sidecar that parsed.
#[derive(Debug, Clone)]
pub struct LoadedSidecar {
    /// Where it came from (reported verbatim for row provenance).
    pub path: PathBuf,
    /// The parsed sidecar.
    pub sidecar: FrameTimeSidecar,
}

/// A sidecar that did not parse (named — the report must be able to say
/// exactly which file of an A/B pair is missing evidence).
#[derive(Debug, Clone)]
pub struct SidecarFailure {
    /// Where it came from.
    pub path: PathBuf,
    /// The [`SidecarError`], rendered.
    pub reason: String,
}

/// Collect sidecars from explicit files and/or directories (a directory is
/// walked recursively for `*.frames.json`; hidden dirs + `target`/`vendor`/
/// `node_modules` skipped). Deterministic: results sort by path.
#[must_use]
pub fn collect_sidecars(paths: &[PathBuf]) -> (Vec<LoadedSidecar>, Vec<SidecarFailure>) {
    let mut files: Vec<PathBuf> = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk_dir(p, &mut files);
        } else {
            files.push(p.clone());
        }
    }
    files.sort();
    let mut loaded = Vec::new();
    let mut failures = Vec::new();
    for path in files {
        match load_sidecar(&path) {
            Ok(sidecar) => loaded.push(LoadedSidecar { path, sidecar }),
            Err(e) => failures.push(SidecarFailure {
                path,
                reason: e.to_string(),
            }),
        }
    }
    (loaded, failures)
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(std::result::Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.')
                || matches!(name, "target" | "vendor" | "node_modules")
            {
                continue;
            }
            walk_dir(&path, out);
        } else if path.file_name().is_some_and(|n| {
            n.to_string_lossy().ends_with(".frames.json")
        }) {
            out.push(path);
        }
    }
}

/// One budget-report row: the measured stats + the budget verdict.
#[derive(Debug, Clone)]
pub struct BudgetRow {
    /// Sidecar path (verbatim provenance).
    pub path: PathBuf,
    /// Effect slug.
    pub effect: String,
    /// Frames sampled (None = the no-sample capture class).
    pub n_samples: Option<u64>,
    pub p50_ms: Option<f64>,
    pub p90_ms: Option<f64>,
    pub mean_ms: Option<f64>,
    /// The budget in force: the CLI override if given, else the sidecar's
    /// echo; None = neither (no verdict is possible — reported, never
    /// implied).
    pub budget_ms: Option<f64>,
    /// `Some(true)` = p50 over budget. None = no budget or no p50.
    pub over_budget: Option<bool>,
    /// The warmup config in force (the A/B parity key).
    pub warmup: Option<(u32, u64)>,
}

/// Build budget rows. `budget_override` (from `--frame-budget`) wins over
/// the sidecar echo — an explicit CLI budget is the stricter statement.
#[must_use]
pub fn budget_rows(loaded: &[LoadedSidecar], budget_override: Option<f64>) -> Vec<BudgetRow> {
    let mut rows: Vec<BudgetRow> = loaded
        .iter()
        .map(|l| {
            let sc = &l.sidecar;
            let budget_ms = budget_override.or(sc.frame_budget_ms);
            let over_budget = budget_ms.and_then(|b| sc.p50_ms.map(|p| p > b));
            BudgetRow {
                path: l.path.clone(),
                effect: sc.effect.clone(),
                n_samples: sc.n_samples,
                p50_ms: sc.p50_ms,
                p90_ms: sc.p90_ms,
                mean_ms: sc.mean_ms,
                budget_ms,
                over_budget,
                warmup: sc.warmup_frames.zip(sc.warmup_min_ms),
            }
        })
        .collect();
    rows.sort_by(|a, b| (&a.effect, &a.path).cmp(&(&b.effect, &b.path)));
    rows
}

/// One side of an A/B pair.
#[derive(Debug, Clone)]
pub struct AbSide {
    pub p50_ms: Option<f64>,
    pub warmup: Option<(u32, u64)>,
    /// How many sidecars carried this effect on this side (always 1 in a
    /// well-formed run; >1 is the refusal class below).
    pub count: usize,
}

/// The A/B verdict per effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbVerdict {
    /// p50 dropped by at least the threshold.
    Improved,
    /// p50 rose by at least the threshold.
    Regressed,
    /// Within the threshold (and yes — "flat" after a claimed perf fix is
    /// exactly what the healer's gain claims must survive).
    Flat,
    /// The effect exists on only one side.
    Unpaired,
    /// Paired, but a side carries no samples (null stats).
    NoSamples,
}

impl AbVerdict {
    /// Stable label for the report table.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Improved => "IMPROVED",
            Self::Regressed => "REGRESSED",
            Self::Flat => "FLAT",
            Self::Unpaired => "UNPAIRED",
            Self::NoSamples => "NO-SAMPLES",
        }
    }
}

/// An A/B refusal row: the pair is not evidence.
#[derive(Debug, Clone)]
pub enum AbMismatch {
    /// Different warmup configs across the pair.
    WarmupParity {
        effect: String,
        pre: Option<(u32, u64)>,
        post: Option<(u32, u64)>,
    },
    /// Multiple sidecars for one effect on a side (the burst class) — the
    /// tool refuses to invent pairings.
    Ambiguous { effect: String, pre: usize, post: usize },
}

/// The A/B diff report.
#[derive(Debug, Clone)]
pub struct AbReport {
    /// One row per effect seen on either side, sorted by effect.
    pub rows: Vec<(String, AbSide, AbSide, Option<f64>, AbVerdict)>,
    /// Refusal rows (warmup parity + pairing ambiguity).
    pub mismatches: Vec<AbMismatch>,
    /// Named unparsable sidecars, both sides.
    pub failures: Vec<SidecarFailure>,
    /// The threshold the verdicts used (echoed in the header).
    pub threshold_pct: f64,
}

/// Pair pre/post budget rows by effect slug and verdict the deltas.
///
/// Verdicts: `(post - pre) / pre * 100` on p50; `<= -threshold_pct` →
/// [`AbVerdict::Improved`], `>= +threshold_pct` → [`AbVerdict::Regressed`],
/// else [`AbVerdict::Flat`]. Unpaired and no-sample effects get their own
/// verdicts; warmup-parity and pairing-ambiguity refusals land in
/// `mismatches`.
#[must_use]
pub fn diff_ab(
    pre: &[BudgetRow],
    post: &[BudgetRow],
    threshold_pct: f64,
    pre_failures: Vec<SidecarFailure>,
    post_failures: Vec<SidecarFailure>,
) -> AbReport {
    let mut pre_by: BTreeMap<&str, Vec<&BudgetRow>> = BTreeMap::new();
    let mut post_by: BTreeMap<&str, Vec<&BudgetRow>> = BTreeMap::new();
    for r in pre {
        pre_by.entry(r.effect.as_str()).or_default().push(r);
    }
    for r in post {
        post_by.entry(r.effect.as_str()).or_default().push(r);
    }
    let mut effects: Vec<&str> = pre_by.keys().chain(post_by.keys()).copied().collect();
    effects.sort_unstable();
    effects.dedup();

    let mut rows = Vec::new();
    let mut mismatches = Vec::new();
    for effect in effects {
        let pre_list = pre_by.get(effect);
        let post_list = post_by.get(effect);
        let pre_count = pre_list.map_or(0, Vec::len);
        let post_count = post_list.map_or(0, Vec::len);
        let pre_row = pre_list.and_then(|l| l.first().copied());
        let post_row = post_list.and_then(|l| l.first().copied());
        if pre_count > 1 || post_count > 1 {
            mismatches.push(AbMismatch::Ambiguous {
                effect: (*effect).to_string(),
                pre: pre_count,
                post: post_count,
            });
            continue;
        }
        let (Some(pre_row), Some(post_row)) = (pre_row, post_row) else {
            // Unpaired: build the row with whichever side exists.
            let p = pre_row.map(|r| AbSide {
                p50_ms: r.p50_ms,
                warmup: r.warmup,
                count: 1,
            });
            let q = post_row.map(|r| AbSide {
                p50_ms: r.p50_ms,
                warmup: r.warmup,
                count: 1,
            });
            rows.push((
                (*effect).to_string(),
                p.unwrap_or(AbSide { p50_ms: None, warmup: None, count: 0 }),
                q.unwrap_or(AbSide { p50_ms: None, warmup: None, count: 0 }),
                None,
                AbVerdict::Unpaired,
            ));
            continue;
        };
        // Warmup parity: both sides must carry the SAME config. An absent
        // field on either side (a pre-warmup-field sidecar) is also a
        // mismatch — parity must be POSITIVE evidence.
        match (pre_row.warmup, post_row.warmup) {
            (Some(w), Some(v)) if w == v => {}
            (pre_w, post_w) => {
                mismatches.push(AbMismatch::WarmupParity {
                    effect: (*effect).to_string(),
                    pre: pre_w,
                    post: post_w,
                });
                continue;
            }
        }
        let (Some(a), Some(b)) = (pre_row.p50_ms, post_row.p50_ms) else {
            rows.push((
                (*effect).to_string(),
                AbSide { p50_ms: pre_row.p50_ms, warmup: pre_row.warmup, count: 1 },
                AbSide { p50_ms: post_row.p50_ms, warmup: post_row.warmup, count: 1 },
                None,
                AbVerdict::NoSamples,
            ));
            continue;
        };
        let delta_pct = (b - a) / a * 100.0;
        let verdict = if delta_pct <= -threshold_pct {
            AbVerdict::Improved
        } else if delta_pct >= threshold_pct {
            AbVerdict::Regressed
        } else {
            AbVerdict::Flat
        };
        rows.push((
            (*effect).to_string(),
            AbSide { p50_ms: Some(a), warmup: pre_row.warmup, count: 1 },
            AbSide { p50_ms: Some(b), warmup: post_row.warmup, count: 1 },
            Some(delta_pct),
            verdict,
        ));
    }

    let mut failures = pre_failures;
    failures.extend(post_failures);
    AbReport {
        rows,
        mismatches,
        failures,
        threshold_pct,
    }
}

impl AbReport {
    /// Pairs with a usable delta verdict (the evidence count).
    #[must_use]
    pub fn usable_pairs(&self) -> usize {
        self.rows
            .iter()
            .filter(|(.., v)| matches!(v, AbVerdict::Improved | AbVerdict::Regressed | AbVerdict::Flat))
            .count()
    }

    /// The frontier-report law: an A/B with unparsable sidecars, mismatches,
    /// or zero usable pairs is not a pass — it must exit nonzero.
    #[must_use]
    pub fn blind(&self) -> bool {
        !self.failures.is_empty() || !self.mismatches.is_empty() || self.usable_pairs() == 0
    }

    /// Render the A/B table.
    #[must_use]
    pub fn render(&self) -> String {
        let mut s = String::with_capacity(1024);
        let _ = writeln!(
            s,
            "frame-time A/B diff (Plan 134 T2.1, schema {SIDECAR_SCHEMA}) — verdicts at ±{:.1}% on p50",
            self.threshold_pct
        );
        let _ = writeln!(
            s,
            "paired: {} · unpaired/no-samples: {} · mismatches: {} · unparsable: {}",
            self.usable_pairs(),
            self.rows.iter().filter(|(.., v)| matches!(v, AbVerdict::Unpaired | AbVerdict::NoSamples)).count(),
            self.mismatches.len(),
            self.failures.len(),
        );
        let _ = writeln!(
            s,
            "{:<26} {:>10} {:>10} {:>9}  verdict",
            "effect", "pre p50", "post p50", "delta %"
        );
        for (effect, pre, post, delta, verdict) in &self.rows {
            let fmt = |v: &Option<f64>| v.map_or_else(|| "n/a".to_string(), |x| format!("{x:.3}"));
            let _ = writeln!(
                s,
                "{:<26} {:>10} {:>10} {:>9}  {}",
                truncate(effect, 26),
                fmt(&pre.p50_ms),
                fmt(&post.p50_ms),
                delta.map_or_else(|| "n/a".to_string(), |d| format!("{d:+.2}")),
                verdict.label(),
            );
        }
        for m in &self.mismatches {
            match m {
                AbMismatch::WarmupParity { effect, pre, post } => {
                    let _ = writeln!(
                        s,
                        "MISMATCH warmup-parity {effect}: pre {pre:?} vs post {post:?} — not evidence (same warmup config required)"
                    );
                }
                AbMismatch::Ambiguous { effect, pre, post } => {
                    let _ = writeln!(
                        s,
                        "MISMATCH ambiguous-pairing {effect}: {pre} pre / {post} post sidecars — one per effect per side required (bursts refuse)"
                    );
                }
            }
        }
        for f in &self.failures {
            let _ = writeln!(s, "UNPARSABLE {}: {}", f.path.display(), f.reason);
        }
        s
    }
}

/// The single-run budget report.
#[derive(Debug, Clone)]
pub struct FrameTimeReport {
    /// One row per sidecar (sorted by effect, then path).
    pub rows: Vec<BudgetRow>,
    /// Named unparsable sidecars.
    pub failures: Vec<SidecarFailure>,
}

impl FrameTimeReport {
    /// The frontier-report law: zero sidecars or ANY unparsable one is not
    /// a pass.
    #[must_use]
    pub fn blind(&self) -> bool {
        self.rows.is_empty() || !self.failures.is_empty()
    }

    /// Over-budget row count (the heal-hint population).
    #[must_use]
    pub fn over_budget(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.over_budget == Some(true))
            .count()
    }

    /// Render the budget table.
    #[must_use]
    pub fn render(&self) -> String {
        let mut s = String::with_capacity(1024);
        let _ = writeln!(
            s,
            "frame-time budget report (Plan 134 T2.1, schema {SIDECAR_SCHEMA})"
        );
        let _ = writeln!(
            s,
            "sidecars: {} · unparsable: {}",
            self.rows.len(),
            self.failures.len()
        );
        let _ = writeln!(
            s,
            "{:<26} {:>10} {:>10} {:>10} {:>10}  verdict",
            "effect", "p50 ms", "p90 ms", "mean ms", "budget ms"
        );
        for r in &self.rows {
            let fmt = |v: &Option<f64>| v.map_or_else(|| "n/a".to_string(), |x| format!("{x:.3}"));
            let verdict = match r.over_budget {
                Some(true) => "OVER",
                Some(false) => "OK",
                None => "n/a",
            };
            let _ = writeln!(
                s,
                "{:<26} {:>10} {:>10} {:>10} {:>10}  {verdict}",
                truncate(&r.effect, 26),
                fmt(&r.p50_ms),
                fmt(&r.p90_ms),
                fmt(&r.mean_ms),
                fmt(&r.budget_ms),
            );
        }
        if self.over_budget() > 0 {
            let _ = writeln!(
                s,
                "hint: {} effect(s) over budget — run `cargo heal --domain shader_perf` on their WGSL (the sample-budget / loop-boundedness corpus)",
                self.over_budget()
            );
        }
        for f in &self.failures {
            let _ = writeln!(s, "UNPARSABLE {}: {}", f.path.display(), f.reason);
        }
        s
    }
}

/// The A/B diff report.
/// Bound an effect slug to the table column (slugs are short; paths are not).
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let keep = max.saturating_sub(1);
        let mut out: String = s.chars().take(keep).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sidecar_text(effect: &str, p50: f64, warmup: (u32, u64)) -> String {
        format!(
            r#"{{
  "schema": "riir_shader_frame_time_v1",
  "effect": "{effect}",
  "capture": "/tmp/x/{effect}.png",
  "warmup_frames": {},
  "warmup_min_ms": {},
  "frame_budget_ms": 16.7,
  "n_samples": 100,
  "mean_ms": {p50:.3},
  "p50_ms": {p50:.3},
  "p90_ms": {p50:.3},
  "min_ms": 1.0,
  "max_ms": 60.0,
  "mean_fps": 60.0,
  "samples_ms": [1.0, 2.0]
}}
"#,
            warmup.0, warmup.1
        )
    }

    fn loaded(effect: &str, p50: f64, warmup: (u32, u64)) -> LoadedSidecar {
        LoadedSidecar {
            path: PathBuf::from(format!("/x/{effect}.frames.json")),
            sidecar: parse_sidecar_text(&sidecar_text(effect, p50, warmup)).expect("parses"),
        }
    }

    #[test]
    fn parses_a_valid_sidecar() {
        let sc = parse_sidecar_text(&sidecar_text("gradient", 10.5, (45, 750))).expect("valid");
        assert_eq!(sc.effect, "gradient");
        assert_eq!(sc.p50_ms, Some(10.5));
        assert_eq!(sc.warmup_frames, Some(45));
        assert_eq!(sc.warmup_min_ms, Some(750));
        assert_eq!(sc.frame_budget_ms, Some(16.7));
    }

    #[test]
    fn refuses_a_wrong_schema() {
        let text = sidecar_text("x", 1.0, (1, 1)).replace("v1", "v9");
        let err = parse_sidecar_text(&text).expect_err("must refuse");
        assert!(matches!(err, SidecarError::Schema { .. }), "{err}");
    }

    #[test]
    fn refuses_malformed_json() {
        let err = parse_sidecar_text("{not json").expect_err("must refuse");
        assert!(matches!(err, SidecarError::Parse(_)), "{err}");
    }

    #[test]
    fn null_stats_sidecar_yields_verdict_na() {
        let text = r#"{
  "schema": "riir_shader_frame_time_v1",
  "effect": "x",
  "capture": "/x.png",
  "warmup_frames": 45,
  "warmup_min_ms": 750,
  "frame_budget_ms": null,
  "n_samples": null,
  "mean_ms": null,
  "p50_ms": null,
  "p90_ms": null,
  "min_ms": null,
  "max_ms": null,
  "mean_fps": null,
  "samples_ms": []
}"#;
        let sc = parse_sidecar_text(text).expect("parses");
        assert_eq!(sc.p50_ms, None);
        let rows = budget_rows(
            &[LoadedSidecar { path: PathBuf::from("/x"), sidecar: sc }],
            None,
        );
        assert_eq!(rows[0].over_budget, None);
    }

    #[test]
    fn budget_override_wins_over_sidecar_echo() {
        let rows = budget_rows(&[loaded("a", 20.0, (45, 750))], Some(10.0));
        assert_eq!(rows[0].budget_ms, Some(10.0));
        assert_eq!(rows[0].over_budget, Some(true));
    }

    #[test]
    fn sidecar_budget_applies_without_override() {
        let rows = budget_rows(&[loaded("a", 20.0, (45, 750))], None);
        assert_eq!(rows[0].budget_ms, Some(16.7));
        assert_eq!(rows[0].over_budget, Some(true));
    }

    #[test]
    fn diff_verdicts_at_the_threshold_edges() {
        let pre = budget_rows(&[loaded("a", 10.0, (45, 750))], None);
        // exactly -5% → Improved (<= -threshold)
        let post = budget_rows(&[loaded("a", 9.5, (45, 750))], None);
        let rep = diff_ab(&pre, &post, 5.0, Vec::new(), Vec::new());
        assert_eq!(rep.rows[0].4, AbVerdict::Improved);
        // exactly +5% → Regressed
        let post = budget_rows(&[loaded("a", 10.5, (45, 750))], None);
        let rep = diff_ab(&pre, &post, 5.0, Vec::new(), Vec::new());
        assert_eq!(rep.rows[0].4, AbVerdict::Regressed);
        // +4.9% → Flat
        let post = budget_rows(&[loaded("a", 10.49, (45, 750))], None);
        let rep = diff_ab(&pre, &post, 5.0, Vec::new(), Vec::new());
        assert_eq!(rep.rows[0].4, AbVerdict::Flat);
    }

    #[test]
    fn diff_flags_unpaired_no_samples_and_warmup_mismatch() {
        let pre = budget_rows(
            &[
                loaded("paired", 10.0, (45, 750)),
                loaded("only_pre", 10.0, (45, 750)),
                loaded("nulls", 10.0, (45, 750)),
            ],
            None,
        );
        // Make the "nulls" side carry no p50.
        let mut nulls = loaded("nulls", 10.0, (45, 750));
        nulls.sidecar.p50_ms = None;
        let post = budget_rows(
            &[
                loaded("paired", 10.0, (45, 750)),
                loaded("only_post", 10.0, (45, 750)),
                nulls,
            ],
            None,
        );
        // Warmup mismatch on a third effect.
        let pre2 = budget_rows(&[loaded("warm", 10.0, (45, 750))], None);
        let post2 = budget_rows(&[loaded("warm", 10.0, (10, 100))], None);
        let rep = diff_ab(&pre2, &post2, 5.0, Vec::new(), Vec::new());
        assert_eq!(rep.mismatches.len(), 1);
        assert!(matches!(rep.mismatches[0], AbMismatch::WarmupParity { .. }));

        let rep = diff_ab(&pre, &post, 5.0, Vec::new(), Vec::new());
        let verdict_of = |e: &str| {
            rep.rows
                .iter()
                .find(|(effect, ..)| effect == e)
                .map(|(.., v)| *v)
                .expect("row exists")
        };
        assert_eq!(verdict_of("paired"), AbVerdict::Flat);
        assert_eq!(verdict_of("only_pre"), AbVerdict::Unpaired);
        assert_eq!(verdict_of("only_post"), AbVerdict::Unpaired);
        assert_eq!(verdict_of("nulls"), AbVerdict::NoSamples);
    }

    #[test]
    fn diff_refuses_ambiguous_pairings() {
        let pre = budget_rows(&[loaded("a", 10.0, (1, 1))], None);
        let post = budget_rows(
            &[
                loaded("a", 10.0, (1, 1)),
                LoadedSidecar {
                    path: PathBuf::from("/x/a.b02.frames.json"),
                    sidecar: parse_sidecar_text(&sidecar_text("a", 10.0, (1, 1)))
                        .expect("parses"),
                },
            ],
            None,
        );
        let rep = diff_ab(&pre, &post, 5.0, Vec::new(), Vec::new());
        assert_eq!(rep.rows.len(), 0);
        assert_eq!(rep.mismatches.len(), 1);
        assert!(matches!(rep.mismatches[0], AbMismatch::Ambiguous { .. }));
        assert!(rep.blind());
    }

    #[test]
    fn walker_matches_the_sidecar_suffix_not_the_extension() {
        // `gradient.frames.json` has Path::extension() == "json" — the
        // walker must match the FULL filename suffix, or every real sweep
        // dir reads as empty (the blind-walk class, caught live by the CLI
        // contract test before any consumer ran).
        let dir = std::env::temp_dir().join(format!("fft_walk_{}", std::process::id()));
        let nested = dir.join("nested");
        std::fs::create_dir_all(&nested).expect("mkdir");
        std::fs::write(dir.join("gradient.frames.json"), sidecar_text("gradient", 1.0, (1, 1)))
            .expect("write");
        std::fs::write(nested.join("ocean.frames.json"), sidecar_text("ocean", 2.0, (1, 1)))
            .expect("write");
        std::fs::write(dir.join("ignored.json"), "{}").expect("write");
        let (loaded, failures) = collect_sidecars(std::slice::from_ref(&dir));
        assert!(failures.is_empty(), "{failures:?}");
        let effects: Vec<&str> = loaded.iter().map(|l| l.sidecar.effect.as_str()).collect();
        assert_eq!(effects.len(), 2, "{effects:?}");
        assert!(effects.contains(&"gradient") && effects.contains(&"ocean"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn blind_laws() {
        // Zero rows → blind.
        let rep = FrameTimeReport { rows: Vec::new(), failures: Vec::new() };
        assert!(rep.blind());
        // Any unparsable → blind.
        let rep = FrameTimeReport {
            rows: vec![BudgetRow {
                path: PathBuf::from("/x"),
                effect: "a".into(),
                n_samples: Some(10),
                p50_ms: Some(1.0),
                p90_ms: Some(1.0),
                mean_ms: Some(1.0),
                budget_ms: Some(1.0),
                over_budget: Some(false),
                warmup: Some((1, 1)),
            }],
            failures: vec![SidecarFailure {
                path: PathBuf::from("/bad"),
                reason: "parse: nope".into(),
            }],
        };
        assert!(rep.blind());
        // Usable A/B with clean sides → not blind.
        let rep = diff_ab(
            &budget_rows(&[loaded("a", 10.0, (1, 1))], None),
            &budget_rows(&[loaded("a", 10.0, (1, 1))], None),
            5.0,
            Vec::new(),
            Vec::new(),
        );
        assert!(!rep.blind());
    }
}

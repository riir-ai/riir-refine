//! Hotpath report parsing + SHA keying — the profile-guided-healing oracle
//! lane's data plane (Issue 146 T0.1 / Proposal 020 / Research 234+235).
//!
//! The healer has no view of what is actually hot at runtime. This module
//! makes a [hotpath](https://github.com/pawurb/hotpath-rs) JSON report
//! consumable as a SECOND oracle lane (the `--clippy-json` anchor-lane
//! pattern): spans at measured-hot sites anchor ahead of similarity ranking,
//! tagged `[hot: …]`.
//!
//! ## What this module owns (T0.1)
//! - A MINIMAL serde subset of hotpath's `JsonReport` (schema read from the
//!   source at `pawurb/hotpath-rs` tip 2026-10-04; the R234 pin `ac4096e`
//!   was not fetchable by SHA — disclosed here; ~~the compat gate is T0.4's
//!   e2e with a REAL hotpath run~~ **compat CLOSED BY EXECUTION 2026-10-05**:
//!   a real `hotpath` 0.28.4 report (crates.io, `hotpath`+`hotpath-alloc`
//!   features) parses clean through this subset — sections, `location`,
//!   `meta.source_root`, and the human-number forms all round-trip — in
//!   Issue 146 T0.4's e2e gate `scripts/hotpath_e2e_poc.sh`).
//! - Tolerant parsing of hotpath's human-formatted numbers (durations
//!   `"123 ns"` / `"1.5 µs"` / `"2.3 ms"` / `"1.00 s"`; bytes `"12 B"` /
//!   `"1.2 KB"` / …; `"N/A"` → `None`) — the inverse of hotpath's own
//!   `format_delay`/`format_bytes`.
//! - [`KeyVerdict`] — the stale-profile hazard's gate: a report carrying
//!   `meta.git.sha` is REFUSED on mismatch against the target repo's HEAD
//!   (a profile at commit X must not anchor spans at commit Y). An unkeyed
//!   report (plain `hotpath`, no `hotpath-cloud` git info) cannot mismatch:
//!   it proceeds with a LOUD unkeyed disclosure, never a silent pass.
//! - [`hot_spans`] — the timing entries (joined with alloc entries by
//!   `(file, line, name)`) that carry a source `location`, RANKED by
//!   measured time share (`calls × avg`) — the ranking axis only. Alloc
//!   bytes/count ride along as the DETERMINISTIC verification axis
//!   (Research 032's law: instrumented timing is attribution, never a
//!   verification target — B129).
//!
//! ## What this deliberately does NOT do (T0.2+/T0.3)
//! - Span anchoring lives in `draft::latent_matcher`
//!   (`SpanSuggestion::hot` + `suggest_file_hot_anchored`, T0.2).
//! - The fix-lane attribute guard DOES live here ([`attribute_line_ranges`],
//!   T0.3): the one place that knows hotpath's own attribute vocabulary.
//!
//! Zero new dependencies: `serde_json` is already in the lock. Ungated —
//! pure data plumbing, lib-tested at default features (the house law).

use serde::Deserialize;

// ─────────────────────────────────────────────────────────────────────────
// The report subset
// ─────────────────────────────────────────────────────────────────────────

/// Minimal mirror of hotpath's `JsonReport` — only the sections the oracle
/// lane consumes. Unknown fields are IGNORED (serde default), so a newer
/// hotpath adding sections cannot break the parse; the sections we do read
/// are the compat surface and are contract-tested against the fixture below.
#[derive(Debug, Clone, Deserialize)]
pub struct HotpathReport {
    pub meta: HotpathMeta,
    #[serde(default)]
    pub functions_timing: Option<HotpathFunctionsList>,
    #[serde(default)]
    pub functions_alloc: Option<HotpathFunctionsList>,
}

/// The `meta` subset: `git.sha` keys the report; `source_root` is the prefix
/// to prepend to relative `location.file` values ("" when the process ran
/// from the repo root).
#[derive(Debug, Clone, Deserialize)]
pub struct HotpathMeta {
    #[serde(default)]
    pub git: Option<HotpathGitInfo>,
    #[serde(default)]
    pub source_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HotpathGitInfo {
    pub sha: String,
}

/// One functions section (timing OR alloc — hotpath reuses the list shape).
#[derive(Debug, Clone, Deserialize)]
pub struct HotpathFunctionsList {
    #[serde(default)]
    pub data: Vec<HotpathFunctionEntry>,
}

/// One measured function. `avg`/`total` are hotpath's human-formatted
/// STRINGS; `percent_total` likewise. `location` is present only in builds
/// with the call-site registry (`#[measure]`-macro locations) — entries
/// without one cannot anchor and are skipped by [`hot_spans`].
#[derive(Debug, Clone, Deserialize)]
pub struct HotpathFunctionEntry {
    pub name: String,
    #[serde(default)]
    pub calls: u64,
    #[serde(default)]
    pub avg: Option<String>,
    #[serde(default)]
    pub total: Option<String>,
    #[serde(default)]
    pub percent_total: Option<String>,
    #[serde(default)]
    pub location: Option<HotpathLocation>,
}

/// Structured source location, as captured by `file!()` —
/// workspace-root-relative for workspace members.
#[derive(Debug, Clone, Deserialize)]
pub struct HotpathLocation {
    pub file: String,
    pub line: u32,
}

// ─────────────────────────────────────────────────────────────────────────
// Human-number parsing (the inverse of hotpath's format_delay/format_bytes)
// ─────────────────────────────────────────────────────────────────────────

/// Parse a hotpath duration string into nanoseconds.
/// `"8 ns"` → `8` · `"1.5 µs"` → `1500` · `"2.3 ms"` → `2_300_000` ·
/// `"1.00 s"` → `1_000_000_000`. `"N/A"`/malformed → `None`.
#[must_use]
pub fn parse_duration_ns(s: &str) -> Option<u64> {
    let s = s.trim();
    // Percentile keys reuse the number forms without a unit sometimes;
    // require a unit so a bare number is not silently misread.
    let (num, unit) = s.split_once(' ')?;
    let scale = match unit.trim() {
        "ns" => 1_u64,
        "µs" | "us" => 1_000,
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        _ => return None,
    };
    scaled_u64(num, scale)
}

/// Parse a hotpath byte-count string. `"12 B"` → `12` · `"1.2 KB"` → `1200` ·
/// `"3.4 MB"` → `3_400_000` (hotpath's `format_bytes` is decimal-prefixed;
/// `KiB`-style is accepted defensively). `"N/A"`/malformed → `None`.
#[must_use]
pub fn parse_bytes(s: &str) -> Option<u64> {
    let s = s.trim();
    let (num, unit) = s.split_once(' ')?;
    let unit = unit.trim();
    let scale = match unit {
        "B" => 1_u64,
        "KB" => 10_u64.pow(3),
        "MB" => 10_u64.pow(6),
        "GB" => 10_u64.pow(9),
        "TB" => 10_u64.pow(12),
        "KiB" => 1 << 10,
        "MiB" => 1 << 20,
        "GiB" => 1 << 30,
        _ => return None,
    };
    scaled_u64(num, scale)
}

/// Parse a percentage string (`"12.34%"`). `None` on anything else.
#[must_use]
pub fn parse_percent(s: &str) -> Option<f64> {
    let s = s.trim();
    let num = s.strip_suffix('%')?;
    let v: f64 = num.trim().parse().ok()?;
    v.is_finite().then_some(v)
}

/// Scale a decimal literal (`"1.5"`, `"123"`, `"1.00"`) by `scale` using
/// exact integer math — no `f64` intermediate (hotpath prints at most a
/// handful of significant digits, and a float round-trip is a precision
/// hazard the pedantic casts would rightly flag). Digits beyond what the
/// scale can carry round half-up on the first dropped digit; checked ops
/// throughout so an adversarial report yields `None`, not a wrap.
fn scaled_u64(num: &str, scale: u64) -> Option<u64> {
    let num = num.trim();
    if num.is_empty() {
        return None;
    }
    let (int_part, frac_part) = match num.split_once('.') {
        Some((i, f)) => (i, f),
        None => (num, ""),
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    if !int_part.chars().all(|c| c.is_ascii_digit())
        || !frac_part.chars().all(|c| c.is_ascii_digit())
    {
        return None; // signs, exponents, separators: not hotpath's forms
    }
    let int_val: u64 = if int_part.is_empty() {
        0
    } else {
        int_part.parse().ok()?
    };
    let mut total = int_val.checked_mul(scale)?;
    let mut remainder_scale = scale;
    let mut round_up = false;
    for c in frac_part.chars() {
        let d = u64::from(c as u8 - b'0');
        if remainder_scale == 0 || !remainder_scale.is_multiple_of(10) {
            // Beyond the unit's precision (or a binary scale): the first
            // dropped digit decides rounding, the rest cannot change it.
            if d >= 5 {
                round_up = true;
            }
            continue;
        }
        remainder_scale /= 10;
        total = total.checked_add(d.checked_mul(remainder_scale)?)?;
    }
    if round_up {
        total = total.checked_add(1)?;
    }
    Some(total)
}

// ─────────────────────────────────────────────────────────────────────────
// The SHA key (the stale-profile gate)
// ─────────────────────────────────────────────────────────────────────────

/// The outcome of keying a report against the target repo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyVerdict {
    /// The report's `meta.git.sha` equals the repo's HEAD.
    Matched { sha: String },
    /// MISMATCH — the report must be REFUSED (the stale-profile hazard).
    Mismatched {
        report_sha: String,
        repo_sha: String,
    },
    /// The report carries no git info (plain `hotpath` without
    /// `hotpath-cloud`). Cannot mismatch; proceeds with a loud disclosure.
    Unkeyed,
}

/// Key a parsed report against the repo SHA the healer is pointed at.
#[must_use]
pub fn key_verdict(report: &HotpathReport, repo_sha: &str) -> KeyVerdict {
    match report.meta.git.as_ref() {
        Some(git) => {
            let report_sha = git.sha.trim().to_lowercase();
            let repo_sha = repo_sha.trim().to_lowercase();
            if report_sha == repo_sha {
                KeyVerdict::Matched { sha: report_sha }
            } else {
                KeyVerdict::Mismatched {
                    report_sha,
                    repo_sha,
                }
            }
        }
        None => KeyVerdict::Unkeyed,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Hot-span derivation
// ─────────────────────────────────────────────────────────────────────────

/// One measured-hot span, ready for (T0.2) anchoring: the located entries
/// ranked by measured time share. Timing is RANKING-ONLY (B129); the alloc
/// axis is the deterministic verification axis.
#[derive(Debug, Clone, PartialEq)]
pub struct HotSpan {
    /// `source_root`-prefixed, `/`-separated, workspace-root-relative path
    /// as `file!()` captured it.
    pub file: String,
    /// 1-based line of the measured item.
    pub line: u32,
    /// The instrumented item's name (a function path for `#[measure]` fns).
    pub name: String,
    /// Calls observed (exact — counts are never sampled).
    pub calls: u64,
    /// Average duration per call, ns (None when the entry's avg was N/A —
    /// count-only mode under time sampling).
    pub avg_ns: Option<u64>,
    /// Measured time share, `calls × avg` — the RANKING quantity.
    pub time_share_ns: u64,
    /// Percent of measured time (as printed).
    pub percent_total: Option<f64>,
    /// Total allocated bytes at this site, when the report carries a
    /// `functions_alloc` entry for the same `(file, line, name)`.
    pub alloc_bytes: Option<u64>,
    /// Allocation calls at this site (the alloc section's `calls`).
    pub alloc_calls: Option<u64>,
}

/// Derive the ranked hot spans from a report: every LOCATED timing entry
/// (joined with its alloc twin by `(file, line, name)` when present),
/// sorted DESC by `time_share_ns` (`calls × avg`); entries without `avg`
/// sort last (count-only evidence still anchors, ranked below timed ones).
#[must_use]
pub fn hot_spans(report: &HotpathReport) -> Vec<HotSpan> {
    let source_root = report.meta.source_root.as_deref().unwrap_or("");
    let prefix = source_root.trim_start_matches("./").trim_end_matches('/');
    let empty: &[HotpathFunctionEntry] = &[];
    let timing = report
        .functions_timing
        .as_ref()
        .map_or(empty, |l| l.data.as_slice());
    let alloc = report
        .functions_alloc
        .as_ref()
        .map_or(empty, |l| l.data.as_slice());

    let mut out: Vec<HotSpan> = Vec::new();
    for entry in timing {
        let Some(loc) = entry.location.as_ref() else {
            continue; // cannot anchor without a location
        };
        let file = join_root(prefix, &loc.file);
        let avg_ns = entry.avg.as_deref().and_then(parse_duration_ns);
        let time_share_ns = avg_ns.map_or(0, |avg| avg.saturating_mul(entry.calls));
        // The alloc twin: same site, same name (a function timed and
        // alloc-profiled is the same instrumented item).
        let twin = alloc.iter().find(|a| {
            a.name == entry.name
                && a.location
                    .as_ref()
                    .is_some_and(|l| l.line == loc.line && join_root(prefix, &l.file) == file)
        });
        out.push(HotSpan {
            file,
            line: loc.line,
            name: entry.name.clone(),
            calls: entry.calls,
            avg_ns,
            time_share_ns,
            percent_total: entry.percent_total.as_deref().and_then(parse_percent),
            alloc_bytes: twin.and_then(|t| t.total.as_deref()).and_then(parse_bytes),
            alloc_calls: twin.map(|t| t.calls),
        });
    }
    // DESC by time share; untimed (count-only) entries keep file/line order.
    out.sort_by(|a, b| {
        b.time_share_ns
            .cmp(&a.time_share_ns)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
    });
    out
}

/// Prefix a workspace-relative location with `meta.source_root`, `/`-joined.
fn join_root(prefix: &str, file: &str) -> String {
    let file = file.replace('\\', "/");
    if prefix.is_empty() {
        file
    } else {
        format!("{prefix}/{file}")
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The fix-lane attribute guard (Issue 146 T0.3)
// ─────────────────────────────────────────────────────────────────────────

/// Byte ranges `(line_start, line_end_exclusive)` of every line carrying
/// hotpath's OWN instrumentation vocabulary — the attributes and macro
/// invocations the profiler injected. The fix lane declines any edit that
/// would remove or rewrite bytes on one of these lines (T0.3: eating the
/// instrumentation silently breaks the user's next profile — the exact
/// oracle this lane consumes).
///
/// Matched forms: `#[hotpath::measure]` / `#[hotpath::measure_block]` /
/// any `#[hotpath::…]` attribute, the imported spellings `#[measure]` /
/// `#[measure_block]` / `#[measure_with]`, the block macro
/// `hotpath::measure_block!`, and the finish/init calls
/// `hotpath::finish` / `hotpath::init`. Bare `#[measure]` risks a false
/// decline against another crate's same-named macro — deliberately
/// conservative: a false DECLINE is loud + overridable (apply manually),
/// a missed decline silently breaks instrumentation (the cries-wolf
/// asymmetry).
#[must_use]
pub fn attribute_line_ranges(src: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut off = 0usize;
    for line in src.lines() {
        let start = off;
        let end = off + line.len();
        off = end + 1; // the consumed '\n' (last line without one is fine)
        if line_contains_hotpath_attr(line) {
            out.push((start, end));
        }
    }
    out.shrink_to_fit();
    out
}

/// One line's test: contains a hotpath instrumentation token. Any
/// `hotpath::` occurrence qualifies (attributes `#[hotpath::measure]`, the
/// block macro `hotpath::measure_block!`, `hotpath::finish`/`init` calls,
/// AND the `use hotpath::measure;` import — removing the import breaks the
/// attributes just as surely). The imported bare spellings are anchored as
/// attribute tokens so an unrelated `measure` identifier alone never
/// matches; a false DECLINE is loud + overridable, a missed one silently
/// breaks instrumentation (the cries-wolf asymmetry).
fn line_contains_hotpath_attr(line: &str) -> bool {
    let t = line.trim();
    if t.contains("hotpath::") {
        return true;
    }
    for attr in ["#[measure]", "#[measure_block]", "#[measure_with]"] {
        if t.starts_with(attr) || t.contains(&format!(" {attr}")) {
            return true;
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────
// Tests (the fixture is OUR contract — the real-hotpath compat gate is
// T0.4's e2e run, which pins the wire end-to-end)
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A report shaped like hotpath's real output: string numbers with
    /// units, percentiles flattened (ignored by the subset), an unkeyed
    /// meta variant toggled by the `keyed` flag, and a `source_root`.
    fn fixture_report(keyed: bool, source_root: Option<&str>) -> String {
        let git = if keyed {
            r#""git": {"sha": "abc123def456", "ref": "refs/heads/develop"},"#
        } else {
            ""
        };
        let root = match source_root {
            Some(r) => format!(r#""source_root": "{r}","#),
            None => String::new(),
        };
        let meta = format!(
            r#"{{"rustc": "1.98.0", "os": "macos-aarch64", "created_at": "2026-10-04T00:00:00Z", {root} {git} "benchmark": null}}"#
        );
        format!(
            r#"{{
              "type": "hotpath-report",
              "version": "0.7.0",
              "meta": {meta},
              "functions_timing": {{
                "profiling_mode": "functions",
                "time_elapsed": "10.0 s",
                "total_elapsed_ns": 10000000000,
                "description": "…",
                "caller_name": "",
                "percentiles": [0.5, 0.9],
                "total_count": 3,
                "included_count": 3,
                "data": [
                  {{
                    "id": 1,
                    "name": "app::engine::decode_row",
                    "calls": 1000,
                    "sampled_calls": 1000,
                    "avg": "1.5 µs",
                    "total": "1.5 ms",
                    "percent_total": "45.0%",
                    "p50": "1.4 µs", "p90": "2.0 µs",
                    "location": {{"file": "src/engine/decode.rs", "line": 42, "column": 5}}
                  }},
                  {{
                    "id": 2,
                    "name": "app::engine::unlocated_hot",
                    "calls": 100000,
                    "sampled_calls": 50000,
                    "avg": "20 ns",
                    "total": "2.0 ms",
                    "percent_total": "40.0%"
                  }},
                  {{
                    "id": 3,
                    "name": "app::engine::serialize",
                    "calls": 10,
                    "sampled_calls": 10,
                    "avg": "N/A",
                    "total": "N/A",
                    "percent_total": "N/A",
                    "location": {{"file": "src/engine/ser.rs", "line": 7, "column": 1}}
                  }}
                ]
              }},
              "functions_alloc": {{
                "profiling_mode": "functions",
                "time_elapsed": "10.0 s",
                "total_elapsed_ns": 10000000000,
                "total_allocated": "12.5 MB",
                "description": "…",
                "caller_name": "",
                "percentiles": [0.5],
                "total_count": 1,
                "included_count": 1,
                "data": [
                  {{
                    "id": 1,
                    "name": "app::engine::decode_row",
                    "calls": 4000,
                    "sampled_calls": 4000,
                    "avg": "256 B",
                    "total": "1.0 MB",
                    "percent_total": "8.0%",
                    "location": {{"file": "src/engine/decode.rs", "line": 42, "column": 5}}
                  }}
                ]
              }},
              "channels": null, "futures": null, "mutexes": null
            }}"#
        )
    }

    #[test]
    fn parses_the_minimal_subset_and_ignores_unknown_sections() {
        let report: HotpathReport =
            serde_json::from_str(&fixture_report(true, None)).expect("fixture parses");
        assert_eq!(report.meta.git.as_ref().unwrap().sha, "abc123def456");
        assert_eq!(
            report.functions_timing.as_ref().unwrap().data.len(),
            3,
            "unknown fields (percentile keys flatten into entries) must not break the parse"
        );
    }

    #[test]
    fn key_verdict_refuses_mismatch_and_passes_match() {
        let report: HotpathReport = serde_json::from_str(&fixture_report(true, None)).unwrap();
        assert_eq!(
            key_verdict(&report, "ABC123DEF456"),
            KeyVerdict::Matched {
                sha: "abc123def456".into()
            },
            "case-insensitive SHA comparison"
        );
        match key_verdict(&report, "deadbeef") {
            KeyVerdict::Mismatched {
                report_sha,
                repo_sha,
            } => {
                assert_eq!(report_sha, "abc123def456");
                assert_eq!(repo_sha, "deadbeef");
            }
            other => panic!("expected Mismatched, got {other:?}"),
        }
    }

    #[test]
    fn unkeyed_reports_cannot_mismatch() {
        let report: HotpathReport = serde_json::from_str(&fixture_report(false, None)).unwrap();
        assert_eq!(key_verdict(&report, "anything"), KeyVerdict::Unkeyed);
    }

    #[test]
    fn hot_spans_skip_unlocated_join_alloc_and_rank_by_time_share() {
        let report: HotpathReport = serde_json::from_str(&fixture_report(true, None)).unwrap();
        let spans = hot_spans(&report);
        // The unlocated entry is dropped; the two located ones remain.
        assert_eq!(spans.len(), 2);
        // decode_row: 1000 × 1.5µs = 1.5ms share; serialize: N/A avg → 0.
        assert_eq!(spans[0].name, "app::engine::decode_row");
        assert_eq!(spans[0].time_share_ns, 1_500_000);
        assert_eq!(spans[0].avg_ns, Some(1_500));
        assert_eq!(spans[0].percent_total, Some(45.0));
        // The alloc twin joined by (file, line, name).
        assert_eq!(spans[0].alloc_bytes, Some(1_000_000));
        assert_eq!(spans[0].alloc_calls, Some(4_000));
        // Count-only entry sorts last, keeps its location.
        assert_eq!(spans[1].name, "app::engine::serialize");
        assert_eq!(spans[1].avg_ns, None);
        assert_eq!(spans[1].time_share_ns, 0);
        assert_eq!(spans[1].line, 7);
    }

    #[test]
    fn source_root_prefixes_locations() {
        let report: HotpathReport =
            serde_json::from_str(&fixture_report(true, Some("crates/app"))).unwrap();
        let spans = hot_spans(&report);
        assert!(
            spans[0].file.starts_with("crates/app/"),
            "{}",
            spans[0].file
        );
        assert_eq!(spans[0].file, "crates/app/src/engine/decode.rs");
    }

    #[test]
    fn duration_and_byte_parsing_round_trip_hotpath_forms() {
        assert_eq!(parse_duration_ns("8 ns"), Some(8));
        assert_eq!(parse_duration_ns("1.5 µs"), Some(1_500));
        assert_eq!(parse_duration_ns("1.5 us"), Some(1_500));
        assert_eq!(parse_duration_ns("2.3 ms"), Some(2_300_000));
        assert_eq!(parse_duration_ns("1.00 s"), Some(1_000_000_000));
        assert_eq!(parse_duration_ns("N/A"), None);
        assert_eq!(parse_duration_ns("1.5"), None, "unitless is not a duration");
        assert_eq!(parse_bytes("12 B"), Some(12));
        assert_eq!(parse_bytes("1.0 KB"), Some(1_000));
        assert_eq!(parse_bytes("1.0 MB"), Some(1_000_000));
        assert_eq!(parse_bytes("1.0 MiB"), Some(1 << 20));
        assert_eq!(parse_bytes("N/A"), None);
        assert_eq!(parse_percent("45.0%"), Some(45.0));
        assert_eq!(parse_percent("N/A"), None);
    }

    #[test]
    fn saturating_time_share_on_absurd_counts() {
        // calls × avg must not overflow u64 on adversarial reports.
        let entry = HotpathFunctionEntry {
            name: "x".into(),
            calls: u64::MAX,
            avg: Some("1.00 s".into()),
            total: None,
            percent_total: None,
            location: Some(HotpathLocation {
                file: "src/x.rs".into(),
                line: 1,
            }),
        };
        let report = HotpathReport {
            meta: HotpathMeta {
                git: None,
                source_root: None,
            },
            functions_timing: Some(HotpathFunctionsList { data: vec![entry] }),
            functions_alloc: None,
        };
        let spans = hot_spans(&report);
        assert_eq!(spans[0].time_share_ns, u64::MAX);
    }

    // ── T0.3: the instrumentation-attribute guard ──────────────────────

    #[test]
    fn attribute_lines_cover_every_hotpath_spelling() {
        let src = "use hotpath::measure;\n\
\n\
#[hotpath::measure]\n\
fn timed() -> u32 { 1 }\n\
\n\
#[measure]\n\
fn imported() -> u32 { 2 }\n\
\n\
fn body() -> u32 {\n\
    hotpath::measure_block!({ 3 })\n\
}\n\
\n\
fn plain() -> u32 {
\
    let measure = 4; // an unrelated identifier — never a hit\n\
    measure\n\
}\n";
        let ranges = attribute_line_ranges(src);
        // Lines (1-based) carrying hotpath vocabulary: 1 (use), 3 (qualified
        // attr), 6 (bare attr), 10 (block macro). The `measure` local (13)
        // and everything else must NOT hit.
        let hit_lines: Vec<usize> = ranges
            .iter()
            .map(|&(s, _)| src[..s].matches('\n').count() + 1)
            .collect();
        assert_eq!(hit_lines, vec![1, 3, 6, 10]);
    }

    #[test]
    fn attribute_lines_empty_on_clean_source() {
        let src = "fn a() -> u32 { 1 }\nfn b() -> u32 { measure(2) }\n";
        assert_eq!(attribute_line_ranges(src), Vec::<(usize, usize)>::new());
    }
}

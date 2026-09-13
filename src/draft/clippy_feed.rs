//! The clippy-fed oracle anchor feed (the 2026-09-13 retrieval-miss intake).
//!
//! `--suggest` ranks spans by corpus similarity alone; the intake's miss —
//! a real `collapsible_if` finding at riir-dapps `tests/art_chaos.rs:320`
//! (the rule exists, the proposer shape matches, but the chunk's top-k
//! never surfaced it while 9 heuristic candidates on unflagged spans did)
//! — is the top-N gate eating a true positive. When a KNOWN finding set
//! exists (the caller ran real clippy), those diagnostics are ground
//! truth and must be force-included AHEAD of similarity ranking, never
//! subject to it. The mirror datum — heuristic candidates where clippy is
//! silent — is the false-positive side of the same coin; spans a feed
//! covers without a diagnostic get the `heuristic` marker (see
//! [`LatentMatcher::suggest_file_anchored`]).
//!
//! The feed is clippy's own `--message-format=json` stream (JSON lines).
//! The healer never spawns clippy itself: toolchain, flags and target
//! selection stay the caller's, and the heal run stays local + fast.
//! Only what anchoring needs is parsed: the primary-span file + line and
//! the `clippy::*` code. Non-`clippy::` codes are ignored — rustc lints
//! have no corpus rules, so feeding them would only manufacture
//! `anchored_missing` noise.
//!
//! The `--fix-compile` driver needs no feed of this kind: its candidate
//! spans ARE diagnostic locations by construction (driver-based, not
//! retrieval). This module serves the retrieval lane only.
//!
//! Parse idiom: per-line [`serde_json::Value`], mirroring
//! `clippy_oracle::parse_verdict` (the repo's established cargo-JSON
//! reader) — tolerant of format drift, strict about the fields that
//! matter.

/// One real clippy finding, reduced to what anchoring needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClippyDiagnostic {
    /// Source file as clippy printed it (cargo convention:
    /// workspace-relative, `/`-separated; normalized from `\` on load).
    pub file: String,
    /// 1-based line of the diagnostic's primary span.
    pub line: usize,
    /// The lint name without the `clippy::` prefix (`collapsible_if`).
    pub lint: String,
}

/// Parse a `cargo clippy --message-format=json` stream into diagnostics.
///
/// One JSON object per line; non-JSON lines and non-`compiler-message`
/// rows are skipped (cargo interleaves build-finished/artifact rows).
/// Warnings and errors both count — `-D` runs report lints as errors.
/// Deduplicated per (file, line, lint): clippy can repeat a diagnostic
/// across compilation stages of the same package.
#[must_use]
pub fn parse_clippy_feed(json_output: &str) -> Vec<ClippyDiagnostic> {
    let mut out: Vec<ClippyDiagnostic> = Vec::new();
    for raw in json_output.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
            continue;
        };
        if v.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = v.get("message") else {
            continue;
        };
        let level = message.get("level").and_then(serde_json::Value::as_str);
        if !matches!(level, Some("warning" | "error")) {
            continue;
        }
        let Some(code) = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(lint) = code.strip_prefix("clippy::") else {
            continue;
        };
        let Some((file_name, file_line)) = primary_span_location(message) else {
            continue;
        };
        let diag = ClippyDiagnostic {
            file: file_name.replace('\\', "/"),
            line: file_line,
            lint: lint.to_string(),
        };
        if !out.contains(&diag) {
            out.push(diag);
        }
    }
    out.shrink_to_fit();
    out
}

/// The primary span's (`file_name`, `line_start`): the first
/// `is_primary: true` span, else the first span (the [`clippy_oracle`]
/// `primary_span_file` rule — diagnostics without an explicit primary
/// still anchor at their only span).
fn primary_span_location(message: &serde_json::Value) -> Option<(&str, usize)> {
    let spans = message.get("spans")?.as_array()?;
    let primary = spans
        .iter()
        .find(|s| s.get("is_primary").and_then(serde_json::Value::as_bool) == Some(true));
    let span = primary.or_else(|| spans.first())?;
    let file = span.get("file_name").and_then(serde_json::Value::as_str)?;
    let line = span
        .get("line_start")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())?;
    Some((file, line))
}

/// Do a feed file name and a scanned display path refer to the same file?
///
/// clippy prints workspace-relative paths; the scan paths are whatever the
/// caller typed. Neither is canonical, so match on the path TAIL with a
/// component boundary: `src/lib.rs` matches `crate/src/lib.rs` and
/// `crate\src\lib.rs`, but `a/mod.rs` does NOT match `b/mod.rs` (a bare
/// `ends_with` would happily equate same-named tails from different
/// directories). Both directions are tried — the feed path can be the
/// deeper or the shallower of the two.
#[must_use]
pub fn paths_ref_match(feed_file: &str, display_path: &str) -> bool {
    let feed = feed_file.replace('\\', "/");
    let disp = display_path.replace('\\', "/");
    if feed == disp {
        return true;
    }
    feed.ends_with(&format!("/{disp}")) || disp.ends_with(&format!("/{feed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(level: &str, code: Option<&str>, file: &str, line: u64, primary: bool) -> String {
        let code_json = code.map_or("null".to_string(), |c| {
            format!(r#"{{"code": "{c}"}}"#)
        });
        format!(
            r#"{{"reason": "compiler-message", "message": {{"level": "{level}", "code": {code_json}, "spans": [{{"file_name": "{file}", "line_start": {line}, "is_primary": {primary}}}]}}}}"#
        )
    }

    #[test]
    fn parses_a_clippy_warning_with_primary_span() {
        let stream = format!(
            "{}\n{}\n",
            row(
                "warning",
                Some("clippy::collapsible_if"),
                "tests/art_chaos.rs",
                320,
                true
            ),
            r#"{"reason": "build-finished", "success": true}"#
        );
        let got = parse_clippy_feed(&stream);
        assert_eq!(
            got,
            vec![ClippyDiagnostic {
                file: "tests/art_chaos.rs".to_string(),
                line: 320,
                lint: "collapsible_if".to_string(),
            }]
        );
    }

    #[test]
    fn prefers_the_primary_span_over_the_first() {
        let stream = format!(
            "{}\n",
            r#"{"reason": "compiler-message", "message": {"level": "warning", "code": {"code": "clippy::collapsible_if"}, "spans": [{"file_name": "src/other.rs", "line_start": 1, "is_primary": false}, {"file_name": "src/lib.rs", "line_start": 42, "is_primary": true}]}}"#
        );
        let got = parse_clippy_feed(&stream);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].file.as_str(), got[0].line), ("src/lib.rs", 42));
    }

    #[test]
    fn ignores_rustc_codes_notes_and_malformed_lines() {
        let stream = format!(
            "{}\n{}\n{}\nnot json\n",
            row("warning", Some("unused_variables"), "src/a.rs", 3, true),
            row("note", Some("clippy::collapsible_if"), "src/a.rs", 4, true),
            row("error", Some("clippy::needless_return"), "src/b.rs", 5, true),
        );
        let got = parse_clippy_feed(&stream);
        // -W runs report lints as "warning"; -D runs as "error" — both count.
        assert_eq!(
            got,
            vec![ClippyDiagnostic {
                file: "src/b.rs".to_string(),
                line: 5,
                lint: "needless_return".to_string(),
            }]
        );
    }

    #[test]
    fn dedups_repeated_diagnostics() {
        let line = row(
            "warning",
            Some("clippy::collapsible_if"),
            "src/lib.rs",
            7,
            true,
        );
        let got = parse_clippy_feed(&format!("{line}\n{line}\n"));
        assert_eq!(got.len(), 1);
    }

    #[test]
    fn path_tails_match_on_component_boundaries_only() {
        assert!(paths_ref_match("src/lib.rs", "crate/src/lib.rs"));
        assert!(paths_ref_match("crate\\src\\lib.rs", "src/lib.rs"));
        assert!(paths_ref_match("src/lib.rs", "src/lib.rs"));
        assert!(!paths_ref_match("a/mod.rs", "b/mod.rs"));
        assert!(!paths_ref_match("lib.rs", "my_lib.rs"));
    }
}

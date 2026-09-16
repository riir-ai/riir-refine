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
//! Since Issue 113 (the auto-oracle, 2026-09-16) the healer SPAWNS clippy
//! itself when a scanning run targets a cargo project and no manual feed
//! was given — default features, loud status lines, `RIIR_HEAL_AUTO_ORACLE=0`
//! restores the old never-spawn posture (see [`crate::clippy_oracle`]).
//! Toolchain and target-dir discipline stay the caller's environment; a
//! manual `--clippy-json` still wins outright. Before that the oracle was
//! manual-only, and the owner's finding was sharp: a gain path that exists
//! only behind a flag nobody discovers is not a gain path — `cargo heal
//! --mine` must do every good for the user (the auto-login/auto-sync
//! precedent).
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
//!
//! ## The fix lane consumes suggestions (P21, 2026-09-14)
//!
//! A diagnostic's help children can carry a `MachineApplicable` suggestion
//! — the probe-measured shape for `unnecessary_map_or` (clippy 1.98) is a
//! TWO-PART suggestion inside ONE child: `map_or` → `is_some_and` plus a
//! deletion of `false, `, with the `(` gap kept. [`parse_clippy_feed`]
//! captures those parts as [`FeedSuggestion`], and [`oracle_edits`] merges
//! them into ONE atomic edit (gap text preserved verbatim — rustc multipart
//! suggestions keep gap bytes unchanged by construction) for the `--fix`
//! lane. This closes the P15 residue — receivers `receiver_kind` cannot
//! classify parse-only (`.min()` chains, `File::open`, `.as_ref()`), where
//! our type-blind proposer refuses (E0599-unsafe) but clippy's suggestion
//! is type-aware and exact. Lints outside the fix lane's set are never
//! consumed — the healer stays a corpus-rule engine, not a generic
//! `cargo clippy --fix` re-run. The ONE lint whose suggestion clippy itself
//! gets wrong on a live shape is guarded: `useless_format` on an
//! escape-carrying literal (P23) — see the refusal list on [`oracle_edits`].

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
    /// The first help child's `MachineApplicable` suggestion, when the
    /// diagnostic carries one (P21). `None` when there is no suggestion,
    /// the applicability is not `MachineApplicable`, the suggestion spans
    /// multiple children (the move/insert class), or a part lands outside
    /// the primary file.
    pub suggestion: Option<FeedSuggestion>,
}

/// One consumed machine-applicable suggestion: the replacement spans of a
/// SINGLE help child, as byte-offset parts into the file clippy ran on.
/// Parts are sorted by start and non-overlapping; the merge into one
/// atomic edit (gap text preserved verbatim) happens in [`oracle_edits`],
/// where the current source is available to fill the gaps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedSuggestion {
    /// Replacement parts `(byte_start, byte_end, replacement)`, sorted by
    /// `byte_start`, non-overlapping. An EMPTY replacement string is a real
    /// part (a deletion) — only a JSON `null` is "no part".
    pub parts: Vec<(usize, usize, String)>,
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
        let suggestion = suggestion_from_message(message, file_name);
        let diag = ClippyDiagnostic {
            file: file_name.replace('\\', "/"),
            line: file_line,
            lint: lint.to_string(),
            suggestion,
        };
        // Dedup on (file, line, lint) — clippy can repeat a diagnostic
        // across compilation stages of the same package. A repeat
        // UPGRADES an earlier occurrence that carried no suggestion
        // (defensive: stages emit content-identical children, but a
        // dropped-children stage must not strand a suggestion-less row).
        if let Some(existing) = out
            .iter_mut()
            .find(|d| d.file == diag.file && d.line == diag.line && d.lint == diag.lint)
        {
            if existing.suggestion.is_none() {
                existing.suggestion = diag.suggestion;
            }
        } else {
            out.push(diag);
        }
    }
    out.shrink_to_fit();
    out
}

/// Extract the suggestion a diagnostic's help children carry, when it is
/// consumable: the replacement spans of the FIRST child that has any,
/// every part `MachineApplicable`, every part in the primary file, byte
/// offsets present and ordered. Anything else refuses the WHOLE
/// suggestion (`None`) — a partially-consumed suggestion is a corrupted
/// edit, so the refusals are all-or-nothing per diagnostic.
fn suggestion_from_message(
    message: &serde_json::Value,
    primary_file: &str,
) -> Option<FeedSuggestion> {
    let children = message.get("children")?.as_array()?;
    let mut chosen: Option<Vec<(usize, usize, String)>> = None;
    for child in children {
        let mut parts: Vec<(usize, usize, String)> = Vec::new();
        if let Some(spans) = child.get("spans").and_then(serde_json::Value::as_array) {
            for sp in spans {
                // JSON `null` = a labeling span, not an edit part. `""` is
                // a REAL part (a deletion) and must survive.
                let repl = sp.get("suggested_replacement")?;
                if repl.is_null() {
                    continue;
                }
                let repl = repl.as_str()?;
                if sp.get("suggestion_applicability").and_then(serde_json::Value::as_str)
                    != Some("MachineApplicable")
                {
                    // The suggestion grades itself unsure — refuse the
                    // whole diagnostic's suggestion, never half of it.
                    return None;
                }
                let file = sp.get("file_name").and_then(serde_json::Value::as_str)?;
                if file != primary_file {
                    return None; // cross-file part: offsets unusable here
                }
                let start = sp.get("byte_start").and_then(serde_json::Value::as_u64)?;
                let end = sp.get("byte_end").and_then(serde_json::Value::as_u64)?;
                let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
                    return None;
                };
                if start > end {
                    return None;
                }
                parts.push((start, end, repl.to_string()));
            }
        }
        if parts.is_empty() {
            continue;
        }
        if chosen.is_some() {
            // Multiple suggestion-bearing children (the remove-here +
            // insert-there move class) — out of the residue this lane
            // exists for; refuse rather than apply half a move.
            return None;
        }
        chosen = Some(parts);
    }
    let mut parts = chosen?;
    parts.sort_by_key(|(start, _, _)| *start);
    if parts.windows(2).any(|w| w[0].1 > w[1].0) {
        return None; // overlapping parts — not a well-formed suggestion
    }
    Some(FeedSuggestion { parts })
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

/// Do two edits' byte ranges overlap? Zero-width edits (insertions) overlap
/// only strictly inside the other's range — an insertion AT a boundary is
/// compatible.
#[must_use]
pub fn spans_overlap(a: &crate::edit_emitter::SourceEdit, b: &crate::edit_emitter::SourceEdit) -> bool {
    a.start < b.end && b.start < a.end
}

/// The `--fix`-lane result of feeding one file's anchors through the
/// suggestion consumer (P21).
#[derive(Debug, Default)]
pub struct OracleLane {
    /// Merged, validated, non-overlapping oracle edits (rule id = the lint
    /// name — the corpus rule is in the lane's set by construction).
    pub edits: Vec<crate::edit_emitter::RuleEdit>,
    /// Candidates that matched this file but were dropped (stale offsets,
    /// misaligned boundaries, or the whole-file parse gate). Non-zero with
    /// an empty/short `edits` usually means the feed predates the current
    /// file state — the caller should surface that loudly.
    pub dropped: usize,
}

/// Compute the oracle-anchored edits for ONE scanned file (P21).
///
/// For each feed diagnostic that matches this file ([`paths_ref_match`]),
/// whose lint is in the lane's set, and that carries a suggestion: merge
/// the parts into ONE atomic [`SourceEdit`] — parts joined left-to-right
/// with the ORIGINAL gap text preserved verbatim (rustc multipart
/// suggestions keep gap bytes unchanged by construction, so the merge is
/// semantics-preserving for every well-formed suggestion) — then pass a
/// whole-file syntax gate: the oracle edits alone, spliced into a copy,
/// must still parse. A gate failure drops ALL of the file's oracle edits
/// (all-or-nothing — a half-applied suggestion set is a corrupted file).
///
/// Refusals (each drops an edit, never produces a wrong one):
/// - lint outside `lane_lints` — the corpus-gap class stays the suggest
///   lane's `[no corpus rule]` signal; the healer never becomes a generic
///   `cargo clippy --fix` re-run;
/// - the P23 `useless_format` escape guard — a brace anywhere in the
///   covered call text means the literal carries `{{`/`}}` escapes (a lone
///   brace would not compile), and clippy's own `.to_string()` suggestion
///   reuses the ESCAPED literal verbatim, freezing the doubled form
///   (`format!("{{x}}")` produces `{x}`; `"{{x}}".to_string()` produces
///   `{{x}}`) — the exact class the intake recorded after it corrupted a
///   JSON fixture. Mirrors the proposer's v1 brace-abort; the manual fix
///   unescapes, never consumes verbatim;
/// - merged range out of bounds / off a char boundary (a stale feed);
/// - oracle-vs-oracle overlap (leftmost wins);
/// - the parse-gate refusal (drops the file's whole oracle set).
#[must_use]
pub fn oracle_edits(
    feed: &[ClippyDiagnostic],
    display_path: &str,
    src: &str,
    lane_lints: &[&str],
) -> OracleLane {
    let mut candidates: Vec<(usize, usize, String, String)> = Vec::new();
    let mut dropped = 0usize;
    for d in feed {
        if !lane_lints.contains(&d.lint.as_str()) {
            continue;
        }
        if !paths_ref_match(&d.file, display_path) {
            continue;
        }
        let Some(sug) = &d.suggestion else {
            continue;
        };
        match merged_suggestion_edit(sug, src) {
            Some(edit) => {
                // P23: refuse `useless_format` on escape-carrying literals.
                // Any brace in a compiling no-args format! literal is an
                // escape (`{{`/`}}` — a lone brace would not compile), and
                // clippy's suggestion reuses the escaped literal verbatim,
                // so consuming it freezes the doubled form — a behavior
                // change on exactly the class the intake recorded (a JSON
                // fixture corrupted 1 row -> 0). The proposer's own v1
                // guard refuses the same shape at generation time; this is
                // the oracle-door mirror. Conservative by construction:
                // drops the edit, never produces a wrong one.
                if d.lint == "useless_format"
                    && src
                        .get(edit.start..edit.end)
                        .is_some_and(|cov| cov.contains(['{', '}']))
                {
                    dropped += 1;
                    continue;
                }
                candidates.push((edit.start, edit.end, edit.replacement, d.lint.clone()));
            }
            None => dropped += 1, // stale offsets / misaligned boundaries
        }
    }
    // Oracle-vs-oracle overlap: leftmost wins (sorted by start; a wide
    // early span covers later ones via the running end).
    candidates.sort_by_key(|(start, _, _, _)| *start);
    let mut covered_until = 0usize;
    let mut kept: Vec<(usize, usize, String, String)> = Vec::new();
    for cand in candidates {
        let (start, end, _, _) = &cand;
        if *start < covered_until {
            dropped += 1;
            continue;
        }
        covered_until = (*end).max(covered_until);
        kept.push(cand);
    }
    if kept.is_empty() {
        return OracleLane { edits: Vec::new(), dropped };
    }
    // The whole-file syntax gate: the oracle edits ALONE must leave the
    // file parseable. All-or-nothing on failure.
    let edits: Vec<crate::edit_emitter::SourceEdit> = kept
        .iter()
        .map(|(start, end, repl, _)| crate::edit_emitter::SourceEdit {
            start: *start,
            end: *end,
            replacement: repl.clone(),
        })
        .collect();
    let patched = splice_sorted(src, &edits);
    if syn::parse_file(&patched).is_err() {
        return OracleLane {
            edits: Vec::new(),
            dropped: dropped + kept.len(),
        };
    }
    let edits = edits
        .into_iter()
        .zip(kept)
        .map(|(edit, (_, _, _, lint))| crate::edit_emitter::RuleEdit {
            rule_id: lint,
            edit,
        })
        .collect();
    OracleLane { edits, dropped }
}

/// Merge one suggestion's parts into a single atomic edit against `src`.
/// Gap text between parts is copied verbatim; any misaligned or
/// out-of-bounds part refuses the whole merge.
fn merged_suggestion_edit(
    sug: &FeedSuggestion,
    src: &str,
) -> Option<crate::edit_emitter::SourceEdit> {
    let &(first_start, _, _) = sug.parts.first()?;
    let last_end = sug.parts.last()?.1;
    if last_end > src.len() || !src.is_char_boundary(first_start) || !src.is_char_boundary(last_end)
    {
        return None;
    }
    let mut replacement = String::new();
    let mut prev_end = first_start;
    for &(start, end, ref repl) in &sug.parts {
        // `get` fails on misaligned or out-of-bounds gaps — refuse.
        replacement.push_str(src.get(prev_end..start)?);
        replacement.push_str(repl);
        prev_end = end;
    }
    Some(crate::edit_emitter::SourceEdit {
        start: first_start,
        end: last_end,
        replacement,
    })
}

/// Splice SORTED, non-overlapping, boundary-validated edits into a copy —
/// the parse-gate's scratch application (no selection semantics of its
/// own; callers validate before calling).
fn splice_sorted(src: &str, edits: &[crate::edit_emitter::SourceEdit]) -> String {
    let mut out = String::with_capacity(src.len());
    let mut pos = 0usize;
    for e in edits {
        if let Some(seg) = src.get(pos..e.start) {
            out.push_str(seg);
        }
        out.push_str(&e.replacement);
        pos = e.end;
    }
    if let Some(seg) = src.get(pos..) {
        out.push_str(seg);
    }
    out
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
                suggestion: None,
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
                suggestion: None,
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

    // ── P21: suggestion capture (the probe-measured shape — clippy 1.98,
    // `.raw/oracle_probe` 2026-09-14 — is a TWO-PART suggestion in ONE
    // child: method rename + `false, ` deletion, `(` gap preserved).

    /// One diagnostic row carrying a suggestion child.
    fn row_with_suggestion(file: &str, line: u64, lint_id: &str, parts: &[(u64, u64, &str, &str)]) -> String {
        let spans: Vec<String> = parts
            .iter()
            .map(|(bs, be, repl, applic)| {
                format!(
                    r#"{{"file_name": "{file}", "byte_start": {bs}, "byte_end": {be}, "suggested_replacement": "{repl}", "suggestion_applicability": "{applic}"}}"#
                )
            })
            .collect();
        format!(
            r#"{{"reason": "compiler-message", "message": {{"level": "warning", "code": {{"code": "clippy::{lint_id}"}}, "spans": [{{"file_name": "{file}", "line_start": {line}, "is_primary": true}}], "children": [{{"level": "help", "message": "for further information", "spans": []}}, {{"level": "help", "message": "use is_some_and instead", "spans": [{}]}}]}}}}"#,
            spans.join(", ")
        )
    }

    #[test]
    fn captures_multipart_machine_applicable_suggestion() {
        // Probe shape: `map_or` → `is_some_and` + delete `false, `.
        let stream = row_with_suggestion(
            "src/lib.rs",
            11,
            "unnecessary_map_or",
            &[
                (467, 473, "is_some_and", "MachineApplicable"),
                (474, 481, "", "MachineApplicable"),
            ],
        );
        let got = parse_clippy_feed(&stream);
        assert_eq!(got.len(), 1);
        let sug = got[0].suggestion.as_ref().expect("suggestion captured");
        assert_eq!(
            sug.parts,
            vec![
                (467usize, 473usize, "is_some_and".to_string()),
                (474, 481, String::new()),
            ]
        );
    }

    #[test]
    fn maybe_incorrect_suggestion_refused() {
        let stream = row_with_suggestion(
            "src/lib.rs",
            3,
            "some_lint",
            &[(10, 16, "rename", "MaybeIncorrect")],
        );
        let got = parse_clippy_feed(&stream);
        assert_eq!(got.len(), 1);
        assert!(got[0].suggestion.is_none(), "unsure grade must refuse");
    }

    #[test]
    fn multi_child_suggestion_refused() {
        // Two children each carrying a replacement span (the move class).
        // Plain data — single braces, no format! (the {{ }} escapes only
        // exist for format!, and clippy's useless_format `.to_string()` fix
        // would otherwise leave them doubled in the literal).
        let stream = r#"{"reason": "compiler-message", "message": {"level": "warning", "code": {"code": "clippy::some_lint"}, "spans": [{"file_name": "src/lib.rs", "line_start": 3, "is_primary": true}], "children": [{"level": "help", "spans": [{"file_name": "src/lib.rs", "byte_start": 10, "byte_end": 16, "suggested_replacement": "a", "suggestion_applicability": "MachineApplicable"}]}, {"level": "help", "spans": [{"file_name": "src/lib.rs", "byte_start": 50, "byte_end": 50, "suggested_replacement": "b", "suggestion_applicability": "MachineApplicable"}]}]}}"#.to_string();
        let got = parse_clippy_feed(&stream);
        assert_eq!(got.len(), 1);
        assert!(got[0].suggestion.is_none(), "multi-child must refuse");
    }

    #[test]
    fn cross_file_part_refused() {
        let stream = row_with_suggestion(
            "src/lib.rs",
            3,
            "some_lint",
            &[(10, 16, "x", "MachineApplicable")],
        );
        // Patch the PART's file to differ from the primary's.
        let stream = stream.replace(
            "\"file_name\": \"src/lib.rs\", \"byte_start\": 10",
            "\"file_name\": \"src/other.rs\", \"byte_start\": 10",
        );
        let got = parse_clippy_feed(&stream);
        assert_eq!(got.len(), 1);
        assert!(got[0].suggestion.is_none());
    }

    #[test]
    fn dedup_upgrades_a_suggestionless_first_occurrence() {
        let bare = row("warning", Some("clippy::unnecessary_map_or"), "src/lib.rs", 11, true);
        let with = row_with_suggestion(
            "src/lib.rs",
            11,
            "unnecessary_map_or",
            &[(20, 26, "is_some_and", "MachineApplicable")],
        );
        let got = parse_clippy_feed(&format!("{bare}\n{with}\n"));
        assert_eq!(got.len(), 1, "same (file,line,lint) dedups");
        assert!(got[0].suggestion.is_some(), "the repeat upgrades the row");
    }

    // ── P21: the oracle edit consumer.

    /// A src whose `map_or(false, ` site sits at a KNOWN byte offset.
    fn probe_src() -> String {
        "fn f(items: &[u32]) -> bool {\n    items.iter().min().map_or(false, |x| *x > 1)\n}\n".to_string()
    }

    fn mapor_feed(src: &str) -> Vec<ClippyDiagnostic> {
        let off = src.find("map_or").expect("fixture has the site");
        vec![ClippyDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 2,
            lint: "unnecessary_map_or".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![
                    (off, off + 6, "is_some_and".to_string()),
                    // `(false, ` after the method name: `(` gap + 7-byte delete
                    (off + 7, off + 14, String::new()),
                ],
            }),
        }]
    }

    /// Issue 120: `doc_markdown` suggestions are single-part
    /// `MachineApplicable` backtick wraps — once the lint is in the lane's
    /// corpus set, the oracle consumes clippy's own span verbatim (the
    /// e2e dogfood at landing ran the real binary against the proposer's
    /// own module docs; this unit test pins the lane-set gate itself).
    #[test]
    fn oracle_edits_consumes_doc_markdown_backtick_wrap() {
        let src = "/// Runs the shader_perf stage over a HashMap.\nfn stage() {}\n";
        let off = src.find("shader_perf").unwrap();
        let feed = vec![ClippyDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 1,
            lint: "doc_markdown".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![(off, off + "shader_perf".len(), "`shader_perf`".to_string())],
            }),
        }];
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["doc_markdown"]);
        assert_eq!(lane.dropped, 0);
        assert_eq!(lane.edits.len(), 1);
        assert_eq!(lane.edits[0].rule_id, "doc_markdown");
        assert_eq!(lane.edits[0].edit.replacement, "`shader_perf`");
        let patched = splice_sorted(src, std::slice::from_ref(&lane.edits[0].edit));
        assert!(patched.contains("Runs the `shader_perf` stage"), "{patched}");
        // The lane-set gate is the point: the SAME feed is refused when
        // the lint is not in the lane (the corpus-gap signal).
        let refused = oracle_edits(&feed, "src/lib.rs", src, &["collapsible_if"]);
        assert!(refused.edits.is_empty());
    }

    #[test]
    fn oracle_edits_merge_parts_with_gap_text_preserved() {
        let src = probe_src();
        let off = src.find("map_or").unwrap();
        let lane = oracle_edits(&mapor_feed(&src), "src/lib.rs", &src, &["unnecessary_map_or"]);
        assert_eq!(lane.dropped, 0);
        assert_eq!(lane.edits.len(), 1);
        let edit = &lane.edits[0].edit;
        assert_eq!(edit.start, off);
        assert_eq!(edit.end, off + 14);
        assert_eq!(edit.replacement, "is_some_and(");
        // The full splice produces the probe's verified terminal form.
        let patched = splice_sorted(&src, std::slice::from_ref(edit));
        assert!(
            patched.contains("items.iter().min().is_some_and(|x| *x > 1)"),
            "terminal form wrong: {patched}"
        );
        syn::parse_file(&patched).expect("parses");
    }

    #[test]
    fn oracle_edits_refuse_lints_outside_the_lane() {
        let src = probe_src();
        let lane = oracle_edits(&mapor_feed(&src), "src/lib.rs", &src, &["collapsible_if"]);
        assert!(lane.edits.is_empty());
        assert_eq!(lane.dropped, 0, "out-of-lane is not a drop, it is a skip");
    }

    #[test]
    fn oracle_edits_refuse_non_matching_files() {
        let src = probe_src();
        let lane = oracle_edits(&mapor_feed(&src), "other/mod.rs", &src, &["unnecessary_map_or"]);
        assert!(lane.edits.is_empty());
    }

    #[test]
    fn oracle_edits_drop_stale_offsets() {
        let src = probe_src();
        let mut feed = mapor_feed(&src);
        // Offsets far past EOF: the feed predates a truncation.
        if let Some(FeedSuggestion { parts }) = feed[0].suggestion.as_mut() {
            for (s, e, _) in parts.iter_mut() {
                *s += 10_000;
                *e += 10_000;
            }
        }
        let lane = oracle_edits(&feed, "src/lib.rs", &src, &["unnecessary_map_or"]);
        assert!(lane.edits.is_empty());
        assert_eq!(lane.dropped, 1);
    }

    #[test]
    fn oracle_edits_parse_gate_drops_the_whole_file_set() {
        let src = probe_src();
        let mut feed = mapor_feed(&src);
        // A replacement that yields an unparseable file.
        if let Some(FeedSuggestion { parts }) = feed[0].suggestion.as_mut() {
            parts[0].2 = "1is_some_and".to_string();
        }
        let lane = oracle_edits(&feed, "src/lib.rs", &src, &["unnecessary_map_or"]);
        assert!(lane.edits.is_empty());
        assert_eq!(lane.dropped, 1, "gate refusal counts the candidate");
    }

    #[test]
    fn oracle_edits_leftmost_wins_on_self_overlap() {
        let src = probe_src();
        let off = src.find("map_or").unwrap();
        let mut feed = mapor_feed(&src);
        // A second diagnostic whose suggestion overlaps the first's span.
        feed.push(ClippyDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 2,
            lint: "unnecessary_map_or".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![(off + 2, off + 20, "zzz".to_string())],
            }),
        });
        let lane = oracle_edits(&feed, "src/lib.rs", &src, &["unnecessary_map_or"]);
        assert_eq!(lane.edits.len(), 1, "leftmost wins");
        assert_eq!(lane.edits[0].edit.start, off);
        assert_eq!(lane.dropped, 1);
    }

    // ── P23: the `useless_format` escape guard — clippy's own suggestion
    // is WRONG on escape-carrying literals (it reuses the escaped literal
    // verbatim in a `.to_string()` position, freezing the doubled braces
    // the macro would have processed). The oracle lane must refuse.

    #[test]
    fn oracle_edits_refuse_useless_format_brace_escapes() {
        // `format!("{{x}}")` produces "{x}" — the literal carries escapes.
        let src = "fn f() -> String {\n    format!(\"{{x}}\")\n}\n".to_string();
        let off = src.find("format!").expect("fixture has the site");
        let covered = "format!(\"{{x}}\")"; // 17 bytes, brace-carrying
        // clippy's own (wrong) suggestion: the escaped literal verbatim.
        let feed = vec![ClippyDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 2,
            lint: "useless_format".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![(
                    off,
                    off + covered.len(),
                    "\"{{x}}\".to_string()".to_string(),
                )],
            }),
        }];
        let lane = oracle_edits(&feed, "src/lib.rs", &src, &["useless_format"]);
        assert!(lane.edits.is_empty(), "escape class must refuse");
        assert_eq!(lane.dropped, 1, "the refusal counts the candidate");
    }

    #[test]
    fn oracle_edits_consume_useless_format_brace_free() {
        // Control: brace-free literal — the suggestion is exact and the
        // lane consumes it (the guard must not over-refuse the clean shape).
        let src = "fn f() -> String {\n    format!(\"plain\")\n}\n".to_string();
        let off = src.find("format!").expect("fixture has the site");
        let covered = "format!(\"plain\")"; // 16 bytes, brace-free
        let feed = vec![ClippyDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 2,
            lint: "useless_format".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![(
                    off,
                    off + covered.len(),
                    "\"plain\".to_string()".to_string(),
                )],
            }),
        }];
        let lane = oracle_edits(&feed, "src/lib.rs", &src, &["useless_format"]);
        assert_eq!(lane.dropped, 0);
        assert_eq!(lane.edits.len(), 1, "clean shape still consumes");
        let patched = splice_sorted(&src, std::slice::from_ref(&lane.edits[0].edit));
        assert!(
            patched.contains("\"plain\".to_string()"),
            "terminal form wrong: {patched}"
        );
        syn::parse_file(&patched).expect("parses");
    }

    #[test]
    fn zero_width_parts_merge_as_pure_insertions() {
        let src = probe_src();
        // Insert `#[inline] ` immediately before `fn f` — a zero-width
        // part at a syntactically valid point (the parse gate accepts).
        let off = src.find("fn f").unwrap();
        let feed = vec![ClippyDiagnostic {
            file: "src/lib.rs".to_string(),
            line: 1,
            lint: "needless_return".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![(off, off, "#[inline] ".to_string())],
            }),
        }];
        let lane = oracle_edits(&feed, "src/lib.rs", &src, &["needless_return"]);
        assert_eq!(lane.edits.len(), 1);
        assert_eq!(lane.edits[0].edit.replacement, "#[inline] ");
        assert_eq!(lane.edits[0].edit.start, lane.edits[0].edit.end);
        let patched = splice_sorted(&src, std::slice::from_ref(&lane.edits[0].edit));
        assert!(patched.starts_with("#[inline] fn f"));
    }

    #[test]
    fn spans_overlap_zero_width_only_strictly_inside() {
        let mk = |s: usize, e: usize, r: &str| crate::edit_emitter::SourceEdit {
            start: s,
            end: e,
            replacement: r.to_string(),
        };
        let range = mk(10, 20, "x");
        let inside = mk(15, 15, "");
        let at_start = mk(10, 10, "");
        let at_end = mk(20, 20, "");
        assert!(spans_overlap(&range, &inside));
        assert!(!spans_overlap(&range, &at_start));
        assert!(!spans_overlap(&range, &at_end));
    }
}

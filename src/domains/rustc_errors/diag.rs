//! rustc JSON diagnostics, suggestion-aware (Issue 035 T1; Plan 019 T1.1).
//!
//! [`crate::fix_verify::parse_error_locations`] already parses cargo's
//! JSON-lines output, but it answers one question — *which lines have
//! errors* — and deliberately drops everything else. The `rustc_errors`
//! domain needs two fields it discards:
//!
//! - `code.code` — the `E0597`-style identifier. This domain is **exact-keyed**
//!   by it (no retrieval; that is why the feature does not pull
//!   `latent_retrieval`, per Plan 019 T0.3).
//! - `spans[].suggested_replacement` + `suggestion_applicability` — whether
//!   rustc already ships a fix, and whether `cargo fix` will apply it.
//!
//! That second field is the whole reason this module exists. Plan 019 opened
//! by asserting rustc emits NO machine-applicable suggestions for the
//! borrowck family; the assertion was false on 4 of 7 codes, and nothing in
//! the tree could have told us so, because no parser here read the field.
//! This module makes the premise measurable instead of assumed.
//!
//! Pure parsing only — no subprocess, no I/O. [`super::probe`] supplies the
//! JSON.

use serde::Deserialize;

/// How confident rustc is that its suggestion can be applied mechanically.
///
/// Only [`Applicability::MachineApplicable`] is applied by `cargo fix`; the
/// rest are display-only hints. The distinction is the contested/uncontested
/// boundary of the whole `rustc_errors` product thesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Applicability {
    /// `cargo fix` will apply this automatically.
    MachineApplicable,
    /// Has placeholders a human must fill in.
    HasPlaceholders,
    /// May not compile / may change semantics.
    MaybeIncorrect,
    /// rustc explicitly declines to say.
    Unspecified,
    /// Field absent or an unrecognized value.
    Unknown,
}

impl Applicability {
    /// Parse rustc's `suggestion_applicability` string.
    #[must_use]
    pub fn from_json(raw: Option<&str>) -> Self {
        match raw {
            Some("MachineApplicable") => Self::MachineApplicable,
            Some("HasPlaceholders") => Self::HasPlaceholders,
            Some("MaybeIncorrect") => Self::MaybeIncorrect,
            Some("Unspecified") => Self::Unspecified,
            _ => Self::Unknown,
        }
    }

    /// Would `cargo fix` apply a suggestion at this applicability?
    #[must_use]
    pub fn is_auto_applied(self) -> bool {
        matches!(self, Self::MachineApplicable)
    }
}

/// One suggested replacement rustc attached to a span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestionSpan {
    pub file: String,
    pub line_start: usize,
    /// Byte offsets of the replaced region in the reported file, when rustc
    /// included them. The Phase 2+ playbooks splice on these; `None` on
    /// older-format lines forces the playbooks to locate by text.
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
    pub replacement: String,
    pub applicability: Applicability,
    /// The `help:`/`note:` text the suggestion hung off — the human-readable
    /// strategy name (`consider cloning the value ...`).
    pub message: String,
}

/// One labeled span from the diagnostic's top-level `spans` — the
/// oracle-provided anchors the E0597 playbooks key on. Unlike suggestions,
/// these are rustc's OWN analysis output ("binding `x` declared here",
/// "value captured here"), so a playbook that splices on them is consuming
/// the oracle's location knowledge, not guessing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabeledSpan {
    pub file: String,
    pub line_start: usize,
    pub is_primary: bool,
    /// Byte range of the span in `file`. rustc JSON always carries these for
    /// file spans; spans without them (macros, older cargo) are dropped.
    pub byte_start: usize,
    pub byte_end: usize,
    pub label: String,
}

/// One `error[E….]` diagnostic, keyed by code, with every suggestion span
/// found anywhere in its `children` tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustcErrorDiag {
    /// `E0597`. Diagnostics without a code are dropped — this domain keys on it.
    pub code: String,
    pub message: String,
    /// Primary span, when the diagnostic has one: `(file, line_start)`.
    pub primary: Option<(String, usize)>,
    pub suggestions: Vec<SuggestionSpan>,
    /// Labeled spans of the top-level diagnostic (primary included, when it
    /// carries a label + byte offsets). The Phase 2 playbooks' anchors.
    pub labeled_spans: Vec<LabeledSpan>,
    /// The full pre-rendered diagnostic text, when cargo included it. The
    /// Phase 4 miss-capture dumps write this verbatim (`<repo>/.heal/misses/`).
    pub rendered: Option<String>,
}

impl RustcErrorDiag {
    /// Suggestions `cargo fix` would apply without asking.
    #[must_use]
    pub fn machine_applicable(&self) -> Vec<&SuggestionSpan> {
        self.suggestions
            .iter()
            .filter(|s| s.applicability.is_auto_applied())
            .collect()
    }

    /// Is this code **uncontested** — i.e. rustc offers no auto-fix, so
    /// `cargo fix` is genuinely blind and any coverage we add is net-new?
    #[must_use]
    pub fn is_uncontested(&self) -> bool {
        self.machine_applicable().is_empty()
    }

    /// Dedupe identity: `(code, primary file, primary line)`. rustc emits
    /// the same site more than once (macro-expansion contexts, repeated
    /// passes); a driver that proposed per-diagnostic would repair the same
    /// site twice.
    #[must_use]
    pub fn site_key(&self) -> (String, String, usize) {
        (
            self.code.clone(),
            self.primary
                .as_ref()
                .map_or_else(String::new, |p| p.0.clone()),
            self.primary.as_ref().map_or(0, |p| p.1),
        )
    }

    /// Is this diagnostic's code emitted at a stage that BLOCKS all later
    /// compiler stages? See [`MASKING_STAGE_CODES`].
    #[must_use]
    pub fn is_masking(&self) -> bool {
        is_masking_code(&self.code)
    }
}

/// Codes emitted at a compiler stage whose failure BLOCKS every later stage:
/// while any of these is present, the later stages' diagnostics do not exist
/// yet — fixing one can REVEAL new errors (unmasking), so the driver's keep
/// rule ranks them ahead of the raw count (Issue 100).
///
/// Membership is MEASURED, never assumed — each member cites the probe that
/// justified it. A taxonomy guess here would silently re-rank every keep.
///
/// - `E0614` (deref of a non-pointer, typeck): measured 2026-09-13 on rustc
///   1.98.1 — `let head = v[0]; … *head` over a broken alias pair reports
///   EXACTLY the E0614; the co-present E0499 appears only after the star is
///   removed (typeck failure blocks borrowck entirely). Bench 089's F1/F3
///   clone-out rows are the same mechanism healer-side.
pub const MASKING_STAGE_CODES: &[&str] = &["E0614"];

/// Membership test for [`MASKING_STAGE_CODES`].
#[must_use]
pub fn is_masking_code(code: &str) -> bool {
    MASKING_STAGE_CODES.contains(&code)
}

/// Collapse diagnostics that share a [`RustcErrorDiag::site_key`] — the
/// multi-span-repeat shape Plan 019 T1.1 dedupes. Suggestions of merged
/// duplicates are concatenated (identical suggestions deduped); the first
/// diagnostic's message + rendered text win.
#[must_use]
pub fn dedupe_by_site(diags: &[RustcErrorDiag]) -> Vec<RustcErrorDiag> {
    let mut out: Vec<RustcErrorDiag> = Vec::new();
    for d in diags {
        match out.iter_mut().find(|o| o.site_key() == d.site_key()) {
            Some(kept) => {
                for s in &d.suggestions {
                    if !kept.suggestions.contains(s) {
                        kept.suggestions.push(s.clone());
                    }
                }
            }
            None => out.push(d.clone()),
        }
    }
    out
}

// ── JSON shapes (cargo's `--message-format=json` lines) ───────────────────

#[derive(Deserialize)]
struct CargoMessage {
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    message: Option<JsonDiag>,
}

#[derive(Deserialize)]
struct JsonDiag {
    #[serde(default)]
    level: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: Option<JsonCode>,
    #[serde(default)]
    spans: Vec<JsonSpan>,
    #[serde(default)]
    children: Vec<JsonDiag>,
    /// The full pre-rendered diagnostic text (the `rendered` field cargo
    /// attaches at the top level of each compiler message).
    #[serde(default)]
    rendered: Option<String>,
}

#[derive(Deserialize)]
struct JsonCode {
    #[serde(default)]
    code: String,
}

#[derive(Deserialize)]
struct JsonSpan {
    file_name: String,
    line_start: usize,
    #[serde(default)]
    is_primary: bool,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    byte_start: Option<usize>,
    #[serde(default)]
    byte_end: Option<usize>,
    #[serde(default)]
    suggested_replacement: Option<String>,
    #[serde(default)]
    suggestion_applicability: Option<String>,
}

/// Parse cargo JSON-lines into error diagnostics carrying an `E….` code.
///
/// - Only `reason == "compiler-message"` at `level` starting `error`.
/// - Diagnostics with no `code` are dropped (unkeyable by this domain).
/// - Suggestion spans are collected from the diagnostic AND, recursively,
///   its children — rustc hangs replacements off `help:` children, never off
///   the top-level span, so a non-recursive walk finds nothing.
/// - Non-JSON lines are skipped (cargo interleaves plain progress output).
#[must_use]
pub fn parse_error_diags(json_stdout: &str) -> Vec<RustcErrorDiag> {
    let mut out = Vec::new();
    for line in json_stdout.lines() {
        let Ok(msg) = serde_json::from_str::<CargoMessage>(line.trim()) else {
            continue;
        };
        if msg.reason.as_deref() != Some("compiler-message") {
            continue;
        }
        let Some(diag) = msg.message else { continue };
        if !diag.level.starts_with("error") {
            continue;
        }
        let Some(code) = diag.code.as_ref().map(|c| c.code.clone()) else {
            continue;
        };
        if code.is_empty() {
            continue;
        }
        let primary = diag
            .spans
            .iter()
            .find(|s| s.is_primary)
            .map(|s| (s.file_name.clone(), s.line_start));
        let mut suggestions = Vec::new();
        collect_suggestions(&diag, &mut suggestions);
        let labeled_spans = diag
            .spans
            .iter()
            .filter_map(|s| {
                let label = s.label.as_deref()?;
                if label.is_empty() {
                    return None;
                }
                Some(LabeledSpan {
                    file: s.file_name.clone(),
                    line_start: s.line_start,
                    is_primary: s.is_primary,
                    byte_start: s.byte_start?,
                    byte_end: s.byte_end?,
                    label: label.to_string(),
                })
            })
            .collect();
        out.push(RustcErrorDiag {
            code,
            message: diag.message.clone(),
            primary,
            suggestions,
            labeled_spans,
            rendered: diag.rendered.clone(),
        });
    }
    out.shrink_to_fit();
    out
}

fn collect_suggestions(diag: &JsonDiag, out: &mut Vec<SuggestionSpan>) {
    for s in &diag.spans {
        let Some(replacement) = s.suggested_replacement.as_ref() else {
            continue;
        };
        out.push(SuggestionSpan {
            file: s.file_name.clone(),
            line_start: s.line_start,
            byte_start: s.byte_start,
            byte_end: s.byte_end,
            replacement: replacement.clone(),
            applicability: Applicability::from_json(s.suggestion_applicability.as_deref()),
            message: diag.message.clone(),
        });
    }
    for child in &diag.children {
        collect_suggestions(child, out);
    }
}

/// Group parsed diagnostics by error code, counting machine-applicable
/// suggestions — the premise table's row shape.
#[must_use]
pub fn machine_applicable_by_code(diags: &[RustcErrorDiag]) -> Vec<(String, usize, usize)> {
    let mut codes: Vec<String> = diags.iter().map(|d| d.code.clone()).collect();
    codes.sort();
    codes.dedup();
    codes
        .into_iter()
        .map(|code| {
            let matching: Vec<&RustcErrorDiag> = diags.iter().filter(|d| d.code == code).collect();
            let total: usize = matching.iter().map(|d| d.suggestions.len()).sum();
            let auto: usize = matching.iter().map(|d| d.machine_applicable().len()).sum();
            (code, total, auto)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real `cargo check --message-format=json` line, trimmed to the fields
    /// this module reads. The suggestion hangs off a `help:` CHILD — the
    /// shape a non-recursive walk misses.
    const E0596_LINE: &str = r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot borrow `x` as mutable","code":{"code":"E0596"},"spans":[{"file_name":"src/lib.rs","line_start":5,"is_primary":true}],"children":[{"level":"help","message":"consider changing this to be mutable","code":null,"spans":[{"file_name":"src/lib.rs","line_start":5,"is_primary":true,"suggested_replacement":"mut ","suggestion_applicability":"MachineApplicable"}],"children":[]}]}}"#;

    const E0499_LINE: &str = r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot borrow `*v` as mutable more than once at a time","code":{"code":"E0499"},"spans":[{"file_name":"src/lib.rs","line_start":9,"is_primary":true}],"children":[{"level":"note","message":"first borrow later used here","code":null,"spans":[{"file_name":"src/lib.rs","line_start":9,"is_primary":false}],"children":[]}]}}"#;

    #[test]
    fn parses_code_primary_and_child_suggestion() {
        let diags = parse_error_diags(E0596_LINE);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert_eq!(d.code, "E0596");
        assert_eq!(d.primary, Some(("src/lib.rs".to_string(), 5)));
        assert_eq!(d.suggestions.len(), 1, "suggestion lives in a help child");
        assert_eq!(d.suggestions[0].replacement, "mut ");
        assert_eq!(
            d.suggestions[0].applicability,
            Applicability::MachineApplicable
        );
        assert!(
            !d.is_uncontested(),
            "E0596 is contested — cargo fix owns it"
        );
    }

    #[test]
    fn uncontested_code_has_no_machine_applicable_suggestion() {
        let diags = parse_error_diags(E0499_LINE);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "E0499");
        assert!(diags[0].suggestions.is_empty());
        assert!(diags[0].is_uncontested());
    }

    #[test]
    fn warnings_and_codeless_and_garbage_are_dropped() {
        let warning = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused","code":{"code":"unused_variables"},"spans":[],"children":[]}}"#;
        let codeless = r#"{"reason":"compiler-message","message":{"level":"error","message":"aborting","code":null,"spans":[],"children":[]}}"#;
        let other = r#"{"reason":"compiler-artifact","target":{"name":"x"}}"#;
        let input = format!("{warning}\n{codeless}\n{other}\nnot json at all\n\n{E0596_LINE}");
        let diags = parse_error_diags(&input);
        assert_eq!(diags.len(), 1, "only the coded error survives");
        assert_eq!(diags[0].code, "E0596");
    }

    #[test]
    fn applicability_maps_every_rustc_value() {
        assert_eq!(
            Applicability::from_json(Some("MachineApplicable")),
            Applicability::MachineApplicable
        );
        assert_eq!(
            Applicability::from_json(Some("MaybeIncorrect")),
            Applicability::MaybeIncorrect
        );
        assert_eq!(
            Applicability::from_json(Some("HasPlaceholders")),
            Applicability::HasPlaceholders
        );
        assert_eq!(
            Applicability::from_json(Some("Unspecified")),
            Applicability::Unspecified
        );
        assert_eq!(Applicability::from_json(None), Applicability::Unknown);
        assert_eq!(
            Applicability::from_json(Some("SomethingNewInRustc")),
            Applicability::Unknown
        );
        // Only MachineApplicable is auto-applied — the contested boundary.
        assert!(Applicability::MachineApplicable.is_auto_applied());
        for a in [
            Applicability::MaybeIncorrect,
            Applicability::HasPlaceholders,
            Applicability::Unspecified,
            Applicability::Unknown,
        ] {
            assert!(!a.is_auto_applied(), "{a:?} must not count as auto-applied");
        }
    }

    #[test]
    fn groups_counts_by_code() {
        let input = format!("{E0596_LINE}\n{E0499_LINE}");
        let table = machine_applicable_by_code(&parse_error_diags(&input));
        assert_eq!(
            table,
            vec![("E0499".to_string(), 0, 0), ("E0596".to_string(), 1, 1),]
        );
    }

    /// A real cargo line with `rendered` + byte offsets on the span — the
    /// Phase 1 parse-target shape (Plan 019 T1.1).
    #[test]
    fn parses_rendered_and_byte_offsets() {
        let line = r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot borrow `x` as mutable","rendered":"error[E0596]: cannot borrow `x` as mutable\n --> src/lib.rs:5:9\n","code":{"code":"E0596"},"spans":[{"file_name":"src/lib.rs","line_start":5,"is_primary":true,"byte_start":82,"byte_end":92,"suggested_replacement":"mut x","suggestion_applicability":"MachineApplicable"}],"children":[]}}"#;
        let diags = parse_error_diags(line);
        assert_eq!(diags.len(), 1);
        let d = &diags[0];
        assert!(d.rendered.as_deref().is_some_and(|r| r.contains("E0596")));
        assert_eq!(d.suggestions.len(), 1);
        let s = &d.suggestions[0];
        assert_eq!(s.byte_start, Some(82));
        assert_eq!(s.byte_end, Some(92));
        // Absent fields decode as None (older toolchains emit no offsets).
        let diags_old = parse_error_diags(E0596_LINE);
        assert_eq!(diags_old[0].rendered, None);
        assert_eq!(diags_old[0].suggestions[0].byte_start, None);
    }

    /// A real E0597 line — the top-level spans carry the labeled anchors the
    /// Phase 2 playbooks key on ("binding … declared here", "value captured
    /// here", "type annotation requires … 'static", the dropped-`}` span).
    /// Spans without a label or without byte offsets are excluded.
    #[test]
    fn parses_labeled_spans_from_top_level() {
        let line = r#"{"reason":"compiler-message","message":{"level":"error","message":"`x` does not live long enough","code":{"code":"E0597"},"spans":[{"file_name":"src/lib.rs","line_start":6,"is_primary":true,"byte_start":40,"byte_end":42,"label":"borrowed value does not live long enough"},{"file_name":"src/lib.rs","line_start":6,"byte_start":61,"byte_end":63,"label":"value captured here"},{"file_name":"src/lib.rs","line_start":5,"byte_start":30,"byte_end":34,"label":"binding `name` declared here"},{"file_name":"src/lib.rs","line_start":6,"byte_start":16,"byte_end":34,"label":"type annotation requires that `name` is borrowed for `'static`"},{"file_name":"src/lib.rs","line_start":6,"byte_start":50,"byte_end":54}],"children":[]}}"#;
        let diags = parse_error_diags(line);
        assert_eq!(diags.len(), 1);
        let ls = &diags[0].labeled_spans;
        assert_eq!(ls.len(), 4, "unlabeled span excluded: {ls:?}");
        assert!(ls[0].is_primary);
        assert!(!ls[1].is_primary);
        let ann = ls
            .iter()
            .find(|s| s.label.contains("type annotation"))
            .unwrap();
        assert_eq!((ann.byte_start, ann.byte_end), (16, 34));
        let cap = ls
            .iter()
            .find(|s| s.label == "value captured here")
            .unwrap();
        assert_eq!((cap.byte_start, cap.byte_end), (61, 63));
        // A label WITHOUT byte offsets is dropped, not half-kept.
        let no_bytes = line.replace(
            "\"file_name\":\"src/lib.rs\",\"line_start\":6,\"byte_start\":61,\"byte_end\":63,\"label\":\"value captured here\"",
            "\"file_name\":\"src/lib.rs\",\"line_start\":6,\"label\":\"value captured here\"",
        );
        let diags2 = parse_error_diags(&no_bytes);
        assert_eq!(diags2[0].labeled_spans.len(), 3);
    }

    #[test]
    fn dedupe_by_site_merges_repeats() {
        // The same (code, file, line) twice — the multi-span-repeat shape
        // (macro expansion, repeated passes). One differing suggestion on
        // the second emission must SURVIVE the merge; the site must count
        // once.
        let second = E0596_LINE.replace(
            "consider changing this to be mutable",
            "consider changing this to be mutable (second pass)",
        );
        let input = format!("{E0596_LINE}\n{second}\n{E0499_LINE}");
        let diags = dedupe_by_site(&parse_error_diags(&input));
        assert_eq!(diags.len(), 2, "3 in → 2 out (one site merged)");
        let e0596 = diags.iter().find(|d| d.code == "E0596").unwrap();
        assert_eq!(
            e0596.suggestions.len(),
            2,
            "differing suggestions merge; identical ones dedupe"
        );
        // Identical duplicates collapse.
        let identical = format!("{E0596_LINE}\n{E0596_LINE}");
        let diags = dedupe_by_site(&parse_error_diags(&identical));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].suggestions.len(), 1);
    }

    /// Issue 100 — the masking-stage membership set is the keep rule's
    /// primary term; it must contain EXACTLY the measured members and the
    /// borrowck family must NOT be in it (borrowck is the masked stage, not
    /// the masking one).
    #[test]
    fn masking_stage_codes_carry_exactly_the_measured_members() {
        assert_eq!(MASKING_STAGE_CODES, &["E0614"]);
        assert!(is_masking_code("E0614"));
        for not_masking in ["E0499", "E0502", "E0505", "E0596", "E0597", "E0507"] {
            assert!(!is_masking_code(not_masking));
        }
        let e0499 = parse_error_diags(E0499_LINE);
        assert!(!e0499[0].is_masking());
        let e0614 = parse_error_diags(
            r#"{"reason":"compiler-message","message":{"level":"error","message":"type `i32` cannot be dereferenced","code":{"code":"E0614"},"spans":[{"file_name":"src/lib.rs","line_start":7,"is_primary":true,"byte_start":100,"byte_end":105,"label":"can't be dereferenced"}],"children":[]}}"#,
        );
        assert_eq!(e0614.len(), 1);
        assert!(e0614[0].is_masking());
    }
}

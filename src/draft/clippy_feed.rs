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
//! was given — default features, loud status lines, `RIIR_REFINE_AUTO_ORACLE=0`
//! restores the old never-spawn posture (see [`crate::clippy_oracle`]).
//! Toolchain and target-dir discipline stay the caller's environment; a
//! manual `--clippy-json` still wins outright. Before that the oracle was
//! manual-only, and the owner's finding was sharp: a gain path that exists
//! only behind a flag nobody discovers is not a gain path — the auto-oracle
//! must do every good for the user (the auto-login/auto-sync
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
    /// The primary span's byte range `(byte_start, byte_end)` in the file
    /// clippy ran on — the coordinates the repo-posture gate matches edits
    /// against (same lint + overlapping span). `None` when the feed's span
    /// row lacked byte offsets (never in real cargo JSON; defensive).
    pub span: Option<(usize, usize)>,
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
        let Some(loc) = primary_span_location(message) else {
            continue;
        };
        let suggestion = suggestion_from_message(message, loc.file);
        let diag = ClippyDiagnostic {
            file: loc.file.replace('\\', "/"),
            line: loc.line,
            lint: lint.to_string(),
            span: loc.bytes,
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
                if sp
                    .get("suggestion_applicability")
                    .and_then(serde_json::Value::as_str)
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

/// What [`primary_span_location`] extracts: the primary span's file, its
/// 1-based start line, and its byte range when the row carries one
/// (`None` when `byte_start`/`byte_end` are absent — never in real cargo
/// JSON; defensive, and a span-less diagnostic cannot posture-match).
struct PrimarySpan<'a> {
    file: &'a str,
    line: usize,
    bytes: Option<(usize, usize)>,
}

/// The primary span's location — the first `is_primary: true` span, else
/// the first span (the [`clippy_oracle`] `primary_span_file` rule —
/// diagnostics without an explicit primary still anchor at their only
/// span).
fn primary_span_location(message: &serde_json::Value) -> Option<PrimarySpan<'_>> {
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
    let start = span
        .get("byte_start")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok());
    let end = span
        .get("byte_end")
        .and_then(serde_json::Value::as_u64)
        .and_then(|n| usize::try_from(n).ok());
    let bytes = match (start, end) {
        (Some(s), Some(e)) if s <= e => Some((s, e)),
        _ => None,
    };
    Some(PrimarySpan { file, line, bytes })
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
pub fn spans_overlap(
    a: &crate::edit_emitter::SourceEdit,
    b: &crate::edit_emitter::SourceEdit,
) -> bool {
    a.start < b.end && b.start < a.end
}

// ── The repo-posture gate (Plan 165, the 2026-09-21 riir-ai intake) ──────
//
// `--fix` on a foreign repo restyled docs the target's own gate never
// complained about (the 26:1 doc_markdown churn: our corpus is BROADER
// than clippy's `[a-z][A-Z]` regex on that lint). The posture gate
// intersects the clippy-lane fix edits with the target repo's OWN clippy
// diagnostics — the same feed the anchor lane consumes — so an edit
// applies only where the target's clippy actually flagged.

/// Build the per-lint posture index for ONE scanned file: the primary-span
/// byte ranges (original on-disk coordinates) of every feed diagnostic
/// that names this file ([`paths_ref_match`]) and carries byte offsets.
/// Later fixpoint passes remap these through the applied splices
/// ([`crate::edit_emitter::map_span_through_splices`]).
#[must_use]
pub fn posture_spans(
    feed: &[ClippyDiagnostic],
    display_path: &str,
) -> std::collections::BTreeMap<String, Vec<(usize, usize)>> {
    let mut map: std::collections::BTreeMap<String, Vec<(usize, usize)>> =
        std::collections::BTreeMap::new();
    for d in feed {
        if !paths_ref_match(&d.file, display_path) {
            continue;
        }
        let Some(span) = d.span else { continue };
        map.entry(d.lint.clone()).or_default().push(span);
    }
    map
}

/// Does the posture index confirm ONE edit? True iff a same-lint span
/// overlaps the edit's byte range in the SAME coordinate frame — the
/// [`spans_overlap`] predicate over raw ranges (zero-width edits pass only
/// strictly inside a flagged region, an insertion at a boundary is
/// compatible). A lint absent from the index confirms nothing.
#[must_use]
pub fn posture_allows(
    index: &std::collections::BTreeMap<String, Vec<(usize, usize)>>,
    lint: &str,
    start: usize,
    end: usize,
) -> bool {
    let Some(spans) = index.get(lint) else {
        return false;
    };
    spans.iter().any(|&(s, e)| s < end && start < e)
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
    /// `needless_range_loop` candidates declined by the P34 length proof
    /// (range end is not a syntactic zero-arg `.len()` call). Reported
    /// separately from `dropped`: a decline is a DELIBERATE refusal of a
    /// well-formed suggestion, never staleness — the caller names the
    /// manual path (bounded slice) instead of questioning the feed.
    pub range_declined: usize,
    /// `doc_markdown` suggestions declined on a shell-command doc line
    /// (Issue 135 T4 — backticking one argv token breaks copy-paste; the
    /// repair is to fence/indent the command, by hand).
    pub doc_command_declined: usize,
    /// P104 (the P98 dogfood's E0433 class): suggestions declined because
    /// the replacement spells a CRATE-ROOT-relative path (`module::Type` —
    /// clippy renders element types from its own resolution session) that
    /// does not resolve at the destination file's scope. The corpus
    /// proposer's file-local spelling is the fallback; else the manual
    /// path.
    pub path_scope_declined: usize,
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
/// - the P34 `needless_range_loop` length guard ([`range_loop_len_proof`]) —
///   a range-dropping rewrite is mechanical only when the range end is the
///   iterated collection's length, and clippy's cross-binding proof of that
///   is not auditable here. The end must be a syntactic zero-arg `.len()`
///   call (literal-0 start, half-open limits); everything else declines
///   (riir-ai Issue 981 — the tail-slice `0..sph` corruption). The manual
///   path keeps the range visible: `&mut buf[base..][..n]` or `.take(n)`;
/// - merged range out of bounds / off a char boundary (a stale feed);
/// - oracle-vs-oracle overlap (leftmost wins);
/// - the P104 scope guard ([`FileScopeIndex`]) — clippy's suggestion may
///   spell a type as a CRATE-ROOT-relative path from its own resolution
///   session (`draft::corpus_index::ScoredRule`); at the destination file
///   that path resolves only where the root module name happens to be in
///   scope (the P98 dogfood measured 11 E0433s). The guard reads the
///   file's use-tree + top-level items and refuses the verbatim apply
///   when the replacement introduces a multi-segment path whose root is
///   unexplained — a lowercase root declines even under a glob import
///   (`use super::*` covers bare type names, not crate-root module
///   paths). Doc-markdown suggestions are exempt (they rewrite comments,
///   where no path must resolve). The corpus proposer's file-local
///   spelling is the fallback;
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
    let mut range_declined = 0usize;
    let mut path_scope_declined = 0usize;
    // P104: the scope index is built lazily — only when a candidate
    // replacement actually carries a `::` path (the common suggestions —
    // method-call rewrites, doc backticks — never pay for the parse).
    let mut scope: Option<FileScopeIndex> = None;
    // the skip arm is cfg'd out under thin feature sets; the counter stays
    // for the OracleLane shape
    #[cfg_attr(not(feature = "clippy_lints"), allow(unused_mut))]
    let mut doc_command_declined = 0usize;
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
                // P34 (riir-ai Issue 981, live evidence): a
                // `needless_range_loop` suggestion DROPS the loop's range —
                // the rewrite is mechanical only when the range end equals
                // the iterated collection's length, and clippy proves that
                // with cross-binding analysis this lane cannot see. Require
                // a LOCAL SYNTACTIC proof (zero-arg `.len()` end, literal-0
                // start, exclusive limits); every other bound shape is
                // declined — the 981 corruption was exactly this rewrite
                // over `let gs = &mut buf[h * sph..]` (a tail slice whose
                // len > sph). Drops the edit, never produces a wrong one.
                if d.lint == "needless_range_loop" && !range_loop_len_proof(src, edit.start) {
                    range_declined += 1;
                    continue;
                }
                // Issue 135 T4: the proposer's command-line skip, mirrored
                // at the oracle door (clippy suggests the same argv-token
                // backtick the proposer refuses). The skip mirrors a
                // clippy_lints proposer rule, so the arm exists only under
                // the domain's own feature (the latent_retrieval-only lane
                // compiles this module with the domain configured out).
                #[cfg(feature = "clippy_lints")]
                if d.lint == "doc_markdown"
                    && crate::domains::clippy_lints::proposers::doc_markdown::doc_source_line_is_command(
                        src, edit.start,
                    )
                {
                    doc_command_declined += 1;
                    continue;
                }
                // P104: the replacement spells a crate-root path this file
                // cannot resolve (the P98 E0433 class) — decline; details on
                // scope_guard_declines. Doc-markdown exempt (comments).
                if d.lint != "doc_markdown"
                    && scope_guard_declines(&mut scope, src, &edit.replacement)
                {
                    path_scope_declined += 1;
                    continue;
                }
                candidates.push((edit.start, edit.end, edit.replacement, d.lint.clone()));
            }
            None => dropped += 1, // stale offsets / misaligned boundaries
        }
    }
    // Oracle-vs-oracle overlap: leftmost wins.
    let kept = leftmost_wins(candidates, &mut dropped);
    if kept.is_empty() {
        return OracleLane {
            edits: Vec::new(),
            dropped,
            range_declined,
            doc_command_declined,
            path_scope_declined,
        };
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
            range_declined,
            doc_command_declined,
            path_scope_declined,
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
    OracleLane {
        edits,
        dropped,
        range_declined,
        doc_command_declined,
        path_scope_declined,
    }
}

/// P104 arm body: does the SCOPE GUARD decline this replacement — i.e.
/// does it spell a crate-root-relative path the destination file cannot
/// resolve? Lazily builds the file's [`FileScopeIndex`] on the first
/// `::`-carrying replacement (the common suggestions never pay for the
/// parse).
fn scope_guard_declines(scope: &mut Option<FileScopeIndex>, src: &str, replacement: &str) -> bool {
    if !replacement.contains("::") {
        return false;
    }
    let index = scope.get_or_insert_with(|| FileScopeIndex::build(src));
    !index.replacement_in_scope(replacement)
}

/// Oracle-vs-oracle overlap: LEFTMOST WINS — sorted by start; a wide
/// early span covers later ones via the running end. Each covered-over
/// candidate counts as a drop.
fn leftmost_wins(
    mut candidates: Vec<(usize, usize, String, String)>,
    dropped: &mut usize,
) -> Vec<(usize, usize, String, String)> {
    candidates.sort_by_key(|(start, _, _, _)| *start);
    let mut covered_until = 0usize;
    let mut kept: Vec<(usize, usize, String, String)> = Vec::new();
    for cand in candidates {
        let (start, end, _, _) = &cand;
        if *start < covered_until {
            *dropped += 1;
            continue;
        }
        covered_until = (*end).max(covered_until);
        kept.push(cand);
    }
    kept
}

/// P104: a file-level scope index for the oracle lane's import-existence
/// guard — the names the FILE can resolve at a path position, plus
/// whether any glob import (a `use x::*`) is present.
///
/// Soundness posture: the guard only PASSes a path root it can explain
/// (a keyword, the extern prelude, a use-tree name, a file-local item) or
/// a glob-plausible TYPE root; everything else declines. A false decline
/// loses one mechanical fix to the manual path; a false pass reintroduces
/// the E0433 class the guard exists for. The verify compile gate remains
/// the backstop for the guard's residual classes.
struct FileScopeIndex {
    /// Every name any `use` item brings into scope: path prefixes
    /// (`use a::b::{c}` makes both `a` and `b` usable as `x::...` roots
    /// AND `c` usable bare), leaf names, and rename targets.
    /// Over-collecting bare coverage is the safe direction here: the
    /// guard's consumers are PATH roots (`a::...`), and a prefix covers
    /// exactly that shape.
    imported: std::collections::HashSet<String>,
    /// Top-level item names of the file (mods, fns, types, consts — any
    /// visibility, any nesting depth).
    local_items: std::collections::HashSet<String>,
    /// Any glob import anywhere in the file (the P98 shape: a tests mod's
    /// `use super::*` covers bare type names but NOT crate-root module
    /// paths).
    has_glob: bool,
}

/// Path roots whose multi-segment forms always resolve: the keywords with
/// path semantics, the extern-prelude crates, and the prelude's own types
/// (`Vec::<T>::new()`, `String::from`, `std::mem::take` — the common
/// suggestion vocabularies must not pay the decline). A name missing from
/// this list that resolves anyway costs one decline, never a wrong edit.
const PATH_SCOPE_ALWAYS_OK: &[&str] = &[
    "crate", "self", "super", "std", "core", "alloc", "bool", "char", "str", "u8", "u16", "u32",
    "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64", "String",
    "Vec", "Box", "Option", "Result", "Some", "None", "Ok", "Err",
];

fn p104_is_ident_start(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphabetic()
}

fn p104_is_ident_continue(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

impl FileScopeIndex {
    /// Build from the file's source. An unparseable source yields an EMPTY
    /// index (every checked path declines) — conservative, and the
    /// whole-file parse gate drops those edits anyway.
    fn build(src: &str) -> Self {
        use syn::visit::Visit as _;
        let Ok(file) = syn::parse_file(src) else {
            return Self {
                imported: std::collections::HashSet::new(),
                local_items: std::collections::HashSet::new(),
                has_glob: false,
            };
        };
        let mut c = Collector::default();
        c.visit_file(&file);
        Self {
            imported: c.imported,
            local_items: c.local_items,
            has_glob: c.has_glob,
        }
    }

    /// Does `replacement` introduce any multi-segment path whose root the
    /// file cannot resolve? String-literal contents and line comments are
    /// skipped (a `"a::b"` is text, not a path); tokens are maximal
    /// `ident(::ident)+` sequences (a `::<` turbofish, a `!` macro, or a
    /// non-ident continuation ends the token).
    fn replacement_in_scope(&self, replacement: &str) -> bool {
        let bytes = replacement.as_bytes();
        let mut i = 0usize;
        let mut in_string = false;
        let mut in_line_comment = false;
        while i < bytes.len() {
            let b = bytes[i];
            if in_line_comment {
                if b == 0x0A {
                    in_line_comment = false;
                }
                i += 1;
                continue;
            }
            if in_string {
                match b {
                    0x5C => i += 2, // backslash: the escaped byte is text
                    0x22 => {
                        in_string = false;
                        i += 1;
                    }
                    _ => i += 1,
                }
                continue;
            }
            match b {
                b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                    in_line_comment = true;
                    i += 2;
                }
                0x22 => {
                    in_string = true;
                    i += 1;
                }
                _ if p104_is_ident_start(b) => {
                    let start = i;
                    while i < bytes.len() && p104_is_ident_continue(bytes[i]) {
                        i += 1;
                    }
                    let first = &replacement[start..i];
                    let mut segments = 1usize;
                    while i + 2 < bytes.len()
                        && bytes[i] == b':'
                        && bytes[i + 1] == b':'
                        && p104_is_ident_start(bytes[i + 2])
                    {
                        i += 2;
                        while i < bytes.len() && p104_is_ident_continue(bytes[i]) {
                            i += 1;
                        }
                        segments += 1;
                    }
                    if segments >= 2 && !self.path_root_in_scope(first) {
                        return false;
                    }
                }
                _ => i += 1,
            }
        }
        true
    }

    fn path_root_in_scope(&self, root: &str) -> bool {
        if PATH_SCOPE_ALWAYS_OK.contains(&root) {
            return true;
        }
        if self.imported.contains(root) || self.local_items.contains(root) {
            return true;
        }
        // A glob import covers bare TYPE roots (the P98 shape: the tests
        // mod's `use super::*` sees the names bare), but a lowercase root
        // is a crate-root MODULE path shape — the measured E0433 class —
        // and globs do not carry those.
        if self.has_glob && root.chars().next().is_some_and(char::is_uppercase) {
            return true;
        }
        false
    }
}

/// P104 collector: use-tree names (prefixes + leaves + renames), item
/// names, and the glob flag, in one AST walk.
#[derive(Default)]
struct Collector {
    imported: std::collections::HashSet<String>,
    local_items: std::collections::HashSet<String>,
    has_glob: bool,
}

impl<'ast> syn::visit::Visit<'ast> for Collector {
    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        self.walk_use_tree(&u.tree);
        syn::visit::visit_item_use(self, u);
    }

    fn visit_item(&mut self, i: &'ast syn::Item) {
        let name = match i {
            syn::Item::Fn(x) => Some(&x.sig.ident),
            syn::Item::Struct(x) => Some(&x.ident),
            syn::Item::Enum(x) => Some(&x.ident),
            syn::Item::Union(x) => Some(&x.ident),
            syn::Item::Trait(x) => Some(&x.ident),
            syn::Item::Type(x) => Some(&x.ident),
            syn::Item::Const(x) => Some(&x.ident),
            syn::Item::Static(x) => Some(&x.ident),
            syn::Item::Mod(x) => Some(&x.ident),
            _ => None,
        };
        if let Some(id) = name {
            self.local_items.insert(id.to_string());
        }
        syn::visit::visit_item(self, i);
    }
}

impl Collector {
    fn walk_use_tree(&mut self, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(p) => {
                // `use a::rest` — `a` is a usable path root.
                self.imported.insert(p.ident.to_string());
                self.walk_use_tree(&p.tree);
            }
            syn::UseTree::Name(n) => {
                self.imported.insert(n.ident.to_string());
            }
            syn::UseTree::Rename(r) => {
                // `use a as b` — `b` is the in-scope spelling.
                self.imported.insert(r.rename.to_string());
            }
            syn::UseTree::Glob(_) => self.has_glob = true,
            syn::UseTree::Group(g) => {
                for t in &g.items {
                    self.walk_use_tree(t);
                }
            }
        }
    }
}

/// P34 (riir-ai Issue 981, live evidence): does the for-loop containing
/// `edit_start` carry a LOCAL, SYNTACTIC length proof — a half-open range
/// from literal `0` whose end is a zero-argument `.len()` method call?
///
/// A `needless_range_loop` suggestion replaces `for i in 0..N { coll[i] }`
/// with full-iteration `for x in coll`, mechanically identical ONLY when
/// `N == coll.len()`. clippy establishes that equality with type-aware
/// cross-binding analysis this parse-only lane cannot audit; the 981
/// corruption (`let gs = &mut buf[h * sph..]`, len > sph — one head's decay
/// multiplied into every later head's slab, misread as a GPU kernel bug for
/// a forensic session) is the cost of trusting a proof you cannot see. The
/// only bounds this lane accepts are ones whose equality is syntactic:
/// `0..coll.len()` where `coll` is also the ident every body slot
/// `<coll>[<loop ident>]` indexes — a `len()` of the PARENT buffer over a
/// tail-slice slot (`0..buf.len() { gs[i] }`, `gs = &mut buf[base..]`)
/// passes the end-shape check and declines on receiver identity.
/// Computed bounds (`n`, `a * b`, `len() / heads`),
/// non-zero starts, and inclusive limits (`0..=n` iterates n+1 — a `len()`
/// end there is off by one) all decline; the sound manual forms keep the
/// range visible (`&mut buf[base..][..n]`, `.take(n)`).
///
/// The INNERMOST containing loop is the one judged — a suggestion inside a
/// nested loop belongs to the inner header, not the outer. Unparseable
/// source declines (the whole-file parse gate drops it anyway).
fn range_loop_len_proof(src: &str, edit_start: usize) -> bool {
    use syn::spanned::Spanned;
    use syn::visit::Visit as _;
    /// Receiver-identity slot scan: counts `<coll>[<loop ident>]` positions
    /// and whether every one indexes the bound's ident.
    struct Slots<'a> {
        idx: &'a syn::Ident,
        bound: &'a str,
        n: usize,
        all_match: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for Slots<'ast> {
        fn visit_expr_index(&mut self, ix: &'ast syn::ExprIndex) {
            let bare = matches!(&*ix.index, syn::Expr::Path(p) if p.path.is_ident(self.idx));
            if bare {
                self.n += 1;
                if single_ident(&ix.expr).is_none_or(|b| b != self.bound) {
                    self.all_match = false;
                }
            }
            syn::visit::visit_expr_index(self, ix);
        }
    }
    struct InnermostFor<'a> {
        start: usize,
        /// (whole-loop span len, loop) — smallest containing loop wins.
        best: Option<(usize, &'a syn::ExprForLoop)>,
    }
    impl<'ast> syn::visit::Visit<'ast> for InnermostFor<'ast> {
        fn visit_expr(&mut self, e: &'ast syn::Expr) {
            if let syn::Expr::ForLoop(fl) = e {
                // The WHOLE loop span (pat + iterable + body): a suggestion's
                // first part may target the pat, which sits outside the
                // iterable's own span.
                let span = e.span().byte_range();
                if span.start <= self.start && self.start < span.end {
                    let smaller = self.best.is_none_or(|(len, _)| span.len() < len);
                    if smaller {
                        self.best = Some((span.len(), fl));
                    }
                }
            }
            syn::visit::visit_expr(self, e);
        }
    }
    let Ok(file) = syn::parse_file(src) else {
        return false;
    };
    let mut v = InnermostFor {
        start: edit_start,
        best: None,
    };
    v.visit_file(&file);
    let Some((_, fl)) = v.best else {
        return false;
    };
    // Plain-ident loop pat only (the proposer's posture).
    let syn::Pat::Ident(pi) = &*fl.pat else {
        return false;
    };
    if pi.by_ref.is_some() || pi.subpat.is_some() {
        return false;
    }
    let syn::Expr::Range(range) = &*fl.expr else {
        return false;
    };
    // Full-prefix iteration only: literal-0 start, half-open limits (a
    // `0..=len()` end iterates one EXTRA element — inclusive limits carry
    // no proof this lane accepts).
    let zero_start = matches!(
        range.start.as_deref(),
        Some(syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(li), .. }))
            if li.base10_digits() == "0"
    );
    let half_open = matches!(range.limits, syn::RangeLimits::HalfOpen(..));
    if !zero_start || !half_open {
        return false;
    }
    // The bound must be a zero-arg `.len()` on a single ident ...
    let Some(syn::Expr::MethodCall(mc)) = range.end.as_deref() else {
        return false;
    };
    if mc.method != "len" || !mc.args.is_empty() {
        return false;
    }
    let Some(bound) = single_ident(&mc.receiver) else {
        return false;
    };
    // ... AND it must name the collection it indexes: every body slot
    // `<coll>[<loop ident>]` (the positions a range-dropping rewrite
    // substitutes) indexes that same ident. A `len()` of the PARENT buffer
    // over a tail-slice slot (`0..buf.len() { gs[i] }`, `gs = &mut
    // buf[1..]`) passes the end-shape check and is exactly as unsound as
    // the plain-ident bound — the receiver identity is the proof.
    let mut slots = Slots {
        idx: &pi.ident,
        bound: &bound,
        n: 0,
        all_match: true,
    };
    slots.visit_block(&fl.body);
    slots.n >= 1 && slots.all_match
}

/// The single ident an expression is (a bare path), else `None` — the
/// receiver-identity seam of [`range_loop_len_proof`].
fn single_ident(e: &syn::Expr) -> Option<String> {
    if let syn::Expr::Path(p) = e {
        return p.path.get_ident().map(ToString::to_string);
    }
    None
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
        let code_json = code.map_or("null".to_string(), |c| format!(r#"{{"code": "{c}"}}"#));
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
                span: None,
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
            row(
                "error",
                Some("clippy::needless_return"),
                "src/b.rs",
                5,
                true
            ),
        );
        let got = parse_clippy_feed(&stream);
        // -W runs report lints as "warning"; -D runs as "error" — both count.
        assert_eq!(
            got,
            vec![ClippyDiagnostic {
                span: None,
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
    fn row_with_suggestion(
        file: &str,
        line: u64,
        lint_id: &str,
        parts: &[(u64, u64, &str, &str)],
    ) -> String {
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
        let bare = row(
            "warning",
            Some("clippy::unnecessary_map_or"),
            "src/lib.rs",
            11,
            true,
        );
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
        "fn f(items: &[u32]) -> bool {\n    items.iter().min().map_or(false, |x| *x > 1)\n}\n"
            .to_string()
    }

    fn mapor_feed(src: &str) -> Vec<ClippyDiagnostic> {
        let off = src.find("map_or").expect("fixture has the site");
        vec![ClippyDiagnostic {
            span: None,
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
            span: None,
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
        assert!(
            patched.contains("Runs the `shader_perf` stage"),
            "{patched}"
        );
        // The lane-set gate is the point: the SAME feed is refused when
        // the lint is not in the lane (the corpus-gap signal).
        let refused = oracle_edits(&feed, "src/lib.rs", src, &["collapsible_if"]);
        assert_eq!(refused.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
    }

    #[test]
    fn oracle_edits_merge_parts_with_gap_text_preserved() {
        let src = probe_src();
        let off = src.find("map_or").unwrap();
        let lane = oracle_edits(
            &mapor_feed(&src),
            "src/lib.rs",
            &src,
            &["unnecessary_map_or"],
        );
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
        assert_eq!(lane.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
        assert_eq!(lane.dropped, 0, "out-of-lane is not a drop, it is a skip");
    }

    #[test]
    fn oracle_edits_refuse_non_matching_files() {
        let src = probe_src();
        let lane = oracle_edits(
            &mapor_feed(&src),
            "other/mod.rs",
            &src,
            &["unnecessary_map_or"],
        );
        assert_eq!(lane.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
    }

    // ── P104: the oracle door's scope guard (the E0433 class) ──────────

    /// One diagnostic whose suggestion replaces `old` with `new` (both
    /// located by substring in `src`), under a lane lint the other
    /// guards do not key on.
    fn scope_feed(src: &str, old: &str, new: &str) -> Vec<ClippyDiagnostic> {
        let off = src.find(old).expect("marker");
        vec![ClippyDiagnostic {
            span: None,
            file: "src/lib.rs".to_string(),
            line: 1,
            lint: "unnecessary_map_or".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![(off, off + old.len(), new.to_string())],
            }),
        }]
    }

    #[test]
    fn p104_scope_index_direct() {
        let src = "use a::b::{c};\nmod inner {\n    use super::*;\n}\nstruct Local;\n";
        let idx = FileScopeIndex::build(src);
        assert!(idx.has_glob, "the glob must be seen");
        for ok in [
            "a::x",
            "b::c::d",
            "c::new",
            "Local::new",
            "std::mem::take",
            "crate::x::y",
            "super::x",
            "Vec::<u8>::new()",
        ] {
            assert!(idx.replacement_in_scope(ok), "{ok} must pass");
        }
        for bad in [
            "draft::corpus_index::ScoredRule",
            "traj_store::TrajNeighbor",
        ] {
            assert!(!idx.replacement_in_scope(bad), "{bad} must decline");
        }
        // String contents are text, not paths.
        assert!(idx.replacement_in_scope("let s = \"a::b::c\";"));
        // Line comments are skipped too.
        assert!(idx.replacement_in_scope("// draft::corpus_index::ScoredRule\nlet x = 1;"));
    }

    #[test]
    fn oracle_edits_decline_crate_root_path_under_glob() {
        // The P98 dogfood's E0433 shape: the suggestion spells the type
        // as a crate-root-relative path; the file's tests mod sees names
        // bare via `use super::*` — which covers the bare type, never the
        // module path. The patched file still PARSES (the whole-file gate
        // is syntax-only), so only the scope guard can catch it.
        let src = "mod inner {\n    use super::*;\n\n    fn check(\n        items: &[DraftPair],\n    ) -> bool {\n        items.iter().min().map_or(false, |x| *x > 1)\n    }\n}\n";
        let feed = scope_feed(
            src,
            "map_or(false, |x| *x > 1)",
            "is_some_and(|x: &draft::corpus_index::ScoredRule| *x > 1)",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["unnecessary_map_or"]);
        assert_eq!(lane.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
        assert_eq!(lane.path_scope_declined, 1, "the scope guard declines");
        assert_eq!(lane.dropped, 0, "a decline is not a drop");
    }

    #[test]
    fn oracle_edits_decline_unimported_root_without_glob() {
        // No glob in the file: an unexplained root declines regardless of
        // leading case.
        let src = "fn check(x: u32) -> bool {\n    x.map_or(false, |v| v > 1)\n}\n";
        let feed = scope_feed(
            src,
            "map_or(false, |v| v > 1)",
            "is_some_and(|v: &SomeType::Assoc| v > 1)",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["unnecessary_map_or"]);
        assert_eq!(lane.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
        assert_eq!(lane.path_scope_declined, 1);
    }

    #[test]
    fn oracle_edits_pass_imported_and_glob_plausible_roots() {
        // An imported prefix root passes...
        let src = "use draft::corpus_index;\n\nfn check(x: u32) -> bool {\n    x.map_or(false, |v| v > 1)\n}\n";
        let feed = scope_feed(
            src,
            "map_or(false, |v| v > 1)",
            "is_some_and(|v: &corpus_index::ScoredRule| v > 1)",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["unnecessary_map_or"]);
        assert_eq!(lane.edits.len(), 1, "imported root passes");
        assert_eq!(lane.path_scope_declined, 0);

        // ...and a glob-file UPPERCASE root passes (the P98 bare-name
        // lesson: `use super::*` covers type names).
        let src = "mod inner {\n    use super::*;\n\n    fn check(x: u32) -> bool {\n        x.map_or(false, |v| v > 1)\n    }\n}\n";
        let feed = scope_feed(
            src,
            "map_or(false, |v| v > 1)",
            "is_some_and(|v: &ScoredRule::Assoc| v > 1)",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["unnecessary_map_or"]);
        assert_eq!(lane.edits.len(), 1, "glob-plausible type root passes");
        assert_eq!(lane.path_scope_declined, 0);
    }

    // ── P34: the needless_range_loop length proof (riir-ai Issue 981) ──

    /// One `needless_range_loop` diagnostic whose suggestion rewrites the
    /// loop header `header_old` → `header_new` plus one index slot
    /// `slot_old` → `slot_new` (both located by substring).
    fn range_loop_feed(
        src: &str,
        header_old: &str,
        header_new: &str,
        slot_old: &str,
        slot_new: &str,
    ) -> Vec<ClippyDiagnostic> {
        let h = src.find(header_old).expect("header marker");
        let s = src.find(slot_old).expect("slot marker");
        vec![ClippyDiagnostic {
            span: None,
            file: "src/lib.rs".to_string(),
            line: 1,
            lint: "needless_range_loop".to_string(),
            suggestion: Some(FeedSuggestion {
                parts: vec![
                    (h, h + header_old.len(), header_new.to_string()),
                    (s, s + slot_old.len(), slot_new.to_string()),
                ],
            }),
        }]
    }

    #[test]
    fn oracle_edits_decline_doc_markdown_on_command_lines() {
        // Issue 135 T4: clippy's own suggestion for the reflex-site line is
        // the argv-token backtick — declined at the oracle door too, while
        // the same lint on a prose line is still consumed.
        let src = "//! Regen the shader_perf corpus.\n//!   cargo run --bin gen_corpus\npub fn one() {}\n";
        let diag = |word: &str| {
            let at = src.find(word).expect("marker");
            ClippyDiagnostic {
                span: Some((at, at + word.len())),
                file: "src/lib.rs".to_string(),
                line: 1,
                lint: "doc_markdown".to_string(),
                suggestion: Some(FeedSuggestion {
                    parts: vec![(at, at + word.len(), format!("`{word}`"))],
                }),
            }
        };
        let feed = vec![diag("shader_perf"), diag("gen_corpus")];
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["doc_markdown"]);
        assert_eq!(lane.doc_command_declined, 1);
        assert_eq!(lane.dropped, 0, "a decline is not staleness");
        assert_eq!(lane.edits.len(), 1);
        assert_eq!(lane.edits[0].edit.replacement, "`shader_perf`");
    }

    #[test]
    fn oracle_edits_decline_range_loop_with_computed_bound() {
        // The 981 shape: `for i in 0..sph { gs[i] *= d }` over
        // `let gs = &mut buf[h * sph..]` — a tail slice whose len > sph.
        // Whether clippy proved sph == gs.len() by cross-binding analysis
        // is untrustable here; the rewrite multiplied every later head's
        // slab by this head's decay and was misread as a GPU kernel bug
        // for a forensic session. Declined, loudly counted, never applied.
        let src = "fn decay(buf: &mut [f32], h: usize, sph: usize) {\n    let gs = &mut buf[h * sph..];\n    for i in 0..sph {\n        gs[i] *= 0.5;\n    }\n}\n";
        let feed = range_loop_feed(src, "i in 0..sph", "g in gs.iter_mut()", "gs[i]", "*g");
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["needless_range_loop"]);
        assert!(lane.edits.is_empty(), "981 shape must not be applied");
        assert_eq!(lane.range_declined, 1);
        assert_eq!(
            lane.dropped, 0,
            "a decline is a deliberate refusal, not staleness"
        );
    }

    #[test]
    fn oracle_edits_consume_range_loop_with_len_end() {
        // The syntactic proof: the bound IS the iterated collection's
        // `len()` — full-prefix iteration is identical by construction.
        let src = "fn sum(v: Vec<i32>) -> i32 {\n    let mut t = 0;\n    for i in 0..v.len() {\n        t += v[i];\n    }\n    t\n}\n";
        let feed = range_loop_feed(src, "i in 0..v.len()", "x in &v", "v[i]", "*x");
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["needless_range_loop"]);
        assert_eq!(lane.range_declined, 0);
        assert_eq!(lane.edits.len(), 1, "len-proven rewrite must be consumed");
        let patched = splice_sorted(src, std::slice::from_ref(&lane.edits[0].edit));
        assert!(patched.contains("for x in &v"), "{patched}");
        syn::parse_file(&patched).expect("parses");
    }

    #[test]
    fn oracle_edits_decline_parent_len_and_inclusive_bounds() {
        // (a) the bound is a len() of the PARENT buffer — `grad_s.len() /
        // heads` is not the tail slice's len for any offset head;
        // (b) `0..=n` — even a len() end there iterates one EXTRA element
        // (n+1 vs len): inclusive limits carry no proof this lane accepts.
        let parent = "fn f(grad_s: &mut [f32], h: usize, heads: usize) {\n    let sph = grad_s.len() / heads;\n    let gs = &mut grad_s[h * sph..];\n    for i in 0..(grad_s.len() / heads) {\n        gs[i] *= 0.5;\n    }\n}\n";
        let feed = range_loop_feed(
            parent,
            "i in 0..(grad_s.len() / heads)",
            "g in gs.iter_mut()",
            "gs[i]",
            "*g",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", parent, &["needless_range_loop"]);
        assert!(lane.edits.is_empty(), "parent-len bound must decline");
        assert_eq!(lane.range_declined, 1);

        let incl = "fn s(v: &[i32]) -> i32 {\n    let mut t = 0;\n    for i in 0..=v.len() {\n        t += v[i];\n    }\n    t\n}\n";
        let feed = range_loop_feed(incl, "i in 0..=v.len()", "x in v", "v[i]", "*x");
        let lane = oracle_edits(&feed, "src/lib.rs", incl, &["needless_range_loop"]);
        assert!(lane.edits.is_empty(), "inclusive limit must decline");
        assert_eq!(lane.range_declined, 1);
    }

    #[test]
    fn oracle_edits_decline_len_bound_over_different_collection() {
        // The receiver-identity case: the bound IS a zero-arg `.len()` (the
        // end-shape check alone passes) but of the PARENT buffer, while the
        // body slots index `gs`, a tail slice — `gs.len() < buf.len()` and
        // the suggestion iterates gs. The 981 hazard wearing a len() bound;
        // the receiver identity is the proof the end-shape cannot supply.
        let src = "fn decay(buf: &mut [f32], base: usize, sph: usize) {\n    let gs = &mut buf[base..];\n    for i in 0..buf.len() {\n        gs[i] *= 0.5;\n    }\n}\n";
        let feed = range_loop_feed(
            src,
            "i in 0..buf.len()",
            "g in gs.iter_mut()",
            "gs[i]",
            "*g",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["needless_range_loop"]);
        assert!(
            lane.edits.is_empty(),
            "len() of the parent over a tail-slice slot must decline"
        );
        assert_eq!(lane.range_declined, 1);
    }

    #[test]
    fn oracle_edits_range_guard_judges_the_innermost_loop() {
        // A suggestion inside a nested loop is judged by the INNER header:
        // the outer loop carries the len proof, the inner does not — the
        // guard must not be satisfied by the enclosing loop's proof.
        let src = "fn f(v: Vec<Vec<i32>>, n: usize) -> i32 {\n    let mut t = 0;\n    for r in 0..v.len() {\n        for i in 0..n {\n            t += v[r][i];\n        }\n    }\n    t\n}\n";
        let feed = range_loop_feed(
            src,
            "i in 0..n",
            "(r, row) in v.iter().enumerate()",
            "v[r][i]",
            "row[i]",
        );
        let lane = oracle_edits(&feed, "src/lib.rs", src, &["needless_range_loop"]);
        assert!(lane.edits.is_empty(), "inner unproven bound must decline");
        assert_eq!(lane.range_declined, 1);
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
        assert_eq!(lane.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
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
        assert_eq!(lane.edits, [] as [crate::edit_emitter::RuleEdit; 0]);
        assert_eq!(lane.dropped, 1, "gate refusal counts the candidate");
    }

    #[test]
    fn oracle_edits_leftmost_wins_on_self_overlap() {
        let src = probe_src();
        let off = src.find("map_or").unwrap();
        let mut feed = mapor_feed(&src);
        // A second diagnostic whose suggestion overlaps the first's span.
        feed.push(ClippyDiagnostic {
            span: None,
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
            span: None,
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
            span: None,
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
            span: None,
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

    // ── Plan 165: the repo-posture gate primitives ────────────────────

    /// A `row` variant whose primary span carries byte offsets — the real
    /// cargo JSON shape (`byte_start`/`byte_end` are always present on
    /// file-bearing spans; the base `row` omits them to pin the defensive
    /// `span: None` decode).
    fn row_bytes(
        level: &str,
        code: &str,
        file: &str,
        line: u64,
        byte_start: u64,
        byte_end: u64,
    ) -> String {
        format!(
            r#"{{"reason": "compiler-message", "message": {{"level": "{level}", "code": {{"code": "{code}"}}, "spans": [{{"file_name": "{file}", "line_start": {line}, "byte_start": {byte_start}, "byte_end": {byte_end}, "is_primary": true}}]}}}}"#
        )
    }

    #[test]
    fn parse_retains_primary_span_bytes() {
        let stream = format!(
            "{}\n{}\n",
            row_bytes(
                "warning",
                "clippy::manual_range_contains",
                "src/a.rs",
                7,
                120,
                145
            ),
            row("warning", Some("clippy::doc_markdown"), "src/a.rs", 9, true)
        );
        let got = parse_clippy_feed(&stream);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].span, Some((120, 145)));
        // A span row without byte offsets (defensive — never real cargo
        // JSON) decodes None: not posture-matchable, never a guessed span.
        assert_eq!(got[1].span, None);
    }

    #[test]
    fn posture_spans_partitions_by_lint_and_file() {
        let feed = vec![
            ClippyDiagnostic {
                span: Some((10, 30)),
                file: "src/lib.rs".to_string(),
                line: 2,
                lint: "manual_range_contains".to_string(),
                suggestion: None,
            },
            ClippyDiagnostic {
                span: Some((40, 60)),
                file: "src/lib.rs".to_string(),
                line: 4,
                lint: "manual_range_contains".to_string(),
                suggestion: None,
            },
            ClippyDiagnostic {
                span: Some((70, 90)),
                file: "src/other.rs".to_string(),
                line: 6,
                lint: "manual_range_contains".to_string(),
                suggestion: None,
            },
            ClippyDiagnostic {
                span: None,
                file: "src/lib.rs".to_string(),
                line: 8,
                lint: "doc_markdown".to_string(),
                suggestion: None,
            },
        ];
        // Path matching is the paths_ref_match tail rule, not equality.
        let idx = posture_spans(&feed, "crate/src/lib.rs");
        assert_eq!(
            idx.get("manual_range_contains").map(Vec::as_slice),
            Some(&[(10usize, 30usize), (40, 60)][..])
        );
        // Span-less diagnostics and other files contribute nothing.
        assert!(!idx.contains_key("doc_markdown"));
        let other = posture_spans(&feed, "src/other.rs");
        assert_eq!(
            other.get("manual_range_contains").map(Vec::as_slice),
            Some(&[(70usize, 90usize)][..])
        );
    }

    #[test]
    fn posture_allows_requires_same_lint_overlap() {
        let mut idx = std::collections::BTreeMap::new();
        idx.insert(
            "manual_range_contains".to_string(),
            vec![(10usize, 30usize)],
        );
        // Same lint, overlapping (contained / containing / partial).
        assert!(posture_allows(&idx, "manual_range_contains", 12, 18));
        assert!(posture_allows(&idx, "manual_range_contains", 8, 40));
        assert!(posture_allows(&idx, "manual_range_contains", 25, 35));
        // Zero-width insertion strictly inside passes; at a boundary not.
        assert!(posture_allows(&idx, "manual_range_contains", 15, 15));
        assert!(!posture_allows(&idx, "manual_range_contains", 30, 30));
        // Same lint, disjoint span — the target's gate never flagged here.
        assert!(!posture_allows(&idx, "manual_range_contains", 31, 50));
        // Different lint over the same bytes — posture is per-lint.
        assert!(!posture_allows(&idx, "doc_markdown", 12, 18));
        // Lint absent from the index entirely.
        assert!(!posture_allows(&idx, "needless_return", 12, 18));
    }
}

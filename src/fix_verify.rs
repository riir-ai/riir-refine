//! `--fix --verify` — the compile gate as tooling (Issue 015).
//!
//! The Bench 019 §G5 real-file run proved the manual compile gate is the
//! real safety net for parse-only proposers: run 1 wrote 30 edits, 6 errors
//! in 4 files; the proposers were hardened at root cause; run 2 landed
//! clean. But the hardened gates are deliberately CONSERVATIVE because
//! `--fix` is parse-only — the usize gate skips plain-path capacity bounds
//! (consts, associated consts, locals) even when they are `usize` in
//! context, and `redundant_closure` stays demoted because fn-path rewrites
//! lose call-site coercions.
//!
//! This module closes the loop IN the tool: apply → `cargo check
//! --all-targets` → map rustc's JSON errors back to files + lines →
//! revert ONLY the offending edits (line-matched span-level; whole-file as
//! the fallback) → re-check → report. With rustc backing the revert, the
//! parse-only gates can be loosened under `--verify` (plain-path capacity
//! bounds attempted; `redundant_closure` restored behind
//! [`crate::domains::clippy_lints::proposers::VERIFY_LINTS`]).
//!
//! This is the `rust_perf_verify` / `clippy_verify` pattern (already
//! shipped, opt-in) applied to the EDIT path instead of the draft path.
//!
//! ## The verify loop
//!
//! 1. **Baseline** — `cargo check` on the UNMODIFIED tree. If it already
//!    fails, abort + write nothing (a broken tree cannot host a
//!    verification; errors would be unattributable).
//! 2. **Write** — splice + write every fixed file.
//! 3. **Check** — `cargo check --all-targets --message-format=json`
//!    (`--all-targets` because edit sites live in `#[cfg(test)]` code too
//!    — `collapse_detector`'s `N_TRACES` const, caught by the G5 run;
//!    plain `cargo check` would "verify" them without compiling them).
//! 4. **Attribute** — parse error-level diagnostics → `(file, line-range)`
//!    set. For each edited file with error lines:
//!    - edits whose final-source line falls inside an error span are the
//!      suspects → **span-level revert**: the edit's replacement text is
//!      located on the error line (validated byte-exact against the
//!      recorded replacement; any mismatch degrades to whole-file), and
//!      the recorded original text is spliced back;
//!    - no edit on the error lines (e.g. a signature change breaking a
//!      caller elsewhere in the same file) → **whole-file revert**.
//! 5. **Cascade rule** — if errors point ONLY at files we did not edit,
//!    the culprit is an edit's semantic ripple (a changed signature used
//!    across files) and cannot be attributed: revert ALL edited files
//!    whole. Conservative by design — the tool's promise is "kept edits
//!    compile", not "kept edits are maximally many".
//! 6. Loop (bounded by the edit count) until a clean check or everything
//!    is reverted.
//!
//! ## Failure-mode honesty
//!
//! - Interrupted mid-verify (Ctrl-C): the tree holds exactly what
//!   `--fix --write` would have produced — never worse than the
//!   non-verify path, but unverified.
//! - `cargo check` fails to RUN (bad `--verify-args`, broken manifest):
//!   all edits are reverted + the error reported — unverified writes are
//!   never left behind by a failed verification.
//! - Warnings do not fail the check (no `-D warnings`): verification
//!   proves the edits COMPILE, not that they are warning-free.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// Plan 019 T0.3: the subprocess + path-walk substrate moved to
// `compile_oracle` so the `rustc_errors` driver can consume it without the
// `latent_retrieval` tree. Re-exported here for back-compat — the bin and
// the tests reach them through `fix_verify::`.
pub use crate::compile_oracle::{
    owning_package_root, resolve_reported_path, workspace_root, CheckRunner,
};

/// One edit, positioned for revert: its final-source line, its replacement
/// text (to locate it on the error line), and the original text to restore.
#[derive(Debug, Clone)]
pub struct EditAtLine {
    pub rule_id: String,
    /// 1-based line of the edit in the FINAL (fixed) source.
    pub final_line: usize,
    /// The spliced-in replacement text (used to locate + validate the
    /// span before reverting).
    pub replacement: String,
    /// The original source text of the replaced span (put back on revert).
    pub restore: String,
}

/// One file's fix, ready for verification: the original text, the fixed
/// text, and the positioned edit records.
#[derive(Debug, Clone)]
pub struct FileEditSet {
    pub path: PathBuf,
    pub original: String,
    pub fixed: String,
    pub edits: Vec<EditAtLine>,
}

/// A revert that happened, for the report.
#[derive(Debug, Clone)]
pub struct RevertRecord {
    pub path: PathBuf,
    pub rule_id: String,
    pub line: usize,
    /// `true` when the whole file went back to its original (either no
    /// edit matched the error lines, or the span-level validation failed).
    pub whole_file: bool,
}

/// The outcome of a [`FixVerifier::verify`] run.
#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    /// Whether the cargo loop ran at all (false: nothing to verify).
    pub ran: bool,
    /// Whether the baseline (pre-edit) check passed. `false` → nothing
    /// was written; the tree is untouched.
    pub baseline_clean: bool,
    /// Files whose edits (all or part) survived.
    pub kept_files: usize,
    /// Edits that survived.
    pub kept_edits: usize,
    /// Edits reverted, with granularity.
    pub reverted: Vec<RevertRecord>,
    /// cargo check invocations (baseline + rounds).
    pub n_checks: usize,
    /// Why verification could not run (cargo missing, spawn failure...).
    pub error: Option<String>,
}

/// One cargo check result: the JSON-lines stdout + the exit status.
struct CheckOutcome {
    json: String,
    exit_ok: bool,
}

/// Verifier applying the compile gate to a batch of file edits.
///
/// ## Performance
///
/// Each round spawns `cargo check --all-targets` per owning package —
/// seconds, not µs (the first round is the slow one; reverts re-use
/// cargo's incremental cache). This is a landing-time tool, not a hot
/// path.
pub struct FixVerifier {
    runner: CheckRunner,
}

impl Default for FixVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl FixVerifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            runner: CheckRunner::new(),
        }
    }

    /// Append extra cargo args to every check (feature sets etc.).
    #[must_use]
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.runner = self.runner.with_extra_args(args);
        self
    }

    /// Run the verify loop over `files`. On return, disk holds: the fixed
    /// content for files whose edits survived + the original content for
    /// reverted files. See the module docs for the algorithm.
    #[must_use]
    pub fn verify(&self, files: &[FileEditSet]) -> VerifyReport {
        let mut report = VerifyReport::default();
        let touched: Vec<&FileEditSet> = files.iter().filter(|f| f.fixed != f.original).collect();
        if touched.is_empty() {
            return report; // nothing to verify — trivially fine, `ran` stays false
        }
        report.ran = true;

        // Resolve every touched path to its real (canonical) form so
        // rustc's reported file names compare equal after the same
        // treatment below.
        let canon: Vec<PathBuf> = touched
            .iter()
            .map(|f| std::fs::canonicalize(&f.path).unwrap_or_else(|_| f.path.clone()))
            .collect();

        // Group by owning package so each check compiles the crate the
        // edits live in (cross-file effects inside the package are caught
        // by the whole-package check).
        let mut by_pkg: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        for (idx, f) in touched.iter().enumerate() {
            let Some(pkg) = owning_package_root(&f.path) else {
                report.error = Some(format!("no Cargo.toml above {}", f.path.display()));
                return report;
            };
            by_pkg.entry(pkg).or_default().push(idx);
        }
        let mut pkgs: Vec<PathBuf> = by_pkg.keys().cloned().collect();
        pkgs.sort();

        // 1. Baseline on the unmodified tree (nothing has been written
        // yet — a broken tree cannot host a verification).
        if let Err(msg) = self.run_baseline(&pkgs, &mut report) {
            report.error = Some(msg);
            return report;
        }
        if !report.baseline_clean {
            report.error = Some(
                "baseline cargo check failed — tree already broken, nothing written".to_string(),
            );
            return report;
        }

        // 2. Write every fixed file.
        for f in &touched {
            if let Err(e) = std::fs::write(&f.path, &f.fixed) {
                report.error = Some(format!("write failed for {}: {e}", f.path.display()));
                restore_all(&touched);
                return report;
            }
        }

        // 3-6. The loop.
        let max_rounds: usize = touched.iter().map(|f| f.edits.len()).sum::<usize>() + 2;
        let mut state = LoopState {
            current: touched.iter().map(|f| f.fixed.clone()).collect(),
            applied: touched.iter().map(|f| vec![true; f.edits.len()]).collect(),
        };
        for _round in 0..max_rounds {
            let mut error_lines: HashMap<PathBuf, Vec<(usize, usize)>> = HashMap::new();
            let mut run_err: Option<String> = None;
            for pkg in &pkgs {
                match self.run_check(pkg) {
                    Ok(outcome) => {
                        report.n_checks += 1;
                        if outcome.exit_ok && parse_error_locations(&outcome.json).is_empty() {
                            continue; // this package is clean
                        }
                        merge_error_locations(
                            &outcome.json,
                            workspace_root(pkg).as_deref(),
                            pkg,
                            &mut error_lines,
                        );
                    }
                    Err(e) => run_err = Some(e),
                }
            }
            if let Some(e) = run_err {
                // cargo could not run — restore everything, leave nothing
                // unverified behind.
                report.error = Some(e);
                restore_all(&touched);
                return report;
            }

            if error_lines.is_empty() {
                break; // clean everywhere — everything still applied survives
            }
            let step = attribute_and_revert(&touched, &canon, &error_lines, &mut state, &mut report);
            match step {
                RevertStep::Progress => {}
                RevertStep::RestoreAll(reason) => {
                    restore_all_state(&touched, &mut state);
                    for f in &touched {
                        let _ = std::fs::write(&f.path, &f.original);
                        report.reverted.push(RevertRecord {
                            path: f.path.clone(),
                            rule_id: reason.to_string(),
                            line: 0,
                            whole_file: true,
                        });
                    }
                    break;
                }
            }
        }

        // Tally.
        for (idx, _f) in touched.iter().enumerate() {
            if state.applied[idx].iter().any(|a| *a) {
                report.kept_files += 1;
                report.kept_edits += state.applied[idx].iter().filter(|a| **a).count();
            }
        }
        report
    }

    /// Step 1 of the loop: the baseline check over every owning package.
    /// `Err` = cargo could not run; `Ok` with `report.baseline_clean ==
    /// false` = the tree already fails to compile (the error message is
    /// set on the report).
    fn run_baseline(&self, pkgs: &[PathBuf], report: &mut VerifyReport) -> Result<(), String> {
        for pkg in pkgs {
            let outcome = self.run_check(pkg)?;
            report.n_checks += 1;
            let errors = parse_error_locations(&outcome.json);
            if !outcome.exit_ok || !errors.is_empty() {
                report.baseline_clean = false;
                return Ok(()); // message set by the caller from the flag
            }
        }
        report.baseline_clean = true;
        Ok(())
    }

    /// One `cargo check --all-targets --message-format=json` run in `pkg`,
    /// delegated to the shared [`CheckRunner`] (Plan 019 T0.3). Fails (Err)
    /// only when cargo could not run at all — compile errors come back as
    /// `Ok(outcome)` with `exit_ok = false` + error diagnostics in the JSON.
    fn run_check(&self, pkg: &Path) -> Result<CheckOutcome, String> {
        let outcome = self.runner.run_check(pkg)?;
        Ok(CheckOutcome {
            json: outcome.json,
            exit_ok: outcome.exit_ok,
        })
    }
}

/// Mutable per-file state across verify rounds.
struct LoopState {
    current: Vec<String>,
    applied: Vec<Vec<bool>>,
}

/// One revert-round outcome: progress, or a terminal restore-all.
enum RevertStep {
    Progress,
    RestoreAll(&'static str),
}

/// Steps 4-5 of the loop: attribute error lines onto edits + revert the
/// suspects (span-level where validatable, whole-file otherwise). Writes
/// modified files back; appends to `report.reverted`.
fn attribute_and_revert(
    touched: &[&FileEditSet],
    canon: &[PathBuf],
    error_lines: &HashMap<PathBuf, Vec<(usize, usize)>>,
    state: &mut LoopState,
    report: &mut VerifyReport,
) -> RevertStep {
    let mut attributed_any = false;
    let mut reverted_this_round = false;
    for (idx, f) in touched.iter().enumerate() {
        let lines = match error_lines.get(&canon[idx]) {
            Some(l) => l.clone(),
            None => continue,
        };
        attributed_any = true;
        // Suspects: edits whose final line falls inside an error span.
        let suspects: Vec<usize> = f
            .edits
            .iter()
            .enumerate()
            .filter(|(i, e)| state.applied[idx][*i] && line_in_ranges(e.final_line, &lines))
            .map(|(i, _)| i)
            .collect();
        if suspects.is_empty() {
            // Error in our file but not on an edit line — the
            // semantic-ripple case. Whole-file revert.
            revert_whole_file(&mut state.current[idx], &mut state.applied[idx], f);
            report.reverted.push(RevertRecord {
                path: f.path.clone(),
                rule_id: "(whole file)".to_string(),
                line: lines.first().map_or(0, |r| r.0),
                whole_file: true,
            });
            reverted_this_round = true;
        } else {
            for i in suspects {
                let span_ok =
                    revert_single_edit(&mut state.current[idx], &mut state.applied[idx], f, i);
                report.reverted.push(RevertRecord {
                    path: f.path.clone(),
                    rule_id: f.edits[i].rule_id.clone(),
                    line: f.edits[i].final_line,
                    whole_file: !span_ok,
                });
                if !span_ok {
                    // Span validation failed — degrade to whole-file.
                    revert_whole_file(&mut state.current[idx], &mut state.applied[idx], f);
                }
                reverted_this_round = true;
            }
        }
        // Write the modified text back before the next check.
        let _ = std::fs::write(&f.path, &state.current[idx]);
    }

    if !attributed_any {
        // Cascade: errors only in files we did not edit — the culprit is
        // a semantic ripple we cannot attribute.
        return RevertStep::RestoreAll("(cascade)");
    }
    if !reverted_this_round {
        // Errors persist but nothing was revertible (progress guarantee:
        // the loop always ends via this arm).
        return RevertStep::RestoreAll("(unattributed)");
    }
    RevertStep::Progress
}

/// Restore every touched file to its original (catastrophic-failure path —
/// never leave unverified writes behind).
fn restore_all(touched: &[&FileEditSet]) {
    for f in touched {
        let _ = std::fs::write(&f.path, &f.original);
    }
}

/// Restore the loop state + disk for every file still holding edits.
fn restore_all_state(touched: &[&FileEditSet], state: &mut LoopState) {
    for (idx, f) in touched.iter().enumerate() {
        revert_whole_file(&mut state.current[idx], &mut state.applied[idx], f);
    }
}

/// Is 1-based `line` inside any `(start, end)` inclusive range?
fn line_in_ranges(line: usize, ranges: &[(usize, usize)]) -> bool {
    ranges.iter().any(|(s, e)| line >= *s && line <= *e)
}

/// Revert every still-applied edit in the file (back to the original).
fn revert_whole_file(current: &mut String, applied: &mut [bool], f: &FileEditSet) {
    current.clone_from(&f.original);
    applied.fill(false);
}

/// Revert ONE edit: locate its replacement on the edit's final line in
/// `current`, validate byte-exact, splice the original text back.
/// Returns `false` when the span cannot be validated (the caller degrades
/// to whole-file). Multi-line replacements are matched by their first
/// line, then the full replacement length is validated.
fn revert_single_edit(current: &mut String, applied: &mut [bool], f: &FileEditSet, i: usize) -> bool {
    let e = &f.edits[i];
    let Some(span_start) = find_replacement_on_line(current, e.final_line, &e.replacement) else {
        return false;
    };
    let span_end = span_start + e.replacement.len();
    if current.get(span_start..span_end) != Some(e.replacement.as_str()) {
        return false; // text drifted — not safe to surgically revert
    }
    current.replace_range(span_start..span_end, &e.restore);
    applied[i] = false;
    true
}

/// Byte offset where `replacement` starts on 1-based `line` of `text`
/// (matched by the replacement's first line — the search is bounded to
/// THE line, but the full replacement may extend past its newline).
/// `None` when the line or the text cannot be found.
fn find_replacement_on_line(text: &str, line: usize, replacement: &str) -> Option<usize> {
    if line == 0 {
        return None;
    }
    let mut seen = 0usize;
    let mut line_start = 0usize;
    if line > 1 {
        for (n, ch) in text.char_indices() {
            if ch == '\n' {
                seen += 1;
                if seen == line - 1 {
                    line_start = n + 1;
                    break;
                }
            }
        }
        if seen < line - 1 {
            return None; // fewer lines than that
        }
    }
    let line_end = text[line_start..]
        .find('\n')
        .map_or(text.len(), |n| line_start + n);
    let line_text = &text[line_start..line_end];
    let first_repl_line = replacement.lines().next().unwrap_or("");
    if first_repl_line.is_empty() {
        return None;
    }
    let col = line_text.find(first_repl_line)?;
    Some(line_start + col)
}

// Walk up from a package root to the workspace root is now
// `crate::compile_oracle::workspace_root`; the manifest walk lives
// beside it (Plan 019 T0.3).

// ── cheap manifest scans (verify-specific) ───────────────────────────────
/// Cheap text scan: does this manifest text reference the dependency
/// `dep`? Matches `dep = ...` / `dep = { ... }` under any dependencies
/// table — precise enough for a hard-skip gate (the rewrite cannot
/// compile without the dep, so a false negative only skips an edit; a
/// false positive lets cargo catch it).
#[must_use]
pub fn manifest_has_dep(manifest_text: &str, dep: &str) -> bool {
    let table_entry = format!("{dep} =");
    manifest_text
        .lines()
        .any(|l| l.trim_start().starts_with(&table_entry))
        || manifest_text.contains(&format!("\"{dep}\""))
}

/// Issue 031 H3: count the package's non-default declared features (cheap
/// text scan of the `[features]` table — keys not covered by `default`'s
/// dependency list). The `--verify` baseline runs default-features
/// `cargo check`, so code behind these features is NOT compile-gated; the
/// caller warns + suggests `--verify-args "--all-features"`. A misparse
/// yields 0 (silent) or a spurious warning — never a wrong gate.
#[must_use]
pub fn manifest_nondefault_feature_count(manifest_text: &str) -> usize {
    let mut in_features = false;
    let mut default_deps: Vec<String> = Vec::new();
    let mut keys: Vec<&str> = Vec::new();
    // `default = [\n "a",\n "b",\n]` spans lines — accumulate the value
    // until the next key or section.
    let mut pending_default: Option<String> = None;
    let finalize_default = |pending: &mut Option<String>, out: &mut Vec<String>| {
        if let Some(value) = pending.take() {
            for part in value.trim_matches(['[', ']']).split(',') {
                let p = part.trim().trim_matches('"');
                if !p.is_empty() {
                    out.push(p.to_string());
                }
            }
        }
    };
    for line in manifest_text.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_features = t == "[features]";
            finalize_default(&mut pending_default, &mut default_deps);
            continue;
        }
        if !in_features {
            continue;
        }
        if let Some(eq) = t.find('=') {
            finalize_default(&mut pending_default, &mut default_deps);
            let key = t[..eq].trim().trim_matches('"');
            let value = t[eq + 1..].trim();
            if key == "default" {
                pending_default = Some(value.to_string());
            } else {
                keys.push(key);
            }
        } else if let Some(pd) = pending_default.as_mut() {
            pd.push(' ');
            pd.push_str(t);
        }
    }
    finalize_default(&mut pending_default, &mut default_deps);
    keys.into_iter()
        .filter(|k| !default_deps.iter().any(|d| d == k))
        .count()
}

// ── rustc JSON parsing ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CargoMessage {
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    message: Option<Diagnostic>,
}

#[derive(Deserialize)]
struct Diagnostic {
    #[serde(default)]
    level: String,
    #[serde(default)]
    code: Option<DiagCode>,
    #[serde(default)]
    spans: Vec<DiagSpan>,
    #[serde(default)]
    children: Vec<Diagnostic>,
}

#[derive(Deserialize)]
struct DiagCode {
    code: String,
}

#[derive(Deserialize)]
struct DiagSpan {
    file_name: String,
    line_start: usize,
    line_end: usize,
    #[serde(default)]
    primary: bool,
}

/// One error location: a file plus the inclusive line ranges its error
/// spans cover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocation {
    pub file: PathBuf,
    pub ranges: Vec<(usize, usize)>,
}

/// Parse cargo's JSON-lines stdout into per-file error line ranges.
///
/// - `reason == "compiler-message"` with `level` starting `error` → the
///   diagnostic's own spans AND its children's spans (notes often point
///   at the binding site — attribution signal, conservatively included).
/// - Warnings are ignored (they do not fail the build; verification
///   proves compilation, not warning-freedom).
/// - Non-JSON lines are skipped.
#[must_use]
pub fn parse_error_locations(json_stdout: &str) -> Vec<ErrorLocation> {
    let mut map: HashMap<PathBuf, Vec<(usize, usize)>> = HashMap::new();
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
        collect_diag_spans(&diag, &mut map);
    }
    let mut out: Vec<ErrorLocation> = map
        .into_iter()
        .map(|(file, ranges)| ErrorLocation { file, ranges })
        .collect();
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out
}

fn collect_diag_spans(diag: &Diagnostic, map: &mut HashMap<PathBuf, Vec<(usize, usize)>>) {
    for s in &diag.spans {
        let entry = map.entry(PathBuf::from(&s.file_name)).or_default();
        let r = (s.line_start, s.line_end.max(s.line_start));
        if !entry.contains(&r) {
            entry.push(r);
        }
    }
    for child in &diag.children {
        collect_diag_spans(child, map);
    }
}

/// Merge a check round's JSON into the running `(abs path → ranges)` map,
/// resolving rustc's workspace-relative `file_name`s against the package's
/// workspace root (rustc runs with cwd = workspace root, so relative
/// names are workspace-relative).
fn merge_error_locations(
    json: &str,
    ws_root: Option<&Path>,
    pkg: &Path,
    out: &mut HashMap<PathBuf, Vec<(usize, usize)>>,
) {
    let ws = ws_root.map_or_else(|| pkg.to_path_buf(), Path::to_path_buf);
    for loc in parse_error_locations(json) {
        let resolved = resolve_reported_path(&loc.file, &ws, pkg);
        let entry = out.entry(resolved).or_default();
        for r in loc.ranges {
            if !entry.contains(&r) {
                entry.push(r);
            }
        }
    }
}

// `resolve_reported_path` moved to `compile_oracle` (re-exported above)
// — Plan 019 T0.3. The doc on it there describes the resolution order.

// ── 037-T2: the lint-resolution oracle helpers ────────────────────────────

/// One clippy finding: the lint code (e.g. `clippy::uninlined_format_args`)
/// plus the file + inclusive line range of its PRIMARY span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClippyFinding {
    pub code: String,
    pub file: PathBuf,
    pub line_start: usize,
    pub line_end: usize,
}

/// Parse `cargo clippy --message-format=json` stdout into findings:
/// warning-level `clippy::*` diagnostics, PRIMARY spans only (notes/children
/// point at binding sites elsewhere — not resolution signal). This is the
/// 037-T2 resolution oracle's read side: a kept edit's named lint resolved
/// iff no finding of that lint covers the edit's final line.
#[must_use]
pub fn parse_clippy_findings(json_stdout: &str) -> Vec<ClippyFinding> {
    let mut out = Vec::new();
    for line in json_stdout.lines() {
        let Ok(msg) = serde_json::from_str::<CargoMessage>(line.trim()) else {
            continue;
        };
        if msg.reason.as_deref() != Some("compiler-message") {
            continue;
        }
        let Some(diag) = msg.message else { continue };
        if diag.level != "warning" {
            continue;
        }
        let Some(code) = diag.code.as_ref().map(|c| c.code.clone()) else {
            continue;
        };
        if !code.starts_with("clippy::") {
            continue;
        }
        for s in &diag.spans {
            if !s.primary {
                continue;
            }
            out.push(ClippyFinding {
                code: code.clone(),
                file: PathBuf::from(&s.file_name),
                line_start: s.line_start,
                line_end: s.line_end.max(s.line_start),
            });
        }
    }
    out.shrink_to_fit();
    out
}

/// The 037-T2 verdict matrix for a kept edit (the compile gate kept it —
/// the code compiles):
///
/// - `Some(true)` — the clippy oracle confirms the named lint resolved at
///   the edit site → `(success, verified) = (true, true)` → **Certified**
///   (the only promotion path, Issue 021).
/// - `Some(false)` — oracle ran + the lint PERSISTS at the edit site →
///   `(false, true)` → a **verified failure** (the honest persist signal;
///   three consecutive verified failures withdraw the trajectory).
/// - `None` — no oracle verdict available (oracle not built, cargo
///   unrunnable, or a non-clippy rule the oracle cannot judge) →
///   `(true, false)` → **Heuristic outcome-success** (the rustc-only
///   pass — exactly 037-T2's split).
#[must_use]
pub fn kept_edit_verdict(oracle_resolved: Option<bool>) -> (bool, bool) {
    match oracle_resolved {
        Some(true) => (true, true),
        Some(false) => (false, true),
        None => (true, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(rule: &str, line: usize, replacement: &str, restore: &str) -> EditAtLine {
        EditAtLine {
            rule_id: rule.to_string(),
            final_line: line,
            replacement: replacement.to_string(),
            restore: restore.to_string(),
        }
    }

    #[test]
    fn parse_extracts_error_spans_and_ignores_warnings() {
        let json = concat!(
            "{\"reason\":\"compiler-artifact\",\"profile\":{\"test\":false}}\n",
            "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":9,\"line_end\":9}],\"children\":[]}}\n",
            "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":3,\"line_end\":3}],\"children\":[{\"level\":\"note\",\"spans\":[{\"file_name\":\"src/other.rs\",\"line_start\":10,\"line_end\":12}],\"children\":[]}]}}\n",
            "{\"reason\":\"build-finished\",\"success\":false}\n",
            "not json at all\n",
        );
        let locs = parse_error_locations(json);
        assert_eq!(locs.len(), 2, "{locs:?}");
        let lib = locs
            .iter()
            .find(|l| l.file == Path::new("src/lib.rs"))
            .unwrap();
        assert_eq!(lib.ranges, vec![(3, 3)]);
        let other = locs
            .iter()
            .find(|l| l.file == Path::new("src/other.rs"))
            .unwrap();
        assert_eq!(other.ranges, vec![(10, 12)]);
    }

    #[test]
    fn manifest_dep_scan_finds_and_misses() {
        let with = "[dependencies]\nrustc-hash = \"2\"\nserde = \"1\"\n";
        assert!(manifest_has_dep(with, "rustc-hash"));
        assert!(!manifest_has_dep(with, "rustc_hash_renamed"));
        assert!(!manifest_has_dep(with, "serde_json"));
        let without = "[dependencies]\nserde = \"1\"\n";
        assert!(!manifest_has_dep(without, "rustc-hash"));
        // workspace-dependency inheritance form
        let inherited = "[dependencies]\nrustc-hash = { workspace = true }\n";
        assert!(manifest_has_dep(inherited, "rustc-hash"));
    }

    #[test]
    fn manifest_feature_count_shapes() {
        // Issue 031 H3: the warning's signal — non-default feature count.
        // katgpt-pruners shape: `default = []` + many opt-ins.
        let gated = "[features]\ndefault = []\npartial_scoring = []\nmonopoly = []\n";
        assert_eq!(manifest_nondefault_feature_count(gated), 2);
        // Default-on features don't count (they ARE the baseline).
        let defaulted = "[features]\ndefault = [\"a\"]\na = []\nb = []\n";
        assert_eq!(manifest_nondefault_feature_count(defaulted), 1);
        // No features table → 0 (silent).
        let none = "[dependencies]\nserde = \"1\"\n";
        assert_eq!(manifest_nondefault_feature_count(none), 0);
        // Multi-line default list (quoted members).
        let multiline = "[features]\ndefault = [\n    \"a\",\n    \"b\",\n]\na = []\nb = []\nc = []\n";
        assert_eq!(manifest_nondefault_feature_count(multiline), 1);
    }

    // `workspace_detection` / `owning_package_root_*` tests moved with the
    // functions to `compile_oracle` (Plan 019 T0.3) — the re-exports above
    // are the same functions, so the pins live beside the bodies.

    #[test]
    fn revert_single_edit_validates_and_splices() {
        let f = FileEditSet {
            path: PathBuf::from("x.rs"),
            original: "let mut v = Vec::new();\nlet x = 1;\n".to_string(),
            fixed: "let mut v = Vec::with_capacity(N);\nlet x = 1;\n".to_string(),
            edits: vec![edit(
                "vec-with-capacity",
                1,
                "Vec::with_capacity(N)",
                "Vec::new()",
            )],
        };
        let mut current = f.fixed.clone();
        let mut applied = vec![true];
        assert!(revert_single_edit(&mut current, &mut applied, &f, 0));
        assert!(!applied[0]);
        assert_eq!(current, f.original);
    }

    #[test]
    fn revert_single_edit_rejects_drifted_text() {
        let f = FileEditSet {
            path: PathBuf::from("x.rs"),
            original: "a\n".to_string(),
            fixed: "b\n".to_string(),
            edits: vec![edit("r", 1, "EXPECT_THIS", "a")],
        };
        let mut current = f.fixed.clone();
        let mut applied = vec![true];
        assert!(!revert_single_edit(&mut current, &mut applied, &f, 0));
        assert!(applied[0]);
        assert_eq!(current, f.fixed);
    }

    #[test]
    fn revert_two_edits_on_one_line_by_replacement() {
        // Two field-shorthand edits sharing line 1 — disambiguated by
        // replacement text.
        let f = FileEditSet {
            path: PathBuf::from("x.rs"),
            original: "Foo { x: x, y: y }\n".to_string(),
            fixed: "Foo { x, y }\n".to_string(),
            edits: vec![
                edit("redundant_field_names", 1, "x", "x: x"),
                edit("redundant_field_names", 1, "y", "y: y"),
            ],
        };
        let mut current = f.fixed.clone();
        let mut applied = vec![true, true];
        assert!(revert_single_edit(&mut current, &mut applied, &f, 0));
        assert_eq!(current, "Foo { x: x, y }\n");
        assert!(revert_single_edit(&mut current, &mut applied, &f, 1));
        assert_eq!(current, f.original);
    }

    #[test]
    fn find_replacement_handles_line_bounds() {
        let text = "ab\nVec::with_capacity(n)\ncd\n";
        let r = "Vec::with_capacity(n)";
        assert_eq!(find_replacement_on_line(text, 2, r), Some(3));
        assert_eq!(find_replacement_on_line(text, 1, r), None);
        assert_eq!(find_replacement_on_line(text, 9, r), None);
        assert_eq!(find_replacement_on_line(text, 0, r), None);
        // Multi-line replacement matched by its first line.
        let ml = "use a::b;\nuse c::d;";
        assert_eq!(find_replacement_on_line(ml, 1, ml), Some(0));
    }

    #[test]
    fn line_in_ranges_is_inclusive() {
        let ranges = vec![(3, 5), (10, 10)];
        assert!(line_in_ranges(3, &ranges));
        assert!(line_in_ranges(4, &ranges));
        assert!(line_in_ranges(5, &ranges));
        assert!(!line_in_ranges(6, &ranges));
        assert!(line_in_ranges(10, &ranges));
        assert!(!line_in_ranges(9, &ranges));
    }

    /// The e2e acceptance fixture (Issue 015 acceptance item 3): a real
    /// temp cargo crate where the verifier must span-revert the two
    /// intentionally-bad edits (u8 capacity path — the G5 E0308 class;
    /// fn-path coercion — the G5 E0631 class) while keeping the good
    /// edits. Runs real cargo → `#[ignore]`d (run explicitly:
    /// `cargo test --features fix_verify --lib -- --ignored fix_verify_e2e -- --nocapture`).
    #[test]
    #[ignore = "spawns cargo (seconds); run explicitly for the Issue 015 acceptance proof"]
    #[allow(clippy::too_many_lines)] // linear fixture script: setup → apply → verify → assert
    fn fix_verify_e2e_bad_edits_reverted_good_survive() {
        let tmp = std::env::temp_dir().join(format!("fv_e2e_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            tmp.join("Cargo.toml"),
            "[package]\nname = \"fv_fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
             [lib]\npath = \"src/lib.rs\"\n",
        )
        .unwrap();

        // Original: one good vec site (usize param), one bad vec site
        // (u8 const), one bad closure site (fn-path coercion loss).
        let original = concat!(
            "const BOARD_SIZE: u8 = 8;\n",
            "\n",
            "fn is_true(b: &bool) -> bool {\n",
            "    *b\n",
            "}\n",
            "\n",
            "pub fn good_vec(n: usize) -> Vec<u32> {\n",
            "    let mut v = Vec::new();\n",
            "    for i in 0..n {\n",
            "        v.push(i as u32);\n",
            "    }\n",
            "    v\n",
            "}\n",
            "\n",
            "pub fn bad_vec() -> Vec<u8> {\n",
            "    let mut v = Vec::new();\n",
            "    for i in 0..BOARD_SIZE {\n",
            "        v.push(i);\n",
            "    }\n",
            "    v\n",
            "}\n",
            "\n",
            "pub fn ok_closure(items: &[bool]) -> usize {\n",
            "    items.iter().filter(|b| is_true(*b)).count()\n",
            "}\n",
            "\n",
            "pub fn bad_closure(items: &[bool]) -> usize {\n",
            "    items.iter().filter(|b| **b).count()\n",
            "}\n",
        );
        std::fs::write(src_dir.join("lib.rs"), original).unwrap();

        // Simulated fix: good vec edit (usize param — compiles), bad vec
        // edit (u8 const — E0308), bad closure edit (filter(is_true) where
        // the closure received &&bool — E0631, the fn-path coercion loss).
        let fixed = original
            .replace(
                "pub fn good_vec(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();",
                "pub fn good_vec(n: usize) -> Vec<u32> {\n    let mut v = Vec::with_capacity(n);",
            )
            .replace(
                "pub fn bad_vec() -> Vec<u8> {\n    let mut v = Vec::new();",
                "pub fn bad_vec() -> Vec<u8> {\n    let mut v = Vec::with_capacity(BOARD_SIZE);",
            )
            .replace(
                "pub fn bad_closure(items: &[bool]) -> usize {\n    items.iter().filter(|b| **b).count()",
                "pub fn bad_closure(items: &[bool]) -> usize {\n    items.iter().filter(is_true).count()",
            );

        let line_of = |text: &str, needle: &str| -> usize {
            text.lines()
                .position(|l| l.contains(needle))
                .map(|i| i + 1)
                .expect(needle)
        };
        let edits = vec![
            EditAtLine {
                rule_id: "vec-with-capacity".to_string(),
                final_line: line_of(&fixed, "Vec::with_capacity(n);"),
                replacement: "Vec::with_capacity(n)".to_string(),
                restore: "Vec::new()".to_string(),
            },
            EditAtLine {
                rule_id: "vec-with-capacity".to_string(),
                final_line: line_of(&fixed, "Vec::with_capacity(BOARD_SIZE);"),
                replacement: "Vec::with_capacity(BOARD_SIZE)".to_string(),
                restore: "Vec::new()".to_string(),
            },
            EditAtLine {
                rule_id: "redundant_closure".to_string(),
                final_line: line_of(&fixed, "filter(is_true)"),
                replacement: "is_true".to_string(),
                restore: "|b| **b".to_string(),
            },
        ];

        let set = FileEditSet {
            path: src_dir.join("lib.rs"),
            original: original.to_string(),
            fixed,
            edits,
        };
        let report = FixVerifier::new().verify(&[set]);
        assert!(report.ran, "verifier ran");
        assert!(
            report.baseline_clean,
            "baseline must pass: {:?}",
            report.error
        );
        assert_eq!(
            report.kept_edits, 1,
            "only the good vec edit survives: {report:?}"
        );
        assert_eq!(report.kept_files, 1);
        let reverted_rules: Vec<&str> = report.reverted.iter().map(|r| r.rule_id.as_str()).collect();
        assert_eq!(reverted_rules.len(), 2, "{report:?}");
        assert!(reverted_rules.contains(&"vec-with-capacity"));
        assert!(reverted_rules.contains(&"redundant_closure"));
        // Disk state: the bad edits are gone, the good edit remains.
        let on_disk = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap();
        assert!(on_disk.contains("Vec::with_capacity(n)"), "good edit kept");
        assert!(
            !on_disk.contains("Vec::with_capacity(BOARD_SIZE)"),
            "bad u8 edit reverted"
        );
        assert!(on_disk.contains("|b| **b"), "bad closure edit reverted");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The workspace-belief e2e (the container-tick-test incident, 09-04):
    /// a nested crate WITHOUT a [workspace] table under a parent workspace
    /// that does not list it. The baseline `cargo check` must fail-closed
    /// (nothing written, no edits applied) and the error must carry the
    /// enriched diagnosis — cargo's signature line + the root cause + the
    /// self-rooting remedy — not just the one-line stderr the incident
    /// left us with. Runs real cargo → `#[ignore]`d like the acceptance
    /// fixture above.
    #[test]
    #[ignore = "spawns cargo (seconds); run explicitly for the workspace-belief proof"]
    fn fix_verify_e2e_workspace_belief_refusal_names_the_remedy() {
        let tmp = std::env::temp_dir().join(format!("fv_e2e_wsb_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // Parent workspace (empty members is legal) + a nested standalone
        // crate WITHOUT [workspace] — cargo walks up from the nested cwd,
        // adopts the parent workspace, and refuses.
        let nested = tmp.join("cloudflare").join("tick");
        std::fs::create_dir_all(nested.join("src")).unwrap();
        std::fs::write(
            tmp.join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        std::fs::write(
            nested.join("Cargo.toml"),
            "[package]\nname = \"tick\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let original = "pub fn f(n: usize) -> Vec<u32> {\n    let mut v = Vec::new();\n    v\n}\n";
        std::fs::write(nested.join("src").join("lib.rs"), original).unwrap();

        let fixed = original.replace("Vec::new();", "Vec::with_capacity(n);");
        let set = FileEditSet {
            path: nested.join("src").join("lib.rs"),
            original: original.to_string(),
            fixed,
            edits: vec![EditAtLine {
                rule_id: "vec-with-capacity".to_string(),
                final_line: 2,
                replacement: "Vec::with_capacity(n)".to_string(),
                restore: "Vec::new()".to_string(),
            }],
        };
        let report = FixVerifier::new().verify(&[set]);
        assert!(report.ran, "verifier ran");
        let err = report.error.as_deref().expect("workspace refusal surfaces");
        assert!(
            err.contains("believes it's in a workspace"),
            "keeps cargo's signature: {err}"
        );
        assert!(err.contains("[workspace] table"), "names the remedy: {err}");
        assert!(!report.baseline_clean, "baseline never passed");
        // Fail-closed BEFORE any write: disk still holds the original.
        let on_disk = std::fs::read_to_string(nested.join("src").join("lib.rs")).unwrap();
        assert_eq!(on_disk, original, "nothing written on a refused baseline");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── 037-T2: lint-resolution oracle parsing + verdict matrix ─────────

    #[test]
    fn parse_clippy_findings_takes_primary_warning_spans_only() {
        let json = concat!(
            "{\"reason\":\"compiler-artifact\"}\n",
            // A clippy warning: primary span kept, secondary span dropped.
            "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"clippy::uninlined_format_args\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":4,\"line_end\":4,\"primary\":true},{\"file_name\":\"src/lib.rs\",\"line_start\":9,\"line_end\":9,\"primary\":false}]}}\n",
            // An error carries no resolution signal — dropped.
            "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"code\":{\"code\":\"clippy::something\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":2,\"line_end\":2,\"primary\":true}]}}\n",
            // A rustc warning (no clippy:: prefix) — dropped.
            "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"code\":{\"code\":\"unused_variables\"},\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":6,\"line_end\":6,\"primary\":true}]}}\n",
            // A warning with no code — dropped.
            "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"spans\":[{\"file_name\":\"src/lib.rs\",\"line_start\":8,\"line_end\":8,\"primary\":true}]}}\n",
        );
        let findings = parse_clippy_findings(json);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].code, "clippy::uninlined_format_args");
        assert_eq!(findings[0].file, Path::new("src/lib.rs"));
        assert_eq!((findings[0].line_start, findings[0].line_end), (4, 4));
    }

    #[test]
    fn kept_edit_verdict_matrix() {
        // Oracle-confirmed resolve → Certified.
        assert_eq!(kept_edit_verdict(Some(true)), (true, true));
        // Persist → verified failure (the honest persist signal).
        assert_eq!(kept_edit_verdict(Some(false)), (false, true));
        // No oracle verdict (not built / unrunnable / non-clippy rule) →
        // rustc-only Heuristic outcome-success.
        assert_eq!(kept_edit_verdict(None), (true, false));
    }
}

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
use std::process::Command;

use serde::Deserialize;

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
    cargo_bin: String,
    /// Extra args appended to every `cargo check` invocation (e.g.
    /// `["--features", "a,b,c"]`). Whitespace-split by the caller.
    extra_args: Vec<String>,
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
            cargo_bin: "cargo".to_string(),
            extra_args: Vec::new(),
        }
    }

    /// Append extra cargo args to every check (feature sets etc.).
    #[must_use]
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
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

    /// One `cargo check --all-targets --message-format=json` run in `pkg`.
    /// Fails (Err) only when cargo could not run at all (missing binary,
    /// broken manifest, bad args) — compile errors come back as
    /// `Ok(outcome)` with `exit_ok = false` + error diagnostics in the JSON.
    fn run_check(&self, pkg: &Path) -> Result<CheckOutcome, String> {
        let mut cmd = Command::new(&self.cargo_bin);
        cmd.current_dir(pkg)
            .arg("check")
            .arg("--all-targets")
            .arg("--message-format=json");
        for a in &self.extra_args {
            cmd.arg(a);
        }
        let output = cmd
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.status.success() && !stdout.lines().any(is_json_line) {
            // cargo itself failed without emitting JSON (bad args, broken
            // manifest) — surface stderr.
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(format!(
                "cargo check failed to run in {}: {}",
                pkg.display(),
                first_meaningful_line(&stderr)
            ));
        }
        Ok(CheckOutcome {
            json: stdout,
            exit_ok: output.status.success(),
        })
    }
}

fn is_json_line(line: &str) -> bool {
    line.trim_start().starts_with('{')
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

fn first_meaningful_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("(no stderr)")
        .to_string()
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

/// Walk up from a `.rs` file to the nearest directory holding a
/// `Cargo.toml` — the owning package root (cargo forbids nested packages,
/// so the first manifest is THE manifest of the crate the file compiles
/// into).
#[must_use]
pub fn owning_package_root(file: &Path) -> Option<PathBuf> {
    // Normalize to an absolute path first: with a relative input like
    // `src/foo.rs` (files collected from CLI args), `pop()` walks down to
    // the EMPTY path whose `Cargo.toml` join resolves against the process
    // cwd — returning `Some("")` whose `Command::current_dir` then fails
    // with os error 123 (the Issue 015 GOAT run's spawn failure).
    let file = std::path::absolute(file).ok()?;
    let mut dir = file.parent()?.to_path_buf();
    loop {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Walk up from a package root to the workspace root (the nearest
/// ancestor — including the package itself — whose `Cargo.toml` declares
/// a `[workspace]` table). Falls back to the package root for standalone
/// crates. Used to resolve rustc's workspace-relative `file_name`s.
#[must_use]
pub fn workspace_root(pkg_root: &Path) -> Option<PathBuf> {
    let mut dir = Some(pkg_root.to_path_buf());
    while let Some(d) = dir {
        let manifest = d.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest) {
            if manifest_declares_workspace(&text) {
                return Some(d);
            }
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    // No [workspace] found above: the package stands alone.
    Some(pkg_root.to_path_buf())
}

fn manifest_declares_workspace(text: &str) -> bool {
    text.lines()
        .any(|l| l.trim_start().starts_with("[workspace"))
}

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
    spans: Vec<DiagSpan>,
    #[serde(default)]
    children: Vec<Diagnostic>,
}

#[derive(Deserialize)]
struct DiagSpan {
    file_name: String,
    line_start: usize,
    line_end: usize,
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

/// Resolve a rustc-reported `file_name` to a comparable absolute path:
/// absolute → canonicalize; relative → try workspace-root join, then
/// package-root join, then the raw path (comparison-safe miss rather than
/// a wrong hit).
fn resolve_reported_path(p: &Path, ws: &Path, pkg: &Path) -> PathBuf {
    if p.is_absolute() {
        return std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    }
    let joined = ws.join(p);
    if let Ok(c) = std::fs::canonicalize(&joined) {
        return c;
    }
    let joined_pkg = pkg.join(p);
    if let Ok(c) = std::fs::canonicalize(&joined_pkg) {
        return c;
    }
    joined
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
    fn workspace_detection_reads_manifest_tables() {
        assert!(manifest_declares_workspace(
            "[package]\nname = \"x\"\n\n[workspace]\nmembers = [\"crates/a\"]\n"
        ));
        assert!(!manifest_declares_workspace("[package]\nname = \"x\"\n"));
    }

    #[test]
    fn owning_package_root_walks_up() {
        let tmp = std::env::temp_dir().join(format!("fv_pkg_root_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let pkg = tmp.join("ws").join("crates").join("a");
        std::fs::create_dir_all(pkg.join("src")).unwrap();
        std::fs::write(pkg.join("Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        std::fs::write(
            tmp.join("ws").join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/a\"]\n",
        )
        .unwrap();
        let f = pkg.join("src").join("lib.rs");
        assert_eq!(owning_package_root(&f).as_deref(), Some(pkg.as_path()));
        assert_eq!(
            workspace_root(&pkg).as_deref(),
            Some(tmp.join("ws").as_path())
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Issue 015 GOAT-run regression: a RELATIVE input (`src/lib.rs`, the
    /// form CLI path args produce) must resolve to the absolute package
    /// root — not the empty path whose `current_dir` fails with os error
    /// 123. The walk-down-to-`""` case bites when the manifest sits at
    /// the process cwd ("src/x.rs" → "src" → "" → "Cargo.toml").
    #[test]
    fn owning_package_root_relative_input_is_absolute() {
        let tmp = std::env::temp_dir().join(format!("fv_pkg_rel_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        std::fs::write(tmp.join("Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        std::fs::write(tmp.join("src").join("lib.rs"), "").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();
        let rel = Path::new("src/lib.rs");
        let got = owning_package_root(rel).expect("package root found");
        std::env::set_current_dir(prev).unwrap();
        assert!(
            got.is_absolute(),
            "relative input must return an absolute package root, got {got:?}"
        );
        // Form-independent checks (canonicalize would return a \\?\ prefix
        // on Windows while std::path::absolute preserves the plain form).
        assert_eq!(got.file_name(), tmp.file_name());
        assert!(got.join("Cargo.toml").is_file());
        let _ = std::fs::remove_dir_all(&tmp);
    }

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
}

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
//! - A check that exits non-zero with NO attributable diagnostics (the
//!   cargo process killed mid-run — OOM, user kill — output lost, or a
//!   cargo-level refusal that emitted artifact JSON): the same refusal as
//!   the arm above. A killed re-check is indistinguishable from a clean
//!   pass on the JSON alone and USED to read as one — every kept edit
//!   would have ridden an incomplete verification (katgpt-rs Issue 841).
//!   The exit status is the completion sentinel: no keep may ride a
//!   check that did not complete ([`check_verdict`]).
//! - Warnings do not fail the check (no `-D warnings`): verification
//!   proves the edits COMPILE, not that they are warning-free.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// Plan 019 T0.3: the subprocess + path-walk substrate moved to
// `compile_oracle` so the `rustc_errors` driver can consume it without the
// `latent_retrieval` tree. Re-exported here for back-compat — the bin and
// Re-exported here for back-compat — the bin and
// the tests reach them through `fix_verify::`.
pub use crate::compile_oracle::{
    check_verdict, owning_package_root, resolve_reported_path, workspace_root, CheckRunner,
    CheckVerdict,
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

    /// Explicit cargo binary (test/diagnostic seam — the kill-path e2e
    /// points this at a fake cargo; production callers use [`Self::new`]).
    #[must_use]
    pub fn with_cargo_bin(bin: impl Into<String>) -> Self {
        Self {
            runner: CheckRunner::new().with_cargo_bin(bin),
        }
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
                match self.classified_check(pkg, &mut report) {
                    // Errors present: merge them for attribution. Clean:
                    // nothing to do. Err (unrunnable or incomplete — Issue
                    // 841): restore everything, keep nothing unverified.
                    Ok(Some(json)) => merge_error_locations(
                        &json,
                        workspace_root(pkg).as_deref(),
                        pkg,
                        &mut error_lines,
                    ),
                    Ok(None) => {}
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
            let step =
                attribute_and_revert(&touched, &canon, &error_lines, &mut state, &mut report);
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
            // The shared verdict's Errors and Incomplete arms both refuse
            // the baseline: a broken tree cannot host a verification, and a
            // KILLED baseline check must not pass as clean either (Issue
            // 841).
            if check_verdict(outcome.exit_ok, !errors.is_empty()) == CheckVerdict::Clean {
                continue;
            }
            report.baseline_clean = false;
            return Ok(()); // message set by the caller from the flag
        }
        report.baseline_clean = true;
        Ok(())
    }

    /// One package check, classified through the shared verdict (Issue
    /// 841): `Ok(None)` = the package is clean; `Ok(Some(json))` = error
    /// diagnostics present — the caller merges them for attribution; `Err`
    /// = the check cannot be trusted — cargo unrunnable, or the check did
    /// not COMPLETE (non-zero exit, zero parseable diagnostics: the
    /// process was killed mid-run, the output was lost, or cargo refused
    /// after emitting artifact JSON). That incomplete shape is
    /// indistinguishable from a clean pass on the JSON alone and used to
    /// read as one — the caller restores everything and keeps nothing.
    fn classified_check(
        &self,
        pkg: &Path,
        report: &mut VerifyReport,
    ) -> Result<Option<String>, String> {
        let outcome = self.run_check(pkg)?;
        report.n_checks += 1;
        match check_verdict(outcome.exit_ok, !parse_error_locations(&outcome.json).is_empty()) {
            CheckVerdict::Clean => Ok(None),
            CheckVerdict::Errors => Ok(Some(outcome.json)),
            CheckVerdict::Incomplete => Err(format!(
                "cargo check in {} exited unsuccessfully with no error \
                 diagnostics — the check did not complete (killed, output \
                 lost, or a cargo-level refusal); nothing kept as verified",
                pkg.display()
            )),
        }
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
fn revert_single_edit(
    current: &mut String,
    applied: &mut [bool],
    f: &FileEditSet,
    i: usize,
) -> bool {
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
    let (defaults, keys) = manifest_feature_names(manifest_text);
    keys.into_iter()
        .filter(|k| !defaults.iter().any(|d| d == k))
        .count()
}

/// The cfg-audit half of the same scan: the package's `default` feature
/// list + every declared feature key (the feature-set constructor below
/// consumes both; `manifest_nondefault_feature_count` is the count-shaped
/// consumer of the same parse).
#[must_use]
pub fn manifest_feature_names(manifest_text: &str) -> (Vec<String>, Vec<String>) {
    let mut in_features = false;
    let mut default_deps: Vec<String> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
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
                keys.push(key.to_string());
            }
        } else if let Some(pd) = pending_default.as_mut() {
            pd.push(' ');
            pd.push_str(t);
        }
    }
    finalize_default(&mut pending_default, &mut default_deps);
    (default_deps, keys)
}

// ── the post-heal cfg audit (the 2026-09-10 intake) ─────────────────────
//
// The `--verify` gate compiles at the INVOCATION's feature state, so an
// edit inside a `#[cfg(feature = …)]` region can be reported "verified"
// without ever having been compiled. Two recorded casualties: the E0631
// point-free filter closure behind `score_bench` (2026-09-09, `ce74bac` —
// verified green at default features, broke the feature build for 3 days)
// and the `pvp_karma` load-bearing `mut` dropped under a partial feature
// set (P13 — `--verify-args` was PASSED and the coarse Issue-031-H3
// warning correctly stayed silent; the set simply lacked that one
// feature). The verifier is the only honest place for this guard: the
// proposers are typeless (syn), so coercion/liveness-load-bearing
// rewrites are undecidable there.
//
// The audit is REPORT-ONLY (a mark, never a revert): it names each kept
// edit whose enclosing cfg context the baseline provably did not compile,
// plus the exact `--verify-args` re-run that would cover it.
//
// The binding-association half (P13's literal shape, 2026-09-14): an edit
// on a `let` declaring a binding that is ASSIGNED, compound-assigned,
// `&mut`-borrowed, or method-called inside a cfg'd region the baseline
// did not compile is marked too — the `unused_mut` casualty class, where
// the edit sits OUTSIDE the gate and only the load-bearing USAGE lives
// inside it. Association is name-based (v1): single-ident `let` pats only
// (tuple/struct pats and fn params out of scope), method-call receivers
// count as mutations (syn cannot see &self vs &mut self — over-marking,
// conservative), mutations inside macro bodies are invisible, and
// shadowed same-name bindings may over-mark.

/// The enabled-feature set the verify gate compiled with.
#[derive(Debug, Clone, Default)]
pub struct FeatureSet {
    /// `--all-features` was passed — every declared feature is on.
    pub all_on: bool,
    /// Explicitly enabled feature names (crate defaults + parsed
    /// `--features` entries that name THIS crate's features).
    pub names: std::collections::HashSet<String>,
}

impl FeatureSet {
    #[must_use]
    pub fn contains(&self, feature: &str) -> bool {
        self.all_on || self.names.contains(feature)
    }
}

/// Assemble the enabled set the gate compiled with: the manifest's default
/// features (unless `--no-default-features`) + every `--features` entry
/// naming one of the manifest's own features (dep-prefixed `a/b` and
/// `a?/b` forms name OTHER crates' features — ignored). `--all-features`
/// switches `all_on`, which short-circuits every feature atom to enabled.
#[must_use]
pub fn feature_set_from_args(
    manifest_text: &str,
    verify_args: Option<&str>,
) -> FeatureSet {
    let (defaults, keys) = manifest_feature_names(manifest_text);
    let mut set = FeatureSet::default();
    let Some(args) = verify_args else {
        set.names.extend(defaults);
        return set;
    };
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if tokens.contains(&"--all-features") {
        set.all_on = true;
        set.names.extend(defaults);
        set.names.extend(keys);
        return set;
    }
    if !tokens.contains(&"--no-default-features") {
        set.names.extend(defaults);
    }
    let mut requested: Vec<String> = Vec::new();
    let mut iter = tokens.iter().peekable();
    while let Some(t) = iter.next() {
        let value = if let Some(rest) = t.strip_prefix("--features=") {
            Some(rest.to_string())
        } else if *t == "--features" {
            iter.next().map(|v| (*v).to_string())
        } else {
            None
        };
        if let Some(v) = value {
            requested.extend(
                v.split([',', ' ', '\t'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
    }
    for r in requested {
        if keys.contains(&r) {
            set.names.insert(r);
        }
    }
    set
}

/// One cfg predicate, restricted to what the audit can evaluate.
#[derive(Debug, Clone, PartialEq)]
enum CfgAtom {
    Feature(String),
    /// `test` — the gate runs `cargo check --all-targets`, which compiles
    /// the test targets, so `#[cfg(test)]` code IS verified.
    Test,
    /// A host predicate the healer can settle from its own build
    /// (the gate compiles for the host it runs on).
    Host(String, Option<String>),
    /// Anything else (doc, version, compiler, unknown keys…) — undecidable.
    Other,
}

#[derive(Debug, Clone)]
enum CfgExpr {
    Atom(CfgAtom),
    Not(Box<CfgExpr>),
    All(Vec<CfgExpr>),
    Any(Vec<CfgExpr>),
}

impl CfgExpr {
    /// The raw source text (for the report's "undecidable" column).
    fn render(&self) -> String {
        match self {
            Self::Atom(CfgAtom::Feature(f)) => format!("feature = \"{f}\""),
            Self::Atom(CfgAtom::Test) => "test".to_string(),
            Self::Atom(CfgAtom::Host(k, Some(v))) => format!("{k} = \"{v}\""),
            Self::Atom(CfgAtom::Host(k, None)) => k.clone(),
            Self::Atom(CfgAtom::Other) => "<opaque>".to_string(),
            Self::Not(inner) => format!("not({})", inner.render()),
            Self::All(parts) => {
                format!("all({})", parts.iter().map(Self::render).collect::<Vec<_>>().join(", "))
            }
            Self::Any(parts) => {
                format!("any({})", parts.iter().map(Self::render).collect::<Vec<_>>().join(", "))
            }
        }
    }

    /// Every `feature = "…"` name mentioned (for the not-enabled column).
    fn feature_names(&self, out: &mut Vec<String>) {
        match self {
            Self::Atom(CfgAtom::Feature(f)) => {
                if !out.contains(f) {
                    out.push(f.clone());
                }
            }
            Self::Not(inner) => inner.feature_names(out),
            Self::All(parts) | Self::Any(parts) => {
                for p in parts {
                    p.feature_names(out);
                }
            }
            Self::Atom(_) => {}
        }
    }
}

/// Three-valued evaluation: `True` = provably compiled by the gate,
/// `False` = provably compiled to NOTHING, `Unknown` = undecidable from
/// the manifest + args alone.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tv {
    True,
    False,
    Unknown,
}

fn eval_cfg(expr: &CfgExpr, set: &FeatureSet) -> Tv {
    match expr {
        CfgExpr::Atom(CfgAtom::Feature(f)) => {
            if set.contains(f) {
                Tv::True
            } else {
                Tv::False
            }
        }
        CfgExpr::Atom(CfgAtom::Test) => Tv::True,
        CfgExpr::Atom(CfgAtom::Host(key, want)) => {
            // Settled from the healer's OWN build: the verify gate compiles
            // for the host it runs on, so the healer's cfg truth IS the
            // gate's cfg truth for these atoms.
            let truth: Option<bool> = match (key.as_str(), want.as_deref()) {
                ("target_os", Some(v)) => Some(std::env::consts::OS == v),
                ("target_arch", Some(v)) => Some(std::env::consts::ARCH == v),
                ("target_pointer_width", Some(v)) => {
                    Some((std::mem::size_of::<usize>() * 8).to_string() == v)
                }
                ("target_family", Some(v)) => Some(std::env::consts::FAMILY == v),
                ("unix", None) => Some(std::cfg!(unix)),
                ("windows", None) => Some(std::cfg!(windows)),
                _ => None,
            };
            match truth {
                Some(true) => Tv::True,
                Some(false) => Tv::False,
                None => Tv::Unknown,
            }
        }
        CfgExpr::Atom(CfgAtom::Other) => Tv::Unknown,
        CfgExpr::Not(inner) => match eval_cfg(inner, set) {
            Tv::True => Tv::False,
            Tv::False => Tv::True,
            Tv::Unknown => Tv::Unknown,
        },
        CfgExpr::All(parts) => {
            let mut any_unknown = false;
            for p in parts {
                match eval_cfg(p, set) {
                    Tv::False => return Tv::False,
                    Tv::Unknown => any_unknown = true,
                    Tv::True => {}
                }
            }
            if any_unknown {
                Tv::Unknown
            } else {
                Tv::True
            }
        }
        CfgExpr::Any(parts) => {
            let mut any_unknown = false;
            for p in parts {
                match eval_cfg(p, set) {
                    Tv::True => return Tv::True,
                    Tv::Unknown => any_unknown = true,
                    Tv::False => {}
                }
            }
            if any_unknown {
                Tv::Unknown
            } else {
                Tv::False
            }
        }
    }
}

/// Parse `#[cfg(<meta>)]` attribute tokens into a [`CfgExpr`]. `cfg_attr`
/// never gates compilation of the item itself and is skipped.
fn cfg_expr_from_attr(attr: &syn::Attribute) -> Option<CfgExpr> {
    let syn::Meta::List(list) = &attr.meta else {
        return None;
    };
    if !list.path.is_ident("cfg") {
        return None;
    }
    let parsed: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> = list
        .parse_args_with(syn::punctuated::Punctuated::parse_terminated)
        .ok()?;
    let mut exprs: Vec<CfgExpr> = Vec::new();
    for meta in &parsed {
        exprs.push(cfg_expr_from_meta(meta)?);
    }
    match exprs.len() {
        0 => None,
        1 => Some(exprs.pop().expect("len checked")),
        // `#[cfg(a)] #[cfg(b)]` on one item ANDs; a comma-separated list in
        // ONE cfg is not valid rustc, but ANDing is the conservative read.
        _ => Some(CfgExpr::All(exprs)),
    }
}

fn cfg_expr_from_meta(meta: &syn::Meta) -> Option<CfgExpr> {
    match meta {
        syn::Meta::Path(p) => {
            let ident = p.segments.last()?.ident.to_string();
            let atom = match ident.as_str() {
                "test" => CfgAtom::Test,
                "unix" | "windows" => CfgAtom::Host(ident, None),
                _ => CfgAtom::Other,
            };
            Some(CfgExpr::Atom(atom))
        }
        syn::Meta::List(list) => {
            let inner: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> = list
                .parse_args_with(syn::punctuated::Punctuated::parse_terminated)
                .ok()?;
            let mut parts: Vec<CfgExpr> = Vec::new();
            for m in &inner {
                parts.push(cfg_expr_from_meta(m)?);
            }
            if list.path.is_ident("all") {
                Some(CfgExpr::All(parts))
            } else if list.path.is_ident("any") {
                Some(CfgExpr::Any(parts))
            } else if list.path.is_ident("not") {
                let mut it = parts.into_iter();
                let first = it.next()?;
                Some(CfgExpr::Not(Box::new(first)))
            } else {
                Some(CfgExpr::Atom(CfgAtom::Other))
            }
        }
        syn::Meta::NameValue(nv) => {
            let key = nv.path.segments.last()?.ident.to_string();
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) = &nv.value
            else {
                return Some(CfgExpr::Atom(CfgAtom::Other));
            };
            match key.as_str() {
                "feature" => Some(CfgExpr::Atom(CfgAtom::Feature(s.value()))),
                "target_os" | "target_arch" | "target_pointer_width" | "target_family" => {
                    Some(CfgExpr::Atom(CfgAtom::Host(key, Some(s.value()))))
                }
                _ => Some(CfgExpr::Atom(CfgAtom::Other)),
            }
        }
    }
}

/// One cfg-gated region in the fixed source: a line range + its predicate.
struct CfgRegion {
    start_line: usize,
    end_line: usize,
    expr: CfgExpr,
}

/// Fold one node's cfg attrs into a single predicate (multiple attrs AND,
/// matching rustc's semantics for `#[cfg(a)] #[cfg(b)]`).
fn conjoin(mut exprs: Vec<CfgExpr>) -> CfgExpr {
    match exprs.len() {
        0 => CfgExpr::Atom(CfgAtom::Other),
        1 => exprs.pop().expect("len checked"),
        _ => CfgExpr::All(exprs),
    }
}

fn span_lines<S: syn::spanned::Spanned>(node: &S) -> Option<(usize, usize)> {
    let span = node.span();
    let start = span.start();
    let end = span.end();
    // proc_macro2 fallback spans are 1-based lines; a degenerate
    // 0-column/0-line fallback (unparseable spans) is refused.
    if start.line == 0 || end.line == 0 {
        return None;
    }
    Some((start.line, end.line))
}

/// Attributes of an `Item`/`ImplItem`/`TraitItem`/`Stmt` without matching
/// every variant by hand (syn 2 exposes attrs as FIELDS on the variants;
/// these small match shims are the honest accessors).
fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        // syn's item enums are #[non_exhaustive]; Verbatim and any future
        // variant carry no attrs we can reach this way.
        _ => &[],
    }
}

fn impl_item_attrs(ii: &syn::ImplItem) -> &[syn::Attribute] {
    match ii {
        syn::ImplItem::Const(i) => &i.attrs,
        syn::ImplItem::Fn(i) => &i.attrs,
        syn::ImplItem::Macro(i) => &i.attrs,
        syn::ImplItem::Type(i) => &i.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(ti: &syn::TraitItem) -> &[syn::Attribute] {
    match ti {
        syn::TraitItem::Const(i) => &i.attrs,
        syn::TraitItem::Fn(i) => &i.attrs,
        syn::TraitItem::Macro(i) => &i.attrs,
        syn::TraitItem::Type(i) => &i.attrs,
        _ => &[],
    }
}

fn stmt_attrs(stmt: &syn::Stmt) -> &[syn::Attribute] {
    match stmt {
        syn::Stmt::Local(i) => &i.attrs,
        syn::Stmt::Item(i) => item_attrs(i),
        syn::Stmt::Expr(_, _) => &[],
        syn::Stmt::Macro(i) => &i.attrs,
    }
}

fn item_cfg_regions(item: &syn::Item, out: &mut Vec<CfgRegion>) {
    let exprs: Vec<CfgExpr> = item_attrs(item)
        .iter()
        .filter_map(cfg_expr_from_attr)
        .collect();
    if let (Some((start, end)), false) = (span_lines(item), exprs.is_empty()) {
        out.push(CfgRegion {
            start_line: start,
            end_line: end,
            expr: conjoin(exprs),
        });
    }
    match item {
        syn::Item::Mod(syn::ItemMod {
            content: Some((_, items)),
            ..
        }) => {
            for inner in items {
                item_cfg_regions(inner, out);
            }
        }
        syn::Item::Impl(syn::ItemImpl { items, .. }) => {
            for ii in items {
                let inner_exprs: Vec<CfgExpr> = impl_item_attrs(ii)
                    .iter()
                    .filter_map(cfg_expr_from_attr)
                    .collect();
                if let (Some((s, e)), false) = (span_lines(ii), inner_exprs.is_empty()) {
                    out.push(CfgRegion {
                        start_line: s,
                        end_line: e,
                        expr: conjoin(inner_exprs),
                    });
                }
                if let syn::ImplItem::Fn(f) = ii {
                    stmt_cfg_regions(&f.block.stmts, out);
                }
            }
        }
        syn::Item::Trait(syn::ItemTrait { items, .. }) => {
            for ti in items {
                let inner_exprs: Vec<CfgExpr> = trait_item_attrs(ti)
                    .iter()
                    .filter_map(cfg_expr_from_attr)
                    .collect();
                if let (Some((s, e)), false) = (span_lines(ti), inner_exprs.is_empty()) {
                    out.push(CfgRegion {
                        start_line: s,
                        end_line: e,
                        expr: conjoin(inner_exprs),
                    });
                }
                if let syn::TraitItem::Fn(syn::TraitItemFn {
                    default: Some(block),
                    ..
                }) = ti
                {
                    stmt_cfg_regions(&block.stmts, out);
                }
            }
        }
        syn::Item::Fn(f) => stmt_cfg_regions(&f.block.stmts, out),
        _ => {}
    }
}

/// Statement-position cfg regions (the P13 `pvp_karma` class: a cfg'd
/// block inside a fn body re-assigning a binding the edit touched). v1
/// covers item-level gates, fn/impl/trait item bodies, statement-position
/// `#[cfg]` blocks/lets, and cfg'd match arms; attributes on inner
/// non-statement expressions and struct fields are out of scope.
fn stmt_cfg_regions(stmts: &[syn::Stmt], out: &mut Vec<CfgRegion>) {
    for stmt in stmts {
        let exprs: Vec<CfgExpr> = stmt_attrs(stmt)
            .iter()
            .filter_map(cfg_expr_from_attr)
            .collect();
        if let (Some((s, e)), false) = (span_lines(stmt), exprs.is_empty()) {
            out.push(CfgRegion {
                start_line: s,
                end_line: e,
                expr: conjoin(exprs),
            });
        }
        match stmt {
            syn::Stmt::Item(inner) => item_cfg_regions(inner, out),
            syn::Stmt::Expr(e, _) => {
                // A cfg'd statement-position block (`#[cfg(x)] { … }`, the
                // P13 pvp_karma shape) carries its attrs on the ExprBlock
                // itself, not on the Stmt — catch it here, then recurse.
                if let syn::Expr::Block(be) = e {
                    let block_exprs: Vec<CfgExpr> =
                        be.attrs.iter().filter_map(cfg_expr_from_attr).collect();
                    if let (Some((s, e2)), false) = (span_lines(stmt), block_exprs.is_empty()) {
                        out.push(CfgRegion {
                            start_line: s,
                            end_line: e2,
                            expr: conjoin(block_exprs),
                        });
                    }
                }
                expr_cfg_regions(e, out);
            }
            _ => {}
        }
    }
}

fn expr_cfg_regions(expr: &syn::Expr, out: &mut Vec<CfgRegion>) {
    match expr {
        syn::Expr::Block(syn::ExprBlock { block, .. })
        | syn::Expr::Unsafe(syn::ExprUnsafe { block, .. })
        | syn::Expr::Const(syn::ExprConst { block, .. }) => {
            stmt_cfg_regions(&block.stmts, out);
        }
        syn::Expr::If(syn::ExprIf {
            then_branch,
            else_branch,
            ..
        }) => {
            stmt_cfg_regions(&then_branch.stmts, out);
            if let Some((_, els)) = else_branch {
                expr_cfg_regions(els, out);
            }
        }
        syn::Expr::Match(syn::ExprMatch { arms, .. }) => {
            for arm in arms {
                let exprs: Vec<CfgExpr> =
                    arm.attrs.iter().filter_map(cfg_expr_from_attr).collect();
                if let (Some((s, e)), false) = (span_lines(arm), exprs.is_empty()) {
                    out.push(CfgRegion {
                        start_line: s,
                        end_line: e,
                        expr: conjoin(exprs),
                    });
                }
                expr_cfg_regions(&arm.body, out);
            }
        }
        syn::Expr::Loop(syn::ExprLoop { body, .. }) => stmt_cfg_regions(&body.stmts, out),
        _ => {}
    }
}

/// The features missing (and gates undecidable) for ONE region under the
/// gate's feature set — `None` when the baseline provably compiled the
/// region (`Tv::True`). Shared by the span path and the binding path.
fn region_gap(r: &CfgRegion, set: &FeatureSet) -> Option<(Vec<String>, Vec<String>)> {
    if eval_cfg(&r.expr, set) == Tv::True {
        return None;
    }
    let mut features: Vec<String> = Vec::new();
    r.expr.feature_names(&mut features);
    let missing: Vec<String> = features
        .iter()
        .filter(|f| !set.contains(f.as_str()))
        .cloned()
        .collect();
    if missing.is_empty() {
        // Off for a non-additive reason (`not(feature)` with the feature
        // on, or an undecidable atom) — --features cannot fix it; report
        // the raw gate.
        Some((Vec::new(), vec![r.expr.render()]))
    } else {
        let und = if eval_cfg(&r.expr, set) == Tv::Unknown {
            vec![r.expr.render()]
        } else {
            Vec::new()
        };
        Some((missing, und))
    }
}

/// A plain single-segment path expression (`slots`), not `a::b`, not a
/// field (`self.x`) — the shape a local-binding mutation must have.
fn path_ident(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Path(syn::ExprPath { path, .. }) = expr {
        if path.segments.len() == 1 && path.leading_colon.is_none() {
            let seg = &path.segments[0];
            if seg.arguments.is_none() {
                return Some(seg.ident.to_string());
            }
        }
    }
    None
}

/// Name-and-line scan for the binding-association path: where each
/// single-ident `let` lives, and where each mutability-requiring use of
/// an identifier occurs. Line-keyed so region membership is decided by
/// the SAME spans the span path uses.
struct BindingScan {
    /// `(ident, start_line, end_line)` of every single-ident `let`.
    lets: Vec<(String, usize, usize)>,
    /// `(ident, line)` of every assignment / compound assignment /
    /// `&mut` borrow / method-call receiver on a plain identifier.
    mutations: Vec<(String, usize)>,
}

impl BindingScan {
    fn of(file: &syn::File) -> Self {
        let mut scan = BindingScan {
            lets: Vec::new(),
            mutations: Vec::new(),
        };
        syn::visit::visit_file(&mut scan, file);
        scan
    }
}

impl<'ast> syn::visit::Visit<'ast> for BindingScan {
    fn visit_expr_assign(&mut self, i: &'ast syn::ExprAssign) {
        if let (Some(id), Some((l, _))) = (path_ident(&i.left), span_lines(i.left.as_ref())) {
            self.mutations.push((id, l));
        }
        syn::visit::visit_expr_assign(self, i);
    }

    fn visit_expr_binary(&mut self, i: &'ast syn::ExprBinary) {
        let is_assign_op = matches!(
            i.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        );
        if is_assign_op {
            if let (Some(id), Some((l, _))) = (path_ident(&i.left), span_lines(i.left.as_ref())) {
                self.mutations.push((id, l));
            }
        }
        syn::visit::visit_expr_binary(self, i);
    }

    fn visit_expr_reference(&mut self, i: &'ast syn::ExprReference) {
        // `&mut x` — syn 2 models a reference as its own expr kind, not a
        // unary op; `mutability` carries the `mut` token.
        if i.mutability.is_some() {
            if let (Some(id), Some((l, _))) = (path_ident(&i.expr), span_lines(i.expr.as_ref())) {
                self.mutations.push((id, l));
            }
        }
        syn::visit::visit_expr_reference(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast syn::ExprMethodCall) {
        // syn cannot see &self vs &mut self — treat every plain-ident
        // receiver as mutability-requiring (over-marking, conservative).
        if let (Some(id), Some((l, _))) =
            (path_ident(&i.receiver), span_lines(i.receiver.as_ref()))
        {
            self.mutations.push((id, l));
        }
        syn::visit::visit_expr_method_call(self, i);
    }

    fn visit_local(&mut self, i: &'ast syn::Local) {
        if let syn::Pat::Ident(pi) = &i.pat {
            if let Some((s, e)) = span_lines(&i.pat) {
                self.lets.push((pi.ident.to_string(), s, e));
            }
        }
        syn::visit::visit_local(self, i);
    }
}

/// A kept edit the baseline provably did not compile.
#[derive(Debug, Clone, PartialEq)]
pub struct CfgUnverified {
    pub path: PathBuf,
    pub line: usize,
    pub rule_id: String,
    /// Feature atoms in the enclosing gates that are NOT enabled — the
    /// re-run suggestion names exactly these.
    pub not_enabled: Vec<String>,
    /// Raw gate text for regions that could not be settled (undecidable
    /// atoms, or a gate that is OFF for a non-additive reason such as
    /// `not(feature = …)` with the feature ON).
    pub undecidable: Vec<String>,
    /// The local binding whose mutation evidence lives behind a gate the
    /// baseline did not compile (the binding-association path; `None` for
    /// direct span marks).
    pub binding: Option<String>,
}

/// Audit a batch of file edits against the feature set the verify gate
/// compiled with. Pure syn pass over the FIXED content — no cargo, no I/O
/// (the caller owns manifest reading; line numbers are the
/// [`EditAtLine::final_line`]s, consistent with `FileEditSet::fixed`).
///
/// Divergences (documented, conservative direction — an unmarked edit
/// never has mutability evidence behind a gate this baseline skipped):
/// - `#[cfg] mod foo;` gating a whole child FILE is invisible to a
///   per-file parse (the audit sees only files it is handed);
/// - same-line siblings behind different gates conjoin (over-marking);
/// - `#[cfg_attr(...)]`, struct-field gates, and cfg on inner non-statement
///   expressions are out of scope;
/// - binding association is NAME-based, single-ident `let`s only: tuple /
///   struct-pattern lets and fn PARAMS are not associated (v1),
///   method-call receivers count as mutations without &self/&mut-self
///   visibility, mutations inside macro bodies are invisible, and
///   shadowed same-name bindings may over-mark — every miss is in the
///   mark direction, never in the silent direction.
#[must_use]
pub fn cfg_unverified_edits(files: &[FileEditSet], set: &FeatureSet) -> Vec<CfgUnverified> {
    let mut marks: Vec<CfgUnverified> = Vec::new();
    for f in files {
        let Ok(parsed) = syn::parse_file(&f.fixed) else {
            continue; // the verifier already guarantees parseability
        };
        let mut regions: Vec<CfgRegion> = Vec::new();
        for item in &parsed.items {
            item_cfg_regions(item, &mut regions);
        }
        if regions.is_empty() {
            continue;
        }
        let scan = BindingScan::of(&parsed);
        for e in &f.edits {
            let mut not_enabled: Vec<String> = Vec::new();
            let mut undecidable: Vec<String> = Vec::new();
            let mut binding: Option<String> = None;
            for r in &regions {
                if e.final_line < r.start_line || e.final_line > r.end_line {
                    continue;
                }
                if let Some((missing, und)) = region_gap(r, set) {
                    for m in missing {
                        if !not_enabled.contains(&m) {
                            not_enabled.push(m);
                        }
                    }
                    for u in und {
                        if !undecidable.contains(&u) {
                            undecidable.push(u);
                        }
                    }
                }
            }
            // The P13 binding path: the edit rewrites a `let` whose
            // binding is mutated inside an uncovered region — the
            // `unused_mut` casualty shape, where the edit sits outside
            // the gate and only the load-bearing usage lives inside it.
            for (ident, ls, le) in &scan.lets {
                if e.final_line < *ls || e.final_line > *le {
                    continue;
                }
                for (m_ident, m_line) in &scan.mutations {
                    if m_ident != ident || *m_line == e.final_line {
                        continue;
                    }
                    for r in &regions {
                        if *m_line < r.start_line || *m_line > r.end_line {
                            continue;
                        }
                        if let Some((missing, und)) = region_gap(r, set) {
                            for m in missing {
                                if !not_enabled.contains(&m) {
                                    not_enabled.push(m);
                                }
                            }
                            for u in und {
                                if !undecidable.contains(&u) {
                                    undecidable.push(u);
                                }
                            }
                            if binding.is_none() {
                                binding = Some(ident.clone());
                            }
                        }
                    }
                }
            }
            if !not_enabled.is_empty() || !undecidable.is_empty() {
                not_enabled.sort();
                undecidable.sort();
                marks.push(CfgUnverified {
                    path: f.path.clone(),
                    line: e.final_line,
                    rule_id: e.rule_id.clone(),
                    not_enabled,
                    undecidable,
                    binding,
                });
            }
        }
    }
    marks
}

/// Render the audit tail for the report (one line per mark + the re-run
/// suggestion). Testable render: the bin prints this verbatim.
#[must_use]
pub fn render_cfg_audit(marks: &[CfgUnverified], verify_args: Option<&str>) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    if marks.is_empty() {
        return out;
    }
    let _ = writeln!(
        out,
        "verify: cfg audit — {} kept edit(s) behind cfg gate(s) this baseline did NOT compile:",
        marks.len()
    );
    for m in marks {
        let mut reasons: Vec<String> = Vec::new();
        if let Some(b) = &m.binding {
            reasons.push(format!(
                "binding `{b}` is assigned/mutated inside a gate this baseline did not compile"
            ));
        }
        if !m.not_enabled.is_empty() {
            reasons.push(format!(
                "#[cfg(feature = …)] not enabled: {}",
                m.not_enabled.join(", ")
            ));
        }
        for u in &m.undecidable {
            reasons.push(format!("gate not settleable: cfg({u})"));
        }
        let _ = writeln!(
            out,
            "  {}:{} [{}] — {}",
            m.path.display(),
            m.line, m.rule_id, reasons.join("; ")
        );
    }
    let missing: Vec<String> = {
        let mut all: Vec<String> = marks.iter().flat_map(|m| m.not_enabled.clone()).collect();
        all.sort();
        all.dedup();
        all
    };
    if missing.is_empty() {
        let _ = writeln!(
            out,
            "  (no --features re-run covers these — verify the listed gates manually)"
        );
    } else {
        let features = missing.join(",");
        let suggested = match verify_args {
            Some(a) if !a.trim().is_empty() => format!("{a} --features {features}"),
            _ => format!("--features {features}"),
        };
        let _ = writeln!(
            out,
            "  re-run for full coverage: cargo heal --fix --write --verify --verify-args \"{suggested}\""
        );
    }
    out
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
        let multiline =
            "[features]\ndefault = [\n    \"a\",\n    \"b\",\n]\na = []\nb = []\nc = []\n";
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
        let reverted_rules: Vec<&str> =
            report.reverted.iter().map(|r| r.rule_id.as_str()).collect();
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

    /// Issue 841 kill-path e2e: a check that exits non-zero with JSON
    /// artifact lines but NO error diagnostics must REFUSE — restore
    /// everything, keep nothing — never read as a clean pass. Before the
    /// fix, the killed re-check fell into `merge_error_locations` with an
    /// empty parse, `error_lines` stayed empty, the loop broke, and every
    /// still-applied edit was tallied as verified. Driven by a fake cargo
    /// (first invocation = the baseline: exit 0; every later one: one
    /// artifact JSON line + exit 1 — exactly the mid-check-death shape), so
    /// this runs in the normal suite: no compilation, milliseconds.
    #[test]
    #[cfg(unix)]
    fn fix_verify_kill_path_check_refuses_and_restores() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = std::env::temp_dir().join(format!("fv_kill_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let src_dir = tmp.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            tmp.join("Cargo.toml"),
            "[package]\nname = \"fv_kill_fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
             [lib]\npath = \"src/lib.rs\"\n",
        )
        .unwrap();

        let original = "pub fn existing() -> usize {\n    1\n}\n";
        std::fs::write(src_dir.join("lib.rs"), original).unwrap();

        // Fake cargo: the FIRST invocation (the baseline) exits 0 silently;
        // every later one (the post-edit re-check) emits an artifact JSON
        // line and exits 1 with no error diagnostics — the killed-after-
        // flushing-artifacts shape `is_json_line` lets past the Err arm.
        let marker = tmp.join("baseline_done");
        let script = tmp.join("fake_cargo.sh");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\n\
                 if [ -f \"{}\" ]; then\n\
                  echo '{{\"reason\":\"compiler-artifact\",\"package_id\":\"fv_kill\"}}'\n\
                  exit 1\n\
                 fi\n\
                 touch \"{}\"\n\
                 exit 0\n",
                marker.display(),
                marker.display(),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let fixed = original.replace("    1\n", "    2\n");
        let set = FileEditSet {
            path: src_dir.join("lib.rs"),
            original: original.to_string(),
            fixed,
            edits: vec![edit("some-rule", 2, "2", "1")],
        };
        let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[set]);

        assert!(report.ran);
        assert!(
            report.baseline_clean,
            "baseline must pass so the loop path is exercised: {report:?}"
        );
        assert!(
            report.n_checks >= 2,
            "the post-edit re-check must have run: {report:?}"
        );
        let err = report
            .error
            .as_deref()
            .expect("an incomplete check must refuse, not keep");
        assert!(
            err.contains("did not complete"),
            "the error names the incomplete check: {err}"
        );
        assert_eq!(
            report.kept_edits, 0,
            "nothing kept on an incomplete check: {report:?}"
        );
        assert_eq!(report.kept_files, 0);
        assert!(
            report.reverted.is_empty(),
            "a refusal restores whole, it does not attribute: {report:?}"
        );
        // Disk: the file is back to its original bytes — nothing unverified
        // left behind.
        let on_disk = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap();
        assert_eq!(on_disk, original, "tree restored");
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
        std::fs::write(tmp.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
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

    // ── the post-heal cfg audit (2026-09-10 intake) ─────────────────────

    fn fset(names: &[&str]) -> FeatureSet {
        FeatureSet {
            all_on: false,
            names: names.iter().map(|n| (*n).to_string()).collect(),
        }
    }

    fn expr_of(src: &str) -> CfgExpr {
        let file = syn::parse_file(src).expect("parses");
        let item = &file.items[0];
        let attr = item_attrs(item)
            .iter()
            .find(|a| a.path().is_ident("cfg"))
            .expect("item carries a cfg attr");
        cfg_expr_from_attr(attr).expect("cfg parses")
    }

    fn batch(fixed: &str, edits: &[(&str, usize)]) -> FileEditSet {
        FileEditSet {
            path: PathBuf::from("src/probe.rs"),
            original: fixed.to_string(),
            fixed: fixed.to_string(),
            edits: edits
                .iter()
                .map(|(rule, line)| EditAtLine {
                    rule_id: (*rule).to_string(),
                    final_line: *line,
                    replacement: String::new(),
                    restore: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn cfg_eval_feature_gates() {
        let set = fset(&["a"]);
        // Feature atoms: on → True, off → False.
        assert_eq!(
            eval_cfg(&expr_of("#[cfg(feature = \"a\")] fn f() {}"), &set),
            Tv::True
        );
        assert_eq!(
            eval_cfg(&expr_of("#[cfg(feature = \"b\")] fn f() {}"), &set),
            Tv::False
        );
        // all_on short-circuits every feature atom to True.
        let all = FeatureSet {
            all_on: true,
            names: std::collections::HashSet::default(),
        };
        assert_eq!(
            eval_cfg(&expr_of("#[cfg(feature = \"b\")] fn f() {}"), &all),
            Tv::True
        );
    }

    #[test]
    fn cfg_eval_nested_not_all_any() {
        let set = fset(&["a"]);
        // not(feature) inverts: a ON → provably NOT compiled.
        assert_eq!(
            eval_cfg(
                &expr_of("#[cfg(not(feature = \"a\"))] fn f() {}"),
                &set
            ),
            Tv::False
        );
        // any(one_off, unknown_key) → Unknown (an unknown atom could make
        // it true; a False alone cannot).
        assert_eq!(
            eval_cfg(
                &expr_of("#[cfg(any(feature = \"b\", doc))] fn f() {}"),
                &set
            ),
            Tv::Unknown
        );
        // all(feature_on, unknown) → Unknown; all(feature_off, unknown) → False.
        assert_eq!(
            eval_cfg(
                &expr_of("#[cfg(all(feature = \"a\", doc))] fn f() {}"),
                &set
            ),
            Tv::Unknown
        );
        assert_eq!(
            eval_cfg(
                &expr_of("#[cfg(all(feature = \"b\", doc))] fn f() {}"),
                &set
            ),
            Tv::False
        );
    }

    #[test]
    fn cfg_eval_test_and_host_atoms() {
        let set = fset(&[]);
        // --all-targets compiles #[cfg(test)] → verified.
        assert_eq!(eval_cfg(&expr_of("#[cfg(test)] fn f() {}"), &set), Tv::True);
        // The healer settles host atoms from its own build; the gate
        // compiles for the host it runs on.
        let host_os = std::env::consts::OS;
        let src = format!("#[cfg(target_os = \"{host_os}\")] fn f() {{}}");
        assert_eq!(eval_cfg(&expr_of(&src), &set), Tv::True);
        let wrong = if host_os == "macos" { "linux" } else { "macos" };
        let src = format!("#[cfg(target_os = \"{wrong}\")] fn f() {{}}");
        assert_eq!(eval_cfg(&expr_of(&src), &set), Tv::False);
        // Bare unix/windows: settled by cfg!.
        let unix_tv = eval_cfg(&expr_of("#[cfg(unix)] fn f() {}"), &set);
        assert_eq!(unix_tv, if std::cfg!(unix) { Tv::True } else { Tv::False });
    }

    #[test]
    fn manifest_feature_names_lists_defaults_and_keys() {
        let manifest = "\n[features]\ndefault = [\"a\",\n \"b\"]\na = []\nb = []\nc = []\n";
        let (defaults, keys) = manifest_feature_names(manifest);
        assert_eq!(defaults, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(keys, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        // The count consumer keeps its Issue-031-H3 shape.
        assert_eq!(manifest_nondefault_feature_count(manifest), 1);
    }

    #[test]
    fn feature_set_from_args_matrix() {
        let manifest = "[features]\ndefault = [\"a\"]\na = []\nb = []\nc = []\n";
        // No args → defaults only.
        let set = feature_set_from_args(manifest, None);
        assert!(set.contains("a") && !set.contains("b"));
        // --features b,c (space form + comma form) → added; foreign names
        // (dep paths, unlisted names) ignored.
        let set = feature_set_from_args(manifest, Some("--features b,c somedep/x"));
        assert!(set.contains("a") && set.contains("b") && set.contains("c"));
        assert!(!set.contains("somedep/x"));
        // --features=b form.
        let set = feature_set_from_args(manifest, Some("--features=b"));
        assert!(set.contains("b"));
        // --no-default-features drops defaults but keeps explicit requests.
        let set = feature_set_from_args(manifest, Some("--no-default-features --features b"));
        assert!(!set.contains("a") && set.contains("b"));
        // --all-features → everything on.
        let set = feature_set_from_args(manifest, Some("--all-features"));
        assert!(set.all_on && set.contains("b") && set.contains("c"));
    }

    #[test]
    fn cfg_audit_marks_gated_edits_only() {
        // The E0631 casualty shape: a fix landing inside a feature-gated
        // fn reports as "verified" while the gate never compiled it.
        let fixed = "pub fn keep(r: &String) -> bool { r.starts_with('v') }\n\n#[cfg(feature = \"score_bench\")]\npub fn pick(rules: &[String]) -> Vec<String> {\n    rules.iter().filter(|r| keep(r)).cloned().collect()\n}\n";
        let set = fset(&[]);
        let marks = cfg_unverified_edits(
            &[batch(fixed, &[("needless_return", 2), ("needless_return", 5)])],
            &set,
        );
        assert_eq!(marks.len(), 1, "{marks:?}");
        assert_eq!(marks[0].line, 5);
        assert_eq!(marks[0].not_enabled, vec!["score_bench".to_string()]);
        assert!(marks[0].undecidable.is_empty());
        // The render names the edit + the exact re-run.
        let rendered = render_cfg_audit(&marks, None);
        assert!(rendered.contains("src/probe.rs:5 [needless_return]"));
        assert!(rendered.contains("#[cfg(feature = …)] not enabled: score_bench"));
        assert!(
            rendered
                .contains("--verify-args \"--features score_bench\""),
            "{rendered}"
        );
    }

    #[test]
    fn cfg_audit_test_and_enabled_gates_stay_unmarked() {
        // The host-gate term must be TRUE on every host this test runs on — a bare
        // `all(unix, ..)` compiles the region to nothing on Windows and the
        // (correct) undecidable mark fails the test there (the "a platform
        // is part of the claim" class; caught on the 4090 box, 2026-09-14,
        // all-features lib run). `any(unix, windows)` keeps the intent — a
        // host-conjunct gate with the feature enabled stays unmarked — true
        // on BOTH hosts, and exercises the evaluator any() arm for free.
        let fixed = "#[cfg(test)]\nmod tests {\n    #[cfg(feature = \"a\")]
    fn inner() -> u32 { 1 }
}\n\n#[cfg(all(any(unix, windows), feature = \"a\"))]\npub fn host_gated() -> u32 { 2 }
";
        let set = fset(&["a"]);
        // Edits at lines inside #[cfg(test)] and an enabled gate: no marks.
        let marks = cfg_unverified_edits(
            &[batch(fixed, &[("needless_return", 4), ("needless_return", 8)])],
            &set,
        );
        assert!(marks.is_empty(), "{marks:?}");
    }

    #[test]
    fn cfg_audit_stmt_block_gate_pvp_karma_shape() {
        // The P13 casualty shape: a cfg'd BLOCK inside a fn body — the
        // region a plain item-level walk cannot see.
        let fixed = "pub fn boot() -> u32 {\n    let mut slots = 0;\n    #[cfg(feature = \"pvp_karma\")]\n    {\n        slots = 1;\n    }\n    slots\n}\n";
        let set = fset(&["seal_sextet"]);
        // The edit on the `let mut` line (line 2) — the P13 incident's
        // EXACT edit (an unused_mut heal dropping the load-bearing
        // `mut`): the binding's re-assignment lives inside the uncovered
        // region, so the edit marks via BINDING ASSOCIATION. v1 pinned
        // this line unmarked ("line 2 must NOT — it is above the block
        // — pinning the span boundary"); v2 moves that boundary to
        // answer the P13 intake row's literal question. Line 8 (the
        // fn's closing brace) stays unmarked — not a `let`, not a use.
        let marks = cfg_unverified_edits(
            &[batch(fixed, &[("unused_mut", 2), ("needless_return", 8)])],
            &set,
        );
        assert_eq!(marks.len(), 1, "{marks:?}");
        assert_eq!(marks[0].line, 2);
        assert_eq!(marks[0].not_enabled, vec!["pvp_karma".to_string()]);
        assert_eq!(marks[0].binding.as_deref(), Some("slots"));
        // An edit on the trailing read (line 7) — not a `let`, not
        // inside the region — stays unmarked: the binding path is
        // bounded to declaration edits.
        let marks = cfg_unverified_edits(&[batch(fixed, &[("identity_op", 7)])], &set);
        assert!(marks.is_empty(), "{marks:?}");
        // An edit INSIDE the block region (line 5) marks via span.
        let marks = cfg_unverified_edits(&[batch(fixed, &[("identity_op", 5)])], &set);
        assert_eq!(marks.len(), 1, "{marks:?}");
        assert_eq!(marks[0].not_enabled, vec!["pvp_karma".to_string()]);
        assert!(marks[0].binding.is_none());
    }

    #[test]
    fn cfg_audit_binding_method_call_and_mut_borrow() {
        // vec-push receiver + &mut borrow inside uncovered regions both
        // make an edit on the binding's `let` unverified — the
        // mutability evidence lives behind the gate.
        let fixed = "pub fn f(v: &mut Vec<u8>) {\n    let mut tokens = vec![0u8];\n    let mut total = 0u32;\n    #[cfg(feature = \"quest\")]\n    {\n        tokens.push(1);\n    }\n    #[cfg(feature = \"karma\")]\n    {\n        let r = &mut total;\n    }\n    let _ = (v, total);\n}\n";
        let set = fset(&[]);
        let marks = cfg_unverified_edits(
            &[batch(fixed, &[("unused_mut", 2), ("unused_mut", 3)])],
            &set,
        );
        assert_eq!(marks.len(), 2, "{marks:?}");
        assert_eq!(marks[0].line, 2);
        assert_eq!(marks[0].not_enabled, vec!["quest".to_string()]);
        assert_eq!(marks[0].binding.as_deref(), Some("tokens"));
        assert_eq!(marks[1].line, 3);
        assert_eq!(marks[1].not_enabled, vec!["karma".to_string()]);
        assert_eq!(marks[1].binding.as_deref(), Some("total"));
        let rendered = render_cfg_audit(&marks, None);
        assert!(rendered.contains("binding `tokens`"), "{rendered}");
        assert!(rendered.contains("--features karma,quest"), "{rendered}");
    }

    #[test]
    fn cfg_audit_binding_under_enabled_feature_is_clean() {
        // Mutation behind an ENABLED gate — the baseline compiled the
        // evidence; the declaration edit stays unmarked.
        let fixed = "pub fn f() -> u32 {\n    let mut x = 0;\n    #[cfg(feature = \"on\")]\n    {\n        x = 1;\n    }\n    x\n}\n";
        let set = fset(&["on"]);
        let marks = cfg_unverified_edits(&[batch(fixed, &[("unused_mut", 2)])], &set);
        assert!(marks.is_empty(), "{marks:?}");
    }

    #[test]
    fn cfg_audit_binding_mutation_outside_any_region_is_clean() {
        // Same mutation shape with NO gate at all — the baseline compiled
        // the re-assignment; removing `mut` would have failed the gate.
        let fixed = "pub fn f() -> u32 {\n    let mut x = 0;\n    x = 1;\n    x\n}\n";
        let set = fset(&[]);
        let marks = cfg_unverified_edits(&[batch(fixed, &[("unused_mut", 2)])], &set);
        assert!(marks.is_empty(), "{marks:?}");
    }

    #[test]
    fn cfg_audit_binding_shadowed_name_overmarks_documented() {
        // Name-based association (v1 divergence): the mutated `x` is a
        // DIFFERENT binding than the outer `let mut x`, but the audit
        // marks the outer declaration edit — the conservative direction
        // (a mark asks for a re-run; it never blocks).
        let fixed = "pub fn f() -> u32 {\n    let mut x = 0;\n    #[cfg(feature = \"q\")]\n    {\n        let mut x = 1;\n        x = 2;\n    }\n    x\n}\n";
        let set = fset(&[]);
        let marks = cfg_unverified_edits(&[batch(fixed, &[("unused_mut", 2)])], &set);
        assert_eq!(marks.len(), 1, "{marks:?}");
        assert_eq!(marks[0].binding.as_deref(), Some("x"));
    }

    #[test]
    fn cfg_audit_not_gate_with_feature_on_is_not_features_addable() {
        // cfg(not(feature = "a")) with a ENABLED: provably not compiled, but
        // NO --features re-run can cover it — the report must say so.
        let fixed = "#[cfg(not(feature = \"a\"))]\npub fn off() -> u32 { 1 }\n";
        let set = fset(&["a"]);
        let marks = cfg_unverified_edits(&[batch(fixed, &[("needless_return", 2)])], &set);
        assert_eq!(marks.len(), 1, "{marks:?}");
        assert!(marks[0].not_enabled.is_empty());
        assert_eq!(
            marks[0].undecidable,
            vec!["not(feature = \"a\")".to_string()]
        );
        let rendered = render_cfg_audit(&marks, None);
        assert!(rendered.contains("no --features re-run covers these"));
    }

    #[test]
    fn cfg_audit_partial_verify_args_marks_the_missing_feature() {
        // The pvp_karma scenario: --verify-args passed a partial set; the
        // coarse warning is correctly silent (args present) — the audit is
        // the only honest signal left.
        let fixed = "#[cfg(feature = \"seal_sextet\")]\npub fn a() -> u32 { 1 }\n\n#[cfg(feature = \"pvp_karma\")]\npub fn b() -> u32 { 2 }\n";
        let manifest = "[features]\ndefault = []\nseal_sextet = []\npvp_karma = []\n";
        let set = feature_set_from_args(manifest, Some("--features seal_sextet"));
        let marks = cfg_unverified_edits(
            &[batch(fixed, &[("needless_return", 2), ("needless_return", 5)])],
            &set,
        );
        assert_eq!(marks.len(), 1, "{marks:?}");
        assert_eq!(marks[0].line, 5);
        // The suggested re-run APPENDS to the existing args.
        let rendered = render_cfg_audit(&marks, Some("--features seal_sextet"));
        assert!(
            rendered.contains("--verify-args \"--features seal_sextet --features pvp_karma\""),
            "{rendered}"
        );
    }
}

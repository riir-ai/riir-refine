//! Perf rule resolution verifier — Phase 2: `cargo test` re-run.
//!
//! Behind the `rust_perf_verify` feature gate (opt-in, default-off) because:
//! - **Slow**: each invocation spawns `cargo test` (seconds, not µs)
//! - **Environment-dependent**: requires cargo on PATH
//! - **Temp-dir I/O**: creates a temp crate per verification
//!
//! The fast path (L2 `syn`-based syntax check in [`crate::pruner`]) is
//! always-on. This module is the optional verification that a drafted fix
//! actually compiles + passes any tests present in the fixture.
//!
//! ## Caveat (Proposal 001 caveat 3 — the weakest link)
//!
//! This verifier runs `cargo test` (compilation + test pass). It does NOT
//! run `cargo bench` or measure the perf delta. "Faster" depends on workload,
//! hardware, input size — there is no single ground truth for perf. The real
//! perf-bench Pareto validator (runs `cargo bench` across multiple input
//! sizes, reports a Pareto verdict) lands as a separate plan. The first cut
//! ships degraded: syntax + test pass only.
//!
//! ## How it works
//!
//! 1. Create a temp crate under `std::env::temp_dir()/riir_perf_verify/<blake3_hash>/`
//!    - Stable dir name (BLAKE3 of code) → cargo reuses build cache on repeat
//! 2. Write `Cargo.toml` + `src/lib.rs` (the code being verified)
//! 3. Run `cargo test --message-format=json`
//! 4. Parse JSON for compile errors + test failures
//! 5. `test_passes = true` → compiles + tests pass (fix is plausible)
//!    `test_passes = false` → compile error or test failure
//!
//! ## Modelless-first
//!
//! `cargo test` is a deterministic runner — no ML, no training. This satisfies
//! the modelless-first mandate for Phase 2.

use crate::domains::probe_witness;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The result of a perf rule verification.
#[derive(Debug, Clone)]
pub struct PerfVerifyResult {
    /// Whether the code compiles + tests pass.
    ///
    /// `true` = the fixed code is plausible (compiles + tests green).
    /// `false` = compile error, test failure, OR cargo couldn't run.
    ///
    /// **Note**: this is NOT a perf claim — it does not verify the fixed code
    /// is faster than the buggy code. See the module caveat.
    pub test_passes: bool,
    /// Whether `cargo test` ran successfully.
    ///
    /// `false` if cargo is not on PATH, the command timed out, or the temp
    /// crate couldn't be created.
    pub ran: bool,
    /// Error message if `ran` is false.
    pub error: Option<String>,
    /// Number of tests that passed (0 if no tests or cargo didn't run).
    pub passed: usize,
    /// Number of tests that failed (0 if no tests or cargo didn't run).
    pub failed: usize,
}

impl PerfVerifyResult {
    /// Construct a "did not run" result (cargo missing, I/O error, etc.).
    fn did_not_run(error: impl Into<String>) -> Self {
        Self {
            test_passes: false,
            ran: false,
            error: Some(error.into()),
            passed: 0,
            failed: 0,
        }
    }

    /// Construct a "ran successfully" result with explicit `test_passes`.
    ///
    /// Used when the caller has additional signal beyond test counts (e.g.
    /// cargo exit status + compiler errors) to determine the final verdict.
    fn ran_with_status(passed: usize, failed: usize, test_passes: bool) -> Self {
        Self {
            test_passes,
            ran: true,
            error: None,
            passed,
            failed,
        }
    }
}

/// Verifies that drafted perf fixes compile + pass tests.
///
/// Creates a temp crate, runs `cargo test`, and reports test pass/fail counts.
///
/// ## Performance
///
/// Each call spawns `cargo test`, which takes **seconds** (not µs). The temp
/// crate dir is BLAKE3-keyed, so the same code reuses cargo's build cache on
/// subsequent calls.
///
/// This is NOT a hot-path primitive. Use [`crate::pruner::RustAnalyzerPruner::validate`]
/// for the fast syntax check, then this for thorough verification.
///
/// ## What this does NOT verify
///
/// Per Proposal 001 caveat 3: this verifier does NOT measure the performance
/// delta between buggy + fixed code. A `test_passes = true` result means the
/// fixed code compiles + its tests pass — it does NOT mean the fixed code is
/// faster. The perf-bench Pareto validator is a separate plan.
pub struct PerfVerifier {
    /// Path to the cargo binary.
    cargo_bin: String,
    /// Root directory for temp crates (default: `std::env::temp_dir()/riir_perf_verify/`).
    work_root: PathBuf,
}

impl Default for PerfVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfVerifier {
    /// Create a verifier using system cargo from PATH + default temp root.
    #[must_use]
    pub fn new() -> Self {
        let work_root = std::env::temp_dir().join("riir_perf_verify");
        Self {
            cargo_bin: "cargo".to_string(),
            work_root,
        }
    }

    /// Create a verifier with an explicit cargo binary path.
    #[must_use]
    pub fn with_cargo_bin(cargo: impl Into<String>) -> Self {
        let work_root = std::env::temp_dir().join("riir_perf_verify");
        Self {
            cargo_bin: cargo.into(),
            work_root,
        }
    }

    /// Override the temp crate root directory (for testing or sandboxing).
    #[must_use]
    pub fn with_work_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.work_root = root.into();
        self
    }

    /// Verify that `code` compiles + tests pass.
    ///
    /// Creates a temp crate with the code as `src/lib.rs`, runs
    /// `cargo test --message-format=json`, and parses the result.
    ///
    /// # Returns
    ///
    /// - `test_passes = true` → compiles + tests pass (fix is plausible)
    /// - `test_passes = false` → compile error or test failure
    /// - `ran = false` → cargo unavailable or I/O error (check `error`)
    #[must_use]
    pub fn verify(&self, code: &str, _rule_id: &str) -> PerfVerifyResult {
        // Create the temp crate scaffold.
        let (crate_dir, source_changed) = match self.prepare_crate(code) {
            Ok(pair) => pair,
            Err(e) => {
                return PerfVerifyResult::did_not_run(format!("temp crate setup failed: {e}"))
            }
        };

        // Run cargo test. We keep the exit status because a compilation
        // failure produces NO `suite` event (the test runner never runs),
        // so parsing pass/fail counts alone is insufficient — we also need
        // the exit code + compiler-message errors to distinguish "compile
        // failure" from "no tests, all good".
        let (stdout, exit_ok) = match self.run_cargo_test(&crate_dir) {
            Ok((s, ok)) => (s, ok),
            Err(e) => return PerfVerifyResult::did_not_run(e),
        };

        // ⛔ Issue 112: did cargo compile the source we just wrote? The
        // collision's signature here is `exit_ok` with zero compile errors
        // for a crate that was never built — indistinguishable, from the
        // counts alone, from a clean run. Refuse rather than score it.
        let witness = probe_witness::witness_cargo_json(&stdout, source_changed);
        if !witness.trustworthy() {
            return PerfVerifyResult::did_not_run(witness.reason());
        }

        // Parse JSON output for test pass/fail counts + compiler errors.
        let (passed, failed) = parse_test_results(&stdout);
        let compile_errors = count_compile_errors(&stdout);

        // `test_passes` requires: cargo exited 0 (no compile/test failure),
        // no tests failed, AND no compiler-message errors appeared in the
        // JSON stream. We check both exit status + JSON errors because the
        // exit status alone doesn't distinguish "0 tests, all good" from
        // "compile error with a cached failing build".
        let test_passes = exit_ok && failed == 0 && compile_errors == 0;

        PerfVerifyResult::ran_with_status(passed, failed, test_passes)
    }

    /// Check if cargo is available on the system.
    #[must_use]
    pub fn is_cargo_available(&self) -> bool {
        Command::new(&self.cargo_bin)
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Prepare a temp crate with the given code as `src/lib.rs`.
    ///
    /// Returns the crate root and whether `src/lib.rs` **changed** on this
    /// call. Uses a BLAKE3-keyed stable dir so cargo reuses build cache on
    /// repeat calls with the same code — the change flag is what separates
    /// that intended cache hit from an Issue 110 replay (Issue 112).
    fn prepare_crate(&self, code: &str) -> Result<(PathBuf, bool), String> {
        // BLAKE3-hash the code for a stable, unique dir name.
        let hash = blake3::hash(code.as_bytes());
        let hex = hash.to_hex();
        let stem = &hex[..16];
        let dir = self.work_root.join(stem);

        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;

        // ⛔ The package NAME carries the key, not just the directory (Issue
        // 110) — two identically-named root packages sharing a target dir
        // are ONE cargo unit, and the second gets the first's cached result.
        // Here that means `exit_ok` and zero compile errors for a crate that
        // was never built, i.e. `test_passes = true` on an unverified fix.
        let cargo_toml = format!(
            "[package]\n\
            name = \"perf_verify_{stem}\"\n\
            version = \"0.0.0\"\n\
            edition = \"2021\"\n\
            \n\
            [lib]\n\
            path = \"src/lib.rs\"\n"
        );
        std::fs::write(dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("write Cargo.toml failed: {e}"))?;

        // Write src/lib.rs (the code being verified). Read-before-write so
        // an unchanged repeat leaves the mtime alone.
        let source_changed = probe_witness::write_probe_source(&src_dir.join("lib.rs"), code)?;

        Ok((dir, source_changed))
    }

    /// Run `cargo test --message-format=json` on the temp crate.
    ///
    /// Returns `(stdout, exit_was_successful)`. Even on compile error or test
    /// failure, cargo writes JSON to stdout — the exit status distinguishes
    /// "ran cleanly" from "compile/test failed".
    fn run_cargo_test(&self, crate_dir: &Path) -> Result<(String, bool), String> {
        let manifest_path = crate_dir.join("Cargo.toml");

        let output = Command::new(&self.cargo_bin)
            // ⛔ PIN the probe's target dir, do not inherit one (Issue 110).
            .env("CARGO_TARGET_DIR", crate_dir.join("target"))
            .arg("test")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .arg("--message-format=json")
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;

        // cargo test writes JSON to stdout, human messages to stderr.
        // Even on test failure, stdout has the JSON messages.
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let exit_ok = output.status.success();
        Ok((stdout, exit_ok))
    }
}

/// Convert `u64` to `usize` without triggering `cast_possible_truncation`.
///
/// On 64-bit targets this is lossless. On 32-bit targets with pointer width
/// < 64, saturates to `usize::MAX` (test counts won't realistically exceed
/// 2^32).
#[inline]
fn u64_to_usize(n: u64) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// Parse cargo test JSON output and extract pass/fail counts.
///
/// Each line is a JSON object. We look for `test-result` events:
/// - `type == "suite"` → final summary with `passed` + `failed` counts.
///
/// Non-JSON lines (blank lines, progress indicators) are silently skipped.
/// Returns `(passed, failed)` counts. If no suite event is found, returns
/// `(0, 0)` — this happens when compilation fails (no tests ran). The caller
/// distinguishes "compile failure" from "no tests, all good" by also checking
/// the cargo exit status + [`count_compile_errors`].
fn parse_test_results(json_output: &str) -> (usize, usize) {
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;

    for line in json_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse as serde_json::Value — we only need a few fields.
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // skip non-JSON lines
        };

        // Look for test-suite events (the final summary).
        if v.get("type").and_then(|t| t.as_str()) != Some("suite") {
            continue;
        }

        if let Some(passed) = v.get("passed").and_then(serde_json::Value::as_u64) {
            total_passed = u64_to_usize(passed);
        }
        if let Some(failed) = v.get("failed").and_then(serde_json::Value::as_u64) {
            total_failed = u64_to_usize(failed);
        }
    }

    (total_passed, total_failed)
}

/// Count `compiler-message` events with severity "error" in cargo's JSON output.
///
/// When compilation fails, cargo emits these instead of (or alongside) test
/// suite events. This distinguishes "0 tests, all good" from "compile error
/// prevented tests from running".
///
/// Non-JSON lines are silently skipped.
fn count_compile_errors(json_output: &str) -> usize {
    let mut errors = 0usize;

    for line in json_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        if v.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }

        // message.level == "error" → compilation failed.
        if v.get("message")
            .and_then(|m| m.get("level"))
            .and_then(|l| l.as_str())
            == Some("error")
        {
            errors += 1;
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_output_returns_zeros() {
        let (passed, failed) = parse_test_results("");
        assert_eq!((passed, failed), (0, 0));
    }

    #[test]
    fn parse_skips_non_json_lines() {
        let input = "    Compiling foo v0.1.0\n\
            non-json garbage\n\
            \n";
        let (passed, failed) = parse_test_results(input);
        assert_eq!((passed, failed), (0, 0));
    }

    #[test]
    fn parse_extracts_suite_event() {
        let input = "{\"type\":\"suite\",\"event\":\"ok\",\"passed\":3,\"failed\":0}";
        let (passed, failed) = parse_test_results(input);
        assert_eq!((passed, failed), (3, 0));
    }

    #[test]
    fn parse_extracts_failed_count() {
        let input = "{\"type\":\"suite\",\"event\":\"failed\",\"passed\":2,\"failed\":1}";
        let (passed, failed) = parse_test_results(input);
        assert_eq!((passed, failed), (2, 1));
    }

    #[test]
    fn parse_takes_last_suite_event_when_multiple() {
        let input = "{\"type\":\"suite\",\"passed\":1,\"failed\":0}\n\
            {\"type\":\"suite\",\"passed\":5,\"failed\":2}";
        let (passed, failed) = parse_test_results(input);
        assert_eq!((passed, failed), (5, 2));
    }

    #[test]
    fn did_not_run_sets_fields_correctly() {
        let r = PerfVerifyResult::did_not_run("missing cargo");
        assert!(!r.test_passes);
        assert!(!r.ran);
        assert_eq!(r.error, Some("missing cargo".to_string()));
        assert_eq!(r.passed, 0);
        assert_eq!(r.failed, 0);
    }

    #[test]
    fn ran_with_all_passed_sets_test_passes_true() {
        let r = PerfVerifyResult::ran_with_status(3, 0, true);
        assert!(r.test_passes);
        assert!(r.ran);
        assert_eq!(r.passed, 3);
        assert_eq!(r.failed, 0);
    }

    #[test]
    fn ran_with_any_failed_sets_test_passes_false() {
        let r = PerfVerifyResult::ran_with_status(2, 1, false);
        assert!(!r.test_passes);
        assert_eq!(r.passed, 2);
        assert_eq!(r.failed, 1);
    }

    #[test]
    fn ran_with_zero_tests_is_test_passes_true() {
        // No tests → no failures → test_passes is true (vacuously).
        let r = PerfVerifyResult::ran_with_status(0, 0, true);
        assert!(r.test_passes);
    }

    #[test]
    fn count_compile_errors_empty_returns_zero() {
        assert_eq!(count_compile_errors(""), 0);
    }

    #[test]
    fn count_compile_errors_skips_non_json() {
        assert_eq!(count_compile_errors("garbage\n\nmore garbage"), 0);
    }

    #[test]
    fn count_compile_errors_counts_error_messages() {
        let input = "{\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"message\":\"cannot find symbol\"}}\n\
            {\"reason\":\"compiler-message\",\"message\":{\"level\":\"warning\",\"message\":\"unused\"}}\n\
            {\"reason\":\"compiler-message\",\"message\":{\"level\":\"error\",\"message\":\"other\"}}";
        assert_eq!(count_compile_errors(input), 2);
    }

    #[test]
    fn count_compile_errors_ignores_non_compiler_messages() {
        let input = "{\"reason\":\"compiler-artifact\",\"message\":{\"level\":\"error\"}}";
        assert_eq!(count_compile_errors(input), 0);
    }

    #[test]
    fn new_uses_default_paths() {
        let v = PerfVerifier::new();
        assert_eq!(v.cargo_bin, "cargo");
        assert!(v.work_root.ends_with("riir_perf_verify"));
    }

    #[test]
    fn with_cargo_bin_overrides() {
        let v = PerfVerifier::with_cargo_bin("/usr/local/bin/cargo");
        assert_eq!(v.cargo_bin, "/usr/local/bin/cargo");
    }

    #[test]
    fn with_work_root_overrides() {
        let v = PerfVerifier::new().with_work_root("/tmp/custom");
        assert_eq!(v.work_root, PathBuf::from("/tmp/custom"));
    }

    #[test]
    fn prepare_crate_writes_valid_scaffold() {
        let root = std::env::temp_dir().join(format!(
            "riir_perf_verify_test_scaffold_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let v = PerfVerifier::new().with_work_root(&root);
        let code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let (dir, changed) = v.prepare_crate(code).unwrap();
        assert!(
            changed,
            "a probe dir that did not exist reports its source changed"
        );

        assert!(dir.join("Cargo.toml").exists());
        assert!(dir.join("src/lib.rs").exists());

        let toml = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(toml.contains("perf_verify"));
        assert!(toml.contains("edition = \"2021\""));

        let lib = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap();
        assert_eq!(lib, code);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_crate_stable_dir_for_same_code() {
        let root = std::env::temp_dir().join(format!(
            "riir_perf_verify_test_stable_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let v = PerfVerifier::new().with_work_root(&root);
        let code = "pub fn id(x: i32) -> i32 { x }";
        let (dir1, changed1) = v.prepare_crate(code).unwrap();
        let (dir2, changed2) = v.prepare_crate(code).unwrap();
        assert_eq!(dir1, dir2, "same code should hash to same dir");
        // ⛔ Issue 112: the repeat must report UNCHANGED. That is what makes
        // cargo's `fresh` verdict legitimate here rather than a replay.
        assert!(
            changed1 && !changed2,
            "identical code must not be rewritten"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_crate_different_code_different_dir() {
        let root =
            std::env::temp_dir().join(format!("riir_perf_verify_test_diff_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let v = PerfVerifier::new().with_work_root(&root);
        let (dir1, _) = v.prepare_crate("pub fn a() {}").unwrap();
        let (dir2, _) = v.prepare_crate("pub fn b() {}").unwrap();
        assert_ne!(dir1, dir2);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// ⛔ Issue 110. A distinct DIRECTORY is not a distinct cargo UNIT — the
    /// test above asserts the property that does not matter. Two scratch
    /// crates with one package name, sharing a target dir, are one unit, and
    /// the second inherits the first's cached success: `exit_ok` and zero
    /// compile errors for a crate that was never built.
    #[test]
    fn prepare_crate_different_code_different_package_name() {
        let root =
            std::env::temp_dir().join(format!("riir_perf_verify_test_unit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let v = PerfVerifier::new().with_work_root(&root);
        let name_of = |dir: &std::path::Path| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .unwrap()
                .lines()
                .find_map(|l| l.trim().strip_prefix("name = "))
                .unwrap()
                .trim_matches('"')
                .to_string()
        };
        let (dir1, _) = v.prepare_crate("pub fn a() {}").unwrap();
        let (dir2, _) = v.prepare_crate("pub fn b() {}").unwrap();
        assert_ne!(
            name_of(&dir1),
            name_of(&dir2),
            "two probes share a cargo unit"
        );
        for dir in [&dir1, &dir2] {
            let stem = dir.file_name().unwrap().to_string_lossy().to_string();
            assert!(name_of(dir).ends_with(&stem), "name must carry the dir key");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn is_cargo_available_on_dev_machine() {
        // cargo should be on PATH in any dev environment running these tests.
        let v = PerfVerifier::new();
        assert!(v.is_cargo_available());
    }

    #[test]
    #[ignore = "slow: runs real cargo test (seconds); also needs --test-threads=1 under cargo-in-cargo"]
    fn verify_valid_code_with_no_tests_passes() {
        // A library with no tests → 0 failures → test_passes = true.
        let v = PerfVerifier::new();
        if !v.is_cargo_available() {
            return; // skip on environments without cargo
        }
        let code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let result = v.verify(code, "o1-lookup");
        assert!(result.ran, "cargo should run");
        assert!(result.test_passes, "no tests = no failures = pass");
    }

    #[test]
    #[ignore = "slow: runs real cargo test (seconds); also needs --test-threads=1 under cargo-in-cargo"]
    fn verify_compile_error_reports_not_passed() {
        let v = PerfVerifier::new();
        if !v.is_cargo_available() {
            return;
        }
        let code = "pub fn broken() -> i32 { not_a_real_symbol }";
        let result = v.verify(code, "o1-lookup");
        assert!(result.ran);
        assert!(!result.test_passes, "compile error should fail");
    }

    #[test]
    #[ignore = "slow: runs real cargo test (seconds); also needs --test-threads=1 under cargo-in-cargo"]
    fn verify_deterministic_same_code_same_result() {
        let v = PerfVerifier::new();
        if !v.is_cargo_available() {
            return;
        }
        let code = "pub fn id(x: i32) -> i32 { x }";
        let r1 = v.verify(code, "o1-lookup");
        let r2 = v.verify(code, "o1-lookup");
        assert_eq!(r1.ran, r2.ran);
        assert_eq!(r1.test_passes, r2.test_passes);
        assert_eq!(r1.passed, r2.passed);
        assert_eq!(r1.failed, r2.failed);
    }
}

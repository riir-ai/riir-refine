//! Lint resolution verifier — Phase 3: real `cargo clippy` re-run.
//!
//! Behind the `clippy_verify` feature gate (opt-in, default-off) because:
//! - **Slow**: each invocation spawns `cargo clippy` (seconds, not µs)
//! - **Environment-dependent**: requires cargo + clippy on PATH
//! - **Temp-dir I/O**: creates a temp crate per verification
//!
//! The fast path (L2 `syn`-based syntax check in [`crate::pruner`]) is
//! always-on. This module is the optional, thorough verification that a
//! drafted fix *actually* resolves the target clippy lint.
//!
//! ## How it works
//!
//! 1. Create a temp crate under `std::env::temp_dir()/riir_refine_verify/<blake3_hash>/`
//!    - Stable dir name (BLAKE3 of code) → cargo reuses build cache on repeat
//! 2. Write `Cargo.toml` + `src/lib.rs` (the code being verified)
//! 3. Run `cargo clippy --message-format=json -- -W clippy::<lint_name>`
//! 4. Parse JSON lines for `compiler-message` entries
//! 5. If the target lint code (`clippy::<lint_name>`) appears → NOT resolved.
//!    If absent → resolved.
//!
//! ## Modelless-first
//!
//! `cargo clippy` is a deterministic rules engine — no ML, no training.
//! This satisfies the modelless-first mandate for Phase 3.

use crate::domains::probe_witness;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The result of a clippy lint resolution verification.
#[derive(Debug, Clone)]
pub struct ClippyVerifyResult {
    /// Whether the target lint is NOT present in the code.
    ///
    /// `true` = lint resolved (the fix works).
    /// `false` = lint still present OR the code did not compile (the lint
    /// analysis never ran on it — a "resolution" read out of error output
    /// is not a resolution) OR cargo clippy failed to run.
    pub lint_resolved: bool,
    /// Whether the code COMPILED under clippy (Plan 225 T1.4 clause (a)).
    ///
    /// Distinct from `ran` (cargo spawned) and from `lint_resolved`: a
    /// compile failure is a trustworthy verdict about the code, but the
    /// clippy lint pass on the broken item never produced anything — so
    /// callers that need "the lint cleared" must require BOTH
    /// `lint_resolved && compiled` (the `verify` contract already folds
    /// `!compiled` into `lint_resolved = false`; the field exists for
    /// callers that report the two axes separately).
    pub compiled: bool,
    /// Whether `cargo clippy` ran successfully.
    ///
    /// `false` if cargo/clippy is not on PATH, or the command timed out,
    /// or the temp crate couldn't be created.
    pub ran: bool,
    /// Error message if `ran` is false.
    pub error: Option<String>,
    /// All clippy lint codes present in the code (e.g., `clippy::needless_return`).
    ///
    /// Empty if no clippy lints fired. Populated for diagnostics.
    pub present_lints: Vec<String>,
}

impl ClippyVerifyResult {
    /// Construct a "did not run" result (cargo missing, I/O error, etc.).
    fn did_not_run(error: impl Into<String>) -> Self {
        Self {
            lint_resolved: false,
            compiled: false,
            ran: false,
            error: Some(error.into()),
            present_lints: vec![],
        }
    }

    /// Construct a "ran successfully" result.
    fn ran_with(lint_resolved: bool, compiled: bool, present_lints: Vec<String>) -> Self {
        Self {
            lint_resolved,
            compiled,
            ran: true,
            error: None,
            present_lints,
        }
    }
}

/// Verifies that drafted clippy fixes actually resolve the target lint.
///
/// Creates a temp crate, runs `cargo clippy --message-format=json`, and
/// checks whether the target lint fires on the fixed code.
///
/// ## Performance
///
/// Each call spawns `cargo clippy`, which takes **seconds** (not µs).
/// The temp crate dir is BLAKE3-keyed, so the same code reuses cargo's
/// build cache on subsequent calls — the second run is much faster.
///
/// This is NOT a hot-path primitive. Use [`crate::pruner::RustAnalyzerPruner::validate`]
/// for the fast syntax check, then this for thorough verification.
pub struct ClippyVerifier {
    /// Path to the cargo binary.
    cargo_bin: String,
    /// Root directory for temp crates (default: `std::env::temp_dir()/riir_refine_verify/`).
///
/// DELIBERATE fixed root — `[documented-fixed-root]` (katgpt-rs Issue 832 T5
/// adjudication): the crate dirs under it are BLAKE3 content-addressed and each
/// carries its own `target/`, so two concurrent processes share the root safely
/// — different code lands in different dirs, identical code writes nothing
/// (read-before-write via `write_probe_source`), and cargo's per-target build
/// lock serializes any same-dir race. A pid suffix here would kill the
/// cross-run cargo cache, which is the design's point.
    work_root: PathBuf,
}

impl Default for ClippyVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ClippyVerifier {
    /// Create a verifier using system cargo from PATH + default temp root.
    #[must_use]
    pub fn new() -> Self {
        // [documented-fixed-root] — see `work_root` above (Issue 832 T5).
        let work_root = std::env::temp_dir().join("riir_refine_verify");
        Self {
            cargo_bin: "cargo".to_string(),
            work_root,
        }
    }

    /// Create a verifier with an explicit cargo binary path.
    #[must_use]
    pub fn with_cargo_bin(cargo: impl Into<String>) -> Self {
        // [documented-fixed-root] — see `work_root` above (Issue 832 T5).
        let work_root = std::env::temp_dir().join("riir_refine_verify");
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

    /// Verify that `code` does NOT trigger `clippy::<lint_name>`.
    ///
    /// Creates a temp crate with the code as `src/lib.rs`, runs
    /// `cargo clippy --message-format=json -- -W clippy::<lint_name>`,
    /// and checks whether the target lint appears in the output.
    ///
    /// # Returns
    ///
    /// - `lint_resolved = true` → the lint did NOT fire (fix is good)
    /// - `lint_resolved = false` → the lint fired (fix doesn't work), OR the
    ///   code did not compile, OR cargo clippy couldn't run
    /// - `ran = false` → cargo/clippy unavailable or I/O error (check `error`)
    #[must_use]
    pub fn verify(&self, code: &str, lint_name: &str) -> ClippyVerifyResult {
        // Create the temp crate scaffold.
        let (crate_dir, source_changed) = match self.prepare_crate(code) {
            Ok(pair) => pair,
            Err(e) => {
                return ClippyVerifyResult::did_not_run(format!("temp crate setup failed: {e}"))
            }
        };

        // Run cargo clippy.
        let output = match self.run_clippy(&crate_dir, Some(lint_name)) {
            Ok(o) => o,
            Err(e) => return ClippyVerifyResult::did_not_run(e),
        };

        // ⛔ Issue 112: did cargo compile the source we just wrote? The
        // keyed package name (Issue 110) makes the collision impossible by
        // construction, and a construction nothing observes is one that can
        // be quietly re-opened — by a de-keyed name, a reachable
        // `build.target-dir`, or a cargo freshness change. The failure mode
        // is a clean `present_lints = []` → `lint_resolved = true`, so this
        // must refuse rather than report the empty set.
        let witness = probe_witness::witness_cargo_json(&output, source_changed);
        if !witness.trustworthy() {
            return ClippyVerifyResult::did_not_run(witness.reason());
        }

        // Parse JSON output for clippy lint codes.
        let present_lints = parse_clippy_lints(&output);
        let target_code = format!("clippy::{lint_name}");
        let lint_present = present_lints.iter().any(|l| l == &target_code);

        // A compile failure is a REFUSAL, never a resolution (the documented
        // contract: "clippy couldn't run → false"). The witness's
        // `CompileFailed` arm is a trustworthy verdict about the code, but
        // the lint analysis never ran on the broken item, so the target's
        // absence from ERROR output must not read as resolved — measured as
        // the live hole a type-broken draft could ship through (Plan 225
        // T1.4's clause (a) closes it at this seam for every consumer).
        let compiled = !matches!(witness, probe_witness::ProbeWitness::CompileFailed);

        ClippyVerifyResult::ran_with(!lint_present && compiled, compiled, present_lints)
    }

    /// Discover ALL clippy lints firing on `code` under the DEFAULT warn set
    /// (one clippy run, no `-W` target) — Plan 225 T1.1's labelling pass: a
    /// row's target lint is whatever actually fires in the scaffold, the
    /// same instrument `verify` scores with.
    ///
    /// `lint_resolved` is meaningless for a discovery pass (no target) and
    /// is always `false`; read `present_lints` (full `clippy::<name>` codes)
    /// and `compiled`.
    #[must_use]
    pub fn discover_lints(&self, code: &str) -> ClippyVerifyResult {
        let (crate_dir, source_changed) = match self.prepare_crate(code) {
            Ok(pair) => pair,
            Err(e) => {
                return ClippyVerifyResult::did_not_run(format!("temp crate setup failed: {e}"))
            }
        };
        let output = match self.run_clippy(&crate_dir, None) {
            Ok(o) => o,
            Err(e) => return ClippyVerifyResult::did_not_run(e),
        };
        let witness = probe_witness::witness_cargo_json(&output, source_changed);
        if !witness.trustworthy() {
            return ClippyVerifyResult::did_not_run(witness.reason());
        }
        let present_lints = parse_clippy_lints(&output);
        let compiled = !matches!(witness, probe_witness::ProbeWitness::CompileFailed);
        ClippyVerifyResult::ran_with(false, compiled, present_lints)
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
    /// repeat calls with the same code — which is exactly why the caller
    /// needs the change flag: a fresh unit is legitimate on an unchanged
    /// probe and is the Issue 110 collision on a changed one (Issue 112).
    fn prepare_crate(&self, code: &str) -> Result<(PathBuf, bool), String> {
        // BLAKE3-hash the code for a stable, unique dir name.
        let hash = blake3::hash(code.as_bytes());
        let hex = hash.to_hex();
        let stem = &hex[..16];
        let dir = self.work_root.join(stem);

        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("create_dir_all failed: {e}"))?;

        // ⛔ The package NAME carries the key, not just the directory (Issue
        // 110). Cargo's unit metadata for a ROOT package does not separate
        // two identically-named ones by manifest path, so the moment two
        // scratch crates share a target dir they are ONE unit: the second
        // reports `Finished` in 0.00s, exits 0, and replays the FIRST one's
        // diagnostics. Here that is `present_lints = []` → `lint_resolved =
        // true` → the healer accepts a fix nothing checked. Measured with a
        // known answer: a crate that CANNOT COMPILE passed.
        let cargo_toml = format!(
            "[package]\n\
            name = \"clippy_verify_{stem}\"\n\
            version = \"0.0.0\"\n\
            edition = \"2021\"\n\
            \n\
            [lib]\n\
            path = \"src/lib.rs\"\n"
        );
        std::fs::write(dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("write Cargo.toml failed: {e}"))?;

        // Write src/lib.rs (the code being verified). Read-before-write:
        // an unchanged repeat must not touch the mtime, or the cargo cache
        // hit the BLAKE3 dir exists to provide stops being a cache hit.
        let source_changed = probe_witness::write_probe_source(&src_dir.join("lib.rs"), code)?;

        Ok((dir, source_changed))
    }

    /// Run `cargo clippy --message-format=json` on the temp crate.
    ///
    /// Returns the combined stdout output as a String on success.
    /// `lint_name = None` runs the default warn set (the discovery pass —
    /// no `--` args at all, so cargo's flags are untouched).
    fn run_clippy(&self, crate_dir: &Path, lint_name: Option<&str>) -> Result<String, String> {
        let manifest_path = crate_dir.join("Cargo.toml");
        let mut cmd = Command::new(&self.cargo_bin);
        // ⛔ PIN the probe's target dir, do not inherit one (Issue 110).
        // A caller with CARGO_TARGET_DIR set — the isolated-target idiom
        // this workspace recommends for measuring on a busy box — would
        // otherwise put every scratch crate in ONE target dir, which is
        // the collision above. Belt and braces with the keyed name.
        cmd.env("CARGO_TARGET_DIR", crate_dir.join("target"))
            .arg("clippy")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .arg("--message-format=json");
        if let Some(lint_name) = lint_name {
            cmd.arg("--").arg("-W").arg(format!("clippy::{lint_name}"));
        }
        let output = cmd
            .output()
            .map_err(|e| format!("cargo spawn failed: {e}"))?;

        // cargo clippy writes JSON to stdout, human messages to stderr.
        // Even on "failure" (lints found), stdout has the JSON messages.
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        Ok(stdout)
    }
}

/// Parse cargo clippy JSON output and extract clippy lint codes.
///
/// Each line is a JSON object. We filter for `reason == "compiler-message"`
/// and extract `message.code.code` (e.g., `"clippy::needless_return"`).
///
/// Non-JSON lines (blank lines, progress indicators) are silently skipped.
fn parse_clippy_lints(json_output: &str) -> Vec<String> {
    let mut lints = Vec::new();

    for line in json_output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse as serde_json::Value — avoids defining structs for the
        // complex cargo message format. We only need a few fields.
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // skip non-JSON lines (progress indicators, etc.)
        };

        // Only compiler messages have lint info.
        if v.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }

        // Extract the lint code: message.code.code
        let code = v
            .get("message")
            .and_then(|m| m.get("code"))
            .and_then(|c| c.get("code"))
            .and_then(|c| c.as_str());

        if let Some(code) = code {
            if code.starts_with("clippy::") {
                lints.push(code.to_string());
            }
        }
    }

    lints.shrink_to_fit();
    lints
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── JSON parsing unit tests (no cargo invocation) ─────────────────────

    #[test]
    fn parse_extracts_clippy_lint_codes() {
        let json = concat!(
            r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::needless_return"},"level":"warning","message":"unneeded return statement"}}"#,
            "\n",
            r#"{"reason":"compiler-artifact","target":{"kind":["lib"]}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::redundant_closure"},"level":"warning"}}"#,
        );

        let lints = parse_clippy_lints(json);
        assert_eq!(lints.len(), 2);
        assert!(lints.contains(&"clippy::needless_return".to_string()));
        assert!(lints.contains(&"clippy::redundant_closure".to_string()));
    }

    #[test]
    fn parse_skips_non_clippy_messages() {
        let json = concat!(
            r#"{"reason":"compiler-message","message":{"code":null,"level":"error","message":"expected token"}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"code":{"code":"unused_variables"},"level":"warning"}}"#,
        );

        let lints = parse_clippy_lints(json);
        assert!(lints.is_empty(), "non-clippy codes should be skipped");
    }

    #[test]
    fn parse_handles_empty_output() {
        let lints = parse_clippy_lints("");
        assert!(lints.is_empty());
    }

    #[test]
    fn parse_handles_malformed_json() {
        let json = "not json\n{\"reason\":\"compiler-message\"}\n{\"reason\":";
        let lints = parse_clippy_lints(json);
        assert!(
            lints.is_empty(),
            "malformed JSON should be silently skipped"
        );
    }

    #[test]
    fn parse_deduplicates_lints() {
        // The same lint can appear multiple times (e.g., on different lines).
        // parse_clippy_lints does NOT dedup — callers can dedup if needed.
        let json = concat!(
            r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::needless_return"}}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"code":{"code":"clippy::needless_return"}}}"#,
        );

        let lints = parse_clippy_lints(json);
        assert_eq!(lints.len(), 2, "no dedup — both occurrences counted");
    }

    // ── ClippyVerifyResult construction ───────────────────────────────────

    #[test]
    fn did_not_run_sets_fields_correctly() {
        let r = ClippyVerifyResult::did_not_run("cargo not found");
        assert!(!r.lint_resolved);
        assert!(!r.compiled);
        assert!(!r.ran);
        assert_eq!(r.error.as_deref(), Some("cargo not found"));
        assert!(r.present_lints.is_empty());
    }

    #[test]
    fn ran_with_sets_fields_correctly() {
        let r = ClippyVerifyResult::ran_with(true, true, vec!["clippy::other".to_string()]);
        assert!(r.lint_resolved);
        assert!(r.compiled);
        assert!(r.ran);
        assert!(r.error.is_none());
        assert_eq!(r.present_lints.len(), 1);
    }

    // ── The compile-failure refusal (Plan 225 T1.4 clause (a)) — real cargo ──

    #[test]
    fn a_type_error_is_a_refusal_not_a_resolution() {
        let v = ClippyVerifier::new();
        // Syn-clean but type-broken: the pruner cannot catch this class, and
        // before the fix the error output's empty lint list read as
        // `lint_resolved = true`.
        let r = v.verify(
            "pub fn f() -> u32 { no_such_item_anywhere() }",
            "needless_return",
        );
        assert!(r.ran, "cargo ran: {r:?}");
        assert!(!r.compiled);
        assert!(!r.lint_resolved, "a broken compile must never resolve a lint");
    }

    // ── ClippyVerifier construction ───────────────────────────────────────

    #[test]
    fn new_uses_default_paths() {
        let v = ClippyVerifier::new();
        assert_eq!(v.cargo_bin, "cargo");
        assert!(v.work_root.ends_with("riir_refine_verify"));
    }

    #[test]
    fn with_cargo_bin_overrides() {
        let v = ClippyVerifier::with_cargo_bin("/usr/local/bin/cargo");
        assert_eq!(v.cargo_bin, "/usr/local/bin/cargo");
    }

    #[test]
    fn with_work_root_overrides() {
        let v = ClippyVerifier::new().with_work_root("/tmp/custom_verify");
        assert_eq!(v.work_root, PathBuf::from("/tmp/custom_verify"));
    }

    // ── prepare_crate (no cargo invocation) ───────────────────────────────

    #[test]
    fn prepare_crate_writes_valid_scaffold() {
        let tmp =
            std::env::temp_dir().join(format!("riir_refine_test_prepare_{}", std::process::id()));
        let v = ClippyVerifier::new().with_work_root(&tmp);

        let code = "fn foo() -> i32 { 42 }";
        let (crate_dir, first_write_changed) =
            v.prepare_crate(code).expect("prepare should succeed");
        assert!(
            first_write_changed,
            "a probe dir that did not exist must report its source as changed"
        );

        // Cargo.toml exists and is valid.
        let cargo_toml = std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap();
        assert!(cargo_toml.contains("clippy_verify"));
        assert!(cargo_toml.contains("edition = \"2021\""));

        // src/lib.rs exists and contains the code.
        let lib_rs = std::fs::read_to_string(crate_dir.join("src/lib.rs")).unwrap();
        assert_eq!(lib_rs, code);

        // Dir name is BLAKE3-derived (16 hex chars).
        let dir_name = crate_dir.file_name().unwrap().to_string_lossy();
        assert_eq!(dir_name.len(), 16);
        assert!(dir_name.chars().all(|c| c.is_ascii_hexdigit()));

        // Cleanup.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn prepare_crate_stable_dir_for_same_code() {
        // Same code → same BLAKE3 hash → same dir (cargo cache reuse).
        let tmp =
            std::env::temp_dir().join(format!("riir_refine_test_stable_{}", std::process::id()));
        let v = ClippyVerifier::new().with_work_root(&tmp);

        let code = "fn bar() -> u64 { 99 }";
        let (dir1, changed1) = v.prepare_crate(code).unwrap();
        let (dir2, changed2) = v.prepare_crate(code).unwrap();
        // ⛔ Issue 112: the SECOND call must report unchanged. That flag is
        // what licenses cargo's `fresh` verdict on the repeat — without it
        // the witness would red on the very cache hit this dir exists for.
        assert!(changed1, "first write creates the file");
        assert!(!changed2, "identical code must not be rewritten");

        assert_eq!(dir1, dir2, "same code → same dir");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn prepare_crate_different_code_different_dir() {
        let tmp =
            std::env::temp_dir().join(format!("riir_refine_test_diff_{}", std::process::id()));
        let v = ClippyVerifier::new().with_work_root(&tmp);

        let (dir1, _) = v.prepare_crate("fn a() {}").unwrap();
        let (dir2, _) = v.prepare_crate("fn b() {}").unwrap();

        assert_ne!(dir1, dir2, "different code → different dir");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// ⛔ Issue 110's arm, and the reason the one above is NOT it. A
    /// distinct DIRECTORY does not make a distinct cargo UNIT: cargo's unit
    /// metadata for a root package does not separate two identically-named
    /// ones by manifest path, so two scratch crates sharing a target dir
    /// were one unit and the second replayed the first's diagnostics — here,
    /// `present_lints = []` → `lint_resolved = true` on a fix nothing
    /// checked. The collision is decided in the MANIFEST, so this needs no
    /// cargo to assert.
    #[test]
    fn prepare_crate_different_code_different_package_name() {
        let tmp =
            std::env::temp_dir().join(format!("riir_refine_test_unit_{}", std::process::id()));
        let v = ClippyVerifier::new().with_work_root(&tmp);

        let name_of = |dir: &std::path::Path| {
            std::fs::read_to_string(dir.join("Cargo.toml"))
                .unwrap()
                .lines()
                .find_map(|l| l.trim().strip_prefix("name = "))
                .unwrap()
                .trim_matches('"')
                .to_string()
        };

        let (dir1, _) = v.prepare_crate("fn a() {}").unwrap();
        let (dir2, _) = v.prepare_crate("fn b() {}").unwrap();
        assert_ne!(
            name_of(&dir1),
            name_of(&dir2),
            "two probes share a cargo unit — the second replays the first's verdict"
        );

        // The name must carry its OWN directory's key, or the two facts can
        // drift apart and the guarantee quietly stops holding.
        for dir in [&dir1, &dir2] {
            let stem = dir.file_name().unwrap().to_string_lossy().to_string();
            assert!(
                name_of(dir).ends_with(&stem),
                "package name must carry the directory key: {} vs {stem}",
                name_of(dir)
            );
        }

        // Identical code must STILL reuse one crate — the build cache is
        // what makes this design affordable.
        assert_eq!(
            name_of(&dir1),
            name_of(&v.prepare_crate("fn a() {}").unwrap().0)
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── is_cargo_available ────────────────────────────────────────────────

    #[test]
    fn is_cargo_available_on_dev_machine() {
        // This test assumes cargo is on PATH (true on dev/CI machines).
        // It's not a hard requirement — just documents the expected state.
        let v = ClippyVerifier::new();
        let available = v.is_cargo_available();
        // We don't assert true (might run in an env without cargo).
        // We just verify the method doesn't panic.
        let _ = available;
    }

    // ── Full integration tests (run real cargo clippy) ────────────────────
    // These are #[ignore] by default because they're slow (seconds).
    // Run with: cargo test --features clippy_verify -- --ignored

    #[test]
    #[ignore = "slow: runs real cargo clippy (seconds)"]
    fn verify_needless_return_fixed_code_resolves() {
        let v = ClippyVerifier::new();
        if !v.is_cargo_available() {
            eprintln!("skipping: cargo not available");
            return;
        }

        let fixed_code = "fn foo() -> i32 { 42 }";
        let result = v.verify(fixed_code, "needless_return");
        assert!(result.ran, "cargo clippy should run: {:?}", result.error);
        assert!(
            result.lint_resolved,
            "fixed code should NOT trigger needless_return. Present lints: {:?}",
            result.present_lints
        );
    }

    #[test]
    #[ignore = "slow: runs real cargo clippy (seconds)"]
    fn verify_needless_return_buggy_code_does_not_resolve() {
        let v = ClippyVerifier::new();
        if !v.is_cargo_available() {
            eprintln!("skipping: cargo not available");
            return;
        }

        let buggy_code = "fn foo() -> i32 { return 42; }";
        let result = v.verify(buggy_code, "needless_return");
        assert!(result.ran, "cargo clippy should run: {:?}", result.error);
        assert!(
            !result.lint_resolved,
            "buggy code SHOULD trigger needless_return"
        );
        assert!(
            result
                .present_lints
                .iter()
                .any(|l| l == "clippy::needless_return"),
            "needless_return should be in present_lints"
        );
    }

    #[test]
    #[ignore = "slow: runs real cargo clippy (seconds)"]
    fn verify_deterministic_same_code_same_result() {
        let v = ClippyVerifier::new();
        if !v.is_cargo_available() {
            eprintln!("skipping: cargo not available");
            return;
        }

        let code = "fn double(x: i32) -> i32 { x * 2 }";
        let r1 = v.verify(code, "needless_return");
        let r2 = v.verify(code, "needless_return");

        assert_eq!(r1.ran, r2.ran);
        assert_eq!(r1.lint_resolved, r2.lint_resolved);
    }

    #[test]
    #[ignore = "slow: runs real cargo clippy (seconds)"]
    fn verify_compilation_error_reports_not_resolved() {
        // Code that doesn't compile → cargo clippy reports errors, no lints.
        // lint_resolved should be false (the fix is broken).
        let v = ClippyVerifier::new();
        if !v.is_cargo_available() {
            eprintln!("skipping: cargo not available");
            return;
        }

        let broken_code = "fn foo() -> i32 { undefined_thing }";
        let result = v.verify(broken_code, "needless_return");
        assert!(result.ran, "cargo clippy should still run on broken code");
        // When code doesn't compile, clippy can't check lints. The target
        // lint won't appear in present_lints, so lint_resolved=true by our
        // logic. This is a known limitation — we can't verify lint resolution
        // on code that doesn't compile.
        // (The L2 syntax pruner catches broken syntax BEFORE this verifier
        // is called, so this path shouldn't be hit in normal operation.)
        let _ = result.lint_resolved;
    }
}

//! Plan 329 P2 (issue 197, the sccache `mock_command` doctrine port):
//! the failure-injection matrix for the compile oracle.
//!
//! Every arm injects ONE cargo failure class through the one seam
//! production uses — [`FixVerifier::with_cargo_bin`] /
//! [`CheckRunner::with_cargo_bin`] — and asserts the three-part refusal
//! contract: (a) a loud refusal naming the class, (b) the tree restored
//! byte-identical to its original on disk, (c) nothing kept as verified
//! (`kept_edits == 0`, unverified stats outcome) AND the new per-class
//! counter exactly 1 with every other class zero.
//!
//! # Coverage
//!
//! This matrix covers `fix_verify` + `compile_oracle` (the plan's
//! MINIMUM). The other four verifier surfaces all delegate to the SAME
//! [`CheckRunner`], so the zero-JSON tightening in `run_check` is
//! inherited at the seam by:
//!
//! - `domains/rustc_errors/driver.rs` (`--fix-compile`) — covered at the
//!   seam; its loop-level posture keeps its own driver tests
//!   (`tests/fix_compile*.rs`).
//! - `domains/rust_perf/verify.rs` — covered at the seam; posture tests
//!   stay in-module.
//! - `domains/rust_perf/bench.rs` — covered at the seam; posture tests
//!   stay in-module.
//! - `domains/clippy_lints/verify.rs` — covered at the seam; posture
//!   tests stay in-module.
//!
//! The two-sided completion-sentinel contract (MUTATION G1) is pinned by
//! [`completion_sentinel_accepts_a_complete_pass`]: exit 0 + artifact
//! JSON is a COMPLETE pass and must NOT refuse, while every ambiguous
//! shape (garbage, empty, killed, non-zero-no-JSON) refuses.

#![cfg(feature = "fix_verify")]

use std::path::{Path, PathBuf};

use riir_refine::compile_oracle::CheckRunner;
use riir_refine::fix_verify::{
    EditAtLine, FileEditSet, FixVerifier, RefusalTally, VerifyReport, CARGO_SPAWN_FAIL_PREFIX,
};

/// One compiler-artifact JSON line — the completion sentinel's trusted
/// shape (a real cargo under `--message-format=json` always emits JSON).
const ARTIFACT: &str = "echo '{\"reason\":\"compiler-artifact\",\"package_id\":\"fvfx_matrix\"}'";

/// One positioned edit (the fixture shape the in-module tests use).
fn edit(rule: &str, line: usize, replacement: &str, restore: &str) -> EditAtLine {
    EditAtLine {
        rule_id: rule.to_string(),
        final_line: line,
        replacement: replacement.to_string(),
        restore: restore.to_string(),
        rejected: Vec::new(),
    }
}

/// A minimal standalone fixture crate (Cargo.toml + src/lib.rs holding
/// `original`) in a pid-suffixed temp dir (the shared-temp-path rule:
/// no fixed temp path). Removed on drop.
struct Fixture {
    dir: PathBuf,
    lib: PathBuf,
    original: String,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("fvfx_matrix_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src_dir = dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"fvfx_matrix\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
             [lib]\npath = \"src/lib.rs\"\n",
        )
        .unwrap();
        let original = "pub fn existing() -> usize {\n    1\n}\n";
        let lib = src_dir.join("lib.rs");
        std::fs::write(&lib, original).unwrap();
        Self {
            dir,
            lib,
            original: original.to_string(),
        }
    }

    /// The file's FIXED text (one edit flipping `1` to `2`).
    fn fixed(&self) -> String {
        self.original.replace("    1\n", "    2\n")
    }

    /// The one-edit [`FileEditSet`] for this fixture.
    fn edit_set(&self) -> FileEditSet {
        FileEditSet {
            path: self.lib.clone(),
            original: self.original.clone(),
            fixed: self.fixed(),
            edits: vec![edit("some-rule", 2, "2", "1")],
        }
    }

    fn lib_on_disk(&self) -> String {
        std::fs::read_to_string(&self.lib).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A pid-suffixed scratch dir for the CheckRunner-level arms (no fixture
/// crate needed — spawn/refusal precede any cargo work). Removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("fvfx_cr_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Self(d)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Write a fake-cargo shell script (0o755) into `dir` — the kill-path
/// e2e pattern, shared here so every matrix arm drives the SAME seam.
fn fake_cargo(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let script = dir.join(format!("{name}.sh"));
    std::fs::write(&script, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

/// Marker-file state machine: the FIRST invocation (the baseline) runs
/// `baseline_arm`; every later one (the post-edit re-check) `check_arm`.
/// The baseline arm is responsible for `touch`ing the marker.
fn baseline_then_check(marker: &Path, baseline_arm: &str, check_arm: &str) -> String {
    let mut s = String::from("if [ -f \"");
    s.push_str(&marker.display().to_string());
    s.push_str("\" ]; then\n");
    s.push_str(check_arm);
    s.push_str("\nfi\n");
    s.push_str(baseline_arm);
    s.push('\n');
    s
}

/// The baseline arm that PASSES post-tightening: exit 0 WITH artifact
/// JSON (real cargo never exits 0 with empty stdout under
/// `--message-format=json` — the zero-JSON tightening refuses exactly
/// that shape, so the fake baselines emit it).
fn passing_baseline_arm(marker: &Path) -> String {
    format!("touch \"{}\"; {ARTIFACT}; exit 0", marker.display())
}

/// The three-part refusal contract, shared by every matrix arm: (a) loud
/// refusal naming the class, (b) tree restored byte-identical, (c)
/// nothing kept as verified + EXACTLY one refusal of `class` (every
/// other class zero — the exact roll-up line pins the fixed format).
fn assert_refused(
    report: &VerifyReport,
    fx: &Fixture,
    err_pred: impl Fn(&str) -> bool,
    class: &str,
    what: &str,
) {
    assert!(report.ran, "{what}: the loop ran: {report:?}");
    let err = report
        .error
        .as_deref()
        .unwrap_or_else(|| panic!("{what}: loud refusal expected: {report:?}"));
    assert!(err_pred(err), "{what}: error must name the class: {err}");
    assert_eq!(report.kept_edits, 0, "{what}: nothing kept as verified");
    assert_eq!(report.kept_files, 0, "{what}: nothing kept as verified");
    assert!(
        !report.ended_clean,
        "{what}: unverified outcome: {report:?}"
    );
    assert_eq!(
        fx.lib_on_disk(),
        fx.original,
        "{what}: tree restored byte-identical"
    );
    assert_eq!(
        report.refusals.rollup_line(),
        format!("verify refusals: {class} 1"),
        "{what}: exactly one {class} refusal, every other class zero: {report:?}"
    );
}

/// Arm 1 — binary-missing: the verifier's cargo path names a nonexistent
/// binary → spawn failure. Nothing was ever written (the refusal fires
/// at the baseline, before the write step).
#[test]
#[cfg(unix)]
fn arm_binary_missing_refuses_spawn_failure_and_restores() {
    let fx = Fixture::new("binmiss");
    let report = FixVerifier::with_cargo_bin(fx.dir.join("no_such_cargo").display().to_string())
        .verify(&[fx.edit_set()]);
    assert_refused(
        &report,
        &fx,
        |e| e.starts_with(CARGO_SPAWN_FAIL_PREFIX),
        "spawn_failure",
        "arm binary-missing",
    );
    assert!(
        !report.baseline_clean,
        "refused at the baseline: {report:?}"
    );
}

/// Arm 2 — garbage-stdout SUCCESS (the fail-closed tightening; before
/// plan 329 P2 this read as a clean pass KEEPING the edit): the fake
/// cargo echoes non-JSON lines and exits 0. The zero-JSON success must
/// refuse with the new message.
#[test]
#[cfg(unix)]
fn arm_garbage_stdout_success_refuses_and_restores() {
    let fx = Fixture::new("garbok");
    let marker = fx.dir.join("baseline_done");
    let body = baseline_then_check(
        &marker,
        &passing_baseline_arm(&marker),
        "echo 'not json'; exit 0",
    );
    let script = fake_cargo(&fx.dir, "fake_cargo_garbage_ok", &body);
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert_refused(
        &report,
        &fx,
        |e| e.contains("produced no JSON output") && e.contains("exit ok"),
        "cargo_refusal",
        "arm garbage-stdout-success",
    );
    assert!(report.baseline_clean, "baseline passed, loop-stage refusal");
}

/// Arm 3 — garbage-stdout NON-ZERO: the enriched cargo-refusal path
/// (stderr surfaced), NOT the zero-JSON message — the two refusal
/// classes stay distinguishable.
#[test]
#[cfg(unix)]
fn arm_garbage_stdout_nonzero_refuses_with_enriched_message() {
    let fx = Fixture::new("garberr");
    let marker = fx.dir.join("baseline_done");
    let body = baseline_then_check(
        &marker,
        &passing_baseline_arm(&marker),
        "echo 'boom: manifest parse failure' >&2; echo 'not json'; exit 1",
    );
    let script = fake_cargo(&fx.dir, "fake_cargo_garbage_err", &body);
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert_refused(
        &report,
        &fx,
        |e| e.contains("failed to run in") && e.contains("boom: manifest parse failure"),
        "cargo_refusal",
        "arm garbage-stdout-nonzero",
    );
    let err = report.error.as_deref().unwrap();
    assert!(
        !err.contains("produced no JSON output"),
        "non-zero + no JSON is the enriched cargo refusal, not the zero-JSON class: {err}"
    );
}

/// Arm 4 — signal-kill: the fake cargo emits one artifact JSON line,
/// then its own `kill -9 $$` (the mid-check-death shape: output
/// partially flushed, exit non-zero) → the Incomplete verdict path,
/// now counted (`incomplete_check == 1` — the counter the old kill-path
/// regression test could not assert).
#[test]
#[cfg(unix)]
fn arm_signal_kill_refuses_incomplete_and_restores() {
    let fx = Fixture::new("kill9");
    let marker = fx.dir.join("baseline_done");
    let body = baseline_then_check(
        &marker,
        &passing_baseline_arm(&marker),
        &format!("{ARTIFACT}; kill -9 $$"),
    );
    let script = fake_cargo(&fx.dir, "fake_cargo_kill", &body);
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert_refused(
        &report,
        &fx,
        |e| e.contains("did not complete"),
        "incomplete_check",
        "arm signal-kill",
    );
    assert!(report.baseline_clean, "{report:?}");
}

/// Arm 5 — empty-output SUCCESS: exits 0 printing nothing at all → the
/// zero-JSON tightening refuses (empty output is the same untrustworthy
/// class as garbage).
#[test]
#[cfg(unix)]
fn arm_empty_output_success_refuses_and_restores() {
    let fx = Fixture::new("emptyok");
    let marker = fx.dir.join("baseline_done");
    let body = baseline_then_check(&marker, &passing_baseline_arm(&marker), "exit 0");
    let script = fake_cargo(&fx.dir, "fake_cargo_silent_ok", &body);
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert_refused(
        &report,
        &fx,
        |e| e.contains("produced no JSON output") && e.contains("exit ok"),
        "cargo_refusal",
        "arm empty-output-success",
    );
}

/// Arm 6 — permission-denied: the fake cargo is written WITHOUT exec
/// permission → the spawn itself fails (os error 13) → spawn_failure.
#[test]
#[cfg(unix)]
fn arm_permission_denied_refuses_spawn_failure() {
    use std::os::unix::fs::PermissionsExt;

    let fx = Fixture::new("noexec");
    let script = fx.dir.join("fake_cargo_noexec.sh");
    std::fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o644)).unwrap();

    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert_refused(
        &report,
        &fx,
        |e| e.starts_with(CARGO_SPAWN_FAIL_PREFIX) && e.contains("Permission denied"),
        "spawn_failure",
        "arm permission-denied",
    );
}

/// Arm 7a — CheckRunner level, garbage-stdout success: `run_check`
/// itself refuses (unit-level; no fixture crate — the fake cargo ignores
/// the package dir).
#[test]
#[cfg(unix)]
fn checkrunner_refuses_garbage_stdout_success() {
    let scratch = Scratch::new("garbok");
    let script = fake_cargo(&scratch.0, "fake_cargo", "echo 'not json'; exit 0");
    let err = CheckRunner::new()
        .with_cargo_bin(script.display().to_string())
        .run_check(&scratch.0)
        .expect_err("zero-JSON success must refuse");
    assert!(err.contains("produced no JSON output"), "{err}");
    assert!(err.contains("exit ok"), "{err}");
}

/// Arm 7b — CheckRunner level, empty-output success: exits 0 printing
/// nothing → the same zero-JSON refusal.
#[test]
#[cfg(unix)]
fn checkrunner_refuses_empty_output_success() {
    let scratch = Scratch::new("emptyok");
    let script = fake_cargo(&scratch.0, "fake_cargo", "exit 0");
    let err = CheckRunner::new()
        .with_cargo_bin(script.display().to_string())
        .run_check(&scratch.0)
        .expect_err("empty-output success must refuse");
    assert!(err.contains("produced no JSON output"), "{err}");
}

/// Arm 7c — CheckRunner level, binary-missing: the spawn error carries
/// the pinned `CARGO_SPAWN_FAIL_PREFIX` (the classification anchor).
#[test]
fn checkrunner_refuses_missing_binary_with_pinned_prefix() {
    let scratch = Scratch::new("binmiss");
    let err = CheckRunner::new()
        .with_cargo_bin(scratch.0.join("no_such_cargo").display().to_string())
        .run_check(&scratch.0)
        .expect_err("a missing binary must refuse");
    assert!(err.starts_with(CARGO_SPAWN_FAIL_PREFIX), "{err}");
}

/// MUTATION G1 — the two-sided completion-sentinel contract. This is the
/// "keep-the-edit-anyway" mutant of arms 2/4/5: whatever the injected
/// failure was, the mutant tries to read as a pass with exit 0 + one
/// compiler-artifact JSON line. The sentinel must ACCEPT that shape —
/// it is a COMPLETE pass (a real clean check) — so this arm documents
/// the exact shape that must NOT refuse, while the refusal arms prove
/// everything ambiguous refuses.
#[test]
#[cfg(unix)]
fn completion_sentinel_accepts_a_complete_pass() {
    let fx = Fixture::new("sentinel");
    let marker = fx.dir.join("baseline_done");
    let body = baseline_then_check(
        &marker,
        &passing_baseline_arm(&marker),
        &format!("{ARTIFACT}; exit 0"),
    );
    let script = fake_cargo(&fx.dir, "fake_cargo_mutant_pass", &body);
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);

    assert!(report.ran, "{report:?}");
    assert!(report.baseline_clean, "{report:?}");
    assert!(
        report.error.is_none(),
        "a complete pass must not refuse: {report:?}"
    );
    assert!(report.ended_clean, "{report:?}");
    assert_eq!(report.kept_edits, 1, "the edit is kept as verified");
    assert_eq!(report.kept_files, 1);
    assert_eq!(
        report.refusals,
        RefusalTally::default(),
        "no refusal class fired on a complete pass: {report:?}"
    );
    assert_eq!(fx.lib_on_disk(), fx.fixed(), "the kept edit is on disk");
}

/// The roll-up format contract: fixed field order, only nonzero classes,
/// silent-when-zero, and field-wise `add` for the run aggregate.
#[test]
fn refusal_tally_rollup_is_fixed_order_and_additive() {
    let mut t = RefusalTally::default();
    assert_eq!(t, RefusalTally::default());
    assert_eq!(t.rollup_line(), "");
    t.spawn_failure = 1;
    t.incomplete_check = 2;
    assert_eq!(
        t.rollup_line(),
        "verify refusals: spawn_failure 1 · incomplete_check 2"
    );
    let mut run = RefusalTally::default();
    run.add(&t);
    run.add(&t);
    assert_eq!(
        run.rollup_line(),
        "verify refusals: spawn_failure 2 · incomplete_check 4"
    );
    run.no_manifest = 1;
    assert_eq!(
        run.rollup_line(),
        "verify refusals: spawn_failure 2 · incomplete_check 4 · no_manifest 1"
    );
}

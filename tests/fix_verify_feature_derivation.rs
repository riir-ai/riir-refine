//! Issue 259 T3 — the verify loop derives the touched targets'
//! `required-features` per package and verifies AT them (baseline + rounds
//! carry `--features <derived>`), instead of verifying default-only and
//! letting the P43 audit merely DISCLOSE the gap afterwards.
//!
//! Fake-cargo arms (the kill-path e2e pattern from
//! `fix_verify_failure_matrix.rs`): the fake cargo records its argv so the
//! arms assert the exact cargo invocation, not just the report. Unix-only
//! (the fake cargo is a shell script).

#![cfg(unix)]

use riir_refine::fix_verify::{EditAtLine, FileEditSet, FixVerifier, feature_set_from_args};
use std::path::{Path, PathBuf};

fn edit(rule: &str, line: usize, replacement: &str, restore: &str) -> EditAtLine {
    EditAtLine {
        rule_id: rule.to_string(),
        final_line: line,
        replacement: replacement.to_string(),
        restore: restore.to_string(),
        rejected: Vec::new(),
    }
}

/// The scratch fixture: a package whose manifest carries a `[[test]]` row
/// (`required-features = ["mi_probe"]`) owning the edited file, plus a
/// `[[bin]]` row (`required-features = ["mi_probe"]` NOT set) that must
/// NOT leak into the derived set for other files. The fake cargo records
/// every invocation's argv, one line per invocation.
struct GateFixture {
    dir: PathBuf,
    test_file: PathBuf,
    original: String,
    argv_log: PathBuf,
}

impl GateFixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("fvfx_gated_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src_dir = dir.join("tests");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            concat!(
                "[package]\nname = \"fvfx_gated\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
                "[features]\nmi_probe = []\n",
                "[[test]]\nname = \"probe_run\"\npath = \"tests/probe_run.rs\"\n",
                "required-features = [\"mi_probe\"]\n",
            ),
        )
        .unwrap();
        let original = "pub fn existing() -> usize {\n    1\n}\n";
        let test_file = src_dir.join("probe_run.rs");
        std::fs::write(&test_file, original).unwrap();
        let argv_log = dir.join("argv.log");
        Self {
            dir,
            test_file,
            original: original.to_string(),
            argv_log,
        }
    }

    fn fixed(&self) -> String {
        self.original.replace("    1\n", "    2\n")
    }

    fn edit_set(&self) -> FileEditSet {
        FileEditSet {
            path: self.test_file.clone(),
            original: self.original.clone(),
            fixed: self.fixed(),
            edits: vec![edit("mi_probe_rule", 2, "2", "1")],
        }
    }

    /// A fake cargo that appends "$@" to the argv log and then behaves
    /// exactly like the matrix's passing baseline/check pair (the JSON is
    /// ECHOed — the matrix's ARTIFACT pattern; a bare JSON line in sh is a
    /// syntax error and prints nothing).
    fn fake_cargo(&self, name: &str, marker: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let artifact = r#"{"reason":"compiler-artifact","package_id":"fvfx_gated"}"#;
        let body = format!(
            "echo \"$@\" >> \"{argv}\"\nif [ -f \"{m}\" ]; then\n  echo '{artifact}'\n  exit 0\nfi\ntouch \"{m}\"\necho '{artifact}'\nexit 0\n",
            argv = self.argv_log.display(),
            m = marker.display(),
        );
        let script = self.dir.join(format!("{name}.sh"));
        std::fs::write(&script, format!("#!/bin/sh\n{body}")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }
}

impl Drop for GateFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Arm 1 — the derived posture: the verify loop must check AT
/// `--features mi_probe` (the edited file's `[[test]]` row requires it and
/// the invocation posture is default-only), record the package on
/// `verified_features`, and the P43 audit over the AUGMENTED set marks
/// nothing.
#[test]
#[cfg(unix)]
fn arm_derived_features_verify_at_required_features() {
    let fx = GateFixture::new("derived");
    let marker = fx.dir.join("baseline_done");
    let script = fx.fake_cargo("fc_derived", &marker);
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert!(report.ran, "the loop ran: {report:?}");
    assert!(report.baseline_clean, "baseline clean: {report:?}");
    assert!(report.ended_clean, "ended clean: {report:?}");
    assert!(report.error.is_none(), "no error: {report:?}");
    let derived = report
        .verified_features
        .get(&fx.dir)
        .unwrap_or_else(|| panic!("pkg on verified_features: {report:?}"));
    assert_eq!(derived, &["mi_probe"], "the derived set: {report:?}");
    // Every recorded invocation carried the derived features.
    let argv = std::fs::read_to_string(&fx.argv_log).unwrap();
    let lines: Vec<&str> = argv.lines().collect();
    assert!(!lines.is_empty(), "the fake cargo was invoked");
    for line in &lines {
        assert!(
            line.contains("--features mi_probe"),
            "every check runs at the derived posture: {line:?}"
        );
    }
    // The P43 audit over the augmented set: no marks (the audit's own
    // inputs mirror the heal.rs caller — text + the augmented FeatureSet).
    let text = std::fs::read_to_string(fx.dir.join("Cargo.toml")).unwrap();
    let mut set = feature_set_from_args(&text, None);
    set.names.extend(derived.iter().cloned());
    let marks = riir_refine::fix_verify::required_features_unverified_edits(
        &[fx.edit_set()],
        &set,
        &text,
        &fx.dir,
    );
    assert!(
        marks.is_empty(),
        "the augmented posture proves the edit — P43 marks nothing: {marks:?}"
    );
}

/// Arm 2 — pre-broken gated baseline: the unedited tree does not compile
/// AT the derived posture, so the package falls back to the invocation
/// posture (disclosed on `feature_fallbacks`, nothing on
/// `verified_features`) and the P43 audit keeps its marks.
#[test]
#[cfg(unix)]
fn arm_prebroken_gated_baseline_falls_back() {
    let fx = GateFixture::new("fallback");
    let marker = fx.dir.join("baseline_done");
    use std::os::unix::fs::PermissionsExt;
    let artifact = r#"{"reason":"compiler-artifact","package_id":"fvfx_gated"}"#;
    let error_json = r#"{"reason":"compiler-message","message":{"children":[],"message":"pre-broken","spans":[]}}"#;
    let body = format!(
        "echo \"$@\" >> \"{argv}\"\nif echo \"$@\" | grep -q -- --features; then\n  echo '{err}'\n  exit 1\nfi\nif [ -f \"{m}\" ]; then\n  echo '{artifact}'\n  exit 0\nfi\ntouch \"{m}\"\necho '{artifact}'\nexit 0\n",
        argv = fx.argv_log.display(),
        err = error_json,
        m = marker.display(),
    );
    let script = fx.dir.join("fc_fallback.sh");
    std::fs::write(&script, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[fx.edit_set()]);
    assert!(report.ran, "the loop ran: {report:?}");
    assert!(
        report.verified_features.get(&fx.dir).is_none(),
        "the fallback package verifies nothing at the gated posture: {report:?}"
    );
    assert!(
        report.feature_fallbacks.contains(&fx.dir),
        "the fallback is disclosed: {report:?}"
    );
    // The FINAL posture's baseline was clean (the default posture passes),
    // so the loop proceeds — the argv log shows the first (derived)
    // baseline failing, then a re-check WITHOUT --features.
    let argv = std::fs::read_to_string(&fx.argv_log).unwrap();
    let lines: Vec<&str> = argv.lines().collect();
    assert!(
        lines.iter().any(|l| l.contains("--features mi_probe")),
        "the derived attempt ran: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| !l.contains("--features")),
        "the fallback re-check ran at the invocation posture: {lines:?}"
    );
}

/// Arm 3 — an edit in an UNGATED file derives nothing: the loop checks at
/// the invocation posture (no `--features` anywhere in the argv) and
/// `verified_features` stays empty.
#[test]
#[cfg(unix)]
fn arm_ungated_file_derives_nothing() {
    let fx = GateFixture::new("ungated");
    // Point the edit at a lib path the manifest does not gate (the lib
    // implicit row has no required-features).
    let src_dir = fx.dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    let lib = src_dir.join("lib.rs");
    std::fs::write(&lib, fx.original.clone()).unwrap();
    std::fs::remove_file(&fx.test_file).unwrap();
    let marker = fx.dir.join("baseline_done");
    let script = fx.fake_cargo("fc_ungated", &marker);
    let edit = FileEditSet {
        path: lib.clone(),
        original: fx.original.clone(),
        fixed: fx.fixed(),
        edits: vec![edit("plain_rule", 2, "2", "1")],
    };
    let report = FixVerifier::with_cargo_bin(script.display().to_string()).verify(&[edit]);
    assert!(report.ran, "the loop ran: {report:?}");
    assert!(
        report.verified_features.is_empty(),
        "nothing derived for an ungated file: {report:?}"
    );
    let argv = std::fs::read_to_string(&fx.argv_log).unwrap();
    for line in argv.lines() {
        assert!(
            !line.contains("--features"),
            "the invocation posture is unchanged: {line:?}"
        );
    }
}

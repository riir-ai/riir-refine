//! Plan 019 Phase 1 driver e2e — real cargo, real diagnostics, injected
//! test playbooks.
//!
//! The lib-suite tests pin the pure pieces (parse, dedupe, ordering,
//! splice, guard matrix). These tests pin the LOOP end-to-end: a real
//! scaffold crate, a real `cargo check`, real E-codes, and a test
//! playbook that proposes real edits. The driver's keep/revert
//! arbitration is the product — it cannot be unit-tested against a
//! mocked cargo without re-implementing cargo's diagnostic behavior in
//! the mock (the T0.2 lesson: consume the oracle, don't fork it).
//!
//! Spawns `cargo check` per iteration (~seconds per test under load).
//! Like `premise_probe`, gated behind `--features rustc_errors`, skipped
//! (not failed) without cargo on PATH; run explicitly:
//!
//! ```bash
//! cargo test --features rustc_errors --test fix_compile -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use riir_refine::domains::rustc_errors::diag::{Applicability, RustcErrorDiag, SuggestionSpan};
use riir_refine::domains::rustc_errors::driver::{CompileDriver, RevertReason};
use riir_refine::domains::rustc_errors::playbooks::{CompileProposer, FixCandidate, Playbooks};

/// A test playbook: for `code`, replace `from` with `to` (first
/// occurrence in the source). `code == None` proposes nothing (miss arm).
struct ReplacePlaybook {
    code: &'static str,
    from: &'static str,
    to: &'static str,
}

impl CompileProposer for ReplacePlaybook {
    fn name(&self) -> &'static str {
        "test-replace"
    }

    fn code(&self) -> &'static str {
        self.code
    }

    fn propose(&self, diag: &RustcErrorDiag, _file: &Path, source: &str) -> Vec<FixCandidate> {
        if diag.code != self.code {
            return Vec::new();
        }
        let Some(at) = source.find(self.from) else {
            return Vec::new();
        };
        vec![FixCandidate {
            strategy_id: format!("replace-{}-{}", self.from, self.to),
            code: diag.code.clone(),
            file: _file.to_path_buf(),
            line: diag.primary.as_ref().map_or(1, |p| p.1),
            byte_span: (at, at + self.from.len()),
            replacement: self.to.to_string(),
            shape: String::new(),
            semantic_note: None,
            diag_digest: None,
            extra_edits: Vec::new(),
        }]
    }
}

fn diag_shape() -> RustcErrorDiag {
    RustcErrorDiag {
        code: String::new(),
        message: String::new(),
        primary: None,
        suggestions: vec![SuggestionSpan {
            file: String::new(),
            line_start: 0,
            byte_start: None,
            byte_end: None,
            replacement: String::new(),
            applicability: Applicability::Unknown,
            message: String::new(),
        }],
        labeled_spans: Vec::new(),
        rendered: None,
    }
}

/// Scaffold a standalone crate under a fresh temp root (the probe.rs
/// pattern — pid + counter keys the dir so parallel tests never share).
fn scaffold(name: &str, source: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "riir_fix_compile_{}_{seq}_{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fc\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(src.join("lib.rs"), source).unwrap();
    dir
}

fn cargo_available() -> bool {
    std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .arg("--version")
        .output()
        .is_ok()
}

/// E0596 mut case: `let v = Vec::new(); v.push(1);` — `v` is not
/// declared mutable, the push borrows it mutably. The playbook replaces
/// the binding with `let mut v = ...` — a real fix. The driver must KEEP
/// it and reach clean. (An earlier fixture used `let r = v;` with
/// `v: &mut Vec<u8>` — that MOVES the `&mut` into `r`, compiles clean,
/// and never provokes E0596.)
#[test]
fn driver_applies_kept_fix_to_clean() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken =
        "pub fn probe() -> usize {\n    let v = Vec::new();\n    v.push(1);\n    v.len()\n}\n";
    let dir = scaffold("apply", broken);
    let playbooks = Playbooks::empty().with(Box::new(ReplacePlaybook {
        code: "E0596",
        from: "let v = Vec::new();",
        to: "let mut v = Vec::new();",
    }));
    let driver = CompileDriver::isolated().with_max_iters(3);
    let report = driver.run(
        &[dir.join("src").join("lib.rs")],
        &playbooks,
        true, // write
    );
    assert!(report.ran, "cargo ran: {:?}", report.error);
    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.applied.len(), 1, "one kept fix: {report:?}");
    assert!(report.clean, "crate compiles after the fix: {report:?}");
    // Issue 107 T2 — the ring record carries the span embedding of the
    // ORIGINAL trigger text, deterministically reproducible from the
    // pre-fix source (write-time capture).
    assert_eq!(report.attempts.len(), 1);
    let embed = report.attempts[0]
        .span_embed
        .expect("kept-path attempt carries the embedding column");
    assert_eq!(
        embed,
        riir_refine::span_embed::span_embed("let v = Vec::new();"),
        "embedding is a pure function of the attempt-time trigger text"
    );
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(on_disk.contains("let mut v = Vec::new();"), "fix persisted");
    assert!(!on_disk.contains("let v = Vec::new();"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A candidate that does NOT reduce the error count must be reverted
/// byte-exact and marked tried — then the loop stops (no candidates
/// left), reporting the error as remaining.
#[test]
fn driver_reverts_no_progress_candidate() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken =
        "pub fn probe() -> usize {\n    let v = Vec::new();\n    v.push(1);\n    v.len()\n}\n";
    let dir = scaffold("revert", broken);
    let playbooks = Playbooks::empty().with(Box::new(ReplacePlaybook {
        code: "E0596",
        from: "let v = Vec::new();",
        to: "let v = (Vec::new());", // parenthesizes — still E0596, no progress
    }));
    let driver = CompileDriver::isolated().with_max_iters(3);
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks, true);
    assert!(report.ran, "{:?}", report.error);
    assert_eq!(report.applied.len(), 0, "nothing kept: {report:?}");
    assert_eq!(report.reverted.len(), 1, "one revert: {report:?}");
    assert_eq!(report.reverted[0].reason, RevertReason::NoProgress);
    assert_eq!(report.remaining, 1, "the E0596 is still there");
    assert!(!report.clean);
    // Issue 107 T2 — the revert path embeds the same attempt-time text.
    assert!(
        report.attempts[0].span_embed.is_some(),
        "revert-path attempt carries the embedding column"
    );
    // Byte-exact restore.
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(on_disk, broken, "reverted file is byte-identical");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The T2.3 gate, at the driver level: a crate that already compiles →
/// `ran`, `clean`, zero edits, zero writes.
#[test]
fn driver_reports_nothing_on_clean_crate() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let clean = "pub fn probe(v: &mut Vec<u8>) {\n    v.push(1);\n}\n";
    let dir = scaffold("clean", clean);
    let playbooks = Playbooks::empty().with(Box::new(ReplacePlaybook {
        code: "E0596",
        from: "v.push(1);",
        to: "v.push(2);", // WOULD fire if anything were proposed at
    }));
    let driver = CompileDriver::isolated();
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks, true);
    assert!(report.ran);
    assert!(report.clean);
    assert_eq!(report.n_checks, 1, "one check, nothing else");
    assert_eq!(report.iterations, 0);
    assert!(report.applied.is_empty());
    assert!(report.reverted.is_empty());
    assert!(report.planned.is_empty());
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(on_disk, clean, "clean crate untouched");
    let _ = std::fs::remove_dir_all(&dir);
}

/// No playbook matches the code → every diagnostic is a miss, nothing is
/// written, and the loop stops without candidates.
#[test]
fn driver_records_miss_when_no_playbook_matches() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    // E0597 narrow-scope — the uncontested territory (premise probe).
    let broken = "pub fn probe() -> i32 {\n    let r;\n    {\n        let x = 5i32;\n        r = &x;\n    }\n    *r\n}\n";
    let dir = scaffold("miss", broken);
    let playbooks = Playbooks::empty().with(Box::new(ReplacePlaybook {
        code: "E0999", // never matches
        from: "x",
        to: "y",
    }));
    let driver = CompileDriver::isolated();
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks, true);
    assert!(report.ran);
    assert!(!report.clean);
    assert_eq!(report.misses.len(), 1, "the E0597 is a miss: {report:?}");
    assert_eq!(report.misses[0].code, "E0597");
    assert_eq!(report.applied.len() + report.reverted.len(), 0);
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(on_disk, broken, "nothing written");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Review mode: a broken crate with a matching playbook prints a PLAN and
/// writes nothing.
#[test]
fn review_mode_plans_without_writing() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken =
        "pub fn probe() -> usize {\n    let v = Vec::new();\n    v.push(1);\n    v.len()\n}\n";
    let dir = scaffold("review", broken);
    let playbooks = Playbooks::empty().with(Box::new(ReplacePlaybook {
        code: "E0596",
        from: "let v = Vec::new();",
        to: "let mut v = Vec::new();",
    }));
    let driver = CompileDriver::isolated();
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks, false);
    assert!(report.ran);
    assert_eq!(report.planned.len(), 1, "one plan line: {report:?}");
    assert!(
        report.planned[0]
            .replacement_head
            .contains("let mut v = Vec::new();")
    );
    assert_eq!(report.iterations, 0, "no loop in review mode");
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(on_disk, broken, "review mode never writes");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The diag_shape helper exists so future tests can construct
/// diagnostics; pin that it builds (compile-time coverage).
#[test]
fn diag_shape_helper_builds() {
    let d = diag_shape();
    assert_eq!(d.code, "");
}

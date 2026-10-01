//! Plan 019 Phase 2 e2e — the real E0597 playbooks against real cargo.
//!
//! T2.2 is the founding-incident gate: the `settlement_goat.rs` shape
//! (`Vec<Case<'static>>` holding closures that borrow locals — the exact
//! footgun the incident's committed fix documented) → `--fix-compile
//! --write` → compiles, the scaffold's TESTS pass, and the applied diff is
//! the min-delta candidate (the `'static` → `'_` token swap).
//!
//! Every fixture here was oracle-probed BEFORE authoring (see the plan's
//! probe record): the broken forms fire E0597 with the labeled anchors the
//! playbooks key on, and every fixed form compiles.
//!
//! Spawns `cargo check` per iteration (~seconds per test under load). Run:
//!
//! ```bash
//! cargo test --features rustc_errors --test fix_compile_e0597 -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use riir_refine::domains::rustc_errors::driver::CompileDriver;
use riir_refine::domains::rustc_errors::playbooks::family;

/// Scaffold a standalone crate under a fresh temp root (the fix_compile.rs
/// pattern — pid + counter keys the dir so parallel tests never share).
fn scaffold(name: &str, source: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("riir_e0597_{}_{seq}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"e0597e2e\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n",
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

/// Run the scaffold's own tests (the T2.2 "tests pass" bar). `cargo test`
/// output's last `test result:` line carries the counts.
fn scaffold_tests_pass(dir: &std::path::Path) -> bool {
    let out = std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["test", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .output();
    let Ok(out) = out else { return false };
    if !out.status.success() {
        eprintln!(
            "scaffold tests failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // `test result: ok. …` — the `ok.` prefix IS the zero-failure verdict
    // (`FAILED.` otherwise). Every result line must carry it.
    let failed = stdout
        .lines()
        .filter_map(|l| l.strip_prefix("test result: "))
        .any(|r| !r.starts_with("ok."));
    !failed
}

/// **T2.2 — the founding incident, end to end.** The
/// `settlement_goat.rs` shape: a `Vec<Case<'static>>` holding closures that
/// borrow locals (the trait-object lifetime-default footgun the incident's
/// committed fix documented). The driver must reach CLEAN via the
/// min-delta candidate (the `'static` → `'_` swap), keep exactly that
/// edit, and the scaffold's own tests must pass on the fixed tree.
#[test]
fn incident_shape_fixed_to_clean_with_min_delta() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub struct Case<'a> {\n    pub f: Box<dyn Fn() -> usize + 'a>,\n}\n\npub fn run_all() -> usize {\n    let n = 5usize;\n    let cases: Vec<Case<'static>> = vec![Case { f: Box::new(|| n) }];\n    cases.iter().map(|c| (c.f)()).sum()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::run_all;\n    #[test]\n    fn counts() {\n        assert_eq!(run_all(), 5);\n    }\n}\n";
    let dir = scaffold("incident", broken);
    let driver = CompileDriver::isolated().with_max_iters(5);
    let playbooks = family();
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks, true);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.error.is_none(), "{:?}", report.error);
    assert!(report.clean, "reached clean: {report:?}");
    assert_eq!(report.applied.len(), 1, "one kept fix: {report:?}");
    assert_eq!(
        report.applied[0].strategy_id, "e0597-anon-lifetime",
        "the min-delta candidate won: {report:?}"
    );
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(
        on_disk.contains("Vec<Case<'_>>"),
        "the 'static → '_ swap is the diff"
    );
    assert!(!on_disk.contains("'static>>"));
    assert!(scaffold_tests_pass(&dir), "the scaffold's tests pass");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The narrow-scope escape (shape A) — hoisted to clean by template 6,
/// with the spliced form matching the oracle-verified fixed source.
#[test]
fn scope_escape_hoisted_to_clean() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub fn probe() -> i32 {\n    let r;\n    {\n        let x = 5i32;\n        r = &x;\n    }\n    *r\n}\n";
    let dir = scaffold("scope", broken);
    let driver = CompileDriver::isolated().with_max_iters(3);
    let report = driver.run(&[dir.join("src").join("lib.rs")], &family(), true);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.clean, "hoisted to clean: {report:?}");
    assert_eq!(report.applied.len(), 1, "{report:?}");
    assert_eq!(report.applied[0].strategy_id, "e0597-scope-hoist");
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    let expected = "pub fn probe() -> i32 {\n    let r;\n    let x = 5i32;\n    {\n        r = &x;\n    }\n    *r\n}\n";
    assert_eq!(on_disk, expected, "the oracle-verified hoist form");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **The arbitration ladder** — the product claim over `cargo fix`'s single
/// blind suggestion. The Holder shape (struct field demands `'static`, no
/// let-annotation, binding used AFTER the closure): `move` alone moves the
/// binding out and trips E0382 (error count 1 → 1: NO progress) → the
/// driver reverts byte-exact and tries the next rung — `move`+`.clone()`
/// — which fixes it. Applied: move-clone. Reverted: move. Clean.
#[test]
fn ladder_reverts_move_and_keeps_move_clone() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub struct Holder {\n    pub f: Box<dyn Fn() -> usize + 'static>,\n}\n\npub fn make() -> usize {\n    let name = String::from(\"hello\");\n    let h = Holder { f: Box::new(|| name.len()) };\n    let after = name.len();\n    (h.f)() + after\n}\n";
    let dir = scaffold("ladder", broken);
    let driver = CompileDriver::isolated().with_max_iters(5);
    let report = driver.run(&[dir.join("src").join("lib.rs")], &family(), true);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.clean, "the ladder reaches clean: {report:?}");
    let applied: Vec<&str> = report
        .applied
        .iter()
        .map(|a| a.strategy_id.as_str())
        .collect();
    let reverted: Vec<&str> = report
        .reverted
        .iter()
        .map(|r| r.strategy_id.as_str())
        .collect();
    assert_eq!(applied, vec!["e0597-move-clone"], "{report:?}");
    assert_eq!(reverted, vec!["e0597-move"], "move was tried and reverted");
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(on_disk.contains("{ let name = name.clone(); move || name.len() }"));
    assert!(scaffold_tests_pass(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

/// **T2.3, standing** — already pinned at the driver level
/// (`driver_reports_nothing_on_clean_crate` in fix_compile.rs); pinned here
/// against the REAL family too: the incident's FIXED tree re-run through
/// the driver reports nothing to fix, zero edits (the check that would
/// have caught the founding incident's unnecessary fix — a fix committed
/// against code that already compiled).
#[test]
fn already_compiling_incident_tree_is_untouched() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let clean = "pub struct Case<'a> {\n    pub f: Box<dyn Fn() -> usize + 'a>,\n}\n\npub fn run_all() -> usize {\n    let n = 5usize;\n    let cases: Vec<Case<'_>> = vec![Case { f: Box::new(|| n) }];\n    cases.iter().map(|c| (c.f)()).sum()\n}\n";
    let dir = scaffold("probe1", clean);
    let driver = CompileDriver::isolated();
    let report = driver.run(&[dir.join("src").join("lib.rs")], &family(), true);
    assert!(report.ran);
    assert!(report.clean);
    assert_eq!(report.n_checks, 1, "one check, nothing else");
    assert_eq!(report.iterations, 0);
    assert!(report.applied.is_empty() && report.reverted.is_empty());
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(on_disk, clean, "untouched");
    let _ = std::fs::remove_dir_all(&dir);
}

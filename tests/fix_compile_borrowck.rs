//! Plan 019 Phase 3 e2e — the production playbook registry (E0597 +
//! borrowck families) against real cargo on the corpus mutant shapes.
//!
//! The lib-suite fixtures pin the proposers' edit shapes against probed
//! label spans. These tests pin the LOOP end-to-end: the real diagnostic
//! (real label spans, real byte offsets), the real registry, the real
//! keep/revert arbitration. The T3.3 cascade test exercises the
//! multi-iteration loop: two independent errors, one fixed per iteration.
//!
//! Spawns `cargo check` per iteration (~seconds per test under load).
//! Skipped (not failed) without cargo on PATH; run explicitly:
//!
//! ```bash
//! cargo test --features rustc_errors --test fix_compile_borrowck -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use riir_clippy::domains::rustc_errors::driver::CompileDriver;
use riir_clippy::domains::rustc_errors::playbooks::Playbooks;

/// Scaffold a standalone crate under a fresh temp root (pid + counter so
/// parallel tests never share — the probe.rs pattern).
fn scaffold(name: &str, source: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "riir_fix_borrowck_{}_{seq}_{}",
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

fn run(
    name: &str,
    broken: &str,
    max_iters: usize,
) -> (
    PathBuf,
    riir_clippy::domains::rustc_errors::driver::FixCompileReport,
) {
    let dir = scaffold(name, broken);
    let driver = CompileDriver::isolated().with_max_iters(max_iters);
    let report = driver.run(
        &[dir.join("src").join("lib.rs")],
        &Playbooks::production(),
        true,
    );
    (dir, report)
}

/// E0502 on a `Copy` element: the deref-removal PAIR restores the seed
/// byte-identically (`let first = v[0]; … first`) — the MatchedInverse
/// claim for the uncontested pair, end-to-end through real labels.
#[test]
fn e0502_copy_element_fixed_by_deref_removal() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = &v[0];\n    v.push(1);\n    *first\n}\n";
    let (dir, report) = run("e0502_copy", broken, 4);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.error.is_none(), "{:?}", report.error);
    assert!(report.clean, "fixed to clean: {report:?}");
    assert_eq!(report.applied.len(), 1, "one kept fix: {report:?}");
    assert_eq!(report.applied[0].code, "E0502");
    assert_eq!(report.applied[0].strategy_id, "e0502-deref-removal");
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(
        on_disk,
        "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = v[0];\n    v.push(1);\n    first\n}\n",
        "byte-identical to the corpus seed"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// E0502 on an owned element: deref-removal is tried first (min-delta),
/// reverts (E0507 — no progress), then clone-out applies and reaches
/// clean. The LADDER — ranked alternatives with oracle arbitration — is
/// the product claim; this is its e2e pin on the second rung.
#[test]
fn e0502_owned_element_ladders_to_clone_out() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub fn probe(items: &mut Vec<String>) -> usize {\n    let head = &items[0];\n    items.push(String::new());\n    head.len()\n}\n";
    let (dir, report) = run("e0502_owned", broken, 6);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.clean, "fixed to clean: {report:?}");
    let strategies: Vec<&str> = report
        .applied
        .iter()
        .map(|a| a.strategy_id.as_str())
        .collect();
    assert!(
        strategies.contains(&"e0502-clone-out"),
        "clone-out is the keeper: {report:?}"
    );
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(on_disk.contains("let head = items[0].clone();"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// E0499: the swap moves the first borrow's last use before the second
/// decl — clean through the real labels.
#[test]
fn e0499_fixed_by_swap() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub fn probe(v: &mut Vec<u8>) {\n    let a = &mut *v;\n    let b = &mut *v;\n    a.push(1);\n    b.push(2);\n}\n";
    let (dir, report) = run("e0499", broken, 4);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.clean, "fixed to clean: {report:?}");
    assert_eq!(report.applied.len(), 1, "{report:?}");
    assert_eq!(report.applied[0].code, "E0499");
    assert_eq!(report.applied[0].strategy_id, "e0499-swap-second-decl");
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(on_disk.contains("let a = &mut *v;\n    a.push(1);\n    let b = &mut *v;"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// E0505 — the contested opening: `cargo fix` rewrites the borrow site
/// (`let r = &v.clone()`, aliasing diverged — Bench 041); our clone-at-move
/// restores the seed byte-identically. This is the one MEASURED win over
/// the floor on contested territory.
#[test]
fn e0505_clone_at_move_matches_the_seed() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v;\n    r.len() + w.len()\n}\n";
    let (dir, report) = run("e0505", broken, 4);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.clean, "fixed to clean: {report:?}");
    assert_eq!(report.applied.len(), 1, "{report:?}");
    assert_eq!(report.applied[0].code, "E0505");
    assert_eq!(report.applied[0].strategy_id, "e0505-clone-at-move");
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert_eq!(
        on_disk,
        "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v.clone();\n    r.len() + w.len()\n}\n",
        "byte-identical to the corpus seed — where cargo fix DIVERGES"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T3.3 — the two-level cascade: TWO independent E0502 errors in one
/// crate; iteration 1 fixes one (count 2→1), iteration 2 re-proposes on
/// fresh diagnostics and fixes the other (1→0). Exercises the loop's
/// multi-iteration path: byte spans from the FIRST pass go stale after
/// the first kept fix, and only re-proposal keeps them valid.
#[test]
fn cascade_two_e0502_errors_fixed_across_iterations() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let broken = "pub fn first(v: &mut Vec<u8>) -> u8 {\n    let f = &v[0];\n    v.push(1);\n    *f\n}\n\npub fn second(v: &mut Vec<u8>) -> u8 {\n    let s = &v[0];\n    v.push(2);\n    *s\n}\n";
    let (dir, report) = run("cascade", broken, 5);
    assert!(report.ran, "{:?}", report.error);
    assert!(report.clean, "both fixed to clean: {report:?}");
    assert_eq!(
        report.applied.len(),
        2,
        "one fix per error, across iterations: {report:?}"
    );
    assert!(
        report.iterations >= 2,
        "the cascade consumed ≥2 iterations: {report:?}"
    );
    let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(on_disk.contains("let f = v[0];"));
    assert!(on_disk.contains("let s = v[0];"));
    assert!(
        !on_disk.contains("&v["),
        "no residual borrows of the elements"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The T2.3 gate for this family: the corpus SEEDS (already compiling)
/// must be untouched — one check, zero iterations, zero edits.
#[test]
fn already_compiling_seeds_are_untouched() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let seeds = [
        "pub fn probe(v: &mut Vec<u8>) -> u8 {\n    let first = v[0];\n    v.push(1);\n    first\n}\n",
        "pub fn probe(v: &mut Vec<u8>) {\n    {\n        let a = &mut *v;\n        a.push(1);\n    }\n    let b = &mut *v;\n    b.push(2);\n}\n",
        "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v.clone();\n    r.len() + w.len()\n}\n",
    ];
    for (i, seed) in seeds.iter().enumerate() {
        let (dir, report) = run(&format!("clean{i}"), seed, 3);
        assert!(report.ran, "{:?}", report.error);
        assert!(report.clean, "seed {i} compiles");
        assert_eq!(report.n_checks, 1, "seed {i}: one check only");
        assert_eq!(report.iterations, 0, "seed {i}: no iterations");
        assert!(
            report.applied.is_empty() && report.reverted.is_empty(),
            "seed {i}: no edits"
        );
        let on_disk = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
        assert_eq!(&on_disk, seed, "seed {i} byte-identical on disk");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

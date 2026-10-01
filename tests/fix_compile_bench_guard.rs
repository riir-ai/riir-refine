//! Issue 118 T4 — the bench-target guard on the `--fix-compile` write
//! path, e2e against real cargo (the `fix_compile.rs` pattern: real
//! scaffold crate, real `cargo check --all-targets`, real E-codes, an
//! injected test playbook).
//!
//! The guard's postures, one specimen error shape (E0596) in a lib target
//! and/or a `harness = false` bench target:
//! 1. **routine** (the bench file reached only via package discovery):
//!    the bench error is DECLINED — counted in the report, the bench file
//!    on disk byte-identical (no write ever landed), nothing applied;
//! 2. **explicit** (the bench file named on the command line): the fix
//!    proceeds under the recorded flag — `bench_forced_files` carries it,
//!    the file heals, the run reaches CLEAN;
//! 3. the identical error in `src/` is unaffected by the guard (heals
//!    with zero declines when no bench target is involved).
//!
//! Fixture law (measured): a lib error MASKS bench errors — cargo never
//! compiles the bench target until the lib is clean, so a broken-lib +
//! broken-bench crate shows only the lib error and the driver's keep
//! rule reverts the lib fix at equal count once the bench error unmasks
//! (pre-existing driver semantics, guard-independent). The decline
//! fixture therefore uses a CLEAN lib; the src-heals fixture has no bench
//! target at all.
//!
//! Spawns `cargo check` per iteration (~seconds per test under load);
//! skipped (not failed) without cargo on PATH. `rustc_errors` is
//! default-on; run explicitly:
//!
//! ```bash
//! cargo test --test fix_compile_bench_guard -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use riir_clippy::domains::rustc_errors::diag::RustcErrorDiag;
use riir_clippy::domains::rustc_errors::driver::CompileDriver;
use riir_clippy::domains::rustc_errors::playbooks::{CompileProposer, FixCandidate, Playbooks};

/// A test playbook: for `code`, replace `from` with `to` (first
/// occurrence in the source) — the `fix_compile.rs` shape, verbatim.
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

/// The E0596 specimen in a lib target (the `fix_compile.rs` fixture).
const BROKEN_LIB: &str =
    "pub fn probe() -> usize {\n    let v = Vec::new();\n    v.push(1);\n    v.len()\n}\n";

/// The identical E0596 specimen in a bench target — same error shape,
/// one axis over. `harness = false` compiles it as a plain bench binary
/// under `--all-targets`, so the error surfaces with the bench path.
const BROKEN_BENCH: &str =
    "fn main() {\n    let v = Vec::new();\n    v.push(1);\n    println!(\"{}\", v.len());\n}\n";

const CLEAN_LIB: &str = "pub fn ok() -> u32 {\n    1\n}\n";

fn playbooks() -> Playbooks {
    Playbooks::empty().with(Box::new(ReplacePlaybook {
        code: "E0596",
        from: "let v = Vec::new();",
        to: "let mut v = Vec::new();",
    }))
}

/// Scaffold a standalone crate with a lib target AND a `harness = false`
/// bench target under a fresh temp root (pid + counter keys the dir so
/// parallel tests never share — the `fix_compile.rs` pattern).
fn scaffold_bench(name: &str, lib_src: &str, bench_src: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "riir_fc_bench_{}_{seq}_{}",
        std::process::id(),
        name
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(dir.join("benches")).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fcb\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
         [lib]\npath = \"src/lib.rs\"\n\n\
         [[bench]]\nname = \"bench_probe\"\npath = \"benches/bench_probe.rs\"\nharness = false\n\n\
         [workspace]\n",
    )
    .unwrap();
    std::fs::write(src.join("lib.rs"), lib_src).unwrap();
    std::fs::write(dir.join("benches").join("bench_probe.rs"), bench_src).unwrap();
    dir
}

fn cargo_available() -> bool {
    std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .arg("--version")
        .output()
        .is_ok()
}

fn is_bench_probe(p: &Path) -> bool {
    p.ends_with("benches/bench_probe.rs")
}

/// Routine discovery (the LIB named, the bench reached only via
/// `--all-targets`): the bench error is DECLINED, nothing is applied, and
/// the bench file never receives a single byte. (Lib must be clean here —
/// see the fixture law in the module docs.)
#[test]
fn routine_path_declines_bench_without_writing() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let dir = scaffold_bench("routine", CLEAN_LIB, BROKEN_BENCH);
    let driver = CompileDriver::isolated().with_max_iters(5);
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks(), true);
    assert!(report.ran, "cargo must have run");
    assert!(report.error.is_none(), "{:?}", report.error);
    // The bench error was declined, never applied, never forced.
    assert!(report.bench_declined >= 1, "bench site must be declined");
    assert!(
        report
            .bench_declined_files
            .iter()
            .any(|f| is_bench_probe(f)),
        "declined files must name the bench target: {:?}",
        report.bench_declined_files
    );
    assert!(
        report.applied.is_empty(),
        "no write may land on the routine path: {:?}",
        report.applied
    );
    assert!(
        report.reverted.is_empty(),
        "a declined candidate is filtered, not tried-and-reverted: {:?}",
        report.reverted
    );
    assert!(
        report.bench_forced_files.is_empty(),
        "nothing forced without an explicit bench path"
    );
    // Honest end state: the bench error stands.
    assert!(report.remaining >= 1);
    assert!(!report.clean);
    // Byte-identical bench file — the guard's whole claim.
    let after = std::fs::read_to_string(dir.join("benches").join("bench_probe.rs")).unwrap();
    assert_eq!(after, BROKEN_BENCH);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The identical error in `src/` heals with ZERO declines when no bench
/// target is involved — the guard does not leak sideways onto ordinary
/// compile repair.
#[test]
fn identical_error_in_src_heals_unaffected() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let dir = scaffold_bench("srcheal", BROKEN_LIB, "");
    // No bench content: drop the [[bench]] target so the crate is lib-only.
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fcb\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n",
    )
    .unwrap();
    let driver = CompileDriver::isolated().with_max_iters(5);
    let report = driver.run(&[dir.join("src").join("lib.rs")], &playbooks(), true);
    assert!(report.ran, "cargo must have run");
    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.bench_declined, 0, "no bench target, no declines");
    assert!(report.bench_declined_files.is_empty());
    assert_eq!(report.remaining, 0, "remaining: {}", report.remaining);
    assert!(report.clean, "the src error must fully heal");
    let after = std::fs::read_to_string(dir.join("src").join("lib.rs")).unwrap();
    assert!(after.contains("let mut v"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Explicit path (the bench file named on the command line): the fix
/// proceeds under the recorded flag — T3's flag-and-proceed posture,
/// never a silent hard refusal on a user-forced target.
#[test]
fn explicit_bench_path_flags_and_proceeds() {
    if !cargo_available() {
        eprintln!("skip: cargo unavailable");
        return;
    }
    let dir = scaffold_bench("explicit", CLEAN_LIB, BROKEN_BENCH);
    let bench_path = dir.join("benches").join("bench_probe.rs");
    let driver = CompileDriver::isolated().with_max_iters(5);
    let report = driver.run(&[dir.join("benches").join("bench_probe.rs")], &playbooks(), true);
    assert!(report.ran, "cargo must have run");
    assert!(report.error.is_none(), "{:?}", report.error);
    assert!(
        report.bench_forced_files.iter().any(|f| is_bench_probe(f)),
        "the named bench file must be flagged-as-forced: {:?}",
        report.bench_forced_files
    );
    assert_eq!(report.bench_declined, 0, "forced is not declined");
    assert!(
        report.applied.iter().any(|a| is_bench_probe(&a.file)),
        "the bench fix must proceed under the override"
    );
    let after = std::fs::read_to_string(&bench_path).unwrap();
    assert!(
        after.contains("let mut v"),
        "the bench file must carry the applied fix"
    );
    // Only the bench was broken; with it healed the run is CLEAN.
    assert!(report.clean, "remaining: {}", report.remaining);
    let _ = std::fs::remove_dir_all(&dir);
}

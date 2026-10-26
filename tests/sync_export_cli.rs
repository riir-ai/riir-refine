//! riir-dapps Issue 112 T4 — `cargo refine sync --export <file|->`, driven
//! through the REAL binary.
//!
//! The export exists so a reader can see what `--mine` would upload WITHOUT
//! logging in or sending anything. So the properties worth pinning live at
//! the process boundary:
//!
//! | invocation | exit | asserted |
//! |---|---|---|
//! | `--export x` (no `sync`) | **2** | refused before any work |
//! | `sync --export f`, no store | 1 | names the missing store, writes no file |
//! | `sync --export f`, seeded store, no policy | 0 | file verifies; default-policy preview; NO account created |
//! | `sync --export -` | 0 | stdout is the NDJSON alone (status on stderr) |
//! | `sync --export f`, path-less (legacy) row | 0 | fail-closed: 0 included, 1 blocked, the count says so loud (Issue 152) |
//!
//! Hermetic like `unmine_consent_cli`: every run has its own account dir,
//! trajectory root and cwd, a closed service URL, and no inherited policy
//! override — the real `~/.config/riir-heal` is never read or written.
//!
//! Gated on the binary's own `required-features`; without them the file
//! compiles to an empty binary that prints a green `0 passed`, which is why
//! the Cargo.toml `required-features` row is not optional.

#![cfg(all(
    feature = "kat_service",
    feature = "latent_retrieval",
    feature = "rust_perf",
    feature = "clippy_lints"
))]

#[path = "support/hermetic_home.rs"]
mod hermetic_home;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_refine_export_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// One hermetic invocation: `dir/acct` is the account dir (must stay empty),
/// `dir` is both the trajectory root and the cwd (policy discovery walks up
/// from the cwd).
fn refine(dir: &Path, args: &[&str]) -> Output {
    let acct = dir.join("acct");
    Command::new(env!("CARGO_BIN_EXE_cargo-refine"))
        .current_dir(dir)
        .env("RIIR_REFINE_KAT_ACCOUNT_DIR", &acct)
        .envs(hermetic_home::vars(&acct))
        .env("RIIR_AUTH_ACCOUNT_DIR", &acct)
        .env("RIIR_REFINE_TRAJ_ROOT", dir)
        .env("RIIR_REFINE_KAT_SERVICE_URL", "http://127.0.0.1:1")
        .env_remove("RIIR_REFINE_SYNC_POLICY")
        .args(args)
        .output()
        .expect("spawn cargo-refine")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A store holding one recorded fix — what any `--fix` run leaves behind:
/// the row carries its source file, so the default policy's `src/**/*.rs`
/// allow-glob admits it (the path is a gate input only — it never rides the
/// payload).
fn seed_store(dir: &Path) {
    let (store, _) = riir_refine::traj_store::TrajStore::open(dir).expect("open store");
    store
        .sourced("src/lib.rs")
        .record(
            "needless_return",
            "return total;",
            "total",
            true,
            true,
            "run-export-test",
        )
        .expect("record row");
}

#[test]
fn export_without_sync_is_rejected() {
    let dir = scratch("nosync");
    let out = refine(&dir, &["--export", "x.ndjson"]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("--export requires the `sync` command"));
    assert!(!dir.join("x.ndjson").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_with_no_store_names_it_and_writes_nothing() {
    let dir = scratch("nostore");
    let out = refine(&dir, &["sync", "--export", "batch.ndjson"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(text(&out).contains("no trajectory store"), "{}", text(&out));
    assert!(!dir.join("batch.ndjson").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_writes_a_verifiable_batch_without_an_account() {
    let dir = scratch("seeded");
    seed_store(&dir);
    let out = refine(&dir, &["sync", "--export", "batch.ndjson"]);
    let t = text(&out);
    assert_eq!(out.status.code(), Some(0), "{t}");
    assert!(t.contains("previewing the default policy"), "{t}");
    assert!(t.contains("NOT sent"), "{t}");

    let ndjson = std::fs::read_to_string(dir.join("batch.ndjson")).expect("export written");
    let header = riir_refine::kat_sync_payload::verify_payload(&ndjson).expect("export verifies");
    assert_eq!(header.rows_included, 1);
    assert_eq!(header.rows_blocked, 0);
    assert!(ndjson.contains("\"lint_key\":\"needless_return\""));
    // The one row's snippet is the span, never a path.
    assert!(!ndjson.contains(dir.to_string_lossy().as_ref()));

    // No login happened: the account dir holds nothing.
    let acct = dir.join("acct");
    let entries = std::fs::read_dir(&acct).map_or(0, Iterator::count);
    assert_eq!(entries, 0, "export must not create an account");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn export_to_stdout_is_pure_ndjson() {
    let dir = scratch("stdout");
    seed_store(&dir);
    let out = refine(&dir, &["sync", "--export", "-"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let stdout = String::from_utf8(out.stdout.clone()).expect("utf-8");
    riir_refine::kat_sync_payload::verify_payload(&stdout).expect("stdout verifies");
    assert!(String::from_utf8_lossy(&out.stderr).contains("export: 1 row(s) included"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// riir-dapps Issue 119 T2's fail-closed law, at the process boundary: a row
/// recorded WITHOUT its source file (the legacy/path-less writers) is blocked
/// the moment `allow_paths` is non-empty — which the default policy's are. The
/// export still succeeds (exit 0, a verifiable empty batch) and the count line
/// says so loud; the row's snippet never leaves the machine.
#[test]
fn export_fails_closed_on_a_pathless_row_and_says_so() {
    let dir = scratch("blocked");
    let (store, _) = riir_refine::traj_store::TrajStore::open(&dir).expect("open store");
    store
        .record(
            "needless_return",
            "return total;",
            "total",
            true,
            true,
            "run-export-test",
        )
        .expect("record row");
    let out = refine(&dir, &["sync", "--export", "batch.ndjson"]);
    let t = text(&out);
    assert_eq!(out.status.code(), Some(0), "{t}");
    assert!(t.contains("export: 0 row(s) included, 1 blocked"), "{t}");
    let ndjson = std::fs::read_to_string(dir.join("batch.ndjson")).expect("export written");
    let header = riir_refine::kat_sync_payload::verify_payload(&ndjson).expect("export verifies");
    assert_eq!((header.rows_included, header.rows_blocked), (0, 1));
    assert!(!ndjson.contains("needless_return"), "{ndjson}");
    let _ = std::fs::remove_dir_all(&dir);
}

//! Issue 150 (option 3) — the build-commit stamp's tests, at both levels.
//!
//! The CAPTURE tests live in `build_commit.rs`'s own `#[cfg(test)]` mod —
//! build scripts cannot host tests, so that module is compiled HERE (via
//! `#[path]`) to run them: temp git fixtures for the clean/dirty/untracked/
//! unknown degradations and the rerun-list coverage. This file adds the one
//! thing no in-process test can reach: the RERUN contract at the process
//! boundary — a real `cargo build` of a throwaway crate whose build script
//! includes this repo's `build_commit.rs`, proving a commit → rebuild cycle
//! picks up the new hash. A stamp that never updates is the exact lie Issue
//! 150 is about; without this arm nothing proves the detector ever fires.
//!
//! Needs `git` and `cargo` on PATH — both exist wherever this repo builds.
//! No features, no network (the probe crate has zero deps).

#[path = "../build_commit.rs"]
mod build_commit;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// A unique temp root (pid + nanos — the shared-temp-path rule).
fn unique_temp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "riir_refine_p150_e2e_{tag}_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir fixture");
    dir
}

fn run(cwd: &Path, program: &str, args: &[&str], env: &[(&str, &str)]) -> (bool, String) {
    let out = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .envs(env.iter().copied())
        .output()
        .unwrap_or_else(|e| panic!("spawn {program} {args:?}: {e}"));
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// The one benign race for the HEAD-match asserts below: a sibling session
/// committing BETWEEN this read and the build. Re-run resolves it.
fn head_of(repo: &Path) -> String {
    let (ok, out) = run(repo, "git", &["rev-parse", "--short=12", "HEAD"], &[]);
    assert!(ok, "git rev-parse HEAD in the probe repo: {out}");
    out.trim().to_string()
}

fn git_env() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GIT_AUTHOR_NAME", "t"),
        ("GIT_AUTHOR_EMAIL", "t@t"),
        ("GIT_COMMITTER_NAME", "t"),
        ("GIT_COMMITTER_EMAIL", "t@t"),
    ]
}

#[test]
fn a_commit_rebuild_picks_up_the_new_hash() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let repo = unique_temp("rerun");
    let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("build_commit.rs");

    // The probe crate: zero deps, one bin printing its build-script stamp,
    // a build.rs that includes THIS repo's capture module by absolute path.
    std::fs::create_dir_all(repo.join("src")).expect("mkdir src");
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"stamp_probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(
        repo.join("build.rs"),
        format!(
            "#[path = r#\"{}\"#]\nmod build_commit;\nfn main() {{\n    let cwd = std::path::PathBuf::from(\n        std::env::var(\"CARGO_MANIFEST_DIR\").unwrap_or_else(|_| \".\".to_string()),\n    );\n    for line in build_commit::rerun_lines(&cwd) {{\n        println!(\"{{line}}\");\n    }}\n    println!(\n        \"cargo:rustc-env=PROBE_COMMIT={{}}\",\n        build_commit::capture_commit(&cwd)\n    );\n}}\n",
            include.display()
        ),
    )
    .expect("write build.rs");
    std::fs::write(
        repo.join("src/main.rs"),
        "fn main() { println!(\"{}\", env!(\"PROBE_COMMIT\")); }\n",
    )
    .expect("write main.rs");

    let genv = git_env();
    let (ok, out) = run(
        &repo,
        "git",
        &["init", "-q", "--initial-branch=main"],
        &genv,
    );
    assert!(ok, "git init: {out}");
    let (ok, out) = run(&repo, "git", &["add", "-A"], &genv);
    assert!(ok, "git add: {out}");
    let (ok, out) = run(&repo, "git", &["commit", "-q", "-m", "init"], &genv);
    assert!(ok, "git commit: {out}");

    let target = repo.join("target");
    let build_env: Vec<(&str, &str)> = vec![
        ("CARGO_TARGET_DIR", target.to_str().expect("target path")),
        ("CARGO_NET_OFFLINE", "true"),
    ];
    let stamp = |label: &str| -> String {
        let (ok, out) = run(&repo, &cargo, &["build", "--offline", "-q"], &build_env);
        assert!(ok, "cargo build ({label}): {out}");
        let bin = target.join("debug/stamp_probe");
        let (ok, out) = run(&repo, bin.to_str().expect("bin path"), &[], &[]);
        assert!(ok, "run stamp_probe ({label}): {out}");
        out.trim().to_string()
    };

    let head1 = head_of(&repo);
    let h1 = stamp("first");
    assert_eq!(h1, head1, "first stamp must name the committed HEAD");
    assert!(!h1.contains('-'), "clean probe repo must be bare hex: {h1}");

    // Commit 2: src changes AND the branch ref moves — both watched inputs.
    std::fs::write(
        repo.join("src/main.rs"),
        "// touched for the rerun assertion\nfn main() { println!(\"{}\", env!(\"PROBE_COMMIT\")); }\n",
    )
    .expect("rewrite main.rs");
    let (ok, out) = run(&repo, "git", &["add", "-A"], &genv);
    assert!(ok, "git add 2: {out}");
    let (ok, out) = run(&repo, "git", &["commit", "-q", "-m", "touch src"], &genv);
    assert!(ok, "git commit 2: {out}");

    let head2 = head_of(&repo);
    assert_ne!(head1, head2, "fixture sanity: HEAD must have moved");
    let h2 = stamp("second");
    assert_ne!(
        h1, h2,
        "the stamp MUST move with the commit — a frozen stamp is the Issue-150 lie ({h1} stayed)"
    );
    assert_eq!(h2, head2, "second stamp must name the new HEAD, got {h2}");

    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn an_uncommitted_edit_stamps_dirty_at_the_process_boundary() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let repo = unique_temp("dirty_e2e");
    let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("build_commit.rs");

    std::fs::create_dir_all(repo.join("src")).expect("mkdir src");
    std::fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"stamp_probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(
        repo.join("build.rs"),
        format!(
            "#[path = r#\"{}\"#]\nmod build_commit;\nfn main() {{\n    let cwd = std::path::PathBuf::from(\n        std::env::var(\"CARGO_MANIFEST_DIR\").unwrap_or_else(|_| \".\".to_string()),\n    );\n    println!(\n        \"cargo:rustc-env=PROBE_COMMIT={{}}\",\n        build_commit::capture_commit(&cwd)\n    );\n}}\n",
            include.display()
        ),
    )
    .expect("write build.rs");
    std::fs::write(
        repo.join("src/main.rs"),
        "fn main() { println!(\"{}\", env!(\"PROBE_COMMIT\")); }\n",
    )
    .expect("write main.rs");

    let genv = git_env();
    let (ok, out) = run(
        &repo,
        "git",
        &["init", "-q", "--initial-branch=main"],
        &genv,
    );
    assert!(ok, "git init: {out}");
    let (ok, out) = run(&repo, "git", &["add", "-A"], &genv);
    assert!(ok, "git add: {out}");
    let (ok, out) = run(&repo, "git", &["commit", "-q", "-m", "init"], &genv);
    assert!(ok, "git commit: {out}");

    // Uncommitted edit to a COMPILED file, then rebuild from the warm target.
    std::fs::write(
        repo.join("src/main.rs"),
        "// uncommitted\nfn main() { println!(\"{}\", env!(\"PROBE_COMMIT\")); }\n",
    )
    .expect("dirty main.rs");
    let target = repo.join("target");
    let build_env: Vec<(&str, &str)> = vec![
        ("CARGO_TARGET_DIR", target.to_str().expect("target path")),
        ("CARGO_NET_OFFLINE", "true"),
    ];
    let (ok, out) = run(&repo, &cargo, &["build", "--offline", "-q"], &build_env);
    assert!(ok, "cargo build (dirty): {out}");
    let bin = target.join("debug/stamp_probe");
    let (ok, out) = run(&repo, bin.to_str().expect("bin path"), &[], &[]);
    assert!(ok, "run stamp_probe (dirty): {out}");
    let stamp = out.trim().to_string();
    let head = head_of(&repo);
    assert_eq!(
        stamp,
        format!("{head}-dirty"),
        "an uncommitted compiled-file edit must stamp -dirty, got {stamp}"
    );

    let _ = std::fs::remove_dir_all(&repo);
}

//! Issue 150 (option 3) — the build-commit stamp: capture + rerun wiring.
//!
//! WHY THIS FILE EXISTS: the Homebrew-installed release binary shadows the
//! maintained dev symlink on PATH, and `--version` could not tell them apart
//! (it stamps the compiled feature set, not the commit). The stamp names the
//! commit, so a binary built from this tree is identifiable in one command.
//!
//! SHARED-MODULE SHAPE: build scripts cannot host unit tests, so the capture
//! logic lives here — included by `build.rs` via `#[path]` for the real
//! build, and by `tests/build_commit_logic.rs` for the tests. No deps, std
//! only, because build.rs compiles before the dep graph resolves.
//!
//! THE RERUN CONTRACT (the premise the first design got wrong): cargo
//! re-runs a build script only on its declared `rerun-if-*` inputs once ANY
//! such line is emitted — the "any package file changed" default is OFF from
//! the first line. The inputs that can move the stamp are therefore declared
//! here explicitly: every compiled path (the dirty check's set) plus the git
//! state files, resolved through `git rev-parse --git-path` because `.git`
//! is a FILE inside a `git worktree` checkout (this workspace uses them
//! routinely) and a hard-coded `.git/HEAD` would silently watch nothing
//! there. Detached HEAD (CI checkouts) has no branch ref to watch; HEAD +
//! packed-refs still cover it, and a build-once checkout never moves.
//!
//! THE DIRTY SUFFIX IS SCOPED TO COMPILED FILES: a bare `git status` in this
//! shared worktree is almost always dirty from sibling sessions' `.plans`/
//! `.issues` edits — an always-dirty stamp says nothing. Only paths that
//! compile into this binary count (`src`, the build scripts, the manifest,
//! the embedded assets under `assets/` and `tests/fixtures/` — both are
//! `include_bytes!`/`include_str!`'d into the binary).
//!
//! ⛔ EVERY WATCHED PATH IS EXISTENCE-FILTERED: cargo treats a MISSING
//! `rerun-if-changed` path as perpetually dirty (probed: `Dirty p: the file
//! 'x' is missing` on every build), so emitting a `--git-path` result
//! unconditionally would recompile the crate on every build the moment
//! `git gc` packs the loose branch ref away. A path that appears or vanishes
//! later costs ONE script re-run (the emitted set changes), never a
//! perpetual one.

use std::path::Path;
use std::process::Command;

/// The literal exported when git is absent, the dir is not a checkout, or
/// HEAD cannot resolve (fresh `git init`, tarball build). Honest "cannot
/// see" — never a build error, never a guess.
pub const UNKNOWN: &str = "unknown";

/// Paths that compile into the binary — the dirty check's scope AND the core
/// of the rerun list. One list, two uses, so they cannot drift. `assets` and
/// `tests/fixtures` are the `include_bytes!`/`include_str!` inputs: an edit
/// there changes the binary while `src` stays untouched, so an unwatched one
/// would stamp clean through a real change.
pub const COMPILED_PATHS: &[&str] = &[
    "src",
    "build.rs",
    "build_commit.rs",
    "Cargo.toml",
    "Cargo.lock",
    "assets",
    "tests/fixtures",
];

/// Run `git <args>` in `cwd`; `Some(stdout)` on exit-0, else `None` (missing
/// git, not a repo, refused). Stderr is deliberately discarded — the callers
/// degrade to [`UNKNOWN`], they do not diagnose.
#[must_use]
pub fn git(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// The commit stamp: `<12-hex>` or `<12-hex>-dirty`, or [`UNKNOWN`].
///
/// `--short=12` names the commit at the moment build.rs runs; the `-dirty`
/// suffix (the rustc convention) says the tree's compiled files carried
/// uncommitted edits at compile time — the normal state here, and exactly
/// the honesty a shared worktree needs.
#[must_use]
pub fn capture_commit(cwd: &Path) -> String {
    let hash = git(cwd, &["rev-parse", "--short=12", "HEAD"]).map(|s| s.trim().to_string());
    let Some(hash) = hash else {
        return UNKNOWN.to_string();
    };
    let valid = !hash.is_empty() && hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return UNKNOWN.to_string();
    }
    if is_dirty(cwd) {
        format!("{hash}-dirty")
    } else {
        hash
    }
}

/// Do the COMPILED paths carry uncommitted edits? Tracked files only
/// (`--untracked-files=no`): an untracked scratch file does not change what
/// compiles. A repo where git cannot answer reads as clean — the stamp then
/// simply omits the suffix, it never invents one.
#[must_use]
pub fn is_dirty(cwd: &Path) -> bool {
    let mut args = vec!["status", "--porcelain", "--untracked-files=no", "--"];
    args.extend_from_slice(COMPILED_PATHS);
    git(cwd, &args).is_some_and(|s| !s.trim().is_empty())
}

/// The `cargo:rerun-if-changed=…` lines for build.rs to print.
///
/// ⛔ The FIRST rerun line disables cargo's any-file default, so this list
/// must stay a superset of everything that can move the stamp — AND every
/// entry must exist (the module doc's perpetual-dirty hazard). The git
/// files are resolved per-checkout via `--git-path`, never typed, and
/// filtered to what is actually there.
#[must_use]
pub fn rerun_lines(cwd: &Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in COMPILED_PATHS {
        if cwd.join(p).exists() {
            out.push(format!("cargo:rerun-if-changed={p}"));
        }
    }
    // What moves `rev-parse HEAD`, existence-filtered (see the module doc):
    // HEAD itself (branch switch), the reflog (appends on every commit),
    // the refs/heads DIRECTORY (entries appear/vanish when loose refs are
    // written or packed away — the one watch that fires even when the
    // branch's own ref file is packed), packed-refs (the pack event), and
    // the branch's ref file (rewritten on every commit). `--git-path`
    // keeps every one of them worktree-safe, where `.git` is a file.
    // Detached HEAD (CI checkouts) has no branch ref — HEAD + logs/HEAD
    // still fire on whatever moves in a build-once checkout.
    for query in ["HEAD", "logs/HEAD", "refs/heads", "packed-refs"] {
        if let Some(p) = git_path(cwd, query) {
            if resolve_exists(cwd, &p) {
                out.push(format!("cargo:rerun-if-changed={p}"));
            }
        }
    }
    if let Some(branch) =
        git(cwd, &["symbolic-ref", "--short", "HEAD"]).map(|s| s.trim().to_string())
    {
        if !branch.is_empty() {
            if let Some(p) = git_path(cwd, &format!("refs/heads/{branch}")) {
                if resolve_exists(cwd, &p) {
                    out.push(format!("cargo:rerun-if-changed={p}"));
                }
            }
        }
    }
    out
}

/// `--git-path` answers relative to `cwd` OR absolute (worktrees); existence
/// must be asked from the same base the emitted path resolves from.
fn resolve_exists(cwd: &Path, p: &str) -> bool {
    let path = Path::new(p);
    if path.is_absolute() {
        path.exists()
    } else {
        cwd.join(path).exists()
    }
}

/// `git rev-parse --git-path <name>` — the per-checkout path of a git-state
/// file, worktree-safe. Empty/failed answers contribute nothing.
fn git_path(cwd: &Path, name: &str) -> Option<String> {
    let p = git(cwd, &["rev-parse", "--git-path", name]).map(|s| s.trim().to_string());
    p.filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A throwaway git repo. `git init` + one commit; the temp dir lives
    /// under the system temp dir, never inside THIS checkout (git resolves
    /// upward — a fixture inside a repo would find OUR git state).
    struct TempRepo {
        dir: PathBuf,
    }

    impl TempRepo {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "riir_refine_p150_{}_{tag}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock before epoch")
                    .as_nanos()
            ));
            std::fs::create_dir_all(dir.join("src")).expect("mkdir fixture");
            Self { dir }
        }

        fn git(&self, args: &[&str]) -> String {
            let out = Command::new("git")
                .args(args)
                .current_dir(&self.dir)
                .env("GIT_AUTHOR_NAME", "t")
                .env("GIT_AUTHOR_EMAIL", "t@t")
                .env("GIT_COMMITTER_NAME", "t")
                .env("GIT_COMMITTER_EMAIL", "t@t")
                .output()
                .expect("spawn git in fixture");
            assert!(
                out.status.success(),
                "git {args:?} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).expect("utf8")
        }

        fn commit_all(&self, msg: &str) {
            self.git(&["add", "-A"]);
            self.git(&["commit", "-m", msg, "-q"]);
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn clean_repo_yields_bare_hex_and_matches_head() {
        let repo = TempRepo::new("clean");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        repo.commit_all("init");
        let stamp = capture_commit(&repo.dir);
        let head = repo.git(&["rev-parse", "--short=12", "HEAD"]);
        assert_eq!(stamp, head.trim(), "{stamp} vs {head}");
        assert!(
            !stamp.contains('-'),
            "clean repo must not carry -dirty: {stamp}"
        );
    }

    #[test]
    fn edited_tracked_source_is_dirty_but_untracked_is_not() {
        let repo = TempRepo::new("dirty");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        repo.commit_all("init");
        // Untracked scratch: outside the compiled set, never dirties.
        std::fs::write(repo.dir.join("scratch.txt"), "x").expect("write");
        assert!(
            !is_dirty(&repo.dir),
            "untracked files must not dirty the stamp"
        );
        // Tracked compiled file edited: dirties.
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn g() {}\n").expect("write");
        assert!(is_dirty(&repo.dir), "an edited tracked src file must dirty");
        let stamp = capture_commit(&repo.dir);
        let head = repo.git(&["rev-parse", "--short=12", "HEAD"]);
        assert_eq!(stamp, format!("{}-dirty", head.trim()), "{stamp}");
    }

    #[test]
    fn staged_only_edits_dirty_too() {
        let repo = TempRepo::new("staged");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        repo.commit_all("init");
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn h() {}\n").expect("write");
        repo.git(&["add", "src/lib.rs"]);
        assert!(
            is_dirty(&repo.dir),
            "a staged edit is uncommitted and must dirty"
        );
    }

    #[test]
    fn no_git_and_unborn_head_both_degrade_to_unknown() {
        // A plain directory with no git anywhere above it.
        let bare = TempRepo::new("nogit");
        let stamp = capture_commit(&bare.dir);
        assert_eq!(stamp, UNKNOWN, "no repo = unknown, never a guess");
        // A repo with zero commits: HEAD exists but cannot resolve.
        let repo = TempRepo::new("unborn");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        assert_eq!(capture_commit(&repo.dir), UNKNOWN, "unborn HEAD = unknown");
    }

    #[test]
    fn rerun_lines_cover_compiled_paths_and_git_state() {
        let repo = TempRepo::new("rerun");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        // The fixture must carry every COMPILED_PATHS entry it demands
        // coverage for — the list is existence-filtered by design (the
        // perpetual-dirty hazard), so an absent path is filtered, not
        // watched. Cargo.lock stays absent (cargo creates it on the first
        // build) and is the filtering test's subject, not this one's.
        std::fs::write(repo.dir.join("build.rs"), "fn main() {}\n").expect("write");
        std::fs::write(repo.dir.join("build_commit.rs"), "// probe copy\n").expect("write");
        std::fs::write(
            repo.dir.join("Cargo.toml"),
            "[package]\nname = \"rerun_fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
        )
        .expect("write");
        // The embedded-asset dirs (present in the real repo, filtered out
        // when absent — the filtering test below pins that direction).
        std::fs::create_dir_all(repo.dir.join("assets")).expect("mkdir assets");
        std::fs::create_dir_all(repo.dir.join("tests/fixtures")).expect("mkdir tests/fixtures");
        repo.commit_all("init");
        let lines = rerun_lines(&repo.dir);
        for p in COMPILED_PATHS {
            if p == &"Cargo.lock" {
                continue; // absent by fixture design — see the filter test
            }
            assert!(
                lines.iter().any(|l| l.ends_with(p)),
                "rerun list must cover compiled path {p}: {lines:?}"
            );
        }
        // HEAD + the reflog + the refs/heads dir + the branch ref file,
        // resolved through --git-path.
        assert!(lines.iter().any(|l| l.contains("HEAD")), "{lines:?}");
        assert!(lines.iter().any(|l| l.contains("logs/HEAD")), "{lines:?}");
        assert!(
            lines.iter().any(|l| l.ends_with("refs/heads")),
            "the refs/heads directory must be watched: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("refs/heads/")),
            "the branch ref file must be watched: {lines:?}"
        );
        // The first line disables cargo's any-file default — the list must
        // never be empty, or the stamp would freeze silently.
        assert!(!lines.is_empty());
    }

    /// ⛔ The perpetual-dirty hazard: cargo treats a MISSING watched path as
    /// dirty on every build, so every emitted path must exist AT EMISSION
    /// TIME — here in a fixture, and in the real checkout below.
    #[test]
    fn every_emitted_rerun_path_exists() {
        let repo = TempRepo::new("exists");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        repo.commit_all("init");
        for dir in ["", &repo.dir.display().to_string()] {
            let cwd = if dir.is_empty() {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            } else {
                repo.dir.clone()
            };
            for line in rerun_lines(&cwd) {
                let p = line
                    .strip_prefix("cargo:rerun-if-changed=")
                    .expect("every line is a rerun path");
                let path = std::path::Path::new(p);
                let exists = if path.is_absolute() {
                    path.exists()
                } else {
                    cwd.join(path).exists()
                };
                assert!(exists, "watched path {p:?} does not exist in {} — cargo would recompile on EVERY build: {line}", cwd.display());
            }
        }
    }

    /// The filtering itself: a compiled path that does not exist in a
    /// checkout is NOT emitted (the fixture has no assets/ or Cargo.lock
    /// before a build) — an emitted-but-missing path is the hazard above.
    #[test]
    fn missing_compiled_paths_are_filtered_not_emitted() {
        let repo = TempRepo::new("filter");
        repo.git(&["init", "-q", "--initial-branch=main"]);
        std::fs::write(repo.dir.join("src/lib.rs"), "pub fn f() {}\n").expect("write");
        repo.commit_all("init");
        let lines = rerun_lines(&repo.dir);
        assert!(!lines.iter().any(|l| l.contains("assets")), "{lines:?}");
        assert!(
            !lines.iter().any(|l| l.contains("tests/fixtures")),
            "{lines:?}"
        );
        // Cargo.lock is absent in a fresh fixture (git creates it on the
        // first cargo run) — filtered here, emitted where it exists.
        assert!(
            !lines.iter().any(|l| l.ends_with("Cargo.lock")),
            "{lines:?}"
        );
        assert!(lines.iter().any(|l| l.ends_with("src")), "{lines:?}");
    }
}

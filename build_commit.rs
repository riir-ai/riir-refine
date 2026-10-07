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
//! compile into this binary count (`src`, the build scripts, the manifest).

use std::path::Path;
use std::process::Command;

/// The literal exported when git is absent, the dir is not a checkout, or
/// HEAD cannot resolve (fresh `git init`, tarball build). Honest "cannot
/// see" — never a build error, never a guess.
pub const UNKNOWN: &str = "unknown";

/// Paths that compile into the binary — the dirty check's scope AND the
/// core of the rerun list. One list, two uses, so they cannot drift.
pub const COMPILED_PATHS: &[&str] = &[
    "src",
    "build.rs",
    "build_commit.rs",
    "Cargo.toml",
    "Cargo.lock",
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

/// The `cargo:rerun-if-changed=…` lines for build.rs to print: the compiled
/// paths plus the git state files. ORDER IS LOAD-BEARING ONLY FOR READERS —
/// cargo treats the set as a set.
///
/// ⛔ The FIRST rerun line disables cargo's any-file-change default, so this
/// list must stay a superset of everything that can move the stamp. A new
/// compiled path joins [`COMPILED_PATHS`] (one edit, both uses); the git
/// files are resolved per-checkout, never typed.
#[must_use]
pub fn rerun_lines(cwd: &Path) -> Vec<String> {
    let mut out: Vec<String> = COMPILED_PATHS
        .iter()
        .map(|p| format!("cargo:rerun-if-changed={p}"))
        .collect();
    // The files whose change moves `rev-parse HEAD`: HEAD itself (branch
    // switch), the packed-refs fallback, and the current branch's ref file
    // (where a commit actually lands on a normal checkout). `--git-path`
    // resolves through worktrees, where `.git` is a file. Detached HEAD
    // (CI checkouts) has no branch ref — HEAD + packed-refs still cover it.
    for query in ["HEAD", "packed-refs"] {
        if let Some(p) = git_path(cwd, query) {
            out.push(format!("cargo:rerun-if-changed={p}"));
        }
    }
    if let Some(branch) =
        git(cwd, &["symbolic-ref", "--short", "HEAD"]).map(|s| s.trim().to_string())
    {
        if !branch.is_empty() {
            if let Some(p) = git_path(cwd, &format!("refs/heads/{branch}")) {
                out.push(format!("cargo:rerun-if-changed={p}"));
            }
        }
    }
    out
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
        repo.commit_all("init");
        let lines = rerun_lines(&repo.dir);
        for p in COMPILED_PATHS {
            assert!(
                lines.iter().any(|l| l.ends_with(p)),
                "rerun list must cover compiled path {p}: {lines:?}"
            );
        }
        // HEAD + the branch ref file, resolved through --git-path.
        assert!(lines.iter().any(|l| l.contains("HEAD")), "{lines:?}");
        assert!(
            lines.iter().any(|l| l.contains("refs/heads/")),
            "the branch ref file must be watched: {lines:?}"
        );
        // The first line disables cargo's any-file default — the list must
        // never be empty, or the stamp would freeze silently.
        assert!(!lines.is_empty());
    }
}

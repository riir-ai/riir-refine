//! The auto-oracle (Issue 113): a scanning run over a cargo project lints it
//! with REAL clippy itself, so oracle-anchored spans (the ones that reproduce
//! under the proof oracle and can credit) become the default instead of a
//! flag nobody discovers.
//!
//! Before this module the feed was manual-only (`--clippy-json`): the caller
//! ran clippy and piped the stream in, because a guessed invocation is
//! silently blind (wrong feature set → cfg-gated code compiles to nothing;
//! toolchain pins; target-dir locks). The owner call (2026-09-16, the
//! "agave 6951 rows → +1 credit" session): `cargo heal --mine` must do every
//! good for the user — the manual feed kept that promise only for
//! AGENTS.md readers. The resolution keeps BOTH: the auto-run is the default
//! (spawn `cargo clippy --workspace --no-deps --message-format=json --quiet`
//! at DEFAULT features in the detected project root, `$CARGO`-resolved),
//! falls back LOUD to heuristic-only whenever it cannot run (no cargo,
//! spawn failure, compile error with zero parseable diagnostics), and the
//! explicit kill switch `RIIR_HEAL_AUTO_ORACLE=0` restores the old
//! never-spawn posture. A manual `--clippy-json` still wins outright.
//!
//! Precedent: `--fix-compile` already spawns cargo through
//! [`crate::compile_oracle`] — the healer spawning the user's toolchain is
//! not a new class. What stays true: the auto-run never picks features for
//! you (default features only, said so in the status line) and never
//! silently changes the verdict of a run it could not anchor.
//!
//! Default-features scoping is honest, not complete: cfg-gated code compiles
//! to nothing under it (the workspace's own blindness class). The status
//! line names the arm; widening (feature sets, targets) is a later owner
//! call, not a silent guess.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Kill-switch predicate over the raw `RIIR_HEAL_AUTO_ORACLE` value.
/// Only explicit falsy spellings disable; unknown values default ON (a typo
/// must not silently restore the low-yield posture this module exists to fix).
#[must_use]
pub fn enabled_from(v: Option<&str>) -> bool {
    !matches!(v.map(str::trim), Some("0" | "false" | "no" | "off"))
}

/// The live env read.
#[must_use]
pub fn auto_oracle_enabled() -> bool {
    enabled_from(std::env::var("RIIR_HEAL_AUTO_ORACLE").ok().as_deref())
}

/// Nearest cargo project root for the scan targets (v1: the FIRST target —
/// multi-project scans get one oracle run, loudly scoped by the status
/// line). A directory is probed for its own manifest, then up to
/// [`MAX_ANCESTOR_STEPS`] parents; a file starts at its parent. Bounded so a
/// stray scan under an unrelated ancestor never picks up that project.
const MAX_ANCESTOR_STEPS: usize = 4;

#[must_use]
pub fn detect_project_root(paths: &[PathBuf], cwd: &Path) -> Option<PathBuf> {
    let start: &Path = paths.first().map_or(cwd, |p| p.as_path());
    let mut dir: &Path = if start.is_dir() {
        start
    } else {
        start.parent()?
    };
    for _ in 0..=MAX_ANCESTOR_STEPS {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

/// One oracle invocation's raw result: the JSON stream (lossy-decoded —
/// clippy escapes non-ASCII in JSON, lossy only defends against a
/// misbehaving toolchain) and the exit code (None = killed by signal).
pub struct OracleRun {
    pub text: String,
    pub exit: Option<i32>,
}

/// Spawn `cargo clippy --workspace --no-deps --message-format=json --quiet`
/// in `root`.
///
/// # Errors
///
/// Returns the spawn failure verbatim — the caller degrades to
/// heuristic-only with this message; a spawned-but-failing clippy is NOT an
/// error here (its exit code travels in [`OracleRun`]).
pub fn run_clippy(root: &Path, cargo_bin: &str) -> Result<OracleRun, String> {
    let output = Command::new(cargo_bin)
        .args(["clippy", "--workspace", "--no-deps", "--message-format=json", "--quiet"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot spawn `{cargo_bin} clippy`: {e}"))?;
    Ok(OracleRun {
        text: String::from_utf8_lossy(&output.stdout).into_owned(),
        exit: output.status.code(),
    })
}

/// What a finished run means for the feed.
pub enum OracleOutcome {
    /// Parsed ≥1 clippy diagnostic — anchor them (a non-zero exit alongside
    /// parsed diagnostics is a PARTIAL compile: crates that built were
    /// linted; the diagnostics are real either way).
    Feed(usize),
    /// Exit 0 with zero diagnostics — a genuinely clean tree at default
    /// features; an empty feed (marks covered files heuristic, same as the
    /// manual 0-diagnostic feed).
    Clean,
    /// Non-zero exit and nothing parseable — the compile itself failed;
    /// heuristic-only, never a fabricated "clean".
    Failed(String),
}

#[must_use]
pub fn classify(exit: Option<i32>, n_parsed: usize) -> OracleOutcome {
    if n_parsed > 0 {
        return OracleOutcome::Feed(n_parsed);
    }
    match exit {
        Some(0) => OracleOutcome::Clean,
        Some(code) => OracleOutcome::Failed(format!(
            "clippy exited {code} with no parseable diagnostics (compile error?)"
        )),
        None => OracleOutcome::Failed(
            "clippy was killed by a signal with no parseable diagnostics".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join("cargo-heal-clippy-oracle-tests")
            .join(format!("{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn kill_switch_only_explicit_falsy_spellings() {
        assert!(enabled_from(None));
        assert!(enabled_from(Some("")));
        assert!(enabled_from(Some("1")));
        assert!(enabled_from(Some("on")));
        assert!(enabled_from(Some(" yes ")), "trimmed truthy stays on");
        assert!(!enabled_from(Some("0")));
        assert!(!enabled_from(Some("false")));
        assert!(!enabled_from(Some("off")));
        assert!(enabled_from(Some("2")), "unknown values default ON, not silent-off");
    }

    #[test]
    fn detects_a_direct_project_dir() {
        let d = tmp_dir("direct");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert_eq!(
            detect_project_root(std::slice::from_ref(&d), Path::new("/nowhere")),
            Some(d)
        );
    }

    #[test]
    fn detects_the_parent_of_a_file_target() {
        let d = tmp_dir("file");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let f = d.join("src").join("main.rs");
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        fs::write(&f, "fn main() {}").unwrap();
        assert_eq!(
            detect_project_root(&[f], Path::new("/nowhere")),
            Some(d)
        );
    }

    #[test]
    fn empty_paths_fall_back_to_cwd() {
        let d = tmp_dir("cwd");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        assert_eq!(detect_project_root(&[], &d), Some(d));
    }

    #[test]
    fn no_project_within_bounds_is_none_not_a_walk_to_the_root() {
        let base = tmp_dir("none");
        let nested = base.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(detect_project_root(&[nested], Path::new("/nowhere")), None);
    }

    #[test]
    fn classify_splits_feed_clean_failed() {
        assert!(matches!(classify(Some(0), 3), OracleOutcome::Feed(3)));
        // Partial compile: diagnostics exist despite the failure.
        assert!(matches!(classify(Some(101), 2), OracleOutcome::Feed(2)));
        assert!(matches!(classify(Some(0), 0), OracleOutcome::Clean));
        assert!(matches!(classify(Some(101), 0), OracleOutcome::Failed(_)));
        assert!(matches!(classify(None, 0), OracleOutcome::Failed(_)));
    }
}

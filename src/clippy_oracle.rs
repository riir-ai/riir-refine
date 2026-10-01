//! The auto-oracle (Issue 113): a scanning run over a cargo project lints it
//! with REAL clippy itself, so oracle-anchored spans (the ones that reproduce
//! under the proof oracle and can credit) become the default instead of a
//! flag nobody discovers.
//!
//! Before this module the feed was manual-only (`--clippy-json`): the caller
//! ran clippy and piped the stream in, because a guessed invocation is
//! silently blind (wrong feature set → cfg-gated code compiles to nothing;
//! toolchain pins; target-dir locks). The owner call (2026-09-16, the
//! "agave 6951 rows → +1 credit" session): `cargo refine --mine` must do every
//! good for the user — the manual feed kept that promise only for
//! AGENTS.md readers. The resolution keeps BOTH: the auto-run is the default
//! (spawn `cargo clippy --workspace --no-deps --message-format=json --quiet`
//! at DEFAULT features in the detected project root, `$CARGO`-resolved),
//! falls back LOUD to heuristic-only whenever it cannot run (no cargo,
//! spawn failure, compile error with zero parseable diagnostics), and the
//! explicit kill switch `RIIR_REFINE_AUTO_ORACLE=0` restores the old
//! never-spawn posture. A manual `--clippy-json` still wins outright.
//!
//! Precedent: `--fix-compile` already spawns cargo through
//! [`crate::compile_oracle`] — the healer spawning the user's toolchain is
//! not a new class. What stays true: the auto-run never GUESSES features
//! for you (default features unless you say otherwise, said so in the
//! status line) and never silently changes the verdict of a run it could
//! not anchor.
//!
//! Default-features scoping is honest, not complete: cfg-gated code compiles
//! to nothing under it (the workspace's own blindness class —
//! `consumer_residue_sweep.sh` runs `--all-features` for exactly this
//! reason). Issue 115 added the operator re-aim:
//! `RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv>` spawns with `--features <csv>`
//! (a loud status line when active — a feature-superset clippy run is
//! materially slower; unknown feature names fail the clippy spawn, which
//! degrades LOUD to heuristic-only naming the knob). `--all-features`
//! stays a deliberate non-default (cost + sibling feature-combo explosion);
//! the auto-run never widens its own scope that way.
//!
//! `--all-targets` IS the default (2026-09-21, the riir-chain/riir-ai intake
//! pair): without it the spawn lints only lib targets, so every `mod tests`
//! span — where fixture code lives, and where BOTH intake incidents sat — is
//! invisible to the anchor lane (a `manual_range_contains` in `mod tests`
//! was hand-applied while the oracle ran blind beside it). A knob nobody
//! passes is not a repair, so the WIDE posture is the default and
//! `RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS=1` restores the fast lib-only one
//! (only explicit truthy spellings disable; a typo must not silently
//! restore the blind posture).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Kill-switch predicate over the raw `RIIR_REFINE_AUTO_ORACLE` value.
/// Only explicit falsy spellings disable; unknown values default ON (a typo
/// must not silently restore the low-yield posture this module exists to fix).
#[must_use]
pub fn enabled_from(v: Option<&str>) -> bool {
    !matches!(v.map(str::trim), Some("0" | "false" | "no" | "off"))
}

/// The live env read.
#[must_use]
pub fn auto_oracle_enabled() -> bool {
    enabled_from(std::env::var("RIIR_REFINE_AUTO_ORACLE").ok().as_deref())
}

/// Parsed `RIIR_REFINE_AUTO_ORACLE_FEATURES` value (Issue 115): `None` = the
/// default-features posture (unset, blank, or nothing but empty segments);
/// `Some(non-empty)` = spawn clippy with `--features <csv>`. The oracle can
/// only name features the TARGET project's manifests carry — unknown names
/// fail the clippy spawn, which the caller degrades to heuristic-only LOUD
/// (never a silent un-anchored run that looks anchored).
#[must_use]
pub fn features_from(v: Option<&str>) -> Option<Vec<String>> {
    let raw = v?.trim();
    if raw.is_empty() {
        return None;
    }
    let list: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if list.is_empty() { None } else { Some(list) }
}

/// The live env read.
#[must_use]
pub fn auto_oracle_features() -> Option<Vec<String>> {
    features_from(std::env::var("RIIR_REFINE_AUTO_ORACLE_FEATURES").ok().as_deref())
}

/// Kill-switch predicate over the raw
/// `RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS` value (2026-09-21): the
/// `--all-targets` posture is the DEFAULT (test modules are half the
/// healing surface); only explicit truthy spellings disable it. Unknown
/// values keep it ON — a typo must not silently restore the blind posture.
#[must_use]
pub fn all_targets_from(v: Option<&str>) -> bool {
    !matches!(v.map(str::trim), Some("1" | "true" | "yes" | "on"))
}

/// The live env read.
#[must_use]
pub fn auto_oracle_all_targets() -> bool {
    all_targets_from(
        std::env::var("RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS")
            .ok()
            .as_deref(),
    )
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
    let mut dir: PathBuf = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    // Anchor to the caller's cwd BEFORE walking up: a RELATIVE 1-component
    // target (`cargo refine --suggest src/` — the documented invocation form)
    // parents to the EMPTY path, and `"".join("Cargo.toml")` then resolves
    // against the process cwd — returning `Some("")` as the "root", whose
    // `Command::current_dir("")` spawn fails with os error 123 on Windows /
    // ENOENT on Unix. The exact class `compile_oracle::owning_package_root`
    // already fixed for the --fix-compile lane (the Issue 015 GOAT run's
    // spawn failure); ported here when the 2026-09-20 4090 sweep hit it on
    // the auto-oracle.
    if dir.as_os_str().is_empty() {
        dir = cwd.to_path_buf();
    } else if dir.is_relative() {
        // `cwd.join(".")` would leave a trailing CurDir component
        // ("<cwd>/." — spawns fine but is neither displayable nor
        // PathBuf-equal to the root it names); normalize the bare-dot
        // input to cwd itself.
        dir = if dir == Path::new(".") {
            cwd.to_path_buf()
        } else {
            cwd.join(dir)
        };
    }
    for _ in 0..=MAX_ANCESTOR_STEPS {
        if dir.join("Cargo.toml").is_file() {
            return Some(dir);
        }
        dir = dir.parent()?.to_path_buf();
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

/// Spawn `cargo clippy --workspace --no-deps --message-format=json --quiet
/// [--all-targets] [--features <csv>]` in `root`.
///
/// `--all-targets` (the default posture — see [`all_targets_from`]) lints
/// test/bench/example targets too, so `mod tests` spans anchor like any
/// other. `features` is the Issue-115 re-aim (`None` = default features).
///
/// # Errors
///
/// Returns the spawn failure verbatim — the caller degrades to
/// heuristic-only with this message; a spawned-but-failing clippy is NOT an
/// error here (its exit code travels in [`OracleRun`]).
pub fn run_clippy(
    root: &Path,
    cargo_bin: &str,
    features: Option<&[String]>,
    all_targets: bool,
) -> Result<OracleRun, String> {
    let mut cmd = Command::new(cargo_bin);
    cmd.args([
        "clippy",
        "--workspace",
        "--no-deps",
        "--message-format=json",
        "--quiet",
    ]);
    if all_targets {
        cmd.arg("--all-targets");
    }
    if let Some(set) = features {
        cmd.arg("--features").arg(set.join(","));
    }
    let output = cmd
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
            .join("cargo-refine-clippy-oracle-tests")
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
    fn all_targets_default_on_only_explicit_truthy_disables() {
        assert!(all_targets_from(None));
        assert!(all_targets_from(Some("")));
        assert!(all_targets_from(Some("0")));
        assert!(all_targets_from(Some("false")), "falsy keeps the wide posture");
        assert!(all_targets_from(Some("2")), "unknown values keep it ON");
        assert!(!all_targets_from(Some("1")));
        assert!(!all_targets_from(Some("true")));
        assert!(!all_targets_from(Some(" on ")), "trimmed truthy disables");
    }

    #[test]
    fn features_knob_unset_blank_or_empty_segments_is_default_posture() {
        assert_eq!(features_from(None), None);
        assert_eq!(features_from(Some("")), None);
        assert_eq!(features_from(Some("   ")), None);
        assert_eq!(features_from(Some(",, ,,")), None, "nothing but empty segments");
    }

    #[test]
    fn features_knob_parses_trimmed_csv_dropping_empty_segments() {
        assert_eq!(features_from(Some("gated")), Some(vec!["gated".to_owned()]));
        assert_eq!(
            features_from(Some(" a , b ,c")),
            Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
        );
        assert_eq!(
            features_from(Some("a ,, b")),
            Some(vec!["a".to_owned(), "b".to_owned()]),
            "interior empty segments drop, the rest keep their order"
        );
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
    fn one_component_relative_target_resolves_against_cwd_never_empty() {
        // The documented invocation shape: `cargo refine --suggest src/` run
        // from the repo root. Before the cwd-anchor fix the walk went
        // `src` → `""` → `"".join("Cargo.toml")` resolves against the
        // process cwd → `Some("")` — an empty "root" whose spawn failed
        // with os error 123 (the 2026-09-20 4090 sweep find). Both arms of
        // the start ("src" resolves as a dir at process level, or not)
        // must now converge on the ABSOLUTE project root.
        let d = tmp_dir("relative");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::create_dir_all(d.join("src")).unwrap();
        let root =
            detect_project_root(&[PathBuf::from("src")], &d).expect("root found via cwd");
        assert_eq!(root, d, "the cwd-anchored walk finds the tmp project");
        assert!(root.is_absolute(), "a relative root would spawn against the healer's own cwd");
    }

    #[test]
    fn relative_file_target_resolves_via_cwd_not_the_empty_parent() {
        // `cargo refine --suggest src/lib.rs` from the repo root: the file's
        // parent ("src") is relative; the empty-path walk used to fire only
        // for 1-component DIRS, but the anchor must hold for files too.
        let d = tmp_dir("relative-file");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        fs::create_dir_all(d.join("src")).unwrap();
        fs::write(d.join("src").join("lib.rs"), "").unwrap();
        let root = detect_project_root(&[PathBuf::from("src/lib.rs")], &d)
            .expect("root found via cwd");
        assert_eq!(root, d);
        assert!(root.is_absolute());
    }

    #[test]
    fn bare_dot_target_is_the_cwd_project() {
        // `cargo refine --suggest .` — the dir "." is relative; anchored to
        // cwd it IS the project. Must never walk through "" to find it.
        let d = tmp_dir("dot");
        fs::write(d.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        let root =
            detect_project_root(&[PathBuf::from(".")], &d).expect("root found via cwd");
        assert_eq!(root, d);
        assert!(root.is_absolute());
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

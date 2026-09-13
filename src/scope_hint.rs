//! Scan-scope instrument hint (the 2026-09-14 self-audit intake row).
//!
//! `--config-audit` / `--orphan-report` adjudicate field reads against the
//! SCANNED set only — a `src`-only invocation over-reports stillborn rows
//! whose readers live in `examples/`, `benches/`, or `tests/` (live
//! specimen: `PavDataConfig.ground_truth`, flagged on a `src`-only scan
//! while the reader sat in `examples/pav_data_curate.rs`).
//!
//! The hint NEVER widens the scan — explicit paths stay the operator's
//! scope statement (same law as `DOCS_GATE_PARTIAL_CLONE`: never
//! auto-detect, make the blind half visible). It fires only on the
//! PARTIALLY-widened shape (at least one standard root covered, at least
//! one present-but-uncovered): a `.` scan covers everything (nothing to
//! say), a single-file targeted scan covers no root at all (the narrowing
//! is obviously deliberate — nothing to say).

use std::path::{Path, PathBuf};

/// The standard Rust source roots a read-evidence scan may need.
const STANDARD_ROOTS: [&str; 4] = ["src", "examples", "tests", "benches"];

/// Walk up from `start` to the nearest ancestor holding a `Cargo.toml`
/// (the crate root that owns the scanned paths). `None` when none exists
/// — the hint never guesses.
fn crate_root(start: &Path) -> Option<&Path> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            return Some(d);
        }
        dir = d.parent();
    }
    None
}

/// The one-line instrument-alive hint, or `""` when the scan shape says
/// nothing (full scope, no partial widening, or no inferable crate root).
///
/// A standard root is COVERED when some scanned path IS the root or an
/// ancestor of it (canonical comparison — cross-repo relative targets
/// like `../riir-ai/src` resolve correctly).
#[must_use]
pub fn unscanned_roots_hint(targets: &[PathBuf]) -> String {
    let Some(first) = targets.first() else {
        return String::new();
    };
    // Canonicalize FIRST: a relative target (the common `src` shape) walks
    // up to the empty path, whose Cargo.toml probe is CWD-relative but
    // whose canonicalize() FAILS on Windows — the absolute walk is the
    // only form that survives both halves (found by the live probe; the
    // unit tests' absolute temp paths masked it).
    let Ok(start_abs) = first.canonicalize() else {
        return String::new();
    };
    let start = if start_abs.is_file() {
        let Some(parent) = start_abs.parent() else {
            return String::new();
        };
        parent
    } else {
        start_abs.as_path()
    };
    let Some(root) = crate_root(start) else {
        return String::new();
    };
    // `root` is absolute already (walked from the canonicalized start).
    let root_abs = root.to_path_buf();

    let covered: Vec<PathBuf> = targets
        .iter()
        .filter_map(|t| t.canonicalize().ok())
        .filter(|t| t.is_dir())
        .collect();
    if covered.is_empty() {
        // No scanned directory survived canonicalization — a file-only or
        // stale-path scan. The narrowing is deliberate; nothing to say.
        return String::new();
    }

    let mut missing: Vec<&'static str> = Vec::new();
    let mut covered_roots = 0usize;
    for name in STANDARD_ROOTS {
        let root_dir = root_abs.join(name);
        if !root_dir.is_dir() {
            continue;
        }
        let is_covered = covered
            .iter()
            .any(|c| *c == root_dir || root_dir.starts_with(c));
        if is_covered {
            covered_roots += 1;
        } else {
            missing.push(name);
        }
    }
    if covered_roots == 0 || missing.is_empty() {
        return String::new();
    }
    format!(
        "scope hint: unscanned sibling roots present on disk: {} — readers there are invisible to this report (pass them to widen the read-evidence scope)",
        missing.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fake_crate(tag: &str) -> PathBuf {
        fake_crate_at(&std::env::temp_dir().join(format!("scope_hint_{tag}")), tag)
    }

    fn fake_crate_at(base: &Path, tag: &str) -> PathBuf {
        let root = base.join(format!("{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("examples")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        // No benches/ — the absent root must never be named.
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        root
    }

    #[test]
    fn partial_widening_names_the_uncovered_roots() {
        let root = fake_crate("partial");
        let hint = unscanned_roots_hint(&[root.join("src")]);
        assert!(hint.contains("examples"), "names examples: {hint}");
        assert!(hint.contains("tests"), "names tests: {hint}");
        assert!(!hint.contains("benches"), "absent root never named: {hint}");
        assert!(!hint.contains("src,"), "the covered root is not named: {hint}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn full_scope_is_silent() {
        let root = fake_crate("full");
        assert_eq!(unscanned_roots_hint(std::slice::from_ref(&root)), "");
        assert_eq!(
            unscanned_roots_hint(&[root.join("src"), root.join("examples"), root.join("tests")]),
            ""
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn single_file_target_is_silent() {
        let root = fake_crate("file");
        let f = root.join("src").join("lib.rs");
        fs::write(&f, "pub fn x() {}\n").unwrap();
        assert_eq!(unscanned_roots_hint(&[f]), "");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_targets_are_silent() {
        assert_eq!(unscanned_roots_hint(&[]), "");
    }

    #[test]
    fn relative_target_paths_fire_too() {
        // The live-probe regression: a relative `src` argument walks up to
        // the EMPTY path whose canonicalize() fails on Windows — the hint
        // went silent exactly when it mattered. The fake crate goes under
        // `target/` (gitignored, guaranteed same volume as cwd — the OS
        // temp can sit on another drive and make a relative form
        // impossible) and the hint must fire through the relative path.
        let cwd = std::env::current_dir().unwrap();
        let base = cwd.join("target").join("scope_hint_reltest");
        let root = fake_crate_at(&base, "relpath");
        // The PARTIAL shape: scan the fake crate's src/ through a relative
        // path — examples/tests stay uncovered, so the hint must fire.
        let src_dir = root.join("src");
        let rel = src_dir.strip_prefix(&cwd).unwrap();
        let hint = unscanned_roots_hint(&[rel.to_path_buf()]);
        assert!(hint.contains("examples"), "relative src fires: {hint}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn live_shape_relative_src_from_this_repo_root() {
        // The exact live-probe shape: this test process runs with CWD =
        // the REAL repo root, so `src` resolves against it — the same
        // invocation shape as `cargo heal --orphan-report src`.
        let hint = unscanned_roots_hint(&[PathBuf::from("src")]);
        assert!(
            hint.contains("examples") && hint.contains("benches"),
            "the real repo's src-only scan names its uncovered roots: {hint}"
        );
    }
}

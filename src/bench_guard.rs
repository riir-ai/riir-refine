//! The bench-target guard (the 2026-09-16 post-mining intake — P30; scope
//! extended by Issue 118, same day).
//!
//! Live evidence (katgpt-rs `448c77f91` clippy-sweep session): a dry run
//! over `crates/katgpt-core/benches/bench_775_adjoint_goat.rs` proposed
//! `[perf] dead-result-timing-loop-folded-under-lto → std::hint::black_box`
//! (+ loop/alloc restructures) at bench spans. Applying any of them changes
//! WHAT the bench measures — `--verify` compiles baseline/after but cannot
//! know whether the measured quantity survived; a benchmark file's numbers
//! ARE its artifact (GOAT gate evidence), and a timing loop folded under LTO
//! is often precisely the defect the bench exists to expose.
//!
//! Policy — **DECLINE-OR-FLAG, never silence**: the fix lane declines
//! `rust_perf` edits on bench targets with a loud `bench-guard:` line naming
//! the count; the suggest lane keeps every suggestion visible but stamps
//! perf rows with a `⚠ bench-guard` marker. Either way the operator sees
//! why the bench lint was left standing (the intake's requirement).
//!
//! Scope — PERF (P30) + the compile-repair surfaces (Issue 118):
//! - the `rustc_errors` `--fix-compile` WRITE path declines bench-file
//!   candidates (a compile repair can hoist a loop, change a type's
//!   size/copy semantics, or delete the "dead" result that was the timing
//!   subject — the exact specimen class that motivated P30 was itself
//!   compile-shaped; P30's founding note called making a bench COMPILE
//!   not measurement-altering, and Issue 118 re-adjudicated that claim);
//! - the P21 `--clippy-json` oracle-edits lane rides the same door (a
//!   `MachineApplicable` suggestion consumed VERBATIM can be structural —
//!   the P15 two-part rename+delete shape);
//! - the explicit-path override ([`user_path_forces`]): naming the bench
//!   file (or its `benches/` directory) on the command line prints the
//!   flag and proceeds — the operator's consent, never a silent
//!   hard-refusal.
//!
//! Still out of scope: the healer's OWN clippy-lint proposer lane (bounded
//! style rewrites), sec, docker and dist.
//!
//! This module lives at the crate TOP LEVEL (ungated) so the `rustc_errors`
//! driver can consume it without gating on the `rust_perf` feature;
//! `domains::rust_perf::bench_guard` re-exports it for the established
//! path (the P30 call sites).
//!
//! Detection is deliberately CONSERVATIVE: over-matching means a false
//! DECLINE, which is operator-visible and manually overridable;
//! under-matching means the defect class persists silently. Two axes:
//! - **path** — any `benches` path component (cargo compiles everything
//!   under `benches/` as bench-target infrastructure: the targets AND the
//!   helper modules they include);
//! - **syntax** — the common harness signatures (`#[bench]`,
//!   `criterion_group!`/`criterion_main!`, `bench_function(`,
//!   `#[divan::bench]`, `iai::bench`, `bencher::`). String-contains, so a
//!   comment mentioning a harness can over-match — the safe direction
//!   here, and cheaper than a syn walk at the read site.
//!
//! NOT a marker: `std::hint::black_box` in the source. It appears in
//! production code (it is the FIX half of perf rules) and in benches —
//! keying on it would decline every file that already uses it correctly.

/// Harness signatures for the syntax axis (see [`is_bench_target`]).
/// A comment containing one of these over-matches to `true` — documented,
/// and the safe direction for a decline guard.
const HARNESS_MARKERS: [&str; 7] = [
    "#[bench]",
    "criterion_group!",
    "criterion_main!",
    "bench_function(",
    "#[divan::bench]",
    "iai::bench",
    "bencher::",
];

/// Is this file a measurement target — a cargo bench target, a helper
/// module under `benches/`, or any file invoking a bench harness?
///
/// Conservative by construction (see module docs): over-match declines
/// loudly and is manually overridable; under-match is the silent defect.
#[must_use]
pub fn is_bench_target(path: &str, src: &str) -> bool {
    // Path axis: normalize separators so a Windows display path matches.
    let norm = path.replace('\\', "/");
    if norm.split('/').any(|c| c == "benches") {
        return true;
    }
    // Syntax axis: harness signatures.
    HARNESS_MARKERS.iter().any(|m| src.contains(m))
}

/// Lexically normalize a path to comparable components: `\` → `/`, drop
/// `.`, resolve `..` by popping. Symlinks are NOT resolved here — callers
/// get that via best-effort canonicalization in [`user_path_forces`].
fn norm_components(p: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in p.to_string_lossy().replace('\\', "/").split('/') {
        match c {
            "" | "." => {}
            ".." => {
                let _ = out.pop();
            }
            other => out.push(other.to_string()),
        }
    }
    out
}

/// Does the operator's explicit path `user` FORCE `file` past the bench
/// guard? (Issue 118 T3 — the override posture.)
///
/// Two shapes count as consent, and nothing else:
/// - `user` NAMES the exact file (`cargo heal --fix-compile --write
///   benches/bench_x.rs`);
/// - `user` IS a `benches` directory above the file (`--fix-compile
///   benches/`) — the component must equal `benches`, so a crate root or
///   `.` (routine discovery) never forces, and neither does
///   `benches_foo/`.
///
/// Best-effort canonicalization on both sides first (both paths normally
/// exist at call time — the driver walked them), then lexical fallback —
/// so a relative user argument still matches an absolute diagnostic path.
/// Over-forcing is impossible without the operator typing a bench path;
/// under-forcing just leaves the loud decline standing (name the file).
#[must_use]
pub fn user_path_forces(user: &std::path::Path, file: &std::path::Path) -> bool {
    let u = std::fs::canonicalize(user).unwrap_or_else(|_| user.to_path_buf());
    let f = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    lexical_forces(&norm_components(&u), &f)
}

/// The pure core of [`user_path_forces`] over already-normalized user
/// components — the testable half (fixtures use nonexistent roots so the
/// canonicalize fallback never fires).
fn lexical_forces(user: &[String], file: &std::path::Path) -> bool {
    if user.is_empty() {
        return false;
    }
    let fc = norm_components(file);
    if fc.is_empty() {
        return false;
    }
    // Exact file: the operator typed this path.
    if user == fc.as_slice() {
        return true;
    }
    // Directory ancestor — but ONLY when the named directory IS a
    // `benches` dir (naming a crate root or `.` is routine discovery).
    fc.starts_with(user) && user.last().is_some_and(|c| c == "benches")
}

#[cfg(test)]
mod tests {
    use super::{is_bench_target, lexical_forces, norm_components, user_path_forces};
    use std::path::Path;

    /// The exact intake specimen path (katgpt-rs `448c77f91` session).
    #[test]
    fn intake_specimen_path_detected() {
        assert!(is_bench_target(
            "crates/katgpt-core/benches/bench_775_adjoint_goat.rs",
            "fn plain() {}"
        ));
    }

    #[test]
    fn windows_separators_normalized() {
        assert!(is_bench_target(
            "E:\\git\\katgpt-rs\\crates\\katgpt-core\\benches\\bench_x.rs",
            ""
        ));
    }

    #[test]
    fn helper_module_under_benches_detected() {
        // benches/common/mod.rs is bench infrastructure by the same rule —
        // a perf edit in the setup helper can alter the measured section
        // it feeds.
        assert!(is_bench_target("benches/common/mod.rs", "pub fn mk() {}"));
    }

    #[test]
    fn benches_prefix_component_is_not_benches() {
        // A component must EQUAL `benches` — `benches_foo` is a different
        // directory and must fall through to the syntax axis.
        assert!(!is_bench_target(
            "src/benches_helper.rs",
            "fn plain() -> u32 { 1 }"
        ));
    }

    #[test]
    fn libtest_bench_attribute_detected() {
        assert!(is_bench_target(
            "src/anywhere.rs",
            "#[bench]\nfn timing(b: &mut test::Bencher) { b.iter(|| 1 + 1) }\n"
        ));
    }

    #[test]
    fn criterion_macros_detected() {
        assert!(is_bench_target(
            "src/criterion_lane.rs",
            "criterion_group! { benches = [sum] }\ncriterion_main! { benches }\n"
        ));
        assert!(is_bench_target(
            "src/criterion_lane.rs",
            "c.bench_function(\"sum\", |b| b.iter(|| 1 + 1));\n"
        ));
    }

    #[test]
    fn divan_iai_bencher_detected() {
        assert!(is_bench_target("src/d.rs", "#[divan::bench]\nfn f() {}\n"));
        assert!(is_bench_target("src/i.rs", "iai::bench! { f }"));
        assert!(is_bench_target("src/b.rs", "bencher::bench_repeatedly!"));
    }

    #[test]
    fn plain_production_code_not_detected() {
        assert!(!is_bench_target(
            "src/lib.rs",
            "fn label() -> String { \"ready\".to_string() }\nfn main() {}\n"
        ));
    }

    #[test]
    fn black_box_is_not_a_marker() {
        // black_box in the source is the FIX half of perf rules and legal
        // in production code — keying on it would decline every correct
        // user of it.
        assert!(!is_bench_target(
            "src/hot.rs",
            "std::hint::black_box(&result);\n"
        ));
    }

    #[test]
    fn comment_mentioning_harness_over_matches_documented() {
        // Conservative direction, documented in the module docs: a comment
        // naming `#[bench]` reads as a bench file. A false decline is
        // loud and overridable; the miss direction is the defect.
        assert!(is_bench_target("src/doc.rs", "// see the #[bench] file\n"));
    }

    // ── Issue 118 T3: the explicit-path override matcher ─────────────────

    /// Nonexistent roots so canonicalize falls back to the lexical core.
    const REPO: &str = "/w118-no-such-root/repo";

    fn user(rel: &str) -> Vec<String> {
        norm_components(Path::new(&format!("{REPO}/{rel}")))
    }

    fn file(rel: &str) -> std::path::PathBuf {
        Path::new(REPO).join(rel)
    }

    #[test]
    fn exact_file_match_forces() {
        assert!(lexical_forces(
            &user("benches/bench_x.rs"),
            &file("benches/bench_x.rs")
        ));
    }

    #[test]
    fn different_file_does_not_force() {
        assert!(!lexical_forces(
            &user("benches/bench_y.rs"),
            &file("benches/bench_x.rs")
        ));
    }

    #[test]
    fn benches_directory_ancestor_forces() {
        assert!(lexical_forces(&user("benches"), &file("benches/bench_x.rs")));
        // Nested: the named dir IS a benches dir one level down.
        assert!(lexical_forces(
            &user("crates/foo/benches"),
            &file("crates/foo/benches/bench_x.rs")
        ));
    }

    #[test]
    fn repo_root_does_not_force() {
        // `--fix-compile .` is routine discovery — never consent.
        assert!(!lexical_forces(&user("."), &file("benches/bench_x.rs")));
        assert!(!lexical_forces(
            &user("crates/foo"),
            &file("crates/foo/benches/bench_x.rs")
        ));
    }

    #[test]
    fn benches_prefix_directory_does_not_force() {
        // Component equality, not prefix: `benches_foo` is not `benches`.
        assert!(!lexical_forces(
            &user("benches_foo"),
            &file("benches_foo/bench_x.rs")
        ));
    }

    #[test]
    fn src_file_match_forces_but_src_dir_does_not() {
        assert!(lexical_forces(&user("src/lib.rs"), &file("src/lib.rs")));
        assert!(!lexical_forces(&user("src"), &file("src/lib.rs")));
    }

    #[test]
    fn dotdot_and_backslash_normalize() {
        assert!(lexical_forces(
            &user("benches/../benches/bench_x.rs"),
            &file("benches/bench_x.rs")
        ));
        assert!(lexical_forces(
            &norm_components(Path::new("\\w118-no-such-root\\repo\\benches")),
            &file("benches/bench_x.rs")
        ));
    }

    #[test]
    fn user_path_forces_end_to_end_lexical_fallback() {
        // Both paths nonexistent → canonicalize fails on both → lexical.
        assert!(user_path_forces(
            Path::new("/w118-no-such-root/repo/benches/bench_x.rs"),
            Path::new("/w118-no-such-root/repo/benches/bench_x.rs"),
        ));
        assert!(!user_path_forces(
            Path::new("/w118-no-such-root/repo"),
            Path::new("/w118-no-such-root/repo/benches/bench_x.rs"),
        ));
    }
}

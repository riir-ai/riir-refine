//! The bench-target guard (the 2026-09-16 post-mining intake — P30).
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
//! rust_perf edits on bench targets with a loud `bench-guard:` line naming
//! the count; the suggest lane keeps every suggestion visible but stamps
//! perf rows with a `⚠ bench-guard` marker. Either way the operator sees
//! why the bench lint was left standing (the intake's requirement).
//!
//! Scope — the PERF domain only. Clippy lint fixes, the rustc_errors
//! `--fix-compile` lane (making a bench COMPILE is not measurement-
//! altering), sec, docker and dist are out of scope: the recorded defect
//! class is perf rewrites falsifying a measurement.
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

/// Harness signatures for the syntax axis (see [`is_bench_target"]).
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

#[cfg(test)]
mod tests {
    use super::is_bench_target;

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
}

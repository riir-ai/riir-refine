//! Issue 025 T5 — the non-syn scanner GOAT control: a REAL git-history
//! fix pair, committed as fixtures, pinning the mining-loop garbage-in guard.
//!
//! Fixtures (extracted 2026-08-16 from the riir-ai sibling):
//! - `m2p_attention_prefix_701.wgsl` — `8bb716954^` (pre-fix): the fused
//!   out-projection computing `attn_val * rowsum(wo)` — the Issue 701 H-C
//!   wrong-math bug.
//! - `m2p_attention_postfix_701.wgsl` — `8bb716954` (post-fix): two-phase
//!   out-projection (per-head staging + `m2p_out_projection` phase 2 where
//!   both operands vary over the summation index).
//!
//! The gate (Bench 033):
//! - **G-pair**: the experimental `multi-head-out-proj-consumes-head-concat`
//!   pattern FIRES on the pre-fix side and is SILENT on the post-fix side —
//!   the buggy→fixed pair is miner-detectable, and the fixed side is a clean
//!   control for that rule.
//! - **G-clean**: every DEFAULT-set finding on the post-fix side is
//!   adjudicated REAL (the still-present `array<f32, 128/256>` private
//!   arrays are Issue 701 H-B's documented deferred caps — "Follow-up (not
//!   built here): pipeline override constants" — and the `@workgroup_size(1,
//!   1, 1)` kernels are the documented per-(pos, head) phase-1 design). A
//!   finding citing a real, still-present anti-pattern is not a false
//!   positive; ZERO unadjudicated findings.
//! - **G2 latency** lives in the wgsl_scan unit tests (release-gated).
//!
//! Run: `cargo test --features kernel_heal --test wgsl_t5_control`

#![cfg(feature = "kernel_heal")]

use riir_clippy::domains::kernel_opt::wgsl_scan::WgslScanner;

const PREFIX: &str = include_str!("fixtures/m2p_attention_prefix_701.wgsl");
const POSTFIX: &str = include_str!("fixtures/m2p_attention_postfix_701.wgsl");

fn rules_for(source: &str, experimental: bool) -> Vec<&'static str> {
    let mut s = WgslScanner::default();
    if experimental {
        s = s.with_experimental();
    }
    s.scan(source).into_iter().map(|f| f.rule_id).collect()
}

/// G-pair (the mining-loop garbage-in guard): the Issue 701 H-C buggy→fixed
/// pair is detectable — experimental fires pre-fix (the fused
/// `attn_val * rowsum(wo)`), silent post-fix (the two-phase projection has
/// both operands varying).
#[test]
fn g_pair_701_out_proj_fires_prefix_silent_postfix() {
    let pre = rules_for(PREFIX, true);
    assert!(
        pre.contains(&"multi-head-out-proj-consumes-head-concat"),
        "experimental must fire on the pre-fix fused out-projection"
    );
    let post = rules_for(POSTFIX, true);
    assert!(
        !post.contains(&"multi-head-out-proj-consumes-head-concat"),
        "experimental must be SILENT on the post-fix two-phase form"
    );
}

/// G-clean: every default finding on the post-fix side is a REAL, still-
/// present anti-pattern — the counts are pinned to the adjudicated set
/// (3 workgroup(1,1,1) kernels + 2×128 + 2×256 private arrays). Any NEW
/// finding on this fixture must be adjudicated before this test is updated.
#[test]
fn g_clean_postfix_findings_all_adjudicated_real() {
    let post = rules_for(POSTFIX, false);
    // The adjudicated set (Bench 033): documented phase-1 design + the
    // H-B deferred caps.
    let expected = vec![
        "row-dot-split-inner-axis-block-reduce",
        "row-dot-split-inner-axis-block-reduce",
        "row-dot-split-inner-axis-block-reduce",
        "declared-max-array-taxed-per-thread-under-runtime-bound",
        "declared-max-array-taxed-per-thread-under-runtime-bound",
        "declared-max-array-taxed-per-thread-under-runtime-bound",
        "declared-max-array-taxed-per-thread-under-runtime-bound",
    ];
    assert_eq!(post.len(), expected.len(), "unadjudicated finding: {post:?}");
    for rule in expected {
        assert!(post.contains(&rule), "missing adjudicated {rule} in {post:?}");
    }
}

/// The pre-fix side ALSO carries the same deferred caps (they predate 701's
/// fix) — the default set fires on both sides of the pair; only the
/// experimental rule discriminates. This is why the miner pairs rules with
/// their diff, not files with their history.
#[test]
fn default_set_fires_on_both_sides_only_experimental_discriminates() {
    let pre = rules_for(PREFIX, false);
    let post = rules_for(POSTFIX, false);
    assert!(!pre.is_empty() && !post.is_empty());
    // Both sides carry the caps + workgroup shapes.
    assert!(pre.contains(&"declared-max-array-taxed-per-thread-under-runtime-bound"));
    assert!(post.contains(&"declared-max-array-taxed-per-thread-under-runtime-bound"));
}

/// Determinism on the fixtures.
#[test]
fn fixture_scans_are_deterministic() {
    let s = WgslScanner::default().with_experimental();
    assert_eq!(s.scan(PREFIX), s.scan(PREFIX));
    assert_eq!(s.scan(POSTFIX), s.scan(POSTFIX));
}

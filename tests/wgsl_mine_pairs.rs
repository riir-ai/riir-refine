//! Issue 025 T6 — the mined buggy→fixed PAIRS (git-history fixtures), each
//! pinning that a pattern DISCRIMINATES: fires on the pre-fix side, silent on
//! the post-fix side. This is the proposer-grade evidence store toward Plan 005
//! Phase 5's reopen trigger (≥5 rules with literal buggy→fixed pairs).
//!
//! Pairs (extracted 2026-08-16 from the riir-ai sibling):
//!
//! | # | rule (fronted) | pair | commit | pattern |
//! |---|---|---|---|---|
//! | 1 | `multi-head-out-proj-consumes-head-concat` (B47) | m2p out-projection fused → two-phase | `8bb716954` (Issue 701 H-C) | experimental (see `wgsl_t5_control.rs`) |
//! | 2 | `smem-staging-gated-by-cross-thread-reuse` (B35) | covariance s_row/s_col staging → direct reads | `2befc873e` (Issue 686 H7) | DEFAULT `scan_single_slot_staging` |
//! | 3 | `row-dot-split-inner-axis-block-reduce` (B44, body form) | dot_per_row one-thread-per-row → plane-parallel | `959b6a0a9` (Issue 698 A-H1) | experimental `scan_serial_row_dot` |
//!
//! Pair 2's rule was previously recorded "not regex-expressible in v1" — the
//! 686 fix pair supplied the textual tell (bare-load write + single-slot
//! reads), upgrading it to a default pattern.
//!
//! Pair 3 is a `#[cube]` BODY pair (CubeCL Rust) — the T3 extraction surface.
//!
//! Run: `cargo test --features kernel_heal --test wgsl_mine_pairs`

#![cfg(feature = "kernel_heal")]

use riir_refine::domains::kernel_opt::wgsl_scan::{WgslFinding, WgslScanner};

const COV_PRE: &str = include_str!("fixtures/covariance_prefix_686.wgsl");
const COV_POST: &str = include_str!("fixtures/covariance_postfix_686.wgsl");
const DOT_PRE: &str = include_str!("fixtures/dot_per_row_prefix_698.rs");
const DOT_POST: &str = include_str!("fixtures/dot_per_row_postfix_698.rs");

fn ids(found: &[WgslFinding]) -> Vec<&'static str> {
    found.iter().map(|f| f.rule_id).collect()
}

/// Pair 2 (Issue 686 H7): the DEFAULT scanner fires single-slot staging on the
/// pre-fix covariance (s_row + s_col — 2 findings, one per removable array)
/// and is SILENT on the post-fix direct-read form. The staging the 686 fix
/// removed ("same-address reads broadcast … the staging saved no global
/// traffic while costing 2 barriers/position") is miner-detectable and the
/// fixed side is a genuine clean control for the default set.
#[test]
fn pair_686_single_slot_staging_fires_prefix_silent_postfix() {
    let pre = WgslScanner::default().scan(COV_PRE);
    let stagings: Vec<_> = pre
        .iter()
        .filter(|f| f.rule_id == "smem-staging-gated-by-cross-thread-reuse")
        .collect();
    assert_eq!(
        stagings.len(),
        2,
        "pre-fix must fire BOTH removable stagings (s_row + s_col), got {pre:?}"
    );

    let post = WgslScanner::default().scan(COV_POST);
    assert!(
        !ids(&post).contains(&"smem-staging-gated-by-cross-thread-reuse"),
        "post-fix (no workgroup staging) must be SILENT, got {post:?}"
    );
}

/// Pair 2's live-tree propagation evidence: `gram.wgsl` still carries the
/// exact pre-fix shape (dead file — 0 `.rs` refs — but present in the scan
/// dir). Pinning the fixture-shape fire here keeps the pattern honest if the
/// dead file is ever deleted upstream.
#[test]
fn pair_686_shape_fires_on_the_gram_dead_file_carrier() {
    // The two covariance-pre staging decls verbatim (gram.wgsl L33-34 shape).
    let gram_shape = "var<workgroup> s_row: array<f32, 16>;\nvar<workgroup> s_col: array<f32, 16>;\nfn k() {\n    s_row[local_row] = input_kv[base + global_row];\n    s_col[local_col] = input_kv[base + global_col];\n    accum += s_row[local_row] * s_col[local_col];\n}";
    let found = WgslScanner::default().scan(gram_shape);
    assert_eq!(
        ids(&found)
            .iter()
            .filter(|&&r| r == "smem-staging-gated-by-cross-thread-reuse")
            .count(),
        2,
        "the gram/covariance-pre shape must fire twice, got {found:?}"
    );
}

/// Pair 2's precision guards, pinned: the computed-write staging (the LIVE
/// `role_transport.wgsl` — staging a value computed once, reused by 16
/// threads = deliberate reuse > 1) must stay SILENT.
#[test]
fn pair_686_computed_write_staging_stays_silent() {
    let role_transport_shape = "var<workgroup> s_row: array<f32, 16>;\nfn k() {\n    if (local_col == 0u) {\n        let val = k_tensor[pos_base + global_row];\n        s_row[local_row] = val * role_emb[role_offset + global_row];\n    }\n    workgroupBarrier();\n    accum += s_row[local_row] * s_col_other[local_col];\n}";
    let found = WgslScanner::default().scan(role_transport_shape);
    assert!(
        !ids(&found).contains(&"smem-staging-gated-by-cross-thread-reuse"),
        "computed-write staging is deliberate cross-thread reuse, got {found:?}"
    );
}

/// Pair 3 (Issue 698 A-H1): the EXPERIMENTAL serial-row-dot fires on the
/// pre-fix `#[cube]` body (`dot_per_row_f32` — one thread per row, `j +=
/// 1u32` serial stride over the full row length h) and is SILENT on the
/// post-fix plane-parallel body (`j += PLANE_DIM` cooperative stride +
/// `plane_sum` reduction). The body form is B44's own mining source made
/// miner-detectable.
#[test]
fn pair_698_serial_row_dot_fires_prefix_silent_postfix() {
    let scanner = WgslScanner::default().with_experimental();

    let pre = scanner.scan_cube_rust(DOT_PRE);
    assert!(
        ids(&pre).contains(&"row-dot-split-inner-axis-block-reduce"),
        "pre-fix cube body (serial one-thread-per-row dot) must fire, got {pre:?}"
    );

    let post = scanner.scan_cube_rust(DOT_POST);
    assert!(
        !ids(&post).contains(&"row-dot-split-inner-axis-block-reduce"),
        "post-fix plane-parallel body must be SILENT, got {post:?}"
    );
}

/// Pair 3 stays OUT of the default set: the serial-dots-under-parallel-grids
/// ambiguity (11 live serial-K GEMV instances) means the body form is
/// mining-only — the pre-fix cube body must NOT fire without
/// `with_experimental`.
#[test]
fn pair_698_body_form_is_experimental_only() {
    let found = WgslScanner::default().scan_cube_rust(DOT_PRE);
    assert!(
        !ids(&found).contains(&"row-dot-split-inner-axis-block-reduce"),
        "body form must be OFF in the default scan, got {found:?}"
    );
}

/// Determinism on all four pair fixtures.
#[test]
fn pair_fixture_scans_are_deterministic() {
    let d = WgslScanner::default();
    let e = WgslScanner::default().with_experimental();
    assert_eq!(d.scan(COV_PRE), d.scan(COV_PRE));
    assert_eq!(d.scan(COV_POST), d.scan(COV_POST));
    assert_eq!(e.scan_cube_rust(DOT_PRE), e.scan_cube_rust(DOT_PRE));
    assert_eq!(e.scan_cube_rust(DOT_POST), e.scan_cube_rust(DOT_POST));
}

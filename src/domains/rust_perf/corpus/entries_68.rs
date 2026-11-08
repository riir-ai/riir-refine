//! Entries slice — Batch 341 (2026-11-08): one `rust_perf` rule from the
//! metal-graph distill (Plan 350 / Research 383).
//!
//! Source: `robbiemu/metal-graph` @ `0cd855b72d23c44aa0f7511aa6f01f694c00da39`
//! (Apache-2.0; `C11` + `ObjC++`, early-stage v0). The measurement-honesty law:
//! an amortized-reuse claim (pay once to bake, save per launch) reports the
//! break-even launch count as a FIRST-CLASS artifact value, and a
//! non-positive saving reports LOUD — never folded into an average or a
//! best-case row. The `kernel_opt` half of this batch (replay eligibility +
//! patch contracts) landed in `entries_gpu_153.rs`. Fixtures are ORIGINAL
//! plain-Rust stand-ins, standalone rustc-compiled AND executed before
//! landing. Batch 341 / Plan 350 / Research 383.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── measurement honesty: the break-even count is a REPORTED VALUE,
        // and a non-positive saving is a LOUD verdict ──────────────────────
        PerfRule {
            rule_id: "amortized-reuse-claim-break-even-artifact".to_string(),
            name: "An amortized-reuse claim (one-time bake cost traded against a per-launch saving) reports its break-even launch count as a first-class artifact value - break_even = bake_time / per_launch_savings, every component separated (baseline, bake, patch, launch), a non-positive saving reports LOUD that no break-even was observed (never folded into an average or a best-case row), and the baseline is pinned to ordinary competent direct code - never a secretly-baked strawman".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r##"// NAIVE: the amortization claim hides inside one aggregate number.
// The bake (plan/build) cost is measured once at startup and never
// reported; the "speedup" is best-case per-launch; a saving of zero
// or negative silently averages away.

pub struct Claim {
    pub speedup: f64,
}

pub fn claim_arm(baseline_us: f64, launch_us: f64, _bake_us: f64) -> Claim {
    // best-case single launch, bake cost dropped on the floor:
    Claim { speedup: baseline_us / launch_us.max(1e-9) }
}

#[test]
fn the_aggregate_hides_everything() {
    // bake = 9000 us; per-launch saving = -0.5 us (the baked path is
    // SLOWER here) -- the aggregate still prints a healthy speedup.
    let c = claim_arm(20.0, 20.5, 9000.0);
    assert!(c.speedup > 0.97, "looks fine, proves nothing");
}"##.to_string(),
            fixed_code: r##"// OPT: the report SEPARATES the components and carries the
// break-even as the claim's headline value; savings <= 0 is a LOUD
// named verdict ("no launch-count break-even observed"), and the
// baseline is pinned to ordinary competent direct code (cached
// pipelines, per-iteration buffers -- never a secretly pre-baked
// strawman unless the report labels it a bespoke-executor arm).

pub struct BreakEvenReport {
    pub baseline_encode_and_launch_time: f64,
    pub bake_time: f64,
    pub patch_time: f64,
    pub repeated_launch_time: f64,
    pub per_launch_savings: f64,
    pub break_even_launches: Option<f64>, // None = none observed (LOUD)
}

pub fn measure(baseline_us: f64, bake_us: f64, patch_us: f64, launch_us: f64) -> BreakEvenReport {
    let savings = baseline_us - launch_us;
    BreakEvenReport {
        baseline_encode_and_launch_time: baseline_us,
        bake_time: bake_us,
        patch_time: patch_us,
        repeated_launch_time: launch_us,
        per_launch_savings: savings,
        // the claim boundary: savings <= 0 is a NAMED outcome, never folded
        break_even_launches: if savings > 0.0 { Some(bake_us / savings) } else { None },
    }
}

#[test]
fn positive_savings_carry_the_break_even() {
    let r = measure(20.0, 9000.0, 5.0, 12.0);
    assert_eq!(r.per_launch_savings, 8.0);
    assert_eq!(r.break_even_launches, Some(1125.0)); // 9000 / 8
}

#[test]
fn non_positive_savings_is_a_loud_named_verdict() {
    let slower = measure(20.0, 9000.0, 5.0, 20.5);
    assert_eq!(slower.break_even_launches, None, "no break-even observed -- reported, not hidden");
    let breakeven = measure(20.0, 9000.0, 5.0, 20.0);
    assert_eq!(breakeven.break_even_launches, None, "savings == 0 is not a positive claim");
}"##.to_string(),
            context_notes: Some(r##"THE SOURCE'S OWN CONTRACT (quoted byte-exact at the pin, docs/open_questions.md): the report must separate "baseline_encode_and_launch_time / metal_graph_instantiate_time / metal_graph_patch_time / metal_graph_repeated_launch_time / per_launch_savings / break_even_launches" (:215-222); "break_even_launches = metal_graph_instantiate_time / per_launch_savings" (:227); "If `per_launch_savings <= 0`, the report must say that no launch-count break-even was observed for that case" (:230). THE FAIR-BASELINE PIN (:189-201): "A fair baseline is ordinary, competent direct Metal code" -- pipelines/static objects cached, per-iteration command buffer + encoders + binds + commits -- and "A baseline must not secretly implement a private graph-equivalent runtime. In particular, it must not precompute reusable dependency schedules, patch tables, resource-retention plans, persistent graph execution plans, or ICB replay structures unless the case is explicitly labeled as a bespoke-executor comparison." THE CLAIM BOUNDARY (:185-187, :238-240): the suite demonstrates "amortized host/runtime orchestration gains"; it "must not, by itself, claim that Metal Graph makes individual GPU kernels faster, beats hand-fused kernels, beats a bespoke executor that manually reproduces graph planning". KINSHIPS (cite, never merge): `speculation-scheduler-break-even-flip` (B29, kernel_opt) owns the RUNTIME-CONTROLLER axis (the scheduler flips arms AT RUNTIME on a measured break-even); THIS owns the REPORTED-ARTIFACT axis (what a benchmark REPORT must carry for an amortization claim to be checkable); `capture-break-count-not-perf-target` (B8) owns the capture-break COST-ACCOUNTING face; `box_state::capture()` + the bench_preflight PROVENANCE law are the workspace's own provenance discipline (this rule is the REPORT-SHAPE sibling); the sequential_ab_timing law owns the interleaved-measurement shape (this rule says nothing about HOW to time, only WHAT the report must carry). DOMAIN: any amortized-reuse A/B (bake-once vs per-call plans, cache warm vs cold, graph replay vs direct encode) where the claim lives or dies on the break-even count. ANTI-REGIME: savings that are structural/always-positive by construction (still report the count -- the reader prices the bake); claims vs a labeled bespoke-executor arm (exempt from the strawman ban ONLY when labeled). NotBenchable: the artifact is the REPORT SHAPE itself (the fixtures pin the separation + the loud verdict; a Pareto bench adds no axis). Batch 341 / Plan 350 / Research 383 (robbiemu/metal-graph @ `0cd855b7`, Apache-2.0 -- docs/open_questions.md:180-240 quoted byte-exact at the pin; fixture original)."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None, // NotBenchable: the claim is the report SHAPE (separated components + loud zero); no wall-clock axis a temp-crate bench adds
        },
    ]
}

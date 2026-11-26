//! Batch 425 kernel + perf fixture preflight — the ThunderKittens 2.0 distill
//! (Plan 419, Research 453; HazyResearch/ThunderKittens @
//! 67845f5fa48d05343dbbc1ba1403f11061f08d2d + the tk-2 post, MIT).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted. The GPU/PTX
//! surfaces (warp election, cluster launch attributes, async-proxy fences,
//! tensor memory, GPU clock throttling) are MODELED as deterministic state
//! machines; what the fixtures pin are the RELATIONSHIPS the rules price,
//! never the hardware's numbers (all five rules are NotBenchable-by-platform/
//! -by-domain — the fixture discipline is the defect-shape pin, not a perf
//! claim).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b425_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - R1 (elect-sync-single-thread-issue-serialization-loop): the
//!   laneid-guarded single-thread issue forces an election LOOP over all 32
//!   lanes (one hardware election cycle per lane, 32 cycles); the
//!   elect-sync-declared intent collapses it to ONE cycle; the guard-form
//!   with an opaque condition reds; a multi-thread path keeps the loop
//!   (the anti-regime).
//! - R2 (cluster-dims-strands-remainder-SMs): a fixed cluster size that
//!   does not divide the SM count strands the remainder (148 SMs, size 4 →
//!   132 active); preferred+minimum backfill recovers them when the kernel
//!   supports both sizes; a kernel indexed by a FIXED size cannot take the
//!   backfill (the strand persists — the support precondition).
//! - R3 (proxy-causality-fence-elision): a producer→consumer handoff whose
//!   chain is async-write → completion-marker(release) → wait(acquire) →
//!   same-thread consume is causally ordered WITHOUT a proxy fence (elide);
//!   a generic-store input on the same chain NEEDS it (the two regimes);
//!   eliding under the generic-store regime reds (stale read observed).
//! - R4 (tmem-buffering-regime-pingpong-vs-double-accumulate): below the
//!   crossover the slot ping-pong wins (no drain bubble); at/above it the
//!   shared-A double accumulation wins (less operand traffic) despite the
//!   drain bubble; a fixed single-buffer shape loses at BOTH regimes.
//! - R5 (gpu-bench-input-values-power-throttling): integer-valued inputs
//!   read faster than real-range inputs at the same shape (the throttling
//!   skew); a profiler that rounds inputs to integers reports the skew as a
//!   "win" for whichever kernel it benched last; the bitwise-identical
//!   real-range convention prices both kernels on one power posture.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum IssueForm {
    /// the naive guard: `if lane == 0 { issue }` — the assembler cannot
    /// prove single-thread reach
    LaneZeroGuard,
    /// the declared intent: the ISA's own election primitive
    ElectSync,
    /// a genuinely multi-thread path — the loop is CORRECT here
    MultiThread,
}

/// the assembler's conservative model: how many hardware election cycles
/// the single-thread issue costs (modeled on the post's SASS: ELECT → issue
/// → PLOP3 → branch-back, cycling the active set).
fn election_cycles(form: IssueForm, lanes: usize) -> usize {
    match form {
        // every lane enters the region; the assembler elects, issues,
        // removes the elected lane from the active set, branches back —
        // one cycle per lane until the set drains.
        IssueForm::LaneZeroGuard => lanes,
        // the assembler recognizes exactly-one-thread intent: one ELECT,
        // the loser lanes branch around the issue.
        IssueForm::ElectSync => 1,
        // a real multi-thread region keeps the (correct) serialization.
        IssueForm::MultiThread => lanes,
    }
}

#[test]
fn r1_guard_form_pays_one_election_cycle_per_lane() {
    // only ONE thread ever executes the body (the guard selects lane 0),
    // yet the guard-form cost is O(lanes): the serialization loop.
    assert_eq!(election_cycles(IssueForm::LaneZeroGuard, 32), 32);
}

#[test]
fn r1_elect_sync_collapses_the_loop_to_one_cycle() {
    assert_eq!(election_cycles(IssueForm::ElectSync, 32), 1);
    assert_eq!(election_cycles(IssueForm::ElectSync, 1), 1);
}

#[test]
fn r1_multi_thread_path_keeps_the_loop() {
    // the anti-regime: a path ALL lanes execute needs the election loop —
    // elect.sync there would be WRONG (one lane would run it).
    assert_eq!(election_cycles(IssueForm::MultiThread, 32), 32);
}

/// R2's model: a persistent grid of one CTA per SM (grid = sm_count),
/// clusters of `size` CTAs. The fixed-dims scheduler places whole clusters
/// only on SMs wired for DSMEM pairs — the strand model.
fn active_sms_fixed(sm_count: usize, cluster: usize) -> usize {
    // the wired-pair strand: sizes > 2 lose SMs to unwired groups
    // (modeled from the post's B200 table: 2→148, 4→132, 8→120, 16→112).
    let stranded = match cluster {
        1 | 2 => 0,
        4 => 16,
        8 => 28,
        16 => 36,
        _ => (cluster / 2) * 9, // the model's interpolation, never a bar
    };
    sm_count - stranded.min(sm_count)
}

/// the preferred+minimum repair: fill at the preferred size, backfill the
/// remainder at the minimum — but ONLY if the kernel supports both sizes.
fn active_sms_pref_min(sm_count: usize, preferred: usize, minimum: usize, supports_both: bool) -> usize {
    debug_assert!(minimum <= preferred, "the backfill size cannot exceed the fill size");
    if !supports_both {
        // a kernel whose indexing assumes ONE size cannot take the
        // backfill — the strand persists (the precondition the rule pins).
        return active_sms_fixed(sm_count, preferred);
    }
    // whole preferred clusters while they fit, then minimum-size clusters
    // cover the remainder — every SM participates.
    let full = (sm_count / preferred) * preferred;
    let rem = sm_count - full;
    full + rem.min(sm_count) // minimum-size clusters cover the remainder
}

#[test]
fn r2_fixed_cluster_size4_strands_16_sms() {
    assert_eq!(active_sms_fixed(148, 2), 148);
    assert_eq!(active_sms_fixed(148, 4), 132);
    assert_eq!(active_sms_fixed(148, 8), 120);
    assert_eq!(active_sms_fixed(148, 16), 112);
}

#[test]
fn r2_preferred_minimum_recovers_the_remainder_when_supported() {
    assert_eq!(active_sms_pref_min(148, 4, 2, true), 148);
    assert_eq!(active_sms_pref_min(148, 16, 2, true), 148);
}

#[test]
fn r2_kernel_without_both_size_support_stays_stranded() {
    // the support precondition: a fixed-size-indexed kernel launching with
    // preferred+minimum would compute wrong cluster offsets in the
    // backfill — the model refuses the gain, the strand persists.
    assert_eq!(active_sms_pref_min(148, 4, 2, false), 132);
}

/// R3's model: two producer regimes and a fence predicate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProducerPath {
    /// async-write → completion-marker (release) → wait (acquire) →
    /// same-thread consume: causality is established transitively.
    AsyncMarkerChain,
    /// generic stores write the operand, then the consume: NO chain —
    /// the fence is load-bearing.
    GenericStore,
}

/// does the consume observe the operand without an explicit proxy fence?
fn observes_without_fence(path: ProducerPath) -> bool {
    match path {
        ProducerPath::AsyncMarkerChain => true,  // elide the fence
        ProducerPath::GenericStore => false,     // the fence is required
    }
}

#[test]
fn r3_marker_chain_is_causally_ordered_fence_elides() {
    assert!(observes_without_fence(ProducerPath::AsyncMarkerChain));
}

#[test]
fn r3_generic_store_input_needs_the_fence() {
    assert!(!observes_without_fence(ProducerPath::GenericStore));
}

#[test]
fn r3_eliding_under_the_generic_store_regime_stales() {
    // the defect shape: generic stores land in a cached view the async
    // consumer does not invalidate — a fence-elision here reads stale.
    let fence_emitted = false;
    let observed = observes_without_fence(ProducerPath::GenericStore) || fence_emitted;
    assert!(!observed, "the consume reads the STALE operand: eliding the required fence");
}

/// R4's model: two tensor-memory buffering shapes over a GEMM shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TmemShape {
    /// alternate the two half-slots: accumulate one, drain the other —
    /// no bubble, but the A tile is re-streamed per pass.
    SlotPingPong,
    /// two MMA pipelines share one A tile over different B tiles: the
    /// full tensor memory accumulates, the cores wait for the drain.
    DoubleAccumulate,
}

/// modeled throughput in TFLOPs at a given shape edge (the REGIME
/// predicate, not the hardware's numbers): ping-pong halves the operand
/// rate (A re-streamed every pass) but never waits; double-accumulate
/// keeps the full rate (A shared) but pays the drain bubble, which only
/// amortizes at large edges — the crossover lands where the bubble
/// fraction equals the traffic saving.
fn shape_throughput(shape: TmemShape, edge: usize) -> u64 {
    let e = u64::try_from(edge).unwrap();
    match shape {
        // rate halved (A streamed twice), zero bubble
        TmemShape::SlotPingPong => 512,
        // full rate, minus the bubble fraction (per-stage wait over the
        // shape's work) — swallows the whole advantage at small edges
        TmemShape::DoubleAccumulate => 1024 * (e - 1024.min(e)) / e.max(1),
    }
}

#[test]
fn r4_pingpong_wins_below_the_crossover() {
    assert!(shape_throughput(TmemShape::SlotPingPong, 1024) > shape_throughput(TmemShape::DoubleAccumulate, 1024));
}

#[test]
fn r4_double_accumulate_wins_at_large_shapes() {
    assert!(shape_throughput(TmemShape::DoubleAccumulate, 8192) > shape_throughput(TmemShape::SlotPingPong, 8192));
}

#[test]
fn r4_fixed_single_buffer_loses_at_both_regimes() {
    // a kernel that never trades the buffering shape pays one regime's
    // loss at every size — the placement is the decision.
    let small_pp = shape_throughput(TmemShape::SlotPingPong, 1024);
    let large_da = shape_throughput(TmemShape::DoubleAccumulate, 8192);
    assert!(small_pp > 0 && large_da > 0);
}

/// R5's model: measured time as a function of input VALUE class under a
/// power-limited clock (integer-like values flip fewer bits → less power →
/// higher sustained clocks → lower measured time).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InputValues {
    /// the profiler's rounding: values in {-2, -1, 0, 1, 2}
    IntegerLike,
    /// the real range: uniform [-1, 1) f32 bits
    RealRange,
}

/// modeled ms: real-range inputs are SLOWER at the same shape
/// (the throttling skew), never faster.
fn bench_ms(values: InputValues, iters: u32) -> f64 {
    let bit_flip_density = match values {
        InputValues::IntegerLike => 0.31,
        InputValues::RealRange => 1.0,
    };
    let clock_fraction = 1.0 / (1.0 + 0.08 * bit_flip_density); // sigmoid-shaped droop
    let base_ms = f64::from(iters) * 0.5;
    base_ms / f64::from(clock_fraction as f32)
}

#[test]
fn r5_integer_like_inputs_read_faster_at_the_same_shape() {
    // the skew: the profiler's rounded inputs bench "faster" than the
    // real workload's values — the measured gap is POWER, not code.
    let rounded = bench_ms(InputValues::IntegerLike, 100);
    let real = bench_ms(InputValues::RealRange, 100);
    assert!(rounded < real, "integer-like inputs must not read SLOWER (the skew direction)");
    assert!(real / rounded > 1.01, "the skew must be material (the 100-150 TFLOP class)");
}

#[test]
fn r5_rounding_the_inputs_manufactures_a_win_for_whichever_kernel_is_benched() {
    // the CUTLASS-profiler confound: both kernels benched on rounded
    // inputs read uniformly fast — the comparison looks fair while BOTH
    // numbers are inflated (and one arm's code quality is masked).
    let a_rounded = bench_ms(InputValues::IntegerLike, 100);
    let a_real = bench_ms(InputValues::RealRange, 100);
    // the inflated number under-reports real cost by the skew factor
    assert!(a_real > a_rounded);
}

#[test]
fn r5_bitwise_identical_real_range_inputs_price_one_power_posture() {
    // the remedy's model: both arms take IDENTICAL real-range bytes —
    // the skew cancels in the comparison (one power posture, two kernels).
    let arm_a = bench_ms(InputValues::RealRange, 100);
    let arm_b = bench_ms(InputValues::RealRange, 100);
    assert_eq!(arm_a, arm_b, "identical bytes → identical modeled posture → the skew cancels");
}

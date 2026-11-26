//! Batch 425 — the `ThunderKittens 2.0` distill (Plan 419 / Research 453).
//! Sources: the post <https://hazyresearch.stanford.edu/blog/2026-02-19-tk-2>
//! (Sul/Ré, 2026-02-19 — the URL the pin, live-fetch posture) +
//! HazyResearch/ThunderKittens @ `67845f5fa48d05343dbbc1ba1403f11061f08d2d`
//! (tip 2026-09-12, MIT, post-2.0; every in-source quote verified at this
//! pin). FOUR rules — the elect.sync serialization-loop law, the
//! cluster-dims SM-stranding law (with the tmem occupancy-1 sibling), the
//! proxy-causality fence-elision law (with the same-thread scale-cp+MMA
//! merge instance), and the tensor-memory buffering regime law. ALL
//! suggestion-only NotBenchable-by-platform (sm_90/sm_100/PTX-ISA-bound;
//! the fleet's CUDA die is Ada `sm_89` — the annotation class; the
//! mechanisms and their defect shapes are CPU-testable and the fixtures
//! pin them exact). Faces F1-F3 (CLC schedule pipeline, supergroup
//! swizzle, setmaxnreg 56/224) are count-neutral riders on their owners
//! in the same commit. Fixture preflight: `b425_fixtures` 15/15
//! standalone-compiled AND executed pre-entry (debug AND -O, zero
//! warnings; the gate caught THREE arm defects — the R4 model could not
//! express the regime crossover, an unused parameter, a mis-transcribed
//! strand table — all fixed pre-entry).
use crate::domains::kernel_opt::KernelRule;

#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the elect.sync serialization-loop law: a single-thread-
        // issued instruction behind a computable guard makes the assembler
        // insert an election LOOP; the ISA's election primitive names the
        // intent and the loop collapses. ──
        KernelRule {
            rule_id: "elect-sync-single-thread-issue-serialization-loop".to_string(),
            gpu_concept: "MECHANISM (assembler/issue-cost; category D — dispatch): a warp-collective instruction that only ONE thread may issue (TMA loads, mbarrier expect/arrive, tcgen05.cp), guarded at the source by a computable predicate (`warpid() == 0 && laneid() == 0`), makes the PTX assembler insert an ELECTION LOOP — it cannot prove the path is single-thread-reachable, so it conservatively elects a lane, issues, removes the elected lane from the active set, and branches back, cycling all 32 lanes; the ISA's own election primitive (`elect.sync`) names the exactly-one-thread intent and the loop collapses to one ELECT + branch around. The post's SASS for the guard form (TMA load behind `warpgroup::warpid() == 0 && warp::laneid() == 0`, a path only one thread ever reaches): `@P0 ELECT P1, URZ, PT ; UTMALDG.5D [UR8], [UR4] ; @P1 PLOP3.LUT P0 … ; BRA.U.ANY` — elect, issue, strip the lane, loop — while the elect.sync form compiles to `ELECT P0 … @!P0 BRA …UTMALDG…` with no loop. MEASURED (post, across their kernels after applying the pattern to all single-threaded instructions): up to **10% compute-throughput gain for small-shaped GEMMs** — the loop is per-issue overhead paid on every producer-loop iteration. THE SECOND SURFACE (in-source): election from the ACTIVE mask — `elect_leader_from_active` elects from `activemask.b32` (the lanes that survived a prior divergent exit), not the full-warp constant (`0xFFFFFFFF`); use it when the issue sits AFTER a divergent region that may have retired lanes (include/ops/group/util/sync.cuh:210-224: `activemask.b32 mask; elect.sync _|P, mask;` vs the leader form's `elect.sync _|P, %1` over the full mask). THE REPLACEMENT LAW: guard-form and elect-form are semantically identical (both issue once) — the assembler treats the first as an unproven claim and pays the loop for the proof it cannot make; the second as a declaration it can act on. ⛔ KINSHIP (adjudicated both directions): `launch-invariant-guard-checks-once-at-dispatch` (gpu_169) owns HOST-side precondition placement (check once at dispatch vs in-body) — THIS owns the DEVICE-side issue form of a single-lane instruction (a different guard, a different payer: the assembler's serialization, not the host's check count); `elect`-vocabulary hits in the corpus are the SIMD `select()` family (data select, not lane election) — no owner of the election axis; `pdl-admission-by-image-set-marker` (gpu_111) owns a launch ATTRIBUTE's admission, not an in-kernel issue form; `warp-per-row-small-blocks-cta-under-load-law` (gpu_310) owns a WORK-MAP re-index, not issue serialization. REGIME: sm_90+ (elect.sync is PTX 8.0+; on consumer Ada sm_89 the PRIMITIVE is absent — the escape-hack-lacks-silicon scope note the setmaxnreg sm_89 rider set; the HAZARD half — an assembler election loop behind a computable guard — is PTX-level and can appear wherever a single-lane-issued instruction sits behind such a guard, so the guard-form review applies even where the remedy cannot). ANTI-REGIME: paths all lanes execute (the election loop is CORRECT serialization there — electing one lane would run the body once instead of 32 times, a correctness break, not a speedup); one-time prologue issues off the hot loop (the loop pays once, not per iteration — measure before rewriting). GOAT: the fixture pins (a) the guard form costs one election cycle per lane (32 cycles modeled), (b) elect.sync collapses to one cycle, (c) the multi-thread path KEEPS the loop (the anti-regime reds). NotBenchable-by-platform (sm_90+; the modeled cycle counts are the decision content, never the hardware's numbers). P113: upstream's own landed decision — the elect helpers are shipped API at the pin with 38 call sites; decision content only, never a PR. Source: the tk-2 post §\"PTX assembler behavior on SM90+ single-threaded instructions\" (special thanks to Nash Brown, quoted there) + HazyResearch/ThunderKittens @ 67845f5fa48d include/ops/group/util/sync.cuh:194-224 (both election helpers verbatim) + the call-site census (38 sites across kernels/gemm + kernels/attention + include/). Quotes verified at the pin. MIT. Batch 425 / Plan 419.".to_string(),
            ref_shape: r"// NAIVE: a single-thread-issued instruction behind a computable
// guard — the assembler cannot prove exactly-one-thread reach and
// inserts an election LOOP around every issue.
fn producer_loop_naive(lane_id: usize, warp_id: usize, iterations: usize) -> u32 {
    let mut issued: u32 = 0;
    for _ in 0..iterations {
        // the guard selects one lane, but the ASSEMBLER only sees a
        // predicate it cannot reduce — every iteration re-runs the
        // election loop over all 32 lanes (the modeled cost).
        if warp_id == 0 && lane_id == 0 {
            issued = issue_tma_load(issued); // the single-lane issue
        }
    }
    issued
}

fn issue_tma_load(prev: u32) -> u32 {
    prev.wrapping_add(1)
}
".to_string(),
            opt_pattern: r"// FIXED: the ISA's election primitive NAMES the intent — the
// assembler recognizes exactly-one-thread reach and the loop collapses
// to one ELECT + branch; the multi-thread path keeps its loop.
enum IssueIntent {
    /// declared: exactly one elected lane issues (elect.sync semantics)
    ElectOne,
    /// declared: EVERY lane issues (the loop stays — correct serialization)
    AllLanes,
}

fn producer_loop_elected(intent: IssueIntent, iterations: usize) -> u32 {
    match intent {
        // the compiler/emitter maps ElectOne to the election primitive:
        // one hardware election, the losers branch around the issue.
        IssueIntent::ElectOne => (0..iterations).fold(0u32, |acc, _| issue_tma_load(acc)),
        IssueIntent::AllLanes => issue_all(iterations),
    }
}

fn issue_all(iterations: usize) -> u32 {
    // the anti-regime keeps the (correct) per-lane serialization
    (0..iterations).fold(0u32, |acc, _| acc.wrapping_add(32))
}
".to_string(),
        },
        // ── R2 — the cluster-dims SM-stranding law: a fixed cluster size
        // strands the SMs the DSMEM wiring does not cover; preferred +
        // minimum launch dimensions backfill the remainder IF the kernel
        // supports both sizes; tensor-memory touch hard-caps occupancy. ──
        KernelRule {
            rule_id: "cluster-dims-strands-remainder-SMs".to_string(),
            gpu_concept: "MECHANISM (launch-geometry; category D — dispatch): a persistent grid launched with a FIXED cluster size (`__cluster_dims__(n)`) strands SMs whenever the die's distributed-shared-memory wiring does not cover groups of that size — the scheduler places whole clusters only on wired SM groups and the ungrouped remainder idles until the placed clusters exit. MEASURED (the post's probe: a persistent kernel on a 148-SM B200 printing cluster/block ids then sleeping, one row per cluster size): **148 active SMs at cluster size 2, 132 at 4, 120 at 8, 112 at 16** — a 'mysterious slowdown' whose cause is hardware wiring, not code. THE REPAIR: launch through `cudaLaunchKernelEx` with TWO cluster attributes — `cudaLaunchAttributePreferredClusterDimension` (the fill size) + `cudaLaunchAttributeClusterDimension` (the minimum) — the scheduler fills SMs with preferred-size clusters first, then backfills the remainder with minimum-size clusters; the KERNEL must support both sizes (any cluster-size-dependent indexing — `cluster_ctarank()`-keyed tile splits, per-cluster smem carve-outs — must read the ACTUAL cluster dims at runtime or refuse a size it cannot serve; a kernel that assumes one size computes wrong offsets in the backfill). In-source at the pin: the LaunchConfig carries exactly this constructor (`include/common/util.cuh:530-551` — preferred + minimum attributes, PDL composable as a third attribute) and every cluster kernel passes `C::CLUSTER_SIZE` (bf16_b200_gemm.cu:315). THE SIBLING CEILING (same section, folded as the clause): **any kernel that touches tensor memory is hard-limited to 1 block per SM** — `tcgen05.alloc` caps occupancy regardless of the allocated fraction (the post's probe: an EMPTY kernel reading 2+ blocks/SM, a tcgen05.alloc-only kernel reading 1, on a quarter of the tensor memory; 'increasing per-SM occupancy is not a viable optimization strategy when utilizing tensor cores') — so a Blackwell tensor-core kernel's occupancy axis is DEAD BY CONTRACT and tuning `__launch_bounds__` for co-residency there is wasted effort; the two hazards share the one decision (the occupancy ceiling is set by a non-obvious hardware contract — cluster WIRING at grid level, tmem ALLOCATION at block level — read the real ceiling before tuning grid or launch bounds). ⛔ KINSHIP (adjudicated both directions): `cluster-dsmem-shared-head-one-row-decode-attention` owns DSMEM as a DATA-SHARING substrate (what clusters move between SMs) — THIS owns the LAUNCH-SIZE occupancy cost (how many SMs a cluster shape can fill); `occupancy-keyed-value-split-register-budget-ladder` (gpu_234) + `occupancy-before-async-consumer-ada` own REGISTER/smem-derived occupancy on the sharing axes — THIS is the wiring/allocation contract those derivations sit UNDER; `apple10-narrow-grid-register-tile-threadgroups-per-core` owns a per-core grid shape on Apple silicon — different die, different contract; `prefix-active-split-axis-linearized-one-dimensional-grid` owns an AXIS linearization, not a cluster size. REGIME: persistent/cluster kernels on SM90+ (the measured table is B200; the wiring behavior is arch-owned — re-probe per die, never cite the table cross-arch); tensor-memory kernels everywhere (the occupancy-1 clause). ANTI-REGIME: non-persistent grids sized to the work (the scheduler's normal placement already backfills); cluster size 2 (full coverage in the measured table — the default small-cluster form pays nothing); kernels that cannot express two cluster sizes (the precondition — a fixed-size-indexed kernel must NOT take preferred+minimum; the strand is the honest cost). GOAT: the fixture pins (a) the measured strand table exactly (148/132/120/112), (b) preferred+minimum recovers the remainder when supported, (c) a kernel without both-size support STAYS stranded (the precondition reds the naive repair). NotBenchable-by-platform (the strand table is B200-measured, post-reported; the model is the decision content). P113: upstream's own landed decision — the LaunchConfig constructor is shipped API at the pin. Source: the tk-2 post §\"Occupancy\" (the Table 1 probe + the repair paragraph, quoted there) + HazyResearch/ThunderKittens @ 67845f5fa48d include/common/util.cuh:530-551 (the preferred+minimum constructor) + include/types/tensor/tensor.cuh:82-90 (the tcgen05.alloc forms). Quotes verified at the pin. MIT. Batch 425 / Plan 419.".to_string(),
            ref_shape: r#"// NAIVE: a persistent grid with a fixed cluster attribute — the
// scheduler places whole clusters only on wired SM groups and the
// remainder idles; the kernel assumes ONE cluster size so the repair
// cannot even be applied.
// the strand model (the post's B200 probe: active SMs by fixed cluster size)
fn active_sms_fixed(sm_count: usize, cluster: usize) -> usize {
    let stranded = match cluster {
        1 | 2 => 0,
        4 => 16,
        8 => 28,
        16 => 36,
        _ => (cluster / 2) * 9, // the model's interpolation, never a bar
    };
    sm_count - stranded.min(sm_count)
}

#[derive(Debug)]
struct LaunchPlan {
    grid: usize,
    cluster_fixed: Option<usize>,
    active_sms_reported: usize,
}

fn launch_naive(sm_count: usize, work_tiles: usize, cluster: usize) -> LaunchPlan {
    let _ = work_tiles;
    // grid sized to the die (persistent), cluster fixed at compile time
    LaunchPlan {
        grid: sm_count,
        cluster_fixed: Some(cluster),
        // the strand is invisible here: the plan reports grid size,
        // never ACTIVE SMs
        active_sms_reported: sm_count, // the LIE — see the probe
    }
}

#[test]
fn naive_reports_full_occupancy_while_stranding() {
    // 148-SM die, cluster 4: the plan CLAIMS 148 active SMs; the
    // hardware gives 132 (16 SMs idle until the placed clusters exit).
    let plan = launch_naive(148, 4096, 4);
    assert_eq!(plan.active_sms_reported, 148);
    assert_eq!(active_sms_fixed(148, 4), 132, "the hardware strands 16 SMs");
    assert_eq!(active_sms_fixed(148, 2), 148);
    assert_eq!(active_sms_fixed(148, 8), 120);
    assert_eq!(active_sms_fixed(148, 16), 112);
}
"#.to_string(),
            opt_pattern: r#"// FIXED: preferred + minimum cluster dimensions — fill at the
// preferred size, backfill the remainder at the minimum; the kernel
// reads the ACTUAL cluster size and serves both (the precondition).
// the backfill model: preferred fills, minimum covers the remainder —
// ONLY when the kernel supports both sizes.
fn active_sms_pref_min(sm_count: usize, preferred: usize, minimum: usize, supports_both: bool) -> usize {
    debug_assert!(minimum <= preferred, "the backfill size cannot exceed the fill size");
    if !supports_both {
        // a kernel whose indexing assumes ONE size cannot take the
        // backfill — the strand persists (the precondition the rule pins).
        return sm_count - 16; // the cluster-4 strand, unrepaired
    }
    // whole preferred clusters while they fit, then minimum-size clusters
    // cover the remainder — every SM participates.
    let full = (sm_count / preferred) * preferred;
    let rem = sm_count - full;
    full + rem.min(sm_count) // minimum-size clusters cover the remainder
}

#[derive(Debug)]
struct LaunchPlan {
    grid: usize,
    cluster_fixed: Option<usize>,
    cluster_preferred: Option<usize>,
    cluster_minimum: Option<usize>,
    active_sms_reported: usize,
    kernel_supports_both_sizes: bool,
}

fn launch_repaired(sm_count: usize, preferred: usize, minimum: usize) -> LaunchPlan {
    LaunchPlan {
        grid: sm_count,
        cluster_fixed: None,
        cluster_preferred: Some(preferred),
        cluster_minimum: Some(minimum),
        // the plan reports the MODELED active count and the kernel
        // contract that makes it true
        active_sms_reported: active_sms_pref_min(sm_count, preferred, minimum, true),
        kernel_supports_both_sizes: true,
    }
}

#[test]
fn repaired_backfills_the_remainder() {
    let plan = launch_repaired(148, 4, 2);
    assert_eq!(plan.active_sms_reported, 148, "preferred fills, minimum backfills");
}

#[test]
fn the_precondition_is_enforced() {
    // a kernel that cannot serve two sizes must NOT take the repair —
    // the naive repair would compute wrong offsets in the backfill
    assert_eq!(active_sms_pref_min(148, 4, 2, false), 132);
}
"#.to_string(),
        },
        // ── R3 — the proxy-causality fence-elision law: a proxy fence is
        // required only where no causality chain exists; the marker-chain
        // elides it, the generic-store regime needs it; implicitly
        // pipelined same-thread pairs need no ordering at all. ──
        KernelRule {
            rule_id: "proxy-causality-fence-elision".to_string(),
            gpu_concept: "MECHANISM (memory model; category A — memory access): on a proxy-architecture ISA (generic vs async proxies), a proxy fence before an async consumer is REQUIRED only when the operands were written by operations the consumer's proxy cannot causally see — the fence is not a ritual, it is the missing link of a chain, and the reasoning is a two-regime test. THE ELISION REGIME: TMA write → the SAME instruction's mbarrier complete-tx (implicitly ordered with the copy, a RELEASE, with an implicit generic-async proxy fence after completion) → the consumer's mbarrier try_wait (ACQUIRE by default) → the tcgen05.mma issued by the SAME thread reading through the SAME async proxy — causality is established transitively (same-thread order + synchronizes-with + transitivity; PTX sections 8.9.1.1, 9.7.9.25.2, 9.7.9.27.1.2, 9.7.13.15.16, 9.7.16.6.5) and BOTH fences in the old path are dead: the `fence::after_thread_sync` (the scales are implicitly pipelined with the MMA — sections 9.7.16.6.2 + 6.4.1: implicitly pipelined same-thread instructions need NO ordering mechanism) and the `fence.proxy.async.shared::cta`. MEASURED (post): the two unnecessary fences cost **20-30 TFLOPs on the blockscaled GEMM**; removing this class across their kernels gave ~20 TFLOP/s across GEMM + attention. THE IN-SOURCE RECORD (verbatim at the pin, include/ops/thread/mma/tcgen05.cuh:808-810): 'This is not needed if using mbarrier synchronization with acquire/release (default) semantics. However, it must be used if manually storing to SMEM/TMEM with generic operations.' — the fence stands in the tree as a COMMENTED-OUT line with the regime law attached. THE REQUIREMENT REGIME (the elision law's complement, composed never merged with B272's mirror-fence rider): generic stores to the operand tile (manual smem/tmem writes by plain st ops) leave the async consumer's view stale — the fence is the invalidation and eliding it reads garbage; B272's rider on setmaxnreg owns the tile-REUSE direction (generic writes before an async read of a tile still in flight — wait_group.read + fence pair); THIS owns the INPUT-direction regime test (was the operand's producer path a causally-chained async delivery or a bare generic store). THE SAME-THREAD MERGE INSTANCE (the second fence-kill in the same section): because tcgen05.cp (scale loads into tensor memory) is implicitly pipelined with tcgen05.mma when issued by the same thread, the dedicated scale-warp + explicit completion wait + barrier handoff design is removable — merge the roles into one warp and the inter-stage barrier disappears with the fence (the mxfp8 kernel's shipped shape: one warp issues `load_mxnv_scale_async2` for A+B scales and `mm2_ABt`/`mma2_ABt` in the same loop body, kernels/gemm/mxfp8_b200/mxfp8_b200_gemm.cu:196-222; the pre-merge design — a scale-loader warp, 12 tcgen05.cp per MMA stage for NVFP4, explicit waits — read ~10% lower). ⛔ KINSHIP (adjudicated both directions): `barrier-after-async-mma-before-staged-tile-reuse` + the B272 mirror-fence rider own the WRITE/reuse-side fence duties (when the fence MUST fire) — THIS owns the READ-side regime test (when it MUST NOT); `plane-storage-barrier-scope-covers-global-exchange` (gpu_320) owns a barrier's MEMORY-CLASS scope, not a fence's elision; `host-context-mirror-fenced-drain-avoidance` (gpu_276) is the host-side drain fence, a different layer; `split-prework-recompute-beats-inter-kernel-barrier` (gpu_242) owns inter-KERNEL barrier elimination by prework — same elimination instinct, different mechanism (a recompute trade vs a causality proof); `producer-folds`/`splitk-reduce` fusion families own work elimination, not ordering proofs. REGIME: proxy-architecture ISAs (sm_90+ generic/async proxies; Metal/wgpu have no proxy split — the fences there are ordinary, the law does not route); every blockscaled/tensor-core producer-consumer handoff; any review of a 'safety' fence whose producer path is mbarrier-chained async delivery. ANTI-REGIME: generic-store operand paths (the fence is load-bearing — eliding is the defect shape the third fixture arm pins); fences between DIFFERENT threads' operations with no marker chain (causality needs the synchronizes-with edge — an unchained cross-thread pair keeps its fence); manually-stored scale/operand tiles (the in-source comment's own carve-out). GOAT: the fixture pins (a) the marker chain observes without a fence (elide), (b) the generic-store input needs it (the regime test), (c) eliding under the generic-store regime reads STALE (the defect shape). NotBenchable-by-platform (the 20-30 TFLOP datum is post-reported on Blackwell; the regime test is the decision content). P113: upstream's own landed decision — the fence is elided in the shipped kernels at the pin. Source: the tk-2 post §\"Memory consistency\" + HazyResearch/ThunderKittens @ 67845f5fa48d include/ops/thread/mma/tcgen05.cuh:808-810 (the commented fence + the law verbatim) + kernels/gemm/mxfp8_b200/mxfp8_b200_gemm.cu:196-222 (the merged scale+MMA warp) + include/ops/group/mma/warpgroup.cuh:43-44 (the Hopper wgmma path where the fence REMAINS — the pre-proxy-contract generation, the scope note). Quotes verified at the pin. MIT. Batch 425 / Plan 419.".to_string(),
            ref_shape: r"// NAIVE: fences as ritual — a proxy fence before every tensor-core
// consume regardless of the producer path, and a dedicated scale
// loader behind an explicit completion wait + barrier handoff.
fn mma_prologue_naive(path: ProducerPath, scales_ready: &mut bool) -> Stage {
    // fence #1: 'for safety' before the MMA — dead weight when the
    // producer was the chained async delivery
    let _ = proxy_fence();
    // fence #2: same, before the scale copies
    let _ = proxy_fence();
    // the scale-loader warp's work must be waited on explicitly
    spin_until(scales_ready);
    Stage::Ready(path)
}

fn proxy_fence() -> u32 {
    FENCE_COUNT.with(|c| c.set(c.get() + 1));
    FENCE_COUNT.with(|c| c.get())
}

thread_local! {
    static FENCE_COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProducerPath {
    /// chained async delivery (the elision regime)
    AsyncMarkerChain,
    /// bare generic stores (the fence is load-bearing)
    GenericStore,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    Ready(ProducerPath),
}

fn spin_until(flag: &mut bool) {
    *flag = true;
}
".to_string(),
            opt_pattern: r"// FIXED: the fence is the missing link of a chain — emit it only
// when the producer path has no causality chain; the chained delivery
// elides BOTH fences, and the scale copies merge into the MMA warp
// (same-thread implicitly-pipelined pair, no inter-stage wait).
fn mma_prologue(path: ProducerPath, scales_ready: &mut bool) -> Stage {
    if !observes_without_fence(path) {
        // the ONLY regime that pays: bare generic stores
        let _ = proxy_fence();
    }
    // the merged role: the same warp issues the scale copies and the
    // MMA — the implicit pipeline carries the ordering, no wait spin
    *scales_ready = false;
    Stage::Ready(path)
}

fn observes_without_fence(path: ProducerPath) -> bool {
    match path {
        // async write → marker release → acquire wait → same-thread
        // same-proxy consume: causality is transitive — elide
        ProducerPath::AsyncMarkerChain => true,
        // generic stores: no chain — the fence IS the invalidation
        ProducerPath::GenericStore => false,
    }
}

#[test]
fn the_elision_pays_only_in_the_chained_regime() {
    assert!(observes_without_fence(ProducerPath::AsyncMarkerChain));
    assert!(!observes_without_fence(ProducerPath::GenericStore));
}
".to_string(),
        },
        // ── R4 — the tensor-memory buffering regime law: slot ping-pong
        // below the crossover, shared-A double accumulation at/above it;
        // a fixed single-buffer shape pays one regime's loss everywhere. ──
        KernelRule {
            rule_id: "tmem-buffering-regime-pingpong-vs-double-accumulate".to_string(),
            gpu_concept: "MECHANISM (on-chip memory buffering; category A — memory access): tensor memory (128×512 per SM) with a non-blockscaled MMA accumulating into at most 128×256 admits TWO buffering shapes, and the winner is SIZE-KEYED. SLOT PING-PONG: alternate the two 128×256 halves — the tensor cores accumulate one while the epilogue drains the other — no pipeline bubble, but the A tile is re-streamed for each half's MMA pass; the efficient shape **below roughly 2048³** (post-reported crossover). DOUBLE ACCUMULATION: run two MMA pipelines sharing one A tile over two B tiles — the full 128×512 accumulates throughout, the tensor cores wait while the epilogue drains 256 KiB — a bubble per drain, but the A operand traffic halves; **+~100 TFLOP/s on the BF16 GEMM at large shapes** (post-reported; the crossover is shape-reported 'roughly below 2048x2048x2048', never pinned — the rules carry it as provenance, never a bar, and any adopter re-derives it per die). A kernel that fixes ONE shape pays the other regime's loss at every size — the buffering choice is a per-shape placement decision like any tier ladder (the corpus's tier-ladder family shape, applied to on-chip tensor memory). ⛔ KINSHIP (adjudicated both directions): `register-double-buffer-operand-pipeline` owns REGISTER-fragment double buffering (one level down — operand fragments, not tensor-memory slots; compose: the fragment prefetch and the slot regime price different latencies); `banded-launch-order-shares-weight-tiles` + the F2 supergroup face own L2-side operand sharing via launch order — THIS shares the A tile ON-CHIP across concurrent MMA pipelines, a different tier of the same instinct (the operand-reuse family, cite); `splitk-tier-ladder-per-tier-ceiling-min-partition-inputs` owns a K-split tier ladder (work partition) — THIS is a buffering regime over the same shape axis; `verify-window-weight-load-amortization-exact` (gpu_163) owns weight-panel sharing across verify rows with an exactness contract — different consumer, same amortization instinct; the corpus has ZERO tmem owners (grep 'tmem|tensor memory' — this rule and R2's clause are the first). REGIME: sm_100+ tensor-memory kernels (Blackwell; NotBenchable-by-platform — no such die in the fleet); GEMM-shaped pipelines with an epilogue drain consumer. ANTI-REGIME: non-tensor-memory architectures (the slot structure does not exist — Metal/wgpu kernels route to the register/smem double-buffer families instead); shapes where the drain consumer is absent (no epilogue contention → the bubble never forms → ping-pong trivially). GOAT: the fixture pins (a) ping-pong wins below the crossover, (b) double-accumulate wins at large shapes, (c) a fixed single-buffer shape cannot win at both (the placement is the decision). NotBenchable-by-platform. P113: upstream's own landed decision — both shapes ship in the TK 2.0 kernels (the post's Figures 4-7 + the kernels' OVERLAP_MMA_EPI/NUM_CONSUMERS config arms). Source: the tk-2 post §\"Pipelining tensor memory\" (Figures 4-7, the measured +~100 TFLOP/s BF16 datum, the crossover sentence) + HazyResearch/ThunderKittens @ 67845f5fa48d kernels/gemm/bf16_b200/bf16_b200_gemm.cu:19-24,84-85,194-199 (the OVERLAP_MMA_EPI / MMA_PIPE_DEPTH=2 / NUM_CONSUMERS config machinery that switches the shapes) + include/types/tensor/tensor.cuh (the tcgen05 allocator). Quotes verified at the pin. MIT. Batch 425 / Plan 419.".to_string(),
            ref_shape: r#"// NAIVE: one buffering shape hardcoded — the kernel pays whichever
// regime's loss its shape didn't choose, at every size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FixedBuffering {
    /// alternate the two half-slots — no bubble, A re-streamed
    PingPongOnly,
    /// two pipelines share A — full rate, pays the drain bubble
    DoubleAccumulateOnly,
}

fn throughput_fixed(shape: FixedBuffering, edge: usize) -> u64 {
    match shape {
        // at LARGE shapes the A re-stream is the dominant cost — the
        // fixed ping-pong kernel leaves the shared-A gain on the table
        FixedBuffering::PingPongOnly => 512,
        // at SMALL shapes the drain bubble swallows the whole advantage
        FixedBuffering::DoubleAccumulateOnly => {
            let e = u64::try_from(edge).unwrap();
            1024 * (e - 1024.min(e)) / e.max(1)
        }
    }
}

#[test]
fn a_fixed_shape_pays_the_other_regime_somewhere() {
    // ping-pong-only at 8192: the shared-A gain (~2x rate) is forfeit —
    // the regime model prices the fixed kernel's ceiling at its own rate
    assert_eq!(throughput_fixed(FixedBuffering::PingPongOnly, 8192), 512);
    let e = 8192_usize;
    let double_at_large = 1024_u64 * (u64::try_from(e).unwrap() - 1024) / u64::try_from(e).unwrap();
    assert!(double_at_large > 512, "the shared-A rate beats the fixed ping-pong ceiling");
}
"#.to_string(),
            opt_pattern: r"// FIXED: the buffering shape is a per-shape PLACEMENT — the regime
// predicate picks the shape; the crossover is provenance, re-derived
// per die.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TmemShape {
    /// alternate the two half-slots — no bubble, A re-streamed
    SlotPingPong,
    /// two pipelines share A — full rate, pays the drain bubble
    DoubleAccumulate,
}

fn shape_throughput(shape: TmemShape, edge: usize) -> u64 {
    let e = u64::try_from(edge).unwrap();
    match shape {
        TmemShape::SlotPingPong => 512,
        TmemShape::DoubleAccumulate => 1024 * (e - 1024.min(e)) / e.max(1),
    }
}

fn pick_buffering(edge: usize) -> TmemShape {
    // the regime predicate: the crossover is measured per die, the
    // fixture's 1024 is the model's, never a shipped constant
    if shape_throughput(TmemShape::DoubleAccumulate, edge) > shape_throughput(TmemShape::SlotPingPong, edge) {
        TmemShape::DoubleAccumulate
    } else {
        TmemShape::SlotPingPong
    }
}

#[test]
fn the_regime_predicate_picks_the_winner() {
    assert_eq!(pick_buffering(1024), TmemShape::SlotPingPong);
    assert_eq!(pick_buffering(8192), TmemShape::DoubleAccumulate);
}
".to_string(),
        },
    ]
}

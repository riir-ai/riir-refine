//! Entries slice -- Batch 111 (11 rules) -- the vLLM `libtorch_stable`
//! remainder tier + disposition (Research 133 / Plan 082). Source:
//! vllm-project/vllm `csrc/libtorch_stable/` @ 8277c42e4c74c5dd604f29b6d938a3456658f247
//! (Apache-2.0; re-cloned + re-pinned at the verdict sha at mine time -- origin had
//! moved to 16328c7a minutes before the clone). Two read-only inspection lanes
//! re-verified every quote at the pin; the coordinator byte-verified 4 lead quotes
//! (one per sub-family). This file lands rules #12-#20, #22, #25 of the verdict
//! table: #21 (noinline rare path) and #23 (clamp-before-collective) were KILLED
//! at mine time as COVERAGE (B110's persistent-kernel-length-bucketed-mechanism-
//! ladder protection (b) and spin-barrier companion contract already carry them,
//! same source citations); #24 (capture-stable grid shape) was demoted to a RIDER
//! on device-m-runtime-mblock-graph-stable-gemm (B92 owns the capture-stable
//! core; the per-row-length variant is rider-sized). Fixtures are original
//! minimal Rust reproductions (parse-only; no --fix path). Category membership
//! lives ONLY in `KERNEL_CATEGORIES` (mod.rs) -- never encode it here.
use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic -- grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![

// ============================================================
// -- grid-dependency-wait-scoped-to-dependent-operands -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "grid-dependency-wait-scoped-to-dependent-operands".to_string(),
            gpu_concept: "Scope a programmatic-dependent-launch (PDL) grid-dependency wait to the warps and loads that actually consume producer output, instead of paying the wait at kernel top for every warp. Three in-source postures, escalating in precision: CONSERVATIVE TOP (wait once before any work -- correct when most operands ARE producer output), LOAD-SCOPED (wait inside the branch that performs the dependent load; warps whose operands are pre-wait inputs -- slot mapping, scales, rope table, cache rows -- skip the wait entirely), and ONCE-FLAG (the first consuming iteration waits, later iterations skip via a bool). The TRIGGER half is a visibility contract, not a perf knob: the producer's cudaTriggerProgrammaticLaunchCompletion stays at kernel END -- it 'must not fire before this kernel's stores are issued' or the consumer reads torn output (the recorded upstream hazard: a KDA trigger placed before the output stores). Regime: wins when the kernel consumes only PART of its input from the producer -- non-dependent loads and skip-wait warps overlap producer execution; NO-OP pre-sm90 (guard __CUDA_ARCH__ >= 900) and when PDL is not enabled on the launch; degenerates to the ref shape when the first touched byte is producer output. Distinct from side-consumer-readback-alias-live-handle (B26 -- host readback ordering after a CPU read, not cross-kernel hardware dependency), from spin-barrier-needs-co-residency-capped-launch (B110 -- an intra-grid barrier among co-resident CTAs, not a cross-kernel dependency edge), and from captured-graph-init-unconditional-and-stream-ordered (B110 -- the capture node-set axis of inter-kernel ordering; PDL is the launch-time edge between two LIVE kernels). Portable note: wgpu/CubeCL have no PDL equivalent -- the portable residue is the SCOPING discipline on any explicit producer/consumer handshake (wait as late as the data dependency allows). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- fused_kimi_k3_mla_key_concat_kv_cache_kernel.cu:675-680 ('Only ql_nope / q_pe come from the producing GEMM chain, so everything else ... read before the grid-dependency wait, and the cache-slot warps skip the wait entirely') + :727-729 (load-scoped wait under __CUDA_ARCH__ >= 900), kimi_k3/attn_res_kernel.cu:392-395 (the once-flag), fused_deepseek_v4_qnorm_rope_kv_insert_kernel.cu:445-451 ('No-op when PDL is not enabled on the launch'; conservative top) + :573-575 (the end-of-kernel trigger). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: every warp pays the grid-dependency wait before ANY
// work, even warps whose inputs the producer never wrote.
fn kernel_top(wait: fn()) {
    wait(); // unconditional: every warp, every operand
    load_independent_tables();
    compute();
}

fn load_independent_tables() {}
fn compute() {}
".to_string(),
            opt_pattern: "// optimized: scope the wait to what consumes producer output.
// Independent tables load pre-wait; consuming warps wait once right
// before the dependent work; cache-slot warps never wait at all.
fn kernel_scoped(slot_idx: usize, num_heads: usize, waited: &mut bool, wait: fn()) {
    load_independent_tables(); // slot mapping, scales, rope tables
    if slot_idx < num_heads {
        if !*waited {
            wait(); // once-flag: only the first consuming pass
            *waited = true;
        }
        write_producer_output();
    } else {
        write_cache_row(); // cache-slot warps skip the wait entirely
    }
}

fn load_independent_tables() {}
fn write_producer_output() {}
fn write_cache_row() {}
".to_string(),
        },

// ============================================================
// -- ue8m0-pow2-scale-as-biased-exponent-byte -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "ue8m0-pow2-scale-as-biased-exponent-byte".to_string(),
            gpu_concept: "Encode a per-block dynamic fp8 quantization scale as ONE biased-exponent byte (UE8M0) instead of a 4-byte f32 (or e4m3) scale: floor absmax (fmaxf(absmax, 1e-4f)) so log2f never sees 0 or -inf, take exponent = ceil(log2(absmax / kFp8Max)) -- the CEILING is the correctness half, it guarantees absmax * 2^-exponent fits the fp8 range -- derive inv_scale = exp2f(-exponent), and store clamp(exponent + 127.0, 0.0, 255.0) as the u8. Dequant is a pure exponent-field bump: scale = 2^(byte - 127), an EXACT power of two with zero multiplicative rounding (Issue-845 PoT kin). Sidecar layout 7 real + 1 pad byte per token (kScaleBytesPerToken = kNumQuantBlocks + 1; the pad zeroed) -- 8 sidecar bytes riding a 576-byte token row instead of 28. Regime: wins when scale-sidecar bytes matter against the payload (KV-cache insert bandwidth) and one power-of-two exponent per 64-element block carries acceptable range loss; loses when the block's dynamic range needs sub-octave scale granularity (the whole block shares ONE exponent -- strictly wider error than an e4m3 scale) and when the encoder's log2f/exp2f transcendentals land in a quantize-rate hot path. Distinct from half-domain-exact-band-dequant-with-raw-bit-oracle (B97 -- CONSUMES an existing E8M0 byte for dequant-arithmetic exactness; this rule CREATES the byte from a runtime absmax), from quant-group-scale-fetch-gating (B92 -- when the scale is fetched, not what it is), and from constant-scale-folded-into-upload-copy (B36 -- a CONSTANT hyperparameter folded at upload; this is a per-block RUNTIME scale that must be computed and stored). Gate: batch111_ue8m0_byte_contract pins the floor/ceiling/clamp arithmetic, the PoT-exact decode, and monotonicity. Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- fused_deepseek_v4_qnorm_rope_kv_insert_kernel.cu:88-95 (layout + kFp8Max), :326-328 (floor + ceil + exp2f), :349-350 (clamp(exponent + 127.0, 0.0, 255.0)), :352-353 (pad zeroed). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: a 4-byte f32 scale rides beside every 64-element fp8
// block -- 4 bytes of sidecar where the information fits in an
// exponent, and the dequant multiplies a rounded reciprocal.
struct ScaleF32 {
    scale: f32,
}

fn quantize_ref(absmax: f32, fp8_max: f32) -> ScaleF32 {
    ScaleF32 { scale: absmax / fp8_max }
}
".to_string(),
            opt_pattern: "// optimized: ONE biased-exponent byte. Floor absmax so log2 never
// sees 0/-inf; CEIL the log2 so the scaled payload provably fits the
// fp8 range; clamp the bias-127 exponent into a u8. Dequant is a
// pure exponent-field bump -- an exact power of two, zero rounding.
fn encode_scale_byte(absmax_raw: f32, fp8_max: f32) -> u8 {
    let absmax = absmax_raw.max(1e-4); // floor: log2 never sees 0
    let exponent = (absmax / fp8_max).log2().ceil();
    (exponent + 127.0).clamp(0.0, 255.0) as u8
}

fn decode_scale_byte(byte: u8) -> f32 {
    2.0f32.powi(byte as i32 - 127) // exact power of two
}
".to_string(),
        },

// ============================================================
// -- register-rotation-prefetch-loop -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "register-rotation-prefetch-loop".to_string(),
            gpu_concept: "Software-pipeline independent per-iteration global loads through REGISTERS in a slot loop: prime the current slot's loads before the loop, prefetch the NEXT slot into separate registers guarded by a has_next boolean, process the current slot, and rotate at the loop tail (curr = next). This is the register-only rung of the prefetch ladder -- no shared memory, no cp.async, ragged trip counts safe via the boolean guard -- portable to every backend, unlike the smem ring it sits beside. The rotation (v0_curr = v0_next) hands the issued loads to the next iteration so the memory latency of iteration i+1 overlaps the compute of iteration i. Regime: wins when iterations are INDEPENDENT and the body carries enough compute to overlap the prefetched load (the source body is a full quantize+RoPE+cache-insert per slot); loses when iterations are dependent, the body is too short to hide the latency (the stall merely moves), register pressure would spill (the operand footprint doubles), or the loop is DRAM-bandwidth-bound -- run the weight-tile-consumption-order / defer-readback-gated-by-live-output-size binding-axis test before crediting the lever. Distinct from register-double-buffer-operand-pipeline (B92 -- MMA-fragment arrays under an even-length static-access invariant; this is the scalar slot-loop form with a ragged tail), from cpasync-multistage-tile-pipeline (B92 -- the S-stage SHARED-MEMORY ring above this rung), and from interleave-loads-between-tile-matmuls (B79 -- instruction placement INSIDE a tile pipeline, not a slot-loop rotation). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- fused_deepseek_v4_qnorm_rope_kv_insert_kernel.cu:546-571 (prime :548-549, guarded prefetch :552-559, 'Buffer rotation: hand the prefetched LDGs to the next iter' :566-569). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: load-then-process per iteration -- every iteration
// stalls on its own global loads before its compute can start.
fn slot_loop_ref(slots: usize, mut curr: usize, stride: usize) {
    while curr < slots {
        let (v0, v1) = load_slot(curr); // stall exposed every slot
        process(v0, v1);
        curr += stride;
    }
}

fn load_slot(_slot: usize) -> (u32, u32) {
    (0, 0)
}

fn process(_v0: u32, _v1: u32) {}
".to_string(),
            opt_pattern: "// optimized: prime the first slot, prefetch the next slot into
// SEPARATE registers guarded by has_next, process curr, rotate at
// the loop tail -- the register-only rung: no smem, no async copy,
// ragged-safe via the guard, portable to every backend.
fn slot_loop_prefetch(slots: usize, mut curr: usize, stride: usize) {
    if curr >= slots {
        return;
    }
    let (mut v0_curr, mut v1_curr) = load_slot(curr); // prime
    while curr < slots {
        let next = curr + stride;
        let has_next = next < slots;
        let (v0_next, v1_next) = if has_next { load_slot(next) } else { (0, 0) };
        process(v0_curr, v1_curr);
        v0_curr = v0_next; // Buffer rotation: hand the prefetch on
        v1_curr = v1_next;
        curr = next;
    }
}

fn load_slot(_slot: usize) -> (u32, u32) {
    (0, 0)
}

fn process(_v0: u32, _v1: u32) {}
".to_string(),
        },

// ============================================================
// -- shared-operand-groups-per-warp -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "shared-operand-groups-per-warp".to_string(),
            gpu_concept: "Group the K work items that SHARE one small operand into a single warp -- the operand-affinity axis: K token-heads of one token share that token's cos/sin row, so one warp processes all K heads and stages the operand once instead of K warps each fetching the same row (N x redundant global loads). Stage the operand via cp.async and ORDER the two commit groups by consumer need: group 0 (QKV, consumed first by the RMSNorm) committed first, group 1 (cos/sin) second; cp_async_wait_group<1>() then unblocks group-0 compute while group 1 is still in flight, and the full wait_group<0> defers to the first head's RoPE -- by which time group 1 has overlapped the entire RMSNorm + weight-preload phase. Regime: wins when multiple consumers share one small operand (a rotary_dim x 2-byte row per token) AND a compute phase separates the first consumer from the second; loses when heads-per-token is 1 (no grouping gain -- the launcher's base kernel is the declared fallback), when the operand outgrows the warp's smem share, or when both operands are needed immediately (a front wait_group<0> is the ref shape again). Distinct from cpasync-multistage-tile-pipeline (B92 -- S-stage ring WAIT-DEPTH for one tile stream; this is TWO groups ordered by consumer dependency inside one warp), and from share-activation-stream-keep-projection-reductions (B98 -- the GEMV-side same-activation walk-share with separate accumulators; this is the load-side operand staging). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- fused_qknorm_rope_kernel.cu:308-311 ('one warp processes HEADS_PER_WARP token-heads for the same token, sharing cos/sin'), :407-426 (commit group 0 then group 1; wait_group<1>), :433-441 (the weight preload inside the overlap window), :483-484 ('On first head: wait for group 1 before RoPE' -- wait_group<0>). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: one warp per token-head -- every warp fetches the same
// cos/sin row for its token, and a single blocking wait stalls all
// compute before either operand group is consumed.
fn rope_ref(heads: usize, fetch_cos_sin: fn()) {
    for _head in 0..heads {
        fetch_cos_sin(); // N x redundant loads of the same row
        apply_rope();
    }
}

fn fetch_cos_sin() {}

fn apply_rope() {}
".to_string(),
            opt_pattern: "// optimized: group the K heads of ONE token into one warp (the
// operand-affinity axis); stage cos/sin once; order the two async
// groups by consumer need -- wait_group<1> unblocks group-0 compute
// while group 1 is still in flight; the full wait moves to the first
// head's RoPE, after group 1 overlapped the whole RMSNorm phase.
fn rope_grouped(heads: usize) {
    commit_group_qkv(); // group 0: consumed by RMSNorm -- commit FIRST
    issue_cos_sin_copy(); // group 1: the shared operand, staged once
    rmsnorm_with_weight_preload(); // overlaps group 1 in flight
    wait_group_1(); // group 0 done; group 1 may still be landing
    for head in 0..heads {
        if head == 0 {
            wait_all(); // the full wait: first RoPE is group 1's consumer
        }
        apply_rope_head(head);
    }
}

fn commit_group_qkv() {}

fn issue_cos_sin_copy() {}

fn rmsnorm_with_weight_preload() {}

fn wait_group_1() {}

fn wait_all() {}

fn apply_rope_head(_head: usize) {}
".to_string(),
        },

// ============================================================
// -- launch-heuristic-pinned-to-calibration-arch -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "launch-heuristic-pinned-to-calibration-arch".to_string(),
            gpu_concept: "An inline launch heuristic's measured thresholds are facts about the SKU they were calibrated on: initialize the CONSERVATIVE baseline as the default and gate the tuned ladder to the calibration architecture (sm_version == 90 at the source), leaving other SKUs on the baseline 'until profiled', with an explicit forced override as the escape hatch. The naive shape -- an if-chain of tuned thresholds applied to every architecture -- silently routes every other SKU onto another SKU's optimum, and launch optima are non-monotonic across SKU x shape (the premise the swept-table rules exist for). This rule is the NO-SWEEP-INFRASTRUCTURE guardrail that refuses to extrapolate at all: it costs leaving measured perf on the table on non-calibrated SKUs, which is the deliberate trade, and the B104 offline-swept table is the upgrade path once the sweep investment is justified -- B104's ref_shape IS this rule's naive shape, and the two compose (pinned-heuristic today, swept table tomorrow). Regime: any inline heuristic encoding measured thresholds (heads-per-warp ladders, stage counts, block-size picks); the win is preventing un-profiled regressions, the cost is foregone unmeasured gains. Distinct from swept-shape-bucket-launch-tables + no-harm-bucket-representative (B104 -- the offline-swept TABLE with family-representative inheritance and worst-case-regret rows; neither carries the SKU-gate-the-INLINE-heuristic discipline), and from vendor-fused-op-instantiation-check (B106 -- reads the shipped binary's actual specializations; this gates YOUR OWN heuristic's calibration scope). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- fused_qknorm_rope_kernel.cu:769-796 ('Auto thresholds are calibrated on SM 9.0 (H100). On other architectures, fall back to token_heads_per_warp=1 (base kernel) until profiled' :770-771; baseline :776; the sm_version == 90 gate :779; forced override :773-775). Batch 111 / Research 133 / Plan 082.An inline launch heuristic's measured thresholds are facts about the SKU they were calibrated on: initialize the CONSERVATIVE baseline as the default and gate the tuned ladder to the calibration architecture (sm_version == 90 at the source), leaving other SKUs on the baseline 'until profiled', with an explicit forced override as the escape hatch. The naive shape -- an if-chain of tuned thresholds applied to every architecture -- silently routes every other SKU onto another SKU's optimum, and launch optima are non-monotonic across SKU x shape (the premise the swept-table rules exist for). This rule is the NO-SWEEP-INFRASTRUCTURE guardrail that refuses to extrapolate at all: it costs leaving measured perf on the table on non-calibrated SKUs, which is the deliberate trade, and the B104 offline-swept table is the upgrade path once the sweep investment is justified -- B104's ref_shape IS this rule's naive shape, and the two compose (pinned-heuristic today, swept table tomorrow). Regime: any inline heuristic encoding measured thresholds (heads-per-warp ladders, stage counts, block-size picks); the win is preventing un-profiled regressions, the cost is foregone unmeasured gains. Distinct from swept-shape-bucket-launch-tables + no-harm-bucket-representative (B104 -- the offline-swept TABLE with family-representative inheritance and worst-case-regret rows; neither carries the SKU-gate-the-INLINE-heuristic discipline), and from vendor-fused-op-instantiation-check (B106 -- reads the shipped binary's actual specializations; this gates YOUR OWN heuristic's calibration scope). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- fused_qknorm_rope_kernel.cu:769-796 ('Auto thresholds are calibrated on SM 9.0 (H100). On other architectures, fall back to token_heads_per_warp=1 (base kernel) until profiled' :770-771; baseline :776; the sm_version == 90 gate :779; forced override :773-775). Batch 111 / Research 133 / Plan 082. RIDER (Batch 130 external instance, cloudrift-ai/emmy @ 2aaa0328 -- deplodock/compiler/cuda/tuning.py:7-11,59-62,76-101,92-101): three sharpenings of the calibration-key discipline. (1) COMPUTE CAPABILITY IS NOT A SUFFICIENT SKU KEY -- 'Dispatch is by GPU name (from nvidia-smi --query-gpu=name) because both cards report sm_120 and identical per-SM smem, so compute capability cannot distinguish them' (:7-9): RTX 5090 and RTX PRO 6000 tune to different thread_m at 8192 (28 vs 26) and different k_splits at 1024 (1 vs 4), so the profile table keys on nvidia-smi NAME regexes (:92-98) with the 5090 profile as the validated fallback ('within ~5-10% of the per-card optimum', :9-11). (2) THE TILE OPTIMUM IS A PROPERTY OF THE DMA GENERATION, NOT JUST THE SM -- 'Quick TM sweep on H200 showed TM=8 was optimal at every size -- larger thread tiles regress (likely the first-gen Hopper TMA has more issue-pressure than Blackwell's refined unit)' (:59-62): same TMA instruction, opposite tile answer per silicon generation. (3) A SAME-CC DIVERGENCE CAN BE LIBRARY-SIDE -- Pro 6000 batched loses not because of hardware but because 'cuBLAS dispatches the LARGE 256x128_8x4 kernel for batched on Pro 6000 (73% FMA pipe util) instead of the SMALL 128x32_8x5 kernel it picks on 5090 (41% util) ... the difference is in cuBLAS's dispatcher heuristic, not the hardware' (:76-82) -- a vendor heuristic delta can erase or counterfeit a per-SKU tuning win, so attribute before tuning. Companion regime note (lower.py:529-534): 'The smaller per-thread tile (vs the previous tm=28 single-8192-tuned default) generalizes better across grid-saturation regimes -- wins in 6/9 of the (size, batch) sweep cases' -- calibrate for the REGIME MIX, never the single headline size.".to_string(),
            ref_shape: "// naive: thresholds calibrated on one SKU applied to every
// architecture -- other SKUs silently take another SKU's optimum.
fn heads_per_warp_ref(total_qk_units: i64, head_dim: usize) -> usize {
    if total_qk_units > 10240 && head_dim >= 256 {
        4
    } else {
        2
    }
}
".to_string(),
            opt_pattern: "// optimized: the conservative baseline is the DEFAULT; the tuned
// ladder is gated to the SKU it was calibrated on; other SKUs keep
// the baseline until profiled, with an explicit forced override.
fn heads_per_warp_pinned(
    sm_version: u32,
    total_qk_units: i64,
    head_dim: usize,
    forced: usize,
) -> usize {
    if forced > 0 {
        return forced; // explicit override for un-profiled SKUs
    }
    let mut heads = 1; // conservative baseline everywhere
    if sm_version == 90 {
        if total_qk_units > 10240 && head_dim >= 256 {
            heads = 4; // calibrated on SM 9.0 (H100) only
        } else {
            heads = 2;
        }
    }
    heads
}
".to_string(),
        },

// ============================================================
// -- warp-aggregated-atomic-scatter -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "warp-aggregated-atomic-scatter".to_string(),
            gpu_concept: "Aggregate a warp's append-order scatter into ONE leader atomicAdd: ballot the hit predicate, the leader adds the warp's hit COUNT to the shared output counter once, __shfl_sync broadcasts the base to every lane, and each lane's rank is popc(mask & lanemask_lt) -- its number of hitting peers below it. Contention drops from O(hits) serialized read-modify-writes to O(warps), and the scatter order stays deterministic (lanes land in lane order within each warp's block). The ANTI-INSTANCE ships in the same file: the radix stage-3 collect does a per-element atomicAdd in a strided loop with no aggregation. Regime: wins when hits are dense across the warp (append-order scatter into shared/global slots); neutral-to-loss when at most one lane hits per warp -- the ballot + shuffle overhead buys nothing there. Distinct from ballot-matrix-tie-rank (B110 -- popc(ballot) as a RANK among ties with zero atomics; the two compose: aggregation assigns slots, the ballot matrix orders ties), from atomic-scatter-reduction (popcorn -- VALUE accumulation by atomic; this is SLOT ASSIGNMENT), and from expert-major-work-map-token-scatter-clamped (B97 -- map building by atomic claims with clamped writes; different consumer contract). Portable: Metal simd_ballot + broadcast, WGSL subgroup ops; the scalar fallback is a per-lane atomicAdd -- correct, just contended. Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- persistent_topk.cuh:271-283 (ballot / popc rank / leader atomicAdd :277 / shfl base :279; the equal-arm twin :285-295), :835-838 (the per-element anti-instance). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: every hit lane does its own atomicAdd on the shared
// output counter -- contention O(hits); a warp of 32 hits pays 32
// serialized read-modify-writes.
fn scatter_ref(hit_mask: u32, lane: u32, counter: &mut u32, out: &mut [Option<u32>]) {
    if hit_mask & (1 << lane) != 0 {
        let base = *counter; // the per-lane atomicAdd
        *counter += 1;
        out[base as usize] = Some(lane);
    }
}
".to_string(),
            opt_pattern: "// optimized: ballot the predicate, ONE leader atomicAdd for the
// warp's hit count, shfl the base to every lane, rank = popc of the
// hits below -- contention O(warps), deterministic lane order.
fn scatter_warp(hit_mask: u32, lane: u32, counter: &mut u32, out: &mut [Option<u32>]) {
    let count = hit_mask.count_ones(); // the ballot result
    if count == 0 {
        return;
    }
    let base = *counter; // ONE leader atomicAdd for the whole warp
    *counter += count;
    let rank = (hit_mask & ((1u32 << lane) - 1)).count_ones(); // lanemask_lt
    if hit_mask & (1 << lane) != 0 {
        out[(base + rank) as usize] = Some(lane);
    }
}
".to_string(),
        },

// ============================================================
// -- refine-by-rescan-prefix-filter-vs-buffered-ties -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "refine-by-rescan-prefix-filter-vs-buffered-ties".to_string(),
            gpu_concept: "When a multi-pass top-k refinement must collect the threshold bin's ties, the collection strategy is ONE decision gated on a MEASURED size, and the wrong arm is a silent wrong answer: a fixed-capacity tie buffer whose occupancy guard merely expects to fit (__builtin_expect(pos < CAP, 1)) SILENTLY DROPS the overflow -- coarse ties differing in low fp32 bits then refine unwritten workspace slots and the emitted top-K is wrong with no error (the recorded H1 hazard: 'le_full may exceed the per-block tie buffer cap; using it here creates holes in tie_ws and can make TopK=2048 refine unwritten workspace slots'). The fix contract is the sampler's FIT-GUARD: measure the bin size against the buffer capacity FIRST and let the measurement pick the strategy -- buffer only when the measured size fits (fill the final items only when smemFinalBinSize <= kNumFinalItems), else keep radix-stepping and RE-SCAN the full row each round filtered by the accumulated bit-prefix (no buffer until the final bin). When a buffer is unavoidable, CLAMP to what was actually written (le = min(le_full, kMaxTies) -- 'written smem tie_buffer entries'), never to what was counted. One decision, three in-source instances: the sampler's buffered-vs-rescan pair, the cooperative clamp, and use_singlepass = per_block <= kFusedMax choosing fused-rescan vs double-buffered streaming. Regime: the fit-guard costs nothing in the common fits case; the buffered arm wins when the tie set fits AND the row is expensive to re-stream; the rescan arm pays bandwidth but never drops. Distinct from persistent-kernel-length-bucketed-mechanism-ladder (B110 -- names the ladder tiers and carries the H1 prose; the strategy PAIR + fit-guard contract was uncovered), from tile-loop-back-edge-needs-multi-tile-bitexact-gate (B57 -- the fixture-population lesson that makes the overflow class testable: fixtures must cross the boundary), and from its sibling coarse-precision-first-exact-refine (the coarse-key contract this refinement consumes). Gateable: the fit-guard is assertable (a planted oversized bin must take the rescan arm or refuse loudly, never drop). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- sampler.cu:282-284 (the fit-guard), :328 (return smemFinalBinSize[0] > kNumFinalItems -- continue to next step), cooperative_topk.cuh:404-406 (the H1 hazard comment), :396 (the clamp), persistent_topk.cuh:294/:402 (the silent-drop class), :521-523 (the single-pass-vs-streaming third instance). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: a fixed-capacity tie buffer whose guard SILENTLY DROPS
// the overflow -- unwritten slots then refine into the emitted
// top-K, wrong, with no error anywhere.
fn refine_ref(ties: &[Tie], cap: usize, out: &mut Vec<Tie>) {
    for (pos, tie) in ties.iter().enumerate() {
        if pos < cap {
            out.push(*tie); // __builtin_expect(pos < CAP, 1): silent drop
        }
    }
}

struct Tie {
    idx: u32,
    score: f32,
}
".to_string(),
            opt_pattern: "// optimized: MEASURE the bin size against the capacity FIRST and
// let the measurement pick the strategy: buffer when it fits, else
// keep radix-stepping and re-scan the row under the accumulated
// bit-prefix. The fit-guard IS the H1 fix contract.
fn refine_measured(ties: &[Tie], cap: usize, out: &mut Vec<Tie>) -> bool {
    let bin_size = ties.len();
    if bin_size <= cap {
        out.extend_from_slice(ties); // buffered arm: fits, zero re-scan
        true
    } else {
        rescan_with_prefix(ties, cap, out); // rescan arm: never drops
        false
    }
}

fn rescan_with_prefix(ties: &[Tie], cap: usize, out: &mut Vec<Tie>) {
    for tie in ties.iter().take(cap) {
        out.push(*tie); // the prefix filter bounds the work
    }
}

struct Tie {
    idx: u32,
    score: f32,
}
".to_string(),
        },

// ============================================================
// -- coarse-precision-first-exact-refine -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "coarse-precision-first-exact-refine".to_string(),
            gpu_concept: "Run the FIRST selection pass on a LOSSY coarse key and reserve the exact key for refinement: convert f32 to f16 (__float2half_rn), sign-flip into an order-preserving integer ((bits & 0x8000) ? ~bits : bits | 0x8000 -- negatives flipped so ascending integer order tracks ascending float order), and keep only the top 8/12 bits as the histogram bin index. The cheap pass isolates the threshold BIN; exact refinement then recomputes ordered keys from the ORIGINAL f32 values, never from the lossy key -- the lossy key may only DISCRIMINATE bins, never rank within them. The dtype pairing is mandatory and per-type (FilteredTopKTraits<float> vs half/bf16 variants): the coarse width and the exact ordered-key width must be chosen together. Regime: wins when the coarse bin's tie population is small (bin width ~4 elements/bin at the source's 4096 bins over 16384 elements) and shared memory is scarce -- the narrow coarse key buys the histogram its footprint; loses on degenerate score distributions where huge tie bins pile up (many refinement rounds, or H1-overflow territory -- pair with refine-by-rescan-prefix-filter-vs-buffered-ties). Distinct from radix-select-ordered-key-topk (B107 -- starts from the FULL-precision key; the delta is exactly this: a finer first pass trades histogram smem for refinement rounds), from persistent-kernel-length-bucketed-mechanism-ladder (B110 -- names coarse tiers in its ladder but not the lossy-vs-exact key contract), and from ballot-matrix-tie-rank (B110 -- ranks the tie set the coarse pass produces). Gateable: the sign-flip form is exhaustively assertable (order preservation over a f32 sweep incl. +/-0 and the f16-rounding collision band). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- persistent_topk.cuh:1004-1025 (the float trait: 'coarse histogram uses FP16 high 8 bits, 4 refinement rounds'; ToCoarseKey :1012-1020; exact ToOrdered :1022-1025), topk_histogram_4096.cuh:76-84 (extract_coarse_bin_N, the 12-bit variant). Batch 111 / Research 133 / Plan 082. RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- ggml-metal-ops.cpp:5360-5362): the Metal radix-select is the same coarse-pass shape with a different radix width -- 'Maps each float to an order-preserving unsigned key, finds the k-th largest via 4 radix-8 histogram passes, then compacts the top-k indices' -- the per-dtype key-map + fixed-pass-count structure on a second backend (4 passes of radix 8 vs this rule's derived-pass-count; the DERIVED count remains the sharper content).".to_string(),
            ref_shape: "// naive: radix select from the FULL-precision ordered key in
// every pass -- every pass pays the wide key's smem and bandwidth.
fn ordered_key_ref(x: f32) -> u32 {
    let bits = x.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits | 0x8000_0000
    }
}
".to_string(),
            opt_pattern: "// optimized: the FIRST histogram pass runs on a LOSSY coarse key
// (f32 -> f16 -> sign-flip order-preserving -> top 8 bits) to
// isolate the threshold bin cheaply; exact refinement recomputes
// keys from the ORIGINAL f32 -- the coarse key only picks the bin.
fn coarse_key(x: f32) -> u8 {
    let bits = f16_bits(x); // __float2half_rn
    let key = if bits & 0x8000 != 0 {
        !bits & 0xFFFF
    } else {
        bits | 0x8000
    };
    (key >> 8) as u8
}

fn f16_bits(_x: f32) -> u16 {
    0
}
".to_string(),
        },

// ============================================================
// -- whole-row-register-resident-under-length-budget -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "whole-row-register-resident-under-length-budget".to_string(),
            gpu_concept: "When a selection row fits the per-thread register budget -- 16 floats = 4 x float4 per thread, i.e. rows up to 16384 at 1024 threads -- load the row ONCE and let every phase (histogram, scatter, tie collection) consume the registers, storing tie candidates as (idx, score) pairs in shared memory so refinement never re-reads global. Beyond the budget, buffer INDICES only (4 bytes instead of 8 per candidate, no score re-derivation cost) and pay the re-read per phase -- the fallback is the correct arm, not a failure, and the constexpr budget constant (kHist4096VecsPerThread * 4 * kBlockSize) is the compile-time switch between the arms. Regime: wins in the decode/medium regime where rows fit the budget -- each dropped re-read is a full row's DRAM (or L2) traffic; neutral beyond it, and counterproductive if forcing residency spills registers (the occupancy loss outweighs the re-reads -- the same trade register-budget-head-split-gqa-reread-trade prices). Distinct from fwht-shuffle-xor-butterfly-register-resident (B96 -- a TRANSFORM's exchange network held in registers; this is multi-phase selection over one resident row), from row-tile-amortize-input-load (B01 -- weight-TILE amortization across output rows; this is row residency across PHASES), and from multi-simdgroup-shared-staged-tile (B108 -- the SHARED-MEMORY staging tier between registers and DRAM). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- topk_histogram_4096.cuh:24-26 (the budget constant, '// 16384'), :347-349 ('Phase 1: Load all data into RF' -- float4 vecs[4]), :31-34 + :471 (struct Tie {idx, score}; 'smem->tie_buffer[pos] = {idx, elems[e]}' -- refinement never re-reads), persistent_topk.cuh:403 (the beyond-budget arm: buffer indices only). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: every phase re-reads the row from global -- histogram,
// scatter, and refinement each pay the full row's traffic again.
fn phases_ref(row: &[f32]) -> u32 {
    histogram_pass(row) + scatter_pass(row) + refine_pass(row)
}

fn histogram_pass(_row: &[f32]) -> u32 {
    0
}

fn scatter_pass(_row: &[f32]) -> u32 {
    0
}

fn refine_pass(_row: &[f32]) -> u32 {
    0
}
".to_string(),
            opt_pattern: "// optimized: when the row fits the per-thread register budget
// (16 floats = 4 x float4 -> rows <= 16384 at 1024 threads), load
// the row ONCE; every phase consumes registers; ties are stored as
// (idx, score) pairs so refinement never re-reads. Beyond the
// budget: buffer indices only and pay the re-read.
const VECS_PER_THREAD: usize = 4;
const BLOCK: usize = 1024;

fn phases_resident(row: &[f32]) -> u32 {
    if row.len() > VECS_PER_THREAD * 4 * BLOCK {
        return phases_ref(row); // beyond the budget: the fallback arm
    }
    let mut regs = [0.0f32; 16]; // the row, register-resident
    for (i, r) in regs.iter_mut().enumerate() {
        *r = row.get(i).copied().unwrap_or(0.0);
    }
    histogram_regs(&regs) + scatter_and_store_ties(&regs)
}

fn histogram_regs(_regs: &[f32; 16]) -> u32 {
    0
}

fn scatter_and_store_ties(_regs: &[f32; 16]) -> u32 {
    0
}

fn phases_ref(row: &[f32]) -> u32 {
    row.len() as u32
}
".to_string(),
        },

// ============================================================
// -- lane-owned-topk-output -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "lane-owned-topk-output".to_string(),
            gpu_concept: "Distribute the K top-k results across lanes -- result k is parked on lane k DURING the iterative reduction (if (laneIdx == kk) lanePacked = packedMax) and unpacked only on lanes < K (others take minValue / -1) -- instead of every lane holding every result and each consumer thread gathering its slot afterward. Zero post-reduction gather: the consumer's per-lane divergent follow-up reads its own register. The distribution is GATED, twice: a compile-time resource-fit check (LaneOwnedResourcesFit: NumWorkerWarps <= BlockDim / WARP_SIZE && MergeValuesPerLane <= 64 -- the intermediate values per lane must fit the register file) AND an expert-count range (kEnabled = 512..=1024 experts, 9..=16 top experts -- the regime where per-lane ownership beats a broadcast). Regime: wins when each of the K results feeds PER-LANE DIVERGENT follow-up (each lane routes its own expert); does NOT apply when consumers need all K per thread -- the distribution then costs an extra exchange the broadcast never paid. Distinct from ballot-matrix-tie-rank (B110 -- lane i holds tie i for RANKING, a different lane-ownership: ties during rank computation, not results after selection), and from cross-lane-column-packed-projections (B81 -- dispatch packing of DIFFERENT inputs over one shared weight, not result distribution). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- moe/moeTopKFuncs.cuh:309-318 (the range constants), :344-363 (the K loop with 'if (laneIdx == kk) lanePacked = packedMax' and the laneIdx < K unpack), moe/grouped_topk_kernels.cu:940-943 (LaneOwnedResourcesFit + UseHierarchicalLaneTopK). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: all lanes hold all K results and each consumer thread
// gathers its slot afterward -- K x lanes of register file plus a
// post-reduction gather.
fn topk_ref(scores: &[f32], k: usize) -> Vec<(f32, usize)> {
    let mut v: Vec<(f32, usize)> = scores
        .iter()
        .copied()
        .enumerate()
        .map(|(i, s)| (s, i))
        .collect();
    v.sort_by(|a, b| b.0.total_cmp(&a.0));
    v.truncate(k);
    v // every lane holds every result; consumers gather later
}
".to_string(),
            opt_pattern: "// optimized: result k is parked on lane k during the reduction
// and unpacked only on lanes < K -- zero post-reduction gather --
// gated by a compile-time resource-fit check and an expert range.
const MIN_EXPERTS: usize = 512;
const MAX_EXPERTS: usize = 1024;
const MIN_TOP: usize = 9;
const MAX_TOP: usize = 16;

fn lane_owned_enabled(num_experts: usize, top: usize, merge_per_lane: usize) -> bool {
    num_experts >= MIN_EXPERTS
        && num_experts <= MAX_EXPERTS
        && top >= MIN_TOP
        && top <= MAX_TOP
        && merge_per_lane <= 64 // LaneOwnedResourcesFit
}

fn unpack_lane_owned(lane: usize, packed: Option<(f32, usize)>) -> (f32, i32) {
    if lane < MAX_TOP {
        match packed {
            Some((score, idx)) => (score, idx as i32),
            None => (f32::NEG_INFINITY, -1),
        }
    } else {
        (f32::NEG_INFINITY, -1)
    }
}
".to_string(),
        },

// ============================================================
// -- routing-by-precomputed-lookup -- Batch 111 / Research 133 / Plan 082.
        KernelRule {
            rule_id: "routing-by-precomputed-lookup".to_string(),
            gpu_concept: "When expert routing is a deterministic function of token IDENTITY -- not of the current scores -- precompute it offline into a table and turn the hot path into a gather: the kernel loads the token's pre-selected expert ids (6 gather loads instead of an E-expert scoring pass + top-k reduction) and recomputes only the gate WEIGHTS from the live input. Out-of-table classes still get a compute path, behind an explicit sentinel check (image-sentinel token ids 'skip the tid2eid lookup and select experts by score' on the full E-expert branch) -- the fallback is a BRANCH, never a silent miss. Regime: wins when table hits dominate (text tokens in a mixed-modality serving mix) and E is large (E-expert top-k collapses to 6 loads); loses or does-not-apply when routing depends on current scores rather than identity (the table cannot see the input), when the identity-to-expert map is not stable across edits (a stale table is silently wrong -- pair the table with a content/version commitment), or when table misses are common (the fallback arm dominates and buys nothing). Distinct from static-classification-hoist-to-routing-table (B32b -- host-side classification hoisted to a routing table with no in-kernel out-of-table compute arm; this keeps a live in-kernel fallback), from degenerate-selection-falls-back-to-identity-uniform (B110 -- NaN/Inf neutralization before selection, not identity-routed lookup), and from monomorphic-hot-caller-last-hit-memo (B98 -- memoizes the LOOKUP cost at a monomorphic call site; this replaces the COMPUTATION itself with the lookup). Source: vllm-project/vllm @ 8277c42e4 (Apache-2.0) -- moe/topk_softplus_sqrt_kernels.cu:168-176 ('only load and calculate for 6 experts; expert = static_cast<int>(tid2eid[token_id * 6 + lane])'), :92-96 (the sentinel-class comment), :98/:106-167 (the full E-expert fallback branch). Batch 111 / Research 133 / Plan 082.".to_string(),
            ref_shape: "// naive: every token runs the full E-expert scoring + top-k,
// even though its expert set is a fixed function of its identity.
fn route_ref(token_id: u64, expert_scores: &[f32], e: usize) -> Vec<usize> {
    let _ = token_id; // identity unused: full compute every token
    let mut idx: Vec<usize> = (0..e).collect();
    idx.sort_by(|&a, &b| expert_scores[b].total_cmp(&expert_scores[a]));
    idx.truncate(6);
    idx
}
".to_string(),
            opt_pattern: "// optimized: routing is a deterministic function of token
// IDENTITY -- an offline-built table turns the hot path into 6
// gather loads over pre-selected experts; the weight recompute stays
// live; only OUT-OF-TABLE tokens take the E-expert fallback branch.
const OUT_OF_TABLE: u64 = u64::MAX;

fn route_lookup(
    token_id: u64,
    table: &[[usize; 6]],
    scores: &[f32],
    e: usize,
) -> Vec<usize> {
    if token_id == OUT_OF_TABLE {
        return route_ref(token_id, scores, e); // the sentinel branch
    }
    table[(token_id as usize) % table.len()].to_vec() // 6 gather loads
}

fn route_ref(token_id: u64, expert_scores: &[f32], e: usize) -> Vec<usize> {
    let _ = token_id;
    let mut idx: Vec<usize> = (0..e).collect();
    idx.sort_by(|&a, &b| expert_scores[b].total_cmp(&expert_scores[a]));
    idx.truncate(6);
    idx
}
".to_string(),
        },
    ]
}

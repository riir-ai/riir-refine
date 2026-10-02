use super::*;
use crate::domain::HealEntry;

/// The stub-check accessor across the domain's two `Verdict` cfg states:
/// without `kernel_verify` the verdict is `KernelVerdict` (`.parses`); with
/// it, `verify::KernelVerifyResult` (`.compiles` — the stub verifier maps a
/// parse failure to `compiles = false`, so both fields assert the same
/// property). Every batch G1 below asserts through this helper so BOTH
/// feature states compile the same test body (the all-features lib suite
/// red at the Batch 138/139 conflict was exactly this asymmetry).
#[cfg(not(feature = "kernel_verify"))]
#[must_use]
fn verdict_passes(v: &crate::domains::kernel_opt::KernelVerdict) -> bool {
    v.parses
}

#[cfg(feature = "kernel_verify")]
#[must_use]
fn verdict_passes(v: &crate::domains::kernel_opt::verify::KernelVerifyResult) -> bool {
    v.compiles
}

#[test]
fn seed_corpus_round_trips() {
    let corpus = build_kernel_seed_corpus();
    assert!(corpus.verify());
    let frozen = corpus.freeze().unwrap();
    let thawed = Corpus::<KernelOptDomain>::thaw(&frozen).unwrap();
    assert_eq!(thawed.commitment, corpus.commitment);
    assert!(thawed.verify());
}

#[test]
fn seed_corpus_has_expected_entry_count() {
    let corpus = build_kernel_seed_corpus();
    // Batch 1 (21): 2 original + 19 popcorn-distilled.
    // Batch 2 (7): flash-attention tiling.
    // Batch 3 (6): linear-attention recurrence.
    // Batch 4 (9): MoE dispatch + backward pass.
    // Batch 5 (4): NSA block-sparse selection.
    // Batch 6 (3): varlen packing + chunked reductions.
    // Batch 7 (9): burn-cubecl autotune dispatch + conv-as-GEMM + layout.
    // Batch 8 (9): burn-cubecl kernel polymorphism + dtype/alignment + cubecl-opt IR strength reduction.
    // Batch 9 (8): popcorn fusion + dtype/quant + dispatch (add_rms_norm, group_norm_linear, jsd, int8_int2_matmul, _quant).
    // Batch 10 (12): popcorn compute + chunked reduction + polymorphism (mean_pooling, solve_tril, hadamard_transform, comba, dyt, token_shift, sparsemax, selective_log_softmax, softmax, rms_norm, geglu, swiglu, rope, gelu).
    // Batch 11 (11): popcorn linear-attention recurrence family (gla, simple_gla, based, rebased, linear_attn, retention, lightning_attn, rwkv7, delta_rule, iplr_delta_rule, gated_oja_rule, deltaformer, chunk_global_cumsum, chunk_local_cumsum).
    // Batch 12 (14): popcorn gated delta + specialized recurrence + advanced attention (gated_delta_rule, gated_delta_product, mesa_net, mesa_net_decode, titans_linear, ttt, rwkv4, rwkv6, wall_attn, multi_token_attention, momoe, kda_gate, kda_gate_cumsum, log_linear_attn).
    // Batch 13 (6): popcorn final pass — RL loss stability (grpo_offpolicy), complex-free RoPE (llama4_rope), multimodal RoPE (qwen2vl_mrope), RWKV-7 channel mixing + gate output (rwkv7_channel_mixing, rwkv7_gate_output, sqrelu).
    // Batch 14 (4): riir-gpu production kernels — norm-fold into matmul, reduction-split pre-pass, single-plane reduction, CPU-bottleneck offload (gemv_q4k_batched_rmsnorm_cubecl, gpu_transpose).
    // Batch 15 (4): riir-gpu production kernels — block-contiguous weight layout, fused dual GEMV + GeGLU, orthogonal pre-rotation, water-fill tail-dim shared codebook (gemm_ternary_block_contiguous, gemv_geglu_f16, spectralquant).
    // Batch 16 (4): riir-gpu production kernels — cmma hardware unit, dequant-once-per-k-tile, dispatch axis swap, norm-decoupled low-bit scoring (gemm_ternary_simdgroup, forward_turboquant).
    // Batch 17 (4): riir-gpu production kernels — skip-all-zero-packed-words-via-or-reduce, hoist-group-invariant-scale-out-of-word-loop, split-sign-accumulators-defer-subtraction, clamp-tile-indices-to-sentinel-discard-at-write (gemm_ternary_batched, gemv_ternary_block_contiguous).
    // Batch 18 (3): riir-gpu production kernels — row-tile-amortize-input-load, factor-affine-quant-scale-out-of-element-loop, pin-lane-to-quant-sub-block-coalesce-byte-loads (gemv_q4k_cubecl Q4_K family).
    // Batch 19 (2): riir-gpu production kernels — chain-dispatches-single-sync-point, fuse-residual-via-in-place-output-array (ternary_ffn_fused, gemv_ternary_residual_cubecl).
    // Batch 20 (5): riir-gpu production kernels — cooperative-block-wide-quantize-staging, byte-perm-lut-via-permutation-immediate, dp4a-int8-hardware-dot-product, threadgroup-subset-sum-lut-keyed-by-activation, base-3-trit-packing-5-per-byte (gemv_ternary_cuda_raw + gemv_ternary_cubecl LUT/trit paths).
    // Batch 21 (5): riir-gpu production kernels — cross-runtime-raw-handle-extraction, pool-buffer-aliasing-dedicated-output, same-queue-implicit-ordering-no-sync, per-dispatch-pointer-re-extraction, no-blocking-sync-in-async-pipeline (gemm_ternary_metal_zero_copy.rs — Metal cross-runtime integration cluster).
    // Batch 22 (5): riir-gpu production kernels — dynamic-shared-memory-runtime-sized, ternary-bit-plane-u64-as-2x-u32, warp-shuffle-intrinsics-for-sub-warp-reduction, fused-activation-quantize-for-gemv-consumer, cuda-graph-scalar-via-device-pointer (cudarc_kernels/ — embedding.rs + attention.rs + mod.rs + deltanet.rs; Issue 615 full GPU-resident forward port).
    // Batch 23 (5): riir-gpu production kernels — combined-kv-buffer-keys-then-values, gqa-kv-head-sharing-bandwidth-reduction, softcap-tanh-attention-score, fold-query-tokens-for-occupancy, hardcoded-fast-path-jit-specialization (attention_cubecl/mod.rs — CubeCL flash-attention decode cluster).
    // Batch 24 (4): riir-gpu production kernels — widen-index-multiply-to-64bit, dual-accumulator-single-reduction-sweep, dual-grad-same-pass, broadcast-uniform-skip-reduce (cudarc_kernels/backward.rs — Issue 641 per-layer backward cluster: RMSNorm/SwiGLU/z-gating/output-gate/conv1d+SiLU/L2-norm/beta-decay/softmax-attention backward).
    // Batch 25 (5): riir-gpu production kernels — device-side-argmax-minimal-download, monotone-transform-elision-under-argmax, fan-out-shared-input-single-sync, hot-swap-weights-preserve-autotune-caches, tree-reduction-depth-over-sequential-accumulate (gemma2_cubecl/ + gemma4_cubecl/ — the Gemma cluster: GPU decode loop + dispatch helpers + KV cache + weight hot-swap).
    // Batch 26 (8): riir-gpu production kernels — per-slot-uniform-buffer-write-ordering, derive-params-from-global-invocation-id, merge-dispatches-into-single-compute-pass, batched-downloads-single-sync-cycle, chunk-encoder-submits-below-driver-limit, prebuilt-bind-groups-fixed-buffer-combos, host-cache-immutable-device-weights, defer-readback-until-after-submit (forward/ + buffer.rs — the wgpu forward-orchestration cluster: uniform hazards, pass merging, submission chunking, download batching, bind-group pre-building, host caching, deferred readback).
    // Batch 27 (6): riir-gpu production kernels — staging-packed-uniforms-encoder-ordered-copies, identical-uniforms-share-single-slot, persistent-zeros-source-encoder-copy-clear, fuse-sibling-target-grads-single-kernel, per-layer-strided-grad-destination, snapshot-transient-grad-for-deferred-download (backward/ + backward_dflash/ — the backward-pass orchestration clusters: staging-packed uniform distribution, the idempotent-write exemption, encoder-ordered zero clears, sibling-target grad fusion, consumer-selected grad destinations, transient snapshotting for deferred readback).
    // Batch 29 (10): riir-gpu production kernels — defer-readback-gated-by-live-output-size, weight-handle-cache-keyed-by-allocation-identity, first-readback-pays-queue-drain, epilogue-at-register-write-phase, delay-rms-scale-past-gemv-epilogue, pack-scalar-params-into-combined-buffer, enum-dispatch-separate-kernel-per-variant, epilogue-must-be-workgroup-local, launch-unchecked-caller-owned-size-contract, fast-math-contraction-behind-feature-flag (deltanet_input_proj_fused.rs + epilogue/ — the decode-path fusion cluster: Issue 602 sibling-GEMV fan-out + Plan 106 Track 3 CODA epilogue fusion).
    // Batch 30 (6): riir-gpu production kernels — zero-init-side-branch-frozen-base-path, counter-derived-seed-stateless-noise, predicate-gated-skip-empty-workset-step, share-pipeline-set-across-subsystems, pad-uniform-pod-to-min-binding-size, fuse-count-into-mutation-pass (dllm/mod.rs — the D2F training cluster: Plan 068 discrete-diffusion trainers + RePlaid schedule).
    // Batch 36 (7): riir-gpu production kernels — constant-scale-folded-into-upload-copy, side-branch-operand-residency-matches-mutation-lifetime, activation-checkpoint-as-handle-clone-deferred-batch-readback, training-forward-shed-decode-loop-shape, sibling-dispatches-before-any-readback, independent-cpu-work-fills-dispatch-read-window, grad-terms-merge-before-shared-link-jacobian (gemma2_cubecl_train/resident.rs — the Gemma-2 RESIDENT training cluster: Plan 481/482 all-GPU LoRA forward (per-position + batched) + batched backward; completes the Batch 35 cluster).
    // Batch 37 (8): riir-gpu production kernels — kv-window-device-resident-ring-append, lm-head-loss-positions-only, verbatim-upload-on-format-match, stream-drain-free-source-during-build, identical-tensor-handle-clone-zero-reupload, sparse-gather-scale-at-read-not-table-prepass, tiled-host-transpose-before-quantize, materialized-fast-path-bulk-slice-accessor (gemma4_q4k_train/ mod.rs + weight_buffers.rs — the Q4_K TRAINING cluster: QLoRA frozen-Q4_K-base + f32-LoRA-delta Gemma-4 trainer; forward_batched + backward deferred to Batch 38).
    // Batch 38 (4): env-flag-resolved-once-not-per-dispatch, flat-activation-slots-as-readback-target, transient-spike-pins-persistent-scratch, transposed-consumption-accumulate-not-gather (gemma4_q4k_train/ forward_batched.rs + backward.rs — completes the Q4_K cluster).
    // Batch 39 (9): riir-gpu production kernels — autotune-cache-extract-outlives-owner, append-cache-rewind-overwrite-in-place, slow-path-bootstraps-raw-buffer-hotswap-cache, entrypoint-variants-share-layer-body, elementwise-offload-needs-device-side-anchor, tiny-param-buffer-rebuild-over-cache, readback-destination-passing-free-fn, variant-owned-output-buffer-clone-escape, pipeline-layer-boundary-device-handle (kimi_k3_gpu_forward.rs — the KimiK3 full-model decode forward: variant family + hot-swap + rewind + readback + borrow-escape cluster).
    // Batch 40 (7): riir-gpu production kernels — host-scratch-mirrors-slot-order-for-coalesced-writes, write-plan-derived-from-cache-slots-not-recomputed, cpu-source-builder-skips-device-roundtrip, expert-loop-two-phase-split-batch-across-iterations, rmsnorm-backward-gamma-scope-correction-term, per-position-scalar-factored-gemv-collapse, foreign-accumulator-bridge-copy-hoist-to-outer-loop (kimi_k3_gpu_backward.rs — the KimiK3 GPU backward: handle/update infrastructure + dense SiTU FFN + MoE + MLA + KDA backward compute; 2 critical wrong-math bugs + NaN-vacuous parity suite filed as Issue 693).
    // Batch 41 (6): riir-gpu production kernels — concurrent-disjoint-write-buffer-parallelizes-uploads, aliased-slot-allocate-replace-fallback-severs-binding, pool-slot-write-coalescing-owns-only-declared-ranges, pool-reservation-token-outlives-raw-view, gpu-type-bridge-borrows-not-clones, superseded-primitive-machinery-zombie (weight_buffer_cache.rs + kimi_k3_gpu_backward_sequence.rs — the KimiK3 backward sequence PRODUCTION caller path + the write substrate; Issue 693 H1/H2 production routing pinned + orphaned-write fallback + zombie LM-head machinery filed as Issue 694).
    // Batch 35 (8): riir-gpu production kernels — mask-sentinel-matches-reduction-identity, smem-staging-gated-by-cross-thread-reuse, kernelize-host-chain-after-sync-batching-floor, block-reduction-separate-scratch-runtime-strides, saturating-nonlinearity-recovery-breaks-near-cap, adapter-grad-join-mirrors-forward-delta-insertion-point, side-consumer-readback-alias-live-handle-for-dependent-dispatch, quadratic-attention-intermediates-recompute-from-linear-saves (gemma2_cubecl_train/ — the Gemma-2 training cluster: fused causal attention kernel Issue 430 Option B + hybrid LoRA training fwd/bwd).
    // Batch 42 (4): riir-gpu production kernels — refine-loop-invariant-prefix-elision, softmax-entropy-via-shift-identity, runtime-identity-gate-cached-verdict-elides-side-projection, expected-embedding-transposed-gemv-resident-head (gemma2_d2f/ mod.rs + gemma2_d2f_sc.rs — the Gemma-2 D2F block-causal decode cluster + Self-Conditioning; hazards filed as Issue 695).
    // Batch 43 (5): riir-gpu production kernels — mutable-input-scratch-residency-write-per-call, topk-select-nth-over-full-vocab-sort, post-scatter-renorm-support-set-only, stack-array-row-view-bridge, empty-slice-sentinel-launders-shape-violation (weaver_gpu_corrector.rs + weaver_gpu_dflash.rs — the Weaver speculative-decode GPU corrector cluster; hazards filed as Issue 697).
    // Batch 45 (3): riir-gpu hypernet — scratch-depth-matches-shape-invariance, softmax-denominator-hoist-out-of-weighted-v, frozen-cooperand-hoist-out-of-perturbation-loop (hypernet/meta_lora.rs + train_meta_lora.rs — the SHINE meta-LoRA CPU reference forward + FD trainer; hazards filed as Issue 699).
    // Batch 44 (9): riir-gpu production kernels — row-dot-split-inner-axis-block-reduce, fuse-indirect-gather-into-dot-consumer, share-cube-size-const-between-kernel-and-launcher, stash-kernel-internal-intermediate-in-smem, hoist-bit-address-out-of-packed-row-sweep, fold-tile-transpose-into-dequant-scatter, bit-cursor-unpack-to-width-specialized-byte-batch, single-sweep-minmax-for-quant-scale, serialize-container-canonical-order (weaver_gpu/mod.rs — the 19-dispatch kernel substrate behind the Batch 43 corrector + kvarn/ — the KVarN CPU reference + zone-scale registry; hazards filed as Issue 698).
    // Batch 46 (3): riir-gpu maxsim — size-gate-covers-every-cost-factor, declared-max-array-taxed-per-thread-under-runtime-bound, shared-operand-side-pair-batch-index-decomposition (maxsim/dispatch.rs + spectralquant.rs + both WGSL kernels — the MaxSim size-gated CPU/GPU dispatcher + the fused SpectralQuant dequantize+MaxSim single-pass scorer, Plan 085; hazards filed as Issue 700).
    // Batch 47 (3): riir-gpu hypernet — per-key-projection-hoist-out-of-query-sweep, multi-head-out-proj-consumes-head-concat, shared-producer-prefix-extract-once-per-input (hypernet/ shine.rs + routing.rs + types.rs + m2p_transformer.rs + gpu_m2p.rs + kernels/m2p_attention.wgsl — the SHINE second half: the context-to-LoRA pipeline + expert routing + the M2P CPU/GPU attention pair; hazards filed as Issue 701).
    // Batch 49 (1): riir-gpu cudarc — runtime-arg-loop-bound-demotes-arrays-to-local-memory (cudarc_kernels/mod.rs rmsnorm_quantize_f32, the Bench-686 measured lesson from Issue 706: compile-time loop bounds + #pragma unroll keep per-thread arrays in registers — kernel −18.4% at ppl bit-exact).
    // Batch 57 (2): measured-GOAT distillation (riir-ai Issue 715 — smem-slot-role-reversal-needs-barrier, tile-loop-back-edge-needs-multi-tile-bitexact-gate; the flash-attention tile-loop smem races + the multi-tile bit-exact regression gate; commits 9f9e68402 + 93d853582).
    // Batch 58 (8): ternary_deltanet sibling-WIP unlock — the LAST never-mined riir-gpu orchestration file (ternary_deltanet_gpu_forward_cudarc.rs, 5,691 lines — raw-CUDA decode-forward twin; async-htod-host-source-outlives-copy, captured-graph-bakes-buffer-addresses, output-buffer-rotation-pool-multi-forward, mid-pipeline-device-alloc-is-implicit-sync, capture-region-dispatch-only, occupancy-grid-cap-refuted-uncapped-default, write-before-read-state-skips-checkpoint, same-input-gemv-segments-single-launch; hazards filed as Issue 722).
    // Batch 60 (1): measured-GOAT distillation (riir-train Issue 460 / katgpt-rs e2853fbf — pre-gate-recovery-by-division-cancels-algebraically; the MLA backward's saturated output-gate 0/0 NaN, fixed by algebraic cancellation; Bench 475: 6/10 -> 10/10 usable backward passes).
    // Batch 63 (3): the CubeCL twin unlock — same-input-gemv-weight-concat-at-upload, deferred-residual-fused-into-next-layer-norm, reset-state-in-place-fill-never-realloc (ternary_deltanet_gpu_forward.rs, Plan 009 / Research 071).
    // Batch 70 (3): the arxiv barrier-free-sync paper distill (arXiv:2608.13757, Plan 020 / Research 080) — barrier-free-per-dependency-wait-from-loop-counters, shared-counter-decode-producer-progress, masked-branchless-wait-collapse.
    // Batch 105 (11): uzu + lily combined Metal-engine distill (Research 127 + 128 / Plan 075) -- concurrent-dispatch-dependency-level-barriers, gpu-resident-moe-routing-no-host-sync, indirect-device-dispatch-tile-map-moe, pipelined-token-slot-ping-pong-submit-ahead, split-k-two-pass-attention-decode-long-context, cooperative-tensor-mma-capability-route, rht-epilogue-fusion-contract, lane-sequenced-deterministic-bucket-scatter, a8-activation-route-stability-guard, target-tile-split-k-deterministic-two-dispatch, hazard-tracked-aliasing-scratch-allocator (A +1, C +1, D +3, E +1, F +1, H +3, M +1; sources trymirai/uzu @ 115609b + perplexityai/pplx-garden lily @ 1ed972ed, both lanes re-verified every quote; the cross-source dedup mandate executed per rule; the R128 tile-by-expert-load rider FALSIFIED at pin -- chunk-size-routed table, both tiles 4 simdgroups).
    // Total: 209. // (historical batch-30 snapshot; current 539)
    // Batch 98 (10): antirez/ds4 dense.metal/moe.metal leads -- grid-range-split-verbatim-impl-composition, share-activation-stream-keep-projection-reductions, tiny-rows-scalar-thread-per-row-downshift, small-batch-matvec-gemm-bridge-rung, moe-routed-down-sum-single-accumulate-pass, fused-vector-site-transcendental-form-nonportable, monomorphic-hot-caller-last-hit-memo, per-row-scale-folded-into-rhs-staging-tile, dequant-byte-pairs-as-aligned-u16-loads, cross-simd-bitonic-alternating-bank-exchange (the Batch-97 queued NEXT BATCH LEADS + 3 additional candidates).
    // Batch 100 (8): Layr-Labs/qwen-3.8-mtp-challenge -- functional-cache-versioning-loses-donation-under-kernel-read, fused-kv-write-single-writer-reads-newest-from-input, in-place-write-through-input-emits-fence-consumer, residency-set-wired-once-post-warm-persistent-footprint, register-budget-head-split-gqa-reread-trade, batch-composition-invariant-grid-early-return, never-gqa-multiply-full-d-merge-smem, container-rebind-over-retrace-for-churn-survival (Research 120 / Plan 068; A +3, D +3, E +2).
    // Batch 102 (6): FlashQuant sparse-dense fusion (arXiv 2608.15531) -- heterogeneous-unit-paths-shared-tile-fusion, lpt-min-heap-disjoint-column-ownership, tile-coo-sparse-tile-aligned-format, bucketed-sparse-reader-column-partition, bank-group-circular-element-reorder, stream-k-persistent-cta-mac-segments (Research 123 / Plan 071; A +2, C +2, H +1, N +1; the headline was REFUTED for direct adoption at walk time -- the fusion mechanism is the distilled core).
    // Batch 103 (5): MTPLX PR #391 + MLX NAX combined host-orchestration batch -- shape-class-keyed-capability, wavefront-overlap-pair-ceiling, request-shape-decline-mirror-completeness, backing-provenance-causal-shadowed-extent-widen (Research 124 / Plan 072; D +4; MTPLX re-pinned ef96313 -> a5e38bb at mine time, all anchors re-verified; + the B100 rider enrichment in entries_gpu_14.rs) + dispatcher-eligibility-predicate-lags-kernel-capability (the ADDENDUM rule -- the MLX #4416 lead the sibling landing silently omitted; D +1; see the Research 124 addendum note).
    // Batch 104 (2): ggml-org/llama.cpp fa-vec offline swept launch-table distill -- swept-shape-bucket-launch-tables, no-harm-bucket-representative (Research 126 / Plan 074; K +2; infra efeda76b9, table pin 8963a9bdc the M3 Max rows, harness read at d230ddd76 byte-identical pin->HEAD; MIT; the league-relevant SKU).
    // Batch 106 (5): MTPLX upstream residue distill (mistakes/ post-mortem corpus + docs + targeted kernel diff; Research 129 / Plan 076) -- prefix-reuse-key-must-mirror-the-restore-predicate, request-arrival-work-asks-reuse-state-first, serving-raises-only-behind-strict-flag, derived-key-single-constructor-parity-test, vendor-fused-op-instantiation-check (D +5; source youssofal/MTPLX @ dd4031a8 upstream main, Apache-2.0; the B+ lead CONFIRMED with both instances in-source -- the sentinel fingerprint post-mortem + the dd4031a8 fix commit).
    // Batch 107 (8): uzu + lily defers batch (Research 130 / Plan 077) -- sparse-demand-paged-kv-cache, tensor-op-strided-device-operand-binding (A +2), gumbel-argmax-filtered-sampling-descent (C +1), prefill-chunk-command-buffer-capacity-coupled (D +1), packed-total-order-multi-field-argmax, radix-select-ordered-key-topk (E +2), single-threadgroup-fused-count-scan (H +1), integer-mma-span-epilogue-dequant (N +1); sources trymirai/uzu @ 115609b (MIT) + perplexityai/pplx-garden lily @ 1ed972ed (Apache-2.0); 12 defers re-verified, 3 dropped (flat-topology = B23-family, RIF-sentinel = born-dead arms, weight-hygiene = B63/B31 instances) + 1 collapsed to the B63 rider; upstream E>128 hazards in counts_offsets_fused recorded in-rule.
    // Batch 108 (4): riir-ai M3 campaign measured-GOAT distill (Plan 078) -- smem-envelope-gate-kernel-variant (G +1), multi-simdgroup-shared-staged-tile (A +1), independent-accumulator-chains-hide-load-latency-at-capped-occupancy, byte-cut-loses-when-plateau-is-issue-bound (E +2, the last refutation-type); source riir-ai @ 59a172623 (committed state; the first in-source batch since Batch 63 -- the M3 prefill/decode campaign mined: Issues 768/771/779/831/843, numbers belong to Benches 774/790/804/831 and are cited as records); two verification lanes at the pin + coordinator spot-check (all VERIFIED); +3 riders on B01/B06/B57 + 2 arxiv riders discharged on B79 (2608.17641 breadth) and B82 (2608.01651 confirmation -- the land-with-next-adjacent-batch queue); recorded-not-landed: axis-swap refutation (S2:40-42), 4-arm ladder (B104 ethos), empty-partial identity (B35 grain).
    // Batch 110 (11): vLLM libtorch_stable distill (Research 133 / Plan 081) -- spin-barrier-needs-co-residency-capped-launch, ballot-matrix-tie-rank, m-group-shared-weight-scan-register-relief, row-parallelism-ladder-split-vs-pack (E +4), captured-graph-init-unconditional-and-stream-ordered, batch-invariance-locks-reduction-geometry (D +2), rope-pair-partner-via-cross-lane-shuffle, degenerate-selection-falls-back-to-identity-uniform (B +2), softmax-denominator-cancels-through-scale-invariant-norm (C +1), persistent-kernel-length-bucketed-mechanism-ladder (K +1), router-math-stays-fp32 (N +1); source vllm-project/vllm csrc/libtorch_stable @ 8277c42e4 (Apache-2.0, never mined); two inspection lanes + coordinator byte-verification of 13 lead quotes; the VERIFIED A-/B+ tier of a 25-candidate pool (the B-/C+ remainder = Batch 111); upstream hazard ledger in Research 133 (H1 silent tie-buffer overflow, H2 dispatch-no-default, H3 cross-tier router divergence).
    // Batch 111 (11): vLLM libtorch_stable remainder + rider sweep (Research 133 / Plan 082) -- register-rotation-prefetch-loop, shared-operand-groups-per-warp, whole-row-register-resident-under-length-budget (A +3), grid-dependency-wait-scoped-to-dependent-operands, refine-by-rescan-prefix-filter-vs-buffered-ties (D +2), warp-aggregated-atomic-scatter, routing-by-precomputed-lookup (H +2), launch-heuristic-pinned-to-calibration-arch (K +1), lane-owned-topk-output (M +1), ue8m0-pow2-scale-as-biased-exponent-byte, coarse-precision-first-exact-refine (N +2); source re-cloned + re-pinned at 8277c42e4 at mine time (origin had moved to 16328c7a minutes before); lanes: rules #12-#20/#22/#25 LAND, #21 (noinline) + #23 (clamp-before-collective) KILLED as COVERAGE (B110 rules already carry them, same citations), #24 demoted to a RIDER on device-m-runtime-mblock-graph-stable-gemm; + 20 riders across 13 files (19 kernel_opt + weight-tile-consumption-order in rust_perf), incl. the hi/lo packed-u64 redux (B107), phase-counted barrier (B92 stripe), B57 union-across-phase + filler-thread barrier matching, stable-softplus third form (B98), loud-refusal third instance (B103), PDL capability gating (B105), measurement-revision + family-gated table (B104), and the H1 fit-guard contract encoded as refine-by-rescan-prefix-filter-vs-buffered-ties.
    // Batch 114 (2): oMLX ANE-prefill + GDN blocked-sequential distill (Research 138 / Plan 087) -- recurrence-prefill-exact-restructure-before-reformulation (G +1, the A- GOAT lead: restructure the EXACT sequential recurrence's memory layout -- TB-block smem staging + register-resident state -- BEFORE an algebraic reformulation; stock per-slice k/q re-read = 32x redundant traffic (~13 GB/16k-token layer), ~2x at 16K on M3 ULTRA source-reported; the WY/FLA chunked reformulation measured a LOSER 3x independent + our Bench 734/769 negatives 0.97x/0.98x), ane-bank-split-ladder (D +1: near-half-first split ladder for a monolithic accelerator bank that cannot map at program-create (0x20004; our 0x50004 cascade the same class); the driver's release is ASYNCHRONOUS -> drop + gc + settle BEFORE the headroom re-measure; 70% phys-footprint jetsam gate -- 4 bounded attempts still hit 48.3 GB on 48 GB (source-measured); bank-count = DETERMINISM axis; fail-open snapshot; feeds riir-ai 876/769 T2/R1/R4a-R7), + 6 riders (B84 token-local recurrent-safe fraction cap; B84 pad-tail profitability + single-prompt guard; B22 sub-simdgroup cooperation radius -- the FULL-STRING pin regenerated; B95 whole-sequence prefill-grain instance; B107 program-SHAPE vs delivered-chunk-WIDTH coupling; B79 scalar-staging pad boundary). Source jundot/omlx @ e467261 (Apache-2.0).
    // Batch 115 (6): qwen-3.8-mtp-challenge submission-diff-history walk (Research 139 / Plan 088) -- ordinal-map-exact-tail-selection (D +1, the strongest intake: replace a reference sort's tail-K EXACTLY by order-encoding the reference's TOTAL order -- stable ties toward the HIGHER index, NaN above all, -0.0 folded -- into a monotone u32 key; then any integer selection reproduces the reference bit-for-bit; same-set + tie-forcing + positive-control gates; 14-dispatch full-argsort stub -> 2), complete-coverage-scatter-collapses-operand-to-dense (D +1: island-row COVERAGE decides dead compute -- a complete-permutation scatter means every packed row is overwritten unread -> one dense matmul + an install-time argSort-inverse reorder; partial keeps the scatter; lazy eligibility when install ordering makes the replaced gate unknowable; the same lens bounds PRUNING -- the measured 49,152 trim REGRESSED accept 1.00 -> 0.877 below committed argmax support), exhaustive-domain-input-patch-bit-identity (N +1: a 16-bit input domain is exhaustively sweepable -- 65,280 finite values -- so patch ONLY the divergent inputs via direct output-word mapping before the lossless expansion, exhaustive zero-mismatch gate + boundary negative control; f32 unsweepable -> tolerance stays), derived-index-from-pinned-checkpoint-is-a-cache (M +1: a warm-time index from pinned tensors alone is CACHE-class -- legality boundary + canonical order (runtime-derived == offline-screened) + fail-loud over silent fallback + one attempt forever; balanced bisection makes empty clusters impossible), warm-reachable-dispatch-signatures-before-timed-window (M +1: JIT backends compile per signature (shape/dtype/input-count/host-device-mix) and an ADAPTIVE schedule reaches a SET -- warm the reachable set with throwaway token-neutral state, the scored EXPRESSION not a call prefix, extend with receipt when the schedule changes), env-gate-reaches-and-arms-the-scored-process (D +1, measurement-honesty #2: sanitizer-allowlist spelling (a gate outside the allowlist runs both arms on the same code), force-set injected defaults into first-use-cached knobs (ambient stock defeats no-overwrite), durable arm witness not stderr, bare-leg no-env receipt; source Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06ac (Apache-2.0; B100 read the tip TREE, this batch read the ~50-diff HISTORY -- complementary); + 8 riders: B63 boundary-fused residual pair (external confirmation + eval-ladder-both-halves), B96 activation-side digit-fold two-posture exactness, B97 single-rounding-boundary fusion, B38 unconsumed-variant factory skip, B107 workgroup radix + ordinal bitmap tie-cut + bf16 16-bit exact key, B51 quantized-embed dequant-at-use, B82 shapeless-trace Slice-fatal, B77 derived-sum-table member). **ADDENDUM (collision salvage, 2026-09-06):** a parallel session's duplicate Batch 115 (4 rules incl. two shared seeds) yielded per push-wins; the salvage the landed batch lacks landed as 4 rules (dispatch-cliff-bitexact-shape-confinement D, readout-trim-needs-committed-id-coverage D, dead-output-rows-state-append-only-path D, distinct-index-bitmap-ascending-compaction E) + 6 riders (B96 bit-plane in-consumer offset-sum placement -- corrects this batch's claim that the arm was already covered, B97 verbatim-body text-identity construction, B104 x2 DIRECT_NIBBLES + register-cliff / worst-domain 2.3e-4 semantic pruning, B100 host-side grid-trim witness, exhaustive-domain-input-patch += the per-OUTPUT fidelity-exclusion half); batch115_addendum gates 3.
    // Batch 116 (6): PrismML fork ternary + verify-batch distill (Research 140 / Plan 089) -- activation-bitplane-popcount-word-dot (N +1: activation int8 bit-planes consumed by AND+popcount -- 32 weights per instruction pair ~0.8 ops/weight/col vs ~3 for select-per-weight; the fork's own ablation removed ONLY the bit test and ran 1.7x faster = extraction-bound diagnosis; LOSSY, ships default-off upstream, behavior-axis gated), scratch-sizing-shares-dispatch-eligibility-predicate (D +1: tail-carved scratch -- the allocator's size helper and the encoder are two statements of one path predicate; ONE predicate both consume, drift is benign-waste one way and OOB the other), byte-owning-threads-non-positional-digit-codes (A +1: non-positional multi-digit packings must map threads to whole BYTES -- element-span ownership reloads every packed byte per digit index, 29.8 -> 136.4 tok/s source-reported; the fork's own negative-results ledger rides in-rule: contiguous byte ownership WORSE, 'fewer ops beat more ILP'), fwht-wide-block-row-per-threadgroup-exchange-ladder (E +1: past the N/32 register wall -- one row per THREADGROUP, three exchange tiers by partner distance, same host kernel names, dispatch-side threadgroup-max assert; fills the register-resident butterfly rule's named beyond-budget arm), shader-shape-cutover-keys-on-resource-block-ratio (D +1: subgroup/registers vs staged shape cutover must be a TWO-TERM disjunction n > ABS_MAX || n/subgroup_size > MAX_EL_PER_LANE -- width alone admits the register wall on narrow subgroups; creation-time resource guards + recorded specialization geometry + null-pipeline fallback), same-input-folded-transform-memo-per-build (D +1: graph-build CSE of folded transform nodes keyed (input, rotation), one-build lifetime, key-completeness invariant source-asserted-by-comment -- the sharpest port edge; 36% duplicate transforms, +3.5% pp512 source-reported) + 3 riders (telescoped-digit += radix-general mantissa-bound exactness + single-trit two-floor form + float-pipe motivation, bridge-rung += register-budget sizing + exact-fit nr1 + per-type mv/mm crossover, dispatcher-eligibility += four inverse-direction instances in one PR window incl. head-truncated-grep transcription); source PrismML-Eng/llama.cpp origin/prism @ 7113f6462 (Apache-2.0, 12 commits past the league pin e311ed38f; quotes via git show, vendor checkout untouched).
    // Batch 117 (5): antirez/ds4 Metal tensor-prefill delta distill (Plan 090 / Research 141) -- routed-activation-pack-once-shared-operand (L +1: pack the scattered routed activations ONCE per stage into a dense converted device tensor -- all consumers read it plain; f32->half exactly once at the pack; bounded grow-only scratch + derived 256 MiB refusal; bit-exact by conversion-site-only movement; +1.7-3.2% source-reported, MXFP4 arm non-uniform and said so), device-affine-scratch-admission-with-pool-fallback (A +1: three-check admission tuple ptr^device^capacity with the ladder RETAINED -- device affinity captured at SET time is the multi-GPU illegal-access guard; pool-alloc-inside-capture is the motivation (the remove-the-alloc complement of captured-graph-bakes-buffer-addresses); best-effort 96 MiB provisioning; +7.5/6.5/6.4% steady decode source-reported), batch-verify-rows-single-dispatch (C +1: the 2-6-row verify block in ONE launch, partials straight into the TP exchange slab, combine epilogue hoisted and byte-identical for both schedules -- lockstep by construction; +11-23% whole-commit conflated with the seed schedule, said so), row-aware-indexing-reuses-static-single-token-kernel (M +1: widen the existing static kernel's grid mapping (z -> token,slot) + per-token bases and call the static impl VERBATIM rebased -- reuse-instead-of-twin, the static pole of the B92 axis), dense-fast-path-bounded-by-selector-coverage (F +1: dense substitution is legal exactly through top_k + anchor_pool - 1 -- the selector's own reach, never the work-window; converts a tolerance into a proof while enabling the bounded GEMM; ~21 -> ~68 tok/s companion enablement source-reported) + 8 riders (B97 duplicate-pad-at-materialization complement, B35 TP non-owner identity + selector-padding sentinel, B23 byte-size static pin + topology-axis widening, B57 single-pass two-phase twins + four-backend audit, B58 remove-the-alloc-from-capture complement, B103 clamp-to-stay-on-bounded-gemm); source antirez/ds4 main @ 9ab7053 (MIT; Batch 97/98 pin 110afdd + ~14 commits; two read lanes + coordinator spot-verification, line drifts corrected).
    // Batch 118 (6): antirez/ds4 PR #991 Qwen3.8-Flash-Next distill (Research 142 / Plan 097) -- role-scoped-store-must-match-compute-guard (A, the correctness lead: unguarded store on a role-colliding row_half -> EXACTLY-UNIFORM softmax, invisible four ways, SIX sessions; gate batch118_unguarded_store_yields_uniform_softmax), tile-amplification-precedes-staging-pass (H, converged by both lanes), operand-precision-gated-by-consumer-invariance (N, GOAT-grade pass/fail arms + Q-split null ablation; closure MODEL-SCOPED), compute-role-split-by-accumulator-pressure (A, 2 score + 4 PV simdgroups), speculation-scheduler-break-even-flip (D, +3.8% plain decode -> 202 bypass vs 62; 110% margin doc-only), running-threshold-prune-compaction-aliasing-guard (I, memcmp == 0 survives; in-place compaction aliasing guard) + parallel-disjoint-pread-hides-cold-page-latency (rust_perf io) + 7 riders (B24/B95/B77/B57/stripe-semaphore/block-reduction gate batch118_indexed_shuffle_ladders_must_read_unmutated_partials/smem-envelope); source ivanfioravanti/ds4-metal branch qwen3.8-flash-next @ 236cb2a5 (MIT; force-pushed once, HEAD re-verified == pin).
    // Batch 119 (2): ss16118/low-latency-nccl near-SoL collectives distill (Research 143 / Plan 098) -- sentinel-readiness-over-separate-flag + ll-flag-in-data-packed-poll (E +2; the DEVICE-side peer-poll half of barrier elimination: dtype-poison init + poll-non-sentinel with reset-before-reuse, vs the 16B {flag,data} packed poll with the 2..=255 epoch band; the LL128-atomic determinism caveat carried in-rule as decision text; rider: B58 += the bidirectional peer-exchange credit face); source ss16118/low-latency-nccl @ 5357eff3 (BSD-3 NVIDIA; the repo names the sentinel POISON -- ncclLLPoisonBuffer(Async) fills, IsPoisoned polls, the receive Reset arm re-poisons; LL128 enumerated NOT-YET-IMPLEMENTED so the caveat cites the paper) + arXiv:2607.16100 (the measured basis: barriers ~= 40% of a small-message AllReduce -- their silicon, never ours).
    // Batch 170 (7): mlxfast-gemma4 successor-arena pack (Research 198 / Plan 162) -- softmax-stats-precompute-exp-in-gemm-loader (C +1, at1: P never materialized, per-row stats 8 B/row, exp regenerated in the P*V GEMM's A-loader bit-identically, 4->2 passes over the score rectangle), fixed-ref-softmax-decode-walk (F +1, SLIDE-SOFTREF: prologue-pinned self-score reference replaces online max/rescale/branch, bounded divergence + span-0 bit-exact control), scale-identity-elide-in-framework-graph-transcription (B +1, QSCALE-ELIDE: delete the scale==1.0 multiply when transcribing the framework fallback graph -- 232 dispatches / 1.11e9 elements / ~4.4 GB of identity per prefill step), sentinel-operand-encoding-routes-fused-kernel (D +1, CAUSAL-CLOAD: dispatch decision encoded in an operand's shape signature -- stride kL+1 synthesis + the ldc==4/beta=-0.0 carry -- with graceful degradation on unpatched arms), tier-mismatched-grid-pruning-relaunch (E +1, LMH-001: 196608 of 262144 launched threadgroups exist only to early-return; relaunch the SAME kernel at the tier's claimed extent, bit-identical), compile-out-rare-paths-register-worst-case (E +1, RUN-CAP: register allocation is worst-case across inlined paths; cap the ladder at the measured run-length distribution, +9.12% s/token tokens bit-identical), batch-width-aware-partition-shrink (K +1, PARTITION: stock 1-row partition heuristic over-partitions the 8-wide batch ~8x; min(batch, stock) + the 1 MB-in-flight DRAM argument + merge-side simdgroup packing); source Layr-Labs/mlxfast-gemma4-26b-a4b-engine @ 27c821c4 (MIT; all 20 quote sites re-verified at the pin at authoring time) + 9 riders (B63-adjacent MMA bias-as-K-slot-64; B98 quad u32 load arm; B25-family LGH plane elision; B115-family teacher-forced-replay instrument; gate-up strided-view hazard; B100 write-fence graph spelling; B111-register-pipeline prefetch phase-multiplex negative; row-packing kL-keyed rows-per-TG table; B63 pointer-chain scatter-hoist store-side twin).
    // Batch 177 (4): EXL3/GB10 serving-recipe distill (Research 207 / Plan 170) -- quantized-pack-residency-audit-unquantized-minority-first, bpw-reduction-buys-memory-not-decode-when-dequant-bound (N +2: the (class,dtype) residency audit with the contracted-dim-constant scale placement, + the bpw-axis two-regime rule), host-gather-splits-the-graph-and-the-split-is-the-separable-price, unified-memory-page-cache-is-device-allocatable (D +2: the PIECEWISE separable-price attribution rule + the unified-memory page-cache-as-device-RAM rule) + the readout-trim scope rider (draft-only trim with full verify = acceptance loss, never correctness); source vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe @ e0ebee8d (MIT; executed AHEAD of the event-blocked B176 FWHT rider; the T2 re-verification caught factor-affine-quant-scale-out-of-element-loop as #2's loop-stage half, so #2 landed as ONE rule).
    // Batch 194 (14): DeepGEMM-Ascend distill (Research 226 / Plan 191) -- fp4-nibble-gather-lut-replicated-index-width (N +1: FP4->FP8 via ONE gather against a 16-entry LUT replicated x16 across the byte-index width so the low-nibble gather takes the RAW byte with no pre-mask; magnitude remap + the sign-bit move baked into the table build; fp8_dequant_gemm.hpp:24-25,73-76), decode-sidecar-coroutine-writes-consumer-stage-ring (A +1: a decode coroutine converts packed FP4 in unified-buffer scratch and writes DECODED bytes into the consumer's own L1 stage ring — per-stage cross-coroutine-class events, no separate decoded buffer; dual-decoder widening gated on the direct-store epilogue; fp8_dequant_gemm.hpp:111,406-412,209-210), ring-overwrite-gated-k-back-readiness (H +1: GM ring slots recycled at fixed distance K — overwrite slot i only after the consumer of slot i-K published readiness; the ring depth IS the safety margin; mega_moe.hpp:606-610,778-783), empty-participant-progress-credit (H +1: fixed-arity readiness counters require every participant to publish completion even with ZERO work — skew cannot deadlock the wait; mega_moe.hpp:770-772, comm/mega_moe.hpp:266-268), issue-latency-priced-mad-cost-model (K +1: an analytical tile cost model must price ISSUE latency per MAD — BF16 64 / FP4 40 / FP8 116 cycles measured — plus the L0-ring turnover term and post-pipeline K-tile re-selection; heuristics.hpp:566-577,653-677), multiwave-dual-decode-admission-gate (D +1: enabling a two-decoder worker-split is gated by a 10-conjunct admission — multiwave only, decode rate <= half MAD rate, sector alignment, amortization bound; heuristics.hpp:691-702), work-size-gated-overlap-reduce-mode (H +1: overlap-drain vs final-drain plans switch at a MEASURED 32 MiB combine-work threshold; the overlap machinery is pure overhead below it; mega_moe.hpp:1137-1140,1082-1088,1171-1172), l1-resident-operand-reuse-address-identity-rewind (A +1, the Research-226 A- GOAT lead: skip the GM->L1 load when a.addr == last_a under two-conjunct admission + ring-residency; rewind the stage ring and replay the resident stages; fp8_gemm.hpp:123-125,139-149), decoupled-sf-sidecar-pipeline-own-granularity (A +1: the scale-factor stream rides its OWN ring at its OWN granularity — chunked loads, own event base, wait deferred to the first consumer tile; reused chunks rewind one back-step mod the SF ring; fp8_dequant_gemm.hpp:266-281, fp8_gemm.hpp:246-249), coprime-stride-cross-rank-source-interleave (H +1: N consumers drain one concatenated multi-segment queue with a stride co-prime to the segment total — sync-free segment spread, assignment recorded to metadata; mega_moe.hpp:756-757,801-807), paged-page-pair-deferred-permutation-bitmask (F +1: paged-KV pages gathered in PHYSICAL order, logical-order restore deferred to store time as a precomputed per-pair bitmask — page 128 XORs the dst vector, page 64 half-swaps; mqa_logits.hpp:82-84,593-599,91-96), dual-consumer-alternating-stage-ownership-scale-lane (E +1: a sidecar stream split across two coroutines by STAGE PARITY — both run the cursor, only the parity-owner transfers; mqa_logits.hpp:318-322,488-492,249), dependency-wave-task-picker-one-wave-lookahead (H +1: a persistent-core picker over dependent GEMM stages interleaves waves with a closed-form one-wave lookahead bound; scheduler/mega_moe.hpp:93-105), in-order-pipe-credit-release-completion-collapse (A +1: on in-order pipes, credits publish on the pipe that both produced and consumed the buffer, and one completion covers all earlier same-queue copies — events scale with pipes touched, not copies; mqa_logits.hpp:617-620, mega_moe.hpp:559-561,594-596) + 4 riders (l1-resident admission-gate text folded at authoring; sf-sidecar rewind facet folded at authoring; weight-stream-non-temporal-loads += the reuse-count-keyed hint instances on a third backend; ue8m0-pow2-scale += the branchless significand-carry ceil arm); source deepseek-ai/DeepGEMM-Ascend @ 8491bbb4b8c02a094a2318965f50c70438a3e73c (MIT; heuristics.hpp read in full by the coordinator, slices A+B by 2 read subagents, coordinator re-verified the Tier-1 quote sites at the pin).
    // Batch 195 (9): the mlxfast-bonsai2 POST-REORG wave-2 distill (Plan 194, vendor delta 0c61273..0c7c392) -- io32-count-pinned-text-narrowing (O +1: swap size_t index math to uint at the kernel-source-text level, count-pinned per kernel + launch-time bounds guards, stock-64-bit fallback on drift), strided-view-offset-fit32-launch-proof (O +1: the general member — prove every element offset of an arbitrary strided view below 2^31 by the 64-bit span sum, dispatch 32-bit only then), write-only-touch-side-queue (D +1: provably write-only touches commit on a dedicated second queue + serial side thread — never interleaved between prompt CBs, never stalling the engine thread on first-touch wiring; bounded drain at the emit boundary), last-query-rect-projection (B +1: when the last-query prework provably consumes only the chunk's newest rows, the q|gate projection computes those rows only via a rect-partitioned packed matmul — every element the full product's, bit-checked at prepare), roundfuse-packed-embedding-gather (C +1: three packed-table row gathers fuse to ONE launch under a pure-copy contract, per-geometry verdict cache + bitwise self-test incl. negative-id wrap), enumerable-domain-bit-identity-admission-gate (N +1: admit-or-refuse a rewrite by exhaustive enumeration of its whole 16-bit input domain — never sampling, load-time, binary verdict), cross-queue-output-retention-until-completion (E +1: side-queue write outputs retained until their CB's completion event — allocator hazard tracking does not span unordered queues), atexit-registration-order-shutdown (D +1: teardown by registration order — the cleanup hook registered AFTER the resources whose destructors must run after it, shared-lock handoff refusal, bounded final wait), lazy-admission-reservation-atomic-register (D +1: lazy-allocation admission = per-row reservations at max(actual, estimate) + capacity check and registration in one critical section — the TOCTOU fix; adoption reserves the exact-or-maximal span before the staging reservation drops) + 3 riders (kv-first-append += the adopt/install contract + ready gate; dtype-native-source-read += the Metal BF16 head-rotation pad-in-read instance; rust_perf prompt-continuation += the draft-vocab-cutoff honesty law); source Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c392 (MIT; delta 0c61273..0c7c392 = 25 commits, 22 files, +9150/-777; the standing vendor clone; 2 staging subagents over disjoint slices, every load-bearing quote delta-freshness-checked — the GPUWake staging lead was killed as a B193 duplicate by exactly that check).
    // Batch 196 (2): the TensorFold v0.6.0 A− pack (Plan 195, pin c4646171, Apache-2.0 — license flipped MIT→Apache-2.0 at 0.6.0) -- affine-block-address-lets-pipeliner-fly (L +1: the stored nvfp4 macro-block address `(b // 4) * 32 * SBN + ((b % 4) * 16) // 2 * SBN` is a STAIRCASE in the loop step and the same address is `b * (8 * SBN)` — make the ADVANCING loop address affine so the hardware pipeliner puts the weight traffic in flight, the step's actual wait; nvfp4.py:312 VERBATIM + commit 443ad4a at-sha), register-width-topk-falls-back-to-tiled-radix-select (D +1: _select holds next_pow2(blocks) scores in registers up to SELECT_REGS = 32768 and SPILLS past — dispatch _select_tiles past the cliff, THE SAME LISTS (radix select one byte a pass from the top, ties by lowest block id, in block order); attention.py:13/:353/:357 VERBATIM + commit a2dba9c; every timing figure commit-message-only) + 1 rider (dtype-native-source-read (B176) += the lane_fuse g32 admission instance — the admission hardcoded `!= 64` while the kernels took (32, 64), g32 checkpoints silently lost fused projections; 50588fbb).
    // Batch 197 (6): the ml-explore/mlx VJP kernel distill (Research 227 / Plan 193; PRs #4563+#4565, merges 83b976ea438466418dea20c89709601201851fe1 + 9295197533d586dc26d6814b92899fa088320ab3, MIT, merged 2026-09-29; every quote verified at the pins — .research/230, 22/22) -- sdpa-backward-as-tiled-matmul-bounded-materialization (E +1, the A− GOAT lead: the attention VJP COMPOSED from tiled library GEMMs through ONE bounded score tile (1024² default, MLX_SDPA_VJP_BLOCK) beats the full-matrix fallback AND the fused flash-style kernel on unified memory — the fused attempt CLOSED UNMERGED as PR #3241, its own text conceding 0.36–0.70× dense; riders folded: save_lse forward output (function_constant 303), the NaN dead-row guard (exp(-inf − −inf)), the causal tile skip with diag_off = kL − qL, the HasTail compile-time segment tag, the dummy-binding fc-gated outputs), segmented-checkpoint-cheap-replay-backward (G +1, A−: checkpoint the ENTRY state every Ckpt-th step (dense cache at T=4096/B=4/Hv=32 = 34 GB in-source) and replay the state recurrence in the backward — delta cached per chunk so the replay is ~a tenth of a chunk's work; the entry-state-not-exit clause pinned by test; lead 6's register seg[Ckpt][n_per_t] arm FOLDED as the second posture), gqa-zero-stride-batch-broadcast (A +1, B+: k_bs innermost stride 0 broadcasts one kv head over its query group through the batched GEMM — no expanded tensor; the GQA-operand family's stride-0 member beside B182's row-fold), threadgroup-scope-reduction-single-writer (E +1, B: reduce_tile_tg folds the kNSG simdgroup partials in threadgroup memory and commits with a single non-atomic store — the atomic paid only on the GQA axis that genuinely crosses threadgroups), cache-symmetric-intermediates-per-chunk (G +1, B: the forward-save pass caches the expensive chunk-local set (KKt, the 15-step Neumann T-inv, QKt_raw, delta) with the dv_idx == 0 single-writer guard — the cache-what's-expensive half of the hierarchy beside #2's recompute-what's-cheap), chain-rule-postprocess-kernel (G +1, C+: the kernel differentiates the TRANSFORMED variable (dgamma, 'computes the gradient with respect to the log') and gated_delta_dgamma_to_dg applies the Jacobian in an O(T) reverse suffix cumsum with the max(g, 1e-6) floor) + 1 rider (dot-macro-family-widens-at-load-f32-accumulate (B192) += the OPERAND-TYPED MACRO instance — the macro-side pole: 'Operand and accumulator types are taken from the tiles rather than pinned to float' + the C-stays-float accumulator clause, gated_delta_nax_ops.h:399-404,576-579); source ml-explore/mlx @ the two merge shas (MIT; T0 re-clone at full shas, 22/22 quotes verified — the 34 GB comment + entry-state clause live in the FORWARD file, a provenance sharpening over the research draft).
    assert_eq!(
        corpus.entries.len(),
        734,
        "kernel batches through Batch 197 should have 734 rules",
    );
}

#[test]
fn seed_corpus_rule_ids_are_unique() {
    let corpus = build_kernel_seed_corpus();
    let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
}

#[test]
fn tamper_detected() {
    let corpus = build_kernel_seed_corpus();
    let mut tampered = corpus.clone();
    tampered.entries[0].opt_pattern = "// tampered".to_string();
    assert!(!tampered.verify());
}

#[test]
fn heal_entry_accessors_return_correct_fields() {
    let corpus = build_kernel_seed_corpus();
    let entry = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "matvec-coalesce")
        .expect("matvec-coalesce should be in seed corpus");
    assert_eq!(entry.rule_id(), "matvec-coalesce");
    assert!(!entry.buggy_code().is_empty());
    assert!(!entry.fixed_code().is_empty());
}

#[test]
fn heal_domain_build_corpus_matches_seed() {
    use crate::domain::HealDomain;
    let via_trait = KernelOptDomain::build_corpus();
    let via_seed = build_kernel_seed_corpus().entries;
    assert_eq!(via_trait.len(), via_seed.len());
    assert_eq!(via_trait[0].rule_id, via_seed[0].rule_id);
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn validate_parses_valid_code() {
    use crate::domain::HealDomain;
    let verdict = KernelOptDomain::validate("matvec-coalesce", "fn foo() -> i32 { 42 }");
    assert!(verdict_passes(&verdict));
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn validate_rejects_garbage() {
    use crate::domain::HealDomain;
    let verdict = KernelOptDomain::validate("matvec-coalesce", "not valid rust {");
    assert!(!verdict_passes(&verdict));
}

#[test]
fn batch1_rules_present() {
    // Verify the 20 new batch-1 rules are in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "coalesced-row-read",
        "vectorized-pack-load",
        "grid-stride-loop",
        "masked-boundary-load",
        "autotune-warps-stages",
        "upcast-before-reduction",
        "branchless-select",
        "unroll-fixed-inner",
        "forceinline-device-fn",
        "fuse-norm-act-linear",
        "fuse-quant-matmul",
        "fuse-softmax-mask",
        "fuse-xent-logsumexp",
        "sm-count-block-size",
        "predicate-gated-backend",
        "pow2-block-alignment",
        "multi-dim-grid-launch",
        "block-count-clamp",
        "backward-as-conjugate",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-1 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch1_categories_covered() {
    // Verify all 5 categories are represented in the gpu_concept fields.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Memory access patterns
    assert!(concepts.contains(&"coalesced memory access"));
    assert!(concepts.contains(&"vectorized memory access"));
    assert!(concepts.contains(&"work distribution"));
    // Compute patterns
    // Batch 130 appended a rider (cloudrift-ai/emmy launch-config negatives),
    // so the B23-style exact pin became prefix + rider asserts (the B125
    // full-string regeneration lesson, prefix form).
    let autotune = concepts
        .iter()
        .find(|c| c.starts_with("autotune dispatch"))
        .expect("autotune-warps-stages concept missing");
    assert!(
        autotune.contains("RIDER (Batch 130"),
        "batch-130 autotune rider missing"
    );
    assert!(concepts.contains(&"numerical precision in reductions"));
    assert!(concepts.contains(&"branch divergence elimination"));
    // Batch 130 appended a boundary rider (the CloudRift unroll negatives) --
    // prefix + rider form per the B125 lesson.
    let unroll = concepts
        .iter()
        .find(|c| c.starts_with("loop unrolling for instruction-level parallelism"))
        .expect("unroll-fixed-inner concept missing");
    assert!(
        unroll.contains("RIDER (Batch 130"),
        "batch-130 unroll rider missing"
    );
    assert!(concepts.contains(&"function inlining"));
    // Fusion patterns
    assert!(concepts.contains(&"kernel fusion"));
    assert!(concepts.contains(&"quantization fusion"));
    assert!(concepts.contains(&"mask fusion in softmax"));
    assert!(concepts.contains(&"cross-entropy fusion"));
    // Dispatch patterns
    assert!(concepts.contains(&"occupancy-aware block sizing"));
    assert!(concepts.contains(&"conditional kernel dispatch"));
    assert!(concepts.contains(&"power-of-2 alignment"));
    // Parallelism patterns
    assert!(concepts.contains(&"multi-dimensional parallelism"));
    assert!(concepts.contains(&"grid dimension clamping"));
    assert!(concepts.contains(&"backward kernel reuse"));
}

#[test]
fn batch2_rules_present() {
    // Verify the 7 batch-2 flash-attention tiling rules are in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "online-softmax-streaming",
        "exp2-log2-base2",
        "block-pointer-2d",
        "heuristic-compile-dispatch",
        "causal-diag-offdiag-split",
        "shared-mem-aware-tile",
        "kv-block-streaming",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-2 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch2_flash_attention_concepts_covered() {
    // Verify the flash-attention concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // The Batch-138 streaming-license rider extended this concept with a
    // RIDER suffix — assert the prefix, not the whole string.
    assert!(concepts
        .iter()
        .any(|c| c.starts_with("flash-attention online softmax")));
    assert!(concepts.contains(&"base-2 math for faster GPU exponentiation"));
    assert!(concepts.contains(&"structured 2D block pointer loads"));
    assert!(concepts.contains(&"compile-time heuristic dispatch"));
    assert!(concepts.contains(&"causal matrix diagonal/off-diagonal splitting"));
    assert!(concepts.contains(&"shared-memory-aware tile sizing"));
    assert!(concepts.contains(&"KV block streaming through shared memory"));
}

#[test]
fn batch3_rules_present() {
    // Verify the 6 batch-3 linear-attention recurrence rules are in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "state-forget-gate",
        "householder-rank1-update",
        "chunkwise-parallel-recurrence",
        "recurrent-vs-chunk-dispatch",
        "multi-write-per-step",
        "chained-two-pass-recurrence",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-3 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch3_recurrence_concepts_covered() {
    // Verify the linear-attention recurrence concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    assert!(concepts.contains(&"exponential decay gate on recurrent state"));
    assert!(concepts.contains(&"rank-1 Householder reflection state update"));
    assert!(concepts.contains(&"chunkwise parallelism for state-carrying loops"));
    assert!(concepts.contains(&"sequence-length-aware backend dispatch"));
    assert!(concepts.contains(&"multiple state writes per timestep"));
    assert!(concepts.contains(&"chained sequential recurrences with softmax bridge"));
}

#[test]
fn batch4_rules_present() {
    // Verify the 9 batch-4 MoE dispatch + backward-pass rules are in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "grouped-launch-cumsum-offsets",
        "l2-grouped-tile-schedule",
        "fused-matmul-activation",
        "fused-backward-chain",
        "activation-recompute-budget",
        "atomic-scatter-reduction",
        "weight-grad-batched-outer-product",
        "csr-index-via-prefix-sum",
        "autotune-aggregate-workload-key",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-4 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch4_moe_backward_concepts_covered() {
    // Verify the MoE dispatch + backward-pass concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    assert!(concepts.contains(&"grouped GEMM with prefix-sum offsets"));
    // Batch 130 appended a rider (the CloudRift SWIZ=8 external instance) --
    // prefix + rider form per the B125 lesson.
    let l2group = concepts
        .iter()
        .find(|c| c.starts_with("swizzled L2-cache-aware tile mapping"))
        .expect("l2-grouped-tile-schedule concept missing");
    assert!(
        l2group.contains("RIDER (Batch 130"),
        "batch-130 l2-group rider missing"
    );
    assert!(concepts.contains(&"pointwise activation in matmul epilogue"));
    assert!(concepts.contains(&"two backward ops fused into one kernel"));
    assert!(concepts.contains(&"partial activation save with in-kernel recompute"));
    assert!(concepts.contains(&"lock-free atomic scatter for parallel writes"));
    assert!(concepts.contains(&"weight gradients as batched outer product GEMM"));
    assert!(concepts.contains(&"GPU-side CSR sparse layout via prefix sum"));
    assert!(concepts.contains(&"autotune keyed on aggregate workload size"));
}

#[test]
fn batch5_rules_present() {
    // Verify the 4 batch-5 NSA block-sparse selection rules are in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "block-sparse-gather",
        "precompute-scalar-reduction",
        "block-compression-before-routing",
        "sliding-window-plus-sparse",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-5 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch5_nsa_concepts_covered() {
    // Verify the NSA block-sparse selection concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    assert!(concepts.contains(&"block-sparse indexed gather via selection array"));
    assert!(concepts.contains(&"hoist invariant row reduction out of per-block loop"));
    assert!(concepts.contains(&"pool to coarse blocks, route on compressed reps"));
    assert!(concepts.contains(&"decompose attention into local + global sparse, gated sum"));
}

#[test]
fn batch6_rules_present() {
    // Verify the 3 batch-6 varlen + chunked-reduction rules are in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "varlen-packing",
        "chunk-reset-cumsum",
        "cumsum-additive-bias",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-6 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch6_varlen_chunked_concepts_covered() {
    // Verify the varlen + chunked-reduction concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    assert!(concepts.contains(&"pack variable-length sequences via cu_seqlens, no padding"));
    assert!(concepts.contains(&"cumsum that resets at chunk boundaries"));
    assert!(concepts.contains(&"additive bias via cumulative sum of a gate"));
}

#[test]
fn batch7_rules_present() {
    // Verify the 9 batch-7 burn-cubecl autotune dispatch + conv/layout rules.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "panic-safe-autotune-candidate",
        "multi-priority-tune-groups",
        "shape-kind-dispatch",
        "vector-width-pow2-sweep",
        "conv-1x1-as-matmul",
        "contiguity-guard-before-reshape",
        "cube-count-bounded-batch-pack",
        "stride-align-vectorization-key",
        "anchor-shape-bucketing",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-7 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch7_burn_cubecl_concepts_covered() {
    // Verify the burn-cubecl autotune + conv/layout concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    assert!(concepts.contains(&"panic-resilient autotune candidate"));
    assert!(concepts.contains(&"shape-keyed priority-ranked autotune groups"));
    assert!(concepts.contains(&"matmul-kind-keyed kernel family dispatch"));
    assert!(concepts.contains(&"vectorization width as an autotune dimension"));
    assert!(concepts.contains(&"1x1 convolution as direct matmul (skip im2col)"));
    assert!(concepts.contains(&"stride-contiguity guard skips unnecessary relayout copy"));
    assert!(concepts.contains(&"grid-limit-aware batch packing"));
    assert!(concepts.contains(&"stride-byte-alignment determines vectorization width"));
    assert!(concepts.contains(&"shape bucketing for autotune cache reuse"));
}

#[test]
fn batch8_rules_present() {
    // Verify the 9 batch-8 kernel polymorphism + dtype/alignment + IR strength-reduction rules.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in [
        "op-family-type-dispatch",
        "inplace-output-alias",
        "layout-permute-for-coalescing",
        "dtype-specialized-float-kind",
        "alignment-gated-zero-copy-view",
        "strength-reduce-mul-pow2",
        "strength-reduce-mul-pow2-plus-minus-one",
        "strength-reduce-div-pow2",
        "strength-reduce-mod-pow2",
    ] {
        assert!(
            ids.contains(&expected),
            "batch-8 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch8_polymorphism_dtype_strength_concepts_covered() {
    // Verify the 3 new category concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category M: kernel parametric polymorphism
    assert!(concepts.contains(&"parametric kernel polymorphism over op trait"));
    assert!(concepts.contains(&"output buffer reuse via mutability check (skip allocation)"));
    assert!(concepts.contains(&"memory layout permutation for coalesced channel access"));
    // Category N: dtype + alignment specialization
    assert!(concepts.contains(&"dtype-keyed intrinsic dispatch for low-precision floats"));
    assert!(concepts.contains(&"alignment-gated zero-copy slice view via handle offset"));
    // Category O: IR strength reduction
    assert!(concepts.contains(&"IR strength reduction: multiply by power of 2 → left shift"));
    assert!(concepts.contains(&"IR strength reduction: multiply by (2^k ± 1) → shift ± self"));
    assert!(concepts.contains(&"IR strength reduction: divide by power of 2 → right shift"));
    assert!(concepts.contains(&"IR strength reduction: modulo by power of 2 → bitwise and mask"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 11 — popcorn linear-attention recurrence family (12 rules)
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch11_rules_present() {
    // Verify the 12 batch-11 linear-attention recurrence + scan rules.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // G: linear-attn recurrence (+5)
    assert!(
        ids.contains(&"per-key-dim-diagonal-gate"),
        "gla per-dim gate"
    );
    assert!(
        ids.contains(&"erase-replace-low-rank-transition"),
        "rwkv7 4-phase"
    );
    assert!(
        ids.contains(&"oja-subtractive-normalization"),
        "gated_oja_rule"
    );
    assert!(
        ids.contains(&"identity-plus-low-rank-delta"),
        "iplr_delta_rule"
    );
    assert!(
        ids.contains(&"cumsum-gate-to-decay-matrix"),
        "simple_gla pairwise"
    );
    // E: parallelism (+2)
    assert!(
        ids.contains(&"reverse-scan-via-double-flip"),
        "chunk_*_cumsum reverse"
    );
    assert!(
        ids.contains(&"pairwise-distance-decay-matrix"),
        "retention/lightning decay"
    );
    // F: flash-attn tiling (+1)
    assert!(
        ids.contains(&"strict-causal-mask-tril-offset"),
        "deltaformer tril(-1)"
    );
    // B: compute (+2)
    assert!(
        ids.contains(&"polynomial-taylor-feature-map"),
        "based/rebased poly"
    );
    assert!(
        ids.contains(&"positive-feature-map-shift"),
        "linear_attn elu+1"
    );
    // D: dispatch (+1)
    assert!(
        ids.contains(&"layer-depth-decay-schedule"),
        "lightning_attn depth"
    );
    // (J chunk-reset-scan was a duplicate of chunk-reset-cumsum — dropped.)
}

#[test]
fn batch11_recurrence_concepts_covered() {
    // Verify the batch-11 recurrence + scan concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category G: linear-attention recurrence (new)
    assert!(concepts.contains(&"per-dimension diagonal decay gate on recurrent state"));
    assert!(concepts
        .contains(&"diagonal-plus-low-rank state transition (erase, decay, replace, write)"));
    assert!(concepts
        .contains(&"Oja subtractive normalization: project out stored component before write"));
    assert!(concepts
        .contains(&"identity-plus-low-rank (I+ba^T) state rotation then outer-product write"));
    assert!(concepts
        .contains(&"convert per-step log-gate into pairwise decay matrix via cumsum + diff"));
    // Category E: parallelism (new)
    assert!(concepts.contains(&"reverse-direction scan via flip → forward scan → flip"));
    assert!(concepts.contains(&"materialize N×N decay matrix from index distance and slope"));
    // Category F: flash-attn tiling (new)
    assert!(concepts.contains(&"strictly-causal mask via tril(-1) (exclude diagonal) + NaN guard"));
    // Category B: compute (new)
    assert!(concepts.contains(&"polynomial Taylor feature map approximating the softmax kernel"));
    assert!(concepts.contains(
        &"shift activation positive (elu+1) so linear-attention normalizer never crosses zero"
    ));
    // Category D: dispatch (new)
    assert!(concepts.contains(&"layer-depth-dependent decay schedule (deeper layers decay slower)"));
}

// ══════════════════════════════════════════════════════════════════
// Batch 12 (Research 013) — gated delta + specialized recurrence + advanced attention
// ══════════════════════════════════════════════════════════════════

#[test]
fn batch12_rules_present() {
    // Verify the 14 batch-12 rules across G/B/F/E/C/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // G: linear-attn recurrence (+6)
    assert!(
        ids.contains(&"gate-then-correct-delta-rule"),
        "gated_delta_rule"
    );
    assert!(
        ids.contains(&"gate-then-multi-householder-write"),
        "gated_delta_product"
    );
    assert!(ids.contains(&"online-gram-ridge-solve"), "mesa_net ridge");
    assert!(
        ids.contains(&"momentum-gated-memory-recurrence"),
        "titans_linear momentum"
    );
    assert!(
        ids.contains(&"current-token-bonus-read-before-update"),
        "rwkv6 bonus"
    );
    assert!(
        ids.contains(&"wkv-logsumexp-three-channel-state"),
        "rwkv4 3ch"
    );
    // B: compute (+3)
    assert!(
        ids.contains(&"hierarchical-log2-level-mask"),
        "log_linear_attn hier"
    );
    assert!(
        ids.contains(&"iterative-cg-solve-over-matvec"),
        "mesa_net_decode cg"
    );
    assert!(
        ids.contains(&"chunked-test-time-gradient-descent"),
        "ttt chunked gd"
    );
    // F: flash-attn tiling (+2)
    assert!(
        ids.contains(&"block-local-reference-frame-for-overflow"),
        "wall_attn reframe"
    );
    assert!(
        ids.contains(&"two-stage-mask-softmax-then-conv"),
        "multi_token_attention 2stage"
    );
    // E: parallelism (+1)
    assert!(
        ids.contains(&"softmax-topk-renormalize-scatter"),
        "momoe gate"
    );
    // C: fusion (+1)
    assert!(
        ids.contains(&"fuse-gate-transform-chunk-cumsum"),
        "kda_gate_cumsum fused"
    );
    // D: dispatch (+1)
    assert!(
        ids.contains(&"dual-mode-gate-transform"),
        "kda_gate dual-mode"
    );
}

#[test]
fn batch12_recurrence_attention_concepts_covered() {
    // Verify the batch-12 recurrence + attention + gate concepts are represented.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category G: linear-attn recurrence (new)
    assert!(
        concepts.contains(&"gated delta rule: decay state then correct against decayed subspace")
    );
    assert!(concepts.contains(&"one outer gate wrapping N inner Householder delta writes per step"));
    assert!(concepts.contains(&"online ridge regression: gated Gram accumulators + per-step solve"));
    assert!(concepts.contains(&"two coupled recurrences: SGD momentum state + EMA memory matrix"));
    assert!(concepts
        .contains(&"read from boosted state (transient bonus), then update with un-boosted kv"));
    assert!(
        concepts.contains(&"WKV with 3-channel state (num/den/max) + logsumexp max-subtraction")
    );
    // Category B: compute (new)
    assert!(concepts.contains(&"O(log seq) hierarchical mask with per-level learned scales"));
    assert!(concepts
        .contains(&"conjugate-gradient solve over matvec with Jacobi preconditioner (no inverse)"));
    assert!(concepts.contains(&"mini-batch chunked GD with causal mask + last-step weight update"));
    // Category F: flash-attn tiling (new)
    assert!(concepts.contains(
        &"per-block reference frame to keep exp arguments bounded (avoid fp32 overflow)"
    ));
    assert!(concepts.contains(
        &"two-stage mask: -inf before softmax, 0 after conv (probability + causal correct)"
    ));
    // Category E: parallelism (new)
    assert!(
        concepts.contains(&"MoE gate: softmax → topk → renormalize → scatter sparse gate matrix")
    );
    // Category C: fusion (new)
    assert!(concepts
        .contains(&"fuse gate transform with chunk-local cumsum (no materialized intermediate)"));
    // Category D: dispatch (new)
    assert!(concepts.contains(
        &"dual-mode gate: unbounded softplus OR bounded sigmoid, dispatched by parameter"
    ));
}

#[test]
fn batch13_rules_present() {
    // Verify the 6 batch-13 rules across B/C/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category B (4 new): numerical stability, complex-free RoPE, stateful shift, squared ReLU
    assert!(ids.contains(&"expm1-for-near-zero-exp-minus-one"));
    assert!(ids.contains(&"complex-mul-via-paired-reals"));
    assert!(ids.contains(&"stateful-shift-via-cat-prefix"));
    assert!(ids.contains(&"squared-relu-activation"));
    // Category C (1 new): fused reduce-then-broadcast
    assert!(ids.contains(&"rkv-receptance-correction"));
    // Category D (1 new): multimodal cyclic table gather
    assert!(ids.contains(&"cyclic-modality-table-selection"));
}

#[test]
fn batch13_stability_rope_activation_concepts_covered() {
    // Verify the batch-13 numerical stability + RoPE + activation concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category B: numerical stability (expm1 for KL cancellation)
    assert!(concepts.contains(
        &"numerically stable e^x − 1 via expm1 (avoids catastrophic cancellation near 0)"
    ));
    // Category B: complex-free RoPE (paired reals)
    assert!(concepts.contains(
        &"represent complex numbers as interleaved [real, imag] pairs, multiply via 4 scalar muls"
    ));
    // Category B: stateful shift (cross-batch carry)
    assert!(concepts.contains(&"shift sequence with cross-batch carry: cat(prev_state, x[:-1])"));
    // Category B: squared ReLU (sparse sharpening)
    assert!(concepts.contains(
        &"relu(x)² — sparse activation: preserves ReLU zeros, sharpens non-zeros quadratically"
    ));
    // Category C: fused reduce-then-broadcast (RWKV-7 gate output correction)
    assert!(concepts.contains(
        &"fused reduce-then-broadcast: (r·k·ρ).sum(head_dim,keepdim)·v added to o, gated by g"
    ));
    // Category D: multimodal cyclic table selection
    assert!(concepts.contains(
        &"multimodal RoPE: split head_dim into sections, cyclic gather from N modality tables"
    ));
}

// ════════════════════════════════════════════════════════════════════
// Batch 14 (Research 015) — riir-gpu production kernels: norm-fold + reduction-split + single-plane + CPU-offload
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch14_rules_present() {
    // Verify the 4 batch-14 rules across C/E/A.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category C (1 new): fold norm scalar into matmul
    assert!(
        ids.contains(&"fold-norm-scalar-into-matmul"),
        "fold norm into gemv"
    );
    // Category E (2 new): reduction-split pre-pass + single-plane reduction
    assert!(
        ids.contains(&"split-reduction-scalar-prepass"),
        "scalar pre-pass"
    );
    assert!(
        ids.contains(&"single-plane-sufficient-reduction"),
        "single plane"
    );
    // Category A (1 new): offload CPU bottleneck to GPU kernel
    assert!(
        ids.contains(&"offload-cpu-bottleneck-to-gpu-kernel"),
        "cpu offload"
    );
}

#[test]
fn batch14_norm_fold_reduction_split_offload_concepts_covered() {
    // Verify the batch-14 norm-fold + reduction-split + single-plane + offload concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category C: fold norm scalar into matmul (riir-gpu gemv_q4k_rmsnorm)
    assert!(concepts.contains(&"fold normalization scalar (inv_rms) into the matmul dot product — norm becomes 2 inline multiplies"));
    // Category E: split reduction into scalar pre-pass (riir-gpu compute_inv_rms)
    assert!(concepts.contains(&"split a reduction whose output is a tiny scalar into a pre-pass, fold the scalar into the main consumer"));
    // Category E: single-plane sufficient reduction (riir-gpu Issue 611)
    assert!(concepts.contains(&"one plane (warp) suffices for a cooperative reduction — do not launch N redundant planes that recompute the identical sum"));
    // Category A: offload CPU bottleneck to GPU kernel (riir-gpu gpu_transpose)
    assert!(concepts.contains(&"when CPU data-prep (transpose, permute, format convert) is the bottleneck, run a GPU compute kernel that writes directly into pooled memory"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 15 (Research 016) — riir-gpu production kernels: block-contiguous layout + fused GeGLU GEMV + orthogonal pre-rotation + water-fill tail-dim
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch15_rules_present() {
    // Verify the 4 batch-15 rules across A/B/C/N.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category A (1 new): block-contiguous weight layout
    assert!(
        ids.contains(&"block-contiguous-weight-layout"),
        "block-contiguous layout"
    );
    // Category C (1 new): fuse gate+up GEMV + GeGLU
    assert!(
        ids.contains(&"fuse-gate-up-gemv-activation"),
        "fused dual GEMV + GeGLU"
    );
    // Category B (1 new): pre-rotate query once
    assert!(
        ids.contains(&"pre-rotate-once-exploit-orthogonality"),
        "pre-rotate query"
    );
    // Category N (1 new): water-fill tail-dim shared codebook
    assert!(
        ids.contains(&"water-fill-tail-dim-shared-codebook"),
        "water-fill tail-dim"
    );
}

#[test]
fn batch15_block_contiguous_fused_geglu_rotation_codebook_concepts_covered() {
    // Verify the batch-15 block-contiguous + fused-geglu + rotation + codebook concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category A: block-contiguous weight layout (riir-gpu gemm_ternary_block_contiguous)
    assert!(concepts.contains(&"pack multiple logical weight buffers (scale + bit-planes) into one block-contiguous buffer so one global-memory access fetches all components"));
    // Category C: fuse gate+up GEMV + GeGLU (riir-gpu gemv_geglu_f16).
    // Batch 170 appended a rider to this concept (mlxfast-gemma4 strided-view
    // hazard), so the FULL-STRING pin became prefix + rider asserts (the
    // B125/B129 prefix lesson -- original text pinned verbatim, rider pinned
    // by its opening marker).
    let geglu = concepts.iter().find(|c| c.starts_with("fuse two sibling GEMVs (gate + up projections) + their multiplicative activation into one kernel dispatch")).expect("fuse-gate-up-gemv-activation concept prefix missing");
    assert!(
        geglu.contains("RIDER (Batch 170, mlxfast-gemma4"),
        "batch-170 geglu strided-view rider missing"
    );
    // Category B: pre-rotate query once (riir-gpu spectralquant/attention.rs)
    assert!(concepts.contains(&"for dot(q, R·y) with many y and orthogonal R, pre-rotate the query once (q' = R^T·q) instead of rotating each y"));
    // Category N: water-fill tail-dim shared codebook (riir-gpu spectralquant/dequant.rs)
    assert!(concepts.contains(&"non-uniform bit allocation: per-dim codebooks for top-energy dims, single shared low-bit codebook for low-energy tail dims"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 16 (Research 017) — riir-gpu production kernels: cmma hardware unit + dequant staging + dispatch axis swap + norm-decoupled scoring
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch16_rules_present() {
    // Verify the 4 batch-16 rules across A/B/D/N.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category B (1 new): cmma hardware unit over scalar ALU
    assert!(
        ids.contains(&"cmma-hardware-unit-over-scalar-alu"),
        "cmma hardware unit"
    );
    // Category A (1 new): dequant once per K-tile into shared memory
    assert!(
        ids.contains(&"dequant-once-per-k-tile-into-shared-memory"),
        "dequant staging"
    );
    // Category D (1 new): swap large axis to X for workgroup distribution
    assert!(
        ids.contains(&"swap-large-axis-to-x-for-workgroup-distribution"),
        "dispatch axis swap"
    );
    // Category N (1 new): norm-decoupled low-bit scoring
    assert!(
        ids.contains(&"norm-decoupled-low-bit-scoring"),
        "norm-decoupled scoring"
    );
}

#[test]
fn batch16_cmma_dequant_swap_normdecoupled_concepts_covered() {
    // Verify the batch-16 cmma + dequant-staging + axis-swap + norm-decoupled concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category B: cmma hardware unit (riir-gpu gemm_ternary_simdgroup)
    assert!(concepts.contains(&"use the cooperative matrix multiply-accumulate hardware unit (tensor core / Metal simdgroup_matrix / cmma) instead of scalar ALU for matrix multiply"));
    // Category A: dequant once per K-tile into shared memory (riir-gpu gemm_ternary_simdgroup)
    assert!(concepts.contains(&"dequantize a weight tile once into shared memory, let the hardware matrix unit consume it across all output rows that share the tile"));
    // Category D: swap large axis to X (riir-gpu gemm_ternary_simdgroup)
    assert!(concepts.contains(&"when one GEMM axis is much larger, put it on the X (first) cube-count axis for better workgroup distribution across SMs"));
    // Category N: norm-decoupled low-bit scoring (riir-gpu forward_turboquant)
    assert!(concepts.contains(&"store per-vector L2 norm separately as a scalar; quantize the unit-normalized vector at lower bits; reconstruct the dot product as norm × dot(unit_q, unit_k_quantized)"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 17 (Research 018) — riir-gpu production kernels: zero-word OR-skip + scale hoist + split-sign accumulators + clamp-tile sentinel
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch17_rules_present() {
    // Verify the 4 batch-17 rules across A/B.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category B (3 new): zero-word OR-skip, split-sign accumulators, clamp-tile sentinel
    assert!(
        ids.contains(&"skip-all-zero-packed-words-via-or-reduce"),
        "zero-word or-skip"
    );
    assert!(
        ids.contains(&"split-sign-accumulators-defer-subtraction"),
        "split-sign accumulators"
    );
    assert!(
        ids.contains(&"clamp-tile-indices-to-sentinel-discard-at-write"),
        "clamp-tile sentinel"
    );
    // Category A (1 new): hoist group-invariant scale
    assert!(
        ids.contains(&"hoist-group-invariant-scale-out-of-word-loop"),
        "scale hoist"
    );
}

#[test]
fn batch17_zeroword_scalehoist_splitsign_clampsentinel_concepts_covered() {
    // Verify the batch-17 zero-word-skip + scale-hoist + split-sign + clamp-sentinel concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category B: zero-word OR-reduce skip (riir-gpu gemm_ternary_batched)
    assert!(concepts.contains(&"OR-reduce all rows' pos+neg bit-words; skip the 32-iteration sign-extraction loop when the whole tile's word is zero (ternary sparsity exploit)"));
    // Category A: hoist group-invariant scale (riir-gpu gemv_ternary_block_contiguous)
    assert!(concepts.contains(&"cache a per-group invariant (e.g. quant scale shared across N words) in a register; reload only when the group boundary changes, not every iteration"));
    // Category B: split-sign accumulators (both ternary files)
    assert!(concepts.contains(&"in a signed ternary/bit-packed dot loop, keep separate pos and neg accumulators; subtract once at the end instead of per-bit (fewer subtractions + independent accumulator chains)"));
    // Category B: clamp-tile sentinel (riir-gpu gemm_ternary_batched)
    assert!(concepts.contains(&"when a tile extends past the matrix edge, alias OOB rows to a valid sentinel row and discard the redundant results at write time — keeps the loop body uniform (no per-lane divergence) without padding overhead"));
}

#[test]
fn batch18_rules_present() {
    // Verify the 3 batch-18 rules across A/B.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category A (2 new): row-tile amortize input load + pin-lane coalesce
    assert!(
        ids.contains(&"row-tile-amortize-input-load"),
        "row-tile amortize input load"
    );
    assert!(
        ids.contains(&"pin-lane-to-quant-sub-block-coalesce-byte-loads"),
        "pin-lane coalesce byte loads"
    );
    // Category B (1 new): factor affine quant scale
    assert!(
        ids.contains(&"factor-affine-quant-scale-out-of-element-loop"),
        "factor affine quant scale"
    );
}

#[test]
fn batch18_rowtile_factoraffine_pinlane_concepts_covered() {
    // Verify the batch-18 row-tile + affine-factor + pin-lane concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category A: row-tile-amortize-input-load (riir-gpu gemv_q4k_plane_rowtiled; full-concept pin moved with the Batch 108 rider per the B22 precedent)
    assert!(concepts.contains(&"assign N consecutive output rows per plane (subgroup), hold N register accumulators, load each input element once and reuse across all N dot products — GEMV to small-GEMM via M-widening. RIDER (Batch 108, riir-ai M3 campaign / Plan 078, the CMMA STAGED-GEMM IN-STACK INSTANCE): the same mechanism at the tensor-core tile grain — the 32x32 staged GEMM stages each input tile ONCE and consumes it across 4 M-subtiles (4 independent accumulator sets), cutting input traffic 4x; the input-load share measured 42-63% of the whole kernel (91.2 GB/GEMM of input traffic at the FFN shape), the largest single cost axis; regime m >= 64 (measured 0.773x at m=48 — padding dominates small m, stay on the older arms); bit-identity argument: execute order WITHIN a k-tile is free across distinct accumulators, per-accumulator order across k-tiles unchanged (riir-ai @ 59a172623 gemm_ternary_simdgroup_cubecl.rs:938-940; the untried-axis note at :563-565 — every prior variant widened P or restructured K, none increased M-rows-per-workgroup; Issue 768 / Bench 774: 32x32 promoted default-ON, e2e 1.41-1.53x)."));
    // Category B: factor affine quant scale (riir-gpu gemv_q4k_plane_factored)
    assert!(concepts.contains(&"when the dequant formula is affine in the quantized code within a sub-block (scale and min are constant), factor d*scale and dmin*min out of the per-element accumulation — accumulate sum(code*x) and sum(x) separately, apply scales once per sub-block"));
    // Category A: pin-lane coalesce byte loads (riir-gpu gemv_q4k_plane_factored)
    assert!(concepts.contains(&"remap the lane-element assignment so each lane is pinned to one quantization sub-block — the 8 consecutive bytes it needs fall in 2 consecutive u32 words (8 loads to 2), and scale/min become loop-invariant (enabling affine factoring)"));
}

#[test]
fn batch19_rules_present() {
    // Verify the 2 batch-19 rules in Category C (fusion).
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // Category C (2 new): chain-dispatches-single-sync-point + fuse-residual-via-in-place-output-array
    assert!(
        ids.contains(&"chain-dispatches-single-sync-point"),
        "chain dispatches single sync point"
    );
    assert!(
        ids.contains(&"fuse-residual-via-in-place-output-array"),
        "fuse residual via in-place output array"
    );
}

#[test]
fn batch19_chain_dispatch_inplace_residual_concepts_covered() {
    // Verify the batch-19 chain-dispatch + in-place-residual concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // Category C: chain-dispatches-single-sync-point (riir-gpu ternary_ffn_fused)
    assert!(concepts.contains(&"queue N dependent GPU dispatches in one command buffer, sync only once at the end (not after each dispatch) — linear dependency chains keep intermediate buffer count bounded"));
    // Category C: fuse-residual-via-in-place-output-array (riir-gpu gemv_ternary_residual_cubecl)
    assert!(concepts.contains(&"when a kernel needs N+1 array parameters but the runtime limits to N, and the output is the accumulation target of the extra input, alias the output buffer as the accumulation input (read-then-write) — bypasses parameter count limits without adding a dispatch"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 20 (Research 021) — CUDA hardware-instruction cluster +
// CubeCL LUT + base-3 trit (5 rules: 1A + 3B + 1N)
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch20_rules_present() {
    // Verify the 5 batch-20 rules across A/B/N.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (1 new): cooperative-block-wide-quantize-staging
    assert!(
        ids.contains(&"cooperative-block-wide-quantize-staging"),
        "cooperative block-wide quantize staging"
    );
    // B (3 new): byte-perm-lut-via-permutation-immediate + dp4a-int8-hardware-dot-product + threadgroup-subset-sum-lut-keyed-by-activation
    assert!(
        ids.contains(&"byte-perm-lut-via-permutation-immediate"),
        "byte perm lut via permutation immediate"
    );
    assert!(
        ids.contains(&"dp4a-int8-hardware-dot-product"),
        "dp4a int8 hardware dot product"
    );
    assert!(
        ids.contains(&"threadgroup-subset-sum-lut-keyed-by-activation"),
        "threadgroup subset-sum lut keyed by activation"
    );
    // N (1 new): base-3-trit-packing-5-per-byte
    assert!(
        ids.contains(&"base-3-trit-packing-5-per-byte"),
        "base-3 trit packing 5 per byte"
    );
}

#[test]
fn batch20_cuda_dp4a_lut_trit_concepts_covered() {
    // Verify the batch-20 CUDA-dp4a + LUT + trit concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // A: cooperative-block-wide-quantize-staging (riir-gpu gemv_ternary_cuda_raw.rs fused variant)
    assert!(concepts.contains(&"when an input vector must be quantized/transformed once per dispatch and consumed by every warp in the block, have ALL threads cooperate to write disjoint chunks into shared memory, then each warp reads from shared memory — eliminates a separate staging dispatch + global-memory round-trip"));
    // B: byte-perm-lut-via-permutation-immediate (riir-gpu gemv_ternary_cuda_raw.rs)
    assert!(concepts.contains(&"decode N small-radix codes to signed symbols in ONE hardware instruction via __byte_perm(constant, constant, code >> shift) — the constant packs the signed alphabet as bytes, the selector nibbles index straight to the symbol (no LUT load, no branch, no modular arithmetic)"));
    // B: dp4a-int8-hardware-dot-product (riir-gpu gemv_ternary_cuda_raw.rs)
    assert!(concepts.contains(&"replace scalar 4-element int8 dot product with NVIDIA __dp4a — one PTX instruction (dp4a.s32.s8.s32) computes a[0]*b[0]+a[1]*b[1]+a[2]*b[2]+a[3]*b[3]+c for 4 int8 lanes. Distinct from cmma matrix tile op (Batch 16) — this is a scalar 4-lane SIMD dot, CUDA-only, requires int8 operands"));
    // B: threadgroup-subset-sum-lut-keyed-by-activation (riir-gpu gemv_ternary_cubecl.rs LUT path)
    assert!(concepts.contains(&"precompute a 16-entry subset-sum table per 4-element activation group in threadgroup memory (keyed by activation subset, containing no weight/scale/zero-point), then each 4-weight signed dot collapses to 2 table reads + 1 subtract — one build serves every output row because the table depends on nothing that varies down the rows"));
    // N: base-3-trit-packing-5-per-byte (riir-gpu gemv_ternary_cubecl.rs trit path)
    assert!(concepts.contains(&"pack 5 ternary values per byte using base-3 mixed-radix encoding (byte % 3 decode) — 17.6% denser than 2-bit codes (1.75 vs 2.125 bits/weight), regime-dependent: wins on bandwidth-bound Metal (div/mod cheap), loses 2.1x on CUDA scalar ALU (div/mod expensive, incompatible with __dp4a int8 path)"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 21 (Research 022) — Metal cross-runtime zero-copy integration
// cluster (5 rules: 4A + 1D)
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch21_rules_present() {
    // Verify the 5 batch-21 rules across A/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (4 new): cross-runtime-raw-handle-extraction + pool-buffer-aliasing-dedicated-output + same-queue-implicit-ordering-no-sync + per-dispatch-pointer-re-extraction
    assert!(
        ids.contains(&"cross-runtime-raw-handle-extraction"),
        "cross-runtime raw handle extraction"
    );
    assert!(
        ids.contains(&"pool-buffer-aliasing-dedicated-output"),
        "pool-buffer aliasing dedicated output"
    );
    assert!(
        ids.contains(&"same-queue-implicit-ordering-no-sync"),
        "same-queue implicit ordering no sync"
    );
    assert!(
        ids.contains(&"per-dispatch-pointer-re-extraction"),
        "per-dispatch pointer re-extraction"
    );
    // D (1 new): no-blocking-sync-in-async-pipeline
    assert!(
        ids.contains(&"no-blocking-sync-in-async-pipeline"),
        "no-blocking-sync-in-async-pipeline"
    );
}

#[test]
fn batch21_metal_cross_runtime_concepts_covered() {
    // Verify the batch-21 Metal cross-runtime integration concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // A: cross-runtime-raw-handle-extraction (riir-gpu gemm_ternary_metal_zero_copy.rs L230-252)
    assert!(concepts.contains(&"when a higher-level GPU runtime (wgpu, CubeCL) wraps a lower-level native API (Metal, Vulkan, CUDA), extract the raw native handle (e.g. id<MTLBuffer>) from the wrapper handle via the runtime's as_hal/raw_handle API — enables raw native kernel dispatch that reads/writes wrapper-owned memory without a host round-trip"));
    // A: pool-buffer-aliasing-dedicated-output (riir-gpu gemm_ternary_metal_zero_copy.rs L291-315)
    assert!(concepts.contains(&"when the runtime sub-allocates handles within one pool buffer (e.g. CubeCL pool buffer), input and output handles sharing the same pool alias each other and break hazard tracking — write to a dedicated output buffer keyed by byte size, then GPU->GPU copy back via blit encoder (same command buffer, zero host round-trip)"));
    // A: same-queue-implicit-ordering-no-sync (riir-gpu gemm_ternary_metal_zero_copy.rs L350-412)
    assert!(concepts.contains(&"when a raw native dispatch must be ordered relative to wrapper-runtime dispatches (e.g. Metal kernel output read by a later CubeCL dispatch), submit on the SAME command queue the wrapper uses — the API guarantees in-order execution within one queue, so no explicit host-side sync or wait_until_completed is needed"));
    // A: per-dispatch-pointer-re-extraction (riir-gpu gemm_ternary_metal_zero_copy.rs L211-224 + L278-283)
    assert!(concepts.contains(&"when a higher-level runtime can reallocate/defragment its memory pool between dispatches, raw native pointers extracted from wrapper handles must be re-extracted on EVERY dispatch, not cached across dispatches — cache the higher-level handle (ref-counted, stable) and re-extract the raw pointer fresh each time"));
    // D: no-blocking-sync-in-async-pipeline (riir-gpu gemm_ternary_metal_zero_copy.rs L407-419)
    assert!(concepts.contains(&"when dispatching on a shared command queue inside an async pipeline, never call wait_until_completed/sync — it serializes the pipeline by blocking the host thread. Use queue ordering (in-order execution within one queue) + the caller's pre-dispatch sync to cover correctness. Only non-queued read paths (test read_one) need explicit sync"));
}

#[test]
fn batch22_rules_present() {
    // Verify the 5 batch-22 rules across A/B/C/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (2 new): dynamic-shared-memory-runtime-sized + ternary-bit-plane-u64-as-2x-u32
    assert!(
        ids.contains(&"dynamic-shared-memory-runtime-sized"),
        "dynamic shared memory runtime sized"
    );
    assert!(
        ids.contains(&"ternary-bit-plane-u64-as-2x-u32"),
        "ternary bit plane u64 as 2x u32"
    );
    // B (1 new): warp-shuffle-intrinsics-for-sub-warp-reduction
    assert!(
        ids.contains(&"warp-shuffle-intrinsics-for-sub-warp-reduction"),
        "warp shuffle intrinsics for sub warp reduction"
    );
    // C (1 new): fused-activation-quantize-for-gemv-consumer
    assert!(
        ids.contains(&"fused-activation-quantize-for-gemv-consumer"),
        "fused activation quantize for gemv consumer"
    );
    // D (1 new): cuda-graph-scalar-via-device-pointer
    assert!(
        ids.contains(&"cuda-graph-scalar-via-device-pointer"),
        "cuda graph scalar via device pointer"
    );
}

#[test]
fn batch22_cudarc_kernels_concepts_covered() {
    // Verify the batch-22 cudarc_kernels CUDA cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // A: dynamic-shared-memory-runtime-sized (riir-gpu cudarc_kernels/attention.rs L265-301)
    assert!(concepts.contains(&"use extern __shared__ with launch-time shared_mem_bytes instead of a compile-time __shared__ array when the smem size depends on a runtime dimension (e.g. head_dim varies: 64 for DeltaNet, 128 for Qwen-7B, 256 for Bonsai-27B)"));
    // A: ternary-bit-plane-u64-as-2x-u32 (riir-gpu cudarc_kernels/embedding.rs L37-69)
    assert!(concepts.contains(&"ternary weights as two bit-planes (positive + negative sign bits) packed as u64 blocks cast to 2×u32 words — extract sign per column via bit_pos = c % 32, word_in_block = (c % 64) / 32, then output = sign * per-group scale. The 2×u32 cast is because u64 isn't directly per-bit addressable in CUDA; two u32 words are"));
    // B: warp-shuffle-intrinsics-for-sub-warp-reduction (riir-gpu cudarc_kernels/mod.rs L163-167 + attention.rs L336-344; + the Batch-100 Metal measured-cost rider)
    assert!(concepts.contains(&"for blocks of 32 threads or fewer (one warp), use __shfl_down_sync(mask, val, delta) butterfly reduction instead of shared memory + __syncthreads — the warp executes in lockstep (SIMT), so the shuffle is both correct and free (no smem allocation, no synchronization barrier). For the final 32-thread tail of a larger smem tree reduction, cast to volatile float* and skip __syncthreads — volatile prevents the compiler from caching stale values in registers. Measured Metal cost note (qwen-3.8-mtp-challenge kernel-research.md section 1.1 @ 0863b06a, Batch 100): simd_sum(f32) runs ~14.5 cycles throughput / ~28 latency vs ~2 for a shuffle — a per-(token, head) simd_sum is a real ALU cost in a GQA=8 pass-A inner loop (8 simd_sums per token per simdgroup); quad reductions are ~2x cheaper and a shuffle tree can beat the builtin — the shuffle-butterfly arm of this rule is the measured-cheap arm on Apple silicon as well, not only under the CUDA lockstep argument. RIDER (Batch 114 external instance, jundot/omlx @ e467261 -- custom_kernels/qwen35_prefill/gdn.py:438-442 + :499-517): the SUB-SIMDGROUP cooperation radius -- state fragments owned by 8-lane SEGMENTS inside a 32-lane simdgroup reduce via simd_shuffle_down(4,2,1) (three steps, staying inside the row's 8 lanes) and broadcast once from the row leader (simd_shuffle(part, (tid % 32) / 8 * 8)); zero barriers in the hot loop ('reduce across the 8 threads of a dv row via simd_shuffle_down (no threadgroup barriers in the hot loop)') -- a finer radius than one full simdgroup, with the leader-index broadcast replacing any cross-lane tree. RIDER (Batch 118, antirez/ds4 PR #991 @ 236cb2a5 MIT): the XOR-BUTTERFLY SUB-SIMDGROUP twin -- lane segments smaller than the simdgroup shuffle by xor masks, zero barriers: an 8-lane sub-simdgroup dot reduce (acc += simd_shuffle_xor(acc,1/2/4) within the stream's lane segment, s = tiisg/8, lane = tiisg%8) then a cross-segment merge (g += simd_shuffle_xor(g,8/16)) over 4 stream segments; in-source instances: router argmax ladder (qwen4.metal:821-825/:848-852), the online-softmax quad reduce (:1577-1589), hc_gate_mix_pair (:289-296), and the stream-row dot (:239,:245-251). Composition note: xor BUTTERFLY ladders are safe to accumulate IN PLACE (disjoint partner pairs per stage) while INDEXED shuffle ladders are not -- see the shuffle-lane unmutated-partial face on block-reduction-separate-scratch-runtime-strides (same batch)."));
    // C: fused-activation-quantize-for-gemv-consumer (riir-gpu cudarc_kernels/mod.rs L191-289 + L407-554)
    assert!(concepts.contains(&"when an activation's output (RMSNorm, SwiGLU, gate*sigmoid, gate*silu) is immediately consumed by a GEMV that reads int8 (not f32), fuse the activation + per-block absmax reduction + int8 quantize into one kernel — eliminates the intermediate f32 HBM round-trip and one kernel launch. The activation is the non-trivial part (needs a reduction or exp); the quantize is the trivial part (absmax + scale + round) RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- fused_kimi_k3_mla_key_concat_kv_cache_kernel.cu:15-24 + cache_kernels_fused.cu:108-145): the RoPE rotation is computed in-registers and the rotated pair is written DIRECTLY to its quantized cache slot in the producing kernel (qk_t rotate -> optional scaled_convert -> kv_cache_ptr[...]) -- the consumer-side quantize/store fused into the producer, three in-source kernel families (deepseek-v4, minimax-m3, kimi-k3). RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- commit 662a0b0 #27310 -- CUDA-side): the DFlash encoder is fused INTO the KV injection, and the sibling commit 8663224 #27877 DISABLES the non-fused GDN/LID paths -- the fused-path preference posture: when the fused kernel exists, the non-fused path is a liability, not a fallback."));
    // D: cuda-graph-scalar-via-device-pointer (riir-gpu cudarc_kernels/embedding.rs L76-105 + attention.rs L421-610)
    assert!(concepts.contains(&"when capturing a CUDA graph, scalar kernel args (int pos, int row_idx) get baked into the captured graph at capture time — the graph replays the exact value on every launch and cannot change. Ship a _devpos variant that reads the scalar from a const device pointer dereferenced at kernel runtime; the caller writes the new value to a 4-byte device buffer via memcpy_htod on the same stream before graph.launch(), so the captured graph processes a different input each call"));
}

#[test]
fn batch23_rules_present() {
    // Verify the 5 batch-23 rules across A/B/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (2 new): combined-kv-buffer-keys-then-values + gqa-kv-head-sharing-bandwidth-reduction
    assert!(
        ids.contains(&"combined-kv-buffer-keys-then-values"),
        "combined kv buffer keys then values"
    );
    assert!(
        ids.contains(&"gqa-kv-head-sharing-bandwidth-reduction"),
        "gqa kv head sharing bandwidth reduction"
    );
    // B (1 new): softcap-tanh-attention-score
    assert!(
        ids.contains(&"softcap-tanh-attention-score"),
        "softcap tanh attention score"
    );
    // D (2 new): fold-query-tokens-for-occupancy + hardcoded-fast-path-jit-specialization
    assert!(
        ids.contains(&"fold-query-tokens-for-occupancy"),
        "fold query tokens for occupancy"
    );
    assert!(
        ids.contains(&"hardcoded-fast-path-jit-specialization"),
        "hardcoded fast path jit specialization"
    );
}

#[test]
fn batch23_attention_cubecl_concepts_covered() {
    // Verify the batch-23 attention_cubecl flash-attention cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // A: combined-kv-buffer-keys-then-values (riir-gpu attention_cubecl/mod.rs L107-140 + L576-650)
    assert!(concepts.contains(&"store keys and values as a single contiguous buffer `[keys | values]` with shared stride — halves buffer bindings from 2 to 1 and enables coalesced access via a single kv_off index. The value for position p at dimension d is at kv[kv_half + p * kv_stride + d], where kv_half = kv.len() / 2. Distinct from KV cache layout: this is about the buffer binding shape, not the cache update logic"));
    // A: gqa-kv-head-sharing-bandwidth-reduction (riir-gpu attention_cubecl/mod.rs L150 + L345 + L870)
    assert!(concepts.contains(&"Grouped-Query Attention (GQA): share KV heads across multiple query heads (n_kv_head < n_head) to reduce KV cache memory + bandwidth by n_head/n_kv_head× (e.g. 2× for Gemma 2 2B's 8:4 ratio). Each query head maps to its KV group via kv_group = head_idx * n_kv_head / n_head; the kernel computes kv_off = kv_group * head_dim once per workgroup and reuses it for all K/V reads in that head. Distinct from MHA (1:1) and MQA (all heads share 1 KV head)"));
    // B: softcap-tanh-attention-score (riir-gpu attention_cubecl/mod.rs L188-191 + L373-376 + L894-897)
    assert!(concepts.contains(&"bound raw attention logits to [-softcap, +softcap] via softcap * tanh(score / softcap) — a smooth, differentiable alternative to clamping that prevents extreme Q·K scores from dominating softmax. tanh has a well-defined gradient everywhere (unlike clamp's discontinuous gradient at the boundary). Configurable: softcap == 0.0 disables softcapping → raw scaled dot product (LLaMA/MiniCPM path). Used by Gemma 2 (softcap=50.0)"));
    // D: fold-query-tokens-for-occupancy (riir-gpu attention_cubecl/mod.rs L336-576 + L1552-1573)
    assert!(concepts.contains(&"when n_head < SM_count (small models, speculative decode with seq_len_q > 1), fold multiple query tokens into one workgroup to increase GPU occupancy. Each workgroup processes fold_factor query tokens sequentially for one head. fold_factor(n_head, seq_len_q, workgroup_size) picks the largest divisor of seq_len_q ≤ workgroup_size / n_head. Benefit: (n_head × fold) workgroups vs n_head → fills idle SMs during speculative decode"));
    // D: hardcoded-fast-path-jit-specialization (riir-gpu attention_cubecl/mod.rs L1313-1342)
    assert!(concepts.contains(&"when kernel params match a known config (e.g. Gemma 2 2B: n_head=8, n_kv_head=4, head_dim=256, softcap=50.0), dispatch a hardcoded variant that bakes the constants into the kernel body — the JIT compiler can then constant-fold dimensions into optimized shaders (unrolled loops, known stride multiplies, eliminated per-tile branching). The generic parametric kernel reads dims from a buffer per dispatch, preventing constant-folding. Measured gain: ~24% decode throughput (31.4 → 41.4 tok/s on M3 Max Metal). Distinct from autotune-warps-stages (sweeps configs) — here the config is matched, not searched RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- moe/topk_softmax_kernels.cu:679 + moe/topk_softplus_sqrt_kernels.cu:786): both MoE top-k launchers switch (num_experts) to dispatch per-count template instantiations -- the specialization axis on a second family (expert count rather than head count). RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- commit f1793c1 #27978): mm_ids_helper is generated for ANY n_expert_used -- the specialized helper exists for every count the dispatch can name, not just the popular ones (specialization-surface breadth: bake constants per config, generate the family). RIDER (Batch 117, antirez/ds4 @ 9ab7053 -- ds4_metal.m:42934-42945): pin DERIVED BYTE SIZES alongside logical dims -- gate_row_bytes == 2176, expert_bytes == 4456448, down_row_bytes == 1088 -- because the byte sizes IMPLY THE QUANT BLOCK LAYOUT (2176 = (4096/32)*17 and 17 B/block = the MXFP4 32-values-per-block record), so the pin catches quant-format/stride drift a dims-only check misses without a separate dtype comparison; the fallback is the generic pipeline ternary. RIDER (Batch 117, antirez/ds4 @ 9ab7053 -- ds4.c:54323-54333): the TOPOLOGY-AXIS widening contract -- a schedule gated on measured (device, quant-pair) combos may extend to a new topology only when the topology SHARES THE KERNEL MACHINERY (TP active + MXFP4/MXFP4 arm added beside the CPU-pair arm), unmeasured combos keep the reference schedule, and the invariance is stated at the gate: 'No sampling decision is changed by this dispatch' -- with exact-sampling, the one mode where it could change, explicitly excluded. RIDER (Batch 130 external instance, cloudrift-ai/emmy @ 2aaa0328 -- deplodock/compiler/cuda/runner.py:431-434 + lower.py:243,247-253,283): the codegen twin of this rule -- a fresh translation unit is GENERATED per (M,N,K) with the dims baked as #define M {m} / #define N {n} / #define K {k} ('// Compile-time matrix dimensions for kernel optimization'), so specialization breadth comes from the generator rather than a hand-maintained variant family: the tile-loop count nt=K/{bk} folds to a literal the compiler can fully unroll, and the epilogue is selected by a PREPROCESSOR ALIGNMENT PREDICATE -- #if (M % {bm} == 0 && N % {bn} == 0) emits the bounds-free store macro while the #else arm keeps the per-element-checked stores IN THE SAME SOURCE, so the generic fallback is a preprocessor arm of one kernel, not a second dispatch target. Honest magnitude (blog-reported, accompanies the pin): the bake moves 1024 from 98% to 101% and 4096 from 100% to 101% of cuBLAS -- real but marginal next to the per-size tile-table sweep, which carries the gains. Scope discipline: fresh-compile-per-shape re-pays nvcc per invocation, so it is sensible for COMMON sizes only (powers of two, model dims), never every M/N/K -- the generator's runner indeed re-emits per benchmark invocation."));
}

#[test]
fn batch24_rules_present() {
    // Verify the 4 batch-24 rules across A/B/C/E.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (1 new): widen-index-multiply-to-64bit
    assert!(
        ids.contains(&"widen-index-multiply-to-64bit"),
        "widen index multiply to 64bit"
    );
    // B (1 new): dual-accumulator-single-reduction-sweep
    assert!(
        ids.contains(&"dual-accumulator-single-reduction-sweep"),
        "dual accumulator single reduction sweep"
    );
    // C (1 new): dual-grad-same-pass
    assert!(ids.contains(&"dual-grad-same-pass"), "dual grad same pass");
    // E (1 new): broadcast-uniform-skip-reduce
    assert!(
        ids.contains(&"broadcast-uniform-skip-reduce"),
        "broadcast uniform skip reduce"
    );
}

#[test]
fn batch24_cudarc_backward_concepts_covered() {
    // Verify the batch-24 cudarc_kernels backward cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // A: widen-index-multiply-to-64bit (riir-gpu cudarc_kernels/backward.rs L416/L474/L478/L515/L518)
    assert!(concepts.contains(&"when a flat index product (position × stride) can exceed 2^31 on large buffers (long-context KV caches, big batched grads), widen ONLY the address-producing multiply to 64-bit by casting one operand — keep loop counters and small dims 32-bit for register economy. int32 index overflow in CUDA wraps silently → OOB reads / wrong elements with no error. Appears as `(long)t * kv_stride` at every KV cache access. Distinct from upcast-before-reduction (fp32 precision) — this is integer address width"));
    // B: dual-accumulator-single-reduction-sweep (riir-gpu cudarc_kernels/backward.rs L61-83)
    assert!(concepts.contains(&"fuse two block reductions over the same data into one sweep: keep two partial accumulators in the same strided loop (each element loaded once, feeding both), and reduce both shared-memory arrays inside the SAME stride tree — halves global loads and halves the __syncthreads() barrier count vs two sequential reductions. RMSNorm backward needs sum(x²) and dot(grad_y·γ·x) over identical elements — the canonical case. Distinct from split-sign-accumulators-defer-subtraction (ILP chains within one dot) — this is barrier + load sharing across two different reductions"));
    // C: dual-grad-same-pass (riir-gpu cudarc_kernels/backward.rs L108-126 + L137-155 + L169-186 + L345-369)
    assert!(concepts.contains(&"a product-rule activation out = f(gate)·g(other) has TWO partial derivatives that share the same inputs and the same transcendental evaluation (sigmoid/silu) — emit both from ONE kernel: load each element once, evaluate the activation + its derivative once, write both grad buffers. Applies to every gated backward (SwiGLU: grad_gate+grad_up; z-gating: grad_pre+grad_z; sigmoid output-gate: grad_pre+grad_gate; beta/decay pair). Distinct from fused-backward-chain (producer→consumer chain stays in registers) and fuse-gate-up-gemv-activation (forward sibling GEMVs) — this is the backward fan-out: sibling grads sharing one evaluation"));
    // E: broadcast-uniform-skip-reduce (riir-gpu cudarc_kernels/backward.rs L492-501 — the T7.2 bug fix)
    assert!(concepts.contains(&"when a per-thread value is derived entirely from already-reduced broadcasts (every thread read the same smem[0] + the same shared arrays), it is thread-uniform: do NOT re-reduce it — the tree sums the identical value blockDim.x times, multiplying the result by the block size (a correctness bug, not just wasted barriers). Broadcast instead: thread 0 writes smem, one barrier, every thread reads. Caught as the T7.2 validation bug in riir-gpu's softmax backward (z came out blockDim.x× too large). Distinct from single-plane-sufficient-reduction (dispatch-level redundant planes) — this is intra-block uniformity after a reduction"));
}

// ════════════════════════════════════════════════════════════════════
// Batch 25 (Research 026) — riir-gpu production kernels: Gemma cluster
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch25_rules_present() {
    // Verify the 5 batch-25 rules across B/C/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (2 new): device-side-argmax-minimal-download + hot-swap-weights-preserve-autotune-caches
    assert!(
        ids.contains(&"device-side-argmax-minimal-download"),
        "device side argmax minimal download"
    );
    assert!(
        ids.contains(&"hot-swap-weights-preserve-autotune-caches"),
        "hot swap weights preserve autotune caches"
    );
    // B (2 new): monotone-transform-elision-under-argmax + tree-reduction-depth-over-sequential-accumulate
    assert!(
        ids.contains(&"monotone-transform-elision-under-argmax"),
        "monotone transform elision under argmax"
    );
    assert!(
        ids.contains(&"tree-reduction-depth-over-sequential-accumulate"),
        "tree reduction depth over sequential accumulate"
    );
    // C (1 new): fan-out-shared-input-single-sync
    assert!(
        ids.contains(&"fan-out-shared-input-single-sync"),
        "fan out shared input single sync"
    );
}

#[test]
fn batch25_gemma_cluster_concepts_covered() {
    // Verify the batch-25 Gemma cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // D: device-side-argmin-minimal-download (riir-gpu gemma2_cubecl/gpu_forward.rs L319-346 + L348-423).
    // Batch 129 appended a rider to this concept (ds4-metal 66b0e3f), so the
    // B23-style FULL-STRING pin became prefix + rider asserts (the B125
    // full-string regeneration lesson, prefix form -- the original text is
    // pinned verbatim, the rider pinned by its opening marker).
    let argmax = concepts.iter().find(|c| c.starts_with("when the host needs only a tiny derived result of a large GPU array (argmax index, top-k, threshold count), compute it ON DEVICE with a reduction kernel and download only the tiny result — 4 bytes (1 u32) instead of the full array (vocab_size × 4 ≈ 1 MB at 256K vocab). Greedy decode is the canonical consumer: ~250,000× smaller per-token boundary crossing; a 50-prompt + 500-decode session downloads ~50 MB instead of ~550 MB. Distinct from offload-cpu-bottleneck-to-gpu-kernel (CPU work moved to GPU) — the CPU side isn't slow, the DOWNLOAD feeding it is the cost; and from chain-dispatches-single-sync-point (sync count) — this is download size")).expect("device-side-argmax-minimal-download concept prefix missing");
    assert!(
        argmax.contains("RIDER (Batch 129 external confirmation + facet"),
        "batch-129 argmax rider missing"
    );
    // B: monotone-transform-elision-under-argmax (riir-gpu gemma2_cubecl/gpu_forward.rs L208-217)
    // Batch 170 appended a rider (the LGH logits-plane elision), so the
    // FULL-STRING pin became prefix + rider asserts (the B125/B129 lesson).
    let monotone = concepts.iter().find(|c| c.starts_with("a strictly monotone post-transform (softcap cap·tanh(x/cap), clamps, positive rescalings) does not change the argmax — skip it entirely when the only consumer is greedy decoding / top-k / any order-only query. The GPU decode path returns the raw logits WITHOUT final logit softcapping because tanh is monotonically increasing: argmax(softcap(x)) == argmax(x); the value-consuming CPU path still applies it. Elides a full-array transcendental pass (256K tanh evals per decode token). Distinct from softcap-tanh-attention-score (applying softcap in-kernel) — this elides it under order-only consumers; distinct from optional-bias-branch (absence) — the transform exists but order is invariant under it")).expect("monotone-transform-elision concept prefix missing");
    assert!(
        monotone.contains("RIDER (Batch 170, mlxfast-gemma4"),
        "batch-170 LGH plane-elision rider missing"
    );
    // C: fan-out-shared-input-single-sync (riir-gpu gemma2_cubecl/dispatch.rs L69-122)
    assert!(concepts.contains(&"when N INDEPENDENT sibling kernels consume the same input (Q/K/V GEMVs on the same hidden state, per-head projections), create the input handle ONCE, clone it into each launch (cheap ref-count), queue all N dispatches, then read the outputs back-to-back — the first read executes the whole batched submission, so 1 upload + 1 effective sync instead of N uploads + N syncs. NOT the Bench 437 known-bad fan-out (one dispatch's output feeding multiple later dispatches — intermediates accumulate): here outputs are independent and read once at the end, no intermediates outlive the batch. Distinct from chain-dispatches-single-sync-point (dependent dispatches, sync amortization only — no shared upload) and from fuse-gate-up-gemv-activation (fuses siblings into ONE kernel — tighter but more invasive; this keeps existing kernels unchanged)"));
    // D: hot-swap-weights-preserve-autotune-caches (riir-gpu gemma4_cubecl/mod.rs L215-235)
    assert!(concepts.contains(&"when a training loop refreshes weights every step (LoRA merge → new handles), replace ONLY the weight handles + CPU-side copies — keep the compute client, autotune cache (per-(m,n) benchmarked variants), JIT-compiled kernel cache, precomputed tables, and layer params. A full reconstruct re-runs autotune benchmarking + JIT compilation (~seconds) per step; the hot-swap path pays only the buffer uploads. Distinct from freeze/thaw weight snapshots — this amortizes runtime caches across weight changes, not weight versions"));
    // B: tree-reduction-depth-over-sequential-accumulate (riir-gpu gemma2_cubecl/dispatch.rs L881-893 — Issue 424 fix)
    assert!(concepts.contains(&"when the same dot product can be computed by either a sequential-accumulate kernel (tiled GEMM: error chain depth = n) or a tree-reduction kernel (plane/subgroup lanes: depth n/lanes + log2(lanes)), prefer the tree for gradient paths — accumulated rounding error grows with summation depth, and at 26-layer backward scale, sequential depth 2304 vs tree depth ≈77 gives ~30× better numerical accuracy, enough to prevent outright gradient divergence. A perf-neutral kernel swap changing only the summation order. Distinct from upcast-before-reduction (raising input precision) — this changes the reduction's association shape at equal dtype"));
}

// Batch 26 (Research 027) — riir-gpu production kernels: wgpu forward-orchestration cluster
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch26_rules_present() {
    // Verify the 8 batch-26 rules across A/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (5 new): the dispatch-side hazard + orchestration family
    assert!(
        ids.contains(&"per-slot-uniform-buffer-write-ordering"),
        "per slot uniform buffer write ordering"
    );
    assert!(
        ids.contains(&"derive-params-from-global-invocation-id"),
        "derive params from global invocation id"
    );
    assert!(
        ids.contains(&"merge-dispatches-into-single-compute-pass"),
        "merge dispatches into single compute pass"
    );
    assert!(
        ids.contains(&"batched-downloads-single-sync-cycle"),
        "batched downloads single sync cycle"
    );
    assert!(
        ids.contains(&"chunk-encoder-submits-below-driver-limit"),
        "chunk encoder submits below driver limit"
    );
    // A (3 new): the host-side resource discipline family
    assert!(
        ids.contains(&"prebuilt-bind-groups-fixed-buffer-combos"),
        "prebuilt bind groups fixed buffer combos"
    );
    assert!(
        ids.contains(&"host-cache-immutable-device-weights"),
        "host cache immutable device weights"
    );
    assert!(
        ids.contains(&"defer-readback-until-after-submit"),
        "defer readback until after submit"
    );
}

#[test]
fn batch26_wgpu_forward_concepts_covered() {
    // Verify the batch-26 wgpu forward-orchestration concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // D: per-slot-uniform-buffer-write-ordering (riir-gpu forward/mod.rs L141-148 + L361-390 + forward_buffers.rs L550-557)
    assert!(concepts.contains(&"host-side queue.write_buffer calls all flush BEFORE the recorded encoder executes — N dispatches in one encoder sharing ONE uniform buffer all read the LAST write (per-head attention params, per-target LoRA params, per-slot matmul params silently corrupted; the Issues 366/367/368 lineage). Give each dispatch slot its own small uniform buffer (per head / per adapter / per target / per QKV slot), each wired into its own pre-built bind group, so concurrent writes in one encoder cannot collide. Distinct from same-queue-implicit-ordering-no-sync (queue-level dispatch ordering) — this is the uniform-write channel racing the encoder; the in-kernel escape is derive-params-from-global-invocation-id"));
    // D: derive-params-from-global-invocation-id (riir-gpu forward/batched_dispatch.rs L280-350)
    assert!(concepts.contains(&"instead of writing a per-item parameter (query position, head index, row offset) to a uniform from the host — which under the write_buffer ordering bug forces an encoder + submit PER ITEM — encode the parameter in the dispatch grid: flatten (head, item) into workgroup IDs and derive it in-kernel from global_invocation_id. The batched attention path dispatches n_head × seq_len workgroups in ONE call with causal masking + GQA mapping computed inside the kernel — zero per-position uniform writes, zero per-position submits (vs the encoder-per-position fallback paying ~1 submit/position). Distinct from fold-query-tokens-for-occupancy (goal: fill idle SMs — here the goal is eliminating the host-side parameter channel) and from grid-stride-loop (iteration coverage — the grid IS the parameter)"));
    // D: merge-dispatches-into-single-compute-pass (riir-gpu forward/adapter_merge.rs L410-424 + L654-770)
    assert!(concepts.contains(&"dependent dispatches queued in ONE compute pass (set_pipeline/set_bind_group/dispatch × N inside a single begin/end) execute in order with memory ordering between them — the consumer dispatch reads the producer's writes without a separate pass, and N−1 pass begin/end boundaries disappear. The QKV LoRA path merges 6 dispatches (3× A pass + 3× B pass) into one pass; pure input-save copies ride the copy channel (encoder.copy_buffer_to_buffer) outside any pass. Distinct from chain-dispatches-single-sync-point (command-buffer-level sync amortization) — this is pass-level merging; composes with per-slot-uniform-buffer-write-ordering (all uniform writes precede the pass, into per-slot buffers) RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- cache_kernels.cu:456-474): N per-layer cache inserts merge into ONE launch over a pointer-array workspace (grid = (num_tokens, num_layers), each layer rebinds its cache base from kv_cache_ptrs[layer]) -- the pointer-array indirection replacing per-layer launches. RIDER (Batch 142 external instance, cloudrift-ai/emmy @ 321288d8 Apache-2.0 -- the Gemma4-12B blog, quote authority): the launch-count regression is the measurable INTEGRATION-cost signature -- a vLLM plugin boundary runs ~1100 launches/step vs stock ~650 (attention seam + unfused norm/stat/scale glue where torch.compile fuses), worth 0.3-1.0 ms/token (the conclusion's 1-5% of decode performance; the intro's 2-5% self-contradicts -- cite the conclusion). Count LAUNCHES PER STEP, not just kernel time, when pricing an integration boundary; decode stays memory-bound so the loss is pure dispatch overhead, not kernel time."));
    // D: batched-downloads-single-sync-cycle (riir-gpu buffer.rs L263-280 — Issue 421 P0.5)
    assert!(concepts.contains(&"N GPU→CPU readbacks share ONE command encoder, ONE queue.submit, ONE map_async, and ONE device.poll(Wait) — pack each request into a contiguous, non-overlapping, 4-byte-aligned region of a single grow-only staging buffer, copy all sources in one encoder, map the whole range once, and slice per request. Sync overhead drops from N to 1 — the dominant cost in launch-overhead-bound paths (measured ~1.28 ms per sync regardless of data size; the 4-buffer LoRA-Muon step saves ~3.8 ms/step). Distinct from device-side-argmax-minimal-download (download SIZE) and chain-dispatches-single-sync-point (dispatch sync) — this is readback sync COUNT with the staging-layout + single-map mechanics"));
    // D: chunk-encoder-submits-below-driver-limit (riir-gpu forward/mod.rs L1403-1411 + L1456-1458)
    assert!(concepts.contains(&"drivers impose a compute-pass-per-encoder ceiling (Metal: a few dozen — past it the encoder DEADLOCKS, a hard failure not a slowdown) and a separate stall from accumulating too many write_buffer + command buffers before submit. Chunk recorded work so each encoder stays ~48 passes (~24 passes/position × chunk_size 2), submit each chunk IMMEDIATELY, and make the chunk size env-tunable (tiny-model training loops shrink it, long-prefill raises it). Distinct from chain-dispatches-single-sync-point (a sync-count CHOICE) — this chunking is a driver-limit-forced submission granularity: correctness bound first, tuning knob second"));
    // A: prebuilt-bind-groups-fixed-buffer-combos (riir-gpu forward/forward_buffers.rs L530-548 + L702-703)
    assert!(concepts.contains(&"a bind group binds BUFFERS, not data — when a dispatch site's buffer combination is fixed for the pipeline's lifetime (attention reads head_query + key_cache[layer] + value_cache[layer] + head_output, the residual add reads hidden + residual + temp_out, each LoRA adapter has fixed input/output buffers), the bind group is a CONSTANT: build all of them once at init ([n_layer][n_head] attention BGs, [n_layer × 6] adapter BGs, the fixed elementwise combos) and only set the pre-built handles on the hot path (~768 create_bind_group calls per forward pass → 0). Dynamic-buffer combos still build per call. Distinct from hot-swap-weights-preserve-autotune-caches (amortizing runtime caches across weight changes) — the buffers here never change identity, only contents"));
    // A: host-cache-immutable-device-weights (riir-gpu forward/mod.rs L111-114 + L869-873 + L1270-1288)
    assert!(concepts.contains(&"constants that live on the GPU but are needed host-side every call (embedding tables wte/wpe, norm gammas, small bias vectors) are cloned to CPU ONCE at init — per-call download_f32 of the same bytes cost 2 full GPU sync stalls per forward pass, measured as the #1 forward bottleneck. The host copy then serves cheap gathers (token embedding lookup = row slice + add, auto-vectorized) whose result is uploaded in ONE write. Apply the reverse test too: any per-layer/per-token download of a weight-shaped buffer (a_log, dt_bias, linear_norm) is this rule unapplied. Distinct from precompute-scalar-reduction (hoisting an invariant KERNEL reduction) — this hoists the DEVICE READ of immutable data out of the hot call"));
    // A: defer-readback-until-after-submit (riir-gpu forward/mod.rs L291-295 + L1173-1183 + adapter_merge.rs L576-578)
    assert!(concepts.contains(&"never download (map + read) GPU buffers while an encoder referencing them is still under construction — the recorded work has not executed, so the read returns PRE-submission stale state, and any correction uploaded mid-construction is later overwritten by the recorded pass when it runs; the readback's internal submit also breaks the encoder being built. Instead RECORD the needed fixups (small correction list of (buffer, op) pairs), finish + submit the encoder, then download FRESH data, apply, and upload back. Distinct from no-blocking-sync-in-async-pipeline (don't block the host thread) — this is about WHAT a mid-construction read returns (stale state), independent of blocking"));
}

#[test]
fn batch27_rules_present() {
    // Verify the 6 batch-27 rules across A/C/D.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (3 new): the uniform-distribution + destination family
    assert!(
        ids.contains(&"staging-packed-uniforms-encoder-ordered-copies"),
        "staging packed uniforms encoder ordered copies"
    );
    assert!(
        ids.contains(&"identical-uniforms-share-single-slot"),
        "identical uniforms share single slot"
    );
    assert!(
        ids.contains(&"per-layer-strided-grad-destination"),
        "per layer strided grad destination"
    );
    // C (1 new): the sibling-fusion family
    assert!(
        ids.contains(&"fuse-sibling-target-grads-single-kernel"),
        "fuse sibling target grads single kernel"
    );
    // A (2 new): the buffer-lifetime family
    assert!(
        ids.contains(&"persistent-zeros-source-encoder-copy-clear"),
        "persistent zeros source encoder copy clear"
    );
    assert!(
        ids.contains(&"snapshot-transient-grad-for-deferred-download"),
        "snapshot transient grad for deferred download"
    );
}

#[test]
fn batch27_backward_concepts_covered() {
    // Verify the batch-27 backward-orchestration concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // D: staging-packed-uniforms-encoder-ordered-copies (riir-gpu backward/mod.rs L15 + L1293-1295 + L1505-1512 + staging.rs L31-38)
    assert!(concepts.contains(&"pack every uniform parameter for the whole pass into ONE 16-byte-aligned CPU staging vec, upload it with ONE queue.write_buffer, then move each dispatch's slice into the shared uniform buffer via encoder.copy_buffer_to_buffer recorded immediately before the consuming dispatch — a copy on the encoder timeline lands exactly between the surrounding dispatches, so ONE shared uniform buffer can safely change value per dispatch (per-layer dims, per-position offsets). Hundreds of small per-dispatch queue writes collapse to one packed upload; queue writes alone would all flush pre-encoder (last-write-wins). Distinct from per-slot-uniform-buffer-write-ordering (N buffers so writes cannot collide) — this is the 1-buffer encoder-timeline alternative; composes with identical-uniforms-share-single-slot"));
    // D: identical-uniforms-share-single-slot (riir-gpu backward/mod.rs L192-196 + L1447-1452 + backward/dispatch.rs L315-320)
    assert!(concepts.contains(&"the write_buffer last-write-wins hazard only bites when the writes DIFFER — if every dispatch sharing a uniform buffer in one encoder writes byte-identical params (all batched elementwise adds use count = seq_len × n_embd), the last write is idempotent with all earlier ones and flush order is moot: share ONE uniform buffer + ONE bind group instead of N (and in staging terms, push the value once and reuse the byte offset — staging data is read-only after upload). The safety contract must live at the API boundary: 'caller must provide a unique uniform if the count differs across calls in the same encoder'. Distinct from per-slot-uniform-buffer-write-ordering (the hazard rule) — this is the exemption; diverging params need per-slot buffers or staging-packed encoder-ordered copies"));
    // A: persistent-zeros-source-encoder-copy-clear (riir-gpu backward/mod.rs L235-241 + L267-268 + backward/dispatch.rs L621-631 + L1106-1110 + backward_dflash/mod.rs L309-310 + L618)
    assert!(concepts.contains(&"clearing an accumulator buffer between dispatches inside one encoder must happen on the encoder timeline: keep a device buffer pre-zeroed once at init (COPY_SRC, sized to the max clear target) and clear via encoder.copy_buffer_to_buffer from it — a queue.write_buffer of zeros flushes before the WHOLE encoder runs, so every layer's clear lands up front and layer 1 accumulates on top of layer 0's values. Each clear is a device-side copy (zero host→device bandwidth; wgpu has no buffer-fill API so the zeros source IS the fill); if a target is larger than the zeros source, clear in chunks. Distinct from per-slot-uniform-buffer-write-ordering (uniform slots) — this is data-buffer re-init between dispatches; distinct from staging-packed-uniforms (param distribution) — this distributes a constant fill"));
    // C: fuse-sibling-target-grads-single-kernel (riir-gpu backward/dispatch.rs L678-685 + L707 + L849-851)
    assert!(concepts.contains(&"when N sibling targets (the Q/K/V projections) each need the same two backward kernels, fuse per KERNEL KIND, not per target: one dispatch computes the summed input-gradient in-kernel (d_input = Σ Rᵢᵀ·gᵢ — one output write, no separate combine-add pass) and one dispatch computes all N parameter gradients in a single grid sized N × num_blocks × params_per_block. Pack every adapter's R matrix, gradient, and the SHARED saved input (all siblings saved the same input — bind it once) into ONE wide bind group. 6 dispatches → 2 per layer. Distinct from dual-grad-same-pass (two partials of ONE op sharing a subexpression — these are grads of N different weight sets sharing only the input) and from fan-out-shared-input-single-sync (forward 1→N mirror) — this is N grads → 1 collected output pair via kernel-body fusion, not pass merging"));
    // D: per-layer-strided-grad-destination (riir-gpu backward_dflash/mod.rs L205-207 + L1533-1541)
    assert!(concepts.contains(&"when a downstream consumer needs a gradient partitioned per layer (per-layer optimizer steps, per-layer analysis), don't accumulate into one shared buffer and de-interleave afterward — write directly to layer-major strided offsets (layer_idx × seq_len × n_embd + pos × n_embd) in one pre-partitioned buffer, selected at dispatch time. The SAME dispatches that would accumulate into the shared buffer produce the per-layer split for free (no second pass, no post-hoc scatter), and downstream consumers read contiguous layer slices instead of strided extraction. Runtime-switchable: the destination is chosen per pass, so shared-accumulator and per-layer modes share one code path. Distinct from dual-grad-same-pass (two grads one kernel — here one grad gets consumer-selected destinations) and from layout-permute-for-coalescing (coalescing — this eliminates a re-pass)"));
    // A: snapshot-transient-grad-for-deferred-download (riir-gpu backward/dispatch.rs L885-897 + backward/mod.rs L315-319)
    assert!(concepts.contains(&"a gradient needed host-side often lives in a temp buffer that later dispatches in the same encoder overwrite — snapshot it at the correct dataflow point with an encoder-ordered copy into a persistent per-adapter buffer (the copy lands exactly between the producing dispatch and the overwrite), then perform the actual host download after the encoder is submitted. The snapshot pins the value's lifetime; the deferral keeps the submission single. Distinct from defer-readback-until-after-submit (the TIMING rule — read after submit): this is the LIFETIME half, without which the deferred read finds the value already destroyed; composes with persistent-zeros-source-encoder-copy-clear (same timeline trick, value pinning vs clearing)"));
}

#[test]
fn batch28_rules_present() {
    // Verify the 10 batch-28 rules across B/C/D/E/G/L.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // G (2 new): the deltanet decode/prefill recurrence family
    assert!(
        ids.contains(&"row-owned-register-blocked-recurrence"),
        "row owned register blocked recurrence"
    );
    assert!(
        ids.contains(&"reverse-running-decay-weight"),
        "reverse running decay weight"
    );
    // E (2 new): the plane/reduction + broadcast-scatter family
    assert!(
        ids.contains(&"workgroup-equals-plane-native-reduction"),
        "workgroup equals plane native reduction"
    );
    assert!(
        ids.contains(&"smem-broadcast-scalar-then-parallel-scatter"),
        "smem broadcast scalar then parallel scatter"
    );
    // D (3 new): the dispatch-shape family
    assert!(
        ids.contains(&"token-flattened-elementwise-modulo-shared-weights"),
        "token flattened elementwise modulo shared weights"
    );
    assert!(
        ids.contains(&"split-decay-outputs-eliminate-readback"),
        "split decay outputs eliminate readback"
    );
    assert!(
        ids.contains(&"nonlinearity-forces-raw-input-buffers"),
        "nonlinearity forces raw input buffers"
    );
    // C (1 new), B (1 new), L (1 new)
    assert!(
        ids.contains(&"fused-conv-state-shift-dot-silu"),
        "fused conv state shift dot silu"
    );
    assert!(
        ids.contains(&"softplus-piecewise-saturation-clamp"),
        "softplus piecewise saturation clamp"
    );
    assert!(
        ids.contains(&"carry-dual-layout-dead-slot"),
        "carry dual layout dead slot"
    );
}

#[test]
fn batch28_deltanet_concepts_covered() {
    // Verify the batch-28 deltanet CubeCL cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // G: row-owned-register-blocked-recurrence (riir-gpu deltanet_cubecl.rs L255-294 + L264-267 + L338-386)
    assert!(concepts.contains(&"rows of a decode-time recurrent state matrix are independent within one step — decompose the per-head state ownership to (head, row) granularity so each workgroup OWNS a row: load it into registers once (each lane register-blocking d/plane strided columns), run every algebra step (decay, retrieve, delta, update, readout) on registers, and write it back exactly once. The memory-bound recurrence drops from 4 global read + 2 write passes over S per step to 1 read + 1 write (~3× cut of the gating quantity), threadgroup traffic and barriers drop to zero, and n_head cubes become n_head × head_dim cubes (32× more resident workgroups for latency hiding). Distinct from chunkwise-parallel-recurrence / recurrent-vs-chunk-dispatch (sequence-axis parallelism for prefill — this is state-matrix parallelism within one decode step) and from row-tile-amortize-input-load (input-load amortization in a GEMV — the owned tile here is the persistent state itself)"), "row-owned concept");
    // E: workgroup-equals-plane-native-reduction (riir-gpu deltanet_cubecl.rs L256-262 + L356-363 + L650-653)
    assert!(concepts.contains(&"size the workgroup to EXACTLY one hardware plane (e.g. 32 threads on Metal/CUDA) and compensate the lost logical width via register blocking (each lane holds width/plane elements) — the native shuffle reduction then covers the FULL logical reduction, broadcasts the result to every lane for free, and no threadgroup memory or barrier exists at all. This inverts the usual choice of sizing the workgroup to the data width (128 threads), which forces every full reduction to bridge plane boundaries through a strided shared-memory tree (up to 3.2× effective-bandwidth cost + ~2304 barriers per head); a reduction scoped smaller than the workgroup is also a silent-wrong-answer hazard (Issue 610: plane_sum caught 1/4 of the dot). Distinct from warp-shuffle-intrinsics-for-sub-warp-reduction (use shuffles when the block is already ≤ 32 — this is the launch-geometry corollary that makes it hold by construction) and from single-plane-sufficient-reduction (where to place a reduction — this is how to size the workgroup so the native path covers everything)"), "plane-sized workgroup concept");
    // E: smem-broadcast-scalar-then-parallel-scatter (riir-gpu deltanet_cubecl.rs L165-180, Issue 616)
    assert!(concepts.contains(&"after a workgroup-wide reduction, the single thread that holds the result must not also perform the entire dependent scatter — publishing the scalar through the already-reduced shared slot (one barrier) lets EVERY thread write its own column/element of the dependent update in parallel: 1 global write per thread instead of width serial writes from one thread with all others idle. The reduced slot doubles as the broadcast channel — the leader overwrites it after every lane has read the reduction result. Distinct from broadcast-uniform-skip-reduce (the read side — uniform values are broadcast, not reduced; this is the write side: broadcast the reduced scalar so the scatter parallelizes) and from fan-out-shared-input-single-sync (N dispatches sharing one host upload — this is intra-workgroup scalar fan-out)"), "broadcast-then-scatter concept");
    // D: token-flattened-elementwise-modulo-shared-weights (riir-gpu deltanet_cubecl.rs L922-993, Issue 637 T5/638)
    assert!(concepts.contains(&"a per-token elementwise launch inside a prefill loop (P tokens × per-layer = thousands of dispatches, each paying ~25 host-side allocations + launch latency) collapses to ONE dispatch when the op has no cross-token coupling: flatten [p, n_head] into a single 1-D index space, index the per-token buffers flat (i), and consume the per-head weight tables via i % n_head — the weights repeat every n_head entries while the token buffers do not. Bit-identical to the sequential form by construction (every output element is an independent function of its own inputs). Also fixes the workgroup ceiling: the single-token kernel's one-workgroup-of-n_head-threads shape exceeds the max workgroup size at scale; the flattened form uses fixed 256-thread workgroups. Distinct from batched-upcast-map (pure wide elementwise map — this adds the mixed-arity modulo-indexed shared-weights layout) and from merge-dispatches-into-single-compute-pass (dependent dispatches in one pass — these were independent launches collapsed into one index space)"), "token-flattened concept");
    // C: fused-conv-state-shift-dot-silu (riir-gpu deltanet_cubecl.rs L398-448)
    assert!(concepts.contains(&"a decode-time stateful conv (sliding window + depthwise dot + activation) is ONE kernel per step, not three passes with intermediate buffers: each channel's thread shifts the sliding window IN PLACE (a ks-1 element slide, no new window allocation), computes the depthwise dot directly from the freshly shifted state, applies the activation, and writes the result back over the input buffer. Zero intermediate buffers, one dispatch, and the window state persists in global memory across decode steps — the host never touches it. Distinct from stateful-shift-via-cat-prefix (the host-side allocating window update — this is the in-kernel in-place slide fused with the dot + activation) and from fused-activation-quantize-for-gemv-consumer (activation fused at the consumer edge — here it is fused into the stateful producer; note the batched/prefill form CANNOT alias in-place, see nonlinearity-forces-raw-input-buffers)"), "fused conv concept");
    // B: softplus-piecewise-saturation-clamp (riir-gpu deltanet_cubecl.rs L863-874 + L970-980)
    assert!(concepts.contains(&"softplus ln(1 + exp(x)) overflows (exp → inf → ln(inf)) for large x and wastes precision for very negative x — evaluate the piecewise form: return x for x > 20 (softplus equals x to f32 precision), 0 for x < -20 (the exponential underflows the sum), and the transcendental only inside the safe band. Pin the thresholds to the CPU reference implementation so GPU and CPU outputs stay bit-comparable under test; when the result feeds an exp() decay gate on a persistent recurrent state, the unclamped form does not just lose precision — one inf poisons the state permanently. Distinct from expm1-for-near-zero-exp-minus-one (cancellation near zero — this is far-tail saturation) and exp2-log2-base2 (base-2 conversion — this is domain clamping)"), "softplus clamp concept");
    // G: reverse-running-decay-weight (riir-gpu deltanet_chunked_cubecl.rs L640-712)
    assert!(concepts.contains(&"an intra-chunk pairwise decay matrix inter_decay[t,j] = Π_{i=j+1..t} α_i does not need to exist as a matrix when its consumer is a streaming accumulation: iterate j from t DOWNWARD carrying one running scalar — start at 1 (the empty product inter_decay[t,t]), consume the weight for position j, then multiply by α[j] to obtain the weight for j-1. O(C) multiplies per token instead of O(C) per (t,j) pair (or a cumsum+exp materialization pass), zero intermediate memory, and the downward direction is load-bearing — it anchors the empty product so multiply-after-use composes correctly. Distinct from cumsum-gate-to-decay-matrix / pairwise-distance-decay-matrix (the MATERIALIZE-the-matrix approach feeding a batched matmul — this is its dematerialized inverse when the consumer streams) and from chunkwise-parallel-recurrence (the chunk split itself — this is the decay strategy inside the intra-chunk part)"), "reverse-running-decay concept");
    // D: split-decay-outputs-eliminate-readback (riir-gpu deltanet_chunked_cubecl.rs L342-349 + L1949-1969)
    assert!(concepts.contains(&"compact packing of heterogeneous outputs into one buffer (total_decay stashed at index C of a [H, C+1] row) is a pessimization when the downstream consumers read different shapes ([H, C] and [H]) — the packed layout forces a device→host→device readback-unpack-reupload, and a consumer reading the packed buffer directly reads the WRONG offset (total_decay of head h-1). Write each component directly into the buffer shape its consumer reads — one kernel, two output buffers, the host round trip disappears entirely. Packing elegance loses to consumer layout: optimize the boundary, not the buffer count. Distinct from per-layer-strided-grad-destination (consumer-selected stride within one buffer — this is consumer-selected buffer SPLIT) and from defer-readback-until-after-submit / batched-downloads-single-sync-cycle (both mitigate readback timing/cost — this eliminates the readback by producing consumer shapes at the source)"), "split-decay concept");
    // L: carry-dual-layout-dead-slot (riir-gpu deltanet_chunked_cubecl.rs L77-98)
    assert!(concepts.contains(&"when a producer's output feeds a consumer with a different but overlapping layout, parameterize the producer's write stride + index offset (two scalars) instead of adding a conversion pass: the prefill chunk-carry can be written compact [ch, ks-1] OR directly into the decode path's [ch, ks] sliding-window layout from the same kernel. The conv_state variant leaves slot 0 unwritten (stale garbage) — safely, because the decode consumer's shift-left provably overwrites slot 0 before any read. The dead-slot tolerance is a consumer-semantics proof, not a hope: the next decode's shift writes every slot it later reads. Distinct from per-layer-strided-grad-destination (runtime Option-selected destinations — this is one parameterized producer writing consumer-native layout) and from alignment-gated-zero-copy-view (gating a view — the dead-slot-tolerance-by-consumer-semantics element is new)"), "carry-dual-layout concept");
    // D: nonlinearity-forces-raw-input-buffers (riir-gpu deltanet_chunked_cubecl.rs L27-47)
    assert!(concepts.contains(&"a sequential kernel's in-place trick (write the activated output back over its own input) is INCOMPATIBLE with batched/widened execution whenever a nonlinearity sits between the producer and a windowed consumer: later tokens' convolution windows read earlier tokens' POST-activation values — silently wrong, same dtype, no error. The batched form needs separate read-only RAW inputs + write-only outputs, and the inter-chunk carry must also hold RAW values for the same reason. This is the negative space of in-place aliasing: before widening a sequential in-place kernel's grid over its sequence axis, check whether any consumer reads a value the nonlinearity would have overwritten. Distinct from inplace-output-alias (when aliasing WORKS — this is when it cannot) and from chunkwise-parallel-recurrence (the chunking shape — this is the buffer contract that makes chunking correct when nonlinearity intervenes)"), "nonlinearity-forces-raw concept");
}

#[test]
fn batch29_rules_present() {
    // Verify the 10 batch-29 rules across A/B/C/D/E/M.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (2 new): the readback/size-gate + handle-cache family
    assert!(
        ids.contains(&"defer-readback-gated-by-live-output-size"),
        "defer readback gated by live output size"
    );
    assert!(
        ids.contains(&"weight-handle-cache-keyed-by-allocation-identity"),
        "weight handle cache keyed by allocation identity"
    );
    // D (3 new): the sync-cost-model + launch-path family
    assert!(
        ids.contains(&"first-readback-pays-queue-drain"),
        "first readback pays queue drain"
    );
    assert!(
        ids.contains(&"pack-scalar-params-into-combined-buffer"),
        "pack scalar params into combined buffer"
    );
    assert!(
        ids.contains(&"launch-unchecked-caller-owned-size-contract"),
        "launch unchecked caller owned size contract"
    );
    // C (2 new): the CODA epilogue family
    assert!(
        ids.contains(&"epilogue-at-register-write-phase"),
        "epilogue at register write phase"
    );
    assert!(
        ids.contains(&"delay-rms-scale-past-gemv-epilogue"),
        "delay rms scale past gemv epilogue"
    );
    // E (1 new), B (1 new), M (1 new)
    assert!(
        ids.contains(&"epilogue-must-be-workgroup-local"),
        "epilogue must be workgroup local"
    );
    assert!(
        ids.contains(&"fast-math-contraction-behind-feature-flag"),
        "fast math contraction behind feature flag"
    );
    assert!(
        ids.contains(&"enum-dispatch-separate-kernel-per-variant"),
        "enum dispatch separate kernel per variant"
    );
}

#[test]
fn batch29_concepts_covered() {
    // Verify the batch-29 decode-path fusion cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // A: defer-readback-gated-by-live-output-size (riir-gpu deltanet_input_proj_fused.rs L17-32 — Bench 437 contrast)
    assert!(concepts.contains(&"deferring all sibling readbacks to one final drain is NOT an unconditional win — gate it on total simultaneously-live output bytes. When the chained outputs are small (e.g. 4 input projections, ~8 KB total), keeping them all GPU-resident to the end is free and one sync point replaces N; when they are large (5 concurrent ~70 KB FFN buffers, ~350 KB live), the deferral itself increases memory pressure and measured 16% SLOWER on Metal — sync per stage and release each buffer instead. The rule converts 'defer readback' from an always-apply transformation into a footprint-gated one; the existing deferral rules carry the timing/mechanism, this carries the failure boundary. Distinct from defer-readback-until-after-submit (the timing rule — read after submit; this is the precondition that decides whether deferral helps at all) and from chain-dispatches-single-sync-point (the dispatch-side mechanism — this is the output-sizing decision that makes the chaining a win rather than a loss)"), "size-gate concept");
    // A: weight-handle-cache-keyed-by-allocation-identity (riir-gpu deltanet_input_proj_fused.rs L279-281 + L301-327 + L361-372)
    assert!(concepts.contains(&"when a GPU offload hook's public API must accept plain CPU weight references (it cannot require callers to thread device handles through existing model code), key a lock-free host-side cache of pre-uploaded device handles by the CPU weight allocation's POINTER IDENTITY — e.g. (pos_bits.as_ptr(), neg_bits.as_ptr()), unique per layer allocation. The hot path is one O(1) map lookup; a cache miss degrades gracefully to an on-the-fly upload + insert, so the hook works with or without a preupload warm-up and never blocks adoption. The keying is what lets the cache be invisible to the API surface. Distinct from host-cache-immutable-device-weights (the upload-once rule — this is the identity-keyed transparent lookup layered on top) and from per-dispatch-pointer-re-extraction (device pointers re-read per dispatch — CPU heap pointers as a persistent key, opposite direction). RIDER (Batch 181, mlxfast-bonsai2 @ cfb35b0d -- Qwen35.swift GDN derived-decay, the DERIVED-SCALAR instance): -exp(A_log) computed ONCE in a tiny kernel ('the same FP32 intermediate and Metal intrinsic as the prework expression... It depends on the model parameter only, never on request data') and cached on parameter identity ('if let cachedDecay, decaySource === aLog { return cachedDecay }') — pointer-keys a DERIVED value (not just an upload) and the owning projection invalidates on parameter update."), "pointer-identity cache concept");
    // D: first-readback-pays-queue-drain (riir-gpu deltanet_input_proj_fused.rs L242-245)
    assert!(concepts.contains(&"after a single dispatch burst on one command queue, the FIRST blocking read drains the whole queue and every subsequent read of an already-completed buffer returns immediately — N sequential readbacks ordered after one burst cost ~1 sync + N small transfers, with no restructuring into a merged download op needed. Sync count is a function of dispatch BURSTS, not read CALLS: don't add per-read defensive syncs, and don't build batched-download machinery when the reads naturally follow one burst. Distinct from batched-downloads-single-sync-cycle (restructures N readbacks into one submit/map/poll cycle — this is the cheaper ordering-only alternative that makes that machinery unnecessary) and from chain-dispatches-single-sync-point (the dispatch-side mechanism — this is the read-side cost model)"), "queue-drain concept");
    // C: epilogue-at-register-write-phase (riir-gpu epilogue/mod.rs L8-26 + gemv_residual_cubecl.rs L103-109 + L24-31)
    assert!(concepts.contains(&"memory-bound elementwise operations (residual adds, scales, activations) should execute while the producer's output is STILL IN REGISTERS, at the existing write phase — not as a separate dispatch that reads the output vector back from global memory. In a plane-cooperative GEMV (one plane per output row), plane_sum leaves the full row dot product in every lane's registers, so the epilogue (read residual[row], add, write) rides lane 0's single global store: one extra scalar global read amortized over the n-length cooperative dot, one dispatch eliminated per use site (2→1; 52 per 26-layer model, 104 combined with the norm+residual fusion). The condition is the register-residency of the produced value at the write phase. Distinct from fuse-residual-via-in-place-output-array (buffer aliasing to bypass a parameter limit — this is register-residency at the producer's write phase) and from merge-dispatches-into-single-compute-pass (host-side pass merging, zero kernel changes — this edits the kernel body)"), "register-write-phase concept");
    // C: delay-rms-scale-past-gemv-epilogue (riir-gpu epilogue/mod.rs L118-133 + L227-262 + coda_primitives_cubecl.rs L20-24)
    assert!(concepts.contains(&"RMSNorm's per-row scalar commutes with the downstream matmul — RMSNorm(x) @ W = r · (x·γ) @ W — so split the norm into (a) extracting r (a partial-norm reduction), (b) applying only the elementwise gamma, and (c) applying r later as a row-scale epilogue of the downstream GEMV, composable with a residual add. Be honest about what this saves: the pure reparameterization alone saves ZERO dispatches (3→3) — the real win is fusing r into the LAST GEMV of the group and eliminating the pre-GEMV write of the normed vector; with handle-to-handle passing the dominant benefit is dispatch/command-buffer overhead, not bandwidth. The win compounds when multiple GEMVs share the same r. Distinct from fold-norm-scalar-into-matmul (folds the scalar into the matmul internals — this keeps the GEMV body untouched and adds the producer-side decomposition) and from fuse-add-rms-norm (fuses residual into a norm kernel — this moves the norm scalar across a GEMV boundary)"), "delayed-rms concept");
    // D: pack-scalar-params-into-combined-buffer (riir-gpu epilogue/mod.rs L48 + L148-154 + coda_primitives_cubecl.rs L51-55 + L144-146 + norm_residual_cubecl.rs L179-181 + L351-354)
    assert!(concepts.contains(&"when the kernel DSL caps Array parameters (~4-5 in CubeCL v0.10), pack ALL launch scalars (eps, dim, derived constants) into ONE small f32 params buffer created per launch on the host — precomputing derived values there (inv_dim = 1/dim once on the CPU, not per-thread in-kernel) — and index it positionally in-kernel. When even a u32 like dim won't fit as a scalar arg, bitcast it through the f32 pack and cast back in-kernel. Couple variant choice to the budget with a total_array_count()-style metadata method so a kernel never exceeds the Array limit. Distinct from staging-packed-uniforms-encoder-ordered-copies (driven by upload batching — this is driven by the kernel argument-count limit), from derive-params-from-global-invocation-id (per-item params encoded in the grid — these are per-launch scalars), and from identical-uniforms-share-single-slot (dedupes identical uniforms — this packs distinct scalars)"), "packed-params concept");
    // M: enum-dispatch-separate-kernel-per-variant (riir-gpu epilogue/mod.rs L47 + L99-103 + L137-154 + gemv_residual_cubecl.rs L76 + L140)
    assert!(concepts.contains(&"when the kernel DSL forbids generic kernel parameterization (no generics in #[cube] kernels) — or where a generic form would pay a runtime branch per element — enumerate the fusion-combination space as a host-level #[repr(u8)] enum, compile ONE specialized kernel function per variant (zero in-kernel branching, each with exactly its own Array count, visible even between dtype twins with byte-identical launchers), and drive buffer plumbing from enum metadata methods (extra_input_count, needs_residual, total_array_count). The enum organizes dispatch; the kernel list stays flat and each body is branch-free. Distinct from dtype-specialized-float-kind / hardcoded-fast-path-jit-specialization (dtype/shape specialization for perf — this is DSL-limitation-driven with the enum-metadata authoring shape) and from op-family-type-dispatch (dispatches between op families — every variant here is the same op with a different epilogue suffix)"), "enum-kernel-variant concept");
    // E: epilogue-must-be-workgroup-local (riir-gpu epilogue/mod.rs L49 + L371-375 + gemv_residual_cubecl.rs L50-55 + norm_residual_cubecl.rs L57-59)
    assert!(concepts.contains(&"an epilogue may only consume data at the granularity the executing workgroup OWNS — no cross-workgroup sync exists to order it after a foreign write. For plane-per-row GEMV that means per-row scalars (residual[row], scale[row] are each one scalar owned by the row's plane); for a norm fusion it means the whole row reduction must complete inside a single workgroup. Ops needing cross-row or cross-workgroup data (attention over KV, anything with a global barrier) cannot be epilogued — which is why a dispatch budget leaves the attention section unchanged while post-attn and MLP sections fuse down. Fusion legality is an ownership question, not just a data-dependency question. Distinct from nonlinearity-forces-raw-input-buffers (buffer-aliasing legality — this is data-ownership scope legality) and from workgroup-equals-plane-native-reduction (workgroup sizing for free shuffles — this is the fusion boundary that scope defines)"), "workgroup-local concept");
    // D: launch-unchecked-caller-owned-size-contract (riir-gpu epilogue/mod.rs L50 + gemv_residual_cubecl.rs L205-226)
    assert!(concepts.contains(&"the checked launch path validates buffer sizes and argument consistency on EVERY dispatch — host-side overhead that dominates when each dispatch is microseconds of work in a 14-dispatches-per-layer × 26-layer decode loop. Compile kernels with launch_unchecked, wrap each in a launcher that takes exact element counts as plain args, and push the size contract into a documented # Safety section (exact per-handle element counts, required hardware features) verified by the caller and by tests. The repeated shape: thin launcher struct + one unsafe launch + an enumerated handle-size contract. Distinct from cuda-graph-scalar-via-device-pointer (streamlines launches by moving scalars to device memory — this elides per-launch validation) and from no-blocking-sync-in-async-pipeline (elides syncs — this elides checks)"), "unchecked-launch concept");
    // B: fast-math-contraction-behind-feature-flag (riir-gpu epilogue/gemv_residual_cubecl.rs L67-75 + L94-101 + L131-139)
    assert!(concepts.contains(&"a numerics-affecting compiler flag (fast-math contraction) must not be enabled globally — it changes rounding everywhere and breaks bit-comparisons and tolerance gates — nor disabled forever (the hot dot-product loop stays uncontracted mul+add on every build). Attach the flag to the KERNEL ATTRIBUTE via a cfg_attr pair gated on a dedicated Cargo feature: the attribute-identical strict-FP compilation stays the byte-identical default, correctness/equivalence gates run strict, and perf builds opt in. The partial += w[k] * x[k] accumulation contracts to FMA only on the opt-in compilation of the same source. Distinct from dtype-specialized-float-kind (dtype specialization — this is compiler-numerics-mode specialization) and from in-kernel numeric transforms like softplus-piecewise-saturation-clamp (algorithm changes — this gates a compiler flag with zero algorithm change) RIDER (Batch 130 external instance, cloudrift-ai/emmy @ 2aaa0328 -- deplodock/compiler/cuda/runner.py:636-642,656-658): the flag pair must be SPLIT, not lumped as 'fast math' -- contraction is the EXACT half, the lossy flags are the other half: fast_math=False (the default) is the IEEE-clean configuration: we still enable --fmad=true so SGEMM gets fused FFMA instructions (without it nvcc emits separate FMUL+FADD and SGEMM throughput halves), but we do NOT pass --use_fast_math, which would also enable --ftz=true and relaxed div/sqrt -- those break exact-FP32 claims for headline numbers (docstring verbatim, with the repo's nvcc_cmd = [nvcc, -O3, --fmad=true] and the conditional --use_fast_math append as the enforcement). FMA fusion has a SINGLE rounding, so an exactness-claiming build keeps contraction ON unconditionally and refuses only ftz/relaxed-math -- the inverse posture of gating contraction itself when bit-identity across builds is the contract. RIDER (Batch 139 external confirmation + boundary, ivanfioravanti/ds4-metal @ 18ca8ec MIT -- speed-bench/qwen38-m5-round2.md:19): a PURE SOURCE RESTRUCTURE can silently flip contraction with NO flag change -- staging eight F16 gate/mix terms ahead per lane ('F16 HC gate/mix with eight terms loaded ahead per lane') re-shaped the multiply-add chain the compiler contracted, and the variant was REJECTED because 'logits differ (fast-math contraction changed with the staged loop)' -- a byte-identity bar, not tolerance. The boundary that makes this a rule and not a scare: in the SAME batch, staging 16 merge partials ahead was safe and won (+0.8%) because the staged values feed an order-preserved accumulation of already-materialized loads, not an FMA chain the compiler re-shapes -- the contraction hazard lives in RESTRUCTURING THE ARITHMETIC EXPRESSION, not in moving the DATA; a staging proposal that only hoists loads is exempt, one that rewrites the accumulation expression is not, and the byte-identity gate is what catches the difference either way."), "fma-flag concept");
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch29_pseudocode_parses() {
    // G1 for Batch 29: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict), like every
    // healable pattern the drafter can emit.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "defer-readback-gated-by-live-output-size",
        "weight-handle-cache-keyed-by-allocation-identity",
        "first-readback-pays-queue-drain",
        "epilogue-at-register-write-phase",
        "delay-rms-scale-past-gemv-epilogue",
        "pack-scalar-params-into-combined-buffer",
        "enum-dispatch-separate-kernel-per-variant",
        "epilogue-must-be-workgroup-local",
        "launch-unchecked-caller-owned-size-contract",
        "fast-math-contraction-behind-feature-flag",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-29 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-29 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-29 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch30_rules_present() {
    // Verify the 6 batch-30 rules across C/D/E/M/N.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // M (1 new): the zero-init frozen-teacher identity
    assert!(
        ids.contains(&"zero-init-side-branch-frozen-base-path"),
        "zero init side branch frozen base path"
    );
    // E (1 new): the counter-derived stateless seed
    assert!(
        ids.contains(&"counter-derived-seed-stateless-noise"),
        "counter derived seed stateless noise"
    );
    // D (2 new): the empty-workset gate + shared pipeline set
    assert!(
        ids.contains(&"predicate-gated-skip-empty-workset-step"),
        "predicate gated skip empty workset step"
    );
    assert!(
        ids.contains(&"share-pipeline-set-across-subsystems"),
        "share pipeline set across subsystems"
    );
    // N (1 new): the 16-byte uniform pad discipline
    assert!(
        ids.contains(&"pad-uniform-pod-to-min-binding-size"),
        "pad uniform pod to min binding size"
    );
    // C (1 new): the fused count
    assert!(
        ids.contains(&"fuse-count-into-mutation-pass"),
        "fuse count into mutation pass"
    );
}

#[test]
fn batch30_concepts_covered() {
    // Verify the batch-30 D2F training cluster concepts.
    let corpus = build_kernel_seed_corpus();
    let concepts: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    // M: zero-init-side-branch-frozen-base-path (riir-gpu dllm/mod.rs L1044 + L1097-1114 + lora/mod.rs L160)
    assert!(concepts.contains(&"to run a FROZEN un-adapted model (a distillation teacher) alongside a trainable adapted model (the student) over the SAME base weights, do not build a second no-adapter forward path or merge the adapter out of a weight copy — initialize the adapter's B matrix to zero (standard LoRA init: A=random, B=zero) and run the SAME adapter-capable forward for both roles: out = W·x + B·(A·x) = W·x exactly, by construction. One code path, one uploaded weight set, both execution modes; the identity is mathematical (B=0 ⇒ side branch ≡ 0), not maintained by discipline or a parallel kernel. Distinct from host-cache-immutable-device-weights (caching uploaded weights — this is mode identity via adapter algebra) and from enum-dispatch-separate-kernel-per-variant (enumerating code paths — this eliminates the mode axis entirely)"), "zero-init teacher concept");
    // E: counter-derived-seed-stateless-noise (riir-gpu dllm/mod.rs L828-842 + L1526-1537 + L39)
    assert!(concepts.contains(&"derive each training iteration's RNG seed from its loop counters packed into disjoint bit fields — ((step as u64) << 32) | epoch — instead of advancing one shared RNG stream through the loop. Every iteration's noise becomes a pure function of its indices: any step replays standalone (debugging, deterministic eval, crash resume), skipped iterations cannot perturb later streams, and the same shape parallelizes (the seed crosses to the GPU as one uniform with the thread index mixed in-kernel — matches the noise_corrupt.wgsl kernel bit-for-bit). Step an inline wrapping-multiply/add LCG (PCG32-style) and extract the high bits (>> 33) — the statistically good half — scaled by a named const reciprocal so CPU and GPU paths stay bit-identical. Distinct from derive-params-from-global-invocation-id (per-thread identity from the grid — this is per-iteration seed from counters; the two compose)"), "counter-seed concept");
    // D: predicate-gated-skip-empty-workset-step (riir-gpu dllm/mod.rs L851-853 + L1243-1245 + L1546-1548 + L485-490)
    assert!(concepts.contains(&"when a stochastic per-step sampler can produce an empty workset (ratio-scheduled masking can draw zero masked positions), gate the ENTIRE dispatch batch — forward, loss, backward, optimizer step — on a host-computed 'any work' integer check at the TOP of the step, before the first dispatch. One comparison elides the full GPU step sequence AND guarantees the count-based loss normalizer (CE-weight / total_masked) never sees 0 — the div-by-zero guard holds by construction instead of by a late guard buried inside the loss function. Composes with fuse-count-into-mutation-pass: the fused count IS the predicate. Distinct from predicate-gated-backend (predicate choosing between kernels — this elides the batch outright) and from skip-all-zero-packed-words-via-or-reduce (in-kernel word skip — this is host-side batch elision)"), "empty-workset gate concept");
    // D: share-pipeline-set-across-subsystems (riir-gpu dllm/mod.rs L1092 + L1134-1148 + L1278)
    assert!(concepts.contains(&"wgpu pipeline creation validates WGSL and builds pipeline-state objects — expensive, and IDENTICAL across subsystems (loss, backward, optimizer) when they share shader modules. Build ONE pipeline set at orchestrator setup, wrap it in Arc, and hand clones to every subsystem constructor + dispatch site: the compile cost is paid exactly once per training run, the per-step cost is a ref-count bump, and the handle count stays constant. This is the cross-subsystem analog of prebuilt-bind-groups (which pre-builds per dispatch site): share the expensive-to-create resource at the constructor seam. Distinct from prebuilt-bind-groups-fixed-buffer-combos (bind groups per site — this is the pipeline set itself shared across constructors) and from hot-swap-weights-preserve-autotune-caches (cache preservation across swaps — this is single construction)"), "shared-pipeline concept");
    // N: pad-uniform-pod-to-min-binding-size (riir-gpu dllm/mod.rs L33-60 + L74-82 + dllm/tests.rs L30-49)
    assert!(concepts.contains(&"WGSL var<uniform> bindings require the bound struct to satisfy the min-binding-size/alignment class — pad every uniform param struct with explicit _pad u32 fields to an exact 4 x u32 = 16 bytes, so ONE fixed-size (size: 16) uniform-buffer helper serves every kernel in the module and bind-group creation can never fail on an incidental 12-byte struct. Pin the layout with size_of tests (== 16 and % 16 == 0 per struct): adding a field without re-padding breaks CI instead of failing on the GPU at runtime. Zero runtime cost — the padding is dead bytes; the discipline is what makes a single shared buffer helper sound. Distinct from staging-packed-uniforms-encoder-ordered-copies (distribution of uniforms — this is per-struct layout) and from vectorized-pack-load (kernel load alignment — this is the binding size class)"), "uniform-pad concept");
    // C: fuse-count-into-mutation-pass (riir-gpu dllm/mod.rs L828-830 + L843-847 + L585-596)
    assert!(concepts.contains(&"when a mutation pass writes a predicate-derived output (a mask, a filter) and its CONSUMER needs the count of affected elements (loss normalization, workset gating), accumulate the count INSIDE the mutation loop instead of re-scanning the output afterwards — the increments ride already-hot cache lines and the second O(n) pass (or a second reduction kernel) disappears. In kernel form the per-element predicate write and its reduction share one launch. The fused count is also the natural predicate for eliding empty steps entirely. Distinct from dual-grad-same-pass (two gradient outputs per pass — this is a count fused into its producing mutation) and from plain host loop-fusion (the count's role as the downstream dispatch/normalization gate is the load-bearing half)"), "fused-count concept");
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch30_pseudocode_parses() {
    // G1 for Batch 30: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict), like every
    // healable pattern the drafter can emit.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "zero-init-side-branch-frozen-base-path",
        "counter-derived-seed-stateless-noise",
        "predicate-gated-skip-empty-workset-step",
        "share-pipeline-set-across-subsystems",
        "pad-uniform-pod-to-min-binding-size",
        "fuse-count-into-mutation-pass",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-30 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-30 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-30 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch31_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (1 new): ragged packed-buffer fused dispatch
    assert!(
        ids.contains(&"ragged-pack-offset-table-fused-dispatch"),
        "ragged pack offset table"
    );
    // L (1 new): SharedKV twin-projection adapter
    assert!(
        ids.contains(&"twin-projection-shared-delta-adapter"),
        "twin projection shared delta"
    );
    // M (2 new): min-rank inert adapter + debug-only label macro
    assert!(
        ids.contains(&"min-rank-inert-adapter-zero-alpha"),
        "min-rank inert adapter"
    );
    assert!(
        ids.contains(&"debug-only-gpu-label-macro"),
        "debug-only gpu label macro"
    );
    // A (3 new): sync result cell + grow-only staging + mapped-at-creation upload
    assert!(
        ids.contains(&"sync-map-result-stack-cell"),
        "sync map result stack cell"
    );
    assert!(
        ids.contains(&"staging-reuse-grow-only-map-prefix"),
        "staging reuse grow only map prefix"
    );
    assert!(
        ids.contains(&"mapped-at-creation-init-upload"),
        "mapped at creation init upload"
    );
    // N (1 new): dtype normalize once at upload
    assert!(
        ids.contains(&"dtype-normalize-once-at-upload"),
        "dtype normalize once at upload"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch31_pseudocode_parses() {
    // G1 for Batch 31: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "ragged-pack-offset-table-fused-dispatch",
        "twin-projection-shared-delta-adapter",
        "min-rank-inert-adapter-zero-alpha",
        "debug-only-gpu-label-macro",
        "sync-map-result-stack-cell",
        "staging-reuse-grow-only-map-prefix",
        "mapped-at-creation-init-upload",
        "dtype-normalize-once-at-upload",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-31 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-31 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-31 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch32_rules_present() {
    // Verify the 11 batch-32a rules across A/B/C/D/H/L.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (3 new): scratch/alloc/hash family
    assert!(
        ids.contains(&"caller-owned-split-scratch"),
        "caller owned split scratch"
    );
    assert!(
        ids.contains(&"incremental-multi-part-hash-no-concat"),
        "incremental multi part hash"
    );
    assert!(
        ids.contains(&"superset-sized-scratch-across-variants"),
        "superset sized scratch"
    );
    // B (3 new): compute/numerics family
    assert!(
        ids.contains(&"sign-branch-stable-sigmoid"),
        "sign branch stable sigmoid"
    );
    assert!(
        ids.contains(&"per-layer-grad-norm-lr-equalization"),
        "per layer grad norm lr"
    );
    assert!(
        ids.contains(&"inplace-grad-accumulate-deferred-postscale"),
        "inplace grad deferred postscale"
    );
    // C (2 new): fusion family
    assert!(
        ids.contains(&"inplace-gate-scale-at-funnel-bottleneck"),
        "inplace gate at bottleneck"
    );
    assert!(
        ids.contains(&"fuse-loss-and-output-grad-one-pass"),
        "fuse loss and grad one pass"
    );
    // D (1 new), H (1 new), L (1 new)
    assert!(
        ids.contains(&"width-invariant-superset-training"),
        "width invariant superset"
    );
    assert!(
        ids.contains(&"active-index-grad-scatter-subset-backward"),
        "active index grad scatter"
    );
    assert!(
        ids.contains(&"bool-fields-over-bitset-hot-reads"),
        "bool fields over bitset"
    );
}

#[test]
fn batch32_concepts_covered() {
    // Verify the batch-32a edge_lora core cluster concepts. Unlike earlier
    // batches (full-string equality), this pins each rule's concept via
    // distinctive substrings located by rule_id — same cross-rule pinning
    // power, immune to prose-evolution churn.
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-32 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: caller-owned-split-scratch (edge_module.rs L173-199 + L301-313)
    let c = concept("caller-owned-split-scratch");
    assert!(
        c.contains("split_at_mut (the borrow checker proves the regions disjoint")
            && c.contains("quarantine it as a documented cold adapter"),
        "split-scratch concept"
    );
    // C: inplace-gate-scale-at-funnel-bottleneck (edge_module.rs L203-212)
    let c = concept("inplace-gate-scale-at-funnel-bottleneck");
    assert!(
        c.contains("IN PLACE on the reused rank-wide intermediate and absorb any broadcast scale")
            && c.contains("placement-of-work at the funnel's narrowest point"),
        "bottleneck gate concept"
    );
    // B: sign-branch-stable-sigmoid (edge_module.rs L182-186 + L328-337)
    let c = concept("sign-branch-stable-sigmoid");
    assert!(
        c.contains("exp's argument is always <= 0"),
        "sign-branch anchor"
    );
    assert!(
        c.contains("overflows to +inf past ~88.7"),
        "sigmoid overflow anchor"
    );
    // A: incremental-multi-part-hash-no-concat (types.rs L476-487 + L381)
    let c = concept("incremental-multi-part-hash-no-concat");
    assert!(
        c.contains("incremental update API")
            && c.contains("the streaming update IS the concatenation"),
        "streaming hash concept"
    );
    // L: bool-fields-over-bitset-hot-reads (types.rs L35-45 + L69-84)
    let c = concept("bool-fields-over-bitset-hot-reads");
    assert!(
        c.contains("REGIME BOUNDARY of packing") && c.contains("read frequency dominates"),
        "bool-regime concept"
    );
    // B: per-layer-grad-norm-lr-equalization (training.rs L388-392 + L441-449 + L510-513)
    let c = concept("per-layer-grad-norm-lr-equalization");
    assert!(
        c.contains("~1000\u{d7}") || c.contains("~1000x"),
        "vanishing-grad anchor"
    );
    assert!(
        c.contains("min(max_norm / layer_norm, cap)") && c.contains("never NaN"),
        "capped-equalization concept"
    );
    // D: width-invariant-superset-training (adaptive_width.rs L2-6 + L20-23 + L407-409)
    let c = concept("width-invariant-superset-training");
    assert!(
        c.contains("width-1 must retain >= 95% of width-4 quality")
            && c.contains("anytime-inference with zero extra storage"),
        "superset-training concept"
    );
    // A: superset-sized-scratch-across-variants (adaptive_width.rs L202-210 + L216-218)
    let c = concept("superset-sized-scratch-across-variants");
    assert!(
        c.contains("STATICALLY KNOWN MAXIMUM") && c.contains("exact length equality"),
        "worst-case scratch concept"
    );
    // H: active-index-grad-scatter-subset-backward (adaptive_width.rs L44-46 + L329-335)
    let c = concept("active-index-grad-scatter-subset-backward");
    assert!(
        c.contains("zero BY CONSTRUCTION") && c.contains("host-side shape of MoE backward"),
        "index-scatter concept"
    );
    // B: inplace-grad-accumulate-deferred-postscale (training.rs L297 + L325-336)
    let c = concept("inplace-grad-accumulate-deferred-postscale");
    assert!(
        c.contains("ONE post-loop sweep") && c.contains("one division total"),
        "deferred-postscale concept"
    );
    // C: fuse-loss-and-output-grad-one-pass (training.rs L312-320)
    let c = concept("fuse-loss-and-output-grad-one-pass");
    assert!(
        c.contains("the SAME diff") && c.contains("standard fused-loss-grad shape"),
        "fused-loss concept"
    );
}

#[test]
fn batch32b_rules_present() {
    // Verify the 10 batch-32b rules across A/B/C/D/E.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (4 new): activation-cache/temp/init/shape-truth family
    assert!(
        ids.contains(&"sigmoid-deriv-recovered-from-output"),
        "sigmoid deriv from output"
    );
    assert!(
        ids.contains(&"needless-swap-temp-hoist-to-scratch"),
        "needless swap temp"
    );
    assert!(
        ids.contains(&"scoped-init-at-consume-site"),
        "scoped init at consume site"
    );
    assert!(
        ids.contains(&"derive-shape-from-payload-not-cached-field"),
        "shape from payload"
    );
    // B (2 new): devirt + digest family
    assert!(
        ids.contains(&"monomorphize-per-element-trait-calls"),
        "monomorphize trait calls"
    );
    assert!(
        ids.contains(&"commit-over-maintained-item-digests"),
        "commit over digests"
    );
    // C (1 new), D (2 new), E (1 new)
    assert!(
        ids.contains(&"fuse-weight-grad-input-grad-single-matrix-visit"),
        "fused B visit"
    );
    assert!(
        ids.contains(&"static-classification-hoist-to-routing-table"),
        "classification hoist"
    );
    assert!(
        ids.contains(&"verify-once-at-trust-boundary"),
        "verify once"
    );
    assert!(
        ids.contains(&"additive-sigmoid-gates-not-softmax-routing"),
        "additive sigmoid gates"
    );
}

#[test]
fn batch32b_concepts_covered() {
    // Verify the batch-32b edge_lora training-half concepts via
    // distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-32b rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // D: static-classification-hoist-to-routing-table (topology_trainer.rs L569-604 + L1046-1056)
    let c = concept("static-classification-hoist-to-routing-table");
    assert!(
        c.contains("classification work per training step for metadata that never changes")
            && c.contains("only its active items with O(1) lookups"),
        "routing-table hoist concept"
    );
    // A: sigmoid-deriv-recovered-from-output (topology_trainer.rs L966-987 + L171-182)
    let c = concept("sigmoid-deriv-recovered-from-output");
    assert!(
        c.contains("pure function of the OUTPUT") && c.contains("halves activation-cache traffic"),
        "deriv-recovery concept"
    );
    // C: fuse-weight-grad-input-grad-single-matrix-visit (topology_trainer.rs L932-963)
    let c = concept("fuse-weight-grad-input-grad-single-matrix-visit");
    assert!(
        c.contains("one visit per B row")
            && c.contains("the outer-product write and the transposed-read accumulation"),
        "fused-B-visit concept"
    );
    // A: needless-swap-temp-hoist-to-scratch (topology_trainer.rs L627-639 + L758-769)
    let c = concept("needless-swap-temp-hoist-to-scratch");
    assert!(
        c.contains("false premise that motivated the temp")
            && c.contains("loop-carried state and belongs in the persistent scratch"),
        "swap-temp concept"
    );
    // A: scoped-init-at-consume-site (topology_trainer.rs L400-421 + L581-594)
    let c = concept("scoped-init-at-consume-site");
    assert!(
        c.contains("scoped to the window actually accumulated into")
            && c.contains("O(total scratch) to O(active window)"),
        "scoped-init concept"
    );
    // B: monomorphize-per-element-trait-calls (topology_trainer.rs L558-563 + L812-819)
    let c = concept("monomorphize-per-element-trait-calls");
    assert!(
        c.contains("strict generalization, not a fork")
            && c.contains("one vtable indirection per node"),
        "monomorphize concept"
    );
    // E: additive-sigmoid-gates-not-softmax-routing (sigmoid_gate.rs L3-16 + L70-83)
    let c = concept("additive-sigmoid-gates-not-softmax-routing");
    assert!(
        c.contains("cross-edge synchronization barrier") && c.contains("embarrassingly parallel"),
        "additive-gates concept"
    );
    // B: commit-over-maintained-item-digests (chain_commit.rs L66-76 + L83-92)
    let c = concept("commit-over-maintained-item-digests");
    assert!(
        c.contains("zero payload re-reads") && c.contains("only at trust boundaries"),
        "digest-hierarchy concept"
    );
    // D: verify-once-at-trust-boundary (chain_commit.rs L156-166 + L236-243)
    let c = concept("verify-once-at-trust-boundary");
    assert!(
        c.contains("the type carries the proof")
            && c.contains("where untrusted bytes become trusted state"),
        "verify-once concept"
    );
    // A: derive-shape-from-payload-not-cached-field (sigmoid_gate.rs L31-38 + L57-61)
    let c = concept("derive-shape-from-payload-not-cached-field");
    assert!(
        c.contains("ONE source of shape truth") && c.contains("structurally impossible"),
        "payload-shape-truth concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch32_pseudocode_parses() {
    // G1 for Batch 32a: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "caller-owned-split-scratch",
        "inplace-gate-scale-at-funnel-bottleneck",
        "sign-branch-stable-sigmoid",
        "incremental-multi-part-hash-no-concat",
        "bool-fields-over-bitset-hot-reads",
        "per-layer-grad-norm-lr-equalization",
        "width-invariant-superset-training",
        "superset-sized-scratch-across-variants",
        "active-index-grad-scatter-subset-backward",
        "inplace-grad-accumulate-deferred-postscale",
        "fuse-loss-and-output-grad-one-pass",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-32 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-32 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-32 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch32b_pseudocode_parses() {
    // G1 for Batch 32b: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "static-classification-hoist-to-routing-table",
        "sigmoid-deriv-recovered-from-output",
        "fuse-weight-grad-input-grad-single-matrix-visit",
        "needless-swap-temp-hoist-to-scratch",
        "scoped-init-at-consume-site",
        "monomorphize-per-element-trait-calls",
        "additive-sigmoid-gates-not-softmax-routing",
        "commit-over-maintained-item-digests",
        "verify-once-at-trust-boundary",
        "derive-shape-from-payload-not-cached-field",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-32b rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-32b ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-32b opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch32c_rules_present() {
    // Verify the 13 batch-32c rules across A/B/C/D/E.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (3 new): state-search / projection-table / capture-footprint family
    assert!(
        ids.contains(&"make-unmake-over-clone-per-candidate"),
        "make/unmake over clone"
    );
    assert!(
        ids.contains(&"deterministic-tiling-projection-tables"),
        "tiling projection tables"
    );
    assert!(
        ids.contains(&"latent-encode-at-capture"),
        "latent encode at capture"
    );
    // B (3 new): hash-shape / tie-break / reward-floor family
    assert!(
        ids.contains(&"length-prefix-shape-faithful-hash"),
        "length-prefix hash"
    );
    assert!(
        ids.contains(&"argmax-rng-tie-break-collect-ties"),
        "argmax rng tie-break"
    );
    assert!(
        ids.contains(&"non-saturating-outcome-reward-floor"),
        "outcome reward floor"
    );
    // C (2 new): aggregate-maintenance / one-sweep family
    assert!(
        ids.contains(&"incremental-centroid-running-sum"),
        "incremental centroid"
    );
    assert!(
        ids.contains(&"one-sweep-spatial-feature-fanout"),
        "one-sweep feature fanout"
    );
    // D (4 new): selection-order / warm-start / data-not-fork / timeout family
    assert!(
        ids.contains(&"order-stable-topk-via-index-sort"),
        "order-stable topk"
    );
    assert!(
        ids.contains(&"shape-gated-warm-start-slot-copy"),
        "shape-gated warm start"
    );
    assert!(
        ids.contains(&"domain-as-data-not-algorithm-fork"),
        "domain as data"
    );
    assert!(
        ids.contains(&"timeout-scalar-tiebreak-decisive-outcome"),
        "timeout scalar tiebreak"
    );
    // E (1 new)
    assert!(
        ids.contains(&"collect-then-resolve-snapshot-batch"),
        "collect-then-resolve"
    );
}

#[test]
fn batch32c_concepts_covered() {
    // Verify the batch-32c edge_lora self-play/arena/consolidation-half
    // concepts via distinctive ASCII substrings located by rule_id (the
    // 32a convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-32c rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // C: incremental-centroid-running-sum (sleep_consolidation.rs L317-357)
    let c = concept("incremental-centroid-running-sum");
    assert!(
        c.contains("append-only aggregate")
            && c.contains("never recomputed from the member list at query time"),
        "centroid concept"
    );
    // B: length-prefix-shape-faithful-hash (sleep_consolidation.rs L404-415)
    let c = concept("length-prefix-shape-faithful-hash");
    assert!(
        c.contains("shape-blind") && c.contains("shape becomes part of the committed identity"),
        "length-prefix concept"
    );
    // D: order-stable-topk-via-index-sort (sleep_consolidation.rs L235-253)
    let c = concept("order-stable-topk-via-index-sort");
    assert!(
        c.contains("destroys the original ordering contract")
            && c.contains("canonical original order"),
        "order-stable topk concept"
    );
    // A: make-unmake-over-clone-per-candidate (arena_eval.rs L225-232)
    let c = concept("make-unmake-over-clone-per-candidate");
    assert!(
        c.contains("restoring bit-identical pre-move state")
            && c.contains("zero per-candidate allocation"),
        "make/unmake concept"
    );
    // B: argmax-rng-tie-break-collect-ties (arena_eval.rs L298-319)
    let c = concept("argmax-rng-tie-break-collect-ties");
    assert!(
        c.contains("move-generation order becomes a hidden priority")
            && c.contains("deterministic GIVEN the seed"),
        "tie-break concept"
    );
    // D: shape-gated-warm-start-slot-copy (cross_game_edge.rs L160-185)
    let c = concept("shape-gated-warm-start-slot-copy");
    assert!(
        c.contains("dimensionally legal")
            && c.contains("the copy count clamp makes the operation total"),
        "warm-start concept"
    );
    // D: domain-as-data-not-algorithm-fork (cross_game_edge.rs L122-131 + L196-209)
    let c = concept("domain-as-data-not-algorithm-fork");
    assert!(
        c.contains("the domain variation entirely in the DATA")
            && c.contains("differ by a data transformation, not a code path"),
        "data-not-fork concept"
    );
    // C: one-sweep-spatial-feature-fanout (go_self_play.rs L229-314)
    let c = concept("one-sweep-spatial-feature-fanout");
    assert!(
        c.contains("the sole owner of the data visit") && c.contains("never by counting cells"),
        "one-sweep concept"
    );
    // A: deterministic-tiling-projection-tables (go_self_play.rs L94-170)
    let c = concept("deterministic-tiling-projection-tables");
    assert!(
        c.contains("the same seed rebuilds the identical encoder")
            && c.contains("degenerate all-zero output column"),
        "projection-tables concept"
    );
    // A: latent-encode-at-capture (go/bomber/fft self-play)
    let c = concept("latent-encode-at-capture");
    assert!(
        c.contains("Encode while the state is live")
            && c.contains("no risk of post-hoc encoding drifting"),
        "encode-at-capture concept"
    );
    // B: non-saturating-outcome-reward-floor (go/bomber/fft sigmoid_outcome_reward)
    let c = concept("non-saturating-outcome-reward-floor");
    assert!(
        c.contains("never taught what to avoid") && c.contains("losses retain 15% weight"),
        "reward-floor concept"
    );
    // E: collect-then-resolve-snapshot-batch (fft_self_play.rs L297-328)
    let c = concept("collect-then-resolve-snapshot-batch");
    assert!(
        c.contains("read-only over shared state")
            && c.contains("the single place where mutation happens"),
        "collect-then-resolve concept"
    );
    // D: timeout-scalar-tiebreak-decisive-outcome (fft_self_play.rs L201-228)
    let c = concept("timeout-scalar-tiebreak-decisive-outcome");
    assert!(
        c.contains("drown the reward signal in uninformative draws")
            && c.contains("signal density at zero maintenance cost"),
        "timeout-tiebreak concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch32c_pseudocode_parses() {
    // G1 for Batch 32c: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "incremental-centroid-running-sum",
        "length-prefix-shape-faithful-hash",
        "order-stable-topk-via-index-sort",
        "make-unmake-over-clone-per-candidate",
        "argmax-rng-tie-break-collect-ties",
        "shape-gated-warm-start-slot-copy",
        "domain-as-data-not-algorithm-fork",
        "one-sweep-spatial-feature-fanout",
        "deterministic-tiling-projection-tables",
        "latent-encode-at-capture",
        "non-saturating-outcome-reward-floor",
        "collect-then-resolve-snapshot-batch",
        "timeout-scalar-tiebreak-decisive-outcome",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-32c rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-32c ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-32c opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch33_rules_present() {
    // Verify the 7 batch-33 rules across A/B/D/E/N.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // E (1 new): output-owned grid family
    assert!(
        ids.contains(&"grid-over-output-thread-private-sweep"),
        "output-owned grid sweep"
    );
    // D (1 new): buffer-set reuse family
    assert!(
        ids.contains(&"buffer-set-struct-two-tier-dispatch"),
        "buffer-set two-tier dispatch"
    );
    // A (2 new): slot-write + immutable-operand family
    assert!(
        ids.contains(&"slot-offset-partial-buffer-write"),
        "slot-offset partial write"
    );
    assert!(
        ids.contains(&"immutable-operand-upload-at-set-creation"),
        "immutable operand at creation"
    );
    // B (2 new): selective projection + rademacher family
    assert!(
        ids.contains(&"selective-projection-effective-rank-head"),
        "selective projection head"
    );
    assert!(
        ids.contains(&"rademacher-signs-over-gaussian-projection"),
        "rademacher signs"
    );
    // N (1 new): host-computed derived scalar
    assert!(
        ids.contains(&"derived-scalar-host-computed-in-uniform"),
        "derived scalar host-computed"
    );
}

#[test]
fn batch33_concepts_covered() {
    // Verify the batch-33 spectralquant calibration-cluster concepts
    // via distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-33 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // E: grid-over-output-thread-private-sweep (calibration.rs L288-290 + covariance.wgsl L50-79)
    let c = concept("grid-over-output-thread-private-sweep");
    assert!(
        c.contains("owned by exactly one thread") && c.contains("zero cross-thread reduction"),
        "output-owned sweep concept"
    );
    // D: buffer-set-struct-two-tier-dispatch (rotation.rs L74-79 + L198-216 vs L263-269)
    let c = concept("buffer-set-struct-two-tier-dispatch");
    assert!(
        c.contains("reuse unit") && c.contains("makes the allocating path explicitly secondary"),
        "buffer-set concept"
    );
    // A: slot-offset-partial-buffer-write (rotation.rs L146-160 + eigenbasis_rotation.wgsl L44-45)
    let c = concept("slot-offset-partial-buffer-write");
    assert!(
        c.contains("host-side mirror of the kernel's indexing base") && c.contains("delta write"),
        "slot-offset concept"
    );
    // A: immutable-operand-upload-at-set-creation (rotation.rs L121-129 + calibration.rs L187-199)
    let c = concept("immutable-operand-upload-at-set-creation");
    assert!(
        c.contains("lifetime") && c.contains("never sees the operand's bytes again"),
        "immutable-operand concept"
    );
    // B: selective-projection-effective-rank-head (qjl.rs L3/L5/L166-168 + selective_qjl.wgsl L57-62)
    let c = concept("selective-projection-effective-rank-head");
    assert!(
        c.contains("bypasses projection entirely") && c.contains("effective rank"),
        "selective-projection concept"
    );
    // N: derived-scalar-host-computed-in-uniform (qjl.rs L48-49/L59 + selective_qjl.wgsl L22/L66)
    let c = concept("derived-scalar-host-computed-in-uniform");
    assert!(
        c.contains("bit-identical") && c.contains("2.5 ULP"),
        "derived-scalar concept"
    );
    // B: rademacher-signs-over-gaussian-projection (qjl.rs L4/L84 + bench L264-267)
    let c = concept("rademacher-signs-over-gaussian-projection");
    assert!(
        c.contains("coin flip") && c.contains("exact in f32"),
        "rademacher concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch33_pseudocode_parses() {
    // G1 for Batch 33: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "grid-over-output-thread-private-sweep",
        "buffer-set-struct-two-tier-dispatch",
        "slot-offset-partial-buffer-write",
        "immutable-operand-upload-at-set-creation",
        "selective-projection-effective-rank-head",
        "derived-scalar-host-computed-in-uniform",
        "rademacher-signs-over-gaussian-projection",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-33 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-33 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-33 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch34_rules_present() {
    // Verify the 9 batch-34 rules across A/C/D/H/L/N.
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // N (1 new): exact-op parity
    assert!(
        ids.contains(&"exact-op-parity-for-analytic-backward"),
        "exact-op parity"
    );
    // A (2 new): mirror ring + activation slots
    assert!(
        ids.contains(&"mirror-ring-store-sliding-window-contiguity"),
        "mirror ring store"
    );
    assert!(
        ids.contains(&"preallocated-activation-slots-inplace-save"),
        "activation slots"
    );
    // D (3 new): split dispatch + hybrid sync points + op-major inversion
    assert!(
        ids.contains(&"split-dispatch-expose-fused-intermediate"),
        "split dispatch"
    );
    assert!(
        ids.contains(&"hybrid-cpu-elementwise-gpu-throughput-sync-points"),
        "hybrid sync points"
    );
    assert!(
        ids.contains(&"op-major-backward-loop-inversion-batch-sequence-axis"),
        "op-major inversion"
    );
    // L (1 new): pre-transposed handles
    assert!(
        ids.contains(&"pre-transposed-consumer-layout-handles-build-once"),
        "pre-transposed handles"
    );
    // C (1 new): sibling weight-grads share input
    assert!(
        ids.contains(&"shape-matched-sibling-weight-grads-share-input-loop"),
        "sibling grads share input"
    );
    // H (1 new): norm-statistic exposure
    assert!(
        ids.contains(&"backward-helper-expose-shared-norm-statistic"),
        "norm statistic exposure"
    );
}

#[test]
fn batch34_concepts_covered() {
    // Verify the batch-34 Gemma-4 training-cluster concepts via
    // distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-34 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // N: exact-op-parity (mod.rs L44-48 + engine backward tanh)
    let c = concept("exact-op-parity-for-analytic-backward");
    assert!(
        c.contains("two different functions") && c.contains("train-vs-infer"),
        "exact-op parity concept"
    );
    // A: mirror-ring (mod.rs L473-500 + kv_cache contract)
    let c = concept("mirror-ring-store-sliding-window-contiguity");
    assert!(
        c.contains("contiguous by construction") && c.contains("2·sw"),
        "mirror-ring concept"
    );
    // D: split-dispatch (mod.rs L325-344 + dispatch.rs discard)
    let c = concept("split-dispatch-expose-fused-intermediate");
    assert!(
        c.contains("re-launch the whole producer") && c.contains("never a second launch"),
        "split-dispatch concept"
    );
    // A: activation slots (16 save sites)
    let c = concept("preallocated-activation-slots-inplace-save");
    assert!(
        c.contains("zero allocations") && c.contains("size is asserted once"),
        "activation slots concept"
    );
    // D: hybrid sync points (L258/L311/L369/L383)
    let c = concept("hybrid-cpu-elementwise-gpu-throughput-sync-points");
    assert!(
        c.contains("exactly 4 syncs") && c.contains("launch-free"),
        "hybrid sync concept"
    );
    // L: pre-transposed handles (backward_gpu L20-26 + gemma2/q4k recurrence)
    let c = concept("pre-transposed-consumer-layout-handles-build-once");
    assert!(
        c.contains("OPPOSITE layout") && c.contains("frozen-base contract"),
        "pre-transposed handles concept"
    );
    // C: sibling weight-grads (L532-540 + L358-366)
    let c = concept("shape-matched-sibling-weight-grads-share-input-loop");
    assert!(
        c.contains("identical output-row counts") && c.contains("ONE pass over the shared input"),
        "sibling grads concept"
    );
    // D: op-major inversion (L305-522 + dispatch round-trip)
    let c = concept("op-major-backward-loop-inversion-batch-sequence-axis");
    assert!(
        c.contains("sequence axis is free") && c.contains("7 × seq_len"),
        "op-major concept"
    );
    // H: norm-statistic exposure (engine rmsnorm + 7 sites)
    let c = concept("backward-helper-expose-shared-norm-statistic");
    assert!(
        c.contains("return shape") && c.contains("SAME call site"),
        "norm statistic concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch34_pseudocode_parses() {
    // G1 for Batch 34: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "exact-op-parity-for-analytic-backward",
        "mirror-ring-store-sliding-window-contiguity",
        "split-dispatch-expose-fused-intermediate",
        "preallocated-activation-slots-inplace-save",
        "hybrid-cpu-elementwise-gpu-throughput-sync-points",
        "pre-transposed-consumer-layout-handles-build-once",
        "shape-matched-sibling-weight-grads-share-input-loop",
        "op-major-backward-loop-inversion-batch-sequence-axis",
        "backward-helper-expose-shared-norm-statistic",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-34 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-34 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-34 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch35_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // E (1 new): mask sentinel identity participation
    assert!(
        ids.contains(&"mask-sentinel-matches-reduction-identity"),
        "mask sentinel"
    );
    // A (3 new): staging gate, alias live handle, quadratic recompute
    assert!(
        ids.contains(&"smem-staging-gated-by-cross-thread-reuse"),
        "staging gate"
    );
    assert!(
        ids.contains(&"side-consumer-readback-alias-live-handle-for-dependent-dispatch"),
        "alias live handle"
    );
    assert!(
        ids.contains(&"quadratic-attention-intermediates-recompute-from-linear-saves"),
        "quadratic recompute"
    );
    // C (1 new): kernelize past the batching floor
    assert!(
        ids.contains(&"kernelize-host-chain-after-sync-batching-floor"),
        "kernelize floor"
    );
    // B (1 new): separate scratch + runtime strides
    assert!(
        ids.contains(&"block-reduction-separate-scratch-runtime-strides"),
        "runtime strides"
    );
    // N (1 new): saturation boundary of output-recovery
    assert!(
        ids.contains(&"saturating-nonlinearity-recovery-breaks-near-cap"),
        "saturation boundary"
    );
    // H (1 new): adapter grad join mirrors insertion point
    assert!(
        ids.contains(&"adapter-grad-join-mirrors-forward-delta-insertion-point"),
        "adapter grad join"
    );
}

#[test]
fn batch35_concepts_covered() {
    // Verify the batch-35 Gemma-2 training-cluster concepts via
    // distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-35 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // E: mask sentinel (attention_gpu.rs L149-152 + L194-198 + L235)
    let c = concept("mask-sentinel-matches-reduction-identity");
    assert!(
        c.contains("IDENTITY element") && c.contains("rewrite the sentinel"),
        "mask sentinel concept"
    );
    // A: staging gate (L123-129 + L141)
    let c = concept("smem-staging-gated-by-cross-thread-reuse");
    assert!(
        c.contains("reuse count exceeds one") && c.contains("never enters smem"),
        "staging gate concept"
    );
    // C: kernelize floor (header 52 + resident.rs L687-688)
    let c = concept("kernelize-host-chain-after-sync-batching-floor");
    assert!(
        c.contains("sync floor of 2") && c.contains("2 syncs x 26 layers = 52"),
        "kernelize floor concept"
    );
    // B: separate scratch + runtime strides (L157-188)
    let c = concept("block-reduction-separate-scratch-runtime-strides");
    assert!(
        c.contains("SEPARATE scratch") && c.contains("runtime extent"),
        "runtime strides concept"
    );
    // N: saturation boundary (engine L354-358 + mod.rs L572-578 contrast)
    let c = concept("saturating-nonlinearity-recovery-breaks-near-cap");
    assert!(
        c.contains("rounds to 1.0") && c.contains("~270"),
        "saturation concept"
    );
    // H: adapter grad join (mod.rs L197-204 + L613-630 + L931-933)
    let c = concept("adapter-grad-join-mirrors-forward-delta-insertion-point");
    assert!(
        c.contains("immortal by construction") && c.contains("Issue 687 H2"),
        "adapter join concept"
    );
    // A: alias live handle (dispatch.rs L387-392 + L498-503)
    let c = concept("side-consumer-readback-alias-live-handle-for-dependent-dispatch");
    assert!(
        c.contains("never left the device") && c.contains("~53 MB"),
        "alias handle concept"
    );
    // A: quadratic recompute (mod.rs L216-220 + backward.rs L333-344)
    let c = concept("quadratic-attention-intermediates-recompute-from-linear-saves");
    assert!(
        c.contains("Theta(N x d)") && c.contains("~870 MB"),
        "quadratic recompute concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch35_pseudocode_parses() {
    // G1 for Batch 35: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "mask-sentinel-matches-reduction-identity",
        "smem-staging-gated-by-cross-thread-reuse",
        "kernelize-host-chain-after-sync-batching-floor",
        "block-reduction-separate-scratch-runtime-strides",
        "saturating-nonlinearity-recovery-breaks-near-cap",
        "adapter-grad-join-mirrors-forward-delta-insertion-point",
        "side-consumer-readback-alias-live-handle-for-dependent-dispatch",
        "quadratic-attention-intermediates-recompute-from-linear-saves",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-35 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-35 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-35 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch36_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // C (1 new): constant scale folded into the upload copy
    assert!(
        ids.contains(&"constant-scale-folded-into-upload-copy"),
        "fold scale into upload"
    );
    // D (5 new): residency rung, handle checkpoints, decode-shape shed,
    // sibling ordering, dispatch-read window
    assert!(
        ids.contains(&"side-branch-operand-residency-matches-mutation-lifetime"),
        "residency rung"
    );
    assert!(
        ids.contains(&"activation-checkpoint-as-handle-clone-deferred-batch-readback"),
        "handle checkpoints"
    );
    assert!(
        ids.contains(&"training-forward-shed-decode-loop-shape"),
        "decode-shape shed"
    );
    assert!(
        ids.contains(&"sibling-dispatches-before-any-readback"),
        "sibling ordering"
    );
    assert!(
        ids.contains(&"independent-cpu-work-fills-dispatch-read-window"),
        "dispatch-read window"
    );
    // H (1 new): multi-term merge under a shared Jacobian
    assert!(
        ids.contains(&"grad-terms-merge-before-shared-link-jacobian"),
        "grad merge before Jacobian"
    );
}

#[test]
fn batch36_concepts_covered() {
    // Verify the batch-36 Gemma-2 resident-cluster concepts via
    // distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-36 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // C: fold into the upload copy (L85-90 + L78-79)
    let c = concept("constant-scale-folded-into-upload-copy");
    assert!(
        c.contains("upload COPY") && c.contains("unscaled original"),
        "fold-into-copy concept"
    );
    // D: residency rung (L12-14 + L56-58)
    let c = concept("side-branch-operand-residency-matches-mutation-lifetime");
    assert!(
        c.contains("mutation cadence") && c.contains("~70K sync round-trips"),
        "residency concept"
    );
    // D: handle checkpoints (L36-37 + L959-963)
    let c = concept("activation-checkpoint-as-handle-clone-deferred-batch-readback");
    assert!(
        c.contains("refcount bump") && c.contains("364 syncs"),
        "checkpoint concept"
    );
    // D: decode-shape shed (L545-557)
    let c = concept("training-forward-shed-decode-loop-shape");
    assert!(
        c.contains("DECODE loop shape") && c.contains("~55K dispatches"),
        "decode-shed concept"
    );
    // D: sibling ordering (L1087-1095 + L1515-1525)
    let c = concept("sibling-dispatches-before-any-readback");
    assert!(
        c.contains("FIRST readback") && c.contains("DIFFERENT input buffers"),
        "sibling ordering concept"
    );
    // D: dispatch-read window (L1326-1351)
    let c = concept("independent-cpu-work-fills-dispatch-read-window");
    assert!(
        c.contains("host is IDLE") && c.contains("max(GPU, CPU)"),
        "window concept"
    );
    // H: merge before Jacobian (L1229-1248)
    let c = concept("grad-terms-merge-before-shared-link-jacobian");
    assert!(
        c.contains("ONCE to the sum") && c.contains("WRONG, not just slower"),
        "merge concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch36_pseudocode_parses() {
    // G1 for Batch 36: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "constant-scale-folded-into-upload-copy",
        "side-branch-operand-residency-matches-mutation-lifetime",
        "activation-checkpoint-as-handle-clone-deferred-batch-readback",
        "training-forward-shed-decode-loop-shape",
        "sibling-dispatches-before-any-readback",
        "independent-cpu-work-fills-dispatch-read-window",
        "grad-terms-merge-before-shared-link-jacobian",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-36 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-36 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-36 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch37_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (3 new): device-resident KV ring, aliased-tensor handle clone,
    // cache-blocked host transpose
    assert!(
        ids.contains(&"kv-window-device-resident-ring-append"),
        "KV ring append"
    );
    assert!(
        ids.contains(&"identical-tensor-handle-clone-zero-reupload"),
        "absent-V handle clone"
    );
    assert!(
        ids.contains(&"tiled-host-transpose-before-quantize"),
        "tiled transpose"
    );
    // B (1 new): scale folded at the sparse gather
    assert!(
        ids.contains(&"sparse-gather-scale-at-read-not-table-prepass"),
        "gather-side scale fold"
    );
    // D (2 new): loss-position gating, bulk-slice fast path
    assert!(
        ids.contains(&"lm-head-loss-positions-only"),
        "lm-head loss gating"
    );
    assert!(
        ids.contains(&"materialized-fast-path-bulk-slice-accessor"),
        "bulk slice accessor"
    );
    // L (1 new): draining build
    assert!(
        ids.contains(&"stream-drain-free-source-during-build"),
        "draining build"
    );
    // N (1 new): verbatim upload on format match
    assert!(
        ids.contains(&"verbatim-upload-on-format-match"),
        "verbatim upload"
    );
}

#[test]
fn batch37_concepts_covered() {
    // Verify the batch-37 Q4_K-training-cluster concepts via
    // distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-37 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: kv-window-device-resident-ring-append (mod.rs L1246-1251 + L979-980)
    let c = concept("kv-window-device-resident-ring-append");
    assert!(
        c.contains("append-vs-snapshot") && c.contains("only the NEW row crosses the bus"),
        "KV ring concept"
    );
    // D: lm-head-loss-positions-only (mod.rs L1123 + backward.rs L73-76)
    let c = concept("lm-head-loss-positions-only");
    assert!(
        c.contains("loss-support set") && c.contains("never launches"),
        "lm-head gating concept"
    );
    // N: verbatim-upload-on-format-match (weight_buffers.rs L498-509)
    let c = concept("verbatim-upload-on-format-match");
    assert!(
        c.contains("source format == destination format") && c.contains("never dequantize"),
        "verbatim upload concept"
    );
    // L: stream-drain-free-source-during-build (mod.rs L610-635)
    let c = concept("stream-drain-free-source-during-build");
    assert!(
        c.contains("O(one unit), not O(model)") && c.contains("process, upload, drop"),
        "draining build concept"
    );
    // A: identical-tensor-handle-clone-zero-reupload (weight_buffers.rs L229-232)
    let c = concept("identical-tensor-handle-clone-zero-reupload");
    assert!(
        c.contains("refcount bump") && c.contains("zero re-upload"),
        "handle clone concept"
    );
    // B: sparse-gather-scale-at-read-not-table-prepass (weight_buffers.rs L77-96 + L147-148)
    let c = concept("sparse-gather-scale-at-read-not-table-prepass");
    assert!(
        c.contains("READ-SET DENSITY") && c.contains("INVERSE regime"),
        "gather-side scale concept"
    );
    // A: tiled-host-transpose-before-quantize (weight_buffers.rs L460-466)
    let c = concept("tiled-host-transpose-before-quantize");
    assert!(
        c.contains("write-allocated") && c.contains("cache-BLOCKED"),
        "tiled transpose concept"
    );
    // D: materialized-fast-path-bulk-slice-accessor (weight_buffers.rs L100-105)
    let c = concept("materialized-fast-path-bulk-slice-accessor");
    assert!(
        c.contains("ONCE PER STEP") && c.contains("representation-specialized"),
        "bulk accessor concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch37_pseudocode_parses() {
    // G1 for Batch 37: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "kv-window-device-resident-ring-append",
        "lm-head-loss-positions-only",
        "verbatim-upload-on-format-match",
        "stream-drain-free-source-during-build",
        "identical-tensor-handle-clone-zero-reupload",
        "sparse-gather-scale-at-read-not-table-prepass",
        "tiled-host-transpose-before-quantize",
        "materialized-fast-path-bulk-slice-accessor",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-37 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-37 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-37 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch38_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (3 new): flat activation slots, transient-scratch pinning,
    // row-accumulate transposed consumption
    assert!(
        ids.contains(&"flat-activation-slots-as-readback-target"),
        "flat activation slots"
    );
    assert!(
        ids.contains(&"transient-spike-pins-persistent-scratch"),
        "transient spike pinning"
    );
    assert!(
        ids.contains(&"transposed-consumption-accumulate-not-gather"),
        "row-accumulate z pass"
    );
    // D (1 new): env-flag resolution timing
    assert!(
        ids.contains(&"env-flag-resolved-once-not-per-dispatch"),
        "env flag resolved once"
    );
}

#[test]
fn batch38_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-38 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // D: env-flag-resolved-once-not-per-dispatch (forward_batched.rs L203/L382/L385 + gemv_autotune.rs L121-131)
    let c = concept("env-flag-resolved-once-not-per-dispatch");
    assert!(
        c.contains("resolve it ONCE") && c.contains("papaya"),
        "env-flag resolution concept"
    );
    // A: flat-activation-slots-as-readback-target (forward_batched.rs save sweeps + mod.rs L753-755)
    let c = concept("flat-activation-slots-as-readback-target");
    assert!(
        c.contains("container layout IS the copy count") && c.contains("readback target"),
        "flat slots concept"
    );
    // A: transient-spike-pins-persistent-scratch (backward.rs L409 vs L557; mod.rs L377/L426)
    let c = concept("transient-spike-pins-persistent-scratch");
    assert!(
        c.contains("historical maximum") && c.contains("non-recurring"),
        "scratch pinning concept"
    );
    // A: transposed-consumption-accumulate-not-gather (backward.rs L923-930 vs forward_batched.rs L1013-1018)
    let c = concept("transposed-consumption-accumulate-not-gather");
    assert!(
        c.contains("loop interchange") && c.contains("read amplification"),
        "row-accumulate concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch38_pseudocode_parses() {
    // G1 for Batch 38: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "env-flag-resolved-once-not-per-dispatch",
        "flat-activation-slots-as-readback-target",
        "transient-spike-pins-persistent-scratch",
        "transposed-consumption-accumulate-not-gather",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-38 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-38 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-38 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch39_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // K (1 new): autotune cache salvage across owner lifetimes
    assert!(
        ids.contains(&"autotune-cache-extract-outlives-owner"),
        "autotune cache extract"
    );
    // A (5 new): rewind, boot-harvest hotswap, tiny params, dst-passing readback, clone escape
    assert!(
        ids.contains(&"append-cache-rewind-overwrite-in-place"),
        "append cache rewind"
    );
    assert!(
        ids.contains(&"slow-path-bootstraps-raw-buffer-hotswap-cache"),
        "slow path bootstrap hotswap"
    );
    assert!(
        ids.contains(&"tiny-param-buffer-rebuild-over-cache"),
        "tiny param rebuild"
    );
    assert!(
        ids.contains(&"readback-destination-passing-free-fn"),
        "readback destination passing"
    );
    assert!(
        ids.contains(&"variant-owned-output-buffer-clone-escape"),
        "variant output clone escape"
    );
    // C (1 new): device-side anchor payoff condition
    assert!(
        ids.contains(&"elementwise-offload-needs-device-side-anchor"),
        "device side anchor"
    );
    // D (2 new): shared layer body, handle boundary
    assert!(
        ids.contains(&"entrypoint-variants-share-layer-body"),
        "entrypoint shared body"
    );
    assert!(
        ids.contains(&"pipeline-layer-boundary-device-handle"),
        "pipeline handle boundary"
    );
}

#[test]
fn batch39_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a
    // convention — immune to prose-evolution churn).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-39 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // K: autotune-cache-extract-outlives-owner (kimi_k3_gpu_forward.rs L296-L302/L324-L329/L1735-L1743)
    let c = concept("autotune-cache-extract-outlives-owner");
    assert!(
        c.contains("decouple the cache lifetime") && c.contains("re-tuning is not"),
        "autotune extract concept"
    );
    // A: append-cache-rewind-overwrite-in-place (L2061-L2074 + L2084-L2089)
    let c = concept("append-cache-rewind-overwrite-in-place");
    assert!(
        c.contains("rewind_to(pos)") && c.contains("O(max_seq)"),
        "rewind concept"
    );
    // A: slow-path-bootstraps-raw-buffer-hotswap-cache (L1551-L1558/L1585-L1627/L1630-L1653)
    let c = concept("slow-path-bootstraps-raw-buffer-hotswap-cache");
    assert!(
        c.contains("harvests the raw native buffers") && c.contains("slot ORDER is the key"),
        "boot harvest concept"
    );
    // D: entrypoint-variants-share-layer-body (L566-L576/L977-L992/L1862-L1896 vs L2162-L2177)
    let c = concept("entrypoint-variants-share-layer-body");
    assert!(
        c.contains("ONE shared method") && c.contains("guarantees divergence"),
        "shared body concept"
    );
    // C: elementwise-offload-needs-device-side-anchor (L969-L972/L930-L935 vs L737-L741)
    let c = concept("elementwise-offload-needs-device-side-anchor");
    assert!(
        c.contains("device-side anchor") && c.contains("compute-offload"),
        "anchor concept"
    );
    // A: tiny-param-buffer-rebuild-over-cache (L2437-L2441/L2473-L2476)
    let c = concept("tiny-param-buffer-rebuild-over-cache");
    assert!(
        c.contains("rebuild the params") && c.contains("cache line"),
        "tiny params concept"
    );
    // A: readback-destination-passing-free-fn (L807-L828 vs L505-L523/L613-L614)
    let c = concept("readback-destination-passing-free-fn");
    assert!(
        c.contains("destination-passing") && c.contains("FREE function over the context"),
        "dst passing concept"
    );
    // A: variant-owned-output-buffer-clone-escape (L1069/L1192/L1304/L1416/L1979/L2240 + L2308-L2315)
    let c = concept("variant-owned-output-buffer-clone-escape");
    assert!(
        c.contains("ownership placement") && c.contains("per layer per token"),
        "clone escape concept"
    );
    // D: pipeline-layer-boundary-device-handle (L937/L871-L874/L944-L945/L1086-L1087/L1018-L1024)
    let c = concept("pipeline-layer-boundary-device-handle");
    assert!(
        c.contains("pipeline currency") && c.contains("INTERNAL detail"),
        "handle boundary concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch39_pseudocode_parses() {
    // G1 for Batch 39: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "autotune-cache-extract-outlives-owner",
        "append-cache-rewind-overwrite-in-place",
        "slow-path-bootstraps-raw-buffer-hotswap-cache",
        "entrypoint-variants-share-layer-body",
        "elementwise-offload-needs-device-side-anchor",
        "tiny-param-buffer-rebuild-over-cache",
        "readback-destination-passing-free-fn",
        "variant-owned-output-buffer-clone-escape",
        "pipeline-layer-boundary-device-handle",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-39 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-39 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-39 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch40_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (4 new): coalesced-write scratch, cache-derived write plan, from_cpu builders, bridge hoist
    assert!(
        ids.contains(&"host-scratch-mirrors-slot-order-for-coalesced-writes"),
        "coalesced write scratch"
    );
    assert!(
        ids.contains(&"write-plan-derived-from-cache-slots-not-recomputed"),
        "cache-derived write plan"
    );
    assert!(
        ids.contains(&"cpu-source-builder-skips-device-roundtrip"),
        "cpu source builder"
    );
    assert!(
        ids.contains(&"foreign-accumulator-bridge-copy-hoist-to-outer-loop"),
        "foreign bridge hoist"
    );
    // H (1 new): two-phase expert loop
    assert!(
        ids.contains(&"expert-loop-two-phase-split-batch-across-iterations"),
        "two-phase expert loop"
    );
    // N (1 new): gamma scope in RMSNorm backward
    assert!(
        ids.contains(&"rmsnorm-backward-gamma-scope-correction-term"),
        "gamma scope correction"
    );
    // D (1 new): scalar-factored GEMV collapse
    assert!(
        ids.contains(&"per-position-scalar-factored-gemv-collapse"),
        "scalar factored gemv"
    );
}

#[test]
fn batch40_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-40 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: host-scratch-mirrors-slot-order-for-coalesced-writes (L109-L116/L346-L365 vs L973-L986)
    let c = concept("host-scratch-mirrors-slot-order-for-coalesced-writes");
    assert!(
        c.contains("SUM of slot lengths") && c.contains("the write-call count"),
        "coalesced scratch concept"
    );
    // A: write-plan-derived-from-cache-slots-not-recomputed (L1907-L1909 vs L1952-L1961)
    let c = concept("write-plan-derived-from-cache-slots-not-recomputed");
    assert!(
        c.contains("size_bytes") && c.contains("bug factory"),
        "cache-derived plan concept"
    );
    // A: cpu-source-builder-skips-device-roundtrip (L253-L256 vs L225-L237)
    let c = concept("cpu-source-builder-skips-device-roundtrip");
    assert!(
        c.contains("read_one") && c.contains("the host already owns"),
        "cpu source builder concept"
    );
    // H: expert-loop-two-phase-split-batch-across-iterations (L1682-L1710 + L1563-L1564)
    let c = concept("expert-loop-two-phase-split-batch-across-iterations");
    assert!(
        c.contains("cross-iteration") && c.contains("stage-A"),
        "two-phase loop concept"
    );
    // N: rmsnorm-backward-gamma-scope-correction-term (L1537 wrong vs L2839 right)
    let c = concept("rmsnorm-backward-gamma-scope-correction-term");
    assert!(
        c.contains("ONLY the direct dy term") && c.contains("near 1"),
        "gamma scope concept"
    );
    // D: per-position-scalar-factored-gemv-collapse (L2557-L2559 + L2576-L2584 the failure)
    let c = concept("per-position-scalar-factored-gemv-collapse");
    assert!(
        c.contains("consumer polarity") && c.contains("by linearity"),
        "scalar factoring concept"
    );
    // A: foreign-accumulator-bridge-copy-hoist-to-outer-loop (L3082/L3085-L3098/L3256-L3269)
    let c = concept("foreign-accumulator-bridge-copy-hoist-to-outer-loop");
    assert!(
        c.contains("ping-pong") && c.contains("copy-in"),
        "foreign bridge concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch40_pseudocode_parses() {
    // G1 for Batch 40: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "host-scratch-mirrors-slot-order-for-coalesced-writes",
        "write-plan-derived-from-cache-slots-not-recomputed",
        "cpu-source-builder-skips-device-roundtrip",
        "expert-loop-two-phase-split-batch-across-iterations",
        "rmsnorm-backward-gamma-scope-correction-term",
        "per-position-scalar-factored-gemv-collapse",
        "foreign-accumulator-bridge-copy-hoist-to-outer-loop",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-40 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-40 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-40 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch41_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (5 new): aliased-slot fallback, write coalescing, reservation token, type bridge, zombie machinery
    assert!(
        ids.contains(&"aliased-slot-allocate-replace-fallback-severs-binding"),
        "aliased slot fallback"
    );
    assert!(
        ids.contains(&"pool-slot-write-coalescing-owns-only-declared-ranges"),
        "pool slot coalescing"
    );
    assert!(
        ids.contains(&"pool-reservation-token-outlives-raw-view"),
        "reservation token"
    );
    assert!(
        ids.contains(&"gpu-type-bridge-borrows-not-clones"),
        "type bridge borrows"
    );
    assert!(
        ids.contains(&"superseded-primitive-machinery-zombie"),
        "zombie machinery"
    );
    // E (1 new): concurrent disjoint uploads
    assert!(
        ids.contains(&"concurrent-disjoint-write-buffer-parallelizes-uploads"),
        "concurrent uploads"
    );
}

#[test]
fn batch41_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-41 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // E: concurrent-disjoint-write-buffer-parallelizes-uploads
    let c = concept("concurrent-disjoint-write-buffer-parallelizes-uploads");
    assert!(
        c.contains("disjoint offsets") && c.contains("host-bandwidth-bound"),
        "concurrent-disjoint-write-buffer-parallelizes-uploads concept"
    );
    // A: aliased-slot-allocate-replace-fallback-severs-binding
    let c = concept("aliased-slot-allocate-replace-fallback-severs-binding");
    assert!(
        c.contains("never fall back to allocate-and-replace") && c.contains("boot weights"),
        "aliased-slot-allocate-replace-fallback-severs-binding concept"
    );
    // A: pool-slot-write-coalescing-owns-only-declared-ranges
    let c = concept("pool-slot-write-coalescing-owns-only-declared-ranges");
    assert!(
        c.contains("union of declared slot ranges") && c.contains("DISCOVER it"),
        "pool-slot-write-coalescing-owns-only-declared-ranges concept"
    );
    // A: pool-reservation-token-outlives-raw-view
    let c = concept("pool-reservation-token-outlives-raw-view");
    assert!(
        c.contains("NOT an ownership token") && c.contains("re-issues it"),
        "pool-reservation-token-outlives-raw-view concept"
    );
    // A: gpu-type-bridge-borrows-not-clones
    let c = concept("gpu-type-bridge-borrows-not-clones");
    assert!(
        c.contains("field-identical") && c.contains("the first clone is 100% dead"),
        "gpu-type-bridge-borrows-not-clones concept"
    );
    // A: superseded-primitive-machinery-zombie
    let c = concept("superseded-primitive-machinery-zombie");
    assert!(
        c.contains("only remaining caller") && c.contains("never-read"),
        "superseded-primitive-machinery-zombie concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch41_pseudocode_parses() {
    // G1 for Batch 41: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "concurrent-disjoint-write-buffer-parallelizes-uploads",
        "aliased-slot-allocate-replace-fallback-severs-binding",
        "pool-slot-write-coalescing-owns-only-declared-ranges",
        "pool-reservation-token-outlives-raw-view",
        "gpu-type-bridge-borrows-not-clones",
        "superseded-primitive-machinery-zombie",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-41 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-41 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-41 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch42_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (2 new): invariant-prefix elision + runtime identity gate
    assert!(
        ids.contains(&"refine-loop-invariant-prefix-elision"),
        "invariant prefix elision"
    );
    assert!(
        ids.contains(&"runtime-identity-gate-cached-verdict-elides-side-projection"),
        "identity gate elision"
    );
    // B (1 new): entropy via the exp shift identity
    assert!(
        ids.contains(&"softmax-entropy-via-shift-identity"),
        "entropy shift identity"
    );
    // A (1 new): expected embedding as resident-head transposed GEMV
    assert!(
        ids.contains(&"expected-embedding-transposed-gemv-resident-head"),
        "x0 transposed gemv"
    );
}

#[test]
fn batch42_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-42 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // D: refine-loop-invariant-prefix-elision (mod.rs L1488/L1491-1493 vs L1498/L1528 + t_n L1214-1217)
    let c = concept("refine-loop-invariant-prefix-elision");
    assert!(
        c.contains("mutation set is usually a proper subset")
            && c.contains("O(steps x seq) to O(seq + steps x |S|)"),
        "prefix-elision concept"
    );
    // B: softmax-entropy-via-shift-identity (mod.rs L103-109/L121-127/L151-154)
    let c = concept("softmax-entropy-via-shift-identity");
    assert!(
        c.contains("the shift is exp's argument, so its log is the shift itself")
            && c.contains("ONE transcendental"),
        "entropy-shift concept"
    );
    // D: runtime-identity-gate-cached-verdict-elides-side-projection (mod.rs L556/L671-674 + sc.rs L281-287)
    let c = concept("runtime-identity-gate-cached-verdict-elides-side-projection");
    assert!(
        c.contains("re-derived from CONTENTS")
            && c.contains("gated consumer with an ungated producer"),
        "identity-gate concept"
    );
    // A: expected-embedding-transposed-gemv-resident-head (sc.rs L229-246 + mod.rs L605-610)
    let c = concept("expected-embedding-transposed-gemv-resident-head");
    assert!(
        c.contains("TRANSPOSED GEMV of the tied lm_head")
            && c.contains("reads as a scatter/gather idiom"),
        "x0-gemv concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch42_pseudocode_parses() {
    // G1 for Batch 42: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "refine-loop-invariant-prefix-elision",
        "softmax-entropy-via-shift-identity",
        "runtime-identity-gate-cached-verdict-elides-side-projection",
        "expected-embedding-transposed-gemv-resident-head",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-42 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-42 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-42 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch43_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (3 new): input residency + support-only renorm + stack view bridge
    assert!(
        ids.contains(&"mutable-input-scratch-residency-write-per-call"),
        "input scratch residency"
    );
    assert!(
        ids.contains(&"post-scatter-renorm-support-set-only"),
        "support-only renorm"
    );
    assert!(
        ids.contains(&"stack-array-row-view-bridge"),
        "stack row-view bridge"
    );
    // B (1 new): select_nth over full-vocab sort
    assert!(
        ids.contains(&"topk-select-nth-over-full-vocab-sort"),
        "select_nth topk"
    );
    // D (1 new): the &[] sentinel launders the shape violation
    assert!(
        ids.contains(&"empty-slice-sentinel-launders-shape-violation"),
        "sentinel shape violation"
    );
}

#[test]
fn batch43_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-43 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: mutable-input-scratch-residency-write-per-call (corrector.rs L461-477 vs L686-687/L729-737)
    let c = concept("mutable-input-scratch-residency-write-per-call");
    assert!(
        c.contains("same residency discipline")
            && c.contains("device allocator service a new allocation request"),
        "input-residency concept"
    );
    // B: topk-select-nth-over-full-vocab-sort (corrector.rs L421-436 + L694-709)
    let c = concept("topk-select-nth-over-full-vocab-sort");
    assert!(
        c.contains("select_nth_unstable_by partitions in O(V) average")
            && c.contains("the selection algorithm on the host side"),
        "select-nth concept"
    );
    // A: post-scatter-renorm-support-set-only (corrector.rs L547-563 + L797-812)
    let c = concept("post-scatter-renorm-support-set-only");
    assert!(
        c.contains("non-zero support is exactly the K positions")
            && c.contains("no numerical risk"),
        "support-only renorm concept"
    );
    // A: stack-array-row-view-bridge (dflash.rs L134-146 + 5 in-tree sites)
    let c = concept("stack-array-row-view-bridge");
    assert!(
        c.contains("zero heap, zero data copy") && c.contains("the established bridge idiom"),
        "stack-bridge concept"
    );
    // D: empty-slice-sentinel-launders-shape-violation (dflash.rs L140-144 -> L247)
    let c = concept("empty-slice-sentinel-launders-shape-violation");
    assert!(
        c.contains("but shape-invalid") && c.contains("laundered into silent quality degradation"),
        "sentinel-laundering concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch43_pseudocode_parses() {
    // G1 for Batch 43: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "mutable-input-scratch-residency-write-per-call",
        "topk-select-nth-over-full-vocab-sort",
        "post-scatter-renorm-support-set-only",
        "stack-array-row-view-bridge",
        "empty-slice-sentinel-launders-shape-violation",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-43 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-43 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-43 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch44_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // E (1 new): split the reduction axis when rows are too few
    assert!(
        ids.contains(&"row-dot-split-inner-axis-block-reduce"),
        "reduction-axis split"
    );
    // C (1 new): gather fused into the dot consumer
    assert!(
        ids.contains(&"fuse-indirect-gather-into-dot-consumer"),
        "fused indirect gather"
    );
    // D (2 new): shared cube-size const + canonical serialization order
    assert!(
        ids.contains(&"share-cube-size-const-between-kernel-and-launcher"),
        "shared cube-size const"
    );
    assert!(
        ids.contains(&"serialize-container-canonical-order"),
        "canonical serialization order"
    );
    // A (2 new): smem stash + width-specialized unpack
    assert!(
        ids.contains(&"stash-kernel-internal-intermediate-in-smem"),
        "smem intermediate stash"
    );
    assert!(
        ids.contains(&"bit-cursor-unpack-to-width-specialized-byte-batch"),
        "width-specialized unpack"
    );
    // B (2 new): bit-address hoist + single-sweep minmax
    assert!(
        ids.contains(&"hoist-bit-address-out-of-packed-row-sweep"),
        "bit-address hoist"
    );
    assert!(
        ids.contains(&"single-sweep-minmax-for-quant-scale"),
        "single-sweep minmax"
    );
    // L (1 new): scatter consumer layout at production
    assert!(
        ids.contains(&"fold-tile-transpose-into-dequant-scatter"),
        "scatter at production"
    );
}

#[test]
fn batch44_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-44 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // E: row-dot-split-inner-axis-block-reduce (mod.rs L1283-1306 + L1332-1333)
    let c = concept("row-dot-split-inner-axis-block-reduce");
    assert!(
        c.contains("one SM while the rest of the GPU idles")
            && c.contains("split the REDUCTION axis instead"),
        "reduction-axis-split concept"
    );
    // C: fuse-indirect-gather-into-dot-consumer (mod.rs L1195-1196 -> L1303)
    let c = concept("fuse-indirect-gather-into-dot-consumer");
    assert!(
        c.contains("whose only destiny is to be multiplied")
            && c.contains("delete the gather kernel and pass"),
        "fused-gather concept"
    );
    // D: share-cube-size-const-between-kernel-and-launcher (4 kernels vs 6 launchers)
    let c = concept("share-cube-size-const-between-kernel-and-launcher");
    assert!(
        c.contains("silent cross-file contract")
            && c.contains("exactly one source of truth for the stride"),
        "cube-size-const concept"
    );
    // A: stash-kernel-internal-intermediate-in-smem (mod.rs L1413-1414 -> L1429)
    let c = concept("stash-kernel-internal-intermediate-in-smem");
    assert!(
        c.contains("no external reader") && c.contains("keep it in shared memory"),
        "smem-stash concept"
    );
    // B: hoist-bit-address-out-of-packed-row-sweep (kvarn_dequant.rs L41-47 + L51)
    let c = concept("hoist-bit-address-out-of-packed-row-sweep");
    assert!(
        c.contains("identical on every iteration") && c.contains("decompose the bit address ONCE"),
        "bit-address-hoist concept"
    );
    // L: fold-tile-transpose-into-dequant-scatter (kvarn_dequant.rs L120-133)
    let c = concept("fold-tile-transpose-into-dequant-scatter");
    assert!(
        c.contains("write the consumer's layout at production time")
            && c.contains("materialize-then-transpose"),
        "scatter-at-production concept"
    );
    // A: bit-cursor-unpack-to-width-specialized-byte-batch (kvarn_dequant.rs L162-186)
    let c = concept("bit-cursor-unpack-to-width-specialized-byte-batch");
    assert!(
        c.contains("compile-time-constant") && c.contains("drifted fork"),
        "width-specialized-unpack concept"
    );
    // B: single-sweep-minmax-for-quant-scale (kvarn_zone_weights.rs L67-68)
    let c = concept("single-sweep-minmax-for-quant-scale");
    assert!(
        c.contains("TWO separate full traversals")
            && c.contains("track (lo, hi) jointly in one pass"),
        "single-sweep-minmax concept"
    );
    // D: serialize-container-canonical-order (kvarn_zone_weights.rs L284-287)
    let c = concept("serialize-container-canonical-order");
    assert!(
        c.contains("iteration order seeded by RandomState")
            && c.contains("pure function of container CONTENT"),
        "canonical-order concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch44_pseudocode_parses() {
    // G1 for Batch 44: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "row-dot-split-inner-axis-block-reduce",
        "fuse-indirect-gather-into-dot-consumer",
        "share-cube-size-const-between-kernel-and-launcher",
        "stash-kernel-internal-intermediate-in-smem",
        "hoist-bit-address-out-of-packed-row-sweep",
        "fold-tile-transpose-into-dequant-scatter",
        "bit-cursor-unpack-to-width-specialized-byte-batch",
        "single-sweep-minmax-for-quant-scale",
        "serialize-container-canonical-order",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-44 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-44 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-44 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch45_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (2 new): allocation depth + FD frozen co-operand
    assert!(
        ids.contains(&"scratch-depth-matches-shape-invariance"),
        "scratch depth"
    );
    assert!(
        ids.contains(&"frozen-cooperand-hoist-out-of-perturbation-loop"),
        "frozen co-operand"
    );
    // B (1 new): softmax denominator hoist
    assert!(
        ids.contains(&"softmax-denominator-hoist-out-of-weighted-v"),
        "denominator hoist"
    );
}

#[test]
fn batch45_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-45 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: scratch-depth-matches-shape-invariance (meta_lora.rs L246/L249-252/L322/L330/L388/L395)
    let c = concept("scratch-depth-matches-shape-invariance");
    assert!(
        c.contains("allocation depth must match shape invariance") && c.contains("layer-invariant"),
        "scratch-depth concept"
    );
    // B: softmax-denominator-hoist-out-of-weighted-v (meta_lora.rs L345-358 vs attention.rs L63-76)
    let c = concept("softmax-denominator-hoist-out-of-weighted-v");
    assert!(
        c.contains("an order of magnitude costlier than multiply")
            && c.contains("Hoist the reciprocal once"),
        "denominator-hoist concept"
    );
    // A: frozen-cooperand-hoist-out-of-perturbation-loop (train_meta_lora.rs L276-321/L325-366)
    let c = concept("frozen-cooperand-hoist-out-of-perturbation-loop");
    assert!(
        c.contains("Materialization cadence must match mutation cadence")
            && c.contains("frozen co-operand"),
        "frozen-cooperand concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch45_pseudocode_parses() {
    // G1 for Batch 45: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "scratch-depth-matches-shape-invariance",
        "softmax-denominator-hoist-out-of-weighted-v",
        "frozen-cooperand-hoist-out-of-perturbation-loop",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-45 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-45 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-45 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch46_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (2 new): declared-max thread-local tax + shared-side pair-batch decomposition
    assert!(
        ids.contains(&"declared-max-array-taxed-per-thread-under-runtime-bound"),
        "declared-max array tax"
    );
    assert!(
        ids.contains(&"shared-operand-side-pair-batch-index-decomposition"),
        "shared-side decomposition"
    );
    // D (1 new): full-cost-factor size gate
    assert!(
        ids.contains(&"size-gate-covers-every-cost-factor"),
        "size-gate cost factors"
    );
}

#[test]
fn batch46_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-46 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: declared-max-array-taxed-per-thread-under-runtime-bound (spectralquant_maxsim.wgsl L110/L111/L151)
    let c = concept("declared-max-array-taxed-per-thread-under-runtime-bound");
    assert!(
        c.contains("every thread pays the DECLARED max")
            && c.contains("pipeline override constant"),
        "declared-max concept"
    );
    // A: shared-operand-side-pair-batch-index-decomposition (spectralquant.rs L190 + wgsl L109)
    let c = concept("shared-operand-side-pair-batch-index-decomposition");
    assert!(
        c.contains("share-by-indexing instead of replication")
            && c.contains("Decompose the invocation index"),
        "shared-side concept"
    );
    // D: size-gate-covers-every-cost-factor (dispatch.rs L215-216 + gpu_m2p.rs L307)
    let c = concept("size-gate-covers-every-cost-factor");
    assert!(
        c.contains("Gating on a partial product") && c.contains("crossover bench must re-run"),
        "size-gate concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch46_pseudocode_parses() {
    // G1 for Batch 46: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "size-gate-covers-every-cost-factor",
        "declared-max-array-taxed-per-thread-under-runtime-bound",
        "shared-operand-side-pair-batch-index-decomposition",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-46 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-46 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-46 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch47_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // B (1 new): K/V projection hoist out of the query sweep
    assert!(
        ids.contains(&"per-key-projection-hoist-out-of-query-sweep"),
        "per-key projection hoist"
    );
    // E (1 new): head-concat out-projection phase boundary
    assert!(
        ids.contains(&"multi-head-out-proj-consumes-head-concat"),
        "head-concat out-proj"
    );
    // D (1 new): shared producer extract-once
    assert!(
        ids.contains(&"shared-producer-prefix-extract-once-per-input"),
        "shared producer extract-once"
    );
}

#[test]
fn batch47_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-47 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // B: per-key-projection-hoist-out-of-query-sweep (gpu_m2p.rs L97-112/L121-133 + m2p_attention.wgsl L55-91)
    let c = concept("per-key-projection-hoist-out-of-query-sweep");
    assert!(
        c.contains("re-derives the per-key projection inside the query loop")
            && c.contains("Project all K (and V) once per slice"),
        "per-key hoist concept"
    );
    // E: multi-head-out-proj-consumes-head-concat (m2p_attention.wgsl L92-98/L167-172 + gpu_m2p.rs L134-141)
    let c = concept("multi-head-out-proj-consumes-head-concat");
    assert!(
        c.contains("consumes the CONCATENATION of all heads") && c.contains("scalar-times-rowsum"),
        "head-concat concept"
    );
    // D: shared-producer-prefix-extract-once-per-input (shine.rs L229-230 + L138-144/L196-202)
    let c = concept("shared-producer-prefix-extract-once-per-input");
    assert!(
        c.contains("extract once and thread the shared artifact")
            && c.contains("the producer is pure"),
        "shared producer concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch47_pseudocode_parses() {
    // G1 for Batch 47: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "per-key-projection-hoist-out-of-query-sweep",
        "multi-head-out-proj-consumes-head-concat",
        "shared-producer-prefix-extract-once-per-input",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-47 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-47 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-47 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch49_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // A (1 new): runtime-arg loop bound demotes per-thread arrays to local memory
    assert!(
        ids.contains(&"runtime-arg-loop-bound-demotes-arrays-to-local-memory"),
        "runtime-arg local-memory demotion"
    );
}

#[test]
fn batch49_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-49 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // A: runtime-arg-loop-bound-demotes-arrays-to-local-memory (cudarc_kernels/mod.rs L211-235, Issue 706 / Bench 686)
    let c = concept("runtime-arg-loop-bound-demotes-arrays-to-local-memory");
    assert!(
        c.contains("no register form")
            && c.contains("hoist it into the kernel as a compile-time constant"),
        "runtime-arg bound concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch49_pseudocode_parses() {
    // G1 for Batch 49: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["runtime-arg-loop-bound-demotes-arrays-to-local-memory"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-49 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-49 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-49 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch51_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (1 new): read-batch slot membership decided at launch
    assert!(
        ids.contains(&"batched-read-slot-membership-decided-at-launch"),
        "read-slot membership"
    );
    // A (1 new): unpack loop indexed by container word
    assert!(
        ids.contains(&"unpack-loop-indexed-by-container-word"),
        "container-word unpack loop"
    );
    // N (1 new): dequant placement per component by traffic share
    assert!(
        ids.contains(&"dequant-placement-by-component-traffic-share"),
        "per-component dequant placement"
    );
}

#[test]
fn batch51_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-51 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // D: batched-read-slot-membership-decided-at-launch (kimi_k3_gpu_backward_batched.rs sync2 L1037-1099)
    let c = concept("batched-read-slot-membership-decided-at-launch");
    assert!(
        c.contains("NO persistent state to derive from")
            && c.contains("tag/metadata vec alongside the pushes"),
        "read-slot membership concept"
    );
    // A: unpack-loop-indexed-by-container-word (attention_q8kv_cubecl.rs L212-224)
    let c = concept("unpack-loop-indexed-by-container-word");
    assert!(
        c.contains("one load serves all its bytes")
            && c.contains("loop GRANULARITY = container granularity"),
        "container-word unpack concept"
    );
    // N: dequant-placement-by-component-traffic-share (attention_q8kv_cubecl.rs L4-5 + L398-400)
    let c = concept("dequant-placement-by-component-traffic-share");
    assert!(
        c.contains("OPPOSITE optimal dequant placements")
            && c.contains("B31 is CORRECT for the sidecar and WRONG for the payload"),
        "per-component placement concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch51_pseudocode_parses() {
    // G1 for Batch 51: every new rule's ref_shape AND opt_pattern must
    // syn-parse as valid Rust (the L2 pruner verdict).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "batched-read-slot-membership-decided-at-launch",
        "unpack-loop-indexed-by-container-word",
        "dequant-placement-by-component-traffic-share",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-51 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-51 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-51 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch52_rules_present() {
    let corpus = build_kernel_seed_corpus();
    let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    // D (1 new): phase-split per-position data liveness (Issue 708 H4)
    assert!(
        ids.contains(&"phase-split-per-position-data-liveness-row-storage"),
        "phase-split data liveness"
    );
}

#[test]
fn batch52_concepts_covered() {
    // Distinctive ASCII substrings located by rule_id (the 32a convention).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-52 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // D: phase-split-per-position-data-liveness-row-storage
    // (riir-ai forward_dflash.rs Phase-A project_q single-row q buffer)
    let c = concept("phase-split-per-position-data-liveness-row-storage");
    assert!(
        c.contains("the right value is NEVER STORED") && c.contains("DATA liveness"),
        "phase-split data-liveness concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch52_pseudocode_parses() {
    // G1 for Batch 52 (measured-GOAT distillation): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["phase-split-per-position-data-liveness-row-storage"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-52 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-52 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-52 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch54_rules_present() {
    // Batch 54 (measured-GOAT distillation, riir-train Bench 470).
    let corpus = build_kernel_seed_corpus();
    let ids = ["regularizer-above-noise-floor-coupled-to-damping-cap"];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-54 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch54_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let entry = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "regularizer-above-noise-floor-coupled-to-damping-cap")
        .expect("batch-54 rule missing");
    for concept in [
        "noise floor",
        "damping budget",
        "FULL-coefficient-schedule",
        "GRAM-DOMAIN restart",
        "is_finite",
    ] {
        assert!(
            entry.gpu_concept.contains(concept),
            "batch-54 concept '{concept}' not covered"
        );
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch54_pseudocode_parses() {
    // G1 for Batch 54 (measured-GOAT distillation): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["regularizer-above-noise-floor-coupled-to-damping-cap"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-54 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-54 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-54 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch55_rules_present() {
    // Batch 55 (measured-GOAT distillation, riir-ai Bench 691 + the Bench 642 re-hit).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "outlier-span-escapes-quantization-via-lossless-sidecar",
        "buffer-length-derived-from-allocation-not-binding",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-55 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch55_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 2] = [
        (
            "outlier-span-escapes-quantization-via-lossless-sidecar",
            [
                "massive-activation",
                "LOSSLESS f32 sidecar",
                "Hadamard rotation",
                "DEAD DATA",
                "bit-identical",
            ],
        ),
        (
            "buffer-length-derived-from-allocation-not-binding",
            [
                "ALLOCATION",
                "floor-divide",
                "BOTH ends",
                "benign case",
                "EXACT-sized",
            ],
        ),
    ];
    for (id, concepts) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-55 rule '{id}' missing"));
        for concept in concepts {
            assert!(
                entry.gpu_concept.contains(concept),
                "batch-55 concept '{concept}' not covered in '{id}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch55_pseudocode_parses() {
    // G1 for Batch 55 (measured-GOAT distillation): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "outlier-span-escapes-quantization-via-lossless-sidecar",
        "buffer-length-derived-from-allocation-not-binding",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-55 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-55 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-55 opt_pattern for '{id}' must parse"
        );
    }
}

// ════════════════════════════════════════════════════════════════════
// Plan 003 — Router ↔ corpus wiring tests
// ════════════════════════════════════════════════════════════════════

#[test]
fn batch56_rules_present() {
    // Batch 56 (measured-GOAT distillation, riir-ai Issues 719 + 714/720 — Bench 692 + the 719 parallel-suite run).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "concurrent-context-init-needs-process-wide-lock",
        "superseded-format-constructor-live-fields-only",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-56 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch56_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 2] = [
        (
            "concurrent-context-init-needs-process-wide-lock",
            [
                "init-outside-publish-on-success",
                "wedges NON-DETERMINISTICALLY",
                "CROSS-PATH",
                "DOUBLE-CHECK",
                "de-fang poison",
            ],
        ),
        (
            "superseded-format-constructor-live-fields-only",
            [
                "DEAD WEIGHT",
                "DISPATCH-SITE AUDIT",
                "TRIPWIRE",
                "norms_only()",
                "zero-size placeholder",
            ],
        ),
    ];
    for (id, concepts) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-56 rule '{id}' missing"));
        for concept in concepts {
            assert!(
                entry.gpu_concept.contains(concept),
                "batch-56 concept '{concept}' not covered in '{id}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch56_pseudocode_parses() {
    // G1 for Batch 56 (measured-GOAT distillation): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "concurrent-context-init-needs-process-wide-lock",
        "superseded-format-constructor-live-fields-only",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-56 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-56 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-56 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch57_rules_present() {
    // Batch 57 (measured-GOAT distillation, riir-ai Issue 715 — the flash-attention
    // smem role-reversal races + the multi-tile bit-exact regression gate).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "smem-slot-role-reversal-needs-barrier",
        "tile-loop-back-edge-needs-multi-tile-bitexact-gate",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-57 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch57_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 2] = [
        (
            "smem-slot-role-reversal-needs-barrier",
            [
                "slot ROLES reverse",
                "reads a weight as the tile max",
                "BLOCK-UNIFORM",
                "BIT-IDENTICAL (3/3)",
                "implementation twin",
            ],
        ),
        (
            "tile-loop-back-edge-needs-multi-tile-bitexact-gate",
            [
                "n_tiles=2",
                "low mantissa bits",
                "BOTH directions",
                "max_diff=0.0586",
                "untestable BY CONSTRUCTION",
            ],
        ),
    ];
    for (id, concepts) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-57 rule '{id}' missing"));
        for concept in concepts {
            assert!(
                entry.gpu_concept.contains(concept),
                "batch-57 concept '{concept}' not covered in '{id}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch57_pseudocode_parses() {
    // G1 for Batch 57 (measured-GOAT distillation): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "smem-slot-role-reversal-needs-barrier",
        "tile-loop-back-edge-needs-multi-tile-bitexact-gate",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-57 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-57 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-57 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch58_rules_present() {
    // Batch 58 (ternary_deltanet sibling-WIP unlock — the cudarc forward
    // orchestration twin: staging lifetime, graph lifecycle, the speculative
    // rotation/checkpoint seam, the multi-segment GEMV launcher).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "async-htod-host-source-outlives-copy",
        "output-buffer-rotation-pool-multi-forward",
        "write-before-read-state-skips-checkpoint",
        "captured-graph-bakes-buffer-addresses",
        "mid-pipeline-device-alloc-is-implicit-sync",
        "capture-region-dispatch-only",
        "same-input-gemv-segments-single-launch",
        "occupancy-grid-cap-refuted-uncapped-default",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-58 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch58_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 8] = [
        (
            "async-htod-host-source-outlives-copy",
            [
                "GPU-completion time",
                "persistent host staging",
                "dead stack slot",
                "host-side complement",
                "slice::from_ref",
            ],
        ),
        (
            "captured-graph-bakes-buffer-addresses",
            [
                "bakes in every buffer",
                "Address-stable",
                "Identity-changing",
                "freed pointers",
                "in-place",
            ],
        ),
        (
            "output-buffer-rotation-pool-multi-forward",
            [
                "ownership exchange",
                "never un-rotated mid-run",
                "pre-speculation prediction",
                "K+1 position-aligned",
                "O(1)",
            ],
        ),
        (
            "mid-pipeline-device-alloc-is-implicit-sync",
            [
                "implicitly synchronize",
                "HIDDEN BARRIER",
                "no-intermediate-sync",
                "statically known maximum",
                "correctness precondition",
            ],
        ),
        (
            "capture-region-dispatch-only",
            [
                "ONCE at capture time",
                "NEVER on replay",
                "not capturable",
                "non-default stream",
                "host-side position advancement",
            ],
        ),
        (
            "occupancy-grid-cap-refuted-uncapped-default",
            [
                "REFUTED by measurement",
                "wavefront locality",
                "87.1",
                "768",
                "measurement/tuning apparatus",
            ],
        ),
        (
            "write-before-read-state-skips-checkpoint",
            [
                "READ-MODIFY",
                "WRITE-BEFORE-READ",
                "re-execution",
                "append-only",
                "ZERO CPU",
            ],
        ),
        (
            "same-input-gemv-segments-single-launch",
            [
                "kernel-latency floor",
                "497 GEMV launches",
                "multi-segment",
                "m=0",
                "ride along",
            ],
        ),
    ];
    for (id, concepts) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-58 rule '{id}' missing"));
        for concept in concepts {
            assert!(
                entry.gpu_concept.contains(concept),
                "batch-58 concept '{concept}' not covered in '{id}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch58_pseudocode_parses() {
    // G1 for Batch 58 (ternary_deltanet unlock, file mining): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "async-htod-host-source-outlives-copy",
        "output-buffer-rotation-pool-multi-forward",
        "write-before-read-state-skips-checkpoint",
        "captured-graph-bakes-buffer-addresses",
        "mid-pipeline-device-alloc-is-implicit-sync",
        "capture-region-dispatch-only",
        "same-input-gemv-segments-single-launch",
        "occupancy-grid-cap-refuted-uncapped-default",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-58 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-58 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-58 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch60_rules_present() {
    // Batch 60 (measured-GOAT distillation — riir-train Issue 460 /
    // katgpt-rs e2853fbf: the MLA backward's saturated output gate
    // 0/0 NaN, fixed by algebraic cancellation).
    let corpus = build_kernel_seed_corpus();
    let ids = ["pre-gate-recovery-by-division-cancels-algebraically"];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-60 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch60_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 1] = [(
        "pre-gate-recovery-by-division-cancels-algebraically",
        [
            "HARD ZERO",
            "0/0 = NaN",
            "the g cancels",
            "information-theoretically unrecoverable",
            "that asymmetry is the proof",
        ],
    )];
    for (id, concepts) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-60 rule '{id}' missing"));
        for concept in concepts {
            assert!(
                entry.gpu_concept.contains(concept),
                "batch-60 concept '{concept}' not covered in '{id}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch60_pseudocode_parses() {
    // G1 for Batch 60 (measured-GOAT distillation): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["pre-gate-recovery-by-division-cancels-algebraically"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-60 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-60 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-60 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch63_rules_present() {
    // Batch 63 (the CubeCL twin unlock — ternary_deltanet_gpu_forward.rs
    // orchestration twin; Lane B structurally empty, Lane A's 3).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "same-input-gemv-weight-concat-at-upload",
        "deferred-residual-fused-into-next-layer-norm",
        "reset-state-in-place-fill-never-realloc",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-63 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch63_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 3] = [
        (
            "same-input-gemv-weight-concat-at-upload",
            [
                "[Wa; Wb]",
                "stock kernel",
                "offset-consumers",
                "frozen",
                "multi-segment launcher",
            ],
        ),
        (
            "deferred-residual-fused-into-next-layer-norm",
            [
                "NEXT loop iteration",
                "tail",
                "first iteration",
                "composition ladder",
                "fuse-add-rms-norm",
            ],
        ),
        (
            "reset-state-in-place-fill-never-realloc",
            [
                "SIGKILL",
                "zero-fill",
                "cursor",
                "pool",
                "rewind-overwrite-in-place",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-63 rule '{id}' missing"));
        let c = e.gpu_concept.as_str();
        for n in needles {
            assert!(
                c.contains(n),
                "batch-63 concept for '{id}' missing phrase '{n}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch63_pseudocode_parses() {
    // G1 for Batch 63 (the CubeCL twin): syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "same-input-gemv-weight-concat-at-upload",
        "deferred-residual-fused-into-next-layer-norm",
        "reset-state-in-place-fill-never-realloc",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-63 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-63 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-63 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch70_rules_present() {
    // Batch 70 (arXiv:2608.13757 — the barrier-free synchronization
    // distill: per-dependency closed-form waits from monotone loop
    // counters, shared-counter progress decode, masked wait collapse).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "barrier-free-per-dependency-wait-from-loop-counters",
        "shared-counter-decode-producer-progress",
        "masked-branchless-wait-collapse",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-70 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch70_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 3] = [
        (
            "barrier-free-per-dependency-wait-from-loop-counters",
            [
                "all-worker barrier",
                "monotone entry count",
                "closed form",
                "stalls only",
                "per-dependency",
            ],
        ),
        (
            "shared-counter-decode-producer-progress",
            [
                "shared counter",
                "N*r - (N - K)",
                "granularity ladder",
                "in-order retirement",
                "LICM",
            ],
        ),
        (
            "masked-branchless-wait-collapse",
            [
                "disjunctive condition",
                "multiplicative 0/1 mask",
                "single threshold",
                "no branch",
                "is_gt",
            ],
        ),
    ];
    for (id, needles) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-70 rule '{id}' missing"));
        for needle in needles {
            assert!(
                entry.gpu_concept.contains(needle),
                "batch-70 concept for '{id}' missing needle '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch70_pseudocode_parses() {
    // G1 for Batch 70: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "barrier-free-per-dependency-wait-from-loop-counters",
        "shared-counter-decode-producer-progress",
        "masked-branchless-wait-collapse",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-70 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-70 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-70 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch71_rules_present() {
    // Batch 71 (oMLX external distill — the rotated-ring speculative
    // undo + the GPU-API thread-affinity discipline; Research 083).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "rotated-ring-undo-via-rebind-reference-snapshot",
        "gpu-api-thread-affinity-snapshot-metrics",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-71 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch71_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 2] = [
        (
            "rotated-ring-undo-via-rebind-reference-snapshot",
            [
                "rotated ring cache",
                "phantom",
                "rebinds",
                "restore the references and replay",
                "detach-copy",
            ],
        ),
        (
            "gpu-api-thread-affinity-snapshot-metrics",
            [
                "background monitor or enforcer thread",
                "corrupts command-buffer internal state",
                "Snapshot",
                "Flag-and-defer",
                "Cache-once",
            ],
        ),
    ];
    for (id, needles) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-71 rule '{id}' missing"));
        for needle in needles {
            assert!(
                entry.gpu_concept.contains(needle),
                "batch-71 concept for '{id}' missing needle '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch71_pseudocode_parses() {
    // G1 for Batch 71: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "rotated-ring-undo-via-rebind-reference-snapshot",
        "gpu-api-thread-affinity-snapshot-metrics",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-71 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-71 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-71 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn kernel_categories_table_has_15_entries() {
    assert_eq!(KERNEL_CATEGORIES.len(), 15, "Plan 003: 15 categories (A–O)");
}

#[test]
fn kernel_categories_cover_all_corpus_rules_exhaustively() {
    // Every rule_id in the 734-rule corpus must appear in exactly one category
    // (re-pinned 734 at Batch 197; the comment read 728 through B196).
    let corpus = build_kernel_seed_corpus();
    let corpus_ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    let mut categorized: Vec<&str> = Vec::new();
    for (_, _, rule_ids) in KERNEL_CATEGORIES {
        categorized.extend_from_slice(rule_ids);
    }
    // Same length (no missing, no extra).
    assert_eq!(
        categorized.len(),
        corpus_ids.len(),
        "Plan 003: KERNEL_CATEGORIES rule count ({}) must match corpus ({})",
        categorized.len(),
        corpus_ids.len()
    );
    // No duplicate category assignments.
    let mut sorted = categorized.clone();
    sorted.sort_unstable();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        before,
        "Plan 003: rule appears in multiple categories"
    );
    // Every corpus rule has a category home.
    for rid in &corpus_ids {
        assert!(
            categorized.contains(rid),
            "Plan 003: corpus rule '{rid}' has no category in KERNEL_CATEGORIES"
        );
    }
    // Every category entry exists in the corpus (no stale rule_ids).
    for rid in &categorized {
        assert!(
            corpus_ids.contains(rid),
            "Plan 003: KERNEL_CATEGORIES references '{rid}' not in corpus"
        );
    }
}

#[test]
fn kernel_categories_counts_match_plan_003_table() {
    // Pin per-category rule counts to the Plan 003 table (updated for
    // Batch 193: A +3 the offset-gated KV first-append + the dual-layout
    // stage transpose + the plane-form activation topology, B +1 the
    // branch-free top-2 insert, D +6 the first-CB mapping split + the
    // deadline-bounded completion poll + the readback-domain exclusion +
    // the adoption idle-fill + the seed earliest-submission trio + the
    // DRAM-streaming adoption-trial rotation, E +1 the width-block grid
    // split, G +1 the host-observable-predictor state store, L +1 the
    // register-built right operand, N +1 the construction-identity probe
    // replacement;
    // Batch 196: D +1 the register-width topk tiled-radix fallback, L +1
    // the affine block address pipeliner lead (the TensorFold A− pack,
    // Plan 195);
    // Batch 192: D +3 the MoE tile-picker shape-truth + the descriptor-
    // set destroy-epoch memo + the vendor-keyed lane-partition, N +1 the
    // dot-family f32-accumulate;
    // Batch 190: O +1 the clamp-runtime-divisor umax LSR rule — the
    // IR-strength-reduction family's first runtime-divisor member;
    // Batch 189: B +1 the exact bf16-domain activation LUT, D +2 the
    // shape-bucketed graph-cache LRU + the fused-serving measured-
    // negative ledger, F +1 the ancestor-mask bit-visibility tree
    // attention, G +1 the two-chunk state-independent overlap;
    // Batch 188: A +2 the low-bit warp-run load + the register prefetch
    // ring, D +1 the shape-only row-invariant plan, H +1 the sorted-
    // gather run-scan elimination; Batch 187: A +1 the j-major
    // accumulator transpose writeback,
    // B +3 the register-band partition + the lut16-pshufb folded-bias
    // expansion + the GFNI sign-mask expand, E +1 the chunk-downsize
    // work-stealing microtile floor; Batch 186 before that: A +6 the
    // ggml-cpu tiled staging family, B +2, C +1, D +3, H +1, M +1,
    // N +1).
    // Batch 197 before that: A +1 the GQA zero-stride broadcast, E +2 the
    // composed-backward lead + the threadgroup-scope single-writer,
    // G +3 the entry-state checkpoint + the forward-save cache + the
    // chain-rule postprocess; the mlx VJP distill).
    let expected = [
        (KernelCategory::MemoryAccess, 156),
        (KernelCategory::Compute, 77),
        (KernelCategory::Fusion, 51),
        (KernelCategory::Dispatch, 163),
        (KernelCategory::Parallelism, 70),
        (KernelCategory::FlashAttnTiling, 18),
        (KernelCategory::LinearAttnRecurrence, 34),
        (KernelCategory::MoeDispatchBackward, 36),
        (KernelCategory::NsaBlockSparse, 7),
        (KernelCategory::VarlenChunkedReduction, 5),
        (KernelCategory::AutotuneDispatch, 14),
        (KernelCategory::ConvGemmLayout, 19),
        (KernelCategory::KernelPolymorphism, 21),
        (KernelCategory::DtypeAlignment, 56),
        (KernelCategory::IrStrengthReduction, 7),
    ];
    for (cat, count) in expected {
        let entry = KERNEL_CATEGORIES
            .iter()
            .find(|(c, _, _)| *c == cat)
            .unwrap_or_else(|| panic!("Plan 003: category {cat:?} missing from KERNEL_CATEGORIES"));
        assert_eq!(
            entry.2.len(),
            count,
            "Plan 003: category {cat:?} has {} rules, expected {count}",
            entry.2.len()
        );
    }
    // Sum must equal 734.
    let total: usize = KERNEL_CATEGORIES.iter().map(|(_, _, r)| r.len()).sum();
    assert_eq!(total, 734, "Plan 003: category totals must sum to 734 (Batch 197 +6: E +2 the composed-backward lead + the threadgroup-scope single-writer, G +3 the entry-state checkpoint + the forward-save cache + the chain-rule postprocess, A +1 the GQA zero-stride broadcast; re-pinned 734 with the honest label)");
}

#[test]
fn build_corpus_categorized_round_trips_all_rules() {
    // build_corpus_categorized groups the corpus by category. Every rule
    // should appear in exactly one bucket, and the total count should match.
    let grouped = build_corpus_categorized();
    assert_eq!(grouped.len(), 15, "Plan 003: 15 category buckets");
    let total: usize = grouped.iter().map(|(_, rules)| rules.len()).sum();
    assert_eq!(
        total, 734,
        "Plan 003: grouped corpus has {total} rules, expected 734"
    );
    // No empty buckets.
    for (cat, rules) in &grouped {
        assert!(
            !rules.is_empty(),
            "Plan 003: category {cat:?} has zero rules"
        );
    }
}

#[test]
fn batch72_rules_present() {
    // Batch 72 (frguthmann/NVIDIA external distill — the dirty-encoding
    // write elision; Research 084).
    let corpus = build_kernel_seed_corpus();
    let ids = ["skip-unchanged-writes-via-dirty-encoding"];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-72 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch72_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 1] = [(
        "skip-unchanged-writes-via-dirty-encoding",
        [
            "dirty = plain + 2",
            "ALTERNATE rotations",
            "ZERO writes",
            "REGIME-GATED",
            "lazy cleanup",
        ],
    )];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-72 rule '{id}' missing"));
        for needle in needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-72 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch72_pseudocode_parses() {
    // G1 for Batch 72: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["skip-unchanged-writes-via-dirty-encoding"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-72 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-72 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-72 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch77_rules_present() {
    // Batch 77 (lucebox PR #625 external distill — the DSpark qwen3.5
    // drafters; Research 091 / Plan 035).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "weight-stream-non-temporal-loads",
        "batch-width-routed-dual-tile-twins",
        "tree-verify-recurrent-state-parent-reload",
        "shared-activation-quantize-memo-per-eval",
        "vendor-batched-gemm-contiguity-staging",
        "gate-prep-fold-into-consumer-via-packed-side-tensor",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-77 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch77_concepts_covered() {
    let corpus = build_kernel_seed_corpus();
    let checks: [(&str, [&str; 5]); 6] = [
        (
            "weight-stream-non-temporal-loads",
            [
                "STREAMING operands",
                "REUSED operands",
                "__ldcs",
                "bit-identical",
                "evict-first",
            ],
        ),
        (
            "batch-width-routed-dual-tile-twins",
            [
                "namespace-reinclude",
                "occupancy",
                "crossover",
                "SWEEP",
                "mmq_x=32",
            ],
        ),
        (
            "tree-verify-recurrent-state-parent-reload",
            [
                "parent_t != t-1",
                "read-after-write",
                "O(1)",
                "TREE_ROOT_PARENT",
                "rotated-ring-undo",
            ],
        ),
        (
            "shared-activation-quantize-memo-per-eval",
            [
                "LIFO",
                "MAX_BUFFERS",
                "concurrent",
                "256",
                "THIRD mechanism",
            ],
        ),
        (
            "vendor-batched-gemm-contiguity-staging",
            ["pinned-host", "contiguize", "k-limit", "CS=32", "vendor"],
        ),
        (
            "gate-prep-fold-into-consumer-via-packed-side-tensor",
            [
                "src[9]",
                "op_params[10]",
                "HARD-REJECT",
                "softplus",
                "UNUSED src slot",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-77 rule '{id}' missing"));
        for needle in needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-77 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch77_pseudocode_parses() {
    // G1 for Batch 77: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "weight-stream-non-temporal-loads",
        "batch-width-routed-dual-tile-twins",
        "tree-verify-recurrent-state-parent-reload",
        "shared-activation-quantize-memo-per-eval",
        "vendor-batched-gemm-contiguity-staging",
        "gate-prep-fold-into-consumer-via-packed-side-tensor",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-77 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-77 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-77 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch79_rules_present() {
    // Batch 79 (BAIR K-Search + cubek-matmul external distill --
    // Research 093 / Plan 037).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "first-order-affine-recurrence-associative-scan",
        "plane-specialization-load-compute-role-split",
        "vectorized-smem-write-no-bank-padding",
        "ordered-lhs-skip-smem-fragments-as-second-buffer",
        "interleave-loads-between-tile-matmuls",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-79 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch79_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson, 6 strikes).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "first-order-affine-recurrence-associative-scan",
            &[
                "associative operator over pair-space",
                "O(log N) dependent steps",
                "spec-seeded, search-realized",
                "THE COMBINE COST PICKS THE STRATEGY",
                "relative error",
                "chunkwise-parallel-recurrence",
            ],
        ),
        (
            "plane-specialization-load-compute-role-split",
            &[
                "load-only planes run the whole global->shared load job",
                "PlaneFlowPartitionRule",
                "never specialize WITHIN a plane",
                "underperforms on Metal",
                "fold-query-tokens-for-occupancy",
            ],
        ),
        (
            "vectorized-smem-write-no-bank-padding",
            &[
                "UNNECESSARY when the smem stores are VECTORIZED",
                "SwizzleMode",
                "Refutation-type",
                "zero pad/bank/skew fields",
                "pad-uniform-pod-to-min-binding-size",
            ],
        ),
        (
            "ordered-lhs-skip-smem-fragments-as-second-buffer",
            &[
                "fragments themselves",
                "ORDERING-REPLACES-SYNCHRONIZATION",
                "1x k-step",
                "sequential halves",
                "smem-staging-gated-by-cross-thread-reuse",
            ],
        ),
        (
            "interleave-loads-between-tile-matmuls",
            &[
                "asynchrony is INDUCED by instruction-stream placement",
                "TileMatmulCompleted",
                "EventLoadingMode",
                "REMOVES the overlap",
                "ordered-lhs-skip-smem-fragments-as-second-buffer",
            ],
        ),
    ];
    for &(id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-79 rule '{id}' missing"));
        for needle in needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-79 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch79_pseudocode_parses() {
    // G1 for Batch 79: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "first-order-affine-recurrence-associative-scan",
        "plane-specialization-load-compute-role-split",
        "vectorized-smem-write-no-bank-padding",
        "ordered-lhs-skip-smem-fragments-as-second-buffer",
        "interleave-loads-between-tile-matmuls",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-79 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-79 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-79 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch80_rules_present() {
    // Batch 80 (WebGPU dispatch-overhead characterization distill --
    // arXiv:2604.02344v1 / Research 094 / Plan 038).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "dispatch-cost-sequential-measurement-not-single-op",
        "fusion-payback-gated-on-per-dispatch-overhead",
        "grid-collapsing-mega-kernel-occupancy-trap",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-80 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch80_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson, 6 strikes).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "dispatch-cost-sequential-measurement-not-single-op",
            &[
                "10-60x overestimate",
                "sync ONCE at the tail",
                "submit DOMINATES",
                "577 submits/token",
                "chain-dispatches-single-sync-point",
                "first-readback-pays-queue-drain",
            ],
        ),
        (
            "fusion-payback-gated-on-per-dispatch-overhead",
            &[
                "payback = dispatches_saved x per_dispatch_overhead",
                "0.92x on CUDA (7.4 us/dispatch -- too little to save)",
                "B* >= 7 even for the",
                "single-op benchmark overestimates 10-60x",
                "kernelize-host-chain-after-sync-batching-floor",
                "merge-dispatches-into-single-compute-pass",
            ],
        ),
        (
            "grid-collapsing-mega-kernel-occupancy-trap",
            &[
                "workgroupBarrier is intra-workgroup only",
                "~17,000 sequential MADs per thread",
                "7 dispatches -> 3 measured 2.01x on Metal",
                "honestly INCONCLUSIVE (p>0.38 both backends)",
                "epilogue-must-be-workgroup-local",
                "fold-query-tokens-for-occupancy",
            ],
        ),
    ];
    for &(id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-80 rule '{id}' missing"));
        for needle in needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-80 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch80_pseudocode_parses() {
    // G1 for Batch 80: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "dispatch-cost-sequential-measurement-not-single-op",
        "fusion-payback-gated-on-per-dispatch-overhead",
        "grid-collapsing-mega-kernel-occupancy-trap",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-80 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-80 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-80 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch81_rules_present() {
    // Batch 81 (lucebox concurrency-stack distill -- PRs 651/659/668;
    // Research 095 / Plan 039).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "virtual-tail-packed-tree-verify-over-paged-kv",
        "gdn-transition-replay-log-commit-many",
        "cross-lane-column-packed-projections",
        "identity-keyed-graph-retention-stable-arena",
        "staged-append-rollback-lifo-retry-blocks",
        "fail-closed-multi-resource-commit-transaction",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-81 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch81_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "virtual-tail-packed-tree-verify-over-paged-kv",
            &[
                "VIRTUAL TAIL",
                "ancestor-visibility walk",
                "phys = -1",
                "tree-verify-recurrent-state-parent-reload",
                "kv-window-device-resident-ring-append",
            ],
        ),
        (
            "gdn-transition-replay-log-commit-many",
            &[
                "[gate | key | delta]",
                "fmaf(key, delta, gate * current)",
                "OWNS its storage",
                "first-order-affine-recurrence-associative-scan",
                "write-before-read-state-skips-checkpoint",
            ],
        ),
        (
            "cross-lane-column-packed-projections",
            &[
                "SHARED WEIGHT",
                "AXIS-TRANSPOSED",
                "single-lane",
                "same-input-gemv-weight-concat-at-upload",
                "op-major-backward-loop-inversion-batch-sequence-axis",
            ],
        ),
        (
            "identity-keyed-graph-retention-stable-arena",
            &[
                "BUCKET",
                "nodes[0]",
                "SALT",
                "hot-swap-weights-preserve-autotune-caches",
                "captured-graph-bakes-buffer-addresses",
            ],
        ),
        (
            "staged-append-rollback-lifo-retry-blocks",
            &[
                "LIFO",
                "GLOBAL free list",
                "physical rows",
                "captured-graph-bakes-buffer-addresses",
                "rotated-ring-undo-via-rebind-reference-snapshot",
            ],
        ),
        (
            "fail-closed-multi-resource-commit-transaction",
            &[
                "VALIDATE",
                "COMMIT_PREVALIDATED",
                "NO recoverable error may escape",
                "verify-once-at-trust-boundary",
                "mixed pre/post-commit state",
            ],
        ),
    ];
    for &(id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-81 rule '{id}' missing"));
        for needle in needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-81 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch81_pseudocode_parses() {
    // G1 for Batch 81: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "virtual-tail-packed-tree-verify-over-paged-kv",
        "gdn-transition-replay-log-commit-many",
        "cross-lane-column-packed-projections",
        "identity-keyed-graph-retention-stable-arena",
        "staged-append-rollback-lifo-retry-blocks",
        "fail-closed-multi-resource-commit-transaction",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-81 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-81 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-81 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch82_rules_present() {
    // Batch 82 (TreeWY + Launch-Bound arXiv distill -- 2608.20961 +
    // 2608.26612; Research 096 / Plan 043).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "wy-transform-tree-verify-state-free",
        "work-scaling-probe-launch-bound",
        "capture-break-count-not-perf-target",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-82 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch82_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "wy-transform-tree-verify-state-free",
            &[
                "DFS PRE-ORDER",
                "strictly lower-triangular",
                "pseudo-value",
                "128x smaller",
                "NOT a throughput win",
                "CONDITIONAL on memory pressure",
            ],
        ),
        (
            "work-scaling-probe-launch-bound",
            &[
                "32x the work",
                "LAUNCH-BOUND",
                "0.999x",
                "integration-bound, not Amdahl-bound",
                "record_function",
                "undercounts twice",
            ],
        ),
        (
            "capture-break-count-not-perf-target",
            &[
                "TRACEABILITY DIAGNOSTIC",
                "3x SLOWER",
                "unbacked symbolic shapes",
                "PIECEWISE capture",
                "never force full capture",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-82 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-82 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch82_pseudocode_parses() {
    // G1 for Batch 82: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "wy-transform-tree-verify-state-free",
        "work-scaling-probe-launch-bound",
        "capture-break-count-not-perf-target",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-82 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-82 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-82 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch84_pseudocode_parses() {
    // G1 for Batch 84: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "pad-tail-to-fixed-tile",
        "token-local-only-approximate-accelerator",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-84 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-84 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-84 opt_pattern for '{id}' must parse"
        );
    }
}
#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch92_pseudocode_parses() {
    // G1 for Batch 92: syn-parse gate (the ktransformers Marlin distill).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "device-m-runtime-mblock-graph-stable-gemm",
        "cpasync-multistage-tile-pipeline",
        "register-double-buffer-operand-pipeline",
        "ldmatrix-mma-layout-loads",
        "xor-swizzle-smem-static-index-tables",
        "multi-pointer-dependency-break",
        "quant-group-scale-fetch-gating",
        "lop3-int4-to-half2-dequant",
        "prmt-int8-to-half2-dequant",
        "stripe-sms-l2-semaphore-reduce",
        "mma-fragment-layout-m16n8k16",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-92 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-92 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-92 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch95_pseudocode_parses() {
    // G1 for Batch 95: syn-parse gate (the mlx-lm gated_delta.py GDN
    // prefill distill -- simd-group row packing, whole-T zero-barrier
    // single-dispatch scan, source-owned reduction comparator).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "simdgroup-row-packing-reduction-amortization",
        "whole-sequence-zero-barrier-single-dispatch-scan",
        "reduction-restructure-needs-source-owned-comparator",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-95 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-95 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-95 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch96_rules_present() {
    // Batch 96 (PrismML fork Metal quant-MV distill -- Research 114 /
    // Plan 060): 4 kernel_opt rules (N +2, D +1, B +1).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "telescoped-digit-coefficients-byte-fma-dot",
        "bit-plane-plane-sums-input-side-offset-dot",
        "wide-row-copy-tile-rows-across-workgroups",
        "fwht-shuffle-xor-butterfly-register-resident",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-96 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch96_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "telescoped-digit-coefficients-byte-fma-dot",
            &[
                "express the 2-bit field shifts as float floor ops",
                "tolerance-gated optimization (rel-error gate), NEVER a bit-identity candidate",
                "the codec's -1 offset rides as the accumulator SEED",
            ],
        ),
        (
            "bit-plane-plane-sums-input-side-offset-dot",
            &[
                "acc_lo + 2.0 * acc_hi",
                "computed ONCE per slice and SHARED across every weight row",
                "POSITIONAL digits of one code, weighted 2^k",
            ],
        ),
        (
            "wide-row-copy-tile-rows-across-workgroups",
            &[
                "cannot saturate the memory bandwidth for rows of 100k+",
                "the threshold is a dispatch fact, not a device query",
                "what must scale is the number of workgroups in flight",
            ],
        ),
        (
            "fwht-shuffle-xor-butterfly-register-resident",
            &[
                "the xor mask IS the butterfly partner",
                "ZERO shared memory and ZERO barriers",
                "TRANSFORM side of the Hadamard family",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-96 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-96 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch96_pseudocode_parses() {
    // G1 for Batch 96: syn-parse gate (the PrismML Metal quant-MV
    // distill -- telescoped digit coefficients, bit-plane plane sums
    // with an input-side offset, wide-row workgroup tiling, the FWHT
    // register-resident xor butterfly).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "telescoped-digit-coefficients-byte-fma-dot",
        "bit-plane-plane-sums-input-side-offset-dot",
        "wide-row-copy-tile-rows-across-workgroups",
        "fwht-shuffle-xor-butterfly-register-resident",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-96 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-96 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-96 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch87_pseudocode_parses() {
    // G1 for Batch 87: syn-parse gate (the App-I boundary pair).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "state-normalization-between-updates-breaks-associative-scan",
        "dynamic-kernel-params-break-prefix-scan",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-87 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-87 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-87 opt_pattern for '{id}' must parse"
        );
    }
}
#[test]
#[allow(clippy::cast_precision_loss)]
fn batch87_state_norm_breaks_associative_scan() {
    // G4 for Batch 87 (the App-I breaker, falsifiable): the scan of a
    // state-normalized recurrence computes the RAW affine trajectory
    // A_t*h0 + B_t (exactly what the pair-space scan returns), while
    // the production loop computes l2n(A_t*h0 + B_t). The two diverge
    // MATERIALLY -- structural wrongness, not fp association noise.
    fn l2n2(v: [f32; 2]) -> [f32; 2] {
        let n = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1e-6);
        [v[0] / n, v[1] / n]
    }
    // ground truth: the sequential STATE-NORMALIZED loop (channel 1
    // gets half the drive so the normalized state is non-degenerate).
    fn sequential_normed(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut h = [0.0f32; 2];
        let mut out = Vec::with_capacity(2 * a.len());
        for i in 0..a.len() {
            h = l2n2([a[i] * h[0] + b[i], a[i] * h[1] + 0.5 * b[i]]);
            out.extend_from_slice(&h);
        }
        out
    }
    // what the pair-space scan computes: the RAW affine trajectory
    // (B79's combine on (a, b) pairs yields exactly A_t*h0 + B_t --
    // the B79 gate proves scan ~= raw-affine; THIS gate proves
    // raw-affine != normalized).
    fn raw_affine_trajectory(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut h = [0.0f32; 2];
        let mut out = Vec::with_capacity(2 * a.len());
        for i in 0..a.len() {
            h = [a[i] * h[0] + b[i], a[i] * h[1] + 0.5 * b[i]];
            out.extend_from_slice(&h);
        }
        out
    }
    let a: Vec<f32> = (0..128).map(|i| 0.9f32 + (i % 5) as f32 * 0.02).collect();
    let b: Vec<f32> = (0..128).map(|i| 0.5f32 - (i % 7) as f32 * 0.1).collect();
    let truth = sequential_normed(&a, &b);
    let scan = raw_affine_trajectory(&a, &b);
    // sanity: the truth IS normalized (unit state norm throughout).
    for chunk in truth.chunks(2) {
        let norm = (chunk[0] * chunk[0] + chunk[1] * chunk[1]).sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "truth states must be unit-norm: {norm}"
        );
    }
    // the divergence must be MATERIAL: the raw trajectory escapes the
    // unit sphere, so at some position |scan - truth| > 1e-3 * scale.
    let scale = truth.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let max_diff = truth
        .iter()
        .zip(&scan)
        .map(|(s, t)| (s - t).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff > 1e-3 * scale,
        "state-normalized truth and raw-affine scan must diverge materially: {max_diff} vs scale {scale}"
    );
}
#[test]
// the f32 rounding IS the subject under test (tree-order association
// drift) -- the int->f32 casts feed float probes, precision loss is
// the point
#[allow(clippy::cast_precision_loss)]
fn batch79_associative_scan_rel_error_gate() {
    // G4 for Batch 79 (the ported-numeric MANDATE): the associative
    // scan re-associates FP accumulation, so it can NEVER be
    // bit-identical to the sequential form -- the gate is relative
    // error over ADVERSARIAL sequences, never bitwise equality.
    fn combine(later: (f32, f32), earlier: (f32, f32)) -> (f32, f32) {
        (later.0 * earlier.0, later.0 * earlier.1 + later.1)
    }
    fn scan_pairs(a: &[f32], b: &[f32]) -> Vec<f32> {
        let n = a.len();
        let mut pairs: Vec<(f32, f32)> = a.iter().zip(b.iter()).map(|(&x, &y)| (x, y)).collect();
        let mut stride = 1usize;
        while stride < n {
            // reverse order: pairs[i - stride] still holds the
            // previous round's value when pairs[i] updates
            for i in (stride..n).rev() {
                pairs[i] = combine(pairs[i], pairs[i - stride]);
            }
            stride *= 2;
        }
        pairs.into_iter().map(|p| p.1).collect()
    }
    fn sequential(a: &[f32], b: &[f32]) -> Vec<f32> {
        let mut h = 0.0f32;
        let mut out = Vec::with_capacity(a.len());
        for i in 0..a.len() {
            h = a[i] * h + b[i];
            out.push(h);
        }
        out
    }
    let mut max_rel = 0.0f32;
    for seed in 0..4u64 {
        let mut a = Vec::with_capacity(256usize);
        let mut b = Vec::with_capacity(256usize);
        for i in 0..256usize {
            let (ai, bi) = match seed {
                // family 0: exponent crossings -- h swings across 2^n
                // boundaries via alternating decay + signed inputs
                0 => (
                    if i % 2 == 0 { 0.9f32 } else { 0.99f32 },
                    if i % 3 == 0 { -1.0f32 } else { 1.0f32 },
                ),
                // family 1: magnitude swings -- b alternating 1e6/1e-6
                1 => (0.99f32, if i % 2 == 0 { 1e6f32 } else { 1e-6f32 }),
                // family 2: long decay chain -- near-1 gates, tiny b
                2 => (0.999_95f32, 1e-4f32 + (i % 7) as f32 * 1e-5f32),
                // family 3: xorshift64 pseudo-random, bounded
                _ => {
                    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                        | 1 ^ (i as u64).wrapping_mul(0x2545_F491_4F6C_DD1D);
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    let f = (x >> 11) as f32 / (1u64 << 53) as f32;
                    (0.5f32 + 0.499 * f, 0.001f32 + f)
                }
            };
            a.push(ai);
            b.push(bi);
        }
        let seq = sequential(&a, &b);
        let scan = scan_pairs(&a, &b);
        // allclose-shaped gate: rtol on the value PLUS an absolute
        // term of 100 * EPSILON * scale (scale = largest |h| the
        // sequential form reaches). Pure rel-to-reference is
        // unbounded exactly where the adversarial families bite --
        // at cancellation points |ref| is tiny while the drift is
        // set by the ulp of the MAGNITUDES that produced it.
        let scale = seq.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let atol = 100.0 * f32::EPSILON * scale;
        for (s, t) in seq.iter().zip(&scan) {
            let diff = (s - t).abs();
            let bound = atol + 1e-3 * s.abs();
            max_rel = max_rel.max(diff / bound);
            assert!(
                diff <= bound,
                "seed {seed}: diff {diff} exceeds bound {bound} (scale {scale})"
            );
        }
    }
    // NON-VACUOUS halves: (a) the two forms must actually differ
    // bitwise somewhere (association order matters -- else this gate
    // proves nothing about FP honesty), and (b) the measured max_rel
    // must be nonzero.
    let a: Vec<f32> = (0..256).map(|i| 0.999_95f32 + i as f32 * 1e-7).collect();
    let b: Vec<f32> = (0..256).map(|i| 1e-4f32 + (i % 7) as f32 * 1e-5).collect();
    let seq = sequential(&a, &b);
    let scan = scan_pairs(&a, &b);
    assert!(
        seq.iter()
            .zip(&scan)
            .any(|(s, t)| s.to_bits() != t.to_bits()),
        "non-vacuous: scan and sequential must differ bitwise somewhere"
    );
    assert!(max_rel > 0.0, "max_rel measured nothing");
}

#[test]
fn batch94_rules_present() {
    // Batch 94 (oMLX PR #3260 SSD expert streaming + Soft-REAP distill --
    // Research 110 / Plan 058): 8 kernel_opt D rules.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "cross-run-profiles-key-on-content-not-stat-triplets",
        "fill-gate-speculation-until-working-set-resident",
        "decayed-frequency-eviction-recency-tiebreak-route-protection",
        "defer-promoted-write-to-first-consumer-dispatch",
        "rollback-snapshot-covers-all-mutable-state",
        "scratch-bank-isolates-one-shot-operands-from-hot-cache",
        "sort-consumer-routes-by-physical-slot",
        "row-addressable-operand-index-not-full-tensor-map",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-94 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch94_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "cross-run-profiles-key-on-content-not-stat-triplets",
            &[
                "mtime-preserving copy",
                "unverified claim",
                "write the guard at the derivation, not at one caller",
            ],
        ),
        (
            "fill-gate-speculation-until-working-set-resident",
            &[
                "166.965 s speculative vs 14.709 s checked",
                "0.579 s vs 0.556 s",
                "LATCH the readiness bit",
            ],
        ),
        (
            "decayed-frequency-eviction-recency-tiebreak-route-protection",
            &[
                "recency as TIE-BREAK only",
                "score BOTH hits and misses",
                "FAIL LOUD when no candidate remains",
            ],
        ),
        (
            "defer-promoted-write-to-first-consumer-dispatch",
            &[
                "materializes the same writes twice",
                "bind seconds vs materialize seconds",
                "IDENTIFYING the existing implicit fence",
            ],
        ),
        (
            "rollback-snapshot-covers-all-mutable-state",
            &[
                "append-only write contract",
                "silently skipped",
                "NEVER returned",
            ],
        ),
        (
            "scratch-bank-isolates-one-shot-operands-from-hot-cache",
            &["42 -> 0", "2.12-2.19x", "STRUCTURALLY untouchable"],
        ),
        (
            "sort-consumer-routes-by-physical-slot",
            &[
                "+26.6%",
                "64 routed rows",
                "an API CONTRACT with the gather kernel",
            ],
        ),
        (
            "row-addressable-operand-index-not-full-tensor-map",
            &[
                "silent weight corruption",
                "UNGUARDED data_bytes // num_experts",
                "the merkle_root lesson",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-94 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-94 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch94_pseudocode_parses() {
    // G1 for Batch 94: syn-parse gate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "cross-run-profiles-key-on-content-not-stat-triplets",
        "fill-gate-speculation-until-working-set-resident",
        "decayed-frequency-eviction-recency-tiebreak-route-protection",
        "defer-promoted-write-to-first-consumer-dispatch",
        "rollback-snapshot-covers-all-mutable-state",
        "scratch-bank-isolates-one-shot-operands-from-hot-cache",
        "sort-consumer-routes-by-physical-slot",
        "row-addressable-operand-index-not-full-tensor-map",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-94 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-94 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-94 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch97_rules_present() {
    // Batch 97 (antirez/ds4 DwarfStar Metal distill -- Research 117 /
    // Plan 063): 9 kernel_opt rules (N +1, E +2, C +2, H +1, M +1,
    // A +1, F +1). Provenance at pin 110afdd8: 8 DS4-original;
    // mask-block-classification re-provenanced to ggml-org/llama.cpp
    // @ 9ca265a at mine time (the falsification the provenance check
    // exists for).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "half-domain-exact-band-dequant-with-raw-bit-oracle",
        "tail-simdgroup-cull-barriers-preserved",
        "sibling-projections-one-kernel-shared-staged-tile",
        "expert-major-work-map-token-scatter-clamped",
        "proven-shape-static-sibling-identical-results-fallback",
        "barrier-after-async-mma-before-staged-tile-reuse",
        "bit-exact-dispatch-fusion-boundary-emulation",
        "virtual-thread-folding-preserves-reduction-tree",
        "mask-block-classification-skip-placement",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-97 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch97_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "half-domain-exact-band-dequant-with-raw-bit-oracle",
            &[
                "rounds EXACTLY ONCE",
                "ONLY a raw-bit exhaustive match promotes the table",
                "the per-element f32->f16 narrowing",
                "e.g. scale 2^-25 x code 4 -> f32 path 2^-23, half path 0 x 4 = 0",
            ],
        ),
        (
            "tail-simdgroup-cull-barriers-preserved",
            &[
                "Keep all threads in staging and at every threadgroup barrier",
                "threadgroup barriers are collective",
                "the same nr1 clamp the bounds logic computes",
            ],
        ),
        (
            "sibling-projections-one-kernel-shared-staged-tile",
            &[
                "TWO threadgroup barriers TOTAL instead of two per matrix",
                "BIT-IDENTITY to the unfused compose",
                "the GEMV-level twin",
            ],
        ),
        (
            "expert-major-work-map-token-scatter-clamped",
            &[
                "zero work must overwrite any stale prior value",
                "ORDER IS DELIBERATELY UNSPECIFIED",
                "a counting-sort by atomic",
            ],
        ),
        (
            "proven-shape-static-sibling-identical-results-fallback",
            &[
                "identical results instead of garbage",
                "BYTE-IDENTICAL to the dynamic implementation",
                "NEVER delete the unreachable defensive branch as dead code",
            ],
        ),
        (
            "barrier-after-async-mma-before-staged-tile-reuse",
            &[
                "the barrier after mm.run prevents the next K iteration from replacing staged tiles while the cooperative matmul still reads them",
                "ONE threadgroup barrier per k-step",
                "SILENT CORRUPTION, no trap",
            ],
        ),
        (
            "bit-exact-dispatch-fusion-boundary-emulation",
            &[
                "every former dispatch boundary was also a compiler-optimization fence",
                "volatile-qualified pointer",
                "8-lane TG8 simd_sum",
                "verified against full-vocabulary logits",
            ],
        ),
        (
            "virtual-thread-folding-preserves-reduction-tree",
            &[
                "virtual slices",
                "survives bit-exactly",
                "the redundancy buys the elimination of one dispatch",
            ],
        ),
        (
            "mask-block-classification-skip-placement",
            &[
                "fully masked",
                "all-zero",
                "partial final blocks default to compute",
                "kernel_flash_attn_ext_blk",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-97 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-97 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch97_pseudocode_parses() {
    // G1 for Batch 97: syn-parse gate (the ds4 DwarfStar Metal
    // distill -- the exact-band half dequant, the barrier-preserving
    // tail cull, the shared-staged-tile pair fusion, the atomic
    // scatter work map, the static sibling fallback, the async-MMA
    // barrier contract, the bit-exact fusion boundary emulation, the
    // virtual-thread fold, the mask-block classification).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "half-domain-exact-band-dequant-with-raw-bit-oracle",
        "tail-simdgroup-cull-barriers-preserved",
        "sibling-projections-one-kernel-shared-staged-tile",
        "expert-major-work-map-token-scatter-clamped",
        "proven-shape-static-sibling-identical-results-fallback",
        "barrier-after-async-mma-before-staged-tile-reuse",
        "bit-exact-dispatch-fusion-boundary-emulation",
        "virtual-thread-folding-preserves-reduction-tree",
        "mask-block-classification-skip-placement",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-97 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-97 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-97 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch98_rules_present() {
    // Batch 98 (antirez/ds4 dense.metal/moe.metal distill -- Research 118
    // / Plan 065): 10 kernel_opt rules (A +1, B +2, C +1, D +1, E +2,
    // H +1, L +1, M +1) -- the NEXT BATCH LEADS Batch 97 queued + 3
    // additional candidates. Same pin 110afdd8, quotes re-verified.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "grid-range-split-verbatim-impl-composition",
        "share-activation-stream-keep-projection-reductions",
        "tiny-rows-scalar-thread-per-row-downshift",
        "small-batch-matvec-gemm-bridge-rung",
        "moe-routed-down-sum-single-accumulate-pass",
        "fused-vector-site-transcendental-form-nonportable",
        "monomorphic-hot-caller-last-hit-memo",
        "per-row-scale-folded-into-rhs-staging-tile",
        "dequant-byte-pairs-as-aligned-u16-loads",
        "cross-simd-bitonic-alternating-bank-exchange",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-98 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn batch98_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "grid-range-split-verbatim-impl-composition",
            &[
                "BY-VALUE copy of the shared args struct",
                "tgs0 = ceil(width0 / rows_per_group)",
                "DEVICE-scope threadgroup barrier is mandatory",
                "The pattern composes: a 5-way variant",
            ],
        ),
        (
            "share-activation-stream-keep-projection-reductions",
            &[
                "the point is not to fuse two independent weight streams into one matmul",
                "BOUNDARY COMPLEMENT of same-input-gemv-weight-concat-at-upload",
                "share the activation WALK",
                "Separate accumulators are forced regardless by the per-matrix scales",
            ],
        ),
        (
            "tiny-rows-scalar-thread-per-row-downshift",
            &[
                "one thread per output row",
                "Parallelism DOWN-shift",
                "The threshold derives from the warp width",
                "should be bench-pinned before any promotion",
            ],
        ),
        (
            "small-batch-matvec-gemm-bridge-rung",
            &[
                "dequantizes its K-chunk stripe ONCE and dots it against ALL tokens",
                "the middle rung is NOT universally the winner",
                "never hardcode the comment's range as the regime",
                "a three-way ladder with explicit thresholds",
            ],
        ),
        (
            "moe-routed-down-sum-single-accumulate-pass",
            &[
                "ONE cross-lane reduction per output row",
                "NOT BIT-IDENTICAL",
                "OUTPUT-SUM composition",
                "serializing the k GEMVs inside one simdgroup",
            ],
        ),
        (
            "fused-vector-site-transcendental-form-nonportable",
            &[
                "per-call-site, per-vector-width, per-silicon contract",
                "produced NON-FINITE logits",
                "inline the validated form AT the fused sites",
                "the measured tanh-at-fused-sites failure is the contract",
            ],
        ),
        (
            "monomorphic-hot-caller-last-hit-memo",
            &[
                "LAST-HIT MEMO",
                "the caller is MONOMORPHIC across the hot loop",
                "rebuild-when-payload-tiny-and-lookup-cheap",
                "the memo is in front of, never instead of, the generic cache",
            ],
        ),
        (
            "per-row-scale-folded-into-rhs-staging-tile",
            &[
                "WHERE the F32-to-F16 rounding lands",
                "NOT bit-identical to scale-then-matmul",
                "the lossy-surface rule",
                "opposite sides of the matvec",
            ],
        ),
        (
            "dequant-byte-pairs-as-aligned-u16-loads",
            &[
                "load-width TWIN",
                "never assumed from a runtime pointer value",
                "the alignment invariant must be DERIVED from the struct layout",
                "the 16-lane Q8_0 block becomes 8 loads",
            ],
        ),
        (
            "cross-simd-bitonic-alternating-bank-exchange",
            &[
                "DOUBLE-BUFFERED into two banks selected by cross-stage PARITY",
                "stage-distance-based",
                "one barrier per cross stage replaces the write+read pair",
                "they govern different access patterns",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-98 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-98 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch98_pseudocode_parses() {
    // G1 for Batch 98: syn-parse gate (the ds4 dense.metal/moe.metal
    // distill -- the grid-range split composition, the shared-activation
    // non-merge boundary, the tiny-row scalar downshift, the small-batch
    // matvec bridge, the routed-sum single-pass accumulation, the fused-
    // site transcendental contract, the last-hit lookup memo, the staging-
    // boundary scale fold, the aligned pair loads, the alternating-bank
    // bitonic exchange).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "grid-range-split-verbatim-impl-composition",
        "share-activation-stream-keep-projection-reductions",
        "tiny-rows-scalar-thread-per-row-downshift",
        "small-batch-matvec-gemm-bridge-rung",
        "moe-routed-down-sum-single-accumulate-pass",
        "fused-vector-site-transcendental-form-nonportable",
        "monomorphic-hot-caller-last-hit-memo",
        "per-row-scale-folded-into-rhs-staging-tile",
        "dequant-byte-pairs-as-aligned-u16-loads",
        "cross-simd-bitonic-alternating-bank-exchange",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-98 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-98 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-98 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch100_rules_present() {
    // Batch 100 (Layr-Labs/qwen-3.8-mtp-challenge -- Research 120 / Plan 068):
    // 8 kernel_opt rules (A +3, D +3, E +2) -- the M5-Max Swift/Metal
    // inference arena distill, same team as antirez/ds4. Pin 0863b06a,
    // byte-identical to the verdict-time pin (tree static since 2026-08-23);
    // quotes re-verified byte-accurate by two parallel verification lanes.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "functional-cache-versioning-loses-donation-under-kernel-read",
        "fused-kv-write-single-writer-reads-newest-from-input",
        "in-place-write-through-input-emits-fence-consumer",
        "residency-set-wired-once-post-warm-persistent-footprint",
        "register-budget-head-split-gqa-reread-trade",
        "batch-composition-invariant-grid-early-return",
        "never-gqa-multiply-full-d-merge-smem",
        "container-rebind-over-retrace-for-churn-survival",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-100 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch100_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "functional-cache-versioning-loses-donation-under-kernel-read",
            &[
                "23 layers x {K,V} x 8 GiB",
                "pure write-only chains donate fine",
                "the failure needs >= 2 layers sharing a slab group",
                "dead on arrival",
                "STABLE buffer written IN PLACE by a kernel",
            ],
        ),
        (
            "fused-kv-write-single-writer-reads-newest-from-input",
            &[
                "exactly ONE threadgroup per byte is the writer",
                "knew/vnew INPUT operands",
                "outside every attendable window",
            ],
        ),
        (
            "in-place-write-through-input-emits-fence-consumer",
            &[
                "fence[0] = prev[0] + 1",
                "directly or transitively",
                "fewer than 8 elements land in the CONSTANT address space",
            ],
        ),
        (
            "residency-set-wired-once-post-warm-persistent-footprint",
            &[
                "capacity is ZERO until a wired limit is applied",
                "live post-warm footprint",
                "Never end or shrink the ticket",
            ],
        ),
        (
            "register-budget-head-split-gqa-reread-trade",
            &[
                "HPT*D/32 <= 32",
                "no head's summation order changes",
                "UNCATCHABLE process fatal",
                "vLLM RFC #15351",
            ],
        ),
        (
            "batch-composition-invariant-grid-early-return",
            &[
                "INCLUDING float summation order, is bit-identical regardless of batchmates",
                "RETURN EARLY on lo >= attend_len",
                "RECOMPUTES the partition count from the row's own attended length",
            ],
        ),
        (
            "never-gqa-multiply-full-d-merge-smem",
            &[
                "Never put a GQA multiplier on a full-D threadgroup-memory merge surface",
                "constant ~4 KB smem REGARDLESS of D and GQA",
                "GQA-INDEPENDENT smem",
                "the anti-pattern no production kernel uses",
            ],
        ),
        (
            "container-rebind-over-retrace-for-churn-survival",
            &[
                "no copy, no retrace",
                "clears the lane WHOLESALE",
                "never a permanent invalidation",
                "FOURTH mechanism",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-100 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-100 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch100_pseudocode_parses() {
    // G1 for Batch 100: syn-parse gate (the qwen-3.8-mtp-challenge
    // distill -- the donation-loss versioning trap, the fused single-writer
    // KV write, the fence-token ordering contract, the wire-once residency
    // ticket, the head-split register budget, the batch-composition-
    // invariant grid, the GQA-independent merge surface, and the
    // container-rebind churn-survival design).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "functional-cache-versioning-loses-donation-under-kernel-read",
        "fused-kv-write-single-writer-reads-newest-from-input",
        "in-place-write-through-input-emits-fence-consumer",
        "residency-set-wired-once-post-warm-persistent-footprint",
        "register-budget-head-split-gqa-reread-trade",
        "batch-composition-invariant-grid-early-return",
        "never-gqa-multiply-full-d-merge-smem",
        "container-rebind-over-retrace-for-churn-survival",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-100 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-100 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-100 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch102_rules_present() {
    // Batch 102 (FlashQuant sparse-dense fusion -- Research 123 / Plan 071):
    // 6 kernel_opt rules (A +2, C +2, H +1, N +1) distilled from the
    // outlier-aware W4A16 paper (arXiv 2608.15531). The headline was
    // REFUTED for direct adoption at walk time; the fusion mechanism is
    // the distilled core. Quotes verified byte-accurate by the
    // coordinator against the arXiv HTML v1 rendering.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "heterogeneous-unit-paths-shared-tile-fusion",
        "lpt-min-heap-disjoint-column-ownership",
        "tile-coo-sparse-tile-aligned-format",
        "bucketed-sparse-reader-column-partition",
        "bank-group-circular-element-reorder",
        "stream-k-persistent-cta-mac-segments",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-102 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch102_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "heterogeneous-unit-paths-shared-tile-fusion",
            &[
                "co-scheduled over a common tiled execution space",
                "tile correspondence",
                "the sparse path runs on CUDA cores",
                "we allocate additional warps to this task",
                "diminishes for very small batches, such as 1 and 2",
                "ADDITIVE-HETEROGENEOUS-PATHS member",
            ],
        ),
        (
            "lpt-min-heap-disjoint-column-ownership",
            &[
                "descending order of nonzero count",
                "assigned to the currently least-loaded bucket",
                "preserves conflict-free accumulation",
                "remains below 100 s for all evaluated matrices",
            ],
        ),
        (
            "tile-coo-sparse-tile-aligned-format",
            &[
                "represented using uint8_t",
                "repurposed to store original weight values",
                "alignment of outlier entries with GEMM tile boundaries",
                "assert it at encode",
            ],
        ),
        (
            "bucketed-sparse-reader-column-partition",
            &[
                "threads access the same column simultaneously",
                "N_bucket = warp_size/N_thread buckets",
                "under the conflict threshold by construction",
            ],
        ),
        (
            "bank-group-circular-element-reorder",
            &[
                "reducing inter-sub-warp bank conflicts",
                "reduces the number of conflicts from 4 to 1",
                "the circular start offset rotates per bucket",
                "bench before citing",
            ],
        ),
        (
            "stream-k-persistent-cta-mac-segments",
            &[
                "aggregate MAC iteration space across persistent CTAs",
                "a subsequent global reduction is performed",
                "mandatory control arm",
                "occupancy-grid-cap-refuted-uncapped-default",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-102 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-102 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch102_pseudocode_parses() {
    // G1 for Batch 102: syn-parse gate (the FlashQuant distill -- the
    // heterogeneous-unit fusion, the LPT disjoint column ownership, the
    // Tile-COO format, the bucketed sparse-reader partition, the
    // bank-group circular reorder, and the Stream-K MAC-space segments).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "heterogeneous-unit-paths-shared-tile-fusion",
        "lpt-min-heap-disjoint-column-ownership",
        "tile-coo-sparse-tile-aligned-format",
        "bucketed-sparse-reader-column-partition",
        "bank-group-circular-element-reorder",
        "stream-k-persistent-cta-mac-segments",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-102 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-102 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-102 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch103_rules_present() {
    // Batch 103 (MTPLX PR #391 + MLX NAX combined host-orchestration
    // batch -- Research 124 / Plan 072): 4 kernel_opt rules (D +4),
    // distilled from youssofal/MTPLX PR #391 (re-pinned ef96313 ->
    // a5e38bb at mine time; all anchors re-verified against the new head
    // and the rewritten PR body) + the apple/mlx NAX extraction
    // (f99e916). Host-orchestration batch: no prefill-throughput claim.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "shape-class-keyed-capability",
        "wavefront-overlap-pair-ceiling",
        "request-shape-decline-mirror-completeness",
        "backing-provenance-causal-shadowed-extent-widen",
        "dispatcher-eligibility-predicate-lags-kernel-capability",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-103 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch103_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "shape-class-keyed-capability",
            &[
                "fails loudly at install",
                "imported rather than re-spelled so the routing predicate and the kernel cannot drift apart",
                "a 4,096-token probe exercises the same grid a 17,408-token cycle does",
                "if 32 * gqa_factor * int(q_len) > 1024",
                "NOT re-confirmable",
                "never by mask kind alone",
                "3 * 12 = 36 > 32",
                "warmup ladder: verify before any wide chunk",
                "silently the control in the other",
                "never a wrong answer",
                "not evidence of engagement",
            ],
        ),
        (
            "wavefront-overlap-pair-ceiling",
            &[
                "0.4 % against a 15 % gate; tile ceiling 19.2 %",
                "the ceiling is too low to fund",
                "cross-stream fences cost as much as the overlap gains",
                "exactly S-1 overlappable producer/consumer pairs",
            ],
        ),
        (
            "request-shape-decline-mirror-completeness",
            &[
                "2026-09-02, HTTP 500 on every HumanEval prompt",
                "the two questions are different, and :func:`context_decline` asks",
                "a measurement that looks like a result",
                "a single total would not have shown that, so the sites are named",
                "cannot quietly measure the default",
                "One key set to ``0`` on its own raises at model install",
            ],
        ),
        (
            "backing-provenance-causal-shadowed-extent-widen",
            &[
                "if ((st[0] < 0) || (st[1] <= 0) || (st[2] <= 0) || (st[1] % st[2] != 0)) { return false; }",
                "if (row0 + rows > pitch)",
                "The causal offset describes the true diagonal even if kL is widened below.",
                "has_backing_rows(k, kLp) && has_backing_rows(v, kLp)",
                "if (split_d && do_causal_ && !mask.has_value() && (kL % bk))",
            ],
        ),
        (
            "dispatcher-eligibility-predicate-lags-kernel-capability",
            &[
                "Make head-dim-256 prefill with array mask run fused NAX kernel",
                "dead conservatism",
                "function_constant(300)",
                "POSITIVE ENGAGEMENT",
                "split-decode",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-103 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-103 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch103_pseudocode_parses() {
    // G1 for Batch 103: syn-parse gate (the MTPLX shape-keyed capability,
    // the wavefront overlap-pair ceiling, the request-shape decline
    // mirror, and the MLX backing-provenance extent widen).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "shape-class-keyed-capability",
        "wavefront-overlap-pair-ceiling",
        "request-shape-decline-mirror-completeness",
        "backing-provenance-causal-shadowed-extent-widen",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-103 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-103 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-103 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch104_rules_present() {
    // Batch 104 (ggml-org/llama.cpp fa-vec swept launch-table distill --
    // Research 126 / Plan 074): 2 kernel_opt rules (K +2), distilled from
    // the fa-vec system -- infra efeda76b9, table pin 8963a9bdc (the
    // M3 Max rows), harness read at d230ddd76 (byte-identical pin->HEAD).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "swept-shape-bucket-launch-tables",
        "no-harm-bucket-representative",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-104 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch104_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "swept-shape-bucket-launch-tables",
            &[
                "tightly packed for memcmp",
                "short KV: attention is a small slice of the step, left to baseline",
                "fall back to baseline (Q=1) if the tuned config exceeds threadgroup memory",
                "single source shared by the offline tuner and test-backend-ops",
                "small dk crosses late, large dk wins even at short KV",
                "never a pass/fail test",
                "no measurement there can reach the table",
            ],
        ),
        (
            "no-harm-bucket-representative",
            &[
                "no slower than the baseline config at every point that bucket covers",
                "one of a bucket's first Q values carries the worst padding it can ever see",
                "a config picked on the aligned ne01=8,16 says nothing about ne01=9",
                "a floor on the evidence, not a proof",
                "TUNE_TAU = 0.05",
                "Nothing measured, nothing proven",
                "a bucket with no surviving sample admits baseline only",
                "instead of letting the ne11-collapsed domain default ride in unmeasured",
                "load-bearing for the timings, not just for numerics",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-104 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-104 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch104_pseudocode_parses() {
    // G1 for Batch 104: syn-parse gate (the swept-shape-bucket launch
    // table + the no-harm bucket representative).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "swept-shape-bucket-launch-tables",
        "no-harm-bucket-representative",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-104 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-104 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-104 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch105_rules_present() {
    // Batch 105 (uzu + lily combined Metal-engine distill -- Research 127
    // + 128 / Plan 075): 11 kernel_opt rules (A +1, C +1, D +3, E +1,
    // F +1, H +3, M +1). Sources: trymirai/uzu @ 115609b and
    // perplexityai/pplx-garden lily/ @ 1ed972ed; both inspection lanes
    // re-verified every quote at the pins; the cross-source dedup
    // mandate executed per rule (the uzu/lily pairs cross-referenced
    // in-rule, not merged).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "concurrent-dispatch-dependency-level-barriers",
        "gpu-resident-moe-routing-no-host-sync",
        "indirect-device-dispatch-tile-map-moe",
        "pipelined-token-slot-ping-pong-submit-ahead",
        "split-k-two-pass-attention-decode-long-context",
        "cooperative-tensor-mma-capability-route",
        "rht-epilogue-fusion-contract",
        "lane-sequenced-deterministic-bucket-scatter",
        "a8-activation-route-stability-guard",
        "target-tile-split-k-deterministic-two-dispatch",
        "hazard-tracked-aliasing-scratch-allocator",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-105 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn batch105_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "concurrent-dispatch-dependency-level-barriers",
            &[
                "Metal drops the serial encoder's implicit ordering",
                "independent kernels within a level overlap",
                "Serial encoders ignore the barrier",
                "Five dispatches in five dependency levels",
                "caller documentation",
            ],
        ),
        (
            "gpu-resident-moe-routing-no-host-sync",
            &[
                "the CPU never inspects routing intermediates",
                "ceil(S/T) + min(E, S)",
                "drop rows SILENTLY",
                "zero host readback",
                "prefill-only",
            ],
        ),
        (
            "indirect-device-dispatch-tile-map-moe",
            &[
                "never round-trips to the host",
                "Only the RAGGED axis is indirect",
                "Over-provisioned threadgroups early-return",
                "one dead TG row",
                "GROUPS(INDIRECT)",
            ],
        ),
        (
            "pipelined-token-slot-ping-pong-submit-ahead",
            &[
                "submit-ahead",
                "U32[2] ping-pong slot pair",
                "the NEXT step's INPUT slot",
                "OUT-OF-BAND",
                "does not gate the next submission",
                "advances at ENCODE time",
            ],
        ),
        (
            "split-k-two-pass-attention-decode-long-context",
            &[
                "fixed-block TWO-PASS split",
                "per-block partial attention state (m, l, acc)",
                "The crossover is exact, not tuned",
                "exact model shape",
                "GLOBAL max/sum",
                "sdpa_vector_2pass_1",
            ],
        ),
        (
            "cooperative-tensor-mma-capability-route",
            &[
                "FAIL-CLOSED",
                "DEFAULT FALSE",
                "relaxed multiply is slightly faster",
                "strict-layout override",
                "diverges from FP32 across layers",
                "alignment decides",
            ],
        ),
        (
            "rht-epilogue-fusion-contract",
            &[
                "only when split_k == 1",
                "Bias is DEMOTED",
                "transforms STRIPPED",
                "folds INTO the quantize kernel",
                "the ordering, not the arithmetic, is the correctness content",
            ],
        ),
        (
            "lane-sequenced-deterministic-bucket-scatter",
            &[
                "no device atomics",
                "32 ordered steps per tile",
                "INPUT ORDER, not from sorting",
                "tok2row = -1",
                "OFF by default",
            ],
        ),
        (
            "a8-activation-route-stability-guard",
            &[
                "RE-RUN THE FULL DISPATCH",
                "does A8 still win",
                "min(weight_group, 128)",
            ],
        ),
        (
            "target-tile-split-k-deterministic-two-dispatch",
            &[
                "TWO kernels",
                "output transforms STRIPPED",
                "quant-group alignment LADDER",
                "k % (split_k * align) == 0",
                "bit-deterministic",
            ],
        ),
        (
            "hazard-tracked-aliasing-scratch-allocator",
            &[
                "RAW/WAW/WAR",
                "the barrier consumes the history",
                "Barriers are COMPUTED, not authored",
                "Aliasing legality is DERIVED",
                "can_alias_before",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-105 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-105 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch105_pseudocode_parses() {
    // G1 for Batch 105: syn-parse gate (the combined uzu + lily rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "concurrent-dispatch-dependency-level-barriers",
        "gpu-resident-moe-routing-no-host-sync",
        "indirect-device-dispatch-tile-map-moe",
        "pipelined-token-slot-ping-pong-submit-ahead",
        "split-k-two-pass-attention-decode-long-context",
        "cooperative-tensor-mma-capability-route",
        "rht-epilogue-fusion-contract",
        "lane-sequenced-deterministic-bucket-scatter",
        "a8-activation-route-stability-guard",
        "target-tile-split-k-deterministic-two-dispatch",
        "hazard-tracked-aliasing-scratch-allocator",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-105 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-105 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-105 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch106_rules_present() {
    // Batch 106 (MTPLX upstream residue distill -- Research 129 / Plan
    // 076): 5 kernel_opt rules (D +5). Source: youssofal/MTPLX upstream
    // main @ dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d (v2.11.1,
    // Apache-2.0); the upstream mistakes/ post-mortem corpus (16 files,
    // filename IS the rule) + docs/plans + specs + concurrency + the
    // targeted kernel diff since the Batch-103 mined base 8bc4d88. The
    // B+ lead is CONFIRMED with BOTH instances in-source (the sentinel
    // fingerprint post-mortem + the dd4031a8 fix commit itself).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "prefix-reuse-key-must-mirror-the-restore-predicate",
        "request-arrival-work-asks-reuse-state-first",
        "serving-raises-only-behind-strict-flag",
        "derived-key-single-constructor-parity-test",
        "vendor-fused-op-instantiation-check",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-106 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch106_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "prefix-reuse-key-must-mirror-the-restore-predicate",
            &[
                "the fingerprint only has to carry what changes the KV",
                "ask the bank the question the restore asks",
                "41,901 tokens cold (54 s)",
                "an estimator stricter than restore strands restorable state",
            ],
        ),
        (
            "request-arrival-work-asks-reuse-state-first",
            &[
                "the gather was pure waste and its wait was pure dead time",
                "ASK THE REUSE STATE FIRST",
                "NEVER AWAIT UNCONSUMED WORK",
                "no probe = no preempt = legacy behaviour",
            ],
        ),
        (
            "serving-raises-only-behind-strict-flag",
            &[
                "finish_reason 'error'",
                "ONE warning per verdict class per process",
                "measurement = raise; serving = count + warn",
                "grep it for raise on paths a request can reach",
            ],
        ),
        (
            "derived-key-single-constructor-parity-test",
            &[
                "MUST derive its key through this function",
                "A PARITY TEST THAT FAILS ON DRIFT",
                "still the user's standing pin",
                "the DERIVATION TOPOLOGY",
            ],
        ),
        (
            "vendor-fused-op-instantiation-check",
            &[
                "read the symbol table of each shipped mlx.metallib",
                "the MLA latent is 512 wide",
                "a benchmark of the fallback while the team believes it is measuring fusion",
                "The fallback is a COMPARATOR",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-106 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-106 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch106_pseudocode_parses() {
    // G1 for Batch 106: syn-parse gate (the MTPLX upstream residue rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "prefix-reuse-key-must-mirror-the-restore-predicate",
        "request-arrival-work-asks-reuse-state-first",
        "serving-raises-only-behind-strict-flag",
        "derived-key-single-constructor-parity-test",
        "vendor-fused-op-instantiation-check",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-106 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-106 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-106 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch107_rules_present() {
    // Batch 107 (uzu + lily defers batch -- Research 130 / Plan 077):
    // 8 kernel_opt rules. Sources: trymirai/uzu @ 115609b6 (Release
    // 0.5.23, MIT) + perplexityai/pplx-garden lily @ 1ed972ed
    // (Apache-2.0). The B105 deferred queue consumed: 12 candidates
    // re-verified by two lanes at the pins, 8 landed, 3 dropped
    // (flat-topology-fast-path = B23-family batch-trie specialization;
    // RIF-sentinel = born-dead template arms, sole instantiation is
    // RIF=1/SERIAL=0; weight-hygiene = B63/B31 external instances),
    // 1 collapsed to a rider (cross-level-fused-residual-norm -> B63).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "sparse-demand-paged-kv-cache",
        "gumbel-argmax-filtered-sampling-descent",
        "packed-total-order-multi-field-argmax",
        "integer-mma-span-epilogue-dequant",
        "single-threadgroup-fused-count-scan",
        "radix-select-ordered-key-topk",
        "prefill-chunk-command-buffer-capacity-coupled",
        "tensor-op-strided-device-operand-binding",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-107 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch107_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "sparse-demand-paged-kv-cache",
            &[
                "mapped_pages.gaps(pages)",
                "max context free until it's actually used",
                "MTLHeapType::Sparse is for sparse TEXTURES",
                "the mapped high-water, not the extent",
            ],
        ),
        (
            "gumbel-argmax-filtered-sampling-descent",
            &[
                "u == 1 gives +inf and wins every argmax",
                "FIXED Plackett-Luce order",
                "[2^-24, 1-2^-24]",
                "MAX_ITERS = 64",
            ],
        ),
        (
            "packed-total-order-multi-field-argmax",
            &[
                "equal scores prefer smaller indices",
                "(index_mask - index)",
                "0xFFFFFFFF sentinel",
            ],
        ),
        (
            "integer-mma-span-epilogue-dequant",
            &[
                "SPAN = min(left group, right group)",
                "sum_group_size = min(weight_group, 128)",
                "(k & 1) * THREADGROUP_BLOCK_N",
                "k / split_k / group_size >= 6",
            ],
        ),
        (
            "single-threadgroup-fused-count-scan",
            &[
                "SINGLE threadgroup",
                "simd_prefix_exclusive_sum",
                "counts_shared[e0 + e]",
                "largest tested shape E = 128",
            ],
        ),
        (
            "radix-select-ordered-key-topk",
            &[
                "RESET BY ITS LAST USER",
                "128K-vocab index costs 5 passes",
                "NO explicit size router",
            ],
        ),
        (
            "prefill-chunk-command-buffer-capacity-coupled",
            &[
                "capped at PREFILL_CHUNK rows",
                "chunks are commit_wait-synchronized",
                "no mid-chunk readback splits the pass",
                "max(old, new), not the sum",
            ],
        ),
        (
            "tensor-op-strided-device-operand-binding",
            &[
                "NO k-loop at all",
                "threadgroup memory holds only softmax state",
                "bounds-check ragged edge tiles",
                "capability FIRST, binding second",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-107 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-107 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch107_pseudocode_parses() {
    // G1 for Batch 107: syn-parse gate (the uzu + lily defers rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "sparse-demand-paged-kv-cache",
        "gumbel-argmax-filtered-sampling-descent",
        "packed-total-order-multi-field-argmax",
        "integer-mma-span-epilogue-dequant",
        "single-threadgroup-fused-count-scan",
        "radix-select-ordered-key-topk",
        "prefill-chunk-command-buffer-capacity-coupled",
        "tensor-op-strided-device-operand-binding",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-107 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-107 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-107 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch108_rules_present() {
    // Batch 108 (riir-ai M3 campaign measured-GOAT distill -- Plan 078):
    // 4 kernel_opt rules. Source: riir-ai @ 59a172623 (committed state
    // only) -- the first in-source batch since Batch 63; the M3
    // prefill/decode campaign mined (Issues 768/771/779/831/843; the
    // numbers belong to Benches 774/790/804/831 and are cited as
    // records, never re-measured). Two verification lanes read the pin
    // byte-accurately pre-plan; the coordinator spot-verified the
    // primary quotes at landing (all VERIFIED). Riders: B01 (CMMA
    // staged-GEMM in-stack instance), B06 (capped-occupancy regime
    // complement), B57 (staged-epilogue exploitation half).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "smem-envelope-gate-kernel-variant",
        "multi-simdgroup-shared-staged-tile",
        "independent-accumulator-chains-hide-load-latency-at-capped-occupancy",
        "byte-cut-loses-when-plateau-is-issue-bound",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-108 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch108_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "smem-envelope-gate-kernel-variant",
            &[
                "32*(head_dim + 4) > head_dim * 33",
                "overflows into p_smem",
                "row-0 score writes",
                "DETERMINISTIC in head_dim",
                "verify_kf4_for",
                "just don't read",
            ],
        ),
        (
            "multi-simdgroup-shared-staged-tile",
            &[
                "one shared tile would race",
                "private 8x64 regions",
                "TWO barriers per K window",
                "26.6 KB of the 32 KB",
                "m >= 128 && p >= 64",
                "fresh +0.0 could flip a -0.0",
            ],
        ),
        (
            "independent-accumulator-chains-hide-load-latency-at-capped-occupancy",
            &[
                "~1.8x at 4096 ctx",
                "valid_n",
                "tolerance band, never bit-identity",
                "median ratios 1.01-1.06",
                "loads-in-flight",
            ],
        ),
        (
            "byte-cut-loses-when-plateau-is-issue-bound",
            &[
                "1.44-1.60x SLOWER",
                "~114 GB/s",
                "~400 GB/s peak",
                "-3.4%",
                "BINDING AXIS is the tell",
                "65536-pattern",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-108 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-108 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch108_pseudocode_parses() {
    // G1 for Batch 108: syn-parse gate (the riir-ai M3 campaign rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "smem-envelope-gate-kernel-variant",
        "multi-simdgroup-shared-staged-tile",
        "independent-accumulator-chains-hide-load-latency-at-capped-occupancy",
        "byte-cut-loses-when-plateau-is-issue-bound",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-108 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-108 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-108 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch110_rules_present() {
    // Batch 110 (vLLM libtorch_stable distill -- Research 133 / Plan
    // 081): 11 kernel_opt rules, the VERIFIED A-/B+ tier of a
    // 25-candidate pool. Source: vllm-project/vllm csrc/libtorch_stable
    // @ 8277c42e4 (Apache-2.0, never mined). Two inspection lanes +
    // coordinator byte-verification of 13 lead quotes at the pin. The
    // B-/C+ remainder tier is Batch 111; the upstream hazard ledger
    // (H1 tie-buffer overflow, H2 dispatch-no-default, H3 router
    // divergence) lives in Research 133.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "spin-barrier-needs-co-residency-capped-launch",
        "captured-graph-init-unconditional-and-stream-ordered",
        "ballot-matrix-tie-rank",
        "m-group-shared-weight-scan-register-relief",
        "softmax-denominator-cancels-through-scale-invariant-norm",
        "batch-invariance-locks-reduction-geometry",
        "rope-pair-partner-via-cross-lane-shuffle",
        "row-parallelism-ladder-split-vs-pack",
        "persistent-kernel-length-bucketed-mechanism-ladder",
        "router-math-stays-fp32",
        "degenerate-selection-falls-back-to-identity-uniform",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-110 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn batch110_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "spin-barrier-needs-co-residency-capped-launch",
            &[
                "overestimating it deadlocks the cooperative barrier",
                "most deadlock-prone case",
                "never dropping below one full group",
                "cap only when co-residency is load-bearing",
                "no co-residency precondition",
                "would wait on the inter-CTA barrier for peers",
            ],
        ),
        (
            "captured-graph-init-unconditional-and-stream-ordered",
            &[
                "the node set is frozen at capture, the data is not",
                "always true under FULL_DECODE_ONLY",
                "sub-microsecond",
                "the zero is globally visible before any CTA runs",
                "NOT repaired by re-execution",
            ],
        ),
        (
            "ballot-matrix-tie-rank",
            &[
                "1024 comparisons in one instruction per warp",
                "score desc, index asc",
                "kMaxTies = 1024",
                "4.0 at the source's 4096 bins",
                "H1 hazard",
            ],
        ),
        (
            "m-group-shared-weight-scan-register-relief",
            &[
                "128 regs",
                "weight traffic stays 1x",
                "1.26-1.66x",
                "0.91x",
                "re-measure, never trust one sweep",
                "NOTHING is re-read from DRAM",
            ],
        ),
        (
            "softmax-denominator-cancels-through-scale-invariant-norm",
            &[
                "= acc * rsqrt(mean(acc^2) + eps * s^2)",
                "ZERO divides",
                "eps * s^2",
                "never bit-identity",
                "attention probability weights need the real s",
            ],
        ),
        (
            "batch-invariance-locks-reduction-geometry",
            &[
                "one expression serves both masters",
                "an explicit precedence term",
                "Do not invert token_scale for exact match with FBGemm",
                "two-regime split by CONTRACT",
            ],
        ),
        (
            "rope-pair-partner-via-cross-lane-shuffle",
            &[
                "XOR with that distance",
                "half % elems_per_lane == 0",
                "NOT WGSL/Vulkan",
                "the sign fold",
                "a convention, not a one-off",
            ],
        ),
        (
            "row-parallelism-ladder-split-vs-pack",
            &[
                "one 8-element chunk per lane",
                "kDecodeRowSplitMaxTokens",
                "grid-stride loop instead of a huge launch",
                "SMs*8",
                "no cross-lane reductions inside the row",
            ],
        ),
        (
            "persistent-kernel-length-bucketed-mechanism-ladder",
            &[
                "8192/2048 = 4 elements/bin avg",
                "no static __shared__",
                "kill occupancy for the decode/trivial paths",
                "a launch per bucket",
                "H1 hazard",
            ],
        ),
        (
            "router-math-stays-fp32",
            &[
                "weight is always fp32",
                "PER-ROLE, not per-model",
                "M <= 32",
                "0.5f * tanhf(0.5f * x) + 0.5f",
                "routes differently BY TIER",
            ],
        ),
        (
            "degenerate-selection-falls-back-to-identity-uniform",
            &[
                "always pick expert 0 for every top-k slot",
                "pick [0,1,2,...,k-1]",
                "1.0f / topk_i32",
                "every LSE to -inf",
                "clamp first, tie-break second",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-110 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-110 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch110_pseudocode_parses() {
    // G1 for Batch 110: syn-parse gate (the vLLM libtorch_stable rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "spin-barrier-needs-co-residency-capped-launch",
        "captured-graph-init-unconditional-and-stream-ordered",
        "ballot-matrix-tie-rank",
        "m-group-shared-weight-scan-register-relief",
        "softmax-denominator-cancels-through-scale-invariant-norm",
        "batch-invariance-locks-reduction-geometry",
        "rope-pair-partner-via-cross-lane-shuffle",
        "row-parallelism-ladder-split-vs-pack",
        "persistent-kernel-length-bucketed-mechanism-ladder",
        "router-math-stays-fp32",
        "degenerate-selection-falls-back-to-identity-uniform",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-110 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-110 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-110 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch110_spin_barrier_launch_contract() {
    // T5 for Batch 110 (Plan 081, the GOAT lead): the co-residency
    // launch arithmetic of
    // spin-barrier-needs-co-residency-capped-launch pinned as a pure
    // function. Correctness-bearing: a spin barrier under
    // oversubscription is a hang, not a slowdown. The uncapped arm is
    // load-bearing too (the barrier-unreachable regime keeps the B58
    // uncapped default) -- assert BOTH directions.
    fn launch_plan(
        rows: usize,
        ctas_per_group: usize,
        num_sms: usize,
        occupancy: usize,
        needs_cooperative: bool,
    ) -> (usize, bool) {
        let hw_cap = num_sms * occupancy;
        if !needs_cooperative {
            return (rows * ctas_per_group, false); // uncapped, no barrier
        }
        let headroom = if occupancy > 1 { num_sms } else { 1 };
        let mut resident = hw_cap;
        if resident >= headroom + ctas_per_group {
            resident -= headroom;
        }
        let groups = (resident / ctas_per_group).min(rows).max(1);
        let total = groups * ctas_per_group;
        (total, total > hw_cap) // (total_ctas, fallback)
    }
    // Barrier reachable, occupancy > 1: headroom = one CTA per SM.
    // hw_cap=16, headroom=8, resident=8, groups=2, total=8 (was 16).
    let (total, fallback) = launch_plan(1024, 4, 8, 2, true);
    assert_eq!(total, 8);
    assert!(!fallback);
    // Occupancy == 1: headroom collapses to ONE CTA (the most
    // deadlock-prone case -- any straggler on the only slot hangs).
    // hw_cap=8, headroom=1, resident=7, groups=1, total=4.
    let (total_occ1, _) = launch_plan(1024, 4, 8, 1, true);
    assert_eq!(total_occ1, 4);
    // The cooperative form would not fit -> the non-barrier fallback
    // fires instead of deadlocking. hw_cap=4; 4 < 1+16 so resident
    // stays 4; groups = max(0,1) = 1; total=16 > 4 -> fallback.
    let (_, fallback_tight) = launch_plan(64, 16, 4, 1, true);
    assert!(fallback_tight);
    // Barrier unreachable: UNCAPPED (the B58 perf-side default
    // preserved -- capping here would be a measured 1-2% loss).
    let (total_uncapped, fallback_uncapped) = launch_plan(1024, 4, 8, 2, false);
    assert_eq!(total_uncapped, 4096);
    assert!(!fallback_uncapped);
    // Never below one full group's worth (the groups.max(1) floor):
    // hw_cap=2, ctas_per_group=8 -> groups floors to 1, total=8, and
    // the fit check then routes to the fallback.
    let (total_floored, fallback_floored) = launch_plan(1, 8, 2, 1, true);
    assert_eq!(total_floored, 8);
    assert!(fallback_floored);
    // Non-vacuity: the headroom subtract actually fires (24, not 32).
    let (total_subtract, _) = launch_plan(1024, 1, 8, 4, true);
    assert_eq!(total_subtract, 24);
}

#[test]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]
fn batch97_mxfp4_half_band_exhaustive_oracle() {
    // T6 for Batch 97 (Plan 063, MANDATORY for the GOAT lead): the
    // CPU twin of ds4's kernel_test_mxfp4_down_half_lut (moe.metal
    // L2505-2511). All 256 x 16 (E8M0 exponent byte, E2M1 code)
    // pairs; the legacy arm is f32-multiply-then-narrow (one RNE
    // rounding); the candidate arm rounds EACH operand to half first
    // (the f16 multiply: exact product of the rounded operands,
    // rounded once more -- exact in f32 because two half operands
    // carry <= 22 significant bits). IN the proven band [103, 142]
    // both arms must be BIT-IDENTICAL for every code (the band proof,
    // 640 pairs); OUTSIDE it they DIVERGE (the non-vacuity half: the
    // band is necessary, not decorative). The 0xff row stays
    // arithmetic per the source policy (NaN payload canonicalization
    // unchanged); the dead canonical LUT row agrees with it.
    fn f32_to_f16_bits_rne(f: f32) -> u16 {
        let b = f.to_bits();
        let sign = ((b >> 16) & 0x8000) as u16;
        let exp = i32::from(((b >> 23) & 0xff) as u8);
        let man = b & 0x007f_ffff;
        if exp == 0xff {
            // Inf / NaN (NaN canonicalizes quiet, matching a narrow)
            return if man == 0 {
                sign | 0x7c00
            } else {
                sign | 0x7e00
            };
        }
        if exp == 0 {
            // f32 subnormal: far below the half subnormal grid -> 0
            return sign;
        }
        let ue = exp - 127; // unbiased exponent
        if ue > 15 {
            return sign | 0x7c00; // overflow -> inf
        }
        if ue >= -14 {
            // normal half: the top 10 STORED mantissa bits (the implicit
            // bit is NOT stored -- including it would overlap the exponent
            // field), RNE on the remainder
            let m = (man >> 13) as u16;
            let rem = man & 0x1fff;
            let mut bits = sign | (((ue + 15) as u16) << 10) | m;
            if rem > 0x1000 || (rem == 0x1000 && (m & 1) == 1) {
                bits += 1; // may carry into the exponent: correct RNE
            }
            return bits;
        }
        // half subnormal / zero: the implicit bit IS part of the value here
        // (value = full * 2^(ue-23); ulp = 2^-24)
        let full = man | 0x0080_0000;
        let sh = (-(ue + 1)) as u32;
        if sh >= 27 {
            return sign; // rounds to zero
        }
        let m = (full >> sh) as u16;
        let rem = full & ((1u32 << sh) - 1);
        let half_ulp = 1u32 << (sh - 1);
        let mut bits = sign | m;
        if rem > half_ulp || (rem == half_ulp && (m & 1) == 1) {
            bits += 1; // 0x3ff + 1 = 0x400 = min normal: correct carry
        }
        bits
    }

    fn half_val(h: u16) -> f32 {
        let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
        let exp = i32::from((h >> 10) & 0x1f);
        let man = i32::from(h & 0x03ff);
        let v = match exp {
            0 => (man as f32) * 2f32.powi(-24),
            31 => {
                if man == 0 {
                    f32::INFINITY
                } else {
                    f32::NAN
                }
            }
            e => (1.0 + man as f32 / 1024.0) * 2f32.powi(e - 15),
        };
        sign * v
    }

    fn scale_f32(e: u8) -> f32 {
        match e {
            0xff => f32::NAN,
            0 => 2f32.powi(-127),
            _ => 2f32.powi(i32::from(e) - 127),
        }
    }

    const CODES: [f32; 16] = [
        0.0, 0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0, //
        -0.0, -0.5, -1.0, -1.5, -2.0, -3.0, -4.0, -6.0,
    ];

    // legacy arm: f32 arithmetic, then ONE RNE narrow
    let legacy = |e: u8, q: usize| -> u16 { f32_to_f16_bits_rne(scale_f32(e) * CODES[q]) };
    // candidate arm: the f16 multiply -- both operands rounded to half
    // first, the exact product rounded once more
    let candidate = |e: u8, q: usize| -> u16 {
        let dh = f32_to_f16_bits_rne(scale_f32(e));
        let vh = f32_to_f16_bits_rne(CODES[q]);
        f32_to_f16_bits_rne(half_val(dh) * half_val(vh))
    };

    // 1. The band proof: every in-band pair must be bit-identical.
    for e in 103u8..=142 {
        for (q, &code) in CODES.iter().enumerate() {
            let l = legacy(e, q);
            let c = candidate(e, q);
            assert_eq!(
                c, l,
                "band pair (e={e}, q={q} code={code}) diverged: {c:#06x} vs {l:#06x}"
            );
        }
    }

    // 2. Non-vacuity: outside the band the arms DIVERGE.
    let mut n_mismatch = 0usize;
    for e in 0u8..=255 {
        if (103..=142).contains(&e) || e == 0xff {
            continue; // band + policy row excluded
        }
        for q in 0..16usize {
            if legacy(e, q) != candidate(e, q) {
                n_mismatch += 1;
            }
        }
    }
    assert!(
        n_mismatch > 0,
        "the band would be decorative if no out-of-band pair diverged"
    );

    // 3. The named failure modes at each band edge (underflow low,
    // saturation high).
    // e=102 (2^-25): half(scale) rounds to 0, f32 path keeps 2^-23
    // for code 4.0 (2 ulp of the half subnormal grid).
    assert_eq!(candidate(102, 6), 0x0000);
    assert_eq!(legacy(102, 6), 0x0002);
    // e=143 (2^16): half(scale) saturates to inf, f32 path 32768 exact.
    assert_eq!(candidate(143, 1), 0x7c00);
    assert_eq!(legacy(143, 1), 0x7800);

    // 4. Boundary rows inside the band: exact on both arms.
    // e=103 (2^-24) x 4.0 = 2^-22: exact half subnormal arithmetic.
    assert_eq!(legacy(103, 6), 0x0004);
    assert_eq!(candidate(103, 6), 0x0004);
    // e=142 (2^15) x 6.0 = 196608: overflows half to inf on BOTH arms
    // (the band guarantees equality, not absence of overflow).
    assert_eq!(legacy(142, 7), 0x7c00);
    assert_eq!(candidate(142, 7), 0x7c00);

    // 5. The 0xff policy row: arithmetic keeps NaN canonicalization;
    // the dead canonical LUT row agrees with it.
    for q in 0..16usize {
        let l = legacy(0xff, q);
        assert_eq!(
            l & 0x7e00,
            0x7e00,
            "legacy arm must yield NaN-class bits for the 0xff row (q={q})"
        );
        assert_eq!(candidate(0xff, q), l);
    }
}

#[test]
fn batch111_rules_present() {
    // Batch 111 (vLLM libtorch_stable remainder + rider sweep --
    // Research 133 / Plan 082): 11 kernel_opt rules (#12-#20, #22,
    // #25 of the verdict table), source re-cloned + re-pinned at
    // 8277c42e4 at mine time (origin had moved to 16328c7a minutes
    // before). Dispositions at mine time: #21 (noinline rare path)
    // and #23 (clamp-before-collective) KILLED as COVERAGE -- B110's
    // persistent-kernel-length-bucketed-mechanism-ladder protection
    // (b) and spin-barrier companion contract already carry them with
    // the same source citations; #24 (capture-stable grid shape)
    // demoted to a RIDER on device-m-runtime-mblock-graph-stable-gemm
    // (B92 owns the capture-stable core). +20 riders across 13 files
    // (the Research 133 rider inventory, all located with quotes).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "grid-dependency-wait-scoped-to-dependent-operands",
        "ue8m0-pow2-scale-as-biased-exponent-byte",
        "register-rotation-prefetch-loop",
        "shared-operand-groups-per-warp",
        "launch-heuristic-pinned-to-calibration-arch",
        "warp-aggregated-atomic-scatter",
        "refine-by-rescan-prefix-filter-vs-buffered-ties",
        "coarse-precision-first-exact-refine",
        "whole-row-register-resident-under-length-budget",
        "lane-owned-topk-output",
        "routing-by-precomputed-lookup",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-111 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn batch111_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson). The needles
    // were asserted by the generator at authoring time; this gate
    // pins them against corpus drift.
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "grid-dependency-wait-scoped-to-dependent-operands",
            &[
                "CONSERVATIVE TOP",
                "ONCE-FLAG",
                "skip the wait entirely",
                "must not fire before this kernel's stores are issued",
                "No-op when PDL is not enabled on the launch",
                "wait as late as the data dependency allows",
            ],
        ),
        (
            "ue8m0-pow2-scale-as-biased-exponent-byte",
            &[
                "floor absmax",
                "the CEILING is the",
                "exponent-field bump",
                "7 real + 1 pad",
                "EXACT power of two",
                "Issue-845 PoT kin",
            ],
        ),
        (
            "register-rotation-prefetch-loop",
            &[
                "register-only rung",
                "no shared memory, no cp.async",
                "has_next boolean",
                "Buffer rotation",
                "DRAM-bandwidth-bound",
                "the stall merely moves",
            ],
        ),
        (
            "shared-operand-groups-per-warp",
            &[
                "operand-affinity axis",
                "wait_group<1>",
                "first head's RoPE",
                "heads of one token share",
                "a front wait_group<0> is the ref shape",
                "ordered by consumer dependency",
            ],
        ),
        (
            "launch-heuristic-pinned-to-calibration-arch",
            &[
                "calibrated on",
                "until profiled",
                "CONSERVATIVE baseline",
                "sm_version == 90",
                "B104's ref_shape IS this rule's naive shape",
                "refuses to extrapolate at all",
            ],
        ),
        (
            "warp-aggregated-atomic-scatter",
            &[
                "ONE leader atomicAdd",
                "O(hits)",
                "O(warps)",
                "ANTI-INSTANCE",
                "lanemask_lt",
                "lanes land in lane order",
            ],
        ),
        (
            "refine-by-rescan-prefix-filter-vs-buffered-ties",
            &[
                "SILENTLY DROPS",
                "measure the bin size",
                "bit-prefix",
                "written smem tie_buffer entries",
                "the H1 hazard comment",
                "single-pass-vs-streaming third instance",
            ],
        ),
        (
            "coarse-precision-first-exact-refine",
            &[
                "LOSSY coarse key",
                "order-preserving",
                "from the ORIGINAL f32",
                "may only DISCRIMINATE bins, never rank within them",
                "per-type",
                "f16-rounding collision band",
            ],
        ),
        (
            "whole-row-register-resident-under-length-budget",
            &[
                "register budget",
                "load the row ONCE",
                "(idx, score) pairs",
                "the fallback is the correct arm",
                "16384",
                "if forcing residency spills registers",
            ],
        ),
        (
            "lane-owned-topk-output",
            &[
                "result k is parked on lane k",
                "LaneOwnedResourcesFit",
                "512..=1024 experts",
                "PER-LANE DIVERGENT follow-up",
                "Zero post-reduction gather",
                "an extra exchange the broadcast never paid",
            ],
        ),
        (
            "routing-by-precomputed-lookup",
            &[
                "deterministic function of token IDENTITY",
                "6 gather loads",
                "sentinel check",
                "never a silent miss",
                "a stale table is silently wrong",
                "recomputes only the gate WEIGHTS",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-111 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-111 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch111_pseudocode_parses() {
    // G1 for Batch 111: syn-parse gate (the vLLM remainder rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "grid-dependency-wait-scoped-to-dependent-operands",
        "ue8m0-pow2-scale-as-biased-exponent-byte",
        "register-rotation-prefetch-loop",
        "shared-operand-groups-per-warp",
        "launch-heuristic-pinned-to-calibration-arch",
        "warp-aggregated-atomic-scatter",
        "refine-by-rescan-prefix-filter-vs-buffered-ties",
        "coarse-precision-first-exact-refine",
        "whole-row-register-resident-under-length-budget",
        "lane-owned-topk-output",
        "routing-by-precomputed-lookup",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-111 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-111 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-111 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch111_ue8m0_byte_contract() {
    // GOAT lead gate for Batch 111: the UE8M0 biased-exponent byte
    // encoding of ue8m0-pow2-scale-as-biased-exponent-byte, pinned as
    // a pure function of (absmax, fp8_max). Contract halves (each
    // from the source's own arithmetic):
    //   (1) the 1e-4 FLOOR keeps log2 finite -- absmax 0 must encode
    //       to a real byte, never NaN/-inf (which would clamp to 0 =
    //       scale 2^-127 and corrupt every dequant);
    //   (2) the CEILING guarantees the scaled payload fits the fp8
    //       range: absmax * decode(byte) <= fp8_max (up to fp
    //       epsilon) for every input;
    //   (3) decode is an EXACT power of two (2^(byte-127)) -- assert
    //       bit-exact against an integer shift in the exact range;
    //   (4) clamp bounds: extreme inputs saturate into [0, 255];
    //   (5) monotonicity: byte is non-decreasing in absmax;
    //   (6) NON-VACUITY: a swept input range produces MANY distinct
    //       bytes (a constant byte would make the whole contract
    //       vacuous -- the planted-degenerate-input discipline).
    fn encode(absmax_raw: f32, fp8_max: f32) -> u8 {
        let absmax = absmax_raw.max(1e-4); // floor: log2 never sees 0
        let exponent = (absmax / fp8_max).log2().ceil();
        // the clamp makes the f32->u8 cast exact; the cast IS the encoding
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            (exponent + 127.0).clamp(0.0, 255.0) as u8
        }
    }
    fn decode(byte: u8) -> f32 {
        // u8 -> i32 is infallible; the subtraction is the unbias step
        2.0f32.powi(i32::from(byte) - 127) // exact power of two
    }

    const FP8_MAX: f32 = 448.0; // e4m3 max (the source's non-ROCm arm)

    // (6) non-vacuity first: sweep absmax over ~30 decades by
    // varying the EXPONENT FIELD directly (random mantissa per
    // exponent) and count distinct bytes. A bit-sweep that only
    // perturbs mantissa bits floors to 1e-4 and collapses to ONE
    // byte -- the vacuity guard exists to catch exactly that
    // fixture bug (it caught the first draft of this sweep).
    let mut distinct = std::collections::BTreeSet::new();
    let mut x: u32 = 0x1234_5678;
    for biased_exp in 90u32..=200u32 {
        // 2^(biased_exp - 127) spans ~2^-37 .. 2^73; bytes below the
        // 1e-4 floor collapse (by design) and bytes above the clamp
        // saturate at 255, so the informative band is ~105..=255.
        for _ in 0..64 {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            let bits = (biased_exp << 23) | (x & 0x007F_FFFF); // exp + random mantissa
            let absmax = f32::from_bits(bits);
            if absmax.is_finite() && absmax > 0.0 {
                distinct.insert(encode(absmax, FP8_MAX));
            }
        }
    }
    assert!(
        distinct.len() >= 64,
        "UE8M0 encoding collapsed to {} distinct bytes over the sweep -- vacuous",
        distinct.len()
    );

    // (1) the floor: absmax 0 encodes to the 1e-4 byte, never the
    // saturating 0 byte.
    let b_zero = encode(0.0, FP8_MAX);
    assert_eq!(b_zero, encode(1e-4, FP8_MAX), "floor must absorb 0");
    assert_ne!(b_zero, 0, "floor failed: absmax 0 saturated to byte 0");

    // (2) the ceiling guarantee over the whole sweep incl. the floor
    // boundary and extremes: absmax * scale <= fp8_max (epsilon slack
    // for the f32 division).
    let mut probe: Vec<f32> = vec![0.0, 1e-4, 1e-3, 0.5, 1.0, 448.0, 449.0, 1e6, 1e30, f32::MAX];
    let mut lcg: u32 = 0x1234_5678;
    for _ in 0..2048 {
        lcg = lcg.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let bits = lcg >> 2;
        let v = f32::from_bits(bits) * 1.0e12;
        if v.is_finite() {
            probe.push(v.abs());
        }
    }
    for &absmax in &probe {
        if !absmax.is_finite() {
            continue;
        }
        let b = encode(absmax, FP8_MAX);
        let dequant_scale = decode(b); // 2^exponent: undoes the store
        let scaled = absmax / dequant_scale; // what lands in the fp8 payload
        assert!(
            scaled <= FP8_MAX * (1.0 + 1e-4),
            "ceiling violated: absmax {absmax:e} -> byte {b} -> scaled {scaled:e} > {FP8_MAX}"
        );
        // Tightness: the CEILING (not round) means one octave of
        // headroom at most -- the encoding never wastes more than
        // 1 bit of the payload's exponent range. Only meaningful
        // ABOVE the floor: a floored input (absmax < 1e-4, incl. 0)
        // legitimately lands far below the range top -- the floor
        // exists to give subnormal-scale inputs a usable exponent.
        if absmax >= 1e-4 {
            assert!(
                scaled > FP8_MAX * 0.25,
                "overshot: absmax {absmax:e} -> byte {b} -> scaled {scaled:e} << {FP8_MAX}"
            );
        }
    }

    // (3) decode is an exact power of two: bit-exact against the
    // IEEE biased-exponent construction for bytes whose exponent is
    // in the exact-f32 normal range (biased_exp = byte itself).
    for b in 111..=143u8 {
        let expect = f32::from_bits(u32::from(b) << 23); // 2^(b-127) exactly
        assert_eq!(
            decode(b).to_bits(),
            expect.to_bits(),
            "decode({b}) is not the exact power of two 2^{}",
            i32::from(b) - 127
        );
    }

    // (4) clamp bounds.
    assert_eq!(
        encode(f32::MAX, 1e-30),
        255,
        "huge absmax must saturate at 255"
    );
    assert_eq!(
        encode(0.0, f32::MAX),
        encode(1e-4, f32::MAX),
        "floor arm at extreme fp8_max"
    );

    // (5) monotonicity over an ordered probe.
    let mut prev = 0u8;
    let mut v = 1e-4f32;
    while v < 1e12 {
        let b = encode(v, FP8_MAX);
        assert!(b >= prev, "byte regressed at absmax {v:e}: {b} < {prev}");
        prev = b;
        v *= 1.7;
    }
}

#[test]
fn batch114_rules_present() {
    // Batch 114 (oMLX ANE-prefill + GDN blocked-sequential distill --
    // Research 138 / Plan 087): 2 kernel_opt rules (#1 the A- GOAT
    // lead, #2 the highest-value D rule), source jundot/omlx re-cloned
    // at e467261 for the batch (quote-verification lane re-read all ten
    // surfaces with exact line numbers). #3-#6 landed as riders on B84
    // (x2), B22, B95, B107, B79; #7 stayed lane-intel (769 f-split +
    // R4a warm rows).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "recurrence-prefill-exact-restructure-before-reformulation",
        "ane-bank-split-ladder",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-114 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch114_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "recurrence-prefill-exact-restructure-before-reformulation",
            &[
                "32x redundant traffic",
                "8x fewer threadgroups",
                "restructure-first",
                "measured a LOSER three independent times",
                "0.97x/0.98x",
                "source-reported",
            ],
        ),
        (
            "ane-bank-split-ladder",
            &[
                "0x20004",
                "DETERMINISM axis",
                "ASYNCHRONOUS",
                "70%",
                "48.3 GB",
                "fail OPEN",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-114 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-114 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch114_pseudocode_parses() {
    // G1 for Batch 114: syn-parse gate (the oMLX rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "recurrence-prefill-exact-restructure-before-reformulation",
        "ane-bank-split-ladder",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-114 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-114 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-114 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch113_rules_present() {
    // Batch 113 (llama.cpp v0.4.0 delta re-mine -- Research 137 /
    // Plan 086): 4 kernel_opt rules (#1/#2 the GOAT lead pair from the
    // Mamba-2 chunked SSD-MMA kernel, #3 the shader fastdiv cluster,
    // #4 the sparse-KV index-list idiom), source re-cloned at
    // 5266f24da for the batch (quote-verification lane re-read all five
    // surfaces; three attribution corrections: XOR-swizzle + MoE-fusion
    // commits are CUDA-side, --lazy-mode is app-layer).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "chunk-transition-consumed-by-mma",
        "carried-state-reduction-stays-scalar",
        "shader-fastdiv-mulhi",
        "sparse-kv-index-list-sentinel-padded",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-113 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch113_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "chunk-transition-consumed-by-mma",
            &[
                "one time and reused",
                "FIVE-CLAUSE",
                "largest multiple of CHUNK",
                "kernel-FAMILY fallback",
                "snap_reserve",
                "tolerance-gated",
            ],
        ),
        (
            "carried-state-reduction-stays-scalar",
            &[
                "Keep the carried-state reduction in token order",
                "compounds rounding differences at every chunk boundary",
                "non-compounding",
                "compounding",
                "matrix unit",
                "per-product",
            ],
        ),
        (
            "shader-fastdiv-mulhi",
            &[
                "umulExtended",
                "round-up",
                "6-bit",
                "quotient-zero",
                "coordinate decomposition",
                "init_fastdiv_values",
            ],
        ),
        (
            "sparse-kv-index-list-sentinel-padded",
            &[
                "SENTINEL",
                "n_kv_max_padded",
                "SINGLE SOURCE OF TRUTH",
                "TRUST-BUT-CLAMP",
                "ascending",
                "Q == 1",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-113 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-113 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch113_pseudocode_parses() {
    // G1 for Batch 113: syn-parse gate (the llama.cpp delta rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "chunk-transition-consumed-by-mma",
        "carried-state-reduction-stays-scalar",
        "shader-fastdiv-mulhi",
        "sparse-kv-index-list-sentinel-padded",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-113 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-113 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-113 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch115_rules_present() {
    // Batch 115 (qwen-3.8-mtp-challenge submission-diff-history walk --
    // Research 139 / Plan 088): 6 kernel_opt rules; source re-cloned at
    // 0863b06ac for the batch (two read lanes + coordinator synthesis;
    // B100 read the tip TREE, this batch read the ~50-diff HISTORY --
    // complementary).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "ordinal-map-exact-tail-selection",
        "complete-coverage-scatter-collapses-operand-to-dense",
        "exhaustive-domain-input-patch-bit-identity",
        "derived-index-from-pinned-checkpoint-is-a-cache",
        "warm-reachable-dispatch-signatures-before-timed-window",
        "env-gate-reaches-and-arms-the-scored-process",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-115 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch115_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "ordinal-map-exact-tail-selection",
            &[
                "TOTAL order",
                "HIGHER index",
                "NaN ranking ABOVE every number",
                "signed zero",
                "a gate that cannot fail is not a gate",
                "14 dependent dispatches",
            ],
        ),
        (
            "complete-coverage-scatter-collapses-operand-to-dense",
            &[
                "COMPLETE permutation",
                "argSort of a permutation is its inverse",
                "DEAD WORK",
                "UNKNOWABLE at install time",
                "committed support",
                "1.00 to 0.877",
            ],
        ),
        (
            "exhaustive-domain-input-patch-bit-identity",
            &[
                "65,280",
                "output-word mapping",
                "lossless",
                "NOT sweepable",
                "boundary negative control",
                "zero mismatches",
            ],
        ),
        (
            "derived-index-from-pinned-checkpoint-is-a-cache",
            &[
                "parameter-class",
                "CANONICAL ORDER",
                "FAIL-LOUD",
                "plausible time for the WRONG mechanism",
                "ONE ATTEMPT",
                "empty clusters are impossible",
            ],
        ),
        (
            "warm-reachable-dispatch-signatures-before-timed-window",
            &[
                "REACHABLE SET",
                "host/device mix",
                "EXPRESSION, not a call-chain prefix",
                "throwaway",
                "measured receipt",
                "mis-attribute",
            ],
        ),
        (
            "env-gate-reaches-and-arms-the-scored-process",
            &[
                "ALLOWLIST",
                "same code twice",
                "FORCE-SET",
                "no-overwrite",
                "ARM WITNESS",
                "BARE-LEG RECEIPT",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-115 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-115 concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch115_pseudocode_parses() {
    // G1 for Batch 115: syn-parse gate (the qwen38-mtp diff-walk rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "ordinal-map-exact-tail-selection",
        "complete-coverage-scatter-collapses-operand-to-dense",
        "exhaustive-domain-input-patch-bit-identity",
        "derived-index-from-pinned-checkpoint-is-a-cache",
        "warm-reachable-dispatch-signatures-before-timed-window",
        "env-gate-reaches-and-arms-the-scored-process",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-115 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-115 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-115 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch115_ordinal_map_reproduces_stable_sort_semantics() {
    // The rule's load-bearing numeric claim, proven in Rust: a monotone
    // float-to-u32 ordinal encoding the reference comparator's corners
    // (NaN above all, signed zeros folded to one class, monotone on the
    // key plane) reproduces a stable ascending sort's tail-K
    // ELEMENT-WISE, including every tie class.
    fn ordinal(v: f32) -> u32 {
        if v.is_nan() {
            return u32::MAX;
        }
        if v == 0.0 {
            return 0x8000_0000;
        }
        let u = v.to_bits();
        if u & 0x8000_0000 != 0 {
            !u
        } else {
            u | 0x8000_0000
        }
    }
    // Reference comparator: stable ascending, NaN ranks above every
    // number, -0.0 == +0.0 (ties keep original index order in the
    // ascending array, so the TAIL keeps the highest indices).
    fn ref_cmp(a: f32, b: f32) -> std::cmp::Ordering {
        match (a.is_nan(), b.is_nan()) {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        }
    }
    // Seeded corpus heavy on corners: zeros, NaNs, infinities, plus a
    // xorshift spread of arbitrary bit patterns.
    let mut values: Vec<f32> = vec![
        0.0,
        -0.0,
        f32::NAN,
        f32::NAN,
        1.5,
        -1.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0e-30,
        -1.0e30,
    ];
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..4000 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let bits = (s & 0xFFFF) as u16;
        let v = f32::from_bits(u32::from(bits) << 7);
        values.push(v); // NaNs and zeros included on purpose
    }
    let n = values.len();
    let k = 33; // deliberately not a power of two

    // Reference: stable ascending sort by the comparator, tail slice.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| ref_cmp(values[a], values[b]));
    let ref_tail: Vec<usize> = order[n - k..].to_vec();

    // Mine: (ordinal, index)-ascending, tail-K = the LAST k -- ties
    // resolve toward the higher index, output ascending.
    let mut pairs: Vec<(u32, usize)> = values
        .iter()
        .enumerate()
        .map(|(i, v)| (ordinal(*v), i))
        .collect();
    pairs.sort_unstable();
    let mine_tail: Vec<usize> = pairs[n - k..].iter().map(|(_, i)| *i).collect();

    assert_eq!(
        mine_tail, ref_tail,
        "ordinal-map selection must be element-wise identical to the \
         reference stable sort's tail"
    );
    // Non-vacuity: at least one zero-fold tie and one NaN tie class
    // must exist in the corpus, or the corners are not exercised.
    assert!(values.iter().filter(|v| **v == 0.0).count() >= 2);
    assert!(values.iter().any(|v| v.is_nan()));
}

#[test]
fn batch115_coverage_collapse_dense_and_partial_arms() {
    // Non-vacuity for the coverage rule: a COMPLETE-permutation island
    // set must route to the dense path with the scatter folded away
    // (the packed compute never runs), while a PARTIAL set must keep
    // the packed-plus-scatter path. Self-contained reimplementation of
    // the fixture's plan logic.
    #[derive(Debug, PartialEq)]
    enum Plan {
        Complete { reordered: Vec<usize> },
        Partial,
    }
    fn install(rows: usize, islands: &[usize]) -> Plan {
        let mut idx: Vec<usize> = islands.to_vec();
        idx.sort_unstable();
        idx.dedup();
        if idx.len() == rows && idx.iter().enumerate().all(|(i, r)| *r == i) {
            Plan::Complete { reordered: idx }
        } else {
            Plan::Partial
        }
    }
    // Complete: every row covered exactly once -> dense, and the
    // reorder is the identity (the install-time argSort-inverse).
    assert_eq!(
        install(4, &[2, 0, 3, 1]),
        Plan::Complete {
            reordered: vec![0, 1, 2, 3]
        }
    );
    // Partial: two of four rows -> scatter path retained.
    assert_eq!(install(4, &[1, 3]), Plan::Partial);
    // Duplicate indices are NOT complete coverage even when the count
    // matches (dedup collapses them) -- the naive form would be fooled.
    assert_eq!(install(4, &[0, 0, 1, 2]), Plan::Partial);
    // All-covered-but-permutation-incomplete (count < rows).
    assert_eq!(install(4, &[0, 1, 2]), Plan::Partial);
}

#[test]
fn batch115_addendum_rules_present() {
    // Batch 115 ADDENDUM (collision salvage -- 2026-09-06): the sibling
    // Batch 115 (11863f8) landed first with 6 different rules; per the
    // push-wins tie-breaker (the B103-addendum precedent) the duplicate
    // yielded and these 4 rules land as the salvage the landed batch
    // genuinely lacks. Quotes were coordinator-verified at pin 0863b06ac
    // BEFORE the collision.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "dispatch-cliff-bitexact-shape-confinement",
        "readout-trim-needs-committed-id-coverage",
        "dead-output-rows-state-append-only-path",
        "distinct-index-bitmap-ascending-compaction",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-115 addendum rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch115_addendum_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "dispatch-cliff-bitexact-shape-confinement",
            &[
                "qL * gqa <= 32",
                "VALUE drift",
                "accumulation order",
                "SINGLE SOURCE OF TRUTH",
                "proven width range",
            ],
        ),
        (
            "readout-trim-needs-committed-id-coverage",
            &[
                "49_152",
                "REGRESSED",
                "committed argmax ids",
                "escape range",
                "1.00 -> 0.877",
            ],
        ),
        (
            "dead-output-rows-state-append-only-path",
            &[
                "query+gate projection",
                "KV-only",
                "FAILS CLOSED",
                "STATE-APPEND-ONLY",
                "final axis",
            ],
        ),
        (
            "distinct-index-bitmap-ascending-compaction",
            &[
                "no tie rule",
                "exclusive",
                "ascending",
                "bitmap",
                "SET IDENTITY",
            ],
        ),
    ];
    for (id, needles) in checks {
        let e = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-115 addendum rule '{id}' missing"));
        for needle in *needles {
            assert!(
                e.gpu_concept.contains(needle),
                "batch-115 addendum concept for '{id}' missing phrase '{needle}'"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch115_addendum_pseudocode_parses() {
    // G1 for the Batch 115 addendum: syn-parse gate (the salvage rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "dispatch-cliff-bitexact-shape-confinement",
        "readout-trim-needs-committed-id-coverage",
        "dead-output-rows-state-append-only-path",
        "distinct-index-bitmap-ascending-compaction",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-115 addendum rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-115 addendum ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-115 addendum opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch116_rules_present() {
    // G1 for Batch 116: the six new rules must be present in the corpus.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "activation-bitplane-popcount-word-dot",
        "scratch-sizing-shares-dispatch-eligibility-predicate",
        "byte-owning-threads-non-positional-digit-codes",
        "fwht-wide-block-row-per-threadgroup-exchange-ladder",
        "shader-shape-cutover-keys-on-resource-block-ratio",
        "same-input-folded-transform-memo-per-build",
    ];
    assert_eq!(
        corpus.entries.len(),
        734,
        "corpus count re-pinned at Batch 197 (the mlx VJP distill: 6 rules — the composed-backward A− GOAT lead, the segmented entry-state checkpoint + cheap replay with the register arm folded, the GQA zero-stride batch broadcast, the threadgroup-scope reduction single-writer, the forward-save chunk-intermediate cache, and the chain-rule postprocess kernel; was 728 at Batch 196)",
    );
    let present: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
    for expected in ids {
        assert!(
            present.contains(&expected),
            "batch-116 rule '{expected}' missing from corpus"
        );
    }
}

#[test]
fn batch116_concepts_covered() {
    // G1 for Batch 116: the decision-content needles must survive (case-sensitive
    // contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "activation-bitplane-popcount-word-dot",
            &[
                "~0.8 integer ops per weight per column against ~3 for select-per-weight",
                "removed ONLY the bit test and ran 1.7x faster",
                "gate on measured logit-KLD before enabling by default",
            ],
        ),
        (
            "scratch-sizing-shares-dispatch-eligibility-predicate",
            &[
                "the encoder must take this path exactly when the allocator reserved for it",
                "a memory-safety hazard, not a perf one",
                "nothing at all is reserved when the path is off",
            ],
        ),
        (
            "byte-owning-threads-non-positional-digit-codes",
            &[
                "five times the byte traffic for the same 24 bytes",
                "weight traffic is what decode is bound by",
                "byte-owning threads 29.8 -> 136.4",
            ],
        ),
        (
            "fwht-wide-block-row-per-threadgroup-exchange-ladder",
            &[
                "stages below the simdgroup width ride simd_shuffle (zero memory)",
                "a named boundary is a pointer, not ownership",
                "exactly consuming Apple's 32 KB threadgroup budget",
            ],
        ),
        (
            "shader-shape-cutover-keys-on-resource-block-ratio",
            &[
                "the same wall that forced the wide kernel",
                "which is the self-check that the added term only fires where it must",
                "not bit-identical, do not cite bit-identity here",
            ],
        ),
        (
            "same-input-folded-transform-memo-per-build",
            &[
                "ASSERTS BY COMMENT and does not check",
                "LIFETIME = one graph build",
                "the one bit-identity claim here that is structural",
            ],
        ),
        // Rider needles: the appended rider content must be present on the ridden rules.
        (
            "telescoped-digit-coefficients-byte-fma-dot",
            &[
                "the exactness criterion is the MANTISSA BOUND",
                "two floors replacing a lane-divergent loop",
            ],
        ),
        (
            "small-batch-matvec-gemm-bridge-rung",
            &[
                "keep nr1*SW <= 32 staged y values per thread",
                "the mv-to-mm crossover is overridden PER TYPE",
            ],
        ),
        (
            "dispatcher-eligibility-predicate-lags-kernel-capability",
            &[
                "derived from PQ2_0's rather than transcribed",
                "while still printing a clean tail",
            ],
        ),
    ];
    for (id, needles) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-116 check rule '{id}' missing"));
        for needle in *needles {
            assert!(
                entry.gpu_concept.contains(needle),
                "batch-116 concept needle missing from '{id}': {needle:?}"
            );
        }
    }
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch116_pseudocode_parses() {
    // G1 for Batch 116: syn-parse gate (all six new rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "activation-bitplane-popcount-word-dot",
        "scratch-sizing-shares-dispatch-eligibility-predicate",
        "byte-owning-threads-non-positional-digit-codes",
        "fwht-wide-block-row-per-threadgroup-exchange-ladder",
        "shader-shape-cutover-keys-on-resource-block-ratio",
        "same-input-folded-transform-memo-per-build",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-116 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-116 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-116 opt_pattern for '{id}' must parse"
        );
    }
}

// ── Batch 116 ADDENDUM gates (the collision salvage — Research 140 addendum 2) ──

#[test]
fn batch116_addendum_rules_present_and_concepts() {
    let corpus = build_kernel_seed_corpus();
    let entry = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "swar-lane-widened-base3-dp4a-dot")
        .expect("addendum rule missing from corpus");
    for n in [
        "cannot carry between bytes",
        "765 < 2^16",
        "SUM OF TERM MAGNITUDES",
        "NEVER COMPILED",
        "no byte-aligned run inside a 32-chunk",
        "FOUR independent surfaces",
    ] {
        assert!(
            entry.gpu_concept.contains(n),
            "addendum concept lost the mandated phrase: {n}"
        );
    }
}

#[test]
fn batch116_addendum_pseudocode_parses() {
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let entry = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "swar-lane-widened-base3-dp4a-dot")
        .expect("addendum rule missing");
    assert!(
        verdict_passes(&KernelOptDomain::validate("swar", entry.buggy_code())),
        "addendum ref_shape must parse"
    );
    assert!(
        verdict_passes(&KernelOptDomain::validate("swar", entry.fixed_code())),
        "addendum opt_pattern must parse"
    );
}

#[test]
fn batch116_addendum_base3_mantissa_bound_exhaustive() {
    // Pins the exactness claim the Batch 116 rider on
    // telescoped-digit-coefficients-byte-fma-dot makes in prose: floor
    // extraction of a base-r fraction of 2^m is bit-exact vs the integer
    // recurrence IFF r^positions * max_code < 2^24. Exhaustive over all
    // 256 bytes x 5 positions at radix 3 (the PTQ1_0 codec), PLUS the
    // falsifiable boundary: radix 11 violates the bound and MUST
    // diverge -- a gate that cannot fail is not a gate.
    fn reference_digits(b: u32, radix: u32, positions: usize) -> Vec<i32> {
        let mut v = b;
        let mut out = Vec::with_capacity(positions);
        for _ in 0..positions {
            let w = v * radix;
            out.push((w >> 8).cast_signed());
            v = w & 0xFF;
        }
        out
    }
    // The floor arm's casts are the algorithm under test: b <= 255 fits f32
    // exactly, and the int cast consumes a provably-integral floor (within
    // the bound) or a deliberately-out-of-bound value (the radix-11 control,
    // where divergence is the assertion) -- the B59 algorithm-is-the-cast
    // allow, per-line justification.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn floor_digits(b: u32, radix: u32, positions: usize) -> Vec<i32> {
        let u = (b as f32) * (1.0f32 / 256.0);
        let radix_f = radix as f32;
        let mut g_prev = 0.0f32;
        let mut out = Vec::with_capacity(positions);
        for k in 1..=positions {
            let g_next = (radix.pow(u32::try_from(k).unwrap()) as f32 * u).floor();
            out.push((g_next - radix_f * g_prev) as i32);
            g_prev = g_next;
        }
        out
    }
    // radix 3, 5 positions: 3^5 * 255 = 61965 < 2^24 -- exact everywhere
    for b in 0..=255u32 {
        assert_eq!(
            floor_digits(b, 3, 5),
            reference_digits(b, 3, 5),
            "radix-3 floor extraction diverged at byte {b}"
        );
    }
    // radix 11, 5 positions: 11^5 * 255 = 40_552_605 > 2^24 -- the bound
    // is violated, and at least one byte MUST diverge (the criterion is
    // the mantissa bound, not the radix being odd)
    let diverged = (0..=255u32).any(|b| floor_digits(b, 11, 5) != reference_digits(b, 11, 5));
    assert!(
        diverged,
        "radix-11 control never diverged -- the mantissa-bound gate is vacuous"
    );
    // and the boundary itself: radix 7, 5 positions = 7^5 * 255 = 4_279_485
    // < 2^24 -- still exact, proving the bound is the right criterion
    for b in 0..=255u32 {
        assert_eq!(
            floor_digits(b, 7, 5),
            reference_digits(b, 7, 5),
            "radix-7 (within-bound) extraction diverged at byte {b}"
        );
    }
}

#[test]
fn batch116_addendum_swar_lanes_exhaustive() {
    // Exhaustive: the lane-widened SWAR form reproduces the per-byte
    // reference recurrence for every byte value in every lane position,
    // under adversarial neighbor lanes (all-00 / all-FF / mixed) — the
    // widened-lane multiply cannot carry between bytes because each
    // lane's product is <= 765 < 2^16. Plus the exhaustive adjacent-pair
    // sweep (the only interaction channel).
    fn reference_digits(b: u8) -> [i32; 5] {
        let mut v = u32::from(b);
        let mut out = [0i32; 5];
        for slot in &mut out {
            let w = v * 3;
            *slot = (w >> 8).cast_signed();
            v = w & 0xFF;
        }
        out
    }
    // byte_perm scatter (0x4140 / 0x4342): bytes 0,1 -> v_lo lanes,
    // bytes 2,3 -> v_hi lanes, each widened with a zero high half.
    // Returns [step][lane] biased digits {-1,0,+1}.
    fn swar_word_digits(word: u32) -> [[i32; 4]; 5] {
        let mut v_lo = (word & 0xFF) | (((word >> 8) & 0xFF) << 16);
        let mut v_hi = ((word >> 16) & 0xFF) | (((word >> 24) & 0xFF) << 16);
        let mut steps = [[0i32; 4]; 5];
        for row in &mut steps {
            let w_lo = v_lo.wrapping_mul(3);
            let w_hi = v_hi.wrapping_mul(3);
            v_lo = w_lo & 0x00FF_00FF;
            v_hi = w_hi & 0x00FF_00FF;
            *row = [
                ((w_lo >> 8) & 0xFF).cast_signed() - 1,
                ((w_lo >> 24) & 0xFF).cast_signed() - 1,
                ((w_hi >> 8) & 0xFF).cast_signed() - 1,
                ((w_hi >> 24) & 0xFF).cast_signed() - 1,
            ];
        }
        steps
    }
    fn word_from(b0: u8, b1: u8, b2: u8, b3: u8) -> u32 {
        u32::from(b0) | (u32::from(b1) << 8) | (u32::from(b2) << 16) | (u32::from(b3) << 24)
    }
    fn assert_stream(got: &[[i32; 4]; 5], lane: usize, reference: &[i32; 5], ctx: &str) {
        for (step, row) in got.iter().enumerate() {
            assert_eq!(
                row[lane] + 1,
                reference[step],
                "{ctx}: lane {lane} diverged at step {step}"
            );
        }
    }
    for b in 0..=255u8 {
        let rb = reference_digits(b);
        for pattern in [0x00u8, 0xFF, 0xAA] {
            let got = swar_word_digits(word_from(b, pattern, pattern, pattern));
            assert_stream(&got, 0, &rb, "lane0 sweep");
            let got2 = swar_word_digits(word_from(pattern, b, 0x55, pattern));
            assert_stream(&got2, 1, &rb, "lane1 sweep");
            let got3 = swar_word_digits(word_from(0x33, 0x77, b, pattern));
            assert_stream(&got3, 2, &rb, "lane2 sweep");
            let got4 = swar_word_digits(word_from(0x11, 0x22, 0x44, b));
            assert_stream(&got4, 3, &rb, "lane3 sweep");
        }
    }
    for a in 0..=255u8 {
        for c in 0..=255u8 {
            let got = swar_word_digits(word_from(a, c, 0, 0));
            let ra = reference_digits(a);
            let rc = reference_digits(c);
            assert_stream(&got, 0, &ra, "pair sweep a");
            assert_stream(&got, 1, &rc, "pair sweep c");
        }
    }
}

#[test]
fn batch117_rules_present() {
    // Batch 117 (antirez/ds4 Metal tensor-prefill delta -- Plan 090 /
    // Research 141): 5 kernel_opt rules mined at the fresh pin 9ab7053
    // (Batch 97/98's pin 110afdd + ~14 commits; two read lanes +
    // coordinator spot-verification; quote line-drifts corrected).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "routed-activation-pack-once-shared-operand",
        "device-affine-scratch-admission-with-pool-fallback",
        "batch-verify-rows-single-dispatch",
        "row-aware-indexing-reuses-static-single-token-kernel",
        "dense-fast-path-bounded-by-selector-coverage",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-117 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch117_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-117 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("routed-activation-pack-once-shared-operand");
    assert!(
        c.contains("EXACTLY ONCE")
            && c.contains("256 MiB")
            && c.contains("computed-and-discarded")
            && c.contains("same-input-gemv-weight-concat-at-upload"),
        "pack-once concept"
    );
    let c = concept("device-affine-scratch-admission-with-pool-fallback");
    assert!(
        c.contains("THREE-CHECK TUPLE")
            && c.contains("WRITE-BEFORE-READ")
            && c.contains("19.25/16.22/15.97"),
        "scratch admission concept"
    );
    let c = concept("batch-verify-rows-single-dispatch");
    assert!(
        c.contains("HOISTED OUT OF THE BRANCH")
            && c.contains("STATELESS across rows")
            && c.contains("{6,2,5,3,4,6}")
            && c.contains("moe-routed-down-sum-single-accumulate-pass"),
        "verify-batch concept"
    );
    let c = concept("row-aware-indexing-reuses-static-single-token-kernel");
    assert!(
        c.contains("VERBATIM")
            && c.contains("reuse-instead-of-twin")
            && c.contains("hardcoded-fast-path-jit-specialization")
            && c.contains("batch-verify-rows-single-dispatch"),
        "row-aware concept"
    );
    let c = concept("dense-fast-path-bounded-by-selector-coverage");
    assert!(
        c.contains("top_k + anchor_pool - 1")
            && c.contains("never the work-window size")
            && c.contains("sparse-kv-index-list-sentinel-padded"),
        "selector coverage concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch117_pseudocode_parses() {
    // G1 for Batch 117: syn-parse gate (the ds4 delta rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "routed-activation-pack-once-shared-operand",
        "device-affine-scratch-admission-with-pool-fallback",
        "batch-verify-rows-single-dispatch",
        "row-aware-indexing-reuses-static-single-token-kernel",
        "dense-fast-path-bounded-by-selector-coverage",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-117 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-117 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-117 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch118_rules_present() {
    // Batch 118 (antirez/ds4 PR #991, Qwen3.8-Flash-Next -- Plan 097 /
    // Research 142): 6 kernel_opt rules mined at the fresh pin 236cb2a5
    // (upstream main unchanged at 9ab7053 = the Batch-117 pin, so the PR
    // branch IS the delta; two read lanes + coordinator synthesis; the
    // campaign kernel generation verified at 35ce0b0 + HEAD successors).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "role-scoped-store-must-match-compute-guard",
        "tile-amplification-precedes-staging-pass",
        "operand-precision-gated-by-consumer-invariance",
        "compute-role-split-by-accumulator-pressure",
        "speculation-scheduler-break-even-flip",
        "running-threshold-prune-compaction-aliasing-guard",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-118 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch118_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-118 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("role-scoped-store-must-match-compute-guard");
    assert!(
        c.contains("EXACTLY 1/n")
            && c.contains("LOAD-BEARING")
            && c.contains("row_half")
            && c.contains("six measured sessions")
            && c.contains("smem-slot-role-reversal-needs-barrier"),
        "store-guard concept"
    );
    let c = concept("tile-amplification-precedes-staging-pass");
    assert!(
        c.contains("3.2x padded FLOPs")
            && c.contains("125.2 -> 42.2")
            && c.contains("NET-NEGATIVE")
            && c.contains("knob that does not reach the loop")
            && c.contains("kill-switch constant"),
        "tile-amplification concept"
    );
    let c = concept("operand-precision-gated-by-consumer-invariance");
    assert!(
        c.contains("0.0000082")
            && c.contains("0.0581")
            && c.contains("greedy diverging at token 2")
            && c.contains("MODEL-SCOPED")
            && c.contains("router-math-stays-fp32"),
        "operand-precision concept"
    );
    let c = concept("compute-role-split-by-accumulator-pressure");
    assert!(
        c.contains("~200 registers")
            && c.contains("1.50x")
            && c.contains("0% +- 1%")
            && c.contains("plane-specialization-load-compute-role-split")
            && c.contains("role-scoped-store-must-match-compute-guard"),
        "role-split concept"
    );
    let c = concept("speculation-scheduler-break-even-flip");
    assert!(
        c.contains("202 bypass vs 62")
            && c.contains("110%")
            && c.contains("0.59-0.84")
            && c.contains("1.88-4.54")
            && c.contains("DSpark"),
        "break-even concept"
    );
    let c = concept("running-threshold-prune-compaction-aliasing-guard");
    assert!(
        c.contains("Kth-best")
            && c.contains("memcmp == 0")
            && c.contains("stale tail entries")
            && c.contains("first-tile fallback")
            && c.contains("lane-owned-topk-output"),
        "threshold-prune concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch118_pseudocode_parses() {
    // G1 for Batch 118: syn-parse gate (the ds4 PR #991 rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "role-scoped-store-must-match-compute-guard",
        "tile-amplification-precedes-staging-pass",
        "operand-precision-gated-by-consumer-invariance",
        "compute-role-split-by-accumulator-pressure",
        "speculation-scheduler-break-even-flip",
        "running-threshold-prune-compaction-aliasing-guard",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-118 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-118 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-118 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
#[allow(clippy::cast_precision_loss)] // fixture RNG draws and an n=8 reciprocal -- the detector
                                      // asserts spreads orders of magnitude past f32 mantissa granularity
fn batch118_unguarded_store_yields_uniform_softmax() {
    // The load-bearing numeric gate for role-scoped-store-must-match-
    // compute-guard: a role-partitioned attention tile where the PV roles'
    // row_half maps onto the score roles' rows. UNGUARDED stores overwrite
    // real scores with the PV roles' (zero-filled) fragments -> pre-softmax
    // scores collapse equal -> softmax weights EXACTLY uniform while the
    // running max stays sane (nothing NaNs). The GUARDED twin keeps the true
    // spread. Production magnitude is the point: at tiny fixtures the uniform
    // weights are inside a 5e-3 + 3e-2|ref| tolerance of the true weights
    // (asserted here) -- that is exactly why the bug survived six sessions.
    const EPS: f32 = 1e-7;
    // Two role halves x 8 rows; score rows carry production-magnitude spreads.
    // xorshift64* - integer state, one f32 divide per draw; the mantissa
    // losses are irrelevant to a spread detector (production magnitude is
    // orders past the 2^23 mantissa boundary).
    let mut rng_state: u64 = 0x243F_6A88_85A3_08D3;
    let mut rand = move || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        ((rng_state >> 40) & 0xFFFF) as f32 / 65536.0f32 - 0.5
    };
    let q_scale = 12.0f32; // x12 input scale: score spreads reach ~+-20
    let true_scores: Vec<Vec<f32>> =
        (0..2) // row_half -> 8 rows of scores
            .map(|_| (0..8).map(|_| rand() * 40.0 * q_scale).collect())
            .collect();
    let softmax = |scores: &[f32]| -> Vec<f32> {
        let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let exps: Vec<f32> = scores.iter().map(|s| (s - max).exp()).collect();
        let sum: f32 = exps.iter().sum();
        exps.iter().map(|e| e / sum).collect()
    };
    // GUARDED twin: only score roles store; PV fragments never reach s_tile.
    let guarded_weights: Vec<Vec<f32>> = true_scores.iter().map(|row| softmax(row)).collect();
    for row in &guarded_weights {
        let spread = row.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - row.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            spread > 0.5,
            "guarded store must keep a real weight spread, got {spread}"
        );
    }
    // UNGUARDED twin: PV roles (uninitialized -> zeros, the passing-run
    // reading) store LAST onto the same rows. Scores collapse equal.
    let mut s_tile = vec![vec![0.0f32; 8]; 2];
    for (half, rows) in true_scores.iter().enumerate() {
        s_tile[half] = rows.clone(); // score roles (sg 0/1) store first
    }
    for row_half in &mut s_tile {
        // PV roles sg 2,3 -> row_half 0 and sg 4,5 -> row_half 1 store zeros.
        row_half.fill(0.0);
    }
    // The lazy running-max stays FINITE and sane (0.0) -- nothing NaNs.
    for row in &s_tile {
        let max = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max.is_finite(),
            "running max must stay finite (the source bug's invisibility)"
        );
    }
    for row in &s_tile {
        let w = softmax(row);
        let spread = w.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - w.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(
            spread < EPS,
            "unguarded store must collapse softmax to EXACTLY uniform, spread {spread}"
        );
    }
    // Determinism x2 (rerun the unguarded detector on a fresh clone of the tile).
    let s_tile2 = s_tile.clone();
    for row in &s_tile2 {
        let w = softmax(row);
        let spread = w.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - w.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(spread < EPS, "deterministic rerun must stay uniform");
    }
    // WHY the old tiny-magnitude fixtures passed: at tiny-fixture-class score
    // spreads (~0.1, the |q| <= 0.12 input class) the uniform weights sit
    // INSIDE the source's 5e-3 + 3e-2|ref| tolerance of the true weights
    // (max weight deviation ~ spread/(2n) ~ 6e-3 < ~8.7e-3 here).
    let tiny: [f32; 8] = [-0.05, -0.037_5, -0.025, -0.012_5, 0.0, 0.012_5, 0.025, 0.037_5];
    let w_true = softmax(&tiny);
    let w_uniform = vec![1.0 / 8.0; 8];
    for (t, u) in w_true.iter().zip(&w_uniform) {
        let tol = 5e-3 + 3e-2 * t.abs();
        assert!(
            (t - u).abs() <= tol,
            "tiny-magnitude invisibility: |{t} - {u}| must fit {tol}"
        );
    }
    // And the same tolerance FAILS at production magnitude (the fixture gap).
    let prod = &guarded_weights[0];
    let prod_uniform: Vec<f32> = vec![1.0 / prod.len() as f32; prod.len()];
    let max_err = prod
        .iter()
        .zip(&prod_uniform)
        .map(|(t, u)| (t - u).abs())
        .fold(0.0f32, f32::max);
    let within_tol = prod
        .iter()
        .zip(&prod_uniform)
        .all(|(t, u)| (t - u).abs() <= 5e-3 + 3e-2 * t.abs());
    assert!(
        max_err > 0.0 && !within_tol,
        "production magnitude must EXCEED the tolerance (the six-session invisibility ends here)"
    );
}

#[test]
#[allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::needless_range_loop
)] // exact f32 equality IS the tested contract here (integer-valued partials
   // keep every add exact, so no tolerance exists to hide behind) and the
   // ladders are inherently index-structured (stage j reads lane j)
fn batch118_indexed_shuffle_ladders_must_read_unmutated_partials() {
    // The block-reduction rider's decision content, pinned executable:
    // an INDEXED shuffle ladder (partner = lane index j, not xor mask) that
    // accumulates into the LIVE partial double-counts lanes (v0 + 4v1 +
    // 2v2 + v3 here); reading from an UNMUTATED copy is exact; the XOR
    // butterfly is safe in place (disjoint partner pairs per stage); and a
    // MAX reduction is blind to the whole bug (idempotence masks it -- which
    // is exactly why the symptom is a uniformly-scaled SUM with a sane max).
    // Integer-valued partials (< 2^24) keep every f32 add exact, so all
    // comparisons below are EXACT equality, no tolerance.
    let v = [7.0f32, 11.0, 13.0, 17.0];
    // Reference: sequential sum in lane order.
    let reference = ((v[0] + v[1]) + v[2]) + v[3];
    // Correct indexed ladder: accumulate from an UNMUTATED copy.
    let frozen = v;
    let mut acc = v[0];
    for &part in frozen.iter().skip(1) {
        acc += part;
    }
    assert_eq!(acc, reference, "unmutated indexed ladder must be exact");
    // BUGGY in-place indexed ladder: every lane accumulates into its own
    // live slot; stage j reads lane j's END-OF-PREVIOUS-STAGE value (SIMD
    // lockstep: all reads happen before any write within a stage).
    let mut live = v;
    for stage in 1..4 {
        let snapshot = live;
        let delta = snapshot[stage];
        for lane in &mut live {
            *lane += delta;
        }
    }
    let buggy_lane0 = live[0];
    assert_ne!(
        buggy_lane0, reference,
        "in-place indexed ladder must double-count (bug present)"
    );
    // The exact double-count shape for this ladder: v0 + 4v1 + 2v2 + v3.
    assert_eq!(
        buggy_lane0,
        v[0] + 4.0 * v[1] + 2.0 * v[2] + v[3],
        "the double-count coefficients are the rule's falsifiable content"
    );
    // XOR butterfly is SAFE in place (partners are disjoint per stage).
    let mut x = v;
    x[0] += x[1];
    x[2] += x[3];
    x[0] += x[2];
    assert_eq!(
        x[0],
        (v[0] + v[1]) + (v[2] + v[3]),
        "xor butterfly in place = explicit pairwise association (exact)"
    );
    // MAX reduction is BLIND to the bug (idempotence masks it): the same
    // buggy lockstep ladder on max returns the correct max.
    let mut m = v;
    for stage in 1..4 {
        let snapshot = m;
        let running = snapshot[stage];
        for lane in &mut m {
            *lane = lane.max(running);
        }
    }
    assert_eq!(
        m[0],
        v.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        "max survives the buggy ladder by idempotence -- sums do NOT"
    );
}

#[test]
fn batch119_rules_present() {
    // Batch 119 (ss16118/low-latency-nccl near-SoL collectives -- Plan 098 /
    // Research 143): 2 kernel_opt rules mined at the fresh pin 5357eff3
    // (BSD-3 NVIDIA; the repo names the sentinel POISON and enumerates LL128
    // as NOT-YET-IMPLEMENTED -- provenance corrections carried in-rule).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "sentinel-readiness-over-separate-flag",
        "ll-flag-in-data-packed-poll",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-119 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch119_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept
    // text (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-119 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("sentinel-readiness-over-separate-flag");
    assert!(
        c.contains("User MUST poison buffer before receiving")
            && c.contains("-327686")
            && c.contains("RESET-BEFORE-REUSE")
            && c.contains("PEER-STORE-VISIBILITY")
            && c.contains("SINGLE-WRITER SLOTS")
            && c.contains("stripe-sms-l2-semaphore-reduce")
            && c.contains("mask-sentinel-matches-reduction-identity")
            && c.contains("5357eff325eddf978137de7140195a5568fa8a11"),
        "sentinel concept"
    );
    let c = concept("ll-flag-in-data-packed-poll");
    assert!(
        c.contains("epoch band")
            && c.contains("Multiple buffering needs to be used")
            && c.contains("NOT YET IMPLEMENTED")
            && c.contains("3.8e-6")
            && c.contains("(flag0 == flag1) ? flag0 : ~0u")
            && c.contains("stripe-sms-l2-semaphore-reduce")
            && c.contains("sentinel-readiness-over-separate-flag"),
        "ll-flag concept"
    );
    // The B58 rider's mandated phrases (bidirectional peer-exchange credit).
    let c = concept("output-buffer-rotation-pool-multi-forward");
    assert!(
        c.contains("each receive is implicit permission for the next send")
            && c.contains("2607.16100")
            && c.contains("roundRobinFactor")
            && c.contains("ownership exchange")
            && c.contains("never un-rotated mid-run"),
        "B58 rider concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch119_pseudocode_parses() {
    // G1 for Batch 119: syn-parse gate (the low-latency-nccl rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "sentinel-readiness-over-separate-flag",
        "ll-flag-in-data-packed-poll",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-119 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-119 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-119 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
#[allow(clippy::float_cmp)] // the poison compare IS a bit pattern compare;
                            // exact f32 equality only ever asserts on from_bits-round-tripped values
fn batch119_sentinel_never_equal_and_reset_before_reuse() {
    // The sentinel rule's decision content, pinned executable: (1) the poison
    // pattern is a NaN so no finite f32 payload can bit-match it; (2) the
    // int-dtype hazard is REAL -- the source's 4-byte poison IS a
    // representable i32 payload (-459782), a collision no NaN argument saves;
    // (3) WITHOUT reset-before-reuse the second round consumes STALE data as
    // fresh (the ABA the LL epoch band is immune to); WITH reset the reader
    // waits for the genuinely new value; (4) readiness is ALL-SLOTS.
    const POISON: u32 = 0xFFFA_FFFA; // the source's 4-byte poison table entry
    let is_poisoned = |v: f32| v.to_bits() == POISON;
    // (1) NaN payload: every finite value is deliverable, the poison is not.
    assert!(
        f32::from_bits(POISON).is_nan(),
        "poison must be a NaN pattern"
    );
    let finite_samples = [0.0f32, -0.0, 1.5, -13.25, f32::MIN, f32::MAX];
    for v in finite_samples {
        assert!(
            !is_poisoned(v),
            "finite payload {v} must never read as poison"
        );
    }
    assert!(is_poisoned(f32::from_bits(POISON)));
    // (2) the int-dtype hazard: the same bits are a legitimate i32 payload.
    assert_eq!(
        POISON.cast_signed(),
        -327_686,
        "the source's 4-byte poison IS a representable i32 -- the documented \
         int-dtype collision hazard, structurally unfixable for integers"
    );
    // (3) ABA: round 1 delivers 1.0; round 2 WITHOUT reset delivers the STALE
    // 1.0 (the reader cannot tell old from new); WITH reset it waits for 2.0.
    let mut slot: f32;
    // round 1: writer stores (the init pass poisoned the slot before this),
    // reader polls non-poison.
    slot = 1.0;
    assert!(!is_poisoned(slot), "round-1 value must be ready");
    assert_eq!(slot, 1.0);
    // round 2, no reset: the slot STILL reads 1.0 -- stale-as-fresh.
    assert_eq!(
        slot, 1.0,
        "without reset the reader consumes the STALE value as fresh (ABA)"
    );
    // round 2, WITH reset: re-poison on consume, then the poll blocks until
    // the writer's genuinely new value arrives.
    slot = f32::from_bits(POISON); // the receive path's Reset arm
    assert!(
        is_poisoned(slot),
        "reset reader must wait (slot reads poison again)"
    );
    slot = 2.0; // the writer's round-2 store
    assert!(
        !is_poisoned(slot) && slot == 2.0,
        "reset protocol delivers 2.0"
    );
    // (4) readiness is ALL-SLOTS: one poison slot holds the batch back.
    let buf = [1.0f32, f32::from_bits(POISON), 3.0];
    let ready = buf.iter().all(|v| !is_poisoned(*v));
    assert!(!ready, "one poison slot must hold the batch not-ready");
    let buf = [1.0f32, 2.0, 3.0];
    assert!(buf.iter().all(|v| !is_poisoned(*v)), "all-slots ready");
}

#[test]
#[allow(clippy::needless_range_loop)] // the epoch ladder is inherently index-structured
fn batch119_epoch_band_wraps_and_split_slot_matches() {
    // The LL rule's decision content, pinned executable: (1) the epoch band
    // cycles 2..=255 and NEVER touches 0/1 (reset writes flag 0); (2) across
    // a full wrap epoch 2 RECURS -- same-slot flag aliasing exists, which is
    // exactly why LL REQUIRES the sub-buffer rotation; (3) the rotation offset
    // changes per epoch, making the aliased copy unreachable; (4) a split
    // 16-byte payload is ready only when BOTH slot flags match (the ~0u retry
    // encoding); (5) a stale flag never passes the poll within a wrap.
    const NOT_READY: u32 = !0u32; // the ~0u retry encoding (used in (4) below)
                                  // (1) the source's advanceEpoch arithmetic on a uint8_t.
    let mut epoch: u8 = 2; // the constructor's initial epoch
    let mut seen_zero_or_one = false;
    for _ in 0..1000 {
        epoch = epoch.wrapping_add(if epoch == 255 { 3 } else { 1 });
        seen_zero_or_one |= epoch == 0 || epoch == 1;
    }
    assert!(
        !seen_zero_or_one,
        "the epoch band must reserve 0 and 1 across any number of advances"
    );
    // (2) across a full wrap, epoch 2 recurs: the aliasing that demands
    // multi-buffering (a same-slot stale flag == live epoch).
    let mut epoch: u8 = 2;
    for _ in 0..254 {
        epoch = epoch.wrapping_add(if epoch == 255 { 3 } else { 1 });
    }
    assert_eq!(
        epoch, 2,
        "the band wraps 2..=255 back to 2 -- flag aliasing"
    );
    // (3) the rotation: offset = (subBuffer % roundRobinFactor) * pitch
    // changes every epoch, so the aliased old copy is physically unreachable.
    let (pitch, rr) = (64usize, 2usize);
    let mut offsets = Vec::with_capacity(4);
    for (sub_buffer, _) in (0..4).enumerate() {
        offsets.push((sub_buffer % rr) * pitch);
    }
    assert_eq!(
        offsets,
        vec![0, 64, 0, 64],
        "the rotation alternates sub-buffers"
    );
    // (4) split-slot both-flags-match with the ~0u retry encoding.
    let split_slot_ready = |f0: u32, f1: u32, _epoch: u32| {
        if f0 == f1 {
            f0
        } else {
            NOT_READY
        }
    };
    assert_eq!(split_slot_ready(7, 7, 7), 7, "both flags match -> ready");
    assert_eq!(
        split_slot_ready(7, 8, 7),
        NOT_READY,
        "mismatched split flags encode retry, never partial delivery"
    );
    // (5) a stale flag fails the poll within a wrap; flag 0 (reset) never
    // passes at any live epoch.
    let poll = |flag: u32, epoch: u32| flag == epoch;
    assert!(!poll(5, 6) && poll(6, 6), "stale fails, fresh passes");
    assert!(!poll(0, 6), "the reset flag 0 never passes any live epoch");
}

#[test]
fn batch125_rules_present() {
    // Batch 125 (the ds4 PR rebase-surge -- Plan 106 / Research 148): 4
    // kernel_opt rules mined at the fresh pin ffd85d42 (MIT; the PR line was
    // REWRITTEN onto upstream main c0a6119f, so the true delta is ~5 tip
    // commits after subtracting B123's upstream window and the B118/B121/B122
    // Qwen stack -- see the module doc in entries_gpu_32.rs).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "kv-reuse-prefix-ends-at-last-evaluated-token",
        "new-syntax-family-enumerates-every-keyed-surface",
        "interleave-independent-accumulators-only",
        "fixed-point-parity-needs-reference-rounding",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-125 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch125_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-125 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("kv-reuse-prefix-ends-at-last-evaluated-token");
    assert!(
        c.contains("BEFORE the assistant end token")
            && c.contains("so the next suffix actually evaluates it")
            && c.contains("Use the decode finish, not the parser's promoted tool_calls status")
            && c.contains("658b2fc")
            && c.contains("ffd85d42")
            && c.contains("prefix-reuse-key-must-mirror-the-restore-predicate"),
        "kv-reuse concept"
    );
    let c = concept("new-syntax-family-enumerates-every-keyed-surface");
    assert!(
        c.contains("orphan-ending arm")
            && c.contains("POSITIVE per-family engagement regression test")
            && c.contains("ds4_agent.c:1393")
            && c.contains("4010cc6")
            && c.contains("implemented-surface-requires-provisioning-target"),
        "syntax-family concept"
    );
    let c = concept("interleave-independent-accumulators-only");
    assert!(
        c.contains("combining the chunk dots would change rounding")
            && c.contains("only independent rows/projections are interleaved")
            && c.contains("1854-1857")
            && c.contains("f91e217")
            && c.contains("operand-precision-gated-by-consumer-invariance"),
        "interleave concept"
    );
    let c = concept("fixed-point-parity-needs-reference-rounding");
    assert!(
        c.contains("half-level bias")
            && c.contains("(crr * 91881 + 32768) >> 16")
            && c.contains("first CPU-side instance")
            && c.contains("681b9ce")
            && c.contains("exhaustive-domain-input-patch-bit-identity"),
        "fixed-point concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch125_pseudocode_parses() {
    // G1 for Batch 125: syn-parse gate (the ds4 rebase-surge rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "kv-reuse-prefix-ends-at-last-evaluated-token",
        "new-syntax-family-enumerates-every-keyed-surface",
        "interleave-independent-accumulators-only",
        "fixed-point-parity-needs-reference-rounding",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-125 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-125 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-125 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch127_rules_present() {
    // Batch 127 (the PrismML llama.cpp CUDA ternary MMQ mma distill -- Plan
    // 108 / Research 149): 6 kernel_opt rules mined at the fresh pin
    // e311ed38 (MIT; the league opponent's unmined CUDA side -- prior fork
    // contact B96/B113/B116 was Metal/format lanes). The shared vendor
    // checkout is NOT at the pin (pin not an ancestor of its HEAD); the
    // mine used a fresh .raw clone verified HEAD == pin.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "lut-free-bit-spread-vadd4-unpack",
        "per-block-int32-chunk-rescale-fp32-accum",
        "one-time-dense-repack-keyed-sync-publish",
        "sm90a-window-shape-gated-wgmma-dispatch",
        "streamk-starved-tile-grid-dispatch",
        "moe-broadcast-quantize-once-scatter",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-127 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch127_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-127 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("lut-free-bit-spread-vadd4-unpack");
    assert!(
        c.contains("serialize the constant cache")
            && c.contains("~1.7x end-to-end")
            && c.contains("Synthetic uniform test data hides this entirely")
            && c.contains("mmq-hopper-q1.cu:213-215")
            && c.contains("byte-perm-lut-via-permutation-immediate"),
        "lut-free concept"
    );
    let c = concept("per-block-int32-chunk-rescale-fp32-accum");
    assert!(
        c.contains("acc_f32(e) += float(acc_i32(e))")
            && c.contains("clear(acc_i32)")
            && c.contains("family-conditional")
            && c.contains("per-block-quantized")
            && c.contains("operand-precision-gated-by-consumer-invariance"),
        "per-block rescale concept"
    );
    let c = concept("one-time-dense-repack-keyed-sync-publish");
    assert!(
        c.contains("dense bit words + fp32 scales")
            && c.contains("publish")
            && c.contains("cudaStreamSynchronize")
            && c.contains("mmq-hopper-q1.cu:62-63"),
        "dense repack concept"
    );
    let c = concept("sm90a-window-shape-gated-wgmma-dispatch");
    assert!(
        c.contains("cc >= GGML_CUDA_CC_BLACKWELL")
            && c.contains("no GGML_CUDA_CC_HOPPER macro")
            && c.contains("tcgen05")
            && c.contains("streamk-starved-tile-grid-dispatch"),
        "wgmma dispatch concept"
    );
    let c = concept("streamk-starved-tile-grid-dispatch");
    assert!(
        c.contains("ntiles < 8 * nsm")
            && c.contains("fp32-additive atomic flush into zeroed C")
            && c.contains("stream-k-persistent-cta-mac-segments")
            && c.contains("NON-DETERMINISTIC"),
        "streamk starvation concept"
    );
    let c = concept("moe-broadcast-quantize-once-scatter");
    assert!(
        c.contains("ne11 == 1 && n_expert_used > 1")
            && c.contains("inverse map (token slot -> compact row)")
            && c.contains("mmq.cu:194-196"),
        "moe broadcast concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch127_pseudocode_parses() {
    // G1 for Batch 127: syn-parse gate (the PrismML CUDA MMQ rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "lut-free-bit-spread-vadd4-unpack",
        "per-block-int32-chunk-rescale-fp32-accum",
        "one-time-dense-repack-keyed-sync-publish",
        "sm90a-window-shape-gated-wgmma-dispatch",
        "streamk-starved-tile-grid-dispatch",
        "moe-broadcast-quantize-once-scatter",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-127 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-127 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-127 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch128_rules_present() {
    // Batch 128 (the PrismML llama.cpp Metal/FA distill -- Plan 109 /
    // Research 150): 5 kernel_opt rules mined at the fresh tip 5ea87dda
    // (MIT; the same fork Batch 127 mined for the CUDA ternary MMQ mma
    // slice -- this batch takes the unmined Metal/FA surface, the M3
    // box's first-priority domain; quotes re-verified at the tip; the
    // clone removed at landing).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "kv-dequant-materialization-batch-threshold-gate",
        "offline-device-tuned-table-dispatch",
        "tuned-config-resource-limit-fallback",
        "quantize-scale-math-wide-precision-denorm-safe",
        "extent-clamped-operand-views-partial-k-tile",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-128 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch128_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-128 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("kv-dequant-materialization-batch-threshold-gate");
    assert!(
        c.contains("dequant to f16 kv is not always beneficial")
            && c.contains("ne[1] < 32")
            && c.contains("always reserve the temp buffer")
            && c.contains("TODO: tune per device")
            && c.contains("dequant-once-per-k-tile-into-shared-memory"),
        "kv dequant gate concept"
    );
    let c = concept("offline-device-tuned-table-dispatch");
    assert!(
        c.contains("do not hand-edit")
            && c.contains("re-run the sweep and paste its output")
            && c.contains("short KV: attention is a small slice of the step")
            && c.contains("family representative SKU")
            && c.contains("vec 80 -> 133"),
        "offline table dispatch concept"
    );
    let c = concept("tuned-config-resource-limit-fallback");
    assert!(
        c.contains("fall back to baseline (Q=1)")
            && c.contains("max_theadgroup_memory_size")
            && c.contains("never half-clamp")
            && c.contains("smem-envelope-gate-kernel-variant"),
        "resource limit fallback concept"
    );
    let c = concept("quantize-scale-math-wide-precision-denorm-safe");
    assert!(
        c.contains("could be a denorm and 1/qd would overflow")
            && c.contains("const float qd = amax / 127.0f;")
            && c.contains("per-block-int32-chunk-rescale-fp32-accum"),
        "denorm-safe quantize concept"
    );
    let c = concept("extent-clamped-operand-views-partial-k-tile");
    assert!(
        c.contains("min(N_MM_NK_TOTAL, K - loop_k)")
            && c.contains("section 2.22.2")
            && c.contains("Assisted-by: pi:llama.cpp/Qwen3.8-27B")
            && c.contains("pad-tail-to-fixed-tile"),
        "extent clamp concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch128_pseudocode_parses() {
    // G1 for Batch 128: syn-parse gate (the PrismML Metal/FA rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "kv-dequant-materialization-batch-threshold-gate",
        "offline-device-tuned-table-dispatch",
        "tuned-config-resource-limit-fallback",
        "quantize-scale-math-wide-precision-denorm-safe",
        "extent-clamped-operand-views-partial-k-tile",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-128 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-128 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-128 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch129_rules_present() {
    // Batch 129 (the ds4 Qwen Q2 decode-fusion + prefill-structural distill
    // -- Plan 110 / Research 151): 5 kernel_opt rules mined at the fresh
    // pins ds4-metal b85a6174 (branch qwen3.8-flash-next, LINEAR 12-commit
    // delta past the B125 pin) + upstream antirez/ds4 f62ca29a (the
    // b6af0ad SSD seed bounding rode in via the PR's upstream merge and is
    // banked as a rider on the Batch-123 static-seed rule).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "pipeline-cache-key-covers-every-function-constant",
        "block-quant-weight-pad-never-extends-activations",
        "expert-remainder-routed-to-smaller-tile-twins",
        "window-recurrence-halo-snapshot-block-parallel",
        "fused-residual-update-writes-separate-next-buffer",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-129 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch129_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-129 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("pipeline-cache-key-covers-every-function-constant");
    assert!(
        c.contains("SILENT WRONG MATH")
            && c.contains("type >= 40u -> return 0")
            && c.contains("38d3e45")
            && c.contains("b4c3550")
            && c.contains("proven-shape-static-sibling-identical-results-fallback"),
        "cache-key concept"
    );
    let c = concept("block-quant-weight-pad-never-extends-activations");
    assert!(
        c.contains("Activations remain 640 wide; only the weight row stride is padded")
            && c.contains("Nonzero random padding ensures kernels ignore the physical tail")
            && c.contains("5bd8796")
            && c.contains("rejects a partial activation group")
            && c.contains("pad-tail-to-fixed-tile"),
        "weight-pad concept"
    );
    let c = concept("expert-remainder-routed-to-smaller-tile-twins");
    assert!(
        c.contains("saved arithmetic did not compensate for the resulting compiled kernel cost")
            && c.contains("tail_tt = remainder <= 8u ? 8u")
            && c.contains("b4c3550")
            && c.contains("192 teacher-forced full-vocabulary vectors bit-identical")
            && c.contains("batch-width-routed-dual-tile-twins"),
        "tile-twins concept"
    );
    let c = concept("window-recurrence-halo-snapshot-block-parallel");
    assert!(
        c.contains("Only incoming block windows need a snapshot")
            && c.contains("+2.10%")
            && c.contains("b85a617")
            && c.contains("kernel_qwen4_conv_halo")
            && c.contains("chunkwise-parallel-recurrence"),
        "halo-snapshot concept"
    );
    let c = concept("fused-residual-update-writes-separate-next-buffer");
    assert!(
        c.contains("write separate next buffers")
            && c.contains("0.04-0.42% slower")
            && c.contains("82d0314")
            && c.contains("192 full-vocabulary FP32 vectors byte-identical")
            && c.contains("inplace-output-alias"),
        "separate-next-buffer concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch129_pseudocode_parses() {
    // G1 for Batch 129: syn-parse gate (the ds4 Qwen Q2 rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "pipeline-cache-key-covers-every-function-constant",
        "block-quant-weight-pad-never-extends-activations",
        "expert-remainder-routed-to-smaller-tile-twins",
        "window-recurrence-halo-snapshot-block-parallel",
        "fused-residual-update-writes-separate-next-buffer",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-129 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-129 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-129 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch130_rules_present() {
    // Batch 130 (the CloudRift cuBLAS-5090 TMA distill -- Plan 111 /
    // Research 152): 5 kernel_opt rules mined at cloudrift-ai/emmy @
    // 2aaa0328 (Apache-2.0; the blog landing commit #89 -- NOT the
    // DeploDock/Emmy path in the queue lead, which 404s).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "vendor-dispatch-verify-before-outsource",
        "sub-ms-bench-boost-clock-never-engages",
        "lds-cluster-cross-warp-phase-not-static-distance",
        "tma-single-issuer-bulk-tile",
        "multiplexed-grid-axis-partition-modes-exclusive",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-130 rule '{id}' missing from corpus"
        );
    }
}

#[allow(clippy::too_many_lines)] // the B130 phrase-table test: one assertion block per mandated phrase
#[test]
fn batch130_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-130 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("vendor-dispatch-verify-before-outsource");
    assert!(
        c.contains("no escalation at any threshold")
            && c.contains("simt_sgemm_128x32_8x5")
            && c.contains("vendor-fused-op-instantiation-check")
            && c.contains("cublas_loop_vs_strided.cu"),
        "vendor-dispatch concept"
    );
    let c = concept("sub-ms-bench-boost-clock-never-engages");
    assert!(
        c.contains("clock governor never engages boost")
            && c.contains("562->1890 MHz")
            && c.contains("INTERLEAVED for thermal fairness")
            && c.contains("dispatch-cost-sequential-measurement-not-single-op"),
        "sub-ms-bench concept"
    );
    let c = concept("lds-cluster-cross-warp-phase-not-static-distance");
    assert!(
        c.contains("dispatch_stall 44.21% vs 22.36%")
            && c.contains("hypothesis was wrong")
            && c.contains("Generate FMA block: B load first, then interleave A load + FMA per row")
            && c.contains("interleave-loads-between-tile-matmuls"),
        "lds-cluster concept"
    );
    let c = concept("tma-single-issuer-bulk-tile");
    assert!(
        c.contains("cp.async.bulk.tensor.2d")
            && c.contains("mbarrier.arrive.expect_tx")
            && c.contains("TRUNCATES a ragged K tail silently")
            && c.contains("cpasync-multistage-tile-pipeline"),
        "tma-single-issuer concept"
    );
    let c = concept("multiplexed-grid-axis-partition-modes-exclusive");
    assert!(
        c.contains("Both modes consume blockIdx.z")
            && c.contains("BATCH > 1 ? BATCH : k_splits_val")
            && c.contains("target-tile-split-k-deterministic-two-dispatch"),
        "grid-axis-exclusivity concept"
    );
    // Rider pins: the 7 riders this batch appended to existing rules,
    // pinned by their opening marker (the B129 prefix+rider form).
    let corpus_all: Vec<&str> = corpus
        .entries
        .iter()
        .map(|e| e.gpu_concept.as_str())
        .collect();
    let rider_targets = [
        "when kernel params match a known config", // hardcoded-fast-path
        "a numerics-affecting compiler flag",      // fast-math
        "swizzled L2-cache-aware tile mapping",    // l2-grouped
        "loop unrolling for instruction-level parallelism", // unroll-fixed-inner
        "autotune dispatch",                       // autotune-warps-stages
        "Stage global-to-shared tile fetches",     // cpasync-multistage
        "pin the launch heuristic to the architecture", // launch-heuristic (prefix probe below)
    ];
    let _ = rider_targets;
    for (prefix_probe, marker) in [
        (
            "when kernel params match a known config",
            "RIDER (Batch 130 external instance, cloudrift-ai/emmy",
        ),
        (
            "a numerics-affecting compiler flag",
            "RIDER (Batch 130 external instance, cloudrift-ai/emmy",
        ),
        (
            "swizzled L2-cache-aware tile mapping",
            "RIDER (Batch 130 external instance, cloudrift-ai/emmy",
        ),
        (
            "loop unrolling for instruction-level parallelism",
            "RIDER (Batch 130, cloudrift-ai/emmy",
        ),
        ("autotune dispatch", "RIDER (Batch 130, cloudrift-ai/emmy"),
        (
            "Stage global-to-shared tile fetches",
            "RIDER (Batch 130, cloudrift-ai/emmy",
        ),
    ] {
        let concept = corpus_all
            .iter()
            .find(|c| c.starts_with(prefix_probe))
            .unwrap_or_else(|| panic!("rider target concept missing: {prefix_probe}"));
        assert!(
            concept.contains(marker),
            "batch-130 rider missing on: {prefix_probe}"
        );
    }
    // launch-heuristic-pinned-to-calibration-arch: concept opening differs;
    // locate by rule id.
    let lh = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "launch-heuristic-pinned-to-calibration-arch")
        .expect("launch-heuristic rule missing");
    assert!(
        lh.gpu_concept
            .contains("RIDER (Batch 130 external instance, cloudrift-ai/emmy"),
        "batch-130 launch-heuristic rider missing"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch130_pseudocode_parses() {
    // G1 for Batch 130: syn-parse gate (the CloudRift TMA rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "vendor-dispatch-verify-before-outsource",
        "sub-ms-bench-boost-clock-never-engages",
        "lds-cluster-cross-warp-phase-not-static-distance",
        "tma-single-issuer-bulk-tile",
        "multiplexed-grid-axis-partition-modes-exclusive",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-130 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-130 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-130 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch137_rules_present() {
    // Batch 137 (the CloudRift "Modern GPU Matmul Optimization" distill --
    // Plan 116): 11 kernel_opt rules mined at cloudrift-ai/emmy @ 1c058c9d
    // (Apache-2.0; the renamed successor of the Batch-130 deplodock pin)
    // x the matmul-optimization article (Trifonov, 2026-05-29; the queue
    // lead's guessed blog path 404s -- /blog/gpu-matmul-optimization).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "smem-fill-align-chunk-not-element",
        "software-swizzle-shift-row-stride",
        "companion-slab-ring-slot-sync",
        "ptxas-recoalesces-scalar-runs-wash",
        "ptxas-reschedules-load-order-wash",
        "smem-pad-transport-gated",
        "split-k-atomic-arm-legality",
        "setmaxnreg-fit-gate-elected-arrive",
        "splitk-width-shape-need-gate",
        "splitk-atomic-free-contention-threshold",
        "ldmatrix-pair-x2-to-x4-adjacency",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-137 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch137_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-137 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("smem-fill-align-chunk-not-element");
    assert!(
        c.contains("Aligning to the ELEMENT would leave an f16 slab 2 B-aligned")
            && c.contains("_cp_async_width(cols, elem_bytes) * elem_bytes")
            && c.contains("CUDA_ERROR_MISALIGNED_ADDRESS")
            && c.contains("stride-align-vectorization-key"),
        "smem-fill-align concept"
    );
    let c = concept("software-swizzle-shift-row-stride");
    assert!(
        c.contains("7.8-way ldmatrix conflict at D = 128")
            && c.contains("a TMA copy deadlock")
            && c.contains("sm_89 bank-conflict fix")
            && c.contains("xor-swizzle-smem-static-index-tables"),
        "software-swizzle concept"
    );
    let c = concept("companion-slab-ring-slot-sync");
    assert!(
        c.contains("a minority off by a factor of 2-3")
            && c.contains("COMPUTE-FILLED and single-buffer")
            && c.contains("quant-group-scale-fetch-gating"),
        "companion-slab concept"
    );
    let c = concept("ptxas-recoalesces-scalar-runs-wash");
    assert!(
        c.contains("not a search dimension")
            && c.contains("0% difference at every -O level")
            && c.contains("CUDA_ERROR_MISALIGNED_ADDRESS")
            && c.contains("vector-width-pow2-sweep"),
        "ptxas-recoalesces concept"
    );
    let c = concept("ptxas-reschedules-load-order-wash");
    assert!(
        c.contains("ptxas reschedules both forms to nearly the same SASS")
            && c.contains("interleave-loads-between-tile-matmuls")
            && c.contains("interleave-independent-accumulators-only"),
        "ptxas-reschedules concept"
    );
    let c = concept("smem-pad-transport-gated");
    assert!(
        c.contains("Derived, not tuned")
            && c.contains("a conflict-free scalar beats a serialized vector four times over")
            && c.contains("Operand.pad_cols")
            && c.contains("vectorized-smem-write-no-bank-padding"),
        "smem-pad concept"
    );
    let c = concept("split-k-atomic-arm-legality");
    assert!(
        c.contains("a max carrier's partials would be SUMMED, silently wrong")
            && c.contains("Storage is asked BEFORE the projection")
            && c.contains("never a crash")
            && c.contains("target-tile-split-k-deterministic-two-dispatch"),
        "split-k-legality concept"
    );
    let c = concept("setmaxnreg-fit-gate-elected-arrive");
    assert!(
        c.contains("provably fits the 64K-register file")
            && c.contains("is UB on the divergent role branch")
            && c.contains("fence.proxy.async")
            && c.contains("plane-specialization-load-compute-role-split"),
        "setmaxnreg concept"
    );
    let c = concept("splitk-width-shape-need-gate");
    assert!(
        c.contains("ranked the winning g8k golden ~9000-deep")
            && c.contains("pure atomic/combine waste")
            && c.contains("streamk-starved-tile-grid-dispatch"),
        "splitk-width concept"
    );
    let c = concept("splitk-atomic-free-contention-threshold");
    assert!(
        c.contains("above threshold splits REWARD the deferred combine kernel")
            && c.contains("a hand-set constant here would be a constant the fit optimizes around")
            && c.contains("D_finalize_kernel goes dead")
            && c.contains("target-tile-split-k-deterministic-two-dispatch"),
        "splitk-contention concept"
    );
    let c = concept("ldmatrix-pair-x2-to-x4-adjacency");
    assert!(
        c.contains("bit-identical to its unpaired sibling")
            && c.contains("a cooperative gather, not an ldmatrix")
            && c.contains("Reading the KEPT fragment is fine")
            && c.contains("ldmatrix-mma-layout-loads"),
        "ldmatrix-pair concept"
    );
}

#[test]
#[cfg(not(feature = "kernel_verify"))]
fn batch137_pseudocode_parses() {
    // G1 for Batch 137: syn-parse gate (the CloudRift matmul rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "smem-fill-align-chunk-not-element",
        "software-swizzle-shift-row-stride",
        "companion-slab-ring-slot-sync",
        "ptxas-recoalesces-scalar-runs-wash",
        "ptxas-reschedules-load-order-wash",
        "smem-pad-transport-gated",
        "split-k-atomic-arm-legality",
        "setmaxnreg-fit-gate-elected-arrive",
        "splitk-width-shape-need-gate",
        "splitk-atomic-free-contention-threshold",
        "ldmatrix-pair-x2-to-x4-adjacency",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-137 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-137 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-137 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch138_rules_present() {
    // Batch 138 (the CloudRift "Learning FlashAttention the Hard Way"
    // Parts 1+2 distill -- Plan 118): 10 kernel_opt rules mined at
    // cloudrift-ai/emmy @ bdd26c53 (Apache-2.0; the tip past Batch 137's
    // 1c058c9d, carrying #758) x the FA articles (Trifonov, 2026-07-06/08;
    // the queue's bare blog path 404s -- the real paths carry -part-1/-part-2).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "split-copy-issue-deposit-depth-uniform",
        "stage-domain-product-of-independent-knobs",
        "fold-invariant-factor-commutes-to-epilogue",
        "twisted-monoid-dependent-reduce-fusion",
        "c-fragment-to-a-operand-register-repack",
        "stream-invariant-operand-skips-staging-pipeline",
        "rewrite-decline-names-refusing-predicate",
        "defensive-sync-peephole-tile-body-scope",
        "sfu-exp-swap-pays-only-when-sfu-limited",
        "left-right-fold-pair-proves-associative-combine",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-138 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // the B51-style phrase pins, one block per rule
fn batch138_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-138 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("split-copy-issue-deposit-depth-uniform");
    assert!(
        c.contains("one meaning of the knob across all three transports")
            && c.contains("the slot a deposit writes was freed two chunks back")
            && c.contains("896 CTAs onto 80 SMs")
            && c.contains("register-double-buffer-operand-pipeline"),
        "split-copy concept"
    );
    let c = concept("stage-domain-product-of-independent-knobs");
    assert!(
        c.contains("the domain is their PRODUCT")
            && c.contains("Nothing here picks which pairings are worth trying")
            && c.contains("reached only four of its combinations")
            && c.contains("autotune-warps-stages"),
        "stage-domain concept"
    );
    let c = concept("fold-invariant-factor-commutes-to-epilogue");
    assert!(
        c.contains("commutes out of an additive fold")
            && c.contains("Divisors hoist as divisions; a varying divisor keeps the fold whole")
            && c.contains("once, after the fold")
            && c.contains("factor-affine-quant-scale-out-of-element-loop"),
        "fold-invariant concept"
    );
    let c = concept("twisted-monoid-dependent-reduce-fusion");
    assert!(
        c.contains("TWISTED MONOID stated as its mathematics")
            && c.contains("evaluating the two apart denotes")
            && c.contains("a mean of nothing is any value")
            && c.contains("online-softmax-streaming"),
        "twisted-monoid concept"
    );
    let c = concept("c-fragment-to-a-operand-register-repack");
    assert!(
        c.contains("lane-aligned and converts two adjacent C fragments directly")
            && c.contains("Neither needs a shared-memory round trip")
            && c.contains("cvt.rn.f16x2")
            && c.contains("mma-fragment-layout-m16n8k16"),
        "c-to-a-repack concept"
    );
    let c = concept("stream-invariant-operand-skips-staging-pipeline");
    assert!(
        c.contains("copied ONCE ahead of the loop and never refilled")
            && c.contains("with nothing advancing there is no live range")
            && c.contains("disabled by default")
            && c.contains("cpasync-multistage-tile-pipeline"),
        "stream-invariant concept"
    );
    let c = concept("rewrite-decline-names-refusing-predicate");
    assert!(
        c.contains("Declining is not neutral")
            && c.contains("the only visible symptom is the measurement")
            && c.contains("names the predicate that refused instead of leaving a silent demotion")
            && c.contains("warm-reachable-dispatch-signatures-before-timed-window"),
        "named-decline concept"
    );
    let c = concept("defensive-sync-peephole-tile-body-scope");
    assert!(
        c.contains("before any smem access fences nothing")
            && c.contains("load-bearing")
            && c.contains("one barrier per chunk instead of two")
            && c.contains("barrier-after-async-mma-before-staged-tile-reuse"),
        "defensive-sync concept"
    );
    let c = concept("sfu-exp-swap-pays-only-when-sfu-limited");
    assert!(
        c.contains("MUFU.EX2")
            && c.contains("not bit-exact")
            && c.contains("NEVER silently on")
            && c.contains("exp2-log2-base2"),
        "sfu-exp concept"
    );
    let c = concept("left-right-fold-pair-proves-associative-combine");
    assert!(
        c.contains("guaranteed to exist")
            && c.contains("a test for when any loop is secretly associative")
            && c.contains("either a recipe clicks or it does not")
            && c.contains("first-order-affine-recurrence-associative-scan"),
        "fold-pair concept"
    );
    // The four Batch-138 riders must be present on their host rules.
    let host = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("rider host '{id}' missing"))
            .gpu_concept
            .clone()
    };
    assert!(
        host("setmaxnreg-fit-gate-elected-arrive")
            .contains("ASYNC-MMA half of the admission predicate")
            && host("setmaxnreg-fit-gate-elected-arrive").contains("no wgmma"),
        "setmaxnreg rider"
    );
    assert!(
        host("autotune-warps-stages")
            .contains("occupancy loss beats extra overlap at every geometry"),
        "deeper-rings rider"
    );
    assert!(
        host("online-softmax-streaming").contains("never amplifying"),
        "streaming-license rider"
    );
    assert!(
        host("state-normalization-between-updates-breaks-associative-scan")
            .contains("many-to-one, hence not invertible"),
        "quantize-breaks rider"
    );
}

#[test]
fn batch138_pseudocode_parses() {
    // G1 for Batch 138: syn-parse gate (the CloudRift FA rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "split-copy-issue-deposit-depth-uniform",
        "stage-domain-product-of-independent-knobs",
        "fold-invariant-factor-commutes-to-epilogue",
        "twisted-monoid-dependent-reduce-fusion",
        "c-fragment-to-a-operand-register-repack",
        "stream-invariant-operand-skips-staging-pipeline",
        "rewrite-decline-names-refusing-predicate",
        "defensive-sync-peephole-tile-body-scope",
        "sfu-exp-swap-pays-only-when-sfu-limited",
        "left-right-fold-pair-proves-associative-combine",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-138 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-138 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-138 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch139_rules_present() {
    // Batch 139 (the ds4-metal PR #5 distill -- Plan 119 / Research 154):
    // 4 kernel_opt rules mined at ivanfioravanti/ds4-metal branch
    // qwen3.8-flash-next @ 18ca8ec (MIT; the merged PR #5 + 6 post-merge
    // maintainer commits, delta past Batch 129's b85a6174 pin = 19 commits).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "expert-major-tile-order-token-tile-cache-resident",
        "narrow-matvec-one-row-per-simdgroup",
        "tile-max-prefiltered-exact-topk",
        "sustained-load-drift-interleave-protocol",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-139 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // the B51-style phrase pins, one block per rule
fn batch139_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-139 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("expert-major-tile-order-token-tile-cache-resident");
    assert!(
        c.contains("pure PERMUTATION of threadgroup execution")
            && c.contains("bit-exact BY CONSTRUCTION")
            && c.contains("cache-resident across ALL of that expert's tiles")
            && c.contains("l2-grouped-tile-schedule"),
        "expert-major concept"
    );
    let c = concept("narrow-matvec-one-row-per-simdgroup");
    assert!(
        c.contains("rows <= 1024")
            && c.contains("per-row arithmetic is UNCHANGED")
            && c.contains("ORDERED override chain")
            && c.contains("row-parallelism-ladder-split-vs-pack"),
        "narrow-matvec concept"
    );
    let c = concept("tile-max-prefiltered-exact-topk");
    assert!(
        c.contains("LOWER BOUND on the k-th block score")
            && c.contains("BYTE-IDENTICAL to the full-row selection")
            && c.contains("must CONJOIN the producer's enable predicate")
            && c.contains("0x7f7fffff")
            && c.contains("running-threshold-prune-compaction-aliasing-guard"),
        "tile-max concept"
    );
    let c = concept("sustained-load-drift-interleave-protocol");
    assert!(
        c.contains("1030 -> 910 -> 870 -> 850")
            && c.contains("ABBA/BAAB")
            && c.contains("walk the whole prefix/context UNTIMED")
            && c.contains("deltas under ~2% are not treated as evidence")
            && c.contains("settings cached when the graph is allocated look identical here")
            && c.contains("sub-ms-bench-boost-clock-never-engages"),
        "drift-protocol concept"
    );
    // The eight Batch-139 riders must be present on their host rules.
    let host = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("rider host '{id}' missing"))
            .gpu_concept
            .clone()
    };
    assert!(
        host("output-buffer-rotation-pool-multi-forward")
            .contains("COMPLETE-OVERWRITE SNAPSHOT + SWAP-ADOPTION")
            && host("write-before-read-state-skips-checkpoint").contains(
                "the snapshot SET here is exactly this rule's read-modify classification"
            ),
        "swap-restore riders"
    );
    assert!(
        host("pipeline-cache-key-covers-every-function-constant")
            .contains("the DETECTION half of this defect class"),
        "stale-source rider"
    );
    assert!(
        host("cpasync-multistage-tile-pipeline")
            .contains("the METAL TENSOROPS double-buffer instance"),
        "mpp-dbuf rider"
    );
    assert!(
        host("fast-math-contraction-behind-feature-flag")
            .contains("a PURE SOURCE RESTRUCTURE can silently flip contraction"),
        "contraction rider"
    );
    assert!(
        host("kv-dequant-materialization-batch-threshold-gate").contains("the PREFILL CHUNK WIDTH"),
        "chunk-width rider"
    );
    assert!(
        host("swept-shape-bucket-launch-tables").contains("the SINGLE-ROW launch-table entry"),
        "gdn-threads rider"
    );
    assert!(
        host("multi-simdgroup-shared-staged-tile").contains("the SMALL-grain instance"),
        "scorer-staging rider"
    );
    assert!(
        host("register-rotation-prefetch-loop")
            .contains("the FIXED-WIDTH BATCH form on a DEPENDENT chain"),
        "merge-prefetch rider"
    );
}

#[test]
fn batch139_pseudocode_parses() {
    // G1 for Batch 139: syn-parse gate (the ds4-metal PR #5 rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "expert-major-tile-order-token-tile-cache-resident",
        "narrow-matvec-one-row-per-simdgroup",
        "tile-max-prefiltered-exact-topk",
        "sustained-load-drift-interleave-protocol",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-139 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-139 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-139 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch142_rules_present() {
    // Batch 142 (the CloudRift Gemma4-12B distill -- Plan 122 / Research 163):
    // 2 kernel_opt rules mined at cloudrift-ai/emmy @ 321288d8 (Apache-2.0;
    // HEAD == ls-remote verified at mine; #761 e0d16cf0 read before quoting).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "fast-accumulate-periodic-promote-bounded-error",
        "smem-budget-single-buffer-refill-at-kill-points",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-142 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // the B51-style phrase pins, one block per rule
fn batch142_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-142 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    let c = concept("fast-accumulate-periodic-promote-bounded-error");
    assert!(
        c.contains("HALF the FP16-accumulate rate")
            && c.contains("330.3 vs 165.2")
            && c.contains("PROMOTE_CADENCE = 64 K-elements")
            && c.contains("f32 shadow accumulator")
            && c.contains("FLAT ~3.3e-4 at every K")
            && c.contains("6.0e-4 -> 6.6e-3")
            && c.contains("CONSTANT in K")
            && c.contains("allow_fp16_accumulation")
            && c.contains("per-block-int32-chunk-rescale-fp32-accum"),
        "fast-accumulate concept"
    );
    let c = concept("smem-budget-single-buffer-refill-at-kill-points");
    assert!(
        c.contains("96 KB against a ~100 KB per-CTA budget")
            && c.contains("min(depth, budget // slot_bytes)")
            && c.contains("refills chunk i+1 at its kill point")
            && c.contains("single-buffer degenerate")
            && c.contains("re-fetched, never waited")
            && c.contains("CTA-uniform"),
        "smem-budget concept"
    );
    // The six Batch-142 riders must be present on their host rules.
    let host = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("rider host '{id}' missing"))
            .gpu_concept
            .clone()
    };
    assert!(
        host("merge-dispatches-into-single-compute-pass")
            .contains("~1100 launches/step vs stock ~650"),
        "launch-count rider"
    );
    assert!(
        host("fusion-payback-gated-on-per-dispatch-overhead")
            .contains("PER-REALIZED-FORM and fail-closed"),
        "fusion-admission rider"
    );
    assert!(
        host("split-k-two-pass-attention-decode-long-context").contains("OCCUPANCY patch"),
        "splitkv-trigger rider"
    );
    assert!(
        host("vendor-dispatch-verify-before-outsource")
            .contains("forward-ported Ampere-era cutlass_80_* kernels")
            && host("vendor-dispatch-verify-before-outsource")
                .contains("no longer reproduces at bdd26c539"),
        "vendor-lag + stale-blocker rider"
    );
    assert!(
        concept("fast-accumulate-periodic-promote-bounded-error").contains("ENUMERATION knob"),
        "enumeration-knob rider"
    );
    assert!(
        concept("fast-accumulate-periodic-promote-bounded-error")
            .contains("exact/lossy flag SPLIT"),
        "exact-lossy-split rider"
    );
}

#[test]
fn batch142_pseudocode_parses() {
    // G1 for Batch 142: syn-parse gate (the Gemma4-12B rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "fast-accumulate-periodic-promote-bounded-error",
        "smem-budget-single-buffer-refill-at-kill-points",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-142 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-142 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-142 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch144_rules_present() {
    // Batch 144 (the measured-GOAT self-distill of the riir-ai 4090
    // prefill-mma campaign, Benches 878-884 -- Plan 129 / Research 169):
    // 5 kernel_opt rules, sources are our own league-lane records
    // (read-only citations). Two independent quote lanes + caller
    // spot-check; the intake's drifts corrected at admission.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "packed-code-cpasync-stage-per-use-prmt-decode",
        "ldmatrix-byte-subword-granularity-limit",
        "transposed-smem-stage-repairs-strided-epilogue",
        "per-block-scalar-slab-cpasync-stage",
        "profile-sol-table-before-compute-rung",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-144 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // the B51-style phrase pins, one block per rule
fn batch144_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-144 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // (e) packed-code staging: both denominators, the resident-vs-payload
    // distinction, the ONE-barrier claim, the negative arms with their
    // record attributions, and the L2-SOL cap.
    let c = concept("packed-code-cpasync-stage-per-use-prmt-decode");
    for phrase in [
        "1.222x",
        "1.062x",
        "6,144 B",
        "4,096 B",
        "ONE barrier per group",
        "0.989x",
        "0.785x",
        "1.175x",
        "1.069x",
        "B879",
        "89.8%",
        "RIIR_PREFILL_MMQ_FMT=2",
        "Distinct from `cpasync-multistage-tile-pipeline`",
        "0x7632",
    ] {
        assert!(c.contains(phrase), "packed-code concept missing '{phrase}'");
    }
    // (f) ldmatrix granularity limit: the complement framing + the
    // positive rule's measured win.
    let c = concept("ldmatrix-byte-subword-granularity-limit");
    for phrase in [
        "WHOLE",
        "quad-broadcast",
        "+3.8%",
        "1.156x",
        "Distinct from `ldmatrix-mma-layout-loads`",
        "1.175x",
    ] {
        assert!(c.contains(phrase), "ldmatrix-limit concept missing '{phrase}'");
    }
    // (g) strided epilogue: the exact ratio, the stage geometry, the
    // reader map, the row pitch, and the write-side distinct-from.
    let c = concept("transposed-smem-stage-repairs-strided-epilogue");
    for phrase in [
        "8.01x",
        "[TOKS][33]",
        "r = idx & 31",
        "69,632 B",
        "4,456,461",
        "1.533x",
        "Distinct from `tile-amplification-precedes-staging-pass`",
        "inherited v8's schedule",
    ] {
        assert!(c.contains(phrase), "strided-epilogue concept missing '{phrase}'");
    }
    // (h) scalar staging: the honest cap (read sectors UNMOVED), the
    // combined-IPC framing, the per-group-per-block contract.
    let c = concept("per-block-scalar-slab-cpasync-stage");
    for phrase in [
        "84.4M",
        "IPC 1.13 -> 1.82",
        "NOT a bandwidth rule",
        "ONCE per group per BLOCK",
        "0.7 GB",
        "Distinct from `cpasync-multistage-tile-pipeline`",
    ] {
        assert!(c.contains(phrase), "scalar-slab concept missing '{phrase}'");
    }
    // (i) profile-first methodology: the measured SOL range, the
    // first-ncu fact, the minimum-routing-evidence framing.
    let c = concept("profile-sol-table-before-compute-rung");
    for phrase in [
        "69.9%",
        "89.8%",
        "74.7%",
        "first ncu data for the family",
        "MINIMUM routing evidence",
        "kernel bugs, not physics",
        "from `cross-run-profiles-key-on-content-not-stat-triplets`",
    ] {
        assert!(c.contains(phrase), "profile-first concept missing '{phrase}'");
    }
}

#[test]
fn batch144_pseudocode_parses() {
    // G1 for Batch 144: syn-parse gate (the B878-884 self-distill rules).
    // The B122/B123 lesson: syn parses WHOLE FILES, so every snippet is
    // fn-wrapped Rust-shaped code, not bare statements.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "packed-code-cpasync-stage-per-use-prmt-decode",
        "ldmatrix-byte-subword-granularity-limit",
        "transposed-smem-stage-repairs-strided-epilogue",
        "per-block-scalar-slab-cpasync-stage",
        "profile-sol-table-before-compute-rung",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-144 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-144 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-144 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch145_rules_present() {
    // Batch 145 (the Metal-shape pack -- Plan 130 / Research 171):
    // 2 kernel_opt rules (EFQ paper lane + MTPLX PR #391 lane) + 1 rust_perf
    // rule (mincore-probe routing, rust_perf tests) + 2 riders. The charter's
    // own split-K dedup verdict was CORRECTED at admission: the Batch-105
    // dense two-pass rule exists; this batch's rule is scoped to the delta.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "direct-low-bit-generation-replaces-exp-quantize-roundtrip",
        "kv-split-decode-underfill-per-slot-validity",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-145 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // the B51-style phrase pins, one block per rule
fn batch145_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-145 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // EFQ: the direct-generation mapping, the numerator/denominator
    // consistency, the honest scope of the -40.33% figure, the quality
    // framing, and the mandated dedup boundary.
    let c = concept("direct-low-bit-generation-replaces-exp-quantize-roundtrip");
    for phrase in [
        "40.33%",
        "vector-stage latency",
        "A5 vector unit",
        "LUT",
        "0.005 of the FP16 baseline",
        "0.6749",
        "0.6773",
        "Distinct from `operand-precision-gated-by-consumer-invariance`",
        "precision selection vs intermediate elimination",
        "SAME (code, scale) operand",
        "E2M1",
        "row-level exp retained",
        "scalar nearest-rounding optimality",
    ] {
        assert!(c.contains(phrase), "efq concept missing '{phrase}'");
    }
    // KV-split: the underfill arithmetic, the dual-source negative cell,
    // the per-slot validity block, the no-empty-split clamp, and the
    // mandated distinct-from on the Batch-105 rule.
    let c = concept("kv-split-decode-underfill-per-slot-validity");
    for phrase in [
        "EIGHT threadgroups",
        "13.2 tok/s",
        "67.8 control",
        "2609.12923",
        "64-row fragments",
        "PER SLOT",
        "complete_blocks",
        "prefix cut",
        "double-count",
        "no-empty-split clamp",
        "ONE encoder",
        "Distinct from `split-k-two-pass-attention-decode-long-context`",
        "WIDTH UNDERFILL",
    ] {
        assert!(c.contains(phrase), "kv-split concept missing '{phrase}'");
    }
}

#[test]
fn batch145_pseudocode_parses() {
    // G1 for Batch 145: syn-parse gate (fn-wrapped Rust-shaped code only --
    // the B122/B123 lesson).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "direct-low-bit-generation-replaces-exp-quantize-roundtrip",
        "kv-split-decode-underfill-per-slot-validity",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-145 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-145 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-145 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch148_pseudocode_parses() {
    // G1 for Batch 148 (Plan 133 / Research 174 -- the Bakhshaliyev sm_89
    // FP8 GEMM series): syn-parse gate (fn-wrapped Rust-shaped code only --
    // the B122/B123 lesson) + presence + the 5 rider surfaces.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "occupancy-before-async-consumer-ada",
        "hw-barrier-beats-mbarrier-polling",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-148 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-148 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-148 opt_pattern for '{id}' must parse"
        );
    }
    // The 5 riders must be present on their rules (B9 x4 + B15 x1).
    let rider_marks = [
        (
            "xor-swizzle-smem-static-index-tables",
            "OCCUPANCY-COMPOUNDING face",
        ),
        (
            "ldmatrix-mma-layout-loads",
            "the sm_89 LOSER cell",
        ),
        (
            "mma-fragment-layout-m16n8k16",
            "QMMA.16832.F16.E4M3.E4M3 SASS reads the A-fragment register pair SWAPPED",
        ),
        (
            "cpasync-multistage-tile-pipeline",
            "OCCUPANCY-STARVED vs HIGH-OCCUPANCY regime split",
        ),
        (
            "stream-k-persistent-cta-mac-segments",
            "COMPUTE-BOUND sm_89 loser cell",
        ),
    ];
    for (rid, mark) in rider_marks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == rid)
            .unwrap_or_else(|| panic!("batch-148 rider host '{rid}' missing"));
        assert!(
            entry.gpu_concept.contains(mark),
            "batch-148 rider on '{rid}' missing its mark text"
        );
        assert!(
            entry.gpu_concept.contains("Research 174"),
            "batch-148 rider on '{rid}' missing its Research 174 provenance"
        );
    }
}

#[test]
fn batch149_rules_present() {
    // Batch 149 (the sm_89 spill-law / nodecode-probe STANDALONE pack --
    // Plan 135 / Research 175): 3 kernel_opt rules -- the measured LAWS of
    // the riir-ai Plan-597 close (Benches 895+933) as independent rules,
    // re-issued after the Issue-665 dual-allocation yield against Batch 148
    // (the sibling landing embedded these laws' evidence as route-split
    // regime content; the standalone forms serve the retrievals the
    // integrator cannot answer) + 1 rider on setmaxnreg-fit-gate.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "spill-fragility-law-accumulator-regcap-cliff",
        "nodecode-arm-bounds-overhead-elimination",
        "tile-shrink-occupancy-pays-feed-economics",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-149 rule '{id}' missing from corpus"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)] // the B51-style phrase pins, one block per rule
fn batch149_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-149 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // Spill law: the four measured cliff cells, the envelope arithmetic,
    // both escape hatches, the reject-before-benchmark discipline, the
    // distinct-from on the Batch-148 integrator.
    let c = concept("spill-fragility-law-accumulator-regcap-cliff");
    for phrase in [
        "104 B -> 0.550x",
        "192 B -> 0.328x",
        "56 B -> 0.516x",
        "104 B -> 0.340x",
        "dh[2][4][4] i32 + o[2][4][4] f32",
        "65,536",
        "255 regs/thread",
        "spill bytes sit in the INNERMOST k-group loop",
        "local_size_bytes()",
        "Distinct from occupancy-before-async-consumer-ada",
        "from m-group-shared-weight-scan-register-relief (B110",
    ] {
        assert!(c.contains(phrase), "spill-law concept missing '{phrase}'");
    }
    // Nodecode probe: the share cells, the single-vs-single correction,
    // the 64.6% ceiling, the three measurement traps.
    let c = concept("nodecode-arm-bounds-overhead-elimination");
    for phrase in [
        "7.3%",
        "8.4%",
        "+10.7%",
        "64.6%",
        "427 TF",
        "VALUES GARBAGE BY DESIGN",
        "SINGLE-vs-SINGLE",
        "scheduler stretches differently",
        "from work-scaling-probe-launch-bound (B82",
    ] {
        assert!(c.contains(phrase), "nodecode concept missing '{phrase}'");
    }
    // Tile-shrink: the v12 cells, the feed mechanism, the local optimum,
    // the two reopen triggers.
    let c = concept("tile-shrink-occupancy-pays-feed-economics");
    for phrase in [
        "0.648x",
        "0.656x",
        "75 regs",
        "25,088 B",
        "cannot pay for their own feed",
        "LOCAL OPTIMUM",
        "sm_90+",
        "Machete",
        "from spill-fragility-law-accumulator-regcap-cliff (Batch 149 companion",
    ] {
        assert!(c.contains(phrase), "tile-shrink concept missing '{phrase}'");
    }
}

#[test]
fn batch149_pseudocode_parses() {
    // G1 for Batch 149: syn-parse gate (fn-wrapped Rust-shaped code only --
    // the B122/B123 lesson).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "spill-fragility-law-accumulator-regcap-cliff",
        "nodecode-arm-bounds-overhead-elimination",
        "tile-shrink-occupancy-pays-feed-economics",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-149 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-149 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-149 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch166_rules_present() {
    // Batch 166 (the FWHT butterfly staging ladder -- Plan 157 / Research
    // 194): 1 kernel_opt rule, admitted only after the benchable instance
    // landed in-tree (riir-ai Plan 600 / Issue 980's folded-prefill
    // rotation lane, bonsai2_hadamard default-on 2026-09-19) -- the second
    // butterfly-transform instance the 09-17 "STAGE, don't admit" grading
    // waited for.
    let corpus = build_kernel_seed_corpus();
    let ids = ["fwht-butterfly-staging-ladder-warp-smem-register"];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-166 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch166_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-166 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // The staging ladder: the three bands, the incumbent defect shape,
    // the fork evidence (expectation-priced), the sign-fold note, the
    // bit-identity gate, the fused-family contract, the in-repo
    // consumers, the W>2048 rider, the Batch-96 distinctness.
    let c = concept("fwht-butterfly-staging-ladder-warp-smem-register");
    for phrase in [
        "WARP SHUFFLES",
        "SHARED MEMORY with one barrier per stage",
        "REGISTER-RESIDENT",
        "256 threads, W = 1024, __shared__ float smem[1024]",
        "~20 __syncthreads",
        "+17% / +11% decode",
        "25.8% of GPU time at block 4096",
        "+2.2-4.8% pp",
        "sign-first smem load",
        "riir-ai d07542ee3",
        "butterfly: fwht_rotate_forward_f32 verbatim",
        "fwht_rotate_forward_f32 / fwht_rotate_inverse_f32",
        "bonsai2_hadamard is DEFAULT-ON",
        "6f1c6908d",
        "Distinct from fwht-shuffle-xor-butterfly-register-resident (Batch 96",
        "1bc46a8a7",
    ] {
        assert!(c.contains(phrase), "staging-ladder concept missing '{phrase}'");
    }
}

#[test]
fn batch166_pseudocode_parses() {
    // G1 for Batch 166: syn-parse gate (fn-wrapped Rust-shaped code only --
    // the B122/B123 lesson).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["fwht-butterfly-staging-ladder-warp-smem-register"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-166 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-166 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-166 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch170_rules_present() {
    // Batch 170 (the mlxfast-gemma4 successor-arena pack -- Plan 162 /
    // Research 198): 7 kernel_opt rules from the promoted diff-history of
    // Layr-Labs/mlxfast-gemma4-26b-a4b-engine @ 27c821c4 (MIT; the qwen38
    // harness family's successor arena, 180 promoted submissions; scoring
    // prefill^0.25 x decode^0.75 with a 10% token-tolerance budget -- the
    // deliberate opposite of qwen38's fidelity gate). All quotes
    // re-verified verbatim at the pin at authoring time; every entry
    // NotBenchable decision content (Metal source -- the R122/B100
    // precedent; M5 Max numbers are expectation pricing, never ours).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "softmax-stats-precompute-exp-in-gemm-loader",
        "fixed-ref-softmax-decode-walk",
        "scale-identity-elide-in-framework-graph-transcription",
        "sentinel-operand-encoding-routes-fused-kernel",
        "tier-mismatched-grid-pruning-relaunch",
        "compile-out-rare-paths-register-worst-case",
        "batch-width-aware-partition-shrink",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-170 rule '{id}' missing from corpus"
        );
    }
}

#[test]
fn batch170_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let concept = |id: &str| -> String {
        corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-170 rule '{id}' missing"))
            .gpu_concept
            .clone()
    };
    // at1: stats precompute + exp-in-loader, the 4->2 pass census, the
    // bit-transport contract, the streaming distinction.
    let c = concept("softmax-stats-precompute-exp-in-gemm-loader");
    for phrase in [
        "8 bytes/row against 2*kL bytes/row",
        "four passes over the score rectangle, ~17.5 GB per ranked prefill",
        "transported bit for bit",
        "Distinct from online-softmax-streaming",
    ] {
        assert!(c.contains(phrase), "at1 concept missing '{phrase}'");
    }
    // fixed-ref: the prologue reference, the ceiling, the underflow
    // argument, the span-0 control.
    let c = concept("fixed-ref-softmax-decode-walk");
    for phrase in [
        "the query's self-score",
        "min(..., 60.0f)",
        "~87 nats under the reference underflow to zero",
        "BIT-EXACT CONTROL",
    ] {
        assert!(c.contains(phrase), "fixed-ref concept missing '{phrase}'");
    }
    // qscale-elide: the census, the bit-exactness argument, the
    // order-invariance neighbor distinction.
    let c = concept("scale-identity-elide-in-framework-graph-transcription");
    for phrase in [
        "232 dispatches and 1.11 x 10^9 bf16 elements",
        "~4.4 GB of traffic",
        "sNaN quieting",
        "Distinct from monotone-transform-elision-under-argmax",
    ] {
        assert!(c.contains(phrase), "qscale-elide concept missing '{phrase}'");
    }
    // sentinel: the stride signature, the addmm combination, the
    // degradation property.
    let c = concept("sentinel-operand-encoding-routes-fused-kernel");
    for phrase in [
        "(kL -> kL + 1)",
        "SYNTHESIZE this operand in its epilogue",
        "not a combination any other addmm of the declared shape produces",
        "GRACEFUL DEGRADATION",
    ] {
        assert!(c.contains(phrase), "sentinel concept missing '{phrase}'");
    }
    // tight-grid: the 262144/196608 census, the same-pointers claim, the
    // box receipt, the twin census.
    let c = concept("tier-mismatched-grid-pruning-relaunch");
    for phrase in [
        "196608 of them exist only to hit that early return",
        "the same threadgroup does the same rows with the same pointers",
        "+0.216% composite / +0.383% decode",
        "211,200 threadgroups; 158,400 of them are guaranteed early returns",
    ] {
        assert!(c.contains(phrase), "tight-grid concept missing '{phrase}'");
    }
    // run-cap: the RUN-OCT proof, the 14%/86% distribution, the measured
    // cap, the bit-identity clause.
    let c = concept("compile-out-rare-paths-register-worst-case");
    for phrase in [
        "merely compiling an eight-stream path cost 9.1% even when unused",
        "runs of three or more are only 14% of runs",
        "cap4 (incumbent) 0.002927 s/token, cap2 0.002660 = +9.12%",
        "only an already-unreachable branch is removed",
    ] {
        assert!(c.contains(phrase), "run-cap concept missing '{phrase}'");
    }
    // partition: the 8x over-partition, the DRAM in-flight argument, the
    // dead-lanes census, the attribution.
    let c = concept("batch-width-aware-partition-shrink");
    for phrase in [
        "over-partitions by 8x",
        "roughly 1 MB is in flight against the ~225 KB a 450 GB/s part",
        "eight live lanes and twenty-four dead ones",
        "samfenwick",
    ] {
        assert!(c.contains(phrase), "partition concept missing '{phrase}'");
    }
}

#[test]
fn batch170_pseudocode_parses() {
    // G1 for Batch 170: syn-parse gate (fn-wrapped Rust-shaped code only --
    // the B122/B123 lesson).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "softmax-stats-precompute-exp-in-gemm-loader",
        "fixed-ref-softmax-decode-walk",
        "scale-identity-elide-in-framework-graph-transcription",
        "sentinel-operand-encoding-routes-fused-kernel",
        "tier-mismatched-grid-pruning-relaunch",
        "compile-out-rare-paths-register-worst-case",
        "batch-width-aware-partition-shrink",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-170 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-170 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-170 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch177_rules_present() {
    // Batch 177 (the EXL3/GB10 serving-recipe distill -- Plan 170 /
    // Research 207): 4 kernel_opt rules (N +2: the residency audit + the
    // bpw-axis rule; D +2: the graph-split separable price + the
    // unified-memory page-cache rule) + the readout-trim scope rider on
    // entries_gpu_26, distilled from
    // vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe @ e0ebee8d (MIT;
    // provenance three-deep: vcruz305/exllamav3 PR #2 -> turboderp-org/
    // exllamav3 -> QTIP arXiv:2406.11235). Executed AHEAD of the
    // event-blocked B176 FWHT rider (#29096 frozen through the 35th
    // check). The T2 re-verification caught the verdict's phrase-grep miss
    // (factor-affine-quant-scale-out-of-element-loop is candidate #2's
    // loop-stage half), so #2 lands as ONE rule with the placement as its
    // fix-regime clause. All quotes re-verified verbatim at the pin THIS
    // batch; GB10 numbers are source-reported, never ours.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "quantized-pack-residency-audit-unquantized-minority-first",
        "bpw-reduction-buys-memory-not-decode-when-dequant-bound",
        "host-gather-splits-the-graph-and-the-split-is-the-separable-price",
        "unified-memory-page-cache-is-device-allocatable",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-177 rule '{id}' missing from corpus"
        );
    }
    // The rider: the scope correction must be IN the landed rule text.
    let trim = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "readout-trim-needs-committed-id-coverage")
        .expect("readout-trim rule present");
    assert!(
        trim.gpu_concept.contains("SCOPE CORRECTION (Batch 177 rider"),
        "batch-177 readout-trim scope rider missing"
    );
    assert!(trim.gpu_concept.contains("becomes a rejection, not an error"));
}

#[test]
fn batch177_concepts_covered() {
    // Distinctness + mandated phrases must survive in the concept text
    // (case-sensitive contains -- the B51 lesson).
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "quantized-pack-residency-audit-unquantized-minority-first",
            &[
                "Run this first on any new pack",
                "CONSTANT ALONG THAT TENSOR'S CONTRACTED DIM",
                "sum_k s[k] * (q[k] * scale) == scale * sum_k s[k] * q[k]",
                "ONLY THE WEIGHT LOAD CHANGES",
                "int4 sim COLLAPSED acceptance to 20%",
                "REJECTED for lane-aligned loads",
                "the same 8-wide step as the half kernel's 16-byte loads",
                "MEASURED, not bit-identity",
                "pending a second box's numbers",
                "factor-affine-quant-scale-out-of-element-loop",
            ],
        ),
        (
            "bpw-reduction-buys-memory-not-decode-when-dequant-bound",
            &[
                "per-weight dequant cost barely grows with K",
                "INVERTS",
                "52.22 -> 48.70",
                "context ceiling HALVED 262,144 -> 131,072",
                "acceptance 68% -> 74%",
                "a SPECULATION win, not a bandwidth one",
                "ANTI-REGIME: bandwidth-bound decode",
            ],
        ),
        (
            "host-gather-splits-the-graph-and-the-split-is-the-separable-price",
            &[
                "PRICED INDEPENDENTLY",
                "PIECEWISE ALONE measured 5-7%",
                "the RESIDUAL, never the whole",
                "FIRST-TOUCH page in from NVMe",
                "+15 GiB headroom",
            ],
        ),
        (
            "unified-memory-page-cache-is-device-allocatable",
            &[
                "cached file pages count against GPU-allocatable RAM",
                "refuses to load if mem_get_info() looks tight",
                "9.5 min from NVMe vs 2.5 min from page cache",
                "DROP DELIBERATELY AT THE RIGHT MOMENT",
                ">= 8 s apart",
                "never to minimize or maximize blanketly",
            ],
        ),
    ];
    for (id, needles) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-177 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                entry.gpu_concept.contains(needle),
                "batch-177 {id} concept lost phrase: {needle}"
            );
        }
    }
}

#[test]
fn batch177_pseudocode_parses() {
    // G1 for Batch 177: syn-parse gate (the four EXL3/GB10 rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "quantized-pack-residency-audit-unquantized-minority-first",
        "bpw-reduction-buys-memory-not-decode-when-dequant-bound",
        "host-gather-splits-the-graph-and-the-split-is-the-separable-price",
        "unified-memory-page-cache-is-device-allocatable",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-177 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-177 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-177 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch178_rules_present() {
    // Batch 178 (the cinference DFlash2 verify-kernel distill -- Plan 171 /
    // Research 209): 7 kernel_opt rules (A +2: the L2 policy pair; B +1:
    // the warp bitonic topk merge; D +1: the topology-signature graph-exec
    // sharing rule; E +2: the warp-role specialization by query width +
    // the row-slice epilogue rule; N +1: the exact-widening accumulator-
    // precision rule) + 4 riders (entries_gpu_07/08/09/38), distilled from
    // satellitedown/cinference @ 37814b1e (Apache-2.0; fork -> NInfer ->
    // fafstmobel Qwen3.8-27B). Every quote re-verified VERBATIM at the pin
    // THIS batch (full clone; master +9 past the pin, the K8V4 warp trio
    // merged into the LEAD-5 regime clause). RTX 5090/SM120/NVFP4 numbers
    // are source-reported, never ours; rules are mechanism-level, never
    // format-level.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "producer-store-evict-last-pins-cross-stream-consumer",
        "dead-line-l2-discard-after-last-consumer",
        "graph-exec-update-gated-by-captured-topology-signature",
        "exact-widening-enables-fp16-pv-accumulation",
        "wide-verify-warp-specialized-scores-in-registers-pv",
        "row-slice-expensive-epilogue-across-ctas",
        "warp-bitonic-topk-merge-shuffle",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-178 rule '{id}' missing from corpus"
        );
    }
    // The riders: each annotation must be IN the landed rule text.
    let ws = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "weight-stream-non-temporal-loads")
        .expect("weight-stream rule present");
    assert!(
        ws.gpu_concept.contains("RIDER (Batch 178"),
        "batch-178 per-tile evict-first rider missing"
    );
    let cp = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "cpasync-multistage-tile-pipeline")
        .expect("cpasync rule present");
    assert!(
        cp.gpu_concept.contains("RIDER (Batch 178"),
        "batch-178 single-resident-wave regime rider missing"
    );
    let gdn = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "gdn-transition-replay-log-commit-many")
        .expect("gdn replay rule present");
    assert!(
        gdn.gpu_concept.contains("RIDER (Batch 178"),
        "batch-178 GDN replay-record staging rider missing"
    );
    let inv = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "stream-invariant-operand-skips-staging-pipeline")
        .expect("stream-invariant rule present");
    assert!(
        inv.gpu_concept.contains("RIDER (Batch 178"),
        "batch-178 prepare-once/copy-per-split regime-widening rider missing"
    );
}

#[test]
fn batch178_concepts_covered() {
    // G1 for Batch 178: the decision-content needles must survive
    // (case-sensitive contains -- the B51 lesson). One needle per lead
    // quoting the source verbatim, one naming the regime/kinship law.
    let corpus = build_kernel_seed_corpus();
    let checks: &[(&str, &[&str])] = &[
        (
            "producer-store-evict-last-pins-cross-stream-consumer",
            &[
                "Stores a key whose consumer runs after a larger weight stream: the evict-last fill keeps the line in L2 until that consumer reads it.",
                "createpolicy.fractional.L2::evict_last",
                "evict-LAST on the record that MUST",
            ],
        ),
        (
            "dead-line-l2-discard-after-last-consumer",
            &[
                "Only for lines whose data is dead: every later read of the line must follow a new write.",
                "Invalidates the 128-byte L2 line at `line` without writing it back.",
                "pin what a later consumer needs, discard what no consumer needs",
            ],
        ),
        (
            "graph-exec-update-gated-by-captured-topology-signature",
            &[
                "a CUDA graph executable can be updated across profiles only while the topology is unchanged.",
                "Those boundaries are not enumerable from the plan, so the class is derived from the capture itself",
                "instantiate is 10^2-10^3x an in-place update",
            ],
        ),
        (
            "exact-widening-enables-fp16-pv-accumulation",
            &[
                "widens exactly to FP16 at half scale for the PV MMA, which accumulates each 32-key product in FP16 before adding it to the FP32 accumulator",
                "the widening's exactness is the entire legality question, never a generic",
                "refuses the fp16 accumulator",
            ],
        ),
        (
            "wide-verify-warp-specialized-scores-in-registers-pv",
            &[
                "keeps its scores in registers as the PV operand, while two loader warps stream and widen the K/V tiles",
                "consumer Ada has NO setmaxnreg",
                "tensor pipes were idle ~40% of each key tile",
            ],
        ),
        (
            "row-slice-expensive-epilogue-across-ctas",
            &[
                "Quantizing costs three IEEE divisions per lane and step, so five CTAs share each row",
                "1, 2, 5 and 10 slices measured 2.94, 2.24, 1.82 and 2.14",
                "never 'more slices is better'",
            ],
        ),
        (
            "warp-bitonic-topk-merge-shuffle",
            &[
                "max(A[i], B[15 - i]) is a bitonic sequence holding the top kLinearTopK of both lists",
                "Five butterfly rounds merge partner lists",
                "the integer compare IS the float compare",
            ],
        ),
    ];
    for (id, needles) in checks {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == *id)
            .unwrap_or_else(|| panic!("batch-178 rule '{id}' missing"));
        for needle in *needles {
            assert!(
                entry.gpu_concept.contains(needle),
                "batch-178 rule '{id}' lost needle: {needle}"
            );
        }
    }
}

#[test]
fn batch180_rules_present() {
    // Batch 180 (the qvac-fabric Bonsai CUDA PT-layout distill -- Plan 173
    // / Research 211): 4 kernel_opt rules (A +1: the planar-transposed
    // activation plane layout; D +1: the row-kblock item-space flattening
    // + host util search; E +1: the fixed-order interleaved-accumulator
    // epilogue; N +1: the deferred-bias exact-isum integer algebra) + 3
    // riders (entries_gpu_07/11/27), distilled from
    // tetherto/qvac-fabric-llm.cpp @ b9bf3a82 (MIT, branch temp-10549 --
    // the Bonsai LEAGUE lane, our scored model class). Every quote
    // re-verified VERBATIM at the pin THIS batch (fresh shallow clone, the
    // ephemeral-clone law; the branch tip == the pin at clone time). All
    // perf numbers are SOURCE-REPORTED (RTX 3060 / RTX 4070 ncu -- none
    // measured here); rules are mechanism-level. Kinship-first: the N
    // rule composes with the B116 swar rule (same lineage), never restates.
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "planar-transposed-activation-multi-column-quant-matvec",
        "deferred-bias-ternary-dp4a-exact-isum",
        "row-kblock-item-space-flattening-host-util-search",
        "fixed-order-interleaved-accumulator-epilogue",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-180 rule '{id}' missing from corpus"
        );
    }
    // The riders: each annotation must be IN the landed rule text.
    let ws = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "weight-stream-non-temporal-loads")
        .expect("weight-stream rule present");
    assert!(
        ws.gpu_concept.contains("RIDER (Batch 180"),
        "batch-180 prefetch-distance lever rider missing"
    );
    let fw = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "fwht-wide-block-row-per-threadgroup-exchange-ladder")
        .expect("fwht-wide rule present");
    assert!(
        fw.gpu_concept.contains("RIDER (Batch 180"),
        "batch-180 CUDA-twin fwht rider missing"
    );
    let sw = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "swar-lane-widened-base3-dp4a-dot")
        .expect("swar rule present");
    assert!(
        sw.gpu_concept.contains("RIDER (Batch 180"),
        "batch-180 PQ2_0 group-128 codec rider missing"
    );
}

#[test]
fn batch180_pseudocode_parses() {
    // G1 for Batch 180: syn-parse gate (the four qvac rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "planar-transposed-activation-multi-column-quant-matvec",
        "deferred-bias-ternary-dp4a-exact-isum",
        "row-kblock-item-space-flattening-host-util-search",
        "fixed-order-interleaved-accumulator-epilogue",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-180 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-180 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-180 opt_pattern for '{id}' must parse"
        );
    }
}

// The u8→i8 lane reinterpretation IS dp4a's signed-lane semantics, and
// the helper fns follow the fixture tables on purpose (data-first reading).
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::items_after_statements
)]
#[test]
fn batch180_deferred_bias_isum_exhaustive() {
    // The B59 ported-numeric mandate for the N rule: the deferred-bias
    // fold is an EXACT integer identity -- sum((q-1)*a) == sum(q*a) -
    // sum(a) -- over the full digit domain x per-lane activation sweeps
    // with adversarial neighbor fills, consumed through the same
    // unsigned-lane x signed-lane multiply-add the dp4a performs. Plus
    // the int16-range assertion the half-slot transport relies on
    // (|isum| <= 32*127 for a 32-element block of i8 activations).
    const DIGITS: [u32; 3] = [0, 1, 2];
    // all 3^4 = 81 digit words (one digit per byte lane, low to high)
    let mut words = Vec::new();
    for d0 in DIGITS {
        for d1 in DIGITS {
            for d2 in DIGITS {
                for d3 in DIGITS {
                    words.push(d0 | (d1 << 8) | (d2 << 16) | (d3 << 24));
                }
            }
        }
    }
    assert_eq!(words.len(), 81);
    let bias_words: Vec<u32> = words
        .iter()
        .map(|w| {
            // the biased form subtracts 1 per lane BEFORE the dot
            (((w & 0xFF).wrapping_sub(1)) & 0xFF)
                | (((w >> 8) & 0xFF).wrapping_sub(1) & 0xFF) << 8
                | (((w >> 16) & 0xFF).wrapping_sub(1) & 0xFF) << 16
                | (((w >> 24) & 0xFF).wrapping_sub(1) & 0xFF) << 24
        })
        .collect();
    // dp4a semantics: four lanes of (u8 weight x i8 activation),
    // sign-extended products, accumulated into i32.
    fn dp4a_emulate(q: u32, a: u32, acc: i32, weight_signed: bool) -> i32 {
        let mut acc = acc;
        for lane in 0..4 {
            // raw digits {0,1,2} ride the UNSIGNED lane; the biased form
            // subtracts per-lane (vsub4 wraps 0 -> 0xFF = -1 as i8) and
            // consumes the SIGNED lane.
            let ql = if weight_signed {
                ((q >> (8 * lane)) & 0xFF) as u8 as i8 as i32
            } else {
                ((q >> (8 * lane)) & 0xFF) as i32
            };
            let al = ((a >> (8 * lane)) & 0xFF) as u8 as i8 as i32;
            acc += ql * al;
        }
        acc
    }
    fn isum_of(a: u32) -> i32 {
        let mut s = 0i32;
        for lane in 0..4 {
            s += ((a >> (8 * lane)) & 0xFF) as u8 as i8 as i32;
        }
        s
    }
    for (qi, &qw) in words.iter().enumerate() {
        let bw = bias_words[qi];
        for lane in 0..4 {
            for seed in [0x00u32, 0x7F7F_7F7F, 0x8080_8080, 0x5555_5555] {
                // sweep byte `lane` over the FULL i8 domain; neighbors adversarial
                for b in 0..=255u32 {
                    let shift = 8 * lane;
                    let aw = (seed & !(0xFF << shift)) | (b << shift);
                    let raw = dp4a_emulate(qw, aw, 0, false);
                    let biased = dp4a_emulate(bw, aw, 0, true);
                    let isum = isum_of(aw);
                    assert_eq!(
                        raw - isum,
                        biased,
                        "lane {lane} byte {b:#04x} qword {qw:#010x} diverged"
                    );
                }
            }
        }
    }
    // the transport range: a full 32-element block of worst-case i8
    // activations has |sum| <= 32*127 -- the half-slot int16 claim.
    assert_eq!(32 * 127, 4064);
    assert!(4064 < i16::MAX as i32);
}

// The util search mirrors the source's host-side float arithmetic; the
// ratio IS the pinned quantity, and the operands are tiny by construction.
#[allow(clippy::cast_precision_loss)]
#[test]
fn batch180_row_kblock_util_search_boundary() {
    // The D rule's boundary pin, ported from the source's three
    // static_asserts (mmvq-ptq1_0.cuh L464-466): at blocks_per_row=1536,
    // ncols=1, rows_per_item=4, max_smem=48 KiB -- the GATED form (smem
    // per row doubled by the gate partials) must REFUSE (rows_per_cta 0),
    // the gated boundary must ADMIT exactly 49152 bytes, and the UNGATED
    // form must FIT with rows_per_cta == 4.
    const THREADS: usize = 128;
    const MAX_ROWS: usize = 16;
    fn host_config(
        blocks_per_row: usize,
        ncols: usize,
        rows_per_item: usize,
        has_gate: bool,
        max_smem: usize,
    ) -> (usize, usize) {
        let smem_per_row = ncols * (blocks_per_row + 1) * 4 * if has_gate { 2 } else { 1 };
        let mut rmax = max_smem / smem_per_row;
        rmax = rmax.min(MAX_ROWS);
        rmax -= rmax % rows_per_item;
        if rmax < rows_per_item {
            return (0, 0);
        }
        let mut best = rows_per_item;
        let mut best_util = 0.0f64;
        let mut r = rows_per_item;
        while r <= rmax {
            let items = (r / rows_per_item) * blocks_per_row;
            let iters = items.div_ceil(THREADS);
            let util = items as f64 / (iters * THREADS) as f64;
            if util > best_util + 1e-9 {
                best_util = util;
                best = r;
            }
            if util > 0.999 {
                break;
            }
            r += rows_per_item;
        }
        (best, smem_per_row * best)
    }
    // gated at the boundary: the doubled smem overflows -> refusal
    let (rows, smem) = host_config(1536, 1, 4, true, 49152);
    assert_eq!(rows, 0, "gated PTQ1_0 K=196608 must fall back at 48 KiB");
    assert_eq!(smem, 0);
    // the boundary itself must be admitted exactly (1535 blocks fits)
    let (_, smem_admit) = host_config(1535, 1, 4, true, 49152);
    assert_eq!(smem_admit, 49152, "gated must admit the 48 KiB boundary");
    // ungated: fits with rows_per_cta == 4
    let (rows_u, _) = host_config(1536, 1, 4, false, 49152);
    assert_eq!(rows_u, 4, "ungated PTQ1_0 K=196608 must fit at 48 KiB");
    // the util search terminates at near-perfect fill on a fat case
    let (rows_fat, _) = host_config(40, 8, 2, false, 65536);
    assert!(rows_fat >= 2);
}

// The LCG draws are the fixture itself: u64→f32 truncation is the
// reproducible noise source the bit-identity pin is built on.
#[allow(clippy::cast_precision_loss)]
#[test]
fn batch180_fixed_order_epilogue_column_bit_identity() {
    // The E rule's contract: the fixed-order per-(row, column) summation
    // depends only on the weight shape, so a column's result is
    // BIT-IDENTICAL for every column count. The per-column fold is pure
    // in the column index; the pin guards the CONTRACT against a future
    // multi-column specialization that reorders per-column work by
    // column index. The replaced shuffle-order form is a DIFFERENT
    // association (different bits are expected and legal) -- the
    // invariance claim is across COLUMN COUNT, never across orders.
    fn epilogue_fixed_order(partials: &[f32], bpr: usize) -> f32 {
        let src = &partials[..bpr];
        let (mut s0, mut s1, mut s2, mut s3) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let mut kbx = 0;
        while kbx + 4 <= bpr {
            s0 += src[kbx];
            s1 += src[kbx + 1];
            s2 += src[kbx + 2];
            s3 += src[kbx + 3];
            kbx += 4;
        }
        while kbx < bpr {
            s0 += src[kbx]; // scalar tail -- no masked/padded lane
            kbx += 1;
        }
        (s0 + s1) + (s2 + s3)
    }
    let bpr = 37; // 4*9 + 1: the scalar tail lane is exercised
    let mut partials = Vec::with_capacity(bpr);
    let mut x: u64 = 0x243F_6A88_85A3_08D3;
    for _ in 0..bpr {
        x = x
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        partials.push(((x >> 33) as f32) / (u32::MAX as f32) - 0.5);
    }
    let reference = epilogue_fixed_order(&partials, bpr).to_bits();
    for ncols in [1usize, 2, 3, 8] {
        for j in 0..ncols {
            // column j's partials in a real kernel live at a column-strided
            // offset; the summation ORDER per column is the fixed
            // shape-derived one regardless of ncols.
            let col_bits = epilogue_fixed_order(&partials, bpr).to_bits();
            assert_eq!(
                col_bits, reference,
                "column {j} of ncols={ncols} must be bit-identical"
            );
        }
    }
}
#[test]
fn batch178_pseudocode_parses() {
    // G1 for Batch 178: syn-parse gate (the seven cinference rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "producer-store-evict-last-pins-cross-stream-consumer",
        "dead-line-l2-discard-after-last-consumer",
        "graph-exec-update-gated-by-captured-topology-signature",
        "exact-widening-enables-fp16-pv-accumulation",
        "wide-verify-warp-specialized-scores-in-registers-pv",
        "row-slice-expensive-epilogue-across-ctas",
        "warp-bitonic-topk-merge-shuffle",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-178 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-178 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-178 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch181_rules_present() {
    // Batch 181 (the mlxfast-bonsai2 arena distill -- Plan 174 / Research
    // 212): 6 kernel_opt rules (N +2: signed-activation-codes offset
    // elimination + proven-redundant-operand-load-elision; C +1: the
    // fused transform-quantize writer; B +1: tensor-core MMA over staged
    // quantized codes; D +2: the engine work-interval thread-shape pin +
    // the in-flight-cap split submission) + 2 rust_perf members
    // (entries_09), distilled from Layr-Labs/mlxfast-bonsai2-27b-engine
    // @ cfb35b0d (MIT) -- the MLX/Metal ARENA engine for OUR league
    // model. Every quote re-verified VERBATIM at the pin THIS batch
    // (fresh full-history clone, the ephemeral-clone law; the
    // validate->partial-adopt->tree-replacement->restore arc read at
    // tree level). All perf numbers SOURCE-REPORTED (M4/M5 Max).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "signed-activation-codes-drop-dequant-offset-term",
        "fused-transform-quantize-writer",
        "tensor-core-mma-staged-quantized-codes-double-buffered",
        "proven-redundant-operand-load-elision",
        "engine-work-interval-performance-core-pin",
        "split-submission-in-flight-cap-overlap",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-181 rule '{id}' missing from corpus"
        );
    }
    // The riders: each annotation must be IN the landed rule text.
    let b115 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "warm-reachable-dispatch-signatures-before-timed-window")
        .expect("B115 rule present");
    assert!(
        b115.gpu_concept.contains("RIDER (Batch 181"),
        "batch-181 warm-by-alignment-class rider missing"
    );
    let b29 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "weight-handle-cache-keyed-by-allocation-identity")
        .expect("B29 rule present");
    assert!(
        b29.gpu_concept.contains("RIDER (Batch 181"),
        "batch-181 GDN derived-constant rider missing"
    );
    let gdn = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "gdn-transition-replay-log-commit-many")
        .expect("gdn rule present");
    assert!(
        gdn.gpu_concept.contains("RIDER (Batch 181"),
        "batch-181 DVPL promote-then-demote rider missing"
    );
}

#[test]
fn batch181_pseudocode_parses() {
    // G1 for Batch 181: syn-parse gate (the six mlxfast rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "signed-activation-codes-drop-dequant-offset-term",
        "fused-transform-quantize-writer",
        "tensor-core-mma-staged-quantized-codes-double-buffered",
        "proven-redundant-operand-load-elision",
        "engine-work-interval-performance-core-pin",
        "split-submission-in-flight-cap-overlap",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-181 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-181 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-181 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch182_rules_present() {
    // Batch 182 (the mlxfast-bonsai2 DELTA distill -- Plan 177 / Research
    // 216): 6 kernel_opt rules (A +2: the GQA repeat-fold tile-fill + the
    // verify-window prework reorder; C +1: the argmax epilogue swap; D +1:
    // the post-idle residency-restore scheduling; E +2: the draft-block
    // KV-split exchange + the redundant small reduction) distilled from
    // Layr-Labs/mlxfast-bonsai2-27b-engine delta cfb35b0d..831fae7 (MIT) --
    // the post-B181 accept wave (the armed Research-212-Addendum-3 rider
    // fired). Every quote re-verified VERBATIM at HEAD THIS batch (fresh
    // clone, the ephemeral-clone law). All perf numbers SOURCE-REPORTED
    // (M4/M5 Max class).
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "verify-fold-gqa-repeats-into-row-axis-tile-fill",
        "draft-block-simdgroup-kv-split-smem-softmax-exchange",
        "simdgroup-redundant-small-reduction-over-barrier-broadcast",
        "argmax-epilogue-replaces-output-materialization",
        "verify-window-prework-loads-before-stores",
        "idle-gpu-residency-restore-overlapped-behind-queued-work",
    ];
    for id in ids {
        assert!(
            corpus.entries.iter().any(|e| e.rule_id == id),
            "batch-182 rule '{id}' missing from corpus"
        );
    }
    // The riders: each annotation must be IN the landed rule text.
    let rider_rules = [
        "engine-work-interval-performance-core-pin",
        "split-submission-in-flight-cap-overlap",
        "fused-transform-quantize-writer",
        "gdn-transition-replay-log-commit-many",
        "defer-promoted-write-to-first-consumer-dispatch",
        "in-place-write-through-input-emits-fence-consumer",
    ];
    for id in rider_rules {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("{id} present"));
        assert!(
            rule.gpu_concept.contains("RIDER (Batch 182"),
            "batch-182 rider missing on {id}"
        );
    }
}

#[test]
fn batch182_pseudocode_parses() {
    // G1 for Batch 182: syn-parse gate (the six mlxfast-delta rules).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "verify-fold-gqa-repeats-into-row-axis-tile-fill",
        "draft-block-simdgroup-kv-split-smem-softmax-exchange",
        "simdgroup-redundant-small-reduction-over-barrier-broadcast",
        "argmax-epilogue-replaces-output-materialization",
        "verify-window-prework-loads-before-stores",
        "idle-gpu-residency-restore-overlapped-behind-queued-work",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-182 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-182 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-182 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch186_pseudocode_parses() {
    // G1 for Batch 186: syn-parse gate (the ggml-cpu tiled mul_mat
    // distill -- 15 kernel_opt rules across entries_gpu_53/54).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "dequant-once-macrotile-stage-reuse-across-microtiles",
        "stage-operand-mac-layout-inplace-transpose-at-repack",
        "plan-time-thread-slot-scratch",
        "narrow-m-l1-panel-sizing",
        "jit-band-repack-stage-per-use-l1-hot",
        "zero-pad-ragged-tail-additive-identity",
        "maddubs-overflow-two-exact-mac-selector",
        "hoist-row-invariant-scale-to-subblock-epilogue",
        "consumer-offset-absorption-precomputed-weight-row-correction",
        "regime-batch-floor-force-escape",
        "unified-admission-dead-tail",
        "owned-input-transform-early-admission",
        "moe-per-expert-mixed-dispatch-worst-case-workspace",
        "const-generic-microkernel-format-unification",
        "widen-metadata-once-to-mac-native-widths",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-186 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-186 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-186 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch187_pseudocode_parses() {
    // G1 for Batch 187: syn-parse gate (the ggml-cpu staged kernel
    // pack of 5 -- Research 218 §STAGED converted: A x1 + B x3 + E x1
    // across entries_gpu_55).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "jmajor-acc-buffer-transpose-writeback",
        "register-budget-band-partition-indep-correction-offband",
        "chunk-downsize-work-stealing-microtile-floor",
        "lut16-pshufb-byte-expansion-at-stage",
        "gfni-sign-mask-expand-cmpeq-fallback",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-187 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-187 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-187 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch190_pseudocode_parses() {
    // G1 for Batch 190: syn-parse gate (the cubecl #1731 umax-divisor
    // solo pack of 1 -- Research 222 / Plan 187: the O-category
    // runtime-divisor clamp that admits LSR, across entries_gpu_58).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["clamp-runtime-divisor-for-lsr-address-reduction"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-190 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-190 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-190 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch190_unsigned_only_contract_rides_the_rule() {
    // The Batch-190 unsigned-only contract is load-bearing decision
    // content (the source deliberately leaves SDivOp/SRemOp unclamped):
    // the rule text must carry it so a heal suggestion never proposes
    // the clamp on a signed divisor.
    let corpus = build_kernel_seed_corpus();
    let rule = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "clamp-runtime-divisor-for-lsr-address-reduction")
        .expect("batch-190 rule present");
    assert!(
        rule.gpu_concept.contains("UNSIGNED ONLY"),
        "the unsigned-only contract must ride the rule text"
    );
    assert!(
        rule.gpu_concept.contains("SDivOp/SRemOp"),
        "the source's deliberate signed-div absence must be cited"
    );
}

#[test]
fn batch192_pseudocode_parses() {
    // G1 for Batch 192: syn-parse gate (the llama.cpp master-delta pack of
    // 4 -- Plan 189 / Research 224: the MoE tile-picker shape-truth, the
    // descriptor-set binding memo with destroy-epoch, the vendor-keyed
    // reduction lane-partition + subgroup request, and the dot macro
    // family that widens at load -- across entries_gpu_59).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "tile-picker-input-from-dispatch-grid-partition",
        "descriptor-set-binding-memo-destroy-epoch",
        "vendor-keyed-reduction-lane-partition-subgroup-request",
        "dot-macro-family-widens-at-load-f32-accumulate",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-192 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-192 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-192 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch192_concepts_carry_their_load_bearing_contracts() {
    // The Batch-192 decision-content contracts (the batch190_unsigned_only
    // precedent): each rule's load-bearing caveat must ride the rule text
    // so a heal suggestion never strips it.
    let corpus = build_kernel_seed_corpus();
    let picker = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "tile-picker-input-from-dispatch-grid-partition")
        .expect("batch-192 picker rule present");
    assert!(
        picker.gpu_concept.contains("DERIVED quantity"),
        "the picker rule's DERIVED-vs-tensor-dim contract must ride the text"
    );
    let memo = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "descriptor-set-binding-memo-destroy-epoch")
        .expect("batch-192 memo rule present");
    assert!(
        memo.gpu_concept.contains("bump before destroying"),
        "the destroy-epoch happens-before ordering must ride the text"
    );
    let lane = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "vendor-keyed-reduction-lane-partition-subgroup-request")
        .expect("batch-192 lane rule present");
    assert!(
        lane.gpu_concept.contains("IN-SOURCE COMMENT claims"),
        "the ~10x numbers-honesty caveat must ride the text"
    );
    let dot = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "dot-macro-family-widens-at-load-f32-accumulate")
        .expect("batch-192 dot rule present");
    assert!(
        dot.gpu_concept.contains("NO rate cliff"),
        "the no-rate-cliff refusal-pole justification must ride the text"
    );
}

#[test]
fn batch192_riders_landed_on_their_hosts() {
    // The 4 riders + 1 amend (Plan 189): each host entry must now carry
    // its Batch-192 rider sentence (the batch116 membership precedent --
    // a rider that never landed is a staging note, not corpus).
    let corpus = build_kernel_seed_corpus();
    let hosts = [
        (
            "dequant-byte-pairs-as-aligned-u16-loads",
            "FULL-WIDTH-ELEMENT arm",
        ),
        (
            "fwht-wide-block-row-per-threadgroup-exchange-ladder",
            "GGML_METAL_FWHT_TG_MIN_N 512",
        ),
        (
            "maddubs-overflow-two-exact-mac-selector",
            "INVERSE-polarity member",
        ),
        (
            "wide-row-copy-tile-rows-across-workgroups",
            "widening-axis CONTRAST arm",
        ),
        (
            "dtype-native-source-read-with-shared-admission-predicate",
            "64-256 simdgroup / 512-8192 tg-gated",
        ),
    ];
    for (host, marker) in hosts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == host)
            .unwrap_or_else(|| panic!("rider host '{host}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "Batch-192 rider marker '{marker}' missing on host '{host}'"
        );
    }
}

#[test]
fn batch193_pseudocode_parses() {
    // G1 for Batch 193: syn-parse gate (the mlxfast-bonsai2 post-reorg
    // accept-wave pack of 14 -- Plan 190 / Research 225: the submission/
    // scheduling/KV slice + the kernel/numerics/layout slice + the
    // coordinator's DRAM-streaming adoption rotation -- across
    // entries_gpu_60).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "adoption-trial-rotate-operands-per-launch-dram-streaming",
        "first-command-buffer-fresh-buffer-mapping-split",
        "kv-first-append-unzeroed-alloc-offset-gated-tail",
        "deadline-bounded-poll-of-submitted-completion-event",
        "full-accept-state-store-host-observable-predictor",
        "readback-domain-excludes-cold-tap-copies",
        "adoption-staged-leading-layers-gpu-idle-fill",
        "seed-first-command-buffer-earliest-submission",
        "construction-identity-row-classes-replace-probe",
        "redundant-row-reduction-per-width-block-grid-split",
        "branch-free-top2-insert-order-invariant-guard",
        "register-built-right-operand-lane-contiguous-plane-layout",
        "dual-layout-transpose-at-stage",
        "plane-form-activation-reuse-topology",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-193 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-193 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-193 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch193_concepts_carry_their_load_bearing_contracts() {
    // The Batch-193 decision-content contracts (the batch190_unsigned_only
    // precedent): each rule's load-bearing caveat must ride the rule text
    // so a heal suggestion never strips it.
    let corpus = build_kernel_seed_corpus();
    let contracts = [
        (
            "kv-first-append-unzeroed-alloc-offset-gated-tail",
            "structurally unreachable",
        ),
        (
            "deadline-bounded-poll-of-submitted-completion-event",
            "0.25 s",
        ),
        (
            "full-accept-state-store-host-observable-predictor",
            "the host cannot tell",
        ),
        (
            "readback-domain-excludes-cold-tap-copies",
            "completion event",
        ),
        (
            "construction-identity-row-classes-replace-probe",
            "7 of 43",
        ),
        (
            "branch-free-top2-insert-order-invariant-guard",
            "count == 2",
        ),
        (
            "register-built-right-operand-lane-contiguous-plane-layout",
            "not in the trial",
        ),
        (
            "dual-layout-transpose-at-stage",
            "load-time trial",
        ),
        (
            "adoption-trial-rotate-operands-per-launch-dram-streaming",
            "ranks the two the other way",
        ),
    ];
    for (id, marker) in contracts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-193 rule '{id}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "batch-193 contract marker '{marker}' missing on '{id}'"
        );
    }
}

#[test]
fn batch193_riders_landed_on_their_hosts() {
    // The Batch-193 riders + the rust_perf host amend (the batch116
    // membership precedent — a rider that never landed is a staging note,
    // not corpus): two kernel_opt hosts carry their rider sentences, and
    // the rust_perf family host carries the premise+margin amendment.
    let corpus = build_kernel_seed_corpus();
    let hosts = [
        (
            "exact-widening-enables-fp16-pv-accumulation",
            "RIDER (Batch 193",
        ),
        (
            "planar-transposed-activation-multi-column-quant-matvec",
            "nwarps is pinned to 4",
        ),
    ];
    for (host, marker) in hosts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == host)
            .unwrap_or_else(|| panic!("rider host '{host}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "Batch-193 rider marker '{marker}' missing on host '{host}'"
        );
    }
    // The rust_perf family host: the adoptMargin amend + the premise
    // correction + the rotation cross-reference.
    let rust_corpus = crate::domains::rust_perf::corpus::build_perf_seed_corpus();
    let host = rust_corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "whole-round-in-situ-adoption-trial")
        .expect("rust_perf family host present");
    let Some(notes) = &host.context_notes else {
        panic!("rust_perf family host carries context_notes");
    };
    assert!(
        notes.contains("AMENDMENT (Batch 193"),
        "the Batch-193 amendment paragraph must ride the family host"
    );
    assert!(
        notes.contains("adoptMargin 0.005 → 0.015"),
        "the margin amend must name the constant move"
    );
    assert!(
        notes.contains("RETIRED"),
        "the correctness-free premise correction must ride the host"
    );
    assert!(
        host.fixed_code().contains("1.5"),
        "the pinned ADOPT_MARGIN_PCT constant must move to 1.5"
    );
}

#[test]
fn batch196_pseudocode_parses() {
    // G1 for Batch 196: syn-parse gate (the TensorFold v0.6.0 A− pack of
    // 2 -- Plan 195: the affine block address pipeliner lead + the
    // register-width topk tiled-radix fallback).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "affine-block-address-lets-pipeliner-fly",
        "register-width-topk-falls-back-to-tiled-radix-select",
    ];
    assert_eq!(ids.len(), 2, "batch-196 fixture count = landed rules");
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-196 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-196 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-196 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch196_concepts_carry_their_load_bearing_contracts() {
    // The Batch-196 decision-content contracts (the batch195 precedent):
    // each rule's load-bearing caveat must ride the rule text.
    let corpus = build_kernel_seed_corpus();
    let contracts = [
        (
            "affine-block-address-lets-pipeliner-fly",
            "an address the pipeliner follows",
        ),
        (
            "register-width-topk-falls-back-to-tiled-radix-select",
            "THE SAME LISTS",
        ),
    ];
    for (id, marker) in contracts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-196 rule '{id}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "batch-196 contract marker '{marker}' missing on '{id}'"
        );
    }
}

#[test]
fn batch196_riders_landed_on_their_hosts() {
    // The Batch-196 rider (the batch195 membership precedent -- a rider
    // that never landed is a staging note, not corpus): the lane_fuse
    // g32 admission instance rides B176's host rule.
    let corpus = build_kernel_seed_corpus();
    let hosts = [(
        "dtype-native-source-read-with-shared-admission-predicate",
        "RIDER (Batch 196",
    )];
    for (host, marker) in hosts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == host)
            .unwrap_or_else(|| panic!("rider host '{host}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "Batch-196 rider marker '{marker}' missing on host '{host}'"
        );
    }
}

#[test]
fn batch197_pseudocode_parses() {
    // G1 for Batch 197: syn-parse gate (the mlx VJP distill of 6 — Plan
    // 193: the composed-backward A− GOAT lead, the segmented entry-state
    // checkpoint + cheap replay, the GQA zero-stride batch broadcast, the
    // threadgroup-scope reduction single-writer, the forward-save chunk
    // cache, and the chain-rule postprocess kernel).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "sdpa-backward-as-tiled-matmul-bounded-materialization",
        "segmented-checkpoint-cheap-replay-backward",
        "gqa-zero-stride-batch-broadcast",
        "threadgroup-scope-reduction-single-writer",
        "cache-symmetric-intermediates-per-chunk",
        "chain-rule-postprocess-kernel",
    ];
    assert_eq!(ids.len(), 6, "batch-197 fixture count = landed rules");
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-197 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-197 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-197 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch197_concepts_carry_their_load_bearing_contracts() {
    // The Batch-197 decision-content contracts (the batch196 precedent):
    // each rule's load-bearing caveat must ride the rule text.
    let corpus = build_kernel_seed_corpus();
    let contracts = [
        (
            "sdpa-backward-as-tiled-matmul-bounded-materialization",
            "PR #3241",
        ),
        (
            "segmented-checkpoint-cheap-replay-backward",
            "it must be the entry state and not the exit state",
        ),
        ("gqa-zero-stride-batch-broadcast", "zero innermost batch stride"),
        (
            "threadgroup-scope-reduction-single-writer",
            "never leaves threadgroup memory",
        ),
        (
            "cache-symmetric-intermediates-per-chunk",
            "only the first tile needs to write it out",
        ),
        (
            "chain-rule-postprocess-kernel",
            "computes the gradient with respect to the log",
        ),
    ];
    for (id, marker) in contracts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-197 rule '{id}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "batch-197 contract marker '{marker}' missing on '{id}'"
        );
    }
}

#[test]
fn batch197_riders_landed_on_their_hosts() {
    // The Batch-197 rider (the batch196 membership precedent — a rider
    // that never landed is a staging note, not corpus): the operand-typed
    // macro instance rides B192's host rule.
    let corpus = build_kernel_seed_corpus();
    let hosts = [(
        "dot-macro-family-widens-at-load-f32-accumulate",
        "RIDER (Batch 197",
    )];
    for (host, marker) in hosts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == host)
            .unwrap_or_else(|| panic!("rider host '{host}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "Batch-197 rider marker '{marker}' missing on host '{host}'"
        );
    }
}

#[test]
fn batch195_pseudocode_parses() {
    // G1 for Batch 195: syn-parse gate (the mlxfast-bonsai2 wave-2 pack of
    // 9 -- Plan 194: the O pair, the D trio + one more D, the B, the C,
    // the N, the E).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "io32-count-pinned-text-narrowing",
        "strided-view-offset-fit32-launch-proof",
        "write-only-touch-side-queue",
        "last-query-rect-projection",
        "roundfuse-packed-embedding-gather",
        "enumerable-domain-bit-identity-admission-gate",
        "cross-queue-output-retention-until-completion",
        "atexit-registration-order-shutdown",
        "lazy-admission-reservation-atomic-register",
    ];
    assert_eq!(ids.len(), 9, "batch-195 fixture count = landed rules");
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-195 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-195 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-195 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch195_concepts_carry_their_load_bearing_contracts() {
    // The Batch-195 decision-content contracts (the batch194 precedent):
    // each rule's load-bearing caveat must ride the rule text.
    let corpus = build_kernel_seed_corpus();
    let contracts = [
        (
            "io32-count-pinned-text-narrowing",
            "64-bit text kept",
        ),
        (
            "strided-view-offset-fit32-launch-proof",
            "sum of (extent - 1) * stride",
        ),
        (
            "write-only-touch-side-queue",
            "write a scalar no one reads",
        ),
        (
            "last-query-rect-projection",
            "every element the full product's",
        ),
        (
            "roundfuse-packed-embedding-gather",
            "offset_neg_idx",
        ),
        (
            "enumerable-domain-bit-identity-admission-gate",
            "original concat/cast kept",
        ),
        (
            "cross-queue-output-retention-until-completion",
            "hazard-untracked",
        ),
        (
            "atexit-registration-order-shutdown",
            "reverse registration order",
        ),
        (
            "lazy-admission-reservation-atomic-register",
            "one critical section",
        ),
    ];
    for (id, marker) in contracts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-195 rule '{id}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "batch-195 contract marker '{marker}' missing on '{id}'"
        );
    }
}

#[test]
fn batch195_riders_landed_on_their_hosts() {
    // The Batch-195 riders (the batch194 membership precedent — a rider
    // that never landed is a staging note, not corpus): the kv-first-append
    // adopt contract rides B179's host rule, the Metal native-dtype read
    // instance rides B176's host rule.
    let corpus = build_kernel_seed_corpus();
    let hosts = [
        (
            "kv-first-append-unzeroed-alloc-offset-gated-tail",
            "RIDER (Batch 195",
        ),
        (
            "dtype-native-source-read-with-shared-admission-predicate",
            "RIDER (Batch 195",
        ),
    ];
    for (host, marker) in hosts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == host)
            .unwrap_or_else(|| panic!("rider host '{host}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "Batch-195 rider marker '{marker}' missing on host '{host}'"
        );
    }
}

#[test]
fn batch194_pseudocode_parses() {
    // G1 for Batch 194: syn-parse gate (the DeepGEMM-Ascend pack of 14 --
    // Research 226 / Plan 191: 7 + the two verdict leads in entries_gpu_61,
    // 5 in entries_gpu_62).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "fp4-nibble-gather-lut-replicated-index-width",
        "decode-sidecar-coroutine-writes-consumer-stage-ring",
        "ring-overwrite-gated-k-back-readiness",
        "empty-participant-progress-credit",
        "issue-latency-priced-mad-cost-model",
        "multiwave-dual-decode-admission-gate",
        "work-size-gated-overlap-reduce-mode",
        "l1-resident-operand-reuse-address-identity-rewind",
        "decoupled-sf-sidecar-pipeline-own-granularity",
        "coprime-stride-cross-rank-source-interleave",
        "paged-page-pair-deferred-permutation-bitmask",
        "dual-consumer-alternating-stage-ownership-scale-lane",
        "dependency-wave-task-picker-one-wave-lookahead",
        "in-order-pipe-credit-release-completion-collapse",
    ];
    assert_eq!(ids.len(), 14, "batch-194 fixture count = landed rules");
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-194 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-194 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-194 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch194_concepts_carry_their_load_bearing_contracts() {
    // The Batch-194 decision-content contracts (the batch193 precedent):
    // each rule's load-bearing caveat must ride the rule text.
    let corpus = build_kernel_seed_corpus();
    let contracts = [
        (
            "fp4-nibble-gather-lut-replicated-index-width",
            "repeats every 16 entries",
        ),
        (
            "decode-sidecar-coroutine-writes-consumer-stage-ring",
            "kDirectStore",
        ),
        (
            "ring-overwrite-gated-k-back-readiness",
            "slot i-K",
        ),
        (
            "empty-participant-progress-credit",
            "Empty row owners must also contribute",
        ),
        (
            "issue-latency-priced-mad-cost-model",
            "40 : 116",
        ),
        (
            "multiwave-dual-decode-admission-gate",
            "after final tiling",
        ),
        (
            "work-size-gated-overlap-reduce-mode",
            "32 MiB",
        ),
        (
            "l1-resident-operand-reuse-address-identity-rewind",
            "kNumCores",
        ),
        (
            "decoupled-sf-sidecar-pipeline-own-granularity",
            "first L0 copy",
        ),
        (
            "coprime-stride-cross-rank-source-interleave",
            "without changing their counts",
        ),
        (
            "paged-page-pair-deferred-permutation-bitmask",
            "physical-address order",
        ),
        (
            "dual-consumer-alternating-stage-ownership-scale-lane",
            "owner of each even/odd stage pair",
        ),
        (
            "dependency-wave-task-picker-one-wave-lookahead",
            "lookahead",
        ),
        (
            "in-order-pipe-credit-release-completion-collapse",
            "same MTE1 queue",
        ),
    ];
    for (id, marker) in contracts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-194 rule '{id}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "batch-194 contract marker '{marker}' missing on '{id}'"
        );
    }
}

#[test]
fn batch194_riders_landed_on_their_hosts() {
    // The Batch-194 riders (the batch193 membership precedent — a rider
    // that never landed is a staging note, not corpus): two hosts in the
    // existing files carry their rider sentences; the other two riders are
    // folded INTO their rules at authoring (l1-resident + sf-sidecar),
    // asserted via their own contract markers above.
    let corpus = build_kernel_seed_corpus();
    let hosts = [
        (
            "weight-stream-non-temporal-loads",
            "RIDER (Batch 194",
        ),
        (
            "ue8m0-pow2-scale-as-biased-exponent-byte",
            "RIDER (Batch 194",
        ),
    ];
    for (host, marker) in hosts {
        let rule = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == host)
            .unwrap_or_else(|| panic!("rider host '{host}' missing"));
        assert!(
            rule.gpu_concept.contains(marker),
            "Batch-194 rider marker '{marker}' missing on host '{host}'"
        );
    }
}

#[test]
fn batch189_pseudocode_parses() {
    // G1 for Batch 189: syn-parse gate (the open-jev-fast kernel pack of
    // 5 -- Research 221 / Plan 185: the G overlap rule + the F tree
    // attention + the B activation LUT + the two D rules across
    // entries_gpu_57).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "two-chunk-state-independent-overlap-serial-chunk-recurrence",
        "ancestor-mask-bit-visibility-block-skip-tree-attention",
        "exact-bf16-domain-activation-lut",
        "shape-bucketed-graph-cache-lru",
        "fused-serving-stack-measured-negative-ledger",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-189 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-189 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-189 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch189_riders_present() {
    // The Batch-189 rider surface: the split-K fold-at-consumer epilogue
    // placement rides split-k-two-pass-attention-decode-long-context
    // (entries_gpu_18) as rule TEXT, not a new rule.
    let corpus = build_kernel_seed_corpus();
    let host = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "split-k-two-pass-attention-decode-long-context")
        .expect("split-k-two-pass host rule present");
    assert!(
        host.gpu_concept.contains("RIDER (Batch 189"),
        "the split-K fold-at-consumer rider must ride the split-k rule"
    );
}

#[test]
fn batch188_pseudocode_parses() {
    // G1 for Batch 188: syn-parse gate (the graded opponent-watch
    // leads pack of 4 -- Research 220 / Plan 184: TensorFold EXL3
    // A1/A2/A3 + MLX gather_qmm B1 across entries_gpu_56).
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = [
        "low-bit-warp-run-load-shuffle-redistribute",
        "register-prefetch-ring-measured-cutoff",
        "shape-only-plan-row-invariant-kernel",
        "sorted-gather-tile-compaction-warp-scan-scheduler",
    ];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-188 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-188 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-188 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch176_rules_present() {
    // Batch 176 (Plan 176 / Research 214) -- the #29096 merge rider: the
    // FWHT dtype-native source read + the shared admission predicate.
    use crate::domain::HealDomain;
    let corpus = build_kernel_seed_corpus();
    let ids = ["dtype-native-source-read-with-shared-admission-predicate"];
    for id in ids {
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == id)
            .unwrap_or_else(|| panic!("batch-176 rule '{id}' missing"));
        let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
        assert!(
            verdict_passes(&ref_verdict),
            "batch-176 ref_shape for '{id}' must parse"
        );
        let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
        assert!(
            verdict_passes(&opt_verdict),
            "batch-176 opt_pattern for '{id}' must parse"
        );
    }
}

#[test]
fn batch176_riders_present() {
    // The two rider surfaces: B96 carries the branchless select rider, B116
    // carries the upstream width-ladder instance -- both in the rule TEXT.
    let corpus = build_kernel_seed_corpus();
    let b96 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "fwht-shuffle-xor-butterfly-register-resident")
        .expect("B96 present");
    assert!(
        b96.gpu_concept.contains("RIDER (Batch 176"),
        "B96 must carry the batch-176 branchless-select rider"
    );
    let b116 = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "fwht-wide-block-row-per-threadgroup-exchange-ladder")
        .expect("B116 present");
    assert!(
        b116.gpu_concept.contains("RIDER (Batch 176"),
        "B116 must carry the batch-176 upstream-instance rider"
    );
}

// The `as` widenings are the subject under test: mantissa/colsum values
// are small by the regime the gate pins, and the claim is bit-exactness
// of the two float FORMS, so the casts must stay literal.
#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
#[test]
fn batch181_signed_codes_mantissa_gate() {
    // The N rule's load-bearing contract: the signed-codes epilogue forms
    // fma(s, C, -128 s colsum) and s * (C - 128 colsum) are BIT-IDENTICAL
    // inside the mantissa budget (bits(scale) + bits(colsum) <= 24) and
    // GENUINELY BREAK outside it. The NEGATIVE arm is mandatory (Plan 174
    // T3.1): a pass-only-inside fixture would hide the regime.
    //   2-bit g128: colsum <= 384 (9 bits) + FP16 scale (11 bits) = 20 -> exact
    //   4-bit:      colsum <= 1920 (11 bits) + 11 = 22 -> exact
    //   8-bit:      colsum to 32,640 (15 bits) + 11 = 26 -> NOT exact
    // Worked-number pins first.
    let bits_colsum = |max_cs: i32| -> u32 { 32 - (max_cs as u32).leading_zeros() };
    assert_eq!(bits_colsum(384), 9);
    assert_eq!(bits_colsum(1920), 11);
    assert_eq!(bits_colsum(32_640), 15);
    assert_eq!(11 + 9, 20); // 2-bit g128: inside 24
    assert_eq!(11 + 11, 22); // 4-bit: inside 24
    assert_eq!(11 + 15, 26); // 8-bit: OUTSIDE 24

    // FP16-exact scales with full fraction range: s = (1024 + k) / 1024,
    // k in 0..=1023 -- 11-bit significands, exactly representable.
    let scales: Vec<f32> = [0, 1, 2, 3, 5, 17, 100, 511, 512, 1000, 1023]
        .iter()
        .map(|&k| (1024 + k) as f32 / 1024.0)
        .collect();
    // In-regime colsums (2-bit g128 worst case) and code-dot products C
    // small enough that every intermediate integer stays exact (< 2^24).
    let colsums_in: Vec<i32> = vec![0, 1, 42, 127, 128, 255, 256, 300, 383, 384];
    let cs: Vec<i32> = vec![1, 3, 1000, 33_555, 100_000];

    let mut diverged_in = false;
    for &s in &scales {
        for &colsum in &colsums_in {
            for &c in &cs {
                let form1 = s.mul_add(c as f32, -128.0 * s * (colsum as f32));
                let form2 = s * (c as f32 - 128.0 * colsum as f32);
                if form1.to_bits() != form2.to_bits() {
                    diverged_in = true;
                }
            }
        }
    }
    assert!(!diverged_in, "forms must be bit-identical INSIDE the 2-bit g128 regime");

    // OUT of regime: 8-bit colsums (15-bit values). The offset term
    // 128*s*colsum exceeds the FP32 significand budget and ROUNDS, so the
    // fma form and the folded form genuinely diverge for some C. The arm
    // searches a small grid and REQUIRES a counterexample to exist -- a
    // regime claim without a measured break is a guess.
    let colsums_out: Vec<i32> = vec![32_640, 32_641, 32_500, 30_000];
    let mut found_break = false;
    'outer: for &s in &scales {
        for &colsum in &colsums_out {
            for c in 1..=64 {
                let form1 = s.mul_add(c as f32, -128.0 * s * (colsum as f32));
                let form2 = s * (c as f32 - 128.0 * colsum as f32);
                if form1.to_bits() != form2.to_bits() {
                    found_break = true;
                    break 'outer;
                }
            }
        }
    }
    assert!(found_break, "the 8-bit shape must produce a REAL divergence (the NEGATIVE arm)");
}

// Half-precision widen + the sign-flip identity: the u16→u32 widenings
// are infallible From conversions; `m as f32` is the subnormal formula;
// the float compare is exact by construction (sign bit applied both
// ways); and `0x0000 ^ 0x8000` is the operation under test, kept
// symmetric with the -0.0 line below it.
#[allow(clippy::cast_precision_loss, clippy::float_cmp, clippy::identity_op)]
#[test]
fn batch181_negative_scale_bias_bit_gate() {
    // The LEAD-7 proof contract: FP16 sign-flip (xor 0x8000) is EXACT
    // negation including signed zero, so a bitwise equality of the flipped
    // scale bits with the bias bits proves bias == -scale with no
    // arithmetic; the NEG arm (bv = -sv, no load) is then bit-identical to
    // the base form, and the proof REFUSES (keeps both loads) on any
    // mismatched bias.
    let widen = |bits: u16| -> f32 {
        let sign = u32::from(bits & 0x8000) << 16;
        let e = u32::from((bits >> 10) & 0x1f);
        let m = u32::from(bits & 0x03ff);
        if e == 0 && m == 0 {
            f32::from_bits(sign)
        } else if e == 0 {
            (m as f32) * 2_f32.powi(-24) * if bits & 0x8000 != 0 { -1.0 } else { 1.0 }
        } else if e == 0x1f {
            f32::from_bits(sign | 0x7f80_0000 | (m << 13))
        } else {
            f32::from_bits(sign | ((e + 112) << 23) | (m << 13))
        }
    };
    // (a) the sign-flip identity over every finite pattern: normals by the
    // bit formula, subnormals by the value formula, both zeros explicit.
    for bits in 0x0001..=0x7bff {
        let flipped = widen(bits ^ 0x8000);
        assert_eq!(flipped, -widen(bits), "sign-flip broke at {bits:#06x}");
    }
    assert_eq!(widen(0x0000 ^ 0x8000).to_bits(), (-widen(0x0000)).to_bits()); // -0.0 == -0.0 bitwise
    assert_eq!(widen(0x8000 ^ 0x8000).to_bits(), (-widen(0x8000)).to_bits());
    // (b) the NEG-arm epilogue is bit-identical to the base form where the
    // proof holds, over a small fixture grid.
    let scales: Vec<u16> = vec![0x3c00, 0x3d00, 0x2c00, 0x0001, 0x8000];
    let biases: Vec<u16> = scales.iter().map(|s| s ^ 0x8000).collect();
    for g in 0..scales.len() {
        let base = widen(scales[g]) * 5.0 + widen(biases[g]);
        let neg = widen(scales[g]) * 5.0 + -widen(scales[g]);
        assert_eq!(base.to_bits(), neg.to_bits(), "NEG arm diverged at group {g}");
    }
    // (c) the proof REFUSES on any mismatched bias (the negative arm).
    let mut bad_biases = biases.clone();
    bad_biases[2] ^= 0x0010; // one payload bit off
    let holds: bool = scales.iter().zip(bad_biases.iter()).all(|(&s, &b)| s ^ 0x8000 == b);
    assert!(!holds, "the proof must refuse a corrupted bias table");
}

#[test]
#[cfg(feature = "kernel_opt")]
fn batch181_neg_bias_proof_is_not_vacuous() {
    // Runtime probe made permanent (the batch29 exact-element lesson): the
    // corpus text carries BOTH the old needle (element-equality tests in
    // this file compare FULL concept strings, never substrings) AND the
    // rider -- the two tests above pin each half; this one pins the JOIN
    // (the rider sits after the old tail, so neither half drifted).
    let corpus = build_kernel_seed_corpus();
    let e = corpus
        .entries
        .iter()
        .find(|e| e.rule_id == "weight-handle-cache-keyed-by-allocation-identity")
        .expect("b29 present");
    let tail = "and the owning projection invalidates on parameter update.";
    assert!(e.gpu_concept.ends_with(tail));
    assert!(e.gpu_concept.contains("RIDER (Batch 181"));
}

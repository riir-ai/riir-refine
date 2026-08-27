    use super::*;
    use crate::domain::HealEntry;

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
        // Total: 209. // (historical batch-30 snapshot; current 370)
        assert_eq!(corpus.entries.len(), 384, "kernel batches through Batch 79 should have 384 rules");
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
        assert!(verdict.parses);
    }

    #[test]
    #[cfg(not(feature = "kernel_verify"))]
    fn validate_rejects_garbage() {
        use crate::domain::HealDomain;
        let verdict = KernelOptDomain::validate("matvec-coalesce", "not valid rust {");
        assert!(!verdict.parses);
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
        let concepts: Vec<&str> = corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Memory access patterns
        assert!(concepts.contains(&"coalesced memory access"));
        assert!(concepts.contains(&"vectorized memory access"));
        assert!(concepts.contains(&"work distribution"));
        // Compute patterns
        assert!(concepts.contains(&"autotune dispatch"));
        assert!(concepts.contains(&"numerical precision in reductions"));
        assert!(concepts.contains(&"branch divergence elimination"));
        assert!(concepts.contains(&"loop unrolling for instruction-level parallelism"));
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"flash-attention online softmax"));
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"grouped GEMM with prefix-sum offsets"));
        assert!(concepts.contains(&"swizzled L2-cache-aware tile mapping"));
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        assert!(ids.contains(&"per-key-dim-diagonal-gate"), "gla per-dim gate");
        assert!(ids.contains(&"erase-replace-low-rank-transition"), "rwkv7 4-phase");
        assert!(ids.contains(&"oja-subtractive-normalization"), "gated_oja_rule");
        assert!(ids.contains(&"identity-plus-low-rank-delta"), "iplr_delta_rule");
        assert!(ids.contains(&"cumsum-gate-to-decay-matrix"), "simple_gla pairwise");
        // E: parallelism (+2)
        assert!(ids.contains(&"reverse-scan-via-double-flip"), "chunk_*_cumsum reverse");
        assert!(ids.contains(&"pairwise-distance-decay-matrix"), "retention/lightning decay");
        // F: flash-attn tiling (+1)
        assert!(ids.contains(&"strict-causal-mask-tril-offset"), "deltaformer tril(-1)");
        // B: compute (+2)
        assert!(ids.contains(&"polynomial-taylor-feature-map"), "based/rebased poly");
        assert!(ids.contains(&"positive-feature-map-shift"), "linear_attn elu+1");
        // D: dispatch (+1)
        assert!(ids.contains(&"layer-depth-decay-schedule"), "lightning_attn depth");
        // (J chunk-reset-scan was a duplicate of chunk-reset-cumsum — dropped.)
    }

    #[test]
    fn batch11_recurrence_concepts_covered() {
        // Verify the batch-11 recurrence + scan concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category G: linear-attention recurrence (new)
        assert!(concepts.contains(&"per-dimension diagonal decay gate on recurrent state"));
        assert!(concepts.contains(&"diagonal-plus-low-rank state transition (erase, decay, replace, write)"));
        assert!(concepts.contains(&"Oja subtractive normalization: project out stored component before write"));
        assert!(concepts.contains(&"identity-plus-low-rank (I+ba^T) state rotation then outer-product write"));
        assert!(concepts.contains(&"convert per-step log-gate into pairwise decay matrix via cumsum + diff"));
        // Category E: parallelism (new)
        assert!(concepts.contains(&"reverse-direction scan via flip → forward scan → flip"));
        assert!(concepts.contains(&"materialize N×N decay matrix from index distance and slope"));
        // Category F: flash-attn tiling (new)
        assert!(concepts.contains(&"strictly-causal mask via tril(-1) (exclude diagonal) + NaN guard"));
        // Category B: compute (new)
        assert!(concepts.contains(&"polynomial Taylor feature map approximating the softmax kernel"));
        assert!(concepts.contains(&"shift activation positive (elu+1) so linear-attention normalizer never crosses zero"));
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
        assert!(ids.contains(&"gate-then-correct-delta-rule"), "gated_delta_rule");
        assert!(ids.contains(&"gate-then-multi-householder-write"), "gated_delta_product");
        assert!(ids.contains(&"online-gram-ridge-solve"), "mesa_net ridge");
        assert!(ids.contains(&"momentum-gated-memory-recurrence"), "titans_linear momentum");
        assert!(ids.contains(&"current-token-bonus-read-before-update"), "rwkv6 bonus");
        assert!(ids.contains(&"wkv-logsumexp-three-channel-state"), "rwkv4 3ch");
        // B: compute (+3)
        assert!(ids.contains(&"hierarchical-log2-level-mask"), "log_linear_attn hier");
        assert!(ids.contains(&"iterative-cg-solve-over-matvec"), "mesa_net_decode cg");
        assert!(ids.contains(&"chunked-test-time-gradient-descent"), "ttt chunked gd");
        // F: flash-attn tiling (+2)
        assert!(ids.contains(&"block-local-reference-frame-for-overflow"), "wall_attn reframe");
        assert!(ids.contains(&"two-stage-mask-softmax-then-conv"), "multi_token_attention 2stage");
        // E: parallelism (+1)
        assert!(ids.contains(&"softmax-topk-renormalize-scatter"), "momoe gate");
        // C: fusion (+1)
        assert!(ids.contains(&"fuse-gate-transform-chunk-cumsum"), "kda_gate_cumsum fused");
        // D: dispatch (+1)
        assert!(ids.contains(&"dual-mode-gate-transform"), "kda_gate dual-mode");
    }

    #[test]
    fn batch12_recurrence_attention_concepts_covered() {
        // Verify the batch-12 recurrence + attention + gate concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category G: linear-attn recurrence (new)
        assert!(concepts.contains(&"gated delta rule: decay state then correct against decayed subspace"));
        assert!(concepts.contains(&"one outer gate wrapping N inner Householder delta writes per step"));
        assert!(concepts.contains(&"online ridge regression: gated Gram accumulators + per-step solve"));
        assert!(concepts.contains(&"two coupled recurrences: SGD momentum state + EMA memory matrix"));
        assert!(concepts.contains(&"read from boosted state (transient bonus), then update with un-boosted kv"));
        assert!(concepts.contains(&"WKV with 3-channel state (num/den/max) + logsumexp max-subtraction"));
        // Category B: compute (new)
        assert!(concepts.contains(&"O(log seq) hierarchical mask with per-level learned scales"));
        assert!(concepts.contains(&"conjugate-gradient solve over matvec with Jacobi preconditioner (no inverse)"));
        assert!(concepts.contains(&"mini-batch chunked GD with causal mask + last-step weight update"));
        // Category F: flash-attn tiling (new)
        assert!(concepts.contains(&"per-block reference frame to keep exp arguments bounded (avoid fp32 overflow)"));
        assert!(concepts.contains(&"two-stage mask: -inf before softmax, 0 after conv (probability + causal correct)"));
        // Category E: parallelism (new)
        assert!(concepts.contains(&"MoE gate: softmax → topk → renormalize → scatter sparse gate matrix"));
        // Category C: fusion (new)
        assert!(concepts.contains(&"fuse gate transform with chunk-local cumsum (no materialized intermediate)"));
        // Category D: dispatch (new)
        assert!(concepts.contains(&"dual-mode gate: unbounded softplus OR bounded sigmoid, dispatched by parameter"));
    }

    #[test]
    fn batch13_rules_present() {
        // Verify the 6 batch-13 rules across B/C/D.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
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
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category B: numerical stability (expm1 for KL cancellation)
        assert!(concepts.contains(&"numerically stable e^x − 1 via expm1 (avoids catastrophic cancellation near 0)"));
        // Category B: complex-free RoPE (paired reals)
        assert!(concepts.contains(&"represent complex numbers as interleaved [real, imag] pairs, multiply via 4 scalar muls"));
        // Category B: stateful shift (cross-batch carry)
        assert!(concepts.contains(&"shift sequence with cross-batch carry: cat(prev_state, x[:-1])"));
        // Category B: squared ReLU (sparse sharpening)
        assert!(concepts.contains(&"relu(x)² — sparse activation: preserves ReLU zeros, sharpens non-zeros quadratically"));
        // Category C: fused reduce-then-broadcast (RWKV-7 gate output correction)
        assert!(concepts.contains(&"fused reduce-then-broadcast: (r·k·ρ).sum(head_dim,keepdim)·v added to o, gated by g"));
        // Category D: multimodal cyclic table selection
        assert!(concepts.contains(&"multimodal RoPE: split head_dim into sections, cyclic gather from N modality tables"));
    }

    // ════════════════════════════════════════════════════════════════════
    // Batch 14 (Research 015) — riir-gpu production kernels: norm-fold + reduction-split + single-plane + CPU-offload
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn batch14_rules_present() {
        // Verify the 4 batch-14 rules across C/E/A.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category C (1 new): fold norm scalar into matmul
        assert!(ids.contains(&"fold-norm-scalar-into-matmul"), "fold norm into gemv");
        // Category E (2 new): reduction-split pre-pass + single-plane reduction
        assert!(ids.contains(&"split-reduction-scalar-prepass"), "scalar pre-pass");
        assert!(ids.contains(&"single-plane-sufficient-reduction"), "single plane");
        // Category A (1 new): offload CPU bottleneck to GPU kernel
        assert!(ids.contains(&"offload-cpu-bottleneck-to-gpu-kernel"), "cpu offload");
    }

    #[test]
    fn batch14_norm_fold_reduction_split_offload_concepts_covered() {
        // Verify the batch-14 norm-fold + reduction-split + single-plane + offload concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category A (1 new): block-contiguous weight layout
        assert!(ids.contains(&"block-contiguous-weight-layout"), "block-contiguous layout");
        // Category C (1 new): fuse gate+up GEMV + GeGLU
        assert!(ids.contains(&"fuse-gate-up-gemv-activation"), "fused dual GEMV + GeGLU");
        // Category B (1 new): pre-rotate query once
        assert!(ids.contains(&"pre-rotate-once-exploit-orthogonality"), "pre-rotate query");
        // Category N (1 new): water-fill tail-dim shared codebook
        assert!(ids.contains(&"water-fill-tail-dim-shared-codebook"), "water-fill tail-dim");
    }

    #[test]
    fn batch15_block_contiguous_fused_geglu_rotation_codebook_concepts_covered() {
        // Verify the batch-15 block-contiguous + fused-geglu + rotation + codebook concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category A: block-contiguous weight layout (riir-gpu gemm_ternary_block_contiguous)
        assert!(concepts.contains(&"pack multiple logical weight buffers (scale + bit-planes) into one block-contiguous buffer so one global-memory access fetches all components"));
        // Category C: fuse gate+up GEMV + GeGLU (riir-gpu gemv_geglu_f16)
        assert!(concepts.contains(&"fuse two sibling GEMVs (gate + up projections) + their multiplicative activation into one kernel dispatch"));
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category B (1 new): cmma hardware unit over scalar ALU
        assert!(ids.contains(&"cmma-hardware-unit-over-scalar-alu"), "cmma hardware unit");
        // Category A (1 new): dequant once per K-tile into shared memory
        assert!(ids.contains(&"dequant-once-per-k-tile-into-shared-memory"), "dequant staging");
        // Category D (1 new): swap large axis to X for workgroup distribution
        assert!(ids.contains(&"swap-large-axis-to-x-for-workgroup-distribution"), "dispatch axis swap");
        // Category N (1 new): norm-decoupled low-bit scoring
        assert!(ids.contains(&"norm-decoupled-low-bit-scoring"), "norm-decoupled scoring");
    }

    #[test]
    fn batch16_cmma_dequant_swap_normdecoupled_concepts_covered() {
        // Verify the batch-16 cmma + dequant-staging + axis-swap + norm-decoupled concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category B (3 new): zero-word OR-skip, split-sign accumulators, clamp-tile sentinel
        assert!(ids.contains(&"skip-all-zero-packed-words-via-or-reduce"), "zero-word or-skip");
        assert!(ids.contains(&"split-sign-accumulators-defer-subtraction"), "split-sign accumulators");
        assert!(ids.contains(&"clamp-tile-indices-to-sentinel-discard-at-write"), "clamp-tile sentinel");
        // Category A (1 new): hoist group-invariant scale
        assert!(ids.contains(&"hoist-group-invariant-scale-out-of-word-loop"), "scale hoist");
    }

    #[test]
    fn batch17_zeroword_scalehoist_splitsign_clampsentinel_concepts_covered() {
        // Verify the batch-17 zero-word-skip + scale-hoist + split-sign + clamp-sentinel concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category A (2 new): row-tile amortize input load + pin-lane coalesce
        assert!(ids.contains(&"row-tile-amortize-input-load"), "row-tile amortize input load");
        assert!(ids.contains(&"pin-lane-to-quant-sub-block-coalesce-byte-loads"), "pin-lane coalesce byte loads");
        // Category B (1 new): factor affine quant scale
        assert!(ids.contains(&"factor-affine-quant-scale-out-of-element-loop"), "factor affine quant scale");
    }

    #[test]
    fn batch18_rowtile_factoraffine_pinlane_concepts_covered() {
        // Verify the batch-18 row-tile + affine-factor + pin-lane concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category A: row-tile amortize input load (riir-gpu gemv_q4k_plane_rowtiled)
        assert!(concepts.contains(&"assign N consecutive output rows per plane (subgroup), hold N register accumulators, load each input element once and reuse across all N dot products — GEMV to small-GEMM via M-widening"));
        // Category B: factor affine quant scale (riir-gpu gemv_q4k_plane_factored)
        assert!(concepts.contains(&"when the dequant formula is affine in the quantized code within a sub-block (scale and min are constant), factor d*scale and dmin*min out of the per-element accumulation — accumulate sum(code*x) and sum(x) separately, apply scales once per sub-block"));
        // Category A: pin-lane coalesce byte loads (riir-gpu gemv_q4k_plane_factored)
        assert!(concepts.contains(&"remap the lane-element assignment so each lane is pinned to one quantization sub-block — the 8 consecutive bytes it needs fall in 2 consecutive u32 words (8 loads to 2), and scale/min become loop-invariant (enabling affine factoring)"));
    }

    #[test]
    fn batch19_rules_present() {
        // Verify the 2 batch-19 rules in Category C (fusion).
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category C (2 new): chain-dispatches-single-sync-point + fuse-residual-via-in-place-output-array
        assert!(ids.contains(&"chain-dispatches-single-sync-point"), "chain dispatches single sync point");
        assert!(ids.contains(&"fuse-residual-via-in-place-output-array"), "fuse residual via in-place output array");
    }

    #[test]
    fn batch19_chain_dispatch_inplace_residual_concepts_covered() {
        // Verify the batch-19 chain-dispatch + in-place-residual concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (1 new): cooperative-block-wide-quantize-staging
        assert!(ids.contains(&"cooperative-block-wide-quantize-staging"), "cooperative block-wide quantize staging");
        // B (3 new): byte-perm-lut-via-permutation-immediate + dp4a-int8-hardware-dot-product + threadgroup-subset-sum-lut-keyed-by-activation
        assert!(ids.contains(&"byte-perm-lut-via-permutation-immediate"), "byte perm lut via permutation immediate");
        assert!(ids.contains(&"dp4a-int8-hardware-dot-product"), "dp4a int8 hardware dot product");
        assert!(ids.contains(&"threadgroup-subset-sum-lut-keyed-by-activation"), "threadgroup subset-sum lut keyed by activation");
        // N (1 new): base-3-trit-packing-5-per-byte
        assert!(ids.contains(&"base-3-trit-packing-5-per-byte"), "base-3 trit packing 5 per byte");
    }

    #[test]
    fn batch20_cuda_dp4a_lut_trit_concepts_covered() {
        // Verify the batch-20 CUDA-dp4a + LUT + trit concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (4 new): cross-runtime-raw-handle-extraction + pool-buffer-aliasing-dedicated-output + same-queue-implicit-ordering-no-sync + per-dispatch-pointer-re-extraction
        assert!(ids.contains(&"cross-runtime-raw-handle-extraction"), "cross-runtime raw handle extraction");
        assert!(ids.contains(&"pool-buffer-aliasing-dedicated-output"), "pool-buffer aliasing dedicated output");
        assert!(ids.contains(&"same-queue-implicit-ordering-no-sync"), "same-queue implicit ordering no sync");
        assert!(ids.contains(&"per-dispatch-pointer-re-extraction"), "per-dispatch pointer re-extraction");
        // D (1 new): no-blocking-sync-in-async-pipeline
        assert!(ids.contains(&"no-blocking-sync-in-async-pipeline"), "no-blocking-sync-in-async-pipeline");
    }

    #[test]
    fn batch21_metal_cross_runtime_concepts_covered() {
        // Verify the batch-21 Metal cross-runtime integration concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (2 new): dynamic-shared-memory-runtime-sized + ternary-bit-plane-u64-as-2x-u32
        assert!(ids.contains(&"dynamic-shared-memory-runtime-sized"), "dynamic shared memory runtime sized");
        assert!(ids.contains(&"ternary-bit-plane-u64-as-2x-u32"), "ternary bit plane u64 as 2x u32");
        // B (1 new): warp-shuffle-intrinsics-for-sub-warp-reduction
        assert!(ids.contains(&"warp-shuffle-intrinsics-for-sub-warp-reduction"), "warp shuffle intrinsics for sub warp reduction");
        // C (1 new): fused-activation-quantize-for-gemv-consumer
        assert!(ids.contains(&"fused-activation-quantize-for-gemv-consumer"), "fused activation quantize for gemv consumer");
        // D (1 new): cuda-graph-scalar-via-device-pointer
        assert!(ids.contains(&"cuda-graph-scalar-via-device-pointer"), "cuda graph scalar via device pointer");
    }

    #[test]
    fn batch22_cudarc_kernels_concepts_covered() {
        // Verify the batch-22 cudarc_kernels CUDA cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // A: dynamic-shared-memory-runtime-sized (riir-gpu cudarc_kernels/attention.rs L265-301)
        assert!(concepts.contains(&"use extern __shared__ with launch-time shared_mem_bytes instead of a compile-time __shared__ array when the smem size depends on a runtime dimension (e.g. head_dim varies: 64 for DeltaNet, 128 for Qwen-7B, 256 for Bonsai-27B)"));
        // A: ternary-bit-plane-u64-as-2x-u32 (riir-gpu cudarc_kernels/embedding.rs L37-69)
        assert!(concepts.contains(&"ternary weights as two bit-planes (positive + negative sign bits) packed as u64 blocks cast to 2×u32 words — extract sign per column via bit_pos = c % 32, word_in_block = (c % 64) / 32, then output = sign * per-group scale. The 2×u32 cast is because u64 isn't directly per-bit addressable in CUDA; two u32 words are"));
        // B: warp-shuffle-intrinsics-for-sub-warp-reduction (riir-gpu cudarc_kernels/mod.rs L163-167 + attention.rs L336-344)
        assert!(concepts.contains(&"for blocks of 32 threads or fewer (one warp), use __shfl_down_sync(mask, val, delta) butterfly reduction instead of shared memory + __syncthreads — the warp executes in lockstep (SIMT), so the shuffle is both correct and free (no smem allocation, no synchronization barrier). For the final 32-thread tail of a larger smem tree reduction, cast to volatile float* and skip __syncthreads — volatile prevents the compiler from caching stale values in registers"));
        // C: fused-activation-quantize-for-gemv-consumer (riir-gpu cudarc_kernels/mod.rs L191-289 + L407-554)
        assert!(concepts.contains(&"when an activation's output (RMSNorm, SwiGLU, gate*sigmoid, gate*silu) is immediately consumed by a GEMV that reads int8 (not f32), fuse the activation + per-block absmax reduction + int8 quantize into one kernel — eliminates the intermediate f32 HBM round-trip and one kernel launch. The activation is the non-trivial part (needs a reduction or exp); the quantize is the trivial part (absmax + scale + round)"));
        // D: cuda-graph-scalar-via-device-pointer (riir-gpu cudarc_kernels/embedding.rs L76-105 + attention.rs L421-610)
        assert!(concepts.contains(&"when capturing a CUDA graph, scalar kernel args (int pos, int row_idx) get baked into the captured graph at capture time — the graph replays the exact value on every launch and cannot change. Ship a _devpos variant that reads the scalar from a const device pointer dereferenced at kernel runtime; the caller writes the new value to a 4-byte device buffer via memcpy_htod on the same stream before graph.launch(), so the captured graph processes a different input each call"));
    }

    #[test]
    fn batch23_rules_present() {
        // Verify the 5 batch-23 rules across A/B/D.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (2 new): combined-kv-buffer-keys-then-values + gqa-kv-head-sharing-bandwidth-reduction
        assert!(ids.contains(&"combined-kv-buffer-keys-then-values"), "combined kv buffer keys then values");
        assert!(ids.contains(&"gqa-kv-head-sharing-bandwidth-reduction"), "gqa kv head sharing bandwidth reduction");
        // B (1 new): softcap-tanh-attention-score
        assert!(ids.contains(&"softcap-tanh-attention-score"), "softcap tanh attention score");
        // D (2 new): fold-query-tokens-for-occupancy + hardcoded-fast-path-jit-specialization
        assert!(ids.contains(&"fold-query-tokens-for-occupancy"), "fold query tokens for occupancy");
        assert!(ids.contains(&"hardcoded-fast-path-jit-specialization"), "hardcoded fast path jit specialization");
    }

    #[test]
    fn batch23_attention_cubecl_concepts_covered() {
        // Verify the batch-23 attention_cubecl flash-attention cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // A: combined-kv-buffer-keys-then-values (riir-gpu attention_cubecl/mod.rs L107-140 + L576-650)
        assert!(concepts.contains(&"store keys and values as a single contiguous buffer `[keys | values]` with shared stride — halves buffer bindings from 2 to 1 and enables coalesced access via a single kv_off index. The value for position p at dimension d is at kv[kv_half + p * kv_stride + d], where kv_half = kv.len() / 2. Distinct from KV cache layout: this is about the buffer binding shape, not the cache update logic"));
        // A: gqa-kv-head-sharing-bandwidth-reduction (riir-gpu attention_cubecl/mod.rs L150 + L345 + L870)
        assert!(concepts.contains(&"Grouped-Query Attention (GQA): share KV heads across multiple query heads (n_kv_head < n_head) to reduce KV cache memory + bandwidth by n_head/n_kv_head× (e.g. 2× for Gemma 2 2B's 8:4 ratio). Each query head maps to its KV group via kv_group = head_idx * n_kv_head / n_head; the kernel computes kv_off = kv_group * head_dim once per workgroup and reuses it for all K/V reads in that head. Distinct from MHA (1:1) and MQA (all heads share 1 KV head)"));
        // B: softcap-tanh-attention-score (riir-gpu attention_cubecl/mod.rs L188-191 + L373-376 + L894-897)
        assert!(concepts.contains(&"bound raw attention logits to [-softcap, +softcap] via softcap * tanh(score / softcap) — a smooth, differentiable alternative to clamping that prevents extreme Q·K scores from dominating softmax. tanh has a well-defined gradient everywhere (unlike clamp's discontinuous gradient at the boundary). Configurable: softcap == 0.0 disables softcapping → raw scaled dot product (LLaMA/MiniCPM path). Used by Gemma 2 (softcap=50.0)"));
        // D: fold-query-tokens-for-occupancy (riir-gpu attention_cubecl/mod.rs L336-576 + L1552-1573)
        assert!(concepts.contains(&"when n_head < SM_count (small models, speculative decode with seq_len_q > 1), fold multiple query tokens into one workgroup to increase GPU occupancy. Each workgroup processes fold_factor query tokens sequentially for one head. fold_factor(n_head, seq_len_q, workgroup_size) picks the largest divisor of seq_len_q ≤ workgroup_size / n_head. Benefit: (n_head × fold) workgroups vs n_head → fills idle SMs during speculative decode"));
        // D: hardcoded-fast-path-jit-specialization (riir-gpu attention_cubecl/mod.rs L1313-1342)
        assert!(concepts.contains(&"when kernel params match a known config (e.g. Gemma 2 2B: n_head=8, n_kv_head=4, head_dim=256, softcap=50.0), dispatch a hardcoded variant that bakes the constants into the kernel body — the JIT compiler can then constant-fold dimensions into optimized shaders (unrolled loops, known stride multiplies, eliminated per-tile branching). The generic parametric kernel reads dims from a buffer per dispatch, preventing constant-folding. Measured gain: ~24% decode throughput (31.4 → 41.4 tok/s on M3 Max Metal). Distinct from autotune-warps-stages (sweeps configs) — here the config is matched, not searched"));
    }

    #[test]
    fn batch24_rules_present() {
        // Verify the 4 batch-24 rules across A/B/C/E.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (1 new): widen-index-multiply-to-64bit
        assert!(ids.contains(&"widen-index-multiply-to-64bit"), "widen index multiply to 64bit");
        // B (1 new): dual-accumulator-single-reduction-sweep
        assert!(ids.contains(&"dual-accumulator-single-reduction-sweep"), "dual accumulator single reduction sweep");
        // C (1 new): dual-grad-same-pass
        assert!(ids.contains(&"dual-grad-same-pass"), "dual grad same pass");
        // E (1 new): broadcast-uniform-skip-reduce
        assert!(ids.contains(&"broadcast-uniform-skip-reduce"), "broadcast uniform skip reduce");
    }

    #[test]
    fn batch24_cudarc_backward_concepts_covered() {
        // Verify the batch-24 cudarc_kernels backward cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // D (2 new): device-side-argmax-minimal-download + hot-swap-weights-preserve-autotune-caches
        assert!(ids.contains(&"device-side-argmax-minimal-download"), "device side argmax minimal download");
        assert!(ids.contains(&"hot-swap-weights-preserve-autotune-caches"), "hot swap weights preserve autotune caches");
        // B (2 new): monotone-transform-elision-under-argmax + tree-reduction-depth-over-sequential-accumulate
        assert!(ids.contains(&"monotone-transform-elision-under-argmax"), "monotone transform elision under argmax");
        assert!(ids.contains(&"tree-reduction-depth-over-sequential-accumulate"), "tree reduction depth over sequential accumulate");
        // C (1 new): fan-out-shared-input-single-sync
        assert!(ids.contains(&"fan-out-shared-input-single-sync"), "fan out shared input single sync");
    }

    #[test]
    fn batch25_gemma_cluster_concepts_covered() {
        // Verify the batch-25 Gemma cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // D: device-side-argmax-minimal-download (riir-gpu gemma2_cubecl/gpu_forward.rs L319-346 + L348-423)
        assert!(concepts.contains(&"when the host needs only a tiny derived result of a large GPU array (argmax index, top-k, threshold count), compute it ON DEVICE with a reduction kernel and download only the tiny result — 4 bytes (1 u32) instead of the full array (vocab_size × 4 ≈ 1 MB at 256K vocab). Greedy decode is the canonical consumer: ~250,000× smaller per-token boundary crossing; a 50-prompt + 500-decode session downloads ~50 MB instead of ~550 MB. Distinct from offload-cpu-bottleneck-to-gpu-kernel (CPU work moved to GPU) — the CPU side isn't slow, the DOWNLOAD feeding it is the cost; and from chain-dispatches-single-sync-point (sync count) — this is download size"));
        // B: monotone-transform-elision-under-argmax (riir-gpu gemma2_cubecl/gpu_forward.rs L208-217)
        assert!(concepts.contains(&"a strictly monotone post-transform (softcap cap·tanh(x/cap), clamps, positive rescalings) does not change the argmax — skip it entirely when the only consumer is greedy decoding / top-k / any order-only query. The GPU decode path returns the raw logits WITHOUT final logit softcapping because tanh is monotonically increasing: argmax(softcap(x)) == argmax(x); the value-consuming CPU path still applies it. Elides a full-array transcendental pass (256K tanh evals per decode token). Distinct from softcap-tanh-attention-score (applying softcap in-kernel) — this elides it under order-only consumers; distinct from optional-bias-branch (absence) — the transform exists but order is invariant under it"));
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // D (5 new): the dispatch-side hazard + orchestration family
        assert!(ids.contains(&"per-slot-uniform-buffer-write-ordering"), "per slot uniform buffer write ordering");
        assert!(ids.contains(&"derive-params-from-global-invocation-id"), "derive params from global invocation id");
        assert!(ids.contains(&"merge-dispatches-into-single-compute-pass"), "merge dispatches into single compute pass");
        assert!(ids.contains(&"batched-downloads-single-sync-cycle"), "batched downloads single sync cycle");
        assert!(ids.contains(&"chunk-encoder-submits-below-driver-limit"), "chunk encoder submits below driver limit");
        // A (3 new): the host-side resource discipline family
        assert!(ids.contains(&"prebuilt-bind-groups-fixed-buffer-combos"), "prebuilt bind groups fixed buffer combos");
        assert!(ids.contains(&"host-cache-immutable-device-weights"), "host cache immutable device weights");
        assert!(ids.contains(&"defer-readback-until-after-submit"), "defer readback until after submit");
    }

    #[test]
    fn batch26_wgpu_forward_concepts_covered() {
        // Verify the batch-26 wgpu forward-orchestration concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // D: per-slot-uniform-buffer-write-ordering (riir-gpu forward/mod.rs L141-148 + L361-390 + forward_buffers.rs L550-557)
        assert!(concepts.contains(&"host-side queue.write_buffer calls all flush BEFORE the recorded encoder executes — N dispatches in one encoder sharing ONE uniform buffer all read the LAST write (per-head attention params, per-target LoRA params, per-slot matmul params silently corrupted; the Issues 366/367/368 lineage). Give each dispatch slot its own small uniform buffer (per head / per adapter / per target / per QKV slot), each wired into its own pre-built bind group, so concurrent writes in one encoder cannot collide. Distinct from same-queue-implicit-ordering-no-sync (queue-level dispatch ordering) — this is the uniform-write channel racing the encoder; the in-kernel escape is derive-params-from-global-invocation-id"));
        // D: derive-params-from-global-invocation-id (riir-gpu forward/batched_dispatch.rs L280-350)
        assert!(concepts.contains(&"instead of writing a per-item parameter (query position, head index, row offset) to a uniform from the host — which under the write_buffer ordering bug forces an encoder + submit PER ITEM — encode the parameter in the dispatch grid: flatten (head, item) into workgroup IDs and derive it in-kernel from global_invocation_id. The batched attention path dispatches n_head × seq_len workgroups in ONE call with causal masking + GQA mapping computed inside the kernel — zero per-position uniform writes, zero per-position submits (vs the encoder-per-position fallback paying ~1 submit/position). Distinct from fold-query-tokens-for-occupancy (goal: fill idle SMs — here the goal is eliminating the host-side parameter channel) and from grid-stride-loop (iteration coverage — the grid IS the parameter)"));
        // D: merge-dispatches-into-single-compute-pass (riir-gpu forward/adapter_merge.rs L410-424 + L654-770)
        assert!(concepts.contains(&"dependent dispatches queued in ONE compute pass (set_pipeline/set_bind_group/dispatch × N inside a single begin/end) execute in order with memory ordering between them — the consumer dispatch reads the producer's writes without a separate pass, and N−1 pass begin/end boundaries disappear. The QKV LoRA path merges 6 dispatches (3× A pass + 3× B pass) into one pass; pure input-save copies ride the copy channel (encoder.copy_buffer_to_buffer) outside any pass. Distinct from chain-dispatches-single-sync-point (command-buffer-level sync amortization) — this is pass-level merging; composes with per-slot-uniform-buffer-write-ordering (all uniform writes precede the pass, into per-slot buffers)"));
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // D (3 new): the uniform-distribution + destination family
        assert!(ids.contains(&"staging-packed-uniforms-encoder-ordered-copies"), "staging packed uniforms encoder ordered copies");
        assert!(ids.contains(&"identical-uniforms-share-single-slot"), "identical uniforms share single slot");
        assert!(ids.contains(&"per-layer-strided-grad-destination"), "per layer strided grad destination");
        // C (1 new): the sibling-fusion family
        assert!(ids.contains(&"fuse-sibling-target-grads-single-kernel"), "fuse sibling target grads single kernel");
        // A (2 new): the buffer-lifetime family
        assert!(ids.contains(&"persistent-zeros-source-encoder-copy-clear"), "persistent zeros source encoder copy clear");
        assert!(ids.contains(&"snapshot-transient-grad-for-deferred-download"), "snapshot transient grad for deferred download");
    }

    #[test]
    fn batch27_backward_concepts_covered() {
        // Verify the batch-27 backward-orchestration concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // G (2 new): the deltanet decode/prefill recurrence family
        assert!(ids.contains(&"row-owned-register-blocked-recurrence"), "row owned register blocked recurrence");
        assert!(ids.contains(&"reverse-running-decay-weight"), "reverse running decay weight");
        // E (2 new): the plane/reduction + broadcast-scatter family
        assert!(ids.contains(&"workgroup-equals-plane-native-reduction"), "workgroup equals plane native reduction");
        assert!(ids.contains(&"smem-broadcast-scalar-then-parallel-scatter"), "smem broadcast scalar then parallel scatter");
        // D (3 new): the dispatch-shape family
        assert!(ids.contains(&"token-flattened-elementwise-modulo-shared-weights"), "token flattened elementwise modulo shared weights");
        assert!(ids.contains(&"split-decay-outputs-eliminate-readback"), "split decay outputs eliminate readback");
        assert!(ids.contains(&"nonlinearity-forces-raw-input-buffers"), "nonlinearity forces raw input buffers");
        // C (1 new), B (1 new), L (1 new)
        assert!(ids.contains(&"fused-conv-state-shift-dot-silu"), "fused conv state shift dot silu");
        assert!(ids.contains(&"softplus-piecewise-saturation-clamp"), "softplus piecewise saturation clamp");
        assert!(ids.contains(&"carry-dual-layout-dead-slot"), "carry dual layout dead slot");
    }

    #[test]
    fn batch28_deltanet_concepts_covered() {
        // Verify the batch-28 deltanet CubeCL cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (2 new): the readback/size-gate + handle-cache family
        assert!(ids.contains(&"defer-readback-gated-by-live-output-size"), "defer readback gated by live output size");
        assert!(ids.contains(&"weight-handle-cache-keyed-by-allocation-identity"), "weight handle cache keyed by allocation identity");
        // D (3 new): the sync-cost-model + launch-path family
        assert!(ids.contains(&"first-readback-pays-queue-drain"), "first readback pays queue drain");
        assert!(ids.contains(&"pack-scalar-params-into-combined-buffer"), "pack scalar params into combined buffer");
        assert!(ids.contains(&"launch-unchecked-caller-owned-size-contract"), "launch unchecked caller owned size contract");
        // C (2 new): the CODA epilogue family
        assert!(ids.contains(&"epilogue-at-register-write-phase"), "epilogue at register write phase");
        assert!(ids.contains(&"delay-rms-scale-past-gemv-epilogue"), "delay rms scale past gemv epilogue");
        // E (1 new), B (1 new), M (1 new)
        assert!(ids.contains(&"epilogue-must-be-workgroup-local"), "epilogue must be workgroup local");
        assert!(ids.contains(&"fast-math-contraction-behind-feature-flag"), "fast math contraction behind feature flag");
        assert!(ids.contains(&"enum-dispatch-separate-kernel-per-variant"), "enum dispatch separate kernel per variant");
    }

    #[test]
    fn batch29_concepts_covered() {
        // Verify the batch-29 decode-path fusion cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // A: defer-readback-gated-by-live-output-size (riir-gpu deltanet_input_proj_fused.rs L17-32 — Bench 437 contrast)
        assert!(concepts.contains(&"deferring all sibling readbacks to one final drain is NOT an unconditional win — gate it on total simultaneously-live output bytes. When the chained outputs are small (e.g. 4 input projections, ~8 KB total), keeping them all GPU-resident to the end is free and one sync point replaces N; when they are large (5 concurrent ~70 KB FFN buffers, ~350 KB live), the deferral itself increases memory pressure and measured 16% SLOWER on Metal — sync per stage and release each buffer instead. The rule converts 'defer readback' from an always-apply transformation into a footprint-gated one; the existing deferral rules carry the timing/mechanism, this carries the failure boundary. Distinct from defer-readback-until-after-submit (the timing rule — read after submit; this is the precondition that decides whether deferral helps at all) and from chain-dispatches-single-sync-point (the dispatch-side mechanism — this is the output-sizing decision that makes the chaining a win rather than a loss)"), "size-gate concept");
        // A: weight-handle-cache-keyed-by-allocation-identity (riir-gpu deltanet_input_proj_fused.rs L279-281 + L301-327 + L361-372)
        assert!(concepts.contains(&"when a GPU offload hook's public API must accept plain CPU weight references (it cannot require callers to thread device handles through existing model code), key a lock-free host-side cache of pre-uploaded device handles by the CPU weight allocation's POINTER IDENTITY — e.g. (pos_bits.as_ptr(), neg_bits.as_ptr()), unique per layer allocation. The hot path is one O(1) map lookup; a cache miss degrades gracefully to an on-the-fly upload + insert, so the hook works with or without a preupload warm-up and never blocks adoption. The keying is what lets the cache be invisible to the API surface. Distinct from host-cache-immutable-device-weights (the upload-once rule — this is the identity-keyed transparent lookup layered on top) and from per-dispatch-pointer-re-extraction (device pointers re-read per dispatch — CPU heap pointers as a persistent key, opposite direction)"), "pointer-identity cache concept");
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
        assert!(concepts.contains(&"a numerics-affecting compiler flag (fast-math contraction) must not be enabled globally — it changes rounding everywhere and breaks bit-comparisons and tolerance gates — nor disabled forever (the hot dot-product loop stays uncontracted mul+add on every build). Attach the flag to the KERNEL ATTRIBUTE via a cfg_attr pair gated on a dedicated Cargo feature: the attribute-identical strict-FP compilation stays the byte-identical default, correctness/equivalence gates run strict, and perf builds opt in. The partial += w[k] * x[k] accumulation contracts to FMA only on the opt-in compilation of the same source. Distinct from dtype-specialized-float-kind (dtype specialization — this is compiler-numerics-mode specialization) and from in-kernel numeric transforms like softplus-piecewise-saturation-clamp (algorithm changes — this gates a compiler flag with zero algorithm change)"), "fma-flag concept");
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
            assert!(ref_verdict.parses, "batch-29 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-29 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch30_rules_present() {
        // Verify the 6 batch-30 rules across C/D/E/M/N.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // M (1 new): the zero-init frozen-teacher identity
        assert!(ids.contains(&"zero-init-side-branch-frozen-base-path"), "zero init side branch frozen base path");
        // E (1 new): the counter-derived stateless seed
        assert!(ids.contains(&"counter-derived-seed-stateless-noise"), "counter derived seed stateless noise");
        // D (2 new): the empty-workset gate + shared pipeline set
        assert!(ids.contains(&"predicate-gated-skip-empty-workset-step"), "predicate gated skip empty workset step");
        assert!(ids.contains(&"share-pipeline-set-across-subsystems"), "share pipeline set across subsystems");
        // N (1 new): the 16-byte uniform pad discipline
        assert!(ids.contains(&"pad-uniform-pod-to-min-binding-size"), "pad uniform pod to min binding size");
        // C (1 new): the fused count
        assert!(ids.contains(&"fuse-count-into-mutation-pass"), "fuse count into mutation pass");
    }

    #[test]
    fn batch30_concepts_covered() {
        // Verify the batch-30 D2F training cluster concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
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
            assert!(ref_verdict.parses, "batch-30 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-30 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch31_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // D (1 new): ragged packed-buffer fused dispatch
        assert!(ids.contains(&"ragged-pack-offset-table-fused-dispatch"), "ragged pack offset table");
        // L (1 new): SharedKV twin-projection adapter
        assert!(ids.contains(&"twin-projection-shared-delta-adapter"), "twin projection shared delta");
        // M (2 new): min-rank inert adapter + debug-only label macro
        assert!(ids.contains(&"min-rank-inert-adapter-zero-alpha"), "min-rank inert adapter");
        assert!(ids.contains(&"debug-only-gpu-label-macro"), "debug-only gpu label macro");
        // A (3 new): sync result cell + grow-only staging + mapped-at-creation upload
        assert!(ids.contains(&"sync-map-result-stack-cell"), "sync map result stack cell");
        assert!(ids.contains(&"staging-reuse-grow-only-map-prefix"), "staging reuse grow only map prefix");
        assert!(ids.contains(&"mapped-at-creation-init-upload"), "mapped at creation init upload");
        // N (1 new): dtype normalize once at upload
        assert!(ids.contains(&"dtype-normalize-once-at-upload"), "dtype normalize once at upload");
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
            assert!(ref_verdict.parses, "batch-31 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-31 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch32_rules_present() {
        // Verify the 11 batch-32a rules across A/B/C/D/H/L.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (3 new): scratch/alloc/hash family
        assert!(ids.contains(&"caller-owned-split-scratch"), "caller owned split scratch");
        assert!(ids.contains(&"incremental-multi-part-hash-no-concat"), "incremental multi part hash");
        assert!(ids.contains(&"superset-sized-scratch-across-variants"), "superset sized scratch");
        // B (3 new): compute/numerics family
        assert!(ids.contains(&"sign-branch-stable-sigmoid"), "sign branch stable sigmoid");
        assert!(ids.contains(&"per-layer-grad-norm-lr-equalization"), "per layer grad norm lr");
        assert!(ids.contains(&"inplace-grad-accumulate-deferred-postscale"), "inplace grad deferred postscale");
        // C (2 new): fusion family
        assert!(ids.contains(&"inplace-gate-scale-at-funnel-bottleneck"), "inplace gate at bottleneck");
        assert!(ids.contains(&"fuse-loss-and-output-grad-one-pass"), "fuse loss and grad one pass");
        // D (1 new), H (1 new), L (1 new)
        assert!(ids.contains(&"width-invariant-superset-training"), "width invariant superset");
        assert!(ids.contains(&"active-index-grad-scatter-subset-backward"), "active index grad scatter");
        assert!(ids.contains(&"bool-fields-over-bitset-hot-reads"), "bool fields over bitset");
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
        assert!(c.contains("split_at_mut (the borrow checker proves the regions disjoint")
            && c.contains("quarantine it as a documented cold adapter"), "split-scratch concept");
        // C: inplace-gate-scale-at-funnel-bottleneck (edge_module.rs L203-212)
        let c = concept("inplace-gate-scale-at-funnel-bottleneck");
        assert!(c.contains("IN PLACE on the reused rank-wide intermediate and absorb any broadcast scale")
            && c.contains("placement-of-work at the funnel's narrowest point"), "bottleneck gate concept");
        // B: sign-branch-stable-sigmoid (edge_module.rs L182-186 + L328-337)
        let c = concept("sign-branch-stable-sigmoid");
        assert!(c.contains("exp's argument is always <= 0"), "sign-branch anchor");
        assert!(c.contains("overflows to +inf past ~88.7"), "sigmoid overflow anchor");
        // A: incremental-multi-part-hash-no-concat (types.rs L476-487 + L381)
        let c = concept("incremental-multi-part-hash-no-concat");
        assert!(c.contains("incremental update API") && c.contains("the streaming update IS the concatenation"), "streaming hash concept");
        // L: bool-fields-over-bitset-hot-reads (types.rs L35-45 + L69-84)
        let c = concept("bool-fields-over-bitset-hot-reads");
        assert!(c.contains("REGIME BOUNDARY of packing") && c.contains("read frequency dominates"), "bool-regime concept");
        // B: per-layer-grad-norm-lr-equalization (training.rs L388-392 + L441-449 + L510-513)
        let c = concept("per-layer-grad-norm-lr-equalization");
        assert!(c.contains("~1000\u{d7}") || c.contains("~1000x"), "vanishing-grad anchor");
        assert!(c.contains("min(max_norm / layer_norm, cap)") && c.contains("never NaN"), "capped-equalization concept");
        // D: width-invariant-superset-training (adaptive_width.rs L2-6 + L20-23 + L407-409)
        let c = concept("width-invariant-superset-training");
        assert!(c.contains("width-1 must retain >= 95% of width-4 quality") && c.contains("anytime-inference with zero extra storage"), "superset-training concept");
        // A: superset-sized-scratch-across-variants (adaptive_width.rs L202-210 + L216-218)
        let c = concept("superset-sized-scratch-across-variants");
        assert!(c.contains("STATICALLY KNOWN MAXIMUM") && c.contains("exact length equality"), "worst-case scratch concept");
        // H: active-index-grad-scatter-subset-backward (adaptive_width.rs L44-46 + L329-335)
        let c = concept("active-index-grad-scatter-subset-backward");
        assert!(c.contains("zero BY CONSTRUCTION") && c.contains("host-side shape of MoE backward"), "index-scatter concept");
        // B: inplace-grad-accumulate-deferred-postscale (training.rs L297 + L325-336)
        let c = concept("inplace-grad-accumulate-deferred-postscale");
        assert!(c.contains("ONE post-loop sweep") && c.contains("one division total"), "deferred-postscale concept");
        // C: fuse-loss-and-output-grad-one-pass (training.rs L312-320)
        let c = concept("fuse-loss-and-output-grad-one-pass");
        assert!(c.contains("the SAME diff") && c.contains("standard fused-loss-grad shape"), "fused-loss concept");
    }

    #[test]
    fn batch32b_rules_present() {
        // Verify the 10 batch-32b rules across A/B/C/D/E.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (4 new): activation-cache/temp/init/shape-truth family
        assert!(ids.contains(&"sigmoid-deriv-recovered-from-output"), "sigmoid deriv from output");
        assert!(ids.contains(&"needless-swap-temp-hoist-to-scratch"), "needless swap temp");
        assert!(ids.contains(&"scoped-init-at-consume-site"), "scoped init at consume site");
        assert!(ids.contains(&"derive-shape-from-payload-not-cached-field"), "shape from payload");
        // B (2 new): devirt + digest family
        assert!(ids.contains(&"monomorphize-per-element-trait-calls"), "monomorphize trait calls");
        assert!(ids.contains(&"commit-over-maintained-item-digests"), "commit over digests");
        // C (1 new), D (2 new), E (1 new)
        assert!(ids.contains(&"fuse-weight-grad-input-grad-single-matrix-visit"), "fused B visit");
        assert!(ids.contains(&"static-classification-hoist-to-routing-table"), "classification hoist");
        assert!(ids.contains(&"verify-once-at-trust-boundary"), "verify once");
        assert!(ids.contains(&"additive-sigmoid-gates-not-softmax-routing"), "additive sigmoid gates");
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
        assert!(c.contains("classification work per training step for metadata that never changes")
            && c.contains("only its active items with O(1) lookups"), "routing-table hoist concept");
        // A: sigmoid-deriv-recovered-from-output (topology_trainer.rs L966-987 + L171-182)
        let c = concept("sigmoid-deriv-recovered-from-output");
        assert!(c.contains("pure function of the OUTPUT")
            && c.contains("halves activation-cache traffic"), "deriv-recovery concept");
        // C: fuse-weight-grad-input-grad-single-matrix-visit (topology_trainer.rs L932-963)
        let c = concept("fuse-weight-grad-input-grad-single-matrix-visit");
        assert!(c.contains("one visit per B row")
            && c.contains("the outer-product write and the transposed-read accumulation"), "fused-B-visit concept");
        // A: needless-swap-temp-hoist-to-scratch (topology_trainer.rs L627-639 + L758-769)
        let c = concept("needless-swap-temp-hoist-to-scratch");
        assert!(c.contains("false premise that motivated the temp")
            && c.contains("loop-carried state and belongs in the persistent scratch"), "swap-temp concept");
        // A: scoped-init-at-consume-site (topology_trainer.rs L400-421 + L581-594)
        let c = concept("scoped-init-at-consume-site");
        assert!(c.contains("scoped to the window actually accumulated into")
            && c.contains("O(total scratch) to O(active window)"), "scoped-init concept");
        // B: monomorphize-per-element-trait-calls (topology_trainer.rs L558-563 + L812-819)
        let c = concept("monomorphize-per-element-trait-calls");
        assert!(c.contains("strict generalization, not a fork")
            && c.contains("one vtable indirection per node"), "monomorphize concept");
        // E: additive-sigmoid-gates-not-softmax-routing (sigmoid_gate.rs L3-16 + L70-83)
        let c = concept("additive-sigmoid-gates-not-softmax-routing");
        assert!(c.contains("cross-edge synchronization barrier")
            && c.contains("embarrassingly parallel"), "additive-gates concept");
        // B: commit-over-maintained-item-digests (chain_commit.rs L66-76 + L83-92)
        let c = concept("commit-over-maintained-item-digests");
        assert!(c.contains("zero payload re-reads")
            && c.contains("only at trust boundaries"), "digest-hierarchy concept");
        // D: verify-once-at-trust-boundary (chain_commit.rs L156-166 + L236-243)
        let c = concept("verify-once-at-trust-boundary");
        assert!(c.contains("the type carries the proof")
            && c.contains("where untrusted bytes become trusted state"), "verify-once concept");
        // A: derive-shape-from-payload-not-cached-field (sigmoid_gate.rs L31-38 + L57-61)
        let c = concept("derive-shape-from-payload-not-cached-field");
        assert!(c.contains("ONE source of shape truth")
            && c.contains("structurally impossible"), "payload-shape-truth concept");
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
            assert!(ref_verdict.parses, "batch-32 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-32 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-32b ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-32b opt_pattern for '{id}' must parse");
        }
    }


    #[test]
    fn batch32c_rules_present() {
        // Verify the 13 batch-32c rules across A/B/C/D/E.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (3 new): state-search / projection-table / capture-footprint family
        assert!(ids.contains(&"make-unmake-over-clone-per-candidate"), "make/unmake over clone");
        assert!(ids.contains(&"deterministic-tiling-projection-tables"), "tiling projection tables");
        assert!(ids.contains(&"latent-encode-at-capture"), "latent encode at capture");
        // B (3 new): hash-shape / tie-break / reward-floor family
        assert!(ids.contains(&"length-prefix-shape-faithful-hash"), "length-prefix hash");
        assert!(ids.contains(&"argmax-rng-tie-break-collect-ties"), "argmax rng tie-break");
        assert!(ids.contains(&"non-saturating-outcome-reward-floor"), "outcome reward floor");
        // C (2 new): aggregate-maintenance / one-sweep family
        assert!(ids.contains(&"incremental-centroid-running-sum"), "incremental centroid");
        assert!(ids.contains(&"one-sweep-spatial-feature-fanout"), "one-sweep feature fanout");
        // D (4 new): selection-order / warm-start / data-not-fork / timeout family
        assert!(ids.contains(&"order-stable-topk-via-index-sort"), "order-stable topk");
        assert!(ids.contains(&"shape-gated-warm-start-slot-copy"), "shape-gated warm start");
        assert!(ids.contains(&"domain-as-data-not-algorithm-fork"), "domain as data");
        assert!(ids.contains(&"timeout-scalar-tiebreak-decisive-outcome"), "timeout scalar tiebreak");
        // E (1 new)
        assert!(ids.contains(&"collect-then-resolve-snapshot-batch"), "collect-then-resolve");
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
        assert!(c.contains("append-only aggregate")
            && c.contains("never recomputed from the member list at query time"), "centroid concept");
        // B: length-prefix-shape-faithful-hash (sleep_consolidation.rs L404-415)
        let c = concept("length-prefix-shape-faithful-hash");
        assert!(c.contains("shape-blind")
            && c.contains("shape becomes part of the committed identity"), "length-prefix concept");
        // D: order-stable-topk-via-index-sort (sleep_consolidation.rs L235-253)
        let c = concept("order-stable-topk-via-index-sort");
        assert!(c.contains("destroys the original ordering contract")
            && c.contains("canonical original order"), "order-stable topk concept");
        // A: make-unmake-over-clone-per-candidate (arena_eval.rs L225-232)
        let c = concept("make-unmake-over-clone-per-candidate");
        assert!(c.contains("restoring bit-identical pre-move state")
            && c.contains("zero per-candidate allocation"), "make/unmake concept");
        // B: argmax-rng-tie-break-collect-ties (arena_eval.rs L298-319)
        let c = concept("argmax-rng-tie-break-collect-ties");
        assert!(c.contains("move-generation order becomes a hidden priority")
            && c.contains("deterministic GIVEN the seed"), "tie-break concept");
        // D: shape-gated-warm-start-slot-copy (cross_game_edge.rs L160-185)
        let c = concept("shape-gated-warm-start-slot-copy");
        assert!(c.contains("dimensionally legal")
            && c.contains("the copy count clamp makes the operation total"), "warm-start concept");
        // D: domain-as-data-not-algorithm-fork (cross_game_edge.rs L122-131 + L196-209)
        let c = concept("domain-as-data-not-algorithm-fork");
        assert!(c.contains("the domain variation entirely in the DATA")
            && c.contains("differ by a data transformation, not a code path"), "data-not-fork concept");
        // C: one-sweep-spatial-feature-fanout (go_self_play.rs L229-314)
        let c = concept("one-sweep-spatial-feature-fanout");
        assert!(c.contains("the sole owner of the data visit")
            && c.contains("never by counting cells"), "one-sweep concept");
        // A: deterministic-tiling-projection-tables (go_self_play.rs L94-170)
        let c = concept("deterministic-tiling-projection-tables");
        assert!(c.contains("the same seed rebuilds the identical encoder")
            && c.contains("degenerate all-zero output column"), "projection-tables concept");
        // A: latent-encode-at-capture (go/bomber/fft self-play)
        let c = concept("latent-encode-at-capture");
        assert!(c.contains("Encode while the state is live")
            && c.contains("no risk of post-hoc encoding drifting"), "encode-at-capture concept");
        // B: non-saturating-outcome-reward-floor (go/bomber/fft sigmoid_outcome_reward)
        let c = concept("non-saturating-outcome-reward-floor");
        assert!(c.contains("never taught what to avoid")
            && c.contains("losses retain 15% weight"), "reward-floor concept");
        // E: collect-then-resolve-snapshot-batch (fft_self_play.rs L297-328)
        let c = concept("collect-then-resolve-snapshot-batch");
        assert!(c.contains("read-only over shared state")
            && c.contains("the single place where mutation happens"), "collect-then-resolve concept");
        // D: timeout-scalar-tiebreak-decisive-outcome (fft_self_play.rs L201-228)
        let c = concept("timeout-scalar-tiebreak-decisive-outcome");
        assert!(c.contains("drown the reward signal in uninformative draws")
            && c.contains("signal density at zero maintenance cost"), "timeout-tiebreak concept");
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
            assert!(ref_verdict.parses, "batch-32c ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-32c opt_pattern for '{id}' must parse");
        }
    }


    #[test]
    fn batch33_rules_present() {
        // Verify the 7 batch-33 rules across A/B/D/E/N.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // E (1 new): output-owned grid family
        assert!(ids.contains(&"grid-over-output-thread-private-sweep"), "output-owned grid sweep");
        // D (1 new): buffer-set reuse family
        assert!(ids.contains(&"buffer-set-struct-two-tier-dispatch"), "buffer-set two-tier dispatch");
        // A (2 new): slot-write + immutable-operand family
        assert!(ids.contains(&"slot-offset-partial-buffer-write"), "slot-offset partial write");
        assert!(ids.contains(&"immutable-operand-upload-at-set-creation"), "immutable operand at creation");
        // B (2 new): selective projection + rademacher family
        assert!(ids.contains(&"selective-projection-effective-rank-head"), "selective projection head");
        assert!(ids.contains(&"rademacher-signs-over-gaussian-projection"), "rademacher signs");
        // N (1 new): host-computed derived scalar
        assert!(ids.contains(&"derived-scalar-host-computed-in-uniform"), "derived scalar host-computed");
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
        assert!(c.contains("owned by exactly one thread")
            && c.contains("zero cross-thread reduction"), "output-owned sweep concept");
        // D: buffer-set-struct-two-tier-dispatch (rotation.rs L74-79 + L198-216 vs L263-269)
        let c = concept("buffer-set-struct-two-tier-dispatch");
        assert!(c.contains("reuse unit")
            && c.contains("makes the allocating path explicitly secondary"), "buffer-set concept");
        // A: slot-offset-partial-buffer-write (rotation.rs L146-160 + eigenbasis_rotation.wgsl L44-45)
        let c = concept("slot-offset-partial-buffer-write");
        assert!(c.contains("host-side mirror of the kernel's indexing base")
            && c.contains("delta write"), "slot-offset concept");
        // A: immutable-operand-upload-at-set-creation (rotation.rs L121-129 + calibration.rs L187-199)
        let c = concept("immutable-operand-upload-at-set-creation");
        assert!(c.contains("lifetime")
            && c.contains("never sees the operand's bytes again"), "immutable-operand concept");
        // B: selective-projection-effective-rank-head (qjl.rs L3/L5/L166-168 + selective_qjl.wgsl L57-62)
        let c = concept("selective-projection-effective-rank-head");
        assert!(c.contains("bypasses projection entirely")
            && c.contains("effective rank"), "selective-projection concept");
        // N: derived-scalar-host-computed-in-uniform (qjl.rs L48-49/L59 + selective_qjl.wgsl L22/L66)
        let c = concept("derived-scalar-host-computed-in-uniform");
        assert!(c.contains("bit-identical")
            && c.contains("2.5 ULP"), "derived-scalar concept");
        // B: rademacher-signs-over-gaussian-projection (qjl.rs L4/L84 + bench L264-267)
        let c = concept("rademacher-signs-over-gaussian-projection");
        assert!(c.contains("coin flip") && c.contains("exact in f32"), "rademacher concept");
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
            assert!(ref_verdict.parses, "batch-33 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-33 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch34_rules_present() {
        // Verify the 9 batch-34 rules across A/C/D/H/L/N.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // N (1 new): exact-op parity
        assert!(ids.contains(&"exact-op-parity-for-analytic-backward"), "exact-op parity");
        // A (2 new): mirror ring + activation slots
        assert!(ids.contains(&"mirror-ring-store-sliding-window-contiguity"), "mirror ring store");
        assert!(ids.contains(&"preallocated-activation-slots-inplace-save"), "activation slots");
        // D (3 new): split dispatch + hybrid sync points + op-major inversion
        assert!(ids.contains(&"split-dispatch-expose-fused-intermediate"), "split dispatch");
        assert!(ids.contains(&"hybrid-cpu-elementwise-gpu-throughput-sync-points"), "hybrid sync points");
        assert!(ids.contains(&"op-major-backward-loop-inversion-batch-sequence-axis"), "op-major inversion");
        // L (1 new): pre-transposed handles
        assert!(ids.contains(&"pre-transposed-consumer-layout-handles-build-once"), "pre-transposed handles");
        // C (1 new): sibling weight-grads share input
        assert!(ids.contains(&"shape-matched-sibling-weight-grads-share-input-loop"), "sibling grads share input");
        // H (1 new): norm-statistic exposure
        assert!(ids.contains(&"backward-helper-expose-shared-norm-statistic"), "norm statistic exposure");
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
        assert!(c.contains("two different functions")
            && c.contains("train-vs-infer"), "exact-op parity concept");
        // A: mirror-ring (mod.rs L473-500 + kv_cache contract)
        let c = concept("mirror-ring-store-sliding-window-contiguity");
        assert!(c.contains("contiguous by construction")
            && c.contains("2·sw"), "mirror-ring concept");
        // D: split-dispatch (mod.rs L325-344 + dispatch.rs discard)
        let c = concept("split-dispatch-expose-fused-intermediate");
        assert!(c.contains("re-launch the whole producer")
            && c.contains("never a second launch"), "split-dispatch concept");
        // A: activation slots (16 save sites)
        let c = concept("preallocated-activation-slots-inplace-save");
        assert!(c.contains("zero allocations")
            && c.contains("size is asserted once"), "activation slots concept");
        // D: hybrid sync points (L258/L311/L369/L383)
        let c = concept("hybrid-cpu-elementwise-gpu-throughput-sync-points");
        assert!(c.contains("exactly 4 syncs")
            && c.contains("launch-free"), "hybrid sync concept");
        // L: pre-transposed handles (backward_gpu L20-26 + gemma2/q4k recurrence)
        let c = concept("pre-transposed-consumer-layout-handles-build-once");
        assert!(c.contains("OPPOSITE layout")
            && c.contains("frozen-base contract"), "pre-transposed handles concept");
        // C: sibling weight-grads (L532-540 + L358-366)
        let c = concept("shape-matched-sibling-weight-grads-share-input-loop");
        assert!(c.contains("identical output-row counts")
            && c.contains("ONE pass over the shared input"), "sibling grads concept");
        // D: op-major inversion (L305-522 + dispatch round-trip)
        let c = concept("op-major-backward-loop-inversion-batch-sequence-axis");
        assert!(c.contains("sequence axis is free")
            && c.contains("7 × seq_len"), "op-major concept");
        // H: norm-statistic exposure (engine rmsnorm + 7 sites)
        let c = concept("backward-helper-expose-shared-norm-statistic");
        assert!(c.contains("return shape")
            && c.contains("SAME call site"), "norm statistic concept");
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
            assert!(ref_verdict.parses, "batch-34 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-34 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch35_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // E (1 new): mask sentinel identity participation
        assert!(ids.contains(&"mask-sentinel-matches-reduction-identity"), "mask sentinel");
        // A (3 new): staging gate, alias live handle, quadratic recompute
        assert!(ids.contains(&"smem-staging-gated-by-cross-thread-reuse"), "staging gate");
        assert!(ids.contains(&"side-consumer-readback-alias-live-handle-for-dependent-dispatch"), "alias live handle");
        assert!(ids.contains(&"quadratic-attention-intermediates-recompute-from-linear-saves"), "quadratic recompute");
        // C (1 new): kernelize past the batching floor
        assert!(ids.contains(&"kernelize-host-chain-after-sync-batching-floor"), "kernelize floor");
        // B (1 new): separate scratch + runtime strides
        assert!(ids.contains(&"block-reduction-separate-scratch-runtime-strides"), "runtime strides");
        // N (1 new): saturation boundary of output-recovery
        assert!(ids.contains(&"saturating-nonlinearity-recovery-breaks-near-cap"), "saturation boundary");
        // H (1 new): adapter grad join mirrors insertion point
        assert!(ids.contains(&"adapter-grad-join-mirrors-forward-delta-insertion-point"), "adapter grad join");
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
        assert!(c.contains("IDENTITY element") && c.contains("rewrite the sentinel"), "mask sentinel concept");
        // A: staging gate (L123-129 + L141)
        let c = concept("smem-staging-gated-by-cross-thread-reuse");
        assert!(c.contains("reuse count exceeds one") && c.contains("never enters smem"), "staging gate concept");
        // C: kernelize floor (header 52 + resident.rs L687-688)
        let c = concept("kernelize-host-chain-after-sync-batching-floor");
        assert!(c.contains("sync floor of 2") && c.contains("2 syncs x 26 layers = 52"), "kernelize floor concept");
        // B: separate scratch + runtime strides (L157-188)
        let c = concept("block-reduction-separate-scratch-runtime-strides");
        assert!(c.contains("SEPARATE scratch") && c.contains("runtime extent"), "runtime strides concept");
        // N: saturation boundary (engine L354-358 + mod.rs L572-578 contrast)
        let c = concept("saturating-nonlinearity-recovery-breaks-near-cap");
        assert!(c.contains("rounds to 1.0") && c.contains("~270"), "saturation concept");
        // H: adapter grad join (mod.rs L197-204 + L613-630 + L931-933)
        let c = concept("adapter-grad-join-mirrors-forward-delta-insertion-point");
        assert!(c.contains("immortal by construction") && c.contains("Issue 687 H2"), "adapter join concept");
        // A: alias live handle (dispatch.rs L387-392 + L498-503)
        let c = concept("side-consumer-readback-alias-live-handle-for-dependent-dispatch");
        assert!(c.contains("never left the device") && c.contains("~53 MB"), "alias handle concept");
        // A: quadratic recompute (mod.rs L216-220 + backward.rs L333-344)
        let c = concept("quadratic-attention-intermediates-recompute-from-linear-saves");
        assert!(c.contains("Theta(N x d)") && c.contains("~870 MB"), "quadratic recompute concept");
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
            assert!(ref_verdict.parses, "batch-35 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-35 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch36_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // C (1 new): constant scale folded into the upload copy
        assert!(ids.contains(&"constant-scale-folded-into-upload-copy"), "fold scale into upload");
        // D (5 new): residency rung, handle checkpoints, decode-shape shed,
        // sibling ordering, dispatch-read window
        assert!(ids.contains(&"side-branch-operand-residency-matches-mutation-lifetime"), "residency rung");
        assert!(ids.contains(&"activation-checkpoint-as-handle-clone-deferred-batch-readback"), "handle checkpoints");
        assert!(ids.contains(&"training-forward-shed-decode-loop-shape"), "decode-shape shed");
        assert!(ids.contains(&"sibling-dispatches-before-any-readback"), "sibling ordering");
        assert!(ids.contains(&"independent-cpu-work-fills-dispatch-read-window"), "dispatch-read window");
        // H (1 new): multi-term merge under a shared Jacobian
        assert!(ids.contains(&"grad-terms-merge-before-shared-link-jacobian"), "grad merge before Jacobian");
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
        assert!(c.contains("upload COPY") && c.contains("unscaled original"), "fold-into-copy concept");
        // D: residency rung (L12-14 + L56-58)
        let c = concept("side-branch-operand-residency-matches-mutation-lifetime");
        assert!(c.contains("mutation cadence") && c.contains("~70K sync round-trips"), "residency concept");
        // D: handle checkpoints (L36-37 + L959-963)
        let c = concept("activation-checkpoint-as-handle-clone-deferred-batch-readback");
        assert!(c.contains("refcount bump") && c.contains("364 syncs"), "checkpoint concept");
        // D: decode-shape shed (L545-557)
        let c = concept("training-forward-shed-decode-loop-shape");
        assert!(c.contains("DECODE loop shape") && c.contains("~55K dispatches"), "decode-shed concept");
        // D: sibling ordering (L1087-1095 + L1515-1525)
        let c = concept("sibling-dispatches-before-any-readback");
        assert!(c.contains("FIRST readback") && c.contains("DIFFERENT input buffers"), "sibling ordering concept");
        // D: dispatch-read window (L1326-1351)
        let c = concept("independent-cpu-work-fills-dispatch-read-window");
        assert!(c.contains("host is IDLE") && c.contains("max(GPU, CPU)"), "window concept");
        // H: merge before Jacobian (L1229-1248)
        let c = concept("grad-terms-merge-before-shared-link-jacobian");
        assert!(c.contains("ONCE to the sum") && c.contains("WRONG, not just slower"), "merge concept");
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
            assert!(ref_verdict.parses, "batch-36 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-36 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch37_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (3 new): device-resident KV ring, aliased-tensor handle clone,
        // cache-blocked host transpose
        assert!(ids.contains(&"kv-window-device-resident-ring-append"), "KV ring append");
        assert!(ids.contains(&"identical-tensor-handle-clone-zero-reupload"), "absent-V handle clone");
        assert!(ids.contains(&"tiled-host-transpose-before-quantize"), "tiled transpose");
        // B (1 new): scale folded at the sparse gather
        assert!(ids.contains(&"sparse-gather-scale-at-read-not-table-prepass"), "gather-side scale fold");
        // D (2 new): loss-position gating, bulk-slice fast path
        assert!(ids.contains(&"lm-head-loss-positions-only"), "lm-head loss gating");
        assert!(ids.contains(&"materialized-fast-path-bulk-slice-accessor"), "bulk slice accessor");
        // L (1 new): draining build
        assert!(ids.contains(&"stream-drain-free-source-during-build"), "draining build");
        // N (1 new): verbatim upload on format match
        assert!(ids.contains(&"verbatim-upload-on-format-match"), "verbatim upload");
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
        assert!(c.contains("append-vs-snapshot")
            && c.contains("only the NEW row crosses the bus"), "KV ring concept");
        // D: lm-head-loss-positions-only (mod.rs L1123 + backward.rs L73-76)
        let c = concept("lm-head-loss-positions-only");
        assert!(c.contains("loss-support set")
            && c.contains("never launches"), "lm-head gating concept");
        // N: verbatim-upload-on-format-match (weight_buffers.rs L498-509)
        let c = concept("verbatim-upload-on-format-match");
        assert!(c.contains("source format == destination format")
            && c.contains("never dequantize"), "verbatim upload concept");
        // L: stream-drain-free-source-during-build (mod.rs L610-635)
        let c = concept("stream-drain-free-source-during-build");
        assert!(c.contains("O(one unit), not O(model)")
            && c.contains("process, upload, drop"), "draining build concept");
        // A: identical-tensor-handle-clone-zero-reupload (weight_buffers.rs L229-232)
        let c = concept("identical-tensor-handle-clone-zero-reupload");
        assert!(c.contains("refcount bump")
            && c.contains("zero re-upload"), "handle clone concept");
        // B: sparse-gather-scale-at-read-not-table-prepass (weight_buffers.rs L77-96 + L147-148)
        let c = concept("sparse-gather-scale-at-read-not-table-prepass");
        assert!(c.contains("READ-SET DENSITY")
            && c.contains("INVERSE regime"), "gather-side scale concept");
        // A: tiled-host-transpose-before-quantize (weight_buffers.rs L460-466)
        let c = concept("tiled-host-transpose-before-quantize");
        assert!(c.contains("write-allocated")
            && c.contains("cache-BLOCKED"), "tiled transpose concept");
        // D: materialized-fast-path-bulk-slice-accessor (weight_buffers.rs L100-105)
        let c = concept("materialized-fast-path-bulk-slice-accessor");
        assert!(c.contains("ONCE PER STEP")
            && c.contains("representation-specialized"), "bulk accessor concept");
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
            assert!(ref_verdict.parses, "batch-37 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-37 opt_pattern for '{id}' must parse");
        }
    }


    #[test]
    fn batch38_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (3 new): flat activation slots, transient-scratch pinning,
        // row-accumulate transposed consumption
        assert!(ids.contains(&"flat-activation-slots-as-readback-target"), "flat activation slots");
        assert!(ids.contains(&"transient-spike-pins-persistent-scratch"), "transient spike pinning");
        assert!(ids.contains(&"transposed-consumption-accumulate-not-gather"), "row-accumulate z pass");
        // D (1 new): env-flag resolution timing
        assert!(ids.contains(&"env-flag-resolved-once-not-per-dispatch"), "env flag resolved once");
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
        assert!(c.contains("resolve it ONCE")
            && c.contains("papaya"), "env-flag resolution concept");
        // A: flat-activation-slots-as-readback-target (forward_batched.rs save sweeps + mod.rs L753-755)
        let c = concept("flat-activation-slots-as-readback-target");
        assert!(c.contains("container layout IS the copy count")
            && c.contains("readback target"), "flat slots concept");
        // A: transient-spike-pins-persistent-scratch (backward.rs L409 vs L557; mod.rs L377/L426)
        let c = concept("transient-spike-pins-persistent-scratch");
        assert!(c.contains("historical maximum")
            && c.contains("non-recurring"), "scratch pinning concept");
        // A: transposed-consumption-accumulate-not-gather (backward.rs L923-930 vs forward_batched.rs L1013-1018)
        let c = concept("transposed-consumption-accumulate-not-gather");
        assert!(c.contains("loop interchange")
            && c.contains("read amplification"), "row-accumulate concept");
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
            assert!(ref_verdict.parses, "batch-38 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-38 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch39_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // K (1 new): autotune cache salvage across owner lifetimes
        assert!(ids.contains(&"autotune-cache-extract-outlives-owner"), "autotune cache extract");
        // A (5 new): rewind, boot-harvest hotswap, tiny params, dst-passing readback, clone escape
        assert!(ids.contains(&"append-cache-rewind-overwrite-in-place"), "append cache rewind");
        assert!(ids.contains(&"slow-path-bootstraps-raw-buffer-hotswap-cache"), "slow path bootstrap hotswap");
        assert!(ids.contains(&"tiny-param-buffer-rebuild-over-cache"), "tiny param rebuild");
        assert!(ids.contains(&"readback-destination-passing-free-fn"), "readback destination passing");
        assert!(ids.contains(&"variant-owned-output-buffer-clone-escape"), "variant output clone escape");
        // C (1 new): device-side anchor payoff condition
        assert!(ids.contains(&"elementwise-offload-needs-device-side-anchor"), "device side anchor");
        // D (2 new): shared layer body, handle boundary
        assert!(ids.contains(&"entrypoint-variants-share-layer-body"), "entrypoint shared body");
        assert!(ids.contains(&"pipeline-layer-boundary-device-handle"), "pipeline handle boundary");
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
        assert!(c.contains("decouple the cache lifetime")
            && c.contains("re-tuning is not"), "autotune extract concept");
        // A: append-cache-rewind-overwrite-in-place (L2061-L2074 + L2084-L2089)
        let c = concept("append-cache-rewind-overwrite-in-place");
        assert!(c.contains("rewind_to(pos)")
            && c.contains("O(max_seq)"), "rewind concept");
        // A: slow-path-bootstraps-raw-buffer-hotswap-cache (L1551-L1558/L1585-L1627/L1630-L1653)
        let c = concept("slow-path-bootstraps-raw-buffer-hotswap-cache");
        assert!(c.contains("harvests the raw native buffers")
            && c.contains("slot ORDER is the key"), "boot harvest concept");
        // D: entrypoint-variants-share-layer-body (L566-L576/L977-L992/L1862-L1896 vs L2162-L2177)
        let c = concept("entrypoint-variants-share-layer-body");
        assert!(c.contains("ONE shared method")
            && c.contains("guarantees divergence"), "shared body concept");
        // C: elementwise-offload-needs-device-side-anchor (L969-L972/L930-L935 vs L737-L741)
        let c = concept("elementwise-offload-needs-device-side-anchor");
        assert!(c.contains("device-side anchor")
            && c.contains("compute-offload"), "anchor concept");
        // A: tiny-param-buffer-rebuild-over-cache (L2437-L2441/L2473-L2476)
        let c = concept("tiny-param-buffer-rebuild-over-cache");
        assert!(c.contains("rebuild the params")
            && c.contains("cache line"), "tiny params concept");
        // A: readback-destination-passing-free-fn (L807-L828 vs L505-L523/L613-L614)
        let c = concept("readback-destination-passing-free-fn");
        assert!(c.contains("destination-passing")
            && c.contains("FREE function over the context"), "dst passing concept");
        // A: variant-owned-output-buffer-clone-escape (L1069/L1192/L1304/L1416/L1979/L2240 + L2308-L2315)
        let c = concept("variant-owned-output-buffer-clone-escape");
        assert!(c.contains("ownership placement")
            && c.contains("per layer per token"), "clone escape concept");
        // D: pipeline-layer-boundary-device-handle (L937/L871-L874/L944-L945/L1086-L1087/L1018-L1024)
        let c = concept("pipeline-layer-boundary-device-handle");
        assert!(c.contains("pipeline currency")
            && c.contains("INTERNAL detail"), "handle boundary concept");
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
            assert!(ref_verdict.parses, "batch-39 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-39 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch40_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (4 new): coalesced-write scratch, cache-derived write plan, from_cpu builders, bridge hoist
        assert!(ids.contains(&"host-scratch-mirrors-slot-order-for-coalesced-writes"), "coalesced write scratch");
        assert!(ids.contains(&"write-plan-derived-from-cache-slots-not-recomputed"), "cache-derived write plan");
        assert!(ids.contains(&"cpu-source-builder-skips-device-roundtrip"), "cpu source builder");
        assert!(ids.contains(&"foreign-accumulator-bridge-copy-hoist-to-outer-loop"), "foreign bridge hoist");
        // H (1 new): two-phase expert loop
        assert!(ids.contains(&"expert-loop-two-phase-split-batch-across-iterations"), "two-phase expert loop");
        // N (1 new): gamma scope in RMSNorm backward
        assert!(ids.contains(&"rmsnorm-backward-gamma-scope-correction-term"), "gamma scope correction");
        // D (1 new): scalar-factored GEMV collapse
        assert!(ids.contains(&"per-position-scalar-factored-gemv-collapse"), "scalar factored gemv");
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
        assert!(c.contains("SUM of slot lengths")
            && c.contains("the write-call count"), "coalesced scratch concept");
        // A: write-plan-derived-from-cache-slots-not-recomputed (L1907-L1909 vs L1952-L1961)
        let c = concept("write-plan-derived-from-cache-slots-not-recomputed");
        assert!(c.contains("size_bytes")
            && c.contains("bug factory"), "cache-derived plan concept");
        // A: cpu-source-builder-skips-device-roundtrip (L253-L256 vs L225-L237)
        let c = concept("cpu-source-builder-skips-device-roundtrip");
        assert!(c.contains("read_one")
            && c.contains("the host already owns"), "cpu source builder concept");
        // H: expert-loop-two-phase-split-batch-across-iterations (L1682-L1710 + L1563-L1564)
        let c = concept("expert-loop-two-phase-split-batch-across-iterations");
        assert!(c.contains("cross-iteration")
            && c.contains("stage-A"), "two-phase loop concept");
        // N: rmsnorm-backward-gamma-scope-correction-term (L1537 wrong vs L2839 right)
        let c = concept("rmsnorm-backward-gamma-scope-correction-term");
        assert!(c.contains("ONLY the direct dy term")
            && c.contains("near 1"), "gamma scope concept");
        // D: per-position-scalar-factored-gemv-collapse (L2557-L2559 + L2576-L2584 the failure)
        let c = concept("per-position-scalar-factored-gemv-collapse");
        assert!(c.contains("consumer polarity")
            && c.contains("by linearity"), "scalar factoring concept");
        // A: foreign-accumulator-bridge-copy-hoist-to-outer-loop (L3082/L3085-L3098/L3256-L3269)
        let c = concept("foreign-accumulator-bridge-copy-hoist-to-outer-loop");
        assert!(c.contains("ping-pong")
            && c.contains("copy-in"), "foreign bridge concept");
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
            assert!(ref_verdict.parses, "batch-40 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-40 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch41_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (5 new): aliased-slot fallback, write coalescing, reservation token, type bridge, zombie machinery
        assert!(ids.contains(&"aliased-slot-allocate-replace-fallback-severs-binding"), "aliased slot fallback");
        assert!(ids.contains(&"pool-slot-write-coalescing-owns-only-declared-ranges"), "pool slot coalescing");
        assert!(ids.contains(&"pool-reservation-token-outlives-raw-view"), "reservation token");
        assert!(ids.contains(&"gpu-type-bridge-borrows-not-clones"), "type bridge borrows");
        assert!(ids.contains(&"superseded-primitive-machinery-zombie"), "zombie machinery");
        // E (1 new): concurrent disjoint uploads
        assert!(ids.contains(&"concurrent-disjoint-write-buffer-parallelizes-uploads"), "concurrent uploads");
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
        assert!(c.contains("disjoint offsets")
            && c.contains("host-bandwidth-bound"), "concurrent-disjoint-write-buffer-parallelizes-uploads concept");
        // A: aliased-slot-allocate-replace-fallback-severs-binding
        let c = concept("aliased-slot-allocate-replace-fallback-severs-binding");
        assert!(c.contains("never fall back to allocate-and-replace")
            && c.contains("boot weights"), "aliased-slot-allocate-replace-fallback-severs-binding concept");
        // A: pool-slot-write-coalescing-owns-only-declared-ranges
        let c = concept("pool-slot-write-coalescing-owns-only-declared-ranges");
        assert!(c.contains("union of declared slot ranges")
            && c.contains("DISCOVER it"), "pool-slot-write-coalescing-owns-only-declared-ranges concept");
        // A: pool-reservation-token-outlives-raw-view
        let c = concept("pool-reservation-token-outlives-raw-view");
        assert!(c.contains("NOT an ownership token")
            && c.contains("re-issues it"), "pool-reservation-token-outlives-raw-view concept");
        // A: gpu-type-bridge-borrows-not-clones
        let c = concept("gpu-type-bridge-borrows-not-clones");
        assert!(c.contains("field-identical")
            && c.contains("the first clone is 100% dead"), "gpu-type-bridge-borrows-not-clones concept");
        // A: superseded-primitive-machinery-zombie
        let c = concept("superseded-primitive-machinery-zombie");
        assert!(c.contains("only remaining caller")
            && c.contains("never-read"), "superseded-primitive-machinery-zombie concept");
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
            assert!(ref_verdict.parses, "batch-41 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-41 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch42_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // D (2 new): invariant-prefix elision + runtime identity gate
        assert!(ids.contains(&"refine-loop-invariant-prefix-elision"), "invariant prefix elision");
        assert!(ids.contains(&"runtime-identity-gate-cached-verdict-elides-side-projection"), "identity gate elision");
        // B (1 new): entropy via the exp shift identity
        assert!(ids.contains(&"softmax-entropy-via-shift-identity"), "entropy shift identity");
        // A (1 new): expected embedding as resident-head transposed GEMV
        assert!(ids.contains(&"expected-embedding-transposed-gemv-resident-head"), "x0 transposed gemv");
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
        assert!(c.contains("mutation set is usually a proper subset")
            && c.contains("O(steps x seq) to O(seq + steps x |S|)"), "prefix-elision concept");
        // B: softmax-entropy-via-shift-identity (mod.rs L103-109/L121-127/L151-154)
        let c = concept("softmax-entropy-via-shift-identity");
        assert!(c.contains("the shift is exp's argument, so its log is the shift itself")
            && c.contains("ONE transcendental"), "entropy-shift concept");
        // D: runtime-identity-gate-cached-verdict-elides-side-projection (mod.rs L556/L671-674 + sc.rs L281-287)
        let c = concept("runtime-identity-gate-cached-verdict-elides-side-projection");
        assert!(c.contains("re-derived from CONTENTS")
            && c.contains("gated consumer with an ungated producer"), "identity-gate concept");
        // A: expected-embedding-transposed-gemv-resident-head (sc.rs L229-246 + mod.rs L605-610)
        let c = concept("expected-embedding-transposed-gemv-resident-head");
        assert!(c.contains("TRANSPOSED GEMV of the tied lm_head")
            && c.contains("reads as a scatter/gather idiom"), "x0-gemv concept");
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
            assert!(ref_verdict.parses, "batch-42 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-42 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch43_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (3 new): input residency + support-only renorm + stack view bridge
        assert!(ids.contains(&"mutable-input-scratch-residency-write-per-call"), "input scratch residency");
        assert!(ids.contains(&"post-scatter-renorm-support-set-only"), "support-only renorm");
        assert!(ids.contains(&"stack-array-row-view-bridge"), "stack row-view bridge");
        // B (1 new): select_nth over full-vocab sort
        assert!(ids.contains(&"topk-select-nth-over-full-vocab-sort"), "select_nth topk");
        // D (1 new): the &[] sentinel launders the shape violation
        assert!(ids.contains(&"empty-slice-sentinel-launders-shape-violation"), "sentinel shape violation");
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
        assert!(c.contains("same residency discipline")
            && c.contains("device allocator service a new allocation request"), "input-residency concept");
        // B: topk-select-nth-over-full-vocab-sort (corrector.rs L421-436 + L694-709)
        let c = concept("topk-select-nth-over-full-vocab-sort");
        assert!(c.contains("select_nth_unstable_by partitions in O(V) average")
            && c.contains("the selection algorithm on the host side"), "select-nth concept");
        // A: post-scatter-renorm-support-set-only (corrector.rs L547-563 + L797-812)
        let c = concept("post-scatter-renorm-support-set-only");
        assert!(c.contains("non-zero support is exactly the K positions")
            && c.contains("no numerical risk"), "support-only renorm concept");
        // A: stack-array-row-view-bridge (dflash.rs L134-146 + 5 in-tree sites)
        let c = concept("stack-array-row-view-bridge");
        assert!(c.contains("zero heap, zero data copy")
            && c.contains("the established bridge idiom"), "stack-bridge concept");
        // D: empty-slice-sentinel-launders-shape-violation (dflash.rs L140-144 -> L247)
        let c = concept("empty-slice-sentinel-launders-shape-violation");
        assert!(c.contains("but shape-invalid")
            && c.contains("laundered into silent quality degradation"), "sentinel-laundering concept");
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
            assert!(ref_verdict.parses, "batch-43 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-43 opt_pattern for '{id}' must parse");
        }
    }




    #[test]
    fn batch44_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // E (1 new): split the reduction axis when rows are too few
        assert!(ids.contains(&"row-dot-split-inner-axis-block-reduce"), "reduction-axis split");
        // C (1 new): gather fused into the dot consumer
        assert!(ids.contains(&"fuse-indirect-gather-into-dot-consumer"), "fused indirect gather");
        // D (2 new): shared cube-size const + canonical serialization order
        assert!(ids.contains(&"share-cube-size-const-between-kernel-and-launcher"), "shared cube-size const");
        assert!(ids.contains(&"serialize-container-canonical-order"), "canonical serialization order");
        // A (2 new): smem stash + width-specialized unpack
        assert!(ids.contains(&"stash-kernel-internal-intermediate-in-smem"), "smem intermediate stash");
        assert!(ids.contains(&"bit-cursor-unpack-to-width-specialized-byte-batch"), "width-specialized unpack");
        // B (2 new): bit-address hoist + single-sweep minmax
        assert!(ids.contains(&"hoist-bit-address-out-of-packed-row-sweep"), "bit-address hoist");
        assert!(ids.contains(&"single-sweep-minmax-for-quant-scale"), "single-sweep minmax");
        // L (1 new): scatter consumer layout at production
        assert!(ids.contains(&"fold-tile-transpose-into-dequant-scatter"), "scatter at production");
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
        assert!(c.contains("one SM while the rest of the GPU idles")
            && c.contains("split the REDUCTION axis instead"), "reduction-axis-split concept");
        // C: fuse-indirect-gather-into-dot-consumer (mod.rs L1195-1196 -> L1303)
        let c = concept("fuse-indirect-gather-into-dot-consumer");
        assert!(c.contains("whose only destiny is to be multiplied")
            && c.contains("delete the gather kernel and pass"), "fused-gather concept");
        // D: share-cube-size-const-between-kernel-and-launcher (4 kernels vs 6 launchers)
        let c = concept("share-cube-size-const-between-kernel-and-launcher");
        assert!(c.contains("silent cross-file contract")
            && c.contains("exactly one source of truth for the stride"), "cube-size-const concept");
        // A: stash-kernel-internal-intermediate-in-smem (mod.rs L1413-1414 -> L1429)
        let c = concept("stash-kernel-internal-intermediate-in-smem");
        assert!(c.contains("no external reader")
            && c.contains("keep it in shared memory"), "smem-stash concept");
        // B: hoist-bit-address-out-of-packed-row-sweep (kvarn_dequant.rs L41-47 + L51)
        let c = concept("hoist-bit-address-out-of-packed-row-sweep");
        assert!(c.contains("identical on every iteration")
            && c.contains("decompose the bit address ONCE"), "bit-address-hoist concept");
        // L: fold-tile-transpose-into-dequant-scatter (kvarn_dequant.rs L120-133)
        let c = concept("fold-tile-transpose-into-dequant-scatter");
        assert!(c.contains("write the consumer's layout at production time")
            && c.contains("materialize-then-transpose"), "scatter-at-production concept");
        // A: bit-cursor-unpack-to-width-specialized-byte-batch (kvarn_dequant.rs L162-186)
        let c = concept("bit-cursor-unpack-to-width-specialized-byte-batch");
        assert!(c.contains("compile-time-constant")
            && c.contains("drifted fork"), "width-specialized-unpack concept");
        // B: single-sweep-minmax-for-quant-scale (kvarn_zone_weights.rs L67-68)
        let c = concept("single-sweep-minmax-for-quant-scale");
        assert!(c.contains("TWO separate full traversals")
            && c.contains("track (lo, hi) jointly in one pass"), "single-sweep-minmax concept");
        // D: serialize-container-canonical-order (kvarn_zone_weights.rs L284-287)
        let c = concept("serialize-container-canonical-order");
        assert!(c.contains("iteration order seeded by RandomState")
            && c.contains("pure function of container CONTENT"), "canonical-order concept");
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
            assert!(ref_verdict.parses, "batch-44 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-44 opt_pattern for '{id}' must parse");
        }
    }


    #[test]
    fn batch45_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (2 new): allocation depth + FD frozen co-operand
        assert!(ids.contains(&"scratch-depth-matches-shape-invariance"), "scratch depth");
        assert!(ids.contains(&"frozen-cooperand-hoist-out-of-perturbation-loop"), "frozen co-operand");
        // B (1 new): softmax denominator hoist
        assert!(ids.contains(&"softmax-denominator-hoist-out-of-weighted-v"), "denominator hoist");
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
        assert!(c.contains("allocation depth must match shape invariance")
            && c.contains("layer-invariant"), "scratch-depth concept");
        // B: softmax-denominator-hoist-out-of-weighted-v (meta_lora.rs L345-358 vs attention.rs L63-76)
        let c = concept("softmax-denominator-hoist-out-of-weighted-v");
        assert!(c.contains("an order of magnitude costlier than multiply")
            && c.contains("Hoist the reciprocal once"), "denominator-hoist concept");
        // A: frozen-cooperand-hoist-out-of-perturbation-loop (train_meta_lora.rs L276-321/L325-366)
        let c = concept("frozen-cooperand-hoist-out-of-perturbation-loop");
        assert!(c.contains("Materialization cadence must match mutation cadence")
            && c.contains("frozen co-operand"), "frozen-cooperand concept");
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
            assert!(ref_verdict.parses, "batch-45 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-45 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch46_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (2 new): declared-max thread-local tax + shared-side pair-batch decomposition
        assert!(ids.contains(&"declared-max-array-taxed-per-thread-under-runtime-bound"), "declared-max array tax");
        assert!(ids.contains(&"shared-operand-side-pair-batch-index-decomposition"), "shared-side decomposition");
        // D (1 new): full-cost-factor size gate
        assert!(ids.contains(&"size-gate-covers-every-cost-factor"), "size-gate cost factors");
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
        assert!(c.contains("every thread pays the DECLARED max")
            && c.contains("pipeline override constant"), "declared-max concept");
        // A: shared-operand-side-pair-batch-index-decomposition (spectralquant.rs L190 + wgsl L109)
        let c = concept("shared-operand-side-pair-batch-index-decomposition");
        assert!(c.contains("share-by-indexing instead of replication")
            && c.contains("Decompose the invocation index"), "shared-side concept");
        // D: size-gate-covers-every-cost-factor (dispatch.rs L215-216 + gpu_m2p.rs L307)
        let c = concept("size-gate-covers-every-cost-factor");
        assert!(c.contains("Gating on a partial product")
            && c.contains("crossover bench must re-run"), "size-gate concept");
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
            assert!(ref_verdict.parses, "batch-46 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-46 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch47_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // B (1 new): K/V projection hoist out of the query sweep
        assert!(ids.contains(&"per-key-projection-hoist-out-of-query-sweep"), "per-key projection hoist");
        // E (1 new): head-concat out-projection phase boundary
        assert!(ids.contains(&"multi-head-out-proj-consumes-head-concat"), "head-concat out-proj");
        // D (1 new): shared producer extract-once
        assert!(ids.contains(&"shared-producer-prefix-extract-once-per-input"), "shared producer extract-once");
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
        assert!(c.contains("re-derives the per-key projection inside the query loop")
            && c.contains("Project all K (and V) once per slice"), "per-key hoist concept");
        // E: multi-head-out-proj-consumes-head-concat (m2p_attention.wgsl L92-98/L167-172 + gpu_m2p.rs L134-141)
        let c = concept("multi-head-out-proj-consumes-head-concat");
        assert!(c.contains("consumes the CONCATENATION of all heads")
            && c.contains("scalar-times-rowsum"), "head-concat concept");
        // D: shared-producer-prefix-extract-once-per-input (shine.rs L229-230 + L138-144/L196-202)
        let c = concept("shared-producer-prefix-extract-once-per-input");
        assert!(c.contains("extract once and thread the shared artifact")
            && c.contains("the producer is pure"), "shared producer concept");
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
            assert!(ref_verdict.parses, "batch-47 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-47 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch49_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // A (1 new): runtime-arg loop bound demotes per-thread arrays to local memory
        assert!(ids.contains(&"runtime-arg-loop-bound-demotes-arrays-to-local-memory"), "runtime-arg local-memory demotion");
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
        assert!(c.contains("no register form")
            && c.contains("hoist it into the kernel as a compile-time constant"), "runtime-arg bound concept");
    }

    #[test]
    #[cfg(not(feature = "kernel_verify"))]
    fn batch49_pseudocode_parses() {
        // G1 for Batch 49: every new rule's ref_shape AND opt_pattern must
        // syn-parse as valid Rust (the L2 pruner verdict).
        use crate::domain::HealDomain;
        let corpus = build_kernel_seed_corpus();
        let ids = [
            "runtime-arg-loop-bound-demotes-arrays-to-local-memory",
        ];
        for id in ids {
            let entry = corpus
                .entries
                .iter()
                .find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("batch-49 rule '{id}' missing"));
            let ref_verdict = KernelOptDomain::validate(id, entry.buggy_code());
            assert!(ref_verdict.parses, "batch-49 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-49 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn batch51_rules_present() {
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // D (1 new): read-batch slot membership decided at launch
        assert!(ids.contains(&"batched-read-slot-membership-decided-at-launch"), "read-slot membership");
        // A (1 new): unpack loop indexed by container word
        assert!(ids.contains(&"unpack-loop-indexed-by-container-word"), "container-word unpack loop");
        // N (1 new): dequant placement per component by traffic share
        assert!(ids.contains(&"dequant-placement-by-component-traffic-share"), "per-component dequant placement");
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
        assert!(c.contains("NO persistent state to derive from")
            && c.contains("tag/metadata vec alongside the pushes"), "read-slot membership concept");
        // A: unpack-loop-indexed-by-container-word (attention_q8kv_cubecl.rs L212-224)
        let c = concept("unpack-loop-indexed-by-container-word");
        assert!(c.contains("one load serves all its bytes")
            && c.contains("loop GRANULARITY = container granularity"), "container-word unpack concept");
        // N: dequant-placement-by-component-traffic-share (attention_q8kv_cubecl.rs L4-5 + L398-400)
        let c = concept("dequant-placement-by-component-traffic-share");
        assert!(c.contains("OPPOSITE optimal dequant placements")
            && c.contains("B31 is CORRECT for the sidecar and WRONG for the payload"), "per-component placement concept");
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
            assert!(ref_verdict.parses, "batch-51 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-51 opt_pattern for '{id}' must parse");
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
            c.contains("the right value is NEVER STORED")
                && c.contains("DATA liveness"),
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
            assert!(ref_verdict.parses, "batch-52 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-52 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-54 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-54 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-55 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-55 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-56 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-56 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-57 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-57 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-58 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-58 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-60 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-60 opt_pattern for '{id}' must parse");
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
            let e = corpus.entries.iter().find(|e| e.rule_id == id)
                .unwrap_or_else(|| panic!("batch-63 rule '{id}' missing"));
            let c = e.gpu_concept.as_str();
            for n in needles {
                assert!(c.contains(n), "batch-63 concept for '{id}' missing phrase '{n}'");
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
            assert!(ref_verdict.parses, "batch-63 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-63 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-70 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-70 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-71 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-71 opt_pattern for '{id}' must parse");
        }
    }

    #[test]
    fn kernel_categories_table_has_15_entries() {
        assert_eq!(KERNEL_CATEGORIES.len(), 15, "Plan 003: 15 categories (A–O)");
    }

    #[test]
    fn kernel_categories_cover_all_corpus_rules_exhaustively() {
        // Every rule_id in the 379-rule corpus must appear in exactly one category.
        let corpus = build_kernel_seed_corpus();
        let corpus_ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        let mut categorized: Vec<&str> = Vec::new();
        for (_, _, rule_ids) in KERNEL_CATEGORIES {
            categorized.extend_from_slice(rule_ids);
        }
        // Same length (no missing, no extra).
        assert_eq!(categorized.len(), corpus_ids.len(),
            "Plan 003: KERNEL_CATEGORIES rule count ({}) must match corpus ({})",
            categorized.len(), corpus_ids.len());
        // No duplicate category assignments.
        let mut sorted = categorized.clone();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "Plan 003: rule appears in multiple categories");
        // Every corpus rule has a category home.
        for rid in &corpus_ids {
            assert!(categorized.contains(rid),
                "Plan 003: corpus rule '{rid}' has no category in KERNEL_CATEGORIES");
        }
        // Every category entry exists in the corpus (no stale rule_ids).
        for rid in &categorized {
            assert!(corpus_ids.contains(rid),
                "Plan 003: KERNEL_CATEGORIES references '{rid}' not in corpus");
        }
    }

    #[test]
    fn kernel_categories_counts_match_plan_003_table() {
        // Pin per-category rule counts to the Plan 003 table (updated for Batch 79).
        let expected = [
            (KernelCategory::MemoryAccess, 91),
            (KernelCategory::Compute, 53),
            (KernelCategory::Fusion, 35),
            (KernelCategory::Dispatch, 78),
            (KernelCategory::Parallelism, 22),
            (KernelCategory::FlashAttnTiling, 10),
            (KernelCategory::LinearAttnRecurrence, 21),
            (KernelCategory::MoeDispatchBackward, 14),
            (KernelCategory::NsaBlockSparse, 4),
            (KernelCategory::VarlenChunkedReduction, 5),
            (KernelCategory::AutotuneDispatch, 5),
            (KernelCategory::ConvGemmLayout, 11),
            (KernelCategory::KernelPolymorphism, 10),
            (KernelCategory::DtypeAlignment, 21),
            (KernelCategory::IrStrengthReduction, 4),
        ];
        for (cat, count) in expected {
            let entry = KERNEL_CATEGORIES.iter().find(|(c, _, _)| *c == cat)
                .unwrap_or_else(|| panic!("Plan 003: category {cat:?} missing from KERNEL_CATEGORIES"));
            assert_eq!(entry.2.len(), count,
                "Plan 003: category {cat:?} has {} rules, expected {count}",
                entry.2.len());
        }
        // Sum must equal 384.
        let total: usize = KERNEL_CATEGORIES.iter().map(|(_, _, r)| r.len()).sum();
        assert_eq!(total, 384, "Plan 003: category totals must sum to 384");
    }

    #[test]
    fn build_corpus_categorized_round_trips_all_rules() {
        // build_corpus_categorized groups the corpus by category. Every rule
        // should appear in exactly one bucket, and the total count should match.
        let grouped = build_corpus_categorized();
        assert_eq!(grouped.len(), 15, "Plan 003: 15 category buckets");
        let total: usize = grouped.iter().map(|(_, rules)| rules.len()).sum();
        assert_eq!(total, 384, "Plan 003: grouped corpus has {total} rules, expected 384");
        // No empty buckets.
        for (cat, rules) in &grouped {
            assert!(!rules.is_empty(), "Plan 003: category {cat:?} has zero rules");
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
            assert!(ref_verdict.parses, "batch-72 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-72 opt_pattern for '{id}' must parse");
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
                [
                    "pinned-host",
                    "contiguize",
                    "k-limit",
                    "CS=32",
                    "vendor",
                ],
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
            assert!(ref_verdict.parses, "batch-77 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-77 opt_pattern for '{id}' must parse");
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
            assert!(ref_verdict.parses, "batch-79 ref_shape for '{id}' must parse");
            let opt_verdict = KernelOptDomain::validate(id, entry.fixed_code());
            assert!(opt_verdict.parses, "batch-79 opt_pattern for '{id}' must parse");
        }
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
            let mut pairs: Vec<(f32, f32)> =
                a.iter().zip(b.iter()).map(|(&x, &y)| (x, y)).collect();
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
            let mut a = Vec::new();
            let mut b = Vec::new();
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
                        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1
                            ^ (i as u64).wrapping_mul(0x2545_F491_4F6C_DD1D);
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
            seq.iter().zip(&scan).any(|(s, t)| s.to_bits() != t.to_bits()),
            "non-vacuous: scan and sequential must differ bitwise somewhere"
        );
        assert!(max_rel > 0.0, "max_rel measured nothing");
    }

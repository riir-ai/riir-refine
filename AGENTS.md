# AGENTS.md — riir-clippy

The global `~/.agents/` rules apply; this file documents repo-local context.

## Role

**Multi-domain modelless code healer** — a pluggable framework behind the
`HealDomain` trait (Plan 001) that heals code across multiple domains using a
drafter+pruner+corpus pattern. This is a **developer tooling repo**, NOT a
game concern, NOT a training repo, NOT a chain repo.

Three domains ship (Plan 001):
1. **`clippy_lints`** (default-on) — heals `cargo clippy` warnings. The
   original domain; 10 common lints across style/complexity/perf/nursery.
2. **`rust_perf`** (default-on since 2026-08-13, Phase 2) — heals rust perf
   issues. **62 rules** distilled from the `rust-optimize` skill checklist +
   nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs
   profiling guide (Research 032) (complexity,
   allocation, layout, concurrency, simd, arithmetic, iterators, dispatch,
   caching, io, bevy, wasm). The **perf-bench
   Pareto validator** (Bench 013 promotion gate + Bench 015 Phase 4
   finalization + Bench 016 Issue 012 expansion + Bench 017 Issue 013 data-structure-choice
   expansion + Bench 020 Issue 014 alloc-axis expansion, `rust_perf_bench` feature)
   is the promotion gate — GOAT G1+G5+G-det PASS (the alloc-count axis: exact
   integers, bit-identical across 3 repeat runs on 52/52 benchable rules).
   12 of 62 rules carry `context_notes` annotations recording the
   bench finding (10 verdict-class — Bench 015/016: 2 ParetoDominated + 4
   regime-fragile/Mixed in the default config; Batch 48: `filter-count-to-any`
   ParetoDominated + `iter-copied-deref-map` Mixed; Batch 53 / Bench 038:
   `powi-vs-powf` all-Tie (LLVM lowers powf(2.0) ≡ powi(2) in release — the
   folk wisdom only holds for debug profiles / exponents ≥ 3) +
   `soa-hot-field` latency-bound (a serial f32 add-reduction hides the layout
   win; SoA pays on throughput-bound field consumers); 2 metric-mismatch-class — Issue 014:
   `tokio-guard-across-await` + `bounded-channel-backpressure`, NotBenchable
   because the cost metric is contention latency / queue depth, not
   wall-clock — consumers should treat annotated rules as
   context-dependent).
3. **`kernel_opt`** (opt-in, Phase 3) — heals GPU kernel optimization. 344 rules
   (Batch 1-6: 50 popcorn-distilled from Research 002-007; Batch 7: 9 burn-cubecl
   Rust-native autotune dispatch + conv-as-GEMM + layout rules from Research 008;
   Batch 8: 9 burn-cubecl kernel polymorphism + dtype/alignment specialization +
   cubecl-opt IR strength-reduction rules from Research 009;
   Batch 9: 8 popcorn fusion + dtype/quant + dispatch rules from Research 010;
   Batch 10: 12 popcorn compute + chunked reduction + polymorphism rules from Research 011;
   Batch 11: 11 popcorn linear-attention recurrence family rules from Research 012;
   Batch 12: 14 popcorn gated delta + specialized recurrence + advanced attention rules from Research 013;
   Batch 13: 6 popcorn final-pass rules (RL loss stability, complex-free RoPE, multimodal RoPE, RWKV-7 channel mixing + gate output) from Research 014;
   popcorn corpus distillation COMPLETE — every popcorn kernel file read;
   Batch 14: 4 riir-gpu production-kernel rules (norm-fold into matmul, reduction-split pre-pass, single-plane reduction, CPU-bottleneck offload) from Research 015 — the first batch mining the stack's own kernels, not external libraries);
   Batch 15: 4 riir-gpu production-kernel rules (block-contiguous weight layout, fused dual GEMV + GeGLU, orthogonal pre-rotation, water-fill tail-dim shared codebook) from Research 016 — the second batch mining the stack's own kernels (ternary GEMM + fused GeGLU GEMV + SpectralQuant);
   Batch 16: 4 riir-gpu production-kernel rules (cmma hardware unit, dequant-once-per-k-tile, dispatch axis swap, norm-decoupled low-bit scoring) from Research 017 — the third batch mining the stack's own kernels (gemm_ternary_simdgroup + forward_turboquant).
   Batch 17: 4 riir-gpu production-kernel rules (skip-all-zero-packed-words-via-or-reduce, hoist-group-invariant-scale-out-of-word-loop, split-sign-accumulators-defer-subtraction, clamp-tile-indices-to-sentinel-discard-at-write) from Research 018 — the fourth batch mining the stack's own kernels (gemm_ternary_batched + gemv_ternary_block_contiguous — the ternary scalar-ALU hot loop). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 18: 3 riir-gpu production-kernel rules (row-tile-amortize-input-load, factor-affine-quant-scale-out-of-element-loop, pin-lane-to-quant-sub-block-coalesce-byte-loads) from Research 019 — the fifth batch mining the stack's own kernels (gemv_q4k_cubecl — the Q4_K quantized GEMV family: GEMV→small-GEMM M-widening, affine dequant factoring, lane-pinned byte coalescing). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 19: 2 riir-gpu production-kernel rules (chain-dispatches-single-sync-point, fuse-residual-via-in-place-output-array) from Research 020 — the sixth batch mining the stack's own kernels (ternary_ffn_fused + gemv_ternary_residual_cubecl — command-buffer-level dispatch fusion + in-place output-array parameter-count bypass). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 20: 5 riir-gpu production-kernel rules (cooperative-block-wide-quantize-staging, byte-perm-lut-via-permutation-immediate, dp4a-int8-hardware-dot-product, threadgroup-subset-sum-lut-keyed-by-activation, base-3-trit-packing-5-per-byte) from Research 021 — the seventh batch mining the stack's own kernels (gemv_ternary_cuda_raw + gemv_ternary_cubecl LUT/trit paths — first CUDA-specific hardware-instruction cluster: __byte_perm permutation LUT + __dp4a int8 dot product + cooperative block-wide quantize staging; plus CubeCL threadgroup subset-sum LUT keyed by activation + base-3 trit packing). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 21: 5 riir-gpu production-kernel rules (cross-runtime-raw-handle-extraction, pool-buffer-aliasing-dedicated-output, same-queue-implicit-ordering-no-sync, per-dispatch-pointer-re-extraction, no-blocking-sync-in-async-pipeline) from Research 022 — the eighth batch mining the stack's own kernels (gemm_ternary_metal_zero_copy — the Metal cross-runtime zero-copy integration cluster: raw `id<MTLBuffer>` extraction from CubeCL/wgpu handles + pool-buffer aliasing hazard workaround + same-queue implicit ordering + per-dispatch pointer re-extraction + no-blocking-sync in async pipeline; patterns are generic for any wrapper-runtime→raw-native-dispatch integration). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 22: 5 riir-gpu production-kernel rules (dynamic-shared-memory-runtime-sized, ternary-bit-plane-u64-as-2x-u32, warp-shuffle-intrinsics-for-sub-warp-reduction, fused-activation-quantize-for-gemv-consumer, cuda-graph-scalar-via-device-pointer) from Research 023 — the ninth batch mining the stack's own kernels (cudarc_kernels/ — the full raw-CUDA forward port for Issue 615: embedding + attention + elementwise + deltanet; patterns include `extern __shared__` runtime-sized smem, bit-plane ternary dequant, `__shfl_down_sync` warp reduction for ≤32-thread blocks, activation+quantize fusion for GEMV consumers, and `_devpos` variants for CUDA Graph capture). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 23: 5 riir-gpu production-kernel rules (combined-kv-buffer-keys-then-values, gqa-kv-head-sharing-bandwidth-reduction, softcap-tanh-attention-score, fold-query-tokens-for-occupancy, hardcoded-fast-path-jit-specialization) from Research 024 — the tenth batch mining the stack's own kernels (attention_cubecl/ — the `CubeCL` flash-attention decode cluster: combined KV buffer `[keys | values]`, GQA KV-head sharing via `kv_group = head_idx * n_kv_head / n_head`, Gemma's `softcap * tanh(score/softcap)` bounded logits, fold-factor query-token coalescing for speculative-decode occupancy, and hardcoded fast-path vs parametric JIT specialization; ~24% decode throughput gain on M3 Max). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 24: 4 riir-gpu production-kernel rules (widen-index-multiply-to-64bit, dual-accumulator-single-reduction-sweep, dual-grad-same-pass, broadcast-uniform-skip-reduce) from Research 025 — the eleventh batch mining the stack's own kernels (cudarc_kernels/backward.rs — the Issue 641 per-layer backward cluster; closes `cudarc_kernels/` 6/6: 64-bit index widening for address math on large buffers, two block reductions fused into one sweep sharing loads + barrier tree, both partials of a product-rule activation in one kernel sharing one sigmoid eval, and thread-uniform values broadcast-not-reduced — the T7.2 validation catch where z came out blockDim.x× too large). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 25: 5 riir-gpu production-kernel rules (device-side-argmax-minimal-download, monotone-transform-elision-under-argmax, fan-out-shared-input-single-sync, hot-swap-weights-preserve-autotune-caches, tree-reduction-depth-over-sequential-accumulate) from Research 026 — the twelfth batch mining the stack's own kernels (the Gemma cluster: gemma2_cubecl/ + gemma4_cubecl/ — GPU argmax downloads 4 B instead of ~1 MB logits/token, strictly monotone softcap skipped under argmax-only consumers, N sibling QKV dispatches sharing one uploaded input + one batched sync, `replace_weights` hot-swap keeping autotune + JIT caches alive across LoRA steps, and plane tree-reduction depth ≈77 vs sequential 2304 for ~30× backward accuracy — the Issue 424 gradient-divergence fix). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 26: 8 riir-gpu production-kernel rules (per-slot-uniform-buffer-write-ordering, derive-params-from-global-invocation-id, merge-dispatches-into-single-compute-pass, batched-downloads-single-sync-cycle, chunk-encoder-submits-below-driver-limit, prebuilt-bind-groups-fixed-buffer-combos, host-cache-immutable-device-weights, defer-readback-until-after-submit) from Research 027 — the thirteenth batch mining the stack's own kernels (forward/ + buffer.rs — the wgpu forward-orchestration cluster, the first HOST-SIDE batch: wgpu flushes all `queue.write_buffer` calls before the encoder executes so shared uniforms corrupt (per-slot fix), per-item params encoded in the dispatch grid + decoded in-kernel (n_head × seq_len workgroups, one dispatch), dependent dispatches merged into one compute pass with in-order semantics + copies on the copy channel, N readbacks sharing one encoder/submit/map/poll (~1.28 ms per measured sync), Metal compute-pass-per-encoder DEADLOCK avoided by chunked immediate submits (~48 passes/chunk, env-tunable), fixed-combo bind groups pre-built at init (~768 `create_bind_group` calls per forward → 0), immutable weights host-cached at init (per-call download was the #1 forward bottleneck at 2 sync stalls), and readback deferred until after submit (mid-encoder downloads read stale pre-submission state). Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 27: 6 riir-gpu production-kernel rules (staging-packed-uniforms-encoder-ordered-copies, identical-uniforms-share-single-slot, persistent-zeros-source-encoder-copy-clear, fuse-sibling-target-grads-single-kernel, per-layer-strided-grad-destination, snapshot-transient-grad-for-deferred-download) from Research 028 — the fourteenth batch mining the stack's own kernels (backward/ + backward_dflash/ — the backward-pass orchestration clusters, completing the host-orchestration trilogy: all uniform params packed into ONE 16-byte-aligned staging vec + ONE upload + encoder-ordered per-dispatch copies (the 1-buffer complement to per-slot's N buffers), the idempotent-write exemption (sharing is safe IFF every write is byte-identical — N uniforms + N bind groups → 1), persistent device zeros + encoder-timeline copy clears (queue writes flush before the whole encoder so layer 1 would accumulate on layer 0's values; three-way independent mining convergence), sibling-target backward fusion per KERNEL KIND (one summed d_input + one grid-sized param-grad dispatch; 6 → 2), consumer-selected layer-major strided grad destinations (the per-layer split falls out of the existing pass — no de-interleave), and transient-grad snapshots into persistent buffers for deferred download (the LIFETIME complement to defer-readback's TIMING rule). Mining also surfaced a suspected live bug in backward_dflash (uniforms written via queue.write_buffer DURING encoder build → wrong query_pos for all but the last position per chunk with chunk_size=2) — filed as a riir-ai issue. Absorbs Proposal 012's design
   (thermal-tier + latent-MoE + numeric validator); 4090 LoRA deferred.
   Batch 28: 10 riir-gpu production-kernel rules (row-owned-register-blocked-recurrence, workgroup-equals-plane-native-reduction, smem-broadcast-scalar-then-parallel-scatter, token-flattened-elementwise-modulo-shared-weights, fused-conv-state-shift-dot-silu, softplus-piecewise-saturation-clamp, reverse-running-decay-weight, split-decay-outputs-eliminate-readback, carry-dual-layout-dead-slot, nonlinearity-forces-raw-input-buffers) from Research 029 — the fifteenth batch mining the stack's own kernels (deltanet_cubecl.rs + deltanet_chunked_cubecl.rs — the deltanet CubeCL cluster, the two largest single files in the crate: decode-time recurrence + chunkwise-parallel prefill): state-matrix row ownership with register blocking (rows independent within one decode step → one workgroup per (head,row), 4R+2W → 1R+1W global passes, ~3× memory-bound cut, 32× occupancy headroom; Issue 619), plane-sized workgroup geometry (size to EXACTLY one hardware plane + register-block width — native shuffle covers the full reduction, broadcasts free, zero barriers; the Issue 610 plane_sum-caught-1/4 hazard is the inverse), post-reduction broadcast scatter (leader publishes the scalar through the already-reduced slot, ALL threads scatter — 1 write/thread vs head_dim serial leader writes; Issue 616), mixed-arity launch flattening ([p, n_head] → one 1-D index space with per-head weights via i % n_head; 6144 → 48 dispatches), fused in-place conv shift+dot+SiLU decode kernel, far-tail softplus saturation clamp (±20, pinned to the CPU reference), dematerialized decay matrix (one running scalar, j downward, empty-product anchor — the inverse of cumsum-gate-to-decay-matrix), consumer-shaped output split (packing that breaks consumer layouts forces readback-unpack-reupload; write [H,C] + [H] directly), parameterized dual-layout carry with a provably-dead slot (prefill writes decode's conv_state natively; slot 0 stale — the decode shift overwrites before read), and the nonlinearity buffer contract (sequential in-place tricks are incompatible with batching when a nonlinearity intervenes — raw RO inputs + WO outputs; the negative space of inplace-output-alias). Corpus 183 → 193 (B 33, C 19, D 26, E 11, G 19, L 6). Mining CONFIRMED a production bug (deltanet_cross_chunk_f32 plane_sum mixes partials of different columns → block-constant output, masked by test tolerance, production-wired via prefill_deltanet_recurrence_chunked) + 3 latent/edge bugs (legacy recurrence broken for head_dim≠128 while being the documented fallback; partial-chunk carry GPU-vs-CPU divergence; GPU L2-normalize missing the CPU zero-norm guard) — filed as a riir-ai issue.
   Batch 29: 10 riir-gpu production-kernel rules (defer-readback-gated-by-live-output-size, weight-handle-cache-keyed-by-allocation-identity, first-readback-pays-queue-drain, epilogue-at-register-write-phase, delay-rms-scale-past-gemv-epilogue, pack-scalar-params-into-combined-buffer, enum-dispatch-separate-kernel-per-variant, epilogue-must-be-workgroup-local, launch-unchecked-caller-owned-size-contract, fast-math-contraction-behind-feature-flag) from Research 030 — the sixteenth batch mining the stack's own kernels (deltanet_input_proj_fused.rs + epilogue/ — the decode-path fusion cluster: Issue 602 sibling-GEMV fan-out + Plan 106 Track 3 CODA epilogue fusion; ternary_deltanet_gpu_forward.rs deliberately skipped — sibling actively working there): the readback-deferral footprint gate (Bench 437's 5 concurrent ~70 KB FFN buffers measured 16% SLOWER vs ~8 KB input-proj outputs chaining freely — deferral is NOT unconditional), pointer-identity device-handle caching (CPU weight refs → lock-free cache keyed by the allocation's (pos, neg) pointer pair, on-the-fly miss path), the queue-drain readback cost model (first read drains the burst's queue, later reads free — syncs are a function of BURSTS, not read calls), the CODA register-phase epilogue (memory-bound elementwise ops execute while the GEMV output is still in registers, riding lane 0's single store; 2→1 dispatches, 52/26 layers), the delayed-RMS commutation (RMSNorm(x)@W = r·(x·γ)@W with the honest 3→3 accounting — the win is dispatch overhead, not bandwidth), packed scalar params for the ~4-5 Array budget (u32s bitcast through the f32 pack), enum + one-kernel-per-variant under the no-generics DSL constraint, workgroup-local fusion legality (ownership scope, not just dependency), launch_unchecked + # Safety size contracts, and fast-math contraction behind a cfg_attr feature pair (strict-FP default byte-identical). Corpus 193 → 203 (A 29, B 34, C 21, D 29, E 12, M 6). First standing syn-parse gate (batch29_pseudocode_parses). Mining surfaced 2 latent cache hazards + a stale-doc-numbers bug — filed as riir-ai Issue 674.
   Batch 30: 6 riir-gpu production-kernel rules (zero-init-side-branch-frozen-base-path, counter-derived-seed-stateless-noise, predicate-gated-skip-empty-workset-step, share-pipeline-set-across-subsystems, pad-uniform-pod-to-min-binding-size, fuse-count-into-mutation-pass) from Research 031 — the seventeenth batch mining the stack's own kernels (dllm/mod.rs — the D2F training cluster: Plan 068 discrete-diffusion LM trainers + the Plan 078 RePlaid adaptive noise schedule): the frozen distillation teacher run through the SAME adapter-capable forward with B=0 (W·x + B·(A·x) = W·x exactly — one code path + one weight set for both modes, grounded on the lora/mod.rs zero-B constructor contract), counter-derived stateless noise seeding (((step << 32) | epoch) → pure-function-per-iteration streams, standalone replay, skip-independence, the same seed reaching the GPU as one uniform bit-identical to noise_corrupt.wgsl), whole-step elision on an empty workset (one host-side integer check skips forward+loss+backward+optimizer AND guarantees the count-based loss normalizer never divides by zero — guarded by construction), the Arc-shared pipeline set (WGSL validation + compilation paid once per run across loss/backward/optimizer constructors), the explicit 16-byte uniform pad discipline (explicit _pad u32 fields + size_of test pins — one fixed-size buffer helper serves all kernels, layout breaks CI instead of the GPU), and the count fused into its producing mutation (the masked count rides already-hot cache lines instead of a second O(n) scan; feeds normalization + the workset gate). Corpus 203 → 209 (C 22, D 31, E 13, M 7, N 12). Standing syn-parse gate extended (batch30_pseudocode_parses). 13 candidates dropped as duplicates/adjacent (GPU→GPU copy triply covered — the source comment cites Issue 018; per-call bind groups + naive downloads + CPU argmax = new SLOW instances of Batches 25/26 rules; host scratch reuse = cross-domain rust_perf reuse-allocation-clear; 3 RePlaid controller patterns out of kernel_opt scope, recorded in Research 031 §4). Mining bug yield filed as a riir-ai issue: unwrap_or_default swallowing GPU download errors (silently pins best_loss to 0.0 forever — the checkpoint-best gate can never fire again), the unit-incoherent RePlaid adapt() sign basis (loss EMA compared against a mask ratio), the adapt_ratios() sort orphaning tracker state, the zero-masked skip firing AFTER the expensive teacher forward in the distill loop, DistillMode::D2fWeighted dead config, and RePlaid being unwired dead code re-exported as public API with a misleadingly-named wiring test.
   Batch 31: 8 riir-gpu production-kernel rules (ragged-pack-offset-table-fused-dispatch, twin-projection-shared-delta-adapter, min-rank-inert-adapter-zero-alpha, debug-only-gpu-label-macro, sync-map-result-stack-cell, staging-reuse-grow-only-map-prefix, mapped-at-creation-init-upload, dtype-normalize-once-at-upload) from Research 033 — the eighteenth batch mining the stack's own kernels (the training-substrate cluster: lora/mod.rs + optimizer.rs + dataloader.rs + buffer.rs — the layers UNDER the dllm loop mined in Batch 30): the 12-per-adapter GEMV dispatches repacked into contiguous buffers + a prefix-sum offset table (one launch over Σ out_dim; ragged segments need offsets, a modulo cannot identify them), GQA K/V shape-twins sharing ONE adapter pair (adapters 6→5/layer, params −33%, KV delta cache halves at inference), structurally-required-but-inert adapters at rank=1 + alpha=0 (minimum legal footprint, annihilated by scale not contents — the ALLOCATION contract complement to Batch 30's B=0 VALUE contract), the cfg-split GPU label macro (debug = format!'d capture labels, release = a typed None constant), the map-async callback parking its result in an UnsafeCell under the poll(Wait) barrier (zero lock + zero alloc per readback — poll IS the happens-before edge), the loop-owned grow-only MAP_READ staging buffer (~50-500μs driver alloc amortized to zero steady-state, mapping only the copied prefix; merged from two subagent proposals), one-shot weight uploads via mapped-at-creation (no encoder, no submit, no queue-timeline ordering hazard), and bf16→f32 normalization once at ingestion (a pure bit-shift instead of a per-dispatch conversion forever). Corpus 209 → 217 (A 32, D 32, L 7, M 9, N 13). Standing syn-parse gate extended (batch31_pseudocode_parses). Mining bug yield: the CRITICAL AdamWOptimizer shared-uniform truncation (every recorded pass saw the LAST write's param_count — Mlp1.B/Mlp2.A silently trained only 25% at the 4× SwiGLU ratio in every GPU LoRA loop) FIXED as riir-ai Issue 679A (per-slot uniforms + full-length regression test verified to fail under the old aliasing at exactly 96/128) + 10 residual hazards filed as Issue 679 B1-B10.

   Batch 32a: 11 riir-gpu production-kernel rules (caller-owned-split-scratch, inplace-gate-scale-at-funnel-bottleneck, sign-branch-stable-sigmoid, incremental-multi-part-hash-no-concat, bool-fields-over-bitset-hot-reads, per-layer-grad-norm-lr-equalization, width-invariant-superset-training, superset-sized-scratch-across-variants, active-index-grad-scatter-subset-backward, inplace-grad-accumulate-deferred-postscale, fuse-loss-and-output-grad-one-pass) from Research 034 — the nineteenth batch mining the stack's own kernels (the edge_lora CORE cluster: edge_module.rs + types.rs + training.rs + adaptive_width.rs — host-side training orchestrators; first half of the edge_lora module per the 32a/32b split, 32b = self-play/arena/consolidation remains): the hot-path fn taking ONE caller-owned scratch carved by split_at_mut into intermediate+output with a borrowed-view return (trait-shaped allocating paths quarantined as cold adapters), the sigmoid gate running IN PLACE on the rank-wide intermediate AND absorbing the broadcast scale (rank ops not out_dim — placement-of-work at the funnel's narrowest point), the sign-branch stable sigmoid (exp's argument always <= 0; the naive form overflows f32 exp at x < -88.7 — the house gate-math style, matches katgpt-rs latent_to_raw_scalar), BLAKE3 hasher.update per tensor replacing the materialized a||b concatenation (the streaming update IS the concatenation — zero per-step allocation on the commitment path), the REGIME BOUNDARY of packing (per-tick-READ flags stay bools — decode ALU dominates; pack only when bytes streamed dominate — the inverse of the packed-weights rules), per-step per-layer grad-norm LR equalization min(max_norm/layer_norm, cap) combating a measured ~1000× vanishing gradient (zero-norm→1.0 rails), ONE superset weight set trained across difficulty-sampled widths serving every inference budget (anytime-inference; gate: width-1 ≥ 95% of width-4), scratch sized ONCE for the statically-known worst-case variant (caveat: exact-length consumer checks force per-variant allocs — relax to prefix-view semantics), backward in dense sub-space + precomputed active-index scatter into the full buffer (inactive params zero BY CONSTRUCTION — no masks, no atomics; the host-side shape of MoE backward), accumulate-RAW-then-ONE-post-scale (1 division total, not batch×params), and loss scalar + analytic output-gradient from the SAME diff in one sweep. Corpus 217 → 228 (A 35, B 37, C 24, D 33, H 10, L 8). Standing syn-parse gate extended (batch32_pseudocode_parses). Mining bug yield filed as riir-ai Issue 683: the AdaptiveWidthTrainer per-episode construction storm (edges.clone() + fresh TopologyTrainer + fresh ForwardCache + fresh EdgeGradients per EPISODE when only 3 schedules exist — root cause documented in-source as the exact-length cache check) + silently ignored TrainingConfig fields (weight_decay/grad_clip_norm/per_layer_lr_scale) + misreported mean_weighted_loss + the zeroed() doc causal error (B=0, not A=0) + route_into release-panic path + the additive-aggregation stale-out footgun.
   Batch 32b: 10 riir-gpu production-kernel rules (static-classification-hoist-to-routing-table, sigmoid-deriv-recovered-from-output, fuse-weight-grad-input-grad-single-matrix-visit, needless-swap-temp-hoist-to-scratch, scoped-init-at-consume-site, monomorphize-per-element-trait-calls, additive-sigmoid-gates-not-softmax-routing, commit-over-maintained-item-digests, verify-once-at-trust-boundary, derive-shape-from-payload-not-cached-field) from Research 035 — the twentieth batch mining the stack's own kernels (the edge_lora training half: topology_trainer.rs + chain_commit.rs + sigmoid_gate.rs + mod.rs — the topology-evolution trainer + chain commitment + per-edge additive gating; the 32b-1 half per the suggested split, 32b-2 = self-play/arena/consolidation remains): immutable-topology classification hoisted ONCE at construction into CSR-style routing buckets (the naive form re-scans an O(L) probe per edge per call in ALL THREE hot loops while edge_source_layers() already computes the mapping that only a cold path consumes), sigmoid' recovered algebraically from the cached OUTPUT (σ'(x) = σ(x)·(1−σ(x)) incl. through a scale factor — halves activation-cache traffic), the weight-grad outer-product write + input-grad transposed matvec read fused into ONE visit per B row (two gradient ROLES of one matrix), the false-premise defensive swap temp deleted (the comment claimed in-place aliasing but read/write hit DIFFERENT buffers — genuine ping-pong temps are loop-carried scratch state), blanket whole-scratch clear replaced by scoped accumulate-window zeroing (only ACCUMULATION targets need pre-zero; O(total scratch) → O(active window)), per-element &dyn devirtualized to generic + ?Sized (the same file's backward was already generic — the inconsistent pair was the evidence), per-edge independent sigmoid gates composed additively instead of softmax routing (no cross-edge reduction barrier; multiple edges simultaneously near 1.0 by design — the house sigmoid-not-softmax rule as a corpus rule), container commitments digesting maintained per-item digests (O(items) not O(total payload bytes)), verification ONCE at the trust boundary via a capability type (the load path paid up to 3 full BLAKE3 passes), and shape truth derived from the payload not a cached field (the merkle_root-class desync bug structurally impossible). Corpus 228 → 238 (A 39, B 39, C 25, D 35, E 14). Standing syn-parse gate extended (batch32b_pseudocode_parses + batch32b_rules_present + batch32b_concepts_covered). Mining bug yield filed as riir-ai Issue 684: the topology_trainer hot-loop allocation storm (per-node vec! on a FALSE aliasing premise ×2 sites + per-edge to_vec() borrow workaround + backward staging buffer) + the O(E·L²) routing re-derivation ×3 loops + dead/redundant scratch clears (gates_scratch/edge_outputs never read in production) + the triple-BLAKE3 load path + the 7-site sigmoid duplication + the discarded topology validation error + silent u32→u16 narrowing.
   Batch 32c: 13 riir-gpu production-kernel rules (incremental-centroid-running-sum, length-prefix-shape-faithful-hash, order-stable-topk-via-index-sort, make-unmake-over-clone-per-candidate, argmax-rng-tie-break-collect-ties, shape-gated-warm-start-slot-copy, domain-as-data-not-algorithm-fork, one-sweep-spatial-feature-fanout, deterministic-tiling-projection-tables, latent-encode-at-capture, non-saturating-outcome-reward-floor, collect-then-resolve-snapshot-batch, timeout-scalar-tiebreak-decisive-outcome) from Research 036 — the twenty-first batch mining the stack's own kernels (the edge_lora self-play/arena/consolidation half: sleep_consolidation.rs + arena_eval.rs + cross_game_edge.rs + cross_game_real.rs + {go,bomber,fft}_self_play.rs — the 32b-2 half; the edge_lora module is now FULLY MINED): the greedy clustering loop recomputing each centroid from scratch per comparison (full member sweep + fresh Vec per (item, cluster) pair) replaced by a maintained (sum, count) per cluster (the numeric-aggregate twin of digest maintenance), payload-only multi-part hashing feeding part LENGTHS into the stream first (shape-blind otherwise: a=[1,2],b=[3] collides with a=[1],b=[2,3]), top-K retention via sort-index-truncate-re-sort (items never move, canonical output order, NaN-safe comparator), candidate search make/unmake instead of whole-state clone per probe (bit-identical rollback, zero per-candidate alloc), argmax with collect-epsilon-ties + one seeded RNG draw (vs max_by's hidden first-max priority or softmax's normalization), warm-start slot copy gated on the full shape tuple, cross-game training as DATA conversion over the unchanged loop (no forked trainer — correctness inherited), 12 spatial statistics from ONE board sweep (classification indices + signed indicator + geometry-derived denominators), a modelless fixed projection as construction-time tables (modulo schedule + even-spacing-with-jitter phases — same seed rebuilds the identical encoder), latent encoding snapshotted at capture (peak memory moves × hidden_dim, not moves × full state), terminal outcomes ramped off the extremes (0.15 + 0.70·clamp — losses keep non-vanishing gradient weight), simultaneous-turn ticks split into collect-against-frozen-snapshot then resolve-in-fixed-order (no information leak, parallelizable select), and timeout falling through to a maintained-scalar HP tiebreak instead of a flat draw. Corpus 238 → 251 (A 42, B 42, C 27, D 39, E 15). Standing gates extended (batch32c_pseudocode_parses + batch32c_rules_present + batch32c_concepts_covered; awk orphan-rule_id pre-compile scan clean — the Python-generator landing produced a clean first compile). Mining bug yield filed as riir-ai Issue 685: the quadratic centroid storm + swallowed failed move + wasted encodes on Pass moves + warm_start weight duplication + per-call dataset deep-clone + the false-premise sigmoid comment (named sigmoid, computes an affine ramp) + dead init-touch loop + Wait-vs-continue mismatch + per-tick nested-Vec grid rebuild + the adapter-skeleton triplication family (7 duplicated shapes × 3 game adapters — the self-play half of the Issue 684 sigmoid dedup family).
   Batch 33: 7 riir-gpu production-kernel rules (grid-over-output-thread-private-sweep, buffer-set-struct-two-tier-dispatch, slot-offset-partial-buffer-write, immutable-operand-upload-at-set-creation, selective-projection-effective-rank-head, derived-scalar-host-computed-in-uniform, rademacher-signs-over-gaussian-projection) from Research 037 — the twenty-second batch mining the stack's own kernels (the spectralquant calibration cluster: calibration.rs + rotation.rs + qjl.rs — the calibration path Research 016 explicitly deferred; the spectralquant/ directory is now FULLY MINED; bench-only today, sole consumer spectralquant_bench.rs): the output-owned grid + thread-private seq_len sweep (zero cross-thread reduction, no partials, no reduce kernel — the dual of split-K), the buffer-set struct as reuse unit with a two-tier dispatch API (preferred entry only writes uniform content; raw entry self-allocates for cold callers), slot-offset delta writes mirroring the kernel's base index (d_h×4 bytes not the whole tensor), immutable operands uploaded ONCE in the constructor (dispatch never sees operand bytes again), selective projection on the effective-rank head only (operator + FLOPs scale with d_eff ≈ 4–6, not d_h = 128 — ~21× on both; sibling of water-fill: bits-by-importance vs projection-work-by-importance), config-derived scalars computed once on host + shipped in the uniform (CPU reference and kernel consume bit-identical f32 by construction; WGSL sqrt is 2.5-ULP not correctly rounded), Rademacher ±1 signs over Gaussian (one coin flip per entry, ±1.0 exact in f32 so every product is exact; conditional-negate strength reduction licensed but unrealized — honest caveat in the rule). Corpus 251 → 258 (A 44, B 44, C 27, D 40, E 16, N 14). Standing gates extended (batch33_pseudocode_parses + batch33_rules_present + batch33_concepts_covered; 8 clippy doc warnings fixed pre-commit; drive-by fix of the stale M-category (7 rules) comment → 9). ONE seed REJECTED on verification: two-stage tile→reduce is NOT what covariance.wgsl does — the producer writes no tile partials, so n_tiles > 1 would reduce uninitialized memory (filed as riir-ai Issue 686 H1 instead of becoming a rule — encoding it would have put a bug in the corpus as wisdom). 11 further candidates dropped as duplicates/thin. Mining bug yield filed as riir-ai Issue 686: the n_tiles>1 silent-corruption path + layer_offset dead in eigenbasis_rotation.wgsl (multi-layer rotation silently reads layer 0) + total_layers dead + per-dispatch bind-group creation in all four dispatchers + create_uniform_buffer doc-lie + duplicated host/kernel stride contract + 2-barriers-per-position smem staging + dead code + tautological test + latent shared-uniform ordering hazard.
   Batch 34: 9 riir-gpu production-kernel rules (exact-op-parity-for-analytic-backward, mirror-ring-store-sliding-window-contiguity, split-dispatch-expose-fused-intermediate, preallocated-activation-slots-inplace-save, hybrid-cpu-elementwise-gpu-throughput-sync-points, pre-transposed-consumer-layout-handles-build-once, shape-matched-sibling-weight-grads-share-input-loop, op-major-backward-loop-inversion-batch-sequence-axis, backward-helper-expose-shared-norm-statistic) from Research 038 — the twenty-third batch mining the stack's own kernels (the Gemma-4 TRAINING cluster: gemma4_cubecl_train/mod.rs + backward_gpu.rs — hybrid GPU training forward + Phase 2.5 GPU backward; the training counterpart of Batch 25's inference-side Gemma cluster): the training forward evaluating the EXACT op the analytic backward differentiates (standard tanh not fast_tanh — forward value + derivative from two different functions = systematically biased gradients), the 2·sw mirrored KV ring making any ≤sw window contiguous by construction (read contract raw-indexed, valid while seq_len ≤ 2·sw), the fused chain that discards its intermediate forcing a producer re-launch (ship a split entry returning both outputs from ONE execution — gemma2's dispatch_attention_wo_split is the proven precedent), activations written into pre-sized [seq][dim] slots at the moment of production (forward loop alloc-free regardless of save count), the hybrid CPU/GPU boundary with exactly 4 syncs per layer per position (GPU takes only throughput-bound GEMV/attention; elementwise runs CPU-side in the sync gaps), backward's W^T materialized ONCE as resident handles under the frozen-base contract (~2× memory, triple cross-model recurrence: gemma2 + gemma4 + gemma4_q4k), shape-matched sibling weight-grads sharing ONE walk of the shared input (wk+wv over normed_attn, gate+up over normed_mlp — honest caveat: L1-resident CPU inputs make it loop-overhead halving), the op-major backward loop inversion (positions free inside each op stage — attention backward sits BETWEEN stages — so per-position GEMVs become sequence-batched GEMMs, 7/layer/step not 7×seq_len), and the backward helper exposing the norm statistic it already computed (rmsnorm_backward must return inv_rms, not hide it from the gamma-grad path — 2-3× reduction waste at 7 sites). Corpus 258 → 267 (A 46, B 44, C 28, D 43, E 16, H 11, L 9, N 15). Standing gates extended (batch34_pseudocode_parses + batch34_rules_present + batch34_concepts_covered; 8 clippy doc warnings fixed pre-commit incl. a NEW lint variant — a leading `+` on a wrapped doc line parses as a markdown list item). TWO seeds resolved by verification: the softcap backward does NOT recompute tanh — it algebraically recovers it from the cached POST-cap logits (already sigmoid-deriv-recovered-from-output; falsified by the subagent), and the dual-KV-cache "pattern" was REJECTED as wisdom (avoidable 2-3× duplication — filed as Issue 687 H9). Mining bug yield filed as riir-ai Issue 687 (13 hazards): the mirrored-ring backward OOB panic for seq_len > 2·sw (index algebra verified — correct below the boundary, no assertion anywhere), backward-handle staleness under replace_weights + the self-contradictory LoRA doc (base-only transposes drop the ΔW^T·g term under the merged-weight convention — found independently by both subagents), Phase 2 processing zero-gradient positions (only Phase 1 skips), the ~2.06M-alloc/27GB-per-step + 345,088-round-trips-per-step storms (quantified at real 12B dims), the CPU weight-grad accumulation being the ACTUAL per-step bottleneck (wte grad 17× the largest layer grad; the header's GEMV claim is per-op true, per-step misleading), the 24-line byte-identical CE-backward duplication, the dispatch_attention_only re-run, dual CPU-side KV caches, the RoPE freq-table dual derivation (currently identical — drift = silent gradient corruption), the norm-statistic recomputation, the naive strided transpose at 12B scale, + the fast_tanh header misattribution (GPU inference uses standard tanh — grep-verified).
   Batch 35: 8 riir-gpu production-kernel rules (mask-sentinel-matches-reduction-identity, smem-staging-gated-by-cross-thread-reuse, kernelize-host-chain-after-sync-batching-floor, block-reduction-separate-scratch-runtime-strides, saturating-nonlinearity-recovery-breaks-near-cap, adapter-grad-join-mirrors-forward-delta-insertion-point, side-consumer-readback-alias-live-handle-for-dependent-dispatch, quadratic-attention-intermediates-recompute-from-linear-saves) from Research 039 — the twenty-fourth batch mining the stack's own kernels (the Gemma-2 TRAINING cluster: gemma2_cubecl_train/mod.rs + attention_gpu.rs — the Issue 430 Option B fused causal attention kernel + the hybrid LoRA training forward/backward; the training counterpart of Batch 25's inference-side Gemma cluster; resident.rs 1,639 deferred to a later batch): masked lanes PARTICIPATE in a block reduction by writing the identity element with the sentinel REWRITTEN between reductions of different identity (neg-inf for max, 0.0 before the sum; non-reduction consumers bound at the valid extent), smem staging gated by cross-thread reuse (stage iff within-workgroup reuse > 1 — the single-use K row streams from global in the SAME dot loop), the kernelize-past-the-batching-floor decision (a host-serviced segment floors at 2 syncs after all batching; kernelize when intermediates are chain-local + no host work in the gap — the 52 = 26 layers x 2 decomposition verified post-batching in resident.rs), block reductions into SEPARATE scratch with strides from the runtime extent (both failure modes — in-place + hardcoded unroll — shipped and failed G1 on 4090 before the fix), the saturation boundary of derivative-recovery-from-cached-output (1 - t^2 = exactly 0 once the output rounds to 1.0 in f32; the codebase recomputes the RAW argument at attention but keeps the recovery at the LM head where 9 x cap ~ 270 is unreachable), the adapter-grad-join placement mirroring the forward's additive delta insertion point (additive CPU-side LoRA deltas on GEMV outputs make the base transposes IMMORTAL by construction — no invalidation surface; the merged convention silently drops the delta-W^T g term, Issue 687 H2's class), reading an intermediate for side consumers while aliasing the still-live handle for the dependent dispatch (the re-upload burns ~53 MB/step at seq 256 — the Q4_K sibling is the standing proof), and saving Theta(N x d) attention tensors while recomputing the Theta(N^2) chain in hoisted scratch (the recompute also yields the raw pre-softcap argument the saturating backward needs). Corpus 267 → 275 (A 49, B 45, C 29, D 43, E 17, H 12, L 9, N 16). Standing gates extended (batch35_pseudocode_parses + batch35_rules_present + batch35_concepts_covered; 4 doc-markdown backtick warnings fixed pre-commit; a SIXTEENTH count-pin site surfaced via test failure — the kernel_categories_counts_match_plan_003_table per-category expected-counts table). ONE coordinator seed KILLED by verification ("thread role rotation" is ambient fused-kernel structure, not decision content — subagent falsification now 3-for-3 across batches) + ONE seed CORRECTED (the 52 syncs is the POST-batching floor, not pre-batching — that is what makes the floor rule distinct from the existing batching rules). 12 candidates dropped as duplicates/thin (the predicted dedup pressure from the Batch 34 sibling + Batch 25/26/29 neighbors: the grid -> derive-params, GQA/softcap -> B23, the sweeps -> B33, params/launch -> B29, h-recompute -> the B34 helper rule). Mining bug yield filed as riir-ai Issue 688 (15 hazards): the 3-way sync-count doc inconsistency + the stale "8-step unrolled" module doc, ~1.1 GB/step alloc churn from the unused _into variants + the engine doc-lie claiming the GPU path uses them, ~82 MB dead activation slots, the per-launch params buffer (26 device allocs/step for 9 never-changing floats), the 256-constant triplication, un-asserted q_dim/kv_dim derivation, the KV-cache liveness contract, per-layer grad-buffer churn, 262 MB EngineActivations per step, the 2.6x handle-memory doc understatement, the h-recompute ~9.7 G MACs/step, + the sliding-window divergence note.
   Batch 36: 7 riir-gpu production-kernel rules (constant-scale-folded-into-upload-copy, side-branch-operand-residency-matches-mutation-lifetime, activation-checkpoint-as-handle-clone-deferred-batch-readback, training-forward-shed-decode-loop-shape, sibling-dispatches-before-any-readback, independent-cpu-work-fills-dispatch-read-window, grad-terms-merge-before-shared-link-jacobian) from Research 040 — the twenty-fifth batch mining the stack's own kernels (the Gemma-2 RESIDENT training cluster: gemma2_cubecl_train/resident.rs — the Plan 481/482 all-GPU LoRA training path (per-position GEMV forward + batched GEMM forward + batched backward); completes the Batch 35 cluster, gemma2_cubecl_train/ now FULLY MINED): a hyperparameter-derived CONSTANT scale (LoRA alpha/rank) folded into the byte-level upload COPY so no kernel ever applies it (fold-into-copy, never the source — the backward re-derives from the unscaled original), an aux computation riding a GPU hot path keeping ITS OWN operands device-resident at the step boundary (the middle rung between config-lifetime and call-local; the hybrid CPU-LoRA path paid ~70K sync round-trips and ran 4.8x slower than pure CPU), the checkpoint medium during compute being the device-handle clone (a refcount bump that cannot stall) with the tail read_batched collapsing 364 syncs to 26, shedding the inherited DECODE loop shape in the training forward (the incremental KV cache is the structural blocker; make attention whole-sequence — ~55K dispatches -> ~182), queueing ALL independent sibling dispatches BEFORE the FIRST readback (the interleave serializes completely; applies with DIFFERENT input buffers), hoisting read-independent CPU work INSIDE the dispatch->read window (the LoRA param-grad half is the canonical hoistable work — visible-but-unexploited in source), and SUMMING multiple loss terms at a shared nonlinear link's INPUT with ONE Jacobian application (adding a term after the Jacobian is silently WRONG, not just slower). Corpus 275 → 282 (A 49, B 45, C 30, D 48, E 17, H 13, L 9, N 16). Standing gates extended (batch36_pseudocode_parses + batch36_rules_present + batch36_concepts_covered; 7 doc-markdown backtick warnings fixed pre-commit). FOUR coordinator seeds falsified by verification — the highest count yet (F4 the per-position path already caches its cos/sin table via RopeCosSinCache + Batch 17 had explicitly dropped cos/sin-table rules; F5 the env-var A/B gate is GOAT-gate process, its kernel content already in the B35 floor rule; B2 the ping-pong swap is needless-swap-temp fix(2)'s loop-carried scratch; B4 the scratch hoist is a 3-rule composition) + 2 premise corrections (F3 the per-position path's "ONLY sync point" doc-lie — 93,440 unbatched reads; B1 q/k/v consume different inputs so fan-out's premise doesn't apply). 1 weak candidate dropped (phase-disjoint buffer aliasing — 9 KB local win). Mining bug yield filed as riir-ai Issue 689 (15 hazards): the ~10.2M-transcendental CPU RoPE recompute per forward from not saving k_post_rope (P2, cheapest fix — one handle clone), the 93,440-read unbatched legacy readback + its doc-lie, the 262 MB g_logits_all fresh alloc + dead zero-fill per backward step, the comment-lie g_attn_out_vecs per-layer allocations (26 x seq_len Vecs/step), the nested<->flat reshape round-trip ~327 MB/step, read_handle fresh-Vec ~1.2 GB/step download churn (the read_handle_into fix substrate already in-tree at kimi_k3), missed CPU/GPU overlap windows, the rmsnorm scratch->dest copy API asymmetry, + 7 more.
   Batch 37: 8 riir-gpu production-kernel rules (kv-window-device-resident-ring-append, lm-head-loss-positions-only, verbatim-upload-on-format-match, stream-drain-free-source-during-build, identical-tensor-handle-clone-zero-reupload, sparse-gather-scale-at-read-not-table-prepass, tiled-host-transpose-before-quantize, materialized-fast-path-bulk-slice-accessor) from Research 041 — the twenty-sixth batch mining the stack's own kernels (the Q4_K TRAINING cluster: gemma4_q4k_train/mod.rs + weight_buffers.rs — the QLoRA frozen-Q4_K-base + f32-LoRA-delta Gemma-4 trainer; forward_batched.rs + backward.rs deliberately deferred to a future batch on predicted dedup pressure): a KV cache mutated by append + consumed every position lives ON the device as a persistent ring (only the NEW row crosses the bus — append-vs-snapshot transfer; the sliding path re-uploads ~40 GB/step), the loss-support set (pos >= n_in-1) gating the vocab-head GEMV (prompt positions' lm_head + readback + save are provably dead via the backward's d_logits.is_empty() skip — ~127 MB/step dead), on-disk quant format == device target format → cast the mmap + upload raw blocks (NEVER dequant-to-f32-and-requantize — the 4090 OOM class; proposed independently by BOTH agents, merged on collision), a one-time build draining the source per unit (peak host memory O(unit), not O(model) — the build that lets a 32 GB box build 12B GPU buffers), a tensor legally absent from the export because identical to a sibling → clone the device handle (refcount bump, zero re-upload — the GGUF V-shares-K case), a sparse read set + shared unscaled consumer selecting the GATHER as the scale fold point (the deliberately INVERSE regime of B36's upload-copy fold — encodes the fold-point DECISION), cache-blocked host transpose (naive inner-c store strides rows*4B = 15 KB — every store a cold line, each 64B dest line written 16x), and a bulk Option<&[f32]> accessor branching ONCE per step (the materialized arm runs direct-index + rayon, not per-row enum dispatch). Corpus 282 → 290 (A 52, C 31, D 50, L 10, N 17 — note sparse-gather landed in C beside its inverse twin constant-scale-folded-into-upload-copy, not the proposed B). SIX seeds falsified by verification (highest-per-agent yet: F1 rope tables ARE cached once — 48 layers share 2 via rope_table_idx; F3 LoraSite::none() is the correct frozen-base skip, not B31's inert adapter; F4 gammas built once at construction; F5 the 3 backward constructors field-identical — no merkle_root-class miss; F4' the setup path is pure enqueue, 0 round-trips; F5' the GEMV path already ships packed Q4_K + in-kernel dequant = the B15 direction) + 2 confirmations with regime corrections that became rules (scale-at-gather is CORRECT for a sparse read set; quantize runs once but is paid TWICE: forward + transposed backward ≈ 14 GB on a 24 GB 4090). 1 rule dropped as dedup (variant-dedup-constant-table-by-index — share-pipeline-set-across-subsystems + B17's rope-table-drop precedent). Mining bug yield filed as riir-ai Issue 690 (~19 hazards): 43,008 blocking reads/step on the unbatched path (the file's own Bench 433 doc), the dead lm_head for prompt positions, the dual-KV-cache quadruple-write ~402 MB/step, the O(S^2) full-attention window collects ~537 MB/step/layer (rows already contiguous — pure waste), ~40 GB/step sliding-window re-upload, the 2x GPU weight residency, one-time dead zero-inits 7 GB + 48 GB, two doc-lies, + more, with a 6-step fix order. GOAT: 327 kernel_heal + 201 default tests, clippy 0 in all 4 states (45 doc_lazy_continuation + 8 backtick warnings fixed pre-commit — root cause: line-initial "+ " in //! list-item continuations parses as a NEW markdown list item, flagging every subsequent line; plus the B36 gotcha again: bare lm_head/Q4_K/QLoRA/RoPE/.rs filenames need backticks). Generator lesson: TWO slice-anchor misplacements caught by the count-pin test (the B/C and L/M confusion — slice-tail anchors are only safe when extracted from the enclosing KernelCategory::X block, never by line-number proximity); ONE malformed edit_file op (schema-foreign "replace" key) silently truncated a header sentence — caught + restored; a git stash near-miss (self-caught, pop verified complete — the correct comparison is git show HEAD:file, never stash).
   Batch 38: 4 riir-gpu production-kernel rules (env-flag-resolved-once-not-per-dispatch, flat-activation-slots-as-readback-target, transient-spike-pins-persistent-scratch, transposed-consumption-accumulate-not-gather) from Research 042 — the twenty-seventh batch mining the stack's own kernels (the Q4_K TRAINING cluster, second half: gemma4_q4k_train/forward_batched.rs + backward.rs — the layer-major batched forward, the 43,008 → 337 dispatch fix, + the per-position and batched backwards; the cluster is now FULLY MINED): std::env::var resolved ONCE at construction or behind a cheap guard, never per dispatch/layer/token (97 lookups/step in one trainer; one instance IN FRONT of the autotuner's deliberately lock-free papaya cache hit — the env lock poisons the exact property the cache was chosen for; the correct live-toggle form exists at gemma2_cubecl/mod.rs L1353-1358: cheap guard BEFORE the env read), nested [pos] → Vec<f32> activation slots force a scratch + scatter two-copy tail after every batched readback (flatten the container to [seq × dim] so the slot IS the readback target — the container layout IS the copy count; distinct from preallocated-activation-slots which retains the nesting and therefore the scatter), a one-shot resize(batch × vocab) ≈ 134 MB pinning a persistent RefCell scratch at ~8× its recurring max (grow-only is right for RECURRING sizes and wrong for a non-recurring spike — transient oversized phases use a local buffer dropped at phase end), and z = Bᵀ·g against row-major B via i-outer row accumulation instead of k-outer column gather at stride rank × 4B (up to 16× read amplification; the forward/backward asymmetry on the SAME buffer is the tell; summation order changes — fine for grads, NOT for bit-identity gates). Corpus 290 → 294 (A 55, D 51). ONE subagent rule KILLED by coordinator math (zero-gradient-position elision: K/V cross-position coupling re-introduces non-zero gradient at prompt positions after the topmost layer — support queries attend to prompt keys — so only the topmost layer's pre-attention chain + the head can skip; the SAME flag applies to Issue 687 H3's ~50% magnitude claim, which is ~1/48 — 687 flagged for its owner, not modified). Dedup kills: the backward's per-layer RoPE powf table rebuild (third strike — B17 drop + B36-F4 + B37-F1), sibling-batch region packing, resize-churn, rayon-attention direct-write (a 2nd in-file instance of materialized-fast-path), LoRA-delta add-pass fusion (B19), the RefCell-scratch API shape (ambient in 3 rules). Mining bug yield filed as riir-ai Issue 691 (headline: BOTH backward variants have ZERO in-tree callers — the 941-LOC file is unreached, its math unpinned; then dead x_normed compute in both Phase 1s, 48× per-layer RoPE table rebuilds while the trainer caches bit-identical tables, the ~94 MB/step clone storm, ~380 MB/step fresh output-Vec churn + ~380 MB/step redundant hidden re-uploads (5×/layer, create_from_slice per dispatch — with the Bench-437 nuance that upload-sharing ALONE was never tested), the 134 MB scratch pinning, the dead attn_scores field, the strided LoRA z-gather at 2 sites, 2 doc inconsistencies). GOAT: 330 kernel_heal + 201 default tests, clippy 0 in all 4 states. Generator lesson re-learned: bare json.dumps WITHOUT ensure_ascii=False produced \u2014 escapes in Rust string literals → 33 compile errors (the 32c lesson, now twice-observed — treat ensure_ascii=False as non-optional); a 17th count-pin site surfaced (the stale "282-rule" comment in kernel_categories_cover_all_corpus_rules_exhaustively — B37 missed it).
   Batch 39: 9 riir-gpu production-kernel rules (autotune-cache-extract-outlives-owner, append-cache-rewind-overwrite-in-place, slow-path-bootstraps-raw-buffer-hotswap-cache, entrypoint-variants-share-layer-body, elementwise-offload-needs-device-side-anchor, tiny-param-buffer-rebuild-over-cache, readback-destination-passing-free-fn, variant-owned-output-buffer-clone-escape, pipeline-layer-boundary-device-handle) from Research 043 — the twenty-eighth batch mining the stack's own kernels (kimi_k3_gpu_forward.rs 2,601 LOC — the KimiK3 full-model GPU decode forward: hybrid CPU/GPU stack, 7 forward entry-point variants, dual weight-update paths, MLA KV cache rewind for the Ouro loop, + the dense SiTU FFN helper family; the largest single unmined file): salvage the autotune cache across owner destruction via Arc inject/extract (re-bench ~5-10 min vs upload ~64 ms — the ownership-lifecycle complement to hot-swap), re-execution workloads OVERWRITE the append-only cache via rewind_to(pos) never re-append (O(max_seq) not O(iters × max_seq); recurrent-state layers are structural no-ops), the in-place weight-update two-tier bootstrap (first call runs the slow allocate path + harvests raw wgpu buffers into ORDER-keyed slots; subsequent calls queue.write_buffer in place — training weights change identity every step so pointer-keying is useless), versioned entry points multiply but the per-layer loop body stays shared per axis (forking the whole body guarantees divergence — observed TWICE in one file: a resident fork that initially called the OLD unbatched kernels + two saved forwards whose kernel-variant selection diverged under the same env vars), a GPU op sandwiched between upload+readback is compute-offload ONLY (the win requires a device-side anchor consumer — found INDEPENDENTLY by both mining agents), 3 f32 = 12 bytes rebuilds beat caching below a cache line, GPU readback helpers should be destination-passing free fns (dst: &mut [f32] not -> Vec<f32>; the free-fn form also splits the struct borrow — the API-level half of the Issue-689 churn fix), enum-variant-owned output buffers force per-layer-per-token clones out of the match arm (hoist to the parent + free fns over disjoint fields; merged from both agents — the free-fn extraction alone does NOT fix the clone), and the Handle IS the pipeline layer-boundary currency (host materialization becomes a layer-internal detail — later fusion composes without boundary changes). Corpus 294 → 303 (A 60, C 32, D 53, K 5). TWO rules merged from independent two-agent convergence (the anchor rule + the clone-escape rule). 13 dedup kills/falsifications (update_weights = B25 verbatim; OURO_* env reads = the B38 instance; inv_rms double-compute ×6 = B34 forward-side; the 4-buffer drain = B36+B26; the saved clones are checkpoint captures, NOT waste — B34 instance). Mining bug yield filed as riir-ai Issue 692 (14 hazards: the dead 640 MB embed_handle uploaded at construction + retained forever despite the Issue 402 comment proving it dead; the per-layer-per-token FFN clone/fresh-alloc storm (6+4 sites) across ALL variants; the two saved forwards' ~310-line divergence with LOST OURO env dispatch; contradictory sync-count docs 7→3 vs 3→1; gate+up duplicated GPU upload vs the doc's sharing claim; vocab logits double-copy ×4 sites; + 8 more). GOAT: 333 kernel_heal + 201 default tests, clippy 0 in all 4 states. The O() expressions joined the doc_markdown gotcha list (O(max_seq) needs backticks in //! prose).
   Batch 40: 7 riir-gpu production-kernel rules (host-scratch-mirrors-slot-order-for-coalesced-writes, write-plan-derived-from-cache-slots-not-recomputed, cpu-source-builder-skips-device-roundtrip, expert-loop-two-phase-split-batch-across-iterations, rmsnorm-backward-gamma-scope-correction-term, per-position-scalar-factored-gemv-collapse, foreign-accumulator-bridge-copy-hoist-to-outer-loop) from Research 044 — the twenty-ninth batch mining the stack's own kernels (kimi_k3_gpu_backward.rs 4,332 LOC — the KimiK3 GPU backward: the LARGEST FILE in the crate; hybrid CPU/GPU gradient stack + dense SiTU FFN / MoE / MLA / KDA backwards; the backward half of the Batch 39 forward): the host scratch layout IS the write-call count (size to the SUM of slot lengths + carve with split_at_mut in cache slot order so the batched writer coalesces adjacent device buffers; the anti-instance sits 60 lines from the positives in the same file), the mirrored hand-maintained slot list IS the bug factory (boot pushes slots on handle-presence while the fast path recomputes lengths on handle-AND-source-presence — the guards drift + the only tripwire is a debug_assert compiled out in release, where shifted writes land in WRONG buffers + the tail goes OOB; derive the plan from the cache's own size_bytes), when the host owns the source bytes the *_from_cpu twin deletes the per-matrix read_one syncs entirely (batching is the lesser fix; GPU-source twins are boot-only), a per-iteration helper encapsulating launch→read→CPU→launch→read hides cross-iteration independence (split at the CPU-dependency point + batch each half across ALL experts: 2+2k_r+2 syncs to ~3), in RMSNorm backward gamma multiplies ONLY the direct dy term never the normalization correction (the slip is invisible at near-unity gamma init under percent tolerances — found LIVE WRONG in the production MoE backward while the same file carried the correct form), factor data-dependent per-position scalars out of GEMVs by linearity but keep scaled and unscaled consumers on separate accumulators (the fused form is the live d_attn_w wrong-math bug), and bridging to a foreign crate's whole-struct gradient accumulator via full-state copy-in/out per token is a gradient ping-pong (hoist the bridge to the outer loop). Corpus 303 → 310 (A 64, D 54, H 14, N 18). ONE proposal dropped (rayon parallel transforms — thin vs rust_perf generic guidance). PRUNER root-cause fix shipped (the L2 wrap strategy glued the closing brace into a trailing line-comment producing false parse failures; hardened with boundary newlines — the corpus found a pruner bug). Mining bug yield filed as riir-ai Issue 693 — the biggest of the mining program: TWO coordinator-verified critical wrong-math bugs on the production training path (MLA d_attn_w spurious attention-weight factor L2576-2584 + MoE latent-norm gamma over-application L1537 — masked by near-unity gamma init + loose tolerances + two-wrongs-agree parity), a NaN-VACUOUS parity suite (the LCG test generator leaks random bits into the f32 exponent field — the [2,4) comment is false; both max-statistics silently skip NaN making 4 of 5 tests pass vacuously), one ACTIVELY-FAILING KDA test (diff 4.95e27, empirically observed), the feature in NO CI gate, + 15 more hazards. GOAT: 336 kernel_heal + 201 default tests, clippy 0 in all 4 states (1 doc_markdown fix: 2+2k_r-style math with subscript underscores needs backticks).
   Batch 41: 6 riir-gpu production-kernel rules (concurrent-disjoint-write-buffer-parallelizes-uploads, aliased-slot-allocate-replace-fallback-severs-binding, pool-slot-write-coalescing-owns-only-declared-ranges, pool-reservation-token-outlives-raw-view, gpu-type-bridge-borrows-not-clones, superseded-primitive-machinery-zombie) from Research 045 — the KimiK3 backward SEQUENCE production caller (kimi_k3_gpu_backward_sequence.rs 1,328) + weight_buffer_cache.rs 459 (the write substrate; the ORIGIN of the B39 hotswap-cache pattern); corpus 310 → 316 (A 69, E 18). Issue 693 H1/H2 production routing pinned (both wrong-math bugs inherited via the batched twin); hazards filed as Issue 694 (orphaned-write fallback on cuda_backend — GEMVs read boot weights forever; zombie LM-head ~640 MB/step + ~1.3 GB dead VRAM; Ouro GPU-source update path ~237 read syncs).
   Batch 42: 4 riir-gpu production-kernel rules (refine-loop-invariant-prefix-elision, softmax-entropy-via-shift-identity, runtime-identity-gate-cached-verdict-elides-side-projection, expected-embedding-transposed-gemv-resident-head) from Research 046 — the Gemma-2 D2F block-causal decode cluster (gemma2_d2f/mod.rs 1,579 + gemma2_d2f_sc.rs 800, now FULLY MINED): the denoise loop re-forwards the PROVABLY-invariant prompt prefix every step (masking closes it; mutation-set closure proves it — O(steps × seq) → O(seq + steps × |S|)), ln(e_t) = logit_t − max so the entropy sum needs ONE transcendental not N, content-classified inertness cached write-through at the mutation boundary gates whole-stage elision (the gate must cover producer AND consumer), and the soft-token expectation x̂ = Σ p_v · wte[v,:] factors as the tied lm_head transposed GEMV on the already-resident table (not a 2.36 GB/position/step host mirror walk); corpus 316 → 320 (A 70, B 46, D 56). 18 falsifications (the file is a museum of already-mined anti-patterns). Hazards filed as Issue 695 (KV GPU→CPU→GPU round-trip 1.7 GB/block, invariant-prefix recompute ~227 GMACs, 46.6K blocking reads, hardcoded softcap divergence, truncate panic, the unreachable trained-SC path + the ungated x̂₀ producer).
   Batch 43: 5 riir-gpu production-kernel rules (mutable-input-scratch-residency-write-per-call, topk-select-nth-over-full-vocab-sort, post-scatter-renorm-support-set-only, stack-array-row-view-bridge, empty-slice-sentinel-launders-shape-violation) from Research 047 — the Weaver speculative-decode GPU corrector cluster (weaver_gpu_corrector.rs 1,449 + weaver_gpu_dflash.rs 576, Plan 436 Phase 4; the weaver_gpu/ kernel dir is a SEPARATE cluster, still unmined): per-iteration create_from_slice input churn → scratch-resident handles written per call via client.write (the file's own batched twin is the positive contrast), O(V log V) full pair sort → select_nth_unstable partition + K-prefix sort over a u32 index buffer (composes with order-stable-topk), dense re-sum + rescale after a K-scatter → sum-during-scatter + support-only rescale (bit-identical by IEEE — +0.0 adds exact, 0×inv = +0.0), Vec<&[f32]> heap collect → a fixed [&[f32]; 64] stack bridge (the speculative-decode idiom at 5+ in-tree sites), and a defensive &[] sentinel type-checks past the shape contract into a swallowed Err (the negative space of launch-unchecked); corpus 320 → 325 (A 73, B 47, D 57). 23 falsifications (Issue 468 P0/P2 already shipped several rules' fixes — the per-depth vs batched paths sit side by side as naive/opt contrast). Hazards filed as Issue 697 (the 3-layer silent no-correction chain: undersized capture → &[] row → HiddenShape Err → let-_= swallow; the release-mode embedding OOB window ~2.1 GB from a debug_assert-only length check; ~29 alloc requests/call on the hot path; test-only reachability with no CI row).
   Batch 44: 9 riir-gpu production-kernel rules (row-dot-split-inner-axis-block-reduce, fuse-indirect-gather-into-dot-consumer, share-cube-size-const-between-kernel-and-launcher, stash-kernel-internal-intermediate-in-smem, hoist-bit-address-out-of-packed-row-sweep, fold-tile-transpose-into-dequant-scatter, bit-cursor-unpack-to-width-specialized-byte-batch, single-sweep-minmax-for-quant-scale, serialize-container-canonical-order) from Research 048 — the Weaver GPU KERNEL substrate + the KVarN quantization cluster (weaver_gpu/mod.rs 1,657 — the 19-dispatch kernel implementations behind the Batch 43 corrector: GpuWeaverWeights upload/download, pos-emb add, batched SwiGLU, causal MHA with smem softmax, indirect embedding gather, dot-per-row, softmax-over-K with the Issue 468 P0 probs_offset write-shift + kvarn/ — kvarn_zone_weights.rs + kvarn_dequant.rs + kvarn_quant.rs, the game-side CPU reference + zone-scale registry for KVarN KV-cache quantization, post-Issue-042 orchestration over canonical katgpt-kv primitives; Lane B cross-verified every fix-shape against the canonical kv_cache.rs/var_norm.rs): one-thread-per-output-row pins 128 threads to a single workgroup at production + 16× read amplification → split the reduction axis (workgroup per row, lane-strided, block-reduced — the INVERSE regime of grid-over-output-thread-private-sweep), a materialized D*K*h gather whose only consumer re-reads it in a dot → gather in-loop via the device-resident index array, the workgroup-size literal restated in 4 kernels vs 6 launchers is a silent stride contract → one shared const, a global write + re-read nobody outside the kernel observes → smem stash + drop the buffer from the signature, the per-row unpack call re-derives an identical bit address (pos_in_tile invariant) → decompose once per position, materialize-then-transpose = 2 reads + 2 writes per element → scatter the consumer layout at production, a software bit cursor re-derives what a per-width arm makes compile-time-constant → match bits {8/4/2/1} byte-batch arms, two folds scan the group twice for the (lo, hi) pair every RTN scale prep needs → one joint sweep (the canonical var_norm.rs comment documents the identical fix), and HashMap iteration order leaks into serialized bytes breaking BLAKE3 commitment stability → sorted keys; corpus 325 → 334 (A 75, B 49, C 33, D 59, E 19, L 11). 22 falsifications (the file's own documented trade-offs — thread-0 softmax at tiny K, naive-per-head MHA at seq_len ≤ 16 — plus 8 dedup kills + the canonical katgpt-kv positives). Hazards filed as Issue 698 (the legacy KVarN round-trip DROPS the VarN inversion — scales discarded at kvarn_quant.rs L160, masked by vacuous tolerances; the CPU/GPU auto-route doc-lie; the drifted unpack fork corrupting non-pow2 widths; GpuWeaverScratch dead with known-wrong sizing; the dot-per-row perf cliff).
   Batch 45: 3 riir-gpu production-kernel rules (scratch-depth-matches-shape-invariance, softmax-denominator-hoist-out-of-weighted-v, frozen-cooperand-hoist-out-of-perturbation-loop) from Research 050 — the SHINE hypernet meta-LoRA cluster, first half (hypernet/meta_lora.rs 732 — the CPU reference bidirectional forward for memory extraction, Plan 098/104 + hypernet/train_meta_lora.rs 601 — the CPU finite-difference Meta-LoRA trainer): a layer × position × head nest allocates every scratch at the deepest convenient point though all shapes are bound by (seq_len, dim) fixed for the whole call — allocation depth must match shape invariance (the tell: two same-shaped residuals at different depths, attn_residuals per-layer vs mlp_residual per-position; scores alone is L×S×n_head allocations), the softmax-weighted V accumulation divides by the denominator inside the innermost (d × t) nest — scores[t]/sum_exp recomputed head_dim times per key position; hoist inv_sum = 1.0/sum_exp once per (position, head) so the PV loop multiplies never divides (the in-tree optimized twin is riir-engine attention.rs attention_head pass 3 — the file re-implemented it naively one crate away), and an FD/perturbation sweep rebuilds the FROZEN co-operand per candidate while the callee re-derives its own sweep-invariants per eval — materialization cadence must match mutation cadence (build a LossPrelude at sweep entry: frozen mem + context target + pre-converted adapters; complements make-unmake which fixes the VARYING operand); corpus 334 → 337 (A 77, B 50). 9+ falsifications (representation-conversion-clone = the immutable-operand-upload family's CPU twin — mechanism is not decision content; FD unflatten-per-perturbation = make-unmake instance; loop-order swap = ambient matvec-coalesce; per-position GEMV = op-major-loop-inversion + deliberate CPU reference; powi already hoisted; FD O(P) structure = documented trade-off). Hazards filed as Issue 699 (lora_buf sized by model_config.lora_rank vs adapter rank 128 → silent get_unchecked_mut OOB with real configs' lora_rank: 0, masked by a 4==4 test coincidence; GQA kvd < out_dim slice OOB; apply_meta_lora dead-but-public + 16× scaling + same-bytes-transposed A/B layout vs the production path; FD clone storm quantified 1.4 MB/step test scale, ~636 GB/step extrapolated; baseline ctx-8 hardcode + single-sample GOAT estimator).
   Batch 46: 3 riir-gpu production-kernel rules (size-gate-covers-every-cost-factor, declared-max-array-taxed-per-thread-under-runtime-bound, shared-operand-side-pair-batch-index-decomposition) from Research 051 — the MaxSim cluster (maxsim/dispatch.rs 449 + spectralquant.rs 581 + both WGSL kernels + the Benchmark 002 crossover bench; size-gated CPU/GPU MaxSim late-interaction scoring Plan 085 + the fused SpectralQuant dequantize+MaxSim single-pass scorer T5): a CPU/GPU size gate reading a partial work product ld*batch while CPU cost scales with lq*ld*dim*batch silently transposes the crossover at unbenched shapes (the pinned 256 bakes in BOTH implementations' current overheads — two in-tree instances incl. gpu_m2p rows*num_heads), a WGSL function-scope array sized to a hard max array<f32,256> but runtime-bounded by d_eff pays the DECLARED max in indexable thread-local per thread (fix: pipeline override constants, one pipeline per bound class), and a [batch x lq x d_h] pair API bakes per-pair replication of the shared query side into the batch geometry (decompose the launch index (query_idx, doc_idx) — share-by-indexing, not replication; the CPU substrate twin maxsim_score_packed already does it). 14 falsifications incl. the d_eff unpack-bounds hypothesis KILLED by kernel read (unpack bound IS d_eff — tail dims never unpacked). Hazards filed as Issue 700 (validate() never checks compressed_k; the documented-but-unenforced +1-padding one-word OOB read executing in tests; d_eff bounds unchecked anywhere; the FALSE no-katgpt-dep premise hiding katgpt_core::simd::maxsim_score one feature-forward away; the bench's own crossover finder warning against the shipped constant; the 800K-pair doc example ~133 GB infeasible; zero production callers). GOAT: 354 kernel_heal + 201 default, clippy 0 both states (15 doc fixes — the B37 line-initial + RECURRED). Generator NEW lesson: Python evaluation-order truncation — open(p,"w").write(apply(...)) truncates at open-time BEFORE apply() runs (live-catched, restored from HEAD; compute content first, then open).
   Batch 47: 3 riir-gpu production-kernel rules (per-key-projection-hoist-out-of-query-sweep, multi-head-out-proj-consumes-head-concat, shared-producer-prefix-extract-once-per-input) from Research 052 — the SHINE hypernet SECOND half (hypernet/shine.rs 402 + routing.rs 268 + types.rs 269 + m2p_transformer.rs 611 + gpu_m2p.rs 568 + kernels/m2p_attention.wgsl 173 — the end-to-end context-to-LoRA pipeline + expert routing + the M2P alternating-attention CPU reference + its size-gated GPU dispatch twin; completes the hypernet module alongside Batch 45's meta-LoRA pair; meta_lora/train_meta_lora were ACTIVE sibling WIP — quotes drawn only from clean files): K/V projections re-derived inside the query loop multiply projection work by seq (O(seq*H^2) -> O(seq^2*H^2); the optimized twin projects ALL QKV once at sweep entry in the same module family; row attention at default M=425,984 is arithmetically infeasible), the attention out-projection consumes the CONCATENATION of all heads' outputs — a per-head thread degenerates to scalar-times-rowsum (attn_val times the summed wo row) silently, masked by identity-weight fixtures whose rows all sum to 1.0 (two-phase fix: per-head scratch + full-row projection), and two sibling methods sharing an expensive pure producer (extract_memory_states — a full frozen-backbone forward) pay it twice under one composed call (context_to_lora_with_span_width — extract once, thread the artifact via *_from_grid variants); corpus 340 -> 343 (B 51, D 61, E 20). ~17 dedups (workgroup(1,1,1) = B44/B23/B28 composite; per-column weight re-upload = B33/B28/B26/B40; the size gate = B46's pre-recorded 2nd instance; Vec churn = B45 depth; powf freq table = 3rd-strike cos/sin; routing norms = norm-decoupled; shape metadata = derive-shape-from-payload). Hazards filed as Issue 701 (the WRONG out-projection math in both WGSL kernels AND the CPU fallback + the print-only pseudo-parity check comparing the two wrong arms; release-mode stack OOB at default head_dim 512 vs the [f32; 128] array; WGSL array<f32, 256> scores overflow at seq > 256; the ~251 GB default memory grid + the TRIPLE-inconsistent 13/15/19H provision constant with 99.96% of the grid discarded at default; structurally-dead all-zero LoRA modules at toy scale via the min() clamp; row-attention dispatch exceeding the 65535 workgroup limit; zero production callers — bench-only). GOAT: 357 kernel_heal + 201 default, clippy 0 both states (10 doc fixes: the B37 line-initial + struck a 4TH time + 7 pre-existing KERNEL_CATEGORIES slice-header backticks from the B46 split — whole-crate clippy after any corpus-layout change is the standing check). The 2-lane split (compute pair vs pipeline/routing) resolved cleanly; both lanes' quotes re-verified by the coordinator in clean files.
   Batch 49: 1 riir-gpu production-kernel rule (runtime-arg-loop-bound-demotes-arrays-to-local-memory) from Research 055 — the Bench-686 measured-lesson distillation (NOT file mining: riir-ai Issue 706 / Bench 686, the rmsnorm_quantize_f32 single-pass + register-unroll fix on the 4090 cudarc decode path): a correctly-sized per-thread array whose per-element loops are bounded by a RUNTIME kernel arg (constant at every call site — a LAUNCHER fact the kernel cannot see; the host passes a Rust const as an int arg) is demoted to local memory — the compiler cannot unroll a runtime-bounded loop and a dynamically-indexed per-thread array has no register form, so every access is a local-memory (spill) round-trip and every array the loop touches inherits the demotion; fix = hoist the bound into the kernel as a compile-time constant + #pragma unroll (statically-indexed arrays promote to registers; the ascending accumulation order is preserved so outputs stay bit-identical; keep the (i < len) tail-guard for partial tiles); the pathology is INVISIBLE by default — it hides under existing global traffic and surfaces only when a neighboring change adds same-address-space traffic: the naive v1 (gamma prefetch only) measured 0.6-1% SLOWER in paired interleaved A/B (trading L1-hot global reads for more local-memory traffic is a LOSS); evidence: kernel −18.4% (nsys 5,460.9 → 4,455.7 ns avg, 20,702 launches, same-code-drift controlled), ppl bit-exact 5.4681/3.7778/0.6909; distinct from B46 declared-max-array-taxed-per-thread-under-runtime-bound on three axes (SIZE-declared-max footprint tax vs BOUND-driven per-access local addressing; oversized vs exactly-sized array; pipeline-override size specialization vs const-bound hoist + unroll); corpus 343 → 344 (A 80). GOAT: 574 kernel_heal + 415 default + 9 retrieval-gate tests (floors held), clippy 0 both states, all-but-ternary combo check passes — the full CI guard layers 3/4/5 on the 4090 box are blocked by the PRE-EXISTING un-gated metal dep in riir-gpu (diagnosed via cargo tree -i core-foundation; filed as riir-ai Issue 707 with the one-line fix; RESOLVED 2026-08-16 - M3 T2 verified metal_tensor_gemm + bench_663 interop green, riir-ai c15cb947d). 4090 toolchain note: the repo pin resolves to the GNU host whose mingw linker is absent — every build on this box needs cargo +1.97.0-x86_64-pc-windows-msvc.
   Batch 50: 0 rules — the file-mining ZERO-RULE batch (Research 056; 8 files across 2 lanes: small leaves game_mux/domain_latent/context/cpu_reference/depth_tier/elf + standalone twins forward_dflash/forward_flashprefill + 5 WGSL kernels verified). Every candidate was a positive instance (context.rs IS B21 construction-side, domain_latent.rs IS B33, elf's sigmoid IS B32a, flashprefill mean_k IS B35-applied) or an ANTI-instance (forward_dflash's stale uniforms are B26's live ref_shape — the SAME bug class Batch 27 filed in backward_dflash, forward twin; the two-pass softmax is popcorn online-softmax-streaming's ref_shape; the one-thread-per-row reductions are B44's) — 28 total drop-table rows across both lanes. Corpus stays 344. The batch's VALUE is the hazard yield: 31 hazards filed as riir-ai Issues 708 (forward_dflash is an UNFINISHED MVP presenting as a forward pass — two no-op stubs (rmsnorm L1027-1047 + attention L1212-1225, comments admit) + two live stale-uniform wrong-math classes (uniform_qkv.pos = seq_len-1 for ALL Phase-A dispatches via per-call queue.write_buffer + one shared uniform_matmul across wo/w1/w2/lm_head) + tests asserting only nonzero+finite on garbage logits, feature-gated dflash_training opt-in) + 709 (game_mux demux math UNJUSTIFIED — softmax after the weighted sum + non-negative hash distributions → h_i·h_j > 0 cross-contamination, with a live riir-games consumer + is_finite()-only tests; cpu_reference f64 doc-lie (all f32 accumulators) + lora_forward doc omits lora_b.wgsl's BASE GEMV (parity trap); elf rms_norm is L2-normalize mislabeled (sqrt(d) scale factor, collides with cpu_reference's true RMSNorm in-crate) + elf embedding-SDAR is a silent zero-loss stub at its riir-train consumer). Coordinator verified all 9 load-bearing claims against source before filing. Recommendation encoded in the queue: the kernel file-mining program approaches its natural end — next (final) file batch = kimi_k3_gpu_backward_batched + attention_q8kv_cubecl (expect 0-2 rules), then declare the riir-gpu corpus COMPLETE and shift to rust_perf + issue-driven work.
   Batch 51: 3 riir-gpu production-kernel rules (batched-read-slot-membership-decided-at-launch D, unpack-loop-indexed-by-container-word A, dequant-placement-by-component-traffic-share N) from Research 057 — the FINAL riir-gpu file-mining batch (kimi_k3_gpu_backward_batched.rs 1,398 + attention_q8kv_cubecl.rs 967; corpus 344 → 347, A 81 / D 62 / N 19; the riir-gpu corpus is now COMPLETE — ~115 files mined, the remainder is the sibling-blocked ternary_deltanet cluster + tests + thin leaves judged out-of-scope at B50). Rule 1: a batched readback whose push site guards on a local Option PAIR while the extraction site guards on the config flag that merely IMPLIES it drifts when flag=true + handle absent (constructible through the real builder) — every slot shifts by one + guaranteed OOB at the tail; a READ list has NO persistent state to derive from (unlike B40's write-side derive-from-cache), so the fix is decide-once-at-launch or a recorded tag vec (the file's own dead sync2_layout is the unwired fix; its t6_meta is the wired positive). Rule 2: a byte-packed unpack loop indexed by the LOGICAL element re-loads the same u32 4× + runtime div/mod + runtime shift per element — index by the CONTAINER word (one load serves 4, shifts become immediates); honest scope: L1-hit loads, win = L1 bandwidth + issue slots (distinct from B44 bit-cursor width-dispatch + B44 invariant-hoist + popcorn pack-side cousin). Rule 3: a quantized persistent operand splits dequant placement PER COMPONENT by traffic share — VALUES stay packed + dequant in registers (4:1 bytes × every-step reads; within-workgroup reuse 1 so B14's tile-staging doesn't apply), SCALES pre-decode f16→f32 at upload (sidecar is 1/16 the traffic); encodes the tension with B31 (CORRECT for the sidecar, WRONG for the payload — normalizing the whole operand at upload is the naive shape). Coordinator verification REFUTED Lane B's orphaned-module CRITICAL (the module IS wired: lib.rs L895-898 cfg cubecl_runtime + 3 production launch sites in gemma2_cubecl/dispatch.rs) — the subagent's repo-wide grep silently failed on the multi-root workspace; every CRITICAL reachability claim gets a coordinator grep now (Research 057 lesson 1). Issue 693 closeout verified: H1/H2 fixes PRESENT in both twins + the LCG generator fixed at all 3 sites + the stale B40 corpus note refreshed (entries_gpu_05.rs). Bug yield filed as riir-ai Issue 710 (15 hazards: the sync2 guard drift w/ OOB panic path, the 9-site MLA CPU weight-grad backlog capping Issue 586's conversion, the t-invariant k_c recompute, the production-wired q8kv multi-tile path with ZERO test coverage at n_positions > 256, dual n_positions source of truth, + 10 more). GOAT: 577 kernel_heal + 415 default + 9 retrieval tests (kernel 100/100/100 self-variants, blended 77.8/87.3/92.9 — ALL above the Issue 022 floors), clippy 0 both states (2 doc-markdown backtick fixes — the .rs-filenames-in-//! gotcha, 5th recurrence), all-but-ternary combo clean. Generator lessons: category-membership anchor verified by ROW INSPECTION not rule-text memory (rademacher-signs lives in B, not N — first pass mis-filed dequant-placement into B; caught by row-span check), concepts-covered case-sensitivity struck a 4th time (NO vs no). The idle-loop mining queue shifts to rust_perf + issue-driven work; kernel_opt stays open for measured-GOAT distillations (the B49 path) + the ternary_deltanet sibling-WIP unlock.
   Batch 52: 1 rule (phase-split-per-position-data-liveness-row-storage, D) from Research 059 - the first measured-GOAT distillation AFTER corpus-complete (the B49 reopen path (c), same-day): distilled from the riir-ai Issue 708 H4 fix where the dflash forward's Phase-A project_q wrote a single-row [n_embd] q buffer per position so Phase-B bidirectional attention could only ever see the LAST position's Q - invisible while the consumer was a no-op stub, and NOT a stale-uniform bug (the right value was NEVER STORED; distinct from B26/B27 wrong-VALUES). Fix shape: per-position row storage ([seq_len x dim] q_cache + row-pos writes riding the existing pos-in-uniform + backward reading row pos). The fix itself landed as riir-ai `42b564759` (dflash forward FUNCTIONAL: per-pos uniform slots H1, per-site matmul uniforms H2, real gamma-less rmsnorm H3, real bidirectional concat attention H4 + q_cache/attn_out_cache, H6/H7/H8/H11 dead-work deletions, 3 backward tests un-ignored; G1 CPU-reference parity max_rel=5e-6, 565/565 feature tests). Corpus 347 -> 348 (A 81 / D 63 / N 19). GOAT: 600 kernel_heal + 135 corpus tests (3 new batch52 gates), clippy 0 both states (2 doc_markdown fixes - the [x y]-bracket family needs backticks in //! prose), retrieval floors HELD (kernel 100/100/100, blended 95.2/97.6/97.6). Landing lesson: multi-line doc-bullet anchors must match the FULL bullet, not its first line (the B52 header initially split B51's bullet - the same interleaving quirk B49/B51 already exhibit in that header, pre-existing).
   Batch 54: 1 rule (regularizer-above-noise-floor-coupled-to-damping-cap, B) from Research 061 - the SECOND measured-GOAT distillation after corpus-complete and the FIRST from riir-train (the Plan 339 T5.1 handoff's strongest candidate, pre-verdicted so refuted ones wouldn't be mined): a reduced-precision matrix-polynomial iteration (NS / inverse-PSD-sqrt over a TF32 Gram) on rank-deficient input gets spurious null-space eigenvalues from SIGNED per-GEMM format noise (~1e-3 TF32), not from the eps regulariser (cond~1e12 decay is CLEAN - exact/near rank deficiency with O(1) columns is the trigger). Three coupled facts, none sufficient alone (Bench 470 decision table, 26 adversarial cases): (1) eps must sit ABOVE the format's noise floor - eps=1e-5 is 100x below TF32 GEMM noise, "a regulariser quieter than the thing it regularises"; (2) eps is capped by the damping budget (normalisation makes lambda_max = 1+eps; gamma budgets exactly gamma-1 of overshoot past the polynomial's basin [0,1] - the coupling eps <= gamma-1 must be ASSERTED in code, 10x the budget diverges to sigma_max ~1e24 on the CONTROL); (3) raising eps alone shrinks singular values at high d_min (0.36) - the repair is a second FULL-coefficient-schedule pass with the Gram re-formed from the DATA iterate (after pass 1 S ~= I so eps is no longer large vs the spectrum; rank-1 lambda_min/lambda_max -1.9e-1 -> -1.1e-9). Measured: shipped 16/26; restart alone 14/26 (WORSE - a fresh Gram gets fresh signed noise); eps=1e-3 alone 26/26 but convergence 0.3639; eps=1e-3 + 2 full passes 26/26 AND production 0.9587 > 0.9454. Two refuted variants encoded INSIDE the rule as decision content: Gram-domain restart P = X*P0*X (algebraically identical, no extra tall GEMMs - but 4 digits of cancellation in TF32's 3; broke the control 16/26 -> 6/26) + partial-schedule restart (re-applies the aggressive a=7.42 prefix to an already-orthogonal spectrum -> fixed point sigma ~= 1.7 that more passes don't improve). Guard corollary: `t < 1e-20` is FALSE for NaN - an overflowed Gram sails through and poisons the weights permanently; write `!t.is_finite() || t < threshold` (landed unconditionally in riir-train). Corpus 348 -> 349 (B 51 -> 52). GOAT: 735 kernel_heal + 549 default lib, clippy 0 both states, retrieval floors HELD (clippy top-1 89.6 vs 87.5 floor; 13 retrieval tests green). Also fixed the B49/B51/B52 header interleave in corpus/mod.rs (the orphaned B49 content rejoined to its stub - span-marker replacement, the recommended method for that header).

   Batch 55: 2 rules from Research 062 - the THIRD measured-GOAT distillation after corpus-complete, from riir-ai Issue 716 / Bench 691 (q8kv sink guard, landed `e794b0033` the same day) + the Bench 642 / Issue 639 re-hit: (1) `outlier-span-escapes-quantization-via-lossless-sidecar` (N 19 -> 20) - 1-2 massive-activation channels (x100/x300 on sink pos 0 + delimiter) collapse non-outlier NEIGHBORS in the same 32-block 58x (attention output error 584x/18,844x over uniform, concentrated at the poisoned blocks 88x/299x); the rows are KNOWN A PRIORI (attention sinks = oldest fixed positions, hybrid schedule boundaries from config - no detection), so the known span escapes quantization into a LOSSLESS f32 sidecar ([keys || values]) while the q8 buffers keep the rows as DEAD DATA so strides never shift; kernel derives row count from the sidecar allocation length / stride; A/B: sidecar BEATS Hadamard rotation on BOTH axes (rotation spreads MA to absmax ~MA/16, neighbors regain only ~8x resolution - nonzero - at strictly larger kernel cost; sidecar exact on the guard span 4.781 -> 0.000698 = 6,850x, full sidecar == f32 kernel at 0.00000000) at 0.96x perf (f32 reads replace byte-dequant loops); S=0 bit-identical to the unguarded launch. (2) `buffer-length-derived-from-allocation-not-binding` (D 63 -> 64) - a TWO-OCCURRENCE root-caused class: cubecl's `KernelLauncher::process_buffer` derives the kernel-visible length from `handle.size_in_used()` (the ALLOCATION), NOT the bound `BufferArg` length (validation-only in the checked path; `launch_unchecked` never sees it); over-length half (Bench 642: `let dim = input.len()` made rmsnorm reduce x^2 over FOLLOWING rows - fix = both-ends binding trims `offset_start(start).offset_end(tail)`, filed class-wide as Issue 639) + zero-length-over-a-real-handle half (Bench 691: `from_raw_parts(query_handle.clone(), 0)` derived sink_rows=1 from the 2048-f32 allocation, silently reading the QUERY buffer as position-0 K/V, uniform error 0.000268 -> 0.12 - fix = a REAL 4-byte allocation that floor-divides to the semantic zero, 1 f32 -> 1/2/1024 = 0; contract is EXACT-sized, oversizing CHANGES semantics); probe corollary: a benign-case-only probe (the LAST row where remaining allocation coincidentally equals dim) is WORSE than no probe - it converted an open question into a wrong answer. Corpus 349 -> 351 (N 19 -> 20, D 63 -> 64). GOAT: 738 kernel_heal + 549 default lib + 163 corpus tests, clippy 0 both states (2 doc_markdown backtick fixes - `handle.size_in_used()` + `BufferArg` in `//!` header prose), retrieval floors HELD with ZERO decay (kernel self-variants 100/100/100, blended 94.6/97.7/97.7 of 130 - identical to the Batch 53 re-pin). Record: Research 062.


The pipeline (shared across all domains):
1. **L0 — Rule distillation**: each rule → named entry + bounded fix-space
2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec, zero-alloc (Issue 001, DONE)
3. **L2 — Validator**: `syn`-based syntax parse check (always-on, modelless — Issue 002, DONE).
   Domain-specific verification is opt-in:
   - clippy: `cargo clippy` re-run (`clippy_verify`, Issue 003, DONE)
   - rust_perf: `cargo test` re-run (`rust_perf_verify`, Plan 001 Phase 2, DONE)
   - kernel_opt: CubeCL compile (`kernel_verify`, Plan 001 Phase 3, STUB —
     full numeric compare deferred per Proposal 012 §Phase 0)
4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
5. **L4 — LoRA fallback** (deferred): adaptive learning, last resort

**Plan 005 T3.5 (2026-08-16) — the G1 recovery contingency (Bench 027).**
Root-caused the clippy crowding to the BM25 tokenizer: the substrate default
`CodeTokenizer` discards ALL punctuation, but the clippy corpus is 40 minimal
generic functions whose entire discriminative signal IS punctuation
(`identity_op` = `* 1`, `unused_unit` = `-> ()`, `deref_addrof` = `&*`) — so
every rule collapsed to the same `fn f x` bag and the most generic one
(`identity_op`) topped 7 of 22 misses. Perf/kernel never had the problem
because they are identifier-rich. Fix: `src/draft/op_tokenizer.rs`
(`RustOpTokenizer` / `RustOpBigramTokenizer`) — identifier tokens
byte-identical to `CodeTokenizer` plus one token per maximal punctuation run
plus operator-adjacency bigrams, injected via the substrate's own
`with_bm25_tokenizer` hook; BM25's IDF decides what discriminates, no tuned
weights. **clippy top-1 54.2% → 70.8%, perf 87.5% → 97.9%, blended
70.8% → 84.4%; bit-identical across 3 runs.** Floors rebased (blended
80/89/90 of 96, clippy 33/42 of 48, perf top-3 46/48). Two REFUTATIONS
recorded: the latent fusion weight is inert (identical top-1 at 1.0/0.5/
0.25/0.0 — BM25 dominates), and centroid re-fitting cannot move G1 at all
(retrieval is fan-out, the router is annotation-only). **The ≥80% G1 bar is
still 9.2pp away — Phase 3 stays open on T3.5.4** (a top-k re-ranker: 9 of
14 top-1 misses have the right rule at rank 2-3, and top-3 is 89.6%).
Sweep: `cargo test --features latent_retrieval --lib retrieval_sweep_report
-- --nocapture`.

**Plan 005 T3.5.4 (2026-08-16) — instantiation re-ranker: built, benchmarked,
NOT promoted (Bench 027).** `src/draft/instantiation_rerank.rs` re-ranks the
retrieved top-8 by running each candidate rule's `propose_fix` visitor over the
span (one parse, K visitor walks) — rule *instantiation* instead of text
similarity, the distinction Bench 018's threshold refutation named as the
ceiling. On paper it clears the G1 bar: clippy top-1 70.8% → 87.5%
(`Structural`) → 95.8% (`All`), blended 84.4% → 96.9%. **Those deltas are
CONTAMINATED and must not be quoted as a gain**:
`clippy_fixture_labels_verify_via_proposers` asserts every clippy fixture fires
its OWN proposer and zero of the other 40 cross-fire — exactly the property the
re-ranker exploits. `rerank_circularity_diagnostic` proves it to the digit
(`All` top-1 = pool recall @8 = 95.8% = 46/48), so the table measures "was the
right rule retrieved", not "did re-ranking pick it". Per the GOAT rule the gate
did NOT pass → **`RerankMode::default()` stays `Off`, asserted by a test**.
Established non-circularly: safety (`clean_spans_fire_no_lint` — structural
proposers don't fire on 12 real production excerpts), coherence (`--fix`
already gates on the same visitor), zero cost when `Off`. **Unblocking it needs
an oracle independent of our proposers** (real `cargo clippy` labels — the
Bench 024 G7 harness already exists) plus spans where several lints genuinely
co-apply. Tracked as Plan 005 T3.5.5.

**Plan 005 T3.5.5 (2026-08-16) — uncontaminated oracle; re-ranker PROMOTED;
PHASE 3 CLOSED (Bench 027).** `src/draft/clippy_oracle.rs` asks real
`cargo clippy` (all three lint groups, JSON, compile-failure detection) what
fires on a span — an oracle with no connection to our corpus or proposers.
**It overturned half of the T3.5.4 diagnosis:** the labels were never wrong
(real clippy confirms 43/43 of the compiling fixtures); the defect is SAMPLE
SELECTION — 0/43 fixtures trip more than one corpus lint, so exactly one rule
can be right and a ranker has nothing to decide. Re-running G1 against real
labels reproduced the degeneracy exactly. Fixed by authoring the missing
sample: `src/draft/eval_fixtures_composite.rs`, 28 multi-anti-pattern spans
**labeled by the oracle, not by hand** (27 compile, 25 genuinely multi-lint;
a second batch avoids the first's `needless_return` pairing bias).
**Uncontaminated G1: off 84.0% → Structural 96.0% (+12.0pp)**, with the
degeneracy check now failing to fire (`all` 96.0% != pool recall 100.0%).
GOAT: G1 PASS, G2 +52-64us/span (+5.6-7.2%), G3 415/461/481/471 lib tests
across four feature states + clippy 0, G4 one `syn::File` per span, modelless
throughout → **`RerankMode::Structural` promoted to default**, pinned by
assertion. Committed eval after promotion: clippy 87.5/93.8/93.8, perf
97.9/97.9/97.9, blended 92.7/95.8/95.8; floors rebased (blended 88/91/91 of
96, clippy 41/44 of 48). **Phase 3's >=80% G1 bar is MET on both the committed
eval (87.5%) and the honest composite set (96.0%) — Phase 3 CLOSES.** Two
caveats carried forward: the committed clippy rows are a regression floor, not
an effect size; and 5 of the 48 original fixtures do not compile, so no
independent oracle can validate them. Run:
`cargo test --release --features latent_retrieval,clippy_verify --test uncontaminated_g1 -- --nocapture --test-threads=1`.

**Issue 024 (2026-08-16, RESOLVED + removed per noise-reduction rule; commit `e34ade9`) —
cross-domain switch-cost measurement + `RERANK_POOL` 8→12.** Research 484's
switch-cost question asked of our healer: 12 directed clippy×perf pairs × 2
orders, paired fresh-baseline (`src/draft/switch_cost.rs` +
`eval_fixtures_cross_domain.rs`; real-clippy truth via
`tests/cross_domain_oracle_truth.rs` — 24/24 merged spans compile + carry
exactly the one labeled lint). At pool 8: perf→clippy **+8.3pp** (exactly one
pair, `needless_return`+`vec-with-capacity`, BOTH orders), clippy→perf 0.0pp.
Mechanism probed = **retrieval pool crowding, NOT fix interference**: the
correct rule crowded to raw similarity rank 9 by loop-shaped rules scoring 3×
its solo winner — one position past the re-rank pool, evicted before the
instantiation re-ranker ever saw it. Mitigation: `RERANK_POOL` 8→12 (deeper
pools add visitor walks, never new promotions). Post-mitigation **0.0pp both
directions, zero drops**; the Issue 017 eval IMPROVED (clippy 87.5/93.8 →
89.6/95.8, blended 92.7→93.8 — other latent near-misses rescued too; floors
re-pinned to blended 89/92/92, clippy 42/45/45); T3.5.5 composite G1 unchanged
at 96.0% (+12.0pp); rerank latency +39.6µs/span (+7.1%, same envelope);
`pool_depth_rescue_guard` pins the rescue. **Verdict: dense fan-out is
switching-cost-free at the corrected pool depth; the fix-ordering
SwitchCostTable consumer (the issue's anticipated gap branch) is REFUTED by
mechanism — ordering fixes targets edit interference, the gap was pool
crowding.** T3 (fix-axis) honestly skipped — the measured gap lives entirely
on the retrieval axis and the cross-fire label test already pins disjoint
Record: [Bench 032](.benchmarks/032_cross_domain_switch_cost.md)
+ the Research 484 healer-consumer row (katgpt-rs) resolves CLOSED.

**Issue 025 (2026-08-16, RESOLVED + removed per noise-reduction rule; commits
`589b0c9` + the `wgsl_mine` required-features fix in `bedf795`) — non-syn
surface scanner (`.wgsl` + `#[cube]` bodies).** T1–T6 all shipped: the
WGSL/cube scanner (`wgsl_scan.rs` — regex-pattern surface scanning with
precision guards discovered by live adjudication), the T5 control gate
(`tests/wgsl_t5_control.rs`), and the T6 mining integration
(`examples/wgsl_mine.rs` — git-history differential scan over both surfaces,
finding-level pairing, MOVED auto-refutation, Removed/Hunk diff association).
Two new patterns mined from the fix pairs: `scan_single_slot_staging` (DEFAULT,
fronts B35 — 2 live findings / 0 FPs across 33 `var<workgroup>` files; the
computed-write role_transport, tree reductions, transpose compounds, and
domino multi-phase are all guarded silent) + `scan_serial_row_dot` (P7
experimental, mining-only — 11 live serial-K GEMV instances textually
unadjudicable). Pair inventory 1 → **3 validated** (B47×Issue 701 H-C,
B35×Issue 686 H7, B44-body×Issue 698 A-H1); Phase 5 reopen trigger honestly
open at 3/5. Mechanized survey finding: ZERO `.wgsl` commits ever removed a
default-pattern shape (Bench 033's caveat now a proven fact); the genuine fix
pairs live on `#[cube]` bodies + other shapes. Live default-set 30 → 32
(gram.wgsl dead-file carriers, adjudicated real). Gates: 419 default / 602
kernel_heal / 727 all-features + t5_control 4/4 + mine_pairs 6/6 + Issue 017
retrieval floors PASS (corpus unchanged). Record:
[Bench 033](.benchmarks/033_non_syn_scanner_goat.md) +
[Bench 034](.benchmarks/034_wgsl_mining_integration.md). Remaining: the
Phase-5 reopen trigger (future fix commits enter the harness automatically,
one command per surface); the dead-`.wgsl` cleanup is DONE (riir-ai
`738dc2a66`, 2026-08-16 — 6 files / 211 lines removed, `wgsl_mine_pairs`
re-run 10/10 green); inline-CUDA surface as scope extension.

**Issue 026 (2026-08-17, RESOLVED-NEGATIVE + removed per noise-reduction rule;
commit `bedf795`) — entropic (max-seeking) selection mode (TTT-Discover
arXiv:2601.16175 transferred to heal-loop selection).** Shipped
`src/selection_entropic.rs` (KL-budgeted Boltzmann tilt: `solve_beta`
streaming zero-alloc bisection, `tilted_weights`, `tilt_advantages_into`; 9
unit tests incl. shift/scale invariance + exact KL budget + degenerate pools +
rare-success upweighting + auto-adaptation) + `SelectionMode::Entropic` opt-in
wired at `heal()` Step 4. **T2 wiring-site honesty:** the issue's proposed
sites were file-guesses — `AdaptiveDrafter` is an exact-key KV cache,
`RuliologySearch` is a structural Pareto ranking; neither has an outcome axis;
the ONLY outcome-scored pool is `heal()` Step 4 (loud degradation everywhere
else). **T3/T4 measured NEGATIVE (Bench 035): ALL FOUR reward-source ×
mapping variants (sigmoid(A)/w-max-norm × belief/raw-rate) lose the max
metric to BetaPosterior (0.9240/0.9379 vs best entropic 0.9132/0.9226) AND to
LinearBlend, across both evo-noise regimes.** Mechanism: heal-loop pools are
starved (0–3 observations) — the belief ranking the tilt consumes is mostly
thin-evidence noise; amplifying it drowns the correlated evo axis
(BetaPosterior's moderate swing leaves room for evo to rescue good
candidates). The paper's regime (dense reward batches, 512 rollouts/step)
does not have starved pools — same failure class as the `gemma2_directions`
GOAT (real signal, wrong regime). NOT promoted; variant + harness kept as the
reproducible negative-result artifact. B=10 saturated (~0.99 every mode);
B=3 discriminates. Also fixed in passing: the `wgsl_mine` example's missing
`required-features = ["kernel_heal"]` (latent `--all-targets` break from
`589b0c9`). Gates: 428 default lib / 448 full suite / 663
kernel_heal+latent_retrieval (floors HELD) / clippy 0 `--all-targets` / t93
unchanged. Record: [Bench 035](.benchmarks/035_entropic_selection_negative.md).

**Issue 027 (2026-08-17, RESOLVED same day + removed per noise-reduction
rule) — `--fix --write` false-positive cluster: single_match_else /
collapsible_if proposers fired where real clippy is silent.** Found live
during the queue-3 clippy sweep on katgpt-rs: `cargo heal --fix --write` on
`cluster_build.rs` applied 4 edits — 3 outright false positives + 1
pedantic-legit match_bool with mangled formatting (all reverted pre-commit;
katgpt-rs landed only the manual sweep fix `380aa9fc`). Root causes pinned
by an empirical probe matrix against real clippy 1.93 (minimal probe crate,
default + pedantic): **(1) single_match_else** — real clippy fires ONLY when
the wildcard arm is a statement BLOCK (expression wildcards `_ => 0` are
silent even under pedantic; a literal first arm fires under a DIFFERENT fix
shape — equality `if x == 0`, not `if let 0 = x`); the proposer fired on any
non-unit wildcard + any non-wildcard first pattern, and its own unit test
encoded the false-positive shape. Guards added: block-wildcard arm +
non-literal first arm. **(2) collapsible_if** — real clippy skips when a
comment sits between the outer block's open brace and the inner `if` (the
merge would displace it); the proposer now scans the lead + trail gap
slices textually (syn has no comment nodes). **(3) match_bool formatting**
DEFERRED `[-]` — span-preserving edits keep original text verbatim by
design; mitigation is rustfmt-after-write. The pedantic-lints-in-default-
runs policy is BY DESIGN (corpus includes pedantic lints; G6 truthfulness
runs all groups). NOT bugs: the needless_range_loop miss (indexing inside
`assert!` — syn cannot see into macro token streams; conservative skip is
correct, real clippy only fires on post-expansion HIR) + span coverage
(proposers scan whole files; the displayed spans are retrieval-only).
Memory cleanup: the 4 polluted Heuristic-tier trajectories removed
(memory.json restored to pre-incident state). Gates: 433 default lib / 484
latent_retrieval lib (retrieval floors + composite eval + corpus truthfulness
PASS) / 453 full default suite / clippy 0 both states; end-to-end re-verified
— the healer now proposes exactly 1 edit on the same file (the
pedantic-legit match_bool), zero false positives. The probe matrix extends
the `clean_spans_fire_no_lint` sample with the four rejected shapes.

**Issue 028 (2026-08-17, RESOLVED same day + removed per noise-reduction
rule) — the `collapsible_else_if` comment-guard gap (Issue 027 class in the
sibling proposer).** Found during the katgpt-speculative heal sweep (landed
katgpt-rs `e8cefad44`): 8 proposed `collapsible_else_if` edits where real
clippy (`--all-features`, pedantic) had ZERO findings — every site has a
comment between `else {` and the inner `if`, the exact Issue 027
`collapsible_if` shape the sibling proposer never got the guard for. Fix:
the lead + trail gap-slice comment scan ported verbatim (syn has no comment
nodes — the raw source slices between `{`/`if`/`}` are scanned; a broken
span range refuses rather than displaces). Root-cause note on the issue's
"match-arm guard" spec: the proposer structurally only visits `ExprIf`
else-branches — it can never fire on a match arm DIRECTLY; the observed
`and_or_builder.rs:228` comment deletion happened through composition (a
`match_bool` edit turns `false => { // c \n if let .. }` into an else-block
in the fixpoint pass, where the comment-blind proposer then collapsed
through the comment) — the comment guard blocks exactly that path, pinned
by the `match_bool_composed_arm_comment_blocks` fixture. A would-be label
guard on the else ExprBlock was verified UNREACHABLE (syn's else grammar
accepts only `if` or `{` — labeled blocks cannot be else branches) and
dropped, not shipped dead. Gates: 17 collapsible tests (6 new fixtures)
/ 477 default lib / `clean_spans_fire_no_lint` extended with the real
`find_decomposition_regions` shape / retrieval floors unchanged / clippy 0
in default + latent_retrieval states / the katgpt-speculative dry `--fix`
re-run: **0 edits proposed** (was 44 — 36 landed last session, the 8
comment-guarded sites now correctly silent).

**Issue 029 (2026-08-17, RESOLVED-PARTIAL same day + removed per
noise-reduction rule) — the healer MISS side of the sweep truth-diff
(the inverse of 027/028: real clippy fires, the healer stays silent).**
Two classes FIXED + one live hazard closed, three documented open:
**(T1) `range_plus_one` paren ends** — `0..(depth + 1)` parses the end as
`Expr::Paren` wrapping the Binary, rejected by the Binary-only match;
paren layers are now unwrapped first (weaver.rs:2642 class).
**(T2) `single_match_else` equality form** — int-literal first arms
(non-negative `Pat::Lit` holding `Lit::Int`) + block wildcard now fire as
`if scrut == lit { } else { }` (the Issue 027 probe P1 finding made
implementable); scrutinee parenthesized unless postfix-safe
(`helpers::is_postfix_safe` reused); bool/char/str literal arms stay
skipped (unprobed). **The live hazard: whole-match splices drop interior
comments** — `single_match_else` now refuses any match whose comment sits
outside kept fragments (scrutinee/pattern/arm bodies) via the new
`helpers::drops_interior_comment` (kept-range check; broken span
bookkeeping refuses). caddtree_budget.rs:232 was the guard's first
production test — `// First observation: use directly` survives verbatim
inside the kept arm body while the match still fires. Both fixes
LANDED end-to-end (katgpt-rs `c9c626120`): ground-truth confirmed vs
real clippy 1.97 pedantic `--all-features --all-targets` (single_match_else
33→31, range_plus_one 2→1), 1092/1093 lib tests (the 1 failure —
`peira_pruner::hot_path_overhead_vs_baseline` — PRE-EXISTING at HEAD
`e8cefad44`, verified via detached worktree; timing-sensitive debug-mode
test, unrelated). **Deliberate divergence documented:** decomp_reviewer:62
fires real clippy but stays healer-silent (interior comment between arms
would be displaced — our tool is SAFER than clippy there). **T3 — DONE
same day** (riir-clippy `ea76b3c`/`3983c15` + katgpt-rs `2215cac06`): the
`None`/`Err(_)` second-arm class, closed by a 20-probe matrix vs real
clippy 1.97 pedantic. **Probe verdict:** on std Option/Result scrutinees
clippy treats a NO-BINDING constructor arm as the wildcard — fires on
`None`, `Option::None`, `Err(_)`, `Err(..)`, `Some(_)`, `Ok(_)` as the
second arm (incl. reversed pairs `None`+`Some(_)`, `Err(_)`+`Ok(_)`);
SILENT on binding arm2s (`Err(e)`/`Some(v)`), user-enum arms
(`R2::Ok(_)`, `E3::None`, glob-imported `None` — clippy is
TYPE-INFORMED), expression arm2 bodies, and `None`-first + binding
second. All firing shapes suggest MachineApplicable `if let P1 = scrut
{..} else {..}`. **Parse-only approximation:** the extended arms fire
only when the FIRST arm is a std-shaped Some/Ok/Err/None pattern (bare
or `Option::`/`Result::`-qualified) pinning the scrutinee type — user
enums stay silent exactly like clippy; the emitted if-let form needs no
trait bounds (a compiling 2-arm exhaustive match proves P2 ≡ ¬P1, so
the rewrite is sound for every shape that compiles). 13 new fixtures;
clean_spans extended with the user-enum + binding-second silent shapes.
**Healed end-to-end:** 18 sites (katgpt-speculative 8 incl. the original
belief_cache/dd_tree misses, src/pruners/bomber 4, benches 4,
katgpt-pruners 2) — ground truth 118 → 96 repo-wide, EXACTLY −22
diagnostics for 18 edits (the 4 bomber sites each counted twice across
lib+test targets); zero remaining findings in healed files; no new
findings introduced. Healer-silences that AGREE with real clippy:
fft/sr2am_player HarnessUpdate (expression arm2 body) + comment-guarded
matches. Empirical syn facts: bare `None` parses as `Pat::Ident` in
syn (the `is_none_pat` three-arm lesson), `Err(..)` elems hold
`Pat::Rest`. Discovered in passing: the `cargo test -p katgpt-speculative
-p katgpt-pruners --lib` dual-crate invocation has a PRE-EXISTING
feature-unification break (katgpt-speculative's dev-dep
`echo_env_predictor` unifies into katgpt-pruners; the un-gated
`test_prediction_scorer_basic` references the `partial_scoring`-gated
`EchoPredictionScorer`) — verified identical at HEAD `4ccb2f280` via
detached worktree; candidate for its own issue. Remaining backlog
(next sweep unit): 96 repo-wide (31 in unswept katgpt-pruners files +
katgpt-forward/dd_tree + katgpt-core). **T4 — DONE same day** (riir-clippy
`337b0fa`/`7554d7b` + katgpt-rs `436cdda10`): the `uninlined_format_args`
miss classes closed by a 30-probe matrix (all `MachineApplicable` vs real
clippy 1.97 pedantic). Real scale measured: **1434 findings / 339 files**
repo-wide (the earlier ~96 estimate was one target subset; tests/benches/
examples dominate). The three extensions: **(1) spec carryover** — `{:?}`
`{:.1}` `{:>12}` `{:#x}` `{:5}` `{:+}` with ident args inline as `{v:?}`
etc. (spec verbatim; same arg, same trait bounds); ABORTS on dynamic
specs referencing/consuming another positional arg — `{:1$}` (digit-$)
and `{:.*}` (dot-star precision-from-next) — the inline deletes the
referenced arg; scope-captured name refs `{:>width$}` + fill chars
`{:*<5}` carry. **(2) macro family** — `panic!`/`format_args!`
(template-first) + `assert!`/`debug_assert!`/`write!`/`writeln!`
(template SECOND) + `assert_eq`-family (template THIRD); top-level
comma-sliced args with leading slices untouched; `unreachable!` stays
OUT (probe-verified clippy-SILENT while `panic!` fires). **(3)
already-named** `{name}`/`{name:spec}` copy verbatim + consume nothing
(mixed templates inline only the positional side). Documented divergences
(clippy fires, we stay silent): `{0}` indices, `{:1$}`/`{:.*}`, named-arg
`$` refs (scan_args aborts on `w = expr`). Coverage on the ground-truth
sample `tests/nf_flow_goat.rs`: 4/34 proposable before → **34/34**, healed
end-to-end 34 → 0 (compiles clean, 7/7 tests pass). Remaining: the
~1400-finding / 339-file sweep (high-coverage now; the next sweep unit) —
**slice 1 landed 2026-08-18** (katgpt-rs `9e9e1f89`): the root `tests/` dir
measured 2348 findings / 165 files (94% uninlined_format_args); 10 files /
554 edits healed with 77/77 tests post-heal, incl. 4 feature-gated binaries
compiled+run WITH their gates (ega_attn 11/11, parallax_attn 5/5,
kimi_k3_backward 5/5, bake_precision+bfcf_tree 10/10). ~1794 findings remain
across ~155 root-tests/ files. **Gate-identification lesson**: 3 of the 4
whole-file gates are `#![cfg(feature)]` inner attrs sitting BELOW the
doc-comment header (lines 11/19) — grep `^#!\[cfg` over the whole header,
not just line 1. Feature-gated files are invisible to default-features
`--verify` (the riir-chain gate-coverage trap), so their gate is a
feature-on test RUN — stronger than check. Slice selection rule: prefer
default-compiled + 0-ignored files (validation = verify + plain test run);
feature-gated files are fine but need their gate identified + exercised.
**slice 2 landed 2026-08-18** (katgpt-rs `7751f771`): 23 default-compiled
files / 556 edits (uninlined_format_args 552, map_unwrap_or 4), net −217
lines. Validation: `--verify` 0 reverts; all 23 binaries count-identical to
pre-heal baseline (113 tests); rustfmt applied + clean; ONE pre-existing
flaky gate documented (`bench_290 g2_motif_mining_overhead_under_5pct` —
debug-timing flap, failed a pre-heal baseline run too, passes on re-run).
**Second gate lesson (slice 2)**: whole-file `#![cfg]` + `required-features`
are NOT the only invisibility class — goat_090_tower_search has a MOD-level
`#[cfg(feature = "unit_distance")]` at line 12 (compiles EMPTY under
default, 0 tests run; the header scan classified it default-compiled).
The complete gate scan = `required-features` in Cargo.toml + `^#!\[cfg` over
the whole header + **mod-level `#[cfg(feature` greps cross-referenced
against the default feature set** — then exercise each non-default gate
(goat_090 @ unit_distance: 10/10; bench_ldt/bench_217 @ 5-feature union:
compile clean + green). ~1238 findings remain across ~132 root-tests/
~1238 findings remain across ~132 root-tests/
files; the gated giants are slice 3: goat_234 (103 @ manifold_pruner),
bench_245 (41 @ still_kv), parallax_sigmoid_stability_grad_clip (29 @
parallax_attn — gate at line 49, below the header); all-`#[ignore]`
bench_block_diagonal/bench_octopus stay skipped (compile-only validation).
**slice 3 landed 2026-08-18** (katgpt-rs `cea73d5d`): the 3 gated giants /
173 edits (all uninlined_format_args), net −87 lines. The healer's
`--verify` is blind to all 3 (default-features check strips them), so each
gate was exercised via a full test RUN pre/post-heal, count-identical:
goat_234 11/0 (debug), bench_245 12/0 (debug; `t22_still_kv_compression`
alone ~17 min — check per-file test runtime BEFORE picking slice members,
a 17-min test doubles validation cost), parallax 3/0 (**RELEASE — its doc
marks release REQUIRED: debug is ~30× slower and the divergence steps
shift, a 4th validation-mode class: mode-locked, not just feature-locked**).
Real clippy @ feature-on: 0 uninlined_format_args remaining on all 3
(remaining per-file warnings = pre-existing out-of-scope pedantic:
cast-precision, doc backticks, single-char names). **Clippy-target lesson:
`cargo clippy --features X --test NAME` lints the LIB dependency too — the
target's total warning count is dominated by `src/` findings (69
uninlined_format_args from the lib appear in a goat_234 target run);
per-file confirmation needs a file-path grep, not the run total.**
~1065 findings remain across ~129 root-tests/ files.
**slice 4 landed 2026-08-18** (katgpt-rs `ccf9fb98`): 48 default-compiled
files / 277 edits applied (268 single-pass sites: uninlined_format_args 224,
match_bool 24, map_unwrap_or 20; +9 bounded-fixpoint compositions), net −27
lines post-rustfmt. Validation: 41 debug + 7 release test runs count-identical
pre/post. **Gate-scan lesson (5th class): mod-level `#[cfg(any/all(feature…))`
around the test mod** — the 3-part scan (required-features + `^#!\[cfg` header
+ mod-level `#[cfg(feature` cross-ref) still missed it because these gates wrap
`mod tests` with `any(` lists of NON-default features; under default features
the binary compiles EMPTY ("0 passed; 0 failed") — 14 such files found
(bench_dflare_modelless dflare*, test_rubric_scalar_collapse ropd_rubric,
test_122_toast_goat toast_tokenizer, stepcode, …), all excluded (compile-only
validation class). **The reliable detector is the baseline run itself: "0
passed" ⇒ exclude.** **Debug-slow class (6th validation nuance): a
default-compiled file can be debug-unrunnable** — funcattn_g6_bug_poc burned
33+ min CPU in debug (doc says `--release`, ~30× slow) vs 390 s green in
release; bench_657/574/246/275/104/176 all baselined in release mode instead.
**Flake rule addition:** bfcf_lsh_cms_goat's baseline failed once under load
(9p/1f; `g5_roaring_batch_speedup` timing flap — the slice-2 bench_290
class), 3/3 green post-heal — a mismatch on a perf-gated test needs a re-run
before it counts as a regression. **Pre-existing failure left unfixed:**
bench_104_mls_k_sweep::bench_mls_k_sweep fails deterministically at HEAD
(K=1 cos 0.8839 vs 0.9 threshold, Plan-104-era tuning, machine-marginal) —
fails IDENTICALLY pre/post; heal validated on same-signature.
**Clippy group note: `uninlined_format_args` is PEDANTIC-group in clippy
1.93** — default `cargo clippy` emits ZERO of them; per-file confirmation
needs `-- -W clippy::uninlined_format_args` (that run: 0 remaining in every
tests/ dir workspace-wide; ~376 remain in crates' src/ — katgpt-core 27,
katgpt-spectral 16, attn-match 6, pruners 5, root 3 — the future src/ sweep).**
~788 findings remain across ~81 root-tests/ files — ALL in feature-gated
files now (~50 gate groups; biggest: bomber-wasm 31, mech_attribution 30,
hla_attention 30, msa_adaptive_k 29, parallax_attn 25); standing exclusions:
14 empty-under-default binaries + 6 all-#[ignore] files.
**slice 5 landed 2026-08-18** (katgpt-rs `a75dc385`): 12 files / 205 edits
across 8 validation cycles, cheapest-feature-first ordering — default
(bench_105_gdn2_goat 5 + bench_core_optimization 33), hla_attention (25,
zero-dep feature — the cheapest possible cycle), ict_branching (20), ega_attn
(19), sink_aware_attn (18), msa_adaptive_k (29), parallax_attn (25),
dynamic_rank,bandit (31, one superset cycle covers both the bandit-only and
the both-features gates). All runs count-identical pre/post; 1 edit
auto-reverted by the compile gate (redundant_closure @ bench_483:506).
**Gate-cycle method (the load-bearing lesson): feature-gated heals MUST pass
`--verify-args "--features F"`** — the default-features cargo check compiles
gated files EMPTY (0 tests), so a plain --verify proves nothing about them;
the same feature set must drive baseline run, verify check, + post-heal run.
**bench_core_optimization validation nuance (7th class): all-#[ignore]
no-assertion benches CAN be cheaply validated** — its 9 tests are pure
measurement ("no assertions" per their ignore reasons) but a release
`--ignored` run completes in ~6 s, giving panic-checking validation where the
all-#[ignore] exclusion class would otherwise defer the file wholesale.
**New divergence class recorded (named-arg-with-rename):** 3 sites in 2 files
(bench_105:416/420, bench_256:496) — `{spread:.3}` + `spread =
gdn2_max_spread` where real clippy inlines the variable into the template
(`{gdn2_max_spread:.3}`); our proposer documents already-named args as
copy-verbatim (Issue 029 T4's conservative rule). Extending the proposer to
rewrite named templates is future work (needs template-string transformation
+ fixtures). **Tooling notes:** `.goat`-suffixed files map to underscored
cargo target names (bench_256_adaptive_k.goat.rs →
bench_256_adaptive_k_goat — check the [[test]] name field, not the
filename); a dry-run's per-file edit attribution MUST be parsed from the
`fix <path> (N edits):` review sections, NOT from suggest-span headers (a
session wasted baselining 36 mis-attributed files before catching it —
the suggest pass interleaves per-file, so edit lines following a span header
belong to the PREVIOUS file's fix section).
**slice 6 landed 2026-08-18** (katgpt-rs `a02b68c1` + the Issue-669 fix commit
`a7bf7c0d`): 32 gated files / 223 edits across 8 feature-gate cycles,
cheapest-feature-first — (1) hardware_aware_scheduler,smear_classifier,
adaptive_gamma_forecast,dllm combined 42 (light/zero-dep features combine
safely in one cycle); (2) kimi_k3_backward SUPERSET 46 — it implies
kimi_k3_loader, so one cycle covered both gate populations (5 files);
(3) elf_sde,bandit 24; (4) dense_mesh 23; (5) mux_latent_wire 16;
(6) tiled_attention,kog_cpu_fusion,toast_tokenizer 21; (7) slod,engram,
event_log,vocab_channel_pruner,coexplain_riir 26; (8) the bandit-family
stragglers (committee_boost,delta_mem,partial_scoring,proof_sketch_evolution,
bfcf_tree,and_or_dtree) 25. All runs count-identical pre/post; 0 compile-gate
reverts; per-file real-clippy grep clean on all 32.
**Bug found at baseline (Issue 669, fixed same day):**
bench_delta_mem_modelless::test_phase1_state_interference ABORTED pre-heal —
the test passed 3-element value vectors to a single feature_dim=5 hasher
(production uses TWO hashers: key dim=feature_dim, val dim=3 per
OutcomeFeatures), so simd_dot_f32 read 2 floats past the features slice:
SIGABRT in debug, SILENT UB in release (stack garbage folded into every
hash_value). The abort killed the whole 16-test binary; the feature sits in
no pinned CI suite (the gate-coverage lesson again). Fix: pad the test
vectors + add `assert_eq!(features.len(), self.feature_dim)` in
project_into (mirroring the existing out.len() assert — the guard belongs at
the typed boundary where the intended dimension is known, not inside
simd_dot_f32 where caller bugs are indistinguishable from intent).
**Validation-class additions:** (8th) `#[cfg_attr(debug_assertions, ignore)]`
release-required files (ugc_g1b_gate: 0p/3i debug, 3/3 release in 1.4 s);
(9th) `harness = false` + `fn main()` targets have NO test-result line —
validate by exit code + output diff (bench_299_engram_goat: byte-diff showed
only compile-line + debug-timing jitter 698→756 ns, same PASS verdicts);
(10th) heavy-manual all-ignored files (kimi_k3_4b_a3_forward_verify ~17.7 GB
alloc, kimi_k3_phase6_perf 1.5 GB model download) are compile-only — the
aborting-test class needs `-- --skip test_name` for count-identity when the
failure is pre-existing (or fix it, as Issue 669 did). **fmt note:** several
slice-6 files carried pre-existing rustfmt drift, so post-heal fmt converged
them (net +108 lines incl. fmt churn — the heal itself is line-neutral);
post-heal test runs predate the fmt pass, so spot-check one heavily-reformatted
target post-fmt (rustfmt is semantics-preserving by construction but the
spot-check is cheap insurance).
**riir-train slice 1 landed 2026-08-18** (riir-train `e6084429`, this repo
`7ed7c6c`): the sweep's first riir-train unit — engine `src/`
default-compiled files only, 30 files / 183 edits (131 uninlined_format_args,
37 match_bool, 5 map_unwrap_or, 4 single_match, 3 vec-with-capacity,
3 single_match_else). **Scope discipline (the empirical-gate method):** the
engine src/ tree is ~75% feature-gated (~25 distinct gates; only 31 of 118
preview files are default-compiled by parser, 30 confirmed empirically via
rustc dep-info `.d` files after the parse — `csp_answer_pruner.rs` +
`rust_syntax_pruner.rs` parse-DEFAULT but are gated inside `pruner_aux`-style
parents; never trust a parse-only gate map when dep-info can prove it).
Sibling Plan 339/340 files excluded (kimi_k3_train.rs 162 edits,
optimizer_lora_muon{,_sf,_tests}.rs). Validation: check --lib + clippy --lib
0 warnings + 1253/1253 lib tests (default build only — no gate runs needed
by construction). **Root-cause proposer fix landed same cycle** (riir-clippy
`7ed7c6c`): map_unwrap_or on a provably-bool closure + `false` default now
emits `is_some_and` (real clippy 1.97 pedantic's own suggestion; the old
`map_or(false, f)` output triggered the DEFAULT-group unnecessary_map_or —
the heal had created a warning where the original was clean, Issue 027
class, probe-verified on /tmp/probe_mapor); bool+`true` skipped (clippy's
own suggestion there leaves its own warning); 7 fixtures; 555/555 lib.
**Remaining in riir-train** (~3645 edits): engine tests/ 71 files / 1259,
examples/ 88 / 926, gpu 84 / 562 + the excluded sibling files — each needs
its own gate map (tests/ carry `required-features`; gpu pulls cubecl),
slice 2+ material.
**riir-train slice 2 landed 2026-08-18** (riir-train `22447d9d`): engine
`tests/` default-runnable slice — 11 files / 182 edits (176
uninlined_format_args + 6 map_unwrap_or→map_or chain collapses; zero
control-flow rewrites). **Gate-map lesson for TEST files (supersedes
dep-info alone):** a `#![cfg(feature)]`-gated test file COMPILES to an
EMPTY binary under default features and still emits a `.d` file — dep-info
said 17 default-compiled files, but `cargo test -- --list` proved only 2
run by default (goat_247 via collimation_lora riding `training_verification`
transitively + plan_313 ungated). Ground truth for "what runs" is the
per-config `--list` count, not the .d file. **5 validation configs, all
count-identical pre/post-heal (47 tests green both sides):** default
(goat_247 7 + plan_313 1 — the slowest single test at 433s),
jvp_velocity_supervision (6 a5b files / 22 tests; full_go_arena 7 @ 299s),
weaver (bench_314: 1 run + 5 ignored), gemma4_lora (plan320_c2: 4), remax_ppo
(remax_ppo_convnet_minatar: 10). **Pre-existing breakage found:**
plan320_c2_goat_losses under `--features gemma4_train` ALONE fails E0432
(unresolved `gemma4_lora` — the file's inner cfg names gemma4_train but the
import needs the separate gemma4_lora feature, which implies gemma4_train;
the gemma4_train-only combo is a broken gate nobody runs). Clippy 0
warnings in healed files across all 5 configs. **Fmt scoped OUT with
reason:** the 11 files carry 94 pre-existing rustfmt diffs at HEAD + no
rustfmt.toml + no CI fmt gate — wholesale fmt would drag unrelated
formatting into a mechanical diff (the heal adds ~10 net-new diffs, same
class as the pre-existing ones). **Remaining in riir-train** (~3463 edits):
engine tests/ gated remainder ~60 files / ~1077 (each needs its gate's
build+run — the 5-config pattern scales), examples/ 88 / 926, gpu 84 / 562
+ the 4 excluded sibling src files (~206).
**riir-train slice 3 landed 2026-08-18** (riir-train `91c5f68b`): engine
`tests/` **xhc_pretraining gate cluster** — the largest single-feature
cluster in the gated remainder. 7 files share the gate (phase1-6c),
5 carried edits: **64 edits (61 uninlined_format_args + 3
vec-with-capacity)**; phase2/phase3 were clean. **Gate-map completion
lesson:** a whole-file `grep -n '#!\[cfg'` is required — phase1's gate
sits at line 38 BELOW the head-30 doc-comment window, so a head-N scan
under-reports; the complete map = Cargo.toml `[[test]] required-features`
rows AND a whole-file inner-cfg grep (files can have BOTH — xhc does).
**Validation under `--features xhc_pretraining`** (isolated /tmp target):
compile 0 errors, clippy 0 warnings in healed files, 0 remaining
uninlined_format_args (pedantic -W check); tests count-identical pre/
post-heal all green: phase1 5/0/0 (0.15s), phase4 4/0/1 (67s), phase6
4/0/1 (534s), phase6b 2/0/1 (938s), phase6c 2/0/2 (494s). **Slow-binary
lesson:** the cluster runs ~35 min/side (phase6b 15.4 min alone) — check
per-file runtime BEFORE the baseline (the slice-2 rule), then (a) split
post-heal runs one binary per command under the 20-min timeout, (b) SKIP
re-running binaries whose files got 0 edits (phase2 108s + phase3 25s
saved — content unchanged means the baseline run stands), (c) unscoped
`cargo test -p` builds the 1253-test lib suite too — always scope with
`--test <name>` (an unscoped run OOM-killed here).
**Remaining in riir-train** (~3399 edits): engine tests/ gated remainder
~53 files / ~1013 (largest next clusters: lora_still 4 files, nextlat 3,
ciacot 4 AVOID real-model deps, goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206).
**riir-train slice 4 landed 2026-08-18** (riir-train `1c03e1eb`): engine
`tests/` **lora_still gate cluster** — the largest next single-feature
cluster. 4 files share the gate (goat_267 family, BOTH mechanisms again:
Cargo.toml required-rows + inner `#![cfg]`); 3 carried edits: **10 edits
(all uninlined_format_args)**; goat_267_lora_still_training clean (0 edits).
Pure CPU feature (`lora_still = ["riir-engine/lora_still"]`, 0 GPU deps).
Cluster is FAST — every binary sub-second (4+4+7+8 tests, 0.03/0.02/0.11/
0.02s), so combined post-heal runs in ONE command are fine here (the
slow-binary split-run pattern from slice 3 applies only when per-binary
runtime demands it). Validated under `--features lora_still` (isolated
/tmp target): tests count-identical pre/post (7/4/4 green both sides,
training 8 green baseline), clippy 0 warnings in healed files, 0 remaining
uninlined_format_args. **Side observation (follow-up candidate):** the
pedantic -W check surfaced **17 pre-existing lib-side uninlined_format_args
in lora_still-compiled src files** (collimation_lora.rs etc.) — feature-
gated lib surface the default-compiled src slice 1 never saw; candidate
for a future src/ gated-lib slice.
**Remaining in riir-train** (~3389 edits): engine tests/ gated remainder
~49 files / ~1003 (largest next clusters: nextlat 3, ciacot 4 AVOID
real-model deps (gemma2/qwen25), goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206).

**riir-train slice 5 landed 2026-08-18** (riir-train `6ed32675` +
riir-clippy proposer `6aa0c82`): engine `src/` **lora_still lib slice —
the slice-4 side observation, corrected + closed**. The "17 gated lib
findings" framing was WRONG in an instructive way: enumerating the
findings under `--features lora_still` then under DEFAULT showed **12 of
13 are default-visible** — `collimation_lora.rs` (7 findings) sits behind
`#[cfg(feature = "collimation_lora")]` in lib.rs but `training_verification`
(the ONLY default feature) transitively enables `collimation_lora` — a
**parse-only gate-map failure in the REVERSE direction** from the slice-1
lesson (slice 1: parser said default, dep-info proved gated; here:
lib.rs cfg said gated, feature-chain reality said default). Gate truth =
resolve the feature CHAIN (default → implied features → compiled mod
set), never the cfg attribute alone. The remaining finding split:
3 × `game_concept_graph.rs` `bail!` + 2 × `rosetta/mod.rs`
`anyhow::ensure!` were sites slice 1's heal structurally could NOT touch,
and only 1 (`lora_still_training.rs`) was genuinely lora_still-gated.
**Proposer extension (the root-cause half, riir-clippy `6aa0c82`):**
anyhow's `bail!` (template-first, like panic!) + `ensure!` (cond-first,
like assert!) joined the macro family table, + `macro_family` now
resolves the LAST path segment instead of `get_ident()` (single-segment
only) so qualified `anyhow::ensure!`/`anyhow::bail!` — the production
spelling — reach the family match; qualifier-agnostic like real clippy's
post-expansion behavior. Probe evidence = the live findings themselves.
5 new fixtures (bare bail/ensure, qualified bail with escaped quotes,
qualified ensure, qualified std::format, ensure-without-message negative
control) + a real gpart.rs clean-span entry (already-inlined `{version}`
+ no-placeholder templates must stay silent). Gates: 560 default / 612
latent_retrieval / 623 fix_verify lib tests, clippy 0; consumer proof:
all 13 sites healed end-to-end. **Heal: 4 files / 54 edits** (the extra
41 beyond the 13 lib findings = cfg(test) mod sites the `--lib` clippy
scan doesn't see + 3 map_unwrap_or→map_or) — lib tests count-identical
pre/post (default 1253, lora_still 1271), clippy 0 both states, pedantic
-W residue 0 both states.
**Remaining in riir-train** (~3335 edits): engine tests/ gated remainder
~49 files / ~1003 (largest next clusters: nextlat 3, ciacot 4 AVOID
real-model deps (gemma2/qwen25), goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206 —
re-check sibling state at slice time). The lib-side gated-surface scan
(other features' src residue) remains a standing follow-up — enumerate
per feature the same way (default diff vs feature-on diff) before
assuming a finding class is gated.
**riir-train slice 6 landed 2026-08-18** (riir-train `0f76ae41`): engine
tests/ **nextlat-family gate slice** — the map's "nextlat 3" cluster was
actually a **7-file gate family**: the 3 nextlat_training binaries + 3
superset-feature siblings (substrate_nextlat / dagger_lora /
compression_sweep — all `= ["nextlat_training"]`) + the 4-feature-union
goat_gate. **One feature cycle covers the whole family** (the kimi_k3_backward
superset pattern): `--features nextlat_training,substrate_nextlat,dagger_lora,
compression_sweep`. 6 files / 35 edits: 4 test files / 30 (uninlined 28 +
map_unwrap_or→map_or 2) + 2 SRC stragglers / 5. goat_248_nextlat /
bench_248_nextlat_mlp / bench_254_goat_gate scanned clean (0 edits — no
validation runs needed, the slice-3 skip rule).
**The cheap all-ignored validation win:** all 4 edited binaries are
all-#[ignore] benches — a debug `--ignored` run gives full panic-checking
validation at 0.5-13 s per binary (~16 s total; the bench_core_optimization
pattern from the katgpt-rs slice 6).
**Third enumeration blind-spot class (the slice-5 lesson extended):** the
union `--tests` residue check exposed 2 src files that BOTH prior
enumerations missed — `src/compression_sweep.rs` (439 map_or in PRODUCTION
code + 731 assert) + `src/collimation_game_attribution.rs` (400 assert,
default-VISIBLE via the training_verification→collimation_lora chain).
Chain-default + cfg(test)-mod = invisible to slice 1's parse-only gate map
(said gated), slice 5's `--lib` clippy scan (no cfg(test)), AND default
`--lib` sweeps. **Enumeration rule for any gate slice: run `--tests` (not
just `--lib`) under the gate's feature union, then heal the union-minus-default
delta INCLUDING src cfg(test) mods.** Validation: lib count-identical
(default 1253 / union 1382 pre+post), benches 4/7/4/6 `--ignored`, clippy 0
residue union `--tests` + plain default.
**Remaining in riir-train** (~3300 edits): engine tests/ gated remainder
~43 files / ~968 (next clusters: goat_296 family ~7 via per-file feature
gates, degrpo/wasm_reward_shaping/ted/posterior/mdl 2 each; ciacot 4 AVOID
— real-model deps (gemma2/qwen25)), examples/ 88 / 926, gpu 84 / 562 +
the 4 excluded sibling src files (~206 — re-check sibling state at slice
time). The lib-side gated-surface scan remains the standing follow-up
(now with the --tests cfg(test) component folded into every gate slice).
**riir-train slice 7 landed 2026-08-18** (riir-train `639d2eec`): engine
tests/ **goat_296 per-file-gate family slice** — the map's largest named
cluster: 7 test binaries, each behind its OWN feature gate (no supersets —
7 independent gates). One feature cycle with the 7-feature union covers
all: sparse_subnetwork_training (zero-dep), off_principal_init
(katgpt-core/newton_schulz), pion_optimizer_goat (riir-gpu/pion_optimizer
— compiles clean on M3 Metal; CPU clippy+test cycle, no GPU-exclusivity
concern), module_rank_profile_goat, sparse_lora_export_goat,
cross_game_mask_transfer_goat, cross_game_retention_goat (= cpu_lora_train
+ cross_game_mask_transfer). **6 files / 104 edits**: uninlined_format_args
38 clippy findings = 104 placeholder-level edits + 1 single_match_else
collapse (cross_game_retention run_game); goat_296_module_rank scanned
clean (0 findings, no validation run — the slice-3 skip rule). Per-file
edit counts (the `fix <path> (N edits)` review sections): pion 36,
off_principal 34, sparse_subnetwork 22, mask 5, retention 5, export 2.
**Validation:** 6 binaries count-identical pre/post (3/2/5/5/2/3 all
green, ~2min/side — cross_game_retention 105s the slow one); clippy
--tests union residue 0 in healed files; src-side union-minus-default
residue 0 (the slice-6 standing rule check — NO stragglers this time,
the src surface for these 7 gates is already clean); default-features
run skipped with reason (test-file-only edits — the lib is bit-identical
under any feature set, so a default cycle proves nothing about them).
All 6 files rustfmt-clean at HEAD → rustfmt applied post-heal (the katgpt
pattern; re-indents the single_match_else body), net −49 lines.
**Remaining in riir-train** (~3196 edits): engine tests/ gated remainder
~36 files / ~864 (next clusters: degrpo/wasm_reward_shaping/ted/posterior/
mdl 2 each; ciacot 4 AVOID — real-model deps (gemma2/qwen25)), examples/
88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206 — re-check
sibling state at slice time).

**riir-train slice 8 landed 2026-08-18** (riir-train `07f39f27` + the
proposer root-cause fix riir-clippy `fa820d0`): engine tests/ **the 5 small
clusters — degrpo / wasm_reward_shaping / ted / posterior / mdl — ONE
5-feature-union cycle** (all pure-CPU; posterior_training pulls
rank_one_spectral, ted_regularized_lora implies rule_init_lora). 10 test
binaries scanned (the ted cluster = goat_244_rule_init_lora +
rule_init_convergence — goat_244's gate is the INNER
ted_regularized_lora, not rule_init_lora); 7 clean → skipped (slice-3
rule). **8 files / 48 edits** = 20 clippy findings: uninlined 9 (37
placeholder-level edits), match_bool 4, map_unwrap_or 6, single_match_else
1 (goat_posterior_264:602 — the diagnostic's "equality check" text is the
single_match_ELSE lint per its help URL; the healer's rule choice was
correct).
**TWO proposer root-cause fixes shipped mid-slice** (the bench_241 sites
were invisible to the old proposer): (1) the `unwrap_or_else` arm —
`x.map(f).unwrap_or_else(g)` → `x.map_or_else(g, f)` (default closure
FIRST, mapper second — probe-verified against clippy 1.95's own
suggestion on the live sites); (2) **macro-argument descent** — the 4
bench_241 sites sit inside `writeln!` args, which are OPAQUE token
streams to syn's visitor (real clippy matches post-expansion HIR);
visit_expr_macro/visit_stmt_macro now reuse the format-family table via
the shared `post_template_expr_args` helper, parse each post-template
arg as an Expr, run the same matcher — proc-macro2 spans survive the
token round-trip so edits slice the ORIGINAL file bytes. Named/
non-parsing args are skipped, not aborting. 8 new fixtures; 568 lib
tests (+8); clippy 0.
**Third slice-1 miss class closed: nested-path default file.**
lattice_operad/bench.rs is default-visible (ungated `pub mod` +
non-cfg(test) `pub mod bench`) — its map_unwrap_or_else fired under
default `--lib`; invisible to slice 1's parse+dep-info map. Post-slice-8
the engine default `--tests` residue is ZERO (fully clean).
**Validation (all count-identical pre/post):** bench_246_mdl 7/0,
goat_posterior_264 6/0, posterior_arena_proof 5/0, default lib 1253+1i,
union lib 1386+1i, bench_241's own 12 tests green under
wasm_reward_shaping. Union `--tests` residue 0 in healed files.
rustfmt applied ONLY to the 4 files fmt-clean at HEAD (mdl_curriculum,
ted_regularizer, lattice_operad/bench, degrpo); the 3 test files +
bench_241 carry pre-existing fmt drift (scoped out, the slice-2 rule);
post-fmt lib re-run count-identical (the slice-6 spot-check rule).
**Side observation:** the gpu-crate findings visible under this union
(loss_sdar 5, compress 3, training_loop/types 2, + 4 singles —
DEFAULT-visible in riir-train-gpu) belong to the gpu 84/562 bucket,
confirmed not lost — the gpu slice owns them.
**Remaining in riir-train** (~3146 edits): engine tests/ gated remainder
~26 files / ~814 (ciacot 4 AVOID — real-model deps (gemma2/qwen25); the
remainder is the long tail of 1-file gates + the manifold/bake/archetype
families), examples/ 88 / 926, gpu 84 / 562 + the 4 excluded sibling src
files (~206 — re-check sibling state at slice time).
**T5 — DONE same day** (benches/ swept,
katgpt-rs `4ccb2f280`:
bench_424:97 range_plus_one + bench_136:208 uninlined — range_plus_one
now ZERO findings across all targets).
Note: syn 2.0.119's `PatLit.lit` holds a `Lit` (not `Box<Expr>`) whose
token preserves a leading `-` — the negative-literal exclusion checks the
token text. Gates: 36 range/single_match tests (23 single_match_else) +
28 uninlined tests / 514 default lib / retrieval floors PASS / clippy 0
lib + all-targets.

**riir-train slice 9 landed 2026-08-18** (riir-train `d64bc8ae`): engine
tests/ **the manifold/bake/archetype family union** — ONE 9-feature cycle
(boundary_stages,federated_alignment,curvature_regularization,symmetric_boundary,
dual_direction + manifold_lora,tjs_lora + bake_lora + archetype_library_training;
all CPU — archetype pulls dep:riir-neuron-db + katgpt-core/committed_field_blend,
in-tree, no models). 12 test binaries scanned; 7 clean → skipped (slice-3
rule). **5 test files / 150 edits + 2 gated-src stragglers / 38 edits**:
uninlined 184 (placeholder-level), match_bool 3 (goat_manifold_261 + msa
rescue — `match cond { true => X, false => Y }` → if/else, spot-checked),
vec-with-capacity 1 (deep_manifold_integration_test:99, the 1000-push
losses vec). The src delta (slice-6 standing rule): archetype_library_train.rs
(16 — incl. the PRODUCTION hex_encode `{:02x}` at 1182) + tjs_lora.rs (22,
box-drawing GOAT tables in cfg(test)). **Filename gotcha re-hit:**
bench_300_t4_4_msa_rescue is a `.goat.rs` file (target name
bench_300_t4_4_msa_rescue_goat via the [[test]] name field) — a wrong-path
scan silently covers 4-of-5 files (the slice-5 lesson, now twice-observed).
**Test-artifact hazard (new):** issue_307_archetype_library_gates REGENERATES
`models/archetype_library_v1/*.neuron` + manifest on every run (Bin 400→432
bytes) — restored via `git checkout -- models/` before both commits; future
slices touching archetype tests must NOT stage models/ churn.
**Validation (all count-identical pre/post):** the 5 edited binaries 1/2/13/1/5
(msa rescue 1, deep_manifold_integration 2, bake 13, manifold_261 1, archetype
5), union lib 1339+1i (pre/post via the copy-out/revert/baseline/restore dance
— the src heal landed before the lib baseline), default lib 1253+1i. Union +
default `--tests` residue 0 in the engine; plain clippy 0 new warnings.
rustfmt applied to all 7 (all fmt-clean at HEAD); post-fmt re-run
count-identical. Known divergence recorded: `match sphere_pass { true =>
"PASS", ... }` inside println! args stays healer-silent (macro-arg descent
is format-family-only) — real clippy would flag it; the documented
we-stay-silent class.
**Remaining in riir-train** (~2958 edits): engine tests/ gated remainder
~38 files / ~664 (the prior ~26-file estimate undercounted; the honest
enumeration: lota_ternary x2, lr_qat, dl_qat, coa_meta, osc_rat_plus,
zero_qat, gpart, rosetta_lora, rosetta_filter, polarization_stop,
adapter_centroid, nf_riir, snr_filter, eql x2, curvature_curriculum, nitp,
peira, set_diffusion x3, multi_precision_npc, latent_functor, fol,
dendritic, d2f_quality, ccar x2, integration_244, memory_soup family x8,
bench_401 sibling-caution; ciacot 4 AVOID — real-model deps (gemma2/qwen25)),
examples/ 88 / 926, gpu 84 / 562 + the 4 excluded sibling src files (~206 —
re-check sibling state at slice time).

**riir-train slice 10 landed 2026-08-18** (riir-train `ac082be1`): engine
tests/ **the long-tail + memory_soup mega-slice — the named list from the
slice-9 map is now COMPLETE**. ONE 23-feature-union cycle (latent_functor-
rank_k-transfer, osc_rat_plus, coa_meta_adapter, dl_qat_magnitude,
lota_ternary_lora, lr_qat_lora, fol_extraction+game_concept_graph, gpart,
adapter_centroid [papaya], nf_riir_lora, dendritic_lora+ns_csg, eql_explain,
multi_precision_npc, nitp_loss, peira_loss+peira_metrics,
memory_soup_training+collimation_lora, set_diffusion, ccar_routing_goat
[katgpt-band], multi_source_lora — all CPU, in-tree). 38 files scanned /
26 edited (433 edits: 412 uninlined_format_args + 10 match_bool + 6
identity_op + 3 single_match_else + 1 vec-with-capacity + 1
collapsible_else_if) + **5 gated-src stragglers** (93 edits: eql_symbolic 20,
eql_explain 15, loss_nitp 3, multi_source_lora 1, dendritic_training 1 —
incl. 2 PRODUCTION single_match_else if-let rewrites in eql_symbolic.rs,
spot-checked semantically identical) + 12 clean files skipped (slice-3
rule).

**Exclusions honored:** bench_253_rosetta_memory_soup DEFERRED
(rosetta_cross_crate = dep:riir-gpu — belongs to the gpu bucket), ciacot 4
AVOID (real-model deps), bench_401 sibling-caution (Plan 339/340 territory).

**Validation (all count-identical pre/post):** all 26 binaries +
goat_240_eql_symbolic (src-affected CLEAN file — copy-out/revert/baseline/
restore dance, 10/0/0 both sides; the slice-9 method, now twice-proven);
union lib 1523+1i (the biggest union lib yet — memory_soup/rosetta/gpart
enable many cfg(test) mods); default lib 1253+1i (src edits confirmed
union-gated); clippy `--tests` union residue: **2 documented divergences**
(eql_explain.rs:243 — raw-string template `r#"{{...}}"#` w/ escaped braces,
the conservative JSON guard; eql_symbolic.rs:1036 — `format!("{}", x)`
nested inside assert_eq! COMPARISON args, second-level macro nesting the
descent doesn't recurse into; both join the we-stay-silent class — 2/526 =
99.6% coverage) + 2 riir-train-gpu sites (loss_sdar.rs:304, training_loop/
types.rs:621 — the gpu bucket's known property). rustfmt applied to all 31
(all fmt-clean at HEAD); post-fmt spot-check count-identical. No models/
churn (no archetype tests run). Pre-existing noted-not-fixed: unused `b_len`
at multi_source_lora.rs:113 (gated-src warning, not a heal-class lint).

**Terminal-tool workaround confirmed:** `'.' is ambiguous in multi-root
workspaces` fires RELIABLY on `cd <abs-path> && <cmd-with-relative-paths>`
but NOT on the tool's `cd` parameter + absolute paths in the command body —
the slice-9 "transient" is deterministic; use the parameter form.

**Remaining in riir-train** (~2432 by arithmetic: 2958 − 433 − 93): engine
tests/ exceptions only (ciacot 4 AVOID real-model deps; bench_401
sibling-caution; bench_253_rosetta_memory_soup gpu-gated) — **the engine
tests/ long tail is DONE**; engine examples/ 88 / 926; riir-train-gpu 84 /
562 (owns the known loss_sdar/compress/training_loop findings + the 2 sites
above); the 4 excluded sibling src files (~206 — re-check sibling state at
slice time); standing follow-up: lib-side gated-surface scan per feature
(this slice retired 5 of the gated-src surface files).

**riir-train slice 11 landed 2026-08-18** (riir-train `a1197e06` + the
proposer root-cause fix riir-clippy `c73770e`): engine **examples/** cheap-
gate slice — **49 examples / 578 edits + 11 gated-src / 139 edits (60
files, +737/−1087 post-fmt, net −350 lines)**. ONE 36-feature-union cycle
(all CPU, in-tree). **Gate-map discovery:** the examples tree is 120/124
GATED (only 4 default-compiled) — the healer dry-run found 90 files / 985
edits, classified by feature-dep chain into cheap (57 files/616 —
cpu_lora_train ×15 is zero-dep; game_llmexec + wasm_proof_witness + a dozen
more are EMPTY features) vs heavy (31/358 — kimi/gemma4/gemma2_lora_gpu/
shine/weaver/flashmemory/teacher/strandset/go, deferred) vs sibling-WIP
(13 src/370 — the Plan-339 kimi/muon/weaver/xhc family, last commit 00:04,
no fresh WIP but their plans 339/340/341 are live). **Dep-info src delta
(the slice-6 standing rule, examples variant):** `cargo build --examples
--features <union>` `.d` files → 131 src files compiled → intersect the
healer's 74-file src list → exactly 12 in-scope (reward_projection_data 60,
svr_driver 38, loss_latent_pred 9, csp_answer_pruner 7, svr 6,
token_sampler 4, smooth_latent 4, svr_answer 3, g_rollout 3,
rust_syntax_pruner 3, action_hoare 2, lora_outlier_guard 2 = 141; minus
action_hoare after the proposer fix below = 139 landed). 49 src files /
208 edits are NOT in this union (gated by other features — the standing
follow-up owns them). **Validation (all count-identical):** union examples
build exit 0 (65 binaries — dev-deps incl. riir-games/civ compile anyway,
making the civ/dual_leo examples cheap); default lib 1253/0/1i; union lib
**1802/0/1i** (the biggest union yet); spot-runs: trajectory_evolve_demo +
zero_qat_calibration byte-identical, ties_merge_bench timing-only diff
with ALL gate verdicts PASS both sides; clippy residue 0 across
uninlined_format_args/match_bool/map_unwrap_or (examples + src); rustfmt
applied to the 32 fmt-clean-at-HEAD files.

**THE HEADLINE — first live compile-gate catch ROOT-CAUSED in the healer
(the action_hoare.rs E0308):** the `bool-to-bitpack` proposer rewrote
`count_satisfied(&[bool])` → `&[u64]` while its live same-file caller still
passed `&[bool]`. A signature-changing rewrite cannot be applied
mechanically — it breaks every time the pattern has a live caller (the
NORMAL case; a zero-caller fn is dead code). **Fix (riir-clippy `c73770e`):
both signature-changing proposers (`bool-to-bitpack` param type +
`static-str` return type) now carry a caller gate — they refuse when the
fn ident occurs more than once in the file (word-boundary scan; call
sites, fn-as-value, macro args, comments all count). New fixtures for
both (mirroring the production shape) + `param_name_preserved` renamed
`fn count` → `count_bits` (its own `.count()` tripped the gate). 570/0
default lib, clippy 0, verified end-to-end (0 edits on action_hoare.rs).
`fxhashmap` is now the one remaining body-local type-changing rewrite
without a caller gate (type-flow exposure, self-gated on an existing
rustc_hash import).**

**Process lessons (both self-inflicted, both recovered):** (1) a
"dry-check" fmt loop accidentally executed `git stash push -- $f` per
file (62 stash entries; the sibling's pre-existing autostash sat at the
bottom — `.plans/320`, Cargo.lock, plan320 example, svr_driver — PRESERVED,
its pop correctly aborted on conflict). Recovery = sequential `git stash
pop` (60/60 restored, verified file-set-exact vs the planned scope).
NEVER put a stash call in a check script — the check ran it before the
intended `git show` fallback; use `git show HEAD:<f>` only. (2) One
timing-gate flake (`lattice_operad::transfer::tests::bench_transfer_overhead`
111.9x vs 100x) fired in the full parallel suite while the katgpt-rs
sibling's release bench ran at 298% CPU (load 7.7-9.3) — passes isolated
AND at `--test-threads=2` (the slice-5 flake rule + bench_290 precedent;
untouched file).

**Remaining in riir-train** (~1715 by arithmetic: 2432 − 578 − 139):
engine examples/ heavy remainder 41 files / 407 edits (kimi ×13/202,
gemma4 ×17/56, gemma2_lora_gpu + gemma2_latent_cot ×6/36, shine ×2/20,
weaver ×2/6, teacher ×2/3, strandset 1/5, go ×2/4, flashmemory ×2/4);
sibling-WIP src (kimi/muon/weaver/xhc family ~366 — re-check at slice
time, plans 339/340/341 live); 49 out-of-union src / 208 edits (the
standing gated-surface follow-up); engine tests/ exceptions (ciacot 4
AVOID real-model deps; bench_401 sibling-caution; bench_253_rosetta gpu-
gated); riir-train-gpu 84 / 562 (owns loss_sdar/compress/training_loop +
the 2 known sites).

**riir-train slice 12 landed 2026-08-18** (riir-train `67a77667` + the
proposer root-cause fixes riir-clippy `f0b47b3`): the **riir-train-gpu
crate slice** — the 84/562 bucket closed as **56 files / 425 edits**
(dry-run ground truth 63/468; 7 files/43 edits excluded). ONE
32-feature-union cycle. **Exclusions:** 3 sibling-WIP (muon_dense_cuda.rs
+ _sampled.rs at fresh Plan-339 mtimes 00:03, bench_335_hyperball_goat.rs
Plan-335 — the Aug-17+ mtime find is the detector) + 4 pre-existing
broken gates (dllm+gdsd_loss: distill_gdsd.rs:337 E0063 DllmTrainingReport
missing eval_errors/eval_masked_positions/train_loss_errors — the
bench_gdsd_gpu_goat gate compiles NOTHING at HEAD; sft_warmup:
bench_sft_rl_pipeline 6× E0063 mux_config + E0432 loss_asft — unfixable
by feature selection). **Union shape lessons:** the union needed
dion3_row_selection added (the sibling's committed muon tests reference
dion3-gated slab_l1_norms/gram_ns_ortho_slab — muon_dense_cuda without
its dev feature is another broken-at-HEAD gate) + asft_loss +
lora_outlier_guard (dasd_lora_goat's stale feature row); katgpt-rs is
STRUCTURALLY in the dep graph (riir-engine → katgpt-rs → katgpt-backend →
katgpt-forward) — dropping kimi features does NOT avoid it, so the
sibling's mid-flight TreePath migration blocked batch-2 for ~15 min
(poll `cargo check -p katgpt-forward` until coherent, then race the
build). **TWO proposer root-cause fixes** (the slice's compile-gate
catches): question_mark reference-scrutinee (match &self.field →
`self.field.as_ref()?`, not `(&self.field)?` — real clippy's own
suggestion is MaybeIncorrect here, probe-verified via JSON applicability)
+ vec-with-capacity suffixed literals (0..5u64 → with_capacity(5), bare
digits infer usize). **Validation all count-identical:** union lib
576/16/3i @ threads=2 (16 = 15 CUDA-absent + distill_attention_hla_
converges CPU divergence, both pre-existing), default lib 186/0/0, all
23 runnable test binaries (incl. goat_195's pre-existing 0/2 red),
test_kimi_k3_cublas_forward_g1 compile-only; 3 full-parallel flakes (2×
muon optimizer gpu_step + training_loop toy) all pass isolated + at
threads=2 — contention class, sibling bench at 200% CPU concurrent.
**Fmt-scoping lesson:** the stdin-mode `rustfmt --check` (git show |
rustfmt) is LENIENT vs file-mode — 11 files classified CLEAN were
actually drift at HEAD; caught post-fmt via file-mode re-check, fixed by
revert + re-heal (11 files land unfmt'd per the slice-2 drift rule;
45 fmt'd). Always fmt-check via `git show > tmp; rustfmt --check tmp`.
**Test-artifact hazard:** bench_interventional_sft writes a results CSV
(rm'd, unstaged). Clippy residue 0 in healed classes (union + default).

**Remaining in riir-train** (~1290 by arithmetic: 1715 − 425): engine
examples/ heavy remainder 41 / 407 (kimi ×13 needs sibling Plan-339/340
files settled; gemma4 ×17/56 plan320-autostash-adjacent — re-check at
slice time; gemma2_lora_gpu+latent_cot ×6/36, shine ×2/20, weaver ×2/6
teacher ×2/3, strandset 1/5, go ×2/4, flashmemory ×2/4); sibling-WIP
src (kimi/muon/weaver/xhc family ~366); 49 out-of-union src / 208; the
7 gpu-crate exclusions (43 edits: 3 sibling + 4 broken-gate — the
broken gates need owner fixes, not heals); engine tests/ exceptions
(ciacot AVOID, bench_401 sibling-caution); engine benches/ unswept.

**riir-train slice 13 landed 2026-08-18** (riir-train `df0ab789`): the
examples/ heavy remainder MINUS kimi/gemma4 — **16 files / 78 edits**
(15 examples + shine_d2l_train.rs src, 1 edit) across ONE 8-feature-union
cycle (shine_training, go_training_arena, weaver, teacher_data_gen,
strandset_data, gemma2_lora_gpu, gemma2_latent_cot,
flashmemory_indexer_train). **Pre-existing broken gates found at HEAD +
repaired** (all riir-ai Issue 701/699 API drift, fixes determined by the
API's own docs): shine_train_meta_lora + bomber_14
`compute_baseline_loss` ctx_len arg; go_11_shine_routing ×2
`route_context_to_expert` Option return (let-else); **shine_d2l_train
test gate was 7/17-red at HEAD** (Issue 701 H-K hard error on the toy
config's under-provisioning) — `memory_length: Some(4)` → `None` (derive,
the sanctioned shape) → 17/17 green. Validation: union examples build
EXIT 0; shine_d2l 17/17 pre+post; default lib check clean; clippy residue
0 in healed classes. **CRLF trap (new fmt class):** issue429_logit_drift.rs
has CRLF line terminators at HEAD — `rustfmt --check` calls it CLEAN but a
write-mode fmt CONVERTS EOL, exploding the diff to the whole file
(`@@ -1,574 +1,552 @@`, every line). Caught via diff-stat; recovered with
`perl -i -pe 's/\n/\r\n/'`. Check EOL before fmt-write: `git show HEAD:f |
head -c 400 | xxd | grep '0d 0a'`. **Exclusions:** plan320_gemma4_q4k_gpu_train
(mtime Aug 17 = sibling plan320 territory), issue446_single_layer_lora_poc
(mtime Aug 18 06:48 = sibling-fresh; its 1 uninlined site stays), weaver.rs
+ weaver/autograd.rs ×5 (sibling Plan-339/340 src), riir-ai gpu-dep
residue ×4 (not this repo). 2 residual `unnecessary_sort_by` (default-on
lint, no corpus proposer — the manual_is_multiple_of corpus-gap class).
**Remaining:** ~1212 by arithmetic (1290 − 78): kimi ×13/202 + gemma4 ×17/56
+ plan320 (sibling-adjacent, re-check at slice time); sibling-WIP src
~366; 49 out-of-union src / 208; the 7 gpu exclusions; tests/ exceptions;
benches/ unswept.

**map_unwrap_or array-literal guard landed 2026-08-18** (`2bf2a44`, found
live in a concurrent session's WIP): `x.map(f).unwrap_or(&[...])` →
`x.map_or(&[...], f)` relies on the CALL SITE forcing U for the unsize
coercion `&[T; N] -> &[T]` — probe-verified both `&[]` AND `&[1.0]`
defaults E0308 in inference-transparent positions (plain `let`) while both
compile context-forced (function arg with known `&[f32]` param); the
original `map(f).unwrap_or` form ALWAYS compiles (`map` resolves the type
before `unwrap_or` unifies). The proposer cannot see call-site context →
guard skips ALL array-literal defaults, both forms (direct for `unwrap_or`,
default-closure body for `unwrap_or_else`); casts exempt (`as` ascribes).
3 new fixtures; 577 default lib tests; joins the we-stay-silent divergence
class (real clippy still flags the shape; our output would not compile).

**engine benches/ slice BLOCKED-DEFERRED 2026-08-18** (dry-run ground
truth: bench_382_math_dedup ×1 [default-compiled] + bench_317_poincare_g2_strict
×23 [`poincare_phi_train`] + gpu benches bench_381_artifact_freeze_goat ×19
[`artifact_freeze`] + bench_421_muon_ns_dispatch ×1 [`lora_muon_optimizer`]
= 44 edits across 4 files): **every non-default build path is blocked by a
concurrent session's live katgpt-rs src/ heal WIP** (137 modified files,
mid-flight since ~09:29) carrying the exact E0308 the guard above now
prevents — `katgpt-forward/src/step.rs:196` `map_or(&[], |m| m.as_slice())`
(3 instances in their diff) breaks every riir-train build whose dep chain
compiles katgpt-forward (poincare, gpu-crate, AND — after fingerprint
invalidation — the default path too; the default-path baseline squeaked
through at 09:28, ~60s before their 09:29:27 edit). bench_382's heal was
applied + surgically fmt'd, then REVERTED unvalidated per the discipline
(an unvalidated heal is not a heal — their tree is the live demonstration).
**Re-scan at next cycle WRITE time: `stat step.rs` + one default-features
`cargo bench -p riir-train-engine --bench bench_382_math_dedup` probe; once
the sibling's tree compiles, the full 44-edit slice is ~30 min** (heal
re-application is 1 command; baselines: 382 exit-code/output, 317
exit-0-all-pass semantics, 381+421 exit+output).

**riir-neuron-db slice 1 landed 2026-08-18** (riir-neuron-db `5db2e3d`) —
the re-probe confirmed the block STILL LIVE (step.rs mtime unchanged
09:29:27, E0308 reproduced via a direct `cargo check -p katgpt-forward`),
so the sweep pivoted to the only buildable heal surfaces while katgpt-rs
WIP is broken: **riir-chain + riir-neuron-db (neither depends on
katgpt-forward — leaf crates on katgpt-core only)**. Dry-run sizing:
chain 105 files / 420 edits, ndb 66 files / 978 edits (946 uninlined —
pedantic-group, invisible to default clippy, hence the accumulation).
This cycle: **ndb src/+tests/ — 37 files / 248 edits** (uninlined 231,
match_bool 12, map_unwrap_or 8, single_match_else 5, range_plus_one 1;
rustfmt on the 14 fmt-clean-at-HEAD files, 23 carry pre-existing drift
→ left unfmt'd per the slice-2 rule). Validation ALL count-identical:
lib default 412p/0f/4i + lib --all-features 1090p/0f/14i + all 7 test
binaries under their exact gates (elastic_lifecycle, limit_recall,
dense_embed+diverse_retrieval+local_kv, ed_freeze_poc, seed_recall,
item_funcattn_scorer, hebbian_fact_store+spectral_shard). Clippy
all-features: 1 PRE-EXISTING warning (index/mod.rs:397 unused_assignments
— untouched line, heal was at 1665), 0 uninlined residue.

**Tooling lesson (cargo tree --bench is a FALSE-negative probe):**
`cargo tree -p <pkg> --bench <name>` FAILS with "unexpected argument"
(exit 1, stderr) — piped `2>&1 | grep -c katgpt-forward` counts 0 matches
in the ERROR TEXT and reads as "not in graph". This cycle that
false-negative nearly overturned the (correct) block diagnosis: all four
bench targets showed "0 katgpt-forward" while the actual build failed on
katgpt-forward via `riir-train-engine → riir-engine → katgpt-rs →
katgpt-backend → katgpt-forward` (+ riir-engine → katgpt-forward direct).
Correct probes: `cargo tree -p <pkg>` (no --bench) or an actual build.

**Gate-map lesson re-confirmed (3rd time) + the shared-checkout interleave:**
4 ndb test files carry inner `#![cfg(feature)]` gates at lines 36-97 —
BELOW the head-30 doc-comment window (slice-3 lesson); the whole-file
`grep -n '#!\[cfg'` is mandatory and "0 passed" on a default run is the
reliable detector. A sibling commit (`7d19917`, Issue 603) landed on the
shared checkout mid-cycle touching only `.benchmarks/`+`.issues/` (dot-dir
benches never compile into --lib/--test runs) — 0 file overlap, baselines
unaffected; `git add src tests` scoping kept my commit exactly 37 files.

**Baseline flake documented (thermal/load class, user-suggested):** the
first COLD build+test run failed 1 timing test in BOTH lib configs
(411p/1f + 1089p/1f — compile load + M3 thermal throttle under the 503s
cold build); warm re-runs passed clean 412/0 + 1090/0 twice each. The
slice-5 re-run rule applies: a post-heal perf-gate mismatch needs a
re-run before it counts as a regression.

**Remaining heal territory (both unblocked during the katgpt-rs WIP):**
ndb benches/ 27 files / ~730 edits (validation class 9 per bench — slow,
future slice) + ndb examples/ 2 files; riir-chain 105 files / 420 edits
(root lib src/ first, then tests/, then chaind + wallet crates — NOTE the
riir-chain branch rule: one branch per ticket + `git merge --no-ff` into
develop, NOT direct commit).

**slice 7 landed 2026-08-18** (katgpt-rs `805feb22`..`81ef2631`, 12
commits) — **the katgpt-rs root tests/ sweep is COMPLETE**: 40 files /
358 heal edits + 1 manual lint + 3 manual import-gate fixes across 12
feature-gate cycles, all count-identical or same-signature validated, 0
compile-gate reverts in the applied set. Cycle shape (cheapest-first,
unrelated light features combined per the slice-6 method): c1 11 files/45
edits (9 forward-only features in ONE cycle — dec_operators +
dflare_kv_routing + cs_kv_probe + rat_plus_bridge + toast_tokenizer +
vocab_coreset + interval_pruner + parallel_probe + unit_distance);
c2 7/17 (bandit warm: ropd_rubric, sdpg_bandit, stepcode,
sense_composition, questbench, fpcg_selector + go — reqwest is a lib-level
dep only, the tests are offline analytics); c3 2/15 (bfcf_lfu_shard implies
bfcf_tree; speculative_generator default-on satisfies the cargo gate);
c4 3/10 (d2f_3sr_warm_start + flashar_consensus, dllm warm); c5 1/7
(shard_kv); c6 2/21 (bench_277's 5-feature gate reduced to 3 explicit —
temporal_deriv + collapse_aware_thinking are default-on; + causal head);
c7 6/15 (bomber family incl. bomber-agent + g_zero); 8a 1/30
(mech_attribution); 8b 2/31 (bomber-wasm); 8c 1/2 (percepta_compile);
c9 2/163 (the two ex-"compile-only giants"); c10 1/2+1 (the missed
maxsim straggler, caught by the final verification dry-run).

**The headline — the "compile-only giants" were misclassified by two
prior slices and heal like anything else**: bench_block_diagonal_goat (93)
+ bench_octopus_goat (70) carry `#[ignore = "pure measurement benchmark
(no assertions), slow in debug; run with --release --ignored"]` — the
attribute-value form a literal `#\[ignore\]` grep misses, which is how
they got labeled compile-only. A release `--ignored` run validates BOTH
in ~15 s total (9.2 s + 5.7 s) — the slice-6 bench_core_optimization
class. Standing rule confirmed again: the baseline run itself ("0 passed;
9 ignored") is the only reliable gate detector; grep on attribute FORMS
lies.

**Three pre-existing failures found at baseline, documented not fixed**
(the bench_104 precedent — test semantics are the author's call):
(1) bench_fixed_vs_procedural fails DETERMINISTICALLY (ratio 15.39 vs the
1.5× CV bound; identical pre/post + on re-run) — the CV-ratio statistic is
broken at mean≈0: fixed-map mean is +0.06 so CV explodes to 72.7 while the
actual StdDevs (fixed 4.51 vs procedural ~5.0) SATISFY the variance claim;
the file's own history shows prior assertion softening (a5a4278f).
(2) bench_064_futamura_evaluator::proof_futamura_specialized_has_fewer_
dimensions — specialized=300 dims vs universal=216 (expects REDUCTION);
dormant direction (percepta_compile not in full/default); ALSO
debug-unrunnable (15+ min for 3 transformer-eval tests → release 235 s —
the funcattn class; release is the practical baseline mode).
(3) bomber_wasm_bench is MODE-LOCKED: debug fails 3 WASM perf gates
(is_safe_action 198µs vs <10µs etc.), release 11/11 — the file's own
header documents `--release` (the ugc_g1b class; validated in release,
debug same-signature).

**Tooling lessons:** (1) rustfmt on bare test files needs `--edition 2024`
(let-chains in bench_stepcode_modelless.rs:644 — bare-file rustfmt defaults
to a pre-2024 edition and hard-fails; first recurrence in the sweep).
(2) CPU contention splits cycles: the combined bomber-wasm +
mech_attribution + percepta_compile compile timed out at 25 min under the
sibling's parallel load — split into 8a/8b/8c and each compiled fine
(mech_attribution was even warm already); heavy WASM/MILP feature sets get
their OWN cycle, backgrounded with nohup + log polling. (3) A sibling
rebased my 9 in-flight commits mid-session (origin/develop moved under me;
reflog shows the clean replay, c10 committed on top, push fast-forwarded) —
the shared-checkout reality; validations all ran post-rebase so the final
tree is what was validated. (4) `manual_is_multiple_of` is a rust-1.93
default-on lint with NO healer proposer — fixed manually at bench_maxsim_
rerank.rs:131 ((rng.next() % 4) == 0 → rng.next().is_multiple_of(4));
corpus-gap candidate (the Issue-060 drive-by class, now with a second
occurrence). (5) The vec-with-capacity array-literal divergence now has
FOUR recorded sites (2 prior + bench_120_renyi_efficiency:159 +
test_120_toast_renyi_goat:31 — the iterable is a multi-line array literal
with no name the emitter can reference for .len(); extending the proposer
to bind a temp first is the standing backlog item).

**Final state of the katgpt-rs root tests/ dir: 2 findings remain, both
the documented vec-with-capacity divergence class.**

**benches/ sweep landed 2026-08-18** (katgpt-rs `c70669f8`..`0eadbc54`, 10
commits rebased onto the sibling's TokenPath commit `62e01c1a`) — **the
root benches/ dir sweep is COMPLETE: 42 files / 406 edits + 4 manual
fixes** across 7
cycles (c1 default-compiled 14 files/132; c2 probe/sink/belief 6/48; c3
sudoku/halt/eigenbasis/ict/delta_mem 5/118; c4 cgsp/eventlog/drift/causal/
geometry 6/35; c5 quant 1/9; c6 flashmemory 1/18; c7 kimi family 9/46) +
1 residue-fix commit. **Benches are a different validation surface**:
harness=false `fn main()` binaries (validation class 9 — exit code +
normalized output diff, no test-result line). c1 got the full pre/post
protocol (14 baselines captured pre-heal; 12/14 normalized-identical, 2
timing-column-padding-only, ALL exit codes identical incl. the 2
pre-existing failures: fpcg G6 timing gate exit=1 under load + alg-prob
RLE cached-cumsum panic exit=101 — a REAL pre-existing correctness bug,
documented per bench_104 precedent). Gated cycles validated post-heal
(exit + verdict sanity + computation-untouched spot-checks) — justified
because the edits are output-identical-by-construction (format-arg
inlining: same spec, same values, positional→named mapping) +
compile-gated under each cycle's EXACT feature set.

**Two tooling discoveries that change future sweeps:** (1) **`--verify`
mode writes edits UNCONDITIONALLY** — the `--write` flag only gates the
non-verify path, so the initial "dry run" (`--fix --verify` WITHOUT
`--write`) landed the full 406-edit set to disk while reporting "fixed 0"
(a verify-mode counter quirk); worse, the default-features verify check
compiled gated benches EMPTY — the exact slice-6 hazard, caught here by
re-running explicit per-set `cargo check --benches --features F` for all
6 sets (all clean). Standing rule: NEVER trust `--fix --verify` without
`--write` as a dry run — it is not one. (2) **Residue scans must filter
by warning TEXT, not `--> path` alone** — the uninlined warning message
is "variables can be used directly", NOT the lint name, so the slice-7
residue-grep pattern (`^warning.*uninlined_format_args`) matched NOTHING
and produced a false "no residue"; a main-checkout scan was ALSO
corrupted mid-session by a sibling's broken uncommitted WIP (the kimi
set failed compiling lodestar.rs → no bench warnings emitted at all) —
the isolated-worktree re-scan is the only trustworthy surface when
siblings are active.

**Heal-created pedantic warnings (the Issue-027 class, 3 more sites):**
`.map(f).unwrap_or(false)` → `.map_or(false, f)` trips clippy 1.95's
`unnecessary_map_or` (suggests `is_some_and`) when the proposer's
provably-bool gate can't see through tuple-destructure closures
(salience ×2) or `matches!` macros (bench_011) — all 3 fixed manually;
the proposer gap (extend provably-bool to these shapes) is the standing
backlog. Also 1 missed-site fix: `format!` nested in an if-expression
argument to `println!` (procrustes:174 — the second-level macro-nesting
descent gap, same class riir-train slice 10 documented).

**Final state of katgpt-rs root benches/: 2 findings remain** — both the
named-width-arg divergence class (sudoku_speculate 813/814,
`println!("║ {:<width$} ║", hero, width = w)` — pre-existing, pedantic-only,
healer-silent by documented design). Sibling coordination: a live
TokenPath→TreePath naming WIP broke the shared checkout mid-sweep — all
c4-c7 validation moved to `git worktree add /tmp/heal8_wt` (the 9 kimi
benches ran there with `KIMI_K3_MODEL_DIR` pointing at the main repo's
1.58 GB model.safetensors — the worktrees lack data/ dirs); the final
push used the worktree-rebase-then-push dance (`git push origin
HEAD:develop` from a temp branch) because the shared checkout's dirty
sibling WIP blocked in-place rebase.

Remaining sweep territory: src/ ~376 findings in workspace crates + the
benches/ + examples/ surfaces of the member crates.

**Issue 031 (2026-08-17, RESOLVED same day + removed per noise-reduction
rule) — 3 healer defects from the katgpt-pruners 587-edit heal
validation (katgpt-rs `4b6675dc5`).** **H1 (CRITICAL)** — the
`comparison_to_empty` proposer rewrote `self.len() == 0` →
`self.is_empty()` INSIDE `fn is_empty` (3 sites: decision_explainer,
bfcp_region_cache, skill_catalog) → infinite recursion →
`STATUS_STACK_OVERFLOW` at test runtime — the first false-positive class
that produces a RUNTIME CRASH, structurally invisible to the `--verify`
compile gate (it compiles fine; only tests catch it; real clippy is
silent on the shape). Fix (`94cecbd`): enclosing-fn stack on the visitor
(`visit_item_fn`/`visit_impl_item_fn`/`visit_trait_item_fn`) + refuse
when receiver text is exactly `self` AND innermost fn is `is_empty`
(both forms: `len()==0` and `== ""`); non-self receivers inside
is_empty + self receivers elsewhere still fire (positive-control
fixtures); `clean_spans_fire_no_lint` extended with the exact production
shape. **H2** — `--domain` never scoped the `--fix` path: the proposer
loops ran BOTH `FIXABLE_RULES` (rust_perf) + `FIXABLE_LINTS` (clippy)
unconditionally (vec-with-capacity edits landed under `--domain clippy`).
Fix (`4a72c79`): `fix_domain_scopes()` — auto/None keeps both; explicit
domain runs only its own proposers; kernel runs neither; 4 bin tests
(`cargo test --bins` under the bin's required features). **H3** — the
verify baseline is default-features `cargo check`, so feature-gated
modules are NOT compile-gated (2 breaks shipped past verify: u8-const
`with_capacity` E0308 in monopoly + E0631 fn-path coercion in
roaring_membership — the exact Bench 031 classes escaping via module
gating). Fix (`f78dceb`): `manifest_nondefault_feature_count()` text
scan (multi-line default lists accumulate); verify warns + suggests
`--verify-args "--all-features"` when an owning package declares
non-default features and the caller passed no override — a warning, not
a default flip, because broken all-features sets exist (katgpt-dec `-D
warnings`). Gates: 519 default / 582 fix_verify / 4 bin tests / clippy 0
lib+bin under latent_retrieval,fix_verify. The katgpt-rs heal itself:
single_match_else 49→13 (remainder = documented deliberate skips), 8
fixable lints at 0, 3006/3006 lib tests.

**Plan 005 (2026-08-14) — latent retrieval + `cargo heal --suggest` CLI
(Phases 1–4 DONE).** `src/draft/` ships the span-level retrieval layer:
real `.rs` files → `AstChunker` spans → 8-D modelless embedding →
domain-centroid routing (annotation only) → fan-out retrieval against
per-domain latent indexes of the corpus → ranked rules + pattern
snippets. ZERO retrieval code reimplemented — composes riir-rag as-is
(`AstChunker` + `ModellessEmbedder` + `NeuronDbRetriever`) behind the
opt-in `latent_retrieval` feature. GOAT G1–G4 ALL PASS (Bench 018):
G1 100% (12/12 top-1 precision), G2 349µs/chunk (<1ms target, 2.9×
headroom), G3 104/104 lib tests, G4 `SuggestScratch` stable across 50
repeat scans. Two empirical deviations: fan-out retrieval instead of
gated routing (the 8-D centroids mis-routed fixtures — verdict is an
annotation) + multi-scale indexing/queries (corpus patterns are 1–5
line excerpts vs whole-item spans). The `cargo-heal` bin exposes it:
`cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/`
(smoke: 19 files, ~0.4s incl. index build). Phase 4 (2026-08-14) landed
the top-5 rust_perf `syn` AST proposers (`src/domains/rust_perf/proposers/`,
`propose_fix` dispatch + 36 fixture tests — see
[`.docs/11_domains/rust_perf.md`](.docs/11_domains/rust_perf.md) §Proposers).
Phase 6 T6.1 wired `--fix [--write] [--quiet]` with the span-preserving
edit splice (Bench 019, G1–G4 PASS). **T6.4 (2026-08-14)** migrated the
10 clippy_lints proposers to the same emitter — shared substrate in
`src/edit_emitter.rs` (feature-neutral, re-exported by both domains),
`propose_fix` signature preserved for `self_evolve`, `--fix` extended to
the structural clippy lints (`FIXABLE_LINTS`; needless_return,
redundant_field_names, manual_map, unnecessary_unwrap,
needless_range_loop, needless_lifetimes — 6) with bounded-fixpoint
application so nested fixes compose; the type-blind lints
(needless_borrow, useless_conversion, unnecessary_mut_passed, and
redundant_closure — demoted from FIXABLE by the T6.2 G5 real-file run
after it caught an E0631 fn-path coercion break) stay `self_evolve`-only.
The migration fixed four latent detection bugs in the old mutation-based
proposers (needless_lifetimes elision unsoundness incl. `FnMut(&'a S)`
positions + receivers; needless_range_loop `&v`-on-ref-param `&&Vec`
non-iterator + token-heuristic `v[v[i]]` false positive; unnecessary_unwrap
receiver mismatch) — see Plan 005 §Phase 6 T6.4 notes.

**Plan 005 Phases 7–14 closeout (2026-08-16) — the plan is COMPLETE as a
document: every task shipped, refuted, or defer-marked with a measured
reason.** No phase is left "not started".
- **Phase 7 CLOSED as a NEGATIVE RESULT** (commits `92b8436`+`76d5ad2`):
  every "the healer learns its way to better retrieval" mechanism measured
  flat-to-negative — domain-keyed EMA evolution **−1.0pp** vs the corpus
  centroid (T7.1, `tests/t71_evolved_directions.rs`; the class mean IS the
  cosine optimum, so online EMA is a noisier estimate of the same quantity);
  the router half rests on two stale premises (T7.2 — router is display-only
  + the 8-D directions are off the shipped TfCosine path); the
  trajectory-memory axis has **0/25 improvable held-out rankings and would
  break 19/25 correct ones** (T7.3, `tests/t73_memory_axis_headroom.rs`).
  What actually lifted retrieval was EVIDENCE, not learning (tokenizer
  +16.6pp, instantiation re-rank +12.0pp). Shipped from the phase: **T7.4** —
  `LatentFixMemory::to_json` emits a BLAKE3 commitment over the canonical
  trajectory serialization; `load_json_verified` reports
  `CommitmentStatus::{Absent, Verified, Mismatch}` (advisory not gate, per
  the `Corpus::thaw` precedent; pre-T7.4 files load as Absent).
- **Phase 9 CLOSED** — **T9.3 SHIPPED + GOAT PASS (Bench 028)**:
  `HealPipeline` candidate selection is now `SelectionMode::BetaPosterior`
  (DEFAULT) — the ε-quantile of Beta(1+S, 1+F) replaces the raw S/(S+F)
  rate, so a lucky 1/1 (Beta 0.224) no longer outranks a proven 18/20
  (Beta 0.729); selection regret **−8.4%** over the 1–5-attempt regime
  (−23.8% at 1 attempt; honest +2.9% crossover at 30 attempts documented at
  the enum variant — BetaPosterior is right for the thin-evidence regime
  the healer occupies, not universally). Unseen candidates now score 0.32,
  not 0.5. **T9.4 REFUTED four ways (Bench 029)**: `clr_vote` doesn't exist
  (ships as `clr_reliability_scores` + a `clr_vote` *parameter* on
  `compaction::gate`), a "draft" is a verbatim corpus row (no generative
  diversity to sample), the K=8 ensemble collapses (identity guard declines
  189/192, distinct drafts per producing case 1.00 of 8), and L2 accepts
  3/3 (no discriminative power). T9.1/T9.2/T9.2b DEFERRED —
  `DraftedFix.confidence` is written (`src/drafter.rs:246`) and read by
  nothing; Issue 023's findings (a cold calibrator emits a ZERO-WIDTH
  0%-coverage interval; coverage crosses 90% at exactly n=19 residuals)
  are recorded and binding for any reopen. Per-domain keying (not per-rule)
  is the amended default if T9.2 reopens.
- **Phases 10, 12, 14 DEFERRED after pre-flight (Bench 030)** — substrates
  verified REAL except `ConsolidationPipeline::can_freeze` (does not exist;
  4th stale substrate claim in the plan). Blockers are measured: T10.6's
  ≥10% gate vs **4.0pp total headroom** (24/25 held-out already correct at
  96.0% top-1); Phase 12 has no multi-step trajectories to classify (33/33
  single-step; `oscillation` unreachable by construction; `--fix --verify`
  already ships the escalation it wanted); T14.2's O(N)-scan premise is
  false (HashMap lookup 1.40× over a 100× size increase), T14.3 refuted by
  T7.3's measurement, T14.5 demands ≥5% from the same 4.0pp budget.
- **Phase 11 DEFERRED with a reopen condition (Bench 030)** — latency
  justification measured dead (ceiling 1.0655× vs the ≥1.5× gate,
  Issue 023); the quality gate targets an EMPTY set (L2 accepts 33/33 real
  proposer drafts — would-fail-L2 has zero members, G1 is a recall over
  nothing, G2 degenerates to trivially-true). The validator that does
  reject is `fix_verify`'s cargo check: first measured rejection rate
  **2/14 = 14.3%** (Bench 031), ground truth n=2. Reopen = re-specify G1/G2
  against `fix_verify` AND accumulate enough verify-tier revert data to fit
  + certify a rejection predictor. No `poincare_imagination` feature gate
  was created.
- **Phase 13 research half MET** — [Research 058](.research/058_convex_ast_vocab.md)
  (`kind + arity` colouring; compression gate re-specified to downstream
  retrieval top-1); experiment deferred on headroom (4.0pp).
- **Bench 027 collision resolved at closeout:** T3.5's G1-recovery bench
  (commit `0c5af53`, first) and Issue 015's verify-mode GOAT (commit
  `4b5c465`) were both allocated 027 same-day. Verify-mode renumbered
  **027 → 031** per the tie-breaker (fewer live references moves); T3.5
  keeps 027. Highwater now 31.

**Issue 016 (2026-08-16) — structural lint expansion 6→33 + `--verify`
tier 1→4.** 30 new lints distilled frequency-ranked from kornel's
100K-crate clippy run (v1.80–v1.83; `semicolon_if_nothing_returned`
41,601× — the most frequent structural lint in its top-10) + Li et al.'s
8.09M-warning crates.io study (top-5 lints = >80% of all warnings) ∩
clippy 1.93's `MachineApplicable` axis ∩ parse-only provability. 27
landed in `FIXABLE_LINTS` (semicolon_if_nothing_returned,
collapsible_if, collapsible_else_if, single_match, single_match_else,
redundant_pattern_matching, question_mark, range_plus_one,
uninlined_format_args, identity_op, needless_bool, needless_bool_assign,
match_bool, needless_match, redundant_static_lifetimes, double_parens,
unnecessary_lazy_evaluations, map_unwrap_or, filter_next, iter_nth_zero,
manual_unwrap_or, unused_unit, useless_format, deref_addrof,
wildcard_in_or_patterns, unnecessary_sort_by, iter_next_loop) + 3 in
`VERIFY_LINTS` (comparison_to_empty — 12 live `.len() == 0` hits in our
own workspace; while_let_on_iterator; single_char_pattern — type-blind
shapes the Issue 015 compile gate auto-reverts). Notable spec guards:
identity_op's IEEE-754 split (`+0.0` is NOT identity for `-0.0`, so
unsuffixed `+ 0` never fires; `- 0`, `* 1`, `/ 1` are float-safe),
manual_unwrap_or/unnecessary_lazy_evaluations purity gates (eager-vs-lazy
evaluation timing), uninlined_format_args' trait-bound identity +
abort-on-any-spec conservativeness, iter_next_loop as the one correctness
lint (the rewrite IS the intended semantics fix). Corpus 10→40;
per-lint proposers under `proposers/` with buggy/clean fixture tests;
corpus-lockstep test (`all_buggy_codes_fire_their_lint`) + suggest
gretrieval test (`every_lint_retrievable_via_suggest`, latent+BM25 top-5).
One pre-existing latent_matcher span test relaxed top-1 → top-3 (corpus
growth legitimately re-ranked one query — manual_unwrap_or's fixture is a
stronger BM25 token match). GOAT: 369 default + 383 latent_retrieval lib
tests, clippy 0 both states. See
[`.benchmarks/021`](.benchmarks/021_clippy_structural_expansion_goat.md).

**Issue 017 (2026-08-16) — retrieval-precision eval at scale.** Closes the
12-fixture gap: the Plan 005 G1 claim (100% top-1 on 12 fixtures) is **dead
at scale — measured 58.3% clippy top-1 on 48 spans / 40-rule corpus**
(nearest-neighbor crowding is real; concentrated in tiny-token patterns —
`identity_op` alone tops 5 misses). The eval
([`src/draft/retrieval_eval.rs`](src/draft/retrieval_eval.rs) + the two
fixture data files) ships **130 spans** (48 clippy variants real-clippy-
verified at authoring + proposer-verified standing, 40 perf variants, 30
kernel self-variant probes via the deterministic `mutate_shape` rename map,
12 REAL production clean excerpts), per-domain + blended top-1/3/5 recall,
a miss taxonomy, the clean false-positive profile, router-annotation
accuracy (53.4% 2-domain / 46.6% 3-domain — feeds Issue 020), and a
committed regression gate with floors = baseline − 1 span (tie-order
slack). One command: `cargo test --features latent_retrieval --lib
retrieval_precision_regression_gate -- --nocapture`; kernel-inclusive
(~54 s): `--features latent_retrieval,kernel_heal`. Baseline at Issue 017
land time: clippy 58.3/75.0/87.5, perf 87.5/95.0/97.5, kernel
self-variants 100/100/100 (crowding alarm if it drops). **SUPERSEDED by
Plan 005 T3.5 (Bench 027, 2026-08-16) — see the block below.** **Side-effect root-cause fix:** the fixture
pass exposed our `needless_bool_assign` proposer matching the assign-RHS
form (real clippy calls that `needless_bool`) — proposer rewritten to the
real branch-assign shape + corpus/row/fixtures updated. Known drift
recorded, not fixed: `comparison_to_empty` implements `.len() == 0` (real
`len_zero`); widening to `== ""` is a small follow-up. GOAT G1–G4 PASS
(370 default / 391 latent_retrieval / 526 kernel_heal, clippy 0 new
warnings). See [`.benchmarks/022`](.benchmarks/022_retrieval_precision_eval_at_scale.md).

**Issue 018 (2026-08-16) — `self_evolve` promoted to DEFAULT-ON with
graceful oracle degradation.** The mechanism/ oracle split: the memory
half (`LatentFixMemory` — every applied fix is a trajectory, every
compile-revert a definitive fail) needs NO oracle and now runs in EVERY
build (modules unconditional incl. `--no-default-features`); the EMA
learning half needs trustworthy verdicts and no-ops with **exactly ONE**
log line when the `clippy_verify` oracle is unavailable (the same shape as
Issue 015's verify-absent path). New `EvolveRecorder`
([`src/recorder.rs`](src/recorder.rs)) — the standalone consumer for the
`cargo heal --fix` path (the HealPipeline is untouched): `record_applied`/
`record_reviewed` (memory-only), `record_outcome` (stats always, EMA
gated), `disable_learning(reason)` (runtime oracle failure), plus
**persistence** — a CLI process is short-lived, so the memory persists as
a v1 JSON envelope (`RIIR_HEAL_MEMORY` override, else the platform cache
dir → `riir-clippy/memory.json`; merge-on-load keeps live stats;
corrupt/unknown-version → `InvalidData` + continue fresh, never aborts a
run). `--fix --write` records written edits; `--verify` records kept edits
memory-only (a rustc pass proves non-breakage, not lint resolution) +
reverted edits as `success=false` (sound verdicts — the clippy verifier
cannot soundly verify span edits standalone, a non-compiling crate reports
"lint absent" = false success). Opt-out: `RIIR_HEAL_SELF_EVOLVE=0|off|
false|no` — fully inert (no recording, no persistence, no log). The
`self_evolve` feature stays as a default-on back-compat alias
(benches'/examples' `required-features`). GOAT (Bench 023): G1 contract
tests both oracle states + the production `--fix` accumulation path; G2
real-bin e2e (1→3 trajectories across runs, one degradation line per run,
inert opt-out); G3 405 default / 419 clippy_verify / 652 combined lib tests
+ clippy 0 warnings in 5 states + `ci_feature_guard.sh` all-layers; G4
`cargo tree` diff EMPTY (dep surface unchanged). **Honest finding:** the
G7 demo re-run aborted G6 at 35/40 PRE-EXISTING (proven by baseline
worktree at `a532b7c` — 5 Issue-016 corpus entries' `buggy_code` didn't
trigger under real clippy; filed as Issue 022 — **RESOLVED same day, see
the Issue 022 section below: G7 now runs end-to-end at 40/40 truthfulness
+ 50%→100% healing**). See
[`.benchmarks/023`](.benchmarks/023_self_evolve_promotion_goat.md) +
[`.docs/10_self_evolve/evolve_recorder.md`](.docs/10_self_evolve/evolve_recorder.md).

**Issue 022 (2026-08-16, RESOLVED same day) — corpus real-clippy drift
(5 Issue-016 entries).** Found during the Issue 018 GOAT: the G6 demo
aborted at 35/40 — 5 corpus entries' `buggy_code` didn't trigger the named
lint under real cargo clippy **1.97** (the toolchain upgraded since the
Issue 016 authoring; proven pre-existing via a baseline worktree run at
`a532b7c`). Root causes, each verified by temp-crate + JSON-probe:
`iter_next_loop` + `while_let_on_iterator` code **didn't compile**
(missing `mut it` — `.next()` needs `&mut self`; invisible to parse-only
proposers); `.iter().nth(0)` on a slice routes to 1.97's more-specific
`iter_nth` (`iter_nth_zero` needs a non-slice iterator — fixed to the
IntoIter shape); `single_match_else` fires only when the else arm is a
STATEMENT BLOCK (a bare-default else routes to `manual_unwrap_or[_
default]` — 10 shapes probed; the Issue 017 fixture had the firing form all
along); `comparison_to_empty`'s real shape is `s == ""` (ours was
`.len() == 0` = real clippy's `len_zero` — the proposer was WIDENED to
detect both forms, the corpus moved to the truthful shape). **GOAT (Bench
024): G6 40/40; G7 restored — baseline 50.0% → evolved 100.0% (40/40,
+50.0pp) on real cargo clippy, now over the full 40-lint corpus** (the
strongest oracle-present evidence for Issue 018's promotion); 408/429 lib
tests + clippy 0 in 4 states + CI guard all-layers; retrieval floors
RE-PINNED per the Issue 017 contract (clippy 58.3→54.2 top-1, blended
3-domain 78.8→77.1; the −2 spans are the len-shaped
`comparison_to_empty` fixtures — truthfulness wins, a corpus that
misrepresents lint behavior poisons `self_evolve` learning). Bonus
root-cause fix: the eval's determinism test caught a score TIE on the
top-5 boundary flipping via backing-map iteration order → `canonical_rank`
(score desc via `total_cmp`, `rule_id` tie-break) makes the measurement
reproducible (a substrate-level riir-rag tie-break remains a documented
follow-up). Probe-harness lesson: command-line-requested lints render
HYPHENATED in human output (`clippy::needless-return`) — grep JSON
(`"code":"clippy::..."`) instead. See
[`.benchmarks/024`](.benchmarks/024_corpus_real_clippy_drift_goat.md).

**Issue 020 (2026-08-16, RESOLVED-KILLED) — DomainRouter spectral routing
POC.** The falsifiable question: does a router-local **token-frequency
spectral block** (k=8 DFT coefficient pairs over a top-128-token vocabulary,
two orderings + structural suffix + a full-TF ablation — 7 variants) reach
the ≥85% routing-accuracy kill-bar (Bench 022's bar: below that, gating
trades correctness for a fraction of the 349µs/chunk fan-out cost)?
**Verdict: KILL for gating** — best variant 70.3% (tf-cosine-disc, 3-domain)
/ 75.0% (dft8-global+struct, 2-domain — but mis-routes the documented
fixtures). The load-bearing finding (the ablation splits the failure):
**full-128 TF cosine ≈ DFT-k8 over the discriminative vocab (70.3% vs
69.5%) — the k-coefficient compression loses NOTHING; the lexical signal
itself is the ~70% ceiling**. Domain membership is semantic, not lexical
(a perf HashMap rule + a clippy HashMap rule share a token spectrum — the
issue's predicted mode, now measured). Only a semantic-signal embedder
(768-D class) could re-open gating; the lexical branch is CLOSED.
**Salvage shipped (annotation-only, no gating risk):** `LatentMatcher`'s
router now builds with `TfCosine { order: GlobalFreq }` — annotation
accuracy 53.4%→69.3% (2d) / 48.3%→68.6% (3d) at 0.96µs vs 3.31µs per span
(global over disc ordering: equal accuracy, insensitive to the
enabled-domain set). A standing floor (63.6% both states) joined the Issue
017 regression gate — annotation decay now re-measures + re-pins, never
silently weakens. The 7-variant measurement matrix lives on as the
reproducible negative-result artifact (`run_router_eval` + the
`router_spectral_poc_report` test — the `gemma2_directions` precedent).
Baseline reproduced Bench 022 exactly (53.4% 2-domain) — harness valid.
Determinism (BTreeMap vocab + precomputed twiddle tables, lookup-only
HashMap) + cost (0.96–4.06µs release; the gate is release-only, the `g2_*`
Debug pattern — debug pays ~10× on the transcendental paths) both PASS.
Note: the two original mis-route fixtures now route correctly even under
the baseline (current-corpus centroids) — overall accuracy, not those two
fixtures, is the live failure mode. 408/435/592 tests + clippy 0 in all
states. See
[`.benchmarks/025`](.benchmarks/025_router_spectral_poc_kill.md).

**Issue 021 (2026-08-16, RESOLVED) — evidential-status axis for
`LatentFixMemory`.** The retelling-bias defense (arXiv:2608.11195 /
Research 049): the old schema (success/fail u32s + boolean
`record_outcome`) could not distinguish a full-validator admission from a
syn-parse-only success, and iterated retrieval launders the latter into
the former. Shipped (Bench 026): `EvidenceTier` (`#[repr(u8)]`
Certified/Heuristic/Withdrawn) on `FixTrajectory` with
absorbing-Withdrawn transitions (the riir-chain slashing-monotonicity
pattern: 3 consecutive post-hoc failures → Withdrawn; ONLY
`(success && verified)` — a fresh full-validator pass — promotes or
exits); `consecutive_fails` streak driver (reset on success — stats
climb honestly while the tier decides); trust-weighted retrieval
(`RetrievalWeights.trust` default 0.2: Certified 1.0 / Heuristic 0.5 /
Withdrawn EXCLUDED from suggestions, still reachable via `iter()`/`get()`
for failure analysis); certification admission
`record_certified` wired into the Stage 2 tuning loop (verifier-passed
fixes now land in memory as Certified — previously the tuning loop only
fed the adaptive pool); `record_outcome` gains `verified` on
`LatentFixMemory` + `HealPipeline::report_outcome` (the demo passes
`vr.ran`; G5's simulated outcomes honestly pass `false`);
`EvolveRecorder` keeps its 4-arg CLI shape (`heal.rs` untouched — its
call sites are compile-gate reverts where the flag is behaviorally
irrelevant) + `record_outcome_verified` for future full-validator
callers. Pre-021 memory files decode as Heuristic (`#[serde(default)]`).
**The honest A/B finding:** the issue's original A/B ("two entries,
equal similarity, different tiers") is NOT constructible through the
real API — `record_outcome` matches by `(lint_name, fixed_code)`, so
entries sharing that pair are ONE stats bucket (the first test run
failed on exactly this, proving the gate is load-bearing). The
falsifiable A/B is the within-subject certification delta (exactly the
0.1 trust gap, tolerance 1e-5) + the cross-entry small-edge flip (a
Heuristic entry with STRICTLY higher latent, found by runtime search —
the hash embedder has no locality — still loses when the edge is under
the gap). Design constraint measured: at trust 0.2, a different-lint
Certified entry can never outrank a same-lint Heuristic one — trust
breaks ties, relevance dominates. Scope-of-validity (the stretch goal)
deferred per the issue's own defer clause. 415/429 lib + full CI guard
all-layers (668 combined) + clippy 0 in default/clippy_verify/
latent_retrieval states. See
[`.benchmarks/026`](.benchmarks/026_evidential_status_axis_goat.md).

**Issue 015 (2026-08-16, RESOLVED) — `--fix --verify` compile-gate mode.**
The mode (feature `fix_verify`, implies `latent_retrieval`; landed in
`2ace442`) closes the loop IN the tool: apply edits with loosened gates →
baseline `cargo check --all-targets --message-format=json` → write → map
rustc JSON errors to file/line → revert ONLY the offending edits
(line-matched span-level, whole-file fallback) → re-check → report. Under
`--verify`: the vec-with-capacity usize gate loosens to `untyped_paths`
(plain-path bounds attempted; E0308 backs the revert — the scope gate
STAYS ON: use-before-decl is a correctness issue, not a type question);
`VERIFY_LINTS` (`redundant_closure`, `comparison_to_empty`,
`while_let_on_iterator`, `single_char_pattern`) join the fix set; the
`fxhashmap` dep gate text-scans the owning crate manifest for `rustc-hash`
(hard-skip without cargo — the measured 422-edit instance class). **GOAT
(Bench 031):** real-repo run on the 6 katgpt-pruners files holding the
Bench 019 G5 skipped sites — 14 edits proposed / 12 kept / **2
auto-reverted on the real repo** (the u8 `BOARD_SIZE` E0308 + the
`|v| from_bool_vec(v)` E0631 coercion, exactly the G5 run-1 failure
classes), every named skipped site landed compile-confirmed usize
(`hydra_budget` n, `collapse_detector` N_TRACES, `rosetta` len,
`episode_pruner` min_len, `monopoly` `Strategy::COUNT`), **1245/1245 tests
pass + no new clippy warnings** under the 15-feature covering set; landed
as katgpt-rs `212ed7942`. The GOAT run itself surfaced + fixed a root
cause: relative path args made `owning_package_root` walk to the EMPTY
path (os error 123 from `current_dir("")`) — now normalized via
`std::path::absolute` with a regression test. The `#[ignore]`d e2e
fixture test (`fix_verify_e2e_bad_edits_reverted_good_survive`, spawns
real cargo) is the standing acceptance proof. Honest scope note: a
full-crate katgpt-pruners scan now proposes **293 edits / 104 files**
(Issue 016's expansion) — full-repo verify batches are a consumer
decision, not a tool default; this GOAT was deliberately file-scoped.
See [`.benchmarks/031`](.benchmarks/031_fix_verify_mode_goat.md).

## The `riir-clippy` skill (auto-continue idle loop)

`.agents/skills/riir-clippy/SKILL.md` (symlinked into `~/.agents/skills/`
for global discovery — canonical home is THIS repo, global is a symlink
never a copy) encodes the idle-time workflow for the whole workspace:
(1) fix open issues, (2) finish in-flight plans, (3) clippy sweep across
repos (heal-driven), (4) resolved-issue hygiene (doc then rm), (5) batch
mining + heal dry-run (the centerpiece — includes the proposer adaptation
loop: wrong suggestion → fix the proposer + fixture, never suppress),
(6) boundary-guard routine check, (7) substrate-first drift audit,
(8) benchmark regression sweep, (9) online unblock search, (10)
defer-marking sweep, (11) online perf search, (12) paper research via the
research skill, (13) doc-sync. One unit per cycle, commit + push each
unit. The cross-session resume state (mining queue snapshot: Batch 32
targets, sibling-blocked files, corpus count) lives IN the SKILL.md and
is updated in the same commit as each batch. Triggers: "continue",
"auto continue", "idle", "nothing to do", "next batch", "Batch 32+".
**Issue 019 RESOLVED 2026-08-16 (removed per noise-reduction rule; gap
analysis in git history — `git log -- .issues/019_*.md`):** T6's full
clean cycle completed via queue items 10–13 — item 10 defer-marking sweep
(riir-train 387 defer-marked with unblock conditions; riir-ai 529
queued-not-stalled note; riir-chain 026 §5.5 checkbox fix), item 11 online
perf search (8 rust_perf candidates → the Batch 48 pipeline, recorded in
the SKILL.md mining snapshot), item 12 paper research (Research 053
EvolveMem retrieval-config co-evolution, Gain verdict), item 13 doc-sync
(5 repos clean/deferred-by-rule + game-sdk vocabulary fix `40f0971`).
All 13 queue items have now been exercised ≥1× through the idle loop.
T7 stays blocked on sibling WIP (`ternary_deltanet_gpu_forward.rs`).

## Source secrecy & binary-only distribution (non-negotiable)

**Per [Research 003](../riir-ai/.research/003_Commercial_Open_Source_Strategy_Verdict.md):
"anything `riir-*` is internal. No exceptions."** This repo's source never
becomes public. The product surface (`cargo heal`) ships as **prebuilt
binaries only**, via GitHub Releases on a small PUBLIC distribution repo —
NOT on this repo (release assets on a private repo are not publicly
downloadable). Full design: [Proposal 003](../.proposals/003_binary_only_distribution.md).

Hard rules for any future session:

1. **Never publish source.** No `cargo publish` (already blocked:
   `publish = false` in `Cargo.toml`), no public mirrors, no source tarballs
   in release assets, no "open-source it for adoption" — that path is
   reserved for `katgpt-rs` (the public funnel) alone.
2. **Public surface = 3 tiny source-free repos**: `gist-rs/cargo-heal`
   (releases + `install.sh` + `install.ps1` + capabilities-only README),
   `gist-rs/homebrew-tap` (Formula), `gist-rs/scoop-bucket` (manifests).
   **Zero `.rs` files allowed in any of them** (CI guard). Do not create
   other public distribution surfaces without a proposal.
3. **Release binary hygiene**: build with `default + latent_retrieval +
   rust_perf` features ONLY (the `cargo-heal` bin's required-features);
   `strip = true` in the release profile; ship `SHA256SUMS` +
   `THIRD_PARTY_LICENSES.md` in every archive (MIT/Apache binary-distribution
   obligation); never ship dev/debug builds or `ternary_inference`.
4. **Windows users** get `install.ps1` + the scoop bucket; winget only after
   Authenticode signing (deferred). macOS/Linux get `install.sh` + the brew
   tap.

## Why this is a separate repo (not in riir-ai)

The code healer borrows the **drafter+pruner+corpus pattern** from
`riir-games-quest/quest_grammar` (TernaryDraftModel + SealQuestCorpus +
ConstraintPruner), but has **zero game-domain coupling**:

- No dependency on `riir-games*`, `riir-game-sdk`, or any game crate —
  from ANY feature combination
- No dependency on `riir-chain`; `riir-neuron-db` only transitive (via
  `riir-rag` under the opt-in `latent_retrieval` feature)
- Default build: only public dep is `katgpt-core` (ConstraintPruner trait
  + ternary SIMD matvec + `pick_domain` for kernel_opt's expert router,
  Plan 558). Two opt-in feature-gated arms consume non-game sibling
  crates where reimplementation would duplicate whole substrates:
  `ternary_inference` → `riir-engine` + `riir-gpu` (Bench 010 inference
  arm); `latent_retrieval` → `riir-rag` (Plan 005 retrieval substrate,
  `cargo heal --suggest`)
- The `TernaryDraftModel` pattern is reimplemented locally (~200 LOC), not
  imported cross-workspace

Putting it in riir-ai would violate the domain boundary: riir-ai is for **game
runtime + engine**, not developer tools. The prior rejection of extracting
riir-games from riir-ai (Proposal 017 era) was correct because riir-games has
heavy bidirectional coupling with riir-engine. Code healing has no such
coupling — it's a clean, self-contained developer tool.

### Plan 001 multi-domain architecture (2026-08-13)

Plan 001 generalized the framework from a clippy-only healer into a pluggable
multi-domain framework behind the [`HealDomain`](src/domain.rs) trait. The
three domains (clippy_lints, rust_perf, kernel_opt) share the same
L0→L1→L2→L3→L4 pipeline; each domain owns its corpus + validator. **Proposal
012's GPU kernel optimizer is absorbed as the `kernel_opt` domain** (Phase 3)
— the user's correction ("why it can't be clippy?") overrode Proposal 012's
"new repo" recommendation. The trait abstraction makes future extraction a
move, not a rewrite.

**Cross-domain auto-routing** — Plan 005 Phase 1/2 (2026-08-14) shipped
the unified `cargo heal --suggest` CLI with `--domain auto`, revising the
original "not in scope" stance: `auto` **fans out across all
feature-enabled domains** with the router verdict as a per-span
annotation, NOT a gate (empirical finding — the 8-D structural centroids
mis-routed fixtures, starving the correct domain; re-gating via tuned
centroids or the 768-D embedder is a future tuning option). Each domain is
still feature-selected; requesting a feature-off domain is a hard error.
Since Phase 6 the bin also runs **`--fix [--write] [--quiet]`** — the
span-preserving edit-splice emitter + the rust_perf + structural-clippy
proposers (T6.1/T6.4), hardened by the T6.2 G5 real-file run on
katgpt-rs/katgpt-pruners (Bench 019 §G5 addendum: 3 proposer bug classes
caught by the compile gate + fixed at root cause; 12 files / 20 edits
landed, compiled + 1233 tests passed; katgpt-rs `8c331cd9`). Kernel
proposers remain Phase 5 (deferred — concept-grade corpus).

### `kernel_opt`-specific layers (Phase 3)

The `kernel_opt` domain adds two layers on top of the shared pipeline:

1. **`KernelExpertRouter`** (`src/domains/kernel_opt/router.rs`) — wraps
   `katgpt_core::variable_rank_domain_expert::pick_domain<15, 8>` (Plan 558).
   Activity vector = kernel signature `[M, N, K, dtype, hw, is_attention, is_fusion, is_ir_transform]`
   (8-dim — Plan 004 extended from 5 to 8 dims by adding 3 operation-class
   one-hot axes, resolving the Plan 003 C+O unreachability); domain directions
   = per-category centroids (Plan 003: hand-crafted centroids wired to the
   88-rule corpus via `KERNEL_CATEGORIES`; **15/15 categories reachable**;
   mining via `CommittedFieldBlend` deferred). The `build_activity()` helper is
   the consumer-facing constructor. Modelless argmax — zero allocation.

2. **`KernelThermalRouter`** (`src/domains/kernel_opt/thermal.rs`) — the
   FlashAR/QGF thermal-tier pattern: Plasma→Hot→Warm→Cold→Freeze escalation.
   `ThermalTier` is `#[repr(u8)]` (1 byte, sync-efficient). The scaffold
   returns Plasma for all calls (modelless-first mandate); Warm/Cold/Freeze
   require 4090 CUDA + LoRA (deferred to riir-train).

## Sibling-Repo Layout

```
/git/riir-clippy          ← this repo (clippy-healing pipeline)
/git/katgpt-rs            ← katgpt-core (NON-OPTIONAL: ConstraintPruner trait + ternary matvec)
/git/riir-ai              ← OPTIONAL dep (feature-gated arms only: ternary_inference →
                          riir-engine + riir-gpu, Bench 010; latent_retrieval → riir-rag,
                          Plan 005. Default build: NOT a dep)
/git/riir-train           ← OPTIONAL dep (only for L4 LoRA fallback, deferred)
/git/riir-chain           ← NOT a dep
/git/riir-neuron-db       ← transitive only (via riir-rag under latent_retrieval)
```

Path deps in `Cargo.toml` assume the standard `/git/` sibling layout.

## Build Commands

```bash
# Default (clippy_lints domain + modelless path)
cargo check
cargo clippy --all-targets
cargo test --lib

# rust_perf domain (Plan 001 Phase 2)
cargo check --features rust_perf
cargo run --example perf_heal_demo --features rust_perf

# kernel_opt domain (Plan 001 Phase 3, 4090 DEFERRED)
cargo check --features kernel_heal
cargo run --example kernel_heal_demo --features kernel_heal

# Plan 005 latent retrieval + the cargo-heal CLI (opt-in; pulls riir-rag)
cargo check --features latent_retrieval
cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/

# With ruliology candidate search (L3)
cargo check --features ruliology_search

# All features (the pre-existing ternary_inference/riir-gpu failure was fixed
# in riir-ai commit 4bfe3b30 — relaxed attn_sub_on cfg gate to cubecl_runtime)
cargo check --all-features

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# Re-run GOAT benches (see .benchmarks/001_goat_consolidated.md for measured results)
cargo bench --bench bench_001_l1_matvec
cargo bench --bench bench_002_l2_pruner_syn
cargo run --example ruliology_search_demo --features ruliology_search --release
```

## Feature Flag Discipline

- `default` = `["clippy_lints"]`: clippy lint healing domain + L0 (corpus) + L1 (ternary drafter) + L2 (native rust-analyzer pruner)
- `clippy_lints` (default-on): the clippy lint healing domain.
- `clippy_verify` (opt-in): Phase 3 lint resolution verification via real `cargo clippy` re-run. Slow (~130ms/verify), requires cargo+clippy on PATH. Implies `clippy_lints`.
- `rust_perf` (default-on since 2026-08-13, Plan 001 Phase 2): rust performance healing domain. **62 rules** (Issue 014 / Bench 020 expanded 46→50; Batch 48 / Research 054 expanded 50→58 with the 8 online-search candidates; Batch 53 / Bench 038 expanded 58→62 with the 4 online-search candidates) from the `rust-optimize` skill checklist + nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs profiling guide (Research 032). Promoted after Bench 013 GOAT G1+G5 PASS; re-confirmed by Bench 015; Issue 012 (Bench 016) added 4 sorting/IO/copy rules; Issue 013 (Bench 017) added 3 data-structure-choice rules; Issue 014 (Bench 020) added `regex-hoist-lazylock` (5443-5675× — the corpus's largest win), `arc-str-payload` (5.4-11.4× + 42× bytes), + the 2 NotBenchable metric-mismatch rules (`tokio-guard-across-await`, `bounded-channel-backpressure`). Batch 48 (2026-08-16, Research 054) added 6 more ParetoDominant rules (`collect-append-to-extend` 8.5-12.8×, `vec-insert-front-deque` 7.8-115.3×, `vec-macro-zero-fill` 32-2505×, `unwrap-or-else-lazy` 1.9-2.3×, `hashset-insert-bool-return` 1.19-1.31×, `filter-map-fuse` 1.37-1.64×) + 2 honest annotations — `filter-count-to-any` ParetoDominated 0.82-0.90× (the folk-wisdom refutation: LLVM VECTORIZES the count loop while `any`'s early-exit forces scalar iteration — `any` wins only for ≲10%-position hits), `iter-copied-deref-map` Mixed (0.80× at 1M on sum folds). The `perf_bench_demo` gate runner accepts `PERF_BENCH_FILTER` (comma-separated rule-id substrings) for batch-scoped runs. Batch 53 (2026-08-17, Bench 038) added 4 online-search candidates — 2 ParetoDominant (`distance-sq-no-sqrt` 1.21-1.24×, `hashmap-with-capacity` 2.4-3.1× with allocs 12→1 — the rehash ladder re-SipHashes all entries per resize) + 2 honest annotations (`powi-vs-powf` all-Tie refutation: LLVM lowers powf(2.0) identically to powi(2) in release — the folk wisdom only holds for debug profiles / exponents ≥ 3; `soa-hot-field` latency-bound Mixed: a serial f32 add-reduction hides the layout win, SoA pays on throughput-bound field consumers — count/min-max/transform). **12 of 62** rules carry `context_notes` (10 verdict-class: 3 ParetoDominated + 7 regime-fragile/Mixed; 2 metric-mismatch-class — consumers should treat as context-dependent).
  **Batch 48 follow-up (2026-08-17): the three deferred proposers LANDED** - `hashset-insert-bool-return`, `collect-append-to-extend`, `vec-macro-zero-fill` are `--fix` eligible (`FIXABLE_RULES` 5->8) with fixture + clean-span + cross-domain gates; two syn facts learned: a type annotation wraps the let pattern (`Pat::Type` around the ident), and `LitFloat::base10_digits` keeps `.`/`e` separators (`"0.0"` digits are `"0.0"`); `filter-count-to-any` stays proposer-less BY DESIGN (ParetoDominated - a losing rewrite never gets a `--fix` path).
  **Batch 53 follow-up (2026-08-17 evening): the two Batch 53 ParetoDominant proposers LANDED** - `distance-sq-no-sqrt` (`(x*x+y*y).sqrt() < r` → `(x*x+y*y) < r * r`, either operand order; sum-of-squares receiver gate + pure-path/literal threshold gate; negative-threshold + 1-ulp boundary caveats documented) + `hashmap-with-capacity` (`HashMap::new()` + first-mention for-loop over a len-able iterable → `HashMap::with_capacity(iter.len())`; capacity-is-a-hint semantics make any pre-loop len safe). `FIXABLE_RULES` 8→10. `powi-vs-powf` stays proposer-less BY DESIGN (ALL-TIE 1.00x in release — zero gain gets no `--fix` path, the filter-count-to-any rationale applied to a tie) + `soa-hot-field` stays suggestion-only (architectural). Two implementation facts learned: field access `s.rows` parses as `Expr::Field` not `Expr::Path` (len-bases need a field-chain gate for `self.items` shapes), and the reversed-form emit order must be `t * t OP sos` (`t OP t * sos` parses as `t OP (t * sos)` — `*` binds tighter than comparisons; caught by the `reversed_operands_fire` fixture).
- `rust_perf_verify` (opt-in): `cargo test` re-run for perf rule verification. Slow (seconds), requires cargo on PATH. Implies `rust_perf`.
- `rust_perf_bench` (opt-in, Plan 001 Phase 2 GOAT gate): generated microbenchmarks at multiple input sizes → Pareto verdict + the **deterministic alloc-count axis** (Issue 014 / Bench 020: a counting `#[global_allocator]` in every generated crate reports exact per-call alloc count + bytes — bit-identical across repeat runs on 40/40 benchable rules; timing stays the verdict axis, alloc gates Tie-class candidates). **GOAT G1+G5+G-det PASS (Bench 013/015/016/017/020)** — 37/40 benchable rules Pareto-dominant (o1-lookup 570x, bsearch-sorted-slice 16-2709x, simd-chunk 3.9x, bool-to-bitpack 7-11x, branchless-conditional 10x, static-str 42x, vec-with-capacity 1.1-4.3x, format-concat-loop 42-95x, mutex-to-atomic 2.5x, f32-counter-to-int 7.5-11x, entry-vs-contains-insert 1.5x, loop-fusion 1.5x, collect-avoidance 2.3-3.0x, dyn-to-enum 2.5-2.9x, prefix-sum-2d 34-30000x, ok-or-else 97x, swap-remove 3-7927x, chunks-exact 13-16x, fxhashmap 3.3-5.8x, retain-inplace 1.3-2.0x, mem-take 1.9-2.6x, avoid-clone 1.3-3.0x, box-str-immutable 1.1-1.3x, box-avoid 9.6-42x, smallvec-stack 27-75x, buffered-io 99-132x, log-write-direct 1.9-2.0x, sort-unstable 1.08-1.25x, bufwriter-batched 2.48-21.57x, vec-remove-front-to-vecdeque 3.65-1270x, btreemap-sorted-iteration 3.73-6.91x, cow-conditional-alloc 1.32-1.49x, **regex-hoist-lazylock 5443-5675x** (allocs 1.3M→0), **arc-str-payload 5.38-11.38x** (bytes 42×)); 2/40 honestly dominated (precompute-table: compiler optimizes popcount to POPCNT in release; papaya-lockfree: benefit needs multi-threaded contention); 1/40 Mixed (sort-by-key). Closes Proposal 001 caveat 3. Implies `rust_perf`.
- `kernel_heal` (opt-in, Plan 001 Phase 3): GPU kernel optimization healing domain. Absorbs Proposal 012. Enables `katgpt-core/variable_rank_domain_expert` for the `pick_domain` router (Plan 558). 15 category experts (Plan 003), 8-dim activity vector with 15/15 reachable (Plan 004), routed heal pipeline composed in `kernel_heal_demo` example. 4090 LoRA + real popcorn corpus DEFERRED.
- `kernel_verify` (opt-in): CubeCL compile check. **STUB** — delegates to shared `RustAnalyzerPruner`; full numeric compare deferred (Proposal 012 §Phase 0). Implies `kernel_heal`.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (Bench 572) without a code-level dep. Runs once per lint invocation (~400µs/10 candidates), not per-tick. Implies `clippy_lints`.
- `latent_retrieval` (opt-in, Plan 005 Phase 1): real-file span-level latent retrieval — the `--suggest` MVP. Indexes every enabled domain's corpus via riir-rag (`RuleIndex`: `ShardIndex` Clifford-wedge KNN + BM25, multi-scale), routes spans via corpus-derived domain centroids (`DomainRouter`, annotation only), retrieves ranked rules (`LatentMatcher`, fan-out across enabled domains). The suggest path itself has ZERO proposer code — suggestions are the corpus pattern texts; the proposers live in each domain's `proposers/` module (rust_perf Phase 4, clippy T6.4) + the shared `src/edit_emitter.rs` substrate, gated by the proposers' own Cargo features. Gates `dep:riir-rag` (heavy tree: riir-rag + transitively riir-neuron-db — the default build stays light). Powers the `cargo-heal` bin (`--suggest` / `--fix` / `--domain auto|clippy|perf|kernel` / `--top-k`). **GOAT G1–G4 ALL PASS (Bench 018, 2026-08-14):** G1 100% (12/12 top-1, target ≥80%), G2 349µs/chunk (<1ms, 2.9× headroom), G3 104 lib tests, G4 `SuggestScratch` stable across 50 repeat scans. Known limits: fan-out routing (no gate), no minimum-score threshold (every span suggests), 12-item G1 fixture.
- `self_evolve` (DEFAULT-ON since Issue 018, 2026-08-16; Bench 023): latent-first self-evolution loop — `LatentFixMemory` (3-axis retrieval over fix trajectories) + `LatentSkillEvolution` (EMA direction-vector update) + `EvolveRecorder` (the oracle-split standalone recorder for the `cargo heal --fix` path). GOAT G1-G5 PASS (G5: 70pp healing rate improvement in controlled fixture). **G6/G7 validated against real cargo clippy feedback (Issue 008): G6 10/10 corpus truthfulness PASS, G7 50.0pp improvement (50% → 100%) PASS** (re-validated 2026-08-16 post-promotion at the 40-lint corpus scale after the Issue 022 corpus-truthfulness fix — G6 40/40, G7 50%→100%, Bench 024). The modules are UNCONDITIONAL — the memory half runs wherever fixes are applied (+ persists across runs via the v1 JSON envelope); the EMA learning half no-ops with one log line without the `clippy_verify` oracle. The feature flag is a default-on back-compat alias (required-features lists). Opt-out: `RIIR_HEAL_SELF_EVOLVE=0`.
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc)
- `l4_lora` (opt-in, riir-train Plan 336 T5.2/T5.3): the **L4 generative fallback seam**. Ships the composition SEAM ONLY — trait + pipeline wiring, no model, no GPU, no heavy deps — and is **inert unless a fallback is installed** via `HealPipeline::with_l4_fallback`. Surface: `pub trait L4Fallback` (`src/l4_fallback.rs:56`), `HealPipeline::with_l4_fallback` / `has_l4_fallback` (`src/self_evolve.rs`), `impl L4Fallback for DaemonL4` (`src/l4_daemon.rs:186`, the `bonsai_clippy_l4_eval --serve` transport client). Contract enforced by `tests/g4_l4_composition.rs` (**13 tests**).
  **When L4 is consulted — the reachability contract.** L4 sees *only* misses, never a working modelless answer:
  1. **miss #1** — `heal()` has no candidate at all.
  2. **miss #2** — the best candidate does not parse.
  3. **miss #3 (relevance, Issue 030 — resolved, commit `d8440f6`)** — the best candidate is a memory trajectory for a rule the corpus has never covered. Before this gate the serve/decline decision was `parses` alone, i.e. a **syntactic test used as a relevance proxy**; off-distribution it served valid-but-unrelated Rust (e.g. `ptr_arg` answered with `fn f() -> String { "done".to_string() }`) and **suppressed L4 entirely** — measured miss rate 0/29 (Bench 037). With the gate: out-of-corpus 9/9 decline, in-distribution unchanged, addressable gap **9/29** (Bench 037).
  A parsing **and relevant** modelless fix is never replaced by L4 (`a_parsing_modelless_fix_is_never_replaced_by_l4`), and Stage-2 hits return before the gate, so the Issue 010 path cannot regress (`stage2_hit_never_reaches_l4`).
  **NOT promoted, and promotion is not close:** riir-train Plan 336 G1 is a measured **FAIL (0/60**, tied with the frozen-backbone control, Bench 465) and T2.7's re-gate repeated **0/60 DEGENERATE** (Bench 467); G2 is also a measured FAIL (Bench 463, ~191 s/fix). The bar is now concrete: on the 9 reachable cases an adapter must beat **decline**, and declining is a *correct* answer — a wrong-but-parsing L4 fix is worse than the `None` the pipeline now returns.
- `clippy_lora` (opt-in): **DEAD PLACEHOLDER — superseded by `l4_lora`.** Declared at `Cargo.toml:168` but referenced by no code anywhere in the tree (verified 2026-08-17). The L4 work ships under `l4_lora`. Kept only to avoid breaking any external `--features clippy_lora` invocation; do not add code behind it.
- `ternary_inference` (opt-in, Bench 010): katgpt ternary generation on clippy prompts. The pre-existing compile error (`attn_sub_on` not found in riir-gpu) was **FIXED** in riir-ai commit `4bfe3b30` — the cfg gate on `attn_sub_on` + `ATTN_SUBSTAGE_MASK` + `ATTN_SUBSTAGE_ALL` was relaxed from `all(cubecl_runtime, ternary_gemm_batched)` to just `cubecl_runtime` (behavior-preserving: the mask defaults to all-on). `--all-features` CI layer 4 now passes clean.

The modelless path (L0-L3) is PRIMARY. LoRA (L4) is the conservative last resort,
and "last resort" is enforced in code, not by convention — see the three-condition
reachability contract under `l4_lora` above. The load-bearing lesson from Issue 030
generalizes beyond L4: **a syntactic check (`parses`) is not a relevance check.** Any
future stage that decides "did the modelless path succeed?" must test whether the
answer pertains to the input, or it will silently serve confident nonsense
off-distribution and starve whatever fallback sits behind it.
The self-evolution layer (Issue 004) is additive — it composes on top of L0-L2
without modifying the frozen pipeline. `rust_perf` was promoted to default-on
2026-08-13 (Bench 013 GOAT G1+G5 PASS) with the 2 ParetoDominated rules
carrying `context_notes` annotations ("demote loser" per global rule). The
annotations now propagate end-to-end through the drafter via a non-breaking
default method `HealEntry::context_notes() -> Option<&str> { None }` →
`DraftedFix::context_notes` (2026-08-13 post-promotion wiring). The drafter
surfaces the annotation; the consumer decides whether their context matches.
`kernel_opt` is NOT promoted to default-on until its GOAT gate (real corpus +
full numeric validator) passes.

## Documentation Shape

This repo ships a **11-folder numbered `.docs/` book** (opened 2026-08-11,
Issue 007; the 11th folder was added for the Plan 001 domains), matching the `riir-ai/.docs/` / `riir-neuron-db/.docs/` /
`riir-game-sdk/.docs/` format: numbered folders for sort order, unnumbered
`.md` files inside, each folder has a `README.md` index with a purpose
blockquote + file table + "See also" cross-refs.

| Folder | Scope |
|---|---|
| `01_orientation/` | Crate role, sibling layout, boundary rationale, multi-domain architecture (Plan 001), build commands, numbering |
| `02_corpus/` | L0 rule distillation: `Corpus<D>` (generic) + BLAKE3 commitment + per-domain seed corpora |
| `03_drafter/` | L1 ternary SIMD matvec: `TernaryDrafter` (generic over `E: HealEntry`) + BLAKE3 `embedder` + Plan 005 latent retrieval (`RuleIndex`/`DomainRouter`/`LatentMatcher`, `--suggest` MVP, Bench 018 GOAT G1–G4) |
| `04_pruner/` | L2 syn validator: `RustAnalyzerPruner` + `ValidationResult` + honesty contract |
| `05_clippy_verify/` | clippy lint resolution: `ClippyVerifier` (opt-in `clippy_verify`) |
| `06_ruliology/` | L3 Pareto-front: `RuliologySearch` + `pareto_front` + `ast_delta_size` (opt-in) |
| `07_examples/` | The 8 in-tree examples (`heal_demo`, `perf_heal_demo`, `perf_bench_demo`, `kernel_heal_demo`, `clippy_verify_demo`, `ruliology_search_demo`, `self_evolve_real_world_demo`, `stage2_tuning_demo`) + the `cargo-heal` bin (Plan 005 `--suggest` CLI) |
| `08_benchmarks/` | GOAT gate methodology + measured results |
| `09_lessons/` | Canonical patterns: borrow-without-dep, modelless-first, freeze/thaw, honest POC caveats, sigmoid-not-softmax |
| `10_self_evolve/` | Issue 004 + Issue 008: latent-first self-evolution loop (`HealPipeline` + `LatentFixMemory` + `LatentSkillEvolution`, opt-in `self_evolve`; G1–G7 ALL PASS) |
| `11_domains/` | **Plan 001 Phase 2+3**: multi-domain architecture — `rust_perf` (Phase 2) + `kernel_opt` (Phase 3, absorbs Proposal 012) |

The top-level `.docs/README.md` is the index. `README.md` (repo root) remains
the build surface (Status + Quick Start + Usage); `AGENTS.md` (this file)
remains the authoritative repo-local context.

For `.docs/`: files inside numbered folders have **no number prefix**.
Filenames are bare slugs (e.g. `crate_role.md`). The folder number provides
sort order; the filename provides identity. Add a new doc by dropping
`slug.md` in the right group folder and adding one line to that folder's
`README.md` index table.

## Numbering Discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and never
reused**. Before creating a new `.issues/` file, read `.issues/.highwater`, use
`value + 1`, write back. Same for `.plans/`, `.docs/`, `.benchmarks/`, `.research/`.

### Research-note shape (convention, adopted 2026-08-16)

`.research/` notes are **born-complete distillation records**, not task
trackers — no `- [ ]` checkboxes (pending work lives in `.issues/`, the
SKILL.md mining queue, or the plan the note feeds). Two header rules:

1. Every new note carries a `**Status:**` line (not `**Outcome:**` — legacy
   notes using it are grandfathered) from a small vocabulary: `RECORD`
   (batch landed, closed) · `DISTILLED — pending owner decision` (has an
   actionable unimplemented delta) · `NEGATIVE` (measured refutation).
2. A note whose delta gets filed/implemented later gets a one-line status
   update pointing at the issue/commit — the note itself stays a record.

Rationale: 36/58 historical notes have no Status line, which is benign only
because they are closed records; the two genuinely-pending notes (053, 058)
DO carry explicit Status lines. The vocabulary makes that distinction
machine-skimmable for future sessions.

## Branch

`develop` is the working branch. Commit directly on `develop` per the global rule.

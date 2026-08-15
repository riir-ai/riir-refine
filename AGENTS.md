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
   issues. 46 rules distilled from the `rust-optimize` skill checklist +
   nnethercote/perf-book + rustc-hash/bevy real-world patterns (complexity,
   allocation, layout, concurrency, simd, arithmetic, iterators, dispatch,
   caching, io, bevy, wasm). The **perf-bench
   Pareto validator** (Bench 013 promotion gate + Bench 015 Phase 4
   finalization + Bench 016 Issue 012 expansion + Bench 017 Issue 013 data-structure-choice
   expansion, `rust_perf_bench` feature) is the promotion gate — GOAT G1+G5 PASS.
   6 of 38 benchable rules (`precompute-table`, `papaya-lockfree`,
   `dedupe-hashset-vs-vec`, `reuse-allocation-clear`, `sort-by-key`,
   `copy-from-slice`) carry `context_notes` annotations recording the
   bench finding (Bench 015 + Bench 016: 2 ParetoDominated + 4 regime-fragile/Mixed in the
   default config — consumers should treat annotated rules as
   context-dependent).
3. **`kernel_opt`** (opt-in, Phase 3) — heals GPU kernel optimization. 209 rules
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
See `.issues/019` for the gap analysis that shaped it.

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
- `rust_perf` (default-on since 2026-08-13, Plan 001 Phase 2): rust performance healing domain. 46 rules from the `rust-optimize` skill checklist + nnethercote/perf-book + rustc-hash/bevy real-world patterns. Promoted after Bench 013 GOAT G1+G5 PASS; re-confirmed on the expanded corpus by Bench 015; Issue 012 (Bench 016) added 4 more rules (sort-unstable, sort-by-key, bufwriter-batched, copy-from-slice) filling sorting/IO-write/bulk-copy gaps; Issue 013 (Bench 017) added 3 data-structure-choice rules (vec-remove-front-to-vecdeque, btreemap-sorted-iteration, cow-conditional-alloc) filling the collections-choice gap. 6 of 38 benchable rules (`precompute-table`, `papaya-lockfree`, `dedupe-hashset-vs-vec`, `reuse-allocation-clear`, `sort-by-key`, `copy-from-slice`) carry `context_notes` annotations (Bench 015 + Bench 016: 2 ParetoDominated + 4 regime-fragile/Mixed in default config — consumers should treat as context-dependent).
- `rust_perf_verify` (opt-in): `cargo test` re-run for perf rule verification. Slow (seconds), requires cargo on PATH. Implies `rust_perf`.
- `rust_perf_bench` (opt-in, Plan 001 Phase 2 GOAT gate): generated microbenchmarks at multiple input sizes → Pareto verdict. **GOAT G1+G5 PASS (Bench 013 promotion + Bench 015 Phase 4 finalization + Bench 016 Issue 012 expansion + Bench 017 Issue 013 data-structure-choice expansion)** — 34/38 benchable rules Pareto-dominant (o1-lookup 570x, bsearch-sorted-slice 16-2709x, simd-chunk 3.9x, bool-to-bitpack 7-11x, branchless-conditional 10x, static-str 42x, vec-with-capacity 1.1-4.3x, format-concat-loop 42-95x, mutex-to-atomic 2.5x, f32-counter-to-int 7.5-11x, entry-vs-contains-insert 1.5x, loop-fusion 1.5x, collect-avoidance 2.3-3.0x, dyn-to-enum 2.5-2.9x, prefix-sum-2d 34-30000x, ok-or-else 97x, swap-remove 3-7927x, chunks-exact 13-16x, fxhashmap 3.3-5.8x, retain-inplace 1.3-2.0x, mem-take 1.9-2.6x, avoid-clone 1.3-3.0x, box-str-immutable 1.1-1.3x, box-avoid 9.6-42x, smallvec-stack 27-75x, buffered-io 99-132x, log-write-direct 1.9-2.0x, sort-unstable 1.08-1.25x, bufwriter-batched 2.48-21.57x, vec-remove-front-to-vecdeque 3.65-1270x, btreemap-sorted-iteration 3.73-6.91x, cow-conditional-alloc 1.32-1.49x, dedupe-hashset-vs-vec 0.92-7.5x regime-fragile, reuse-allocation-clear 0.87-4.0x regime-fragile, sort-by-key 0.99-1.02x regime-fragile, copy-from-slice 0.95-1.26x regime-fragile); 2/38 honestly dominated (precompute-table: compiler optimizes popcount to POPCNT in release; papaya-lockfree: benefit needs multi-threaded contention). Closes Proposal 001 caveat 3. Implies `rust_perf`.
- `kernel_heal` (opt-in, Plan 001 Phase 3): GPU kernel optimization healing domain. Absorbs Proposal 012. Enables `katgpt-core/variable_rank_domain_expert` for the `pick_domain` router (Plan 558). 15 category experts (Plan 003), 8-dim activity vector with 15/15 reachable (Plan 004), routed heal pipeline composed in `kernel_heal_demo` example. 4090 LoRA + real popcorn corpus DEFERRED.
- `kernel_verify` (opt-in): CubeCL compile check. **STUB** — delegates to shared `RustAnalyzerPruner`; full numeric compare deferred (Proposal 012 §Phase 0). Implies `kernel_heal`.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (Bench 572) without a code-level dep. Runs once per lint invocation (~400µs/10 candidates), not per-tick. Implies `clippy_lints`.
- `latent_retrieval` (opt-in, Plan 005 Phase 1): real-file span-level latent retrieval — the `--suggest` MVP. Indexes every enabled domain's corpus via riir-rag (`RuleIndex`: `ShardIndex` Clifford-wedge KNN + BM25, multi-scale), routes spans via corpus-derived domain centroids (`DomainRouter`, annotation only), retrieves ranked rules (`LatentMatcher`, fan-out across enabled domains). The suggest path itself has ZERO proposer code — suggestions are the corpus pattern texts; the proposers live in each domain's `proposers/` module (rust_perf Phase 4, clippy T6.4) + the shared `src/edit_emitter.rs` substrate, gated by the proposers' own Cargo features. Gates `dep:riir-rag` (heavy tree: riir-rag + transitively riir-neuron-db — the default build stays light). Powers the `cargo-heal` bin (`--suggest` / `--fix` / `--domain auto|clippy|perf|kernel` / `--top-k`). **GOAT G1–G4 ALL PASS (Bench 018, 2026-08-14):** G1 100% (12/12 top-1, target ≥80%), G2 349µs/chunk (<1ms, 2.9× headroom), G3 104 lib tests, G4 `SuggestScratch` stable across 50 repeat scans. Known limits: fan-out routing (no gate), no minimum-score threshold (every span suggests), 12-item G1 fixture.
- `self_evolve` (opt-in, Issue 004 + Issue 008): latent-first self-evolution loop — `LatentFixMemory` (3-axis retrieval over fix trajectories) + `LatentSkillEvolution` (EMA direction-vector update). GOAT G1-G5 ALL PASS (G5: 70pp healing rate improvement in controlled fixture). **G6/G7 validated against real cargo clippy feedback (Issue 008): G6 10/10 corpus truthfulness PASS, G7 50.0pp improvement (50% → 100%) PASS.** Borrows `dot+sigmoid` from riir-agents + 3-axis retrieval from riir-rag, reimplemented locally (~730 LOC, boundary-compliant). Stays opt-in (deployment-environment constraint: `clippy_verify` is itself opt-in). The mechanism is validated; the opt-in gate is not a correctness concern.
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc)
- `clippy_lora` (opt-in, deferred): L4 adaptive LoRA fallback (blocked on riir-train integration)
- `ternary_inference` (opt-in, Bench 010): katgpt ternary generation on clippy prompts. The pre-existing compile error (`attn_sub_on` not found in riir-gpu) was **FIXED** in riir-ai commit `4bfe3b30` — the cfg gate on `attn_sub_on` + `ATTN_SUBSTAGE_MASK` + `ATTN_SUBSTAGE_ALL` was relaxed from `all(cubecl_runtime, ternary_gemm_batched)` to just `cubecl_runtime` (behavior-preserving: the mask defaults to all-on). `--all-features` CI layer 4 now passes clean.

The modelless path (L0-L3) is PRIMARY. LoRA (L4) is the conservative last resort.
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

## Branch

`develop` is the working branch. Commit directly on `develop` per the global rule.

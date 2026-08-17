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
   issues. **58 rules** distilled from the `rust-optimize` skill checklist +
   nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs
   profiling guide (Research 032) (complexity,
   allocation, layout, concurrency, simd, arithmetic, iterators, dispatch,
   caching, io, bevy, wasm). The **perf-bench
   Pareto validator** (Bench 013 promotion gate + Bench 015 Phase 4
   finalization + Bench 016 Issue 012 expansion + Bench 017 Issue 013 data-structure-choice
   expansion + Bench 020 Issue 014 alloc-axis expansion, `rust_perf_bench` feature)
   is the promotion gate — GOAT G1+G5+G-det PASS (the alloc-count axis: exact
   integers, bit-identical across 3 repeat runs on 48/48 benchable rules).
   10 of 58 rules carry `context_notes` annotations recording the
   bench finding (6 verdict-class — Bench 015/016: 2 ParetoDominated + 4
   regime-fragile/Mixed in the default config; 2 metric-mismatch-class — Issue 014:
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
would be displaced — our tool is SAFER than clippy there). **Open (kept
in the issue's task list, tracked by the T-numbers):** T3 — the
`None`/`Err(_)` second-arm single_match_else class (~30 findings in
katgpt-speculative; type-informed wildcard detection — needs a probe
matrix before any parse-only implementation); T4 — `uninlined_format_args`
~60 miss shapes (diagnose from ground truth, extend only the verified
subset); T5 — **DONE same day** (benches/ swept, katgpt-rs `4ccb2f280`:
bench_424:97 range_plus_one + bench_136:208 uninlined — range_plus_one
now ZERO findings across all targets).
Note: syn 2.0.119's `PatLit.lit` holds a `Lit` (not `Box<Expr>`) whose
token preserves a leading `-` — the negative-literal exclusion checks the
token text. Gates: 30 range/single_match tests / 487 default lib / 539
latent_retrieval lib / retrieval floors unchanged / clippy 0 both states.

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
- `rust_perf` (default-on since 2026-08-13, Plan 001 Phase 2): rust performance healing domain. **58 rules** (Issue 014 / Bench 020 expanded 46→50; Batch 48 / Research 054 expanded 50→58 with the 8 online-search candidates) from the `rust-optimize` skill checklist + nnethercote/perf-book + rustc-hash/bevy real-world patterns + the hotpath-rs profiling guide (Research 032). Promoted after Bench 013 GOAT G1+G5 PASS; re-confirmed by Bench 015; Issue 012 (Bench 016) added 4 sorting/IO/copy rules; Issue 013 (Bench 017) added 3 data-structure-choice rules; Issue 014 (Bench 020) added `regex-hoist-lazylock` (5443-5675× — the corpus's largest win), `arc-str-payload` (5.4-11.4× + 42× bytes), + the 2 NotBenchable metric-mismatch rules (`tokio-guard-across-await`, `bounded-channel-backpressure`). Batch 48 (2026-08-16, Research 054) added 6 more ParetoDominant rules (`collect-append-to-extend` 8.5-12.8×, `vec-insert-front-deque` 7.8-115.3×, `vec-macro-zero-fill` 32-2505×, `unwrap-or-else-lazy` 1.9-2.3×, `hashset-insert-bool-return` 1.19-1.31×, `filter-map-fuse` 1.37-1.64×) + 2 honest annotations — `filter-count-to-any` ParetoDominated 0.82-0.90× (the folk-wisdom refutation: LLVM VECTORIZES the count loop while `any`'s early-exit forces scalar iteration — `any` wins only for ≲10%-position hits), `iter-copied-deref-map` Mixed (0.80× at 1M on sum folds). The `perf_bench_demo` gate runner accepts `PERF_BENCH_FILTER` (comma-separated rule-id substrings) for batch-scoped runs. **10 of 58** rules carry `context_notes` (8 verdict-class: 3 ParetoDominated + 5 regime-fragile/Mixed; 2 metric-mismatch-class — consumers should treat as context-dependent).
  **Batch 48 follow-up (2026-08-17): the three deferred proposers LANDED** - `hashset-insert-bool-return`, `collect-append-to-extend`, `vec-macro-zero-fill` are `--fix` eligible (`FIXABLE_RULES` 5->8) with fixture + clean-span + cross-domain gates; two syn facts learned: a type annotation wraps the let pattern (`Pat::Type` around the ident), and `LitFloat::base10_digits` keeps `.`/`e` separators (`"0.0"` digits are `"0.0"`); `filter-count-to-any` stays proposer-less BY DESIGN (ParetoDominated - a losing rewrite never gets a `--fix` path).
- `rust_perf_verify` (opt-in): `cargo test` re-run for perf rule verification. Slow (seconds), requires cargo on PATH. Implies `rust_perf`.
- `rust_perf_bench` (opt-in, Plan 001 Phase 2 GOAT gate): generated microbenchmarks at multiple input sizes → Pareto verdict + the **deterministic alloc-count axis** (Issue 014 / Bench 020: a counting `#[global_allocator]` in every generated crate reports exact per-call alloc count + bytes — bit-identical across repeat runs on 40/40 benchable rules; timing stays the verdict axis, alloc gates Tie-class candidates). **GOAT G1+G5+G-det PASS (Bench 013/015/016/017/020)** — 37/40 benchable rules Pareto-dominant (o1-lookup 570x, bsearch-sorted-slice 16-2709x, simd-chunk 3.9x, bool-to-bitpack 7-11x, branchless-conditional 10x, static-str 42x, vec-with-capacity 1.1-4.3x, format-concat-loop 42-95x, mutex-to-atomic 2.5x, f32-counter-to-int 7.5-11x, entry-vs-contains-insert 1.5x, loop-fusion 1.5x, collect-avoidance 2.3-3.0x, dyn-to-enum 2.5-2.9x, prefix-sum-2d 34-30000x, ok-or-else 97x, swap-remove 3-7927x, chunks-exact 13-16x, fxhashmap 3.3-5.8x, retain-inplace 1.3-2.0x, mem-take 1.9-2.6x, avoid-clone 1.3-3.0x, box-str-immutable 1.1-1.3x, box-avoid 9.6-42x, smallvec-stack 27-75x, buffered-io 99-132x, log-write-direct 1.9-2.0x, sort-unstable 1.08-1.25x, bufwriter-batched 2.48-21.57x, vec-remove-front-to-vecdeque 3.65-1270x, btreemap-sorted-iteration 3.73-6.91x, cow-conditional-alloc 1.32-1.49x, **regex-hoist-lazylock 5443-5675x** (allocs 1.3M→0), **arc-str-payload 5.38-11.38x** (bytes 42×)); 2/40 honestly dominated (precompute-table: compiler optimizes popcount to POPCNT in release; papaya-lockfree: benefit needs multi-threaded contention); 1/40 Mixed (sort-by-key). Closes Proposal 001 caveat 3. Implies `rust_perf`.
- `kernel_heal` (opt-in, Plan 001 Phase 3): GPU kernel optimization healing domain. Absorbs Proposal 012. Enables `katgpt-core/variable_rank_domain_expert` for the `pick_domain` router (Plan 558). 15 category experts (Plan 003), 8-dim activity vector with 15/15 reachable (Plan 004), routed heal pipeline composed in `kernel_heal_demo` example. 4090 LoRA + real popcorn corpus DEFERRED.
- `kernel_verify` (opt-in): CubeCL compile check. **STUB** — delegates to shared `RustAnalyzerPruner`; full numeric compare deferred (Proposal 012 §Phase 0). Implies `kernel_heal`.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (Bench 572) without a code-level dep. Runs once per lint invocation (~400µs/10 candidates), not per-tick. Implies `clippy_lints`.
- `latent_retrieval` (opt-in, Plan 005 Phase 1): real-file span-level latent retrieval — the `--suggest` MVP. Indexes every enabled domain's corpus via riir-rag (`RuleIndex`: `ShardIndex` Clifford-wedge KNN + BM25, multi-scale), routes spans via corpus-derived domain centroids (`DomainRouter`, annotation only), retrieves ranked rules (`LatentMatcher`, fan-out across enabled domains). The suggest path itself has ZERO proposer code — suggestions are the corpus pattern texts; the proposers live in each domain's `proposers/` module (rust_perf Phase 4, clippy T6.4) + the shared `src/edit_emitter.rs` substrate, gated by the proposers' own Cargo features. Gates `dep:riir-rag` (heavy tree: riir-rag + transitively riir-neuron-db — the default build stays light). Powers the `cargo-heal` bin (`--suggest` / `--fix` / `--domain auto|clippy|perf|kernel` / `--top-k`). **GOAT G1–G4 ALL PASS (Bench 018, 2026-08-14):** G1 100% (12/12 top-1, target ≥80%), G2 349µs/chunk (<1ms, 2.9× headroom), G3 104 lib tests, G4 `SuggestScratch` stable across 50 repeat scans. Known limits: fan-out routing (no gate), no minimum-score threshold (every span suggests), 12-item G1 fixture.
- `self_evolve` (DEFAULT-ON since Issue 018, 2026-08-16; Bench 023): latent-first self-evolution loop — `LatentFixMemory` (3-axis retrieval over fix trajectories) + `LatentSkillEvolution` (EMA direction-vector update) + `EvolveRecorder` (the oracle-split standalone recorder for the `cargo heal --fix` path). GOAT G1-G5 PASS (G5: 70pp healing rate improvement in controlled fixture). **G6/G7 validated against real cargo clippy feedback (Issue 008): G6 10/10 corpus truthfulness PASS, G7 50.0pp improvement (50% → 100%) PASS** (re-validated 2026-08-16 post-promotion at the 40-lint corpus scale after the Issue 022 corpus-truthfulness fix — G6 40/40, G7 50%→100%, Bench 024). The modules are UNCONDITIONAL — the memory half runs wherever fixes are applied (+ persists across runs via the v1 JSON envelope); the EMA learning half no-ops with one log line without the `clippy_verify` oracle. The feature flag is a default-on back-compat alias (required-features lists). Opt-out: `RIIR_HEAL_SELF_EVOLVE=0`.
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

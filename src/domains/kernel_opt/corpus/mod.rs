//! Seed corpus — GPU kernel optimization rules.
//!
//! Distilled from popcorn (`tilde-research/popcorn`, 100 kernels × ~100 impls × 7
//! backends). See `.research/002_popcorn_kernel_opt_distillation_batch1.md` for the
//! methodology + per-rule source attribution.
//!
//! Each rule pairs a **reference shape** (the naive/slow pattern) with an **optimized
//! pattern** (the hand-tuned GPU pattern). The patterns are Rust-shaped pseudocode (not
//! real CUDA/WGSL/Triton) so the shared `RustAnalyzerPruner` can validate them in the
//! fast path. The GPU *concept* is encoded in the algorithm structure, not the language
//! syntax.
//!
//! ## Current state
//!
//! - **Batch 1 (Research 002):** 21 rules (2 original placeholders + 19 popcorn-distilled)
//!   across 5 categories: memory access, compute, fusion, dispatch, parallelism.
//! - **Batch 2 (Research 003):** 7 flash-attention tiling rules.
//! - **Batch 3 (Research 004):** 6 linear-attention recurrence rules.
//! - **Batch 4 (Research 005):** 9 `MoE` dispatch + backward-pass rules.
//! - **Batch 5 (Research 006):** 4 NSA block-sparse selection rules.
//! - **Batch 6 (Research 007):** 3 varlen packing + chunked reduction rules.
//! - **Batch 7 (Research 008):** 9 Rust-native rules from `burn-cubecl` — autotune
//!   dispatch (panic-safe candidates, multi-priority tune groups, shape-kind
//!   dispatch, vector-width sweep) + conv-as-GEMM + memory layout (1×1 conv as
//!   matmul, contiguity guard, cube-count batch pack, stride-align vectorization,
//!   anchor shape bucketing).
//! - **Batch 8 (Research 009):** 9 burn-cubecl + cubecl-opt rules (kernel
//!   polymorphism + dtype/alignment specialization + IR strength reduction).
//! - **Batch 9 (Research 010):** 8 popcorn rules — fusion (`add_rms_norm`,
//!   `group_norm_linear`, `jsd`) + dtype/quant (`int8_int2_matmul`, `_quant`) + dispatch.
//! - **Batch 10 (Research 011):** 12 popcorn rules — compute (`hadamard_transform`,
//!   `dyt`, `token_shift`, `sparsemax`) + fusion (`selective_log_softmax`) + dispatch
//!   (`softmax`, `rms_norm`) + chunked reduction (`mean_pooling`, `solve_tril`) +
//!   polymorphism (`geglu`, `swiglu`, `rope`) + dtype (`comba`, `gelu`).
//! - **Batch 11 (Research 012):** 11 popcorn rules — linear-attention recurrence
//!   family (`gla`, `simple_gla`, `rwkv7`, `delta_rule`, `iplr_delta_rule`,
//!   `gated_oja_rule`) + parallelism (`chunk_global_cumsum`, `retention`,
//!   `lightning_attn`) + flash-attn tiling (`deltaformer`) + compute (`based`,
//!   `rebased`, `linear_attn`) + dispatch (`lightning_attn`). A 12th candidate
//!   (`chunk-reset-scan`) was dropped as a duplicate of `chunk-reset-cumsum`
//!   (Batch 6). Corpus: **99 rules across 15 categories (A–O)**.
//! - **Batch 12 (Research 013):** 14 popcorn rules — gated delta family
//!   (`gated_delta_rule`, `gated_delta_product`), specialized recurrences
//!   (`mesa_net`, `mesa_net_decode`, `titans_linear`, `ttt`, `rwkv4`, `rwkv6`),
//!   advanced attention (`wall_attn`, `multi_token_attention`), `MoE` gate
//!   (`momoe`), KDA gate (`kda_gate`, `kda_gate_cumsum`), hierarchical mask
//!   (`log_linear_attn`).
//!   Three candidates (`kda` per-key-dim gate, `forgetting_attn` cumsum decay,
//!   `abc` two-state softmax) were dropped as duplicates of existing rules.
//!   Corpus: **113 rules across 15 categories (A–O)**. Category G (linear-attn
//!   recurrence) is now the largest cluster at 17 rules.
//! - **Batch 13 (Research 014):** 6 popcorn rules — final unmined pass. RL loss
//!   stability (`grpo_offpolicy`: `expm1` for KL cancellation), complex-free
//!   `RoPE` (`llama4_rope`: paired reals), multimodal `RoPE` (`qwen2vl_mrope`:
//!   cyclic modality table), `RWKV-7` channel mixing (`rwkv7_channel_mixing`:
//!   stateful cat-prefix shift + squared `ReLU` from `sqrelu`), `RWKV-7` gate output
//!   (`rwkv7_gate_output`: fused reduce-then-broadcast). Two files (`rwkv7_k_update`,
//!   `sigmoid`) yielded no new rule; `tvd` was covered by `upcast-before-reduction`.
//!   Corpus: **119 rules across 15 categories (A–O)**. Category B (compute) is now
//!   the largest cluster at 18 rules.
//! - **Batch 14 (Research 015):** 4 riir-gpu production rules — first batch mining
//!   the stack's own kernels (not external libraries). Norm-fold into matmul
//!   (`gemv_q4k_batched_rmsnorm`: fold `inv_rms` scalar into the GEMV dot),
//!   reduction-split pre-pass (`compute_inv_rms`: scalar pre-pass + fold),
//!   single-plane reduction (Issue 611 fix: one plane suffices), CPU-bottleneck
//!   offload (`gpu_transpose`: GPU compute kernel into pooled memory). Two riir-gpu
//!   files (`lora_fused`, `rope_geglu`) assessed + dropped (`SGMV` overlaps `MoE` grouped,
//!   section-view is pointer arithmetic not an opt). Corpus: **123 rules across 15
//!   categories (A–O)**. Opens riir-gpu (~196K lines) as the next distillation source.
//! - **Batch 15 (Research 016):** 4 riir-gpu production rules — second batch mining
//!   the stack's own kernels. Block-contiguous weight layout (`gemm_ternary_block_contiguous`:
//!   pack 3 separate weight buffers into 1 to cut global-memory accesses 3×), fused
//!   dual GEMV + `GeGLU` (`gemv_geglu_f16`: fuse 2 sibling GEMVs + act into 1 dispatch,
//!   saves 52 dispatches/decode at 26 layers), orthogonal pre-rotation
//!   (`spectralquant/attention.rs`: pre-rotate query once vs each key, ~4096× fewer
//!   rotation FLOPs), water-fill tail-dim shared codebook (`spectralquant/dequant.rs`:
//!   top dims get per-dim codebooks, tail dims share one low-bit codebook, 2.3×
//!   codebook memory reduction). One riir-gpu file (`forward_flashprefill`) assessed
//!   and dropped (cascade structure overlaps `NSA` Category I; 4 staged dispatches
//!   is a non-opt). Corpus: **127 rules across 15 categories (A–O)**. `riir-gpu`
//!   remains open as the next distillation source (5 of ~200 files mined).
//! - **Batch 16 (Research 017):** 4 riir-gpu production rules — third batch mining
//!   the stack's own kernels. `cmma` hardware unit (`gemm_ternary_simdgroup`:
//!   use the cooperative matrix multiply-accumulate hardware unit instead of
//!   scalar ALU — 64 FMAs/cycle on the dedicated unit vs occupancy-bound scalar),
//!   dequant-once-per-K-tile into shared memory (`gemm_ternary_simdgroup`: dequant
//!   a weight tile once, let the hardware matrix unit consume it across all
//!   output rows — m× amortization), swap-large-axis-to-X (`gemm_ternary_simdgroup`:
//!   put the large GEMM axis on the X cube-count for workgroup distribution),
//!   norm-decoupled low-bit scoring (`forward_turboquant`: store per-position L2
//!   norm separately, quantize the unit-normalized key at 2-4 bits, reconstruct
//!   score as norm × dot). Four files assessed + dropped (`lora_fused`: `SGMV`
//!   overlaps Category H `grouped-launch-cumsum-offsets`; `spectralquant/rotation`:
//!   boilerplate, pre-rotate identity already mined; `spectralquant/qjl`: composition
//!   of water-fill with `JL`; `spectralquant/calibration`: two-pass reduce overlaps
//!   `split-reduction-scalar-prepass`). Corpus: **131 rules across 15 categories
//!   (A–O)**. `riir-gpu` remains open as the next distillation source (7 of ~200
//!   files mined).
//! - **Batch 17 (Research 018):** 4 riir-gpu production rules — fourth batch mining
//!   the stack's own kernels, focused on the ternary scalar-ALU hot loop.
//!   Skip-all-zero-packed-words-via-or-reduce (`gemm_ternary_batched`: OR-reduce all
//!   tile rows' pos+neg bit-words, skip the 32-iteration sign-extraction loop when
//!   the whole tile's word is zero — ternary sparsity exploit), hoist-group-invariant-
//!   scale-out-of-word-loop (`gemv_ternary_block_contiguous`: cache the per-group
//!   quant scale in a register across the 4 words it covers, reload only at the
//!   group boundary — 4× fewer global loads), split-sign-accumulators-defer-
//!   subtraction (both ternary files: keep separate pos/neg accumulators inside
//!   the bit loop, subtract once at the end — 32→1 subtracts/word + independent
//!   ILP chains), clamp-tile-indices-to-sentinel-discard-at-write
//!   (`gemm_ternary_batched`: alias OOB tile rows to m-1, compute fully, discard
//!   at write time — keeps the loop body uniform without padding overhead). Two
//!   files assessed + dropped (`rope_geglu_cubecl`: pairing-generic + precomputed
//!   cos/sin table are composition of existing dispatch/precompute principles;
//!   `forward_dflare`: 17-binding layout + heterogeneous KV are architectural, not
//!   kernel optimization). Corpus: **135 rules across 15 categories (A–O)**.
//!   `riir-gpu` remains open as the next distillation source (9 of ~200 files mined).
//! - **Batch 18 (Research 019):** 3 riir-gpu production rules — fifth batch mining
//!   the stack's own kernels, focused on the `Q4_K` quantized GEMV family (4-bit
//!   block-quant format with per-block scale + min). Row-tile-amortize-input-load
//!   (`gemv_q4k_plane_rowtiled`: assign N output rows per plane, load input once,
//!   reuse across N dot products — GEMV to small-GEMM via M-widening), factor-
//!   affine-quant-scale-out-of-element-loop (`gemv_q4k_plane_factored`: when
//!   dequant is affine in the code within a sub-block, factor d·scale and
//!   dmin·min out — `Q4_K`-specific, ternary has no min offset to factor), pin-
//!   lane-to-quant-sub-block-coalesce-byte-loads (`gemv_q4k_plane_factored`: remap
//!   lane→element so each lane is pinned to one sub-block — 8 loads→2, enables
//!   affine factoring). Three sibling `Q4_K` files (qkv, geglu, batched) assessed
//!   and dropped: qkv = composition of fuse-gate-up-gemv-activation (triple fusion);
//!   geglu = same fuse + `Q4_K` dequant; batched = no new opt. Corpus: **138
//!   rules across 15 categories (A–O)**. `riir-gpu` remains open (13 of ~200
//!   files mined).
//! - **Batch 19 (Research 020):** 2 riir-gpu production rules — sixth batch mining
//!   the stack's own kernels, focused on the fused-dispatch family.
//!   Chain-dispatches-single-sync-point (`ternary_ffn_fused`: queue N dependent
//!   dispatches in one command buffer, sync only once at the end — command-buffer-
//!   level fusion distinct from single-kernel fusion; linear dependency chains keep
//!   intermediates bounded, fan-out is the known-bad case per Bench 437),
//!   fuse-residual-via-in-place-output-array (`gemv_ternary_residual_cubecl`: when
//!   a kernel needs N+1 array params but the runtime limits to N, alias the output
//!   buffer as the accumulation input (read-then-write) — bypasses `CubeCL` v0.10's
//!   hard 5-array limit, collapses 128 dispatches/token → 64). Three sibling files
//!   assessed + dropped (`deltanet_input_proj_fused`: same pattern as
//!   `ternary_ffn_fused` — confirms generality; `deltanet_chunked` + base recurrence:
//!   covered by recurrent-vs-chunk-dispatch Batch 11). Corpus: **140 rules across
//!   15 categories (A–O)**. `riir-gpu` remains open (16 of ~200 files mined).
//! - **Batch 20 (Research 021):** 5 riir-gpu production rules — seventh batch mining
//!   the stack's own kernels, focused on the CUDA hardware-instruction cluster +
//!   `CubeCL` LUT + base-3 trit paths. Cooperative-block-wide-quantize-staging
//!   (`gemv_ternary_cuda_raw` fused variant: ALL 256 threads cooperate to quantize
//!   f32→int8 into shared memory once, then each warp reads from shared —
//!   eliminates a separate staging dispatch + global round-trip; distinct from
//!   dequant-once-per-k-tile which is per-tile WEIGHT dequant, this is block-wide
//!   ACTIVATION quantize), byte-perm-lut-via-permutation-immediate
//!   (`gemv_ternary_cuda_raw`: pack {−1,0,1,2} in one int, `__byte_perm`(constant,
//!   constant, code >> shift) decodes 4 codes to 4 signed bytes in ONE PTX
//!   instruction — first CUDA-specific HW-instruction rule, no `CubeCL` analogue),
//!   dp4a-int8-hardware-dot-product (`gemv_ternary_cuda_raw`: NVIDIA `__dp4a` does
//!   4-lane int8 dot in one instruction — distinct from cmma Batch 16 which is
//!   `CubeCL` 8×8×8 matrix tile op, this is scalar 4-lane int8 dot, CUDA-only),
//!   threadgroup-subset-sum-lut-keyed-by-activation (`gemv_ternary_cubecl` LUT
//!   path gated `ternary_lut_gemv`: precompute 16-entry subset-sum table per
//!   4-activation group in threadgroup memory, 4-weight ternary dot = 2 reads + 1
//!   subtract; one build serves every row because the table is keyed by ACTIVATION
//!   subset — structurally different amortization from the `Q4_K` LUT negative
//!   result which was keyed by weight+scale+zero-point and rebuilt 160 tables/row),
//!   base-3-trit-packing-5-per-byte (`gemv_ternary_cubecl` trit path: pack 5
//!   ternary per byte using base-3 mixed-radix, 17.6% denser than 2-bit codes;
//!   REGIME-DEPENDENT — wins on Metal, loses 2.1× on CUDA scalar ALU per Issue
//!   607). Three Metal files assessed + dropped (all 3 REFUTED end-to-end — 3.72×
//!   kernel gain lost to 0.87×/0.95× integration overhead; 2 genuinely novel
//!   patterns deferred to Batch 21). Corpus: **145 rules across 15 categories
//!   (A–O)**. `riir-gpu` remains open (18 of ~200 files mined).
//! - **Batch 21 (Research 022):** 5 riir-gpu production rules — eighth batch mining
//!   the stack's own kernels, focused on the **Metal cross-runtime zero-copy
//!   integration cluster** (`gemm_ternary_metal_zero_copy.rs`, Issue 663 T5). The
//!   patterns are about the INTEGRATION ARCHITECTURE (how a higher-level runtime
//!   hands GPU memory to a raw native dispatch), not the kernel itself.
//!   Cross-runtime-raw-handle-extraction (extract `id<MTLBuffer>` from CubeCL/wgpu
//!   handles via `as_hal::<Metal>().raw_handle()` to bypass wgpu-core validation —
//!   generic, applies to any cross-runtime integration where a wrapper runtime
//!   needs to pass GPU memory to a raw native dispatch without a host round-trip),
//!   pool-buffer-aliasing-dedicated-output (when the runtime sub-allocates handles
//!   within one pool buffer, input/output aliasing breaks hazard tracking — fix:
//!   dedicated output buffer keyed by byte size + GPU→GPU copy back; distinct from
//!   inplace-output-alias Batch 19 which is parameter-count bypass), same-queue-
//!   implicit-ordering-no-sync (dispatch on the shared command queue the wrapper
//!   uses — in-order execution makes explicit sync unnecessary; eliminates the
//!   host-round-trip serialization that refuted Issue 656), per-dispatch-pointer-
//!   re-extraction (when the runtime can reallocate its pool between dispatches,
//!   raw pointers must be re-extracted on EVERY dispatch, not cached — the
//!   correctness invariant that makes raw-handle-extraction safe), no-blocking-
//!   sync-in-async-pipeline (`wait_until_completed` serializes the async pipeline —
//!   use queue ordering + caller's pre-dispatch sync to cover correctness;
//!   complement of chain-dispatches-single-sync-point Batch 19). The 2 refuted
//!   siblings (`gemm_ternary_metal_tensor.rs` + `_wgpu.rs`) are covered by these
//!   general rules: the host-round-trip is the anti-pattern rule #3 fixes, the
//!   per-dispatch staging copies are a special case of rule #2 when the runtime
//!   can't extract raw handles. Corpus: **150 rules across 15 categories (A–O)**.
//!   `riir-gpu` remains open (19 of ~200 files mined).
//! - **Batch 22 (Research 023, 2026-08-14):** mined the `cudarc_kernels/`
//!   directory — the full raw-CUDA forward port for Issue 615 (4 files:
//!   `embedding.rs` + `attention.rs` + `mod.rs` + `deltanet.rs`, ~8278 LOC).
//!   The port exists because the dp4a GEMV kernel's 4.7× speedup is
//!   uncapturable in mixed mode (CPU elementwise + GPU GEMV) — the only way
//!   to capture it is a full GPU-resident forward where ALL ops run on the
//!   same CUDA stream. 5 new rules: dynamic-shared-memory-runtime-sized (A —
//!   `extern __shared__` with launch-time `shared_mem_bytes` for runtime-sized
//!   dimensions like `head_dim`; fixes the Bonsai-27B `head_dim=256` bug where a
//!   compile-time `Shared::<[f32]>::new_slice(128)` was too small),
//!   ternary-bit-plane-u64-as-2x-u32 (A — two bit-planes packed as u64
//!   blocks cast to 2×u32 for per-bit sign extraction; distinct from
//!   base-3-trit-packing-5-per-byte which is arithmetic base-3),
//!   warp-shuffle-intrinsics-for-sub-warp-reduction (B — `__shfl_down_sync`
//!   butterfly reduction for blocks ≤32 threads = one warp; no smem alloc,
//!   no syncthreads; includes the `volatile float* vsmem` companion for the
//!   final 32-thread tail of a larger smem tree), fused-activation-quantize-
//!   for-gemv-consumer (C — fuse RMSNorm/SwiGLU/gate*sigmoid + absmax + int8
//!   quantize into one kernel when the activation feeds a GEMV that reads
//!   int8; eliminates the intermediate f32 HBM round-trip; distinct from
//!   fuse-quant-matmul which is quantize+matmul), cuda-graph-scalar-via-
//!   device-pointer (D — ship a `_devpos` variant that reads scalars from a
//!   device pointer dereferenced at kernel runtime so the captured CUDA graph
//!   can process different inputs each launch; scalar args get baked into the
//!   captured graph at capture time). Corpus: **155 rules across 15 categories
//!   (A–O)**. `riir-gpu` remains open (23 of ~200 files mined; `cudarc_kernels/`
//!   fully covered).
//! - **Batch 23 (Research 024, 2026-08-14):** mined the `attention_cubecl/`
//!   directory — `CubeCL` flash-attention decode cluster (`mod.rs`, 1573 lines).
//!   This is the `CubeCL` portable-path counterpart to the Batch 22 CUDA raw
//!   port. 5 new rules: combined-kv-buffer-keys-then-values (A — store keys
//!   and values as a single contiguous buffer `[keys | values]` to halve
//!   buffer bindings + coalesce via one `kv_off` index), gqa-kv-head-sharing-
//!   bandwidth-reduction (A — Grouped-Query Attention: share KV heads across
//!   query heads via `kv_group = head_idx * n_kv_head / n_head`; reduces KV
//!   cache bandwidth by `n_head/n_kv_head`×; e.g. 2× for Gemma 2 2B's 8:4),
//!   softcap-tanh-attention-score (B — Gemma's `softcap * tanh(score/softcap)`
//!   bounds raw logits to `[-softcap, +softcap]`; smooth + differentiable,
//!   unlike clamp; `softcap=0` disables for LLaMA/MiniCPM), fold-query-tokens-
//!   for-occupancy (D — when `n_head < SM_count`, fold multiple speculative-
//!   decode query tokens into one workgroup via `fold_factor()`; largest
//!   divisor of `seq_len_q` ≤ `workgroup_size/n_head`), hardcoded-fast-path-
//!   jit-specialization (D — dispatch a hardcoded kernel variant when params
//!   match a known config so the JIT can constant-fold dims into optimized
//!   Metal/WGSL shaders; ~24% decode throughput gain on M3 Max).
//! - **Batch 24 (Research 025):** 4 rules from `cudarc_kernels/backward.rs`
//!   (1923 lines; Issue 641 per-layer backward CUDA kernels): widen-index-
//!   multiply-to-64bit (A — cast one operand of an address-producing multiply
//!   to 64-bit when position × stride can exceed 2^31; int32 wraps silently),
//!   dual-accumulator-single-reduction-sweep (B — two block reductions over
//!   the same data fused into one sweep: shared loads + shared barrier tree;
//!   (`RMSNorm` backward's sum(x²) + `dot(grad_y·γ·x)`), dual-grad-same-pass
//!   (C — both partials of a product-rule activation emitted from ONE kernel
//!   sharing input reads + one sigmoid evaluation), broadcast-uniform-skip-
//!   reduce (E — thread-uniform values derived from already-reduced
//!   broadcasts must be broadcast, not re-reduced; re-reducing multiplies by
//!   blockDim.x — the T7.2 validation bug in softmax backward).
//! - **Batch 25 (Research 026):** 5 rules from the Gemma cluster
//!   (`gemma2_cubecl/` + `gemma4_cubecl/`): device-side-argmax-minimal-download
//!   (D — greedy decode needs ONE u32, not the logits: run argmax on device,
//!   download 4 B instead of ~1 MB/token, ~250,000× smaller boundary
//!   crossing), monotone-transform-elision-under-argmax (B — softcap
//!   `cap·tanh(x/cap)` is strictly increasing → argmax unchanged → skip the
//!   256K-tanh pass for greedy decode), fan-out-shared-input-single-sync
//!   (C — N INDEPENDENT sibling kernels sharing one input: upload once,
//!   clone the handle, queue all, first read executes the batched submission),
//!   hot-swap-weights-preserve-autotune-caches (D — `LoRA` training refreshes
//!   weight handles per step while keeping autotune + JIT caches; avoids
//!   ~seconds of re-benchmark + recompile per step), tree-reduction-depth-
//!   over-sequential-accumulate (B — plane tree depth ≈77 vs sequential
//!   depth 2304: ~30× better accuracy, prevents 26-layer gradient divergence,
//!   Issue 424). Corpus: 169 rules across 15 categories (A–O).
//! - **Batch 26 (Research 027):** 8 rules from the wgpu forward-orchestration
//!   cluster (`forward/` + the `buffer.rs` download substrate it consumes) —
//!   the host-side dispatch discipline previous kernel-only batches never
//!   touched: per-slot-uniform-buffer-write-ordering (D — wgpu flushes ALL
//!   `queue.write_buffer` calls before the encoder executes, so N dispatches
//!   sharing ONE uniform buffer all read the LAST write; one 64-byte uniform
//!   per slot, each wired into its own pre-built bind group — the Issues
//!   366/367/368 corruption lineage), derive-params-from-global-invocation-id
//!   (D — encode the per-item parameter in the dispatch grid and decode it
//!   in-kernel: one dispatch for `n_head × seq_len` workgroups with causal
//!   masking + GQA mapping computed inside, zero per-position uniform writes
//!   and zero per-position submits), merge-dispatches-into-single-compute-
//!   pass (D — dispatches within one compute pass execute IN ORDER, so the
//!   consumer reads the producer's writes: 6 `LoRA` dispatches in one pass,
//!   input saves riding the copy channel with no pass at all),
//!   batched-downloads-single-sync-cycle (D — N readbacks share one encoder,
//!   submit, `map_async`, and poll; ~1.28 ms per sync measured regardless of
//!   data size), chunk-encoder-submits-below-driver-limit (D — Metal
//!   DEADLOCKS past a compute-pass-per-encoder ceiling; chunk to ~48
//!   passes/encoder and submit each chunk immediately), prebuilt-bind-
//!   groups-fixed-buffer-combos (A — bind groups for fixed buffer combos are
//!   built once at init: ~768 `create_bind_group` calls per forward → 0),
//!   host-cache-immutable-device-weights (A — constant weights cloned
//!   host-side at init; per-call download was the #1 forward bottleneck at
//!   2 sync stalls), defer-readback-until-after-submit (A — downloading
//!   mid-encoder-construction reads pre-submission stale state; record the
//!   correction and apply it after submit + completion). Corpus: **177 rules
//!   across 15 categories (A–O)**. `riir-gpu` remains open (38 of ~200 files
//!   mined; the forward/ cluster + buffer.rs download substrate assessed).
//! - **Batch 27 (Research 028):** 6 rules from the wgpu backward clusters
//!   (the `backward/` + `backward_dflash/` directories — the low-rank-adapter
//!   backward pass host orchestration + the `DFlare` concat-KV backward):
//!   `staging-packed-uniforms-encoder-ordered-copies` (D — the complementary
//!   fix to per-slot buffers: pack ALL uniform params into ONE 16-byte-aligned
//!   staging vec, upload once, distribute via `encoder.copy_buffer_to_buffer`
//!   immediately before each consuming dispatch — the encoder timeline lands
//!   each copy exactly between dispatches, so ONE shared uniform can change
//!   value per dispatch), `identical-uniforms-share-single-slot` (D — the
//!   exemption to the write-ordering hazard: sharing is safe IFF every write
//!   is byte-identical, the last write being idempotent with all earlier
//!   ones; N uniforms + N bind groups → 1), `persistent-zeros-source-encoder-
//!   copy-clear` (A — re-zeroing accumulators between dispatches needs
//!   encoder-timeline ordering: a device buffer zeroed once at init + copy
//!   clears; queue writes flush before the whole encoder so layer 1 would
//!   accumulate on top of layer 0's values), `fuse-sibling-target-grads-
//!   single-kernel` (C — N sibling targets sharing the same two backward
//!   kernels fuse per KERNEL KIND, not per target: one summed gradient + one
//!   grid-sized param-grad dispatch; 6 → 2), `per-layer-strided-grad-
//!   destination` (D — when a consumer needs per-layer partitioned
//!   gradients, write layer-major strided offsets directly from the same
//!   dispatches: the split falls out for free, no de-interleave pass),
//!   `snapshot-transient-grad-for-deferred-download` (A — a host-needed
//!   gradient living in an overwrite-doomed temp is pinned by an encoder-
//!   ordered copy into a persistent buffer, downloaded after submit — the
//!   LIFETIME complement to defer-readback's TIMING rule). Corpus: **183
//!   rules across 15 categories (A–O)**. `riir-gpu` remains open (42 of
//!   ~200 files mined; the `forward/` + `backward/` + `backward_dflash/`
//!   orchestration clusters assessed).
//! - **Batch 28 (Research 029):** 10 rules from the two largest single
//!   files in riir-gpu — the deltanet `CubeCL` cluster (`deltanet_cubecl.rs`
//!   decode-time recurrence + `deltanet_chunked_cubecl.rs` chunkwise-
//!   parallel prefill, Issue 652 / Plan 533, algorithm Yang & Wang 2024):
//!   `row-owned-register-blocked-recurrence` (G — decompose the per-head
//!   state matrix WITHIN one decode step: one workgroup per (head, row),
//!   the row register-blocked into 4 strided columns per lane, all five
//!   algebra steps on registers between exactly ONE global read + ONE
//!   write — 6 passes over S → 2, ~3× cut of the gating memory quantity,
//!   plus 32× occupancy headroom), `workgroup-equals-plane-native-
//!   reduction` (E — size the workgroup to EXACTLY one hardware plane and
//!   compensate width via register blocking: the native shuffle reduction
//!   then covers the full logical reduction, broadcasts for free, and no
//!   threadgroup memory or barrier exists at all), `smem-broadcast-
//!   scalar-then-parallel-scatter` (E — after a workgroup reduction, the
//!   leader publishes the resulting scalar through the already-reduced
//!   slot and ALL threads scatter their own column: 1 global write per
//!   thread instead of `head_dim` serial writes from one idle-heavy
//!   leader), `token-flattened-elementwise-modulo-shared-weights` (D —
//!   per-token elementwise launches flatten `[p, n_head]` into ONE 1-D index
//!   space with per-head weight tables indexed `i % n_head`: 6144
//!   dispatches → 48, each launch's ~25 CPU allocations amortized away),
//!   `fused-conv-state-shift-dot-silu` (C — the decode conv1d shifts its
//!   sliding window IN PLACE, convolves from the freshly shifted state,
//!   applies `SiLU`, and writes back over the input — one kernel, zero
//!   intermediates, state persists across steps), `softplus-piecewise-
//!   saturation-clamp` (B — `ln(1+e^x)` saturates: x for x > 20, 0 for
//!   x < −20, transcendental only in the safe band; thresholds pinned to
//!   the CPU reference for GPU/CPU bit-comparability), `reverse-running-
//!   decay-weight` (G — the intra-chunk pairwise decay matrix dematerialized
//!   into ONE running scalar carried by the consumer loop running j from t
//!   DOWN to 0: the empty product (=1 at j=t) anchors the recursion and
//!   each step multiplies α[j] after use), `split-decay-outputs-eliminate-
//!   readback` (D — compact packing is a pessimization when it breaks
//!   consumer layouts: write each output component directly into the
//!   buffer shape its consumer reads and the device→host→device unpack
//!   round trip disappears entirely), `carry-dual-layout-dead-slot` (L —
//!   the prefill carry producer parameterized (stride + offset) to write
//!   the decode consumer's native sliding-window layout directly, leaving
//!   slot 0 stale because the decode shift provably overwrites it before
//!   any read), `nonlinearity-forces-raw-input-buffers` (D — the negative
//!   space of in-place aliasing: a sequential in-place producer trick is
//!   incompatible with batched execution when a nonlinearity intervenes,
//!   because later tokens' windows read earlier tokens' POST-activation
//!   values — separate raw read-only inputs + write-only outputs are the
//!   correctness-enabling shape). Corpus: **193 rules across 15 categories
//!   (A–O)**. `riir-gpu` remains open (44 of ~200 files mined; the
//!   deltanet `CubeCL` cluster assessed). Mining also surfaced a confirmed
//!   production bug (cross-chunk `plane_sum` mixing partials of different
//!   columns → block-constant output) — filed as a riir-ai issue.
//! - **Batch 29 (Research 030):** 10 rules from the decode-path fusion
//!   cluster — `deltanet_input_proj_fused.rs` (Issue 602: the 4 `DeltaNet`
//!   input-projection ternary GEMVs chained into one command buffer) +
//!   `epilogue/` (Plan 106 Track 3: the CODA GEMM-plus-epilogue cluster —
//!   `mod.rs` + `coda_primitives` + `gemv_residual` + `norm_residual`):
//!   `defer-readback-gated-by-live-output-size` (A — deferral is NOT
//!   unconditional: 5 concurrent ~70 KB FFN buffers measured 16% SLOWER
//!   via memory pressure while ~8 KB of input-projection outputs chain
//!   freely — gate on total live bytes), `weight-handle-cache-keyed-by-
//!   allocation-identity` (A — a hook receiving CPU weight refs resolves
//!   to cached device handles via the allocation's pointer pair, with an
//!   on-the-fly miss path), `first-readback-pays-queue-drain` (D — after
//!   one dispatch burst the first read drains the queue and the rest
//!   return immediately: syncs are a function of bursts, not read calls),
//!   `epilogue-at-register-write-phase` (C — the CODA insight: memory-
//!   bound elementwise ops execute while the GEMV output is still in
//!   registers, riding lane 0's single store), `delay-rms-scale-past-
//!   gemv-epilogue` (C — the commutation RMSNorm(x)@W = r·(x·γ)@W splits
//!   the norm into extract/γ/defer with the honest 3→3 accounting),
//!   `pack-scalar-params-into-combined-buffer` (D — the ~4-5 Array-param
//!   budget met by one packed f32 params buffer, derived values host-
//!   computed, u32s bitcast through the pack), `enum-dispatch-separate-
//!   kernel-per-variant` (M — no generics in `#[cube]` kernels → host-
//!   level enum + one branch-free kernel per variant + metadata-driven
//!   plumbing), `epilogue-must-be-workgroup-local` (E — fusion legality
//!   is data OWNERSHIP scope, not just dependency: attention stays 9
//!   dispatches while post-attn drops to 1), `launch-unchecked-caller-
//!   owned-size-contract` (D — elide per-launch validation into a
//!   documented `# Safety` contract at the launcher seam), and
//!   `fast-math-contraction-behind-feature-flag` (B — a `cfg_attr` pair
//!   keeps strict-FP the byte-identical default while the dot loop
//!   contracts to FMA only on the opt-in build). Corpus: **203 rules
//!   across 15 categories (A–O)**. `riir-gpu` remains open (49 of ~200
//!   files mined). Mining also surfaced 2 latent cache hazards (pointer-key
//!   staleness on model reload; the key covers only qkv, not z/a/b) and a
//!   stale-doc-numbers bug in `epilogue/mod.rs` L43 — filed as an issue in
//!   the `riir-ai` repo.
//! - **Batch 30 (Research 031):** 6 rules from the D2F training cluster —
//!   `dllm/mod.rs` (Plan 068: discrete-diffusion LM GPU training — the
//!   bidirectional dLLM trainer, the asymmetric distillation loop, the
//!   direct block-causal trainer, and the `RePlaid` adaptive noise schedule):
//!   `zero-init-side-branch-frozen-base-path` (M — run the frozen distillation
//!   teacher through the SAME adapter-capable forward with B=0: W·x +
//!   B·(A·x) = W·x exactly — one code path + one weight set for both
//!   modes), `counter-derived-seed-stateless-noise` (E — the corruption
//!   RNG seeded from ((step << 32) | epoch): every iteration's noise is a
//!   pure function of its indices, replays standalone, and the seed
//!   crosses to the GPU as one uniform), `predicate-gated-skip-empty-
//!   workset-step` (D — one host-side integer check elides the ENTIRE
//!   forward/loss/backward/optimizer batch when masking drew zero, and
//!   guarantees the count-based loss normalizer never divides by zero),
//!   `share-pipeline-set-across-subsystems` (D — one Arc-shared pipeline
//!   set handed to every subsystem constructor: WGSL validation +
//!   compilation paid once per run), `pad-uniform-pod-to-min-binding-size`
//!   (N — explicit _pad u32 fields pin every uniform param struct to
//!   exactly 16 B so one fixed-size buffer helper serves all kernels,
//!   with `size_of` tests pinning the layout in CI), and
//!   `fuse-count-into-mutation-pass` (C — the masked count accumulated
//!   INSIDE the corruption loop instead of a second O(n) scan; the count
//!   feeds both loss normalization and the empty-workset gate). Corpus:
//!   **209 rules across 15 categories (A–O)**. `riir-gpu` remains open (50
//!   of ~200 files mined). Mining also surfaced a unit-incoherent sign
//!   basis in `RePlaid` `adapt()` + a tracker-state orphaning bug in
//!   `adapt_ratios()` + error-swallowing `unwrap_or_default()` on GPU
//!   downloads — filed as an issue in the `riir-ai` repo.
//! - **Batch 31 (Research 033):** 8 rules from the training-substrate
//!   cluster — `lora/mod.rs` + `optimizer.rs` + `dataloader.rs` +
//!   `buffer.rs` (the layers UNDER the dllm training loop mined in
//!   Batch 30): `ragged-pack-offset-table-fused-dispatch` (D — the six
//!   per-adapter GEMV dispatches repacked into contiguous buffers + a
//!   prefix-sum offset table so ONE launch over Σ `out_dim` covers every
//!   adapter: ragged segments need an offset table, a modulo cannot
//!   identify them), `twin-projection-shared-delta-adapter` (L — GQA K/V
//!   twins share ONE adapter pair: params −33%, optimizer dispatches −1
//!   per layer, KV delta cache halves at inference),
//!   `min-rank-inert-adapter-zero-alpha` (M — structurally-required-but-
//!   inert adapters allocate rank=1 + alpha=0: minimum legal footprint,
//!   annihilated by scale, not by buffer contents),
//!   `debug-only-gpu-label-macro` (M — cfg-split label macro: debug gets
//!   format!'d capture labels, release gets a typed None constant),
//!   `sync-map-result-stack-cell` (A — poll(Wait) IS the barrier, so the
//!   map callback parks its result in an `UnsafeCell` instead of an
//!   Arc<Mutex>: zero lock + zero alloc on the readback critical path),
//!   `staging-reuse-grow-only-map-prefix` (A — one loop-owned grow-only
//!   `MAP_READ` staging buffer (~50-500μs driver alloc amortized to zero
//!   steady-state; maps only the copied prefix), `mapped-at-creation-
//!   init-upload` (A — one-shot weights upload via `create_buffer_init`:
//!   no encoder, no submit, no queue-timeline ordering hazard), and
//!   `dtype-normalize-once-at-upload` (N — bf16→f32 is a bit-shift; do
//!   it once at ingestion instead of per-dispatch forever). Corpus:
//!   **217 rules across 15 categories (A–O)**. Mining ALSO surfaced the
//!   CRITICAL `AdamWOptimizer` shared-uniform truncation — every recorded
//!   pass saw the LAST write's `param_count`, silently training only 25%
//!   of `Mlp1.B`/`Mlp2.A` in every GPU `LoRA` loop — FIXED as riir-ai Issue
//!   679A (per-slot uniforms) + 10 residual hazards filed (Issue 679
//!   B1-B10).
//! - **Batch 32a (Research 034):** 11 rules from the `edge_lora` CORE cluster —
//!   `edge_module.rs` + `types.rs` + `training.rs` + `adaptive_width.rs`
//!   (host-side training orchestrators; the first half of the `edge_lora`
//!   module, the 32b half = self-play/arena/consolidation):
//!   `caller-owned-split-scratch` (A — hot-path fn takes ONE caller-owned
//!   scratch, `split_at_mut` carves intermediate+output, returns a
//!   borrowed view; trait-shaped allocating paths quarantined as cold
//!   adapters), `inplace-gate-scale-at-funnel-bottleneck` (C — the sigmoid
//!   gate runs IN PLACE on the rank-wide intermediate AND absorbs the
//!   broadcast scale: rank ops, not `out_dim`, and no wide second pass),
//!   `sign-branch-stable-sigmoid` (B — branch on sign so exp's argument is
//!   always <= 0; the naive form overflows f32 exp at x < -88.7),
//!   `incremental-multi-part-hash-no-concat` (A — BLAKE3 `hasher.update`
//!   per tensor replaces materializing `a || b`: the streaming update IS
//!   the concatenation, zero per-step allocation on the commitment path),
//!   `bool-fields-over-bitset-hot-reads` (L — the REGIME BOUNDARY of
//!   packing: per-tick-read flags stay as bools (decode ALU dominates);
//!   pack only when bytes streamed dominate), `per-layer-grad-norm-lr-
//!   equalization` (B — per-step per-layer grad-norm LR scaling
//!   `min(max_norm/layer_norm, cap)` combats a measured ~1000× vanishing
//!   gradient; zero-norm → 1.0 rails), `width-invariant-superset-
//!   training` (D — ONE superset weight set trained across sampled widths
//!   serves every inference budget; gate: width-1 ≥ 95% of width-4),
//!   `superset-sized-scratch-across-variants` (A — scratch sized ONCE for
//!   the worst-case variant; caveat: consumers with exact-length checks
//!   force per-variant allocs instead), `active-index-grad-scatter-
//!   subset-backward` (H — backward in dense sub-space + precomputed
//!   index scatter into the full buffer; inactive params zero by
//!   construction, no masks, no atomics), `inplace-grad-accumulate-
//!   deferred-postscale` (B — accumulate RAW per sample, ONE post-loop
//!   reciprocal sweep: 1 division total, not batch×params), and
//!   `fuse-loss-and-output-grad-one-pass` (C — loss scalar + analytic
//!   output-gradient from the SAME diff in one sweep). Corpus: **228
//!   rules across 15 categories (A–O)**. Mining ALSO surfaced the
//!   `AdaptiveWidthTrainer` per-episode construction storm (`edges.clone()` +
//!   fresh `TopologyTrainer` + fresh `ForwardCache` + fresh `EdgeGradients`
//!   per EPISODE, root cause = exact-length cache check) + silently ignored
//!   `TrainingConfig` fields (`weight_decay`/`grad_clip_norm`/
//!   `per_layer_lr_scale`) + misreported `mean_weighted_loss` — filed as a
//!   riir-ai issue.
//! - **Batch 32b (Research 035):** 10 rules from the `edge_lora` training
//!   half — `topology_trainer.rs` (1468L, the topology-evolution trainer),
//!   `chain_commit.rs`, `sigmoid_gate.rs`, and `mod.rs` (chain commitment
//!   and per-edge additive gating):
//!   `static-classification-hoist-to-routing-table` (D — the hot loop
//!   re-classifies immutable topology per call via an O(L) scan per edge
//!   plus a fresh Vec per call; classify ONCE at construction into CSR-style
//!   buckets — notably `edge_source_layers()` already computed the exact
//!   mapping but only a cold path consumed it), `sigmoid-deriv-recovered-
//!   from-output` (A — σ'(x) = σ(x)·(1−σ(x)) makes the derivative a pure
//!   function of the OUTPUT: cache only the post-activation, recover the
//!   derivative algebraically incl. through a scale factor — halves
//!   activation-cache traffic), `fuse-weight-grad-input-grad-single-
//!   matrix-visit` (C — the weight-grad outer product WRITES B's grads
//!   while the input-grad transposed matvec READS B: one visit per row
//!   serves both roles), `needless-swap-temp-hoist-to-scratch` (A — a
//!   'defensive' per-iteration `vec![0.0; hd]` motivated by a FALSE
//!   aliasing premise (read buffer ≠ write buffer): write the destination
//!   directly; genuine ping-pong temps are loop-carried state in the
//!   persistent scratch), `scoped-init-at-consume-site` (A — blanket
//!   whole-scratch `clear()` memsets dead lanes + buffers the consumer
//!   re-zeroes anyway: only ACCUMULATION targets need pre-zeroing, scoped
//!   to the live window at the consume site), `monomorphize-per-element-
//!   trait-calls` (B — per-node `&dyn` vtable indirection in the hot loop
//!   → generic `+ ?Sized` (the same file's backward is already generic;
//!   the inconsistent pair was the evidence)), `additive-sigmoid-gates-
//!   not-softmax-routing` (E — softmax routing forces a cross-edge
//!   reduction barrier (max+sum) before ANY gate is final; per-edge
//!   independent sigmoid gates compose additively — embarrassingly
//!   parallel, multiple edges simultaneously near 1.0 by design; the
//!   house sigmoid-not-softmax rule as a kernel-corpus rule),
//!   `commit-over-maintained-item-digests` (B — container commitment
//!   digests the maintained 32-byte per-item digests: O(items), not
//!   O(total payload bytes) per commit), `verify-once-at-trust-boundary`
//!   (D — the load path re-verifies at every layer (deserialize →
//!   `edges_from_bundle` → per-artifact re-hash): verify ONCE where
//!   untrusted bytes become trusted state, return a capability type;
//!   the canonical path paid up to 3 full BLAKE3 passes per load), and
//!   `derive-shape-from-payload-not-cached-field` (A — no cached `in_dim`
//!   field that can drift: `gate_vectors.first().map(|v| v.len())` at the
//!   API boundary is the single source of shape truth; the desync bug
//!   class becomes structurally impossible). Corpus: **238 rules across
//!   15 categories (A–O)**. 21 candidates dropped as duplicates/thin
//!   (routing-table-per-call → covered combos; unroll/scratch/prepass
//!   patterns already in corpus). Mining ALSO surfaced the hot-loop
//!   allocation storm (per-node `vec!` ×2 sites + per-edge `to_vec()`
//!   borrow workaround) + the O(E·L²) routing re-derivation + dead
//!   scratch clears + a false-premise aliasing comment — filed as a
//!   riir-ai issue.
//! - **Batch 32c (Research 036):** 13 rules from the `edge_lora`
//!   self-play/arena/consolidation half — `sleep_consolidation.rs`,
//!   `arena_eval.rs`, `cross_game_edge.rs`, `cross_game_real.rs`,
//!   `{go,bomber,fft}_self_play.rs`:
//!   `incremental-centroid-running-sum` (C — the greedy clustering loop
//!   recomputes each cluster centroid from scratch per comparison: full
//!   member sweep + fresh Vec per (item, cluster) pair; maintain
//!   (sum, count) per cluster instead — the numeric-aggregate twin of
//!   digest maintenance), `length-prefix-shape-faithful-hash` (B —
//!   payload-only multi-part hashing is shape-blind: a=[1,2],b=[3] and
//!   a=[1],b=[2,3] collide; feed part lengths into the stream FIRST),
//!   `order-stable-topk-via-index-sort` (D — sort an INDEX array by
//!   score, truncate to the top-K set, re-sort indices ascending: the
//!   items never move + the output is canonical original order),
//!   `make-unmake-over-clone-per-candidate` (A — candidate search
//!   clones the whole state per probe; apply in place + roll back the
//!   recorded delta, bit-identical, zero per-candidate allocation),
//!   `argmax-rng-tie-break-collect-ties` (B — `max_by` returns the first
//!   max (hidden move-order priority) and softmax sampling adds
//!   normalization to solve a tie problem; one sweep collects
//!   epsilon-ties + one seeded RNG draw), `shape-gated-warm-start-
//!   slot-copy` (D — warm-start copies slot-by-slot only when the full
//!   shape tuple matches; mismatched slots learn during training),
//!   `domain-as-data-not-algorithm-fork` (D — the cross-game aspect is
//!   in the DATA, not the algorithm: convert episodes, delegate to the
//!   unchanged loop; no forked trainer), `one-sweep-spatial-feature-
//!   fanout` (C — 12 spatial statistics from ONE board visit via
//!   per-cell classification indices + a signed indicator + geometry-
//!   derived denominators), `deterministic-tiling-projection-tables`
//!   (A — a modelless fixed projection as construction-time tables:
//!   modulo schedule + even-spacing-with-jitter phases, same seed
//!   rebuilds the identical encoder), `latent-encode-at-capture` (A —
//!   snapshot the latent encoding at capture instead of buffering raw
//!   states: peak memory moves × `hidden_dim`, not moves × full state),
//!   `non-saturating-outcome-reward-floor` (B — raw 0/1 outcomes zero
//!   the gradient weight of every losing episode; 0.15 + 0.70 · clamp
//!   keeps losses teaching), `collect-then-resolve-snapshot-batch` (E —
//!   simultaneous-turn loops collect ALL selections against the frozen
//!   pre-tick snapshot (read-only, parallelizable) then resolve in
//!   fixed order: no information leak, order-independent outcomes), and
//!   `timeout-scalar-tiebreak-decisive-outcome` (D — a capped game
//!   falls through to a maintained-scalar (HP) tiebreak instead of a
//!   flat draw: signal density at zero maintenance cost). Corpus:
//!   **251 rules across 15 categories (A–O)**. 14 candidates dropped
//!   as duplicates/thin (scratch reuse + `legal_buf` hoisting + seed
//!   derivation + 3-accumulator sweeps + modulo zip-cycling + digest
//!   audit trail + per-tick grid-view rebuild etc. — all covered by
//!   existing rules; the grid-view rebuild went to the issue as the
//!   naive side). Mining ALSO surfaced the quadratic centroid storm +
//!   swallowed failed move + `warm_start` weight duplication + per-call
//!   dataset deep-clone + the false-premise sigmoid comment + the dead
//!   init-touch loop + the Wait-vs-continue mismatch + the wasted
//!   encodes — filed as a riir-ai issue.
//! - **Batch 33 (Research 037):** 7 rules from the `spectralquant`
//!   calibration cluster — `calibration.rs`, `rotation.rs`, `qjl.rs`
//!   (the calibration path Research 016 explicitly deferred; the
//!   cluster is bench-only today, sole consumer
//!   `benches/spectralquant_bench.rs`):
//!   `grid-over-output-thread-private-sweep` (E — grid over the OUTPUT
//!   matrix, the long `seq_len` axis swept thread-private in registers,
//!   one store per element: zero cross-thread reduction, no partials
//!   buffer, no reduce kernel), `buffer-set-struct-two-tier-dispatch`
//!   (D — all buffers a kernel family needs hoisted into ONE struct
//!   bound to the config; the preferred entry only writes uniform
//!   content, the raw entry self-allocates for cold callers),
//!   `slot-offset-partial-buffer-write` (A — update one (pos, head)
//!   slot via a computed byte offset mirroring the kernel's base
//!   index; `d_h`×4 bytes, not the whole tensor),
//!   `immutable-operand-upload-at-set-creation` (A — V^T / codebooks /
//!   sign-matrix uploaded ONCE in the buffer-set constructor; mutable
//!   buffers created empty; dispatch never sees operand bytes again),
//!   `selective-projection-effective-rank-head` (B — after an
//!   energy-concentrating transform, the projection operator runs on
//!   the top `d_eff` coords only; operator + FLOPs scale with effective
//!   rank, not ambient dim), `derived-scalar-host-computed-in-uniform`
//!   (N — config-derived scalars computed once on the host + shipped
//!   in the uniform: CPU reference and kernel consume bit-identical
//!   f32 by construction; WGSL sqrt is 2.5-ULP, not correctly
//!   rounded), `rademacher-signs-over-gaussian-projection` (B — ±1
//!   entries: one coin flip per entry, ±1.0 exact in f32 so every
//!   product is exact; conditional-negate strength reduction licensed
//!   but unrealized in the source kernel). Corpus: **258 rules across
//!   15 categories (A–O)**. ONE seed REJECTED on verification:
//!   two-stage tile→reduce is NOT what covariance.wgsl does — the
//!   producer writes no tile partials, so `n_tiles` > 1 would reduce
//!   uninitialized memory (filed as riir-ai Issue 686 H1, not a rule —
//!   encoding it would have put a bug in the corpus as wisdom). 11
//!   further candidates dropped as duplicates/thin (`offset_of!` pin
//!   tests → completion of pad-uniform-pod; flat-1D modulo decode →
//!   derive-params + token-flattened; sign-matrix bitpacking →
//!   bool-fields regime boundary; smem staging + coalescing variants →
//!   covered). Mining ALSO surfaced the layer_offset-dead-in-kernel
//!   multi-layer hazard + the `create_uniform_buffer` doc-lie + the
//!   per-dispatch bind-group family — filed as riir-ai Issue 686.
//! - **Batch 34 (2026-08-15, Research 038):** the Gemma-4 TRAINING
//!   cluster (`gemma4_cubecl_train/`: mod.rs + `backward_gpu.rs` — hybrid
//!   GPU training forward + Phase 2.5 GPU backward; the training
//!   counterpart of Batch 25's inference-side Gemma cluster):
//!   `exact-op-parity-for-analytic-backward` (N — the training forward
//!   evaluates the EXACT op the analytic backward differentiates;
//!   fast_tanh-class approximations bias every gradient), `mirror-ring-
//!   store-sliding-window-contiguity` (A — 2·sw mirrored ring makes any
//!   ≤sw window contiguous by construction; raw-index reads need
//!   `seq_len` ≤ 2·sw), `split-dispatch-expose-fused-intermediate` (D — a
//!   fused chain that discards its intermediate forces consumers to
//!   re-launch the producer; ship a split entry), `preallocated-
//!   activation-slots-inplace-save` (A — save activations into
//!   pre-sized [seq][dim] slots; the forward loop is alloc-free),
//!   `hybrid-cpu-elementwise-gpu-throughput-sync-points` (D — GPU takes
//!   only throughput-bound kernels; elementwise runs CPU-side between
//!   the 4 per-layer syncs), `pre-transposed-consumer-layout-handles-
//!   build-once` (L — backward's W^T materialized once as resident
//!   handles under the frozen-base contract), `shape-matched-sibling-
//!   weight-grads-share-input-loop` (C — wk+wv / gate+up rank-1 updates
//!   share ONE walk of the shared input), `op-major-backward-loop-
//!   inversion-batch-sequence-axis` (D — positions are free inside each
//!   backward op stage; batch each stage into one GEMM + one readback),
//!   `backward-helper-expose-shared-norm-statistic` (H — rmsnorm
//!   backward must return `inv_rms`, not hide it from the gamma-grad
//!   path). Corpus: **267 rules across 15 categories (A–O)**. TWO seeds
//!   resolved by verification: the softcap backward does NOT recompute
//!   tanh — it algebraically recovers it from the cached post-cap
//!   logits (already `sigmoid-deriv-recovered-from-output`); the
//!   dual-KV-cache "pattern" was REJECTED as wisdom — it's an avoidable
//!   2-3× duplication (hazard H9). Bug yield filed as riir-ai Issue
//!   687: mirrored-ring backward OOB panic for `seq_len` > 2·sw (P1,
//!   latent), backward-handle staleness + self-contradictory `LoRA` doc
//!   (P1), Phase 2 processing zero-gradient positions, the
//!   ~2.06M-alloc and 345,088-round-trip storms, the CPU weight-grad
//!   dominance contradicting the header's bottleneck claim, plus 8 more.
//! - **Batch 35 (2026-08-15, Research 039):** the Gemma-2 TRAINING
//!   cluster (`gemma2_cubecl_train/`: `mod.rs` + `attention_gpu.rs` — the
//!   fused causal attention kernel of Issue 430 Option B + the hybrid
//!   `LoRA` training forward/backward; the training counterpart of Batch
//!   25's inference-side cluster, with resident.rs deferred to a later
//!   batch): `mask-sentinel-matches-reduction-identity` (E — masked
//!   lanes PARTICIPATE in a block reduction by writing the identity
//!   element, and the sentinel is REWRITTEN between reductions with
//!   different identities: -inf for max, then 0.0 before the sum),
//!   `smem-staging-gated-by-cross-thread-reuse` (A — stage an operand
//!   into smem iff its within-workgroup reuse exceeds one; single-use
//!   operands stream from global in the same dot loop),
//!   `kernelize-host-chain-after-sync-batching-floor` (C — a
//!   host-serviced segment floors at 2 syncs (down+up) after all
//!   batching; kernelize it when intermediates are chain-local and the
//!   host does no work in the gap — the 52 = 26 layers x 2 proof),
//!   `block-reduction-separate-scratch-runtime-strides` (B — reduce
//!   into separate scratch (the source feeds a later pass) + strides
//!   derived from the runtime extent, never a hardcoded unroll — both
//!   failure modes shipped and failed G1 on 4090),
//!   `saturating-nonlinearity-recovery-breaks-near-cap` (N — recovering
//!   a derivative from the cached output yields exactly 0 once the
//!   output rounds to 1.0 in f32; recompute the RAW argument when
//!   saturation is reachable — the codebase makes the opposite choice
//!   at its two sites), `adapter-grad-join-mirrors-forward-delta-
//!   insertion-point` (H — additive CPU-side `LoRA` deltas on GEMV
//!   outputs mean the backward joins the adapter grad at the same
//!   point and the base transposes are immortal by construction; the
//!   merged convention silently drops the term — Issue 687 H2),
//!   `side-consumer-readback-alias-live-handle-for-dependent-dispatch`
//!   (A — read an intermediate for side consumers but hand the next
//!   dispatch the still-live handle; the re-upload burns ~53 MB/step),
//!   `quadratic-attention-intermediates-recompute-from-linear-saves`
//!   (A — save the Theta(N x d) tensors, recompute the Theta(N^2)
//!   chain in hoisted scratch; the recompute also yields the raw
//!   pre-softcap argument the saturating backward needs). Corpus:
//!   **282 rules across 15 categories (A–O)**. ONE coordinator seed
//!   KILLED by verification ("thread role rotation" is ambient fused-
//!   kernel structure, not decision content — falsification 3-for-3);
//!   Seed 2 CORRECTED (the 52 is post-batching, not pre — that's what
//!   makes the floor rule distinct). 12 candidates dropped as
//!   duplicates/thin. Bug yield filed as riir-ai Issue 688 (15
//!   hazards: the 3-way sync-count doc inconsistency, ~1.1 GB/step
//!   alloc churn from unused `_into` variants + the engine doc-lie,
//!   ~82 MB dead activation slots, the per-launch params buffer, the
//!   256-constant triplication, un-asserted `q_dim`/`kv_dim` derivation,
//!   KV-cache liveness contract, per-layer grad-buffer churn, 262 MB
//!   `EngineActivations` per step, the 2.6x handle-memory doc
//!   understatement, the h-recompute ~9.7 G MACs, and more).
//! - **Batch 36 (2026-08-15, Research 040):** the Gemma-2 RESIDENT
//!   training cluster (`gemma2_cubecl_train/resident.rs` — the Plan `481/482`
//!   all-GPU `LoRA` training path: per-position GEMV forward + batched GEMM
//!   forward + batched backward; the deferred third file of the Batch 35
//!   cluster, completing it): `constant-scale-folded-into-upload-copy`
//!   (C — a hyperparameter-derived CONSTANT scale is folded into the
//!   upload COPY so no kernel ever applies it; the backward keeps the
//!   unscaled original — fold-into-copy, never into the source operand),
//!   `side-branch-operand-residency-matches-mutation-lifetime` (D — an
//!   aux computation riding a GPU hot path keeps ITS operands
//!   device-resident at the step boundary; the hybrid CPU-LoRA path paid
//!   ~70K sync round-trips and ran 4.8x slower than pure CPU),
//!   `activation-checkpoint-as-handle-clone-deferred-batch-readback` (D —
//!   the checkpoint medium during compute is the handle clone (a
//!   refcount bump), never an inline read; the tail `read_batched`
//!   collapses 364 syncs to 26),
//!   `training-forward-shed-decode-loop-shape` (D — the per-position
//!   loop + incremental KV cache is inherited DECODE structure; free the
//!   axis by making attention whole-sequence, ~55K dispatches → ~182),
//!   `sibling-dispatches-before-any-readback` (D — queue ALL independent
//!   sibling dispatches before the FIRST readback; the interleave
//!   serializes completely; applies with DIFFERENT input buffers),
//!   `independent-cpu-work-fills-dispatch-read-window` (D — CPU work
//!   independent of the readback belongs INSIDE the dispatch→read
//!   window; the `LoRA` `param-grad` half is the canonical hoistable work;
//!   visible-but-unexploited in source),
//!   `grad-terms-merge-before-shared-link-jacobian` (H — multiple loss
//!   terms through the SAME nonlinear link must SUM at the link input
//!   with ONE Jacobian application; adding a term after the Jacobian is
//!   silently WRONG, not just slower). Corpus: **282 rules across 15
//!   categories (A–O)**. FOUR seeds falsified by verification (F4 the
//!   per-position path already caches its cos/sin table via
//!   `RopeCosSinCache` + Batch 17 explicitly dropped cos/sin-table
//!   rules; F5 the env-var A/B gate is promotion process, its kernel
//!   content already in the B35 floor rule; B2 the ping-pong swap is
//!   `needless-swap-temp` fix(2)'s loop-carried scratch; B4 the scratch
//!   hoist is a 3-rule composition) + 2 premise corrections (F3 the
//!   per-position path's "ONLY sync point" doc-lie — it issues 93,440
//!   unbatched reads; B1 q/k/v consume different inputs so fan-out's
//!   premise doesn't apply). 1 weak candidate dropped (phase-disjoint
//!   buffer aliasing — 9 KB local win). Bug yield filed as riir-ai
//!   Issue 689 (15 hazards: ~10.2M-transcendental CPU `RoPE` recompute
//!   per forward from not saving `k_post_rope` (P2, cheapest fix), the
//!   93,440-read unbatched legacy readback, the 262 MB `g_logits_all`
//!   fresh alloc + dead zero-fill per backward step, the
//!   comment-lie `g_attn_out_vecs` per-layer allocations, the
//!   nested↔flat reshape round-trip ~327 MB/step, `read_handle` fresh
//!   Vecs ~1.2 GB/step download churn with the `read_handle_into` fix
//!   substrate already in-tree, missed CPU/GPU overlap windows, the
//!   rmsnorm scratch→dest copy API asymmetry, + more).
//! - **Batch 37 (2026-08-15):** the `Q4_K` TRAINING cluster — the trainer
//!   core and weight-buffer management (`gemma4_q4k_train/`: `mod.rs` 1,389 +
//!   `weight_buffers.rs` 553; the `QLoRA` frozen-`Q4_K`-base + `f32`-LoRA-delta
//!   Gemma-4 trainer; `forward_batched.rs` + `backward.rs` deferred to Batch 38
//!   on predicted dedup pressure) — **8 rules, corpus 282 → 290**
//!   (A 52, C 31, D 50, L 10, N 17): `kv-window-device-resident-ring-append`
//!   (A — a KV cache mutated by append + consumed every position lives ON
//!   the device as a persistent ring; only the NEW row crosses the bus —
//!   append-vs-snapshot transfer), `lm-head-loss-positions-only` (D — the
//!   loss-support set is known before the forward; prompt positions' vocab
//!   GEMV + readback + save are provably dead via the backward's
//!   `d_logits[p].is_empty()` skip), `verbatim-upload-on-format-match`
//!   (N — on-disk quant format == device target format → cast the mmap +
//!   upload raw blocks; NEVER dequantize-to-f32-and-requantize — the 4090
//!   OOM class; proposed independently by BOTH agents, merged),
//!   `stream-drain-free-source-during-build` (L — drain the source per
//!   unit during a one-time build: peak host memory O(unit), not O(model)),
//!   `identical-tensor-handle-clone-zero-reupload` (A — a tensor legally
//!   absent from the export because identical to a sibling → clone the
//!   device handle (refcount), never re-upload the bytes),
//!   `sparse-gather-scale-at-read-not-table-prepass` (B — a sparse read
//!   set plus a shared unscaled consumer select the GATHER as the fold
//!   point — the deliberately inverse regime of B36's upload-copy fold),
//!   `tiled-host-transpose-before-quantize` (A — cache-block the host
//!   transpose: naive inner-c store strides 15 KB; every store a cold
//!   line), `materialized-fast-path-bulk-slice-accessor` (D — a bulk
//!   `Option<&[f32]>` accessor branches ONCE per step; the materialized
//!   arm runs direct-index + rayon, not per-row enum dispatch). SIX seeds
//!   falsified by verification (highest-per-agent yet: `mod.rs` F1 the rope
//!   tables ARE cached once — 48 layers share 2 via `rope_table_idx`; F3
//!   `LoraSite::none()` is the correct frozen-base skip, not B31's inert
//!   adapter; F4 gammas built once at construction; F5 the 3 backward
//!   constructors are field-identical — no `merkle_root`-class miss;
//!   `weight_buffers.rs` F4 the setup path is pure enqueue, 0 round-trips;
//!   F5 the GEMV path already ships packed `Q4_K` + in-kernel dequant =
//!   the B15 direction) + 2 confirmations with regime corrections
//!   (scale-at-gather is CORRECT for a sparse read set; quantize runs
//!   once — but the one-time cost is paid TWICE: forward + transposed
//!   backward handles ≈ 14 GB GPU residency on a 24 GB 4090). 1 rule
//!   dropped as dedup (`variant-dedup-constant-table-by-index` —
//!   `share-pipeline-set-across-subsystems` + B17's rope-table-drop
//!   precedent). Bug yield filed as riir-ai Issue 690 (~19 hazards:
//!   43,008 blocking reads/step on the unbatched path (the file's own
//!   Bench 433 doc), ~127 MB/step dead `lm_head` readback at prompt=127,
//!   the dual-KV-cache quadruple-write ~402 MB/step, the O(S^2)
//!   full-attention window collect ~537 MB/step/layer — pure waste,
//!   rows already contiguous, ~40 GB/step sliding-window re-upload, the
//!   2× GPU weight residency, one-time dead zero-inits 7 GB + 48 GB,
//!   two doc-lies, + more).
//! - **Batch 38 (2026-08-15, Research 042):** the `Q4_K` TRAINING cluster,
//!   second half (`gemma4_q4k_train/`: `forward_batched.rs` 1,077 +
//!   `backward.rs` 941 — the layer-major batched forward, the 43,008 to 337
//!   dispatch fix, + the per-position and batched backwards; completes the
//!   cluster, `gemma4_q4k_train/` now FULLY MINED) — **4 rules, corpus
//!   290 → 294** (A 55, D 51): `env-flag-resolved-once-not-per-dispatch`
//!   (D — `std::env::var` on the hot path takes the global env lock +
//!   allocates a `String` per lookup; one instance sits IN FRONT of the
//!   autotuner's deliberately lock-free papaya cache hit),
//!   `flat-activation-slots-as-readback-target` (A — nested
//!   `[pos] → Vec<f32>` activation slots force a scratch + scatter two-copy
//!   tail after every batched readback; the container layout IS the copy
//!   count), `transient-spike-pins-persistent-scratch` (A — a one-shot
//!   `resize(batch × vocab)` ≈ 134 MB pins a persistent scratch at ~8× its
//!   recurring max; grow-only is right for recurring sizes and wrong for a
//!   non-recurring spike), `transposed-consumption-accumulate-not-gather`
//!   (A — `z = Bᵀ·g` against row-major B: k-outer column gather at stride
//!   `rank × 4B`, up to 16× read amplification, → i-outer row accumulation;
//!   summation order changes — fine for grads, NOT for bit-identity gates).
//!   ONE subagent rule KILLED by coordinator math (zero-gradient-position
//!   elision: K/V cross-position coupling re-introduces non-zero gradient at
//!   prompt positions after the topmost layer — support queries attend to
//!   prompt keys — so only the topmost layer's pre-attention chain + the
//!   head can skip; the same flag applies to Issue 687 H3's magnitude
//!   claim). Dedup kills: the backward's per-layer `RoPE` `powf` table rebuild
//!   (third strike — B17 drop + B36-F4 + B37-F1), sibling-batch region
//!   packing (composition of superset-sized + slot-offset), resize-churn
//!   (superset-sized unapplied), rayon-attention direct-write (a 2nd
//!   instance of materialized-fast-path in the SAME file), LoRA-delta
//!   add-pass fusion (B19), the RefCell-scratch API shape (ambient in 3
//!   rules' `opt_pattern`s), the B36 `sibling-dispatches` rule (a measured
//!   NON-application — Bench 437 found the noread combo ~16% slower on
//!   Metal; B29's live-output gate explains it). Bug yield filed as riir-ai
//!   Issue 691:
//!   the headline is that BOTH backward variants have ZERO in-tree callers
//!   (the 941-LOC file is unreached, which is how its dead `x_normed`
//!   compute survived), then dead `x_normed` in both Phase 1s, 48× per-layer
//!   `RoPE` table rebuilds while the trainer caches bit-identical tables, the
//!   ~94 MB/step clone storm, ~380 MB/step fresh output-Vec churn +
//!   ~380 MB/step redundant `hidden` re-uploads (5× per layer), the 134 MB
//!   scratch pinning, the dead `attn_scores` field, 97 env lookups/step,
//!   the strided `LoRA` z-gather at 2 sites, 2 doc inconsistencies.
//! - **Batch 39 (2026-08-15, Research 043):** the `KimiK3` full-model decode
//!   forward (`kimi_k3_gpu_forward.rs`, 2,601 LOC — the largest single
//!   unmined file; hybrid CPU/GPU decode stack: GPU GEMVs + CPU elementwise
//!   glue, 7 forward entry-point variants, dual weight-update paths, MLA KV
//!   cache rewind for the Ouro loop, + the dense `SiTU` FFN helper family) —
//!   **9 rules, corpus 294 → 303** (A 60, C 32, D 53, K 5):
//!   `autotune-cache-extract-outlives-owner` (K — salvage the tuning cache
//!   across owner destruction via Arc inject/extract; re-benchmarking costs
//!   ~5-10 min, the upload ~64 ms), `append-cache-rewind-overwrite-in-place`
//!   (A — re-execution workloads overwrite, never re-append; `O(max_seq)` not
//!   `O(iters × max_seq)`)), `slow-path-bootstraps-raw-buffer-hotswap-cache`
//!   (A — first call boots via the slow path + harvests raw buffers into
//!   order-keyed slots; subsequent calls write in place, zero alloc),
//!   `entrypoint-variants-share-layer-body` (D — versioned entry points
//!   multiply, loop bodies stay shared per axis; forks diverge — observed
//!   twice in one file), `elementwise-offload-needs-device-side-anchor`
//!   (C — a GPU op sandwiched between upload + readback is compute-offload
//!   ONLY; the win is the handle chain — found independently by both mining
//!   agents), `tiny-param-buffer-rebuild-over-cache` (A — 3 f32 = 12 bytes:
//!   rebuild beats caching below a cache line),
//!   `readback-destination-passing-free-fn` (A — `dst: &mut [f32]`, not
//!   `-> Vec<f32>`; the free-fn form also splits the struct borrow),
//!   `variant-owned-output-buffer-clone-escape` (A — enum-variant-owned
//!   output buffers force per-layer-per-token clones out of the match arm;
//!   hoist to the parent + dispatch via free fns over disjoint fields;
//!   merged from both agents), `pipeline-layer-boundary-device-handle`
//!   (D — the Handle is the pipeline currency; host materialization becomes
//!   an internal detail, later fusion composes without boundary changes).
//!   ONE coordinator-verified divergence pair + the merged-both-agents rule
//!   ×2 (the anchor rule + the clone-escape rule — each found independently
//!   by both agents). Bug yield filed as riir-ai Issue 692.
//! - **Batch 40 (2026-08-15, Research 044):** the `KimiK3` GPU backward
//!   (`kimi_k3_gpu_backward.rs`, 4,332 LOC — the largest file in the crate;
//!   hybrid CPU/GPU gradient stack: transposed-GEMV backward via harvested
//!   weight-buffer slots + dense `SiTU` FFN / `MoE` / `MLA` / `KDA` backwards)
//!   — **7 rules, corpus 303 → 310** (A 64, D 54, H 14, N 18):
//!   `host-scratch-mirrors-slot-order-for-coalesced-writes` (A — the host
//!   scratch layout IS the write-call count; slot-ordered contiguity lets the
//!   batched writer coalesce adjacent device buffers), `write-plan-derived-
//!   from-cache-slots-not-recomputed` (A — the mirrored hand-maintained slot
//!   list IS the bug factory; derive the plan from the cache's own
//!   `size_bytes`), `cpu-source-builder-skips-device-roundtrip` (A — when the
//!   host owns the source bytes, the `*_from_cpu` twin deletes the per-matrix
//!   `read_one` syncs entirely; batching them is the lesser fix),
//!   `expert-loop-two-phase-split-batch-across-iterations` (H — split the
//!   per-expert helper at the CPU-dependency point and batch each half across
//!   ALL experts: `2+2·k_r` syncs → ~3), `rmsnorm-backward-gamma-scope-
//!   correction-term` (N — gamma multiplies ONLY the direct `dy` term, never
//!   the normalization correction; the slip is invisible at near-unity gamma
//!   init under percent tolerances — found LIVE wrong in the production `MoE`
//!   backward while the same file carried the correct form),
//!   `per-position-scalar-factored-gemv-collapse` (D — factor data-dependent
//!   per-position scalars out of GEMVs by linearity, but keep scaled and
//!   unscaled consumers on separate accumulators — the fused form is the live
//!   `d_attn_w` wrong-math bug), `foreign-accumulator-bridge-copy-hoist-to-
//!   outer-loop` (A — bridging to a foreign crate's whole-struct gradient
//!   accumulator via full-state copy-in/out per token is a gradient
//!   ping-pong; hoist the bridge to the outer loop). TWO critical wrong-math
//!   bugs (MLA `d_attn_w` spurious attention-weight factor + `MoE` latent
//!   norm gamma over-application) + a NaN-vacuous parity suite + one
//!   actively-failing KDA test filed as riir-ai Issue 693.
//! - **Batch 41 (2026-08-15, Research 045):** the `KimiK3` GPU backward
//!   sequence — the PRODUCTION full-model backward caller (`kimi_k3_gpu_backward_sequence.rs`,
//!   1,328 LOC) + the write substrate it updates through (`weight_buffer_cache.rs`,
//!   459 LOC) — **6 rules, corpus 310 → 316** (A 69, E 18):
//!   `concurrent-disjoint-write-buffer-parallelizes-uploads` (E — native
//!   backends do not serialize disjoint `write_buffer` calls; 256-aligned
//!   chunks from rayon workers, ~8× per-layer update parallelism),
//!   `aliased-slot-allocate-replace-fallback-severs-binding` (A — the B39
//!   boundary condition: an aliased slot's allocate-replace fallback orphans
//!   every subsequent write; `cuda_backend` hits it on EVERY update),
//!   `pool-slot-write-coalescing-owns-only-declared-ranges` (A — the writer-side
//!   complement of the B40 build-side rule: discover contiguous runs at
//!   runtime, never write gap bytes owned by other pool tenants),
//!   `pool-reservation-token-outlives-raw-view` (A — the raw view is not
//!   ownership; drop the wrapper `Handle` and the pool re-issues the range
//!   under the still-alive view), `gpu-type-bridge-borrows-not-clones` (A —
//!   the CPU→GPU type seam borrows activations instead of double-cloning
//!   field-identical types), `superseded-primitive-machinery-zombie` (A —
//!   retire support machinery with its consumer; the tell is a test-only
//!   caller — the zombie LM-head pipeline uploads ~640 MB/step with zero
//!   production readers). Issue 693 H1/H2 production routing PINNED (both
//!   wrong-math bugs inherited via `mla_backward_gpu_batched`/
//!   `moe_backward_gpu_batched`); new hazards filed as riir-ai Issue 694.
//! - **Batch 42 (2026-08-15, Research 046):** the Gemma-2 D2F block-causal
//!   decode cluster — `gemma2_d2f/mod.rs` (1,579 LOC, the CPU-orchestrated
//!   hybrid decode loop) + `gemma2_d2f_sc.rs` (800, Self-Conditioning,
//!   Plan 250) — **4 rules, corpus 316 → 320** (A 70, B 46, D 56):
//!   `refine-loop-invariant-prefix-elision` (D — the denoise loop re-forwards
//!   the provably-invariant prompt prefix every step; masking closes it,
//!   mutation-set closure proves it), `softmax-entropy-via-shift-identity`
//!   (B — `ln(e_t)` = `logit_t` − max, ONE transcendental for the whole entropy
//!   sum), `runtime-identity-gate-cached-verdict-elides-side-projection`
//!   (D — content-classified inertness cached write-through at the mutation
//!   boundary gates whole-stage elision; the gate must cover producer AND
//!   consumer), `expected-embedding-transposed-gemv-resident-head` (A — the
//!   soft-token expectation is the tied `lm_head`'s transposed GEMV on the
//!   already-resident table, not a host mirror walk). Hazards filed as
//!   riir-ai Issue 695 (KV GPU→CPU→GPU round-trip 1.7 GB/block, invariant-
//!   prefix recompute ~227 GMACs, 46.6K blocking reads, hardcoded softcap
//!   divergence, truncate panic, the unreachable trained-SC path).
//! - **Batch 43 (2026-08-15, Research 047):** the Weaver speculative-decode
//!   GPU corrector cluster — `weaver_gpu_corrector.rs` (1,449 LOC, Plan 436
//!   Phase 4 `T4.1`: `GpuWeaverCorrector` + batched scratch + the per-depth and
//!   batched `correct_marginals` entry points) + `weaver_gpu_dflash.rs` (576
//!   LOC, `T4.2`: the `DFlash` draft orchestration wrapper) — **5 rules, corpus
//!   320 → 325** (A 73, B 47, D 57):
//!   `mutable-input-scratch-residency-write-per-call` (A — per-iteration
//!   `create_from_slice` input churn → scratch-resident handles written per
//!   call; the file's own batched path is the positive contrast),
//!   `topk-select-nth-over-full-vocab-sort` (B — O(V log V) full pair sort →
//!   `select_nth_unstable` partition + K-prefix sort over a u32 index buffer),
//!   `post-scatter-renorm-support-set-only` (A — dense re-sum + rescale after
//!   a K-scatter → sum-during-scatter + support-only rescale, bit-identical),
//!   `stack-array-row-view-bridge` (A — `Vec<&[f32]>` heap collect → a
//!   fixed `[&[f32]; 64]` stack bridge, the speculative-decode idiom at 5+
//!   in-tree sites), `empty-slice-sentinel-launders-shape-violation` (D — a
//!   defensive `&[]` sentinel type-checks past the shape contract and lands
//!   in a swallowed `Err`; slice hard + `debug_assert!`). 23 falsifications
//!   (Issue 468 P0/P2 already shipped several rules' fixes — the per-depth
//!   vs batched paths sit side by side as naive/opt contrast). Hazards filed
//!   as riir-ai Issue 697 (the 3-layer silent no-correction chain, the
//!   release-mode embedding OOB window, ~29 alloc requests/call, test-only
//!   reachability with no CI row).
//! - **Batch 44 (2026-08-15, Research 048):** the Weaver GPU kernel
//!   substrate + the `KVarN` quantization cluster — `weaver_gpu/mod.rs`
//!   (1,657 LOC — the 19-dispatch kernel implementations backing the Batch
//!   43 corrector: `GpuWeaverWeights` upload/download, pos-emb add, batched
//!   `SwiGLU`, causal `MHA` with smem softmax, indirect embedding gather,
//!   dot-per-row, softmax-over-K with the Issue 468 `P0` `probs_offset`
//!   write-shift) + the `kvarn/` dir (`kvarn_zone_weights.rs` +
//!   `kvarn_dequant.rs` + `kvarn_quant.rs` — the game-side CPU reference +
//!   zone-scale registry for `KVarN` KV-cache quantization, post-Issue-042
//!   orchestration over canonical `katgpt-kv` primitives) — **9 rules,
//!   corpus 325 → 334** (A 75, B 49, C 33, D 59, E 19, L 11):
//!   `row-dot-split-inner-axis-block-reduce` (E — one thread per output row
//!   pins 128 threads to a single workgroup at production + 16x read
//!   amplification; split the reduction axis instead — workgroup per row,
//!   lane-strided, block-reduced), `fuse-indirect-gather-into-dot-consumer`
//!   (C — a materialized `D*K*h` gather whose only consumer re-reads it in
//!   a dot; gather in-loop via the device-resident index array),
//!   `share-cube-size-const-between-kernel-and-launcher` (D — the
//!   workgroup-size literal restated in 4 kernels vs 6 launchers is a
//!   silent stride contract; one shared const), `stash-kernel-internal-
//!   intermediate-in-smem` (A — a global write + re-read nobody outside
//!   the kernel observes; stash in smem + drop the buffer),
//!   `hoist-bit-address-out-of-packed-row-sweep` (B — the per-row unpack
//!   call re-derives an identical bit address; decompose once per
//!   position), `fold-tile-transpose-into-dequant-scatter` (L —
//!   materialize-then-transpose = 2 reads + 2 writes per element; scatter
//!   the consumer layout at production), `bit-cursor-unpack-to-width-
//!   specialized-byte-batch` (A — a software bit cursor re-derives what a
//!   per-width arm makes compile-time-constant), `single-sweep-minmax-
//!   for-quant-scale` (B — two folds scan the group twice for the (lo, hi)
//!   pair every RTN scale prep needs; one joint sweep),
//!   `serialize-container-canonical-order` (D — `HashMap` iteration order
//!   leaks into serialized bytes, breaking commitment stability; sorted
//!   keys). 22 falsifications (the file's own documented trade-offs —
//!   thread-0 softmax at tiny K, naive-per-head MHA at `seq_len` <= 16 —
//!   plus 8 dedup kills + the canonical `katgpt-kv` positives). Hazards
//!   filed as riir-ai Issue 698 (the legacy `KVarN` round-trip drops the
//!   `VarN` inversion — quantized scales discarded at `kvarn_quant.rs` L160
//!   — masked by vacuous tolerances; the CPU/GPU auto-route doc-lie; the
//!   drifted unpack fork corrupting non-pow2 widths; `GpuWeaverScratch`
//!   dead with known-wrong sizing; the dot-per-row perf cliff).
//! - **Batch 45 (2026-08-15, Research 050):** the SHINE hypernet
//!   meta-LoRA cluster, first half (`hypernet/meta_lora.rs` 732 — the
//!   CPU reference bidirectional forward for memory extraction, Plan
//!   098/104 + `hypernet/train_meta_lora.rs` 601 — the CPU
//!   finite-difference Meta-LoRA trainer) — **3 rules, corpus 334 →
//!   337** (A 77, B 50): `scratch-depth-matches-shape-invariance` (A —
//!   a layer x position x head nest allocates each scratch at the
//!   deepest convenient point though every shape is bound by
//!   (`seq_len`, dim) fixed for the whole call; the tell is two
//!   same-shaped residuals at different depths), `softmax-denominator-
//!   hoist-out-of-weighted-v` (B — `scores[t] / sum_exp` recomputed
//!   `head_dim` times per key position inside the PV loop; the in-tree
//!   optimized twin is `attention.rs` `attention_head` pass 3),
//!   `frozen-cooperand-hoist-out-of-perturbation-loop` (A — the FD
//!   sweep rebuilds the frozen co-operand per candidate and the callee
//!   re-derives its own sweep-invariants per eval; materialization
//!   cadence must match mutation cadence). Hazards filed as riir-ai
//!   Issue 699 (`lora_buf` sized by model `lora_rank` vs adapter rank →
//!   silent `get_unchecked_mut` OOB with real configs' `lora_rank`: 0;
//!   GQA `kvd < out_dim` OOB; `apply_meta_lora` dead + 16x scaling +
//!   transposed A/B layout vs the production path; FD clone storm
//!   quantified; baseline ctx-8 hardcode + single-sample GOAT
//!   estimator).

//! - **Batch 46 (2026-08-15, Research 051):** the `MaxSim` cluster
//!   (`maxsim/dispatch.rs` 449 + `maxsim/spectralquant.rs` 581 + both
//!   WGSL kernels — size-gated CPU/GPU `MaxSim` late-interaction scoring
//!   Plan 085 + the fused `SpectralQuant` dequantize+`MaxSim` single-pass
//!   scorer T5) — **3 rules, corpus 337 → 340** (A 79, D 60):
//!   `size-gate-covers-every-cost-factor` (D — a CPU/GPU size gate
//!   that reads a partial work product `ld * batch` while CPU cost
//!   scales with `lq * ld * dim * batch` silently transposes the
//!   crossover at unbenched shapes; the pinned constant bakes in BOTH
//!   sides' current overheads — SIMD-izing the CPU path or removing
//!   the per-call GPU alloc floor each invalidate it; two in-tree
//!   instances: maxsim `ld*batch` + `gpu_m2p` `rows*num_heads`),
//!   `declared-max-array-taxed-per-thread-under-runtime-bound` (A — a
//!   WGSL function-scope array sized to a hard max `array<f32, 256>`
//!   but bounded at runtime by `d_eff` pays the DECLARED max in
//!   indexable thread-local per thread; pipeline override constants
//!   make the bound compile-time per class),
//!   `shared-operand-side-pair-batch-index-decomposition` (A — a
//!   `[batch × lq × d_h]` pair API bakes per-pair replication of the
//!   shared query side into the batch geometry; decompose the launch
//!   index `(query_idx, doc_idx)` so the shared side crosses the bus
//!   once — share-by-indexing, not replication). 14 falsifications
//!   (per-call buffer churn = documented trade-off + B33/B43
//!   families; `d_eff` scoring = B33 selective-projection;
//!   one-thread-per-pair = B44 row-dot-split regime; unpack bounds
//!   already match scoring bounds — the hypothesized gap does not
//!   exist). Hazards filed as riir-ai Issue 700 (`validate()` never
//!   checks `compressed_k`; the word-vs-byte `bitstream_offset`
//!   doc-lie; the documented-but-unenforced "+1 padding" one-word
//!   OOB read executing in tests; unchecked `d_eff` ≤ 256 and
//!   `d_eff` ≤ `d_h`; the scalar-CPU-fallback substrate-first
//!   violation while `katgpt_core::simd::maxsim_score` sits one
//!   feature-forward away; the bench's own crossover finder
//!   warning against the shipped constant; zero production
//!   callers).
//! - **Batch 47 (Research 052):** the SHINE hypernet second half
//!   (`hypernet/shine.rs` 402 + `routing.rs` 268 + `types.rs` 269 +
//!   `m2p_transformer.rs` 611 + `gpu_m2p.rs` 568 +
//!   `kernels/m2p_attention.wgsl` 173 — the end-to-end
//!   context-to-LoRA pipeline + expert routing + the M2P
//!   alternating-attention CPU reference + its size-gated GPU
//!   dispatch twin; completes the hypernet module alongside Batch
//!   45's meta-LoRA pair) — **3 rules, corpus 340 → 343** (B 51,
//!   D 61, E 20): `per-key-projection-hoist-out-of-query-sweep`
//!   (B — K/V projections re-derived inside the query loop
//!   multiply projection work by seq; the reference twin projects
//!   all QKV once at sweep entry), `multi-head-out-proj-consumes-
//!   head-concat` (E — the attention out-projection needs the head
//!   concat; a per-head thread degenerates to scalar-times-rowsum,
//!   masked by identity-weight fixtures), `shared-producer-prefix-
//!   extract-once-per-input` (D — two sibling methods sharing an
//!   expensive pure producer pay it twice under a composed call;
//!   extract once, thread the artifact). ~17 dedups (the
//!   workgroup(1,1,1) shape = B44/B23/B28 composite; per-column
//!   weight re-upload + round-trip = B33/B28/B26/B40; the size gate
//!   = B46's pre-recorded 2nd instance; the scores/column/hidden
//!   Vec churn = B45 depth instances; the powf freq table =
//!   3rd-strike cos/sin; routing norms = norm-decoupled; the shape
//!   metadata = derive-shape-from-payload). Hazards filed as riir-ai
//!   Issue 701 (WRONG out-projection math in both WGSL kernels AND
//!   the CPU fallback, masked by identity fixtures + a print-only
//!   pseudo-parity check; release-mode stack OOB at default
//!   `head_dim` 512 vs the `[f32; 128]` array; WGSL `array<f32, 256>`
//!   scores overflow at seq > 256; the ~251 GB default memory grid
//!   and the triple-inconsistent 13/15/19H provision constant;
//!   structurally-dead zero modules at toy scale; row-attention
//!   dispatch exceeding the 65535 workgroup limit).
//! - **Batch 49 (Research 055, the Bench-686 measured-lesson distillation):** 1 rule,
//!   corpus 343 → 344 (A 80) — `runtime-arg-loop-bound-demotes-arrays-to-local-memory`
//!   (A — a correctly-sized per-thread array whose per-element
//!   loops are bounded by a RUNTIME kernel arg that is constant at
//!   every call site is demoted to local memory: no unroll, no
//!   register form for dynamically-indexed arrays; hoist the bound
//!   to a compile-time const + unroll — measured −18.4% kernel at
//!   ppl bit-exact; the paired A/B that CAUGHT it is part of the
//!   rule: trading L1-hot global reads for more local-memory
//!   traffic is a LOSS).
//! - **Batch 51 (Research 057):** 3 rules, corpus 344 → 347 (A 81, D 62, N 19) —
//!   the FINAL riir-gpu file-mining batch (`kimi_k3_gpu_backward_batched.rs` +
//!   `attention_q8kv_cubecl.rs`): batched-read-slot-membership-decided-at-launch (D),
//!   unpack-loop-indexed-by-container-word (A), dequant-placement-by-component-
//!   traffic-share (N). Declares the riir-gpu corpus COMPLETE (~115 files mined).
//! - **Batch 52 (measured-GOAT distillation, riir-ai Issue 708 H4):** 1 rule, corpus
//!   347 → 348 (D 62 → 63) — phase-split-per-position-data-liveness-row-storage (a
//!   Phase-A loop writing a single-row buffer per position followed by a Phase-B
//!   per-position consumer only preserves the LAST write; fix = `[seq_len × dim]` row
//!   storage. Distinct from B26/B27 uniform staleness — `DATA` liveness, not values).
//! - **Batch 54 (measured-GOAT distillation, riir-train Plan 339 T1.5 / Bench 470):**
//!   1 rule, corpus 348 → 349 (B 51 → 52) — regularizer-above-noise-floor-coupled-
//!   to-damping-cap (B — a reduced-precision matrix-polynomial iteration on a
//!   rank-deficient Gram gets spurious null-space eigenvalues from SIGNED per-GEMM
//!   format noise (~1e-3 TF32), not from the eps regulariser: eps must sit ABOVE the
//!   noise floor AND below the damping budget (eps <= gamma-1, asserted in code), and
//!   the repair is a second FULL-schedule pass with the Gram re-formed from the DATA
//!   iterate — each knob alone measured FAIL; guard corollary: `t < 1e-20` is false
//!   for NaN, write `!t.is_finite() || t < threshold`).
//! - **Batch 55 (measured-GOAT distillation, riir-ai Issue 716 / Bench 691):** 2 rules,
//!   corpus 349 -> 351 (N 19 -> 20, D 63 -> 64) - outlier-span-escapes-
//!   quantization-via-lossless-sidecar (N - 1-2 massive-activation channels in a
//!   32-block collapse non-outlier neighbors 58x; the KNOWN outlier span (attention
//!   sinks, known a priori) escapes quantization into a lossless f32 sidecar, dead
//!   q8 rows keep strides fixed, 6850x quality at 0.96x perf; Hadamard rotation
//!   loses on BOTH axes - nonzero residual ~MA/16 + strictly larger kernel cost)
//!   and buffer-length-derived-from-allocation-not-binding (D - cubecl derives the
//!   kernel-visible length from `handle.size_in_used()`, NOT the bound `BufferArg`
//!   length; two root-caused occurrences: over-length bindings consume FOLLOWING
//!   rows (Bench 642 rmsnorm, fix = both-ends trims) and zero-length-over-a-real-
//!   handle reads the WRONG buffer (Bench 691 dummy sidecar, fix = real 4-byte
//!   allocation floor-dividing to zero rows); probe corollary: a benign-case-only
//!   probe is worse than no probe).
//! - **Batch 56 (measured-GOAT distillation, riir-ai Issues 719 + 714/720):**
//!   2 rules, corpus 351 -> 353 (A 81 -> 82, D 64 -> 65) -
//!   concurrent-context-init-needs-process-wide-lock (D - the
//!   init-outside-publish-on-success pattern does NOT serialize construction:
//!   every first-caller that misses the fast path runs the full driver-level
//!   sequence (Instance/Adapter/Device + server registration) concurrently,
//!   which wedges NON-DETERMINISTICALLY at the driver level on
//!   4090/Windows/Vulkan (7-thread race: 4 wedged forever inside
//!   `request_device`, another run 1/7 inside `init_device`; a parallel test
//!   suite froze with every worker blocked behind first-call init) AND every
//!   loser leaks its device + server for process lifetime (no deregistration);
//!   the cross-path half: multiple constructors each with their OWN cache
//!   running the SAME sequence need ONE shared process-wide init Mutex with a
//!   double-checked fast path, poison de-fanged, failure not cached) and
//!   superseded-format-constructor-live-fields-only (A - a constructor that
//!   accepts an optional second weight format while unconditionally uploading
//!   the primary f32 set uploads DEAD WEIGHT (~9.7 GiB device stacking on a
//!   ~1.4 GiB `Q4_K` set; 14,089 -> ~3,500 MiB peak); the dead-set proof is a
//!   DISPATCH-SITE audit, never the constructor alone; fix = tiered
//!   live-fields-only variants (`skipped()` / `norms_only()`) with zero-size
//!   placeholder TRIPWIRES (accidental bind fails validation loudly), wired
//!   automatically in the shared constructor; the class shipped twice
//!   back-to-back (Issue 714 cubecl half-skip, Issue 720 the WGSL `Q4_K`
//!   constructors) - bit-identical logits, -10.3 GiB device + -2.2 GiB host).
//! - **Batch 57 (measured-GOAT distillation, riir-ai Issue 715):**
//!   2 rules, corpus 353 -> 355 (A 82 -> 83, D 65 -> 66) -
//!   smem-slot-role-reversal-needs-barrier (A - a shared-memory scratch
//!   whose slot roles reverse between phases needs a barrier at each
//!   reversal: (a) every thread reads the reduction result `smem[0]`, then
//!   thread 0 overwrites that slot with its weight - a slow warp reads a
//!   weight as the tile max and threads disagree on the softmax shift,
//!   length-independent; (b) the last phase reads every `smem[p]` while the
//!   next tile's first phase writes `smem[tid]` with no barrier on the loop
//!   back-edge (reachable only `n_tiles >= 2`); measured 27B / 256-token
//!   probe: NONDETERMINISTIC before (first divergent token 71),
//!   BIT-IDENTICAL (3/3) after; barriers in BLOCK-UNIFORM control flow; the
//!   fix must be audited against every implementation twin - the `CubeCL`
//!   fix did not reach the `cudarc` arm, which dispatches hand-written CUDA)
//!   and tile-loop-back-edge-needs-multi-tile-bitexact-gate (D - a loop that
//!   trips twice only past a size threshold hides every back-edge defect
//!   from fixtures that fit inside ONE iteration: every prior decode test
//!   used `n_positions <= 200` < the 256-wide tile, so the back-edge never
//!   ran; the gate needs a fixture crossing the boundary (`n_positions=300`
//!   -> `n_tiles=2`) AND a bit-exact reproducibility assert over repeats - a
//!   tolerance-only assert cannot see a race in the low mantissa bits;
//!   validated BOTH directions: fails run 0 with `max_diff=0.0586`
//!   barriers-removed, passes with them; second instance: the `q8kv`
//!   multi-tile path, Issue 710 H10).
//! - **Batch 58 (the `ternary_deltanet` sibling-WIP unlock, file mining):**
//!   8 rules, corpus 355 -> 363 (A 83 -> 86, D 66 -> 70, E 20 -> 21) —
//!   `ternary_deltanet_gpu_forward_cudarc.rs` (5,691 lines, the raw-CUDA
//!   decode-forward twin — the LAST never-mined riir-gpu orchestration
//!   file; unblocked when the Issue 717/715 work landed; the `CubeCL` twin
//!   stays blocked on Issue 721 T3, which rewrites its chunked-recurrence
//!   kernels): async-htod-host-source-outlives-copy (A - an async `HtoD`
//!   memcpy reads the host pointer at GPU-COMPLETION time; a stack local
//!   dangles and the GPU silently reads the dead stack slot — persistent
//!   host staging fields + `slice::from_ref`; the host-side complement of
//!   the readback-timing family), captured-graph-bakes-buffer-addresses
//!   (D - state mutators split into address-stable (in-place `memset_zeros`
//!   (the graph SURVIVES) vs identity-changing (alloc/free; the graph
//!   MUST be dropped for lazy re-capture, else replay runs against freed
//!   pointers), output-buffer-rotation-pool-multi-forward (A - K pipelined
//!   forwards writing the same output field: swap the field's handle with
//!   pool[i] per dispatch — an O(1) ownership exchange; ONE sync, K+1
//!   position-aligned reads; the field KEEPS the last buffer between
//!   cycles so the next cycle's out[0] is the live pre-speculation
//!   prediction), mid-pipeline-device-alloc-is-implicit-sync (D -
//!   `cuMemAlloc` can implicitly synchronize the device: an allocation
//!   inside a no-intermediate-sync dispatch sequence is a HIDDEN BARRIER;
//!   pre-allocate the whole cycle at construction sized to the max K),
//!   capture-region-dispatch-only (D - the captured region records GPU
//!   work ONLY: host effects run ONCE at capture and NEVER on replay
//!   (a baked `pos += 1` freezes position silently); syncs/dtoh are not
//!   capturable; preconditions: event tracking off BEFORE any allocation,
//!   non-default stream, pre-upload), occupancy-grid-cap-refuted-uncapped-
//!   default (E - the Issue 705/Bench 684 measured negative, never
//!   distilled at resolution time: the occupancy-derived cap measured a
//!   1-2% LOSS at every size; wavefront locality beats the wave tail; the
//!   cap stays env-gated as measurement apparatus), write-before-read-
//!   state-skips-checkpoint (A - a speculative checkpoint backs up ONLY
//!   the read-modify state; write-before-read append-only caches (KV at
//!   position p) are repaired by re-execution itself — async dtod, ZERO
//!   CPU-GPU sync, host bookkeeping snapshotted by the caller), and
//!   same-input-gemv-segments-single-launch (D - same-input GEMV groups
//!   batched into ONE multi-segment launch, at most 4 segments; nsys
//!   measured 497 GEMV launches/token with m=48 projections at the ~5.6
//!   microsecond latency floor; pad unused slots with segment 0's pointers
//!   (m=0 matches no rows); the generic middle between shared-upload-N-
//!   launches and the bespoke fused pair). Mining yield filed as riir-ai
//!   Issue 722 (13 hazards: the `spec_restore_buffers`/`spec_unrotate`
//!   doc-lie (referenced but nonexistent); `set_lora` release-mode silent
//!   no-op; capture-error-path stream leak; conv1d backward full-buffer
//!   download to recover host-owned data; attention-backward per-token
//!   alloc storm; uncached `lm_head` `dequant_transpose` per backward call;
//!   plus 7 more).
//! - **Batch 60 (measured-GOAT distillation, riir-train Issue 460):**
//!   1 rule, corpus 363 -> 364 (N 20 -> 21) -
//!   pre-gate-recovery-by-division-cancels-algebraically (N - a backward
//!   that needs the pre-gate activation of a multiplicative gate
//!   (`gated = g * pre`, `g = sigmoid(W_g · h)`) must never reconstruct it
//!   by dividing the saved gated output by the gate: sigmoid underflows
//!   to a HARD ZERO in f32 below ~-88 pre-activation and the
//!   reconstruction is `0/0` NaN - silent (the forward stays finite; the
//!   NaN surfaces steps later as an opaque optimizer failure) and common
//!   (4 of 10 real sequences at random init, 4 of 5 seeds; measured
//!   Bench 475, fixed katgpt-rs `e2853fbf`: 6/10 -> 10/10 usable
//!   backward passes); the division is unnecessary because every
//!   gradient consumer multiplies the pre-gate value by gate-derived
//!   factors - `pre * g * (1-g) == gated * (1-g)`, the factor cancels,
//!   bit-identical wherever the old form was finite; where `g == 0` the
//!   pre-gate value is information-theoretically unrecoverable (`gated`
//!   is 0 too - `0/0` carries no information): guard public
//!   reconstructions to 0.0 and SAVE the true value at checkpoint
//!   boundaries; detection tell: the NaN localizes to the single weight
//!   matrix consuming the reconstruction while every post-gate-reading
//!   gradient stays clean).
//! - **Batch 63 (2026-08-19, the `CubeCL` twin unlock — Plan 009 / Research
//!   071):** 3 rules from `ternary_deltanet_gpu_forward.rs` (5,432 lines,
//!   the `CubeCL` decode-forward orchestration twin; the 721-closeout
//!   unblock fired). Two-lane mining, Lane B returned ZERO candidates
//!   (structurally empty: the chunked-recurrence/tree-verify kernels live
//!   in sibling modules already mined at B28, and the spec-decode
//!   mechanisms at B58 from the cudarc twin). Lane A's 3 (coordinator-
//!   verified distinct against the B58/B9/B19/B27/B39 rule texts):
//!   same-input-gemv-weight-concat-at-upload (D 70 -> 71 - N same-input
//!   GEMVs merged by baking the weight CONCAT into the upload; the stock
//!   kernel dispatches it, offset-consumers pay zero extra dispatches,
//!   separate-buffer consumers pay ONE split; the substrate-constrained
//!   third branch of the merge family — B58's segments need a
//!   multi-segment launcher, B15's fusion needs a bespoke kernel, this
//!   needs only frozen weights; Issues 642 F3/F2 + 648 F9);
//!   deferred-residual-fused-into-next-layer-norm (C 33 -> 34 - the
//!   post-FFN residual add deferred ACROSS the layer-loop boundary into
//!   the next iteration's fused add+norm kernel, -63 dispatches/token,
//!   first-layer + tail special cases, the composition ladder documents
//!   B19's producer-side fusion as the better rung; Issue 645 T3);
//!   reset-state-in-place-fill-never-realloc (A 86 -> 87 - sequence reset
//!   via in-place zero-fill kernels over process-lifetime buffer identity,
//!   never re-allocation (128 `create_from_slice` per reset fragmented the
//!   pool to SIGKILL under the Go arena); cursor-bounded caches skip even
//!   the fill; Plan 528 T0.2). 11 Lane-A dedup drops + 24 Lane-B
//!   examinations all mapped to covering rules. Hazard yield filed as
//!   riir-ai Issue (the dead wk/wv device upload, dual/triple weight
//!   residency, the ResidualAdd-as-copy doc-lie, the un-ported B58
//!   rotation pool, the aliasing-UB probe default-off).
//! - **Batch 70 (2026-08-22, Plan 020 — the arxiv barrier-free-sync paper
//!   distill, the Research-076 intake channel):** 3 rules, corpus 367 ->
//!   370 (B 52 -> 53, D 71 -> 73) from arXiv:2608.13757 (Sung, Shyamsunder,
//!   Zhang, Kroening, Choi — barrier-free semaphore allocation for
//!   multi-engine accelerators, CGO '27; Lean 4 bisimulation ~15K lines
//!   0 admitted; measured on real Trainium 2): barrier-free-per-
//!   dependency-wait-from-loop-counters (D — no all-worker barriers at
//!   iteration boundaries; monotone entry counters + the closed-form
//!   wait thresholds of the three allocatable cases; 10-45% latency cut,
//!   3.3x sync-bound microbench), shared-counter-decode-producer-progress
//!   (D — one shared counter per (loop, worker) decoded as `N*r - (N - K)`
//!   under in-order retirement; the per-instruction/per-loop/per-engine
//!   granularity ladder + LICM/CSE reductions), masked-branchless-wait-
//!   collapse (B — the no-wait OR threshold disjunction collapsed into a
//!   single masked threshold via a 0/1 `isGT` multiply; the branchless-
//!   select mechanism on a sync predicate). Honest applicability encoded:
//!   wgpu/CubeCL/Metal expose no fine-grained semaphore (cudarc CUDA
//!   events, host orchestration loops, worker threads are the live
//!   surfaces); the 3.3x is Trainium-specific evidence, the rules encode
//!   the mechanism; the fallback-to-barrier subset is stated in-rule.
//!   Record: Research 080.
//! - **Batch 71 (2026-08-24, Plan 022 — the oMLX external distill, Research
//!   082 consumed / Research 083):** 2 rules, corpus 370 -> 372 (A 87 -> 88,
//!   D 73 -> 74) from jundot/omlx (Apache-2.0 Python/MLX Apple-Silicon
//!   serving engine; the SECOND external batch + first live-repo source):
//!   rotated-ring-undo-via-rebind-reference-snapshot (A — a rotated ring
//!   cache cannot trim, so a rejected speculative verify leaves PHANTOM
//!   tokens; exact undo via the concat path's rebind contract: stash
//!   pre-update attribute refs + staged inputs, restore + replay the
//!   confirmed prefix; thread-local arming; detach-copy only where the
//!   path may setitem in place) + gpu-api-thread-affinity-snapshot-metrics
//!   (D — a background enforcer never calls GPU runtime APIs during active
//!   requests: snapshot (executor-published cached sample + kernel ledger),
//!   flag-and-defer (atomic flag; the clear runs on the inference thread at
//!   the next step boundary through the synchronized path — the
//!   callback-drain SIGABRT class merged in as the justification), cache-once
//!   for immutable device caps). The Research 082 pool's #4 MMA kernel was
//!   killed as B16 dedup (validation evidence only) + #5 Laguna fusions dropped
//!   (`optimizations.py` read at mine time: a hardware-detection stub, the
//!   fusions are not in it). Record: Research 083.
//! - **Batch 72 (2026-08-26, Plan 085 — the frguthmann/NVIDIA external
//!   distill, Research 084):** 1 rule, corpus 372 -> 373 (A 88 -> 89) + 1
//! - **Batch 77 (2026-08-27, lucebox PR #625 external distill, Plan 035 /
//!   Research 091):** 6 rules, corpus 373 -> 379 (A 89 -> 90, C 34 -> 35,
//!   D 74 -> 76, G 19 -> 20, M 9 -> 10) + 1 B63 rider.
//!   spec rider. skip-unchanged-writes-via-dirty-encoding (A — per-element
//!   write elision on ping-pong state buffers; dirty = plain + 2 piggybacked
//!   on the encoding; lazy alternate-rotation cleanup; REGIME-GATED:
//!   sparse ~20% win / dense measured LOSS). Rider: l2-grouped-tile-schedule
//!   gained the NVIDIA applicability conditions (VRAM-limiter, L2 hit below
//!   80%, footprint overlap; N=16; Morton loses; 47%/63->86% evidence) — the
//!   thread-group-tiling candidate itself KILLED at dedup as that rule's
//!   positive instance (the B66 rider precedent).
//!   Record: Research 084.
//! - **Plan 003 (2026-08-14):** router ↔ corpus wiring. `KERNEL_CATEGORIES`
//!   maps each of the 15 corpus categories to its `rule_ids`; the
//!   `KernelExpertRouter` now uses 15 category-keyed experts (one per category)
//!   instead of 4 placeholder identity-vector experts.
//! - **Batch 79 (2026-08-27, BAIR K-Search + cubek-matmul external
//!   distill, Plan 037 / Research 093):** 5 rules, corpus 379 -> 384
//!   (A 90 -> 91, D 76 -> 78, E 21 -> 22, G 20 -> 21).
//!   `first-order-affine-recurrence-associative-scan` (G -- the affine
//!   recurrence as the pair-space monoid, `O(log N)` prefix scan vs `O(N)`
//!   sequential; spec-seeded, search-realized; REGIME: the combine cost
//!   picks the strategy, scalar pairs scan / dxd state chunks;
//!   REL-ERROR gate required, tree-order association is never
//!   bit-identical); `plane-specialization-load-compute-role-split`
//!   (E -- comptime `PlaneFlowPartitionRule` + runtime plane-id branch;
//!   load-only planes raise occupancy without accumulator registers;
//!   never specialize WITHIN a plane; underperforms on Metal);
//!   `vectorized-smem-write-no-bank-padding` (A -- REFUTATION:
//!   line-sized vector stores distribute across banks, the +1 pad
//!   lineage is unnecessary; `SwizzleMode` is the modern complement,
//!   not padding); `ordered-lhs-skip-smem-fragments-as-second-buffer`
//!   (D -- the register fragments ARE the second Lhs buffer; ordering
//!   discipline replaces inter-plane sync; the sequential-halves
//!   allocation correction rides); `interleave-loads-between-tile-matmuls`
//!   (D -- asynchrony is INDUCED by instruction-stream placement between
//!   tile matmuls via `StageEventListener` on `TileMatmulCompleted`;
//!   clean load/compute stage separation REMOVES the overlap). Record:
//!   Research 093 / Plan 037.
//! - **Batch 80 (2026-08-27, WebGPU dispatch-overhead characterization
//!   distill, Plan 038 / Research 094):** 3 rules, corpus 384 -> 387
//!   (C 35 -> 36, D 78 -> 79, E 22 -> 23). Source: arXiv:2604.02344v1
//!   (Maczan -- 4 GPU vendors, Dawn + wgpu-native, 3 browsers; open
//!   data; cross-validated in-tree by riir-ai Bench 761).
//!   `dispatch-cost-sequential-measurement-not-single-op` (D -- single-op
//!   dispatch benchmarks conflate the submit->completion-drain round trip
//!   with enqueue cost, 10-60x overestimate: Dawn 496.8 vs 23.8 us,
//!   Chrome 2071-3123 vs 32.8-66.5 us; N back-to-back dispatches + ONE
//!   tail sync isolates the true cost; submit alone is 40% of it);
//!   `fusion-payback-gated-on-per-dispatch-overhead` (C -- the SAME
//!   `RMSNorm` 6->1 fusion: 1.41x wgpu/Vulkan (24-36 us/dispatch), 0.95x
//!   wgpu/Metal + 0.91x Safari (kernel-penalty-dominated), 0.92x CUDA
//!   (7.4 us); B* batch crossover marks the overhead-bound regime;
//!   payback = `dispatches_saved` x per-dispatch overhead vs fused-kernel
//!   penalty); `grid-collapsing-mega-kernel-occupancy-trap` (E -- WebGPU
//!   has no cross-workgroup sync, so the mega-kernel is ONE workgroup;
//!   17k sequential MADs/thread at production scale vs 65k+ threads; the
//!   tiled middle ground 7->3 dispatches preserves the grid: 2.01x Metal,
//!   1.17x Vulkan; mega-vs-multi honestly inconclusive at toy scale
//!   p>0.38 -- the regime split IS the rule). Record: Research 094 /
//!   Plan 038.
//! - **Deferred:** remaining popcorn kernels are mostly already-covered patterns
//!   or domain-specific math; future batches need new source impl files.
//! - **Batch 81 (2026-08-27, lucebox concurrency-stack distill -- PRs
//!   642/651/659/668, Research 095 / Plan 039):** 6 rules, corpus
//!   387 -> 393 (A 91 -> 93, D 79 -> 82, G 21 -> 22). Source is C++/HIP
//!   on vendored ggml (R9700/gfx1201); patterns port, code does not --
//!   all measured p50/goodput tables are PR-body numbers (absent from
//!   the diffs), cited as source evidence only.
//!   `virtual-tail-packed-tree-verify-over-paged-kv` (A -- tree
//!   candidates as a VIRTUAL TAIL of the committed length; block-table
//!   prefix rows + scratch-slab candidate rows behind ONE logical
//!   index; ancestor-visibility walk gates siblings; `phys = -1` for
//!   invisible rows). `gdn-transition-replay-log-commit-many` (G -- the
//!   forward journals `[gate | key | delta]` per token/head; ONE kernel
//!   replays accepted prefixes via `fmaf(key, delta, gate * current)`:
//!   no second target forward). `cross-lane-column-packed-projections`
//!   (D -- the AXIS-TRANSPOSED fourth member of the same-input family:
//!   N lanes' inputs packed over ONE shared weight; lane-local
//!   consumers carve back out). `identity-keyed-graph-retention-stable-
//!   arena` (D -- composite owner/backend/topology key with the kv axis
//!   BUCKET-quantized; retained metadata arena keeps `nodes[0]` stable;
//!   the SALT prevents cross-bucket aliasing).
//!   `staged-append-rollback-lifo-retry-blocks` (A -- rollback routes
//!   freed blocks to a per-sequence LIFO so retry reproduces the SAME
//!   physical rows; captured graphs + block-table deltas survive
//!   reject+retry). `fail-closed-multi-resource-commit-transaction`
//!   (D -- VALIDATE every destination across ALL resources first, then
//!   `COMMIT_PREVALIDATED`; after the first mutation no recoverable error
//!   may escape). Plus the Batch-81 rider on
//!   `shared-activation-quantize-memo-per-eval` (lucebox 668: the
//!   REQUEST-boundary release half -- memos popped LIFO before pool
//!   trim). Record: Research 095 / Plan 039.
//! - **Batch 84 (2026-08-28, oMLX v0.5.4rc1 -> v0.6.3 release delta --
//!   jundot/omlx PRs #2966/#3133/#3110, Research 098 / Plan 045):** 2 rules
//!   + 1 rider, corpus 396 -> 398 (D 83 -> 84, N 21 -> 22).
//!
//!   Sources are the PR pages + diffs + release notes
//!   (subagent-verified at mine time); the +26.3%/+22.7% figures are
//!   PREFILL-THROUGHPUT, never quality (the research record's "PP"
//!   gloss corrected at mine time). `pad-tail-to-fixed-tile` (D --
//!   residual tails large enough to pay run the resident fixed-shape
//!   tile; intermediate-only padding, strip-before-stateful, never
//!   extend state; floor(S*G/H)+1 = 1357; +29.0% at a 2,047-token
//!   tail) + `token-local-only-approximate-accelerator` (N -- the
//!   delegation boundary for approximate compute is the consumer's
//!   STATE HORIZON: token-local rows tolerate bounded approximation,
//!   recurrent-feeding rows stay checkpoint-precision; the fraction is
//!   a logged ceiling) + the B56 rider
//!   `assigned-columns-only-staging-dequant` (slice to the device
//!   boundary BEFORE dequant; spike 35.8 -> 4.7 GB, bit-identical).
//!   The 4th queued candidate (latch-failed-accelerator-shed-to-
//!   fallback) deferred per the queue verdict -- fold into a
//!   measured-GOAT from our own status=0x9 work instead.
//! - **Batch 82 (2026-08-28, `TreeWY` + Launch-Bound arXiv distill --
//!   2608.20961 + 2608.26612, Research 096 / Plan 043):** 3 rules, corpus
//!   393 -> 396 (D 82 -> 83, E 23 -> 24, G 22 -> 23). Sources are arXiv
//!   papers (B200/vLLM + A100/torch) with measured tables in the papers;
//!   fixtures are original minimal reproductions.
//!   `wy-transform-tree-verify-state-free` (G -- DFS pre-order makes the
//!   GDN transition system strictly lower-triangular; ONE forward
//!   substitution verifies the whole draft state-free; reconstruct only
//!   the accepted state; `O(N*d_v)` pseudo-values vs `N+1` state blocks,
//!   128x smaller per head; win CONDITIONAL on memory pressure; trees
//!   affordable but NOT a throughput win -- externally confirming our
//!   Issue 721 structural negative). `work-scaling-probe-launch-bound`
//!   (D -- 32x the work at 1.55x the time = launch-bound; 5.6-9.0x
//!   isolated kernels returned 0.999x end-to-end vs a measured 1.072x
//!   Amdahl ceiling; f from wrapped `record_function` modules, never
//!   kernel-name matching). `capture-break-count-not-perf-target` (E --
//!   forcing zero graph breaks via dynamic-shape capture made the model
//!   3x SLOWER; unbacked symbolic-shape guards cost more than the eager
//!   dispatch they replace; corroborated by `TreeWY`'s piecewise-capture
//!   eviction cost). Record: Research 096 / Plan 043.
//! - **Batch 87 (2026-08-29, the TTT-KVB App-I boundary distill --
//!   arXiv:2602.21204 "TTT with KV Binding Is Secretly Linear Attention"
//!   (NVIDIA, ICML 2026), Research note = katgpt-rs R516 (`513088cd`) /
//!   Plan 048 / Issue 054):** 2 rules + 1 B79 rider, corpus 398 -> 400
//!   (G 23 -> 25). The BOUNDARY COMPLEMENT of Batch 79's
//!   `first-order-affine-recurrence-associative-scan` (B79 = affine =>
//!   scan; this = what BREAKS the scan):
//!   `state-normalization-between-updates-breaks-associative-scan` (G --
//!   Norm(A+B) != Norm(A)+Norm(B): state normalization between updates
//!   voids associativity outright; the `LaCT` weight-norm anti-shape;
//!   fix-space = normalize q/k instead (phi-side, scan-SAFE) or accept
//!   the sequential form) + `dynamic-kernel-params-break-prefix-scan`
//!   (G -- Theta updated INSIDE the recurrence makes phi
//!   history-dependent through nested-nonlinearity chains; fix-space =
//!   freeze Theta; the paper's V1/V2 reduction rungs are the design
//!   table). Rider on B79's `gpu_concept`: qk/head-side normalization is
//!   part of phi and does NOT break the scan (negative control:
//!   riir-gpu `expand_and_l2_normalize_heads_f32` -- in-tree SAFE).
//!   THEORY-DISTILLED, grade B (throughput AND quality numbers are `LaCT`
//!   end-to-end, never cite isolated); ZERO in-tree firing population
//!   (paper-distilled precedent = B79 itself). Content gate:
//!   `batch87_state_norm_breaks_associative_scan` (the breaker's
//!   wrongness pinned materially, non-vacuous).
//! - **Batch 130 (2026-09-08, the `CloudRift` `cuBLAS`-5090 TMA distill,
//!   Plan 111 / Research 152):** 5 rules, corpus 575 -> 580 (A 121 -> 122,
//!   B 60 -> 61, D 130 -> 132, E 46 -> 47). Source: `cloudrift-ai/emmy`
//!   @ `2aaa0328` (Apache-2.0; the blog landing commit "Mini GPU compiler:
//!   TMA SGEMM kernel beating cuBLAS on RTX 5090" #89 — NOT the
//!   `DeploDock/Emmy` path in the queue lead, which 404s; org is
//!   `cloudrift-ai`, and commit `8764cc2e` later restructured the repo).
//!   `vendor-dispatch-verify-before-outsource` (D — a suspiciously-good
//!   win over a vendor baseline is a dispatcher-bug signal: cuBLAS 5090
//!   batched FP32 dispatches ONE `simt_sgemm_128x32_8x5` across a 32x
//!   range at 33-42% FMA pipe while the same binary escalates to 73/82%
//!   on Pro 6000/H200 — no escalation at ANY threshold means the ladder
//!   is missing, not mistuned), `sub-ms-bench-boost-clock-never-engages` (D — below ~1 ms the boost governor never engages; interleaved-pair
//!   iterations + median + ratio-not-absolute + clock sampling),
//!   `lds-cluster-cross-warp-phase-not-static-distance` (B — after
//!   static latency hiding suffices, the LDS temporal distribution
//!   governs warp phase: clustered LDS halves `dispatch_stall` 44%->22%,
//!   the generator-side cause is emission order at `lower.py:192-198)`,
//!   `tma-single-issuer-bulk-tile` (A — one thread issues the whole 2D
//!   tile, mbarrier byte-count completion, double-buffer parity; the
//!   honest claim is instruction count + simplicity, not FMA pipe;
//!   ragged-K truncation caveat in-rule),
//!   `multiplexed-grid-axis-partition-modes-exclusive` (E — split-K and
//!   batch both consuming blockIdx.z must be refused at config time;
//!   the source's own `ValueError`). Riders: `l2-grouped-tile-schedule`
//!   (SWIZ=8 formula + DRAM parity), `unroll-fixed-inner` (partial-unroll
//!   monotonicWORSE boundary), `autotune-warps-stages` (`launch_bounds`
//!   20x spill negative + triple-buffer same-IPC + tensor-pipe positive
//!   caveat), `cpasync-multistage-tile-pipeline` (82.6% crossover +
//!   `mbarrier.try_wait`), `hardcoded-fast-path-jit-specialization` (the
//!   codegen `#define`/`#if` twin), `launch-heuristic-pinned-to-calibration-arch`
//!   (CC is not a sufficient SKU key), `fast-math-contraction-behind-feature-flag`
//!   (fmad split from `use_fast_math`).
//! - **Batch 144 (2026-09-16, the measured-GOAT self-distill of the riir-ai
//!   4090 prefill-mma campaign, Plan 129):** 5 rules + 2 riders, corpus 607 ->
//!   612 (A 130 -> 134, D 133 -> 134). Source: OUR OWN bench records --
//!   riir-ai `.benchmarks/878`-`884` (Issue 884 rungs T2a/T2b/T4; read-only
//!   citations, the owning lanes keep their code); two independent quote
//!   lanes + caller spot-check, the intake's drifts corrected at admission.
//!   `packed-code-cpasync-stage-per-use-prmt-decode` (A — the B883 rung-5
//!   GOAT: packed 2-bit `Q2_0` codes shrink the A stage 4x — resident 6,144 B
//!   vs payload 4,096 B, pad-skip reconciles the 256-chunk fill — buying
//!   double-buffered depth at ONE barrier per group; per-use 9-op PRMT
//!   decode measured ~free (1.26x from the decode alone); v9 1.222x/1.224x
//!   vs v4-q8 AND 1.062x/1.064x vs v6tb; negative arms in-rule: per-fragment
//!   global-A staging (B881/B882, both formats, both warp geometries), the
//!   256-entry smem decode LUT (B879, sequential-protocol caveat), full-size
//!   double-A v6d), `ldmatrix-byte-subword-granularity-limit` (A — the
//!   complement of B9's positive rule: byte-sub-word fragment consumption
//!   cannot be served under any swizzle; LDS quad-broadcast + per-use decode
//!   is forced and measured winning), `transposed-smem-stage-repairs-
//!   strided-epilogue` (A — the B884 rung-6 GOAT: an epilogue lane map
//!   across the contiguous axis amplifies L2 write sectors 8.01x; the
//!   [TOKS][33] stage (aliases the freed B slab, stride = 1 mod 32) + the
//!   `r = idx & 31` reader map = 35.65M -> 4.46M sectors, L2 SOL 74.7 ->
//!   59.0%, 1.533x over the route winner, T4's 1.8x gate CLEARED),
//!   `per-block-scalar-slab-cpasync-stage` (A — the honest-cap arm: scalar
//!   slab cp.async'd ONCE per group per block via 4-B .ca; read sectors
//!   UNMOVED 84.4M — an issue-count/IPC rule, never quote it as bandwidth),
//!   `profile-sol-table-before-compute-rung` (D — the methodology rung:
//!   record the SOL table + sector counts and route by the TOP limiter
//!   before any compute-side rung; the family's prior rungs ran with no ncu
//!   data while L2 SOL sat at 69.9-89.8%). Riders ×2 on B9:
//!   `cpasync-multistage-tile-pipeline` (+ the packed-code depth face),
//!   `ldmatrix-mma-layout-loads` (+ the granularity-boundary face).
//!   Benchability: the measurements ARE riir-gpu's own scored-lane records
//!   — the tables are the decision content; no re-bench in this repo
//!   (`kernel_verify` STUB). Record: Plan 129 / Research 169.
//! - **Batch 145 (2026-09-16, the Metal-shape pack, Plan 130):** 2 rules +
//!   2 riders + 1 rust-perf rule (`entries_06.rs`), corpus 612 -> 614
//!   (B 63 -> 64, F 14 -> 15). Sources: arXiv 2609.09721 (EFQ-Softmax,
//!   walk #9 lead 1) + youssofal/MTPLX @ `a5e38bb7` (PR #391, Apache-2.0;
//!   the staged charter in `.distill/001`). Two quote lanes + caller
//!   spot-checks; the charter's own dedup verdict CORRECTED at admission:
//!   the `split.?k` grep was truncated and missed
//!   `split-k-two-pass-attention-decode-long-context` (Batch 105) -- the
//!   KV-split rule is scoped to the genuine delta (width-underfill trigger
//!   plus sparse per-slot validity) with a loud distinct-from + a reciprocal
//!   rider on the Batch-105 rule. `direct-low-bit-generation-replaces-
//!   exp-quantize-roundtrip` (B -- the EFQ lane: shifted scores ->
//!   block-scaled E2M1 codes DIRECTLY via the exponent-only scale + one
//!   calibrated affine rule, same (code, scale) operand in numerator AND
//!   denominator; the exp-then-quantize roundtrip deleted; -40.33% is the
//!   VECTOR STAGE of the fused probability- generation kernel (fixpipe
//!   0.3-1.0%, mmad excluded; the paper's kernel table is labeled "LUT" --
//!   variant ambiguity carried); quality claim = beats MXFP4 within 0.005
//!   of FP16 (language gain is operating- point-dependent; the large gains
//!   are VL grounding)); `kv-split-decode-underfill-per-slot-validity`
//!   (F -- the MTPLX lane: decode grids sized over query rows only
//!   underfill at decode M (M=4 -> 8 threadgroups on 40 cores); split the
//!   REDUCTION axis with unnormalized partials + an LSE merge that owns
//!   the divide; PER-SLOT validity for the unsorted argpartition selection
//!   (prefix cut admits/ drops, the <= predicate double-counts),
//!   no-empty-split clamp, one-encoder combine; the underfill negative
//!   cell is DUAL-SOURCED in-regime (MTPLX `hyper_read` v1 13.2 vs 67.8
//!   tok/s + Hopper 2609.12923 fixed 64-row GMMA fragments)).
//!   Riders x2: `same-input-gemv-weight-concat-at-upload` (+ the
//!   hyper-read fused read: family confirmed, the row-parallel v1
//!   negative cell, and the pointer-switch virtual-row weight fold --
//!   concat semantics with zero upload-time copy),
//!   `split-k-two-pass-attention-decode-long-context` (+ the sparse
//!   decode twin cross-ref). `rust_perf`: `mincore-probe-routes-gather-arm`
//!   (suggest-only -- the ~0.5 ms sampled mincore probe routes
//!   warm->fancy-index vs cold->pooled pread against a ~165 ms warm pass;
//!   threshold 0.99, fail-safe to pread, worst-of-all-maps,
//!   small-gather inversion; the macOS FILE-WIDE mincore caveat
//!   encoded). Record: Plan 130 / Research 171.
//! - **Batch 142 (2026-09-10, the `CloudRift` "Outperforming vLLM and
//!   Llama.cpp on Gemma4-12B" distill, Plan 122):** 2 rules, corpus 605 ->
//!   607 (A 129 -> 130, N 43 -> 44). Source: `cloudrift-ai/emmy` @
//!   `321288d8` (Apache-2.0; the tip past Batch 138's `bdd26c53` — HEAD ==
//!   ls-remote verified at mine) x the Gemma4-12B blog (2026-08-01,
//!   fetched full-text; the #761 correction commit `e0d16cf0` read BEFORE
//!   quoting baselines — the 5090 attention Emmy lane has no recorded
//!   goldens, so article ATTENTION-baseline claims carry that caveat; the
//!   accumulation-error table is the repo's own harness and unaffected).
//!   `fast-accumulate-periodic-promote-bounded-error` (N — the A− GOAT
//!   lead: keep the WHOLE consumer-die FP16-input mma chain on the
//!   full-rate f16-accumulate atom (330.3 vs 165.2 TFLOPS on the 4090;
//!   419 vs 209.5 on the 5090) + every 64 K-elements fold the packed f16x2
//!   partials into an f32 shadow + rezero; error FLAT ~3.3e-4 at every K
//!   vs pure-FP16's K-growing 6.0e-4 -> 6.6e-3; blocked promotion = the
//!   bound CONSTANT in K (Blanchard); 1.24-1.55× over the same compiler's
//!   FP32 lane; the UNPROTECTED `PyTorch` `allow_fp16_accumulation` is the
//!   negative control; provenance `DeepGEMM` -> `CUTLASS` -> `Ootomo` ->
//!   Blanchard -> Patterson), `smem-budget-single-buffer-refill-at-kill-
//!   points` (A — the B79/B130 staging-depth family's budget-clamped
//!   boundary arm: d256 FA holds Q+K+V at 96 KB of ~100 KB/CTA, the ring
//!   clamps to 1 (`min(depth, budget // slot_bytes)`), and overlap shifts
//!   from buffer COUNT to refill TIMING at each operand's KILL POINT,
//!   lag 1; live-across-body is the single-buffer degenerate; split-KV is
//!   the occupancy escape, never a deeper ring). Riders ×6: B58 launch-
//!   count (the vLLM plugin-boundary instance, ~1100 vs ~650 launches/step
//!   — count launches per step when pricing an integration boundary), B142
//!   fusion-payback admission axis (a catalog-only fused form FAILED the
//!   fp16 accuracy gate in isolation — admission is per-realized-form,
//!   fail-closed), B18 splitkv trigger (occupancy-deficit read off the
//!   profile: split+combine kernels + CTA count under SM count), B36
//!   vendor-lag external confirmation (cuBLAS FP16 tensorop on consumer
//!   Blackwell = forward-ported Ampere `cutlass_80_*` still on cp.async) +
//!   the stale-blocker corollary (the 20015µs SDPA blocker no longer
//!   reproduces at `bdd26c539`), B38 enumeration-knob governance face,
//!   B130 exact/lossy flag SPLIT face. DEFERRED [-]: the dynM catalog-gap
//!   rider (B104/B128, `entries_gpu_34.rs` — sibling WIP; lands with the
//!   next batch touching the file). `NotBenchable` in this stack
//!   (consumer-GeForce FP16-mma specific; riir-gpu scored lanes are
//!   int8-mma/int32-accum); no league row (Research 163 lane-intel NONE
//!   triple-crossed). Records: Plan 122 / Research 163.
//! - **Batch 139 (2026-09-09, the ds4-metal PR #5 distill, Plan 119):** 4
//!   rules, corpus 601 -> 605 (D 132 -> 133, E 53 -> 54, H 28 -> 29,
//!   I 6 -> 7). Source: `ivanfioravanti/ds4-metal` branch `qwen3.8-flash-next`
//!   @ `18ca8ec` (MIT; the merge of polymorf's PR #5 + 6 post-merge maintainer
//!   commits — the delta past Batch 129's `b85a6174` pin, 19 commits). Two
//!   read lanes (kernel surfaces / host-io-measurement) + coordinator;
//!   Research 154's round1.md caveat closed (read in full at mine time).
//!   `expert-major-tile-order-token-tile-cache-resident` (H — the GOAT lead:
//!   the `MoE` prefill grid re-enumerated (`row_blocks` x `tiles`, experts) so
//!   consecutive threadgroups share a token tile and one expert's weight rows
//!   stay cache-resident across its tiles; a pure permutation of threadgroup
//!   execution — bit-exact by construction; +1.8-2.6% on M5 Max),
//!   `narrow-matvec-one-row-per-simdgroup` (E — decode matvecs with rows
//!   <= 1024 launch one row per simdgroup: threadgroup count doubles, per-row
//!   arithmetic unchanged, rollback -0.55%/-0.54% vs no-op -0.005%; the
//!   `Q4_K` and `MXFP4` `MoE` twins instantiate the ladder at their own grain),
//!   `tile-max-prefiltered-exact-topk` (I — tau = k-th largest tile maximum
//!   is a lower bound on the k-th block score: keep only bound-reaching
//!   tiles, run the SAME radix over the compact set, byte-identical ties
//!   included; capacity fallback reruns the full row in the SAME dispatch;
//!   the producer/consumer enable predicates are CONJOINED at both dispatch
//!   sites — the stale-sidecar defense is dispatch-granularity coupling, not
//!   in-kernel detection), `sustained-load-drift-interleave-protocol` (D —
//!   whole-run drift 10-20%/minutes + a ~25% first-session-at-new-context
//!   penalty: ABBA/BAAB over four fresh processes per frontier OR per-chunk
//!   interleaving in one session, untimed full pass first, no-op control as
//!   the noise floor, ~2% evidence threshold, and the graph-alloc-time-knob
//!   blind spot for same-engine A/Bs). Riders: output-buffer-rotation-pool-
//!   multi-forward (+ the MTP swap-restore face: complete-overwrite snapshot,
//!   pointer-swap adoption, invalidation; B58 cross-ref),
//!   pipeline-cache-key-covers-every-function-constant (+ the DETECTION
//!   half: failed lookups memoized log-once + the working-directory-compile
//!   diagnostic naming the binary/checkout skew), cpasync-multistage-tile-
//!   pipeline (+ the MPP double-buffer Metal instance: one barrier per
//!   k-step, `Q4_K`-only), fast-math-contraction-behind-feature-flag (+ the
//!   staged-loop rejection: eight-terms-ahead staging changed contraction,
//!   logits differ — the negative control), kv-dequant-materialization-
//!   batch-threshold-gate (+ the chunk-width-per-format facet: `Q2` fastest
//!   at 8192-token chunks +35%, `Q4` at 4096 2.2x, regresses at 8192),
//!   swept-shape-bucket-launch-tables (+ the GDN front thread-count row:
//!   256 elsewhere, 1024 on M5, +0.69% over 512's +0.14%),
//!   multi-simdgroup-shared-staged-tile (+ the indexer-scorer instance:
//!   4 query rows staged once per threadgroup, half4 keys, index-order
//!   accumulation preserved), register-rotation-prefetch-loop (+ the merge
//!   16-split prefetch: sixteen splits' loads in flight per round, chain
//!   visits in order — 64 dependent-load rounds -> 4). DROPS: `c73d32a`
//!   tool-call parsing (host chat-template correctness, zero perf content);
//!   the four-row `Q8_0` matvec tile + tg-64/128 sweeps (the source's own
//!   rejected-variants table, riders of record only). Lane intel: M3-decode
//!   leads (one-row matvec occupancy + GDN widening) named for 844/851, NOT
//!   edited; Flash-Next re-arm trigger stays unlit (neither pack <=24-GB
//!   feasible). Records: Plan 119 / Research 154.
//! - **Batch 138 (2026-09-09, the `CloudRift` "Learning `FlashAttention` the Hard
//!   Way" Parts 1+2 distill, Plan 118):** 10 rules, corpus 591 -> 601 (A 127 ->
//!   129, B 61 -> 63, E 51 -> 53, F 13 -> 14, K 11 -> 12, L 16 -> 17, M 19 ->
//!   20). Source: `cloudrift-ai/emmy` @ `bdd26c53` (Apache-2.0; the tip past
//!   Batch 137's `1c058c9d` — carries #758, post-article content) x the FA
//!   articles (Trifonov, 2026-07-06/08; the queue's bare blog path 404s — the
//!   real paths carry `-part-1`/`-part-2`). `split-copy-issue-deposit-depth-
//!   uniform` (A — #758: split the blocking gmem→smem copy into issue+deposit
//!   halves either side of the drain so the depth knob means "chunks in flight"
//!   on EVERY transport; V100 1.53x/1.69x with an honest −39% loser row that
//!   keeps depth 1), `stage-domain-product-of-independent-knobs` (K — the
//!   staging search domain is the PRODUCT of transport×depth×reg-depth,
//!   capability-filtered + resolver-sized; the hand-picked catalog reached only
//!   4 of the register double-buffer's combinations, 16→24 recovery),
//!   `fold-invariant-factor-commutes-to-epilogue` (B — the twist pass algebra:
//!   a factor constant along the fold axis commutes out; divisors hoist as
//!   divisions, a varying divisor keeps the fold whole; exposes the expectation
//!   channel — 1/l applied once, after the fold), `twisted-monoid-dependent-
//!   reduce-fusion` (F — the RECOGNITION mechanism: a dependent reduce fuses
//!   with its producer reduce under a bijection-conjugated monoid, recipes
//!   tried to fixpoint, stable form spelled, singleton injection read, RECIPES
//!   = (SOFTMAX, WELFORD); the FA-2-parity convergence factoid rides here),
//!   `c-fragment-to-a-operand-register-repack` (L — P converts in registers
//!   when fragment lane maps align, four `cvt.rn.f16x2` — the 271µs rung;
//!   Volta m8n8k4 shuffles variant), `stream-invariant-operand-skips-staging-
//!   pipeline` (A — delta-0 operands fill once in the prologue and never enter
//!   the pipeline schedule; the Q hoist, 233µs rung; FA-2 ships it disabled by
//!   default — per-geometry evidence decides), `rewrite-decline-names-refusing-
//!   predicate` (M — "Declining is not neutral": name the refusing predicate
//!   at -vv + report residual clusters post-fixpoint; the only visible symptom
//!   of a silent demotion is the measurement), `defensive-sync-peephole-tile-
//!   body-scope` (E — drop provably-no-op template syncs, tile-body scope only,
//!   nested Loop/Cond bodies are load-bearing), `sfu-exp-swap-pays-only-when-
//!   sfu-limited` (B — the fast-exp refusal twice-measured: an SFU-side swap
//!   pays only when the SFU binds; tensor-bound at ~80% roofline it moves
//!   nothing, and FA-4's FMA-exp answers a DATACENTER-Blackwell imbalance),
//!   `left-right-fold-pair-proves-associative-combine` (E — the 3HT licenser:
//!   foldl + foldr both exist ⇒ an associative combine exists, constructible
//!   from the two folds; "a test for when any loop is secretly associative").
//!   Riders: `setmaxnreg-fit-gate-elected-arrive` (+ the async-MMA admission
//!   half: WS without wgmma/tcgen05 is a pessimization, 213 vs 206 measured),
//!   `autotune-warps-stages` (+ deeper-rings d3/d4 tie→loss),
//!   `online-softmax-streaming` (+ the never-amplifies accuracy license),
//!   `state-normalization-between-updates-breaks-associative-scan` (+ the
//!   injectivity breaker: quantize-in-the-loop voids the scan license,
//!   dequant-precision carrier is the counter-arm). Lane intel: the 4090 lane
//!   is consumer silicon — FA-2 parity is the honest frontier there (the
//!   Q-hoist + C→A repack + split-copy moves are the transferable set; the
//!   FA-3/4 asynchronous-MMA rungs stay locked behind datacenter parts).
//!   Records: Plan 118 / Research 157.
//! - **Batch 137 (2026-09-09, the `CloudRift` matmul-optimization distill,
//!   Plan 116):** 11 rules, corpus 580 -> 591 (A 122 -> 127, E 47 -> 51,
//!   L 15 -> 16, N 42 -> 43). Source: `cloudrift-ai/emmy` @ `1c058c9d`
//!   (Apache-2.0; the renamed successor of the Batch-130 `deplodock` pin
//!   `2aaa0328`) x the "Modern GPU Matmul Optimization" article (Trifonov,
//!   2026-05-29; the queue lead's guessed blog path 404s — the real path is
//!   `/blog/gpu-matmul-optimization`). `smem-fill-align-chunk-not-element`
//!   (N — the slab base alignment derives from the FILL CHUNK, not the
//!   element width: an f16 slab 2 B-aligned takes 16 B writes that fault at
//!   8 mod 16; TMA destinations pin 128 B even at NONE swizzle),
//!   `software-swizzle-shift-row-stride` (A — the software-fill XOR swizzle
//!   shift derives from the ROW STRIDE, not log2(atom): a 128-elem fp16 row
//!   is two atoms and collapses rows — the measured 7.8-way ldmatrix
//!   conflict at D=128; disagreement between the three derivation consumers
//!   is a TMA COPY DEADLOCK), `companion-slab-ring-slot-sync` (A — a scale
//!   sidecar riding the same ring as its codes must carry the slot-row
//!   offset; bare coordinates read slot 0 while codes read slot s — a
//!   minority of cells off 2-3x, invisible in source AND aggregate parity;
//!   found live in-source), `ptxas-recoalesces-scalar-runs-wash` (A — hand
//!   vectorization of regular stride-1 runs is a wash: ptxas recoalesces at
//!   every deployable -O; the pass is deliberately NOT a search dimension;
//!   escape arm = irregular kernels with a static alignment proof),
//!   `ptxas-reschedules-load-order-wash` (A — hand load/FMA interleaving is
//!   a wash for the same reason; ASYNC issue-site placement stays a real
//!   decision), `smem-pad-transport-gated` (A — the bank pad is dead under
//!   TMA (descriptor swizzle) and live under cp.async, with a granularity
//!   fork: 16 B chunk-aligned vs deliberate +1 de-vectorization — the
//!   article's 3.7x arm; fill/drain share ONE pad source of truth),
//!   `split-k-atomic-arm-legality` (E — the atomicAdd arm is semantically
//!   illegal regardless of shape unless ONE component + ADDITIVE monoid +
//!   output-rounds-once + distributive projection; every refusal names the
//!   deferred finalize), `setmaxnreg-fit-gate-elected-arrive` (E, `sm_90`+ —
//!   the .dec/.inc pair is gated on register-file arithmetic (24/240/64K),
//!   the consumer band syncs via a NAMED barrier (CTA-wide __syncthreads is
//!   UB on the divergent branch), ONE elected arrive releases the slot +
//!   carries fence.proxy.async), `splitk-width-shape-need-gate` (E —
//!   needed = max(2*`SM/free_ctas`, K/sqrt(M*N), 1): grid fullness alone
//!   under-splits K-heavy shapes (`mlp_down` ranked its g8k golden ~9000-deep
//!   under the occupancy-only gate)), `splitk-atomic-free-contention-threshold`
//!   (E — the finalize fork keys on SPLITK*M*N contention with a FITTED
//!   threshold and a round-trip cost side; the fork key must be STAMPED on
//!   candidate rows or the interaction dead-weights to atomic),
//!   `ldmatrix-pair-x2-to-x4-adjacency` (L — fuse two +8-adjacent x2
//!   fragment loads into one x4, bit-identical, WAR-checked). Lane intel:
//!   4090 = `sm_89` Ada — cp.async pad arm live, split-K width gate strongest
//!   (prefill GEMMs are the K-heavy family), setmaxnreg Hopper+ only.
//! - **Batch 129 (2026-09-08, the ds4 Qwen Q2 decode-fusion + prefill-structural distill,
//!   Plan 110 / Research 151):** 5 rules, corpus 570 -> 575 (C 45 -> 46,
//!   H 27 -> 28, L 14 -> 15, M 18 -> 19, N 41 -> 42). Source:
//!   `ivanfioravanti/ds4-metal` @ `b85a6174` branch `qwen3.8-flash-next`
//!   (MIT; the PR line's 12-commit linear delta past the Batch-125 pin,
//!   plus the upstream `b6af0ad` SSD seed-bounding commit riding in via
//!   the PR's upstream merge — rider on the Batch-123 static-seed rule).
//!   `pipeline-cache-key-covers-every-function-constant` (M — a keyed
//!   pipeline cache serving driver-specialized variants must name EVERY
//!   bound function constant; subset keys serve one variant's binary for
//!   another's launch args), `block-quant-weight-pad-never-extends-activations`
//!   (N — `Q2_K` 256-block row padding is weight-stride-only; the dequant
//!   loop gates on the LOGICAL bound and partial activation groups are
//!   rejected fail-closed), `expert-remainder-routed-to-smaller-tile-twins`
//!   (H — per-expert tail rows run the smallest covering tile twin under a
//!   compile-time `tail_base`; the runtime-branch alternative measured
//!   -10.98% and was removed in-source),
//!   `window-recurrence-halo-snapshot-block-parallel` (L — a K<=4 causal
//!   conv parallelizes over token blocks behind a HALO-only snapshot; the
//!   full-input snapshot measured slower and was superseded),
//!   `fused-residual-update-writes-separate-next-buffer` (C — fused
//!   residual+norm reads old buffers read-only and writes distinct next
//!   buffers, alias-rejects fail-closed, and gates on the measured session
//!   shape). Riders: device-side-argmax CPU-parity facets (`entries_gpu_02`),
//!   fusion-payback fan-in instance (`entries_gpu_07`), static-seed
//!   dynamic-producer twin (`entries_gpu_31`), kv-dequant expansion-ratio
//!   sharpening (`entries_gpu_34`).
//! - **Batch 128 (2026-09-07, the `PrismML` llama.cpp Metal/FA distill,//!   Plan 109 / Research 150):** 5 rules, corpus 565 -> 570 (A 120 -> 121,
//!   K 9 -> 11, N 39 -> 41). Source: `PrismML-Eng/llama.cpp` master @
//!   `5ea87dda` (MIT; the same fork Batch 127 mined for the CUDA slice —
//!   this batch takes the unmined Metal/FA surface, the M3 box's
//!   first-priority domain).
//!   `kv-dequant-materialization-batch-threshold-gate` (N — the KV
//!   dequant-to-f16 copy is gated on an amortization threshold, ne[1] <
//!   32 keeps the quantized cache; the wide-form scratch stays RESERVED
//!   even on the quantized path so the graph allocator never re-plans),
//!   `offline-device-tuned-table-dispatch` (K — runtime autotune
//!   replaced by a generated per-SKU (Q, NE) table keyed with BUCKETED
//!   context dims, dispatched through exact -> domain-default ->
//!   family-representative -> baseline; the header IS the regeneration
//!   contract), `tuned-config-resource-limit-fallback` (K — the tuned
//!   config re-validated against THIS device's threadgroup-memory limit
//!   at dispatch; exceed falls back to the known-safe baseline, never
//!   errors, never half-clamps),
//!   `quantize-scale-math-wide-precision-denorm-safe` (N — the dynamic
//!   quantize transform (amax/127, 1/qd, round) computed in fp32 inside
//!   the fp16 FA kernel: the scale can be a DENORM and its inverse then
//!   overflows to inf — correctness, no tolerance applies),
//!   `extent-clamped-operand-views-partial-k-tile` (A — static-tile-K
//!   Tensor-API matmul read OOB on partial tails; fix = `dynamic_extent`
//!   with per-iteration operand-view extents clamped to `min(tile, K -
//!   loop_k)`, degenerating to a handful of integer ops on aligned K).
//!   Riders/ledger: `3af988fa` bias-fold SKIPPED (epilogue family
//!   covers), `369e1cd6` row-memcpy SKIPPED
//!   (`wide-row-copy-tile-rows-across-workgroups` covers), `b615f5b4`
//!   per-op compile split SKIPPED (build-infra), and the a3027337
//!   provenance note ('Assisted-by: pi:llama.cpp/Qwen3.8-27B'). Record:
//!   Research 150 / Plan 109.

use std::collections::HashMap;

use crate::corpus::Corpus;
use crate::domains::kernel_opt::{KernelCategory, KernelOptDomain, KernelRule};

mod entries_gpu_01;
mod entries_gpu_02;
mod entries_gpu_03;
mod entries_gpu_04;
mod entries_gpu_05;
mod entries_gpu_06;
mod entries_gpu_07;
mod entries_gpu_08;
mod entries_gpu_09;
mod entries_gpu_10;
mod entries_gpu_11;
mod entries_gpu_12;
mod entries_gpu_13;
mod entries_gpu_14;
mod entries_gpu_15;
mod entries_gpu_16;
mod entries_gpu_17;
mod entries_gpu_18;
mod entries_gpu_19;
mod entries_gpu_20;
mod entries_gpu_21;
mod entries_gpu_22;
mod entries_gpu_23;
mod entries_gpu_24;
mod entries_gpu_25;
mod entries_gpu_26;
mod entries_gpu_27;
mod entries_gpu_28;
mod entries_gpu_29;
mod entries_gpu_30;
mod entries_gpu_31;
mod entries_gpu_32;
mod entries_gpu_33;
mod entries_gpu_34;
mod entries_gpu_35;
mod entries_gpu_36;
mod entries_gpu_37;
mod entries_gpu_38;
mod entries_gpu_39;
mod entries_gpu_40;
mod entries_gpu_41;
mod entries_gpu_42;
mod entries_gpu_43;
mod entries_gpu_44;
mod entries_gpu_45;
mod entries_gpu_46;
mod entries_gpu_47;
mod entries_gpu_48;
mod entries_gpu_49;
mod entries_gpu_50;
mod entries_gpu_51;
mod entries_gpu_52;
mod entries_gpu_53;
mod entries_gpu_54;
mod entries_gpu_55;
mod entries_gpu_56;
mod entries_gpu_57;
mod entries_gpu_58;
mod entries_gpu_59;
mod entries_gpu_60;
mod entries_gpu_61;
mod entries_gpu_62;
mod entries_gpu_63;
mod entries_gpu_64;
mod entries_gpu_65;
mod entries_gpu_66;
mod entries_popcorn;

#[cfg(test)]
mod tests;

/// Build the kernel optimization corpus from popcorn-distilled rules.
///
/// Returns a BLAKE3-committed [`Corpus<KernelOptDomain>`]. The corpus grows as
/// popcorn distillation batches land (Research 002 is batch 1).
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic — grows with distillation batches
pub fn build_kernel_seed_corpus() -> Corpus<KernelOptDomain> {
    let mut entries = entries_popcorn::entries();
    entries.extend(entries_gpu_38::entries());
    entries.extend(entries_gpu_39::entries());
    entries.extend(entries_gpu_40::entries());
    entries.extend(entries_gpu_41::entries());
    entries.extend(entries_gpu_42::entries());
    entries.extend(entries_gpu_43::entries());
    entries.extend(entries_gpu_44::entries());
    entries.extend(entries_gpu_45::entries());
    entries.extend(entries_gpu_46::entries());
    entries.extend(entries_gpu_47::entries());
    entries.extend(entries_gpu_48::entries());
    entries.extend(entries_gpu_49::entries());
    entries.extend(entries_gpu_50::entries());
    entries.extend(entries_gpu_51::entries());
    entries.extend(entries_gpu_52::entries());
    entries.extend(entries_gpu_53::entries());
    entries.extend(entries_gpu_54::entries());
    entries.extend(entries_gpu_55::entries());
    entries.extend(entries_gpu_56::entries());
    entries.extend(entries_gpu_57::entries());
    entries.extend(entries_gpu_58::entries());
    entries.extend(entries_gpu_59::entries());
    entries.extend(entries_gpu_60::entries());
    entries.extend(entries_gpu_61::entries());
    entries.extend(entries_gpu_62::entries());
    entries.extend(entries_gpu_63::entries());
    entries.extend(entries_gpu_64::entries());
    entries.extend(entries_gpu_65::entries());
    entries.extend(entries_gpu_66::entries());
    entries.extend(entries_gpu_37::entries());
    entries.extend(entries_gpu_36::entries());
    entries.extend(entries_gpu_01::entries());
    entries.extend(entries_gpu_02::entries());
    entries.extend(entries_gpu_03::entries());
    entries.extend(entries_gpu_04::entries());
    entries.extend(entries_gpu_05::entries());
    entries.extend(entries_gpu_06::entries());
    entries.extend(entries_gpu_07::entries());
    entries.extend(entries_gpu_08::entries());
    entries.extend(entries_gpu_09::entries());
    entries.extend(entries_gpu_10::entries());
    entries.extend(entries_gpu_11::entries());
    entries.extend(entries_gpu_12::entries());
    entries.extend(entries_gpu_13::entries());
    entries.extend(entries_gpu_14::entries());
    entries.extend(entries_gpu_15::entries());
    entries.extend(entries_gpu_16::entries());
    entries.extend(entries_gpu_17::entries());
    entries.extend(entries_gpu_18::entries());
    entries.extend(entries_gpu_19::entries());
    entries.extend(entries_gpu_20::entries());
    entries.extend(entries_gpu_21::entries());
    entries.extend(entries_gpu_22::entries());
    entries.extend(entries_gpu_23::entries());
    entries.extend(entries_gpu_24::entries());
    entries.extend(entries_gpu_25::entries());
    entries.extend(entries_gpu_26::entries());
    entries.extend(entries_gpu_27::entries());
    entries.extend(entries_gpu_28::entries());
    entries.extend(entries_gpu_29::entries());
    entries.extend(entries_gpu_30::entries());
    entries.extend(entries_gpu_31::entries());
    entries.extend(entries_gpu_32::entries());
    entries.extend(entries_gpu_33::entries());
    entries.extend(entries_gpu_34::entries());
    entries.extend(entries_gpu_35::entries());
    Corpus::new(entries)
}

/// The category → `rule_ids` mapping — the corpus wiring for [`super::router::KernelExpertRouter`].
///
/// One entry per [`KernelCategory`] (15 total). Each entry carries the category's
/// display name + the static list of `rule_ids` that belong to it. This is the
/// **router→corpus link**: given a routed expert index, a heal pipeline can look
/// up the category + its `rule_ids` without scanning the full corpus.
///
/// # Plan 003 invariants (enforced by tests)
///
/// - **Exhaustive:** every `rule_id` in the 739-rule corpus appears in exactly one
///   category (no orphan rules, no duplicate appearances; re-pinned 739 at Batch
///   197 — the mlx VJP distill; was 728 at Batch 196).
/// - **Order-stable:** categories appear in enum-discriminant order (A=0 … O=14).
/// - **Count-stable:** the per-category counts are pinned by tests.
///
/// See [Plan 003](../../../.plans/003_kernel_opt_router_corpus_wiring.md) for the
/// design rationale (one expert per category, not one per rule).
pub const KERNEL_CATEGORIES: [(KernelCategory, &str, &[&str]); 15] = [
    // A: Memory access patterns (157 rules; Batch 198 +1: the weight-decode
    // amortized across output columns (the column-axis register amortization
    // beside the staging amortizations); Batch 197 +1: the GQA zero-stride batch broadcast (the family's stride-0 member beside B182's row-fold); Batch 194 +4: the decode sidecar ring write, the l1-resident address-identity rewind, the SF sidecar own-granularity, the in-order credit collapse; was 148; Batch 188 +2: the\n    // low-bit warp-run load + the register prefetch-ring cutoff;
    // pair -- evict-last producer stores + dead-line discard;
    // Batch 180 +1: the planar-transposed activation plane layout;
    // Batch 181 +0; Batch 182 +2: the GQA repeat-fold tile-fill + the
    // verify-window prework reorder; Batch 186 +6: the ggml-cpu tiled
    // mul_mat family -- dequant-once staging, VNNI stage transpose,
    // plan-time thread slots, narrow L1 panels, jit band repack,
    // zero-pad ragged tails; Batch 187 +1: the j-major accumulator
    // buffer + the 16x8 register-transpose writeback)
    (
        KernelCategory::MemoryAccess,
        "A: memory access",
        &[
            "packed-code-cpasync-stage-per-use-prmt-decode",
            "ldmatrix-byte-subword-granularity-limit",
            "transposed-smem-stage-repairs-strided-epilogue",
            "per-block-scalar-slab-cpasync-stage",
            "smem-budget-single-buffer-refill-at-kill-points",
            "stream-invariant-operand-skips-staging-pipeline",
            "split-copy-issue-deposit-depth-uniform",
            "smem-pad-transport-gated",
            "ptxas-reschedules-load-order-wash",
            "ptxas-recoalesces-scalar-runs-wash",
            "companion-slab-ring-slot-sync",
            "software-swizzle-shift-row-stride",
            "tma-single-issuer-bulk-tile",
            "matvec-coalesce",
            "coalesced-row-read",
            "vectorized-pack-load",
            "grid-stride-loop",
            "gemm-tiling",
            "masked-boundary-load",
            "offload-cpu-bottleneck-to-gpu-kernel",
            "block-contiguous-weight-layout",
            "batch-pool-read-makes-serial-readahead-redundant",
            "dequant-once-per-k-tile-into-shared-memory",
            "hoist-group-invariant-scale-out-of-word-loop",
            "row-tile-amortize-input-load",
            "pin-lane-to-quant-sub-block-coalesce-byte-loads",
            "cooperative-block-wide-quantize-staging",
            "cross-runtime-raw-handle-extraction",
            "pool-buffer-aliasing-dedicated-output",
            "same-queue-implicit-ordering-no-sync",
            "per-dispatch-pointer-re-extraction",
            "dynamic-shared-memory-runtime-sized",
            "ternary-bit-plane-u64-as-2x-u32",
            "combined-kv-buffer-keys-then-values",
            "gqa-kv-head-sharing-bandwidth-reduction",
            "widen-index-multiply-to-64bit",
            "prebuilt-bind-groups-fixed-buffer-combos",
            "host-cache-immutable-device-weights",
            "defer-readback-until-after-submit",
            "persistent-zeros-source-encoder-copy-clear",
            "snapshot-transient-grad-for-deferred-download",
            "defer-readback-gated-by-live-output-size",
            "weight-handle-cache-keyed-by-allocation-identity",
            "sync-map-result-stack-cell",
            "staging-reuse-grow-only-map-prefix",
            "mapped-at-creation-init-upload",
            "caller-owned-split-scratch",
            "incremental-multi-part-hash-no-concat",
            "superset-sized-scratch-across-variants",
            "sigmoid-deriv-recovered-from-output",
            "needless-swap-temp-hoist-to-scratch",
            "scoped-init-at-consume-site",
            "derive-shape-from-payload-not-cached-field",
            "make-unmake-over-clone-per-candidate",
            "deterministic-tiling-projection-tables",
            "latent-encode-at-capture",
            "slot-offset-partial-buffer-write",
            "immutable-operand-upload-at-set-creation",
            "mirror-ring-store-sliding-window-contiguity",
            "preallocated-activation-slots-inplace-save",
            "smem-staging-gated-by-cross-thread-reuse",
            "side-consumer-readback-alias-live-handle-for-dependent-dispatch",
            "quadratic-attention-intermediates-recompute-from-linear-saves",
            "kv-window-device-resident-ring-append",
            "identical-tensor-handle-clone-zero-reupload",
            "tiled-host-transpose-before-quantize",
            "flat-activation-slots-as-readback-target",
            "transient-spike-pins-persistent-scratch",
            "transposed-consumption-accumulate-not-gather",
            "append-cache-rewind-overwrite-in-place",
            "slow-path-bootstraps-raw-buffer-hotswap-cache",
            "tiny-param-buffer-rebuild-over-cache",
            "readback-destination-passing-free-fn",
            "variant-owned-output-buffer-clone-escape",
            "host-scratch-mirrors-slot-order-for-coalesced-writes",
            "write-plan-derived-from-cache-slots-not-recomputed",
            "cpu-source-builder-skips-device-roundtrip",
            "foreign-accumulator-bridge-copy-hoist-to-outer-loop",
            "aliased-slot-allocate-replace-fallback-severs-binding",
            "pool-slot-write-coalescing-owns-only-declared-ranges",
            "pool-reservation-token-outlives-raw-view",
            "gpu-type-bridge-borrows-not-clones",
            "superseded-primitive-machinery-zombie",
            "expected-embedding-transposed-gemv-resident-head",
            "mutable-input-scratch-residency-write-per-call",
            "post-scatter-renorm-support-set-only",
            "stack-array-row-view-bridge",
            "stash-kernel-internal-intermediate-in-smem",
            "bit-cursor-unpack-to-width-specialized-byte-batch",
            "scratch-depth-matches-shape-invariance",
            "frozen-cooperand-hoist-out-of-perturbation-loop",
            "declared-max-array-taxed-per-thread-under-runtime-bound",
            "shared-operand-side-pair-batch-index-decomposition",
            "runtime-arg-loop-bound-demotes-arrays-to-local-memory",
            "unpack-loop-indexed-by-container-word",
            "superseded-format-constructor-live-fields-only",
            "smem-slot-role-reversal-needs-barrier",
            "async-htod-host-source-outlives-copy",
            "output-buffer-rotation-pool-multi-forward",
            "write-before-read-state-skips-checkpoint",
            "rotated-ring-undo-via-rebind-reference-snapshot",
            "reset-state-in-place-fill-never-realloc",
            "skip-unchanged-writes-via-dirty-encoding",
            "weight-stream-non-temporal-loads",
            "vectorized-smem-write-no-bank-padding",
            "virtual-tail-packed-tree-verify-over-paged-kv",
            "staged-append-rollback-lifo-retry-blocks",
            "cpasync-multistage-tile-pipeline",
            "register-double-buffer-operand-pipeline",
            "xor-swizzle-smem-static-index-tables",
            "multi-pointer-dependency-break",
            "quant-group-scale-fetch-gating",
            "barrier-after-async-mma-before-staged-tile-reuse",
            "share-activation-stream-keep-projection-reductions",
            "functional-cache-versioning-loses-donation-under-kernel-read",
            "fused-kv-write-single-writer-reads-newest-from-input",
            "never-gqa-multiply-full-d-merge-smem",
            "bucketed-sparse-reader-column-partition",
            "bank-group-circular-element-reorder",
            "hazard-tracked-aliasing-scratch-allocator",
            "sparse-demand-paged-kv-cache",
            "tensor-op-strided-device-operand-binding",
            "multi-simdgroup-shared-staged-tile",
            "register-rotation-prefetch-loop",
            "shared-operand-groups-per-warp",
            "whole-row-register-resident-under-length-budget",
            "byte-owning-threads-non-positional-digit-codes",
            "routed-activation-pack-once-shared-operand",
            "role-scoped-store-must-match-compute-guard",
            "compute-role-split-by-accumulator-pressure",
            "kv-reuse-prefix-ends-at-last-evaluated-token",
            "sm90a-window-shape-gated-wgmma-dispatch",
            "streamk-starved-tile-grid-dispatch",
            "extent-clamped-operand-views-partial-k-tile",
            "producer-store-evict-last-pins-cross-stream-consumer",
            "dead-line-l2-discard-after-last-consumer",
            "planar-transposed-activation-multi-column-quant-matvec",
            "verify-fold-gqa-repeats-into-row-axis-tile-fill",
            "gqa-zero-stride-batch-broadcast",
            "verify-window-prework-loads-before-stores",
            "dequant-once-macrotile-stage-reuse-across-microtiles",
            "stage-operand-mac-layout-inplace-transpose-at-repack",
            "plan-time-thread-slot-scratch",
            "narrow-m-l1-panel-sizing",
            "jit-band-repack-stage-per-use-l1-hot",
            "zero-pad-ragged-tail-additive-identity",
            "jmajor-acc-buffer-transpose-writeback",
            "low-bit-warp-run-load-shuffle-redistribute",
            "register-prefetch-ring-measured-cutoff",
            "kv-first-append-unzeroed-alloc-offset-gated-tail",
            "dual-layout-transpose-at-stage",
            "plane-form-activation-reuse-topology",
            "decode-sidecar-coroutine-writes-consumer-stage-ring",
            "l1-resident-operand-reuse-address-identity-rewind",
            "decoupled-sf-sidecar-pipeline-own-granularity",
            "in-order-pipe-credit-release-completion-collapse",
            "weight-decode-amortized-across-output-columns"
        ],
    ),
    // B: Compute patterns (78 rules; Batch 198 +1: the fewrow tensor tile
    // with the PQ2_0 decode fused into the RHS operand -- the A− lead of the
    // PrismML pack) -- was 64 in the comment while the
    // pinned truth was 65 (stale en passant, fixed at Batch 166); 66 at
    // Batch 166, 67 at Batch 170 (scale-identity-elide); 68 at Batch 178
    // (warp-bitonic-topk-merge-shuffle); 69 at Batch 181
    // (tensor-core-mma-staged-quantized-codes-double-buffered); 71 at
    // Batch 186 (the maddubs overflow selector + the scale-hoist epilogue);
    // 74 at Batch 187 (the register-band partition, the lut16-pshufb
    // folded-bias expansion, the GFNI sign-mask expand); 75 at Batch 189
    // (the exact bf16-domain activation LUT); 76 at Batch 193
    // (register-built-right-operand — the comment missed it, repaired at
    // Batch 195); 77 at Batch 195 (last-query-rect-projection)
    (
        KernelCategory::Compute,
        "B: compute",
        &[
            "sfu-exp-swap-pays-only-when-sfu-limited",
            "fold-invariant-factor-commutes-to-epilogue",
            "lds-cluster-cross-warp-phase-not-static-distance",
            "autotune-warps-stages",
            "upcast-before-reduction",
            "branchless-select",
            "unroll-fixed-inner",
            "forceinline-device-fn",
            "recursive-kronecker-cat",
            "optional-bias-branch",
            "shift-via-pad-subtract",
            "sorted-cumsum-threshold",
            "polynomial-taylor-feature-map",
            "positive-feature-map-shift",
            "hierarchical-log2-level-mask",
            "iterative-cg-solve-over-matvec",
            "chunked-test-time-gradient-descent",
            "expm1-for-near-zero-exp-minus-one",
            "complex-mul-via-paired-reals",
            "stateful-shift-via-cat-prefix",
            "squared-relu-activation",
            "pre-rotate-once-exploit-orthogonality",
            "cmma-hardware-unit-over-scalar-alu",
            "skip-all-zero-packed-words-via-or-reduce",
            "split-sign-accumulators-defer-subtraction",
            "clamp-tile-indices-to-sentinel-discard-at-write",
            "factor-affine-quant-scale-out-of-element-loop",
            "byte-perm-lut-via-permutation-immediate",
            "dp4a-int8-hardware-dot-product",
            "threadgroup-subset-sum-lut-keyed-by-activation",
            "warp-shuffle-intrinsics-for-sub-warp-reduction",
            "softcap-tanh-attention-score",
            "dual-accumulator-single-reduction-sweep",
            "monotone-transform-elision-under-argmax",
            "tree-reduction-depth-over-sequential-accumulate",
            "softplus-piecewise-saturation-clamp",
            "fast-math-contraction-behind-feature-flag",
            "sign-branch-stable-sigmoid",
            "per-layer-grad-norm-lr-equalization",
            "inplace-grad-accumulate-deferred-postscale",
            "monomorphize-per-element-trait-calls",
            "commit-over-maintained-item-digests",
            "length-prefix-shape-faithful-hash",
            "argmax-rng-tie-break-collect-ties",
            "non-saturating-outcome-reward-floor",
            "selective-projection-effective-rank-head",
            "rademacher-signs-over-gaussian-projection",
            "block-reduction-separate-scratch-runtime-strides",
            "softmax-entropy-via-shift-identity",
            "topk-select-nth-over-full-vocab-sort",
            "hoist-bit-address-out-of-packed-row-sweep",
            "single-sweep-minmax-for-quant-scale",
            "softmax-denominator-hoist-out-of-weighted-v",
            "per-key-projection-hoist-out-of-query-sweep",
            "regularizer-above-noise-floor-coupled-to-damping-cap",
            "masked-branchless-wait-collapse",
            "hw-barrier-beats-mbarrier-polling",
            "mma-fragment-layout-m16n8k16",
            "fwht-shuffle-xor-butterfly-register-resident",
            "fwht-butterfly-staging-ladder-warp-smem-register",
            "fused-vector-site-transcendental-form-nonportable",
            "per-row-scale-folded-into-rhs-staging-tile",
            "rope-pair-partner-via-cross-lane-shuffle",
            "degenerate-selection-falls-back-to-identity-uniform",
            "lut-free-bit-spread-vadd4-unpack",
            "direct-low-bit-generation-replaces-exp-quantize-roundtrip",
            "scale-identity-elide-in-framework-graph-transcription",
            "warp-bitonic-topk-merge-shuffle",
            "tensor-core-mma-staged-quantized-codes-double-buffered",
            "maddubs-overflow-two-exact-mac-selector",
            "hoist-row-invariant-scale-to-subblock-epilogue",
            "register-budget-band-partition-indep-correction-offband",
            "lut16-pshufb-byte-expansion-at-stage",
            "gfni-sign-mask-expand-cmpeq-fallback",
            "exact-bf16-domain-activation-lut",
            "branch-free-top2-insert-order-invariant-guard",
            "last-query-rect-projection",
            "fewrow-tensor-tile-decode-into-rhs-operand"
        ],
    ),
    // C: Fusion patterns (51 rules; Batch 195 +1:
    // roundfuse-packed-embedding-gather; Batch 181 +1:
    // fused-transform-quantize-writer; Batch 182 +1: the argmax/top-k
    // epilogue swap; Batch 186 +1: consumer-offset-absorption — the
    // comment read 49 through B186..B189, repaired at Batch 190)
    (
        KernelCategory::Fusion,
        "C: fusion",
        &[
            "fuse-norm-act-linear",
            "fuse-quant-matmul",
            "fuse-softmax-mask",
            "fuse-xent-logsumexp",
            "fuse-add-rms-norm",
            "fuse-group-norm-linear",
            "fuse-jsd-logsumexp",
            "gather-selective-reduction",
            "fuse-gate-transform-chunk-cumsum",
            "rkv-receptance-correction",
            "fold-norm-scalar-into-matmul",
            "fuse-gate-up-gemv-activation",
            "chain-dispatches-single-sync-point",
            "fuse-residual-via-in-place-output-array",
            "deferred-residual-fused-into-next-layer-norm",
            "fused-activation-quantize-for-gemv-consumer",
            "dual-grad-same-pass",
            "fan-out-shared-input-single-sync",
            "fuse-sibling-target-grads-single-kernel",
            "fused-conv-state-shift-dot-silu",
            "epilogue-at-register-write-phase",
            "delay-rms-scale-past-gemv-epilogue",
            "fuse-count-into-mutation-pass",
            "inplace-gate-scale-at-funnel-bottleneck",
            "fuse-loss-and-output-grad-one-pass",
            "fuse-weight-grad-input-grad-single-matrix-visit",
            "incremental-centroid-running-sum",
            "one-sweep-spatial-feature-fanout",
            "shape-matched-sibling-weight-grads-share-input-loop",
            "kernelize-host-chain-after-sync-batching-floor",
            "constant-scale-folded-into-upload-copy",
            "sparse-gather-scale-at-read-not-table-prepass",
            "elementwise-offload-needs-device-side-anchor",
            "fuse-indirect-gather-into-dot-consumer",
            "gate-prep-fold-into-consumer-via-packed-side-tensor",
            "fusion-payback-gated-on-per-dispatch-overhead",
            "sibling-projections-one-kernel-shared-staged-tile",
            "bit-exact-dispatch-fusion-boundary-emulation",
            "grid-range-split-verbatim-impl-composition",
            "heterogeneous-unit-paths-shared-tile-fusion",
            "stream-k-persistent-cta-mac-segments",
            "rht-epilogue-fusion-contract",
            "gumbel-argmax-filtered-sampling-descent",
            "softmax-denominator-cancels-through-scale-invariant-norm",
            "batch-verify-rows-single-dispatch",
            "fused-residual-update-writes-separate-next-buffer",
            "softmax-stats-precompute-exp-in-gemm-loader",
            "fused-transform-quantize-writer",
            "argmax-epilogue-replaces-output-materialization",
            "consumer-offset-absorption-precomputed-weight-row-correction",
            "roundfuse-packed-embedding-gather",
        ],
    ),
    // D: Dispatch patterns (165 rules; Batch 198 +2: the ids-path stride
    // rebind correctness law (the #258 per-slot bound) + the single-seq
    // contiguity copy elision; Batch 196 +1: the register-width topk tiled-radix fallback; Batch 195 +3: the write-only touch side queue + the atexit registration-order shutdown law + the lazy-admission reservation TOCTOU fix; Batch 194 +1: the multiwave dual-decode admission gate; was 152; Batch 192 +3: the MoE tile-picker
    // shape-truth + the descriptor-set binding memo with destroy-epoch +
    // the vendor-keyed reduction lane-partition; Batch 189 +2: the shape-bucketed
    // graph-cache LRU + the fused-serving measured-negative ledger;
    // Batch 188 +1: the shape-only\n    // row-invariant plan; Batch 177 +2: the host-gather graph
    // split + the unified-memory page-cache rule; Batch 178 +1: the
    // topology-signature graph-exec sharing rule; Batch 180 +1: the
    // row-kblock item-space flattening + host util search; Batch 181 +2:
    // the engine work-interval thread-shape pin + the in-flight-cap
    // split submission; Batch 182 +1: the post-idle residency-restore
    // touch scheduling; Batch 186 +3: the ggml-cpu tiled family -- the
    // batch-floor force escape, the dead-tail admission defect, the
    // early-admission owned transform)
    (
        KernelCategory::Dispatch,
        "D: dispatch",
        &[
            "nodecode-arm-bounds-overhead-elimination",
            "profile-sol-table-before-compute-rung",
            "sub-ms-bench-boost-clock-never-engages",
            "vendor-dispatch-verify-before-outsource",
            "sm-count-block-size",
            "predicate-gated-backend",
            "pow2-block-alignment",
            "predicate-gate-div512",
            "multi-condition-predicate",
            "layer-depth-decay-schedule",
            "dual-mode-gate-transform",
            "cyclic-modality-table-selection",
            "swap-large-axis-to-x-for-workgroup-distribution",
            "no-blocking-sync-in-async-pipeline",
            "cuda-graph-scalar-via-device-pointer",
            "fold-query-tokens-for-occupancy",
            "hardcoded-fast-path-jit-specialization",
            "device-side-argmax-minimal-download",
            "hot-swap-weights-preserve-autotune-caches",
            "per-slot-uniform-buffer-write-ordering",
            "derive-params-from-global-invocation-id",
            "merge-dispatches-into-single-compute-pass",
            "batched-downloads-single-sync-cycle",
            "chunk-encoder-submits-below-driver-limit",
            "staging-packed-uniforms-encoder-ordered-copies",
            "identical-uniforms-share-single-slot",
            "per-layer-strided-grad-destination",
            "token-flattened-elementwise-modulo-shared-weights",
            "split-decay-outputs-eliminate-readback",
            "nonlinearity-forces-raw-input-buffers",
            "first-readback-pays-queue-drain",
            "pack-scalar-params-into-combined-buffer",
            "launch-unchecked-caller-owned-size-contract",
            "predicate-gated-skip-empty-workset-step",
            "share-pipeline-set-across-subsystems",
            "ragged-pack-offset-table-fused-dispatch",
            "width-invariant-superset-training",
            "static-classification-hoist-to-routing-table",
            "verify-once-at-trust-boundary",
            "order-stable-topk-via-index-sort",
            "shape-gated-warm-start-slot-copy",
            "domain-as-data-not-algorithm-fork",
            "timeout-scalar-tiebreak-decisive-outcome",
            "buffer-set-struct-two-tier-dispatch",
            "split-dispatch-expose-fused-intermediate",
            "hybrid-cpu-elementwise-gpu-throughput-sync-points",
            "op-major-backward-loop-inversion-batch-sequence-axis",
            "side-branch-operand-residency-matches-mutation-lifetime",
            "activation-checkpoint-as-handle-clone-deferred-batch-readback",
            "training-forward-shed-decode-loop-shape",
            "sibling-dispatches-before-any-readback",
            "independent-cpu-work-fills-dispatch-read-window",
            "lm-head-loss-positions-only",
            "materialized-fast-path-bulk-slice-accessor",
            "env-flag-resolved-once-not-per-dispatch",
            "batched-read-slot-membership-decided-at-launch",
            "entrypoint-variants-share-layer-body",
            "pipeline-layer-boundary-device-handle",
            "per-position-scalar-factored-gemv-collapse",
            "refine-loop-invariant-prefix-elision",
            "runtime-identity-gate-cached-verdict-elides-side-projection",
            "empty-slice-sentinel-launders-shape-violation",
            "share-cube-size-const-between-kernel-and-launcher",
            "serialize-container-canonical-order",
            "size-gate-covers-every-cost-factor",
            "shared-producer-prefix-extract-once-per-input",
            "phase-split-per-position-data-liveness-row-storage",
            "buffer-length-derived-from-allocation-not-binding",
            "concurrent-context-init-needs-process-wide-lock",
            "gpu-api-thread-affinity-snapshot-metrics",
            "tile-loop-back-edge-needs-multi-tile-bitexact-gate",
            "captured-graph-bakes-buffer-addresses",
            "mid-pipeline-device-alloc-is-implicit-sync",
            "capture-region-dispatch-only",
            "same-input-gemv-segments-single-launch",
            "same-input-gemv-weight-concat-at-upload",
            "barrier-free-per-dependency-wait-from-loop-counters",
            "shared-counter-decode-producer-progress",
            "shared-activation-quantize-memo-per-eval",
            "vendor-batched-gemm-contiguity-staging",
            "ordered-lhs-skip-smem-fragments-as-second-buffer",
            "interleave-loads-between-tile-matmuls",
            "dispatch-cost-sequential-measurement-not-single-op",
            "cross-lane-column-packed-projections",
            "identity-keyed-graph-retention-stable-arena",
            "fail-closed-multi-resource-commit-transaction",
            "work-scaling-probe-launch-bound",
            "pad-tail-to-fixed-tile",
            "reduction-restructure-needs-source-owned-comparator",
            "cross-run-profiles-key-on-content-not-stat-triplets",
            "fill-gate-speculation-until-working-set-resident",
            "decayed-frequency-eviction-recency-tiebreak-route-protection",
            "defer-promoted-write-to-first-consumer-dispatch",
            "rollback-snapshot-covers-all-mutable-state",
            "scratch-bank-isolates-one-shot-operands-from-hot-cache",
            "sort-consumer-routes-by-physical-slot",
            "row-addressable-operand-index-not-full-tensor-map",
            "wide-row-copy-tile-rows-across-workgroups",
            "monomorphic-hot-caller-last-hit-memo",
            "in-place-write-through-input-emits-fence-consumer",
            "residency-set-wired-once-post-warm-persistent-footprint",
            "container-rebind-over-retrace-for-churn-survival",
            "shape-class-keyed-capability",
            "wavefront-overlap-pair-ceiling",
            "request-shape-decline-mirror-completeness",
            "backing-provenance-causal-shadowed-extent-widen",
            "dispatcher-eligibility-predicate-lags-kernel-capability",
            "concurrent-dispatch-dependency-level-barriers",
            "pipelined-token-slot-ping-pong-submit-ahead",
            "a8-activation-route-stability-guard",
            "prefix-reuse-key-must-mirror-the-restore-predicate",
            "request-arrival-work-asks-reuse-state-first",
            "serving-raises-only-behind-strict-flag",
            "derived-key-single-constructor-parity-test",
            "vendor-fused-op-instantiation-check",
            "prefill-chunk-command-buffer-capacity-coupled",
            "captured-graph-init-unconditional-and-stream-ordered",
            "batch-invariance-locks-reduction-geometry",
            "grid-dependency-wait-scoped-to-dependent-operands",
            "refine-by-rescan-prefix-filter-vs-buffered-ties",
            "ane-bank-split-ladder",
            "carried-state-reduction-stays-scalar",
            "ordinal-map-exact-tail-selection",
            "complete-coverage-scatter-collapses-operand-to-dense",
            "env-gate-reaches-and-arms-the-scored-process",
            "dispatch-cliff-bitexact-shape-confinement",
            "readout-trim-needs-committed-id-coverage",
            "dead-output-rows-state-append-only-path",
            "scratch-sizing-shares-dispatch-eligibility-predicate",
            "shader-shape-cutover-keys-on-resource-block-ratio",
            "same-input-folded-transform-memo-per-build",
            "speculation-scheduler-break-even-flip",
            "new-syntax-family-enumerates-every-keyed-surface",
            "one-time-dense-repack-keyed-sync-publish",
            "sustained-load-drift-interleave-protocol",
            "sentinel-operand-encoding-routes-fused-kernel",
            "host-gather-splits-the-graph-and-the-split-is-the-separable-price",
            "unified-memory-page-cache-is-device-allocatable",
            "graph-exec-update-gated-by-captured-topology-signature",
            "row-kblock-item-space-flattening-host-util-search",
            "engine-work-interval-performance-core-pin",
            "split-submission-in-flight-cap-overlap",
            "idle-gpu-residency-restore-overlapped-behind-queued-work",
            "regime-batch-floor-force-escape",
            "unified-admission-dead-tail",
            "owned-input-transform-early-admission",
            "shape-only-plan-row-invariant-kernel",
            "shape-bucketed-graph-cache-lru",
            "fused-serving-stack-measured-negative-ledger",
            "tile-picker-input-from-dispatch-grid-partition",
            "descriptor-set-binding-memo-destroy-epoch",
            "vendor-keyed-reduction-lane-partition-subgroup-request",
            "first-command-buffer-fresh-buffer-mapping-split",
            "deadline-bounded-poll-of-submitted-completion-event",
            "readback-domain-excludes-cold-tap-copies",
            "adoption-staged-leading-layers-gpu-idle-fill",
            "seed-first-command-buffer-earliest-submission",
            "adoption-trial-rotate-operands-per-launch-dram-streaming",
            "multiwave-dual-decode-admission-gate",
            "write-only-touch-side-queue",
            "atexit-registration-order-shutdown",
            "lazy-admission-reservation-atomic-register",
            "register-width-topk-falls-back-to-tiled-radix-select",
            "ids-path-rebinds-stride-meaning",
            "single-seq-slice-contiguity-copy-elision"
        ],
    ),
    // E: Parallelism patterns (70 rules; Batch 197 +2: the composed-backward A− GOAT lead (tiled library GEMMs through a bounded materialized tile — the fused kernel lost, closed PR #3241) + the threadgroup-scope reduction single-writer; Batch 195 +1: the cross-queue output-retention-until-completion rule; Batch 194 +1: the parity stage-ownership scale lane; was 65; Batch 178 +2: the warp-role
    // specialization by query width + the row-slice epilogue rule;
    // Batch 180 +1: the fixed-order interleaved-accumulator epilogue;
    // Batch 182 +2: the draft-block KV-split simdgroup exchange + the
    // redundant small reduction; Batch 187 +1: the chunk-downsize
    // work-stealing microtile-floor ladder)
    (
        KernelCategory::Parallelism,
        "E: parallelism",
        &[
            "spill-fragility-law-accumulator-regcap-cliff",
            "tile-shrink-occupancy-pays-feed-economics",
            "left-right-fold-pair-proves-associative-combine",
            "defensive-sync-peephole-tile-body-scope",
            "splitk-atomic-free-contention-threshold",
            "splitk-width-shape-need-gate",
            "setmaxnreg-fit-gate-elected-arrive",
            "split-k-atomic-arm-legality",
            "multiplexed-grid-axis-partition-modes-exclusive",
            "multi-dim-grid-launch",
            "block-count-clamp",
            "backward-as-conjugate",
            "reverse-scan-via-double-flip",
            "pairwise-distance-decay-matrix",
            "softmax-topk-renormalize-scatter",
            "split-reduction-scalar-prepass",
            "single-plane-sufficient-reduction",
            "broadcast-uniform-skip-reduce",
            "workgroup-equals-plane-native-reduction",
            "smem-broadcast-scalar-then-parallel-scatter",
            "epilogue-must-be-workgroup-local",
            "counter-derived-seed-stateless-noise",
            "additive-sigmoid-gates-not-softmax-routing",
            "collect-then-resolve-snapshot-batch",
            "grid-over-output-thread-private-sweep",
            "mask-sentinel-matches-reduction-identity",
            "concurrent-disjoint-write-buffer-parallelizes-uploads",
            "row-dot-split-inner-axis-block-reduce",
            "multi-head-out-proj-consumes-head-concat",
            "occupancy-grid-cap-refuted-uncapped-default",
            "plane-specialization-load-compute-role-split",
            "grid-collapsing-mega-kernel-occupancy-trap",
            "occupancy-before-async-consumer-ada",
            "capture-break-count-not-perf-target",
            "stripe-sms-l2-semaphore-reduce",
            "simdgroup-row-packing-reduction-amortization",
            "tail-simdgroup-cull-barriers-preserved",
            "virtual-thread-folding-preserves-reduction-tree",
            "tiny-rows-scalar-thread-per-row-downshift",
            "cross-simd-bitonic-alternating-bank-exchange",
            "register-budget-head-split-gqa-reread-trade",
            "batch-composition-invariant-grid-early-return",
            "target-tile-split-k-deterministic-two-dispatch",
            "packed-total-order-multi-field-argmax",
            "radix-select-ordered-key-topk",
            "independent-accumulator-chains-hide-load-latency-at-capped-occupancy",
            "byte-cut-loses-when-plateau-is-issue-bound",
            "spin-barrier-needs-co-residency-capped-launch",
            "ballot-matrix-tie-rank",
            "m-group-shared-weight-scan-register-relief",
            "row-parallelism-ladder-split-vs-pack",
            "distinct-index-bitmap-ascending-compaction",
            "fwht-wide-block-row-per-threadgroup-exchange-ladder",
            "sentinel-readiness-over-separate-flag",
            "ll-flag-in-data-packed-poll",
            "interleave-independent-accumulators-only",
            "narrow-matvec-one-row-per-simdgroup",
            "tier-mismatched-grid-pruning-relaunch",
            "compile-out-rare-paths-register-worst-case",
            "wide-verify-warp-specialized-scores-in-registers-pv",
            "row-slice-expensive-epilogue-across-ctas",
            "fixed-order-interleaved-accumulator-epilogue",
            "draft-block-simdgroup-kv-split-smem-softmax-exchange",
            "simdgroup-redundant-small-reduction-over-barrier-broadcast",
            "chunk-downsize-work-stealing-microtile-floor",
            "redundant-row-reduction-per-width-block-grid-split",
            "dual-consumer-alternating-stage-ownership-scale-lane",
            "cross-queue-output-retention-until-completion",
            "sdpa-backward-as-tiled-matmul-bounded-materialization",
            "threadgroup-scope-reduction-single-writer",
        ],
    ),
    // F: Flash-attention tiling (18 rules; Batch 194 +1: the deferred-permutation bitmask; was 17; Batch 189 +1: the ancestor-mask
    // bit-visibility block-skip tree attention with its measured
    // dispatch cross-over)
    (
        KernelCategory::FlashAttnTiling,
        "F: flash-attn tiling",
        &[
            "twisted-monoid-dependent-reduce-fusion",
            "online-softmax-streaming",
            "exp2-log2-base2",
            "block-pointer-2d",
            "heuristic-compile-dispatch",
            "causal-diag-offdiag-split",
            "shared-mem-aware-tile",
            "kv-block-streaming",
            "strict-causal-mask-tril-offset",
            "block-local-reference-frame-for-overflow",
            "two-stage-mask-softmax-then-conv",
            "mask-block-classification-skip-placement",
            "split-k-two-pass-attention-decode-long-context",
            "dense-fast-path-bounded-by-selector-coverage",
            "kv-split-decode-underfill-per-slot-validity",
            "fixed-ref-softmax-decode-walk",
            "ancestor-mask-bit-visibility-block-skip-tree-attention",
            "paged-page-pair-deferred-permutation-bitmask"
        ],
    ),
    // G: Linear-attention recurrence (34 rules; Batch 197 +3: the segmented entry-state checkpoint with the cheap-replay backward (the register-posture arm folded), the forward-save cache of the expensive chunk-local intermediates, and the chain-rule postprocess kernel; the comment read 30 while the table held 31 since B189's two-chunk overlap — the stale-comment miss repaired here; Batch 189 +1: the
    // two-chunk state-independent overlap scheduling rule)
    (
        KernelCategory::LinearAttnRecurrence,
        "G: linear-attn recurrence",
        &[
            "state-forget-gate",
            "householder-rank1-update",
            "chunkwise-parallel-recurrence",
            "recurrent-vs-chunk-dispatch",
            "multi-write-per-step",
            "chained-two-pass-recurrence",
            "per-key-dim-diagonal-gate",
            "erase-replace-low-rank-transition",
            "oja-subtractive-normalization",
            "identity-plus-low-rank-delta",
            "cumsum-gate-to-decay-matrix",
            "gate-then-correct-delta-rule",
            "gate-then-multi-householder-write",
            "online-gram-ridge-solve",
            "momentum-gated-memory-recurrence",
            "current-token-bonus-read-before-update",
            "wkv-logsumexp-three-channel-state",
            "row-owned-register-blocked-recurrence",
            "reverse-running-decay-weight",
            "tree-verify-recurrent-state-parent-reload",
            "first-order-affine-recurrence-associative-scan",
            "gdn-transition-replay-log-commit-many",
            "wy-transform-tree-verify-state-free",
            "state-normalization-between-updates-breaks-associative-scan",
            "dynamic-kernel-params-break-prefix-scan",
            "whole-sequence-zero-barrier-single-dispatch-scan",
            "smem-envelope-gate-kernel-variant",
            "recurrence-prefill-exact-restructure-before-reformulation",
            "chunk-transition-consumed-by-mma",
            "two-chunk-state-independent-overlap-serial-chunk-recurrence",
            "segmented-checkpoint-cheap-replay-backward",
            "cache-symmetric-intermediates-per-chunk",
            "chain-rule-postprocess-kernel",
            "full-accept-state-store-host-observable-predictor",
        ],
    ),
    // H: MoE dispatch + backward pass (36 rules; Batch 194 +5: the distance-K ring-overwrite gate, the empty-participant credit, the 32 MiB overlap-reduce mode, the coprime source interleave, the dependency-wave lookahead; was 31; Batch 188 +1: the\n    // sorted-gather run-scan elimination + warp-scan scheduler;\n    // Batch 186 +1: the per-expert mixed dispatch + worst-case\n    // workspace rule)
    (
        KernelCategory::MoeDispatchBackward,
        "H: MoE dispatch + backward",
        &[
            "grouped-launch-cumsum-offsets",
            "l2-grouped-tile-schedule",
            "fused-matmul-activation",
            "fused-backward-chain",
            "activation-recompute-budget",
            "atomic-scatter-reduction",
            "weight-grad-batched-outer-product",
            "csr-index-via-prefix-sum",
            "autotune-aggregate-workload-key",
            "active-index-grad-scatter-subset-backward",
            "backward-helper-expose-shared-norm-statistic",
            "adapter-grad-join-mirrors-forward-delta-insertion-point",
            "grad-terms-merge-before-shared-link-jacobian",
            "expert-loop-two-phase-split-batch-across-iterations",
            "expert-major-work-map-token-scatter-clamped",
            "moe-routed-down-sum-single-accumulate-pass",
            "lpt-min-heap-disjoint-column-ownership",
            "gpu-resident-moe-routing-no-host-sync",
            "indirect-device-dispatch-tile-map-moe",
            "lane-sequenced-deterministic-bucket-scatter",
            "single-threadgroup-fused-count-scan",
            "warp-aggregated-atomic-scatter",
            "routing-by-precomputed-lookup",
            "tile-amplification-precedes-staging-pass",
            "cold-large-prefill-whole-layer-lookahead-over-partial-cache",
            "static-seed-priority-bounded-to-observed-scale",
            "moe-broadcast-quantize-once-scatter",
            "expert-remainder-routed-to-smaller-tile-twins",
            "expert-major-tile-order-token-tile-cache-resident",
            "moe-per-expert-mixed-dispatch-worst-case-workspace",
            "sorted-gather-tile-compaction-warp-scan-scheduler",
            "ring-overwrite-gated-k-back-readiness",
            "empty-participant-progress-credit",
            "work-size-gated-overlap-reduce-mode",
            "coprime-stride-cross-rank-source-interleave",
            "dependency-wave-task-picker-one-wave-lookahead"
        ],
    ),
    // I: NSA block-sparse selection (7 rules)
    (
        KernelCategory::NsaBlockSparse,
        "I: NSA block-sparse",
        &[
            "block-sparse-gather",
            "precompute-scalar-reduction",
            "block-compression-before-routing",
            "sliding-window-plus-sparse",
            "sparse-kv-index-list-sentinel-padded",
            "running-threshold-prune-compaction-aliasing-guard",
            "tile-max-prefiltered-exact-topk",
        ],
    ),
    // J: Varlen packing + chunked reductions (5 rules)
    (
        KernelCategory::VarlenChunkedReduction,
        "J: varlen + chunked reduction",
        &[
            "varlen-packing",
            "chunk-reset-cumsum",
            "cumsum-additive-bias",
            "chunk-split-mean-pool",
            "ragged-chunk-pad",
        ],
    ),
    // K: Autotune dispatch (14 rules; Batch 194 +1: the issue-latency MAD cost model)
    (
        KernelCategory::AutotuneDispatch,
        "K: autotune dispatch",
        &[
            "stage-domain-product-of-independent-knobs",
            "panic-safe-autotune-candidate",
            "multi-priority-tune-groups",
            "shape-kind-dispatch",
            "vector-width-pow2-sweep",
            "autotune-cache-extract-outlives-owner",
            "swept-shape-bucket-launch-tables",
            "no-harm-bucket-representative",
            "persistent-kernel-length-bucketed-mechanism-ladder",
            "launch-heuristic-pinned-to-calibration-arch",
            "offline-device-tuned-table-dispatch",
            "tuned-config-resource-limit-fallback",
            "batch-width-aware-partition-shrink",
            "issue-latency-priced-mad-cost-model"
        ],
    ),
    // L: Conv-as-GEMM + memory layout (19 rules; Batch 196 +1: the affine block address pipeliner lead; the comment read 17 while the table held 18 since B193's register-built add — the stale-comment miss repaired here)
    (
        KernelCategory::ConvGemmLayout,
        "L: conv-as-GEMM + layout",
        &[
            "c-fragment-to-a-operand-register-repack",
            "ldmatrix-pair-x2-to-x4-adjacency",
            "conv-1x1-as-matmul",
            "contiguity-guard-before-reshape",
            "cube-count-bounded-batch-pack",
            "stride-align-vectorization-key",
            "anchor-shape-bucketing",
            "carry-dual-layout-dead-slot",
            "twin-projection-shared-delta-adapter",
            "bool-fields-over-bitset-hot-reads",
            "pre-transposed-consumer-layout-handles-build-once",
            "stream-drain-free-source-during-build",
            "fold-tile-transpose-into-dequant-scatter",
            "ldmatrix-mma-layout-loads",
            "dequant-byte-pairs-as-aligned-u16-loads",
            "device-affine-scratch-admission-with-pool-fallback",
            "window-recurrence-halo-snapshot-block-parallel",
            "register-built-right-operand-lane-contiguous-plane-layout",
            "affine-block-address-lets-pipeliner-fly",
        ],
    ),
    // M: Kernel parametric polymorphism (21 rules; Batch 186 +1: the
    // const-generic format unification rule)
    (
        KernelCategory::KernelPolymorphism,
        "M: kernel polymorphism",
        &[
            "rewrite-decline-names-refusing-predicate",
            "op-family-type-dispatch",
            "inplace-output-alias",
            "layout-permute-for-coalescing",
            "reshape-rank-for-backend",
            "detached-broadcast-insert",
            "enum-dispatch-separate-kernel-per-variant",
            "zero-init-side-branch-frozen-base-path",
            "min-rank-inert-adapter-zero-alpha",
            "debug-only-gpu-label-macro",
            "batch-width-routed-dual-tile-twins",
            "device-m-runtime-mblock-graph-stable-gemm",
            "proven-shape-static-sibling-identical-results-fallback",
            "cooperative-tensor-mma-capability-route",
            "small-batch-matvec-gemm-bridge-rung",
            "lane-owned-topk-output",
            "derived-index-from-pinned-checkpoint-is-a-cache",
            "warm-reachable-dispatch-signatures-before-timed-window",
            "row-aware-indexing-reuses-static-single-token-kernel",
            "pipeline-cache-key-covers-every-function-constant",
            "const-generic-microkernel-format-unification",
        ],
    ),
    // N: dtype + alignment specialization (57 rules; Batch 198 +1: the tied-Hadamard output fold at load (the versioned metadata contract); Batch 195 +1: the enumerable-domain bit-identity admission gate; Batch 194 +1: the FP4 gather-LUT replicated index width; was 53; Batch 192 +1: the dot
    // macro family that widens at load + f32 accumulation — the CPU refusal
    // pole of the accumulator-dtype family; Batch 177 +2: the
    // residency-audit + bpw-axis rules, both quantization-format decisions;
    // Batch 178 +1: the exact-widening accumulator-precision rule;
    // Batch 180 +1: the deferred-bias exact-isum integer algebra;
    // Batch 186 +1: the convert-once metadata widening)
    (
        KernelCategory::DtypeAlignment,
        "N: dtype + alignment",
        &[
            "smem-fill-align-chunk-not-element",
            "dtype-specialized-float-kind",
            "alignment-gated-zero-copy-view",
            "int2-pack-weights",
            "per-token-activation-quant",
            "ternary-weight-quant",
            "upcast-before-loss",
            "batched-upcast-map",
            "dtype-restricted-backend",
            "water-fill-tail-dim-shared-codebook",
            "norm-decoupled-low-bit-scoring",
            "base-3-trit-packing-5-per-byte",
            "pad-uniform-pod-to-min-binding-size",
            "dtype-normalize-once-at-upload",
            "derived-scalar-host-computed-in-uniform",
            "exact-op-parity-for-analytic-backward",
            "saturating-nonlinearity-recovery-breaks-near-cap",
            "verbatim-upload-on-format-match",
            "rmsnorm-backward-gamma-scope-correction-term",
            "dequant-placement-by-component-traffic-share",
            "outlier-span-escapes-quantization-via-lossless-sidecar",
            "pre-gate-recovery-by-division-cancels-algebraically",
            "token-local-only-approximate-accelerator",
            "lop3-int4-to-half2-dequant",
            "prmt-int8-to-half2-dequant",
            "telescoped-digit-coefficients-byte-fma-dot",
            "bit-plane-plane-sums-input-side-offset-dot",
            "half-domain-exact-band-dequant-with-raw-bit-oracle",
            "tile-coo-sparse-tile-aligned-format",
            "integer-mma-span-epilogue-dequant",
            "router-math-stays-fp32",
            "ue8m0-pow2-scale-as-biased-exponent-byte",
            "coarse-precision-first-exact-refine",
            "shader-fastdiv-mulhi",
            "exhaustive-domain-input-patch-bit-identity",
            "activation-bitplane-popcount-word-dot",
            "swar-lane-widened-base3-dp4a-dot",
            "operand-precision-gated-by-consumer-invariance",
            "fixed-point-parity-needs-reference-rounding",
            "per-block-int32-chunk-rescale-fp32-accum",
            "kv-dequant-materialization-batch-threshold-gate",
            "quantize-scale-math-wide-precision-denorm-safe",
            "block-quant-weight-pad-never-extends-activations",
            "fast-accumulate-periodic-promote-bounded-error",
            "quantized-pack-residency-audit-unquantized-minority-first",
            "bpw-reduction-buys-memory-not-decode-when-dequant-bound",
            "exact-widening-enables-fp16-pv-accumulation",
            "deferred-bias-ternary-dp4a-exact-isum",
            "signed-activation-codes-drop-dequant-offset-term",
            "proven-redundant-operand-load-elision",
            "dtype-native-source-read-with-shared-admission-predicate",
            "widen-metadata-once-to-mac-native-widths",
            "dot-macro-family-widens-at-load-f32-accumulate",
            "construction-identity-row-classes-replace-probe",
            "fp4-nibble-gather-lut-replicated-index-width",
            "enumerable-domain-bit-identity-admission-gate",
            "tied-hadamard-output-fold-at-load"
        ],
    ),
    // O: IR strength reduction (7 rules; Batch 195 +2: the io32
    // count-pinned kernel-source-text narrowing (the TEXT-level static
    // member) + the strided-view offset fit32 launch proof (the
    // launch-proven arbitrary-view member — the family's admission half);
    // Batch 190 +1: the runtime-divisor umax clamp that admits LSR — the
    // family's first RUNTIME-divisor member, generalizing the constant
    // pow2 shift arms)
    (
        KernelCategory::IrStrengthReduction,
        "O: IR strength reduction",
        &[
            "strength-reduce-mul-pow2",
            "strength-reduce-mul-pow2-plus-minus-one",
            "strength-reduce-div-pow2",
            "strength-reduce-mod-pow2",
            "clamp-runtime-divisor-for-lsr-address-reduction",
            "io32-count-pinned-text-narrowing",
            "strided-view-offset-fit32-launch-proof",
        ],
    ),
];

/// Group the flat corpus entries by [`KernelCategory`].
///
/// Returns 15 buckets (one per category), each containing the `KernelRule`s whose
/// `rule_id` appears in that category's slice of [`KERNEL_CATEGORIES`]. This is
/// the category-scoped view of the corpus — a future heal pipeline can consult
/// routed expert's rules first without scanning all 351.
///
/// # Allocation
///
/// Allocates the grouped `Vec` (one per call). This is NOT a hot-path function
/// — it's a corpus-construction helper, called once at boot or test setup. The
/// per-category rule consultation is `O(rules_in_category)` via the static
/// `KERNEL_CATEGORIES` slices.
#[must_use]
pub fn build_corpus_categorized() -> Vec<(KernelCategory, Vec<KernelRule>)> {
    let corpus = build_kernel_seed_corpus();
    // Index rule_id → entry for O(1) category-bucket lookup.
    let by_id: HashMap<&str, &KernelRule> = corpus
        .entries
        .iter()
        .map(|e| (e.rule_id.as_str(), e))
        .collect();
    KERNEL_CATEGORIES
        .iter()
        .map(|(cat, _label, rule_ids)| {
            let rules: Vec<KernelRule> = rule_ids
                .iter()
                .filter_map(|rid| by_id.get(*rid).copied().cloned())
                .collect();
            (*cat, rules)
        })
        .collect()
}

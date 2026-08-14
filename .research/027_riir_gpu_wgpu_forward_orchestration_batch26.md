# Research 027 — riir-gpu wgpu Forward-Orchestration Cluster (Batch 26): Per-Slot Uniforms + Grid-Derived Params + Single-Pass Dispatch Merging + Batched Readbacks + Chunked Submits + Prebuilt Bind Groups + Host-Cached Weights + Deferred Readback

**Date:** 2026-08-14
**Source:** `riir-ai/crates/riir-gpu/src/forward/` (8030 lines: mod.rs 1733, dispatch.rs 1687, adapter_merge.rs 984, forward_buffers.rs 958, advanced.rs 703, batched_dispatch.rs 522, forward_params.rs 240, layout_tests.rs 291, test_helpers.rs 865, tests.rs 47) + `src/buffer.rs` (the download substrate forward/ consumes; deep-read L40-280) — the thirteenth batch mining the stack's own kernels.
**Corpus:** 169 → **177 rules** (A 22→25, D 15→20).

## 1. Coverage

Deep-read: forward `mod.rs` (full — GpuForwardPass structure, P0-1 pre-built BG construction, cached weights, forward/forward_per_position/forward_bidirectional/forward_block_causal), `dispatch.rs` (full — set-causal attention, dispatch_layer, dispatch_layer_batched, dispatch_deltanet_layer, D2F two-phase helpers), `batched_dispatch.rs` (full — batched elementwise/matmul/attention/LoRA), `adapter_merge.rs` (L360-790 — LoRA merge, fused QKV, SPEFT deferred corrections, GPart), `forward_buffers.rs` (full — activation + scratch buffer definitions, per-head uniforms, pre-built BG construction), `advanced.rs` (grep-swept + targeted reads — plain matmul, IA3, attention variants, draft forward, download_logits_into), `buffer.rs` L40-280 (DownloadStaging, download_f32, download_f32_reuse_into, download_f32_batched_reuse_into). Skimmed: forward_params.rs (param structs), the three test files. Honest file count: **38 of ~200 mined** (~19%).

This batch is thematically distinct from Batches 14-25: those mined KERNEL-INTERNAL patterns (WGSL/CUDA/CubeCL compute). This one mines the **wgpu host-orchestration layer** — the dispatch discipline around the kernels: uniform-write hazards, pass structure, submission granularity, bind-group lifetime, download cycles. No previous batch touched this layer (verified: zero corpus rules mentioning bind groups, uniforms, write_buffer, encoders, or submission before Batch 26).

## 2. The eight new rules

### 2.1 `per-slot-uniform-buffer-write-ordering` (D)

Source: forward/mod.rs L141-148 (uniform_relu: "Without this, the shared uniform_elem gets overwritten by the last write_uniform call before the encoder runs (wgpu write_buffer ordering bug), causing ReLU to see count=n_embd instead of count=mlp_hidden") + L361-390 (three per-slot `Vec<Buffer>` families for batched matmul/elementwise/LoRA) + forward_buffers.rs L550-557 + L622-635 (per-head `attn_uniforms`, "Always allocated ... because the standard attention path also needs them to avoid the wgpu write_buffer ordering bug (all heads' write_buffer calls flush before the encoder runs, so all heads see the last write)").

The hazard: wgpu queues `queue.write_buffer` calls — ALL writes flush BEFORE the recorded encoder executes. N dispatches in one encoder sharing ONE uniform buffer therefore all read the LAST write. This silently corrupted per-head attention params, per-target LoRA params, and per-slot matmul params across the Issues 366/367/368 lineage (attention pos-uniform, stale LoRA saved buffers, uniform slot conflict → zero grad_hidden). The fix is structural and applied at 6+ distinct sites: one small (64-byte) uniform buffer per dispatch SLOT — per head, per adapter, per QKV target, per batched matmul slot — each wired into its own pre-built bind group, so concurrent writes in one encoder cannot collide.

Distinct from `same-queue-implicit-ordering-no-sync` (A — queue-level ordering BETWEEN dispatches): this is the uniform WRITE channel racing the encoder, not dispatch ordering.

### 2.2 `derive-params-from-global-invocation-id` (D)

Source: forward/dispatch.rs L316-321 ("the batched forward (`forward_batched_full`) sidesteps it by computing q_pos from the global invocation ID") + batched_dispatch.rs L280-350 (`dispatch_attention_batched`: "One workgroup per (head, query_position). Total = n_head × seq_len ... The kernel implements causal masking internally (position q attends to KV positions 0..=q). GQA mapping is computed inside the kernel").

The per-position path needs a per-position uniform write — which under the write_buffer ordering hazard forces an **encoder + submit PER POSITION** (the set-causal/block-causal fallback, L311-325 + L1646-1658: "each position gets its OWN encoder + submit so the per-position uniform write (query_pos) is committed before the kernel runs ... Cost: one extra submit per position"). Encoding the parameter in the DISPATCH GRID eliminates the uniform channel entirely: flatten (head, q_pos) into the X dimension (n_head × seq_len workgroups, ONE dispatch) and decode in-kernel. Zero per-position uniform writes, zero per-position submits.

Distinct from `fold-query-tokens-for-occupancy` (Batch 23 — folding fills idle SMs when n_head < SM_count; here the goal is eliminating the host-side parameter channel) and from `grid-stride-loop` (iteration coverage — the grid IS the parameter).

### 2.3 `merge-dispatches-into-single-compute-pass` (D)

Source: forward/adapter_merge.rs L410-424 (`dispatch_lora_merge`: "Combine A + B dispatches into single compute pass. Dispatches within a compute pass execute in order, so B reads A's output.") + L654-770 (`dispatch_qkv_fused`: "Saves 2 compute passes (was 3 separate A+B passes → 1 combined pass). Flow: 1. Hardware copy input to per-adapter saved-input buffers (no compute pass) 2. Single compute pass with 6 dispatches").

Dependent dispatches do NOT need separate compute passes: pass-scoped `set_pipeline`/`set_bind_group`/`dispatch` sequences execute IN ORDER with memory ordering between them, so the consumer dispatch reads the producer's buffer writes. The QKV LoRA path merges 6 dispatches (3× A + 3× B) into one pass; the interleaved pure input-save copies ride the COPY channel (`encoder.copy_buffer_to_buffer` — no pass, no pipeline, no bind group) instead of copy kernels.

Distinct from `chain-dispatches-single-sync-point` (Batch 19 — command-buffer-level chaining to amortize SYNC points; this is PASS-level merging where the one sync was never in question). Composes with 2.1: all uniform writes happen before the pass begins, into per-slot buffers.

### 2.4 `batched-downloads-single-sync-cycle` (D)

Source: buffer.rs L263-280 (`download_f32_batched_reuse_into`, Issue 421 P0.5): "All copies share one command encoder, one `queue.submit`, one `map_async`, and one `device.poll(Wait)`. This reduces GPU sync overhead from N (one per download) to 1 — the dominant cost in launch-overhead-bound paths like the LoRA-Muon optimizer step, where P0 profiling measured each sync at ~1.28 ms regardless of data size."

Mechanics: each request is packed into a contiguous, non-overlapping, 4-byte-aligned region of a single grow-only staging buffer (wgpu 23 requires copy size/offset multiples of 4, not 256); all copies go in ONE encoder; ONE submit; ONE map over the whole range; ONE poll; slice per request. The 4-buffer LoRA-Muon step saves ~3.8 ms/step.

Distinct from `device-side-argmax-minimal-download` (Batch 25 — download SIZE) and `chain-dispatches-single-sync-point` (dispatch SYNC): this is readback-path sync COUNT with the staging-layout + single-map mechanics.

### 2.5 `chunk-encoder-submits-below-driver-limit` (D)

Source: forward/mod.rs L1403-1411 ("Chunked encoder submission to avoid Metal compute pass limit. Each position creates ~24 compute passes (rmsnorm×2, copy×2, lora_a×6, lora_b×6, attn×4, add×2, relu×1, lm_head×1). Metal deadlocks with too many compute passes per encoder. CHUNK_SIZE=2 → ~48 passes/chunk. Issue 016: chunk_size is now tunable via GpuContext (env var RIIR_GPU_CHUNK_SIZE)") + L1456-1458 ("Submit each chunk immediately to avoid Metal driver stall from accumulating too many write_buffer + command buffers before submit").

Two distinct driver constraints in one rule: (a) a compute-pass-per-encoder ceiling past which Metal DEADLOCKS (hard failure, not slowdown); (b) a stall from accumulating too many write_buffer + command buffers before submit. The discipline: chunk recorded work (~48 passes/encoder at chunk_size 2), submit each chunk IMMEDIATELY, and make the chunk size env-tunable — tiny-model training loops shrink it, long-prefill inference raises it.

Distinct from `chain-dispatches-single-sync-point` (a sync-count CHOICE): this chunking is a driver-limit-forced submission granularity — correctness bound first, tuning knob second.

### 2.6 `prebuilt-bind-groups-fixed-buffer-combos` (A)

Source: forward_buffers.rs L530-548 + L702-703 ("Pre-built bind groups (avoids create_bind_group per dispatch)" ... "These eliminate ~768 create_bind_group calls per forward pass (seq_len=16, n_layer=6)") + mod.rs L412-501 (P0-1 per-adapter LoRA bind groups: "Each adapter has fixed input/output buffers determined by its target, so we can pre-build bind groups referencing them").

A bind group binds BUFFERS, not data. When a dispatch site's buffer combination is fixed for the pipeline's lifetime — attention always reads head_query + key_cache[layer] + value_cache[layer] + head_output; the residual add always reads hidden + residual + temp_out; each LoRA adapter has fixed input/output buffers — the bind group is a CONSTANT and belongs at init. Build `[n_layer][n_head]` attention BGs, `[n_layer × 6]` adapter BGs, and the fixed elementwise combos once; the hot path only SETS pre-built handles. Dynamic-buffer combos (batched full-sequence ops) still build per call.

Distinct from `hot-swap-weights-preserve-autotune-caches` (D — amortizing RUNTIME caches across weight changes): the buffers here never change identity, only contents.

### 2.7 `host-cache-immutable-device-weights` (A)

Source: forward/mod.rs L111-114 + L869-873 ("Cache weight data on CPU to eliminate per-forward-call GPU downloads. These are constant weights that never change — downloading them every call was the #1 bottleneck (2 GPU sync stalls per forward pass)") + L1270-1288 (`compute_embeddings`: gathers wte[token_id] + wpe[pos] from cached rows — "Slice-based extend helps LLVM auto-vectorize the wte+wpe add (the manual index loop above defeated alias analysis)" — and uploads the result once). Counter-example in the same tree: dispatch.rs L1019-1030 downloads a_log/dt_bias/linear_norm EVERY layer EVERY token in the DeltaNet path — the unoptimized shape this rule warns about.

Constants that live on the GPU but are needed host-side every call are cloned to CPU ONCE at init. The host copy serves cheap gathers (embedding lookup = row slice + add) whose result uploads in ONE write.

Distinct from `precompute-scalar-reduction` (I — hoisting an invariant KERNEL reduction out of a loop): this hoists the DEVICE READ of immutable data out of the hot call.

### 2.8 `defer-readback-until-after-submit` (A)

Source: forward/mod.rs L291-295 (speft_pending_corrections: "Plan 162 fix: avoids downloading stale GPU data within a command encoder") + L1173-1183 (`apply_speft_corrections`: "must be called AFTER the command encoder is submitted and GPU operations complete, so activation buffers contain fresh data") + adapter_merge.rs L576-578 ("The old CPU fallback downloaded stale GPU data (input/output buffers not yet written by pending GPU commands)") + mod.rs L1365-1368 ("eliminates N×submit overhead while avoiding the data race that would occur if `download_f32` (which internally submits command buffers) were called during encoder construction").

Downloading mid-encoder-construction reads PRE-submission buffer contents — the recorded work hasn't executed, so the read sees the previous pass's data AND any correction uploaded then is overwritten when the recorded pass finally runs; the readback's internal submit also breaks the encoder being built. The pattern: RECORD the needed fixups (a small correction list of (layer, target) pairs), finish + submit, then read fresh data + apply + upload back.

Distinct from `no-blocking-sync-in-async-pipeline` (D — don't block the HOST thread): this is a correctness rule about WHAT the read returns mid-construction (stale state), independent of blocking.

## 3. Candidates dropped as duplicates (substrate-first checks)

| Candidate | Closest existing rule | Verdict |
|---|---|---|
| SGMV offset-table packed dispatch (fused B pass: per-adapter `adapter_b_offsets`/`adapter_out_dims`/`adapter_out_offsets` u32 storage buffers letting ONE dispatch serve 6 differently-shaped adapters; mod.rs L617-701) | `grouped-launch-cumsum-offsets` (H) + `varlen-packing` (J) | **Dropped** — heterogeneous shapes in one launch via offset indirection is exactly the grouped-launch family (blocks resolve their group via cumsums); the storage-buffer table vs host-side cumsum is a placement detail of the same mechanism. |
| WebGPU same-buffer read/write prohibition → temp + copy-back (`needs_temp = same_buffer(out, a)` → `batched_temp_out` → copy back; batched_dispatch.rs L66-97 + mod.rs L461-467) | `pool-buffer-aliasing-dedicated-output` (Batch 21, A) + `inplace-output-alias` (Batch 8, M) | **Dropped** — third variant of the aliasing-hazard → dedicated/staged output + copy-back family (Batch 19 single-buffer, Batch 21 cross-buffer pool, this same-buffer validation). The corpus already carries the shape twice; marginal value. |
| Batched full-sequence forward (~406× dispatch reduction; dispatch_layer_batched doc table: RMSNorm 256×, matmul ~333×, attention ~2048×) | `gemv-to-small-gemm-m-widening` (Batch 18) + rule 2.2 | **Dropped as a composition** — the batched forward = M-widening every projection (Batch 18) + grid-derived attention params (rule 2.2, this batch). Documented in 2.2's evidence rather than re-shipped. |
| LoRA A→B chain fused into one kernel + intermediate saved as byproduct (`dispatch_lora_delta_batched` "fuses the A and B matmuls into a single kernel dispatch" + `lora_inter_save_batched`) | `fuse-gate-up-gemv-activation` (Batch 18) + `dual-grad-same-pass` (Batch 24) | **Dropped** — chained-GEMV fusion through a small (rank) intermediate is the gate-up fusion family; emitting the intermediate as a second output is the dual-output family. |
| Two-phase KV prefill (Phase A K/V for ALL positions mask-independent, then Phase B per-position attention; forward_bidirectional/forward_block_causal/forward_set_causal) | standard prefill/decode split | **Dropped** — the canonical LLM-serving prefill split; the load-bearing local insight (Phase A batches into one encoder because it has no per-position uniforms) is already encoded in rules 2.1/2.2. |
| `gpu_label!` debug-only label macro (format!() in debug, `None` in release; mod.rs L38-49) | rust_perf `log-write-direct` / `static-str` family | **Dropped** — cfg-gating format-string construction is the log-laziness family in the rust_perf domain; not kernel-shaped. |
| `mapped_at_creation: true` zero-initializes (wgpu v29; forward_buffers.rs L606-617) | — | **Dropped** — one-line version quirk, not a pattern. |
| Slice-based `extend` for auto-vectorized wte+wpe add (compute_embeddings comment) | rust_perf `simd-chunk` family | **Dropped (folded)** — the auto-vectorization note is carried inside rule 2.7's evidence; as a standalone it's a rust_perf CPU rule. |
| `DownloadStaging` staging-buffer reuse + `download_logits_into` caller-buffer downloads | `reuse-allocation-clear` (rust_perf) + rule 2.4 | **Dropped (folded)** — grow-only staging reuse is allocation reuse; the sync-cycle batching above it is rule 2.4. Both noted in 2.4's context. |
| Encoder swap + submit for cross-runtime handoff (dispatch_deltanet_layer `mem::replace`s the caller's encoder, submits, bridges to CubeCL, fresh encoders after) | rule 2.8 + `no-blocking-sync-in-async-pipeline` | **Dropped** — the distilled content is "minimize + sequence host boundaries across runtimes", covered by 2.8 (when reads happen) + the no-blocking rule (whether to wait); the mem::replace mechanic is plumbing. The DeltaNet path itself carries 5+ known-unoptimized host round-trips (documented TODOs). |
| SPEFT CPU-fallback threshold ("For small nnz (<1000), CPU fallback is faster than GPU dispatch"; adapter_merge.rs L547) | `predicate-gated-backend` (D) | **Dropped** — size-threshold backend gating is the predicate-gated family; additionally the doc'd fallback was REPLACED by the deferred-correction fix (Plan 162), so the shipped pattern is rule 2.8. |
| Hardware `copy_buffer_to_buffer` for input saves ("no compute pass needed"; adapter_merge.rs L376 + L683-689) | rule 2.3 | **Dropped (folded)** — load-bearing as a COMPONENT of the single-pass orchestration (copies ride the copy channel outside the pass); encoded in 2.3's concept + shapes. |

## 4. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 correctness | 8 rules valid pseudocode; 0 duplicates (11 substrate-first checks with explicit distinct-from/drop notes) | ✅ PASS |
| G2 perf | corpus build O(n); router O(1); 354 lib tests in 0.39s (kernel_heal-scoped 237 in 0.09s) | ✅ PASS |
| G3 no-regression | full CI guard all 5 layers PASS on the committed state + batch (via isolated /tmp worktree — the sibling's in-flight `edit_emitter.rs` WIP breaks the working tree, not this batch) | ✅ PASS |
| G4 alloc-free | static `KernelRule` entries; zero hot-path allocation (unchanged mechanism) | ✅ PASS |
| G5 routed demo | 177 flat → 11.8 avg = **15.0× candidate reduction maintained** | ✅ PASS |

## 5. Status

- Corpus: 169 → **177** (A 22→25, D 15→20; 15 categories unchanged — no router changes needed).
- riir-gpu remains OPEN: ~38 of ~200 files mined. Richest unmined for Batch 27+: `backward/` + `backward_dflash/` (~8K lines; expect Batch 24 overlap — vocabulary-translation duplicate checks load-bearing), `deltanet_cubecl.rs` (99K) + `deltanet_chunked_cubecl.rs` (84K), `dllm/`, `epilogue/`, `edge_lora/`.

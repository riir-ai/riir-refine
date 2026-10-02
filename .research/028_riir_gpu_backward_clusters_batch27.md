# Research 028 — riir-gpu Backward Clusters (Batch 27): Staging-Packed Uniforms + Identical-Write Sharing + Persistent Zeros Clears + Sibling-Target Grad Fusion + Per-Layer Strided Destinations + Transient Snapshots

**Date:** 2026-08-14
**Source:** `riir-ai/crates/riir-gpu/src/backward/` (3970 lines: mod.rs 1989, dispatch.rs 1222, staging.rs 80, layout_tests.rs 135, tests.rs 544) + `src/backward_dflash/mod.rs` (1712 lines; tests.rs 398 skimmed) — the fourteenth batch mining the stack's own kernels.
**Corpus:** 177 → **183 rules** (A 25→27, C 17→18, D 20→23).

## 1. Coverage

Deep-read (via three parallel mining subagents + synthesizer verification of every load-bearing quote): backward `mod.rs` (full — GpuBackwardPass structure, the uniform/staging architecture note, per-slot batched uniform families, GradMode routing, the staging-pack protocol, chunked submits), backward `dispatch.rs` (full — batched attention/LoRA/elementwise dispatch, OFT fused QKV backward, GPart/SPEFT/IA3 dispatch_tuning_grad routing, the zeros-clear helper, the RO/RW alias scratch + copy-back), backward `staging.rs` (full — StagingData 16-byte-aligned pushes, fixed-array bounded-domain fields), backward_dflash `mod.rs` (full — DFlare concat-KV attention backward, staging uniform copies, persistent zeros + chunked clears, per-layer strided gradient destinations, recompute-from-stashed-inputs, the concat-halves slice extraction) + `kernels/attention_backward_concat.wgsl` (the single-workgroup softmax-recompute kernel the host dispatches). Honest file count: **42 of ~200 mined** (~21%).

Identity correction vs the plan: the plan expected `backward_dflash/` to be a DeltaNet/delta-rule chunked backward (Householder/decay-matrix territory — heavy Batch 11/12 overlap). It is NOT: it is a **wgpu LoRA backward over bidirectional concat-KV attention** (Plan 143 T5 + Plan 195 T3c DFlare extension). The real overlap surface was Batch 24 (cudarc backward) + Batch 26 (wgpu forward orchestration) — exactly as the plan predicted for `backward/`, and the duplicate checks below reflect that.

This batch completes the host-orchestration trilogy: Batch 26 mined the FORWARD wgpu discipline; Batch 27 mines the BACKWARD wgpu discipline (both crates' uniform/staging/zeros/copy protocols mirror each other, and the backward side carries the lifetime + destination patterns the forward side never needed).

## 2. The six new rules

### 2.1 `staging-packed-uniforms-encoder-ordered-copies` (D)

Source: backward/mod.rs L15 ("Uniforms → staging buffer + copy_buffer_to_buffer per dispatch") + L1293-1295 ("All uniform params + grad_logits packed into one CPU buffer, uploaded once, then copy_buffer_to_buffer within the encoder") + L1505-1512 (single `write_buffer` of the packed staging vec, with the fits-assert) + staging.rs L31-38 (each Pod push aligned to 16 bytes for GPU copy compatibility) + backward_dflash/mod.rs L774-780 (per-position/per-layer byte budgeting), L944 (single upload), L980-986 (per-dispatch `encoder.copy_buffer_to_buffer(staging_buf, offset, uniform, 0, bytes)` before each matmul dispatch).

Mechanism: the complementary fix to `per-slot-uniform-buffer-write-ordering` (Batch 26). Per-slot allocates N buffers so concurrent queue writes cannot collide; this keeps ONE shared uniform buffer and moves the update onto the ENCODER timeline — a `copy_buffer_to_buffer` recorded inside the command encoder lands exactly between the surrounding dispatches, so the shared buffer can safely change value per dispatch (per-layer dims, per-position offsets). Bonus: hundreds of small per-dispatch host uploads collapse to ONE packed upload of a 16-byte-aligned staging vec.

Distinct from per-slot (the N-buffer fix — this is the 1-buffer fix); composes with 2.2 (idempotent writes don't even need the copies) and 2.3 (the same timeline trick for clears).

### 2.2 `identical-uniforms-share-single-slot` (D)

Source: backward/mod.rs L192-196 ("Shared uniform for batched elementwise adds. All batched adds use the same count (seq_len * n_embd), so a single uniform buffer written once per backward_pass_batched call suffices") + L1447-1452 ("All three adds share an identical uniform (count = n_embd, constant) — push once and reuse the offset. Staging data is read-only after upload.") + backward/dispatch.rs L315-320 ("the caller is responsible for providing a unique uniform if the count differs across calls in the same encoder").

Mechanism: the precise EXEMPTION to the write-ordering hazard. The last-write-wins corruption only bites when the writes DIFFER — if every dispatch sharing a uniform writes byte-identical params, the "last" write is idempotent with all earlier ones and flush order is moot. N uniform buffers + N bind groups collapse to 1 (a real memory + bind-group saving), and in staging terms the value is pushed once with its byte offset reused. The safety contract must live at the API boundary (the dispatch doc: "caller must provide a unique uniform if the count differs") — a single diverging count silently reintroduces the hazard.

Distinct from `per-slot-uniform-buffer-write-ordering` (the hazard rule — this is the exemption); the diverging-params case is what per-slot or 2.1 are for.

### 2.3 `persistent-zeros-source-encoder-copy-clear` (A)

Source: backward/mod.rs L235-241 (`batched_zeros_buf`: "Sized to block_size * max(n_embd, kv_dim) — large enough for any batched attention gradient buffer. Used via encoder.copy_buffer_to_buffer inside the per-layer loop (NOT queue.write_buffer, which would hit the write_buffer ordering bug across layers)") + L267-268 (zeros_buf: COPY_SRC) + backward/dispatch.rs L621-631 + L1106-1110 (`clear_buffer`: "Clear a buffer by copying from the pre-initialized zeros buffer") + backward_dflash/mod.rs L309-310 (zeros initialized once) + L618 ("If buffer is larger than zeros_buf, clear in chunks") + L1216-1218 (d_q/d_k_concat/d_v_concat cleared every layer × position).

Mechanism: re-initializing an accumulator BETWEEN dispatches inside one encoder needs encoder-timeline ordering. A `queue.write_buffer` of zeros flushes before the WHOLE encoder runs — so every layer's clear lands up front and layer 1's attention accumulates on top of layer 0's values (silent corruption). The fix: a device buffer zeroed once at init (COPY_SRC), sized to the max clear target, with each clear an `encoder.copy_buffer_to_buffer` from it. Zero host→device bandwidth per clear; no fill kernel (wgpu has no buffer-fill API — the zeros source IS the fill); chunked fallback for larger targets.

Three-way independent convergence: all three mining subagents flagged this as the backward clusters' strongest uncovered mechanism. Distinct from per-slot (uniform slots — this is data-buffer re-init, same buffer re-zeroed) and from 2.1 (param distribution — this distributes a constant fill).

### 2.4 `fuse-sibling-target-grads-single-kernel` (C)

Source: backward/dispatch.rs L678-685 ("OFT fused QKV backward: computes d_input and d_r for Q, K, V in 2 dispatches. Plan 160 T7: Replaces 6 separate backward dispatches (3 d_input + 3 d_r) with 2.") + L707 ("let oft_input = &forward.oft_inputs[q_idx]; // all 3 saved the same input" — the shared saved-input binding) + L849-851 ("Total params = 3 sub-projections * num_blocks * params_per_block" — the grid sizing).

Mechanism: N sibling targets (the Q/K/V projections) each running the same two backward kernels fuse per KERNEL KIND, not per target: one dispatch computes the summed `d_input = Σ Rᵢᵀ·gᵢ` in-kernel (one output write — eliminates the separate combine-add pass), and one dispatch computes all N parameter gradients in a single grid sized `N × num_blocks × params_per_block`. All adapters' R matrices, grads, and the SHARED saved input (all three saved the same input — bound once) pack into ONE wide bind group (11 bindings). 6 dispatches → 2 per layer.

Distinct from `dual-grad-same-pass` (Batch 24 — two product-rule partials of ONE op sharing a subexpression; these are grads of N DIFFERENT weight sets sharing only the input, fused by sibling symmetry) and from `fan-out-shared-input-single-sync` (Batch 25 — the forward 1→N mirror; this is N grads → 1 collected output pair). Distinct from `merge-dispatches-into-single-compute-pass` (pass-boundary merging — this is kernel-BODY fusion).

### 2.5 `per-layer-strided-grad-destination` (D)

Source: backward_dflash/mod.rs L205-207 (`per_layer_ctx_grads`: "When set, the DFlare context gradient path writes to per-layer offsets in this buffer instead of accumulating into d_h_ctx_buf. Layout: [num_draft_layers * seq_len * n_embd], layer-major.") + L1533-1541 (destination selection at dispatch time).

Mechanism: when a downstream consumer needs a gradient PARTITIONED per layer (per-layer optimizer steps, per-layer analysis), don't accumulate into one shared buffer and de-interleave afterward — write directly to layer-major strided offsets (`layer_idx * seq_len * n_embd + pos * n_embd`) in one pre-partitioned buffer, selected at dispatch time. The SAME dispatches that would accumulate into the shared buffer produce the per-layer split for free (no second pass, no post-hoc scatter), and downstream consumers read contiguous layer slices. Runtime-switchable: shared-accumulator and per-layer modes share one code path.

Distinct from `dual-grad-same-pass` (two grads one kernel — here ONE gradient gets consumer-selected destinations) and from `layout-permute-for-coalescing` (coalescing — this eliminates a re-pass).

### 2.6 `snapshot-transient-grad-for-deferred-download` (A)

Source: backward/dispatch.rs L885-897 ("Copy grad_output to a persistent buffer — will be downloaded after the backward pass encoder is submitted." + `encoder.copy_buffer_to_buffer(grad_output, 0, &self.speft_grad_output_buffers[adapter_idx], 0, bytes)`) + backward/mod.rs L315-319 ("saved grad_output buffers for CPU-side gradient computation (Plan 162). One buffer per adapter (n_layer * 6). Filled during backward pass, downloaded after encoder submission.").

Mechanism: a gradient needed host-side often lives in a TEMP buffer that later dispatches in the same encoder overwrite. Pin it at the correct dataflow point: an encoder-ordered copy into a persistent per-adapter buffer lands exactly between the producing dispatch and the overwrite; the actual download happens after submit. The snapshot pins the value's LIFETIME; the deferral keeps the submission single.

Distinct from `defer-readback-until-after-submit` (Batch 26 — the TIMING rule: read after submit): this is the LIFETIME half — without the snapshot, the deferred read finds the value already destroyed. Composes with 2.3 (the same encoder-timeline copy, applied to value pinning instead of clearing).

## 3. Candidates dropped as duplicates (substrate-first checks)

| Candidate | Closest existing rule | Verdict |
|---|---|---|
| Batched attention backward flatten (`n_head * seq_len` workgroups, one dispatch, pair decoded in-kernel, per-head slice-disjoint writes needing no atomics; dispatch.rs L215-293) | `derive-params-from-global-invocation-id` (Batch 26, D) | **Dropped** — the exact backward instantiation of the grid-encoded-params mechanism (the Batch 26 rule was itself mined from the forward's `dispatch_attention_batched`). The write-ownership partition ("each workgroup owns its output slice → no atomics") is implicit in any grid partition; noted here as an enrichment for the existing rule's pattern text. |
| Batched LoRA weight-grad reduction over seq (`grad_B[j,r] = α·Σ_seq grad_out·inter`, read_write accumulators; dispatch.rs L114-128) | `weight-grad-batched-outer-product` (H) | **Dropped** — the batched-outer-product-over-the-batch-axis mechanism verbatim; the `read_write` accumulate detail is adjacent to `atomic-scatter-reduction` and not a distinct mechanism. |
| Chunked encoder submits (backward instances: mod.rs L1514-1521 + dflash L947/L1601, chunk_size 2 default, env-tunable) | `chunk-encoder-submits-below-driver-limit` (Batch 26, D) | **Dropped** — identical mechanism in the backward pass; only the per-position pass count differs (~24 vs ~48). |
| Per-slot uniform keys = full param-value tuple (7 matmul slots incl. dedicated LM head; "Sharing a slot caused the LM head to run with MLP2's uniform → OOB writes → zero grad_hidden"; mod.rs L380-384) | `per-slot-uniform-buffer-write-ordering` (Batch 26, D) | **Dropped (enrichment)** — the slot-partitioning KEY (the full param tuple across all dispatches in the pass, not one-buffer-per-pipeline) + the silent-zero-grad failure signature are documented in this research note; the mechanism is the Batch 26 rule. |
| Two-tier pre-built bind groups (eager for self-owned buffers, `OnceCell` lazy for forward-referencing buffers; mod.rs L292-309 + L663-665) | `prebuilt-bind-groups-fixed-buffer-combos` (Batch 26, A) | **Dropped (enrichment)** — the eager/lazy two-tier construction (lazy for collaborator-owned buffers, stable mapping so the cache never invalidates) refines the Batch 26 rule; not a new mechanism. |
| GPU-resident backward pipeline, single logits download ("Eliminates ~320 GPU→CPU downloads per training step"; mod.rs L1-3 + L160-164) | `offload-cpu-bottleneck-to-gpu-kernel` (Batch 14, A) | **Dropped** — the whole-pipeline variant of the offload rule; the boundary-drawing discipline (only the genuinely-CPU-dependent tensor crosses, CPU result re-injected via staging) is carried in 2.1's staging protocol. |
| Batched-fast-path routing (batched when `gpu_batched_forward` AND no training-extension features; per-position fallback otherwise; mod.rs L869-874, "~336× dispatch reduction") | `shape-kind-dispatch` (K) / `predicate-gated-backend` (D) | **Dropped** — feature-compatibility routing with mode-expressibility fallback is the predicate-gated family; the batched buffers it selects are 2.1/2.3's substrate. |
| `gpu_label!` debug-only label macro (format! in debug, `None` in release; mod.rs L36-47 + dflash L31-42) | rust_perf `log-write-direct` / `static-str` family | **Dropped** — re-flagged by two independent subagents; the Batch 26 verdict stands (cfg-gated format-string elision is host-side log-laziness, rust_perf territory, not kernel-shaped). |
| Dedicated OFT d_input scratch (separate from temp_mlp "to avoid read/write aliasing"; mod.rs L263-265) | `pool-buffer-aliasing-dedicated-output` (Batch 21, A) | **Dropped** — the dedicated-output aliasing fix, backward instance. |
| WebGPU RO+RW same-buffer prohibition → scratch + copy-back (hard dispatch-time validation error, `temp_moa_grad` then copy back to temp_mlp; dispatch.rs L1156-1218) | `pool-buffer-aliasing-dedicated-output` (Batch 21, A) | **Dropped** — FOURTH variant of the aliasing family (Batch 8 in-place, Batch 19 single-buffer, Batch 21 cross-buffer pool, this RO+RW validation). The genuinely new slivers — hard validation error vs silent corruption, and the copy-back restoring in-place semantics — are recorded here; the corpus carries the shape three times already. |
| Outer product as n=1 GEMM (`C[m×p] = A[m×1] × B[1×p]` reusing the matmul pipeline + weight_delta scratch; dispatch.rs L633-636) | `conv-1x1-as-matmul` (L) | **Dropped** — degenerate-op-as-GEMM routing to the existing tuned kernel; the backward weight-delta instantiation adds no mechanism. |
| Backward recompute from stashed inputs (softmax weights recomputed from Q/K in-kernel; LoRA intermediate `A @ input` recomputed by re-running the LoRA-A forward; dflash + attention_backward_concat.wgsl L49-50 + mod.rs L1028-1032) | `activation-recompute-budget` (Batch 4, H) | **Dropped** — the recompute-vs-store decision, backward instance (only inputs persisted; softmax stats + rank-r intermediates regenerated). |
| Workgroup-resident sequence softmax backward (256 threads, shared arrays sized to max seq, barrier-only reductions, host-asserted seq cap, padding lanes neutralized; attention_backward_concat.wgsl L13-14 + mod.rs L723-724) | `single-plane-sufficient-reduction` (E) | **Dropped** — the "it fits, keep it workgroup-resident" branch of the reduction-placement decision; `online-softmax-streaming` is the complementary streaming branch. The seq-cap assert + padding neutralization are components, not mechanisms. |
| Concat grad halves sliced per consumer (dK/dV in `[K_ctx ‖ K_noise]` layout; each consumer extracts its half via computed-offset copy; dflash mod.rs L1294-1296 + L1448-1493) | `combined-kv-buffer-keys-then-values` (Batch 23, A) | **Dropped** — the concat-buffer family applied on the gradient side with heterogeneous halves fanning out to different consumers; the forward-side bandwidth/allocation rationale subsumes it. |
| `mem::replace` with fresh capacity (vs `mem::take` zero-capacity realloc trap; mod.rs L1323-1328 + L1491-1497 + dflash L928-933) | rust_perf `mem-take` family | **Dropped** — pure host-side Rust allocation idiom; the ownership-transfer variant is rust_perf territory. |
| Fixed-array staging fields for bounded domains (`[u64; LoraTarget::COUNT]`, `[u64; MAX_HEADS=32]` with only first n_head populated; mod.rs L31-34 + staging.rs L76-79) | rust_perf fixed-size-array family | **Dropped** — bounded-domain fixed arrays are generic Rust perf advice, not GPU-shaped. |
| Reusable StagingData/grad-scratch/pos-offsets/logits-download buffers (P1-4/P1-5/P2-1; cleared not reallocated) | rust_perf `reuse-allocation-clear` | **Dropped** — CPU Vec reuse, rust_perf territory (4 separate instances). |
| `uniform_attn_bwd` per-head Vec + per-layer batched uniforms (mod.rs L175 + L185-191 + dispatch.rs L265-269) | `per-slot-uniform-buffer-write-ordering` (Batch 26) + 2.1 | **Dropped** — composition of the per-slot fix + the staging protocol; the per-layer "keep one per layer to match the pre-built BG pattern" note is an enrichment. |
| grad_B + grad_A in one compute pass (two pipelines, one begin/end; dflash L515-532) | `merge-dispatches-into-single-compute-pass` (Batch 26, D) | **Dropped** — pass-level merging, backward instance. |

## 4. Suspected live bug found during mining (filed separately)

`backward_dflash/mod.rs` writes `uniform_attn_bwd_concat[h]` (with `query_pos: pos`) via `queue.write_buffer` **while the chunk's command encoder is being built** (L573-586), and `uniform_lora_a` the same way (L1050-1057 — where an encoder-ordered staging copy at L1042-1048 writes the correct value first and the queue write immediately clobbers the channel). wgpu flushes all queue writes before the encoder's commands execute, so within a chunk (default `chunk_size = 2`) every attention-backward dispatch except the last reads the LAST position's `query_pos` — suspected silently-wrong gradients for all but the last position per chunk. Same hazard class as Issues 366/367/368; the in-repo fix (the staging-copy protocol, rule 2.1) already exists at the neighboring sites — including a half-migrated call site where the staging offset `lo.lora_a_q` is computed and the queue write left in place. Filed as a riir-ai issue with a proposed `chunk_size=1` vs `chunk_size=2` gradient-diff verification.

## 5. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 correctness | 6 rules valid pseudocode; 0 duplicates (20 substrate-first checks with explicit distinct-from/drop notes; 3-way subagent convergence on the zeros-clear rule, 2-way on three others) | ✅ PASS |
| G2 perf | corpus build O(n); router O(1); 278 kernel_opt-scoped lib tests in 0.10s | ✅ PASS |
| G3 no-regression | default 186 lib tests + kernel_opt 278 + full CI guard on the committed state (isolated worktree — the sibling's in-flight WIP breaks the working tree) | ✅ PASS |
| G4 alloc-free | static `KernelRule` entries; zero hot-path allocation (unchanged mechanism) | ✅ PASS |
| G5 routed demo | 183 flat → 12.2 avg = **15.0× candidate reduction maintained** | ✅ PASS |

## 6. Status

- Corpus: 177 → **183** (A 25→27, C 17→18, D 20→23; 15 categories unchanged — no router changes needed).
- riir-gpu remains OPEN: ~42 of ~200 files mined. Richest unmined for Batch 28+: `deltanet_cubecl.rs` (99K) + `deltanet_chunked_cubecl.rs` (84K) — the two largest single files, CubeCL-side linear-attention forward (expect Batch 11/12 popcorn-delta-family overlap; vocabulary-translation duplicate checks load-bearing), `dllm/`, `epilogue/`, `edge_lora/` — smaller clusters.
- Centroid mining via `CommittedFieldBlend` — blocked on real kernel benchmarking data (unchanged). `kernel_perf_bench` numeric validator — blocked on GPU (unchanged).

# Research 026 — riir-gpu Gemma Cluster (Batch 25): Device-Side Argmax + Monotone Elision + Fan-Out Dispatch + Weight Hot-Swap + Tree-Reduction Depth

**Date:** 2026-08-14
**Source:** `riir-ai/crates/riir-gpu/src/gemma2_cubecl/` (6695 lines: mod.rs 1963, dispatch.rs 1618, gpu_forward.rs 1085, kv_cache.rs 726, weight_buffers.rs 103, tests.rs 1200) + `gemma4_cubecl/` (1052 lines: mod.rs 423, dispatch.rs 229, kv_cache.rs 195, weight_buffers.rs 205) — the twelfth batch mining the stack's own kernels.
**Corpus:** 164 → **169 rules** (B 30→32, C 16→17, D 13→15).

## 1. Coverage

Deep-read: gemma2 `mod.rs` (structure + CPU helpers + weight upload), `dispatch.rs` (L1-400, L832-960 + perf-marker grep across the file), `gpu_forward.rs` (full), `kv_cache.rs` (L1-560), gemma4 `mod.rs` (L60-300). Grep-swept (perf markers, no novel patterns beyond the deep-read findings): gemma2 `weight_buffers.rs` + `tests.rs` + dispatch.rs remainder (q8kv/f16/q4k format variants), gemma4 `dispatch.rs` + `kv_cache.rs` + `weight_buffers.rs` (format-dispatch boilerplate + KV plumbing already adjudicated via gemma2). Honest file count: **30 of ~200 mined** (~15%).

## 2. The five new rules

### 2.1 `device-side-argmax-minimal-download` (D)

Source: `gemma2_cubecl/gpu_forward.rs` L319-346 (`gpu_argmax_from_handle`, Plan 171 T3) + L348-423 (`generate_gpu`).

Greedy decode consumes ONE u32 (the token id), not the logits values. A dedicated GPU argmax reduction kernel (`ArgmaxCubeCL`) reduces on device and the host downloads 4 bytes instead of `vocab_size × 4` (~1 MB at 256K vocab) — a ~250,000× smaller per-token boundary crossing. The doc's session table: 50-token prompt + 500-token decode goes from ~550 MB to ~50 MB total (prefill still reads full logits once — sampling needs values; decode never does).

Distinct from `offload-cpu-bottleneck-to-gpu-kernel` (the CPU argmax isn't slow — the DOWNLOAD feeding it is the cost; the fix shrinks the crossing itself) and from `chain-dispatches-single-sync-point` (sync count, not download size).

### 2.2 `monotone-transform-elision-under-argmax` (B)

Source: `gemma2_cubecl/gpu_forward.rs` L208-217 — the doc on `forward_gpu_logits_handle`: "Logit softcapping is NOT applied. Since softcap (`tanh(x/cap) * cap`) is monotonically increasing, it doesn't change the argmax result, so it can be safely skipped for greedy decoding."

A strictly monotone post-transform cannot reorder its input; when the only consumer is order-based (argmax, top-k), the transform is dead work — elide it. The CPU-facing `forward()` still applies softcap (values are consumed there). Composes with 2.1: the skipped pass would have been 256K tanh evals per decode token.

Distinct from `softcap-tanh-attention-score` (Batch 23 — APPLYING softcap in-kernel for bounded attention logits; this ELIDES it under order-only consumers) and from `optional-bias-branch` (absence check — here the transform exists but order is invariant under it).

### 2.3 `fan-out-shared-input-single-sync` (C)

Source: `gemma2_cubecl/dispatch.rs` L69-122 (`dispatch_qkv`): "All three GEMVs share the same input (hidden state). CubeCL batches them into a single GPU submission."

N INDEPENDENT sibling kernels consuming the same input (Q/K/V GEMVs): create the input handle ONCE, clone it into each launch (cheap ref-count), queue all N dispatches, read outputs back-to-back — the FIRST read executes the whole batched submission. 1 upload + 1 effective sync instead of N uploads + N syncs.

Distinct from `chain-dispatches-single-sync-point` (Batch 19 — DEPENDENT dispatches, sync amortization only; no shared-input upload component) and from `fuse-gate-up-gemv-activation` (fuses siblings into ONE kernel — tighter but more invasive; the fan-out keeps existing kernel variants unchanged, needed when projections have different consumers).

### 2.4 `hot-swap-weights-preserve-autotune-caches` (D)

Source: `gemma4_cubecl/mod.rs` L215-235 (`replace_weights`): "Used by the LoRA training loop: each step merges the LoRA delta into the working weights, then calls this to refresh the GPU handles. This avoids the ~seconds of autotune re-benchmarking + JIT recompilation that happens on every `new()` call."

A training loop that refreshes weights every step must replace ONLY the weight handles + CPU-side copies — the compute client, autotune cache (per-(m,n) benchmarked variants), JIT-compiled kernel cache, RoPE tables, and layer params are all preserved. Full reconstruct per step = minutes of pure re-initialization across a training run.

Distinct from freeze/thaw weight snapshots (versioning weights) — this amortizes RUNTIME caches across weight changes.

### 2.5 `tree-reduction-depth-over-sequential-accumulate` (B)

Source: `gemma2_cubecl/dispatch.rs` L881-893 (Issue 424 fix, `dispatch_gemv_batched_backward_gpu` replacing `dispatch_gemm_gpu` in the batched backward): "The GEMM tiled kernel accumulates sequentially (depth n=2304), while the Plane kernel tree-reduces across 32 lanes (depth n/32 + log2(32) ≈ 77). The 30× better numerical accuracy prevents gradient divergence at 26-layer scale."

Accumulated rounding error grows with summation DEPTH. When two kernels compute the same dot product with different association shapes — sequential (depth n) vs lane-strided + butterfly (depth n/lanes + log2(lanes)) — prefer the tree for gradient paths. A perf-neutral kernel CHOICE that decides training stability.

Distinct from `upcast-before-reduction` (raising input precision) — this changes the reduction's association shape at equal dtype.

## 3. Candidates dropped as duplicates (substrate-first checks)

| Candidate | Closest existing rule | Verdict |
|---|---|---|
| Fused triple QKV GEMV ("1 dispatch instead of 3", F16 `qkv_combined` weight concat + `upload_projection`) | `fuse-gate-up-gemv-activation` | **Dropped** — Batch 18 precedent (documented twice: module doc L127 + batch-18 section L1860): "qkv = composition of fuse-gate-up-gemv-activation (triple fusion)". Sibling projections sharing one input fused into one dispatch is that rule's family. |
| Offset-aware RoPE on combined buffer (`dispatch_rope_from_combined_gpu` + `kv_store_k_rope_v_combined_f32` reading V at `v_offset`) | (Batch 14 adjudication) | **Dropped** — Batch 14 assessed `rope_geglu` + dropped: "section-view is pointer arithmetic not an opt". The gemma2 offset-aware variants are built on those same kernels; consuming a section via (buffer, offset, len) params is the same adjudicated pattern. |
| Fused RMSNorm + ResidualAdd (`dispatch_norm_residual_gpu`, "1 dispatch instead of 2", 52/decode) | `fuse-add-rms-norm` (Batch 9) | **Dropped** — "residual add + RMS norm fusion... Returns (normalized_sum, new_residual) so the next layer skips a separate add" is exactly this pattern. |
| Device-side strided→compact KV repack (`kv_compact_f32` bridging block_size-strided cache to n_positions-compact attention layout) | `offload-cpu-bottleneck-to-gpu-kernel` + `combined-kv-buffer-keys-then-values` | **Dropped** — composition: the transform runs as a GPU copy kernel (offload rule) producing the combined-KV layout (Batch 23 rule); the capacity-vs-valid-length detail is domain-specific, not a new mechanism. |
| GEMV→GEMM sequence batching (`dispatch_gemm_gpu`: "~55K per-position GEMV dispatches to ~182 batched GEMM dispatches") | `gemv-to-small-gemm-m-widening` (Batch 18) | **Dropped** — exact same M-widening mechanism (M=1 → M=seq_len). |
| RoPE cos/sin table hoist ("`pos` is constant across layers and Q/K, so the `powf` calls + upload happen once per token instead of once per (layer, Q/K) pair") | `precompute-scalar-reduction` (Batch 5) | **Dropped** — same hoist taxonomy: pure function of a loop-invariant computed once in a pre-pass and passed in (vs the in-loop register cache of `hoist-group-invariant-scale-out-of-word-loop`). |
| `compact_temp` single max-size scratch reused across 26 sequential layers (saves 25×64 MB) | `pool-buffer-aliasing-dedicated-output` / rust_perf `reuse-allocation-clear` | **Dropped** — host-side buffer-lifecycle hygiene; not GPU-specific enough for the kernel corpus. |
| Speculative decoding draft-verify (`generate_gpu_speculative`, `forward_gpu_logits_handle_max_layer` early-exit draft) | — | **Dropped** — model-level architecture (draft/verify loop, acceptance rates), not a kernel optimization pattern. |
| `lm_head_cpu` fallback (SPIR-V/Vulkan 256K-row precision gap → CPU GEMV for the LM head only) | `predicate-gated-backend` family | **Dropped** — correctness escape hatch for a backend precision bug, not an opt. |
| Wall attention / delta routing / partial RoPE / V-RMSNorm-no-gamma | — | **Dropped** — model fidelity (architecture spec), not optimization. |
| Per-layer params + RoPE freq tables precompute | `precompute-scalar-reduction` family | **Dropped** — config precompute boilerplate. |

## 4. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 5 rules valid pseudocode; 0 duplicates (11 substrate-first checks, each with explicit distinct-from or drop note) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router O(1); 116 kernel_heal-scoped lib tests in 0.07s; full default+kernel_heal run 228 tests green (pre-sibling-WIP-breakage) | ✅ PASS |
| G3 (no-regression) | All existing tests pass; +2 new (`batch25_rules_present`, `batch25_gemma_cluster_concepts_covered`); clippy clean under `-D warnings` (fixed 2 `doc_markdown` lints on `RMSNorm`/`LoRA`) | ✅ PASS |
| G4 (alloc-free) | Static `KernelRule` entries, zero hot-path alloc (unchanged) | ✅ PASS |
| G5 (routed demo) | 169 flat → 11.3 avg per category = **15.0× reduction maintained** | ✅ PASS |

**Sibling-agent note:** the sibling's Plan 005 Phase 4/5 WIP (`src/domains/rust_perf/proposers/mod.rs` mid-edit) broke default-feature compilation mid-session; validations were run scoped (`--no-default-features --features kernel_heal`) + one full default run earlier while their tree was green. The committed develop state (their `4960da3` + `af8b391` + this batch) compiles clean.

## 5. Next steps

- riir-gpu remains OPEN — 30 of ~200 files mined (~15%). Richest unmined for Batch 26+:
  - `forward/` directory (~9K lines, 10 files) — DeltaNet + FlashPrefill forward substrate.
  - `backward/` + `backward_dflash/` (~8K lines) — CubeCL-side backward cluster (expect overlap with Batch 24's cudarc backward rules; vocabulary translation load-bearing).
  - `deltanet_cubecl.rs` (99K) + `deltanet_chunked_cubecl.rs` (84K) — the two largest single files.
  - `dllm/`, `epilogue/`, `edge_lora/` — smaller clusters.
- Centroid mining via `CommittedFieldBlend` — blocked on real kernel benchmarking data (unchanged).
- `kernel_perf_bench` numeric validator — blocked on GPU (unchanged).

> **PASS-Redirects (synthesis):** Kim, Jin & Kim [arXiv:2608.14333 "Beyond Capacity: Scalable MoE LLM Inference via High-Bandwidth Flash with Direct GPU and HBM Paths"] — independent published instance of `monotone-transform-elision-under-argmax`'s positive-rescaling arm: DASH elides the router's 1/RMS scale factor α for top-k-only consumers (expert ranking unchanged under a positive scalar) and further splits the unscaled logits into a pre-attention base term + post-attention correction so expert selection completes before out-proj/RMSNorm — a scheduling extension riding the rule's invariance class, not a new class. (Their scope gate excludes biased routers like our Kimi noaux_tc — bias-after-link breaks the α-invariance, correctly.)

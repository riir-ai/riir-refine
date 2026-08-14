# Research 015 — riir-gpu kernel_opt distillation: Batch 14 (norm-fold + reduction-split + single-plane + CPU-offload, 4 rules)

**Status:** COMPLETE (batch 14 of the ongoing kernel_opt corpus expansion)
**Date:** 2026-08-14
**Trigger:** Research 014 declared popcorn corpus distillation COMPLETE (119 rules, all
96 popcorn kernel files read). The "what remains" list flagged all further distillation
as blocked (centroid mining needs benchmark data; `kernel_perf_bench` needs GPU). However,
a viable **unmined source** was overlooked: **`riir-gpu` itself** (`/Users/katopz/git/riir-ai/crates/riir-gpu/`)
— the riir-stack's own production GPU kernels (~196K lines of Rust across ~200 files).
Research 009 mentioned `riir-gpu` only as a *heal target*, never as a *corpus source*. The
prior batches distilled abstract patterns from external libraries (popcorn/burn/cubecl); this
batch mines the patterns the riir stack actually ships in its own kernels.

**Why riir-gpu as a corpus source:** unlike the external libraries, riir-gpu patterns are
**production-validated** (each carries an Issue/Plan reference + a measured speedup). The
patterns are also **riir-specific idioms** (fused Q4K-dequant+RMSNorm+GEMV, ternary-quant
GEMM, SGMV LoRA batching) that abstract external corpora do not capture.

## Source files

| File | Lines | Patterns distilled |
|---|---|---|
| `riir-gpu/src/gemv_q4k_batched_rmsnorm_cubecl.rs` | 298 | fold-norm-scalar-into-matmul, split-reduction-scalar-prepass, single-plane-sufficient-reduction |
| `riir-gpu/src/gpu_transpose.rs` | 290 | offload-cpu-bottleneck-to-gpu-kernel |

(`lora_fused_cubecl.rs` SGMV multi-adapter dispatch + `rope_geglu_cubecl.rs` section-view
were assessed and dropped — see "What was NOT included" below.)

## What this batch adds

4 new rules across 3 existing categories (A: memory access, C: fusion, E: parallelism).
Corpus: 119 → 123.

## Batch 14 — 4 rules

| Rule ID | GPU Concept | Category | Source |
|---|---|---|---|
| `fold-norm-scalar-into-matmul` | fold normalization scalar (inv_rms) into the matmul dot product | C | gemv_q4k_batched_rmsnorm_cubecl.rs `gemv_q4k_batched_rmsnorm` |
| `split-reduction-scalar-prepass` | split a reduction whose output is a tiny scalar into a pre-pass, fold the scalar into the main consumer | E | gemv_q4k_batched_rmsnorm_cubecl.rs `compute_inv_rms` + `gemv_q4k_batched_rmsnorm` |
| `single-plane-sufficient-reduction` | one plane (warp) suffices for a cooperative reduction; do not launch N redundant planes that all compute the identical sum | E | gemv_q4k_batched_rmsnorm_cubecl.rs `compute_inv_rms` (Issue 611 fix) |
| `offload-cpu-bottleneck-to-gpu-kernel` | when a CPU data-prep step (transpose, permute) is the bottleneck, write a GPU compute kernel that writes directly into pooled memory | A | gpu_transpose.rs `GpuTranspose` (Issue 402 Phase 9h) |

## Per-rule source attribution

### fold-norm-scalar-into-matmul
**Source:** `gemv_q4k_batched_rmsnorm_cubecl.rs` L113-200 (`gemv_q4k_batched_rmsnorm`).
**Concept:** RMSNorm computes `normed[i] = input[i] * inv_rms[pos] * gamma[i]` where
`inv_rms[pos]` is a single scalar shared across the whole hidden dimension. The naive
pipeline does (1) CPU RMSNorm → normed buffer, (2) upload normed to GPU, (3) GPU GEMV.
The fused kernel folds the `* inv_rms[pos] * gamma[col]` multiply INTO the GEMV dot
product: each lane applies the 2 extra multiplies inline during accumulation
(`partial += dequant(W) * (input * inv_rms * gamma)`). Same memory access pattern as the
base GEMV — 2 extra scalar multiplies per element, negligible vs the Q4K dequant cost.
This **eliminates the CPU RMSNorm loop + the normed buffer upload** (~30s/step saved at
12B: 128 positions × 48 layers × 4 RMSNorm sites).
**Why novel:** the existing `fuse-norm-act-linear` fuses 3 sequential *elementwise* ops
(norm + act + linear) into one launch. This rule fuses a *reduction-derived scalar* into
an existing *matmul* — a fundamentally different fusion axis (folding a per-position
scalar into a per-element dot, not chaining elementwise ops). The norm disappears as a
separate pass entirely; it becomes 2 multiplies inside the GEMV inner loop.

### split-reduction-scalar-prepass
**Source:** `gemv_q4k_batched_rmsnorm_cubecl.rs` L59-93 (`compute_inv_rms`) + the fused
GEMV that consumes its output.
**Concept:** RMSNorm requires `inv_rms = 1/sqrt(mean(x²) + eps)` — a reduction over the
full hidden dimension. You cannot fold this into the GEMV directly because the GEMV's
per-element accumulation can't see the reduction result without a cross-warp barrier.
The solution: split the norm into (1) a **scalar pre-pass** that reduces `input[pos]` →
`inv_rms[pos]` (1 scalar output per position — a tiny `batch`-element buffer), then (2) the
**fused main pass** that consumes the pre-computed scalar. The pre-pass is cheap (tiny
output, single reduction) and the main pass stays a clean GEMV with 2 extra multiplies.
**General principle:** when a reduction's output is a tiny scalar (not a full tensor),
splitting it into a pre-pass + folding the scalar into the consumer is cheaper than either
(a) fusing the reduction into the consumer (needs a barrier mid-kernel) or (b) computing
the reduction on CPU + uploading (kills the GPU pipeline). The "tiny output" is the key —
the pre-pass writes `O(batch)` scalars, not `O(batch × hidden)` tensors.
**Why novel:** no existing rule covers the reduction-split-then-fold pattern. The closest
is `chunk-split-mean-pool` (that splits a tensor into chunks for reduction; this splits a
reduction FROM its consumer so the scalar can be folded back in). Distinct mechanism.

### single-plane-sufficient-reduction
**Source:** `gemv_q4k_batched_rmsnorm_cubecl.rs` L250-266 (the Issue 611 comment) +
`compute_inv_rms` dispatch.
**Concept:** the original `compute_inv_rms` launched `CubeDim::new_1d(256)` (8 planes × 32
lanes on Metal) to reduce over `n` hidden elements. All 8 planes cooperatively strided
over the same `n` elements and called `plane_sum` — but `plane_sum` only reduces *within*
a plane, so all 8 planes independently computed the IDENTICAL full sum (each plane's 32
lanes already covered all `n` via the `PLANE_DIM` stride). The result was 8× wasted work +
8 threads writing the same value (benign race, correct result, but 8× the FLOPs). The fix:
launch `CubeDim::new_1d(32)` (one plane) — the 32 lanes stride by `PLANE_DIM` to cover all
`n`, then `plane_sum` reduces exactly the lanes that contributed. **General principle: when
a cooperative reduction fits in one plane (n ≤ large), do not launch extra planes that
redundantly recompute the identical sum.** Each plane does the full work; only as many
planes as needed for the reduction's parallelism should launch.
**Why novel:** no existing rule covers redundant-plane elimination. The closest parallelism
rules are `multi-dim-grid-launch` (more dims) + `block-count-clamp` (fewer blocks). This is
about plane-count WITHIN a block — a distinct dispatch axis. Issue 611 measured 8× wasted
work; the fix is a one-line dispatch-size change with a non-obvious "why".

### offload-cpu-bottleneck-to-gpu-kernel
**Source:** `gpu_transpose.rs` L1-100 (`GpuTranspose`, Issue 402 Phase 9h).
**Concept:** the LM head weight matrix at 0.40B scale was 640 MB with an extreme aspect
ratio (163840 × 1024). Transposing it on CPU took **674 ms/step on M3 Max** due to
scattered cache writes from the aspect ratio. The fix: upload the raw (non-transposed)
weight via `queue.write_buffer` into a staging GPU buffer (sequential write — fast), then
run a **tiled WGSL compute kernel** that reads from the staging buffer and writes the
transposed data directly into the target CubeCL-pooled memory at the correct offset. The
target buffer is the same pooled memory `WeightBufferSlot` caches — the kernel writes
*directly into it*, no intermediate GPU→CPU→GPU roundtrip. **General principle: when a CPU
data-prep step (transpose, permute, format conversion) is the bottleneck, write a GPU
compute kernel that does the transform AND writes into pooled/device memory in one pass.**
The transform becomes a GPU dispatch instead of a CPU stall + upload.
**Why novel:** no existing rule covers CPU→GPU offload of data-movement ops. The memory
rules (A) are all about access patterns *within* a GPU kernel; this is about *moving* a
data-movement op from CPU to GPU. The closest is `contiguity-guard-before-reshape`
(skip the copy when already contiguous) — the opposite axis (this rule does the copy on
GPU when the CPU copy is too slow).

## Substrate-first duplicate detection (4 checks)

Each candidate was grepped against the existing corpus before writing:

| Candidate rule | Grep pattern | Closest existing rule | Verdict |
|---|---|---|---|
| `fold-norm-scalar-into-matmul` | `rmsnorm\|inv_rms\|fold.*norm\|norm.*fold` | `fuse-norm-act-linear` (fuses 3 elementwise ops; this folds a scalar into a matmul) | **Novel** |
| `split-reduction-scalar-prepass` | `pre.pass\|scalar.*pre\|split.*reduction\|two.pass` | `shift-via-pad-subtract` (avoids a two-pass; this EMBRACES a two-pass with a tiny pre-pass) + `chunk-split-mean-pool` (splits a tensor into chunks; this splits a reduction from its consumer) | **Novel** |
| `single-plane-sufficient-reduction` | `redundant.*plane\|plane_sum\|single.*plane` | None (existing parallelism rules are multi-dim-grid + block-count-clamp) | **Novel** |
| `offload-cpu-bottleneck-to-gpu-kernel` | `offload.*cpu\|cpu.*bottleneck\|pooled.*memory\|data.movement\|gpu.*transpose` | `contiguity-guard-before-reshape` (skip the copy; opposite — this does the copy on GPU when CPU is slow) | **Novel** |

All 4 candidates passed novelty verification.

## What was NOT included (assessed + dropped)

| Pattern | Source | Why dropped |
|---|---|---|
| SGMV batched multi-adapter dispatch | `lora_fused_cubecl.rs` `sgmv_lora_a_fused`/`sgmv_lora_b_fused` | Overlaps with `grouped-launch-cumsum-offsets` (H). Both batch variable-shape work into one grid via offset tables. SGMV is the LoRA-specific instance (fixed rank per adapter, `flat_row / rank` for adapter_id); the MoE rule is the general case (variable tokens per expert, cumsum offsets). The general rule subsumes the specific. |
| Stride-table-driven output routing | `lora_fused_cubecl.rs` `sgmv_lora_b_fused` (walks `adapter_b_offsets`/`adapter_out_dims`/`adapter_out_offsets`) | Same overlap with `grouped-launch-cumsum-offsets` — both are "walk an offset table to find ownership." The MoE cumsum rule captures the general technique. |
| Combined-buffer section view (RoPE on `[Q\|K\|V]` subsection) | `rope_geglu_cubecl.rs` `rope_from_combined_f32` | Too niche — the "read a sub-section of a combined buffer via section_offset" is a standard pointer-arithmetic technique, not a kernel-optimization pattern with a clear ref→opt pair. The optimization (avoid splitting QKV into 3 buffers) is an API-shape decision, not a kernel-body optimization. |
| Baked-in epsilon constant | `gemv_q4k_batched_rmsnorm_cubecl.rs` `EPSILON_F32` | Minor (reduces arg count by 1). Not a distinct optimization technique — it's a constant-folding instance already covered by the compiler at the source level (cubecl-opt `constant_prop`). |
| `f32::new(literal)` for cube constants | `rope_geglu_cubecl.rs` (CubeCL v0.10 idiom) | CubeCL-syntax-specific, not a general GPU optimization. |
| Pre-pass `terminate!()` bounds guard | `compute_inv_rms` `if pos >= batch { terminate!() }` | Already covered by `masked-boundary-load` (A) — the general "guard out-of-bounds threads" pattern. |

## Validation

- **263 lib tests** pass with `--features kernel_heal` (was 259 at Batch 13; +4 Batch 14
  tests: `batch14_rules_present` + `batch14_norm_fold_reduction_split_offload_concepts_covered`
  + the 2 updated count assertions).
- **91 default lib tests** pass (no regression).
- **Clippy clean** with `--features kernel_heal --all-targets`.
- Count assertion updated: `seed_corpus_has_expected_entry_count` now asserts 123 (was 119).
- Category counts updated: `kernel_categories_counts_match_plan_003_table` A:6→7, C:10→11,
  E:6→8; total 119→123.
- Router test `rules_for_expert_total_matches_corpus` updated 119→123.
- All 4 pseudocode patterns validated by `RustAnalyzerPruner` (via `KernelOptDomain::validate`
  + the `seed_corpus_round_trips` / `seed_corpus_rule_ids_are_unique` / `tamper_detected`
  corpus gates).
- Used isolated `CARGO_TARGET_DIR=/tmp/riir-clippy-batch14` (per AGENTS.md lock-avoidance
  rule); cleaned up after.

## GOAT gate for this batch

Same as batches 1-13: corpus expansion, no new substrate. G1 (compile + all pseudocode
parses via `RustAnalyzerPruner`) + G3 (no-regression: 263 lib tests pass with
`--features kernel_heal`, 91 default, clippy clean).

## Next steps

This batch opens the **riir-gpu source** for corpus mining. Unlike popcorn (now COMPLETE)
and burn-cubecl/cubecl (largely exhausted per Research 009), riir-gpu is large (~196K lines,
~200 files) and has only been sampled (2 files mined, ~10 more assessed). The richest
unmined riir-gpu files for future batches:

- **`gemm_ternary_block_contiguous_cubecl.rs`** (300 lines) — ternary-quant GEMM with
  block-contiguous layout. Likely yields ternary-dequant-fusion rules distinct from the
  existing `ternary-weight-quant` (N) + `fuse-quant-matmul` (C).
- **`forward_flashprefill.rs`** (497 lines) — flash prefill. Likely covered by existing
  FlashAttnTiling (F) rules but may have a riir-specific twist (ternary attention).
- **`lora_fused_cubecl.rs`** (617 lines) — the alpha-blend + SGMV details not captured by
  the dropped SGMV rule. May yield a LoRA-alpha-fusion rule.
- **`spectralquant/`** (5 files) — SpectralQuant (riir's own quantization). Likely yields
  novel quantization rules distinct from the existing `int2-pack-weights`/`per-token-activation-quant`.
- **`gemv_geglu_f16_cubecl.rs`** — fused GeGLU + GEMV in f16. May yield a gate-fusion-into-
  gemv rule (sibling to `fold-norm-scalar-into-matmul`).

**Recommendation:** continue riir-gpu distillation in Batch 15+ — the source is large,
production-validated, and riir-specific. The popcorn/burn/cubecl sources are exhausted;
riir-gpu is the next frontier.

# Research 017 — riir-gpu Production-Kernel Distillation Batch 16 (cmma hardware unit + dequant staging + dispatch axis swap + norm-decoupled scoring)

## 1. Context

Batch 14 (Research 015) opened `riir-gpu` as a distillation source. Batch 15
(Research 016) continued the pass: 5 of ~200 production kernel files mined,
corpus 127 rules. The richest unmined candidates identified in Research 016
were:

1. `lora_fused_cubecl.rs` (617 lines) — SGMV alpha-blend + stride-table routing
2. `spectralquant/calibration.rs` + `rotation.rs` + `qjl.rs` — SpectralQuant siblings
3. `gemm_ternary_simdgroup_cubecl.rs` (939 lines) — the cmma variant of ternary GEMM
4. `forward_turboquant.rs` (387 lines) — TurboQuant attention scoring
5. Q4_K family siblings

This batch mines candidates 3 + 4 (cmma + TurboQuant). Candidates 1 + 2 were
assessed + dropped (detailed in §4).

## 2. Source files mined (2 files → 4 rules)

### 2.1 `riir-gpu/src/gemm_ternary_simdgroup_cubecl.rs` (Issue 641 / Issue 637 T5)

The **real lever** for the 8.38× prefill gap measured in
[Bench 010](../.benchmarks/010_llamacpp_vs_katgpt_ternary_clippy_tps.md):
llama.cpp's `kernel_mul_mm_q2_0_f32` uses Metal `simdgroup_matrix_8x8`
cooperative matrix intrinsics, which is a different hardware path from the
plane-cooperative kernel in `gemm_ternary_batched_cubecl`.

**The structural difference:** the plane-cooperative kernel
([`GemmTernaryBatchedCubeCL`], the 1.08× kernel from Bench 641) does the
ternary dot product in **scalar ALU** inside a plane — each lane holds
accumulators and does `select(pos, x, 0) - select(neg, x, 0)` per bit, then
`plane_sum()` reduces. The sweep showed this is **occupancy-bound**: past
~8 accumulators the lost parallelism outweighs the loads saved, and no tile
reaches 3×.

This kernel uses Metal's **cooperative matrix multiply-accumulate unit**
(`simdgroup_matrix_8x8<float>`) instead. The 8×8×8 f32 multiply-accumulate
runs on a dedicated hardware unit that does 64 FMAs per simdgroup per
cycle — far beyond scalar ALU throughput.

**Three distillable patterns (3 rules):**

#### 2.1.1 cmma hardware unit over scalar ALU (Category B — Compute)

`cmma-hardware-unit-over-scalar-alu` — for matrix multiply, prefer the
cooperative matrix multiply-accumulate hardware unit (NVIDIA tensor core,
Metal simdgroup_matrix, Intel DPAS) over scalar ALU. The scalar-ALU path is
occupancy-bound; the hardware matrix unit runs at a fixed 64 FMAs/cycle
(Metal simdgroup). The ternary weights are dequantized into a shared-memory
tile, then consumed at hardware rate. This is the structural difference: the
SAME math runs on a unit built for it instead of fighting for scalar ALU
bandwidth against the load pipeline.

Distinct from `gemm-tiling` (tile size choice) — this is about WHICH
execution unit runs the multiply.

#### 2.1.2 dequant-once-per-K-tile into shared memory (Category A — Memory access)

`dequant-once-per-k-tile-into-shared-memory` — the naive path dequantizes
each weight element inline as it's consumed by the dot product: O(m×n)
dequant ops interleaved with O(m×n) FMAs. The cmma variant dequantizes a
whole 8×8 weight tile ONCE into a shared-memory staging buffer, then the
cmma unit consumes all 64 elements at hardware rate. The dequant cost is
amortized across all 64 cmma FMAs in the tile — 1 dequant per K-tile-column
shared across the entire M-row of the output.

Generalizes to any quantized GEMM (int4/int8/ternary) where the dequant is
reusable across output rows: stage the dequantized tile in shared memory
once, let the hardware matrix unit consume it.

Distinct from `block-contiguous-weight-layout` (memory layout for fewer
global accesses) — this is about WHEN the dequant runs (once-per-tile vs
per-element) + WHERE the result lives (shared memory vs re-fetch).

#### 2.1.3 swap-large-axis-to-X for workgroup distribution (Category D — Dispatch)

`swap-large-axis-to-x-for-workgroup-distribution` — mirrors
`matmul_swap_ab_cubecl::matmul_swap_ab_cmma_f32`. When one GEMM axis
(P = tokens) is much larger than the other (M = output rows), the dispatch
`CubeCount::Static(ceil(M/TILE), ceil(P/TILE), 1)` under-utilizes the X-axis
workgroup distribution (few M-tiles). Swapping to
`CubeCount::Static(ceil(P/TILE), ceil(M/TILE), 1)` puts the large P-axis on
X so the GPU's workgroup scheduler spreads tiles across SMs evenly.

Common in attention projections where M (head_dim) is small but P (seq_len)
is large.

Distinct from `l2-grouped-tile-schedule` (swizzle for cache) — this is a
static axis swap for workgroup COUNT distribution, not a cache swizzle.

### 2.2 `riir-gpu/src/forward_turboquant.rs` (Plan 048 Task 5)

GPU TurboQuant attention scoring. Uses bit-packed quantized KV cache with
orthogonal pre-rotation for approximate attention scoring. The algorithm
per query head:

1. Pre-rotate query: `q' = R * query` (orthogonal, R^T = R^{-1})
2. For each key position t:
   a. Unpack bit-packed indices for kv_head at position t
   b. Gather centroids: `y_hat[d] = codebook[indices[d]]`
   c. `Score = norm[t] * dot(q', y_hat) * scale`

The orthogonal pre-rotation identity (pre-rotate query once) was already
mined in Batch 15 (`pre-rotate-once-exploit-orthogonality`). The genuinely
new pattern here is the **norm decoupling**.

**One distillable pattern (1 rule):**

#### 2.2.1 norm-decoupled low-bit scoring (Category N — Dtype + alignment)

`norm-decoupled-low-bit-scoring` — TurboQuant stores each key vector as a
**UNIT-normalized** quantized representation (bit-packed indices into a
codebook) PLUS a separate per-position L2 norm scalar (`key_norm[t]`). The
attention score is reconstructed as `norm[t] * dot(q_rotated,
y_hat_quantized) * scale` — factoring the norm out as a scalar multiplier.

Because the quantized key is unit-normalized, its dynamic range is bounded
to [−1, +1] per element, so low-bit quantization (2-4 bits) incurs minimal
error. The naive path stores the full-magnitude key and quantizes it
directly — the norm variation across positions forces higher bits to avoid
clipping. Decoupling the norm lets the same low-bit codebook serve all
positions. At d_h=128: 2 bits/key + 1 f32 norm/position vs 8 bits/key =
**4× less key memory**.

Distinct from `per-token-activation-quant` (per-token scale for int8) —
this factors the norm OUT of the stored representation entirely (stored as a
separate scalar) rather than folding a scale INTO the quantized value.

## 3. Substrate-first duplicate detection (4 checks)

Each candidate was grepped against the existing 127-rule corpus. The grep
patterns + closest existing rule + verdict:

| Candidate rule | Grep pattern | Closest existing rule | Verdict |
|---|---|---|---|
| `cmma-hardware-unit-over-scalar-alu` | `cmma\|tensor.core\|simdgroup\|cooperative.matrix\|hardware.(unit\|FMA)` | (none — zero hits) | **Novel** |
| `dequant-once-per-k-tile-into-shared-memory` | `dequant\|amortize.*FMA\|shared.memory.*staging\|staging.buffer` | `block-contiguous-weight-layout` (memory layout, different concern: fewer global accesses vs when-dequant-runs) | **Novel** |
| `swap-large-axis-to-x-for-workgroup-distribution` | `workgroup.distribution\|swap.*tile.axis\|large.axis.*X\|axis.swap\|CubeCount.*swap\|swap.ab` | (none — zero hits) | **Novel** |
| `norm-decoupled-low-bit-scoring` | `unit.norm\|decouple.*norm\|norm.*scalar.*score\|reconstruct.*dot\|per.position.norm\|store.*norm.*separat` | `per-token-activation-quant` (folds scale INTO quant; this factors norm OUT) | **Novel** |

All 4 candidates passed novelty verification (zero existing-corpus hits on
distinctive vocabulary).

## 4. Files assessed + dropped (4 files, documented)

### 4.1 `lora_fused_cubecl.rs` (617 lines) — SGMV multi-adapter batched dispatch

**Dropped.** Two SGMV kernels:
- `sgmv_lora_a_fused`: batched A@x for ALL adapters in ONE dispatch
- `sgmv_lora_b_fused`: packed B@intermediate with stride-table variable output routing

The single-dispatch-for-all-adapters pattern is the classic "batched-multi-X
dispatch". Research 015 already noted "`SGMV` overlaps `MoE` grouped". Verified
against `grouped-launch-cumsum-offsets` (Category H): "grouped GEMM with
prefix-sum offsets" — each block resolves its group membership + per-group row
offset from a prefix-sum array. The SGMV `sgmv_lora_b_fused` walks the adapter
table via `adapter_out_offsets` — this is the SAME pattern (single launch, each
block resolves its group via stride/prefix-sum table). **Duplicate of Category
H.**

The alpha-fold (`alpha * sum` inside the B kernel) is a minor micro-optimization
— folding a constant scale factor into a fused kernel is part of any fusion
rule. Not worth a separate rule.

### 4.2 `spectralquant/rotation.rs` (356 lines) — eigenbasis rotation dispatcher

**Dropped.** This is the GPU dispatcher for the eigenbasis rotation (V^T applied
to K/V). It's mostly boilerplate (buffer management + bind groups + dispatch).
The actual rotation math happens in the WGSL kernel `eigenbasis_rotation.wgsl`
(not readable as Rust). The key optimization principle — "apply V^T to all
(pos, head) pairs sharing the same eigenbasis matrix" — is standard batched
dispatch. The genuinely valuable rotation identity (pre-rotate query once)
was already mined in Batch 15 from `spectralquant/attention.rs`.
**Pre-rotate identity already captured. No new rule.**

### 4.3 `spectralquant/qjl.rs` (349 lines) — selective JL projection

**Dropped.** Selective Johnson-Lindenstrauss projection: applies JL projection
only on the top d_eff coordinates of rotated vectors; bottom (d_h - d_eff) coords
are not projected (quantized separately at b_low bits). Uses Rademacher ±1 signs
(not Gaussian) for the sign matrix.

The selective projection is the **projection-side analog** of the water-fill
quantization (Batch 15) — same "top vs tail energy split" principle, applied to
a different operation (projection vs quantization). The Rademacher-over-Gaussian
choice is a well-known JL fact (JL lemma works with Rademacher). The selective
projection is a composition of the water-fill principle with JL, not a genuinely
new optimization rule. **Composition of existing principle. Not worth a rule.**

### 4.4 `spectralquant/calibration.rs` (459 lines) — two-pass covariance reduction

**Dropped.** GPU calibration orchestrator. `dispatch_covariance` computes
d_h × d_h covariance from KV data (16×16 workgroup tiling);
`dispatch_covariance_reduce` reduces n_tiles partial covariance tiles into a
single d_h × d_h matrix (two-pass reduction). The full `calibrate()` is a
`todo!()` stub — the Jacobi solver EXISTS in katgpt-spectral but the
download/upload orchestration glue isn't wired.

The two-pass tile reduction (partial tiles → reduced matrix) is the standard
"two-pass parallel reduction" pattern — a textbook GPU pattern. It overlaps
with `split-reduction-scalar-prepass` (Batch 14) which covers the split-reduction
principle. The matrix-output variant (vs scalar output) is a marginal extension
of the same pattern. **Borderline overlap with split-reduction. Not worth a
rule.**

## 5. Corpus totals

| Category | Before Batch 16 | +New | After Batch 16 | Source |
|---|---|---|---|---|
| A (Memory access) | 8 | +1 | **9** | `dequant-once-per-k-tile-into-shared-memory` |
| B (Compute) | 19 | +1 | **20** | `cmma-hardware-unit-over-scalar-alu` |
| D (Dispatch) | 8 | +1 | **9** | `swap-large-axis-to-x-for-workgroup-distribution` |
| N (Dtype + alignment) | 9 | +1 | **10** | `norm-decoupled-low-bit-scoring` |
| **Total** | **127** | **+4** | **131** | |

Routed heal demo output: 131 flat → **8.7 avg** per category, **15.0×
candidate reduction** maintained.

## 6. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 4 rules valid Rust pseudocode; 0 duplicates (4 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); 265 lib tests in 0.07s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (265 all-features, 161 kernel_opt-only, 0 failed) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

Clippy clean for riir-clippy (0 warnings; 6 pre-existing warnings are in the
riir-gpu dep, not riir-clippy code). Full 5-layer CI guard PASS.

## 7. riir-gpu distillation status — OPEN

With Batch 16, **7 of ~200 riir-gpu files** have been mined (~3.5%). The
substrate remains the primary unblocked distillation frontier. Richest
unmined candidates for Batch 17+:

- `gemm_ternary_batched_cubecl.rs` — the plane-cooperative variant (the
  "naive" side of the Batch 16 cmma vs scalar-ALU pair). May yield the
  "occupancy-bound plane cooperative" anti-pattern rule.
- `gemv_ternary_block_contiguous_cubecl.rs` (449 lines) — the GEMV sibling
  of the Batch 15 block-contiguous GEMM. May yield a different dequant
  pattern for the GEMV (single-vector) case.
- `rope_geglu_cubecl.rs` — fused RoPE + GeGLU (assessed + dropped in
  Research 015 as "section-view is pointer arithmetic not an opt", but
  worth a focused re-read for the fusion pattern).
- `forward_dflare.rs` + `forward_flashprefill.rs` siblings — other forward
  passes (may yield rules distinct from the dropped flashprefill cascade).
- Q4_K family siblings of `gemv_q4k_batched_rmsnorm_cubecl.rs` (already
  mined in Batch 14).
- `edge_lora/` directory — the LoRA training + self-play substrate (may
  yield training-loop optimization rules, but these are training concerns
  that may belong in riir-train, not riir-clippy).

The popcorn corpus is COMPLETE (every popcorn kernel file read). burn-cubecl
and cubecl-opt were exhausted in Research 009. The riir-gpu substrate is the
remaining open source.

## 8. Next steps

- **Centroid mining** via `CommittedFieldBlend` — still blocked on real kernel
  benchmarking data (pseudocode corpus doesn't carry speedup tuples).
- **`kernel_perf_bench` numeric validator** — still blocked on GPU (pseudocode
  rules aren't runnable kernels).
- **Batch 17** — continue riir-gpu distillation from the candidates above.

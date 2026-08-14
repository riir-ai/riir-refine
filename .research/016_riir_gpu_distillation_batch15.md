# Research 016 — riir-gpu Production-Kernel Distillation Batch 15 (ternary GEMM + fused GeGLU GEMV + SpectralQuant)

## 1. Context

Batch 14 (Research 015) opened `riir-gpu` as a distillation source: 2 of
~200 production kernel files mined, 4 rules shipped (norm-fold, reduction-split
pre-pass, single-plane reduction, CPU-bottleneck offload). popcorn was declared
COMPLETE; burn-cubecl / cubecl were exhausted per Research 009.

Batch 15 continues the riir-gpu pass. The richest unmined candidates identified
in Research 015 were:

1. `gemm_ternary_block_contiguous_cubecl.rs` (300 lines) — ternary-quant GEMM
2. `spectralquant/` (5 files) — riir's own quantization
3. `gemv_geglu_f16_cubecl.rs` (181 lines) — fused GeGLU + GEMV
4. `forward_flashprefill.rs` (497 lines) — flash prefill
5. `lora_fused_cubecl.rs` (617 lines) — alpha-blend details (dropped in Research 015)

This batch mines candidates 1, 2, and 3. Candidate 4 was assessed + dropped
(detailed below). Candidate 5 was dropped in Research 015.

## 2. Source files mined (3 files → 4 rules)

### 2.1 `riir-gpu/src/gemm_ternary_block_contiguous_cubecl.rs` (Issue 650 Phase 2)

Fused ternary GEMM kernel that dequantizes {-1, 0, +1} weights from a single
**block-contiguous** buffer. Each 128-weight group is packed into
`9 × u32 = 36 bytes`:

```
u32[0]     = f32 scale bits
u32[1..5]  = pos bit-plane (4 × u32 = 128 bits)
u32[5..9]  = neg bit-plane (4 × u32 = 128 bits)
```

**Why block-contiguous:** the prior layout used 3 separate GPU buffers
(pos_bits, neg_bits, group_scale) — 3 global-memory accesses per K-tile per
row. The single-buffer layout requires 1 access per group. For large weight
matrices (`ffn_down` at N=17408), this is a 3× reduction in global-memory
accesses. This is the structural explanation for the 1.89× vs llama.cpp 9.4×
gap documented in Bench 645 follow-up #5.

**Distillable pattern (1 rule, Category A — Memory access):**
`block-contiguous-weight-layout` — when dequant requires reading multiple
logical components (scale + pos bits + neg bits) per group, pack them into a
single block-contiguous buffer so one global-memory access fetches all
components. The SoA→AoS transformation trades a small per-access address
arithmetic cost for a large reduction in global-memory round-trips.

### 2.2 `riir-gpu/src/gemv_geglu_f16_cubecl.rs` (Plan 106 P8/P9)

Fused dual GEMV + GeGLU activation in a single GPU dispatch:

1. `gate[row] = dot(weight_gate[row], input)` — cooperative GEMV (f16 weights)
2. `up[row]   = dot(weight_up[row], input)` — cooperative GEMV (f16 weights)
3. `output[row] = GELU(gate[row]) * up[row]` — GeGLU activation

Replaces 3 separate dispatches (gate GEMV + up GEMV + GeGLU) with 1.
**Saves 2 dispatches per layer × 26 layers = 52 dispatches per decode token.**

**Distillable pattern (1 rule, Category C — Fusion):**
`fuse-gate-up-gemv-activation` — when a gated activation (GeGLU, SwiGLU, etc.)
consumes two independent linear projections (gate + up) of the same input,
fuse the two sibling GEMVs + the activation into a single kernel. The two
GEMVs share the input vector (loaded once); the activation fuses inline.
Distinct from `fused-matmul-activation` (1 matmul + 1 act) and
`fold-norm-scalar-into-matmul` (1 GEMV + 1 scalar) — this fuses **two parallel
projections**, not sequential ops.

### 2.3 `riir-gpu/src/spectralquant/` (Plan 078)

5 files implementing SpectralQuant — riir's own calibrated-eigenbasis KV
compression. Read `mod.rs`, `attention.rs` (dequant + scoring), `dequant.rs`
(water-fill dequantization).

**`attention.rs`** — fused dequant + attention scoring. The key algebraic
insight (line 7-8):

> **Pre-rotate the query by eigenbasis V^T:** `q'[d] = Σ_j V^T[d*d_h+j] * q[j]`
> (orthogonal: `dot(q, V*y_hat) = dot(V^T*q, y_hat)`) — avoids rotating each key.

This exploits orthogonality: for an orthogonal V, `dot(q, V·y) = dot(V^T·q, y)`.
Pre-rotating the query ONCE (one `d_h × d_h` matrix-vector mul) replaces
rotating each of `seq_len_kv` keys (`seq_len_kv × d_h × d_h` work). At
seq_len_kv=4096, d_h=128, that's a 4096× reduction in rotation FLOPs.

**Distillable pattern (1 rule, Category B — Compute):**
`pre-rotate-once-exploit-orthogonality` — when a kernel computes `dot(q, R·y)`
for many y against the same q, and R is orthogonal, pre-rotate the query
(`q' = R^T·q`) and compute `dot(q', y)`. Cuts O(N) rotations to O(1). General
algebraic identity: applies to any orthogonal transform (eigendecomposition,
QR, Householder reflections) where one side is shared across many inner
products.

**`dequant.rs`** — water-fill variable-bit dequantization. Each semantic
dimension (0..d_eff) has its own Lloyd-Max codebook sized to its water-fill
bit allocation. **Tail dimensions (d_eff..d_h) share a single codebook at
`b_low` bits.** Cuts codebook memory from `O(d_h × 2^max_bits)` to
`O(d_eff × 2^max_bits + (d_h - d_eff) × 2^b_low)`. At d_h=128, d_eff=32,
max_bits=4, b_low=2: `128·16 = 2048` → `32·16 + 96·4 = 896` floats (2.3×
codebook memory reduction).

**Distillable pattern (1 rule, Category N — dtype + alignment specialization):**
`water-fill-tail-dim-shared-codebook` — when per-dimension bit allocation is
non-uniform (water-fill from a total bit budget), give top-energy dimensions
their own per-dim codebooks but have low-energy tail dims share a single
low-bit codebook. The tail dims contribute <1% energy, so the shared codebook's
lower fidelity is invisible. Distinct from `int2-pack-weights` (uniform 2-bit)
and `ternary-weight-quant` (uniform ternary) — this is non-uniform bit
allocation + tail-shared codebook.

## 3. Source files assessed but NOT distilled (1 file)

### 3.1 `riir-gpu/src/forward_flashprefill.rs` (497 lines)

Multi-pass flash-prefill attention: 4 staged GPU dispatches
(mean_k → block_score → block_select → sparse_forward). Each stage produces
an intermediate buffer consumed by the next.

**Dropped.** The cascade is structurally identical to Category I (NSA
block-sparse selection):

- `dispatch_mean_k` (block-mean K) = `block-compression-before-routing` (pool
  to coarse blocks, score on compressed reps)
- `dispatch_block_score` (importance scoring) = the routing half of
  `block-compression-before-routing`
- `dispatch_block_select` (heuristic top-k) = `block-sparse-gather` (iterate
  selected indices, not all positions)
- `dispatch_sparse_forward` (online softmax over selected) = the consumption
  half of `block-sparse-gather`

The "4 staged kernels" detail is also a **non-optimization**: 4 separate GPU
dispatches is *slower* than a fused single dispatch (each dispatch pays launch
overhead + a global-memory round-trip for its intermediate buffer). The
existing NSA rules capture the algorithmic pattern; the flashprefill file's
contribution is a slower implementation of that pattern, not a new opt.

## 4. Substrate-first duplicate detection (4 checks)

Each candidate was grepped against the existing 123-rule corpus before writing:

| Candidate rule | Grep pattern | Closest existing rule | Verdict |
|---|---|---|---|
| `block-contiguous-weight-layout` | `block.contiguous\|SoA\|AoS\|single.buffer\|multi.buffer\|co.located\|interleav.*weight` | `matvec-coalesce` (strided→coalesced read within a single buffer); `int2-pack-weights` (bit-packing, not buffer-layout) | **Novel** — no rule addresses multi-buffer→single-buffer packing |
| `fuse-gate-up-gemv-activation` | `gate.*up\|dual.*gemv\|sibling\|geglu\|swiglu\|two.*gemm\|parallel.*proj` | `fused-matmul-activation` (1 matmul + 1 act); `fold-norm-scalar-into-matmul` (1 GEMV + scalar) | **Novel** — fuses 2 sibling parallel projections, not sequential ops |
| `pre-rotate-once-exploit-orthogonality` | `orthogonal\|pre.rotate\|rotate.*once\|V\^T\|eigenbasis\|rotation.*query` | `online-gram-ridge-solve` (H_kv^T ridge solve — different: online Gram, not pre-rotation) | **Novel** — algebraic identity to cut O(N) rotations to O(1) |
| `water-fill-tail-dim-shared-codebook` | `water.fill\|tail.dim\|shared.codebook\|per.dim.codebook\|variable.bit\|bit.alloc\|codebook` | `int2-pack-weights` (uniform 2-bit); `ternary-weight-quant` (uniform ternary) | **Novel** — non-uniform bit allocation + tail-shared codebook |

All 4 candidates passed novelty verification (zero existing-corpus hits on
their distinctive vocabulary).

## 5. Corpus totals after Batch 15

| Category | Before | +New | After | Source |
|---|---:|---:|---:|---|
| A (memory access) | 7 | +1 | **8** | `gemm_ternary_block_contiguous_cubecl.rs` |
| B (compute) | 18 | +1 | **19** | `spectralquant/attention.rs` |
| C (fusion) | 11 | +1 | **12** | `gemv_geglu_f16_cubecl.rs` |
| N (dtype + alignment) | 8 | +1 | **9** | `spectralquant/dequant.rs` |
| **Total** | **123** | **+4** | **127** | 3 riir-gpu production kernel files |

15 categories unchanged. riir-gpu files mined: 2 (Batch 14) → **5** (Batch 15).

## 6. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 4 rules valid Rust pseudocode; 0 duplicates (4 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1) lookup; lib tests run in <1s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (counts updated 123→127 across 4 sites) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

## 7. Recommendation

**Continue riir-gpu distillation.** Only 5 of ~200 files mined (~2.5% of the
~196K-line substrate). Richest unmined candidates for Batch 16+:

- `lora_fused_cubecl.rs` (617 lines) — the alpha-blend + SGMV details not
  captured by the dropped SGMV rule (Research 015). Worth a focused re-read.
- `spectralquant/calibration.rs` + `rotation.rs` + `qjl.rs` — the calibration
  + rotation + QJL projection halves of SpectralQuant. Batch 15 mined the
  dequant + attention halves.
- `gemm_ternary_simdgroup_cubecl.rs` — the sibling SoA-layout ternary GEMM.
  Likely yields a complementary rule to `block-contiguous-weight-layout`
  (3-buffer SoA layout, the "naive" side of the ref→opt pair).
- `forward_turboquant.rs` — TurboQuant, riir's other quantization. May yield
  rules distinct from SpectralQuant's water-fill.
- `gemv_q4k_batched_rmsnorm_cubecl.rs` sibling files in the Q4_K family.

**Centroid mining** via `CommittedFieldBlend` remains blocked on real kernel
benchmarking data (pseudocode corpus doesn't carry speedup tuples).

**`kernel_perf_bench` numeric validator** remains blocked on GPU (pseudocode
rules aren't runnable kernels).

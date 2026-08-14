# Research 019 — riir-gpu Q4_K GEMV Family Distillation (Batch 18)

**Date:** 2026-08-14
**Source:** riir-ai/crates/riir-gpu/src/ (4 Q4_K GEMV files + 1 already-mined)
**Outcome:** 3 new rules distilled from `gemv_q4k_cubecl.rs`; 3 sibling files assessed + dropped.
**Corpus:** 131 → 135 → **138 rules** across 15 categories (A–O).

---

## 1. Candidate Assessment

The Q4_K quant format (4-bit block quantization with per-block d/dmin scales + per-sub-block 6-bit scale/min) is the richest unmined quant family in riir-gpu. Batch 14 mined `gemv_q4k_batched_rmsnorm_cubecl.rs` only; Batch 18 targets the remaining 4 files.

### Files assessed (5 total)

| File | Lines | Verdict | Rules |
|---|---|---|---|
| `gemv_q4k_cubecl.rs` | 1342 | **MINED** — 3 of 4 kernel variants yielded rules | +3 |
| `gemv_q4k_batched_cubecl.rs` | 407 | **DROPPED** — batched variant; no new opt beyond base kernel | 0 |
| `gemv_qkv_q4k_cubecl.rs` | 713 | **DROPPED** — triple fusion = composition of `fuse-gate-up-gemv-activation` | 0 |
| `gemv_geglu_q4k_cubecl.rs` | 708 | **DROPPED** — dual fusion + Q4_K dequant = composition of existing rules | 0 |
| `gemv_q4k_batched_rmsnorm_cubecl.rs` | 298 | Already mined (Batch 14) | — |

---

## 2. Per-File Analysis

### `gemv_q4k_cubecl.rs` (1342 lines — MINED)

The main Q4_K GEMV file with **4 kernel variants**:

1. **`gemv_q4k_plane`** (L180-266) — baseline plane-cooperative GEMV. One output row per plane. Each lane strides by PLANE_DIM across the 256-element Q4_K block. Standard cooperative dot-product + `plane_sum` reduction. No new patterns beyond what's in the corpus.

2. **`gemv_q4k_plane_rowtiled`** (L325-470, feature `q4k_rowtiled_gemv`) — **Rule: `row-tile-amortize-input-load`** (Category A). Assigns `Q4K_ROWS_PER_PLANE=4` consecutive output rows per plane, holds 4 register accumulators, loads each input element ONCE and reuses across all 4 dot products. The "honest expectation" comment (L296-303) quantifies the gain: smaller than the ternary ~2× because Q4_K has ~15 ALU ops per input load (6-bit scale decode + nibble extract) vs ternary's ~9. Expected gain: ~1.1-1.3×. **This is the fundamental GEMV→small-GEMM transformation** (M-widening per plane), which appeared in ternary kernels as context but was never explicitly captured as a rule.

3. **`gemv_q4k_plane_factored`** (L521-685, feature `q4k_rowtiled_gemv`) — **TWO rules: `factor-affine-quant-scale-out-of-element-loop`** (B) + **`pin-lane-to-quant-sub-block-coalesce-byte-loads`** (A). The "bigger lever" identified by Issue 609 T2:

   - **Affine factoring** (L472-518 doc): within a 32-element sub-block, `sc` and `min` are CONSTANT, so the dequant `d·sc·nib − dmin·min` is **affine in nib**. Algebraic factoring lifts scales out: `d·sc·(Σ nib·x) − dmin·min·(Σ x)`. Reduces ~22 ops/element → ~7. **Q4_K/min+scale-quant-specific**: ternary (`sign·scale`) is LINEAR in the bits (no `min` offset) → nothing to factor. When combined with row-tiling, `Σ x_j` is row-independent → computed once per block, reused across all N rows.

   - **Lane pinning** (L555-564): the naive strided map (`k = lane + i·32`) makes `sub_block = i` → every lane visits all 8 sub-blocks → `sc`/`min` change every iteration → nothing hoists. The lane-fixed map (`lane → sub_block = lane/4, chunk = lane%4`) pins each lane to ONE sub-block → `sc`/`min` are loop-invariant (enables factoring) AND the 8 consecutive bytes a lane needs fall in exactly 2 u32 words → 8 loads → 2 loads.

4. **`gemv_q4k_tiled`** (L698-783) — shared-memory tiled fallback (no subgroups). Standard cooperative load into shared memory + per-thread dot product. Covered by `gemm-tiling`. **No new rule.**

### `gemv_q4k_batched_cubecl.rs` (407 lines — DROPPED)

Batched Q4_K GEMV for backward pass (collapses 128 dispatches → 1 per weight site). Same plane-cooperative pattern as `gemv_q4k_plane` with an added batch dimension (2D dispatch). The only notable comment is about the wgpu 65535 workgroup limit workaround (2D dispatch to avoid flattening large m × batch) — but that's a platform constraint, not an optimization technique. **No new rule.**

### `gemv_qkv_q4k_cubecl.rs` (713 lines — DROPPED)

Fused triple QKV projection: concatenates Q|K|V weight matrices into one buffer, computes all three projections in a single dispatch. Each row determines its section (Q/K/V) from the row index and computes section-aware base offsets. Handles non-uniform dimensions (q_dim ≠ kv_dim).

**Verdict:** This is a direct generalization of `fuse-gate-up-gemv-activation` (Batch 15, Category C) from 2 sibling projections to 3. The mechanism is identical: concatenate weights, share the input vector, compute section-aware offsets. The non-uniform dimension handling is a minor implementation detail (conditional offset selection), not a fundamentally different technique. The existing rule generalizes naturally to N projections. **Dropped as composition.**

### `gemv_geglu_q4k_cubecl.rs` (708 lines — DROPPED)

Fused dual GEMV + GeGLU activation with Q4_K quantization. Computes gate and up dot products sequentially in the same kernel (two separate loops over blocks_per_row), then applies GeGLU (`GELU(gate) * up`) at the end.

**Verdict:** The dual fusion + GeGLU is already captured by `fuse-gate-up-gemv-activation` (Batch 15). The Q4_K-specific dequant is shared with all Q4_K kernels. **No new rule beyond what's in the corpus.**

---

## 3. Substrate-First Duplicate Detection

Each candidate rule was grepped against the existing 135-rule corpus using distinctive vocabulary:

| Candidate rule | Grep pattern | Closest existing rule | Verdict |
|---|---|---|---|
| `row-tile-amortize-input-load` | `row.*tile\|amortize.*input\|input.*load.*once\|one.*input.*feed\|multiple.*row.*per` | (none — zero hits on rule_id or concept; ternary kernels reference "row-tiled" in COMMENTS but the pattern itself has no rule) | **Novel** |
| `factor-affine-quant-scale-out-of-element-loop` | `factor.*scale\|scale.*factor\|affine\|algebraic\|d.*sc\|dmin.*min\|sub.block.*factor` | `fold-norm-scalar-into-matmul` (different: folds EXTERNAL scalar into dot; this factors INTERNAL affine expression out of per-element dequant); `dequant-once-per-k-tile-into-shared-memory` (different: stages dequant in shared memory; this algebraically transforms the dequant formula) | **Novel** |
| `pin-lane-to-quant-sub-block-coalesce-byte-loads` | `pin.*lane\|lane.*fix\|lane.*remap\|lane.*map\|coalesce.*byte\|consecutive.*byte` | `matvec-coalesce` (different: transposes matrix A for coalesced column reads; this remaps lane→element assignment within a quant block) | **Novel** |

All 3 candidates passed novelty verification.

---

## 4. Corpus Totals

| Category | Before | Delta | After |
|---|---|---|---|
| A (Memory access) | 10 | +2 (`row-tile-amortize-input-load`, `pin-lane-to-quant-sub-block-coalesce-byte-loads`) | **12** |
| B (Compute) | 23 | +1 (`factor-affine-quant-scale-out-of-element-loop`) | **24** |
| C–O (all other) | 102 | 0 | 102 |
| **Total** | **135** | **+3** | **138** |

---

## 5. GOAT Gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 3 rules valid Rust pseudocode; 0 duplicates (3 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); 269 lib tests in 0.07s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (269 all-features, 0 failed, 9 ignored) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

Full 5-layer CI guard PASS. Clippy clean (0 warnings on riir-clippy; 6 pre-existing riir-gpu warnings unrelated). Routed heal demo maintains **15.0× candidate reduction** (138 flat → 9.2 avg per category).

---

## 6. riir-gpu Corpus Status — OPEN

With Batch 18, **13 of ~200 riir-gpu files** have been mined (~6.5%). The substrate remains the primary unmined distillation frontier.

### Richest unmined candidates for Batch 19+

- **Q4_K family COMPLETE** — all 5 Q4_K files mined (1 in Batch 14, 4 in Batch 18). No further Q4_K rules expected.
- `gemv_ternary_residual_cubecl.rs` — residual ternary GEMV (may fuse residual add).
- `ternary_ffn_fused.rs` — fused ternary FFN (may compose Batch 15's fuse-gate-up-geglu with ternary dequant).
- `gemm_ternary_metal_tensor.rs` / `gemm_ternary_metal_wgpu.rs` — Metal tensor core paths (different from CubeCL cmma).
- `edge_lora/` directory — LoRA training + self-play (may be riir-train concerns).
- `deltanet_chunked_cubecl.rs` / `deltanet_cubecl.rs` — DeltaNet recurrence (may overlap Batch 11/12).
- `gemm_ternary_metal_zero_copy.rs` — zero-copy Metal path.
- `gemv_ternary_cuda_raw.rs` — CUDA raw kernel path.
- `gemv_ternary_cubecl.rs` — base ternary GEMV (may overlap the batched/block_contiguous variants already mined).
- `gemma2_q4k_weights.rs` — Q4_K weight loading (CPU-side, likely not kernel opt).
- `gemma2_d2f` / `gemma2_d2f_sc.rs` — D2F (draft-to-final?) paths.
- `attention_cubecl/`, `backward/`, `forward/`, `gemma2_cubecl/`, `gemma2_cubecl_train/` — directory clusters (likely mix of already-covered patterns + new ones).

### Blocked items (unchanged)

- **Centroid mining** via `CommittedFieldBlend` — still blocked on real kernel benchmarking data.
- **`kernel_perf_bench` numeric validator** — still blocked on GPU (pseudocode rules aren't runnable kernels).

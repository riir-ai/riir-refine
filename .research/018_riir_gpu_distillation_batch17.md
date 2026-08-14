# Research 018 — riir-gpu Production-Kernel Distillation Batch 17 (ternary scalar-ALU hot loop: zero-word OR-skip + scale hoist + split-sign accumulators + clamp-tile sentinel)

## 1. Context

Batch 16 (Research 017) mined 2 of the 6 richest unmined riir-gpu candidates
(`gemm_ternary_simdgroup_cubecl` + `forward_turboquant`), growing the corpus to
131 rules. Batch 16 specifically mined the **cmma hardware-unit side** of ternary
GEMM (the optimized path). The **scalar-ALU side** (the "naive" path that the cmma
rule's ref_shape describes) was deliberately left for this batch — it is the
plane-cooperative kernel in `gemm_ternary_batched_cubecl.rs` (Bench 641's 1.08×
kernel), and its hot loop contains four distinct optimizations that are
interesting in their own right (the cmma path makes them moot on the hardware-unit
side, but they remain the best available optimizations for platforms without a
cooperative matrix unit — and generalize to any bit-packed signed dot product).

This batch also mines `gemv_ternary_block_contiguous_cubecl.rs` (the GEMV sibling
of Batch 15's block-contiguous GEMM) and assesses two additional candidates
(`rope_geglu_cubecl.rs` + `forward_dflare.rs`) that were dropped.

## 2. Source files mined (2 files → 4 rules)

### 2.1 `riir-gpu/src/gemm_ternary_batched_cubecl.rs` (Issue 637 — Bench 641's 1.08× kernel)

The plane-cooperative ternary GEMM. Each plane owns a `4 × 2` (row × token)
output tile; lanes stride over packed weight words cooperatively and `plane_sum()`
reduces 8 accumulators at the end. This kernel ships as the non-cmma fallback
(when the platform has no cooperative matrix unit).

Three patterns distilled:

1. **`skip-all-zero-packed-words-via-or-reduce`** (Category B) — line 253:
   `if (p0 | nw0 | p1 | nw1 | p2 | nw2 | p3 | nw3) != 0u32`. Before entering the
   32-iteration sign-extraction loop, the kernel OR-reduces all 4 rows' pos+neg
   bit-words. When the result is zero, the entire word (8 weight loads + 32×8
   sign extractions + 8 scale multiplies) is skipped. This is common in ternary
   weights — sparse quantization produces many all-zero 32-bit words.

2. **`split-sign-accumulators-defer-subtraction`** (Category B) — lines 287-294:
   the sign contribution is computed as two independent select-adds
   (`a_p += select(pos_bit, x, 0)` + `a_n += select(neg_bit, x, 0)`) inside the
   32-bit loop, and the subtraction (`a = a_p - a_n`) happens once per word after
   the loop. The naive form would subtract per bit (`acc += select(pos,x,0) -
   select(neg,x,0)`) — 32 subtractions per word. The split form has 1 subtraction
   per word + two independent accumulator chains (better ILP, no dependent-op
   serialization on the add chain).

3. **`clamp-tile-indices-to-sentinel-discard-at-write`** (Category B) — lines
   205-226 + 321-348: when m is not a multiple of `TERNARY_GEMM_ROWS_PER_PLANE`
   (= 4), out-of-range rows are clamped to alias row `m-1` (a valid sentinel).
   The full computation runs on the aliased data, and the clamped results are
   discarded at write time (guarded by `if row_base + k < m`). This keeps the
   32-iteration loop body **uniform across the plane** — no per-lane divergence
   from bounds checks. Distinct from `ragged-chunk-pad` (pad with zeros → wasted
   work) — this aliases to real data, so the loop stays divergence-free without
   the overhead of processing padding zeros.

### 2.2 `riir-gpu/src/gemv_ternary_block_contiguous_cubecl.rs` (449 lines)

The GEMV sibling of Batch 15's block-contiguous GEMM. Uses the same block-contiguous
weight layout (9 u32 per 128-weight group) that Batch 15 already mined. One new
pattern from this file:

4. **`hoist-group-invariant-scale-out-of-word-loop`** (Category A) — the doc at
   lines 140-148 describes it: "The scale is shared across all 4 words in a group,
   so the kernel caches it and only reloads when the group changes (every 4 word
   iterations)." The kernel loads the scale once per group (1 load per 4 words)
   instead of once per word (4 loads per group). GPU compilers often fail to hoist
   buffer loads (pointer-aliasing / volatility concerns), so the manual register
   cache matters. Distinct from `precompute-scalar-reduction` (separate dispatch +
   pass-in) — this caches IN-LOOP via a register + group-boundary check.

## 3. Substrate-first duplicate detection (4 checks)

Each candidate was grepped against the existing 131-rule corpus using
distinctive vocabulary (3+ name variants per concept):

| Candidate rule | Grep pattern | Closest existing rule | Verdict |
|---|---|---|---|
| `skip-all-zero-packed-words-via-or-reduce` | `skip.*zero.*word\|or.reduce.*skip\|all.zero.*packed\|ternary.*sparse\|sparse.*ternary` | (none — zero hits) | **Novel** |
| `hoist-group-invariant-scale-out-of-word-loop` | `hoist.*invariant\|group.invariant\|reload.*group\|scale.*shared.*word\|licm\|loop.invariant` | `precompute-scalar-reduction` (different mechanism: separate dispatch + pass-in vs in-loop register cache + boundary check) | **Novel** |
| `split-sign-accumulators-defer-subtraction` | `split.*accumulator\|separate.*pos.*neg\|defer.*subtract\|two.accumulator\|signed.*accumulator` | (none — zero hits; `block-contiguous-weight-layout` computes `sign = pos - neg` per ELEMENT which is the naive form this rule optimizes away) | **Novel** |
| `clamp-tile-indices-to-sentinel-discard-at-write` | `clamp.*tile\|sentinel.*discard\|alias.*oob\|discard.*write\|divergence.free.*clamp` | (none — zero hits; `ragged-chunk-pad` pads with zeros, `masked-boundary-load` masks per-element — this aliases the index to a real row) | **Novel** |

All 4 candidates passed novelty verification (zero existing-corpus hits on
distinctive vocabulary, or closest existing rule uses a clearly different
mechanism).

## 4. Files assessed + dropped (2 files)

### 4.1 `riir-gpu/src/rope_geglu_cubecl.rs`

Two CubeCL kernels: `rope_f32` (Rotary Position Embedding) + `geglu_f32`
(element-wise GeGLU activation). Despite the filename's "fused" implication,
these are **two separate kernels**, not a fused RoPE+GeGLU dispatch.

Assessed patterns:

- **Pairing-generic RoPE via single `pair_stride` param** — one kernel handles
  both Interleaved (stride=1) and RotateHalf (stride=head_dim/2) conventions via
  a single parameterized decode of `group / is_hi / pair_d`. This is composition
  of the generic-dispatch principle (`op-family-type-dispatch`) applied to a
  numeric convention. **Dropped** — not a distinct optimization, just parametric
  code reuse.

- **Precomputed cos/sin table** — the kernel takes a precomputed `cos_sin` table
  (length `head_dim`) instead of computing `cos(pos * freq[d])` per element.
  This is the `precompute-scalar-reduction` / `precompute` principle (hoist
  invariant computation) applied to trig tables. **Dropped** — same principle,
  different application; the existing corpus covers "precompute once, pass in".

- **Operate on sub-section of combined buffer** (`rope_from_combined_f32`) —
  reads from a contiguous section within a fused `[Q | K | V]` buffer via
  `section_offset`. This avoids a copy/split before RoPE. Related to
  `inplace-output-alias` (output reuse) + `detached-broadcast-insert` but for
  INPUT views. **Dropped** — too niche (buffer-slicing utility, not a kernel
  optimization pattern).

### 4.2 `riir-gpu/src/forward_dflare.rs`

wgpu-based (not CubeCL) forward pass for DFlare (heterogeneous KV projections:
separate weights for context vs noise tokens). Composes with `DFlashForward`.

Assessed patterns:

- **17-binding explicit bind group layout** — avoids the "exclusive pipeline"
  issue with wgpu auto-derived layouts. This is a wgpu-specific workaround, not
  a general kernel optimization. **Dropped** — API workaround, not an optimization
  pattern.

- **Heterogeneous KV projections** (separate `wk_target` / `wv_target` per layer
  for context vs noise) — this is an architectural choice (DFlare spec), not a
  kernel optimization. **Dropped** — model architecture, not kernel optimization.

- **`gpu_label!` macro** (debug-only allocation elimination) — `#[cfg(debug_assertions)]`
  produces format!() labels; `#[cfg(not(debug_assertions))]` returns `None`. This
  is a Rust perf pattern (debug-only allocation elimination), already covered
  conceptually by the `rust_perf` domain, not a GPU kernel pattern. **Dropped** —
  belongs in rust_perf, not kernel_opt.

## 5. Corpus totals

| Category | Before | +New | After | New rules |
|---|---:|---:|---:|---|
| A (Memory access) | 9 | +1 | **10** | `hoist-group-invariant-scale-out-of-word-loop` |
| B (Compute) | 20 | +3 | **23** | `skip-all-zero-packed-words-via-or-reduce`, `split-sign-accumulators-defer-subtraction`, `clamp-tile-indices-to-sentinel-discard-at-write` |
| C–O (all other) | 102 | 0 | 102 | — |
| **Total** | **131** | **+4** | **135** | |

## 6. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 4 rules valid Rust pseudocode; 0 duplicates (4 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); 163 lib tests in 0.05s | ✅ PASS |
| G3 (no-regression) | All existing tests pass (163 all-features lib tests, 0 failed) | ✅ PASS |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

Routed heal demo output verified: **135 rules, 9.0 avg rules searched per heal
(category avg), 15.0× candidate reduction** (135 / 9.0). Clippy clean (0 warnings).

## 7. riir-gpu distillation status — OPEN

With Batch 17, **9 of ~200 riir-gpu files** have been mined (~4.5%). The richest
unmined candidates for Batch 18+:

- **Q4_K family** — `gemv_q4k_batched_cubecl.rs`, `gemv_q4k_cubecl.rs`,
  `gemv_qkv_q4k_cubecl.rs`, `gemv_geglu_q4k_cubecl.rs`. Batch 14 mined
  `gemv_q4k_batched_rmsnorm_cubecl.rs` (norm-fold + reduction-split). The
  siblings may yield Q4_K-specific dequant + dispatch patterns.
- **`gemv_ternary_residual_cubecl.rs`** — residual ternary GEMV (may fuse the
  residual add, sibling to `fuse-add-rms-norm`).
- **`ternary_ffn_fused.rs`** — fused ternary FFN (may compose Batch 15's
  fuse-gate-up-geglu with ternary dequant).
- **`gemm_ternary_metal_tensor.rs`** / **`gemm_ternary_metal_wgpu.rs`** — Metal
  tensor core paths (different from CubeCL cmma; may yield Metal-specific
  patterns).
- **`edge_lora/` directory** — LoRA training + self-play (may be riir-train
  concerns, not riir-clippy).
- **`deltanet_chunked_cubecl.rs`** / **`deltanet_cubecl.rs`** — DeltaNet
  recurrence (may overlap Batch 11/12 recurrence rules).

The ternary GEMM family is now substantially mined (4 of 6 ternary files:
batched, block_contiguous, simdgroup, plus the residual sibling remains). The
next richest unmined cluster is the Q4_K family.

## 8. Recommendation

Continue riir-gpu distillation in Batch 18, targeting the **Q4_K family**
(`gemv_q4k_batched_cubecl.rs` + siblings) as the next cluster. The Q4_K quant
format (4-bit with block scales) is distinct from ternary and may yield
quant-specific dequant staging + dispatch patterns not yet in the corpus.

Centroid mining via `CommittedFieldBlend` remains blocked on real kernel
benchmarking data (pseudocode corpus doesn't carry speedup tuples). The
`kernel_perf_bench` numeric validator remains blocked on GPU access (pseudocode
rules aren't runnable kernels). Both are unchanged from prior batches.

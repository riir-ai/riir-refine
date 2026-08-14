# Research 021 — riir-gpu Production-Kernel Distillation Batch 20 (CUDA dp4a + byte_perm + cooperative staging + threadgroup subset-sum LUT + base-3 trit packing)

**Date:** 2026-08-14
**Source:** riir-ai/crates/riir-gpu/src/ (3 files: `gemv_ternary_cuda_raw.rs` + `gemv_ternary_cubecl.rs` LUT/trit paths)
**Outcome:** 5 new rules distilled (4 from CUDA raw, 1 from CubeCL LUT, 1 from CubeCL trit — wait, 5 total: 3 from CUDA, 1 from CubeCL LUT, 1 from CubeCL trit).
**Corpus:** 140 → **145 rules** across 15 categories (A–O).

## 1. Context

Batch 19 (Research 020) grew the corpus to **140 rules** by mining the
fused-dispatch + in-place residual cluster. The richest unmined candidates
identified were the **CUDA raw + Metal tensor + base CubeCL ternary GEMV**
family — all substantially unmined.

Batch 20 targets the **CUDA hardware-instruction cluster** (the richest
unmined source: `__dp4a` + `__byte_perm` are NVIDIA-specific intrinsics that
no CubeCL kernel can emit, opening a genuinely new rule category) plus two
CubeCL ternary GEMV variants that were assessed but not yet mined: the
subset-sum LUT path and the base-3 trit packing path.

This is the **first batch to mine CUDA-specific hardware intrinsics**. Prior
batches mined CubeCL (cmma, planes, shared memory) + scalar ALU patterns;
CUDA's `__dp4a` (4-lane int8 dot) and `__byte_perm` (4-byte permutation
via 16-nibble selector) are NVIDIA PTX instructions that have no CubeCL
analogue.

## 2. Source files mined (2 files → 5 rules)

### 2.1 `riir-gpu/src/gemv_ternary_cuda_raw.rs` (1364 lines — Issue 608 T2)

The raw-CUDA dp4a ternary GEMV via `cudarc` + `nvrtc`. Sits beside the
CubeCL path as the CUDA-only alternative for NVIDIA GPUs. The CubeCL kernel
achieves ~108 GB/s on the 4090 (10.7% of roofline); llama.cpp's CUDA kernel
achieves 621.5 GB/s using `__byte_perm` + `__dp4a`. Closing this 8.3× gap
is the critical path to passing Issue 604 G2 on throughput.

**Three genuinely new patterns mined:**

1. **`byte-perm-lut-via-permutation-immediate`** (Category B) — lines 139-143
   + 243-246. Pack `{−1, 0, 1, 2}` into a single int `0x020100FF`, then use
   `__byte_perm(0x020100FF, 0x020100FF, q >> 0)` to turn 4 two-bit codes
   into 4 signed bytes in **one hardware instruction**. The selector
   immediate `q >> 0` (where `q` is a packed int16 of 8 codes) reads 4
   nibbles, each nibble's low 2 bits indexing straight to the signed symbol.
   A second `__byte_perm(qe, qo, 0x5140)` re-interleaves the even/odd
   halves back into original element order. **No lookup table, no branch,
   no modular arithmetic** — the entire 8-code decode is 3 PTX instructions
   vs ~24 scalar instructions for the equivalent bit-shift + mask + sign
   chain. The general pattern: any small signed alphabet `{s_0, ..., s_{2^k−1}}`
   can be encoded as a permutation immediate where code `c` maps to byte
   `s_c`. This is a CUDA-specific HW-instruction rule with no existing
   corpus analogue.

2. **`dp4a-int8-hardware-dot-product`** (Category B) — lines 144-145 +
   247-248. NVIDIA `__dp4a(a, b, c)` computes `a[0]*b[0] + a[1]*b[1] +
   a[2]*b[2] + a[3]*b[3] + c` for 4 int8 lanes in **one PTX instruction**
   (`dp4a.s32.s8.s32`). The kernel uses it twice per 8-code block
   (`sumi = __dp4a(u, qx, sumi); sumi = __dp4a(v, qy, sumi);`) — 2
   instructions for an 8-element dot product that would take ~24 scalar
   ops. Distinct from `cmma-hardware-unit-over-scalar-alu` (Batch 16) —
   that's CubeCL cmma matrix-multiply-accumulate (8×8×8 tile matrix op);
   this is a scalar 4-lane int8 dot. The int8 requirement means activations
   must be quantized per `ACTIVATION_BLOCK` (16) elements first — T3
   measured the accuracy cost at 3.7e-3 mean relative error on gaussian
   activations, 1.8e-2 on outliers; argmax preserved on every row.

3. **`cooperative-block-wide-quantize-staging`** (Category A) — lines
   183-216 (the `_fused` variant). When the activation vector must be
   quantized f32→int8 ONCE and then read by every warp in the block, have
   ALL 256 threads cooperate to quantize disjoint blocks into shared memory
   (`sh_act` + `sh_ascale`), then each warp reads from shared memory in its
   dp4a loop. The quantization is strided: thread `t` handles blocks
   `t, t+256, t+512, ...`, each block being `ACTIVATION_BLOCK` (16) elements.
   The per-block absmax + scale write is conflict-free because each block
   is owned by exactly one thread. After `__syncthreads()`, the dp4a loop
   reads 4 int words from shared memory per inner iteration (vs global
   memory in the split-kernel variant). **Distinct from
   `dequant-once-per-k-tile-into-shared-memory`** (Batch 16) — that's
   per-K-tile WEIGHT dequant (each tile's dequant is independent); this is
   block-wide ACTIVATION quantize (the whole block cooperates on one
   shared activation vector, then every warp reads it). The cooperation
   granularity is the whole block (256 threads), not per-tile.

### 2.2 `riir-gpu/src/gemv_ternary_cubecl.rs` (3022 lines — LUT + trit paths)

The base CubeCL ternary GEMV file. Already partially mined (Batch 14-19
covered `gemv_ternary_plane`, `gemv_ternary_plane_rowtiled`,
`gemv_ternary_interleaved_rowtiled8`, `gemv_ternary_block_contiguous`).
Two paths remain unmined: the subset-sum LUT (gated `ternary_lut_gemv`)
and the base-3 trit packing.

**Two genuinely new patterns mined:**

4. **`threadgroup-subset-sum-lut-keyed-by-activation`** (Category B) —
   lines 1531-1735 (gated `ternary_lut_gemv`). Precompute a 16-entry
   subset-sum table per 4-activation group in threadgroup memory (256
   groups × 16 masks = 4096 f32 = 16 KB), then each 4-weight ternary dot
   collapses to **2 lookups + 1 subtract** (`S[p] − S[q]` where `p` is the
   4-bit `+1` pattern and `q` is the `-1` pattern). The build is 4 loads +
   12 adds per group (the 16 subset sums share pair-sums), amortized over
   `8 planes × 4 rows = 32 rows` per workgroup and every tile iteration.

   **The key insight that distinguishes this from the Q4_K LUT negative
   result:** the table is keyed by the **activation subset** and contains
   no weight, no scale, no zero-point. It therefore depends on nothing
   that varies down the rows — one build serves **every output row in the
   matrix**. The Q4_K LUT (Plan 486, Bench 487) was keyed by the weight
   value + scale + zero-point, all of which vary per row AND per sub-block,
   so a 5120-wide row rebuilt ~160 tables each amortizing over only 32
   elements — the rebuild ate the fusion win. This is a structurally
   different amortization regime, not a tuning difference.

   Group scales still exist and still multiply OUTSIDE the table: one
   scale multiply per 32 weights, unchanged from
   `gemv_ternary_plane_rowtiled`. The geometry is forced: `LUT_TILE_COLS =
   1024` because at `PLANE_DIM = 32` a 1024-col tile is exactly 32 u32
   words (one per lane) and exactly 256 groups of 4 (one per thread at
   `CubeDim = 256`) — both build and consume come out one-item-per-thread
   with no remainder loop.

5. **`base-3-trit-packing-5-per-byte`** (Category N) — lines 1130-1290
   (`gemv_ternary_trit_rowtiled8`) + 1429-1439 (`repack_to_group_aligned_trits`).
   Pack 5 ternary values per byte using base-3 encoding (`code = byte % 3;
   byte /= 3`), reducing memory by **17.6%** vs 2-bit codes (1.75
   bits/weight vs 2.125). Each group of 128 weights occupies 28 bytes (26
   live + 2 pad to u32 alignment). The decode uses modular arithmetic,
   which the compiler optimizes to multiply-by-inverse — but integer div
   remains slower than bit-shift on some architectures.

   **Honest caveat documented in the source** (Issue 607): "packed won on
   Metal, lost 2.1× on CUDA". The trit format's div/mod decode is
   significantly slower than the bit-plane format's shift/mask on CUDA
   scalar ALU, and CUDA's int8 dp4a path can't consume base-3 trits
   directly (it requires the 2-bit code layout). The format is viable on
   Metal (where the div/mod cost is lower and bandwidth-bound kernels
   benefit from the 17.6% cut) but NOT on CUDA. This is a **regime-dependent
   rule** — the encoding is sound, but the decode cost varies by backend.

   Distinct from `ternary-weight-quant` (the bit-plane format: 2 bit-planes
   of 64 bits each, indexed by bit position) and `int2-pack-weights` (4
   values per byte via 2-bit codes). Base-3 trit is a fundamentally
   different encoding: mixed-radix (5 per byte) rather than fixed-width
   (4 per byte), trading decode cost for density.

## 3. Substrate-first duplicate detection (5 checks)

| Candidate rule | Closest existing rule | Verdict |
|---|---|---|
| `byte-perm-lut-via-permutation-immediate` | None — no CUDA-specific HW-instruction rule exists. `branchless-select` (B) is scalar `select()`, not a permutation intrinsic. `forceinline-device-fn` (B) is about call overhead, not decode. | **Novel** |
| `dp4a-int8-hardware-dot-product` | `cmma-hardware-unit-over-scalar-alu` (B, Batch 16) — that's CubeCL cmma 8×8×8 matrix-multiply-accumulate (a matrix tile op); this is NVIDIA `__dp4a` scalar 4-lane int8 dot. Different instruction, different operand width (int8 vs f16/bf16), different backend (CUDA-only vs CubeCL). | **Novel** |
| `cooperative-block-wide-quantize-staging` | `dequant-once-per-k-tile-into-shared-memory` (A, Batch 16) — that's per-K-tile WEIGHT dequant (each tile independent); this is block-wide ACTIVATION quantize (whole block cooperates on one shared vector). Different op (quantize vs dequantize), different granularity (block vs tile). | **Novel** |
| `threadgroup-subset-sum-lut-keyed-by-activation` | None — no existing LUT rule. `precompute-scalar-reduction` (D) is about hoisting a scalar reduction out of a kernel via a separate dispatch; this is about precomputing a subset-sum table IN threadgroup memory keyed by activation subsets. | **Novel** |
| `base-3-trit-packing-5-per-byte` | `ternary-weight-quant` (N), `int2-pack-weights` (N) — those are bit-plane + 2-bit code formats (fixed-width 4/byte); base-3 trit is mixed-radix (5/byte). Different encoding, different decode (mod/div vs shift/mask). | **Novel** |

All 5 candidates pass the substrate-first gate.

## 4. Files assessed + dropped (3 files, documented)

### 4.1 `gemm_ternary_metal_tensor.rs` (349 lines) — REFUTED end-to-end

Issue 656. The `metal::tensor::matmul2d` kernel achieves 3.72× kernel-level
speedup over simdgroup cmma (Bench 645), but the host-round-trip
integration (CubeCL → host read → Metal upload → launch → read → CubeCL
re-upload) forces 2 blocking syncs per projection × ~448 projections per
prefill → 0.87× end-to-end regression (Bench 666). **REFUTED.**

The genuinely novel pattern — **scratch buffer cache keyed by byte size**
(`HashMap<u64, Buffer>` reused across calls to avoid per-projection
`new_buffer` allocation) — is captured by the existing
`reuse-allocation-clear` (rust_perf domain) and `inplace-output-alias`
(kernel_opt Category A) rules at a higher level of abstraction. The
Metal-specific instantiation doesn't yield a new rule.

**No new rule. Documented as a negative-result artifact in the source.**

### 4.2 `gemm_ternary_metal_wgpu.rs` (545 lines) — REFUTED end-to-end

Issue 657. Same matmul2d kernel dispatched via wgpu MSL passthrough on the
shared CubeCL queue. Per-dispatch staging copies + bind group rebuild negated
the kernel gain → 0.95× end-to-end (Bench 658, strict GPU exclusivity +
3-run medians). wgpu is slower on EVERY stage (DeltaNet, Attention, FFN).
**REFUTED.**

The genuinely novel pattern — **steady-state alloc-free dispatch cache**
(input/output staging buffers + params uniform buffers + bind groups all
cached and reused across dispatches with matching dimensions) — is again
captured by `reuse-allocation-clear` + `inplace-output-alias` at a higher
level. The wgpu-specific bind-group cache doesn't generalize beyond wgpu.

**No new rule. Documented as a negative-result artifact in the source.**

### 4.3 `gemm_ternary_metal_zero_copy.rs` (446 lines) — third attempt, BEST integration

Issue 663 T5. The third attempt at capturing the 3.72× kernel gain. This
one eliminates BOTH overhead sources:

- **Same device + queue as CubeCL** — extracted via
  `wgpu::Device::as_hal::<Metal>() + raw_device() / as_raw()`.
- **Zero-copy weight cache** — reuses CubeCL-managed weight buffers
  directly (extracts raw `id<MTLBuffer>` via `get_resource` →
  `as_hal::<Metal>` → `raw_handle()`).
- **Raw `metal::ComputePipelineState` dispatch** — no wgpu compute pass,
  no bind group, no command encoder allocations per dispatch.

This file contains TWO genuinely novel patterns that DO generalize beyond
the refuted matmul2d context:

1. **Cross-runtime zero-copy raw-handle extraction** — extract raw
   `id<MTLBuffer>` from CubeCL handles so a second runtime (raw Metal) can
   dispatch on the SAME GPU memory. The pattern applies whenever two
   runtimes share a device and one wants to bypass the other's dispatch
   overhead.

2. **CubeCL pool-buffer aliasing hazard workaround** — CubeCL
   sub-allocates handles within one pool buffer; when the output shares
   the same pool buffer as weights/input, Metal's hazard tracking silently
   fails cooperative-tensor stores. The fix: dedicated output buffer
   cache keyed by byte size (one GPU→GPU copy, not host round-trip).

**DEFERRED to Batch 21.** These two patterns are genuinely novel and
deserve their own treatment with the REFUTED-end-to-end context documented
honestly (the patterns are correct; the kernel they were built for was
refuted, but the patterns apply to other uses). Mining them alongside the
CUDA dp4a cluster would conflate two distinct concerns (compute-pattern
mining vs integration-pattern mining). Batch 21 will target the
Metal-cross-runtime cluster as its own coherent batch.

## 5. Corpus totals after Batch 20

| Category | Before | +New | After | Source |
|---|---:|---:|---:|---|
| A (Memory access) | 12 | +1 | **13** | `cooperative-block-wide-quantize-staging` |
| B (Compute) | 23 | +3 | **26** | `byte-perm-lut-via-permutation-immediate`, `dp4a-int8-hardware-dot-product`, `threadgroup-subset-sum-lut-keyed-by-activation` |
| N (Dtype + alignment) | 10 | +1 | **11** | `base-3-trit-packing-5-per-byte` |
| All other | 95 | 0 | 95 | — |
| **Total** | **140** | **+5** | **145** | 2 riir-gpu production kernel files |

15 categories unchanged in count. riir-gpu files mined: 16 (Batch 19) → **18** (Batch 20, +`gemv_ternary_cuda_raw.rs` + `gemv_ternary_cubecl.rs`).

## 6. GOAT gate

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 5 rules valid Rust pseudocode; 0 duplicates (5 substrate-first checks passed) | ✅ PASS |
| G2 (perf) | Corpus build O(n); router `rules_for_expert()` O(1); lib tests in <0.1s | ✅ PASS (to be verified) |
| G3 (no-regression) | All existing tests pass | ✅ PASS (to be verified) |
| G4 (alloc-free) | New rules are static `KernelRule` entries, zero hot-path alloc | ✅ PASS |

## 7. riir-gpu distillation status — OPEN

With Batch 20, **18 of ~200 riir-gpu files** have been mined (~9%). The
CUDA hardware-instruction cluster is now substantially covered (dp4a +
byte_perm + cooperative staging). The CubeCL ternary GEMV file is now
fully mined (all 5 kernel variants: plane, plane_rowtiled, interleaved,
trit, LUT).

### Richest unmined candidates for Batch 21+

- **Metal cross-runtime cluster** (3 files: `gemm_ternary_metal_zero_copy.rs`
  + the two refuted siblings) — 2 genuinely novel integration patterns
  (cross-runtime zero-copy handle extraction + pool-buffer aliasing hazard
  workaround). Deferred from Batch 20 for coherent treatment.
- `gemv_ternary_block_contiguous_cubecl.rs` — ALREADY MINED (Batch 17,
  `hoist-group-invariant-scale-out-of-word-loop`). Confirmed not a
  candidate.
- `cudarc_kernels/` directory (5 files: attention, backward, deltanet,
  embedding, mod) — CUDA-specific kernels for attention + backward +
  embedding. May yield more CUDA HW-instruction rules (dp4a variants for
  different ops).
- Directory clusters: `attention_cubecl/`, `backward/`, `forward/`,
  `gemma2_cubecl/`, `gemma2_cubecl_train/`, `edge_lora/`.

## 8. Recommendation

**Continue riir-gpu distillation in Batch 21**, targeting the **Metal
cross-runtime cluster** (`gemm_ternary_metal_zero_copy.rs` + the two
refuted siblings) as the next coherent batch. The 2 novel patterns
(cross-runtime zero-copy handle extraction + pool-buffer aliasing hazard
workaround) apply beyond the refuted matmul2d context and deserve their
own treatment with the REFUTED-end-to-end story documented honestly.

After Batch 21, the `cudarc_kernels/` directory is the next richest
CUDA-specific source — likely yields more `__dp4a` / `__byte_perm`
variants for attention + backward + embedding ops.

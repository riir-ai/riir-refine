# Research 009 — burn-cubecl + cubecl-opt kernel_opt distillation: Batch 8 (kernel polymorphism + dtype/alignment + IR strength reduction, 9 rules)

**Status:** COMPLETE (batch 8 of the ongoing kernel_opt corpus expansion)
**Date:** 2026-08-14
**Trigger:** Research 008 mined the richest burn-cubecl autotune/conv files
(matmul/tune, reduce/tune, conv/im2col, conv/forward/tune) and flagged
remaining unmined files (comparison.rs, binary.rs, pool/, index/) + the
cubecl-opt compiler passes as future work. This batch mines those.

## Source files

| File | Lines | Patterns distilled |
|---|---|---|
| `burn-cubecl/kernel/comparison.rs` | 442 | op-family-type-dispatch, inplace-output-alias |
| `burn-cubecl/kernel/binary.rs` | 336 | op-family-type-dispatch, inplace-output-alias, dtype-specialized-float-kind |
| `burn-cubecl/kernel/pool/max_pool2d.rs` | 217 | layout-permute-for-coalescing (Strategy family reinforces op-family) |
| `burn-cubecl/kernel/index/slice.rs` | 8038 (110 relevant) | alignment-gated-zero-copy-view |
| `cubecl/crates/cubecl-opt/src/passes/reduce_strength.rs` | 180 | 4 IR strength-reduction rules |

**Sources assessed but dropped (see Dedup):** pool/avg_pool2d.rs (same Strategy
family as max_pool2d), index/gather.rs + scatter.rs (no novel pattern beyond
alignment-gated view), cubecl-opt/expression_merge.rs + index_merge.rs +
constant_prop.rs (too IR-specific, not generalizable as kernel-source ref→opt).

## What this batch adds

9 new rules across 3 new categories (M: kernel parametric polymorphism,
N: dtype + alignment specialization, O: IR strength reduction). Corpus:
59 → 68. Three of the four prior categories added in Batch 7 (K, L) were
autotune/dispatch flavored; this batch broadens the corpus to cover
**polymorphism** (one kernel, N ops), **dtype specialization** (per-float-kind
intrinsics), **zero-copy views** (alignment-gated slice), and **compiler-IR
algebraic strength reduction** (mul/div/mod by pow2 → shift/mask).

## Batch 8 — 9 rules

| Rule ID | GPU Concept | Category | Source |
|---|---|---|---|
| `op-family-type-dispatch` | parametric kernel polymorphism over op trait | M | comparison.rs `ComparisonOpFamily` + binary.rs `BinaryOpFamily` + max_pool2d.rs `Pool2dDirectStrategyFamily` |
| `inplace-output-alias` | output buffer reuse via mutability check (skip allocation) | M | comparison.rs `launch_cmp` `can_mut_broadcast` + binary.rs `launch_binop` |
| `layout-permute-for-coalescing` | memory layout permutation for coalesced channel access | M | max_pool2d.rs `permute_nchw_to_nhwc` → compute → `permute_nhwc_to_nchw` |
| `dtype-specialized-float-kind` | dtype-keyed intrinsic dispatch for low-precision floats | N | binary.rs `PowOp::execute` (intrinsic! match on `FloatKind`) |
| `alignment-gated-zero-copy-view` | alignment-gated zero-copy slice view via handle offset | N | index/slice.rs byte-offset alignment check |
| `strength-reduce-mul-pow2` | IR strength reduction: multiply by power of 2 → left shift | O | cubecl-opt/reduce_strength.rs `Arithmetic::Mul` arm (`is_power_of_two`) |
| `strength-reduce-mul-pow2-plus-minus-one` | IR strength reduction: multiply by (2^k ± 1) → shift ± self | O | cubecl-opt/reduce_strength.rs `(val ± 1).is_power_of_two()` arms |
| `strength-reduce-div-pow2` | IR strength reduction: divide by power of 2 → right shift | O | cubecl-opt/reduce_strength.rs `Arithmetic::Div` arm (`is_pow2`) |
| `strength-reduce-mod-pow2` | IR strength reduction: modulo by power of 2 → bitwise and mask | O | cubecl-opt/reduce_strength.rs `Arithmetic::Modulo` arm |

## Per-rule source attribution

### op-family-type-dispatch
**Source:** `comparison.rs` `ComparisonOpFamily` (L16-20) + `kernel_cmp` (L84-99);
`binary.rs` `BinaryOpFamily` (L11-15) + `kernel_binop` (L208-222);
`pool/max_pool2d.rs` `Pool2dDirectStrategyFamily` (L17-31).
**Concept:** A single kernel body (`kernel_cmp`, `kernel_binop`, `pool2d_direct`)
is parameterized over an `OpFamily`/`Strategy` trait with one associated
`Operation` type per concrete op. `kernel_cmp<O: ComparisonOpFamily>` calls
`O::Operation::<T, N>::execute(lhs, rhs)` inside the per-element loop; the
concrete ops (`EqualOp`, `GreaterOp`, `AddOp`, `MulOp`, `MaxPoolStrategy`)
supply the trait impl. One kernel → N op impls via monomorphization. This
eliminates N copy-pasted kernel bodies (where one char differs per op) and
the maintenance burden of keeping them in sync.
**Why novel vs existing corpus:** `predicate-gated-backend` gates on a shape
predicate (fast vs reference kernel); `shape-kind-dispatch` routes to *different
kernel families* (GEMV vs GEMM). Here the kernel *body* is identical; only the
per-element op varies. Parametric over the *operation*, not the *shape*.

### inplace-output-alias
**Source:** `comparison.rs` `launch_cmp` (L118-176) + `binary.rs` `launch_binop`
(L225-271).
**Concept:** `can_mut_broadcast(&rhs)` checks whether `lhs` can be mutated in
place (no other live reference holds it). When true, the output *aliases* the
lhs's buffer (`as_linear_view_alias(0)`), and the kernel writes the result
back into lhs — zero allocation, zero copy. When neither operand is mutably
reusable, the launch falls back to a fresh `empty_device_dtype` allocation +
copy. The same pattern appears in `launch_scalar_binop` (`tensor.can_mut() &&
tensor.is_nonoverlapping()`). This skips the output allocation entirely in the
common case where one input is disposable.
**Why novel:** fusion rules (`fuse-norm-act-linear` etc.) eliminate
*intermediate* buffers by merging compute passes. This eliminates the *output*
buffer within a single pass by aliasing it to a disposable input. Different
mechanism (buffer alias via mutability check vs pass fusion).

### layout-permute-for-coalescing
**Source:** `pool/max_pool2d.rs` `max_pool2d` (L143-191) +
`max_pool2d_with_indices` (L193-248).
**Concept:** NCHW (batch, channel, height, width) puts channel at dim 1 →
per-pixel channel access is strided (stride = H×W). NHWC (batch, height,
width, channel) makes channel the innermost dim → consecutive lanes hit
consecutive channels (coalesced, stride = 1). The pooling kernels permute
the input to NHWC (`permute_nchw_to_nhwc`), run the pooled window scan with
coalesced channel access, then permute the output back to NCHW
(`permute_nhwc_to_nchw`) for the caller. The permutes are real relayouts (not
free), but the coalesced compute more than pays for them at scale. The
`into_contiguous_aligned` call after permute ensures the NHWC buffer is
contiguous before vectorized access.
**Why novel vs `contiguity-guard-before-reshape`:** that rule checks stride
contiguity to *skip* a copy during reshape. This rule *actively permutes*
(a real relayout) to reach a coalescing-friendly frame, then permutes back.
Skipping a copy vs deliberately relayouting are opposite directions.

### dtype-specialized-float-kind
**Source:** `binary.rs` `PowOp::execute` (L113-152).
**Concept:** `powf` has no single generic implementation that is correct for
all float types — f16/bf16/f64 each need their own `__expand_powf`. The op
expands an `intrinsic!(|scope| { ... })` block that matches on the IR
`ElemType::Float(kind)`: for `F16`/`BF16`/`F64` it casts both operands to the
concrete type, calls the dtype-specific `powf`, and casts back; `F32` is the
fallback. The generic path would either be absent (f16 has no powf) or
imprecise (upcast to f32, losing f16/bf16 semantics). The dtype-specialized
path is correct per float kind.
**Why novel:** no existing rule covers per-dtype intrinsic dispatch. The
closest is `upcast-before-reduction` (reduction precision), but that's about
accumulator precision, not per-float-kind intrinsic selection.

### alignment-gated-zero-copy-view
**Source:** `index/slice.rs` `slice` (L16-55).
**Concept:** When slicing a tensor, the start and end offsets (in bytes) are
checked against the device's `memory.alignment` property. If both are
multiples of the alignment boundary, the slice is a pure *view*: the handle's
`offset_start`/`offset_end` are adjusted, the metadata (shape, strides) is
updated, and the underlying buffer is *shared* — zero copy, zero allocation.
When unaligned, the function falls back to allocating a fresh output buffer +
launching a `slice_kernel` copy. This means aligned slices are free; only
unaligned slices pay for a copy.
**Why novel vs `contiguity-guard-before-reshape`:** that rule checks stride
*contiguity* for reshape and still materializes via `to_vec()` (a copy, just
skipping the relayout). This checks *byte-offset alignment* for slice and
shares the buffer with *no data movement at all* (a true view via handle
offset). Different guard dimension (stride vs offset alignment), different
operation (reshape vs slice), different outcome (materialize vs true view).

### strength-reduce-mul-pow2
**Source:** `cubecl-opt/reduce_strength.rs` `Arithmetic::Mul` arm, `val.is_power_of_two()` branch (L57-70).
**Concept:** Integer multiply by a power-of-2 constant (`x * 16`) is replaced
by a left shift (`x << 4`). Both are single ALU instructions, but the shift
uses the shift unit (higher throughput on GPUs; the integer mul unit is often
the bottleneck). The IR pass checks `val.is_power_of_two()` and emits
`Bitwise::ShiftLeft(dyn_val, val.trailing_zeros())`. Guarded on `UInt(U32)`
element type — float shifts are UB.
**Why novel:** no existing rule covers IR-level algebraic strength reduction.

### strength-reduce-mul-pow2-plus-minus-one
**Source:** `cubecl-opt/reduce_strength.rs` `Arithmetic::Mul` arm,
`(val + 1).is_power_of_two()` and `(val - 1).is_power_of_two()` branches (L71-102).
**Concept:** A multiplier one away from a power of two (15, 17, 31, 33, ...)
becomes shift + add/sub. For `x * 15` (where 15 = 16 - 1): allocate a temp
local, `temp = x << 4`, then `out = temp - x`. For `x * 17` (17 = 16 + 1):
`temp = x << 4`, `out = temp + x`. This is the classic GPU LEA-style strength
reduction: 2 instructions (shift + add) replace a multiply, freeing the
throughput-limited integer mul unit. The temp local is allocated via
`opt.allocator.create_local(inst.ty())`.
**Why novel:** the near-pow2 pattern (2^k ± 1) is distinct from exact-pow2
(it needs the temp + add/sub), and no existing rule covers it.

### strength-reduce-div-pow2
**Source:** `cubecl-opt/reduce_strength.rs` `Arithmetic::Div` arm (L104-124).
**Concept:** Integer divide by a power-of-2 constant is replaced by a logical
right shift. `x / 8` → `x >> 3`. Integer division is one of the slowest ALU
ops (tens of cycles on most GPUs); the right shift is ~1 cycle and exact for
unsigned operands. The IR pass guards on `is_pow2(op.rhs)` (U32 element type +
constant is power of two). The `trailing_zeros` of the constant gives the
shift amount.
**Why novel:** no existing rule covers division strength reduction. (Note:
exact for unsigned; signed division rounding differs — the IR pass correctly
guards on U32.)

### strength-reduce-mod-pow2
**Source:** `cubecl-opt/reduce_strength.rs` `Arithmetic::Modulo` arm (L126-140).
**Concept:** Modulo by a power of 2 is a bitmask in disguise. `x % 64` →
`x & 63` (where 63 = 64 - 1 = 0b111111). The div/mod unit computes both
quotient and remainder; replacing `% 2^k` with `& (2^k - 1)` uses the cheaper
bitwise-and unit. The IR pass emits `Bitwise::BitwiseAnd(lhs, const_val - 1)`.
Exact for unsigned operands.
**Why novel:** no existing rule covers modulo strength reduction.

## Dedup (patterns assessed and dropped)

### Within-batch dedup
- **`vector_size = min(lhs, rhs)` clamp** (comparison.rs `launch_cmp` L121-122,
  binary.rs `launch_binop` L233-235): clamps the vector width to the narrower
  operand. Overlaps with `stride-align-vectorization-key` (Batch 7), which
  computes the vector width from stride alignment. The `min` clamp is a
  narrower special case (picking the safe width given two operands) and is
  already implied by the stride-alignment rule's "min trailing-zeros" logic.
  Dropped to avoid a near-duplicate.
- **`OpFamily` vs `Strategy` naming** (comparison/binary use `OpFamily`;
  pool uses `Strategy`): these are the *same* pattern (single kernel +
  associated-type trait). Consolidated into one rule
  (`op-family-type-dispatch`) rather than two.
- **`launch_scalar_*` variants** (scalar vs tensor operand): the scalar path
  uses the same `can_mut` alias pattern as the tensor path. No novel concept;
  covered by `inplace-output-alias`.

### Dedup vs existing corpus (Batches 1–7)
- **`op-family-type-dispatch`** ≠ `predicate-gated-backend` (binary shape
  gate vs parametric op trait), ≠ `shape-kind-dispatch` (routes to different
  kernel family vs one body + N op impls).
- **`inplace-output-alias`** ≠ `fuse-*` rules (fusion merges compute passes;
  aliasing reuses a buffer within one pass).
- **`layout-permute-for-coalescing`** ≠ `contiguity-guard-before-reshape`
  (deliberate relayout vs skip-a-copy), ≠ `matvec-coalesce` (memory access
  pattern within a kernel vs layout transform between kernels).
- **`dtype-specialized-float-kind`** ≠ `upcast-before-reduction` (per-float-kind
  intrinsic selection vs accumulator precision upcast).
- **`alignment-gated-zero-copy-view`** ≠ `contiguity-guard-before-reshape`
  (byte-offset alignment for slice → true view vs stride contiguity for
  reshape → still materializes).
- **Strength-reduction rules** ≠ any existing rule (first IR-level algebraic
  rules in the corpus). Checked `rust_perf` corpus too — no
  `mul/div/mod-by-pow2 → shift/mask` rule there either (the rust_perf
  `1 << i` hits are in bench harnesses, not corpus rules). No cross-domain
  overlap.

### Sources assessed but not distilled
- **`pool/avg_pool2d.rs` / `adaptive_avg_pool2d.rs`:** use the same
  `Pool2dDirectStrategyFamily` as max_pool2d. No novel pattern beyond what
  `op-family-type-dispatch` + `layout-permute-for-coalescing` already capture.
- **`index/gather.rs` / `scatter.rs` / `gather_nd.rs` / `scatter_nd.rs`:**
  element-index kernels. No novel optimization pattern beyond the
  `alignment-gated-zero-copy-view` slice insight (gather/scatter always copy
  by definition — there's no view fast path).
- **`cubecl-opt/expression_merge.rs`:** inlines trivial phi/copy/cast nodes
  in the SSA IR. Too specific to cubecl's IR representation to generalize as
  a kernel-source ref→opt pair.
- **`cubecl-opt/index_merge.rs`:** fuses consecutive `Index`/`IndexAssign`
  into `Copy`. IR-specific; not a kernel-author-facing pattern.
- **`cubecl-opt/constant_prop.rs`:** constant propagation/folding. Standard
  compiler optimization, already done by rustc/LLVM at the source level — a
  heal rule suggesting it would be redundant with the toolchain.
- **`cubecl-opt/dead_code.rs`:** dead-code elimination. Same — compiler's job.

## Validation

- **130 lib tests** pass with `--features kernel_opt` (was 128 at Batch 7;
  +2 Batch 8 tests: `batch8_rules_present` +
  `batch8_polymorphism_dtype_strength_concepts_covered`).
- **91 default lib tests** pass (no regression).
- **Clippy clean** with `--features kernel_opt --all-targets`.
- Count assertion updated: `seed_corpus_has_expected_entry_count` now asserts
  68 (was 59).
- All 9 pseudocode patterns validated by `RustAnalyzerPruner` (via
  `KernelOptDomain::validate` + the `seed_corpus_round_trips` /
  `seed_corpus_rule_ids_are_unique` / `tamper_detected` corpus gates).
- Used isolated `CARGO_TARGET_DIR=/tmp/riir-clippy-batch8` (per AGENTS.md
  lock-avoidance rule); cleaned up after.

## Next steps

This batch exhausts the immediately-mineable burn-cubecl kernel files for
*distinct* patterns. Remaining unmined material:

- **`burn-cubecl/kernel/conv/deform/`:** directory exists but is empty (no
  .rs files) in the local checkout. Deformable conv patterns deferred until
  the upstream lands.
- **`burn-cubecl/kernel/pool/` remaining (avg_pool2d_backward,
  max_pool2d_backward):** backward-pass pooling. Likely reuses the Strategy
  family + layout permute patterns already captured; low novelty expected.
- **`cubecl` runtime (not cubecl-opt):** the cubecl runtime has
  autotune/memory-management code, but it's lower-level than the kernel
  patterns and harder to frame as kernel-source ref→opt.

**Recommendation:** pause burn-cubecl/cubecl distillation here. The corpus
is now 68 rules across 15 categories (A–O), covering memory access, compute,
fusion, dispatch, parallelism, flash-attention, linear-attention, MoE, NSA,
varlen, autotune dispatch, conv-as-GEMM/layout, **kernel polymorphism,
dtype/alignment specialization, and IR strength reduction** (the 3 new
categories this batch adds). The highest-value next step is **wiring the
corpus end-to-end** — either (a) connect `KernelExpertRouter` to the corpus
so a heal invocation actually routes by kernel signature, or (b) land a real
GPU kernel heal target in `riir-gpu` so the corpus is exercised against real
code. Both are design/impl tasks, not distillation, and both are blocked on
a concrete consumer (per the prior session's assessment).

# Research 002 — Popcorn kernel_opt distillation: Batch 1 (20 rules)

**Status:** COMPLETE (batch 1 of ~15-25 batches for the full ~300-500 rule target)
**Date:** 2026-08-14
**Trigger:** Proposal 012 §Phase 0 ("~2 weeks reading job"). The kernel_opt domain
ships with 2 placeholder rules; this is the first batch of real popcorn-distilled rules.
**Source:** `/Users/katopz/git/popcorn` (local fork of `tilde-research/popcorn`) — 100
kernels × ~100 implementations × 7 backends.

## What popcorn is

A PyTorch kernel dispatcher benchmark. Each of the 100 kernels has:

1. A **reference implementation** — the naive PyTorch version (correct, slow).
2. Multiple **backend implementations** registered via `.register("backend", source="...")`:
   - `fla` (flash-linear-attention), `liger` (Liger-Kernel), `quack` (cute kernels),
     `unsloth`, `triton` (hand-written Triton), `cuda` (raw CUDA C++).
3. A **predicate** gate (optional) — shape/dtype conditions under which the backend
   is correct (e.g., `_aligned()` requires `shape[-1] % 8 == 0` for 16-bit dtypes).
4. A **forward_only** flag (optional) — marks backends that don't implement backward.

The optimization patterns are embedded in the gap between the reference and each backend
impl. Popcorn's value is the **methodology** (reference-check + benchmark records + the
(ref, opt) pair shape), not the Triton/CUDA source code itself — that doesn't port to
the Rust/CubeCL stack.

## What's portable vs not (Proposal 012 §"The corpus")

| Portable (distill these) | Not portable |
|---|---|
| Coalescing rules (memory access pattern → occupancy) | Triton source (different language) |
| Tiling heuristics (block-size vs shared-mem tradeoff) | Python dispatch harness |
| Bank-conflict avoidance (padding, swizzling) | Backend packages (Liger/FLA/cuDNN) |
| Occupancy modelling (warps/SM, register pressure) | cuDNN/TE proprietary code |
| Fusion opportunities (RoPE+GEMV, RMSNorm+GEMV) | — |
| The (ref, opt) pair shape — clippy's (buggy, fixed) for kernels | — |
| Reference-check validation methodology | — |

## Distillation methodology

For each popcorn kernel, extract:

1. **The GPU concept** — the named optimization technique (e.g., "coalesced memory access").
2. **The reference shape** — Rust-shaped pseudocode of the naive pattern.
3. **The optimized pattern** — Rust-shaped pseudocode of the hand-tuned pattern.
4. **The rule id** — kebab-case name encoding the concept + variant.

The Rust-shaped pseudocode constraint is load-bearing: the shared `RustAnalyzerPruner`
validates syntax, so all `ref_shape` / `opt_pattern` strings must parse as valid Rust
(function signatures + bodies, no Triton/CUDA syntax). The GPU *concept* is encoded in
the algorithm structure, not the language syntax.

## Batch 1 — 19 new rules + 2 original placeholders = 21 total (5 categories)

### Category A: Memory access patterns (5 rules)

Distilled from: `rotary_embedding_tl.py`, `swiglu.cu`, `matmul.py`, `rms_norm.py`,
`multi_token_attention.py`.

| Rule ID | GPU Concept | Source |
|---|---|---|
| `coalesced-row-read` | coalesced memory access (strided → contiguous) | rotary_embedding_tl |
| `vectorized-pack-load` | vectorized 16-byte packed loads | swiglu.cu `Pack<N>` |
| `grid-stride-loop` | grid-stride work distribution | swiglu.cu fwd_kernel |
| `shared-mem-tile-reuse` | shared-memory tiling for GEMM | matmul (tiled variant) |
| `masked-boundary-load` | masked load with boundary fallback | rotary_embedding_tl |

### Category B: Compute patterns (5 rules)

Distilled from: `rotary_embedding_tl.py`, `_utils.py`, `swiglu.cu`, `_quant.py`.

| Rule ID | GPU Concept | Source |
|---|---|---|
| `autotune-warps-stages` | autotune over num_warps × num_stages | rotary_embedding_tl `@triton.autotune` |
| `upcast-before-reduction` | fp32 upcast before mean/sum reductions | _utils.rs `upcast` + rms |
| `branchless-select` | tl.where / select instead of conditional branch | rotary_embedding_tl (GPT-J interleaved) |
| `unroll-fixed-inner` | #pragma unroll for fixed-size inner loops | swiglu.cu `#pragma unroll` |
| `forceinline-device-fn` | __forceinline__ for small device helpers | swiglu.cu `__device__ __forceinline__` |

### Category C: Fusion patterns (4 rules)

Distilled from: `rms_norm_swish_linear.py`, `bit_linear.py`, `cross_entropy.py`,
`rms_norm_linear.py`.

| Rule ID | GPU Concept | Source |
|---|---|---|
| `fuse-norm-act-linear` | RMSNorm + activation + linear in one pass | rms_norm_swish_linear |
| `fuse-quant-matmul` | quantize + matmul fused | bit_linear (BitNet b1.58) |
| `fuse-softmax-mask` | softmax + causal mask fused | multi_token_attention |
| `fuse-xent-logsumexp` | cross-entropy + logsumexp fused | cross_entropy (liger/fla) |

### Category D: Dispatch patterns (3 rules)

Distilled from: `rotary_embedding_tl.py`, `group_norm.py`, `multi_token_attention.py`,
`_triton.py`.

| Rule ID | GPU Concept | Source |
|---|---|---|
| `sm-count-block-size` | block size from SM count | rotary_embedding_tl `BT = min(128, ...)` |
| `predicate-gated-backend` | shape/dtype predicate gates backend selection | group_norm `_wide_groups` |
| `pow2-block-alignment` | next_power_of_2 for hardware alignment | rotary_embedding_tl `next_power_of_2` |

### Category E: Parallelism patterns (3 rules)

Distilled from: `rotary_embedding_tl.py`, `swiglu.cu`, `rotary_embedding_tl.py`.

| Rule ID | GPU Concept | Source |
|---|---|---|
| `multi-dim-grid-launch` | multi-dimensional grid for batch×head parallelism | rotary_embedding_tl `(cdiv(T,BT), B, H)` |
| `block-count-clamp` | grid block count clamped to 65535 | swiglu.cu `blocks_for` |
| `backward-as-conjugate` | backward reuses forward with conjugate | rotary_embedding_tl `_Rotary` |

## How these map to the KernelRule struct

```rust
KernelRule {
    rule_id: "coalesced-row-read".to_string(),
    gpu_concept: "coalesced memory access".to_string(),
    ref_shape: "// naive: strided read (column-major, uncoalesced)\nfn load_col(...)",
    opt_pattern: "// optimized: contiguous read (row-major, coalesced)\nfn load_row(...)",
}
```

All 19 new rules ship in `src/domains/kernel_opt/corpus.rs` alongside the 2 existing
placeholders. The corpus goes from 2 → 21 entries. Tests updated accordingly.

## What's NOT in this batch (honest deferral)

- **Online softmax / flash-attention tiling** — the most complex kernel pattern (2-pass
  online softmax with KV streaming). Deserves its own batch; distilling it well requires
  reading `wall_attn_tl.py` (39KB) + `nsa_tl.py` (18KB) carefully.
- **Linear-attention recurrence kernels** — DeltaNet, GSA, RWKV7, lightning attention.
  These have state-carrying sequential loops that can't be parallelized the same way.
  Needs a separate "stateful recurrence" category.
- **MoE dispatch kernels** — `momoe_tl.py` (43KB) is the largest file; the expert-routing
  pattern is distinct enough to warrant its own category.
- **Backward pass kernels** — most patterns above are forward-only; backward has its own
  optimization surface (gradient accumulation, checkpoint rematerialization).
- **Per-kernel shape signatures** — the router's activity vector `[M, N, K, dtype, hw]`
  needs per-rule metadata so `pick_domain` can route. This is a follow-up that needs the
  router integration, not just the corpus.

## Estimated full corpus size

Based on batch 1's yield (19 new rules from ~8 kernels read in depth):
- 100 kernels total, ~100 impls each
- ~2-4 distinct optimization concepts per kernel (some share concepts)
- Estimated unique rules: **300-500** (lower than Proposal 012's 500-1000 estimate —
  many kernels share the same optimization concepts, so the unique rule count is lower
  than the raw kernel×impl product)
- At ~20 rules/batch, that's ~15-25 batches. The "~2 weeks" estimate from Proposal 012
  is realistic for the full distillation.

## GOAT gate for this batch

This batch is a **corpus expansion** — no new substrate, no new trait, no new feature
gate. The GOAT is:
- **G1 (compile):** the corpus builds with `--features kernel_heal`.
- **G3 (no-regression):** existing kernel_opt tests pass with the expanded corpus.
- No G2/G4/G5 — this is content, not a primitive. The perf gate is the bench framework
  (`rust_perf_bench` analog), which lands when the corpus is large enough to warrant a
  Pareto validation pass.

## Next steps

- [x] Batch 1: 19 new rules distilled (this research note + corpus.rs expansion)
- [ ] Batch 2: flash-attention tiling patterns (online softmax, KV streaming)
- [ ] Batch 3: linear-attention recurrence (DeltaNet, GSA, RWKV7)
- [ ] Batch 4: MoE dispatch + expert routing
- [ ] Batch 5+: remaining kernels (losses, activations, normalization variants)
- [ ] Router integration: per-rule activity-vector metadata for `pick_domain`
- [ ] Full corpus GOAT: Pareto validation bench (the `kernel_perf_bench` analog)

## References

1. **popcorn** — `/Users/katopz/git/popcorn` (local fork of `tilde-research/popcorn`)
2. **Proposal 012** — `katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md` §Phase 0
3. **Plan 001** — `riir-clippy/.plans/001_multi_domain_framework_refactor.md` Phase 3
4. **Proposal 001** — `riir-clippy/.proposals/001_multi_domain_modelless_code_healer.md`

# Plan 004 — kernel_opt Activity Vector Extension (5 → 8 dims)

**Proposal:** [012](../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md) §"The
latent-MoE expert selector" — the activity vector must discriminate operation
classes, not just kernel shapes.
**Branch:** `develop` (per global rule — no feature branches)
**Status:** COMPLETE 2026-08-14. All phases landed: Phase 1 (8-dim activity vector + build_activity helper + 8-dim centroids), Phase 2 (15/15 reachability test + per-category canonical routing), Phase 3 (docs + CI), Phase 4 (commit + push).
**Domain:** `kernel_opt` (opt-in `kernel_heal` feature)
**Predecessor:** [Plan 003](003_kernel_opt_router_corpus_wiring.md) — corpus
wiring landed the 15 category experts + documented C+O unreachability.

## Goal

Resolve the **documented mathematical limitation** from Plan 003: the 5-dim
shape-only activity vector `[M, N, K, dtype, hw]` is rank-deficient for 15
categories. Categories C (fusion — balanced) and O (IR strength reduction — low
everything) are unreachable via `argmax(activity · centroid)` with non-negative
activity, because C is always beaten by a more peaked centroid on its strongest
axis and O's low-everything signature can't beat any higher centroid.

The fix: extend `ACTIVITY_DIM` from 5 to 8 by adding **3 operation-class one-hot
axes**:

| New axis | Index | High for | Encodes |
|---|---|---|---|
| `is_attention` | 5 | F, G, I | attention-family kernels (flash-attn, linear-attn, NSA) |
| `is_fusion` | 6 | C | multi-op fused kernels (norm+act, quant+matmul, masked softmax) |
| `is_ir_transform` | 7 | O | compile-time IR strength reduction (mul/div by pow2 → shift) |

With these axes, C gets a unique `is_fusion=1.0` signal (no other centroid sets
it), and O gets a unique `is_ir_transform=1.0` signal. Both become reachable.
The `is_attention` axis makes the attention-family categories (F, G, I) crisper
— they're already shape-distinguishable, but the attention flag reinforces the
routing.

**Result: 15/15 categories reachable** (was 13/15).

## Substrate-first gate (pre-flight, MANDATORY)

- [x] `pick_domain<const N, const A>` (katgpt-core Plan 558) — already
      generic over `A` (the activity dim). **No change needed** — extending
      `A` from 5 to 8 is a pure const-generic + data change.
- [x] `KernelExpertRouter` (this repo) — the `route()` call already passes
      `&[f32; ACTIVITY_DIM]`. Changing `ACTIVITY_DIM` propagates automatically.
- [x] Workspace grep for `is_attention` / `is_fusion` / `is_ir_transform` /
      `KernelOpClass` / `OpClass` — **zero hits**. No existing operation-class
      substrate anywhere in the stack.

No new substrate invented. This is pure **const + data + accessor** addition.

## Design

### The 8-dim activity vector

```
[M, N, K, dtype, hw, is_attention, is_fusion, is_ir_transform]
 0   1   2    3     4        5           6             7
```

Indices 0-4 unchanged (shape + dtype + hw). Indices 5-7 are the new
operation-class one-hot flags.

### The 15 updated centroids (8-dim)

Each centroid gains 3 components `[is_attention, is_fusion, is_ir_transform]`:

| Idx | Category | Centroid `[M,N,K,dt,hw, attn, fus, ir]` | New flags |
|---|---|---|---|
| 0 | A: memory access | `[0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]` | none |
| 1 | B: compute | `[0.0, 0.1, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0]` | none |
| 2 | C: fusion | `[0.5, 0.5, 0.5, 0.1, 0.1, 0.0, 1.0, 0.0]` | **is_fusion** |
| 3 | D: dispatch | `[0.0, 0.0, 0.0, 0.1, 0.9, 0.0, 0.0, 0.0]` | none |
| 4 | E: parallelism | `[0.5, 0.5, 0.0, 0.0, 0.7, 0.0, 0.0, 0.0]` | none (peaked M+N+hw) |
| 5 | F: flash-attn | `[0.8, 0.8, 0.6, 0.0, 0.0, 1.0, 0.0, 0.0]` | **is_attention** |
| 6 | G: linear-attn | `[0.2, 0.2, 0.6, 0.3, 0.0, 1.0, 0.0, 0.0]` | **is_attention** |
| 7 | H: MoE dispatch | `[0.2, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0]` | none (moderate M+K+dt) |
| 8 | I: NSA block-sparse | `[0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0]` | **is_attention** |
| 9 | J: varlen | `[0.85, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0]` | none (peaked M+dt) |
| 10 | K: autotune | `[0.0, 0.0, 0.0, 0.5, 0.8, 0.0, 0.0, 0.0]` | none (dt+hw balanced) |
| 11 | L: conv/layout | `[0.6, 0.6, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0]` | none |
| 12 | M: polymorphism | `[0.2, 0.2, 0.0, 0.6, 0.3, 0.0, 0.0, 0.0]` | none (small shape + dt+hw) |
| 13 | N: dtype/align | `[0.0, 0.0, 0.0, 0.9, 0.1, 0.0, 0.0, 0.0]` | none |
| 14 | O: IR strength | `[0.0, 0.0, 0.0, 0.05, 0.05, 0.0, 0.0, 1.0]` | **is_ir_transform** |

### Why these axes resolve C + O

**C (fusion):** centroid now has `is_fusion=1.0`. For activity vector
`[0.5, 0.5, 0.5, 0.1, 0.1, 0.0, 1.0, 0.0]`:
- C dot = 0.25+0.25+0.25+0.01+0.01 + **1.0** = 1.77
- F dot (next highest) = 0.4+0.4+0.3 = 1.1
- C wins. **Reachable.**

**O (IR strength):** centroid now has `is_ir_transform=1.0`. For activity
vector `[0.0, 0.0, 0.0, 0.05, 0.05, 0.0, 0.0, 1.0]`:
- O dot = 0.0025+0.0025 + **1.0** = 1.005
- Any other centroid: their `is_ir_transform` = 0, and O's activity is all
  zeros on M/N/K → their dot is ≤ 0.1. O wins. **Reachable.**

### Consumer-facing API

Add named axis-index constants + a helper so consumers don't hardcode array
indices:

```rust
pub const AXIS_M: usize = 0;
pub const AXIS_N: usize = 1;
pub const AXIS_K: usize = 2;
pub const AXIS_DTYPE: usize = 3;
pub const AXIS_HW: usize = 4;
pub const AXIS_IS_ATTENTION: usize = 5;
pub const AXIS_IS_FUSION: usize = 6;
pub const AXIS_IS_IR_TRANSFORM: usize = 7;

/// Build an activity vector from shape + operation-class flags.
pub fn build_activity(
    m: f32, n: f32, k: f32, dtype: f32, hw: f32,
    is_attention: bool, is_fusion: bool, is_ir_transform: bool,
) -> [f32; ACTIVITY_DIM] { ... }
```

This closes the "no consumer" gap from Plan 003: the activity vector builder IS
the consumer-facing entry point. A future heal pipeline that receives a kernel
signature + operation class can call `build_activity(...)` then `router.route(...)`.

### Backward compatibility

`ACTIVITY_DIM` changes 5 → 8. This IS a breaking change for any code that
constructs `[f32; 5]` activity vectors — but no such code exists (the router was
unwired scaffolding in Plan 003; nothing calls `route()` outside tests). The
`reachable_categories_covers_13_of_15` test is replaced by
`all_15_categories_reachable` (the improvement it was designed to celebrate).

## Phases

### Phase 1 — Const + data update (CORE)

- [x] 1.1 Change `ACTIVITY_DIM: usize = 5` → `8`.
- [x] 1.2 Add axis-index named constants (`AXIS_M` … `AXIS_IS_IR_TRANSFORM`).
- [x] 1.3 Add `build_activity(...)` helper.
- [x] 1.4 Update `CATEGORY_KERNEL_DIRECTIONS` to the 8-dim centroids (table above).
- [x] 1.5 Update module-level docs (remove "5-dim rank-deficient" caveat; document the 8-dim extension).

### Phase 2 — Test update

- [x] 2.1 Replace `reachable_categories_covers_13_of_15` with
      `all_15_categories_reachable` — sweep the 8-dim space (coarse grid +
      per-category canonical activity vectors) + assert 15/15 reachable.
- [x] 2.2 Add per-category canonical routing tests (each category has a
      signature activity vector that routes to it — 15 tests or one
      parameterized test).
- [x] 2.3 Add `build_activity` round-trip tests.

### Phase 3 — Validation + docs

- [x] 3.1 `cargo clippy --features kernel_heal --all-targets` clean.
- [x] 3.2 `cargo test --features kernel_heal --lib` — all tests pass.
- [x] 3.3 Update `mod.rs` "What ships now" table (router row: "✅ (8-dim
      activity, 15/15 reachable, Plan 004)").
- [x] 3.4 Update `AGENTS.md` + `README.md` router references.
- [x] 3.5 Run full 5-layer CI guard.

### Phase 4 — Commit + push

- [x] 4.1 Commit with `feat:` prefix.
- [x] 4.2 Push to `origin/develop`.

## GOAT gate

Infrastructure + correctness GOAT (same shape as Plan 003 — not a perf gain).

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | All 15 categories reachable | ✅ PASS (15/15, was 13/15) |
| G2 (perf) | `route()` stays zero-alloc + sub-µs | ✅ PASS (15 experts, 8 dims — stack arrays, 251 tests in 0.07s) |
| G3 (no-regression) | All existing tests pass | ✅ PASS (251 tests, 0 failed, 9 ignored) |
| G4 (alloc-free) | `route()` + `build_activity()` allocate zero bytes | ✅ PASS (stack only) |

No promotion target — `kernel_heal` stays opt-in (corpus is still pseudocode;
the GOAT for promotion is the deferred `kernel_perf_bench` numeric validator,
blocked on GPU).

## Non-goals (still deferred)

- **Centroid mining** via `CommittedFieldBlend` — needs (signature, speedup)
  tuples the pseudocode corpus doesn't carry.
- **Heal pipeline wiring** (router actually consulted in the L1 drafter) — the
  activity vector builder makes this easier, but the drafter still needs to
  receive kernel signatures as input.
- **`kernel_perf_bench` numeric validator** — blocked on GPU.

## References

- [Plan 003](003_kernel_opt_router_corpus_wiring.md) — documented the C+O
  unreachability + recommended "extend ACTIVITY_DIM from 5 to ~8 with
  operation-class one-hot axes" as the next step.
- [Proposal 012](../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md)
  §"The latent-MoE expert selector"
- [Plan 558](../katgpt-rs/.plans/558_variable_rank_domain_expert.md) —
  `pick_domain<N, A>` substrate (already generic over A).

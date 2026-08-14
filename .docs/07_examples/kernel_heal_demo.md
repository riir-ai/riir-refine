# kernel_heal_demo — kernel_opt routed healing pipeline (Plan 003/004)

**Source:** [`examples/kernel_heal_demo.rs`](../../examples/kernel_heal_demo.rs)
**Feature:** `kernel_heal` (or `kernel_verify` for CubeCL compile check)
**Run:** `cargo run --example kernel_heal_demo --features kernel_heal`

## What it demonstrates

The `kernel_opt` domain end-to-end with **expert routing** — the composition
that closes the Plan 003/004 gap (the `KernelExpertRouter` was corpus-wired +
15/15 reachable but had no runnable code path that actually called it):

1. **Build corpus**: `build_kernel_seed_corpus()` → 68 `KernelRule` entries
   across 15 categories (A–O).
2. **Build router**: `KernelExpertRouter::new()` → 15 category experts with
   8-dim activity vectors (Plan 004).
3. **Route + narrow**: for each representative kernel signature, `build_activity()`
   → `router.route()` → expert index → `rules_for_expert()` → narrowed candidate
   set (~4–6 rules vs 68 flat — a 15× candidate reduction).
4. **Draft**: `TernaryDrafter::draft_fix()` on the narrowed candidate set (L1
   ternary SIMD matvec or L0 corpus template fallback).
5. **Validate**: `KernelOptDomain::validate()` — syntax check (or CubeCL compile
   with `kernel_verify`).
6. **Thermal tier**: `KernelThermalRouter::route()` → always Plasma (modelless-first
   mandate; Warm/Cold/Freeze deferred to 4090 + riir-train).
7. **Flat vs routed comparison**: shows the 15× candidate reduction from routing.
8. **Fallback mode**: demonstrates the L0 corpus template path (no weights).

## The 8 representative signatures

Each routes to a different category expert, demonstrating that the router
discriminates kernel classes by shape + operation-class flags:

| Signature | Activity vector | Routes to |
|---|---|---|
| Naive matvec (strided read → coalescing) | M=0.9, rest low | A: memory access |
| Full flash-attention | M=N=K=1.0, is_attention | F: flash-attn tiling |
| Fused quantized GEMM | is_fusion | C: fusion |
| Conv2d 1×1 (layout transform) | M=0.25, N=0.9, dtype=1.0 | L: conv-as-GEMM + layout |
| Compile-time mul-by-pow2 | is_ir_transform | O: IR strength reduction |
| Varlen packing (cu_seqlens) | M=1.0, dtype=1.0 | J: varlen + chunked reduction |
| Compute-bound inner-product | K=1.0 | B: compute |
| Autotune dispatch | dtype=1.0, hw=1.0 | K: autotune dispatch |

## The composition pattern

This example is the **reference composition** for future kernel_opt consumers:

```rust
// 1. Build the router + corpus once at boot
let router = KernelExpertRouter::new();
let corpus = build_kernel_seed_corpus();

// 2. Per kernel signature: route → narrow → draft
let activity = build_activity(m, n, k, dtype, hw, is_attn, is_fusion, is_ir);
let expert = router.route(&activity);
let rule_ids = KernelExpertRouter::rules_for_expert(expert);
let candidates: Vec<KernelRule> = corpus.entries.iter()
    .filter(|e| rule_ids.contains(&e.rule_id.as_str()))
    .cloned()
    .collect();
let fix = drafter.draft_fix(&candidates[0].rule_id, &candidates[0].ref_shape, &candidates);
```

The router does the narrowing; the domain-agnostic drafter does the drafting.
This is the MoE (Mixture of Experts) pattern: the router selects the relevant
expert domain, then the drafter operates within that domain.

## Output shape

```text
📦 L0 Corpus: 113 rules (BLAKE3 verified: ✅)
   Categories: 15
     A: memory access — 6 rules
     ...

🔀 Router: 15 experts, 8-dim activity vector

─── Routed Heal Pipeline ──────────────────────────────────────────────────────

🔑 Signature: Naive matvec (strided read → coalescing)
   Routed to: expert 0 (A: memory access) — category: A: memory access
   Candidates: 6/113 rules (narrowed by router)
   Thermal:    Plasma
   Drafted:   coalesced-row-read [TernaryMatvec] conf=0.5234
   Validate:  ✅ VALID
   ...

─── Flat vs Routed Candidate Count ─────────────────────────────
  Flat corpus:     113 rules searched per heal
  Routed (avg):    7.5 rules searched per heal (category avg)
  Reduction:       15.0×
```

## Caveat

This demo validates that drafted patterns **parse as valid Rust** (or **compile**
with `kernel_verify`). It does NOT verify that the optimized kernel is actually
faster — the `kernel_perf_bench` numeric validator is deferred (blocked on GPU;
pseudocode rules aren't runnable kernels).

The routing centroids are hand-crafted (Plan 003/004). Mined centroids via
`CommittedFieldBlend` (on real `(signature, speedup)` tuples) would improve
routing accuracy for ambiguous signatures.

## See also

- [`../11_domains/kernel_opt.md`](../11_domains/kernel_opt.md) — the kernel_opt domain design
- [Plan 003](../../.plans/003_kernel_opt_router_corpus_wiring.md) — router ↔ corpus wiring
- [Plan 004](../../.plans/004_kernel_opt_activity_vector_extension.md) — 8-dim activity vector extension

# Plan 003 — kernel_opt Router ↔ Corpus Wiring (Category-Keyed Experts)

**Proposal:** [012](../../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md) §"The
latent-MoE expert selector" — the placeholder identity-direction router must be
replaced with corpus-aware experts.
**Branch:** `develop` (per global rule — no feature branches)
**Status:** COMPLETE 2026-08-14. All phases landed: Phase 1 (category mapping substrate), Phase 2 (router rewiring), Phase 3 (validation + docs), Phase 4 (commit + push). 13/15 categories reachable; C+O documented as unreachable (5-dim activity vector limitation).
**Domain:** `kernel_opt` (opt-in `kernel_heal` feature)

## Goal

Replace the `KernelExpertRouter`'s **placeholder** state — 4 identity-vector
experts (`coalesce`/`tile`/`fuse`/`vectorize`) disconnected from the 68-rule
corpus — with **corpus-aware experts**: one expert per corpus category (15
experts, A–O), each owning its category's rule_ids + a hand-crafted centroid
reflecting the kernel-shape signature that triggers that category.

The router remains **unwired into the heal pipeline** (the L1 drafter still
consults all 68 rules via flat `build_corpus()`). The deliverable is the
**router→corpus mapping** itself: a future heal pipeline can call
`router.category_for_expert(idx)` to get the category name, then
`corpus.rules_in_category(name)` to consult that category's rules first.

This is the **highest-value open item per Research 009** ("Router → corpus
wiring ... now the highest-value next step"). It is a design/impl task, not
distillation.

## Substrate-first gate (pre-flight, MANDATORY)

Before implementing, verify no substrate is duplicated:

- [x] `pick_domain<const N, const A>` (katgpt-core Plan 558) — the load-bearing
      expert-selection primitive. Pure `argmax(activity · directions)`,
      deterministic tie-break by lowest index, zero-alloc. **Consumed, not
      reinvented.**
- [x] `KernelExpertRouter` (this repo, `src/domains/kernel_opt/router.rs`) — the
      existing wrapper around `pick_domain`. **Extended, not replaced.**
- [x] `build_kernel_seed_corpus()` (this repo, `corpus.rs`) — 68 rules in 15
      categories. **Consumed via the new category mapping.**
- [x] Workspace grep for `category_for_rule` / `expert_for_category` /
      `route_to_category` / `KernelCategory` — **zero hits**. No existing
      category-keyed routing substrate anywhere in the 7-repo stack.

No new substrate invented. This is pure **data + accessor** addition on top of
the existing `pick_domain` + `KernelExpertRouter` + corpus.

## Design

### Why 15 experts (one per category), not 68 (one per rule)?

Proposal 012 §"The latent-MoE expert selector" specified "one expert per kernel
implementation in the corpus". The actual corpus has 68 *rules* (GPU concepts),
not 68 *kernel implementations*. The natural expert granularity is the
**category** — each category is a coherent optimization domain (memory access,
compute, fusion, dispatch, parallelism, flash-attn tiling, linear-attn
recurrence, MoE, NSA, varlen, autotune, conv/layout, polymorphism, dtype/align,
IR strength reduction). 15 experts cover the full pattern space; 68 experts
would be overfit (many rules in the same category have identical signatures).

### The activity vector problem (honest caveat)

The current activity vector is 5-dim: `[M, N, K, dtype, hw]` (normalized to
`[0,1]`). This is **shape-only** — it cannot discriminate operation-class
categories (F flash-attn, G linear-attn, H MoE, I NSA, J varlen) which need an
*operation-type* signal. The 15×5 centroid matrix is rank-deficient.

**This plan does NOT extend the activity vector.** Extending it (adding
operation-class axes) is a separate design task that needs a concrete consumer
to validate the signal encoding. The hand-crafted centroids here use the
existing 5 axes as best they can:
- Shape-driven categories (A memory, B compute, C fusion, D dispatch, E
  parallelism, K autotune, L conv/layout) get centroids keyed on M/N/K ratios.
- Operation-driven categories (F, G, H, I, J) get centroids that encode
  "long-sequence + reduction-heavy" signatures — the shape proxy for their
  operation class. This is a **deliberate approximation**, documented per
  category.
- Source-driven categories (M polymorphism, N dtype/align, O IR strength) get
  centroids keyed on dtype/hw axes (the signal that distinguishes them from
  shape-driven categories).

**Recommendation for the next plan:** extend `ACTIVITY_DIM` from 5 to ~8 by
adding operation-class one-hot axes (`is_attention`, `is_moe`, `is_reduction`).
Blocked on a concrete consumer that exercises the router in a real heal path.

### The 15 experts + centroids

| Idx | Category | Centroid signature (M, N, K, dtype, hw) | Rule count |
|---|---|---|---|
| 0 | A: memory access | `(0.8, 0.5, 0.3, 0.5, 0.5)` — M-dominated (large-tile read) | 6 |
| 1 | B: compute | `(0.5, 0.5, 0.8, 0.5, 0.5)` — K-dominated (reduction inner) | 5 |
| 2 | C: fusion | `(0.6, 0.6, 0.6, 0.5, 0.5)` — balanced (multi-op kernel) | 4 |
| 3 | D: dispatch | `(0.5, 0.5, 0.5, 0.5, 0.9)` — hw-dominated (backend choice) | 3 |
| 4 | E: parallelism | `(0.7, 0.7, 0.3, 0.5, 0.5)` — M+N-dominated (grid launch) | 3 |
| 5 | F: flash-attn tiling | `(0.9, 0.9, 0.7, 0.5, 0.5)` — large M+N (seq×seq) | 7 |
| 6 | G: linear-attn recurrence | `(0.4, 0.4, 0.6, 0.5, 0.5)` — small M+N, stateful | 6 |
| 7 | H: MoE dispatch | `(0.6, 0.4, 0.9, 0.5, 0.5)` — K-dominated (expert routing) | 9 |
| 8 | I: NSA block-sparse | `(0.7, 0.7, 0.5, 0.5, 0.5)` — block-indexed M+N | 4 |
| 9 | J: varlen packing | `(0.8, 0.3, 0.3, 0.5, 0.5)` — M-dominated (packed seq) | 3 |
| 10 | K: autotune | `(0.5, 0.5, 0.5, 0.5, 0.7)` — hw-leaning (config sweep) | 4 |
| 11 | L: conv/layout | `(0.8, 0.8, 0.4, 0.5, 0.5)` — image-shape M+N | 5 |
| 12 | M: polymorphism | `(0.5, 0.5, 0.5, 0.6, 0.6)` — dtype/hw (op trait) | 3 |
| 13 | N: dtype/alignment | `(0.4, 0.4, 0.4, 0.9, 0.5)` — dtype-dominated | 2 |
| 14 | O: IR strength reduction | `(0.3, 0.3, 0.3, 0.3, 0.3)` — low-rank u32 IR | 4 |

Centroids are **hand-crafted** (not mined via `CommittedFieldBlend` as Proposal
012 originally specified). Mining requires (signature, observed_speedup) tuples
per rule, which the pseudocode corpus doesn't carry. The hand-crafted centroids
are a documented placeholder; mining lands when a real consumer provides the
speedup signal.

### API additions

1. `KernelCategory` enum — 15 variants (A–O), `#[repr(u8)]`.
2. `KERNEL_CATEGORIES: [(KernelCategory, &str, &[&str]); 15]` — the
   category → rule_ids mapping (the corpus wiring).
3. `KernelExpertRouter::category_for_expert(idx) -> KernelCategory` —
   maps expert index to category.
4. `KernelExpertRouter::rules_for_expert(idx) -> &'static [&'static str]` —
   maps expert index to its category's rule_ids.
5. `KernelExpertRouter::expert_for_rule(rule_id) -> Option<usize>` —
   reverse lookup (which expert owns this rule).
6. `build_corpus_categorized() -> Vec<(KernelCategory, Vec<KernelRule>)>` —
   the corpus grouped by category (for future heal pipelines that want
   category-scoped consultation).

### Backward compatibility

`NUM_KERNEL_EXPERTS` changes from 4 → 15. `KERNEL_EXPERT_NAMES` changes from
the 4 placeholder names to the 15 category names. `IDENTITY_KERNEL_DIRECTIONS`
is replaced by `CATEGORY_KERNEL_DIRECTIONS` (the hand-crafted centroids). The
`KernelExpertRouter::new()` constructor now uses the category centroids.

The old `IDENTITY_KERNEL_DIRECTIONS` const + the 4 placeholder expert names
are **removed** (not deprecated) — they were always documented as placeholders
("Real per-expert centroids from popcorn distillation replace this const in a
follow-up plan"). This IS that follow-up plan.

The `with_directions()` constructor is kept (consumers can still inject custom
centroids).

## Phases

### Phase 1 — Category mapping substrate (CORE)

- [x] 1.1 Add `KernelCategory` enum (15 variants, `#[repr(u8)]`) to `mod.rs`.
- [x] 1.2 Add `KERNEL_CATEGORIES` const table (category → display name →
      rule_ids) to `corpus.rs`. Verify every rule_id in the 68-rule corpus
      appears in exactly one category (exhaustiveness test).
- [x] 1.3 Add `build_corpus_categorized()` to `corpus.rs` — groups the flat
      corpus by category.
- [x] 1.4 Add exhaustiveness + uniqueness tests (every rule in exactly one
      category; category rule counts match the plan table).

### Phase 2 — Router rewiring

- [x] 2.1 Replace `NUM_KERNEL_EXPERTS = 4` with `NUM_KERNEL_EXPERTS = 15`.
- [x] 2.2 Replace `KERNEL_EXPERT_NAMES` (4 placeholders) with the 15 category
      names.
- [x] 2.3 Replace `IDENTITY_KERNEL_DIRECTIONS` with `CATEGORY_KERNEL_DIRECTIONS`
      (the 15 hand-crafted centroids from the plan table).
- [x] 2.4 Update `KernelExpertRouter::new()` to use `CATEGORY_KERNEL_DIRECTIONS`.
- [x] 2.5 Add `category_for_expert()`, `rules_for_expert()`,
      `expert_for_rule()` accessors.
- [x] 2.6 Update existing router tests (the identity-direction tests change
      to category-centroid tests; the `with_directions` override test stays).

### Phase 3 — Validation + docs

- [x] 3.1 `cargo clippy --features kernel_heal --all-targets` clean.
- [x] 3.2 `cargo test --features kernel_heal --lib` — all tests pass (144).
- [x] 3.3 Update `lib.rs` re-exports (add `KernelCategory` + `KERNEL_CATEGORIES`
      + `build_corpus_categorized`).
- [x] 3.4 Update `mod.rs` "What ships now" table — router row moves from
      "✅ (identity directions)" to "✅ (category centroids, Plan 003)".
- [x] 3.5 Update `.docs/11_domains/kernel_opt.md` with the new expert layout.
- [x] 3.6 Update `AGENTS.md` + `README.md` router references.
- [x] 3.7 Run full 5-layer CI guard (248 tests, 0 failed, 9 ignored).

### Phase 4 — Commit + push

- [x] 4.1 Commit with `feat:` prefix.
- [x] 4.2 Push to `origin/develop`.

## GOAT gate

This is **not a perf-gain GOAT** — it's an **infrastructure + correctness GOAT**
(the same shape as the `authority` feature promotion in riir-mmorpg-examples
Plan 008). The value is establishing the router→corpus mapping so a future heal
pipeline can consult category-relevant rules first.

| Gate | Criterion | Status |
|---|---|---|
| G1 (correctness) | Every corpus rule appears in exactly one category; category rule counts match the plan table | ✅ PASS |
| G2 (perf) | `route()` stays zero-alloc + sub-µs (15 experts vs 4 — still `[f32; 15]` on stack) | ✅ PASS (0.03s for 144 tests) |
| G3 (no-regression) | All existing tests pass (with updated expectations for the 4→15 expert change) | ✅ PASS (248 tests, 0 failed) |
| G4 (alloc-free) | `route()` + accessors allocate zero bytes | ✅ PASS (stack arrays only) |

No promotion target — `kernel_heal` stays opt-in (the corpus is pseudocode; the
GOAT gate for promotion is the deferred `kernel_perf_bench` numeric validator,
blocked on GPU).

## Non-goals (deferred)

- **Activity vector extension** (5 → 8 dims with operation-class axes) — needs
  a concrete consumer to validate the signal encoding.
- **Centroid mining** via `CommittedFieldBlend` — needs (signature, speedup)
  tuples the pseudocode corpus doesn't carry.
- **Heal pipeline wiring** (router actually consulted in the L1 drafter) —
  needs a concrete consumer that exercises `kernel_heal` end-to-end.
- **`kernel_perf_bench` numeric validator** — blocked on GPU.

## References

- [Proposal 012](../../katgpt-rs/.proposals/012_kernel_heal_thermal_moe.md) §"The
  latent-MoE expert selector"
- `Plan 558` —
  `pick_domain` substrate
- [Research 009](../.research/009_burn_cubecl_cubecl_opt_kernel_opt_distillation_batch8.md)
  — "Router → corpus wiring ... now the highest-value next step"
- [Plan 001](001_multi_domain_framework_refactor.md) — the multi-domain
  framework that introduced `KernelExpertRouter` as scaffolding

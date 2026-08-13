# AGENTS.md — riir-clippy

The global `~/.agents/` rules apply; this file documents repo-local context.

## Role

**Multi-domain modelless code healer** — a pluggable framework behind the
`HealDomain` trait (Plan 001) that heals code across multiple domains using a
drafter+pruner+corpus pattern. This is a **developer tooling repo**, NOT a
game concern, NOT a training repo, NOT a chain repo.

Three domains ship (Plan 001):
1. **`clippy_lints`** (default-on) — heals `cargo clippy` warnings. The
   original domain; 10 common lints across style/complexity/perf/nursery.
2. **`rust_perf`** (default-on since 2026-08-13, Phase 2) — heals rust perf
   issues. 21 rules distilled from the `rust-optimize` skill checklist
   (complexity, allocation, layout, concurrency, simd, arithmetic, iterators,
   dispatch, caching, bevy, wasm). The **perf-bench Pareto validator** (Bench 013,
   `rust_perf_bench` feature) is the promotion gate — GOAT G1+G5 PASS. 3 of
   17 benchable rules (`precompute-table`, `papaya-lockfree`,
   `dedupe-hashset-vs-vec`) carry `context_notes` annotations recording the
   bench finding (2 ParetoDominated + 1 Mixed in the default config —
   consumers should treat annotated rules as context-dependent).
3. **`kernel_opt`** (opt-in, Phase 3) — heals GPU kernel optimization. 2
   placeholder rules (real popcorn distillation deferred per Proposal 012
   §Phase 0). Absorbs Proposal 012's design (thermal-tier + latent-MoE +
   numeric validator); 4090 LoRA deferred.

The pipeline (shared across all domains):
1. **L0 — Rule distillation**: each rule → named entry + bounded fix-space
2. **L1 — Plasma/hot path drafter**: modelless ternary SIMD matvec, zero-alloc (Issue 001, DONE)
3. **L2 — Validator**: `syn`-based syntax parse check (always-on, modelless — Issue 002, DONE).
   Domain-specific verification is opt-in:
   - clippy: `cargo clippy` re-run (`clippy_verify`, Issue 003, DONE)
   - rust_perf: `cargo test` re-run (`rust_perf_verify`, Plan 001 Phase 2, DONE)
   - kernel_opt: CubeCL compile (`kernel_verify`, Plan 001 Phase 3, STUB —
     full numeric compare deferred per Proposal 012 §Phase 0)
4. **L3 — Ruliology candidate search** (opt-in): Pareto-front enumeration
5. **L4 — LoRA fallback** (deferred): adaptive learning, last resort

## Why this is a separate repo (not in riir-ai)

The code healer borrows the **drafter+pruner+corpus pattern** from
`riir-games-quest/quest_grammar` (TernaryDraftModel + SealQuestCorpus +
ConstraintPruner), but has **zero game-domain coupling**:

- No dependency on `riir-engine`, `riir-games`, or any game crate
- No dependency on `riir-chain`, `riir-neuron-db`
- Only public dep: `katgpt-core` (ConstraintPruner trait + ternary SIMD matvec
  + `pick_domain` for kernel_opt's expert router, Plan 558)
- The `TernaryDraftModel` pattern is reimplemented locally (~200 LOC), not
  imported cross-workspace

Putting it in riir-ai would violate the domain boundary: riir-ai is for **game
runtime + engine**, not developer tools. The prior rejection of extracting
riir-games from riir-ai (Proposal 017 era) was correct because riir-games has
heavy bidirectional coupling with riir-engine. Code healing has no such
coupling — it's a clean, self-contained developer tool.

### Plan 001 multi-domain architecture (2026-08-13)

Plan 001 generalized the framework from a clippy-only healer into a pluggable
multi-domain framework behind the [`HealDomain`](src/domain.rs) trait. The
three domains (clippy_lints, rust_perf, kernel_opt) share the same
L0→L1→L2→L3→L4 pipeline; each domain owns its corpus + validator. **Proposal
012's GPU kernel optimizer is absorbed as the `kernel_opt` domain** (Phase 3)
— the user's correction ("why it can't be clippy?") overrode Proposal 012's
"new repo" recommendation. The trait abstraction makes future extraction a
move, not a rewrite.

**Cross-domain auto-routing is NOT in scope** — each domain is selected by
enabling its feature flag. A unified `cargo heal` CLI that auto-detects
domain from code shape is a future design sketch (Proposal 001 §Product
vision), not shipped.

### `kernel_opt`-specific layers (Phase 3)

The `kernel_opt` domain adds two layers on top of the shared pipeline:

1. **`KernelExpertRouter`** (`src/domains/kernel_opt/router.rs`) — wraps
   `katgpt_core::variable_rank_domain_expert::pick_domain<4, 5>` (Plan 558).
   Activity vector = kernel signature `[M, N, K, dtype, hw]`; domain
   directions = per-expert centroids (placeholder: identity vectors until
   popcorn distillation lands). Modelless argmax — zero allocation.

2. **`KernelThermalRouter`** (`src/domains/kernel_opt/thermal.rs`) — the
   FlashAR/QGF thermal-tier pattern: Plasma→Hot→Warm→Cold→Freeze escalation.
   `ThermalTier` is `#[repr(u8)]` (1 byte, sync-efficient). The scaffold
   returns Plasma for all calls (modelless-first mandate); Warm/Cold/Freeze
   require 4090 CUDA + LoRA (deferred to riir-train).

## Sibling-Repo Layout

```
/git/riir-clippy          ← this repo (clippy-healing pipeline)
/git/katgpt-rs            ← katgpt-core (NON-OPTIONAL: ConstraintPruner trait + ternary matvec)
/git/riir-ai              ← NOT a dep (game runtime — wrong domain)
/git/riir-train           ← OPTIONAL dep (only for L4 LoRA fallback, deferred)
/git/riir-chain           ← NOT a dep
/git/riir-neuron-db       ← NOT a dep
```

Path deps in `Cargo.toml` assume the standard `/git/` sibling layout.

## Build Commands

```bash
# Default (clippy_lints domain + modelless path)
cargo check
cargo clippy --all-targets
cargo test --lib

# rust_perf domain (Plan 001 Phase 2)
cargo check --features rust_perf
cargo run --example perf_heal_demo --features rust_perf

# kernel_opt domain (Plan 001 Phase 3, 4090 DEFERRED)
cargo check --features kernel_heal

# With ruliology candidate search (L3)
cargo check --features ruliology_search

# All features (the pre-existing ternary_inference/riir-gpu failure was fixed
# in riir-ai commit 4bfe3b30 — relaxed attn_sub_on cfg gate to cubecl_runtime)
cargo check --all-features

# Full CI guard (5 layers: no-dep / default / per-feature / all-features / clippy+tests)
./scripts/ci_feature_guard.sh

# Re-run GOAT benches (see .benchmarks/001_goat_consolidated.md for measured results)
cargo bench --bench bench_001_l1_matvec
cargo bench --bench bench_002_l2_pruner_syn
cargo run --example ruliology_search_demo --features ruliology_search --release
```

## Feature Flag Discipline

- `default` = `["clippy_lints"]`: clippy lint healing domain + L0 (corpus) + L1 (ternary drafter) + L2 (native rust-analyzer pruner)
- `clippy_lints` (default-on): the clippy lint healing domain.
- `clippy_verify` (opt-in): Phase 3 lint resolution verification via real `cargo clippy` re-run. Slow (~130ms/verify), requires cargo+clippy on PATH. Implies `clippy_lints`.
- `rust_perf` (default-on since 2026-08-13, Plan 001 Phase 2): rust performance healing domain. 21 rules from the `rust-optimize` skill checklist. Promoted after Bench 013 GOAT G1+G5 PASS. 3 of 17 benchable rules (`precompute-table`, `papaya-lockfree`, `dedupe-hashset-vs-vec`) carry `context_notes` annotations (Bench 013: 2 ParetoDominated + 1 Mixed in default config — consumers should treat as context-dependent).
- `rust_perf_verify` (opt-in): `cargo test` re-run for perf rule verification. Slow (seconds), requires cargo on PATH. Implies `rust_perf`.
- `rust_perf_bench` (opt-in, Plan 001 Phase 2 GOAT gate): generated microbenchmarks at multiple input sizes → Pareto verdict. **GOAT G1+G5 PASS (Bench 013)** — 14/17 benchable rules Pareto-dominant (o1-lookup 570x, bsearch-sorted-slice 16-2709x, simd-chunk 3.9x, bool-to-bitpack 7-11x, branchless-conditional 10x, static-str 42x, vec-with-capacity 1.1-4.3x, format-concat-loop 42-95x, mutex-to-atomic 2.5x, f32-counter-to-int 7.5-11x, entry-vs-contains-insert 1.5x, loop-fusion 1.5x, collect-avoidance 2.3-3.0x, dyn-to-enum 2.5-2.9x); 2/17 honestly dominated (precompute-table: compiler optimizes popcount to POPCNT in release; papaya-lockfree: benefit needs multi-threaded contention); 1/17 Mixed (dedupe-hashset-vs-vec: wins at N ≥ 1000, loses at small N due to HashSet overhead). Closes Proposal 001 caveat 3. Implies `rust_perf`.
- `kernel_heal` (opt-in, Plan 001 Phase 3): GPU kernel optimization healing domain. Absorbs Proposal 012. Enables `katgpt-core/variable_rank_domain_expert` for the `pick_domain` router (Plan 558). 4090 LoRA + real popcorn corpus DEFERRED.
- `kernel_verify` (opt-in): CubeCL compile check. **STUB** — delegates to shared `RustAnalyzerPruner`; full numeric compare deferred (Proposal 012 §Phase 0). Implies `kernel_heal`.
- `ruliology_search` (opt-in): L3 Pareto-front candidate search. Reuses the algorithm pattern from `katgpt-ruliology` (Bench 572) without a code-level dep. Runs once per lint invocation (~400µs/10 candidates), not per-tick. Implies `clippy_lints`.
- `self_evolve` (opt-in, Issue 004 + Issue 008): latent-first self-evolution loop — `LatentFixMemory` (3-axis retrieval over fix trajectories) + `LatentSkillEvolution` (EMA direction-vector update). GOAT G1-G5 ALL PASS (G5: 70pp healing rate improvement in controlled fixture). **G6/G7 validated against real cargo clippy feedback (Issue 008): G6 10/10 corpus truthfulness PASS, G7 50.0pp improvement (50% → 100%) PASS.** Borrows `dot+sigmoid` from riir-agents + 3-axis retrieval from riir-rag, reimplemented locally (~730 LOC, boundary-compliant). Stays opt-in (deployment-environment constraint: `clippy_verify` is itself opt-in). The mechanism is validated; the opt-in gate is not a correctness concern.
- `wasm_pruner` (opt-in, deferred): L2 WASM rust-analyzer variant (blocked on rubrc)
- `clippy_lora` (opt-in, deferred): L4 adaptive LoRA fallback (blocked on riir-train integration)
- `ternary_inference` (opt-in, Bench 010): katgpt ternary generation on clippy prompts. The pre-existing compile error (`attn_sub_on` not found in riir-gpu) was **FIXED** in riir-ai commit `4bfe3b30` — the cfg gate on `attn_sub_on` + `ATTN_SUBSTAGE_MASK` + `ATTN_SUBSTAGE_ALL` was relaxed from `all(cubecl_runtime, ternary_gemm_batched)` to just `cubecl_runtime` (behavior-preserving: the mask defaults to all-on). `--all-features` CI layer 4 now passes clean.

The modelless path (L0-L3) is PRIMARY. LoRA (L4) is the conservative last resort.
The self-evolution layer (Issue 004) is additive — it composes on top of L0-L2
without modifying the frozen pipeline. `rust_perf` was promoted to default-on
2026-08-13 (Bench 013 GOAT G1+G5 PASS) with the 2 ParetoDominated rules
carrying `context_notes` annotations ("demote loser" per global rule). The
annotations now propagate end-to-end through the drafter via a non-breaking
default method `HealEntry::context_notes() -> Option<&str> { None }` →
`DraftedFix::context_notes` (2026-08-13 post-promotion wiring). The drafter
surfaces the annotation; the consumer decides whether their context matches.
`kernel_opt` is NOT promoted to default-on until its GOAT gate (real corpus +
full numeric validator) passes.

## Documentation Shape

This repo ships a **10-folder numbered `.docs/` book** (opened 2026-08-11,
Issue 007), matching the `riir-ai/.docs/` / `riir-neuron-db/.docs/` /
`riir-game-sdk/.docs/` format: numbered folders for sort order, unnumbered
`.md` files inside, each folder has a `README.md` index with a purpose
blockquote + file table + "See also" cross-refs.

| Folder | Scope |
|---|---|
| `01_orientation/` | Crate role, sibling layout, boundary rationale, multi-domain architecture (Plan 001), build commands, numbering |
| `02_corpus/` | L0 rule distillation: `Corpus<D>` (generic) + BLAKE3 commitment + per-domain seed corpora |
| `03_drafter/` | L1 ternary SIMD matvec: `TernaryDrafter` (generic over `E: HealEntry`) + BLAKE3 `embedder` |
| `04_pruner/` | L2 syn validator: `RustAnalyzerPruner` + `ValidationResult` + honesty contract |
| `05_clippy_verify/` | clippy lint resolution: `ClippyVerifier` (opt-in `clippy_verify`) |
| `06_ruliology/` | L3 Pareto-front: `RuliologySearch` + `pareto_front` + `ast_delta_size` (opt-in) |
| `07_examples/` | The 5 in-tree examples (`heal_demo`, `perf_heal_demo`, `clippy_verify_demo`, `ruliology_search_demo`, `self_evolve_real_world_demo`) |
| `08_benchmarks/` | GOAT gate methodology + measured results |
| `09_lessons/` | Canonical patterns: borrow-without-dep, modelless-first, freeze/thaw, honest POC caveats, sigmoid-not-softmax |
| `10_self_evolve/` | Issue 004 + Issue 008: latent-first self-evolution loop (`HealPipeline` + `LatentFixMemory` + `LatentSkillEvolution`, opt-in `self_evolve`; G1–G7 ALL PASS) |
| `11_domains/` | **Plan 001 Phase 2+3**: multi-domain architecture — `rust_perf` (Phase 2) + `kernel_opt` (Phase 3, absorbs Proposal 012) |

The top-level `.docs/README.md` is the index. `README.md` (repo root) remains
the build surface (Status + Quick Start + Usage); `AGENTS.md` (this file)
remains the authoritative repo-local context.

For `.docs/`: files inside numbered folders have **no number prefix**.
Filenames are bare slugs (e.g. `crate_role.md`). The folder number provides
sort order; the filename provides identity. Add a new doc by dropping
`slug.md` in the right group folder and adding one line to that folder's
`README.md` index table.

## Numbering Discipline

Issue, plan, doc, benchmark, and research numbers are **monotonic and never
reused**. Before creating a new `.issues/` file, read `.issues/.highwater`, use
`value + 1`, write back. Same for `.plans/`, `.docs/`, `.benchmarks/`, `.research/`.

## Branch

`develop` is the working branch. Commit directly on `develop` per the global rule.

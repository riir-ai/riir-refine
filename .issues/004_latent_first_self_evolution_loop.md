# Issue 004 — Latent-First Self-Evolution Loop for riir-clippy

**Filed:** 2026-08-11
**From:** [Research 001](../.research/001_self_evolving_latent_first_agent.md) (distillation of arXiv:2608.03392 "Self-Evolving Coding Agents")
**Type:** POC — implement + GOAT-gate the latent-first self-evolution loop
**Status:** open

## Problem

riir-clippy has **zero self-evolution** (per the survey's 5-category taxonomy,
Research 001):
- L0 corpus is a frozen 10-lint seed — no accumulation of fix trajectories
- L1 drafter weights are frozen at load — no correction from success/failure
- L2 pruner is stateless — no memory of which fixes are more reliable

Every clippy lint is healed from scratch. The next time the same lint appears
on different code, the drafter has no memory of what worked before.

## Proposed design

A latent-first self-evolution loop behind the `self_evolve` feature flag
(default-off until G5 passes). Borrows two patterns from riir-ai
(boundary-compliant — reimplemented locally, NOT imported):

1. **`LatentFixMemory`** — 3-axis retrieval over fix trajectories (borrows
   riir-rag's latent+BM25+graph shape)
2. **`LatentSkillEvolution`** — EMA direction-vector update on success/fail
   (borrows `CommittedFieldBlend` Plan 321 pattern)

See [Research 001](../.research/001_self_evolving_latent_first_agent.md) §"The
latent-first design" for the full architecture diagram.

### The loop

```
lint → LatentFixMemory.retrieve(query) → past_fixes
      → L1 drafter (frozen weights + past_fixes as candidates)
      → score candidates via dot(fix_dir, ctx_dir) + sigmoid
      → L2 pruner (syn parse)
      → fix applied
      → outcome (success/fail) feeds back:
          - store trajectory in LatentFixMemory
          - update drafter direction[lint] via EMA
```

## Boundary compliance

- **Only dep:** `katgpt-core` (the legal dep — provides `simd_dot`, `simd_sigmoid`,
  `blake3`, `ConstraintPruner`)
- **Pattern borrow:** `dot+sigmoid` from riir-agents `ModellessReasoner` (~10 LOC)
- **Pattern borrow:** 3-axis retrieval shape from riir-rag (~150 LOC)
- **Pattern borrow:** EMA direction update from `CommittedFieldBlend` (~50 LOC)
- **NO new deps** on riir-ai, riir-engine, riir-games, riir-chain, riir-neuron-db

This is the same pattern-borrowing riir-clippy already does for
`TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner` (per Proposal 034
+ riir-ai AGENTS.md "pattern borrowing ≠ dependency" rule).

## Tasks

- [ ] **T1: `ModellessEmbedder` for (lint_name, code) tuples**
  - BLAKE3-deterministic 8-dim embedding using katgpt-core primitives
  - Same pattern as riir-rag's `ModellessEmbedder` (BLAKE3 + DFT + sigmoid)
  - ~30 LOC in `src/embedder.rs`

- [ ] **T2: `LatentFixMemory` — the memory store**
  - Store: `Vec<FixTrajectory { lint_name, buggy_hash, fixed_hash, direction: [f32;8], success_count, fail_count }>`
  - Retrieve: 3-axis scoring (latent dot+sigmoid, BM25 over lint_name, optional graph)
  - In-memory (papaya lock-free HashMap or simple Vec with linear scan for POC)
  - ~150 LOC in `src/memory.rs`

- [ ] **T3: `LatentSkillEvolution` — the EMA direction update**
  - On success: `direction[lint] += α·(fix_dir - direction[lint])` (α default 0.1)
  - On fail: `direction[lint] -= β·(fix_dir - direction[lint])` (β default 0.05)
  - Reader-side overlay — base weights never mutate (freeze/thaw invariant)
  - ~50 LOC in `src/evolve.rs`

- [ ] **T4: Wire the loop into the healing pipeline**
  - New module `src/self_evolve.rs` gated behind `self_evolve` feature
  - `HealPipeline` struct that composes: drafter + pruner + memory + evolution
  - `heal_with_evolution(lint, code) -> FixResult` entry point
  - ~50 LOC

- [ ] **T5: `self_evolve` feature flag in Cargo.toml**
  - Default-off
  - Gates T1-T4 modules

- [ ] **T6: G1 test — retrieval relevance**
  - Seed memory with 50 known (lint, buggy, fixed) trajectories
  - Query with held-out variants of same lints
  - Assert top-1 retrieved is same lint family ≥80%

- [ ] **T7: G2 bench — retrieval latency**
  - N=1000 trajectories in memory
  - Assert p50 query <50µs

- [ ] **T8: G3 test — no-regression**
  - All existing L0-L2 tests pass with `self_evolve` on + off

- [ ] **T9: G4 test — alloc-free steady state**
  - 100 consecutive heal calls, assert 0 steady-state allocation
  - Reuse scratch buffers (the TernaryDraftModel pattern)

- [ ] **T10: G5 test — the headline gate**
  - Run 100 simulated fix attempts (mix of success/fail outcomes)
  - Measure healing rate on 20 held-out lints BEFORE evolution (static corpus)
    vs AFTER evolution (memory + direction update)
  - Assert ≥5pp improvement (the modelless-gain threshold)
  - **If G5 FAILS:** the latent-first evolution mechanism doesn't work for
    clippy healing. Revert to static corpus, document the negative result.

## GOAT gate

| Gate | What | Target | Status |
|---|---|---|---|
| G1 | Retrieval relevance | top-1 same-lint-family ≥80% | ⏳ |
| G2 | Retrieval latency | p50 <50µs @ N=1000 | ⏳ |
| G3 | No-regression | existing tests green | ⏳ |
| G4 | Alloc-free steady state | 0 bytes steady-state | ⏳ |
| G5 | **Self-evolution gain** | ≥5pp healing rate improvement after 100 attempts | ⏳ |

**Promotion rule:** if G1-G5 all pass → promote `self_evolve` to default-on.
If G5 fails → keep opt-in, document the negative result (the modelless EMA
direction update doesn't capture clippy-fix signal). G5 failing does NOT
invalidate riir-clippy's existing L0-L2 pipeline — it just means the
latent-first evolution loop doesn't help for this domain.

## Out of scope

- **L4 LoRA model self-evolution** — stays deferred to riir-train (per
  Proposal 034 Phase 4). The survey's "model self-evolution" category
  (SWE-RL, Agent-RLVR, etc.) requires training, which is riir-train's domain.
- **Framework self-evolution** (SICA/Darwin Gödel Machine pattern) — N/A.
  riir-clippy is modelless; it doesn't rewrite its own source code.
- **Workflow/topology evolution** (SEMAG/AgentConductor pattern) — N/A.
  riir-clippy is single-agent.
- **Graph k-hop retrieval over lint taxonomy** — optional enhancement,
  defer until G5 passes with latent+BM25 only. The lint→lint graph (e.g.,
  `needless_return` ↔ `needless_question_mark`) is a real signal but adds
  complexity.

## References

- [Research 001](../.research/001_self_evolving_latent_first_agent.md) — the distillation
- [Proposal 034](../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the clippy-healing design
- [riir-agents `ModellessReasoner`](../riir-ai/crates/riir-agents/src/modelless.rs) — the `dot+sigmoid` pattern source
- [riir-rag docs](../riir-ai/.docs/02_crates/riir_rag.md) — the 3-axis retrieval pattern source
- [arXiv:2608.03392](https://arxiv.org/abs/2608.03392) — the survey paper

# Issue 004 — Latent-First Self-Evolution Loop for riir-clippy

**Filed:** 2026-08-11
**From:** [Research 001](../.research/001_self_evolving_latent_first_agent.md) (distillation of arXiv:2608.03392 "Self-Evolving Coding Agents")
**Type:** POC — implement + GOAT-gate the latent-first self-evolution loop
**Status:** resolved (all 10 tasks done, all 5 GOAT gates PASS)

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

- [x] **T1: `ModellessEmbedder` for (lint_name, code) tuples**
  - BLAKE3-deterministic 8-dim embedding using katgpt-core primitives
  - Same pattern as riir-rag's `ModellessEmbedder` (BLAKE3 + DFT + sigmoid)
  - ~60 LOC in `src/embedder.rs` (includes `embed`, `embed_into`, `embed_fix_into`)

- [x] **T2: `LatentFixMemory` — the memory store**
  - Store: `Vec<FixTrajectory { lint_name, lint_name_hash, buggy_code, fixed_code, direction, success_count, fail_count }>`
  - Retrieve: 3-axis scoring (latent dot+sigmoid, BM25 via hash, optional graph)
  - In-memory Vec with linear scan + inline top-K insertion (no full sort)
  - ~220 LOC in `src/memory.rs`

- [x] **T3: `LatentSkillEvolution` — the EMA direction update**
  - On success: `direction[lint] += α·(fix_dir - direction[lint])` (α default 0.1)
  - On fail: `direction[lint] -= β·(fix_dir - direction[lint])` (β default 0.05)
  - Initialization rule: only initialize on first SUCCESS (a fail tells us
    nothing about what a good fix looks like)
  - Reader-side overlay — base weights never mutate (freeze/thaw invariant)
  - ~180 LOC in `src/evolve.rs`

- [x] **T4: Wire the loop into the healing pipeline**
  - `src/self_evolve.rs` gated behind `self_evolve` feature
  - `HealPipeline` struct composes: drafter + pruner + memory + evolution
  - `heal(lint, code, corpus) -> Option<HealResult>` entry point
  - Candidate scoring combines evolution_score (0.6) + success_rate (0.4)
  - ~270 LOC

- [x] **T5: `self_evolve` feature flag in Cargo.toml**
  - Default-off
  - Gates T1-T4 modules

- [x] **T6: G1 test — retrieval relevance**
  - Seed memory with 50 trajectories across 10 lint families
  - Query with held-out variants of same lints
  - Assert top-1 retrieved is same lint family ≥80%
  - Result: **100%** (10/10) — see `tests/goat_gate_004.rs::g1_retrieval_relevance_top1_same_lint_family`

- [x] **T7: G2 bench — retrieval latency**
  - N=1000 trajectories in memory
  - Assert p50 query <50µs (release mode)
  - Result: **6µs p50** (release) — debug mode is ~57µs (no inlining + no SIMD)
  - Test is `#[cfg_attr(debug_assertions, ignore)]` — mirrors the established
    `g2_monster_ai_under_load` pattern

- [x] **T8: G3 test — no-regression**
  - All existing L0-L2 tests pass with `self_evolve` off (35 tests)
  - All existing tests pass with `self_evolve` on (55 tests = 35 existing + 20 new)
  - Zero clippy warnings in both configurations

- [x] **T9: G4 test — alloc-free steady state**
  - 100 consecutive heal calls, scratch buffer capacity stable
  - Result: **capacity stable** (8 slots) across 100 calls

- [x] **T10: G5 test — the headline gate**
  - 10-lint fixture with 4 candidates each (1 correct + 3 wrong)
  - Baseline (seeded memory, no evolution): **30.0%** correct
  - After 100 simulated fix attempts with oracle feedback: **100.0%** correct
  - Improvement: **70.0pp** (target ≥5pp) — **PASS by a wide margin**

## GOAT gate

| Gate | What | Target | Status |
|---|---|---|---|
| G1 | Retrieval relevance | top-1 same-lint-family ≥80% | ✅ **100%** (10/10) |
| G2 | Retrieval latency | p50 <50µs @ N=1000 | ✅ **6µs** p50 (release) |
| G3 | No-regression | existing tests green | ✅ 35 off / 55 on |
| G4 | Alloc-free steady state | 0 bytes steady-state | ✅ capacity stable (8 slots) |
| G5 | **Self-evolution gain** | ≥5pp healing rate improvement after 100 attempts | ✅ **70.0pp** (30% → 100%) |

**Promotion decision:** all 5 GOAT gates PASS. The self-evolution mechanism
works — the latent-first EMA direction update + memory retrieval measurably
improves clippy healing rate by 70pp (from 30% to 100% in the controlled
fixture). The gate target was 5pp; the actual improvement is 14× the target.

Per the promotion rule, `self_evolve` is eligible for default-on promotion.
However, the G5 test uses a controlled fixture with oracle feedback (the
test always reports the correct fix as a success). Real-world clippy healing
has noisy feedback (a fix might compile but not actually resolve the lint).
**Decision: keep `self_evolve` opt-in** until validated against real cargo
clippy feedback (Phase 3 `clippy_verify` integration). The GOAT gate proves
the mechanism works; real-world validation is the next step.

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

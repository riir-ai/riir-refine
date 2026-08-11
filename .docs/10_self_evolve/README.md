# 10 — Self-evolve (Issue 004)

> **Purpose:** the latent-first self-evolution loop that composes on top
> of the frozen L0–L2 pipeline. `LatentFixMemory` (3-axis retrieval over
> past fix trajectories) + `LatentSkillEvolution` (EMA direction-vector
> update) + `HealPipeline` (the composition). Opt-in (`self_evolve`
> feature).

## Files

| File | Scope |
|---|---|
| [`heal_pipeline.md`](heal_pipeline.md) | `HealPipeline` + `HealResult` + `HealSource` — the composition |
| [`latent_fix_memory.md`](latent_fix_memory.md) | `LatentFixMemory` + `FixTrajectory` + 3-axis retrieval |
| [`latent_skill_evolution.md`](latent_skill_evolution.md) | `LatentSkillEvolution` + `EvolveConfig` + EMA rule |

## Source

- [`src/self_evolve.rs`](../../src/self_evolve.rs) — `HealPipeline`
- [`src/memory.rs`](../../src/memory.rs) — `LatentFixMemory`, `FixTrajectory`
- [`src/evolve.rs`](../../src/evolve.rs) — `LatentSkillEvolution`, `EvolveConfig`
- [`src/embedder.rs`](../../src/embedder.rs) — BLAKE3 embedding (shared with L1)
- [`tests/goat_gate_004.rs`](../../tests/goat_gate_004.rs) — GOAT gate G1–G5

## Feature gate

`self_evolve` (opt-in). Default-off. The layer is **additive** — it
composes on top of L0–L2 without modifying the frozen pipeline. When no
memory exists, it degenerates to the existing static-corpus path.

## Status

- **Shipped:** commit `bf342f1` (2026-08-11). GOAT G1–G5 ALL PASS.
- **Open:** [Issue 008](../../.issues/008_real_world_clippy_verify_validation.md)
  — real-world validation via `cargo clippy` feedback (G6 corpus
  truthfulness + G7 self-evolution with real clippy). In progress.

## See also

- [`../03_drafter/embedder.md`](../03_drafter/embedder.md) — the BLAKE3 embedder (shared)
- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) — why this is modelless

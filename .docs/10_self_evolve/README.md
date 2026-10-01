# 10 — Self-evolve (Issue 004 / Issue 018)

> **Purpose:** the latent-first self-evolution loop that composes on top
> of the frozen L0–L2 pipeline. `LatentFixMemory` (4-axis retrieval over
> past fix trajectories — latent + BM25 + the Issue 021 evidential-tier
> trust axis) + `LatentSkillEvolution` (EMA direction-vector update) +
> `HealPipeline` (the composition) + `EvolveRecorder` (the standalone
> oracle-split recorder). **DEFAULT-ON + unconditional since Issue 018**
> (graceful oracle degradation — see below).

## Files

| File | Scope |
|---|---|
| [`heal_pipeline.md`](heal_pipeline.md) | `HealPipeline` + `HealResult` + `HealSource` — the composition |
| [`scope_model.md`](scope_model.md) | `ScopeModel` — the input-scope gate at the heal seam (Bench 040 / Plan 016; the Bench 669 consumer adoption) |
| [`latent_fix_memory.md`](latent_fix_memory.md) | `LatentFixMemory` + `FixTrajectory` + `EvidenceTier` (Issue 021) + 4-axis retrieval + dual-backend persistence (store-first Issue 038; JSON envelope Issue 018 fallback) |
| [`latent_skill_evolution.md`](latent_skill_evolution.md) | `LatentSkillEvolution` + `EvolveConfig` + EMA rule |
| [`evolve_recorder.md`](evolve_recorder.md) | `EvolveRecorder` — the oracle-split recorder + the `cargo refine --fix` wiring (Issue 018) |
| [`self_evolve_flow.md`](self_evolve_flow.md) | The e2e self-evolve flywheel diagram (local loop + fleet loop) + the canonical static SVG for the KAT network pages' hero (Issue 131) |
| [`l4_fallback.md`](l4_fallback.md) | The L4 generative fallback seam (`l4_lora`): trait + `DaemonL4` transport + the local Bonsai-27B story + measured verdicts + the serving-lane decision (UNARMED) |
| [`structural_admission.md`](structural_admission.md) | Issue 136 / reef P1+P3+P5: per-kind shape gates + kind-keyed review classes + the two-head CAS commit chain over content-addressed artifacts (`.refine/structural/`) — vocabulary only, no auto-serve activated |

## Source

- [`src/self_evolve.rs`](../../src/self_evolve.rs) — `HealPipeline`
- [`src/memory.rs`](../../src/memory.rs) — `LatentFixMemory`, `FixTrajectory`, envelope persistence
- [`src/traj_store.rs`](../../src/traj_store.rs) — `TrajStore` / `TrajRowDoc` / `LearnRowDoc` (the neuron-db store backend, Issue 038)
- [`src/evolve.rs`](../../src/evolve.rs) — `LatentSkillEvolution`, `EvolveConfig`
- [`src/recorder.rs`](../../src/recorder.rs) — `EvolveRecorder` (Issue 018)
- [`src/embedder.rs`](../../src/embedder.rs) — BLAKE3 embedding (shared with L1)
- [`src/l4_fallback.rs`](../../src/l4_fallback.rs) — `L4Fallback` trait + `NullL4`
- [`src/l4_daemon.rs`](../../src/l4_daemon.rs) — `DaemonL4` transport client
- [`tests/goat_gate_004.rs`](../../tests/goat_gate_004.rs) — GOAT gate G1–G5

## Feature gate

**DEFAULT-ON + unconditional since Issue 018** (Bench 023). The modules
compile in every build (incl. `--no-default-features`); the `self_evolve`
feature remains as a back-compat alias for benches'/examples'
`required-features`. The layer is **additive** — it composes on top of
L0–L2 without modifying the frozen pipeline. When no memory exists, it
degenerates to the existing static-corpus path.

### The oracle split (Issue 018)

- **Memory half — no oracle needed**: every applied fix is a trajectory
  worth storing; reverted fixes carry a definitive fail verdict. Runs in
  EVERY build + persists across `cargo refine` runs (v1 JSON at
  `RIIR_HEAL_MEMORY` or the platform cache dir).
- **Learning half — oracle-gated**: the EMA direction update needs
  trustworthy verdicts. Without the `clippy_verify` oracle it no-ops with
  ONE log line (`self-evolve: memory-only (learning disabled: …)`);
  runtime oracle failure flips it off the same way (memory unaffected).
- **Opt-out**: `RIIR_HEAL_SELF_EVOLVE=0` (`off`/`false`/`no`) — fully
  inert (no recording, no persistence, no log).

See [Bench 023](../../.benchmarks/023_self_evolve_promotion_goat.md) for
the GOAT record + the honest finding that the G7 demo re-run is blocked
pre-existing by corpus drift (Issue 022).

## Status

- **Shipped:** commit `bf342f1` (2026-08-11). GOAT G1–G5 ALL PASS.
- **Validated:** commit `458f1b4` (2026-08-11). G6 (10/10 corpus truthfulness)
  + G7 (50.0pp improvement with real `cargo clippy` feedback) ALL PASS.
  See [Bench 008](../../.benchmarks/008_real_world_validation.md).
- **Feature gate:** stays opt-in — deployment constraint
  (`clippy_verify` is itself opt-in), not a correctness concern.

## See also

- [`../03_drafter/embedder.md`](../03_drafter/embedder.md) — the BLAKE3 embedder (shared)
- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) — why this is modelless
- [`l4_fallback.md`](l4_fallback.md) — the L4 seam + the UNARMED decision
- [Bench 026](../../.benchmarks/026_evidential_status_axis_goat.md) — the Issue 021 evidential-tier GOAT (retelling-bias defense; the honest A/B finding)

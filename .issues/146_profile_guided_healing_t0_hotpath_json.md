# Issue 146 — Profile-Guided Healing T0: the `--hotpath-json` oracle lane POC

**Status:** OPEN (POC; Research 235's T0 — the fusion's first measured gate)

## What

The healer has no view of what is actually hot at runtime (grep-verified: zero dynamic-profile consumption in `src/`). This issue lands the **T0 POC** of the profile-guided healing fusion (`.research/235`): hotpath's JSON report becomes a SECOND oracle lane, mirroring the shipped `--clippy-json` anchor lane — spans at measured-hot sites force-included ahead of similarity ranking, tagged `[hot: avg × calls, alloc bytes]`.

**Zero new deps**: the user runs hotpath (`cargo install hotpath`; `hotpath init --agent ...` automates instrumentation) and exports the report; `cargo refine --hotpath-json <report>` consumes it (serde_json already in). The loop this enables (full anatomy + prior art in `.research/235`): hotness → corpus retrieval (modelless reasoning) → fix → **re-measured verification** → (T1) Elo credit → (T2) iterative loop → (T3, gated) MCTS → (L5) unhealed hot spans become real mining leads.

## Tasks

- [ ] **T0.1 Parse + key**: the report reader (functions/locks/channels/futures sections); key the report to the repo's git SHA (or per-file blake3) and REFUSE on mismatch (the stale-profile hazard — a profile at commit X anchoring spans at commit Y).
- [ ] **T0.2 Anchor**: reuse the clippy-json lane's span-mapping machinery (mind the AstChunker quirk, riir-ai Issue 942); `[hot]` spans ranked by time share (calls × avg) as RANKING signal (B129 governs: instrumented timing is ranking evidence, never a verification target); alloc bytes/count carried as the deterministic verification axis.
- [ ] **T0.3 Guards**: route all profile-guided edits through the bench-target guard (P30 — a hot span in `benches/` is measurement-altering, decline-or-flag); the fix lane must never edit/remove hotpath's own attributes; disclose instrumentation coverage bias (only annotated functions measured).
- [ ] **T0.4 The e2e POC gate**: one toy repo: profile → heal → re-profile → **alloc-delta verified with N≥3 agreeing runs** (threads + HashMap RandomState make counts only nearly deterministic) **+ timing/RSS no-regression guard** (alloc↓ can still mean slower/bigger — over-reserved capacity, stack bloat, RSS-growing caches) **+ the repo's tests pass** (compiling ≠ behaving — perf rewrites can change behavior and compile). GATE: ≥1 real alloc-axis improvement end-to-end.
- [ ] **T0.5 CLI + tests**: the `--hotpath-json` flag (manual feed wins over none; a clean/absent report degrades loudly, the `OracleOutcome` precedent); contract tests at the process boundary (the `repo_posture_cli.rs` hermetic pattern).

## Deferred to later issues (this one is T0 only)

- T1 Elo measured-impact outcomes (one-fix-per-run credit; held-out second workload for Certified; the tier_guard no-free-rider law carries over) — after T0 passes.
- T2 the iterative loop + the **recorded go/no-go interaction test** (greedy vs greedy-with-rollback vs shuffled on cumulative verified gain — perf fixes interact more than lint fixes because each fix moves the hot spot; the decision either way gets written into the issue).
- T3 MCTS over (site × rule) — only on a measured T2 interaction; FIRST task is checking katgpt-core `mcts`'s API fit for a (site×rule) action space (do not assume). Beam/greedy-with-rollback is the rung before.
- L5 local-first mining leads (privacy: user code never enters a shared corpus unless opted in) + T4 KAT fixstats deltas.

## Prior art context (why this shape)

The loop profile→catalog→fix→verify is ESTABLISHED architecture (Google ECO arXiv 2503.15669 OSDI'26 — fleet-scale, LLM-driven, dictionary grown from historical commits; Uber PerfInsights 2025 — LLM jury, curated catalog, hundreds of merged diffs; MOA 2026 — profiling-guided LLM memory optimization). Free-form+LLM fails (PerfBench ACM 2026: 3%). Our position is the **catalog+modelless quadrant** (unoccupied per the sweep) + **L5 corpus growth at currently-hot unhealed spans** (vs ECO's historical commits — the ECO dictionary-growth section read is owed at plan time) + persisted Elo/Beta credit. The plan opens at conversion (the B201 yield holds `.plans/.highwater` on the 4090).

Refs: `.research/235` (the fusion note) · `.research/234` (hotpath distill) · `.research/032` (the deterministic-signal foundation) · `.research/081` (the contention rules).

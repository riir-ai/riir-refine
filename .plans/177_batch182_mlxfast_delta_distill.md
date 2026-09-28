# Plan 177 — Batch 182: mlxfast-bonsai2 delta distill (Research 212 Addendum 3 armed rider)

**Status:** IN FLIGHT (2026-09-28, M3 idle-loop batch slot — the 09-28 cadence slot; B176+B181 landed 09-27)

## TL;DR

Execute the ARMED RIDER from Research 212 Addendum 3 (the 09-27 ~17:4x watch row): delta-distill
`Layr-Labs/mlxfast-bonsai2-27b-engine` scoped `cfb35b0d..HEAD` (B181's pin → current tip `2471492`
at the 09-27 watch; re-verify at clone time). Trigger FIRED: new accepts confirmed (leaderboard
381.9% vs the B181-era 297.5% = +28%, drafter still stock DFlash 10.7 — engine-side arc #3).
The 09-27 watch census: ~26 commits past pin, paths ALL under `Vendor/mlx-swift*`, the arc is
KV-cache/recurrent-state/MTP-verify (LayerCacheV2, FullSequenceKV, RecurrentStateV2, AttentionV1,
EngineLoopV2+MTP*, MTPContractsV2) + NEW model files (PrismHadamardQwen35.swift,
Qwen35A3BOptimization.swift, Qwen35MTPTopTwo.swift) — NOT the Hadamard/quant kernels B181
graded → fresh family ground. Slot adjudication: bonsai/ternary-first priority argues mlxfast
over the staged ggml-cpu tiled lead (`d834d44e6`, stays staged) and the pre-graded P40/P41
fixture pack (stays staged for the next clippy-corpus slot).

## Tasks

- [ ] T0 — Clone at HEAD into `.raw/mlxfast-bonsai2-delta` (ephemeral-clone law: removed at
  close); verify HEAD vs the 09-27 watch's `2471492` (any movement since = widen the census);
  per-commit delta census `git log --name-status cfb35b0d..HEAD` (the watcher's lesson: the
  per-commit log is the authoritative instrument, never a merged-stream path sort).
- [ ] T1 — Dedup grep BEFORE proposing: 3+ vocabulary variants per candidate concept
  (kv-cache/recurrent-state/mtp/spec-verify/lora-adapter/frozen-brain/naive-scan...) against
  `src/domains/kernel_opt/` + `src/domains/rust_perf/`; record drops with reasons.
- [ ] T2 — Spawn 2 parallel subagents over disjoint delta slices to propose rules (pattern,
  bounded fix-space, category, rationale, verbatim source quote); re-verify EVERY quoted line
  against the real file (Batch 31 discipline).
- [ ] T3 — Author corpus entries + extend the standing syn-parse gate
  (`batch182_pseudocode_parses`); route per domain (kernel_opt vs rust_perf; signed-codes/
  kernel-shaped → kernel_opt, host/lane shapes → rust_perf).
- [ ] T4 — GOAT: G1 parse/syn (all new rules) · G2 lib tests pass · G3 no-regression
  (clippy -D clean BOTH feature states) · retrieval re-measure (kernel batch →
  `CARGO_TARGET_DIR=/tmp/riir-clippy-b182 cargo test --features kernel_heal,latent_retrieval
  --lib retrieval` — floors 21/21 must hold; a decay FAILS the batch) · score-bench NOT owed
  (zero clippy-corpus delta, P30) · NO freeze regen (kernel corpus untouched).
- [ ] T5 — Count pins walked: router.rs + per-category table + corpus tests pins + mod.rs doc
  + AGENTS.md (kernel_opt bullet + kernel_heal feature line) + kernel_opt.md (seed row + batch
  paragraph) + README kernel row + corpus_history.
- [ ] T6 — Research note `.research/216_mlxfast_bonsai2_delta_distill.md` (next free number,
  highwater 215 → 216) with mine-time appendix (quotes re-verified at the clone); update
  AGENTS.md batch paragraph + `.distill/001_mining_queue_snapshot.md` (same commit).
- [ ] T7 — Commit `feat: batch 182: ...` + push; plan COMPLETE; `.raw` removed.

## Adjudication record

Slot candidates were (per `.distill/001` 09-27 watch row): mlxfast delta rider (ARMED, trigger
fired) vs ggml-cpu tiled mul_mat `d834d44e6` (staged) vs P40/P41 fixture pack (pre-probed).
**Verdict: mlxfast** — bonsai/ternary-first priority (the snapshot's own note); the tiled lead
and the P40/P41 pack stand for the next slots. GPU exclusivity not required at landing (gates
are CPU-side: parse/lib/clippy/retrieval; no Pareto bench owed for NotBenchable-class kernel
rules per the B176/B180/B181 precedent). Box note: sibling cargo tests (riir-instinct noul
bridge) live — builds may be slow, correctness gates unaffected.

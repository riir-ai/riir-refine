# Research 235 — Profile-Guided Healing: hotpath as the Healer's Dynamic Oracle (hotness → reason → Elo/MCTS → verified fix → corpus growth)

**Status:** FUSION PROPOSAL — novelty RE-PINNED at verdict round 1 against the closest art (Google ECO / Uber PerfInsights / MOA: the loop shape is ESTABLISHED 2025-2026 architecture; our deltas = the modelless quadrant + frontier growth + persisted credit); GOAT-tier integration for the healer surface (fusion priority #2), POC-gated via Issue 146; not Super-GOAT (no new katgpt-rs primitive — it consumes shipped `mcts`/`rating`/`best_belief`)

**Date:** 2026-10-04
**Proposer:** owner, in-conversation ("should we use it to find hot part and reason about it too? not just distill? we even can ELO and maybe MTCS it and fix it? novel fusion?")
**Verdict path:** §5 Claude ping-pong (REVISE r1: ECO/PerfInsights/MOA prior art + 8 hazards — all verified then incorporated; see footer).

## 1. The idea

The healer today fixes what **matches shapes**: clippy diagnostics (the auto-oracle anchor lane), rustc error codes (the driver), or rule-similarity (the retrieval lane). It has **no view of what is actually hot at runtime** — grep-verified: zero dynamic-profile consumption in `src/` (the only "hot" hits are corpus rule content); zero `mcts` consumption (the two hits are score_bench fixture text; katgpt-core ships `mcts` un-consumed by the healer).

The fusion: close the healer's loop on **measured reality** — the third hotpath distill's natural continuation (R032 gave the validator its deterministic alloc axis; R081/B65 gave the contention rules; R234 priced the observer itself; this note makes the profiler an input to the loop rather than only a mining source).

```
user workload --(hotpath report: JSON)--> [L1 hotness oracle: anchor spans]
   --> [L2 reason: corpus retrieval on hot spans, modelless-first]
   --> [L3 select+learn: Elo outcomes weighted by measured delta (self_evolve)]
   --> [L4 sequential: re-profile loop; MCTS over (site × rule) iff interaction measured]
   --> [L5 THE MOAT: unhealed hot spots = REAL corpus gaps → frontier miner leads]
```

**Honest framing (post-round-1): L1–L3 are the ECO/PerfInsights architecture, reimplemented modelless.** The loop shape — profile finds costly functions → match against an anti-pattern catalog → propose edits → verify → measure the saving — is established, deployed art at fleet scale (ECO: warehouse-scale savings; PerfInsights: hundreds of merged Go diffs). What this note claims is narrower (§3.5).

## 2. The five layers (what exists vs what is new)

| Layer | Consumes (ships today) | New |
|---|---|---|
| **L1 hotness oracle** | the `--clippy-json`/auto-oracle anchor lane (`src/clippy_oracle.rs` — force-include spans at real diagnostics ahead of similarity ranking); hotpath's JSON report (functions timing/allocs, lock wait/hold, channel depth, poll stats) | `cargo refine --hotpath-json <report>` — spans at measured hot sites anchored + tagged `[hot: avg × calls, alloc bytes]`. **ZERO new deps** (serde_json already in; the user runs hotpath — its `hotpath init --agent` flow automates instrumentation). Ranking key: time share (calls × avg) **ranking-only**; alloc bytes/count **deterministic** (R032). Report keyed to a git SHA (or per-file blake3); REFUSED on mismatch (the stale-profile hazard) |
| **L2 reason** | the rust_perf corpus + retrieval + `context_notes` regime matching | regime match gains a profile context (hot loop? contention-bound? alloc-heavy? poll-heavy?); the corpus IS the reasoning, modelless-first; the LLM advisory only at the frontier (§L5) — never in the fix/credit path |
| **L3 Elo on measured outcomes** | `self_evolve.rs` + `traj_store.rs` + `elo.rs` (rule↔span Elo, Beta posterior, Certified/Heuristic tiers, the tier_guard likelihood-ratio gate) | outcome rows gain a **measured-impact dimension**: a kept fix that also improves the re-profiled metric records a weighted win. CREDIT DISCIPLINE: one fix per re-profile run (or factorial design); alloc-delta needs N≥3 agreeing runs (threads + HashMap RandomState make counts only NEARLY deterministic) PLUS a timing/RSS no-regression guard (alloc↓ can still mean slower/bigger); a Certified credit needs a held-out second workload |
| **L4 sequential decisions** | `nonergodic_driver` (a learned ordering = a bandit over strategies); katgpt-core `mcts` (shipped, un-consumed) | phase-gated with a RECORDED go/no-go test (Issue 146 T2): greedy vs greedy-with-rollback vs shuffled order on cumulative verified gain — perf fixes interact MORE than lint fixes (each fix moves the hot spot), so the test is expected to fire; ladder = greedy → beam / greedy-with-rollback → MCTS over (site × rule) **only on a measured interaction**. T3 must first check katgpt-core `mcts`'s API actually fits a (site×rule) action space — do not assume |
| **L5 corpus growth at real hot spots** | frontier miner (synthetic fixtures the corpus cannot repair); coverage-report (clippy's derived surface) | anchored hot spans with NO corpus rule = real-world mining leads (`[no corpus rule]` at a measured-hot site = a product-grade gap signal); feeds the frontier miner + `--misses` queues with present reality, not only self-play. **LOCAL-FIRST: leads stay on the user's machine unless opted in** (the privacy/licensing boundary — user code must never leak into a shared/published corpus by default) |

## 3. Prior art (§4 sweep + verdict-round-1 additions, 2026-10-04)

### 3.1 The closest art (added at round 1 — the note's first sweep MISSED these)

| Prior art | What it does | Delta vs this fusion |
|---|---|---|
| **Google ECO** (arXiv 2503.15669, OSDI '26) — "An LLM-Driven Efficient Code Optimizer" | fleet profiling localizes costly functions → matches against a **dictionary of performance anti-patterns mined from historical commits** → LLM generates edits → verified → savings measured in production | L1 + bounded catalog + measured credit = ECO's architecture. Deltas: ECO's dictionary grows from PAST COMMITS (ours from CURRENT unhealed hot spans — L5; **the ECO dictionary-growth section read is owed at plan time to confirm**); ECO is LLM-driven (ours modelless end-to-end); ECO is fleet-scale (ours is one repo on a workstation) |
| **Uber PerfInsights** (2025) | production profiles find hot Go functions → full source + a curated anti-pattern list → **LLM judges** → hundreds of merged diffs | same L1 shape, LLM in the judging seat; curated (static) catalog vs our growing corpus; fleet vs workstation |
| **MOA** (arXiv, Jun 2026) — "A Profiling-Guided LLM Framework for Memory" | profiling-guided LLM memory optimization | same family, memory axis, LLM-driven |

### 3.2 The wider art

| Prior art | What it does | Why it does not kill this |
|---|---|---|
| **PGO** (LLVM/GCC/BOLT) | profile → compiler-internal optimization decisions | IR-level, compiler-internal; no source rewrites, no learning corpus |
| **CompilerGym / ECCO (2026-01) / WasmRL** | RL/MCTS over compiler **pass ordering** with measured reward | validates sequential-transform+measured-reward (the L4 shape) — action space is compiler passes, not bounded source-fix rules; no growing catalog |
| **PerfBench** (ACM 2026) | free-form agents on real perf bugs: **3% success** (OpenHands/GPT-4.1) | the free-form+LLM quadrant's failure datum |
| **ACCLAIM** | iterative agent loop with compiler feedback | static feedback, free-form edits |
| **RepairAgent / HyperAgent / APR lineage** | autonomous agents for functional repair | not perf/profile-driven |

### 3.3 The 2×2 (the re-pinned position)

| | **LLM in the loop** | **Modelless** |
|---|---|---|
| **Bounded catalog of fix patterns** | ECO ✓ · PerfInsights ✓ · MOA ✓ (deployed, wins at scale) | **THIS FUSION — unoccupied per the sweep** |
| **Free-form edits** | PerfBench ✗ (3%) | — (nonsensical) |

**The re-pinned claim (§1.5 form):** *a profile-guided perf healer occupying the catalog+modelless quadrant — bounded rule-corpus actions, compile + TEST gates, deterministic alloc-delta verification with N≥3 agreement, no-regression timing/RSS guards, and Elo/Beta credit persisted per rule↔site — distinguished from ECO/PerfInsights (catalog+LLM) by the absence of any LLM in the fix/credit path, from PerfBench (free-form+LLM) by bounded actions, and additionally by L5 (the corpus grows at currently-hot unhealed spans rather than historical commits — ECO's dictionary-growth section to be read at plan time) and by persisted credit ratings feeding an economy (impact-verified fixes as a network trust signal, T4).* The quadrant claim is sweep-limited (web + workspace); the plan-time re-check is owed — including the CLASSIC non-LLM anti-pattern lineage the r2 reviewer named (Toddler, Caramel, Coccinelle performance patches, clippy's `perf` group — none profile-anchored or corpus-growing per their shapes, but the plan says so explicitly).

## 4. Hazards (r1 additions incorporated — the weak axes, named upfront)

1. **Compositional novelty, re-scoped.** L1–L3 are established architecture (§3.1). The load-bearing claims are the quadrant position + L5 + persisted credit. The ECO dictionary-growth read is the one prior-art check that could narrow L5.
2. **Q2 unmeasured.** "The healer fixes real hot paths" is asserted-by-design until T0 runs. Hence issue-first, no plan.
3. **The observer perturbs.** hotpath costs ~40ns/call (its own docs) and sampling distorts timing — **B129 (`instrumented-attribution-not-production-times`) + R234's own C1/C2 rules govern**: timing ranks, allocs verify; pure-compute workloads (no alloc signal) fall back to ranking + score-bench microbenches.
4. **The alloc reward can be gamed.** Fewer allocations can still mean slower/bigger (over-reserved `with_capacity`, stack bloat, RSS-growing caches) → the no-regression timing/RSS guard beside the alloc axis, box state recorded.
5. **Compiling ≠ behaving.** `--verify` proves compilation only; perf rewrites (dropped clones, iterator changes, drop/evaluation-order changes) can alter behavior and compile → profile-guided edits carry a TEST-SUITE gate (the repo's own tests; decline-or-flag when absent).
6. **Instrumentation coverage bias.** hotpath sees only annotated functions — unannotated code never reads hot; the report's coverage is disclosed, never treated as exhaustive. The healer must never edit/remove hotpath's own attributes.
7. **Stale profiles.** A profile at commit X applied at commit Y anchors wrong spans → report keyed to git SHA (or per-file blake3), refused on mismatch.
8. **Credit attribution.** Several fixes in one re-profile run cannot attribute the gain → one fix per credit run (or factorial design).
9. **Workload overfitting.** One profiled workload is a gameable benchmark → Certified credits need a held-out second workload.
10. **Near-determinism.** Alloc counts vary with threads + HashMap RandomState → N≥3 runs must agree before a delta counts as deterministic.
11. **Privacy/licensing.** L5 leads from user repos must stay LOCAL unless opted in (no leak into shared/published corpora — the KAT/public-dist boundary).
12. **MCTS may not fit.** katgpt-core `mcts`'s API fit for a (site×rule) space is unverified — T3's first task. Shipping MCTS over independent fixes would be architecture theater.
13. **Span mapping** reuses the clippy-json lane machinery (the AstChunker quirk, riir-ai Issue 942, applies). **Bench files**: hot spans in `benches/` are measurement-altering — the P30 bench-guard routes profile-guided edits.

## 5. Staging (POC-first; `.plans/` deferred under the B201 yield — the plan opens at conversion, the PRISMML precedent)

- **T0 (Issue 146)**: parse the hotpath JSON report; SHA-key it; anchor `[hot]` spans through the existing feed lane; ONE e2e on a toy repo: profile → heal → **alloc-delta verified (N≥3 agreeing) + timing/RSS no-regression + tests pass**. Gate: ≥1 real alloc-axis improvement end-to-end, zero new deps, bench-guard respected.
- **T1**: self_evolve outcome rows gain the measured-impact dimension (one-fix-per-run credit; held-out workload for Certified; tier_guard carries over).
- **T2**: the iterative loop (fix top-k → re-profile → repeat) + the **recorded go/no-go interaction test** (greedy vs greedy-with-rollback vs shuffled, cumulative verified gain) — the decision, either way, is written into the issue.
- **T3**: katgpt-core `mcts` API-fit check, then MCTS over (site × rule) **only if T2 measured interaction** (beam/greedy-with-rollback is the rung before).
- **T4 (optional)**: KAT fixstats carry measured deltas (impact-verified fixes as a network trust signal; opt-in); frontier-miner real-profile leads productized (L5, local-first).

## 6. Fusion kin within the workspace (force-multiplier axis)

Oracle lanes (`clippy_oracle.rs` pattern) · rust_perf corpus + Pareto/alloc-axis validator (R032's child) · `self_evolve`/`traj_store`/`elo` (L3 substrate, shipped) · `nonergodic_driver` (the bandit precedent) · frontier miner + coverage-report (L5 consumers) · katgpt-core `mcts` + `rating` (L4 substrate) · KAT fixstats (T4). Six+ existing pillars connect.

## §5 Verdict ping-pong

Round 1 (2026-10-04): **REVISE** — the reviewer surfaced ECO (arXiv 2503.15669) / PerfInsights / (search-verified additionally: MOA) as the closest art the first sweep missed, endorsed GOAT-not-Super-GOAT + the MCTS phase gate, and added 8 hazards. All incorporated above (novelty re-pinned to the 2×2 quadrant + L5 + persisted credit; hazards §4.4–4.11; the T2 go/no-go test; the T3 API-fit check). Rounds continue to AGREE before commit.

# Research 053: EvolveMem — Co-Evolving the Retrieval Configuration of LatentFixMemory

> **Source:** "EvolveMem: Self-Evolving Memory Architecture via AutoResearch for LLM Agents" — [arXiv:2605.13941](https://arxiv.org/abs/2605.13941) (Liu et al., 2026-05-13, cs.LG; code: aiming-lab/SimpleMem)
**Date:** 2026-08-16
**Status:** Distilled — actionable delta identified; no implementation yet (owner decides filing; no new .issue per queue-item rule)
> **Related Research:** R001 (self-evolving agents survey → LatentFixMemory origin), R049 (evidential status → Issue 021 trust axis)
> **Related Benchmarks:** Bench 022 (retrieval precision at scale — the eval), Bench 025 (router spectral POC KILL), Bench 024 (floor re-pin)
> **Classification:** Private

---

## TL;DR

EvolveMem's thesis: existing agent-memory systems evolve the **stored content** but freeze the **retrieval mechanism** (scoring functions, fusion weights) at deployment; truly adaptive memory must **co-evolve both levels**. Their LLM-powered diagnosis module reads per-question failure logs each round, proposes targeted retrieval-config changes, and a guarded meta-analyzer applies them with **revert-on-regression** and **explore-on-stagnation** safeguards. +25.7%/+18.9% relative over the strongest baselines on LoCoMo/MemBench; evolved configs transfer positively across benchmarks.

**Distilled for riir-clippy:** this names exactly what our `self_evolve` line does NOT evolve. `LatentFixMemory` content evolves (trajectories accumulate, tiers advance, EMA updates drafter directions) but `RetrievalWeights { latent: 0.4, bm25: 0.6, trust: 0.2 }` is a **hand-reasoned constant** (src/memory.rs L246-257) — the comment argues its margins by arithmetic, never by measurement. Meanwhile Bench 022 already measured the consequence: clippy top-1 = 54.2% with nearest-neighbor crowding concentrated in tiny-token patterns. The measurement instrument (retrieval_eval, 130 spans + miss taxonomy + committed regression floors) exists; the optimization loop over it does not.

---

## 1. Paper core findings (what's load-bearing for us)

1. **Two-level evolution.** Content-level (what is stored) vs configuration-level (how it is scored/fused/retrieved). Freezing level 2 caps the benefit of level 1 — a better memory cannot be found by a frozen retriever.
2. **Failure-log-driven diagnosis.** Each evolution round reads per-question failure logs, clusters root causes, and proposes a *targeted* config change — not a grid sweep. This is why it beats AutoRAG-style exhaustive HPO (arXiv:2410.20878) in their comparison.
3. **Two guards, both cheap.** *Revert-on-regression*: a proposed change that drops eval metrics is rolled back automatically. *Explore-on-stagnation*: no improvement for N rounds → deliberately perturb (explore) instead of hill-climbing forever.
4. **Evolution discovers new dimensions.** Starting from a minimal action space, the diagnosis module introduced config axes not present initially — retrieval strategy space is not known a priori.
5. **Positive cross-benchmark transfer** — evolved configs capture general retrieval principles, not benchmark heuristics.

## 2. What already ships here (external validation + the gap)

| EvolveMem level | riir-clippy state |
|---|---|
| Content evolution | ✅ `LatentFixMemory` trajectories + `EvidenceTier` transitions (Issue 021) + `EvolveRecorder` persistence (Issue 018, default-on) |
| Skill evolution | ✅ `LatentSkillEvolution` EMA over drafter direction vectors |
| Retrieval-config evolution | ❌ `RetrievalWeights` hardcoded; tunable via `LatentFixMemory::new(weights)` but **no caller ever tunes** — Stage 2 tuning loop admits *content* (verified-fix pool), never *weights* |
| Failure logs | ✅ untapped: `retrieval_eval.rs` miss taxonomy (Issue 017) + per-trajectory outcome stats sit right there |
| Revert-on-regression guard | ✅ latent: the committed regression floors (`retrieval_precision_regression_gate`) ARE a revert trigger — currently used only to fail CI, not to drive search |
| Explore-on-stagnation | ❌ nothing |

## 3. Distilled delta (the modelless translation)

Replace EvolveMem's **LLM diagnosis module** with the **miss taxonomy** — the taxonomy is already a root-cause classifier (crowding / wrong-domain / tie-order), which is precisely what their diagnosis module produces from raw failure logs:

1. **Guarded coordinate descent over `RetrievalWeights`** as a bench/CLI step (not hot path): propose one-axis perturbation → run `retrieval_eval` → accept iff all committed floors hold (revert-on-regression for free — the floors are already tests) → on N stagnant rounds, epsilon-perturb all axes (explore-on-stagnation). Deterministic seed → G1-safe. ~100-150 LOC.
2. **New config dimension (their finding #4, our measured pain):** a code-token lexical axis. Our "BM25" axis is exact `lint_name` hash match only (memory.rs L12-16 explicitly defers token-level BM25 as "marginal POC gain"). Bench 022's crowding misses (e.g. `identity_op` topping 5 misses) are exactly the class a cheap token-overlap axis breaks — nearest neighbors in 8-D latent space that share no tokens with the query.
3. **Weights as persisted config, not constants** — the tuned result serializes next to `memory.json` (same envelope pattern as Issue 018), so the evolved config survives process restarts.

## 4. Fusion (paper × LatentFixMemory × retrieval_eval)

None of the three alone produces it: EvolveMem needs an LLM to diagnose; our eval measures but never optimizes; our memory evolves content under a frozen retriever. Fused: a **modelless closed loop** — taxonomy-driven proposals, eval-scored acceptance, floor-gated reverts — i.e. AutoResearch on our own retrieval config with zero model calls. The eval's per-domain breakdown also gives EvolveMem's cross-benchmark-transfer claim a cheap analogue: tune on clippy+perf, verify kernel floors hold (transfer check), never accept a per-domain win that regresses the blend.

## 5. Prior art + honest verdict

- Technique family is established: AutoRAG (arXiv:2410.20878) grid-sweeps RAG pipeline configs; AutoRAG-HP (EMNLP-F 2024) does online HPO for RAG. EvolveMem's contribution is the memory-agent framing + diagnosis-driven (not exhaustive) search + the two guards. Our delta on top: **modelless diagnosis** (taxonomy vs LLM) + **eval-floors-as-revert-guard** (tests as the meta-analyzer).
- In-repo prior art checked: R001 covers memory+skill evolution (not retrieval config); R049/Issue 021 covers the trust axis (one weight, hand-set); Issue 020 was *routing* (which domain), killed for gating — this is *fusion* (how domains' axes combine), which Issue 020 explicitly left as "a future tuning option" in its salvage note.
- **Verdict: Gain** — actionable, modelless, in-repo (~150 LOC + one new axis), with the fitness function and revert guard already shipped as tests. Expected payoff is bounded and honest: top-1 recall headroom on clippy is measurable (54.2% baseline), not guaranteed — the loop may confirm the hand-set weights are already near-optimal, which would itself be a finding worth one line in Bench 022.
- **No Super-GOAT claim.** This is a tuning loop over an existing substrate, not a new capability class.

## 6. GOAT gate sketch (if implemented)

| Gate | What | Target |
|---|---|---|
| G1 | Determinism — same seed → same tuned weights | bit-identical |
| G2 | Tuner wall-clock (one round = 1 eval run) | < 60 s/round (eval is ~54 s kernel-inclusive) |
| G3 | No-regression — all committed floors hold | floors green |
| G5 | Headline — tuned vs default on held-out spans | ≥ +3pp top-1 on at least one domain, blend non-regressing; OR documented confirmation that defaults are near-optimal |

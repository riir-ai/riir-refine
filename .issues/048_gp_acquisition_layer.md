# Issue 048: GP acquisition layer — input-space uncertainty ordering on the healer's shipped seams

**Status:** OPEN — 0 GPU-h, M3-local, ready to run (pending owner go)
**Date:** 2026-08-27
**Source:** [arXiv:2606.08802](https://arxiv.org/abs/2606.08802) (ActFlow) — Eq 10 linear-kernel GP posterior variance as the active-learning acquisition signal; distilled in [katgpt-rs R510](../../katgpt-rs/.research/510_ActFlow_Certified_Frontier_Expansion.md) (adversarial-panel item R4). Sibling tracks: [riir-train Plan 357](../../riir-train/.plans/357_actflow_discrete_expansion.md) (weight-space), [katgpt-rs Plan 580](../../katgpt-rs/.plans/580_certified_frontier_primitive.md) (public primitive).

## Finding

The healer's candidate selection is **count-based per candidate** (`select_best_candidate`, `SelectionMode::BetaPosterior` default — bars 0.9240/0.9379) and its rustc-error repair ordering is a static min-delta (`CompileDriver.ordering: Option<OrderingFn>` — a shipped, swappable seam built for exactly this). Neither models **input-space epistemic uncertainty**: which (rule, span) pair to QUERY next, given the 8-D `ModellessEmbedder` span embeddings + the 8,000+ verdict-bearing trajectories the store already holds (P5 closed 2026-08-24 — outcome verdicts wired end-to-end).

ActFlow's acquisition — GP posterior variance over a frozen representation, `σ²(x) = k(x,x) − k(x,X)(K+λI)⁻¹k(X,x)`, linear kernel = dot product — is exactly that signal, closed-form, ~100–150 LOC, zero new deps.

**This is a different axis than the two measured selection negatives** (Issue 026 Entropic, Bench 035; Issue 033 TetherBlend, Bench 042): those modified pool *scoring* under starved per-candidate evidence (0–3 observations/candidate) and lost. The GP layer modifies *acquisition ordering* over input-space embeddings; at the rule level the store is NOT starved (8K+ observations). Per-candidate Beta selection and input-space GP ordering compose (select WHICH candidate for a rule via Beta; select WHICH span/rule pair to attempt next via GP).

## Tasks

- [ ] **T1** `src/draft/gp_acquisition.rs`: linear-kernel GP posterior variance over `ModellessEmbedder` 8-D vectors + incremental rank-1 Cholesky append (borrow from katgpt-core Plan 580 when it lands; local until then). Unit: pinned against dense solve at N=64 (1e-6).
- [ ] **T2** Ordering fn impl for the shipped `CompileDriver.ordering` seam (`rustc_errors/driver.rs`): order candidate repairs by posterior variance of their span embeddings (high-uncertainty first) vs incumbent static min-delta.
- [ ] **T3** `select_best_candidate` composition mode: acquisition hint = GP σ over the span embedding, blended as a TIE-BREAK on BetaPosterior (lexicographic, the ruliology evidence-axis precedent — never overriding payoff).
- [ ] **T4** A/B gate (the Issue-026 harness, reusable verbatim): GP-ordered vs BetaPosterior-ordered vs random, on the frozen 34-fixture score-bench + a real-repo dry `--fix` pass; 3 runs bit-identical; measure heal-rate + **discovery-within-budget** (rules attempted per N spans). PASS bar: ≥ BetaPosterior on heal (0.9240 floor) with discovery strictly greater; never-wrong axis: `created_rate` ≤ 1.9%.
- [ ] **T5** Honest negative arm: if the GP's uncertainty surface is flat over the fixture embeddings (8-D may be too coarse to separate spans), record NEGATIVE + keep the ordering seam documented — flatness is a measurable outcome, not a failure to hide.

## Why 0 GPU-h / M3-local

GP fit + score over 8K×8-D rows: minutes on CPU. Score-bench runs: seconds. No 4090 involvement; no GPU-exclusivity constraint.

## Risk register

| Risk | Mitigation |
|---|---|
| 8-D embeddings too coarse → flat σ surface | T5 negative arm pre-registered; escalation path = richer features (BM25 TF-IDF dims) before any model change |
| Starved evidence at the SPAN level (new spans unseen) | σ(x) for an unseen span is maximal by construction (k(x,X) small → σ→k(x,x)) — that IS the exploration signal; no counts needed |
| Perturbs the BetaPosterior default | T3 is lexicographic tie-break only; default mode unchanged until T4 PASSes; feature-flag the blend |

## Record on close

Update katgpt-rs R510 §Routing with the verdict + commit hash; remove this file per the noise-reduction rule.

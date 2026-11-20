# Research 426: Epiplexity-Guided Mining Allocation — the Rulewright's Structural-Gain Queue (Guide)

**Source**: katgpt-rs Research 685 (arXiv:2608.11746, EpiSelect/EpiGen) + the shipped substrate (`katgpt-pruners::epiplexity`, Plan 130 / Bench 041)
**Date**: 2026-11-20
**Verdict**: Guide — consumer-first moat investment (fusion priority #1). The mining queue is the workspace's most direct marginal-gain allocation surface.

## TL;DR — commercial value

The rulewright mines rule batches from external repos one domain at a time; today the order is queue-history and hand judgment. This guide pins the upgrade: **order the mining queue by predicted marginal structural gain — headroom × exposure × influence — fitted from our own append-only score history, and stop mining domains whose curve has hit its floor.** Commercial claim: the fixer's quality-per-mining-hour rises because every batch lands where the corpus still learns, and saturated domains stop consuming mining budget. The signal is fully modelless (closed-form derivative over a GD-free fit of logged accuracy curves) and reuses an existing katgpt primitive family (post-hoc estimator + default-on bandit-context heuristic in-stack; this filing gives the family its first consumer outside katgpt-rs) instead of new inference cost.

## The distilled modelless primitive

Per domain k, maintain the resolution curve: score-bench (or held-out-fix) accuracy vs corpus spans consumed from k. Fit `L̂_m = ε_m + β_m·(Σ_k γ_m,k·n_k)^(−α_m)` GD-free (grid + coordinate descent over the γ simplex; log-space; refit at each batch landing). Marginal gain of the next k-batch:

```
gain_k = Σ_m [ n_m·α_m·γ_m,k / (Σ_i γ_m,i·n_i) ] · (L̂_m − ε_m)
```

decomposed as **headroom (L̂_m−ε_m) × exposure (n_m/effective count) × influence (γ_m,k)**. Selection weights are **sigmoid-tempered** (house law, never softmax): `w_k = σ((gain_k − median gain)/τ_s)`, normalized; σ saturation replaces the paper's clip; momentum stays as hysteresis. ε-floor per domain = the saturation gate — "this domain is done, stop mining it" (the paper's Pile-saturation law as a queue gate: if one domain dominates the area integral, skip allocation entirely).

## Connection map

| Existing surface | Consumed as |
|---|---|
| `score_history.jsonl` (append-only, per corpus generation) | the accuracy curves the fits consume |
| `.distill/001_mining_queue_snapshot.md` (queue order) | the allocation decision this signal re-orders |
| Elo + `elo_k_eff` + active-set demote | complementary per-rule OUTCOME layer — judges landed rules; headroom orders the NEXT batch (different decision/signal/granularity; no replacement) |
| `coverage_gap_event` | coverage axis — orthogonal to learnable-structure gain; a fully-covered domain can still be saturated |
| `katgpt-core` (the allowed dep) | home of the new fit+derivative primitive (katgpt-rs Plan 641); **consume, never fork** (goat-audit direction) |
| Corpus-admission hygiene | the paper's two collapse modes become planted refusal arms: a repetitive batch (n-gram rep ≥ ~0.4-class) and an off-manifold batch must be refused by similarity×novelty admission — PPL-only admission is a documented negative import (repetition collapse) |

## Latent↔raw + boundary

The signal is a per-domain SCALAR computed from logged curves — no embeddings, no latent state crosses anything. If scores are ever compared cross-box, sequential-fold order pins per the determinism law (katgpt-rs Issue 940). **Stationary-surface law:** allocation surfaces only — the signal NEVER routes the scan/decide path (`pick_domain`, rerank): those are pure functions of the query with no step size (the ESC Issue-047 stationary trap). Warm-start gate: mining-gain signals arm only on an already-rated corpus (Elo floor).

## Private vs open

Open: the fit + derivative + sigmoid weights (katgpt-core, Plan 641). Private (this repo): the mining-queue consumer, the corpus-admission gates, the saturation gate, and every threshold table — rulewright-quality surface. The guide itself is private; the primitive note (katgpt-rs 685) is public.

## Validation protocol

1. **Census first (blocking):** does score_history carry enough mix variation for the cross-domain γ fit (effective rank ≥ K−1)? Under floor → typed Sparse verdict + univariate fallback (headroom×exposure survives; influence does not).
2. **G1 recovery + G1b identifiability + saturation tripwire** — pre-registered in katgpt-rs Plan 641 (consume that gate, don't re-derive).
3. **G2 allocation value (this repo's bar):** resolutions per mining-hour, gain-ordered queue vs incumbent order over ≥2 corpus generations, paired, one frozen read, LB95 > 0. **FAIL ⇒ demote in-landing** (Research 591 precedent).
4. **G3 feature-off byte-identity; G4 O(K·W) zero-alloc steady state, refit bounded by the paper's ~12 s-class fit budget.**
5. **Collapse canaries** in the admission gate (repetitive + off-manifold planted arms).

## Priority

- **P0** — mix-variation census on score_history (decides the fit's shape; hours, CPU-only).
- **P1** — katgpt-core primitive (Plan 641) once P0 picks the fit family.
- **P2** — queue-ordering consumer + saturation gate (Plan 392) + G2 cell.
- **P3** — admission-gate canaries; cross-domain γ (only if P0 supports it).

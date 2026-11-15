# Issue 239: Conditional-geometry diagnostic axes for the rulewright corpus (Research 675 consumer)

**Date:** 2026-11-15
**Status:** Open — unclaimed. **2026-11-16 upstream verdict (Plan 638 T2.7, delegated 2 rounds AGREE): DEMOTE — axis (i)'s value premise is MEASURED-NEGATIVE.** The primitives landed (`katgpt-rs` `3eb1cb713`, estimators-gated, opt-in), but at the genuinely-curved decision arm both the cond_id two-ball form AND the spectral incumbents fail (incumbents mis-order PR 7.53→11.11; cond_id d̂_B 8.86 vs the 4-plant at n=128) — no complementary signal on any shape measured. Axis (i) as written ("a cheap corpus-health gate in the spirit of geometric-features-recover-class") has no working estimator behind it at the measured scales; the axis re-opens only under Plan 638's 4 recorded triggers (a labeled real shape / the G2 bar / a consumer need the incumbents miss / a larger-n ω=3 rerun). Axis (ii) was always gated on a richer embedding and is untouched by this verdict.
**Source:** katgpt-rs Research 675 ([arXiv:2608.25166](https://arxiv.org/abs/2608.25166) distill) + Plan 638 (the `cond_id`/`cond_imbalance` primitives, katgpt-core `estimators`-gated). Verdict: GOAT, pending Plan 638 Phase 2 gates.

Two consumer axes for the rulewright surface. Both consume the Plan 638 primitives; neither gates on them until the plan's G1–G5 gates pass.

## Axis (i) — geometry-only domain-probe corpus audit (cond-ID ONLY)

Fit a linear/logistic probe on geometry-only features of rule embeddings vs their domain labels (clippy_lints / rust_perf / kernel_opt / sec / docker / …), over `assets/rule_embed_frozen_v1.bin` (frozen, BLAKE3-sealed — read-only). If geometric structure alone recovers the domain labels well above a shuffled-label control, the embedding is category-structured — a cheap corpus-health gate in the spirit of Research 394's "geometric features recover class" result transplanted to the rule corpus; a near-chance probe is an embedding-quality red that precedes any retrieval-quality complaint.

**Scope pin (verdict amendment):** this axis is **cond-ID only**. The frozen embed is a single space — the information-imbalance measure has no consumer here until a second space exists (e.g. two embedding generations side by side, at which point `cond_imbalance` measures how much of the old corpus's neighborhood structure the new embedding preserves — a lease/overlay regression read).

**Proof path:** fixture = the frozen embed + the domain labels already carried by the corpus; falsification arm = shuffled-label control must score near chance; pre-registered bar to be pinned at claim time (suggest: probe ROC-AUC ≥ shuffled + 0.15 = structured; within 0.03 of shuffled = saturated-red).

## Axis (ii) — batch_diversity ID-ratio rider (GATED on the richer embedding)

Rider on Issue 222 (batch_diversity) axis (a): once a richer rule embedding exists, replace/augment the div/corpus_div spread ratio with a per-batch conditional-ID ratio (batch `cond_id` vs corpus `cond_id` over the same population). Motivation: Issue 222 measured the hashed 8-D embedding saturating spread at corpus granularity (ratios 0.994–1.007); an ID ratio is a different functional (local neighborhood dimensionality, not global spread) and may discriminate where variance ratios saturate.

**Honest caveat (carried verbatim from Research 675):** the ID diagnostic INHERITS embedding saturation — it is a richer functional of the future richer embedding, not a fix for its absence. Do not run it on the 8-D hashed embed and expect separation.

## Routing

- Primitives land katgpt-rs-side (Plan 638); this issue is the refine-side consumer record.
- Natural claimant: any riir-refine corpus/retrieval session; axis (i) is actionable on the existing frozen embed once Plan 638 Phase 2 passes.

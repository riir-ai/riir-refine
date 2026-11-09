# Issue 222: Rule-Mining Batch-Intake Diversity Arm (Diffusion-Diversity Div)

**Date:** 2026-11-09
**Status:** Open
**Origin:** katgpt-rs Research 660 ([arXiv:2606.01861](https://arxiv.org/abs/2606.01861)) — self-play curriculum-diversity distillation. Primitive: katgpt-rs Plan 634; siblings: riir-ai Plan 646, reflex Issue 115, riir-train Issue 680.

## Problem

L0 mining batches grow the rule corpora source-by-source with per-item dedup gates. The failure mode this does not measure: batches whose rules pile into one code-shape cluster (the paper's cluster-local curriculum failure — many rules, one fundamental idea). Corpus spread across shape-space is unmeasured; retrieval floors are per-lane, not per-cluster.

## Signal-diff vs shipped cousins (why dedup does not cover it)

Dedup kills per-item near-duplicates (pairwise, 1-hop). The paper's `Div_P^τ` is distribution-level: a batch of pairwise-distinct rules that all diffuse into the same cluster still scores low Div — exactly the case pairwise gates pass. `set_admission`'s log-det/Vendi machinery (Plan 599) is set-level in a fixed cosine kernel without graph diffusion — the nearest shipped shape, still no multi-hop structure.

## Task

- [ ] **T1** After katgpt-rs Plan 634 Phase 1 lands: REPORT-ONLY `Div_P^τ` readout per mined batch intake over the rule-embedding kNN graph, printed beside the existing batch-intake gate lines (the score-bench/provenance strata precedent — report lines, no gate changes).
- [ ] **T2** Correlation cell: batch Div vs downstream score-bench heal-rate deltas across batches; pre-register the promotion question; only a measured correlation promotes T3.
- [ ] **T3** `- [-]` DEFERRED (promotion-gated): a batch-intake diversity floor or intake weighting. Never wires without T2 evidence + the primitive's GOAT pass + the `latent_retrieval` graph build cost measured (the kernel_route_narrow cost lesson applies).

## Gates

- Report-only T1 changes no exit codes; opt-in flag; feature-off byte-identity.
- Any weighting (T3) is sigmoid-compatible, never softmax.

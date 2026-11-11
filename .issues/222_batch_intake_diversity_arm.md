# Issue 222: Rule-Mining Batch-Intake Diversity Arm (Diffusion-Diversity Div)

**Date:** 2026-11-09
**Status:** Open — T1 claimed 2026-11-11
**Claimed:** amo-idle-i222-t1, 1794451200, expires 2026-11-13T10:00:00+00:00
**Origin:** katgpt-rs Research 660 ([arXiv:2606.01861](https://arxiv.org/abs/2606.01861)) — self-play curriculum-diversity distillation. Primitive: katgpt-rs Plan 634; siblings: riir-ai Plan 646, reflex Issue 115, riir-train Issue 680.

## Problem

L0 mining batches grow the rule corpora source-by-source with per-item dedup gates. The failure mode this does not measure: batches whose rules pile into one code-shape cluster (the paper's cluster-local curriculum failure — many rules, one fundamental idea). Corpus spread across shape-space is unmeasured; retrieval floors are per-lane, not per-cluster.

## Signal-diff vs shipped cousins (why dedup does not cover it)

Dedup kills per-item near-duplicates (pairwise, 1-hop). The paper's `Div_P^τ` is distribution-level: a batch of pairwise-distinct rules that all diffuse into the same cluster still scores low Div — exactly the case pairwise gates pass. `set_admission`'s log-det/Vendi machinery (Plan 599) is set-level in a fixed cosine kernel without graph diffusion — the nearest shipped shape, still no multi-hop structure.

## Task

- [ ] **T1** After katgpt-rs Plan 634 Phase 1 lands: REPORT-ONLY `Div_P^τ` readout per mined batch intake over the rule-embedding kNN graph, printed beside the existing batch-intake gate lines (the score-bench/provenance strata precedent — report lines, no gate changes). **— PRECONDITION SCOPED 2026-11-11 (zed-a647854c@m3, substantiation subagent: plan 634 read in full, API surface grepped, zero sibling code consumers found — only riir-ai Plan 646 BLOCKED + Research 407 mention the lane):** Plan 634's own phase definition makes Phase 1 = T1.1–T1.6 (T1.6 the last row under `## Phase 1 — Core primitive`), so the WRITTEN precondition is unmet today (T1.6 batch selector + the set_admission const-D seam + T2.1–T2.7 open; claim `amo-idle-634p1` expired 11-11T15:47Z). BUT the readout consumes only the LANDED T1.4 scalar — `SimGraph::build(n, &edges)` + `div_tau(&mu, tau, &mut q, &mut r) -> f32` (feature `diffusion_diversity` = `["set_admission"]`, landed `982e303ba`; n ≤ 256 is the spectral `DiffusionEmbedding` cap, NOT div_tau's; mu = the len-n batch distribution, q/r caller scratch, zero-alloc, Issue-940 ascending-index folds pinned for the snapshot posture) — NOT T1.6's selector (a T3/646 concern; Plan 634 itself: "the Div scalar may promote report-only regardless"). The rule embeddings exist in-repo: `ModellessEmbedder.embed(&str) -> [f32; LATENT_DIM]` (`src/ragplane/embedder.rs:281 embed_modelless`, the corpus_index/DiffAnchor/embedder triangle), so the kNN graph is buildable TODAY. **Amendment: T1's precondition narrows to Plan 634 T1.4 (the Div scalar + its F2 seed — landed), NOT the full Phase 1 (which stays 646's block; 646's wording is the plan-holder's to keep).** T1 remains unstarted here — it needs a mining-batch intake surface to print beside (the snapshot's intake rows are .md bookkeeping, not a gate line), which is its own design slice; filed work stands.
- [ ] **T2** Correlation cell: batch Div vs downstream score-bench heal-rate deltas across batches; pre-register the promotion question; only a measured correlation promotes T3.
- [ ] **T3** `- [-]` DEFERRED (promotion-gated): a batch-intake diversity floor or intake weighting. Never wires without T2 evidence + the primitive's GOAT pass + the `latent_retrieval` graph build cost measured (the kernel_route_narrow cost lesson applies).

## Gates

- Report-only T1 changes no exit codes; opt-in flag; feature-off byte-identity.
- Any weighting (T3) is sigmoid-compatible, never softmax.

# Issue 094: Trajectory Covariability-Class Mining POC

**Status:** Open — POC (blocked on katgpt-rs Plan 596 `slice_tca` primitive)
**Date:** 2026-09-11
**Research:** ../riir-neuron-db/.research/309_slice_tca_covariability_class_consolidation.md (panel + prior art; this is the consumer-surface hypothesis from the model-based advocate, angle B2)
**Source:** Pellegrino, Stein & Cayco-Gajic, *Nat Neurosci* 2024 (sliceTCA), doi:10.1038/s41593-024-01626-2

## Hypothesis

Decompose the self-evolve trajectory store (`.heal/trajectories.kv`, Warm tier) as X[rule × heal-event × session] with the deterministic slice-rank fit (katgpt-core `slice_tca`, Plan 596). The three covariability classes map to:

- **stable-class mass** (neuron-slicing over rules) = rules with stable, convergent fix shapes → strong corpus candidates;
- **routine-class mass** (trial-slicing: stereotyped fix sequences with session-scaled amplitude) → rule families that always fix the same way → automatable/`--fix` promotable;
- **drift-class mass** (time-slicing: fix shape drifting across sessions) → **lint-fashion / environment-drift rules → re-mining candidates**.

Measurable: drift-class mass per rule should predict which rules needed re-mining in retrospective batch history (labeled ground truth exists in `.distill/001_mining_queue_snapshot.md` lineage + batch records).

## POC tasks

- [ ] Extract X[rule × heal-event × session] from the trajectory store (evidence tiers Certified/Heuristic respected — Withdrawn excluded).
- [ ] Fit per-rule-class shares (CPU-scale; the classifier path alone may suffice — 3 SVDs, no joint fit needed if per-rule routing is the only output).
- [ ] Retrospective eval: does drift share rank re-mined rules above stable ones (AUC)? Naive floor: majority-class + per-unfolding-SVD baselines.
- [ ] PASS ⇒ feed the ranking into post-mining intake (`.distill/001` §"Post-mining intake"); FAIL ⇒ negative record here, close.

## Notes

- No new substrate in this repo — consumes `katgpt_core::slice_tca` (path dep exists via katgpt-core).
- Confidence: medium-high (labeled outcomes; cheap). Novelty unclaimed (analysis application).

# Issue 91: Fix-loop trap detector — saddle-circling between near-fixes

**Status:** Open — fusion idea, novelty TBD; PoC-first on real `.heal/` trajectories (2026-09-11)

> **Research:** katgpt-rs [Research 546](../../katgpt-rs/.research/546_Fractal_Basins_Saddle_Trap_Latent_Reasoning.md) (arXiv:2609.04963)

Fusion idea: `--fix-compile` iterates fix→compile cycles; an oscillating pair (error A → fix B → error B → fix A) is saddle-circling between nearly-correct fixes — the same transient-chaos shape the paper documents for latent loops. A flip-rate/persistence detector over per-span fix attempts could escalate, switch domain, or decline instead of burning the fix budget (the L4 datum: ~191 s/fix at 0/60 EM — trapped loops are expensive here).

- [ ] T1 Mine existing `.heal/trajectories.kv` + burn-ack history: how prevalent are oscillating fix pairs (same error code reappearing after ≥1 intervening fix)? Prevalence gates everything else.
- [ ] T2 If prevalence material: prototype a two-way detector (flip-count threshold on the error-code ring) in the rustc_errors domain driver; measure decline-early vs burn-full-budget on replayed trajectories
- [ ] T3 Escape (kick) semantics do NOT transfer (fixes are discrete edits, not latent states) — only trap *detection* is in scope here; record the delta in the closing note

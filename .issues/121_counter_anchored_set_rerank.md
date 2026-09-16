# Issue 121: Counter-anchored set rerank for the LatentMatcher span pool

> **Status:** OPEN (2026-09-16) — filed from katgpt-rs Research 564 / Plan 599; blocked on Plan 599 Phase 1 landing (interim path exists via `vendi_diversity` directly)

## Context

`LatentMatcher` fans a code span out across seven per-domain `RuleIndex`es and merges the hits — an **unanchored set decision**. The stack already detects the failure mode *post-hoc*: `instantiation_rerank::detect_collapse` returns `CollapseVerdict::Collapsed { family, oracle_outside }` when the top-k pool collapses onto one rule family (the healer's exact "4 tents" — four near-duplicate spans from one lint family), and `collapse_monitor` tracks a pool-diversity entropy axis. Detection is not prevention: nothing in the selection objective counter-anchors the collapse, and there is no recovery loop when it fires.

R4T (arXiv:2603.06397, ICML 2026 — katgpt-rs Research 564) supplies the missing structure as a **set-level composite with mutual counter-anchors**: alignment (similarity to the query span) × diversity (Vendi/kernel spread over the admitted set) × groundedness (identity here — every candidate is a real rule). Their measured ablation law: drop any one anchor and a specific degenerate family becomes optimal (groundedness-only → coordinate-gaming; +alignment → paraphrase collapse; +diversity → the balanced region).

## Ask

1. **Selection-time counter-anchored admission** over the merged span pool — consume `katgpt_core::set_admission` once katgpt-rs Plan 599 lands; interim: compose `vendi_diversity` (already in katgpt-core `certified_frontier`) + alignment score + colinearity cap directly in the rerank.
2. **Vendi axis in `retrieval_eval`** — report the geometric diversity of the admitted slate beside `canonical_rank` (the categorical family-entropy detector stays: family space exceeds the 8-D latent dims, they are complementary).
3. **Collapse → re-fan-out recovery loop** — on `CollapseVerdict::Collapsed` with `oracle_outside`, re-query with the θ-ladder (wider tangent directions off the span embedding) instead of returning the collapsed pool.
4. **Eval:** `detect_collapse` stays the evaluation axis, not the guard — the gate is judged by whether the collapse *rate* drops and oracle-recall rises on the composite fixtures + the OOD fresh-family rig.

## Tasks

- [ ] Interim rerank: vendi + alignment + colinearity cap over the merged pool (no new deps — katgpt-core re-export)
- [ ] `retrieval_eval`: Vendi axis + saturation report (`vendi ≥ 0.95·min(K,d)` at d=8 is a ceiling, not a win)
- [ ] Recovery loop on collapse verdict (θ-ladder re-query)
- [ ] Consume `set_admission` from katgpt-rs Plan 599 when it lands (replace the interim)
- [ ] A/B on composite fixtures + OOD rig: collapse rate ↓, oracle-recall ↑; kill-switch env per house convention

## Refs

- katgpt-rs `.research/564_R4T_Counter_Anchored_Fanout_Retrieval.md` (distillation + Path 0 table)
- katgpt-rs `.plans/599_counter_anchored_set_admission.md` (the primitive; PRIMARY track)
- riir-train `.plans/412_r4t_fanout_diffusion_recipe.md` (the trained twin; SECONDARY — its GOAT gate is judged against the modelless baseline this issue wires)

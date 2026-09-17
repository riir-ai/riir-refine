# Issue 121: Counter-anchored set rerank for the LatentMatcher span pool

> **Status:** RESOLVED 2026-09-17 — the consumer landed (opt-in `set_rerank` feature, Bench 096) and the A/B **refuted the promotion**: the canonical-merge control beat both admission arms on the composite rig and every ablation arm lost to shipped on the OOD rig. The real yield is the `all_rules` canonical-order fix (landed DEFAULT, +55.6pp top-1). Katgpt-rs Plan 599 T4.2 complete; `set_admission` stays opt-in per the gate-loss verdict.

## Context

`LatentMatcher` fans a code span out across seven per-domain `RuleIndex`es and merges the hits — an **unanchored set decision**. The stack already detects the failure mode *post-hoc*: `instantiation_rerank::detect_collapse` returns `CollapseVerdict::Collapsed { family, oracle_outside }` when the top-k pool collapses onto one rule family (the healer's exact "4 tents" — four near-duplicate spans from one lint family), and `collapse_monitor` tracks a pool-diversity entropy axis. Detection is not prevention: nothing in the selection objective counter-anchors the collapse, and there is no recovery loop when it fires.

R4T (arXiv:2603.06397, ICML 2026 — katgpt-rs Research 564) supplies the missing structure as a **set-level composite with mutual counter-anchors**: alignment (similarity to the query span) × diversity (Vendi/kernel spread over the admitted set) × groundedness (identity here — every candidate is a real rule). Their measured ablation law: drop any one anchor and a specific degenerate family becomes optimal (groundedness-only → coordinate-gaming; +alignment → paraphrase collapse; +diversity → the balanced region).

## Resolution (2026-09-17)

katgpt-rs Plan 599 Phases 0-3 landed (`set_admission`, Bench 811 GOAT), so the **interim path (task 1) was SKIPPED by design** — the substrate-first rule forbids building the vendi+alignment composition when the real primitive exists. The consumer consumed `set_admission` directly.

The A/B ([Bench 096](../.benchmarks/096_set_rerank_ab.md)) measured, against the canonical-merge control and with real-clippy truth:

- **Composite rig (n=27)**: canonical merge 85.2% top-1 / 96.3% recall@5 vs set-rerank Full8 70.4/88.9 and Structural4 74.1/85.2 — the control wins; the colinearity cap (θ=0.95) demotes near-duplicate TRUE siblings (composite spans are multi-lint by design).
- **OOD rig (pure retrieval)**: shipped 100% ID / 50% OOD vs the set arm's best ablation 83.3% / 33.3% — every α ∈ {0.6, 0.2, 0.0} arm loses. NOROLE stayed 0 under every arm (grounding holds).
- **Collapse rate 0 in every arm** — the failure mode this issue was filed against does not materialize at merge depth on today's corpus (Issue 024's crowding was at POOL depth, fixed by `RERANK_POOL=20`).
- **The real defect found**: `all_rules`'s merge discarded the instantiation re-rank's structural key (contradicting its own doc) — fixed on the DEFAULT path (+55.6pp top-1 / +40.7pp recall@5), regression test `all_rules_keeps_the_canonical_key_across_domains`.

**Standing**: `set_rerank` feature stays opt-in (kill-switch `RIIR_CLIPPY_NO_SET_RERANK=1`); the vendi axis (`EvalReport.vendi_axis`) + the OOD policy arm (`run_withheld_pair_set_admission`) ship as the standing re-measure instruments. **Re-open trigger**: the `pool_depth_rescue_guard` firing again (corpus growth reintroducing merge-depth crowding) — re-run the A/B then.

## Tasks

- [-] Interim rerank: vendi + alignment + colinearity cap over the merged pool — SKIPPED by design (the real `set_admission` substrate landed before the interim was needed; building both would violate substrate-first)
- [x] `retrieval_eval`: Vendi axis + saturation report (`EvalReport.vendi_axis` — mean vendi / saturated / cert-collapsed beside the categorical detector; printed in `print_report`)
- [x] Recovery loop on collapse verdict (θ-ladder re-query) — `admit_with_recovery_into` wired in `counter_anchored_slate`; unit-pinned; measured inert (0 certificates collapsed on the fixtures)
- [x] Consume `set_admission` from katgpt-rs Plan 599 — the consumer IS the landing (task 1's interim skipped)
- [x] A/B on composite fixtures + OOD rig — executed with the canonical-merge control + α-ablation; verdict recorded (Bench 096); kill-switch env per house convention

## Refs

- katgpt-rs `.research/564_R4T_Counter_Anchored_Fanout_Retrieval.md` (distillation + Path 0 table)
- katgpt-rs `.plans/599_counter_anchored_set_admission.md` (the primitive; PRIMARY track — T4.2 = this issue)
- riir-train `.plans/412_r4t_fanout_diffusion_recipe.md` (the trained twin; SECONDARY — its GOAT gate judged against the modelless baseline this issue wired; the baseline REFUTED the arm on these rigs)
- [Bench 096](../.benchmarks/096_set_rerank_ab.md) — the full A/B record

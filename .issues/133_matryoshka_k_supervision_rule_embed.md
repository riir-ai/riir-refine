# Issue 133 — Matryoshka-k supervision for `rule_embed` (reopen-lane enhancement)

**Status:** OPEN (DEFERRED-TRIGGER — do not start until `rule_embed_poc`'s reopen trigger fires: organic fixseq rings or oracle-anchored real-repo spans at scale, Bench 099). Filed 2026-09-24 from katgpt-rs [Research 584](../katgpt-rs/.research/584_Matryoshka_Attribution_Sigmoid_Topk_Budget_Primitives.md) (arXiv:2609.25518 "Matryoshka attribution" distill).

## The idea

`rule_embed_poc` (Issue 126 lane (a), Bench 099) trains the span↔rule two-tower scorer against a fixed objective; the shipped suggest lane consumes top-k at whatever slate size the caller picks. The MAttr distill's transferable recipe: **randomize the budget k during training** (`k ~ Uniform` / `log` / `log_both` over slate sizes) and merge through `exact_mass_admit` (the exact-mass gate: `Σσ = k` via bisection — consuming katgpt-rs Issue 879's primitive once it lands) so **one training run yields a ranking calibrated at every slate size** — the whole top-k-accuracy-vs-k curve from one run, instead of a ranking implicitly optimized for one operating point.

Why it matters here: the healer's slate consumers run at different widths (`counter_anchored_slate` k=5 today; fix-lane dispatch has no width constraint), and the withheld-rule anti-memorization read (Bench 099's rule-level BLAKE3 split) would extend naturally to a withheld-WIDTH read (held-out k values) — a cheap extra generalization axis for the reopen gate.

## Scope guard

- Mechanism is prior-art-known (Nested Dropout 2014 / MRL 2022 own "randomize budget → nested ordering"); the claim is only the application to the (span → rule slate) binding corpus. Novelty is TBD at reopen time — measure before claiming.
- No new deps; consumes `katgpt_core::exact_mass_admit` (opt-in feature `exact_mass_admit`, Issue 879 — NOT `gate_sigmoid_topk`, which is katgpt-spectral's hard-cut router gate) — re-exports via the existing katgpt-core path dep.
- The `log_both` two-sided variant (supervise keep-first AND exclude-last) is the corpus-lifecycle shape (Issue 102) — record its measurement in the reopen bench, do not wire it into active_set.

## Tasks (on trigger)

- [ ] **T1** Re-run Bench 099 training with randomized-k supervision (uniform + log_both arms) vs the shipped fixed objective; evaluate at k ∈ {1, 3, 5, 10, 20} including held-out widths.
- [ ] **T2** If any arm beats the shipped scorer on the full-rig top-1/Gate floor set (Bench 099's standing gates), promote the training recipe inside `rule_embed_poc` and re-freeze `assets/rule_embed_frozen_v1.bin` (v2, BLAKE3-sealed).
- [ ] **T3** Record negative result here if the fixed objective wins — close without changing the shipped lane.

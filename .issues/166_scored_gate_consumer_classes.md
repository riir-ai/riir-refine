# Issue 166: Consumer-class audit of the healer's scored sigmoid gates (capacity-law triage)

**Status:** Open — triage + measurement-first; expected-null pre-registered for the ranking class

**Source:** [katgpt-rs/.research/624_Associative_Memory_Capacity_Laws.md](../katgpt-rs/.research/624_Associative_Memory_Capacity_Laws.md) (blog: alexwang.ai "You could have invented transformers from memory", 2026-10-15; theory: Zhong et al. arXiv:2505.19488 interference-to-signal)
**Related:** `.plans/005` L1444–1468 (corpus-growth precision-liability law) · katgpt-core `ssmax.rs` (the log-N law, attention-side) · katgpt-rs Plan 411

## Finding (narrowed in review — consumer classes, not a blanket lever)

Research 624's scope law 1: **a monotone sigmoid gate cannot change any pure-ranking outcome** at any sharpness or threshold. Scope law 3: **a trained scorer's temperature/threshold is part of its trained contract** — moving it at runtime is a calibration break, not a lever. Sorting this repo's scored gates by those laws:

| Site | Class | Law's verdict |
|---|---|---|
| `src/draft/rule_embed.rs` — per-pair `score_rule`, `SIGMOID_TAU = 0.1` (sigmoid-NCE **temperature**, shared train/runtime **by contract**: "the shipped score distribution must be the trained one") | Trained two-tower, consumed per-pair/ranking | **Not applicable — pre-marked.** Both scope laws exclude it; any τ change invalidates frozen artifacts AND the trained calibration. Do not audit further. |
| `src/traj_store.rs` — `sigmoid_dot_8` retrieval (top-k) | Pure ranking | Gate-invariant. Any margin drift vs corpus size is intrinsic (max-of-M tail), not gate-mediated — expected-null. |
| `src/evolve.rs` — `sigmoid(dot/temperature)` candidate scoring (temperature is a **config field**, default 1.0) | Ranking use → null; **blend use** (if its output is weighted with other signals) → sweep item | Ranking arm: expected-null. Blend arm: relative-scale question (Issue-013-style), not capacity. |
| `src/draft/margin_weight.rs` — `w(m) = e^{−2m}` where **m is the match margin** (not store size), blended into selection | **Weighted-sum consumer** — the one class where gate shape can matter | The only site worth a live measurement first. (Careful: its `M` in code is a per-match margin; the capacity-law `M` is store size. The blend's corpus-size exposure depends on how the margin distribution shifts with corpus growth — that is what T2 measures.) |

The corpus-growth liability is already documented (`.plans/005`: "retrieval precision does not survive corpus growth — treat corpus growth as a precision liability, not a free win"). What this issue adds is the consumer-class triage: the capacity law's lever class (absolute thresholds, weighted sums) is nearly ABSENT here — the healer's gates are mostly ranking consumers, so the honest expectation is **null**, and a clean negative closes the question cheaply.

## Tasks

- [ ] **T1** Inventory, don't assume: confirm the consumption shape of each site's output — is the gated score ever (a) summed/blended with other signals, or (b) compared to an absolute threshold? List every consumer per site. Ranking-only consumers are closed at inventory time with the scope-law citation, no measurement owed.
- [ ] **T2** For every weighted-sum/threshold consumer found (margin_weight's blend is the known one): measure its outcome vs corpus size (1× / 2× / 4×) on the existing `retrieval_eval` / `withheld_pair_ood` fixtures; for ranking uses, run ONE bounded expected-null confirmation (top-1 recall at 4×, fixed gate) and record the negative. Box state per AGENTS bench discipline; scratch `CARGO_TARGET_DIR=/tmp/...`.
- [ ] **T3** Verdict gate: if inventory finds no weighted-sum/threshold consumer beyond margin_weight AND the null confirms, close this issue with the negative measurement and a one-line pointer from `.plans/005`'s liability law to the consumer-class explanation.
- [ ] **T4** Lever arm (only if T2 shows a real weighted-sum/threshold drift): corpus-calibrated shape behind an opt-in feature flag, runtime only, byte-identical OFF; GOAT gate — floors flat at current corpus sizes, strictly slower degradation at 4× when ON, G4 zero-alloc unchanged. Promote only on measured gain; demote the incumbent otherwise.
- [-] (deferred) Sigmoid-gate capacity estimator as a shared katgpt-core primitive — lands with the first consumer that needs it, not speculatively.
- [-] (deferred, by scope law 3) Any `SIGMOID_TAU` retune — trained contract; a retune is a retrain decision, not a runtime lever.

## Constraints

- Sigmoid, never softmax — scale existing sigmoid shapes only; no normalization, no new exp-family scoring.
- Frozen artifacts stay frozen: `rule_embed`'s BLAKE3-sealed tables and its shared-temperature contract are off-limits to this issue.

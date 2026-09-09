# Issue 077: Margin-Weighted Rule Mining + Entropy-Gap OOD Axis (arXiv:2604.26841)

**Status:** T1 + T4 LANDED 2026-09-09 (riir-clippy `1994a8a`; rides katgpt-rs Issue 740 — consumer half landed, `regime_probe` opt-in). T2 + T3 remain open. First measured row: score-history run #84, gap −0.729 nats (d = −1.11) → anomalous_negative — interpretation in `.docs/08_benchmarks/entropy_gap_ood_axis.md`.
**Date:** 2026-09-08
**Research:** [katgpt-rs/.research/541_UDDM_Associative_Memory_Regime_Probes.md](../../katgpt-rs/.research/541_UDDM_Associative_Memory_Regime_Probes.md) @ katgpt-rs `77169275`
**Source:** [arXiv:2604.26841](https://arxiv.org/abs/2604.26841) — cross-entropy implicitly maximizes classification margins; margin spectrum `e^{−2M}` concentrates effort on smallest-margin items; entropy-gap as a critic-free regime/OOD detector.

## Ask

Two healer-consumer fusions of the regime-probe suite (rows M5 + M2-consumer of Research 541 §2):

1. **Margin-weighted mining (M5):** rank corpus rules / frontier pairs by prediction margin `M = observed-score − top-competitor-score` (per the drafter's score shape; sigmoid/score deltas, not softmax). Weight review/mining priority by the paper's `w(M) ∝ e^{−2M}` law — smallest-margin (decision-boundary) cases first. GOAT: `e^{−2M}`-weighted mining beats uniform-ordered mining on measured score-bench heal rate (G2), margin sign/monotonicity contract (G1), zero-alloc ranking pass (G3).
2. **Entropy-gap OOD axis (M2 consumer):** add the conditional-entropy-gap statistic to `score_bench` as a corpus-regime axis — quantifies whether the rule corpus sits in the memorization regime (held-out shapes get near-zero drafter entropy ≈ overfit) vs generalization regime — the measurable OOD generalization axis this repo wants for withheld-pair evaluation. GOAT: detected regime boundary agrees with an external oracle verdict inflection on the same probe set (convergent validity, G2 of Research 541 §5).

## Tasks

- [x] **T1** Consume `katgpt-core::regime_probe` (katgpt-rs Issue 740) once it lands; until then, prototype against a local feature-gated shim — no parallel entropy implementation in this repo. ✅ `1994a8a` — `score_bench` forwards `katgpt-core/regime_probe` (no separate feature, no new deps); entropy + gap kernels consumed verbatim (`conditional_entropy_nats`, `entropy_gap_into`), zero local entropy arithmetic.
- [ ] **T2** Margin spectrum over drafter candidates: expose per-candidate margin in the score path; offline analysis script first (does the existing score-bench corpus show the expected low-margin concentration?).
- [ ] **T3** `e^{−2M}`-weighted mining order behind a feature flag; A/B on score-bench heal rate vs current order; demote if no gain.
- [x] **T4** Entropy-gap OOD row in score_bench output (reference corpus vs held-out pairs); document interpretation in `.docs/` (which regime the shipped corpora sit in). ✅ `1994a8a` — always-on row in every score-bench run + the additive `ood` JSONL field; reference = `v1_frozen` (35, mean 3.840 nats) vs OOD = `gen6_real_repo` (17, mean 3.110 nats): gap −0.729 nats (d −1.11) → anomalous_negative, artifact `faa021c8…61904` (id-sorted, bit-deterministic); interpretation + the provenance-family reading in `.docs/08_benchmarks/entropy_gap_ood_axis.md`. Tests: 1396 passed (`--features score_bench --lib`, 8 new); clippy clean at score_bench + default.

## Notes

- This is the "did I ask what this does for the healer?" answer for Research 541 — the paper's margin law lands directly on corpus mining, and its regime probe lands on the withheld-pair OOD axis.
- Boundary: zero new deps; the primitive lives in katgpt-core (public), this repo consumes it.

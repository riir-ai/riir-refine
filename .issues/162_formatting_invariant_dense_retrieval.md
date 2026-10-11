# Issue 162: Formatting-invariant dense retrieval via compile-gated fix pairs

**Status:** Open — P1. Source: `../katgpt-rs/.research/618_SplitJEPA_Invariant_Variant_Block_Identifiability.md` (SplitJEPA, arXiv:2610.12349); primitive lands via katgpt-rs Plan 625 (`basis_split`). This is the healer (#2) fusion.

## Problem

The dense retrieval channel ranks spans by an embedding that moves when formatting moves. The healer already owns the world's best labeled-pair generator for exactly this: **compile-gated mechanical fixes** — before/after spans of `--verify`-passing edits are semantics-preserving by construction. SplitJEPA's matched-pair spectral head turns those pairs into the *formatting-variant basis* of dense-embed space; the invariant complement is the rule-relevant content. (Lineage note, verdict round 1: the estimator is within-class-scatter/SFA-class — classical; the identifiability gate + the instrument are the new part.) Nothing currently measures how much embedding movement is formatting churn, and no retrieval path is invariant to it.

## Tasks

- [ ] T1 Wait for katgpt-rs Plan 625 Phase 1, then mine before/after span pairs from the existing fix-trajectory store (offline, batch; the store already holds the pairs — no new collection).
- [ ] T2 Instrument FIRST (report-only): formatting-mutant top-1 flip rate — apply verified-mechanical-only edits to a frozen eval slice, measure how often dense top-1 flips; report the `pair_leakage` residual beside it. **Holdout law:** fit the formatting basis on fix pairs from one rule family, evaluate the residual on a DIFFERENT family (cross-generator holdout) — a same-family holdout is a calibration residual, never purity. The mutant infrastructure already exists for behavioral-equivalence work; reuse it.
- [ ] T3 Decide from T2 numbers: if formatting churn is material, opt-in projected dense KNN behind a feature flag (projection pre-baked at index build; query-time cost = one matvec).
- [ ] T4 Gate (if T3 arms): G1 top-1 must not flip under verified-mechanical-only edits on a frozen slice; G3 paired non-inferiority (LB95) of projected-KNN vs plain on the frozen slice — BM25 untouched; G2 pair mining offline, KNN cost unchanged; G4 zero-alloc accumulation.
- [ ] T5 Optional rider (cheap, same pairs): corpus-churn feature = fraction of embedding movement attributable to formatting; feed mining triage.

## Acceptance

T2 numbers recorded under `.benchmarks/` numbering discipline; any armed lane carries the T4 gates with the frozen-slice LB95 verdict; off-flag behavior bit-identical.

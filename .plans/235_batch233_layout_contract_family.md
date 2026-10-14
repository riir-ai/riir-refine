# Plan 235 — Batch 233: layout-contract family rule + B224 UB-confirmation rider

**Status:** LANDED (2026-10-14, cycle 88-M3; claim `51322feb` discharged)
**Claim:** `51322feb` (2026-10-14, cycle 88-M3)
**Sources:** ggml-org/llama.cpp @ `6868a52` (#28738) + `e7268bf` (#30480) + `8c2134a` (#30501), all MIT
**Intake:** the cycle-87-M3 row (1a) + (1b), `.distill/001`

## Tasks

- [x] T0: fetch + read all three .diff files at the pins; re-verify every staged quote VERBATIM (the cycle-87 row's quotes vs the actual diffs) — ALL VERBATIM (the conv_1d header, the contents-oracle sentence, the mask-layout line, both GGML_ASSERTs, the ggml.h UB note, the #if 0 disable + TODO)
- [x] T1: dedup grep vs the B211 layout family + B224 moe-duplicate-id family (3+ vocabulary variants; concepts ship under operator names) — CLEAN: axis-order/batch-fold zero hits; kinships adjudicated in-rule (ids-path-rebinds-stride-meaning = the stride face; B211 = the codegen face)
- [x] T2: adjudicate rule-vs-rider per face at the slot with the full family read — BOTH faces of (1a) compose ONE own L-category rule; (1b) = count-neutral B224 rider
- [x] T3: author the corpus entry (entries_gpu_92.rs, `reshape-relabel-axis-scramble-content-oracle`, L)
- [x] T4: author the B224 rider (count-neutral, #30501 UB confirmation + the three-part guard law)
- [x] T5: batch233 gates — pseudocode parse + needle tests (18 rule + 7 rider needles) — FIRST RUN GREEN
- [x] T6: G2 — kernel_opt lib 3106/0/1i; default lib 2659/0
- [x] T7: G3 — clippy ×4 postures clean (2 doc_markdown healed via cargo refine --fix --write --verify, compile-gated)
- [x] T8: retrieval floors — 21/21 HELD (311 s, no decay, no re-pin)
- [x] T9: count pins — 805→806 at ×9 sites (tests ×5 + historical ×3 + table + sum/grouped + mod.rs L comment/table/doc + router.rs + AGENTS + kernel_opt.md + README honest-label)
- [x] T10: freeze/score-bench adjudication — NOT owed (zero clippy-corpus delta, P30)
- [x] T11: research note 264 (`.research/.highwater` 263→264, dual_allocation_gate 0/0/0)
- [x] T12: docs — AGENTS.md kernel_opt bullet 806 + B233 narrative; kernel_opt.md B233 pack row + 806 + batch list; README honest-label repair (795-stale → 806)
- [x] T13: snapshot landing row (`.distill/001` — LF-preserving python) + intake rows (1a)/(1b) marked CONSUMED
- [x] T14: commit `feat: batch 233: ...` + push

## Notes

- The two faces compose ONE law candidate: a multi-axis fold (batch/head/broadcast axes) that preserves the declared output SHAPE while silently reordering VALUES under it is a layout CONTRACT violation — the oracle is content-comparison against the per-axis decomposition (never shape equality), and the seam asserts same-shape between the folded carrier and its mask/selection.
- Count discipline: 1 own rule max (kernel_opt +1 → 806); riders count-neutral.

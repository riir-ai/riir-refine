# Research 264 — Batch 233: the layout-contract family rule + the B224 UB-confirmation rider

**Status:** RECORD
**Date:** 2026-10-14 (M3, riir-refine idle loop cycle 88)
**Claim:** `51322feb` (pushed pre-authoring per the claim-first protocol)
**Sources:** ggml-org/llama.cpp @ `6868a52` (#28738) + `e7268bf` (#30480) + `8c2134a` (#30501), all MIT
**Intake:** the cycle-87-M3 row (1a) + (1b), `.distill/001`

## The rule — `reshape-relabel-axis-scramble-content-oracle` (kernel_opt, category L)

The layout-contract family law: **a reshape re-declares dimension bounds over FIXED memory; it cannot
permute axes or restore per-axis structure.** When the producer's true layout and the consumer's
documented layout differ by an axis order, every shape check passes and the values are wrong.

Two upstream faces, one genus:

1. **Axis-order face (#28738):** conv_1d folds the batch into the GEMM rows, so the product's flat
   layout is `[OL, N, OC]`; the documented output is `[OL, OC, N]` (ne-order). The old wiring was ONE
   reshape declaring the documented bounds over the true-layout memory. At N == 1 the two layouts are
   byte-identical (a 1×1 permutation is the identity) — every batch-1 caller (whisper-class) was never
   wrong, and the defect hid for years. At N > 1 every off-diagonal element reads another's value.
   Repair: reshape to the TRUE layout, then an explicit `permute` under a materializing `cont`.
2. **Structure face (#30480):** the DSA k-pool selection flatten — 4D → `reshape_2d` → elementwise
   add → `reshape_4d` back silently lost the per-batch/per-head structure. Repair: carry the selection
   in the mask layout end-to-end (a strided 4D view over the real row pitch, `sel->nb[2]`-derived) +
   the seam pinned: `GGML_ASSERT(ggml_are_same_shape(sel, kq_mask))` + the count assert
   `GGML_ASSERT(n_sel == (int64_t) inp_kpool->n_sel)`.

**The oracle** (the load-bearing half, verbatim from the shipped test header): "A wrong layout keeps
the shape intact and scrambles the values, so the comparison is on the contents." Content comparison
against the per-axis decomposition (one batched run vs N singleton runs sharing the same weight and
input slices, per-batch NMSE ≤ 1e-7, worst batch decides the case) — never shape equality. The assert
is the SEAM guard; the decomposition is the ORACLE. Both, not either.

## The rider — B224 `moe-duplicate-id-compact-prepass-static-bound-tile-dispatch` += #30501 UB-contract face

The duplicate-id precondition moved from an implementation assumption to an API CONTRACT: the ggml.h
note ("this op has undefined behaviour if a given `a` is referenced more than once for a given `b`",
tagged `[TAG_MUL_MAT_ID_DUPLICATE_ID]`) + the dup test matrix `#if 0`-disabled — NOT deleted — with
the CUDA-support TODO pinned to the #29998 thread. The three-part guard law for an explicitly-UB
input class: (a) the contract at the API declaration, (b) the tests stay behind a literal disable
(they are the re-enable gate), (c) the disable comment carries the re-enable link. Count-neutral.

## Dedup

`axis-order|batch-fold` zero corpus hits. The permute hits are coalescing/bank-conflict/
deterministic-scatter families. Closest kinships adjudicated in-rule: `ids-path-rebinds-stride-meaning`
(B198 — the STRIDE-rebinding hazard, addressing; this is the AXIS-ORDER hazard, semantics) and B211
`repr-align-register-partition-store-forwarding` (the codegen face; this is the graph face — the
intake's "axis-order sibling" framing confirmed distinct faces, one genus). B176's same-shape-assert
hazard arm is the consumer-side half of the mask face's seam assert.

## Gates

- G1: batch233 gate — 18 rule needles + 7 rider needles + both pseudocode halves parse. FIRST RUN.
- G2: kernel_opt lib 3106/0/1i; default lib 2659/0.
- G3: clippy ×4 postures clean (2 doc_markdown mechanicals in the new module header healed via
  `cargo refine --fix --write --verify` — compile-gated; the B192 class).
- Retrieval floors HELD 21/21 (311 s, no decay, no re-pin).
- Freeze/score-bench NOT owed (zero clippy-corpus delta, P30).
- Count pins: 805→806 at ×9 sites (tests.rs ×5 incl. batch116 + the historical re-pins ×3 +
  per-category table + the plan-003 sum/grouped, mod.rs ×2 comments + the L category comment,
  router.rs expert-total — the B231-lesson site, AGENTS.md, kernel_opt.md, README with the honest
  795-stale label).

## En-route lesson (the string-escape class)

Appending rider text to a corpus `gpu_concept` string literal via python replace inserted RAW `"`
characters — the file's literals escape inner quotes as `\"`, so the raw quotes terminated the string
early (5 compile errors, then an inconsistent-escape second attempt). The repair that worked: build
the plain text, escape `\` → `\\` and `"` → `\"` in ONE pass, replace the whole broken span, and
restore the closing quote. Recorded for the next rider author.

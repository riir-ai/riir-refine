# Batch 52 — Measured-GOAT distillation: phase-split per-position data liveness (riir-ai Issue 708 H4)

**Status:** RECORD (batch landed, closed)
**Date:** 2026-08-16
**Kind:** measured-GOAT distillation (the Batch 49 path — not file mining; a measured fix in riir-ai became a corpus rule the same day)
**Corpus:** 347 → 348 (D 62 → 63)
**Rule:** `phase-split-per-position-data-liveness-row-storage`

## Source event

riir-ai Issue 708 Part A (`forward_dflash.rs` non-functional forward). During the
H4 implementation (bidirectional concat attention), the fix surfaced a hazard
class the corpus did not cover:

- The Phase-A loop (`for layer { for pos { dispatch_qkv_concat } }`) projected Q
  per position into a **single-row** `[n_embd]` working buffer (`activations.q`)
  — each dispatch overwwrites the whole row, and the encoder is submitted only
  after BOTH loops are built, so **only the LAST position's Q survives** into
  the Phase-B consumer.
- The bug was invisible for the module's whole life because the Phase-B consumer
  (`dispatch_attention_bidirectional_manual`) was a no-op stub — nothing read
  the buffer, so nothing was observably wrong.
- It is **NOT** the B26/B27 stale-uniform class: those read the wrong *VALUE*
  per dispatch; here the right value is **never stored** — the buffer has no
  per-position storage at all.

## The fix (evidence)

- `q_cache: [seq_len × n_embd]` + `project_q` writing row `pos`
  (`out[params.pos * n_embd + i]`) in BOTH qkv kernels (dflash + dflare) — the
  write index rides the SAME `pos` field the kernel already receives in its
  uniform (zero new dispatch state).
- `attn_out_cache` for the per-position attention outputs (same class: the
  single-row `attn_out` was fine only because the per-pos Phase B immediately
  consumed it — the cache makes the one-dispatch-per-layer attention possible).
- The backward's Q source updated to copy its per-head slice from `q_cache`
  row `pos` (it previously read `activations.q` — whichever Q the LAST Phase-A
  dispatch left behind).
- GOAT: `test_forward_matches_cpu_reference` — full-stack GPU vs CPU reference
  **max_abs=3e-6, max_rel=5e-6** (Config::micro, seq_len=4, LoRA on Q/K/V);
  565/565 lib tests under `dflash_training,dflare_training`;
  `test_chunk_size_grad_invariance` PASS on real data (bit-identical grads,
  chunk 1 vs 2); 3 previously-ignored backward tests un-ignored + passing.
- Commit: riir-ai `42b564759`.

## The rule (corpus entry)

- **Category:** D (dispatch/lifecycle)
- **Pattern:** Phase-A dispatch loop writes a single-row working buffer per
  position; a separately-scheduled Phase-B loop needs every position's value.
  Only the last write survives.
- **Tell:** dispatch fn binds a whole single-row buffer; kernel writes flat
  `[0..dim)`; caller loops positions BEFORE the consumer phase executes.
- **Fix shape:** per-position row storage (`[seq_len × dim]` cache; kernel
  writes row `pos` via the existing pos-in-uniform; consumers index rows).
- **Distinct from:** B26 `per-slot-uniform-buffer-write-ordering` + B27
  `staging-packed-uniforms-encoder-ordered-copies` (uniform PARAMS staleness —
  wrong values read; this is output `DATA` liveness — per-position values
  never stored); B34 `preallocated-activation-slots-inplace-save` (saves FULL
  per-layer activations for backward replay; this fixes the forward's own
  cross-phase consumption); B39 `append-cache-rewind-overwrite-in-place`
  (cache growth/lifetime; row count here is fixed by construction seq_len).

## Dedup evidence

Grep of the full corpus (`corpus/*.rs`) for `single-row|per-position row|row
storage|liveness|last-write-survives|two-phase` before proposing: only
`expert-loop-two-phase-split-batch-across-iterations` (sync/batching across
iterations — loop structure, not data liveness) and an unrelated softmax
two-phase note. No existing rule covers cross-phase per-position data liveness.

## GOAT for this batch

- G1 syn-parse: `batch52_pseudocode_parses` PASS (ref_shape + opt_pattern).
- G2 lib tests: 600 passed under `kernel_opt` (135 corpus tests incl. the 3
  new batch52 gates + all count pins).
- G3 no-regression: clippy 0 new warnings in default + kernel_opt states (2
  doc_markdown fixes pre-commit — `[seq_len × dim]` needs backticks in `//!`
  prose; the `[x × y]` bracket family joins the gotcha list alongside O()
  expressions).
- **Retrieval floors re-measured (the Issue 017 kernel-batch gate): kernel
  self-variants 100.0/100.0/100.0, blended 3-domain 95.2/97.6/97.6 — ALL above
  the re-pinned minimums. No decay from the +1 rule.**
- Count-pin sites updated: entries.len() assert, KERNEL_CATEGORIES D row + D
  comment 62→63, counts-match table, grouped totals ×2, 347→348 mentions
  (mod.rs ×2, tests.rs ×4, router.rs ×4 lines/5 occurrences), the Batch 52
  header bullet, entries_gpu_06.rs slice header (rules 325-348 of 348).

## Landing notes (honest)

- The Batch 52 header bullet initially landed BETWEEN Batch 51's header line
  and its continuation (the anchor matched only the first line of a multi-line
  bullet) — the same class of mistake the historical B49/B51 interleaving in
  that header exhibits (B49's "the Bench-686..." continuation still dangles
  after B51's block, pre-existing). Caught on read-back; my bullet relocated
  after B51's complete bullet. Lesson re-confirmed: multi-line doc bullets need
  FULL-bullet anchors, not first-line anchors.
- The riir-gpu file-mining program remains COMPLETE/closed; this batch is the
  (c) reopen path exactly as designed (measured-GOAT distillation).

# Research 171 — Batch 145: the Metal-shape pack (EFQ + MTPLX split-K + mincore routing)

**Status:** RECORD — 2026-09-16 (Batch 145 landed; gates green; commit recorded in the plan)

## Sources & provenance

1. **arXiv 2609.09721** (EFQ-Softmax: Exp-Free Quantization for Softmax) — the
   walk #9 lead 1 (Research 170 graded it B rule-lead). Full text fetched via
   the arXiv HTML (LaTeXML) rendering; quotes verified by the paper lane +
   caller spot-check. No `.raw` clone needed (paper lane).
2. **youssofal/MTPLX @ `a5e38bb7d6c099fe79b3a7d06fae91c64092cb9f`** (PR #391
   head, 2026-09-03, "perf(qwen4): Qwen3.8 Flash-Next stack, 16K decode 57.7
   -> 80.9 tok/s", Apache-2.0, MLX 0.32.2) — `.raw/mtplx` clone, fetched via
   `refs/pull/391/head` (the default branch HEAD `21be78b3` postdates the PR).
   Removed at landing per the `.raw` discipline. Kernel provenance in-tree:
   oMLX (jundot) `steel_qwen4_qsa_sparse_gqa.h` <- mlx-serve MIT
   `msv_attn_p256`.

Both lanes were quote-verified independently (2 subagent quote lanes), then
every load-bearing quote re-checked against the files by the caller (the
Batch-31 discipline).

## What landed (corpus 612 → 614; rust_perf 126 → 127)

1. **`direct-low-bit-generation-replaces-exp-quantize-roundtrip`** (kernel_opt
   B/Compute, `entries_gpu_42.rs`) — the EFQ lane.
2. **`kv-split-decode-underfill-per-slot-validity`** (kernel_opt
   F/FlashAttnTiling, `entries_gpu_42.rs`) — the MTPLX lane.
3. **`mincore-probe-routes-gather-arm`** (rust_perf caching, `entries_06.rs`,
   suggestion-only) — the MTPLX host lane.
4. **Rider** on `same-input-gemv-weight-concat-at-upload`
   (`entries_gpu_07.rs`) — the hyper-read face.
5. **Reciprocal rider** on `split-k-two-pass-attention-decode-long-context`
   (`entries_gpu_18.rs`) — the sparse decode twin cross-ref.

The underfill negative cell is DUAL-SOURCED in the KV-split rule's regime
note (MTPLX hyper_read v1 13.2 vs 67.8 tok/s + Hopper 2609.12923 fixed
64-row GMMA fragments) — never a standalone rule, per the charter.

## The probe-first law, again: the charter's dedup verdict was WRONG

The staged charter (snapshot L87, commit `259746bc`) recorded the split-K
dedup as **CLEAN**: "`split.?k` corpus hits are only a covariance-rule
dual-note + a CDNA MMVQ geometry comment; NO attention split-K rule exists."

That was a truncated-grep artifact: the mine-time rule-id grep found
**`split-k-two-pass-attention-decode-long-context`** (Batch 105,
`entries_gpu_18.rs`) — the dense long-context two-pass split with the same
(m, l)-partials + LSE-merge mechanism — AND its Batch-142 rider already
carries an occupancy-deficit trigger ("split-KV fires as an OCCUPANCY patch,
not a length threshold alone"). The same class as the earlier truncation
lesson: a `head`-limited grep output ate the finding; the rule-id list grep
(the load-bearing step) caught it before authoring.

**Adjudication: ADJACENT, author the delta.** The genuine delta the new rule
carries:
- the TRIGGER axis: grid-WIDTH underfill at decode M (rows x heads vs SM
  count: M=4 -> 8 threadgroups on 40 cores; M=1 -> 2) vs the Batch-105
  rule's context-LENGTH crossover (32K+) with its fixed-block shape gate;
- the SPARSE half: per-slot validity for an UNSORTED selection
  (`block_id < complete_blocks`, the prefix-cut admits/drops hazard, the
  `<=` incomplete-block double-count trap) — dense split-K has no validity
  axis at all;
- the no-empty-split clamp (`exact_splits` round-down) + the one-encoder
  two-dispatch mitigation.

The name was changed from the charter's `kv-split-decode-attention-
reduction-axis` to `kv-split-decode-underfill-per-slot-validity` so
retrieval does not shadow the Batch-105 rule; the two carry loud
distinct-from rows in both directions (rule text + rider).

## Paper-vs-charter corrections carried (the quote lanes' value)

- **"Apple A5-class" is not in the paper** — the paper says only "the A5
  vector unit" (Huawei-affiliated); no Apple attribution in the entry.
- **−40.33% is the VECTOR STAGE of the fused probability-generation kernel
  only** (fixpipe 0.3–1.0%; mmad identical by construction and excluded);
  end-to-end is bounded by the vector-stage share. The paper's kernel table
  is labeled "LUT" — the timed variant may be the fused-LUT ablation; the
  ambiguity is carried in the entry, never flattened.
- **Quality:** the durable claim is "beats MXFP4 within 0.005 of FP16"
  (0.6749 → 0.6773 at batch 8; +0.0002 at batch 16 — operating-point
  dependent); the LARGE gains are VL grounding (0.7826 → 0.8000).
- **MTPLX:** the 13.2/67.8 tok/s cell belongs to `hyper_read` v1 (used
  in-tree as the general underfill negative); the QSA-specific motivation
  arithmetic is "EIGHT threadgroups at M=4" (M5 Max comment). Both quoted
  with their own hardware contexts, never merged.
- `qsa_sparse_gqa_decode.h` is the SPLIT variant's host; the row-parallel
  single-pass kernel is `steel_qsa_sparse_gqa.h` (the charter's file map
  had it reversed).
- The hyper-read rider's one genuinely-new trick: the in-kernel
  POINTER-SWITCH virtual-row weight fold (inject weights folded in as
  virtual output rows 320..323 — concat semantics with zero upload-time
  copy), an alternative to B63's bake-into-one-buffer for weights that
  arrive as separate buffers at route time.

## GOAT (G1–G3 + retrieval)

- **G1:** `batch145_rules_present` + `batch145_concepts_covered` +
  `batch145_pseudocode_parses` (syn-parse both shapes, fn-wrapped) + the
  rust_perf `batch145_mincore_rule_present_and_annotated` gate — 4/4.
  Phrase-pin corrections en route (the batch144 precedent repeated): the
  pin must carry the entry's actual casing ("SAME (code, scale) operand")
  and reference the concept's actual words ("no-empty-split clamp").
- **G2:** kernel_opt lib **1877/0** (was 1875 + 2 new gates) + default lib
  **1531/0** (was 1530 + 1 new gate).
- **Retrieval floors HELD byte-identical** (Issue 022/Bench 024 pins):
  kernel self-variants 100%, blended 3-domain 94.7/96.9 (op-bigram, =
  Batch 144's numbers) vs floors 77.1/86.4/92.4; clippy top-1 89.6% ≥ the
  80% bar. No re-pin.
- **G3:** clippy 0 on the write-set in default / kernel_opt / all-features
  (all-targets). En-route doc-lint repairs (rustdoc parses the `//!`
  header as markdown): a `+` at line start inside a `//!` history
  paragraph IS a markdown list bullet — one line starting with
  "+ sparse per-slot validity" made the parser read the rest of the
  paragraph as lazy list continuation (28 `doc_lazy_continuation`
  warnings from one character); backticked the snake_case identifiers
  (`rust_perf`, `hyper_read`, `entries_*.rs`) and split "FlashAttn".
- **score-bench:** heal_rate **96.2% FLAT** (row #117; 11th→12th consecutive
  flat), shaped_rate 94.3% (1 shaped fixture at 0.5 — the gen-8 axis stays
  non-vacuous), frontier snapshot appended (generation 8), cross-OOO axis
  unchanged (3 candidates, 12/17 audited).

## Authoring lessons (→ authoring_discipline feed)

1. **Grep truncation is a dedup verdict hazard**: the charter's `split.?k`
   grep ran with `head -8`-style output limits; the rule-id list grep at
   mine time is the load-bearing step (authoring discipline §2), and it
   needs the FULL hit list before adjudicating CLEAN. Same class as the
   09-14 sweep's clippy-prefix lesson: read the whole stream.
2. **A `+`/`-` at line start inside `//!` doc comments is markdown**: the
   Batch-144-style history paragraphs are list items; a continuation line
   beginning with `+` silently re-roots the list. Writer-side check:
   never line-start `+`/`-`/`*` in `//!` prose.

## Non-goals honored

- The two MTPLX host trims (`_stack_hot_rows`, `uniq.tolist()`) — unclaimed
  (ordinary dedup candidates).
- Walk #9 league-intel rows — no rule-shaped content.
- No riir-ai/riir-gpu edits; no league row; kernel_verify STUB stays.

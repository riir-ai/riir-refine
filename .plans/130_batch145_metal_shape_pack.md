# Plan 130 — Batch 145: the Metal-shape pack (EFQ + MTPLX split-K + mincore routing)

**Status:** COMPLETE — 2026-09-16 (Batch 145 landed; all gates green)

## Source

The staged Batch 145 charter in `.distill/001_mining_queue_snapshot.md` (L87,
commit `259746bc`): 2 clean kernel_opt rules + 1 suggest-only rust_perf rule +
2 evidence riders, accumulated from walk #9 (Research 170, EFQ lead) + the
MTPLX PR #391 read verdict (snapshot L49). Sources:
- arXiv 2609.09721 (EFQ-Softmax) — paper lane.
- youssofal/MTPLX @ `a5e38bb7` (PR #391 head, Apache-2.0) — `.raw/mtplx`
  clone, pinned; `rm -rf` when the batch lands.
Authoring discipline: `.docs/02_corpus/authoring_discipline.md` (Issue 102 T5).

## Candidate rules (charter + quote-lane results)

1. **`direct-low-bit-generation-replaces-exp-quantize-roundtrip`** (kernel_opt;
   EFQ lane A): shifted scores → block-scaled E2M1 via exponent-only scale
   (Eq. 8) + ONE calibrated affine rule (Eq. 11), same (code, scale) operand
   in numerator AND denominator; the dense elementwise exp + quantize pair is
   deleted. QUOTE-LANE CORRECTIONS (paper wins): "A5 vector unit" only (never
   Apple); −40.33% is VECTOR-STAGE of the fused probability-generation kernel
   (fixpipe 0.3–1.0%, mmad excluded); the kernel table is labeled "LUT" —
   carry the variant ambiguity; quality claim = "within 0.005 of FP16 while
   beating MXFP4" (language gain operating-point-dependent; VL gains large).
   Dedup vs `operand-precision-gated-by-consumer-invariance`: that rule picks
   WHICH precision by consumer tolerance; this deletes the high-precision
   intermediate entirely — direct generation vs precision selection.
2. **`kv-split-decode-attention-reduction-axis`** (kernel_opt; MTPLX lane B):
   decode-attention grids sized over query rows only underfill at decode M
   (M=4 → 8 threadgroups on 40 cores); split the REDUCTION axis (KV seq) —
   per-split unnormalised online-softmax state + log-sum-exp merge pass that
   owns the normalising divide; per-slot validity (`block < complete_blocks`)
   NOT a prefix cut (decode selector hands argpartition's UNSORTED output);
   no-empty-split clamp (`exact_splits` rounding); combine trade mitigated by
   one-encoder dispatch. Regime: decode M ≤ 8 at long KV; prefill keeps
   row-parallel. QUOTE-LANE CORRECTION: `qsa_sparse_gqa_decode.h` is the
   SPLIT host; the row-parallel variant is `steel_qsa_sparse_gqa.h`.
   The 13.2/67.8 negative cell belongs to hyper_read v1 (used in-tree as the
   general underfill negative) — cite it as the underfill rider datum, with
   the QSA-specific motivation being "EIGHT threadgroups at M=4".
3. **`mincore-probe-routes-gather-arm`** (rust_perf, suggestion-only;
   MTPLX `ple_row_gather.py`): per-call residency-probe ROUTING — a sampled
   mincore(2) probe (~0.5 ms, 256 fixed-stride samples, worst-of-all-maps)
   picks warm→vectorized fancy-index vs cold→pooled pread against a ~165 ms
   warm pass; threshold 0.99 (false-warm is the expensive mistake); fail-safe
   to pread on any probe error; small gathers keep the shipped path outright.
   ADJACENT not dup: Batch-139 PLE prefault (EAGER admission by RAM headroom)
   + B118 `parallel-disjoint-pread-hides-cold-page-latency` (OVERLAP cold
   page-ins); the NEW arm is the probe PICKING the arm. macOS mincore
   FILE-WIDE-residency caveat MUST be encoded (the Batch-139 refusal note —
   NOT-IN-TREE in the source).
4. **Rider (3→2)**: the underfill negative cell is DUAL-SOURCED (MTPLX v1
   `(1024,S,1)` 13.2 vs 67.8 + Hopper 2609.12923 fixed 64-row GMMA fragments)
   — lands INSIDE rule 2's regime note as eligibility evidence, never
   standalone.
5. **Rider (4→hyper-read)**: `qwen4_m4_hyper_read` (11→3 dispatch fused read,
   weights-read-once column-tile GEMV) — dedup-OVERLAPPED with the
   same-input-gemv family → RIDER on the nearest family entry
   (`same-input-gemv-weight-concat-at-upload`, entries_gpu_XX mod.rs:2308)
   carrying: the 13.2/67.8 negative cell + the ONE new trick (in-kernel
   pointer-switch virtual-row weight fold — concat semantics with no
   upload-time copy). Never standalone.

## Tasks

- [x] T1 — batch plan FIRST (this file; `.plans/.highwater` 129 → 130).
- [x] T2 — dedup greps (name variants): EFQ vs operand-precision (entries_gpu_34/40
      distinct-from rows confirm the boundary); split-K (covariance dual-note +
      CDNA MMVQ comment only — CLEAN); underfill (ZERO — fresh vocabulary);
      mincore/prefault (entries_05.rs:804 PLE-prefault + entries_06 B118 —
      ADJACENT, routing arm new); same-input-gemv family (mod.rs:2308 +
      2198 + cross-lane-column-packed-projections — rider target located).
- [x] T3 — 2 parallel quote-lane subagents: A = EFQ paper (6 quote groups + 4
      paper-vs-charter corrections); B = MTPLX pack (split-K rule + hyper rider
      + mincore rule, quote tables with file:line). DONE — both returned.
- [x] T4 — caller spot-checks of load-bearing quotes (Batch-31 discipline):
      steel_qsa_sparse_gqa_decode.h:13-20 + 42-52; qsa_sparse_gqa_decode.cpp:296-312;
      qwen4_m4_hyper_read.py:14-24; ple_row_gather.py:93-103; qsa_sparse_decode.py:44-50.
      ALL VERBATIM. DONE.
- [x] T5 — author `entries_gpu_42.rs` (2 kernel_opt rules with distinct-from rows +
      underfill dual-source in rule 2's regime) + wire mod.rs (decl + concat +
      slice counts + Batch-145 history paragraph + 614 doc comment) + the
      mincore rust_perf entry (entries_06.rs — the current tail file; caching
      category) + rider on the same-input-gemv entry (entries_gpu_07.rs) + the
      reciprocal cross-ref rider on split-k-two-pass-attention-decode-long-context
      (entries_gpu_18.rs). DONE — plus the DEDUP CORRECTION: the charter's
      "split-K dedup CLEAN" was a truncated-grep artifact; the Batch-105 dense
      two-pass rule exists, so the rule was scoped to the delta (width-underfill
      trigger + sparse per-slot validity) and renamed
      `kv-split-decode-underfill-per-slot-validity`.
- [x] T6 — standing gates: `batch145_rules_present` / `batch145_concepts_covered` /
      `batch145_pseudocode_parses` (3 kernel gates, kernel_heal-gated) +
      `batch145_mincore_rule_present_and_annotated` (rust_perf) + count pins
      walked: tests.rs (corpus 612→614 ×4 + category table B 63→64 / F 14→15 +
      Batch-144 tail count + comment), router.rs (614), rust_perf tests
      (127 total / 36 NotBenchable / 70 annotated / 57 unannotated + serde +
      drafter-propagation 70-pins), AGENTS ×4 (kernel_opt 614 ×2, rust_perf
      127 ×2), README ×3 (kernel_opt 614 + tree comment + rust_perf 127/70),
      `.docs/11_domains/kernel_opt.md` (row + ledger paragraph),
      `.docs/11_domains/rust_perf.md` (corpus header + 70-of-127 + split-time
      note), `.docs/11_domains/README.md` (127). DONE.
- [x] T7 — (quiet window arrived mid-cycle: the seal-game-editor sibling test
      exited, load 10.7→4.0) GOAT: G1 batch gates 4/4 (two phrase-pin casing
      corrections: "SAME (code, scale) operand" + "no-empty-split clamp"); G2
      kernel_heal lib 1877/0 + default 1531/0; retrieval-precision floors HELD
      byte-identical (kernel self-variants 100%, blended op-bigram 94.7/96.9,
      clippy top-1 89.6% ≥ 80% bar — no re-pin); G3 clippy 0 on the write-set
      in default / kernel_heal / all-features (3 PRE-EXISTING
      `unnecessary_hashes` warnings in orphan_report.rs are the sibling's
      landed `cbb0e40a` — noted, not mine); score-bench FLAT 96.2 row #117 +
      frontier snapshot appended (gen 8). En-route doc-lint lesson: a `+` at
      line start inside `//!` history paragraphs is a markdown list bullet
      (28 doc_lazy_continuation warnings from one character). DONE.
- [x] T8 — research note `.research/171_batch145_metal_shape_pack.md`
      (`**Status:** RECORD`) + AGENTS.md corpus-count ×4 + snapshot queue update
      (the staged charter block → LANDED with the three admission corrections) +
      `.raw/mtplx` removal + commit + push. DONE.
- [x] T9 — plan close-out: tasks marked, `.plans/.highwater` = 130 (written),
      `.research/.highwater` = 171 (written), queue-snapshot live-state in the
      same commit. DONE.

## Non-goals

- No riir-ai / riir-gpu / league edits (read-only citations; owning lanes own their code).
- No kernel_verify wiring (STUB stays; the G1 content is quote + parse gates).
- The two host trims (`_stack_hot_rows`, `uniq.tolist`) stay UNCLAIMED (charter).
- Walk #9 league-intel rows have no rule-shaped content (charter).

## Hold/unblock

T5–T7 build-gated on box load: the seal-game-editor sibling test saturated the box
at cycle start (load 10.7–15.9); at T4 time load fell to 4.01/7.33/9.82 and the
test binary exited — proceeding, checkpoint if load re-saturates. Use an isolated
`CARGO_TARGET_DIR=/tmp/b145` for the gate runs (sibling katgpt-rs agents active).

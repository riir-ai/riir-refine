# Research 144 — ds4 PR #991 delta verdict: the calibrated IQ2_XXS build commit (`c5ae2eb5`)

**Status:** RECORD — Batch 121 landed 2026-09-07 (3 rust_perf io rules, corpus 102 → 105; commit on `develop`; `.raw/ds4-metal` removed at landing).

## Verdict: YES — B+ corpus (build-integrity + measurement-design), NO kernel mapping

The PR head moved past the Batch-118 pin at 2026-09-06T18:42:55Z (the snapshot's
own trigger): one LINEAR commit `c5ae2eb5` on top of `236cb2a5` — parent ==
the Batch-118 pin, so B118's quote pins remain valid and the delta is exactly
this commit. Upstream `antirez/ds4` main unchanged at `9ab70534` (the B117
pin). MIT (re-verified in the fresh clone at the pin).

**6,734+/3− across 5 files:** `gguf-tools/qwen4_iq2.py` (NEW 181 ln),
`speed-bench/qwen38-iq2-quality.md` (NEW 62 ln),
`speed-bench/qwen38-iq2-quality-results.json` (NEW, 6,435 ln),
`docs/QWEN38_FLASH_NEXT.md` (+15/−3), `gguf-tools/README.md` (+41).
No kernel code — the delta is BUILD TOOLING + MEASUREMENT METHODOLOGY.

## What was mined (3 rules + 1 rider, all rust_perf io, all NotBenchable-annotated)

1. **`resume-revalidates-provenance-before-reuse`** — resumable multi-part
   builds revalidate every identity input (source payload, calibration
   imatrix, the tool/library hash) against the on-disk manifest before
   reusing cached parts, and re-digest each part before skipping
   (`qwen4_iq2.py:29-46,49-52`). Failure class: presence-keyed resume mixes
   outputs across tool versions invisibly.
2. **`write-verify-readback-before-publish`** — hidden `.incomplete` write,
   per-part template guards BEFORE any byte, independent-reader hash verify
   of every payload, atomic-rename publish + sidecar manifest,
   refuse-existing (`:78-160`). The independent reader is load-bearing:
   verifying through the write path confirms the cache, not the disk.
3. **`ab-isolate-one-component-by-baseline-byte-copy`** (measurement honesty)
   — byte-copy every non-under-test tensor from the baseline artifact so the
   artifact diff IS the variable (96 trunk gate/up tensors swapped, all else
   verbatim from the Q4_K build); paired per-case diffs over raw counts;
   confounded recipes (Q8 = down weights too, Q4_0 = earlier recipe) named
   and kept out of the headline; un-instrumented memory counters (peak RSS
   vs mmap-backed GPU weights) reported as PARTIAL budgets, labelled.
   RIDER folded in: calibration provenance (same pinned imatrix as the
   baseline; zero-count entries take a deterministic fallback).

## League intel (named-not-edited — sibling acts)

- **Quantization economics for the Flash-Next watch row:** hard-suite passes
  FLAT (IQ2 36/50 vs Q4_K 35/50, same evaluator/kernels/seed on M3 Ultra)
  while BF16-referenced drift is 3–6× worse (NLL 0.27828 vs 0.21287, top-1
  91.32% vs 96.57%, MAE 0.13992 vs 0.04307). Headline metrics hide
  quantization drift; paired probability comparisons expose it.
- **64 GB feasibility is UNPROVEN by the source's own accounting:** 50.34 GB
  model + 2.92 GiB tracked runtime = 53.48 GB partial budget on a 512 GB
  box — "does not validate Metal allocation limits or OS headroom on a
  physical 64 GB Mac." If the league ever arms a Flash-Next row, IQ2 is the
  candidate recipe, pending real-64GB validation.
- riir-ai Issue 769 (ANE): no ANE content in this delta — untouched.
- MTP note: ordinary/MTP greedy parity byte-for-byte on a short-prompt smoke
  (88.9% draft acceptance, 120/135) — consistent with our closed MTP
  economics (Issues 717/721), no reopen.

## Dedup record

`commitment-hashing` family (gpu_03) = digest INPUT design — distinct from
resume IDENTITY checking. Quantization placement family (gpu_08 state-horizon,
dequant placement) = runtime placement — distinct from calibration workflow.
io family (B94 coalesce, B118 parallel-pread) = access patterns — distinct
from build orchestration. No collisions; the three rules are new axes
(build-resume identity, publish integrity, A/B artifact design).

## Gates (Batch 121 GOAT)

- batch121 gates 3/3 FIRST-SHOT (rules_present, pseudocode_parses, concepts_covered
  incl. verbatim source-phrase pins)
- lib default 1159/0 (was 1156; +3 gates; count pins walked: total 102→105,
  annotated 45→48 ×2, serde 45→48, io 6→9, header member-list +3, benchable
  ratio line 87/102→87/105, doc line 101→105)
- kernel_heal+latent lib 1589/0/2i (floor 1574 held)
- retrieval re-run: kernel 100/100/100, blended 94.7/96.9/97.7 of 131 —
  byte-identical to Batches 118/119
- score-bench FLAT 96.2/1.9/100 (row appended)
- clippy `-D warnings` clean in default + kernel_heal lib states
- Isolated `CARGO_TARGET_DIR=/tmp/b121_target` (sibling session gating in
  riir-chain concurrently); removed at landing.

Quote re-verification: all key phrases grep-verified against the fresh clone
at the pin before authoring (T0 of Plan 101).

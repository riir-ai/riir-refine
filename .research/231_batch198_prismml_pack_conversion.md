# Batch 198 — the PrismML fork pack: conversion record + quote verification

**Status:** RECORD (landed 2026-10-02, shikuwa/4090, riir-refine idle loop — Plan 196)

Source: `PrismML-Eng/llama.cpp` (MIT), read at the standing vendor clone
`E:/git/vendor/prismml-llama.cpp` @ tip `88c4bc60b9`. The pack was staged TWICE
with independent quote verification (M3 intake 2026-10-01 + the 4090 watch-pass
distill 2026-10-02 — `.distill/001` rows); this batch re-verified every
load-bearing quote at the pins a third time before authoring.

## Landed

**+5 kernel_opt rules + 1 rust_perf rule + 3 instance riders, corpus 734→739
(kernel: A 156→157, B 77→78, D 163→165, N 56→57; rust_perf 167→168).**

| rule | cat | pin | grade |
|---|---|---|---|
| `fewrow-tensor-tile-decode-into-rhs-operand` | B | `3285e757a6` #284/#286 | A− GOAT lead |
| `weight-decode-amortized-across-output-columns` | A (adjudicated over staging-B) | `3285e757a6` #284 | B |
| `ids-path-rebinds-stride-meaning` | D | `88c4bc60b9` #258 | B− correctness, NotBenchable |
| `tied-hadamard-output-fold-at-load` | N (adjudicated over D) | `0781925904` #257 | B− decision content |
| `single-seq-slice-contiguity-copy-elision` | D | `3285e757a6` #284 | C+ kept for decision content |
| `selector-lattice-beam-over-greedy-walk` | rust_perf complexity | `3285e757a6` #284 | C+ (NOT-ISOLATED caveat carried) |

Riders: gpu_09 `simdgroup-row-packing-reduction-amortization` += `445fa82098`
#292 (the ggml-Metal GDN 4-rows-per-simdgroup instance — the rule's THIRD
independent backend: mlx-lm Python origin → ds4 riders → ggml Metal, full
bit-identity contract reproduced); gpu_53 `dequant-once-macrotile-stage-reuse-across-microtiles`
+= `78e16408f2` #254 (the AVX-512 single-token FA tile instance — KV rows
staged once for all G q heads) + `maddubs-overflow-two-exact-mac-selector`
+= `bda59eaa03` #256 (the PQ2_0 VNNI TR=8 instance — the dpbusd no-overflow
pole with the precomputed `cy` row-sum offset correction).

## Quote verification (T1 — the third pass)

Coordinator + both authoring subagents re-read every pin with `git show`:

- **#258 `88c4bc60b9`**: commit message verbatim ("With ids, stride_col_dst
  is the token stride... failed in 8 of 10 runs on an RTX 4090") + the guard
  `const uint32_t nrows_dst = ids ? stride_channel_dst : stride_col_dst;` +
  both predicate sites — all present, all quoted exactly.
- **#284/#286 `3285e757a6`**: "no threadgroup staging and no barriers" +
  "the generic tensor mul_mm tile is 64 x 128... mul_mv_ext is ALU-bound at
  2..8 columns" + "n=1..2 are slower on the new tile (150 vs 71 and 130 us),
  hence min 3" + the FEWROW env knobs + the weight-decode floors comment +
  the dflash contiguity comment + the beam code (env band 2..4, log-softmax,
  partial_sort) — all verbatim.
- **#257 `0781925904`**: the schema-3/tied_output manifest gating + the
  writer refusals + the GGUF key `prism.hadamard.tied_output` — verbatim.
- **#292 `445fa82098`**: `LANES_PER_ROW = 8`, `SEG = NSG*(32/LANES_PER_ROW)`,
  `is = lane*SEG + j` — verbatim.
- **#254/#256**: ops.cpp +177L / repack.cpp +101−47; the instance lines the
  riders quote were read at the pins by the authoring subagent.

**Quote corrections vs the staging rows: ZERO.** Two brief-level corrections
recorded in the entries file's module doc instead: the staged
`few-row-m-amortized-weight-streaming` kin reference is a DROPPED staging
lead (not a corpus rule — carried as a conversion note, never a KINSHIP), and
the brief's `quantized-stored-bytes-not-widened-patterns B196` citation does
not exist in the corpus (the actual load-time-transform precedent used is
B176's `dtype-native-source-read-with-shared-admission-predicate`). The
beam rule's two verify-or-drop claims (NaN→−inf at speculative.cpp:2052,
lexicographic slots tie-break :2058) were VERIFIED PRESENT and kept.

## Category adjudications (stated in-rule, coordinator-rebucketable)

- weight-decode → A (the corpus A family owns amortize-decode-over-reuse:
  `dequant-once-per-k-tile-into-shared-memory`, the B186 macro-tile stage).
- tied-hadamard → N over D (a stored-representation decision with a metadata
  contract — the B176 class — not a launch/routing decision).
- copy-elision stays D over L (the guard is a route-selection predicate; L's
  contiguity-guard pattern was considered and declined).

## Gates (all green)

- batch198 parse gate (5/5 ref+opt parse) + contracts gate (5/5 markers) +
  riders gate (3/3 hosts) — `batch198_pseudocode_parses` /
  `batch198_concepts_carry_their_load_bearing_contracts` /
  `batch198_riders_landed_on_their_hosts`.
- kernel_opt lib **2830/0**; default lib **2424/0** (== the B196/B197
  baseline — the new rules are opt-in); rust_perf corpus slice 284/0.
- Count pins re-pinned with honest labels: tests.rs ×4 (category table A157/
  B78/D165/N57, totals 739 ×2, the corpus-count asserts) + mod.rs category
  slices + headers + the exhaustive-comment + router.rs:472 + AGENTS.md
  ×2 + kernel_opt.md + rust_perf.md + README ×2.
- rust_perf annotation pins: 167→168 total, annotated 113→114 (the
  NOT-ISOLATED caveat), unannotated UNCHANGED at 54 (the new rule is
  annotated — one early 53/115 arithmetic slip caught by the pins and
  repaired with the honest label), without_spec 68→69 (bench_spec None).
- Retrieval floors **21/21 HELD** (222s, isolated target dir, removed).
- Clippy `-D warnings` clean at 3 postures (kernel_opt / kernel+perf+latent /
  default; the 9 doc_markdown mechanicals in the two new files' headers
  healed by hand — the B192 class; the residual "1 warning" is the known
  build-script hardlink box artifact, non-denyable, recorded since B196).
- Freeze + score-bench NOT owed (zero clippy-corpus delta, P30).

## Method note (the subagent-commit deviation)

Both authoring subagents committed + pushed their own slices directly
(`de420973` rust_perf entries_12.rs; `8a003c90` kernel entries_gpu_66.rs +
the two rider files) instead of leaving them uncommitted for one batch
landing commit — the briefs did not forbid it. Disclosed rather than
rewritten (pushed history is immutable per discipline); the batch landing
commit carries the registration + gates + pins + docs. Future batch briefs
should state "leave uncommitted" explicitly.

## Queue after

- The TensorFold B/B−/C+ pool (15 rules + instances — leads 5/6/7/8/11/14/
  16/18/19/20/23 + the kernel items 9/10/12/13/15/17/21/22/24 + the 10
  instance filings) — the largest staged pool, next slot(s).
- The MTP-slice leads (12 verified, batch-ready per Research 229).
- The nnethercote `dataflow-fixpoint-convergence-traversal-order` lead
  (min-rank worklist, precision-corrected wording) — next rust_perf slot.
- The +19 fork lead residue: none unconverted (the MUL_MAT_ID D-rule,
  the fewrow pack, the beam, the contiguity, the tied-Hadamard, and the
  CPU twins all landed here; #261/#272 remain skip-class infra).
- Clocks: walk #13 ~10-05, soak read ~10-07.

# Research 240 — Batch 205: the 10-06 unstaged kernel pool (llama.cpp ×4 + MLX ×2)

**Status:** CONVERTED — 2026-10-07 (Plan 205; all six members graded at the pins and landed as 8 kernel_opt rules; corpus 761 → 769: D +3, B +4, F +1).

**Box:** shikuwa/4090 (riir-refine idle loop, post-Batch-204).
**Staging provenance:** riir-ai `.benchmarks/rematch/2026-10-06_watch.md` (the 10-06 first-read grades, "quote verification owed at batch composition" — this batch paid that debt); the queue position came from `.distill/001_mining_queue_snapshot.md` (the BATCH-204 row).

## Sources (all MIT; clones `.raw/b205-llamacpp` + `.raw/b205-mlx`, removed at close)

| Sha | Repo | Subject | PR | Disposition |
|---|---|---|---|---|
| `9d3aba6b5e10a6dd086bd0555bd12c00366a8916` | ggml-org/llama.cpp | CUDA: use MMVF for thin f16/bf16 mul_mat at small batch size | #29633 | CONVERTED → `sibling-kernel-admission-gap-absorbed-by-generalist` (D) |
| `d89651a7b205c03c4a0b13cd0646d400dc929f79` | ggml-org/llama.cpp | CUDA: prefer whole-tile FlashAttention scheduling for efficient two-stage kernels | #29435 | CONVERTED → `work-splitting-requires-work-shape-homogeneity` (D) |
| `1b43d311698269073fb4827db10f9d6f195d2e7d` | ggml-org/llama.cpp | cuda: tile the lightning indexer over keys and tokens for 4 heads | #29901 | CONVERTED → `too-few-channels-token-tile-broadcast-ownership` (F) + `partial-unroll-bounds-register-spill-cliff` (B) — one source, two axes |
| `8b2fbaf32cad6fbe08cf85348dbbb84d8e3c70f0` | ggml-org/llama.cpp | CUDA: Optimize accumulation in mmq for NVFP4 type | #29857 | CONVERTED → `mma-accumulate-into-resident-accumulator-c-operand` (B) |
| `f8aaf49d4a7e4c0ccd31986b45b69d791f925ed8` | ml-explore/mlx | Use the fast qmv kernel for outputs not divisible by 8 | #4516 | CONVERTED → `tail-clamped-row-reuse-widens-fast-kernel-admission` (D) + `auxiliary-sum-widens-at-add-sites-against-cancellation` (B) — one source, two axes |
| `0e3ff3643b1c3719f78814b98e0d222afbad867c` | ml-explore/mlx | Unroll TK=4 keys per iter in sdpa_vector_2pass_1 | #4596 | CONVERTED → `strided-loop-batched-prefetch-unroll-tail` (B) |

**DEDUP dispositions (the watch-row riders):**
- `a3a1c47` (upstream metal few-row MMA #29869): the B196 fewrow family's upstream instance — **NO rule**; the family is owned by `fewrow-tensor-tile-decode-into-rhs-operand` (entries_gpu_66). Family instance recorded here only; the M3 decode razor stays watch-only per the standing Issue-890 Ultra-bound.
- fork member `6bfcd79a2` (NEON PQ2_0 vec_dot): landed in Batch 204 — not re-mined.

## Quote verification (the Batch-31 discipline)

All quotes in the eight landed rules were re-verified **verbatim** at the pins — the new-file line numbers were read from `git show <sha>:<path>` inside the ephemeral clones, NOT derived from diff hunk headers (the subagent drafts derived lines from hunks; the coordinator verified every one at the real files before landing). Corrections caught by that pass:
- mmvf.cu: the F16/NVIDIA complement is :830-832 (comment :830, guard :831, return :832), BF16 twin :863-865, AMD RDNA complements :845-846 / :878-879 — as drafted.
- quantized.h: the partial-rows comment is :790-793 (the draft said :792-795 — corrected).
- sdpa_vector.h: main loop at :272-274 (staged "~:272-276" — kept as a region cite), tail at :331-332.
- lightning-indexer.cu: TILE_TOKENS :240, contract comment :388-391, bank pad :418-419, `#pragma unroll 8` :487, the two read sites :492/:496, dispatch :680/:700.

**Count correction (the pins did their job):** the R1 first draft said "eight sites" — counted from the REMOVED `- return ne11 <= 8;` lines (8). The diff greps measured **TWELVE** added `return ne11 <= MMVF_MAX_BATCH_SIZE;` lines: the 8 replaced fall-through literals + **4 NEW returns inside the complement guards** (:832/:846/:865/:879). Corrected in-rule with the honest count; the batch205 contracts gate asserts the "TWELVE sites total" needle.

## Grades (the 10-06 first-read grades, held or revised at conversion)

- #29633 MMVF: B+ (upstream-merged, a dispatch-coverage defect shape with a structural drift defense — the warp_size threading). No published figures.
- #29435 whole-tile FA: B (the guard is a one-SKU pin — the class is portable, the constant is not; disclosed in-rule).
- #29901 lightning indexer: B+ for the tile design (F), B+ for the register repair (B — the 36× is source-reported, R9700+gfx908, never re-measured here).
- #29857 NVFP4 mmq: B (crisp placement rule; no numerics claim in-source — flagged UNVERIFIED bit-identity in-rule).
- #4516 MLX qmv: B+ (the padded-reference exact-equality proof is the model admission-widening contract) + B for the aux-sum fixture (a textbook exact-cancellation proof).
- #4596 MLX sdpa TK=4: B (structure + register-budget law; no figures — the added bench harness publishes no results in-commit).

ALL rules carry `NotBenchable` (the standing kernel_opt posture — GOAT gate pending repo-wide; no sm120+ host for NVFP4, no gfx908/R9700, no DGX Spark, no in-corpus Metal bench harness).

## Dedup greps (recorded per-rule in-rule)

Every rule's corpus greps ran over `src/domains/kernel_opt/corpus/` with 3+ vocabulary variants; the near-neighbor sets are cited in-rule (B103-addendum, B196 fewrow, B127 streamk-starved, B102 stream-k segments, B111 calibration-arch, B142 periodic-promote, B85/B23/B56 prefetch family, entries_gpu_12's ragged-tail pair + dense-pad rider, entries_gpu_66's column-clamp members, entries_popcorn's grid-stride/unroll-fixed/upcast-before-reduction). All 25 cited kinship rule_ids verified present in the corpus before landing. The two same-batch pairs cross-distinct each other (R1↔R6 two-kernel vs one-kernel admission widening; R4↔R5 tile design vs register repair from one source).

## Gates (all green)

- batch205 parse+contracts 2/2 (8 ids, both sides parse; 38 needle markers + NotBenchable + batch/plan stamps + the cross-distinct pairs).
- kernel_opt+latent_retrieval lib 3070/0 (includes the 761→769 count pins: tests.rs ×4, router.rs ×1, corpus/mod.rs category headers + KERNEL_CATEGORIES slices).
- default lib 2471/0.
- clippy `-D warnings` ×3 postures clean (default / all-features / no-default).
- retrieval 21/21 floors HELD (231 s, no decay, no re-pin).
- freeze/score-bench NOT owed (zero clippy-corpus delta — kernel_opt only, P30).

## En-route repairs (disclosed)

1. **Pre-existing all-features clippy red at HEAD `925f73db`** (NOT from this batch): `tests/corruption_corpus_freeze.rs`'s `Verdict::Signal` is constructed only under `#[cfg(unix)]` — on Windows the variant is dead code and `-D warnings` reds the all-features × all-targets posture. The B204 "clippy ×4 postures clean" claim had not covered this test target. Repair: `#[cfg_attr(not(unix), allow(dead_code))]` with the platform-axis reason at the site (the platform-dead-code class; the variant stays load-bearing on unix).
2. 4 `doc_markdown` mechanicals in `rust_perf/corpus/tests.rs` (the Batch-204 comment) healed by the riir-refine healer (`--fix --write --verify`, compile-gated).

## Queue after

- the 10-07 staged leads (`29442` cuda BF16/FP16→f32 conversion chunking, `28782` per-thread stream memset) rise one slot each — full quote verification owed at THEIR batch composition.
- nnethercote R134 next rust_perf slot (none staged).
- walk #14 opens Tue 10-13.
- the Research-239 follow-up pool (same-file `q2_0_q8_0` origin, ggml-org upstream arm quants, karusrus i8mm branch — pin before reading, SYCL XMX `a14c7de9` watch-only) stays in intake.
- Plan 204 T11 (the M3-Max GOAT for the B204 rule) stays deferred to the M3 idle loop.

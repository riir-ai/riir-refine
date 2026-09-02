# Plan 060 — PrismML fork Metal quant-MV distill (Batch 96)

Status: DONE — Batch 96 landed 2026-09-02 (4 rules, corpus 422 -> 426, N 24->26 / D 93->94 / B 54->55; GOAT all gates PASS per T7). Mining armed by the league opponent-watch intake (riir-clippy `a749024`, riir-ai Issue 843 T5); the arxiv re-walk (walk #4) completed 2026-09-02, so this was the next idle-loop batch per the queue note.

Source: `E:/git/vendor/prismml-llama.cpp @ e311ed38f` (the league-repinned fork tip; verified ZERO PQ2_0 in `ggml-metal` at the old pin `9ca265a` — the 1.4x prefill jump is REAL, from native PQ2_0/Q2_0/TQ2_0 Metal kernels + set_rows wide-row + FWHT weight-fold). Apache-2.0. Corpus record: this batch. Research record: `.research/114`.

## Scope (4 rule candidates, all source-read at mine time)

1. **R1 (N)** — `telescoped-digit-coefficients-byte-fma-dot`: the TQ2_0 mul_mv kernel scores a 2-bit ternary-class block WITHOUT unpacking fields: the packed byte rides as an exact integer-valued f32, digit extraction is `floor(v * 4^-k)` (power-of-2 scale => exact), and four pre-combined per-byte y-coefficients (`c0 = y0`, `c_k = y_k - 4*y_{k-1}`) telescope the positional decomposition so the byte contributes 4 FMAs total. The codec's -1 offset is absorbed by seeding the accumulator with the row-shared `-sumy`. Coefficients hoisted across the row loop (computed once per byte-column + y-slice). Upstream lineage: ggml-org/llama.cpp PR 26980 (cited in-source).
2. **R2 (N)** — `bit-plane-plane-sums-input-side-offset-dot`: the Q2_0/PQ2_0 dot scores as two POSITIONAL bit-plane sums (`acc_lo + 2*acc_hi`) with the offset plane on the INPUT side: `sumy = sum(y-slice)` is computed once per y-slice, shared across all weight rows, and the block scale applies once at the end: `d * (acc_lo + 2*acc_hi - sumy)`. Branchless `select(0, y, bit)` conditional adds from raw byte masks — no unpack to weight values anywhere.
3. **R3 (D)** — `wide-row-copy-tile-rows-across-workgroups`: `kernel_set_rows_f32_wide` — the generic set_rows assigns ONE threadgroup per row, which "cannot saturate the memory bandwidth for rows of 100k+ elements" (in-source comment; recurrent-state snapshots). Tile the row ACROSS workgroups (2D grid: intra-row tile x row, dims folded into grid.z) with float4 accesses; caller-side width threshold routes wide vs generic.
4. **R4 (B)** — `fwht-shuffle-xor-butterfly-register-resident`: `kernel_fwht` applies the Walsh-Hadamard transform fully in registers: NE = N/32 strided elements per lane, cross-lane butterfly stages via `simd_shuffle_xor(val, i)` (the xor mask IS the partner; sign from `(lane & i) == 0`), remaining stages in-register per lane, ZERO smem + ZERO barriers, constant `1/sqrt(N)` folded into the initial load. N in {64..2048}, regime N <= lanes x register budget.

Riders (no corpus edit): B20/B92 byte-perm family gains EXTERNAL MEASURED confirmation on a third substrate (fork `a7cc29645`: HIP `v_perm_b32` symbol expansion; MI300X Q1_0 112->322 tok/s @4B, 81->269 @8B, 31.5->97 @27B; PQ2_0 113->259 @8B, 48.6->91.7 @27B — source-measured, bit-exact gates per the commit); CDNA-vs-RDNA geometry regime note (keep split-K MMVQ geometry on CDNA where the small_k geometry "runs several times slower per call"). PQ2_0 = the same 2-bit codec at group-128 (8 sub-blocks x 16 weights) — codec-design note (format decision, no bounded consumer fix-space).

## Dedup (adjudicated against full rule texts at mine time)

- `recursive-kronecker-cat` (B) = Hadamard MATRIX CONSTRUCTION via recursive cat — R4 is the transform-application side (butterfly network), distinct + composing.
- B95 `simdgroup-row-packing-reduction-amortization` + `whole-sequence-zero-barrier-single-dispatch-scan` = REDUCTION-scoped butterflies (many-to-one) — R4 is a full-exchange TRANSFORM (all lanes emerge with distinct outputs).
- B17 `split-sign-accumulators-defer-subtraction` = ternary SIGN planes (pos/neg of +-1) — R2's planes are POSITIONAL digits of an offset codec with the subtracted mass on the INPUT side (B17's shape cannot express Q2_0's `(w-1)` codec).
- B92 `prmt-int8-to-half2-dequant` + B44 `bit-cursor-unpack` = EXTRACT-then-consume recipes — R1 eliminates extraction entirely.
- `norm-decoupled-low-bit-scoring` (N) = norm x dot reconstruction — composes with R2, different axis.
- E-family (`row-dot-split-inner-axis-block-reduce`, `grid-over-output-thread-private-sweep`) = reduction splits — R3 has NO reduction; the axis is machine-level workgroup concurrency for a pure streaming op.

## Tasks

- [x] T0 source verification: fork at `e311ed38f`, `mul_mv.metal` q2_0/pq2_0/tq2_0 kernels + `quantize.metal` set_rows family + `misc.metal` fwht + codec headers read; provenance commits `73f0fed04`/`e8b6452b1`/`e0c828b90`/`a7cc29645` read.
- [x] T1 dedup pass vs full neighbor texts (recursive-kronecker-cat, B95 x3, B17, B92 prmt/lop3, B44, norm-decoupled, E-family) — verdicts above.
- [x] T2 write `.research/114_prismml_metal_quant_mv_distill.md` (verdict, quotes w/ line anchors, measured numbers, rider dispositions).
- [x] T3 author `entries_gpu_11.rs` (4 rules, B94/B95 entry shape) via Python generator (ensure_ascii=False, rustlit, real-newline sources).
- [x] T4 wire corpus: mod decl + extend + KERNEL_CATEGORIES B/D/N slices + category header comments (B 54->55, D 93->94, N 24->26) + mod doc count line.
- [x] T5 count pins: tests.rs total 422->426 (+ stale "423" message fix), expected table (Compute 55, Dispatch 94, DtypeAlignment 26), sum pins x2; router.rs `rules_for_expert_total_matches_corpus` 414->426 (+ stale "(400)" comment fix).
- [x] T6 gates: batch96_rules_present + batch96_concepts_covered + batch96_pseudocode_parses (cfg(not(kernel_verify)) per B70).
- [x] T7 GOAT: `cargo test --lib --features kernel_heal` + default lib + clippy both states `--all-targets` + retrieval floors (latent_retrieval) + score-bench run. Measured: kernel_heal lib 1248/0 (+3), default lib 1008/0, latent_retrieval lib 1109/0 with retrieval_precision_regression_gate PASS (floors HELD, no re-pins), clippy `--lib` kernel_heal + default + `--all-targets` kernel_heal all 0 (one doc-markdown round on the `//!` header — the B37 lesson, 9th strike: `PrismML`/`PQ2_0`/`FWHT`/bare-path backticks), score-bench #26 96.2/1.9/100/100 FLAT. One gate-catch: concepts_covered failed on a case-mismatched needle (POSITIONAL vs positional — the B51 case-sensitivity lesson re-proven live).
- [x] T8 snapshot queue update (mark mined; record disposition) + plan status + commit + push.

## Honesty notes

- R1 is tolerance-gated, NOT bit-identical: the digit EXTRACTION is exact (integer-valued f32, power-of-2 scaling), but the products/sums reassociate vs the unpacked kernel — the rule states both halves; never cite it as bit-exact.
- All quant tok/s numbers are the fork's own measured rows (HIP commit message + league context); the Metal kernels carry no published numbers beyond the league's 1.4x prefill jump attribution (Issue 843 T5).
- R2/R3/R4 fixture code is original minimal Rust; source quotes stay in the concept text with line anchors.

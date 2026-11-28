# Plan 421 — the mlx 83-commit window batch (kernel_opt + rust_perf)

**Status:** CLAIMED — the standing mining lane (zed-5426a3b4@amo, epoch 1795872400, expires 2026-12-01T01:00:00+00:00). Queue head per the B429 landing handoff + zed-f888bfac's staged watch row (the compare-API pass 2026-11-28). amo contended until ~12:15 ICT: the T0 clone + read half runs now; the compile-gated fixture cells go to rig jobs (`agent-mail rig run bmo --shared`) or the post-12:15 quiet window.

**Claimed:** zed-5426a3b4@amo, 1795872400, expires 2026-12-01T01:00:00+00:00

## Source

ml-explore/mlx — pin `c69303b0` (2026-11-03, the B410/B411/B412 base) → tip `266a9bd0`, **83 commits ahead** (measured 2026-11-28 via compare API by zed-f888bfac, docs-only). resin `157eb81d` and hunch `22cf73010b0d` both UNMOVED — no competing windows.

## Staged surfaces (diff-observed NAMES only — every quote re-verified in-source at the fresh T0 clone before any staging; the Batch-409 prior-coverage lesson: each candidate re-greps the corpus at T1)

- CPU `quantized.cpp` rewrite (~430 lines — f32 accumulators via a cached `qmm_accumulator`, scales re-encoding with `inv_scale_enc`, the `QQMatmul` general-case unquantized-w path)
- `row_reduce.cu` split-K second pass (`SPLIT_TARGET_THREADS` 2^17 / `SPLIT_MIN_ROWS` 16 / the combine kernel — the few-outputs split law)
- `simd/math.h` exp integer-bits trick (the `12582912.0f` = 1.5·2²³ magic anchor — the house magic-anchor family, gpu_136's anchor algebra + B251, and the `bias = 127 - 0x4B400000` unsigned-wrap form)
- `unary_ops.h` `narrow_round_to_odd` (double→float round-to-odd before fp8 — the double-rounding tie hazard with its 1.0625+2^-30 fixture)
- `mlx/compile/fuse.cpp` horizontal fusion (merge_vertical/merge_horizontal with byte-scored pair candidates + the is_connected cycle refusal — sibling groups)
- Metal `quantized_nax.h` affine int8 QMM (quantize-rows + int8 MMA + the group-boundary correction terms + split-K)
- `grouped_mm` cudnn MoE path
- the empty-array guard sweep (~30 sites)

## Tasks

- [x] T0 — DONE 2026-11-28: fresh clone at `9cb7f69b` (tip moved past the staged 266a9bd0 — 84 commits ahead of the c69303b0 base, MIT re-verified); all staged surfaces confirmed live in the range (188 files, +7088/−2308); two reader subagents delivered the candidate slate — RULES: round-to-odd-intermediate-narrowing (A−, numerics, joins B366's rounding family, the 1.0625+2^-30 fixture byte-exact) + few-outputs-split-second-pass-break-even (B+, kernel_opt, three-bound admission compose gpu_67/gpu_117) + accumulate-in-accumulator-type-narrow-once-at-store (B, ReduceAccum trait + RMW separation, B366-face compose) + per-group-reciprocal-scale-fold (B, benchable, lossy-regime condition) + two-sided-encoded-magnitude-matching (B, parity, cross-refs the reciprocal seam) + group-boundary-int8-mma-deferred-descaling (A−, kernel_opt, the int8 NAX lane confirmed absent from B410-412, gpu_136 compose) + class-relative-split-cap-min-partition-work (B+, duration-keyed split cap) + magic-anchor-bits-encode-via-unsigned-wrap (B−, anchor face + the UB half new); FACES: mma-operand-limb-hi-lo-precomputed-sum → B379-R2 (same Dekker limb, precomputed twist) + biased-code-to-signed-byte-parallel-decode → decode family (6-bit spread the new atom) + tiles-per-core-admission-floor → admission family (gpu_119/gpu_18) + the anchor half → gpu_136/B251/B242/gpu_20 family (same 1.5·2^23 constant); LANE INTEL (no domain, record-only): byte-scored-horizontal-fusion-with-cycle-refusal (fuse.cpp — genuinely new decision content, no compiler lane in the corpus); DROPS: sincos_checked (prior pin), QQMatmul validation throw, empty-tensor guards ×4 classes, quantize_impl alloc plumbing, arch-suffix detection, double-buffered staging (prior pin), bit_cast plumbing, fuse Groups merge/unmerge plumbing
- [x] T1 — PARTIAL 2026-11-28: the T0 readers ran the kin adjudications per candidate (in-rule compose/dedup notes banked in the T0 row); the load-bearing quotes spot-verified byte-exact at `9cb7f69b` by the lane (unary_ops.h:121-127 round-to-odd, math.h:65-72 the anchor + bias form, row_reduce.cu:289-296 + :316 the split constants + ReduceAccum, quantized.cpp:505-507 + :1251-1254 the reciprocal fold); the remaining T1 half re-greps per candidate at LANDING HEAD (corpus may move before the landing commit)
- [x] T2 — GRADED 2026-11-28 (routing decided): SEC Numerics family tranche (4 rules — round-to-odd-intermediate-narrowing [A− lead], accumulate-in-accumulator-type-narrow-once-at-store [B], two-sided-encoded-magnitude-matching [B], magic-anchor-bits-encode-via-unsigned-wrap [B− anchor-face + UB-half-new]) — the B366-family home `src/domains/sec/corpus.rs`, suggest-only, Detector: none-at-landing posture accepted; KERNEL_OPT tranche (3 rules — group-boundary-int8-mma-deferred-descaling [A−], few-outputs-split-second-pass-break-even [B+], class-relative-split-cap-min-partition-work [B+] + 4 count-neutral FACES: mma-operand-limb-hi-lo → B379-R2, biased-code-to-signed-decode → decode family, tiles-per-core-admission-floor → admission family, the anchor half → gpu_136 family); RUST_PERF tranche (1 rule — per-group-reciprocal-scale-fold [B, BENCHABLE], the batch's only benchable candidate, needs its Pareto fixture + validator cell); LANDING SHAPE: one batch, three domains, batch number re-derives at landing from the snapshot (B430+); the sec tranche authors first (next window), kernel tranche second, the rust_perf Pareto cell rides the bmo shared rig or the post-12:15 quiet amo
- [ ] T2 — grade + select (verdict table per candidate: domain routing kernel_opt vs rust_perf vs sec; benchability adjudication; the B298-bbtree slot law: nothing lands NotBenchable-only without a decision-content justification)
- [ ] T3 — author the rules (verbatim pin quotes + full context_notes + regime/anti-regime + both-directions adjudications) — the quote set is banked (T1 row); the corpus write opens on the quiet window (sec tranche first)
- [ ] T4 — fixtures standalone-compiled AND executed pre-entry (debug AND -O), the count gate wired
- [ ] T5 — honest drops recorded (names + reasons, in the plan + research note)
- [ ] T6 — gates: lib both postures + clippy -D ×3 + pins_manifest + count web one-landing + fmt + dual_allocation
- [ ] T7 — retrieval floors re-measure if the corpus moved; score-bench/freeze NOT owed unless clippy_lints delta
- [ ] T8 — records: research note (allocate .research/356+ per its highwater — check at write), landing row, lease fold
- [ ] T9 — teardown (.raw/mlx-window + /tmp dirs; df delta)

## Notes

- The surface list is zed-f888bfac's names from the compare API — the batch's own T0 is the authority. Nothing invented.
- yaml: the mlx repo is C++/CUDA/Metal — transcription-only posture, outbound-PR FAIL-CLOSED per licence (check LICENSE at T0; BSL/Apache per B410's prior transcription).
- amo contention: heavy verify legs ride `agent-mail rig run bmo --shared --est 30m`.

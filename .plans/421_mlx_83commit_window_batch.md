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

- [ ] T0 — fresh clone at a fresh pin (record the tip sha at clone time; re-verify every staged surface name against the actual diff; two reader subagents on the big files per the house T0 discipline)
- [ ] T1 — dedup re-grep per candidate at landing HEAD (vocabulary translation, both directions; the Batch-409 prior-coverage lesson)
- [ ] T2 — grade + select (verdict table per candidate: domain routing kernel_opt vs rust_perf vs sec; benchability adjudication; the B298-bbtree slot law: nothing lands NotBenchable-only without a decision-content justification)
- [ ] T3 — author the rules (verbatim pin quotes + full context_notes + regime/anti-regime + both-directions adjudications)
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

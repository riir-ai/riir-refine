# Plan 269 — Batch 265: the vllm Mamba2 host-overhead + llama.cpp graph/conversion wave

**Status:** IN FLIGHT — claimed 2026-10-20 ~22:5x +07 (shikuwa/4090, riir-refine idle loop), expiry 48 h (2026-10-22 ~22:5x +07); push is the arbiter.

**Sources (3 commits, all read at pin this session):**
1. vllm-project/vllm `e4a676cd` #61497 "[Perf][Mamba] Reduce host overhead in the Mamba2 Triton SSD prefill path" (merged 2026-10-20 08:28 UTC, Apache-2.0, 8 files +244/−126) — clone `E:/git/vendor/vllm`.
2. ggml-org/llama.cpp `13786fbb` #26771 "CUDA: Clear CUDA graphs after legacy memory pool flush" (merged 2026-10-20 11:30 UTC, MIT, 2 files +25/−6) — clone `F:/llama.cpp`.
3. ggml-org/llama.cpp `119f68d7` #30505 "ggml-cpu : select the NEON fp32->fp16 conversion" (merged 2026-10-20 11:35 UTC, MIT, 1 file +11/−6; cites B237's own source PR 30342) — clone `F:/llama.cpp`.

**Watch rows (no corpus):** vllm `bc01c069` #61601 (PLE conv-only checkpoint exporter — Qwen4Exp spec-decode lane intel, functional); MLX `bff1022` #4682 (staged affine_qmm_t_nax split-k — M3 NAX mechanism watch); llama.cpp `df117021` #30862 (metal argmax/pool_1d — M3 intel), `704366d7` #30699 (metal MoE 9-expert variant — variant-coverage intel), `b6844985` #30836 (HIP/CUDA mmf tile-width fix — thin), `a01ce9ff`/`8115b4c3`/`522f2b6e`/`84663953`/`dd002b91` (platform/NULL).

## Tasks

- [x] T0 — movement enumeration: watch poll `/tmp/watch_poll_1020c.log`, diff vs the tracked walk-#15 record = 15 new commits (10 llama.cpp + 4 vllm + 1 MLX); graded above.
- [x] T1 — dedup greps (Protocol A step 2): autotune-family hosts = `autotune-fidelity-synthetic-carries-producer-params` (K, entries_gpu_87) + K=AutotuneDispatch 18 members — the per-launch key-REBUILD cost axis is UNOWNED (P61 rider owns key CONTENT hardware-path; B223 owns key fidelity); graph-capture host = `per-request-dirty-range-clear-graph-capture-sticky-marks` (A, entries_gpu_105, B251) — the pool-free INVALIDATION axis is UNOWNED (B251 owns which ranges clear); never-materialize host = `moe-router-weight-fold-into-reduce-never-materialize` (entries_gpu_84, B220) — the slot-indexed write face UNOWNED; f16 conversion host = `f16-to-f32-accumulator-lut-breaks-conversion-serialization` (rust_perf entries_25, B237) — the defined-not-selected face UNOWNED (the coverage rider on `fused-vector-site-transcendental-form-nonportable` owns missing-ARM, not defined-but-unselected).
- [ ] T2 — author K own rule `autotuner-key-memoized-per-launch-not-rebuilt` (vllm #61497 `launch_autotuned`: "Autotuner.run rebuilds its cache key on every launch, matching each argument name against a list and formatting the dtype of every tensor argument, several microseconds of host time per launch"; memoize `(kernel, key-args, arg-dtypes) → best_config`, hit launches `kernel.fn[grid](**kwargs, **config.all_kwargs())`; autotune on first launch per key; the pre-hook refusal disclosed). New file `entries_gpu_115.rs`, category K (AutotuneDispatch).
- [ ] T3 — rider on B220 never-materialize (entries_gpu_84): the FUSED FINAL-STATE SLOT-WRITE face — the SSD state-passing kernel writes each sequence's final state straight into its cache slot (`slot = tl.load(final_state_indices_ptr...)` then store), the materialized `varlen_states` intermediate + the host-side `ssm_state[state_indices] = ...` scatter both vanish; consumer address is DATA (slot indices); the CPU-fallback posture disclosed ("Implementations without the fused write (CPU) return the states").
- [ ] T4 — rider on B251 graph-sticky (entries_gpu_105): the POOL-FREE GRAPH-SET INVALIDATION face — "captured CUDA graphs can still use freed buffers, so they must be captured again"; BOTH free paths invalidate (the explicit `free()` AND the OOM-retry `clear_pool()` before the retry alloc); the VMM pool exempt (virtual addresses persist across its free); the debug log names the captured count.
- [ ] T5 — rider on B237 f16-conversion (rust_perf entries_25): the DEFINED-BUT-NOT-SELECTED face — the NEON branch defined `GGML_CPU_COMPUTE_FP32_TO_FP16` (one `fcvt`) but selected only FP16_TO_FP32, so f32→fp16 silently fell through to the portable software conversion ("the only symptom is a hardware instruction nothing ever reaches"); the compile-time `#error` pair asserts defined-implies-selected PER DIRECTION; the `!defined(__F16C__)` carve-out preserves B237's own table-on-purpose direction (upstream cites issue 30342).
- [ ] T6 — wiring: `KERNEL_CATEGORIES` + K slice registration; count pins 923→924 at every site (tests.rs ×~5 + mod.rs doc + AGENTS.md + .docs/11_domains/kernel_opt.md); K 18→19 pin; batch265 needle gate (K rule needles + 3 rider needles).
- [ ] T7 — gates: kernel_opt lib (floor 3193 + new), default lib (2715), clippy −D ×3 postures (default/all/no-default), rust_perf corpus count NEUTRAL (rider only — rust_perf stays 241, its pins untouched), retrieval floors RE-MEASURE (own rule added).
- [ ] T8 — docs: Research 301 (`.research/.highwater` 300→301), AGENTS.md kernel_opt bullet + rust_perf rider note, `.docs/11_domains/kernel_opt.md` B265 row, rust_perf.md rider note, snapshot landing row (same commit).
- [ ] T9 — binary rebuild owed (rust_perf rider IS release-set surface — the §0 bin-age law) + `--version` verify; freeze/score-bench NOT owed (zero clippy-corpus delta, P30/B250).
- [ ] T10 — commit `feat: batch 265: ...` + push; ephemeral scratch cleaned.

## Notes

- Zero-GPU throughout (kernel mining, no benches); box quiet (CPU 8%).
- vllm #61497 measured numbers: PR body not yet read (source-reported numbers wanted at T2 authoring — GitHub API fetch; commit itself carries none).
- The watch artifact `2026-10-20_watch.md` was regenerated by the poll and RESTORED to HEAD bytes (the walk-#15 record stays canonical; poll evidence in `/tmp/watch_poll_1020c.log`).

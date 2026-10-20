# Research 301 — Batch 265: the vllm Mamba2 SSD prefill host-overhead wave + the llama.cpp graph/conversion riders

**Status:** RECORD — Batch 265 landed (Plan 269), 2026-10-20, shikuwa/4090, riir-refine idle loop.

## Sources (3 commits, every quote verbatim at the pin)

1. **vllm-project/vllm `e4a676cdc22f99e808093a87ff8509a6089d392d`** — PR #61497 "[Perf][Mamba] Reduce host overhead in the Mamba2 Triton SSD prefill path", Apache-2.0, merged 2026-10-20 08:28 UTC, 8 files +244/−126. Clone `E:/git/vendor/vllm`. PR body fetched (measured numbers source-reported).
2. **ggml-org/llama.cpp `13786fbb`** — #26771 "CUDA: Clear CUDA graphs after legacy memory pool flush", MIT, merged 2026-10-20, 2 files +25/−6. Clone `F:/llama.cpp`.
3. **ggml-org/llama.cpp `119f68d7`** — #30505 "ggml-cpu : select the NEON fp32->fp16 conversion", MIT, merged 2026-10-20, 1 file +11/−6. Cites issue 30342 (the corpus's own B237 source) in-source.

## The lead atom — `launch_autotuned` (own rule, K/AutotuneDispatch)

The Triton `Autotuner.run` wrapper rebuilds its cache key on EVERY launch — "matching each argument name against a list and formatting the dtype of every tensor argument, several microseconds of host time per launch" — even after the config is stable. The fix (`vllm/model_executor/layers/mamba/ops/triton_helpers.py`):

- key = `(kernel, *[kwargs.get(name) for name in kernel.keys], *[getattr(arg, "dtype", None) for arg in kwargs.values()])` — an EQUIVALENCE with the incumbent key semantics, not an approximation;
- miss → `kernel[grid](**kwargs)` (autotunes once) then caches `kernel.best_config` — ONLY `if kernel.best_config.pre_hook is None` (a pre-hook is a per-launch side effect the raw-fn bypass would silently skip; hook-bearing configs are never cached — the refusal clause is load-bearing);
- hit → `kernel.fn[grid](**kwargs, **config.all_kwargs())` — the wrapper class never executes again for a warm key.

Companion face: `vllm/platforms/cuda.py` `is_arch_support_pdl` gains `@cache` — the device-capability query answered once per process.

**Measured (PR body, the author's same-node claims, never our bar):** prefill SSD scan + final-state write 263 → 179 µs per Mamba layer ("about 1.9 ms less per mixed step with 23 Mamba layers"); the key memoization "accounts for 43 of the 84 µs saved per layer"; same-node A/B +1.0–2.5% OTPS/gpu at concurrency 4–64, flat at 1–2 and 128; end-to-end up to 1.08× OTPS/gpu on 1× B200 (Nemotron-3.5-Lightning-NVFP4-W4A4, MTP=2).

**League relevance:** the Mamba2 mixer runs EAGERLY in prefill and mixed prefill+decode steps, so its host overhead lands directly on step latency — the prefill axis, our highest priority. The Rust surface: any hot dispatch site rebuilding a HashMap key or formatting a config string per call where the tuned decision is stable per key.

## The riders

- **B220 `moe-router-weight-fold-into-reduce-never-materialize` += the OUTPUT-SIDE SLOT-WRITE face** (vllm #61497, `ssd_state_passing.py`): the SSD state-passing kernel writes each sequence's final state straight into its cache slot (`slot = tl.load(final_state_indices_ptr...)` then store) — the materialized `varlen_states` + the host-side `ssm_state[state_indices]` scatter both vanish; the consumer's ADDRESS is DATA; the CPU fallback posture disclosed ("Implementations without the fused write (CPU) return the states"); companion faces `do_not_specialize_on_alignment=["final_state_indices_ptr"]` (JIT-variant control) + the warmup widened to the FULL constexpr cross-product ("Warm up all code paths so none triggers JIT compilation during inference"). Source-reported: "4 fewer launches per Mamba layer".
- **B251 `per-request-dirty-range-clear-graph-capture-sticky-marks` += the POOL-FREE GRAPH-SET INVALIDATION face** (llama.cpp #26771): "captured CUDA graphs can still use freed buffers, so they must be captured again" — BOTH free paths invalidate (the explicit `free()` AND the OOM-retry `clear_pool()` before the retry alloc); the clear is counted loud ("pool[%d]: memory freed, %d captured CUDA graphs must be captured again"); the VMM pool EXEMPT (virtual addresses persist across its frees — the exemption IS the boundary).
- **B237 `f16-to-f32-accumulator-lut-breaks-conversion-serialization` += the DEFINED-BUT-NOT-SELECTED face** (rust_perf, llama.cpp #30505): the NEON branch defined `GGML_CPU_COMPUTE_FP32_TO_FP16` (one `fcvt`) but selected only FP16_TO_FP32 — "FP32_TO_FP16 falls through to the portable software conversion while neon_compute_fp32_to_fp16, a single fcvt, stays defined and referenced by nothing"; "the fallback silently supplies a correct software implementation, so the only symptom is a hardware instruction nothing ever reaches"; the guard is a compile-time `#error` pair asserting defined-implies-selected PER DIRECTION, with the `!defined(__F16C__)` carve-out preserving B237's own law ("PR 30343 keeps that direction on the lookup table on purpose" — upstream cites issue 30342). Distinct from the kernel_opt missing-ARM coverage face (the transcendental-coverage rider): THERE the arm is absent from the chain; HERE the arm exists and the selection line is missing.

## Watch rows (no corpus)

- vllm `bc01c069` #61601 — Qwen4Exp PLE conv-only prefill checkpoint exporter (functional/infra; Qwen4Exp spec-decode lane intel).
- MLX `bff1022` #4682 — staged affine_qmm_t_nax kernels with split-k (Apple NAX quantized matmul; M3-side mechanism watch).
- llama.cpp `df117021` #30862 metal argmax/pool_1d optimize (M3 intel); `704366d7` #30699 metal MoE 9-expert variant fix (variant-coverage intel); `b6844985` #30836 HIP/CUDA mmf tile-width divisibility fix (thin); `a01ce9ff` #30850 opencl small-batch gemm (Adreno); `8115b4c3` #30812 sycl memset ordering; `522f2b6e` #30512 zDNN FA; `84663953` #30854 hexagon toolchain; `dd002b91` #30271 sycl warnings — platform/NULL.

## Gates (all run this session)

- batch265 needle gates 2/2 FIRST-RUN PASS (kernel_opt + rust_perf).
- kernel_opt lib 3203/0; default lib 2724/0.
- Retrieval floors 21/21 HELD no re-pin (321 s, isolated CARGO_TARGET_DIR `/tmp/b265_target`).
- clippy −D ×3 postures clean (hardlink-cache infra noise only, body-verified).
- freeze/score-bench NOT owed (zero clippy-corpus delta, P30/B250); binary rebuild OWED (rust_perf rider is release-set surface).

## En-route defects (both caught pre-landing, recorded for the class)

1. **The adjacent-string-literal splice defect (the B261 class, second occurrence):** the coordinator's first count-pin edit inserted the B265 message prefix as a SECOND string literal inside a multi-line assert — 23 compile errors (unterminated character literal at the `'[cuda]'` inside the now-orphaned tail). Recovery: `git checkout -- tests.rs`, then ONE idempotent python pass (count tokens by shape + message prefixes + witness notes + the K tuple), verified by the gates themselves. LESSON: count re-pins across 10+ message sites are a SCRIPT job with shape asserts, never a fuzzy-edit job; the edit tool's fuzzy match duplicates long identical lines instead of refusing.
2. **The router pin site OUTSIDE corpus (the B259 lesson, re-confirmed):** `kernel_opt/router.rs` `rules_for_expert_total_matches_corpus` carries its own 923 total — the corpus-dir sweep misses it; caught by the lib test at 3202/1, repaired to 924.

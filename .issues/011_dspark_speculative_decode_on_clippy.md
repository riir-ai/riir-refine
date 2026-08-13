# Issue 011 — Measure DSpark speculative decode on clippy prompts

**Type:** proof / measurement
**Status:** RESOLVED 2026-08-13 — measured; DSpark is a net **slowdown (0.55×)** on the current checkpoint. See [Bench 012](../.benchmarks/012_dspark_speculative_acceptance_clippy.md).
**Filed:** 2026-08-12
**Evidence base:** [Bench 010](../.benchmarks/010_llamacpp_vs_katgpt_ternary_clippy_tps.md) — raw forward, **no speculation enabled**

## The question

Bench 010 measured the **raw** ternary forward: 17.85 tok/s decode vs llama.cpp's
31.21 on identical clippy prompts (1.75× behind). Neither arm used speculative
decoding.

**DSpark** (entropy-based skip-verify, modelless — riir-ai Proposal 018) raises
*effective* tok/s by accepting several drafted tokens per verified forward. If it
achieves a 2–3× acceptance rate on clippy prompts, our effective throughput would
**exceed llama.cpp's unspeculated 31.21 tok/s without fixing either underlying
gap** (Issue 637 prefill GEMM, or the decode kernel bandwidth deficit).

That makes it plausibly the cheapest route to "faster than llama.cpp" on this
workload — and clippy fixes look like a favourable case, since fix text is highly
predictable given the lint name (the corpus already proves the fix space is
small and repetitive).

## Why clippy prompts specifically

Speculative acceptance depends on draft predictability. Clippy healing is close
to best-case:

- The fix for a given lint is near-deterministic (`needless_return` → drop the
  `return`), which is exactly what the L1 corpus exploits.
- **We already have a drafter.** `ClippyTernaryDrafter` retrieves corpus fixes in
  ~2.3 µs and is right 10/10 in-distribution (Bench 009). A corpus-retrieved fix
  is a *free, already-validated* draft — the retriever's fatal flaw (0/10 on
  held-out, Issue 010) is harmless here, because a wrong draft is simply
  rejected by verification rather than emitted.

That is the interesting composition: **the retriever cannot be trusted to
answer, but it can be trusted to guess.** Issue 010's defect stops mattering the
moment its output is verified rather than returned.

## Gates

- **G1 (correctness):** speculative output must be **token-identical** to greedy
  non-speculative decode on the same prompts. Skip-verify is only sound if it
  cannot change the emitted text; any divergence is a fail, not a tradeoff.
- **G2 (acceptance rate):** mean accepted tokens per verified forward, over the
  10 seed + 10 held-out fixtures. Report the distribution, not just the mean —
  a bimodal accept pattern behaves very differently in latency terms.
- **G3 (effective throughput):** effective tok/s vs the 17.85 raw baseline, and
  vs llama.cpp's 31.21. **Must be reported as "effective (speculative)" and
  never as a raw forward rate** — comparing a speculated number against an
  unspeculated one is the same category error that produced the mixed results in
  Bench 009/585.
- **G4 (fairness):** llama.cpp also supports speculative decoding
  (`llama-speculative`, and the fork ships `test-dspark-*` binaries). If we claim
  a speculation win, the honest comparison is **speculated vs speculated** — G3's
  llama.cpp column must eventually be its speculative mode, not its raw mode.
- **G5 (held-out):** acceptance must not collapse on the held-out fixture. If it
  only works where the corpus already holds the answer, the win is scoped to
  cache hits and must be stated that way.

## Not to be confused with

- **Issue 637** (riir-ai) — batched-prefill ternary GEMM. Orthogonal: it fixes
  the 8.38× prefill gap in the *raw* forward. Speculation does not help prefill.
- **Issue 010** (this repo) — the drafter cannot generalize. Unaffected as an
  answering path; consumed here only as a *draft source*.

## Prerequisite

- [x] **T0** — **RESOLVED 2026-08-12. The runnable path is the fork binary, not
      a riir-ai crate, and it is Metal-capable.**

  **Correction to this issue's original text:** it claimed the binaries live in
  `vendor/prismml-llama.cpp/build/bin/` *relative to this repo*. That is wrong —
  **riir-clippy has no `vendor/` directory at all.** The fork is a sibling of
  the repo set:

  ```text
  /Users/katopz/git/vendor/prismml-llama.cpp/build/bin/
  ```

  Findings:

  | question | answer |
  |---|---|
  | Binaries present? | Yes — `test-dspark-forward`, `test-dspark-loop`, `test-dspark-real-eval`, all built 2026-08-09 |
  | Native on M3? | Yes — `Mach-O 64-bit executable arm64`; `test-dspark-forward` runs and prints usage |
  | Metal-capable? | **Yes** — links `libggml-metal.0.dylib`, *identical* backend linkage to the `llama-cli` that produced Bench 010's 149.95 tok/s Metal numbers |
  | CUDA-gated? | No — `common/dspark-markov.cu` is the device path, but the header documents a host fallback via `LLAMA_DSPARK_MARKOV_CUDA=0` |
  | Rust implementation in riir-ai? | **None.** A grep over `crates/**/*.rs` finds zero hits; every DSpark reference is prose in `.proposals/018`, `.plans/485`, `.plans/528`, `.issues/606`, `.issues/608` |

  So this issue does **not** convert to "wire DSpark into the ternary forward" —
  the fork binary is the measurement vehicle. Note the consequence for G3/G4:
  the fork arm measures *llama.cpp + DSpark*, so it answers "does speculation
  help llama.cpp on clippy prompts", **not** "does it help katgpt". A katgpt-side
  speculative path would still need building before G3 can compare our
  effective throughput against our own 17.85 raw baseline.

  Usage discovered:
  ```text
  test-dspark-forward <model.gguf> --tier1 | --tier2 <ref.bin> \
      [min_argmax_match_rate] [min_top5_overlap]
  ```

- [x] **T0b** — **RESOLVED 2026-08-13. Measured in [Bench 012](../.benchmarks/012_dspark_speculative_acceptance_clippy.md). DSpark is a net slowdown (0.554×) on the current `dspark-Q4_1` checkpoint; the accept rate (0.558 overall, 0.774 code) is below the breakeven the block-diffusion draft cost demands. Speculation moves clippy throughput the wrong way — it will not reach llama.cpp's 31.21 tok/s.**

  **Two corrections to this task's original text** (factual, verified against
  `vendor/prismml-llama.cpp/tests/test-dspark-forward.cpp`):

  1. **`--tier1` takes the drafter, not the target.** `run_tier1` reads
     `dspark.dspark.block_size` from the loaded model and fails otherwise
     (`"could not read dspark.dspark.block_size from GGUF"`). Passing the Q2_0
     target exits 1 instantly. The drafter GGUF is the correct argument.
  2. **tier1 is a CPU synthetic finiteness/shape smoke test, NOT an
     argmax-agreement statistic between draft and target.** It pins
     `n_gpu_layers = 0` (*"CPU: deterministic, no Metal precision surprises"*)
     and asserts logits are finite + correctly shaped. There is no draft-vs-target
     argmax comparison in tier1 — that lives in `test-dspark-real-eval` (real
     target + drafter, per-token accept). tier1 has **no GPU surface**, so the
     GPU-exclusivity blockade that held this task for two sessions was premised
     on a misread: contention cannot corrupt a CPU smoke test.

  **What was actually run** (both binaries):

  - `test-dspark-forward --tier1 <drafter>` → **PASSED** (~1 s, CPU). Drafter
    `Bonsai-27B-dspark` loads clean: 3.65B, 6 trunk layers, `block_size=4`,
    `target_layers=[1,16,31,46,61]`, `markov_rank=256`, `confidence_head=true`,
    vocab 248320. Logits finite (`non_finite=0`), shape `4×248320`. This is a
    forward-graph validity check; it does not measure acceptance.
  - `test-dspark-real-eval <target> <drafter>` on GPU (24 built-in prompts,
    clear GPU) → **accept 0.558** (code 0.774 / reasoning 0.678 / chat 0.344),
    τ 3.23, **speedup 0.554** (AR 12.31 tok/s → spec 6.81 tok/s). A CPU
    cross-check (`n_gpu_layers=0`, contention-immune for the accept metric)
    agreed within Metal precision (overall 0.514 vs 0.558).

  **GPU exclusivity remains a hard precondition — but for `real-eval`, not
  tier1.** The accept-rate measurement *does* use the GPU (`n_gpu_layers=999`)
  and *is* an argmax-agreement statistic, so the Bench 649 discipline correctly
  applies to it. The GPU run was measured under verified exclusivity: pre-flight
  showed `0` compute procs after the sibling `bench_655_tasks_max_ab` loop
  exited; run window 15:56:48→16:05:30, `exit=0`, stable ~14–15 tok/s AR on code
  (no Bench-649 divergence signature). See Bench 012 §"GPU exclusivity".

  **Why the win does not materialize.** Accept-by-depth is healthy and even
  rises with depth (0.76→0.86), so the drafter's block-diffusion refinement is
  working. But τ≈3.2 accepted tokens per round does not pay for the cost of
  drafting a full `block_size=4` block (fixed-length markov-resample loop) plus
  the verify forward. Every prompt was slower under speculation (best 0.78×).
  This matches the vendor scope doc: *"accept rate does not yet beat MTP… around
  600 training steps in."* The drafter is under-trained.

  **Open follow-ups (do NOT reopen this issue for these):**
  - Re-measure when a later-trained dspark checkpoint lands (the scope doc gates
    runtime on training convergence).
  - **G5 clippy-specific:** the clippy held-out fixtures were *not* run — the
    target GGUF has **no chat_template**, blocking the `dataset.jsonl` path
    (which applies the chat template). The converted clippy set is staged
    (`clippy_l4_sft_test.jsonl` → 116 prompts `{"turns":[...]}`) and ready when a
    templated target is. The built-in chat/reasoning categories already preview
    the G5 collapse (chat 0.344; worst prompts 0.17–0.27).
  - **G3 katgpt-side:** a katgpt+DSpark path does not exist; this bench positions
    the *ceiling of the idea* on the llama.cpp engine, which is sub-1.0.

## Status of the alternative route (riir-ai Issue 637 T3) — 2026-08-12

This issue exists because DSpark looked like the cheapest way to beat
llama.cpp's 31.21 tok/s without fixing the underlying gaps. The *other* route —
batched prefill — has now been built and gated, and it did not work:

- **riir-ai Issue 637 T1/T2**: the batched ternary GEMM is correct (2.0e-7 vs P
  sequential GEMVs) but caps at **1.08×** on the projection roll-up against a
  ≥3× gate; the wins are launch-overhead amortization on under-occupied shapes,
  and `ffn_gate/up` + `ffn_down` (63% of projection time) are unwinnable.
- **riir-ai Issue 637 T3**: `prefill()` shipped behind `ternary_gemm_batched`
  but **fails G1** — exact at P=1, diverging from P=2 (max rel 2.7e-1 → 4.2e0),
  though argmax holds at every P. Root cause open. G2 measured 1.05×.

So the prefill route is not going to close the gap either, which raises rather
than lowers the value of measuring speculation here.

**G3/G4 caveat that constrains what this issue can conclude.** Per T0, the only
runnable DSpark is the *fork binary*, so its arm measures **llama.cpp + DSpark**,
not katgpt + DSpark. That means:

- **G3** (effective throughput) cannot be stated against our own 17.85 raw
  baseline from the fork arm alone — it would compare llama.cpp's speculated
  rate against katgpt's unspeculated one, the exact category error that had to
  be corrected in Bench 009 and flagged in Bench 585. A katgpt-side speculative
  path must exist first.
- **G4** (fairness) is *satisfiable* by this route and is what it is actually
  good for: llama.cpp-speculated vs llama.cpp-raw is a clean same-engine pair
  and answers "how much does DSpark buy on clippy prompts at all" — which is
  the prerequisite question before anyone builds a katgpt-side drafter.

Reframe accordingly: T0b measures the **ceiling** of the idea, not our position
against it.

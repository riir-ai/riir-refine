# Issue 011 — Measure DSpark speculative decode on clippy prompts

**Type:** proof / measurement
**Status:** OPEN — not measured
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

- [ ] **T0b** — Run `test-dspark-forward <Bonsai-Q2_0.gguf> --tier1` on Metal and
      record acceptance behaviour. Not yet done: it loads a second 7 GB model and
      was deferred to avoid contending with the Issue 637 T3 prefill run on the
      same box.

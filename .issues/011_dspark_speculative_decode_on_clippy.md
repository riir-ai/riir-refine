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

The fork ships `test-dspark-forward`, `test-dspark-loop`, `test-dspark-real-eval`
in `vendor/prismml-llama.cpp/build/bin/`, and riir-ai references DSpark in
Proposal 018 / Plan 485. **First task is to establish what is actually runnable
on M3 Metal against Bonsai** — Proposal 018 records DSpark as the winner of a
GOAT gate that FlashAR failed, but does not confirm a wired ternary path.

- [ ] **T0** — Locate the runnable DSpark path (fork binary vs riir-ai crate) and
      confirm it accepts the Bonsai Q2_0 GGUF on Metal. If nothing is wired,
      this issue converts to "wire DSpark into the ternary forward" and the
      measurement follows.

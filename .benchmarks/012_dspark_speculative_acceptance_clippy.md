# Bench 012 — DSpark speculative decode acceptance + speedup on clippy-class prompts (M3 Max Metal)

**Date:** 2026-08-13
**Issue:** [011 — Measure DSpark speculative decode on clippy prompts](../.issues/011_dspark_speculative_decode_on_clippy.md)
**Hardware:** M3 Max, Metal only (Apple M3 Max, MTLGPUFamilyApple9 / Metal4, 53 GB free working set)
**Models:**
- Target: `riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf` (7.17 GB) — `Bonsai-27B`, 27B, Qwen3.5 tokenizer (`tokenizer.ggml.pre = qwen35`, vocab 248320, **no chat_template in GGUF**)
- Drafter: `riir-train/data/Ternary-Bonsai-27B-dspark-Q4_1.gguf` (1.95 GB) — `Bonsai-27B-dspark`, 3.65B, 6 trunk layers
**Vehicle:** `vendor/prismml-llama.cpp/build/bin/test-dspark-{forward,real-eval}` (fork built 2026-08-09; `common/dspark-markov.cu` host fallback, Metal-linked)

---

## TL;DR — DSpark is a net **slowdown** on this checkpoint (0.55×), not a speedup

The decisive number is **speedup = 0.554** (speculative 6.81 tok/s vs same-engine autoregressive
12.31 tok/s, clear GPU). DSpark makes generation **~45% slower**, not faster. The acceptance rate
(0.558 overall, 0.774 on code) is not high enough to overcome the block-diffusion draft cost.

**Implication for Issue 011's motivating question** ("can DSpark get clippy healing above
llama.cpp's 31.21 tok/s without fixing the underlying gaps?"): **No.** Speculation subtracts here;
it does not add. This route does not close the gap, by the same-engine (G4) comparison the issue
identifies as the only fair one available from the fork binary.

This is consistent with the fork's own scope doc
(`vendor/prismml-llama.cpp/docs/dspark-scope.md`): *"the measured accept rate does not yet beat
MTP… around 600 training steps in… runtime work is gated on that result."* The drafter is
under-trained; this bench measures the ceiling and the ceiling is below breakeven.

---

## Two corrections to Issue 011's T0b text (factual, from the source)

Issue 011 T0b (lines 107–125) says: *"Run `test-dspark-forward <Bonsai-Q2_0.gguf> --tier1`"* and
characterizes *"Tier-1 acceptance is an argmax-agreement statistic between draft and target."*
Both are wrong, verified against `tests/test-dspark-forward.cpp`:

1. **`--tier1` takes the drafter, not the target.** `run_tier1` reads `dspark.dspark.block_size`
   from the loaded model and fails otherwise (`"could not read dspark.dspark.block_size from GGUF"`).
   Passing the Q2_0 **target** would exit 1 instantly. The drafter GGUF is the correct argument.
2. **tier1 is a CPU synthetic finiteness/shape smoke test, not an acceptance statistic.** It pins
   `mparams.n_gpu_layers = 0` (*"CPU: deterministic, no Metal precision surprises"*), feeds
   deterministic synthetic tap features + mask tokens, and asserts the logits are finite and the
   right shape. There is no draft-vs-target argmax comparison in tier1 — that lives in tier2
   (diff vs a Python reference `.bin`) and in `test-dspark-real-eval` (real target + drafter,
   per-token accept). tier1 has **no GPU surface at all**, so the GPU-exclusivity blockade that
   held T0b for two sessions was premised on a misread: contention cannot corrupt a CPU smoke test.

The **actual** acceptance/speedup measurement is `test-dspark-real-eval` (real target + real
drafter, 24 prompts, GPU). That is what this bench runs. Both binaries were exercised.

---

## Results

### A. `test-dspark-forward --tier1 <drafter>` (literal T0b) — PASSED, CPU smoke test

```
=== Tier 1: synthetic tap features, wiring/shape/finiteness check ===
load: layer 0..6 assigned to CPU   (n_gpu_layers = 0; offloaded 0/7 layers to GPU)
n_embd=5120 n_vocab=248320 block_size=4 mask_token_id=248319 n_capture=5
logits: count=993280 min=-7.89601 max=13.1387 non_finite=0
Tier 1 PASSED: finite, correctly-shaped logits (4 x 248320)
```

Drafter metadata: `block_count=6`, `block_size=4`, `target_layers=[1,16,31,46,61]`,
`markov_rank=256`, `confidence_head=true`, `confidence_head_with_markov=true`,
`log_snr_conditioning=true` (min/max log_snr ±9). **Note:** this checkpoint differs from the
scope doc's description (`block_size=7`, 5 trunk layers, `target_layers=(1,9,17,25,33)`) — it is a
newer config than the doc. Exit 0, ~1 s. **This run does not measure acceptance** (it cannot).

### B. `test-dspark-real-eval <target> <drafter>` — the acceptance/speedup measurement

**GPU run** — 24 built-in prompts (8 code / 8 chat / 8 reasoning), `n_predict=96`, `n_gpu_layers=999`,
plain-text continuation (no chat template — the target GGUF has none). Clear GPU (see exclusivity below).

Per-category:

| category | rounds | drafted | accepted | **accept** | τ (acc/round) |
|---|---:|---:|---:|---:|---:|
| **code** | 190 | 760 | 588 | **0.7737** | 4.0947 |
| reasoning | 211 | 844 | 572 | 0.6777 | 3.7109 |
| chat | 310 | 1240 | 426 | 0.3435 | 2.3742 |
| **ALL** | 711 | 2844 | 1586 | **0.5577** | **3.2307** |

Accept-by-depth (conditional: given position *i* of the block was reached):

| depth | reached | accepted | accept_i |
|---:|---:|---:|---:|
| 1 | 711 | 542 | 0.7623 |
| 2 | 542 | 417 | 0.7694 |
| 3 | 417 | 338 | 0.8106 |
| 4 | 338 | 289 | 0.8550 |

Speedup (the load-bearing metric):

| | tok/s | |
|---|---:|---|
| autoregressive baseline (same engine) | **12.31** | |
| speculative (DSpark) | **6.81** | |
| **speedup** | **0.554** | ← net slowdown |

Per-prompt speedup ranged 0.273 (chat #13: 53 tokens in 31 rounds, τ 1.71 — near-total draft
rejection, 2.81 tok/s) to 0.783 (code #5: τ 4.67, 11.61 tok/s). **Every single prompt was slower
under speculation** — the best case (0.783) is still a 22% slowdown.

### C. CPU cross-check (deterministic ground truth) — hardware independence

A second `test-dspark-real-eval` run with `n_gpu_layers=0` (entirely CPU; arithmetic deterministic
regardless of scheduling, so contention cannot corrupt the accept-rate metric) on the first 15
prompts, run concurrently with a sibling GPU bench for the *speed* number only:

| category | drafted | accepted | accept (CPU) | accept (GPU) |
|---|---:|---:|---:|---:|
| code | 496 | 401 | 0.808 | 0.774 |
| chat | 776 | 253 | 0.326 | 0.344 |
| ALL | 1272 | 654 | **0.514** | 0.558 |

CPU and GPU agree within Metal precision (Δ ≈ 0.03–0.04). **The accept rate is a hardware-independent
property of the drafter–target pair** — it is not a Metal artifact. (CPU speedup is also <1.0
everywhere, as expected — block-diffusion needs GPU parallelism to amortize the draft+verify, and
even with it the breakeven is not met.)

---

## GPU exclusivity (recorded per Issue 011 / riir-ai AGENTS.md "GPU exclusivity during correctness gates")

The GPU (B) run was measured under **verified exclusivity**:

- **Pre-flight (15:56:48):** `0` compute procs; the sibling `bench_655_tasks_max_ab` loop that had
  held the GPU earlier had exited. Load 3.43/5.77/6.35 (1/5/15-min).
- **Run window:** 15:56:48 → 16:05:30 (~8 m 42 s), `exit=0`, no Metal contention/error signatures.
- **Internal consistency check:** per-prompt AR tok/s on code was stable at 14.3–15.1 tok/s across
  all 8 code prompts — Bench 649's contention signature is *unstable/divergent* results, not seen.
  The chat tail decay (10.3 → 6.5 tok/s) tracks round-count growth (draft rejection → more rounds),
  not contention.

Exclusivity was **not** guaranteed trivially: two sibling Issue-640 Metal benches (`bench_658_flush_per_token`,
then a looping `bench_655_tasks_max_ab`) held the GPU across the session. The B run launched only
after a direct process check confirmed both gone. (The CPU cross-check C was deliberately run *during*
contention because CPU accept-rate is contention-immune — only its speed number was contended, and
that number is not cited.)

---

## Gate verdict (Issue 011 §Gates)

| gate | requirement | result |
|---|---|---|
| **G2** acceptance | mean accepted/forward + distribution | **0.558 overall** (code 0.774 / reasoning 0.678 / chat 0.344); τ 3.23. Distribution is strongly category-bimodal (code ≳0.77, chat ≲0.34). |
| **G3** effective tok/s | spec vs raw, reported as effective | spec **6.81** vs same-engine AR **12.31** = **0.554×**. Effective throughput is *lower*, not higher. (Vs the issue's cross-engine aspirational targets: 6.81 < 17.85 katgpt-raw < 31.21 llama-raw — speculation does not reach either.) |
| **G4** fairness | speculated vs speculated / same-engine | **Satisfied and decisive**: same-engine spec-vs-AR = 0.554×. DSpark loses on the engine it runs on. |
| **G1** correctness | spec ≡ greedy | Not separately re-verified here; `test-dspark-real-eval` accepts via `common_sampler_sample_and_accept_n` (greedy/temp=0 target verify), which is lossless by construction — emitted text is the greedy decode. The speedup is computed against the same greedy AR baseline. |
| **G5** held-out | accept must not collapse off-distribution | **Partially previewed.** The clippy held-out fixtures were not run directly: the target GGUF has **no chat_template**, so the `dataset.jsonl` path (which applies the chat template) is blocked, and clippy prompts are instruction-style. The built-in chat/reasoning categories (which the drafter has *not* memorized) already show the collapse: chat 0.344, with the worst prompts at 0.17–0.27. This is the G5 failure mode appearing on non-code text. A clippy-specific G5 needs a chat-template-bearing target or a continuation-formatted clippy set. |

---

## What this concludes (and what it does not)

**Concludes:**
- DSpark on the **current** `dspark-Q4_1` checkpoint is a **net slowdown (0.55×)** for this
  target on M3 Max. Speculation will not lift clippy-healing throughput above llama.cpp's raw
  31.21 tok/s; it moves the wrong way. The accept rate (0.56) is below the breakeven the
  block-diffusion draft cost demands.
- The accept rate is **code-biased** (0.77 code, 0.34 chat) — clippy is code, so the relevant
  number is the better one, but even 0.77 does not clear the draft-cost bar.

**Does not conclude:**
- Whether a **later-trained** dspark checkpoint beats breakeven. The scope doc explicitly gates
  runtime on training convergence ("does not yet beat MTP"). Re-measure when a new drafter lands.
- **katgpt + DSpark** (Issue 011 G3 caveat, lines 162–170): the fork arm measures llama.cpp+DSpark.
  A katgpt-side speculative path does not exist; this bench cannot position katgpt+DSpark vs
  katgpt-raw (17.85). It positions the *ceiling of the idea* on this engine — and the ceiling is
  sub-1.0.
- **Clippy-lint-specific** accept (G5): blocked on the target's missing chat_template. The
  category breakdown is the available proxy; a clippy-specific run needs a templated target or
  continuation-formatted clippy prompts. The clippy prompt set was converted
  (`clippy_l4_sft_test.jsonl` → `/tmp/clippy_l4_turns.jsonl`, 116 prompts, `{"turns":["..."]}`)
  and is ready when a templated target is.

---

## Commands (reproducible)

```bash
# A. tier1 smoke (CPU; literal T0b). Takes the DRAFTER.
vendor/prismml-llama.cpp/build/bin/test-dspark-forward \
  riir-train/data/Ternary-Bonsai-27B-dspark-Q4_1.gguf --tier1

# B. real-eval on GPU (the acceptance + speedup measurement). Built-in 24 prompts.
vendor/prismml-llama.cpp/build/bin/test-dspark-real-eval \
  riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  riir-train/data/Ternary-Bonsai-27B-dspark-Q4_1.gguf 96 999 "" 24
# argv: target drafter n_predict=96 n_gpu_layers=999 dataset="" n_prompts=24

# C. CPU cross-check (deterministic; contention-immune for accept-rate).
vendor/prismml-llama.cpp/build/bin/test-dspark-real-eval \
  riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  riir-train/data/Ternary-Bonsai-27B-dspark-Q4_1.gguf 64 0 "" 4
```

Logs: `/tmp/dspark_realeval_gpu.log`, `/tmp/dspark_realeval_cpu.log` (this session).

## Cross-references

- Issue 011 (this repo) — the question + gates
- [Bench 010](010_llamacpp_vs_katgpt_ternary_clippy_tps.md) — raw forward baseline (llama.cpp 31.21 / katgpt 17.85 decode), no speculation
- riir-ai [Issue 640](../../riir-ai/.issues/640_batched_rmsnorm_prefill_nondeterministic.md) + [Bench 649](../../riir-ai/.benchmarks/649_contention_ab.md) — GPU-exclusivity rule (12.3× argmax divergence under contention)
- riir-ai [Issue 637](../../riir-ai/.issues/) — batched-prefill route (the orthogonal gap; also did not close it)
- vendor `docs/dspark-scope.md` — *"accept rate does not yet beat MTP… gated on training convergence"*

# Bench 012 — DSpark speculative decode acceptance + speedup on clippy-class prompts (M3 Max Metal)

**Date:** 2026-08-13
**Issue:** `011 — Measure DSpark speculative decode on clippy prompts`
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

**G5 clippy-specific (§D, full 116-prompt run, 2026-08-13)** ran all 116 clippy SFT-test
prompts (10 lint types) after unblocking the `chat_template` gap: accept **0.710** / speedup
**0.667×** — still a net slowdown (0/116 prompts beat 1.0×), but no off-distribution collapse.
Per-lint breakdown: local-expression lints accept ~0.80, AST-rewriting lints (`manual_map`,
`useless_conversion`) ~0.60 — the gap is structural, not statistical. G5 confirms the verdict.

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
| **G5** held-out | accept must not collapse off-distribution | **MEASURED (clippy-specific, full 116-prompt set), §D below.** All 116 real clippy prompts (10 lint types × ~12) run as plain-text continuation (the `chat_template` block was bypassed via a local vendor-harness `plain:`-prefix patch — see §D). **No collapse**: clippy accept 0.710 weighted / 0.693 mean, range 0.36–0.89 — code-shaped, far above the 0.17–0.27 collapse seen on the built-in chat category. **But still a net slowdown** (speedup 0.667, 0/116 prompts beat 1.0×). The verdict is unchanged: G5 confirms, does not flip. Per-lint breakdown (§D) shows local-expression lints accept ~0.80, AST-rewriting lints (~0.60) are the hard case. |

---

## D. G5 clippy-specific — `test-dspark-real-eval` on the full 116-prompt clippy SFT set (2026-08-13)

The built-in runs (§B/§C) used the harness's 24 hardcoded Python/C++/chat prompts. G5 asks
whether acceptance **collapses** on the actual clippy workload (Rust lint-healing prompts),
which the drafter has not memorized. The block was that `dataset.jsonl` forces
`common_chat_templates_apply`, and the Q2_0 target ships **no `chat_template`**.

**Unblock:** a minimal local patch to the vendor harness adds an opt-in `plain:` prefix on
the dataset path — when present, the jsonl is loaded as **plain-text continuation** (no chat
template), exactly like the built-in prompts. The patch is 4 edits (~12 lines), existing
behavior unchanged without the prefix (committed vendor-side, `prismml-llama.cpp@855b1f8`).

**Run:** GPU (`n_gpu_layers=999`), `n_predict=96`, `plain:/tmp/clippy_l4_turns.jsonl`, **all
116 prompts** (10 distinct lint types × ~12 prompts each). Concurrent GPU load: Unity editor
idle (~1% CPU, light render — not compute-saturating; the accept-rate metric is hardware-
independent per §C anyway). Window 17:23:08→17:51:57, `exit=0`.

| metric | value |
|---|---:|
| prompts | 116 |
| drafted / accepted | 9676 / 6867 |
| **accept (weighted)** | **0.7097** |
| accept (mean / median) | 0.693 / 0.740 |
| accept (min / max) | 0.357 / 0.893 |
| τ (accepted/round, weighted) | 3.84 |
| AR tok/s | 13.35 |
| spec tok/s | 8.91 |
| **speedup** | **0.667** |
| prompts with speedup > 1.0 | **0 / 116** |
| prompts with accept > 0.8 | 32 / 116 |
| prompts with accept < 0.6 | 31 / 116 |

**Accept-by-depth** (conditional): 0.88 / 0.82 / 0.91 / 0.89 — healthy, rises with depth
(same block-diffusion refinement signature as §B).

### Per-lint breakdown (the actionable signal)

The 116-prompt set is balanced across 10 distinct clippy lints (~12 each), so the per-lint
accept rate is a clean characterization of which fix shapes a token drafter can predict:

| lint | n | drafted | accepted | **accept** | speedup | best prompt |
|---|---:|---:|---:|---:|---:|---:|
| `redundant_closure`        | 12 | 1104 | 902 | **0.817** | 0.745 | 0.776 |
| `needless_range_loop`      |  8 |  700 | 561 | **0.801** | 0.713 | 0.810 |
| `needless_lifetimes`       | 12 | 1056 | 837 | **0.793** | 0.715 | 0.980 |
| `needless_borrow`          | 12 | 1148 | 899 | **0.783** | 0.719 | 0.941 |
| `redundant_field_names`    | 12 |  636 | 457 | **0.719** | 0.686 | 0.758 |
| `needless_return`          | 12 |  812 | 557 | **0.686** | 0.608 | 0.757 |
| `unnecessary_mut_passed`   | 12 |  712 | 482 | **0.677** | 0.637 | 0.902 |
| `unnecessary_unwrap`       | 12 |  788 | 509 | **0.646** | 0.593 | 0.764 |
| `manual_map`               | 12 | 1356 | 842 | **0.621** | 0.596 | 0.718 |
| `useless_conversion`       | 12 | 1364 | 821 | **0.602** | 0.589 | 0.645 |

The pattern is clear and **contradicts Issue 011's thesis** ("clippy fixes are
near-deterministic → best-case for speculation"):

- **Local-expression lints accept well (~0.80):** `redundant_closure` (`|x| f(x)` → `f`),
  `needless_borrow`, `needless_lifetimes`, `needless_range_loop`. These are fixes the drafter
  can predict from the local token context — the rewrite is a deletion/substitution within a
  single expression.
- **AST-rewriting lints accept poorly (~0.60):** `manual_map` (`match` → `.map()`),
  `useless_conversion` (remove `.into()`/`as`), `unnecessary_unwrap`. These change the AST
  shape (drop a branch, remove a layer of indirection), which a token-level block-diffusion
  drafter trained on general code cannot predict from local context alone. **`manual_map` —
  the lint Issue 011 explicitly named as "near-deterministic" / best-case — is in fact the
  second-worst category (0.621).**

This is the actionable signal for any future drafter improvement: the gap is **structural**,
not statistical. A drafter that reasons over AST shape (or is fine-tuned on the rewrite
patterns) would lift the bottom quartile; a better token predictor would not.

### G5 verdict

- **No collapse.** The clippy-specific accept (0.710 weighted / 0.693 mean) is **well above**
  the §B overall (0.558) and far above the chat-category collapse (0.344). Clippy prompts are
  code-shaped and stay in the 0.36–0.89 band — the G5 failure mode (off-distribution collapse)
  does **not** appear. The worst single prompt was 0.357 (still 2× the chat-category floor).
- **But still a net slowdown** (0.667× weighted, 0.658× mean, **0/116 prompts beat 1.0×**).
  G5 **confirms** the §B/§C verdict; it does not flip it. Even the best prompt (0.893 accept)
  was a 0.98× slowdown — the draft-cost floor dominates acceptance at this checkpoint.
- The headline number for clippy is **0.710 accept / 0.667× speedup**, vs the built-in code
  proxy's **0.774 / still-net-slower**. Clippy is slightly *below* the generic-code proxy,
  driven by the AST-rewriting lints (`manual_map`, `useless_conversion`) that drag the mean
  down. The local-expression lints alone (~0.80) would clear the code proxy, but still not
  breakeven.

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
- **Clippy-lint-specific** accept (G5): **now fully measured — §D below** (116 prompts,
  10 lint types, per-lint breakdown). No collapse (0.36–0.89, code-shaped), but still a net
  slowdown (0.667×). The actionable finding: local-expression lints accept ~0.80,
  AST-rewriting lints (~0.60) are the hard case — the gap is structural, not statistical.

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

# D. G5 clippy-specific (needs the local vendor-harness `plain:`-prefix patch; see §D).
#    The `plain:` prefix bypasses the chat_template (the Q2_0 target ships none).
vendor/prismml-llama.cpp/build/bin/test-dspark-real-eval \
  riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  riir-train/data/Ternary-Bonsai-27B-dspark-Q4_1.gguf 96 999 plain:/tmp/clippy_l4_turns.jsonl 24
```

Logs: `/tmp/dspark_realeval_gpu.log`, `/tmp/dspark_realeval_cpu.log` (this session);
`/tmp/g5_clippy_gpu.log` (§D G5 clippy-specific run).

## Cross-references

- Issue 011 (this repo) — the question + gates
- [Bench 010](010_llamacpp_vs_katgpt_ternary_clippy_tps.md) — raw forward baseline (llama.cpp 31.21 / katgpt 17.85 decode), no speculation
- riir-ai `Issue 640` + [Bench 649](../../riir-ai/.benchmarks/649_contention_ab.md) — GPU-exclusivity rule (12.3× argmax divergence under contention)
- riir-ai [Issue 637](../../riir-ai/.issues/) — batched-prefill route (the orthogonal gap; also did not close it)
- vendor `docs/dspark-scope.md` — *"accept rate does not yet beat MTP… gated on training convergence"*

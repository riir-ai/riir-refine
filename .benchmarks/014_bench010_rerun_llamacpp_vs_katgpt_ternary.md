# Bench 014 — Bench 010 re-run: llama.cpp vs katgpt ternary, tok/s on clippy prompts (M3 Max Metal)

**Date:** 2026-08-13
**Parent:** [Bench 010](./010_llamacpp_vs_katgpt_ternary_clippy_tps.md) (2026-08-12)
**Hardware:** M3 Max, Metal only
**Model:** `Ternary-Bonsai-27B-Q2_0.gguf` — **the same file for both arms**
**Task:** the clippy-healing prompts from `benches/fixtures/e2e_cases_seed.json` (first 3 cases)
**Arm A:** `prismml-llama.cpp` binary built at `9ca265a` (binary `--version` reports `1 (9ca265a)` — **same commit as Bench 010**) at `/Users/katopz/git/vendor/prismml-llama.cpp/build/bin/llama-server`, `--temp 0.0`, `--jinja`
**Arm B:** katgpt ternary — `TernaryDeltanetGpuForward` (riir-gpu, CubeCL `wgpu<msl>`), greedy argmax
**Harness:** `benches/bench_clippy_ternary_tps.rs` (Arm B) + `scripts/bench010_llama_arm.py` (Arm A, new)
**Why:** the user asked to re-run Bench 010 against current code ("refer to prev git, re-bench needed"). This is the same protocol, same session, both arms timed with the other's process stopped.

---

## The one table (re-run)

Same 3 clippy prompts, same 64-token decode budget, same session, both arms measured with the other arm's process stopped. Load average 3.2–5.1 for both.

| metric | llama.cpp (re-run) | katgpt ternary (re-run) | llama.cpp faster by | Bench 010 gap (for ref) |
|---|---:|---:|---:|---:|
| **prefill tok/s** | **145.11** | **18.12** | **8.01×** | 8.38× |
| **decode tok/s** | **30.89** | **18.95** | **1.63×** | 1.75× |
| **total tok/s** | **62.75** | **18.41** | **3.41×** | 3.57× |
| prompt tokens (3 cases) | 349 | 349 | identical | 349 |
| decode tokens | 192 | 192 | identical | 192 |
| prefill wall | 2.41 s | 19.26 s | | |
| decode wall | 6.22 s | 10.13 s | | |
| model load (excluded) | ~3.6 s | 136.8 s | | |
| GPU init (excluded) | — | 4.4 s | | |

Per-case decode tok/s:

| lint | prompt tok | llama.cpp | katgpt |
|---|---:|---:|---:|
| `needless_return` | 93 | 31.34 | 19.06 |
| `redundant_closure` | 154 | 31.22 | 18.81 |
| `needless_lifetimes` | 102 | 30.14 | 18.99 |

**Tokenizer parity VERIFIED, not assumed:** the llama arm independently
tokenized the pre-rendered chat string to 93 / 154 / 102 = 349 tokens via the
resident model's `/tokenize` endpoint — bit-identical to the katgpt arm's count.
The prompt text is byte-identical between the Rust harness and the Python
driver (the same `<|im_start|>` chat template + the shared system message),
rendered by hand in both arms rather than relying on server-side templating.

---

## Verdict vs Bench 010

The re-run **confirms Bench 010 within measurement noise.** Every rate moved
<7% and the structural story is unchanged:

| | Bench 010 (2026-08-12) | Bench 014 re-run (2026-08-13) | Δ |
|---|---:|---:|---:|
| llama.cpp prefill | 149.95 | 145.11 | −3.2% |
| llama.cpp decode | 31.21 | 30.89 | −1.0% |
| llama.cpp total | 63.80 | 62.75 | −1.6% |
| katgpt prefill | 17.89 | 18.12 | +1.3% |
| katgpt decode | 17.85 | **18.95** | **+6.2%** |
| katgpt total | 17.88 | 18.41 | +3.0% |
| prefill gap | 8.38× | 8.01× | slightly narrower |
| decode gap | 1.75× | **1.63×** | **slightly narrower** |
| total gap | 3.57× | 3.41× | slightly narrower |

The most notable movement is **katgpt decode 17.85 → 18.95 tok/s (+6.2%)**,
which narrows the decode gap from 1.75× to 1.63×. One genuine improvement vs
Bench 010: this run's decode (18.95) is now **faster than prefill (18.12)**,
whereas Bench 010 had them essentially tied (17.85 ≈ 17.89). That is the
expected ordering — decode should beat prefill because decode reuses the KV
cache. The Bench 010 near-tie was a small anomaly; the re-run shows the
structurally-expected shape.

Bench 010's two load-bearing conclusions both **hold on the re-run**:

1. **The gap is prefill, not decode.** Decode is 1.63× behind (close); prefill
   is 8.01× behind (far). Same as 010's 1.75× / 8.38×.
2. **Our prefill rate ≈ our decode rate** because we push prompt tokens
   one-at-a-time through the decode path (no batched GEMM). llama.cpp batches
   prefill and gets 4.7× its own decode rate (145.11 / 30.89). The missing
   piece is a ternary GEMM on Metal — exactly as 010 diagnosed.

Bench 010's "path to parity" ordering (batched prefill GEMM > kernel bandwidth
> speculative decode > PFlash) is **unchanged** by this re-run and is not
re-argued here.

---

## What changed in the harness (the `system_prompt` bug)

Bench 010's reproduction note says the llama arm was driven by "POST the
identical chat-template string to `/completion` with `n_predict=64`" — but no
driver script was committed, so the exact request shape was lost. The first
attempt at this re-run used `/completion`'s `system_prompt` field + server-side
`--jinja` templating. That produced **prompt token counts of 39 / 100 / 48**
(187 total) — **162 tokens short** of the katgpt arm's 349. The `system_prompt`
field is only honored on slot load, not per-request, so the system message +
template overhead was silently dropped.

The fix: `scripts/bench010_llama_arm.py` **pre-renders the exact chat string**
the Rust harness builds (`<|im_start|>system\n...<|im_end|>\n<|im_start|>user\n...`)
and sends it as a raw `prompt` with `chat_template: false`, then **verifies
parity** by counting the rendered string via the resident `/tokenize` endpoint
before each timed request. The verified counts (93 / 154 / 102) match the
katgpt arm bit-for-bit.

**Lesson:** when a bench claims "tokenizer parity verified", the verification
must be reproducible. Bench 010's claim was correct (the numbers agree), but
the mechanism that produced it was undocumented. Committing the driver script
alongside the bench (this doc + `scripts/bench010_llama_arm.py`) closes that
gap — the next re-run is one command, not an archaeology exercise.

---

## Caveats

- **Same binary, different working tree.** The prismml binary reports
  `--version` as `1 (9ca265a)` — the exact commit Bench 010 cited. The
  *working tree* at `/Users/katopz/git/vendor/prismml-llama.cpp` has since
  moved to `855b1f8` (`feat(dspark-real-eval): add 'plain:' dataset prefix`),
  but the **built artifact** is still `9ca265a`. Arm A therefore runs the same
  code path Bench 010 ran. (A stale source tree relative to a built binary is
  the normal case for a bench artifact.)
- **The homebrew `llama-server` (build 1, `952a47f`) cannot load this model**
  — it fails with `tensor 'output.weight' has invalid ggml type 42 (NONE)`.
  Type 42 is the ternary/deltanet quantization that only the prismml fork
  understands. Standard llama.cpp does not support it. This is why Bench 010
  (and this re-run) MUST use the prismml fork for Arm A — it is not a drop-in
  replacement.
- Load averages were 3.2–5.1 across the run (vs Bench 010's 3.6–4.5). Within
  the same band; not material — Bench 010 itself showed Arm B decode is
  insensitive to load in this range.
- Same caveats as Bench 010 §"What this does not measure": throughput only (no
  quality — quality is Bench 009's business), 3 of 10 cases, no end-to-end at
  realistic output length.

## Reproduce

```sh
# Arm B — katgpt ternary on clippy prompts (this repo)
BONSAI_GGUF=/Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
CLIPPY_CASES=benches/fixtures/e2e_cases_seed.json DECODE_TOKENS=64 N_CASES=3 \
OUT_JSONL=/tmp/clippy_ternary_tps.jsonl \
  cargo bench --bench bench_clippy_ternary_tps --features ternary_inference

# Arm A — llama.cpp (prismml fork) on the SAME prompts (stop Arm B first)
/Users/katopz/git/vendor/prismml-llama.cpp/build/bin/llama-server \
  --model /Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  --port 8125 --ctx-size 4096 --temp 0.0 --seed 42 -ngl 99 --jinja --no-webui &
# wait for /health == ok, then:
python3 scripts/bench010_llama_arm.py --port 8125 \
  --cases benches/fixtures/e2e_cases_seed.json --n-predict 64 --n-cases 3 \
  --out /tmp/bench010_llama.json
```

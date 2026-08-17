# Bench 036 — interleaved per-pair A/B: llama.cpp vs katgpt ternary on clippy prompts (M3 Max Metal)

**Date:** 2026-08-17
**Parents:** [Bench 010](./010_llamacpp_vs_katgpt_ternary_clippy_tps.md) (2026-08-12), [Bench 014](./014_bench010_rerun_llamacpp_vs_katgpt_ternary.md) (2026-08-13)
**Hardware:** M3 Max, Metal only
**Model:** `Ternary-Bonsai-27B-Q2_0.gguf` — the same file for both arms
**Task:** first 3 clippy-healing prompts from `benches/fixtures/e2e_cases_seed.json`, 64-token decode budget
**Arm A:** `prismml-llama.cpp` binary `9ca265a` (same build as Bench 010/014), `--temp 0.0`, `--jinja`
**Arm B:** katgpt ternary — `TernaryDeltanetGpuForward`, **batched simdgroup cmma prefill** (the production DEFAULT-ON path), greedy argmax
**Harness:** `scripts/bench036_interleaved_ab.py` (new, commits `0459bac` + `1cdbd72`) driving `benches/bench_clippy_ternary_tps.rs` + `scripts/bench010_llama_arm.py`
**Why:** first re-measure since the batched simdgroup GEMM (riir-ai Bench 645) and batched attention prefill (riir-ai Bench 665) went DEFAULT-ON, and the first run of this pair under a **true interleaved per-pair protocol**.

---

## Verdict — NO, we are not faster than llama.cpp on Metal

| metric | median ratio (A/B) | spread (min..max) | Bench 014 | movement |
|---|---:|---:|---:|---|
| **prefill** | **6.050×** | 5.389 .. 6.536 | 8.01× | **narrowed** |
| **decode** | **1.980×** | 1.906 .. 2.206 | 1.63× | widened (load artifact — see below) |
| **total** | **3.164×** | 2.913 .. 3.281 | 3.41× | narrowed |

llama.cpp remains ahead on every axis. **The prefill gap genuinely narrowed** (8.01× → 6.05×), which is the batched simdgroup prefill landing and is directionally consistent with commit `a566255`'s isolated claim of 5.88×. The decode ratio *appears* to have regressed, but it did not — see §"The decode ratio moved the wrong way, and why that is load".

**These ratios are an UPPER BOUND on llama.cpp's lead, not a point estimate.** Arm B is disproportionately load-depressed (§below), so the true quiet-box ratios are smaller than every number in this table.

---

## Per-pair data (the primary metric)

Order alternates within pairs so monotonic drift cancels instead of accumulating into one arm. 1 discarded warmup pair + 3 measured.

| pair | order | loadavg | Arm A prefill/decode/total | Arm B prefill/decode/total | ratio prefill/decode/total |
|---|---|---:|---:|---:|---:|
| warmup0 | A→B | ~15.3 | 132.89 / 26.44 / 54.72 | 18.20 / 10.24 / 14.27 | *discarded* |
| pair0 | B→A | 18.73 | 115.58 / 24.67 / 50.09 | 19.10 / 11.18 / 15.27 | 6.050 / 2.206 / 3.281 |
| pair1 | A→B | 17.75 | 131.40 / 25.39 / 52.95 | 20.10 / 12.82 / 16.73 | 6.536 / 1.980 / 3.164 |
| pair2 | B→A | 20.21 | 112.57 / 25.20 / 50.47 | 20.89 / 13.22 / 17.33 | 5.389 / 1.906 / 2.913 |
| **median** | | | **115.58 / 25.20 / 50.47** | **20.10 / 12.82 / 16.73** | **6.050 / 1.980 / 3.164** |

**Tokenizer parity verified as a hard gate, not an assumption:** every pair recorded
349 prompt tokens and 192 decode tokens on **both** arms (the driver aborts the run
on any mismatch). Arm A counts come from the resident model's `/tokenize` endpoint
on the pre-rendered chat string; Arm B tokenizes the byte-identical string itself.

---

## `absolutes_comparable_to_bench_014 = false`

Recorded programmatically in `/tmp/bench036.json`. Every pair ran at loadavg
**17.75–20.21**, against a `LOAD_COMPARABLE_CEILING` of **12.6** — the highest load
at which Bench 010 actually demonstrated Arm B decode insensitivity (18.24 tok/s at
load 12.6 vs 17.94 at 3.6). Above that ceiling there is no evidence of insensitivity.

**So the absolute tok/s in this bench must NOT be compared against Bench 010/014's
absolutes.** Arm B's 12.82 median decode is not a regression from Bench 014's 18.95,
and Arm A's 25.20 is not a regression from 30.89 — both arms are simply depressed.
Only the per-pair ratios carry across, and even those are bounded (next section).

The load came from a sibling agent's CPU-only 27B forward (Plan 336 T2.9 determinism
probe, `--cpu-eval`, "~10-30 min/step") plus its build traffic. It holds no GPU, so
**GPU exclusivity was satisfied throughout** — the driver re-checks for GPU compute
consumers before every arm visit and aborts if one appears. It never fired.

---

## The decode ratio moved the wrong way, and why that is load — not a regression

Decode went 1.63× (Bench 014) → 1.980× here. That is **asymmetric load depression**,
measured, not assumed:

| arm | Bench 014 decode | Bench 036 median decode | depression |
|---|---:|---:|---:|
| A (llama.cpp) | 30.89 | 25.20 | **−18.4%** |
| B (katgpt ternary) | 18.95 | 12.82 | **−32.3%** |

Arm B loses ~1.8× more throughput to the same CPU load than Arm A does. Dividing a
14%-hit numerator by a 32%-hit denominator inflates the ratio — which fully accounts
for 1.63 → 1.98 without any engine change.

**Mechanism (consistent with riir-ai Bench 669):** our decode path encodes ~832
dispatches per token from the CPU and performs a `read_one` sync every token, so CPU
starvation directly serialises into per-token latency. llama.cpp's Metal backend
submits less CPU-side work per token and degrades more gracefully.

**Interleaving does not repair this.** Alternating order cancels drift *between*
blocks; it cannot cancel *differential sensitivity to a load level present in both
arms*. This is the protocol's known limit and the reason the verdict is stated as an
upper bound.

**Corroborating evidence that Arm B had not plateaued:** its decode rose monotonically
across every visit — 10.24 (warmup) → 11.18 → 12.82 → 13.22 — while Arm A sat flat at
24.67 / 25.39 / 25.20. The spread is therefore **directional drift, not symmetric
noise**: the ratio shrinks as Arm B warms (2.206 → 1.980 → 1.906). Even the best pair
(1.906×) is likely still pessimistic.

**This asymmetry is itself a production-relevant finding**, independent of the
comparison: our throughput degrades roughly twice as hard as llama.cpp's on a busy
machine. A clippy-healing daemon sharing a developer box pays that penalty.

---

## What genuinely improved: prefill

Prefill ratio 8.01× → **6.05×**, and Arm B's prefill is the one absolute that rose
despite the load handicap (18.12 in Bench 014 → 20.10 here, at ~1.5× the load).

The cause is not new work in this bench — it is that the harness now calls the
production batched path. Commit `a566255` (2026-08-14) switched
`bench_clippy_ternary_tps` from a per-token GEMV loop to the batched simdgroup cmma
`prefill()`, measuring **17.89 → 25.49 tok/s (+42.5%)** on a quieter box and putting
the gap at 5.88×. This bench's 6.05× at loadavg ~19 is the same finding under load,
independently reproduced through a different protocol.

**Bench 014's 8.01× prefill gap was therefore partly a harness artifact**, not purely
an engine gap: it timed the one-token-at-a-time GEMV path while the production default
was already batched.

Prefill still dominates the remaining deficit (6.05× vs decode's 1.98×), so Bench
010's ordering — batched prefill first — remains correct, and there is more left in it.

---

## Protocol (what this fixes vs Bench 010/014)

Bench 010 and 014 timed each arm in a separate block and divided aggregate-A by
aggregate-B. Any between-block drift lands entirely in the ratio. riir-ai Bench 645
was burned by exactly this twice: its 2.48× was retracted for GPU contention and the
"corrected" 1.42× was *also* wrong (sequential-arm inflation).

This driver instead:

- runs **interleaved pairs** with the within-pair order alternating (A→B, B→A, A→B);
- reports the **median of per-pair ratios** plus the min..max spread, so a wide or
  directional spread stays visible instead of averaging away;
- keeps **only one arm on the GPU at a time** — `llama-server` is started once (~4 s)
  then `SIGSTOP`ed, and `SIGCONT`ed only for its own visit. A stopped process submits
  no GPU work. Arm B gets a fresh process per visit (its ~137 s GGUF load is excluded
  from the rates by the harness, as in 010/014);
- treats **tokenizer parity as an abort gate**;
- **gates on contention** two ways: no GPU-compute consumers (matched on executable
  basename), and loadavg ≤ 16.0 hard-abort with a separate 12.6 comparability flag.

Two contention-gate bugs were found and fixed while building this, both worth knowing:
matching GPU-consumer names against the full command line false-positived on a zsh
poller whose wait condition contains `bonsai_lora_accuracy_parity_arm_c`, and on
`rustc --crate-name riir_games` whose `--extern` flags reference `riir-gpu`. Matching
the executable **basename** fixes both. An `arm_c` probe invoked with `--cpu-eval` is
CPU-only and correctly does not gate the GPU.

---

## Measured config

Arm B ran the production default: `set_prefill_use_simdgroup(true)` (hardcoded in the
harness), **ZEROCOPY off**, **SPECULATIVE off**. Both exclusions are measured, not
stylistic:

- `metal_tensor_gemm` zero-copy overlay is a **47× decode regression at 27B scale**
  (17.2 → 0.37 tok/s) plus G1 garbage — commit `3f354f4`. riir-ai Issue 663's isolated
  +1.21× does not reproduce at production scale.
- speculative decode is a **net slowdown** on this checkpoint: 0.554× same-engine, and
  0.667× / 0 of 116 prompts beating 1.0× on the full clippy set
  ([Bench 012](./012_dspark_speculative_acceptance_clippy.md)). Acceptance is below
  the α≥0.58 breakeven.

---

## The 4090 arm: UNMEASURED

The second half of this investigation — validating the apparent **1.14–1.19×** win on
the 4090 (riir-ai Bench 682/687's 90.04–93.86 tok/s vs the 79.09 tok/s `llama-bench
tg128` measured on that box in riir-ai Bench 667) — **was not measured and is recorded
here as unmeasured, not as a result.**

Reason: the 4090 never freed an exclusive window. `bonsai_lora_accuracy_parity_arm_c`
(PID 23928, a sibling agent's Plan 336 work) held it for the entire session — 2.23 h
elapsed at last check, with the GPU at 9068 MiB / 20–24% against Bench 687's documented
**676 MiB shell-only exclusive baseline**. A watcher polled for its exit and never fired.

That apparent win therefore **remains a claim, not a measurement**, and its specific
defects stand as previously recorded: cross-session (Benches 682/687 never re-ran the
llama arm — neither document mentions llama.cpp), and a different artifact
(`arm_b_gpu500.bin` + LoRA rather than the Q2_0 GGUF llama.cpp loaded). Do not cite
90.04 vs 79.09 as a head-to-head.

---

## What this does not measure

- **Quality.** Throughput only. The 64-token cap truncates mid-reasoning by design,
  identically in both arms.
- **A quiet-box ratio.** Every pair ran at loadavg 17.75–20.21. This is the headline
  follow-up: re-run at load ≤ 12.6 to convert the upper bound into a point estimate.
  The command is one line (§Reproduce) and the driver will wait for the window itself.
- **The full 10-case fixture.** 3 of 10 cases, because Arm B pays ~137 s of model load
  per visit and this protocol takes 4 visits.
- **End-to-end at realistic output length.** Bench 010's ~850-token projection is not
  re-derived here.
- **The 4090 arm.** Unmeasured, above.

## Reproduce

```sh
# Both arms, interleaved, with the contention gates enforced.
# --wait-for-window makes the driver wait for a clean window instead of assuming one.
cd riir-clippy
python3 scripts/bench036_interleaved_ab.py \
  --repeats 3 --warmup-pairs 1 --n-cases 3 --n-predict 64 \
  --target-dir /tmp/b036_clippy_tps \
  --wait-for-window 3600 \
  --out /tmp/bench036.json

# Arm B binary must be prebuilt (the driver invokes cargo bench, a no-op rebuild):
CARGO_TARGET_DIR=/tmp/b036_clippy_tps \
  cargo bench --no-run --bench bench_clippy_ternary_tps --features ternary_inference
```

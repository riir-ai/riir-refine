# Bench 036 — interleaved per-pair A/B: llama.cpp vs katgpt ternary on clippy prompts (M3 Max Metal)

> **Headline (3 clean pairs, 2026-08-17):** prefill **5.343×** (5.192–5.556), decode
> **1.512×** (1.446–1.513), total **2.523×** (2.405–2.575). Prefill genuinely narrowed
> from Bench 014's 8.01×; **decode is unchanged** once Arm A's 11% thermal drift is
> accounted for (honest bracket 1.45–1.70× vs 1.63×). Still behind llama.cpp.

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

## ✅ THREE-CLEAN-PAIR RUN (2026-08-17, per-pair re-gated driver) — the point estimate

Re-run after adding per-pair load re-gating. **All three pairs cleared the 12.6
ceiling** (loads 8.35 / 5.59 / 11.10) and the driver recorded
`absolutes_comparable_to_bench_014 = true`. Token parity 349/349 prompt, 192/192
decode in every pair.

| pair | order | loadavg | Arm A prefill/decode | Arm B prefill/decode | ratio prefill/decode/total |
|---|---|---:|---:|---:|---:|
| pair0 | B→A | 8.35 ✅ | 149.51 / 29.35 | 26.91 / 19.41 | 5.556 / 1.512 / 2.575 |
| pair1 | A→B | 5.59 ✅ | 139.29 / 27.43 | 26.07 / 18.13 | 5.343 / 1.513 / 2.523 |
| pair2 | B→A | 11.10 ✅ | 132.19 / 25.01 | 25.46 / 17.30 | 5.192 / 1.446 / 2.405 |
| **median** | | | **139.29 / 27.43** | **26.07 / 18.13** | **5.343 / 1.512 / 2.523** |
| spread | | | | | 5.192–5.556 / 1.446–1.513 / 2.405–2.575 |

**Headline vs Bench 014:**

| metric | Bench 014 | **median (3 clean pairs)** | spread |
|---|---:|---:|---:|
| prefill | 8.01× | **5.343×** | 5.192–5.556 |
| decode | 1.63× | **1.512×** | 1.446–1.513 |
| total | 3.41× | **2.523×** | 2.405–2.575 |

The spread is now **±3.4% on prefill and ±2.2% on decode**, against the previous
attempt's ±13% / ±32%. That is the per-pair gate working: it paused before pair0
at loadavg 20.6 rather than measuring, so every pair here was taken inside the
window.

### One caveat that matters, and it cuts against us

Arm A degraded monotonically across the run — decode **29.35 → 27.43 → 25.01**
(−15% end to end) — and that is *not* explained by load, which went 8.35 → 5.59 →
11.10 (non-monotonic). Sustained GPU work heating the M3 is the likely cause. Arm
A's median decode (27.43) therefore sits **11.2% below** Bench 014's 30.89, while
Arm B's (18.13) is only **4.3% below** its 18.95.

So the decode ratio is **flattered** this run — the opposite direction from the
loaded run's inflation. Normalising Arm A to its Bench 014 baseline gives
`30.89 / 18.13 = 1.703×`. Honest bracket:

> **decode gap is 1.45–1.70×, i.e. materially unchanged from Bench 014's 1.63×.**
> Do not read 1.512× as a decode improvement.

Prefill is robust to the same correction because Arm B's gain dominates: Arm B
prefill **18.12 → 26.07 (+43.9%)**, and normalising Arm A gives
`145.11 / 26.07 = 5.566×`. So:

> **prefill gap is 5.34–5.57×, genuinely narrowed from 8.01× (~1.44–1.50×).**
> This is the batched simdgroup prefill, independently reproducing commit
> `a566255`'s 25.49 tok/s through a third protocol.

### Verdict

**Still not faster than llama.cpp on Metal**, on three clean interleaved pairs:
behind by **~5.3–5.6× on prefill** and **~1.45–1.70× on decode**. The prefill gap
has genuinely closed by about a third since Bench 014; the decode gap has not
moved. Prefill remains the dominant deficit, so Bench 010's ordering still holds.

---

## Quiet-box re-run (2026-08-17, `--max-load 12.6`) — one clean pair, not three

The follow-up this bench asked for was run. It **partially succeeded**, and the
partial failure is the more useful finding.

| pair | order | **loadavg** | Arm A prefill/decode/total | Arm B prefill/decode/total | ratio prefill/decode/total |
|---|---|---:|---:|---:|---:|
| warmup0 | A→B | ~11.5 | 150.90 / 30.57 / 62.96 | 26.14 / 18.53 / 22.82 | *discarded* |
| **pair0** | B→A | **12.06** ✅ | **150.04 / 30.70 / 63.05** | **26.17 / 18.38 / 22.75** | **5.733 / 1.670 / 2.771** |
| pair1 | A→B | 24.61 ❌ | 142.03 / 27.29 / 56.98 | 23.21 / 15.17 / 19.54 | 6.120 / 1.799 / 2.917 |
| pair2 | B→A | 35.29 ❌ | 116.29 / 19.90 / 42.77 | 16.14 / 7.01 / 11.04 | 7.203 / 2.840 / 3.875 |
| median (all 3) | | | | | 6.120 / 1.799 / 2.917 |
| spread | | | | | 5.733..7.203 / 1.670..2.840 / 2.771..3.875 |

**Do not quote the median.** The window opened at loadavg 11.47, then a sibling's
`cargo build` (bevy — `rustc --crate-name bevy_render` at 587% CPU) drove load to
24.61 and 35.29 during pairs 1 and 2. Only **pair0 (12.06)** met the ≤12.6
ceiling. Because the median of three pairs *is the middle pair*, the headline
median is pair1's number — **a load-24.61 measurement wearing a quiet-box label**.
The driver recorded `absolutes_comparable_to_bench_014 = false` (it requires ALL
pairs under the ceiling), which is the correct verdict for the run as a whole.

### The defensible quiet-box numbers are pair0's (n=1)

pair0's absolutes land on top of Bench 014's, which is what earns it:

| | Bench 014 (load 3.2–5.1) | pair0 (load 12.06) | Δ |
|---|---:|---:|---:|
| Arm A prefill / decode | 145.11 / 30.89 | 150.04 / 30.70 | +3.4% / **−0.6%** |
| Arm B prefill / decode | 18.12 / 18.95 | **26.17** / 18.38 | **+44.4%** / −3.0% |

| metric | Bench 014 | **pair0 quiet** | movement |
|---|---:|---:|---|
| prefill | 8.01× | **5.733×** | **narrowed 1.40×** |
| decode | 1.63× | **1.670×** | **unchanged** (+2.5%, within run-to-run) |
| total | 3.41× | **2.771×** | narrowed |

Two conclusions, both now on comparable absolutes:

1. **The prefill gain is real and reproduced.** Arm B prefill 18.12 → 26.17
   (+44.4%) independently reproduces commit `a566255`'s 25.49 through a different
   protocol, and cuts the prefill gap 8.01× → 5.733×.
2. **Decode has not moved.** 1.63× → 1.670× with Arm B decode 18.95 → 18.38. This
   **retires the loaded run's apparent decode regression** (1.980×, and 2.840× in
   the load-35 pair) as pure load artifact — exactly as the asymmetric-sensitivity
   section predicted. Our decode gap to llama.cpp is where it was.

### Method fix this exposed

The driver gated loadavg **only at startup**, so a mid-run spike could not abort or
pause it. Fixed: it now **re-gates before every pair** and waits for the window to
reopen (`--pair-load-timeout`, default 1800 s), stopping with the clean pairs it
already has rather than appending contaminated ones. Startup-only gating is what
turned three pairs into one usable pair here.

**Still open:** a run with three pairs all under the ceiling, to give the ratios an
honest spread instead of a single pair. pair0 and Bench 014 agree closely, so the
expected value is not in doubt — the *variance* is unmeasured.

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

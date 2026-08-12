# Bench 010 — llama.cpp vs katgpt ternary, tok/s on clippy prompts (M3 Max Metal)

**Date:** 2026-08-12
**Hardware:** M3 Max, Metal only
**Model:** `Ternary-Bonsai-27B-Q2_0.gguf` — **the same file for both arms**
**Task:** the clippy-healing prompts from `benches/fixtures/e2e_cases_seed.json`
**Arm A:** `prismml-llama.cpp` @ `9ca265a` — `llama-server`, `--temp 0.0`
**Arm B:** katgpt ternary — `TernaryDeltanetGpuForward` (riir-gpu, CubeCL `wgpu<msl>`), greedy argmax
**Harness:** `benches/bench_clippy_ternary_tps.rs` (Arm B) — this repo

---

## The one table

Same 3 clippy prompts, same 64-token decode budget, **same session**, both arms
measured with the other arm's process stopped so neither holds the GPU while the
other is timed. Load average 3.6–4.5 for both.

| metric | llama.cpp | katgpt ternary | llama.cpp faster by |
|---|---:|---:|---:|
| **prefill tok/s** | **149.95** | **17.89** | **8.38×** |
| **decode tok/s** | **31.21** | **17.85** | **1.75×** |
| **total tok/s** | **63.80** | **17.88** | **3.57×** |
| prompt tokens (3 cases) | 349 | 349 | identical |
| decode tokens | 192 | 192 | identical |
| prefill wall | 2.33 s | 19.51 s | |
| decode wall | 6.15 s | 10.76 s | |
| model load (excluded) | ~15 s | 127.0 s | |
| GPU init (excluded) | — | 1.3 s | |

Per-case decode tok/s:

| lint | prompt tok | llama.cpp | katgpt |
|---|---:|---:|---:|
| `needless_return` | 93 | 31.26 | 17.94 |
| `redundant_closure` | 154 | 31.20 | 17.98 |
| `needless_lifetimes` | 102 | 31.17 | 17.64 |

**Tokenizer parity is verified, not assumed:** both arms independently tokenized
the same prompt strings to the same counts (93 / 154 / 102 → 349 total). The
prompt text is byte-identical between the Rust harness and the Python driver,
including the `<|im_start|>` chat template and the shared system message.

---

## How this differs from Bench 009 (they measure different Arm B's)

| | Bench 009 | **Bench 010 (this)** |
|---|---|---|
| Arm A | llama.cpp + Bonsai | llama.cpp + Bonsai |
| **Arm B** | **the retriever** — `HealPipeline`, a corpus lookup that emits a stored string | **our inference stack** — `TernaryDeltanetGpuForward` generating tokens |
| Arm B can generalize? | **No** — 0/10 held-out, emits the same 30.9 tokens whatever you ask (Issue 010) | Yes — it is the same 27 B model llama.cpp runs |
| metric | per-fix e2e tok/s + fixes/sec, incl. thinking tokens | prefill / decode / total tok/s |
| ratio | ~74,000× (cache vs compute) | **1.75× decode, 8.38× prefill** |
| condition | contended (`load 77`), corrected post-hoc | same session, load 3.6–4.5 both arms |
| quality measured | yes (10/10 seed, 0/10 held-out) | no — throughput only |

**They answer different questions.** 009 asks "should a cache serve known lints?"
(yes, by 5 orders of magnitude, but it cannot generalize). 010 asks "how far is
our inference from llama.cpp's on the same work?" (1.75× on decode). Only 010 is
a like-for-like engine comparison; only 009 speaks to quality. Neither number
belongs in the other's table — mixing them is what made the earlier reporting
confusing.

---

## Where the gap actually is: prefill, not decode

**Decode is only 1.75× behind.** That is far closer than any earlier number in
this repo implied, and it means the ternary decode path is broadly competitive.

**Prefill is 8.38× behind, and the reason is visible in our own numbers:**

```text
katgpt prefill 17.89 tok/s  ≈  katgpt decode 17.85 tok/s     <-- identical
llama.cpp prefill 149.95 tok/s = 4.8× its own decode 31.21   <-- batched
```

Our prefill rate *equals our decode rate* because we push prompt tokens through
the **same one-token-at-a-time path** as generation. llama.cpp batches the
prompt into a single `mul_mm` and gets 4.8× its own decode rate. We have no
batched prefill at all.

### We have PFlash — it does not fix this

`riir-gpu/src/forward_flashprefill.rs` implements **PFlash**: block-sparse
prefill *attention* (mean-K → block score → sinks/window/top-score select →
sparse online-softmax forward). It is real and it works, but it accelerates the
**attention** term at long context. Our clippy prompts are **93–154 tokens**,
where attention is a rounding error and the cost is the **projections**. PFlash
is the right tool for 100 K-token contexts, not for this.

### What is actually missing: a ternary GEMM on Metal

Every Metal ternary kernel in `gemv_ternary_cubecl.rs` — `launch`, `launch_lut`,
`launch_rowtiled`, `launch_rowtiled8`, `launch_packed`, `launch_plane` — takes a
**single `input_handle` and a single `output_handle`. There is no batch
dimension.** They are all GEM**V**. The only ternary GEM**M** in the tree,
`TernaryGemmCudaRaw`, lives in `gemv_ternary_cuda_raw.rs` and is CUDA/dp4a — **not
available on Metal.**

That is precisely llama.cpp's prefill advantage, and its own kernel names show
it: `kernel_mul_mv_q2_0_f32` for decode vs **`kernel_mul_mm_q2_0_f32`** for
prefill. The payoff is measured, not assumed — same Q2_0 kernel, same shape,
varying batch:

| batch `n` | GFLOPS | vs n=1 |
|---:|---:|---:|
| 1 | 617.7 | 1.0× |
| 8 | 744.5 | 1.2× |
| **512** | **5,830** | **9.4×** |

**So the top fix for clippy-context latency is a batched ternary GEMM.** In this
64-token test it is 19.51 s of our 30.26 s total — **64% of our wall clock spent
on the prompt.** For a realistic clippy fix (~850 output tokens, Bench 009
measured 847 avg) prefill's share falls to ~12%, so the payoff depends on output
length:

| output length | our prefill share | fixing prefill buys |
|---|---:|---:|
| 64 tok (this test) | 64% | up to 2.6× |
| 850 tok (real fix) | ~12% | ~1.13× |

Projected per-fix time at the measured rates, for a 116-token prompt + 847
output tokens:

| | prefill | decode | **total per fix** |
|---|---:|---:|---:|
| llama.cpp | 0.77 s | 27.1 s | **27.9 s** |
| katgpt ternary | 6.5 s | 47.5 s | **54.0 s** |

**≈1.9× on the real clippy task.** (Derived from measured rates, not measured
end-to-end at 847 tokens — flagged as such.)

---

## Reconciling the three numbers I previously reported

The earlier figures were not measuring the same thing, and two of them should
never have been compared. For the record:

| figure | what it actually was | valid for this comparison? |
|---|---|---|
| **16.571 tok/s** (Bench 009) | llama.cpp full fix generation *including thinking tokens*, measured at **load 77** with a sibling GPU job resident | No — contended, and it is Arm A only |
| **25.50 tok/s** (Bench 585) | `llama-bench tg128` — synthetic decode, empty context, quiet box | No — not a clippy prompt |
| **9.16 tok/s** (Bench 585) | **our GEMV projection-only sum** (Bench 606 amortized µs × per-token call counts) — never a real forward | **No — and it is misleading** |
| **17.85 tok/s** (this bench) | our real GPU-resident forward decoding a real clippy prompt | **Yes** |
| **31.21 tok/s** (this bench) | llama.cpp decoding the same prompt, same session | **Yes** |

**The 9.16 figure was wrong in an instructive way.** It summed *individual*
amortized GEMV timings, so it modelled a forward that pays per-dispatch cost on
every projection. The real GPU-resident forward measures **17.85 tok/s — nearly
2× faster than the sum of its parts** — because activations stay on the GPU and
the per-dispatch round-trips never happen. A GEMV-sum is therefore **not a valid
lower bound** for a fused forward, and Bench 585's Stage-3 comparison
(9.16 "projections only" vs llama.cpp's full token) overstated our deficit.

Bench 585's *kernel-level* finding still stands — our large-shape ternary GEMV is
at parity-or-better with llama.cpp's Q2_0 `mul_mv` — and this bench is
consistent with it: if the kernels are comparable and decode is only 1.75× off,
the remaining loss is structural (prefill batching), exactly as the kernel data
predicted.

---

## Decode: we are kernel-bound, not dispatch-bound

Effective aggregate bandwidth over the whole forward (dense model, `n_expert=0`,
so every weight is read every token — 6.81 GB/token at 2.125 bits/weight):

| | tok/s | ms/token | effective GB/s | % of 400 GB/s |
|---|---:|---:|---:|---:|
| llama.cpp | 31.21 | 32.0 | **212** | 53.1% |
| katgpt ternary | 17.85 | 56.0 | **121** | 30.4% |

Our best **measured single-kernel** bandwidth is 132.6 GB/s (`lm_head`) and
113.7 GB/s (FFN) — so the real fused forward at **121 GB/s is already ~91% of our
own best kernel number.** There is almost no dispatch overhead left to reclaim in
the GPU-resident decode path.

**This reverses Bench 585's P1 priority for decode.** 585 ranked "kill the
launch-bound dispatches" first, but that ranking came from the invalid GEMV-sum
model (which assumed per-dispatch cost everywhere). In the resident path, decode
is **bandwidth-bound at the kernel level**: closing the remaining 1.75× requires
taking the kernel from ~121 to ~213 GB/s (30% → 53% of roofline), not removing
more syncs.

## Path to parity, in order of measured payoff

1. **Ternary GEMM on Metal (batched prefill).** Missing entirely; llama.cpp's own
   n=1→n=512 sweep shows 9.4× available. Fixes the 8.38× prefill gap. Largest and
   least ambiguous win.
2. **Kernel bandwidth 121 → 213 GB/s** to close the 1.75× decode gap. This is the
   hard one: we are at our own kernel ceiling, so it means a faster kernel, not
   better scheduling. Bench 585 shows our large-shape GEMV already matches
   llama.cpp's isolated Q2_0 `mul_mv`, so the gap is likely in llama.cpp's
   shape-specialised variants (`mul_mv_ext_q2_0_f32_r1_{2,3,4,5}`, `nsg` tuning)
   which we have no equivalent of.
3. **DSpark speculative decode — orthogonal, and possibly the fastest route to
   "faster than llama.cpp".** DSpark (entropy-based skip-verify, modelless) and
   FlashAR raise *effective* tok/s by accepting several drafted tokens per
   verified forward. **Bench 010 measured the raw forward with no speculation
   enabled**, and llama.cpp's 31.21 is likewise unspeculated. A working 2–3×
   acceptance rate would put us ahead on effective throughput without fixing (1)
   or (2) — but it changes *what* is being compared, so it must be reported as
   effective-with-speculation, never as a raw forward rate.
4. **PFlash** — keep for long-context work; irrelevant at 93–154 prompt tokens.

## What this does not measure

- **Quality.** This bench is throughput only. Bonsai's fix quality on these
  prompts is Bench 009's business (10/10 seed, 10/10 held-out). The 64-token cap
  here truncates mid-reasoning by design — enough to establish a *rate*, not a
  fix. Both arms were truncated identically, so the rate comparison is fair.
- **The full 10-case fixture.** 3 of 10 cases, because Arm B pays a 127 s model
  load per process. The three cases span 93–154 prompt tokens and their decode
  rates agree within 2% (17.64–17.98), so the aggregate is stable.
- **End-to-end at realistic output length.** The 27.9 s vs 54.0 s per-fix row is
  *derived* from measured rates, not measured directly.
- **Retriever comparison.** Deliberately excluded. The riir-clippy retriever
  (16.5 µs/fix, Bench 009) is not doing the same work as either arm here — it
  looks up a stored string and cannot generalize (0/10 held-out, Issue 010).
  Mixing it into this table is what made the earlier reports confusing.

## Caveats

- Toolchain is pinned to **1.97.0** via `rust-toolchain.toml`, unified across all
  8 repos in the stack (cubecl 0.11 floors at 1.95; riir-chain already pinned
  1.97). The unpinned-repo default of 1.93 is what broke this build initially.
- Arm A's prefill benefits from `cache_prompt: false` being honoured per request;
  no prompt cache reuse inflates its prefill number.
- Load averages were 3.6–4.5 for both arms but not zero. Arm B's decode proved
  insensitive to load (18.24 tok/s at load 12.6 vs 17.94 at load 3.6, same case),
  so residual contention is not material for Arm B.

## Reproduce

```sh
# Arm B — katgpt ternary on clippy prompts (this repo)
BONSAI_GGUF=/Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
CLIPPY_CASES=benches/fixtures/e2e_cases_seed.json DECODE_TOKENS=64 N_CASES=3 \
  cargo bench --bench bench_clippy_ternary_tps --features ternary_inference

# Arm A — llama.cpp on the SAME prompts (stop Arm B first)
llama-server --model Ternary-Bonsai-27B-Q2_0.gguf --port 8125 \
  --ctx-size 4096 --temp 0.0 --seed 42 -ngl 99 --no-webui
# then POST the identical chat-template string to /completion with n_predict=64
# and read `timings.prompt_ms` / `timings.predicted_per_second`.
```

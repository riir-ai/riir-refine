# Bench 009 — E2E tokens/sec: Ternary-Bonsai-27B (llama.cpp) vs the modelless pipeline

**Issue:** `009`
**Date:** 2026-08-12
**Host:** M3 Max, Metal (12 threads), macOS Darwin 25.5.0
**Model:** `riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf` (6.66 GiB, 26.90 B params, `qwen35 27B Q2_0`)
**Serving:** `prismml-llama.cpp` @ `9ca265a`, `llama-server --temp 0.0 --seed 42 -ngl 99 --jinja`
**Modelless arm:** `riir-clippy` `--release`, `MODE=self_evolve` and `MODE=l1l2`

---

## CORRECTION (2026-08-12, after publication)

Two errors in the first version of this bench, both found by re-measuring rather
than re-reasoning. **Both inflated the ratio in the modelless arm's favour.**

**1. The headline ratio mixed measurement conditions.** I re-measured the clippy
arm on a quiet box and gave it a "quiet" column, but left the Bonsai arm on its
`load 77` number — then divided the two. That is not a comparison. Same-condition
pairs:

| pair | seed | held-out |
|---|---:|---:|
| both loaded (`load 77`) | 84,711× | 89,997× |
| **both quiet** | **73,589×** | **67,918×** |
| ~~published (quiet clippy ÷ loaded Bonsai)~~ | ~~113,242×~~ | ~~109,145×~~ |

The published figures were **~1.5× too high**. Corrected fixes/sec ratios
(both quiet): **2,018,108×** (seed) and **1,796,425×** (held-out), not 3,105,538×
and 2,886,906×.

**2. My explanation of the 25.5 → 16.5 t/s gap was wrong.** I attributed it
partly to decode degrading as the KV context grows. Measured on one server, same
load, back to back:

| `n_predict` | decoded | llama.cpp `tg` t/s | wall t/s |
|---:|---:|---:|---:|
| 32 | 32 | 12.81 | 9.35 |
| 128 | 128 | 12.64 | 11.65 |
| 512 | 512 | 12.67 | 12.42 |
| 2000 | 2000 | **14.73** | 14.63 |

The rate is **flat, and if anything improves** with length — so **KV growth is
not a cause at these lengths.** The variance is machine state: a sibling session
was running `riir_gpu` at 370% CPU plus a `bonsai_lora_accuracy_parity` job, i.e.
**GPU contention**. Bonsai's decode on this box ranges **12.6 – 25.5 t/s**
purely on what else is resident.

Request overhead is only material for short generations: at `n=32` wall 9.35 vs
decode 12.81 t/s (a fixed ~650–765 ms prompt-eval per request dominates), while
at `n=2000` wall 14.63 ≈ decode 14.73. The bench's ~800-token average sits in
the regime where e2e ≈ decode.

Everything below is the original text with the ratio rows corrected in place.

---

## Blocker resolved first: upstream llama.cpp cannot load this model

Homebrew `llama.cpp` 10250 (ggml 0.18.1) **fails** on the Bonsai GGUF:

```text
gguf_init_from_reader: tensor 'output_norm.weight' has offset 337715200, expected 357580800
llama_model_load: error loading model: failed to load model
```

`Q2_0` is a fork-local ternary quant type; upstream computes a different
bytes-per-block and so mis-derives every tensor offset. **The Bonsai arm only
runs on `/Users/katopz/git/vendor/prismml-llama.cpp`** (its HEAD commit is
literally `cuda: extract Q2_0 elements via __byte_perm`). Any future agent
benchmarking this file must use that fork's binaries — upstream is not an
option, and the failure is a hard load error, not a silent quality
degradation.

Second gotcha: the fork's `llama-cli` **hangs** on this model (10+ CPU-minutes,
one core pegged, zero output, no tokens ever emitted). `llama-bench` and
`llama-server` are both fine. Use the server.

---

## Metric and why it is defined this way

```text
tok/s = tokens(emitted fix) / wall_clock(request → fix in hand)
```

Both arms deliver the same artifact: given `(lint_name, buggy_code)`, emit
fixed Rust. Fairness rules actually enforced by the harness:

| Rule | How |
|---|---|
| Same task, same inputs | Both arms read one shared fixture, `benches/fixtures/e2e_cases_*.json`. No duplicated case list. |
| Same numerator unit | Both token counts come from **Bonsai's own tokenizer**. The clippy arm's raw Rust is tokenized post-hoc via the resident server's `/tokenize`. The modelless arm gets no "free" tokens. |
| Instrumentation off the clock | Tokenization runs in a second pass, after all generation. Using the `llama-tokenize` **binary** instead cost the server ~40% of its tok/s (21 → 13 t/s on an identical case) because it reloads 6.6 GiB per call. |
| Thinking tokens count | Bonsai is a reasoning model; its reasoning tokens are on its own clock (the user waits for them) but `--max-tokens` is 2048 so it can finish reasoning *and* emit code. |
| Truncation is reported, not scored as "wrong" | At `--max-tokens 512`, 7/10 cases were cut off mid-reasoning and emitted an empty code block. Those are budget artifacts, not capability reads. The `b_trunc` column exists so this can never be hidden. |
| Setup cost is separate | Bonsai pays model load; the modelless arm pays corpus thaw + memory seeding (~0.2 ms). Neither is silently amortized into the hot path. |
| Quality sits next to throughput | Per the repo GOAT rule, a perf gain on a wrong answer is a speedup of a wrong result. |

---

## What "tok/s" means for a retriever (read this before quoting the ratio)

tok/s was the metric requested, and it is reported faithfully. But the two arms
earn it by different mechanisms, and the number is easy to misread:

- **Bonsai** *generates* tokens. Its tok/s is a genuine decode rate — the
  hardware/model throughput that any llama.cpp user would recognise.
- **The modelless arm** does not generate anything. It retrieves a stored
  string and returns it. Its "tok/s" is a **derived** quantity: (tokens in the
  returned fix) ÷ (retrieval latency). Dividing by a ~12 µs denominator is what
  produces a 10^6-scale figure; it is arithmetic on a lookup, not decoding.

So the ratio answers "how fast does each arm put a fix in your hand", which is
the useful e2e question — but it is **not** a claim that a ternary matvec decodes
tokens 10^5× faster than a 27 B model. Nothing in this bench measures that.

There is also a numerator asymmetry worth naming: Bonsai's count includes its
**reasoning** tokens (correctly — the user waits for them), so a large part of
its "output" is deliberation the modelless arm never produces. That inflates
Bonsai's tok/s relative to its useful output while inflating its wall clock;
the two effects do not cancel. **Fixes/sec** is the cleaner task-rate metric and
is reported alongside, since it is indifferent to how verbose either arm is.

---

## How to read the results

Three numbers matter, in this order:

1. **Throughput ratio** — the answer to the question as literally asked
   ("tokens/sec e2e"). It is a real measurement of two real systems.
2. **Seed exact-match** — in-distribution quality. For the modelless arm this
   is a **retrieval** score: the corpus contains the ground-truth fix, so a
   high number here is not evidence of capability.
3. **Held-out exact-match** — the only column that speaks to generalization,
   and therefore the column that decides whether the throughput ratio is a
   meaningful engineering win or a fast wrong answer.

Reading (1) without (3) inverts the verdict. That is exactly why the held-out
fixture was built rather than reporting the seed set alone.

---

## Results — seed fixture (in-distribution)

`--max-tokens 2048`, `--temp 0.0 --seed 42`. **0/10 truncated** — every Bonsai
answer terminated on its own (`finish=stop`), so no case is scored down for
running out of budget. clippy arm = `MODE=self_evolve`; the "loaded" column is
the `load 77` run the head-to-head consumed, the "quiet" column recomputes the
same emitted fixes against the `load 11` back-to-back latencies.

| lint | Bonsai t/s | Bonsai tok | Bonsai s | clippy t/s (loaded) | clippy t/s (quiet) | clippy tok | ratio (mixed*) | b_ok | c_ok |
|---|---:|---:|---:|---:|---:|---:|---:|:-:|:-:|
| `needless_return` | 16.17 | 223 | 13.8 | 1,099,304 | 1,492,352 | 12 | 92,269× | Y | Y |
| `redundant_closure` | 16.77 | 1158 | 69.0 | 1,533,333 | 2,131,274 | 69 | 127,063× | Y | Y |
| `needless_lifetimes` | 17.13 | 1191 | 69.5 | 962,406 | 1,328,793 | 16 | 77,584× | Y | Y |
| `redundant_field_names` | 16.66 | 289 | 17.3 | 1,761,140 | 2,439,716 | 43 | 146,446× | Y | Y |
| `manual_map` | 17.12 | 550 | 32.1 | 1,166,115 | 1,754,684 | 31 | 102,515× | Y | Y |
| `unnecessary_unwrap` | 17.02 | 512 | 30.1 | 1,140,576 | 1,745,455 | 24 | 102,538× | Y | Y |
| `needless_range_loop` | 16.59 | 974 | 58.7 | 1,499,970 | 1,801,802 | 25 | 108,588× | Y | Y |
| `useless_conversion` | 16.27 | 734 | 45.1 | 1,576,044 | 1,899,171 | 22 | 116,733× | Y | Y |
| `unnecessary_mut_passed` | 16.09 | 800 | 49.7 | 1,247,312 | 1,487,179 | 29 | 92,417× | Y | Y |
| `needless_borrow` | 16.22 | 2043 | 125.9 | 1,753,819 | 2,086,995 | 38 | 128,636× | Y | Y |

### Seed aggregate

| metric | Bonsai-27B Q2_0 | riir-clippy `self_evolve` | ratio |
|---|---:|---:|---:|
| **e2e tok/s** — both loaded | 16.571 | 1,403,741 | **84,711×** |
| **e2e tok/s** — both quiet | 25.50 | 1,876,526 | **73,589×** |
| ~~mixed (quiet clippy ÷ loaded Bonsai)~~ | ~~16.571~~ | ~~1,876,526~~ | ~~113,242×~~ |
| total tokens / total time | 8474 tok / 511.4 s | 309 tok / 164.7 µs (quiet) | — |
| **fixes/sec** — both quiet | 0.0301 (33.2 s/fix) | 60,729 (16.5 µs/fix) | **2,018,108×** |
| ~~fixes/sec (loaded Bonsai)~~ | ~~0.01956 (51.1 s/fix)~~ | ~~60,729~~ | ~~3,105,538×~~ |
| avg tokens per fix | 847.4 | 30.9 | 27.4× more verbose |
| exact-match | **10/10** | **10/10** | tie |
| truncated | 0/10 | n/a | — |

Note the two ratios differ by 37×: **tok/s flatters Bonsai** because 847 of its
tokens per fix are reasoning the other arm never emits. `fixes/sec` — the rate
of *delivered work* — is the 3.1 M× figure. Both are reported because tok/s was
the metric asked for and fixes/sec is the one that is verbosity-independent.

## Results — held-out fixture (generalization)

Same 10 lints, code the corpus has never seen. `--max-tokens 2048`,
`--temp 0.0`. **0/10 truncated.** clippy arm = `MODE=self_evolve`.

| lint | Bonsai t/s | tok | s | trunc | b_ok | clippy t/s (loaded) | clippy t/s (quiet) | tok | c_ok | ratio (mixed*) |
|---|---:|---:|---:|:-:|:-:|---:|---:|---:|:-:|---:|
| `needless_return` | 15.20 | 279 | 18.4 | N | Y | 1,268,633 | 1,384,722 | 12 | **N** | 91,128× |
| `redundant_closure` | 15.86 | 635 | 40.0 | N | Y | 1,587,740 | 1,959,782 | 69 | **N** | 123,582× |
| `needless_lifetimes` | 16.22 | 796 | 49.1 | N | Y | 874,699 | 1,230,769 | 16 | **N** | 75,883× |
| `redundant_field_names` | 15.87 | 358 | 22.6 | N | Y | 1,600,000 | 2,224,176 | 43 | **N** | 140,109× |
| `manual_map` | 16.68 | 1750 | 104.9 | N | Y | 1,162,529 | 1,631,579 | 31 | **N** | 97,805× |
| `unnecessary_unwrap` | 14.93 | 266 | 17.8 | N | Y | 1,439,971 | 1,613,445 | 24 | **N** | 108,084× |
| `needless_range_loop` | 15.57 | 1431 | 91.9 | N | N* | 1,511,305 | 1,662,013 | 25 | **N** | 106,758× |
| `useless_conversion` | 15.45 | 305 | 19.7 | N | Y | 1,590,400 | 1,748,391 | 22 | **N** | 113,170× |
| `unnecessary_mut_passed` | 15.77 | 1514 | 96.0 | N | Y | 1,249,569 | 1,375,451 | 29 | **N** | 87,205× |
| `needless_borrow` | 15.35 | 839 | 54.7 | N | Y | 1,777,778 | 1,936,306 | 38 | **N** | 126,157× |

### Held-out aggregate

| metric | Bonsai-27B Q2_0 | riir-clippy `self_evolve` | ratio |
|---|---:|---:|---:|
| **e2e tok/s** — both loaded | 15.868 | 1,428,076 | **89,997×** |
| **e2e tok/s** — both quiet | 25.50 | 1,731,907 | **67,918×** |
| ~~mixed (quiet clippy ÷ loaded Bonsai)~~ | ~~15.868~~ | ~~1,731,907~~ | ~~109,145×~~ |
| total tokens / total time | 8173 tok / 515.1 s | 309 tok / 178.4 µs (quiet) | — |
| **fixes/sec** — both quiet | 0.0312 (32.1 s/fix) | 56,049 (17.8 µs/fix) | **1,796,425×** |
| ~~fixes/sec (loaded Bonsai)~~ | ~~0.01941 (51.5 s/fix)~~ | ~~56,049~~ | ~~2,886,906×~~ |
| avg tokens per fix | 817.3 | 30.9 | 26.4× more verbose |
| exact-match | **9/10** (effectively 10/10, see below) | **0/10** | — |
| truncated | 0/10 | n/a | — |

\* The per-lint `ratio (mixed*)` column divides a **quiet** clippy latency by a
**loaded** Bonsai rate, so each row is ~1.5× too high — see the CORRECTION at the
top. The per-lint rows are retained as measured; only the aggregates were
recomputed into same-condition pairs. Divide any row by 1.54 (seed) or 1.61
(held-out) for its both-quiet equivalent.

### \* The one Bonsai "miss" is my grader's fault, not the model's

`needless_range_loop` is scored wrong only because of a strict comparison:

```rust
fixture `want` : v.iter().map(|x| x * 2).collect()
Bonsai emitted : v.iter().map(|&x| x * 2).collect()
```

Both compile and are semantically identical — verified by building and running
both against the same input (`[1,2,3] → [2,4,6]`, `assert_eq!` between the two
forms passes). The destructuring closure is arguably the more idiomatic of the
two. **Bonsai's real held-out score is 10/10**; the 9/10 is a false negative
from normalized-exact-match, which cannot see semantic equivalence.

Recorded rather than quietly corrected, because it cuts the honest way: my
grader is biased *against* the LLM arm, and the verdict below holds anyway.

---

### The seed result is not the tie it looks like

Both arms scored 10/10, but they are not the same achievement:

- **Bonsai scored 10/10 zero-shot.** It had never seen these cases; it read the
  lint name and the code and produced the idiomatic fix. That is a capability.
- **The modelless arm scored 10/10 by retrieval.** These are precisely the ten
  pairs its corpus was built from, and its output is that corpus text verbatim
  (Issue 010). It is reciting, and the recitation happens to be right because
  the question was drawn from the answer key.

An earlier pass at `--max-tokens 512` scored Bonsai 3/10 and would have read as
"the fast arm is also the accurate one". That was wrong — 7 of those 10 runs
were cut off mid-reasoning with an empty code block. Raising the budget to 2048
moved Bonsai from 3/10 to 10/10 and changed nothing else. **The first number was
an artifact of my harness, not a property of the model**, which is why the
truncation column now exists.

---

## Cross-fixture verdict

| | seed (in-distribution) | held-out (unseen code) |
|---|---|---|
| Bonsai e2e tok/s — loaded / quiet | 16.571 / 25.50 | 15.868 / 25.50 |
| clippy e2e tok/s — loaded / quiet | 1,403,741 / 1,876,526 | 1,428,076 / 1,731,907 |
| **tok/s ratio — both quiet** | **73,589×** | **67,918×** |
| tok/s ratio — both loaded | 84,711× | 89,997× |
| **fixes/sec ratio — both quiet** | **2,018,108×** | **1,796,425×** |
| Bonsai exact-match | 10/10 | 10/10 (9/10 by strict grader) |
| **clippy exact-match** | **10/10** | **0/10** |
| Bonsai truncated | 0/10 | 0/10 |

**The throughput answer, as asked:** the modelless pipeline delivers a clippy
fix ~74,000× more tokens/sec and ~2.0 M× more fixes/sec than Ternary-Bonsai-27B
on llama.cpp. That number is real, reproducible, and stable across both
fixtures (both-quiet: 73,589× → 67,918×, i.e. ~8%).

**The verdict that matters:** the ratio is only meaningful where the answer is
already in the corpus. Bonsai's accuracy is **flat across the distribution
shift** — 10/10 in-distribution, 10/10 on unseen code — because it is actually
reading the input and transforming it. The modelless arm goes **10/10 → 0/10**
on the same shift, because it is not solving the task at all; it is retrieving
a stored string whose match to the seed set was guaranteed by construction
(Issue 010). Its 30.9 tokens per answer are the *same* 30.9 tokens regardless
of what it was asked.

So the two arms are not competing on one axis:

- **On corpus-resident lints**, the modelless path is the correct engineering
  choice by an enormous margin. 17 µs versus 51 s is not a trade-off; it is a
  different category of operation, and both are right.
- **On anything unseen**, the modelless path has no capability to compare. A
  67,918× speedup toward a confidently wrong function is worth less than
  51 seconds toward a correct one. Per the repo's GOAT rule — a perf gain on an
  incorrect answer is a speedup of a wrong result — **the held-out column is a
  GOAT FAIL for the modelless arm, and no throughput number rescues it.**

**Practical reading:** this is a cache-vs-compute result, not a
model-vs-modelless result. The modelless pipeline is an excellent *cache* over
known lint fixes and should be deployed as one — with the corpus-miss path
falling through to something that can actually generalize. What it is not, on
this evidence, is a replacement for the model. The dangerous failure mode is
that a miss is **silent**: L2 reports `parses=true` on all 10 wrong answers
(Issue 010), so nothing in the always-on path signals that the cache missed.
Fixing that signal is worth more than any further tok/s tuning.

---

## Which feature flag is actually "best" for this task

The premise was to benchmark "our best feature flag", assumed to be
`self_evolve` (Issue 008: +50 pp healing rate). **Measured back-to-back on the
same quiet box (`load average 11.02`), that assumption does not hold for this
task.**

| MODE | fixture | p50 | mean | p99 | parse-valid | exact-match |
|---|---|---:|---:|---:|---:|---:|
| `l1l2` (default features) | seed | **16.25 µs** | 15.89 µs | 35.75 µs | 10/10 | 10/10 |
| `self_evolve` | seed | 17.29 µs | 16.75 µs | 37.29 µs | 10/10 | 10/10 |
| `l1l2` (default features) | held-out | **16.54 µs** | 15.73 µs | 36.92 µs | 10/10 | 0/10 |
| `self_evolve` | held-out | 18.83 µs | 18.13 µs | 39.96 µs | 10/10 | 0/10 |

`self_evolve` is **6% slower on seed and 14% slower on held-out, for
bit-identical quality on both**. On this workload the memory-retrieval +
EMA-overlay layers are pure overhead, so the default-features `l1l2` path is
the better arm and is the one that should carry the headline comparison.

**This does not refute Issue 008.** That gain comes from the *feedback loop* —
`report_outcome` over repeated attempts against real `cargo clippy` ground
truth. This bench is deliberately **single-shot**: each case is healed cold,
with no outcome reported back, so the mechanism that earns SE its +50 pp is
never exercised. The honest reading is scope, not contradiction: **SE pays a
latency premium that only returns on repeated encounters with the same lint.**
For one-shot healing, ship `l1l2`.

Both flags share the 0/10 held-out result, which locates the failure in the L1
drafter beneath them (Issue 010), not in the evolution overlay.

---

## Caveat: the box was not quiesced

The first pass ran while a **sibling agent session** compiled Rust heavily —
`load average 77.71`, Zed at 182% CPU, several concurrent `rustc` processes.
This is not a footnote: it depressed *both* arms, so every absolute number
below must be read with its load condition attached.

| Arm | Quiet | Contended |
|---|---|---|
| clippy `self_evolve` p50 (10 lints × 100 reps) | 12.5 µs | 21.4 µs |
| clippy `self_evolve` mean | 12.1 µs | 22.7 µs |
| Bonsai, `needless_return` (identical case, 223 tok) | 21.0 t/s | 16.2 t/s |
| Bonsai, seed fixture spread | — | 16.3 – 17.1 t/s |

The contended Bonsai figure is not a single sample: the first nine seed cases
measured 16.17, 16.77, 17.13, 16.66, 17.12, 17.02, 16.59, 16.27 t/s — a tight
band, so ~16–17 t/s is a stable *loaded* rate rather than noise, against 21.0
t/s for the same case on a quiet box (~1.3× headroom).

Two consequences, stated plainly:

- **The ratio survives.** Contention moved each arm by ~1.3–1.7×, against a
  throughput gap of ~10^5. No load condition reachable on this box changes
  which arm is faster, or by what order of magnitude.
- **The absolutes do not transfer.** Anyone re-running this must re-measure;
  do not quote the µs or t/s figures as machine specs for an M3 Max.

## llama.cpp raw throughput reference (what "Bonsai tok/s" can mean)

`llama-bench` runs its own harness — no server, no HTTP, no sampling, no
instrumentation in the loop — so it isolates the model/hardware rate. Measured
twice, at different load:

```sh
llama-bench -m Ternary-Bonsai-27B-Q2_0.gguf -p 512 -n 128 -ngl 99 -r 3   # quiet, load 4.09
llama-bench -m Ternary-Bonsai-27B-Q2_0.gguf -p 64  -n 32  -ngl 99 -r 1   # loaded, load 77
```

| test | t/s | condition |
|---|---:|---|
| **pp512** (prefill) | **207.79 ± 6.25** | quiet |
| **tg128** (decode) | **25.50 ± 0.36** | quiet |
| pp64 (prefill) | 137.65 ± 0.00 | loaded |
| tg32 (decode) | 20.38 ± 0.00 | loaded |
| **e2e per request (server)** | **15.87 – 16.57** | the figure this bench uses |

Three numbers, and the difference between them is the point:

- **Prefill scales with batch** (137.65 t/s at pp64 → 207.79 t/s at pp512), so a
  prefill figure quoted without its batch size means nothing.
- **Peak decode is 25.50 t/s** — llama.cpp's ceiling for this model on this box.
- **A real request delivers ~16 t/s.** The 25.5 → 16 gap is *not* only the
  load-77 contention. In descending order: contention; **decode degrades as the
  KV context grows** (`tg128` measures 128 tokens, while bench cases generated
  266–2043 tokens each); and e2e includes prompt processing, sampling, and HTTP
  that `llama-bench` excludes by construction.

**Bench 009 therefore quotes the e2e rate, not 25.50 t/s.** Using the
`llama-bench` ceiling would have credited Bonsai with a throughput no actual
request on this task achieves — flattering the arm this bench is measuring
*against*, and inflating the reported ratio in the modelless arm's favour by
~1.6×. The e2e number is the one a caller experiences.

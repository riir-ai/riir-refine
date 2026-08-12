# Issue 009 — E2E tokens/sec: Ternary-Bonsai-27B (llama.cpp) vs the modelless pipeline

**Type:** proof / benchmark
**Status:** COMPLETE — both fixtures measured; verdict in
[Bench 009](../.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md)
**Date:** 2026-08-12
**Host:** M3 Max (Metal, 12 threads), macOS Darwin 25.5.0

## Question

What is the **end-to-end tokens/sec** of the riir-clippy modelless pipeline
(best feature flag) versus Ternary-Bonsai-27B-Q2_0 served by llama.cpp, on the
same clippy-healing task?

## Metric

```text
tok/s = tokens(emitted fix) / wall_clock(request → fix in hand)
```

Both arms produce the same deliverable: given `(lint_name, buggy_code)`, emit
fixed Rust source. The **numerator uses one tokenizer for both arms** —
Bonsai's own vocab via `llama-tokenize` — so the unit is identical. The clippy
arm emits raw Rust and gets tokenized post-hoc; it is not credited with
"free" tokens.

## Fairness constraints (all enforced in the harness)

1. **Shared fixture.** Both arms read the same `(lint, buggy, want)` triples
   from `benches/fixtures/e2e_cases_*.json`. No duplicated case list.
2. **Two fixtures, because one is not a fair read.**
   - `e2e_cases_seed.json` — the exact 10 pairs the seed corpus was built from.
     The clippy corpus **contains the ground-truth fix**, so this measures
     retrieval, not generalization.
   - `e2e_cases_heldout.json` — same 10 lints, unseen code. This is the
     generalization axis and the one that decides the quality verdict.
3. **Bonsai is a thinking model.** Its reasoning tokens count toward its own
   e2e time (correct — the user waits for them), but `--max-tokens` must be
   large enough to finish reasoning *and* emit code. At 512 tokens, 7/10 runs
   were cut off mid-reasoning and scored 0 for a budget reason, not a
   capability reason. Re-run at 2048 with a `truncated` column reported.
4. **Setup/load cost reported separately**, never silently amortized: Bonsai
   pays model load; the clippy arm pays corpus thaw + memory seeding.
5. **Quality is reported next to throughput.** tok/s on a wrong fix is not a
   win — per the repo's GOAT rule, a perf gain on an incorrect answer is a
   speedup of a wrong result.

## Gates

- **G1 (throughput):** report clippy tok/s ÷ Bonsai tok/s on both fixtures.
- **G2 (in-distribution quality):** exact-match rate on the seed fixture.
- **G3 (generalization quality):** exact-match rate on the held-out fixture.
  **This gate decides whether the throughput number means anything.**
- **G4 (honesty):** truncation count reported for the Bonsai arm; the clippy
  arm's emitted bytes checked for input-independence (does it emit the same
  fix regardless of the buggy code it was handed?).

## Reproduce

```sh
# Arm A — modelless pipeline (both fixtures)
CARGO_TARGET_DIR=/tmp/clippy_e2e_target cargo bench --bench bench_e2e_tokps --features self_evolve -- \
  benches/fixtures/e2e_cases_seed.json /tmp/clippy_e2e_seed.jsonl

# Arm B — Bonsai via llama.cpp (Q2_0 needs the prismml fork; upstream
# llama.cpp CANNOT load this file — see Bench 009 "Blocker" section)
/Users/katopz/git/vendor/prismml-llama.cpp/build/bin/llama-server \
  --model /Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  --port 8123 --ctx-size 4096 --temp 0.0 --seed 42 -ngl 99 --no-webui --jinja

python3 scripts/bench_bonsai_vs_clippy_tokps.py --port 8123 \
  --cases benches/fixtures/e2e_cases_seed.json \
  --clippy-jsonl /tmp/clippy_e2e_seed.jsonl --out /tmp/h2h_seed.json \
  --max-tokens 2048
```

---

## Outcome (2026-08-12)

| gate | result |
|---|---|
| **G1 throughput** | **CORRECTED** — same-condition pairs: tok/s **73,589×** (seed) / **67,918×** (held-out) both quiet; 84,711× / 89,997× both loaded. fixes/sec **2.02 M×** / **1.80 M×**. Bonsai 25.50 tok/s quiet (12.6–25.5 depending on GPU contention), 16.57 / 15.87 loaded; modelless 1.88 M / 1.73 M quiet. The originally published 113,242× / 109,146× divided a quiet clippy number by a loaded Bonsai number and were ~1.5× too high. |
| **G2 in-distribution quality** | modelless **10/10**, Bonsai **10/10** — a tie, but the modelless 10/10 is retrieval of its own corpus. |
| **G3 generalization quality** | modelless **0/10**, Bonsai **10/10** (9/10 by strict grader; the one miss is a grader false negative on `\|&x\|` vs `\|x\|`, both verified to compile and be semantically identical). **FAIL for the modelless arm.** |
| **G4 honesty** | 0/10 Bonsai truncations at 2048 tokens (7/10 at 512 — harness artifact, corrected). Modelless input-independence confirmed: 10/10 held-out emissions byte-identical to seed emissions → Issue 010. |

**Verdict:** cache-vs-compute, not model-vs-modelless. The modelless pipeline is
a ~74,000× faster *cache* over corpus-resident lint fixes and should ship as
one; it has no measured capability on unseen code, and its misses are silent
(L2 reports `parses=true` on all 10 wrong answers). Bonsai's accuracy is flat
across the distribution shift.

**Follow-ups filed:** [Issue 010](010_drafter_cannot_generalize_heldout_code.md)
(drafter returns corpus text verbatim — the root cause of G3).

**Flag note:** `MODE=l1l2` (default features) beat `MODE=self_evolve`
back-to-back on a quiet box for bit-identical quality on both fixtures, so the
premise that `self_evolve` is "the best flag" does not hold for single-shot
healing. Issue 008's gain needs the `report_outcome` feedback loop this bench
does not exercise.


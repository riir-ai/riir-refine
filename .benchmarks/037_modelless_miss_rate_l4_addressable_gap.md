# Bench 037 — the modelless MISS RATE: L4 is unreachable, and the gate is syntactic

**Date:** 2026-08-17
**Purpose:** riir-train [Plan 336](../../riir-train/.plans/336_code_llm_rlvr_clippy_l4_lora.md) T5.1 — measure the residual gap that could justify or retire Phase 5 (the L4 generative fallback)
**Harness:** `benches/bench_003_modelless_miss_rate.rs` (new)
**Hardware:** M3 Max — **CPU only, no GPU, no model load, no `cargo clippy`**. `heal()` is the whole measurement.
**Config:** `--features self_evolve,clippy_lints`, `HealPipeline` seeded via `seed_from_corpus`, Stage 2 **untuned**

---

## Why the miss set is the only metric that matters here

The shipped composition seam consults L4 on exactly two conditions
(`src/self_evolve.rs:517`, `:574` — enforced by `tests/g4_l4_composition.rs`):

- **miss #1** — `heal()` returns `None` (no candidate at all)
- **miss #2** — `heal()` returns `Some(r)` with `r.parses == false`

A modelless fix that **parses but is wrong** never reaches L4 — the invariant
`a_parsing_modelless_fix_is_never_replaced_by_l4` forbids it. So **exact-match
accuracy is not L4's addressable set**, and a poor EM score is not evidence that
L4 would help. Only the miss rate is.

---

## Results

| arm | cases | served/parses | miss #1 | miss #2 | **miss rate** | EM (context only) |
|---|---:|---:|---:|---:|---:|---:|
| seed (in-distribution) | 10 | 10 | 0 | 0 | **0.0%** | 10/10 (100%) |
| heldout (generalization) | 10 | 10 | 0 | 0 | **0.0%** | 0/10 (0%) |
| **out-of-corpus** (real lints with NO corpus entry) | 9 | 9 | 0 | 0 | **0.0%** | n/a |
| **combined** | **29** | **29** | **0** | **0** | **0.0%** | — |

**Verdict: the L4-addressable gap is 0/29. L4 is UNREACHABLE in the shipped
configuration** — not for lack of a good checkpoint, but because the pipeline
never declines. No adapter, however well trained, can be consulted.

---

## The out-of-corpus arm is the load-bearing one

Measuring only the two shipped fixtures would have produced 0% **by
construction**: both use the same 10 lint names, and all 10 have corpus entries,
so `corpus.lookup(lint)` always returns candidates. Real `cargo clippy` emits
hundreds of distinct lints, so out-of-corpus input is the *normal* production
case and the only place miss #1 can fire.

So the harness adds 9 real clippy lints verified absent from the corpus
(`or_fun_call`, `large_enum_variant`, `too_many_arguments`, `redundant_clone`,
`vec_init_then_push`, `manual_strip`, `box_collection`, `option_if_let_else`,
`ptr_arg`). The harness **asserts** `candidates.is_empty()` for each rather than
assuming it, so the arm fails loudly if the corpus grows to cover one.

**All 9 were still served a parsing fix. Miss rate 0/9.**

---

## Mechanism — and why this is a gating defect, not a success

With zero corpus candidates the only remaining paths are memory/evolution, and
memory always returns its nearest trajectory. Diagnostics from the run
(`source`, `echoes_input`, first line of the served fix):

| lint (input) | source | served fix (first line) |
|---|---|---|
| `ptr_arg` — `fn f(v: &Vec<u8>) -> usize { v.len() }` | `Memory` | `fn f() -> String { "done".to_string() }` |
| `redundant_clone` — `let s2 = s.clone(); drop(s2);` | `Memory` | `fn f(b: bool) -> bool { b }` |
| `too_many_arguments` — `fn f(a: u8, …, h: u8) {}` | `Memory` | `fn score(g: u8) -> u8 { g * 2 }` |
| `box_collection` — `struct S { v: Box<Vec<u8>> }` | `Memory` | `fn opt_double(x: Option<i32>) -> Option<i32> {` |
| `or_fun_call` — `let x = opt.unwrap_or(expensive());` | `Memory` | `struct S;` |

Every served answer is **valid Rust that is semantically unrelated to the
input** (`echoes_input=false` in all 9 — it is not even echoing the buggy code
back). Because it *parses*, the seam classifies it as a successful serve and
**suppresses L4**.

**The defect:** the seam uses a **syntactic** test (`parses`) as a proxy for
**relevance**. That proxy holds while input stays in-distribution and breaks
completely off-distribution, where the pipeline confidently emits an unrelated
snippet instead of declining. Two consequences:

1. **L4 can never fire**, so Plan 336 Phase 5 is blocked on a *gating defect*,
   not on checkpoint quality. Training a better adapter changes nothing until the
   pipeline can decline.
2. **Independently of L4, this is a quality problem.** For an unknown lint the
   pipeline returns nonsense that parses. `heal()`'s docs do place verification
   on the caller ("the caller is responsible for verifying the fix actually
   resolves the rule"), and `fix_verify`/`ClippyVerifier` would reject it
   downstream — but the L4 decision happens *inside* `heal()`, before any
   verification, so L4 stays suppressed regardless.

Filed as [Issue 030](../.issues/030_l4_seam_syntactic_gate_suppresses_fallback.md).

---

## Correction to an earlier claim in this session

I previously stated that the modelless path "scores 10/10 on held-out" (commit
`8ca3b2c`). That is true **only with Stage 2 tuned**, which requires the real
`cargo clippy` verifier via `tune_against_fixture`. This bench runs Stage 2
**untuned**, and measures held-out EM at **0/10** — the Stage-1 path serves
parsing-but-wrong fixes. The two numbers do not conflict; they describe tuned vs
untuned. The blanket phrasing was imprecise and the distinction matters, because
Stage-2 tuning is what buys generalization.

It does **not** affect this bench's verdict: Stage-2 tuning can only *reduce* the
miss rate, and the miss rate is already 0. So the untuned run is a valid **upper
bound**, and 0% is 0% either way.

---

## What this does not measure

- **A large corpus.** 29 cases (10 + 10 + 9). The out-of-corpus arm is 9
  hand-picked lints, not a sample of real `cargo clippy` output on a real repo.
  A wider sweep would sharpen the rate but cannot raise it above 0 while memory
  always answers.
- **Stage-2-tuned behaviour.** Untuned only (needs cargo + seconds per rule).
  Direction of the bias is known and stated above.
- **Fix quality on the out-of-corpus arm.** Deliberately not scored — those
  inputs have no `want`, and the question asked was reachability, not accuracy.
- **Whether a relevance gate is cheap.** Issue 030's job, not this bench's.

## Recommendation for Plan 336 Phase 5

**Do not train toward T5.1.** The prerequisite is a decline path in the seam
(Issue 030). Order of operations:

1. Fix the gate so out-of-distribution input yields a miss instead of an
   unrelated parsing snippet.
2. Re-run this bench. The miss rate then becomes the real L4-addressable gap.
3. Only if that gap is non-trivial does T5.1/T5.3/T5.4 have a justification —
   and G1 (currently 0/60) still has to clear before promotion.

## Reproduce

```sh
cd riir-clippy
CARGO_TARGET_DIR=/tmp/b003_missrate cargo bench \
  --bench bench_003_modelless_miss_rate --features self_evolve,clippy_lints
# writes /tmp/bench003_miss_rate.json (per-arm counts + l4_reachable flag)
```

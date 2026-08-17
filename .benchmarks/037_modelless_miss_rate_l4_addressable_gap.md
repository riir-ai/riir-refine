# Bench 037 — the modelless MISS RATE: L4 was unreachable (syntactic gate) — FOUND and FIXED

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

## Results — BEFORE the fix (the measurement that found the defect)

| arm | cases | served/parses | miss #1 | miss #2 | **miss rate** | EM (context only) |
|---|---:|---:|---:|---:|---:|---:|
| seed (in-distribution) | 10 | 10 | 0 | 0 | **0.0%** | 10/10 (100%) |
| heldout (generalization) | 10 | 10 | 0 | 0 | **0.0%** | 0/10 (0%) |
| **out-of-corpus** (real lints with NO corpus entry) | 9 | 9 | 0 | 0 | **0.0%** | n/a |
| **combined** | **29** | **29** | **0** | **0** | **0.0%** | — |

**Verdict: the L4-addressable gap was 0/29. L4 was UNREACHABLE in the shipped
configuration** — not for lack of a good checkpoint, but because the pipeline
never declined.

## Results — AFTER the relevance gate (commit `d8440f6`)

The defect this bench found was fixed in the same session. Same harness, same
fixtures, re-run against the gated pipeline:

| arm | cases | served/parses | miss #1 | miss #2 | **miss rate** | EM (context only) |
|---|---:|---:|---:|---:|---:|---:|
| seed (in-distribution) | 10 | 10 | 0 | 0 | **0.0%** *(unchanged)* | 10/10 (100%) *(unchanged)* |
| heldout (generalization) | 10 | 10 | 0 | 0 | **0.0%** *(unchanged)* | 0/10 (0%) *(unchanged)* |
| **out-of-corpus** | 9 | **0** | **9** | 0 | **100.0%** *(was 0.0%)* | n/a |
| **combined** | **29** | **20** | **9** | **0** | **31.0%** | — |

`l4_reachable = true`. **The L4-addressable gap is now 9/29 (31.0%)**, and it is
entirely the out-of-corpus arm — exactly where it should be. In-distribution
behaviour is bit-for-bit unchanged: seed still serves 10/10 at 100% EM, held-out
still serves 10/10, and L4 is never consulted for any corpus rule.

**The fix** (`src/self_evolve.rs`, commit `d8440f6`): the serve/decline decision
gains a provenance term, `relevant = rule_in_corpus || cand.source != Memory`,
computed from the corpus scan that already existed for `rule_context_notes`. Miss
handling moved to `HealPipeline::resolve_miss`, which distinguishes *relevant but
non-parsing* (hand it back, pre-existing behaviour) from *irrelevant* (decline).
Verified by 3 new tests in `tests/g4_l4_composition.rs` (13/13 pass, including all
10 original invariants) and 11/11 `self_evolve` lib tests.

**Consequence for Plan 336 Phase 5:** it flips from *unjustifiable* to *justified
if a checkpoint can beat decline*. G1 (0/60) still has to clear before promotion —
the gap being real does not make the adapter good.

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

**Before the fix: all 9 were served a parsing fix. Miss rate 0/9.** After the
relevance gate (`d8440f6`): all 9 decline. Miss rate 9/9.

---

## Mechanism (pre-fix) — and why this was a gating defect, not a success

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

Filed as Issue 030 — **RESOLVED and removed** per the noise-reduction rule; the
fix is commit `d8440f6` and this bench is the record. See §"Results — AFTER the
relevance gate".

---

## Correction to an earlier claim in this session

I previously stated that the modelless path "scores 10/10 on held-out" (commit
`8ca3b2c`). That is true **only with Stage 2 tuned**, which requires the real
`cargo clippy` verifier via `tune_against_fixture`. This bench runs Stage 2
**untuned**, and measures held-out EM at **0/10** — the Stage-1 path serves
parsing-but-wrong fixes. The two numbers do not conflict; they describe tuned vs
untuned. The blanket phrasing was imprecise and the distinction matters, because
Stage-2 tuning is what buys generalization.

It does **not** affect the pre-fix verdict: Stage-2 tuning can only *reduce* a
miss rate, and the pre-fix rate was already 0, so the untuned run was a valid
**upper bound** and 0% is 0% either way. Post-fix it is a genuine caveat on the
9/29 figure — tuning Stage 2 could serve some out-of-corpus rules and lower the
31.0%, so **9/29 is an upper bound on the addressable gap**, not a floor.

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
- **Whether a relevance gate is cheap.** Answered by the fix itself: it reuses an
  existing corpus scan and adds one boolean, and `heal_is_alloc_free_after_warmup`
  still passes — so the gate costs nothing measurable on the hot path.

## Recommendation for Plan 336 Phase 5

Steps 1 and 2 below are **DONE** in this session; step 3 is where Phase 5 now sits.

1. ~~Fix the gate so out-of-distribution input yields a miss instead of an
   unrelated parsing snippet.~~ **DONE** — commit `d8440f6`, Issue 030 resolved.
2. ~~Re-run this bench. The miss rate then becomes the real L4-addressable gap.~~
   **DONE** — **9/29 (31.0%)**, all of it the out-of-corpus arm.
3. **The gap is real, so T5.1/T5.3/T5.4 now have a justification — but the
   adapter still does not.** G1 is 0/60 (T4.1) and T2.7's re-gate repeated 0/60
   DEGENERATE (Bench 467). The bar is now concrete and stricter than "score
   something": on the 9 reachable cases the adapter must beat **decline**, and
   declining is a *correct* answer. A wrong-but-parsing L4 fix is worse than the
   `None` the pipeline now returns, because it re-introduces exactly the
   false-confidence failure this bench was written to catch.

**So promotion of `l4_lora` still fails its gate, for an unchanged reason (G1),
not for want of reachability.** What changed is that the work is no longer
pointless — it is merely unproven.

## Reproduce

```sh
cd riir-clippy
CARGO_TARGET_DIR=/tmp/b003_missrate cargo bench \
  --bench bench_003_modelless_miss_rate --features self_evolve,clippy_lints
# writes /tmp/bench003_miss_rate.json (per-arm counts + l4_reachable flag)
```

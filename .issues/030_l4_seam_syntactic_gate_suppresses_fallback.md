# Issue 030 — the L4 seam gates on `parses`, which suppresses the fallback entirely

**Filed:** 2026-08-17
**Found by:** [Bench 037](../.benchmarks/037_modelless_miss_rate_l4_addressable_gap.md) (riir-train Plan 336 T5.1 measurement)
**Severity:** blocks riir-train Plan 336 Phase 5 (T5.1/T5.3/T5.4); independent quality bug for off-distribution lints
**Status:** OPEN

## What is wrong

`HealPipeline::heal` consults the L4 fallback on exactly two conditions:

- `src/self_evolve.rs:517` — miss #1: no candidate at all (`None`)
- `src/self_evolve.rs:574` — miss #2: the best candidate does not parse

Both are **syntactic**. Neither asks whether the candidate is *relevant to the
lint being healed*. Off-distribution, the memory path always returns its nearest
stored trajectory, that trajectory is valid Rust, so `parses == true`, so the
seam records a successful serve and **L4 is never consulted**.

## Measured evidence (Bench 037)

Miss rate is **0/29** across three arms — including 9 real clippy lints
**verified to have no corpus entry** (the harness asserts
`candidates.is_empty()`). All 9 were served a parsing fix from `source=Memory`,
semantically unrelated to the input, with `echoes_input=false`:

| lint (input) | served fix (first line) |
|---|---|
| `ptr_arg` — `fn f(v: &Vec<u8>) -> usize { v.len() }` | `fn f() -> String { "done".to_string() }` |
| `redundant_clone` — `let s2 = s.clone(); drop(s2);` | `fn f(b: bool) -> bool { b }` |
| `too_many_arguments` — `fn f(a: u8, …, h: u8) {}` | `fn score(g: u8) -> u8 { g * 2 }` |
| `or_fun_call` — `let x = opt.unwrap_or(expensive());` | `struct S;` |

## Two consequences

1. **L4 is unreachable.** Plan 336 Phase 5 is blocked on this gate, not on
   checkpoint quality. Training a better adapter cannot help while the seam never
   asks for one. This reverses the standing reading that Phase 5 only awaits a
   G1-passing checkpoint.
2. **Off-distribution output is nonsense that parses.** `heal()`'s contract does
   put verification on the caller, and `fix_verify` / `ClippyVerifier` would
   reject these downstream — but the L4 decision happens *inside* `heal()`,
   before verification, so the suppression stands regardless of what the caller
   does afterwards.

## Why `parses` was a reasonable proxy, and where it breaks

In-distribution (corpus-covered lints) a parsing candidate is almost always
relevant, because it came from a corpus entry for that lint. The proxy is
load-bearing only off-distribution, where the candidate's provenance is the
memory nearest-neighbour rather than a lint-matched entry. The bug is the
*unconditional* trust in `parses`, not the use of `parses` at all.

## Fix direction (modelless — no model, no GPU)

A relevance predicate on the served candidate, applied before the seam decides
"served". All candidates are modelless and cheap:

- **Provenance gate (simplest, likely sufficient):** if the candidate came from
  `HealSource::Memory` **and** no corpus entry exists for `rule_id`, treat it as a
  miss. Pure bookkeeping — the information is already at the call site.
- **Latent relevance threshold:** dot-product of the input's latent embedding
  against the trajectory's, sigmoid-bounded, with a floor below which the answer
  is a miss. Matches the repo's latent-domain rules (sigmoid, not softmax).
- **Rule-identity gate:** require the served trajectory's `rule_id` to match the
  requested `rule_id` unless the drafter (not memory) produced it.

Any of these turns the 0% miss rate into a real number, which is the actual
prerequisite for deciding Phase 5.

## Acceptance

- [ ] A relevance/decline condition exists and is unit-tested alongside the
      existing four invariants in `tests/g4_l4_composition.rs`.
- [ ] The existing 10 composition invariants still pass (no regression) —
      especially `a_parsing_modelless_fix_is_never_replaced_by_l4`, which must
      keep holding for *in-distribution, relevant* fixes.
- [ ] Bench 037 re-run: seed arm miss rate stays 0%; out-of-corpus arm miss rate
      becomes > 0% (declines instead of serving nonsense).
- [ ] Held-out EM unchanged or better once Stage 2 is tuned (the gate must not
      break the path that Issue 010 was closed with, commit `8ca3b2c`).
- [ ] riir-train Plan 336 T5.1 updated with the resulting real miss rate.

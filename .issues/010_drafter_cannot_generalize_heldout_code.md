# Issue 010 — L1 drafter emits corpus templates verbatim: 0/10 on held-out code

**Type:** proof / architecture defect
**Status:** OPEN — root-caused, no fix proposed yet
**Found by:** [Bench 009](../.benchmarks/009_bonsai_vs_modelless_e2e_tokps.md) (2026-08-12)
**Severity:** blocks any generalization claim for L1/SE; does NOT affect the
shipped GOAT gates (they only ever measured in-distribution corpus lints)

## Symptom

On the held-out fixture (`benches/fixtures/e2e_cases_heldout.json` — the **same
10 lints**, different code), the pipeline scores **0/10 exact-match** while
scoring **10/10** on the seed fixture. Worse than a low score: **10/10 of the
held-out emissions are byte-identical to the seed emissions.**

```text
held-out input : fn bar(x: i32) -> i32 { return x + 1; }
emitted        : fn foo() -> i32 { 42 }        <-- the SEED entry, verbatim
wanted         : fn bar(x: i32) -> i32 { x + 1 }
```

Every held-out case behaves this way. The L2 pruner still reports
`parses = true` 10/10, because the emitted code *is* syntactically valid Rust —
it is simply a different function than the one the caller asked about. **The
always-on validator cannot catch this class of failure.**

## Root cause (confirmed in source, not inferred)

`src/drafter.rs::draft_fix` constructs its output as:

```rust
let entry = &candidates[idx];
Some(DraftedFix { code: entry.fixed_code.clone(), .. })   // <-- verbatim corpus text
```

`buggy_code` enters the computation **only** as BLAKE3 hash bytes →
`simd_ternary_matvec` → FNV-1a fold → `idx = hash % candidates.len()`. So the
input code selects *which stored fix to return*; it never influences the
returned text. The drafter is a **corpus retriever with a hash-driven index**,
not a code transformer.

Because the seed corpus holds exactly **one** entry per lint,
`candidates.len() == 1` ⇒ `idx == 0` always ⇒ the single stored `fixed_code` is
the only reachable output for that lint, for any input whatsoever.

**Generalization is architecturally out of reach here, not a tuning gap.** No
amount of weight/EMA refinement changes it: the output alphabet for a lint is
the set of `fixed_code` strings already in the corpus.

## What this does and does not invalidate

| Claim | Status |
|---|---|
| Bench 001 L1 latency (p50 2.3 µs), Bench 002 L2 latency | **Unaffected** — latency is real. |
| Issue 008 G6/G7 (+50 pp healing rate on real `cargo clippy`) | **Unaffected but in-distribution** — measured on corpus lints whose fixes the corpus contains. |
| Bench 009 throughput ratio (~10^5×) | **Unaffected** — it is a real throughput measurement. |
| "The modelless pipeline heals clippy warnings" | **Scope-limited** to `(lint, code)` pairs already in the corpus. On unseen code it returns a valid-but-wrong function. |

## Why the pruner does not save us

L2 (`syn` parse) answers "is this Rust?", not "is this a fix for *my* code".
The `clippy_verify` path (Phase 3) re-runs real `cargo clippy` and *would*
catch it, but it is opt-in and ~130 ms — 4 orders of magnitude above the
drafter's own hot path, so it cannot be the default guard.

## Candidate directions (none validated — do not treat as a plan)

1. **Span-local edit, not whole-body replacement.** The obvious modelless fix:
   treat a corpus entry as a *rewrite rule* (a `syn`-level pattern → template)
   applied to the caller's AST, rather than a literal string to hand back.
   Emits the caller's own identifiers by construction. This is real work and
   is per-lint.
2. **Identity guard (cheap, and honest).** Detect that the emitted fix shares
   no identifiers with the input and refuse to emit rather than returning a
   confidently-wrong function. Turns a silent wrong answer into a decline —
   strictly better than today, and near-free.
3. **Corpus breadth.** More entries per lint raises `candidates.len()` but the
   output is still drawn from stored strings; this does not fix the class of
   failure, only widens the memorized set.

Direction 2 is the small honest step; direction 1 is the actual capability.
Both need a GOAT gate against the held-out fixture, which now exists and should
be the standing regression target.

## Reproduce

```sh
CARGO_TARGET_DIR=/tmp/clippy_e2e_target cargo run --release \
  --example bench_e2e_tokps --features self_evolve -- \
  benches/fixtures/e2e_cases_heldout.json /tmp/clippy_e2e_heldout.jsonl
# => exact-match: 0/10, parse-valid: 10/10
```

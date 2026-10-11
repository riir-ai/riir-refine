# Research 132 — Batch 109 (post-mining P10): the LTO fold-elimination timing-honesty rule

**Status:** RECORD — landed with Batch 109 (corpus 99 → 100 rust_perf; see the commit for the
hash). Consumes the post-mining intake's single open class (P10, 2026-09-04).

## Source

- Intake: `.distill/001_mining_queue_snapshot.md` §Post-mining intake, P10 entry
  (2026-09-04), authored from the katgpt-rs Issue 723 Class A2 fixes (`b4cef008`).
- Evidence files: `katgpt-rs/tests/bench_gdsd_modelless.rs` (the `let _ =
  baseline.relevance(..)` timed loops) + `katgpt-rs/tests/rv_gated_routing.rs` sink docs
  (the 0 ns → NaN% release-life gate).
- Probe numbers (from the intake, measured by the finding session): used result 16.6 µs vs
  `let _ = f()` ≈ 0 in ONE binary, rustc 1.98.1 + fat LTO.

## What landed

`dead-result-timing-loop-folded-under-lto` (rust_perf, complexity, suggestion-only,
`bench_spec: None`, annotated NotBenchable — the first rule on the measurement-honesty
axis). Buggy = timed loop discarding its work result. Fixed = chained state (xorshift-PRNG
shape) + consumer-side `black_box` + the loud nonzero-baseline assert.

## The gate-shaped the rule (the honest part — TWO refutations of the first draft)

The batch's own G1 fixture caught TWO further cancellation modes beyond the intake's
constant-work fold, each now pinned as an assertion in
`batch109_sink_consumes_per_iteration_data`:

1. **XOR sink over a GF(2)-linear work fn cancels exactly.** xorshift is linear over
   GF(2); XOR of `work(i)` for i in 0..2^k equals `work(XOR of inputs)` = `work(0)` = 0.
   The first draft's fix pattern (`sink ^= work(i)`) was ITSELF foldable to zero with
   per-iteration data — the gate measured `xor_sink == 0` and failed. Batched XOR of a
   linear function carries no information regardless of input variety.
2. **wrapping_add over a balanced power-of-two batch collapses to a seed-independent
   constant.** The second draft used `sink = sink.wrapping_add(work(seed ^ i))` — carries
   break GF(2) linearity, so it survived draft 1's trap. But over i in 0..4096, every
   output bit of the (thoroughly bit-spread) work fn is 1 in exactly 2048 of the 4096
   terms regardless of seed, so the sum is Σ_b 2^b · 2048 = 2048·(2^64 − 1) mod 2^64 =
   **2^64 − 2048 = 18446744073709549568** for EVERY seed — measured, both arms identical.
   Balanced-bit cancellation: any associative combine whose per-bit terms are seed-balanced
   folds over power-of-two batches.

The robust fix pattern is **chained state**: each iteration consumes the previous
accumulator (`state = work(state ^ black_box(i))`), the xorshift-PRNG shape — the final
value depends on the whole sequential history and no batch algebra can fold it. Pinned
non-vacuous: `chained(0) != chained(0xDEAD_BEEF)`.

## Rule content summary

- `black_box` INSIDE the callee does not save work the CALLER discards (one CFG under fat
  LTO; the barrier pins the callee's return value, not the caller's dead use).
- NaN direction contract: NaN compares false against everything — a `>=` PASS bar fails
  loudly on a zero baseline, a `<=` budget bar passes silently forever; the
  nonzero-baseline assert must fire BEFORE the ratio.
- Ratio-gate harness discipline (from the intake's in-tree findings): equal non-pruner
  work in both arms (one shared timed fn) + interleaved (baseline, candidate) chunks with
  median-of-ratios (sequential arms drifted +5.2%/+21.7% thirty seconds apart; paired
  medians read −0.0%/+0.6% twice).
- Differentiation: `debug-stat-full-scan-out-of-hot-path` (unconsumed PRODUCTION
  statistic; this is the HARNESS side), `encoded-layout-borrowed-view`'s bench note (same
  physics from the bench-consumption side).

## Gates

- G1 syn parse (buggy + fixed) ✓; corpus truthfulness membership ✓
- G2 default lib 1082/0 · rust_perf_bench 1100/0 · latent_retrieval 1189/0/1i ✓
- G3 clippy 0 real findings both states (exFAT hard-link noise only) ✓; retrieval
  precision regression gate PASS, floors HELD, zero re-pins ✓
- Count pins walked: corpus.rs header (100/19/14), build fn doc (100), entries pin (100),
  annotated pins 42→43 ×4 sites + fn rename forty_two→forty_three, unannotated message
  (100 − 43), NotBenchable pin 13→14 + fn rename thirteen→fourteen + membership assert,
  batch109 present/parse/concepts/G1 gates added. AGENTS.md + .docs updated in the same
  commit.

## Consumers

Any timing/latency gate in the workspace with a discarded-result loop; the detector to
adopt first is the loud nonzero-baseline assert (machine-checkable, no proposer needed).
katgpt-rs Issue 723's per-target triage is the standing consumer lane.

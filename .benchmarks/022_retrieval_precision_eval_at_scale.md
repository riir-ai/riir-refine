# Bench 022 — Issue 017: retrieval-precision eval at scale

**Date:** 2026-08-16
**Feature:** `latent_retrieval` (eval module) + `kernel_opt` (kernel probe)
**Verdict:** GOAT G1–G4 PASS; **the 12-fixture 100% top-1 claim is dead at
scale — clippy top-1 is 58.3% at 48 spans / 40-rule corpus. The crowding
concern was justified.**

## What this is

The scaled, repeatable eval harness Issue 017 asked for — closes the gap
between Bench 018's 12-item fixture (100% top-1) and the real corpus (40
clippy + 50 rust_perf + 343 kernel rules at land time).

- **Module:** `src/draft/retrieval_eval.rs` + fixture data files
  `eval_fixtures_clippy.rs` / `eval_fixtures_perf.rs`.
- **One command** (prints the metrics table + miss taxonomy + clean profile):
  `cargo test --features latent_retrieval --lib retrieval_precision_regression_gate -- --nocapture`
- **Kernel-inclusive** (on-demand — the 343-rule multi-scale index makes this
  run ~54 s):
  `cargo test --features latent_retrieval,kernel_opt --lib retrieval -- --nocapture`

## Fixture composition (honest provenance)

| Set | n | Source | Label oracle |
|---|---:|---|---|
| clippy | 48 | 40 hand-authored variants (one per lint; real `cargo clippy` 1.93 exactly-one-lint sweep at authoring, subagent-verified) + 8 second-instances of high-frequency lints | OUR proposers (`clippy_fixture_labels_verify_via_proposers`: own-lint fires + zero cross-fires across all 40) |
| rust_perf | 40 | 30 hand-authored variants of corpus `buggy_code` shapes + 10 second-wave additions | 5 proposer-covered rules re-verified via `perf_fixture_labels_verify_via_proposers`; the rest semantic labels authored against the corpus |
| kernel | 30 | deterministic self-variant probe: every step-th corpus `ref_shape`, identifiers renamed via the fixed `KERNEL_RENAMES` map (`mutate_shape`) | by construction (optimistic upper bound — measures crowding displacement of own-variants) |
| clean | 12 | REAL production excerpts from this crate (`corpus_index.rs`, `edit_emitter.rs`, proposers, structs) | `clean_spans_fire_no_lint` vs the 37 STRUCTURAL lints |

Total: 118 labeled + 12 clean = **130 spans** (100 static without
`kernel_opt`; the `fixture_counts_meet_the_issue_floor` test pins the ≥100
floor across feature states).

**Why the clean oracle excludes 3 lints:** the type-blind self_evolve-only
proposers (`needless_borrow`, `useless_conversion`, `unnecessary_mut_passed`)
over-match on parse-only information by design (e.g. every necessary
`f(&local)` borrow — real production code trips them). That is exactly why
they are not `--fix` eligible; using them as a negative oracle on real code
would be wrong.

## Baseline (clippy+perf state, 88 labeled spans)

| domain | labeled | top-1 | top-3 | top-5 |
|---|---:|---:|---:|---:|
| clippy | 48 | **58.3%** | 75.0% | 87.5% |
| perf | 40 | **87.5%** | 95.0% | 97.5% |
| blended | 88 | 71.6% | 84.1% | 92.0% |

Kernel-inclusive (118 spans): kernel **100/100/100** (self-variants hold at
343 rules — BM25 token overlap dominates; a drop here is the crowding
alarm), blended 78.8/88.1/94.1.

**Stability:** identical output across 3 separate processes (the ranking
tie-break drains a `HashMap` with process-seeded `RandomState`; no boundary
tie observed in practice). Floors are pinned at baseline − ONE span as cheap
robustness, documented in the `floors` module.

## Miss taxonomy (all 14 misses, clippy+perf state)

Wrong-domain (router): 0 misses caused — retrieval is fan-out, the router is
annotation-only. Right-domain-wrong-rule:

- `identity_op` tops 5 clippy misses (map_unwrap_or, double_parens,
  unused_unit, deref_addrof, uninlined_format_args) — tiny token signatures
  (single-expression patterns) over-match longer spans' lines.
- `unnecessary_mut_passed` tops 2 (needless_borrow, single_char_pattern).
- `needless_match` ↔ `redundant_pattern_matching` ↔ `manual_unwrap_or`
  cluster confusion (3 misses).
- Type-blind lints retrieve worst (`redundant_closure`, `needless_borrow`
  not-in-top8) — their fixtures are ordinary-looking code, few distinctive
  tokens.
- Perf: `dyn-to-enum` outranked by `simd-chunk` (rank 3);
  `shrink-to-fit-after-build` not-in-top8 (top: `arc-str-payload`) — the
  second-wave fixture is a `with_capacity`+push loop, token-close to several
  allocation rules.
- Clean-span false-positive profile: `unused_unit` / `unnecessary_sort_by` /
  `unnecessary_mut_passed` top clippy; `mutex-to-atomic` / `wasm-batch-ffi`
  top perf. Consistent with the no-threshold finding (Bench 018 addendum).

## Router re-gate recommendation (feeds Issue 020)

Router annotation accuracy on labeled spans: **53.4%** (2 domains) /
**46.6%** (3 domains). At that accuracy a GATED router would starve the
correct domain on every other span. Recommendation: fan-out stays the
shipped behavior; the Issue 020 spectral POC's kill-bar is ≥85% routing
accuracy on THIS fixture set — below that, gating trades correctness for a
fraction of 349 µs/chunk. The eval now measures this number per batch.

## Regression gate

`retrieval_precision_regression_gate` (runs in the `latent_retrieval` suite):
blended ≥ 62/73/80 hits of 88, clippy ≥ 27/35 of 48, perf top-3 ≥ 37/40,
kernel top-3 ≥ 29/30 (combined state). A corpus batch that decays precision
below floor FAILS the suite — re-measure, re-pin, record the delta in the
batch's Research note. Never weaken silently.

For KERNEL batches (the frequent case), the mining GOAT's `--features
kernel_opt` suite does NOT include the eval (no `latent_retrieval`) — run
the combined-state command above as the batch's re-measure step (SKILL.md
Protocol A step 5 now says so).

## Proposer fix landed as a side effect (root cause, not suppression)

The fixture pass exposed that our `needless_bool_assign` proposer matched
the assign-RHS form (`x = if c { true } else { false }`) — which real
clippy reports as `needless_bool`, not `_assign`. Real clippy's shape is
branch assignments (`if c { x = true; } else { x = false; }`). Fixed the
proposer to the real shape (statement-level splice, emits its own `;`),
rewrote its fixtures + the corpus entry + the `all_lints_fire_and_parse`
row. The eval fixture (real-clippy-correct branch form) now fires it.

Known semantics drift, NOT fixed here (recorded): our
`comparison_to_empty` implements `.len() == 0`, which real clippy 1.93
reports as `len_zero`; real `comparison_to_empty` is `s == ""` → widening
the proposer to the `== ""` shape is a small follow-up.

## GOAT

- **G1 correctness:** label tests — 48/48 clippy fixtures fire own lint +
  zero cross-fires (40×40 matrix); 5/5 proposer-covered perf fixtures fire;
  12/12 clean spans fire no structural lint; kernel mutation deterministic
  (selection + rename, two calls bit-identical); eval output deterministic
  across runs and processes.
- **G2 perf:** eval adds ~2.8 s to the `latent_retrieval` lib suite (88
  spans, multi-scale queries, 2 domains) — no per-tick hot path touched.
  Kernel-inclusive run ~54 s (on-demand only).
- **G3 no-regression:** 370 default / 391 `latent_retrieval` / 526
  `kernel_opt` lib tests pass; `clippy_verify` + `fix_verify` states
  compile; clippy 0 new warnings in both standard states (the 1 remaining
  `latent_retrieval`-state warning is the sibling's pre-existing
  `heal.rs:415`).
- **G4 alloc:** N/A (test/eval path, not runtime); scratch reuse inherited
  from `LatentMatcher`.

## Files

- `src/draft/retrieval_eval.rs` — harness + oracle tests + gate
- `src/draft/eval_fixtures_clippy.rs` — 48 spans
- `src/draft/eval_fixtures_perf.rs` — 40 spans
- `src/domains/clippy_lints/proposers/needless_bool_assign.rs` — real-shape fix
- `src/domains/clippy_lints/corpus.rs` + `proposers/mod.rs` — entry/row updates

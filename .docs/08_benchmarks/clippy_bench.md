# clippy-bench — the frozen real-code healing bench + score curve

> **Plan 014.** The bench that can falsify "we're getting better": a FROZEN
> fixture set healed by the production `--fix` path, scored against real
> `cargo clippy` before/after, with an append-only score history rendered
> as an SVG curve. The denominator never grows with the corpus — the score
> can only move by healing better (or worse, which is the point).

## Why (the gap it closes)

| prior number | why it couldn't answer the question |
|---|---|
| retrieval precision (Bench 022→032) | authored fixtures coupled to the corpus; kernel rows are self-variant (circular) |
| corpus rule count | coverage metric — grows by construction, cannot fall |
| Bench 009's 0/10 held-out | measures the L1 verbatim cache tier, not the proposer path |
| the sweep campaign | real evidence, but episodic — no unified score, no time series |

## The fixture set (frozen v1, 34 fixtures)

| provenance kind | count | note |
|---|---|---|
| `real:` mined from pre-sweep sibling commits | 7 | std-only fns that compile standalone; the bare-stub filter drops crate-internal spans by construction |
| `authored-composite:` T3.5.5 spans | 27 | realistic MULTI-lint spans (2+ co-applying lints), oracle-labeled at freeze — NOT the degenerate minimal set |
| `real:` clean (decline axis) | included in the 7 | correct behavior = propose NOTHING |

Mining: `examples/score_bench_mine.rs` (`git show <sha>:<path>`, top-level
fns, the oracle's `compiles` flag is the std-only filter). Freeze labels at
mine time; every run re-derives + warns loudly on toolchain drift.

## Axes

| axis | definition |
|---|---|
| `heal_rate` | Σ resolved vocabulary lints / Σ vocabulary lints before |
| `created_rate` | Σ newly-created clippy lints (ANY) / Σ vocab-before — the false-positive axis |
| `parse_safety` | healed output still compiles / edited fixtures |
| `decline_correctness` | clean fixtures untouched (declining is a correct answer) |

Plus the **miss map**: which lints persist where, most-frequent first —
the diagnostic for WHERE healing still fails.

## Baseline (run #1, 2026-08-19)

heal 84.6% · created 3 (5.8%) · safety 92.9% · decline 100% — and the run
caught a REAL proposer bug (Issue 032: `needless_range_loop` ×
`identity_op` composition breaks the loop variable) on its first
execution. Fixing it is the first step the curve will show.

## Run

```sh
cargo run --features score_bench --bin cargo-heal -- --score-bench   # run + append + render
cargo run --features score_bench --bin cargo-heal -- --score-report  # re-render SVG only
```

History: `.benchmarks/score_history.jsonl` (append-only) · curve:
`.benchmarks/score_history.svg` (embedded in the repo README).

## Honest scope + Phase 3

Expect a STEP curve (root-cause fixes), not gradient descent — the
self-learning mechanisms measured negative and were killed (Plan 005
Phase 7 / Issue 026); improvement is engineering-iterative. Deferred:
perf/format axes (Phase 2), external crates.io source + per-fixture stub
manifests + cold/warm memory lines + statement-wrap mining for
crate-internal spans (Phase 3 — grows the `real:` fraction).

## See also

- [Plan 014](../../.plans/014_clippy_bench.md)
- [Issue 032](../../.issues/032_needless_range_loop_composition_breaks_loop_var.md) — the bug baseline run #1 caught
- [`clippy_oracle.rs`](../../src/draft/clippy_oracle.rs) — the T3.5.5 real-clippy oracle

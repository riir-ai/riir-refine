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

## Score history (the curve so far)

| run | sha | heal | created | safety | decline | note |
|---|---|---|---|---|---|---|
| #1 2026-08-19 | `17e3d8b` | 84.6% | 3 (5.8%) | 92.9% | 100% | baseline — caught a REAL proposer bug on its first execution (Issue 032) |
| #2 2026-08-19 | `86ba7f1` | **90.4%** | 3 (5.8%) | **100%** | 100% | Issue 032 fixed (atomic whole-loop edit) — the first step the curve shows: +5.8pp heal, safety back to 100 |

Run #1's two parse-safety failures were exactly the Issue 032 fixtures
(`needless_range_loop` × `identity_op`/`double_parens` same-pass composition
half-applied the rewrite → dangling loop variable). The remaining miss map
(needless_range_loop, double_parens, needless_return,
redundant_pattern_matching, useless_format — one each) is the honest open
heal surface; `created_rate` 3 is the separate false-positive class.

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
- Issue 032 (resolved + removed 2026-08-19, fix `86ba7f1`) — the partial-application composition bug baseline run #1 caught; the canonical record now lives in the score-history table above
- [`clippy_oracle.rs`](../../src/draft/clippy_oracle.rs) — the T3.5.5 real-clippy oracle

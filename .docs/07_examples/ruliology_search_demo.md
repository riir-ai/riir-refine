# ruliology_search_demo — L3 GOAT gate

**Source:** [`examples/ruliology_search_demo.rs`](../../examples/ruliology_search_demo.rs)
**Feature:** `ruliology_search` (opt-in)
**Run:** `cargo run --example ruliology_search_demo --features ruliology_search --release`

## What it demonstrates

The L3 GOAT gate for Pareto-front candidate search:

- **G1 (correctness)**:
  - G1a: `pareto_front` filters dominated candidates
  - G1b: `ast_delta_size` gives simpler fixes lower complexity
  - G1c: `best_fix` picks the minimum-complexity Pareto-optimal fix
- **G2 (latency)**: search over the seed corpus (10 lints) completes in
  < 10ms. Measured: ~393µs.
- **G3 (no-regression)**: default features (no `ruliology_search`) still
  pass.
- **G4 (alloc-free)**: N/A — runs once per invocation, not per-tick.

## Output shape

```text
=== Issue 005 GOAT gate: L3 ruliology Pareto-front search ===

G1 (correctness — Pareto-front + minimum-complexity pick):
  pareto_front filters dominated: front=[2, 3] → ✅
  ast_delta simpler < complex: 1 < 4 → ✅
  best_fix picks min-complexity Pareto-optimal: ✅

G2 (latency — search over seed corpus):
  10 candidates searched in 392.96µs → ✅

G3 (no-regression — seed corpus searchable):
  10/10 lints have a best_fix → ✅

=== All GOAT gates PASS ===
```

## See also

- [`../06_ruliology/ruliology_search.md`](../06_ruliology/ruliology_search.md) — the search API
- [`../06_ruliology/pareto_front.md`](../06_ruliology/pareto_front.md) — the algorithm

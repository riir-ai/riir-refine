# 06 — Ruliology (L3 Pareto-front candidate search)

> **Purpose:** when the L1 drafter proposes multiple candidates and L2
> accepts more than one, L3 enumerates the Pareto-front of
> (lint_satisfied, AST delta size) and picks the minimum-complexity fix
> among the Pareto-optimal set.

## Files

| File | Scope |
|---|---|
| [`ruliology_search.md`](ruliology_search.md) | `RuliologySearch` + `ScoredCandidate` + `best_fix` |
| [`pareto_front.md`](pareto_front.md) | The `pareto_front` algorithm + `dominates` predicate |
| [`ast_delta_size.md`](ast_delta_size.md) | AST node-counting via `syn::visit` |

## Source

- [`src/ruliology_search.rs`](../../src/ruliology_search.rs)

## Feature gate

`ruliology_search` (opt-in). Default-off because:

1. **Refinement layer**: only useful when multiple valid fixes exist
2. **AST traversal**: pulls `syn`'s `visit` feature (already in dep tree
   transitively, but explicitly enabled for the AST walker)
3. **Not hot-path**: runs once per lint invocation (~400µs), not per-tick

## See also

- [`../04_pruner/`](../04_pruner/README.md) — L2 provides the `parses` signal
- [`../09_lessons/borrow_without_dep.md`](../09_lessons/borrow_without_dep.md) —
  why the Pareto-front is reimplemented locally

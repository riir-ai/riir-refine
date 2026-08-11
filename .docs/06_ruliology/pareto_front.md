# Pareto-front algorithm

The `pareto_front` function filters a set of scored candidates to those
that are Pareto-optimal (not dominated by any other candidate).

## The dominance predicate

```rust
fn dominates(a: &ScoredCandidate, b: &ScoredCandidate) -> bool {
    // A dominates B if A is at least as good as B on all axes,
    // and strictly better on at least one axis.
    let a_payoff = a.lint_satisfied as u8;  // true=1, false=0
    let b_payoff = b.lint_satisfied as u8;

    (a_payoff >= b_payoff && a.ast_delta_size <= b.ast_delta_size)
        && (a_payoff > b_payoff || a.ast_delta_size < b.ast_delta_size)
}
```

## The algorithm

```rust
pub fn pareto_front(candidates: &[ScoredCandidate]) -> Vec<&ScoredCandidate> {
    candidates.iter()
        .filter(|&a| !candidates.iter().any(|b| dominates(b, a)))
        .collect()
}
```

For each candidate A, check if any other candidate B dominates A. If
none do, A is Pareto-optimal.

O(n²) — fine for small candidate sets (typically < 20). Not a hot-path
concern (runs once per lint invocation).

## Why reimplemented locally (not consumed from katgpt-ruliology)

The substrate's `pareto_front` lives in
`katgpt-rs/crates/katgpt-ruliology/src/types.rs::WinMatrix::pareto_front`
(GOAT PASS, Bench 572). But it's:

1. **Coupled to `WinMatrix`** (2-player tournament semantics) — clippy
   fixes have ground-truth payoff (`lint_satisfied: bool`), not pairwise
   tournament payoffs.
2. **Non-optionally pulls `katgpt-pruners`** (the `g_zero` feature) —
   too heavy for clippy's needs.
3. **~30 LOC to reimplement** — the core algorithm is trivial.

This mirrors the repo's existing pattern: `ClippyTernaryDrafter` reuses
the `TernaryDraftModel` pattern from `quest_grammar` without a code-level
dep. See [borrow_without_dep.md](../09_lessons/borrow_without_dep.md).

## Test coverage

- `pareto_front_empty` — empty input → empty output
- `pareto_front_single_candidate` — single candidate → returned
- `pareto_front_filters_dominated` — dominated candidate excluded
- `pareto_front_keeps_incomparable` — incomparable candidates both kept
- `pareto_front_all_satisfy_picks_min_complexity` — tie-break on complexity
- `pareto_front_mixed_satisfaction` — satisfied + unsatisfied mixed
- `dominates_same_payoff_lower_complexity` — tie-break dominance
- `dominates_higher_payoff_same_complexity` — payoff dominance
- `does_not_dominate_when_incomparable` — no false dominance
- `does_not_dominate_identical` — identical candidates don't dominate

## See also

- [`ruliology_search.md`](ruliology_search.md) — the search API
- [`ast_delta_size.md`](ast_delta_size.md) — the complexity metric

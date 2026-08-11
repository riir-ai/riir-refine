# RuliologySearch — Pareto-front candidate search

The L3 refinement layer. When the L1 drafter proposes multiple
candidates and L2 accepts more than one, L3 enumerates the Pareto-front
of `(lint_satisfied, ast_delta_size)` and picks the minimum-complexity
fix among the Pareto-optimal set.

## ScoredCandidate

```rust
pub struct ScoredCandidate<'a> {
    pub entry: &'a ClippyFixEntry,  // the corpus entry being scored
    pub lint_satisfied: bool,       // does L2 validate this fix?
    pub ast_delta_size: usize,      // AST node count difference (buggy → fixed)
    pub pareto_rank: usize,         // 0 = Pareto-optimal, higher = dominated
}
```

## The search

```rust
let search = RuliologySearch::new();
let scored: Vec<ScoredCandidate> = search.search(buggy_code, lint_name, &candidates);
let best: Option<&ClippyFixEntry> = search.best_fix(buggy_code, lint_name, &candidates);
```

1. **Score each candidate**: run L2 `validate()` → `lint_satisfied`;
   compute `ast_delta_size` via `syn::visit` AST traversal.
2. **Compute Pareto-front**: filter dominated candidates.
3. **Pick minimum-complexity**: among Pareto-optimal candidates with
   `lint_satisfied = true`, pick the one with the smallest
   `ast_delta_size`.

## Pareto dominance

A candidate A **dominates** candidate B if:

- A's `lint_satisfied` ≥ B's `lint_satisfied`, AND
- A's `ast_delta_size` ≤ B's `ast_delta_size`, AND
- at least one inequality is strict.

Pareto-optimal candidates are those not dominated by any other
candidate. See [pareto_front.md](pareto_front.md).

## API

```rust
pub struct RuliologySearch {
    pruner: RustAnalyzerPruner,
}

impl RuliologySearch {
    pub fn new() -> Self;
    pub fn with_pruner(pruner: RustAnalyzerPruner) -> Self;
    pub fn search<'a>(&self, buggy_code: &str, lint_name: &str,
                      candidates: &'a [ClippyFixEntry])
        -> Vec<ScoredCandidate<'a>>;
    pub fn best_fix<'a>(&self, buggy_code: &str, lint_name: &str,
                        candidates: &'a [ClippyFixEntry])
        -> Option<&'a ClippyFixEntry>;
    pub fn pareto_optimal<'a>(&self, scored: &[ScoredCandidate<'a>])
        -> Vec<&ScoredCandidate<'a>>;
}
```

## GOAT gate results

| Gate | Target | Measured | Status |
|---|---|---|---|
| G1 (correctness) | Pareto-front filters dominated; min-complexity picked | all 3 sub-checks pass | ✅ PASS |
| G2 (latency) | search over 10 candidates, < 10ms | **392.96µs** | ✅ PASS |
| G3 (no-regression) | seed corpus (10 lints) all searchable | 10/10 | ✅ PASS |
| G4 (alloc-free) | N/A — runs once per invocation | N/A | ✅ PASS (documented) |

## See also

- [`pareto_front.md`](pareto_front.md) — the algorithm
- [`ast_delta_size.md`](ast_delta_size.md) — the complexity metric

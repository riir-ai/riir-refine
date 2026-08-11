# Issue 005 — L3: Ruliology Pareto-front candidate search

**Status:** RESOLVED — L3 ruliology search shipped behind `ruliology_search` feature. GOAT G1-G4 ALL PASS.
**Type:** Feature (modelless refinement layer — the L3 slice of Proposal 034 §Phase 3)
**Scope:** new `src/ruliology_search.rs` behind feature `ruliology_search` + composition API

## Problem

The L1 drafter proposes ONE fix; L2 validates it. But for many clippy lints,
the corpus contains MULTIPLE valid fixes (e.g. `needless_return` can be fixed
by removing the `return` keyword OR by rewriting as a trailing expression OR
by wrapping in a block). Currently the drafter picks one (matvec-weighted)
and the pruner accepts-or-rejects. There is no mechanism to:

1. Enumerate the candidates that L2 accepts
2. Rank them by (lint_satisfied, AST delta size)
3. Pick the minimum-AST-delta fix as the default emission

This is exactly the ruliology Pareto-front pattern: when multiple strategies
satisfy the constraint, pick the one with minimum complexity. Proposal 034
§Phase 3 T3.1-T3.2 specifies this.

## Substrate-first audit (MANDATORY before implementing)

**Substrate inventory:**
- `katgpt-rs/crates/katgpt-ruliology/src/types.rs` ships:
  - `WinMatrix::pareto_front(&[f32]) -> Vec<(u64, f64, f32)>` — the algorithm
  - `RuliologyPruner { payoff_threshold, complexity_threshold }` with `.filter()`
  - `SimpleProgram` trait (FSM/CA/TM-specific)
- GOAT PASS (Bench 572, 2026-08-05): 97/97 release tests, F4 RuliologyPruner covered

**Domain mapping (substrate → clippy L3):**
| Substrate concept | Clippy L3 analog |
|---|---|
| payoff (tournament avg over WinMatrix) | lint_satisfied (ground-truth bool from L2/L3-verify, no tournament) |
| complexity (FSM state count, normalized [0,1]) | AST delta size (count of changed syn nodes between buggy + fixed) |
| strategies (FSM/CA/TM enumeration) | candidate fixes (from corpus, finite per-lint) |
| WinMatrix (2-player tournament) | NOT NEEDED — clippy fixes have ground-truth payoff, no pairwise competition |
| Pareto-front ALGORITHM | REUSE (pattern, reimplemented locally ~30 LOC) |

**Consume-vs-build verdict: REIMPLEMENT LOCALLY (~30 LOC).**

`katgpt-ruliology` non-optionally depends on `katgpt-pruners` (g_zero feature)
for `DeltaGatedConfig` — heavy dep for a ~30 LOC algorithm. riir-clippy
currently depends only on `katgpt-core` (for `TernaryWeights` + SIMD matvec);
adding `katgpt-ruliology` would pull `katgpt-pruners` + FSM/CA/TM enumeration
code that clippy doesn't need.

This mirrors the existing repo decision: riir-clippy reimplements the
`TernaryDraftModel` pattern from `quest_grammar` locally (~200 LOC) rather
than depending on `riir-games-quest`. The "pattern borrowing ≠ dependency"
rule (riir-ai AGENTS.md) applies — the Pareto-front algorithm is the
transferable pattern; the FSM/CA/TM types are not.

**Pattern source:** `katgpt-rs/crates/katgpt-ruliology/src/types.rs::WinMatrix::pareto_front`
(O(n²) dominance check, returns `(id, payoff, complexity)` for each
non-dominated strategy, sorted descending by payoff).

## Design

### Architecture

New module `src/ruliology_search.rs`, feature-gated `ruliology_search`:

```text
L1 drafter → proposes N candidates (from corpus)
                │
                ▼
L2 pruner → filters to M ≤ N that parse (always-on)
                │
                ▼
L3 ruliology search → ranks M by (lint_satisfied, AST delta size)
  ├── Pareto-front enumeration (reuse substrate algorithm)
  └── pick minimum-AST-delta among Pareto-optimal
                │
                ▼
        [ best fix ]
```

### API

```rust
#[cfg(feature = "ruliology_search")]
pub struct ScoredCandidate<'a> {
    pub entry: &'a crate::corpus::ClippyFixEntry,
    pub lint_satisfied: bool,   // from L2 (parses) or L3-verify (clippy_confirm)
    pub ast_delta_size: usize,  // changed syn nodes vs buggy_code
    pub pareto_rank: usize,     // 0 = Pareto-optimal
}

#[cfg(feature = "ruliology_search")]
pub struct RuliologySearch { ... }

#[cfg(feature = "ruliology_search")]
impl RuliologySearch {
    /// Enumerate corpus candidates for a lint, score each, return Pareto-front.
    pub fn search<'c>(
        &self,
        buggy_code: &str,
        lint_name: &str,
        candidates: &'c [crate::corpus::ClippyFixEntry],
    ) -> Vec<ScoredCandidate<'c>>;

    /// Pick the minimum-AST-delta fix among Pareto-optimal candidates.
    pub fn best_fix<'c>(...) -> Option<&'c crate::corpus::ClippyFixEntry>;
}
```

### AST delta size

The complexity metric. Count the number of changed syn AST nodes between
`buggy_code` and `fixed_code`. Uses `syn::parse_file` (already a dep).
Two strategies:

1. **Node count diff** (PRIMARY): parse both, walk both ASTs in lockstep,
   count nodes that differ. Simple + deterministic.
2. **Levenshtein on AST node sequence** (deferred): more accurate but
   heavier. Not needed for the POC.

For the POC, use strategy 1: count `total_nodes(fixed) - shared_nodes(buggy, fixed)`.
Lower = simpler fix = preferred.

### Pareto-front algorithm (reimplemented from substrate)

```rust
/// Returns indices into `candidates` that are Pareto-optimal.
///
/// A candidate dominates another if it has higher payoff (lint_satisfied)
/// AND lower complexity (AST delta size). O(n²) — fine for small per-lint
/// candidate counts (typically <10).
fn pareto_front(payoffs: &[bool], complexities: &[usize]) -> Vec<usize> {
    let n = payoffs.len();
    debug_assert_eq!(complexities.len(), n);
    let mut front = Vec::with_capacity(n);
    for i in 0..n {
        let dominated = (0..n).any(|j| {
            j != i
                && payoffs[j] >= payoffs[i]   // payoff ≥
                && complexities[j] < complexities[i]  // AND strictly lower complexity
                && (payoffs[j] > payoffs[i] || complexities[j] < complexities[i])
        });
        if !dominated {
            front.push(i);
        }
    }
    front
}
```

(Direct adaptation of `WinMatrix::pareto_front` in
`katgpt-ruliology/src/types.rs` lines 119-126, with bool payoff +
usize complexity in place of f64/f32.)

## Substrate

- `syn` — already a dep ✓ (used by L2 pruner)
- `blake3` — already a dep ✓ (used for AST canonicalization if needed)
- NO new dependencies

## GOAT gates

- **G1** (correctness): Pareto-front correctly filters dominated candidates.
  Two fixes with same lint_satisfied but different AST delta → smaller wins.
  One fix that satisfies + one that doesn't → satisfying one wins regardless
  of complexity.
- **G2** (perf): search over 10 candidates < 1ms (syn parse is the cost).
  Not a hot-path gate — runs once per lint invocation, not per-tick.
- **G3** (no-regression): feature off by default → existing tests pass
  unchanged.
- **G4** (alloc): steady-state alloc-free (reuse scratch buffers for AST
  walk). The Pareto-front Vec is the output — counts as "output allocation",
  not steady-state.

## Non-goals

- FSM/CA/TM enumeration — that's the substrate's domain, not clippy's
- Adaptive strategy mutation (substrate F5) — deferred indefinitely
- Tournament semantics — clippy fixes have ground-truth payoff, no 2-player game
- Cross-lint Pareto analysis — each lint is independent

## References

- [Proposal 034 §Phase 3](../riir-ai/.proposals/034_clippy_healing_drafter_pruner.md) — the design argument
- [Bench 572](../katgpt-rs/.benchmarks/572_katgpt_ruliology_goat.md) — substrate GOAT
- [`katgpt-ruliology/src/types.rs::WinMatrix::pareto_front`](../katgpt-rs/crates/katgpt-ruliology/src/types.rs) — pattern source

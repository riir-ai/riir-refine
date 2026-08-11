# Borrow the pattern, not the code-level dependency

riir-clippy borrows **architectural patterns** from sibling repos but
reimplements them locally (~30–200 LOC each) because it cannot depend on
those repos (boundary rule: developer tool ≠ game runtime ≠ training ≠
chain ≠ storage).

## The three borrowed patterns

### 1. Drafter + pruner + corpus (from `riir-games-quest/quest_grammar`)

**Pattern**: `TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner`

**Borrowed as**: `ClippyTernaryDrafter` + `ClippyCorpus` + `RustAnalyzerPruner`

**Reimplemented locally** (~200 LOC for the drafter, ~120 LOC for the
corpus). The pruner ships its own `validate()` API (returns
`ValidationResult`, not the trait's `bool`) because the two-level
(syntax + lint) validation model didn't fit the trait's shape.

**Why not import**: `riir-games-quest` is a game crate in riir-ai.
riir-clippy cannot depend on riir-ai (boundary rule). The only
code-level dep is `katgpt-core` (the ternary SIMD matvec substrate).

### 2. Modelless embedder (from `riir-rag`)

**Pattern**: `ModellessEmbedder` — BLAKE3 + deterministic projection +
bounded output.

**Borrowed as**: `embed` / `embed_into` / `embed_fix_into` in
`src/embedder.rs`.

**Reimplemented locally** (~30 LOC). Same algorithm (BLAKE3 hash →
u32 split → f32 normalization), different types (clippy lint tuples
instead of text chunks).

**Why not import**: `riir-rag` is in riir-ai. Boundary rule.

### 3. Pareto-front (from `katgpt-ruliology`)

**Pattern**: `WinMatrix::pareto_front` — filter dominated candidates.

**Borrowed as**: `pareto_front` + `dominates` in
`src/ruliology_search.rs`.

**Reimplemented locally** (~30 LOC). The substrate's version is coupled
to `WinMatrix` (2-player tournament semantics) + non-optionally pulls
`katgpt-pruners` (`g_zero` feature). Clippy fixes have ground-truth
payoff (`lint_satisfied: bool`), not pairwise tournament payoffs, so
`WinMatrix` doesn't fit.

**Why not import**: the coupling is wrong (tournament ≠ ground-truth),
not just the boundary rule.

## The rule

> A new project may borrow an **architectural pattern** from a sibling
> crate without needing a **code-level dependency** on that crate. If
> the pattern can be reimplemented in ~200 LOC and the new project has
> no other domain coupling, it belongs in a **new repo**, not as a new
> module in the sibling.

## Canonical precedent

**(2026-08-10):** the clippy-healing pipeline (Proposal 034) borrows the
`TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner` pattern from
`quest_grammar` but has zero game-domain coupling. It ships in
`/git/riir-clippy` (new repo), NOT in `crates/riir-games-quest/src/clippy_healing/`.

## See also

- [`../01_orientation/boundary_rationale.md`](../01_orientation/boundary_rationale.md) — why this is a separate repo
- [`../01_orientation/sibling_layout.md`](../01_orientation/sibling_layout.md) — the dependency graph

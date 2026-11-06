# Boundary rationale — why this is a separate repo

## The question

> Why isn't the clippy rulewright a module in riir-ai (e.g.
> `crates/riir-games-quest/src/clippy_healing/`)?

## The answer

The clippy rulewright borrows an **architectural pattern** from
`riir-games-quest/quest_grammar` (TernaryDraftModel + SealQuestCorpus +
ConstraintPruner) but has **zero game-domain coupling**. If the pattern
can be reimplemented in ~200 LOC and the project has no other game-domain
coupling, it belongs in a **new repo**, not as a new module in riir-ai.

## The evidence (zero game-domain coupling)

- No dependency on `riir-games*`, `riir-game-sdk`, or any game crate —
  from ANY feature combination
- No dependency on `riir-chain`; `riir-neuron-db` only transitive (via
  `riir-rag` under the opt-in `latent_retrieval` feature)
- Default build: only public dep is `katgpt-core` (ConstraintPruner
  trait + ternary SIMD matvec). Two opt-in feature-gated arms consume
  non-game sibling crates where reimplementation would duplicate whole
  substrates: `ternary_inference` → `riir-engine` + `riir-gpu` (Bench 010
  inference arm), `latent_retrieval` → `riir-rag` (Plan 005 retrieval
  substrate, the `cargo refine suggest` CLI)
- The `TernaryDraftModel` pattern is reimplemented locally (~200 LOC),
  not imported cross-workspace

## The prior precedent

**riir-ai/AGENTS.md §"The prior rejection (Proposal 017)"** records a
different case: Proposal 017 proposed extracting `riir-games` entirely
out of riir-ai into a separate repo. That was correctly **rejected**
because `riir-games` has heavy **bidirectional coupling** with
`riir-engine` (NPC cognition dispatches into engine runtimes; engine
exports game-shaped types).

The clippy rulewright is fundamentally different: it has **zero** coupling
with any game crate.

## Canonical precedent

**(2026-08-10):** the clippy-fix pipeline (Proposal 034) borrows the
`TernaryDraftModel` + `SealQuestCorpus` + `ConstraintPruner` pattern from
`quest_grammar` but has zero game-domain coupling. It ships in
`/git/riir-refine` (new repo), NOT in `crates/riir-games-quest/src/clippy_healing/`.
The prior draft of Proposal 034 that proposed
`crates/riir-games-quest/src/clippy_healing/` was a boundary violation —
corrected before implementation.

## When in doubt

Run the `boundary-guard` skill (`.agents/skills/boundary-guard/`). It
enforces these rules via grep checks.

## See also

- [`crate_role.md`](crate_role.md) — what belongs here vs. elsewhere
- [`../09_lessons/borrow_without_dep.md`](../09_lessons/borrow_without_dep.md) —
  the borrow-pattern-not-code-dep convention

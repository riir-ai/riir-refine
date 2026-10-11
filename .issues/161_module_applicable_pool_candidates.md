# Issue 161 — module-applicable pool candidates: the `heal` seat's pool substrate cannot support a rerank bench

**Status:** OPEN — 2026-10-11, filed from Plan 225 T1's BLOCKED verdict (Bench 113). Blocks: the heal-seat expert bench lane (Plan 225 T2–T5) until the pool carries candidates that can apply to real code.

## The finding (measured, Bench 113)

The harvest replayed the FULL deduped v2edit population (1500 real mined
modules) through `heal_with_pool` with production-shaped memory warming and
scored every pool candidate through the arm-H oracle:

- 2400 candidates, **98.3% fail at merge** — they are corpus-shaped snippets,
  not transformations of the target module;
- **4 gold (a∧c) candidates in the entire population**, 0 under the full
  oracle (behaviour clause);
- deciding cases **3** vs the T1.6 requirement **≥ 621** (p_d = 0.20) —
  BLOCKED ON CASE SUPPLY.

Root cause: `TernaryDrafter::draft_fix` is a RETRIEVER — it emits the corpus
entry's `fixed_code` verbatim (corpus-local fn names), so every pool member is
structurally alien to the module under repair. Memory warming cannot fix this:
on real modules nothing is ever kept, so trajectories stay corpus-shaped and
pools stay corpus-shaped forever.

## The fix direction (substrate that already exists)

Widen the pool's birth points (Plan 202 R6's `heal_with_pool`) with
**module-applicable candidates**:

1. **The per-lint AST proposers** (`domains::clippy_lints::proposers` —
   `collect_edits`/`FIXABLE_LINTS`): span-preserving edits computed FROM the
   target module; they power `--fix` and apply by construction.
2. **The oracle feed's MachineApplicable suggestions** (the P21 lane's
   verbatim-apply shape) where a feed is present.

Design questions to settle at implementation (NOT here): pool member order +
dedup, whether proposer candidates carry their own confidence axis, whether the
min_pool ≥ 2 gate counts them (it should — they are real candidates the seat
can pick), and whether Stage-2 hits still short-circuit. The drafter snippet
STAYS a pool member — the expert's job becomes choosing among
{proposer edit, drafter snippet, memory trajectory}, which is a real decision.

## The gate that re-opens the bench

After the widening, re-run `fixer_regate --arm HARVEST --source v2edit_all`
(the tool is landed, deterministic, ~11 min on the M3) and re-compute T1.6 on
the new census: the gate passes iff deciding ≥ N(p̂_d) with the SAME pinned
inputs (δ₀ = 0.05, α = 0.05, power 0.80 — never lowered post hoc). No mint, no
GPU, until it does.

## Laws that bind

- The rerank wire stays options-in/pick-or-abstain-out (decision_wire v1) —
  pool composition is transparent to the seat.
- A0 stays the floor, never demotable; every widened pool member must pass the
  same three-clause gold oracle the bench scores with.
- No new harness: the HARVEST arm IS the census instrument.

## References

- Bench 113 (the census + verdict + oracle hardening)
- Plan 225 (T1.6 + the amended revisit condition) · Bench 108 (the original
  revisit condition this supersedes)
- `src/drafter.rs` (the retriever) · `src/self_evolve.rs::heal_with_pool`
  (the pool birth points) · `src/bin/fixer_regate.rs` (`--arm HARVEST`)

# Research 191 — Batch 164: the coverage-report re-arm (the frontier-lead mining pack X)

**Status:** RECORD (batch landed 2026-09-19 — commit recorded in Plan 155)

## What this batch was

The recorded-spare queue ran EMPTY at Batch 163 (`6408ea44`) — the first time
since the miner's admission loop began. The owed unit was the **re-arm**: a
fresh probe round over clippy's live lint surface, derived from
`cargo heal --coverage-report` (Issue 119's instrument) instead of the
miner's own frontier table. This note records what the derivation yielded,
what landed, and what the batch measured along the way.

## The derivation → leads pipeline (the repeatable shape)

1. `cargo heal --coverage-report` — the clippy-driver `-W help` derivation
   diffed against the corpus: 738 uncovered / 425 default-firing / 118 with
   default-eco evidence (clippy 0.1.98, the Bench 094 three-source join).
2. Adjudicate the head rows with reasons (the skip list is the record:
   `missing_safety_doc` doc-authoring, `unnecessary_cast` typeck,
   `result_unit_err` error-type invention, `too_many_arguments`
   architectural, `needless_borrowed_reference` VERIFY-tier → retires in
   the FIXABLE-only miner loop, `len_zero`/`double_ended_iterator_last`/
   `clone_on_copy` type-blind).
3. `scripts/plan155_frontier_probe.py` — real-clippy group arms + guard
   matrix per lead, BEFORE any code.
4. Land corpus + proposers, then the full gate walk (freeze regen → GOAT →
   frontier e2e → score-bench).

Two leads landed: `match_like_matches_macro` (count 21) +
`excessive_precision` (count 56, rank 2 of the whole head) — both
style/default-on, the EIGHTH consecutive all-FIXABLE pack.

## The batch's finding — TWO freeze bad-atom joins, a first for one batch

**`excessive_precision` is structurally entangled with `unreadable_literal`.**
Every plain-decimal ep-firing literal carries >6 significant digits —
unreadable_literal's exact territory (threshold: int part ≥6 digits or frac
>3 digits). The corpus fixture `2.141592653589793238462643f32` co-fires
unreadable_literal on the same span. Probe-predicted, freeze-measured: ep
joined the bad-atom set on the first freeze e2e run.

**`match_like_matches_macro` is a BROAD rule — the Batch-155 `let_and_return`
class.** Its if-let-else arm fires on the EXISTING
`redundant_pattern_matching` corpus fixture
(`if let Ok(_) = r { true } else { false }` — exactly mlmm's other firing
form), which the first freeze e2e surfaced as a surprise 8th bad atom after
the first splice had labeled it clean. Construction proposes, the oracle
disposes; the bad-atom set was extended and the tables regenerated.

Bad sets 6→8 (v2) / 7→9 (v3, +persist_array_default as ever). Recorded cost
of admitting a broad rule pair: the ladder narrows (86 vs 90 v2 labels) and
two atoms lost their k=1 rungs — truthful, not fought.

## The frontier-fixture corollary (the same entanglement, one layer down)

The miner's HeldOut invariant ("own-view heal leaves the fixture
byte-untouched") REFUSES the plain-decimal ep shape: hold ep's rule out and
the in-corpus `unreadable_literal` proposer regroups the digits
(`2.5000000000000001f32` → `2.500_000_000_000_000_1f32`) — a different
byte string, gate red. The separable shape is the **exponent form**
(`2.5000000000000001e-7f32`): unreadable's own exponent guard refuses it,
the padding rounds away (the heal recovers `2.5e-7f32` byte-identically),
and upstream ep still fires. The miner's ep family fixtures are
exponent-form for exactly this reason — the same structural co-fire that
made the atom bad, solved at the fixture layer by notation.

The seed-clean gate adds a second measured constraint: the natural fixed
form `2.1415927f32` (8 digits) FIRES the pedantic unreadable_literal under
the full oracle (all+pedantic+nursery) — the seed must be a shortest form
with ≤6 significant digits (`2.5e-7f32`, clean).

## GOAT record

- lib default 2019/0 · latent 2173/0 · miner,score,latent 2337/0
- retrieval floors: byte-identical to the batch-163 baseline (worktree A/B
  at `6408ea44` — zero ranking decay from the 2 new corpus docs; the
  batch-163 message's blended figures were a different lane's numbers)
- clippy `-D` clean ×3 states
- freeze regen v2 **86/17** + v3 **86/21**; both e2es PASS 0 drift 0 newly
  excluded (v2 35.2s: solve 100.0% at every k; v3: 95.2/91.7/95.8/100.0%,
  stratification PASS)
- frontier e2e VERDICT ✓ (44 seeds clean, 21 family loops, 2 fresh
  admissions, 19 rows 0 drift, 2 appended → the store holds 21 solved
  rows, band fold 21/21)
- score-bench #140/#141 (double-run, identical) **96.36% FLAT
  bit-identical** (vocab 67; the 2 PRE-EXISTING toolchain drift warnings
  unchanged)
- 5 mechanical clippy findings healed (uninlined_format_args ×2,
  doc_markdown, needless_pass_by_ref_mut ×2) + 5 manual (the question_mark
  else-if chain, comment-guarded; needless_borrow ×4, not in the healer's
  corpus)

## Re-arm economics (the NEXT POOL state)

The coverage-report derivation produced 2 leads from a 425-row
default-firing head — but the head's remainder is mostly un-fixable classes
(doc authoring, typeck-dependent, architectural). The pipeline is
repeatable at the next empty-queue window; the derivation itself is the
re-arm (no separate spares queue needed). Next windows: arxiv walk #10
opens Mon 09-21; the nnethercote trigger stays checked per idle pass.

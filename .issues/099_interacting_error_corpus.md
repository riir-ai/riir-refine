# Issue 099 — An INTERACTING-error corpus for the fix-compile floor harness

**Status:** OPEN — filed 2026-09-12 from the convergence of three measurements that all bottomed out on the same missing instrument (Bench 087 §6, Bench 088, Issue 093 T4). Design here; implementation is one `corpus_gen` family + one flag. No new dep, no new store.

## Premise — three open threads, one missing corpus

Every corpus the `--fix-compile` driver has ever been measured on is built from
**independent** single-mutation cases:

- `MUTANT_CASES` + `generate_corpus()` — one mutation, one error, one repair.
- `compose_multi` / `compose_multi_strided` (Bench 087, Bench 088) — K
  single-mutation cases concatenated into ONE scaffold. Depth, and (strided)
  menu width — but the members **cannot affect each other**: they are separate
  functions over separate bindings.

Three independent findings stalled on exactly that property:

| thread | what it measured | why the corpus blocked it |
|---|---|---|
| Issue 91 T2 / Bench 087 | flip prevalence **0/230**; reverts **0/230** | a flip needs a fix to RE-BREAK something. Independent members cannot. §6 reopen trigger 1 is literally "the revert rate leaves ~0" |
| Issue 093 T4 / Bench 088 | ordering gain **null**, one regression | with 0 reverts there is no wasted budget for a better order to reclaim; the resolved rate cannot move |
| floor realism (Bench 041/045 lineage) | 126/212 repaired, uncontested 105/105 | real code has coupled borrows; the corpus's repair difficulty is bounded below reality |

The instrument all three want is the same: **a scaffold where repairing error A
changes the diagnostic set of error B.** That is the one shape under which the
driver's iteration budget, its candidate ordering, and its revert path all
become load-bearing at once.

## What "interacting" must mean (and what it must not)

Interaction = the SECOND check's diagnostics depend on the FIRST repair's
choice. Two admissible mechanisms, both expressible in the existing
`GeneratedCase { seed, broken }` shape:

1. **Shared binding, competing borrows.** One `Vec`, one function: a mutable
   alias pair (E0499 shape) *and* an immutable read held across the mutation
   (E0502 shape) over the SAME binding. Reordering to satisfy one moves the
   other's span — the classic case where `e0499-swap-second-decl` fixes the
   first error and *creates* the second.
2. **Repair-class collision.** A repair whose strategy is `clone-at-move`
   (E0505) applied to a value that a later statement borrows (E0597): the
   clone changes which value dies where, so the second error's candidate
   ladder changes rung.

Explicitly NOT interaction, and the trap to avoid: two errors in one function
that merely *share a file*. The existing composer already does that. The gate
below is designed to refuse a corpus that only looks coupled.

## Tasks

- [ ] **T1 — the admissibility gate, FIRST.** Before any measurement is
      reported off this corpus, prove the members actually interact: for each
      case, check that repairing member A changes member B's diagnostic set vs
      repairing B alone (a 2×2: fix-A-then-check-B vs check-B-cold). A case
      whose 2×2 is identical is NOT interacting and must be dropped from the
      family, loudly, with a count. **This gate is the deliverable** — an
      "interacting" corpus that turns out independent would silently restore
      the exact blind spot the issue exists to remove, and would do it while
      wearing the name of the fix. (Precedent: Bench 087's own `--multi` gave
      depth but width-1 menus — a lever that looked like the missing one and
      was not; Bench 088 §"a depth lever is not a width lever".)
- [ ] **T2 — `corpus_gen::interacting_pairs()`** behind the existing
      `rustc_errors` feature: the two mechanisms above, table-driven like the
      current families (explicit per-row values — the E0502 comment already
      records why derived indices panic the ground-truth test). Every case
      keeps a ground-truth `#[cfg(test)]` module, same as every other family,
      so the G1 semantics pass covers it.
- [ ] **T3 — re-run Bench 087 over it** (Issue 91 reopen trigger 1 + 2's
      cousin): flip prevalence, revert rate, depth histogram. A non-zero revert
      rate here does NOT reopen T2's verdict by itself — the verdict was
      scoped to the corpus it ran on; report the new number as its own row.
- [ ] **T4 — re-run Bench 088's ordering A/B over it** (Issue 093 T4's owed
      "corpus where the resolved rate can move"). Arms unchanged: static /
      flat / shape-conditional / nonergodic per-run / nonergodic shared. If the
      resolved rate still cannot move, the ordering question is ANSWERED
      negative on the hardest corpus available and 093 can close on evidence
      rather than on a missing instrument.
- [ ] **T5 — floor-harness row.** Add the family to `floor_case_list()` only
      AFTER T1 passes, and re-pin the count floors it moves (every count pin
      naming 212 is part of the blast radius — enumerate them, do not grep once
      and assume).

## Decision criteria (written before the work, per the GOAT discipline)

- T1 fails (cases do not interact) ⇒ the family is dropped, the issue closes
  NEGATIVE, and Bench 087/088's verdicts stand unqualified. That is a real
  outcome, not a failure.
- T1 passes and reverts stay 0 ⇒ the driver's monotone keep rule is stronger
  than suspected; record it as a second, independent confirmation of Bench
  087 §4's Lyapunov argument.
- T1 passes and reverts are non-zero ⇒ both Bench 087 and Bench 088 re-run,
  and the ordering claim finally has a surface with something to win.

## Non-goals

- NOT a new driver, ordering policy, or detector — this issue ships a CORPUS
  and the gate that proves it is what it claims.
- NOT organic data. The organic `.heal/fixseq/` surface (zero rings across 33
  repos, Bench 087 §0) stays the separate, better evidence; this is a
  synthetic instrument and every number off it is labelled synthetic.

# Issue 101 — the E0502 read-relocate template: F3's head-read reorder (the Bench-090 residual)

**Status:** OPEN — in flight (probes measured; implementing). The named lead from Bench
090 §Honest limits: "F3 remains unfixed (12/12): the residual state after the honest
reverts is the E0502-on-the-indexed-clone-line shape, whose repair is a head-read
REORDER no playbook ships."

## The measured premise (rustc 1.98.1 probe, 2026-09-13)

F3 cold (mid-read) E0502 carries exactly the anchors a relocate template needs:

```
error[E0502]: cannot borrow `*v` as immutable because it is also borrowed as mutable
  L2 |     let a = &mut *v;    |  ------- mutable borrow occurs here
  L3 |     let head = &v[0];   |          ^ immutable borrow occurs here   (PRIMARY)
  L5 |     a.push(4);          |  - mutable borrow later used here        (INSERTION ANCHOR)
```

**Label-pair scoping is free**: anchoring on `immutable borrow occurs here` +
`mutable borrow later used here` selects ONLY the immutable-later shape (the
mid-read). The head-first shapes (F1/F2/gen_e0502) carry `immutable borrow later
used here` instead → the template is silent there — the existing deref-removal /
clone-out paths are untouched.

## The design

**`e0502-read-relocate`** (borrowck template C) — ONE atomic candidate, three edits:
delete the `let head = &v[0];` statement (full line), re-insert `let head = v[0];`
(by value) after the conflicting borrow's LAST-USE statement, delete each `*` star
at the uses. Fires only when it heals:

- **Gap guard**: between the decl and the insertion point, the source must contain
  NO `&mut` and NO use of the relocated binding — on COLD F3 the gap contains b's
  `&mut` decl → silent; the E0499 swap clears the way FIRST, then relocate fires
  on the post-swap state and heals to the SEED TEXT EXACTLY
  (a-decl, a.push, head-by-value, b-decl, b.push — verified by statement-level
  walkthrough). Copy strata heal; the String stratum self-rejects (E0507 move-out
  via the oracle) so clone-out stays that family's correct repair.
- Stars must all lie after the insertion point (a use between decl and insertion
  would be orphaned → silence, not wrong); the insertion statement must not
  mention the binding (the tail-expr-as-last-use shape self-guards here).
- Single-line, comment-free decl + use statements (the E0499 template's guards).
- `semantic_note`: the read relocates past the borrow's last use — the value
  differs if that use mutated the indexed place (G1 verifies behavior on the
  corpus; the note names the class, the clone-out precedent).

**`max_iters` 5 → 8**: the F3 ladder path is a measured 6 attempts
(deref-removal REV, clone-out REV, swap KEEP, deref-removal REV, clone-out REV,
relocate KEEP → clean). The 5 ceiling predates revert-dynamics corpora and now
truncates a measured-clean path. Termination is unchanged — `max_iters` is the
safety bound; the tried-set + lex invariant bound the loop regardless. Deeper
exploration costs ~130ms/iter and only when candidates remain.

## Non-goals

- No ordering-policy changes; no new corpus family (F3 already ships); no
  E0614/MASKING_STAGE work (Issue 100 closed that).
- The template refuses (stays silent on) every shape it cannot heal locally —
  the family discipline: rustc drift degrades to misses, never wrong edits.

## Tasks

- [ ] T1 — probes (DONE — the label table above; F3-cold E0502 spans recorded).
- [ ] T2 — `E0502ReadRelocate` in `playbooks/borrowck.rs` + `family()`
      registration + unit tests (fires post-swap producing the 3-edit candidate
      that splices to the seed; silent on cold-F3/foreign-code/missing anchors/
      gap-`&mut`/gap-self-use/decl-after-use/stars-before-insert).
- [ ] T3 — `max_iters` 5 → 8 (both constructors) + doc citing the measured path.
- [ ] T4 — `FRONTIER_LANDINGS` row (`E0502`, `e0502-read-relocate`).
- [ ] T5 — measurements: lib tests; gate (46/46 must hold); T3 lane
      (`fixseq_prevalence --interacting --runs` — expect F3 12/12 clean at depth
      6, reverts 36→24); T4 A/B (expect every arm 46/46); floor re-pin
      (expect E0502 row 45/26/0 — F3 heals to seed text → MATCHED; driver
      62.0%→66.7%; G1 160→172); full-corpus integrity; clippy.
- [ ] T6 — Bench 091 + HISTORY + issue removed per the noise-reduction rule.

## Decision criteria (pre-written)

- The gate must still admit 46/46 (menu changes are fine; admissibility is not).
- F3 heals to the SEED TEXT (matched, not diverged) — if it heals diverged-only,
  record honestly and keep the template only if G1 passes.
- The loop invariant must still print HOLDS on every lane (deeper runs, same
  lex rule).

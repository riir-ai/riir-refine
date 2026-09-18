# Plan 155 — Batch 164: the frontier-lead mining pack X (the coverage-report re-arm)

**Status:** COMPLETE (2026-09-19 — Batch 164 landed at `f84037c2`, pushed; global binary rebuilt to the full release set — `--version` reports complete)

The recorded-spare queue ran EMPTY at Batch 163 (`6408ea44`) — the snapshot's
NEXT POOL line owes a fresh probe round over clippy's live lint surface, derived
from `cargo heal --coverage-report` (Issue 119). This plan is that round, executed
as Batch 164 under Protocol A (plan-first).

## Leads (the coverage-report head, 2026-09-19 derivation)

- `match_like_matches_macro` — style/warn, default-eco count **21**. Fix: the
  match → `matches!` rebuild (verbatim pattern + scrutinee).
- `excessive_precision` — style/warn, default-eco count **56** (rank 2 of the
  whole evidence head). Fix: float literal → shortest round-trip at the
  suffixed width.

Skipped head rows, with reasons (the adjudication record):
`missing_safety_doc` (186 — doc authoring, no bounded fix), `unnecessary_cast`
(48 — typeck-dependent, needless_borrow's exclusion class), `result_unit_err`
(28 — error-type invention), `too_many_arguments` (28 — architectural),
`needless_borrowed_reference` (27 — the `&&` twin of the type-blind-excluded
`needless_borrow`; VERIFY-tier at best → retires in the FIXABLE-only miner
loop), `len_zero`/`double_ended_iterator_last`/`clone_on_copy` (type-blind
over-fire risk), `len_without_is_empty` (17 — insertion-shaped, new fix class,
deferred as a future lead).

## Tasks

- [x] T1: probe both leads with real clippy (`scripts/plan155_frontier_probe.py`) —
  groups, suggestions, guard matrix.
- [x] T2: dedup grep (pre-plan: `matches!?|match_like` → 0 corpus hits;
  `excessive_precision|static_lifetime|shortest|round_trip` → only the existing
  redundant_static_lifetimes rule — both leads CLEAN).
- [x] T3: author 2 corpus entries (Batch 164 block) + 2 proposers +
  module fixture tests.
- [x] T4: FIXABLE_LINTS + dispatch + standing tables + count-pin walk.
- [x] T5: GOAT — batch gates + lib tests (default + latent + miner,score,latent)
  + retrieval floors + clippy ×3 states. (lib 2019/0 · latent 2173/0 ·
  miner,score,latent 2337/0; clippy clean ×3; retrieval A/B vs `6408ea44`
  worktree = byte-identical, zero decay.)
- [x] T6: freeze-table regen (corpus delta) + both freeze e2es 0 drift.
  (**TWO bad-atom joins** — ep→unreadable_literal structural co-fire +
  rpm→mlmm broad-rule, found by the first e2e after the first splice; bad
  sets 6→8 / 7→9; v2 86/17, v3 86/21; both e2es PASS 0 drift 0 newly
  excluded; temp example + Cargo.toml row + out file deleted.)
- [x] T7: frontier e2e VERDICT (2 fresh admissions) + score-bench FLAT +
  frontier snapshot append. (44 seeds clean, 21 loops, 2 admissions, store
  19→21, band 21/21; score-bench #140+#141 96.36% FLAT vocab 67; snapshot
  gen-8 appended. The ep family fixtures are EXPONENT-FORM — the
  unreadable-proposer regroup would touch any plain-decimal shape under
  the held-out view; measured, Research 191.)
- [x] T8: Research note 191 (RECORD) + AGENTS.md batch entry + corpus-count bump
  + `.docs/11_domains/` (no clippy_lints domain doc exists — the counts
  live in AGENTS.md, updated there) + snapshot update.
- [x] T9: commit + push + global binary rebuild (same-session rule). (`f84037c2`
  pushed to origin/develop; binary rebuilt in the `.wbin` worktree with
  latent_retrieval,kat_service,kat_transport,fix_verify — was 6 batches stale.)

## T1 probe verdict (2026-09-19, clippy 0.1.98)

`match_like_matches_macro`:
- Fires on: 2-arm bool-literal match (`{ P => true, _ => false }` and the
  negated twin), or-patterns in P, `@`-bindings in P, call scrutinees,
  nested/expr positions. Suggestion = `matches!(scrutinee, P)` /
  `!matches!(scrutinee, P)` — P and scrutinee VERBATIM.
- **Guard arm `mlmm-guard` FIRES upstream with `matches!(x, 0 if x > 1)` —
  INVALID RUST** (guard inside `matches!` pattern position is a compile
  error; the upstream known-problem class, string_lit_as_bytes precedent).
  Our proposer REFUSES guards — documented stricter-than-upstream divergence.
- `mlmm-three-arms` → upstream fires only `match_same_arms` (the 1/`_` arms
  collapse); mlmm stays SILENT on 3-arm matches — our 2-arm-only scope is
  exactly upstream's.
- `mlmm-named-catchall` (`other => false`) → SILENT upstream — refuse matches
  our scope (no divergence to record).
- Default-level confirmed (group arm fires with no -W flags).

`excessive_precision`:
- Fires on: over-precise float literals — suffixed (f32/f64) AND
  annotation-typed (`ep-nosuffix` FIRES upstream). Suggestion = the shortest
  round-tripping decimal at the target width, MachineApplicable-rendered
  (`2.141592653589793238462643f32` → `2.141_592_7_f32` — note upstream's
  suggestion carries SEPARATORS, matching unreadable_literal's style; our
  emit is separator-free on separator-free input, value-identical either way).
- `ep-ones` (`1.0000000f32`) → excessive_precision SILENT (only
  unreadable_literal fires) — trailing-ZERO-only forms are NOT the lint's
  target (they truncate to the same digits; `1.0` == `1.0000000` digit-wise).
  Our proposer inherits the refuse (mantissa must DIFFER after shortening).
- `ep-separator` → SILENT upstream (unreadable_literal's territory) — our
  separator refusal is upstream-exact, not a divergence.
- `ep-clean` (`0.1f32`) → SILENT (already shortest).
- `ep-exponent`: upstream emits `3e-10_f32` — exponent form survives; our
  shortest-repr emit must handle the exponent class (fire only when the
  shortening changes the mantissa digits; emit keeps `e` notation).
- `ep-tiny` (32.9 digits → `0.123_456_79_f32`): the f32-wide cut — our
  {:?}-round-trip search reproduces `0.12345679`.
- DIVERGENCE to record: `ep-nosuffix` (annotation-typed) fires upstream; our
  proposer REFUSES (suffix decides the width — type-blind scope), the
  manual_string_new `"".to_string()` precedent.
- Co-fire note: unreadable_literal rides every over-precise fixture (both
  lints fire on the same span; our oracle-anchored lane dedups by lint, the
  proposer lane only claims the excessive_precision family).
- Default-level confirmed (group arm fires with no -W flags).

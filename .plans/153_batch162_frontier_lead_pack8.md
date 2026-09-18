# Plan 153 — Batch 162: the frontier-lead mining pack VIII (bytes_count_to_len + bool_comparison)

**Status:** COMPLETE — Batch 162 landed at `b8605224` (2026-09-19; the full record: HISTORY.md 2026-09-19 + Research 189; the frontier population EMPTY, the next re-arm = the plan152 spares). — opened 2026-09-19 (riir-shader-t2 idle session, the
Batch-161 re-arm leads; leads probe-verified by `scripts/plan152_frontier_probe.py`
the same day: seed clean, broken fires EXACTLY its target under the full oracle).

- [x] T1 — Dedup grep BOTH vocabularies (`bytes|count`, `bool|comparison|== true`)
      against the clippy_lints corpus; record any adjacent rows (expected clean —
      the plan152 probe run recorded zero corpus hits).
- [x] T2 — Author the two rules (batch162 files beside the pack VII/VII previews):
      - `bytes_count_to_len` (complexity, default-on): `s.bytes().count()` →
        `s.len()` — std's own definition identity (`str::bytes` yields exactly
        the `as_bytes()` slice as `u8` items, so `bytes().count()` IS
        `as_bytes().len()`; `[u8]::iter().count()` is `len()` likewise).
        Bounded scope: method-call chains of EXACTLY `.bytes().count()` on a
        place expression; refuse `bytes_mut`, `chars`, any argument, any
        intermediate adapter.
      - `bool_comparison` (complexity, default-on): `x == true` → `x`,
        `x != true` → `!x`, plus the `false` twins (`x == false` → `!x`,
        `x != false` → `x`) — boolean algebra on a proven-`bool` operand.
        Bounded scope: binary comparison whose operand is (a) a function
        parameter annotated `bool`, or (b) a `let` bound annotated `bool`
        (the provenance ladder the Batch-158 scalar derivation uses, booleans
        instead of scalars); literals on the LHS refuse (mirror orders stay —
        the pack V clamp precedent: mirrored operand orders ARE the family).
- [x] T3 — Corpus entries (clippy_lints) via the deterministic Python `lit()`
      path — the Batch-154 PROCESS LESSON: never hand-type multi-line fixtures
      through an edit payload. Count pins: corpus 80 → 82, FIXABLE 61 → 63,
      atoms 61 → 63 (walked at gate time, not asserted here).
- [x] T4 — Standing gates extended: `batch162_pseudocode_parses` (syn parse)
      + presence arms + the parse-negative twin for each rule.
- [x] T5 — Freeze-table regeneration in the SAME batch (plan 137 T7.5): run
      the regen example (`--features score_bench` temp example) + the KEPT
      `scripts/plan150_splice_freeze_tables.py`; verify bad-atom sets — the
      broad-lint risk is real here: `bool_comparison`'s shape (comparison
      against a bool literal) may crossfire `needless_bool`-adjacent atoms in
      EXISTING fixtures; the v2/v3 freeze e2e must PASS 0 drift.
- [x] T6 — Frontier e2e: the two consumed rows ARCHIVED FIRST (the KnownSet
      family-done guard — the Batch-159 load-bearing lesson), then the e2e:
      2 fresh heldout admissions + rows appended + prior rows 0 drift +
      band fold 17/17; the next re-arm recorded from the spares
      (`manual_string_new`, `needless_late_init`).
- [x] T7 score-bench run (row #138 expected; FLAT is the expected verdict)
      + frontier snapshot dedup
      **DONE (2026-09-19): row #138 — heal_rate 0.96363634 FLAT vs the
      96.36% baseline, vocab 61 → 63 (the two new atoms); frontier snapshot
      appended (generation 8, the boundary recorded).**
- [x] T8 — GOAT: G1 parse gates 4/4, G2 lib tests (default + latent), G3
      no-regression (clippy -D clean write-set, 3 states), G4 alloc-free
      (the fixpoint path's existing law).
- [x] T9 — docs: AGENTS corpus counts (80 → 82), HISTORY record, queue
      snapshot update (consumed-lead rows flipped + next re-arm), commit+push.
- [x] T10 — `rm -rf` any scratch; final gate re-run.

## Design sources

- `.benchmarks/frontier_miner_rows.jsonl` rows `3b35e0599fb14afc` /
  `5b19cbe52cd96362` (construction provenance, `solving_rule: null`).
- `scripts/plan152_frontier_probe.py` — the probe record (seed clean /
  broken exact both).
- Plans 146–152 (the seven prior packs) + `scripts/plan150_splice_freeze_tables.py`.

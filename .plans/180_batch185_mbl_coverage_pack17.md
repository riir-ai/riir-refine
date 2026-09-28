# Plan 180 — Batch 185: coverage-report pack XVII, `multiple_bound_locations` (MBL solo)

**Status:** COMPLETE — landed `d95bb8d1d` on develop, pushed 2026-09-28 (4090). All tasks [x].

**Context.** Batch 184 (Plan 179) consumed the staged pack XVI pair. The
staged queue for pack XVII was `multiple_bound_locations` (staged fresh,
Plan 169) + `derivable_impls` (staged DEFER, Plans 163/169). This session's
fresh derivation + probe re-adjudicated both and the MTA candidate:

- **`multiple_bound_locations`** (clippy #11, style/warn): the pack's LEAD.
  Fires when a generic param carries the same bound inline (`<T: Default>`)
  AND in the where clause. Fix = synthesized deletion (upstream emits NO
  suggested replacement — both duplicate locations are primary spans).
- **`missing_transmute_annotations`** (13, default-eco): the strongest
  unadjudicated LEAD-2 candidate — **REJECTED on probe evidence** (see T1).
- **`derivable_impls`** (16): **DEFER maintained** — the fix is a synthesized
  two-item edit (`#[derive(Default)]` onto the struct + whole-impl-block
  deletion); today's probe re-recorded the suggestion shape (whole-impl
  primary span, `suggested_replacement: None`). No new evidence overturns
  the Plan-163 semantic-eq + multi-part attr-edit DEFER.
- Everything else in the fresh derivation's evidence head is standing
  skip-list: doc-authoring (missing_*_doc, Research 191), API-shape
  (result_unit_err/too_many_arguments/type_complexity), alias-gap
  (len_zero → comparison_to_empty; needless_borrows_for_generic_args →
  needless_borrow), recorded REJECTS (double_ended_iterator_last Plan 158;
  manual_div_ceil/int_plus_one Batch 168; useless_borrows_in_formatting
  Batch 171 measured-inert), and single-crate anecdotes
  (obfuscated_if_else 10/1, wave-4's not-staged class).
- **Pack composition: MBL solo** (the Batch-156 `io_other_error` solo
  precedent). A thin pack is what the evidence supports today — the
  alternative (delaying for a second lead) is not owed by any law.

## Probe-measured facts (plan180 probe, 2 runs + raw-cargo runs)

- **MBL fires on FN signatures only**: `pub fn f<T: Default>(t: T) where
  T: Default` fires with count=2 — BOTH duplicate locations are primary
  spans, `suggested_replacement: None`, no applicability. The multi-bound
  duplicate (`<T: Default + Clone>` + `where T: Default + Clone`) also fires
  (count=2).
- **MBL is SILENT on trait-impl and struct/impl definitions** — clippy's own
  surface is fn-only. The proposer is fn-only: upstream parity, not
  divergence.
- **Single-location forms silent both ways** (where-only, inline-only) —
  the `len(mbl) == 0` guard matrix.
- **Both fixed forms compile clean + silent** (drop the inline bound and
  keep where / drop the where arm and keep inline — the proposer may pick
  either; the FIXABLE-tier oracle (real clippy) is the judge).
- **MTA negative matrix (the REJECT record)**: naked `transmute(y)` silent;
  partial turbofish (`<_, u32>`, `<f32, _>`) silent; full turbofish silent;
  `transmute::<_, _>` is a compile ERROR (E0282) so the "missing both"
  shape cannot exist. The count-13 default-eco row fires on
  inference-across-items shapes — typeck territory beyond any syntax-only
  proposer.

## Tasks

- [x] T0 — fresh `--coverage-report` derivation + adjudication (822
      derived / 98 covered / 724 uncovered / 411 default-firing / 104
      evidenced; the head read against the standing skip list; MBL + MTA +
      DI the only live rows)
- [x] T1 — probe `scripts/plan180_pack17_probe.py` — **PASS** (first run
      FAILed honestly on 4 rows: the MTA firing shape was wrong BOTH ways —
      not naked calls, not partial turbofish; re-adjudicated to the REJECT
      record above; mbl_trait_impl/mbl_struct moved FIXTURE→SILENT,
      fn-only surface measured)
- [x] T2 — `multiple_bound_locations` proposer (`src/domains/clippy_lints/
      proposers/multiple_bound_locations.rs`): syn-scan fn items; the guard
      set is (a) a generic param with ≥1 inline bound, (b) a where-clause
      predicate naming the SAME param with a SUBSET-or-equal bound set —
      delete the where-clause arm (keep inline — the where block's shape
      survives removal of one arm; single-arm where clauses delete the
      whole clause) when the where set ⊆ inline set, else delete the
      duplicated inline bounds when inline ⊆ where (the docs' own
      "Use instead" consolidates into the where clause — match that
      direction first); fix is a synthesized deletion spanning the where
      predicate (or the inline bound segment); every other shape refuses
      (lifetimes, const params, associated-type bounds, where-for<> forms)
- [x] T3 — corpus entry (`buggy_code` = the MBL_PLAIN fixture shape,
      `fixed_code` = the docs' consolidated-into-where form) + module tests
      (fire/refuse matrix mirroring the probe; the lint name in the entry
      metadata stays the canon)
- [x] T4 — wiring: dispatch arm + `pub mod` decl + FIXABLE_LINTS (79→80) +
      `CLIPPY_LINTS` count pin (98→99) wherever the corpus count test pins it
- [x] T5 — the Batch-151 law: corpus delta → v2+v3 freeze tables regenerated
      in-batch (the plan179_freeze_paste.py mechanisation, re-labelled for
      this batch) + the e2es; bad-atom delta recorded
- [x] T6 — gates: default lib count-pin move (+proposer tests), score_bench
      lib suite (the Batch-183 gap closure, ~140s), floor_corpus_integrity,
      retrieval floors HELD, clippy `-D` clean default+all-features
- [x] T7 — score-bench appended (gen-1 row; heal-rate floor HELD) + frontier
      snapshot + gate-calibration rows as owed
- [x] T8 — docs: AGENTS.md clippy_lints row (99 corpus / 80 fixable) + the
      pack XVII narrative line; `.docs/11_domains/` if the folder's index
      carries a count
- [x] T9 — commit `feat: batch 185: ...` + push (develop) + snapshot update +
      global binary rebuild (the release feature set; verify `--version`
      stamp complete)

## Notes

- The MTA REJECT joins the standing skip list: any future derivation that
  re-surfaces `missing_transmute_annotations` reads this plan's probe
  record FIRST (the "the probe is the record, do not re-derive from
  memory" law, Batch 167's dei_last precedent).
- pack_depth_rescue_guard: if it fires a third time during T6, the systemic
  remedy (marked-rule pre-retrieval promotion or corpus-size-scaled pool)
  is priced in THIS batch, not a fourth bump (the Plan-179 recorded
  decision).

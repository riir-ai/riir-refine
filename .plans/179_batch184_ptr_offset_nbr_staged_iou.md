# Plan 179 — Batch 184: the coverage-report pack XVI (the Plan-169 staged IOU pair: ptr_offset_with_cast + needless_borrowed_reference)

**Status:** COMPLETE 2026-09-28 ~09:0x UTC (shikuwa/4090, Windows; box quiet — the sibling riir-infer lane in its measurement phase, no cargo contention). The standing pipeline (Batch 164+): fresh `--coverage-report` derivation → adjudicate the evidenced head against the standing verdicts → probe-verify the leads → land. T0/T1 probe-first; the probe's two honest re-adjudications are recorded in T1.

## T0 adjudication

The derivation (clippy 0.1.98, 822 lints / 96 covered / 726 uncovered /
413 default-firing, 106 default-firing with evidence). The head, against
the standing verdicts:

| candidate | count | standing verdict | this batch |
|---|---|---|---|
| **ptr_offset_with_cast** | 16, complexity/warn | STAGED — overturn-ready (Plan 169: "clippy's own MachineApplicable two-part splice measured … Lands in a later pack") | **LEAD-1** — the IOU lands |
| **needless_borrowed_reference** | 27, complexity/warn | STAGED (Plan 169: the plan158 RETIRE overturned on plan160's measured co-fire + the probe's two firing shapes) | **LEAD-2** |
| derivable_impls | 16 | STAGED DEFER (semantic-eq + multi-part attr edit; no new evidence) | stays staged |
| multiple_bound_locations | 11 | STAGED fresh (no machine suggestion; synthesized deletion) | stays staged |
| missing_safety_doc / missing_errors_doc / missing_panics_doc | 186/90/78 | REFUSED (the Research-191 doc-authoring class, four adjudications) | stays refused |
| double_ended_iterator_last | 17 | REFUSED (type-blind — the Plan-169 standing set) | stays refused |

## T1 probe — `scripts/plan179_pack16_probe.py` — PASS after two honest re-adjudications

The plan163 envelope (one clippy run per fixture, exact attribution,
utf-8 writes, ASCII output) PLUS two new arms the pack needed:

- **Applicability dump** — every suggestion span prints its
  `suggestion_applicability` (both leads' upstream suggestions are
  MachineApplicable);
- **FIXED-FORMS section** — every hand-written fixed form is COMPILED
  and must produce zero error diagnostics AND stay silent on its lint.
  The batch-183 finding (the pipeline never compiles `fixed_code`) is
  therefore closed AT PROBE TIME for both new atoms.

First run FAILed honestly on two rows (the probe-first law's designed
outcome), both re-adjudicated on the measurement:

1. **`powc_neg` SILENT** — `p.offset(-(i as isize))` does NOT fire: the
   lint matches the `ExprCast` node and unary-minus wraps it. The
   negative arm (and its `sub` twin) is OUT — upstream parity, not a
   guard. The `powc_sub` row stays in FIXED_FORMS as a compile check
   only.
2. **`powc_u64` SILENT** — `i: u64` cast to isize does not fire either:
   upstream's own firing precondition is a **usize** operand. Our
   usize-only guard is therefore upstream PARITY, not the
   narrower-than-clippy divergence it was staged as.

Measured and held:

- **`powc_custom_offset` SILENT** — upstream TYPECKS the receiver (a
  custom `offset` method never fires), so the pointer-provenance guard
  is parity where syntax can see and the documented divergence only
  where it cannot (an unannotated helper-call pointer refuses).
- **`powc_param_ptr` FIRES + co-fires `not_unsafe_ptr_arg_deref`** —
  the param arm stays supported in the proposer; the corpus atom is the
  `as_ptr`-local shape (zero co-fires, measured).
- **`nbr_let` FIRES**, one deletion span, MachineApplicable.
- **`nbr_match` FIRES + CO-FIRES `match_ref_pats`** (the corpus.rs
  evidence reproduced) — the match shape is refused by construction
  (LET-bindings only; the landed rule owns it).
- **`nbr_let_mut`**: the guessed `let &ref mut y = &mut b;` shape does
  not even compile (E0308 — a `&` pattern without `mut` cannot match a
  `&mut` scrutinee) and is SILENT — v1 stays non-mut.
- **`powc_isize_cast`**: the identity-cast spelling is silent on this
  lint and belongs to `unnecessary_cast` (which fires) — no gap.
- Fixed forms: `powc_add/sub/wrapping_add` + `nbr_let` all compile
  clean and silent.

## Tasks

- [x] T0 fresh `--coverage-report` derivation + adjudication (the table
      above)
- [x] T1 probe `scripts/plan179_pack16_probe.py` — PASS (first run
      FAILed on powc_neg + powc_u64; both moved to the standing-silence
      set with their measured rows recorded)
- [x] T2 `ptr_offset_with_cast` proposer
      (`proposers/ptr_offset_with_cast.rs`): whole-method-call splice
      (both receiver and operand are proven bare idents, so formatting
      is preserved everywhere else); provenance = fn-head `*const/_ *mut`
      param (the scalar-param law's pointer twin, `body_rebinds` guard)
      OR `as_ptr`/`as_mut_ptr`-initialized local (PatIdent count == 1 in
      the whole fn, never-assigned — assignment and shadowing revoke);
      operand = `usize` param; `drops_interior_comment` over the splice;
      17 module tests (incl. fixpoint, comment, paren/field/literal
      refusals, rebound/assigned/shadowed revocations)
- [x] T3 `needless_borrowed_reference` proposer
      (`proposers/needless_borrowed_reference.rs`): the one-`&`-deletion
      on `let &ref IDENT` locals only; `LocalInit.diverge` guard (the
      let-else spelling would birth an IRREFUTABLE `let ref y = .. else`
      — broken code, found empirically when the test fired and fixed by
      the guard); mut spellings, subpats, tuples refuse; 12 module tests
- [x] T4 wiring: dispatch arms + `pub mod` decls + FIXABLE_LINTS (77→79)
      + the module-doc table rows + LINT_GROUPS rows (both complexity,
      probe/coverage-confirmed) + corpus entries 96→98 (the
      powc `as_ptr`-local atom and the nbr auto-deref-usage atom; both
      zero-co-fire, both fixed forms probe-compiled) + corpus_size
      pin 96→98
- [x] T5 the corpus-delta obligations (the Batch-151 law): v2 + v3
      freeze-table regen in-batch — the e2e runs printed the emitted
      records and the paste is mechanised in
      `scripts/plan179_freeze_paste.py` (96 v2 labels / 21 excl;
      96 v3 labels / 25 incl 4 persist excl; both new atoms verified
      CLEAN, 0 newly excluded; v2 M4 PASS 100% every k; v3 95.8/91.7/
      95.8/100 — reported, not asserted, per the v3 convention)
- [x] T6 the pool_depth_rescue_guard re-fire (the designed prompt, the
      SECOND fire): the needless_return casualty first-found at depth
      29-30. Verified RED AT HEAD in a detached worktree
      (`E:/git/riir-clippy.wprobe`, HEAD `2fba14678`, isolated target
      dir — first-found 30 there vs 29 here) ⇒ PRE-EXISTING red, not
      caused by this batch's entries; Batch 183's gate set did not
      include the score_bench-feature lib suite. Remedy per the P11/
      Batch-162 ladder: RERANK_POOL 28 → 33 (measured minimum 30,
      slack 3) + HEADROOM_DEPTHS rungs 33 and 40 appended
      (PROD_DEPTH_IDX 5 → 7) so the recoverable band keeps a rung above
      production. score_bench lib suite green after: 2721/0.
- [x] T7 gates: default lib 2370/0 (+30 module tests) · score_bench lib
      2721/0 · `floor_corpus_integrity` 3/0 · retrieval
      `retrieval_precision_regression_gate` PASS (floors HELD, no
      re-pin) · clippy lib clean at default AND all-features for this
      crate (2 doc_markdown + unused-self + constructor-name mechanicals
      fixed in-authoring) · score-bench appended (run #172, gen-1
      heal 96.36% FLAT vs the standing 96.4%)
- [x] T8 docs: AGENTS.md clippy_lints row (98 corpus / 79 fix + the
      pack XVI narrative) + `.distill/001` snapshot row + this plan;
      `.plans/.highwater` 177 → **179** (REPAIRS the Batch-183 missed
      bump — the file landed at 178 with the counter left at 177, the
      B174 `a9d1bbe1` class; the jump consumes both numbers)
- [x] T9 commit + push (develop) + global binary rebuild (the release
      feature set) so adoption serves the widened corpus

## Standing notes

- The freeze-paste script is generic (v2|v3) and idempotent-by-regen;
  the next corpus batch runs its e2es and pastes the same way.
- The guard re-fire pattern is now measured at two fires: raw rank 9
  (pre-P11) → 25 (B162) → 29-30 (B184). The casualty slides ~4-5
  positions per corpus pair-batch. A third fire should price the
  systemic remedy (marked-rule pre-retrieval promotion or a
  corpus-size-scaled pool) instead of a third bump.
- Batch 183's gate gap (score_bench-feature lib suite not in its gate
  set) is now closed by T7 making it part of this batch's gate record —
  the suite runs in ~140 s and carries the freeze-status hole checkers.

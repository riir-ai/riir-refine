# Plan 169 — Batch 175: the coverage-report pack XV (len_without_is_empty + cast_abs_to_unsigned)

**Status:** PRE-SLOTTED — T0/T1 probe-verified read-only 2026-09-24 ~01:2x +07 (fresh session, post-B174 close-out); corpus authoring + landing at the NEXT cadence slot (~19:5x 09-24 or ~00:0x 09-25 +07; B174 landed 09-24 00:43).

The standing pipeline (Batch 164+): fresh `--coverage-report` derivation →
adjudicate the evidenced head with reasons → probe-verify the leads → land.
The derivation (clippy 0.1.98, 822 lints / 94 covered / 728 uncovered /
415 default-firing, 108 with evidence; both stores STALLED per
`--gate-calibration` — the prescribed route is exactly this re-arm).

## T0 adjudication (the full table, with the OVERTURNS explicit)

Four of the six candidates are rows Plan 163 refused or retired. Overturning
a standing adjudication needs NEW evidence, not a re-argument — each
overturn below names its evidence; anything without new evidence keeps its
Plan-163 verdict:

| candidate | count | Plan-163 verdict | this batch's verdict | the evidence |
|---|---|---|---|---|
| **len_without_is_empty** | 17, style/warn | REFUSE (fn-body invention, the 191 class) | **LEAD-1 — OVERTURNED** | the Batch-168 `new_without_default` landing: insertion-of-trivial-delegation is a landed, gate-proven shape (`impl Default { fn default() -> Self { Self::new() } }` is exactly as inventive as `pub fn is_empty(&self) -> bool { self.len() == 0 }`); NWD's recursive token descent + insertion machinery is substrate |
| **cast_abs_to_unsigned** | 12, suspicious/warn | REFUSE (typeck) | **LEAD-2 — OVERTURNED** | the `scalar_param_heads_over` substrate (Plan 149): when the receiver is a plain ident param, its signed int type is SYNTAX-VISIBLE in the fn head; the cast target is spelled IN the cast — both fix regimes decidable without typeck (the into_iter_on_ref / from_str_radix_10 provenance law) |
| ptr_offset_with_cast | 16, complexity/warn | REFUSE (unsafe pointer arithmetic) | STAGED — overturn-ready | clippy's own MachineApplicable two-part splice measured (`.offset`→`.add` + delete ` as isize`); the unsafe block content preserved verbatim — only method + cast change; compile-gated. Lands in a later pack |
| needless_borrowed_reference | 27, complexity/warn | RETIRE (probe-silent, plan158) | STAGED — **the RETIRE is overturned** | plan160 measured the mrf binding shape CO-FIRING it (corpus.rs:935-939 records it); THIS probe measured 2 independent firing shapes (`&Some(ref x)` match, `let &ref y` binding) with deletion suggestions — the plan158 silence was a shape miss, not inertness |
| derivable_impls | 16, complexity/warn | DEFER (semantic-eq + multi-part attr edit) | STAGED (DEFER maintained) | no new evidence; the probe measured the block-rewrite suggestion shape for the record |
| multiple_bound_locations | 11, style/warn | (not in the 163 table — under its count cut) | STAGED (fresh) | probe-verified firing; deletion fix, no span suggestion (synthesized) |

Prior adjudications that STAND (re-confirmed this cycle, no new evidence):
`missing_*_doc` (doc-authoring), `expect_used`/`absolute_paths` (restriction),
`result_unit_err`/`too_many_arguments` (architectural), `len_zero`
(DUPLICATE — comparison_to_empty Form 2), `manual_div_ceil` + `int_plus_one`
(overflow divergence at the extremes), `useless_borrows_in_formatting`
(MEASURED INERT in 0.1.98 incl. its own doc example),
`needless_borrows_for_generic_args` + `double_ended_iterator_last` +
`get_first` (type-blind).

## T1 probe — `scripts/plan169_pack15_probe.py` — VERDICT: PASS (23/23)

One clippy run per fixture (exact attribution by construction), utf-8
writes, ASCII output — the plan163 envelope. All six lints confirmed
present + warn-by-default in 0.1.98 (`-W help`, dash-spelled there).

**len_without_is_empty** — FIRES: inherent-impl `pub fn len` (no co-fires);
TRAIT `fn len` (no co-fires). SILENT: with-is_empty sibling; non-pub len
(the description's "public" term is real). NO machine suggestion span
(clippy ships a help note only) → the proposer SYNTHESIZES the insertion
(the NWD precedent). Guard matrix for the proposer: refuse when is_empty
exists; refuse non-pub len; **v1 refuses TRAITS** — inserting a required
method into a trait breaks every downstream impl (the surface-break class,
semantic not measured; the documented narrower-than-clippy divergence).
Insertion point: inside the existing impl block, before its closing brace
(brace-matched — the needless_late_init syn-respan lesson).

**cast_abs_to_unsigned** — FIRES: same-width (i64→u64), let-bound (i32→u32),
narrowing (i64→u8), widening (i32→u64 — span-dump follow-up). Suggestion
behavior measured THREE-REGIME (span columns dumped for the plan):
- same-width iN→uN: span = WHOLE `x.abs() as uN`, replacement
  `x.unsigned_abs()` — the cast is dropped;
- narrowing or widening (M ≠ N): span = `x.abs()` ONLY, replacement
  `x.unsigned_abs()` — **the cast SURVIVES** (truncation/widening
  preserved; all regimes compile).
The regime rule: whole-expr replacement iff `unsigned_abs()`'s return
width == the cast target width. Divergence note (recorded, adopted):
`iN::MIN.abs()` panics in debug before the cast; `unsigned_abs()` returns
2^(N-1); release-mode values IDENTICAL for every input — the fix is
release-identical + debug-strictly-better, which is the LINT'S OWN
motivation ("can panic" in its description). Receiver provenance guard:
plain ident param with declared primitive signed int type in the fn head
(the Plan-149 law); everything else refuses.

**ptr_offset_with_cast** — FIRES on `p.offset(i as isize)` with i: usize.
Two-part machine suggestion: `offset`→`add` + delete ` as isize`. SILENT:
`p.add(i)`; already-isize operands. (Staged.)

**derivable_impls** — FIRES on derive-equal manual Default; SILENT on
non-zero field inits (the "equal to a derive" term is strict). Suggestion =
block rewrite (impl header → `#[derive(Default)]` + body deletion).
(Staged.)

**multiple_bound_locations** — FIRES on T: Default inline + `where
T: Default`; SILENT single-location both spellings. No span suggestion —
synthesized deletion. (Staged.)

**needless_borrowed_reference** — FIRES on `match r { &Some(ref x) => ... }`
(CO-FIRES match_ref_pats — the corpus.rs:935-939 evidence reproduced) AND on
`let &ref y = &a;` (**NO co-fires — the clean corpus atom**). Suggestions =
deletions: two spans in the match shape (`&` + `ref`), one span in the let
shape. SILENT: ref-bind on owned scrutinee; match-ergonomics form.
(Staged — now probe-verified; the plan158 RETIRE stands overturned.)

## Tasks

- [x] T0 fresh `--coverage-report` derivation + adjudication (the table
      above, the four overturns with evidence)
- [x] T1 probe `scripts/plan169_pack15_probe.py` — PASS 23/23 (first run
      FAILed honestly: `nbr_let` silent-guess wrong → moved to FIXTURES
      with its measured count; `cau_unsigned_in` retired — `u32` has no
      `.abs()`, E0599, the shape cannot exist; replaced by `cau_narrow`)
- [ ] T2 `len_without_is_empty` proposer (inherent-impl insertion of
      `pub fn is_empty(&self) -> bool { self.len() == 0 }` before the impl
      closing brace; guards: existing is_empty, non-pub len, traits
      refused v1) + module tests
- [ ] T3 `cast_abs_to_unsigned` proposer (three-regime swap keyed on
      width equality via fn-head param provenance + the spelled cast
      target; guards: non-plain-ident receivers, non-primitive/unsigned
      receiver types refuse) + module tests
- [ ] T4 registration: doc-table rows + pub mods + FIXABLE_LINTS 75→77
      (the mod.rs:1273 pin) + dispatch arms + `all_lints_fire_and_parse`
      rows
- [ ] T5 corpus.rs entries ×2 + count pin 94→96 (corpus.rs:1101); the
      LWIE atom = the plain impl shape; the CAU atom = cau_i64 (both
      fire ALONE — zero co-fires measured)
- [ ] T6 G1: clippy_lints module + corpus tests green
- [ ] T7 freeze regen gen2/gen3 — 2-run convergence (the corpus-touching
      law: new entries move the stride windows; labels/exclusions
      regenerate as data in the SAME batch)
- [ ] T8 G2: lib default + kernel_heal floors green
- [ ] T9 retrieval gate floors held
- [ ] T10 G3: clippy -D clean at both feature postures (healer first for
      mechanical findings)
- [ ] T11 score-bench row + frontier snapshot + the Issue-102 cadence row
- [ ] T12 `.docs/11_domains/clippy_lints.md` + AGENTS.md counts (94→96)
      + `.distill/001` BATCH 175 LANDED row
- [ ] T13 commit `feat: batch 175: ...` + push at the cadence slot
- [ ] T14 global binary rebuild (release set) + `--corpus-stats` verify
      clippy_lints 96

## Standing notes

- The plan158→163 NBR arc is the standing lesson RECORDED POSITIVELY: a
  probe silence is a shape miss until the shape space is walked — the
  plan160 co-fire + this probe's two shapes are the counter-evidence class.
  When a future probe reports silence, cite this row before retiring.
- The four overturns do NOT weaken the probe-first law — they exercise it:
  every overturned row names NEW evidence (a landed precedent, a shipped
  substrate, fresh measurements), and the two rows without new evidence
  (derivable_impls DEFER, the standing REFUSE set) kept their verdicts.
- `.plans/.highwater` repair rides this unit (167→168→169): the B174
  session landed 168 without the bump — the 7fc14393 precedent, folder
  re-scanned, 168 is the max, no reuse risk.

# Plan 230 — Batch 228: the coverage-report re-arm pack XIX (clippy corpus)

**Status:** LANDED 2026-10-13 — Batch 228, all gates green (Research 259)

The recorded-spare queue ran empty at Batch 227 (the pool consumed: intake
done=28/open=0, walk #14 done, nothing search-unblockable). The owed unit is
the **coverage-report re-arm** (the Research-191 repeatable shape): a fresh
`cargo refine --coverage-report` derivation, adjudicated against the standing
skip ledger, converted into a clippy_lints corpus pack.

## T0 — the derivation + adjudication (DONE before the claim)

Fresh derivation (2026-10-13, clippy 0.1.98 48a229ceae, the installed driver):

- 822 lints / 9 groups; corpus 100 clippy_lints rules; compiled fix set 97
- UNCOVERED 722 (409 fire by default); evidence-backed uncovered 136
  (top-20 + 116 more); multi-crate (spread ≥ 2) uncovered evidence rows: 49
- Dual-evidence head: missing_errors_doc (90 local+study),
  missing_panics_doc (78 local+study)
- Since the 09-24 derivation (plan 169: 94 covered / 728 uncovered): corpus
  +6 covered, uncovered −6 — same toolchain, same t04 evidence dataset; the
  moved rows are the landed batches' own lints

Adjudication of the 49 multi-crate uncovered rows (the standing ledger,
every verdict with its reason — re-confirmed against plans 150/155/156/163/
165/169/179/180/184/185/188 + Research 108/191):

| rows | verdict |
|---|---|
| missing_safety_doc (186) | STAND skip — doc-authoring (Research 191) |
| missing_errors_doc / missing_panics_doc | STAND skip — same missing_*_doc class |
| expect_used (34) / absolute_paths (30, spread-1) | STAND skip — restriction |
| result_unit_err (28) / too_many_arguments (28) | STAND skip — architectural |
| len_zero (20) | STAND — DUPLICATE (comparison_to_empty Form 2) |
| needless_borrows_for_generic_args (20) / double_ended_iterator_last (17) / get_first (6) | STAND — type-blind / receiver-collision |
| derivable_impls (16) | STANDING DEFER — semantic-eq + multi-part attr edit, no new evidence (plans 163/169/179/188) |
| type_complexity (15) | STAND skip — architectural (plans 163/165: alias invention, signature rippling) |
| manual_div_ceil (13) / int_plus_one (10) | STAND REJECT — overflow divergence at the extremes (B168) |
| missing_transmute_annotations (13) | STAND REJECT — probe-REJECTED, negative matrix permanent (B185) |
| useless_borrows_in_formatting (12) | STAND — MEASURED INERT in 0.1.98 incl. its own doc example (B171) |
| obfuscated_if_else (10) / upper_case_acronyms (8) / transmute_ptr_to_ref (8, spread-1 confirmed this cycle from the t04 histogram) | STAND — wave-4 single-crate anecdotes, recorded not staged |
| to_string_trait_impl (8/2) | REFUSE — body-reauthoring class (impl Display requires authoring a fmt body from a String-returning body — the fn-body-invention REFUSE class, Research 191); recorded here first |
| multiple_crate_versions (6/3) | REFUSE — cargo-group (Cargo.toml dep dedup), not a code fix |
| test_attr_in_doctest (5/3) | REFUSE — markdown bodies, unparseable by the L2 syn pruner |
| large_enum_variant (4/4) | REFUSE — architectural (box-the-variant redesign) |
| only_used_in_recursion (4/3) | REFUSE — signature rippling (param removal crosses call sites) |
| should_implement_trait (2/2) / enum_variant_names (3/3) | REFUSE — call-site/type rippling renames |
| cast_sign_loss (3/2) | REFUSE — the cast-family typeck class (unnecessary_cast precedent) |
| needless_maybe_sized (3/2) | REFUSE — bound-resolution semantics (removing ?Sized changes the bound set), typeck-adjacent |
| unwrap_or_default (4/3) | STAND — receiver-collision refuse (B188) |
| unbuffered_bytes (5/2) | REFUSE — semantic buffering behavior (read vs read_exact is an I/O-contract choice, not a mechanical rewrite) |
| non_canonical_clone_impl (4/4) | REFUSE — derive applicability needs field-type resolution (typeck) |
| field_reassign_with_default (5/2) | STAND — plan-165 adjudicated (repo-posture mode era) |
| nonminimal_bool (7/3) / borrow_deref_ref (3/2) / needless_question_mark (4/4) / explicit_auto_deref (3/3) / collapsible_match (2/2) / redundant_guards (2/2) / explicit_counter_loop (2/2) | DEFER — bounded only under clippy's own firing on arbitrary expr shapes; sub-thin (≤7) with a structural risk each (boolean restructure / borrow-of-deref typeck / question-mark body variants / deref-coercion sites / pattern nesting / guard-expr patterns / counter-state); recorded, revisit on new evidence |
| elidable_lifetime_names (3/2) / empty_docs (3/2) / needless_maybe_sized | see above rows; thin-tail recorded |
| iter_cloned_collect (2/2) / manual_next_back (2/2) / manual_ok_err (2/2) / implicit_saturating_sub (3/3) / single_component_path_imports (5/5) / manual_repeat_n (5/4) | **THE LEADS below** |
| extra_unused_lifetimes (7/4) | DEFER — lifetime elision-compat needs bound-graph reads (typeck-adjacent); revisit |
| from_over_into (4/3) / inherent_to_string (4/2) / manual_rotate (4/2) / useless_nonzero_new_unchecked (2/2) / manual_next_back | inherent_to_string + from_over_into: impl-reauthoring class (Display body authoring / Into header+body rewrite) — DEFER thin; manual_rotate: loop-carried state shape — DEFER thin; useless_nonzero_new_unchecked: unsafe + NonZero invariant shape — DEFER thin |
| empty_docs (3/2) | DEFER thin — deletion is bounded but doc-position attr surgery on comments (the tabs_in_doc_comments class had span-respan traps) |

**The pack: THREE fresh bounded leads, eq-by-construction, dedup-clean**
(corpus + proposers greps: repeat/take, ok/err, rev/next_back families all
zero-hit):

1. `manual_repeat_n` (5/4, style/warn — GROUP CORRECTED at gate time:
   the installed driver says STYLE, not the upstream-docs perf read; the
   plan-188 lesson) — `std::iter::repeat(x).take(n)` →
   `std::iter::repeat_n(x, n)`. MSRV note: `repeat_n` stabilized 1.82;
   workspace toolchain 1.98 — context_note carries the floor.
2. `manual_ok_err` (2/2, complexity/warn — same gate correction, was
   read style) — the Option-conversion match
   (`Ok(v) => Some(v), Err(_) => None` → `.ok()`; the `.err()` twin).
3. `manual_next_back` (2/2, style/warn) — `iter.rev().next()` →
   `iter.next_back()`.

- [x] T0 — derivation + the 49-row adjudication table above + dedup greps
- [ ] T1 — the probe (`scripts/plan230_pack19_probe.py`, the plan188
      envelope): per-fixture clippy JSON runs; FIRING arms (std-path +
      imported spellings per lead), SILENT arms (repeat without take /
      take on non-repeat / non-identity ok-err arms / rev-then-forall /
      .last() forms), POSTURE arms (recorded, never gated), FIXED forms
      (compile + silent), suggestion-applicability dump, co-fire census
      (let_and_return / redundant_pattern_matching / manual_map / double_compare
      watch set)
- [x] T2 — author on PASS: corpus entries ×3 + proposers ×3 + the
      `all_fixed_code_parses` standing gate (the clippy_lints domain's
      per-entry G1 — the batchNN_pseudocode_parses convention is
      rust_perf/kernel_opt only) + FIXABLE/lint_level/dispatch/fixture-table
      registration + count pins (81→84 fixable, 100→103 corpus)
- [x] T3 — gates ALL GREEN: lib default 2538/0 (+33 vs prior 2505);
      clippy ×4 postures clean (incl. the pre-existing
      formatting_flip dead-code repair); freeze regen v2+v3 e2es PASS
      (84/88 atoms, 101 verified / 0 newly excluded each, M4 PASS,
      stratification PASS; 4 composites reshuffled, labels landed);
      score-bench double-run 96.36% FLAT identical (parse 1.0);
      retrieval floors HELD (kernel 100.0%, blended 91.2/94.7/95.3 —
      byte-identical to B227, no re-pin); Layer 5c 3920/0
- [x] T4 — Research 259; AGENTS.md corpus count 100→103 (84 fixable);
      snapshot discharge row; plan ticked (this commit)
- [ ] T5 — commit `feat: batch 228: ...` + push; .plans/.highwater
      already 229→230 at allocation

## Scope discipline

- Probe-first law: no authoring before the probe verdict is PASS.
- A drifted row narrows the pack (the lead drops to POSTURE/staged), it is
  never forced.
- manual_ok_err + manual_next_back ride only if their probes PASS; the
  pack may land as single-lead (manual_repeat_n) if the thin pair drifts —
  the plan-188 precedent (rule class justifies a thin single lead).

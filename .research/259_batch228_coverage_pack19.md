# Research 259 — Batch 228: the coverage-report re-arm pack XIX (the definition-identity trio)

**Status:** RECORD (batch landed 2026-10-13 — Plan 230)

## What this batch was

The second empty-queue re-arm (the Research-191 pipeline, fourth run):
pools exhausted at Batch 227, walk #14 dry, nothing search-unblockable —
so the owed unit was a fresh `cargo refine --coverage-report` derivation
converted into a clippy_lints corpus pack.

## The derivation (2026-10-13, clippy 0.1.98 48a229ceae, the installed driver)

- 822 lints / 9 groups; corpus 100 clippy_lints rules; compiled fix set 97
- UNCOVERED 722 (409 fire by default); evidence-backed 136; multi-crate
  (spread ≥ 2) uncovered evidence rows: **49**
- vs the 09-24 derivation (Plan 169: 94 covered / 728 uncovered / 415
  default-firing): corpus +6 covered, uncovered −6 — same toolchain, same
  t04 evidence dataset; the moved rows are the landed batches' own lints.
  **The head is CONSUMED** — every top-20 row carries a standing
  adjudication (the skip ledger), and the pack leads came from the tail.

## The adjudication (Plan 230 T0 — the full 49-row table)

Standing verdicts re-confirmed: missing_*_doc (doc-authoring),
expect_used/absolute_paths (restriction), result_unit_err/too_many_arguments
(architectural), len_zero (DUPLICATE), needless_borrows_for_generic_args/
double_ended_iterator_last/get_first (type-blind / receiver-collision),
derivable_impls (structural DEFER, no new evidence), manual_div_ceil +
int_plus_one (overflow REJECTS), missing_transmute_annotations
(probe-REJECT), useless_borrows_in_formatting (MEASURED INERT),
unwrap_or_default (receiver-collision), field_reassign_with_default
(plan-165), the wave-4 anecdotes (obfuscated_if_else 10/1,
upper_case_acronyms 8/1 — and transmute_ptr_to_ref CONFIRMED spread-1
this cycle from the t04 histogram, same class).

Fresh REFUSES recorded (first adjudication, with reasons): to_string_trait_impl
(body-reauthoring — impl Display requires authoring a fmt body from a
String-returning body, the fn-body-invention class), multiple_crate_versions
(cargo group — not a code fix), test_attr_in_doctest (markdown — unparseable
by the L2 syn pruner), large_enum_variant (architectural),
only_used_in_recursion (signature rippling), should_implement_trait +
enum_variant_names (call-site/type rippling renames), cast_sign_loss (the
cast-family typeck class), needless_maybe_sized (bound-resolution semantics),
unbuffered_bytes (semantic I/O-contract choice), non_canonical_clone_impl
(derive applicability needs field types).

Fresh thin DEFERs (structural risk each, ≤7 counts): nonminimal_bool,
borrow_deref_ref, needless_question_mark, explicit_auto_deref,
collapsible_match, redundant_guards, explicit_counter_loop,
extra_unused_lifetimes, from_over_into, inherent_to_string, manual_rotate,
useless_nonzero_new_unchecked, empty_docs.

## The pack — three leads, all eq-by-construction

1. **`manual_repeat_n`** (5/4) — `iter::repeat(x).take(n)` →
   `iter::repeat_n(x, n)`. The adapter is DEFINED as the expansion; the
   input's compilation proves `T: Clone`. MSRV floor 1.82 carried as the
   proposer's context_note (the first MSRV-flavored context_note).
2. **`manual_ok_err`** (2/2) — the two-arm identity match → `.ok()` /
   `.err()`. `Option::ok/err` are DEFINED as these matches. The if-let
   form fires upstream (probe-measured, MachineApplicable) but refuses v1
   — the documented narrower-than-clippy divergence (plan-169
   len_without_is_empty v1-refuses-traits precedent).
3. **`manual_next_back`** (2/2) — `it.rev().next()` → `it.next_back()`.
   `Rev<I>` inverts direction, so the rewrite is the adapter's own
   definition; the input's compilation proves `DoubleEndedIterator`.

Probe (`scripts/plan230_pack19_probe.py`, the plan188 envelope): VERDICT
PASS after two expectation corrections — clippy is BROADER than assumed on
both: manual_repeat_n fires on ARBITRARY take-args (`take(n + 1)` →
`repeat_n(1u64, n + 1)`, whole-chain splice), and manual_ok_err fires on
the if-let form too (recorded as POSTURE). Zero co-fires on the watch set;
6 fixed forms compile clean + silent; the imported spelling's upstream
suggestion is FULLY-QUALIFIED (`std::iter::repeat_n`) even under
`use std::iter::repeat;` — the proposer emits exactly that (the
divergence-free choice).

## The batch's finding — the GROUP CORRECTION class, again

The installed driver's derivation (`table_and_corpus_match_installed_driver`,
the lint_level gate) caught BOTH group reads wrong at test time:
`manual_repeat_n` is STYLE (upstream docs + the t04 histogram era say
perf), `manual_ok_err` is COMPLEXITY (was read style). `manual_next_back`
= style passed. Same lesson as plan 188's `mem_replace_with_default`
(style, not perf): **the installed driver is the authority, never the
docs** — the gate's own catch, recorded.

## Freeze record (plan 137 T7.5 — corpus delta)

- v2 e2e PASS: 84 atoms (3 new — all verify clean, none in the bad set),
  101 verified / 0 newly excluded / 19 pre-excluded; per-k solve 100.0%
  at EVERY k; M4 monotone-decay PASS. 4 composites reshuffled by the
  stride windows (the 3 new atoms sort into the `manual_*` band):
  k2 `manual_next_back+manual_ok_err`, k3 `manual_next_back+manual_ok_err+
  manual_range_contains` + `manual_range_patterns+manual_repeat_n+
  manual_slice_fill`, k4 `manual_let_else+manual_map+manual_next_back+
  manual_ok_err` — labels landed as data, exclusions unchanged.
- v3 e2e PASS: 88 atoms (4 persist + 84 corpus; bad 10), 101 verified /
  0 newly excluded / 23 pre-excluded; stratification gate PASS; the v3
  labels re-spliced (same 4-window reshuffle shape).
- The loudness gates red BEFORE the regen (the two
  `freeze_status_holes_are_loud` failures) and green after — the plan-137
  mechanism working exactly as designed.

## GOAT record

- lib default **2538/0** (prior 2505 — +33 from the three proposers' test
  arms + registration); clippy_lints slice 1361/0
- `--all-features --all-targets` clippy clean (one PRE-EXISTING
  dead-code error repaired in `examples/formatting_flip.rs` — the
  Bench-115 instrument's `fail_count` field was declared-but-never-read;
  now read into the per-file summary line as failed-trajectory mass;
  `e6c02dae` predates this batch)
- Layer 5c all-features tests **3920/0**
- score-bench double-run: gen-1 **96.36% FLAT bit-identical** (created
  1.8%, parse safety 1.0 — the new rules ride the gen-2 windows; the
  gen-1 floors untouched as expected)
- retrieval floors HELD: kernel self-variants **100.0%**, blended
  **91.2/94.7/95.3** — byte-identical to the batch-227 baseline, no re-pin

## Re-arm economics (the NEXT POOL state)

The 49-row evidence tail is now FULLY adjudicated: 3 landed, ~14 standing
verdicts re-confirmed with reasons, ~13 fresh REFUSES recorded, ~13 thin
DEFERs recorded (bounded only under structural risk; revisit on new
evidence), the rest single-crate anecdotes. The NEXT re-arm window needs
either a toolchain move (new lints) or a fresh evidence dataset (a t05
histogram wave) — the t04-era tail is consumed. Next clocks: arxiv walk
#15 opens Tue 10-20 (claim first); the nnethercote trigger stays checked
per idle pass.

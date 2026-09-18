# Plan 146 — Batch 155: the frontier-lead mining pack (5 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch landing

Source: the frontier miner's first yield (Issue 122 / Plan 145 / Research 182)
— 5 PENDING mining leads committed as frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, each oracle-verified to fire EXACTLY
its target lint under all+pedantic+nursery (probe-first already done: the
fixture IS the probe). This batch consumes them: land the rules, graduate the
families, re-arm the frontier with fresh targets.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `let_and_return` | `let x = expr; x` tail → `expr` | FIXABLE | guards: simple `let` (no annotation/mut/attrs), binding used exactly once (the tail), never assigned after init, let is last stmt — parse-provable, inference-identical |
| `ptr_arg` | `&Vec<T>`/`&String` param → `&[T]`/`&str` | VERIFY | type-blind callee/body shapes (Vec-only methods, `&Vec`-expecting callees) — E0308 check-visible, the compile gate reverts |
| `redundant_slicing` | `f(&s[..])` → `f(s)` | FIXABLE | only when the ident resolves to a same-file fn param of type `&str`/`&[T]` — then the slice is provably redundant |
| `map_flatten` | `.map(f).flatten()` → `.flat_map(f)` | FIXABLE | adjacent-chain rewrite; Map's Item IS f's output, `flatten`/`flat_map` share the IntoIterator bound — semantics-identical whenever both compile |
| `needless_borrow` (receiver extension) | `(&s).to_uppercase()` | stays self_evolve-only | extend `RECEIVER_METHODS` with str-inherent methods (no `impl for &T` shadowing in std); CANNOT join FIXABLE/VERIFY — `clean_spans_fire_no_lint` forbids it (free-fn shape over-matches, the documented BY-DESIGN exclusion) |

Counts: corpus 64 → 68 (4 new entries; needless_borrow's exists), FIXABLE
47 → 50 (let_and_return, redundant_slicing, map_flatten), VERIFY 14 → 15
(ptr_arg), atoms 46 → 49 (the 3 FIXABLE additions — `vocabulary()` is
FIXABLE-only, so ptr_arg/needless_borrow never touch the freeze tables).

## The graduation (frontier miner)

Landing the rules makes the 5 committed frontier rows UNVERIFIABLE
(`verify_committed` requires `healed_default == broken` — the rule now heals
it). The designed consumption path (KnownSet doc): **archive the row, re-open
the family**. Design decisions:

- The 5 families flip `covered: true` and join the **HeldOut population**:
  per-family `CorpusView::holding_out(rule)` + `Solution::Reenable(rule)` —
  the full invariant loop (unsolved-before / solved-after / regression /
  exactly-once) re-proves every landed rule against its family fixture on
  every e2e rerun, via the DISPATCH view (works for self_evolve-only
  needless_borrow too — tier is a --fix axis, not a dispatch axis).
- The 5 frontier rows are ARCHIVED out of the active store (git history keeps
  them; they were PENDING leads, now consumed).
- The frontier arm needs ≥1 live family for the exit condition — re-arm with
  2 NEW oracle-verified frontier families (candidates: `io_other_error`,
  `assigning_clones`, `unit_arg`, `wrong_self_convention`; probe + take the
  2 cleanest — seeds clean under all+pedantic+nursery incl. the
  missing_const_for_fn discipline, broken fires exactly the target).

## Tasks

- [x] T0 Probe the 4 new-frontier candidates with real clippy; pick 2 clean
  → `io_other_error` + `wrong_self_convention` (both perfect: clean seeds,
  exact-fire brokens; `assigning_clones` seed dirty, `unit_arg` refuted)
- [x] T1 `let_and_return` proposer + module tests (10 tests)
- [x] T2 `ptr_arg` proposer + module tests (10 tests; VERIFY tier)
- [x] T3 `redundant_slicing` proposer + module tests (9 tests; param-resolver
      + local/closure shadow prepass — the name-collision soundness hole the
      own test suite caught)
- [x] T4 `map_flatten` proposer + module tests (6 tests)
- [x] T5 `needless_borrow` RECEIVER_METHODS extension (str-inherent trio) + tests
- [x] T6 Dispatch rows + tier rows (FIXABLE 47→50, VERIFY 14→15) + tier-count test
- [x] T7 Corpus entries ×4 + size-pin 64→68 + header doc
- [x] T8 mod.rs `all_lints_fire_and_parse` rows (64→68) + count comments
- [x] T9 Miner graduation: `holding_out`, per-family e2e loop,
      verify_committed generalization, exit rework, 5 rows archived.
      **Design correction (measured):** `heal_fixpoint_cfg` dispatches
      FIXABLE_LINTS only — `ptr_arg` (VERIFY) + `needless_borrow`
      (self_evolve-only) CANNOT be loop-verified, so their families RETIRED
      (regression home = module tests + the `all_buggy_codes_fire_their_lint`
      corpus lockstep); 3 FIXABLE families graduated to the heldout loop.
      +2 new frontier families re-armed.
- [x] T10 Freeze regen v2+v3 — the REAL oracle found `let_and_return` is a
      BROAD rule (3 pre-existing atoms' let-tail fixtures cross-fire it:
      deref_addrof/legacy_numeric_constants/unreadable_literal → bad-atom
      set 2→5): 12 v2 / 14 v3 exclusions, 74/76 labeled, both e2es green
      (zero drift, 100% solve at every k, M4 PASS) + the record paragraph.
      Also: the needless_bool eval fixture reshaped (its let-tail wrapper
      legitimately fired let_and_return — fix-before-repin).
- [x] T11 score-bench: row #130 heal_rate 96.4% (baseline 96.2% + the new
      atoms), FLAT-class; frontier snapshot dedup consistent
- [x] T12 Gates: lib default **1773/0** · lib `frontier_miner` **2090/0**
      (retrieval floors HELD, no re-pin) · clippy `-D` clean in 3 states
      (default --tests / latent_retrieval --lib / frontier example) ·
      frontier-miner e2e **VERDICT ✓ twice** (graduation run + verify-pass
      rerun; 6 committed rows, 0 drift)
- [x] T13 Docs: HISTORY batch entry, `.distill/001` intake, AGENTS counts,
      plan status; commit + push

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat 96.2% ± the new atoms' contribution;
  gen-8 shaped-verdict non-vacuity is the admission bar).
- G3: freeze e2e v2/v3 re-verified green (100% k=1 solve on the 3 new atoms);
  frontier miner e2e VERDICT ✓ (graduation + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract).

# Plan 148 — Batch 157: the frontier-lead mining pack III (2 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch landing

Source: the Batch 156 re-arm's 2 committed leads (frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified 2026-09-18 by
`scripts/plan147_frontier_probe.py`) — consumed as rules, families graduated,
frontier re-armed with fresh probe-verified leads.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `redundant_closure_for_method_calls` | `v.iter().map(\|s\| s.len())` → `v.iter().map(String::len)` where `v: &[String]` | FIXABLE | The rewrite needs the closure param's type — parse-only undecidable in general, so the rule derives it from the ENCLOSING FN SIGNATURE: binding typed `&[T]` / `&Vec<T>` / `&[T; N]` (shared ref) + `.iter()` receiver + single-param closure whose body is exactly `p.m()` (0 args). Emit `T::m` ONLY for a whitelisted (T head, method) table — String/Vec/HashMap/HashSet × len/is_empty/capacity, all inherent `&self` 0-arg std methods — so the fn-item signature `fn(&T) -> _` matches the closure's by construction. Guards: binding never rebound/shadowed in the fn body (any PatIdent/assign occurrence refuses), file shadow guard (items or non-std use-aliases named String/Vec/HashMap/HashSet refuse the file, the io_other_error pattern — factored into helpers), generic type params named like the head refuse, no turbofish/attrs/interior comments. Pedantic-group lint (fires under `-W clippy::pedantic`; the bench oracle runs pedantic). |
| `iter_skip_next` | `v.iter().skip(n).next()` → `v.get(n)` (proven slice/Vec/array binding) — generic fallback arm `x.skip(n).next()` → `x.nth(n)` (clippy's own suggestion, identical by construction for every Iterator) | FIXABLE | TWO arms. Arm A (the oracle-clean inverse, one step better than clippy's own fix — measured round 3: `.nth(n)` fires `iter_nth` on slice iters): when the receiver chain is `<ident>.iter()` and the ident is signature-proven `&[T]`/`&Vec<T>`/`&[T; N]`, emit `<ident>.get(n)` — slice/Vec `get(i)` ≡ `iter().skip(i).next()` exactly (both `Option<&T>`, both None out of range, no panic). Arm B (full surface, no derivation): `X.skip(A).next()` → `X.nth(A)` — `Iterator::nth(n)` IS `skip(n).next()` by std semantics for every iterator; the parse-only method-name match is the `iter_nth_zero`/`filter_next` precedent. Deleted-range comment guards both arms; skip-arg text rides verbatim. |

Counts: corpus 70 → 72 (2 new entries), FIXABLE 51 → 53 (BOTH — unlike
Batch 156's VERIFY retiree), atoms 51 → 53, both new atoms verified
exact-fire (no new bad atom expected — the e2e measures).

## The graduation (frontier miner)

- BOTH families flip `covered: true` and join the HeldOut per-family loop
  (6th + 7th family loops — both FIXABLE, so both are loop-verifiable).
- The 2 consumed frontier rows are ARCHIVED out of the active store.
- Re-arm the frontier with 2 NEW oracle-verified families. Candidates:
  `to_string_in_format_args` (the Batch 156 verified spare — seed must be the
  INLINED `format!("n={x}")` form; the `format!("n={}", x)` inverse is dirty
  via uninlined_format_args, which skips method-call args) + probe
  `manual_range_contains` (Bench 094 wave-2 evidence 121 warnings / 5 crates)
  with `unit_arg` (46/2) as fallback. Probe-first: seeds clean under
  all+pedantic+nursery, broken fires EXACTLY its target lint.

## Tasks

- [x] T0 Plan file (this) + `.plans/.highwater` 147→148
- [x] T1 Re-arm probe (`scripts/plan148_frontier_probe.py`, modeled on
       plan147's): → **`to_string_in_format_args`** + **`manual_range_contains`**
       (both clean/exact; `single_char_add_str` verified spare; rejects:
       mrc's iterator form + `iter_next_slice` SILENT)
- [x] T2 Helpers: factored the io_other_error shadow guard into
       `helpers::file_shadows` (io_other_error refactored onto it) +
       `helpers::slice_param_heads_over` (signature derivation + rebind
       guard), shared by both proposers
- [x] T3 `redundant_closure_for_method_calls` proposer + module tests (17)
- [x] T4 `iter_skip_next` proposer + module tests (14, both arms)
- [x] T5 Dispatch rows + FIXABLE 51→53 + tier-count test (len 53) +
       excluded-lints test rows
- [x] T6 Corpus entries ×2 + size pin 70→72 (`corpus_size_is_72`) +
       header doc + `all_lints_fire_and_parse` rows 70→72 + mod.rs doc counts
- [x] T7 Miner graduation: families flip ×2 + 2 new frontier families +
       family_cases + hermetic probe markers + test counts (heldout 12→16
       cases / 5→7 families; frontier 4→4)
- [x] T8 Freeze regen v2+v3 (real oracle): atoms 51→53 — both e2es re-run
       green (v2 75/20, v3 76/23; bad-atom set v2 5→6 — the rcmfc atom
       fires on `unnecessary_sort_by`'s 2-param fixture, ours rightly
       refuses) + record paragraph; **12 orphaned EXCLUDED rows pruned**
       (the gen-2/gen-3 lanes' orphan check caught them — the e2e's printed
       record carries stale pre-exclusions blindly)
- [x] T9 frontier-miner e2e ×2 (graduation + verify-pass — VERDICT ✓ both;
       9 committed rows, 0 drift); the 2 consumed frontier rows archived
- [x] T10 score-bench row #132 (heal_rate 96.36% FLAT vs baseline, vocab
       53) + frontier snapshot (generation 7, unchanged-dedup) + an honest
       gen-3 lane row at 95.0% (the orphan-check run)
- [x] T11 Gates: lib default 1829/0 (+32) · lib `latent_retrieval` 1982/0 ·
       lib `frontier_miner` 2146/0 (retrieval floors HELD) · clippy `-D`
       clean in the 3 states (default --all-targets / latent_retrieval
       --lib / frontier example)
- [x] T12 Docs: Research 184 note, HISTORY batch entry, `.distill/001`
       queue line, AGENTS counts (72/53) + frontier_miner bullet, README
       clippy_lints row; commit + push

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat vs the 96.36% baseline ± the 2 new
  atoms).
- G3: freeze e2e v2/v3 re-verified green (0 drift); frontier miner e2e
  VERDICT ✓ ×2 (graduation incl. the 2 new family loops + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract).

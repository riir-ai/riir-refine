# Plan 147 — Batch 156: the frontier-lead mining pack II (2 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch landing

Source: the frontier miner's re-armed leads (the Batch 155 T0 re-arm) — 2
PENDING mining leads committed as frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, each oracle-verified to fire EXACTLY
its target lint under all+pedantic+nursery (probe: `scripts/plan146_frontier_probe.py`,
2026-09-18). This batch consumes them: land the rules, graduate the families,
re-arm the frontier with fresh targets.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `io_other_error` | `io::Error::new(io::ErrorKind::Other, e)` → `io::Error::other(e)` | FIXABLE | `Error::new` and `Error::other` share the IDENTICAL `E: Into<Box<dyn Error + Send + Sync>>` bound — inference-identical whenever the kind is literally `ErrorKind::Other` (path-form only). Shadow guard (the `legacy_numeric_constants` precedent): refuse files carrying items named `Error`/`ErrorKind`/`io` or use-aliases rebinding those names. MSRV floor: `Error::other` stabilized 1.74 (workspace floats 1.98.1). Default-eco evidence: 57 warnings / 8 crates (Bench 094 wave 4). |
| `wrong_self_convention` | `fn into_*(&self)` → `fn into_*(self)` | VERIFY | receiver change is type-blind (body may borrow self → E0515; callers pass borrows → E0308/E0382) — all check-visible, the compile gate reverts (the `ptr_arg` precedent). v1 scope: INHERENT impls, `into_*` prefix, plain `&self`/`&mut self` receiver, NO visibility modifier (clippy's `avoid_breaking_exported_api` default skips exported items), no method attrs. Default-eco evidence: 19 warnings / 6 crates. |

Counts: corpus 68 → 70 (2 new entries), FIXABLE 50 → 51 (`io_other_error`),
VERIFY 15 → 16 (`wrong_self_convention`), atoms 50 → 51 (vocabulary is
FIXABLE-only; `wrong_self_convention` never touches the freeze tables' atom
set — but the 2 corpus entries still reshuffle the stride windows → freeze
regen REQUIRED, the Batch-151 lesson).

## The graduation (frontier miner)

- `io_other_error` FIXABLE → family flips `covered: true`, joins the HeldOut
  per-family loop (`CorpusView::holding_out("io_other_error")` — every e2e
  rerun re-proves the full invariant against real clippy).
- `wrong_self_convention` VERIFY → family RETIRED (the fixpoint dispatch is
  FIXABLE-only; regression home = module tests + the
  `all_buggy_codes_fire_their_lint` corpus lockstep).
- The 2 committed frontier rows are ARCHIVED out of the active store (git
  history keeps them; they are consumed leads).
- Re-arm the frontier arm with 2 NEW oracle-verified families (candidates
  from the Bench-094 wave-4 staged rows: `double_ended_iterator_last` 17,
  `match_ref_pats` 17, spares `redundant_closure_for_method_calls`,
  `obfuscated_if_else` — probe + take the 2 cleanest; seeds must be clean
  under all+pedantic+nursery incl. the missing_const_for_fn discipline).

## Tasks

- [x] T0 Plan file (this) + `.plans/.highwater` 146→147
- [x] T1 Probe the re-arm candidates with real clippy
      (`scripts/plan147_frontier_probe.py`, 3 rounds): → **`redundant_closure_for_method_calls`**
      + **`iter_skip_next`** (both clean/exact; `to_string_in_format_args` verified spare;
      rejects recorded: del_last/match_ref_pats/manual_strip silent, rcmc v1
      co-fired implicit_clone, len_zero/iter_count const-ability-dirty)
- [x] T2 `io_other_error` proposer + module tests (shadow guard + path-form
      variants + turbofish/arg-count refusals)
- [x] T3 `wrong_self_convention` proposer + module tests (inherent-impl-only,
      no-vis-only, into_ prefix, receiver splice)
- [x] T4 Dispatch rows + tier rows (FIXABLE 50→51, VERIFY 15→16) +
      tier-count test (len 51) + excluded-lints test row
- [x] T5 Corpus entries ×2 + size pin 68→70 + header doc
- [x] T6 mod.rs `all_lints_fire_and_parse` rows (68→70) + doc-count lines
- [x] T7 Miner graduation: families table (flip io_other_error, retire
      wrong_self_convention, add 2 new), family_cases, doc comments; archive
      the 2 consumed frontier rows from the store
- [x] T8 Freeze regen v2+v3 (real oracle): atom 50→51 + corpus 68→70 moves
      the stride windows — regenerate FROZEN_LABELS/EXCLUDED_IDS as data,
      both e2es green + record paragraph
- [x] T9 frontier-miner e2e ×2 (graduation run + verify-pass rerun)
- [x] T10 score-bench row (heal_rate vs 96.4% baseline) + frontier snapshot
- [x] T11 Gates: lib default · lib `frontier_miner` (retrieval floors HELD)
      · clippy `-D` clean in the 3 states (default --tests /
      latent_retrieval --lib / frontier example)
- [x] T12 Docs: Research 183 note, HISTORY batch entry, `.distill/001`
      intake, AGENTS counts; commit + push

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat vs the 96.4% baseline ± the new atom).
- G3: freeze e2e v2/v3 re-verified green; frontier miner e2e VERDICT ✓
  (graduation + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract).

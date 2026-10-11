# Plan 149 — Batch 158: the frontier-lead mining pack IV (2 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch landing

Source: the Batch 157 re-arm's committed leads (frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified 2026-09-18 by
`scripts/plan148_frontier_probe.py`) — consumed as rules, families
graduated, frontier re-armed with fresh probe-verified leads.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `to_string_in_format_args` | `format!("n={}", x.to_string())` → `format!("n={}", x)` (the strip; the seed's inlined form is reached by the `uninlined_format_args` fixpoint step on the NEXT iteration — the composition both proposers were designed for: uninlined skips method-call args, this rule fires exactly there) | FIXABLE | The rewrite needs the receiver's `to_string` to be the std Display BLANKET — undecidable parse-only in general. Solved by SIGNATURE DERIVATION (the `slice_param_heads_over` pattern): the receiver is a single-segment ident that is a fn/impl-method param with an EXPLICIT scalar annotation (`u8`/`i32`/`f64`/`bool`/`char`/… + `&str`/`String`/`&String`), never rebound in the body. For every such type the ONLY in-scope `to_string` is `impl<T: Display> ToString for T` (a same-file or imported trait with its own `to_string` for a primitive would make the ORIGINAL not compile — E0031 ambiguity — so every compiling input is safe); `{x.to_string()}` ≡ `{x}` by blanket-Display identity. The literal-receiver arm (`"lit".to_string()`) is construction-identical (str blanket). Placeholder guard: fires ONLY where the arg fills a bare `{}` (implicit Display) — a `{:?}`/spec placeholder makes the strip output-CHANGING (Debug String ≠ Debug u32), so spec args refuse (the `has_dynamic_spec_ref`/`classify_placeholder` machinery reused from `uninlined_format_args`). Macro family table reused verbatim; named/assign args abort the macro (positional mapping must be exact); direct post-template family-macro children are skipped (the `format_in_format_args` boundary, the uninlined precedent). Perf-group lint (default-on; group pinned by probe). |
| `manual_range_contains` | `x >= 3 && x <= 5` → `(3..=5).contains(&x)` (+ the exclusive `x > lo && x < hi` → `(lo..hi).contains(&x)` twin, and the mirrored bounds-left forms `lo <= x && hi >= x` / `lo < x && hi < … > x`) | FIXABLE | `lo <= x && x <= hi` ≡ `(lo..=hi).contains(&x)` for EVERY PartialOrd by std's `contains` definition (matches! partial_cmp Less/Equal on both legs — NaN false on both sides, infinities equal both sides); `lo < x && x < hi` ≡ `(lo..hi).contains(&x)` identically. The compile-direction hole (custom `E: PartialOrd<L>` mixed types — original compiles, fixed needs `L: PartialOrd<E>`) is closed by the SAME signature derivation: the shared operand must be a param-proven NUMERIC/char/bool scalar and the bounds LITERALS (int/float/char, optional sign) — then both range ends unify with the operand's type and every impl is same-type. Shared-operand text identity is byte-compared (the `identity_op` precedent). Mixed >=/< pairs, leading-`&` operands, `||` chains, and comments inside the deleted glue refuse. |

Counts: corpus 72 → 74 (2 new entries), FIXABLE 53 → 55, VERIFY unchanged
(16), atoms 53 → 55 (both leads FIXABLE — every lead joins the atom set).

## The graduation (frontier miner)

- BOTH families flip `covered: true` and join the HeldOut per-family loop
  (8th + 9th family loops — both FIXABLE, so both are loop-verifiable).
- The 2 consumed frontier rows are ARCHIVED out of the active store.
- Re-arm the frontier with 2 NEW oracle-verified families (probe-first:
  `scripts/plan149_frontier_probe.py`; candidates: `single_char_add_str`
  the Batch-156/157 verified spare, `unit_arg` (Bench 094 wave-4, 46/2),
  fallbacks `manual_clamp` / `from_str_radix_10`).

## Tasks

- [x] T0 Plan file (this) + `.plans/.highwater` 148→149
- [x] T1 Re-arm probe (`scripts/plan149_frontier_probe.py`): pick the 2
       fresh leads + pin `to_string_in_format_args`'s lint group
       (default-on arm) — record accepts/rejects
- [x] T2 Helpers: `scalar_param_heads_over` (the signature-derivation twin
       of `slice_param_heads_over` for scalar annotations, shared rebind
       guard) in `helpers.rs`
- [x] T3 `to_string_in_format_args` proposer + module tests (incl. the
       `{:?}`-spec refusal, named-arg abort, nested-macro boundary,
       non-proven receiver refusal, literal-receiver arm)
- [x] T4 `manual_range_contains` proposer + module tests (both operand
       orders, inclusive + exclusive, mixed-pair refusal, comment refusal,
       unproven-ident refusal, `while` condition arm)
- [x] T5 Dispatch rows + FIXABLE 53→55 + tier-count test (len 55)
- [x] T6 Corpus entries ×2 + size pin 72→74 (`corpus_size_is_74`) + header
       doc + `all_lints_fire_and_parse` rows 72→74 + mod.rs doc counts
- [x] T7 Miner graduation: families flip ×2 + 2 new frontier families +
       family_cases + hermetic probe markers (family_probe += 2 arms;
       frontier_probe = the new leads) + test counts (heldout 16→20
       cases / 7→9 families; frontier 4→4)
- [x] T8 Freeze regen v2+v3 (real oracle) + orphan prune + record
       paragraph — RUN the generation lane before claiming green (the
       Batch 157 process note)
- [x] T9 frontier-miner e2e ×2 (graduation + verify-pass — VERDICT ✓
       expected ×2); the 2 consumed frontier rows archived
- [x] T10 score-bench row (heal_rate ~96.36% FLAT vs baseline, vocab
       55) + frontier snapshot
- [x] T11 Gates: lib default · lib `latent_retrieval` · lib
       `frontier_miner` (retrieval floors HELD) · clippy `-D` clean in
       the 3 states
- [x] T12 Docs: Research note, HISTORY batch entry, `.distill/001` queue
       line, AGENTS counts (74/55) + frontier_miner bullet, README row;
       commit + push
- [x] T13 Global fleet binary rebuild (release set, isolated worktree
       beside the repo + `/tmp` target) + `--corpus-stats` verify

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat vs the 96.36% baseline ± the 2 new
  atoms).
- G3: freeze e2e v2/v3 re-verified green (0 drift); frontier miner e2e
  VERDICT ✓ ×2 (graduation incl. the 2 new family loops + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract).

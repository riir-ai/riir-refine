# Plan 152 — Batch 161: the frontier-lead mining pack VII (2 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch
landing. T1's probe measured three design-correcting facts: (1) clippy's
own unit_arg suggestion is BYTE-FOR-BYTE the hoist shape we designed;
(2) the TRAILING-block shape (`touch(hits, { .. })`) is clippy-SILENT on
1.98 — our index-0 scope loses nothing live; (3) `string_lit_as_bytes` is
NURSERY (the miner comment's "default-level perf" wrong on both counts)
AND clippy's suggestion for the `\u{41}` escape shape is INVALID RUST
(`b"\u{41}"` does not compile) — we refuse the shape clippy breaks on.
En-route lessons: the heldout MARKER must be BROKEN-only (the first
`{ hits.push(1); }` marker matched the healed hoist too and the
solved-after clause rejected the family); the freeze regen's member parse
must split the id's THIRD dash segment (`gen3-k`-prefix stripping failed
silently — caught by the exclusion-count sanity check before any splice).

Source: the Batch 160 re-arm's leads (frontier rows 14–15 in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified 2026-09-18 by
`scripts/plan151_frontier_probe.py`) — consumed as rules, families
graduated, frontier re-armed with fresh probe-verified leads.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `unit_arg` | `touch({ hits.push(1); }, hits)` → `{ hits.push(1); } touch((), hits)` | FIXABLE | The block is PROVABLY unit-typed purely syntactically: its last stmt is a `;`-statement (no tail expression) — a block with no tail expr has type `()` by the language definition, so the arg's value IS `()` and spelling it as the literal is value-identical. The fix HOISTS the block before the call and passes `()`. Evaluation order is identical by construction: Rust evaluates call args left-to-right, so hoisting the block at arg index 0 (the only index where order is provable) preserves the exact sequence (block → rest args → call). Borrow scoping unchanged (the block ends before the call either way). Guards: the unit block must be the FIRST arg and the ONLY block arg; the call must be a direct statement-or-tail child of a `Block` (hoisting needs statement context); block labels/attrs refuse. Sub-case: an EMPTY block `{}` at any index → `()` in place (no hoist, no order concern, no side effects). Comments in glue: the block text rides VERBATIM inside the replacement (its interior can't be lost); the rest of the call span is re-spelled — refuse on interior comments outside the block span (the `drops_interior_comment` keep-list = the block span). |
| `string_lit_as_bytes` | `"abc".as_bytes()` → `b"abc"` | FIXABLE | Construction identity in every coercion context: `b"abc"` is `&'static [u8; 3]`, `"abc".as_bytes()` is `&'static [u8]` — at every use where a `&[u8]` (or wider) is expected the array-ref coerces to the slice-ref with the same bytes and the same lifetime. clippy's own documented known-problem (nursery lint, MachineApplicable): inference-sensitive contexts (`fn f<T: ?Sized>(t: &T)`, `into_iter()`) can pick `[u8; N]` — the `--fix --verify` compile gate reverts exactly those edits, the parity posture with clippy's own MachineApplicable suggestion. Guards: the receiver must be the literal ITSELF (paren/ref receivers refuse); ASCII-only snippet content (byte literals reject non-ASCII source chars — clippy's own lint is ASCII-scoped for the same reason); NO `\u{` escape in the snippet (byte strings cannot spell unicode escapes — everything else `\n \t \\ \xNN \0 \'` is valid byte-string syntax and rides verbatim); raw strings prefix `b` before `r` (`r"k9"` → `br"k9"`); exactly 0 args; attrs refuse; interior comments refuse (the whole span is re-spelled). Group: NURSERY (allow-by-default — pinned by the plan152 probe's default-groups arm; the miner comment's "default-level perf" was wrong on both counts). |

Counts: corpus 78 → 80 (2 new entries), FIXABLE 59 → 61, atoms 59 → 61
(both leads FIXABLE — the fifth consecutive all-FIXABLE pack if both
graduate).

## The graduation (frontier miner)

- BOTH families flip `covered: true` and join the HeldOut per-family loop
  (14th + 15th family loops — both FIXABLE, both loop-verifiable).
- The 2 consumed frontier rows are ARCHIVED out of the active store
  (the Batch-159 measured lesson: without the archive the e2e reads
  heldout_ok=false — the KnownSet family-done guard).
- Re-arm the frontier with 2 NEW oracle-verified families (probe-first:
  `scripts/plan152_frontier_probe.py`; candidates from the plan151
  verified-alternates record — `bytes_count_to_len` + `bool_comparison`
  preferred (both complexity/default-on, both definition-identity fixes:
  `str::len` IS `bytes().count()`, boolean algebra on proven-bool
  operands), `manual_string_new` + `needless_late_init` (twice-used
  -binding dodge) as recorded spares).

## Tasks

- [x] T0 Plan file (this) + `.plans/.highwater` 151→152
- [x] T1 Probe (`scripts/plan152_frontier_probe.py`): default-groups arms
       pinned BOTH leads' lint groups (unit_arg complexity/default-ON —
       fires bare; string_lit_as_bytes NURSERY — silent bare, the stale
       miner comment corrected) + shape arms (ua: unit-fn-call arg fires
       [+missing_const_for_fn dirt]; idx1 trailing-block SILENT; empty {}
       → `()`; value-tail SILENT — the refusal twin; suggestion captured:
       the hoist IS clippy's own suggestion shape; slb: raw → `br"a\nc"`,
       escapes verbatim, `\u{41}` FIRES with an INVALID-RUST suggestion —
       refused upstream bug, non-ASCII silent) + the re-arm candidates
       (4 verified pairs) — ARMED `bytes_count_to_len` + `bool_comparison`
- [x] T2 `unit_arg` proposer + module tests (13 arms: seed clean, hoist,
       second family case, stmt-with-semi position, empty-block in-place,
       idx1 refusal, value-tail refusal, labeled refusal, second-block-arg
       refusal, nested-call refusal, method-call refusal, comment-in-glue
       refusal, comment-inside-block rides)
- [x] T3 `string_lit_as_bytes` proposer + module tests (11 arms: seed
       clean, plain, second family case, raw, escape-verbatim,
       unicode-escape refusal [the upstream bug], non-ASCII refusal, paren
       receiver, named receiver, nested-in-larger-expr, comment refusal)
- [x] T4 Dispatch rows + FIXABLE 59→61 + tier-count test (len 61) + the
       mod.rs doc-table rows + scope-count prose 74→76
- [x] T5 Corpus entries ×2 + size pin 78→80 (`corpus_size_is_80`) + header
       doc + `all_lints_fire_and_parse` rows 76→78 + mod.rs doc counts
- [x] T6 Miner graduation: families flip ×2 + 2 new frontier families
       (`bytes_count_to_len` + `bool_comparison`, probe fixtures verbatim)
       + family_cases + hermetic probe markers (family_probe += the
       `({ hits.push` broken-only marker [the healed-hoist-match lesson]
       + `.as_bytes()`; frontier_probe = the new leads' markers) + test
       counts (heldout 28→32 cases / 13→15 families; frontier 4→4)
- [x] T7 Freeze regen v2+v3 (real oracle) + splice + record paragraph
       (the transient example WRITES ITS OWN OUTPUT FILE, invoked
       directly; the KEPT `scripts/plan150_splice_freeze_tables.py`
       spliced; 84 v2 labels / 13 exclusions + 85/16 v3 — PURELY ADDITIVE
       vs 160 [every composite window byte-identical, 0 reshuffle], bad
       sets UNCHANGED 6/7; v2 e2e 28.9s PASS + v3 e2e 28.2s PASS, 0 drift)
- [x] T8 frontier-miner e2e — the 2 consumed frontier rows ARCHIVED first
       (13 committed rows); VERDICT ✓ (36 seeds clean, 15 family loops, 2
       fresh admissions, 13 committed rows 0 drift, 4 rows appended, band
       fold 15/15)
- [x] T9 score-bench row #137 — heal_rate 96.36% FLAT (vocab 61) +
       frontier snapshot (generation 7)
- [x] T10 Gates: lib default 1941/0 · lib `latent_retrieval` 2094/0
       (floors held) · lib `frontier_miner,score_bench` 2258/0 · clippy
       clean ×4 states (4 structural fixes in the new modules: 2 doc
       backticks, a cloned-ref-to-slice, a single-match if-let, a
       too_many_lines table-allow; the transient regen example tripped an
       all-targets baseline failure class in the healer run — deleted
       first, the B160 transient pattern)
- [x] T11 Docs: Research note (188), HISTORY batch entry, `.distill/001`
       queue line (seventh turn), AGENTS counts (80/61) + frontier_miner
       bullet + README row; commit + push
- [x] T12 Global fleet binary rebuild (release set, isolated worktree
       beside the repo + `/tmp` target) + `--corpus-stats` verify —
       clippy_lints 80 through the installed copy

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat vs the 96.36% baseline ± the 2 new
  atoms).
- G3: freeze e2e v2/v3 re-verified green (0 drift); frontier miner e2e
  VERDICT ✓ ×2 (graduation incl. the 2 new family loops + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract) — the
  unit_arg hoist is a single-span replacement re-emitting the block text
  verbatim.

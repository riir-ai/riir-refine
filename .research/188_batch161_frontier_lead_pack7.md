# Research 188 — Batch 161: the frontier-lead mining pack VII (`unit_arg` + `string_lit_as_bytes`)

**Status:** RECORD (batch landed 2026-09-18, Plan 152; the Batch 160 re-arm's
leads consumed as rules, both families graduated, frontier re-armed with 2
fresh probe-verified leads).

## Source

The Batch 160 re-arm's frontier rows (`.benchmarks/frontier_miner_rows.jsonl`
rows 14–15, `solving_rule: null`, seeds verified firing exactly their targets
by `scripts/plan151_frontier_probe.py`). Consumed per the frontier-miner
discipline: land the rule → graduate the family to the HeldOut loop → archive
the consumed row → re-arm with the next probe-verified pair
(`scripts/plan152_frontier_probe.py`, this round).

## The two rules

### `unit_arg` (complexity, default-on — group pinned by the plan152 default-groups arm)

`touch({ hits.push(1); }, hits)` → `{ hits.push(1); };\ntouch((), hits)`.
The FIXABLE-bar identity is **syntactic unit-typeness**: a block whose last
stmt is a `;`-statement (no tail expression) has type `()` by the language
definition — the argument's value is ALREADY unit, so re-spelling it as the
literal is value-identical, and hoisting the block before the call preserves
evaluation order BY CONSTRUCTION (args evaluate left-to-right; the hoist arm
is index-0-only, the only position where order is provable without type
knowledge). The probe measured clippy's own MachineApplicable suggestion to be
byte-for-byte this shape (`'{ hits.push(1); };\ntouch((), hits)'`). The
empty-block arm (`touch({}, hits)` → `touch((), hits)`) is in-place.

Probe-measured boundaries the proposer mirrors:
- `touch(hits, { hits.push(1); })` (block at index 1, trailing) is **SILENT**
  on clippy 1.98 — our index-0-only scope is not narrower than the live lint
  on this shape class.
- A block WITH a tail expression (`{ num() }`) is silent — the refusal twin
  is the same syntactic check the lint's firing implies.
- A unit-returning CALL arg fires (+ `missing_const_for_fn` dirt on the
  trivial helper) — we refuse call-shaped args (v1; the probed shape is the
  block).

### `string_lit_as_bytes` (NURSERY — group corrected by the plan152 default-groups arm)

`"abc".as_bytes()` → `b"abc"`. Construction identity in every coercion
context (`&'static [u8; N]` → `&[u8]`); clippy's own documented known problem
(inference-sensitive contexts) is reverted by the `--fix --verify` compile
gate — the upstream `MachineApplicable` posture, documented in the module.

**Group correction**: the Batch 160 miner comment called this a "default-level
perf lint" — the probe's bare arm (no `-W` flags) is SILENT and the docs page
says nursery. Both halves of the old comment were wrong; the corpus entry
pins `nursery` and the FIXABLE_LINTS comment records the correction.

**The upstream suggestion bug we refuse to inherit** (probe-measured):
`"\u{41}".as_bytes()` FIRES and clippy suggests `b"\u{41}"` — which does not
compile (byte strings cannot spell unicode escapes). Our proposer audits the
literal snippet for `\u{` and refuses the shape; every other escape
(`\n`, `\xNN`, `\0`, `\\`) is valid byte-string syntax and rides verbatim
(measured: clippy re-emits the same spelling). Raw strings prefix `b` before
`r` (`r"a\n"` → `br"a\n"`, exactly clippy's suggestion). Non-ASCII content
refuses (byte literals reject non-ASCII source chars; clippy's lint is
ASCII-scoped — measured silent).

## GOAT

- **G1**: module tests ×24 (13 unit_arg + 11 string_lit_as_bytes);
  `all_buggy_codes_fire_their_lint` + `all_lints_fire_and_parse` + corpus
  `all_fixed_code_parses` lockstep at 80.
- **G2**: score-bench row #137 — heal_rate 0.96363634 **FLAT** vs the 96.36%
  baseline, vocab 61 (59→61).
- **G3**: freeze v2 e2e PASS (28.9 s) + v3 e2e PASS (28.2 s), **0 drift**;
  frontier e2e **VERDICT ✓** — 36 seeds clean / 15 family loops / 2 fresh
  admissions / 13 committed rows 0 drift / band fold 15/15; the re-arm
  admitted `bytes_count_to_len` + `bool_comparison` exact-fire.
- **G4**: no new deps; span-preserving edits (the unit_arg hoist is a
  single-span replacement re-emitting the block text verbatim — interior
  comments survive by construction, tested).

Freeze tables: 84 v2 labels / 13 exclusions + 85 v3 / 16 — **purely
additive** vs Batch 160 (82/13 + 83/16): the two new k1 labels landed and
every composite window came out byte-identical (0 reshuffle — the new atoms'
sorted positions shift nothing upstream). Bad-atom sets UNCHANGED (6/7).

## The graduation + re-arm

- `unit_arg` + `string_lit_as_bytes` flipped `covered: true` → the HeldOut
  per-family loop is 15 families deep (32 cases); the heldout marker for
  unit_arg is `({ hits.push` — the BROKEN-only shape (block inside call
  parens); the first marker attempt (`{ hits.push(1); }`) matched the HEALED
  hoist too and the solved-after clause rejected the family — the marker must
  distinguish broken from healed when the fix preserves the matched text.
- The 2 consumed frontier rows archived (13 committed rows remain).
- Re-armed: `bytes_count_to_len` (`s.len()` ⇄ `s.bytes().count()` — std's
  own definition identity) + `bool_comparison` (`x` ⇄ `x == true` — boolean
  algebra on proven-bool operands), both complexity/default-on, both
  probe-verified seed-clean/broken-exact-fire. Recorded spares:
  `manual_string_new`, `needless_late_init` (the twice-used-binding dodge).

## Counts

corpus 78 → 80, FIXABLE 59 → 61, atoms 59 → 61, scope 76 lints, lib 1941/0 ·
latent 2094/0 · frontier_miner+score_bench 2258/0, clippy clean ×4 states
(4 structural fixes in the new modules: 2 doc_markdown backticks, a
cloned-ref-to-slice, a single-match if-let; the transient regen example also
tripped an all-targets baseline failure class — deleted before the healer
run, the B160 transient pattern).

# Bench 074 — generation 7: the overlap-edit-chain ladder (Bench 069 §What remains candidate #3)

**Status:** RECORD — gen-7 **ADMITTED** at 78.26% < v1 96.23% (the M5
non-vacuity gate), deterministic ×3, freeze 15/15 with 0 standing exclusions.
Plan: [`.plans/096_generation7_overlap_edit_chain_ladder.md`](../.plans/096_generation7_overlap_edit_chain_ladder.md).
This is the LAST unexecuted difficulty axis from Bench 069's ranked list.

## What the generation is

Generations 2/3/5 compose disjoint atoms (the fixpoint converges in one
pass); generation 6 mined organic interference; **generation 7 manufactures
maximal overlap** — composites whose proposers' edits overlap, exercising the
fixpoint's drop-and-refire semantics (`apply_edits_traced` is outer-first: an
edit starting inside an already-selected region is dropped and its lint
re-fires on the next pass), including one spec engineered past
`MAX_FIX_PASSES = 4`. The ladder dimension is CHAIN DEPTH (the expected pass
count), not composition width: k ∈ {1..5} where k=5 is the capped rung (the
depth the chain would need; the cap stops it at 4).

Four territories: **flat** (the disjoint control — same lints, no nesting),
**chain** (overlap, expected 2..=4 passes), **capped** (depth exceeds the
cap), **refused** (a guard refusal under overlap pressure — the nested
`needless_range_loop` `__elem` collision).

## Freeze (T5)

15/15 verified at the bench instrument, **0 standing exclusions**, 0
relabels, drift ×2 green, 2.0 s (15 single-snippet oracle runs, release).
The first freeze run excluded 2 atoms — both probe-gap REPAIRS (the T1 probe
validated parse only; the compile fence is the freeze's own instrument):
E0308 return-type on `d2-nrl-semi`, E0425 cross-arm binding on
`d2-sme-distsq`. Full record in Plan 096 §T1 addendum + §freeze-fence
amendments, including the **created-chain subtype**: on `gen7-d2-nrl-format`
the outer rewrite CREATES the inner lint's firing site (an index expression
cannot be inlined by `uninlined_format_args`; the rewrite-created bare ident
`__elem` is exactly what it fires on), so the pre-heal label is the outer
member only and the chain still measures depth 2.

## The bench (T6)

`cargo heal --score-bench --generation 7`, release, rows #57–#59
(`score_history_v7.svg`), byte-identical ×3:

| metric | value |
|---|---|
| heal_rate | **78.26%** (23 resolved / 38 vocab-before) — < v1 96.23% → ADMITTED |
| created_rate | 13.0% (3 created — see Finding 3) |
| parse_safety | 93.3% (1 spec heals to non-compiling output — see Finding 1) |
| decline_correctness | 100% (vacuous — no clean fixtures, the gens-3/4 convention) |
| edits_total | 37 |

Per-depth (k = expected passes): depth 1 4/5 = 80% · depth 2 11/13 = 84.6% ·
**depth 3 1/1 = 100% · depth 4 1/1 = 100%** · depth 5 1/3 = 33.3%.
Per-territory: **flat 4/4 = 100% · chain 13/15 = 86.7% · capped 1/3 = 33.3% ·
refused 0/1 = 0%**. Passes-to-quiet: 1 pass ×3 · 2 ×8 · 3 ×1 · 4 ×3 —
matches the pre-registration exactly.

The depth gradient is CONFIRMED: the drop-and-refire semantics fully absorb
overlap at depth 2–4 (100% at depths 3–4 — every chain that fits under the
cap resolves); difficulty lives ENTIRELY past the cap and in the refusal —
exactly the axis Bench 069 hypothesized ("forcing multi-pass convergence past
MAX_FIX_PASSES = 4").

## Findings (the bench mining real bugs — all pre-existing, none caused by gen-7)

1. **The `needless_range_loop` rewrite produces NON-COMPILING output when a
   slot sits in a comparison context** (`gen7-d2-nrl-semi`, parse_safety
   93.3% + its 2 members persist). Mechanism: the rewrite substitutes
   `hits[i]` → the fresh binding `__elem: &u32`; `n += __elem` compiles
   (std ships `AddAssign<&u32> for u32`) but `if __elem > 2` is E0308 —
   `PartialOrd` for `&u32` compares against `&u32` only, so the literal
   needs a deref. The corpus atom only ever covered `total += v[i]`
   (where `Add` exists), so the class had NO fixture coverage until
   gen-7 manufactured it. Production note: `--fix --verify`'s compile gate
   auto-reverts this edit (the verify tier exists for exactly this); the
   bench measures the proposers UNVERIFIED by design, which is why it
   surfaces here as a hard measurement. **Post-mining intake**: a
   context-aware slot guard (refuse slots under comparison/binop contexts
   that lose the deref) or a `*__elem` splice for value-context slots.
2. **The `__elem` collision guard turns nested `needless_range_loop` into a
   hard persist** (`gen7-ref-nrl-nrl`, refused 0%): level 1's rewrite
   introduces `__elem` into level 2's body and level 2's
   `body_mentions_prefix` guard then refuses — 1 pass, the inner loop
   persists. A fresh-binding uniquing scheme (`__elem2`) is the fix-space;
   recorded as intake, not fixed here (Plan 096 non-goal).
3. **The fix cascade EXPOSES deeper lints** (created_rate 13.0%, 3 created —
   all OUTSIDE the fixable vocabulary, so they cannot be re-healed):
   `gen7-d2-mm-identity` resolves to `x.map(|v| v)` which fires clippy's
   `map_identity` + `single_option_map` (nursery); `gen7-d2-sme-distsq`'s
   distance-sq fix `(... ) < r * r` fires `suboptimal_flops` (pedantic,
   suggests `powi(2)`). The created axis is doing its job: a resolving chain
   can leave a NEW warning the vocabulary has no fix for. Recorded as
   intake context; no action in this unit.
4. **The capped territory behaves exactly as engineered** (`gen7-cap-cif6`:
   4 merges consume the cap, the innermost collapsible pair persists;
   `gen7-cap-cif5-identity`: the merges resolve, the identity member needs
   pass 5 and persists) — the pass cap is a REAL difficulty boundary, now
   measured on both sides of it (d4-cif5 converges AT 4 passes = 100%).

## What remains

- **The difficulty-axis list is now fully executed** (Bench 069 #1b → gen-5,
  #2 → gen-6, #3 → gen-7; #4 → gen-4). The real-G8 conversion A/B stays
  data-gated (`.heal/misses/` empty). Saturation telemetry on gen-7
  activates at 6 rows (currently 3).
- The three findings above are post-mining intake (the mining-queue
  snapshot §Post-mining intake carries them); fixing any re-runs this bench
  and moves the gen-7 series.
- The score-bench series can keep absorbing harder generations; the
  candidate list is empty until a new axis is argued (a proposal, not a
  plan).

## Non-goals held

No proposer work (the cap, the collision guard, and the comparison-slot bug
are measured AS SHIPPED); no v1 re-baseline; no new lint/rule corpus content.

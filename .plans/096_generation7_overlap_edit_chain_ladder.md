# Plan 096 — generation 7: the overlap-edit-chain ladder (Bench 069 candidate #3)

Status: **IN FLIGHT** — design pre-registered before implementation (plan-first
discipline). Bench 069 §"What actually remains" candidate #3, the last
unexecuted difficulty axis: *"composites whose proposers' edits overlap (the
fixpoint's drop-and-refire semantics), forcing multi-pass convergence past
`MAX_FIX_PASSES = 4`."* Generations 2/3/5 compose disjoint atoms; generation 6
mined organic interference; **generation 7 manufactures maximal overlap** (the
gen-6 non-goal #2, explicitly deferred to a separate generation).

## Pre-registered design (written before the first probe/freeze run)

1. **A new generation, NOT a v1 re-baseline** (the gen-4/5 rule). v1 fixtures,
   tables, rates: untouched. The admission gate stays the shared M5 gate
   (first gen-7 row < v1's latest), the cross-domain non-vacuity anchor.

2. **The chain mechanism** (pre-registered from the documented semantics, to
   be MEASURED not cited): `apply_edits_traced` selection is outer-first — an
   edit starting inside an already-selected region is DROPPED and its lint
   re-fires on the next fixpoint pass (`edit_emitter.rs` header +
   needless_range_loop.rs §"Atomicity (Issue 032)", the canonical documented
   case: identity_op on `hits[i] * 1` nested in a `needless_range_loop`).
   Therefore: a spec is a **chain** iff `heal_fixpoint`'s pass count ≥ 2; the
   chain **depth** = the pass count. A construct whose nesting forces more
   whole-construct merges than `MAX_FIX_PASSES = 4` is **capped**: passes ==
   4 and the deepest shapes persist (measured by the oracle at bench time).
   Two additional honest territories fall out of the same mechanics:
   - **flat** (the control): the same lints composed DISJOINTLY (separate
     fns, no nesting) — expected passes == 1. Proves the pass count is caused
     by the overlap, not by the lint count.
   - **refused**: a guard refusal under overlap pressure — the pre-registered
     case is NESTED `needless_range_loop`: level 1's rewrite introduces the
     fresh binding `__elem` into level 2's body, and level 2's
     `body_mentions_prefix` collision guard then refuses — persist without a
     second pass. Measures a documented limitation's bench impact as shipped.

3. **The ladder dimension is DEPTH, not composition width.** `spec.k` = the
   pre-registered expected pass depth (the per-k table becomes the depth
   ladder; k=5 is the capped rung — the depth the chain WOULD need). Id
   prefixes encode the claim; the freeze adjudicates:

   | id prefix | territory | expected passes | k |
   |---|---|---|---|
   | `gen7-flat-…` | flat | 1 | 1 |
   | `gen7-d2-…` | chain | 2 | 2 |
   | `gen7-d3-…` | chain | 3 | 3 |
   | `gen7-d4-…` | chain | 4 (converges AT the cap) | 4 |
   | `gen7-cap-…` | capped | 4 (the cap; deepest shape persists) | 5 |
   | `gen7-ref-…` | refused | 1 (the guard refuses; persist) | 1 |

4. **The oracle**: real clippy (the v1/v6 instrument — `run_bench`, the
   production heal loop, per-distinct-lint resolution). All chain members are
   clippy-vocabulary lints so every member is scored. The ONE cross-domain
   spec (`gen7-d2-sme-vwc`, a rust_perf rule nested inside a clippy rewrite)
   carries `expect_lints = [single_match_else]` only — the vwc member is
   invisible to the clippy oracle (the `vocabulary()` = `FIXABLE_LINTS`
   constraint gen-5 already hit); its chain behavior is still measured by the
   pass count, and the spec is documented as cross-domain evidence, not a
   scored pair.

5. **Atom roster** (15 specs, 8 lint classes — CLAIMS adjudicated by the T1
   probe and the freeze; dead atoms are reshaped or excluded LOUDLY, the
   gen-3/gen-6 discipline):
   - flat: `gen7-flat-nrl-identity` (nrl fn + identity fn), `gen7-flat-cif-smx`
     (collapsible_if fn + single_match_else fn).
   - chain d2: `gen7-d2-nrl-identity` (the Issue-032 canonical),
     `gen7-d2-nrl-format` (uninlined_format_args in the loop body),
     `gen7-d2-nrl-return` (needless_return in the loop body),
     `gen7-d2-smx-identity` (single_match_else arm), `gen7-d2-mm-identity`
     (manual_map arm), `gen7-d2-qm-identity` (question_mark, match form —
     PROBE-ADJUDICATED: the if-form's rewrite does not cover the tail), 
     `gen7-d2-inl-identity` (iter_next_loop body),
     `gen7-d2-sme-vwc` (cross-domain), `gen7-d2-cif3` (3-deep collapsible_if,
     2 merges).
   - chain d3/d4: `gen7-d3-cif4` (4-deep, 3 merges), `gen7-d4-cif5` (5-deep,
     4 merges — converges exactly AT the cap).
   - capped: `gen7-cap-cif6` (6-deep, 5 merges needed — collapsible_if
     persists), `gen7-cap-cif5-identity` (5-deep cif + identity in the
     innermost body: the 4 merges consume the cap, identity_op persists).
   - refused: `gen7-ref-nrl-nrl` (nested needless_range_loop, the collision
     refusal).
   - **The collapsible_if depth family rests on ONE pre-registered
     assumption**: the proposer merges ONE pair per pass (the outer edit
     contains the inner pair's span → inner dropped → refire). If the probe
     shows a single-edit whole-nest merge (1 pass), the cif family collapses
     and the depth ladder PIVOTS — candidate backup: `iter_next_loop` nesting
     (if its fresh-binding prefix differs from `__elem`) or another
     whole-construct outer whose inner construct survives the rewrite. The
     pivot is recorded here BEFORE the freeze lands (the gen-6 §1a shape).

6. **Freeze gates** — the gen-5/6 SPLIT (always-on cargo-less checks vs the
   `#[ignore]`d clippy e2e):
   - Always-on (module tests): G1 parse fence per atom; **G3 CHAIN-truth** —
     `heal_fixpoint` over every spec measures passes == the id's expected
     depth (flat/ref == 1, chain == 2..=4, capped == 4); curation floors
     (≥ 8 chain+cap specs, ≥ 2 flat, ≥ 1 refused, ≥ 5 distinct lint classes);
     id uniqueness across ALL SEVEN generations.
   - Freeze e2e (`tests/generation_v7_freeze.rs --ignored`, real clippy):
     **G2 label fence** (oracle hits ∩ vocabulary == the frozen label, both
     directions) + the drift gate (identical re-run). NO solve gate (the
     gens 2–6 pre-registration: solve behavior is the bench's measurement;
     the M5 admission gate carries non-vacuity).

7. **Scoring**: the PRODUCTION v1 runner verbatim — `run_bench` heals each
   spec through the bounded fixpoint, real clippy before/after decides
   resolution, `heal_rate` = Σ resolved / Σ vocab-before. Gen-7 tables
   (reported, never asserted): per-depth resolution (the k ladder), per-
   territory resolution, and the passes distribution per spec. `capped`/`refused`
   persist lands in the persist map naturally — that IS the cap's and the
   guard's measured cost.

8. **Admission prediction** (falsifiable, pre-registered): 23 scored members,
   3 persisting by construction (cap-cif6: 1, cap-cif5-identity: 1 of 2,
   ref-nrl-nrl: 1) → predicted heal_rate ≈ 87% < v1 96.2% → **ADMITTED**.
   If the capped/refused territories fail to persist (premise falsified at
   probe), gen-7 reads ~100% and the admission gate REFUSES — recorded as the
   gen-2 outcome honestly; the depth table remains the measurement.

9. **Metric honesty**: resolution is per-DISTINCT-lint (the v1 semantics);
   spec 14's cif member resolving while its identity member persists scores
   1/2 — correct. Chain depth is a WHOLE-SPEC property (nested rewrites
   serialize by overlap, not per member); the per-depth table is therefore
   per-spec, reported beside the per-member rate.

## Tasks

- [x] T0 — This plan committed BEFORE any implementation (plan-first
  discipline).
- [x] T1 — The probe: run `heal_fixpoint` over every candidate atom (scratch
  harness, deleted after), record passes + refire behavior + persist; amend
  the roster here (addendum) BEFORE the freeze lands. The cif one-pair-per-
  pass assumption is the primary probe question.
- [ ] T2 — `src/score_bench/generations.rs` gen-7 section: `RAW_ATOMS_V7`
  (the authored table), `specs_v7()`, `v7_territory()`, `v7_expected_passes()`,
  `FROZEN_LABELS_V7`, `EXCLUDED_IDS_V7`, `v7_freeze_status()`; always-on
  tests (G1 parse, G3 chain-truth via `heal_fixpoint`, floors, id uniqueness
  across generations 1–7).
- [ ] T3 — `tests/generation_v7_freeze.rs` (the `#[ignore]`d e2e: G2 label
  fence + drift, paste-ready record) + `Cargo.toml` `[[test]]` row with
  `required-features = ["score_bench"]` (the 713-T3 reader axis).
- [ ] T4 — CLI: `--generation 7` arm (the v6 clippy-runner path), parse range
  1..=7, help text, `print_v7_chain_tables` (per-depth + per-territory +
  passes distribution), `HISTORY_SVG_V7`, `finish_score_bench` arms (svg_path
  7, the per-generation SVG loop, the history line).
- [ ] T5 — Freeze run → land the tables → drift re-run (dual-mode loud).
- [ ] T6 — The bench: `--score-bench --generation 7` ×3 (determinism) →
  `score_history_v7.svg` → the M5 admission verdict recorded either way.
- [ ] T7 — `.benchmarks/074_generation7_overlap_chain_ladder.md` (highwater
  73→74) + AGENTS.md Issue-072 row update + Bench 069-073 §What-remains
  pointers + the mining-queue snapshot refresh.
- [ ] T8 — Full validation: lib default + score_bench union counts; freeze
  drift ×2; bench deterministic ×3; clippy zero findings on every touched
  line; CLI surfaces re-verified (`--generation 7` admission, `--generation 8`
  → validation error exit 2, the v2 refusal byte-identical through the shared
  gate, `--score-report` re-render).

## T1 addendum — the probe verdict (measured 2026-09-06, before the freeze)

The probe ran `heal_fixpoint` over every candidate atom and adjudicated the
roster by MEASUREMENT. Confirmed chains: d2-nrl-identity (the Issue-032
canonical, 2 passes), d2-nrl-format (2), d2-nrl-semi (2), d2-smx-identity
(2, after reshape), d2-mm-identity (2), d2-nrl-dblparen (2, added),
d2-sme-distsq (2, the cross-domain replacement), and the collapsible_if
depth ladder EXACTLY as pre-registered: cif3 → 2, cif4 → 3, cif5 → 4
(converges AT the cap), cif6 → 4 with the innermost pair persisting (capped),
cif5+identity → 4 with `n * 1` unfixed (capped). ref-nrl-nrl → 1 pass with
the inner loop collision-refused (`inner[j]` untouched, nrl persists) — the
refused territory confirmed. Flat controls → 1 pass, both members applied
(controls confirmed).

Amendments (each a measured finding, recorded before the freeze):
1. **`question_mark` has no match-form support** — on
   `match x { None => return None, Some(v) => Some(v * 1) }` the identity
   member fixed while the match stood UNTOUCHED (passes=1): the proposer only
   handles the `if x.is_none() { return None; }` form, whose rewrite does not
   cover the tail. The qm atom is DROPPED (a qm chain requires match-form
   support — proposer-extension candidate, not this unit).
2. **`iter_next_loop`'s edit is HEADER-ONLY, not whole-loop atomic** — on
   `for x in it.next() { s += x * 1; }` both edits applied in pass 1 (the
   iterable-span edit does not cover the body). No chain is constructible on
   it; the atom is DROPPED. Probe finding: not every loop rewrite is atomic.
3. **`single_match_else` requires a STATEMENT-block second arm** — `_ => { 0 }`
   is silent (expression-only block), `_ => { let fallback = 2; fallback }`
   fires; mirrors the Issue-027 real-clippy probe. The three smx atoms were
   RESHAPED to the corpus's multi-statement block form.
4. **NEW MINED FINDING — the capacity-hint nested-block blind spot**:
   `vec-with-capacity` and `hashmap-with-capacity` fire on the STANDALONE
   `fn build(n: usize) { let mut v = Vec::new(); for i in 0..n { v.push(..); } v }`
   shape but are SILENT when the identical shape sits inside an `if let`/
   match-arm block (edits=0 on the post-smx text). The capacity-source
   derivation appears fn-body-level. The planned `d2-sme-vwc` atom is
   replaced by `d2-sme-distsq` (distance-sq is expression-level and fires
   inside the arm); the blind spot itself is a post-mining intake candidate
   (the miss side, not this unit's to fix).
5. The cross-domain chain CONFIRMED via distance-sq: smx whole-match rewrite
   pass 1, the inner expression-level perf-rule edit refires pass 2. Scored
   member: single_match_else (the clippy oracle cannot see rust_perf rules).

## Freeze-fence amendments (the compile fence + label fence at work, 2026-09-06)

The first freeze run verified 13/15 and EXCLUDED 2 — every exclusion a
probe-gap finding, recorded before the record landed:

- **`gen7-d2-nrl-semi` failed the G1 compile fence** (E0308): `n` sums u32
  slice elements but the fn returned `usize`. The T1 probe validated PARSE
  (`heal_fixpoint` is syn-based); the compile fence is the freeze's own
  instrument. Repaired (return `u32`), not excluded.
- **`gen7-d2-sme-distsq` failed the G1 compile fence** (E0425): the `_` arm's
  `eps < r` referenced `r`, which the OTHER arm's pattern binds. Repaired
  (`eps < 1.0`), not excluded.
- **`gen7-d2-nrl-format` label fence: `uninlined_format_args` did not fire** —
  `println!("{}", names[i])` carries an INDEX expression, which the lint
  cannot inline; the firing site only EXISTS after pass 1's rewrite creates
  the bare ident `__elem`. A **created-chain subtype**: the outer edit
  manufactures the inner lint's firing site. Label relanded as
  `[needless_range_loop]`; the chain depth is still 2 (probe + freeze
  both measure it) and the bench's created axis records nothing (the
  created lint is resolved in the same run).

Final frozen roster: 15/15 (13 first-pass verified + 2 repaired), zero
standing exclusions.

Final roster: 15 specs — 2 flat, 9 chain (7× d2 + cif3/d3-cif4/d4-cif5),
2 capped, 1 refused; 8 scored lint classes. Predicted scored members 24,
persisting 3 (cap-cif6 1, cap-cif5-identity 1 of 2, ref-nrl-nrl 1) →
predicted heal_rate 21/24 = 87.5% < v1 96.2% → ADMITTED predicted.

## Non-goals

- No proposer work: the pass cap, the drop-and-refire semantics, and the
  `__elem` collision guard are measured AS SHIPPED. A cap raise or a
  multi-fresh-binding scheme is a separate unit INFORMED by this measurement
  (the persist map is its input, not this unit's to fix).
- No changes to v1/v2/v3/v4/v5/v6 fixture sets, tables, or rates.
- No new lint/rule corpus content; no memchr/dep-gated rules (not vocabulary).
- No `--fix-compile` (gen-4) or mechanical-detector (gen-5) oracle — the chain
  members are clippy lints with external existence; the v1 oracle is the
  authority.

# Plan 094 — generation 5: the rust_perf vocabulary ladder (Bench 070 candidate #1b)

**Status:** IN FLIGHT — design pre-registered before the first freeze run.

**Source:** [Bench 070](../.benchmarks/070_generation3_persist_stratified_ladder.md)
§What remains candidate #1b + [Bench 071](../.benchmarks/071_generation4_rustc_error_ladder.md)
§What remains. The candidate's own words: "vocabulary extension to rust_perf
rules … the rust_perf divergence classes (array-literal vec-with-capacity,
dep-gated proposers) are open frontier but changing `vocabulary()`
re-baselines v1 — its own plan."

## Pre-registered design (written before the first freeze run)

1. **A new generation, NOT a v1 re-baseline.** The candidate's literal
   re-baseline wording predates generation 4. Mutating `vocabulary()` would
   corrupt the v1 series in place: the 96.2% anchor feeds the M5 admission
   gate for every later generation, the saturation detector reads the v1
   series, and the SVG curve would render the re-baseline row as a
   catastrophic regression. Generation 4's precedent is strictly better and
   already landed: a SEPARATE generation with its own oracle, sharing only
   the admission gate as a CROSS-DOMAIN non-vacuity anchor. The candidate's
   VALUE — measuring the rust_perf persist frontier inside the frozen bench
   — is preserved exactly. v1 fixtures, tables, and rates: UNTOUCHED.

2. **The oracle (the load-bearing decision).** rust_perf rules have NO
   external detector — clippy does not ship them; that absence is why the
   domain exists. The gen-5 oracle is therefore a set of per-rule MECHANICAL
   SHAPE DETECTORS owned by the bench (a pure-textual module that never
   calls the proposers), one per FIXABLE rule:

   | rule | polarity | persist predicate (on the member region) |
   |---|---|---|
   | vec-with-capacity | removal | contains `Vec::new()` |
   | entry-vs-contains-insert | removal | contains `contains_key(` |
   | static-str | removal | contains `".to_string(` |
   | bool-to-bitpack | removal | contains `.filter(|&&` |
   | fxhashmap | removal | contains `std::collections::HashMap` |
   | hashset-insert-bool-return | removal (compound) | contains `.contains(&` AND `.insert(` |
   | collect-append-to-extend | removal | contains `.append(&mut` |
   | vec-macro-zero-fill | removal | contains `.resize(` |
   | distance-sq-no-sqrt | removal | contains `.sqrt()` |
   | hashmap-with-capacity | removal | contains `HashMap::new()` |
   | shrink-to-fit-after-build | compound | contains `Vec::new()` AND `.push(` AND NOT `shrink_to_fit` |

   Honest caveats, pre-registered:
   - The detectors are written from the rules' documented shape contracts —
     the same contracts the distill process probe-verified against the rule
     sources. They cannot be independent of the corpus's own rule
     definitions, because the rules have no external existence. This is the
     domain-shaped limit of "an oracle with zero relationship to our
     corpus": independence is from the HEAL LANE, not from the vocabulary.
   - They ARE tool-agnostic the way the clippy oracle is at lint level: any
     edit that removes the shape credits, whoever authored it. Resolution =
     shape absent, exactly like `lint absent after`.
   - `shrink-to-fit-after-build` is COMPOUND for a measured reason: its own
     fix is additive (keeps `Vec::new()`, inserts `shrink_to_fit()`), AND
     the mutual-exclusion contract with `vec-with-capacity`
     (shrink_to_fit_after_build.rs §"The exact-capacity complement") means
     the vwc `with_capacity(n)` fix ALSO resolves the over-allocation
     hazard (exact capacity ⇒ no excess to release). A token-addition
     detector would FALSELY mark vwc-healed regions unresolved; the
     compound shape predicate credits either honest resolution.

3. **Atom table** — 14 atoms → 14 + 7 + 4 + 3 = 28 specs:
   - **10 corpus atoms** (territory **addressable**): per FIXABLE rule
     except fxhashmap, the perf-seed-corpus entry (the factory takes the
     min `buggy_code` per rule under the parse-standalone filter; the
     corpus carries exactly one entry per rule id, sorted).
   - **4 persist atoms** (territory **persist**), each an authored claim
     from a documented divergence class, adjudicated by the freeze (the
     gen-3 discipline — a persist atom is a CLAIM, not a fact):
     - `persist_ref_array_capacity` (vec-with-capacity): the
       reference-array-literal iterable `for x in &[..] { v.push(*x) }` —
       the temp-binding rewrite binds the ARRAY and would change iteration
       mode, so the guard keeps it silent (test-pinned
       `array_literal_reference_iterable_skipped`). Bench 070's
       "array-literal" class survives as this REFERENCE form: the direct
       `[..]` form was SOLVED by the 2026-08-28 temp-binding extension.
     - `persist_coldstart_fxhashmap` (fxhashmap): std HashMap with NO
       `rustc_hash` import anywhere — the proposer's Gate 0 refuses (a
       span-preserving splice can rewrite uses but cannot add the
       rustc-hash dep to Cargo.toml; the game-sdk E0432 smoke). Persists in
       PRODUCTION, not only in the bench. Shape uses a bare-path iterable
       so hashmap-with-capacity's bare-path guard keeps the atom's surface
       single-class.
     - `persist_caller_gate_str` (static-str): a `-> String { "lit".
       to_string() }` fn WITH a same-file call site — the signature-changing
       rewrite is refused when the fn ident occurs more than once
       (`count_ident_occurrences > 1` → refuse; the action_hoare E0308
       lesson). The single-definition corpus shape heals; the called shape
       persists.
     - the corpus `fxhashmap` atom (territory persist **by context**): the
       bench stub crate has no rustc-hash dep, and `heal_fixpoint`'s
       explicit skip mirrors the production dep gate for exactly that
       context (mod.rs's documented divergence).
   - Persist atoms FIRST in the atom list (the gen-3 convention — stride
     windows cover the list head, so a persist shape sits in every k≥2
     rung), then corpus atoms in sorted rule order. Order is load-bearing,
     pinned by test.

4. **Freeze gates** (cargo-LESS — the whole freeze is pure textual; the e2e
   costs milliseconds, unlike gens 2-4's per-spec cargo spawns):
   - **G1 parse gate**: every atom's `buggy_code` parses standalone.
   - **G2 marker-discrimination fence** (per corpus atom): the persist
     predicate is TRUE on `buggy_code` and FALSE on `fixed_code`. A marker
     that fails discrimination is a FALSIFIED claim — the atom is excluded
     (or reshaped), never pinned (the gen-3 lesson: the freeze adjudicates).
   - **G3 stratification gate** (per persist atom): `heal_fixpoint` over the
     atom leaves the predicate TRUE (resolves 0). This is the class claim —
     NOT a solve gate on addressable atoms (pre-registered: solve behavior
     is the bench's measurement).
   - **G4 composite-validity fence** (per spec, k≥2): the composed file
     parses, and each member's persist predicate is TRUE in its own
     pre-heal region (composition must not destroy or shadow a member's
     shape). Violations are EXCLUDED, never pinned.
   - **G5 drift gate**: landed `FROZEN_LABELS_V5` == fresh derivation on
     re-run.
   - **NO solve gate on addressable atoms, NO k-decay gate** — the gens
     2/3/4 pre-registration (per-k tables are REPORTED; the M5 admission
     gate carries non-vacuity).
   Region extraction is syn-span based (the mod's items' byte range — the
   proposers' own `span().byte_range()` pattern), not brace matching, so
   braces inside string literals cannot mis-attribute.

5. **Scoring** (per-member, region-attributed; the shared row schema):
   each spec heals through the production `heal_fixpoint` loop (both
   proposer domains, the fxhashmap skip, MAX_FIX_PASSES=4), the healed file
   re-parses, and member i's verdict reads module `gen_{i}`'s region.
   `heal_rate` = Σ resolved / Σ members (blended headline — admission's
   input); `created` = removal-polarity markers newly present in a member
   region that were absent before (the compound shrink predicate is
   excluded from the created axis — an inserted `shrink_to_fit` is a fix,
   not a created problem); `parse_safety` = 1 − parse-unsafe/ran (a
   non-parsing healed file scores all members unreduced, never clean);
   `decline_correctness` = 1.0 (no clean fixtures in the ladder); persist
   map = unresolved-rule → member count (the `--misses` feed). Per-k and
   per-territory tables REPORTED side by side, never blended into the
   headline decision.

6. **Admission gate UNCHANGED** (v1-latest anchor) — the cross-domain
   non-vacuity floor, the gen-4 convention. Projection (pre-registered, to
   be MEASURED not cited): territory-A 10 atoms solve (each is the rule's
   own fixture-pinned shape), territory-B 4 persist 0/4 → blended ≈
   10/14 = 71.4% < v1 96.2% → ADMITTED. Honest cross-fire expectations
   (pre-registered): the shrink proposer fires on `persist_ref_array_
   capacity` (its module doc names exactly this fallback regime) and
   hashmap-with-capacity's bare-path guard keeps
   `persist_coldstart_fxhashmap` single-class — neither flips a persist
   label.

7. **Scope**: generation 5 covers the FIXABLE vocabulary (11 rules). The
   90 suggestion-only rules are NOT vocabulary (no fix path exists —
   scoring them would measure a permanent zero, which is not a difficulty
   axis). memchr (`byte-find-position-to-memchr`) is dep-gated AND not in
   FIXABLE_RULES — not vocabulary.

## Tasks

- [ ] T0 — This plan committed BEFORE any implementation (plan-first
  discipline).
- [ ] T1 — `src/score_bench/generations.rs` gen-5 section:
  `PERSIST_ATOMS_V5`, `perf_atoms()` (the corpus factory),
  `v5_atoms()`/`specs_v5()`/`v5_member_rules()`, `v5_territory()`,
  `FROZEN_LABELS_V5`/`EXCLUDED_IDS_V5`, `v5_freeze_status()`, the
  detector table + pure predicate helpers, the syn-based
  `module_region()` extractor. cfg-gating: none beyond `score_bench`
  (rust_perf is a default feature AND score_bench implies it).
- [ ] T2 — always-on tests in the generations tests module: atom-table
  determinism + full-FIXABLE-vocabulary coverage (every rule exactly one
  atom or persist row); ladder shape (28 specs, k counts, id uniqueness
  across ALL five generations); both territories present at every k≥2
  rung; marker discrimination buggy-vs-fixed per corpus atom (the G2
  fence as an always-on pin — pure textual, no cargo); persist-claim
  stratification via `heal_fixpoint` (the G3 fence, also always-on);
  `v5_freeze_status` holes-are-loud (dual-mode, mirror v4).
- [ ] T3 — `src/score_bench/report.rs`: `HISTORY_SVG_V5`.
  `src/bin/heal.rs`: `--generation` range 1..=5 + help text; the gen-5
  dispatch arm BEFORE the oracle-availability check (cargo-less runner);
  `run_score_bench_perf()` (per-spec measurement + per-k/per-territory
  tables + BenchScore + the shared `admission_gate`/`finish_score_bench`
  tail); the finish tail's SVG loop `[2,3,4]` → `[2,3,4,5]`, the
  `svg_path` arm, the history line's gen-5 count.
- [ ] T4 — `tests/generation_v5_freeze.rs`: the `#[ignore]`d freeze e2e
  (cargo-less; runs G1–G5, prints the paste-ready label/exclusion tables,
  lands the record) + the always-on table-properties test.
  `Cargo.toml` `[[test]]` row with `required-features = ["score_bench"]`
  (the 713-T3 reader axis; rust_perf rides score_bench).
- [ ] T5 — Freeze run → land the tables → re-run (drift green, dual-mode
  flips). Adjudicate any falsified persist claim per the gen-3 discipline
  (reshape or drop + record, never pin).
- [ ] T6 — The bench: `--score-bench --generation 5` → admission verdict →
  rows appended (determinism ×2) → `score_history_v5.svg` → per-k +
  per-territory tables into the record.
- [ ] T7 — `.benchmarks/072_generation5_rust_perf_ladder.md` (highwater
  71→72) + Bench 070/071 §What remains pointers + AGENTS.md Issue-072 row
  update.
- [ ] T8 — Full validation: lib default + union counts, clippy zero on
  every touched line, the CLI surfaces re-verified (gen-5 admission ×2,
  `--generation 6` validation error, v2 refusal still byte-identical
  through the shared gate, `--score-report` re-render).

## Non-goals

- No proposer work: the persist classes are measured AS SHIPPED; the miss
  side is the persist map's input, not this unit's to fix.
- No changes to v1/v2/v3/v4 fixture sets, tables, or rates; v1 is NOT
  re-baselined (design decision 1).
- No perf-bench (validator) changes; no memchr/rule-corpus changes.
- No behavior tests (cargo) on healed outputs — the heal loop's own gates
  (syn parse) plus the oracle contracts carry the claim; compilation of
  healed atoms is the heal lane's existing verify-tier concern, not this
  bench's axis.

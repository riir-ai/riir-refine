# Bench 072 — generation 5: the rust_perf vocabulary ladder (Bench 070 §What remains candidate #1b)

**Status:** RECORD — generation 5 frozen (28/28 verified, zero exclusions, zero
drift; freeze 0.05 s — the first CARGO-LESS freeze in the family), **ADMITTED**
(gen-5 69.2% < v1 96.2%), deterministic ×3. The bench's fourth live series and the
first measurement of the rust_perf persist frontier.

**Box:** 4090 / Windows, CPU-only, no GPU anywhere.
**Freeze command:**
`cargo test --release --features score_bench --test generation_v5_freeze -- --ignored --test-threads=1 --nocapture`
**Bench command:**
`cargo run --release --features score_bench --bin cargo-heal -- --score-bench --generation 5`

## The design (pre-registered in `.plans/094` before the first freeze run)

Bench 070's ranked reopen list, candidate #1b: "vocabulary extension to rust_perf
rules … the rust_perf divergence classes (array-literal vec-with-capacity,
dep-gated proposers) are open frontier". Two pre-registered divergences from the
candidate's literal wording, each with a reason:

- **A new generation, NOT a v1 re-baseline.** Mutating `vocabulary()` would
  corrupt the v1 series in place (the 96.2% anchor feeds the M5 admission gate
  for every generation, the saturation detector reads the v1 series, and the
  re-baseline row would render as a catastrophic regression). Gen 4's precedent:
  a separate generation with its own oracle, sharing only the admission gate as a
  CROSS-DOMAIN non-vacuity anchor. v1 fixtures/tables/rates: untouched.
- **The oracle is a bench-owned set of per-rule MECHANICAL SHAPE DETECTORS**
  (11 pure-textual predicates, Plan 094 §2). rust_perf rules have NO external
  detector — clippy does not ship them; that absence is why the domain exists.
  The honest caveats, pre-registered: the detectors are written from the rules'
  documented shape contracts (the same contracts the distill process
  probe-verified), so they cannot be independent of the corpus's own rule
  definitions — independence here is from the HEAL LANE, not from the vocabulary.
  They ARE tool-agnostic at the shape level: any edit that removes the shape
  credits. One rule is COMPOUND polarity (`shrink-to-fit-after-build`): its own
  fix is additive, AND the mutual-exclusion contract with `vec-with-capacity`
  (the exact-capacity complement) means a `with_capacity(n)` fix also resolves
  the over-allocation hazard — a token-addition detector would falsely mark
  vwc-healed regions unresolved.

## The freeze: 28/28 verified, zero exclusions, in 0.05 s

| Measure | Value |
|---|---|
| Atoms | 14 (3 authored persist + 11 corpus; 10 addressable + 4 persist territory) |
| Specs | 28 (14 k=1 + 7 k=2 + 4 k=3 + 3 k=4) |
| Verified | **28/28** — marker-discrimination fence (buggy TRUE / fixed FALSE) + composite-validity fence |
| Exclusions | **0** — zero composition interference measured |
| Stratification gate | **PASS** — every persist atom resolves 0 through the production `heal_fixpoint` loop |
| Re-freeze drift | PASS (identical re-run through the landed `FROZEN_LABELS_V5`) |
| Freeze cost | **0.05 s** — the whole freeze is pure textual + syn; gens 2-4 spawned a real oracle per spec (minutes) |

The persist atoms are CLAIMS from documented divergence classes, adjudicated by
the freeze (the gen-3 discipline) — all three held at HEAD:

| persist atom | rule | why the heal stays silent |
|---|---|---|
| `persist_ref_array_capacity` | vec-with-capacity | `for x in &[..]` — the P2 temp-binding rewrite binds the ARRAY and would change iteration mode; the direct `[..]` form was SOLVED 2026-08-28, the REFERENCE form survives (test-pinned `array_literal_reference_iterable_skipped`) |
| `persist_coldstart_fxhashmap` | fxhashmap | Gate 0: no `rustc_hash` import anywhere — a span-preserving splice can rewrite uses but cannot add the Cargo.toml dep (the game-sdk E0432 smoke); persists in PRODUCTION |
| `persist_caller_gate_str` | static-str | the signature-changing rewrite refuses when the fn ident occurs >1× in the file (the action_hoare E0308 lesson; a live call site would break) |
| corpus `fxhashmap` atom | fxhashmap | persist BY CONTEXT — the bench stub has no rustc-hash dep; `heal_fixpoint`'s skip mirrors the production dep gate for exactly that context |

## The bench: ADMITTED at 69.2%, deterministic ×3

| Measure | Value |
|---|---|
| heal_rate | **69.2%** (36/52 members resolved) — < v1 96.2% → ADMITTED |
| created_rate | 0.0% (no removal-marker newly appeared in any member region) |
| parse_safety | 100% (every healed file re-parses) |
| Per-k (reported, not asserted) | k=1 **71.4%** (10/14) · k=2 71.4% (10/14) · k=3 66.7% (8/12) · k=4 66.7% (8/12) |
| Per-territory | **addressable 100.0% (36/36)** · **persist 0.0% (0/16)** |
| Persist map | fxhashmap ×8, static-str ×4, vec-with-capacity ×4 (each persist shape in every composite containing it) |
| Determinism | runs #50/#51/#52 identical (rates, edits, per-k, per-territory; only timestamps differ) |
| Rows | #50-#52 appended (`generation: 5`), `score_history_v5.svg` rendered |

**The territory split is the headline, and it matches the projection exactly:**
the 10 addressable corpus atoms (the rules' own fixture-pinned shapes) resolve
10/10 at every composite width — 36/36 members across all rungs — and the 4
persist atoms resolve 0/4 at every rung (16/16 members). k-composition costs
nothing: the same shapes solve (or persist) at every k. The blended 69.2% is a
DESIGN floor — the persist classes are documented parse-only divergences; no
planned work moves them except proposer extensions (a reference-literal
temp-binding variant, a Gate-0 dep-adoption path, a caller-aware
signature rewrite), and those are the persist map's input, not this bench's.

## En-route find: the falsified projection (the freeze-vs-bench discipline)

The plan pre-registered that `persist_coldstart_fxhashmap`'s bare-path iterable
would keep hashmap-with-capacity's bare-path guard between the atom and a
capacity cross-heal. **Falsified at the first bench run**: the atom took 1 edit —
`HashMap::with_capacity(pairs.len())`. The VWC bare-path guard
(`bare_path_iterable_unchanged`) is VWC-only; `hashmap_with_capacity` is
deliberately PERMISSIVE for param-bound bare-path roots ("params/outer-scope
bindings stay permissive — the `--verify` compile gate backstops them",
hashmap_with_capacity.rs L109-110). The persist VERDICT was unaffected and was
measured, not assumed: the fully-qualified return type keeps
`std::collections::HashMap` in the region after the capacity fix, so the
fxhashmap shape persists (stratification gate PASS, bench persist 0/16). The
cross-heal is honest and the marker design demonstrably does not false-credit it
— the atom stays as the recorded instance. Full addendum in the plan.

## Structural notes

- `run_score_bench_perf` is cfg(`score_bench`) only — `score_bench` implies
  `rust_perf` (Cargo.toml) and `rust_perf` is default-on; no extra feature gate.
  It dispatches BEFORE the clippy-oracle availability check (cargo-less).
- The M5 admission gate + append/SVG/saturation tail run through the SHARED
  `admission_gate`/`finish_score_bench` implementation (the gen-4 extraction);
  the SVG loop now covers `[2,3,4,5]` and the history line reports gen-5.
- `tests/generation_v5_freeze.rs` carries `required-features = ["score_bench"]`
  (the 713-T3 reader axis; rust_perf rides score_bench). The freeze gates G2/G3
  run as ALWAYS-ON module tests (pure textual, no cargo) — the drift gate runs
  in the e2e.
- Region extraction is syn-span based (the mod's items' byte range — the
  proposers' own `span().byte_range()` pattern), not brace matching, so braces
  inside string literals cannot mis-attribute.

## What remains

- **Saturation telemetry on gen-5** activates at 6 rows (currently 3).
- **The persist frontier is now MEASURED and pinned**: three classes × every
  composite width. A proposer extension that closes one class (reference-literal
  temp-binding for vec-with-capacity; a dep-adoption path for fxhashmap; a
  caller-aware rewrite for static-str) MOVES the gen-5 rate and the ratchet
  sees it — the same property gen 3 bought for the clippy lane.
- **Next difficulty axes** (Bench 070 §What remains, unchanged): real-repo
  interference spans (#2); overlap-edit chains (#3). The real-G8 conversion A/B
  stays data-gated (`.heal/misses/` empty).

## Validation

- lib default (run below) + lib union (score_bench) green; clippy zero findings
  on every touched line (the pre-existing `composite.rs` needless-not /
  `evolve.rs` needless-borrow / `heal.rs` manual-checked-division warnings are
  outside this unit's hunks — noted, not absorbed).
- CLI surfaces live-verified: gen-5 admission (×3 runs), `--generation 6`
  validation error, v2 refusal byte-identical through the shared gate,
  `--score-report` re-render.
- Files: `src/score_bench/generations.rs` (gen-5 section + frozen tables +
  always-on tests), `src/score_bench/report.rs` (HISTORY_SVG_V5),
  `src/bin/heal.rs` (gen-5 dispatch + `run_score_bench_perf` + the finish-tail
  gen-5 wiring), `tests/generation_v5_freeze.rs` (new), `Cargo.toml` (the
  [[test]] row), `.benchmarks/score_history.jsonl` (+3 gen-5 rows),
  `.benchmarks/score_history_v5.svg` (new).

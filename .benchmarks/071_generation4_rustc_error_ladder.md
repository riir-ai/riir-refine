# Bench 071 — generation 4: the rustc-errors-side k-ladder (Bench 069 reopen #4)

**Status:** RECORD — generation 4 frozen (76/76 verified labels, zero exclusions, zero
drift on re-freeze), **ADMITTED** (gen-4 54.3% < v1 96.2%), deterministic ×2. The
bench's third live series and its first COMPILE-domain measurement.

**Box:** 4090 / Windows, CPU-only (the oracle spawns cargo per spec; no GPU anywhere).
**Freeze command:**
`cargo test --release --features score_bench,rustc_errors --test generation_v4_freeze -- --ignored --test-threads=1`
**Bench command:**
`cargo run --release --features score_bench,rustc_errors --bin cargo-heal -- --score-bench --generation 4`

## The design (pre-registered in `.plans/093` before the first freeze run)

Bench 069's ranked reopen list, candidate #4 — the salvage of the removed duplicate
Plan 091's T2/T3: compose `corpus_gen` cases at k ∈ {1..4}, measure the
`--fix-compile` driver per k. A different oracle (rustc coded-error reduction, exact
by construction) and a different repair domain than the clippy bench. Key
pre-registered decisions, each a divergence from the salvaged text with a reason:

- **Labels = member error codes** in the existing `(spec_id, sorted set)` frozen
  shape — the whole freeze engine (`freeze_status_for`/`generation_fixtures`/`k_of`)
  ran UNCHANGED; only the runner is new.
- **Atom factory**: stride-40 slice of `generate_corpus()` (family-proportional;
  the corpus is family-ordered), round-robin interleaved by expected code before
  windowing so composites mix codes.
- **Scoring = per-member span-attributed code reduction**, NOT the salvaged T3.1's
  textual gold-region comparison: the corpus's own module doc records
  semantically-equal-but-textually-diverged playbook fixes, so text-identity against
  seed would misclassify fixed members as unreduced.
- **No solve-based freeze gate, no k-decay gate** — same pre-registration as gen 3,
  plus gen 2's branch-B lesson. Freeze = label fence only (one check per spec);
  solve behavior is the bench's measurement.
- **Admission gate UNCHANGED** (v1-latest anchor) — recorded as a CROSS-DOMAIN
  non-vacuity floor, not a same-domain difficulty claim.
- **Territory honesty**: per-territory rates reported side by side, never blended.

## The freeze: 76/76 verified, zero exclusions

| Measure | Value |
|---|---|
| Specs | 76 (40 k=1 + 16 k=2 + 12 k=3 + 8 k=4) over 40 atoms |
| Verified labels | **76/76** — composed codes ≡ union of member codes, count == k |
| Exclusions | **0** — zero composition interference measured |
| Re-freeze drift | PASS (identical re-run through the landed `FROZEN_LABELS_V4`) |
| Families | 8 codes: E0373 ×20, E0382 ×16, E0499 ×18, E0502 ×20, E0505 ×16, E0507 ×12, E0596 ×16, E0597 ×21 (spec appearances) |

**Instrument fix caught by the fence's first run (the gate IS the test):** every spec
initially reported its code TWICE — `cargo check --all-targets` compiles the lib
twice (normal + the `--test` harness profile), duplicating every diagnostic. Fix:
`coded_lines` dedupes exact `(code, line)` pairs (the driver's own
`diag::dedupe_by_site` artifact class; distinct members sit at distinct lines, so
composite multisets are preserved). NOT a composition finding — repaired the
instrument, then decided the disposition.

## The bench: ADMITTED at 54.3%, deterministic ×2

| Measure | Value |
|---|---|
| heal_rate | **54.3%** (116/208 members reduced) — < v1 96.2% → ADMITTED |
| created_rate | 0.0% (no member's fix introduced a new code) |
| parse_safety | 100% (no parse-unsafe final states) |
| Per-k (reported, not asserted) | k=1 **60.0%** (24/40) · k=2 50.0% (16/32) · k=3 55.6% (20/36) · k=4 50.0% (16/32) |
| Per-territory | **uncontested 100.0% (60/60)** · **contested 20.0% (16/80)** |
| Persist map | E0373 ×20, E0382 ×16, E0507 ×12, E0596 ×16 — the delegate-only families |
| Determinism | run #49 == run #48 exactly (rates, edits, persist map) |
| Row | #48 + #49 appended (`generation: 4`), `score_history_v4.svg` rendered |

**The territory split is the headline, and it is the product measured as shipped:**
uncontested members (E0597/E0499/E0502 — the templates' home turf) reduce 100/100 at
every composite width; contested members reduce only where a template exists
(E0505CloneAtMove, 16/16) — E0596/E0382/E0507/E0373 ship NO template by policy (the
floor measured divergences; cargo fix owns them) and account for all 64 unreduced
members. k-composition itself costs little: k=1 → k=2 drops 10pp and then plateaus —
multi-error diagnosis under the min-delta loop is nearly free for covered codes, and
the uncovered codes are structural misses at every k.

**Read the gen-4 rate honestly:** 54.3% is a DESIGN floor (the delegate-only
families), not a frontier — like gen 3, no planned work moves it. The floor moves
only if a template family ships for one of the four delegate-only codes, and that is
the floor harness's measured-divergence gate, not this bench's. The persist rows now
feed `--misses` with the exact code pool an authoring decision would draw from.

## En-route find: the scoring inversion (caught by the bench's first live run)

The first bench run measured **45.7% with an impossible territory table**
(uncontested 0/60, contested 64/80) — physically incoherent on its face (the driver
cannot fix template-less codes). Root cause in `composite::finalize_attribution`:
the per-member reduction predicate was INVERTED (`any()` on still-present
diagnostics counted as reduced, an empty final code set counted as all-unproven) —
the module's own doc stated the correct predicate. The instrument was repaired, the
**garbage row was removed from the JSONL** (a broken-instrument measurement is not
evidence — the Bench 649/833 class), and the corrected run became the gen-4 series'
first row. Debug probes on a template-covered and a template-less case confirmed
the driver's behavior was correct both ways before the fix landed.

**Lesson:** the freeze label fence could not catch a SCORING-semantics defect (it
verifies labels, not reductions); the always-on pure-helper tests covered the
attribution helper but not the reduction predicate. The first live bench run is
itself a gate — an incoherent table (a code reducing without a template) is a
finding, and "repair the instrument first, decide the disposition second" applies
to a suspicious-green as much as to a suspicious-red.

## Structural notes

- **The M5 admission gate + the append/SVG/saturation tail are now ONE shared
  implementation** (`admission_gate` + `finish_score_bench` in `src/bin/heal.rs`),
  exercised by both runners. The v2 refusal path reproduces byte-identically
  through the extraction (exit 1, same message, JSONL length unchanged), and
  `--score-report` re-renders unchanged. The history line now reports
  `gen-4 {n}` for every generation.
- The gen-4 section of `generations.rs` is cfg-gated on `rustc_errors`
  (`score_bench` does not imply it; a score_bench-only build stays green and the
  CLI arm errors cleanly).
- `tests/generation_v4_freeze.rs` carries `required-features = ["score_bench",
  "rustc_errors"]` (the 713-T3 reader axis; the `#![cfg]` protects the count).

## What remains

- **Saturation telemetry on gen-4** activates at 6 rows (currently 2).
- **The next difficulty axes** (Bench 070 §What remains): vocabulary
  extension to rust_perf rules (candidate #1b — **EXECUTED 2026-09-06 as
  generation 5, ADMITTED at 69.2%:
  [`.benchmarks/072_generation5_rust_perf_ladder.md`](072_generation5_rust_perf_ladder.md)**);
  real-repo interference spans (#2); overlap-edit chains (#3). The real-G8
  conversion A/B stays data-gated (`.heal/misses/` empty).
- **A template family for a delegate-only code** would move the gen-4 floor —
  gated on the floor harness's divergence measurement, not scheduled here.

## Validation

- lib default 1129/0; lib union (score_bench, rustc_errors) 1313/0/8i;
  `generation_v4_freeze` always-on 1/0/1i + the freeze e2e green twice
  (deterministic, zero drift).
- clippy zero findings on every touched line (`--lib --all-targets` union state).
- CLI surfaces live-verified: gen-4 admission ×2 (deterministic), v2 refusal
  byte-identical through the shared gate, `--generation 5` validation error,
  `--score-report` re-render.
- Files: `src/domains/rustc_errors/composite.rs` (new — regions/scaffold/label
  fence/measurement + the dedupe + the inversion fix), `src/domains/rustc_errors/mod.rs`
  (wiring), `src/score_bench/generations.rs` (v4 section + frozen tables),
  `src/score_bench/report.rs` (HISTORY_SVG_V4), `src/bin/heal.rs` (gen-4 arm +
  `run_score_bench_compile` + the shared `admission_gate`/`finish_score_bench`),
  `tests/generation_v4_freeze.rs` (new), `Cargo.toml` (the [[test]] row),
  `.benchmarks/score_history.jsonl` (+2 gen-4 rows; the garbage pre-fix row removed),
  `.benchmarks/score_history_v4.svg` (new).

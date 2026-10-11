# Plan 093 — generation 4: the rustc-errors-side ladder (Bench 069 reopen #4)

**Status:** RECORD — landed 2026-09-06; gen-4 frozen 76/76 (zero exclusions, zero
drift), ADMITTED at 54.3% (< v1 96.2%), deterministic ×2.
[Bench 071](../.benchmarks/071_generation4_rustc_error_ladder.md) is the record.
Headline finding: the scoring inversion caught by the bench's first live run
(uncontested 0/60 was physically incoherent — repair the instrument first).

**Source:** [Bench 069](../.benchmarks/069_issue072_t5_t6_generations.md) reopen candidate #4 +
[Bench 070](../.benchmarks/070_generation3_persist_stratified_ladder.md) §What remains.
The design descends from the salvage of the removed duplicate Plan 091 (`git show
3d43c90d^:.plans/091_issue072_t5_t6_composite_and_generationed_bench.md`, their T2/T3)
— composed `corpus_gen` cases at k ∈ {1..4} measured through the `--fix-compile`
driver: a different oracle (rustc coded-error reduction, exact by construction) and a
different repair domain than the clippy bench.

## Pre-registered design (written before the first freeze run)

1. **Atom factory** — `v4_atoms()`: a deterministic stride slice of
   `corpus_gen::generate_corpus()` (TARGET 40 atoms; stride = n/TARGET) mapped into
   the generic `V3Atom` shape `{ id: case.name, expect: vec![expect_code], code:
   case.broken }`. The `broken` half is the ladder's subject (the driver repairs, the
   seed is the target state — never frozen as an oracle label).
2. **Composite windows** — the atom list is round-robin interleaved by expected code
   (sorted-code groups, name order within group) BEFORE `specs_over_atoms("gen4", …)`
   windows it, so k≥2 composites mix codes (name order alone would family-cluster).
   Ladder sizes reuse `count_for_k` unchanged: 40 + 16 + 12 + 8 = 76 specs.
3. **Label shape REUSED, not forked** — a gen-4 label is the member error-code set;
   `(spec_id, sorted codes)` is exactly the existing `(id, &[&str])` frozen-record
   shape, so `freeze_status_for` / `generation_fixtures` / `k_of` run UNCHANGED. The
   per-member ordered code list is derived from the spec id (`gen4-k{k}-{names
   joined +}` → name→code map over `v4_atoms()`).
4. **Freeze gates** (the rustc analog of the clippy label fence):
   - **Label fence**: each non-excluded spec's composed `broken` file produces, on one
     real `cargo check`, EXACTLY the union of member codes (set equality) with count
     == k (composition validity — two defects must not merge, shadow, or cascade into
     a different code). Violations are EXCLUDED (the interference class), never pinned.
   - **Drift gate**: landed `FROZEN_LABELS_V4` entries must equal fresh labels on
     re-freeze (panic on drift — the v3 discipline).
   - **Structure gates** (always-on, arithmetic): every expected code appears in ≥1
     k≥2 composite; both territories (`territory_of`) present at every k≥2 rung;
     determinism + id uniqueness + ladder shape.
   - **NO solve-based gate at freeze, NO k-decay gate** — same pre-registration as
     gen 3 (per-k tables are REPORTED; the admission gate carries non-vacuity), now
     also justified by gen 2's branch-B lesson (corpus-trained shapes do not decay).
   The freeze e2e therefore runs ONE check per spec (no driver write loops) —
   minutes, not tens of minutes. Solve behavior is the BENCH's measurement.
5. **Scoring = per-member span-attributed code reduction** (diverges from the
   salvaged T3.1's "textual gold-region comparison", for a measured reason: the
   corpus's own module doc records that some playbook fixes are
   semantically-equal-but-TEXTUALLY-diverged from seed, so text-identity against seed
   would misclassify fixed members as unreduced). Per spec: initial check →
   `CompileDriver::isolated()` write loop with `Playbooks::production()` (max_iters 5,
   the product default) → final check → each coded diagnostic attributes to the
   `mod gen_{i}` region containing its primary line. Member i is REDUCED iff its
   expected code is absent from its region at the final check. Created = non-initial
   codes attributed inside member regions (+ unattributed non-initial codes,
   reported separately). Parse guard: a final check that fails with ZERO coded
   diagnostics means the file no longer parses — the spec counts parse-unsafe and all
   members unreduced (never a false clean).
6. **Metrics mapping** (the row schema is shared): heal_rate = Σ reduced / Σ members;
   created_rate = Σ created / Σ members; parse_safety = 1 − parse_unsafe/ran;
   decline_correctness = 1.0 (the ladder has no clean fixtures); persist = per-code
   unreduced-member counts (feeds `--misses`).
7. **Admission gate UNCHANGED** — the first gen-4 row must measure strictly below
   v1's latest heal_rate. Honest caveat, pre-registered: this is a CROSS-DOMAIN
   anchor — the gate is a non-vacuity floor (the new series must demonstrate
   hardness against the incumbent), not a same-domain difficulty claim. v1 floors
   stay pinned either way.
8. **Territory honesty** — `Playbooks::production()` covers E0597/E0499/E0502/E0505;
   E0596/E0382/E0507/E0373 are delegate-only (no template — the floor measured
   divergences, cargo fix owns them). Those members are STRUCTURAL driver misses and
   are expected to dominate the contested stratum's unreduced set; the bench reports
   per-territory rates side by side, never blended (the corpus's own stratification
   rule), and their persist rows feed the authoring queue.

## Tasks

- [x] T0 — This plan committed BEFORE any implementation (plan-first discipline).
- [x] T1 — `src/domains/rustc_errors/composite.rs` (new, rustc_errors-domain):
  region extraction (`module_regions`, brace-matched, format-independent), scaffold
  helper, `check_composite_labels` (freeze entry), `measure_composite` (bench entry),
  attribution + guards; unit tests for the pure parts (no cargo). En-route: the
  `(code, line)` dedupe for the `--all-targets` double-compile artifact (caught by
  the freeze's first run) + the reduction-predicate inversion fix (caught by the
  bench's first run).
- [x] T2 — `src/score_bench/generations.rs`: `GEN4_ATOM_TARGET`, `v4_atoms()`
  (stride slice + code interleave), `specs_v4()`, `v4_member_codes(id)`,
  `FROZEN_LABELS_V4`/`EXCLUDED_IDS_V4` (landed post-freeze), `v4_freeze_status()`;
  the v4 section cfg-gated on `rustc_errors` (score_bench does not imply it);
  always-on tests mirror the v3 set (dual-mode holes test, determinism, ladder shape,
  family/territory coverage, id uniqueness across all three generations).
- [x] T3 — `src/score_bench/report.rs`: `HISTORY_SVG_V4`. `src/bin/heal.rs`:
  `--generation` range 1..=4 + help text; the gen-4 dispatch arm (freeze gate →
  compile bench); `run_score_bench_compile` (aggregate CompositeOutcomes →
  BenchScore, per-k + per-territory printing, per-spec progress lines); the M5
  admission block EXTRACTED into one helper shared by both paths (byte-identical
  messages — v2 refusal re-verified through it); SVG loop `[2, 3, 4]` + the history
  line's gen counts.
- [x] T4 — `tests/generation_v4_freeze.rs`: the `#[ignore]`d label-fence e2e
  (one real check per spec; cascade exclusions; drift gate; per-k/per-family/per-territory
  label tables; paste-ready literals) + the always-on table-properties test.
  `Cargo.toml` `[[test]]` row with `required-features = ["score_bench",
  "rustc_errors"]` (the 713-T3 reader axis).
- [x] T5 — Freeze run → land the tables → re-run (drift green, dual-mode flips).
- [x] T6 — The bench: `--score-bench --generation 4` → admission verdict → row
  appended → `score_history_v4.svg` → per-k + per-territory tables into the record.
  En-route: the first run's garbage row (45.7%, broken scoring instrument) was
  REMOVED from the JSONL; the corrected run (#48) + its determinism re-run (#49)
  are the landed series.
- [x] T7 — `.benchmarks/071_generation4_rustc_error_ladder.md` (highwater 70→71) +
  Bench 070 pointer + AGENTS.md Issue-072 row update.
- [x] T8 — Full validation: lib default 1129/0 + union 1313/0/8i, clippy zero on
  every touched line, freeze e2e ×2 (drift), the v1/v2/v3 CLI surfaces re-verified
  (v2 refusal byte-identical through the shared gate; `--generation 5` validation;
  `--score-report` re-render).

## Non-goals

- No playbook/template work for the delegate-only codes (the driver is measured as
  shipped; the miss side is the persist map's input, not this unit's to fix).
- No behavior-test (`cargo test`) pass per composite — code reduction is the metric;
  semantics are the templates' own G1-verified property (recorded as a caveat in the
  bench doc, not silently assumed).
- No change to v1/v2/v3 fixture sets, tables, or rates.

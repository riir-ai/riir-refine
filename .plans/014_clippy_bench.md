# Plan 014 — clippy-bench: frozen real-code healing bench + SVG score history

## Motivation

The repo has no number that can falsify "we're getting better". What exists:

- **Retrieval precision** (Bench 022→027→032): real improvement (clippy top-1
  58.3%→89.6%), but measured on authored fixtures coupled to the corpus; the
  kernel "100/100/100" rows are self-variant retrieval (the corpus retrieving
  renamed copies of itself — circular).
- **Corpus rule count** (10→40 lints, 46→72 perf, →355 kernel): a coverage
  metric, not a competence metric. Growth cannot not look like progress.
- **Bench 009's 0/10 held-out**: measures the L1 verbatim cache tier (by
  design a cache), not the proposer path the production sweeps ran.
- **The sweep campaign** (thousands of validated edits across the workspace):
  the real generalization evidence, but episodic per-slice validation — no
  unified score, no time series, no frozen denominator.

Owner ask (2026-08-19): a training-curve-style graph ("like training have")
for a lint/perf/format healing bench — see improvement per iteration, and a
train/test-style split so corpus growth cannot inflate the score.

## Design

**clippy-bench v1 — lint axis only** (perf/format axes are Phase 2).

- **Fixture = real code span** (std-only fn/impl chunk) mined from PRE-SWEEP
  commits of sibling repos, FROZEN into `src/score_bench/fixtures.rs` with
  provenance (`repo@sha:path`). Ground truth = what real `cargo clippy`
  (all+pedantic+nursery) fires on the span embedded in a bare stub crate —
  the T3.5.5 `ClippyOracle`, which has zero relationship to our corpus.
- **Frozen denominator**: the bench set is committed + versioned. Corpus
  growth never moves it. The score can only move by healing better.
- **Drift check**: each run re-derives ground truth with the CURRENT
  toolchain; disagreement with the frozen `expect_lints` prints a loud
  warning (clippy's lint set moves between releases — same caveat as the
  oracle's, which is why this is a measurement harness, not a pinned floor).
- **Heal pass** = the SAME bounded-fixpoint loop as `cargo heal --fix`
  (MAX_FIX_PASSES=4, perf + clippy lanes, `apply_edits_traced`), minus the
  `fxhashmap` rule (the stub crate has no rustc-hash dep — mirroring the
  production dep gate for a crate without it).
- **Scoring** (per lint-NAME presence, oracle dedupes):
  - `heal_rate` = Σ resolved-vocab-lints / Σ vocab-lints-before (the up axis)
  - `created_total` = Σ newly-appearing clippy lints after healing, ANY lint
    (the false-positive axis — Issue 027/028/029 classes land here)
  - `parse_safety` = healed-text-parses / fixtures-edited
  - `decline_correctness` = clean fixtures (0 vocab lints) left untouched by
    the clippy lane / clean fixtures (declining is a correct answer — the
    L4 relevance lesson applied to the modelless lanes)
- **History**: append-only `.benchmarks/score_history.jsonl` (one record per
  run: ts, git sha, per-axis scores, edits/vocab sizes) + zero-dep SVG line
  chart `.benchmarks/score_history.svg` (4 series, no crates — hand-rolled
  string building). README embeds the SVG.

## Honest scope notes (v1)

- **Std-only spans**: fixtures must compile in a bare stub crate, so crate-
  internal spans are excluded by construction (the oracle's `compiles` flag
  is the filter). Per-fixture stub manifests are Phase 3.
- **Sweep-history provenance**: the pre-sweep states are code the development
  process has seen (proposers were hardened against these classes). v1
  anchors the curve + guards regressions; TRUE unseen-source fixtures
  (crates.io excerpts) are Phase 3. Provenance is recorded per fixture.
- Expect a STEP curve (root-cause fixes), not a gradient-descent curve —
  the learning mechanisms measured negative and were killed; improvement is
  engineering-iterative. That is the truthful shape.

## Tasks

### Phase 1 — harness
- [x] P1.1 `score_bench` feature (`clippy_verify` + `latent_retrieval`) + lib wiring
- [x] P1.2 `src/score_bench/mod.rs` — fixture model, oracle runner, heal
      loop (mirror of heal.rs's), scoring math + unit tests
- [x] P1.3 `src/score_bench/report.rs` — JSONL history + SVG renderer + tests
- [x] P1.4 CLI: `cargo heal --score-bench` (run + append + render) and
      `--score-report` (render only)
- [ ] P1.5 `#[ignore]`d e2e smoke (first N fixtures, real clippy) — deferred
      `[-]`: the CLI run IS the e2e (ran for the baseline); a pinned test
      would re-run the same oracle path for no additional coverage

### Phase 1.5 — frozen v1 fixture set
- [x] P1.6 `examples/score_bench_mine.rs` — git-show a sibling file at a
      pre-sweep sha, extract std-only item spans, oracle-label, emit fixture
      entries (provenance + expected lints) + `--freeze-composite` mode
- [x] P1.7 Mine + curate the v1 set: 34 fixtures = 27 authored-composite
      (T3.5.5 realistic multi-lint, oracle-labeled) + 1 real healable
      (map_unwrap_or, pre-sweep katgpt-rs) + 6 real clean (decline axis).
      HONEST FINDING: the bare-stub compile filter drops nearly all dirty
      real fns (crate-internal types) — real self-contained dirty fns are
      structurally rare; growing the `real:` fraction is Phase 3
      (statement-wrap mining + stub manifests + crates.io sources).
      Lint classes covered: needless_return, redundant_field_names,
      identity_op, double_parens, useless_format, unused_unit,
      map_unwrap_or, needless_range_loop, uninlined_format_args, + more.
- [x] P1.8 Fixture-integrity tests (unique ids, provenance, parses, ≥1 clean,
      provenance-split report)

### Phase 1.9 — baseline + docs
- [x] Ran the bench once; committed baseline JSONL + SVG; README section
      embedding the SVG + run commands; `.docs/08_benchmarks/clippy_bench.md`
      + the docs index row.
      **Baseline (run #1): heal 84.6% · created 3 (5.8%) · safety 92.9% ·
      decline 100% — and the run caught a REAL proposer bug** (Issue 032:
      needless_range_loop × identity_op composition breaks the loop var;
      the 2 safety failures are exactly those fixtures).
- [x] **Issue 032 RESOLVED same day (fix `86ba7f1`)** — the range-loop
      rewrite is now ONE atomic whole-loop edit (the header + every slot
      substitution must land together; as separate edits a foreign
      same-pass fix overlapping one slot was dropped by outer-first
      selection while the headers survived → dangling `i`). **Run #2:
      heal 90.4% (+5.8pp) · safety 100%** — the first upward step on the
      curve, exactly as the bench was designed to show. 3 new tests
      (atomicity + both frozen-fixture compositions through the production
      fixpoint shape); 19/19 module, 623 default / 707 score_bench lib.
- [x] **Run #3 — the run-#2 miss map triaged to root causes (fix
      `c8d0d34`, same day): heal 98.1% · created 1 (1.9%)** — 5 proposer
      extensions, each probe-verified against real clippy 1.97 BEFORE
      implementation (needless_return bare `return;`; double_parens
      call-arg single wrap; redundant_pattern_matching 2-arm bool match;
      needless_range_loop macro-arg descent — format-family args parse as
      exprs, `{i}` captures refuse, opaque macros mentioning `i` refuse,
      `__elem` collision check walks macro tokens; useless_format
      single-positional-arg for non-ident args) + static-str `const fn`
      emission (closing both `missing_const_for_fn` creations) + a latent
      static-str trait-impl soundness fix (trait signatures must match —
      the old walker fired where the rewrite can never compile). Two
      residuals documented as boundaries: `single_option_map` (nursery,
      clippy's own manual_map fix produces the same cascade) + one
      String-typed `useless_format` persist (the type-aware terminal fix
      is beyond parse-only reach; firing would trade the persist for an
      `implicit_clone` created — bench-symmetric, and `created` is the
      trust axis). Per-fixture triage ships as the `#[ignore]`d
      `per_fixture_diagnostics` test (the standing workflow). 649 default
      / 733 score_bench / 712 fix_verify / 701 latent_retrieval lib green;
      clippy 0 all states.

### GOAT (the harness itself)
- [x] G1 measures what it claims: scores derive ONLY from real-clippy
      verdicts before/after (no proposer-oracle contamination)
- [x] G2 full run < 10 min on a warm cargo cache (measured ~2 min; the
      BLAKE3-keyed oracle crates make re-runs fast)
- [x] G3 no-regression: default + score_bench feature states green, clippy 0
      (620 default / 704 score_bench lib tests; clippy 0 in score_bench files
      both states — the 5 default-state lib-test warnings are pre-existing)
- [x] G4 history append-only; SVG regenerates deterministically from the
      JSONL (render is a pure function of the records)

### Phase 2 (deferred) — perf + format axes
Perf-axis fixtures from mined anti-pattern sites + the existing Pareto
validator; format axis = heal-must-not-break-rustfmt sub-check.

### Phase 3 (deferred) — generalization tiers
- [ ] External crates.io fixture source (truly unseen distribution)
- [ ] Per-fixture stub manifests (crate-internal spans)
- [ ] Cold vs warm memory lines (`RIIR_HEAL_MEMORY` override)
- [ ] Holdout tier: fixtures frozen + never used for proposer tuning

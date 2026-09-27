# Bench 103 — generation v9: the CRB corruption corpus + localization metrics + DLM remask arming gate (Issue 137 T1–T6)

**Date:** 2026-09-28 · **Machine:** M3 Max (Apple M3 Max, 16 cores), AC power, macOS 26.6.2 ·
**Build:** `--release`, `CARGO_TARGET_DIR=/tmp/crb_build` (isolated; sibling agents active on
the box — none contending for this lane's cargo) · **Commits:** `c2faf248` (T1–T4+T6) ·
`5912e144` (T5)

## What this bench is

The generation-v9 instrument (Issue 137, Research 215 — arXiv:2512.15596 "Corrective
Diffusion Language Models", CRB instrument): a compile-clean / test-failing corruption
corpus with KNOWN error positions, localization metrics with loud-zero laws, the DLM
remask arming gate, and the healer span-ranking POC. Own runner
(`cargo heal --score-bench --generation 9`); no clippy oracle; never appended to the
heal score history.

## The corpus (T1/T3)

- **8 seed programs** (range_gates, fib_pairs, slice_search, rolling_stats, bool_gates,
  char_count, grid_steps, edge_counts): compile-clean, test-passing, for-loops only,
  asymmetric tests. The seed gate (run before any corruption) caught **two real
  authoring bugs** pre-run — `fib_sum(5)` asserted 7 (true value 12) and
  `bracketed("ab", 'a')` asserted true (true value false) — the gate works.
- **Mutation classes:** operator table `+↔-`, `*↔/`, `<↔<=`, `==↔!=`, `&&↔||`;
  same-type integer literals (value XOR 1, suffix preserved); identifier-from-same-scope
  (all uses of a same-type parameter swapped, shadow-guarded). `while`-condition sites
  are excluded by construction (the hang-safety law). Test modules are never corrupted
  (the grader is trusted).
- **Two-tier filter** (real cargo, pid-scoped probe crate, 30 s wall cap):
  `cargo check` must stay CLEAN ∧ `cargo test` must FAIL. Signal-killed (abort /
  stack overflow) and timed-out candidates are REJECTED — only a clean non-zero test
  verdict counts. Landing run: 240 draws → **48 admitted** (16 per |E| ∈ {1,2,4}),
  3 test-passed (rejected: not broken), 48 text dups, 141 skipped by the
  per-(seed, |E|) cap of 2. 45.5 s wall.
- **Verify mode re-executed every committed fixture**: 48/48 still check-clean ∧
  test-fail at landing (36.4 s). The corpus is pinned by a blake3 digest over
  ids+code+spans+classes; hand-edits red `v9_freeze_status` (append-refusal law,
  always-on `corpus_v9_pins_hold`).
- **Floors:** 10/10/10 per |E| rung (≈62% of the achieved 16s), firing DOWNWARD only.
- **Control arm (compile-FAILING corruption):** wired but **empty by construction for
  the v1 type-preserving engine** — every swap is type-safe by design, so no candidate
  can fail `cargo check` on these seeds. Disclosed in the pin test and the panel; the
  bucket + verify arm stay live for future non-type-preserving engines.

## Localization metrics + live control laws (T2)

Confidence polarity (high = likely correct — the d2f/Plan-414 native polarity); the
flagged set is the LOW-confidence tail; ranking ties break by byte position.

| scorer | gap | hit@1 | hit@3 | hit@5 | fpr@5 | mrr |
|---|---|---|---|---|---|---|
| oracle | +1.000 | 1.000 | 1.000 | 1.000 | 0.404 | 1.000 |
| constant(0.5) | +0.000 | 0.000 | 0.000 | 0.000 | 1.000 | 0.033 |
| shuffled(oracle) | +0.004 | 0.042 | 0.083 | 0.104 | 0.979 | 0.088 |
| inverse(oracle) | −1.000 | 0.000 | 0.000 | 0.000 | 1.000 | 0.006 |

- chance hit@1 (rank-random, hypergeometric over per-fixture token counts): **0.018**;
  shuffled reads 0.042 — inside the chance+0.15 band. The metric discriminates.
- constant reads gap **exactly 0.0** and is REFUSED (`constant_confidence`) — never
  "perfectly calibrated".
- inverse reads a regressive gap and is REFUSED (`regressive_gap`).
- oracle fpr@5 0.404 is the structural (K−|E|)/K floor talking (the corpus mixes
  |E| ∈ {1,2,4} → per-fixture floors 0.8/0.6/0.2): the FPR cap (0.85) catches pure
  noise, not the floor.
- Per-class rows (never pooled): operator n=37, literal n=23, ident n=3 — every class
  gap +1.000 / hit@1 1.000 under the oracle; `class_floor_verdict` binds min-class
  (the pooled-equals-min arm fails by unit test).
- Re-draw CI (hit@1, seeded bootstrap B=500): est 1.000 [1.000, 1.000] width 0.000 —
  degenerate because the oracle is exact; the disclosure machinery is the deliverable.

## Arming verdicts (T4)

Pre-registered `DEFAULT_ARMING_FLOORS` (min_gap 0.05, min_hit1 0.30, max_fpr5 0.85,
shuffle_margin 0.10), fixed check order, named refusals:

- oracle → **ARMED** (control sanity — if the truth fails to arm, the floors are wrong
  and the panel exits 1)
- constant → REFUSED `constant_confidence`
- shuffled(oracle) → REFUSED `no_signal_over_shuffle`
- inverse(oracle) → REFUSED `regressive_gap`

**No production confidence source is wired** — `SpanScorer` is the consumer contract.
The d2f remask confidence (katgpt-forward) / Plan 414's trained gap implement the trait
and face these floors; byte-identical when not invoked (the panel only runs under
`--generation 9`).

## The healer span-ranking POC (T5) — OUTCOME (c): infrastructure-ahead-of-consumer

Item-granularity split of every fixture (top-level items, test modules excluded), each
item scored by the healer's own `LatentMatcher` (max fused rule-similarity across
domains):

- clean items mean **87.92** vs corrupted items mean **71.14** — relative gap **+0.191**
- hit@1 (corrupted item is the fixture's lowest-similarity item): **0.438** vs
  per-fixture chance **0.323**

The ranking **separates** CRB corruption — a real contrast with the Bench-018 caveat on
`ScoredRule.score` (which measured NO separation for LINT-shaped corruption, where the
clean twin of an anti-pattern is corpus-adjacent): semantic corruption is
corpus-ATYPICAL, so similarity drops. But the healer has **no semantic-bug fixer** that
could consume a span ranking, so the pre-registered outcome is (c)
infrastructure-ahead-of-consumer; the instrument rides the generation-9 panel for the
DLM remask gate.

## Prior art

Corruption-instrument lineage (cited, never claimed): **cargo-mutants** (type-preserving
compile-clean Rust mutations — exactly the operator class our compile-clean bucket
uses), **Pydra** (synthetic clones + bugs), **BugsInPy** (real bugs), classical mutation
testing. The benchmark design (type-preserving tokenizer-consistent corruption,
execution-validated, difficulty = |E| replaced tokens, confidence-gap/Top-K metrics) is
**CRB from arXiv:2512.15596** — our delta is the healer-localization fusion, the
oracle-anchored control arm design, and the remask-arming consumer loop.

## Honest limits

- The corpus is authored-seed-based (48 fixtures over 8 programs) — small, and the
  seed programs are simpler than production Rust. Expansion (more seeds, deeper
  programs, float literals, compound-assign classes) is a corpus-growth commit with
  floor re-pins, never a silent widen.
- The control arm is structurally empty for the v1 type-preserving engine.
- hit@1 at item granularity over 3-item fixtures is a coarse instrument (chance 0.32);
  token granularity is where the DLM confidence source will actually compete.
- The `--no-default-features --features latent_retrieval` lane fails at HEAD
  (`clippy_feed.rs:462`, E0433 `domains::clippy_lints` configured out) — PRE-EXISTING,
  untouched by this work (our modules compile to nothing without `score_bench`), noted
  for the owner.

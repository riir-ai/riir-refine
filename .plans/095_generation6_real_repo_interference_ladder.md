# Plan 095 — generation 6: the real-repo interference ladder (Bench 069 reopen candidate #2)

Status: **IN FLIGHT** (design pre-registered before implementation — the Plan-094
discipline; execution updates this file in place)

## 0. What this is

Bench 069's ranked reopen list, candidate #2: **mine multi-lint spans from
sibling repos (the v1 `score_bench_mine` provenance discipline) where lint
interactions are ORGANIC** — real fns carrying 2+ distinct vocabulary lints,
not authored atoms concatenated by `mod gen_{i}` wrapping.

Generations 2/3/5 compose atoms (the corpus teaches its own shapes →
addressable ~100%); generation 4 switched repair domain. Generation 6
switches SHAPE ORIGIN: the spans come from real code the corpus never saw,
with the lint MIX the sweeps actually found. This is the first generation
whose k is not a construction parameter — k = the span's organic distinct-lint
count, and the interactions (an edit for lint A creating/removing lint B's
firing, the fixpoint's drop-and-refire semantics under real overlap) are
properties of the mined code, not of the composer.

## 1. Pre-registered design (before any mining run)

- **Mining shas**: the PARENT of each repo's first `fix(clippy): heal` sweep
  commit — every file that sweep healed is guaranteed lint-carrying at the
  parent (provenance-by-construction, the sweep diff IS the lint evidence).
  Candidate repos on this box: katgpt-rs, riir-ai, riir-train, riir-chain,
  riir-neuron-db. Final set = whichever yield ≥ the curation floor.
- **Extraction**: top-level `fn` items only (the v1 mine constraint — impl
  blocks rarely compile standalone), labeled by the REAL oracle
  (`ClippyOracle`, all+pedantic+nursery) via `lints_for_batch` (one clippy
  run per ~25-span chunk — the batch API's honesty rule: one instrument).
  Spans that fail the bare-stub compile drop (the std-only filter).
- **The interference criterion**: a span qualifies as MULTI when the oracle
  reports ≥ 2 DISTINCT vocabulary lints. Single-lint (k=1) real spans are
  kept as the CONTROL territory — they measure real-SHAPE difficulty with
  interactions removed; the multi−single delta is the interaction signal.
- **Curation floors** (pre-registered, checked by an always-on test):
  - ≥ 24 multi spans (k≥2) — below this the interaction table is anecdote.
  - 6..=12 single control spans (k=1).
  - ≥ 8 distinct vocabulary lint classes across the set; ≤ 4 fixtures per
    lint class per source repo (the v1 diversity discipline — the bench
    measures healing competence, not one lint's frequency).
  - Zero id/provenance collision with v1's real fixtures (ids are
    `gen6-<stem>-<idx>`; v1's are `<repo>-<stem>-<idx>` — disjoint by
    prefix, asserted).
- **Scoring**: the PRODUCTION v1 path — `run_bench` heals each fixture
  through the same bounded-fixpoint loop `cargo heal --fix` runs, and real
  clippy before/after decides resolution. `heal_rate` = Σ resolved
  vocabulary lints / Σ vocabulary lints before. The admission gate is the
  shared M5 gate (first gen-6 row < v1's latest), recorded as a
  cross-regime floor exactly like gen-4's.
- **Territory** (never blended into the headline): `single` (k=1, real
  shape control) vs `multi` (k≥2, the interference axis), split by the
  frozen label's distinct-lint count. Per-k table reported, not asserted
  (the gen-3 convention).
- **Freeze gates** (`tests/generation_v6_freeze.rs`, `#[ignore]`d — spawns
  real clippy; always-on gates live in module tests):
  - **G1 compile fence**: every span compiles in the bare stub crate; a
    non-compiling span is EXCLUDED (lands in `EXCLUDED_IDS_V6`), never
    scored.
  - **G2 label fence**: oracle hits ∩ vocabulary == the frozen label, per
    span. Mismatch → loud exclusion (or label correction), never a silent
    bench label. This fence is the instrument check — mining-time labels
    are claims, the freeze oracle adjudicates them (the gen-3 lesson).
  - **G3 k-truth**: spec.k == the frozen label's distinct-lint count
    (organic k is DERIVED, not authored).
  - **G4 non-vacuity**: the multi population carries ≥2 lints on ≥24
    admitted spans (the axis exists).
  - **Re-freeze drift**: identical re-run, zero label drift.
- **Prediction (falsifiable, pre-registered)**: multi < single, and single
  < v1 — organic interactions are harder than none, and real shapes are
  somewhat harder than the shapes the corpus teaches. If multi ≥ v1's rate
  the axis FAILED and the admission gate refuses (nothing appended; the
  bench doc records the verdict). If single ≥ multi the interaction
  hypothesis is refuted — report it honestly; the generation may still
  admit (it is harder overall by construction of the v1 anchor) but the
  interaction claim dies.

## 2. Metric honesty notes

- Resolution is per-LINT-INSTANCE (v1's heal_rate semantics) — the member
  count of a k-lint span is k. Per-span FULL-SOLVE rate is reported beside
  it (a span with 2/3 lints resolved is not a solved span).
- The mined distribution is expected to be dominated by
  `uninlined_format_args`+X (it was the sweeps' modal lint). The ≤4-per-lint
  cap per repo keeps the X-axis diverse; the dominance itself is REPORTED
  as a finding (the organic mix is what it is).
- G2's freeze labels are toolchain-pinned (clippy 1.98.1 today). The bench
  re-derives at run time and the fresh verdict wins (the v1 drift rule).

## 3. Tasks

- [x] **T1** Plan pre-registered (this file) + commit BEFORE implementation.
- [ ] **T2** Mining driver `examples/score_bench_mine_v6.rs` (batch oracle,
      sweep-diff file discovery, multi-lint filter, census output).
- [ ] **T3** Mine candidate repos; curate the span set to the floors; land
      `RAW_SPANS_V6` data + `specs_v6()` + territory helpers.
- [ ] **T4** Freeze e2e (`tests/generation_v6_freeze.rs`, G1-G4 + drift) →
      run → land `FROZEN_LABELS_V6` + `EXCLUDED_IDS_V6`.
- [ ] **T5** CLI: `--generation 6` arm (the v2/v3 clippy runner path),
      parse range, help text, `HISTORY_SVG_V6`, territory/per-k tables.
- [ ] **T6** Bench ×3 → rows #53-#55 + `score_history_v6.svg`; the M5
      admission gate verdict recorded either way.
- [ ] **T7** Bench 073 + AGENTS.md Issue-072 pointer update + Bench
      070/072 §What remains + mining-queue snapshot refresh; all `[x]`.
- [ ] **T8** Validation: lib default + score_bench union counts; freeze
      drift ×2; clippy clean on touched lines.

## 4. Non-goals

- NOT a v1/v2 re-baseline (the gen-4/5 rule — a new generation number,
  v1 floors pinned forever).
- No overlap-edit CHAIN construction (Bench 069 candidate #3 — separate
  generation if pursued; gen-6 measures interference as it occurs
  organically, it does not manufacture maximal overlap).
- No per-fixture stub manifests (v1's Phase-3 dream — spans still must
  compile std-only; crate-internal spans are invisible to this generation,
  recorded as the standing scope limit).

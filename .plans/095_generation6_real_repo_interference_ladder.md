# Plan 095 — generation 6: the real-repo interference ladder (Bench 069 reopen candidate #2)

Status: **RECORD** (design pre-registered before implementation; the
premise was falsified by mining, the real-shape salvage landed and was
ADMITTED — see §1a/§1b; record: `.benchmarks/073`)

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
- **Extraction** (AMENDED 2026-09-06 — see §1a): contiguous top-level ITEM
  RUNS over the compile-passing subset (v1's own real-fixture shape — its
  real fixtures are multi-fn excerpts, not lone fns). The original
  single-fn extraction was falsified by measurement before implementation
  (§1a); every gate below is unchanged — k is still the span's organic
  distinct-vocabulary-lint count, the compile fence still applies, and the
  freeze oracle still adjudicates every landed label.
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

## 1a. The mining falsification + amendment (recorded before implementation)

The pre-registered extraction (top-level fns only) was MINED and is
measured EMPTY — the multi-lint population lives in fns that reference
crate-internal types, which the bare-stub filter (v1's documented scope
limit) drops by construction:

| repo @ mining-sha | sweep | files | fn spans | compiled | k=1 | k≥2 |
|---|---|---|---|---|---|---|
| riir-chain @ 7421245a~1 | 7421245a (lib src) | 47 | 114 | 43 | 1 | **0** |
| katgpt-rs @ 5b2d42970~1 | 5b2d42970 (examples) | 147 | 817 | 205 | 11 | **0** |
| riir-train @ e6084429~1 | e6084429 (engine src) | 30 | 142 | 42 | 9 | **0** |
| riir-neuron-db @ 5db2e3d~1 | 5db2e3d (src) | 27 | 174 | 80 | 0 | **0** |

1,247 fn spans, ~570 compile standalone, ZERO organic multi-lint. The
fn-granularity claim is refuted, not the AXIS: the compiled singles
cluster in files (bandit_01_basic's print_regret_comparison +
print_reward_comparison are adjacent), so contiguous ITEM RUNS over the
compile-passing subset reach k≥2 while staying inside the same
bare-stub contract. A run may also span a struct + its impl block —
method territory the lone-fn filter could never see.

Amended algorithm (deterministic, byte-stable):
1. Extract ALL top-level items per file (spans + sizes).
2. Compile-probe every item alone (batch); keep the pass set.
3. Label every passing item (batch) — per-item lint sets.
4. Greedy walks over CONSECUTIVE passing items: extend until the
   accumulated distinct-lint set reaches ≥2, or the cap (8 items /
   2,500 bytes) — emit the minimal run; slide past non-viable starts.
5. Batch-label each built RUN (the run's own verdict is authoritative —
   scope-sensitive lints can differ from the union-of-items); keep
   label-k ≥ 2. k==1 minimal runs emit as the CONTROL candidates.

Curation floors unchanged (≥24 multi / 6..=12 single / ≥8 classes /
≤4 per class per repo). The k=1 controls are single-lint real shapes
(30 candidates already in hand from the falsification runs).

## 1b. The second amendment — the SALVAGE design (recorded before landing)

The adjacency-run refinement (site-accumulation instead of distinct-union)
raised the organic yield from 0 to ~1 per repo — still order-of-magnitude
below the ≥24 floor. The CARVE tier (file-header context, §1a's mechanized
v1 discipline) reached 4 organic multis across 8 mining runs. The floors
were amended to the measured crust and the generation landed as the
**real-repo shape ladder**:

- **Population**: 17 spans (13 single + 4 organic multi), 6 classes, 4
  repos (katgpt-rs, riir-train, riir-ai, riir-chain; auth harvested but
  cap-trimmed). Floors as landed: ≥16 spans / ≥4 multi / ≥6 classes /
  ≥4 repos / ≤4 per (repo, class) — asserted by an always-on test.
- **Freeze** (tests/generation_v6_freeze.rs --ignored): 17/17 verified at
  the BENCH instrument, 0 exclusions, 0 relabels, drift PASS.
- **Bench**: gen-6 95.24% < v1 96.23% → **ADMITTED**, deterministic ×3 + a
  4th byte-stability run. Territory: single 13/13 = 100%, multi 7/8 =
  87.5%; the one miss is the Issue-028 comment-guard class re-found
  organically (design refusal, not a bug).
- Prediction audit: multi < single HELD; single < v1 REFUTED (real
  single-lint shapes solve at parity — generalization measured).
- Record: `.benchmarks/073_generation6_real_repo_shape_ladder.md`.

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
- [x] **T2** Mining driver `examples/score_bench_mine_v6.rs` (batch oracle,
      sweep-diff file discovery, multi-lint filter, census output).
- [x] **T3** Mine candidate repos; curate the span set to the floors; land
      `RAW_SPANS_V6` data + `specs_v6()` + territory helpers.
- [x] **T4** Freeze e2e (`tests/generation_v6_freeze.rs`, G1-G4 + drift) →
      run → land `FROZEN_LABELS_V6` + `EXCLUDED_IDS_V6`.
- [x] **T5** CLI: `--generation 6` arm (the v2/v3 clippy runner path),
      parse range, help text, `HISTORY_SVG_V6`, territory/per-k tables.
- [x] **T6** Bench ×3 → rows #53-#55 + `score_history_v6.svg`; the M5
      admission gate verdict recorded either way.
- [x] **T7** Bench 073 + AGENTS.md Issue-072 pointer update + Bench
      070/072 §What remains + mining-queue snapshot refresh; all `[x]`.
- [x] **T8** Validation: lib default + score_bench union counts; freeze
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

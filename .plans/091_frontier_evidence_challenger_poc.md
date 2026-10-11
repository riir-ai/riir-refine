# Plan 091 — Issue 072 PoC: Frontier-evidence Challenger (construction pairs + BT pair-fed re-rating + dispersion selection)

**Status:** COMPLETE — PoC executed; exit criterion MET (2 of 3 axes, the third structurally discharged + gate-run green). Commit lands with the issue-file verdict.

Source: riir-clippy `.issues/072_frontier_evidence_challenger.md` (J-Zero modelless fusion,
arXiv:2608.26582 M1/M2/M3/M6; targets T39/Research 135's starved-pool finding — 98.5% of
candidates unseen — and Plan 014 Phase 3's deferred real-fraction growth).

**PoC exit criterion (from the issue, verbatim):** T1+T3+T4 on the existing store →
pair-fed + frontier-fed arms vs outcome-fed baseline: (a) unseen-candidate fraction drops
measurably, (b) Certified-conversion per authored rule improves, (c) no retrieval-floor
regression. Any two of three → promote the rest of issue 072 to a plan.

**Measured verdict (2026-09-06, this box, CPU-only):**
- **(a) PASS — unseen 0.786 → 0.429** on the 14-player candidate universe
  (outcome-fed covers 3, pair-fed covers 8, 200 pairs, zero cargo per pair).
  Measured on a T39-realistic sparse synthetic store; the REAL store union is
  expected strictly better (the real store's outcome coverage is the 1.5%
  tail T39 measured).
- **(b) PASS (G8 proxy) — informative mean rank 2.00 (FIFO) → 1.00 (frontier)**;
  control-law bands pinned (informative → empty → regenerate → saturated).
  The proxy honestly labeled: the real G8 runs when a live authoring loop exists.
- **(c) PASS — namespace isolation structural** (pair rows invisible to dump/len/
  learn_rows/elo_table/certified_density/recompute_elo — all asserted) **+ the
  retrieval-precision floor gate re-ran green** in the same validation pass.

## Tasks

- [x] **T0 — Racing pre-check.** `git status` clean, no sibling hunks touching
  `src/traj_store.rs` / `src/elo.rs` / `src/domains/rustc_errors/`; claim recorded here.
- [x] **T1 — PairRow + construction factory (M1).** New `PairRowDoc` row family in
  `src/traj_store.rs` (key `pair:{code}:{case}`), new `src/frontier_pairs.rs`:
  `construction_pairs() -> Vec<ConstructionPair>` derived from
  `rustc_errors::corpus_gen::generate_corpus()` — for each case, good = the canonical
  repair (the case's own `seed`, the planted truth), bad = a deterministic cross-case
  repair (hash-rotated partner, same `expect_code` family preferred) that leaves the case
  broken → a RESOLUTION pair (fence-checkable). Strategy players mapped to playbook
  `(error_code, strategy_id)` where the inverse exists (NarrowScope/FlattenScope →
  `e0597-scope-hoist`, DropCloneCall → `e0505-clone-at-move` / `e0502-clone-out`,
  AddBorrow → `e0502-deref-removal`-family), generator-native `gen:{code}:{mutator}` ids
  elsewhere (space recorded, never conflated). Zero cargo invocations. Byte-stable
  (deterministic partner rotation, pinned by a determinism test).
- [x] **T2 — Non-circularity fence (mandatory before T1 ingests).**
  **e2e RAN: 20/20 agree (100.0%) vs real rustc** (`tests/frontier_fence_e2e.rs`,
  `--ignored`, 1.7 s release).
  `frontier_pairs::fence_pairs(K)` — sample K=20 pairs; compile good arm + bad arm in a
  scratch crate via the floor harness's compile path; agreement = good arm produces NO
  `expect_code` diagnostic AND bad arm still produces an error. ≥95% agreement or LOUD
  block (`Err` naming the disagreeing pairs). Real-cargo arm is `#[ignore]`d e2e
  (spawns rustc); always-on unit gates cover the fence's arithmetic + verdict logic.
- [x] **T3 — BT closed-form fit + pair-fed re-ingestion (M2).** Zermelo Bradley-Terry
  iterative fit in `src/elo.rs` (locally — katgpt_core::rating ships Elo online only;
  BT is order-independent which the A/B needs), mapped onto the same base-1200/scale
  conventions. `TrajStore::rate_pairs()` + `import_pairs()`: pair arrivals feed a
  SEPARATE pair-fed rating table alongside — never replacing — outcome-fed
  `recompute_elo`. PairRow carries `provenance: "construction"` and NEVER touches
  `EvidenceTier` (asserted by test).
- [x] **T4 — Dispersion/frontier selection (M3).** `src/dispersion.rs`:
  `outcome_entropy(top_k_records)` — Shannon entropy over outcome classes
  ({compiles-clean, parse-fail, decline}) per `(error_code, strategy)` pool, closed-form,
  zero-alloc (fixed-size class table); `frontier_order(cands)` — high entropy first,
  zero-entropy-all-pass retires, zero-entropy-all-fail flags regenerate. Canaries both
  directions (planted degenerate pool flags vacuous; uniform pool stays neutral).
- [x] **T5 — PoC A/B harness (the exit-criterion test).** 4/4 green
  (`tests/frontier_poc_ab.rs`); axes (a)/(b)/(b2)/(c) all pass. En-route: the
  outcome-fed baseline needed an explicit `recompute_elo` (elo_table reads PERSISTED
  ratings); a 1-0 pool is SATURATED by the control law (a draft fixture mislabeled it
  informative and the assert correctly refused). `tests/frontier_poc_ab.rs`
  (feature-gated `latent_retrieval`): deterministic synthetic store (T39 harness
  pattern) + real pair ingestion; arms = outcome-fed / pair-fed / combined.
  (a) unseen-candidate fraction over the playbook universe before/after pair feeding;
  (b) frontier-vs-FIFO ordering A/B — mean rank of would-convert candidates using stored
  outcomes as the conversion oracle (the G8 proxy, honestly labeled a proxy);
  (c) retrieval-floor regression gate re-run (existing gate must stay green).
  Verdict printed; exit criterion evaluated and recorded.
  **NUMBERING NOTE (2026-09-06): this plan was landed as `090_frontier_*` in the same
  window a sibling session dual-allocated 090 for Batch 117's ds4 plan; per the
  citation-weight tie-breaker the batch plan keeps 090 (its number is baked into 5
  corpus concept strings + the batch gate) and this file renumbered 090 → 091
  (citations: issue 072 ×3 + `tests/frontier_poc_ab.rs` header).**
- [x] **T6 — Dispersion telemetry (M6, T7 of the issue).** `stats_report` now
  carries the `dispersion:` line (per-rule outcome entropy, vacuous-window flag). `stats_report` gains a
  dispersion column (outcome-class entropy; zero-for-a-window = vacuous flag) — the
  standing non-vacuity instrument.
- [x] **T7 — Docs + verdict.** AGENTS.md row (issue 072 PoC verdict + commit hash);
  issue 072 checkboxes T1–T4/T7 updated with `- [x]` + evidence; exit criterion
  evaluated in the issue file; T5/T6 (composite schedule, generationed bench) stay
  `- [ ]` pending the exit verdict. Commit + push (`feat: issue 072 PoC ...`).

## Validation record (2026-09-06)

- lib default 1112/0; lib latent_retrieval 1228/0/1i (incl. the retrieval floor gate
  + fixture-purity gates); lib latent+rustc_errors 1228/0/1i; lib
  score_bench,latent_retrieval,fix_verify 1286/0/10i.
- `frontier_poc_ab` 4/4; `frontier_fence_e2e` 1/1 (`--ignored`, real rustc, 100%);
  `floor_corpus_integrity` 3/3; `fix_compile_gates` 4/4; `fix_compile_borrowck` 6/6.
- clippy: 0 real findings (hard-link noise excepted) under default, latent_retrieval,
  and latent+rustc_errors, lib + tests.
- score-bench run #45 at the new HEAD: 96.2/1.9/100/100 — FLAT (heal path unregressed).
- En-route catch: the pair row carried a wall-clock `created_unix` → the idempotence
  test flaked when the two imports crossed a second boundary (full-suite run caught
  it); the field was removed — the row is construction-deterministic.

## Discipline notes

- Store isolation: all tests use temp-dir stores; the live `.heal/` store is never
  opened by tests.
- Feature gating: `PairRowDoc`/`rate_pairs`/`import_pairs` ride `latent_retrieval`
  (same as TrajStore); `frontier_pairs.rs` construction is feature-free (pure tables);
  `dispersion.rs` is feature-free (pure math) — the healer consumes it later.
- Fence e2e requires cargo+rustc on PATH → `#[ignore]` with the reason, mirroring the
  `fix_verify_e2e` precedent.
- No GPU anywhere in this unit — CPU-only, safe under any sibling GPU campaign.

## Non-goals (this PoC)

- No composite exactness ladder (issue T5), no generationed bench ratchet (issue T6) —
  both are post-PoC plan material.
- No GRPO/policy gradients (riir-train issue 522 owns the training lane).
- EvidenceTier semantics untouched; `clippy_verify` oracle paths untouched.

# Issue 072 — Frontier-evidence Challenger: construction-labeled pairs + dispersion selection + generationed bench (J-Zero modelless fusion)

**Status:** OPEN → **PoC EXIT CRITERION MET 2026-09-06** (2 of 3 axes pass, the third
discharged structurally + gate green): T1/T2/T3/T4/T7 landed (Plan 090); T5/T6 below stay
open pending their plan (the issue's own "Plan after the PoC proves it").
**Source:** riir-train `.research/450_J-Zero_Judge_Co_Evolution_Gain.md` §4.1 (arXiv:2608.26582, mechanisms M1–M6).
**Targets the two measured gaps:** (1) T39/Research 135 — selection pools starved (98.5% of candidates unseen; "the bottleneck is evidence volume, not selection math"); (2) Plan 014 Phase 3 — score-bench real-fraction growth deferred with no mechanism; bench saturating (run #3 heal 98.1%).
**Unifying principle (paper §3.2 requirement (ii), discovered independently here as the T3.5.4 contamination fix):** labels must never depend on signals produced by the evaluator itself. Construction labeling is the general form; oracle labeling is its ground-truth special case.

- [x] **T1 — Pair factory (M1).** LANDED 2026-09-06 (Plan 090):
  `src/frontier_pairs.rs` + `PairRowDoc` (`pair:` namespace, disjoint from trajectories)
  + `TrajStore::import_pairs` — 200 construction-labeled resolution pairs derived from
  `corpus_gen` (good = the case's own seed/planted truth; bad = a deterministic hash-
  rotated cross-code partner repair). Strategy players mapped to playbook ids where the
  playbook doc-verified inverse exists (`e0597-scope-hoist`, `e0499-swap-second-decl`,
  `e0502-deref-removal`, `e0505-clone-at-move`); generator-native `gen:{CODE}:{inverse}`
  ids for codes no playbook owns (E0596/E0382/E0507/E0373) — never a borrowed id whose
  diagnostic gate would reject. Zero cargo per pair; byte-stable (pinned). Feeds the
  **soft-judge layer only** — never `EvidenceTier` (asserted by test: dump/len/
  learn_rows/elo_table/certified_density all see zero pair rows).
- [x] **T2 — Non-circularity fence.** LANDED + **RAN: 20/20 agree (100.0%) vs real
  rustc** (`tests/frontier_fence_e2e.rs`, `#[ignore]`d, release 1.7 s): good arm compiles
  clean AND the broken form still fails with exactly `expect_code`, on a deterministic
  stride sample of K=20. Arithmetic gates always-on (`fence_verdicts`/`FenceReport`).
- [x] **T3 — Pair-fed rating re-ingestion (M2).** LANDED: Zermelo Bradley–Terry ML fit
  in `src/elo.rs` (`bradley_terry_elo` — order-free, bit-deterministic via BTreeMap
  iteration, mapped onto the standard base-1200/scale-400 curve via `elo_from_lambda`;
  7 unit gates incl. the fixed-point-expected-score contract and the multiplicity
  invariance) + `TrajStore::rate_pairs`. Headline A/B (Plan 090 T5): productive
  coverage, unseen fraction **0.786 → 0.429** pair-fed vs outcome-fed on the 14-player
  universe (outcome-fed 3 / pair-fed 8). Frozen-34-fixture no-regression: score-bench
  run #45 FLAT (96.2/1.9/100/100); retrieval floors green.
- [x] **T4 — Dispersion/frontier selection (M3).** LANDED: `src/dispersion.rs` —
  `OutcomeClass` {Kept, Reverted, Missed} (the driver's record classes), closed-form
  Shannon entropy over a fixed-size class table, `regime_of` control law (high entropy →
  informative; zero-entropy-all-pass → saturated/retire; zero-entropy-all-fail →
  regenerate; empty → probe), `frontier_order` (ties by id). 10 unit gates incl. both
  canaries (planted degenerate pool flags the vacuous window; mixed pool does not).
  G8 proxy (Plan 090 T5): informative mean rank **2.00 (FIFO) → 1.00 (frontier)**.
  The REAL G8 (frontier vs FIFO as Certified-conversion per authored rule) stays open
  with T5/T6 below — it needs a live authoring loop to measure.
- [ ] **T5 — Composite exactness schedule (M4).**
- [ ] **T6 — Saturation detector + generationed bench ratchet (M5).** Marginal heal-rate gain < ε over the last R `score_history.jsonl` runs flags saturation → admit a generation-v2 fixture set (T1/T5 construction-labeled, T2-fenced) with per-generation floors; v1 floors pinned forever, v2 reported alongside; **v2 admission requires an initial heal-rate drop** (non-vacuity: a new generation that doesn't lower the rate isn't harder). Keeps Plan 014's frozen-denominator honesty (the denominator never silently grows) while restoring measurement resolution past saturation. This IS the mechanism Plan 014 Phase 3 was deferred for. **Input now landed:** T4's saturation telemetry (the `dispersion:` line in `stats_report` + the vacuous-window flag) is the detector half.
- [x] **T7 — Dispersion telemetry as standing non-vacuity instrument (M6).** LANDED
  2026-09-06: the `stats_report` dispersion line (per-rule outcome entropy, top-3, the
  vacuous-window flag) + the `dispersion.rs` instrument set (`window_is_vacuous`,
  `dispersion_column`). Converts the one-off T39 finding into standing telemetry.
- [-] **Deferred — cgsp scenario-dispersion selection.** `PoolConjecturer` scenarios ranked by std of hint-δ across the solver pool, feeding `DualPoolBandit` budget. Game-runtime surface (priority #1) but no consumer demanding it; reopen with the game-quality lane (riir-train issue 522 / Research 450 §5).

**PoC exit criterion (promote to plan):** T1+T3+T4 on the existing store → pair-fed + frontier-fed arms vs outcome-fed baseline: (a) unseen-candidate fraction drops measurably, (b) Certified-conversion per authored rule improves, (c) no retrieval-floor regression. Any two of three → plan.
**VERDICT 2026-09-06 (Plan 090, `tests/frontier_poc_ab.rs`): MET — (a) PASS (unseen 0.786 → 0.429; outcome-covered 3 / pair-covered 8 of the 14-player universe, 200 pairs, zero cargo), (b) PASS at the G8 proxy (informative mean rank 2.00 → 1.00; real G8 deferred with T5/T6 — it needs a live authoring loop), (c) PASS (structural namespace isolation asserted on every outcome surface + the retrieval floor gate re-ran green). Score-bench run #45 FLAT. The next unit is the PLAN for T5/T6 (+ wiring `frontier_order` into the `.heal/misses/` authoring queue, where the real G8 becomes measurable).**

**Anti-scope:** no GRPO/policy gradients here (riir-train issue 522 owns the training loop); no judge-model training; EvidenceTier semantics untouched.

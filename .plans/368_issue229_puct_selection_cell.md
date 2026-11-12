# Plan 368 — Issue 229 T1–T5: the PUCT selection-mode cell

**Status:** CLOSED 2026-11-12 — NOT PROMOTED, §8 withdrawal executed (T1–T5
+ the round-1 REVISE folds all landed; the feat commit carries the
withdrawal + the ρ̂ instrument; the issue keeps the negative record).

Pre-registered cell: `.issues/229_puct_selection_mode_cell.md` (verdict AGREE
x2, Research 396 §9). This plan is the execution ledger, batch-first per the
Protocol A law.

## Tasks

- [x] T1 — `SelectionMode::Puct` implemented (`src/self_evolve.rs`):
  `select_best_candidate_puct` two-pass scoring (Q journaled to the `puct_q`
  scratch + evo to the shared `tilt_rewards` scratch — one `score_candidate`
  call per candidate ever; pass 2 applies `c·P·√(ΣN)/(1+N)` and picks with
  the incumbent's exact best/takes + GP σ² tie-break discipline);
  `DEFAULT_PUCT_C` 0.2 (the PUCB paper value); `with_puct_exploration`
  dose builder; pool ≤ 1 degrades to Q with the `warn_puct_degraded_once`
  loud line (Entropic precedent); enum variant doc carries the disclosed
  double-count + the no-retry-loop scope.
- [x] T2 — dose-response: c ∈ {0, 0.2, 2.0} on the replay. Exploration rate
  0.0000 / 0.1148 / 0.5035 — monotone, asserted in the harness.
- [x] T3 — sequential replay (2,500 instances × 80k episodes, thin 20k +
  dense 60k, common random numbers, counts update per episode;
  `tests/t229_puct_replay.rs`): BetaPosterior / LinearBlend / Entropic arms
  + the PUCT grid c×ρ ∈ {0,0.2,2.0}×{0,0.3,0.7,1.0} + shuffled-prior. P
  synthesized at controlled correlation (`ρ·p + (1−ρ)·z`). Regret axes
  recorded in the landing row (issue T5).
- [x] T4 — falsifier arms in-replay: (a) shuffled-prior control — gain
  vanishes (0.02543 honest vs 0.07011 shuffled at ρ=0.7); (b) Entropic
  comparison — separated (0.03415 vs 0.02061 best PUCT).
- [x] T5 — verdict + demotion-law execution + the goat-audit row close.
  **Verdict round 1 REVISE (Claude, session a8204575) — the decision rule
  pre-registered below was NOT met; §8 executes.** Evidence chain:
  (a) the replay's G2 pass decomposed by prior quality — ρ≥0.7 passes,
  ρ≤0.3 REGRESSES (+30.8% at ρ=0.3, +60.8% at ρ=0; c=2.0 catastrophic
  everywhere, +84.7% even at ρ=1.0); (b) the fold's real-stream ρ̂
  measurement (the `rho_real_stream` example over this box's durable
  store): **ρ̂(Spearman, N≥3) = +0.24, Pearson +0.31; ρ̂(N≥1) = +0.21 —
  an INFLATED upper bound** (selection bias: only picked candidates get
  outcomes), n=14 measured rows at N≥3 (population thin — the honest
  reading is "no evidence of a ρ≥0.7-class prior"); (c) the shipped
  binary's learning half is INERT by design (RELEASE_FEATURES carries no
  clippy_verify — the EMA half never updates), so the deployment prior
  degenerates toward the neutral 0.5 (the ρ≈0 arm) wherever the durable
  learn rows are absent; (d) the pre-registered rule below put the bar's
  interpolated crossing at ρ* ≈ 0.55–0.60 (the +10% bar first cleared
  between ρ=0.5 and 0.6 — see the grid in the landing row), and ρ̂'s
  upper bound (≈0.24–0.31) sits FAR below it with the bias already
  favoring PUCT. **SelectionMode::Puct is WITHDRAWN** (the variant, the
  scoring path, the builder, the harness cells — the measurement
  instrument and the negative record stay); no zombie mode. The lane's
  contribution is the crossover-law pricing: the bonus axis re-opens
  only where a real prior ≥ ρ* exists (the seat-rerank head is the named
  candidate — Bench 119), per Research 396 §8's own re-open conditions.
- [x] T5a (verdict round-1 folds, in order): the ρ̂ instrument
  (`examples/rho_real_stream.rs` — ran 2026-11-12 on this box's durable
  store: Spearman +0.24 / Pearson +0.31 at N≥3, n=14; +0.21 at N≥1,
  n=113 — selection-bias-INFLATED upper bound; per-rule breakdown shows
  no uniform signal: doc_markdown −0.02, explicit_auto_deref −0.06,
  assert_is_empty +0.44); the ρ ∈ {0.4, 0.5, 0.6} cells at the fixed
  c=0.2 dose — −18.3% / +3.4% / +18.3%, bar-crossing ρ* ≈ 0.544; the
  pre-registered decision rule (upper < ρ* ⇒ withdraw; lower ≥ ρ* ⇒
  PASS-conditional pin; straddle ⇒ withdraw, ties to the incumbent)
  recorded BEFORE the ρ̂ run; the dose finding recorded as the hard
  ceiling on any future re-open (c=0.2 the only measured-good dose;
  c=2.0 catastrophic at every ρ: +84.7% even at ρ=1.0).
- [x] T5 executed — **ρ̂_upper 0.31 < ρ* 0.544 ⇒ NOT PROMOTED, §8
  WITHDRAWAL**: the variant, scoring path, builder, constants and both
  harness binaries are REMOVED (git history carries the reproducible
  cell; re-open = a NEW cell against a real ≥ρ* prior — the seat-rerank
  head, Bench 119, the named candidate). Kept: the ρ̂ instrument (its
  [[example]] required-features row landed per the P43 law) + this
  plan + the issue's landing row + the enum-site withdrawal note. The
  goat-audit row closes as RECORDED NEGATIVE (Research 396 §7's
  either/or satisfied).
- [x] Gates (post-withdrawal): default lib 3855/0 · all-features lib
  5073/0 · no-default 1070/0 · clippy −D clean at lib+all-targets ·
  pins_manifest agree · fmt gate PASS (the same counts as the
  executed-cell posture: the withdrawal restores the incumbent surface).
- [x] Commit `feat:` + push; issue 229 updated (landing + withdrawal) in
  the same commit.
- [x] Gates (T4 of Protocol A): default lib 3855/0 · all-features lib
  5073/0 · no-default 1070/0 · clippy −D clean at lib+all-targets ·
  retrieval floors HELD (29/0, `precision_regression_gate` + determinism
  named) · pins_manifest agree · fmt gate PASS · G4 alloc binary PASS
  (101766 == 101766 == 101766) · score-bench ×3 appended (heal_rate
  0.96428573 byte-stable across 4 runs) — bench files REVERTED (selection
  code, not corpus; no history lane consumed).
- [ ] Commit `feat:` + push; update `.issues/229` (task rows + landing) in
  the same commit.

## Files

- `src/self_evolve.rs` — the Puct variant + scoring path + builder + warn.
- `tests/t229_puct_replay.rs` — the cell (T2/T3/T4 axes + G1/G2/G3).
- `tests/t229_puct_g4_alloc.rs` — the G4 counting-allocator binary.
- `tests/t93_selection_regret.rs` — match-arm completion (Puct → Q axis).

# Issue 105: Strand-Keep Credit Repair — Integral-of-Journey Dual Accumulator on Fix Trajectories

**Status:** LANDED 2026-09-14 (`e58ff9b9`, T1–T5 all green; research → katgpt-rs [Research 554](../../katgpt-rs/.research/554_PC_ALM_Ballistic_Dual_Wave_Credit_Propagation.md), [arXiv:2605.31022](https://arxiv.org/abs/2605.31022) PC-ALM, Sakana AI)

## The documented gap this fixes

The rustc_errors per-attempt outcome counters are **poisoned by strand-keeps** (Bench 089/090; Issue 099's interacting corpus): a KEEP that ends the run at a typeck-masked E0614 scores as success, because selection consumes the **terminal** verdict. PC-ALM's core semantic names the exact repair: **credit = λ + ρ·r** — the *integral* of the residual history plus the terminal residual — not the terminal residual alone. A strand-keep has r_T ≈ 0 (ends stable) while the accumulated λ_T = α·Σ_t r_t is large: the journey's error is the credit.

The raw material already ships: the `.heal/fixseq/run-*.jsonl` ring records the ORDERED attempt sequence with `errors [before, after]` per attempt (Issue 91 T1.5) — a signed per-step diagnostic delta is one subtraction away. Nothing accumulates it.

## Signal-diff vs shipped cousins (why this is not covered)

- `select_best_candidate`: consumes `W_EVO·evolution + W_RATE·reliability` — per-candidate history statistics, **terminal-outcome-shaped**; no journey integral. (Same granularity class as the TETHER/Le-Critique finds: the expression one level up from the cousin is where the gap lives.)
- `EvolveRecorder::record_outcome`: fires on applied-fix outcomes — event feed, not a sequence integral.
- `frontier_report` / Beta-LCB: cumulative counters over rules — no within-run ordering, no signed deltas.
- The Issue-100 lexicographic keep rule fixed the *keep decision*; this issue fixes the *credit signal* the selection layer consumes — orthogonal, composable.

## Tasks

- [x] **T1** Accumulator: during a fix run, fold the ordered attempt deltas into `λ_run = α·Σ_t Δ_t` where `Δ_t` is the signed change in the `(masking, count)` lexicographic pair (issue-100 vocabulary); stamp `λ_T` + terminal `r_T` into the ring row (`#[serde(default)]`, pre-100 rings read as absent — never a replay hazard).
- [x] **T2** Selection third axis: `select_best_candidate` consumes composite `sigmoid(λ_T + ρ·r_T)` as an evidence input ORTHOGONAL to evolution/reliability (a blend-weight axis, not a replacement — the TETHER/R135 sweep discipline; sweep ρ, never hand-pin).
- [x] **T3** Gate — measurable on existing fixtures: replay the recorded Bench 089/090 strand-keep populations; report mis-attribution rate (strand-keeps scored as full success) before/after. PASS = strictly lower mis-attribution at equal resolved rate (34/46 parity must not regress).
- [x] **T4** Persist λ_T in the Warm-tier `TrajRowDoc`; `frontier_report` gains the integral axis as instrumentation (not scored — same standing as provenance strata).
- [x] **T5** Score-bench row before/after; `cargo heal --score-bench` append-only history (Issue 089 discipline).

## Landing record (2026-09-14, `e58ff9b9`)

- **T1** — `JourneyCredit` (`driver.rs`): `{journey_residual_sum, terminal_residual,
  terminal_masking}` as u32 (Eq-preserving); `poverty(α,ρ) = sigmoid(α·λ_T + ρ·r_T)`;
  `tainted()` = terminal masking > 0 (ordinary remaining errors are honest partial
  progress — never tainted); the dump stamps every ring row with the run's values
  (`#[serde(default)]`: pre-105 rings decode neutral). α=0.25, ρ=2.0 documented
  consts, applied by consumers — the sweep discipline stays open.
- **T2** — `journey_factor(runs, taint_sum)` (`src/journey.rs`, ungated):
  `clamp01(1 − (sigmoid(mean tainted composite) − 0.5)·2)`; EXACTLY 1.0 with no
  data or all-clean journeys. Consumers: `shape_conditional_score` (discounts
  neighbor-weighted SUCCESSES only, failures in full) + `select_best_candidate`'s
  reliability axis. Wiring: `record_run` + the latent CLI path stamp
  `record_journey` per kept attempt (both backends: TrajStore + memory.json).
- **T3 gate** (`tests/journey_credit_gate`, 3/3): the Bench 089/090 ring
  populations replayed — the tainted key scores STRICTLY LOWER at identical
  counts (10k/0r); the clean-journey key is BIT-IDENTICAL to the no-journey
  legacy key; pre-105 rows decode (0,0) ⇒ factor exactly 1.0. A/B lane
  (`rustc_errors_floor --only ab --interacting`, m3): **46/46 all arms** — the
  34/46 parity bar held with room to spare (the corpus/base moved since Bench
  090 measured 34/46 on the 4090; the discount is provably inert on count-only
  stores, so the arms' parity is unaffected by this change — the honest note:
  the lane's learning pass records outcomes only, so this lane exercises the
  no-journey-data arm; the discount's live surface is real `--fix-compile`
  runs, which DO stamp journeys now).
- **T4** — `TrajRowDoc`/`FixTrajectory` journey fields (serde-default) +
  `TrajStore::journey_rows()` + the `--frontier-report` journey section
  (instrumentation, not scored).
- **T5** — score-bench row #115 @ `e58ff9b9`: **FLAT** 96.2/1.9 (byte-identical
  heal/created rates to the standing rows — the axis is inert on the clippy
  lane, as designed; the journey axis only reprices rustc_errors-domain
  success evidence).

## Non-goals

No change to the keep rule itself (Issue 100's lexicographic rule stands). No ordering change (the Issue 093/099 negative stands — this is the *signal*, not the *ordering*). If T3 shows parity-not-gain, record the negative and close — a measured null on this class is decision-grade.

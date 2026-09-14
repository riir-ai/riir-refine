# Issue 105: Strand-Keep Credit Repair — Integral-of-Journey Dual Accumulator on Fix Trajectories

**Status:** Open — POC/proof task (research → katgpt-rs [Research 554](../../katgpt-rs/.research/554_PC_ALM_Ballistic_Dual_Wave_Credit_Propagation.md), [arXiv:2605.31022](https://arxiv.org/abs/2605.31022) PC-ALM, Sakana AI)

## The documented gap this fixes

The rustc_errors per-attempt outcome counters are **poisoned by strand-keeps** (Bench 089/090; Issue 099's interacting corpus): a KEEP that ends the run at a typeck-masked E0614 scores as success, because selection consumes the **terminal** verdict. PC-ALM's core semantic names the exact repair: **credit = λ + ρ·r** — the *integral* of the residual history plus the terminal residual — not the terminal residual alone. A strand-keep has r_T ≈ 0 (ends stable) while the accumulated λ_T = α·Σ_t r_t is large: the journey's error is the credit.

The raw material already ships: the `.heal/fixseq/run-*.jsonl` ring records the ORDERED attempt sequence with `errors [before, after]` per attempt (Issue 91 T1.5) — a signed per-step diagnostic delta is one subtraction away. Nothing accumulates it.

## Signal-diff vs shipped cousins (why this is not covered)

- `select_best_candidate`: consumes `W_EVO·evolution + W_RATE·reliability` — per-candidate history statistics, **terminal-outcome-shaped**; no journey integral. (Same granularity class as the TETHER/Le-Critique finds: the expression one level up from the cousin is where the gap lives.)
- `EvolveRecorder::record_outcome`: fires on applied-fix outcomes — event feed, not a sequence integral.
- `frontier_report` / Beta-LCB: cumulative counters over rules — no within-run ordering, no signed deltas.
- The Issue-100 lexicographic keep rule fixed the *keep decision*; this issue fixes the *credit signal* the selection layer consumes — orthogonal, composable.

## Tasks

- [ ] **T1** Accumulator: during a fix run, fold the ordered attempt deltas into `λ_run = α·Σ_t Δ_t` where `Δ_t` is the signed change in the `(masking, count)` lexicographic pair (issue-100 vocabulary); stamp `λ_T` + terminal `r_T` into the ring row (`#[serde(default)]`, pre-100 rings read as absent — never a replay hazard).
- [ ] **T2** Selection third axis: `select_best_candidate` consumes composite `sigmoid(λ_T + ρ·r_T)` as an evidence input ORTHOGONAL to evolution/reliability (a blend-weight axis, not a replacement — the TETHER/R135 sweep discipline; sweep ρ, never hand-pin).
- [ ] **T3** Gate — measurable on existing fixtures: replay the recorded Bench 089/090 strand-keep populations; report mis-attribution rate (strand-keeps scored as full success) before/after. PASS = strictly lower mis-attribution at equal resolved rate (34/46 parity must not regress).
- [ ] **T4** Persist λ_T in the Warm-tier `TrajRowDoc`; `frontier_report` gains the integral axis as instrumentation (not scored — same standing as provenance strata).
- [ ] **T5** Score-bench row before/after; `cargo heal --score-bench` append-only history (Issue 089 discipline).

## Non-goals

No change to the keep rule itself (Issue 100's lexicographic rule stands). No ordering change (the Issue 093/099 negative stands — this is the *signal*, not the *ordering*). If T3 shows parity-not-gain, record the negative and close — a measured null on this class is decision-grade.

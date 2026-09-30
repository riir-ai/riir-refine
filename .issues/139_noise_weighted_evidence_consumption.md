# Issue 139 — Consume grouped-evidence noise-weighting primitives (EasyPPO estimation-side extraction)

**Status:** OPEN — gated on katgpt-rs Issue 913 landing (the `grouped_evidence_weighting` primitives). Source: [riir-train Research 463](../../riir-train/.research/463_EasyPPO_Critic_Stabilization.md) ← arXiv:2609.36802 (EasyPPO).

## Why

The healer is the workspace's densest grouped-evidence consumer per LOC, and every evidence surface here aggregates outcomes with heterogeneous noise: a rule fixes many spans (outcome noise varies by span class), trajectory rows record per-fix outcomes (`EvolveRecorder::record_outcome`), and the fade clock needs ≥3 full-corpus sweeps to see a rule die. EasyPPO's decomposition says the noisy groups dominate whatever estimator consumes these rows as the fit tightens — and its exogenous-weight invariance law (`w ⊥ outcome | group` ⇒ argmin preserved) licenses weighting WITHOUT biasing what the Beta-LCB selection layer sees. Selection-side uncertainty already ships (`BetaPosterior`, `best_belief_score`); estimation-side weighting composes with it.

## Tasks

- [ ] **T1** `self_evolve::select_best_candidate`: consume `best_belief_score_weighted` (katgpt-rs 913 T3) — per-rule evidence weighted by `1/max(σ̂,ε)` over the rule's span-outcome groups, mean-one normalized, `n_eff` disclosed. Behind a feature flag default-off; A/B on the score-bench + retrieval eval (the standing instruments); byte-identical unarmed.
- [ ] **T2** SNR exhaustion gate as a cheap fade signal: `err²/σ̂² ≤ 1` per rule (prediction error below the group's irreducible variance ⇒ stop collecting evidence on it) — a per-rule candidate feed for the active-set/fade machinery at a fraction of the ≥3-sweep fade clock's cost. Report-only first; wiring into `--active-set-demote` decisions is owner-gated (the Issue-102 law: nothing demotes automatically).
- [ ] **T3** `frontier_report`: the censoring decomposition `p̂ = P(completes) × E[quality | completed]` + the `δ_C = E[R|C] − E[R]` lift column + `filter_bias_bound` disclosure on any filtered readout (the Report-the-Floor sibling).
- [ ] **T4** `tier_guard` fusion row F1: the noise-domination regime (err² ≪ Var) is exactly where the Elo model and the Laplace history may legitimately disagree — ground the likelihood-ratio arbitration window in the decomposition. Analysis task, no code until the window is measured on real trajectories.

## Gates

- Every landing: disabled = byte-identical (the standing contract); measured gains at paired LB95 on the score-bench / retrieval-eval / frontier instruments — no vibe promotions.
- T2's promotion path is explicitly owner-gated (fade automation touches the corpus lifecycle).

## Verdict

GOAT-tier Gain as a consumer tracker (fusion priority #2 surface — the healer gets the consumption plan for the same mechanism the training track files as riir-train Plan 431). Not Super-GOAT: no product selling point; published mechanism classical. Closes as WONT-FIX if katgpt-rs 913's T3 GOAT fails (the estimation-side thesis falsified).

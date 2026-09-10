# Research 165 — Frog-family distill: learnability-frontier curriculum for the deterministic healer

**Status:** SUPER-GOAT (healer track) — POC filed as `.issues/089`; training track routed to `riir-train/.research/451` + `riir-train/.plans/392`. Gistify + debug-gym: PASS with extracts folded here.

**Date:** 2026-09-10
**Sources:**
- arXiv:2609.07925 — "FrogNano: Training a 4B Coding Agent via Online Task Synthesis" (MSR Montréal Froggy team, 2026-09-07; fetched v2 2026-09-09)
- arXiv:2510.19898 — "BugPilot: Complex Bug Generation for Efficient Learning of SWE Skills" (v2 2025-10-28)
- arXiv:2510.26790 — "Gistify! Codebase-Level Understanding via Runtime Execution" (v1 2025-10-30)
- arXiv:2503.21557 — "debug-gym: A Text-Based Environment for Interactive Debugging" (v1 2025-03-27)
- https://github.com/microsoft/ShadowFrog — **NOT the FrogNano code**. Same team's agent-memory skills product (`.shadow/` per-file markdown KB, dream/meditate modes, MIT, 1,063 tests). FrogNano's TaskPilot/Leaf harness has **no announced code or weights release**.
- https://microsoft.github.io/debug-gym/ — team hub + MIT env (microsoft/debug-gym, 305★, pushed 2026-09-09). FrogNano does **not** train in it (uses the Claude-Code-style Leaf harness; debug-gym cited only as related work [62]).

## 1. TL;DR

The four papers are one pipeline: debug-gym (interactive env) → BugPilot (realistic bug synthesis) → Gistify (comprehension eval) → FrogNano (RL at the learnability frontier). The transferable core is **not** the RL: FrogNano's TaskPilot has **no learned difficulty model anywhere** — the frontier estimator is the policy's own empirical resolve rate `p̂` over blind rollouts, with a hard admission band `0 < p̂ ≤ 0.5`, an executable oracle gate (F2P fail→pass ∧ P2P stable), and a one-axis refinement loop (mutate the problem statement only, behavioral precision ↔ solution localization; measured 0% / 50% / 100% resolve for under/precise/over-specified variants, over-specified REJECTED for naming the fix).

Every one of those rules ports verbatim to a **deterministic consumer**: our healer's "policy rollouts" are exact per-span oracle outcomes already recorded in the trajectory store, and the paper's stochasticity confound (policy variance ⊕ task difficulty) disappears — `p̂` over a span population is exact. Commercial value: the mining network's corpus self-calibrates — mining batches and corpus slots land exactly where the healer is learning (`p̂` mid-band), saturated rules demote to suggest-only, unreachable rules triage, and the score bench gains a difficulty axis that predicts which batch will move the needle.

## 2. Per-paper, per-track verdicts

| Paper | Modelless/healer track | Training track |
|---|---|---|
| FrogNano 2609.07925 | **SUPER-GOAT** — frontier-band admission + refinement axis (this note → Issue 089) | Gain — RL-only DPPO recipe → `riir-train/.plans/392` (G0 GO/NO-GO < 10 GPU-h) |
| BugPilot 2510.19898 | **Gain** — provenance weighting law (+2.06 vs +0.00 is the largest measured effect), hard-tail fixture generation, difficulty-as-quality-axis → Issue 089 T5 | Gain — SFT recipes (teacher assistant-content finding, AllData mix) → `riir-train/.research/451` §4 |
| Gistify 2510.26790 | **PASS** — extracts folded (oracle-payload context rule, difficulty proxy) + redirects | n/a |
| debug-gym 2503.21557 | **PASS** — validates compile-gate/oracle-first design; anti-cheat rails analog already shipped; redirects | n/a |

## 3. The extraction (No-GD advocate inventory, merged)

Translation key: paper *task* = heal trial (span context + corpus entry + oracle verdict); *gold patch* = mined fix-pair post-image; *F2P tests* = the domain oracle re-run (clippy clear / rustc error gone / `cargo test` / hadolint clean); *N blind rollouts* = N sampled spans (heterogeneity replaces stochasticity → exact `p̂`).

| # | Rule (exact) | Lands on | Gate |
|---|---|---|---|
| 1 | **Band admission**: `p̂(E) = resolves/matched` per corpus entry; admit iff `0 < p̂ ≤ 0.5`; `p̂=1` → demote suggest-only; `p̂=0` → triage | Mining-queue admission + corpus stats | In-band yield/batch ↑; G3 heal rate flat |
| 2 | **Beta-LCB band**: `LCB₅ = Beta(1+r, 1+f).q(0.05)`; admit iff `LCB₅ ≤ 0.5 ∧ r ≥ 1 ∧ n ≥ n_min` | Same, confidence-aware | False-admit rate ↓ (substrate: katgpt-core `beta_lcb_order_into` — consume, never fork) |
| 3 | **Saturation demotion**: trailing W=50 oracle encounters at `p̂=1` → out of retrieval fan-out, entry kept, auto re-admit on first fresh unresolved span | Fan-out + corpus stats | Fan-out size ↓ at flat heal rate; membership floor |
| 4 | **Zero-resolve triage fork**: `fires=0` → unreachable → drop; `fires>0 ∧ resolves=0` → expressiveness gap → L3 ruliology or `decline` | Post-batch triage | Routing precision |
| 5 | **Mining-side fail-before/pass-after**: admit pair iff `D ∈ diags(pre) ∧ D ∉ diags(post) ∧ diags(post) ⊆ diags(pre)` | Fix-pair mining admission | Mined-pair precision ↑ (1 extra oracle run/pair — price it) |
| 6 | **Diagnostic-set monotonicity**: verdict 1.0 iff post-diags ⊆ pre-diags; **0.5** iff target cleared ∧ new diags | Compile-gate verdict ladder | Post-fix regression rate ↓ |
| 7 | **Shaped heal score**: `{0, 0.5, 1.0}` in score_history.jsonl (new append-only generation) | Score bench | Score variance ↑; aggregates match binary when no regressions |
| 8 | **Log-length tie-break**: among oracle-passing candidates `argmax R(n)`, `R = 1 if n≤N else 1−α·ln(n/N)`; N = per-rule median mined diff; α offline grid-fit (no gradients) | Candidate ranking | Fix churn ↓; chosen (α,N) beats default on replay |
| 9 | **Half-win Elo/Beta feed**: 0.5-outcomes feed Elo as ties / Beta as half-count, never discarded | Candidate scoring | Score oscillation damping |
| 10 | **Provenance weighting law**: `w(pair)`: organic-feature-churn = 1.0; synthetic/injected = ε; per-class yield per batch | Mining admission + corpus schema | Predict organic ≫ synthetic (paper: +2.06 vs **+0.00**) — **largest paper effect size** |
| 11 | **Oracle-payload context rule**: span context MUST carry the raw diagnostic payload (message/span/suggestion); A/B Δresolve on frozen spans (paper: 42→60%, r=0.76) | Drafter context builder | Δresolve ≥ floor |
| 12 | **Closed-form difficulty proxy**: `D = log₂(1+oracle_calls) + log₂(1+files_touched)` (+ error-chain depth for rustc_errors) | Span metadata; bench stratification | Spearman ρ(D, resolve) monotone |
| 13 | **Measured-not-filtered corpus report**: p̂ histogram + D distribution; health = band-mass fraction; report-only, exit 0 | Score bench / corpus-stats | Report floors; band-mass predicts next-gen yield |
| 14 | **Over-localization rejection**: flag entries whose fix-template pins instance-specific tokens (`|tokens(template) ∩ identifiers(span)| > k`) | Corpus L0 authoring | Flagged vs unflagged resolve spread |
| 15 | **Specification-triple probe**: author under/precise/over variants of contested entries; expected `under≈0, precise∈band, over≈1∧flagged`; accept precise only | Mining QA | Probe predictive validity |
| 16 | **One-axis refinement driver**: per cycle mutate exactly one authoring axis; keep iff `|p̂−0.5|` decreases on frozen population, else revert | Post-mining improvement queue | Cycles-to-band vs untargeted drift |
| 17 | **Context-budget bisection**: per rule-family, largest context `c` with `resolve(c)=resolve(c_max)`; ship as family constant | Drafter context budget | Context bytes ↓ at flat resolve |

## 4. Signal-diff vs shipped substrate (the §3.6 check, run before claiming novelty)

- **Plan 091 `--misses` frontier queue** (src/bin/heal.rs, `evolve::frontier_queue_report`): the closest shipped cousin. It orders the **rustc_errors authoring queue** by dispersion-informativeness with outcome classes `unobserved / regenerate / saturated` — a per-CODE, single-domain, ORDERING-only frontier. The delta: this distill is per-RULE/per-FIXTURE **admission** (gate, not order), corpus-wide across all 6 domains, with Beta-LCB uncertainty (n_min), the p̂=0 triage fork, the refinement axis, and the provenance law. Issue 089 T3 explicitly generalizes Plan 091 rather than replacing it. Saturation is NOT claimed as novel — the class already ships there.
- **Elo + BetaPosterior candidate selection** (`src/elo.rs`, `select_best_candidate`): per-CANDIDATE selection WITHIN a span (which fix to apply). Frontier admission is per-TASK/per-RULE (which work to take next). Different granularity, different consumer. **Load-bearing caution — R135/Bench 047**: at real pool sparsity (98.5% unseen), selection sophistication measured ≤0.04pp; the bottleneck is evidence volume. Consequence baked into Issue 089: **report-first** (T1–T2), gate only when band-mass is proven predictive; hard gates require n_min + online count-updates (never frozen tables — R135 lesson 2).
- **M5 non-vacuity admission gate** (score-bench generations): bench-side difficulty control (a new generation must be strictly harder than v1). Adjacent axis, distinct mechanism — cited, not duplicated.
- **katgpt-core `beta_lcb_order_into`**: the Beta-posterior LCB primitive already ships (consumed by riir-dao). Row 2 consumes it — zero new math, substrate-first ✓.

## 5. Novelty gate

**Pinned claim:** *LLM-free, gradient-free learnability-frontier curriculum for a deterministic code-healer — per-rule/per-fixture Beta-LCB admission band (`0 < p̂ ≤ 0.5`) + one-axis authoring refinement + provenance weighting, generalizing the rustc_errors-only Plan 091 frontier-ORDERING queue to corpus-wide ADMISSION across all 6 domains.*

Published bounds (prior-art agent, 15 searches):
- **FrogNano/TaskPilot** — the exact mechanism class, but LLM-in-the-loop (the policy IS the estimator; task generation is an LLM). (a)✓(b)✓(c)✓(d)✗
- **POET** (arXiv:1901.01753) — no-LLM minimal-criterion frontier gating, but bipedal locomotion, not code. (a)✓(b)✓(c)✗(d)✓
- **PAIRED/UED** — regret-maximizing frontier generation, multi-agent games. (c)✗
- **SWE-smith** (arXiv:2504.21798) — deterministic programmatic code-task generation, but STATIC (no per-agent competence, no online loop). (a)✗(b)✗
- **Absolute Zero** (arXiv:2505.03335), **GASP** (arXiv:2603.15957) — frontier self-play for LLMs. (d)✗
- **EnvGen, TSCL, self-paced, IRT/adaptive-eval** — frontier selection for non-code domains, or evaluation-only.

The three-way intersection (frontier-calibrated × code-repair × deterministic-no-LLM) is open — a **composition delta**. Verdict per question:
1. **No prior art?** YES (composition delta, workspace delta vs Plan 091 verified by grep + read).
2. **New behavior class?** YES — self-calibrating corpus: mining batches land where the healer learns; band-mass becomes a bench-sensitivity predictor no incumbent axis carries.
3. **Product selling point?** YES — "cargo heal's corpus self-calibrates to your codebase: it mines exactly the fix classes at the edge of the healer's competence, and tells you when it is saturated."
4. **Force multiplier?** YES — traj_store (outcome feed) + score_bench (measurement) + mining queue (admission) + corpus_gen (refinement axis) + katgpt-core Beta primitive.

**All 4 YES → Super-GOAT.** Mandatory outputs this session: this note (guide) + Issue 089 (POC) + training-track routing (451/392). Open-primitive check: the generic math (Beta-LCB band) already ships in katgpt-core — consume, do not open a parallel primitive.

## 6. Fusion

paper × Plan 091 × R135: TaskPilot's online-against-a-policy loop becomes the healer's **offline-against-a-frozen-span-population** loop (rows 14–16) — the one place the paper's method transfers exactly, because a deterministic consumer can hold the population fixed. R135's starvation law converts TaskPilot's hard admission filter into a two-stage report→gate design. BugPilot's BugInstruct ablation (+0.00) becomes the corpus-generation law: never inject failures on purpose; mine organic feature-churn and manufacture only the hard tail via feature-introduction-shaped composition (Issue 089 T5).

## 7. MOAT gate (riir-clippy: consumer-first)

Measured healer-quality lever: in-band yield per mining batch ↑ at flat binary heal rate (G1), false-admit rate ↓, fan-out shrink at flat heal rate (G3). In scope (corpus/trajectory/selection/bench structure). Passes.

## 8. PoC + validation protocol

The PoC IS Issue 089 (offline replay over score_history.jsonl + trajectory store; no GPU). §3.6 standing: the Super-GOAT claim is composition-level; the mechanism-level evidence (signal-diff §4 + prior-art bounds §5) is complete; the quality claim ("band-mass predicts next-gen yield") is asserted by T2's report and gated by T4's GO condition — it is NOT claimed proven here. Gate: T1's report must show band-mass ↔ next-generation yield correlation on ≥3 historical generations before any admission gate wires.

## 9. Priorities

- **P0** — T1/T2 report-first (read-only, exit 0, floors not ceilings).
- **P1** — T3 generalize Plan 091 to all domains; T5 provenance field.
- **P2** — T4 admission wiring (GO-gated on T2 evidence); T6 difficulty proxy + stratified bench.
- **P3** — T7 shaped 0.5 verdict generation; T8 log-length tie-break.

## 10. PASS-Redirects

- **PASS-Redirects (synthesis):** Lee, Kim, Singh, et al. [arXiv:2510.26790 "Gistify! Codebase-Level Understanding via Runtime Execution"] — PASS: the reusable extracts (oracle-payload-in-context r=0.76 rule → row 11; closed-form difficulty proxy → row 12; test-splicing anti-cheat ≈ our compile-gate) are folded into Issue 089; the task shape itself (single-file reproduction eval) has no healer consumer today.
- **PASS-Redirects (synthesis):** Yuan, Moss, El Feghali, et al. [arXiv:2503.21557 "debug-gym: A Text-Based Environment for Interactive Debugging"] — PASS: validates oracle-first design (grounded tool feedback beats LLM-generated feedback; capability threshold below which tools hurt → keep the drafter's toolset minimal); `.debugreadonly` anti-cheat ≈ our compile-gate + fixture immutability; Rust port would ride DAP — no healer surface changes.

## 11. Adversarial panel + discard ledger

Panel: No-GD advocate (17-row inventory, §3) + model-based advocate (training track, `riir-train/.research/451`). Auditable discards:
- *Faithful FrogNano replication (8×B200)* — hardware-bound discard (M3 + 1×4090 = 2+ orders short on rollout throughput); mechanism retained at POC scale (392).
- *BugInstruct-class intentional injection* — the recipe's own measurement kills it (+0.00 SFT gain); adopted as a law, not a technique.
- *Frontier admission as a hard gate now* — R135/Bench 047 starvation evidence; demoted to report-first two-stage (T1→T4).
- *Healer-inversion task synthesis (re-inject lints to make training tasks)* — BugInstruct-shaped; dead weight by the same +0.00 evidence; git-history revert-repair is the sanctioned source.

## 12. Track routing summary

Healer/modelless → this note + `.issues/089`. Training → `riir-train/.research/451` (recipe + Path 0 decomposition) + `riir-train/.plans/392` (G0 GO/NO-GO, DPPO POC 40–120 GPU-h at 0.6B, Repair-RL GOAT G0–G4). ShadowFrog + debug-gym fetched artifacts: MIT; no FrogNano code exists to clone — `.raw/` clone not applicable.

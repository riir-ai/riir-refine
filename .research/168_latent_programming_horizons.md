# Latent Programming Horizons in Coding Agents → Probe-Gated Verification

**Status:** DISTILLED — pending owner decision (spawns Issue 107 here, katgpt-rs Issue 780, riir-train Plan 403)

> Silva, Tu, Monperrus — [arXiv:2607.05188 "Latent Programming Horizons in Coding Agents"](https://arxiv.org/abs/2607.05188) (KTH, 2026-07-06).
> Code: https://github.com/ASSERT-KTH/program-probes (README read @ release `55f885816989a20633f7dd7b1081a067cb6045f1`; no `.raw/` clone — verdict rests on abstract + README, not a mining batch). Paper license CC BY-SA 4.0; repo/dataset license unverified.

## TL;DR

Linear (logistic-regression) probes on an LLM's residual streams during agentic coding decode properties of the evolving program — `compiles`, `tests pass`, `reduces failing`, `has regressions` — at AUC up to 0.83, predict **future edit outcomes up to ~25 steps before materialization** ("latent programming horizon"), and **transfer across benchmarks without retraining**. For us this is not a training paper: a linear probe is exactly mutation class #3 (direction vector + sigmoid gate), and the healer already logs the paper's entire label set in `.heal/fixseq/run-*.jsonl`. The distillation: **an outcome-fit, calibrated probe that GATES the verification oracle (skip/relax/run) instead of only ranking candidates** — a composition none of our surfaces (and, per prior-art sweep, no published work) currently ships.

## Paper findings (evidence basis: abstract + repo README)

1. **Linear decodability**: `σ(w·h + b)` per (property × layer), layers [0,10,20,30,39], models Laguna-XS2 + Qwen3.6-35B-A3B, benchmarks SWE-bench Verified + Pro. AUC ≤ 0.83 (correctness).
2. **Lookahead**: probes trained on future-edit outcomes beat chance up to ~25 steps ahead — the residual state runs ahead of the agent's own edits.
3. **Transfer**: probes fit on one benchmark decode the other without retraining.
4. Label construction = replay every edit in a sandbox (compiles from pytest log, SWE-bench eval script); `cmd_idx = -1` clean-checkout baseline; carry-forward expands per-edit labels to per-stride-step.

## Path 0 decomposition (coordinator merge of both advocates)

| Component | Ships in stack? | GD-free extraction |
|---|---|---|
| Linear probe readout `σ(w·x+b)` | **YES** — `FutureBehaviorProbe` (katgpt-pruners, offline-fit, BLAKE3-frozen, <200ns); `IndicatorProbeBank` (katgpt-core, frozen thresholds); `LatentSkillEvolution::score_candidate` (online EMA) | — |
| Per-attempt outcome labels | **YES** — fixseq `AttemptRecord` (riir-clippy `driver.rs:57-99`): outcome Kept/Reverted, `errors(before→after)`, `masking(before→after)` ≡ the paper's compiles/reduces_failing/has_regressions verbatim; `will_resolve` ≡ run reaches clean | — |
| Online closed-form fit | PARTIAL — EMA direction (`evolve.rs:168`) is online but uncalibrated | **YES** — ridge/RLS moment accumulators (`G += x·xᵀ; h += y·x`), closed-form LDA `Sw⁻¹(μ₊−μ₋)`, perceptron/Elo-class updates, Beta counting on projections. IRLS fallback is weaker than the already-sanctioned deterministic-LoRA class |
| Pre-verification **gate** (skip/relax/run) | **NO** — probes only *rank*; the gates (`llmexec_guard`, `ActionVerifyGate`, `SoftReject`) are hand-set sigmoids on entropy/pruner evidence, not outcome-fit | **YES** — composition of the three halves |
| Horizon measurement AUC(k) | NO | **YES** — one pass over ordered fixseq rings; report-first |
| Transfer protocol (fit repo A / eval repo B) | NO (the withheld-pair OOD axis is unmeasured everywhere) | **YES** — pure evaluation |
| Residual-stream capture on our engine | YES — `PatchableTransformer::forward_capture` (gemma2 wiring) + ANE hidden-state cosine arbiter (≥0.999) | read-only tap; modelless-compatible |

**Path 0 verdict: MODELLESS-VALIDABLE** for the healer-side core. All components have analogs or GD-free extractions; no deferral required. Model-based track items are real but SECONDARY (serving-envelope rule — riir-train Plan 403).

## Signal-diff vs closest cousins (§3.6 — formula-level, one read each)

| Cousin | Formula / signal consumed | Delta vs the paper's mechanism |
|---|---|---|
| `LatentSkillEvolution::score_candidate` (`evolve.rs:197`) | `sigmoid(dot(evolved_dir, candidate_dir)/T)`; EMA fit from outcome labels (online); current-state candidate embedding | Closest in-tree. Uncalibrated (no AUC/LCB discipline), **ranking-only** (feeds `select_best_candidate`), per-lint scalar, neutral-0.5 cold start, no horizon/transfer axis |
| `FutureBehaviorProbe` (`katgpt-pruners/src/future_probe.rs:108`) | `σ(w·act + b)`, w **trained offline** (logistic regression, Kortukov et al. 2026 / Research 267), BLAKE3-committed, hot-swap | The literal paper primitive — but no label loop from the agent's own logged outcomes, built for steering (FPCG), no verify-gating, no horizon measurement |
| `NonergodicOrdering::filter_score` (`nonergodic_order.rs:257`) | marginal P(resolved) next-attempt from within-run Bayesian filter ⊕ shape-retrieval-weighted Beta | Predicts next attempt (good!) but keyed by strategy/menu position, not a linear readout over a latent vector; **orders only, never skips verification** |
| `llmexec_guard` / `ActionVerifyGate` / `SoftReject` | entropy/depth sigmoids → VerifyTier (Skip/Screening/FullVerify); SoftReject 3-way graded verdict | The **gate consumers** exist and are GOAT'd — but hand-set, not outcome-fit. This is exactly where a fitted probe plugs in |
| `KarcForecaster` (katgpt-core, Plan 308) | online ridge readout forecasting **next latent state** | Predicts next-latent (SSL), not labeled action outcome; no sigmoid success gate |

**Gap statement (grep-verified):** every half exists — outcome labels (`fixseq`, `report_outcome`), linear+sigmoid readouts (`score_candidate`, `FutureBehaviorProbe`, `IndicatorProbeBank`), verification-tier gates (`llmexec_guard`, `SoftReject`) — but nothing composes them: no surface fits a probe online from recorded outcome labels and then reads `sigmoid(w·current_latent)` to **gate** (skip/relax/run) the subsequent verification pass.

## Prior-art verdict (§4, subagent sweep ~17 searches)

Component analysis of the pinned claim ("pre-verification outcome prediction from current latent state, fit online from logged per-attempt labels"):

1. Linear latent readout — **saturated** (Azaria & Mitchell 2023; Burns CCS 2022; Kadavath 2022; Orgad ICLR'25).
2. Per-attempt action outcome pre-verification — published in **disjoint halves**: code-correctness-without-test-execution exists one-shot (Ribeiro et al., ICSE'26, arXiv:2512.07404); agent-loop probes target *episode-level doom*, not *this edit's outcome* (Ruan et al. 2026 "Doomed from the Start", arXiv:2607.06503 — probe cascade aborts doomed episodes, 45–60% compute saved; Mammen et al. arXiv:2609.09448 success monitors).
3. **Online fitting from logged outcome labels — genuinely unoccupied** in the latent-state literature (every probe work trains offline and freezes; the only online-fit verification gates are non-latent classic SE — predictive test selection, Meta 2018+/PEELER).
4. Pre-verification confidence gate — abort-gate pattern exists (2607.06503); "skip the verifier on THIS attempt because the probe says the outcome is determined" is not published for coding agents.

**Novelty is defensible only anchored on the online-fit loop + per-attempt verification gate.** Claiming "latent state predicts code outcome before verification" alone would be falsified by ICSE'26; "probe-gated agent compute" alone by 2607.06503.

## Novelty gate (§1.5)

- **Q1 (no prior art?): NO** — `FutureBehaviorProbe` ships the literal primitive in-repo (offline-fit variant), and the literature margin is a composition margin, not primitive novelty. → **not Super-GOAT**.
- **Q2 (new behavior class?): YES** — calibrated pre-verification gating changes the oracle-invocation contract; no incumbent skips/relaxes the compile gate from a fitted readout.
- **Q3 (product selling point?): YES (healer domain)** — "cargo heal predicts whether a fix will land before invoking the compiler" (label-budget halving is the measurable form). Game-side weaker (below).
- **Q4 (force multiplier?): YES** — composes fixseq ring + `self_evolve` + `nonergodic_order` + `SoftReject`/verify-tier seams + `FutureBehaviorProbe` + freeze/thaw (probe ≈ 260 B artifact through the existing envelope).

**Verdict: GOAT (Gain), not Super-GOAT.** Per-track (TTPO rule): modelless track = Gain (Issue 107 + katgpt-rs Issue 780); model-based track = Gain, SECONDARY (riir-train Plan 403).

## Fusion (§1 Distillation)

**Primary: paper × fixseq ring × SoftReject/verify-tier seams = probe-gated verification.** Fit `w` online (closed-form ridge over span embeddings × `AttemptRecord` outcomes, decaying moments), read `sigmoid(w·x)` at decision time, map to the 3-way SoftReject pattern: confident-keep → relax/skip the compile gate; confident-fail → skip attempt + escalate; uncertain → run the oracle (uncertainty sampling pays the label only where it teaches). Calibration via Beta-LCB over sign-agreement counts — the `best_belief_score` discipline applied to a vector readout.

**Secondary fusions:**
- Paper × `frontier_report` = `--horizon-report`: AUC(k) over lead k on ordered rings; horizon = max k above floor+margin. Report-first, exit 0, refuses green zero. Decides whether the gate lane pays at all (horizon ≥ 2).
- Paper × rule corpora = cold-start rule priors: unseen rule inherits Beta prior from embedding-neighbors (kernel form, no fit) — the paper's transfer finding in miniature, leave-one-rule-out measurable.
- Paper × `k_selector` (riir-ai) = belief-horizon planning depth: per-NPC belief horizon measured, k chosen from it ("don't plan deeper than you can predict"). Fusion idea, novelty TBD — no file.
- Paper × spec-decode (katgpt-rs) = acceptance probe gating whether to spec-decode a stretch; never skips verification itself → G1 bit-identity holds by construction. Recorded in Plan 403 (secondary lane).

## Step-4 reframes

- **Game-context:** the two-brain model's think-brain confidence already decays (`sigmoid(−λ·Δt)`); the paper's horizon is the same law measured forward instead of backward — "how far ahead can this NPC's belief steer" as a per-entity scalar driving planning depth (k_selector) and curiosity (horizon deficit). Weak/speculative — recorded, not filed.
- **Consumer-context (healer, priority #2):** the mechanism manifests on (a) the retrieval index (span embeddings are the latent a probe reads), (b) fix-trajectory memory (fixseq = the label feed), (c) score-bench (label-budget axis + AUC-vs-floor gates), (d) drafter-pruner selection (the gate plugs into candidate ordering), (e) rustc_errors playbooks (per-error-code probe snapshots). This is why the note lives here.

## Defend-wrong posture (§3.6)

No quality-parity claim is made (we do NOT claim our probe matches the paper's 0.83 AUC — that is their models, their labels). Architectural claim (grep-proven gap) + latency claim (readout is one `simd_dot_f32` + sigmoid, the `FutureBehaviorProbe` cost class) stand; the quality axis is exactly what Issue 107's GOAT gate measures against OUR floors. UQ rule ("Report the Floor") applies: every probe claiming a probability must beat (a) the per-error-code base-rate Beta-LCB predictor and (b) the Elo/Beta history-aggregate floor — a probe that cannot beat history-only evidence is a hollow AUC.

## Honest caveats

- Full PDF not read (jina PDF fetch failed); findings from abstract + repo README @ pinned release. Layer-wise AUC tables unread.
- The healer's "latent state" is a hash/AST span embedding (8-dim `ModellessEmbedder` today), not a transformer residual stream — linearity evidence transfers as motivation, not as measurement; D may need raising for probe separation.
- fixseq ring stores `span_hash`, not embeddings — needs one additive column or an offline re-embed pass (by `span_hash`).
- Q2_0-quantized residual AUC on our engine is empirically open (ANE cosine arbiter encouraging, not proof); dataset/repo licenses unverified before ingestion.
- Agent A caveats: two prior-art rows (OpenReview) rest on snippet TL;DRs; venue uncertain.

## Cross-refs

- Issue 107 (this repo): probe-gated fix verification — the composition PoC.
- katgpt-rs Issue 780: `OnlineLinearReadout` katgpt-core primitive (feature `linear_probe`) — the DRY home for the online fit (evolve.rs EMA is a local uncalibrated twin).
- riir-train Plan 403 (SECONDARY): replay/agentic probe measurement on our engine + aux-loss ladder, GPU-hours + GOAT gates.
- Lineage already consumed: Research 267 / Plan 292 (`FutureBehaviorProbe`, Kortukov et al. 2026) — this paper is the program-outcome specialization of that lineage.

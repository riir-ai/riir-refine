# Probe-gated fix verification — compose outcome-fit probe × verify oracle

**Status:** OPEN (filed from Research 168 — arXiv:2607.05188 distillation)

## Summary

Every half of "predict whether a fix attempt will land BEFORE running the compile oracle" ships — outcome labels (fixseq `AttemptRecord`: Kept/Reverted, `errors(before→after)`, `masking(before→after)`), linear+sigmoid readouts (`LatentSkillEvolution::score_candidate`, katgpt-rs `FutureBehaviorProbe`), and verification-tier gates (`SoftReject`, `llmexec_guard`-class) — but nothing composes them. The fitted probes only **rank** candidates; the gates are **hand-set** sigmoids on entropy/pruner evidence, never outcome-fit. This issue closes the composition: an online-fit, calibrated probe that **gates** (skip/relax/run) the `fix_verify` compile oracle.

## Why now (evidence)

- arXiv:2607.05188: program-outcome properties are linearly decodable from latent state, with lookahead — the signal class exists; the question is only whether OUR span embeddings carry it at useful AUC (the GOAT gate answers, honestly either way).
- The label corpus already accumulates: every `--fix-compile --write` run appends `.heal/fixseq/run-*.jsonl` (seq, code, span_hash, strategy, outcome, errors b→a, masking b→a).
- The gate seam is GOAT'd: `SoftReject`'s 3-way graded verdict (Accept/SoftReject/Reject from a `[0,1]` confidence) is the exact contract a probe confidence plugs into.
- Prior-art sweep (Research 168 §Prior-art): the composition — online-fit from logged per-attempt labels + per-attempt verify-gate — is unoccupied in the published literature (closest: arXiv:2607.06503 episode-doom abort cascade, offline-frozen; arXiv:2512.07404 one-shot correctness reranking).

## Design sketch

- **Fit (offline-first, online later):** closed-form ridge/RLS over span embeddings × attempt outcomes — moment accumulators `G += x·xᵀ`, `h += y·x`, `w = (G+λI)⁻¹h`, decay-able. Per-error-code family probes. NO gradient descent (mutation class #3: direction vector + sigmoid gate; see Research 168 Path 0 table).
- **Read:** `sigmoid(dot(w, span_embedding) + b)` at candidate-decision time — one `simd_dot_f32`, the `FutureBehaviorProbe` cost class, alloc-free.
- **Gate (behind feature flag, kill-switch env):** probe confident-keep → relax/skip the compile gate for that attempt; confident-fail → skip attempt + escalate ordering; uncertain → run the oracle (uncertainty sampling — the label is paid only where it teaches).
- **Calibration:** Beta-LCB over sign-agreement counts (`best_belief_score` discipline) — a probe that cannot beat (a) the per-error-code base-rate floor and (b) the Elo/Beta history-aggregate floor is a hollow AUC (UQ "Report the Floor" rule).
- **Substrate-first:** the generic primitive lands in katgpt-core as `OnlineLinearReadout` (katgpt-rs Issue 780, feature `linear_probe`) — this repo consumes it; `evolve.rs`'s EMA stays as the ranking lane, not forked into a second local fit.

## Tasks

- [ ] T1 — `--horizon-report`: one pass over existing fixseq rings, AUC(k) vs lead k (attempt index before outcome), horizon = max k ≥ floor+margin; floors = base-rate Beta-LCB and history-aggregate predictors. Report-first, exit 0, refuses green zero (empty ring / blind walk ⇒ exit 1). Decides lane viability (horizon ≥ 2 pays).
- [ ] T2 — embedding column: extend `AttemptRecord` with the span embedding (or offline re-embed by `span_hash`); ring format stays jsonl-append.
- [ ] T3 — probe fit harness: closed-form ridge fit per error-code family over rings; held-out-run AUC vs both floors; katgpt-core `OnlineLinearReadout` consumption once Issue 780 lands (local fit until then, clearly marked provisional).
- [ ] T4 — gate wiring behind `probe_gated_verify` feature: SoftReject-pattern 3-way mapping; kill-switch `RIIR_HEAL_PROBE_GATE=0`; never gates raw correctness — only oracle invocation (compiles still verified on the keep path per the Issue-100 keep rule).
- [ ] T5 — GOAT gate: (G1) held-out AUC ≥ floor + margin on ≥2 error-code families; (G2) label-budget curve — reference heal rate at ≥50% fewer compile-oracle invocations vs label-everything AND vs random skip; (G3) no-regression on score-bench heal rate + the existing fixseq e2e; (G4) alloc-free read path.
- [ ] T6 — transfer row: fit on repo A rings, eval on repo B (`swept_repos.txt` population); transfer ratio recorded in the bench doc (two-floor discipline).

## Non-goals

MLP/multi-layer probe heads (GD; the paper's own result says linear suffices), policy RL, gating anything other than oracle invocation (never the keep rule's correctness semantics).

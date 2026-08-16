# Bench 035 — Entropic (max-seeking) selection mode: measured NEGATIVE on the max metric

> **Issue:** [026](../.issues/026_entropic_selection_mode.md) — T3/T4
> **Date:** 2026-08-17
> **Source paper:** arXiv:2601.16175 (TTT-Discover), prior art RS-GRPO arXiv:2509.24261 / RSPO arXiv:2508.01174; distilled at `katgpt-rs/.research/019_TTT_Discover_Test_Time_Training.md` (+ 2026-08-16 addendum)
> **Verdict:** **NEGATIVE — not promoted.** `SelectionMode::BetaPosterior` stays the default. The variant + harness are kept as the reproducible negative-result artifact (`gemma2_directions` precedent).

## 1. What was built

- `src/selection_entropic.rs` — the KL-budgeted Boltzmann tilt, modelless + zero-alloc (streaming bisection on β, `r_max`-subtracted exponentials): `solve_beta` (β such that `KL(q_β ‖ uniform) = γ`, paper default `γ = ln 2`), `tilted_weights` (`w_β(i) = n·q_β(i)`, mean 1), `tilt_advantages_into` (`A_β = w_β − 1`, zero-sum). 9 unit tests: shift/scale invariance (paper App. A.1), exact KL budget at convergence, degenerate pools (all-equal → uniform, n=1 → neutral), rare-success upweighting + zero-sum, auto-adaptation (β shrinks as gaps grow), budget-bounded max weight, bit-identical determinism, scratch capacity stability (the honest CountingAllocator equivalent for a pure-f32 module).
- `SelectionMode::Entropic` (opt-in) wired at the ONE outcome-scored pool in the crate: `HealPipeline::heal` Step 4 (candidates blend). Pools ≤ 1 + per-candidate-only call sites degrade LOUDLY to `BetaPosterior` (one `eprintln` via `Once`, mirroring the oracle-degradation pattern).
- `tests/t26_entropic_max_metric.rs` — the T3 harness (LCG-deterministic, 3-repeat determinism pinned).

### T2 wiring-site honesty (issue text vs reality)

The issue proposed wiring "the Stage-2 tuning loop's adaptive pool + the ruliology candidate ranking". Both were file-guess assumptions that do not survive contact:

- `AdaptiveDrafter` is an exact-key **KV cache** (`lookup(rule, buggy) -> Option<fix>`) — no candidate ranking exists to tilt.
- `RuliologySearch::search` ranks by `(lint_satisfied, ast_delta, pareto_rank)` — a deterministic **structural** ranking with no outcome/history axis.

The only genuine outcome-scored pool is `heal()` Step 4. That is where the variant landed; everywhere else degrades loudly.

## 2. The measurement (G4 — the load-bearing gate)

Metric: **discovery within a fixed attempt budget** — `P(≥1 Bernoulli(p) success in B=3 attempts)`, the per-span "best fix found" quantity (the MAX metric, not mean heal rate). 20 000 trials/cell; pools of 2–5 candidates; 50 % rare-good shape (one gem `p ∈ [0.4, 0.9)` buried in `p ∈ [0.02, 0.2)` crowd) / 50 % uniform; thin-evidence start (0–3 pre-observations); evo = noisy view of true `p` at two noise levels. Selector sees only counts + evo — mirroring `heal()` Step 4's inputs exactly.

B=3 chosen after B=10 saturated (~0.99 for every mode — no discriminating power; recorded).

| Arm | discovery @ evo-noise 0.6 | discovery @ evo-noise 0.2 | mean-attempts 0.6 / 0.2 |
|---|---:|---:|---:|
| **BetaPosterior** (default) | **0.9240** | **0.9379** | 1.533 / 1.488 |
| LinearBlend | 0.9170 | 0.9278 | 1.564 / 1.529 |
| Ent-sigmoid (`sigmoid(A_β)`, belief rewards) | 0.9091 | 0.9173 | 1.585 / 1.557 |
| Ent-sig-rate (`sigmoid(A_β)`, raw-rate rewards) | 0.9132 | 0.9226 | 1.575 / 1.540 |
| Ent-maxnorm (`w_β/w_max`, belief rewards) | 0.8770 | 0.8766 | 1.685 / 1.686 |

**All four reward-source × mapping variants lose — to BetaPosterior AND to LinearBlend, consistently across both noise regimes.** The paper specifies no [0,1] blend mapping (it has no evo axis); both principled mappings were measured, plus the raw-rate reward reading, before the verdict.

## 3. Why it loses (the mechanism)

The heal loop's pools are **starved**: 0–3 observations per candidate. In that regime:

1. An unseen true-gem candidate has belief score `ε = 0.05` (the ε-quantile of an empty Beta) — the belief *ranking* the tilt consumes is mostly thin-evidence noise.
2. The tilt amplifies that noise into decisive selection (any belief gap → near-budget-max discrimination) — pushing selection *away* from unseen-but-good candidates.
3. Meanwhile the evo axis is a noisy-but-**correlated** view of true quality. `BetaPosterior`'s moderate reliability swing (~0.27 at W_RATE) leaves room for the evo signal to rescue good candidates; the entropic mappings either cap the swing below the evo axis (sigmoid: ~0.18 — noisy evo overrides proven candidates) or remove it entirely (maxnorm: pool ranking is everything — the worst arm measured).

TTT-Discover's regime is dense reward batches (512 rollouts/step) where the within-batch distribution is well-estimated. The transfer fails at the healer's evidence density — the same class of finding as the `gemma2_directions` GOAT (real signal, wrong regime/strength for the hot path).

## 4. GOAT (G-lite per the issue)

| Gate | Result |
|---|---|
| G1 correctness | 9 solver unit tests + 3 harness tests (incl. budget-protects-against-lucky-1/1, single-candidate degrade) — PASS |
| G2 perf | Selection O(pool) + ~60 streaming bisection iters, zero steady-state alloc (capacity-stability test) — PASS |
| G3 no-regression | 428 default lib / 448 full default suite / 663 `kernel_heal,latent_retrieval` (Issue 017 retrieval floors HELD — corpus untouched) / clippy 0 `--all-targets` — PASS |
| G4 discovery | **NEGATIVE** (§2) — the honest gate outcome; no promotion |

`t93_selection_regret` gate unchanged: BetaPosterior −23.8 %/−8.4 %/+2.9 % at 1/5/30 attempts — the promotion stands.

## 5. Fix in passing

`examples/wgsl_mine.rs` (Issue 025 T6, commit `589b0c9`) was missing its `[[example]] required-features = ["kernel_heal"]` declaration — `cargo clippy --all-targets` (default features) failed E0433 on `domains::kernel_opt`. Declared alongside `wgsl_scan_smoke`. A latent CI break from the previous session, caught by this session's clippy gate.

## 6. Reproduce

```bash
cargo test --release --test t26_entropic_max_metric -- --nocapture
cargo test --lib selection_entropic
```

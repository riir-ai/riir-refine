# Bench 084 — Issue 093 T1: per-run strategy-posterior PoC (NonergodicFilter vs static shape-conditional vs min-delta)

**Status:** RECORD — T1 defend-wrong verdict measured: **the per-run `NonergodicFilter` WINS all three scenarios** (aligned/stale/cold store), paired McNemar significant everywhere, robust to the ±20% misspec pass. T2 (opt-in wiring at the driver ordering seam) is unlocked by the issue's own gate; the honest caveats below bound the claim.

**Date:** 2026-09-12 · **Commits:** this repo: `examples/nonergodic_driver_poc.rs` + feature `nonergodic_poc` (opt-in, measurement-only) + dep `katgpt-micro-belief` (path, opt-in, `nonergodic_belief` forward) · kernel: katgpt-rs Plan 592 `NonergodicFilter<K,D>` (Bench 706, unchanged).

## Setup

- **World:** 240 files × 5 seeds (1,200 paired file-runs per arm per scenario — identical world per arm; the store is the only scenario variable). Sites 2..=4 per file, codes drawn regime-conditionally (E0597-heavy in R3), `max_iters = 5`, P(hard file) = 0.3, cascade 0.2 (a resolve may clear one dependent site — same rule every arm).
- **Scripted outcomes:** deterministic BLAKE3 draws from a frozen (regime × difficulty) emission table per (site, strategy, context-hash). Context = hash of the remaining-site set — the real compiler's verdict on a re-proposed edit depends on the CURRENT file state; same bytes + same context → same outcome (the tried-set's core guarantee), changed context → fresh draw. 10 REAL playbook strategies (borrowck.rs + e0597.rs ids; E0597 carries the rich 6-strategy menu where ordering matters).
- **Arms:**
  - **A min-delta** — the shipped no-posterior ordering (`driver::order_candidates` semantics: line → code → total replacement length → id).
  - **B static-shape** — the REAL `traj_store::shape_conditional_order` over a seeded temp `TrajStore` (400 shapes × 2 events; the shipped BetaPosterior-over-retrieved-neighbors path, static within the run).
  - **C nonergodic** — per-run `NonergodicFilter<K=4, D=2>` over 4 regime ComponentModels (token = `strat*3 + outcome`; inner states {tractable, intractable} with sticky transitions — the world's fixed per-file difficulty as the filter's inner-state model). Uniform prior; ordering by the filter-marginalized predicted resolve probability; revive-on-contradiction: a suppressed candidate retries (once) when the filter's committed regime flips AND its suppression context changed AND `revive_gate() ≥ 0.35`.
  - **D composed** — the production shape T2 would wire: score = (1−λ_t)·B + λ_t·C, λ_t = t/(t+1) over observed attempts (B is λ≡0, C is λ≡1).
- **Scenarios:** `aligned` (store history from the world's true tables — B's home ground), `stale` (same shapes, outcomes from regime-swapped tables R0↔R1, R2↔R3 — the world changed under the store), `cold` (empty store).
- **Defend-wrong instrumentation:** the aligned scenario gives B near-oracle priors; the misspec pass perturbs C/D's model tables ±20% (world unchanged); per-regime + hard-file breakdowns expose which world structure drives deltas.

## Results

### Resolved-within-budget % (1,200 paired files per cell)

| arm | aligned | stale | cold |
|---|---|---|---|
| A min-delta | 35.2 | 35.2 | 35.2 |
| B static-shape | 36.7 | **34.3** (below A) | 35.2 (= A, unseen prior) |
| **C nonergodic** | **41.7** | **41.7** | **41.7** |
| D composed | 41.3 | 38.2 | 39.8 |

C is store-independent by construction (identical across scenarios — the sanity check holds); its paired McNemar vs A is **160/82 in every scenario** (z ≈ 5.0, p < 1e-6); vs B: **146/86 aligned** (z ≈ 3.9, p ≈ 8e-5), **154/66 stale** (z ≈ 5.2).

### Wasted-attempt % (non-resolving attempts / total attempts)

| arm | aligned | stale | cold |
|---|---|---|---|
| A min-delta | 58.6 | 58.6 | 58.6 |
| B static-shape | 56.5 | 61.6 | 58.6 |
| **C nonergodic** | **54.8** | **54.8** | **54.8** |
| D composed | 54.0 | 59.5 | 56.2 |

Mean errors left (aligned): A 1.12 · B 1.10 · C 0.99 · D 1.02. Hard files resolved (aligned): A 7% · C 9%.

### Revive-on-contradiction (the suppression-semantics treatment)

| arm | aligned (helped/hurt) | stale | cold |
|---|---|---|---|
| C | 107 fires (**57/50** — net positive) | 57/50 | 57/50 |
| D | 157 (75/82) | 215 (**78/137** — harmful) | 119 (54/65) |

Scoped to context-changed retries, revive is net-positive for C; under D in the stale scenario the bad store keeps the belief churning and revives amplify the churn — the honest cost of composing with a lying prior.

### Misspec pass (±20% on C/D's model tables, world unchanged)

C holds its entire edge: 41.7% resolved, McNemar 160/82 — bit-identical verdict; wasted 55.1 (vs 54.8). The win does not depend on exact table knowledge: the load-bearing property is the regime winner-SPREAD (0.85 vs 0.25–0.45 across regimes), which ±20% noise cannot cross.

## The honest bounds of the claim

1. **C's model FAMILY matches the world generator family by construction** — the 4 regimes ARE the generators, and the emission-table shape is shared. The misspec pass covers parameter error, not family error. A world with different generator count/structure (or real rustc, whose "regimes" are latent and unnamed) is T2's GOAT to test, on the real cargo corpus via the G6/G7 floor A/B harness.
2. **The toy store is deliberately sparse** (2 events/shape — a young store). B's aligned gain over A is NOT significant at this density (138/120, p ≈ 0.24) — this bench makes NO claim about the shipped ordering at real store densities (its G6-measured benefit stands on its own record); it only establishes that a sparse empirical prior loses to within-run identification with a structurally-correct model.
3. **λ-schedule is uninformed** (t/(t+1)); D under a stale store pays for its early store-trust (38.2 vs C's 41.7). A belief-quality-weighted schedule (or just C + a stale-detection gate) is T2 design space, not measured here.
4. **Defend-wrong check passed in the intended direction**: B does win its home-ground point estimate (36.7 > 35.2) — the harness is not rigged against the baseline — but the toy world's within-run structure is rich enough that 1–2 informative outcomes identify the regime faster than a sparse prior can.

## Mechanism (why C wins within 5 attempts)

Per attempt, the outcome-conditional-on-action likelihood ratio across regimes spans up to ~7× (e.g. scope-hoist failing moves R2 mass by 0.12-vs-0.55); after 1–2 failures the posterior re-ranks the menu toward the true regime's winners, and the remaining budget is spent on high-true-p strategies. The inner {tractable, intractable} state additionally learns per-file difficulty — visible as C's hard-file gain (9% vs 7%) and its lower wasted rate on hard files. This is exactly R135 lesson 2's shape: selection that updates ONLINE from this run's outcomes, vs a cross-run prior frozen at run start.

## Verdict + next step

**T1: per-run NonergodicFilter ordering beats both the no-posterior min-delta and the shipped static shape-conditional ordering on resolved-within-budget (+6.5pp / +5.0pp aligned; paired p < 1e-4) and wasted-attempt rate (−3.8pp), is immune to a stale store by construction, and keeps its edge under ±20% model misspecification.** The revive-on-contradiction semantics are net-positive when scoped to context-changed retries (57/50) — the naive unscoped rule (first harness draft) was measured 0/356-harmful and was fixed BEFORE the recorded run (a frozen-context retry is provably the same bytes → same outcome).

**T2 is unlocked** (T1 win + Issue 91 T1.5 ring landed, `795ddc4c`): wire behind an opt-in feature at the `rustc_errors` driver ordering seam — the T2 GOAT gate (correctness = resolved rate at equal budget on the real 1%-fail surface; no regression) is the family-error test this PoC cannot provide. The T2 design must consume the ring's per-attempt observables and start from uniform (or store-seeded) priors per run.

## Reproduction

```sh
cargo run --release --features nonergodic_poc --example nonergodic_driver_poc           # 3 scenarios
cargo run --release --features nonergodic_poc --example nonergodic_driver_poc -- --misspec
```

Deterministic (two runs byte-identical; SplitMix64 streams + BLAKE3 outcome draws + sorted iteration). Temp stores under `/tmp/riir_nonergodic_poc_*` are removed per seed iteration.

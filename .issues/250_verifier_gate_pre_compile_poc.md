# Issue 250: Verifier-gate shadow-score → decline-guard POC for the fix lane (ITERLOGIC transplant)

**Status:** Open — research-derived POC (riir-refine Research 424, EMNLP 2026 Findings "Iterative Natural Logic Verification for Multi-hop QA"); verdict round 1 corrections applied (compile-label scope, cost model first, G1 per-posture)
**Lease closed:** katgpt-rs-93@amo, 1795118000, was expires 2026-11-22T00:00:00Z — T1a complete (FAIL, Bench 162 addendum); T1b unclaimed
**Lease closed:** katgpt-rs-93@amo, 1795116000, was expires 2026-11-22T00:00:00Z — T0 complete (Bench 162); T1 unclaimed
**Priority:** after the next maintained-binary lane (the cost asymmetry is real but the lane is not red today)

## The idea, scoped to the lane's actual shape

The paper's pattern: score each candidate with a CHEAP graded verifier BEFORE the expensive gate; use the score to order/decline. Two coordinator-verified facts pin what that can mean HERE:

1. `heal` picks ONE candidate per finding (`select_best_candidate` → `candidates[best_idx]`, `src/self_evolve.rs`) before verify — the drafter+memory pool exists but only the selected candidate is ever compiled. There is no next-candidate retry after a verify failure (a reverted edit is recorded unverified, lane ends).
2. `fix_verify::verify(&[FileEditSet])` (`src/fix_verify.rs:316`) batches ALL chosen edits per owning package: one baseline check + compile rounds with per-file revert. "Order within a batch" does not exist — a package's edits compile together.

Therefore the transplant's honest first posture is NOT "order compiles" — it is **measure, then decline-guard**:

- **Shadow-score posture (zero risk, pure evidence):** score every candidate/finding before verify, change NOTHING, and measure offline what a floor WOULD have saved.
- **Decline-guard posture (only if shadow savings are real):** below-floor candidates are not written (skipped pre-verify), counted + disclosed, false-decline bar enforced. The compile gate stays the authority for everything that runs.

## Phases (pre-registered; the stop bars bind)

- [x] **T0 — Cost-model census** — DONE 2026-11-20, [Bench 162](../.benchmarks/162_i250_t0_fix_lane_cost_census.md): NOT DOA. Wasted verifies are 11.3% of verify runs, the perfect-guard ceiling is 26.2% without the one 206-check outlier (66.3% with it), and the derived **G3 bar is ≥ 10% realized compile reduction**. Reverts concentrate in two rules, so T1 should run a per-rule revert prior as the floor's floor. (d) pool sizes are not persisted (deferred to the ladder arm's instrumentation). Original text: **T0 — Cost-model census (CPU, no GPU; produces the G3 number, and can kill the POC).** From trajectory records + a shadow fix_verify instrumentation, measure on the real `.refine/misses` queue: (a) compile invocations per fix run (baseline + rounds, per package); (b) the WASTED-VERIFY rate — runs whose entire batch reverts (baseline + every round spent on a doomed set); (c) per-file revert-round counts; (d) pool sizes per finding (candidates drafted vs the one verified). **Bar-setting rule: the G3 target is DERIVED from (b)+(c), not assumed** — e.g. "a floor at held-out PR-AUC X would have skipped Y% of wasted verifies". **DOA stop condition: if wasted verifies are rare (< ~5% of runs) and revert rounds ≈ 1, the decline-guard's savings are structurally thin and the POC stops here as a recorded negative** — that is a valid outcome, not a failure.
- [x] **T1a — FAIL (Bench 162 addendum): the rule prior catches 17/22 reverts but saves 3.9% of checks (bar 10%; held-out ceiling 22%) and false-declines 2.7% (bar 1%) — per-rule resolution is too coarse; T1b must be per-edit and beat 3.9%. The strata under-counted `written` (Issue 252, fixed at row v2); re-run with v1 semantics: FAIL stands (3.9% / 2.74%).** Original: **T1a — the floor's floor: per-rule historical revert prior** (pre-registered 2026-11-20, katgpt-rs-93, before any number).
  - Data: Bench 162's 164 rows. Unit: the per-run rule strata.
  - Split: FIT on runs with ts < median(ts), EVALUATE on the later half.
  - Policy: decline rule r iff fitted p(r) = Σreverted/Σwritten ≥ τ. τ = 0.5 primary; 0.25 / 0.75 reported. Rules unseen in FIT are never declined. The `(whole file)` stratum is non-declinable (it carries no rule).
  - Simulated cost on held-out verify runs: remaining edits all survive → 2 checks; nothing left → 0; reverts still outstanding → the actual checks.
  - Bars: G3 ≥ 10% realized check reduction (outlier-excluded), AND false declines ≤ 1% of kept edits. PASS ⇒ the per-rule prior is the floor T1b's similarity arm must beat.
- [ ] **T1b data source (noted 2026-11-20, katgpt-rs-93@amo):** this does NOT need to wait for blueprint v2 rows. The `latent_retrieval` trajectory store (`.refine/trajectories.kv`, written by `record_traj` / `record_traj_fail` in `src/bin/heal.rs`) already holds every attempted edit: `src_path`, `rule_id`, `restore`, `replacement`, `success`, `verified`, `run_id`. That is 155 stores (~950 KB) on amo alone. Group by `run_id` to rebuild each run's batch for the G3 simulation; the per-edit success label is the held-out target. Read through `riir_refine::traj_store`, never a hand-rolled KV parse.
- [ ] **T1 — Floor arm FIRST: retrieval-similarity score** (candidate diff vs historical accepted fixes). If the SIMILARITY floor clears the derived G3 bar, the modelless floor ships and T2/T3 never run.
- [ ] **T2 — v1 trained head: BCE linear head over hashed features** (hunk n-grams + token-type features + diagnostic rule one-hot + edit-shape features) — the reflex decision-lane shape. CPU-minutes to train. Doubles as the strengthened baseline.
  - **Label scope (verdict round 1 correction):** labels come from compile outcomes, and compile outcomes see SYNTACTIC failure classes only. The T0 perturbation-labeling set must be built from what the compiler can actually adjudicate (unbalanced delimiters, bad idents, type mismatches, unresolved paths) — **inverted guards / swapped operands usually still compile, so they are NOT labelable by the compile gate** and are excluded from the labeling set.
- [ ] **T3 — v2 escalation DECISION, gated on the pre-registered stop:** v1 PR-AUC ≈ similarity floor within noise ⇒ STOP (modelless wins). Only past the stop: a 20–30M span-pair encoder head for compile-outcome prediction, 1–3 h/run × 3–5 runs + calibration ≈ **10–25 GPU-h on the 4090** via `rig run`. DeBERTa-large rejected (~40–80 GPU-h, buys nothing). Hosting: riir-train trains, ternary `.bits` head ships beside the L1 drafter (the `rule_embed_poc` precedent). laya is the WRONG base (parity-pinned intent-classification checkpoints, not diffs).
  - **Behavior-labeled arm is SEPARATE and optional:** the semantic failure classes (inverted guard, dropped bound) need labels the compiler cannot give — test-suite outcomes as the labeler. That arm carries its own cost (test runs per labeled diff) and its own issue if anyone opens it; THIS issue's heads predict compile outcomes only, and no semantic-class claim is made for them.
- [ ] **T4 — Decline-guard posture (only past T0's derived bar):** feature `verifier_gate`, opt-in, default-off, never in the release set until GOAT; env PROMOTE-ONLY per the env-switch law. Below-floor candidates skip verify (not written), every skip counted + disclosed in the run summary (P104-audit shape). The shadow-score instrumentation ships report-only regardless.

## GOAT bars (pre-registered, verdict round 1 corrections applied)

- **G1 (per posture):** shadow posture — no behavior change, byte-identical run (trivial). Decline-guard posture — the kept set MAY differ from baseline by design (that is the posture); its correctness bar is the **false-decline rate ≤ 1%** (a declined candidate that would have compiled+kept, counted against held-out compile outcomes) plus **fix quality non-inferiority** on the queue (reverts + test outcomes, paired). No byte-identity claim — in decline mode, declining a candidate drops that fix by design; and if a future posture grows the pool past one candidate per finding, score-driven selection legitimately changes what is kept. Reproducibility is pinned instead by a fixed tie rule (score order, first-index break) wherever selection is score-driven.
- **G2:** scoring cost added p50 ≤ 1 ms / p95 ≤ 5 ms per candidate on a quiet box (Bench 144/145 bar posture; `bench_preflight` PROVENANCE line quoted).
- **G3 (the value leg — the number comes from T0, not from this file):** compile-invocation reduction ≥ the T0-derived bar (shadow-measured first, then confirmed armed). If T0 cannot produce a bar ≥ ~10%, the POC is DOA per T0's stop condition.
- **G4:** zero-alloc scoring or disclosed.
- **Floor (Report-the-Floor):** the trained head must beat the similarity score by +0.05 PR-AUC on held-out compile outcomes AND deliver G3, else the similarity score ships and the GPU budget stays unspent. Any confidence/UQ claim additionally clears a split-conformal wrapper (conformal-naive floor law).
- **Regeneration-ladder arm (separate, NOT part of the cost POC):** only if T0 shows pool candidates are being left unverified AND a next-candidate retry posture is wanted — that posture SPENDS more compiles for keep-rate (a quality trade, its own bar); the non-degeneracy falsifier (deterministic diversification must produce DISTINCT candidates) applies before any ladder claim.

## Risks (honest, updated)

1. The lane's cost structure may simply not waste enough compiles for any gate to pay — T0 exists to find that out cheaply, and its DOA stop is the cheapest branch.
2. Compile labels see only syntactic classes; the heads' ceiling is compile-outcome prediction, never correctness or semantics.
3. The floor might already be the answer (T1 deliberately runs first).
4. In-repo precedent base rate for trained-head promotions ≈ break-even (`calibrated_dispatch` NOT-promoted, `set_rerank` refuted vs `rule_embed_poc` gated PASS) — hence the pre-registered stop.
5. If T0's census comes back thin on negatives/rule-id diversity, the honest answer is "not yet" — Phase 0 fails its own gate.

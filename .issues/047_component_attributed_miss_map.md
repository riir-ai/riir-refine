# Issue 047: Component-attributed miss map — E/ρ/W/C taxonomy for heal failures

**Filed:** 2026-08-26
**Source:** [riir-ai Research 347](../riir-ai/.research/347_recuris_verified_goal_ledger_invocation_conditional_memory.md) (Recuris, arXiv:2608.24876 — localization-by-construction + component-scoped memory evolution)
**Repo:** riir-clippy
**Kind:** POC / optimization (post-mining improvement — feeds the self-evolve loop)

## Problem

The healer records failures but never attributes them to a *component*. Three failure streams already land on disk with no component tag:

1. `.heal/misses/` — spans where no rule fired or the wrong rule won.
2. Memory trajectories with `success=false` — verify-gate reverts (compile breaks).
3. Created-warnings / score-bench deltas — fixes that introduced new diagnostics.

Recuris's measured finding (their Table 4): failure localization from structured traces reaches **64.8%** vs **13.0%** from outcome alone — and the gain is concentrated exactly on the faults a transcript cannot show (a non-fired invocation is invisible in output; only the *decision-time record* shows it). Our own precedent: **Issue 024** (cross-domain switch-cost) was a hand-run component localization — the correct rule was crowded to raw rank 9 by loop-shaped rules (a ρ-class failure), fixed by `RERANK_POOL` 8→12, verified 0.0pp both directions afterward. That investigation took a full session; the taxonomy below would have flagged ρ from the recorded artifacts alone.

## Proposal

Tag every recorded failure with exactly one component class (the Recuris E/W/ρ/C ontology, healer-translated):

| Class | Healer meaning | Existing evidence |
|---|---|---|
| **E** (corpus) | rule missing / buggy_code fixture wrong / proposer lacks the lint shape | miss with zero plausible candidates; fixture that fails its own proposer |
| **ρ** (retrieval/invocation) | right rule existed, wrong rule won — crowding, domain mis-route, pool depth, rerank miss | miss where the correct rule appears outside top-1 but inside top-k (`retrieval_eval` miss taxonomy already names crowding/coverage classes); cross-domain pairs |
| **W** (drafter context) | right rule retrieved, wrong fix drafted — context/annotation starvation, span state insufficient | parse failures; rule fired but edit reverted as semantically wrong (not compile-broken) |
| **C** (verifier/gate) | fix was good but gate rejected / fix was bad but gate passed | created-warnings (gate too loose); unnecessary reverts (gate too strict) |

Then surface the per-class counts in `self_evolve_audit.sh` §1b alongside the ELO leaderboard, so the *next* improvement targets the dominant component instead of guessing.

## Tasks

- [x] T1 — Schema: add `component: Option<ComponentClass>` to the miss record + trajectory failure records (backwards-compatible decode = None).
- [x] T2 — Classifier: derive the tag at record time from existing signals (rank of correct rule if known from eval fixtures; parse result; revert reason; created-warning presence). Zero-LLM, deterministic.
- [x] T3 — Retro-validation (the POC gate): re-analyze the Issue 024 switch-cost artifacts + the Issue 027/028 false-positive clusters under the taxonomy; confirm the classes they were eventually hand-diagnosed as (ρ, E-proposer-shape, C-comment-guard respectively) are what the classifier emits. If the classifier cannot reproduce those three known answers, the taxonomy is wrong — fix before shipping.
- [x] T4 — Audit surfacing: per-class counts + top representative spans per class in `self_evolve_audit.sh`.
- [-] T5 — (deferred `[-]` unless T3 passes cleanly) anchor-gated patch admission: when a proposer/retriever change lands, auto-run the score-bench frozen fixtures as the no-regression half of a Recuris-style gate (repair ∧ no-anchor-regression) instead of the current manual run-after-change convention.
  **T3 gate note (2026-08-26): T3 PASSED clean** — `src/component.rs::tests::{retro_issue_024_pool_crowding_is_rho, retro_issue_027_shape_false_positive_is_e, retro_issue_028_comment_guard_is_c}` reproduce all three hand-diagnosed classes from the incidents' recorded facts (Bench 032's raw-rank-9 crowding measurement; the expression-wildcard oracle probe; the comment-displacement observation). The T5 unblock condition is met; the defer stands on scope (the anchor-gate wiring is its own unit — wire the score-bench runner into the proposer-change workflow, no new taxonomy work needed).

## Non-goals

- No LLM Meta-Agent — the classifier is deterministic pattern-matching over recorded artifacts (the paper's Meta-Agent is replaceable wholesale per their §3.4.3; our fixed procedure is the auditor script).
- No new retrieval mechanics in this issue — ρ-class counts are the *input* to a future tuning decision (pool depth, fan-out weights), not an automatic parameter search.

## Close condition

T1–T4 merged with T3's retro-validation passing; note the outcome in Research 347's consumer line.

## Implementation record (2026-08-26)

**T1 — schema (backwards-compatible everywhere):**
- `src/component.rs` (NEW, unconditional — a taxonomy gate must not be feature-skippable): `ComponentClass` (E/rho/W/C, serde tags `"E"`/`"rho"`/`"W"`/`"C"`), `RevertKind` (CompileGate/Infrastructure), `FailureSignal` (the decision-time record: expected rule/rank, candidates, parse result, revert reason, created warnings, oracle-unresolved, lint-absent-before, drops-source-content) + `classify()` — the deterministic precedence tree: verdict-stage (C: created/unresolved/infrastructure-revert/content-drop FP; E: shape FP) → W (compile-gate revert, parse failure) → retrieval-stage (ρ: outranked or crowded-past-probe; E: nothing answered).
- `memory.rs`: `FixTrajectory.component: Option<ComponentClass>` (`#[serde(default, skip_serializing_if)]` — pre-047 envelopes decode None; untagged serializations stay byte-identical so existing BLAKE3 commitments hold) + `LatentFixMemory::tag_component`.
- `traj_store.rs`: `TrajRowDoc.component` (same serde pattern) + `TrajStore::record_classified(.., component)`; `record` delegates with `None` (zero churn on existing callers/tests); successes leave the last tag in place.
- `domains/rustc_errors/driver.rs`: `MissRecord.component` (E by construction — no-candidate miss).
- `draft/retrieval_eval.rs`: eval `MissRecord.component` (the living ρ/E surface — carries ground truth the production paths lack).

**T2 — record-time classifier wiring (never post-hoc):**
- `recorder.rs`: `EvolveRecorder::record_failure_signal(.., &FailureSignal)` — classifies, records the failure, tags the bucket, returns the class for mirroring (classifies even when opted out).
- `bin/heal.rs`: whole-file + span compile-gate reverts → W; kept-but-oracle-unresolved → C (`gate_passed_ineffective`); `record_traj_fail` derives W internally; the rustc_errors store path classifies by `RevertReason` (NoProgress→W, Io→C).
- `domains/rustc_errors/evolve.rs`: `record_run` reverts classified by reason; `write_miss` emits the component line into the authoring-queue markdown.
- `retrieval_eval.rs`: misses classified at construction from (expected, rank, top1) — the eval report now prints per-class counts (`component counts: E(corpus)=0  rho(retrieval)=4` live at landing).

**T3 — retro-validation (the POC gate): PASSED clean.** Three tests in `component.rs` reproduce the hand-diagnosed classes from the incidents' recorded facts:
- Issue 024 / Bench 032 → **ρ**: expected rule outside the top-8 probe with candidates present (the raw-rank-9 crowding measurement), plus the surfaced-but-outranked variant.
- Issue 027 → **E**: oracle-silent pre-fix source + content-preserving rewrite (the expression-wildcard shape gap).
- Issue 028 → **C**: oracle-silent + content-dropping transform (the comment-guard validation gap).

**T4 — audit surfacing:** `self_evolve_audit.sh` §1d "Component-attributed failures" — per-store per-class counts + top-3 representative (rule, shape, fail-count) per class from `--dump-trajectories`; envelope reader in §1 prints component counts when present. Live at landing: all stores report 0 attributed rows (honest pre-047 state; tags accumulate as new failures record).

**Validation:** lib tests 692 (default) / 761 (rustc_errors) / 791 (fix_verify) / 815 (score_bench) all green; clippy 0 warnings in default/fix_verify/rustc_errors lib + fix_verify bins + score_bench all-targets; `store_backend_e2e` (2/2, the real-binary flow) + `fix_compile` integration (6/6) + the Issue 015 e2e revert fixture all pass; audit script `sh -n` + full-run §1d verified; retrieval-eval regression gate green with the component counts printing.

**Non-goals held:** no LLM, no new retrieval mechanics — ρ counts are inputs to a future tuning decision. Score-bench created-warnings map to C by the classifier (`created_warning()` signal); wiring the per-fixture print is left to the bench's own surface (the aggregate `created_rate` already carries the axis).

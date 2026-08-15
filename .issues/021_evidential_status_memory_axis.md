# Issue 021: Evidential-status axis for `LatentFixMemory` (retelling-bias defense)

**Date:** 2026-08-15
**Research:** [riir-clippy/.research/049](../.research/049_Long_Horizon_AI_Research_Evidential_Status.md) (arXiv:2608.11195)
**Feature:** `self_evolve` (existing, opt-in)
**Priority:** P2 — hardens the self-adaptive track against a documented real-world failure mode before any default-on push

---

## Problem

`FixTrajectory` (`src/memory.rs` L35–51) records outcomes as `success_count`/`fail_count` u32s fed by a boolean `record_outcome(success)`. Nothing in the schema distinguishes:

- a fix that passed the **full validator** (`clippy_verify` / `rust_perf_verify` re-run), from
- a fix that only passed the **syn-parse gate** (heuristic), from
- a fix that was **retracted** after later failures.

Retrieval (`retrieve_into`) ranks by latent+BM25 similarity only — no trust axis. This is exactly the schema that arXiv:2608.11195's session-44 incident predicts will fail under iterated retrieval + resummarization: an early heuristic success launders into "certified" as caveats fail to survive compression (their case: an uncertified numerical score held the record for 25 days; a proved lemma was lost and reproved from scratch). The paper's name for it: **retelling bias** — headline values survive retelling, evidential status doesn't.

Secondary gap (same root cause): no **scope-of-validity** — a fix validated on one fixture family is retrievable for any span (the paper's "restricted closures recorded as global progress" recurring error).

## Proposed fix

1. **`EvidenceTier`** — `#[repr(u8)]` enum on `FixTrajectory` (zero-alloc, one byte):
   - `Certified = 0` — admitted by the full validator (admission rule already exists: `adaptive_drafter.rs` verified-fix pool keyed by `(rule_id, buggy_hash)`).
   - `Heuristic = 1` — syn-parse only (constructor default; back-compat: existing serialized memories decode as Heuristic, which is the honest tier for unverified history).
   - `Withdrawn = 2` — absorbing state after `WITHDRAW_CONSECUTIVE_FAILS` (suggest 3) consecutive post-hoc failures. **Monotone by design** (riir-chain slashing-monotonicity pattern): Withdrawn never returns to Heuristic/Certified except via an explicit re-certification event (a fresh full-validator pass), never via outcome counts alone.
2. **Trust-weighted retrieval** — blend tier into `retrieve_into` ranking as a sigmoid-gated prior (Certified 1.0; Heuristic 0.5-neutral, matching the existing untried-neutral convention; Withdrawn excluded from suggestions but still retrievable for the adaptation loop's failure analysis). Sigmoid, never softmax.
3. **`record_outcome` signature** — takes the verification context (`verified: bool`) alongside `success: bool` so the tier transition is a pure function of (tier, outcome, verified). Keep G1 determinism: transitions depend only on recorded events, no wall-clock.
4. **Scope-of-validity (stretch)** — u8 bitset of fixture-family ids the tier was earned on; retrieval down-weights cross-family use. Defer if it complicates the schema review.

## Gates (POC first — this is a behavior claim, needs empirical proof)

- [ ] **T1** Schema + transitions land behind `self_evolve` (no new feature flag needed — the feature is already opt-in); `EvidenceTier` transitions unit-tested incl. the absorbing-Withdrawn property (a `proptest`/loop test: no sequence of outcomes un-withdraws without re-certification).
- [ ] **T2** POC fixture: seed memory with (a) a Certified entry and (b) a Heuristic entry with equal/higher similarity score; assert the Certified entry outranks (b) in `retrieve_into`, and a Withdrawn entry is never emitted as a suggestion. This is the falsifiable A/B — if ranking doesn't move, the trust axis is wired wrong.
- [ ] **T3** G3 no-regression: full `self_evolve` suite (G1–G7) re-passes; existing 201 default tests untouched.
- [ ] **T4** G4 alloc: tier check adds zero allocation (u8 field compare); bench or review-audit.
- [ ] **T5** Note back-ref: close-out line in `.research/049` when done; remove this issue per noise-reduction rule.

## Non-goals

- No change to `adaptive_drafter`'s pipeline-level verified pool (orthogonal layer, stays).
- No game-side (riir-ai NPC memory provenance) work here — that's a separate substrate-first pass noted in Research 049 §2.3.

# Issue 067: growing-input conditioning reopen — condition-first placement + same-rule gating (arXiv:2609.02702)

**Status:** OPEN — gated on the T1 trigger (same-rule exemplar availability in the TrajStore). No code changes until it fires.

## Context

riir-clippy Bench 051 (Issue 049, closed 2026-08-27) measured the growing-input conditioning POC: arm G appended trajectory outcomes + failure patterns AFTER the code module (`cf_workers_ai::prompt_growing`). Verdict `NEGATIVE-no-gain` — strict-keep tied 5/9 across arms A/B/G at 2.8× arm-B tokens — and the channels were **cross-rule** by construction (the reachable lints are off-corpus, so no same-rule history existed anywhere).

katgpt-rs Research 530 (arXiv:2609.02702 "Trace as State", Zou & Tang 2026) adds two falsifiable deltas that recalibrate that negative without overturning the closed verdict:

1. **Placement.** For causal processors, conditioning/derived state placed BEFORE the context dominates placing it after — 26/27 model×task×metric cells, up to +38.8 EM, with a worst-case exponential memory argument (`[z,C]` = `b` bits vs `[C,z]` = `b·2^b` bits). Arm G's channels were condition-LAST relative to the code (the paper's controlled variable). At the healer's short prompt scale the expected effect is small, but the paper's Question-First and Re2 arms show re-anchoring gains even at 256K with far weaker traces than same-rule exemplars would be.
2. **Relevance hazard.** The paper's Random-Trace arm (traces from other problems, prepended) scores BELOW the no-trace first pass (Parents EM 14.2 vs 29.2) — weakly-relevant conditioning content is actively harmful, and its trace-count ablation is monotone only for SAME-problem traces. Bench 051's channels were exactly the weakly-relevant class: the negative may be relevance-poisoned (positive channel × harmful content cancellation), not evidence against the channel design.

Design corollary: the paper's Answer-Feedback arm (45.4 EM) vs full-trace arm (81.8 EM) says process evidence conditions better than outcome evidence — relevant to how future channels should quote trajectories (keep the reasoning/patch narrative, not just before/after pairs), when same-rule content exists.

## Tasks

- [ ] T1 — trigger instrumentation: define the reopen condition as "the TrajStore holds ≥K (K=3) Certified same-rule trajectories for a queried lint"; measure store density at heal time (one counter; cheap). Log when the trigger first fires.
- [ ] T2 — arm G' construction (only after T1 fires): same-rule-only exemplar channels (cap 3, Certified-first), **condition-first placement** — channels before the code module — reusing `prompt_growing`'s splice mechanics and keeping arms A/B byte-identical as controls. Empty-channel degradation must stay byte-identical to arm B (the existing test's contract).
- [ ] T3 — A/B on the Bench 049 harness (reachable-miss set + strict-keep + never-wrong filters), CF arm. Pre-registered verdict: G' beats B iff strict-keep ≥ B + 1 with zero kept-wrong, at token cost ≤ 2× B. A second negative closes this class for the short-context healer permanently; the placement principle then applies only to long-context surfaces.
- [ ] T4 — record in a new `.benchmarks/{NNN}_*.md` (next free number at write time), cross-linked to Bench 051 and katgpt-rs `.research/530`; close this issue per the noise-reduction rule.

## Refs

- Bench 049 (harness + arms A/B verdict) · Bench 051 (the negative being reopened) · katgpt-rs `.research/530` (paper distillation + signal-diff) · `src/cf_workers_ai.rs` (`prompt_growing`, L404–453) · `src/bin/fixer_regate.rs` (`--arm G`).

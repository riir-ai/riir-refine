# Plan 392: Mining-Queue Epiplexity Allocation + Domain Saturation Gate

**Status:** OPEN — filed from katgpt-rs Research 685 + riir-refine Research 426; GATED on the P0 mix-variation census; consumes katgpt-core `epiplexity_gain` (katgpt-rs Plan 641) — consume, never fork.
**Claimed:** zed-m3-p392census@m3, 1795182273, expires 2026-11-22T13:44:18Z
**Priority:** fusion ladder #1 (self-evolve product stack).

## Premise

The mining queue (`.distill/001_mining_queue_snapshot.md`) orders rule-batch mining by hand/history. The epiplexity signal orders it by predicted structural gain: headroom (L̂−ε) × exposure × influence, fitted GD-free from score_history.jsonl's per-corpus-generation accuracy curves; ε-floor per domain = the saturation gate ("stop mining this domain"). The paper's evidence (beats ADO; epiplexity↔OOD r=0.88) is training-side; OUR value bar is the G2 cell below and is UNPROVEN.

## Phase 0 — census (BLOCKING, hours, CPU-only)

- [ ] Mix-variation census over score_history.jsonl + batch landing logs: per-domain batch counts + accuracy points per generation; compute effective rank of the exposure matrix. **Verdict decides the fit family:** rank ≥ K−1 → cross-domain γ fits; under floor → univariate per-domain (headroom×exposure survives, influence does not) with a typed Sparse record. Write the census numbers into this plan before Phase 1 starts.

## Phase 1 — consumer wiring (behind a demo/measurement binary, not the production fixer)

- [ ] `examples/mining_gain_census.rs` (or extend the existing census lane): fit + rank domains by sigmoid-tempered marginal gain each corpus generation; print the ordered queue + per-domain ε-floor status; REPORT-ONLY (exit 0, never changes the queue) until G2 passes.
- [ ] Saturation gate: a domain whose predicted remaining gain Ŝ(∞)−Ŝ(t) sits under the bar is printed DEMOTED — the Pile-saturation law as a queue gate; if one domain dominates the area integral, print SKIP-SELECTION (the allocation machinery refuses to run).

## Phase 2 — GOAT cell

- [ ] **G2 allocation value:** resolutions per mining-hour, gain-ordered queue vs incumbent queue order, over ≥2 corpus generations of REAL mining (paired, one frozen read, LB95 > 0; state box state per the bench_preflight law). **FAIL ⇒ demote in-landing**: the lane records a DONE-NEGATIVE and the queue stays incumbent (Research 591 precedent quoted verbatim in the verdict).
- [ ] **Admission canaries (collapse taxonomy):** for any mined-batch admission gate touched by this lane — a repetitive batch (n-gram rep ≥ ~0.4-class) and an off-manifold batch are planted REFUSAL arms; PPL-only admission is a documented negative import (the paper's rep-0.47 / frozen-scorer-ppl-169.7 fixtures).
- [ ] **G3/G4:** report-only lane (feature-off byte-identity by construction); fit cost ≤ the ~12 s-class budget; zero steady-state alloc.

## Phase 3 — promotion (only after G2)

- [ ] Wire the ordered queue into `.distill/001_mining_queue_snapshot.md` ordering as the DEFAULT lane header (the queue file gains a `gain:` column); demote switch stays documented. Warm-start gate: the gain column arms only on a corpus with a live Elo floor.

## Boundary + laws

- Consume `katgpt-core::epiplexity_gain` (katgpt-rs Plan 641) — no local re-implementation (goat-audit direction).
- Allocation surfaces only: this signal never touches scan/fix/serving paths (stationary-surface ban).
- The guide of record is `riir-refine/.research/426_Epiplexity_Guided_Mining_Allocation_Guide.md`.

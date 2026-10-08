# Issue 156: State-level rescue-trajectory mining filter (b0=0 ∧ bd≥3)

> **Status:** OPEN — PoC/mining task (healer corpus quality); no code landed yet
**Date:** 2026-10-08
**Research:** [katgpt-rs/.research/609_FlyBy_Execution_Knowledge_Bottleneck_Gate.md](../katgpt-rs/.research/609_FlyBy_Execution_Knowledge_Bottleneck_Gate.md) §2.6
**Source:** arXiv:2609.34327 (FlyBy) Algorithm 1
**Consumer plan:** katgpt-rs Plan 621 T5.2 (consumes `ProbeEstimate` once `state_probe` lands)

---

## Context

The healer's paired local-vs-expert measurement exists (arm-H in `src/bin/fixer_regate.rs`: per-case `keep_strict` modelless vs escalated → wins/losses → cap edit) but emits **governance, not a corpus**. `--misses` / `--frontier-report` mine at **rule/span** level. FlyBy Algorithm 1 mines **states**: retain state `s` iff `0/4` no-tool continuations succeed AND `≥3/4` tool-assisted continuations succeed at the shallowest sufficient depth — a state-level measurement of "this is a knowledge bottleneck with a known rescue."

## Relationship to the network-oracle agreement VETO (explicit, per verdict round 2)

**AUGMENT, never replace.** The veto (cross-model agreement, e.g. openthai) remains the acceptance **AUTHORITY**; the `b0/bd` predicate is a **LOCAL pre-filter that can only narrow** the candidate set entering the veto lane — it can never widen acceptance. Rationale: the veto's signal is *independent* (cross-model) evidence, while `b0/bd` is *same-model self-consistency* — a rescue the policy itself produced is weaker evidence than an independent oracle's agreement. A candidate passing local but failing the veto **stays rejected**. Default posture: **veto primary, local predicate pre-filter.**

## Tasks

- [ ] **T1** Extend the arm-H lane to record per-state `b0`/`bd` tallies (the paired continuations already exist there — this is bookkeeping, not new sampling).
- [ ] **T2** Emit the rescue exemplar set `(state, query_span, shallowest_sufficient_depth)` into the `traj_store` Certified-tier shape (no-fix fails, fix verifies = the existing evidence vocabulary).
- [ ] **T3** Convert the measured population into `--misses` queue ordering evidence (frontier band): rescue-rich states are the learnable-hard band, states with `bd=0` are intractable-eviction candidates (RegretMemory's absorbing class).
- [ ] **T4** reflex synth-lane consumer: local predicate as PRE-FILTER before the openthai agreement VETO (augment; veto stays authority).
- [ ] **T5** G1 truth table pinned: `(0,4)/(0,3)` keep · `(1,4)` reject (control succeeded = execution-reachable — FlyBy's own V>0 law) · `(0,2)` reject (rescue undemonstrated); same transcripts → identical keep-set hash (BLAKE3).
- [ ] **T6** Record measured yield vs FlyBy's 95/800 in the landing note; ref commit hashes here and into Research 609 §2.6.

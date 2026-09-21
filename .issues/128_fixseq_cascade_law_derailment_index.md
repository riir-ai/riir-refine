# Issue 128: Fixseq Cascade Law + Derailment Index — measure the long-horizon failure statistics on repair-driver rings

> **Status:** OPEN
> **Source:** [Research 200](../.research/200_traverse_scout_first_mistake_localization.md) — arXiv:2609.17930 (Traverse/Scout). The two-conditional cascade law is unpublished for repair-driver attempt sequences (prior-art sweep in the note §5). ⚠ **Premise measured 2026-09-21: the `.heal/fixseq/` ring WRITER ships in the driver, but the store is EMPTY on this box** (zero `run-*.jsonl` under every workspace repo checked) — T1 is a census that REFUSES on empty, and T5 corpus-generation is the actual entry cost, not a fallback.

**Filed:** 2026-09-21

## Why

The Traverse paper's deterministic statistics (no judge, no training — counting over labeled steps) map 1:1 onto the `.heal/fixseq/run-*.jsonl` ring format, whose labels are *compile-oracle-emitted* (strictly more deterministic than the paper's human annotations) — the format exists in code (`horizon_report.rs` reads it; `evolve::fixseq_dir` writes it); the DATA does not exist yet on this box and must be minted (T5) or found on another box (T1). Three statistics land on it:

- **Cascade law:** P(attempt_{i+1} bad | attempt_i bad) vs P(attempt_{i+1} bad | attempt_i good). Paper (agents that barrel ahead): 13.7×/10.9× jumps. Our driver *reverts* bad attempts — a mechanical cascade-breaker — so the honest hypothesis is the law is WEAKER here, and a null is a real, publishable-shaped result about the keep-rule's design.
- **Derailment index:** first-wrong-attempt under an explicit badness indicator + ±k tolerance curves (paper: ±3 nearly doubles localization).
- **Silent-failure rate:** % of failed runs whose final attempt looks good (kept, errors non-increasing) — the driver-side twin of the katgpt-rs green-zero law.

Three deferred lanes may unblock on the same corpus — with the honest caveat that minted rings are SYNTHETIC: they unblock `horizon_report`'s standing refusal directly; whether they satisfy the Issue-093 (organic multi-revert) and choice_scorer (organic rings / oracle-anchored real-repo spans) reopen triggers is an owner adjudication, not a given.

## Scope guard

REPORT-ONLY first. Nothing enters the driver hot path, no ordering/policy change, no feature promotion — ordering-family interventions are the closed-negative territory of Issue 093 until its reopen triggers fire. Output is statistics + a `horizon_report` feature extension (report lane, ungated per the frontier/horizon-report law).

## Tasks

- [ ] **T1 — Ring census.** Count and classify `.heal/fixseq/run-*.jsonl` rings available on this box (organic runs vs bench-generated); record per-ring attempt counts, revert counts, multi-revert runs (≥2). REFUSE-on-empty per the frontier-report law — never a confident green zero over no data.
- [ ] **T2 — Badness indicators.** Define per-attempt "bad" from ring fields, three variants, all reported: (a) `outcome == reverted`; (b) `errors_after > errors_before`; (c) `masking_after > masking_before`. No single-indicator cherry-pick.
- [ ] **T3 — Cascade law.** Compute both conditionals per ring, pooled with Wilson 95% CIs; split by corpus class (interacting vs solo, terminal vs SWE-analog). Gate: the law reproduces across two disjoint ring corpora, or the non-transfer is recorded as the result.
- [ ] **T4 — Derailment + silent failure + root/cascade attribution.** First-wrong-attempt index under each indicator; ±k tolerance curves; silent-failure rate; root-vs-cascade attribution using `masking[b,a]` as the root signal (E0614 = root family; borrowck-blocked descendants = cascade). Feed per-family first-mistake counts to the `--misses` queue shape (consume only if 093 reopen triggers fire).
- [ ] **T5 — Corpus generation (expected MANDATORY: the 2026-09-21 census measured the store empty).** Mint labeled multi-attempt rings at scale via the score-bench generations harness + `corpus_gen::compose_multi_strided` (real clippy oracle labels, no humans, CPU-side + clippy runs). Record counts + BLAKE3 manifest per house data discipline. Two DIFFERENT generation configurations (see T7's disjointness constraint) — not two seeds of one config.
- [ ] **T6 — `horizon_report` extension.** Add the prev-attempt-outcome indicator to the 9 ring-native features (feature #10); re-run lead-k AUC with/without (the question: does the transition feature raise the horizon?); keep the refusal discipline intact.
- [ ] **T7 — GOAT gate.** (G1) the law ± Wilson CI reproduced across two DISJOINT ring corpora — **disjointness = two different generation configurations, or a generator/organic pair; two seeds of one composer do NOT satisfy it** (that measures the generator's revert policy, not the driver's). A law reproduced only within one generator is recorded as **generator-conditional**, never as a law about the driver; honest non-transfer recorded either way. (G2) single-pass O(n) counters, zero-alloc, report-only; (G3) driver/fixpoint paths byte-identical (nothing wired); (G4) positional-uniformity guard on first-wrong positions (KS vs point-mass — blocks "just check attempt 3" shortcuts from claiming gains).

## Non-goals

No strategy-ordering change, no escalation controller, no revive-gate retuning, no trained verifier (that lane lives in riir-train Plan 342's addendum, owner-gated on GPU windows).

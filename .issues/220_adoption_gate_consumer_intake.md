# Issue 220 — T1 consumer intake: adoption/retention regression gate (riir-refine side)

**Status:** OPEN — intake issue for Plan 633 T1 (katgpt-rs). The gate primitive + fixture are LANDED katgpt-rs-side; this issue is the riir-refine consumer wiring, filed per katgpt-rs `.issues/987_cactus_neurips2026_follow_ups.md` priority-1 (AGCL form, Research 657 §A5) and katgpt-rs `.plans/633_adoption_retention_regression_gate.md`.

## What exists (producer side, landed `cf65c58a7` 2026-11-08)

`katgpt-core::adoption_gate` behind the opt-in feature `adoption_gate` (zero new deps, modelless):

- `OutcomeDigest` — blake3 `[u8; 32]` newtype over an accepted outcome or a produced output.
- `DedupRing` — fixed 32-slot FIFO membership set (1064 B, layout-pinned); the local mirror of riir-neuron-db's `RingRow<T,32>`.
- `Decontaminator` — never-double-file stride-5 splitter (held-out audit sink vs gate window).
- `score_adoption` / `AdoptionGate::decide` — score `= (1−α)·retained + α·(1−repeated)`, f64 sequential folds, ties→challenger, `NoEvidence` → keep incumbent (decline-first), always-from-base purity pinned bit-for-bit.
- 9 fixture tests green; clippy `-D warnings` green at both feature postures.

## What this repo consumes (the T1 wiring, ungated design sketch)

1. **Active-set changes** (`--active-set-demote` / `--active-set-readmit`, Issue 102 machinery): a demotion that later turns out wrong (the demoted rule's lints re-fire and again get fixed by hand or by another rule) is exactly the AGCL regression class. Wire `AdoptionGate::decide` over the outcome stream = (rule, fix verdict) rows: on demote/readmit, score the NEW active set against the previous window's accepted outcomes (fixes that compiled+verified) and rejected outputs (regressions re-fixed). The `corpus gap`/`misses` queues already carry the raw outcome stream; `RingRow`-shaped windows live in the latent store (`latent_retrieval`).
2. **`seat_rerank` head swaps** (Plan 202 R3a, ARMED on the dev box): when a hosted rerank head replaces the modelless pick set, gate the swap on retention of previously-accepted fixes + non-repetition of previously-rejected ones. The escalation manifest row is the natural config home (α + stride + window size ride the row, demote-only envs per the env-switch law).
3. **Consumer-side feature**: a katgpt-core feature forward `adoption_gate` re-export (the `elo_k_eff`/`step_noise` forward precedent — forward, never fork; goat-audit watches this edge). Naming per house law: the consumer feature stays `adoption_gate` (same name, forwarded).

## Bars (pre-registered now, before any wiring)

- G1 (correctness): a seeded synthetic outcome stream replays through the consumer seam bit-identically to the producer fixture verdicts (the purity pin carries over — same inputs, same verdict).
- G2 (no regression): gate-OFF byte-identity — with the consumer feature off, active-set demote/readmit + seat_rerank behave exactly as today (the `elo_k_eff` feature-off byte-identical posture).
- G3 (the GOAT, T3 in plan 633): gate-on vs gate-off on the REAL store — fix trajectories + oracle verdicts; α under judge noise measured (compile-pass/oracle verdicts are noisy judges — the AGCL simulated-perfect-user gap, inherited honestly); may fail honestly — a verdict, not a bug.
- ⛔ No promotion talk before G3; `decline` (keep incumbent) is the default answer forever.

## Refs

- katgpt-rs `.plans/633_adoption_retention_regression_gate.md` (T1 = this issue) · katgpt-rs `.issues/987` priority-1 row · Research 657 §A5 (AGCL)
- Producer precedent: riir-refine 632 `elo_k_eff` (consumer of katgpt-core `step_noise`, `280f15766`)
- The never-double-file decontamination answers the never-double-file decontamination risk the active-set ledger never had a guard for.

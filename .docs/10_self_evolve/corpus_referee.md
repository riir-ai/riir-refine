# The c′ corpus referee — telemetry before/after + the structural-identity certificate

**Status:** STANDING (shipped; Issue 209 closed 2026-11-07 — code T1–T5 `a6eb6be0` + `a7307895`; the case reads below accrue passively, never a task)

The referee replaces the fixture score-bench A/B as the evidence source for
**single-rule corpus-note edits**, where the affected stratum cannot reach
n ≥ 30 on a finite fixture set (Issue 206's structural UNDECIDED). Batch and
multi-rule arms keep the fixture A/B (paired LCB95 > 0, n ≥ 30).

## The two halves

1. **Structural-identity certificate** (`src/corpus_binding.rs`:
   `BenchIdentity`, `CorpusBinding::with_bench_identity` /
   `verify_bench_identity`, domain `riir-refine/bench_identity/v2`). BLAKE3
   of the gen-N atom tables + healed output, baseline vs note-spliced. Bytes
   identical → the edit provably does not touch the bench axis. Bytes differ
   → the fixture bench runs as usual. An ABSENT cert is not an axis move.
2. **Telemetry referee** (`src/corpus_referee.rs`, ungated store-free state
   machine over `AttemptRow`; the store mapping is the one
   `latent_retrieval`-gated seam). Rule R's attempts split at the edit
   timestamp; bar = Bonferroni-split Beta bound on the failure-rate drop
   (`best_belief_score(pre, 0.025) − best_belief_score(post, 0.975) > 0`) AND
   the drop exceeds the co-epoch control drift (median Δ over other rules with
   evidence on both sides — the DiD gate). Needs **n ≥ 30 post-edit attempts**.

Verdict states: **PASS / FAIL / PENDING / UNDECIDED-by-starvation** (8
observed corpus batches without reaching n = 30). PENDING and UNDECIDED are
"no claim" for every consumer — never folded into a pass.

## Operating it

```bash
# rebuild first — a bin predating --referee silently lacks the verb
CARGO_TARGET_DIR=/tmp/referee cargo build --release --features latent_retrieval --bin corpus_diagnose
/tmp/referee/release/corpus_diagnose --referee                 # all enrolled cases
/tmp/referee/release/corpus_diagnose --referee <rule>          # one case
/tmp/referee/release/corpus_diagnose --enroll <rule> --edit-epoch <8hex> \
  --edit-ts <unix> --pre-attempts N --pre-fails M [--batches K]  # seed a case
```

The read is REPORT-ONLY over the stores but persists the batch counter (live
corpus epoch vs the stored `last_seen_epoch`) in the machine-local ledger
`.refine/corpus_referee_enrollments.json` — run it once per corpus batch.

## Enrolled cases

| # | rule | edit epoch | edit ts | pre snapshot | last read |
|---|---|---|---|---|---|
| 1 | `hashmap-with-capacity` | `ac280021` | 1793793600 (2026-11-04T12:00Z) | 9 fails / 18 attempts (compile bucket, 3 stores — biased toward the pre rate, conservative against PASS) | 2026-11-07: **PENDING (BelowSupport)** — 2/30 post-edit attempts, batches 3/8, controls 8 (15 stores / 1,557 rows) |

When a case reaches PASS/FAIL (n ≥ 30) or UNDECIDED-by-starvation (8/8
batches), record the verdict in this table + HISTORY.md. Nothing else waits on
it.

## Arm B (deterministic demote-execution) reopen trigger

Re-opens when a **C-class candidate appears at the pinned predicate in any
live store**. Arm B is verified by the score-bench A/B at its own bar (demotion
touches the fixture axis directly) — NOT by this referee.

## Scope laws

Probe-derivable vocabulary only (templated `context_notes`; demote-execution).
REPORT-ONLY — the referee never writes the corpus. Modelless throughout.

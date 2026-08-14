# Issue 017 — Retrieval precision eval at scale (the 12-fixture gap)

**Filed:** 2026-08-15, out of the improvement-priorities session.
**Status:** OPEN.
**Type:** proof/eval infrastructure (retrieve axis of the flywheel).

## Background

The Plan 005 G1 gate measured **100% top-1 precision on a 12-item fixture
set** (Bench 018). The corpus has since grown to ~270+ rules (209 kernel_opt
after Batch 30, 53 rust_perf, 10 clippy) and keeps growing (Issue 014 in
flight; Issue 016 will add more). Nearest-neighbor crowding is the standard
failure mode as a corpus grows — the precision claim is the core selling
point and currently rests on a fixture set 20× smaller than the corpus.

Two things are already measured and settled (do not re-litigate):

- **Score thresholds are a dead end** (Bench 018 addendum, encoded in the
  `ScoredRule` doc): clean snippets score as high as anti-patterns under
  text similarity. No threshold separates them.
- **Fan-out routing is the current fix** for the mis-routing centroids;
  re-gating is a separate future option.

What is NOT measured: **ranking precision** (is the right rule top-1/top-3)
at the current corpus scale, and its decay curve as batches land.

## Proposal

Build a scaled, repeatable eval harness:

1. **Fixture corpus**: ≥ 100 spans sampled from real code (katgpt-rs +
   this repo + the existing fixture generator), each hand-labeled with the
   ground-truth rule (or "no rule" — the clean-snippet class matters).
2. **Metrics**: top-1 / top-3 / top-5 recall over the labeled set, per
   domain and blended, at the current corpus scale.
3. **Regression gate**: the eval runs as a committed example/test (not just
   a one-time bench), so each new corpus batch re-measures before landing.
4. **Failure analysis**: every miss categorized (wrong domain / right domain
   wrong rule / clean-snippet ranked) — this feeds the routing re-gate
   decision with data instead of intuition.

## Acceptance

- [ ] Labeled fixture set (≥ 100 spans incl. clean/no-rule cases) committed.
- [ ] Eval harness runs via one command; outputs the metrics table.
- [ ] Baseline numbers recorded in `.benchmarks/` (top-1/3/5 at current
      scale, per-domain + blended).
- [ ] Regression gate wired (new corpus batches re-run the eval).
- [ ] Miss taxonomy documented; routing re-gate recommendation recorded.

## Non-goals

- No embedder re-training (modelless mandate stands; this measures, it does
  not tune).
- No score-threshold work — measured dead end (see Background).

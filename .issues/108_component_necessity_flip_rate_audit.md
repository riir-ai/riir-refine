# Issue 108: Component-necessity flip-rate audit (CVRR Research 555 follow-up)

**Status:** Filed 2026-09-15 (deferred from the 2026-09-14 CVRR session —
blocked then on a sibling mid-batch checkout, filed now that the repo pulls
clean).

**Lineage:** [katgpt-rs Research 555](../../katgpt-rs/.research/555_CVRR_Latent_Necessity_Strict_Interface.md)
(CVRR, arXiv:2609.06746 — "informativeness ≠ use") · katgpt-rs Issue 776
(contrastive matched-swap + norm-matched noise interventions) · riir-ai Issue
953 (the think-brain necessity audit — the game-runtime consumer of the same
substrate).

## Problem

The healer's per-rule heal rate reports how often a rule's fix LANDS, not
whether the rule's contribution is **load-bearing** in the pipeline's
composition — the CVRR distinction: a component can be *informative*
(correlated with resolved diagnostics) yet *decorative* (removing or
swapping it changes nothing downstream). Nothing measures the flip-rate
consequence of a component's absence/swaps at the composed-pipeline level.

Concretely: with rule A disabled (or swapped with a contrastive donor
rule's parameters), does the pipeline's resolved-rate actually move? A rule
whose disable/swap moves nothing is decorative in composition — its solo
heal rate is measuring a lane the composition routes around.

## Proposal (report-first, no gate until measured)

Consume the shipped intervention substrate — do NOT re-implement:

- `katgpt_core::faithfulness::perturb::{perturb_matched_swap, perturb_empty}`
  (Issue 776) applied at the per-rule / per-proposer component seam;
- `katgpt_core::interpolation_geometry::InterventionReport::{latent_is_causal,
  flips_to_donor}` for the verdict shapes;
- the score-bench fixture corpus as the deterministic replay ground
  (`.benchmarks/score_history.jsonl` generations pin reproducibility).

Deliverable: a **component-necessity table** over the clippy_lints +
rust_perf corpora — per rule: solo heal rate (known) vs composed flip-rate
under (a) empty (disabled), (b) matched-swap with the highest-co-occurrence
donor rule. Report decoratives by name. Two-sided canaries mandatory (the
audit-instrument law): a known-load-bearing rule (a top-5 heal-rate rule
with downstream dependents) MUST flip; a planted no-op control MUST be
flagged.

## Tasks

- [ ] T1 Component seam audit — where can a single rule be disabled/swapped
  without touching the corpus build? (the proposer dispatch loop).
- [ ] T2 Flip-rate harness behind a dev feature or bench — reuses the
  score-bench runner with per-run component overrides; JSONL out.
- [ ] T3 Measurement pass on the current corpus (one score-bench generation
  per arm × top-20 rules by heal rate + 5 random + 1 planted no-op).
- [ ] T4 The table + verdicts in `.benchmarks/`; decorative findings get
  their own follow-up issues (demote/repair decisions stay owner-side).

## Non-goals

No gate wiring from day one (report-first — a ceiling over an unmeasured
population is a pin nobody has watched fail). No changes to rule emission —
this is an instrument, not a healer behavior change.

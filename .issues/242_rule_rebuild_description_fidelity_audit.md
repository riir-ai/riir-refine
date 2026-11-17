# Issue 242: Rule rebuild description-fidelity audit (the iWSP text-bottleneck metric on the rule corpus)

**Status:** Open — fusion idea, filed unclaimed; report-only until a defect justifies promotion
**Claimed:** zed-m3-i242-t0, 1794900078, expires 2026-11-19T07:21:18Z — scope: T0 (the free redundancy pre-check) + the T0 verdict; T1 (hosted spend) stays outside this lease
**Source:** arXiv:2611.18114 "Interpretable weight-space programming builds transparent transformers and brain encoding models" (Singh et al., MSR/JHU/Columbia/NYU, 2026-11-13) — the **text-bottleneck rebuild metric**: describe an interpretable model in ≤200 words → a fresh agent session rebuilds it from the description ALONE → the rebuild preserves behavior (their numbers: all 13 algorithmic rebuilds 100% hidden accuracy, NLP −1.4 pts, rebuild params 1.74× the original vs **2,043×** for a description-less control, NLP error-correlation 0.74 vs 0.51). Distillation + verdict (Gain): `katgpt-rs/.research/678_iWSP_Interpretable_Weight_Space_Programming.md`; verdict round-1 AGREE conditions are baked into this issue.

## The idea

The rule corpus IS the model (the corpus-is-the-model doctrine), and its rules already travel as descriptions (`context_notes` propagated end-to-end; the hosted seat consumes them). The paper's metric gives an operational test of description fidelity the corpus has never measured: **can a rule's behavior be re-derived from its description alone?**

- For a sampled rule: compose a ≤N-word description of the rule's behavior (trigger + fix shape, not its code), hand the description to a hosted seat (`hosted_expert`, opt-in), and have it RE-DRAFT the fix from the description only.
- Score the re-drafted fixes against the rule's authored fixes on the oracle-labeled eval fixtures (`retrieval_eval.rs` fixtures class) — a fix-match rate per rule.
- **Control arm:** the same re-draft task with NO description (the paper's 2,043× control). The metric is the paired Δ.

A rule whose behavior survives the text bottleneck is a well-formed description; one that doesn't is machinery only its code can speak — a candidate for better `context_notes`, not auto-deletion.

## T0 — the free redundancy pre-check (BEFORE any tokens)

The corpus already carries `context_notes` + quality gates (`--gate-calibration`, `--coverage-report`). First check — zero cost, no hosted calls — whether those surfaces already answer the fidelity question (e.g., a census of how many rules carry substantive `context_notes` vs empty/boilerplate, cross-read against `--gate-calibration` strata). If they do, this issue CLOSES as redundant with the reason recorded. This is the verdict round's named weakness, promoted to a gate.

## T1 — the capped first run (only if T0 leaves a gap)

- **Sample:** 20–30 rules, stratified across domains (clippy_lints / rust_perf / kernel_opt), fixed by hash before the run.
- **Hosted budget:** ≤ $10 list-price cap for the whole run (the dspy-wsp writer-spend datum, Research 671); refuse to start without the cap recorded.
- **Kill bar (pre-registered):** if description-only re-drafts reach **Δ < 10 points of fix-match over the description-less control** on the fixtures, the audit CLOSES NEGATIVE — the corpus's descriptions carry no re-derivable signal beyond task prompts, and the instrument dies with the negative recorded (Issue-565 precedent).
- **Pass bar:** Δ ≥ 10 points with per-rule variance that identifies specific low-fidelity rule families — the only promotion-worthy outcome is **a rule-quality defect the existing gates missed** (e.g., a family whose `context_notes` never survived the bottleneck).
- **Report-only:** `cargo refine --rule-rebuild-audit` (report lane, exit 0); NO promotion path, NO corpus mutation, NO gate wiring until the defect-showing bar is met in a NEW pre-registration.

## Disclosures

- The re-draft needs a hosted writer — `hosted_expert` posture, code-egress consent scopes apply (rule descriptions leave the box, rule code need not).
- The metric is validated on neural networks (this paper), not rule corpora — the domain transfer is exactly what T1 measures; do not cite the paper's rebuild numbers as evidence the audit will work here.
- External metric provenance: the rebuild protocol is the paper's; no novelty claim — this issue is an adoption idea (Research 678 verdict round 1 AGREE, session 1eef961b-5552-47cf-94f2-b87e80914a6b).

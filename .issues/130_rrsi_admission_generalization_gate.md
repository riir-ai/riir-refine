# Issue 130 — RRSI-informed admission-time generalization gate for corpus rules (design decision)

**Status:** OPEN — filed from [Research 203](../.research/203_rrsi_regularized_harness_evolution.md) (arXiv:2609.24972 "RRSI: Regularized Recursive Self-Improvement of Agent Harnesses" distill). **DESIGN DECISION pending owner triage** — the gate's evidence costs a per-candidate bench delta the current batch discipline does not pay; the conservative default is report-only (the Issue-102 precedent).

## The measured problem (why this is not speculative)

The healer's own evolve-set-overfit signature is already measured, three ways:

1. **Provenance strata** (Issue 089 T6, printed every clippy-lane run): authored fixtures heal 98.0% (51 findings) vs repo-mined 50.0% (2) — 6 of 8 repo fixtures are clean spans. The 96.2-96.4% aggregate is authored-dominated.
2. **OOD refutations on record:** set_rerank promotion REFUTED (composite 85.2% vs 70.4/74.1% top-1; every α-ablation lost on the OOD rig — Bench 096); choice_scorer REFUTED (27.7% vs baseline 88.0% in-dist, 5.6% OOD — Bench 098).
3. **The withheld-pair OOD axis is unmeasured almost everywhere** (the riir-clippy factory-table row; Issue 62's bench never landed).

RRSI's diagnosis names exactly this triad: benchmark-specific fitting, noise chasing, complexity accumulation — evolve-split gains that do not transfer.

## The candidate mechanisms (from the paper, signal-diffed against shipped cousins)

| Mechanism | Form | What it would gate here |
|---|---|---|
| E. Noise-adjusted admission floor | admit only if Δ(repo-stratum heal) > δ, where δ comes from REPEATED baseline score-bench runs of the unchanged corpus | a rule whose gain lives only in the authored stratum does not clear the floor |
| —. Within-band credit (Eq 17) | when gain ≤ δ: admissible only via cost reduction or structural novelty (a never-covered lint family) | a marginal rule covering a genuinely new family still enters; a marginal duplicate does not |
| F. Gain-proportional cost line | ΔC ≤ β₀ + β₁·ΔS, C = corpus-table/binary cost per rule (~70% of .text is corpus tables — Issue 102's cargo-bloat measurement) | couples per-rule table cost to per-rule measured gain at admission |

Honest scoping: our compile/clippy oracle is deterministic per pinned toolchain — RRSI's δ machinery exists because their verifiers are stochastic (k-trial LLM judges). The stochastic surface here is the retrieval/heal-rate benches and the OOD rigs, so the floor reads on STRATIFIED BENCH EVIDENCE, never on the oracle.

## Scope split (no race, no overlap)

- **Issue 089** owns frontier-band admission of the CURRICULUM (what the miner generates against).
- **Issue 102** owns the lifecycle half (fade, cap, demotion — what stays active; report-first, T2–T4 clock running since 2026-09-21).
- **This issue** owns the ADMISSION-TIME GENERALIZATION GATE for corpus rules (what ENTERS on generalization evidence). Contribution-fade (102) is the same discipline applied at the other end of the lifecycle.

## Tasks

- [ ] T0 (owner) — Decide: gate vs report-only. If report-only: close this as a Research-203 record (the strata already prints it) and keep 102's clock as the only lifecycle gate. If gate: proceed to T1.
- [ ] T1 — δ calibration harness: N repeated baseline `--score-bench` runs of the unchanged corpus → empirical δ per stratum (authored / repo-mined), the `best_of_us` recipe applied to admission evidence. Zero new stores; pure arithmetic over score_history rows.
- [ ] T2 — Admission predicate + within-band credit: a batch lands a rule only if Δ(repo-stratum) > δ OR within-band credit (new lint family / net cost reduction). Wired at the batch discipline level (`.distill/001` standing rules + the authoring-discipline doc), NOT in the heal path.
- [ ] T3 (optional rider) — Automatic stall predicate over `frontier_report`'s own series (RRSI mechanism C): σ_t = 1[progress ≤ δ over window w] → flag the coverage-report re-arm in the mining queue. Automates what is today operator-judged.

## GOAT gate (if T0 lands on "gate")

- **G1:** repo-stratum heal rate improves across a batch cohort at flat authored-stratum rate (the strata already prints both — no new instrument).
- **G2:** planted-bad control: a rule authored to fire only on its own fixture spellings (the Batch-167 u32/u64 class) is REFUSED by the floor.
- **G3 (no-regression):** batch throughput does not collapse (the floor must not starve the corpus — RRSI's own within-band credit exists to prevent exactly this).
- **G4:** bounded cost — pure arithmetic over existing feeds; zero GPU.

## Cautions

1. **Report-first law applies** (R135/Bench-047: selection sophistication at real sparsity measured ≤ 0.04 pp). The repo stratum is THIN (2 findings in 8 fixtures at the first measurement) — a floor over a thin stratum is noise about noise. **Worse: at 2 findings the stratum's heal rate is QUANTIZED to {0%, 50%, 100%} — one finding flipping moves it 50 points, so G1 as written has no resolution below a 50pp cohort improvement, regardless of δ.** Growing the repo fixture corpus is therefore a PRECONDITION of T1 being informative at all, not the fallback if calibration fails — T1 must report the stratum's own variance AND its finding count first; if the stratum cannot support a δ, the gate defers and the answer is "grow the repo fixture corpus first".
2. Do not wire anything into the heal path — admission discipline lives at mining time.
3. Cross-issue hygiene: if 089 T4's admission-gate wiring subsumes this, consume it there rather than re-landing (the 102↔089 split precedent).

# Issue 098: Rule-corpus Cross-OOO axis — the absorption/splitting detector for the rule bank

> **Status:** OPEN (2026-09-12)
> **Source:** katgpt-rs Research 552 (Super-GOAT, PoC passed — Bench 758) + riir-ai Research 378; primitive shipped in katgpt-core behind `direction_bank_audit` (GOAT G1+G2+G4 PASS, opt-in)
> **Fusion:** Research 552 §F2 (healer = fusion priority #2)
> **Priority:** P2 (offline bench-time instrumentation; never in the heal path)

## Problem

The 580+-rule corpus (clippy_lints 49 + rust_perf 126 + kernel_opt 607 + sec 15 + docker 31) has per-rule outcome stats (`frontier_report` r/f/n/p̂, `rule_outcomes` rows since Issue 089 T8) but **no cross-rule redundancy axis**: nothing measures whether two rules trigger on indistinguishable code shapes. That is exactly the SAE-latent absorption/splitting pathology (Chanin et al. 2024) transplanted to rule space — a merged/redundant rule pair silently splits credit, double-counts coverage, and corrupts the per-rule frontier feed that T4's admission gate will consume.

## Mechanism (all substrate ships)

- rules = units; each rule's top-K oracle-triggering spans (the retrieval fixtures / oracle labels) = MEIs; span embeddings (riir-rag `ModellessEmbedder`) = the similarity matrix `S` (semantic granularity — the Bench 758 metric-granularity law is explicit that the metric defines the concept).
- `cross_ooo(a, b)` ≈ 0.5 → redundant pair (merge candidate); `ooo_score` per rule → interpretability axis (a rule whose top spans are mutually dissimilar is entangled).
- `greedy_curate(0.8, 0.8)` over the whole corpus → `unique_feature_count` = the corpus's measured distinct-content capacity — a new headline stat for the score-bench.
- Sink: append a cross-OOO column to `frontier_report` per-rule rows (additive, instrumentation-only — the provenance-strata precedent from Issue 089 T6).

## Tasks

- [ ] T1: fixture harness — per-domain top-K oracle spans → span embeddings → `S` matrix (CARGO_TARGET_DIR isolated; `katgpt-core/direction_bank_audit` feature on)
- [ ] T2: measure — full-corpus cross matrix + curated count per domain; report the top-20 most-redundant rule pairs (the absorption candidates) in the bench doc
- [ ] T3: verdict — if no pair reads < 0.6 cross, the axis is hygiene-only → record negative, close; if redundant pairs exist, append the axis to `frontier_report` (additive jsonl field, `ood`-style instrumentation)
- [ ] T4 (on T3 pass): score-bench integration — `unique_feature_count` per domain printed + persisted on the run row; the mining queue gets merge candidates

## Acceptance

T3 verdict recorded with numbers either way (the Issue 759 defend-wrong precedent — a negative is a finding, not a failure).

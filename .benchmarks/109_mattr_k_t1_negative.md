# Bench 109 — Issue 133 T1: Matryoshka-k supervision for `rule_embed` — NEGATIVE (fixed objective wins at every measurable depth)

**Date:** 2026-10-08 · **Repo:** riir-refine @ develop `f65c3a58` + the `feat(133)` harness commit · **Box:** M3 Max, quiet, CPU-only, deterministic seeds (no GPU, no new deps — `exact_mass_admit` rides the existing katgpt-core path dep)
**Issue:** `.issues/133_matryoshka_k_supervision_rule_embed.md` (T1; owner arbitration session `9537f3e7` = B1 with three binding conditions) · **Source:** arXiv:2609.25518 (Matryoshka attribution) via katgpt-rs Research 584 · **Prior art:** Bench 099 (the shipped training run + standing gates)
**Artifacts:** `/tmp/mattr_t1/` (5 arm `.bin` artifacts BLAKE3-pinned below, `results.jsonl` every metric row, per-run `report_*.md`) — /tmp scratch; this file is the durable record. Harness: `src/draft/rule_embed_mattr.rs` + `examples/rule_embed_mattr_t1.rs` behind `mattr_k_poc` (default-off), landed for replication (the `gw_corpus`/`llm_gist_author` NEGATIVE-record precedent).

## Setup

- **Corpus:** `049_t01/spans.jsonl` (2128 spans) ∩ current 100-lint clippy corpus = **946 spans / 1095 pairs** (816 single + 130 multi = **1.16 labels/span**, B1 condition 1 recorded first) / 18 distinct lints. Top-4 (doc_markdown 518, suboptimal_flops 206, use_self 130, cast_lossless 77) = 85%. doc_markdown (47%) is WITHHELD under the rule-level BLAKE3 split → the B1 training-side cap is vacuous; the concentration guard runs as the capped eval population (206) — **the cap changes no arm's rank**.
- **Splits:** rule-level blake3 withheld: train 243 pairs (10 lints) / withheld 852 (8 lints); 76% of eval pairs are withheld-lint (OOD) — the corpus's center of gravity IS the generalization read.
- **Arms** (3000 steps, shipped TrainConfig, seed 126, same union data): control (shipped pair-NCE fixed, `9191d98b91393805`) · spanwise_full (k=pool diagnostic, `50759c43a6c0c45a`) · uniform k~U{1,2,3,5,8} (`122450d3e233fb2b`) · all-widths k~U{1,2,3,5,8,10,20} (`076c0d98f1530723`) · logboth P(k)~1/k keep-first+demote-last (`066e892dd8e71dd8`).
- **Harness sanity:** the shipped frozen asset reproduces Bench 099's standing posture exactly (baseline 90/104 + 36/37, trained 95/104 + 36/37, clean 31/31, G1 0).

## Results

**Fixture rig (Bench 099 standing gates, population 141 = 104 train / 37 withheld):**

| arm | trained total vs baseline | G1 | G2 | G2-OOD | G3 | beats shipped asset (131) |
|---|---|---|---|---|---|---|
| control (shipped objective retrained) | 134 vs 126 | **FAIL** (1: `legacy-key-value-format`) | PASS | **FAIL** (35<36) | PASS | true-but-rejected |
| spanwise_full | 130 vs 126 | PASS | PASS | PASS | PASS | no |
| uniform {1,2,3,5,8} | 127 vs 126 | PASS | PASS | PASS | PASS | no |
| all-widths | 129 vs 126 | PASS | PASS | PASS | PASS | no |
| logboth | 127 vs 126 | PASS | PASS | PASS | PASS | no |

**Span-corpus k-curve, tower-only withheld recall@k (train / withheld @k=20):** control 100/92.0 · spanwise 15.5 · uniform 29.3 · all-widths 49.6 · logboth 81.2 — every randomized-k arm trails the fixed objective at every depth. Fused production posture: trained terms move the TRAIN split (control 16.5 vs baseline 11.1 @k=1) and are a near-no-op on WITHHELD (all ≈ baseline 29.6/40.5/44.7/47.5/55.0 — withheld ψ rows score ~uniform, w·σ adds a near-constant).

**Withheld-width nesting (the issue's generalization axis): REAL** — training k∈{1..8} only → 29.3% @k=20; adding the widths → 49.6% (@k=10: 1.4→14.0). The mechanism transfers; the fixed objective still dominates it (92.0%).

**Macro anatomy (B1 condition 2):** doc_markdown top-1 = 0.0% tower-only for EVERY arm (withheld ψ rows never win top-1; the fused posture is what ranks them). logboth BURIES 6 low-frequency train lints outright (0% @k=20) and fails per-family retention (use_self 70 < 100, cast_lossless 81.8 < 100) — the label-blind demote side is a trap for golds below the cut.

## The bit-honesty incident (recorded — it is a finding)

The first logboth weighted the label BCE by BOTH admit masks → **byte-identical artifact to spanwise_full** (caught via blake3): the exclude-last admit over negated scores is the exact complement of keep-first (σ(−x)=1−σ(x), same bisection), so m⁺+m⁻≡1 — one mask wearing two names, and the 3× runtime was the wasted second admit. Corrected to Research 584's actual semantics (label-blind DEMOTE pressure + label BCE keep side); that variant then fails per-family retention. **Lesson: "supervise both ends" must mean opposite LABEL directions, not two weightings of the same BCE.** Cross-invocation determinism measured en route: the degenerate arm reproduced its artifact byte-identically across two separate processes.

## Verdict — T3, NEGATIVE (this coverage)

The shipped fixed objective wins the shallow-k mechanism axis everywhere it is measurable (rig 134 vs ≤130; fused train top-1 16.5 vs 11.1) and the only arm beating the shipped ASSET is the control itself — which FAILS G1/G2-OOD, exactly what the gates exist for. No randomized-k arm passes the T2 bar. Deep-k (10/20) reads **corpus-limited** per the pinned interpretation rule (1.16 labels/span: every arm's withheld tower top-1 ≈ 0% while BM25's is 29.6% — the deep slates are unordered space for all towers). **The negative closes THIS coverage only** (18/100 lints labeled, 85% of pairs top-4): a broader oracle-anchored span pass (the B2 content) is the named re-test ground — optional follow-up, not a precondition. No T2 re-freeze; nothing wired; the shipped lane unchanged.

## Honest gaps

suboptimal_flops (suggest-only, association label) is 206 of 243 train pairs — the train-split read is essentially one lint; the fixture rig carries the mechanism verdict, which is why the gates lead it. Labels predate Aug-30 HEADs (inherited from the corpus). T_admit = 1.0 pinned a priori (not swept — no eval-split fitting). Budget draws are per-step (one k per step, hundreds of visits per budget), not per-span. Fused k-curve window 24 (vs shipped PROBE_TOP_K 8) — required to measure k=20 at all; hits@{1,3,5} window-stable. Count-exact metrics (not timing) — no box-state PROVENANCE line needed; wall-times per artifact in the report.

**En-route finding (fixed same session):** `tests/rule_embed_gate` was RED on develop — its population pin (138) trailed the grown corpus (141); floors held, regressions 0. Re-pinned with the measured numbers in the same session (the `test:` commit beside this record).

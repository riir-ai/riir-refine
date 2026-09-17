# Bench 096 — the counter-anchored set-rerank A/B (Issue 121 / katgpt-rs Plan 599 T4.2)

**Status:** RECORD — landed 2026-09-17. **Verdict: the set-admission arm FAILS its promotion gate on both rigs; the A/B's real yield is the `all_rules` canonical-order fix (+55.6pp top-1 / +40.7pp recall@5 on the merged surface, landed DEFAULT).** The `set_rerank` feature stays opt-in; the vendi axis + the OOD policy arm stay as standing instrumentation.

## The question

Issue 121: `LatentMatcher` fans a span out across seven per-domain `RuleIndex`es and the consumers merge the hits — an **unanchored set decision** (a flat sort where one rule family can fill every slot). katgpt-rs Plan 599 shipped `set_admission` (Bench 811: greedy==OPT 8/8, zero-alloc, exact-Vendi certificate, θ-ladder recovery) with THIS repo's rerank as its designated first consumer (T4.2). The gate: does the counter-anchored admission **lower the collapse rate and raise oracle-recall** on the composite fixtures + the OOD fresh-family rig, with `detect_collapse` staying the eval axis (never the guard)?

## The arms

| arm | what |
|---|---|
| baseline (merged order) | the CURRENT merge — score desc, rule_id asc (what `all_rules` shipped) |
| **canonical merge (control)** | instantiates desc, score desc, rule_id asc ACROSS domains — the defend-wrong control isolating the structural-key confound |
| set-rerank Full8 | `admit_with_recovery_into` over the merged pool, quality = minmax(score)+0.5·instantiates, α/κ/θ/ρ = R4T defaults, latent = native 8-D |
| set-rerank Structural4 | same, hash dims zeroed (dims [0..4] of `ModellessEmbedder` are BLAKE3 buckets — zero locality) |
| OOD α-ablation | Full8/Struct4 at α=0.6, Full8 at α=0.2 and α=0.0 (R4T's own ablation law applied to the losing rig) |

Truth: real `cargo clippy` over the 27 scorable composite spans (the T3.5.5 uncontaminated discipline — the oracle knows nothing about our corpus, scores, or kernel). Harness: `tests/set_rerank_ab.rs` (`--features set_rerank,clippy_lints,rust_perf,clippy_verify --test set_rerank_ab -- --nocapture`).

## The measurements

**Composite rig (n=27):**

| arm | top-1 | recall@5 | collapse raw/miss | mean vendi | recovery |
|---|---|---|---|---|---|
| baseline (merged order) | 29.6% | 55.6% | 0 / 0 | 2.502 | — |
| **canonical merge (control)** | **85.2%** | **96.3%** | 0 / 0 | 2.648 | — |
| set-rerank Full8 | 70.4% | 88.9% | 0 / 0 | 2.534 | 0 fired |
| set-rerank Structural4 | 74.1% | 85.2% | 0 / 0 | 1.212 | 0 fired |

**OOD rig (clippy_population, pure retrieval — no structural evidence):**

| arm | ID top-5 | OOD top-5 | NOROLE top-5 |
|---|---|---|---|
| shipped | 100.0% | 50.0% | 0.0% |
| set Full8 α=0.6 | 50.0% | 16.7% | 0.0% |
| set Struct4 α=0.6 | 50.0% | 16.7% | 0.0% |
| set Full8 α=0.2 | 83.3% | 16.7% | 0.0% |
| set Full8 α=0.0 | 83.3% | 33.3% | 0.0% |

Latency: all arms within noise of baseline (~1.0-1.1 ms/span, release).

## The verdict, honestly

**1. The set-admission arm loses to its own control.** The canonical-merge control (just restore the per-domain rerank's structural key at the merge) beats both admission arms by **11-15pp top-1** and **7-11pp recall@5**. The composite "win" vs baseline is entirely the structural key; the counter-anchors SUBTRACT. Mechanism: the composite spans are multi-lint BY DESIGN — 2+ true rules whose patterns sit near each other — exactly what the θ_coll=0.95 colinearity cap rejects as "too similar" after the first is admitted. Diversity pressure demotes true siblings.

**2. The OOD rig regresses under every ablation arm.** Pure retrieval (no proposers fire), so quality is min-max score alone. The α-ablation pins the mechanism: α=0.6→0 recovers ID 50→83.3% but OOD tops out at 33.3% vs shipped 50% — the alignment anchor (operating in a hash-noise + byte-histogram space that does not reflect token similarity) causes much of it, and the remaining gap is the diversity anchor itself: when the top-scored rule IS the answer, ANY diversity pressure is a loss. NOROLE stayed 0.0% under every arm (the grounding invariant holds — the admission cannot invent an absent role).

**3. The collapse failure mode is absent from today's fixtures.** 0 collapses (raw or miss) in EVERY arm including baseline. The crowding the issue was filed against (the "4 tents") was measured at POOL depth (Issue 024) and fixed by `RERANK_POOL=20`; at MERGE depth on today's corpus it does not materialize. The guard has nothing to guard here. **Re-run this A/B when the `pool_depth_rescue_guard` next fires** (the designed prompt for a corpus-growth re-measure) — that is the regime where counter-anchoring has a job.

**4. The real defect the A/B caught — landed as the GOAT.** `SpanSuggestion::all_rules` sorted by score ALONE while its own doc claimed "same canonical order as `rank_into`/`rerank`/`canonical_rank`" — the merge silently DISCARDED the T3.5.4 instantiation re-rank across domains (a fired proposer's rule could be buried under higher-BM25 cross-domain rivals). Restoring the canonical key: **29.6→85.2% top-1, 55.6→96.3% recall@5**. Landed on the DEFAULT path (both `all_rules` consumers are order-independent — verified before the change; regression test `all_rules_keeps_the_canonical_key_across_domains` pins it).

## What shipped

- `src/draft/set_rerank.rs` (opt-in `set_rerank`): `counter_anchored_slate` (admission+certificate+θ-ladder recovery over the merged pool), `baseline_slate`, `RuleLatentCorpus` (deterministic embedded rule corpus), `LatentChoice` (Full8/Structural4), kill-switch `RIIR_CLIPPY_NO_SET_RERANK=1` (same-binary A/B). 5 unit tests (determinism, recovery invariant, degenerate inputs, canonical order, projection).
- `LatentMatcher::suggest_set_slate` + `with_set_config` + `certify_slate_ids`; the rule-corpus latents built at construction (G4 scratch reused).
- **Vendi axis in `retrieval_eval`** (Issue 121 task 2): `EvalReport.vendi_axis` — exact cosine-kernel certificate of every delivered per-domain top-8, mean/saturated/cert-collapsed beside the categorical detector, printed in `print_report`.
- **OOD policy arm** (task 3/4 wiring): `run_withheld_pair_under` refactor (rank-closure parameterization; `run_withheld_pair` signature byte-identical as a delegate) + `run_withheld_pair_set_admission`.
- `tests/set_rerank_ab.rs`: the A/B + α-ablation harness, `required-features` row (the silent-green-zero rule).
- **`all_rules` canonical-order fix** (the GOAT, default path).

## Standing

- `set_rerank` feature: **opt-in, not promoted** (the plan's own rule — promotion only on gate wins; both rigs lost). katgpt-core `set_admission` correspondingly stays opt-in (its consumer eval recorded here).
- Recovery loop: wired + unit-tested, measured INERT on these fixtures (0 certificates collapsed at ρ=0.2).
- The vendi axis + OOD policy arm are the standing re-measure instruments for the corpus-growth regime.
- Gates run: default lib **1728/0** · `latent_retrieval,clippy_lints,rust_perf` lib **1881/0** (incl. the 6 new tests) · `set_rerank` A/B **2/2** · clippy `-D`-clean at both feature states (5 mechanical findings healed via `cargo heal`, the rest manual) · frame_time 11/11 (the one committed-code residue `cloned_ref_to_slice_refs` fixed in passing).

# Research 420: Goodfire Neuron Communities — Co-Fire Rule Communities for the Rulewright

> **Source:** "Large-scale Neuron Community Discovery Reveals Neuron-Sparse Behaviors in Language Models" — Goodfire + Imperial College (Igor Shilov, Thomas Fel, Tal Haklay, Lucius Bushnaq, Owen Lewis). Blog: <https://www.goodfire.com/research/neuron-communities> (2026-11-17); arXiv: [2611.23112](https://arxiv.org/abs/2611.23112).
> **Date:** 2026-11-18
> **Status:** **NEGATIVE-sparse** (2026-11-18, PoC-0 kill condition — Bench 161). The GAIN verdict is demoted: the pinned co-fire population sits 3 edges under its own sparseness bar (57 weight-≥3 edges vs the pre-registered ≥ 60), so the PoC chain stopped at its census gate. Re-open condition (new data): a wave-1 per-crate re-run. §7 carries the record.
> **Related Research:** [053 (CNA — gradient attribution, SAE rejected)](../../katgpt-rs/.research/053_CNA_Contrastive_Neuron_Attribution.md) · [365 (riir-ai `co_occur` SkillEdgeTable)](../riir-ai/.research/365_SkillGraph_prereq_enhance_co_occur_skill_edges.md) · [393 (BSF — the representation-side sibling)](../../katgpt-rs/.research/393_Block_Sparse_Featurizer_Subspace_Concept_Primitive.md) · [394 (GNN survey — modularity adjudication)](../../katgpt-rs/.research/394_GNN_Survey_Within_Class_Effective_Rank_Fusion.md)
> **Related Issues/Benches (this repo):** Issue 094 / `examples/tca_covariability_poc.rs` (the controlling negative) · Issue 098 cross-OOO (`src/score_bench/cross_ooo.rs`) · Bench 092 (`swap ≈ disable`) · Bench 094 (the fired-histogram data source)
> **Verdict provenance:** Claude verdict ping-pong, 2 rounds (REVISE → AGREE), session `9e2b1616-7416-4104-b50f-9d48ae7ef5bc`, closed with `final_round` 2026-11-18.
> **Classification:** Public

---

## TL;DR

Goodfire discovers, **unsupervised**, that LLMs implement behaviors (grammatical agreement, arithmetic, copying, formatting) in **communities of <10 neurons** that are causally necessary and sufficient: gradient-attribution over millions of prompts → a co-attribution graph (edges = repeatedly co-credited neuron pairs, frequency-adjusted) → community detection → causal ablation tests (remove the community: prediction changes; keep it while ablating the rest of those layers: prediction preserved). Thousands of such communities found in Llama-3.1-8B; replicated on Qwen 3.6 35B-A3B (including knowledge communities — state capitals, painters).

**Distilled for the rulewright (modelless, no forward pass):** the transferable shape is *"co-fire graph over discrete labeled units → community discovery → causal necessity/sufficiency audits"*. Our units are not neurons but **corpus rules**; our "attribution matrix" is not gradients but **per-unit fired-rule sets** (Bench 094's per-crate fired-lint sets; Bench 092's fixture sets); our causal test is not ablation-in-a-forward-pass but **community-level active-set demotion measured against `--necessity-bench`/heal acceptance**. This is the first consumer for the community-detection capability Research 394 adjudicated "Gain, not shipped, orthogonal" — the paper converts "orthogonal" to "consumer exists".

## 1. Paper Core Findings

| Finding | Detail |
|---|---|
| Unsupervised discovery | No function specified in advance: 10M prompts (FineWeb-Edu + Wikipedia + FineMath), gradient attribution per prediction, co-attribution edges adjusted for individual neuron frequency, graph community detection |
| Neuron-sparse behaviors | Communities often <10 neurons across up to ~15 layers; necessity test: removing the community ≈ removing ALL MLP neurons in those layers (mean top-token prob change 0.42 vs 0.38 in the case study); sufficiency: keeping the community (+ 17 "always-on" neurons model-wide) largely preserves predictions (0.08) |
| Behavior classes | Grammar (agreement, tense, case), retrieval & computation (unit conversion, multiplication, in-context retrieval), topic, formatting (Title Case, ALL CAPS, bracket matching), common phrases (phrasal-verb particles), **token suppression** (1–2 neurons silencing one specific token — the most common class) |
| Depth organization | Layer heatmap per class: grammar concentrated L18+, retrieval/computation peaks L17–22, token suppression concentrated in final layers — communities map *where* kinds of computation live, not just *that* they exist |
| Replication | Qwen 3.6 35B-A3B: hundreds of thousands of communities incl. knowledge circuits (state capitals, post-Impressionism painters) |
| Related Goodfire lane | Tigges et al. 2026 "Neurons as an Interpretable Basis" (models sparsify in the neuron basis); Neuron Atlas (2026-11-10) |

## 2. Distillation — Mechanism Table (paper vocab → our analog)

| Paper step | Paper form | Our analog (rulewright) | Status |
|---|---|---|---|
| 1. Neuron attribution | Gradient attribution over millions of prompts | **Not needed** — attribution data already exists observationally: per-crate fired-rule sets (Bench 094 `results_wave*.jsonl`, each row a `lints` dict) and per-fixture fired sets (Bench 092 `vocab_before`) | Data on disk (waves 2–4; wave 1 manifest-only) |
| 2. Co-attribution graph | Edge weight = co-credit frequency adjusted for individual frequency | **Rule co-fire edges** from the same per-unit fired sets. NOT from `.refine/trajectories.kv` — that store keeps cumulative (rule, shape) counters with only the last-verdict timestamp, no session axis (Issue 094 closed NEGATIVE at AUC 0.497 on exactly this; `HISTORY.md:1001`) | Gain — the edge set must be built; count range ~100 crates with ≥2 lints (PoC-0 pins exact counts) |
| 2b. Alternative edges | — | **Cross-OOO shape-similarity edges** already ship (`src/score_bench/cross_ooo.rs`: `cross_ooo(a,b) ≈ 0.5 ⇒ redundant pair (merge candidate)`) — fusable with co-fire edges | Ships (Issue 098) |
| 3. Community detection | Louvain-class community detection on the neuron graph | Same, over the rule graph. Not shipped anywhere in the workspace (Research 394: zero `community_detect`/`modularity` hits) | Gain |
| 4a. Necessity | Ablate the community → prediction changes (KL) | **Community-level demotion** via `--active-set-demote` → heal-acceptance Δ on the community's fire class; **non-additivity gate**: community Δ must exceed the sum of members' per-rule `--necessity-bench` Δ beyond a pre-set noise band | Per-rule halves ship (`--necessity-bench` Issue 108/Bench 092); group granularity new |
| 4b. Sufficiency | Keep community, ablate the rest of those layers → prediction preserved | Communities-only active set retains heal acceptance vs **top-k-by-mass** baseline (same size; the B_mass volume confound from Issue 094) + a random-set control | Gain |

### The honest disanalogy (load-bearing)

Goodfire's neurons interact **within one forward pass** — a community is a causal sub-computation. Rulewright rules fire **independently per span**; nothing "computes together" at fix time. Bench 092 already measured the warning sign: **swap ≈ disable** — the highest-co-occurrence donors are shape-disjoint ("rules co-occur because they fire on DIFFERENT shapes in the same file, so a matched-swap behaves like a disable"). File-level co-fire edges therefore risk recovering **code genre** (FFI-heavy crates, numeric crates), whose community necessity would equal the sum of member necessities — bookkeeping, not group behavior. **The non-additivity gate is the experiment that decides whether group-level behavior exists here at all; failure means "communities are just co-location" and the lane records NEGATIVE.**

## 3. Closest Cousins Considered (and why none kills the claim)

1. **Issue 094 TCA (same store, negative)** — the controlling negative for any trajectory-store-side co-fire source: AUC 0.497 = chance; failure attributed to the store schema ("cumulative counters, last-verdict timestamp — no session axis", `HISTORY.md:1001`). We EXCLUDE the trajectory store and use per-unit fired sets instead.
2. **Issue 098 cross-OOO** — a rule-pair graph already exists (`cross_ooo`, TF-IDF rule-text similarity, merge-candidate threshold ≈0.5). Cousin, not duplicate: cross-OOO measures shape-similar pairs; co-fire measures same-context co-occurrence. The PoC treats cross-OOO as the optional fused edge set and the named donor pool Bench 092 itself pointed at.
3. **`--necessity-bench` (Issue 108 / Bench 092)** — the right TEST at the wrong granularity: per-rule/per-decision retirement pricing, no discovery. Signal-diff: per-rule retirement pricing vs discovered-group minimal-active-set. Not covered — the discovery half is exactly the gap.
4. **riir-ai Issue 875 `co_occur` SkillEdgeTable (Research 365)** — the mechanism ships over TASK episodes in the agent coordination loop (window ticks, decay, claim-hint consumption). Right mechanism, wrong surface (tasks-not-rules) and it stops at step 2 — no clustering, no causal audit. Consumed as pattern, not claimed over.
5. **Research 394 (GNN survey)** — already granted modularity community detection "Gain, not shipped, orthogonal". This paper supplies what that row lacked: a concrete consumer whose economics the communities would drive.
6. **Research 393 (Block-Sparse Featurizers)** — the Goodfire sibling on the other side of the representation/weight line: BSF decomposes activations into concept subspaces (trained, representation-side); this paper finds structure in the network's own units (observational, weight-side). Sibling, not duplicate.
7. **Research 053 (CNA)** — gradient-based contrastive attribution with "don't implement SAE" as the verdict. This paper independently validates that posture: Goodfire's own method is gradient-attribution + graph structure, no SAE anywhere.
8. **Published prior art (why NOT Super-GOAT / no open primitive in katgpt-rs):** the mechanism CLASS is published — Horta et al. 2019/2021 (co-activation graphs → community detection on DNNs), Gross et al. 2025 (Louvain on RL-policy co-activation graphs), plus Goodfire's own paper. Goodfire's deltas are scale (gradients + millions of prompts vs activation correlation), cross-layer communities, and the causal necessity/sufficiency validation. Our delta is the consumer surface: corpus economics from observational outcome data — no model, no forward pass.

## 4. Verdict

| Mechanism | Tier | One-line reasoning |
|---|---|---|
| Gradient attribution | **PASS** | Model-side only; our analog replaces it entirely with observational fired-sets; validates Research 053's no-SAE posture |
| Co-fire edge construction | **Gain** | Mechanism ships over tasks (riir-ai 875) but not over rules; trajectory store excluded per Issue 094 |
| Community detection | **Gain** | Not shipped anywhere (Research 394); first real consumer supplied by this paper |
| Community necessity/sufficiency audits | **Gain** | Per-rule halves ship (`--necessity-bench`, `--active-set`); group-granularity discovery + validation new |
| **Overall** | **GAIN** (GOAT-fusion candidate ONLY on PoC pass) | Mechanism class has published prior art → no Super-GOAT, no open primitive in katgpt-rs; consumer fusion (co-fire graph × community detection × community-level audits driving `--active-set`) is novel for the rulewright but unproven — PoC issue with pre-registered gates, not a plan with claimed gains |

**MOAT gate (riir-refine, consumer-first):** the fusion targets corpus structure + trajectory/audit memory — squarely the repo's moat bar ("measured rulewright-quality gains: retrieval floors, score-bench heal rate, corpus/trajectory structure"). Routing correct. No katgpt-rs filing (the generic mechanism is published prior art; consuming it needs no new substrate primitive). Public note — riir-refine is public-MIT and the content is generic distillation + internal consumer mapping, no moat leak.

**Three-track check:** paper has NO training loop (pure attribution/graph/clustering/ablation) — all modelless-extractable via the fired-set substitution. No riir-train deferral anywhere. Adversarial panel skipped (clearly inference-side interpretability paper, zero optimizer/loss/RL content).

## 5. Consumer Map

- **Primary (this repo):** rule co-fire graph (Bench 094 per-crate sets + Bench 092 fixture sets) → communities → community-level `--active-set` audits. Buys: (a) corpus-compaction candidates (a necessary+sufficient community = the minimal active set for its fire class); (b) redundancy detection (rules never co-firing or subsumed by a community — cross-references the cross-OOO merge-candidate lane); (c) domain-organization sanity (do hand-assigned domains match emergent communities? the paper's layer-heatmap analog: *which domains host which fix classes*); (d) drift signal (a community spanning domains is organization drift).
- **Secondary (decision family):** reflex lanes — per-suite winner attribution over corpus entries → "decision communities" (small entry sets necessary+sufficient for a suite's picks). Parked until the rulewright PoC adjudicates the analogy.
- **Game-side (riir-ai):** `co_occur` SkillEdgeTable (Issue 875) + `influence_ring` (Issue 1178) already ship the step-2 shape; community detection atop them → emergent faction/social-group detection — the exact "could support faction/emergent-social detection later" flag Research 394 left. Not routed now (game lane is not the priority surface for this paper).
- **Named classes to look for, mirroring the paper:** token-suppression analog = suppression/negative-pattern rules (expect 1–2-rule communities); "always-on neurons" analog = always-compiled/default-featured corpus rows; depth-heatmap analog = which corpus segments/domains host which fix classes.

## 6. PoC Pointer

**CLOSED NEGATIVE-sparse at PoC-0 (Bench 161, 2026-11-18)** — the gates lived in Issue 245 (closed + removed at the close; git history recovers the file; the record lives in §7 + [Bench 161](../.benchmarks/161_i245_poc0_cofire_census.md)). PoC-1–4 were never run.

## 7. NEGATIVE-sparse record (PoC-0 kill, Bench 161, 2026-11-18)

**What ran:** the PoC-0 census over the Bench-094 wave scans (702 rows: 589 `ok` / 110 `compile_error` / 3 `timeout`; 580 distinct crates, 122 repeated (wave, crate) rows; wave 1 totals-only, excluded per the filing). Instrument `scripts/i245_poc0_cofire_census.py`, outputs `.benchmarks/data/094_cofire_census/`.

**The kill, under the pinned convention (ok-only + per-crate pair-union):** 78 crates ≥ 40 PASS, **57 edges weight-≥3 < 60 FAIL** — NEGATIVE-sparse by 3 edges. Community discovery at min-support 3 was pre-registered as too sparse on exactly this reading.

**What the census resolved (the filing's motivation):** the two prior parses' disagreement is fully attributed — claude-verdict's 105/1318/124 = all+perwave distinct pairs; coordinator's 99/111 = ok+perwave and **1579 = per-wave edge instances with multiplicity, not distinct pairs**. Both prior parses were per-wave readings; the viability they suggested (111/124 ≥ 60) was the double-counting artifact of the 122 repeated rows. Deduplicated to one entry per crate, the ge3 core is 57 (ok-only) / 68 (all-rows). The instrument reproduces both prior parses exactly — two parsers, one census, zero unexplained deltas.

**The disclosed sensitivity (why the pin is not bar-shopped):** the ok-only filter was pinned before any census ran (`compile_error` scans are truncated runs sampling a different lint regime — rationale in the claim commit `c0f8e83b44c`). The all-rows variant reads 68 ≥ 60 (pass by 8); flipping the pin after seeing 57-vs-68 is the post-hoc bar-shopping the pre-registration exists to prevent. Both margins are razor-thin on a bar designed as a sparseness floor.

**Re-open condition (new data, never a re-parse or a re-pin):** a wave-1 per-crate re-run (`094_t01` is totals-only; the filing itself priced this path "filed separately, do not block PoC-0") adds a fourth wave to the population. A re-open files a NEW issue citing this section + Bench 161, keeps the Bench-161 PoC-1 pre-registration (Louvain-class, min-support 3, degree-preserving permutation null, 1000 draws, one-sided p ≤ 0.05 at size ≥ 3), and re-reads the SAME kill bar on the new pinned population.

**Scope of the demotion:** the paper distillation (§1–§3) stands; the demoted tier is the rulewright CONSUMER fusion (§4's co-fire × community-detection × community-level audits) — unproven and now kill-gated on population sparseness. §5's secondary/game-side analogs inherit the lesson: before any community-detection consumer, census the co-fire population against its own sparseness bar FIRST (the PoC-0 pattern).

## References

- Goodfire blog: <https://www.goodfire.com/research/neuron-communities> · preprint page: <https://www.goodfire.com/preprints/neuron-communities> · arXiv: [2611.23112](https://arxiv.org/abs/2611.23112)
- Prior art: Horta et al., "Extracting knowledge from Deep Neural Networks through co-activation graphs" (2021) · Gross et al., "Co-Activation Graph Analysis of Safety-Verified RL Policies" (2025, Louvain) · Anthropic circuit tracing (2025)
- This repo: `.benchmarks/092_component_necessity_flip_rate.md` (swap ≈ disable) · `.benchmarks/094_default_level_fired_histogram.md` (wave-4: 663 crates, 186 distinct lints — totals only; per-crate sets in `data/094_t0{2,3,4}/results_wave*.jsonl`) · `HISTORY.md:1001` (Issue 094 negative)
- katgpt-rs: `.research/053`, `.research/393`, `.research/394` · riir-ai: Issue 875, Research 365

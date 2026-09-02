# Issue 64: Modelability oracle — scaffold marginal value as a latent-space quality measurement (GenFirst fusion)

**Status:** Open — **Phase 1 (the oracle primitive) DONE 2026-09-02**: `src/modelability.rs` (always-compiled, std-only, zero deps) — `scaffold_value(population, config) -> ModelabilityReport` with the measured design: baseline = single-mode held-out MSE; scaffold = deterministic k-means++/Lloyd (fixed-seed LCG, fixed-stride hold-out, empty-cluster farthest-point rescue); raw gain = `ln(baseline_mse / scaffold_mse)`; **null-calibrated value = `v_raw − v_null` against a diagonal-matched null population** (same n/d/mean/per-dim σ, no cluster or cross-dimensional structure — the `mi_est` permutation-test pattern). The null calibration was FORCED by measurement: a single 2-D gaussian scores `v_raw` ≈ 1.9 at k=8 — pure k-means quantization gain, not structure — and the first failing gate (diffuse < 1.0) caught it exactly as designed. Gates 6/6: clustered-vs-diffuse separation (blobs > 1.5 excess nats, single gaussian |v| < 0.75, ordering + 1.0 margin), held-out overfit resistance (k = n_train must not collapse the value), determinism ×3 bit-identical, degenerate population → 0 not NaN, panic validation, split accounting. Diagnostic tier only (doc-pinned); never gates a heal decision. lib 1018/0, clippy 0. **Phase 2 (the pre-registered Spearman sweep) remains — wiring map below.**

## Phase 2 wiring map (recon done 2026-09-02 — execute as the next unit)

- **Variants (≥ 4, must differ in top-1)**: the `IndexTokenizer` ladder in `src/draft/corpus_index.rs:100` — `CodeDefault` / `OpAware` / `OpAwareBigram` (measured T3.5 spread: clippy top-1 54.2 → 70.8, Bench 027) × the three domain surfaces (clippy 48 / perf 40 / kernel 30 spans in `retrieval_eval.rs`) gives 9+ (variant, population, top-1) points; `run_eval_with(RetrievalConfig { tokenizer, .. })` produces each variant's report (`EvalReport.domain_metrics[..].recall(1)`).
- **Populations for the oracle**: the per-domain CORPUS rule-pattern embeddings (the index chunks `RuleIndex` builds — one embedding per rule pattern + significant lines) and/or the eval-span embeddings; extract via the same `AstChunker → ModellessEmbedder` path the index uses so the oracle measures the space the retriever actually searches.
- **Metric**: Spearman rank correlation of `report.value` vs per-variant top-1 across the sweep points. Gate ≥ 0.8 ⇒ the oracle predicts where embedder investment pays; < 0.5 ⇒ kill (record negative, keep the harness).
- **Harness home**: `tests/modelability_sweep.rs` (integration, requires `latent_retrieval` — pulls riir-rag), reusing the `run_tokenizer_sweep` scaffolding in `retrieval_eval.rs`.
- **Pre-registered in this file's protocol below — do not restate it, execute it.**

## Source

Research 437 (riir-train/.research/437_GenFirst_Generation_Before_Reconstruction.md; arXiv:2608.29335 "GenFirst", Table 5). The paper's striking measurement: on frozen reconstruction-VAE latents, adding a normalizing-flow scaffold to the AR prior is worth **74 FID** (110.10 → 36.33); on GenFirst end-to-end latents it is worth **0.15** (5.27 → 5.12). The marginal value of a density-modeling scaffold is itself a MEASUREMENT of how well-shaped the latent space is.

## The healer fusion (priority-ladder surface #2)

Our analog surfaces carry learned-or-modelless latent populations whose downstream quality is known and measurable:

- Fix-trajectory memory embeddings (`LatentFixMemory` / `TrajStore`, evidence-tiered, retrieval over 8-D latents)
- Retrieval span embeddings (`AstChunker` → `ModellessEmbedder`, the 130-span eval + retrieval floors)

**Oracle:** fit a CHEAP non-GD scaffold to a latent population (blake3-bucketed unigram/bigram over discretized latents, or k-means/Lloyd — EM, not gradient descent), measure held-out fit. Hypothesis: **scaffold marginal value ≈ poor latent shape** — i.e., if a trivial density model already explains the space, investing in a better embedder/reranker is the wrong next move; fix the space (corpus coverage, chunking, evidence tiers) first.

## Pre-registered validation protocol (falsifiable)

- Sweep ≥ 4 corpus/embedder variants on the retrieval surface (the eval fixtures + floors infrastructure already exists).
- Metric: Spearman rank correlation between oracle ranking (scaffold value) and measured downstream top-1 across variants. **Gate: Spearman ≥ 0.8** ⇒ the oracle predicts where embedder investment pays.
- Paper's signature claim reproduced in-domain: high-modelability spaces gain < ε from reranker/embedder upgrades (a pre-flight diagnostic that redirects investment).
- Falsification: Spearman < 0.5 ⇒ the oracle is noise on our populations; record negative, keep the harness as the artifact.

## Constraints

- Suggestion/diagnostic tier only — never gates a heal decision (the L4 relevance-gate lesson: a syntactic/cheap signal must not masquerade as a relevance judge).
- Modelless only: bucketed counts + k-means; no trained density models on the heal path.
- Precedent for the shape: bench_440's "G1 MARGINAL" style honest diagnostics; edge_lora dist guard's advisory (warn-only) posture.

## Related

- Research 437 (No-GD advocate extraction #11, ranked highest-ceiling); katgpt-rs Issue 708 (the entropy substrate P1 feeds the same quality axis); riir-clippy Plan 005 retrieval floors; Bench 024 floors re-pin

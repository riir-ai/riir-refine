# Issue 64: Modelability oracle — scaffold marginal value as a latent-space quality measurement (GenFirst fusion)

**Status:** Open — fusion idea, novelty TBD; validation protocol pre-registered; heuristic-tier candidate for the healer

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

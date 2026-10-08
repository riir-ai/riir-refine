# Issue 157: Span-conditional contrastive negatives in RuleIndex retrieval (v4 `rejected` labels as a negative channel)

**Status:** OPEN (fusion idea, novelty TBD — owner triage) — filed 2026-10-08, surfaced by the katgpt-rs research PASS on arXiv:2509.09691 (wave/resonance memory; the paper itself was PASSed — no adoption — but its context-conditional polarity framing exposed this gap in our own retrieval).

## The idea

The v4 sync payload's per-row `rejected` field (the client matcher's proposed rule ids minus
the applied rule) is a **span-conditional contrastive negative** already computed client-side
on every fix run. Today it feeds the rule-suite MINT lane (training negatives for suite
minting). It does NOT feed retrieval.

Current demotion is rule-GLOBAL: `EvidenceTier::Withdrawn` demotes a rule everywhere. The
gap: a rule can be wrong for ONE span shape and right for others — global Withdrawn throws
away the conditional information; plain retrieval never sees it.

## Mechanism sketch (modelless, real-vector form of the paper's "anti-phase")

- On a rejected proposal, store `(ModellessEmbedder(span), rule_id)` as a negative pair
  (Warm-tier, alongside the existing trajectory store).
- At retrieval, down-weight rule R by `sigmoid(-dot(q, neg_emb_R) * λ)` when a stored
  negative for R is close to the query — i.e. rule R loses rank ONLY for span shapes it has
  been rejected on. This is the paper's context-conditional polarity without complex
  embeddings: the negative companion is the "anti-phase" twin, dot-gated, sigmoid-shaped.

## Delta vs shipped substrate (signal-diff)

- `EvidenceTier` (self_evolve): history-only, rule-global — not span-conditional.
- RuleIndex fusion (BM25 + latent + optional dense rerank): no negative channel at all.
- `score_norm.rs`: channel normalization, not contrastive down-weighting.

## Why TBD, not GO

- Standard contrastive-retrieval ideas exist; the candidate delta is narrow: span-conditioned
  + modelless (BLAKE3+DFT embedding, zero training) + already-available labels.
- BM25 dominates the current fusion (latent term is weak on the clippy corpus — T3.5 sweep);
  a negative channel on the WEAK term may not move end-to-end precision. Price before building.
- Provenance caveat: negatives are self-reported by the matcher; a systematic matcher bias
  would poison the negative memory. Needs the same consent/quality gates as fix-stats.

## Tasks

- [ ] Owner triage: is the retrieval-negative consumer wanted beside the mint consumer?
- [ ] If GO: PoC on the oracle fixtures (`src/draft/retrieval_eval.rs` fixtures) measuring
      top-1 precision delta with vs without the negative channel; G-gate vs incumbent;
      only then a plan.

# Issue 157: Span-conditional contrastive negatives in RuleIndex retrieval (v4 `rejected` labels as a negative channel)

**Status:** OPEN (tripwire — the P0 probe ran, verdict **DECLINE-for-now on volume**; re-open trigger recorded below) — filed 2026-10-08, surfaced by the katgpt-rs research PASS on arXiv:2509.09691 (wave/resonance memory; the paper itself was PASSed — no adoption — but its context-conditional polarity framing exposed this gap in our own retrieval).

**OWNER TRIAGE RESOLVED 2026-10-10** (the task below; Claude verdict session `2b8a1316`, rounds 1–3 → AGREE; probe + record: [`.benchmarks/111_issue157_negative_channel_p0.md`](../.benchmarks/111_issue157_negative_channel_p0.md)). Verdict: **the retrieval-negative consumer is NOT wanted today — the local store carries ~0 negatives** (12 pairs, one span; the `rejected` field landed 2026-10-08), so the channel is vacuous on current data (kill criterion 1). NOT a mechanism negative: the census found the flip target exists (4 of 15 miss spans have gold at rank 1 — one demotion of the wrong top-1 promotes it; 2–5 near-tie at the pre-registered margins; miss-set n = 15). Two data-model blockers recorded for any future P1: the store does not persist pre-fix span text (certification of acceptable-vs-wrong is impossible), and the eval corpus has zero authored acceptable sets. **Re-open trigger:** ≥ 60 negative pairs across ≥ 10 distinct spans in the local store (or any live store) — then: certification first, census re-run, only then the P1 A/B with the round-1 gates (paired LB95 top-1 delta + zero regressions vs baseline + per-family retention + pre-registered λ/threshold). The census instrument: `cargo run --release --features latent_retrieval,clippy_lints,rust_perf,kernel_opt --example p157_near_tie_census`. The mint consumer (the v4 label's existing consumer) is untouched; fleet rows stay out of scope pending a separate consent decision.

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

- [x] Owner triage: is the retrieval-negative consumer wanted beside the mint consumer? — **RESOLVED 2026-10-10: NO for now (volume-vacuous), tripwire above.** Verdict chain: Claude verdict rounds 1–3 AGREE; P0 ran per the amended spec (`.benchmarks/111`); DECLINE-for-now on kill criterion 1 (volume ~0); census shows the flip target exists; re-open trigger ≥60 pairs / ≥10 spans.
- [ ] If GO: PoC on the oracle fixtures (`src/draft/retrieval_eval.rs` fixtures) measuring
      top-1 precision delta with vs without the negative channel; G-gate vs incumbent;
      only then a plan.

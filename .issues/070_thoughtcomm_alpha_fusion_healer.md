# Issue 070: ThoughtComm healer fusion — α-fusion retrieval arm + rerank diversity guard

**Status:** Open — T1 pre-check FIRST (the arm may be dead on arrival; measure before building)
**Source:** arXiv:2510.20733 "Thought Communication in Multiagent Collaboration" (NeurIPS 2025) — distilled in `../riir-ai/.research/364_ThoughtComm_Agreement_Tiered_Thought_Routing.md`
**Priority:** P2

## Why

The healer fans out each span across domain indexes (clippy/perf/kernel/…) but never computes **cross-domain agreement on the fix pattern**: each domain retrieves independently, pools are per-domain, and `RerankMode::Structural` re-ranks within one pool. The paper's α_j (how many agents' states depend on latent item j) is the count-based sibling of shipped RRF (rank-based fusion, `rrf_accumulate` = `1/(K+rank+1)`, implicit multi-channel agreement via summation — but channels are retrieval modalities, not agents, and nothing tiers by count). Secondary import: the paper's consensus≠accuracy discipline (Appx C.2) maps onto the measured Issue-024 pool-crowding failure — a rerank pool can converge (high agreement) on the wrong rule while the right one sits at rank 13.

## Tasks

- [ ] **T1 (pre-check, cheap):** measure the **cross-domain agreement rate** on the composite fixture corpus (Issue-017 fixtures + `eval_fixtures_composite.rs`): how often do ≥2 domain indexes retrieve rules/patterns that address the same span shape? If the rate ≈ 0 (per-domain corpora are disjoint by construction — rule ids cannot collide cross-domain), the α-fusion arm is **dead on arrival** for rule-id fusion and must be re-scoped to pattern-text overlap or closed with that number recorded here.
- [ ] **T2 (if T1 shows signal):** α-fusion arm in the rerank path — count how many domains contribute a fix pattern for the span, tier-weight (σ-saturated ladder, never a hard gate), fuse with (not instead of) RRF + structural rerank. Gate: recall@k on the composite corpus vs additive and RRF under the pinned-regime discipline — **no promotion without a regime where it wins** (expect scope-limited; RRF's rank information is strictly richer).
- [ ] **T3 (independent of T1, P1):** rerank-pool **diversity / private-retention guard** — a collapse detector beside `RerankMode::Structural`: if the top-k pool collapses to one rule family while the oracle label sits outside it, flag (the Issue-024 crowding class, made a first-class diagnostic). G1 planted collapse detected / diverse control passed.
- [-] **T4 (deferred):** agreement-weighted `--suggest` context assembly (weight suggestions by how many domains corroborate) — reopen only if T2 lands and suggests-layer evidence accumulates.

## Non-goals

- No LLM jury/fixer surface (the corpus-rule jury closed NEGATIVE, Issue 036; the fixer lane is demoted/UNARMED).
- No new corpus structure — this composes existing fan-out retrieval.

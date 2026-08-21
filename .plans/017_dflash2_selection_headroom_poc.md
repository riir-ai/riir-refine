# Plan 017 — DFlash2 Selection-Headroom POC (drafter mechanism × our ruliology, no target model)

**Status: CLOSED NEGATIVE 2026-08-21.** Phase 1 ran; the T1.2 stop rule fires on the
selector-addressable band (0 of 101 spans, bar 2pp). Phases 2-4 are closed UNBUILT — G1's
denominator is empty. Raw tables:
[Bench 043](../.benchmarks/043_dflash2_selection_headroom_negative.md); axis verdicts backfilled
into [Research 077](../.research/077_dflash2_draft_only_distillation.md) §PoC addendum. Issue 034
removed per the noise-reduction rule.

**Date:** 2026-08-20
**Origin:** owner ask — "POC DFlash2 as drafter for `cargo heal`: draft quality + our ruliology,
does it boost coverage?" — with the constraint **drafter + our stack only, NO full target model**
("yes we cheating but deal with it").
**Research:** [Research 077](../.research/077_dflash2_draft_only_distillation.md) — the artifact
cannot generate standalone (no tok_embd / no lm_head / encoder consumes 5×5120 target features per
position); what transfers without weights is the **selection-headroom mechanism**.

## Goal

Convert retrieval-pool selection headroom into healed coverage: where the correct fix is *in* the
top-K candidate pool but not at top-1 (the Bench 024 pool-crowding phenomenon = DFlash 2's
Recall@1 85.4% vs Recall@16 99.5% in our domain), a DFlash2-style **pairwise-coherence path
selector** over our latent embeds should recover misses that per-candidate scoring cannot.

This is the only honest reading of "drafter + our stack proof, no full model": the modelless
candidate pool IS our drafter; DFlash 2's contribution we can prove without target weights is its
selector.

## Non-goals (owner calls, 2026-08-20)

- **No target model in this plan.** `Qwen3.8-27B-GGUF` is not downloaded/used; local
  `Qwen3.6-27B-UD-Q4_K_XL.gguf` is NOT a substitute (wrong generation — the drafter distills
  Qwen3.8-27B states specifically). Weights on disk are kept untouched.
- [-] **POC-B (DEFERRED — show-stopper, owner call 2026-08-20):** DFlash2-accelerated target as
  `L4Fallback` via llama-server, re-running the Bench 039 reachable-set harness (bonsai: WIN 5 /
  REGRESSION 4). Two independent blockers: (a) the artifact cannot run standalone (Research 077 §2
  — no `tok_embd`/`lm_head`, encoder consumes 5×5120 target features/position); (b) the in-house
  DSpark precedent closed this class negative on ALL axes we measure (Bench 656: separate-drafter
  family DFlash/DSpark = STRUCTURAL batch-1 non-amortization; Issue 717 Benches 693-697: corpus-proxy
  quality LOST to the modelless bigram table 0.185 vs 0.065, wall-clock FAIL at the chain seam on
  BOTH substrates 0.326x/0.252x Metal + 0.84x/0.32x 4090). **No new test needed — the class verdict
  is established.** Reopen only if: a standalone-capable drafter ships (own embd+head), or a
  multi-request consumer changes the batch-1 premise. Staged + kept for that reopen: llama.cpp
  dflash2 build @ `/Users/katopz/git/llama.cpp` (`5ecbe1a`, Metal), drafter Q4_K_M at
  `riir-train/data/`.
- No new neural inference anywhere in riir-clippy (modelless-first mandate holds).

## Phase 1 — Measure the headroom (gate zero; honest stop rule)

- [x] T1.1 Build the headroom probe (`src/draft/headroom_probe.rs` + the `t1_dflash2_*` tests in
      `tests/uncontaminated_g1.rs`; commit `78d1934` + follow-up). Measured: pool@16 contains the
      correct rule for 100% of 101 spans (ZERO retrieval misses), so the whole band is ordering.
      recall@1 84.2% (`Off`) / 93.1% (`Structural`, shipped) / 97.0% (`All`).
      ORIGINAL: for every eval span where the pipeline's chosen fix fails
      (wrong rule, non-parsing, or verified-fail), check whether a *correct* candidate existed in
      the top-K of the retrieval pool (K = 8, 12, 16) — reusing the Issue 017 eval fixtures
      (130 spans: 48 clippy real-clippy-labeled + 40 perf + 30 kernel probes + 12 clean) and the
      clippy-bench composite set (28 multi-lint spans). Output: `recall@1` vs `recall@K` table +
      the miss taxonomy rows that are "selection misses" (correct candidate present, not chosen).
- [x] T1.2 **Stop rule: FIRED.** Raw band is +6.9pp at the shipped default, but decomposing the 7
      residual misses by what would fix each gives 4 type-blind-exclusion (Bench 022/027 price,
      `RerankMode::All` recovers), 1 `RERANK_POOL` depth (rank 15), 2 unfixable perf targets (no
      proposer → selecting heals nothing), and **0 selector-addressable**. On the uncontaminated
      real-clippy surface the shipped mode reads +0.0pp with a verified-powerful instrument
      (`rerank=off` control: +16.0pp). Negative recorded in Bench 043; no selector built.
      ORIGINAL WORDING BELOW (kept for the record — the literal reading of the raw band would have
      said PROCEED; the addressable band is the honest denominator):
      if `recall@16 − recall@1 < 2pp` on both surfaces → the residual headroom
      is noise; record the negative in the bench doc, close the issue, do NOT build the selector.

## Phase 2 — The selector (feature `dflash_selector`, opt-in, implies `latent_retrieval`)

- [-] T2.1 (CLOSED UNBUILT) `SelectorModel`: keep top-K candidates per span (pool depth already 12 — reuse),
      score **adjacent pairs** `S(a,b) = U(b) + σ(⟨A(a) ⊙ H(ctx), B(b)⟩)` where `U(b)` is the
      existing per-candidate score (BetaPosterior/evolution, unchanged), and `A`/`B` are
      deterministic low-rank codebooks derived from the per-rule 8-D `ModellessEmbedder`
      directions (BLAKE3-seeded — modelless, no training). `H(ctx)` = the span's latent embed as
      the context gate. **Sigmoid, never softmax** (house rule). Zero-alloc: fixed-size
      `[f32; 8]` scratch, no per-pair heap.
- [-] T2.2 (CLOSED UNBUILT) Path trace: greedy best-successor walk over the pair-score matrix (the DFlash 2
      selector's sequential step — O(K²) on a 12-deep pool = 144 dot products, µs-scale),
      producing the chosen candidate *and* the runner-up set for diagnostics.
- [-] T2.3 (CLOSED UNBUILT) Composition seam: the selector runs AFTER `RerankMode` instantiation filtering and
      BEFORE `HealPipeline` fix application; `--fix`/`--suggest` respect it only when the feature
      is on. Default-off until gates pass.

## Phase 3 — Gates (falsifiable; decline-equivalent = current top-1 selection)

- [-] G1 (UNREACHABLE — empty denominator) **Coverage:** on the T1 selection-miss set, selector recovers ≥ 50% of the measurable
      headroom (e.g. headroom 6pp → ≥ 3pp healed), verified by real `cargo clippy`/proposer
      ground truth — no self-graded wins.
- [-] G2 (CLOSED UNBUILT) **Trust:** `created_rate` (new lints introduced) does not regress on clippy-bench
      (Bench 014 axes; created is the trust axis — a coherence win that creates warnings is a loss).
- [-] G3 (CLOSED UNBUILT) **Latency:** selection adds < 100 µs/span release-mode (12-candidate pool, 8-D dots);
      the whole `--suggest` path stays under the Bench 018 budget shape.
- [-] G4 (CLOSED UNBUILT) **Alloc:** zero steady-state heap allocations (counting allocator, 50-repeat scan —
      the `SuggestScratch` pattern).
- [-] G5 (CLOSED UNBUILT) **No-regression:** default-feature suite + `latent_retrieval` suite counts pinned
      unchanged when the feature is OFF (byte-identical path); feature ON adds its own tests.

## Phase 4 — Record + promotion decision

- [x] T4.1 Bench doc — landed as
      [`.benchmarks/043_dflash2_selection_headroom_negative.md`](../.benchmarks/043_dflash2_selection_headroom_negative.md)
      (renamed from `_goat`: there is no GOAT to record, only the measured negative). Raw tables
      for T1 + the three instrument checks, honest verdict per axis, Research 077 PoC-addendum
      backfilled.
- [x] T4.2 Promotion: NONE — nothing to promote. Decision recorded in Bench 043.
      ORIGINAL: feature ON in `ci_feature_guard.sh` layer coverage; default-on only if
      G1 ≥ 50% recovery AND G2-G5 pass AND the recovered misses are real-production-shaped
      (not fixture artifacts). Otherwise stays opt-in with the annotation (`filter-count-to-any`
      precedent for annotated regime-dependent rules).

## References

- Blog: https://inco.ai/blog/dflash2/ (Table 1 selection headroom; selector formula; conv)
- PR: https://github.com/ggml-org/llama.cpp/pull/27342 (`5ecbe1a`)
- DSpark precedent (the show-stopper class evidence): katgpt-rs Bench 656 ("DSpark = DFlash
  pattern", separate-drafter batch-1 structural failure) + Bench 663 (modelless bigram winner,
  23 ns/step); riir-ai Issue 717 Benches 693-697 (artifact ported + measured closed-negative).
- Our prior art: Bench 024 (pool crowding, `RERANK_POOL` 8→12), Issue 017 (top-1 58.3% → 96.0%
  via T3.5.5 re-ranker — the individual-fit selector this plan complements), Bench 039
  (L4-vs-decline harness, POC-B deferred target), Bench 014 (clippy-bench axes).

## Outcome (2026-08-21)

The DFlash2 selector premise is CONFIRMED in our domain and already harvested. The
`Off → Structural` re-rank step *is* the selector DFlash 2 contributes, reached independently in
T3.5.5 and resting on strictly better evidence — a proposer-visitor instantiation match is a
semantic witness that the rule applies, where pair coherence is a similarity. The residual band is
precision-bounded (the deliberately excluded type-blind over-matchers), not ordering-bounded, and a
pairwise selector over the same parse-only latent features carries no type information with which
to break that bound.

Follow-ups spun out (ordinary substrate work, NOT a selector reopen):

- `RERANK_POOL` 12→16 measurement against the Bench 018 latency budget (recovers 1 span).
- Perf proposers for `dyn-to-enum` + `shrink-to-fit-after-build` (turns 2 unfixable top-1
  candidates into healable ones).

Reopen the selector only if a future surface shows a **non-empty selector-addressable band** —
`decompose_headroom()` is the committed instrument that answers it, and it re-measures on every
corpus batch through the `--lib` suite.

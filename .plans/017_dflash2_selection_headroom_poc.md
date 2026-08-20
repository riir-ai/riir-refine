# Plan 017 — DFlash2 Selection-Headroom POC (drafter mechanism × our ruliology, no target model)

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
  Qwen3.8-27B states specifically). Weights on disk are kept untouched for the next POC.
- **POC-B (deferred, next POC):** DFlash2-accelerated target as `L4Fallback` via llama-server,
  re-running the Bench 039 reachable-set harness (bonsai: WIN 5 / REGRESSION 4). Prereqs staged:
  llama.cpp dflash2 build @ `/Users/katopz/git/llama.cpp` (`5ecbe1a`, Metal) + drafter Q4_K_M at
  `riir-train/data/` (acceptance 5.39 @ Q4_K_M per model card).
- No new neural inference anywhere in riir-clippy (modelless-first mandate holds).

## Phase 1 — Measure the headroom (gate zero; honest stop rule)

- [ ] T1.1 Build the headroom probe: for every eval span where the pipeline's chosen fix fails
      (wrong rule, non-parsing, or verified-fail), check whether a *correct* candidate existed in
      the top-K of the retrieval pool (K = 8, 12, 16) — reusing the Issue 017 eval fixtures
      (130 spans: 48 clippy real-clippy-labeled + 40 perf + 30 kernel probes + 12 clean) and the
      clippy-bench composite set (28 multi-lint spans). Output: `recall@1` vs `recall@K` table +
      the miss taxonomy rows that are "selection misses" (correct candidate present, not chosen).
- [ ] T1.2 **Stop rule:** if `recall@16 − recall@1 < 2pp` on both surfaces → the residual headroom
      is noise; record the negative in the bench doc, close the issue, do NOT build the selector.

## Phase 2 — The selector (feature `dflash_selector`, opt-in, implies `latent_retrieval`)

- [ ] T2.1 `SelectorModel`: keep top-K candidates per span (pool depth already 12 — reuse),
      score **adjacent pairs** `S(a,b) = U(b) + σ(⟨A(a) ⊙ H(ctx), B(b)⟩)` where `U(b)` is the
      existing per-candidate score (BetaPosterior/evolution, unchanged), and `A`/`B` are
      deterministic low-rank codebooks derived from the per-rule 8-D `ModellessEmbedder`
      directions (BLAKE3-seeded — modelless, no training). `H(ctx)` = the span's latent embed as
      the context gate. **Sigmoid, never softmax** (house rule). Zero-alloc: fixed-size
      `[f32; 8]` scratch, no per-pair heap.
- [ ] T2.2 Path trace: greedy best-successor walk over the pair-score matrix (the DFlash 2
      selector's sequential step — O(K²) on a 12-deep pool = 144 dot products, µs-scale),
      producing the chosen candidate *and* the runner-up set for diagnostics.
- [ ] T2.3 Composition seam: the selector runs AFTER `RerankMode` instantiation filtering and
      BEFORE `HealPipeline` fix application; `--fix`/`--suggest` respect it only when the feature
      is on. Default-off until gates pass.

## Phase 3 — Gates (falsifiable; decline-equivalent = current top-1 selection)

- [ ] G1 **Coverage:** on the T1 selection-miss set, selector recovers ≥ 50% of the measurable
      headroom (e.g. headroom 6pp → ≥ 3pp healed), verified by real `cargo clippy`/proposer
      ground truth — no self-graded wins.
- [ ] G2 **Trust:** `created_rate` (new lints introduced) does not regress on clippy-bench
      (Bench 014 axes; created is the trust axis — a coherence win that creates warnings is a loss).
- [ ] G3 **Latency:** selection adds < 100 µs/span release-mode (12-candidate pool, 8-D dots);
      the whole `--suggest` path stays under the Bench 018 budget shape.
- [ ] G4 **Alloc:** zero steady-state heap allocations (counting allocator, 50-repeat scan —
      the `SuggestScratch` pattern).
- [ ] G5 **No-regression:** default-feature suite + `latent_retrieval` suite counts pinned
      unchanged when the feature is OFF (byte-identical path); feature ON adds its own tests.

## Phase 4 — Record + promotion decision

- [ ] T4.1 Bench doc `.benchmarks/041_dflash2_selection_headroom_goat.md` — raw tables (T1 +
      gates), honest win/loss per axis, Research 077 PoC-addendum backfill.
- [ ] T4.2 Promotion: feature ON in `ci_feature_guard.sh` layer coverage; default-on only if
      G1 ≥ 50% recovery AND G2-G5 pass AND the recovered misses are real-production-shaped
      (not fixture artifacts). Otherwise stays opt-in with the annotation (`filter-count-to-any`
      precedent for annotated regime-dependent rules).

## References

- Blog: https://inco.ai/blog/dflash2/ (Table 1 selection headroom; selector formula; conv)
- PR: https://github.com/ggml-org/llama.cpp/pull/27342 (`5ecbe1a`)
- Our prior art: Bench 024 (pool crowding, `RERANK_POOL` 8→12), Issue 017 (top-1 58.3% → 96.0%
  via T3.5.5 re-ranker — the individual-fit selector this plan complements), Bench 039
  (L4-vs-decline harness, reused by deferred POC-B), Bench 014 (clippy-bench axes).

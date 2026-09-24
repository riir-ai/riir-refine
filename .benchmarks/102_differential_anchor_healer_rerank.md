# Bench 102 — differential-anchor query correction on the healer's latent leg (katgpt-rs Issue 882 P0 rider)

**Status:** MEASURED 2026-09-25. **Not promoted.** Stays opt-in (`differential_anchor`, default-off). A weak positive on the eval set (+1 span of 138) and a null on real-clippy labels.

Consumer #1 of katgpt-core `differential_anchor` (katgpt-rs Issue 882 P0, Bench 886 there). `q̂ = q − λ·ā` on the 8-D `ModellessEmbedder` query direction, where ā is the per-domain **mean-corpus** embedding (fit at `RuleIndex::build_with` over every indexed chunk). The BM25 leg, the rerank, and the dense leg are untouched.

- **Surfaces:** `RuleIndex::set_differential_anchor`, `LatentMatcher::with_differential_anchor`, and the env knob `RIIR_CLIPPY_DIFF_ANCHOR_LAMBDA=<f32>` for same-binary A/B of gates that build their own matcher (the `RIIR_CLIPPY_NO_TWO_STAGE` precedent).
- **Eval seam:** `retrieval_eval::run_eval_on(matcher)`.
- **Harness:** `tests/differential_anchor_ab.rs` (`--release --features differential_anchor`).

**Box:** M3 Max, AC power at 100%, swap 1.2 of 2.0 GB used, two sibling cargo builds running concurrently. Every figure here is a quality count, not a latency, so the load cannot move it.

## Hypothesis

The embedder is sigmoid-bounded, so every vector sits in the positive orthant and raw cosine carries a large common-mode floor. That would explain the Bench 027 refutation ("the 8-D latent term never changes a ranking"). If the floor is the cause, subtracting ā should make the latent term informative.

## Results

### G3 — the kill switch

- λ=0 is bit-identical to the uncorrected matcher: `to_bits` on every scored rule, every span, every domain, over more than 100 rules (floored).
- Liveness is armed: λ=1 moves scores.
- The env knob was proven to reach the index build. With it set, the liveness arm's "base" became corrected and the arm failed as designed.

### G1 — eval set (138 labeled spans, 7-domain default build)

| latent w | λ=0 top-1 | best λ | top-1 at λ* | Δ | recall@3 at λ* | win-count skew (λ=0 → λ*) |
|---|---|---|---|---|---|---|
| 1.0 (shipped) | 0.8986 | 0.25 (flat through 1.5) | 0.9058 | **+1 span** | 0.9420 (=) | 3.054 → **3.221 ↑** |
| 4.0 | 0.9058 | 0 | 0.9058 | 0 | 0.9493 | 3.221 |
| 16.0 | 0.8623 | 0.5 | 0.8913 | +4 spans | 0.9420 | 2.902 → 2.898 |

- Attribution at w=1, λ=0.5: exactly one flip, and it is a gain. Docker `instruction-casing` moves from top-1 `no-empty-continuation` to the correct rule. Nothing was lost.
- Per-domain top-1: clippy, perf and dist are unchanged at every λ. Docker gains the one span.

### G1 — real-clippy labels (`uncontaminated_g1`, env knob)

| λ | single-lint top-1 (25 spans) | composite top-1 (43 spans) |
|---|---|---|
| 0 | 100.0% | 90.7% |
| 0.5 | 100.0% | 90.7% |
| 1.0 | 100.0% | 90.7% |

Every mode, including `off` (pure retrieval), is identical at every λ.

## Findings

1. **The shared noise floor is real but is not what holds the latent leg back.** Removing it buys one span at the shipped weight, on a flat plateau (λ ∈ [0.25, 1.5] all give the same result). Raising the latent weight to 4 with no anchor recovers the same span and adds recall@3 (0.9420 → 0.9493). So the anchor is a weaker lever than a plain re-weight. What still limits the leg is the embedder's information content (an 8-D hash plus DFT), not an offset.
2. **Hub concentration rose (skew 3.054 → 3.221), the opposite of the P0 claim.** Hubness is not the defect on this surface. The one gained span is a correct rule taking a win from another rule, not wins spreading out.
3. **At an over-weighted latent (w=16) the anchor partly repairs the damage** (+4 spans). That is the regime where the common-mode floor actually costs rankings. It is irrelevant to the shipped config.
4. **Side observation, a separate lever and not this rider:** w=4 beats w=1 on this 7-domain eval (+1 top-1, +1 recall@3). Bench 027's latent-weight refutation was measured on the clippy-only corpus. Re-sweeping the weight on the seven-domain build is a separate question; n=1 per axis is not a promotion.

## Verdict

**Not promoted.** Promotion needs a GOAT on the real-labeled gate, and that gate is flat. The eval-set gain is one span. The feature stays opt-in as a consumer of the katgpt-core primitive. The substrate itself stays proven (Bench 886); this socket just has little common-mode signal to reject.

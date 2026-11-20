# Issue 251: Multiplicative (soft-AND) composition challenger for candidate selection

**Status:** Open — research-derived (riir-refine Research 424; lineage: Issue 033 / `selection_tether.rs`)
**Claimed:** katgpt-rs-d1@m3, 1795182880, expires 2026-11-22T13:54:40+00:00
**Priority:** low — the additive blend is bit-pinned cold-start identity; this is a challenger with a real risk of losing

## The idea

ITERLOGIC composes multi-source evidence by `Score = log p₁ + log p₂` (product in probability space) — "amplifies weak logical support": a candidate that relies on one poorly-supported source scores low, full stop. Our `select_best_candidate` composes the same KIND of signals additively: `W_EVO·evolution + W_RATE·reliability` with the weights hand-pinned 0.6/0.4 (`self_evolve.rs:134-136`; `selection_tether.rs` pins the additive cold-start identity bit-identically, Issue 033 lineage). Additive forgives a weak leg when the other leg is strong; multiplicative does not. That is a genuine composition-law delta, and the fixer has outcome-labeled evidence to adjudicate it.

## Scope

- [ ] **T1 — Feature-gated challenger** (`composition_product`, opt-in, default-off, never in the release set): score = Σ log-sigmoid of leg logits. **Input law (verdict round 1):** every leg score feeding the product must be sigmoid-bounded in (0, 1) — never a raw logit, never a clamped-at-zero value; a leg that can read exactly 0 would zero the whole product and must be refused at construction. House law: sigmoid, never softmax; sequential fold per the determinism law. The additive blend stays the DEMOTE and the cold-start identity.
- [ ] **T2 — Adjudication on evidence:** paired comparison over the stored trajectory corpus (replay `select_best_candidate` decisions under both laws against recorded outcomes): pick-agreement rate, quality delta with paired non-inferiority δ = max(1.0pp, 2.5·SE), and never-slower (alloc + time). Report-only first — the challenger DEMOTES itself unless it wins the paired bar.
- [ ] **T3 — Sweep, not re-pin:** if the challenger wins anywhere, the 0.6/0.4 blend is a hand-pinned composition constant the sweep exposes (Issue 033's own axis) — the landing is a measured law, never a re-pin of the constant.

## Bars

- Paired non-inferiority on selection quality over ≥ the corpus's n≥2 groups; a tie or loss DEMOTES in-landing (grade struck, feature demoted, verdict recorded).
- Never-slower: alloc-neutral and time-neutral or disclosed.
- No wire/behavior change at feature-off (byte-identical run): **the existing cold-start pins must keep passing with the arm OFF** — `selection_tether.rs`'s "cold start must reproduce W_EVO·p1 + W_RATE·p2 exactly" and the `t93_selection_regret` blend rows — they run in every posture and are the standing proof the challenger changed nothing when demoted.

## Why it might lose (named up front)

Two legs (evolution, reliability) is the small case — product-vs-additive differences are marginal at n=2 unless the corpus carries genuinely weak-leg candidates that additive blending has been promoting. If the paired delta lands within noise, the honest verdict is "no separation at two legs" and the law is recorded as a negative with the n-leg generalization question left open.

# Research 412: Rounding-Bug Atlas distill — TorchLean / FloatLib / LeanSoftmax numerics rewrites

> **Source:** Robert Joseph George, ["When Rounding Breaks Machine Learning"](https://www.robertj1.com/ai4science/rounding-bugs-in-ml-formulas/) (2026-11) + Sasha Rush & Robert Joseph George, ["Lean Verified Precision"](https://srush.github.io/LeanSoftmax/) (2026-11) — the companion verification arc to arXiv:2610.31642 (Barron et al.), itself already mined as Issue 185 / Batch ~185's softmax family. Code: `lean-dojo/TorchLean`, `lean-dojo/FloatLib` (arXiv:2609.19352), `srush/LeanSoftmax`.
> **Inspection date:** 2026-11-15 (M3, user-directed @distill)
> **Provenance:** both pages read LIVE over the wire (complete prose); NO clone — Lean repos, nothing Rust-shaped to grep; every quote below is from the live fetch. T0 at mine time re-derives each fixture from first principles regardless (fixtures are OUR Rust reproductions per the transcription law).
> **Status:** DISTILLED — pending owner decision
> **Queue slot:** BEHIND Batch 365 (hydradb) → nominal **Batch 366**, Plan 380

## Why this is a distill and not just research

The sibling note (katgpt-rs Research 673) owns the paper-companion *record*. This note owns the corpus question: the blog's ~15 rewrites are named patterns with bounded fix-spaces and regime conditions — the `sec`/Numerics family's exact shape (precedent: Issue 185 landed the softmax/CE rows of the SAME paper; Batch 352 landed the LoNE numerics family). The transcription law applies: source is Lean, fixtures are ours.

## Dedup gate (live greps, this session)

- `sec/corpus.rs` Numerics family = 7 rules: Issue-185 (`sigmoid-minus-one`, `softmax-onehot-collapse`, `neg-ln-softmax-prob`) + Batch 352 (`fp16-subnormal-product-power-of-two-rescale`, `wide-reduction-precision-split`, `fused-activation-approximation-expansion`, `position-keyed-additive-cache-mask`). **Every softmax/CE row of the blog is already covered** — the gate working; not re-mined.
- Zero corpus hits for: gelu-tanh cancellation, expm1−d KL, two-pass variance, decay-gate low-precision rounding, logit-clamp absorption, single-rounding sums (grep `expm1|log1p|two-pass|variance|decay|gelu` across `src/domains/sec/` + both vocabularies).
- Adjudications owed in-rule at authoring: R1 vs `fused-activation-approximation-expansion` (B352 — fused-NE-approx axis vs FP-cancellation axis; different fix-spaces, compose) and vs `sigmoid-minus-one` (family sibling, different operator); R6 vs `wide-reduction-precision-split` (split-at-width axis vs single-rounding axis); R4 vs `fp16-subnormal-product-power-of-two-rescale` (products-underflow vs value-rounding — different mechanism).

## Ranked candidates

| # | rule_id | grade | before → after | regime | upstream state |
|---|---|---|---|---|---|
| R1 | `gelu-tanh-cancellation-sigmoid-form` | **B+ lead** | `0.5x(1+tanh z)` → `x·σ(2z)` (algebraic identity, NOT the erf switch) | deep-negative tail: `1+tanh` rounds to 0 when 2e^{−2z} < half-ulp-below-1 (2.98e−8 f32); blog-measured −0 vs −8.4e−11 at x=−6, PyTorch/JAX/Transformers/TorchLean all −0 | their own analysis (fresh, 2026-11); not a landed fix anywhere tested — open fix space, transcription-only posture |
| R2 | `kl-penalty-exp-minus-d-cancellation` | B+ | `e^d − d − 1` → `expm1(d) − d` | small \|d\| (collapse at d = 2^−12 f32: `d` is exactly 2048 ulps above 1.0 and `d²/2 = 2^−25` is exactly a quarter-ulp, so correctly-rounded `exp(d)` rounds to exactly `1+d` and the form returns 0.0; true value 2.98e−8); RL KL penalties / divergence estimators | **UPSTREAM-NAIVE and LIVE IN-STACK**: TRL carries the naive form (transcription only, NEVER a PR) AND our own `riir-train-engine` runs it at `nf_grpo.rs:252` + `svr_driver.rs:1151` — in-stack counterpart FILED as riir-train Issue 691 (fixture-first fix), the same way R1 ↔ katgpt-rs Issue 1220 |
| R3 | `variance-cancellation-two-pass` | B | `E[x²] − E[x]²` → two-pass `mean((x−μ)²)` | large common mean + small variance (PG `VarAggregation`: −2 vs 8/9; JAX standardize documents + ships `algorithm="stable"`) | JAX's own stable form = decision content; PG naive = open upstream |
| R4 | `decay-gate-precision-collapse-low-precision` | B− | log-space decay increments / wider accumulator | low-precision lanes only (bf16 EMA stuck at 1/64 ×5000 steps; xLSTM forget-gate decay → 1.0); CARRIES the Issue-940 carve-out (f32 sync-crossing reductions stay order-pinned — the determinism law is not overturned, the regimes are disjoint) | xLSTM upstream-naive (their `parallel_stabilized_simple`) |
| R5 | `logit-clamp-epsilon-absorption` | B− | `x.clamp(eps, 1−eps)` with eps=1e−8 → clamp absorbed (1−eps rounds to 1.0 f32) → `+∞` downstream | eps ≤ half-ulp near the clamp point; fix: clamp in the output domain | upstream-naive (torch.special.logit) |
| R6 | `sum-single-rounding-low-precision` | B− face-or-rule | incremental low-precision sums → exact sum rounded once | binary16/bf16 long reductions (label-smoothing batch sums → +∞); MUST cite the Issue-940 carve-out in-rule; adjudicate vs `wide-reduction-precision-split` both directions | planned upstream ("Planned" in their proof table) |

**Drops:** DDPM noise-level `sqrt(1−ᾱ)` log-accumulation (no `ᾱ` cumprod in our dllm — `NoiseSchedule` is a masking schedule, verified by read; corpus-eligible only as generic-consumer rule, cut for yield), Weibull `1−CDF` (no stack surface), soft-margin/ℓ₃-norm/focal-loss NaN rows (niche, per-case fixes already upstream-tracked: torchvision #9743/#9751, TFP #2040), rsqrt/floor-div single-rounding (≤1 ulp everywhere in f32 per their own table — no magnitude story), squashed-Gaussian epsilon (changes the function itself — their own carve-out).

**Verdict**

**YES B− corpus (`sec`/Numerics; nominal Batch 366) / LANE INTEL HIGH** (TWO live in-stack counterparts, both FILED this session: the tanh-GELU lane divergence → katgpt-rs Issue 1220, and the k3 KL collapse → riir-train Issue 691 at `nf_grpo.rs:252` + `svr_driver.rs:1151`; no league/perf mapping) / **PRODUCT-COVERAGE YES** — seeds the cancellation-rewrite subfamily beyond softmax: the Numerics family is 7 rules against an ecosystem-wide surface (every RL trainer, every low-precision lane, every variance/normalization site). Follow-up pool: Goldberg "What Every Computer Scientist…", Higham's *Accuracy and Stability of Numerical Algorithms*, Blanchard/Highams softmax-LSE paper, Prieto arXiv:2501.04697, Herbie; accumulation clause per the staged-charter precedent.

**GOAT:** no benchable candidate — criterion 1 fails for a DOMAIN reason (the win metric is gradient survival / sign preservation, not latency; the Pareto validator cannot host it). Per the two-class split this is the per-rule `NotBenchable` annotation class (Issue-185/B352 posture), not a demotion; verdict rides decision content × novelty × dedup × yield. R1 carries the strongest magnitude story (whole-tail erasure vs ≤1-ulp rewrite) and the falsification arm is exact: a property test that must FAIL pre-rewrite and PASS post-.

**Yield:** 3–4 rules + 1 face after ~50% coordinator falsification (5 candidates + R6 face).

**Caveats:** Lean sources, quotes blog-stated (live fetch, no clone — T0 re-derives every fixture from first principles); the proofs backing R1/R2 rewrites are upstream "measured" or "bound pending" (R1) and "proved on a restricted domain" (R2) — our fixtures assert the Rust-side behavior, never upstream claims; the underflow-fallback half of the softmax rewrite (log-domain tail) is deliberately NOT a candidate (our f32 lanes never hit it; `fixed_log_softmax` documents its own underflow ceiling).

**Bottom line:** mine as one batch behind Batch 365 — lead with `gelu-tanh-cancellation-sigmoid-form`; the lane-intel deliverable (katgpt-rs Issue 1220) is ALREADY FILED this session. NOTHING INVENTED — every quote from the live fetch, every corpus/issue claim a live grep this session.

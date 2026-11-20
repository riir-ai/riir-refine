# Issue 254: Lepski cascade per-span adaptive fan-out K — consumer of katgpt-core `lepski_cascade`

**Status:** open — routed from katgpt-rs Issue 1229 (primitives + G1/G4 + dash_attn arm landed upstream, katgpt-rs `e866932a1`); unclaimed; first task is a kill-check against Bench 141
**Claimed:** zed-m3-i254kill@m3, 1795200165, expires 2026-11-22T18:42:00Z — scope: the kill-check (read Bench 141 + Research 681 + Issue 1229; adjudicate cost + quality axes at today's corpus sizes; watch-row close or live-wire per the verdict)

Consumer of `katgpt_core::lepski_cascade` (`estimators` umbrella; `A_DEFAULT = 1` serving
constant, katgpt-rs Issue 1229 G1 cell 2). Wire, never fork.

**Idea:** per-span adaptive K over the heal-lane fan-out — retrieve ONCE at K_max, cascade over
prefix-k of the ranked candidates, serve the largest k whose estimate agrees with every smaller
one within A·τ̂. The prefix form keeps one retrieval, so the Bench-126 call-count floor is not
re-paid; the measured global K ≥ 6 floor becomes a per-span derived floor.

- [ ] **Kill-check first:** Bench 141 measured flat dense per-span fan-out RIGHT at today's
      sizes (112–1073 rules/domain, far below the ~32k-bucket LSA crossover). Read whether an
      adaptive K changes the COST axis at all there; if not, and the quality axis has no
      measurable headroom on the heal corpus, record "watch row, no gain at current scale" with
      the re-open trigger (rules/domain approaching the crossover) and close.
- [ ] If live: wire behind its own feature, report-only; σ̂ per surface (ranked scores without a
      covariate ⇒ `sigma_hat_bulk_mad_f32`, see katgpt-rs Issue 1229 for the derivation).
- [ ] Gate: paired heal-success vs fixed-K at the serving posture, pre-registered before the run.
- [-] Watch row (not a task): `elo_k_eff` per-rule adaptive evidence floors (n_floor 16 → widen
      while successive Beta-LCB estimates shift ≤ τ̂) — secondary per Research 681 §2.3(b).

Ref: katgpt-rs `.research/681_Locally_Adaptive_kNN_Neighbourhood_Selection.md`, katgpt-rs Issue 1229.

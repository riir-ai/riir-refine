# Research 061 — Batch 54: regularizer-above-noise-floor-coupled-to-damping-cap

**Status:** RECORD (batch landed, closed)
**Date:** 2026-08-18
**Source:** riir-train Plan 339 T1.5 / [Bench 470](../../riir-train/.benchmarks/470_plan339_ns_hardening_gate.md) (2026-08-17, RTX 4090, TF32 cuBLAS, exclusive) + the Plan 339 T5.1 handoff queued in this repo's SKILL.md mining snapshot
**Kind:** measured-GOAT distillation (the B49/B52 reopen path (c) — a measured fix in a sibling repo becomes a corpus rule; NOT file mining)

## The candidate set (from the Plan 339 T5.1 handoff, with measured verdicts)

The handoff listed 5 kernel-rule candidates from Research 417 §2.3, pre-verdicted so the loop
wouldn't mine refuted ones:

| candidate | verdict | disposition |
|---|---|---|
| `symmetric-half-gemm-mirror-epilogue` | REFUTED as a gain (Bench 469: syrk gives up 2.2×/FLOP for the 2× FLOP cut; 1.01–1.09× slower at every shape) | NOT distilled — negative-annotation material only; the corpus doesn't have a negative-annotation tier for kernels, skipped |
| `gram-domain-iteration` | REFUTED numerically (Bench 470: `P = X·P₀·X` needs 4 digits of cancellation in TF32's 3; broke the control 16/26 → 6/26) | NOT a standalone rule — encoded INSIDE the new rule as a measured refuted variant (the strongest placement: the refutation IS decision content) |
| `restart-to-kill-spurious-eigenvalues` | **measured WORKS with constraints** (Bench 470: ε=1e-3 + 2 full passes → 26/26 + 0.9587 > 0.9454 production) | **DISTILLED — this batch.** The handoff's "strongest candidate"; the constraint set (`ε ≤ γ−1`, full coefficient schedule, data-domain Gram re-form) is the rule |
| `error-feedback-subset-selection` | shipped (Bench 472) but optimizer-loop algorithm content | OUT of kernel_opt scope (recorded in the handoff) |
| `fp16-mantissa-over-bf16-range` | untested by Plan 339 | Still OPEN — no in-repo evidence either way; stays in the handoff list |

## The rule

`regularizer-above-noise-floor-coupled-to-damping-cap` (**B: Compute**, corpus 348 → 349, B 51 → 52):

A reduced-precision matrix-polynomial iteration (Newton–Schulz / inverse-PSD-square-root over a
TF32 Gram) run on rank-deficient input develops **spurious eigenvalues in the Gram's null space,
set by the SIGNED per-GEMM relative noise of the format** (~1e-3 in TF32) — not by the diagonal
ε regulariser, and not by ill-conditioning alone (cond~1e12 decay is clean; exact/near rank
deficiency with O(1) columns is the trigger).

Three coupled facts, none sufficient alone (the measured decision table, 26 adversarial cases):

| configuration | adversarial | production ‖O‖_F/√d_min @ 4096×2048 |
|---|---|---|
| shipped (ε=1e-5, 1 pass) | 16/26 | 0.9454 |
| restart only (ε=1e-5, 3 pass) | 14/26 (WORSE) | — |
| ε=1e-3 only | 26/26 | 0.3639 ✗ |
| **ε=1e-3 + 2 full passes** | **26/26** | **0.9587 ✓** |

1. ε must sit ABOVE the format's noise floor — ε=1e-5 is 100× below TF32 GEMM noise: "a
   regulariser quieter than the thing it regularises" cannot push noise-dominated null
   eigenvalues off the edge of the convergence basin.
2. ε is capped by the damping budget: after normalisation λ_max = 1+ε, and γ budgets exactly
   γ−1 of overshoot past the polynomial's basin [0,1] — **ε ≤ γ−1 must be asserted in code,
   not documented** (10× the budget diverges to σ_max ~1e24 on the well-conditioned CONTROL).
3. Raising ε alone shrinks singular values at high d_min (`(λ/(λ+ε))^{1/2}`); the repair is a
   second **FULL-coefficient-schedule** pass with the Gram **re-formed from the DATA iterate**
   (per pass: S = curᵀ·cur, normalise, whole schedule → R, cur = cur·R). After pass 1 the
   singular values sit near 1 so the next Gram ≈ I (rank-1 case λ_min/λ_max: −1.9e-1 → −1.1e-9).

Refuted variants encoded in the rule (both measured):
- **Gram-domain restart** (`P = X·P₀·X`): algebraically identical, no extra tall GEMMs — but X
  reaches ~1/√ε ≈ 316 while the null-direction product must cancel to ~3e-3: four digits of
  cancellation in a format carrying three. Broke the control 16/26 → 6/26.
- **Partial-schedule restart**: re-applies the aggressive a=7.42 prefix to an
  already-orthogonal spectrum → fixed point at σ ≈ 1.7 that more passes don't improve.

Guard corollary: `if t < 1e-20` is FALSE for NaN (every NaN comparison is false) — an
overflowed Gram sails through and poisons the weights permanently; `!t.is_finite() || t < t`
(a zero step skips, a NaN step is unrecoverable). Landed unconditionally in riir-train as part
of T1.5.

## Category placement

B (Compute) — the category doc explicitly hosts "precision upcast" and the numerics-robustness
family (`softmax-entropy-via-shift-identity` B42, `single-sweep-minmax-for-quant-scale` B44,
`saturating-nonlinearity-recovery-breaks-near-cap` B35, `fast-math-contraction-behind-feature-
flag` B29). N (DtypeAlignment) was considered and rejected: N is about per-dtype code
SPECIALIZATION (`FloatKind` intrinsics, alignment-gated views), not precision SEMANTICS.

## Dedup grep (pre-proposal)

`noise floor|noise-floor|regularis|regulariz|damping|newton.schulz|spurious` over
`src/domains/{kernel_opt,rust_perf}/` — zero corpus hits (the two "spurious" hits are the
Issue 693 wrong-math rules' prose: a wrong gradient factor, not iteration divergence from
format noise). Clean.

## GOAT

- **G1**: `batch54_pseudocode_parses` PASS (both shapes syn-parse; first compile clean).
- **G2**: kernel_heal lib **735/0/1** (the 1 ignored is pre-existing), default lib **549/0**.
- **G3**: clippy 0 in BOTH feature states (default + kernel_heal); whole-crate per the B47
  standing check — zero findings.
- **Retrieval floors**: HELD — 13 retrieval tests green; sweep clippy top-1 89.6% (floor
  87.5%), perf 96.2% (50/52), op-aware blend 94.6/97.7 (floors 92.7→ re-pinned values). No
  batch decay.
- No rust_perf Pareto bench (kernel batch; the perf evidence is the external Bench 470
  decision table — the rule distills a correctness/robustness result, not a timing win; the
  hardened config costs 2× the NS segment and stays opt-in in riir-train via
  `set_ns_eps`/`set_ns_restart_passes` runtime knobs).

## Generator / landing notes

- One-shot Python generator (write-run-delete), anchor-asserted (`sub1` count==1 on every
  replace) — first compile clean, zero pollution. The header-block edit used
  start/end-marker span replacement instead of exact-block reproduction (the B49/B51/B52
  interleave made exact reproduction fragile) — recommended standing method for that header.
- **The B49/B51/B52 header interleave is FIXED in the same commit**: the B49 stub line
  (`- **Batch 49 (Research 055):** 1 rule, corpus 343 → 344 (A 80) —` with content orphaned
  after the B52 bullet) is rejoined; B51/B52 bullets reproduced verbatim; chronology now
  49 → 51 → 52 → 54 → Plan 003.
- Count pins updated at all sites: tests.rs entries.len() 348→349 (+ the stale "Sum must equal
  344" comment → 349), per-category expected (B 51→52), exhaustive-test comments, router.rs
  4 sites, mod.rs 2 sites + B-category comment. The stale `current 348` → `current 349`.
- `.research/.highwater` 60 → 61 (write-back; 060 existed at scan time — the re-scan rule
  held).

> **PASS-Redirects (synthesis):** Lin et al. [arXiv:2402.03496 "Can We Remove the Square-Root in Adaptive Gradient Methods? A Second-Order Perspective"] — root-free (p=1) adaptive methods; the eps-vs-format-noise-floor coupling distilled here (Batch 54) is the in-house twin of its claim 3 (roots are the instability in reduced precision); training-track verdict + Plan filed at `../riir-train/.research/441_Root_Free_Adaptive_Gradient_Methods.md`, modelless extraction rows all discarded with cited coverage (§5 there).

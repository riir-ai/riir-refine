# Research 067 — Batch 60: pre-gate-recovery-by-division-cancels-algebraically (Issue 460 measured-GOAT distillation)

**Status:** RECORD (batch landed 2026-08-18; corpus 363 → 364 kernel rules, N 20 → 21; Plan [006](../.plans/006_batch60_pre_gate_recovery_division_rule.md))

## Source

**riir-train Issue 460** (`kimi_k3_backward_nan_at_random_init`, filed +
RESOLVED 2026-08-18) — the SIXTH measured-GOAT distillation after the
riir-gpu corpus went file-mining-complete (reopen path (c): a measured fix
becomes a rule the same day; lineage B49 → B52 → B54 → B55 → B56 → B57 →
**B60**).

- **The defect** (katgpt-rs `e2853fbf`): the MLA backward reconstructed the
  pre-gate attention output by *inverting* the gate — `attn_out_pre =
  attn_out_gated / gate` with `gate = sigmoid(W_g·h)`. Sigmoid underflows to
  a HARD ZERO in f32 once the pre-activation drops below ~−88; `gated` is
  `pre·g` and therefore 0 there too → the reconstruction is **0/0 = NaN**.
- **Why it stayed hidden**: the forward and loss stay perfectly normal (a
  saturated gate is finite; the gated output is simply 0), the NaN existed
  only in gradients, and it surfaced FAR from its cause as an opaque
  `NsInvSqrtFailed` in the optimizer. Measured: 4 of 10 real sequences at
  `random_train_init(seed 42)` (0.40B shape), 4 of 5 seeds affected.
- **The localization proof** (new example `issue460_nan_localize`): the
  FIRST corrupt tensor is `mla.w_g` — the ONLY consumer of the reconstructed
  value; every other MLA gradient (`w_o, w_uv, w_uk, w_uq, w_qr, w_kr,
  w_dq, w_dkv`, both norms) reads `attn_out_gated` and stays clean. Perfect
  correlation over 10 sequences: min-gate exactly 0.0 on the two failing
  steps, 1e-26..4e-35 on the eight clean ones.
- **The fix — algebraic, not an epsilon**: `gated == pre·g` by construction,
  so `pre·g·(1−g) == gated·(1−g)` — the gate factor CANCELS in the gate
  weight-grad, the singularity disappears with no clamp/no bias/one fewer
  divide per element, and it is **bit-identical wherever the old form was
  finite** (the 4 previously-finite grad norms unchanged to 4 decimals). The
  public `saved.attn_out` reconstruction is guarded to 0.0 where g == 0 (the
  pre-gate value is information-theoretically unrecoverable there — 0/0
  carries no information), and the checkpoint path SAVES the true value
  instead of dividing it back out. Regression
  `issue460_saturated_output_gate_keeps_gradients_finite` forces saturation
  (both signs) + asserts finiteness + asserts its own premise — verified
  load-bearing (FAILS pre-fix, 64/512 embed grads non-finite). Bench 475
  repro: **6/10 → 10/10** usable backward passes (closes Plan 339 T4.1
  blocker 4).

## The rule

`pre-gate-recovery-by-division-cancels-algebraically` (category **N**, the
backward-numerics home beside B34 `exact-op-parity`, B35
`saturating-nonlinearity-recovery-breaks-near-cap`, B40
`rmsnorm-backward-gamma-scope`):

> A backward pass that needs the pre-gate activation of a multiplicative
> gate must never reconstruct it by dividing the saved gated output by the
> gate — sigmoid underflows to a HARD ZERO in f32 and the reconstruction is
> 0/0 = NaN, doubly hidden (finite forward; the NaN surfaces as an opaque
> optimizer failure steps later). The division is unnecessary: every
> gradient consumer multiplies the pre-gate value by gate-derived factors,
> so the g cancels — bit-identical wherever the old form was finite. Where
> g == 0 the value is information-theoretically unrecoverable: guard public
> reconstructions to 0.0 and SAVE the true value at checkpoint boundaries.
> Detection tell: the NaN localizes to the single weight matrix consuming
> the reconstruction while every post-gate-reading gradient stays clean.

**Distinctness** (the dedup gate, both vocabularies grepped):
- vs **B35** — derivative recovered from cached output rounds to exactly **0**
  near cap (dead gradient), fix = **recompute the raw argument**; this rule is
  a **value** reconstruction by **division** producing **NaN**, fix =
  **algebraic elimination** (no recompute). Different failure axis, operation,
  and fix.
- vs **B32b `sigmoid-deriv-recovered-from-output`** — the recovery
  *optimization* family; this rule is the discipline that keeps such recovery
  singular-free when a division is involved.
- vs **B32a `sign-branch-stable-sigmoid`** — forward sigmoid computation
  hygiene; this is backward reconstruction discipline.

## GOAT

- **G1**: syn-parse gate (batch60_pseudocode_parses) + awk orphan-rule_id
  scan clean + first compile clean.
- **G2/G3**: kernel_heal lib **778/0/1** (baseline 775 + 3 new gates),
  default lib **577/0/0**, clippy **0** in both feature states.
- **Retrieval**: kernel self-variants **100/100/100**, blended
  **94.7/97.7/97.7** of 131 — floors HELD (94.6/97.7/97.7 of 130; +1
  denominator = the sibling B59 perf span; top-1 improved 94.6 → 94.7).

## Process notes

- **Plan-first discipline adopted** (owner call, same day): this batch
  executed before its plan existed; Plan 006 was created retroactively +
  marked as such, and BOTH skills (`distill` + `riir-clippy` Protocol A)
  gained the plan step in the same commit so every future batch is
  plan-first. This is the canonical "don't do this again" record.
- Generator: all anchors unique on the FIRST dry run (fifth consecutive).
  No doc_markdown fixes needed post-write (pre-emptive backticks in the
  `//!` header block).
- The separate w_g all-ones-init finding (training-quality, NOT the NaN) was
  deliberately NOT folded into the fix — filed in the source issue for its
  own before/after measurement; not corpus material (an initializer choice,
  not a kernel pattern).

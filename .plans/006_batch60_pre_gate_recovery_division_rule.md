# Plan 006 — Batch 60: pre-gate-recovery-by-division-cancels-algebraically (Issue 460 measured-GOAT distillation)

**Created:** 2026-08-18 — **retroactively, at owner request, AFTER the
implementation was complete + validated.** The batch ran through the idle loop
before the plan-first discipline was adopted; this plan records the executed
task set with evidence so the discipline starts HERE. Future batches create
their plan BEFORE implementation (the `distill` skill + the riir-clippy
Protocol A now carry the plan step — added in the same commit as this file).

**Source:** riir-train Issue 460 (`kimi_k3_backward_nan_at_random_init`,
RESOLVED 2026-08-18) — root-caused + fixed in katgpt-rs `e2853fbf`
(`katgpt-attn/src/mla_backward.rs`: the MLA backward reconstructed the
pre-gate attention output as `attn_out_gated / gate`; `gate = sigmoid(W_g·h)`
underflows to a HARD ZERO in f32 below ~−88 pre-activation → `0/0 = NaN`).
Bench 475 repro: 6/10 → 10/10 usable backward passes; all 6 finite-difference
gradient checks pass post-fix. Measured-GOAT reopen path (c): a measured fix
becomes a rule the same day.

## Tasks

- [x] T1 — Pre-flight: read the Issue 460 narrative + the `e2853fbf` diff
      (the guarded public reconstruction + the `d_gate_pre` algebraic
      cancellation); confirm recurring decision content (any gated backward
      that saves only the post-gate value — sigmoid/tanh/GLU gates).
- [x] T2 — Dedup grep BOTH vocabularies (`saturat|underflow|reconstruct|0/0|
      divide.*gate|invert`) against `src/domains/kernel_opt/`: nearest
      neighbors B35 `saturating-nonlinearity-recovery-breaks-near-cap`
      (derivative→exactly 0, fix = raw recompute), B32b
      `sigmoid-deriv-recovered-from-output` (the optimization family), B32a
      `sign-branch-stable-sigmoid` (forward hygiene) — all distinct on the
      failure axis (NaN vs 0), the operation (division vs identity recovery),
      and the fix (algebraic elimination vs recompute). Corpus-novel.
- [x] T3 — Author the rule `pre-gate-recovery-by-division-cancels-
      algebraically` (category N — the backward-numerics home beside B34/B35/
      B40): concept + ref/opt pseudocode (ORIGINAL minimal reproduction, no
      external source pasted); land in `entries_gpu_07.rs` via the one-shot
      Python generator (all anchors unique on the FIRST dry run).
- [x] T4 — Count pins, corpus 363→364 (N 20→21), every site: entries_gpu_07
      header, mod.rs (Batch-60 `//!` header block, exhaustive comment,
      KERNEL_CATEGORIES N row + N count comment), tests.rs (entry-count
      assert + per-batch comment, DtypeAlignment table row, both sum
      asserts, exhaustiveness comment), router.rs (4 mentions).
- [x] T5 — Standing gates: `batch60_rules_present` + `batch60_concepts_covered`
      (5 exact-case substrings) + `batch60_pseudocode_parses`.
- [x] T6 — GOAT G1+G2: awk orphan-`rule_id` scan clean; first compile clean;
      kernel_opt lib **778/0/1** (baseline 775 + 3 new gates; the +6 vs
      Batch 58's 769 = sibling commits between, verified 0-fail).
- [x] T7 — GOAT G3: default lib **577/0/0**; clippy **0 warnings/errors**
      in BOTH states (kernel_opt + default).
- [x] T8 — Retrieval-precision re-measure: kernel self-variants **100/100/100**,
      blended **94.7/97.7/97.7** of 131 — floors HELD (pinned 94.6/97.7/97.7 of
      130; +1 denominator = the sibling B59 perf span; top-1 improved
      94.6→94.7). 13/13 retrieval tests pass.
- [x] T9 — Record: `.research/067` note (Status: RECORD) + `.docs/11_domains/
      kernel_opt.md` (Total 364 + Batch 60 entry) + AGENTS.md Batch 60 entry +
      SKILL.md queue snapshot — same commit as the corpus change.
- [x] T10 — Commit + push `feat: batch 60 ...` on `develop`; update
      `.plans/.highwater` 5→6 + `.research/.highwater` 66→67.

## Deferrals

(none)

## Skill deltas landed with this plan (the "try again" ask)

- `distill` SKILL.md: "Recording the verdict" YES path now requires the plan
  artifact; Constraints gained the plan-first rule.
- `riir-clippy` SKILL.md Protocol A: step 0 — create the batch plan before
  implementation.

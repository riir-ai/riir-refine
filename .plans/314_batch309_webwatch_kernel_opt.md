# Plan 314 — Batch 309: the 10-30 staged web-watch intake (kernel_opt)

**Status:** IN PROGRESS — claim posted 2026-10-30 (shikuwa-4090-b309; expires 2026-11-01T03:1xZ)

## Scope

The 10-30 staged intake (zed-m3-i1117-collect/watch, snapshot rows P105-P110), converted
kernel_opt-only under the B293 cross-domain precedent: B300 (perfect-hash, to 10-30T09:34Z),
B301/plan-306 (to 10-30T13:41Z), B306/plan-310 (to 10-31T14:21Z), plan-311 hold the
rust_perf/sec heads; riir-infer issues 1156 + 1117 carry foreign live claims (the 1117 lane
touched its file minutes before this cycle's fetch). GPU idle (0%, no compute consumers) —
no contention for the gates.

## Candidate slate (T0 grades; honest drops + consumed-pre-existing recorded)

| # | Lead | Source | Grade | Disposition |
|---|---|---|---|---|
| A | #4596 `e1ef8a9` sdpa TK=4 key-loop unroll (prefetch-all-TK-loads-first, TK-strided main + scalar tail, TK sized from register pressure) + P110 #29298 `4d426d5` template-on-ncols1 trip-count repair (ONE atom, two instances) | mlx + llama.cpp | STRONG DISTINCT B | OWN rule (category B — compute/unroll) |
| B | #29869 `cc470b6` + #30065 `b0cb151` per-type rows_min few-row-MMA crossover table + band + FWHT precedence | llama.cpp | RIDER B | rider face on `small-batch-matvec-gemm-bridge-rung` (gpu_13) — SECOND instance of the B116 per-type crossover face; kinship gpu_119 (per-silicon axis) |
| C | #30553 MMQ Stream-K grid cap | llama.cpp | CONSUMED PRE-EXISTING | `stream-k-grid-capped-at-useful-work-quanta` (gpu_108, Batch 253 / Plan 257) owns the FULL 3-commit state incl. the nwork ceil refinement + fixup coupling + determinism caveat — nothing to add; recorded in the snapshot |
| D | P109 (a) Metal 32-bit offset widening, (b) fattn WAR fix, (c) spec-tree rumor | watch | WATCH-ONLY | no mining until surfaced in a scored artifact (staged row's own law) |
| E | P106 residue: RIIR_PREFILL_GDN_CHUNKED re-measure at the 27B league posture | league | WINDOW/OWNER-GATED | league re-pin owner-gated; benchmark stays window-gated per the staged row — NOT this batch |

## Tasks

- [x] T0 quote re-verification at the pins: the six PRs fetched as .patch into `.raw/b309/`
      (llama.cpp #29869/#30065/#30553/#28770/#29298, mlx #4596); every quote byte-verified
      against the diff text (20:0x-20:3xZ, pre-claim)
- [x] T1 dedup greps at 3+ vocabularies vs the corpus at origin HEAD `565100f4`: unroll
      family 136 hits — ZERO compile-time-trip-count owners (gpu_06 owns the array-footprint
      axis, gpu_01 owns config-baking, the B276 inject rider owns the dependent-K-walk
      scheduling hint — all cited as kinship, none owns the law); per-type crossover —
      gpu_13's B116 rider owns instance 1, P105 is instance 2; stream-k cap — B253 owns it
      (P108 consumed pre-existing)
- [ ] T2 authoring: entries_gpu_140.rs (1 own rule: pattern, bounded fix-space, category B,
      rationale, source quotes, distinct-from kinships, grep evidence) + 1 rider face (B on
      gpu_13, entries_gpu_13.rs)
- [ ] T3 wiring: mod.rs mod-decl + extend + B-slice count comment (B slice +1)
- [ ] T4 count pins: tests.rs count-pin sites (1022 -> 1023) + router expert-total + AGENTS.md
      + README + .docs/11_domains/kernel_opt.md
- [ ] T5 gates: G1 batch309 needle+parse; G2 kernel_opt lib + default lib; G3 clippy -D x3
      postures; retrieval floors both postures; freeze/score-bench NOT owed (zero
      clippy-corpus delta, P30); binary rebuild NOT owed (kernel_opt outside the release set)
- [ ] T6 records: Research note (next .research number); AGENTS.md; snapshot landing row +
      P106/P108 consumed-pre-existing + watch-only rows recorded; this file's tasks ticked
- [ ] T7 commit `feat: batch 309: ...` + push; `.raw/b309/` + the worktree removed at close

## Gates, criteria, risks

- Shared-tree hazard: this box's checkout (E:/git/riir-refine) is 73-behind with sibling WIP
  (10 dirty paths incl. a deleted manifest_gate.rs) — ALL work happens in the detached
  worktree `E:/git/wt-b309` at origin `565100f4`, gated under an isolated CARGO_TARGET_DIR
  (`/tmp/plan314-target`); the local checkout is untouched.
- The embedded-quote defect class (B299's twice-fired lesson): never replace an old prefix
  WITH its leading quote when appending rider notes to gpu_13's concept string.
- Fixtures carry no string literals (the B274 lesson); needles keyed to landed text AFTER the
  entries are written (the Batch-259 lesson).
- The claim row notes P106/P108 consumed-pre-existing so no later lane re-mines them.

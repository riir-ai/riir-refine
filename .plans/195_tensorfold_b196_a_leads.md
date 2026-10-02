# Plan 195 — Batch 196: the TensorFold staged pack's A− leads

**Status:** LANDED 2026-10-02 (4090 slot) — Batch 196 = the 4 A− leads + the B176 instance rider; quotes re-verified VERBATIM at the pin `c4646171139ee8a3c38103eaa1699dad226ec12b` (v0.6.0, **Apache-2.0, license flipped MIT→Apache-2.0 at 0.6.0**). The B/B−/C+ pool (15 rules + instances) stays staged for the next slot(s). Gates: batch196 parse/contracts/riders 5/5 · default lib 2424/0 · kernel_opt lib 2824/0 · rust_perf_bench 2455/0 · latent_retrieval lib 2598/0 · latent_suggest_goat G1 PASS · clippy `-D` ×3 postures green (two pre-existing box artifacts disclosed — Issue 143's G2 platform gap + the non-denyable build-script summary). Freeze + score-bench NOT owed (zero clippy-corpus delta, P30; both rules NotBenchable). Source standing intake: `.distill/001` 2026-10-01 ~06:5x row (staging) — quotes re-verified VERBATIM at the pin by THIS session's coordinator (4/4 Tier-1 sites pass; two line-drifts disclosed in-entry).

## Scope this slot — the 4 A− leads + 1 instance rider

- [x] R1 `affine-block-address-lets-pipeliner-fly` (A−, kernel_opt) — the stored-block address
      `(b // 4) * 32 * SBN + ((b % 4) * 16) // 2 * SBN` is a STAIRCASE in the loop step; the same
      address is `b * (8 * SBN)` — "an address the pipeliner follows" (nvfp4.py:312 VERBATIM ✓).
      Law: "an address the pipeliner cannot see as affine is not put in flight" (443ad4a, at-sha).
      Category: L-candidate (layout/addressability; adjudicate against the table at authoring).
      Distinct-from `hoist-bit-address-out-of-packed-row-sweep` (LICM hoists an INVARIANT; this
      makes an ADVANCING address affine so the pipeliner overlaps it).
- [x] R2 `register-width-topk-falls-back-to-tiled-radix-select` (A−, kernel_opt) — per-row selection
      holding `next_pow2(blocks)` scores in registers hits a SPILL cliff (`SELECT_REGS = 32768` —
      attention.py:13 VERBATIM ✓; dispatch at :357, staged said :353 — minor line drift, disclosed);
      fix = budget-keyed dispatch with an OUTPUT-IDENTICAL fallback (`_select` while `width <=
      SELECT_REGS`, `_select_tiles` past — same lists, ties by lowest block id). Distinct-from
      `wide-verify-warp-specialized-scores-in-registers-pv` (that CHOOSES the registers posture by
      width; this handles the SPILL side with an identical-output fallback).
- [x] R3 `grammar-window-draft-prune-path-local-mask` (A−, rust_perf/serving) — before the verify
      forward the grammar matcher walks the draft tree DFS (`accept_token` down, `rollback` up —
      grammar.py:290-322 VERBATIM ✓ incl. "the parent's masked row could never choose it" :305),
      drops drafts NO accepted path can hold, fills each kept row's allowed-token bitmask from its
      path, −inf's the rest. Exactness law (98fb8f9 VERBATIM ✓): "A row's logits do not depend on
      the other rows, and its mask depends only on its path, so a drafted row and the serial step
      with the same path choose the same token, greedy or keyed." Bounded refusal surface:
      xgrammar missing / schema won't compile / TP rank-1 structured-off with packed-constraint
      sharing ("both ranks then walk and mask the same rows", grammar.py:158). NOVEL — zero
      grammar/constrained-decoding content in corpus.
- [x] R4 `startup-estimate-derived-from-allocation-shapes` (A−, rust_perf/serving) — the capacity
      plan's EXL3 term was a fixed 1.125 GiB/rank guess while the engine allocated an
      `exl3_mm.Scratch` per buffer set + 1 GiB prompt split-K partials nothing counted (~2.5 GiB
      under-count, admission lied); fix derives every term from the allocation shapes themselves
      (`exl3_expert_scratch` geometry.py:222 VERBATIM ✓, consumed at :255-256) and proves it with a
      FAKE-ALLOCATOR RECONCILIATION: at real shapes, allocated ≤ estimate (test_cuda_geometry.py:259).
      Absorbs: NVFP4-widening estimator blindness 4.00×→1.01× (3655360, message-only) + the
      `--checkpoint-slots` knob joins the estimate per kept state and is refused pre-download when
      unservable (f151c6f serve_options.py:36). `cache-slot-knob-budget-counted-refused-pre-download`
      folds here as a sub-arm.
- [x] R5 RIDER (instance-read, kernel_opt) — `fuse-admission-matches-kernel-group-size` →
      INSTANCE of B176 `dtype-native-source-read-with-shared-admission-predicate`: lane_fuse's
      admission hardcoded `group_size not in (32, 64)` while lane_matmul/tile_weight/pack_scales
      already accepted g32 → g32 checkpoints silently lost their fused projections
      (lane_fuse.py:87/90; "one kernel a stack: members of mixed widths or group sizes stay
      separate calls"). The admission-must-match-the-kernel's-actual-contract law; g32 = the
      instance. Rider text on the B176 entry at authoring; not a new rule.

## Execution

- [x] T1 2 authoring subagents over DISJOINT write sets: A = kernel_opt (R1+R2 →
      `src/domains/kernel_opt/corpus/entries_gpu_64.rs` — categories adjudicated L/D by the table's own definitions, reasons in-rule + B176
      rider R5); B = rust_perf (R3+R4 → `src/domains/rust_perf/corpus/entries_11.rs` — NotBenchable corpus-only, the B182/B186 precedent, no proposers).
      Every quote VERBATIM at the pin; provenance labels per number
      (in-source-comment / commit-message-only / absent).
- [x] T2 coordinator spot-verification of every new entry's Tier-1 quote against the pin + pin walk
      (router total 726→728; rust_perf 165→167; per-category comments; tests.rs count pins; 4 stale pre-batch count labels repaired with disclosure).
- [x] T3 Gates: batch196 parse gate (fixtures per entry) · contracts gate · kernel slice (kernel_opt-featured lib) · lib
      default · clippy -D ×3 postures (default / no-default / all-features) · retrieval floors (latent_retrieval lib 2598/0 + G1 PASS;
      G2's platform gap at HEAD filed as Issue 143 — pre-existing, disclosed). Freeze regen NOT owed (zero clippy-corpus delta, P30).
- [x] T4 Commit `feat(batch196)` + push; `.distill/001` queue row (staged pool remainder disclosed);
      `.plans/.highwater` 194→195 in the same commit; `.issues/143` (the G2 platform gap) filed in the same unit.

## Validation

Corpus tests at both kernels' parse gates + full lib + retrieval floors; every new rule carries its
decision content in the entry (annotation class, NotBenchable — no bench spec owed); the B/B−/C+
staged pool remains untouched for the following slots.

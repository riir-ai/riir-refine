# Research 131 — riir-ai M3 campaign measured-GOAT distill (prefill GEMM + decode attention)

**Status:** RECORD (Batch 108 landed, Plan 078 executed; in-source — no `.raw` clone to remove, the sources live in the sibling repo at the pin)

Date: 2026-09-05 · Box: M3 (CPU-only) · Plan: `../.plans/078_riir_ai_m3_campaign_measured_distill.md`
Pin: riir-ai @ `59a172623bb3748ed49061e2202b787b4e4e3f08` (origin/develop at plan-allocation time, committed state only)

## Why this batch exists

The last measured-GOAT distillation from riir-ai's own kernels was Batch 63 (2026-08-20).
Everything since — the entire M3 prefill/decode campaign (Issues 761/764/767/768/771/773/774/779/790/800/804/805/831/843,
Benches 761-858) — landed UN-MINED while the program pivoted external (B64-B107). This batch mines the
campaign's four strongest verdicts; **every rule carries a measured number from our own silicon, cited
from Benches 774/790/804/831 as records, never re-measured** (the plan's explicit non-goal).

## Verification

Two-lane subagent verification (byte-accurate quotes + dedup adjudication) completed BEFORE Plan 078 was
authored. Coordinator spot-check at landing: items A-D (attention.rs envelope/verify_kf4_for/V-guard, smem
GEMM private-region/signed-zero/staged-epilogue/axis-swap, 32x32 bit-identity + untried-axis, f16-KV toggle
verdict) all VERIFIED. Two honest offsets recorded:

1. The Bench-756 −3.4% cross-substrate line sits at `qwen_attention_cubecl.rs:1319` (rung-1 rationale),
   NOT inside the 1348-1360 toggle block.
2. The ~1.8x/`valid_n`/geometry-scan NUMBERS live in `qwen_attention_cubecl.rs:940/:1017/:1090-1092`;
   the probe file (`tests/probe_831_attn_pair_time.rs`) carries the test + the lane-split TIE record at :16.

## Landed (4 rules, corpus 490 → 494: A +1, E +2, G +1)

| Rule | Cat | Grade | Core |
|---|---|---|---|
| `smem-envelope-gate-kernel-variant` | G | A− | Issue-779 derived capacity predicate `32*(head_dim+4) <= head_dim*33` admitting the k4 smem variant at ALL 8 rows-family launchers via one `verify_kf4_for`; below it the staging overflows into p_smem and phase B races row-0 score writes barrier-free (hd-64 corrupts nondeterministically; hd-256 safe by slack 8320 ≤ 8448); fallback bit-identical (load width only); deterministic in head_dim → capture-safe; + dead-lane V-read guard |
| `multi-simdgroup-shared-staged-tile` | A | A | Issue-771-T1 smem 128×64 GEMM: one 256-thread workgroup = 8 simdgroups, W+X staged ONCE per 32-K window, per-sg PRIVATE 8×64 result regions ("one shared tile would race across the 8 sgs' stores"), 2 barriers/K window, 26.6/32 KB budget, m≥128 && p≥64, bit-identical incl. signed-zero tail-skip |
| `independent-accumulator-chains-hide-load-latency-at-capped-occupancy` | E | A− | Issue-831 4-chain unroll: 4 loads in flight/thread = the ONE lever that moved multi-wave lengths (~1.8× @4096, 466→263 µs/call; FLAT @2048 — both regimes are the content); `valid_n` loop bound; tree finish reassociates → band-gated; regime boundary = B44's lane-split TIE (median 1.01-1.06) |
| `byte-cut-loses-when-plateau-is-issue-bound` | E | B+ (refutation-type) | Issue-831 packed-f16-KV: built + G0-exhaustive-65536-bit-exact + G1 band pass, measured 1.44-1.60× SLOWER at every length — the plateau is issue/occupancy-bound (~114 vs ~400 GB/s, supply capped); Bench-756 4090 −3.4% corroboration; regime test BEFORE any byte-cut |

## Riders (5)

- **B01 `row-tile-amortize-input-load`** += CMMA STAGED-GEMM IN-STACK INSTANCE (Issue 768 32×32: input tile
  across 4 M-subtiles; 42-63% input-load share; regime m≥64 with the 0.773×-at-m=48 counter-example;
  bit-identity S1:938-940; untried-axis S1:563-565).
- **B06 `row-dot-split-inner-axis-block-reduce`** += CAPPED-OCCUPANCY REGIME COMPLEMENT (the lane-split TIE
  record; axis splitting buys nothing at a workgroup-supply cap).
- **B57 `smem-slot-role-reversal-needs-barrier`** += EXPLOITATION HALF (Issue 843 T3 staged epilogue: dead W
  staging tile reused as the 8 per-sg epilogue regions, `tile_w[sg*512..+512)`, 32 barriers → 4,
  overwrite-only-after-half-1-drained; +2.5-2.8% @16K / +4.5% @8K, bit-identical, default ON).
- **B79 `first-order-affine-recurrence-associative-scan`** += BREADTH CONFIRMATION (arXiv:2608.17641
  rl-triton — GAE/V-Trace/Retrace(λ)/TD(λ)/eligibility traces are the SAME first-order recurrence,
  O(log T) scan, 1.6-5.7× over torch-compile in the many-env regime; riir-train GRPO/PPO the potential
  consumers). The walk-#6 land-with-next-adjacent-batch queue item DISCHARGED.
- **B82 `wy-transform-tree-verify-state-free`** += EXTERNAL CONFIRMATION (arXiv:2608.01651 Bole — token-level
  factors = the pseudo-value matrix, reconstruct-only-sampled, 82-99× transient-state + 3.4-7.7×
  tree-verify, SGLang-integrated; reconfirms the memory-pressure conditionality — our Issue-721
  closed-negative was COMPUTE-side). The second walk-#6 adjacent-batch queue item DISCHARGED.

## Recorded-not-landed (batch record only)

- Co-resident-wave dispatch-order axis swap REFUTED (−6..−7.5% everywhere; the W-slab-sharing order is
  correct; S2:40-42) — adjacent to B9 swap-large-axis; rider if a second instance appears.
- The 4-arm dispatch ladder (8×8 / 8×32 / 32×32 / smem, hand-measured per arm) — in-stack instance of B104
  `swept-shape-bucket-launch-tables` ethos, no new mechanism.
- Empty-partial reduction identity (S4:975-988) — B35 instance at partial-buffer grain.

## GOAT + gates

- batch108_rules_present / batch108_concepts_covered / batch108_pseudocode_parses — 3/3 FIRST-SHOT
  (cfg(not(kernel_verify)) per the B70 convention).
- kernel_opt lib **1349/0/1i** (B107 baseline 1346 + 3 gates); default lib **1078/0** unchanged;
  kernel_opt+latent **1457/0/2i** (1454 + 3) with retrieval floors HELD (kernel self-variants 100/100/100,
  zero re-pins).
- clippy −D warnings 0: default / kernel_opt / kernel_opt+latent lib + all-targets kernel_opt.
  7 doc_markdown backtick fixes in the new `//!` header — the B37 lesson, 12th strike.
- Count pins walked: tests.rs 490→494 total + message + batch-list + current-494 comment + expected-array
  (A 108→109, E 35→37, G 26→27) + sums ×2 + grouped + the stale 464 cover-all comment repaired;
  router.rs 490→494; mod.rs doc line + mod/extend wiring + 3 category comments + 3 member-list tails.

## Lessons

1. **The B18 full-concept pin moved with the rider** (the B22 precedent): `batch18_rowtile_factoraffine_pinlane_concepts_covered`
   pins `row-tile-amortize-input-load`'s FULL concept string and tripped the moment the rider appended —
   the gate doing its job. Pin updated in the same commit.
2. **In-source mining needs no clone but DOES need the pin discipline**: the sibling worktree is dirty
   (sibling waves), so every quote came from `git show <pin>:<path>` — the working tree was never read.
3. Plan 078 was drafted when the corpus was 466 (pre-B106/B107); its stale numbers (entries_gpu_18,
   corpus 466→470, batch105_* gate names, highwater 74→75) were corrected to current reality at landing
   (entries_gpu_21, 490→494, batch108_*, highwater already 78) — the plan text carries the correction note.

Full task narrative: Plan 078 (checkboxes) + AGENTS.md Batch 108 entry + `entries_gpu_21.rs` header.

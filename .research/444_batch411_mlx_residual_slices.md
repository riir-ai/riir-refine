# Research 444 — Batch 411: the four mlx residual slices (Plan 410)

**Status:** RECORD — landed 2026-11-24, session zed-amo-i411@amo.

## Source

ml-explore/mlx @ submodule pin `c69303b0` (MIT) — the mlx-serve tip `2201ec9`
pin, the SAME pin as Batch 410 / Research 443. These four rules are Batch
410's own recorded staged leads ("Staged for the next batch", Research 443
§staged), converted per plan 410. Clone re-created at the pin this session
(`/tmp/i411-mlx`, removed at close); every quote re-verified at the clone by
the coordinator + two reader subagents.

## The four rules (kernel_opt 1121 → 1125)

1. **`per-row-bounds-bitmask-hoisted-loader`** (D) — the small-filter input
   loader's per-row bounds bitmask: `mask_h[i] |= (in_bounds << kh)` built
   ONCE in the constructor, the hot per-tap load pays two bit-test ANDs
   (loader_channel_l.h:268-292 + :296-318). The input-side pole of gpu_166's
   `whole-tile-inbounds-hoisted-loader-fast-path` (clean compose: the tile
   predicate covers all-interior tiles, the mask covers mixed rows; the same
   tile can carry both). Transcription traps recorded in-rule: mask_t is a
   16-bit `short` (wS ≤ 15 per axis — why large filters keep the
   re-evaluating twin at :112-122), the batch check rides the FIRST axis's
   mask only, a 3D twin carries three masks (:789-797).
2. **`dilation-base-table-tap-odometer`** (D) — the congruence-subsequence
   precompute: taps are {base + k·f_wgt_jump}, f_wgt_jump =
   lcm(idil,kdil)/kdil, the phase = position mod f_out_jump =
   lcm(idil,str)/str; the host solves {base, size} per phase (conv.cpp
   :511-547), the device iterators start at the base and WRAP TO THE BASE
   (loader_general.h :79-80/:187-208/:277-278), and the table sizes the K
   loop (steel_conv_general.h:124-125) — tail taps never visited. The
   negative space is the proof: zero `% idil` sites in device code. The
   scalar half (f_wgt_jump/f_out_jump as uniforms) is gpu_04
   derived-scalar's instance — cited, never re-owned; the table + odometer +
   trip count is the new axis.
3. **`reduce-simd-width-fallback-launch-cap-pairing`** (N) — the width-keyed
   execution split (ops.h:8-20: sizeof < 8 → hardware `simd_reduce_impl`,
   == 8 → 5-round shuffle-down fallback) PAIRED with the host clamp
   (reduce.cpp:493-496: itemsize == 8 → threadgroup min 512). Per-thread
   shuffle rounds are constant (20/40); the AGGREGATE scales with the
   simdgroup count (cross-group phase repeats the slow reduce + barrier).
   Bites at row_size ≥ 2049; exactly ONE clamp site in the backend (looped
   :547 + all-reduce pass-1 :385-388 size without it — the honest residue).
   Adjudications: vs gpu_166's logical-reduce-width-bitcast (read-type law —
   compose in the same call chain), vs gpu_86's byte-tier face (bytes vs
   width key), vs gpu_127 physical-warp-size (the agreement-law's dtype-axis
   instance).
4. **`strided-reduce-shape-ratio-assumption-guarded-tier-ladder`** (D) — the
   four-tier ordered dispatch (reduce.cpp:959-976): small <32 / longcolumn
   stride<32 && R≥1024 / 2pass >256 && out/32<1024 / looped — first
   satisfied guard wins, each guard is exactly the assumption its tier
   makes (longcolumn's threadgroup_x IS the stride, :684). Widening rider:
   longcolumn outer_blocks 32→128 at total ≥ 32768 (:661-665, ×4 not ×2);
   2pass fixed at 32 (:845/:860) — longcolumn-only, pinned as the asymmetry.
   Adjudications: gpu_117 splitk-tier-ladder (depth within one kernel vs
   topology selection), gpu_61 work-size-gate (one binary gate vs a 4-tier
   3-axis ladder), gpu_04 buffer-set (API/lifetime, contrast only).

## The landing record

- **Fixtures `b411_fixtures` 16/16** standalone-compiled AND executed BEFORE
  entry at debug AND -O, zero warnings. The gate caught THREE fixture
  defects: (a) the masked path's missing weight multiply (the probe showed
  masked[0][0] = the raw-input sum −8 vs the weighted 0); (b) the S3 cost
  model's double-scaled narrow weight (adds already ×4; the narrow cross
  weight must be 1, not 4); (c) the S1 tamper expectation's wrong
  surviving-row value (clearing kh=1 from out(0,1) removes the −7 kh=1 row,
  leaving the +2 kh=2 row — not 0).
- **Reader subagents**: two, conv-side (S1+S2) and reduce-side (S3+S4); the
  coordinator re-verified every quote at the clone. Reader corrections
  carried: mask_t is `short`; the n-check asymmetry; the clamp bites only at
  row ≥ 2049 (fixture uses 4096 for the bite, 2048 as the boundary no-op
  pin); the widening is ×4 and longcolumn-only; the per-thread-rounds
  constant / aggregate-scaling refinement; the trip-count clause is
  load-bearing decision content.
- **Gates**: default lib 3981/0; kernel+latent 4128/0; retrieval floors
  kernel-variants 30/0 HELD; clippy -D ×3 postures 0 (16 mechanicals healed
  via `cargo clippy --fix` — the stale-binary fallback, disclosed at
  e4b2dd31); fmt PASSED; numbering PASSED; pins_manifest agrees (1125);
  count web one-landing (router 1125 + the Plan-003 category table
  D274/N105 + health_views kernel 1125/total 1795 + README + AGENTS +
  kernel_opt.md + the re-pin trail + the census count 168 files).
- **Score-bench/freeze NOT owed** (zero clippy_lints delta — corpus-only
  kernel_opt suggestion-only). **Release-set binary NOT owed** (the standing
  disclosure; the rebuild folds into the next maintained-binary lane).
- **The router annotation floor RE-PINNED** (the Issue-020 contract): the
  landing run measured 102/195 (52.3%) vs the Batch-369 pin 100/188 (53.2%)
  — RED at landing. The pre-batch detached worktree at the B410 landing
  (`../riir-refine-b411pre` @ ccbcb76fb, isolated tmp target) measured
  108/195 (55.4%) — the batch's four rules cost SIX correct annotations
  (−3.1pp), the B223 mechanism amplified: the
  bitmask/odometer/launch-cap/tier-ladder vocabulary is token-dense generic
  kernel dispatch vocabulary that attracts clean spans to the kernel
  centroid. Floor = measured − 5 = 97/195 per the Plan-030 convention,
  recorded in the arm. Annotation only; gating stays killed (85% bar,
  Bench 025). Kernel self-variants floor HELD 30/30 (100%).

## Honest caveats

- ONE more slice of the vendored tree at "its own scale" — the excluded
  surfaces of plan 409 stay unread (quantized/steel attention/SDPA,
  fft/radix, hadamard, scan/sort, the CPU backend).
- The upstream-past-pin delta (64 commits) stays UNSWEPT — the watch surface
  (the perf-rematch weekly window owns the schedule; the next window diffs
  against `c69303b0`).
- The router annotation floor is an ANNOTATION quality axis; the 8-byte
  clamp's honest residue (looped + all-reduce unclamped) is recorded, never
  a defect claim.
- Nothing invented — every quote read at the clone at pin `c69303b0` this
  session. Session: zed-amo-i411@amo, epoch 1795491300.

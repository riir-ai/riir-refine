# Research 175 — Batch 149: the sm_89 spill-law / nodecode-probe STANDALONE pack (the dual-allocation yield's re-issue)

**Status:** RECORD (batch landed 2026-09-16; Plan 135 / Research 175; corpus 616 → 619)

## The dual-allocation and the yield (the Issue-665 discipline, exercised)

Two idle sessions independently consumed the SAME staged intake lead (the
Bakhshaliyev 4090 FP8 series, `.distill/001`'s open `- [ ]` row) in the same
window. This session authored a 5-rule + 7-rider batch (local `a970a3e8`,
gates green, preserved on branch `b148-yielded`); the sibling landed theirs
on origin/develop first (`8f8b7dd6` — 2 rules + 5 riders, 614→616). The
rebase conflict surfaced an add/add on `entries_gpu_43.rs` — the duplicate
feature-key shape. Yield executed: `git rebase --abort` → `git branch
b148-yielded a970a3e8` → `git reset --hard origin/develop`. The sibling's
Plan 133 / Research 174 / batch 148 are canonical; never commit through a
duplicate.

## The delta analysis (method: read THEIR rules' full concepts, then classify each of mine)

| my yielded item | their coverage | verdict |
|---|---|---|
| `occupancy-before-async-pipeline-buy-warps-first` | `occupancy-before-async-consumer-ada` (E) — the route-split integrator, RICHER on the external side (launch_bounds −20%, v7 register double-buffer 438) | DUPLICATE — dropped |
| `mbarrier-polling-loses-to-hardware-barrier` | `hw-barrier-beats-mbarrier-polling` (B) — with the producer/consumer split + barrier-count composition | DUPLICATE — dropped |
| `spill-fragility-law-accumulator-regcap-cliff` | cited BY NAME as register-route evidence, not shipped as a rule | KEPT — standalone law (serves B895's fusion context, occupancy irrelevant) |
| `nodecode-arm-bounds-overhead-elimination` | the 64.6% ceiling + 7.3% share consumed as evidence | KEPT — the probe METHODOLOGY (instrument rule) |
| `tile-shrink-occupancy-pays-feed-economics` | the v12 cell carried as tile-route refutation | KEPT (adjacent-delta) — the feed-scaling law + reopen triggers, serving tile decisions that are not occupancy buys |
| riders: cpasync / xor-swizzle / ldmatrix / stream-k | landed by them verbatim-equivalent | DUPLICATE — dropped |
| rider: tma-single-issuer (consumer absence) | their mma-fragment rider carries the FULL probe (error 715, error 700, cluster-only bulk, driver-vs-runtime attr-95 split) | DUPLICATE — dropped |
| rider: fast-accumulate (FP8 2× + QMMA + a1/a2) | their mma-fragment rider carries FP8 642.51/329.33 + the swap + the software-lock + cuobjdump discipline | DUPLICATE — dropped |
| rider: setmaxnreg (sm_89 counterpart) | their occupancy rule mentions no-setmaxnreg INLINE; the setmaxnreg RULE itself carries no boundary | KEPT — the boundary belongs ON the sm_90 rule where its reader needs it |

Net: **3 rules + 1 rider**, corpus 616 → 619 (E 55→57, D 134→135).

## Complementary verification corrections (their Research 174 found 3; two more here)

Both sessions independently re-fetched both articles (free-to-read) and
verified every claim. Corrections that LANDED in their Research 174: the
author-claim attribution, the INT8-s32 sub-claim not-in-source (dropped),
the stock-clock ladder scoping. Corrections this session found that neither
batch may silently drop:

1. **(a) qualifier**: the "at 24 warps/SM" attribution on the cp.async
   cells is INFERENCE (the 546 winner runs 24 warps/SM; the source never
   says the cp.async arms were tested at that occupancy). The Batch-148
   rule's phrasing ("cp.async double buffering measured WORSE ... against
   the 546 winner") avoids the attribution — acceptable; record kept here.
2. **(g) derived percentages**: the 56%/76%/82.7%/78-82% utilization curve
   is OUR derivation (÷1.077 from the OC table), and the article's own
   "89% of peak at 3045 MHz" headline mixes stock/OC clocks (587/660 =
   88.9% vs the stock peak; 74.3% vs the stated OC peak; the ×1.20-clock
   vs ×1.075-TFLOPS scaling contradiction is in-source). Never mine the
   headline; derive percentages with the clock split stated.

## The three rules (decision content, at a glance)

1. `spill-fragility-law-accumulator-regcap-cliff` (E): a reg cap that spills
   the accumulator set is a CLIFF (~0.5×), not a slope — ×4 measured
   (104 B→0.550×, 192→0.328×, 208→0.335×, 56 B→0.516×, 104→0.340× with the
   nodecode twin spilling MORE); the envelope arithmetic (64 acc + ~64
   working set = the whole 65,536 file at 512×128) predicts it pre-compile;
   two measured escape hatches (the 255-reg launch-bounds landing
   spill-free at 8 warps = parity; the tile-shrink, priced by the
   companion); reject on `local_size_bytes() > 0` BEFORE benchmarking.
2. `nodecode-arm-bounds-overhead-elimination` (D): the stripped-arm probe —
   full-elimination ceiling bounds every dependency-chain arm (warp-spec ≤
   the 7.3-8.4% decode share; nodecode ceiling 427 TF = 64.6% of peak);
   three lived traps: pair/single harness ratios (honest math is
   single-vs-single), stripped-body re-scheduling (spill state is its own
   datum), garbage-dispatch gating (test-only launchers).
3. `tile-shrink-occupancy-pays-feed-economics` (E): fragment feed scales as
   1/(tile area per warp) — v12 (the FIRST spill-free 24-warp body, 75
   regs, 0 local) still 0.648×; the 16-warp/128-reg geometry is a local
   optimum bounded by the register file in BOTH directions; reopen
   triggers: a ≤32-reg-acc body clearing +10%, or sm_90+ (TMA + setmaxnreg
   — the Machete class).

## GOAT

- G1: `batch149_pseudocode_parses` 3/3 ✓
- G2: lib default + kernel_opt green (counts in the LANDED block below)
- Retrieval floors: `retrieval_precision_regression_gate` green
- G3: clippy `-D warnings` clean on the write-set; score-bench FLAT
- Count pins walked ×7 (tests.rs ×5 incl. the per-category Plan-003 table
  E 55→57 / D 134→135, router.rs, the mod.rs doc — the last was stale at
  614 from BEFORE the sibling landing, fixed to 619)

## Standing lessons (new)

- **Dual-allocation on a STAGED intake lead**: the snapshot's open `- [ ]`
  intake row is as raceable as a queue-1 issue — the pre-write check
  (`git status` + `ls -lat`) cannot see a sibling's PARALLEL authoring, only
  their landed files. The cheap defense at batch-PLAN time: `git fetch` +
  re-read the snapshot's state ON ORIGIN before allocating numbers (this
  session fetched at start; the sibling landed ~1 h later mid-flight — the
  push rejection was the first signal). The yield path (branch + reset +
  delta-analysis + re-issue with fresh numbers) cost one rebase-abort and
  produced a BETTER corpus read than either batch alone (theirs: the
  integrated route-split; ours: the standalone laws).
- **Line-ending audit is part of any rider append** (the CRLF hazard,
  recorded in the yielded batch's work): check `raw.count(b'\r\n')` against
  HEAD before AND after — entries_gpu_15 is CRLF-native (the only one);
  a text-mode read + `newline=''` write flattens it silently.

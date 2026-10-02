# Plan 133 — Batch 148: the Bakhshaliyev sm_89 FP8 GEMM series (occupancy-before-async + hw-barrier)

**Status:** COMPLETE — Batch 148 landed 2026-09-16 (all GOAT gates green; the admission record + corrections in [Research 174](../.research/174_bakhshaliyev_sm89_fp8_gemm_staged.md) §Admission record). Executed the staged charter: the GOAT re-grade precondition CLEARED at admission — riir-ai's Issue-918 rung-A occupancy lane landed + measured-closed as [Bench 933](../../riir-ai/.benchmarks/933_plan597_s0_rung_a_occupancy.md) (read-only citation) the same window this batch was staged — the occupancy-before-async candidate is graded against BOTH the external source and our own 4090 cells.

## Charter (Research 174, verbatim delta)

1. NEW rule `occupancy-before-async-consumer-ada` — cp.async multistage is a winner only
   in the latency/occupancy-starved regime; at high occupancy on compute-bound sm_89 the
   commit/wait-group overhead exceeds the overlap (external 318–418 vs 546 TF). Re-graded
   at admission against Bench 933: the "+1 block/SM" decision SPLITS by route — bought
   with SAVED SMEM (XOR swizzle stride-144→128: 3 blocks/SM, 513→546) it wins; bought with
   CAPPED REGISTERS (`__launch_bounds__` MINB=3 → 85-reg cap → 56 B spill → 0.516×; the
   nodecode twin 104 B → 0.340×; external launch_bounds −20%) it loses; bought with a
   TILE-SHRINK (v12: spill-free at 75 regs, 24 warps, still 0.648× — tile economics beat
   occupancy) it loses. Category E (the occupancy family: occupancy-grid-cap-refuted-
   uncapped-default, grid-collapsing-mega-kernel-occupancy-trap, plane-specialization).
2. NEW rule `hw-barrier-beats-mbarrier-polling` — software mbarrier polling burned
   −16%/−19% vs BAR.SYNC on the same compute-bound lane (syncthreads 590 / mbar_poll 494
   / mbar_yield 478 TF); "software polling burns issue slots on test instructions that
   could be running MMA or loads". Regime boundary carried in-rule: on sm_90+ TMA
   single-issuer designs mbarrier.try_wait is the CORRECT mechanism (tracks DMA completion
   BAR.SYNC cannot see) and exits on first read — the anti-pattern is consumer-side
   intra-block handoff polling in compute-bound all-thread-wait kernels. Category B (the
   masked-branchless-wait-collapse precedent: wait-mechanism compute cost).
3. Riders ×5: `xor-swizzle-smem-static-index-tables` (+ the occupancy-COMPOUNDING face,
   513→546), `ldmatrix-mma-layout-loads` (+ the sm_89 loser cell 458 — compiler-scheduled
   u32 loads through swizzle4() beat it), `stream-k-persistent-cta-mac-segments` (+ the
   compute-bound sm_89 loser cell 506, −7%, L2 interference), `cpasync-multistage-tile-
   pipeline` (+ the occupancy-starved loser regime, per (1)), `mma-fragment-layout-m16n8k16`
   (+ the SM89 FP8 QMMA.16832 A-fragment a1↔a2 swap + the SASS dead-code probe methodology
   + the hardware-facts row: FP8-f16-acc 642.51 = 2× FP8-f32-acc 329.33 TF; cuBLASLt's
   gate is software-only; TMA DMA physically absent on AD102).
4. NOT staged (charter): the 587 overclock numbers (OC-derived, outside pinned-cell
   methodology); FlashAttention follow-ups (sm_120); the INT8 full-rate sub-claim
   (intake-only provenance, not in either fetched text).

## Tasks

- [x] T0 — Dedup re-verification (coordinator grep, vocabulary translation, both corpora):
  `mbarrier|bar.sync` → 6 files, all adjudicated (the B130 TMA rider's "try_wait NOT a
  cost" cell becomes the NEW rule's regime counter-example, not a duplicate — the TMA
  lane polls for DMA completion, the refuted lane polls for consumer handoff);
  `launch_bounds|minb|blocks.per.sm` → the autotune-warps-stages riders (B130 forcing-
  spills cell + B138 deep-ring-losers cell) become distinct-from rows + the sweep surface
  the new rule's regime boundary rides ON; `occupancy` family → E confirmed as the
  category home (4 existing occupancy rules); `warp.special` → plane-specialization +
  setmaxnreg-fit-gate (sm_90+ scope — the reason sm_89 cannot buy register headroom).
- [x] T1 — Author `entries_gpu_43.rs`: the 2 new rules (fn-wrapped Rust-shaped
  pseudocode, the B122/B123 lesson), quote discipline per Research 174 (author-attributed
  claims labeled; stock-clock cells only; the 506-vs-546 cell split from the 546-8192³
  cell; the INT8 sub-claim dropped).
- [x] T2 — Append the 5 riders to their rules (entries_gpu_09.rs ×4, entries_gpu_15.rs ×1),
  each labeled "RIDER (Batch 148, the Bakhshaliyev sm_89 FP8 GEMM series — Research 174)".
- [x] T3 — Standing gates: `batch148_pseudocode_parses` (G1) + count pins 614→616 at the
  three assert sites (+ comment/message updates through Batch 148) + KERNEL_CATEGORIES
  E +1 / B +1 + per-category count test (E 54→55, B 64→65, sum 616 ×2) +
  `mod entries_gpu_43` wiring in corpus/mod.rs.
- [x] T4 — GOAT: G1 parse gate; G2 lib tests at default + kernel_opt + the
  kernel_opt,latent_retrieval retrieval floors re-measure; G3 clippy 0 on the write-set
  (default + kernel_opt feature states). kernel_verify is a STUB — these land as
  measured-source records (Batch-129/144/145 convention; the numbers ARE the decision
  content, Bench 933 + Research 174 the G-evidence).
- [x] T5 — Docs: Research 174 status DISTILLED → RECORD (consumed by Batch 148, admission
  corrections recorded); AGENTS.md kernel_opt 614→616 (×2 sites);
  `.docs/11_domains/kernel_opt.md` count line + Batch 148 row; `.distill/001` snapshot
  update (charter consumed, next pool unchanged — arxiv walk #10 Mon 09-21).
- [x] T6 — Commit `feat: batch 148: ...` + push.

## Admission-time corrections (recorded in Research 174 + here)

1. **The GOAT re-grade split the rule's own headline.** The staged "+1 block/SM buys more
   than a deeper pipeline" is TRUE for the SMEM route and FALSE for the register route —
   Bench 933 measured the register route refuted three ways (reg-cap spill 0.516×/0.340×,
   launch_bounds −20% external, tile-shrink-economics 0.648×). The rule ships with the
   route split as its primary decision content; the unsplit form would have been a
   corpus defect (citing a win whose losing half we measured ourselves).
2. Category adjudication: occupancy-before-async → E (occupancy family home), NOT the
   charter's "B or E" open question — the four existing occupancy rules all live in E.
   hw-barrier → B per charter, consistent with masked-branchless-wait-collapse.
3. The 0.516× vs 0.522× discrepancy inside Bench 933 (§4 table vs §7 summary): quote §4's
   primary table cells (0.516×, 2.113 ms, 172.8 TF) + §6's nodecode 0.340×; §7's summary
   figures are the re-run's rounding — cite the tables, never the summary line, per the
   quote discipline.

## GOAT gate

- G1: both new rules' ref_shape + opt_pattern parse (syn gate) + present in corpus.
- G2: `cargo test --lib` default + `--features kernel_opt` + the retrieval floor re-run
  (`--features kernel_opt,latent_retrieval --lib retrieval`): kernel self-variants 100%,
  blended floors hold (adding 2 entries grows the population; floors are minimums).
- G3: clippy 0 on the write-set at default + kernel_opt feature states.
- G4 (benchability): N/A-in-repo by design (kernel_verify STUB; Batch-129/145 convention —
  measured-source records; the 4090 re-grade IS the bench evidence, read-only cited).

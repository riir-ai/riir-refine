# Plan 065: Batch 98 — ds4 dense.metal/moe.metal distill (the queued NEXT BATCH LEADS)

**Status:** COMPLETE — all gates green, committed; source already vendored at `../vendor/ds4` @ `110afdd8` (Batch 97's pin, quotes re-verified there).

Source: antirez/ds4 (DwarfStar, the league 250.1 t/s M3 Max opponent). Batch 97 mined the
quant/tile/threading lanes; this batch consumes the leads Batch 97 explicitly queued in the
mining queue snapshot ("NEXT BATCH LEADS ... dense.metal — DS4-native matvec family"):

1. **quad-composition via pair-impl composition + args aliasing** — dense.metal L1100-1153:
   ONE kernel hosts FOUR same-input projections by splitting the grid at tgs0 and calling the
   existing pair impl twice with a per-range aliased args struct (device-scope barrier before
   the state append).
2. **`kernel_mul_mv_id_mxfp4_sum6_*` route-fused down-projection SUM** — moe.metal family:
   six routes fused into one accumulation pass with a single final simd_sum (same-input member).
3. **shared-read-not-merging decision boundary** — dense.metal L325-330: two same-input Q8_0
   matrices share the ACTIVATION read but deliberately keep per-projection reduction shapes
   (the merge family's boundary case).
4. **tiny-dense-matvec scalar fallback** — dense.metal L1440-1441: parallelism DOWN-shift for
   short rows, trading lanes for dispatch + reduction overhead.
5. **small-batch matvec-ext bridging GEMV and GEMM** — dense.metal L1596+L1697, 2..8 prompt
   tokens.
6. **P8-intake: host ds4_metal.m single-entry pipeline memo** — L3077-3081 ("decode calls this
   once per layer with identical arguments").
7. **P8-intake: M5 stable-sigmoid-vs-exp annotation** — tanh form produced non-finite logits at
   fused vector sites on M5 Max while finite in the standalone path (dsv4_hc lane).

- [x] **T0** Plan + numbering (Plan 065 via highwater 64; Research 118 next — `.research/.highwater`
      reads 115 but 116/117 exist, STALE — bump to the true max at write time).
- [x] **T1** Dedup grep BEFORE proposing: all 7 candidates against the existing corpus
      (kernel_opt 435, esp. the same-input family B15/B58/B63/B77/B94-sibling, the merge family
      B26/B19, B23 hardcoded-fast-path, B39 tiny-param rebuild, B44 row-dot, B95 comparator).
- [x] **T2** Two parallel subagent lanes read the target files + propose rules (pattern, bounded
      fix-space, category, rationale, exact source quotes): Lane A = dense.metal leads 1/3/4/5,
      Lane B = moe.metal sum6 + dsv4_hc.metal M5 sigmoid + ds4_metal.m host memo.
- [x] **T3** Coordinator re-verifies EVERY quoted line against the real file (byte-accurate) +
      the dedup verdict per candidate (land / rider / drop, with reasons).
- [x] **T4** Corpus entries + category counts + `batch98_pseudocode_parses` gate (cfg(not(kernel_verify)))
      + any mandatory verification gates (exhaustive oracle where numerics are claimed).
- [x] **T5** GOAT: G1 parse gate; G2 kernel_opt lib tests; G3 clippy 0 in BOTH feature states +
      retrieval floors 14/14 (`--features kernel_opt,latent_retrieval`) + score-bench recorded.
- [x] **T6** Research note `.research/118_*.md` (with `**Status:**` line) + AGENTS.md batch entry +
      corpus-count bump + `.distill/001_mining_queue_snapshot.md` queue update (same commit).
- [x] **T7** Commit `feat: batch 98: ...` + push.

**Dedup pre-analysis (to verify at T1):** lead 1's in-kernel grid-split + args-aliasing vs B58's
launcher-level multi-segment; lead 2's SUM composition vs B15's two-output gate+up; lead 5's
GEMV↔GEMM bridge vs B77 batch-width-routed-dual-tile-twins; candidate 6's state-object memo vs
B39's tiny-scalar inverse rule. Leads 3 (boundary rule) and 7 (numerics annotation) are new
vocabulary with no obvious twin.

# Plan 071 — Batch 102: FlashQuant sparse-dense fusion distill (arXiv 2608.15531)

**Status:** COMPLETE — 6 rules landed (corpus 453 → 459), Research 123; taken over + renumbered 070 → 071 on 2026-09-04 (4090 box, CPU-only unit while the 504 T3 matrix holds the GPU)
**Created:** 2026-09-03
**Batch:** 102 (kernel_opt, paper distill)
**Source:** FlashQuant: Sparse-Dense Fusion for Outlier-Aware W4A16 (2608.15531, 08-16T04:57Z)
**Provenance:** armed by arxiv walk #6 (`.research/121_arxiv_pool_refill_walk_6.md`, commits
`7a99912` + `d5e8461`); headline REFUTED for direct adoption at walk time (INT4+BF16 weight
outliers unreachable from ternary/Q4_K; batch≥8 regime pads to 8, advantage "diminishes for
very small batches such as 1 and 2" — not our batch-1 decode). The FUSION MECHANISM is the
distillable core: heterogeneous-EXECUTION-UNIT warp specialization (TC dense ∥ CUDA-core
sparse over one tile space, Stream-K, disjoint output-column ownership via LPT heap) = the
additive-heterogeneous-paths member of the B58/B98 same-input family, with the outlier
instantiation as evidence only.

**Session constraint:** Plan-371 Stage-1 training window live on the 4090 until ~06:45
09-04 — CPU-only unit, targeted GOAT validation (B95 honest-skip shape: affected feature
states individually, full CI guard NOT run this cycle), repo's own warm target dir (no
sibling cargo build active; RAM 82% — avoid --all-features).

## Tasks

- [x] T1 Fetch + read the paper full text — DONE 2026-09-04: the jina PDF route
      500'd twice; the arXiv HTML v1 rendering (webReader) was the fallback and
      carried the full text (recorded as the paper-fetch lesson). All mechanism
      sections extracted; quotes byte-accurate to the HTML prose.
- [x] T2 Dedup grep BEFORE proposing — DONE: FlashQuant/Tile-COO/stream-k/
      uint8-index-width/LPT/min-heap/bank-group all returned ZERO corpus hits
      across all 15 corpus files (lanes A + B grepped independently); named
      neighbors read in full by the lanes; verdicts in Research 123.
- [x] T3 Spawn 2 parallel subagent lanes — DONE: Lane A = fusion + LPT
      ownership (2 candidates, both LAND); Lane B = Tile-COO + bucket partition
      + bank-group reorder + Stream-K (4 candidates, all LAND). Coordinator
      re-verified every quote against the fetched text (one correction: the
      Stream-K reduction quote's exact form). Adjudication: Stream-K LAND (Lane
      B's decomposition-vs-reduction kill-check beats Lane A's rider deferral);
      EW + Sliced-K folded into the fusion rule; categories remapped from the
      lanes' letters (tile-coo B→N, bank-reorder B→A).
- [x] T4 Write corpus entries (entries_gpu_15.rs — new file; the plan's
      "entries_gpu_07.rs" note was the B57-era pointer) + batch102_rules_present +
      batch102_concepts_covered + batch102_pseudocode_parses (cfg(not(kernel_verify))
      per B70). 6 rules, corpus 453 → 459 (A +2, C +2, H +1, N +1).
- [x] T5 GOAT (targeted, B95 shape): G1 batch102 gates 3/3 first-shot PASS; G2
      kernel_opt lib 1282/0/1i + default lib 1029/0 + kernel_opt+latent
      1389/0/2i; G3 clippy 0 both states (one doc_markdown round: ternary/Q4_K
      backticks); retrieval floors kernel 100/100/100 at 30 labeled — HELD zero
      re-pins. Full CI guard NOT run (the B95 honest-skip shape; the 504 matrix
      holds the box) — recorded.
- [x] T6 Research note `.research/123_flashquant_sparse_dense_fusion_distill.md`
      (highwater was 122 at write, scanned; bumped to 123); AGENTS.md corpus
      count 453→459 + Batch 102 ledger entry; .docs kernel_opt has no rule-count
      table to update (verified); queue snapshot update (same commit).
- [x] T7 Commit `feat: batch 102: ...` + push (stay on develop) — see git log.

## Takeover + renumber note (2026-09-04)

The batch was filed 09-03 by a sibling session that left this plan all-unchecked
and untracked; no cargo processes were live and no corpus edits existed at
takeover (~02:00 09-04, 4090 box, while the 504 T3 NoRA confirm matrix holds
the GPU). Renumbered 070 → 071 per the reserved-071 collision flag in the queue
snapshot (kat062 keeps 070, committed `eae2e4e`); `.plans/.highwater` stays 72
(write-back = max(72, 71) — no regression, the Bench-675 rule).

## Dedup surface (from walk #6, to be confirmed by T2 greps)

| Neighbor | Claimed overlap | Expected delta |
|---|---|---|
| B58 same-input-gemv-segments-single-launch | one launch over heterogeneous segments | B58 = same-input GEMV segments via ONE multi-segment launcher; FlashQuant = different EXECUTION UNITS (TC vs CUDA core) over one tile space — the unit-heterogeneous member |
| B98 moe-routed-down-sum-single-accumulate-pass | fusing heterogeneous compute into one output space | B98 = k expert outputs SUMMED into one row-space; FlashQuant = sparse+dense paths writing DISJOINT output columns (no sum, additive partition) |
| B15 fuse-gate-up-gemv-activation | GEMV-level register sharing | different axis (same-input weight concat vs unit-heterogeneous) |
| B92 stripe-sms-l2-semaphore-reduce | Stream-K-style striped reduce | B92 = cross-BLOCK device-memory reduce behind counting semaphore; FlashQuant Stream-K = work-splitting over the fused tile space — possibly SAME mechanism, verify before landing as separate rule |
| B92 xor-swizzle / B79 vectorized-smem-write | smem bank-conflict avoidance | Tile-COO is a DATA-ENCODING fix (layout of sparse values in smem) vs SWIZZLE (address transform) — verify |
| B79 plane-specialization | load/compute role split per plane | per-plane SPECIALIZATION within homogeneous units vs per-tile-path heterogeneous units |
| B96 bit-plane-plane-sums | sparse encoding for dot | different: bit-plane math vs smem layout encoding |

## Disposition rules (decided at walk #6, binding here)

- The outlier sidecar itself (INT4+BF16) does NOT land — unreachable from our formats.
- If (b) Tile-COO = same mechanism as an existing smem rule → rider/drop, not a rule.
- If Stream-K ≡ B92 stripe → rider on B92, not a standalone rule.
- Honest scope notes in-rule: batch≥8 prefill regime, NOT batch-1 decode.

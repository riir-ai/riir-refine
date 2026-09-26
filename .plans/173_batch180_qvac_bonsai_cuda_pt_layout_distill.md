# Plan 173 — Batch 180: qvac-fabric Bonsai CUDA PT-layout distill

**Status:** COMPLETE 2026-09-26 ~20:4x +07 (the ~19:5x slot executed; 4 rules + 3 riders, corpus 638→642; all gates green — lib 2317/0 · kernel_heal 2684/0 · +latent 2858/0 · retrieval floors held · clippy -D ×2 · batch180 ×5; score-bench not owed, zero clippy-corpus delta; `.raw` removed at close)
**Verdict:** Research 211 (YES, A− corpus / LANE INTEL HIGH / kernel_opt)
**Source pin:** `tetherto/qvac-fabric-llm.cpp @ b9bf3a820731b70e5f05130484bdb1518a66beee` (temp-10549 tip; CUDA arm = merge `a95b44152`, PR #285) — MIT. **T0 re-clones at the pin** (the ephemeral-clone law).

**CROSS-REFERENCE (added 2026-09-26, the Research-212 verdict gate):** LEAD-2 below (`deferred-bias-ternary-dp4a-exact-isum`, WEIGHT-side deferral) is the mirror image of Research 212's LEAD-1 (`signed-activation-codes-drop-dequant-offset-term`, ACTIVATION-side elimination — Plan 174 / nominal Batch 181, the mlxfast-bonsai2 arena distill). Both are members of the offset-handling family (with B107 uzu consumer-lookup + B96 bit-plane). At mine time the two batches MAY share one offset-family fixture with a common A/B; whichever mines second MUST cite the first's landed rule in its entry's distinct-from clause, so the family is not counted twice. Family table: `.research/212` section "Family table".

## Leads (4 + 1 maybe)

1. `planar-transposed-activation-multi-column-quant-matvec` (A−, GOAT) — mmvq-ptq1_0.cuh:1–79, 91–233. Measured 2.2×@2col / 3.3×@3col (RTX 3060); 8+1 aligned 16B pieces vs 32 scattered 4B loads; stride-compat with block_q8_1; bit-exact same-op-order; anti-regimes: HIP refusal, sm_120 NMSE gate, 1-col→SOA. Regime clause carries the NMSE arch escape + the three-regime enum (LEAD-5 folded in unless the dispatch table stands alone at authoring).
2. `deferred-bias-ternary-dp4a-exact-isum` (A−/B+) — mmvq-ptq1_0.cuh:72–79/85–90/114–126 + vecdotq.cuh:892–905 + quantize.cu:177–202. **KINSHIP-FIRST framing**: the lane-widened decode is `swar-lane-widened-base3-dp4a-dot` (B116 ADDENDUM, same prism lineage); the novelty = the deferred-bias fold (raw {0,1,2} digits, `dp4a_us`, per-32-sub-block `(sumi − isum)` exact-int correction, quantize-side `__short_as_half` isum slot). Compose-with, never re-state.
3. `row-kblock-item-space-flattening-host-util-search` (B+) — mmvq-ptq1_0.cuh:414–466. (row-group, K-block) item flattening; the util>0.999 host search; the 48-KiB static_assert boundary pins. Kinship row: `fold-query-tokens-for-occupancy` (attention-axis twin).
4. `fixed-order-interleaved-accumulator-epilogue` (B+) — mmvq-ptq1_0.cuh:548–579. ncu 23%-of-instructions / 50%-of-warp-stalls motivation; 4 interleaved accumulators (k mod 4) + (s0+s1)+(s2+s3); column-count bit-invariance free. Kinship row: `batch-invariance-locks-reduction-geometry` (contract-side twin).
5. MAYBE `three-regime-activation-layout-dispatch` as its own D rule — fold into LEAD-1's regime clause unless authoring shows it stands alone (common.cuh:1043–1046).

**Riders:** the FWHT-CUDA-twin annotation (fwht_cuda_block = the `fwht-wide-block-row-per-threadgroup-exchange-ladder` CUDA instance + the arch-gated 512 boundary + the signs-at-load folding — feeds the B176 pre-graded set's addendum if B176 lands first); `prefetch.global.L2` = the weight-stream family's 4th external proof; the PQ2_0 group-128 "same per-chunk codec" instantiation note (B116 family evidence).

## Tasks

- [x] **T0** — re-clone at the pin `b9bf3a8207` (fresh; the verdict's clone was removed); re-verify EVERY lead + rider quote verbatim against the tree (the B177/B178 law; the branch is an integration tree — also re-check the branch tip for post-pin movement and adjudicate: new commits beyond #286 → read the delta, pin stands unless content moved).
- [x] **T1** — dedup re-run vs the POST-B179 corpus under BOTH vocabularies per lead (the corpus may have moved — B176 may have landed between staging and the slot); confirm LEAD-2's kinship row is still the swar rule; write the six kinship/adjacency rows (occupancy twin, batch-invariance twin, B20/B116 family, weight-stream 4th proof, cast-at-load class, NMSE/tolerance class).
- [x] **T2** — author the entries (original minimal fixtures; the SWAR exhaustive-equivalence gate applies to LEAD-2's integer arithmetic — the B59 ported-numeric mandate; LEAD-1's fixture pair demonstrates the layout-equivalence class with the bit-exactness assertion; LEAD-3's fixture pins the util-search boundary static_asserts; LEAD-4's fixture demonstrates fixed-order vs shuffle-order equivalence).
- [x] **T3** — registration + pins: router table, per-category counts (N/D/E per lead's category at authoring), kernel_opt 638→N at FIVE+ sites (the B178 walk list: router + per-category + legacy test sites + the 638 doc rows), AGENTS.md feature row.
- [x] **T4** — standing gates: batch180 e2e ×4; lib default; kernel_heal; kernel_heal+latent; retrieval floors; clippy -D ×2 postures; score-bench (expect FLAT — no clippy-corpus delta).
- [x] **T5** — the lane-intel follow-through: a dated line in the 971 issue (riir-ai) noting the distill consumed the branch's kernel patterns (the watch's orbit-intel escalation; no trigger implication); the FWHT-twin annotation row feeds B176's addendum if armed.
- [x] **T6** — records: Research 211 → Status RECORD + mine-time appendix; this plan → COMPLETE; queue snapshot row; `.raw/` removed at close.
- [x] **T7** — commit + push (claim protocol: pre-flight claim commit, origin twin-check — the B177 discipline; RIDER adjudications at slot time on the then-current evidence).

## Pre-registered expectations

- Yield 2–3 landed rules (the 50% falsification class); 3–4 authored.
- The claim protocol fires ONLY at the slot (one-batch-per-day after B178; B176 preempts if #29096 merges).
- No freeze-table regen (kernel_opt corpus, not clippy_lints).

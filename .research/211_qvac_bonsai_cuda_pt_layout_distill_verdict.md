# Research 211 — qvac-fabric Bonsai CUDA backend (QVAC-25280) distill verdict

**Status:** RECORD — Batch 180 LANDED 2026-09-26 (~19:5x slot, +4 rules / +3 riders, corpus 638→642; gates lib 2317/0 · kernel_heal 2684/0 · +latent 2858/0 · retrieval floors held · clippy -D ×2 clean; the plan-time re-clone re-verified every quote verbatim — see the mine-time appendix below)
**Date:** 2026-09-25 (~20:1x–21:0x +07, 4090 box — the post-envelope B180 pre-stage)
**Source:** `tetherto/qvac-fabric-llm.cpp` @ `b9bf3a820731b70e5f05130484bdb1518a66beee` (branch `temp-10549` tip = the #286 Vulkan merge; MIT — "Copyright (c) 2023-2026 The ggml authors"; shallow clone `.raw/qvac-bonsai`, **removed at verdict close**). The CUDA arm's merge commit: `a95b44152` (PR #285, 2026-09-25T10:31Z, +2102/−123 across 27 files).

## Verdict

**YES, A− corpus / LANE INTEL HIGH (the Bonsai league lane — our scored model class) / route `kernel_opt`** — the cinference density class (measured numbers in-source, complete non-WIP kernels, three-regime decision content). Queued as nominal **Batch 180** behind the B176 FWHT rider (event-gated) and the walk-#11 cycle; the cadence slot is ~19:5x 09-26.

## Provenance

tetherto/qvac-fabric-llm.cpp = a PrismML-lineage llama.cpp fork (the qvac integration tree; carries PTQ1_0/PQ2_0 — the prism-format quants, SAME lineage as the already-mined `PrismML-Eng/llama.cpp @ 7113f6462` = Batch 116's swar rule source). The QVAC-25280 Bonsai set: #283 core+CPU (09-24) + #284 Metal + **#285 CUDA (this distill's primary)** + #286 Vulkan (09-25) — the complete backend set assembled on the integration branch within the hour (the 971 watch's 18:00 finding). The content may yet land on ggml-org via the threat chain's link 2 (a tetherto→ggml-org PR); the fork pin is load-bearing exactly as B177/B178's were.

## The three-regime spine (common.cuh:1043–1046 — the family's dispatch)

```cpp
GGML_CUDA_Q8_1_SOA_ISUM = 1, // PTQ1_0, one column: warp-transposed, exact int sums
GGML_CUDA_Q8_1_PT       = 2, // PTQ1_0, 2-8 columns or MoE ids: planar-transposed
```
(the default 0 = the stock interleaved block_q8_1 layout). **Activation layout is a REGIME keyed on the destination column count** — the dispatch decision that generates LEAD-1's layout and LEAD-2's arithmetic.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `planar-transposed-activation-multi-column-quant-matvec` — the PT layout: 9 planes of `nblk*16B` 16-byte pieces; a thread's 128 quants = 8 aligned int4 loads + 1 for (d,s); "Adjacent threads read adjacent 16-byte pieces of the same plane, so a warp load touches 4 cache lines instead of 32, and a thread reuses each piece for every row it owns"; **measured 2.2× at 2 columns, 3.3× at 3 (RTX 3060)**; column stride `9*nblk*16 = padded_row*9/8` = exactly the block_q8_1 stride so launcher math survives; "The decode, the dp4a sequence and the fp32 epilogue are the same operations in the same order ... so every column count produces the same bits for a given column"; anti-regimes: HIP keeps block_q8_1 (`ptq1_0_pt_enabled()`), sm_120 NMSE gate (see rider), single column routes SOA | mmvq-ptq1_0.cuh:1–79 + 91–233 | **A− GOAT** | zero corpus hits under planar / PT layout / transposed activation (both vocabularies); the regime clause carries the NMSE arch gate + the HIP refusal |
| 2 | `deferred-bias-ternary-dp4a-exact-isum` — decode emits RAW digits {0,1,2} (no per-lane −1), `ggml_cuda_dp4a_us` (the UNSIGNED dp4a variant) accumulates `sum(q·a)`, and the quantize side stores the EXACT int16 activation sum in the block's ds slot (`__short_as_half((short) isum)`, "abs. <= 32*127 fits int16; keep the raw bits in the half slot"); kernel folds per 32-sub-block: `acc += d8 * (sumi − isum)` — "sum((q-1)*a) = sum(q*a) - sum(a), exact in int32, so the result is bit-identical to the biased-weight form"; "two ALU ops per 4 weights cheaper" than the per-lane bias subtract | mmvq-ptq1_0.cuh:72–79, 85–90, 114–126 + vecdotq.cuh:892–905 + quantize.cu:177–202 | **A−/B+** | **the B177-class T2 framing**: the lane-widened decode ITSELF is `swar-lane-widened-base3-dp4a-dot` (Batch 116 ADDENDUM, same prism lineage — positive instance, NOT a new rule); the NOVELTY is the deferred-bias fold + the quantize-format co-design (the isum slot). Frame as the extension/compose rule, cite the kinship explicitly |
| 3 | `row-kblock-item-space-flattening-host-util-search` — work items are (row group, K block) pairs flattened into ONE index space; host picks rows_per_cta so items fill whole 128-thread iterations ("K = 5120 ... keeps 40 of 128 threads busy and K = 17408 (136 blocks) keeps 53%"); the util search: iterate r in rows_per_item steps, `util = items/(iters*THREADS)`, keep the best, stop at util > 0.999; `static_assert` pins the smem boundary cases (48 KiB gated/ungated) | mmvq-ptq1_0.cuh:414–466 | **B+** | kinship `fold-query-tokens-for-occupancy` (the attention-token-axis twin — distinct mechanism, same underutilization class); the static_assert boundary pins are quotable G-content |
| 4 | `fixed-order-interleaved-accumulator-epilogue` — replace one-warp-per-pair shuffle reduction with one thread per (row,col) doing a fixed-order sequential sum, 4 interleaved accumulators (k mod 4), then `(s0+s1)+(s2+s3)`; **ncu-profiled: the shuffle epilogue "was ~23% of all issued instructions ... and held ~50% of all warp stall samples while the CTA did no memory traffic" (RTX 4070)**; "The order depends only on the weight shape, so a column gives the same bits for every column count" | mmvq-ptq1_0.cuh:548–579 | **B+** | kinship `batch-invariance-locks-reduction-geometry` (the contract-side twin — here the invariance comes FREE with the measured win, not from a lock); distinct mechanism (epilogue replacement vs geometry freeze) |
| 5 | MAYBE `three-regime-activation-layout-dispatch` as its own D-class rule (the enum + switch: default/SOA_ISUM@1col/PT@2-8+MoE) — or fold into LEAD-1's regime clause. Decide at mine time by size; the corpus convention (one mechanism per rule) suggests folding UNLESS the dispatch table stands alone | common.cuh:1043–1046, ggml-cuda.cu dispatch | B/B− | the spine is quotable either way |

**Drops (dedup-killed or weak):**
- The CUDA `fwht_cuda_block` (one row per 256-thread block, three-tier ladder shfl_xor→smem-xor→register) = **the CUDA twin of `fwht-wide-block-row-per-threadgroup-exchange-ladder`** (the Metal #29095 rule) + the ladder family (`fwht-butterfly-staging-ladder-warp-smem-register`, `fwht-shuffle-xor-butterfly-register-resident`) — a "CUDA instance verified" annotation row at mine time, not a rule. The arch-gated 512 boundary (Blackwell rows==1 → block) is a regime datum for that family.
- The `has_signs` per-row sign folding (`signs_row = signs + (r % n_blk) * N`, multiply at load) = the randomized-Hadamard sign vector folded into the FWHT load — the cast-at-load class (composes with the B176 pre-graded set); a RIDER on the family, and lane intel for the #257 Hadamard-contract-v2 format-watch.
- Odd-stride smem (`bprp = bpr + 1`, "odd, so the per-pair epilogue reads are bank-conflict-free") — the classic trick, 9 corpus hits.
- HIP `__byte_perm`→`__builtin_amdgcn_perm` substitutions + the 0x020100FF LUT-perm constants — HIP-lane portability content; our corpus is CUDA/Metal/CubeCL; drop (annotation only).
- `prefetch.global.L2` of the next weight block (mmvq-ptq1_0.cuh:238–242) — the weight-stream family's 4th external proof; rider on `weight-stream-non-temporal-loads`-adjacent rules.
- `ptq1_0_mmvq_small_k` (the K-block-strided generic loop) + GLU fusion epilogue — ambient structure (the fused-activation-quantize family already owns the shape).
- `vec_dot_pq2_0_q8_1` (group-128 with ONE scale, 4×32-chunks) — instantiation of shipped codec rules (the B116 family); the "same per-chunk codec as Q2_0" comment is the dedup evidence.

## GOAT check (lead 1)

Benchable on our lane: kernel_opt measured-GOAT distillation class (the layout rule carries the source's own measured 2.2×/3.3× at 2/3 columns, RTX 3060, + the ncu epilogue profile). Exact semantics: the rule's core claim is bit-exactness by construction ("same operations in the same order ... same bits for a given column") — the fixture pair demonstrates the layout equivalence class; integer path → G1 bit-identical trivially. Magnitude story: 32 scattered 4B loads → 8+1 aligned 16B pieces (8× fewer transactions, 4 cache lines/warp vs 32) — mechanism-level, not "probably faster". Deterministic: yes. The bench-home question (Bench 947/772-style CUDA lane vs fixture-pair) is a mine-time decision — the source's own numbers make it measurable-GOAT eligible either way.

## Lane intel (the Bonsai league lane — HIGH)

- **This IS the opponent's Bonsai serving stack** (the 971 watch's 18:00 finding: "the complete Bonsai backend set assembled on temp-10549"). Our scored model is Ternary-Bonsai-2-27B-PQ2_0; their PTQ1_0/PQ2_0 CUDA kernels are the kernels the league's fork lane will run when the threat chain's link 2 (tetherto→ggml-org PR) fires. Distilling the PATTERNS now = pre-absorbing the opponent's kernel engineering before it lands upstream (the standing league policy — the perf-rematch skill diffs opponent merged PRs into the distill queue; this is that, one step early).
- **riir-infer/riir-ai consumer lanes**: the `bonsai2_hadamard` feature (issue879 GDN quant certification, bonsai2_rotation), the bonsai decode/prefill league rows, and the `#257 Hadamard contract v2` tied-output format-watch (the fwht.cu signed variant IS that contract's kernel side — `ggml_cuda_op_fwht_signed`).
- **Not transferable**: absolute tok/s (their box/silicon); NVFP4 content (none here — this set is ternary/2-bit, our class); the Vulkan/Metal arms (other lanes — the Metal arm #284 is M3-lane intel only if ggml-org picks it up).
- **No scored-cell re-pin owed** (the fork's scored surface did not move — this is the integration branch, not master, and the prism fork tag is unchanged).

## Product coverage

kernel_opt is the private corpus; the famous-lib axis does not gate. Continuity note: this is the Nth llama.cpp-family source (prism `7113f6462`, mtplx, ggml-org #29094/#29095, cinference, now qvac) — the family is a standing corpus vein, and the PTQ1_0/PQ2_0 codec family now has deep coverage (the B116 swar rule + B20 packing + this batch's extension).

## Dedup evidence (per candidate, both vocabularies)

- planar / PT layout / transposed activation / activation layout: 1 file each — READ, unrelated (a fusion-decline diagnostics rule's "planar fold" + a tile-local sparse outlier format). PASS.
- lane-widened trit decode: **HIT** `swar-lane-widened-base3-dp4a-dot` (entries_gpu_27) — same lineage, same constants (0x4140/0x4342, 0x7531, 0x00FF00FF), same consumption-alignment notes. LEAD-2 reframed as the deferred-bias extension (the kinship-first framing, the B177 lesson).
- deferred bias / exact sum / exact activation sum / dp4a_us: 0 hits. PASS (the novel half).
- warp per row / block per row / fwht / butterfly: the 3-rule ladder family — LEAD-6 dropped as the CUDA twin.
- occupancy / item space / work item: `fold-query-tokens-for-occupancy` READ (attention-token fold, distinct mechanism). PASS with kinship.
- interleaved accumulator / shuffle reduction: `batch-invariance-locks-reduction-geometry` READ (contract-side, distinct mechanism). PASS with kinship.
- NMSE / rows_per_cta: 0 hits.

## Yield

**3–4 rules + 3–4 riders** (LEAD-1 A− GOAT, LEAD-2 A−/B+, LEAD-3 B+, LEAD-4 B+, LEAD-5 maybe→fold; riders: the FWHT-CUDA-twin annotation, signs-at-load, prefetch-4th-proof, the NMSE gate). After the ~50% coordinator falsification class: expect 2–3 to land intact.

## Honest caveats

- **The pin is load-bearing and the branch is an integration tree** — quotes verified at `b9bf3a8207` this session; a ggml-org landing (threat-chain link 2) may re-home the content upstream; re-verify every quote at mine time (the B177/B178 law).
- The measured numbers are the AUTHOR's (RTX 3060/4070, single box, no variance reported); our fixture-pair gates are the enforcement, the source's numbers are the motivation only.
- Unread at verdict: mmvq.cu (+236/−79 restructure), convert.cu (+117), quantize.cu beyond the isum block, ggml-cuda.cu dispatch (+33), the mmq-config-* per-arch tables, the Vulkan/Metal arms entirely (other lanes) — re-read at mine time per the standard clause.
- The MoE gate path (`ptq1_0_mmvq_moe_dot`, fusion/has_gate) was read once; the GLU-fusion epilogue is ambient (existing family).

## Records

## Mine-time appendix (2026-09-26 ~19:5x slot, the landing)

Executed at the ~19:5x 09-26 cadence slot by the 4090 idle-loop session (claim
protocol: origin twin-check CLEAN — no twin landing; the M3 sibling's B181 =
Plan 174 is a different batch). T0: fresh shallow clone at the pin — the
branch tip was STILL `b9bf3a82` at clone time (re-verified at every hourly
poll through the window), zero post-pin movement to adjudicate; every lead +
rider quote re-verified verbatim (LEAD-1 L1-79/91-233, LEAD-2 L68-79/114-126
+ quantize.cu 177-202 + vecdotq.cuh 892-905, LEAD-3 L414-470, LEAD-4 L548-579,
LEAD-5 common.cuh 1039-1052, the fwht rider fwht.cu 82/96-100/202-214, the
prefetch rider mmvq-ptq1_0.cuh 236-248 + mmvq.cu 782-793, the PQ2_0 note
vecdotq.cuh 115-117). T1 dedup re-run vs the post-B179 corpus under both
vocabularies: all four leads NOVEL (zero hits); the three kinship rows
confirmed present (`swar-lane-widened-base3-dp4a-dot` entries_gpu_27,
`fold-query-tokens-for-occupancy` entries_gpu_01, `batch-invariance-locks-
reduction-geometry` entries_gpu_22). T2 authoring deviations from the plan:
none — LEAD-5 folded into LEAD-1's regime clause as pre-registered; the
`prefetch.global.L2` rider landed with a LEVER-distinction wording stronger
than the plan's shorthand (prefetch answers LATENCY, evict hints answer
CAPACITY — the intake's '4th external proof' framing corrected to '4th
external instance of the operand class'), and the two test-grade fixtures
beyond the plan's list (the exhaustive isum identity test caught a real
sign-extension subtlety: the biased form consumes SIGNED weight lanes —
the vsub4 wraps 0→0xFF = −1 as i8) + the util-search boundary port. T4
gates: batch180 ×5 green; lib default 2317/0 (+28 vs B178's 2289 — my 5 +
the sibling's Issue-135/OOD additions); kernel_heal 2684/0; kernel_heal+
latent 2858/0; retrieval floors held (the regression gate green with the 4
new rules in the corpus); clippy -D clean at both postures (3 doc_markdown
healed in-passing, compile-gated); score-bench NOT owed (zero clippy-corpus
delta, the P30 precedent; freeze tables unmoved). `.raw/qvac-bonsai` removed
at close.

- Plan: `.plans/173_batch180_qvac_bonsai_cuda_pt_layout_distill.md`
- Queue line: `.distill/001_mining_queue_snapshot.md` (this commit)
- Clone `.raw/qvac-bonsai` REMOVED at close (the ephemeral-clone law; T0 re-clones at the pin).

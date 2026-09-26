# Research 212 — mlxfast-bonsai2-27b-engine arena distill verdict

**Status:** DISTILLED — pending owner decision (nominal Batch 181, Plan 174)
**Date:** 2026-09-26 (~16:0x–17:3x +07, M3)
**Source:** `Layr-Labs/mlxfast-bonsai2-27b-engine` @ `b3050f0f8c11c239efa9661aafcee62741eb7115` (HEAD, MIT — "Copyright (c) 2026 Layr Labs, Inc."), full clone (41 commits), `.raw/mlxfast-bonsai2-27b-engine`, **removed at verdict close**. Quotes below re-verified at the pin this session.

## What this source is

The **MLX/Metal arena engine for track `bonsai2-27b-mlx-v1`** — a Layr-Labs competitive speedup benchmark on **Ternary-Bonsai-2-27B** (`prism-ml/Ternary-Bonsai-2-27B-mlx-2bit` @ `3f926b41`, = 2-bit affine group-128 MLX pack of Qwen3.8-27B with a signed block Walsh-Hadamard transform, block 1024, folded into the input of every packed projection). Scoring `composite = prefill_gain^0.25 · decode_gain^0.75`, serial-anchored: a control leg on the organizer's reference tree vs the candidate in one session on one box (ranked runner = self-hosted Apple M5 Max). Two DECLARABLE spec decoders, both used unmodified: **MTP head** (EigenLabs/Qwen3.8-27B-MTP-4bit, depth 1–7) and **DFlash 2 block drafter** (z-lab/Qwen3.8-27B-DFlash2 @ `50307d4c`, depth 1–16). Participants submit diffs; organizers land accepted ones as "Accept/Validate submission" commits — **HEAD is the accumulated frontier state of every accepted submission**.

**This is byte-for-byte our league model**: `Ternary-Bonsai-2-27B-PQ2_0.gguf` (riir-train/data), the `bonsai2_hadamard` lane (riir-infer, DEFAULT-ON), league focus narrowed to Bonsai 2 + Qwen3.8/DFlash2 (owner 09-20), Issues 846/919 + Plan 602. The same z-lab DFlash2 head our league chat lane pins at 1.242× over greedy (49.9 tok/s, 4090).

## The user's two questions

**"This commit only gained 13.23% — worth distilling?"** The commit alone: **B−** — its diff is mostly vendored-MLX `quantized_nax.h/.cpp` edits + a 3-line `Hadamard.swift` dtype widening (`uint8 || int8`) + the signed-codes route plumbing; the headline gain partly rides upstream MLX kernel edits, and per-op it is polish on a saturated frontier. But the commit is a window into the repo, and **the REPO is the distill target**: it is a third-party arena engine for OUR league model with the frontier techniques accumulated in-tree.

**"The 256% before is a gem so dig it."** The gem is identified and it is in-tree at HEAD:

## The gem — the frontier's structural half

- **`Hadamard.swift` = 1,735 lines at HEAD and rewritten six times** (+499 → +519 → +519 → +777 → +879 → +921-line submissions: `b8a6bc7`, `402d91e`, `c6e82d1`, `e880f54`, `d5c6433`, `4c8964f`) — the most-contested file in the tree. It carries the fused transform WRITER surface: `SignedBlockHadamard.forwardInt8/forwardWithGroupSums` (one kernel emitting codes+scales+scaledSums), `rotatedSwiGLU`, `rotatedSigmoidGate`, `rotatedGatedRMSNorm` (the rotation fused INTO the activation functions — the transform never exists as a separate pass), `applyPreSigned` (presigned codes carried through), and the packed layer types `HadamardQuantizedLinear/Embedding`.
- **`Qwen35.swift` = 6,935 lines** carrying `Qwen35TensorPackedMatmul` (:5287): Metal **tensor-core MMA over quantized codes** via `mpp::tensor_ops::matmul2d<desc, metal::execution_simdgroups<4>>` with descriptors `64,64,128` (the int8-staged prompt kernel) and `16,TN,128` (the verify-width kernel), staged int8 B double-buffered in threadgroup memory (`threadgroup uint32_t bs[2][64*128/4]`, "B0"/"B1" ring), cooperative-tensor destinations; plus `Qwen35FusedBoundaryQ8` (:4889, RMSNorm+quantize in ONE kernel writing normed output + codes + scales + sums) and `Qwen35FusedHadamard` (:4311, fused activation transform+quantize with PERM staged8 permutation and MPERM row-tiled constants).
- **The decode half**: the DFlash2 block drafter (5 sliding-attention layers, ONE `proposeBlock` per round filling `depth` draft columns, target hidden taps at layers [5,19,33,47,61] fused via `fc`, GDN width-generic commit: `stageCaptured` w≤2 per-position vs `stagePrefixReplay` w≥3 one-recurrence+prefix-replay) + the MTP head, warmed by `warmTargetPrefill` (the exact scored prompt seam run once at load — in-source measured 30–42 ms/window of first-use cost otherwise paid inside every timed window, ~36 ms on the control leg too).

The arena's large composite (user-reported ~256%; leaderboard-side, **not verifiable from git** — see caveats) is decode_gain^0.75-shaped: deep block speculation + this engine around the SAME heads our league pins.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | `signed-activation-codes-drop-dequant-offset-term` — store activation codes SIGNED int8 (q) not unsigned (q+128); the dequant epilogue's folded-offset term AND its two 16-byte loads per lane/group are deleted. Verbatim at pin (`Qwen35.swift:5617–5624`): "The int8-staged prompt kernel reads the activation codes as signed int8 (`q`, not `q + 128`): its products then carry no `128 * colsum` offset, so the epilogue's folded-offset term and its two 16-byte loads per lane and group go away. Same values bit for bit: `fma(s, C, -128 s colsum)` and `s * (C - 128 colsum)` each round once to the same number (the offset is exact in FP32). Only that form takes it; the native and 4-bit-staged forms keep the unsigned codes." Kill-switch `DARKBLOOM_BONSAI_TENSOR_ROUTE_SIGNED=0`. | `Qwen35.swift` (b3050f0) | **B+** | see the regime clause below — the note's load-bearing content |
| 2 | `fused-transform-quantize-writer` family — rotation+group-quant in ONE writer kernel (codes+scales+scaledSums); boundary variant fuses RMSNorm+quant; activation-fn variants (`rotatedSwiGLU` etc.) carry presigned codes through so no separate pass exists. | `Hadamard.swift` + `Qwen35FusedBoundaryQ8` | **B+** | the gem class; kinship `fixed-ref-softmax-decode-walk` (B170) checked — different mechanism, clean |
| 3 | `tensor-core-mma-staged-quantized-codes-double-buffered` — TWO instances: (a) `Qwen35.swift:~5310/5401` the int8-codes prompt kernel (`execution_simdgroups<4>`, 64×64×128) + the 16×TN verify kernel; (b) `DFlash2Draft.swift:~928–963` `DFlash2TensorMatmul` — the DRAFTER's BF16 block projections on the tensor unit (`matmul2d_descriptor(16, 32, 256, …, mode::multiply_accumulate)`, `execution_simdgroup`, K-quarters per simdgroup summed through threadgroup). Instance (b) is the Issue-846 chat-row-relevant one (the drafter's own cost). Rider found beside it: `DARKBLOOM_DFLASH2_STACK_GATEUP` — gate+up weights concatenated ONCE so one matmul replaces two over the same input (weight-stack family, likely a rider not a rule). | both files | **B** | Metal-specific; our kernel_opt has Metal precedent (B170/B176) |
| 4 | `mask-memo-hoist-to-owner-lifetime` — the per-forward sliding-mask memo hoisted to the drafter INSTANCE ("Sliding masks depend only on block geometry. Keep the memo with the drafter so repeated speculative forwards can reuse the same graph"). Also a graph-reuse enabler, not just an alloc hoist. | `DFlash2Draft.swift` (b3050f0) | **B−** | rust_perf caching family; re-check the 7 caching rules at mine time |
| 5 | `seed-warm-the-scored-seam` — run the EXACT scored seam once at init on throwaway state; first-use costs otherwise land inside every timed window. In-source measurement discipline (30–42 ms/window). | `Qwen35DFlash2Assistant.swift` (b3050f0) | **B−** | NotBenchable decision-content; measurement-honesty family (B177 `aa-control` precedent) |

**Drops:** greedy-walk dtype widening (ambient), vision-tower/goldens/arming discipline (arena ops), MTP adapter plumbing (ambient), §6.1 shortlist cast defect (Swift type-system hazard — annotation only: a `HadamardQuantizedLinear` IS-A `QuantizedLinear` so a cast succeeds and raw `.weight` reads silently skip the transform; worth one line in the note, not a rule).

## LEAD-1's regime clause (the load-bearing content)

The bit-identity claim is **conditional, and both conditions must be in the rule**:

1. **Mantissa budget.** The rewrite is exact only while `bits(scale) + bits(colsum) ≤ 24` (FP32 significand). 2-bit weights, group 128: colsum ≤ 384 (9 bits) + FP16 scale (11 bits) = 20 bits — exact. 4-bit ≈ 22 bits — exact. 8-bit: colsum to 32,640 (15 bits) → 26 bits — NOT exact in general. An FP32 scale breaks it earlier. Upstream's own scope is NARROWER STILL: a kernel-form restriction to staged8, not a precision bound (it also excludes the 4-bit-staged form, which IS exact under the mantissa budget — do not read upstream's exclusion as a precision verdict). **The fixture needs a NEGATIVE arm showing the equality breaks outside the regime, not only a bit-identical pass inside it.**
2. **Consumer ISA.** The `q+128` offset usually exists because the consumer's dot instruction wants UNSIGNED activations: x86 VNNI `vpdpbusd` and AVX2 `vpmaddubsw` are u8×s8 — on those lanes the offset is FORCED and signed codes win nothing. It pays where the dot accepts signed activations: CUDA `dp4a`, NEON `sdot` (weight codes 0..3 fit s8), Metal's staged-int8 MMA. Regime: **writer control AND a consumer ISA that takes signed activations.** riir-infer's x86 path may not qualify; its NEON and CUDA paths should — the mine-time A/B must be per-ISA.

**Grade B+ deliberately:** the saving is one epilogue term + two 16-byte loads per lane per group — real, small per op, no measured gain in-source; its value is exactness at zero cost. Let B181's bench earn the A−.

## Family table (the offset-handling family — do not double-count)

| Member | Side | Mechanism | Where |
|---|---|---|---|
| B107 uzu | consumer | precomputed per-group code-sum LOOKUP for the unsigned-code correction (sum_group_size = min(weight_group,128), per-span lookup) | `entries_gpu_20.rs`, landed |
| B96 | consumer | bit-plane input-side offset dot (per-block offset codec in scalar GEMV dots) | landed |
| Research 211 LEAD-2 (B180, pending) | weight | deferred-bias TERNARY dp4a: raw digits {0,1,2}, unsigned `dp4a` accumulates `sum(q·a)`, exact int16 activation sum in the block's ds slot, `acc += d8 * (sumi − isum)` — bit-identical to the biased-weight form | `.research/211` / Plan 173 |
| **Research 212 LEAD-1 (B181, this)** | **activation** | **writer-side ELIMINATION: emit signed codes, the correction term is zero — no lookup, no loads** | this note |

**Cross-reference obligation:** Plan 173 (B180) and Plan 174 (B181) each carry the pointer to the other; the two leads are mirror images (weight-side deferral vs activation-side elimination) and MAY share one offset-family fixture with a common A/B at mine time, so the family is not counted twice across two batches. Decision recorded in both plans.

## Lane intel (the Bonsai league lane — HIGH, the MLX/M5 arena axis)

- **Complements Research 211 (qvac = the fork-lineage CUDA axis).** This is the ARENA axis: a third-party competitive benchmark racing the same model on M5/MLX. Between the two, the opponent-class engineering space around Bonsai-2 serving is now visible from three directions (our stack, the llama.cpp fork family, the MLX arena).
- **Issue 846 chat row (0.36×, the live path):** T-R1 (a head beating z-lab 2.648 tok/step) is NOT fed — the arena uses the SAME heads. What is new is ENGINE-side cycle-cost levers: `warmTargetPrefill`, the mask memo, the GDN width-generic block-commit (`stageCaptured`/`stagePrefixReplay` — verify-path state management under block speculation), and LEAD-3 instance (b) (the drafter's own tensor-matmul). These are levers for OUR verify/cycle path, transferable in mechanism, not in numbers.
- **Issue 919 / Plan 602 B5 fusion rung:** the arena's writer-fused kernels are the MLX-side instance of the same rotation family the fork ships on Metal (block-per-row FWHT + sign-fused load path) — Plan 602 B5's "sign+FWHT fused into the consuming GEMV's load path" has a writer-side analog here (fuse the transform INTO the quantize writer; read codes directly in the matmul). Pre-absorbing opponent-class engineering per the standing league policy.
- **riir-infer `bonsai2_hadamard` lane:** LEAD-1 has a live in-house consumer — we OWN the activation-quant writer on the NEON/CUDA paths. The x86 lane must be checked against regime condition 2 before any claim.
- **Measurement law worth recording:** per-depth oracles (port notes §5.1: "A speculative round verifies its draft in one forward at M > 1, and MLX dispatches a different kernel there, so the free-run tape can fork from the serial tape at a near-tie"; the Qwen3.8-125B track measured the six per-depth oracles byte-identical to each other but DIFFERENT from the serial tape). Directly relevant to how our spec lanes (chat row, dspark) pin correctness.
- **NOT transferable:** absolute numbers (M5 Max ≠ M3/4090), the arena's scores, the MTP/DFlash heads themselves (pinned exports, not engine code).

## Product coverage

kernel_opt/rust_perf are private corpora; the famous-lib axis does not gate. Family continuity: this is the **second mlxfast-family source** (B170 = `mlxfast-gemma4` walk, +7 kernel_opt +9 riders; Plan 162 / Research 198) and the fourth Bonsai-serving source in the queue window (cinference B178, qvac B180 staged, this B181).

## Dedup evidence (both vocabularies, per candidate)

- LEAD-1: `colsum`/`zero-point`/`offset term`/`scaled_sums` → ONE corpus hit, `entries_gpu_20.rs` (B107 uzu) — READ; it PRECOMPUTES the correction for unsigned codes; this rule DELETES the term at the writer. PASS with the family table above. `bit-plane-plane-sums-input-side-offset-dot` (B96) cross-referenced, distinct.
- LEAD-2: `hadamard|walsh|rotation.*quant|folded.*transform` → no kernel_opt rule hits (the FWHT ladder family B116/#29094/#29095 is the FFT-exchange mechanics — different layer: they transform WEIGHTS/blocks; this fuses the ACTIVATION transform into the quantize writer). PASS with the kinship note.
- LEAD-3: `tensor_ops|matmul2d|cooperative|simdgroup` → hits in 5 files, all CUDA/tile-staging rules (uzu B107, lily, etc.) — no Metal cooperative-tensor-MMA rule exists. PASS.
- LEAD-4: `memo|first.use|warm` in rust_perf corpus → no identical hoist rule (caching category = 7 rules; re-verify each at mine time). PASS provisionally.
- LEAD-5: measurement-honesty family — B177 `aa-control-prices-the-fidelity-instrument` is the COMPARATOR-side member; this is the SEAM-WARM side. Distinct. PASS.

## Yield

2–3 rules + 3–4 riders/annotations after the ~50% coordinator falsification class (LEAD-1 B+ and LEAD-2 B+ are the batch spine; LEAD-3 B + LEAD-4/5 B− riders).

## Honest caveats

- **The 13.23% and 256% figures are leaderboard-side (Yukon) and NOT verifiable from git** — no scores in-tree, per-submission gains unattributable from accept-commits alone. The verdict does not depend on them: the structural arc (mid-sequence 900–2,342-line submissions vs the ~150–225-line polish tail ending at b3050f0, dated 09-26) corroborates "large early frontier, saturated polish" independently.
- `official_scoring_enabled` was still false at the port-notes snapshot (§7 pending goldens); accepted submissions nonetheless keep landing — treat all score claims, including the user's, as unverified arena-side readings.
- Swift/Metal source — corpus fixtures stay ORIGINAL reproductions (Batch 107 uzu is the Swift-source precedent; never paste source verbatim).
- Unread at verdict: `EngineLoopV2*` internals beyond the diffs read, `AttentionV1.swift` (d5c6433's +247), the M5 `_nax` kernel family details, `quantized_utils.cpp` (+309 in 6acc822), the Tests/ tree — re-read at mine time per the standard clause.
- The reviewer-ping-pong corrections are incorporated: regime-conditional bit-identity (mantissa budget + negative fixture arm), the consumer-ISA condition, the B+ grade, the B180 cross-link, the two LEAD-3 instances, and the b3050f0 content annotation.

## Records

- Plan: `.plans/174_batch181_mlxfast_bonsai2_arena_distill.md` (this commit)
- Queue line: `.distill/001_mining_queue_snapshot.md` (this commit)
- Cross-ref: `.plans/173_batch180_qvac_bonsai_cuda_pt_layout_distill.md` (this commit)
- Clone `.raw/mlxfast-bonsai2-27b-engine` REMOVED at close (the ephemeral-clone law; T0 re-clones at the pin)
- Verdict gate: Claude reviewer ping-pong round 1 REVISE → corrections applied → round 2 AGREE (session `419201be`, 2026-09-26)

## Addendum (2026-09-26 ~18:2x +07) — the scores ARE public: mlx.fast verified, gem commits correlated

The landing-hour caveat "scores NOT verifiable from git (leaderboard-side)" is CORRECTED by the owner's pointer: the arena leaderboard is PUBLIC at **https://mlx.fast** (Yukon / Eigen Labs, the `bonsai2-27b-mlx-v1` challenge page). Verified readings this session:

- **Current record 278.8%** (polymorf, posted Sep 26 10:17 UTC, +22.82%) — Dec 212.9 tok/s, Pre 1732.8 tok/s. Score semantics per the page: composite speedup over the serial control = `prefill^0.25 · decode^0.75`, parity = 0%, i.e. 278.8% = ~3.79x composite.
- **The user-cited 256%** = rank 02 (DPZZxlz, posted Sep 26 7:16 UTC, +0.97%, Dec 202.9 / Pre 1607.3) — the frontier when this verdict's source commit window landed.
- **The drafter column is `stock DFlash` on EVERY promoted row**; the tok/round figure climbs 6.7 → 8.0 → 9.1 → 9.8 → 10.7 across the day. NO new head family appeared — the acceptance-length gain is ENGINE-side depth/acceptance tuning around the stock z-lab drafter. This confirms the lane-intel call (T-R1 not fed) and sharpens it: the arena's entire 33% → 278.8% arc (Sep 25 01:03 → Sep 26 10:17 UTC, under two days) is engine work on a frozen head.
- **Jump → commit correlation (timestamp-matched against the git accept/validate log)**:

| Leaderboard jump | Posted (UTC) | Git commit | Diff |
|---|---|---|---|
| +58.5% → 241.1% | Sep 26 1:43 AM | `4c8964f` Accept @ 01:43:32 | 1,976 ins — the Hadamard.swift +921 rewrite (Int8Activation/forwardInt8/forwardWithGroupSums + the tensor-route dispatch) |
| +29.29% → 170% | Sep 25 8:16 PM | `6acc822` Accept @ 20:16:01 | 2,342 ins — the largest Qwen35 rewrite (quantized_utils +309) |
| +22.33% → 125.8% | Sep 25 11:24 AM | `0f4dea5` Accept @ 11:24:44 | 527 ins |
| +37.91% → 103.5% | Sep 25 10:59 AM | `b0552f2` Accept @ 09:39 (nearest) | 898 ins |
| +22.82% → 278.8% (record) | Sep 26 10:17 AM | `f5960db` Validate @ 08:54 | 200 ins — the staged8 VERIFY route (below) |

- **The record's technique (f5960db, in-tree at the new HEAD `f5960db8`)**: the `staged8` VERIFY-width route — verify/decode kernels run `int8 x int8 -> int32` tensor-unit ops over threadgroup-staged int8 weight slices, reading the prompt route's SIGNED int8 activation codes verbatim ("SIGNED codes only" in the source), affine map in FP32 from per-group scale + scaled-sum. In-source numbers: the int8 tensor rate is ~2x the FP16 rate (116 vs 58 TF/s on M5 Max); staged8 beats the native 2-bit-operand form by 3% of the decode window and the prior record's verify kernels by 5%. Three-form dispatch `staged8 | native2b | none`, env-forced via `DARKBLOOM_BONSAI_TENSOR_ROUTE_VERIFY_FORM`. The staging buffers are REUSED as the reduction buffers ("16 KB of threadgroup memory in all, two threadgroups per core").
- **Verdict implication, stated plainly**: the 13.23%-flagged commit (b3050f0, the signed-codes plumbing) did not itself take the frontier — and it was not polish. It is the load-bearing ENABLER of the current record: signed int8 activation codes are what make the verify route's int8xint8 tensor ops clean (no offset term at the 2x-rate unit). LEAD-1's weight in the batch rises accordingly: the regime it unlocks (writer-controlled signed codes) is the same regime that lets a quantized matmul run at the tensor unit's int8 rate. The three-form dispatch (staged8/native2b/none) is also the answer shape our mine-time rule should carry: probe the ISA, pick the fastest legal operand form, keep the fallbacks.
- Note-holder hygiene: the .raw clone was re-pinned at `f5960db8` for this addendum's reads and removed again at close; the batch's T0 re-clones at the NEW head pin (f5960db8 — supersedes b3050f0f as the mine-time tree, since the record's code is in it).

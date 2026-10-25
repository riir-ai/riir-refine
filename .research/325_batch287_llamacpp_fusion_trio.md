# Batch 287 — the llama.cpp fusion trio (P81/P82/P83), five count-neutral riders

**Status:** RECORD — LANDED 2026-10-26 ~04:xx +07 (shikuwa/4090, riir-refine idle loop; the claim `e0b6376e3` + plan 292 discharged by this note; kernel_opt corpus STANDS 977 — zero own rules, five riders; every quote verified verbatim at the pins this session via the blob-filtered clone `.raw/b287-llamacpp` + the live PR-page reads).

## Sources (all MIT, ggml-org/llama.cpp)

- **P81** — `c49edf0963a76d5e78b2a7278c9bd5b71d26c84f` #29948 "CUDA: fuse FFN gate/up matmuls and GLU in MMQ with interleaved gate/up rows", merged 2026-10-25T09:24:38Z (anujj/NVIDIA; review JohannesGaessler, approved 10-24, merge-ready label).
- **P82** — PR #30489 "ggml-cuda: fuse GLU into the MMQ Q8_1 activation quantization", head `784fd01678cb91227c8de2fa1566308e7e3e2244` (single commit, 2026-10-14), OPEN at authoring (closed 10-18 by JohannesGaessler preferring #29948, REOPENED same day after the am17an push-back + the off-site discussion; awaiting ggerganov + 2 approvals; mergeable_state dirty). Verified by object-absence: `784fd016` NOT an ancestor of origin/master at the T0 read.
- **P83** — `a55c16e4ee532aa36d55f5756f3ba28a21a2b62b` #31025 "ggml-cuda: fuse qwen MRoPE + set-rows", merged 2026-10-25T09:15:45Z (am17an).

## What landed (five riders, all count-neutral)

1. **`fuse-gate-up-gemv-activation` (entries_gpu_01) += #29948** — the MMQ PREFILL instance of the interleave+GLU-write-back law (B251's yukon rider owns the GEMV/decode instance): `GGML_CUDA_MMQ_FUSION_ROWS = 8` chunk interleave (the `ggml_cuda_mmq_x_row` `(i / R) % 2 == 0 ? x : x_gate` arithmetic; chunks of 8 over 16 'also cover the one-tile-per-warp configs'), the C-tile pairing static_assert, the halved write-back loop with GLU applied from the two accumulators at the dst store; the admission faces — no-stream-k + `nrows_x % 128 == 0` static_asserts, the FWHT-hint refusal, Turing+ MMA-accumulator-layout dependence, SWIGLU_OAI default-params-only, the graph-topology-invariance clause ('Buffer- and batch-size independent, so graph_optimize produces the same graph topology for every ubatch size (see #29986)'), the alloc-dep keep-alive face (add_alloc_deps so the weights cannot alias the GLU output under op offload); measured GB10 +4.6..+10.7% / 5090 +1.9..+4.7% / Ada 4090 (our box class) +1.2..+1.3% pp; the per-op-vs-block honesty face (5090 Q4_K fused matmul +2% SLOWER, block −14%); the MUL_MAT_ID extension intel (+10.2% GB10 pp16K, coming) + the UD-quant complementarity (30/64 layers, both-together +6.3%).
2. **`fused-activation-quantize-for-gemv-consumer` (entries_gpu_01) += #30489** — the MMQ/Q8_1 CONSUMER instance of the activation-into-consumer's-quantize fold: 'one extra kernel launch plus a full write+read of [n_ff, n_expert_used, n_tokens] of F32 per FFN per ubatch' eliminated; the admission IS the decision content — (1) the bounded forward window (≤8 nodes ahead after stream reordering; every intervening node + its sources must be memory-disjoint from the output; `nodes_fused` marking; break-on-first-consumer), (2) the dispatch-match restriction ('only fuse where the plain dispatch would pick MMQ for this batch size'), (3) the shape restrictions (MUL_MAT_ID non-broadcast `glu->ne[1] == ids->ne[0]` per the PR body, scatter-path static_assert refusal, four-op allowlist, packed-GLU half-split, native-FP4 + padded-compute-buffer exclusions); measured 4090 box class +1.7% pp4096; bit-identical 52.4M-float census; the closed-reopened history + the anchor-priority complementarity ('They don't stack within the same FFN... covers routed MoE down projections (MUL_MAT_ID), which #29948 doesn't yet support').
3. **`fused-activation-quantize-for-gemv-consumer` += #31025** — the MRoPE/IMRoPE indexed-scatter face on the B111 rope→cache law: the admission widens ('Only norm/neox/multi shaders have the fusion code'), the destination becomes the row_indices scatter (`idst = i1 * s1 + i0 / 2; idst += row_indices[i2] * set_rows_stride;`, sentinel `set_rows_stride != 0`), the T/D template split = f32 compute cast-at-write to the f16 cache, passthrough rows ride the same scatter.
4. **`l2-grouped-tile-schedule` (entries_popcorn) += #29948** — the FUSED-PAIR operand-footprint ORDER law: column tiles innermost so concurrent blocks share the weight tile and only y stays L2 ('100 MB for Qwen3.8-27B Q4_K vs 24 MB of L2 on spark'); measured order swing GB10 pp2048 fused: Q4_K 9970 vs 14610 µs, Q8_0 9119 vs 21024 µs (2.3×), IQ4_XS 8388 vs 10468 µs (the anti-regime datum — smaller streamed operand, smaller swing); the launch-order-inherits-the-fusion clause (the fused path forces plain tiling and pays `GGML_ASSERT(nty <= 65535); // gridDim.y limit`); the order swap extends to the NON-fused tiling path (Pascal/AMD) with JG's 9-GPU flat-to-positive merge table.
5. **`streamk-starved-tile-grid-dispatch` (entries_gpu_33) += #29948** — the fusion-borrows-the-dispatch-rule admission face: a fused kernel that cannot use stream-K must refuse where the FUSED grid would have streamed ('The fused kernel needs one block per tile (no stream-k split), so only fuse if the tiling is efficient.'; `ggml_cuda_mmq_fusion_is_efficient` = the shared ≥90%-wave rule priced at the fused shape); the general law — a fusion that eliminates a dispatch mode re-prices the fused shape under the surviving mode's own rule, never fuse-then-starve; measured ub-dependence (5090 8B pp4096: gate-off at ub 128-512 +0.6..0.7%, fusion fires at ub 1024/2048 +4.9/+6.3%).

## Dedup (T1)

The enrichment's face split (2026-10-25 ~17:4x, a 3+-variant vocabulary grep) held at the current corpus head: the P81 core EXISTS at B251's rider (GEMV instance — this batch is the MMQ instance, extension not duplicate); the consumer-fold law EXISTS at `fused-activation-quantize-for-gemv-consumer` (the GEMV/int8 instance — #30489 is the MMQ/Q8_1 instance); the column-innermost order face and the fusion-borrows-the-dispatch-rule admission face were confirmed unowned (l2-grouped-tile-schedule owns group-WIDTH derivation, not order-for-fused-pairs; B127 owns the launch's own starvation decision, not a fusion borrowing it). ALL FIVE landings are riders — the batch is count-neutral by adjudication, not by scarcity.

## Gates

| gate | verdict |
|---|---|
| batch287 needle gate | 1/1 FIRST-RUN (27 marks across the 4 host rules) |
| kernel_opt lib (`--features kernel_opt`) | 3239/0 (3238 + this batch's test) |
| default lib | 2733/0 |
| clippy −D ×3 postures (--all-targets: default / all-features / no-default) | all Finished exit-0, zero real lints (the box's incremental-cache hardlink noise only) |
| retrieval floors (kernel+latent posture) | 22/22 HELD, no re-pin (~482 s) |
| freeze gen2/gen3 | NOT owed (P30 — zero clippy-corpus delta) |
| score-bench | NOT owed (kernel_opt-only delta) |
| release-set binary | NOT owed (kernel_opt outside the release set) |

## En-route repairs (both witness-class, both the pins doing their job)

1. **batch276_rules_and_riders_landed** — the B276 needle 'Batch 276 / Plan 280 / Research 314.' was DISPLACED by my rider append (I initially dropped the old closing stamp). Repair: the closing stamps RESTORED before the riders on all three stamped hosts (B276 on fuse-gate-up, B238 on l2-grouped, B127 on streamk-starved) — the house form is old-stamp-then-rider, the old needles hold as substrings.
2. **batch22_cudarc_kernels_concepts_covered** — pinned the ENTIRE `fused-activation-quantize-for-gemv-consumer` concept by EXACT equality (Vec::contains); my riders broke it. Repair: converted to the PREFIX assert form per the documented Batch-199 rider law (riders append to the concept tail) with the reason comment — the same conversion line 1293 already carried for the warp-shuffle rule.

## League intel carried

- #29948 + #31025 merged 2026-10-25 09:15/09:24Z — ~4h AFTER the b10822 bout pin `bf8c069` was cut (10-25 05:27Z): the fork's origin/master (re-fetched at the league-intel pass) carries NEITHER, so the queued b10822 bout measures the pre-#29948 opponent; the next scored re-pin carries them (SongXiaoXi's Ada cells are our box class: pp512 +1.3% / pp2048 +1.2% / pp4096 +1.3% — the pp hold narrows ~1-2 pt).
- #30489 if merged adds another ~1-2% opp-side pp (the 4090 +1.7% pp4096 cell) AND the routed-MoE down-projection coverage (#29948 lacks it; the MUL_MAT_ID extension is implemented upstream-side and coming per anujj).
- Watch-join: at #30489's merge, re-verify the rider's head pin `784fd016` against the merge SHA (the disclosure in-rule names this).

## Queue after

Walk #17 grades at/after 07:00 +07 today (this lane's claim, expiry 2026-10-27 ~04:3x; note slot RE-DERIVED to 326 at its landing — this batch took 325); the b10822 bout behind the sibling QRR cell (GPU 100% held all session — zero-GPU batch posture verified at claim + T0); B285 bbtree + B279 = the M3's; nnethercote ~11-01; #30489 watch-join on merge.

NOTHING INVENTED — every quote a live read at the pin this session (git show at c49edf0963/a55c16e4e/784fd0167 in `.raw/b287-llamacpp` + the two PR pages fetched in full); every gate a run.

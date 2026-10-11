# Research 114 — PrismML fork Metal quant-MV distill verdict (Batch 96 source)

**Status:** RECORD — Batch 96 landing (Plan 060); 4 rules + 2 riders.

## Verdict: kernel_opt A- (game-relevant quant-GEMV cluster; zero measured tables on the Metal side, measured rows on the HIP side)

Source: PrismML llama.cpp fork at `e311ed38f` (league opponent tip, tag `prism-b10660-e311ed3`), Apache-2.0, clone `E:/git/vendor/prismml-llama.cpp` (M3 twin: `../vendor/prismml-llama.cpp`). The league opponent-watch intake (riir-ai Issue 843 T5, riir-clippy `a749024`) root-caused the 1.4x prefill jump to this fork's native Metal PQ2_0 kernels; verification: ZERO `pq` mentions in `ggml/src/ggml-metal/` at the old pin `9ca265a` (the old league pin ran type-142 through the generic dequant fallback), while `e311ed38f` ships `kernel_mul_mv_pq2_0_f32` + `kernel_mul_mv_q2_0_f32` + `kernel_mul_mv_tq2_0_f32` (commit `73f0fed04`, 2026-08-25, "ggml-metal: PQ2_0 backend [UNVERIFIED - needs Apple HW]" — the league measurement IS the Apple-HW verification).

## What was read (all at mine time, 2026-09-02)

- `ggml/src/ggml-metal/kernels/mul_mv.metal` (3,360 lines): `kernel_mul_mv_q2_0_f32_impl` L434-512, `kernel_mul_mv_pq2_0_f32_impl` L514-592, the shared `block_q_n_dot_y` helpers (q2_0 L33-78, pq2_0 L82-127, identical bodies), `kernel_mul_mv_tq2_0_f32_impl` L3089-3196 (the telescoped-digit kernel, cites `https://github.com/ggml-org/llama.cpp/pull/26980` in-source), the `mul_mv_ext` template instantiations L990-998 (q2_0 block 64 + `dequantize_q2_0_t4`; pq2_0 block 128 + `dequantize_pq2_0_t4`).
- `ggml/src/ggml-metal/kernels/quantize.metal` (477 lines): the set_rows family — `kernel_set_rows_q` L310-336, `kernel_set_rows_q32` L341-367, `kernel_set_rows_f` L372-399 (ONE threadgroup per row), `kernel_set_rows_f32_wide` L413-439 (the wide-row tiled kernel + in-source regime comment), and the tq2_0 set_rows instantiations L473-476 (KV rows quantized INTO `block_tq2_0` on write).
- `ggml/src/ggml-metal/kernels/misc.metal`: `kernel_fwht<N, src_t>` L378-426 (register-resident FWHT, instantiations N=64..2048 f32/f16 L432-449).
- `ggml/src/ggml-metal/kernels/quantize.h`: `quantize_q2_0` L22-40, `quantize_pq2_0` L43-60 (identical codec, larger block), `quantize_tq2_0` L257-282 (amax scale; `round(x*id)+1` -> {0,1,2}; `(xi & 3) << (2*n)` packing).
- `ggml/src/ggml-common.h`: `block_q2_0` (QK2_0=64, 2 bits/element), `block_pq2_0` (group 128 = 8 sub-blocks x 16), `block_tq2_0` (QK_K/4 bytes, 2.0625 bpw), `block_tq1_0` (1.6875 bpw, 5 trits/byte + high plane).

## The four mechanisms (rule-mapped)

1. **Telescoped digit coefficients** (`mul_mv.metal` L3147-3162): the packed byte is scored as `float v = (float)qs[j]` with digit oracles `f0=v; f1=floor(v*0.25f); f2=floor(v*0.0625); f3=floor(v*0.015625)` (in-source comments: "express the 2-bit field shifts (v>>2, v>>4, v>>6) as float floor ops"). The y-side coefficients `coef[0]=y0; coef[1]=y1-4y0; coef[2]=y2-4y1; coef[3]=y3-4y2` telescope `sum_k digit_k * y_k` into `sum_k f_k * coef_k`, and the ternary offset rides as `float sum = -sumy` seed. Coefficients + sumy computed ONCE per (thread, byte-column, half-block) and shared across ALL `nr0` weight rows (L3141-3155 hoist, L3157-3165 row loop). EXACTNESS CONTRACT: digit extraction is exact (v <= 255 < 2^24 integer-valued f32; multiplication by 4^-k is a power-of-two scaling, exact; floor of an exact integer is exact) — the PRODUCTS/sums are NOT bit-identical to the unpacked kernel (reassociation) => tolerance gate, never bit-identity.
2. **Positional plane sums + input-side offset** (`mul_mv.metal` L33-78): `return qb_curr->d * (acc_lo + 2.0f * acc_hi - sumy);` — the two bit-planes of the 2-bit codec score as weighted plane sums (the `2.0f*` folds the positional digit weight into ONE end-of-block multiply), and the `(w-1)` codec offset is paid from the INPUT side: `sumy` is the sum of the 16-float y-slice (computed once per slice in the ib-loop, L482-486), shared across all weight rows — the offset plane costs ZERO per-row work and needs no per-block zero-point in the format.
3. **Wide-row copy tiling** (`quantize.metal` L405-439): the in-source comment is the whole rule: "the generic kernel above assigns a single threadgroup per row, which cannot saturate the memory bandwidth for rows of 100k+ elements. tile each row across threadgroups (tgpig.x) and copy with float4 accesses. dims 2 and 3 are folded into tgpig.z (split via args.ne02)". Consumer named in-source: "wide f32 rows (e.g. recurrent-state snapshots)" — the GDN/linear-attention state write-back lane.
4. **FWHT shuffle-xor butterfly** (`misc.metal` L378-426): `simd_shuffle_xor(val, i)` butterflies for the first log2(32) stages (the xor mask IS the butterfly partner — no shuffle-index arithmetic; sign from `(lane & i) == 0 ? val2 + val : val2 - val`), in-register stages `i = NW..N` per lane afterwards, `const float scale = 1.0f / sqrt((float) N)` folded into the initial load, zero smem + zero barriers, N in {64..2048}.

## Riders (dispositions, no corpus edit)

- **B20/B92 byte-perm family — external measured confirmation on a THIRD substrate.** Fork commit `a7cc29645` ("hip: faster Q1_0, Q2_0 and PQ2_0 decode on AMD", 2026-08-27): hardware byte permute `v_perm_b32` for the Q1_0 sign unpack and the Q2_0/PQ2_0 symbol expansion "where `__byte_perm` is a software routine". MEASURED (source commit message; bit-exact gates claimed: test-backend-ops MUL_MAT/MUL_MAT_ID unchanged, greedy generations match CPU, KL 0 mean, 99.99+% top-token agreement on 8B/27B): MI300X decode 4B Q1_0 112 -> 322 tok/s, 8B Q1_0 81 -> 269, 27B Q1_0 31.5 -> 97, 8B PQ2_0 113 -> 259, 27B PQ2_0 48.6 -> 91.7. "CUDA and MUSA are unchanged (HIP device-compile guards); other quant types are unchanged (type-scoped rule)."
- **CDNA-vs-RDNA geometry regime** (same commit): "keep the default split-K MMVQ geometry for these types on CDNA, where the small_k geometry runs several times slower per call" — the regime-annotation class (geometry choice is arch-class-dependent, never universal).
- **PQ2_0 = group-128 Q2_0** (8 sub-blocks x 16 weights, `QK_PQ2_0`): identical dot body, half the scale loads per weight. Codec-design note only — a FORMAT decision with no bounded consumer fix-space (same kill class as the docker version-pinning rules).

## Dropped / out-of-scope (with reasons)

- GDN ring decode (`bed6e9a6b` + `08d8c9035`): already distilled — the batch-95 mlx-lm GDN lane's external confirmation (per the queue note; re-verified not re-mined).
- `im2col` kernel/dispatch mismatch fix (`e8b6452b1`): correctness fix for a dispatch-table mismatch, not decision content; our corpus covers dispatch-table discipline via `hardcoded-fast-path-jit-specialization` + `batch-width-routed-dual-tile-twins`.
- Hadamard weight-fold RUNTIME plumbing (`e0c828b90`, fwht.cu/.comp/misc.metal + conversion + model-loader): the fold-into-weights DECISION is codec/training-side (riir-train lane); the fwht KERNEL yields R4. The `out_proj` grouped-order guard bug in that commit (per-tensor vs tensor-kind matching) is a conversion-tool hazard outside kernel scope.
- HIP Q1_0 sign-unpack internals beyond the v_perm_b32 confirmation: same mechanism family as the rider.
- `kernel_mul_mv_id_*` (MMQ id-variant instantiations): plumbing instantiations of the mined kernels, no new mechanism.
- rs-rollback example additions (`e8b6452b1`): out of domain (host tooling).

## Provenance chain

- Fork: PrismML/llama.cpp `9ca265a -> e311ed38f` (league pins; Issue 843 T5 opponent-watch intake `a749024`).
- Upstream lineage cited in-source: ggml-org/llama.cpp PR 26980 (the telescoped-digit trick's origin — the fork transports it; our rule records the lineage per the spec-seeded, search-realized provenance discipline).
- League context: riir-ai Issue 843 (prefill campaign post-jump), Issue 844 (flash-vs-GEMM long-P prize), Bench 839-843; the opponent re-pin `e311ed38f` was verified FNV bit-identical at the T5 re-pin bout (riir-ai Issue 770 record).

## What would change the verdict

- Upstream llama.cpp merging the PQ2_0/TQ2_0 kernels would make the fork-specific pins stale but GROW the rule's firing population (mainline consumers) — verdict strengthens.
- A measured counter-example where the telescoped form LOSES at production dims would demote R1 to annotated (the fork ships no ablation; the +1.4x league jump is end-to-end attribution, never cite it as the kernel's isolated effect).

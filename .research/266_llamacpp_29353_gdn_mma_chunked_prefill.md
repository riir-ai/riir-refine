# Research 266 — llama.cpp #29353 "cuda, hip: GDN chunked kernel": the merged GDN-MMA chunked prefill (the watch-971 merge event)

**Status:** DISTILLED — pending owner decision (the corpus conversion is Batch 234 / Plan 236; the league defense dispatch is ai-971's Plan-605-T5 lane)

**Event:** ggml-org/llama.cpp PR #29353 MERGED 2026-10-14T14:00:59Z (merge commit `62df1815`, by am17an — the author-merge; head `dde6a4d93`; 7 files +564/−14, MIT). This is the PR watched across ~25 ai-971 cycles (born from the #26001 thread; am17an's own authorship of the fused design discussed there).

## The kernel in one line

`gdn_single<C=16, V>` — ONE persistent kernel per layer-pass: 8 warps × 32 lanes, one block owns V value columns of one (seq, head) and walks the whole sequence in C=16 chunks; all five products are `mma.sync` tensor-core ops on double-BF16 operands; the recurrent state is FP32 in per-warp register tiles loaded once at kernel start and stored once at end.

## The six mechanisms (Batch 234's rule decomposition)

1. **Register-resident scan state + one kernel per pass** (G): state in registers whole-sequence; global memory at the pass boundaries only. Versus the incumbent recurrent path (per-token recurrence) AND the 3-stage pipeline shape (global intermediates between stages) — the state never leaves the block.
2. **Double-BF16 residual-pair products** (N): every operand stored as `hi = bf16(x)`, `lo = bf16(x − hi)`; the product issues THREE mma per 16-k step — `(lo,bh), (hi,bl), (hi,bh)`, dropping the lo·lo second-order term. This is the near-F32 numerics that bought the reviewer sign-off ("keeps accuracy high by approximating F32 via 2 BF16s" — ORippler). Their instruments: KL mean 4.63e-4, top-1 98.82%, PPL delta +0.0045 (within reported uncertainty).
3. **In-register (I+L)⁻¹ via warp backward substitution** (G): the unit-lower-triangular inverse built one row per warp — right-to-left pivot elimination with `__shfl_sync` (FLA's solve_tril 16×16 scalar regime, ONE regime, in-kernel; no global round-trip for the solve stage).
4. **Occupancy-keyed value-split ladder + benchmark-gated admission** (D): the path ladder is SM-OCCUPANCY-keyed, not shape-keyed — "value splitting pays off when whole heads leave at least half the SMs idle" (`H*n_seqs*2 <= nsm` → V=32 value-split, grid ×4; a one-wave fit `*4 <= nsm` → the `__launch_bounds__(256,1)` register-budget arm; else whole-head V=128). The CC admission is benchmark-gated with the in-code law "MMA support alone does not imply a speedup" — CC∈{sm_86, sm_89, Blackwell, GB10} only; sm_80/Orin/Hopper EXCLUDED despite having the MMA capability. Admission is a runtime predicate with incumbent fallback (any refusal → the recurrent kernel), never a build flag.
5. **Union smem scratch** (A): `union { matrix<D,V> state; matrix<C,V> solved; } scratch` — the state snapshot (read by the projections) and the solved matrix (written by the solve stage) are lifecycle-disjoint and share one ~36-40KB opt-in allocation (`cudaFuncSetAttribute`, per-device `once_flag` init that checks `smpbo`).
6. **Warp-split phase overlap + cross-chunk prefetch** (E): in the V=32 arm, warps 0-3 run the Gram/inverse while warps 4-7 run the state projections CONCURRENTLY (`project()` hoisted); `PREFETCH = C==16 && V==32` carries the first eight input rows of the next chunk across the barrier ("Carry the first eight input rows across chunks to hide global-load latency").

## The integration shape (riders)

- `mma.cuh`: `load_ldmatrix[_trans]_swizzled` gain I==8 `x2.b16` variants (I==16 keeps `.x4`) + the `j = ((threadIdx.x / t.I) * (t.J / 2)) % t.J` wrap fix — the loader-width family face (load exactly what the tile needs; kinship `ldmatrix-pair-x2-to-x4-adjacency` merges x2s into x4 in the OPPOSITE direction).
- `ggml.h` (ggerganov's dictated declaration): "when n_tokens > K only slot 0 is guaranteed; backends may leave slots 1..K-1 unwritten" — an API UB-contract formalization (the two overflow test rows REMOVED as its consequence: the contract supersedes the tests). B224's UB-contract rider family.
- The eligible predicate: `g->ne[0] == 1 && K < n_tokens && S_v == 128 && q->ne[0] == 128` + `H_k == 16` exactly + `H ∈ {16,32,48,64}` + `n_tokens ≥ 64` — the K==n_tokens refusal is the spec-dec prefill gap (the DanoPTT/am17an exchange, recorded ai-971 09-24).

## Perf record (source-reported, never adoption bars)

RTX 4090 vs master `296538ade` (which includes #30087): Qwen3.8-27B-Q4_K_M pp2048 +3.1% / pp4096 +3.2%; the +10.8-20.6% rows are 35B-A3B MoE (not our class). The ai-971 12:5xZ re-price stands: the OLD +13.31/13.35% rows were measured against the pre-#30087 slow-scan master ("CUDA's GDN scan kernel is pretty bad, about half as fast as Vulkan's, which blows up the relative improvement" — 0cc4m 10-09); #30087's implied prefill uplift ≈ +9.8-9.9% on that cell. The league's own bout at the fork's absorption is the measured bar (the fork's prism-b10776 delta carries ZERO upstream ggml-cuda — the absorption is still pending).

## League context

- The threat scenarios stand as re-priced (ai-971 12:5xZ): fork absorbs #30087-only implied → pp2048 1.071× FAIL; both → 1.038× FAIL; #29353-alone → 1.141× GREEN. The defense (ours-side chunked-GDN MMA prefill into the `gdn_chunked` Arm-12 slot) is dispatched (Plan-605-T5) — both insurance and the standing prefill-push lane.
- Our port seam: `crates/riir-gpu/src/cudarc_kernels/prefill_cuda_gdn_chunked.rs` (the knob-gated CS=64 4-scalar-kernel path, ZERO tensor-core ops) + the `gdn_chunked` slot in `prefill_cuda_full.rs` (Arm-12 A/B knob + knob-generation graph-replay invalidation + buffer growth). The algorithm family (WY: gram → triangular inverse → chunk sequence) is the SAME; the deltas are the six mechanisms above.
- Numerics contract for our port: NOT bit-identical by construction (different reduction order); gate on argmax-stability (the Issue-615 G1 class + the league warm-pin FNV/argmax pins), matching the PR's own near-lossless evidence.

## Provenance

- The .diff read in FULL at the merged head this session (the 442-line kernel + header + integration + mma.cuh rider + tests; quotes verbatim).
- ai-971's three Defense-design-intel sections (10-09 lithos, 10-11 FLA, 10-13 the #29353 read at `377e111e`) are the lineage; the merged shape confirms the 14:3x read with the #29612 swizzle adaptation landed (`load_ldmatrix_*_swizzled`) and the dictated ggml.h clause verbatim.

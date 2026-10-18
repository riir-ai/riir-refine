# Research 286 — Batch 250 slice 2: the envelope pool completes (6 own-rules + the 4 platform watches)

**Status:** RECORD — slice 2 of Batch 250 LANDED (kernel_opt corpus 879 → 885 (+6, all B); the pool FULLY CONVERTED at the claim's pins; T6 x86 Pareto disclosed pending); plan 254 T11-T12 discharged.

## Provenance

Claim `b52a4874` (zed-m3-idle/M3, refreshed 2026-10-18T14:5xZ to expiry 2026-10-20T00:30Z — the measured need: the combo-posture gate ran 3.7 h at this box's load). Sources read at the LOCAL clones: llama.cpp @ 5ecbe1ac (commits verified present: e707790c 2026-10-15, f74e2fa6 10-15, 623c1639 10-16, 13de11e7 10-16, 2744e48d 10-17, f2f667b0 10-17) and the uzu vendor twin @ 115609b6 (tag 0.5.23; 597e5e50 10-16 present). Every quote verbatim at the pins; the pseudocode halves parse AND typecheck (rustc --emit=metadata over all 10 extracted blocks pre-install — the staging lesson applied BEFORE the gate this time).

## What landed (all category B, Compute 94 → 100)

1. **`moe-weights-load-time-soa-accumulation-order-pinned`** (e707790c #29337 + f74e2fa6 #30374): grouped Q8_0 MoE weights → per-expert SoA at load time ([qs][d] per slice, one temp-buffer pass); the accumulation order pinned to the non-reordered kernel ("Keep the accumulation order of the non-reordered MoE kernel." — the vdr param + the manual xor ladder); THE HAZARD FACE as the pairing commit — the dequant readers must be rewritten FOR the new contiguity ("one thread per qs byte", "grid-stride loop because the launcher may shrink the work-group to keep the global range below INT_MAX", "float math, not dequantize_q4_0_reorder"), the graceful-skip log line carried.
2. **`rms-norm-registers-carry-x-between-passes`** (623c1639 #29720): float4 loads into a per-thread `xs[vpt]` register array, carried across the block reduction; vectorization admitted per operand (`vec = vec && ggml_cuda_is_aligned(x_t, 16);` + the packed-width conjuncts); the launch ladder (32/128/256/512/1024 vpt 1, 4096 vpt 4, else scalar).
3. **`kquant-dequant-work-item-geometry-wide-stores`** (13de11e7 #29696): `SYCL_DEQK_WI_PER_BLOCK = 32` — 4 qs bytes per work-item, two vec4 stores; the shared-memory scale staging + its barrier dissolved (direct `get_scale_min_k4` per work-item); flat grid over blocks × WI.
4. **`mmvq-low-bit-vdr-two-rows-share-q8-loads`** (2744e48d #30517): VDR 1 → 2 for q2_K/q3_K (+q6_K rework), array-packed v/vl/vh, interleaved u loads (`u[VDR*i + j]`), packed-scale reads (`scales[i/2] >> (16*(i%2))`), the per-type geometry tables (`mmvq_two_rows_ncols1` for the starved types; the GCN nwarps arms; the type-keyed prefetch distance).
5. **`moe-group-selection-fused-into-topk`** (f2f667b0 #29181): the 14-op group-mask subgraph fused into the topk kernel — warp-local top-2 group scoring (`top2 = fmaxf(top2, fminf(top1, vals[i]))` + the xor fold), rank with lower-index ties, `__ballot_sync` keep-mask; the WHOLE-PATTERN admission (`topk_moe_group_pattern[]`) with named refusals ("kept groups must hold all used experts", `n_groups > WARP_SIZE`, the bias arm).
6. **`attention-row-mask-bitmap-per-row-not-per-block`** (597e5e50 #971, uzu): ONE packed ulong per query row (trie node bits OR the causal bound as the payload — `row_mask[row] = is_trie ? 0ul : ulong(prefix_length + query_positions[row]);`), the bit-test read path (`key < prefix_length || ((row_mask[lane_row] >> ulong(key - prefix_length)) & 1ul) != 0ul`), the unified single-barrier staging, and the head_dim-derived geometry (`query_rows_per_threadgroup(head_dim) = SIMDGROUPS_PER_THREADGROUP / (head_dim / SLICE_COLS) * TILE_ROWS` replacing the misnamed `block_rows`, the HEAD_DIM 128 variant joining 256).

## The 4 platform watches (disclosed, no corpus rows — no adoptable Rust-side host)

- `9d1878a4` HIP `__builtin_amdgcn_perm` q1_0 unpack (HIP-only additive; same files as the MMVQ candidate — carried as its watch note).
- `797de63b` hexagon Q1_0/Q2_0 (809 lines, all ggml-hexagon/, the platform-low class).
- `2f2763d` opencl MxFP4 MoE Adreno (minifloat-adjacent vs B194/B207 — decode arithmetic owned; the platform half is Adreno-only).
- `27c7a76` hexagon softplus HVX (platform low).

## T6 — the x86 Pareto verdict (the slice-1 deferred leg)

Still **BENCH-VERDICT-PENDING-x86**: the M3 is aarch64 (the fixture compiles empty here BY CONSTRUCTION); the 4090 lane's landing-gate runner (`fbab2963`) carries the x86 leg for the 4090 box, coordinated (M3 composes, 4090 measures). The in-rule annotation + this note disclose the posture; the verdict lands in the runner's window either way.

## En-route (disclosed)

1. **The multiline-literal slips** — the rules 5+6 ref/opt strings were first written as real multi-line literals (illegal in Rust) and the first apostrophe edit produced an unescaped inner quote; the final normalization pass ran a Rust-aware string scanner over every ref_shape/opt_pattern/gpu_concept literal (real newlines → `\n`, inner quotes escaped) and a full rustc parse+typecheck pass over all 10 extracted blocks BEFORE the gate — the staging-pass lesson (catch it pre-gate, not in the gate) applied.
2. **The needle trailing-period drift** — the gate's MMVQ needle carried a trailing period the concept quote (quoting the source comment's inner text) doesn't; the needle (not the quote) fixed to match the pin.
3. **The count-pin convention held** — every historical batch gate's asserted value moved 879 → 885 together (the file's own convention: the VALUE tracks live, the MESSAGE records history); the B cell 94 → 100.

## Gates

- `batch250_slice2_rules_landed` **first-run PASS** (6 rules × needles + parse integrity ×12, corpus 885, Compute 100).
- kernel_opt,latent_retrieval lib FULL run: **3347 passed / 0 failed** (this slice's landing gate).
- Default lib: re-asserted (the kernel_opt delta is feature-gated; default corpus untouched by slice 2).
- Retrieval floors: **HELD, no re-pin** (the +6 rows absorbed; the floors stand at 124/130/132 of 141 blended).
- Freeze/score-bench NOT owed (zero clippy_lints corpus delta; zero rust_perf delta this slice — score-bench #189 from slice 1 stands as the batch's score record).

## Queue

Batch 250 COMPLETE (slices 1+2; the envelope pool converted: 879 + 885 pins, 7 own-rules + 1 rust_perf rule + 2 riders + 4 watches + the T6 disclosure). Next: the 4090 lane's x86 Pareto leg (T6's verdict); the b10777 league re-pin queue.

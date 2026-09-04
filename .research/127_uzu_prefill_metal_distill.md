# Research 127 — uzu (trymirai) prefill Metal distill — verdict

**Status:** DISTILLED — pending owner decision

- **Source:** https://github.com/trymirai/uzu — "A high-performance inference engine for AI models" (Apple Silicon, Rust host + MSL kernels, SSM/GDN/Mamba-2 + MoE + attention model families).
- **Provenance:** clone pinned `@ 115609b6ef7ce5a94fc761bf395780f032b09f9b` (Release 0.5.23, committed 2026-09-03 23:12 UTC), **MIT**. Clone lived at `.raw/uzu` (ephemeral, removed after verdict per §0.5; re-clone at the pinned sha at mine time). Inspection 2026-09-05, M3 box, read-only.
- **Method:** 2 disjoint subagent lanes (A: `matmul/**` GEMM stack incl. MXU/schedules/split-K/RHT/qmv; B: `attention/**` + `moe/**` + `gdn/{prefill,chunked}` + `ssm/ssd_prefill` + `radix_top_k_small`) + coordinator verification of 16 load-bearing quotes (all byte-accurate) + dual-vocabulary dedup gate against the corpus.
- **User aim:** prefill (prompt-processing) throughput.

## Verdict

**YES, grade A−. Route: `kernel_opt`.** League-opponent-class Metal source in the B95–B104 lineage (mlx-lm / MTPLX / MLX / llama.cpp fa-vec precedents) — but a *competitor engine* rather than an opponent fork. The tempting wrong domain is `rust_perf` (the host-side policy/selection/route code is Rust) — wrong: every mechanism there is GPU dispatch policy, and the kernel-side surface dominates. Yield estimate: **4–5 rules** after ~50% coordinator falsification + ~10 riders.

Engine scale: `crates/uzu-engine` ~22k lines across backends `{metal, cpu, common}`; kernels for matmul (GEMM/GEMV/QMV), attention (5 kernel families + route ladder), MoE (two-pass prefill/decode), GDN (plain + chunked + tree-verify), Mamba-2 SSD, sampling, plus `weaver/` (spec-decode frontier) and `speculators/` (unread). In-tree pinned numeric artifacts are policy-table boundaries + parity tolerances only — **no committed perf numbers anywhere** (criterion benches + `#[ignore]` perf tests print medians, assert nothing).

## Context map (prefill)

Dispatch is a 3-level host pipeline: `MatmulMetalKernel::choose_dispatch` (GEMV vs GEMM, incl. the measured QMV route table) → `GemmProblem::select_plan` (engine Mxu/Simdgroup + tiling + split_k) → `GemmSpecialization::from_plan` (pipeline-cache key carrying 3 tuning booleans into the Metal template). One templated `KERNEL(Gemm)`; three *schedules* (dense/integer/staged) select the inner loop by operand quantization class — **"staged" is dequant-to-threadgroup staging, not software pipelining** (`static_assert(!Core::Left::QUANTIZED && Core::Right::QUANTIZED, "staged schedule requires dense A and quantized W")`). The MXU path is Apple's **cooperative-vector** MMA (`mpp::tensor_ops::matmul2d`, 16×16 fragments, `MetalPerformancePrimitives`), probed via a private obfstr'd `supportsMXU` selector — "True on M5+ (Gen18+), false on M1-M4", default **false** (fail-closed to simdgroup).

## Ranked candidates (new-rule material)

### 1. `indirect-device-dispatch-tile-map-moe` — MoeDispatchBackward — **A− lead**
The entire MoE prefill grid is computed ON-GPU: histogram → single-TG scan → tile map → a kernel writes `MTLDispatchThreadgroupsIndirectArguments`; the expert GEMMs launch `GROUPS(INDIRECT)` and read their (expert, segment-start, row-offset) assignment from a 3-word tile-map row. Variable tokens-per-expert density never round-trips to the host.
- `moe/experts_two_pass_prefill.metal:45-51`: `const uint n_tile_idx GROUPS(INDIRECT),` … `const uint base = row_tile_idx * 3u;` / `const uint expert_idx = tile_map[base + 0u];`
- `moe/tiles_map.metal:95-96`: `// Write MTLDispatchThreadgroupsIndirectArguments {x, y, z} where:` / `x = num_tiles_n (computed on CPU and passed in), y = total_tiles, z = 1`
- **Decision content:** applies when the M-axis population is device-dependent (ragged expert segments); each TG early-returns on `expert_idx >= e` / `seg_len == 0` so over-provisioning is free. The N-axis (`num_tiles_n`) stays host-static — only the ragged axis is indirect. Empty experts cost one dead TG row, no launch re-shape.
- **Dedup:** `derive-params-from-global-invocation-id` (B26) encodes per-item params in a HOST-static grid; nothing computes the grid SIZE on device. Zero corpus hits for `MTLDispatchThreadgroupsIndirect`/indirect dispatch. NOVEL — kills the per-layer GPU→CPU sync (or blind over-provisioned host launch) that ragged MoE otherwise forces. Magnitude story: one sync per MoE layer per prefill step removed.
- **GOAT candidate** (strongest single): novel mechanism, regime-bound, quote-proven, deterministic, concept-grade per kernel_opt house norm.

### 2. `cooperative-tensor-mma-capability-route` — KernelPolymorphism — B+
The MPP cooperative-vector protocol + fail-closed capability route. 16×16 fragments; MPP pairing constraint (even-N output fragments, or N==1 with even M) resolved by pairing two column fragments per op; `MatmulMode::multiply` (first K step) vs `multiply_accumulate` (rest) with the measured note "MXU relaxed multiply is slightly faster than multiply_accumulate for pure matmul"; `RELAXED=true` layout everywhere, strict kept compiled-out with "it currently performs about the same as simdgroup"; FP dense path runs **zero shared memory** (`GEMM_TGA_ELEMENTS` → fragments load straight from global). Host refuses MXU plans when the probe is false (`supports_mxu` = `optional_selector_value(self, obfstr!("supportsMXU"), false)` — default false).
- Measured refutation instance in-tree: `gdn/chunked/gram.metal:16` `// MXU did not improve e2e vs simdgroup.` (16×32 Gram tiles stay simdgroup).
- **Dedup:** zero corpus hits for mxu/cooperative-tensor/matmul2d (B79 plane-specialization is CubeCL role-split; B103's NAX is MLX's per-core neural accelerator — related next-silicon posture, different API shape). M5+/Gen18+ hardware-unavailable on our pinned league boxes — same posture as the B103 NAX batch (capability rules mined despite M3 unavailability). Category: KernelPolymorphism.

### 3. `packed-nibble-tensor-operand-alignment-gated` — DtypeAlignment — B
W4 weights feed the MXU tensor unit **without unpacking** when the N extent is aligned: the quant cursor's `load` returns a `DeviceTensorOperand` (raw packed pointer + byte stride) wrapped as `tensor<device Element, extents<...>, tensor_inline>` over `int4b_format` — the cooperative MMA consumes packed nibbles natively (zero dequant, zero shared memory). Unaligned → fallback cursor software-unpacks per element. Int8 always direct.
- `quantized/cursor.h:127-129`: `if constexpr (Format::BITS == 4 && ALIGNED) { return Cursor<Format, Fragment, true, HOIST_OPERAND_ADDRESSING>{current, row_stride_bytes};`
- **Decision content:** alignment (N-extent) decides between the dequant-free tensor path and the unpack path — a two-regime placement decision on a novel substrate.

### 4. `rht-epilogue-fusion-contract` — Fusion — B+
Output-side random-Hadamard transform as a GEMM epilogue, with the ORDERING contract as the decision content:
- fused **only when `split_k == 1`** — under split-K the RHT moves to a separate in-place kernel after the reduce (mixing must happen before the Hadamard);
- **bias never fuses with RHT** in the epilogue: `kernel.rs:364-366` `(None, output_bias, output_transform.difference(GemmDTransform::BIAS))` — bias is demoted to a separate post-RHT kernel (the Hadamard of a constant column vector is not that vector — order matters);
- fused output-RHT factors are legal only with **quantized weights** (`linear/matmul.rs` "fused output-hadamard factors require quantized weights"); the FP analog is INPUT-side RHT applied to activations before the GEMM — and the A8 path folds the input-side RHT INTO the quantize kernel (`prepare.encode_quantize(..., &data.rht_factors, ...)`).
- Epilogue mechanics: per-row × per-32-wide-stripe work, each stripe a 32-lane `simd_shuffle_xor` butterfly with the ±1 diagonal factor applied after the transform (block size == `METAL_SIMD_SIZE` statically asserted) — the GEMM-tile sibling of B98's GEMV exchange network.
- **Dedup:** `fwht-shuffle-xor-butterfly-register-resident` (B98) owns the exchange network itself (GEMV/transform side, recursive-kronecker-cat construction); this owns WHERE/WHEN the transform may fuse into a GEMM epilogue and the demotion order. Composes, does not duplicate.

### 5. `target-tile-split-k-deterministic-two-dispatch` — Dispatch — B
Split-K factor = `clamp(target_tiles / base_tiles, 1, k/align)` with a quant-class-keyed target-tile table (`SPLIT_K_TARGET_TILES_FP 512 / _A8 256 / _A8_TILE16X32 4×256 / _A8_TILE32_W4 512 / _A8_TILE32_W8 1024`); a **quant-group alignment ladder** (`max(outer_block_k, group_size, ACTIVATION_SCALE_GROUP_SIZE=128, 2·group_size for W4+ScaleZeroPoint)`) so every partition slice lands on complete quant groups; A8+MXU clamp `k/256`; final decrement loop guarantees `k % (split_k·align) == 0`. Legality: `m·n % 4 == 0` (vec4 reduce).
- Execution is **two dispatches, not atomics**: partial kernel writes per-partition M×N slabs to scratch with output transforms STRIPPED (`ab_scale: 1.0`), then `GemmSplitKReduce` (256 threads, float4) accumulates partitions **in fixed ascending order** into an f32 accumulator with SCALE→ACCUMULATE→BIAS transforms folded into the reduce (specialized per `GemmDTransform` mask, cached).
- `selection.rs:120-121`: `let target_tiles = policy::split_k_target_tiles(!shape.a_full_precision, tiling, shape.b_bits);` / `let mut split_k = (target_tiles / base_tiles).max(1).min((shape.k / align).max(1));`
- **Dedup:** `stripe-sms-l2-semaphore-reduce` (B92) = ONE kernel + cross-block counting semaphore (runtime conflict ordering). uzu = TWO kernels + deterministic fixed-order accumulate, zero atomics. Two-regime complement: semaphore-merge vs deterministic-partials+reduce (also carries the transform-deferral placement B92 lacks). Pinned split_k expectations exist in `selection_test.rs` ((16,4096,4096)→8, (16,1024,4096)→16, (16,34816,4096)→1).

### 6. `a8-activation-route-stability-guard` — DtypeAlignment/Dispatch — B
The host answers `a8_activation_plan` (legality: MXU + BF16 in/out + quantized signed transposed-contiguous W (4/8-bit, group 32/64/128) + `k` multiple of 128 AND the weight group; asymmetric W additionally requires precomputed activation group sums at `min(weight_group, 128)` for the zero-point correction term) — then `select_activation_format` **re-runs the full dispatch with `a_full_precision: false`** and adopts Int8 only if the A8 shape still routes to the MXU GEMM tier (`match self.select_dispatch(&a8_shape, context) { MatmulDispatch::Gemm(plan) if plan.engine == Mxu => ActivationFormat::Int8, _ => Bf16 }`).
- **Decision content:** adopt an operand-class downgrade only if the optimized tier still admits it — a route-stability guard that prevents "int8 activations" silently landing on a slower engine/tile tier. Novel host-side pattern; zero corpus hits.

### 7. `lane-sequenced-deterministic-bucket-scatter` — MoeDispatchBackward — B
Atomic-FREE deterministic expert bucketing (token permute): per-simdgroup threadgroup histograms → prefix bases → a 32-step lockstep write phase ordered across the whole threadgroup (`for (uint step = 0; step < simdgroup_size; ++step) { if (simd_lane_id == step) { ... } threadgroup_barrier(...); }`), so no device atomics and a deterministic layout; optional per-expert capacity clamp drops overflow (`tok2row = -1`, congestion mode, off by default).
- `moe/scatter_buckets.metal:93`: `// Phase 3: deterministic write with 32-lane sequencing, no device atomics`
- **Decision content:** deterministic layout WITHOUT atomics and WITHOUT offline preprocessing, priced at 32 barriers/tile amortized over the d_ff×d_model GEMM that follows (prefill-only economics; decode uses a direct `row_expert_map` GEMV path instead).
- **Dedup:** `expert-major-work-map-token-scatter-clamped` (B97) = runtime fetch-add claims, order deliberately unspecified; B102 LPT = offline balance with disjoint ownership; B94 = argsort serving an external order contract. The runtime-deterministic-no-atomics point is carried by none of them.

### 8. `integer-mma-epilogue-dequant-scale-residency` — Compute/DtypeAlignment — B−
Integer products accumulate in `int` fragments per SPAN (= min of the two group sizes; groups must divide each other, `static_assert`ed); dequantization hoisted to the epilogue (`a_scale·w_scale·int_product`, + `corr_w(col)·corr_a(row)` zero-point correction with the activation side using **precomputed group sums** instead of decoding ZPs). Weight-scale residency is a template choice: `Residency::Threadgroup` (double-banked cache, `group_slot_offset(k) = (k & 1) * THREADGROUP_BLOCK_N`, next-group `prefetch(k+1)` before the barrier) vs `Residency::Registers`; the HOST decides (`b_bits==4 && group==32 && block_m <= 32 → false`; `Tile32x64x256` stages only when `k/split_k/group_size >= 6`).
- Adjacent to the riir-ai Bench-850 integer-dot scoping (per-tensor PoT epilogue there; per-GROUP here) — corpus has no integer-MMA epilogue rule. May merge with candidate 6 into one DtypeAlignment rule at mine time.

### 9. `single-threadgroup-fused-count-scan` — Parallelism — C+
MoE token counting + exclusive scan + total in ONE kernel launch, ONE threadgroup: threadgroup-local atomic histogram (tiled over E), raking exclusive scan with `simd_prefix_exclusive_sum` for the cross-warp step, carry across 128-count chunks. Limits: E ≤ 512 (tile) / 1023 (scan). Kills the multi-kernel scan chain + host sync in the MoE dispatch prologue; does not scale past small E. Adjacent to `csr-index-via-prefix-sum` (popcorn) — the fused single-TG prologue shape is the new bit. Likely rider.

### 10. `radix-select-ordered-key-topk` — Parallelism — B−
MSB-first radix SELECT (not sort) for top-k, k ≤ 512: 10-bit digits, pass count derived from index bits, per-pass refinement, multi-partition histograms merged by the last-arriving TG via a device atomic arrival counter, final k finished with an smem bitonic. Key packing encodes f32 total order + index tie-break in ONE u64: `top_k.h:19-21` `return (ulong(top_k_score_key(score)) << index_bits) | (index_mask - ulong(index));` ("Larger keys follow the f32 total order; equal scores prefer smaller indices").
- Scope answer: NOT the MoE router — `MoeRouterTopK` uses iterative threadgroup-argmax with min-index tie-break + winner poisoning, fine for E ≤ 512 / K ≤ 128; `RadixTopKSmall` is the sampling/speculative vocab top-k (vocab 248_320, k 512; partitions 4 → 8 at ≥131_072 columns). **The repo ships BOTH and routes by size — that routing is itself the lesson.** Bit-exact parity incl. ±0/Inf/NaN payloads. Lower prefill relevance (decode/sampling side).

## Riders / instances (fold at next touch of the cited rule)

1. **B49 external confirmations ×2 + toolchain fact:** `ssm/ssd_prefill.metal:154` `thread float lane_states[SSM_PREFILL_MAX_STATE / 32];` driven by runtime `chunk_count` (the generic arm) vs the dedicated state_size=64 specialization that removes the runtime-indexed array entirely (plain scalars) — a live instance of `runtime-arg-loop-bound-demotes-arrays-to-local-memory`. Plus `gdn/prefill.metal:60-61`: `// Unrolled by hand: METAL_PRAGMA_UNROLL is ignored under metal4.0 -O2, so a` / `// loop over s[i] keeps the array in an alloca and reloads it every token.` — extends B49's corpus note to Metal 4.0 toolchains.
2. **B97 instance — causal frontier block clamp:** three sites clamp at BLOCK/TILE granularity via the tile's LAST row's frontier — `attention_gemm_grouped.metal:215-216` `const uint causal_blocks = (uint(prefix_length) + max_query_index + uint(BK)) / uint(BK);` / `block_end = min(block_end, causal_blocks);` + mask evaluation only on boundary blocks (`masked_block = IS_TAIL || ((is_causal || is_trie) && (key_base + BK > prefix_length))`); GDN gram tile early-exit `col_base >= row_base + valid_rows`. Arithmetic bound vs B97's pre-pass byte-map — same skip-family, different mechanism half.
3. **B103/B100 instances — GQA M-packing grouped MXU attention + measured route ladder:** `gemm_grouped`: `grouped_rows = params.gqa_factor * params.q_len` (GQA query heads packed into the M dimension of ONE MXU GEMM per KV-group — the attention-tile instance of `cross-lane-column-packed-projections`), head-dim sliced across 8 simdgroups (`static_assert(Layout::SIMDGROUPS == 8)`) with ONE threadgroup `score_exchange` buffer (2 barriers/K-block); measured route ladder `attention/mod.rs:122-133`: base `SINGLE_PASS_KV_THRESHOLD 1024`, `suffix_length > 8 → GEMM`, D256 suffix 1..16 kv > `150_000` → single_pass, D128 suffix 9..16 kv > `2_048` → two_pass.
4. **B104 instances — measured occupancy-target table + qmv route table:** `gemm_grouped/policy.rs:50-60` `MEASURED_TG_PER_CORE_TENTHS` — split-KV factor from a measured threadgroups-per-core table (in TENTHS) keyed (head_dim, suffix, kv band), fallback 6.0, non-monotone by measurement, `TODO: validate this table per chip (M1-M5)`. Plus the qmv table: ~300 hand-measured rows keyed `(device_name, apple_gpu_family, bits, group, m, shape_class_bitmask)` over the 7 Qwen3.6-27B layer shapes, routes carrying WHOLE GEMM plans incl. split_k, fallback = exact device → **unanimous same-family consensus** (all matching rows must agree, else None → general policy) → general policy; pinned by `FROZEN_PLAN_FINGERPRINT = 5_839_212_743_558_880_136` (xxh3-64 over the 1176-decision matrix). The unanimous-consensus fallback + the occupancy-target unit are new micro-content vs B104's ladder (exact bucket → collapsed domain → family-representative → baseline).
5. **B98 instance — GEMM-over-GEMV crossover carve-outs:** `prefer_gemm_over_gemv`: GEMM steals the batch only when MXU + 16-row tile + m ≥ 4, with m == 4 requiring `n ≤ 6144 && k ≤ 9728 || k > 3n`, m == 5/BF16 and gathered inputs forced back to GEMV; pinned boundary tests (`gemv_gemm_route_boundaries_are_preserved`). The bridge-rung family's MXU-era instance.
6. **LinearAttnRecurrence instance — state-resident chunked GDN:** chunk-INVARIANT work (cumsum, Gram, A⁻¹, T=(I+A)⁻¹) parallelizes across chunks; only the state recurrence is sequential, and the state slice lives in **threadgroup memory for the whole prefill** (one TG owns (v-head, value-slice); zero per-chunk global state round-trips) — `output_and_state.metal:29-33` documents the R/T/out/S update. Thresholds: `MXU_MIN_T: 256` (below it the plain recurrent kernel wins — dispatch cost dominates), `CHUNK_SIZE: 64`; chunked is **MXU-gated**, so on M1–M4 uzu runs the plain recurrent prefill — converging with our Issue 734 chunked-GDN negative and B95's whole-sequence rule. Parity-pinned at chunk boundaries 63/64/65/129.
7. **Morton counter-regime note on `l2-grouped-tile-schedule` (popcorn):** uzu DOES use Morton on the MXU FP path, gated by a padding budget — `gemm/kernel.rs:220` `let use_morton = min_dim > 1 && morton_total <= 4_u32.saturating_mul(actual_total);` — with quant + split-K paths forcing `use_morton: false`. Counter-evidence to popcorn's "a simple rectangular tiling beat Morton order in practice" (different substrate: cooperative-tensor L2 behavior). Record as a regime annotation, never a rule (the corpus rule's own evidence note already frames Morton as workload-dependent).
8. **Micro-riders:** `masked_score_pre_scale` branchless exp2 masking (mask × scale pre-divided so masked lanes skip a branch); `two_pass` fixed `TOTAL_BLOCKS_COUNT 32` grid-stride split with F32 partials + pass-2 `simd_sum` **no shared memory** + flat head→KV-head mapping for L1 reuse; `IS_TAIL` compile-time tail-block flag so bounded loads exist only in the tail instance; per-lane column-per-lane 16×16 triangular inverse (`tri_inv.h` one column per lane, forward substitution in per-lane registers).

## Drop rows

| Candidate | Reason |
|---|---|
| Morton dispatch as a RULE | Covered by `l2-grouped-tile-schedule` (popcorn) incl. its anti-Morton evidence note; uzu's usage = regime annotation (rider 7). |
| Attention route ladder as a standalone rule | B104 swept-table family instance (rider 4). |
| Chunked GDN decomposition as a standalone rule | B79/B95 family + our Issue 734 negative; the state-residency + thresholds are rider 6 material. |
| `gemm_alignment.h` 8-variant alignment switch | Same family as the corpus's boundary-emulation/alignment rules (B70/B92 class); no new decision content. |
| `SHARED_STRIDE` smem padding | Bank-conflict pad, ambient. |
| hazard_tracker / command_buffer / range_allocator | Host infra; B26/B58/B31-covered classes (not deep-read this pass). |
| `weaver/` frontier kernels + `speculators/` | Spec-decode territory, outside the prefill aim; unread in detail — flag for a future decode-focused pass (B71/B77/B81 family adjacency). |
| `metal/sparse/**` (sparse_heap, sparse_page_size_extensions) | Unread — likely Metal sparse residency for expert weights; re-read at mine time before any claim. |
| sigmoid_gate / qkv_norm / finalize / mask.h | Ambient or covered classes (lane B drop table). |
| MoE decode GEMV shapes | B58/B15/B24/B62 family instances (external confirmations only). |
| Marketing claim "up to 38% faster prompt processing vs MLX" | Marketing, not in-tree evidence — never cite. |

## League reading (priority-3 surface)

uzu is a **competitor engine with an M3 lane** — an opponent *candidate*, not an opponent fork. Adding it as a scored opponent is a perf-rematch-skill / owner call (same posture as the streaming-opponent question). Their M3-relevant prefill surface: the attention route ladder + measured split tables + simdgroup GEMM tiling buckets; chunked GDN is MXU-gated OFF on M1–M4 (plain recurrent prefill), which independently corroborates our Issue 734 negative. The MXU/MPP family is M5+/Gen18+ — next-silicon material (B103 NAX posture), zero effect on pinned league cells today, but the league's next-silicon column should expect Apple cooperative-vector attention/GEMM the way it now expects NAX.

## Caveats

- **No committed perf numbers in-tree** — policy boundaries + parity tolerances are the only pinned numeric artifacts; do not cite uzu numbers as measured evidence.
- Quotes pinned at `115609b`; Release 0.5.23 is fresh (2026-09-03) and the repo moves — **re-verify all quotes at mine time** (the B96–98 discipline).
- Lane A read the qmv routes table schema + lookup + M1/M2/M2Pro/M5Max rows and grepped the remainder (~300 rows, partially unread) — re-read at mine time.
- Unread this pass: `metal/sparse/**`, `speculators/**`, `weaver/**` kernels, `sampling/**`, `engine/` session orchestration, `encodable_block/**` (moe/mamba_mixer/weaver wiring beyond what lanes read) — mark unverified in the batch plan.
- Corpus counts moved (466 at verdict time) — batch plan re-scans per standing discipline.

## Queue slot

`riir-clippy/.distill/001_mining_queue_snapshot.md` — next kernel batch (Batch 105 candidate), 2 inspection lanes already complete; batch plan (`.plans/NNN_*`) opens BEFORE the first corpus edit per the plan-first rule. Lead with `indirect-device-dispatch-tile-map-moe`.

//! Entries slice — Batches 14-46 era (rules 120-160 of 340) — riir-gpu production-kernel mining.
//! Pure data moved verbatim from the former corpus.rs monolith
//! (2026-08-15 split); append new batches to the LAST gpu slice or
//! open a new one past ~900 lines. Category membership lives ONLY in
//! `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // Batch 14 (Research 015): riir-gpu production kernels — norm-fold into matmul,
        // reduction-split pre-pass, single-plane reduction, CPU-bottleneck offload.
        // 4 rules from 2 riir-gpu source files (the first batch mining the stack's own
        // kernels, not external libraries).
        // ════════════════════════════════════════════════════════════════════
        // ── fold-norm-scalar-into-matmul — separate norm pass → scalar fold into GEMV dot (C) ──
        // Source: riir-gpu gemv_q4k_batched_rmsnorm_cubecl.rs — the fused kernel folds
        // `* inv_rms[pos] * gamma[col]` INTO the GEMV dot product. RMSNorm's inv_rms is a
        // single scalar per position (shared across the hidden dim); multiplying it inline
        // during the per-element accumulation adds 2 muls/element — negligible vs the
        // dequant cost. Eliminates the CPU RMSNorm loop + the normed buffer upload
        // (~30s/step saved at 12B). Distinct from `fuse-norm-act-linear` (fuses 3
        // elementwise ops; this folds a reduction-derived scalar into a matmul).
        KernelRule {
            rule_id: "fold-norm-scalar-into-matmul".to_string(),
            gpu_concept: "fold normalization scalar (inv_rms) into the matmul dot product — norm becomes 2 inline multiplies".to_string(),
            ref_shape: "// naive: separate CPU RMSNorm pass → normed buffer → upload → GPU GEMV\nfn rmsnorm_then_gemv_naive(input: &[f32], gamma: &[f32], weight: &[f32], inv_rms: &[f32], m: usize, n: usize, batch: usize) -> Vec<f32> {\n    // pass 1 (CPU): normed[pos][i] = input[pos][i] * inv_rms[pos] * gamma[i]\n    let mut normed = vec![0.0f32; batch * n];\n    for pos in 0..batch { for i in 0..n {\n        normed[pos*n + i] = input[pos*n + i] * inv_rms[pos] * gamma[i];\n    } }\n    // pass 2 (GPU GEMV): output[pos][row] = sum_col weight[row][col] * normed[pos][col]\n    let mut out = vec![0.0f32; batch * m];\n    for pos in 0..batch { for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n { acc += weight[row*n + col] * normed[pos*n + col]; }\n        out[pos*m + row] = acc;\n    } }\n    out // requires uploading `normed` (batch * n * 4 bytes) to GPU\n}".to_string(),
            opt_pattern: "// optimized: fold inv_rms * gamma INTO the GEMV dot (riir-gpu gemv_q4k_rmsnorm)\nfn gemv_with_inline_norm(input: &[f32], gamma: &[f32], weight: &[f32], inv_rms: &[f32], m: usize, n: usize, batch: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; batch * m];\n    for pos in 0..batch { for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n {\n            // 2 extra multiplies/element — negligible vs dequant; same memory pattern\n            let normed_input = input[pos*n + col] * inv_rms[pos] * gamma[col];\n            acc += weight[row*n + col] * normed_input;\n        }\n        out[pos*m + row] = acc;\n    } }\n    out // NO normed buffer, NO upload — input stays raw on GPU from prior layer\n}".to_string(),
        },
        // ── split-reduction-scalar-prepass — fused reduction+consumer → scalar pre-pass + fold (E) ──
        // Source: riir-gpu gemv_q4k_batched_rmsnorm_cubecl.rs — `compute_inv_rms` (tiny
        // pre-pass: input[batch×n] → inv_rms[batch], 1 scalar/position) then the fused GEMV
        // that consumes the scalar. The split is required because the GEMV's per-element
        // accumulation can't see the reduction result without a cross-warp barrier. The
        // pre-pass writes O(batch) scalars (not O(batch×hidden) tensors), so it's cheap.
        // General principle: when a reduction's output is tiny, split + fold the scalar.
        // Distinct from `shift-via-pad-subtract` (avoids a two-pass) + `chunk-split-mean-pool`
        // (splits a tensor into chunks; this splits a reduction FROM its consumer).
        KernelRule {
            rule_id: "split-reduction-scalar-prepass".to_string(),
            gpu_concept: "split a reduction whose output is a tiny scalar into a pre-pass, fold the scalar into the main consumer".to_string(),
            ref_shape: "// naive: compute the reduction (e.g. mean-of-squares for RMSNorm) on CPU, upload the full normed tensor\nfn norm_on_cpu_then_gpu_matmul(input: &[f32], gamma: &[f32], weight: &[f32], m: usize, n: usize, batch: usize) -> Vec<f32> {\n    // CPU computes the full reduction + normalization (slow: O(batch*n) CPU work)\n    let mut normed = vec![0.0f32; batch * n];\n    for pos in 0..batch {\n        let mut sum_sq = 0.0;\n        for i in 0..n { sum_sq += input[pos*n + i] * input[pos*n + i]; }\n        let inv_rms = 1.0 / (sum_sq / n as f32 + 1e-6).sqrt();\n        for i in 0..n { normed[pos*n + i] = input[pos*n + i] * inv_rms * gamma[i]; }\n    }\n    // then GPU matmul on the uploaded normed tensor\n    let mut out = vec![0.0f32; batch * m];\n    for pos in 0..batch { for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n { acc += weight[row*n + col] * normed[pos*n + col]; }\n        out[pos*m + row] = acc;\n    } }\n    out\n}".to_string(),
            opt_pattern: "// optimized: split — scalar pre-pass (GPU) + fused main pass that folds the scalar (riir-gpu)\nfn split_prepass_then_fused_main(input: &[f32], gamma: &[f32], weight: &[f32], m: usize, n: usize, batch: usize) -> Vec<f32> {\n    // PRE-PASS (GPU dispatch 1): reduce input[batch×n] → inv_rms[batch] (O(batch) scalars)\n    let mut inv_rms = vec![0.0f32; batch];\n    for pos in 0..batch {\n        let mut sum_sq = 0.0;\n        for i in 0..n { sum_sq += input[pos*n + i] * input[pos*n + i]; }\n        inv_rms[pos] = 1.0 / (sum_sq / n as f32 + 1e-6).sqrt();\n    }\n    // MAIN PASS (GPU dispatch 2): fused GEMV + inline norm (folds the scalar, see fold-norm rule)\n    let mut out = vec![0.0f32; batch * m];\n    for pos in 0..batch { for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n {\n            acc += weight[row*n + col] * input[pos*n + col] * inv_rms[pos] * gamma[col];\n        }\n        out[pos*m + row] = acc;\n    } }\n    out // NO full-normed-tensor upload — only O(batch) scalars cross the pre-pass boundary\n}".to_string(),
        },
        // ── single-plane-sufficient-reduction — N redundant planes → one plane (E) ──
        // Source: riir-gpu gemv_q4k_batched_rmsnorm_cubecl.rs Issue 611 — the original
        // `compute_inv_rms` launched CubeDim::new_1d(256) = 8 planes × 32 lanes. All 8
        // planes independently strided over the same n elements + each called plane_sum
        // (which only reduces within a plane) → 8× wasted work + 8 threads wrote the same
        // value (benign race, correct result, 8× the FLOPs). Fix: CubeDim::new_1d(32) —
        // one plane's 32 lanes stride by PLANE_DIM to cover all n, then plane_sum reduces
        // exactly the contributing lanes. When a reduction fits in one plane, launch one.
        // Distinct from `multi-dim-grid-launch`/`block-count-clamp` (those are block/grid
        // axes; this is the plane/warp axis WITHIN a block).
        KernelRule {
            rule_id: "single-plane-sufficient-reduction".to_string(),
            gpu_concept: "one plane (warp) suffices for a cooperative reduction — do not launch N redundant planes that recompute the identical sum".to_string(),
            ref_shape: "// naive: launch N planes that ALL redundantly compute the identical reduction (Issue 611)\nfn reduce_with_redundant_planes(input: &[f32], n: usize, batch: usize, plane_dim: usize, n_planes: usize) -> Vec<f32> {\n    // dispatch: CubeDim::new_1d(plane_dim * n_planes) — e.g. 256 = 8 planes × 32 lanes\n    let threads_per_cube = plane_dim * n_planes;\n    let mut out = vec![0.0f32; batch];\n    for pos in 0..batch {\n        for plane in 0..n_planes { // each plane independently does the FULL reduction\n            let mut partial = 0.0;\n            // each lane in this plane strides by plane_dim to cover all n\n            for lane in 0..plane_dim {\n                let col = lane;\n                if col < n { partial += input[pos*n + col]; }\n            }\n            let plane_sum_val = partial; // plane_sum reduces within this plane\n            if plane == 0 { out[pos] = plane_sum_val; } // only plane 0 writes (others wasted)\n        }\n    }\n    out // n_planes× wasted FLOPs — all planes computed the same sum\n}".to_string(),
            opt_pattern: "// optimized: one plane suffices (riir-gpu Issue 611 fix) — CubeDim::new_1d(plane_dim)\nfn reduce_single_plane(input: &[f32], n: usize, batch: usize, plane_dim: usize) -> Vec<f32> {\n    // dispatch: CubeDim::new_1d(plane_dim) — e.g. 32 lanes = 1 plane\n    let mut out = vec![0.0f32; batch];\n    for pos in 0..batch {\n        let mut partial = 0.0;\n        // the plane's lanes stride by plane_dim to cover all n columns\n        let mut col = 0;\n        while col < n {\n            partial += input[pos*n + col];\n            col += plane_dim;\n        }\n        let plane_sum_val = partial; // plane_sum reduces exactly the lanes that contributed\n        out[pos] = plane_sum_val; // one write, no redundant planes\n    }\n    out // 1× the work — the 32 lanes cover all n via the stride\n}".to_string(),
        },
        // ── offload-cpu-bottleneck-to-gpu-kernel — CPU data-prep → GPU compute into pooled memory (A) ──
        // Source: riir-gpu gpu_transpose.rs Issue 402 Phase 9h — the LM head weight matrix
        // (640 MB at 0.40B, 163840×1024 aspect) took 674 ms/step on M3 Max CPU due to
        // scattered cache writes. Fix: upload raw (non-transposed) via write_buffer into a
        // staging GPU buffer (sequential write — fast), then a tiled WGSL compute kernel
        // reads staging + writes transposed DIRECTLY into the CubeCL-pooled target buffer
        // (the same memory WeightBufferSlot caches). The transform becomes a GPU dispatch
        // instead of a CPU stall + upload. Distinct from `contiguity-guard-before-reshape`
        // (skip the copy when already contiguous — opposite: this does the copy on GPU
        // when the CPU copy is the bottleneck).
        KernelRule {
            rule_id: "offload-cpu-bottleneck-to-gpu-kernel".to_string(),
            gpu_concept: "when CPU data-prep (transpose, permute, format convert) is the bottleneck, run a GPU compute kernel that writes directly into pooled memory".to_string(),
            ref_shape: "// naive: CPU transposes the weight matrix, then uploads the transposed result\nfn cpu_transpose_then_upload(weight: &[f32], rows: usize, cols: usize) -> Vec<f32> {\n    // CPU transpose — O(rows*cols) scattered writes, cache-unfriendly for extreme aspect ratios\n    // (e.g. 163840×1024 = 640 MB → 674 ms/step on M3 Max due to cache miss storms)\n    let mut transposed = vec![0.0f32; rows * cols];\n    for r in 0..rows { for c in 0..cols {\n        transposed[c*rows + r] = weight[r*cols + c]; // scattered write — cache killer\n    } }\n    transposed // then upload this (sequential, but the CPU step already dominated)\n}".to_string(),
            opt_pattern: "// optimized: GPU compute kernel transposes into pooled memory (riir-gpu Issue 402)\nfn gpu_transpose_into_pooled(weight: &[f32], rows: usize, cols: usize) -> Vec<f32> {\n    // step 1: upload RAW (non-transposed) weight via write_buffer — sequential write, fast\n    let staging = weight.to_vec(); // (on GPU: queue.write_buffer into staging buffer)\n    // step 2: tiled GPU compute kernel reads staging, writes transposed into pooled target\n    let tile = 32;\n    let mut transposed = vec![0.0f32; rows * cols];\n    for tr in (0..rows).step_by(tile) { for tc in (0..cols).step_by(tile) {\n        // one workgroup per tile — coalesced read + coalesced write\n        for dr in 0..tile { for dc in 0..tile {\n            let r = tr + dr; let c = tc + dc;\n            if r < rows && c < cols {\n                transposed[c*rows + r] = staging[r*cols + c]; // writes into pooled memory directly\n            }\n        } }\n    } }\n    transposed // the CPU stall is gone — the transform is a GPU dispatch\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 15 (Research 016): riir-gpu production kernels — block-contiguous
        // weight layout, fused dual GEMV + GeGLU, orthogonal pre-rotation,
        // water-fill tail-dim shared codebook. 4 rules from 3 riir-gpu source
        // files (the second batch mining the stack's own kernels).
        // ════════════════════════════════════════════════════════════════════
        // ── block-contiguous-weight-layout — 3-buffer SoA → 1-buffer AoS (A) ──
        // Source: riir-gpu gemm_ternary_block_contiguous_cubecl.rs (Issue 650
        // Phase 2). The prior layout used 3 separate GPU buffers (pos_bits,
        // neg_bits, group_scale) — 3 global-memory accesses per K-tile per row.
        // Packing them into a single block-contiguous buffer (9 × u32 per
        // 128-weight group) needs 1 access per group. At large N (ffn_down at
        // N=17408), this is a 3× reduction in global-memory accesses — the
        // structural fix for the 1.89× vs llama.cpp 9.4× gap (Bench 645 #5).
        // Distinct from `matvec-coalesce` (within a single buffer) — this
        // packs multiple LOGICAL buffers into one PHYSICAL buffer.
        KernelRule {
            rule_id: "block-contiguous-weight-layout".to_string(),
            gpu_concept: "pack multiple logical weight buffers (scale + bit-planes) into one block-contiguous buffer so one global-memory access fetches all components".to_string(),
            ref_shape: "// naive: 3 separate GPU buffers (SoA) — 3 global-memory accesses per group\nstruct TernaryWeightsSoA { pos_bits: Vec<u32>, neg_bits: Vec<u32>, scale: Vec<f32> }\nfn dequant_soa(w: &TernaryWeightsSoA, row: usize, col: usize, groups_per_row: usize) -> f32 {\n    let g = col / 128;\n    let scale = w.scale[row * groups_per_row + g];          // access 1\n    let lc = col % 128;\n    let pos = (w.pos_bits[row * groups_per_row * 4 + g * 4 + lc / 32] >> (lc % 32)) & 1; // access 2\n    let neg = (w.neg_bits[row * groups_per_row * 4 + g * 4 + lc / 32] >> (lc % 32)) & 1; // access 3\n    let sign = pos as f32 - neg as f32;\n    sign * scale\n}".to_string(),
            opt_pattern: "// optimized: single block-contiguous buffer (AoS) — 1 access per group\n// layout per 128-weight group: u32[0]=scale, u32[1..5]=pos plane, u32[5..9]=neg plane\nconst U32_PER_BLOCK_GROUP: usize = 9;\nfn dequant_block_contiguous(block_buf: &[u32], row: usize, col: usize, groups_per_row: usize) -> f32 {\n    let g = col / 128;\n    let blk_base = (row * groups_per_row + g) * U32_PER_BLOCK_GROUP;\n    let scale = f32::from_bits(block_buf[blk_base]);          // access 1 (the only access)\n    let lc = col % 128;\n    let pos = (block_buf[blk_base + 1 + lc / 32] >> (lc % 32)) & 1; // same cache line\n    let neg = (block_buf[blk_base + 5 + lc / 32] >> (lc % 32)) & 1; // same cache line\n    let sign = pos as f32 - neg as f32;\n    sign * scale\n}".to_string(),
        },
        // ── fuse-gate-up-gemv-activation — 3 dispatches → 1 fused (C) ────────
        // Source: riir-gpu gemv_geglu_f16_cubecl.rs (Plan 106 P8/P9). A GeGLU
        // (or SwiGLU) activation consumes two independent linear projections
        // (gate + up) of the SAME input vector. The naive path runs 3 GPU
        // dispatches: gate GEMV, up GEMV, GeGLU elementwise. Fusing them into
        // one kernel saves 2 dispatches per layer × 26 layers = **52 dispatches
        // per decode token**. The two GEMVs share the input vector (loaded
        // once); the activation folds inline after the plane_sum reduction.
        // Distinct from `fused-matmul-activation` (1 matmul + 1 act, sequential)
        // — this fuses two PARALLEL sibling projections.
        KernelRule {
            rule_id: "fuse-gate-up-gemv-activation".to_string(),
            gpu_concept: "fuse two sibling GEMVs (gate + up projections) + their multiplicative activation into one kernel dispatch. RIDER (Batch 170, mlxfast-gemma4 @ 27c821c4 -- Gemma4PromptGlueV1.swift L1-33): the VIEW-SHAPE hazard on this fusion's inputs -- gate|up consumed through STRIDED views selects the compiled kernel's 2-D 'strided' variant (elem_to_loc per input per element, scalar bf16 loads, 'the slowest access pattern MLX has'): per layer 277 MB (experts) + 104 MB (dense) = 11 GB per prompt pass; the fused kernel must read the gate|up plane DIRECTLY (a column offset for the up half) over a contiguous plane with a row floor >= 1024 -- fusing the arithmetic while inheriting the strided view keeps the slow loads. RIDER (Batch 251, yukon cudafast-bonsai2-9b-engine @ 36c9536 -- the quantized-matvec second-partials-plane face): the GLU gate rides the SAME quantized matvec as a second shared-memory partials plane -- 'extern __shared__ float partials[];        // [ncols][rows_per_cta][bprp], then partials_gate' (mmvq-ptq1_0.cuh:311), and the smem-DOUBLING IS the admission guard shared by guard and launcher ('return (size_t) ncols_dst * rows_per_cta * (blocks_per_row + 1) * sizeof(float) * (has_gate ? 2 : 1);' :292 -- oversize falls through to the unfused path), the epilogue op-polymorphic over the GLU family ('switch (fusion.glu_op) { case GGML_GLU_OP_SWIGLU: result *= ggml_cuda_op_silu_single(gate_value);' :406-408, swiglu/geglu/swiglu_oai/mul arms) with the posture asserted ('GGML_ASSERT(ncols == 1 && \"fusion only supported for ncols_dst=1\");' :513). RIDER (Batch 251, yukon cudafast-bonsai2-9b-engine @ 36c9536 -- the row-interleaved wire-layout face, pure tiling): rows 0-7 gate / 8-15 up of the same feature per 16-row mma tile -- 'in every 16-row mma tile rows 0-7 are gate rows of 8 features and rows 8-15 the up rows of the same features, so a thread holds gate (row g) and up (row g + 8) of feature row_base/2 + 8*rt + g. It stores silu(gate)*up there with the expression of unary_gated_op_kernel<op_silu, float> (the SWIGLU node's value)' (mmq-pq2-j128.cuh:479-482) -- so one thread's accumulators hold gate (acc[l]) and up (acc[l+2]) and the store is 'out[(int64_t) j*stride_col + i] = ggml_cuda_op_silu_single(acc[rt][ct][l]) * acc[rt][ct][l + 2];' (:500) with ZERO extra traffic; 'GLU (pure tiling only): x is a gate/up row-interleaved weight of 2*F rows, dst the F-row gate output'; 'every accumulator is the one the separate gate and up launches compute' (:512-513) -- bit-identity to the unfused pair BY CONSTRUCTION, the wire layout (not an arithmetic change) is the whole fusion. RIDER (Batch 276, the INJECT-4 LATENCY-CHAIN UNROLL face, Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ 59160bd (MIT), intro commit 709404c -- where this host fuses PARALLEL sibling projections (gate + up) sharing one input, this face is the DEPENDENT-CHAIN half of the same one-launch instinct: a 4-row inject projection rides MLX's small-N `qmv` branch as pure latency -- 'over 4 rows is latency-bound (~50 us in a dependent chain: one' 'threadgroup walking K sequentially); here the walk is unrolled so' 'the loads pipeline, with the accumulation order unchanged.' (Runner/FastModel/TrackFastKernels2.swift:597-599) -- the fused kernel carries MLX's own `qmv_impl` arithmetic verbatim (same lanes, same per-lane accumulation order, same simd_sum) and the unroll is 'UNR is a scheduling hint only; never split or reassociate the sum.' (:712). The simdgroup split: 'MLXFAST-INJSPLIT: one original small-N row per simdgroup. ROW changes' 'only the base addresses' (:709-710, landed 8416766) -- lane-to-K mapping and the sum untouched, bit-exact by construction. The dispatch stays shape-gated, wide windows keep the stock launch: 'One-token windows: MLX routes the 4-row inject GEMV to `qmv`, a' 'latency-bound launch; the fused mixer-head kernel carries the same' 'arithmetic. Wider windows route to `qmv_wide`, so they keep MLX's' 'own launch.' (Runner/FastModel/TrackFastModel.swift:622-625). Kinship only: `same-input-gemv-segments-single-launch` (entries_gpu_07.rs, the segment-table form of one-launch-over-one-input) and `stacked-gate-up-single-launch-split-store` (gpu_106, the CUDA layout-fusion twin -- cite, don't ride: that fuses by WIRE LAYOUT, this by unrolling one dependent K walk). Batch 276 / Plan 280 / Research 314.".to_string(),
            ref_shape: "// naive: 3 separate GPU dispatches (gate GEMV, up GEMV, GeGLU elementwise)\nfn geglu_three_dispatches(weight_gate: &[f32], weight_up: &[f32], input: &[f32], m: usize, n: usize) -> Vec<f32> {\n    let gate = gemv_launch(weight_gate, input, m, n);  // dispatch 1\n    let up = gemv_launch(weight_up, input, m, n);     // dispatch 2 (re-reads input)\n    // dispatch 3: elementwise GeGLU = GELU(gate) * up\n    gate.iter().zip(up.iter()).map(|(g, u)| gelu(*g) * u).collect()\n}".to_string(),
            opt_pattern: "// optimized: one fused kernel — gate + up + GeGLU in a single dispatch (riir-gpu gemv_geglu_f16)\nfn geglu_fused(weight_gate: &[f32], weight_up: &[f32], input: &[f32], m: usize, n: usize, plane_dim: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for row in 0..m {\n        let off = row * n;\n        // gate dot product (cooperative plane reduction)\n        let mut gate = 0.0;\n        for k in 0..n { gate += weight_gate[off + k] * input[k]; }\n        // up dot product — input[k] is already in register, re-use it\n        let mut up = 0.0;\n        for k in 0..n { up += weight_up[off + k] * input[k]; }\n        // GeGLU: GELU(gate) * up — inline, no intermediate buffer\n        out[row] = gelu(gate) * up;\n    }\n    out // 1 dispatch total (was 3) — saves 52 dispatches/decode at 26 layers\n}".to_string(),
        },
        // ── pre-rotate-once-exploit-orthogonality — O(N) rotations → O(1) (B) ─
        // Source: riir-gpu spectralquant/attention.rs (Plan 078). SpectralQuant
        // rotates keys by an orthogonal eigenbasis V: score = dot(q, V·y_hat).
        // Orthogonality gives the identity dot(q, V·y) = dot(V^T·q, y), so we
        // pre-rotate the query ONCE (one d_h × d_h matvec) and compute
        // dot(V^T·q, y_hat) for each key — no per-key rotation. At seq_len_kv =
        // 4096, d_h = 128, this cuts rotation FLOPs by ~4096×. General pattern:
        // any orthogonal transform (eigendecomposition, QR, Householder) where
        // one side is shared across many inner products.
        KernelRule {
            rule_id: "pre-rotate-once-exploit-orthogonality".to_string(),
            gpu_concept: "for dot(q, R·y) with many y and orthogonal R, pre-rotate the query once (q' = R^T·q) instead of rotating each y".to_string(),
            ref_shape: "// naive: rotate each key by orthogonal R, then dot with q\nfn score_rotate_each_key(q: &[f32], keys: &[f32], r: &[f32], seq_len_kv: usize, d_h: usize) -> Vec<f32> {\n    let mut scores = vec![0.0f32; seq_len_kv];\n    for t in 0..seq_len_kv {\n        // rotate key t: rotated[d] = Σ_j R[d*d_h + j] * keys[t*d_h + j]  — O(d_h²) per key\n        let mut rotated = vec![0.0f32; d_h];\n        for d in 0..d_h {\n            for j in 0..d_h { rotated[d] += r[d * d_h + j] * keys[t * d_h + j]; }\n        }\n        // dot with q\n        for d in 0..d_h { scores[t] += q[d] * rotated[d]; }\n    }\n    scores // O(seq_len_kv × d_h²) rotation FLOPs\n}".to_string(),
            opt_pattern: "// optimized: pre-rotate query once (q' = R^T·q), then dot q' with each unrotated key\nfn score_pre_rotate_query(q: &[f32], keys: &[f32], r: &[f32], seq_len_kv: usize, d_h: usize) -> Vec<f32> {\n    // one-time pre-rotation: q'[d] = Σ_j R^T[d*d_h + j] * q[j] = Σ_j R[j*d_h + d] * q[j]\n    let mut q_prime = vec![0.0f32; d_h];\n    for d in 0..d_h {\n        for j in 0..d_h { q_prime[d] += r[j * d_h + d] * q[j]; }\n    }\n    // per-key: just dot(q', key) — NO rotation\n    let mut scores = vec![0.0f32; seq_len_kv];\n    for t in 0..seq_len_kv {\n        for d in 0..d_h { scores[t] += q_prime[d] * keys[t * d_h + d]; }\n    }\n    scores // O(d_h²) pre-rotation + O(seq_len_kv × d_h) dots — ~seq_len_kv× fewer rotation FLOPs\n}".to_string(),
        },
        // ── water-fill-tail-dim-shared-codebook — per-dim codebook everywhere → ─
        // top-dims per-dim, tail-dims shared low-bit (N)
        // Source: riir-gpu spectralquant/dequant.rs (Plan 078). Variable-bit
        // dequant where the top d_eff dimensions get their own per-dim Lloyd-Max
        // codebooks sized to the water-fill bit allocation, but the tail dims
        // (d_eff..d_h, which carry <1% of the energy) share a single codebook
        // at b_low bits. Cuts codebook memory from O(d_h × 2^max_bits) to
        // O(d_eff × 2^max_bits + (d_h - d_eff) × 2^b_low). At d_h=128, d_eff=32,
        // max_bits=4, b_low=2: 2048 → 896 floats (2.3× memory reduction). The
        // tail dims are too low-energy for per-dim codebooks to pay back their
        // memory cost. Distinct from `int2-pack-weights` (uniform 2-bit) and
        // `ternary-weight-quant` (uniform ternary) — this is NON-UNIFORM bit
        // allocation + tail-shared codebook.
        KernelRule {
            rule_id: "water-fill-tail-dim-shared-codebook".to_string(),
            gpu_concept: "non-uniform bit allocation: per-dim codebooks for top-energy dims, single shared low-bit codebook for low-energy tail dims".to_string(),
            ref_shape: "// naive: uniform per-dim codebooks at max_bits for all dims (ignores energy distribution)\nfn dequant_uniform_per_dim(packed: &[u32], codebook: &[f32], d_h: usize, max_bits: usize) -> Vec<f32> {\n    // codebook layout: [d_h][1 << max_bits] — every dim gets a full max_bits codebook\n    let cb_stride = 1usize << max_bits;\n    let mut out = vec![0.0f32; d_h];\n    for d in 0..d_h {\n        let idx = unpack_index(packed, d, max_bits); // max_bits per dim, uniform\n        out[d] = codebook[d * cb_stride + idx];\n    }\n    out // codebook memory = d_h × 2^max_bits (e.g. 128 × 16 = 2048 floats)\n}".to_string(),
            opt_pattern: "// optimized: top d_eff dims get per-dim codebooks (max_bits), tail dims share one b_low-bit codebook\nfn dequant_water_fill(packed: &[u32], codebook_top: &[f32], codebook_tail: &[f32], d_h: usize, d_eff: usize, max_bits: usize, b_low: usize) -> Vec<f32> {\n    let cb_top_stride = 1usize << max_bits;\n    let cb_tail_stride = 1usize << b_low;\n    let mut out = vec![0.0f32; d_h];\n    // top dims: per-dim codebook at max_bits (high fidelity)\n    for d in 0..d_eff {\n        let idx = unpack_index(packed, d, max_bits);\n        out[d] = codebook_top[d * cb_top_stride + idx];\n    }\n    // tail dims: shared single codebook at b_low bits (low fidelity, <1% energy)\n    for d in d_eff..d_h {\n        let idx = unpack_index(packed, d, b_low);\n        out[d] = codebook_tail[idx]; // SAME codebook_tail for all tail dims\n    }\n    out // codebook memory = d_eff × 2^max_bits + (d_h - d_eff) × 2^b_low\n        // e.g. 32×16 + 96×4 = 896 floats (was 2048) — 2.3× reduction\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 16 (Research 017): riir-gpu production kernels — third pass.
        // 4 rules from 2 riir-gpu source files (cmma hardware unit + dequant
        // staging + dispatch axis swap from gemm_ternary_simdgroup; norm-decoupled
        // low-bit scoring from forward_turboquant). Two spectralquant siblings
        // (rotation.rs, qjl.rs) + calibration.rs + lora_fused_cubecl.rs assessed
        // + dropped (rotation = boilerplate, pre-rotate already mined; qjl =
        // composition of water-fill with JL; calibration two-pass reduce overlaps
        // split-reduction; SGMV multi-adapter batched dispatch overlaps Category H
        // grouped-launch-cumsum-offsets). Corpus: **131 rules across 15 categories**.
        // ════════════════════════════════════════════════════════════════════
        // ── cmma-hardware-unit-over-scalar-alu — scalar dot in plane → cmma (B) ─
        // Source: riir-gpu gemm_ternary_simdgroup_cubecl.rs (Issue 641). The
        // plane-cooperative ternary GEMM does the dot product in scalar ALU
        // inside a plane — each lane accumulates `select(pos,x,0) -
        // select(neg,x,0)` per bit then `plane_sum()` reduces. Past ~8
        // accumulators this is occupancy-bound at ~1.08× (Bench 641). The cmma
        // variant uses Metal's `simdgroup_matrix_8x8` cooperative matrix
        // multiply-accumulate unit (CubeCL `cmma`) — a dedicated hardware unit
        // that does 64 FMAs per simdgroup per cycle. The ternary weights are
        // dequantized into a shared-memory tile, then consumed at hardware rate.
        // This is the structural difference: the SAME math runs on a unit built
        // for it (tensor core / cmma) instead of fighting for scalar ALU
        // bandwidth against the load pipeline. Generalizes to any GEMM where a
        // hardware matrix unit exists (NVIDIA tensor core, Metal simdgroup_matrix,
        // Intel DPAS). Distinct from `gemm-tiling` (tile size choice) — this is
        // about WHICH execution unit runs the multiply.
        KernelRule {
            rule_id: "cmma-hardware-unit-over-scalar-alu".to_string(),
            gpu_concept: "use the cooperative matrix multiply-accumulate hardware unit (tensor core / Metal simdgroup_matrix / cmma) instead of scalar ALU for matrix multiply".to_string(),
            ref_shape: "// naive: dot product in scalar ALU inside a plane — occupancy-bound\nfn ternary_dot_scalar_alu(pos_bits: &[u32], neg_bits: &[u32], scale: &[f32], input: &[f32], m: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n {\n            // scalar ALU: extract pos/neg bit, subtract, multiply — 1 FMA equivalent in scalar pipe\n            let pos = ((pos_bits[col / 32] >> (col % 32)) & 1) as f32;\n            let neg = ((neg_bits[col / 32] >> (col % 32)) & 1) as f32;\n            acc += (pos - neg) * scale[col / 128] * input[col];\n        }\n        out[row] = acc;\n    }\n    out // ~1.08× vs llama.cpp — scalar ALU is the bottleneck, not memory\n}".to_string(),
            opt_pattern: "// optimized: cmma hardware unit (Metal simdgroup_matrix_8x8 / tensor core) — 64 FMAs/cycle\nfn ternary_dot_cmma(pos_bits: &[u32], neg_bits: &[u32], scale: &[f32], input: &[f32], m: usize, n: usize) -> Vec<f32> {\n    // dequant weight tile into shared memory once (see dequant-once-per-k-tile rule)\n    // then cmma::execute consumes it at hardware rate — 64 FMAs per simdgroup per cycle\n    let mut out = vec![0.0f32; m];\n    const TILE: usize = 8; // cmma tile dim (simdgroup_matrix_8x8)\n    for tile_m in (0..m).step_by(TILE) {\n        for tile_n in (0..n).step_by(TILE) {\n            // dequant 8×8 weight tile into shared memory (cooperative, 32 threads × 2 elem)\n            let mut tile_w = [0.0f32; TILE * TILE];\n            let mut tile_x = [0.0f32; TILE * TILE];\n            for r in 0..TILE { for c in 0..TILE {\n                let col = tile_n + c;\n                if col < n {\n                    let pos = ((pos_bits[col / 32] >> (col % 32)) & 1) as f32;\n                    let neg = ((neg_bits[col / 32] >> (col % 32)) & 1) as f32;\n                    tile_w[r * TILE + c] = (pos - neg) * scale[col / 128];\n                    tile_x[r * TILE + c] = input[tile_m * n + col];\n                }\n            } }\n            // cmma::execute: acc += tile_w @ tile_x — runs on the hardware matrix unit\n            // (64 FMAs/cycle on Metal simdgroup; far beyond scalar ALU throughput)\n            for r in 0..TILE { for c in 0..TILE {\n                if tile_m + r < m { out[tile_m + r] += tile_w[r * TILE + c] * tile_x[r * TILE + c]; }\n            } }\n        }\n    }\n    out // structurally different hardware path — not occupancy-bound\n}".to_string(),
        },
        // ── dequant-once-per-k-tile-into-shared-memory — per-FMA dequant → once-per-tile (A) ─
        // Source: riir-gpu gemm_ternary_simdgroup_cubecl.rs (Issue 641). The
        // naive path dequantizes each weight element inline as it's consumed by
        // the dot product — O(m×n) dequant ops interleaved with O(m×n) FMAs. The
        // cmma variant dequantizes a whole 8×8 weight tile ONCE into a
        // shared-memory staging buffer, then the cmma unit consumes all 64
        // elements at hardware rate. The dequant cost is amortized across all
        // 64 cmma FMAs in the tile — 1 dequant per K-tile-column shared across
        // the entire M-row of the output. Generalizes to any quantized GEMM
        // (int4/int8/ternary) where the dequant is reusable across output rows:
        // stage the dequantized tile in shared memory once, let the hardware
        // matrix unit consume it. Distinct from `block-contiguous-weight-layout`
        // (memory layout for fewer global accesses) — this is about WHEN the
        // dequant runs (once-per-tile vs per-element) + WHERE the result lives
        // (shared memory vs re-fetch).
        KernelRule {
            rule_id: "dequant-once-per-k-tile-into-shared-memory".to_string(),
            gpu_concept: "dequantize a weight tile once into shared memory, let the hardware matrix unit consume it across all output rows that share the tile. RIDER (Batch 273, ml-explore/mlx @ bff10220 (staged affine_qmm_t_nax with split-k #4682), MIT -- the FETCH/STORE-SPLIT SOFTWARE PIPELINE face, this entry's FIRST rider): the host pins the dequant-once amortization (dequant a weight tile once into shared memory, the consumers share it); this face splits the BLOCK LOADER ITSELF into two phases and software-pipelines them where NO async copy exists. The source's own statement, verbatim (kernels/quantized_nax.h:1027-1030 at bff10220): '// QuantizedBlockLoader's thread mapping split into two steps: fetch() reads the packed words and the group's scale and bias for one BK step into registers,' + '// store() dequantizes them into threadgroup memory. Fetching step k + 1 before the MMAs of step k hides the device reads behind the matrix work.' -- struct QuantizedBlockLoaderStaged (:1039) holds 'uint32_t raw[n_words]; T scale; T bias;' in REGISTERS between the phases (fetch() reads packed words + the group's scale/bias; store() dequantizes into threadgroup memory through 'vec<T, 8>' chunks under 'static_assert(n_vals % 8 == 0, '8-value stores');'). THE DOUBLE-BUFFERED Ws + ONE BARRIER PER K STEP (the mainloop, :1114-1115 + :1194-1282): 'Weights use two threadgroup buffers with one barrier between K steps.' -- 'loader_w.fetch(k + BK);' fires BEFORE the MMAs of step k consume 'Ws + (it & 1) * tile_size', then 'loader_w.store(Ws + ((it + 1) & 1) * tile_size);' fills the alternate buffer and ONE 'threadgroup_barrier(mem_flags::mem_threadgroup);' closes the step; the buffer pair is 'threadgroup T Ws[(WN == 1 ? 2 : 1) * BN * BK_padded];' (:1603) -- at BN=64, BK_padded=72 the staged arm is 2*64*72*2 B = 18,432 B for 16-bit T. THE DTYPE-KEYED SMEM-BUDGET EXCLUSION (the honest anti-regime arm, in-kernel): 'static_assert(sizeof(T) == 2, 'Two tiles fit threadgroup memory for 16-bit T');' -- at fp32 the double buffer is 36,864 B over the 32 KB threadgroup budget (the 32 KB figure is PR-body provenance; the source pins it via sizeof(T) == 2), so fp32 is EXCLUDED from the staged arm entirely, and the host gate agrees (quantized.cpp:832): 'return transpose && aligned_n && (mode == 'affine') && (x.dtype() == float16 || x.dtype() == bfloat16) && (x.offset() % 8 == 0);' -- fp16/bf16 ONLY. REGIME (self-contained): quant-block loaders on backends with NO async copy (Metal), where dequant-once staging already pays and the block's register footprint is one extra fetch set. ANTI-REGIME: async-copy backends (the hardware ring owns the overlap -- cpasync-multistage-tile-pipeline's shape); dtypes whose double buffer busts the threadgroup budget (fp32 -- the exclusion arm); register-pressure-bound kernels (raw[n_words] + scale + bias must fit). DISTINCT-FROM (do not double-count): barrier-after-async-mma-before-staged-tile-reuse (gpu_12, B97) -- the verbatim one-barrier-per-K-step overlap, but the FORK: gpu_12 is the barrier LEGALITY contract ordering an ASYNC tensor-op consumer's in-flight reads (correctness there); THIS face is the LOADER STRUCTURE -- fetch/store split + register prefetch on a PLAIN mma consumer with no async copy (one barrier is cadence here, a race contract there); one-barrier-a-tile-prefetch-occupancy-pair (gpu_102) -- whole-attention-tile staging granularity + a 2-blocks/SM residency budget, no dtype arm; register-double-buffer-operand-pipeline (gpu_09) -- the register-fragment double buffer one level down; THIS's register prefetch (fetch k+1 into raw[n_words]) composes with it; cpasync-multistage-tile-pipeline (gpu_09) -- the CUDA async-copy S-stage ring; the ABSENCE of async copy is precisely why this face exists as an explicit software pipeline; j16-warp-fragment-direct-no-smem-tile (gpu_106) -- the anti-pole (no staging when no reuse); the fp32 arm here is a BUDGET refusal, not a reuse refusal; dequant-once-macrotile-stage-reuse-across-microtiles (gpu_53, B186) -- the host family's sibling (amortize across microtiles), same family span. Verified: mlx/backend/metal/kernels/quantized_nax.h:1027-1030, :1039, :1114-1115, :1119, :1194-1282, :1603 + mlx/backend/metal/quantized.cpp:832 at the pin bff10220. Batch 273 rider / Plan 277 / Research 310.".to_string(),
            ref_shape: "// naive: dequant each weight element inline as it's consumed by the dot\nfn quant_matmul_inline_dequant(w_packed: &[u8], x: &[f32], scale: &[f32], m: usize, n: usize, bits: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n {\n            // dequant EVERY element inline — O(m×n) dequant ops interleaved with O(m×n) FMAs\n            let byte = w_packed[col / (8 / bits)];\n            let code = (byte >> ((col % (8 / bits)) * bits)) & ((1 << bits) - 1);\n            let w = (code as f32 - 8.0) * scale[col / 32]; // int4 dequant inline\n            acc += w * x[col];\n        }\n        out[row] = acc;\n    }\n    out // dequant runs m×n times — once per (row, col) even though the weight is row-independent\n}".to_string(),
            opt_pattern: "// optimized: dequant the weight tile once into shared memory, cmma consumes it across all output rows\nfn quant_matmul_staged_dequant(w_packed: &[u8], x: &[f32], scale: &[f32], m: usize, n: usize, bits: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    const TILE: usize = 8;\n    for k_base in (0..n).step_by(TILE) {\n        // dequant ONE 8-element column tile into shared memory — runs ONCE per K-tile\n        let mut tile_w = [0.0f32; TILE];\n        for c in 0..TILE {\n            let col = k_base + c;\n            if col < n {\n                let byte = w_packed[col / (8 / bits)];\n                let code = (byte >> ((col % (8 / bits)) * bits)) & ((1 << bits) - 1);\n                tile_w[c] = (code as f32 - 8.0) * scale[col / 32];\n            }\n        }\n        // ALL m output rows consume the SAME dequantized tile — dequant cost amortized across m rows\n        for row in 0..m {\n            let mut acc = 0.0;\n            for c in 0..TILE {\n                if k_base + c < n { acc += tile_w[c] * x[row * n + k_base + c]; }\n            }\n            out[row] += acc;\n        }\n    }\n    out // dequant runs n/TILE × TILE = n times total (was m×n) — m× amortization\n}".to_string(),
        },
        // ── swap-large-axis-to-x-for-workgroup-distribution — (M,P) → (P,M) cube count (D) ─
        // Source: riir-gpu gemm_ternary_simdgroup_cubecl.rs (Issue 641) — mirrors
        // matmul_swap_ab_cubecl. When one GEMM axis (P = tokens) is much larger
        // than the other (M = output rows), the dispatch `CubeCount::Static(
        // ceil(M/TILE), ceil(P/TILE), 1)` under-utilizes the X-axis workgroup
        // distribution (few M-tiles). Swapping to `CubeCount::Static(
        // ceil(P/TILE), ceil(M/TILE), 1)` puts the large P-axis on X so the GPU's
        // workgroup scheduler spreads tiles across SMs evenly. Common in
        // attention projections where M (head_dim) is small but P (seq_len) is
        // large. Distinct from `l2-grouped-tile-schedule` (swizzle for cache) —
        // this is a static axis swap for workgroup COUNT distribution, not a
        // cache swizzle.
        KernelRule {
            rule_id: "swap-large-axis-to-x-for-workgroup-distribution".to_string(),
            gpu_concept: "when one GEMM axis is much larger, put it on the X (first) cube-count axis for better workgroup distribution across SMs".to_string(),
            ref_shape: "// naive: (M, P) cube count — small M-axis on X under-utilizes SM distribution\nfn dispatch_small_m_large_p(m: usize, p: usize, tile: usize) -> (u32, u32, u32) {\n    // M = 128 (small), P = 4096 (large) — only ceil(128/8) = 16 X-tiles\n    let x = ((m + tile - 1) / tile) as u32; // 16 — too few X workgroups\n    let y = ((p + tile - 1) / tile) as u32; // 512\n    (x, y, 1) // GPU scheduler can't spread 16 X-groups across 80+ SMs\n}".to_string(),
            opt_pattern: "// optimized: (P, M) cube count — large P-axis on X for even SM distribution\nfn dispatch_swap_large_axis_to_x(m: usize, p: usize, tile: usize) -> (u32, u32, u32) {\n    // SWAP: P on X, M on Y — mirrors matmul_swap_ab_cmma_f32's proven dispatch\n    let x = ((p + tile - 1) / tile) as u32; // 512 X-tiles — spreads across all SMs\n    let y = ((m + tile - 1) / tile) as u32; // 16\n    (x, y, 1) // work inside the kernel reads CUBE_POS_X as the P-tile, CUBE_POS_Y as the M-tile\n}".to_string(),
        },
        // ── norm-decoupled-low-bit-scoring — full-key dot → unit-key dot × norm (N) ─
        // Source: riir-gpu forward_turboquant.rs (Plan 048 Task 5). TurboQuant
        // stores each key vector as a UNIT-normalized quantized representation
        // (bit-packed indices into a codebook) PLUS a separate per-position L2
        // norm scalar (`key_norm[t]`). The attention score is reconstructed as
        // `norm[t] * dot(q_rotated, y_hat_quantized) * scale` — factoring the
        // norm out as a scalar multiplier. Because the quantized key is
        // unit-normalized, its dynamic range is bounded to [−1, +1] per element,
        // so low-bit quantization (2-4 bits) incurs minimal error. The naive
        // path stores the full-magnitude key and quantizes it directly — the
        // norm variation across positions forces higher bits to avoid clipping.
        // Decoupling the norm lets the same low-bit codebook serve all positions.
        // Distinct from `per-token-activation-quant` (per-token scale for int8)
        // — this factors the norm OUT of the stored representation entirely
        // (stored as a separate scalar) rather than folding a scale INTO the
        // quantized value.
        KernelRule {
            rule_id: "norm-decoupled-low-bit-scoring".to_string(),
            gpu_concept: "store per-vector L2 norm separately as a scalar; quantize the unit-normalized vector at lower bits; reconstruct the dot product as norm × dot(unit_q, unit_k_quantized)".to_string(),
            ref_shape: "// naive: store full-magnitude key, quantize directly — norm variation forces high bits\nfn score_full_magnitude_quant(q: &[f32], keys: &[f32], seq_len: usize, d_h: usize) -> Vec<f32> {\n    let mut scores = vec![0.0f32; seq_len];\n    for t in 0..seq_len {\n        // quantize the FULL key — positions with large norm need 8 bits to avoid clipping\n        let quantized_key = quantize_to_int8(&keys[t * d_h..(t + 1) * d_h]); // 8 bits — norm variation is large\n        let mut dot = 0.0;\n        for d in 0..d_h { dot += q[d] * dequant_int8(quantized_key[d]); }\n        scores[t] = dot;\n    }\n    scores // 8 bits/key — high memory cost because norm variation is baked into the quantization range\n}".to_string(),
            opt_pattern: "// optimized: store norm separately, quantize the UNIT-normalized key at 2-4 bits (riir-gpu TurboQuant)\nfn score_norm_decoupled_low_bit(q: &[f32], key_indices: &[u8], key_norm: &[f32], codebook: &[f32], seq_len: usize, d_h: usize, bits: usize) -> Vec<f32> {\n    let mut scores = vec![0.0f32; seq_len];\n    for t in 0..seq_len {\n        let norm = key_norm[t]; // separate per-position scalar (f32, exact)\n        // gather codebook centroids — the stored key is UNIT-normalized, so range ∈ [−1, +1]\n        // → 2 bits (4 centroids) suffice with minimal error\n        let mut dot = 0.0;\n        for d in 0..d_h {\n            let centroid = codebook[key_indices[t * d_h + d] as usize]; // y_hat[d]\n            dot += q[d] * centroid;\n        }\n        scores[t] = norm * dot; // reconstruct full score = norm × dot(unit_q, unit_k)\n    }\n    scores // 2 bits/key + 1 f32 norm/position — 4× less key memory than 8-bit full-magnitude\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 17 (Research 018): riir-gpu production kernels — ternary scalar-ALU
        // hot-loop: zero-word OR-skip, group-invariant scale hoist, split-sign
        // accumulators, clamp-tile-indices-to-sentinel. 4 rules from 2 riir-gpu
        // source files (the fourth batch mining the stack's own kernels).
        // ════════════════════════════════════════════════════════════════════
        // ── skip-all-zero-packed-words-via-or-reduce — per-bit guard → OR-reduce early-out (B) ──
        // Source: riir-gpu gemm_ternary_batched_cubecl.rs — the row-tiled batched
        // kernel OR-reduces the pos+neg bit-words across ALL rows in the output
        // tile before entering the 32-iteration sign-extraction loop:
        //   if (p0 | nw0 | p1 | nw1 | p2 | nw2 | p3 | nw3) != 0u32 { ... }
        // When the entire 32-bit word is zero across every row in the tile (which
        // is common in ternary — sparse weights produce many all-zero words), the
        // kernel skips the 8 weight loads + 32×8 sign-extraction ops + 8 scale
        // multiplies. This is a ternary/bit-packed-specific sparsity skip: the
        // OR-reduce is O(rows_in_tile) per word and gates a 32×(rows) work skip.
        // Distinct from `csr-index-via-prefix-sum` (build a sparse index offline)
        // and `block-sparse-gather` (iterate a pre-built index) — this is an
        // INLINE dynamic skip driven by the data itself, no pre-built index.
        KernelRule {
            rule_id: "skip-all-zero-packed-words-via-or-reduce".to_string(),
            gpu_concept: "OR-reduce all rows' pos+neg bit-words; skip the 32-iteration sign-extraction loop when the whole tile's word is zero (ternary sparsity exploit)".to_string(),
            ref_shape: "// naive: always enter the 32-bit extraction loop (no sparsity awareness)\nfn ternary_dot_no_skip(pos_bits: &[u32], neg_bits: &[u32], scale: &[f32], input: &[f32], m: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for row in 0..m {\n        let mut acc = 0.0;\n        for w in 0..n / 32 {\n            let p = pos_bits[row * (n / 32) + w];\n            let nw = neg_bits[row * (n / 32) + w];\n            // ALWAYS run 32 iterations even when p==0 && nw==0 (common in sparse ternary)\n            for b in 0..32 {\n                let col = w * 32 + b;\n                let pb = (p >> b) & 1;\n                let nb = (nw >> b) & 1;\n                acc += (pb as f32 - nb as f32) * scale[w / 4] * input[col];\n            }\n        }\n        out[row] = acc;\n    }\n    out // 32 iterations × m rows per word — even when the word is all-zero (wasted ALU)\n}".to_string(),
            opt_pattern: "// optimized: OR-reduce all tile rows' words; skip the loop when the whole tile's word is zero\nfn ternary_dot_or_skip(pos_bits: &[u32], neg_bits: &[u32], scale: &[f32], input: &[f32], m: usize, n: usize, rows_per_tile: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for tile in (0..m).step_by(rows_per_tile) {\n        for w in 0..n / 32 {\n            // OR-reduce all rows in the tile — if the word is zero across ALL rows, skip\n            let mut any_nonzero = 0u32;\n            for r in 0..rows_per_tile {\n                let row = tile + r;\n                if row < m {\n                    any_nonzero |= pos_bits[row * (n / 32) + w];\n                    any_nonzero |= neg_bits[row * (n / 32) + w];\n                }\n            }\n            if any_nonzero != 0 {\n                for r in 0..rows_per_tile {\n                    let row = tile + r;\n                    if row < m {\n                        let p = pos_bits[row * (n / 32) + w];\n                        let nw = neg_bits[row * (n / 32) + w];\n                        let mut acc = 0.0;\n                        for b in 0..32 {\n                            let col = w * 32 + b;\n                            let pb = (p >> b) & 1;\n                            let nb = (nw >> b) & 1;\n                            acc += (pb as f32 - nb as f32) * input[col];\n                        }\n                        out[row] += acc * scale[w / 4];\n                    }\n                }\n            } // else: skip 32×rows_per_tile sign extractions — ternary sparsity win\n        }\n    }\n    out // all-zero words skipped — O(rows) OR-reduce gates a 32×rows work skip\n}".to_string(),
        },
        // ── hoist-group-invariant-scale-out-of-word-loop — reload-per-word → cache-per-group (A) ──
        // Source: riir-gpu gemv_ternary_block_contiguous_cubecl.rs — the ternary
        // scale is shared across all 4 words in a 128-weight group (u32[0]=scale,
        // u32[1..5]=pos, u32[5..9]=neg). The kernel loads the scale ONCE per group
        // and reuses it across the 4 word iterations, reloading only when the
        // group boundary changes (every 4 words). GPU compilers often fail to
        // hoist buffer loads (pointer-aliasing / volatility concerns), so the
        // manual hoist matters. Distinct from `precompute-scalar-reduction`
        // (separate pre-pass + pass-in) — this caches a shared value IN-LOOP
        // across a fixed group of iterations via a register + boundary check,
        // not a separate dispatch.
        KernelRule {
            rule_id: "hoist-group-invariant-scale-out-of-word-loop".to_string(),
            gpu_concept: "cache a per-group invariant (e.g. quant scale shared across N words) in a register; reload only when the group boundary changes, not every iteration".to_string(),
            ref_shape: "// naive: reload the scale from the buffer every word iteration\nfn ternary_dot_reload_scale(block_buf: &[u32], input: &[f32], m: usize, groups_per_row: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    let words_per_row = groups_per_row * 4;\n    for row in 0..m {\n        let mut acc = 0.0;\n        for w in 0..words_per_row {\n            let g = w / 4;\n            // RELOAD scale every word — GPU compiler may not hoist (buffer aliasing)\n            let blk = (row * groups_per_row + g) * 9;\n            let scale = f32::from_bits(block_buf[blk]); // access every iteration (4× per group)\n            let p = block_buf[blk + 1 + (w % 4)];\n            let nw = block_buf[blk + 5 + (w % 4)];\n            for b in 0..32 {\n                let pb = (p >> b) & 1;\n                let nb = (nw >> b) & 1;\n                acc += (pb as f32 - nb as f32) * scale * input[w * 32 + b];\n            }\n        }\n        out[row] = acc;\n    }\n    out // scale loaded words_per_row times per row — 4× redundant global loads\n}".to_string(),
            opt_pattern: "// optimized: cache scale in a register; reload only when the group changes (every 4 words)\nfn ternary_dot_hoist_scale(block_buf: &[u32], input: &[f32], m: usize, groups_per_row: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    let words_per_row = groups_per_row * 4;\n    for row in 0..m {\n        let mut acc = 0.0;\n        let mut cached_g: u32 = u32::MAX; // force first-load\n        let mut cached_scale = 0.0f32;\n        for w in 0..words_per_row {\n            let g = w / 4;\n            let blk_base = (row * groups_per_row + g) * 9;\n            // RELOAD only when the group changes (every 4 words) — register-cached\n            if g != cached_g {\n                cached_scale = f32::from_bits(block_buf[blk_base]); // 1 load per group (was 4)\n                cached_g = g;\n            }\n            let p = block_buf[blk_base + 1 + (w % 4)];\n            let nw = block_buf[blk_base + 5 + (w % 4)];\n            for b in 0..32 {\n                let pb = (p >> b) & 1;\n                let nb = (nw >> b) & 1;\n                acc += (pb as f32 - nb as f32) * cached_scale * input[w * 32 + b];\n            }\n        }\n        out[row] = acc;\n    }\n    out // scale loaded groups_per_row times per row (was words_per_row = 4×groups) — 4× fewer global loads\n}".to_string(),
        },
        // ── split-sign-accumulators-defer-subtraction — per-bit subtract → once-at-end (B) ──
        // Source: riir-gpu gemm_ternary_batched_cubecl.rs + gemv_ternary_block_contiguous.
        // The ternary dot accumulates signed contributions: per bit, add
        // select(pos, x, 0) − select(neg, x, 0). The naive form computes the
        // subtraction INSIDE the 32-iteration bit loop (32 subtracts per word).
        // The optimized form keeps separate pos/neg accumulators (a_p +=
        // select(pos, x, 0); a_n += select(neg, x, 0)) inside the loop and does
        // ONE subtraction (a = a_p − a_n) after the loop completes — reducing
        // subtractions from 32/word to 1/word. Also reduces dependent-op latency
        // (the per-bit subtract serializes the add chain; split accumulators are
        // independent). Distinct from `upcast-before-reduction` (precision) —
        // this is about instruction count + ILP in a ternary sign-accumulation.
        KernelRule {
            rule_id: "split-sign-accumulators-defer-subtraction".to_string(),
            gpu_concept: "in a signed ternary/bit-packed dot loop, keep separate pos and neg accumulators; subtract once at the end instead of per-bit (fewer subtractions + independent accumulator chains)".to_string(),
            ref_shape: "// naive: subtract pos-neg inside the bit loop (32 subtractions per word)\nfn ternary_dot_subtract_per_bit(pos_bits: &[u32], neg_bits: &[u32], input: &[f32], n: usize) -> f32 {\n    let mut acc = 0.0;\n    for w in 0..n / 32 {\n        let p = pos_bits[w];\n        let nw = neg_bits[w];\n        for b in 0..32 {\n            let pb = ((p >> b) & 1) as f32;\n            let nb = ((nw >> b) & 1) as f32;\n            // SUBTRACT inside the loop — 32 dependent subtracts per word\n            acc += (pb - nb) * input[w * 32 + b];\n        }\n    }\n    acc // 32 subtracts/word + each subtract is dependent on the prior add\n}".to_string(),
            opt_pattern: "// optimized: split pos/neg accumulators; subtract once at the end\nfn ternary_dot_split_accumulators(pos_bits: &[u32], neg_bits: &[u32], input: &[f32], n: usize) -> f32 {\n    let mut acc_p = 0.0; // positive contributions only\n    let mut acc_n = 0.0; // negative contributions only\n    for w in 0..n / 32 {\n        let p = pos_bits[w];\n        let nw = neg_bits[w];\n        for b in 0..32 {\n            let x = input[w * 32 + b];\n            // two independent select-adds — NO subtract inside the loop\n            acc_p += ((p >> b) & 1) as f32 * x; // pos accumulator\n            acc_n += ((nw >> b) & 1) as f32 * x; // neg accumulator (indep chain)\n        }\n    }\n    acc_p - acc_n // ONE subtract at the end — 32→1 per word; acc_p/acc_n are independent ILP chains\n}".to_string(),
        },
        // ── clamp-tile-indices-to-sentinel-discard-at-write — per-lane bounds check → alias + discard (B) ──
        // Source: riir-gpu gemm_ternary_batched_cubecl.rs — the row-tiled kernel
        // owns a 4-row output tile per plane. When m is not a multiple of 4,
        // out-of-range rows are CLAMPED to alias row m-1 (a valid sentinel),
        // the full computation runs on the aliased data, and the clamped results
        // are discarded at write time (guarded by `if row_base + k < m`). This
        // keeps the 32-iteration bit-extraction loop UNIFORM across the plane —
        // no per-lane divergence from bounds checks. Distinct from
        // `ragged-chunk-pad` (pad with zeros → wasted work processing zeros) —
        // this aliases to real data (the work is real but discarded) so the loop
        // body stays divergence-free without the overhead of processing padding.
        // Also distinct from `masked-boundary-load` (per-element OOB→fallback) —
        // this clamps the INDEX so the load is real, avoiding per-element branches.
        KernelRule {
            rule_id: "clamp-tile-indices-to-sentinel-discard-at-write".to_string(),
            gpu_concept: "when a tile extends past the matrix edge, alias OOB rows to a valid sentinel row and discard the redundant results at write time — keeps the loop body uniform (no per-lane divergence) without padding overhead".to_string(),
            ref_shape: "// naive: per-element bounds check inside the hot loop (per-lane divergence)\nfn gemm_tile_per_element_check(pos: &[u32], neg: &[u32], input: &[f32], m: usize, n: usize, rows_per_tile: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for tile_start in (0..m).step_by(rows_per_tile) {\n        for r in 0..rows_per_tile {\n            let row = tile_start + r;\n            for col in 0..n {\n                // PER-ELEMENT bounds check — causes divergence when some lanes are OOB\n                if row < m {\n                    let p = pos[row * (n / 32) + col / 32];\n                    let pb = (p >> (col % 32)) & 1;\n                    out[row] += pb as f32 * input[col]; // only valid rows write\n                }\n            }\n        }\n    }\n    out // per-lane bounds check inside the loop — divergent when tile extends past m\n}".to_string(),
            opt_pattern: "// optimized: alias OOB rows to m-1 (sentinel); compute fully; discard at write\nfn gemm_tile_clamp_sentinel(pos: &[u32], neg: &[u32], input: &[f32], m: usize, n: usize, rows_per_tile: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    let last = m - 1; // sentinel row\n    for tile_start in (0..m).step_by(rows_per_tile) {\n        // CLAMP row indices — OOB rows alias the sentinel (valid, no divergence)\n        let r0 = tile_start;\n        let r1 = if tile_start + 1 < m { tile_start + 1 } else { last };\n        let r2 = if tile_start + 2 < m { tile_start + 2 } else { last };\n        let r3 = if tile_start + 3 < m { tile_start + 3 } else { last };
        let mut acc0 = 0.0;\n        let mut acc1 = 0.0;\n        let mut acc2 = 0.0;\n        let mut acc3 = 0.0;\n        for col in 0..n {\n            // UNIFORM loop body — all 4 rows are valid indices (no per-lane check)\n            acc0 += ((pos[r0 * (n / 32) + col / 32] >> (col % 32)) & 1) as f32 * input[col];\n            acc1 += ((pos[r1 * (n / 32) + col / 32] >> (col % 32)) & 1) as f32 * input[col];\n            acc2 += ((pos[r2 * (n / 32) + col / 32] >> (col % 32)) & 1) as f32 * input[col];\n            acc3 += ((pos[r3 * (n / 32) + col / 32] >> (col % 32)) & 1) as f32 * input[col];\n        }\n        // DISCARD redundant results at write time (guarded writes only)\n        out[r0] = acc0;\n        if tile_start + 1 < m { out[r1] = acc1; }\n        if tile_start + 2 < m { out[r2] = acc2; }\n        if tile_start + 3 < m { out[r3] = acc3; }\n    }\n    out // loop body uniform (no divergence) — OOB work is real-but-discarded, not zero-padded\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 18 (Research 019): 3 riir-gpu production rules — fifth batch
        // mining the stack's own kernels, focused on the Q4_K quantized GEMV
        // family (4-bit block-quant format with per-block scale + min).
        // Source: riir-gpu gemv_q4k_cubecl.rs — 3 of 4 kernel variants yielded
        // rules; the 4th (gemv_q4k_tiled) is standard shared-memory tiling.
        // Three sibling Q4_K files (qkv, geglu, batched) assessed + dropped:
        // qkv = composition of fuse-gate-up-gemv-activation (triple fusion);
        // geglu = same fuse pattern + Q4_K dequant; batched = no new opt.
        // ════════════════════════════════════════════════════════════════════
        // ── row-tile-amortize-input-load — 1 row/plane → N rows/plane (A) ──
        // Source: riir-gpu gemv_q4k_cubecl.rs `gemv_q4k_plane_rowtiled` (Issue
        // 609). The baseline plane-cooperative GEMV assigns one output row per
        // plane — each input element is loaded from global memory once PER ROW.
        // Row-tiling assigns N consecutive output rows per plane
        // (Q4K_ROWS_PER_PLANE=4), holds N register accumulators, and loads
        // each input element ONCE, reusing it across all N dot products via
        // `acc[r] += w[r][col] * x[col]`. This is the fundamental GEMV →
        // small-GEMM transformation: widening M per work-group converts a
        // memory-bound GEMV into a compute-bound small matrix multiply where
        // input bandwidth is amortized across N output rows. The gain scales
        // with the input-load-to-compute ratio: ternary (~9 ALU/load) gets
        // ~2×; Q4_K (~15 ALU/load: 6-bit scale decode + nibble extract) gets
        // ~1.1-1.3× (Issue 609 honest expectation). Also seen in
        // gemm_ternary_batched_cubecl as the substrate for OR-reduce +
        // sentinel-clamp. Distinct from `gemm-tiling` (shared-memory tile size
        // for large GEMM) — this is M-widening per PLANE for cooperative GEMV.
        // Also distinct from `dequant-once-per-k-tile-into-shared-memory`
        // (amortize the DEQUANT across rows via shared mem) — this amortizes
        // the INPUT LOAD across rows via register reuse, a different resource.
        KernelRule {
            rule_id: "row-tile-amortize-input-load".to_string(),
            gpu_concept: "assign N consecutive output rows per plane (subgroup), hold N register accumulators, load each input element once and reuse across all N dot products — GEMV to small-GEMM via M-widening. RIDER (Batch 108, riir-ai M3 campaign / Plan 078, the CMMA STAGED-GEMM IN-STACK INSTANCE): the same mechanism at the tensor-core tile grain — the 32x32 staged GEMM stages each input tile ONCE and consumes it across 4 M-subtiles (4 independent accumulator sets), cutting input traffic 4x; the input-load share measured 42-63% of the whole kernel (91.2 GB/GEMM of input traffic at the FFN shape), the largest single cost axis; regime m >= 64 (measured 0.773x at m=48 — padding dominates small m, stay on the older arms); bit-identity argument: execute order WITHIN a k-tile is free across distinct accumulators, per-accumulator order across k-tiles unchanged (riir-ai @ 59a172623 gemm_ternary_simdgroup_cubecl.rs:938-940; the untried-axis note at :563-565 — every prior variant widened P or restructured K, none increased M-rows-per-workgroup; Issue 768 / Bench 774: 32x32 promoted default-ON, e2e 1.41-1.53x). RIDER (Batch 276, the MEASURED ROW-COUNT OPTIMUM face, Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ 59160bd (MIT), intro commit 1b0ee62 -- where this host and its Batch-108 rider amortize the input LOAD across M-subtiles at the tile grain, this face amortizes the per-timestep cost that does NOT scale with the row count (the lane's four q and four k loads, the two gate loads, pointer arithmetic, the loop branch) across VALUE ROWS PER SIMDGROUP in a sequential scan recurrence -- and the row count is a MEASURED optimum, not a knob: 'FOUR ROWS PER SIMDGROUP instead of two (-10.9 %). The per-timestep' (Runner/FastModel/TrackFastKernels.swift:461) -- 4 > 2 > 8 > 16 rows per tile, because 'worse (-9.1 %) and sixteen much worse: past four, the shrinking' 'thread count stops covering the memory latency.' (:467-468), each row's own arithmetic sequence untouched so bit-identity is by construction. Two censuses carry the admission. The non-spill census: 'maxTotalThreadsPerThreadgroup == 1024, so nothing spills and nothing' 'trades occupancy for registers' (:449-450) across every variant up to eight rows -- occupancy is never traded for registers, so the win is real latency coverage, not spill suppression. The instruction-issue-bound census (the method that separates issue-bound from bandwidth-bound): count the body's LANE-OPS, 'gives ~118 per lane per timestep, and' '98 304 x 1 024 x 118 x 36 layers / (40 cores x 128 ALUs x 1.4 GHz) is' '59.7 ms against 58.8 ms measured. The kernel is INSTRUCTION-ISSUE BOUND' 'at ~100 % of the machine's issue rate' (:452-455) -- model within 1.5% of measurement against a byte-identical control arm '(noise floor 1 %)' (:459), so the only remaining bit-exact lever is 'faster while staying bit-exact is issuing fewer instructions.' (:456). Batch 276 / Plan 280 / Research 314. RIDER (Batch 276, the GATHER-QMM INPUT-STAGE face, Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ 59160bd (MIT), intro commit 9f6a0ac -- where this host and its Batch-108 CMMA rider own stage-input-once at the tile grain, this face owns the SIMDGROUP-PAIR duplicate-read diagnosis below it: 'WM = WN = 2 the pairs (0,1) and (2,3) issue IDENTICAL reads, so every' 'A row of the tile is fetched twice' (quantized_nax.h:1746-1747), and the fix stages A ONCE cooperatively -- '128-byte line per row. It rides the barrier pair Ws already needs, so' 'it adds no synchronization' (:1750-1751) -- the input stage rides the EXISTING weight-stage barriers, zero added synchronization. The feed chain ships as four labeled faces: APREFETCH, the A-side register prefetch one K block ahead -- 'MLXFAST-APREFETCH: the A block is fetched into registers one K block' 'ahead, so its device latency is hidden behind the current block's MMAs' with '16 bf16 per thread, statically indexed, so it stays in' registers (:1711-1715); AVEC, the uint4 vector stage copies -- 'MLXFAST-AVEC: a_col is a multiple of 16 elements (32 bytes), K is' '16-byte aligned and the 16 elements move as two whole vectors.' (:1718-1720, landed 8981cef); the dead-row zero-fill HOISTED out of the K loop -- 'A dead row's slice of the activation stage is zero for every K step:' so the fill is written once, not once per step (:1734-1736, same commit); and WVEC8, the widened dequant stores -- 'One `words[j]` holds the four bytes that dequantise into values' '`8j` through `8j + 7`, so the group is four 16-byte stores instead of' 'eight 8-byte ones.' (Runner/FastModel/TrackPrefillIndirectMetal.swift:1355-1357, landed 6fbe0e4; the vec4 MLXFAST-WVEC form rides quantized_nax.cpp:878-880). The cost admission is measured in situ: 'MLXFAST-ASTAGE: 32 x 72 bf16 = 4,608 B, only for the P17-eligible shape. An' 'in-situ probe that added exactly this much untouched threadgroup memory to' 'this kernel cost 0.8%, so the allocation is close to free here.' (quantized_nax.h:1854-1856). Kinship mentions only, no double-ride: `vectorized-smem-write-no-bank-padding` (entries_gpu_07.rs -- these stage stores are vectorized, so bank padding buys nothing) and `dequant-once-per-k-tile-into-shared-memory` (the WEIGHT-side stage whose barriers this input stage borrows). Batch 276 / Plan 280 / Research 314.".to_string(),
            ref_shape: "// naive: 1 output row per plane — input[col] re-loaded per row\nfn gemv_one_row_per_plane(weight: &[f32], input: &[f32], m: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for row in 0..m {\n        let mut acc = 0.0;\n        for col in 0..n {\n            acc += weight[row * n + col] * input[col]; // input[col] loaded m times total\n        }\n        out[row] = acc;\n    }\n    out // m * n input loads — memory-bound (input bandwidth = bottleneck)\n}".to_string(),
            opt_pattern: "// optimized: N rows per plane — input[col] loaded once, reused across N accumulators\nfn gemv_row_tiled(weight: &[f32], input: &[f32], m: usize, n: usize, rows_per_tile: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; m];\n    for tile_start in (0..m).step_by(rows_per_tile) {\n        let r0 = tile_start;\n        let r1 = if tile_start + 1 < m { tile_start + 1 } else { m - 1 }; // clamp OOB\n        let mut acc0 = 0.0;\n        let mut acc1 = 0.0;\n        for col in 0..n {\n            let x = input[col]; // ONE load — reused by both accumulators in registers\n            acc0 += weight[r0 * n + col] * x;\n            acc1 += weight[r1 * n + col] * x;\n        }\n        out[r0] = acc0;\n        if tile_start + 1 < m { out[r1] = acc1; }\n    }\n    out // n input loads total (was m * n) — rows_per_tile x fewer global reads\n}".to_string(),
        },
        // ── factor-affine-quant-scale-out-of-element-loop — per-element scale apply to per-sub-block (B) ──
        // Source: riir-gpu gemv_q4k_cubecl.rs `gemv_q4k_plane_factored` (Issue
        // 609 T2 — the bigger lever). Q4_K dequant per element is:
        //   value = d * sc * nibble - dmin * min
        // Within a 32-element sub-block, `sc` and `min` are CONSTANT (decoded
        // once from the 6-bit scale words), so the dequant is AFFINE in the
        // nibble. Algebraic factoring lifts d*sc and dmin*min out of the
        // per-element accumulation:
        //   sum_j (d*sc*nib_j - dmin*min) * x_j = d*sc*(sum_j nib_j*x_j) - dmin*min*(sum_j x_j)
        // This replaces ~22 ops/element (2 mul + 1 sub + 2 scale decodes + FMA)
        // with ~7 ops/element + one FMA for nib*x, one add for x — the scale
        // decode runs once per sub-block (was per-element). When combined with
        // row-tiling, sum(x_j) is row-independent and computed once per block,
        // reused across all N rows (only sum(nib_j*x_j) is per-row). This is
        // Q4_K/min+scale-quant-specific: ternary dequant (sign * scale) is
        // LINEAR in the bits (no min offset), so there is nothing to factor.
        // Distinct from `fold-norm-scalar-into-matmul` (fold an EXTERNAL scalar
        // into the dot) — this factors an INTERNAL affine expression out of
        // the per-element dequant before it enters the dot.
        KernelRule {
            rule_id: "factor-affine-quant-scale-out-of-element-loop".to_string(),
            gpu_concept: "when the dequant formula is affine in the quantized code within a sub-block (scale and min are constant), factor d*scale and dmin*min out of the per-element accumulation — accumulate sum(code*x) and sum(x) separately, apply scales once per sub-block".to_string(),
            ref_shape: "// naive: apply d*sc and dmin*min per element inside the dot loop\nfn q4k_dot_naive(input: &[f32], d: f32, dmin: f32, sc: f32, min_val: f32, nibbles: &[u32], n: usize) -> f32 {\n    let mut acc = 0.0;\n    for j in 0..n {\n        // per-element: 2 mul (d*sc) + 1 sub (dmin*min) + FMA = ~22 ops\n        let dequant = d * sc * (nibbles[j] as f32) - dmin * min_val;\n        acc += dequant * input[j]; // scale applied EVERY element (sc is constant!)\n    }\n    acc // sc/min decoded once but d*sc*nib - dmin*min computed n times\n}".to_string(),
            opt_pattern: "// optimized: factor d*sc and dmin*min out — accumulate sum(nib*x) and sum(x) separately\nfn q4k_dot_factored(input: &[f32], d: f32, dmin: f32, sc: f32, min_val: f32, nibbles: &[u32], n: usize) -> f32 {\n    let mut nib_dot = 0.0; // sum(nib_j * x_j) — the only per-row part\n    let mut sum_x = 0.0;   // sum(x_j) — row-independent when row-tiling\n    for j in 0..n {\n        nib_dot += (nibbles[j] as f32) * input[j]; // ~7 ops: FMA only\n        sum_x += input[j];                         // ~1 add\n    }\n    // apply scales ONCE per sub-block (was per-element)\n    d * sc * nib_dot - dmin * min_val * sum_x\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 19 (Research 020): 2 riir-gpu production rules — sixth batch
        // mining the stack's own kernels, focused on the fused-dispatch family
        // (command-buffer-level fusion + in-place residual bypass).
        // Source: riir-gpu ternary_ffn_fused.rs + gemv_ternary_residual_cubecl.rs.
        // Three sibling files assessed + dropped: deltanet_input_proj_fused
        // (same pattern as ternary_ffn_fused — confirms generality); deltanet
        // chunked + base recurrence (covered by recurrent-vs-chunk-dispatch).
        // ════════════════════════════════════════════════════════════════════
        // ── chain-dispatches-single-sync-point — N dispatches + N syncs → N dispatches + 1 sync (C) ──
        // Source: riir-gpu ternary_ffn_fused.rs (Issue 600). The fused ternary
        // FFN chains 4 dispatches — gate GEMV + up GEMV + SwiGLU + down GEMV —
        // into a single GPU command buffer, eliminating the per-matvec CPU↔GPU
        // sync that made the naive Issue 599 integration slower than CPU. All
        // 4 dispatches are queued WITHOUT `read_one`; only the final output is
        // read back — ONE GPU sync point instead of 4. At most 2 intermediate
        // buffers are live at any point (the dependency chain is linear:
        // gate+up → SwiGLU → inter → down → out), so the command scheduler can
        // reuse buffer memory between dependent dispatches.
        //
        // **Key applicability constraint:** works when the dependency chain is
        // LINEAR (max N_live_intermediates small). The counterexample is
        // Bench 437's failed deferred-readback: it kept Q/K/V/gate/up outputs
        // ALL alive simultaneously (5 concurrent buffers), which INCREASED
        // Metal memory pressure → 16% slower. Fan-out N independent
        // intermediates consumed later is the known-bad case.
        //
        // Distinct from `fuse-gate-up-gemv-activation` (single-KERNEL fusion
        // collapsing N ops into 1 kernel — saves dispatch overhead AND global
        // memory bandwidth; this is command-BUFFER-level fusion keeping N
        // kernels but 1 sync point — saves dispatch overhead only, applies
        // when ops can't legally collapse into one kernel: different
        // work-group shapes, different weight tensors, or independent ops).
        KernelRule {
            rule_id: "chain-dispatches-single-sync-point".to_string(),
            gpu_concept: "queue N dependent GPU dispatches in one command buffer, sync only once at the end (not after each dispatch) — linear dependency chains keep intermediate buffer count bounded".to_string(),
            ref_shape: "// naive: sync after each dispatch (N sync points, N CPU↔GPU round-trips)
fn ffn_naive(weight_gate: &[f32], weight_up: &[f32], weight_down: &[f32], x: &[f32], m: usize, n: usize) -> Vec<f32> {
    let gate = gemv_dispatch_sync(weight_gate, x, m, n); // dispatch 1 + read_one sync (~55-97µs)
    let up = gemv_dispatch_sync(weight_up, x, m, n);     // dispatch 2 + read_one sync
    let inter: Vec<f32> = gate.iter().zip(up.iter()).map(|(g, u)| silu(*g) * u).collect(); // CPU elementwise
    gemv_dispatch_sync(weight_down, &inter, n, m)         // dispatch 3 + read_one sync — 3 sync points total
}".to_string(),
            opt_pattern: "// optimized: queue all dispatches in one command buffer, sync ONCE at the end
fn ffn_deferred_sync(weight_gate: &[f32], weight_up: &[f32], weight_down: &[f32], x: &[f32], m: usize, n: usize) -> Vec<f32> {
    let gate_h = empty_buf(m);  let up_h = empty_buf(m);  let inter_h = empty_buf(m);  let out_h = empty_buf(n);
    gemv_dispatch_noread(weight_gate, x, gate_h.clone()); // dispatch 1 (no sync)
    gemv_dispatch_noread(weight_up,   x, up_h.clone());   // dispatch 2 (no sync — gate+up can run back-to-back)
    swiglu_dispatch(gate_h, up_h, inter_h.clone());        // dispatch 3 (consumes gate+up, frees them)
    gemv_dispatch_noread(weight_down, inter_h, out_h.clone()); // dispatch 4 (no sync)
    read_one(out_h)                                        // ONE sync point — at most 2 intermediates live at once
}".to_string(),
        },
        // ── fuse-residual-via-in-place-output-array — 6th array impossible → alias output as residual input (C) ──
        // Source: riir-gpu gemv_ternary_residual_cubecl.rs (Issue 616).
        // Single-dispatch kernel that computes
        //   residual_output[row] += dot(weight_row, input)
        // for ternary bit-plane weights. Collapses 2 dispatches × 64 layers =
        // 128 dispatches/token into 64 dispatches/token (saves one Metal kernel
        // launch ~55-97µs each).
        //
        // CubeCL v0.10 has a hard **5 Array parameter limit** per kernel. The
        // plain ternary GEMV already uses 5 (pos_bits, neg_bits, scales, input,
        // output). Adding a 6th `residual` array is impossible. The fix: make
        // the kernel in-place — the residual lives in the output buffer and is
        // overwritten by GEMV result + residual. Same 5 args, but the output
        // buffer is read-then-written. Lane 0 of each plane reads
        // `residual_output[row]` before writing
        // `residual_output[row] = residual[row] + gemv_result[row]`.
        //
        // General pattern: when a kernel needs N+1 array parameters but the
        // runtime limits to N, and one of the N outputs is the accumulation
        // target of the (N+1)th input, alias the output to the accumulation
        // input (read-then-write semantics).
        //
        // Distinct from `fuse-add-rms-norm` (Batch 9 — fuses add+norm into
        // one PASS for bandwidth savings); this fuses GEMV+add by ALIASING
        // the output buffer as the residual input to bypass a hard
        // parameter-count constraint — the mechanism is in-place aliasing,
        // not pass-count reduction.
        KernelRule {
            rule_id: "fuse-residual-via-in-place-output-array".to_string(),
            gpu_concept: "when a kernel needs N+1 array parameters but the runtime limits to N, and the output is the accumulation target of the extra input, alias the output buffer as the accumulation input (read-then-write) — bypasses parameter count limits without adding a dispatch. RIDER (Batch 251, yukon cudafast-bonsai2-9b-engine @ 36c9536 -- the stream-K completion face): the residual fold survives NON-DETERMINISTIC stream-k segment boundaries because the fixup kernel combines prior partials in the SAME order before adding the residual -- 'The partial sums of the preceding blocks are combined in the same order into the same `sum`, the completed value is dst + sum (what the plain fixup stores to dst), and dst_add receives that value plus the residual.' (mmq-pq2-j128.cuh:643-645); the folded store issues every residual load before its first store ('All residual loads are issued before the first store: out may be res itself (an in-place ADD), and each element is read and written by this thread only.' :440-443), the segment contract names the overlap ('A segment that runs its whole tile (K blocks 0..nkb) writes dst_add = acc + res; the first segment of a stream-k tile that another block started writes its partial sum to dst as before, and mul_mat_q_pq2_j128_fixup_add completes that tile into dst_add. res and dst_add have dst's layout (stride_col_dst). dst_add may be dst or res; it overlaps nothing else.' :508-511), and the L2 prefetch rides the same starts ('Issued when a segment that adds the residual starts.' :470-471).RIDER (Batch 275, the PROMPT-RESIDUAL-FOLD BITWISE-GATE face, Layr-Labs/mlxfast-bonsai2-9b-engine @ 8cee3ef (MIT), intro commit 9b29449 -- the residual-in-epilogue fold on the MLX int8 prompt path, admitted by a bitwise gate: `PromptResidualFold.matmulFold` folds the residual add into the prompt down-projection matmul epilogue; the admission contract, verbatim: 'A fold serves only after' '`checkFold` found the fused output equal, bit for bit, to the record' 'launch's output plus the residual; otherwise the record launches stay.' -- the gate compares raw uint32 views, and the kill switch is env-keyed: '`DARKBLOOM_BONSAI_PREFILL_RESFOLD=0` keeps them.' (the record launches). Where this host owns the in-place output-array alias, the 9B face owns the ADMISSION half: fold is a verdict, never a default (the cudafast B251 stream-K rider already covers that repo's residual fold; this is the MLX sibling's gate). Batch 275 / Plan 279 / Research 313.".to_string(),
            ref_shape: "// naive: separate GEMV dispatch + ResidualAdd dispatch (2 sync points, 6th array needed for residual)
fn gemv_then_residual_dispatch(weight: &[f32], input: &[f32], residual: &[f32], m: usize, n: usize) -> Vec<f32> {
    let gemv_out = gemv_dispatch_sync(weight, input, m, n); // dispatch 1: 5 arrays (weight, input, output)
    // dispatch 2: residual_add(residual, gemv_out) — needs gemv_out + residual as INPUTS (6th array)
    residual.iter().zip(gemv_out.iter()).map(|(r, g)| r + g).collect() // 2 dispatches × 64 layers = 128/token
}".to_string(),
            opt_pattern: "// optimized: single in-place kernel — output buffer aliases the residual input
fn gemv_residual_inplace(weight: &[f32], input: &[f32], residual_output: &mut [f32], m: usize, n: usize) {
    // SAME 5 arrays as plain GEMV (weight, input, output) — output is read-then-written
    for row in 0..m {
        let mut acc = 0.0;
        for col in 0..n { acc += weight[row * n + col] * input[col]; }
        // LANE 0: read residual_output[row] THEN write residual + gemv_result
        residual_output[row] = residual_output[row] + acc; // in-place: output doubles as residual input
    }
    // 1 dispatch × 64 layers = 64/token (was 128) — no 6th array needed
}".to_string(),
        },
        // ── pin-lane-to-quant-sub-block-coalesce-byte-loads — strided to lane-fixed (A) ──
        // Source: riir-gpu gemv_q4k_cubecl.rs `gemv_q4k_plane_factored` (Issue
        // 609 T2). The naive strided lane-element map (k = lane + i*PLANE_DIM)
        // makes sub_block = i, so every lane visits all 8 sub-blocks and its
        // 8 needed qs bytes scatter across 8 different u32 words — 8 global
        // loads per lane per block, with 4x redundant loads (lanes 0-3 all
        // read word bo+4). The lane-fixed map pins each lane to ONE sub-block:
        //   lane -> sub_block = lane/4, chunk = lane%4, covering 8 consecutive elements
        // Across 32 lanes that is 8*4*8 = 256 = exact block partition. The 8
        // consecutive bytes a lane needs (pair*32 + chunk*8 .. +8) live in
        // exactly 2 consecutive u32 words — 8 loads -> 2 loads per lane. This
        // also ENABLES the affine factoring above: because each lane is pinned
        // to one sub_block, sc and min are loop-invariant and can be factored
        // out. The nibble-select shift folds: sh = is_high * 4 makes low/high
        // nibble a uniform shift, no branch. Distinct from `matvec-coalesce`
        // (transpose A for coalesced column reads) — this remaps the lane-element
        // assignment within a quant block, not the matrix layout.
        KernelRule {
            rule_id: "pin-lane-to-quant-sub-block-coalesce-byte-loads".to_string(),
            gpu_concept: "remap the lane-element assignment so each lane is pinned to one quantization sub-block — the 8 consecutive bytes it needs fall in 2 consecutive u32 words (8 loads to 2), and scale/min become loop-invariant (enabling affine factoring)".to_string(),
            ref_shape: "// naive: strided lane map — each lane visits all 8 sub-blocks, 8 scattered loads\nfn q4k_strided_lane(weight: &[u32], input: &[f32], lane: u32, plane_dim: u32, bo_q4k: u32) -> f32 {\n    let mut acc = 0.0;\n    let mut k = lane;\n    while k < 256 {\n        let sub_block = k / 32; // changes every iteration — sc/min NOT invariant\n        let pos = k % 32;\n        // 8 different qs_word_idx values per lane -> 8 scattered global loads\n        let qs_byte_idx = (sub_block / 2) * 32 + pos;\n        let qs_word = weight[bo_q4k + 4 + qs_byte_idx / 4]; // lanes 0-3 re-read word bo+4\n        acc += ((qs_word >> ((qs_byte_idx % 4) * 8)) & 0xF) as f32 * input[k];\n        k += plane_dim;\n    }\n    acc // 8 loads/lane, 4x redundant across lanes 0-3\n}".to_string(),
            opt_pattern: "// optimized: lane-fixed map — pin each lane to one sub-block, 8 bytes in 2 words\nfn q4k_lane_fixed(weight: &[u32], input: &[f32], lane: u32, bo_q4k: u32) -> f32 {\n    let sub_block = lane / 4;  // CONSTANT for this lane — sc/min are loop-invariant!\n    let chunk = lane % 4;\n    let pair = sub_block / 2;\n    let is_high = sub_block % 2;\n    let word0 = pair * 8 + chunk * 2; // 8 consecutive bytes -> exactly 2 u32 words\n    let sh = is_high * 4;             // nibble shift folds (no branch)\n    let w0 = weight[bo_q4k + 4 + word0];     // load 1\n    let w1 = weight[bo_q4k + 4 + word0 + 1]; // load 2 (the ONLY 2 loads)\n    let elem_off = sub_block * 32 + chunk * 8;\n    (((w0 >> sh) & 15) as f32) * input[elem_off]\n        + (((w0 >> (8 + sh)) & 15) as f32) * input[elem_off + 1]\n        + (((w0 >> (16 + sh)) & 15) as f32) * input[elem_off + 2]\n        + (((w0 >> (24 + sh)) & 15) as f32) * input[elem_off + 3]\n        + (((w1 >> sh) & 15) as f32) * input[elem_off + 4]\n        + (((w1 >> (8 + sh)) & 15) as f32) * input[elem_off + 5]\n        + (((w1 >> (16 + sh)) & 15) as f32) * input[elem_off + 6]\n        + (((w1 >> (24 + sh)) & 15) as f32) * input[elem_off + 7]\n    // 2 loads total (was 8) — no inner loop, exact 256-element partition across 32 lanes\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 20 (Research 021) — CUDA hardware-instruction cluster +
        // CubeCL LUT + base-3 trit (5 rules: 1A + 3B + 1N)
        // ════════════════════════════════════════════════════════════════════
        // ── cooperative-block-wide-quantize-staging — per-call quantize dispatch → block-wide cooperative shared-memory staging (A) ──
        // Source: riir-gpu gemv_ternary_cuda_raw.rs `gemv_ternary_dp4a_fused`
        // (Issue 616 T4). The split-kernel path quantizes f32 activations to
        // int8 in a SEPARATE kernel launch (1 extra dispatch + global-memory
        // round-trip per forward). The fused variant has all 256 threads in
        // the block cooperate to quantize the full N-element activation vector
        // into shared memory ONCE (thread t handles blocks t, t+256, ...,
        // each block = 16 elements), then each warp runs its dp4a loop
        // reading from shared memory. Per-block absmax + scale write is
        // conflict-free (each block owned by exactly one thread). After
        // __syncthreads(), the dp4a loop reads 4 int words from shared memory
        // per inner iteration (vs global memory in the split path).
        //
        // Distinct from `dequant-once-per-k-tile-into-shared-memory` (Batch
        // 16) — that's per-K-tile WEIGHT dequant (each tile's dequant is
        // independent); this is block-wide ACTIVATION quantize (the whole
        // block cooperates on one shared activation vector, then every warp
        // reads it). The cooperation granularity is the whole block (256
        // threads), not per-tile.
        KernelRule {
            rule_id: "cooperative-block-wide-quantize-staging".to_string(),
            gpu_concept: "when an input vector must be quantized/transformed once per dispatch and consumed by every warp in the block, have ALL threads cooperate to write disjoint chunks into shared memory, then each warp reads from shared memory — eliminates a separate staging dispatch + global-memory round-trip".to_string(),
            ref_shape: "// naive: separate quantize dispatch + global-memory read in the GEMV kernel\nfn gemv_split_quantize(act_f32: &[f32], act_i8: &mut [i8], ascale: &mut [f32], n: usize, ablock: usize) {\n    // Dispatch 1: quantize f32 -> int8 (separate kernel, global-memory round-trip)\n    for blk in 0..n.div_ceil(ablock) {\n        let mut absmax = 0.0;\n        for i in blk*ablock..(blk*ablock + ablock).min(n) { absmax = absmax.max(act_f32[i].abs()); }\n        let d = if absmax > 0.0 { absmax / 127.0 } else { 1.0 };\n        ascale[blk] = d;\n        for i in blk*ablock..(blk*ablock + ablock).min(n) {\n            act_i8[i] = (act_f32[i] / d).round().clamp(-128.0, 127.0) as i8;\n        }\n    }\n    // Dispatch 2: GEMV reads act_i8 from GLOBAL memory (1 extra dispatch + 1 upload)\n}".to_string(),
            opt_pattern: "// optimized: fused kernel — all 256 threads cooperate to quantize into shared memory\nfn gemv_fused_cooperative(act_f32: &[f32], out: &mut [f32], n: usize, ablock: usize, m: usize) {\n    // sh_act[N] + sh_ascale[ablocks] live in shared memory\n    let bs = 256; // blockDim.x\n    for blk in tid..n.div_ceil(ablock).step_by(bs) {\n        // Thread t owns block t (and t+bs, t+2bs, ...) — conflict-free scale write\n        let mut absmax = 0.0;\n        for i in blk*ablock..(blk*ablock + ablock).min(n) { absmax = absmax.max(act_f32[i].abs()); }\n        let d = if absmax > 0.0 { absmax / 127.0 } else { 1.0 };\n        sh_ascale[blk] = d;\n        for i in blk*ablock..(blk*ablock + ablock).min(n) {\n            sh_act[i] = (act_f32[i] / d).round().clamp(-128.0, 127.0) as i8;\n        }\n    }\n    __syncthreads();\n    // Each warp now reads sh_act from SHARED memory in its dp4a loop — no global read\n    // 1 dispatch total (was 2) + no act_i8 upload (saves N bytes host->device)\n}".to_string(),
        },
        // ── byte-perm-lut-via-permutation-immediate — shift/mask/sign chain → single PTX __byte_perm with permutation constant (B) ──
        // Source: riir-gpu gemv_ternary_cuda_raw.rs (Issue 608 T2). NVIDIA
        // __byte_perm(a, b, selector) picks 4 bytes from the 8-byte concatenation
        // {a, b} indexed by the 4 nibbles of `selector` — one PTX instruction.
        // The ternary kernel packs {−1, 0, 1, 2} as bytes in a constant int
        // 0x020100FF, then calls __byte_perm(0x020100FF, 0x020100FF, q >> 0)
        // where q is a packed int16 of 8 two-bit codes. Each nibble of (q >> 0)
        // has its low 2 bits indexing straight to the signed symbol — 4 codes
        // decoded to 4 signed bytes in ONE instruction. A second __byte_perm
        // re-interleaves the even/odd halves. Total: 3 PTX instructions for
        // 8-code decode vs ~24 scalar shift/mask/sign instructions.
        //
        // Generalizes to any small signed alphabet {s_0, ..., s_{2^k−1}}:
        // pack the alphabet as bytes in a permutation constant, then use
        // __byte_perm to map k-bit codes to their signed symbols in one
        // instruction. CUDA-specific (no CubeCL analogue). Requires sm_70+.
        KernelRule {
            rule_id: "byte-perm-lut-via-permutation-immediate".to_string(),
            gpu_concept: "decode N small-radix codes to signed symbols in ONE hardware instruction via __byte_perm(constant, constant, code >> shift) — the constant packs the signed alphabet as bytes, the selector nibbles index straight to the symbol (no LUT load, no branch, no modular arithmetic)".to_string(),
            ref_shape: "// naive: shift + mask + sign-extend per code (N codes = ~3N scalar ops)\nfn decode_ternary_naive(q: u16) -> [i8; 8] {\n    let mut out = [0i8; 8];\n    for k in 0..8 {\n        let code = ((q >> (k * 2)) & 0x3) as i8;\n        // code 0 -> -1, 1 -> 0, 2 -> +1\n        out[k] = code - 1; // 2->1, 1->0, 0->-1 — but this is 8 iterations of shift+mask+sub\n    }\n    out // ~24 scalar ops for 8 codes\n}".to_string(),
            opt_pattern: "// optimized: __byte_perm with packed signed-alphabet constant (3 PTX instructions for 8 codes)\n// 0x020100FF holds bytes {-1 (0xFF), 0, 1, 2} at indices 0..3\nfn decode_ternary_byte_perm(q: i32) -> ([i8; 4], [i8; 4]) {\n    // __byte_perm(a, b, sel): pick 4 bytes from {a,b} (8 bytes total), one per nibble of sel\n    let alphabet = 0x020100FFu32; // byte 0 = 0xFF (-1), 1 = 0, 2 = 1, 3 = 2\n    let qe = __byte_perm(alphabet, alphabet, (q >> 0) as u32);  // even nibbles -> 4 signed bytes\n    let qo = __byte_perm(alphabet, alphabet, (q >> 2) as u32);  // odd nibbles  -> 4 signed bytes\n    let qx = __byte_perm(qe, qo, 0x5140);  // re-interleave: elements 0,1,2,3 from qe,qo\n    let qy = __byte_perm(qe, qo, 0x7362);  // re-interleave: elements 4,5,6,7 from qe,qo\n    (unpack(qx), unpack(qy)) // 3 HW instructions total (was ~24 scalar)\n}\n// __byte_perm is sm_70+ CUDA intrinsic; no CubeCL/Metal analogue.".to_string(),
        },
        // ── dp4a-int8-hardware-dot-product — scalar 4-element dot → NVIDIA __dp4a single PTX instruction (B) ──
        // Source: riir-gpu gemv_ternary_cuda_raw.rs (Issue 608 T2). NVIDIA
        // __dp4a(a, b, c) computes a[0]*b[0] + a[1]*b[1] + a[2]*b[2] +
        // a[3]*b[3] + c for 4 int8 lanes in ONE PTX instruction (dp4a.s32.s8.s32).
        // The ternary GEMV uses it twice per 8-code block: 2 instructions for
        // an 8-element dot product that would take ~24 scalar FMA ops.
        //
        // Distinct from `cmma-hardware-unit-over-scalar-alu` (Batch 16) —
        // that's CubeCL cmma matrix-multiply-accumulate (8x8x8 tile matrix
        // op); this is NVIDIA scalar 4-lane int8 dot. Different instruction
        // class, different operand width (int8 vs f16/bf16), different
        // backend (CUDA-only vs CubeCL). The int8 requirement means
        // activations must be quantized per-block first (accuracy cost
        // ~3.7e-3 mean relative error on gaussians — argmax preserved).
        KernelRule {
            rule_id: "dp4a-int8-hardware-dot-product".to_string(),
            gpu_concept: "replace scalar 4-element int8 dot product with NVIDIA __dp4a — one PTX instruction (dp4a.s32.s8.s32) computes a[0]*b[0]+a[1]*b[1]+a[2]*b[2]+a[3]*b[3]+c for 4 int8 lanes. Distinct from cmma matrix tile op (Batch 16) — this is a scalar 4-lane SIMD dot, CUDA-only, requires int8 operands".to_string(),
            ref_shape: "// naive: scalar 4-element dot product (~8 FMA ops)\nfn dot4_scalar(a: &[i8; 4], b: &[i8; 4]) -> i32 {\n    a[0] as i32 * b[0] as i32\n        + a[1] as i32 * b[1] as i32\n        + a[2] as i32 * b[2] as i32\n        + a[3] as i32 * b[3] as i32\n}\n// For an 8-element dot: 2 calls = ~16 scalar FMA ops + 4 sign-extend ops = ~20 ops".to_string(),
            opt_pattern: "// optimized: NVIDIA __dp4a — 1 PTX instruction per 4-element int8 dot\nfn dot8_dp4a(a: &[i8; 8], b: &[i8; 8]) -> i32 {\n    let a_lo = pack_i8_le(a[0], a[1], a[2], a[3]);\n    let a_hi = pack_i8_le(a[4], a[5], a[6], a[7]);\n    let b_lo = pack_i8_le(b[0], b[1], b[2], b[3]);\n    let b_hi = pack_i8_le(b[4], b[5], b[6], b[7]);\n    let mut sum = __dp4a(a_lo, b_lo, 0); // 1 instruction: 4-lane int8 dot + add\n    sum = __dp4a(a_hi, b_hi, sum);       // 1 instruction: 4-lane int8 dot + accumulate\n    sum // 2 PTX instructions total (was ~20 scalar ops)\n}\n// __dp4a is sm_70+ CUDA intrinsic; requires int8 operands (activation quantization needed).\n// Achieves ~6x kernel speedup over scalar ALU on int8 ternary GEMV (llama.cpp 621 GB/s vs CubeCL 108 GB/s on 4090).".to_string(),
        },
        // ── threadgroup-subset-sum-lut-keyed-by-activation — per-bit shift/mask/select loop → 2 threadgroup-LUT reads + 1 subtract per 4 weights (B) ──
        // Source: riir-gpu gemv_ternary_cubecl.rs `gemv_ternary_plane_lut`
        // (Issue 606 T3c path 1, gated `ternary_lut_gemv`). Precompute a
        // 16-entry subset-sum table per 4-activation group in threadgroup
        // memory (256 groups × 16 masks = 4096 f32 = 16 KB), then each
        // 4-weight ternary dot collapses to S[p] − S[q] where p is the 4-bit
        // +1 pattern and q is the -1 pattern. Build is 4 loads + 12 adds per
        // group (the 16 subset sums share pair-sums), amortized over
        // 8 planes × 4 rows = 32 rows per workgroup and every tile iteration.
        //
        // THE KEY INSIGHT (why this is NOT the Q4_K LUT negative result):
        // the table is keyed by the ACTIVATION subset and contains no
        // weight, no scale, no zero-point. It therefore depends on nothing
        // that varies down the rows — one build serves EVERY output row in
        // the matrix. The Q4_K LUT (Plan 486, Bench 487) was keyed by weight
        // value + scale + zero-point, all of which vary per row AND per
        // sub-block, so a 5120-wide row rebuilt ~160 tables each amortizing
        // over only 32 elements — the rebuild ate the fusion win.
        // Structurally different amortization regime, not a tuning difference.
        //
        // Geometry forced: LUT_TILE_COLS = 1024 (= 32 u32 words = PLANE_DIM;
        // = 256 groups of 4 = CubeDim). Both build + consume come out
        // one-item-per-thread with no remainder loop.
        KernelRule {
            rule_id: "threadgroup-subset-sum-lut-keyed-by-activation".to_string(),
            gpu_concept: "precompute a 16-entry subset-sum table per 4-element activation group in threadgroup memory (keyed by activation subset, containing no weight/scale/zero-point), then each 4-weight signed dot collapses to 2 table reads + 1 subtract — one build serves every output row because the table depends on nothing that varies down the rows".to_string(),
            ref_shape: "// naive: per-bit shift/mask/select loop (~28 ops per 4-weight ternary dot)\nfn ternary_dot4_naive(pos_bits: u8, neg_bits: u8, x: &[f32; 4]) -> f32 {\n    let mut acc = 0.0;\n    for k in 0..4 {\n        let p = (pos_bits >> k) & 1;\n        let n = (neg_bits >> k) & 1;\n        let sign = if p == 1 { 1.0 } else if n == 1 { -1.0 } else { 0.0 };\n        acc += sign * x[k]; // 2 shifts + 2 masks + 2 int->float + select + FMA per weight\n    }\n    acc // ~28 ops per 4-weight dot, repeated for every output row\n}".to_string(),
            opt_pattern: "// optimized: threadgroup subset-sum LUT — 2 reads + 1 subtract per 4-weight dot\n// BUILD (once per tile, serves every row):\n//   thread t owns activation group t (4 elements x0..x3)\n//   S[m] = sum of x_i for i in bitmask m (16 subsets of a 4-element group)\nfn build_subset_sum_lut(x: &[f32; 4], lut: &mut [f32; 16]) {\n    let x01 = x[0] + x[1];\n    let x23 = x[2] + x[3];\n    lut[0b0000] = 0.0;\n    lut[0b0001] = x[0];\n    lut[0b0010] = x[1];\n    lut[0b0011] = x01;\n    lut[0b0100] = x[2];\n    lut[0b0101] = x[0] + x[2];\n    // ... 10 more subset sums sharing pair-sums (12 adds total)\n    lut[0b1111] = x01 + x23;\n}\n// CONSUME (every row reuses the same LUT):\nfn ternary_dot4_lut(pos_bits: u8, neg_bits: u8, lut: &[f32; 16]) -> f32 {\n    lut[(pos_bits & 0xF) as usize] - lut[(neg_bits & 0xF) as usize]\n    // 2 reads + 1 subtract (was ~28 ops) — one build amortizes over all rows\n}".to_string(),
        },
        // ── base-3-trit-packing-5-per-byte — 2-bit codes (4/byte) → base-3 trits (5/byte, 17.6% denser) (N) ──
        // Source: riir-gpu gemv_ternary_cubecl.rs `gemv_ternary_trit_rowtiled8`
        // (Issue 628 T2). Pack 5 ternary values per byte using base-3 encoding
        // (code = byte % 3; byte /= 3), reducing memory by 17.6% vs 2-bit
        // codes (1.75 bits/weight vs 2.125). Each group of 128 weights
        // occupies 28 bytes (26 live + 2 pad to u32 alignment = 7 u32 words).
        // The decode uses modular arithmetic, which the compiler optimizes
        // to multiply-by-inverse.
        //
        // HONEST CAVEAT (Issue 607): \"packed won on Metal, lost 2.1x on CUDA\".
        // The trit format's div/mod decode is significantly slower than the
        // bit-plane format's shift/mask on CUDA scalar ALU, and CUDA's int8
        // dp4a path can't consume base-3 trits directly (it requires the
        // 2-bit code layout). The format is viable on Metal (where div/mod
        // cost is lower and bandwidth-bound kernels benefit from the 17.6%
        // cut) but NOT on CUDA. Regime-dependent rule.
        //
        // Distinct from `ternary-weight-quant` (bit-plane format: 2 bit-planes
        // of 64 bits each, indexed by bit position) and `int2-pack-weights`
        // (4 values per byte via 2-bit codes). Base-3 trit is mixed-radix
        // (5 per byte) rather than fixed-width (4 per byte), trading decode
        // cost for density.
        KernelRule {
            rule_id: "base-3-trit-packing-5-per-byte".to_string(),
            gpu_concept: "pack 5 ternary values per byte using base-3 mixed-radix encoding (byte % 3 decode) — 17.6% denser than 2-bit codes (1.75 vs 2.125 bits/weight), regime-dependent: wins on bandwidth-bound Metal (div/mod cheap), loses 2.1x on CUDA scalar ALU (div/mod expensive, incompatible with __dp4a int8 path)".to_string(),
            ref_shape: "// naive: 2-bit code packing (4 ternary values per byte, 2.125 bits/weight)\nfn pack_2bit(codes: &[u8; 4]) -> u8 {\n    // code in {0, 1, 2} maps to {-1, 0, +1}\n    // 4 codes per byte, 2 bits each\n    codes[0] | (codes[1] << 2) | (codes[2] << 4) | (codes[3] << 6)\n}\nfn unpack_2bit(byte: u8) -> [u8; 4] {\n    [\n        (byte >> 0) & 0x3,  // shift + mask — cheap on all backends\n        (byte >> 2) & 0x3,\n        (byte >> 4) & 0x3,\n        (byte >> 6) & 0x3,\n    ]\n}".to_string(),
            opt_pattern: "// optimized: base-3 trit packing (5 ternary values per byte, 1.75 bits/weight)\nfn pack_trit5(codes: &[u8; 5]) -> u8 {\n    // 5 trits per byte: 3^5 = 243 <= 256 (fits in a byte)\n    codes[0] + codes[1] * 3 + codes[2] * 9 + codes[3] * 27 + codes[4] * 81\n}\nfn unpack_trit5(byte: u8) -> [u8; 5] {\n    let mut rem = byte;\n    let c0 = rem % 3; rem /= 3;  // div/mod — compiler optimizes to mul-by-inverse\n    let c1 = rem % 3; rem /= 3;\n    let c2 = rem % 3; rem /= 3;\n    let c3 = rem % 3; rem /= 3;\n    let c4 = rem % 3;\n    [c0, c1, c2, c3, c4]\n    // 17.6% denser than 2-bit (1.75 vs 2.125 bits/weight)\n    // REGIME-DEPENDENT: wins on Metal (bandwidth-bound, div/mod cheap),\n    // loses 2.1x on CUDA (div/mod expensive on scalar ALU, incompatible with __dp4a)\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 21 (Research 022) — Metal cross-runtime zero-copy integration
        // cluster (5 rules: 4A + 1D)
        //
        // Source: riir-gpu gemm_ternary_metal_zero_copy.rs (Issue 663 T5).
        // The third-attempt raw-Metal integration that eliminates both
        // overhead sources from Issue 656 (host round-trip) + Issue 657
        // (per-dispatch staging copies). The patterns are about the
        // INTEGRATION ARCHITECTURE (how a higher-level runtime hands GPU
        // memory to a raw native dispatch), not the kernel itself.
        //
        // Generic — apply to any cross-runtime integration where a wrapper
        // runtime (wgpu, CubeCL) needs to pass GPU memory to a raw native
        // dispatch (Metal, Vulkan, CUDA). The 2 refuted siblings
        // (gemm_ternary_metal_tensor.rs + _wgpu.rs) are covered by these
        // general rules: the host-round-trip is the anti-pattern rule #3
        // fixes, and the per-dispatch staging copies are a special case of
        // rule #2 when the runtime can't extract raw handles.
        // ════════════════════════════════════════════════════════════════════
        // ── cross-runtime-raw-handle-extraction — host round-trip → raw handle (A) ──
        // Source: riir-gpu gemm_ternary_metal_zero_copy.rs L230-252. Extract
        // `id<MTLBuffer>` from a CubeCL/wgpu handle via `get_resource` →
        // `as_hal::<Metal>` → `raw_handle()`. The handle extraction is the
        // ONLY way to dispatch a raw native kernel that reads/writes memory
        // owned by the higher-level runtime, without a host round-trip.
        //
        // Distinct from `alignment-gated-zero-copy-view` (N) — that's view
        // offsets within a single wgpu buffer (same-runtime, no cross-runtime
        // extraction); this is raw GPU API handle extraction across runtimes
        // (cross-runtime, different layer entirely).
        KernelRule {
            rule_id: "cross-runtime-raw-handle-extraction".to_string(),
            gpu_concept: "when a higher-level GPU runtime (wgpu, CubeCL) wraps a lower-level native API (Metal, Vulkan, CUDA), extract the raw native handle (e.g. id<MTLBuffer>) from the wrapper handle via the runtime's as_hal/raw_handle API — enables raw native kernel dispatch that reads/writes wrapper-owned memory without a host round-trip".to_string(),
            ref_shape: "// naive: host round-trip — read wrapper buffer to CPU, upload to native buffer, run native kernel, read back, re-upload to wrapper
fn native_dispatch_via_host(client: &CubeCLClient, input: &Handle, weights: &Handle) -> Result<Vec<f32>, String> {\n    let input_host: Vec<f32> = client.read_one(input.clone());      // GPU -> CPU (blocking sync)\n    let weights_host: Vec<f32> = client.read_one(weights.clone()); // GPU -> CPU (blocking sync)\n    let native_buf_in = native_device.new_buffer(input_host.len() as u64);\n    native_buf_in.upload(&input_host);                              // CPU -> native GPU\n    let out = native_kernel.launch(&native_buf_in, &native_weights); // run\n    let out_host = out.read_to_vec();                               // native GPU -> CPU\n    client.write(output_handle, &out_host);                         // CPU -> wrapper GPU\n    Ok(out_host) // 4 host-device transfers per dispatch — serialized the async pipeline\n}".to_string(),
            opt_pattern: "// optimized: extract raw id<MTLBuffer> from the wrapper handle — zero copy, zero host round-trip\nfn native_dispatch_zero_copy(client: &CubeCLClient, input: &Handle, weights: &Handle) -> Result<(), String> {\n    // 1. Get the wrapper's view of the resource\n    let managed = client.get_resource(input.clone()).map_err(|e| format!(\"get_resource failed: {e:?}\"))?;\n    let resource = managed.resource();\n    let wgpu_buffer = &resource.buffer;\n    let offset = resource.offset;\n    // 2. Extract the raw native handle via the wrapper's hal accessor\n    let raw_ptr = unsafe {\n        let guard = wgpu_buffer.as_hal::<wgpu::hal::api::Metal>();\n        let hal_buf = guard.as_ref().ok_or(\"wrapper buffer is not Metal\")?;\n        let proto = hal_buf.raw_handle(); // &ProtocolObject<dyn MTLBuffer>\n        proto as *const _ as *mut metal::MTLBuffer\n    };\n    // 3. Dispatch the native kernel directly on the raw handle (no copy)\n    let buf_ref: &BufferRef = unsafe { BufferRef::from_ptr(raw_ptr) };\n    encoder.set_buffer(0, Some(buf_ref), offset); // native kernel reads/writes wrapper-owned memory\n    Ok(()) // 0 host-device transfers — async pipeline stays async\n}".to_string(),
        },
        // ── pool-buffer-aliasing-dedicated-output — pool aliasing breaks hazard tracking → dedicated output + GPU->GPU copy (A) ──
        // Source: riir-gpu gemm_ternary_metal_zero_copy.rs L291-315. When the
        // runtime sub-allocates handles within one pool buffer, aliasing
        // between input and output (which share the same pool) breaks Metal's
        // hazard tracking — the kernel runs but writes nothing. Fix: write to
        // a dedicated buffer (not from the wrapper pool) keyed by byte size,
        // then GPU->GPU copy back via `encoder.copy_from_buffer` (same command
        // buffer, zero host round-trip).
        //
        // Distinct from `inplace-output-alias` (M, Batch 19) — that's
        // parameter-count bypass via single-buffer read-then-write within one
        // kernel (output aliases accumulation input); this is cross-buffer
        // hazard resolution when the runtime's pool aliases input and output
        // across DIFFERENT buffers (the output is a separate dedicated
        // buffer, then copied back).
        KernelRule {
            rule_id: "pool-buffer-aliasing-dedicated-output".to_string(),
            gpu_concept: "when the runtime sub-allocates handles within one pool buffer (e.g. CubeCL pool buffer), input and output handles sharing the same pool alias each other and break hazard tracking — write to a dedicated output buffer keyed by byte size, then GPU->GPU copy back via blit encoder (same command buffer, zero host round-trip)".to_string(),
            ref_shape: "// naive: bind the output handle directly — Metal hazard tracking silently fails, kernel runs but writes nothing\nfn dispatch_with_pool_aliasing(client: &CubeCLClient, input: &Handle, output: &Handle) {\n    let (in_ptr, in_off) = extract_raw(client, input);\n    let (out_ptr, out_off) = extract_raw(client, output);\n    // BUG: input + output share the same CubeCL pool buffer — Metal's hazard\n    // tracking sees the read + write on the same id<MTLBuffer> as a conflict\n    // and silently fails the cooperative-tensor store. Kernel runs, writes nothing.\n    encoder.set_buffer(0, Some(in_ref), in_off);\n    encoder.set_buffer(4, Some(out_ref), out_off); // same pool — aliasing hazard\n    encoder.dispatch_thread_groups(grid, threadgroup);\n}".to_string(),
            opt_pattern: "// optimized: dedicated output buffer keyed by byte size + GPU->GPU copy back\nstruct OutputCache { cache: Mutex<HashMap<u64, Buffer>> }\nfn dispatch_with_dedicated_output(cache: &OutputCache, client: &CubeCLClient, input: &Handle, output: &Handle, output_bytes: u64) {\n    let (in_ptr, in_off) = extract_raw(client, input);\n    let (out_ptr, out_off) = extract_raw(client, output); // for the copy-back target\n    // Dedicated output buffer — NOT from the CubeCL pool. Cached by byte size.\n    let dedicated = {\n        let mut c = cache.cache.lock().unwrap();\n        c.entry(output_bytes).or_insert_with(|| {\n            device_ref.new_buffer(output_bytes, MTLResourceOptions::StorageModeShared)\n        }).clone()\n    };\n    encoder.set_buffer(0, Some(in_ref), in_off);\n    encoder.set_buffer(4, Some(&dedicated), 0); // dedicated — no pool aliasing\n    encoder.dispatch_thread_groups(grid, threadgroup);\n    encoder.end_encoding();\n    // GPU->GPU copy back to the CubeCL output handle (same command buffer)\n    let blit = cmd_buffer.new_blit_command_encoder();\n    blit.copy_from_buffer(&dedicated, 0, out_ref, out_off, output_bytes);\n    blit.end_encoding(); // 1 GPU->GPU copy, 0 host round-trips\n}".to_string(),
        },
        // ── same-queue-implicit-ordering-no-sync — dispatch on shared queue, rely on in-order execution (A) ──
        // Source: riir-gpu gemm_ternary_metal_zero_copy.rs L350-412. By
        // dispatching the raw native kernel on the SAME Metal command queue
        // the wrapper runtime uses, ordering is implicit (no host-side
        // sync/wait). The wrapper's subsequent dispatches automatically run
        // after the raw native dispatch (Metal guarantees in-order execution
        // within one queue). This eliminates the host-round-trip serialization
        // that refuted Issue 656.
        //
        // Distinct from `chain-dispatches-single-sync-point` (C, Batch 19) —
        // that's queue N dependent dispatches in ONE command buffer with one
        // end-sync; this is dispatching on a SHARED QUEUE (different command
        // buffers, same queue) and relying on in-order execution WITHOUT a
        // final sync. Different scope (intra-cmdbuf vs cross-cmdbut-same-queue).
        KernelRule {
            rule_id: "same-queue-implicit-ordering-no-sync".to_string(),
            gpu_concept: "when a raw native dispatch must be ordered relative to wrapper-runtime dispatches (e.g. Metal kernel output read by a later CubeCL dispatch), submit on the SAME command queue the wrapper uses — the API guarantees in-order execution within one queue, so no explicit host-side sync or wait_until_completed is needed".to_string(),
            ref_shape: "// naive: separate command queue for the native dispatch — forces explicit sync between wrapper and native contexts\nfn dispatch_separate_queue(native_dev: &Device, wrapper_queue: &WrapperQueue, input: &Handle) {\n    let native_queue = native_dev.new_command_queue(); // SEPARATE queue from wrapper\n    let cmd = native_queue.new_command_buffer();\n    // ... dispatch native kernel ...\n    cmd.commit();\n    cmd.wait_until_completed(); // MUST sync — wrapper has no ordering visibility into native queue\n    // wrapper can now safely read the output\n}".to_string(),
            opt_pattern: "// optimized: extract the wrapper's command queue + dispatch the native kernel on it — ordering is implicit\nfn dispatch_shared_queue(wrapper_dev: Arc<WgpuDevice>, wrapper_queue: Arc<WgpuQueue>, input: &Handle) {\n    // Extract the raw id<MTLCommandQueue> from the wrapper's wgpu queue\n    let queue_ptr = unsafe {\n        let guard = wrapper_queue.as_hal::<wgpu::hal::api::Metal>();\n        let q = guard.as_ref().expect(\"wrapper queue is not Metal\");\n        q.as_raw() as *mut metal::MTLCommandQueue\n    };\n    let queue_ref: &CommandQueueRef = unsafe { CommandQueueRef::from_ptr(queue_ptr) };\n    let cmd = queue_ref.new_command_buffer(); // SAME queue as wrapper\n    // ... dispatch native kernel ...\n    cmd.commit();\n    // NO wait_until_completed — Metal guarantees in-order execution within one queue.\n    // The wrapper's NEXT dispatch (e.g. next prefill projection that reads this\n    // output) is automatically ordered AFTER this native dispatch. Blocking here\n    // would serialize the async pipeline (reintroducing the host-round-trip cost).\n}".to_string(),
        },
        // ── per-dispatch-pointer-re-extraction — cache handle, re-extract raw pointer fresh (A) ──
        // Source: riir-gpu gemm_ternary_metal_zero_copy.rs L211-224 + L278-283.
        // When the runtime can reallocate/defragment its memory pool between
        // dispatches, raw pointers extracted from handles must be re-extracted
        // on EVERY dispatch, not cached. Cache the higher-level handle
        // (cheap, ref-counted, stable), re-extract the raw pointer fresh.
        // This is the CORRECTNESS INVARIANT that makes rule #1 safe.
        //
        // Caching a raw pointer across dispatches is a use-after-free when the
        // pool defragments. The handle clone is cheap (Arc bump); the
        // raw-pointer extraction is a few pointer reads.
        KernelRule {
            rule_id: "per-dispatch-pointer-re-extraction".to_string(),
            gpu_concept: "when a higher-level runtime can reallocate/defragment its memory pool between dispatches, raw native pointers extracted from wrapper handles must be re-extracted on EVERY dispatch, not cached across dispatches — cache the higher-level handle (ref-counted, stable) and re-extract the raw pointer fresh each time".to_string(),
            ref_shape: "// naive: cache the raw pointer across dispatches — use-after-free when the pool defragments\nstruct BadCache {\n    cached_ptr: *mut metal::MTLBuffer, // cached ONCE at boot\n    cached_offset: u64,\n}\nimpl BadCache {\n    fn dispatch(&self, encoder: &Encoder) {\n        // BUG: if the wrapper's pool defragmented between boot and this dispatch,\n        // cached_ptr points to freed/recycled memory — use-after-free.\n        let buf_ref: &BufferRef = unsafe { BufferRef::from_ptr(self.cached_ptr) };\n        encoder.set_buffer(0, Some(buf_ref), self.cached_offset);\n    }\n}".to_string(),
            opt_pattern: "// optimized: cache the WRAPPER HANDLE (stable, ref-counted), re-extract the raw pointer on EVERY dispatch\nstruct SafeCache {\n    handle: Handle, // wrapper handle — cheap to clone (Arc bump), stable across pool defrags\n}\nimpl SafeCache {\n    fn dispatch(&self, client: &CubeCLClient, encoder: &Encoder) -> Result<(), String> {\n        // Re-extract the raw pointer FRESH on every dispatch — the handle is\n        // stable but the underlying pool allocation may have moved.\n        let managed = client.get_resource(self.handle.clone()).map_err(|e| format!(\"{e:?}\"))?;\n        let resource = managed.resource();\n        let wgpu_buffer = &resource.buffer;\n        let offset = resource.offset;\n        let raw_ptr = unsafe {\n            let guard = wgpu_buffer.as_hal::<wgpu::hal::api::Metal>();\n            let hal_buf = guard.as_ref().ok_or(\"not Metal\")?;\n            hal_buf.raw_handle() as *mut metal::MTLBuffer\n        };\n        let buf_ref: &BufferRef = unsafe { BufferRef::from_ptr(raw_ptr) };\n        encoder.set_buffer(0, Some(buf_ref), offset);\n        Ok(()) // safe across pool defragmentation — pointer is always fresh\n    }\n}".to_string(),
        },
        // ── no-blocking-sync-in-async-pipeline — wait_until_completed serializes → rely on queue ordering (D) ──
        // Source: riir-gpu gemm_ternary_metal_zero_copy.rs L407-419. The
        // dispatch does NOT call `cmd_buffer.wait_until_completed()`. The
        // command buffer runs on the shared Metal queue — the wrapper's
        // subsequent dispatches (including the next prefill projection that
        // reads this output) are automatically ordered after it (Metal
        // guarantees in-order execution within one queue). Blocking here would
        // serialize the async pipeline, reintroducing the host-round-trip cost.
        //
        // The `client.sync()` BEFORE dispatch ensures the INPUT buffer's data
        // is committed; the queue ordering ensures the OUTPUT is visible to
        // the next consumer. Only a test that reads the buffer via a
        // non-queued path (e.g. `client.read_one` which may map without a GPU
        // sync) needs explicit `wait_until_completed`.
        //
        // Complement of `chain-dispatches-single-sync-point` (C, Batch 19) —
        // that's about reducing N syncs to 1 WITHIN a command buffer; this is
        // about avoiding the FINAL sync entirely by relying on queue ordering.
        KernelRule {
            rule_id: "no-blocking-sync-in-async-pipeline".to_string(),
            gpu_concept: "when dispatching on a shared command queue inside an async pipeline, never call wait_until_completed/sync — it serializes the pipeline by blocking the host thread. Use queue ordering (in-order execution within one queue) + the caller's pre-dispatch sync to cover correctness. Only non-queued read paths (test read_one) need explicit sync".to_string(),
            ref_shape: "// naive: block on every dispatch — serializes the async pipeline (host thread stalls)\nfn dispatch_blocking(cmd_queue: &CommandQueueRef) -> Result<Vec<f32>, String> {\n    let cmd = cmd_queue.new_command_buffer();\n    let encoder = cmd.new_compute_command_encoder();\n    // ... dispatch kernel ...\n    encoder.end_encoding();\n    cmd.commit();\n    cmd.wait_until_completed(); // BLOCKS the host thread until the GPU finishes\n    // The next dispatch can't even be queued until this one finishes —\n    // the async pipeline is serialized into a sync one. At ~448 prefill\n    // projections, this is 448 blocking round-trips (the Issue 656 regression).\n    Ok(read_output())\n}".to_string(),
            opt_pattern: "// optimized: no blocking sync — submit and return; queue ordering covers correctness\nfn dispatch_non_blocking(cmd_queue: &CommandQueueRef) -> Result<(), String> {\n    let cmd = cmd_queue.new_command_buffer();\n    let encoder = cmd.new_compute_command_encoder();\n    // ... dispatch kernel ...\n    encoder.end_encoding();\n    cmd.commit();\n    // NO wait_until_completed — the host thread returns immediately. The\n    // wrapper's NEXT dispatch on the same queue is automatically ordered\n    // AFTER this one (Metal: in-order execution within one queue).\n    //\n    // The caller's pre-dispatch `client.sync()` ensured the INPUT was committed;\n    // the queue ordering ensures the OUTPUT is visible to the next consumer.\n    // Only an external test using `client.read_one` (which may map without\n    // a GPU sync) needs to call wait_until_completed explicitly.\n    Ok(()) // async pipeline stays async — 448 projections pipeline through the queue\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 22 (Research 023): cudarc_kernels CUDA cluster — 5 rules
        // (embedding.rs + attention.rs + mod.rs + deltanet.rs; Issue 615 full
        // GPU-resident forward port).
        // ════════════════════════════════════════════════════════════════════
        // ── dynamic-shared-memory-runtime-sized — extern __shared__ for runtime-sized smem (A) ──
        // Source: riir-gpu cudarc_kernels/attention.rs L265-301 + deltanet.rs
        // recurrence kernel. The CubeCL `qwen_attention_decode_f32` used
        // `Shared::<[f32]>::new_slice(128)` — a compile-time constant too
        // small for Bonsai-27B (head_dim=256). The raw CUDA port uses
        // `extern __shared__` with `shared_mem_bytes = head_dim * 4` at
        // launch, so the allocation matches the actual dimension.
        KernelRule {
            rule_id: "dynamic-shared-memory-runtime-sized".to_string(),
            gpu_concept: "use extern __shared__ with launch-time shared_mem_bytes instead of a compile-time __shared__ array when the smem size depends on a runtime dimension (e.g. head_dim varies: 64 for DeltaNet, 128 for Qwen-7B, 256 for Bonsai-27B)".to_string(),
            ref_shape: "// naive: compile-time smem — wrong for head_dim != 128\n__shared__ float smem[128];  // too small for Bonsai-27B (head_dim=256)\n// ... uses smem[0..head_dim] — out-of-bounds for head_dim > 128 ...".to_string(),
            opt_pattern: "// optimized: runtime-sized smem via extern __shared__ + launch config\nextern \"C\" __global__ void kernel(float* out, int head_dim) {\n    extern __shared__ float smem[];  // sized at launch via shared_mem_bytes\n    // ... use smem[0..head_dim] — correct for any head_dim ...\n}\n// launch:\nlet cfg = LaunchConfig {\n    grid_dim: (n_head as u32, 1, 1),\n    block_dim: (head_dim as u32, 1, 1),\n    shared_mem_bytes: (head_dim * 4) as u32,  // runtime-sized\n};".to_string(),
        },
        // ── ternary-bit-plane-u64-as-2x-u32 — bit-plane sign extraction for ternary dequant (A) ──
        // Source: riir-gpu cudarc_kernels/embedding.rs L37-69. Ternary
        // weights stored as two bit-planes (pos_bits + neg_bits), packed as
        // u64 blocks cast to 2×u32 for per-bit sign extraction. Column c
        // maps to: block_idx=c/64, word_in_block=(c%64)/32, bit_pos=c%32.
        // Distinct from base-3-trit-packing-5-per-byte (arithmetic base-3)
        // — this is 1 sign bit per position across two planes.
        KernelRule {
            rule_id: "ternary-bit-plane-u64-as-2x-u32".to_string(),
            gpu_concept: "ternary weights as two bit-planes (positive + negative sign bits) packed as u64 blocks cast to 2×u32 words — extract sign per column via bit_pos = c % 32, word_in_block = (c % 64) / 32, then output = sign * per-group scale. The 2×u32 cast is because u64 isn't directly per-bit addressable in CUDA; two u32 words are".to_string(),
            ref_shape: "// naive: store ternary as i8 per element (3 bytes per 2 ternary values)\nfn dequant_naive(weights: &[i8], scales: &[f32], out: &mut [f32], n: usize) {\n    for c in 0..n {\n        let w = weights[c];  // -1, 0, or +1 stored as i8\n        let group = c / GROUP_SIZE;\n        out[c] = w as f32 * scales[group];\n    }\n}".to_string(),
            opt_pattern: "// optimized: bit-plane packing — 64 ternary values per u64 block (2 bits each, pos + neg)\n// Layout: pos_bits_u32[rows * blocks64 * 2] + neg_bits_u32[rows * blocks64 * 2]\nfn dequant_bitplane(pos_bits: &[u32], neg_bits: &[u32], scales: &[f32], out: &mut [f32], row: usize, blocks64: usize, groups_per_row: usize, n: usize) {\n    let words_per_row = blocks64 * 2;  // u64 → 2× u32\n    for c in 0..n {\n        let block_idx = c / 64;\n        let word_in_block = (c % 64) / 32;\n        let bit_pos = c % 32;\n        let word_idx = row * words_per_row + block_idx * 2 + word_in_block;\n        let pos_bit = (pos_bits[word_idx] >> bit_pos) & 1;\n        let neg_bit = (neg_bits[word_idx] >> bit_pos) & 1;\n        let sign = pos_bit as f32 - neg_bit as f32;  // +1, 0, or -1\n        let group = c / GROUP_SIZE;\n        let scale = scales[row * groups_per_row + group];\n        out[c] = sign * scale;\n    }\n}".to_string(),
        },
        // ── warp-shuffle-intrinsics-for-sub-warp-reduction — __shfl_down_sync for ≤32-thread blocks (B) ──
        // Source: riir-gpu cudarc_kernels/mod.rs L163-167 (quantize_f32_to_i8)
        // + attention.rs L336-344 (final warp reduction). For blocks ≤32
        // threads (one warp), use __shfl_down_sync for reduction instead
        // of shared memory + __syncthreads. The warp runs in lockstep (SIMT),
        // so the shuffle is both correct and free (no smem alloc, no sync
        // barrier). Companion: for the final 32-thread tail of a larger smem
        // tree reduction, use `volatile float* vsmem = smem` + skip
        // __syncthreads — volatile defeats register caching, lockstep makes
        // the sync unnecessary.
        KernelRule {
            rule_id: "warp-shuffle-intrinsics-for-sub-warp-reduction".to_string(),
            gpu_concept: "for blocks of 32 threads or fewer (one warp), use __shfl_down_sync(mask, val, delta) butterfly reduction instead of shared memory + __syncthreads — the warp executes in lockstep (SIMT), so the shuffle is both correct and free (no smem allocation, no synchronization barrier). For the final 32-thread tail of a larger smem tree reduction, cast to volatile float* and skip __syncthreads — volatile prevents the compiler from caching stale values in registers. Measured Metal cost note (qwen-3.8-mtp-challenge kernel-research.md section 1.1 @ 0863b06a, Batch 100): simd_sum(f32) runs ~14.5 cycles throughput / ~28 latency vs ~2 for a shuffle — a per-(token, head) simd_sum is a real ALU cost in a GQA=8 pass-A inner loop (8 simd_sums per token per simdgroup); quad reductions are ~2x cheaper and a shuffle tree can beat the builtin — the shuffle-butterfly arm of this rule is the measured-cheap arm on Apple silicon as well, not only under the CUDA lockstep argument. RIDER (Batch 114 external instance, jundot/omlx @ e467261 -- custom_kernels/qwen35_prefill/gdn.py:438-442 + :499-517): the SUB-SIMDGROUP cooperation radius -- state fragments owned by 8-lane SEGMENTS inside a 32-lane simdgroup reduce via simd_shuffle_down(4,2,1) (three steps, staying inside the row's 8 lanes) and broadcast once from the row leader (simd_shuffle(part, (tid % 32) / 8 * 8)); zero barriers in the hot loop ('reduce across the 8 threads of a dv row via simd_shuffle_down (no threadgroup barriers in the hot loop)') -- a finer radius than one full simdgroup, with the leader-index broadcast replacing any cross-lane tree. RIDER (Batch 118, antirez/ds4 PR #991 @ 236cb2a5 MIT): the XOR-BUTTERFLY SUB-SIMDGROUP twin -- lane segments smaller than the simdgroup shuffle by xor masks, zero barriers: an 8-lane sub-simdgroup dot reduce (acc += simd_shuffle_xor(acc,1/2/4) within the stream's lane segment, s = tiisg/8, lane = tiisg%8) then a cross-segment merge (g += simd_shuffle_xor(g,8/16)) over 4 stream segments; in-source instances: router argmax ladder (qwen4.metal:821-825/:848-852), the online-softmax quad reduce (:1577-1589), hc_gate_mix_pair (:289-296), and the stream-row dot (:239,:245-251). Composition note: xor BUTTERFLY ladders are safe to accumulate IN PLACE (disjoint partner pairs per stage) while INDEXED shuffle ladders are not -- see the shuffle-lane unmutated-partial face on block-reduction-separate-scratch-runtime-strides (same batch). RIDER (Batch 251, yukon cudafast-bonsai2-9b-engine @ 36c9536 -- the MULTI-VALUE SINGLE-SCHEDULE face): N independent warp reductions through the SAME butterfly offsets -- the templated `gdn_warp_reduce_sum_n` (`template <int width, int N>`) walks the canonical offset ladder ONCE (`for (int offset = width/2; offset > 0; offset >>= 1)`) and, inside each offset step, exchanges ALL N values (`for (int c = 0; c < N; ++c) { x[c] += __shfl_xor_sync(0xffffffff, x[c], offset, width); }`), so the N per-offset shuffles issue back-to-back and overlap their shuffle latency instead of serializing N whole reductions; the schedule contract is per-value exact, verbatim (gated_delta_net_kernel.cuh:17-18): \"warp_reduce_sum for N independent values: one butterfly step for all of them at a time, so their shuffle latencies overlap. Each value goes through exactly warp_reduce_sum's shuffles and adds.\" -- each value keeps the canonical reduce's exact pairing order, so the multi-value form is bit-identical to N single-value butterflies by construction. The GDN step's two hot reductions share ONE schedule each (`gdn_warp_reduce_sum_n<warp_size, cols_per_warp>(kv_col)` at gated_delta_net_kernel.cuh:396, then `gdn_warp_reduce_sum_n<warp_size, cols_per_warp>(attn_col)` at :411) -- cols_per_warp independent accumulators per call, the shuffle latency paid once for both. Verified: gated_delta_net_kernel.cuh:17-25, :396, :411.".to_string(),
            ref_shape: "// naive: shared-memory tree reduction even for a 16-thread block (overkill — one warp)\n__shared__ float smem[16];\nsmem[tid] = val;\n__syncthreads();\nif (tid < 8) smem[tid] = fmaxf(smem[tid], smem[tid + 8]); __syncthreads();\nif (tid < 4) smem[tid] = fmaxf(smem[tid], smem[tid + 4]); __syncthreads();\nif (tid < 2) smem[tid] = fmaxf(smem[tid], smem[tid + 2]); __syncthreads();\nif (tid < 1) smem[0] = fmaxf(smem[0], smem[1]); __syncthreads();\nfloat result = smem[0];  // 4 syncthreads + 16 floats of smem for a 16-thread block".to_string(),
            opt_pattern: "// optimized: warp-shuffle butterfly — no smem, no syncthreads (blockDim.x <= 32 = one warp)\nfloat a = fabsf(val);\na = fmaxf(a, __shfl_down_sync(0xFFFFFFFFu, a, 8));\na = fmaxf(a, __shfl_down_sync(0xFFFFFFFFu, a, 4));\na = fmaxf(a, __shfl_down_sync(0xFFFFFFFFu, a, 2));\na = fmaxf(a, __shfl_down_sync(0xFFFFFFFFu, a, 1));\nconst float absmax = __shfl_sync(0xFFFFFFFFu, a, 0);  // broadcast lane 0 to all\n// 4 shuffles, 0 smem, 0 syncthreads — hardware warp intrinsic\n//\n// For the final 32-thread tail of a LARGER smem tree reduction:\nif (tid < 32) {\n    volatile float* vsmem = smem;  // volatile: defeat compiler register caching\n    if (tid < 16) vsmem[tid] = fmaxf(vsmem[tid], vsmem[tid + 16]);\n    if (tid < 8)  vsmem[tid] = fmaxf(vsmem[tid], vsmem[tid + 8]);\n    if (tid < 4)  vsmem[tid] = fmaxf(vsmem[tid], vsmem[tid + 4]);\n    if (tid < 2)  vsmem[tid] = fmaxf(vsmem[tid], vsmem[tid + 2]);\n    if (tid < 1)  vsmem[0] = fmaxf(vsmem[0], vsmem[1]);\n    // NO __syncthreads — warp runs in lockstep".to_string(),
        },
        // ── fused-activation-quantize-for-gemv-consumer — activation + absmax + int8 in one kernel (C) ──
        // Source: riir-gpu cudarc_kernels/mod.rs L191-289 (rmsnorm_quantize_f32,
        // Issue 623) + L407-450 (swiglu_quantize_f32, Issue 625) + L468-554
        // (gate_silu/sigmoid_quantize_f32, Issue 626) + L575+ (rmsnorm_gate_silu_quantize_f32,
        // Issue 627 three-way fusion). When an activation's output is immediately
        // consumed by a GEMV that reads int8, fuse the activation + absmax + int8
        // quantize into one kernel — eliminates the intermediate f32 HBM
        // round-trip (~2.6 MB/token for RMSNorm, ~115 KB/layer for SwiGLU).
        // Distinct from fuse-quant-matmul (quantize+matmul) — here the activation
        // is the non-trivial part (RMSNorm needs a reduction, SwiGLU needs an exp).
        KernelRule {
            rule_id: "fused-activation-quantize-for-gemv-consumer".to_string(),
            gpu_concept: "when an activation's output (RMSNorm, SwiGLU, gate*sigmoid, gate*silu) is immediately consumed by a GEMV that reads int8 (not f32), fuse the activation + per-block absmax reduction + int8 quantize into one kernel — eliminates the intermediate f32 HBM round-trip and one kernel launch. The activation is the non-trivial part (needs a reduction or exp); the quantize is the trivial part (absmax + scale + round) RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- fused_kimi_k3_mla_key_concat_kv_cache_kernel.cu:15-24 + cache_kernels_fused.cu:108-145): the RoPE rotation is computed in-registers and the rotated pair is written DIRECTLY to its quantized cache slot in the producing kernel (qk_t rotate -> optional scaled_convert -> kv_cache_ptr[...]) -- the consumer-side quantize/store fused into the producer, three in-source kernel families (deepseek-v4, minimax-m3, kimi-k3). RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- commit 662a0b0 #27310 -- CUDA-side): the DFlash encoder is fused INTO the KV injection, and the sibling commit 8663224 #27877 DISABLES the non-fused GDN/LID paths -- the fused-path preference posture: when the fused kernel exists, the non-fused path is a liability, not a fallback.".to_string(),
            ref_shape: "// naive: separate activation then quantize (2 kernel launches, intermediate f32 HBM round-trip)\nfn rmsnorm_then_quantize_separate(input: &[f32], gamma: &[f32], out: &mut [i8], ascale: &mut [f32]) {\n    let mut norm = vec![0.0f32; input.len()];  // intermediate f32 buffer (~2.6 MB/token)\n    rmsnorm_kernel(input, gamma, &mut norm);  // launch 1: writes f32 to HBM\n    quantize_kernel(&norm, out, ascale);       // launch 2: reads f32 from HBM, writes i8\n}".to_string(),
            opt_pattern: "// optimized: fused activation + absmax + quantize (1 kernel, no intermediate)\nfn rmsnorm_quantize_fused(input: &[f32], gamma: &[f32], out: &mut [i8], ascale: &mut [f32]) {\n    // Phase 1: RMSNorm sum-of-squares (smem tree reduction)\n    // Phase 2: normalize + find per-block absmax (warp shuffle)\n    // Phase 3: quantize — d = absmax / 127, i8 = round(v / d), write directly\n    // One kernel launch, one HBM write (i8), no intermediate f32\n    // Same pattern applies to: SwiGLU+quantize, gate*sigmoid+quantize,\n    // gate*silu+quantize, and three-way RMSNorm+gate+quantize fusion.".to_string(),
        },
        // ── cuda-graph-scalar-via-device-pointer — _devpos variant for graph capture (D) ──
        // Source: riir-gpu cudarc_kernels/embedding.rs L76-105 (dequant_wte_row_f32_devpos,
        // Issue 618) + attention.rs L421-610 (rope_partial_f32_devpos +
        // kv_cache_append_f32_devpos + attention_decode_f32_devpos). When
        // capturing a CUDA graph, scalar kernel args (int, float) get baked
        // into the captured graph — the graph replays the exact value on
        // every launch. Ship a _devpos variant that reads the scalar from a
        // device pointer dereferenced at kernel runtime; the caller writes
        // the new value via memcpy_htod before graph.launch().
        KernelRule {
            rule_id: "cuda-graph-scalar-via-device-pointer".to_string(),
            gpu_concept: "when capturing a CUDA graph, scalar kernel args (int pos, int row_idx) get baked into the captured graph at capture time — the graph replays the exact value on every launch and cannot change. Ship a _devpos variant that reads the scalar from a const device pointer dereferenced at kernel runtime; the caller writes the new value to a 4-byte device buffer via memcpy_htod on the same stream before graph.launch(), so the captured graph processes a different input each call. RIDER (Batch 199 / Plan 197 — the DERIVED-TABLE arm, TensorFold v0.6.0 @ c4646171, Apache-2.0 (license flipped MIT→Apache-2.0 at 0.6.0)): the mirror hazard to a CHANGING scalar is a DERIVED-ONCE table first read inside a capture — a lazily-materialized tensor allocates DURING capture, and capture is where allocation is forbidden. The source materializes the derived per-tensor factor at construction instead: fp4_from_rows builds its ones-factor in the constructor — 'its factor made now (graphs never allocate)' (nvfp4.py:214, in-source-doc at the pin; the commit's ORIGINAL docstring wording was 'The per-tensor factor is materialised here, not lazily: a table both the eager step and a CUDA graph capture read must never allocate during capture.', compressed by a later style pass — both forms cited, the current one verified at the pin) — so 'the pattern form now carries its ones factor from construction, so nothing allocates inside a capture' (commit 1c5ea53, commit-message-only) — 'scale2=torch.ones(n, ...)' rides the constructor (nvfp4.py:216-217, in-source), so the capture reads an existing buffer. The family law gains its second arm: VALUE CHANGES cross captures through device buffers written before launch (this rule); VALUE CREATION must happen before capture entirely — derive eagerly at construction, never lazily at first read, when a capture is among the readers. CITE: ashhart/tensorfold @ c4646171 (v0.6.0, Apache-2.0) — src/tensorfold/families/qwen4_exp/cuda/nvfp4.py:213-217 + commit 1c5ea53.".to_string(),
            ref_shape: "// naive: scalar arg — baked into captured CUDA graph, cannot change per-launch\nextern \"C\" __global__ void kernel_scalar(const float* in, float* out, const int pos) {\n    // pos is a CAPTURE-TIME constant. graphCapture() bakes this value in;\n    // subsequent graph.launch() calls replay the SAME pos forever.\n    out[threadIdx.x] = in[pos + threadIdx.x];\n}\n// capture:\nstream.launch_builder(&kernel_scalar).arg(in).arg(out).arg(&42).launch(cfg);\nstream.graph_capture_begin();\nstream.launch_builder(&kernel_scalar).arg(in).arg(out).arg(&42).launch(cfg);  // pos=42 baked in\nlet graph = stream.graph_capture_end();\n// graph.launch() always reads pos=42 — cannot process pos=43 without re-capturing.".to_string(),
            opt_pattern: "// optimized: device-pointer variant — dereferenced at kernel runtime, graph-capturable\nextern \"C\" __global__ void kernel_devpos(const float* in, float* out, const int* pos_dev) {\n    const int pos = *pos_dev;  // dereference at kernel runtime — NOT baked into graph\n    out[threadIdx.x] = in[pos + threadIdx.x];\n}\n// capture (once):\nstream.graph_capture_begin();\nstream.launch_builder(&kernel_devpos).arg(in).arg(out).arg(pos_dev_buf).launch(cfg);\nlet graph = stream.graph_capture_end();  // captures the POINTER, not the value\n// per-call: write new pos to the device buffer, then launch the graph\nstream.memcpy_htod(pos_dev_buf, &new_pos);  // update before launch\ngraph.launch();  // reads whatever pos_dev_buf contains — different pos each call\n// One graph capture amortized across all calls; per-call cost is just a 4-byte memcpy.".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 23 (Research 024): CubeCL flash-attention cluster — 5 rules
        // (attention_cubecl/mod.rs — 1573 lines; flash-attention decode +
        // folded dispatch + block-causal + GQA + softcap-tanh + combined KV +
        // hardcoded fast-path JIT specialization).
        // ════════════════════════════════════════════════════════════════════
        // ── combined-kv-buffer-keys-then-values — single contiguous KV buffer (A) ──
        // Source: riir-gpu attention_cubecl/mod.rs L107-140 + L576-650. The
        // kernel takes a single `kv` buffer `[2 × n_positions × kv_stride]`
        // laid out as `[keys | values]` (keys occupy the first half, values
        // the second). One buffer binding instead of two; a single kv_off
        // index addresses both K (kv[k_base + d]) and V (kv[kv_half + k_base
        // + d]) via an offset by kv_half. Coalesced because both K and V for
        // the same position share a contiguous stride.
        KernelRule {
            rule_id: "combined-kv-buffer-keys-then-values".to_string(),
            gpu_concept: "store keys and values as a single contiguous buffer `[keys | values]` with shared stride — halves buffer bindings from 2 to 1 and enables coalesced access via a single kv_off index. The value for position p at dimension d is at kv[kv_half + p * kv_stride + d], where kv_half = kv.len() / 2. Distinct from KV cache layout: this is about the buffer binding shape, not the cache update logic".to_string(),
            ref_shape: "// naive: separate key + value buffers (2 bindings, 2 strides to manage)\nfn attention_separate_kv(q: &[f32], keys: &[f32], values: &[f32], out: &mut [f32], n_pos: usize, kv_stride: usize, head_dim: usize) {\n    for pos in 0..n_pos {\n        let k_base = pos * kv_stride;  // key stride\n        let v_base = pos * kv_stride;  // value stride (must match for correctness)\n        let mut dot = 0.0;\n        for d in 0..head_dim { dot += q[d] * keys[k_base + d]; }\n        let score = dot;\n        for d in 0..head_dim { out[d] += score * values[v_base + d]; }\n    }\n    // 2 buffer bindings, 2 index variables, 2 stride computations\n}".to_string(),
            opt_pattern: "// optimized: combined KV buffer [keys | values] — 1 binding, 1 stride\nfn attention_combined_kv(q: &[f32], kv: &[f32], out: &mut [f32], n_pos: usize, kv_stride: usize, head_dim: usize) {\n    let kv_half = kv.len() / 2;  // keys occupy [0, kv_half), values occupy [kv_half, 2*kv_half)\n    for pos in 0..n_pos {\n        let kv_off = pos * kv_stride;\n        let mut dot = 0.0;\n        for d in 0..head_dim { dot += q[d] * kv[kv_off + d]; }  // key read\n        let score = dot;\n        for d in 0..head_dim { out[d] += score * kv[kv_half + kv_off + d]; }  // value read (offset by kv_half)\n    }\n    // 1 buffer binding, 1 stride — halves descriptor table pressure, coalesces\n}".to_string(),
        },
        // ── gqa-kv-head-sharing-bandwidth-reduction — share KV across query heads (A) ──
        // Source: riir-gpu attention_cubecl/mod.rs L150 + L345 + L870. GQA:
        // n_kv_head < n_head (e.g. Gemma 2 2B: 8 query heads, 4 KV heads = 2:1).
        // Each query head maps to its KV group via
        // `kv_group = head_idx * n_kv_head / n_head`. Multiple query heads
        // in the same group read the SAME KV cache slice — reduces KV cache
        // bandwidth by n_head/n_kv_head (2× for 8:4). The kernel computes
        // kv_off = kv_group * head_dim ONCE per workgroup, not per query head.
        KernelRule {
            rule_id: "gqa-kv-head-sharing-bandwidth-reduction".to_string(),
            gpu_concept: "Grouped-Query Attention (GQA): share KV heads across multiple query heads (n_kv_head < n_head) to reduce KV cache memory + bandwidth by n_head/n_kv_head× (e.g. 2× for Gemma 2 2B's 8:4 ratio). Each query head maps to its KV group via kv_group = head_idx * n_kv_head / n_head; the kernel computes kv_off = kv_group * head_dim once per workgroup and reuses it for all K/V reads in that head. Distinct from MHA (1:1) and MQA (all heads share 1 KV head) RIDER (Batch 276, the HPT + BATCHED-COMPONENTS EPILOGUE face, Layr-Labs/mlxfast-qwen38-125b-a6b-engine @ 59160bd (MIT), intro commit 7df55c7 -- where this host owns GQA KV sharing architecturally (kv_group computed once per workgroup), this face owns the kernel-level scheduling half in the 2-pass decode kernel: FOUR QUERY HEADS PER THREADGROUP -- 'Four query heads reuse each K/V load while retaining their own block walk.' with 'K/V/Q move in 16-byte vector loads; the score keeps its sequential' 'component order so the accumulated value is unchanged.' (kernels/sdpa_vector.h:207-209) -- admitted at bf16 D=V=256 by 'if (N >= 1024 && tptg.z == 1 && gqa % 4 == 0 && !has_mask && !has_sinks) {' (:212), so 'each K/V byte is read G / HPT times instead of G times' in the 'template <typename T, int D, int V, int G, int HPT>' twin sdpa_vector_2pass_1_gqa (:389-395, single-token queries without mask or sinks only); the stock `_gqa` dispatch covers head dims 'q.shape(-1) == 64 || q.shape(-1) == 128' only (scaled_dot_product_attention.cpp:469-474), which is why the D=256 arm was inlined. Second face, the BATCHED-COMPONENTS EPILOGUE -- the intra-kernel barrier hoist: 'constexpr bool batch_components = metal::is_same_v<T, bfloat16_t> && D == 256;' (:550) arms the wide staged layout where the epilogue stores ALL components, takes ONE barrier, then runs all simd_sums (:593-605), vs the stock arm's barrier PER component; `two-pass-partial-reduction-inter-dispatch-barrier` (entries_gpu_116.rs) is the inter-dispatch cousin -- cite, don't ride. The epilogue landed 4dbe99b, the HPT arm 7df55c7. Batch 276 / Plan 280 / Research 314.".to_string(),
            ref_shape: "// naive: Multi-Head Attention — each query head has its OWN KV cache (n_kv_head == n_head)\nfn mha_attention(q_heads: &[f32], kv_cache: &[f32], out: &[f32], n_head: usize, n_kv_head: usize, head_dim: usize, n_pos: usize) {\n    // kv_cache layout: [n_head × n_pos × head_dim] — one KV slice per query head\n    assert_eq!(n_head, n_kv_head);  // MHA: 1:1 mapping\n    for h in 0..n_head {\n        for pos in 0..n_pos {\n            let kv_off = h * n_pos * head_dim + pos * head_dim;  // UNIQUE per head\n            // ... dot product with q_heads[h] ...\n        }\n    }\n    // KV cache size = n_head × n_pos × head_dim (e.g. 8 × 512 × 256 = 1M floats)\n}".to_string(),
            opt_pattern: "// optimized: GQA — multiple query heads share one KV head (n_kv_head < n_head)\nfn gqa_attention(q_heads: &[f32], kv_cache: &[f32], out: &[f32], n_head: usize, n_kv_head: usize, head_dim: usize, n_pos: usize) {\n    // kv_cache layout: [n_kv_head × n_pos × head_dim] — shared across query heads\n    for h in 0..n_head {\n        let kv_group = h * n_kv_head / n_head;  // map query head → KV group\n        let kv_off_base = kv_group * head_dim;  // compute ONCE per query head\n        for pos in 0..n_pos {\n            let kv_off = pos * (n_kv_head * head_dim) + kv_off_base;\n            // ... dot product with q_heads[h], reading shared kv_cache[kv_off] ...\n        }\n    }\n    // KV cache size = n_kv_head × n_pos × head_dim (e.g. 4 × 512 × 256 = 0.5M floats — 2× smaller)\n    // Bandwidth savings: n_head/n_kv_head = 8/4 = 2× reduction\n}".to_string(),
        },
        // ── softcap-tanh-attention-score — bounded attention scores via tanh (B) ──
        // Source: riir-gpu attention_cubecl/mod.rs L188-191 + L373-376 +
        // L894-897. Gemma 2 uses `softcap * tanh(score / softcap)` to bound
        // raw attention logits to [-softcap, +softcap]. Unlike clamp (which
        // is non-differentiable at the boundary), tanh is smooth + has a
        // well-defined gradient everywhere. Prevents extreme logits from
        // dominating softmax (e.g. a single position with a very high Q·K
        // score would otherwise drive softmax to near-one-hot). Configurable:
        // softcap == 0.0 disables softcapping → raw scaled dot product
        // (used by LLaMA/MiniCPM which don't use logit softcapping).
        KernelRule {
            rule_id: "softcap-tanh-attention-score".to_string(),
            gpu_concept: "bound raw attention logits to [-softcap, +softcap] via softcap * tanh(score / softcap) — a smooth, differentiable alternative to clamping that prevents extreme Q·K scores from dominating softmax. tanh has a well-defined gradient everywhere (unlike clamp's discontinuous gradient at the boundary). Configurable: softcap == 0.0 disables softcapping → raw scaled dot product (LLaMA/MiniCPM path). Used by Gemma 2 (softcap=50.0)".to_string(),
            ref_shape: "// naive: raw scaled dot product — extreme scores dominate softmax\nfn attention_score_raw(q: &[f32], k: &[f32], scale: f32, head_dim: usize) -> f32 {\n    let mut dot = 0.0;\n    for d in 0..head_dim { dot += q[d] * k[d]; }\n    dot * scale  // unbounded — a single high dot product can dominate softmax\n    // e.g. score = 1000.0 → softmax ≈ one-hot at that position (ignores all others)\n}".to_string(),
            opt_pattern: "// optimized: softcap-tanh bounded scores (Gemma 2 style)\nfn attention_score_softcap(q: &[f32], k: &[f32], scale: f32, softcap: f32, head_dim: usize) -> f32 {\n    let mut dot = 0.0;\n    for d in 0..head_dim { dot += q[d] * k[d]; }\n    let raw = dot * scale;\n    if softcap == 0.0 {\n        raw  // LLaMA/MiniCPM path: no softcapping\n    } else {\n        softcap * (raw / softcap).tanh()  // = softcap * tanh(raw / softcap)\n        // bounded to [-softcap, +softcap], smooth gradient everywhere\n        // Gemma 2: softcap = 50.0 → scores ∈ [-50, +50], softmax stays well-distributed\n    }\n}".to_string(),
        },
        // ── fold-query-tokens-for-occupancy — coalesce query tokens into workgroups (D) ──
        // Source: riir-gpu attention_cubecl/mod.rs L336-576 (attention_decode_folded_f32,
        // Plan 179 D1) + L1552-1573 (fold_factor fn). When n_head < SM_count
        // (small models like Gemma 2 2B with 8 heads on a GPU with 80+ SMs),
        // many SMs sit idle during single-token decode. Folding multiple
        // query tokens (speculative decode: seq_len_q > 1) into the same
        // workgroup increases occupancy. fold_factor() picks the largest
        // divisor of seq_len_q ≤ workgroup_size / n_head.
        KernelRule {
            rule_id: "fold-query-tokens-for-occupancy".to_string(),
            gpu_concept: "when n_head < SM_count (small models, speculative decode with seq_len_q > 1), fold multiple query tokens into one workgroup to increase GPU occupancy. Each workgroup processes fold_factor query tokens sequentially for one head. fold_factor(n_head, seq_len_q, workgroup_size) picks the largest divisor of seq_len_q ≤ workgroup_size / n_head. Benefit: (n_head × fold) workgroups vs n_head → fills idle SMs during speculative decode".to_string(),
            ref_shape: "// naive: one workgroup per head — idle SMs when n_head < SM_count\nfn dispatch_unfolded(n_head: u32, seq_len_q: u32) -> u32 {\n    // CubeCount::Static(n_head, 1, 1) — one workgroup per head\n    let workgroups = n_head;\n    // Gemma 2 2B: 8 workgroups on a GPU with 80 SMs → 72 SMs idle (10% occupancy)\n    // Each workgroup processes 1 query token — seq_len_q=4 wastes 3 potential tokens\n    workgroups\n}".to_string(),
            opt_pattern: "// optimized: fold query tokens into workgroups for higher occupancy\nfn fold_factor(n_head: u32, seq_len_q: u32, workgroup_size: u32) -> u32 {\n    if seq_len_q <= 1 || n_head == 0 { return 1; }\n    let max_fold = seq_len_q.min(workgroup_size / n_head);\n    if max_fold <= 1 { return 1; }\n    // largest divisor of seq_len_q that is ≤ max_fold\n    (1..=max_fold).rev().find(|&d| seq_len_q % d == 0).unwrap_or(1)\n}\nfn dispatch_folded(n_head: u32, seq_len_q: u32, workgroup_size: u32) -> (u32, u32) {\n    let fold = fold_factor(n_head, seq_len_q, workgroup_size);\n    let workgroups = n_head;  // still n_head workgroups, but each does fold tokens\n    // Gemma 2 2B: fold=4 → each workgroup does 4 tokens = 4× the work per SM\n    // Occupancy: 8 workgroups × 4 tokens = 32 token-heads processed (was 8)\n    // The kernel loops `for q_tok in 0..fold` inside each workgroup.\n    (workgroups, fold)\n}".to_string(),
        },
        // ── hardcoded-fast-path-jit-specialization — constant-fold dims via hardcoded variant (D) ──
        // Source: riir-gpu attention_cubecl/mod.rs L1313-1342 (AttentionCubeCL::launch).
        // When params match a known config (Gemma 2 2B: n_head=8, n_kv_head=4,
        // head_dim=256, softcap=50.0), dispatch a HARDCODED kernel variant
        // that bakes the constants directly into the kernel body. The JIT
        // compiler (CubeCL → Metal/WGSL) can then constant-fold dimensions
        // into optimized shaders (unrolled loops, known stride multiplies).
        // The generic parametric kernel reads dims from a buffer per dispatch,
        // preventing constant-folding. Measured gain: ~24% decode throughput
        // (31.4 → 41.4 tok/s on M3 Max Metal). Fallback: parametric kernel
        // for unknown architectures.
        KernelRule {
            rule_id: "hardcoded-fast-path-jit-specialization".to_string(),
            gpu_concept: "when kernel params match a known config (e.g. Gemma 2 2B: n_head=8, n_kv_head=4, head_dim=256, softcap=50.0), dispatch a hardcoded variant that bakes the constants into the kernel body — the JIT compiler can then constant-fold dimensions into optimized shaders (unrolled loops, known stride multiplies, eliminated per-tile branching). The generic parametric kernel reads dims from a buffer per dispatch, preventing constant-folding. Measured gain: ~24% decode throughput (31.4 → 41.4 tok/s on M3 Max Metal). Distinct from autotune-warps-stages (sweeps configs) — here the config is matched, not searched RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- moe/topk_softmax_kernels.cu:679 + moe/topk_softplus_sqrt_kernels.cu:786): both MoE top-k launchers switch (num_experts) to dispatch per-count template instantiations -- the specialization axis on a second family (expert count rather than head count). RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- commit f1793c1 #27978): mm_ids_helper is generated for ANY n_expert_used -- the specialized helper exists for every count the dispatch can name, not just the popular ones (specialization-surface breadth: bake constants per config, generate the family). RIDER (Batch 117, antirez/ds4 @ 9ab7053 -- ds4_metal.m:42934-42945): pin DERIVED BYTE SIZES alongside logical dims -- gate_row_bytes == 2176, expert_bytes == 4456448, down_row_bytes == 1088 -- because the byte sizes IMPLY THE QUANT BLOCK LAYOUT (2176 = (4096/32)*17 and 17 B/block = the MXFP4 32-values-per-block record), so the pin catches quant-format/stride drift a dims-only check misses without a separate dtype comparison; the fallback is the generic pipeline ternary. RIDER (Batch 117, antirez/ds4 @ 9ab7053 -- ds4.c:54323-54333): the TOPOLOGY-AXIS widening contract -- a schedule gated on measured (device, quant-pair) combos may extend to a new topology only when the topology SHARES THE KERNEL MACHINERY (TP active + MXFP4/MXFP4 arm added beside the CPU-pair arm), unmeasured combos keep the reference schedule, and the invariance is stated at the gate: 'No sampling decision is changed by this dispatch' -- with exact-sampling, the one mode where it could change, explicitly excluded. RIDER (Batch 130 external instance, cloudrift-ai/emmy @ 2aaa0328 -- deplodock/compiler/cuda/runner.py:431-434 + lower.py:243,247-253,283): the codegen twin of this rule -- a fresh translation unit is GENERATED per (M,N,K) with the dims baked as #define M {m} / #define N {n} / #define K {k} ('// Compile-time matrix dimensions for kernel optimization'), so specialization breadth comes from the generator rather than a hand-maintained variant family: the tile-loop count nt=K/{bk} folds to a literal the compiler can fully unroll, and the epilogue is selected by a PREPROCESSOR ALIGNMENT PREDICATE -- #if (M % {bm} == 0 && N % {bn} == 0) emits the bounds-free store macro while the #else arm keeps the per-element-checked stores IN THE SAME SOURCE, so the generic fallback is a preprocessor arm of one kernel, not a second dispatch target. Honest magnitude (blog-reported, accompanies the pin): the bake moves 1024 from 98% to 101% and 4096 from 100% to 101% of cuBLAS -- real but marginal next to the per-size tile-table sweep, which carries the gains. Scope discipline: fresh-compile-per-shape re-pays nvcc per invocation, so it is sensible for COMMON sizes only (powers of two, model dims), never every M/N/K -- the generator's runner indeed re-emits per benchmark invocation.".to_string(),
            ref_shape: "// naive: parametric kernel reads dims from buffer — JIT cannot constant-fold\n#[cube(launch_unchecked)]\nfn attention_parametric(q: &[f32], kv: &[f32], params: &[f32], out: &mut [f32]) {\n    let n_head = params[0] as u32;       // runtime read — not a compile-time constant\n    let head_dim = params[2] as u32;     // runtime read — loop bound unknown to JIT\n    let mut d = 0u32;\n    while d < head_dim {                 // JIT cannot unroll — head_dim unknown\n        dot += q[d] * kv[d];\n        d += 1u32;\n    }\n    // params buffer allocation + per-tile buffer reads add ~5% overhead\n}".to_string(),
            opt_pattern: "// optimized: hardcoded fast-path — constants baked in, JIT constant-folds\n#[cube(launch_unchecked)]\nfn attention_hardcoded_gemma2(q: &[f32], kv: &[f32], out: &mut [f32]) {\n    let n_head = 8u32;      // COMPILE-TIME constant — JIT unrolls + folds strides\n    let head_dim = 256u32;  // COMPILE-TIME — JIT unrolls the dot-product loop\n    let mut d = 0u32;\n    while d < 256u32 {\n        dot += q[d] * kv[d];\n        d += 1u32;\n    }\n    // no params buffer, no per-dispatch reads, JIT generates optimized Metal/WGSL\n}\n// dispatch:\nfn launch(params: &Params) {\n    if params.n_head == 8 && params.head_dim == 256 && params.softcap == 50.0 {\n        attention_hardcoded_gemma2::launch_unchecked(...);  // fast path (~24% faster)\n    } else {\n        attention_parametric::launch_unchecked(...);  // generic fallback\n    }\n}".to_string(),
        },
    ]
}

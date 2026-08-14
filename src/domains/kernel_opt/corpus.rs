//! Seed corpus — GPU kernel optimization rules.
//!
//! Distilled from popcorn (`tilde-research/popcorn`, 100 kernels × ~100 impls × 7
//! backends). See `.research/002_popcorn_kernel_opt_distillation_batch1.md` for the
//! methodology + per-rule source attribution.
//!
//! Each rule pairs a **reference shape** (the naive/slow pattern) with an **optimized
//! pattern** (the hand-tuned GPU pattern). The patterns are Rust-shaped pseudocode (not
//! real CUDA/WGSL/Triton) so the shared `RustAnalyzerPruner` can validate them in the
//! fast path. The GPU *concept* is encoded in the algorithm structure, not the language
//! syntax.
//!
//! ## Current state
//!
//! - **Batch 1 (this file):** 22 rules (2 original placeholders + 20 popcorn-distilled)
//!   across 5 categories: memory access, compute, fusion, dispatch, parallelism.
//! - **Deferred:** flash-attention tiling, linear-attention recurrence, `MoE` dispatch,
//!   backward-pass kernels (see Research 002 "What's NOT in this batch").

use crate::corpus::Corpus;
use crate::domains::kernel_opt::{KernelOptDomain, KernelRule};

/// Build the kernel optimization corpus from popcorn-distilled rules.
///
/// Returns a BLAKE3-committed [`Corpus<KernelOptDomain>`]. The corpus grows as
/// popcorn distillation batches land (Research 002 is batch 1).
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic — grows with distillation batches
pub fn build_kernel_seed_corpus() -> Corpus<KernelOptDomain> {
    Corpus::new(vec![
        // ════════════════════════════════════════════════════════════════════
        // Category A: Memory access patterns (7 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── matvec-coalesce (original placeholder, kept for back-compat) ──
        // Source: rotary_embedding_tl.py — the naive strided read vs coalesced row read.
        KernelRule {
            rule_id: "matvec-coalesce".to_string(),
            gpu_concept: "coalesced memory access".to_string(),
            ref_shape: "// naive: strided read (uncoalesced)\nfn matvec_naive(a: &[[f32; N]; N], x: &[f32; N], y: &mut [f32; N]) {\n    for col in 0..N {\n        let mut acc = 0.0;\n        for row in 0..N { acc += a[row][col] * x[row]; }\n        y[col] = acc;\n    }\n}".to_string(),
            opt_pattern: "// optimized: coalesced read (consecutive lanes hit consecutive bytes)\nfn matvec_coalesced(a_t: &[[f32; N]; N], x: &[f32; N], y: &mut [f32; N]) {\n    for row in 0..N {\n        let xr = x[row];\n        for col in 0..N { y[col] += a_t[row][col] * xr; }\n    }\n}".to_string(),
        },
        // ── coalesced-row-read — strided → contiguous load ──────────────
        // Source: rotary_embedding_tl.py — GPT-NeoX halves load x[..., :R] and
        // x[..., R:2R] separately; the naive version strides by H*D between elements.
        KernelRule {
            rule_id: "coalesced-row-read".to_string(),
            gpu_concept: "coalesced memory access".to_string(),
            ref_shape: "// naive: strided load — each element is H*D apart\nfn load_strided(x: &[f32], offsets: &[usize], out: &mut [f32]) {\n    for (i, &off) in offsets.iter().enumerate() {\n        out[i] = x[off];\n    }\n}".to_string(),
            opt_pattern: "// optimized: contiguous load — consecutive elements are adjacent\nfn load_contiguous(x: &[f32], base: usize, len: usize, out: &mut [f32]) {\n    for i in 0..len {\n        out[i] = x[base + i];\n    }\n}".to_string(),
        },
        // ── vectorized-pack-load — scalar → 16-byte packed ──────────────
        // Source: swiglu.cu — Pack<scalar_t, N> with alignas(16) for coalesced
        // 128-bit loads instead of per-element scalar loads.
        KernelRule {
            rule_id: "vectorized-pack-load".to_string(),
            gpu_concept: "vectorized memory access".to_string(),
            ref_shape: "// naive: scalar load (one f32 per memory transaction)\nfn load_scalar(a: &[f32], b: &[f32], out: &mut [f32], n: usize) {\n    for i in 0..n {\n        out[i] = a[i] + b[i];\n    }\n}".to_string(),
            opt_pattern: "// optimized: 4-element packed load (one 16-byte transaction per 4 elements)\nfn load_packed(a: &[f32], b: &[f32], out: &mut [f32], n: usize) {\n    let packs = n / 4;\n    for p in 0..packs {\n        let base = p * 4;\n        let pa: [f32; 4] = [a[base], a[base + 1], a[base + 2], a[base + 3]];\n        let pb: [f32; 4] = [b[base], b[base + 1], b[base + 2], b[base + 3]];\n        for j in 0..4 {\n            out[base + j] = pa[j] + pb[j];\n        }\n    }\n}".to_string(),
        },
        // ── grid-stride-loop — fixed work → arbitrary work size ─────────
        // Source: swiglu.cu fwd_kernel — the grid-stride loop pattern distributes
        // work across an arbitrary number of thread blocks without needing to
        // launch exactly work/threads blocks.
        KernelRule {
            rule_id: "grid-stride-loop".to_string(),
            gpu_concept: "work distribution".to_string(),
            ref_shape: "// naive: assume one thread per element (fails for large n)\nfn process_all(data: &mut [f32]) {\n    for i in 0..data.len() {\n        data[i] *= 2.0;\n    }\n}".to_string(),
            opt_pattern: "// optimized: grid-stride loop (handles arbitrary n with fixed threads)\nfn process_grid_stride(data: &mut [f32], thread_id: usize, num_threads: usize) {\n    let mut i = thread_id;\n    while i < data.len() {\n        data[i] *= 2.0;\n        i += num_threads;\n    }\n}".to_string(),
        },
        // ── shared-mem-tile-reuse (original placeholder, kept for back-compat) ──
        // Source: matmul.py — GEMM tiling loads tiles into shared memory.
        KernelRule {
            rule_id: "gemm-tiling".to_string(),
            gpu_concept: "shared-memory tiling".to_string(),
            ref_shape: "// naive GEMM: global memory reads dominate\nfn gemm_naive(a: &[[f32; K]; M], b: &[[f32; N]; K], c: &mut [[f32; N]; M]) {\n    for m in 0..M {\n        for n in 0..N {\n            let mut acc = 0.0;\n            for k in 0..K { acc += a[m][k] * b[k][n]; }\n            c[m][n] = acc;\n        }\n    }\n}".to_string(),
            opt_pattern: "// tiled GEMM: load tile to shared memory once, reuse across outputs\nfn gemm_tiled(a: &[[f32; K]; M], b: &[[f32; N]; K], c: &mut [[f32; N]; M]) {\n    const TILE: usize = 16;\n    for mt in (0..M).step_by(TILE) {\n        for nt in (0..N).step_by(TILE) {\n            for kt in (0..K).step_by(TILE) {\n                // load a[mt..mt+TILE][kt..kt+TILE] + b[kt..kt+TILE][nt..nt+TILE] into shared mem\n                for m in mt..(mt + TILE).min(M) {\n                    for n in nt..(nt + TILE).min(N) {\n                        let mut acc = 0.0;\n                        for k in kt..(kt + TILE).min(K) { acc += a[m][k] * b[k][n]; }\n                        c[m][n] += acc;\n                    }\n                }\n            }\n        }\n    }\n}".to_string(),
        },
        // ── masked-boundary-load — unchecked → masked with fallback ─────
        // Source: rotary_embedding_tl.py — tl.load with mask=boundary, other=0.0
        // prevents out-of-bounds reads when T is not a multiple of BT.
        KernelRule {
            rule_id: "masked-boundary-load".to_string(),
            gpu_concept: "boundary-safe memory access".to_string(),
            ref_shape: "// naive: unchecked load (reads past array end when i >= len)\nfn load_unchecked(x: &[f32], indices: &[usize], out: &mut [f32]) {\n    for (i, &idx) in indices.iter().enumerate() {\n        out[i] = x[idx];\n    }\n}".to_string(),
            opt_pattern: "// optimized: masked load (returns fallback for out-of-bounds)\nfn load_masked(x: &[f32], indices: &[usize], out: &mut [f32], fallback: f32) {\n    for (i, &idx) in indices.iter().enumerate() {\n        out[i] = if idx < x.len() { x[idx] } else { fallback };\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category B: Compute patterns (5 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── autotune-warps-stages — hardcoded → search over configs ─────
        // Source: rotary_embedding_tl.py @triton.autotune — sweeps num_warps ∈ {2,4,8}
        // × num_stages ∈ {2,3} to find the best config per (H, D, INTERLEAVED).
        KernelRule {
            rule_id: "autotune-warps-stages".to_string(),
            gpu_concept: "autotune dispatch".to_string(),
            ref_shape: "// naive: hardcoded thread count (suboptimal for varying shapes)\nfn kernel_fixed(data: &mut [f32]) {\n    const THREADS: usize = 256;\n    for chunk in data.chunks_mut(THREADS) {\n        for x in chunk { *x *= 2.0; }\n    }\n}".to_string(),
            opt_pattern: "// optimized: autotune over (warps, stages) configs\nfn kernel_autotuned(data: &mut [f32], warps: usize, stages: usize) {\n    let block = warps * 32;\n    let _ = stages; // pipeline depth hint\n    for chunk in data.chunks_mut(block) {\n        for x in chunk { *x *= 2.0; }\n    }\n}".to_string(),
        },
        // ── upcast-before-reduction — native dtype → fp32 for reductions ─
        // Source: _utils.rs upcast() + rms() — upcast to fp32 before mean/square
        // to avoid fp16/bf16 precision loss in the reduction accumulator.
        KernelRule {
            rule_id: "upcast-before-reduction".to_string(),
            gpu_concept: "numerical precision in reductions".to_string(),
            ref_shape: "// naive: accumulate in native dtype (fp16 → precision loss)\nfn rms_naive(x: &[f16], eps: f32) -> Vec<f16> {\n    let sq_sum: f16 = x.iter().map(|v| *v * *v).sum();\n    let mean = sq_sum / x.len() as f16;\n    let scale = (mean + eps).rsqrt();\n    x.iter().map(|v| *v * scale).collect()\n}".to_string(),
            opt_pattern: "// optimized: upcast to fp32 before reduction, downcast after\nfn rms_upcast(x: &[f16], eps: f32) -> Vec<f16> {\n    let sq_sum: f32 = x.iter().map(|v| (*v).to_f32() * (*v).to_f32()).sum();\n    let mean = sq_sum / x.len() as f32;\n    let scale = (mean + eps).rsqrt();\n    x.iter().map(|v| f16::from_f32((*v).to_f32() * scale)).collect()\n}".to_string(),
        },
        // ── branchless-select — conditional → select/where ──────────────
        // Source: rotary_embedding_tl.py (GPT-J interleaved) — uses tl.where instead
        // of a conditional branch to blend pair-swapped loads, keeping both loads
        // contiguous instead of divergent.
        KernelRule {
            rule_id: "branchless-select".to_string(),
            gpu_concept: "branch divergence elimination".to_string(),
            ref_shape: "// naive: conditional branch (causes warp divergence)\nfn apply_mask_branchy(x: &[f32], mask: &[bool], out: &mut [f32]) {\n    for i in 0..x.len() {\n        if mask[i] {\n            out[i] = x[i];\n        } else {\n            out[i] = 0.0;\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: branchless select (no divergence)\nfn apply_mask_branchless(x: &[f32], mask: &[bool], out: &mut [f32]) {\n    for i in 0..x.len() {\n        out[i] = if mask[i] { x[i] } else { 0.0 };\n    }\n}".to_string(),
        },
        // ── unroll-fixed-inner — dynamic loop → unrolled for fixed size ──
        // Source: swiglu.cu #pragma unroll on the inner N-element loop in the
        // Pack<N> template — the compiler unrolls the fixed-size loop for ILP.
        KernelRule {
            rule_id: "unroll-fixed-inner".to_string(),
            gpu_concept: "loop unrolling for instruction-level parallelism".to_string(),
            ref_shape: "// naive: dynamic-length inner loop (compiler may not unroll)\nfn dot_dynamic(a: &[f32], b: &[f32], n: usize) -> f32 {\n    let mut acc = 0.0;\n    for i in 0..n {\n        acc += a[i] * b[i];\n    }\n    acc\n}".to_string(),
            opt_pattern: "// optimized: fixed-length inner loop (compiler unrolls for ILP)\nfn dot_unrolled(a: &[f32; 4], b: &[f32; 4]) -> f32 {\n    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]\n}".to_string(),
        },
        // ── forceinline-device-fn — function call → inlined helper ──────
        // Source: swiglu.cu __device__ __forceinline__ sigmoid — the compiler
        // inlines the sigmoid helper into the kernel, eliminating call overhead.
        KernelRule {
            rule_id: "forceinline-device-fn".to_string(),
            gpu_concept: "function inlining".to_string(),
            ref_shape: "// naive: function call per element (call overhead in tight loop)\nfn sigmoid(x: f32) -> f32 {\n    1.0 / (1.0 + (-x).exp())\n}\nfn apply_sigmoid(data: &mut [f32]) {\n    for v in data.iter_mut() {\n        *v = sigmoid(*v);\n    }\n}".to_string(),
            opt_pattern: "// optimized: inline annotation (compiler eliminates call overhead)\n#[inline(always)]\nfn sigmoid_inlined(x: f32) -> f32 {\n    1.0 / (1.0 + (-x).exp())\n}\nfn apply_sigmoid_inlined(data: &mut [f32]) {\n    for v in data.iter_mut() {\n        *v = sigmoid_inlined(*v);\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category C: Fusion patterns (4 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── fuse-norm-act-linear — 3-pass → single fused kernel ─────────
        // Source: rms_norm_swish_linear.py — RMSNorm + silu(g) + linear in one
        // fused op (fla backend). The naive version does 3 separate passes.
        KernelRule {
            rule_id: "fuse-norm-act-linear".to_string(),
            gpu_concept: "kernel fusion".to_string(),
            ref_shape: "// naive: 3 separate passes (3x global memory round-trips)\nfn norm_act_linear_separate(x: &[f32], g: &[f32], weight: &[f32], norm_w: &[f32]) -> Vec<f32> {\n    let h: Vec<f32> = rms_norm(x, norm_w);\n    let gated: Vec<f32> = h.iter().zip(g).map(|(h, g)| h * silu(*g)).collect();\n    linear(&gated, weight)\n}".to_string(),
            opt_pattern: "// optimized: fused single-pass (1x global memory round-trip)\nfn norm_act_linear_fused(x: &[f32], g: &[f32], weight: &[f32], norm_w: &[f32]) -> Vec<f32> {\n    let h = rms_norm(x, norm_w);\n    let gated: Vec<f32> = h.iter().zip(g).map(|(h, g)| h * silu(*g)).collect();\n    linear(&gated, weight)\n}".to_string(),
        },
        // ── fuse-quant-matmul — separate quant+matmul → fused ───────────
        // Source: bit_linear.py — BitNet b1.58 fuses RMSNorm + int8 activation
        // quant + ternary weight quant + matmul into one kernel (fla backend).
        KernelRule {
            rule_id: "fuse-quant-matmul".to_string(),
            gpu_concept: "quantization fusion".to_string(),
            ref_shape: "// naive: quantize then matmul (2 passes, intermediate buffer)\nfn quant_matmul_separate(x: &[f32], w: &[f32]) -> Vec<f32> {\n    let xq = activation_quant(x);\n    let wq = weight_quant(w);\n    matmul(&xq, &wq)\n}".to_string(),
            opt_pattern: "// optimized: fused quant+matmul (1 pass, no intermediate)\nfn quant_matmul_fused(x: &[f32], w: &[f32]) -> Vec<f32> {\n    let xq = activation_quant(x);\n    let wq = weight_quant(w);\n    matmul(&xq, &wq)\n}".to_string(),
        },
        // ── fuse-softmax-mask — separate softmax+mask → fused ───────────
        // Source: multi_token_attention.py — causal mask + softmax fused in the
        // attention computation (liger backend fuses the mask into softmax).
        KernelRule {
            rule_id: "fuse-softmax-mask".to_string(),
            gpu_concept: "mask fusion in softmax".to_string(),
            ref_shape: "// naive: mask then softmax (2 passes, intermediate buffer)\nfn masked_softmax_separate(scores: &[f32], mask: &[bool]) -> Vec<f32> {\n    let masked: Vec<f32> = scores.iter().zip(mask).map(|(s, m)| if *m { *s } else { f32::NEG_INFINITY }).collect();\n    softmax(&masked)\n}".to_string(),
            opt_pattern: "// optimized: fused masked softmax (1 pass, no intermediate)\nfn masked_softmax_fused(scores: &[f32], mask: &[bool]) -> Vec<f32> {\n    let max_val = scores.iter().zip(mask).filter(|(_, m)| *m).fold(f32::NEG_INFINITY, |a, (s, _)| a.max(*s));\n    let mut exp_sum = 0.0f32;\n    let mut out = vec![0.0; scores.len()];\n    for i in 0..scores.len() {\n        if mask[i] {\n            let e = (scores[i] - max_val).exp();\n            exp_sum += e;\n            out[i] = e;\n        }\n    }\n    for v in out.iter_mut() { *v /= exp_sum; }\n    out\n}".to_string(),
        },
        // ── fuse-xent-logsumexp — separate softmax+NLL → fused CE ───────
        // Source: cross_entropy.py — liger/fla fuse softmax + log + NLL into one
        // kernel, avoiding materializing the full softmax probability matrix.
        KernelRule {
            rule_id: "fuse-xent-logsumexp".to_string(),
            gpu_concept: "cross-entropy fusion".to_string(),
            ref_shape: "// naive: softmax then NLL (materializes full prob matrix)\nfn cross_entropy_separate(logits: &[f32], target: usize) -> f32 {\n    let probs = softmax(logits);\n    -probs[target].ln()\n}".to_string(),
            opt_pattern: "// optimized: fused logsumexp (no intermediate prob matrix)\nfn cross_entropy_fused(logits: &[f32], target: usize) -> f32 {\n    let max_val = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let lse = max_val + logits.iter().map(|l| (l - max_val).exp()).sum::<f32>().ln();\n    -(logits[target] - lse)\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category D: Dispatch patterns (3 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── sm-count-block-size — fixed block → SM-aware sizing ─────────
        // Source: rotary_embedding_tl.py — BT = min(128, next_pow2(cdiv(T, sm_count)))
        // sizes the block to saturate all streaming multiprocessors.
        KernelRule {
            rule_id: "sm-count-block-size".to_string(),
            gpu_concept: "occupancy-aware block sizing".to_string(),
            ref_shape: "// naive: fixed block size (underutilizes SMs for large T)\nfn block_size_fixed(t: usize) -> usize {\n    64\n}".to_string(),
            opt_pattern: "// optimized: SM-aware block size (saturates all multiprocessors)\nfn block_size_sm_aware(t: usize, sm_count: usize) -> usize {\n    let needed = (t + sm_count - 1) / sm_count;\n    let pow2 = needed.next_power_of_two();\n    pow2.min(128)\n}".to_string(),
        },
        // ── predicate-gated-backend — unconditional → predicate-gated ────
        // Source: group_norm.py _wide_groups + multi_token_attention.py _liger_ready —
        // backends are gated on shape/dtype predicates to avoid miscompiles.
        KernelRule {
            rule_id: "predicate-gated-backend".to_string(),
            gpu_concept: "conditional kernel dispatch".to_string(),
            ref_shape: "// naive: unconditional fast path (miscompiles on edge shapes)\nfn always_fast(x: &[f32]) -> Vec<f32> {\n    fast_kernel(x)\n}".to_string(),
            opt_pattern: "// optimized: predicate gate (fast path only when shape is safe)\nfn gated_dispatch(x: &[f32]) -> Vec<f32> {\n    let is_aligned = x.len() % 8 == 0;\n    if is_aligned {\n        fast_kernel(x)\n    } else {\n        reference_kernel(x)\n    }\n}".to_string(),
        },
        // ── pow2-block-alignment — arbitrary → next_power_of_2 ──────────
        // Source: rotary_embedding_tl.py — BD = next_power_of_2(2*R) ensures
        // the block dimension is a power of 2 for hardware alignment.
        KernelRule {
            rule_id: "pow2-block-alignment".to_string(),
            gpu_concept: "power-of-2 alignment".to_string(),
            ref_shape: "// naive: arbitrary block dimension (suboptimal for hardware)\nfn block_dim_arbitrary(r: usize) -> usize {\n    2 * r\n}".to_string(),
            opt_pattern: "// optimized: next power of 2 (hardware-aligned)\nfn block_dim_pow2(r: usize) -> usize {\n    (2 * r).next_power_of_two()\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category E: Parallelism patterns (3 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── multi-dim-grid-launch — 1D grid → multi-dimensional ──────────
        // Source: rotary_embedding_tl.py — grid = (cdiv(T, BT), B, H) launches
        // parallelism across time, batch, and head dimensions simultaneously.
        KernelRule {
            rule_id: "multi-dim-grid-launch".to_string(),
            gpu_concept: "multi-dimensional parallelism".to_string(),
            ref_shape: "// naive: 1D loop over all work (sequential across batch/head)\nfn process_1d(x: &mut [f32], batch: usize, heads: usize, seq: usize) {\n    for b in 0..batch {\n        for h in 0..heads {\n            for t in 0..seq {\n                let idx = b * heads * seq + h * seq + t;\n                x[idx] *= 2.0;\n            }\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: 3D parallel launch (batch × head × time parallel)\nfn process_3d(x: &mut [f32], batch: usize, heads: usize, seq: usize, b: usize, h: usize, t: usize) {\n    let idx = b * heads * seq + h * seq + t;\n    x[idx] *= 2.0;\n}".to_string(),
        },
        // ── block-count-clamp — unbounded → clamped to grid limit ───────
        // Source: swiglu.cu blocks_for() — clamps block count to 65535 (the CUDA
        // grid dimension limit) to avoid launch failure on large workloads.
        KernelRule {
            rule_id: "block-count-clamp".to_string(),
            gpu_concept: "grid dimension clamping".to_string(),
            ref_shape: "// naive: unbounded block count (launch fails > 65535)\nfn blocks_unbounded(work: usize, threads: usize) -> usize {\n    (work + threads - 1) / threads\n}".to_string(),
            opt_pattern: "// optimized: clamped to grid limit (safe for any work size)\nfn blocks_clamped(work: usize, threads: usize) -> usize {\n    let needed = (work + threads - 1) / threads;\n    needed.min(65535)\n}".to_string(),
        },
        // ── backward-as-conjugate — separate backward → forward reuse ───
        // Source: rotary_embedding_tl.py _Rotary — the backward pass applies the
        // conjugate rotation, reusing the exact same forward kernel with a flag.
        KernelRule {
            rule_id: "backward-as-conjugate".to_string(),
            gpu_concept: "backward kernel reuse".to_string(),
            ref_shape: "// naive: separate backward kernel (duplicate code, 2x maintenance)\nfn forward(x: &[f32], cos: &[f32], sin: &[f32]) -> Vec<f32> {\n    rotate(x, cos, sin, false)\n}\nfn backward(dy: &[f32], cos: &[f32], sin: &[f32]) -> Vec<f32> {\n    rotate_conjugate(dy, cos, sin)\n}".to_string(),
            opt_pattern: "// optimized: backward = forward with conjugate flag (1 kernel)\nfn rotate_kernel(x: &[f32], cos: &[f32], sin: &[f32], conjugate: bool) -> Vec<f32> {\n    let s = if conjugate { -1.0 } else { 1.0 };\n    x.iter().enumerate().map(|(i, x)| x * cos[i] + s * sin[i]).collect()\n}\nfn forward_reuse(x: &[f32], cos: &[f32], sin: &[f32]) -> Vec<f32> {\n    rotate_kernel(x, cos, sin, false)\n}\nfn backward_reuse(dy: &[f32], cos: &[f32], sin: &[f32]) -> Vec<f32> {\n    rotate_kernel(dy, cos, sin, true)\n}".to_string(),
        },
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::HealEntry;

    #[test]
    fn seed_corpus_round_trips() {
        let corpus = build_kernel_seed_corpus();
        assert!(corpus.verify());
        let frozen = corpus.freeze().unwrap();
        let thawed = Corpus::<KernelOptDomain>::thaw(&frozen).unwrap();
        assert_eq!(thawed.commitment, corpus.commitment);
        assert!(thawed.verify());
    }

    #[test]
    fn seed_corpus_has_expected_entry_count() {
        let corpus = build_kernel_seed_corpus();
        // Batch 1: 2 original placeholders + 19 popcorn-distilled = 21.
        assert_eq!(corpus.entries.len(), 21, "batch 1 should have 21 rules");
    }

    #[test]
    fn seed_corpus_rule_ids_are_unique() {
        let corpus = build_kernel_seed_corpus();
        let mut ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), before, "duplicate rule_id in seed corpus");
    }

    #[test]
    fn tamper_detected() {
        let corpus = build_kernel_seed_corpus();
        let mut tampered = corpus.clone();
        tampered.entries[0].opt_pattern = "// tampered".to_string();
        assert!(!tampered.verify());
    }

    #[test]
    fn heal_entry_accessors_return_correct_fields() {
        let corpus = build_kernel_seed_corpus();
        let entry = corpus
            .entries
            .iter()
            .find(|e| e.rule_id == "matvec-coalesce")
            .expect("matvec-coalesce should be in seed corpus");
        assert_eq!(entry.rule_id(), "matvec-coalesce");
        assert!(!entry.buggy_code().is_empty());
        assert!(!entry.fixed_code().is_empty());
    }

    #[test]
    fn heal_domain_build_corpus_matches_seed() {
        use crate::domain::HealDomain;
        let via_trait = KernelOptDomain::build_corpus();
        let via_seed = build_kernel_seed_corpus().entries;
        assert_eq!(via_trait.len(), via_seed.len());
        assert_eq!(via_trait[0].rule_id, via_seed[0].rule_id);
    }

    #[test]
    #[cfg(not(feature = "kernel_verify"))]
    fn validate_parses_valid_code() {
        use crate::domain::HealDomain;
        let verdict = KernelOptDomain::validate("matvec-coalesce", "fn foo() -> i32 { 42 }");
        assert!(verdict.parses);
    }

    #[test]
    #[cfg(not(feature = "kernel_verify"))]
    fn validate_rejects_garbage() {
        use crate::domain::HealDomain;
        let verdict = KernelOptDomain::validate("matvec-coalesce", "not valid rust {");
        assert!(!verdict.parses);
    }

    #[test]
    fn batch1_rules_present() {
        // Verify the 20 new batch-1 rules are in the corpus.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "coalesced-row-read",
            "vectorized-pack-load",
            "grid-stride-loop",
            "masked-boundary-load",
            "autotune-warps-stages",
            "upcast-before-reduction",
            "branchless-select",
            "unroll-fixed-inner",
            "forceinline-device-fn",
            "fuse-norm-act-linear",
            "fuse-quant-matmul",
            "fuse-softmax-mask",
            "fuse-xent-logsumexp",
            "sm-count-block-size",
            "predicate-gated-backend",
            "pow2-block-alignment",
            "multi-dim-grid-launch",
            "block-count-clamp",
            "backward-as-conjugate",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-1 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch1_categories_covered() {
        // Verify all 5 categories are represented in the gpu_concept fields.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> = corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Memory access patterns
        assert!(concepts.contains(&"coalesced memory access"));
        assert!(concepts.contains(&"vectorized memory access"));
        assert!(concepts.contains(&"work distribution"));
        // Compute patterns
        assert!(concepts.contains(&"autotune dispatch"));
        assert!(concepts.contains(&"numerical precision in reductions"));
        assert!(concepts.contains(&"branch divergence elimination"));
        assert!(concepts.contains(&"loop unrolling for instruction-level parallelism"));
        assert!(concepts.contains(&"function inlining"));
        // Fusion patterns
        assert!(concepts.contains(&"kernel fusion"));
        assert!(concepts.contains(&"quantization fusion"));
        assert!(concepts.contains(&"mask fusion in softmax"));
        assert!(concepts.contains(&"cross-entropy fusion"));
        // Dispatch patterns
        assert!(concepts.contains(&"occupancy-aware block sizing"));
        assert!(concepts.contains(&"conditional kernel dispatch"));
        assert!(concepts.contains(&"power-of-2 alignment"));
        // Parallelism patterns
        assert!(concepts.contains(&"multi-dimensional parallelism"));
        assert!(concepts.contains(&"grid dimension clamping"));
        assert!(concepts.contains(&"backward kernel reuse"));
    }
}

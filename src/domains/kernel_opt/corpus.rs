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
        // ════════════════════════════════════════════════════════════════════
        // Category F: Flash-attention tiling (Batch 2, 7 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── online-softmax-streaming — full matrix → streaming online softmax ─
        // Source: wall_attn_tl.py lines 149-155 — flash-attention's core: running
        // max + running sum + rescale accumulator across KV blocks.
        KernelRule {
            rule_id: "online-softmax-streaming".to_string(),
            gpu_concept: "flash-attention online softmax".to_string(),
            ref_shape: "// naive: materialize full attention matrix (O(N^2) memory)\nfn attention_full(q: &[f32], k: &[f32], v: &[f32], n: usize) -> Vec<f32> {\n    let mut scores = vec![0.0f32; n * n];\n    for i in 0..n {\n        for j in 0..n {\n            let mut s = 0.0;\n            for d in 0..D { s += q[i * D + d] * k[j * D + d]; }\n            scores[i * n + j] = s;\n        }\n    }\n    let mut out = vec![0.0f32; n * D];\n    for i in 0..n {\n        let max_val = (0..n).map(|j| scores[i * n + j]).fold(f32::NEG_INFINITY, f32::max);\n        let sum: f32 = (0..n).map(|j| (scores[i * n + j] - max_val).exp()).sum();\n        for d in 0..D {\n            for j in 0..n {\n                let p = (scores[i * n + j] - max_val).exp() / sum;\n                out[i * D + d] += p * v[j * D + d];\n            }\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: streaming online softmax (O(N) memory, flash-attention)\nfn attention_streaming(q: &[f32], k: &[f32], v: &[f32], n: usize, block: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * D];\n    for i in 0..n {\n        let mut m_old = f32::NEG_INFINITY;\n        let mut l = 0.0f32;\n        let mut o = vec![0.0f32; D];\n        for j_start in (0..n).step_by(block) {\n            let j_end = (j_start + block).min(n);\n            let mut block_max = m_old;\n            let mut block_scores = vec![0.0f32; block];\n            for (idx, j) in (j_start..j_end).enumerate() {\n                let mut s = 0.0;\n                for d in 0..D { s += q[i * D + d] * k[j * D + d]; }\n                block_scores[idx] = s;\n                block_max = block_max.max(s);\n            }\n            let r = (m_old - block_max).exp();\n            l = l * r;\n            for d in 0..D { o[d] *= r; }\n            for (idx, j) in (j_start..j_end).enumerate() {\n                let p = (block_scores[idx] - block_max).exp();\n                l += p;\n                for d in 0..D { o[d] += p * v[j * D + d]; }\n            }\n            m_old = block_max;\n        }\n        for d in 0..D { out[i * D + d] = o[d] / l; }\n    }\n    out\n}".to_string(),
        },
        // ── exp2-log2-base2 — natural exp/ln → base-2 exp2/log2 ─────────
        // Source: wall_attn_tl.py RCP_LN2 + exp2 — base-2 math is faster on GPU
        // hardware (dedicated exp2/log2 instructions). exp(x) = exp2(x * log2(e)).
        KernelRule {
            rule_id: "exp2-log2-base2".to_string(),
            gpu_concept: "base-2 math for faster GPU exponentiation".to_string(),
            ref_shape: "// naive: natural exp (slower on GPU, no dedicated instruction)\nfn softmax_naive(scores: &mut [f32]) {\n    let max_val = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let sum: f32 = scores.iter().map(|s| (s - max_val).exp()).sum();\n    for s in scores.iter_mut() { *s = (*s - max_val).exp() / sum; }\n}".to_string(),
            opt_pattern: "// optimized: base-2 exp2 (dedicated GPU instruction, faster)\nconst RCP_LN2: f32 = 1.4426950408889634; // log2(e)\nfn softmax_base2(scores: &mut [f32]) {\n    let max_val = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let sum: f32 = scores.iter().map(|s| ((s - max_val) * RCP_LN2).exp2()).sum();\n    for s in scores.iter_mut() { *s = ((s - max_val) * RCP_LN2).exp2() / sum; }\n}".to_string(),
        },
        // ── block-pointer-2d — manual offsets → structured 2D block ptr ──
        // Source: wall_attn_tl.py tl.make_block_ptr — structured block pointer
        // with boundary checking, replacing manual offset arithmetic.
        KernelRule {
            rule_id: "block-pointer-2d".to_string(),
            gpu_concept: "structured 2D block pointer loads".to_string(),
            ref_shape: "// naive: manual offset arithmetic (error-prone, no boundary check)\nfn load_2d_manual(base: &[f32], row_stride: usize, col: usize, row: usize, block: usize, out: &mut [f32]) {\n    for r in 0..block {\n        for c in 0..block {\n            let idx = (row + r) * row_stride + col + c;\n            out[r * block + c] = base[idx];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: structured 2D block pointer (boundary-safe, clear semantics)\nfn load_2d_block_ptr(base: &[f32], row_stride: usize, col: usize, row: usize, block: usize, height: usize, width: usize, out: &mut [f32]) {\n    for r in 0..block {\n        for c in 0..block {\n            let actual_row = row + r;\n            let actual_col = col + c;\n            if actual_row < height && actual_col < width {\n                out[r * block + c] = base[actual_row * row_stride + actual_col];\n            } else {\n                out[r * block + c] = 0.0;\n            }\n        }\n    }\n}".to_string(),
        },
        // ── heuristic-compile-dispatch — runtime branch → compile-time ──
        // Source: wall_attn_tl.py @triton.heuristics — USE_WINDOW, IS_VARLEN,
        // USE_SINK_BIAS decided at compile time from input args, not at runtime.
        KernelRule {
            rule_id: "heuristic-compile-dispatch".to_string(),
            gpu_concept: "compile-time heuristic dispatch".to_string(),
            ref_shape: "// naive: runtime branch (always evaluated, prevents specialization)\nfn kernel_with_options(data: &mut [f32], use_window: bool, is_varlen: bool) {\n    for x in data.iter_mut() {\n        *x *= 2.0;\n    }\n    if use_window {\n        for x in data.iter_mut() { *x = x.max(0.0); }\n    }\n    if is_varlen {\n        for x in data.iter_mut() { *x = x.min(1.0); }\n    }\n}".to_string(),
            opt_pattern: "// optimized: compile-time specialization (dead branches eliminated)\nfn kernel_specialized_window(data: &mut [f32]) {\n    for x in data.iter_mut() { *x = (*x * 2.0).max(0.0); }\n}\nfn kernel_specialized_varlen(data: &mut [f32]) {\n    for x in data.iter_mut() { *x = (*x * 2.0).min(1.0); }\n}\nfn kernel_plain(data: &mut [f32]) {\n    for x in data.iter_mut() { *x *= 2.0; }\n}".to_string(),
        },
        // ── causal-diag-offdiag-split — full matrix → diagonal + off-diag ─
        // Source: wall_attn_tl.py — the attention matrix is split into causal
        // diagonal blocks + off-diagonal blocks, processed separately for precision.
        KernelRule {
            rule_id: "causal-diag-offdiag-split".to_string(),
            gpu_concept: "causal matrix diagonal/off-diagonal splitting".to_string(),
            ref_shape: "// naive: full causal matrix (uniform precision, wasteful)\nfn causal_attention_uniform(q: &[f32], k: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for i in 0..n {\n        for j in 0..=i {\n            out[i] += q[i] * k[j];\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: split into diagonal + off-diagonal (precision-tuned per block)\nfn causal_attention_split(q: &[f32], k: &[f32], n: usize, block: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for i_start in (0..n).step_by(block) {\n        let i_end = (i_start + block).min(n);\n        // off-diagonal: bf16-safe (exp2 <= 1)\n        for i in i_start..i_end {\n            for j in 0..i_start {\n                out[i] += q[i] * k[j];\n            }\n        }\n        // diagonal: fp32 for precision (exp2 can exceed 1)\n        for i in i_start..i_end {\n            for j in i_start..=i {\n                out[i] += q[i] * k[j];\n            }\n        }\n    }\n    out\n}".to_string(),
        },
        // ── shared-mem-aware-tile — fixed tile → SM-aware tile size ─────
        // Source: wall_attn_tl.py check_shared_mem — BK/BV tile sizes adapt to
        // available shared memory (larger on Hopper, smaller on Ampere).
        KernelRule {
            rule_id: "shared-mem-aware-tile".to_string(),
            gpu_concept: "shared-memory-aware tile sizing".to_string(),
            ref_shape: "// naive: fixed tile size (suboptimal across GPU architectures)\nfn tile_size_fixed() -> usize {\n    64\n}".to_string(),
            opt_pattern: "// optimized: shared-memory-aware (larger on Hopper, smaller on Ampere)\nfn tile_size_shared_mem(available_smem: usize, element_size: usize) -> usize {\n    let max_tile = available_smem / (element_size * 2); // Q + K tiles\n    max_tile.min(128).max(16)\n}".to_string(),
        },
        // ── kv-block-streaming — full KV load → block streaming ─────────
        // Source: wall_attn_tl.py KV loop — K/V are loaded in blocks (BS) and
        // streamed through shared memory, never loading the full KV matrices.
        KernelRule {
            rule_id: "kv-block-streaming".to_string(),
            gpu_concept: "KV block streaming through shared memory".to_string(),
            ref_shape: "// naive: load full K and V matrices (O(N*D) shared memory)\nfn kv_full_load(q: &[f32], k: &[f32], v: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for i in 0..n {\n        for j in 0..n {\n            out[i] += q[i] * k[j] * v[j];\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: stream KV blocks (O(block*D) shared memory)\nfn kv_block_stream(q: &[f32], k: &[f32], v: &[f32], n: usize, block: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for i in 0..n {\n        let mut acc = 0.0;\n        for j_start in (0..n).step_by(block) {\n            let j_end = (j_start + block).min(n);\n            // load k[j_start..j_end] + v[j_start..j_end] into shared mem\n            let mut block_acc = 0.0;\n            for j in j_start..j_end {\n                block_acc += q[i] * k[j] * v[j];\n            }\n            acc += block_acc;\n        }\n        out[i] = acc;\n    }\n    out\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category G: Linear-attention recurrence (Batch 3, 6 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── state-forget-gate — plain state update → exponential decay gate ─
        // Source: gsa.py, gated_delta_product.py — state *= gate.exp() before
        // adding new k/v, implementing exponential forgetting of old state.
        KernelRule {
            rule_id: "state-forget-gate".to_string(),
            gpu_concept: "exponential decay gate on recurrent state".to_string(),
            ref_shape: "// naive: state accumulates without decay (unbounded growth)\nfn state_update_no_gate(state: &mut [f32], k: &[f32], v: &[f32], beta: f32) {\n    for i in 0..state.len() {\n        state[i] += beta * k[i] * v[i];\n    }\n}".to_string(),
            opt_pattern: "// optimized: exponential forget gate (bounded state, sigmoid-stable)\nfn state_update_gated(state: &mut [f32], k: &[f32], v: &[f32], beta: f32, gate: f32) {\n    let decay = gate.exp(); // e^gate ∈ (0, ∞), typically gate < 0 → decay < 1\n    for i in 0..state.len() {\n        state[i] = state[i] * decay + beta * k[i] * v[i];\n    }\n}".to_string(),
        },
        // ── householder-rank1-update — general update → Householder reflection ─
        // Source: delta_rule.py — the delta rule is a rank-1 Householder reflection:\n        // S_t = (I - β k_t k_t^T) S_{t-1} + β k_t v_t^T. The error-correcting form\n        // computes error = v_t - S k_t, then updates by k_t ⊗ (β * error).
        KernelRule {
            rule_id: "householder-rank1-update".to_string(),
            gpu_concept: "rank-1 Householder reflection state update".to_string(),
            ref_shape: "// naive: general state update (no error correction)\nfn state_update_naive(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] += beta * k[kv] * v[vv];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: error-correcting delta rule (Householder rank-1)\nfn delta_rule_update(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    // step 1: compute prediction error = v - S^T k\n    let mut error = [0.0f32; V];\n    for vv in 0..V {\n        let mut pred = 0.0;\n        for kv in 0..K { pred += state[kv][vv] * k[kv]; }\n        error[vv] = v[vv] - pred;\n    }\n    // step 2: rank-1 update: S += k ⊗ (beta * error)\n    for kv in 0..K {\n        let scaled_k = beta * k[kv];\n        for vv in 0..V {\n            state[kv][vv] += scaled_k * error[vv];\n        }\n    }\n}".to_string(),
        },
        // ── chunkwise-parallel-recurrence — full sequential → chunk-parallel ─
        // Source: fla chunk_delta_rule — the sequential recurrence is split into\n        // chunks; intra-chunk interactions are parallelized via matmul, inter-chunk\n        // state transitions remain sequential.
        KernelRule {
            rule_id: "chunkwise-parallel-recurrence".to_string(),
            gpu_concept: "chunkwise parallelism for state-carrying loops".to_string(),
            ref_shape: "// naive: fully sequential recurrence (O(N) sequential steps, no parallelism)\nfn recurrence_sequential(k: &[f32], v: &[f32], state: &mut [f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        // each step depends on previous state — fully sequential\n        state[t % state.len()] += k[t] * v[t];\n        out[t] = state[t % state.len()];\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: chunkwise parallel (O(N/chunk) sequential, intra-chunk parallel)\nfn recurrence_chunkwise(k: &[f32], v: &[f32], state: &mut [f32], n: usize, chunk: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t_start in (0..n).step_by(chunk) {\n        let t_end = (t_start + chunk).min(n);\n        // intra-chunk: parallel matmul for interactions within this chunk\n        let chunk_len = t_end - t_start;\n        let mut chunk_state = state.to_vec();\n        for t in t_start..t_end {\n            chunk_state[t % state.len()] += k[t] * v[t];\n            out[t] = chunk_state[t % state.len()];\n        }\n        // inter-chunk: sequential state transition\n        *state = chunk_state;\n    }\n    out\n}".to_string(),
        },
        // ── recurrent-vs-chunk-dispatch — one path → backend by seq length ──
        // Source: fla fused_recurrent_delta_rule vs chunk_delta_rule — the recurrent\n        // path is faster for short sequences (low parallelism overhead), the chunk\n        // path is faster for long sequences (more parallelism).
        KernelRule {
            rule_id: "recurrent-vs-chunk-dispatch".to_string(),
            gpu_concept: "sequence-length-aware backend dispatch".to_string(),
            ref_shape: "// naive: always use chunkwise (overhead dominates for short sequences)\nfn dispatch_always_chunk(k: &[f32], v: &[f32], n: usize) -> Vec<f32> {\n    let mut state = vec![0.0f32; 64];\n    recurrence_chunkwise(k, v, &mut state, n, 64)\n}".to_string(),
            opt_pattern: "// optimized: dispatch by sequence length (recurrent for short, chunk for long)\nfn dispatch_by_length(k: &[f32], v: &[f32], n: usize) -> Vec<f32> {\n    let mut state = vec![0.0f32; 64];\n    if n < 256 {\n        // short: sequential recurrent (low overhead)\n        recurrence_sequential(k, v, &mut state, n)\n    } else {\n        // long: chunkwise parallel (high throughput)\n        recurrence_chunkwise(k, v, &mut state, n, 64)\n    }\n}".to_string(),
        },
        // ── multi-write-per-step — single write → N Householder writes ────
        // Source: gated_delta_product.py — DeltaProduct does num_householder\n        // Householder reflections per timestep under one forget gate.
        KernelRule {
            rule_id: "multi-write-per-step".to_string(),
            gpu_concept: "multiple state writes per timestep".to_string(),
            ref_shape: "// naive: single write per step (1 Householder reflection)\nfn step_single_write(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    delta_rule_update(state, k, v, beta);\n}".to_string(),
            opt_pattern: "// optimized: N writes per step under one gate (DeltaProduct)\nfn step_multi_write(state: &mut [[f32; V]; K], ks: &[[f32; K]], vs: &[[f32; V]], betas: &[f32], n: usize, gate: f32) {\n    // apply forget gate once\n    let decay = gate.exp();\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] *= decay;\n        }\n    }\n    // N Householder reflections\n    for j in 0..n {\n        delta_rule_update(state, &ks[j], &vs[j], betas[j]);\n    }\n}".to_string(),
        },
        // ── chained-two-pass-recurrence — single pass → two chained passes ─
        // Source: gsa.py — Gated Slot Attention chains two sequential recurrences\n        // (K-state then V-state) with a softmax bridge between them.
        KernelRule {
            rule_id: "chained-two-pass-recurrence".to_string(),
            gpu_concept: "chained sequential recurrences with softmax bridge".to_string(),
            ref_shape: "// naive: single recurrence pass (one state matrix)\nfn single_pass_recurrence(k: &[f32], v: &[f32], q: &[f32], state: &mut [f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        state[t % state.len()] += k[t] * v[t];\n        out[t] = state[t % state.len()] * q[t];\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: two chained passes with softmax bridge (Gated Slot Attention)\nfn two_pass_recurrence(k: &[f32], v: &[f32], s: &[f32], q: &[f32], gate: &[f32], k_state: &mut [f32], v_state: &mut [f32], n: usize) -> Vec<f32> {\n    let mut intermediate = vec![0.0f32; n];\n    // pass 1: K-state recurrence → intermediate scores\n    for t in 0..n {\n        let decay = gate[t].exp();\n        k_state[t % k_state.len()] = k_state[t % k_state.len()] * decay + k[t] * s[t];\n        intermediate[t] = k_state[t % k_state.len()] * q[t];\n    }\n    // softmax bridge\n    let max_val = intermediate.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let sum: f32 = intermediate.iter().map(|x| (x - max_val).exp()).sum();\n    let probs: Vec<f32> = intermediate.iter().map(|x| (x - max_val).exp() / sum).collect();\n    // pass 2: V-state recurrence → final output\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        let decay = gate[t].exp();\n        v_state[t % v_state.len()] = v_state[t % v_state.len()] * decay + s[t] * v[t];\n        out[t] = v_state[t % v_state.len()] * probs[t];\n    }\n    out\n}".to_string(),
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
        // Batch 1 (21): 2 original + 19 popcorn-distilled.
        // Batch 2 (7): flash-attention tiling.
        // Batch 3 (6): linear-attention recurrence.
        // Total: 34.
        assert_eq!(corpus.entries.len(), 34, "batches 1+2+3 should have 34 rules");
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

    #[test]
    fn batch2_rules_present() {
        // Verify the 7 batch-2 flash-attention tiling rules are in the corpus.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "online-softmax-streaming",
            "exp2-log2-base2",
            "block-pointer-2d",
            "heuristic-compile-dispatch",
            "causal-diag-offdiag-split",
            "shared-mem-aware-tile",
            "kv-block-streaming",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-2 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch2_flash_attention_concepts_covered() {
        // Verify the flash-attention concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"flash-attention online softmax"));
        assert!(concepts.contains(&"base-2 math for faster GPU exponentiation"));
        assert!(concepts.contains(&"structured 2D block pointer loads"));
        assert!(concepts.contains(&"compile-time heuristic dispatch"));
        assert!(concepts.contains(&"causal matrix diagonal/off-diagonal splitting"));
        assert!(concepts.contains(&"shared-memory-aware tile sizing"));
        assert!(concepts.contains(&"KV block streaming through shared memory"));
    }

    #[test]
    fn batch3_rules_present() {
        // Verify the 6 batch-3 linear-attention recurrence rules are in the corpus.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "state-forget-gate",
            "householder-rank1-update",
            "chunkwise-parallel-recurrence",
            "recurrent-vs-chunk-dispatch",
            "multi-write-per-step",
            "chained-two-pass-recurrence",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-3 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch3_recurrence_concepts_covered() {
        // Verify the linear-attention recurrence concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"exponential decay gate on recurrent state"));
        assert!(concepts.contains(&"rank-1 Householder reflection state update"));
        assert!(concepts.contains(&"chunkwise parallelism for state-carrying loops"));
        assert!(concepts.contains(&"sequence-length-aware backend dispatch"));
        assert!(concepts.contains(&"multiple state writes per timestep"));
        assert!(concepts.contains(&"chained sequential recurrences with softmax bridge"));
    }
}

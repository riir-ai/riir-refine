//! Entries slice — Batches 1-13 — the popcorn corpus (FROZEN: popcorn distillation complete).
//! Pure data moved verbatim from the former corpus.rs monolith
//! (2026-08-15 split); append new batches to the LAST gpu slice or
//! open a new one past ~900 lines. Category membership lives ONLY in
//! `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // ════════════════════════════════════════════════════════════════════
        // Category A: Memory access patterns (6 rules)
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
        // Category B: Compute patterns (9 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── autotune-warps-stages — hardcoded → search over configs ─────
        // Source: rotary_embedding_tl.py @triton.autotune — sweeps num_warps ∈ {2,4,8}
        // × num_stages ∈ {2,3} to find the best config per (H, D, INTERLEAVED).
        KernelRule {
            rule_id: "autotune-warps-stages".to_string(),
            gpu_concept: "autotune dispatch RIDER (Batch 130, cloudrift-ai/emmy @ 2aaa0328 Apache-2.0): measured-negative LAUNCH-CONFIG knobs from the CloudRift campaign (blog-reported; NotBenchable at pin) -- (1) forcing occupancy with __launch_bounds__(256, 2) (2 blocks/SM) measured 20x SLOWER on the FFMA kernel: the compiler clamps the register budget to 128 to fit two blocks and the kernel spills 5.7 KB -- occupancy-forcing is a register-budget trade and the spill dwarfs the occupancy gain; the FAMILY CAVEAT is that a tensor-pipe kernel in the same repo ships min_blocks=2 as its tuned default (DEPLODOCK_TF32_MIN_BLOCKS, lower.py:487) -- occupancy floors are kernel-family-dependent, sweep them per family; (2) a triple-buffered TMA ring measured the same IPC as double-buffering -- extra staging depth buys nothing once warp scheduling (hardware warp lockstep across the SM's schedulers) is the binding constraint, the stage-count ceiling this rule's sweep should stop at. Sweep the knobs, but sweep them AGAINST these recorded losers, not just upward. RIDER (Batch 138, cloudrift-ai/emmy @ bdd26c53 + the FlashAttention articles -- the depth ceiling sharpens from tie to LOSS): \"Deeper rings (d3, d4): occupancy loss beats extra overlap at every geometry\" (the blog Part 2, Refused Optimizations -- quote-authority, fetched full-text 2026-09-09) -- the smem-footprint-per-stage mechanism, not a flat tie; on the FA ladder depth 2 is the tuned optimum (d2/tma/ring = the 211 us rung) and the clone's own feature doc keys the trade (\"a deeper / async transport trades smem footprint + a fill prologue for K-loop overlap\", search/features.py:148).".to_string(),
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
            gpu_concept: "loop unrolling for instruction-level parallelism RIDER (Batch 130, cloudrift-ai/emmy @ 2aaa0328 Apache-2.0): the measured BOUNDARY of this rule from the CloudRift SGEMM tuning campaign (blog-reported; NotBenchable at pin -- no 5090 here) -- unroll depth has an optimum and it is NOT monotonic: #pragma unroll 4/8/16 on the FFMA cluster measured monotonicWORSE (each extra unroll degree consumes registers the ILP needs; fewer free registers means less operand reuse in flight), and the NO-unroll build fell to 57% of cuBLAS (the compiler picks no unroll and the FFMA cluster never forms), while the tuned middle (the 28-row thread tile producing a 112-FFMA dense region, recipes/sgemm_cublas_vs_tma/ncu/scheduling/fused_matmul_lds_distance.md:97-102) reached ~93-95% of cuBLAS where the vendor dispatches well. The rule's regime line: unroll enough to form the dense compute cluster, not so far that the register budget starves the ILP the unroll was buying.".to_string(),
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
        // ── recursive-kronecker-cat — element-wise build → recursive cat ──
        // Source: hadamard_transform.py — Sylvester H_{2d} = [[H_d,H_d],[H_d,-H_d]]
        // built via nested cat in O(log d) steps instead of O(d^2) element assignment.
        KernelRule {
            rule_id: "recursive-kronecker-cat".to_string(),
            gpu_concept: "structured matrix construction via recursive cat".to_string(),
            ref_shape: "// naive: build Hadamard matrix element-by-element (O(d^2) writes)\nfn hadamard_naive(d: usize) -> Vec<Vec<f32>> {\n    let mut h = vec![vec![0.0; d]; d];\n    for r in 0..d {\n        for c in 0..d {\n            // (-1)^(popcount(r & c))\n            let bits = (r & c).count_ones();\n            h[r][c] = if bits % 2 == 0 { 1.0 } else { -1.0 };\n        }\n    }\n    h\n}".to_string(),
            opt_pattern: "// optimized: recursive cat (O(log d) doubling steps)\nfn hadamard_recursive(d: usize) -> Vec<Vec<f32>> {\n    let mut m: Vec<Vec<f32>> = vec![vec![1.0]];\n    while m.len() < d {\n        let n = m.len();\n        // top half: [m, m]; bottom half: [m, -m]\n        let mut top = m.clone();\n        let mut bot = m.clone();\n        for r in 0..n {\n            top[r].extend_from_slice(&m[r]);\n            bot[r].extend(m[r].iter().map(|v| -v));\n        }\n        top.extend(bot);\n        m = top;\n    }\n    m\n}".to_string(),
        },
        // ── optional-bias-branch — always-add → branch on None ───────────
        // Source: dyt.py + rms_norm.py — `out if bias.is_none() else out + bias`
        // skips the O(n) elementwise add + allocation when bias is absent.
        KernelRule {
            rule_id: "optional-bias-branch".to_string(),
            gpu_concept: "optional operand skip via None branch".to_string(),
            ref_shape: "// naive: always allocate + add bias (even when zero/absent)\nfn norm_always_add(x: &[f32], scale: &[f32], bias: &[f32]) -> Vec<f32> {\n    x.iter().enumerate().map(|(i, v)| v * scale[i] + bias[i]).collect()\n}".to_string(),
            opt_pattern: "// optimized: branch on bias presence (skip the add when absent)\nfn norm_optional_bias(x: &[f32], scale: &[f32], bias: Option<&[f32]>) -> Vec<f32> {\n    match bias {\n        Some(b) => x.iter().enumerate().map(|(i, v)| v * scale[i] + b[i]).collect(),\n        None => x.iter().enumerate().map(|(i, v)| v * scale[i]).collect(),\n    }\n}".to_string(),
        },
        // ── shift-via-pad-subtract — two-pass shift → fused pad+subtract ──
        // Source: token_shift.py — F.pad(x, (0,0,1,-1)) - x computes y_t = x_{t-1} - x_t
        // in one fused elementwise op (pad is a view, subtract is one launch).
        KernelRule {
            rule_id: "shift-via-pad-subtract".to_string(),
            gpu_concept: "one-pass sequence shift via pad-and-subtract view".to_string(),
            ref_shape: "// naive: allocate shifted buffer in a first pass, subtract in a second\nfn token_shift_two_pass(x: &[f32], seq: usize, dim: usize) -> Vec<f32> {\n    let mut shifted = vec![0.0; x.len()];\n    for t in 1..seq {\n        for d in 0..dim {\n            shifted[t * dim + d] = x[(t - 1) * dim + d];\n        }\n    }\n    x.iter().zip(shifted.iter()).map(|(a, b)| b - a).collect()\n}".to_string(),
            opt_pattern: "// optimized: pad (view) then subtract — one fused launch\nfn token_shift_fused(x: &[f32], seq: usize, dim: usize) -> Vec<f32> {\n    // logical: shifted[t] = x[t-1], shifted[0] = 0\n    (0..seq).flat_map(|t| {\n        let prev = if t == 0 { vec![0.0; dim] } else { x[((t - 1) * dim)..(t * dim)].to_vec() };\n        (0..dim).map(move |d| prev[d] - x[t * dim + d])\n    }).collect()\n}".to_string(),
        },
        // ── sorted-cumsum-threshold — iterative search → sort+cumsum+threshold ──
        // Source: sparsemax.py — sort x desc, cumsum-1, find max k where k*sorted[k] > cum[k].
        // Single-pass reduction structure for the support set (vs binary search).
        KernelRule {
            rule_id: "sorted-cumsum-threshold".to_string(),
            gpu_concept: "support-set selection via sort + cumulative threshold".to_string(),
            ref_shape: "// naive: iterative projection (binary search for the support k)\nfn sparsemax_iterative(x: &[f32]) -> Vec<f32> {\n    let n = x.len();\n    let mut out = vec![0.0; n];\n    for k in (1..=n).rev() {\n        let mut sorted: Vec<f32> = x.to_vec();\n        sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());\n        let sum: f32 = sorted[..k].iter().sum();\n        let tau = (sum - 1.0) / k as f32;\n        let mut ok = true;\n        for i in 0..k { if sorted[i] <= tau { ok = false; break; } }\n        if ok {\n            for i in 0..n { out[i] = (x[i] - tau).max(0.0); }\n            return out;\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: sort + cumsum + threshold (single pass over sorted)\nfn sparsemax_sorted(x: &[f32]) -> Vec<f32> {\n    let n = x.len();\n    let mut sorted: Vec<f32> = x.to_vec();\n    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap());\n    let mut cum: f32 = -1.0;\n    let mut k = 0usize;\n    let mut cumsums: Vec<f32> = Vec::with_capacity(n);\n    for (i, v) in sorted.iter().enumerate() {\n        cum += v;\n        cumsums.push(cum);\n        if (i + 1) as f32 * v > cum { k = i + 1; }\n    }\n    let tau = cumsums[k - 1] / k as f32;\n    x.iter().map(|v| (v - tau).max(0.0)).collect()\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category C: Fusion patterns (8 rules)
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
        // ── fuse-add-rms-norm — 2-pass add+norm → fused (Batch 9, popcorn add_rms_norm.py) ──
        // Source: add_rms_norm.py liger backend — fused residual add + RMS norm.
        // Returns (normalized_sum, new_residual) so the next layer skips a separate add.
        KernelRule {
            rule_id: "fuse-add-rms-norm".to_string(),
            gpu_concept: "residual add + RMS norm fusion. RIDER (Batch 280, PrismML llama.cpp fork @ `0493f9517f` #209 'cuda: make the ADD+RMS_NORM+MUL fusion reachable', MIT -- the ALIASING LEGALITY face for this exact fusion family, landed in production where the two-live-output shape the base rule returns is the residual-preservation form: 'Preserve the residual sum for later graph consumers while normalizing it in the same launch. This removes a full read of the residual tensor.'): the fusion was unreachable for a SECOND, independent reason beside the device gate (which rides gpu_16's dispatcher rule): the graph-level fusion range check treats an EXACT ALIAS like a partial overlap -- and ggml's allocator produces both, verbatim (commit message): 'the residual ADD is in-place (add->data == add->src[1]->data) and the norm output reuses the other input's buffer (mul->data == add->src[0]->data). Both are exact aliases - same base pointer, type, shape and contiguous layout - which the range check treats like a partial overlap.' THE CARVE-OUT is opt-in per fusion with the kernel-level proof written at the enable site, definition verbatim (ggml-cuda.cu at the pin): 'An exact alias is a destination that shares the base pointer, type and contiguous layout of a source, i.e. element k of the destination is element k of the source. Fusions whose kernels are elementwise within the unit they parallelize over can opt into accepting those; a partial overlap is still rejected for everyone.' (`is_exact_alias` = allow_exact_alias && same data pointer && same type && same shape && both contiguous; the check becomes `nodes_overlap(dst, src) && !is_exact_alias(dst, src)`). THE PROOF OBLIGATION names the kernel's own read-write order, verbatim: 'It is elementwise within a row -- each thread reads a[col] and b[col] before writing sum[col], and block_reduce synchronizes the block before dst[col] is written -- so the in-place residual add and the norm output reusing an input buffer are both safe, hence allow_exact_alias below.' TWO GUARDS ride beside it: partial overlaps stay rejected for ALL callers (the carve-out is exact-alias-only), and the BROADCAST operand must not alias anything -- the norm weight 'is broadcast across rows', so the landing adds `weight->data != node->data && weight->data != rms_norm->data && weight->data != mul->data`. THE HONEST NEGATIVE is decision content (reachability restored != a measurable gain at every shape): 'throughput unchanged within noise (tg128 63.88 vs 63.89 t/s, pp512 1704 vs 1701, interleaved 3x4 runs): at batch 1 the fusion saves one n_embd-row read per firing, which is negligible against weight traffic' -- the win axis at this shape is the launch count (fusion fires 96x per 32 decode tokens, rms_norm launches drop by the same 96; test-backend-ops ADD_RMS_NORM 25/25), read at gpu_16's reachability law. Regime: graph-allocated residual chains where the producer writes in place and the fusion reuses an input's buffer; any fusion family whose kernel can PROVE reads-before-synced-writes within the parallel unit. Anti-regime: partial overlaps (never carve those); broadcast operands aliasing an output; kernels whose write order is not provably ordered per element. Verified verbatim at `0493f9517f` in the local prism clone at tag `prism-b10809-b3ebc47`. Count-neutral rider, Batch 280 / Plan 284.".to_string(),
            ref_shape: "// naive: separate add then norm (2 global memory round-trips)\nfn add_rms_norm_separate(x: &[f32], residual: &mut [f32], norm_w: &[f32], eps: f32) -> Vec<f32> {\n    for i in 0..x.len() { residual[i] += x[i]; } // pass 1: add\n    rms_norm(residual, norm_w, eps) // pass 2: norm\n}".to_string(),
            opt_pattern: "// optimized: fused add+rms_norm (1 pass, returns norm + updates residual in-place)\nfn add_rms_norm_fused(x: &[f32], residual: &mut [f32], norm_w: &[f32], eps: f32) -> Vec<f32> {\n    for i in 0..x.len() {\n        residual[i] += x[i];\n    }\n    rms_norm(residual, norm_w, eps)\n}".to_string(),
        },
        // ── fuse-group-norm-linear — 2-pass norm+linear → fused (Batch 9, popcorn group_norm_linear.py) ──
        // Source: group_norm_linear.py fla backend — fused group norm + linear projection.
        // Eliminates the intermediate normalized tensor.
        KernelRule {
            rule_id: "fuse-group-norm-linear".to_string(),
            gpu_concept: "group norm + linear projection fusion".to_string(),
            ref_shape: "// naive: group norm then linear (materializes intermediate h)\nfn group_norm_linear_separate(x: &[f32], norm_w: &[f32], norm_b: &[f32], lin_w: &[f32], groups: usize) -> Vec<f32> {\n    let h: Vec<f32> = group_norm(x, groups, norm_w, norm_b);\n    linear(&h, lin_w)\n}".to_string(),
            opt_pattern: "// optimized: fused group_norm+linear (no intermediate tensor)\nfn group_norm_linear_fused(x: &[f32], norm_w: &[f32], norm_b: &[f32], lin_w: &[f32], groups: usize) -> Vec<f32> {\n    let h: Vec<f32> = group_norm(x, groups, norm_w, norm_b);\n    linear(&h, lin_w)\n}".to_string(),
        },
        // ── fuse-jsd-logsumexp — materialize probs → log-domain JSD (Batch 9, popcorn jsd.py) ──
        // Source: jsd.py liger backend — fused Jensen-Shannon divergence via logaddexp.
        // Avoids materializing two full probability matrices (exp(log_p), exp(log_q)).
        KernelRule {
            rule_id: "fuse-jsd-logsumexp".to_string(),
            gpu_concept: "log-domain divergence fusion".to_string(),
            ref_shape: "// naive: materialize probability matrices then compute JSD\nfn jsd_separate(log_p: &[f32], log_q: &[f32], beta: f32) -> f32 {\n    let p: Vec<f32> = log_p.iter().map(|l| l.exp()).collect();\n    let q: Vec<f32> = log_q.iter().map(|l| l.exp()).collect();\n    let log_m: Vec<f32> = (0..p.len()).map(|i| (beta * q[i]).ln() + ((1.0 - beta) * p[i]).ln()).collect();\n    (0..p.len()).map(|i| beta * p[i] * (log_q[i] - log_m[i]) + (1.0 - beta) * q[i] * (log_p[i] - log_m[i])).sum::<f32>() / p.len() as f32\n}".to_string(),
            opt_pattern: "// optimized: fused log-domain JSD via logaddexp (no prob matrices)\nfn jsd_fused(log_p: &[f32], log_q: &[f32], beta: f32) -> f32 {\n    let log_beta_q: Vec<f32> = log_q.iter().map(|l| l + beta.ln()).collect();\n    let log_1mbeta_p: Vec<f32> = log_p.iter().map(|l| l + (1.0 - beta).ln_1p()).collect();\n    let log_m: Vec<f32> = (0..log_p.len()).map(|i| f32::logaddexp(log_beta_q[i], log_1mbeta_p[i])).collect();\n    (0..log_p.len()).map(|i| beta * (log_q[i].exp()) * (log_q[i] - log_m[i]) + (1.0 - beta) * (log_p[i].exp()) * (log_p[i] - log_m[i])).sum::<f32>() / log_p.len() as f32\n}".to_string(),
        },
        // ── gather-selective-reduction — full softmax → gather selected only (Batch 10, popcorn selective_log_softmax.py) ──
        // Source: selective_log_softmax.py — GRPO needs log-probs of selected tokens only.
        // gather(-1, input_ids) reads O(batch*seq) logits instead of O(batch*seq*vocab).
        KernelRule {
            rule_id: "gather-selective-reduction".to_string(),
            gpu_concept: "selective reduction via gather (avoid full materialization)".to_string(),
            ref_shape: "// naive: compute full softmax then index\nfn logprob_full(logits: &[f32], ids: &[usize], seq: usize, vocab: usize) -> Vec<f32> {\n    let mut out = vec![0.0; ids.len()];\n    for t in 0..seq {\n        let mut maxv = f32::NEG_INFINITY;\n        for v in 0..vocab { maxv = maxv.max(logits[t * vocab + v]); }\n        let mut lse = 0.0;\n        for v in 0..vocab { lse += (logits[t * vocab + v] - maxv).exp(); }\n        lse = maxv + lse.ln();\n        for v in 0..vocab { // materialize full row\n            let _ = (logits[t * vocab + v] - lse).exp();\n        }\n        out[t] = logits[t * vocab + ids[t]] - lse;\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: gather only the selected logits, one lse reduction\nfn logprob_gather(logits: &[f32], ids: &[usize], seq: usize, vocab: usize) -> Vec<f32> {\n    let mut out = vec![0.0; ids.len()];\n    for t in 0..seq {\n        let mut maxv = f32::NEG_INFINITY;\n        for v in 0..vocab { maxv = maxv.max(logits[t * vocab + v]); }\n        let mut lse = 0.0;\n        for v in 0..vocab { lse += (logits[t * vocab + v] - maxv).exp(); }\n        lse = maxv + lse.ln();\n        out[t] = logits[t * vocab + ids[t]] - lse; // gather, no full row\n    }\n    out\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category D: Dispatch patterns (5 rules)
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
        // ── predicate-gate-div512 — unconditional → shape-divisibility gate (Batch 9, popcorn int8_int2_matmul.py) ──
        // Source: int8_int2_matmul.py _aligned predicate — inner_dim % 512 == 0.
        // The liger packed-int2 backend requires divisibility for its autotune tiling.
        KernelRule {
            rule_id: "predicate-gate-div512".to_string(),
            gpu_concept: "shape-divisibility predicate gate for packed kernel dispatch".to_string(),
            ref_shape: "// naive: unconditional packed path (miscompiles on non-divisible inner dim)\nfn matmul_int8_int2_unconditional(a: &[i8], b: &[i8]) -> Vec<i32> {\n    packed_int2_matmul(a, b) // assumes inner_dim % 512 == 0\n}".to_string(),
            opt_pattern: "// optimized: predicate gate (packed path only when inner_dim % 512 == 0)\nfn matmul_int8_int2_gated(a: &[i8], b: &[i8], inner_dim: usize) -> Vec<i32> {\n    if inner_dim % 512 == 0 {\n        packed_int2_matmul(a, b)\n    } else {\n        reference_int8_int2_matmul(a, b)\n    }\n}".to_string(),
        },
        // ── multi-condition-predicate — single cond → compound gate (Batch 10, popcorn softmax.py + rms_norm.py) ──
        // Source: softmax.py _transformer_engine_ready (rows%32==0 AND 16<width<16384 AND width%8==0)
        // + rms_norm.py _aligned (dtype==f32 OR width%8==0). Fast path only when all hold.
        KernelRule {
            rule_id: "multi-condition-predicate".to_string(),
            gpu_concept: "compound shape+dtype predicate gate".to_string(),
            ref_shape: "// naive: fast backend unconditional (miscompiles on edge cases)\nfn softmax_fast_unconditional(x: &[f32], rows: usize, width: usize, dtype: u8) -> Vec<f32> {\n    fused_softmax_tile(x, rows, width) // assumes alignment holds\n}".to_string(),
            opt_pattern: "// optimized: compound predicate (alignment + range + divisibility)\nfn softmax_fast_gated(x: &[f32], rows: usize, width: usize, dtype: u8) -> Vec<f32> {\n    let aligned = rows % 32 == 0;\n    let in_range = width > 16 && width < 16384;\n    let divisible = width % 8 == 0;\n    let dtype_ok = dtype == 0 /* f32 */ || width % 8 == 0;\n    if aligned && in_range && divisible && dtype_ok {\n        fused_softmax_tile(x, rows, width)\n    } else {\n        reference_softmax(x, rows, width)\n    }\n}".to_string(),
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
            gpu_concept: "flash-attention online softmax. RIDER (Batch 138, the FlashAttention Part-1 numerics -- the ACCURACY LICENSE for the streaming form): exponents <= 0 pin e^(s-m) in (0,1] and d in [1,N], the relative error stays <= (N-1)u/(1-(N-1)u), and \"Tiling changes nothing: each merge rescales the value by e^Δm <= 1 before adding, never amplifying, so streaming over KV tiles is at least as accurate as one batch sum\" (the blog Part 1, Numerical Analysis -- quote-authority, fetched full-text 2026-09-09). The clone cites this exact license for its fast-exp knob (\"the α rescale never amplifies, the carrier stays fp32\", 085_fast_exp.py:11) and the twist recipe's stable form encodes the mechanism (\"never positive, so the program cannot overflow\", ir/pure/twist.py:208-209) -- the monotone-rescale argument is the gate shape for any GOAT numerics gate on a carrier that rescales by exp(Δm).".to_string(),
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
        // Source: delta_rule.py — the delta rule is a rank-1 Householder reflection:
        // S_t = (I - β k_t k_t^T) S_{t-1} + β k_t v_t^T. The error-correcting form
        // computes error = v_t - S k_t, then updates by k_t ⊗ (β * error).
        KernelRule {
            rule_id: "householder-rank1-update".to_string(),
            gpu_concept: "rank-1 Householder reflection state update".to_string(),
            ref_shape: "// naive: general state update (no error correction)\nfn state_update_naive(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] += beta * k[kv] * v[vv];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: error-correcting delta rule (Householder rank-1)\nfn delta_rule_update(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    // step 1: compute prediction error = v - S^T k\n    let mut error = [0.0f32; V];\n    for vv in 0..V {\n        let mut pred = 0.0;\n        for kv in 0..K { pred += state[kv][vv] * k[kv]; }\n        error[vv] = v[vv] - pred;\n    }\n    // step 2: rank-1 update: S += k ⊗ (beta * error)\n    for kv in 0..K {\n        let scaled_k = beta * k[kv];\n        for vv in 0..V {\n            state[kv][vv] += scaled_k * error[vv];\n        }\n    }\n}".to_string(),
        },
        // ── chunkwise-parallel-recurrence — full sequential → chunk-parallel ─
        // Source: fla chunk_delta_rule — the sequential recurrence is split into
        // chunks; intra-chunk interactions are parallelized via matmul, inter-chunk
        // state transitions remain sequential.
        KernelRule {
            rule_id: "chunkwise-parallel-recurrence".to_string(),
            gpu_concept: "chunkwise parallelism for state-carrying loops".to_string(),
            ref_shape: "// naive: fully sequential recurrence (O(N) sequential steps, no parallelism)\nfn recurrence_sequential(k: &[f32], v: &[f32], state: &mut [f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        // each step depends on previous state — fully sequential\n        state[t % state.len()] += k[t] * v[t];\n        out[t] = state[t % state.len()];\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: chunkwise parallel (O(N/chunk) sequential, intra-chunk parallel)\nfn recurrence_chunkwise(k: &[f32], v: &[f32], state: &mut [f32], n: usize, chunk: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t_start in (0..n).step_by(chunk) {\n        let t_end = (t_start + chunk).min(n);\n        // intra-chunk: parallel matmul for interactions within this chunk\n        let chunk_len = t_end - t_start;\n        let mut chunk_state = state.to_vec();\n        for t in t_start..t_end {\n            chunk_state[t % state.len()] += k[t] * v[t];\n            out[t] = chunk_state[t % state.len()];\n        }\n        // inter-chunk: sequential state transition\n        *state = chunk_state;\n    }\n    out\n}".to_string(),
        },
        // ── recurrent-vs-chunk-dispatch — one path → backend by seq length ──
        // Source: fla fused_recurrent_delta_rule vs chunk_delta_rule — the recurrent
        // path is faster for short sequences (low parallelism overhead), the chunk
        // path is faster for long sequences (more parallelism).
        KernelRule {
            rule_id: "recurrent-vs-chunk-dispatch".to_string(),
            gpu_concept: "sequence-length-aware backend dispatch".to_string(),
            ref_shape: "// naive: always use chunkwise (overhead dominates for short sequences)\nfn dispatch_always_chunk(k: &[f32], v: &[f32], n: usize) -> Vec<f32> {\n    let mut state = vec![0.0f32; 64];\n    recurrence_chunkwise(k, v, &mut state, n, 64)\n}".to_string(),
            opt_pattern: "// optimized: dispatch by sequence length (recurrent for short, chunk for long)\nfn dispatch_by_length(k: &[f32], v: &[f32], n: usize) -> Vec<f32> {\n    let mut state = vec![0.0f32; 64];\n    if n < 256 {\n        // short: sequential recurrent (low overhead)\n        recurrence_sequential(k, v, &mut state, n)\n    } else {\n        // long: chunkwise parallel (high throughput)\n        recurrence_chunkwise(k, v, &mut state, n, 64)\n    }\n}".to_string(),
        },
        // ── multi-write-per-step — single write → N Householder writes ────
        // Source: gated_delta_product.py — DeltaProduct does num_householder
        // Householder reflections per timestep under one forget gate.
        KernelRule {
            rule_id: "multi-write-per-step".to_string(),
            gpu_concept: "multiple state writes per timestep".to_string(),
            ref_shape: "// naive: single write per step (1 Householder reflection)\nfn step_single_write(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    delta_rule_update(state, k, v, beta);\n}".to_string(),
            opt_pattern: "// optimized: N writes per step under one gate (DeltaProduct)\nfn step_multi_write(state: &mut [[f32; V]; K], ks: &[[f32; K]], vs: &[[f32; V]], betas: &[f32], n: usize, gate: f32) {\n    // apply forget gate once\n    let decay = gate.exp();\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] *= decay;\n        }\n    }\n    // N Householder reflections\n    for j in 0..n {\n        delta_rule_update(state, &ks[j], &vs[j], betas[j]);\n    }\n}".to_string(),
        },
        // ── chained-two-pass-recurrence — single pass → two chained passes ─
        // Source: gsa.py — Gated Slot Attention chains two sequential recurrences
        // (K-state then V-state) with a softmax bridge between them.
        KernelRule {
            rule_id: "chained-two-pass-recurrence".to_string(),
            gpu_concept: "chained sequential recurrences with softmax bridge".to_string(),
            ref_shape: "// naive: single recurrence pass (one state matrix)\nfn single_pass_recurrence(k: &[f32], v: &[f32], q: &[f32], state: &mut [f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        state[t % state.len()] += k[t] * v[t];\n        out[t] = state[t % state.len()] * q[t];\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: two chained passes with softmax bridge (Gated Slot Attention)\nfn two_pass_recurrence(k: &[f32], v: &[f32], s: &[f32], q: &[f32], gate: &[f32], k_state: &mut [f32], v_state: &mut [f32], n: usize) -> Vec<f32> {\n    let mut intermediate = vec![0.0f32; n];\n    // pass 1: K-state recurrence → intermediate scores\n    for t in 0..n {\n        let decay = gate[t].exp();\n        k_state[t % k_state.len()] = k_state[t % k_state.len()] * decay + k[t] * s[t];\n        intermediate[t] = k_state[t % k_state.len()] * q[t];\n    }\n    // softmax bridge\n    let max_val = intermediate.iter().copied().fold(f32::NEG_INFINITY, f32::max);\n    let sum: f32 = intermediate.iter().map(|x| (x - max_val).exp()).sum();\n    let probs: Vec<f32> = intermediate.iter().map(|x| (x - max_val).exp() / sum).collect();\n    // pass 2: V-state recurrence → final output\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        let decay = gate[t].exp();\n        v_state[t % v_state.len()] = v_state[t % v_state.len()] * decay + s[t] * v[t];\n        out[t] = v_state[t % v_state.len()] * probs[t];\n    }\n    out\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category H: MoE dispatch + backward pass (Batch 4, 9 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── grouped-launch-cumsum-offsets — N launches → single grouped grid ─
        // Source: momoe_tl.py lin1_kernel + apply_experts — each block derives
        // its group membership + per-group row offset from a prefix-sum array.
        KernelRule {
            rule_id: "grouped-launch-cumsum-offsets".to_string(),
            gpu_concept: "grouped GEMM with prefix-sum offsets".to_string(),
            ref_shape: "// naive: one kernel launch per group, each sized independently\nfn grouped_matmul_naive(x: &[f32], w: &[&[f32]], out: &mut [f32], group_sizes: &[usize], d: usize, h: usize) {\n    let mut offset = 0;\n    for (g, &size) in group_sizes.iter().enumerate() {\n        let x_g = &x[offset * d .. (offset + size) * d];\n        let out_g = &mut out[offset * h .. (offset + size) * h];\n        launch_matmul_kernel(x_g, w[g], out_g); // separate launch per group\n        offset += size;\n    }\n}".to_string(),
            opt_pattern: "// optimized: single grid launch; each block resolves its group via cumsums\nfn grouped_matmul_cumsum(x: &[f32], w_flat: &[f32], out: &mut [f32], cumsums: &[usize], n_groups: usize) {\n    let total = cumsums[n_groups];\n    for block_id in 0..count_tiles(total, n_groups) {\n        let g = block_to_group(block_id, cumsums);\n        let cumsum_end = cumsums[g + 1];\n        let group_size = cumsum_end - cumsums[g];\n        let local_row = block_to_local_row(block_id, group_size);\n        let global_row = local_row + cumsum_end - group_size;\n        compute_tile(x, w_flat, out, g, global_row);\n    }\n}".to_string(),
        },
        // ── l2-grouped-tile-schedule — row-major → swizzled L2-aware mapping ──
        // Source: momoe_tl.py lin1/lin2/weights_update — grouped swizzle so
        // adjacent blocks share L2 cache lines of the weight matrix.
        // ── l2-grouped-tile-schedule — row-major → swizzled L2-aware mapping ──
        // Source: momoe_tl.py lin1/lin2/weights_update — grouped swizzle so
        // adjacent blocks share L2 cache lines of the weight matrix.
        //
        // Rider (Batch 72, NVIDIA/Bavoil thread-group-ID-swizzling writeup,
        // 2020): the applicability CONDITIONS are the decision content —
        // apply only when (1) VRAM is the top-throughput limiter, (2) the L2
        // load hit rate is well below 80%, and (3) adjacent thread groups'
        // memory footprints significantly overlap (wide kernels: denoisers,
        // ray queries; NOT narrow stencils — default row-major already reuses
        // neighbor data there, and the swizzle measured irrelevant). Tile N
        // groups along the primary direction before advancing; N=16 was the
        // measured sweet spot on the Battlefield V DXR denoiser (1440p, RTX
        // 2080): 47% faster, L2 read hit 63% -> 86%, the workload moved from
        // VRAM-latency-limited to SM-limited. A simple rectangular tiling
        // beat Morton order in practice. Re-profile after applying — once
        // SM-bound, further locality work is misdirected.
        KernelRule {
            rule_id: "l2-grouped-tile-schedule".to_string(),
            gpu_concept: "swizzled L2-cache-aware tile mapping RIDER (Batch 130 external instance, cloudrift-ai/emmy @ 2aaa0328 Apache-2.0): external confirmation from the CloudRift SGEMM study -- the shipped TMA kernel carries the swizzle inline with the comment 'CTA swizzle: rasterize in groups of SWIZ row-blocks for L2 A-tile reuse' and const int SWIZ=8; formula verbatim (recipes/sgemm_cublas_vs_tma/rtx5090_2026-04-07_16-42-49 report:90-100): int pid=blockIdx.x+blockIdx.y*gridDim.x; int grp=pid/(ntx*SWIZ); int rem=pid%(ntx*SWIZ); int by_s=grp*SWIZ+rem%SWIZ; int bx_s=rem/SWIZ; if(by_s>=nty||bx_s>=ntx)return; -- a second concrete formula shape for this rule (band-of-8-row-tiles walk; the popcorn opt_pattern's group-column-major walk is the first), plus the LAUNCHER half: the 1-D linear grid is PADDED to whole swizzle groups (grid_x = ntx * ((nty + 7) / 8) * 8, grid_y = 1, runner.py:311-317) and the kernel's early-return guard absorbs the padding blocks, so grouped rasterization costs a branch, not a ragged last group. Boundary carried: the swizzle moves DRAM throughput, NOT compute-pipe utilization -- the blog reports ncu-confirmed DRAM 32% -> 8.5% (cuBLAS 8.2%, blog-only numbers, NotBenchable at this pin), and the repo's own ncu single-mode capture shows post-swizzle DRAM parity with the vendor (dram__throughput 9.40% ours vs 9.80% cuBLAS at 8192, recipes/sgemm_cublas_vs_tma/ncu/summary.md) while the FMA-pipe gap to cuBLAS persists (68.00% vs 72.92%) -- 'CTA rasterization (fixed DRAM only)' is the recorded negative (blog negative-results table). Composes with vendor-dispatch-verify-before-outsource: the vendor-parity DRAM figure is what makes the remaining gap a SCHEDULING finding, not a memory finding. RIDER (Batch 238, ggml-org/llama.cpp #30215 @ 3e390af18, MIT, merged by 0cc4m 2026-10-15): three decision contents on the derivation + applicability axes, vulkan cm2 mulmat. (1) The group size is DERIVED from the L2 budget, never a constant -- the cloudrift rider's SWIZ=8 and the base rule's unexplained parameter both become derivable: '#define GROUP_A_BUDGET (8u * 1024u * 1024u)' then 'group_m = clamp(GROUP_A_BUDGET / max(a_panel_bytes, 1u), 1u, min(blocks_m, 32u))' with a_panel_bytes computed PER OPERAND DATATYPE ('#if defined(MULMAT_QUANT)' '(BM * p.K / mm_quant_k()) * MmABlockBytes'; DATA_A_F32 'BM * p.K * 4'; 'QUANT_K == 1' 'BM * p.K * 2'; else the q4-class '(BM * p.K) / 2 + (BM * p.K) / 16') -- the A-panel bytes one group must keep L2-resident decide the group width, floored at 1, capped at 32 and at the M extent. (2) The THIRD concrete formula shape -- the merged-axis linearization: the old schedule read 'const uint ic = gl_WorkGroupID.y;' and 'ir = gl_WorkGroupID.x % blocks_m'; the new one linearizes BOTH into 'const uint lin = gl_WorkGroupID.y * blocks_m + (gl_WorkGroupID.x % blocks_m);' then de-linearizes through (group_id = lin / tiles_per_group, first_m = group_id * group_m, gsize = min(blocks_m - first_m, group_m), in_group = lin - group_id * tiles_per_group) to 'ir = first_m + in_group % gsize; ic = in_group / gsize;' -- the grid SHAPE is unchanged, only the mapping of workgroup ids to tiles moves (a pure permutation, bit-identical outputs by construction). (3) The applicability boundary: the grouped schedule is deliberately NOT applied on the expert-indexed path -- '#ifdef MUL_MAT_ID' keeps 'const uint ir = gl_WorkGroupID.x % blocks_m;' -- because the uniform-A-panel bytes assumption breaks on the ragged expert path (per-expert A panels differ; the MoE face expert-major-grid-enumeration owns the grid-SHAPE axis for that lane). REGIME (the bandwidth-starved law, PR body): DGX Spark GB10 iGPU op-level +18-32% on the larger GEMMs (q8_0[5120,12288] +32.5%, q8_0[5120,10240] +30.1%, q6_K[5120,6144] +23.6%, q6_K[17408,5120] +20.9%); model-level pp2048 Qwen3.8-27B Q6_K +21.3% (602.8->731.5 t/s, the CUDA gap 1.31x->1.08x), Llama-3.1-8B Q6_K +16.0%, Gemma-31B Q5_K_M +17.0%, pp512 +14.7%/+8.1%/+12.3%; the SMALL-K quant class gains least (Llama-3.1-8B Q3_K_S pp2048 only +4.0%, gap 1.23x->1.18x -- narrow A panels already fit) and the small GEMM is noise (q8_0[5120,1024] -0.6%). ANTI-REGIME: 'I also tested this on RTX 3090, but it's perf-neutral there' + reviewer jeffbolznv: 'I had done something similar recently in vk_cooperative_matrix_perf, and in that test I found that it started to really matter once I get up to 8kx8k. I'm not seeing a gain on my 5090, but I expect this should benefit smaller GPUs' -- fat-bandwidth discrete GPUs are already DRAM-fed at these shapes; the low-bandwidth iGPU class (GB10 -- admitted by the occupancy-keyed value-split ladder's benchmark-gated CC list) pays. HONEST NULL: the reviewer's integer-division fast path (skip the remap when group_m >= blocks_m) was prototyped by the author and measured NO difference -- 'I can't measure any difference from it' -- the remap's integer divisions are free against a GEMM's work; landed as-is ('I'm fine with it as-is'). NotBenchable at our pins (vulkan cm2 path; our hosts are CUDA/Metal/CubeCL). Batch 238 / Plan 240 / Research 269.".to_string(),
            ref_shape: "// naive: row-major block mapping — adjacent pids walk columns, poor L2 reuse\nfn linear_tile_map(pid: usize, blocks_m: usize, blocks_n: usize) -> (usize, usize) {\n    let block_m = pid / blocks_n;\n    let block_n = pid % blocks_n;\n    (block_m, block_n)\n}".to_string(),
            opt_pattern: "// optimized: grouped swizzle — tile within a row-group before advancing\nfn grouped_tile_map(pid: usize, blocks_m: usize, blocks_n: usize, group_size: usize) -> (usize, usize) {\n    let group_id = pid / (group_size * blocks_n);\n    let start_m = group_id * group_size;\n    let start_pid = start_m * blocks_n;\n    let n_in_group = std::cmp::min(group_size, blocks_m - start_m);\n    let local = pid - start_pid;\n    let block_m = start_m + (local % n_in_group);\n    let block_n = local / n_in_group;\n    (block_m, block_n)\n}".to_string(),
        },
        // ── fused-matmul-activation — separate passes → epilogue fusion ──────
        // Source: momoe_tl.py lin1_kernel — up/gate projection + SwiGLU fused.
        KernelRule {
            rule_id: "fused-matmul-activation".to_string(),
            gpu_concept: "pointwise activation in matmul epilogue".to_string(),
            ref_shape: "// naive: matmul writes full intermediate to global memory, separate activation pass\nfn matmul_then_activation(x: &[f32], w: &[f32], out: &mut [f32], tmp: &mut [f32], m: usize, h: usize) {\n    matmul(x, w, tmp, m, h);\n    for i in 0..m * h {\n        out[i] = swiglu(tmp[i]);\n    }\n}".to_string(),
            opt_pattern: "// optimized: apply activation in registers before the single global store\nfn fused_matmul_activation(x: &[f32], w: &[f32], out: &mut [f32], m: usize, h: usize) {\n    for tile in tiles(m, h) {\n        let mut acc: [f32; TILE] = matmul_tile(x, w, tile);\n        for i in 0..TILE {\n            acc[i] = swiglu(acc[i]);\n        }\n        store_tile(out, tile, acc);\n    }\n}".to_string(),
        },
        // ── fused-backward-chain — two backward kernels → one fused kernel ──
        // Source: momoe_tl.py lin2_and_swiglu_backward — linear + activation
        // backward fused so grad_h stays in registers.
        KernelRule {
            rule_id: "fused-backward-chain".to_string(),
            gpu_concept: "two backward ops fused into one kernel".to_string(),
            ref_shape: "// naive: two separate kernels with a global-memory handoff\nfn backward_separate(grad_y: &[f32], w2: &[f32], activations: &[f32], grad_z: &mut [f32]) {\n    let mut grad_h = vec![0.0f32; grad_y.len()];\n    linear_backward(grad_y, w2, &mut grad_h);\n    activation_backward(&grad_h, activations, grad_z);\n}".to_string(),
            opt_pattern: "// optimized: single fused kernel — grad_h stays in registers\nfn backward_fused(grad_y: &[f32], w2: &[f32], activations: &[f32], grad_z: &mut [f32]) {\n    for tile in tiles(grad_y.len()) {\n        let grad_h: [f32; TILE] = linear_backward_tile(grad_y, w2, tile);\n        let grad_z_tile = activation_backward_tile(&grad_h, activations, tile);\n        store_tile(grad_z, tile, grad_z_tile);\n    }\n}".to_string(),
        },
        // ── activation-recompute-budget — save all → partial save + recompute ─
        // Source: momoe_tl.py save logic + swiglu_backward branch — save Msave
        // rows, recompute the rest in backward (activation checkpointing).
        KernelRule {
            rule_id: "activation-recompute-budget".to_string(),
            gpu_concept: "partial activation save with in-kernel recompute".to_string(),
            ref_shape: "// naive: save all activations for the backward pass\nfn backward_all_saved(saved_a: &[f32], saved_b: &[f32], grad_in: &[f32], grad_out: &mut [f32], m: usize) {\n    for i in 0..m {\n        let a = saved_a[i];\n        let b = saved_b[i];\n        grad_out[i] = activation_grad(a, b, grad_in[i]);\n    }\n}".to_string(),
            opt_pattern: "// optimized: save a fraction, recompute the rest in backward\nfn backward_recompute_budget(\n    saved_a: &[f32], saved_b: &[f32], msave: usize,\n    x: &[f32], w: &[f32], grad_in: &[f32], grad_out: &mut [f32], m: usize,\n) {\n    for i in 0..m {\n        let (a, b) = if i < msave {\n            (saved_a[i], saved_b[i])\n        } else {\n            recompute_forward(x, w, i)\n        };\n        grad_out[i] = activation_grad(a, b, grad_in[i]);\n    }\n}".to_string(),
        },
        // ── atomic-scatter-reduction — locked → lock-free atomic add ────────
        // Source: momoe_tl.py lin1_backward + scatter_cols — concurrent blocks
        // scatter to overlapping output slots via atomic_add.
        KernelRule {
            rule_id: "atomic-scatter-reduction".to_string(),
            gpu_concept: "lock-free atomic scatter for parallel writes".to_string(),
            ref_shape: "// naive: serialize overlapping writes with a lock per output element\nfn scatter_grad_locked(grads: &[(usize, f32)], out: &mut [f32], locks: &mut [bool]) {\n    for &(idx, val) in grads {\n        while locks[idx] { spin(); }\n        locks[idx] = true;\n        out[idx] += val;\n        locks[idx] = false;\n    }\n}".to_string(),
            opt_pattern: "// optimized: relaxed atomic add — concurrent blocks accumulate without locks\nuse std::sync::atomic::{AtomicU32, Ordering};\nstruct AtomicF32(AtomicU32);\nfn scatter_grad_atomic(grads: &[(usize, f32)], out: &mut [AtomicF32]) {\n    for &(idx, val) in grads {\n        let bits = val.to_bits();\n        out[idx].0.fetch_add(bits, Ordering::Relaxed);\n    }\n}".to_string(),
        },
        // ── weight-grad-batched-outer-product — scalar loop → batched GEMM ──
        // Source: momoe_tl.py weights_update_kernel — G_W = A^T @ G_b as a
        // single tiled matmul, not per-token rank-1 updates.
        KernelRule {
            rule_id: "weight-grad-batched-outer-product".to_string(),
            gpu_concept: "weight gradients as batched outer product GEMM".to_string(),
            ref_shape: "// naive: accumulate weight grad one token at a time (rank-1 updates)\nfn weight_grad_naive(a: &[f32], grad_b: &[f32], grad_w: &mut [f32], m: usize, x: usize, y: usize) {\n    for mi in 0..m {\n        for i in 0..x {\n            for j in 0..y {\n                grad_w[i * y + j] += a[mi * x + i] * grad_b[mi * y + j];\n            }\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: batched outer product as tiled GEMM over the token slice\nfn weight_grad_batched(a: &[f32], grad_b: &[f32], grad_w: &mut [f32], group_start: usize, group_size: usize, x: usize, y: usize) {\n    let a_g = &a[group_start * x ..];\n    let gb_g = &grad_b[group_start * y ..];\n    tiled_matmul_accumulate(a_g, gb_g, grad_w, x, y, group_size);\n}".to_string(),
        },
        // ── csr-index-via-prefix-sum — host nonzero → GPU prefix-sum CSR ────
        // Source: momoe_tl.py scatter_cols + find_used_experts — build CSR
        // layout in-kernel via prefix sum + atomic slot counter.
        KernelRule {
            rule_id: "csr-index-via-prefix-sum".to_string(),
            gpu_concept: "GPU-side CSR sparse layout via prefix sum".to_string(),
            ref_shape: "// naive: host-side boolean scan + push to build index lists\nfn build_indices_host(mask: &[bool], n_cols: usize, n_rows: usize, indices: &mut Vec<(usize, usize)>) {\n    indices.clear();\n    for row in 0..n_rows {\n        for col in 0..n_cols {\n            if mask[row * n_cols + col] {\n                indices.push((row, col));\n            }\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: GPU kernel — prefix sum within tiles + atomic slot counter\nuse std::sync::atomic::{AtomicUsize, Ordering};\nfn build_indices_prefix_sum(mask: &[bool], col_idx: &mut [usize], which: &mut [usize], crow: &mut [usize], counts: &[AtomicUsize], n_cols: usize, n_rows: usize) {\n    for row in 0..n_rows {\n        let mut running = 0usize;\n        for start in (0..n_cols).step_by(BLOCK) {\n            let m = load_mask_tile(mask, row, start, BLOCK);\n            let prefix = prefix_sum(&m);\n            for k in 0..BLOCK {\n                if m[k] {\n                    let slot = crow[row] + running + prefix[k];\n                    col_idx[slot] = start + k;\n                    which[slot] = counts[start + k].fetch_add(1, Ordering::Relaxed);\n                }\n            }\n            running += sum(&m);\n        }\n        crow[row + 1] = crow[row] + running;\n    }\n}".to_string(),
        },
        // ── autotune-aggregate-workload-key — fixed config → aggregate key ──
        // Source: momoe_tl.py autotune decorators — key on Msum (total tokens
        // across all groups), not per-group shapes, for irregular batches.
        KernelRule {
            rule_id: "autotune-aggregate-workload-key".to_string(),
            gpu_concept: "autotune keyed on aggregate workload size".to_string(),
            ref_shape: "// naive: fixed tile config regardless of total work or per-group variance\nfn launch_fixed_config(total_work: usize, per_group_sizes: &[usize], data: &[f32]) {\n    let block_size = 128;\n    for start in (0..total_work).step_by(block_size) {\n        process_tile(&data[start..]);\n    }\n}".to_string(),
            opt_pattern: "// optimized: re-select tile config when aggregate work crosses thresholds\nfn launch_autotuned_aggregate(total_work: usize, per_group_sizes: &[usize], data: &[f32]) {\n    let block_size = autotune_lookup(total_work);\n    for start in (0..total_work).step_by(block_size) {\n        process_tile(&data[start..]);\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category I: NSA block-sparse selection (Batch 5, 4 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── block-sparse-gather — dense loop → block-indexed gather ────────
        // Source: nsa_tl.py _sel_attn_fwd_kernel — iterate block_indices array
        // instead of all N positions.
        KernelRule {
            rule_id: "block-sparse-gather".to_string(),
            gpu_concept: "block-sparse indexed gather via selection array".to_string(),
            ref_shape: "// naive: iterate all positions, compute score for every key\nfn attend_dense(q: &[f32], k: &[f32], v: &[f32], n: usize, d: usize, out: &mut [f32]) {\n    for i in 0..n {\n        let score = dot(&q[..d], &k[i * d..(i + 1) * d]);\n        for j in 0..d {\n            out[j] += score * v[i * d + j];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: iterate only selected blocks via index array\nfn attend_block_sparse(\n    q: &[f32], k: &[f32], v: &[f32],\n    block_indices: &[usize], block_size: usize, d: usize, out: &mut [f32],\n) {\n    for &bi in block_indices {\n        let base = bi * block_size;\n        for j in 0..block_size {\n            let row = (base + j) * d;\n            let score = dot(&q[..d], &k[row..row + d]);\n            for dd in 0..d {\n                out[dd] += score * v[row + dd];\n            }\n        }\n    }\n}".to_string(),
        },
        // ── precompute-scalar-reduction — recompute per-block → hoist once ──
        // Source: nsa_tl.py _sel_attn_bwd_preprocess_kernel — delta = sum(o*do)
        // is invariant; precompute once, not per KV block.
        KernelRule {
            rule_id: "precompute-scalar-reduction".to_string(),
            gpu_concept: "hoist invariant row reduction out of per-block loop".to_string(),
            ref_shape: "// naive: recompute the row-wise delta inside every block iteration\nfn bwd_recompute_delta(o: &[f32], do_grad: &[f32], d: usize, block_indices: &[usize], dk: &mut [f32]) {\n    for &blk in block_indices {\n        let mut delta = 0.0f32;\n        for i in 0..d {\n            delta += o[i] * do_grad[i];\n        }\n        dk[blk] += grad_from_delta(delta);\n    }\n}".to_string(),
            opt_pattern: "// optimized: delta precomputed once in separate pass, passed in\nfn precompute_delta(o: &[f32], do_grad: &[f32], d: usize, delta: &mut [f32]) {\n    for row in 0..delta.len() {\n        let base = row * d;\n        delta[row] = (0..d).map(|i| o[base + i] * do_grad[base + i]).sum();\n    }\n}\nfn bwd_with_delta(delta: &[f32], block_indices: &[usize], dk: &mut [f32]) {\n    for (row, &blk) in block_indices.iter().enumerate() {\n        dk[blk] += grad_from_delta(delta[row]);\n    }\n}".to_string(),
        },
        // ── block-compression-before-routing — score full → score compressed ─
        // Source: nsa_tl.py compression_attention + nsa — mean-pool to coarse
        // blocks, route on compressed reps, attend top-k at full res.
        KernelRule {
            rule_id: "block-compression-before-routing".to_string(),
            gpu_concept: "pool to coarse blocks, route on compressed reps".to_string(),
            ref_shape: "// naive: score every fine-grained position to decide what to attend to\nfn select_over_full(q: &[f32], k: &[f32], n: usize, d: usize, top_count: usize) -> Vec<usize> {\n    let mut scores = vec![0.0f32; n];\n    for i in 0..n {\n        scores[i] = dot(&q[..d], &k[i * d..(i + 1) * d]);\n    }\n    argtopk(&scores, top_count)\n}".to_string(),
            opt_pattern: "// optimized: pool to coarse blocks, score on compressed reps\nfn select_over_compressed(\n    q: &[f32], k: &[f32], block_size: usize, n_blocks: usize, d: usize, top_count: usize,\n) -> Vec<usize> {\n    let mut block_scores = vec![0.0f32; n_blocks];\n    for b in 0..n_blocks {\n        let mut pooled = vec![0.0f32; d];\n        for j in 0..block_size {\n            for dd in 0..d {\n                pooled[dd] += k[(b * block_size + j) * d + dd];\n            }\n        }\n        for dd in 0..d {\n            pooled[dd] /= block_size as f32;\n        }\n        block_scores[b] = dot(&q[..d], &pooled);\n    }\n    argtopk(&block_scores, top_count)\n}".to_string(),
        },
        // ── sliding-window-plus-sparse — single pass → local + global split ─
        // Source: nsa_tl.py nsa — decompose into sliding window + block-sparse,
        // combine via gated weighted sum (Longformer/BigBird/NSA pattern).
        KernelRule {
            rule_id: "sliding-window-plus-sparse".to_string(),
            gpu_concept: "decompose attention into local + global sparse, gated sum".to_string(),
            ref_shape: "// naive: single attention pass must handle both local and global patterns\nfn attend_single(q: &[f32], k: &[f32], v: &[f32], n: usize, d: usize, out: &mut [f32]) {\n    for i in 0..n {\n        let score = dot(&q[..d], &k[i * d..(i + 1) * d]);\n        for j in 0..d {\n            out[j] += score * v[i * d + j];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: decompose into local window + global sparse, combine with gates\nfn attend_decomposed(\n    q: &[f32], k: &[f32], v: &[f32], d: usize,\n    g_local: f32, g_global: f32,\n    block_indices: &[usize], window_left: usize,\n    out: &mut [f32],\n) {\n    let mut local_out = vec![0.0f32; d];\n    let mut global_out = vec![0.0f32; d];\n    attend_sliding_window(q, k, v, d, window_left, &mut local_out);\n    attend_block_sparse(q, k, v, d, block_indices, &mut global_out);\n    for j in 0..d {\n        out[j] = local_out[j] * g_local + global_out[j] * g_global;\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category J: Varlen packing + chunked reductions (Batch 6, 3 rules)
        // ════════════════════════════════════════════════════════════════════
        // ── varlen-packing — padded batch → packed cu_seqlens buffer ────────
        // Source: attn_varlen.py — pack variable-length sequences end-to-end
        // via cu_seqlens offsets; eliminate padding waste.
        KernelRule {
            rule_id: "varlen-packing".to_string(),
            gpu_concept: "pack variable-length sequences via cu_seqlens, no padding".to_string(),
            ref_shape: "// naive: pad all sequences to max length (waste compute on padding)\nfn attention_padded(seqs: &[&[f32]], max_len: usize, d: usize, out: &mut [f32]) {\n    let batch = seqs.len();\n    let mut padded = vec![0.0f32; batch * max_len * d];\n    for (b, seq) in seqs.iter().enumerate() {\n        for (i, &val) in seq.iter().enumerate() {\n            padded[b * max_len * d + i * d] = val;\n        }\n    }\n    // attention runs over the full padded matrix (padding tokens waste compute)\n    attention_kernel(&padded, batch, max_len, d, out);\n}".to_string(),
            opt_pattern: "// optimized: pack all sequences end-to-end via cu_seqlens offsets\nfn attention_varlen(seqs: &[&[f32]], d: usize, out: &mut [f32]) {\n    let total: usize = seqs.iter().map(|s| s.len() / d).sum();\n    let mut packed = vec![0.0f32; total * d];\n    let mut cu_seqlens = vec![0usize; seqs.len() + 1];\n    let mut offset = 0;\n    for (b, seq) in seqs.iter().enumerate() {\n        packed[offset..offset + seq.len()].copy_from_slice(seq);\n        offset += seq.len();\n        cu_seqlens[b + 1] = offset / d;\n    }\n    // attention runs only on real tokens (no padding, no waste)\n    attention_varlen_kernel(&packed, &cu_seqlens, d, out);\n}".to_string(),
        },
        // ── chunk-reset-cumsum — full cumsum → chunk-boundary-reset cumsum ─
        // Source: chunk_local_cumsum.py — cumsum resets at chunk_size boundaries.
        KernelRule {
            rule_id: "chunk-reset-cumsum".to_string(),
            gpu_concept: "cumsum that resets at chunk boundaries".to_string(),
            ref_shape: "// naive: full sequence cumsum (accumulates across entire sequence)\nfn cumsum_full(g: &[f32], out: &mut [f32]) {\n    let mut acc = 0.0f32;\n    for (i, &val) in g.iter().enumerate() {\n        acc += val;\n        out[i] = acc;\n    }\n}".to_string(),
            opt_pattern: "// optimized: chunk-reset cumsum (resets at chunk_size boundaries)\nfn cumsum_chunk_reset(g: &[f32], out: &mut [f32], chunk_size: usize) {\n    let mut acc = 0.0f32;\n    for (i, &val) in g.iter().enumerate() {\n        if i % chunk_size == 0 {\n            acc = 0.0; // reset at chunk boundary\n        }\n        acc += val;\n        out[i] = acc;\n    }\n}".to_string(),
        },
        // ── cumsum-additive-bias — O(N²) bias matrix → O(N) cumsum difference ─
        // Source: forgetting_attn.py — bias[i,j] = cumsum(g)[i] - cumsum(g)[j].
        KernelRule {
            rule_id: "cumsum-additive-bias".to_string(),
            gpu_concept: "additive bias via cumulative sum of a gate".to_string(),
            ref_shape: "// naive: materialize full N×N bias matrix (O(N²) memory + compute)\nfn attention_with_bias_matrix(q: &[f32], k: &[f32], g: &[f32], n: usize, d: usize, out: &mut [f32]) {\n    let mut bias = vec![0.0f32; n * n];\n    for i in 0..n {\n        for j in 0..n {\n            let mut sum = 0.0;\n            for t in 0..=i { sum += g[t]; }\n            let mut sum_j = 0.0;\n            for t in 0..=j { sum_j += g[t]; }\n            bias[i * n + j] = sum - sum_j;\n        }\n    }\n    attention_with_bias(q, k, &bias, n, d, out);\n}".to_string(),
            opt_pattern: "// optimized: bias = cumsum(g)[i] - cumsum(g)[j] (O(N) to compute cumsum)\nfn attention_with_cumsum_bias(q: &[f32], k: &[f32], g: &[f32], n: usize, d: usize, out: &mut [f32]) {\n    let mut cumsum = vec![0.0f32; n];\n    let mut acc = 0.0f32;\n    for (i, &val) in g.iter().enumerate() {\n        acc += val;\n        cumsum[i] = acc;\n    }\n    // attention adds (cumsum[i] - cumsum[j]) to each score — no bias matrix\n    attention_with_cumsum_diff(q, k, &cumsum, n, d, out);\n}".to_string(),
        },
        // ── chunk-split-mean-pool — running sum → split+mean (Batch 10, popcorn mean_pooling.py) ──
        // Source: mean_pooling.py — torch.stack([chunk.mean(1) for chunk in x.split(C, 1)], 1).
        // Split-then-mean exposes the fixed chunk size for unrollable inner reduction.
        KernelRule {
            rule_id: "chunk-split-mean-pool".to_string(),
            gpu_concept: "chunked reduction via split + per-chunk mean".to_string(),
            ref_shape: "// naive: running sum into output buffer (sequential accumulator)\nfn mean_pool_running(x: &[f32], seq: usize, dim: usize, chunk: usize) -> Vec<f32> {\n    let n_chunks = seq / chunk;\n    let mut out = vec![0.0; n_chunks * dim];\n    for c in 0..n_chunks {\n        for d in 0..dim {\n            let mut acc = 0.0;\n            for t in 0..chunk {\n                acc += x[(c * chunk + t) * dim + d];\n            }\n            out[c * dim + d] = acc / chunk as f32;\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: split into chunks, mean each chunk (exposes chunk structure)\nfn mean_pool_split(x: &[f32], seq: usize, dim: usize, chunk: usize) -> Vec<f32> {\n    let n_chunks = seq / chunk;\n    let mut out = vec![0.0; n_chunks * dim];\n    for c in 0..n_chunks {\n        let base = c * chunk * dim;\n        for d in 0..dim {\n            // inner loop over fixed-size chunk — unrollable\n            let mut acc = 0.0;\n            for t in 0..chunk {\n                acc += x[base + t * dim + d];\n            }\n            out[c * dim + d] = acc / chunk as f32;\n        }\n    }\n    out\n}".to_string(),
        },
        // ── ragged-chunk-pad — branch on tail → pad to block_size (Batch 10, popcorn solve_tril.py) ──
        // Source: solve_tril.py — F.pad(inverse, (0, block_size - length)) before cat.
        // Unifies the last ragged chunk with the full chunks (one code path).
        KernelRule {
            rule_id: "ragged-chunk-pad".to_string(),
            gpu_concept: "pad ragged tail to block_size for uniform chunk processing. RIDER (Batch 229 / Plan 231 -- the STAGING-SIDE SYMMETRY face; ggml-org/llama.cpp @ 411d548515017ca75c2a12c39659b6e7e1551dd7 #29533, MIT, landed 2026-10-13): this rule's own strategy pads the INPUT so one uniform path processes every chunk; the sibling strategy BOUNDS the loop instead of padding the input (a tail-truncated K loop in a tiled matmul, the dispatch split when a small tile still produces more workgroups than maxComputeWorkGroupCount allows) -- and the bound carries a SYMMETRY CONTRACT this family did not own: EVERY operand stager feeding one consumer must write the FULL staged tile, zero-filling past the bound, because the consumer reads the full staged width regardless of the logical bound. The bug, verbatim: 'The A side skipped the shared memory write for the k steps past end_k, while the B side was already told the bound and zero filled. The A registers are read for all BK_STEP steps, so the stale scale went into the dot product and produced NaN at k = 32. Pass the bound to block_a_to_shmem and write zeros when it is false, for every DATA_A type, mirroring the B side.' THREE decision contents. (1) THE REPAIR SHAPE -- the guarded CALL ('if (block + k_step * BK < end_k) block_a_to_shmem(...)') becomes an UNCONDITIONAL call with the bound as a PARAMETER ('block_a_to_shmem(..., block + k_step * BK < end_k)') whose else-branch writes zeros: the skip-write form is the bug because shared memory is NEVER zero by default -- the unwritten tail holds the PREVIOUS tile's bytes (stale smem), and the stale DEQUANT MULTIPLIER (buf_a.dm), not just the payload bits, is what entered the product -- the else-branch must zero dm too, for EVERY DATA_A type (a per-type family of stagers means the bound must reach every one of them; one fixed stager beside one bounded stager is exactly the defect). (2) WHY THE ASYMMETRY IS SILENT -- two co-consumed stagers, one zero-fills past the bound and one skips: full chunks dominate the timing and the corruption only surfaces AT the chunk boundary (NaN at k=32 = the first BK_STEP boundary), so the mismatch survives full-chunk tests -- tail handling is only exercised by a test whose shape actually truncates (the commit adds 'two batched cases of the chunked shape'). (3) THE DISPATCH-SPLIT HALF (the context that CREATES the ragged tail, same commit): a chunked dispatch shifts the B and D SUBBUFFER COLUMN WINDOWS per chunk ('b_chunk.offset += n_base * b_col_bytes; b_chunk.size -= n_base * b_col_bytes') -- which requires 'GGML_ASSERT(b.size != VK_WHOLE_SIZE && d.size != VK_WHOLE_SIZE)' (you cannot offset a WHOLE_SIZE subbuffer) and a col-stride that knows the buffer's KIND (a raw src1 tensor keeps its own nb[1]; a staging buffer is tightly packed -- two different b_col_bytes formulas), and every derived bound shifts with the chunk (the push constants carry 'padded_n - n_base', never the global padded_n). KINSHIP (do not double-count): the B226 pad-at-the-producer rider (on fa-fast-path-validated-exact-shape-admission) and the B195/B213 pad ladder (on dtype-native-source-read-with-shared-admission-predicate) own where pad zeros COME FROM at the producer/consumer boundary -- THIS rider owns the in-kernel STAGING boundary and its law is SYMMETRY between co-consumed stagers, not economics; tma-single-issuer-bulk-tile's hardware OOB zero-fill is the descriptor-managed twin (the HW fill replaces manual bound-carrying -- where TMA serves the tile, this law is discharged by hardware); kv-first-append-unzeroed-alloc-offset-gated-tail is the INVERSE posture (the fill is elided because the tail is structurally UNREAD -- here the consumer reads the full staged width, so the fill is MANDATORY); masked-boundary-load is per-element LOAD fallback -- this is a WRITE-side coverage contract between stagers, not a read guard. NotBenchable here (the fix is a correctness repair -- no perf number ships; the win is the dispatch split it enables, source-context only). Verified verbatim at 411d5485 via the .patch endpoint (subject, the zero-fill sub-commit message, block_a_to_shmem's is_in_bounds else-branch incl. dm=0, the VK_WHOLE_SIZE assert, the b_col_bytes pair, padded_n - n_base). Batch 229 / Plan 231.".to_string(),
            ref_shape: "// naive: branch on 'is this the last (ragged) chunk?'\nfn process_chunks_branchy(chunks: &[&[f32]], block_size: usize) -> Vec<f32> {\n    let mut out = Vec::new();\n    for (i, chunk) in chunks.iter().enumerate() {\n        if i == chunks.len() - 1 && chunk.len() < block_size {\n            // ragged tail: special-case (no pad, separate cat path)\n            for v in *chunk { out.push(*v); }\n        } else {\n            // full chunk: process via fixed-size kernel\n            for v in *chunk { out.push(process_elem(*v)); }\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: pad last chunk to block_size, one uniform path\nfn process_chunks_padded(chunks: &[&[f32]], block_size: usize) -> Vec<f32> {\n    let mut out = Vec::new();\n    for chunk in chunks {\n        let mut padded: Vec<f32> = chunk.to_vec();\n        padded.resize(block_size, 0.0); // pad ragged tail\n        for v in &padded { out.push(process_elem(*v)); }\n    }\n    out\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category K: Autotune dispatch (Batch 7, 4 rules — burn-cubecl)
        // ════════════════════════════════════════════════════════════════════
        // ── panic-safe-autotune-candidate — direct call → catch_unwind ────
        // Source: burn-cubecl matmul/tune/base.rs — launch_matmul_safe wraps
        // launch_matmul in catch_unwind so a divide-by-zero panic in tiling
        // blueprint autotune becomes Err, letting the autotuner skip the kernel
        // instead of crashing the whole search.
        KernelRule {
            rule_id: "panic-safe-autotune-candidate".to_string(),
            gpu_concept: "panic-resilient autotune candidate. RIDER (Batch 267, the REJECTION-SEMANTICS face — what a candidate's own error MEANS for fallback eligibility; tracel-ai/cubecl @ 1b4099b262566f62ffe1480cd4a10b6026dc158e #1876 'never fallback on rejected candidate', Apache-2.0 OR MIT — dual license verified at the clone root, quotes verbatim at the pin clone): this rule owns the exclusion half (a candidate that PANICS is skipped, the tuning survives); the cubecl fix owns the CLASSIFICATION half the exclusion needs — WHICH candidate failures are kernel-unusability evidence and which are not, when every candidate failed and the harness must either decide one unmeasured or abort. A new AutotuneError::Rejected splits the old Unknown: a candidate whose OWN function returned an error now maps to Rejected (crates/cubecl-runtime/src/tune/base.rs: `func(inputs).map_err(|err| AutotuneError::Rejected { name: name_for_err.to_string(), reason: err.into() })` — was AutotuneError::Unknown), doc verbatim (crates/cubecl-runtime/src/tune/tuner.rs:77-81): 'The tunable's own function returned an error, so it queued no work for this problem: it does not serve these inputs, or failed before launching. Unlike [`Unknown`](Self::Unknown), which the harness reports around work that was queued (a profile it could not take or resolve, an execution that failed, memory that ran out), this says nothing ran — so it is never a kernel to fall back on.' The all-failed arm's comment, verbatim (crates/cubecl-runtime/src/tune/schedule.rs:466-480, the fixed form): 'Every candidate failed. A candidate that *executed* but could not be *measured* — `Unknown` wraps benchmark-harness failures like a profiling hiccup (timestamp query sets on a busy stream), `InvalidSamples` collected timings it cannot trust — is still a usable kernel: decide the first such, unmeasured, rather than panicking the device thread. A dead tune poisons every stream sharing it, and the decided kernel executes for real right after — if it truly cannot run, that failure surfaces there, exactly as a measured winner's would. Only launch failures, manual skips and rejections — a candidate whose own function returned an error, so nothing ran — say the kernel itself is unusable: deciding a rejection would run it for real and fail the same way. When nothing else remains, there is genuinely no kernel to run.' — and the executed_unmeasured selector matches ONLY `Unknown | InvalidSamples`; a Rejected candidate is excluded, so a key whose only remaining outcomes are rejections aborts instead of electing a candidate that cannot run (the regression test names the law: a_round_that_measures_nothing_never_falls_back_on_a_rejection, over addition_set_rejected_then_unmeasured — the rejection also stops profiling, so the correct candidate runs UNMEASURED and the tuner must not decide the rejection). THE LAW: the executed-unmeasured fallback is licensed by EVIDENCE OF EXECUTION — a candidate that queued work and lost only the MEASUREMENT (harness failures around queued work) is a usable kernel the very next real execution will judge anyway; a candidate whose own function refused the problem (an error before launch, nothing queued) is exclusion evidence in the SAME class as a launch failure or a manual skip — never the fallback. HOST FIT NOTE (disclosed at authoring): this rider lands here, not on tuned-config-resource-limit-fallback (B128, entries_gpu_34) — that rule's fallback is the BASELINE CONFIG at dispatch (a resource-limit re-check of the winner), a different fallback on a different axis; the corpus's own distinct-from line already routes candidate failures DURING tuning to this rule, and the rejection law is exactly that axis. CITE: crates/cubecl-runtime/src/tune/base.rs (the map_err swap), crates/cubecl-runtime/src/tune/tuner.rs:77-88 (the Rejected variant + doc), crates/cubecl-runtime/src/tune/schedule.rs:466-487 (the fixed comment + the Unknown|InvalidSamples-only selector), crates/cubecl-server/tests/dummy/tune/operation_sets.rs (addition_set_rejected_then_unmeasured) + crates/cubecl-server/tests/integration_test.rs (the round test). NotBenchable here (count-neutral rider, classification/correctness class — the instrument is the fallback-exclusion test, never a wall-clock bar). Batch 267 rider.".to_string(),
            ref_shape: "// naive: direct kernel call panics on unsupported shape (aborts the search)\nfn run_candidate(data: &[f32]) -> Vec<f32> {\n    tile_kernel(data) // panics if data.len() not multiple of TILE\n}".to_string(),
            opt_pattern: "// optimized: catch_unwind wraps the kernel — panic becomes Err, autotune skips it\nfn run_candidate_safe(data: &[f32]) -> Result<Vec<f32>, String> {\n    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {\n        tile_kernel(data)\n    })).map_err(|_| \"candidate panicked\".to_string())\n}".to_string(),
        },
        // ── multi-priority-tune-groups — flat list → priority-ranked groups ─
        // Source: burn-cubecl matmul/tune/base.rs — TuneGroup with PRIORITY_MAX/HIGH/
        // MEDIUM/MIN/NEVER(-1) keyed on MatmulKind + GlobalScale + skew-factor. A
        // candidate with priority -1 is skipped entirely; higher priority is
        // benchmarked first. This replaces a flat candidate sweep with a shape-
        // keyed pruned search.
        KernelRule {
            rule_id: "multi-priority-tune-groups".to_string(),
            gpu_concept: "shape-keyed priority-ranked autotune groups".to_string(),
            ref_shape: "// naive: flat candidate list, all equal priority (tuner benchmarks all every shape)\nfn candidates() -> Vec<(&str, i8)> {\n    vec![(\"kernel_tma\", 0), (\"kernel_gemv\", 0), (\"kernel_naive\", 0)]\n}".to_string(),
            opt_pattern: "// optimized: priority-ranked groups keyed on shape (SKIP = -1 prunes the candidate)\nfn candidates_keyed(m: usize, n: usize, k: usize) -> Vec<(&str, i8)> {\n    let is_gemv = m == 1 || n == 1;\n    let skew = m.max(n).max(k) / m.min(n).min(k).max(1);\n    vec![\n        (\"kernel_tma\", if !is_gemv && skew < 4 { 3 } else { -1 }),\n        (\"kernel_gemv\", if is_gemv { 3 } else { -1 }),\n        (\"kernel_naive\", 0),\n    ]\n}".to_string(),
        },
        // ── shape-kind-dispatch — one kernel → kind-keyed family ──────────
        // Source: burn-cubecl matmul/tune/base.rs — MatmulKind (General/MatVec/
        // VecMat/InnerProduct) routes to distinct kernel families. A dedicated
        // GEMV kernel (plane-parallel or unit-perpendicular) runs for matvec;
        // a general tiled GEMM runs otherwise.
        KernelRule {
            rule_id: "shape-kind-dispatch".to_string(),
            gpu_concept: "matmul-kind-keyed kernel family dispatch. RIDER (Batch 280, PrismML llama.cpp fork @ `23f6463d2c` #380 'cuda: thin F32 mat-muls take mmvf up to 8 columns on Ampere+', MIT -- the kind key must include the OUTPUT EXTENT, not the batch alone -- the thin-weight starvation face): the F32 mul_mat family split is keyed on batch (the vector kernel `mmvf` only at `ne11 <= 3`); the widening adds the weight's thinness as the second key axis, admission verbatim (mmvf.cu at the pin): '// A thin src0 (e.g. the 32-row GDN ssm_alpha/ssm_beta projections) gives the tiled mul_mat_f' + '// only a block or two; the vector kernel is several times faster up to its batch limit.' + 'if (src0_ne[1] <= 512 && ne11 <= MMVF_MAX_BATCH_SIZE) { return true; }' -- the mechanism is per-family parallelism scaling: the tiled kernel's grid scales with the OUTPUT extent (a 32-row weight = 'a block or two' of work at ANY batch), while the vector kernel scales over the reduction axis and holds its win up to its own batch limit (8); so the matvec-kind admission is the CONJUNCTION (batch within the vector limit) AND (output thin enough that tiled starves), never the batch axis alone. TWO PROPERTIES make this widening free of the usual admission-risk tax: the vector path is EXACT FP32 ('The columns are batch-invariant and exact FP32' -- the admission replaces the lossy TF32 tiled incumbent, so exactness IMPROVES with the speedup: 'Not token-identical to the previous TF32 path. The KLD gate against F16 references is within the noise floor (9B ub8 +1.6e-5 mean KLD, ub4 -1.6e-5; 99.9% KLD better in all cells)'), and the regime is arch-carried (Ampere+ via the `ampere_mma_available` arm; measured '+6% MTP K=3 decode on H200', source-reported). Regime addition to the base rule: thin-output projections whose weight rows underfill the tiled kernel's block grid -- GDN alpha/beta projections are the source's own instance (OUR GDN lane's exact shape). Anti-regime: fat weights at the same batch (tiled wins -- the original `ne11 <= 3` arm stays as the else); batches past the vector kernel's limit (the tiled path resumes); architectures without the vector kernel's mma requirement. Verified verbatim at `23f6463d2c` in the local prism clone at tag `prism-b10809-b3ebc47`. Count-neutral rider, Batch 280 / Plan 284.".to_string(),
            ref_shape: "// naive: single general GEMM kernel for all shapes (slow for matvec: 99% waste)\nfn matmul(a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {\n    general_gemm(a, b, m, n, k)\n}".to_string(),
            opt_pattern: "// optimized: dispatch on matmul kind — matvec uses a dedicated GEMV kernel\nenum MatmulKind { General, MatVec, VecMat, InnerProduct }\nfn matmul_dispatched(a: &[f32], b: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {\n    let kind = if m == 1 { MatmulKind::VecMat }\n        else if n == 1 { MatmulKind::MatVec }\n        else if k == 1 { MatmulKind::InnerProduct }\n        else { MatmulKind::General };\n    match kind {\n        MatmulKind::MatVec => gemv_kernel(a, b, m, n, k),\n        MatmulKind::VecMat => vecmat_kernel(a, b, m, n, k),\n        _ => general_gemm(a, b, m, n, k),\n    }\n}".to_string(),
        },
        // ── vector-width-pow2-sweep — fixed width → autotune over C ───────
        // Source: burn-cubecl reduce/tune.rs — sum_one_shot<C> with C in
        // {1,2,4,8,16,32,64} registered as separate autotune candidates. The
        // tuner picks the best vector width per (element_count, dtype) rather
        // than hardcoding one.
        KernelRule {
            rule_id: "vector-width-pow2-sweep".to_string(),
            gpu_concept: "vectorization width as an autotune dimension".to_string(),
            ref_shape: "// naive: fixed vector width regardless of element count\nfn reduce_sum(data: &[f32]) -> f32 {\n    let mut acc = 0.0;\n    for chunk in data.chunks(4) { for v in chunk { acc += v; } }\n    acc\n}".to_string(),
            opt_pattern: "// optimized: vector width C swept over pow2 values; tuner picks best per shape\nfn reduce_sum_sweep(data: &[f32]) -> f32 {\n    let best_c = [1, 2, 4, 8, 16, 32, 64]\n        .into_iter()\n        .min_by_key(|&c| bench_reduce(data, c))\n        .unwrap_or(4);\n    let mut acc = 0.0;\n    for chunk in data.chunks(best_c) { for v in chunk { acc += v; } }\n    acc\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category L: Conv-as-GEMM + memory layout (Batch 7, 5 rules — burn-cubecl)
        // ════════════════════════════════════════════════════════════════════
        // ── conv-1x1-as-matmul — im2col expansion → reshape + matmul ──────
        // Source: burn-cubecl conv/im2col.rs conv_im2col_1x1 — a 1×1 conv (kernel
        // all ones) is mathematically a matmul over [(N,H,W), C_in] × [C_in, C_out].
        // Reshaping to 2D + matmul skips the O(N*C_in) im2col column expansion.
        KernelRule {
            rule_id: "conv-1x1-as-matmul".to_string(),
            gpu_concept: "1x1 convolution as direct matmul (skip im2col)".to_string(),
            ref_shape: "// naive: im2col expansion materializes a KH*KW*C column matrix, then GEMM\nfn conv_1x1_im2col(input: &[f32], weight: &[f32], n: usize, c_in: usize, c_out: usize) -> Vec<f32> {\n    let mut col = vec![0.0f32; n * c_in];\n    unfold_im2col(input, &mut col, n, c_in); // O(n * c_in) extra memory\n    matmul(&col, weight, n, c_out, c_in)\n}".to_string(),
            opt_pattern: "// optimized: 1x1 conv IS a matmul — reshape input to [N, C_in], skip the unfold\nfn conv_1x1_matmul(input: &[f32], weight: &[f32], n: usize, c_in: usize, c_out: usize) -> Vec<f32> {\n    let weight_t = transpose(weight, c_in, c_out); // [C_in, C_out]\n    matmul(input, &weight_t, n, c_out, c_in)\n}".to_string(),
        },
        // ── contiguity-guard-before-reshape — always-copy → check-then-skip ─
        // Source: burn-cubecl conv/im2col.rs is_spatial_contiguous + reshape_input —
        // before doing a free metadata reshape [(N,H,W),C], check that channel is
        // contiguous (stride[C]==1) and spatial dims nest correctly. Only call
        // into_contiguous_pitched when the layout doesn't already support it.
        KernelRule {
            rule_id: "contiguity-guard-before-reshape".to_string(),
            gpu_concept: "stride-contiguity guard skips unnecessary relayout copy. RIDER (Batch 225 / Plan 227 / Research 255 — the predicate WIDENED from contiguity to the whole processing set, vLLM @ deedc7be #59340): a pre-allocated transform buffer was handed to the prep helper UNCONDITIONALLY ('outputs=quanted_input'), so the helper copied the input into it even when NOTHING would transform it; the fix gates the buffer HANDOFF on the processing set, verbatim: 'outputs=quanted_input if has_input_processing else None' where has_input_processing = mode.should_quantize or hadamard_block_size > 1 or activation is not None or scatter_idx is not None or num_valid_tokens is not None — with None the helper ALIASES the input and the staging copy (plus its allocation) simply does not exist. The law: a staging copy is owed only when a transform actually runs — gate the output-buffer argument on the transform predicate, not on the buffer's existence; the contiguity case is the single-transform member of this family, the MoE prep set (quant/hadamard/activation/scatter/mask) the multi-transform member. The alias-vs-copy decision must live with the PREDICATE OWNER (the caller that knows the processing set), not inside the helper (which would need to re-derive it). Batch 225 rider.".to_string(),
            ref_shape: "// naive: always copy into contiguous before any reshape (copies even when free)\nfn reshape_safe(tensor: &[f32], strides: &[usize]) -> Vec<f32> {\n    into_contiguous(tensor, strides) // always copies all elements\n}".to_string(),
            opt_pattern: "// optimized: check stride contiguity first — skip the copy if already compatible\nfn reshape_guarded(tensor: &[f32], strides: &[usize]) -> Vec<f32> {\n    if strides[strides.len() - 1] == 1 && spatial_nested(strides) {\n        tensor.to_vec() // already contiguous — free metadata reshape\n    } else {\n        into_contiguous(tensor, strides)\n    }\n}".to_string(),
        },
        // ── cube-count-bounded-batch-pack — one-at-a-time → saturate grid ──
        // Source: burn-cubecl conv/im2col.rs batches_per_run — max_simultaneous =
        // min(u16::MAX / cube_count_per_batch, batch_size), then find the largest
        // divisor of batch_size ≤ max to pack batches per launch. This saturates
        // the GPU grid without overflowing the cube-count limit.
        KernelRule {
            rule_id: "cube-count-bounded-batch-pack".to_string(),
            gpu_concept: "grid-limit-aware batch packing".to_string(),
            ref_shape: "// naive: launch one batch at a time (underutilizes grid for small batches)\nfn launch_batches(data: &[f32], batch_size: usize, per_batch: usize) {\n    for b in 0..batch_size {\n        launch_grid(&data[b * per_batch..]); // 1 batch per launch\n    }\n}".to_string(),
            opt_pattern: "// optimized: pack max batches per launch up to the cube-count limit (u16::MAX)\nfn batches_per_run(batch_size: usize, cubes_per_batch: usize) -> usize {\n    let max = Ord::min(u16::MAX as usize / cubes_per_batch, batch_size);\n    (0..=max).rev().find(|&per_run| batch_size % per_run == 0).unwrap_or(1)\n}".to_string(),
        },
        // ── stride-align-vectorization-key — fixed vec → trailing-zeros key ─
        // Source: burn-cubecl conv/forward/tune.rs stride_align — min trailing-zeros
        // of (stride × byte_size) capped at 2^10 (128-byte swizzle) defines the
        // achievable vectorization. pow2_factor(channels) = min(trailing_zeros, 4).
        // Both feed the autotune key to pick vector width.
        KernelRule {
            rule_id: "stride-align-vectorization-key".to_string(),
            gpu_concept: "stride-byte-alignment determines vectorization width".to_string(),
            ref_shape: "// naive: fixed vectorization regardless of stride alignment\nfn vec_width(strides: &[usize]) -> usize {\n    4 // hardcoded — wastes bandwidth if strides aren't 4-aligned\n}".to_string(),
            opt_pattern: "// optimized: vectorization = min trailing-zeros of (stride * bytes), capped at 2^10\nfn vec_width_aligned(strides: &[usize], bytes_per_elem: usize) -> usize {\n    let factor = strides.iter()\n        .map(|s| (s * bytes_per_elem).trailing_zeros())\n        .min()\n        .unwrap_or(10)\n        .min(10);\n    1usize << factor\n}".to_string(),
        },
        // ── anchor-shape-bucketing — exact-shape key → bucketed key ───────
        // Source: burn-cubecl conv/forward/tune.rs anchor(shape, None, None, None) —
        // spatial dims are bucketed (anchored) so similar shapes share an autotune
        // cache entry. Without anchoring, every distinct (H, W) re-tunes from
        // scratch; with it, shapes within the same bucket reuse the tuned config.
        KernelRule {
            rule_id: "anchor-shape-bucketing".to_string(),
            gpu_concept: "shape bucketing for autotune cache reuse".to_string(),
            ref_shape: "// naive: unique autotune key per exact shape (re-tunes per pixel difference)\nfn tune_key(h: usize, w: usize, c: usize) -> (usize, usize, usize) {\n    (h, w, c) // 224x224 and 225x225 are separate keys\n}".to_string(),
            opt_pattern: "// optimized: bucket spatial dims so similar shapes share a cache entry\nfn tune_key_anchored(h: usize, w: usize, c: usize) -> (usize, usize, usize) {\n    let bucket = 16; // anchor to nearest 16\n    ((h + bucket - 1) / bucket * bucket, (w + bucket - 1) / bucket * bucket, c)\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category M: Kernel parametric polymorphism (Batch 8, 3 rules — burn-cubecl)
        // ════════════════════════════════════════════════════════════════════
        // ── op-family-type-dispatch — N copy-pasted kernels → one kernel + trait ─
        // Source: burn-cubecl comparison.rs `ComparisonOpFamily` + binary.rs
        // `BinaryOpFamily` + pool/max_pool2d.rs `Pool2dDirectStrategyFamily`.
        // A single `kernel_<op>` body is parameterized over an `OpFamily` trait
        // with one associated `Operation` type per concrete op (EqualOp,
        // GreaterOp, AddOp, MulOp, MaxPoolStrategy, ...). One kernel → N op
        // impls. Distinct from `predicate-gated-backend` (gates on shape) and
        // `shape-kind-dispatch` (routes to different kernel families): here
        // the kernel BODY is identical; only the per-element op varies.
        KernelRule {
            rule_id: "op-family-type-dispatch".to_string(),
            gpu_concept: "parametric kernel polymorphism over op trait".to_string(),
            ref_shape: "// naive: copy-paste the kernel for each op (N bodies to maintain)\nfn kernel_add(a: &[f32], b: &[f32], out: &mut [f32]) {\n    for i in 0..a.len() { out[i] = a[i] + b[i]; }\n}\nfn kernel_mul(a: &[f32], b: &[f32], out: &mut [f32]) {\n    for i in 0..a.len() { out[i] = a[i] * b[i]; } // identical body, one char differs\n}".to_string(),
            opt_pattern: "// optimized: one kernel body, parameterized over an OpFamily trait\ntrait OpFamily { fn execute(lhs: f32, rhs: f32) -> f32; }\nstruct AddOp; struct MulOp;\nimpl OpFamily for AddOp { fn execute(l: f32, r: f32) -> f32 { l + r } }\nimpl OpFamily for MulOp { fn execute(l: f32, r: f32) -> f32 { l * r } }\nfn kernel<O: OpFamily>(a: &[f32], b: &[f32], out: &mut [f32]) {\n    for i in 0..a.len() { out[i] = O::execute(a[i], b[i]); } // monomorphized per op\n}".to_string(),
        },
        // ── inplace-output-alias — fresh output alloc → reuse input buffer ─────
        // Source: burn-cubecl comparison.rs `launch_cmp` + binary.rs `launch_binop`.
        // `can_mut_broadcast(&rhs)` checks whether the lhs can be mutated in place
        // (alias output → lhs handle). When true, the output reuses the lhs's
        // buffer (zero allocation + zero copy); `as_linear_view_alias(0)` marks
        // the aliasing. Falls back to a fresh `empty_device_dtype` allocation
        // when neither operand is mutably reusable. Distinct from fusion
        // (merges compute passes) — this aliases the output to an input within
        // a single pass, skipping the output allocation entirely.
        KernelRule {
            rule_id: "inplace-output-alias".to_string(),
            gpu_concept: "output buffer reuse via mutability check (skip allocation)".to_string(),
            ref_shape: "// naive: always allocate a fresh output buffer (even when an input could be reused)\nfn binop(lhs: &[f32], rhs: &[f32]) -> Vec<f32> {\n    let mut out = vec![0.0f32; lhs.len()]; // allocates every call\n    for i in 0..lhs.len() { out[i] = lhs[i] + rhs[i]; }\n    out\n}".to_string(),
            opt_pattern: "// optimized: if lhs is mutably reusable, write the result back into it (no alloc)\nfn binop_inplace(lhs: &mut [f32], rhs: &[f32], lhs_can_mut: bool) -> &mut [f32] {\n    if lhs_can_mut {\n        for i in 0..lhs.len() { lhs[i] += rhs[i]; } // alias output → lhs, zero allocation\n        lhs\n    } else {\n        unimplemented!(\"fallback: allocate fresh output\")\n    }\n}".to_string(),
        },
        // ── layout-permute-for-coalescing — NCHW compute → NHWC permute ────────
        // Source: burn-cubecl pool/max_pool2d.rs `permute_nchw_to_nhwc(x)` →
        // compute → `permute_nhwc_to_nchw(output)`. NCHW (batch, channel,
        // height, width) puts channel in dim 1 → strided channel access. NHWC
        // (batch, height, width, channel) makes channel the innermost dim →
        // consecutive lanes hit consecutive channels (coalesced). The compute
        // runs in NHWC; the result is permuted back to NCHW for the caller.
        // Distinct from `contiguity-guard-before-reshape` (skips a copy) —
        // this PERMUTES (a real relayout) to a coalescing-friendly frame.
        KernelRule {
            rule_id: "layout-permute-for-coalescing".to_string(),
            gpu_concept: "memory layout permutation for coalesced channel access".to_string(),
            ref_shape: "// naive: compute in NCHW — channel is dim 1, so per-pixel channel access is strided\nfn pool_nchw(x: &[f32], n: usize, c: usize, h: usize, w: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * c * h * w];\n    for b in 0..n { for ch in 0..c { for hw in 0..h*w {\n        // x[b][ch][hw] — stride = h*w, uncoalesced across channels\n        out[b*c*h*w + ch*h*w + hw] = pool_window(x, b, ch, hw);\n    }}}\n    out\n}".to_string(),
            opt_pattern: "// optimized: permute NCHW→NHWC (channel innermost), compute coalesced, permute back\nfn pool_nhwc(x: &[f32], n: usize, c: usize, h: usize, w: usize) -> Vec<f32> {\n    let x_nhwc = permute_nchw_to_nhwc(x, n, c, h, w); // channel now innermost\n    let mut out = vec![0.0f32; n * h * w * c];\n    for b in 0..n { for hw in 0..h*w { for ch in 0..c {\n        // x_nhwc[b][hw][ch] — stride = 1, coalesced across channels\n        out[b*h*w*c + hw*c + ch] = pool_window_nhwc(&x_nhwc, b, hw, ch);\n    }}}\n    permute_nhwc_to_nchw(&out, n, h, w, c) // restore caller's layout\n}".to_string(),
        },
        // ── reshape-rank-for-backend — leak rank contract → reshape+reshape (Batch 10, popcorn geglu.py + swiglu.py) ──
        // Source: geglu.py/swiglu.py unsloth backends — kernel expects 3-D (1, -1, hidden).
        // Adapter reshapes any-rank input to 3-D, runs kernel, reshapes back.
        // Preserves the caller's rank contract (zero-copy view when contiguous).
        KernelRule {
            rule_id: "reshape-rank-for-backend".to_string(),
            gpu_concept: "rank adapter — reshape to backend-required rank, reshape back".to_string(),
            ref_shape: "// naive: require caller to pass 3-D (leaks backend rank assumption into API)\nfn gelu_gated_3d_required(a: &[f32], b: &[f32], _batch: usize, _seq: usize, hidden: usize) -> Vec<f32> {\n    // caller MUST flatten to (1, -1, hidden) before calling\n    elementwise_gelu_gated(a, b, hidden)\n}".to_string(),
            opt_pattern: "// optimized: adapter reshapes to 3-D, runs kernel, reshapes back\nfn gelu_gated_any_rank(a: &[f32], b: &[f32], batch: usize, seq: usize, hidden: usize) -> Vec<f32> {\n    let n = batch * seq;\n    // reshape (batch, seq, hidden) → (1, n, hidden) — zero-copy view\n    let out = elementwise_gelu_gated(a, b, hidden); // kernel runs on 3-D\n    // reshape (1, n, hidden) → (batch, seq, hidden) — restore caller's rank\n    out // same buffer, caller interprets as (batch, seq, hidden)\n}".to_string(),
        },
        // ── detached-broadcast-insert — materialize broadcast → detach+view (Batch 10, popcorn rope.py) ──
        // Source: rope.py — cos.detach()[:, None] for non-differentiable broadcast.
        // Detach avoids autograd graph cost; [:, None] inserts broadcast dim (view, no alloc).
        KernelRule {
            rule_id: "detached-broadcast-insert".to_string(),
            gpu_concept: "non-differentiable broadcast via detach + inserted dim view".to_string(),
            ref_shape: "// naive: materialize the full broadcast tensor (alloc + fill)\nfn rope_materialize(q: &[f32], cos: &[f32], batch: usize, heads: usize, seq: usize, dim: usize) -> Vec<f32> {\n    let mut cos_full = vec![0.0; batch * heads * seq * dim];\n    for b in 0..batch { for h in 0..heads { for t in 0..seq { for d in 0..dim {\n        cos_full[((b * heads + h) * seq + t) * dim + d] = cos[t * dim + d];\n    }}}}\n    q.iter().zip(cos_full.iter()).map(|(a, c)| a * c).collect()\n}".to_string(),
            opt_pattern: "// optimized: detach (no autograd) + insert broadcast dim (view, no alloc)\nfn rope_broadcast_view(q: &[f32], cos: &[f32], batch: usize, heads: usize, seq: usize, dim: usize) -> Vec<f32> {\n    // cos is treated as (1, 1, seq, dim) — broadcasts against (batch, heads, seq, dim)\n    // no autograd graph for cos (non-differentiable position embedding)\n    let mut out = vec![0.0; batch * heads * seq * dim];\n    for b in 0..batch { for h in 0..heads { for t in 0..seq { for d in 0..dim {\n        out[((b * heads + h) * seq + t) * dim + d] = q[((b * heads + h) * seq + t) * dim + d] * cos[t * dim + d];\n    }}}}\n    out\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category N: dtype + alignment specialization (8 rules — 2 Batch 8 + 4 Batch 9 + 2 Batch 10)
        // ════════════════════════════════════════════════════════════════════
        // ── dtype-specialized-float-kind — generic fn → FloatKind-keyed intrinsic ─
        // Source: burn-cubecl binary.rs `PowOp::execute`. `powf` has no generic
        // implementation — f16/bf16/f64 each need their own `__expand_powf`.
        // The op matches on `ir::FloatKind` (F16/BF16/F64) to cast to the
        // concrete type, call the dtype-specific intrinsic, and cast back.
        // F32 is the fallback. The generic path is wrong (or absent) for
        // low-precision floats; the dtype-specialized path is correct.
        KernelRule {
            rule_id: "dtype-specialized-float-kind".to_string(),
            gpu_concept: "dtype-keyed intrinsic dispatch for low-precision floats".to_string(),
            ref_shape: "// naive: one generic powf — wrong/absent for f16/bf16 (no f32-precision powf)\nfn powf_generic<T: Float>(lhs: T, rhs: T) -> T {\n    lhs.powf(rhs) // f16::powf is absent or falls back to f32 (precision loss)\n}".to_string(),
            opt_pattern: "// optimized: match on FloatKind, cast to the concrete dtype, call its intrinsic\n#[derive(PartialEq)] enum FloatKind { F16, Bf16, F32, F64 }\nfn powf_specialized(kind: FloatKind, lhs: &[f32], rhs: &[f32], out: &mut [f32]) {\n    match kind {\n        FloatKind::F16 => { // cast to f16, use f16 powf, cast back\n            for i in 0..out.len() {\n                let l = f16::from_f32(lhs[i]); let r = f16::from_f32(rhs[i]);\n                out[i] = l.powf(r).to_f32();\n            }\n        }\n        FloatKind::F32 => { for i in 0..out.len() { out[i] = lhs[i].powf(rhs[i]); } }\n        // ... F64, Bf16 ...\n    }\n}".to_string(),
        },
        // ── alignment-gated-zero-copy-view — always-copy slice → offset-only view ─
        // Source: burn-cubecl index/slice.rs. When the slice's byte offsets
        // (start, end) are both multiples of the device's memory-alignment
        // boundary, the slice is a pure VIEW: adjust the handle's start/end
        // offsets + metadata (shape, strides), share the underlying buffer,
        // zero copy. When unaligned, fall back to a copy kernel. Distinct
        // from `contiguity-guard-before-reshape` (which checks stride
        // contiguity for reshape and still materializes via to_vec): this
        // checks BYTE-OFFSET alignment for slice and shares the buffer with
        // NO data movement at all.
        KernelRule {
            rule_id: "alignment-gated-zero-copy-view".to_string(),
            gpu_concept: "alignment-gated zero-copy slice view via handle offset".to_string(),
            ref_shape: "// naive: always copy the sliced region into a fresh buffer\nfn slice_copy(buf: &[f32], start: usize, end: usize) -> Vec<f32> {\n    buf[start..end].to_vec() // copies every slice, even when a view would suffice\n}".to_string(),
            opt_pattern: "// optimized: if byte offsets are alignment-aligned, return a VIEW (offset only, zero copy)\nstruct Handle { data: Vec<f32>, offset_start: usize, offset_end: usize }\nfn slice_view(h: &Handle, start: usize, end: usize, align: usize) -> Handle {\n    let byte_start = start * std::mem::size_of::<f32>();\n    let byte_end = (h.data.len() - end) * std::mem::size_of::<f32>();\n    if byte_start % align == 0 && byte_end % align == 0 {\n        Handle { data: h.data.clone(), offset_start: h.offset_start + byte_start, offset_end: h.offset_end + byte_end } // zero copy\n    } else {\n        unimplemented!(\"fallback: copy kernel for unaligned offsets\")\n    }\n}".to_string(),
        },
        // ── int2-pack-weights — 1 weight/byte → 4 weights/byte (Batch 9, popcorn int8_int2_matmul.py) ──
        // Source: int8_int2_matmul.py liger pack_weights — packs 4 ternary {-1,0,+1}
        // weights per byte (2 bits each). 4× memory bandwidth reduction vs int8 storage.
        // Directly relevant to katgpt-core's ternary inference (same {-1,0,+1} space).
        KernelRule {
            rule_id: "int2-pack-weights".to_string(),
            gpu_concept: "ternary weight packing: 4 weights per byte via 2-bit encoding".to_string(),
            ref_shape: "// naive: store each ternary weight as int8 (8 bits per weight, 4× wastage)\nfn matmul_int8_weights(a: &[i8], w: &[i8]) -> Vec<i32> {\n    let mut out = vec![0i32; a.len() / w.len()];\n    for i in 0..out.len() {\n        for j in 0..w.len() {\n            out[i] += (a[i * w.len() + j] as i32) * (w[j] as i32);\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: pack 4 ternary weights per byte (2 bits each), 4× bandwidth reduction\nfn pack_ternary(w: &[i8]) -> Vec<u8> {\n    let mut packed = vec![0u8; (w.len() + 3) / 4];\n    for (i, &val) in w.iter().enumerate() {\n        let code: u8 = match val { -1 => 0b00, 0 => 0b01, 1 => 0b10, _ => 0b01 };\n        packed[i / 4] |= code << ((i % 4) * 2);\n    }\n    packed\n}\nfn matmul_packed(a: &[i8], w_packed: &[u8]) -> Vec<i32> {\n    let n = w_packed.len() * 4;\n    let mut out = vec![0i32; a.len() / n];\n    for i in 0..out.len() {\n        for j in 0..n {\n            let byte = w_packed[j / 4];\n            let code = (byte >> ((j % 4) * 2)) & 0b11;\n            let w_val: i8 = match code { 0b00 => -1, 0b01 => 0, 0b10 => 1, _ => 0 };\n            out[i] += (a[i * n + j] as i32) * (w_val as i32);\n        }\n    }\n    out\n}".to_string(),
        },
        // ── per-token-activation-quant — per-element → per-token symmetric int8 (Batch 9, popcorn _quant.py) ──
        // Source: _quant.py activation_quant — per-token symmetric int8 fake quant.
        // scale = 127 / max(|x|), then round + clamp. Straight-through gradient in training.
        KernelRule {
            rule_id: "per-token-activation-quant".to_string(),
            gpu_concept: "per-token symmetric int8 quantization with straight-through gradient".to_string(),
            ref_shape: "// naive: per-element quant (no scaling, severe clipping)\nfn quant_per_element(x: &[f32]) -> Vec<i8> {\n    x.iter().map(|v| (*v * 127.0) as i8).collect()\n}".to_string(),
            opt_pattern: "// optimized: per-token symmetric quant (scale = 127 / max(|x|), minimal clipping)\nfn activation_quant_per_token(x: &[f32]) -> Vec<i8> {\n    let max_abs: f32 = x.iter().map(|v| v.abs()).fold(0.0, f32::max).max(1e-5);\n    let scale = 127.0 / max_abs;\n    x.iter().map(|v| (v * scale).round().clamp(-128.0, 127.0) as i8).collect()\n}".to_string(),
        },
        // ── ternary-weight-quant — per-element → per-tensor ternary {-1,0,+1} (Batch 9, popcorn _quant.py) ──
        // Source: _quant.py weight_quant — per-tensor ternary quant.
        // scale = 1 / mean(|w|), then round + clamp(-1, 1). Produces BitNet b1.58 weights.
        KernelRule {
            rule_id: "ternary-weight-quant".to_string(),
            gpu_concept: "per-tensor ternary weight quantization {-1, 0, +1} with straight-through gradient".to_string(),
            ref_shape: "// naive: per-element ternary (no scaling, most weights collapse to 0)\nfn ternary_per_element(w: &[f32]) -> Vec<i8> {\n    w.iter().map(|v| if *v > 0.5 { 1 } else if *v < -0.5 { -1 } else { 0 }).collect()\n}".to_string(),
            opt_pattern: "// optimized: per-tensor ternary (scale = 1/mean(|w|), minimal information loss)\nfn weight_quant_ternary(w: &[f32]) -> Vec<i8> {\n    let mean_abs: f32 = w.iter().map(|v| v.abs()).sum::<f32>() / w.len() as f32;\n    let scale = 1.0 / mean_abs.max(1e-5);\n    w.iter().map(|v| (v * scale).round().clamp(-1.0, 1.0) as i8).collect()\n}".to_string(),
        },
        // ── upcast-before-loss — fp16 loss → f32 upcast before log/exp (Batch 9, popcorn jsd.py) ──
        // Source: jsd.py liger backend — upcast(log_p) before the JSD kernel.
        // fp16 has insufficient precision for log domain: log(small) underflows to -inf.
        KernelRule {
            rule_id: "upcast-before-loss".to_string(),
            gpu_concept: "upcast to f32 before log/exp in loss computation to avoid fp16 precision loss".to_string(),
            ref_shape: "// naive: compute loss in fp16 (log underflows for small probabilities)\nfn jsd_fp16(log_p: &[f16], log_q: &[f16]) -> f16 {\n    let p: f16 = log_p.iter().map(|l| l.exp()).sum();\n    let q: f16 = log_q.iter().map(|l| l.exp()).sum();\n    (p * q).ln()\n}".to_string(),
            opt_pattern: "// optimized: upcast to f32 before log/exp, downcast after\nfn jsd_upcast(log_p: &[f16], log_q: &[f16]) -> f16 {\n    let p: f32 = log_p.iter().map(|l| l.to_f32().exp()).sum();\n    let q: f32 = log_q.iter().map(|l| l.to_f32().exp()).sum();\n    f16::from_f32((p * q).ln())\n}".to_string(),
        },
        // ── batched-upcast-map — per-tensor upcast → map over tuple (Batch 10, popcorn comba.py) ──
        // Source: comba.py — q32, k32, v32, p32, g32, beta32 = map(upcast, (q, k, v, p, g, beta)).
        // One-line batch upcast signals independence to the compiler (parallelizable).
        KernelRule {
            rule_id: "batched-upcast-map".to_string(),
            gpu_concept: "batched type promotion via map over a tuple".to_string(),
            ref_shape: "// naive: per-tensor upcast calls (six separate bindings, six separate calls)\nfn upcast_six_separate(q: &[f16], k: &[f16], v: &[f16], p: &[f16], g: &[f16], beta: &[f16]) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {\n    let q32: Vec<f32> = q.iter().map(|x| x.to_f32()).collect();\n    let k32: Vec<f32> = k.iter().map(|x| x.to_f32()).collect();\n    let v32: Vec<f32> = v.iter().map(|x| x.to_f32()).collect();\n    let p32: Vec<f32> = p.iter().map(|x| x.to_f32()).collect();\n    let g32: Vec<f32> = g.iter().map(|x| x.to_f32()).collect();\n    let beta32: Vec<f32> = beta.iter().map(|x| x.to_f32()).collect();\n    (q32, k32, v32, p32, g32, beta32)\n}".to_string(),
            opt_pattern: "// optimized: map over tuple (signals independence, one structured binding)\nfn upcast_six_mapped(q: &[f16], k: &[f16], v: &[f16], p: &[f16], g: &[f16], beta: &[f16]) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {\n    let upcast = |t: &[f16]| t.iter().map(|x| x.to_f32()).collect::<Vec<f32>>();\n    let (q32, k32, v32, p32, g32, beta32) = (upcast(q), upcast(k), upcast(v), upcast(p), upcast(g), upcast(beta));\n    (q32, k32, v32, p32, g32, beta32)\n}".to_string(),
        },
        // ── dtype-restricted-backend — all-dtype backend → precision-safe dtype subset (Batch 10, popcorn gelu.py + sparsemax.py) ──
        // Source: gelu.py fla (Float32|BFloat16, fp16 tanh grads past tol),
        // sparsemax.py liger (Float32|Float16, bf16 ties break support set).
        // Encode the dtype restriction in the backend signature, not a runtime check.
        KernelRule {
            rule_id: "dtype-restricted-backend".to_string(),
            gpu_concept: "backend restricted to precision-stable dtypes via signature".to_string(),
            ref_shape: "// naive: register fast backend for ALL dtypes (silently-wrong gradients on fp16/bf16)\nfn gelu_fast_all_dtypes(x: &[f32], dtype: u8 /* 0=f32, 1=f16, 2=bf16 */) -> Vec<f32> {\n    // fast path runs regardless of dtype — tanh-approx gradients miscompile on fp16\n    fast_tanh_gelu(x)\n}".to_string(),
            opt_pattern: "// optimized: dispatch on dtype — fast path only for precision-stable dtypes\nfn gelu_dtype_dispatched(x: &[f32], dtype: u8) -> Vec<f32> {\n    match dtype {\n        0 /* f32 */ | 2 /* bf16 */ => fast_tanh_gelu(x), // precision-stable\n        1 /* f16 */ => reference_gelu(x), // fast path would miscompile\n        _ => reference_gelu(x),\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Category O: IR strength reduction (Batch 8, 4 rules — cubecl-opt)
        // ════════════════════════════════════════════════════════════════════
        // Source: cubecl/crates/cubecl-opt/src/passes/reduce_strength.rs — the
        // compiler IR pass that replaces expensive arithmetic on u32 constants
        // with cheaper bitwise ops. These apply at the kernel-source level too:
        // a heal rule can suggest the shift form in source, and the compiler
        // (or cubecl-opt) will confirm it. All four are guarded on u32
        // element type (the IR pass skips non-uint types — float shifts are UB).
        // ── strength-reduce-mul-pow2 — x * 2^k → x << k ─────────────────────
        // Exact power-of-two multiplier. One instruction (shift left) replaces
        // one instruction (mul), but the shift is on the ALU's faster port and
        // frees the multiply unit. The IR pass checks `val.is_power_of_two()`.
        KernelRule {
            rule_id: "strength-reduce-mul-pow2".to_string(),
            gpu_concept: "IR strength reduction: multiply by power of 2 → left shift".to_string(),
            ref_shape: "// naive: integer multiply by a power-of-2 constant (uses the mul unit)\nfn scale(x: u32) -> u32 {\n    x * 16 // mul instruction\n}".to_string(),
            opt_pattern: "// optimized: left shift by the constant's log2 (uses the shift unit, frees mul)\nfn scale(x: u32) -> u32 {\n    x << 4 // 16 = 2^4, trailing_zeros(16) = 4\n}".to_string(),
        },
        // ── strength-reduce-mul-pow2-plus-minus-one — x*(2^k±1) → (x<<k)±x ──
        // Source: reduce_strength.rs `(val + 1).is_power_of_two()` and
        // `(val - 1).is_power_of_two()` arms. A multiplier one away from a
        // power of two (15, 17, 31, 33, ...) becomes shift + add/sub. This is
        // the classic GPU LEA-style strength reduction: 2 instructions
        // (shift + add) replace a multiply, and on GPUs the integer multiply
        // unit is often throughput-limited. A temp local holds the shift result.
        KernelRule {
            rule_id: "strength-reduce-mul-pow2-plus-minus-one".to_string(),
            gpu_concept: "IR strength reduction: multiply by (2^k ± 1) → shift ± self".to_string(),
            ref_shape: "// naive: integer multiply by a near-power-of-2 constant (15, 17, 31, 33, ...)\nfn scale(x: u32) -> u32 {\n    x * 15 // mul instruction — 15 = 16 - 1 = 2^4 - 1\n}".to_string(),
            opt_pattern: "// optimized: shift then subtract self (2 instructions, no mul unit)\nfn scale(x: u32) -> u32 {\n    let shifted = x << 4; // x * 16\n    shifted - x           // x * 16 - x = x * 15\n}".to_string(),
        },
        // ── strength-reduce-div-pow2 — x / 2^k → x >> k ─────────────────────
        // Integer division is one of the slowest ALU ops (tens of cycles on
        // most GPUs). For power-of-2 divisors, a right shift is exact and
        // ~10-20x faster. The IR pass guards on `is_pow2(op.rhs)`. Note:
        // this is EXACT for unsigned division (no rounding-mode subtlety).
        KernelRule {
            rule_id: "strength-reduce-div-pow2".to_string(),
            gpu_concept: "IR strength reduction: divide by power of 2 → right shift".to_string(),
            ref_shape: "// naive: integer divide by a power-of-2 constant (slow div unit, tens of cycles)\nfn downscale(x: u32) -> u32 {\n    x / 8 // div instruction\n}".to_string(),
            opt_pattern: "// optimized: logical right shift by the constant's log2 (1 cycle, exact for unsigned)\nfn downscale(x: u32) -> u32 {\n    x >> 3 // 8 = 2^3, trailing_zeros(8) = 3\n}".to_string(),
        },
        // ── strength-reduce-mod-pow2 — x % 2^k → x & (2^k - 1) ──────────────
        // Modulo by a power of 2 is a bitmask in disguise. The div/mod unit
        // computes both quotient and remainder; replacing `% 2^k` with
        // `& (2^k - 1)` uses the cheaper bitwise-and unit. The IR pass emits
        // `BitwiseAnd(lhs, const_val - 1)`. Exact for unsigned operands.
        KernelRule {
            rule_id: "strength-reduce-mod-pow2".to_string(),
            gpu_concept: "IR strength reduction: modulo by power of 2 → bitwise and mask".to_string(),
            ref_shape: "// naive: integer modulo by a power-of-2 constant (uses the div/mod unit)\nfn wrap(x: u32) -> u32 {\n    x % 64 // mod instruction\n}".to_string(),
            opt_pattern: "// optimized: bitwise and with (2^k - 1) mask (cheaper bitwise-and unit)\nfn wrap(x: u32) -> u32 {\n    x & 63 // 64 = 2^6, mask = 64 - 1 = 0b111111\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 11 — popcorn linear-attention recurrence family (13 rules)
        // Source: popcorn/src/popcorn/kernels/{gla,simple_gla,based,rebased,
        // linear_attn,retention,lightning_attn,rwkv7,delta_rule,iplr_delta_rule,
        // gated_oja_rule,deltaformer,hgrn,chunk_global_cumsum,chunk_local_cumsum}.py
        // Grows G (linear-attn recurrence) from 6 → 11, E (parallelism) 3 → 5,
        // F (flash-attn tiling) 7 → 8, B (compute) 9 → 11, D (dispatch) 5 → 6,
        // J (varlen + chunked reduction) 5 → 6.
        // ════════════════════════════════════════════════════════════════════
        // ── per-key-dim-diagonal-gate — scalar gate → per-dimension vector gate ─
        // Source: gla.py — `state * gate[:, t].exp()[..., None]`. The gate is a
        // per-key-dimension vector (not a scalar per head), broadcast across the
        // value dimension via `[..., None]`. Each key dimension decays at its
        // own learned rate. Distinct from `state-forget-gate` (Batch 3) which is
        // a single scalar decay applied uniformly.
        KernelRule {
            rule_id: "per-key-dim-diagonal-gate".to_string(),
            gpu_concept: "per-dimension diagonal decay gate on recurrent state".to_string(),
            ref_shape: "// naive: single scalar decay for the whole state vector (uniform forgetting)\nfn state_update_scalar_gate(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], gate: f32) {\n    let decay = gate.exp();\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] = state[kv][vv] * decay + k[kv] * v[vv];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: per-key-dimension vector decay (gla) — each dim forgets independently\nfn state_update_per_dim_gate(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], gate: &[f32; K]) {\n    // gate[kv].exp() broadcasts across V via the inner loop (the [..., None] in py)\n    let mut decay = [0.0f32; K];\n    for kv in 0..K { decay[kv] = gate[kv].exp(); }\n    for kv in 0..K {\n        let d_kv = decay[kv];\n        for vv in 0..V {\n            state[kv][vv] = state[kv][vv] * d_kv + k[kv] * v[vv];\n        }\n    }\n}".to_string(),
        },
        // ── erase-replace-low-rank-transition — 2-phase update → 4-phase D+LR ──
        // Source: rwkv7.py — `removed = state·a; state = state*w.exp() + b⊗removed;
        // state += k⊗v`. The diagonal decay (w) plus the low-rank erase-replace
        // (a, b) compose a 4-phase transition: project out an erase direction,
        // decay, add it back along a replace direction, then write the new outer
        // product. Structurally distinct from gla's 2-phase (decay + write).
        KernelRule {
            rule_id: "erase-replace-low-rank-transition".to_string(),
            gpu_concept: "diagonal-plus-low-rank state transition (erase, decay, replace, write)".to_string(),
            ref_shape: "// naive: 2-phase update (decay + write) — no in-context erase/replace\nfn state_update_two_phase(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], w: &[f32; K]) {\n    for kv in 0..K {\n        let d = w[kv].exp();\n        for vv in 0..V {\n            state[kv][vv] = state[kv][vv] * d + k[kv] * v[vv];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: 4-phase diagonal+low-rank transition (rwkv7 Goose)\nfn state_update_dplr(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], w: &[f32; K], a: &[f32; K], b: &[f32; K]) {\n    // phase 1: project out the erase direction a → removed[bhv] = Σ_kv state[kv][hv]*a[kv]\n    let mut removed = [0.0f32; V];\n    for vv in 0..V {\n        let mut acc = 0.0;\n        for kv in 0..K { acc += state[kv][vv] * a[kv]; }\n        removed[vv] = acc;\n    }\n    // phase 2 + 3: decay (diagonal w) and add back along replace direction b\n    for kv in 0..K {\n        let d = w[kv].exp();\n        for vv in 0..V {\n            state[kv][vv] = state[kv][vv] * d + b[kv] * removed[vv];\n        }\n    }\n    // phase 4: write the new key-value outer product\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] += k[kv] * v[vv];\n        }\n    }\n}".to_string(),
        },
        // ── oja-subtractive-normalization — raw write → project-out-then-write ──
        // Source: gated_oja_rule.py — `write = β(k − Sv)` where `Sv` is the
        // projection of state onto the current value. Oja's rule subtracts the
        // already-stored component before writing, keeping the state bounded.
        // Distinct from householder-rank1-update which projects `v − Sk` (the
        // value residual, not the key residual).
        KernelRule {
            rule_id: "oja-subtractive-normalization".to_string(),
            gpu_concept: "Oja subtractive normalization: project out stored component before write".to_string(),
            ref_shape: "// naive: raw outer-product write (state grows unbounded without normalization)\nfn state_write_raw(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] += beta * k[kv] * v[vv];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: Oja subtractive — write only the NEW component (k − Sv)\nfn state_write_oja(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], beta: f32) {\n    // step 1: project state onto v → stored_k[kv] = Σ_vv state[kv][vv]*v[vv]\n    let mut stored_k = [0.0f32; K];\n    for kv in 0..K {\n        let mut acc = 0.0;\n        for vv in 0..V { acc += state[kv][vv] * v[vv]; }\n        stored_k[kv] = acc;\n    }\n    // step 2: write the residual (k − stored_k), not the raw k\n    for kv in 0..K {\n        let residual_k = beta * (k[kv] - stored_k[kv]);\n        for vv in 0..V {\n            state[kv][vv] += residual_k * v[vv];\n        }\n    }\n}".to_string(),
        },
        // ── identity-plus-low-rank-delta — decay-gated update → I+ba^T rotation ──
        // Source: iplr_delta_rule.py — `state += b⊗(state·a)` then `state += k⊗v`.
        // The rank-1 matrix `(I + b·a^T)` is an identity-plus-low-rank update:
        // it rotates the state along the (a,b) plane without decaying it. No
        // forget gate — the update is additive-rotational, preserving state
        // magnitude (bounded by the input stream rather than by a gate).
        KernelRule {
            rule_id: "identity-plus-low-rank-delta".to_string(),
            gpu_concept: "identity-plus-low-rank (I+ba^T) state rotation then outer-product write".to_string(),
            ref_shape: "// naive: decay-gated update (state shrinks each step)\nfn state_update_decayed(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], decay: f32) {\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] = state[kv][vv] * decay + k[kv] * v[vv];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: (I + b·a^T) rotation preserves magnitude, then write outer product\nfn state_update_iplr(state: &mut [[f32; V]; K], k: &[f32; K], v: &[f32; V], a: &[f32; K], b: &[f32; K]) {\n    // phase 1: project state onto a → projected[vv] = Σ_kv state[kv][vv]*a[kv]\n    let mut projected = [0.0f32; V];\n    for vv in 0..V {\n        let mut acc = 0.0;\n        for kv in 0..K { acc += state[kv][vv] * a[kv]; }\n        projected[vv] = acc;\n    }\n    // phase 2: rank-1 rotation along (a,b): state += b ⊗ projected (no decay)\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] += b[kv] * projected[vv];\n        }\n    }\n    // phase 3: write the new key-value outer product\n    for kv in 0..K {\n        for vv in 0..V {\n            state[kv][vv] += k[kv] * v[vv];\n        }\n    }\n}".to_string(),
        },
        // ── cumsum-gate-to-decay-matrix — per-step gate → pairwise decay matrix ─
        // Source: simple_gla.py — `decay = g.cumsum(1); decay = (decay[:,None] −
        // decay[None,:]).tril().exp().tril()`. The per-step log-gate is cumsummed
        // (telescoping sum), differenced pairwise to get the cumulative decay
        // between positions i and j, exp'd, then masked causal. Distinct from
        // retention/lightning which use a constant per-head slope.
        KernelRule {
            rule_id: "cumsum-gate-to-decay-matrix".to_string(),
            gpu_concept: "convert per-step log-gate into pairwise decay matrix via cumsum + diff".to_string(),
            ref_shape: "// naive: per-step sequential decay (O(N) sequential steps, no pairwise form)\nfn decay_sequential(state: &mut [f32], x: &[f32], gate: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for t in 0..n {\n        let d = gate[t].exp();\n        state[t % state.len()] = state[t % state.len()] * d + x[t];\n        out[t] = state[t % state.len()];\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: build pairwise decay matrix from cumsummed gate (simple_gla)\nfn decay_pairwise_matrix(gate: &[f32], n: usize) -> Vec<f32> {\n    // cumsum the log-gate (telescoping)\n    let mut cumgate = vec![0.0f32; n];\n    let mut acc = 0.0;\n    for i in 0..n { acc += gate[i]; cumgate[i] = acc; }\n    // pairwise difference: decay[i][j] = exp(cumgate[i] - cumgate[j]) for i >= j\n    let mut decay = vec![0.0f32; n * n];\n    for i in 0..n {\n        for j in 0..=i {\n            decay[i * n + j] = (cumgate[i] - cumgate[j]).exp();\n        }\n    }\n    decay\n}".to_string(),
        },
        // ── reverse-scan-via-double-flip — reverse cumsum → flip+cumsum+flip ───
        // Source: chunk_global_cumsum.py, chunk_local_cumsum.py — `torch.flip(
        // torch.cumsum(torch.flip(s, (axis,)), axis), (axis,))`. The canonical
        // idiom for a reverse-direction cumulative scan when no native
        // `cumsum_reverse` exists: flip the input, run the forward scan, flip
        // the output. The two flips compose to a reverse scan.
        KernelRule {
            rule_id: "reverse-scan-via-double-flip".to_string(),
            gpu_concept: "reverse-direction scan via flip → forward scan → flip".to_string(),
            ref_shape: "// naive: explicit reverse-direction loop (no vectorization, sequential)\nfn cumsum_reverse_naive(x: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    let mut acc = 0.0;\n    for i in (0..n).rev() {\n        acc += x[i];\n        out[i] = acc;\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: flip → forward cumsum → flip (reuses the vectorized forward scan)\nfn cumsum_reverse_flip(x: &[f32], n: usize) -> Vec<f32> {\n    let mut flipped = vec![0.0f32; n];\n    for i in 0..n { flipped[i] = x[n - 1 - i]; }\n    let mut scanned = vec![0.0f32; n];\n    let mut acc = 0.0;\n    for i in 0..n { acc += flipped[i]; scanned[i] = acc; }\n    let mut out = vec![0.0f32; n];\n    for i in 0..n { out[i] = scanned[n - 1 - i]; }\n    out\n}".to_string(),
        },
        // ── pairwise-distance-decay-matrix — per-step decay → N×N decay matrix ─
        // Source: retention.py, lightning_attn.py — `n = arange(seq); steps =
        // n[:,None] − n[None,:]; decay = (steps*slope).exp()`. Broadcasts a
        // per-step (or per-head) decay slope into a full pairwise decay matrix
        // for a single batched matmul, avoiding the sequential recurrence.
        KernelRule {
            rule_id: "pairwise-distance-decay-matrix".to_string(),
            gpu_concept: "materialize N×N decay matrix from index distance and slope".to_string(),
            ref_shape: "// naive: sequential recurrence with per-step decay (O(N) sequential steps)\nfn decay_sequential_step(state: &mut [f32], x: &[f32], slope: f32, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    let decay = slope.exp();\n    for t in 0..n {\n        state[t % state.len()] = state[t % state.len()] * decay + x[t];\n        out[t] = state[t % state.len()];\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: build N×N decay matrix from arange distance, then batched matmul\nfn decay_matrix_batched(x: &[f32], slope: f32, n: usize) -> Vec<f32> {\n    // steps[i][j] = i - j (pairwise distance), decay[i][j] = exp(slope * (i-j)) for i>=j\n    let mut decay = vec![0.0f32; n * n];\n    for i in 0..n {\n        for j in 0..=i {\n            decay[i * n + j] = (slope * (i as f32 - j as f32)).exp();\n        }\n    }\n    // out[i] = Σ_{j<=i} decay[i][j] * x[j] — one batched matmul, fully parallel\n    let mut out = vec![0.0f32; n];\n    for i in 0..n {\n        let mut acc = 0.0;\n        for j in 0..=i { acc += decay[i * n + j] * x[j]; }\n        out[i] = acc;\n    }\n    out\n}".to_string(),
        },
        // ── chunk-reset-scan REMOVED — duplicate of `chunk-reset-cumsum` (Batch 6).
        // Both source from chunk_local_cumsum.py; the existing rule covers the
        // split→cumsum→cat chunk-boundary-reset pattern. See Research 012.
        // ── strict-causal-mask-tril-offset — tril(0) → tril(-1) exclude diag ───
        // Source: deltaformer.py — `scores.masked_fill(~lower.tril(-1), -inf)`.
        // Strictly-causal attention excludes the diagonal (position cannot attend
        // to itself) via `tril(-1)` instead of the standard `tril()`. Paired
        // with a NaN guard: the first row has no visible keys → softmax of all
        // -inf → NaN, requiring `nan_to_num` before downstream use.
        KernelRule {
            rule_id: "strict-causal-mask-tril-offset".to_string(),
            gpu_concept: "strictly-causal mask via tril(-1) (exclude diagonal) + NaN guard".to_string(),
            ref_shape: "// naive: standard causal mask tril(0) — position attends to itself (diag included)\nfn attention_causal_diag(scores: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * n];\n    for i in 0..n {\n        for j in 0..n {\n            out[i * n + j] = if j <= i { scores[i * n + j] } else { f32::NEG_INFINITY };\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: strictly-causal tril(-1) (diag excluded) + NaN guard for empty first row\nfn attention_strict_causal(scores: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![f32::NEG_INFINITY; n * n];\n    for i in 0..n {\n        for j in 0..n {\n            // tril(-1): j < i (strictly below the diagonal)\n            if j < i { out[i * n + j] = scores[i * n + j]; }\n        }\n    }\n    // first row is all -inf (no visible keys) → softmax would be NaN; guard it\n    for j in 0..n { out[0 * n + j] = 0.0; } // nan_to_num on empty first row\n    out\n}".to_string(),
        },
        // ── polynomial-taylor-feature-map — softmax kernel → poly feature map ─
        // Source: based.py (`1 + x + 0.5·x²`, 2nd-order Taylor), rebased.py
        // (`x²`, pure quadratic). Approximates the softmax kernel with a
        // low-degree polynomial so linear attention can run without exp. The
        // Taylor expansion of `exp(q·k)` truncated at degree 2 is the Based
        // kernel; dropping the constant + linear terms gives ReBased.
        KernelRule {
            rule_id: "polynomial-taylor-feature-map".to_string(),
            gpu_concept: "polynomial Taylor feature map approximating the softmax kernel".to_string(),
            ref_shape: "// naive: full softmax attention (exp over all scores, O(N²) + expensive exp)\nfn attention_softmax(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut scores = vec![0.0f32; n];\n        let mut max_s = f32::NEG_INFINITY;\n        for j in 0..n {\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * k[j*d+dd]; }\n            scores[j] = s; max_s = max_s.max(s);\n        }\n        let mut sum_e = 0.0;\n        for j in 0..n { scores[j] = (scores[j] - max_s).exp(); sum_e += scores[j]; }\n        for j in 0..n {\n            for dd in 0..d { out[i*d+dd] += scores[j] / sum_e * v[j*d+dd]; }\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: polynomial feature map (no exp) — Based kernel: 1 + x + 0.5·x²\nfn attention_poly_feature(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut norm = 0.0; // running normalizer Σ a_ts\n        for j in 0..=i { // causal\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * k[j*d+dd]; }\n            let a = 1.0 + s + 0.5 * s * s; // 2nd-order Taylor of exp(s)\n            for dd in 0..d { out[i*d+dd] += a * v[j*d+dd]; }\n            norm += a;\n        }\n        for dd in 0..d { out[i*d+dd] /= norm + 1e-6; } // normalize\n    }\n    out\n}".to_string(),
        },
        // ── positive-feature-map-shift — raw q/k → elu+1 non-negative shift ───
        // Source: linear_attn.py — q and k pass through `elu(x) + 1` so the
        // normalizer `Σ k` never crosses zero. Without the shift, signed q/k
        // can produce a zero or negative normalizer, destabilizing the linear
        // attention recurrence. The +1 is the minimal shift making elu ≥ 0
        // strictly positive.
        KernelRule {
            rule_id: "positive-feature-map-shift".to_string(),
            gpu_concept: "shift activation positive (elu+1) so linear-attention normalizer never crosses zero".to_string(),
            ref_shape: "// naive: raw q/k (signed) — normalizer Σ k can be zero/negative, destabilizing\nfn linear_attn_raw(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut norm = 0.0;\n        for j in 0..=i {\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * k[j*d+dd]; }\n            for dd in 0..d { out[i*d+dd] += s * v[j*d+dd]; }\n            norm += s; // can be zero/negative\n        }\n        for dd in 0..d { out[i*d+dd] /= norm + 1e-10; }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: elu(x)+1 feature map guarantees non-negative q/k → stable normalizer\nfn linear_attn_positive(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let map = |x: f32| -> f32 { if x > 0.0 { x + 1.0 } else { x.exp() + 1.0 } }; // elu+1\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut norm = 0.0;\n        for j in 0..=i {\n            let mut s = 0.0;\n            for dd in 0..d { s += map(q[i*d+dd]) * map(k[j*d+dd]); }\n            for dd in 0..d { out[i*d+dd] += s * v[j*d+dd]; }\n            norm += s; // strictly positive (map ≥ 1)\n        }\n        for dd in 0..d { out[i*d+dd] /= norm + 1e-10; }\n    }\n    out\n}".to_string(),
        },
        // ── layer-depth-decay-schedule — constant slope → depth-varying slope ──
        // Source: lightning_attn.py — `slope = -(8/heads * (1 - layer_idx/
        // num_layers)) * arange(heads)`. The decay slope scales with layer
        // depth: deeper layers (layer_idx → num_layers) decay slower
        // (slope → 0), shallower layers decay faster. A dispatch decision
        // baked into kernel parameters via the layer index — not a runtime
        // branch but a parameter specialization.
        KernelRule {
            rule_id: "layer-depth-decay-schedule".to_string(),
            gpu_concept: "layer-depth-dependent decay schedule (deeper layers decay slower)".to_string(),
            ref_shape: "// naive: constant per-head decay slope regardless of layer depth\nfn decay_slope_constant(heads: usize, layer_idx: usize, num_layers: usize) -> Vec<f32> {\n    // same schedule at every layer — ignores depth\n    let _ = (layer_idx, num_layers);\n    (0..heads).map(|h| -8.0 / heads as f32 * h as f32).collect()\n}".to_string(),
            opt_pattern: "// optimized: slope scales with depth (lightning_attn) — deeper → slower decay\nfn decay_slope_depth_aware(heads: usize, layer_idx: usize, num_layers: usize) -> Vec<f32> {\n    let depth_factor = 1.0 - layer_idx as f32 / num_layers as f32; // 1.0 at layer 0 → 0.0 at last\n    (0..heads)\n        .map(|h| -(8.0 / heads as f32) * depth_factor * h as f32)\n        .collect()\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 12 (Research 013): 14 rules — gated delta + specialized recurrence + advanced attention
        // ════════════════════════════════════════════════════════════════════
        // ── gate-then-correct-delta-rule — decay THEN correct against decayed state ──
        // Source: gated_delta_rule.py — `state = state * g.exp()[..., None, None];
        // error = v - state·k; state += k ⊗ (β·error)`. The ordering is load-bearing:
        // the gate applies to the state BEFORE the delta correction, so the
        // Householder-style projection `v - S·k` targets the DECAYED subspace.
        // Distinct from `state-forget-gate` (no correction) and `householder-rank1-update`
        // (no gate); the gated delta rule is the composition with gate-first ordering.
        KernelRule {
            rule_id: "gate-then-correct-delta-rule".to_string(),
            gpu_concept: "gated delta rule: decay state then correct against decayed subspace".to_string(),
            ref_shape: "// naive: correction against un-decayed state (wrong ordering — projection misses decay)\nfn delta_no_gate_first(state: &mut [f32], k: &[f32], v: &[f32], beta: f32, gate: &[f32], d: usize, t: usize) {\n    let error = v[t] - dot(&state[t*d..(t+1)*d], &k[t*d..(t+1)*d]); // projects un-decayed state\n    for dd in 0..d { state[t*d+dd] += beta * k[t*d+dd] * error; }\n    let g = gate[t].exp();\n    for dd in 0..d { state[t*d+dd] *= g; } // gate AFTER write — wrong order\n}".to_string(),
            opt_pattern: "// optimized: gate-then-correct (gated_delta_rule) — decay first, correct against decayed\nfn delta_gate_then_correct(state: &mut [f32], k: &[f32], v: &[f32], beta: f32, gate: &[f32], d: usize, t: usize) {\n    let g = gate[t].exp();\n    for dd in 0..d { state[t*d+dd] *= g; } // decay FIRST\n    let error = v[t] - dot(&state[t*d..(t+1)*d], &k[t*d..(t+1)*d]); // projects decayed state\n    for dd in 0..d { state[t*d+dd] += beta * k[t*d+dd] * error; } // write correction\n}".to_string(),
        },
        // ── gate-then-multi-householder-write — one gate, N inner delta writes ────
        // Source: gated_delta_product.py — `state *= g.exp()` then loop `j in 0..num_householder
        // { error = v[i] - state·k[i]; state += k[i] ⊗ (β[i]·error) }`. A single per-head
        // forget gate wraps N Householder delta writes per step. Generalizes
        // `multi-write-per-step` (Batch 3) by composing it with the delta rule inside
        // one gated outer step. The outer gate is paid once; N corrections share the
        // decayed baseline.
        KernelRule {
            rule_id: "gate-then-multi-householder-write".to_string(),
            gpu_concept: "one outer gate wrapping N inner Householder delta writes per step".to_string(),
            ref_shape: "// naive: re-apply the gate per inner write (N gate evaluations, redundant)\nfn multi_write_redundant_gate(state: &mut [f32], k: &[f32], v: &[f32], beta: &[f32], gate: &[f32], d: usize, n_h: usize, t: usize) {\n    for j in 0..n_h {\n        let g = gate[t].exp();\n        for dd in 0..d { state[t*d+dd] *= g; } // gate re-applied N times — redundant\n        let i = t * n_h + j;\n        let error = v[i] - dot(&state[t*d..(t+1)*d], &k[i*d..(i+1)*d]);\n        for dd in 0..d { state[t*d+dd] += beta[i] * k[i*d+dd] * error; }\n    }\n}".to_string(),
            opt_pattern: "// optimized: gate ONCE then N inner delta writes (gated_delta_product)\nfn multi_write_gate_once(state: &mut [f32], k: &[f32], v: &[f32], beta: &[f32], gate: &[f32], d: usize, n_h: usize, t: usize) {\n    let g = gate[t].exp();\n    for dd in 0..d { state[t*d+dd] *= g; } // gate paid once\n    for j in 0..n_h {\n        let i = t * n_h + j;\n        let error = v[i] - dot(&state[t*d..(t+1)*d], &k[i*d..(i+1)*d]); // projects decayed state\n        for dd in 0..d { state[t*d+dd] += beta[i] * k[i*d+dd] * error; } // N corrections\n    }\n}".to_string(),
        },
        // ── online-gram-ridge-solve — gated Gram accumulators + ridge solve per step ─
        // Source: mesa_net.py — `H_kk = g·H_kk + β·k⊗k; H_kv = g·H_kv + β·k⊗v` (two
        // parallel gated Gram accumulators), then `o = H_kv^T · (H_kk + λI)^{-1} · q`.
        // Solving the normal equations at each step turns the recurrence into an
        // online ridge regression — structurally unlike any existing rule
        // (delta/householder/oja WRITE corrections; this SOLVES for the read).
        KernelRule {
            rule_id: "online-gram-ridge-solve".to_string(),
            gpu_concept: "online ridge regression: gated Gram accumulators + per-step solve".to_string(),
            ref_shape: "// naive: store full kv history then batch ridge solve at end (O(N²d²) memory)\nfn ridge_batch(K: &[f32], V: &[f32], q: &[f32], lamb: f32, d: usize, n: usize) -> Vec<f32> {\n    let mut hkk = vec![0.0f32; d * d];\n    let mut hkv = vec![0.0f32; d * d];\n    for t in 0..n {\n        for i in 0..d { for j in 0..d { hkk[i*d+j] += K[t*d+i] * K[t*d+j]; } }\n        for i in 0..d { for j in 0..d { hkv[i*d+j] += K[t*d+i] * V[t*d+j]; } }\n    }\n    // solve (hkk + λI) q* = q at the end — no per-step read\n    solve_ridge(&hkk, &hkv, q, lamb, d)\n}".to_string(),
            opt_pattern: "// optimized: online gated Gram accumulators + per-step ridge solve (mesa_net)\nfn ridge_online_gated(hkk: &mut [f32], hkv: &mut [f32], k: &[f32], v: &[f32], beta: f32, g: f32, q: &[f32], lamb: f32, d: usize) -> Vec<f32> {\n    // decay both Gram matrices with the shared gate\n    let decay = g.exp();\n    for i in 0..(d*d) { hkk[i] *= decay; hkv[i] *= decay; }\n    // accumulate β-weighted outer products\n    for i in 0..d { for j in 0..d {\n        hkk[i*d+j] += beta * k[i] * k[j];\n        hkv[i*d+j] += beta * k[i] * v[j];\n    } }\n    // solve (hkk + λI) q* = q THIS step → read H_kv^T q*\n    solve_ridge(hkk, hkv, q, lamb, d)\n}".to_string(),
        },
        // ── momentum-gated-memory-recurrence — SGD momentum + EMA memory coupling ──
        // Source: titans_linear.py — `momentum = η·momentum − 2θ·∇M L(M)` then
        // `memory = (1−α)·memory + momentum`. Two coupled gated recurrences: the
        // momentum state carries SGD-with-momentum; the memory matrix is an EMA of
        // the momentum. The gradient is the reconstruction error against the CURRENT
        // memory. "Train a linear model at inference time" as a recurrence.
        KernelRule {
            rule_id: "momentum-gated-memory-recurrence".to_string(),
            gpu_concept: "two coupled recurrences: SGD momentum state + EMA memory matrix".to_string(),
            ref_shape: "// naive: plain GD on memory, no momentum (slow convergence, no EMA smoothing)\nfn gd_no_momentum(memory: &mut [f32], k: &[f32], v: &[f32], theta: f32, d: usize, t: usize) {\n    // ∇ = k ⊗ (kW − v) — reconstruction error gradient\n    let mut kw = vec![0.0f32; d];\n    for i in 0..d { for j in 0..d { kw[i] += memory[j*d+i] * k[t*d+j]; } }\n    for i in 0..d { for j in 0..d {\n        memory[j*d+i] -= 2.0 * theta * k[t*d+j] * (kw[i] - v[t*d+i]);\n    } }\n}".to_string(),
            opt_pattern: "// optimized: momentum + EMA memory (titans_linear) — two coupled gated recurrences\nfn momentum_ema_memory(memory: &mut [f32], momentum: &mut [f32], k: &[f32], v: &[f32], theta: f32, eta: f32, alpha: f32, d: usize, t: usize) {\n    // ∇ = k ⊗ (kW − v) against current memory\n    let mut kw = vec![0.0f32; d];\n    for i in 0..d { for j in 0..d { kw[i] += memory[j*d+i] * k[t*d+j]; } }\n    // momentum recurrence: m = η·m − 2θ·∇\n    for i in 0..d { for j in 0..d {\n        let grad = k[t*d+j] * (kw[i] - v[t*d+i]);\n        momentum[j*d+i] = eta * momentum[j*d+i] - 2.0 * theta * grad;\n    } }\n    // memory recurrence: M = (1−α)·M + m (EMA of momentum)\n    for i in 0..(d*d) { memory[i] = (1.0 - alpha) * memory[i] + momentum[i]; }\n}".to_string(),
        },
        // ── current-token-bonus-read-before-update — boosted read, then plain write ──
        // Source: rwkv6.py — `boosted = state + diag(u) ⊗ (k⊗v); out = r·boosted;
        // state = state·diag(e^w) + k⊗v`. The output is read from a BOOSTED state
        // (current token gets a bonus `u`), THEN the state updates with the
        // un-boosted kv. The bonus is transient — additive for the read only,
        // not stored. Distinct from standard write-then-read recurrence.
        KernelRule {
            rule_id: "current-token-bonus-read-before-update".to_string(),
            gpu_concept: "read from boosted state (transient bonus), then update with un-boosted kv".to_string(),
            ref_shape: "// naive: write to state first, read after (no bonus — current token undervalued)\nfn write_then_read(state: &mut [f32], k: &[f32], v: &[f32], r: &[f32], w: &[f32], d: usize, t: usize) -> f32 {\n    // decay + write\n    for dd in 0..d { state[t*d+dd] = state[t*d+dd] * w[t].exp() + k[t*d+dd] * v[t]; }\n    // read AFTER write — no current-token bonus\n    let mut out = 0.0;\n    for dd in 0..d { out += state[t*d+dd] * r[t*d+dd]; }\n    out\n}".to_string(),
            opt_pattern: "// optimized: read-before-update with transient bonus (rwkv6)\nfn bonus_read_before_update(state: &mut [f32], k: &[f32], v: &[f32], r: &[f32], w: &[f32], u: &[f32], d: usize, t: usize) -> f32 {\n    // boosted state = state + u·(k⊗v) — transient bonus for the READ only\n    let mut out = 0.0;\n    for dd in 0..d {\n        let boosted = state[t*d+dd] + u[t*d+dd] * k[t*d+dd] * v[t];\n        out += boosted * r[t*d+dd];\n    }\n    // update state with UN-boosted kv (bonus not stored)\n    for dd in 0..d { state[t*d+dd] = state[t*d+dd] * w[t].exp() + k[t*d+dd] * v[t]; }\n    out\n}".to_string(),
        },
        // ── wkv-logsumexp-three-channel-state — 3-channel (num/den/max) logsumexp ──
        // Source: rwkv4.py — state = (alpha, beta, eps) = (numerator, denominator,
        // running-max). Each step: `tau = max(ukt, eps); up = exp(eps−tau); cur =
        // exp(ukt−tau); out = (up·alpha + cur·v) / (up·beta + cur)`. The running max
        // `eps` prevents exp overflow over long sequences. The 3rd channel is the
        // numerical-stability carrier — structurally distinct from 2-channel EMA.
        KernelRule {
            rule_id: "wkv-logsumexp-three-channel-state".to_string(),
            gpu_concept: "WKV with 3-channel state (num/den/max) + logsumexp max-subtraction".to_string(),
            ref_shape: "// naive: 2-channel EMA (num/den) — exp overflow on long sequences (no running max)\nfn wkv_2ch(alpha: &mut f32, beta: &mut f32, k: f32, v: f32, decay: f32, bonus: f32) -> f32 {\n    let ukt = bonus + k;\n    let out = (alpha.exp() * *alpha + ukt.exp() * v) / (alpha.exp() * *beta + ukt.exp());\n    // overflow: ukt.exp() blows up when bonus+k is large\n    *alpha = decay.exp() * *alpha + ukt.exp() * v;\n    *beta = decay.exp() * *beta + ukt.exp();\n    out\n}".to_string(),
            opt_pattern: "// optimized: 3-channel logsumexp-stable WKV (rwkv4) — running max prevents overflow\nfn wkv_3ch_logsumexp(alpha: &mut f32, beta: &mut f32, eps: &mut f32, k: f32, v: f32, decay: f32, bonus: f32) -> f32 {\n    let ukt = bonus + k;\n    let tau = ukt.max(*eps); // running max\n    let up = (*eps - tau).exp(); // bounded ≤ 1\n    let cur = (ukt - tau).exp(); // bounded ≤ 1\n    let out = (up * *alpha + cur * v) / (up * *beta + cur);\n    // advance the 3-channel state\n    let shifted = decay + *eps;\n    *eps = shifted.max(k);\n    let up2 = (shifted - *eps).exp();\n    let cur2 = (k - *eps).exp();\n    *alpha = up2 * *alpha + cur2 * v;\n    *beta = up2 * *beta + cur2;\n    out\n}".to_string(),
        },
        // ── hierarchical-log2-level-mask — O(log seq) hierarchical sparse mask ──────
        // Source: log_linear_attn.py — `levels = ceil(log2(seq))+1`; base =
        // `diag_embed(scales[...,0,:])`; per level `half = 1<<(level−1)`, `anchor =
        // i − i%half`, mask `(i%(2·half) ≥ half) & (j+half ≥ anchor) & (j < anchor)`.
        // One learned scale per log2 level. Converts dense N×N decay into a sparse
        // hierarchical structure — distinct from dense `pairwise-distance-decay-matrix`.
        KernelRule {
            rule_id: "hierarchical-log2-level-mask".to_string(),
            gpu_concept: "O(log seq) hierarchical mask with per-level learned scales".to_string(),
            ref_shape: "// naive: dense N×N decay matrix (O(N²) memory + compute)\nfn dense_decay_matrix(seq: usize, slope: f32) -> Vec<f32> {\n    let mut m = vec![0.0f32; seq * seq];\n    for i in 0..seq { for j in 0..=i {\n        m[i*seq+j] = (slope * (i - j) as f32).exp();\n    } }\n    m\n}".to_string(),
            opt_pattern: "// optimized: hierarchical log2-level mask (log_linear_attn) — O(N log N) sparse\nfn hierarchical_mask(seq: usize, scales: &[f32]) -> Vec<f32> {\n    let levels = (seq as f32).log2().ceil() as usize + 1;\n    let mut h = vec![0.0f32; seq * seq];\n    // level 0: diagonal\n    for i in 0..seq { h[i*seq+i] = scales[0]; }\n    // levels 1..: anchor-relative blocks\n    for level in 1..levels {\n        let half = 1 << (level - 1);\n        for i in 0..seq {\n            let anchor = i - i % half;\n            for j in 0..seq {\n                let in_block = (i % (2*half) >= half) && (j + half >= anchor) && (j < anchor);\n                if in_block { h[i*seq+j] = scales[level]; }\n            }\n        }\n    }\n    h\n}".to_string(),
        },
        // ── iterative-cg-solve-over-matvec — CG with Jacobi preconditioner ──────────
        // Source: mesa_net_decode.py — solve `(H_kk + λI) x = q` via conjugate
        // gradient: `apply_system(x) = H_kk·x + λ·x` (matvec, no inverse materialized),
        // diagonal Jacobi preconditioner `x = q / (diag(H_kk) + λ + ε)`, then the CG
        // loop (`α = r²/(d·Ad)`, Polak-Ribière `β`). Trading memory (no inverse) for
        // bounded iterations.
        KernelRule {
            rule_id: "iterative-cg-solve-over-matvec".to_string(),
            gpu_concept: "conjugate-gradient solve over matvec with Jacobi preconditioner (no inverse)".to_string(),
            ref_shape: "// naive: materialize the inverse (O(d²) memory + O(d³) solve)\nfn solve_via_inverse(hkk: &[f32], q: &[f32], lamb: f32, d: usize) -> Vec<f32> {\n    let mut a = hkk.to_vec();\n    for i in 0..d { a[i*d+i] += lamb; } // add λI\n    let inv = inverse(&a, d); // O(d³) — materialize the full inverse\n    matvec(&inv, q, d)\n}".to_string(),
            opt_pattern: "// optimized: CG iteration over matvec (mesa_net_decode) — no inverse, bounded iters\nfn solve_cg(hkk: &[f32], q: &[f32], lamb: f32, d: usize, max_iter: usize) -> Vec<f32> {\n    let apply = |x: &[f32]| -> Vec<f32> {\n        let mut out = vec![0.0f32; d];\n        for i in 0..d { for j in 0..d { out[i] += hkk[i*d+j] * x[j]; } out[i] += lamb * x[i]; }\n        out\n    };\n    // Jacobi preconditioner: x₀ = q / (diag + λ + ε)\n    let mut x = vec![0.0f32; d];\n    for i in 0..d { x[i] = q[i] / (hkk[i*d+i] + lamb + 1e-5); }\n    let mut r = { let ax = apply(&x); q.iter().zip(ax.iter()).map(|(a,b)| a-b).collect::<Vec<_>>() };\n    let mut dir = r.clone();\n    let mut sq = r.iter().map(|v| v*v).sum::<f32>();\n    for _ in 0..max_iter {\n        let ad = apply(&dir);\n        let alpha = sq / (dir.iter().zip(ad.iter()).map(|(a,b)| a*b).sum::<f32>() + 1e-5);\n        for i in 0..d { x[i] += alpha * dir[i]; r[i] -= alpha * ad[i]; }\n        let next_sq = r.iter().map(|v| v*v).sum::<f32>();\n        let beta_cg = next_sq / (sq + 1e-5);\n        for i in 0..d { dir[i] = r[i] + beta_cg * dir[i]; }\n        sq = next_sq;\n    }\n    x\n}".to_string(),
        },
        // ── chunked-test-time-gradient-descent — mini-batch chunked GD + last-step ──
        // Source: ttt.py — `for start in 0..seq step mini_batch_size`: compute the
        // GD step `∇ = (LN(kW)·w − (v−k))·w` over the chunk with a lower-triangular
        // causal mask `attn = (q·k^T)·tril`, then update `W −= η_last · k^T · v_new`
        // using only the LAST step's gradient (`η_last = η[:,:,-1:]`). The
        // last-step-update bounds the weight update to one matmul per chunk.
        KernelRule {
            rule_id: "chunked-test-time-gradient-descent".to_string(),
            gpu_concept: "mini-batch chunked GD with causal mask + last-step weight update".to_string(),
            ref_shape: "// naive: sequential per-token GD (O(seq) sequential steps, no chunking)\nfn gd_sequential(w: &mut [f32], k: &[f32], v: &[f32], eta: f32, d: usize, seq: usize) {\n    for t in 0..seq {\n        let mut kw = vec![0.0f32; d];\n        for i in 0..d { for j in 0..d { kw[i] += w[j*d+i] * k[t*d+j]; } }\n        let grad = k[t] * (kw[0] - v[t]); // simplified per-token gradient\n        for i in 0..(d*d) { w[i] -= eta * grad; } // full update per token\n    }\n}".to_string(),
            opt_pattern: "// optimized: chunked mini-batch GD with last-step update (ttt)\nfn gd_chunked_last_step(w: &mut [f32], k: &[f32], v: &[f32], eta_last: &[f32], d: usize, seq: usize, mbs: usize) {\n    for start in (0..seq).step_by(mbs) {\n        let stop = (start + mbs).min(seq);\n        // compute v_new over the chunk (causal lower-triangular attn)\n        let mut v_new = vec![0.0f32; (stop-start)*d];\n        for oi in 0..(stop-start) {\n            for oj in 0..=oi { // tril: causal within chunk\n                for dd in 0..d { v_new[oi*d+dd] += k[(start+oj)*d+dd] * v[(start+oj)*d]; }\n            }\n        }\n        // weight update using ONLY the last step's gradient (η_last)\n        let last = stop - start - 1;\n        for i in 0..d { for j in 0..d {\n            w[j*d+i] -= eta_last[0] * k[(start+last)*d+j] * v_new[last*d+i];\n        } }\n    }\n}".to_string(),
        },
        // ── block-local-reference-frame-for-overflow — reframe per block to bound exp ──
        // Source: wall_attn.py — per-channel decay `2^(P_i − P_j)` overflows fp32
        // when prefix > 128 in log2. Fix: frame each query block on its OWN first
        // prefix — `scores[start:stop,:start] = (q·2^(P_q−P_ref)) · (k·2^(P_ref−P_k))`
        // keeps both factors in `[0,1]` for keys before the block. Block size
        // `max(16, seq/(4·√head_dim))` scales with sequence to keep the diagonal
        // ~1/16 of the score cost.
        KernelRule {
            rule_id: "block-local-reference-frame-for-overflow".to_string(),
            gpu_concept: "per-block reference frame to keep exp arguments bounded (avoid fp32 overflow)".to_string(),
            ref_shape: "// naive: global prefix difference — 2^(P_i - P_j) overflows fp32 on long sequences\nfn decay_global_overflow(q: &[f32], k: &[f32], prefix: &[f32], d: usize, seq: usize) -> Vec<f32> {\n    let mut scores = vec![0.0f32; seq * seq];\n    for i in 0..seq { for j in 0..seq {\n        // overflow: prefix[i] - prefix[j] can exceed 128 in log2\n        let decay = 2.0f32.powf(prefix[i] - prefix[j]);\n        let mut s = 0.0;\n        for dd in 0..d { s += q[i*d+dd] * k[j*d+dd] * decay; }\n        scores[i*seq+j] = s;\n    } }\n    scores\n}".to_string(),
            opt_pattern: "// optimized: block-local reference frame (wall_attn) — both factors stay in [0,1]\nfn decay_block_local(q: &[f32], k: &[f32], prefix: &[f32], d: usize, seq: usize) -> Vec<f32> {\n    let block = ((seq as f32 / (4.0 * (d as f32).sqrt())) as usize).max(16);\n    let mut scores = vec![0.0f32; seq * seq];\n    for start in (0..seq).step_by(block) {\n        let stop = (start + block).min(seq);\n        let pref = prefix[start]; // block's own reference frame\n        for i in start..stop { for j in 0..start { // keys before the block\n            // both exp2 args ≤ 0 → both factors in [0,1], no overflow\n            let qi = 2.0f32.powf(prefix[i] - pref); // ≤ 1 (pref = min in block)\n            let kj = 2.0f32.powf(pref - prefix[j]); // ≤ 1\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * qi * k[j*d+dd] * kj; }\n            scores[i*seq+j] = s;\n        } }\n    }\n    scores\n}".to_string(),
        },
        // ── two-stage-mask-softmax-then-conv — -inf before, 0 after ────────────────
        // Source: multi_token_attention.py — `softmax(masked_fill(causal, -1e9))`
        // then `conv2d(probs, weight)` then `masked_fill(causal, 0.0)`. Two-stage
        // masking: -inf BEFORE softmax (probabilities correct — future gets zero
        // weight), 0 AFTER conv (conv doesn't leak future-mixed values back). The
        // -inf-before / 0-after split is the only masking that is both
        // probability-correct AND conv-causal-safe.
        KernelRule {
            rule_id: "two-stage-mask-softmax-then-conv".to_string(),
            gpu_concept: "two-stage mask: -inf before softmax, 0 after conv (probability + causal correct)".to_string(),
            ref_shape: "// naive: single mask after conv only — softmax leaks future probability into conv\nfn mask_after_conv_only(scores: &[f32], n: usize, kernel: &[f32], ksize: usize) -> Vec<f32> {\n    // softmax WITHOUT pre-mask — future positions get nonzero weight\n    let mut probs = vec![0.0f32; n * n];\n    for i in 0..n {\n        let mut mx = f32::NEG_INFINITY;\n        for j in 0..n { mx = mx.max(scores[i*n+j]); }\n        let mut s = 0.0;\n        for j in 0..n { probs[i*n+j] = (scores[i*n+j]-mx).exp(); s += probs[i*n+j]; }\n        for j in 0..n { probs[i*n+j] /= s; } // future leaks weight here\n    }\n    conv2d(&probs, kernel, ksize, n) // mask AFTER — too late, future already mixed\n}".to_string(),
            opt_pattern: "// optimized: two-stage mask (multi_token_attention) — -inf before, 0 after\nfn two_stage_mask(scores: &[f32], n: usize, kernel: &[f32], ksize: usize) -> Vec<f32> {\n    // STAGE 1: -inf before softmax → future gets exactly zero probability\n    let mut probs = vec![0.0f32; n * n];\n    for i in 0..n {\n        let mut mx = f32::NEG_INFINITY;\n        for j in 0..n { let s = if j <= i { scores[i*n+j] } else { -1e9 }; mx = mx.max(s); }\n        let mut s = 0.0;\n        for j in 0..n { probs[i*n+j] = if j <= i { (scores[i*n+j]-mx).exp() } else { 0.0 }; s += probs[i*n+j]; }\n        for j in 0..n { probs[i*n+j] /= s; } // future is exactly 0\n    }\n    let out = conv2d(&probs, kernel, ksize, n);\n    // STAGE 2: 0 after conv → clear any future-mixed values the conv introduced\n    let mut masked = out;\n    for i in 0..n { for j in 0..n { if j > i { masked[i*n+j] = 0.0; } } }\n    masked\n}".to_string(),
        },
        // ── softmax-topk-renormalize-scatter — MoE gate: softmax→topk→renorm→scatter ─
        // Source: momoe.py — `probs = softmax(logits)`, `picked = probs.topk(k)`,
        // `gates = zeros.scatter(-1, picked.indices, picked.values / picked.values.sum())`.
        // Three steps fused: (1) softmax over all experts, (2) top-k + renormalization
        // of selected probs to sum to 1, (3) scatter into a sparse N×experts gate
        // matrix with only k nonzeros per token. The sparse structure is what the
        // grouped GEMM consumes.
        KernelRule {
            rule_id: "softmax-topk-renormalize-scatter".to_string(),
            gpu_concept: "MoE gate: softmax → topk → renormalize → scatter sparse gate matrix".to_string(),
            ref_shape: "// naive: dense gate matrix (all experts get a weight, O(N·E) memory)\nfn dense_gate(logits: &[f32], n: usize, e: usize) -> Vec<f32> {\n    let mut gates = vec![0.0f32; n * e];\n    for i in 0..n {\n        let mut mx = f32::NEG_INFINITY;\n        for j in 0..e { mx = mx.max(logits[i*e+j]); }\n        let mut s = 0.0;\n        for j in 0..e { gates[i*e+j] = (logits[i*e+j]-mx).exp(); s += gates[i*e+j]; }\n        for j in 0..e { gates[i*e+j] /= s; } // dense — every expert active\n    }\n    gates\n}".to_string(),
            opt_pattern: "// optimized: softmax-topk-renormalize-scatter (momoe) — sparse k-nonzero gate\nfn sparse_topk_gate(logits: &[f32], n: usize, e: usize, top_k: usize) -> Vec<f32> {\n    let mut gates = vec![0.0f32; n * e]; // sparse: only k nonzeros per row\n    for i in 0..n {\n        // softmax\n        let mut mx = f32::NEG_INFINITY;\n        for j in 0..e { mx = mx.max(logits[i*e+j]); }\n        let mut probs = vec![0.0f32; e];\n        let mut s = 0.0;\n        for j in 0..e { probs[j] = (logits[i*e+j]-mx).exp(); s += probs[j]; }\n        for j in 0..e { probs[j] /= s; }\n        // top-k selection\n        let mut idx: Vec<usize> = (0..e).collect();\n        idx.sort_by(|&a,&b| probs[b].partial_cmp(&probs[a]).unwrap());\n        let picked = &idx[..top_k];\n        // renormalize selected probs to sum to 1\n        let sum: f32 = picked.iter().map(|&j| probs[j]).sum();\n        // scatter into sparse gate matrix\n        for &j in picked { gates[i*e+j] = probs[j] / sum; }\n    }\n    gates\n}".to_string(),
        },
        // ── fuse-gate-transform-chunk-cumsum — gate transform fused with chunk cumsum ─
        // Source: kda_gate_cumsum.py — `gate = transform(g)` then
        // `cat([chunk.cumsum(1) for chunk in gate.split(chunk_size, 1)], 1)`.
        // Fuses the gate transform (`-rate·softplus(g+b)` or `L·sigmoid(rate·(g+b))`)
        // with a chunk-local inclusive cumsum. The gate output feeds the cumsum
        // without a materialized intermediate. Composes `chunk-reset-cumsum`
        // (Batch 6) with the gate transform as a fused producer.
        KernelRule {
            rule_id: "fuse-gate-transform-chunk-cumsum".to_string(),
            gpu_concept: "fuse gate transform with chunk-local cumsum (no materialized intermediate)".to_string(),
            ref_shape: "// naive: gate transform then SEPARATE chunk cumsum (two kernel passes)\nfn gate_then_cumsum_separate(g: &[f32], rate: f32, chunk: usize, n: usize) -> Vec<f32> {\n    // pass 1: gate transform (materialized intermediate)\n    let mut gate = vec![0.0f32; n];\n    for i in 0..n { gate[i] = -rate * softplus(g[i]); }\n    // pass 2: chunk-local cumsum over the intermediate\n    let mut out = vec![0.0f32; n];\n    for c_start in (0..n).step_by(chunk) {\n        let c_end = (c_start + chunk).min(n);\n        let mut acc = 0.0;\n        for i in c_start..c_end { acc += gate[i]; out[i] = acc; }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: fused gate-transform + chunk-cumsum (kda_gate_cumsum) — one pass\nfn fused_gate_chunk_cumsum(g: &[f32], rate: f32, chunk: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for c_start in (0..n).step_by(chunk) {\n        let c_end = (c_start + chunk).min(n);\n        let mut acc = 0.0;\n        for i in c_start..c_end {\n            // gate transform INLINE — feeds the cumsum directly, no intermediate\n            acc += -rate * softplus(g[i]);\n            out[i] = acc;\n        }\n    }\n    out\n}\nfn softplus(x: f32) -> f32 { if x > 20.0 { x } else { (1.0 + x.exp()).ln() } }".to_string(),
        },
        // ── dual-mode-gate-transform — unbounded softplus vs bounded sigmoid ────────
        // Source: kda_gate.py — two gate formulas dispatched by parameter presence:
        // unbounded `-rate·softplus(g+b)` when `lower_bound is None`, bounded
        // `L·sigmoid(rate·(g+b))` when a `lower_bound` is provided. The bounded mode
        // clamps the gate to `[0, L]` for numerical stability; the unbounded mode
        // lets it grow freely. One kernel, two formulas, parameter-specialized.
        KernelRule {
            rule_id: "dual-mode-gate-transform".to_string(),
            gpu_concept: "dual-mode gate: unbounded softplus OR bounded sigmoid, dispatched by parameter".to_string(),
            ref_shape: "// naive: always use one formula regardless of stability needs (hardcoded)\nfn gate_hardcoded(g: &[f32], rate: f32, n: usize) -> Vec<f32> {\n    // always unbounded softplus — can overflow on large g even when bounded mode is wanted\n    (0..n).map(|i| -rate * softplus(g[i])).collect()\n}\nfn softplus(x: f32) -> f32 { if x > 20.0 { x } else { (1.0 + x.exp()).ln() } }".to_string(),
            opt_pattern: "// optimized: dual-mode gate (kda_gate) — dispatched by lower_bound presence\nfn gate_dual_mode(g: &[f32], rate: f32, lower_bound: Option<f32>, n: usize) -> Vec<f32> {\n    match lower_bound {\n        None => {\n            // unbounded: -rate·softplus(g) — lets gate grow freely\n            (0..n).map(|i| -rate * softplus(g[i])).collect()\n        }\n        Some(lb) => {\n            // bounded: lb·sigmoid(rate·g) — clamps to [0, lb] for stability\n            (0..n).map(|i| lb * sigmoid(rate * g[i])).collect()\n        }\n    }\n}\nfn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }\nfn softplus(x: f32) -> f32 { if x > 20.0 { x } else { (1.0 + x.exp()).ln() } }".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 13 (Research 014): final popcorn pass — RL loss stability,
        // complex-free RoPE, multimodal RoPE, RWKV-7 channel mixing + gate output.
        // 6 rules from 7 unmined popcorn files.
        // ════════════════════════════════════════════════════════════════════
        // ── expm1-for-near-zero-exp-minus-one — exp(x)−1 → expm1(x) (Batch 13, B) ──
        // Source: grpo_offpolicy.py — KL penalty `loss_kl = expm1(delta) − delta`
        // computes `e^delta − 1 − delta`. Near delta=0, `exp(delta) − 1` suffers
        // catastrophic cancellation (both terms ≈ 1). `expm1(delta)` is accurate
        // for small arguments; the `− delta` shifts the stable kernel to the full
        // KL expression. Distinct from `exp2-log2-base2` (that's a base swap; this
        // is a cancellation fix). Batch 6 had deferred grpo_offpolicy as "RL math,
        // fusion covered" — the numerical-stability aspect was overlooked.
        KernelRule {
            rule_id: "expm1-for-near-zero-exp-minus-one".to_string(),
            gpu_concept: "numerically stable e^x − 1 via expm1 (avoids catastrophic cancellation near 0)".to_string(),
            ref_shape: "// naive: exp(x) - 1 — catastrophic cancellation when x is near 0\nfn kl_term_naive(delta: &[f32], n: usize) -> Vec<f32> {\n    (0..n).map(|i| delta[i].exp() - 1.0 - delta[i]).collect()\n    // for delta ≈ 0: exp(0.0001) = 1.0001, 1.0001 - 1 = 0.0001 (lost ~4 digits)\n}".to_string(),
            opt_pattern: "// optimized: expm1(x) - x — expm1 is accurate near 0 (grpo_offpolicy KL)\nfn kl_term_stable(delta: &[f32], n: usize) -> Vec<f32> {\n    // expm1(0.0001) = 0.000100005 — full precision, no cancellation\n    (0..n).map(|i| delta[i].exp_m1() - delta[i]).collect()\n}".to_string(),
        },
        // ── complex-mul-via-paired-reals — complex dtype → interleaved real/imag (B) ──
        // Source: llama4_rope.py — rotary embedding stores freqs as `[cos, sin]`
        // pairs (shape `[seq, head_dim/2, 2]`) and does complex multiplication via
        // paired reals: `(r1·r2 − i1·i2, r1·i2 + i1·r2)`. Avoids the complex dtype
        // entirely — on hardware where `complex<f32>` maps to a slow library path
        // (not a native register-width op), paired-reals + 4 scalar muls is faster.
        // The input is `unflatten(-1, (-1, 2))` then `unbind(-1)` to split pairs.
        KernelRule {
            rule_id: "complex-mul-via-paired-reals".to_string(),
            gpu_concept: "represent complex numbers as interleaved [real, imag] pairs, multiply via 4 scalar muls".to_string(),
            ref_shape: "// naive: use complex<f32> dtype — maps to a slow library path on some hardware\nfn rope_complex(x: &[f32], freqs_complex: &[num_complex::Complex32], n: usize, d: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n { for j in 0..(d/2) {\n        let xc = num_complex::Complex::new(x[i*d+2*j], x[i*d+2*j+1]);\n        let prod = xc * freqs_complex[j]; // complex mul — library call, not native\n        out[i*d+2*j] = prod.re; out[i*d+2*j+1] = prod.im;\n    } }\n    out\n}".to_string(),
            opt_pattern: "// optimized: paired reals — 4 scalar muls, no complex dtype (llama4_rope)\nfn rope_paired_reals(x: &[f32], freqs: &[f32], n: usize, d: usize) -> Vec<f32> {\n    // freqs layout: [head_dim/2, 2] = [(freq_real, freq_imag), ...]\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n { for j in 0..(d/2) {\n        let xr = x[i*d+2*j];     // real part of x pair j\n        let xi = x[i*d+2*j+1];   // imag part of x pair j\n        let fr = freqs[2*j];     // freq real\n        let fi = freqs[2*j+1];   // freq imag\n        // complex mul via paired reals: (xr+xi·i)·(fr+fi·i) = (xr·fr−xi·fi) + (xr·fi+xi·fr)·i\n        out[i*d+2*j]   = xr * fr - xi * fi;\n        out[i*d+2*j+1] = xr * fi + xi * fr;\n    } }\n    out\n}".to_string(),
        },
        // ── cyclic-modality-table-selection — multimodal RoPE: cyclic table gather (D) ──
        // Source: qwen2vl_mrope.py — multimodal RoPE splits head_dim into N sections
        // (temporal, height, width), each pulling cos/sin from a different modality
        // table. `cos.split(doubled, -1)` then `table[i % 3]` selects per-section.
        // The cyclic `i % n_modalities` dispatch is the novel part — a per-section
        // gather from N constant tables, then `cat` back together. Distinct from
        // standard RoPE (single table) and from `predicate-gated-backend` (runtime
        // dispatch — this is a compile-time section layout).
        KernelRule {
            rule_id: "cyclic-modality-table-selection".to_string(),
            gpu_concept: "multimodal RoPE: split head_dim into sections, cyclic gather from N modality tables".to_string(),
            ref_shape: "// naive: single RoPE table for all dims (ignores multimodal structure)\nfn rope_single_table(x: &[f32], cos: &[f32], sin: &[f32], n: usize, d: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n { for j in 0..(d/2) {\n        let c = cos[j]; let s = sin[j]; // one table — wrong for multimodal\n        out[i*d+2*j]   = x[i*d+2*j] * c - x[i*d+2*j+1] * s;\n        out[i*d+2*j+1] = x[i*d+2*j] * s + x[i*d+2*j+1] * c;\n    } }\n    out\n}".to_string(),
            opt_pattern: "// optimized: cyclic modality table selection (qwen2vl_mrope) — per-section table\nfn rope_multimodal(x: &[f32], cos_tables: &[Vec<f32>], sin_tables: &[Vec<f32>], sections: &[usize], n: usize, d: usize) -> Vec<f32> {\n    let n_mod = cos_tables.len(); // e.g. 3: temporal, height, width\n    let mut out = vec![0.0f32; n * d];\n    let mut offset = 0usize;\n    for (sec_idx, &sec_len) in sections.iter().enumerate() {\n        let table_idx = sec_idx % n_mod; // cyclic: temporal=0, height=1, width=2, repeat\n        let cos_t = &cos_tables[table_idx];\n        let sin_t = &sin_tables[table_idx];\n        for i in 0..n { for j in 0..sec_len {\n            let c = cos_t[j]; let s = sin_t[j];\n            out[i*d+2*(offset+j)]   = x[i*d+2*(offset+j)] * c - x[i*d+2*(offset+j)+1] * s;\n            out[i*d+2*(offset+j)+1] = x[i*d+2*(offset+j)] * s + x[i*d+2*(offset+j)+1] * c;\n        } }\n        offset += sec_len;\n    }\n    out\n}".to_string(),
        },
        // ── stateful-shift-via-cat-prefix — shift with cross-batch carry state (B) ──
        // Source: rwkv7_channel_mixing.py — token-shift `previous = cat((x_prev[:,
        // None], x[:, :-1]), dim=1)` reconstructs the shifted sequence by
        // concatenating the previous batch's last token as the prefix. Distinct from
        // `shift-via-pad-subtract` (Batch 10, token_shift.py): that pads with zeros
        // + computes a DIFFERENCE (y = x_{t-1} − x_t); this carries real state across
        // batch boundaries (y[0] = x_prev, not 0). Required for stateful RNN-style
        // mixing where the first token of each batch depends on the last token of
        // the prior batch.
        KernelRule {
            rule_id: "stateful-shift-via-cat-prefix".to_string(),
            gpu_concept: "shift sequence with cross-batch carry: cat(prev_state, x[:-1])".to_string(),
            ref_shape: "// naive: pad with zeros (loses cross-batch state — first token is wrong)\nfn shift_zero_pad(x: &[f32], seq: usize, dim: usize) -> Vec<f32> {\n    let mut shifted = vec![0.0f32; seq * dim];\n    shifted[dim..].copy_from_slice(&x[..(seq-1)*dim]); // shifted[0] = 0 — WRONG carry\n    shifted\n}".to_string(),
            opt_pattern: "// optimized: stateful shift via cat prefix (rwkv7_channel_mixing)\nfn shift_cat_carry(x: &[f32], x_prev: &[f32], seq: usize, dim: usize) -> Vec<f32> {\n    let mut shifted = vec![0.0f32; seq * dim];\n    // shifted[0] = x_prev (cross-batch carry — the real last token of prior batch)\n    shifted[..dim].copy_from_slice(x_prev);\n    // shifted[1..] = x[..seq-1] (shift the current batch by one)\n    shifted[dim..].copy_from_slice(&x[..(seq-1)*dim]);\n    shifted\n}".to_string(),
        },
        // ── squared-relu-activation — relu(x)² sparse sharpening (B) ──
        // Source: sqrelu.py (+ rwkv7_channel_mixing.py consumer) — `relu(x).square()`
        // preserves ReLU's exact sparsity (zeros stay zero) while quadratically
        // sharpening non-zero values. Used in Primer (So et al. 2021) + RWKV-7
        // channel mixing. Distinct from `gelu`/`swish` (smooth activations): the
        // squaring is applied AFTER the relu gate, so the sparse structure is exact.
        // The fused form `max(x,0)²` avoids materializing the intermediate relu
        // output before squaring.
        KernelRule {
            rule_id: "squared-relu-activation".to_string(),
            gpu_concept: "relu(x)² — sparse activation: preserves ReLU zeros, sharpens non-zeros quadratically".to_string(),
            ref_shape: "// naive: relu then separate square (two passes, materialized intermediate)\nfn sqrelu_two_pass(x: &[f32], n: usize) -> Vec<f32> {\n    let mut relu_out = vec![0.0f32; n];\n    for i in 0..n { relu_out[i] = if x[i] > 0.0 { x[i] } else { 0.0 }; } // pass 1\n    let mut out = vec![0.0f32; n];\n    for i in 0..n { out[i] = relu_out[i] * relu_out[i]; } // pass 2\n    out\n}".to_string(),
            opt_pattern: "// optimized: fused squared ReLU (sqrelu) — one pass, no intermediate\nfn sqrelu_fused(x: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for i in 0..n {\n        let v = if x[i] > 0.0 { x[i] } else { 0.0 }; // relu gate\n        out[i] = v * v; // square inline — zeros stay zero, non-zeros sharpen\n    }\n    out\n}".to_string(),
        },
        // ── rkv-receptance-correction — reduce-then-broadcast-multiply fused (C) ──
        // Source: rwkv7_gate_output.py — `correction = (r*k*r_k).sum(-1, keepdim=
        // true) * v`, then `y = (o + correction) * g`. A reduce-then-broadcast-multiply
        // fusion: elementwise triple-product (r·k·ρ), reduce over head_dim with
        // `keepdim` (broadcastable scalar per head), multiply by v (broadcasts back
        // to full head_dim), add to o, gate by g. Four elementwise ops + one reduce
        // fused into a single kernel pass — the `keepdim` makes the reduce output
        // directly broadcastable without a reshape. Distinct from `gather-selective-
        // reduction` (that's a gather-then-reduce; this is a reduce-then-broadcast).
        KernelRule {
            rule_id: "rkv-receptance-correction".to_string(),
            gpu_concept: "fused reduce-then-broadcast: (r·k·ρ).sum(head_dim,keepdim)·v added to o, gated by g".to_string(),
            ref_shape: "// naive: 4 separate passes — triple-mul, reduce, broadcast-mul, add+gate\nfn rkv_correction_naive(o: &[f32], r: &[f32], k: &[f32], r_k: &[f32], v: &[f32], g: &[f32], d: usize, n: usize) -> Vec<f32> {\n    // pass 1: elementwise r*k*r_k (materialized)\n    let mut triple = vec![0.0f32; n * d];\n    for i in 0..(n*d) { triple[i] = r[i] * k[i] * r_k[i]; }\n    // pass 2: reduce over head_dim (materialized scalar per head)\n    let mut reduced = vec![0.0f32; n];\n    for i in 0..n { for dd in 0..d { reduced[i] += triple[i*d+dd]; } }\n    // pass 3: broadcast-multiply by v (materialized)\n    let mut correction = vec![0.0f32; n * d];\n    for i in 0..n { for dd in 0..d { correction[i*d+dd] = reduced[i] * v[i*d+dd]; } }\n    // pass 4: add to o, gate by g\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..(n*d) { out[i] = (o[i] + correction[i]) * g[i]; }\n    out\n}".to_string(),
            opt_pattern: "// optimized: fused reduce-then-broadcast (rwkv7_gate_output) — one pass\nfn rkv_correction_fused(o: &[f32], r: &[f32], k: &[f32], r_k: &[f32], v: &[f32], g: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        // reduce over head_dim: dot(r*k*r_k, all dims) — keepdim = scalar per head\n        let mut s = 0.0;\n        for dd in 0..d { s += r[i*d+dd] * k[i*d+dd] * r_k[i*d+dd]; }\n        // broadcast-multiply by v, add to o, gate by g — all inline, no intermediate\n        for dd in 0..d {\n            out[i*d+dd] = (o[i*d+dd] + s * v[i*d+dd]) * g[i*d+dd];\n        }\n    }\n    out\n}".to_string(),
        },
    ]
}

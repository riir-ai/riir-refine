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
//! - **Batch 1 (Research 002):** 21 rules (2 original placeholders + 19 popcorn-distilled)
//!   across 5 categories: memory access, compute, fusion, dispatch, parallelism.
//! - **Batch 2 (Research 003):** 7 flash-attention tiling rules.
//! - **Batch 3 (Research 004):** 6 linear-attention recurrence rules.
//! - **Batch 4 (Research 005):** 9 `MoE` dispatch + backward-pass rules.
//! - **Batch 5 (Research 006):** 4 NSA block-sparse selection rules.
//! - **Batch 6 (Research 007):** 3 varlen packing + chunked reduction rules.
//! - **Batch 7 (Research 008):** 9 Rust-native rules from `burn-cubecl` — autotune
//!   dispatch (panic-safe candidates, multi-priority tune groups, shape-kind
//!   dispatch, vector-width sweep) + conv-as-GEMM + memory layout (1×1 conv as
//!   matmul, contiguity guard, cube-count batch pack, stride-align vectorization,
//!   anchor shape bucketing).
//! - **Batch 8 (Research 009):** 9 burn-cubecl + cubecl-opt rules (kernel
//!   polymorphism + dtype/alignment specialization + IR strength reduction).
//! - **Batch 9 (Research 010):** 8 popcorn rules — fusion (`add_rms_norm`,
//!   `group_norm_linear`, `jsd`) + dtype/quant (`int8_int2_matmul`, `_quant`) + dispatch.
//! - **Batch 10 (Research 011):** 12 popcorn rules — compute (`hadamard_transform`,
//!   `dyt`, `token_shift`, `sparsemax`) + fusion (`selective_log_softmax`) + dispatch
//!   (`softmax`, `rms_norm`) + chunked reduction (`mean_pooling`, `solve_tril`) +
//!   polymorphism (`geglu`, `swiglu`, `rope`) + dtype (`comba`, `gelu`).
//! - **Batch 11 (Research 012):** 11 popcorn rules — linear-attention recurrence
//!   family (`gla`, `simple_gla`, `rwkv7`, `delta_rule`, `iplr_delta_rule`,
//!   `gated_oja_rule`) + parallelism (`chunk_global_cumsum`, `retention`,
//!   `lightning_attn`) + flash-attn tiling (`deltaformer`) + compute (`based`,
//!   `rebased`, `linear_attn`) + dispatch (`lightning_attn`). A 12th candidate
//!   (`chunk-reset-scan`) was dropped as a duplicate of `chunk-reset-cumsum`
//!   (Batch 6). Corpus: **99 rules across 15 categories (A–O)**.
//! - **Batch 12 (Research 013):** 14 popcorn rules — gated delta family
//!   (`gated_delta_rule`, `gated_delta_product`), specialized recurrences
//!   (`mesa_net`, `mesa_net_decode`, `titans_linear`, `ttt`, `rwkv4`, `rwkv6`),
//!   advanced attention (`wall_attn`, `multi_token_attention`), `MoE` gate
//!   (`momoe`), KDA gate (`kda_gate`, `kda_gate_cumsum`), hierarchical mask
//!   (`log_linear_attn`).
//!   Three candidates (`kda` per-key-dim gate, `forgetting_attn` cumsum decay,
//!   `abc` two-state softmax) were dropped as duplicates of existing rules.
//!   Corpus: **113 rules across 15 categories (A–O)**. Category G (linear-attn
//!   recurrence) is now the largest cluster at 17 rules.
//! - **Batch 13 (Research 014):** 6 popcorn rules — final unmined pass. RL loss
//!   stability (`grpo_offpolicy`: `expm1` for KL cancellation), complex-free
//!   `RoPE` (`llama4_rope`: paired reals), multimodal `RoPE` (`qwen2vl_mrope`:
//!   cyclic modality table), `RWKV-7` channel mixing (`rwkv7_channel_mixing`:
//!   stateful cat-prefix shift + squared `ReLU` from `sqrelu`), `RWKV-7` gate output
//!   (`rwkv7_gate_output`: fused reduce-then-broadcast). Two files (`rwkv7_k_update`,
//!   `sigmoid`) yielded no new rule; `tvd` was covered by `upcast-before-reduction`.
//!   Corpus: **119 rules across 15 categories (A–O)**. Category B (compute) is now
//!   the largest cluster at 18 rules.
//! - **Batch 14 (Research 015):** 4 riir-gpu production rules — first batch mining
//!   the stack's own kernels (not external libraries). Norm-fold into matmul
//!   (`gemv_q4k_batched_rmsnorm`: fold `inv_rms` scalar into the GEMV dot),
//!   reduction-split pre-pass (`compute_inv_rms`: scalar pre-pass + fold),
//!   single-plane reduction (Issue 611 fix: one plane suffices), CPU-bottleneck
//!   offload (`gpu_transpose`: GPU compute kernel into pooled memory). Two riir-gpu
//!   files (`lora_fused`, `rope_geglu`) assessed + dropped (`SGMV` overlaps `MoE` grouped,
//!   section-view is pointer arithmetic not an opt). Corpus: **123 rules across 15
//!   categories (A–O)**. Opens riir-gpu (~196K lines) as the next distillation source.
//! - **Batch 15 (Research 016):** 4 riir-gpu production rules — second batch mining
//!   the stack's own kernels. Block-contiguous weight layout (`gemm_ternary_block_contiguous`:
//!   pack 3 separate weight buffers into 1 to cut global-memory accesses 3×), fused
//!   dual GEMV + `GeGLU` (`gemv_geglu_f16`: fuse 2 sibling GEMVs + act into 1 dispatch,
//!   saves 52 dispatches/decode at 26 layers), orthogonal pre-rotation
//!   (`spectralquant/attention.rs`: pre-rotate query once vs each key, ~4096× fewer
//!   rotation FLOPs), water-fill tail-dim shared codebook (`spectralquant/dequant.rs`:
//!   top dims get per-dim codebooks, tail dims share one low-bit codebook, 2.3×
//!   codebook memory reduction). One riir-gpu file (`forward_flashprefill`) assessed
//!   and dropped (cascade structure overlaps `NSA` Category I; 4 staged dispatches
//!   is a non-opt). Corpus: **127 rules across 15 categories (A–O)**. `riir-gpu`
//!   remains open as the next distillation source (5 of ~200 files mined).
//! - **Batch 16 (Research 017):** 4 riir-gpu production rules — third batch mining
//!   the stack's own kernels. `cmma` hardware unit (`gemm_ternary_simdgroup`:
//!   use the cooperative matrix multiply-accumulate hardware unit instead of
//!   scalar ALU — 64 FMAs/cycle on the dedicated unit vs occupancy-bound scalar),
//!   dequant-once-per-K-tile into shared memory (`gemm_ternary_simdgroup`: dequant
//!   a weight tile once, let the hardware matrix unit consume it across all
//!   output rows — m× amortization), swap-large-axis-to-X (`gemm_ternary_simdgroup`:
//!   put the large GEMM axis on the X cube-count for workgroup distribution),
//!   norm-decoupled low-bit scoring (`forward_turboquant`: store per-position L2
//!   norm separately, quantize the unit-normalized key at 2-4 bits, reconstruct
//!   score as norm × dot). Four files assessed + dropped (`lora_fused`: `SGMV`
//!   overlaps Category H `grouped-launch-cumsum-offsets`; `spectralquant/rotation`:
//!   boilerplate, pre-rotate identity already mined; `spectralquant/qjl`: composition
//!   of water-fill with `JL`; `spectralquant/calibration`: two-pass reduce overlaps
//!   `split-reduction-scalar-prepass`). Corpus: **131 rules across 15 categories
//!   (A–O)**. `riir-gpu` remains open as the next distillation source (7 of ~200
//!   files mined).
//! - **Plan 003 (2026-08-14):** router ↔ corpus wiring. `KERNEL_CATEGORIES`
//!   maps each of the 15 corpus categories to its `rule_ids`; the
//!   `KernelExpertRouter` now uses 15 category-keyed experts (one per category)
//!   instead of 4 placeholder identity-vector experts.
//! - **Deferred:** remaining popcorn kernels are mostly already-covered patterns
//!   or domain-specific math; future batches need new source impl files.

use std::collections::HashMap;

use crate::corpus::Corpus;
use crate::domains::kernel_opt::{KernelCategory, KernelOptDomain, KernelRule};

/// Build the kernel optimization corpus from popcorn-distilled rules.
///
/// Returns a BLAKE3-committed [`Corpus<KernelOptDomain>`]. The corpus grows as
/// popcorn distillation batches land (Research 002 is batch 1).
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic — grows with distillation batches
pub fn build_kernel_seed_corpus() -> Corpus<KernelOptDomain> {
    Corpus::new(vec![
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
        // Source: token_shift.py — F.pad(x, (0,0,1,-1)) - x computes y_t = x_{t-1} - x_t\n        // in one fused elementwise op (pad is a view, subtract is one launch).
        KernelRule {
            rule_id: "shift-via-pad-subtract".to_string(),
            gpu_concept: "one-pass sequence shift via pad-and-subtract view".to_string(),
            ref_shape: "// naive: allocate shifted buffer in a first pass, subtract in a second\nfn token_shift_two_pass(x: &[f32], seq: usize, dim: usize) -> Vec<f32> {\n    let mut shifted = vec![0.0; x.len()];\n    for t in 1..seq {\n        for d in 0..dim {\n            shifted[t * dim + d] = x[(t - 1) * dim + d];\n        }\n    }\n    x.iter().zip(shifted.iter()).map(|(a, b)| b - a).collect()\n}".to_string(),
            opt_pattern: "// optimized: pad (view) then subtract — one fused launch\nfn token_shift_fused(x: &[f32], seq: usize, dim: usize) -> Vec<f32> {\n    // logical: shifted[t] = x[t-1], shifted[0] = 0\n    (0..seq).flat_map(|t| {\n        let prev = if t == 0 { vec![0.0; dim] } else { x[((t - 1) * dim)..(t * dim)].to_vec() };\n        (0..dim).map(move |d| prev[d] - x[t * dim + d])\n    }).collect()\n}".to_string(),
        },
        // ── sorted-cumsum-threshold — iterative search → sort+cumsum+threshold ──
        // Source: sparsemax.py — sort x desc, cumsum-1, find max k where k*sorted[k] > cum[k].\n        // Single-pass reduction structure for the support set (vs binary search).
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
            gpu_concept: "residual add + RMS norm fusion".to_string(),
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
        // Source: selective_log_softmax.py — GRPO needs log-probs of selected tokens only.\n        // gather(-1, input_ids) reads O(batch*seq) logits instead of O(batch*seq*vocab).
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
        // Source: softmax.py _transformer_engine_ready (rows%32==0 AND 16<width<16384 AND width%8==0)\n        // + rms_norm.py _aligned (dtype==f32 OR width%8==0). Fast path only when all hold.
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
        KernelRule {
            rule_id: "l2-grouped-tile-schedule".to_string(),
            gpu_concept: "swizzled L2-cache-aware tile mapping".to_string(),
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
        // Source: momoe_tl.py lin1_backward + scatter_cols — concurrent blocks\n        // scatter to overlapping output slots via atomic_add.
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
        // Source: momoe_tl.py autotune decorators — key on Msum (total tokens\n        // across all groups), not per-group shapes, for irregular batches.
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
        // Source: nsa_tl.py _sel_attn_bwd_preprocess_kernel — delta = sum(o*do)\n        // is invariant; precompute once, not per KV block.
        KernelRule {
            rule_id: "precompute-scalar-reduction".to_string(),
            gpu_concept: "hoist invariant row reduction out of per-block loop".to_string(),
            ref_shape: "// naive: recompute the row-wise delta inside every block iteration\nfn bwd_recompute_delta(o: &[f32], do_grad: &[f32], d: usize, block_indices: &[usize], dk: &mut [f32]) {\n    for &blk in block_indices {\n        let mut delta = 0.0f32;\n        for i in 0..d {\n            delta += o[i] * do_grad[i];\n        }\n        dk[blk] += grad_from_delta(delta);\n    }\n}".to_string(),
            opt_pattern: "// optimized: delta precomputed once in separate pass, passed in\nfn precompute_delta(o: &[f32], do_grad: &[f32], d: usize, delta: &mut [f32]) {\n    for row in 0..delta.len() {\n        let base = row * d;\n        delta[row] = (0..d).map(|i| o[base + i] * do_grad[base + i]).sum();\n    }\n}\nfn bwd_with_delta(delta: &[f32], block_indices: &[usize], dk: &mut [f32]) {\n    for (row, &blk) in block_indices.iter().enumerate() {\n        dk[blk] += grad_from_delta(delta[row]);\n    }\n}".to_string(),
        },
        // ── block-compression-before-routing — score full → score compressed ─
        // Source: nsa_tl.py compression_attention + nsa — mean-pool to coarse\n        // blocks, route on compressed reps, attend top-k at full res.
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
        // Source: mean_pooling.py — torch.stack([chunk.mean(1) for chunk in x.split(C, 1)], 1).\n        // Split-then-mean exposes the fixed chunk size for unrollable inner reduction.
        KernelRule {
            rule_id: "chunk-split-mean-pool".to_string(),
            gpu_concept: "chunked reduction via split + per-chunk mean".to_string(),
            ref_shape: "// naive: running sum into output buffer (sequential accumulator)\nfn mean_pool_running(x: &[f32], seq: usize, dim: usize, chunk: usize) -> Vec<f32> {\n    let n_chunks = seq / chunk;\n    let mut out = vec![0.0; n_chunks * dim];\n    for c in 0..n_chunks {\n        for d in 0..dim {\n            let mut acc = 0.0;\n            for t in 0..chunk {\n                acc += x[(c * chunk + t) * dim + d];\n            }\n            out[c * dim + d] = acc / chunk as f32;\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: split into chunks, mean each chunk (exposes chunk structure)\nfn mean_pool_split(x: &[f32], seq: usize, dim: usize, chunk: usize) -> Vec<f32> {\n    let n_chunks = seq / chunk;\n    let mut out = vec![0.0; n_chunks * dim];\n    for c in 0..n_chunks {\n        let base = c * chunk * dim;\n        for d in 0..dim {\n            // inner loop over fixed-size chunk — unrollable\n            let mut acc = 0.0;\n            for t in 0..chunk {\n                acc += x[base + t * dim + d];\n            }\n            out[c * dim + d] = acc / chunk as f32;\n        }\n    }\n    out\n}".to_string(),
        },
        // ── ragged-chunk-pad — branch on tail → pad to block_size (Batch 10, popcorn solve_tril.py) ──
        // Source: solve_tril.py — F.pad(inverse, (0, block_size - length)) before cat.\n        // Unifies the last ragged chunk with the full chunks (one code path).
        KernelRule {
            rule_id: "ragged-chunk-pad".to_string(),
            gpu_concept: "pad ragged tail to block_size for uniform chunk processing".to_string(),
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
            gpu_concept: "panic-resilient autotune candidate".to_string(),
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
            gpu_concept: "matmul-kind-keyed kernel family dispatch".to_string(),
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
            gpu_concept: "stride-contiguity guard skips unnecessary relayout copy".to_string(),
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
        // Source: geglu.py/swiglu.py unsloth backends — kernel expects 3-D (1, -1, hidden).\n        // Adapter reshapes any-rank input to 3-D, runs kernel, reshapes back.\n        // Preserves the caller's rank contract (zero-copy view when contiguous).
        KernelRule {
            rule_id: "reshape-rank-for-backend".to_string(),
            gpu_concept: "rank adapter — reshape to backend-required rank, reshape back".to_string(),
            ref_shape: "// naive: require caller to pass 3-D (leaks backend rank assumption into API)\nfn gelu_gated_3d_required(a: &[f32], b: &[f32], _batch: usize, _seq: usize, hidden: usize) -> Vec<f32> {\n    // caller MUST flatten to (1, -1, hidden) before calling\n    elementwise_gelu_gated(a, b, hidden)\n}".to_string(),
            opt_pattern: "// optimized: adapter reshapes to 3-D, runs kernel, reshapes back\nfn gelu_gated_any_rank(a: &[f32], b: &[f32], batch: usize, seq: usize, hidden: usize) -> Vec<f32> {\n    let n = batch * seq;\n    // reshape (batch, seq, hidden) → (1, n, hidden) — zero-copy view\n    let out = elementwise_gelu_gated(a, b, hidden); // kernel runs on 3-D\n    // reshape (1, n, hidden) → (batch, seq, hidden) — restore caller's rank\n    out // same buffer, caller interprets as (batch, seq, hidden)\n}".to_string(),
        },
        // ── detached-broadcast-insert — materialize broadcast → detach+view (Batch 10, popcorn rope.py) ──
        // Source: rope.py — cos.detach()[:, None] for non-differentiable broadcast.\n        // Detach avoids autograd graph cost; [:, None] inserts broadcast dim (view, no alloc).
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
        // Source: comba.py — q32, k32, v32, p32, g32, beta32 = map(upcast, (q, k, v, p, g, beta)).\n        // One-line batch upcast signals independence to the compiler (parallelizable).
        KernelRule {
            rule_id: "batched-upcast-map".to_string(),
            gpu_concept: "batched type promotion via map over a tuple".to_string(),
            ref_shape: "// naive: per-tensor upcast calls (six separate bindings, six separate calls)\nfn upcast_six_separate(q: &[f16], k: &[f16], v: &[f16], p: &[f16], g: &[f16], beta: &[f16]) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {\n    let q32: Vec<f32> = q.iter().map(|x| x.to_f32()).collect();\n    let k32: Vec<f32> = k.iter().map(|x| x.to_f32()).collect();\n    let v32: Vec<f32> = v.iter().map(|x| x.to_f32()).collect();\n    let p32: Vec<f32> = p.iter().map(|x| x.to_f32()).collect();\n    let g32: Vec<f32> = g.iter().map(|x| x.to_f32()).collect();\n    let beta32: Vec<f32> = beta.iter().map(|x| x.to_f32()).collect();\n    (q32, k32, v32, p32, g32, beta32)\n}".to_string(),
            opt_pattern: "// optimized: map over tuple (signals independence, one structured binding)\nfn upcast_six_mapped(q: &[f16], k: &[f16], v: &[f16], p: &[f16], g: &[f16], beta: &[f16]) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>) {\n    let upcast = |t: &[f16]| t.iter().map(|x| x.to_f32()).collect::<Vec<f32>>();\n    let (q32, k32, v32, p32, g32, beta32) = (upcast(q), upcast(k), upcast(v), upcast(p), upcast(g), upcast(beta));\n    (q32, k32, v32, p32, g32, beta32)\n}".to_string(),
        },
        // ── dtype-restricted-backend — all-dtype backend → precision-safe dtype subset (Batch 10, popcorn gelu.py + sparsemax.py) ──
        // Source: gelu.py fla (Float32|BFloat16, fp16 tanh grads past tol),\n        // sparsemax.py liger (Float32|Float16, bf16 ties break support set).\n        // Encode the dtype restriction in the backend signature, not a runtime check.
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
        // Strictly-causal attention excludes the diagonal (position cannot attend\n        // to itself) via `tril(-1)` instead of the standard `tril()`. Paired
        // with a NaN guard: the first row has no visible keys → softmax of all\n        // -inf → NaN, requiring `nan_to_num` before downstream use.
        KernelRule {
            rule_id: "strict-causal-mask-tril-offset".to_string(),
            gpu_concept: "strictly-causal mask via tril(-1) (exclude diagonal) + NaN guard".to_string(),
            ref_shape: "// naive: standard causal mask tril(0) — position attends to itself (diag included)\nfn attention_causal_diag(scores: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * n];\n    for i in 0..n {\n        for j in 0..n {\n            out[i * n + j] = if j <= i { scores[i * n + j] } else { f32::NEG_INFINITY };\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: strictly-causal tril(-1) (diag excluded) + NaN guard for empty first row\nfn attention_strict_causal(scores: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![f32::NEG_INFINITY; n * n];\n    for i in 0..n {\n        for j in 0..n {\n            // tril(-1): j < i (strictly below the diagonal)\n            if j < i { out[i * n + j] = scores[i * n + j]; }\n        }\n    }\n    // first row is all -inf (no visible keys) → softmax would be NaN; guard it\n    for j in 0..n { out[0 * n + j] = 0.0; } // nan_to_num on empty first row\n    out\n}".to_string(),
        },
        // ── polynomial-taylor-feature-map — softmax kernel → poly feature map ─
        // Source: based.py (`1 + x + 0.5·x²`, 2nd-order Taylor), rebased.py
        // (`x²`, pure quadratic). Approximates the softmax kernel with a
        // low-degree polynomial so linear attention can run without exp. The\n        // Taylor expansion of `exp(q·k)` truncated at degree 2 is the Based\n        // kernel; dropping the constant + linear terms gives ReBased.
        KernelRule {
            rule_id: "polynomial-taylor-feature-map".to_string(),
            gpu_concept: "polynomial Taylor feature map approximating the softmax kernel".to_string(),
            ref_shape: "// naive: full softmax attention (exp over all scores, O(N²) + expensive exp)\nfn attention_softmax(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut scores = vec![0.0f32; n];\n        let mut max_s = f32::NEG_INFINITY;\n        for j in 0..n {\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * k[j*d+dd]; }\n            scores[j] = s; max_s = max_s.max(s);\n        }\n        let mut sum_e = 0.0;\n        for j in 0..n { scores[j] = (scores[j] - max_s).exp(); sum_e += scores[j]; }\n        for j in 0..n {\n            for dd in 0..d { out[i*d+dd] += scores[j] / sum_e * v[j*d+dd]; }\n        }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: polynomial feature map (no exp) — Based kernel: 1 + x + 0.5·x²\nfn attention_poly_feature(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut norm = 0.0; // running normalizer Σ a_ts\n        for j in 0..=i { // causal\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * k[j*d+dd]; }\n            let a = 1.0 + s + 0.5 * s * s; // 2nd-order Taylor of exp(s)\n            for dd in 0..d { out[i*d+dd] += a * v[j*d+dd]; }\n            norm += a;\n        }\n        for dd in 0..d { out[i*d+dd] /= norm + 1e-6; } // normalize\n    }\n    out\n}".to_string(),
        },
        // ── positive-feature-map-shift — raw q/k → elu+1 non-negative shift ───
        // Source: linear_attn.py — q and k pass through `elu(x) + 1` so the\n        // normalizer `Σ k` never crosses zero. Without the shift, signed q/k\n        // can produce a zero or negative normalizer, destabilizing the linear\n        // attention recurrence. The +1 is the minimal shift making elu ≥ 0\n        // strictly positive.
        KernelRule {
            rule_id: "positive-feature-map-shift".to_string(),
            gpu_concept: "shift activation positive (elu+1) so linear-attention normalizer never crosses zero".to_string(),
            ref_shape: "// naive: raw q/k (signed) — normalizer Σ k can be zero/negative, destabilizing\nfn linear_attn_raw(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut norm = 0.0;\n        for j in 0..=i {\n            let mut s = 0.0;\n            for dd in 0..d { s += q[i*d+dd] * k[j*d+dd]; }\n            for dd in 0..d { out[i*d+dd] += s * v[j*d+dd]; }\n            norm += s; // can be zero/negative\n        }\n        for dd in 0..d { out[i*d+dd] /= norm + 1e-10; }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: elu(x)+1 feature map guarantees non-negative q/k → stable normalizer\nfn linear_attn_positive(q: &[f32], k: &[f32], v: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let map = |x: f32| -> f32 { if x > 0.0 { x + 1.0 } else { x.exp() + 1.0 } }; // elu+1\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        let mut norm = 0.0;\n        for j in 0..=i {\n            let mut s = 0.0;\n            for dd in 0..d { s += map(q[i*d+dd]) * map(k[j*d+dd]); }\n            for dd in 0..d { out[i*d+dd] += s * v[j*d+dd]; }\n            norm += s; // strictly positive (map ≥ 1)\n        }\n        for dd in 0..d { out[i*d+dd] /= norm + 1e-10; }\n    }\n    out\n}".to_string(),
        },
        // ── layer-depth-decay-schedule — constant slope → depth-varying slope ──
        // Source: lightning_attn.py — `slope = -(8/heads * (1 - layer_idx/
        // num_layers)) * arange(heads)`. The decay slope scales with layer\n        // depth: deeper layers (layer_idx → num_layers) decay slower\n        // (slope → 0), shallower layers decay faster. A dispatch decision\n        // baked into kernel parameters via the layer index — not a runtime\n        // branch but a parameter specialization.
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
        // Source: rwkv7_channel_mixing.py — token-shift `previous = cat((x_prev[:,\n        // None], x[:, :-1]), dim=1)` reconstructs the shifted sequence by
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
        // Source: sqrelu.py (+ rwkv7_channel_mixing.py consumer) — `relu(x).square()`\n        // preserves ReLU's exact sparsity (zeros stay zero) while quadratically\n        // sharpening non-zero values. Used in Primer (So et al. 2021) + RWKV-7
        // channel mixing. Distinct from `gelu`/`swish` (smooth activations): the\n        // squaring is applied AFTER the relu gate, so the sparse structure is exact.\n        // The fused form `max(x,0)²` avoids materializing the intermediate relu\n        // output before squaring.
        KernelRule {
            rule_id: "squared-relu-activation".to_string(),
            gpu_concept: "relu(x)² — sparse activation: preserves ReLU zeros, sharpens non-zeros quadratically".to_string(),
            ref_shape: "// naive: relu then separate square (two passes, materialized intermediate)\nfn sqrelu_two_pass(x: &[f32], n: usize) -> Vec<f32> {\n    let mut relu_out = vec![0.0f32; n];\n    for i in 0..n { relu_out[i] = if x[i] > 0.0 { x[i] } else { 0.0 }; } // pass 1\n    let mut out = vec![0.0f32; n];\n    for i in 0..n { out[i] = relu_out[i] * relu_out[i]; } // pass 2\n    out\n}".to_string(),
            opt_pattern: "// optimized: fused squared ReLU (sqrelu) — one pass, no intermediate\nfn sqrelu_fused(x: &[f32], n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n];\n    for i in 0..n {\n        let v = if x[i] > 0.0 { x[i] } else { 0.0 }; // relu gate\n        out[i] = v * v; // square inline — zeros stay zero, non-zeros sharpen\n    }\n    out\n}".to_string(),
        },
        // ── rkv-receptance-correction — reduce-then-broadcast-multiply fused (C) ──
        // Source: rwkv7_gate_output.py — `correction = (r*k*r_k).sum(-1, keepdim=\n        // true) * v`, then `y = (o + correction) * g`. A reduce-then-broadcast-multiply\n        // fusion: elementwise triple-product (r·k·ρ), reduce over head_dim with\n        // `keepdim` (broadcastable scalar per head), multiply by v (broadcasts back\n        // to full head_dim), add to o, gate by g. Four elementwise ops + one reduce\n        // fused into a single kernel pass — the `keepdim` makes the reduce output\n        // directly broadcastable without a reshape. Distinct from `gather-selective-\n        // reduction` (that's a gather-then-reduce; this is a reduce-then-broadcast).
        KernelRule {
            rule_id: "rkv-receptance-correction".to_string(),
            gpu_concept: "fused reduce-then-broadcast: (r·k·ρ).sum(head_dim,keepdim)·v added to o, gated by g".to_string(),
            ref_shape: "// naive: 4 separate passes — triple-mul, reduce, broadcast-mul, add+gate\nfn rkv_correction_naive(o: &[f32], r: &[f32], k: &[f32], r_k: &[f32], v: &[f32], g: &[f32], d: usize, n: usize) -> Vec<f32> {\n    // pass 1: elementwise r*k*r_k (materialized)\n    let mut triple = vec![0.0f32; n * d];\n    for i in 0..(n*d) { triple[i] = r[i] * k[i] * r_k[i]; }\n    // pass 2: reduce over head_dim (materialized scalar per head)\n    let mut reduced = vec![0.0f32; n];\n    for i in 0..n { for dd in 0..d { reduced[i] += triple[i*d+dd]; } }\n    // pass 3: broadcast-multiply by v (materialized)\n    let mut correction = vec![0.0f32; n * d];\n    for i in 0..n { for dd in 0..d { correction[i*d+dd] = reduced[i] * v[i*d+dd]; } }\n    // pass 4: add to o, gate by g\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..(n*d) { out[i] = (o[i] + correction[i]) * g[i]; }\n    out\n}".to_string(),
            opt_pattern: "// optimized: fused reduce-then-broadcast (rwkv7_gate_output) — one pass\nfn rkv_correction_fused(o: &[f32], r: &[f32], k: &[f32], r_k: &[f32], v: &[f32], g: &[f32], d: usize, n: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; n * d];\n    for i in 0..n {\n        // reduce over head_dim: dot(r*k*r_k, all dims) — keepdim = scalar per head\n        let mut s = 0.0;\n        for dd in 0..d { s += r[i*d+dd] * k[i*d+dd] * r_k[i*d+dd]; }\n        // broadcast-multiply by v, add to o, gate by g — all inline, no intermediate\n        for dd in 0..d {\n            out[i*d+dd] = (o[i*d+dd] + s * v[i*d+dd]) * g[i*d+dd];\n        }\n    }\n    out\n}".to_string(),
        },
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
            gpu_concept: "fuse two sibling GEMVs (gate + up projections) + their multiplicative activation into one kernel dispatch".to_string(),
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
            gpu_concept: "dequantize a weight tile once into shared memory, let the hardware matrix unit consume it across all output rows that share the tile".to_string(),
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
    ])
}

/// The category → `rule_ids` mapping — the corpus wiring for [`super::router::KernelExpertRouter`].
///
/// One entry per [`KernelCategory`] (15 total). Each entry carries the category's
/// display name + the static list of `rule_ids` that belong to it. This is the
/// **router→corpus link**: given a routed expert index, a heal pipeline can look
/// up the category + its `rule_ids` without scanning the full corpus.
///
/// # Plan 003 invariants (enforced by tests)
///
/// - **Exhaustive:** every `rule_id` in the 131-rule corpus appears in exactly one
///   category (no orphan rules, no duplicate appearances).
/// - **Order-stable:** categories appear in enum-discriminant order (A=0 … O=14).
/// - **Count-stable:** the per-category counts are pinned by tests.
///
/// See [Plan 003](../../../.plans/003_kernel_opt_router_corpus_wiring.md) for the
/// design rationale (one expert per category, not one per rule).
pub const KERNEL_CATEGORIES: [(KernelCategory, &str, &[&str]); 15] = [
    // A: Memory access patterns (9 rules)
    (
        KernelCategory::MemoryAccess,
        "A: memory access",
        &[
            "matvec-coalesce",
            "coalesced-row-read",
            "vectorized-pack-load",
            "grid-stride-loop",
            "gemm-tiling",
            "masked-boundary-load",
            "offload-cpu-bottleneck-to-gpu-kernel",
            "block-contiguous-weight-layout",
            "dequant-once-per-k-tile-into-shared-memory",
        ],
    ),
    // B: Compute patterns (20 rules)
    (
        KernelCategory::Compute,
        "B: compute",
        &[
            "autotune-warps-stages",
            "upcast-before-reduction",
            "branchless-select",
            "unroll-fixed-inner",
            "forceinline-device-fn",
            "recursive-kronecker-cat",
            "optional-bias-branch",
            "shift-via-pad-subtract",
            "sorted-cumsum-threshold",
            "polynomial-taylor-feature-map",
            "positive-feature-map-shift",
            "hierarchical-log2-level-mask",
            "iterative-cg-solve-over-matvec",
            "chunked-test-time-gradient-descent",
            "expm1-for-near-zero-exp-minus-one",
            "complex-mul-via-paired-reals",
            "stateful-shift-via-cat-prefix",
            "squared-relu-activation",
            "pre-rotate-once-exploit-orthogonality",
            "cmma-hardware-unit-over-scalar-alu",
        ],
    ),
    // C: Fusion patterns (12 rules)
    (
        KernelCategory::Fusion,
        "C: fusion",
        &[
            "fuse-norm-act-linear",
            "fuse-quant-matmul",
            "fuse-softmax-mask",
            "fuse-xent-logsumexp",
            "fuse-add-rms-norm",
            "fuse-group-norm-linear",
            "fuse-jsd-logsumexp",
            "gather-selective-reduction",
            "fuse-gate-transform-chunk-cumsum",
            "rkv-receptance-correction",
            "fold-norm-scalar-into-matmul",
            "fuse-gate-up-gemv-activation",
        ],
    ),
    // D: Dispatch patterns (9 rules)
    (
        KernelCategory::Dispatch,
        "D: dispatch",
        &[
            "sm-count-block-size",
            "predicate-gated-backend",
            "pow2-block-alignment",
            "predicate-gate-div512",
            "multi-condition-predicate",
            "layer-depth-decay-schedule",
            "dual-mode-gate-transform",
            "cyclic-modality-table-selection",
            "swap-large-axis-to-x-for-workgroup-distribution",
        ],
    ),
    // E: Parallelism patterns (8 rules)
    (
        KernelCategory::Parallelism,
        "E: parallelism",
        &[
            "multi-dim-grid-launch",
            "block-count-clamp",
            "backward-as-conjugate",
            "reverse-scan-via-double-flip",
            "pairwise-distance-decay-matrix",
            "softmax-topk-renormalize-scatter",
            "split-reduction-scalar-prepass",
            "single-plane-sufficient-reduction",
        ],
    ),
    // F: Flash-attention tiling (10 rules)
    (
        KernelCategory::FlashAttnTiling,
        "F: flash-attn tiling",
        &[
            "online-softmax-streaming",
            "exp2-log2-base2",
            "block-pointer-2d",
            "heuristic-compile-dispatch",
            "causal-diag-offdiag-split",
            "shared-mem-aware-tile",
            "kv-block-streaming",
            "strict-causal-mask-tril-offset",
            "block-local-reference-frame-for-overflow",
            "two-stage-mask-softmax-then-conv",
        ],
    ),
    // G: Linear-attention recurrence (17 rules)
    (
        KernelCategory::LinearAttnRecurrence,
        "G: linear-attn recurrence",
        &[
            "state-forget-gate",
            "householder-rank1-update",
            "chunkwise-parallel-recurrence",
            "recurrent-vs-chunk-dispatch",
            "multi-write-per-step",
            "chained-two-pass-recurrence",
            "per-key-dim-diagonal-gate",
            "erase-replace-low-rank-transition",
            "oja-subtractive-normalization",
            "identity-plus-low-rank-delta",
            "cumsum-gate-to-decay-matrix",
            "gate-then-correct-delta-rule",
            "gate-then-multi-householder-write",
            "online-gram-ridge-solve",
            "momentum-gated-memory-recurrence",
            "current-token-bonus-read-before-update",
            "wkv-logsumexp-three-channel-state",
        ],
    ),
    // H: MoE dispatch + backward pass (9 rules)
    (
        KernelCategory::MoeDispatchBackward,
        "H: MoE dispatch + backward",
        &[
            "grouped-launch-cumsum-offsets",
            "l2-grouped-tile-schedule",
            "fused-matmul-activation",
            "fused-backward-chain",
            "activation-recompute-budget",
            "atomic-scatter-reduction",
            "weight-grad-batched-outer-product",
            "csr-index-via-prefix-sum",
            "autotune-aggregate-workload-key",
        ],
    ),
    // I: NSA block-sparse selection (4 rules)
    (
        KernelCategory::NsaBlockSparse,
        "I: NSA block-sparse",
        &[
            "block-sparse-gather",
            "precompute-scalar-reduction",
            "block-compression-before-routing",
            "sliding-window-plus-sparse",
        ],
    ),
    // J: Varlen packing + chunked reductions (5 rules)
    (
        KernelCategory::VarlenChunkedReduction,
        "J: varlen + chunked reduction",
        &[
            "varlen-packing",
            "chunk-reset-cumsum",
            "cumsum-additive-bias",
            "chunk-split-mean-pool",
            "ragged-chunk-pad",
        ],
    ),
    // K: Autotune dispatch (4 rules)
    (
        KernelCategory::AutotuneDispatch,
        "K: autotune dispatch",
        &[
            "panic-safe-autotune-candidate",
            "multi-priority-tune-groups",
            "shape-kind-dispatch",
            "vector-width-pow2-sweep",
        ],
    ),
    // L: Conv-as-GEMM + memory layout (5 rules)
    (
        KernelCategory::ConvGemmLayout,
        "L: conv-as-GEMM + layout",
        &[
            "conv-1x1-as-matmul",
            "contiguity-guard-before-reshape",
            "cube-count-bounded-batch-pack",
            "stride-align-vectorization-key",
            "anchor-shape-bucketing",
        ],
    ),
    // M: Kernel parametric polymorphism (5 rules)
    (
        KernelCategory::KernelPolymorphism,
        "M: kernel polymorphism",
        &[
            "op-family-type-dispatch",
            "inplace-output-alias",
            "layout-permute-for-coalescing",
            "reshape-rank-for-backend",
            "detached-broadcast-insert",
        ],
    ),
    // N: dtype + alignment specialization (10 rules)
    (
        KernelCategory::DtypeAlignment,
        "N: dtype + alignment",
        &[
            "dtype-specialized-float-kind",
            "alignment-gated-zero-copy-view",
            "int2-pack-weights",
            "per-token-activation-quant",
            "ternary-weight-quant",
            "upcast-before-loss",
            "batched-upcast-map",
            "dtype-restricted-backend",
            "water-fill-tail-dim-shared-codebook",
            "norm-decoupled-low-bit-scoring",
        ],
    ),
    // O: IR strength reduction (4 rules)
    (
        KernelCategory::IrStrengthReduction,
        "O: IR strength reduction",
        &[
            "strength-reduce-mul-pow2",
            "strength-reduce-mul-pow2-plus-minus-one",
            "strength-reduce-div-pow2",
            "strength-reduce-mod-pow2",
        ],
    ),
];

/// Group the flat corpus entries by [`KernelCategory`].
///
/// Returns 15 buckets (one per category), each containing the `KernelRule`s whose
/// `rule_id` appears in that category's slice of [`KERNEL_CATEGORIES`]. This is
/// the category-scoped view of the corpus — a future heal pipeline can consult
/// the routed expert's rules first without scanning all 127.
///
/// # Allocation
///
/// Allocates the grouped `Vec` (one per call). This is NOT a hot-path function
/// — it's a corpus-construction helper, called once at boot or test setup. The
/// per-category rule consultation is `O(rules_in_category)` via the static
/// `KERNEL_CATEGORIES` slices.
#[must_use]
pub fn build_corpus_categorized() -> Vec<(KernelCategory, Vec<KernelRule>)> {
    let corpus = build_kernel_seed_corpus();
    // Index rule_id → entry for O(1) category-bucket lookup.
    let by_id: HashMap<&str, &KernelRule> = corpus
        .entries
        .iter()
        .map(|e| (e.rule_id.as_str(), e))
        .collect();
    KERNEL_CATEGORIES
        .iter()
        .map(|(cat, _label, rule_ids)| {
            let rules: Vec<KernelRule> = rule_ids
                .iter()
                .filter_map(|rid| by_id.get(*rid).copied().cloned())
                .collect();
            (*cat, rules)
        })
        .collect()
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
        // Batch 4 (9): MoE dispatch + backward pass.
        // Batch 5 (4): NSA block-sparse selection.
        // Batch 6 (3): varlen packing + chunked reductions.
        // Batch 7 (9): burn-cubecl autotune dispatch + conv-as-GEMM + layout.
        // Batch 8 (9): burn-cubecl kernel polymorphism + dtype/alignment + cubecl-opt IR strength reduction.
        // Batch 9 (8): popcorn fusion + dtype/quant + dispatch (add_rms_norm, group_norm_linear, jsd, int8_int2_matmul, _quant).
        // Batch 10 (12): popcorn compute + chunked reduction + polymorphism (mean_pooling, solve_tril, hadamard_transform, comba, dyt, token_shift, sparsemax, selective_log_softmax, softmax, rms_norm, geglu, swiglu, rope, gelu).
        // Batch 11 (11): popcorn linear-attention recurrence family (gla, simple_gla, based, rebased, linear_attn, retention, lightning_attn, rwkv7, delta_rule, iplr_delta_rule, gated_oja_rule, deltaformer, chunk_global_cumsum, chunk_local_cumsum).
        // Batch 12 (14): popcorn gated delta + specialized recurrence + advanced attention (gated_delta_rule, gated_delta_product, mesa_net, mesa_net_decode, titans_linear, ttt, rwkv4, rwkv6, wall_attn, multi_token_attention, momoe, kda_gate, kda_gate_cumsum, log_linear_attn).
        // Batch 13 (6): popcorn final pass — RL loss stability (grpo_offpolicy), complex-free RoPE (llama4_rope), multimodal RoPE (qwen2vl_mrope), RWKV-7 channel mixing + gate output (rwkv7_channel_mixing, rwkv7_gate_output, sqrelu).
        // Batch 14 (4): riir-gpu production kernels — norm-fold into matmul, reduction-split pre-pass, single-plane reduction, CPU-bottleneck offload (gemv_q4k_batched_rmsnorm_cubecl, gpu_transpose).
        // Batch 15 (4): riir-gpu production kernels — block-contiguous weight layout, fused dual GEMV + GeGLU, orthogonal pre-rotation, water-fill tail-dim shared codebook (gemm_ternary_block_contiguous, gemv_geglu_f16, spectralquant).
        // Batch 16 (4): riir-gpu production kernels — cmma hardware unit, dequant-once-per-k-tile, dispatch axis swap, norm-decoupled low-bit scoring (gemm_ternary_simdgroup, forward_turboquant).
        // Total: 131.
        assert_eq!(corpus.entries.len(), 131, "batches 1-16 should have 131 rules");
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

    #[test]
    fn batch4_rules_present() {
        // Verify the 9 batch-4 MoE dispatch + backward-pass rules are in the corpus.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "grouped-launch-cumsum-offsets",
            "l2-grouped-tile-schedule",
            "fused-matmul-activation",
            "fused-backward-chain",
            "activation-recompute-budget",
            "atomic-scatter-reduction",
            "weight-grad-batched-outer-product",
            "csr-index-via-prefix-sum",
            "autotune-aggregate-workload-key",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-4 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch4_moe_backward_concepts_covered() {
        // Verify the MoE dispatch + backward-pass concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"grouped GEMM with prefix-sum offsets"));
        assert!(concepts.contains(&"swizzled L2-cache-aware tile mapping"));
        assert!(concepts.contains(&"pointwise activation in matmul epilogue"));
        assert!(concepts.contains(&"two backward ops fused into one kernel"));
        assert!(concepts.contains(&"partial activation save with in-kernel recompute"));
        assert!(concepts.contains(&"lock-free atomic scatter for parallel writes"));
        assert!(concepts.contains(&"weight gradients as batched outer product GEMM"));
        assert!(concepts.contains(&"GPU-side CSR sparse layout via prefix sum"));
        assert!(concepts.contains(&"autotune keyed on aggregate workload size"));
    }

    #[test]
    fn batch5_rules_present() {
        // Verify the 4 batch-5 NSA block-sparse selection rules are in the corpus.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "block-sparse-gather",
            "precompute-scalar-reduction",
            "block-compression-before-routing",
            "sliding-window-plus-sparse",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-5 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch5_nsa_concepts_covered() {
        // Verify the NSA block-sparse selection concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"block-sparse indexed gather via selection array"));
        assert!(concepts.contains(&"hoist invariant row reduction out of per-block loop"));
        assert!(concepts.contains(&"pool to coarse blocks, route on compressed reps"));
        assert!(concepts.contains(&"decompose attention into local + global sparse, gated sum"));
    }

    #[test]
    fn batch6_rules_present() {
        // Verify the 3 batch-6 varlen + chunked-reduction rules are in the corpus.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "varlen-packing",
            "chunk-reset-cumsum",
            "cumsum-additive-bias",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-6 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch6_varlen_chunked_concepts_covered() {
        // Verify the varlen + chunked-reduction concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"pack variable-length sequences via cu_seqlens, no padding"));
        assert!(concepts.contains(&"cumsum that resets at chunk boundaries"));
        assert!(concepts.contains(&"additive bias via cumulative sum of a gate"));
    }

    #[test]
    fn batch7_rules_present() {
        // Verify the 9 batch-7 burn-cubecl autotune dispatch + conv/layout rules.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "panic-safe-autotune-candidate",
            "multi-priority-tune-groups",
            "shape-kind-dispatch",
            "vector-width-pow2-sweep",
            "conv-1x1-as-matmul",
            "contiguity-guard-before-reshape",
            "cube-count-bounded-batch-pack",
            "stride-align-vectorization-key",
            "anchor-shape-bucketing",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-7 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch7_burn_cubecl_concepts_covered() {
        // Verify the burn-cubecl autotune + conv/layout concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        assert!(concepts.contains(&"panic-resilient autotune candidate"));
        assert!(concepts.contains(&"shape-keyed priority-ranked autotune groups"));
        assert!(concepts.contains(&"matmul-kind-keyed kernel family dispatch"));
        assert!(concepts.contains(&"vectorization width as an autotune dimension"));
        assert!(concepts.contains(&"1x1 convolution as direct matmul (skip im2col)"));
        assert!(concepts.contains(&"stride-contiguity guard skips unnecessary relayout copy"));
        assert!(concepts.contains(&"grid-limit-aware batch packing"));
        assert!(concepts.contains(&"stride-byte-alignment determines vectorization width"));
        assert!(concepts.contains(&"shape bucketing for autotune cache reuse"));
    }

    #[test]
    fn batch8_rules_present() {
        // Verify the 9 batch-8 kernel polymorphism + dtype/alignment + IR strength-reduction rules.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        for expected in [
            "op-family-type-dispatch",
            "inplace-output-alias",
            "layout-permute-for-coalescing",
            "dtype-specialized-float-kind",
            "alignment-gated-zero-copy-view",
            "strength-reduce-mul-pow2",
            "strength-reduce-mul-pow2-plus-minus-one",
            "strength-reduce-div-pow2",
            "strength-reduce-mod-pow2",
        ] {
            assert!(
                ids.contains(&expected),
                "batch-8 rule '{expected}' missing from corpus"
            );
        }
    }

    #[test]
    fn batch8_polymorphism_dtype_strength_concepts_covered() {
        // Verify the 3 new category concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category M: kernel parametric polymorphism
        assert!(concepts.contains(&"parametric kernel polymorphism over op trait"));
        assert!(concepts.contains(&"output buffer reuse via mutability check (skip allocation)"));
        assert!(concepts.contains(&"memory layout permutation for coalesced channel access"));
        // Category N: dtype + alignment specialization
        assert!(concepts.contains(&"dtype-keyed intrinsic dispatch for low-precision floats"));
        assert!(concepts.contains(&"alignment-gated zero-copy slice view via handle offset"));
        // Category O: IR strength reduction
        assert!(concepts.contains(&"IR strength reduction: multiply by power of 2 → left shift"));
        assert!(concepts.contains(&"IR strength reduction: multiply by (2^k ± 1) → shift ± self"));
        assert!(concepts.contains(&"IR strength reduction: divide by power of 2 → right shift"));
        assert!(concepts.contains(&"IR strength reduction: modulo by power of 2 → bitwise and mask"));
    }

    // ════════════════════════════════════════════════════════════════════
    // Batch 11 — popcorn linear-attention recurrence family (12 rules)
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn batch11_rules_present() {
        // Verify the 12 batch-11 linear-attention recurrence + scan rules.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // G: linear-attn recurrence (+5)
        assert!(ids.contains(&"per-key-dim-diagonal-gate"), "gla per-dim gate");
        assert!(ids.contains(&"erase-replace-low-rank-transition"), "rwkv7 4-phase");
        assert!(ids.contains(&"oja-subtractive-normalization"), "gated_oja_rule");
        assert!(ids.contains(&"identity-plus-low-rank-delta"), "iplr_delta_rule");
        assert!(ids.contains(&"cumsum-gate-to-decay-matrix"), "simple_gla pairwise");
        // E: parallelism (+2)
        assert!(ids.contains(&"reverse-scan-via-double-flip"), "chunk_*_cumsum reverse");
        assert!(ids.contains(&"pairwise-distance-decay-matrix"), "retention/lightning decay");
        // F: flash-attn tiling (+1)
        assert!(ids.contains(&"strict-causal-mask-tril-offset"), "deltaformer tril(-1)");
        // B: compute (+2)
        assert!(ids.contains(&"polynomial-taylor-feature-map"), "based/rebased poly");
        assert!(ids.contains(&"positive-feature-map-shift"), "linear_attn elu+1");
        // D: dispatch (+1)
        assert!(ids.contains(&"layer-depth-decay-schedule"), "lightning_attn depth");
        // (J chunk-reset-scan was a duplicate of chunk-reset-cumsum — dropped.)
    }

    #[test]
    fn batch11_recurrence_concepts_covered() {
        // Verify the batch-11 recurrence + scan concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category G: linear-attention recurrence (new)
        assert!(concepts.contains(&"per-dimension diagonal decay gate on recurrent state"));
        assert!(concepts.contains(&"diagonal-plus-low-rank state transition (erase, decay, replace, write)"));
        assert!(concepts.contains(&"Oja subtractive normalization: project out stored component before write"));
        assert!(concepts.contains(&"identity-plus-low-rank (I+ba^T) state rotation then outer-product write"));
        assert!(concepts.contains(&"convert per-step log-gate into pairwise decay matrix via cumsum + diff"));
        // Category E: parallelism (new)
        assert!(concepts.contains(&"reverse-direction scan via flip → forward scan → flip"));
        assert!(concepts.contains(&"materialize N×N decay matrix from index distance and slope"));
        // Category F: flash-attn tiling (new)
        assert!(concepts.contains(&"strictly-causal mask via tril(-1) (exclude diagonal) + NaN guard"));
        // Category B: compute (new)
        assert!(concepts.contains(&"polynomial Taylor feature map approximating the softmax kernel"));
        assert!(concepts.contains(&"shift activation positive (elu+1) so linear-attention normalizer never crosses zero"));
        // Category D: dispatch (new)
        assert!(concepts.contains(&"layer-depth-dependent decay schedule (deeper layers decay slower)"));
    }

    // ══════════════════════════════════════════════════════════════════
    // Batch 12 (Research 013) — gated delta + specialized recurrence + advanced attention
    // ══════════════════════════════════════════════════════════════════

    #[test]
    fn batch12_rules_present() {
        // Verify the 14 batch-12 rules across G/B/F/E/C/D.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // G: linear-attn recurrence (+6)
        assert!(ids.contains(&"gate-then-correct-delta-rule"), "gated_delta_rule");
        assert!(ids.contains(&"gate-then-multi-householder-write"), "gated_delta_product");
        assert!(ids.contains(&"online-gram-ridge-solve"), "mesa_net ridge");
        assert!(ids.contains(&"momentum-gated-memory-recurrence"), "titans_linear momentum");
        assert!(ids.contains(&"current-token-bonus-read-before-update"), "rwkv6 bonus");
        assert!(ids.contains(&"wkv-logsumexp-three-channel-state"), "rwkv4 3ch");
        // B: compute (+3)
        assert!(ids.contains(&"hierarchical-log2-level-mask"), "log_linear_attn hier");
        assert!(ids.contains(&"iterative-cg-solve-over-matvec"), "mesa_net_decode cg");
        assert!(ids.contains(&"chunked-test-time-gradient-descent"), "ttt chunked gd");
        // F: flash-attn tiling (+2)
        assert!(ids.contains(&"block-local-reference-frame-for-overflow"), "wall_attn reframe");
        assert!(ids.contains(&"two-stage-mask-softmax-then-conv"), "multi_token_attention 2stage");
        // E: parallelism (+1)
        assert!(ids.contains(&"softmax-topk-renormalize-scatter"), "momoe gate");
        // C: fusion (+1)
        assert!(ids.contains(&"fuse-gate-transform-chunk-cumsum"), "kda_gate_cumsum fused");
        // D: dispatch (+1)
        assert!(ids.contains(&"dual-mode-gate-transform"), "kda_gate dual-mode");
    }

    #[test]
    fn batch12_recurrence_attention_concepts_covered() {
        // Verify the batch-12 recurrence + attention + gate concepts are represented.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category G: linear-attn recurrence (new)
        assert!(concepts.contains(&"gated delta rule: decay state then correct against decayed subspace"));
        assert!(concepts.contains(&"one outer gate wrapping N inner Householder delta writes per step"));
        assert!(concepts.contains(&"online ridge regression: gated Gram accumulators + per-step solve"));
        assert!(concepts.contains(&"two coupled recurrences: SGD momentum state + EMA memory matrix"));
        assert!(concepts.contains(&"read from boosted state (transient bonus), then update with un-boosted kv"));
        assert!(concepts.contains(&"WKV with 3-channel state (num/den/max) + logsumexp max-subtraction"));
        // Category B: compute (new)
        assert!(concepts.contains(&"O(log seq) hierarchical mask with per-level learned scales"));
        assert!(concepts.contains(&"conjugate-gradient solve over matvec with Jacobi preconditioner (no inverse)"));
        assert!(concepts.contains(&"mini-batch chunked GD with causal mask + last-step weight update"));
        // Category F: flash-attn tiling (new)
        assert!(concepts.contains(&"per-block reference frame to keep exp arguments bounded (avoid fp32 overflow)"));
        assert!(concepts.contains(&"two-stage mask: -inf before softmax, 0 after conv (probability + causal correct)"));
        // Category E: parallelism (new)
        assert!(concepts.contains(&"MoE gate: softmax → topk → renormalize → scatter sparse gate matrix"));
        // Category C: fusion (new)
        assert!(concepts.contains(&"fuse gate transform with chunk-local cumsum (no materialized intermediate)"));
        // Category D: dispatch (new)
        assert!(concepts.contains(&"dual-mode gate: unbounded softplus OR bounded sigmoid, dispatched by parameter"));
    }

    #[test]
    fn batch13_rules_present() {
        // Verify the 6 batch-13 rules across B/C/D.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category B (4 new): numerical stability, complex-free RoPE, stateful shift, squared ReLU
        assert!(ids.contains(&"expm1-for-near-zero-exp-minus-one"));
        assert!(ids.contains(&"complex-mul-via-paired-reals"));
        assert!(ids.contains(&"stateful-shift-via-cat-prefix"));
        assert!(ids.contains(&"squared-relu-activation"));
        // Category C (1 new): fused reduce-then-broadcast
        assert!(ids.contains(&"rkv-receptance-correction"));
        // Category D (1 new): multimodal cyclic table gather
        assert!(ids.contains(&"cyclic-modality-table-selection"));
    }

    #[test]
    fn batch13_stability_rope_activation_concepts_covered() {
        // Verify the batch-13 numerical stability + RoPE + activation concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category B: numerical stability (expm1 for KL cancellation)
        assert!(concepts.contains(&"numerically stable e^x − 1 via expm1 (avoids catastrophic cancellation near 0)"));
        // Category B: complex-free RoPE (paired reals)
        assert!(concepts.contains(&"represent complex numbers as interleaved [real, imag] pairs, multiply via 4 scalar muls"));
        // Category B: stateful shift (cross-batch carry)
        assert!(concepts.contains(&"shift sequence with cross-batch carry: cat(prev_state, x[:-1])"));
        // Category B: squared ReLU (sparse sharpening)
        assert!(concepts.contains(&"relu(x)² — sparse activation: preserves ReLU zeros, sharpens non-zeros quadratically"));
        // Category C: fused reduce-then-broadcast (RWKV-7 gate output correction)
        assert!(concepts.contains(&"fused reduce-then-broadcast: (r·k·ρ).sum(head_dim,keepdim)·v added to o, gated by g"));
        // Category D: multimodal cyclic table selection
        assert!(concepts.contains(&"multimodal RoPE: split head_dim into sections, cyclic gather from N modality tables"));
    }

    // ════════════════════════════════════════════════════════════════════
    // Batch 14 (Research 015) — riir-gpu production kernels: norm-fold + reduction-split + single-plane + CPU-offload
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn batch14_rules_present() {
        // Verify the 4 batch-14 rules across C/E/A.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category C (1 new): fold norm scalar into matmul
        assert!(ids.contains(&"fold-norm-scalar-into-matmul"), "fold norm into gemv");
        // Category E (2 new): reduction-split pre-pass + single-plane reduction
        assert!(ids.contains(&"split-reduction-scalar-prepass"), "scalar pre-pass");
        assert!(ids.contains(&"single-plane-sufficient-reduction"), "single plane");
        // Category A (1 new): offload CPU bottleneck to GPU kernel
        assert!(ids.contains(&"offload-cpu-bottleneck-to-gpu-kernel"), "cpu offload");
    }

    #[test]
    fn batch14_norm_fold_reduction_split_offload_concepts_covered() {
        // Verify the batch-14 norm-fold + reduction-split + single-plane + offload concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category C: fold norm scalar into matmul (riir-gpu gemv_q4k_rmsnorm)
        assert!(concepts.contains(&"fold normalization scalar (inv_rms) into the matmul dot product — norm becomes 2 inline multiplies"));
        // Category E: split reduction into scalar pre-pass (riir-gpu compute_inv_rms)
        assert!(concepts.contains(&"split a reduction whose output is a tiny scalar into a pre-pass, fold the scalar into the main consumer"));
        // Category E: single-plane sufficient reduction (riir-gpu Issue 611)
        assert!(concepts.contains(&"one plane (warp) suffices for a cooperative reduction — do not launch N redundant planes that recompute the identical sum"));
        // Category A: offload CPU bottleneck to GPU kernel (riir-gpu gpu_transpose)
        assert!(concepts.contains(&"when CPU data-prep (transpose, permute, format convert) is the bottleneck, run a GPU compute kernel that writes directly into pooled memory"));
    }

    // ════════════════════════════════════════════════════════════════════
    // Batch 15 (Research 016) — riir-gpu production kernels: block-contiguous layout + fused GeGLU GEMV + orthogonal pre-rotation + water-fill tail-dim
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn batch15_rules_present() {
        // Verify the 4 batch-15 rules across A/B/C/N.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category A (1 new): block-contiguous weight layout
        assert!(ids.contains(&"block-contiguous-weight-layout"), "block-contiguous layout");
        // Category C (1 new): fuse gate+up GEMV + GeGLU
        assert!(ids.contains(&"fuse-gate-up-gemv-activation"), "fused dual GEMV + GeGLU");
        // Category B (1 new): pre-rotate query once
        assert!(ids.contains(&"pre-rotate-once-exploit-orthogonality"), "pre-rotate query");
        // Category N (1 new): water-fill tail-dim shared codebook
        assert!(ids.contains(&"water-fill-tail-dim-shared-codebook"), "water-fill tail-dim");
    }

    #[test]
    fn batch15_block_contiguous_fused_geglu_rotation_codebook_concepts_covered() {
        // Verify the batch-15 block-contiguous + fused-geglu + rotation + codebook concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category A: block-contiguous weight layout (riir-gpu gemm_ternary_block_contiguous)
        assert!(concepts.contains(&"pack multiple logical weight buffers (scale + bit-planes) into one block-contiguous buffer so one global-memory access fetches all components"));
        // Category C: fuse gate+up GEMV + GeGLU (riir-gpu gemv_geglu_f16)
        assert!(concepts.contains(&"fuse two sibling GEMVs (gate + up projections) + their multiplicative activation into one kernel dispatch"));
        // Category B: pre-rotate query once (riir-gpu spectralquant/attention.rs)
        assert!(concepts.contains(&"for dot(q, R·y) with many y and orthogonal R, pre-rotate the query once (q' = R^T·q) instead of rotating each y"));
        // Category N: water-fill tail-dim shared codebook (riir-gpu spectralquant/dequant.rs)
        assert!(concepts.contains(&"non-uniform bit allocation: per-dim codebooks for top-energy dims, single shared low-bit codebook for low-energy tail dims"));
    }

    // ════════════════════════════════════════════════════════════════════
    // Batch 16 (Research 017) — riir-gpu production kernels: cmma hardware unit + dequant staging + dispatch axis swap + norm-decoupled scoring
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn batch16_rules_present() {
        // Verify the 4 batch-16 rules across A/B/D/N.
        let corpus = build_kernel_seed_corpus();
        let ids: Vec<&str> =
            corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        // Category B (1 new): cmma hardware unit over scalar ALU
        assert!(ids.contains(&"cmma-hardware-unit-over-scalar-alu"), "cmma hardware unit");
        // Category A (1 new): dequant once per K-tile into shared memory
        assert!(ids.contains(&"dequant-once-per-k-tile-into-shared-memory"), "dequant staging");
        // Category D (1 new): swap large axis to X for workgroup distribution
        assert!(ids.contains(&"swap-large-axis-to-x-for-workgroup-distribution"), "dispatch axis swap");
        // Category N (1 new): norm-decoupled low-bit scoring
        assert!(ids.contains(&"norm-decoupled-low-bit-scoring"), "norm-decoupled scoring");
    }

    #[test]
    fn batch16_cmma_dequant_swap_normdecoupled_concepts_covered() {
        // Verify the batch-16 cmma + dequant-staging + axis-swap + norm-decoupled concepts.
        let corpus = build_kernel_seed_corpus();
        let concepts: Vec<&str> =
            corpus.entries.iter().map(|e| e.gpu_concept.as_str()).collect();
        // Category B: cmma hardware unit (riir-gpu gemm_ternary_simdgroup)
        assert!(concepts.contains(&"use the cooperative matrix multiply-accumulate hardware unit (tensor core / Metal simdgroup_matrix / cmma) instead of scalar ALU for matrix multiply"));
        // Category A: dequant once per K-tile into shared memory (riir-gpu gemm_ternary_simdgroup)
        assert!(concepts.contains(&"dequantize a weight tile once into shared memory, let the hardware matrix unit consume it across all output rows that share the tile"));
        // Category D: swap large axis to X (riir-gpu gemm_ternary_simdgroup)
        assert!(concepts.contains(&"when one GEMM axis is much larger, put it on the X (first) cube-count axis for better workgroup distribution across SMs"));
        // Category N: norm-decoupled low-bit scoring (riir-gpu forward_turboquant)
        assert!(concepts.contains(&"store per-vector L2 norm separately as a scalar; quantize the unit-normalized vector at lower bits; reconstruct the dot product as norm × dot(unit_q, unit_k_quantized)"));
    }

    // ════════════════════════════════════════════════════════════════════
    // Plan 003 — Router ↔ corpus wiring tests
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn kernel_categories_table_has_15_entries() {
        assert_eq!(KERNEL_CATEGORIES.len(), 15, "Plan 003: 15 categories (A–O)");
    }

    #[test]
    fn kernel_categories_cover_all_corpus_rules_exhaustively() {
        // Every rule_id in the 131-rule corpus must appear in exactly one category.
        let corpus = build_kernel_seed_corpus();
        let corpus_ids: Vec<&str> = corpus.entries.iter().map(|e| e.rule_id.as_str()).collect();
        let mut categorized: Vec<&str> = Vec::new();
        for (_, _, rule_ids) in KERNEL_CATEGORIES {
            categorized.extend_from_slice(rule_ids);
        }
        // Same length (no missing, no extra).
        assert_eq!(categorized.len(), corpus_ids.len(),
            "Plan 003: KERNEL_CATEGORIES rule count ({}) must match corpus ({})",
            categorized.len(), corpus_ids.len());
        // No duplicate category assignments.
        let mut sorted = categorized.clone();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before, "Plan 003: rule appears in multiple categories");
        // Every corpus rule has a category home.
        for rid in &corpus_ids {
            assert!(categorized.contains(rid),
                "Plan 003: corpus rule '{rid}' has no category in KERNEL_CATEGORIES");
        }
        // Every category entry exists in the corpus (no stale rule_ids).
        for rid in &categorized {
            assert!(corpus_ids.contains(rid),
                "Plan 003: KERNEL_CATEGORIES references '{rid}' not in corpus");
        }
    }

    #[test]
    fn kernel_categories_counts_match_plan_003_table() {
        // Pin per-category rule counts to the Plan 003 table (updated for Batch 16).
        let expected = [
            (KernelCategory::MemoryAccess, 9),
            (KernelCategory::Compute, 20),
            (KernelCategory::Fusion, 12),
            (KernelCategory::Dispatch, 9),
            (KernelCategory::Parallelism, 8),
            (KernelCategory::FlashAttnTiling, 10),
            (KernelCategory::LinearAttnRecurrence, 17),
            (KernelCategory::MoeDispatchBackward, 9),
            (KernelCategory::NsaBlockSparse, 4),
            (KernelCategory::VarlenChunkedReduction, 5),
            (KernelCategory::AutotuneDispatch, 4),
            (KernelCategory::ConvGemmLayout, 5),
            (KernelCategory::KernelPolymorphism, 5),
            (KernelCategory::DtypeAlignment, 10),
            (KernelCategory::IrStrengthReduction, 4),
        ];
        for (cat, count) in expected {
            let entry = KERNEL_CATEGORIES.iter().find(|(c, _, _)| *c == cat)
                .unwrap_or_else(|| panic!("Plan 003: category {cat:?} missing from KERNEL_CATEGORIES"));
            assert_eq!(entry.2.len(), count,
                "Plan 003: category {cat:?} has {} rules, expected {count}",
                entry.2.len());
        }
        // Sum must equal 131.
        let total: usize = KERNEL_CATEGORIES.iter().map(|(_, _, r)| r.len()).sum();
        assert_eq!(total, 131, "Plan 003: category totals must sum to 131");
    }

    #[test]
    fn build_corpus_categorized_round_trips_all_rules() {
        // build_corpus_categorized groups the corpus by category. Every rule
        // should appear in exactly one bucket, and the total count should match.
        let grouped = build_corpus_categorized();
        assert_eq!(grouped.len(), 15, "Plan 003: 15 category buckets");
        let total: usize = grouped.iter().map(|(_, rules)| rules.len()).sum();
        assert_eq!(total, 131, "Plan 003: grouped corpus has {total} rules, expected 131");
        // No empty buckets.
        for (cat, rules) in &grouped {
            assert!(!rules.is_empty(), "Plan 003: category {cat:?} has zero rules");
        }
    }
}

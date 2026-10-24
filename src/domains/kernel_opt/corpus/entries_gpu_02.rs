//! Entries slice — Batches 14-46 era (rules 161-198 of 340) — riir-gpu production-kernel mining.
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
        // Batch 24 (Research 025): cudarc_kernels backward cluster — 4 rules
        // (cudarc_kernels/backward.rs — 1923 lines; Issue 641 T7.3/T7.5
        // per-layer backward CUDA kernels for DeltaNet + Attention: RMSNorm /
        // SwiGLU / z-gating / output-gate / conv1d+SiLU / L2-norm / beta-decay
        // / softmax-attention backward + in-place vector accumulate).
        // ════════════════════════════════════════════════════════════════════
        // ── widen-index-multiply-to-64bit — address-width for large buffers (A) ──
        // Source: riir-gpu cudarc_kernels/backward.rs L416/L474/L478/L515/L518 —
        // every KV cache access in softmax_attention_backward_f32 uses
        // `(long)t * kv_stride + kv_off`. int32 index arithmetic wraps silently
        // in CUDA when position × stride exceeds 2^31 (long-context KV caches,
        // big batched weight-grad buffers) → OOB reads / wrong elements with
        // no error. The pattern: keep loop counters + small dims 32-bit
        // (register economy), widen ONLY the address-producing multiply by
        // casting ONE operand to 64-bit.
        KernelRule {
            rule_id: "widen-index-multiply-to-64bit".to_string(),
            gpu_concept: "when a flat index product (position × stride) can exceed 2^31 on large buffers (long-context KV caches, big batched grads), widen ONLY the address-producing multiply to 64-bit by casting one operand — keep loop counters and small dims 32-bit for register economy. int32 index overflow in CUDA wraps silently → OOB reads / wrong elements with no error. Appears as `(long)t * kv_stride` at every KV cache access. Distinct from upcast-before-reduction (fp32 precision) — this is integer address width. RIDER (Batch 225 / Plan 227 / Research 255 — the VENDOR-VIEW face, ggml-org/llama.cpp @ 48e9857 #28748): the wrap can live INSIDE a vendor API's typed view, where you cannot widen the arithmetic because the offset type is the vendor's — the fix moves the base pointer manually in 64-bit and SHRINKS the extent instead of slicing from a wide base. The Metal tensor path built its output view from the batch base ('device float * dstBatch = (device float *)dst + im * N * M;' + 'auto tD = tensor(dstBatch, dextents<int32_t, 2>(M, N), ...); cT.store(tD.slice(ra, rb));') — the slice's byte offset is INT32 and wraps at 2 GiB ('// a slice is not used because its byte offset is int32 and wraps at 2 GiB', in-source); the fix computes the tile base in uint64 ('device float * dstTile = (device float *)dst + (uint64_t) im * N * M + (uint64_t) rb * M + ra;') and CLAMPS the tile extents to the real remainder ('const int tileM = min(NRA, M - ra); const int tileN = min(NRB, N - rb);') so the typed view never carries a >2 GiB displacement. Two halves: pointer arithmetic you own stays 64-bit (this rule's original law); pointer arithmetic a vendor TYPE owns (MSL tensor + slice, any typed view with a 32-bit internal offset) is avoided at the base — hand the view a NEAR base plus a small extent, never a far base plus a slice. Same silent-wrong-read signature, third surface (CUDA index math, Metal vendor view, and by extension any dextents/slice-class API). Batch 225 rider.".to_string(),
            ref_shape: "// naive: 32-bit index multiply — wraps when t * kv_stride exceeds 2^31\nfn kv_dot_naive(q: &[f32], kv_cache: &[f32], t: i32, kv_stride: i32, kv_off: i32, head_dim: usize) -> f32 {\n    let base = t * kv_stride + kv_off;  // i32 arithmetic — overflows silently in CUDA\n    let mut dot = 0.0;\n    for d in 0..head_dim { dot += q[d] * kv_cache[(base + d as i32) as usize]; }\n    dot  // t=500_000, kv_stride=8192 → 4.1e9 > 2^31 → wrapped index, silent wrong reads\n}".to_string(),
            opt_pattern: "// optimized: widen ONE operand of the address multiply — loop counters stay 32-bit\nfn kv_dot_widened(q: &[f32], kv_cache: &[f32], t: i32, kv_stride: i32, kv_off: i32, head_dim: usize) -> f32 {\n    let base = t as i64 * kv_stride as i64 + kv_off as i64;  // 64-bit address math\n    let mut dot = 0.0;\n    for d in 0..head_dim { dot += q[d] * kv_cache[(base + d as i64) as usize]; }\n    dot  // correct index at any realistic cache size; the 64-bit pair costs 1 extra register\n}".to_string(),
        },
        // ── dual-accumulator-single-reduction-sweep — two reductions, one barrier tree (B) ──
        // Source: riir-gpu cudarc_kernels/backward.rs L61-83
        // (rmsnorm_batched_backward_f32 pass 1). RMSNorm backward needs TWO
        // reductions over the SAME elements: sum_sq = Σ x[i]² and
        // dot = Σ grad_y[i]·gamma[i]·x[i]. The kernel computes both partial
        // sums in ONE strided loop (x[off+i] loaded once, feeding both
        // accumulators), then reduces BOTH shared-memory arrays inside the
        // SAME stride tree (s_sum_sq[tid] += s_sum_sq[tid+stride] AND
        // s_dot[tid] += s_dot[tid+stride] per step) — halving global loads
        // and halving the __syncthreads() count vs two sequential reductions.
        KernelRule {
            rule_id: "dual-accumulator-single-reduction-sweep".to_string(),
            gpu_concept: "fuse two block reductions over the same data into one sweep: keep two partial accumulators in the same strided loop (each element loaded once, feeding both), and reduce both shared-memory arrays inside the SAME stride tree — halves global loads and halves the __syncthreads() barrier count vs two sequential reductions. RMSNorm backward needs sum(x²) and dot(grad_y·γ·x) over identical elements — the canonical case. Distinct from split-sign-accumulators-defer-subtraction (ILP chains within one dot) — this is barrier + load sharing across two different reductions".to_string(),
            ref_shape: "// naive: two separate reduction passes — x loaded twice, two barrier trees\nfn rmsnorm_bwd_two_passes(grad_y: &[f32], x: &[f32], gamma: &[f32], head_dim: usize) -> (f32, f32) {\n    let sum_sq: f32 = (0..head_dim).map(|i| x[i] * x[i]).sum();              // pass 1: load x\n    let dot: f32 = (0..head_dim).map(|i| grad_y[i] * gamma[i] * x[i]).sum(); // pass 2: load x AGAIN\n    // each pass = its own shared-memory tree + its own __syncthreads() sequence\n    (sum_sq, dot)\n}".to_string(),
            opt_pattern: "// optimized: one sweep, two accumulators, ONE shared barrier tree\nfn rmsnorm_bwd_dual_acc(grad_y: &[f32], x: &[f32], gamma: &[f32], head_dim: usize) -> (f32, f32) {\n    let mut partial_sq = 0.0f32;\n    let mut partial_dot = 0.0f32;\n    for i in 0..head_dim {\n        let xi = x[i];                        // loaded ONCE — feeds both accumulators\n        partial_sq += xi * xi;\n        partial_dot += grad_y[i] * gamma[i] * xi;\n    }\n    // single block-reduction sweeps BOTH accumulator arrays per stride step:\n    //   s_sum_sq[tid] += s_sum_sq[tid + stride];\n    //   s_dot[tid]    += s_dot[tid + stride];\n    // half the __syncthreads() count + half the x[] loads vs two reductions\n    (partial_sq, partial_dot)\n}".to_string(),
        },
        // ── dual-grad-same-pass — both partials of a product rule in one kernel (C) ──
        // Source: riir-gpu cudarc_kernels/backward.rs L108-126 (swiglu),
        // L137-155 (z_gating), L169-186 (output_gate_sigmoid), L345-369
        // (beta_decay). A product-rule activation out = f(gate)·g(other)
        // yields TWO partial derivatives that share the same inputs AND the
        // same transcendental evaluation: grad_a = grad_out·f'(a)·b and
        // grad_b = grad_out·f(a). One kernel loads each element once,
        // evaluates sigmoid/silu ONCE, writes both grad buffers. Two kernels
        // would re-read all inputs + re-evaluate exp + launch twice.
        KernelRule {
            rule_id: "dual-grad-same-pass".to_string(),
            gpu_concept: "a product-rule activation out = f(gate)·g(other) has TWO partial derivatives that share the same inputs and the same transcendental evaluation (sigmoid/silu) — emit both from ONE kernel: load each element once, evaluate the activation + its derivative once, write both grad buffers. Applies to every gated backward (SwiGLU: grad_gate+grad_up; z-gating: grad_pre+grad_z; sigmoid output-gate: grad_pre+grad_gate; beta/decay pair). Distinct from fused-backward-chain (producer→consumer chain stays in registers) and fuse-gate-up-gemv-activation (forward sibling GEMVs) — this is the backward fan-out: sibling grads sharing one evaluation".to_string(),
            ref_shape: "// naive: two kernels — each re-reads all inputs and re-evaluates the sigmoid\nfn swiglu_bwd_gate_kernel(grad_out: &[f32], gate: &[f32], up: &[f32], grad_gate: &mut [f32]) {\n    for i in 0..gate.len() {\n        let sig = 1.0 / (1.0 + (-gate[i]).exp());    // exp evaluated HERE\n        let silu_deriv = sig * (1.0 + gate[i] * (1.0 - sig));\n        grad_gate[i] = grad_out[i] * silu_deriv * up[i];  // reads gate + up + grad_out\n    }\n}\nfn swiglu_bwd_up_kernel(grad_out: &[f32], gate: &[f32], up: &[f32], grad_up: &mut [f32]) {\n    for i in 0..gate.len() {\n        let sig = 1.0 / (1.0 + (-gate[i]).exp());    // SAME exp re-evaluated\n        grad_up[i] = grad_out[i] * gate[i] * sig;    // reads the SAME inputs again\n    }\n}\n// 2× input reads, 2× transcendental evals, 2× kernel launches".to_string(),
            opt_pattern: "// optimized: ONE kernel — one pass, one sigmoid, both partials\nfn swiglu_bwd_dual(grad_out: &[f32], gate: &[f32], up: &[f32],\n                   grad_gate: &mut [f32], grad_up: &mut [f32]) {\n    for i in 0..gate.len() {\n        let g = gate[i];                             // read once\n        let sig = 1.0 / (1.0 + (-g).exp());          // exp once — feeds BOTH partials\n        let silu_g = g * sig;\n        let silu_deriv = sig * (1.0 + g * (1.0 - sig));\n        grad_gate[i] = grad_out[i] * silu_deriv * up[i];\n        grad_up[i] = grad_out[i] * silu_g;           // silu_g reuses the same sig\n    }\n    // halves input reads + transcendental evals + kernel launches\n}".to_string(),
        },
        // ── broadcast-uniform-skip-reduce — thread-uniform values are not reduced (E) ──
        // Source: riir-gpu cudarc_kernels/backward.rs L492-501
        // (softmax_attention_backward_f32, the documented T7.2 BUG FIX):
        // "z is the same for all threads (gp was already reduced to
        // s_reduce[0], so every thread computed the same z_local). No
        // cross-thread reduction needed — just broadcast from thread 0 via
        // shared memory. BUG FIX (T7.2 validation catch): the prior code
        // reduced z_local across threads, which multiplied z by blockDim.x
        // since every thread had the same value." When a per-thread value is
        // derived ENTIRELY from already-reduced broadcasts, it is
        // thread-uniform: re-reducing it sums the identical value blockDim.x
        // times — wrong result, not just wasted barriers. Broadcast instead.
        KernelRule {
            rule_id: "broadcast-uniform-skip-reduce".to_string(),
            gpu_concept: "when a per-thread value is derived entirely from already-reduced broadcasts (every thread read the same smem[0] + the same shared arrays), it is thread-uniform: do NOT re-reduce it — the tree sums the identical value blockDim.x times, multiplying the result by the block size (a correctness bug, not just wasted barriers). Broadcast instead: thread 0 writes smem, one barrier, every thread reads. Caught as the T7.2 validation bug in riir-gpu's softmax backward (z came out blockDim.x× too large). Distinct from single-plane-sufficient-reduction (dispatch-level redundant planes) — this is intra-block uniformity after a reduction".to_string(),
            ref_shape: "// naive: block-reduce a value that is ALREADY identical in every thread\n__shared__ float s_reduce[256];\n// every thread's z_local is IDENTICAL (derived from the same reduced s_reduce[0])\ns_reduce[tid] = z_local;\n__syncthreads();\nfor (int stride = 128; stride > 0; stride >>= 1) {      // full barrier tree\n    if (tid < stride) s_reduce[tid] += s_reduce[tid + stride];  // sums the SAME value\n    __syncthreads();\n}\nfloat z = s_reduce[0];  // = z_local * blockDim.x — WRONG (256× too large), not just slow".to_string(),
            opt_pattern: "// optimized: thread-uniform value → broadcast from thread 0, no reduction\n__shared__ float s_bcast[1];\n// uniform-by-construction: z_local derives only from already-reduced\n// broadcasts (every thread read the same s_reduce[0] + the same probs[t])\nif (tid == 0) { s_bcast[0] = z_local; }   // one write\n__syncthreads();                          // ONE barrier\nfloat z = s_bcast[0];                     // every thread reads the value — correct".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 25 (Research 026): riir-gpu production kernels — the Gemma
        // cluster (gemma2_cubecl/ + gemma4_cubecl/): device-side argmax with
        // minimal download, monotone-transform elision under argmax, fan-out
        // sibling dispatches sharing one input + one sync, weight hot-swap
        // preserving autotune/JIT caches, and tree-reduction depth over
        // sequential accumulation.
        // ════════════════════════════════════════════════════════════════════
        // ── device-side-argmax-minimal-download — download vocab×4B → 4B (D) ──
        // Source: riir-gpu gemma2_cubecl/gpu_forward.rs L319-346
        // (gpu_argmax_from_handle, Plan 171 T3) + L348-423 (generate_gpu).
        // Greedy decode needs ONE u32 (the token id), not the logits. A GPU
        // argmax reduction kernel downloads 4 bytes instead of vocab_size×4
        // (~1 MB at 256K vocab): ~250,000× smaller per-token boundary crossing.
        // Over 50-prompt + 500-token decode: ~550 MB → ~50 MB total (prefill
        // still reads full logits once; decode never does).
        //
        // Distinct from offload-cpu-bottleneck-to-gpu-kernel (A — moving slow
        // CPU data-prep onto GPU): the CPU argmax isn't slow; the DOWNLOAD
        // feeding it is the cost, and the fix shrinks the crossing itself.
        // Distinct from chain-dispatches-single-sync-point (sync COUNT) —
        // this is download SIZE.
        KernelRule {
            rule_id: "device-side-argmax-minimal-download".to_string(),
            gpu_concept: "when the host needs only a tiny derived result of a large GPU array (argmax index, top-k, threshold count), compute it ON DEVICE with a reduction kernel and download only the tiny result — 4 bytes (1 u32) instead of the full array (vocab_size × 4 ≈ 1 MB at 256K vocab). Greedy decode is the canonical consumer: ~250,000× smaller per-token boundary crossing; a 50-prompt + 500-decode session downloads ~50 MB instead of ~550 MB. Distinct from offload-cpu-bottleneck-to-gpu-kernel (CPU work moved to GPU) — the CPU side isn't slow, the DOWNLOAD feeding it is the cost; and from chain-dispatches-single-sync-point (sync count) — this is download size RIDER (Batch 129 external confirmation + facet, ds4-metal @ b85a6174 -- commit 66b0e3f 'Select Qwen MTP drafts on GPU without full logit readback', 248,320 FP32 logits -> one int32, all fifteen paired runs faster, median +0.07-0.22%, 547 full-vocab vectors byte-identical): the on-device selector must REPRODUCE the CPU reference's exact decision semantics or the two paths diverge on adversarial values -- lowest-index tie-break (the source: 'if (v > best || (v == best && p.y < index))'), NaN skipped by a bit test ('(p.x & 0x7fffffffu) > 0x7f800000u'), sentinel -1e30 init with index-0 all-invalid fallback ('The -1e30 initial score and index-zero fallback match sample_argmax'), and a bounds check on the read-back index. The gate is CONSUMER-SHAPED: the readback payload follows the caller's request, not the tensor's existence (full logits still read back when logits_out is requested; 'want_logits && draft_out && !logits_out' at ds4.c:55706-55709), with an env arm (DS4_QWEN4_MTP_GPU_ARGMAX=0) restoring the CPU path for A/B. RIDER (Batch 251, yukon cudafast-bonsai2-9b-engine @ 36c9536 -- DEPENDENT-CHAIN): the greedy DFlash2 selector walk -- n_steps argmaxes where step i's score row is indexed by step i-1's winner -- stays deliberately SINGLE-THREAD on-device: 'One thread runs the walk: n_steps dependent argmaxes over top_k scores each. It keeps std::max_element's rule (replace only on a strict \"best < candidate\", so the first maximum wins and a NaN never replaces the best) and the host walk's float -> int32 truncation, so every id equals the one common/speculative.cpp would pick from the same lattice.' (dflash-walk.cu:3-6) -- a one-thread launch ('if (threadIdx.x != 0) { return; }' :11-13) is chosen over parallelizing an argmax that is trivially cheap at top_k width, because the DEPENDENT-CHAIN semantics must reproduce the host walker's DECISIONS exactly: step i's scores read at 'const float * scores = row + top_k + (int64_t) predecessor*top_k;' (:17), 'predecessor = best;' feeding the next iteration, and the id emitted as the float->int32 truncation 'out[i] = (int32_t) row[predecessor];' (:27-28). Verified: dflash-walk.cu:3-6, :11-13, :17, :27-28.".to_string(),
            ref_shape: "// naive: download the full logits vector every decode token, argmax on CPU\nfn decode_token_naive(logits_handle: Handle, vocab: usize) -> usize {\n    let logits: Vec<f32> = read_handle(&logits_handle); // downloads vocab × 4 bytes (~1 MB)\n    logits.iter().enumerate()                          // CPU argmax over 256K floats\n        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())\n        .map(|(i, _)| i)\n        .unwrap_or(0)\n}\n// 500 decode tokens × ~1 MB = ~500 MB crossed the bus just to pick 1 index/token".to_string(),
            opt_pattern: "// optimized: GPU argmax kernel — download only the winning index (4 bytes)\nfn gpu_argmax_from_handle(logits_handle: Handle, vocab: usize) -> usize {\n    let output_handle = client.empty(core::mem::size_of::<u32>()); // 4-byte output\n    unsafe {\n        ArgmaxCubeCL::launch::<Runtime>(                  // device-side reduction:\n            &client, logits_handle, output_handle.clone(), vocab // blocks reduce slices,\n        );                                               // finals in shared memory\n    }\n    let bytes = client.read_one(output_handle).unwrap(); // downloads 4 bytes\n    u32::from_bytes(&bytes)[0] as usize\n}\n// 500 decode tokens × 4 B ≈ 2 KB total — prefill still reads full logits once\n// (sampling needs values); the decode loop never downloads them again".to_string(),
        },
        // ── monotone-transform-elision-under-argmax — softcap skipped (B) ──
        // Source: riir-gpu gemma2_cubecl/gpu_forward.rs L208-217 — the doc on
        // forward_gpu_logits_handle: "Logit softcapping is NOT applied. Since
        // softcap (tanh(x/cap) * cap) is monotonically increasing, it doesn't
        // change the argmax result, so it can be safely skipped for greedy
        // decoding." The CPU-facing forward() still applies it — values are
        // consumed there. Composes with device-side-argmax: the skipped pass
        // would have been 256K tanh evals per decode token.
        //
        // Distinct from softcap-tanh-attention-score (Batch 23 — APPLYING
        // softcap in-kernel for bounded attention logits): this ELIDES the
        // transform when the consumer only needs order. Distinct from
        // optional-bias-branch (absence check) — the transform exists; its
        // output's ORDER is invariant under it.
        KernelRule {
            rule_id: "monotone-transform-elision-under-argmax".to_string(),
            gpu_concept: "a strictly monotone post-transform (softcap cap·tanh(x/cap), clamps, positive rescalings) does not change the argmax — skip it entirely when the only consumer is greedy decoding / top-k / any order-only query. The GPU decode path returns the raw logits WITHOUT final logit softcapping because tanh is monotonically increasing: argmax(softcap(x)) == argmax(x); the value-consuming CPU path still applies it. Elides a full-array transcendental pass (256K tanh evals per decode token). Distinct from softcap-tanh-attention-score (applying softcap in-kernel) — this elides it under order-only consumers; distinct from optional-bias-branch (absence) — the transform exists but order is invariant under it. RIDER (Batch 170, mlxfast-gemma4 @ 27c821c4 -- LogitslessGreedyHeadV2.swift L3-45, LGH-001): elision extended from the transform to the OUTPUT PLANE itself -- materialising the logits plane costs 'one 4 MB store, one softcap pass that reads it and writes 8 MB, and one argmax pass that reads those 8 MB back --- all of it on the step's dependent chain, all of it to recover eight integers the head kernel already knows' (the softcap is strictly increasing so it cannot move the argmax; every other pipeline transform is inactive under plain greedy); the mechanism is an opt-in PROTOCOL seam (the model exposes cbv2DecodeArgmax, the sampler admits cbv2AdmitsArgmaxDecode -- a pure host predicate over step geometry, so the elision is negotiated at the interface rather than rewritten in the graph): this rule's order-invariance argument carried to its limit, where the elided op is the entire readout".to_string(),
            ref_shape: "// naive: apply the monotone softcap to the full logits vector before argmax\nfn decode_with_unnecessary_softcap(logits: &mut [f32], cap: f32) -> usize {\n    for x in logits.iter_mut() {              // cap·tanh(x/cap) over 256K floats\n        *x = cap * (*x / cap).tanh();         // 256K transcendental evals — ORDER-ONLY\n    }                                          // consumer: argmax below\n    logits.iter().enumerate()\n        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())\n        .map(|(i, _)| i).unwrap_or(0)\n}".to_string(),
            opt_pattern: "// optimized: softcap is strictly increasing → argmax unchanged → skip it\nfn decode_skip_softcap(logits: &[f32]) -> usize {\n    // cap·tanh(x/cap) is monotonically increasing in x, so\n    // argmax(softcap(x)) == argmax(x) — elide the entire pass.\n    // (The CPU-facing forward() still applies softcap when VALUES are consumed;\n    //  sampling/thresholding code that needs bounded logits also keeps it.)\n    logits.iter().enumerate()\n        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())\n        .map(|(i, _)| i).unwrap_or(0)          // zero tanh evaluations\n}".to_string(),
        },
        // ── fan-out-shared-input-single-sync — 3 uploads+3 syncs → 1+1 (C) ──
        // Source: riir-gpu gemma2_cubecl/dispatch.rs L69-122 (dispatch_qkv).
        // "All three GEMVs share the same input (hidden state). CubeCL batches
        // them into a single GPU submission." The input handle is created ONCE
        // and cloned into each launch (Arc bump); the first output read triggers
        // execution of the whole batched submission, so the remaining reads
        // return already-completed buffers.
        //
        // Distinct from chain-dispatches-single-sync-point (Batch 19 — DEPENDENT
        // dispatches, sync amortization only): this is INDEPENDENT sibling
        // kernels where the shared input upload is also hoisted. Note the
        // chain rule's "fan-out is the known-bad case per Bench 437" caveat
        // refers to DAG fan-out (one dispatch's output feeding multiple LATER
        // dispatches — intermediates accumulate); this is input-sharing
        // fan-out with independent outputs read once at the end — no
        // intermediates outlive the batch. Distinct from
        // fuse-gate-up-gemv-activation (fusing siblings into ONE kernel) — a
        // tighter but more invasive option; this keeps existing kernels
        // unchanged (needed when the projections have different consumers or
        // pre-existing kernel variants).
        KernelRule {
            rule_id: "fan-out-shared-input-single-sync".to_string(),
            gpu_concept: "when N INDEPENDENT sibling kernels consume the same input (Q/K/V GEMVs on the same hidden state, per-head projections), create the input handle ONCE, clone it into each launch (cheap ref-count), queue all N dispatches, then read the outputs back-to-back — the first read executes the whole batched submission, so 1 upload + 1 effective sync instead of N uploads + N syncs. NOT the Bench 437 known-bad fan-out (one dispatch's output feeding multiple later dispatches — intermediates accumulate): here outputs are independent and read once at the end, no intermediates outlive the batch. Distinct from chain-dispatches-single-sync-point (dependent dispatches, sync amortization only — no shared upload) and from fuse-gate-up-gemv-activation (fuses siblings into ONE kernel — tighter but more invasive; this keeps existing kernels unchanged)".to_string(),
            ref_shape: "// naive: 3 self-contained dispatch helpers — each uploads the input + syncs\nfn qkv_naive(w_q: &Handle, w_k: &Handle, w_v: &Handle, hidden: &[f32]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {\n    let q = dispatch_gemv_sync(w_q, hidden); // create_from_slice(hidden) + read → upload 1, sync 1\n    let k = dispatch_gemv_sync(w_k, hidden); // SAME hidden uploaded again   → upload 2, sync 2\n    let v = dispatch_gemv_sync(w_v, hidden); // and again                    → upload 3, sync 3\n    (q, k, v) // 3 uploads of the same bytes, 3 CPU↔GPU round-trips\n}".to_string(),
            opt_pattern: "// optimized: hoist the shared upload; queue all 3; one effective sync\nfn qkv_fan_out(w_q: &Handle, w_k: &Handle, w_v: &Handle, hidden: &[f32]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {\n    let input = client.create_from_slice(f32::as_bytes(hidden)); // uploaded ONCE\n    let q_out = client.empty(q_dim * 4);\n    let k_out = client.empty(kv_dim * 4);\n    let v_out = client.empty(kv_dim * 4);\n    unsafe {\n        gemv.launch(&client, w_q.clone(), input.clone(), q_out.clone(), q_dim, n);\n        gemv.launch(&client, w_k.clone(), input.clone(), k_out.clone(), kv_dim, n);\n        gemv.launch(&client, w_v.clone(), input,          v_out.clone(), kv_dim, n);\n    }\n    // Single sync: the FIRST read executes the whole batched submission;\n    // the remaining reads return already-completed buffers.\n    (read_handle(&q_out), read_handle(&k_out), read_handle(&v_out))\n}".to_string(),
        },
        // ── hot-swap-weights-preserve-autotune-caches — new() per step → swap (D) ──
        // Source: riir-gpu gemma4_cubecl/mod.rs L215-235 (replace_weights).
        // "Used by the LoRA training loop: each step merges the LoRA delta into
        // the working weights, then calls this to refresh the GPU handles.
        // This avoids the ~seconds of autotune re-benchmarking + JIT
        // recompilation that happens on every new() call." The CubeCL client,
        // autotune cache, JIT kernel cache, RoPE tables, and layer params are
        // all preserved; only weight handles + CPU gamma copies + KV state
        // are refreshed.
        //
        // Distinct from freeze/thaw weight snapshots (versioning) — this is
        // about amortizing RUNTIME caches (autotune benchmarks, JIT-compiled
        // shaders) across weight changes.
        KernelRule {
            rule_id: "hot-swap-weights-preserve-autotune-caches".to_string(),
            gpu_concept: "when a training loop refreshes weights every step (LoRA merge → new handles), replace ONLY the weight handles + CPU-side copies — keep the compute client, autotune cache (per-(m,n) benchmarked variants), JIT-compiled kernel cache, precomputed tables, and layer params. A full reconstruct re-runs autotune benchmarking + JIT compilation (~seconds) per step; the hot-swap path pays only the buffer uploads. Distinct from freeze/thaw weight snapshots — this amortizes runtime caches across weight changes, not weight versions".to_string(),
            ref_shape: "// naive: rebuild the whole GPU model every training step\nfn lora_step_rebuild(client: ComputeClient, merged: &Weights, config: &Config) -> GpuModel {\n    // LoRA merge changed the weights — construct from scratch:\n    let model = GpuModel::new(client, merged, config);\n    //   → re-uploads all weights (fine)\n    //   → re-benchmarks autotune variants per (m, n)  (~seconds!)\n    //   → re-JIT-compiles every kernel shader         (~seconds!)\n    model\n}\n// 500 training steps × (autotune + JIT) = minutes of pure re-initialization".to_string(),
            opt_pattern: "// optimized: replace_weights keeps client + autotune + JIT caches alive\nfn lora_step_hot_swap(model: &mut GpuModel, merged: &Weights) {\n    model.weights = WeightBuffers::from_weights(&model.client, merged, &model.config);\n    model.norm_gammas = NormGammas::from_weights(merged);   // CPU copies refresh\n    model.wte_cpu = merged.wte.clone();\n    for layer in 0..model.config.n_layer {                  // reset KV cache state\n        model.kv_cache.keys[layer].clear();\n        model.kv_cache.values[layer].clear();\n        model.kv_cache.n_positions[layer] = 0;\n    }\n    // autotune cache, JIT cache, client, rope tables, layer params: preserved.\n    // Per-step cost = weight upload only — the ~seconds of autotune + JIT\n    // recompilation happened once at construction and is never paid again.\n}".to_string(),
        },
        // ── tree-reduction-depth-over-sequential-accumulate — depth 2304 → 77 (B) ──
        // Source: riir-gpu gemma2_cubecl/dispatch.rs L881-893 (Issue 424 fix,
        // dispatch_gemv_batched_backward_gpu replacing dispatch_gemm_gpu in the
        // batched backward): "The GEMM tiled kernel accumulates sequentially
        // (depth n=2304), while the Plane kernel tree-reduces across 32 lanes
        // (depth n/32 + log2(32) ≈ 77). The 30× better numerical accuracy
        // prevents gradient divergence at 26-layer scale."
        //
        // Accumulated rounding error grows with summation DEPTH: sequential
        // adds round on top of n-1 prior roundings (error ~O(n·ε) worst case);
        // a lane-strided + butterfly tree keeps depth ~n/32 + log2(32). A
        // perf-neutral kernel CHOICE that changes only the summation order.
        //
        // Distinct from upcast-before-reduction (raising input precision) —
        // this changes the reduction's association shape at equal dtype.
        KernelRule {
            rule_id: "tree-reduction-depth-over-sequential-accumulate".to_string(),
            gpu_concept: "when the same dot product can be computed by either a sequential-accumulate kernel (tiled GEMM: error chain depth = n) or a tree-reduction kernel (plane/subgroup lanes: depth n/lanes + log2(lanes)), prefer the tree for gradient paths — accumulated rounding error grows with summation depth, and at 26-layer backward scale, sequential depth 2304 vs tree depth ≈77 gives ~30× better numerical accuracy, enough to prevent outright gradient divergence. A perf-neutral kernel swap changing only the summation order. Distinct from upcast-before-reduction (raising input precision) — this changes the reduction's association shape at equal dtype".to_string(),
            ref_shape: "// naive: sequential accumulation — error grows with full reduction depth\nfn gemm_tiled_sequential(a: &[f32], b_row: &[f32], out: &mut [f32], m: usize, n: usize, p: usize) {\n    for r in 0..m { for c in 0..p {\n        let mut acc = 0.0;\n        for k in 0..n {                        // depth n = 2304: each add rounds\n            acc += a[r*n + k] * b_row[c*n + k]; // on top of 2303 prior roundings\n        }\n        out[r*p + c] = acc;\n    } }\n    // 26 layers of backward through depth-2304 sums → gradient divergence\n    // (measured: ~30× worse error vs the plane kernel — Issue 424)\n}".to_string(),
            opt_pattern: "// optimized: plane tree reduction — depth n/32 + log2(32) ≈ 77\nfn gemv_plane_tree(a: &[f32], b_row: &[f32], out: &mut [f32], m: usize, n: usize, p: usize) {\n    const LANES: usize = 32;                  // subgroup size (Metal: always 32)\n    for c in 0..p {\n        // phase 1: each lane sums a strided slice — depth n/32 = 72\n        let mut lane_partial = 0.0;\n        let mut k = lane_id();\n        while k < n { lane_partial += a[k] * b_row[c*n + k]; k += LANES; }\n        // phase 2: log-depth butterfly across lanes — depth log2(32) = 5\n        lane_partial += shuffle_down_xor(lane_partial, 16);\n        lane_partial += shuffle_down_xor(lane_partial, 8);\n        lane_partial += shuffle_down_xor(lane_partial, 4);\n        lane_partial += shuffle_down_xor(lane_partial, 2);\n        lane_partial += shuffle_down_xor(lane_partial, 1);\n        if lane_id() == 0 { out[c] = lane_partial; } // total depth ≈ 77, not 2304\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 26 (Research 027): riir-gpu production kernels — the wgpu
        // forward-orchestration cluster (forward/ + the buffer.rs download
        // substrate it consumes): per-slot uniform buffers vs the write_buffer
        // ordering hazard, grid-encoded params derived in-kernel, dependent
        // dispatches merged into one compute pass, batched readbacks sharing
        // one sync cycle, chunked encoder submits below the Metal pass
        // ceiling, pre-built bind groups, host-cached immutable weights, and
        // deferred readback until after submit.
        // ════════════════════════════════════════════════════════════════════
        // ── per-slot-uniform-buffer-write-ordering — shared uniform → per-slot (D) ──
        // Source: riir-gpu forward/mod.rs L141-148 (uniform_relu: "Without
        // this, the shared uniform_elem gets overwritten by the last
        // write_uniform call before the encoder runs") + L361-390 (per-slot
        // batched matmul/elem/lora uniform Vec<Buffer>: "Each slot gets its
        // own buffer to avoid the wgpu write_buffer ordering bug") +
        // forward/forward_buffers.rs L550-557 + L622-635 (per-head
        // attn_uniforms: "all heads' write_buffer calls flush before the
        // encoder runs, so all heads see the last write") + forward/
        // adapter_merge.rs L691-693. wgpu queues write_buffer calls: ALL
        // writes flush BEFORE the encoder executes, so N dispatches in one
        // encoder sharing ONE uniform buffer all read the LAST write —
        // per-head attention params, per-target LoRA params, per-slot matmul
        // params silently corrupt (Issues 366/367/368 lineage: attention
        // pos-uniform, stale LoRA saved buffers, uniform slot conflict →
        // zero grad_hidden). The fix is structural: one small uniform buffer
        // per dispatch SLOT, each wired into its own pre-built bind group.
        //
        // Distinct from same-queue-implicit-ordering-no-sync (A — queue-level
        // dispatch ordering): this is the uniform WRITE channel racing the
        // encoder, not dispatch ordering. The complementary in-kernel escape
        // is derive-params-from-global-invocation-id (this batch).
        KernelRule {
            rule_id: "per-slot-uniform-buffer-write-ordering".to_string(),
            gpu_concept: "host-side queue.write_buffer calls all flush BEFORE the recorded encoder executes — N dispatches in one encoder sharing ONE uniform buffer all read the LAST write (per-head attention params, per-target LoRA params, per-slot matmul params silently corrupted; the Issues 366/367/368 lineage). Give each dispatch slot its own small uniform buffer (per head / per adapter / per target / per QKV slot), each wired into its own pre-built bind group, so concurrent writes in one encoder cannot collide. Distinct from same-queue-implicit-ordering-no-sync (queue-level dispatch ordering) — this is the uniform-write channel racing the encoder; the in-kernel escape is derive-params-from-global-invocation-id".to_string(),
            ref_shape: "// naive: one shared uniform buffer — every dispatch sees the LAST write\nfn per_head_attention_naive(encoder: &mut Encoder, queue: &Queue, shared_uniform: &Buffer, n_head: usize) {\n    for h in 0..n_head {\n        let params = AttnParams { head: h as u32, scale: 0.08, .. }; // differs per head\n        queue.write_buffer(shared_uniform, 0, cast(&params));       // write N times\n        let mut pass = encoder.begin_compute_pass();\n        pass.set_bind_group(0, &bgs[h]); // ALL bgs bind the SAME shared_uniform\n        pass.dispatch_workgroups(1, 1, 1);\n    } // wgpu flushes ALL N writes before the encoder runs → every head\n}     // computes with head = n_head-1's params — silent numerical corruption".to_string(),
            opt_pattern: "// optimized: one uniform buffer per slot, wired into pre-built bind groups\nfn per_head_attention_slotted(queue: &Queue, encoder: &mut Encoder, layer: usize, n_head: usize) {\n    // allocated ONCE at init: [n_head] 64-byte uniforms + bind groups,\n    // bg[layer][h] binds uniforms[h] — writes land in DIFFERENT buffers\n    for h in 0..n_head {\n        let params = AttnParams { head: h as u32, scale: 0.08, .. };\n        queue.write_buffer(&self.attn_uniforms[h], 0, cast(&params)); // per-slot write\n        let mut pass = encoder.begin_compute_pass();\n        pass.set_bind_group(0, &self.attn_bgs[layer][h]); // binds uniforms[h]\n        pass.dispatch_workgroups(1, 1, 1);\n    } // concurrent writes cannot collide — every head reads its own params\n}".to_string(),
        },
        // ── derive-params-from-global-invocation-id — uniform loop → grid decode (D) ──
        // Source: riir-gpu forward/batched_dispatch.rs L280-350
        // (dispatch_attention_batched: "One workgroup per (head,
        // query_position). Total = n_head × seq_len ... The kernel implements
        // causal masking internally ... GQA mapping is computed inside the
        // kernel") + forward/dispatch.rs L316-321 ("the batched forward
        // (forward_batched_full) sidesteps it by computing q_pos from the
        // global invocation ID"). The per-position path needs a per-position
        // uniform write, which under the write_buffer ordering bug forces an
        // encoder + submit PER POSITION (the set-causal/block-causal fallback:
        // "Cost: one extra submit per position (~microseconds)"). Encoding
        // the parameter in the DISPATCH GRID eliminates the uniform channel
        // entirely: flatten (head, q_pos) into the X dimension and decode
        // in-kernel — 1 dispatch for all heads × all positions.
        //
        // Distinct from fold-query-tokens-for-occupancy (Batch 23 — folding
        // fills idle SMs when n_head < SM_count): the goal here is
        // eliminating the host-side per-item parameter channel, not
        // occupancy. Distinct from grid-stride-loop (A — iteration
        // coverage): the grid IS the parameter.
        KernelRule {
            rule_id: "derive-params-from-global-invocation-id".to_string(),
            gpu_concept: "instead of writing a per-item parameter (query position, head index, row offset) to a uniform from the host — which under the write_buffer ordering bug forces an encoder + submit PER ITEM — encode the parameter in the dispatch grid: flatten (head, item) into workgroup IDs and derive it in-kernel from global_invocation_id. The batched attention path dispatches n_head × seq_len workgroups in ONE call with causal masking + GQA mapping computed inside the kernel — zero per-position uniform writes, zero per-position submits (vs the encoder-per-position fallback paying ~1 submit/position). Distinct from fold-query-tokens-for-occupancy (goal: fill idle SMs — here the goal is eliminating the host-side parameter channel) and from grid-stride-loop (iteration coverage — the grid IS the parameter)".to_string(),
            ref_shape: "// naive: per-position uniform write + encoder-per-position submit\nfn attention_per_position(tokens: usize, n_head: usize) {\n    for pos in 0..tokens {\n        let params = AttnParams { query_pos: pos as u32, scale: 0.08, .. };\n        queue.write_buffer(&uniform, 0, cast(&params));  // per-position write\n        let mut enc = device.create_command_encoder();   // per-position encoder\n        let mut pass = enc.begin_compute_pass();\n        for h in 0..n_head {\n            pass.set_bind_group(0, &bg[layer][h]);\n            pass.dispatch_workgroups(1, 1, 1);\n        }\n        queue.submit(once(enc.finish()));               // per-position submit\n    } // N submits + N uniform writes; one shared encoder instead hits the\n}     // write_buffer ordering bug (every pos sees the LAST pos's params)".to_string(),
            opt_pattern: "// optimized: the grid encodes (head, q_pos) — decode in-kernel, one dispatch\nfn attention_batched(encoder: &mut Encoder, tokens: usize, n_head: usize) {\n    let params = AttnBatchedParams { seq_len: tokens as u32, n_head: n_head as u32, .. };\n    queue.write_buffer(&uniform, 0, cast(&params));     // ONE shared write\n    let mut pass = encoder.begin_compute_pass();\n    pass.set_bind_group(0, &bg);\n    // one workgroup per (head, query_position) — total n_head × seq_len\n    pass.dispatch_workgroups((n_head * tokens) as u32, 1, 1);\n}\n// WGSL: let gid = global_invocation_id.x;\n//       let head = gid / seq_len; let q_pos = gid % seq_len;\n//       causal mask (kv 0..=q_pos) + GQA head mapping computed in-kernel —\n//       zero per-position uniforms, zero per-position submits".to_string(),
        },
        // ── merge-dispatches-into-single-compute-pass — N passes → 1 pass (D) ──
        // Source: riir-gpu forward/adapter_merge.rs L410-424 (dispatch_lora_
        // merge: "Combine A + B dispatches into single compute pass.
        // Dispatches within a compute pass execute in order, so B reads A's
        // output.") + L654-770 (dispatch_qkv_fused: "Saves 2 compute passes
        // (was 3 separate A+B passes → 1 combined pass) ... 1. Hardware copy
        // input to per-adapter saved-input buffers (no compute pass) 2.
        // Single compute pass with 6 dispatches"). Dependent dispatches do
        // not need separate compute passes: pass-scoped set_pipeline/
        // set_bind_group/dispatch sequences execute IN ORDER with memory
        // ordering between them, so the consumer dispatch reads the
        // producer's buffer writes. Pure copies interleaved with the
        // dispatches ride the copy channel (encoder.copy_buffer_to_buffer
        // — no pass at all).
        //
        // Distinct from chain-dispatches-single-sync-point (Batch 19 —
        // command-buffer-level chaining to amortize SYNC points): this is
        // PASS-level merging (begin/end boundary cost per dispatch) where
        // the one sync was never in question. Composes with
        // per-slot-uniform-buffer-write-ordering (this batch): all uniform
        // writes happen before the pass begins, into per-slot buffers.
        KernelRule {
            rule_id: "merge-dispatches-into-single-compute-pass".to_string(),
            gpu_concept: "dependent dispatches queued in ONE compute pass (set_pipeline/set_bind_group/dispatch × N inside a single begin/end) execute in order with memory ordering between them — the consumer dispatch reads the producer's writes without a separate pass, and N−1 pass begin/end boundaries disappear. The QKV LoRA path merges 6 dispatches (3× A pass + 3× B pass) into one pass; pure input-save copies ride the copy channel (encoder.copy_buffer_to_buffer) outside any pass. Distinct from chain-dispatches-single-sync-point (command-buffer-level sync amortization) — this is pass-level merging; composes with per-slot-uniform-buffer-write-ordering (all uniform writes precede the pass, into per-slot buffers) RIDER (Batch 111 external instance, vllm @ 8277c42e4 -- cache_kernels.cu:456-474): N per-layer cache inserts merge into ONE launch over a pointer-array workspace (grid = (num_tokens, num_layers), each layer rebinds its cache base from kv_cache_ptrs[layer]) -- the pointer-array indirection replacing per-layer launches. RIDER (Batch 142 external instance, cloudrift-ai/emmy @ 321288d8 Apache-2.0 -- the Gemma4-12B blog, quote authority): the launch-count regression is the measurable INTEGRATION-cost signature -- a vLLM plugin boundary runs ~1100 launches/step vs stock ~650 (attention seam + unfused norm/stat/scale glue where torch.compile fuses), worth 0.3-1.0 ms/token (the conclusion's 1-5% of decode performance; the intro's 2-5% self-contradicts -- cite the conclusion). Count LAUNCHES PER STEP, not just kernel time, when pricing an integration boundary; decode stays memory-bound so the loss is pure dispatch overhead, not kernel time. RIDER (Batch 274 / Plan 278 — the INDEPENDENT-COPY-RUN face, PrismML llama.cpp fork @ `66ae2fd88c` #379 'cuda: batch runs of independent same-shape F32 copies into one launch', MIT): a graph RUN of F32-to-F32 CPY nodes sharing shape+strides (elementwise copies, no mma, views/noops between them) collapses to ONE launch — each candidate contributes its (src, dst) pointer pair, capped at 32 pairs, and the launch grid indexes pairs via blockIdx.y with the copy loop over the shared element count. THE NEW DECISION CONTENT is the ORDERING-ELISION INDEPENDENCE ORACLE: batching removes the intra-launch ORDER the graph guaranteed, so admission must PROVE pairwise independence before folding — each candidate's src+dst extents must not overlap any EARLIER candidate's destination (a candidate whose dst aliases an earlier src/dst stays behind, its run unchanged); the oracle is the same overlap-at-admission discipline as fused-read-write-dst-input-overlap-refusal (gpu_109) with OPPOSITE polarity — that rule refuses an alias the allocator invited, this one admits only runs PROVEN alias-free. Measured (source-reported): DFlash2 depth-15 +2.5-2.7%, bit-exact. Count-neutral rider, Batch 274.".to_string(),
            ref_shape: "// naive: one compute pass per dispatch — pass begin/end × N\nfn lora_merge_naive(encoder: &mut Encoder, q: &Adapter, k: &Adapter, v: &Adapter) {\n    for adapter in [q, k, v] {\n        save_input_via_kernel(encoder, adapter);      // copy KERNEL + its own pass\n        { let mut p = encoder.begin_compute_pass();  // pass: A @ x\n          p.set_pipeline(&lora_a); p.set_bind_group(0, &adapter.bg_a);\n          p.dispatch_workgroups(rank_wg, 1, 1); }\n        { let mut p = encoder.begin_compute_pass();  // pass: W@x + B@(A@x)\n          p.set_pipeline(&lora_b); p.set_bind_group(0, &adapter.bg_b);\n          p.dispatch_workgroups(out_wg, 1, 1); }\n    } // 9 passes per layer — begin/end boundary cost × dispatch count\n}".to_string(),
            opt_pattern: "// optimized: hardware copies + ONE pass with 6 ordered dispatches\nfn lora_merge_fused(encoder: &mut Encoder, q: &Adapter, k: &Adapter, v: &Adapter) {\n    for adapter in [q, k, v] { // input saves ride the copy channel — no pass\n        encoder.copy_buffer_to_buffer(adapter.input, 0, &adapter.saved, 0, in_bytes);\n    }\n    let mut pass = encoder.begin_compute_pass();    // ONE pass for all 6\n    for adapter in [q, k, v] {\n        pass.set_pipeline(&lora_a);                  // dispatches within a\n        pass.set_bind_group(0, &adapter.bg_a);       // pass execute IN ORDER —\n        pass.dispatch_workgroups(rank_wg, 1, 1);     // B reads A's writes\n        pass.set_pipeline(&lora_b);\n        pass.set_bind_group(0, &adapter.bg_b);\n        pass.dispatch_workgroups(out_wg, 1, 1);\n    } // 1 pass (was 3), 0 copy passes (was 3) — per-layer pass count 9 → 1\n}".to_string(),
        },
        // ── batched-downloads-single-sync-cycle — N sync cycles → 1 (D) ──
        // Source: riir-gpu buffer.rs L263-280 (download_f32_batched_
        // reuse_into, Issue 421 P0.5): "All copies share one command encoder,
        // one queue.submit, one map_async, and one device.poll(Wait). This
        // reduces GPU sync overhead from N (one per download) to 1 — the
        // dominant cost in launch-overhead-bound paths like the LoRA-Muon
        // optimizer step, where P0 profiling measured each sync at ~1.28 ms
        // regardless of data size." Staging regions are contiguous,
        // non-overlapping, 4-byte aligned (wgpu 23 requires copy size/offset
        // multiples of 4, not 256).
        //
        // Distinct from device-side-argmax-minimal-download (Batch 25 —
        // download SIZE) and chain-dispatches-single-sync-point (Batch 19 —
        // dispatch SYNC): this is download-path sync COUNT, with the
        // staging-layout + single-map mechanics the readback channel needs.
        KernelRule {
            rule_id: "batched-downloads-single-sync-cycle".to_string(),
            gpu_concept: "N GPU→CPU readbacks share ONE command encoder, ONE queue.submit, ONE map_async, and ONE device.poll(Wait) — pack each request into a contiguous, non-overlapping, 4-byte-aligned region of a single grow-only staging buffer, copy all sources in one encoder, map the whole range once, and slice per request. Sync overhead drops from N to 1 — the dominant cost in launch-overhead-bound paths (measured ~1.28 ms per sync regardless of data size; the 4-buffer LoRA-Muon step saves ~3.8 ms/step). Distinct from device-side-argmax-minimal-download (download SIZE) and chain-dispatches-single-sync-point (dispatch sync) — this is readback sync COUNT with the staging-layout + single-map mechanics".to_string(),
            ref_shape: "// naive: one encoder + submit + map + poll per download — sync × N\nfn optimizer_step_naive(grads: &[Buffer]) -> Vec<Vec<f32>> {\n    grads.iter().map(|buf| {\n        let staging = device.create_buffer(MAP_READ | COPY_DST, buf.size());\n        let mut enc = device.create_command_encoder();\n        enc.copy_buffer_to_buffer(buf, 0, &staging, 0, buf.size());\n        queue.submit(once(enc.finish()));\n        staging.slice(..).map_async(Read, cb);\n        device.poll(Wait);                  // ~1.28 ms PER download (measured)\n        read_mapped(&staging)\n    }).collect()                             // 4 buffers = 4 full sync cycles\n}".to_string(),
            opt_pattern: "// optimized: N copies, ONE encoder/submit/map/poll — sync × 1\nfn optimizer_step_batched(reqs: &mut [DownloadReq], staging: &mut Staging) {\n    let total: usize = reqs.iter().map(|r| r.count).sum();\n    staging.ensure_capacity(device, total);         // grow-only staging reuse\n    let mut layout = 0;                              // contiguous 4-byte-aligned\n    for r in &reqs { r.offset = layout; layout += r.count; } // (wgpu: 4B, not 256B)\n    let mut enc = device.create_command_encoder();\n    for r in &reqs {\n        enc.copy_buffer_to_buffer(r.src, 0, staging.buf(), (r.offset*4) as u64, (r.count*4) as u64);\n    }\n    queue.submit(once(enc.finish()));                // 1 submit\n    staging.buf().slice(..(layout*4) as u64).map_async(Read, cb);\n    device.poll(Wait);                               // 1 poll (~1.28 ms, once)\n    let mapped = get_mapped(staging);\n    for r in reqs {                                  // slice per request\n        r.output.clear();\n        r.output.extend_from_slice(cast(&mapped[r.offset*4 .. (r.offset+r.count)*4]));\n    } // 4 buffers, 1 sync cycle — bit-identical results, ~3.8 ms saved/step\n}".to_string(),
        },
        // ── chunk-encoder-submits-below-driver-limit — one encoder → chunks (D) ──
        // Source: riir-gpu forward/mod.rs L1403-1411 ("Chunked encoder
        // submission to avoid Metal compute pass limit. Each position
        // creates ~24 compute passes ... Metal deadlocks with too many
        // compute passes per encoder. CHUNK_SIZE=2 → ~48 passes/chunk." +
        // "Issue 016: chunk_size is now tunable via GpuContext (env var
        // RIIR_GPU_CHUNK_SIZE)") + L1456-1458 ("Submit each chunk immediately
        // to avoid Metal driver stall from accumulating too many write_buffer
        // + command buffers before submit"). The driver ceiling is a hard
        // correctness constraint (DEADLOCK, not slowdown) and there is a
        // second stall from accumulating too many write_buffer calls before
        // a submit — so chunk AND flush.
        //
        // Distinct from chain-dispatches-single-sync-point (sync-count
        // choice): this is a driver-limit-forced submission granularity —
        // the chunk is a correctness bound first, a tuning knob second
        // (env-tunable: tiny-model loops shrink it, long-prefill raises it).
        KernelRule {
            rule_id: "chunk-encoder-submits-below-driver-limit".to_string(),
            gpu_concept: "drivers impose a compute-pass-per-encoder ceiling (Metal: a few dozen — past it the encoder DEADLOCKS, a hard failure not a slowdown) and a separate stall from accumulating too many write_buffer + command buffers before submit. Chunk recorded work so each encoder stays ~48 passes (~24 passes/position × chunk_size 2), submit each chunk IMMEDIATELY, and make the chunk size env-tunable (tiny-model training loops shrink it, long-prefill raises it). Distinct from chain-dispatches-single-sync-point (a sync-count CHOICE) — this chunking is a driver-limit-forced submission granularity: correctness bound first, tuning knob second".to_string(),
            ref_shape: "// naive: one encoder for the whole sequence — Metal deadlocks\nfn forward_one_encoder(tokens: usize, n_layer: usize) {\n    let mut enc = device.create_command_encoder();\n    for pos in 0..tokens {\n        for layer in 0..n_layer {\n            dispatch_layer(&mut enc, layer, pos);  // ~24 compute passes / position\n        }\n    }\n    queue.submit(once(enc.finish()));\n} // 256 positions × 6 layers × 24 passes ≈ 36,864 passes in ONE encoder —\n  // past the Metal compute-pass-per-encoder limit the driver DEADLOCKS".to_string(),
            opt_pattern: "// optimized: chunked submission — bounded passes per encoder, submit each chunk\nfn forward_chunked(tokens: usize, n_layer: usize, chunk_size: usize) { // default 2\n    for chunk_start in (0..tokens).step_by(chunk_size) {\n        let chunk_end = (chunk_start + chunk_size).min(tokens);\n        let mut enc = device.create_command_encoder();\n        for pos in chunk_start..chunk_end {\n            for layer in 0..n_layer {\n                dispatch_layer(&mut enc, layer, pos);  // ~48 passes / chunk\n            }\n        }\n        queue.submit(once(enc.finish())); // submit immediately — don't let\n    }                                    // write_buffer + cmd buffers pile up\n} // env-tunable chunk (RIIR_GPU_CHUNK_SIZE): the driver limit is the ceiling\n".to_string(),
        },
        // ── prebuilt-bind-groups-fixed-buffer-combos — per-dispatch BG → init (A) ──
        // Source: riir-gpu forward/forward_buffers.rs L530-548 + L702-703
        // ("Pre-built bind groups (avoids create_bind_group per dispatch)"
        // ... "These eliminate ~768 create_bind_group calls per forward pass
        // (seq_len=16, n_layer=6)") + forward/mod.rs L412-501 (P0-1
        // per-adapter LoRA bind groups: "Each adapter has fixed input/output
        // buffers determined by its target, so we can pre-build bind groups
        // referencing them"). Bind groups bind BUFFERS, not data — when a
        // dispatch site's buffer combo is fixed for the lifetime of the
        // pipeline (attention reads key_cache[layer]/head_query/head_output,
        // elementwise reads hidden/residual/temp_out), the BG is a constant
        // and belongs in init, not on the hot path. Dynamic combos (batched
        // full-sequence ops) still build per call.
        //
        // Distinct from hot-swap-weights-preserve-autotune-caches (D —
        // amortizing RUNTIME caches across weight changes): this amortizes
        // BIND GROUP creation across calls — the buffers are fixed, only
        // their contents change.
        KernelRule {
            rule_id: "prebuilt-bind-groups-fixed-buffer-combos".to_string(),
            gpu_concept: "a bind group binds BUFFERS, not data — when a dispatch site's buffer combination is fixed for the pipeline's lifetime (attention reads head_query + key_cache[layer] + value_cache[layer] + head_output, the residual add reads hidden + residual + temp_out, each LoRA adapter has fixed input/output buffers), the bind group is a CONSTANT: build all of them once at init ([n_layer][n_head] attention BGs, [n_layer × 6] adapter BGs, the fixed elementwise combos) and only set the pre-built handles on the hot path (~768 create_bind_group calls per forward pass → 0). Dynamic-buffer combos still build per call. Distinct from hot-swap-weights-preserve-autotune-caches (amortizing runtime caches across weight changes) — the buffers here never change identity, only contents".to_string(),
            ref_shape: "// naive: create_bind_group per dispatch — allocation churn on the hot path\nfn dispatch_layer_hot(device: &Device, encoder: &mut Encoder, layer: usize, head: usize) {\n    let bg = device.create_bind_group(&BindGroupDescriptor {  // EVERY dispatch\n        layout: &pipeline.layout,\n        entries: &[entry(0, &bufs.head_query), entry(1, &bufs.key_cache[layer]),\n                   entry(2, &bufs.value_cache[layer]), entry(3, &bufs.head_output),\n                   entry(4, &uniforms[head])],\n    });\n    let mut pass = encoder.begin_compute_pass();\n    pass.set_bind_group(0, &bg);\n    pass.dispatch_workgroups(1, 1, 1);\n} // ~768 create_bind_group calls per forward pass (seq_len=16, n_layer=6)".to_string(),
            opt_pattern: "// optimized: pre-build BGs once at init for every fixed buffer combination\nstruct Scratch { attn_bgs: Vec<Vec<BindGroup>>, bg_add_residual: BindGroup, /* .. */ }\nimpl Scratch {\n    fn new(device: &Device, pipelines: &Pipelines, bufs: &Activations, n_layer: usize, n_head: usize) -> Self {\n        // [n_layer][n_head] — the (layer, head) pair determines a FIXED\n        // buffer combo; build every bind group ONCE at init:\n        let attn_bgs = (0..n_layer).map(|l| (0..n_head).map(|h|\n            device.create_bind_group(fixed_combo(l, h, pipelines, bufs))).collect()).collect();\n        // + fixed elementwise combos (hidden,residual,temp), (mlp_hidden,temp),\n        // + per-adapter LoRA BGs [n_layer × 6], + the lm_head BG ...\n    }\n}\nfn dispatch_layer_prebuilt(state: &Scratch, encoder: &mut Encoder, layer: usize, head: usize) {\n    let mut pass = encoder.begin_compute_pass();\n    pass.set_bind_group(0, &state.attn_bgs[layer][head]); // zero allocation\n    pass.dispatch_workgroups(1, 1, 1);\n} // hot path only SETS pre-built handles — dynamic-buffer combos still\n  // build per call".to_string(),
        },
        // ── host-cache-immutable-device-weights — per-call download → init clone (A) ──
        // Source: riir-gpu forward/mod.rs L111-114 + L869-873 ("Cache weight
        // data on CPU to eliminate per-forward-call GPU downloads ... These
        // are constant weights that never change — downloading them every
        // call was the #1 bottleneck (2 GPU sync stalls per forward pass)")
        // + L1270-1288 (compute_embeddings: gathers wte[token_id] + wpe[pos]
        // from the cached rows — "Slice-based extend helps LLVM
        // auto-vectorize the wte+wpe add" — and uploads the result once).
        // Counter-example in the same tree: forward/dispatch.rs L1019-1030
        // downloads a_log/dt_bias/linear_norm EVERY layer EVERY token in the
        // DeltaNet path — the unoptimized shape this rule warns about.
        //
        // Distinct from precompute-scalar-reduction (I — hoisting an
        // invariant KERNEL reduction out of a loop): this is hoisting the
        // DEVICE READ of immutable data out of the call, keeping the host
        // copy authoritative for cheap gathers.
        KernelRule {
            rule_id: "host-cache-immutable-device-weights".to_string(),
            gpu_concept: "constants that live on the GPU but are needed host-side every call (embedding tables wte/wpe, norm gammas, small bias vectors) are cloned to CPU ONCE at init — per-call download_f32 of the same bytes cost 2 full GPU sync stalls per forward pass, measured as the #1 forward bottleneck. The host copy then serves cheap gathers (token embedding lookup = row slice + add, auto-vectorized) whose result is uploaded in ONE write. Apply the reverse test too: any per-layer/per-token download of a weight-shaped buffer (a_log, dt_bias, linear_norm) is this rule unapplied. Distinct from precompute-scalar-reduction (hoisting an invariant KERNEL reduction) — this hoists the DEVICE READ of immutable data out of the hot call".to_string(),
            ref_shape: "// naive: download the embedding tables from the GPU every forward call\nfn embeddings_via_gpu_download(device: &Device, queue: &Queue, wte: &Buffer,\n                               wpe: &Buffer, n: usize, token_ids: &[usize]) -> Vec<f32> {\n    let wte_host = download_f32(device, queue, wte, vocab * n);  // GPU sync stall 1\n    let wpe_host = download_f32(device, queue, wpe, block * n);  // GPU sync stall 2\n    let mut out = Vec::with_capacity(token_ids.len() * n);\n    for (pos, &t) in token_ids.iter().enumerate() {\n        for i in 0..n { out.push(wte_host[t*n + i] + wpe_host[pos*n + i]); }\n    }\n    out\n} // constants re-downloaded per call — the #1 measured forward bottleneck".to_string(),
            opt_pattern: "// optimized: cache immutable weights host-side at init; gather on CPU, upload once\nstruct GpuForwardPass {\n    cached_wte: Vec<f32>,             // cloned ONCE in new() — weights never change\n    cached_wpe: Vec<f32>,\n    embeddings_buf: RefCell<Vec<f32>>, // pre-allocated scratch (clear + reuse)\n}\nimpl GpuForwardPass {\n    fn compute_embeddings(&self, token_ids: &[usize]) {\n        let n = self.config.n_embd;\n        let mut emb = self.embeddings_buf.borrow_mut();\n        emb.clear(); emb.reserve(token_ids.len() * n);\n        for (pos, &t) in token_ids.iter().enumerate() {\n            let wte = &self.cached_wte[t*n .. t*n + n];    // slice-based extend:\n            let wpe = &self.cached_wpe[pos*n .. pos*n + n];  // contiguous writes,\n            emb.extend(wte.iter().zip(wpe).map(|(a, b)| a + b)); // auto-vectorized\n        }\n        // ONE upload — zero GPU syncs, zero per-call downloads\n        queue.write_buffer(&self.scratch.embeddings, 0, cast(&emb));\n    }\n}".to_string(),
        },
        // ── defer-readback-until-after-submit — mid-encoder read → record (A) ──
        // Source: riir-gpu forward/mod.rs L291-295 (speft_pending_corrections:
        // "Plan 162 fix: avoids downloading stale GPU data within a command
        // encoder") + L1173-1183 (apply_speft_corrections: "must be called
        // AFTER the command encoder is submitted and GPU operations complete,
        // so activation buffers contain fresh data") + forward/adapter_merge.rs
        // L576-578 ("The old CPU fallback downloaded stale GPU data (input/
        // output buffers not yet written by pending GPU commands)") +
        // forward/mod.rs L1365-1368 ("avoiding the data race that would occur
        // if download_f32 (which internally submits command buffers) were
        // called during encoder construction"). Downloading mid-construction\        // reads PRE-submission buffer contents — the recorded encoder hasn't
        // executed, so the download sees the previous pass's data AND any
        // correction uploaded then gets overwritten when the recorded work
        // finally runs. A mid-pipeline download also internally submits its
        // own command buffers — breaking the encoder being built. The
        // pattern: RECORD what needs fixing (a small correction list),
        // finish + submit, then read fresh data + apply + upload back.
        //
        // Distinct from no-blocking-sync-in-async-pipeline (D — don't block
        // the HOST thread): this is a correctness rule about WHAT the read
        // returns mid-construction (stale state), independent of blocking.
        KernelRule {
            rule_id: "defer-readback-until-after-submit".to_string(),
            gpu_concept: "never download (map + read) GPU buffers while an encoder referencing them is still under construction — the recorded work has not executed, so the read returns PRE-submission stale state, and any correction uploaded mid-construction is later overwritten by the recorded pass when it runs; the readback's internal submit also breaks the encoder being built. Instead RECORD the needed fixups (small correction list of (buffer, op) pairs), finish + submit the encoder, then download FRESH data, apply, and upload back. Distinct from no-blocking-sync-in-async-pipeline (don't block the host thread) — this is about WHAT a mid-construction read returns (stale state), independent of blocking".to_string(),
            ref_shape: "// naive: read GPU buffers MID-encoder-construction — stale pending state\nfn speft_merge_broken(&self, encoder: &mut Encoder, input: &Buffer, output: &Buffer) {\n    dispatch_plain_matmul(encoder, w, input, output, ..);   // recorded, NOT run yet\n    let out_now = download_f32(device, queue, output, dim); // reads PRE-submission\n    let in_now  = download_f32(device, queue, input, dim);  // contents — the\n    // fix up out_now += sparse_delta @ in_now ...              // encoder hasn't\n    queue.write_buffer(output, 0, cast(&corrected));        // executed yet!\n}   // then the recorded matmul OVERWRITES the correction — silently wrong;\n    // download_f32's internal submit also breaks the in-flight encoder".to_string(),
            opt_pattern: "// optimized: record the correction; apply AFTER submit + completion\nstruct Pass { pending: RefCell<Vec<Correction>> }\nfn speft_merge_deferred(&self, encoder: &mut Encoder, layer: usize, target: Target) {\n    dispatch_plain_matmul(encoder, w, input, output, ..);     // recorded\n    self.pending.borrow_mut().push(Correction { layer, target }); // just RECORD\n}                                                            // (zero GPU reads)\nfn apply_corrections_after_submit(&self) {\n    for c in std::mem::take(&mut *self.pending.borrow_mut()) {\n        let output = download_f32_reuse(.., output_buf(c), ..);  // FRESH: encoder\n        let input  = download_f32_reuse(.., input_buf(c), ..);   // already ran\n        apply_sparse_delta(&mut output, &input, &c);              // post-pass data\n        queue.write_buffer(output_buf(c), 0, cast(&output));\n    }\n}   // called after queue.submit + poll — every download sees fresh contents".to_string(),
        },
        // ── staging-packed-uniforms-encoder-ordered-copies — N writes → 1 upload (D) ──
        // Source: riir-gpu backward/mod.rs L15 (architecture note: "Uniforms
        // → staging buffer + copy_buffer_to_buffer per dispatch") + L1293-1295
        // ("All uniform params + grad_logits packed into one CPU buffer,
        // uploaded once, then copy_buffer_to_buffer within the encoder") +
        // L1505-1512 (single write_buffer of the packed staging vec) +
        // backward/staging.rs L31-38 (push aligns each Pod to 16 bytes for
        // GPU copy compatibility) + backward_dflash/mod.rs L774-780 + L944 +
        // L980-986 (per-dispatch staging→uniform copies). The complementary
        // fix to per-slot buffers: keep ONE shared uniform buffer and update
        // it with encoder-ordered DEVICE-SIDE copies — a copy recorded
        // inside the command encoder lands exactly between the surrounding
        // dispatches, so the shared buffer can safely change value per
        // dispatch (per-layer dims, per-position offsets). Bonus: hundreds
        // of small uniform writes collapse to ONE packed host→device upload.
        //
        // Distinct from per-slot-uniform-buffer-write-ordering (Batch 26):
        // that allocates N buffers so concurrent queue writes cannot
        // collide; this keeps 1 buffer and moves the update onto the
        // encoder timeline — use when the param legitimately changes per
        // dispatch and upload batching matters. Composes with
        // identical-uniforms-share-single-slot (the idempotent-write
        // exemption) and persistent-zeros-source-encoder-copy-clear (the
        // same timeline trick applied to buffer clears).
        KernelRule {
            rule_id: "staging-packed-uniforms-encoder-ordered-copies".to_string(),
            gpu_concept: "pack every uniform parameter for the whole pass into ONE 16-byte-aligned CPU staging vec, upload it with ONE queue.write_buffer, then move each dispatch's slice into the shared uniform buffer via encoder.copy_buffer_to_buffer recorded immediately before the consuming dispatch — a copy on the encoder timeline lands exactly between the surrounding dispatches, so ONE shared uniform buffer can safely change value per dispatch (per-layer dims, per-position offsets). Hundreds of small per-dispatch queue writes collapse to one packed upload; queue writes alone would all flush pre-encoder (last-write-wins). Distinct from per-slot-uniform-buffer-write-ordering (N buffers so writes cannot collide) — this is the 1-buffer encoder-timeline alternative; composes with identical-uniforms-share-single-slot".to_string(),
            ref_shape: "// naive: per-dispatch queue.write_buffer on ONE shared uniform — every\n// dispatch sees the LAST write + N small host uploads\nfn backward_naive(encoder: &mut Encoder, queue: &Queue, shared: &Buffer, layers: usize) {\n    for layer in 0..layers {\n        let mm = MatmulTParams { m: 768, n: 1024, p: 1, _pad: 0 };\n        queue.write_buffer(shared, 0, cast(&mm));            // small upload 1\n        dispatch_matmul_t(encoder, layer, shared);\n        let elem = ElementwiseParams { count: 768, _pad0: 0, _pad1: 0, _pad2: 0 };\n        queue.write_buffer(shared, 0, cast(&elem));          // small upload 2 —\n        dispatch_add(encoder, layer, shared);                // overwrites mm!\n    } // wgpu flushes ALL queue writes before the encoder runs → every\n}     // dispatch reads the LAST params — silent corruption + N uploads".to_string(),
            opt_pattern: "// optimized: pack ALL uniform params once, distribute via encoder-ordered copies\nstruct StagingData { data: Vec<u8>, mm_t: u64, elem: u64 } // per-site byte offsets\nfn backward_staged(encoder: &mut Encoder, queue: &Queue, staging_buf: &Buffer,\n                   shared: &Buffer, sd: &StagingData, layers: usize) {\n    // ONE upload for the whole pass (16-byte-aligned pushes, CPU-side)\n    queue.write_buffer(staging_buf, 0, &sd.data);\n    for layer in 0..layers {\n        // encoder-timeline copies: land EXACTLY between the dispatches\n        encoder.copy_buffer_to_buffer(staging_buf, sd.mm_t, shared, 0, MM_BYTES);\n        dispatch_matmul_t(encoder, layer, shared);  // reads matmul params\n        encoder.copy_buffer_to_buffer(staging_buf, sd.elem, shared, 0, ELEM_BYTES);\n        dispatch_add(encoder, layer, shared);       // reads elem params\n    } // ONE shared uniform buffer, safely re-written between dispatches —\n}     // recorded order guarantees each dispatch reads its own params".to_string(),
        },
        // ── identical-uniforms-share-single-slot — N slots → 1 when idempotent (D) ──
        // Source: riir-gpu backward/mod.rs L192-196 ("Shared uniform for
        // batched elementwise adds. All batched adds use the same count
        // (seq_len * n_embd), so a single uniform buffer written once per
        // backward_pass_batched call suffices") + L1447-1452 ("All three
        // adds share an identical uniform (count = n_embd, constant) —
        // push once and reuse the offset. Staging data is read-only after
        // upload.") + backward/dispatch.rs L315-320 ("the caller is
        // responsible for providing a unique uniform if the count differs
        // across calls in the same encoder"). The write_buffer last-write-
        // wins hazard only bites when the writes DIFFER: if every dispatch
        // sharing a uniform writes byte-identical params, the "last" write
        // is idempotent with all earlier ones and flush order is moot — so
        // N uniform buffers + N bind groups collapse to 1.
        //
        // Distinct from per-slot-uniform-buffer-write-ordering (the hazard
        // rule): this is the precise EXEMPTION — sharing is provably safe
        // iff writes are byte-identical, and the safety contract must be
        // stated at the API boundary (the dispatch doc). The diverging-
        // count case is what per-slot buffers / staging copies are for.
        KernelRule {
            rule_id: "identical-uniforms-share-single-slot".to_string(),
            gpu_concept: "the write_buffer last-write-wins hazard only bites when the writes DIFFER — if every dispatch sharing a uniform buffer in one encoder writes byte-identical params (all batched elementwise adds use count = seq_len × n_embd), the last write is idempotent with all earlier ones and flush order is moot: share ONE uniform buffer + ONE bind group instead of N (and in staging terms, push the value once and reuse the byte offset — staging data is read-only after upload). The safety contract must live at the API boundary: 'caller must provide a unique uniform if the count differs across calls in the same encoder'. Distinct from per-slot-uniform-buffer-write-ordering (the hazard rule) — this is the exemption; diverging params need per-slot buffers or staging-packed encoder-ordered copies".to_string(),
            ref_shape: "// naive: per-slot uniform buffers even though every site writes IDENTICAL params\nfn adds_naive(device: &Device, queue: &Queue, encoder: &mut Encoder,\n              slots: &[Buffer], bgs: &[BindGroup], n: usize) {\n    for i in 0..n {\n        let p = ElemParams { count: 768 };                 // same value every site\n        queue.write_buffer(&slots[i], 0, cast(&p));       // N buffers at init\n        let mut pass = encoder.begin_compute_pass();\n        pass.set_bind_group(0, &bgs[i]);                  // N bind groups at init\n        pass.dispatch_workgroups(768 / 256, 1, 1);\n    } // N uniforms + N bind groups for ONE distinct value — per-slot is\n}     // for params that DIFFER, not for paranoia".to_string(),
            opt_pattern: "// optimized: sharing is safe iff every write is byte-identical — use ONE slot\nfn adds_shared(queue: &Queue, encoder: &mut Encoder, shared: &Buffer, bg: &BindGroup,\n               n: usize, staging: &mut Staging) {\n    // all sites write count = seq_len * n_embd — byte-identical: the \"last\"\n    // write is idempotent with all earlier ones, so flush order is moot\n    let off = staging.push_once(ElemParams { count: 768 }); // push ONCE,\n    encoder.copy_buffer_to_buffer(staging.buf, off, shared, 0, ELEM_BYTES); // reuse offset\n    let mut pass = encoder.begin_compute_pass();\n    for _ in 0..n {\n        pass.set_bind_group(0, bg);         // ONE bind group, set repeatedly\n        pass.dispatch_workgroups(768 / 256, 1, 1);\n    } // N uniform buffers + N bind groups → 1 (a single diverging count\n}     // silently reintroduces the hazard — keep the contract at the API)".to_string(),
        },
        // ── persistent-zeros-source-encoder-copy-clear — per-clear writes → init+copy (A) ──
        // Source: riir-gpu backward/mod.rs L235-241 (batched_zeros_buf:
        // "Used via encoder.copy_buffer_to_buffer inside the per-layer loop
        // (NOT queue.write_buffer, which would hit the write_buffer ordering
        // bug across layers)") + L267-268 (zeros_buf: COPY_SRC) +
        // backward/dispatch.rs L621-631 + L1106-1110 (clear_buffer helper:
        // "Clear a buffer by copying from the pre-initialized zeros buffer")
        // + backward_dflash/mod.rs L309-310 (init once) + L618 ("If buffer
        // is larger than zeros_buf, clear in chunks") + L1216-1218
        // (d_q/d_k_concat/d_v_concat cleared every layer × position).
        // Re-initializing an accumulator BETWEEN dispatches needs encoder-
        // timeline ordering (a queue write flushes before the whole encoder
        // → layer 1 accumulates on top of layer 0's values). The zeros
        // source is allocated + zeroed once at init (COPY_SRC); each clear
        // is a device-side copy with zero host→device bandwidth; wgpu has
        // no buffer-fill API, so the zeros source IS the fill.
        //
        // Distinct from per-slot-uniform-buffer-write-ordering (uniform
        // slots): this is DATA-buffer re-initialization between dispatches,
        // which per-slot buffers cannot express (same buffer, re-zeroed).
        // Distinct from staging-packed-uniforms-encoder-ordered-copies
        // (param distribution): this distributes a constant zero fill from
        // a dedicated device-resident source.
        KernelRule {
            rule_id: "persistent-zeros-source-encoder-copy-clear".to_string(),
            gpu_concept: "clearing an accumulator buffer between dispatches inside one encoder must happen on the encoder timeline: keep a device buffer pre-zeroed once at init (COPY_SRC, sized to the max clear target) and clear via encoder.copy_buffer_to_buffer from it — a queue.write_buffer of zeros flushes before the WHOLE encoder runs, so every layer's clear lands up front and layer 1 accumulates on top of layer 0's values. Each clear is a device-side copy (zero host→device bandwidth; wgpu has no buffer-fill API so the zeros source IS the fill); if a target is larger than the zeros source, clear in chunks. Distinct from per-slot-uniform-buffer-write-ordering (uniform slots) — this is data-buffer re-init between dispatches; distinct from staging-packed-uniforms (param distribution) — this distributes a constant fill".to_string(),
            ref_shape: "// naive: re-zero accumulators via queue.write_buffer per layer — all clears\n// flush BEFORE the encoder runs, so later layers accumulate on stale state\nfn layer_zero_naive(queue: &Queue, encoder: &mut Encoder, d_q: &Buffer,\n                    zeros_host: &[f32], n: usize, layers: usize) {\n    for layer in 0..layers {\n        queue.write_buffer(d_q, 0, zeros_host);       // clear N — flushes early\n        dispatch_attn_backward(encoder, layer, d_q); // accumulates d_q +=\n    } // ALL n_layer zero-writes flush before ANY dispatch executes →\n}     // layer 1's attention accumulates ON TOP of layer 0's values".to_string(),
            opt_pattern: "// optimized: persistent device zeros + encoder-ordered copy clears\nstruct Pass { zeros_buf: Buffer } // allocated + zeroed ONCE at init (COPY_SRC)\nimpl Pass {\n    fn clear(&self, encoder: &mut Encoder, buf: &Buffer, bytes: u64) {\n        let mut off = 0u64;                     // if buf > zeros_buf, chunk\n        while off < bytes {\n            let n = (bytes - off).min(self.zeros_buf.size());\n            encoder.copy_buffer_to_buffer(&self.zeros_buf, 0, buf, off, n);\n            off += n;\n        }\n    }\n}\nfn layer_zero_encoder_ordered(pass: &Pass, encoder: &mut Encoder, d_q: &Buffer,\n                              n: usize, layers: usize) {\n    for layer in 0..layers {\n        pass.clear(encoder, d_q, (n * 4) as u64);  // encoder-timeline copy:\n        dispatch_attn_backward(encoder, layer, d_q); // lands exactly between\n    } // the dispatches — each layer accumulates from a freshly-cleared\n}     // buffer; zero host→device bandwidth per clear, no fill kernel".to_string(),
        },
        // ── fuse-sibling-target-grads-single-kernel — N×2 → 2 dispatches (C) ──
        // Source: riir-gpu backward/dispatch.rs L678-685 ("OFT fused QKV
        // backward: computes d_input and d_r for Q, K, V in 2 dispatches.
        // Plan 160 T7: Replaces 6 separate backward dispatches (3 d_input +
        // 3 d_r) with 2.") + L707 ("all 3 saved the same input" — the shared
        // saved-input binding) + L849-851 ("Total params = 3 sub-projections
        // * num_blocks * params_per_block"). N sibling targets (Q/K/V) each
        // running the same two backward kernels fuse per KERNEL KIND, not
        // per target: one dispatch computes the summed d_input = Σ Rᵢᵀ·gᵢ
        // in-kernel (one output write — no combine add pass), one dispatch
        // computes all N parameter grads in a single grid (N × blocks ×
        // per_block). All adapters' buffers pack into ONE wide bind group;
        // the shared saved input is bound once.
        //
        // Distinct from dual-grad-same-pass (Batch 24 — two product-rule
        // partials of ONE op sharing a subexpression): these are gradients
        // of N DIFFERENT weight sets sharing only the saved input, fused by
        // sibling symmetry. Distinct from fan-out-shared-input-single-sync
        // (Batch 25 — the forward mirror, 1 input → N dispatches): this is
        // N grads → 1 summed/collected output pair. Distinct from
        // merge-dispatches-into-single-compute-pass (pass-boundary merging):
        // this is kernel-BODY fusion — the reduction across siblings
        // happens in-shader.
        KernelRule {
            rule_id: "fuse-sibling-target-grads-single-kernel".to_string(),
            gpu_concept: "when N sibling targets (the Q/K/V projections) each need the same two backward kernels, fuse per KERNEL KIND, not per target: one dispatch computes the summed input-gradient in-kernel (d_input = Σ Rᵢᵀ·gᵢ — one output write, no separate combine-add pass) and one dispatch computes all N parameter gradients in a single grid sized N × num_blocks × params_per_block. Pack every adapter's R matrix, gradient, and the SHARED saved input (all siblings saved the same input — bind it once) into ONE wide bind group. 6 dispatches → 2 per layer. Distinct from dual-grad-same-pass (two partials of ONE op sharing a subexpression — these are grads of N different weight sets sharing only the input) and from fan-out-shared-input-single-sync (forward 1→N mirror) — this is N grads → 1 collected output pair via kernel-body fusion, not pass merging".to_string(),
            ref_shape: "// naive: per-adapter backward — 2 dispatches × N sibling targets + combine adds\nfn oft_qkv_naive(encoder: &mut Encoder, q: &Adapter, k: &Adapter, v: &Adapter) {\n    for adapter in [q, k, v] {                          // 3 siblings, same math\n        { let mut p = encoder.begin_compute_pass();     // d_input for THIS one\n          p.set_pipeline(&din); p.set_bind_group(0, &adapter.bg_din);\n          p.dispatch_workgroups(adapter.dim_wg, 1, 1); }\n        { let mut p = encoder.begin_compute_pass();     // d_r for THIS one\n          p.set_pipeline(&dr); p.set_bind_group(0, &adapter.bg_dr);\n          p.dispatch_workgroups(adapter.blocks_wg, 1, 1); }\n        // ...then a separate add pass combines the three d_input terms\n    } // 6 dispatches + N-1 combine adds per layer — the siblings read the\n}     // SAME saved input and share the kernel bodies; only buffers differ".to_string(),
            opt_pattern: "// optimized: fuse per KERNEL KIND across the N siblings — 2 dispatches total\nfn oft_qkv_fused(encoder: &mut Encoder, qkv: &QkvBundle, saved_input: &Buffer) {\n    // ONE wide bind group: R_q,R_k,R_v + g_q,g_k,g_v + the SHARED saved\n    // input (all three saved the same input — bound once) + outputs\n    let bg_wide = make_bg(&layout, &[entry(0, &qkv.r_q), entry(1, &qkv.r_k),\n        entry(2, &qkv.r_v), entry(3, &qkv.g_q), entry(4, &qkv.g_k),\n        entry(5, &qkv.g_v), entry(6, saved_input), entry(7, &qkv.d_input),\n        entry(8, &qkv.weight_delta), entry(9, &qkv.uniform)]);\n    let mut p = encoder.begin_compute_pass();\n    p.set_pipeline(&din_fused);                  // d_input = Σ Rᵢᵀ·gᵢ summed\n    p.set_bind_group(0, &bg_wide);               // IN-KERNEL — one output\n    p.dispatch_workgroups(qkv.dim_wg, 1, 1);     // write, no add pass\n    p.set_pipeline(&dr_fused);                   // ALL N param grads in ONE\n    p.set_bind_group(0, &bg_wide);               // grid: N × blocks ×\n    p.dispatch_workgroups(3 * qkv.blocks * qkv.per_block, 1, 1); // per_block\n    // 6 dispatches + combine adds → 2; the summed d_input needs no add pass\n}".to_string(),
        },
        // ── per-layer-strided-grad-destination — post-hoc de-interleave → offsets (D) ──
        // Source: riir-gpu backward_dflash/mod.rs L205-207 (per_layer_ctx_
        // grads: "When set, the DFlare context gradient path writes to
        // per-layer offsets in this buffer instead of accumulating into
        // d_h_ctx_buf. Layout: [num_draft_layers * seq_len * n_embd],
        // layer-major.") + L1533-1541 (destination selection at dispatch
        // time). When a downstream consumer needs a gradient PARTITIONED per
        // layer, don't run a second pass or post-hoc de-interleave: the
        // SAME dispatches that accumulate into the shared buffer can write
        // layer-major strided offsets (layer_idx * seq_len * n_embd + pos *
        // n_embd) into a pre-partitioned buffer — the per-layer split falls
        // out of the existing pass for free, and downstream consumers read
        // contiguous layer slices.
        //
        // Distinct from dual-grad-same-pass (two gradients from one
        // kernel): here ONE gradient gets consumer-selected DESTINATIONS.
        // Distinct from layout-permute-for-coalescing (layout for memory
        // coalescing): the destination choice eliminates a re-pass, not a
        // stride.
        KernelRule {
            rule_id: "per-layer-strided-grad-destination".to_string(),
            gpu_concept: "when a downstream consumer needs a gradient partitioned per layer (per-layer optimizer steps, per-layer analysis), don't accumulate into one shared buffer and de-interleave afterward — write directly to layer-major strided offsets (layer_idx × seq_len × n_embd + pos × n_embd) in one pre-partitioned buffer, selected at dispatch time. The SAME dispatches that would accumulate into the shared buffer produce the per-layer split for free (no second pass, no post-hoc scatter), and downstream consumers read contiguous layer slices instead of strided extraction. Runtime-switchable: the destination is chosen per pass, so shared-accumulator and per-layer modes share one code path. Distinct from dual-grad-same-pass (two grads one kernel — here one grad gets consumer-selected destinations) and from layout-permute-for-coalescing (coalescing — this eliminates a re-pass)".to_string(),
            ref_shape: "// naive: accumulate into ONE shared buffer, de-interleave per layer afterward\nfn ctx_grad_naive(d_h_ctx: &mut [f32], per_layer: &mut [Vec<f32>], layers: usize,\n                  seq: usize, n: usize) {\n    for layer in 0..layers {\n        // every layer accumulates into d_h_ctx[pos * n + i] — one shared dest\n        accumulate_ctx_grad(layer, d_h_ctx);\n    }\n    // downstream needs PER-LAYER gradients → post-hoc de-interleave pass:\n    for layer in 0..layers {\n        for pos in 0..seq {\n            for i in 0..n {\n                per_layer[layer][pos * n + i] = d_h_ctx[pos * n + i];\n            }\n        }\n    } // a second full pass + a copy per element — the split existed for free\n}".to_string(),
            opt_pattern: "// optimized: consumer-selected destination offsets — same dispatches, split free\nstruct BackwardPass {\n    d_h_ctx_buf: Buffer,               // shared accumulator (default mode)\n    per_layer_grads: Option<Buffer>,   // [layers × seq × n], layer-major\n}\nimpl BackwardPass {\n    fn ctx_grad_dest(&self, layer: usize, seq: usize, n: usize) -> (&Buffer, u64) {\n        match &self.per_layer_grads {\n            Some(buf) => (buf, (layer * seq * n) as u64), // strided per-layer\n            None => (&self.d_h_ctx_buf, 0),               // shared accumulate\n        }\n    }\n}\nfn ctx_grad_strided(pass: &mut BackwardPass, layers: usize, seq: usize, n: usize) {\n    for layer in 0..layers {\n        let (dest, base) = pass.ctx_grad_dest(layer, seq, n); // runtime-\n        dispatch_ctx_grad(pass, layer, dest, base);  // switchable destination —\n    } // writes land at layer * seq * n + pos * n; downstream reads\n}     // CONTIGUOUS layer slices; no second pass, no de-interleave".to_string(),
        },
        // ── snapshot-transient-grad-for-deferred-download — lost temp → pinned copy (A) ──
        // Source: riir-gpu backward/dispatch.rs L885-897 ("Copy grad_output
        // to a persistent buffer — will be downloaded after the backward
        // pass encoder is submitted." + encoder.copy_buffer_to_buffer into
        // speft_grad_output_buffers[adapter_idx]) + backward/mod.rs L315-319
        // ("saved grad_output buffers for CPU-side gradient computation
        // (Plan 162). One buffer per adapter (n_layer * 6). Filled during
        // backward pass, downloaded after encoder submission."). A gradient
        // needed host-side often lives in a TEMP that later dispatches in
        // the same encoder overwrite. Pin it at the correct dataflow point:
        // an encoder-ordered copy into a persistent per-adapter buffer
        // lands exactly between the producing dispatch and the overwrite,
        // and the actual download happens after submit.
        //
        // Distinct from defer-readback-until-after-submit (Batch 26 — the
        // TIMING rule, read after submit): this is the LIFETIME half —
        // without the snapshot the value is destroyed before the deferred
        // read ever happens. Composes with persistent-zeros-source-encoder-
        // copy-clear (the same encoder-timeline copy, applied to value
        // pinning instead of clearing).
        KernelRule {
            rule_id: "snapshot-transient-grad-for-deferred-download".to_string(),
            gpu_concept: "a gradient needed host-side often lives in a temp buffer that later dispatches in the same encoder overwrite — snapshot it at the correct dataflow point with an encoder-ordered copy into a persistent per-adapter buffer (the copy lands exactly between the producing dispatch and the overwrite), then perform the actual host download after the encoder is submitted. The snapshot pins the value's lifetime; the deferral keeps the submission single. Distinct from defer-readback-until-after-submit (the TIMING rule — read after submit): this is the LIFETIME half, without which the deferred read finds the value already destroyed; composes with persistent-zeros-source-encoder-copy-clear (same timeline trick, value pinning vs clearing)".to_string(),
            ref_shape: "// naive: the transient is overwritten before any legal read can happen\nfn grad_capture_broken(encoder: &mut Encoder, temp_grad: &Buffer, n: usize) {\n    dispatch_adapter_grad(encoder, temp_grad);     // temp holds the value NOW\n    dispatch_next_layer(encoder, temp_grad);       // OVERWRITES temp!\n    // option A: download here — mid-encoder read returns pre-submission\n    let g = download_f32(device, queue, temp_grad, n);  // STALE + the internal\n    // submit breaks the encoder being built                        encoder\n    // option B: download after submit — the value is already destroyed\n}".to_string(),
            opt_pattern: "// optimized: encoder-ordered snapshot into a persistent buffer, download after\nstruct Pass { grad_output_buffers: Vec<Buffer> } // one persistent buffer / adapter\nfn grad_capture_snapshot(pass: &Pass, encoder: &mut Encoder, adapter: usize,\n                         temp_grad: &Buffer, n: usize) {\n    dispatch_adapter_grad(encoder, temp_grad);     // temp holds the value\n    // pin the value at THIS dataflow point — the copy lands between the\n    // producing dispatch and the overwrite:\n    encoder.copy_buffer_to_buffer(temp_grad, 0,\n        &pass.grad_output_buffers[adapter], 0, (n * 4) as u64);\n    dispatch_next_layer(encoder, temp_grad);       // overwrite is fine now —\n}                                                 // the snapshot persists\nfn download_after_submit(pass: &Pass, adapter: usize, n: usize) -> Vec<f32> {\n    // called AFTER queue.submit + poll — reads the pinned snapshot, never\n    // a transient, never mid-encoder\n    download_f32_reuse(device, queue, &pass.grad_output_buffers[adapter], n)\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 28 (Research 029): 10 rules — the deltanet CubeCL cluster
        // (deltanet_cubecl.rs decode-time recurrence + deltanet_chunked_
        // cubecl.rs chunkwise-parallel prefill). Grows B 32→33, C 18→19,
        // D 23→26, E 9→11, G 17→19, L 5→6.
        // ════════════════════════════════════════════════════════════════════
        // ── row-owned-register-blocked-recurrence — workgroup-per-head → per-(head,row) (G) ──
        // Source: riir-gpu deltanet_cubecl.rs L255-294 (the rowpar doc: the
        // row is loaded into registers once and every step operates on
        // registers; global passes over S: 4 read + 2 write → 1 read + 1
        // write; memory-bound, so cutting 6 passes to 2 is a ~3× reduction)
        // + L264-267 (n_head → n_head × head_dim cubes = 32× latency-hiding
        // headroom) + L338-342/L382-386 (single read/write passes). Rows of
        // the state matrix are independent within one decode step — narrow
        // whole-step ownership from head to row granularity; ownership is
        // what enables register blocking.
        //
        // Distinct from chunkwise-parallel-recurrence + recurrent-vs-chunk-
        // dispatch (sequence-axis parallelism — this is state-matrix
        // parallelism within one step) and from row-tile-amortize-input-load
        // (input-load amortization — the owned tile here is the persistent
        // state itself, read-modify-write across steps).
        KernelRule {
            rule_id: "row-owned-register-blocked-recurrence".to_string(),
            gpu_concept: "rows of a decode-time recurrent state matrix are independent within one step — decompose the per-head state ownership to (head, row) granularity so each workgroup OWNS a row: load it into registers once (each lane register-blocking d/plane strided columns), run every algebra step (decay, retrieve, delta, update, readout) on registers, and write it back exactly once. The memory-bound recurrence drops from 4 global read + 2 write passes over S per step to 1 read + 1 write (~3× cut of the gating quantity), threadgroup traffic and barriers drop to zero, and n_head cubes become n_head × head_dim cubes (32× more resident workgroups for latency hiding). Distinct from chunkwise-parallel-recurrence / recurrent-vs-chunk-dispatch (sequence-axis parallelism for prefill — this is state-matrix parallelism within one decode step) and from row-tile-amortize-input-load (input-load amortization in a GEMV — the owned tile here is the persistent state itself)".to_string(),
            ref_shape: "// naive: one workgroup owns the whole head state — every row's dot\n// product needs a barriered tree reduction and every algebra step\n// re-touches global memory (4 read + 2 write passes over S per step)\nfn recurrence_workgroup_per_head(state: &mut [f32], k_col: &[f32], smem: &mut [f32],\n                                 d: usize, row: usize, col: usize,\n                                 decay: f32) {\n    let idx = row * d + col;\n    state[idx] *= decay;                  // pass 1: global read + write\n    smem[col] = state[idx] * k_col[col];  // pass 2: partial to smem\n    barrier();\n    if col < 64 { smem[col] += smem[col + 64]; } // tree 128→64→32→16→8→4→2→1\n    barrier();\n    if col < 32 { smem[col] += smem[col + 32]; }\n    barrier();\n    let kv = smem[0]; // ~2304 barriers + 512 B threadgroup traffic per head;\n    let _ = kv;       // the update + readout steps re-read state from global\n}".to_string(),
            opt_pattern: "// optimized: one workgroup per (head, ROW) — the row is owned, loaded\n// into registers once (lane, lane+32, lane+64, lane+96), all five steps\n// run on registers between ONE global read and ONE global write\nfn recurrence_row_owned(state_row: &mut [f32; 128], k: &[f32; 128], q: &[f32; 128],\n                        v_row: f32, beta: f32, decay: f32, lane: usize,\n                        plane_reduce: fn(f32) -> f32) -> f32 {\n    let c0 = lane; let c1 = lane + 32; let c2 = lane + 64; let c3 = lane + 96;\n    let mut s0 = state_row[c0]; let mut s1 = state_row[c1]; // single global\n    let mut s2 = state_row[c2]; let mut s3 = state_row[c3]; // READ pass\n    s0 *= decay; s1 *= decay; s2 *= decay; s3 *= decay;      // step 1: decay\n    let kv = plane_reduce(s0 * k[c0] + s1 * k[c1] + s2 * k[c2] + s3 * k[c3]);\n    let delta = beta * (v_row - kv);                         // steps 2-3\n    s0 += k[c0] * delta; s1 += k[c1] * delta;                // step 4: update\n    s2 += k[c2] * delta; s3 += k[c3] * delta;\n    let out = plane_reduce(s0 * q[c0] + s1 * q[c1] + s2 * q[c2] + s3 * q[c3]);\n    state_row[c0] = s0; state_row[c1] = s1;                  // single global\n    state_row[c2] = s2; state_row[c3] = s3;                  // WRITE pass\n    out\n}".to_string(),
        },
        // ── workgroup-equals-plane-native-reduction — data-width workgroup → plane-sized (E) ──
        // Source: riir-gpu deltanet_cubecl.rs L256-262 (scattered threadgroup
        // access costs up to 3.2× effective bandwidth; the legacy kernel runs
        // a 7-level strided smem tree per row — 1792 scattered smem round-
        // trips per head; this kernel uses zero threadgroup memory: one cube
        // is exactly one plane, so plane_sum reduces the whole dot natively)
        // + L356-360 (the cube is exactly one plane, so plane_sum covers the
        // FULL dot product — the Issue 610 smem tree existed only because
        // the cube was 128 threads, i.e. 4 planes, and plane_sum caught just
        // 1/4 of it) + L362-363 (plane_sum broadcasts, so every lane has it —
        // no barrier, no thread-0 bottleneck, no smem hand-off) + L650-653.
        // The Issue 610 bug is the cautionary tale: plane_sum silently
        // covered 32 of 128 terms.
        //
        // Distinct from warp-shuffle-intrinsics-for-sub-warp-reduction (the
        // shuffle-instead-of-smem rule for blocks ≤ 32 — this is the LAUNCH
        // GEOMETRY corollary: size the workgroup so the condition holds by
        // construction, compensating logical width via register blocking)
        // and from single-plane-sufficient-reduction (the reduction-PLACEMENT
        // decision — this is the workgroup-SIZING decision that makes the
        // native path cover everything, plus the broadcast-for-free property).
        KernelRule {
            rule_id: "workgroup-equals-plane-native-reduction".to_string(),
            gpu_concept: "size the workgroup to EXACTLY one hardware plane (e.g. 32 threads on Metal/CUDA) and compensate the lost logical width via register blocking (each lane holds width/plane elements) — the native shuffle reduction then covers the FULL logical reduction, broadcasts the result to every lane for free, and no threadgroup memory or barrier exists at all. This inverts the usual choice of sizing the workgroup to the data width (128 threads), which forces every full reduction to bridge plane boundaries through a strided shared-memory tree (up to 3.2× effective-bandwidth cost + ~2304 barriers per head); a reduction scoped smaller than the workgroup is also a silent-wrong-answer hazard (Issue 610: plane_sum caught 1/4 of the dot). Distinct from warp-shuffle-intrinsics-for-sub-warp-reduction (use shuffles when the block is already ≤ 32 — this is the launch-geometry corollary that makes it hold by construction) and from single-plane-sufficient-reduction (where to place a reduction — this is how to size the workgroup so the native path covers everything)".to_string(),
            ref_shape: "// naive: workgroup sized to the data width (D=128 threads) — the\n// hardware shuffle reduction only covers 32 threads, so every full\n// reduction must bridge planes through shared memory + barriers\nfn dot_full_workgroup(x: &[f32; 128], y: &[f32; 128], smem: &mut [f32; 128],\n                      lane: usize) -> f32 {\n    smem[lane] = x[lane] * y[lane];\n    barrier();\n    // tree: 128 -> 64 -> 32 -> 16 -> 8 -> 4 -> 2 -> 1, one barrier per level\n    if lane < 64 { smem[lane] += smem[lane + 64]; }\n    barrier();\n    if lane < 32 { smem[lane] += smem[lane + 32]; }\n    barrier();\n    if lane < 16 { smem[lane] += smem[lane + 16]; }\n    barrier();\n    if lane < 8 { smem[lane] += smem[lane + 8]; }\n    barrier();\n    if lane < 4 { smem[lane] += smem[lane + 4]; }\n    barrier();\n    if lane < 2 { smem[lane] += smem[lane + 2]; }\n    barrier();\n    if lane < 1 { smem[0] += smem[1]; }\n    barrier();\n    smem[0] // visible to all only after the last barrier — and a shuffle\n}           // reduction called here would silently cover just 1/4".to_string(),
            opt_pattern: "// optimized: workgroup sized to EXACTLY one plane (32 threads), each\n// lane register-blocking 4 elements — the native shuffle reduction\n// covers the full logical reduction and broadcasts for free\nfn dot_one_plane(x: &[f32; 128], y: &[f32; 128], lane: usize,\n                 plane_reduce: fn(f32) -> f32) -> f32 {\n    // lane owns the strided elements lane, lane+32, lane+64, lane+96\n    let partial = x[lane] * y[lane]\n        + x[lane + 32] * y[lane + 32]\n        + x[lane + 64] * y[lane + 64]\n        + x[lane + 96] * y[lane + 96];\n    // hardware shuffle reduces across the 32 lanes AND broadcasts the\n    // result back to every lane — no smem, no barrier, no thread-0 read\n    plane_reduce(partial)\n}".to_string(),
        },
        // ── smem-broadcast-scalar-then-parallel-scatter — serial leader scatter → broadcast + all-thread scatter (E) ──
        // Source: riir-gpu deltanet_cubecl.rs L165-173 (the legacy recurrence
        // kernel, Issue 616 fix: the original port did the ENTIRE state update
        // S[row, :] += k[:] * delta_row serially in thread 0 — head_dim serial
        // global memory writes per row, with all other threads idle; this was
        // the single biggest bottleneck in both the cudarc and CubeCL
        // forwards) + L174-180 (fix: thread 0 computes delta, broadcasts via
        // smem[0], then ALL threads update their own column in parallel — 1
        // global write per thread instead of head_dim serial writes).
        //
        // Distinct from broadcast-uniform-skip-reduce (the READ/reduce side —
        // thread-uniform values are broadcast, not reduced; this is the WRITE
        // side: broadcast the reduced scalar so the dependent SCATTER
        // parallelizes) and from fan-out-shared-input-single-sync (N
        // dispatches sharing one upload — this is intra-workgroup broadcast).
        KernelRule {
            rule_id: "smem-broadcast-scalar-then-parallel-scatter".to_string(),
            gpu_concept: "after a workgroup-wide reduction, the single thread that holds the result must not also perform the entire dependent scatter — publishing the scalar through the already-reduced shared slot (one barrier) lets EVERY thread write its own column/element of the dependent update in parallel: 1 global write per thread instead of width serial writes from one thread with all others idle. The reduced slot doubles as the broadcast channel — the leader overwrites it after every lane has read the reduction result. Distinct from broadcast-uniform-skip-reduce (the read side — uniform values are broadcast, not reduced; this is the write side: broadcast the reduced scalar so the scatter parallelizes) and from fan-out-shared-input-single-sync (N dispatches sharing one host upload — this is intra-workgroup scalar fan-out)".to_string(),
            ref_shape: "// naive: after the reduction, the one thread holding the result also\n// performs the entire dependent update — width serial global writes\n// with every other thread idle (the single biggest bottleneck in the\n// original DeltaNet forward port)\nfn update_serial_in_leader(state: &mut [f32; 128], k: &[f32; 128],\n                           kv: f32, v_row: f32, beta: f32, lane: usize) {\n    if lane == 0 {\n        let delta = beta * (v_row - kv); // only thread 0 has/uses delta\n        for col in 0..128 {\n            state[col] += k[col] * delta; // 128 SERIAL global writes;\n        }                              // 127 threads sit idle\n    }\n}".to_string(),
            opt_pattern: "// optimized: the leader publishes the scalar through the already-\n// reduced slot; after ONE barrier every thread scatters its own\n// column — 1 global write per thread instead of width serial writes\nfn update_broadcast_then_scatter(state: &mut [f32; 128], k: &[f32; 128],\n                                 partial: f32, v_row: f32, beta: f32,\n                                 smem: &mut [f32], lane: usize,\n                                 plane_reduce: fn(f32) -> f32) {\n    let kv = plane_reduce(partial); // every lane receives the broadcast\n    if lane == 0 {\n        // leader computes the scalar ONCE and publishes it in the\n        // already-reduced slot (all lanes have consumed the reduction)\n        smem[0] = beta * (v_row - kv);\n    }\n    barrier(); // the single barrier of the whole update\n    let delta = smem[0]; // every thread reads the broadcast scalar\n    state[lane] += k[lane] * delta; // each thread writes its OWN column\n}".to_string(),
        },
        // ── token-flattened-elementwise-modulo-shared-weights — per-token dispatches → one flattened dispatch (D) ──
        // Source: riir-gpu deltanet_cubecl.rs L922-941 (the batched beta/decay
        // doc: the single-token kernel is pure per-head elementwise arithmetic
        // with no cross-head or cross-token coupling, so batching is just a
        // wider index space; prefill calls it once per token per DeltaNet
        // layer — 128 x 48 = 6144 dispatches at P=128 — and each dispatch
        // costs ~25 CPU allocations plus launch latency (Issue 638);
        // collapsing them to 48 is the point) + L932-935 (layout: per-token
        // buffers [p, n_head] row-major; a_log/dt_bias [n_head] per-head
        // weights shared across tokens, indexed i % n_head) + L939-941
        // (dispatch: 256-thread workgroups — unlike the single-token kernel,
        // which uses one workgroup of n_head threads and would exceed the
        // max workgroup size at p * n_head) + L991-993 (bit-identical to p
        // sequential calls: every output element is an independent function
        // of its own inputs).
        //
        // Distinct from batched-upcast-map (the pure wide-map instance — this
        // adds the MIXED-ARITY layout: flat per-token buffers + modulo-indexed
        // per-head weight tables in one kernel) and from merge-dispatches-
        // into-single-compute-pass (pass merging of dependent dispatches —
        // these were independent launches collapsed into one index space).
        KernelRule {
            rule_id: "token-flattened-elementwise-modulo-shared-weights".to_string(),
            gpu_concept: "a per-token elementwise launch inside a prefill loop (P tokens × per-layer = thousands of dispatches, each paying ~25 host-side allocations + launch latency) collapses to ONE dispatch when the op has no cross-token coupling: flatten [p, n_head] into a single 1-D index space, index the per-token buffers flat (i), and consume the per-head weight tables via i % n_head — the weights repeat every n_head entries while the token buffers do not. Bit-identical to the sequential form by construction (every output element is an independent function of its own inputs). Also fixes the workgroup ceiling: the single-token kernel's one-workgroup-of-n_head-threads shape exceeds the max workgroup size at scale; the flattened form uses fixed 256-thread workgroups. Distinct from batched-upcast-map (pure wide elementwise map — this adds the mixed-arity modulo-indexed shared-weights layout) and from merge-dispatches-into-single-compute-pass (dependent dispatches in one pass — these were independent launches collapsed into one index space)".to_string(),
            ref_shape: "// naive: one dispatch per token per layer — only the token changes,\n// but every launch pays ~25 CPU allocations + launch latency\n// (6144 dispatches at P=128, 48 heads; Issue 638)\nfn beta_decay_per_token(a_raw: &[f32], a_log: &[f32], dt_bias: &[f32],\n                        out: &mut [f32], p: usize, n_head: usize) {\n    for t in 0..p {\n        // one kernel launch per token:\n        for h in 0..n_head {\n            let a = a_raw[t * n_head + h] + dt_bias[h];\n            out[t * n_head + h] = a_log[h] * softplus(a);\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: tokens × heads flattened into ONE 1-D index space —\n// per-token buffers index flat, per-head weight tables index modulo\nfn beta_decay_flattened(a_raw: &[f32], a_log: &[f32], dt_bias: &[f32],\n                        out: &mut [f32], total: usize, n_head: usize) {\n    // one dispatch, fixed 256-thread workgroups (the per-token form's\n    // one-workgroup-of-n_head-threads shape exceeds the wg max at scale)\n    for i in 0..total {\n        // i decomposes as (token, head); head = i % n_head because the\n        // per-head weights REPEAT every n_head entries while the flat\n        // per-token buffers do not\n        let h = i % n_head;\n        let a = a_raw[i] + dt_bias[h];\n        out[i] = a_log[h] * softplus(a);\n    } // bit-identical: every output is an independent fn of its own inputs\n}".to_string(),
        },
        // ── fused-conv-state-shift-dot-silu — three passes + intermediates → one in-place kernel (C) ──
        // Source: riir-gpu deltanet_cubecl.rs L398-448 (the decode conv1d
        // kernel: shift conv state left by 1, append input; depthwise dot
        // out[ch] = Σ_k state[ch,k]·weight[ch,k]; SiLU out = x·sigmoid(x) —
        // all in one kernel, input modified in-place, conv_state sliding
        // window modified in-place, zero intermediate buffers, state
        // persists in GPU global memory across decode steps). One thread per
        // channel; the window shift is a ks-1-element in-place slide.
        //
        // Distinct from stateful-shift-via-cat-prefix (the host-side
        // allocating variant — this is the in-kernel in-place slide fused
        // with the dot + activation) and from fused-activation-quantize-
        // for-gemv-consumer (activation fused for a consumer — here the
        // activation is fused into the stateful conv PRODUCER, enabling
        // nonlinearity-forces-raw-input-buffers on the batched side).
        KernelRule {
            rule_id: "fused-conv-state-shift-dot-silu".to_string(),
            gpu_concept: "a decode-time stateful conv (sliding window + depthwise dot + activation) is ONE kernel per step, not three passes with intermediate buffers: each channel's thread shifts the sliding window IN PLACE (a ks-1 element slide, no new window allocation), computes the depthwise dot directly from the freshly shifted state, applies the activation, and writes the result back over the input buffer. Zero intermediate buffers, one dispatch, and the window state persists in global memory across decode steps — the host never touches it. Distinct from stateful-shift-via-cat-prefix (the host-side allocating window update — this is the in-kernel in-place slide fused with the dot + activation) and from fused-activation-quantize-for-gemv-consumer (activation fused at the consumer edge — here it is fused into the stateful producer; note the batched/prefill form CANNOT alias in-place, see nonlinearity-forces-raw-input-buffers)RIDER (Batch 275, the GROUPED-DYNAMIC-DRAFT-CONV face, Layr-Labs/mlxfast-bonsai2-9b-engine @ 8cee3ef (MIT), intro commit be83af0 -- the one-kernel conv law's GROUPED + DYNAMIC-CAUSAL variant on the drafter path: the DFlash2 fast drafter runs 'fused grouped dynamic convolution' -- per-group weights (the drafter's grouped conv) over a per-block dynamic causal window -- inside the DF2_FAST fused draft graph ('int8 activations (per row, per 32-group) x the pinned 8-bit g32 codes on the NAX' 'tensor units (16 x 32 x 32 per group; codes ^ 0x80 staged as int8, offsets + 128 * s);' 'fused grouped dynamic convolution; fused top-16 and greedy selector'), gated by the proposal-only license the batch's own verify-redecide rule records. Where this host owns the decode-slide conv+state+SiLU fold, the 9B face owns the BLOCK-DRAFT posture (the window is the draft block, not the decode slide). Batch 275 / Plan 279 / Research 313.".to_string(),
            ref_shape: "// naive: three passes with two intermediate buffers — shift the\n// window into a fresh buffer, convolve from it, activate into another\nfn conv1d_three_passes(conv_state: &[f32], input: &[f32], weight: &[f32],\n                       shifted: &mut [f32], out: &mut [f32], ks: usize) {\n    // pass 1: shift the sliding window (separate kernel + buffer)\n    for k in 0..ks - 1 { shifted[k] = conv_state[k + 1]; }\n    shifted[ks - 1] = input[0];\n    // pass 2: depthwise dot from the shifted window\n    let mut sum = 0.0f32;\n    for k in 0..ks { sum += shifted[k] * weight[k]; }\n    // pass 3: activation into the output buffer\n    out[0] = sum * sigmoid(sum);\n    // two intermediate buffers + three dispatches per channel per step\n}".to_string(),
            opt_pattern: "// optimized: ONE kernel shifts the window IN PLACE, convolves from\n// the freshly shifted state, applies SiLU, writes back over the\n// input — zero intermediates, state persists across decode steps\nfn conv1d_shift_dot_silu_inplace(conv_state: &mut [f32], input: &mut [f32],\n                                 weight: &[f32], ks: usize) {\n    // step 1: in-place slide (no new window allocation)\n    for k in 0..ks - 1 { conv_state[k] = conv_state[k + 1]; }\n    conv_state[ks - 1] = input[0];\n    // step 2: depthwise dot directly from the shifted window\n    let mut sum = 0.0f32;\n    for k in 0..ks { sum += conv_state[k] * weight[k]; }\n    // step 3: SiLU written back over the input in place\n    input[0] = sum * sigmoid(sum);\n}".to_string(),
        },
        // ── softplus-piecewise-saturation-clamp — direct ln(1+exp) → piecewise ±20 clamp (B) ──
        // Source: riir-gpu deltanet_cubecl.rs L863-874 (the beta/decay kernel:
        // g = a_log[h] · softplus(a_raw + dt_bias); softplus with clamping
        // (matches CPU: x > 20 → x, x < -20 → 0)) + the batched instance at
        // L970-980. The clamp thresholds are pinned to the CPU reference so
        // GPU and CPU stay bit-comparable — and the input feeds a decay
        // gate exp(g) applied to a PERSISTENT recurrent state, where an inf
        // from overflow poisons the state forever.
        //
        // Distinct from expm1-for-near-zero-exp-minus-one (stable exp(x)−1
        // NEAR ZERO — this is the FAR-tail saturation of ln(1+e^x)) and
        // exp2-log2-base2 (base conversion — this is domain clamping).
        KernelRule {
            rule_id: "softplus-piecewise-saturation-clamp".to_string(),
            gpu_concept: "softplus ln(1 + exp(x)) overflows (exp → inf → ln(inf)) for large x and wastes precision for very negative x — evaluate the piecewise form: return x for x > 20 (softplus equals x to f32 precision), 0 for x < -20 (the exponential underflows the sum), and the transcendental only inside the safe band. Pin the thresholds to the CPU reference implementation so GPU and CPU outputs stay bit-comparable under test; when the result feeds an exp() decay gate on a persistent recurrent state, the unclamped form does not just lose precision — one inf poisons the state permanently. Distinct from expm1-for-near-zero-exp-minus-one (cancellation near zero — this is far-tail saturation) and exp2-log2-base2 (base-2 conversion — this is domain clamping)".to_string(),
            ref_shape: "// naive: ln(1 + exp(x)) evaluated directly — exp overflows to inf\n// for x ≳ 88 (ln(inf) = inf poisons a PERSISTENT recurrent state\n// through the decay gate exp(g)), and wastes cycles for very negative x\nfn softplus_naive(x: f32) -> f32 {\n    (1.0f32 + x.exp()).ln()\n}".to_string(),
            opt_pattern: "// optimized: piecewise saturation — x for x > 20 (softplus == x to\n// f32 precision), 0 for x < -20, transcendental only in the safe\n// band; thresholds pinned to the CPU reference for bit-comparability\nfn softplus_clamped(x: f32) -> f32 {\n    if x > 20.0f32 {\n        x\n    } else if x < -20.0f32 {\n        0.0f32\n    } else {\n        (1.0f32 + x.exp()).ln()\n    }\n}".to_string(),
        },
        // ── reverse-running-decay-weight — materialized decay matrix → running scalar (G) ──
        // Source: riir-gpu deltanet_chunked_cubecl.rs L683-712 (the intra-chunk
        // output kernel: loop over j from token downto 0, accumulating
        // inter_decay incrementally; weight starts at 1 — inter_decay[t,t] =
        // empty product = 1; update weight for j-1: inter_decay[t,j-1] =
        // α[j] · inter_decay[t,j], i.e. multiply AFTER use) + the loop
        // contract at L640-643. The intra-chunk pairwise decay matrix
        // inter_decay[t,j] = Π_{i=j+1..t} α_i is never materialized: the
        // consumer loop carries ONE scalar, iterating j DOWNWARD so the
        // empty product anchors the recursion at j == t.
        //
        // The exact INVERSE of cumsum-gate-to-decay-matrix and pairwise-
        // distance-decay-matrix (both MATERIALIZE the matrix for a batched
        // matmul — this dematerializes it into the consumer's loop when the
        // consumer is itself a streaming accumulation). Distinct from
        // chunkwise-parallel-recurrence (the intra/inter chunk split — this
        // is the decay computation strategy INSIDE the intra part).
        KernelRule {
            rule_id: "reverse-running-decay-weight".to_string(),
            gpu_concept: "an intra-chunk pairwise decay matrix inter_decay[t,j] = Π_{i=j+1..t} α_i does not need to exist as a matrix when its consumer is a streaming accumulation: iterate j from t DOWNWARD carrying one running scalar — start at 1 (the empty product inter_decay[t,t]), consume the weight for position j, then multiply by α[j] to obtain the weight for j-1. O(C) multiplies per token instead of O(C) per (t,j) pair (or a cumsum+exp materialization pass), zero intermediate memory, and the downward direction is load-bearing — it anchors the empty product so multiply-after-use composes correctly. Distinct from cumsum-gate-to-decay-matrix / pairwise-distance-decay-matrix (the MATERIALIZE-the-matrix approach feeding a batched matmul — this is its dematerialized inverse when the consumer streams) and from chunkwise-parallel-recurrence (the chunk split itself — this is the decay strategy inside the intra-chunk part)".to_string(),
            ref_shape: "// naive: materialize the pairwise intra-chunk decay matrix — O(C)\n// multiplies per (t, j) pair (or a cumsum + exp pass), plus a second\n// pass to consume it\nfn intra_with_decay_matrix(q_t: &[f32], k: &[f32], v: &[f32], alpha: &[f32],\n                           beta: &[f32], d: usize, t: usize, c: usize,\n                           inter_decay: &mut [f32]) -> Vec<f32> {\n    // pass 1: build the [C, C] lower-triangular decay matrix\n    for tt in 0..c {\n        for j in 0..=tt {\n            let mut w = 1.0f32;\n            for i in (j + 1)..=tt { w *= alpha[i]; } // O(C) per pair\n            inter_decay[tt * c + j] = w;\n        }\n    }\n    // pass 2: consume it\n    let mut out = vec![0.0f32; d];\n    for j in 0..=t {\n        let scale = inter_decay[t * c + j] * beta[j];\n        let mut qv = 0.0f32;\n        for dd in 0..d { qv += q_t[dd] * v[j * d + dd]; }\n        for dd in 0..d { out[dd] += scale * qv * k[j * d + dd]; }\n    }\n    out\n}".to_string(),
            opt_pattern: "// optimized: NO matrix — the consumer loop runs j from t DOWN to 0\n// with a single running scalar; the empty product (= 1 at j == t)\n// anchors the recursion and each step multiplies α[j] AFTER use\nfn intra_with_running_weight(q_t: &[f32], k: &[f32], v: &[f32], alpha: &[f32],\n                             beta: &[f32], d: usize, t: usize) -> Vec<f32> {\n    let mut out = vec![0.0f32; d];\n    let mut weight = 1.0f32; // inter_decay[t, t] = empty product\n    for step in 0..=t {\n        let j = t - step; // iterate DOWNWARD — anchor at the empty product\n        let mut qv = 0.0f32;\n        for dd in 0..d { qv += q_t[dd] * v[j * d + dd]; }\n        let scale = weight * beta[j]; // weight IS inter_decay[t, j] here\n        for dd in 0..d { out[dd] += scale * qv * k[j * d + dd]; }\n        // decay the weight for the NEXT (j-1) iteration — after use:\n        // inter_decay[t, j-1] = alpha[j] * inter_decay[t, j]\n        weight *= alpha[j];\n    }\n    out\n}".to_string(),
        },
        // ── split-decay-outputs-eliminate-readback — packed layout → consumer-shaped outputs (D) ──
        // Source: riir-gpu deltanet_chunked_cubecl.rs L342-349 (the split
        // chunk-decay doc: writes per-token decay [n_head, C] and total
        // decay [n_head] to SEPARATE buffers — no readback needed for
        // production use; this avoids the [n_head, C+1] packed layout the
        // original produces, which requires a CPU readback + re-upload to
        // extract the two components; the split variant writes directly to
        // the layouts expected by DeltaS ([n_head, C]) and StateTransition
        // ([n_head])). The pipeline test L1949-1969 documents the exact
        // readback hack the split variant eliminates — and the wrong-offset
        // hazard ("the DeltaS kernel reads at head*C + j, which would read
        // total_decay of head h-1").
        //
        // Distinct from per-layer-strided-grad-destination (the backward
        // sibling — consumer-selected destination STRIDE within one
        // buffer; this is consumer-selected buffer SPLIT eliminating a host
        // round trip) and from defer-readback-until-after-submit / batched-
        // downloads-single-sync-cycle (both MITIGATE readback cost/timing —
        // this ELIMINATES the readback by producing consumer shapes).
        KernelRule {
            rule_id: "split-decay-outputs-eliminate-readback".to_string(),
            gpu_concept: "compact packing of heterogeneous outputs into one buffer (total_decay stashed at index C of a [H, C+1] row) is a pessimization when the downstream consumers read different shapes ([H, C] and [H]) — the packed layout forces a device→host→device readback-unpack-reupload, and a consumer reading the packed buffer directly reads the WRONG offset (total_decay of head h-1). Write each component directly into the buffer shape its consumer reads — one kernel, two output buffers, the host round trip disappears entirely. Packing elegance loses to consumer layout: optimize the boundary, not the buffer count. Distinct from per-layer-strided-grad-destination (consumer-selected stride within one buffer — this is consumer-selected buffer SPLIT) and from defer-readback-until-after-submit / batched-downloads-single-sync-cycle (both mitigate readback timing/cost — this eliminates the readback by producing consumer shapes at the source)".to_string(),
            ref_shape: "// naive: pack both outputs into ONE compact [H, C+1] buffer — but\n// the two consumers read [H, C] and [H] shapes, so the host must\n// read the packed buffer back, unpack it, and re-upload two buffers\nfn chunk_decay_packed(alpha: &[f32], packed: &mut [f32], h: usize, c: usize,\n                      per_token: &mut [f32], total: &mut [f32],\n                      host_copy: &mut Vec<f32>) {\n    for hh in 0..h {\n        for j in 0..c {\n            let mut w = 1.0f32;\n            for i in (j + 1)..c { w *= alpha[hh * c + i]; }\n            packed[hh * (c + 1) + j] = w;\n        }\n        let mut t = 1.0f32;\n        for i in 0..c { t *= alpha[hh * c + i]; }\n        packed[hh * (c + 1) + c] = t; // total stashed at index C\n    }\n    // consumers need [H, C] and [H]: device -> host -> device round trip\n    host_copy.clear();\n    host_copy.extend_from_slice(packed);\n    for hh in 0..h {\n        for j in 0..c { per_token[hh * c + j] = host_copy[hh * (c + 1) + j]; }\n        total[hh] = host_copy[hh * (c + 1) + c];\n    }\n}".to_string(),
            opt_pattern: "// optimized: write each component DIRECTLY into the buffer shape its\n// consumer reads — one kernel, two output buffers, no readback\nfn chunk_decay_split(alpha: &[f32], per_token: &mut [f32], total: &mut [f32],\n                     h: usize, c: usize) {\n    for hh in 0..h {\n        let mut t = 1.0f32;\n        for i in 0..c { t *= alpha[hh * c + i]; }\n        total[hh] = t; // [H] — StateTransition's shape\n        for j in 0..c { // [H, C] — DeltaS's shape\n            let mut w = 1.0f32;\n            for i in (j + 1)..c { w *= alpha[hh * c + i]; }\n            per_token[hh * c + j] = w;\n        }\n    } // compact packing was a pessimization: it broke both consumer\n}     // layouts and forced the host round trip".to_string(),
        },
        // ── carry-dual-layout-dead-slot — post-hoc conversion kernel → parameterized producer (L) ──
        // Source: riir-gpu deltanet_chunked_cubecl.rs L77-85 (carry_stride: the
        // per-channel stride in the carry buffer — kernel_size - 1 is the
        // original chunked-carry layout; kernel_size is the conv_state layout
        // (the full ks-wide sliding window; the chunked kernel reads/writes
        // positions 1..ks-1, leaving position 0 untouched — it holds a stale
        // value that the next decode's shift overwrites before use); set via
        // carry_idx_offset) + L98 (carry_off = ch · carry_stride +
        // carry_idx_offset). One kernel, two output layouts, two scalar
        // params — the prefill carry is written directly into the decode
        // path's native conv_state, no conversion pass between prefill and
        // decode.
        //
        // Distinct from per-layer-strided-grad-destination (consumer-selected
        // destinations via Option dispatch — this is a PARAMETERIZED producer
        // writing consumer-native layout with a provably-dead slot) and
        // alignment-gated-zero-copy-view (view gating — the dead-slot
        // tolerance by consumer semantics has no analogue there).
        KernelRule {
            rule_id: "carry-dual-layout-dead-slot".to_string(),
            gpu_concept: "when a producer's output feeds a consumer with a different but overlapping layout, parameterize the producer's write stride + index offset (two scalars) instead of adding a conversion pass: the prefill chunk-carry can be written compact [ch, ks-1] OR directly into the decode path's [ch, ks] sliding-window layout from the same kernel. The conv_state variant leaves slot 0 unwritten (stale garbage) — safely, because the decode consumer's shift-left provably overwrites slot 0 before any read. The dead-slot tolerance is a consumer-semantics proof, not a hope: the next decode's shift writes every slot it later reads. Distinct from per-layer-strided-grad-destination (runtime Option-selected destinations — this is one parameterized producer writing consumer-native layout) and from alignment-gated-zero-copy-view (gating a view — the dead-slot-tolerance-by-consumer-semantics element is new)".to_string(),
            ref_shape: "// naive: prefill finishes with a compact [ch, ks-1] carry; before\n// decode starts, a separate conversion kernel copies it into the\n// decode path's [ch, ks] sliding-window layout\nfn convert_carry_for_decode(compact: &[f32], conv_state: &mut [f32],\n                            conv_dim: usize, ks_m1: usize) {\n    // an extra dispatch + pass over the carry between prefill and decode\n    for ch in 0..conv_dim {\n        for j in 0..ks_m1 {\n            conv_state[ch * (ks_m1 + 1) + 1 + j] = compact[ch * ks_m1 + j];\n        }\n    }\n}".to_string(),
            opt_pattern: "// optimized: the prefill producer is parameterized (stride + offset)\n// to write the decode consumer's native layout directly; slot 0 is\n// left stale — provably safe because the decode shift overwrites\n// it before any read\nfn carry_written_in_consumer_layout(carry: &mut [f32], input: &[f32],\n                                    conv_dim: usize, c: usize, ks_m1: usize,\n                                    carry_stride: usize, carry_idx_offset: usize) {\n    let window_start = c.saturating_sub(ks_m1);\n    for token in window_start..c {\n        for ch in 0..conv_dim {\n            let slot = token - window_start;\n            // carry_stride=ks-1, offset=0 → compact layout;\n            // carry_stride=ks,  offset=1 → decode's conv_state (slot 0\n            // left stale — the decode shift overwrites it before use)\n            carry[ch * carry_stride + carry_idx_offset + slot] =\n                input[token * conv_dim + ch];\n        }\n    } // SAME kernel, two layouts — no conversion pass at the boundary\n}".to_string(),
        },
        // ── nonlinearity-forces-raw-input-buffers — in-place widening trap → raw RO + WO (D) ──
        // Source: riir-gpu deltanet_chunked_cubecl.rs L27-36 (the chunked conv1d
        // doc: the single-token kernel shifts conv_state, appends input,
        // convolves, applies SiLU — all in-place on the input buffer; the
        // chunked kernel processes C tokens in one dispatch; the KEY
        // CORRECTNESS constraint: token t's convolution needs the RAW
        // (pre-SiLU) inputs of tokens t-1..t-3 — if those were overwritten
        // in-place (as the single-token kernel does), the convolution would
        // read SiLU'd values, INCORRECT — so the chunked kernel uses
        // SEPARATE read-only input + write-only output buffers) + L43-47
        // (the carry holds RAW inputs, updated in-place for the next chunk).
        //
        // The negative space of inplace-output-alias (WHEN aliasing works —
        // this is when it CANNOT): a nonlinearity between producer and
        // consumer breaks the in-place trick under batching. Distinct from
        // chunkwise-parallel-recurrence (the chunking shape — this is the
        // buffer-contract that enables it when nonlinearity intervenes).
        KernelRule {
            rule_id: "nonlinearity-forces-raw-input-buffers".to_string(),
            gpu_concept: "a sequential kernel's in-place trick (write the activated output back over its own input) is INCOMPATIBLE with batched/widened execution whenever a nonlinearity sits between the producer and a windowed consumer: later tokens' convolution windows read earlier tokens' POST-activation values — silently wrong, same dtype, no error. The batched form needs separate read-only RAW inputs + write-only outputs, and the inter-chunk carry must also hold RAW values for the same reason. This is the negative space of in-place aliasing: before widening a sequential in-place kernel's grid over its sequence axis, check whether any consumer reads a value the nonlinearity would have overwritten. Distinct from inplace-output-alias (when aliasing WORKS — this is when it cannot) and from chunkwise-parallel-recurrence (the chunking shape — this is the buffer contract that makes chunking correct when nonlinearity intervenes)".to_string(),
            ref_shape: "// tempting-but-wrong: reuse the sequential kernel's in-place trick\n// and just widen the grid over tokens — token t+1's convolution\n// window then reads token t's POST-activation value (the\n// nonlinearity intervened between producer and windowed consumer)\nfn chunked_inplace_wrong(input: &mut [f32], weight: &[f32], conv_dim: usize,\n                         ks: usize, c: usize) {\n    for token in 0..c {\n        let mut sum = 0.0f32;\n        for k in 0..ks {\n            let pos = token as isize - ks as isize + 1 + k as isize;\n            // reads NEIGHBOR tokens' inputs — already overwritten by SiLU\n            sum += input[pos.max(0) as usize * conv_dim] * weight[k];\n        }\n        input[token * conv_dim] = sum * sigmoid(sum); // clobbers RAW value\n    } // silently wrong: same dtype, no error, post-activation history\n}".to_string(),
            opt_pattern: "// optimized (correctness-enabling): separate read-only raw inputs +\n// write-only outputs — the raw window stays pristine for ALL\n// consumers inside the chunk; the carry holds RAW values for the next\nfn chunked_raw_buffers(input: &[f32], output: &mut [f32], weight: &[f32],\n                       carry: &mut [f32], conv_dim: usize, ks: usize,\n                       c: usize) {\n    let ks_m1 = ks - 1;\n    for token in 0..c {\n        let mut sum = 0.0f32;\n        for k in 0..ks {\n            let pos = token as isize - ks_m1 as isize + k as isize;\n            let val = if pos < 0 {\n                // previous chunk's RAW tail\n                carry[(pos + ks_m1 as isize) as usize]\n            } else {\n                input[pos as usize * conv_dim] // RAW — never overwritten\n            };\n            sum += val * weight[k];\n        }\n        output[token * conv_dim] = sum * sigmoid(sum); // write-only out\n    }\n}".to_string(),
        },
        // ════════════════════════════════════════════════════════════════════
        // Batch 29 (Research 030): 10 rules — the decode-path fusion cluster
        // (deltanet_input_proj_fused.rs Issue 602 sibling-GEMV fan-out +
        // epilogue/ CODA epilogue fusion, Plan 106 Track 3). Grows
        // A 27→29, B 33→34, C 19→21, D 26→29, E 11→12, M 5→6.
        // ════════════════════════════════════════════════════════════════════
        // ── defer-readback-gated-by-live-output-size — unconditional deferral → footprint gate (A) ──
        // Source: riir-gpu deltanet_input_proj_fused.rs L20-32 (the Bench 437
        // contrast: "Bench 437 kept Q/K/V/gate/up outputs ALL alive
        // simultaneously (5 concurrent buffers each ~70 KB for the FFN case),
        // which increased Metal memory pressure → 16% slower." vs "These 4
        // input projections produce small outputs ... Total intermediate:
        // ~8 KB. Negligible memory pressure.") + L17-18 ("All 4 dispatches
        // are queued without `read_one`. Only the final outputs are read
        // back — ONE GPU sync point instead of 4.").
        //
        // Distinct from defer-readback-until-after-submit (the TIMING rule —
        // read after submit): this is the PRECONDITION that decides whether
        // deferral is profitable at all, with a measured counter-example.
        // Distinct from chain-dispatches-single-sync-point (the dispatch-side
        // mechanism): this is the output-sizing DECISION that makes the
        // chaining a win rather than a memory-pressure loss (Bench 437: 16%
        // regression when applied to large concurrent outputs).
        KernelRule {
            rule_id: "defer-readback-gated-by-live-output-size".to_string(),
            gpu_concept: "deferring all sibling readbacks to one final drain is NOT an unconditional win — gate it on total simultaneously-live output bytes. When the chained outputs are small (e.g. 4 input projections, ~8 KB total), keeping them all GPU-resident to the end is free and one sync point replaces N; when they are large (5 concurrent ~70 KB FFN buffers, ~350 KB live), the deferral itself increases memory pressure and measured 16% SLOWER on Metal — sync per stage and release each buffer instead. The rule converts 'defer readback' from an always-apply transformation into a footprint-gated one; the existing deferral rules carry the timing/mechanism, this carries the failure boundary. Distinct from defer-readback-until-after-submit (the timing rule — read after submit; this is the precondition that decides whether deferral helps at all) and from chain-dispatches-single-sync-point (the dispatch-side mechanism — this is the output-sizing decision that makes the chaining a win rather than a loss)".to_string(),
            ref_shape: "// naive: one shape for every workload — either sync per sibling\n// (N sync points) or, worse, the 'obvious optimization' applied\n// blind: keep ALL sibling outputs alive to the end even when they\n// are large\nfn fanout_always_defer(w: &[Weights], x: &[f32], outs: &mut [Buffer],\n                       n: usize) {\n    for i in 0..n {\n        dispatch_gemv_noread(&w[i], x, &mut outs[i]);\n    }\n    // ALL n outputs live simultaneously — fine at ~8 KB total, but\n    // the SAME shape on 5 x ~70 KB FFN buffers measured 16% slower:\n    // the deferral itself raised Metal memory pressure\n    for i in 0..n { read_one(&outs[i]); }\n}".to_string(),
            opt_pattern: "// optimized: gate the deferral on TOTAL live output bytes — chain the\n// independent dispatches + one drain only when the simultaneous\n// footprint is small; large footprints sync per stage and release\nfn fanout_size_gated(w: &[Weights], x: &[f32], outs: &mut [Buffer],\n                     n: usize, live_bytes: usize, limit: usize) {\n    if live_bytes <= limit {\n        // small outputs: all dispatches queued, one drain at the end —\n        // the first read pays the queue drain, the rest return\n        // immediately (already-completed buffers)\n        for i in 0..n { dispatch_gemv_noread(&w[i], x, &mut outs[i]); }\n        for i in 0..n { read_one(&outs[i]); }\n    } else {\n        // large outputs: per-stage sync + release keeps the peak\n        // footprint at ONE buffer instead of n\n        for i in 0..n {\n            dispatch_gemv_noread(&w[i], x, &mut outs[i]);\n            read_one(&outs[i]); // drain + release while small\n        }\n    }\n}".to_string(),
        },
        // ── weight-handle-cache-keyed-by-allocation-identity — per-call upload → pointer-keyed cache (A) ──
        // Source: riir-gpu deltanet_input_proj_fused.rs L279-281 ("a handle
        // cache keyed by the qkv weight's pointer identity (each layer has a
        // unique qkv allocation)") + L301-302 ("Cached input-projection
        // handles, keyed by (qkv_pos_ptr, qkv_neg_ptr)." — papaya::HashMap)
        // + L323-327 (key construction + insert) + L361-372 (hot-path lookup
        // + on-the-fly miss path: "On-the-fly upload (first call without
        // preupload).").
        //
        // Distinct from host-cache-immutable-device-weights (upload once at
        // init — this is the KEYING strategy layered on top: how a hook
        // receiving CPU refs resolves to those cached handles without
        // changing any call signature, plus the graceful miss path) and
        // from per-dispatch-pointer-re-extraction (device pointers re-read
        // per dispatch against pool aliasing — CPU heap pointers as a
        // persistent cache key are a different pointer, direction, and
        // failure mode).
        KernelRule {
            rule_id: "weight-handle-cache-keyed-by-allocation-identity".to_string(),
            gpu_concept: "when a GPU offload hook's public API must accept plain CPU weight references (it cannot require callers to thread device handles through existing model code), key a lock-free host-side cache of pre-uploaded device handles by the CPU weight allocation's POINTER IDENTITY — e.g. (pos_bits.as_ptr(), neg_bits.as_ptr()), unique per layer allocation. The hot path is one O(1) map lookup; a cache miss degrades gracefully to an on-the-fly upload + insert, so the hook works with or without a preupload warm-up and never blocks adoption. The keying is what lets the cache be invisible to the API surface. Distinct from host-cache-immutable-device-weights (the upload-once rule — this is the identity-keyed transparent lookup layered on top) and from per-dispatch-pointer-re-extraction (device pointers re-read per dispatch — CPU heap pointers as a persistent key, opposite direction). RIDER (Batch 181, mlxfast-bonsai2 @ cfb35b0d -- Qwen35.swift GDN derived-decay, the DERIVED-SCALAR instance): -exp(A_log) computed ONCE in a tiny kernel ('the same FP32 intermediate and Metal intrinsic as the prework expression... It depends on the model parameter only, never on request data') and cached on parameter identity ('if let cachedDecay, decaySource === aLog { return cachedDecay }') — pointer-keys a DERIVED value (not just an upload) and the owning projection invalidates on parameter update.".to_string(),
            ref_shape: "// naive: the offload hook's API receives plain CPU weight\n// references, so it re-uploads the full weight set on EVERY call —\n// a full-matrix host->device copy per decode step\nfn input_projections_naive(client: &Client, qkv: &Weights, z: &Weights,\n                           a: &Weights, b: &Weights, x: &[f32]) -> Out {\n    let qkv_h = upload_weights(client, qkv); // re-uploaded EVERY token\n    let z_h = upload_weights(client, z);\n    let a_h = upload_weights(client, a);\n    let b_h = upload_weights(client, b);\n    dispatch_fused(client, &[qkv_h, z_h, a_h, b_h], x)\n}".to_string(),
            opt_pattern: "// optimized: cache pre-uploaded device handles in a lock-free map\n// keyed by the CPU allocation's POINTER IDENTITY — the API surface\n// stays CPU-ref-shaped; hot path is one O(1) lookup, a miss uploads\n// on the fly (works with or without a preupload warm-up)\nstruct GpuHook { client: Client,\n                 handles: PapayaMap<(usize, usize), DeviceHandles> }\nimpl GpuHook {\n    fn input_projections(&self, qkv: &Weights, z: &Weights,\n                         a: &Weights, b: &Weights, x: &[f32]) -> Out {\n        // each layer owns a unique weight allocation — its address IS\n        // the layer identity; the (pos, neg) pointer pair disambiguates\n        let key = (qkv.pos.as_ptr() as usize, qkv.neg.as_ptr() as usize);\n        let pin = self.handles.pin(); // lock-free read view\n        let h = match pin.get(&key) {\n            Some(h) => h.clone(),       // hot path: O(1) lookup\n            None => {                   // cold path: upload + insert\n                let h = DeviceHandles::upload(&self.client, qkv, z, a, b);\n                pin.insert(key, h.clone());\n                h\n            }\n        };\n        dispatch_fused(&self.client, &h, x)\n    }\n}".to_string(),
        },
        // ── first-readback-pays-queue-drain — N-reads-N-syncs myth → burst-ordered reads (D) ──
        // Source: riir-gpu deltanet_input_proj_fused.rs L242-245 ("CubeCL's
        // `read_one` forces a sync; the 4 calls here all drain the same
        // command queue (the dispatches were already queued by
        // dispatch_input_projections_noread), so the cost is ~1 sync + 4
        // small readbacks rather than 4 independent syncs.").
        //
        // Distinct from batched-downloads-single-sync-cycle (RESTRUCTURES N
        // readbacks into one encoder/submit/map/poll cycle — this achieves
        // ~1 sync with plain sequential read_one calls purely by queue
        // ordering, and explains when the heavier batching machinery is
        // unnecessary) and from chain-dispatches-single-sync-point (the
        // dispatch-side half — this is the read-side completion of the same
        // cost model: what the single sync actually costs and why the
        // remaining reads are free).
        KernelRule {
            rule_id: "first-readback-pays-queue-drain".to_string(),
            gpu_concept: "after a single dispatch burst on one command queue, the FIRST blocking read drains the whole queue and every subsequent read of an already-completed buffer returns immediately — N sequential readbacks ordered after one burst cost ~1 sync + N small transfers, with no restructuring into a merged download op needed. Sync count is a function of dispatch BURSTS, not read CALLS: don't add per-read defensive syncs, and don't build batched-download machinery when the reads naturally follow one burst. Distinct from batched-downloads-single-sync-cycle (restructures N readbacks into one submit/map/poll cycle — this is the cheaper ordering-only alternative that makes that machinery unnecessary) and from chain-dispatches-single-sync-point (the dispatch-side mechanism — this is the read-side cost model)".to_string(),
            ref_shape: "// naive cost model: 'N read calls = N GPU syncs' — so the code\n// either interleaves a defensive sync before EVERY read, or grows\n// merged-download machinery to amortize the (imagined) per-call sync\nfn drain_defensive(client: &Client, bufs: &[Buffer]) -> Vec<Vec<f32>> {\n    let mut out = Vec::new();\n    for b in bufs {\n        client.sync(); // 'to be safe' — but the queue was already\n        out.push(client.read_one(b.clone()).unwrap()); // drained by the\n    }                                                // first read\n    out\n}".to_string(),
            opt_pattern: "// optimized: after ONE dispatch burst on one command queue, the\n// first read blocks until the whole queue drains; every later read\n// of an already-completed buffer returns immediately — plain\n// sequential reads ordered after the burst cost ~1 sync total\nfn drain_burst_ordered(client: &Client, bufs: &[Buffer]) -> Vec<Vec<f32>> {\n    // dispatches were queued back-to-back earlier (one burst, one\n    // queue) — no interleaved syncs anywhere\n    let mut out = Vec::new();\n    for b in bufs {\n        // read #1 pays the queue drain; reads #2..N hit completed\n        // buffers and return immediately\n        out.push(client.read_one(b.clone()).unwrap());\n    }\n    out // sync count is a function of dispatch BURSTS, not read calls\n}".to_string(),
        },
        // ── epilogue-at-register-write-phase — separate elementwise pass → fuse at the store (C) ──
        // Source: riir-gpu epilogue/mod.rs L8-11 ("CODA (Research 67)
        // reparameterizes Transformer computation as GEMM-plus-epilogue
        // programs. The key insight: memory-bound operations (normalization,
        // residual adds, activations) can execute *while the GEMV output is
        // still in registers*, avoiding global memory round-trips.") + L13-16
        // + L21-26 (the per-row architecture) + gemv_residual_cubecl.rs
        // L103-109 ("let result = plane_sum(partial); if lane == 0u32 {
        // output[row as usize] = result + residual[row as usize]; }") +
        // L24-31 (2→1 dispatches per site, 52 over 26 layers; 104 combined
        // with the norm fusion).
        //
        // Distinct from fuse-residual-via-in-place-output-array (buffer
        // ALIASING to bypass a parameter-count limit — this executes the op
        // while the value is still in registers at the producer's write
        // phase) and from merge-dispatches-into-single-compute-pass
        // (host-side pass merging with zero kernel changes — this edits the
        // kernel body).
        KernelRule {
            rule_id: "epilogue-at-register-write-phase".to_string(),
            gpu_concept: "memory-bound elementwise operations (residual adds, scales, activations) should execute while the producer's output is STILL IN REGISTERS, at the existing write phase — not as a separate dispatch that reads the output vector back from global memory. In a plane-cooperative GEMV (one plane per output row), plane_sum leaves the full row dot product in every lane's registers, so the epilogue (read residual[row], add, write) rides lane 0's single global store: one extra scalar global read amortized over the n-length cooperative dot, one dispatch eliminated per use site (2→1; 52 per 26-layer model, 104 combined with the norm+residual fusion). The condition is the register-residency of the produced value at the write phase. Distinct from fuse-residual-via-in-place-output-array (buffer aliasing to bypass a parameter limit — this is register-residency at the producer's write phase) and from merge-dispatches-into-single-compute-pass (host-side pass merging, zero kernel changes — this edits the kernel body)".to_string(),
            ref_shape: "// naive: GEMV kernel writes its output vector to global memory;\n// a SECOND elementwise kernel reads it back, adds the residual,\n// writes again — one extra dispatch + a full global round-trip of\n// the vector per use site\nfn gemv_then_residual(w: &[f32], x: &[f32], residual: &[f32],\n                      out: &mut [f32], m: usize, n: usize) {\n    dispatch_gemv(w, x, out, m, n);            // pass 1: dots -> global\n    dispatch_residual_add(out, residual, m);   // pass 2: read out +\n}                                              // residual, write out".to_string(),
            opt_pattern: "// optimized: plane_sum leaves the FULL row dot in every lane's\n// registers — the residual add executes at lane 0's existing single\n// global store; the unfused output vector never touches memory\nfn gemv_residual_epilogue(w: &[f32], x: &[f32], residual: &[f32],\n                          out: &mut [f32], m: usize, n: usize,\n                          row: usize, lane: usize,\n                          plane_sum: fn(f32) -> f32) {\n    let mut partial = 0.0f32;\n    let mut k = lane;              // cooperative strided dot product\n    while k < n {\n        partial += w[row * n + k] * x[k];\n        k += 32;\n    }\n    let dot = plane_sum(partial);  // every lane holds the row result\n    if lane == 0 {\n        // epilogue AT the write phase: one extra global read of a\n        // single scalar, amortized over the n-length cooperative dot\n        out[row] = dot + residual[row];\n    }\n}".to_string(),
        },
        // ── delay-rms-scale-past-gemv-epilogue — norm-before-GEMV → commute r past the GEMV (C) ──
        // Source: riir-gpu epilogue/mod.rs L227-235 ("The key identity (CODA
        // §3.2.1): RMSNorm(x @ W + z) * γ @ W' = r * ((x @ W + z) * γ) @ W'
        // Where `r` is a per-row scalar that commutes with the matrix
        // multiply. This lets us compute `r` once and apply it *after* the
        // next GEMV, eliminating a separate RMSNorm dispatch.") + L118-124
        // (RowScale: "the RMS scale factor `r` is applied *after* the GEMV
        // instead of *before* (as a separate RMSNorm kernel). The algebraic
        // identity: `RMSNorm(x) @ W = r * (x * gamma) @ W`.") + L251-262
        // (the honest accounting: the pure reparameterization saves ZERO
        // dispatches 3→3 — the win is fusing r into the last GEMV of the
        // group + eliminating the pre-GEMV rmsnorm write; with
        // handle-to-handle passing the dominant benefit is dispatch /
        // command-buffer overhead, not bandwidth) + coda_primitives_
        // cubecl.rs L20-24 (the decomposition: PartialRMS extracts r,
        // NormWeightScale applies gamma, RowScale applies r later).
        //
        // Distinct from fold-norm-scalar-into-matmul (folds the scalar into
        // the matmul internals — this keeps the GEMV body untouched, lands
        // r in its epilogue, and includes the producer-side decomposition
        // so no separate scale pass exists anywhere) and from
        // fuse-add-rms-norm (fuses residual INTO a norm kernel — this moves
        // the norm's scalar ACROSS a GEMV boundary).
        KernelRule {
            rule_id: "delay-rms-scale-past-gemv-epilogue".to_string(),
            gpu_concept: "RMSNorm's per-row scalar commutes with the downstream matmul — RMSNorm(x) @ W = r · (x·γ) @ W — so split the norm into (a) extracting r (a partial-norm reduction), (b) applying only the elementwise gamma, and (c) applying r later as a row-scale epilogue of the downstream GEMV, composable with a residual add. Be honest about what this saves: the pure reparameterization alone saves ZERO dispatches (3→3) — the real win is fusing r into the LAST GEMV of the group and eliminating the pre-GEMV write of the normed vector; with handle-to-handle passing the dominant benefit is dispatch/command-buffer overhead, not bandwidth. The win compounds when multiple GEMVs share the same r. Distinct from fold-norm-scalar-into-matmul (folds the scalar into the matmul internals — this keeps the GEMV body untouched and adds the producer-side decomposition) and from fuse-add-rms-norm (fuses residual into a norm kernel — this moves the norm scalar across a GEMV boundary). RIDER (Batch 223 / Plan 222, lithos-ai/lithos-metal @ 236475fa, Apache-2.0 -- the PREFOLD half of the commuted norm, the statistic side this rule's r-extraction only implies): the commuted norm ships in two halves on the lithos compiler -- passes/fuse_norm.py:2-6 verbatim: 'that GEMV can emit, per block, the partial sum of squares of the BF16 values it writes (STAT_OUT), and the consumer sums the n_blocks partials in block order -- the same statistic, deterministic, one dispatch fewer per norm.' -- the producer GEMV emits per-block PARTIAL sums-of-squares (stat_out=True, stat_value named) and the consumer sums the n_blocks partials IN BLOCK ORDER; a producer feeds ONE statistic ('a producer feeds one statistic' guard, fuse_norm.py:27 -- no double-claim), and the statistic of a norm fed by anything else (the embedding) keeps its own dispatch. The reciprocal-RMS fold itself then runs ONCE in a separate one-SIMD-group kernel so every downstream projection applies the SAME scales after their FP32 product -- mlp_fusion.py:14-15 verbatim: 'One SIMD group computes the eight reciprocal RMS values. Projections still apply them after their FP32 product, preserving the commuted-norm contract.' -- one fold, not per-consumer refolds. The honesty pair rides the rider verbatim (mixers.md:101-104): 'Moving BF16 rounding changes the numerical path.' (default-ON with an opt-out: '--no-commute-norm in the generation CLI or Session(..., commute_norm=False)'); 'Tests must distinguish numerical agreement with the reordered formula from agreement with the original rounding boundaries.'; and 'Removing a dispatch is not sufficient evidence of a latency improvement.' -- the same honesty law this rule's own zero-dispatch-savings disclosure carries, now seconded by an independent engine. Batch 223 rider / Plan 222.".to_string(),
            ref_shape: "// naive: RMSNorm runs as its own kernel BEFORE the GEMV —\n// normalize, WRITE the normed vector to global, GEMV immediately\n// reads it back\nfn norm_then_gemv(hidden: &[f32], gamma: &[f32], w: &[f32],\n                  eps: f32) -> Vec<f32> {\n    let normed = dispatch_rmsnorm(hidden, gamma, eps); // kernel 1:\n    dispatch_gemv(w, &normed)        // writes the full normed vector\n}                                    // kernel 2: reads it all back".to_string(),
            opt_pattern: "// optimized: the per-row scalar r = 1/sqrt(mean(x^2)+eps) COMMUTES\n// with the matmul — extract r en passant, apply only gamma\n// elementwise, and land r in the downstream GEMV's row-scale\n// epilogue (composable with residual: r*dot + residual)\nfn delayed_norm(hidden: &[f32], gamma: &[f32], w: &[f32],\n                eps: f32) -> Vec<f32> {\n    // partial norm: ONE scalar out, not a normed vector\n    let r = partial_rms_scalar(hidden, eps);\n    let d = apply_gamma(hidden, gamma);   // elementwise only\n    // r crosses the kernel boundary as ONE scalar and multiplies the\n    // dot INSIDE the next GEMV's write phase — the normed vector the\n    // GEMV would immediately read back is never written; fuse r into\n    // the LAST GEMV of the group (gate+up share the same r)\n    gemv_rowscale_epilogue(w, &d, r)\n}".to_string(),
        },
    ]
}

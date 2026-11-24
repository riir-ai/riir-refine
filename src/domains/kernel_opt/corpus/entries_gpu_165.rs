//! Batch 409 — the Prism fork ternary-quant family residue (Plan 408 /
//! Research 442): PrismML-Eng/llama.cpp (the league fork, MIT) — `PQ2_0M`
//! min/max KV-cache type #428 @ `d12ca13bb` (2026-11-21, `origin/prism`
//! tip) + the Arm NEON `PQ2_0` repack #449 @ `802b53737` (2026-11-20).
//!
//! THE PRIOR-COVERAGE FINDING is the batch's controlling result: the
//! staged lead's `PTQ1_0` decode surface is ALREADY OWNED —
//! `swar-lane-widened-base3-dp4a-dot` (Batch 116 addendum @ 7113f6462,
//! the same fork lineage) owns the byte-lane-widen/×3/0x00FF00FF/0x7531/
//! vsub4/dp4a mechanism and carries the B180/B204/B211/B251 riders
//! (the isum transport, the three-arm ISA decision, the i8mm halving-add
//! validation with a Ternary Bonsai 2 27B device A/B, the yukon
//! direct-ladder face). The 11-24 watch row's "their CUDA `PTQ1_0` mat-vec
//! kernel shapes" value is CONSUMED (a41dcc7ac, Sept 29, predates the
//! riders and lands inside owned territory) — dropped at dedup, never
//! re-staged. What survived: `PQ2_0M` and the NEON repack, both
//! post-riders and both zero-owner at claim-time greps.
//!
//! - `min-max-range-scale-asymmetric-block-codec` — the codec lead (N):
//!   blocks asymmetric around zero spend a second fp16 on a min/max
//!   range codec instead of forcing a zero-anchored code set; measured
//!   KLD 0.05/0.06 vs the symmetric floor 0.258 (~4×); KV-only
//!   split-surface.
//! - `offset-block-dot-block-sum-dp4a-correction` — the offset dot (B):
//!   the raw-code block dot cancels the per-block offset with a SECOND
//!   dp4a against 0x01010101, the offset riding as a scalar multiply —
//!   no isum transport needed.
//!
//! Faces (count-neutral, same commit): `deferred-bias-ternary-dp4a-
//! exact-isum` (`gpu_106`) += the NEON unsigned-codes block-sum face (the
//! CPU vaddlvq form of the Σ-cancel, from #449); `ane-int8-matmul-shape-
//! silent-wrong-gate` (ane) += the reference-census-catches-dequant
//! face (from #428's sub-commits); `fewrow-mma-fragment-matvec-silicon-
//! tiered-dispatch` (`gpu_53`) += the decode-once-multi-column mmvq face
//! (from the `PTQ1_0` stack); the `dup_half` row-interleave face folded
//! into the NEON face text. Lane intel recorded, never landed: the
//! GGUF type-ID 144/145 branch conflict; T-R3 adjacency (#428 on
//! `origin/prism` tip — the next scored tag carries `pq2_0m` KV).
//!
//! Fixtures: `corpus/fixtures/b409_fixtures.rs` — standalone-compiled
//! AND executed BEFORE entry (debug AND -O), the four-arm discipline.
//! Dedup re-verified at the live corpus at authoring time (Protocol A).
//! NOTHING INVENTED — every quote read at the vendor clone this session
//! (pins `d12ca13bb` + `802b53737`).
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 409 — the Prism fork ternary-quant family residue's `kernel_opt` half.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the codec lead: the symmetric codec's error is dominated
        // by the block's offset from zero, and the second fp16 buys the
        // whole range back. ──
        KernelRule {
            rule_id: "min-max-range-scale-asymmetric-block-codec".to_string(),
            gpu_concept: "MECHANISM (quant codec choice; category N — dtype/alignment): a block-coded quantizer whose code set is anchored at ZERO (symmetric: the codes represent {-x, 0, +x} steps around 0) pays for the BLOCK's own offset from zero as quantization error — when the block's values are systematically asymmetric around zero (KV-cache rows, whose magnitudes and signs drift away from the symmetric assumption), spend a SECOND scale word on a min/max RANGE codec (d = (max−min)/3, m = min, raw codes {0,1,2,3}, dequant y = m + q·d) so all levels span the full data range. The source's measured basis (Vulkan/RX 9070, their numbers — provenance for the ratio, never adoption bars): GPU KLD 0.05 (mixed K=q4_0+bias / V=pq2_0m) and 0.06 (both KV pq2_0m) versus the zero-anchored sibling codec's no-bias floor 0.258 — ~4× better at the SAME 2-bit code width, for +2 bytes per 128-value block (one fp16 m; the block goes 22→24 B). Verbatim (ggml-common.h): 'd = (max - min) / 3 and a per-block offset m = min, so all four levels span the full data range [min, min+d, min+2d, min+3d=max]. Dequant: y = m + q * d. The +16 B over PQ2_0 (one extra fp16 for m) is the price of covering [min, max] with 4 levels.' (their committed text says +16 B meaning +16 bits — 2 bytes; the quantizer clamps q = clamp(round((w−min)·id), 0, 3), worst-case error d/2 = (max−min)/6.) THE SPLIT-SURFACE CLAUSE is the adoption shape: the range codec is a KV-ONLY type — 'KV-cache only. Model weights keep existing PQ2_0 (matmul) unchanged.' — the weight matmul keeps the zero-anchored codec (weights ARE roughly symmetric per group; the +B/block and the m-word buy nothing there), the KV rows (asymmetric per block, per-position) take the range codec. The surfaces are SEPARATE wire types, never a reinterpret: 'adds a separate format; it does not replace or reinterpret existing files.' ⛔ THE LOSSY-SURFACE LAW governs adoption: this is a lossy codec — per-family retention gating on the real consumer stream (their KLD census IS that gate: the KV KLD read on the serving posture, 0.05/0.06 — never an aggregate-perplexity pass; the sibling codec's 0.258 floor the named comparator). THE CODEC-DEFECT TAIL this batch's sub-commits carry (folded here as the adoption-gate evidence, the ane silent-wrong face is the host): the first GPU V-dequant read only 2 of 4 two-bit codes per byte and broadcast each — '[c0,c0,c1,c1] instead of [c0,c1,c2,c3]' — inflating KV-cache KLD to 1.6 versus the Vulkan reference 0.05; the K shmem-staging path indexed the packed scale in the wrong field (a_offset + buf_ib/4 instead of a_offset + global_ib); the CPU flash-attention vec_dot table lacked the type row entirely (null dereference at threadpool init). NONE of the three was caught by a build — the KLD census caught the first, review the others: a lossy-codec landing carries its reference-census gate from the first commit, not after. ⛔ KINSHIP (adjudicated both directions): sign-codec-shared-magnitude-mean-abs owns the SYMMETRIC-side scale choice (mean-abs magnitude sharing across a sign codec) — the OPPOSITE pole of the asymmetry decision, compose (per-block: symmetric-enough blocks take the sign/zero-anchored form, asymmetric blocks take the range form — the decision IS this rule's predicate); base-3-trit-packing-5-per-byte (gpu_01) owns the trit PACKING format (density axis) — THIS owns the SCALE/OFFSET axis of a fixed-width code, orthogonal (a range codec can carry trit codes and vice versa — PQ2_0 vs PQ2_0M share the exact 2-bit code layout, only the scale words differ); bit-plane-plane-sums-input-side-offset-dot (gpu_?) owns the DOT-side algebra for an offset codec (never materialize, fold the offset into the dot — see offset-block-dot-block-sum-dp4a-correction, this batch's B-side, which consumes THIS codec's blocks); the deferred-bias family (B180 gpu_106) owns the DOT-side cancel for a zero-anchored (q−1) codec — the offset-dot sibling of THIS codec-side choice. REGIME: block-codes at 2-4 bits where the block population is asymmetric around zero (KV/activation rows with drifting means); any fixed-width code whose zero-anchored reconstruction wastes its outer levels on empty sign ranges. ANTI-REGIME: symmetric blocks (the +m word and the range step buy nothing — their own sibling stays); blocks whose max−min exceeds the value range the levels can span at useful resolution (d too coarse — measure, never assume); an exact-reconstruction consumer (a lossless surface never takes a range codec). GOAT: the fixture pins (a) the round-trip over a synthetic asymmetric block (max|err| ≤ d/6), (b) the symmetric-block contrast (the range codec on a symmetric block LOSES to the zero-anchored one — the predicate discriminates), (c) the KLD-shaped retention read (a synthetic stream's reconstruction error, per-family), (d) the tamper arm (a dequant missing 2 of 4 codes reads measurably wrong — the [c0,c0,c1,c1] class reds). NotBenchable-by-model (the win metric is a KLD/retention read on a real KV stream — no validator hosts it; the d/6 bound is CPU-testable and pinned). P113: the fork's own landed decision (origin/prism tip) — decision content only, never an outbound PR; the fork is the league opponent build. League intel recorded, never landed here: PQ2_0M sits on `origin/prism` tip — the next scored tag carries pq2_0m KV; the pq1_0-g64 branch and prism CONFLICT on GGUF type id 144 (and #428's own enum says 144 while its glsl says 145u — an intra-commit inconsistency), so a merge of both branches must re-allocate one. Source: PrismML-Eng/llama.cpp @ d12ca13bb (#428, MIT, 2026-11-21) — ggml/src/ggml-common.h (the block_pq2_0m struct + comment verbatim), ggml/src/ggml-quants.c (quantize_row_pq2_0m_ref + dequantize_row_pq2_0m), ggml/src/ggml-cuda/fattn-common.cuh (vec_dot_fattn_vec_KQ_pq2_0m + dequantize_V_pq2_0m + the MUSA twin), the commit body's KLD validation block (provenance), ggml/src/ggml-vulkan/vulkan-shaders/dequant_pq2_0m.comp (the reference dequant the CUDA arm must match). Quotes byte-verified at the pin this session. MIT. Batch 409 / Plan 408 / Research 442.".to_string(),
            ref_shape: r"// NAIVE: the block codec anchors its code set at zero — the
// codes represent {-s, 0, +s} steps, and a block whose values sit
// far from zero wastes its outer levels on sign ranges nothing
// occupies (the measured cost: KLD 0.258 on KV rows vs 0.05).
struct SymBlock {
    d: f32,         // one scale, zero-anchored
    qs: [u8; 32],   // 2-bit codes: q in {0..3}, dequant (q - 1.5) * d
}
fn quant_sym(x: &[f32; 128]) -> SymBlock {
    let d = x.iter().map(|v| v.abs()).fold(0.0_f32, f32::max) / 1.5;
    let mut qs = [0u8; 32];
    for (i, &v) in x.iter().enumerate() {
        let q = ((v / d).round() + 1.5).clamp(0.0, 3.0) as u8; // (q-1.5) code
        // the block's offset from zero rides as ERROR:
        // a block spanning [10.2, 11.4] reconstructs into
        // {0, d, 2d, 3d} — none of it near the data
        qs_set(&mut qs, i, q);
    }
    SymBlock { d, qs }
}
fn dequant_sym(b: &SymBlock, i: usize) -> f32 {
    (code_of(&b.qs, i) as f32 - 1.5) * b.d // zero-anchored
}".to_string(),
            opt_pattern: r"// FIXED: the range codec — a second fp16 buys the block's own
// [min, max] span; all four levels land inside the data range.
// (Their layout: d + m + 32 code bytes per 128 values = 24 B.)
#[derive(Clone, Copy)]
struct RangeBlock {
    d: f32,     // (max - min) / 3 — the range step
    m: f32,     // min — the per-block offset
    qs: [u8; 32], // raw 2-bit codes q in {0..3}
}
fn quant_range(x: &[f32; 128]) -> RangeBlock {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for &v in x {
        min = min.min(v);
        max = max.max(v);
    }
    let d = (max - min) / 3.0;
    let id = if d > 0.0 { 1.0 / d } else { 0.0 };
    let mut b = RangeBlock { d, m: min, qs: [0u8; 32] };
    for (i, &v) in x.iter().enumerate() {
        let q = ((v - min) * id).round().clamp(0.0, 3.0) as u8;
        qs_set(&mut b.qs, i, q); // 2 bits per element
    }
    b
}
fn dequant_range(b: &RangeBlock, i: usize) -> f32 {
    b.m + code_of(&b.qs, i) as f32 * b.d // y = m + q*d — all levels in [min, max]
}
// worst-case error d/2 = (max-min)/6; the same 2-bit code width,
// +2 bytes per block. KV-only: weights keep the zero-anchored codec
// (separate wire types, never a reinterpret).".to_string(),
        },
        // ── R2 — the offset dot: a codec that carries a per-block offset
        // m needs NO isum transport — the offset cancels as a scalar
        // multiply over a second dp4a. ──
        KernelRule {
            rule_id: "offset-block-dot-block-sum-dp4a-correction".to_string(),
            gpu_concept: "MECHANISM (offset-codec dot; category B — compute): a dot against a block codec whose reconstruction carries a PER-BLOCK OFFSET (y = m + q·d, the min/max range class) never materializes the dequant — decompose sum((m + q·d)·u) = m·Σu + d·Σ(q·u) and pay BOTH integer sums with dp4a: the code byte packed as a word against the activation word gives Σ(q·u), and the SAME 0x01010101 constant word against the activation word gives Σu (a dp4a against ones IS a sum) — the offset then rides as one scalar multiply at block granularity. The source's KQ dot, verbatim (fattn-common.cuh): 'Dequantize each K code (m + c * d) and dot with the corresponding Q q8_1 element. sum = d_Q * (m * sum(c0..c3) + d * sum(cK * qQ))' with the code word assembled as 'const int c_byte = (c0 << 0) | (c1 << 8) | (c2 << 16) | (c3 << 24);' and 'const int sum_c_u = ggml_cuda_dp4a(c_byte, u, 0); const int sum_u = ggml_cuda_dp4a(0x01010101, u, 0);' — TWO dp4a per byte-word, no dequant values in registers. THE TRANSPORT-FREE CLAIM is the delta over the deferred-bias family: B180's additive (q−1) cancel needs the activation's exact integer sum TRANSPORTED in the block (the half-slot isum, bit-exact read-back) because the zero-anchored codec has nowhere to carry Σu; a codec with its OWN per-block m-word needs no transport — m rides the block, and the dp4a-against-ones extracts Σu at the consumer. The codec-side choice (min-max-range-scale-asymmetric-block-codec, this batch's N-side) and the dot-side decomposition COMPOSE: the range codec's blocks dot exactly through this form. ⛔ KINSHIP (adjudicated both directions): deferred-bias-ternary-dp4a-exact-isum (B180, gpu_106) owns the ADDITIVE bias cancel for a (q−1) zero-anchored codec WITH the isum transport — compose, never merge: same dp4a-machinery family, different bias ALGEBRA (theirs subtracts a transported Σ once per sub-block inside one FFMA; THIS multiplies a resident m over a second dp4a — and the transport-free property is exactly why the range codec's KV dot needs no quantize-side cooperation); bit-plane-plane-sums-input-side-offset-dot owns the PLANE-WEIGHTED input-side offset algebra for (q−1) codes at MMA granularity (2^k digit-weight folding) — distinct mechanism (bit-plane decomposition vs raw-code two-dp4a), same never-materialize family, cite; consumer-offset-absorption-precomputed-weight-row-correction owns the consumer-SIDE +128 activation bias with a precomputed weight-row correction — the offset rides the OTHER operand there, cite; swar-lane-widened-base3-dp4a-dot (B116 addendum) owns the dp4a DECODE half of the same prism lineage (how trit/code words reach dp4a at all) — THIS owns the OFFSET half of the dot (what happens after the decode), compose. REGIME: integer dots over offset-carrying block codecs (min/max range, offset-bias, any y = m + q·d form) on dp4a-class consumers. ANTI-REGIME: zero-anchored codecs (the m term is absent — B180's transport form is the answer there); consumers without dp4a/sdot (the two-sum trick needs a byte-parallel integer dot); codecs whose m is per-ELEMENT (the scalar multiply no longer amortizes). GOAT: the fixture pins (a) the two-dp4a decomposition vs a reference dot over exhaustive code/activation byte sweeps (bit-exact integer identity), (b) the offset-zero tie (m = 0 collapses to the plain code dot — the decomposition degenerates correctly), (c) the tamper arm (dropping the Σu dp4a reads measurably wrong for m ≠ 0), (d) the transport-free contrast (a codec without an m-word cannot run this form — the type refuses). NotBenchable-by-domain (the win metric lives in attention-kernel latencies; the integer identity is CPU-testable and pinned exhaustively). P113: the fork's own landed decision — decision content only. Source: PrismML-Eng/llama.cpp @ d12ca13bb (#428, MIT) — ggml/src/ggml-cuda/fattn-common.cuh (vec_dot_fattn_vec_KQ_pq2_0m, the two-dp4a form + comment verbatim). Quotes byte-verified at the pin this session. MIT. Batch 409 / Plan 408 / Research 442.".to_string(),
            ref_shape: r"// NAIVE: materialize the dequant, then float-dot — every K
// element pays a fma on values that are 3 bits of code times a
// block scalar, and the offset word rides per-element.
fn kq_dot_matmul(k: &RangeBlock, q: &[i8; 128], ds: f32) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..128 {
        let k_val = k.m + code_of(k, i) as f32 * k.d; // dequant materialized
        sum += k_val * q[i] as f32; // 128 scalar fmas
    }
    sum * ds
}".to_string(),
            opt_pattern: r"// FIXED: two dp4a per byte-word — codes-vs-u gives Σ(q·u),
// ones-vs-u gives Σu — and the offset is ONE scalar multiply.
fn kq_dot_offset(k: &RangeBlock, u_words: &[i32; 32], ds: &[f32; 32]) -> f32 {
    let mut sum = 0.0f32;
    for w in 0..32 {
        let c_byte = code_word_of(k, w); // 4 codes packed {c0,c1,c2,c3}
        let u = u_words[w];
        let sum_c_u = dp4a(c_byte, u, 0); // Σ(q·u)
        let sum_u = dp4a(0x01010101, u, 0); // Σu — dp4a against ones IS a sum
        // sum = d_Q * (m * sum(c0..c3) + d * sum(cK * qQ))
        sum += ds[w] * (k.m * sum_u as f32 + k.d * sum_c_u as f32);
    }
    sum
}
// no dequant values in registers; the codec's own m-word carries
// the offset — no isum transport needed (B180's additive cancel
// needs one; this multiplicative form does not).".to_string(),
        },
    ]
}

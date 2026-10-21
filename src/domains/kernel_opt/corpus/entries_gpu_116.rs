//! Entries slice 116 — the 2026-10-21 collision file: Batch 269 (Plan 272, the
//! llama.cpp Metal reduction window, rule 1) + Batch 267 (Plan 271, the cubecl
//! fold wave, rule 2 — CRLF splice preserved). Two parallel idle-loop lanes
//! allocated slice 116 the same morning; push-order kept BOTH batches (the
//! Batch-115-addendum precedent); corpus 924 -> 926.
//! Batch 269's half: the llama.cpp
//! Metal reduction window — the oversized-row argmax reduction split
//! ACROSS DISPATCHES with bounded parts, scratch past the output, and
//! the load-bearing inter-dispatch encoder memory barrier (own rule;
//! category B — Compute). Every quote re-verified VERBATIM at the pin
//! (clone, ggml-org/llama.cpp @ df1170213, PR #30862 'metal : optimize
//! argmax and `pool_1d`', MIT, merged 2026-10-20; the PR body fetched
//! 2026-10-21 for the source-reported numbers). The commit's `pool_1d`
//! half is the count-neutral Metal face on `grid-over-output-thread-`
//! `private-sweep` (`entries_gpu_06`). House string-literal safety: inner
//! double-quotes of upstream sources render as single quotes. Fixture
//! validated standalone before landing.

use super::KernelRule;

#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── B: compute — a row-reduction axis too long for one cooperative
        // threadgroup splits ACROSS DISPATCHES: a part kernel reduces bounded
        // chunks to per-(row, part) partial pairs in scratch, a final kernel
        // reduces the partials per row, and an EXPLICIT ENCODER MEMORY BARRIER
        // sits between the two dispatches sharing one command buffer — without
        // it the final kernel reads stale partials. ──
        KernelRule {
            rule_id: "two-pass-partial-reduction-inter-dispatch-barrier".to_string(),
            gpu_concept: "MECHANISM (two-pass partial row reduction across dispatches with an inter-dispatch memory barrier; category B — compute): a row-reduction kernel (argmax here; any (value, index) or scalar fold) mapped one-threadgroup-per-row walks the WHOLE row inside one threadgroup, and when the row is too long that single-block geometry serializes on one SM's width while every other SM idles. THE LAW, three load-bearing halves, verbatim from the PR body: 'Add a two-pass partial reduction for large-row argmax and use cooperative threadgroup reductions for pool_1d.' and 'A memory barrier is inserted between the part and final argmax dispatches to avoid reading stale partials.' (1) THE SPLIT — a part kernel (grid npart x nrows) reduces a BOUNDED CHUNK of each row to a per-(row, part) partial pair in scratch, then a final kernel (one threadgroup per row) reduces the npart partials; the geometry constants pin the shape: 'chunk = (ne00 + npart - 1) / npart' with npart = min(N_ARGMAX_NPART_MAX, ceil(ne00 / N_ARGMAX_CHUNK)), N_ARGMAX_CHUNK 4096, N_ARGMAX_NPART_MAX 256 — the part count is DERIVED from the axis length against a chunk floor and CAPPED at the final kernel's reduce width (256 threads), so the final pass is always one cooperative block per row. (2) THE BARRIER IS LOAD-BEARING — the two dispatches share ONE command buffer and may otherwise overlap; between them the encoder emits an explicit memory barrier (the graph's concurrency-reset seam: ggml_metal_op_concurrency_reset = ggml_metal_encoder_memory_barrier + mem-ranges reset); the PR body names the failure it prevents: 'to avoid reading stale partials' — a final kernel reading scratch the part kernel has not published is a silent wrong answer, and nothing in the dispatch API orders the two for you. (3) THE SCRATCH ECONOMY — the partials live in the graph's OWN buffer allocator ((sizeof(float) + sizeof(int32_t)) * npart * ne01 bytes, exposed as an op-extra hook so the allocator sizes the graph) and ride PAST the output tensor in the SAME buffer ('bid_tmp.offs += ggml_nbytes(op)') — no separate allocation, no lifetime to manage, the scratch dies with the graph. THE ADMISSION: rows at or under the chunk floor keep the single-pass kernel (npart == 1 returns 0 extra bytes and the original dispatch) — measured flat on small shapes ('32,10 | 3.01 us | 3.00 us'), the split is pure overhead below the floor. THE (VALUE, INDEX) PAIR REDUCTION: value reduces by simd_max; index reduces by simd_max over a select mask — 'int32_t arg_val = simd_max(select(-1, larg, lmax == max_val));' — the index rides ONLY on lanes holding the max, so no (value, index) struct or comparator is needed and the pair never diverges. TIE-RULE SURFACE (disclosed, not adjudicated): the select-mask index reduction yields LAST-index-wins across lanes on exact ties, while the sequential reference's strict-'>' scan is first-index-within-chunk — a parallel fold over (value, index) pairs carries a tie-rule decision the scalar reference made implicitly; name yours. MEASURED (source-reported, test-backend-ops perf, the PR body — the author's same-node claims, never our bar): M2 Ultra POOL_1D-adjacent ARGMAX '32000,512 | 453.17 us | 78.87 us' (5.7x); the same PR's cooperative-window half measured M2 Ultra POOL_1D 247.51->6.35 us (~39x) through 9613.05->368.94 (~26x) and M5 Max 110.52->2.79 (~40x) through 440.14->5.37 (~82x) — both boxes OUR silicon family (M-series Metal). DECISION CONTENT: (a) a reduction axis longer than one cooperative block's efficient width splits across DISPATCHES, not within one oversized block — the part count is derived (axis/chunk) and capped at the final reduce width; (b) two dispatches in one command buffer DO NOT ORDER THEMSELVES — the encoder-level memory barrier between them is load-bearing correctness, not a perf knob, and its absence is the silent stale-read class; (c) inter-kernel scratch belongs in the graph's allocator sized by an op-extra hook, riding past the output in the same buffer; (d) below the chunk floor the split is pure overhead — the admission keeps the single-pass kernel and returns zero extra bytes. CLASS SIGNATURE: a one-block-per-row reduction whose measured cost grows with row length past the block's efficient width, on an encoder that batches dispatches into one command buffer. REGIME: Metal/compute encoders with command-buffer batching (the concurrency-reset seam), argmax/logits reductions over vocabulary-scale rows (32k-128k), any two-pass fold (max+arg, sum+count, min+index). ANTI-REGIME: rows under the chunk floor (flat measured — the split overheads); encoders that serialize dispatches anyway (the barrier is free but the split still pays); axes already carried by the graph's own two-pass ops. KINSHIP (grep over src/domains/kernel_opt/corpus/, named): block-reduction-separate-scratch-runtime-strides (B) owns the WITHIN-ONE-KERNEL block reduction and where its partials live — THIS splits past the block boundary into a second dispatch and moves the barrier question up to the encoder; row-dot-split-inner-axis-block-reduce (E) owns the split-the-reduction-axis MAPPING decision (output count too small to fill the machine) — THIS owns the cross-dispatch GEOMETRY once the split is chosen and the axis is still too long; tree-reduction-depth-over-sequential-accumulate owns the reduction tree shape inside each pass — compose: tree within a pass, partials across passes, barrier across dispatches. The B267 lane's barrier-family lane intel (cubecl #1871 plane-barrier WorkgroupMemory) is the SPIR-V semantics sibling one layer down. split-prework-recompute-beats-inter-kernel-barrier (mlx-serve pack, D) owns the OPPOSITE trade on the prefill path — when the inter-kernel barrier ITSELF is the tax, recompute beats splitting; THIS owns the geometry where the split wins and legislates the barrier that makes it correct — read together they bound the split-or-fuse decision. swept-shape-bucket-launch-tables (B104) and grid-over-output-thread-private-sweep (E) own the mapping axes; the pool_1d face of THIS commit rides the latter as its Metal threadgroup-radius face. GREP EVIDENCE: 'kernel_argmax_part|argmax_final|N_ARGMAX_CHUNK|concurrency_reset|stale partials' -> ZERO corpus hits; 'two-pass' -> chained-two-pass-recurrence (a recurrence scan, not a cross-dispatch reduction). NotBenchable here (Metal-kernel geometry; the numbers above are the PR body's same-node claims on M2 Ultra/M5 Max — OUR silicon family but not this box; no wall-clock bar asserted; the decision content is verifiable by inspection of any multi-dispatch reduction). Source: ggml-org/llama.cpp @ df11702136d02b9e8fa37971de0bf0e95a91b2de (PR #30862 'metal : optimize argmax and pool_1d', merged 2026-10-20, MIT) -- ggml/src/ggml-metal/kernels/misc.metal (kernel_argmax_part_f32 + kernel_argmax_final_f32) + ggml/src/ggml-metal/ggml-metal-ops.cpp (ggml_metal_op_argmax + ggml_metal_op_argmax_extra + ggml_metal_op_concurrency_reset) + ggml/src/ggml-metal/ggml-metal-impl.h (N_ARGMAX_CHUNK 4096 / N_ARGMAX_NPART_MAX 256). Batch 269 / Plan 272 / Research 304. PR body fetched 2026-10-21; every quote verbatim at the pin.".to_string(),
            ref_shape: "// naive: one threadgroup per row walks the WHOLE row; large rows\n// serialize on one SM while the rest idle. Two dispatches in one\n// command buffer do NOT order themselves — dropping the barrier\n// between the part and final passes reads stale partials.\nuse std::sync::atomic::{AtomicUsize, Ordering};\n\nstatic ENCODER_BARRIERS: AtomicUsize = AtomicUsize::new(0);\nstatic STALE_READS: AtomicUsize = AtomicUsize::new(0);\n\nconst N_ARGMAX_CHUNK: usize = 4096;\nconst N_ARGMAX_NPART_MAX: usize = 256;\n\nstruct Row {\n    vals: Vec<f32>,\n}\n\nimpl Row {\n    // the part kernel's per-threadgroup chunk scan (two-level reduce\n    // flattened to a fold for the fixture)\n    fn chunk_argmax(&self, part: usize, npart: usize) -> (f32, i32) {\n        let chunk = (self.vals.len() + npart - 1) / npart;\n        let start = part * chunk;\n        let end = (start + chunk).min(self.vals.len());\n        let mut lmax = f32::NEG_INFINITY;\n        let mut larg = -1i32;\n        for (i, &v) in self.vals[start..end].iter().enumerate() {\n            if v > lmax {\n                lmax = v;\n                larg = (start + i) as i32;\n            }\n        }\n        (lmax, larg)\n    }\n}\n\n// dispatch 1: npart x nrows — write partial (max, arg) pairs to scratch\nfn argmax_part(rows: &[Row], npart: usize, scratch: &mut Vec<(f32, i32)>) {\n    for (row_idx, row) in rows.iter().enumerate() {\n        for part in 0..npart {\n            scratch[row_idx * npart + part] = row.chunk_argmax(part, npart);\n        }\n    }\n}\n\n// dispatch 2: one block per row — reduce the partials\nfn argmax_final(rows: &[Row], npart: usize, scratch: &[(f32, i32)], dst: &mut [i32]) {\n    let _ = rows;\n    for row_idx in 0..dst.len() {\n        let mut best = (f32::NEG_INFINITY, -1i32);\n        for part in 0..npart {\n            let cand = scratch[row_idx * npart + part];\n            if cand.0 > best.0 {\n                best = cand;\n            }\n        }\n        dst[row_idx] = best.1;\n    }\n}\n\n// the encoder: both dispatches share ONE command buffer\nfn encode(rows: &[Row], dst: &mut [i32], barrier_armed: bool) {\n    let ne00 = rows[0].vals.len();\n    let npart = if ne00 > N_ARGMAX_CHUNK {\n        (ne00 / N_ARGMAX_CHUNK + 1).min(N_ARGMAX_NPART_MAX)\n    } else {\n        1\n    };\n    if npart == 1 {\n        // admission: the single-pass kernel; zero extra bytes\n        for (r, d) in rows.iter().zip(dst.iter_mut()) {\n            *d = r.chunk_argmax(0, 1).1;\n        }\n        return;\n    }\n    let mut scratch = vec![(f32::NEG_INFINITY, -1i32); npart * rows.len()];\n    argmax_part(rows, npart, &mut scratch);\n    if barrier_armed {\n        ENCODER_BARRIERS.fetch_add(1, Ordering::SeqCst); // the encoder memory barrier\n    } else {\n        STALE_READS.fetch_add(1, Ordering::SeqCst); // dispatch 2 may read unpublished scratch\n    }\n    argmax_final(rows, npart, &scratch, dst);\n}\n\n#[test]\nfn the_split_dispatches_partials_with_the_barrier_between_them() {\n    let row = Row { vals: (0..16_384).map(|i| (i % 977) as f32).collect() };\n    let rows = vec![row];\n    let mut dst = vec![0i32; 1];\n\n    encode(&rows, &mut dst, true);\n    assert_eq!(ENCODER_BARRIERS.load(Ordering::SeqCst), 1, \"one barrier between part and final\");\n    assert_eq!(STALE_READS.load(Ordering::SeqCst), 0);\n    // the split result agrees with a straight scan\n    let (mut m, mut a) = (f32::NEG_INFINITY, -1i32);\n    for (i, &v) in rows[0].vals.iter().enumerate() {\n        if v > m { m = v; a = i as i32; }\n    }\n    assert_eq!(dst[0], a);\n}\n\n#[test]\nfn small_rows_keep_the_single_pass_and_zero_extra_bytes() {\n    let rows = vec![Row { vals: vec![1.0, 9.0, 3.0] }];\n    let mut dst = vec![0i32; 1];\n    let barriers_before = ENCODER_BARRIERS.load(Ordering::SeqCst);\n    encode(&rows, &mut dst, true);\n    assert_eq!(dst[0], 1);\n    assert_eq!(\n        ENCODER_BARRIERS.load(Ordering::SeqCst),\n        barriers_before,\n        \"no split dispatches, no barrier owed\"\n    );\n}\n".to_string(),
            opt_pattern: "// optimized: the oversized axis splits ACROSS DISPATCHES — a part kernel\\n// (grid npart x nrows) reduces bounded chunks to per-(row, part) partial pairs\\n// in graph-allocator scratch riding past the output, an EXPLICIT ENCODER MEMORY\\n// BARRIER orders the two dispatches sharing one command buffer, then a final\\n// kernel (one threadgroup per row, nth = next-pow2(npart) capped 256) reduces\\n// the partials. Small rows keep the single-pass kernel (npart == 1 -> zero\\n// extra bytes). The (value, index) pair reduces value by the max and index by\\n// a select-mask over max-holding lanes.\\nuse std::sync::atomic::{AtomicUsize, Ordering};\\n\\nstatic ENCODER_BARRIERS: AtomicUsize = AtomicUsize::new(0);\\n\\nconst N_ARGMAX_CHUNK: usize = 4096;\\nconst N_ARGMAX_NPART_MAX: usize = 256;\\n\\nfn parts_for(ne00: usize) -> usize {\\n    if ne00 > N_ARGMAX_CHUNK {\\n        ((ne00 + N_ARGMAX_CHUNK - 1) / N_ARGMAX_CHUNK).min(N_ARGMAX_NPART_MAX)\\n    } else {\\n        1\\n    }\\n}\\n\\nstruct ReductionPass<'a> {\\n    rows: &'a [Vec<f32>],\\n    npart: usize,\\n    scratch: Vec<(f32, i32)>, // (max, arg) per (row, part) — graph-allocator scratch\\n}\\n\\nimpl<'a> ReductionPass<'a> {\\n    fn new(rows: &'a [Vec<f32>]) -> Self {\\n        let npart = parts_for(rows[0].len());\\n        Self { rows, npart, scratch: vec![(f32::NEG_INFINITY, -1); npart * rows.len()] }\\n    }\\n\\n    // dispatch 1: npart x nrows — each block strided-scans its chunk\\n    fn part(&mut self) {\\n        let ne00 = self.rows[0].len();\\n        let chunk = (ne00 + self.npart - 1) / self.npart;\\n        for (r, row) in self.rows.iter().enumerate() {\\n            for part in 0..self.npart {\\n                let (start, end) = (part * chunk, ((part + 1) * chunk).min(ne00));\\n                let (mut lmax, mut larg) = (f32::NEG_INFINITY, -1i32);\\n                for (i, &v) in row[start..end].iter().enumerate() {\\n                    if v > lmax { lmax = v; larg = (start + i) as i32; }\\n                }\\n                self.scratch[r * self.npart + part] = (lmax, larg);\\n            }\\n        }\\n    }\\n\\n    // THE BARRIER — the two dispatches share one command buffer; without this\\n    // the final kernel may read partials the part kernel has not published.\\n    fn encoder_memory_barrier(&mut self) {\\n        ENCODER_BARRIERS.fetch_add(1, Ordering::SeqCst);\\n    }\\n\\n    // dispatch 2: one block per row — reduce the npart partials\\n    fn finalize(&self, dst: &mut [i32]) {\\n        for (r, d) in dst.iter_mut().enumerate() {\\n            let mut best = (f32::NEG_INFINITY, -1i32);\\n            for part in 0..self.npart {\\n                let cand = self.scratch[r * self.npart + part];\\n                if cand.0 > best.0 { best = cand; }\\n            }\\n            *d = best.1;\\n        }\\n    }\\n}\\n\\nfn encode_argmax(rows: &[Vec<f32>], dst: &mut [i32]) {\\n    let mut pass = ReductionPass::new(rows);\\n    if pass.npart == 1 {\\n        // admission: the single-pass kernel — zero extra scratch bytes\\n        for (r, d) in rows.iter().zip(dst.iter_mut()) {\\n            let (mut m, mut a) = (f32::NEG_INFINITY, -1i32);\\n            for (i, &v) in r.iter().enumerate() {\\n                if v > m { m = v; a = i as i32; }\\n            }\\n            *d = a;\\n        }\\n        return;\\n    }\\n    pass.part();\\n    pass.encoder_memory_barrier();\\n    pass.finalize(dst);\\n}\\n\\n#[test]\\nfn the_optimized_form_splits_and_orders_the_dispatches() {\\n    let rows = vec![(0..16_384).map(|i| (i % 977) as f32).collect::<Vec<_>>()];\\n    let mut dst = vec![0i32; 1];\\n    encode_argmax(&rows, &mut dst);\\n    let (mut m, mut a) = (f32::NEG_INFINITY, -1i32);\\n    for (i, &v) in rows[0].iter().enumerate() {\\n        if v > m { m = v; a = i as i32; }\\n    }\\n    assert_eq!(dst[0], a);\\n    assert_eq!(ENCODER_BARRIERS.load(Ordering::SeqCst), 1);\\n}\\n\\n#[test]\\nfn parts_derive_from_the_axis_and_cap_at_the_reduce_width() {\\n    assert_eq!(parts_for(4_096), 1, \"at the chunk floor the single pass stays\");\\n    assert_eq!(parts_for(4_097), 2);\\n    assert_eq!(parts_for(1 << 20), 256, \"capped at the final kernel's reduce width\");\\n}\\n\\n#[test]\\nfn small_rows_cost_zero_extra_bytes() {\\n    let rows = vec![vec![1.0, 9.0, 3.0]];\\n    let mut dst = vec![0i32; 1];\\n    let before = ENCODER_BARRIERS.load(Ordering::SeqCst);\\n    encode_argmax(&rows, &mut dst);\\n    assert_eq!(dst[0], 1);\\n    assert_eq!(ENCODER_BARRIERS.load(Ordering::SeqCst), before);\\n}\\n".to_string(),
        },
        KernelRule {
            rule_id: "partial-constant-float-fold-ieee-unsound".to_string(),
            gpu_concept: "MECHANISM (IEEE-754-sound constant folding in a GPU IR dialect; category N — dtype/numerics, disclosure below): a float binop whose fold substitutes a VALUE for a PARTIAL constant operand (one side constant, the other runtime) is not IEEE 754-sound — signed zeros, NaN and infinity make the result depend on the runtime operand — so the dialect folds ONLY the both-constants case, and the rewrite pass that forms FMAs from mul+add pairs type-tests the SCALAR and refuses complex. Source commit, verbatim title: 'fix(ir,opt): drop IEEE-invalid float constant folds and keep FMA formation real (#1816)'. THE FIVE DROPPED FOLDS, each with its in-source reason (crates/cubecl-ir/src/dialect/math.rs, at-commit lines). FAdd (both `0.0 + x -> x` simplify arms dropped; no FAdd simplify remains), verbatim (math.rs:266): 'No `FAddOp` simplify: `0.0 + x` is not `x` (`+0.0 + -0.0` is `+0.0`).'. FSub (the `x - x -> +0` custom fold dropped), verbatim (math.rs:322-323): 'No `x - x -> +0` fold: `NaN - NaN` and `inf - inf` are NaN, so the result depends on the runtime operand. Only the both-constants case is folded.'. FMul (the `0 * x -> 0; x * 0 -> 0` custom fold dropped), verbatim (math.rs:383-385): 'A partial fold (one operand a constant zero) is not sound under IEEE 754: `-4.0 * 0.0` is `-0.0` and `NaN * 0.0` is `NaN`, so the result depends on the runtime operand. Only the both-constant case is folded.'. FDiv (the `0 / x -> 0` custom fold dropped), verbatim (math.rs:493-494): '0 / x is NaN for x = 0, so the partial fold is not sound; both constants are folded by the arm below.'. FRem (the `0 % x -> 0` custom fold dropped AND the `x % 1.0 -> x` simplify arm removed with it — that arm was wrong on its own terms: `v % 1.0` is `0.5` for `v = 2.5`, not `v`), verbatim (math.rs:622): '0 % x is NaN for x = 0, so no partial fold; both constants fold below.'; FModFloor rides the same clause, verbatim (math.rs:666): 'As for `FRemOp`: a zero dividend does not decide the result.'. THE IDENTITY ARMS THAT REMAIN are exactly the bit-exact ones — every const_eval! arm still folds BOTH-constants (`FloatAttr: |lhs, rhs| lhs op rhs` — no runtime operand, nothing to depend on), and simplify! keeps `x * 1.0 -> x` in both operand orders (math.rs:388-397), `x / 1.0 -> x` (math.rs:497-502), and `x - 0.0 -> x` (math.rs:326-331, the constant matched by `to_bits() == 0`, i.e. +0.0 specifically): multiplying or dividing by 1.0 and subtracting +0.0 are bit-exact on EVERY operand (signed zeros included; the only behavioral difference is sNaN quieting, which the real op would introduce anyway), while ADDING +0.0 is not (`+0.0 + -0.0` is `+0.0`) — the add/sub asymmetry is the rule's sharpest edge: add keeps NO identity arm, sub keeps its arm, and remainder keeps neither. THE FMA HALF: `as_fma` (the mul+add -> fma combine in crates/cubecl-opt/src/passes/inst_combine.rs) tests the multiply's scalar type and refuses complex, verbatim (inst_combine.rs:51-54): '`fma` exists for real scalars and real vectors: the backends emit the target's native call (or a vectorized one), and there is no complex overload — a complex FMA does not compile. Test the *scalar* type, since a complex vector's own type is a vector, not a complex scalar.' — the guard derefs through the vector (`scalar_ty(ctx)` then `deref(ctx)`, refusing Complex32Type and Complex64Type): a complex VECTOR's own type is a vector, not a complex scalar, so the guard must look through it. A complex product consumed by an add STAYS a multiply and an add — the added runtime tests, verbatim (complex.rs:1207): 'A complex product consumed by an add must stay a multiply and an add.' and (complex.rs:1209-1211): 'Forming an FMA from it does not compile: the backends' `fma` has no complex overload (`fma(cuDoubleComplex, ...)` in the C++ backend, and no `llvm.fmuladd` for complex either).' — with the vector twin (complex.rs:1165-1166): 'The same product-and-add on a *vector* of complex values: a vector's own type is a vector, not a complex scalar, so the FMA guard has to look through it.'. THE INSTRUMENT IS A RUNTIME TEST, not a unit test of the folder: test_ieee_constant_folds launches a kernel computing `v % 1.0`, `v - v`, `0.0 / v`, `v * 0.0`, `0.0 + v`, `0.0 % v` over {2.5, 4.0, -4.0, 0.0, NaN, inf, -0.0} and asserts the IEEE answers — its doc, verbatim (binary.rs:1018-1022): 'A runtime operand must not be rewritten through a constant zero or a reflexive identity. Each of these was a partial fold in the float dialect: `v % 1.0` is `0.5` for `2.5` (not `v`), `v - v` is `NaN` for `NaN` and infinity (not `+0`), `0.0 / v` is `NaN` for `v = 0`, and `v * 0.0` keeps the sign of `v`.' — with the back-end latitude disclosed in the test itself, verbatim (binary.rs:1037-1043): 'Not on wgpu: the shader still carries every one of these operations, but WGSL lets an implementation assume no NaN or infinity arises and ignore the sign of a zero, and so do Vulkan and Metal unless the shader opts into preserving them, which wgpu does not. The shader compilers fold under that latitude (`NaN - NaN` and `0 / 0` read back as `0`, `-4 * 0` as `+0`). What this test guards — that cubecl's own IR does not fold them — holds on every backend; the results wgpu does promise are still checked there. The runtime name says which backend, whatever compiler it picked.' — the distinction the whole rule hangs on: the IR must not ASSUME the latitude even where the shading language grants it, because the IR cannot know which consumer needs the corners; the strict-IEEE assertions are keyed on the runtime name (`!client.name().starts_with('wgpu')`) so the IR-level law is asserted where the backend promises the semantics. DECISION CONTENT: (a) classification — a fold is sound iff the rewritten expression is bit-equivalent to the original for EVERY operand; value-substituting partial folds fail for signed zero (the sign of the runtime operand survives: `-4.0 * 0.0` is `-0.0`), NaN (propagation: `NaN * 0.0` is `NaN`), and inf (NaN results: `inf - inf`, `0 / 0`, `0 % 0`); (b) identity-element simplifications are the sound partial class and they are PER-OPERATION: mul/div-by-1.0 and sub-of-+0.0 survive, add-of-+0.0 does not, remainder has neither; (c) the both-constants fold is ALWAYS sound; (d) formation rewrites (fma) need the type gate the BACKENDS actually implement — form only what every target can lower; (e) test at the boundary the latitude actually applies at (the compiled kernel), and pin the IR-level law separately from what each backend's compiler folds under the language's latitude. REGIME: IR/compiler constant folders and peepholes over float ops (GPU dialects, shader compilers, tensor-compiler simplify passes), FMA-formation rewrites, and the parity tests guarding them. ANTI-REGIME: integer folding (a two's-complement partial fold like `x * 0 -> 0` IS sound — this law is float-specific); passes downstream of an explicit fast-math/nnan/nsz opt-in (once the PROGRAM grants the latitude the folds become sound — gate it, never grant it silently); debug-only value previews where bit-fidelity is already disclaimed. KINSHIP (dedup verdicts at authoring — 'constant fold / const_eval / simplify / peephole / IEEE / signed zero / partial fold' grepped over BOTH corpora): zero prior owners. fast-math-contraction-behind-feature-flag (B29) owns OPTING INTO unsound numerics behind a flag (contraction changes rounding by design); THIS owns the DEFAULT posture staying IEEE-sound — compose (grant the latitude explicitly, never by a silent fold); scale-identity-elide-in-framework-graph-transcription (B170) is the sound direction of the same instinct (`x * 1.0` IS x — elide the identity dispatch), THIS is the unsound direction's refusal (`0.0 * x` is NOT `0.0`); strength-reduce-mul-pow2 + clamp-runtime-divisor-for-lsr-address-reduction (O family) are the INTEGER compiler-transform kin — two's complement makes their partial folds sound, the float law is the contrast; magic-anchor-mma-exact-int-float-ffma-epilogue (B251) owns exact-domain contracts for rewrites (the both-constants clause is the trivially-exact domain here); finiteness-before-diff-nan-blind-comparison-hole (B242) is the DETECTION sibling (a gate that cannot see NaN — the dropped folds were the manufacture side of that wrong-value class); exhaustive-domain-input-patch-bit-identity (B115) / enumerable-domain-bit-identity-admission-gate (B195) own the enumeration instrument whose corner set {2.5, 4.0, -4.0, 0.0, NaN, inf, -0.0} this rule's runtime test samples the frontier of. CATEGORY DISCLOSURE (authored): N (dtype/numerics) chosen over O (IR strength reduction) — the fix lands in cubecl-ir/cubecl-opt and O hosts the corpus's cubecl-compiler precedent, but the load-bearing law is IEEE-754 float semantics (what a fold may assume about signed zeros/NaN/inf), the axis N's numerics family owns (finiteness-before-diff-nan-blind-comparison-hole, where-not-multiply-additive-mask-inf, bit-equal-fusion-mirrors-reference-reduction-regime, minifloat-normal-code-shift-bias-diff-decode). Source: tracel-ai/cubecl @ 1ad5040fbfcd76b54b1aeffc6c08e3dfbaeea1e3 (#1816, merged 2026-10-20, Apache-2.0 OR MIT — dual license verified at the clone root) -- crates/cubecl-ir/src/dialect/math.rs:266,322-323,383-385,493-494,622,666 (the dropped folds' in-source reasons) + :326-331,388-397,497-502 (the identity arms that remain), crates/cubecl-opt/src/passes/inst_combine.rs:51-57 (the FMA scalar gate), crates/cubecl-core/src/runtime_tests/binary.rs:1018-1022,1037-1043 (the runtime instrument + the wgpu latitude), crates/cubecl-core/src/runtime_tests/complex.rs:1165-1166,1207-1211 (the complex refusals). NotBenchable here (correctness-class compiler content — the instrument is the corner-value runtime test, never a wall-clock bar; no perf number rides the commit). Batch 267 / Plan 271 / Research 303.".to_string(),
            ref_shape: "// naive: the float dialect folds PARTIAL constant operands — one side
// constant rewrites the whole op. IEEE 754 makes every one of these
// wrong on the corners: signed zeros, NaN, infinity.
#[derive(Clone, Copy)]
enum Operand {
    Const(f64),
    Runtime,
}

// `0 * x -> 0; x * 0 -> 0` — WRONG: `-4.0 * 0.0` is `-0.0`, `NaN * 0.0`
// is NaN; the result depends on the runtime operand.
fn fold_mul_zero(lhs: Operand, rhs: Operand) -> Option<f64> {
    let zero = |o: &Operand| matches!(o, Operand::Const(v) if *v == 0.0);
    if zero(&lhs) || zero(&rhs) {
        return Some(0.0);
    }
    match (lhs, rhs) {
        (Operand::Const(l), Operand::Const(r)) => Some(l * r),
        _ => None,
    }
}

// `0 / x -> 0` — WRONG: `0 / 0` is NaN.
fn fold_div_zero(lhs: Operand, rhs: Operand) -> Option<f64> {
    if let Operand::Const(v) = lhs {
        if v == 0.0 {
            return Some(0.0);
        }
    }
    match (lhs, rhs) {
        (Operand::Const(l), Operand::Const(r)) => Some(l / r),
        _ => None,
    }
}

// `0.0 + x -> x` — WRONG: `+0.0 + -0.0` is `+0.0`, not `-0.0`.
fn fold_add_zero(lhs: Operand, rhs: Operand) -> Option<Operand> {
    if let Operand::Const(v) = lhs {
        if v == 0.0 {
            return Some(rhs);
        }
    }
    None
}

// `x - x -> 0` — WRONG: `NaN - NaN` and `inf - inf` are NaN.
fn fold_sub_self(lhs: Operand, rhs: Operand) -> Option<f64> {
    if matches!((lhs, rhs), (Operand::Runtime, Operand::Runtime)) {
        return Some(0.0);
    }
    None
}

#[test]
fn partial_folds_disagree_with_ieee_on_the_corners() {
    // `0.0 * -4.0` is `-0.0`: the fold's `0.0` has the wrong SIGN BIT
    let folded = fold_mul_zero(Operand::Const(0.0), Operand::Runtime);
    assert_eq!(folded, Some(0.0));
    assert_ne!((0.0f64 * -4.0).to_bits(), folded.unwrap().to_bits());
    // `NaN * 0.0` is NaN: the fold's `0.0` is a different VALUE CLASS
    assert!((0.0f64 * f64::NAN).is_nan());
    // `0 / 0` is NaN; `NaN - NaN` is NaN; `+0.0 + -0.0` loses the sign
    assert!(fold_div_zero(Operand::Const(0.0), Operand::Runtime) == Some(0.0));
    assert!((0.0f64 / 0.0).is_nan());
    assert!((f64::NAN - f64::NAN).is_nan());
    assert_ne!((0.0f64 + -0.0f64).to_bits(), (-0.0f64).to_bits());
}
"
            .to_string(),
            opt_pattern: "// optimized: ONLY the both-constants case folds; a partial fold is
// refused (the result depends on the runtime operand); the identity
// arms that remain are exactly the bit-exact ones (x * 1.0, x / 1.0,
// x - (+0.0) — never add-of-zero, never remainder); and FMA formation
// tests the SCALAR type and refuses complex.
type Const = Option<f64>;

fn fold_mul(lhs: Const, rhs: Const) -> Const {
    match (lhs, rhs) {
        // the only fold: no runtime operand, nothing to depend on
        (Some(l), Some(r)) => Some(l * r),
        // `-4.0 * 0.0` is `-0.0`; `NaN * 0.0` is `NaN` — refused
        _ => None,
    }
}

fn fold_add(lhs: Const, rhs: Const) -> Const {
    match (lhs, rhs) {
        (Some(l), Some(r)) => Some(l + r),
        _ => None, // `0.0 + x` is not `x`: `+0.0 + -0.0` is `+0.0`
    }
}

// the sound partial class, per operation: identity arms only
fn simplify_mul_identity(lhs: Const, rhs: Const) -> bool {
    lhs == Some(1.0) || rhs == Some(1.0) // x * 1.0 is bit-exact x
}

fn simplify_div_identity(rhs: Const) -> bool {
    rhs == Some(1.0) // x / 1.0 is bit-exact x
}

fn simplify_sub_zero(rhs: Const) -> bool {
    // +0.0 specifically: to_bits() == 0. x - (+0.0) is bit-exact x,
    // even for x = -0.0. ADD has no arm here — `0.0 + x` is not `x`.
    rhs.map_or(false, |v| v.to_bits() == 0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScalarTy {
    Real32,
    Real64,
    Complex32,
    Complex64,
}

// a complex vector's own type is a vector: the guard tests the SCALAR
// it derefs to, never the value type it is spelled with
#[derive(Clone, Copy)]
struct ValueType {
    scalar: ScalarTy,
    is_vector: bool,
}

fn should_form_fma(t: ValueType) -> bool {
    let scalar = t.scalar; // the deref
    !matches!(scalar, ScalarTy::Complex32 | ScalarTy::Complex64)
}

#[test]
fn only_both_constants_fold_and_identities_remain() {
    assert_eq!(fold_mul(Some(2.0), Some(3.0)), Some(6.0));
    assert_eq!(fold_mul(Some(0.0), None), None); // the refusal
    assert_eq!(fold_mul(None, Some(0.0)), None);
    assert_eq!(fold_add(Some(0.0), None), None);
    // identity arms: sound partial simplifications
    assert!(simplify_mul_identity(Some(1.0), None));
    assert!(simplify_mul_identity(None, Some(1.0)));
    assert!(simplify_div_identity(Some(1.0)));
    assert!(simplify_sub_zero(Some(0.0)));
    // x - (+0.0) is bit-exact x even for -0.0:
    assert_eq!((-0.0f64 - 0.0f64).to_bits(), (-0.0f64).to_bits());
    // ...but there is NO add-of-zero arm, and no remainder arm at all
    assert!(!simplify_mul_identity(Some(0.0), None));
    // the FMA gate: real forms, complex refuses — scalar OR vector
    assert!(should_form_fma(ValueType { scalar: ScalarTy::Real32, is_vector: false }));
    assert!(should_form_fma(ValueType { scalar: ScalarTy::Real64, is_vector: true }));
    assert!(!should_form_fma(ValueType { scalar: ScalarTy::Complex32, is_vector: false }));
    assert!(!should_form_fma(ValueType { scalar: ScalarTy::Complex64, is_vector: true }));
}
"
            .to_string(),
        },
    ]
}

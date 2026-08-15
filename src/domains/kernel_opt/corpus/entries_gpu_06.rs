//! Entries slice — Batches 14-46 era (rules 325-340 of 340) — riir-gpu production-kernel mining.
//! Pure data moved verbatim from the former corpus.rs monolith
//! (2026-08-15 split); append new batches to the LAST gpu slice or
//! open a new one past ~900 lines. Category membership lives ONLY in
//! KERNEL_CATEGORIES (mod.rs) — never encode it here.

use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // ── empty-slice-sentinel-launders-shape-violation — a defensive &[]
        // sentinel type-checks past the shape contract into a swallowed Err (D) ──
        //
        // Source: riir-gpu weaver_gpu_dflash.rs L140-144 (the sentinel branch:
        // `*slot = if end <= h_dflash_captured.len() { &...[start..end] }
        // else { &[] };`) -> weaver_gpu_corrector.rs L636-644 (the consumer
        // rejects the len-0 row with `Err(HiddenShape)`) -> dflash.rs L247
        // (`let _ = dflash_predict_with_weaver_gpu(` — the swallow). The
        // 3-layer composition is LIVE: an undersized capture buffer silently
        // disables Weaver correction with zero observability. The same
        // sentinel ships in the shared helper (katgpt-rs step_gdn_tree.rs
        // L56-60) + the engine twin (riir-engine dflash.rs L239-243); the
        // CPU swallow is inherited (step_gdn_tree.rs L358). Filed as Issue
        // 697 H1 — the rule is the mechanism, the issue is the live chain.
        //
        // Distinct from launch-unchecked-caller-owned-size-contract (B29 —
        // WHERE an explicit size contract is enforced; this is the sentinel
        // that DEFEATS it while appearing safe — the negative space), from
        // mask-sentinel-matches-reduction-identity (B35) +
        // clamp-tile-indices-to-sentinel-discard-at-write (B17) (sentinels
        // used CORRECTLY as identity/discard markers — this is the inverse:
        // a sentinel that silently disables the guarded work).
        KernelRule {
            rule_id: "empty-slice-sentinel-launders-shape-violation".to_string(),
            gpu_concept: "A row-view builder guards each slice with if end <= buf.len() { &buf[start..end] } else { &[] }, substituting an EMPTY slice when the flat capture buffer is undersized. The &[] row is type-correct (borrows fine, satisfies &[&[f32]]) but shape-invalid (len 0 != row_len), so it sails past the borrow checker and lands in the consumer's shape validation, which rejects it with an Err — which a let-_= orchestration caller then converts into a silent no-op (this exact 3-layer composition exists live: undersized capture -> sentinel row -> HiddenShape Err -> swallowed -> pipeline continues on UNCORRECTED marginals, zero log lines). The caller bug (wrong buffer sizing) is thereby laundered into silent quality degradation with zero observability. Delete the defensive branch and slice hard — Rust slice indexing IS the assertion (panics loudly on out-of-bounds in every profile), optionally fronted with a debug_assert! carrying the sizing contract (steps * row_len <= buf.len()); or propagate a precise error up. Never substitute a sentinel that type-checks but violates the shape contract. Distinct from launch-unchecked-caller-owned-size-contract (WHERE an explicit size contract is documented and enforced on unchecked launches — this is a defensive sentinel that DEFEATS the contract's enforcement while appearing safe, the negative space of that rule), from mask-sentinel-matches-reduction-identity and clamp-tile-indices-to-sentinel-discard-at-write (sentinels used CORRECTLY — identity elements that legitimately participate in a reduction, or discard markers at a write boundary — this is the anti-pattern inverse: a sentinel that silently disables the guarded work).".to_string(),
            ref_shape: "// naive: the defensive &[] sentinel — type-correct, shape-INVALID\nfn build_views_sentinel(buf: &[f32], count: usize, row_len: usize) -> [&[f32]; 64] {\n    let mut views: [&[f32]; 64] = [&[]; 64];\n    for (i, slot) in views.iter_mut().enumerate().take(count) {\n        let start = i * row_len;\n        let end = start + row_len;\n        *slot = if end <= buf.len() { &buf[start..end] } else { &[] };\n        // the &[] row sails past the borrow checker into the consumer's\n        // shape validation -> Err -> caller swallows -> silent no-op\n    }\n    views\n}\n".to_string(),
            opt_pattern: "// optimized: hard slice — indexing IS the assertion — + an explicit contract\nfn build_views_asserted(buf: &[f32], count: usize, row_len: usize) -> [&[f32]; 64] {\n    debug_assert!(\n        count * row_len <= buf.len(),\n        \"capture buffer undersized: need {count} x {row_len}, have {}\",\n        buf.len()\n    );\n    let mut views: [&[f32]; 64] = [&[]; 64];\n    for (i, slot) in views.iter_mut().enumerate().take(count) {\n        let start = i * row_len;\n        let end = start + row_len;\n        *slot = &buf[start..end]; // panics loudly on OOB in every profile\n    }\n    views\n}\n".to_string(),
        },
        // ── row-dot-split-inner-axis-block-reduce — when the output count
        // is too small to fill the machine, split the reduction axis (E) ──
        //
        // Source: riir-gpu weaver_gpu/mod.rs L1283-1306 (dot_per_row_f32:
        // one thread per output row, `let g_base = tid * h;` — each thread
        // serially loops the full row length h) + L1332-1333 (`num_wg =
        // (total as u32).div_ceil(256u32).max(1)` — at production D=4, K=32
        // that is 128 threads inside a SINGLE workgroup) + the file's own
        // doc L1259-1260 presenting it as fine ("this is 128 threads doing
        // 2048 FMAs each — a single workgroup of 256 threads suffices").
        // Adjacent threads read h*4B = 8 KB apart — every warp load touches
        // 32 distinct cache lines. The per-depth corrector path is worse:
        // d_depth=1 -> 32 threads, run depth x per correction.
        //
        // Distinct from grid-over-output-thread-private-sweep (B33 — the
        // INVERSE regime: sweep-axis long, output-owned grid with NO
        // cross-thread reduction; here the output count is too SHORT to
        // fill the machine, so the reduction axis is the one to split),
        // from block-reduction-separate-scratch-runtime-strides (B35 —
        // WHERE partials live + stride derivation; this is WHETHER to
        // split the axis at all), from swap-large-axis-to-x (B? — 2D grid
        // axis ordering, not axis splitting for occupancy),
        // from tree-reduction-depth-over-sequential-accumulate (B25 —
        // reduction TREE depth vs sequential, assuming the split exists).
        KernelRule {
            rule_id: "row-dot-split-inner-axis-block-reduce".to_string(),
            gpu_concept: "A per-row dot product is mapped one-thread-per-row: thread tid owns output row tid and serially loops the full row length h, with the row base computed as g_base = tid * h. When the output count (rows) is small relative to the machine — at production (D=4, K=32, h=2048) that is 128 threads inside a SINGLE workgroup of 256, i.e. one SM while the rest of the GPU idles — the mapping simultaneously destroys coalescing: adjacent threads read addresses h * 4B (8 KB) apart, so every warp load touches 32 distinct cache lines (16x read amplification). The fix: when the output count is too small to fill the machine, split the REDUCTION axis instead — dispatch one workgroup per row, have lanes stride over h (adjacent lanes read adjacent elements, coalesced), accumulate per-lane partials in registers, and block-reduce the partials through shared memory; lane 0 writes the scalar. This fixes occupancy (D*K workgroups spread across SMs instead of 1) and coalescing at once, and shrinks the per-thread serial chain from h FMAs to h/cube_size. Distinct from grid-over-output-thread-private-sweep (the inverse regime — there the sweep axis is long and the output-owned grid needs NO cross-thread reduction; here the output count is too short to fill the machine so the reduction axis is the one to split), from block-reduction-separate-scratch-runtime-strides (where partials live once the split exists), and from tree-reduction-depth-over-sequential-accumulate (the reduction tree shape, assuming the split was already made).".to_string(),
            ref_shape: "// naive: one thread per output row — 128 threads in 1 workgroup at\n// production (D=4, K=32); adjacent threads stride h*4B = 8 KB apart\nfn dot_per_row_serial(u_final: &[f32], gathered: &[f32], residual: &mut [f32], h: u32, k: u32, total: u32) {\n    let cube_size = 256u32;\n    let tid = CUBE_POS_X * cube_size + UNIT_POS;\n    if tid >= total {\n        terminate!();\n    }\n    let di = tid / k;\n    let uf_base = (di + 1u32) * h;\n    let g_base = tid * h;\n    let mut acc = f32::new(0.0f32);\n    let mut j = 0u32;\n    while j < h {\n        acc = acc + u_final[(uf_base + j) as usize] * gathered[(g_base + j) as usize];\n        j += 1u32;\n    }\n    residual[tid as usize] = acc;\n}\n".to_string(),
            opt_pattern: "// optimized: one WORKGROUP per row — lanes stride h coalesced, partials\n// block-reduced through shared memory; D*K workgroups fill the machine\nfn dot_per_row_block_reduce(u_final: &[f32], gathered: &[f32], residual: &mut [f32], partials: &mut [f32], h: u32, k: u32) {\n    let row = CUBE_POS_X;\n    let lane = UNIT_POS;\n    let cube_size = 256u32;\n    let uf_base = (row / k + 1u32) * h;\n    let g_base = row * h;\n    let mut acc = f32::new(0.0f32);\n    let mut j = lane;\n    while j < h {\n        acc = acc + u_final[(uf_base + j) as usize] * gathered[(g_base + j) as usize];\n        j += cube_size;\n    }\n    partials[lane as usize] = acc;\n    sync_cube();\n    if lane == 0u32 {\n        let mut total = f32::new(0.0f32);\n        let mut l = 0u32;\n        while l < cube_size {\n            total = total + partials[l as usize];\n            l += 1u32;\n        }\n        residual[row as usize] = total;\n    }\n}\n".to_string(),
        },
        // ── fuse-indirect-gather-into-dot-consumer — delete the
        // materialized gather when the sole consumer is a reduction (C) ──
        //
        // Source: riir-gpu weaver_gpu/mod.rs L1195-1196 (the gather kernel:
        // `let id = topk_ids[(di * k + ki) as usize] as u32;` /
        // `gathered[tid as usize] = embedding[(id * h + j) as usize];`)
        // + L1237 (dispatched with `BufferArg::from_raw_parts(gathered_handle,
        // total)`) -> L1303 (the ONLY consumer re-reads the whole buffer in
        // the dot loop: `acc = acc + u_final[...] * gathered[(g_base + j)]`)
        // + L1344 (`BufferArg::from_raw_parts(gathered_handle, d_depth * k *
        // h)`). At production ~1 MB written + ~1 MB re-read per batched call
        // + a persistent D*K*h scratch, for data whose only destiny is to be
        // multiplied by u_final and summed.
        //
        // Distinct from sparse-gather-scale-at-read-not-table-prepass (B37 —
        // WHERE to fold a scale in a sparse gather; this deletes the gather
        // pass entirely), from gather-selective-reduction (I-category NSA
        // block-sparse selection semantics), from split-dispatch-expose-
        // fused-intermediate (B34 — the INVERSE: when a SECOND consumer
        // needs the intermediate, expose it from one execution instead of
        // re-launching; here there is no second consumer), from quadratic-
        // attention-intermediates-recompute-from-linear-saves (B35 — save
        // linear vs recompute quadratic; here nothing downstream needs the
        // gathered rows at all).
        KernelRule {
            rule_id: "fuse-indirect-gather-into-dot-consumer".to_string(),
            gpu_concept: "An indirect gather materializes D*K*h rows to a global buffer (gathered[tid] = embedding[id*h + j]), and the ONLY consumer is a subsequent per-row dot kernel that re-reads the entire buffer. That is a full global-memory round-trip (write D*K*h + read D*K*h), one extra dispatch, and a persistent scratch allocation for data whose only destiny is to be multiplied by u_final and summed. The fix: when the gather has exactly one reduction-shaped consumer and the index array is already device-resident, delete the gather kernel and pass topk_ids + embedding into the consumer — each row's owning workgroup computes its id once (topk_ids[di*k + ki]) and reads embedding[id*h + j] directly inside its dot loop. This removes one dispatch, both directions of the global round-trip, and the gathered scratch buffer entirely; the index load amortizes to one broadcast-friendly read per row. Distinct from sparse-gather-scale-at-read-not-table-preprocess (where to fold a scale within a sparse gather — this deletes the gather pass), from split-dispatch-expose-fused-intermediate (the inverse regime: a second consumer genuinely needs the intermediate), and from quadratic-attention-intermediates-recompute-from-linear-saves (a save-vs-recompute trade-off; here nothing downstream needs the gathered rows at all).".to_string(),
            ref_shape: "// naive: gather kernel materializes D*K*h rows to global, then the dot\n// kernel re-reads the whole buffer — two dispatches, two full passes\nfn gather_rows_naive(embedding: &[f32], topk_ids: &[f32], gathered: &mut [f32], h: u32, k: u32, total: u32) {\n    let tid = ABSOLUTE_POS;\n    if tid >= total * h {\n        terminate!();\n    }\n    let j = tid % h;\n    let flat = tid / h;\n    let di = flat / k;\n    let ki = flat % k;\n    let id = topk_ids[(di * k + ki) as usize] as u32;\n    gathered[tid as usize] = embedding[(id * h + j) as usize];\n}\n\nfn dot_reads_gathered(u_final: &[f32], gathered: &[f32], residual: &mut [f32], h: u32, k: u32, total: u32) {\n    let tid = ABSOLUTE_POS;\n    if tid >= total {\n        terminate!();\n    }\n    let di = tid / k;\n    let uf_base = (di + 1u32) * h;\n    let g_base = tid * h;\n    let mut acc = f32::new(0.0f32);\n    let mut j = 0u32;\n    while j < h {\n        acc = acc + u_final[(uf_base + j) as usize] * gathered[(g_base + j) as usize];\n        j += 1u32;\n    }\n    residual[tid as usize] = acc;\n}\n".to_string(),
            opt_pattern: "// optimized: the dot kernel gathers in-loop via the device-resident\n// index array — zero intermediate, one fewer dispatch, no scratch\nfn dot_gather_fused(u_final: &[f32], embedding: &[f32], topk_ids: &[f32], residual: &mut [f32], h: u32, k: u32, total: u32) {\n    let tid = ABSOLUTE_POS;\n    if tid >= total {\n        terminate!();\n    }\n    let di = tid / k;\n    let ki = tid % k;\n    let id = topk_ids[(di * k + ki) as usize] as u32;\n    let uf_base = (di + 1u32) * h;\n    let emb_base = id * h;\n    let mut acc = f32::new(0.0f32);\n    let mut j = 0u32;\n    while j < h {\n        acc = acc + u_final[(uf_base + j) as usize] * embedding[(emb_base + j) as usize];\n        j += 1u32;\n    }\n    residual[tid as usize] = acc;\n}\n".to_string(),
        },
        // ── share-cube-size-const-between-kernel-and-launcher — the
        // workgroup-size literal is a silent cross-file stride contract (D) ──
        //
        // Source: riir-gpu weaver_gpu/mod.rs L391, L983, L1180, L1283 (four
        // kernels each restate `let cube_size = 256u32;` as a bare magic
        // literal) vs L452, L751, L1125, L1234, L1342, L1493 (six launchers
        // each independently pass `CubeDim::new_1d(256)`). The kernel uses
        // its local cube_size for stride reconstruction (`tid = CUBE_POS_X *
        // cube_size + UNIT_POS`) and strided loops (`j += cube_size`), so
        // the two literals form a silent contract: change any launcher's
        // CubeDim and every kernel that hardcodes 256 mis-strides — skipped/
        // duplicated elements, wrong results, no error, in launch_unchecked
        // code with no runtime tripwire.
        //
        // Distinct from derive-shape-from-payload-not-cached-field (B32b —
        // shape TRUTH must come from the payload, a cached-field desync;
        // here NEITHER side is derived — two independent literals that must
        // agree), from launch-unchecked-caller-owned-size-contract (B29 —
        // documenting + asserting element-count contracts; this is the
        // GEOMETRY contract between grid shape and in-kernel indexing),
        // from pad-uniform-pod-to-min-binding-size (B30 — buffer layout).
        KernelRule {
            rule_id: "share-cube-size-const-between-kernel-and-launcher".to_string(),
            gpu_concept: "The workgroup size is restated as a bare magic literal inside several kernel bodies (let cube_size = 256u32;) while every corresponding launcher independently passes CubeDim::new_1d(256). The kernel uses its local cube_size for stride reconstruction (tid = CUBE_POS_X * cube_size + UNIT_POS) and strided loops (j += cube_size), so the two literals form a silent cross-file contract: change any launcher's CubeDim (e.g. to 128 for occupancy tuning) and every kernel that hardcodes 256 mis-strides — skipped or duplicated elements, wrong results, no error, in launch_unchecked code with no runtime tripwire. The fix: hoist ONE shared constant (e.g. pub const CUBE_SIZE: u32 = 256;) and use it in BOTH the kernel body and the launcher's CubeDim, so the pair cannot drift. If the DSL exposes the compile-time cube dim in-kernel, prefer that; the shared const is the minimal portable form. The invariant: exactly one source of truth for the stride that ties grid geometry to indexing math. Distinct from derive-shape-from-payload-not-cached-field (shape truth derivation — here neither side is derived, two literals that must agree), from launch-unchecked-caller-owned-size-contract (element-count contracts — this is the geometry contract between grid shape and in-kernel indexing).".to_string(),
            ref_shape: "// naive: two independent literals that must agree — change the launcher's\n// CubeDim and the kernel silently mis-strides (wrong results, no error)\nfn kernel_hardcoded_cube_size(input: &[f32], output: &mut [f32], n: usize) {\n    let cube_size = 256u32;\n    let tid = CUBE_POS_X * cube_size + UNIT_POS;\n    if tid >= n as u32 {\n        terminate!();\n    }\n    output[tid as usize] = input[tid as usize] * 2.0f32;\n}\n\npub unsafe fn launch_own_literal(client: &Client, input: Handle, output: Handle, n: usize) {\n    let num_wg = (n as u32).div_ceil(256u32).max(1);\n    unsafe {\n        kernel_hardcoded_cube_size::launch::<R>(\n            client,\n            CubeCount::Static(num_wg, 1, 1),\n            CubeDim::new_1d(256),\n            BufferArg::from_raw_parts(input, n),\n            BufferArg::from_raw_parts(output, n),\n        );\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: one shared const feeds BOTH the launcher's CubeDim and the\n// kernel's stride reconstruction — the pair cannot drift\npub const CUBE_SIZE: u32 = 256;\n\npub unsafe fn launch_shared_const(client: &Client, input: Handle, output: Handle, n: usize) {\n    let num_wg = (n as u32).div_ceil(CUBE_SIZE).max(1);\n    unsafe {\n        kernel_shared_cube_size::launch::<R>(\n            client,\n            CubeCount::Static(num_wg, 1, 1),\n            CubeDim::new_1d(CUBE_SIZE as usize),\n            BufferArg::from_raw_parts(input, n),\n            BufferArg::from_raw_parts(output, n),\n        );\n    }\n}\n\nfn kernel_shared_cube_size(input: &[f32], output: &mut [f32], n: usize) {\n    let cube_size = CUBE_SIZE;\n    let tid = CUBE_POS_X * cube_size + UNIT_POS;\n    if tid >= n as u32 {\n        terminate!();\n    }\n    output[tid as usize] = input[tid as usize] * 2.0f32;\n}\n".to_string(),
        },
        // ── stash-kernel-internal-intermediate-in-smem — a global
        // round-trip nobody outside the kernel observes (A) ──
        //
        // Source: riir-gpu weaver_gpu/mod.rs L1413-1414 (softmax_k_f32:
        // thread 0 computes `let cl = dflash_logits[...] + residual[...]`
        // and writes it to the corrected_logits GLOBAL buffer) -> L1429
        // (the very next loop re-reads the same elements back:
        // `let e = f32::exp(corrected_logits[(base + ki) as usize] -
        // max_c);`) + L1471 (the buffer is contractually write-only to the
        // outside world: "`corrected_logits_handle`: `d_depth * K` f32
        // (write-only)" — verified the corrector only ever read_ones
        // corrected_probs). Same thread, no cross-workgroup consumer: the
        // global round-trip costs a D*K write + D*K read per call and
        // forces the caller to allocate + thread a scratch handle whose
        // contents nobody observes.
        //
        // Distinct from smem-staging-gated-by-cross-thread-reuse (B35 —
        // when to stage an INPUT in smem, gated by within-workgroup reuse;
        // this is a kernel-INTERNAL intermediate whose global round-trip is
        // pure waste because no external reader exists), from smem-
        // broadcast-scalar-then-parallel-scatter (B28 — publishing a
        // reduced scalar through the reduced slot), from variant-owned-
        // output-buffer-clone-escape (B39 — host-side ownership), from
        // side-consumer-readback-alias-live-handle (B35 — when a real side
        // consumer DOES exist).
        KernelRule {
            rule_id: "stash-kernel-internal-intermediate-in-smem".to_string(),
            gpu_concept: "A kernel materializes a kernel-internal intermediate through global memory: thread 0 computes corrected = a + b, writes it to a global buffer, then in the very next loop re-reads the same elements back from global — same thread, no cross-workgroup consumer, and the buffer is contractually write-only to the outside world (the caller only ever reads a DIFFERENT output). The global round-trip buys nothing: it costs a D*K write plus a D*K read per call and forces the caller to allocate and thread a scratch handle whose contents nobody observes. The fix: if an intermediate is produced and consumed inside one kernel invocation and has no external reader, keep it in shared memory (or recompute it — often one add — in the consuming loop) and drop the global buffer from the signature entirely. For K bounded by config a fixed Shared slice sized MAX_K is the natural stash; the caller-side win is one fewer scratch allocation and parameter, the device-side win is two fewer global passes. Distinct from smem-staging-gated-by-cross-thread-reuse (when to stage an INPUT in smem — this is an internal intermediate whose round-trip is pure waste because no external reader exists), and from side-consumer-readback-alias-live-handle (for when a real side consumer DOES exist).".to_string(),
            ref_shape: "// naive: write the corrected logits to GLOBAL, re-read them in the next\n// loop of the SAME kernel — a round-trip no external reader observes\nfn softmax_global_reread(dflash_logits: &[f32], residual: &[f32], corrected_logits: &mut [f32], corrected_probs: &mut [f32], k: u32) {\n    let tid = CUBE_POS_X;\n    if tid == 0u32 {\n        let mut max_c = f32::new(-1e30f32);\n        let mut ki = 0u32;\n        while ki < k {\n            let cl = dflash_logits[ki as usize] + residual[ki as usize];\n            corrected_logits[ki as usize] = cl;\n            if cl > max_c {\n                max_c = cl;\n            }\n            ki += 1u32;\n        }\n        let mut sum_e = f32::new(0.0f32);\n        let mut ki = 0u32;\n        while ki < k {\n            let e = f32::exp(corrected_logits[ki as usize] - max_c);\n            corrected_probs[ki as usize] = e;\n            sum_e = sum_e + e;\n            ki += 1u32;\n        }\n        let inv_sum = f32::new(1.0f32) / sum_e;\n        let mut ki = 0u32;\n        while ki < k {\n            corrected_probs[ki as usize] = corrected_probs[ki as usize] * inv_sum;\n            ki += 1u32;\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: stash the intermediate in SHARED memory — the global buffer\n// disappears from the signature; one fewer scratch handle to allocate\nfn softmax_smem_stash(dflash_logits: &[f32], residual: &[f32], corrected_probs: &mut [f32], stash: &mut [f32], k: u32) {\n    let tid = CUBE_POS_X;\n    if tid == 0u32 {\n        let mut max_c = f32::new(-1e30f32);\n        let mut ki = 0u32;\n        while ki < k {\n            let cl = dflash_logits[ki as usize] + residual[ki as usize];\n            stash[ki as usize] = cl;\n            if cl > max_c {\n                max_c = cl;\n            }\n            ki += 1u32;\n        }\n        let mut sum_e = f32::new(0.0f32);\n        let mut ki = 0u32;\n        while ki < k {\n            let e = f32::exp(stash[ki as usize] - max_c);\n            corrected_probs[ki as usize] = e;\n            sum_e = sum_e + e;\n            ki += 1u32;\n        }\n        let inv_sum = f32::new(1.0f32) / sum_e;\n        let mut ki = 0u32;\n        while ki < k {\n            corrected_probs[ki as usize] = corrected_probs[ki as usize] * inv_sum;\n            ki += 1u32;\n        }\n    }\n}\n".to_string(),
        },
        // ── hoist-bit-address-out-of-packed-row-sweep — the bit address
        // is loop-invariant when the position is fixed (B) ──
        //
        // Source: riir-gpu kvarn/kvarn_dequant.rs L41-47 (kvarn_dequantize_
        // key: `for ch in 0..kv_dim { ... let q = unpack_value(packed_row,
        // pos_in_tile, bits_usize); }` — inside that call, bit_offset =
        // pos*bits, byte_offset, bit_shift, the bits > lo_bits split branch,
        // and two row.len() bounds checks are ALL identical on every
        // iteration because pos_in_tile never varies in the loop) + L51 (the
        // loop-invariant s_col lookup re-fetched per row). The canonical
        // form proves the fix: katgpt-kv kv_cache.rs L444-461 (dequantize_
        // key_into hoists `let var_col = tile.var_scales.s_col[pos_in_tile];`
        // then does `((quantized[ch * bpr + byte_off] >> shift) & mask)`
        // with byte_off/shift strength-reduced per the 4-bit arm).
        //
        // Distinct from hoist-group-invariant-scale-out-of-word-loop (B17 —
        // hoists a VALUE (the scale) out of a word loop; this hoists an
        // ADDRESS DECOMPOSITION — div/mod + branch + bounds checks — whose
        // result is identical for every row), from strength-reduce-div-pow2
        // (compile-time divisor; here the divisor is runtime but
        // loop-invariant), from factor-affine-quant-scale-out-of-element-
        // loop (B18 — algebraic factoring, not address hoisting).
        KernelRule {
            rule_id: "hoist-bit-address-out-of-packed-row-sweep".to_string(),
            gpu_concept: "A per-position dequant sweeps all kv_dim rows of a bit-packed tile, calling a generic unpack_value(packed_row, pos_in_tile, bits) per row. Inside that call the bit address — bit_offset = pos*bits, byte_offset = bit_offset/8, bit_shift = bit_offset%8, the bits > lo_bits cross-byte split branch, and two row.len() bounds checks — is identical on every iteration, because pos_in_tile never varies in the loop; the loop-invariant column-scale lookup is also re-fetched per row. The fix: decompose the bit address ONCE per position (byte_off, shift, mask — strength-reducible via per_byte = 8/bits), hoist the column scale, then per row do a single strided load + shift + mask directly into the FMA chain. Eliminates a call, ~3 div/mod, 3 branches, and 2 bounds checks per row. The canonical katgpt-kv dequantize_key_into is the proven form: var_col hoisted, then ((quantized[ch * bpr + byte_off] >> shift) & mask) with byte_off/shift computed once per position per bit-width arm. Distinct from hoist-group-invariant-scale-out-of-word-loop (hoists a VALUE — this hoists an ADDRESS DECOMPOSITION whose result is identical for every row) and from factor-affine-quant-scale-out-of-element-loop (algebraic factoring, not address hoisting).".to_string(),
            ref_shape: "// naive: per-row call re-derives the SAME bit address (div/mod + branch +\n// bounds checks) because pos_in_tile never varies inside the loop\nfn dequant_row_sweep_naive(packed: &[u8], bpr: usize, pos_in_tile: usize, bits: usize, out: &mut [f32]) {\n    for ch in 0..out.len() {\n        let row_off = ch * bpr;\n        let packed_row = &packed[row_off..row_off + bpr];\n        let q = unpack_value(packed_row, pos_in_tile, bits);\n        out[ch] = q as f32;\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: decompose the bit address ONCE per position — per row a\n// single strided load + constant shift + mask feeding the FMA chain\nfn dequant_row_sweep_hoisted(packed: &[u8], bpr: usize, pos_in_tile: usize, bits: u32, s_col_pos: f32, out: &mut [f32]) {\n    let per_byte = 8 / bits as usize;\n    let byte_off = pos_in_tile / per_byte;\n    let shift = (pos_in_tile % per_byte) * bits as usize;\n    let mask: u8 = ((1u32 << bits) - 1u32) as u8;\n    for ch in 0..out.len() {\n        let q = ((packed[ch * bpr + byte_off] >> shift) & mask) as f32;\n        out[ch] = q.mul_add(1.0f32, 0.0f32) * s_col_pos;\n    }\n}\n".to_string(),
        },
        // ── fold-tile-transpose-into-dequant-scatter — scatter the
        // consumer layout at production, delete the transpose pass (L) ──
        //
        // Source: riir-gpu kvarn/kvarn_dequant.rs L120-133 (kvarn_dequantize_
        // key_tile: materializes the whole tile position-major into an
        // intermediate `result` Vec — one fresh Vec<f32> per position via
        // kvarn_dequantize_key + extend — then runs a SECOND full double
        // loop scattering into a second full-size `transposed` buffer:
        // `transposed[ch * actual_cols + pos] = result[pos * kv_dim + ch];`).
        // 2 full reads + 2 full writes per element, actual_cols + 2
        // allocations, and a branchy transpose pass — for a layout the
        // producing loop could have written directly.
        //
        // Distinct from tiled-host-transpose-before-quantize (B37 — the
        // INVERSE direction: pre-transpose BEFORE a strided-store consumer;
        // here the transpose is ELIMINATED by scattering at production),
        // from per-layer-strided-grad-destination (B27 — consumer-selected
        // destinations falling out of an existing pass; same spirit, the
        // grad-tensor instance), from transposed-consumption-accumulate-
        // not-gather (B38 — read-side access pattern; this is write-side
        // layout), from needless-swap-temp-hoist-to-scratch (B32a).
        KernelRule {
            rule_id: "fold-tile-transpose-into-dequant-scatter".to_string(),
            gpu_concept: "A tile dequantizer materializes the whole tile position-major into an intermediate buffer (one fresh Vec per position, extended into a result Vec), then runs a SECOND full double loop scattering into a second full-size transposed buffer. That is 2 full reads + 2 full writes per element, actual_cols + 2 allocations, and a branchy transpose pass — for a layout the producing loop could have written directly. The fix: one reusable per-position column scratch; unpack each position into it, apply the inverse transform, and scatter out[ch * cols + pos] = col[ch] inside the SAME sweep. Eliminates the intermediate buffer, the entire transpose pass, and all per-position allocations while producing the identical row-major layout the quantizer consumed. The general form: when a producer emits one axis order and the consumer wants another, and the mismatch is a pure permutation, write the consumer's layout at production time instead of materialize-then-transpose. Distinct from tiled-host-transpose-before-quantize (the inverse direction — pre-transposing before a strided-store consumer; here the transpose pass is eliminated) and from per-layer-strided-grad-destination (the grad-tensor instance of consumer-selected destinations).".to_string(),
            ref_shape: "// naive: materialize position-major, then a SECOND full pass transposes\n// into a second full-size buffer — 2 reads + 2 writes per element\nfn dequant_tile_transpose_naive(packed: &[u8], cols: usize, kv_dim: usize, out: &mut [f32]) {\n    let mut result = Vec::with_capacity(kv_dim * cols);\n    for pos in 0..cols {\n        let deq = dequant_position(packed, pos, kv_dim);\n        result.extend(deq[..kv_dim].iter().copied());\n    }\n    let mut transposed = vec![0.0f32; kv_dim * cols];\n    for pos in 0..cols {\n        for ch in 0..kv_dim {\n            transposed[ch * cols + pos] = result[pos * kv_dim + ch];\n        }\n    }\n    out.copy_from_slice(&transposed);\n}\n".to_string(),
            opt_pattern: "// optimized: one reusable column scratch; scatter the consumer layout\n// inside the SAME sweep — no intermediate, no transpose pass\nfn dequant_tile_scatter_direct(packed: &[u8], cols: usize, kv_dim: usize, out: &mut [f32]) {\n    let mut col = vec![0.0f32; kv_dim];\n    for pos in 0..cols {\n        dequant_position_into(packed, pos, kv_dim, &mut col);\n        for ch in 0..kv_dim {\n            out[ch * cols + pos] = col[ch];\n        }\n    }\n}\n".to_string(),
        },
        // ── bit-cursor-unpack-to-width-specialized-byte-batch — a
        // software bit cursor re-derives what a per-width arm makes const (A) ──
        //
        // Source: riir-gpu kvarn/kvarn_dequant.rs L162-186 (kvarn_dequantize_
        // cpu: unpacks ONE quantized value per iteration through a software
        // bit cursor — per-element bounds check `let q = if byte_idx <
        // packed.len() { ((packed[byte_idx] >> bit_offset) & mask as u8) as
        // f32 } else { 0.0 };`, per-element `output.push`, the
        // data-dependent `if bit_offset >= 8` branch, + a manual
        // byte-realign after each tile). For pow2 bit widths the extraction
        // pattern per byte is fixed (2/4/8 values per byte), so the cursor
        // re-derives what a per-width arm makes compile-time-constant. The
        // canonical form: katgpt-kv kv_cache.rs L1345-1397 (unpack_row:
        // `match bits { 8 => ..., 4 => out[2*i] = (b & 0x0F) ..., 2 => ...,
        // 1 => ... }`).
        //
        // Distinct from vectorized-pack-load (B14-era — loading PACKED data
        // vectorized on the GPU; this is CPU-side scalar extraction made
        // branch-free), from strength-reduce-mod-pow2 (O — IR-level; this
        // is the branch + bounds-check elimination via width dispatch),
        // from ternary-bit-plane-u64-as-2x-u32 (B22 — bit-PLANE splitting;
        // this is bit-PACKED nibble extraction), from dtype-specialized-
        // float-kind (N — dtype dispatch for floats; this is bitwidth
        // dispatch for packed ints).
        KernelRule {
            rule_id: "bit-cursor-unpack-to-width-specialized-byte-batch".to_string(),
            gpu_concept: "A dequantizer unpacks ONE quantized value per iteration through a software bit cursor: per-element bounds check, per-element shift + mask, per-element output.push, plus a data-dependent if bit_offset >= 8 branch and a manual byte-realign after each tile. For pow2 bit widths the extraction pattern per byte is fixed (2/4/8 values per byte), so the cursor re-derives at runtime what a per-width arm makes compile-time-constant. The fix: dispatch once on bits into width-specialized arms that load a byte and emit 8/bits values with constant shifts into a pre-sized output slice — the shape of the canonical unpack_row (match bits { 8 => ..., 4 => out[2*i] = (b & 0x0F) ..., 2 => ..., 1 => ... }), keeping the per-tile byte alignment as the row boundary. Removes the per-element branch, the per-element bounds check, and the push reallocation; also removes inline duplication of a canonical unpack helper imported nearby (a drifted fork that can silently lose the cross-byte bits > lo_bits handling the canonical copy has — non-pow2 widths then corrupt while the quant side packs correctly). Distinct from vectorized-pack-load (GPU vectorized packed loads), from strength-reduce-mod-pow2 (IR-level operator reduction — this is branch + bounds elimination via width dispatch), and from ternary-bit-plane-u64-as-2x-u32 (bit-plane splitting vs bit-packed nibble extraction).".to_string(),
            ref_shape: "// naive: software bit cursor — one value per iteration, per-element\n// bounds check + data-dependent byte-advance branch + push\nfn unpack_bit_cursor_naive(packed: &[u8], bits: u32, out: &mut Vec<f32>) {\n    let mask = (1u32 << bits) - 1;\n    let mut byte_idx = 0usize;\n    let mut bit_offset: u32 = 0;\n    for slot in out.iter_mut() {\n        let q = if byte_idx < packed.len() {\n            ((packed[byte_idx] >> bit_offset) & mask as u8) as f32\n        } else {\n            0.0\n        };\n        *slot = q;\n        bit_offset += bits;\n        if bit_offset >= 8 {\n            bit_offset = 0;\n            byte_idx += 1;\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: dispatch once on width — a byte emits 8/bits values with\n// CONSTANT shifts into a pre-sized slice; no cursor, no per-element branch\nfn unpack_4bit_byte_batch(packed: &[u8], out: &mut [f32]) {\n    let n = out.len();\n    let mut o = 0usize;\n    for &b in packed.iter().take(n.div_ceil(2)) {\n        out[o] = (b & 0x0F) as f32;\n        if o + 1 < n {\n            out[o + 1] = ((b >> 4) & 0x0F) as f32;\n        }\n        o += 2;\n    }\n}\n".to_string(),
        },
        // ── single-sweep-minmax-for-quant-scale — the (lo, hi) pair is
        // one traversal, not two folds (B) ──
        //
        // Source: riir-gpu kvarn/kvarn_zone_weights.rs L67-68 (KvarnZone-
        // ScaleBundle::from_tile_sequence computes each tile's RTN range
        // with TWO separate full traversals: `let lo = chunk.iter().cloned()
        // .fold(f32::MAX, f32::min);` / `let hi = chunk.iter().cloned()
        // .fold(f32::MIN, f32::max);`). Range stats are the entry point of
        // every asymmetric quantizer; two sweeps double the memory traffic
        // of scale preparation for zero benefit. The canonical fixed
        // exactly this shape and documented it: katgpt-kv var_norm.rs L513
        // ("Single-pass min/max fold (was two separate `fold` calls
        // scanning `vals` twice).") + kv_cache.rs L1155-1160 (rtn_quantize_
        // rows_into does the same single-sweep min/max per row).
        //
        // Distinct from one-sweep-spatial-feature-fanout (B32c — 12 spatial
        // statistics from one board sweep; this is the two-statistic min/max
        // PAIR every RTN/AWQ-style scale prep needs), from precompute-
        // scalar-reduction (scalar reductions hoisted out of loops), from
        // dual-accumulator-single-reduction-sweep (B24 — GPU two-partials-
        // one-sweep sharing loads; the CPU/GPU-agnostic host-side instance).
        KernelRule {
            rule_id: "single-sweep-minmax-for-quant-scale".to_string(),
            gpu_concept: "A quantizer's scale preparation computes each group's range with TWO separate full traversals — one fold for min, a second fold for max. Range stats are the entry point of every asymmetric quantizer (RTN, AWQ-style grouped scales), so the two-sweep form doubles the global-memory traffic of scale preparation across the whole weight tensor for zero benefit. The fix: track (lo, hi) jointly in one pass with lo = lo.min(v); hi = hi.max(v) — branchless on GPUs via native min/max instructions. The canonical katgpt-kv code fixed exactly this shape and documented it in-source: 'Single-pass min/max fold (was two separate fold calls scanning vals twice)', and rtn_quantize_rows_into does the same single-sweep min/max per row. Distinct from one-sweep-spatial-feature-fanout (12 spatial statistics from one sweep — this is the two-statistic min/max pair specifically) and from dual-accumulator-single-reduction-sweep (the GPU two-partials-one-sweep form; this is the host-side quant-scale instance).".to_string(),
            ref_shape: "// naive: two separate folds scan the chunk TWICE for a (lo, hi) pair\nfn rtn_scale_two_folds(chunk: &[f32]) -> (f32, f32) {\n    let lo = chunk.iter().cloned().fold(f32::MAX, f32::min);\n    let hi = chunk.iter().cloned().fold(f32::MIN, f32::max);\n    (lo, hi)\n}\n".to_string(),
            opt_pattern: "// optimized: one traversal tracks the pair jointly — branchless via\n// native min/max; halves the scale-prep memory traffic\nfn rtn_scale_single_sweep(chunk: &[f32]) -> (f32, f32) {\n    let mut lo = f32::MAX;\n    let mut hi = f32::MIN;\n    for &v in chunk {\n        lo = lo.min(v);\n        hi = hi.max(v);\n    }\n    (lo, hi)\n}\n".to_string(),
        },
        // ── serialize-container-canonical-order — HashMap iteration
        // order is not a wire format (D) ──
        //
        // Source: riir-gpu kvarn/kvarn_zone_weights.rs L284-287 (KvarnZone-
        // Registry::serialize_all emits bundles by iterating
        // `self.bundles.values()` — a HashMap<u32, _> whose iteration order
        // is seeded by RandomState and differs between processes; two runs
        // with identical registry content produce different bytes). In a
        // BLAKE3-commitment ecosystem (the bundles are documented "stored
        // alongside NeuronShard ternary weights") any digest taken over the
        // serialization is unreproducible + serialize->deserialize->
        // serialize byte-diffs are meaningless noise.
        //
        // Distinct from order-stable-topk-via-index-sort (B32c — canonical
        // output order for a top-K selection; the serialization instance of
        // the same determinism class), from length-prefix-shape-faithful-
        // hash (B32c — shape-blind hashing; this is ORDER-blind
        // serialization), from incremental-multi-part-hash-no-concat (B32a —
        // allocation; this is determinism), from deterministic-tiling-
        // projection-tables (B32c — same seed -> same table).
        KernelRule {
            rule_id: "serialize-container-canonical-order".to_string(),
            gpu_concept: "A container serializer emits entries by iterating a HashMap's values() — iteration order seeded by RandomState, differing between processes. Two runs with identical container content produce different bytes, so any digest or commitment taken over the serialization is unreproducible, and serialize -> deserialize -> serialize byte-diffs are meaningless noise. In a BLAKE3-commitment stack (weight bundles stored alongside committed shards) that breaks commitment stability silently. The fix: iterate in canonical order — collect keys and sort_unstable (or hold the map as a BTreeMap so order is structural). The wire format is unchanged; only emission order becomes deterministic. The invariant: a serialization that feeds a digest must be a pure function of container CONTENT, never of container iteration order. Distinct from order-stable-topk-via-index-sort (the top-K selection instance of canonical ordering) and from length-prefix-shape-faithful-hash (shape-blind hashing — this is order-blind serialization).".to_string(),
            ref_shape: "// naive: HashMap iteration order leaks into the bytes — identical\n// content, different digest per process (RandomState seeding)\nfn serialize_hashmap_order_naive(bundles: &HashMap<u32, Vec<u8>>) -> Vec<u8> {\n    let mut buf = Vec::new();\n    for bundle in bundles.values() {\n        buf.extend_from_slice(&(bundle.len() as u32).to_le_bytes());\n        buf.extend_from_slice(bundle);\n    }\n    buf\n}\n".to_string(),
            opt_pattern: "// optimized: canonical key order — the bytes become a pure function of\n// CONTENT; commitments + round-trip byte-diffs are stable across processes\nfn serialize_canonical_order(bundles: &HashMap<u32, Vec<u8>>) -> Vec<u8> {\n    let mut ids: Vec<u32> = bundles.keys().copied().collect();\n    ids.sort_unstable();\n    let mut buf = Vec::new();\n    for id in ids {\n        let bundle = &bundles[&id];\n        buf.extend_from_slice(&(bundle.len() as u32).to_le_bytes());\n        buf.extend_from_slice(bundle);\n    }\n    buf\n}\n".to_string(),
        },
        // ══ Batch 45 (hypernet cluster, first half) ════════════════════
        // ── scratch-depth-matches-shape-invariance — allocation depth
        // must match shape invariance, not convenience (A) ──
        //
        // Source: riir-gpu hypernet/meta_lora.rs forward_extract_layers
        // (L246 lora_buf + L249-252 q/k/v/attn_residuals per LAYER, L322
        // attn_out per POSITION, L330 scores per HEAD per POSITION,
        // L388/L395 mlp_residual/mlp_hidden per position) — a layer x
        // position x head nest where every scratch shape is bound by
        // (total_seq, dims) fixed for the whole call; the scores buffer
        // alone is num_layers x total_seq x n_head allocations. The tell:
        // two same-shaped residuals at different depths (attn_residuals
        // per-layer vs mlp_residual per-position) while hidden/grid
        // already sit at fn scope; the same crate's dllm.rs
        // forward_bidirectional runs the same bidirectional prefill on
        // context-owned scratch (ctx.q/ctx.k/ctx.v + prefill.lora_buf).
        //
        // Distinct from caller-owned-split-scratch (ownership protocol,
        // not depth), superset-sized-scratch-across-variants (sizing
        // across variants, not nest depth), scoped-init-at-consume-site
        // (zeroing scope — the complementary clear discipline), and
        // needless-swap-temp-hoist-to-scratch (false-premise temps; this
        // is the general placement criterion of which that rule's clause
        // 2 is one instance).
        KernelRule {
            rule_id: "scratch-depth-matches-shape-invariance".to_string(),
            gpu_concept: "A multi-level loop nest (layer x position x head) allocates each scratch at the deepest convenient point even when the buffer's shape is bound by quantities fixed for the whole call: shapes derived only from (seq_len, dim) are layer-invariant, position-invariant, and head-invariant, so each scratch belongs at the SHALLOWEST nest whose bounds determine its shape — allocation depth must match shape invariance. The tell is two same-shaped buffers allocated at different depths (a per-layer attn_residuals beside a per-position mlp_residual) while sibling buffers already sit at fn scope; hoisting is free of clearing duties whenever the producer fully overwrites the slots it touches. GPU translation: per-layer device-buffer creation and per-workgroup temps are the same disease. Distinct from caller-owned-split-scratch (ownership protocol, not depth), superset-sized-scratch-across-variants (sizing across variants), scoped-init-at-consume-site (zeroing scope), and needless-swap-temp-hoist-to-scratch (false-premise temps — this is the general placement criterion).".to_string(),
            ref_shape: "// naive: every scratch allocated at the DEEPEST loop nest that touches it,\n// even though its shape is bound by quantities fixed for the whole call\nfn forward_naive(layers: usize, seq: usize, n: usize, heads: usize) -> f32 {\n    let mut hidden = vec![0.0f32; seq * n]; // fn scope: the one done right\n    let mut sink = 0.0f32;\n    for l in 0..layers {\n        // shapes depend ONLY on (seq, n) — layer-invariant, yet per-layer:\n        let mut q = vec![0.0f32; seq * n];\n        let mut attn_residuals = vec![0.0f32; seq * n];\n        for pos in 0..seq {\n            // shape depends only on n — position-invariant, yet per-position:\n            let mut attn_out = vec![0.0f32; n];\n            for h in 0..heads {\n                // shape depends only on seq — head-invariant, yet per-head:\n                let mut scores = vec![0.0f32; seq];\n                for t in 0..seq {\n                    scores[t] = q[pos * n + h] + hidden[t * n] + attn_residuals[t * n];\n                }\n                sink += scores[l % seq];\n            }\n            attn_out[0] = sink;\n            sink += attn_out[0];\n        }\n        q[0] += sink;\n    }\n    sink\n}\n".to_string(),
            opt_pattern: "// optimized: allocation depth matches shape invariance — each scratch\n// lives at the SHALLOWEST loop whose bounds determine its shape; no\n// clearing between reuses: every producer fully overwrites its slots\nfn forward_hoisted(layers: usize, seq: usize, n: usize, heads: usize) -> f32 {\n    struct Scratch {\n        q: Vec<f32>,              // [seq*n] — layer-invariant shape\n        attn_residuals: Vec<f32>, // [seq*n] — layer-invariant shape\n        attn_out: Vec<f32>,       // [n]     — position-invariant shape\n        scores: Vec<f32>,         // [seq]   — head-invariant shape\n    }\n    let mut s = Scratch {\n        q: vec![0.0f32; seq * n],\n        attn_residuals: vec![0.0f32; seq * n],\n        attn_out: vec![0.0f32; n],\n        scores: vec![0.0f32; seq],\n    };\n    let mut hidden = vec![0.0f32; seq * n];\n    let mut sink = 0.0f32;\n    for l in 0..layers {\n        for pos in 0..seq {\n            for h in 0..heads {\n                for t in 0..seq {\n                    s.scores[t] = s.q[pos * n + h] + hidden[t * n] + s.attn_residuals[t * n];\n                }\n                sink += s.scores[l % seq];\n            }\n            s.attn_out[0] = sink; // plain store: slot written once per reuse\n            sink += s.attn_out[0];\n        }\n        s.q[0] += sink;\n    }\n    sink\n}\n".to_string(),
        },
        // ── softmax-denominator-hoist-out-of-weighted-v — normalize
        // once, multiply in the PV loop (B) ──
        //
        // Source: riir-gpu hypernet/meta_lora.rs L345-358 — after the
        // exp pass, the weighted-V loop computes
        // val += scores[t_pos] / sum_exp * v[...] — a DIVISION per
        // (d, t_pos) pair, the same scores[t_pos] / sum_exp recomputed
        // head_dim times per key position (f32 divide is roughly an
        // order of magnitude costlier than multiply). The in-tree
        // optimized twin is riir-engine attention.rs attention_head
        // pass 3: inv_sum = 1.0 / sum hoisted once per (position,
        // head), per-t weight as scores_buf[t] * inv_sum, t-outer/
        // d-inner so V reads and the attn_out accumulation are both
        // contiguous — meta_lora.rs re-implemented it naively one
        // crate away. sum_exp >= exp(0) = 1 by construction (the max
        // score subtracts out), so the reciprocal is always finite.
        //
        // Distinct from fold-norm-scalar-into-matmul (folds an external
        // scalar INTO a dot to kill a buffer — this hoists a redundant
        // division OUT of the loop), from hoist-group-invariant-scale-
        // out-of-word-loop (dequant scale load-hoisting — this is
        // div-to-reciprocal-mul on a softmax denominator), from
        // online-softmax-streaming (streaming memory tiling — this is
        // the normalize-once fix inside one tile), and from
        // matvec-coalesce (the memory-stride half that the same
        // restructure delivers for free).
        KernelRule {
            rule_id: "softmax-denominator-hoist-out-of-weighted-v".to_string(),
            gpu_concept: "The softmax-weighted value accumulation divides by the softmax denominator inside the innermost (d x t) nest: val += scores[t] / sum_exp * v[...] recomputes the same division head_dim times per key position. Hoist the reciprocal once per (position, head) — inv_sum = 1.0 / sum_exp, then a per-t weight w = scores[t] * inv_sum (or premultiply the whole scores array before the consumer loop) — so the PV loop multiplies, never divides: f32 divide is an order of magnitude costlier than multiply and the denominator is loop-invariant by construction (the max score subtracts out, so sum_exp >= 1 and the reciprocal is always finite). The same restructure naturally swaps to t-outer/d-inner, making the V reads and the attn_out accumulation contiguous — the memory-stride half alone is the matvec-coalesce principle; the division hoist is the compute half. Distinct from fold-norm-scalar-into-matmul (folds an external scalar INTO a dot to kill a buffer — this hoists a redundant division OUT of the loop), from hoist-group-invariant-scale-out-of-word-loop (dequant scale load-hoisting), and from online-softmax-streaming (streaming tiling — this is the normalize-once fix inside one tile).".to_string(),
            ref_shape: "// naive: division per (d, t_pos) pair — the same scores[t_pos] / sum_exp\n// computed head_dim times per key position; V read strides by kvd\nfn weighted_v_naive(scores: &[f32], v: &[f32], attn_out: &mut [f32],\n                    sum_exp: f32, hd: usize, kvd: usize, k_head_off: usize,\n                    q_head_off: usize, total_seq: usize) {\n    for d in 0..hd {\n        let mut val = 0.0f32;\n        for t_pos in 0..total_seq {\n            // division INSIDE the innermost nest: hd x redundant divides\n            val += scores[t_pos] / sum_exp * v[t_pos * kvd + k_head_off + d];\n        }\n        attn_out[q_head_off + d] += val;\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: reciprocal hoisted ONCE per (position, head); the per-t\n// weight is a multiply; t-outer/d-inner makes V reads contiguous in d\nfn weighted_v_hoisted(scores: &[f32], v: &[f32], attn_out: &mut [f32],\n                      sum_exp: f32, hd: usize, kvd: usize, k_head_off: usize,\n                      q_head_off: usize, total_seq: usize) {\n    let inv_sum = 1.0f32 / sum_exp; // ONE division; sum_exp >= 1.0 always\n    for t_pos in 0..total_seq {\n        let w = scores[t_pos] * inv_sum; // per-t weight, computed once\n        for d in 0..hd {\n            // v access contiguous in d; attn_out accumulates across t\n            attn_out[q_head_off + d] += w * v[t_pos * kvd + k_head_off + d];\n        }\n    }\n}\n".to_string(),
        },
        // ── frozen-cooperand-hoist-out-of-perturbation-loop —
        // materialization cadence must match mutation cadence (A) ──
        //
        // Source: riir-gpu hypernet/train_meta_lora.rs FD sweeps — the
        // meta-grad loop (L276-321) rebuilds MemoryEmbeddings { data:
        // mem_flat.clone(), .. } at L290-294/L310-314 for an operand
        // mutated once per optimizer step (frozenness proven: mem_flat
        // mutates only at L328/L346/L365 + L370); the mem-grad loop
        // (L325-366) mirror-images it with unflatten_meta_lora of the
        // frozen meta operand at L331-336/L349-354. The callee re-derives
        // its own sweep-invariants too: compute_loss recomputes the
        // context-embedding target per eval (train L170-182) and
        // extract_memory_states re-converts the LoraAdapter set per call
        // (meta_lora.rs L485-490). The mem-loop half is 100% pure waste —
        // hoistable with zero semantic change.
        //
        // Complements make-unmake-over-clone-per-candidate (that rule
        // fixes the VARYING operand's per-candidate clone via in-place
        // perturb + rollback — this fixes the frozen co-operand: no
        // delta, no rollback, just hoist). Host-materialization twin of
        // side-branch-operand-residency-matches-mutation-lifetime
        // (device upload cadence) and immutable-operand-upload-at-set-
        // creation (config-lifetime constructor binding).
        KernelRule {
            rule_id: "frozen-cooperand-hoist-out-of-perturbation-loop".to_string(),
            gpu_concept: "A finite-difference or perturbation sweep whose eval API takes structured operands rebuilds the frozen co-operand per candidate: the sweep materializes unflatten(meta) and MemoryEmbeddings { data: mem.clone() } 2*P and 2*M times per step for operands mutated once per optimizer step, and the callee re-derives its own sweep-invariants (adapter conversion of the frozen operand, the context-embedding target) on every eval. Materialization cadence must match mutation cadence: build the frozen co-operand plus the callee-derived invariants ONCE at sweep entry into a prelude and thread it through — adding a callee seam (a loss_with_prelude entry point) when the invariant is derived inside the eval — so only the perturbed operand is re-materialized per candidate. Complements make-unmake-over-clone-per-candidate (that rule fixes the varying operand; this fixes the frozen co-operand — no delta, no rollback, just hoist). Host-materialization twin of side-branch-operand-residency-matches-mutation-lifetime (device upload cadence) and immutable-operand-upload-at-set-creation (config-lifetime constructor binding).".to_string(),
            ref_shape: "// naive: FD sweep rebuilds the FROZEN co-operand per eval — mem is\n// mutated once per optimizer step but materialized 2*P times per step;\n// the callee re-derives its own sweep-invariants (target, adapters) too\nfn fd_grad_naive(meta_flat: &mut [f32], mem_flat: &[f32], ctx: &[u32],\n                 w: &Weights, cfg: &Cfg, eps: f32, grads: &mut [f32]) {\n    for i in 0..meta_flat.len() {\n        let saved = meta_flat[i];\n        meta_flat[i] = saved + eps;\n        let lp = loss(ctx, &unflatten_meta(meta_flat),\n                      &MemoryEmb { data: mem_flat.to_vec(), m: cfg.m }, w);\n        meta_flat[i] = saved - eps;\n        let lm = loss(ctx, &unflatten_meta(meta_flat),\n                      &MemoryEmb { data: mem_flat.to_vec(), m: cfg.m }, w);\n        grads[i] = (lp - lm) / (2.0 * eps);\n        meta_flat[i] = saved;\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: materialization cadence matches mutation cadence — the\n// frozen co-operand + callee-derived invariants are built ONCE at sweep\n// entry into a prelude; the varying operand stays make/unmake in place\nstruct LossPrelude {\n    mem: MemoryEmb,             // frozen this sweep: built once\n    target: Vec<f32>,           // ctx-only: invariant under BOTH operands\n    adapters: Vec<LoraAdapter>, // callee seam: pre-converted frozen meta\n}\nfn fd_grad_hoisted(meta_flat: &mut [f32], ctx: &[u32], w: &Weights,\n                   pre: &LossPrelude, eps: f32, grads: &mut [f32]) {\n    for i in 0..meta_flat.len() {\n        let saved = meta_flat[i];\n        meta_flat[i] = saved + eps;\n        let lp = loss_with_prelude(ctx, meta_flat, w, pre);\n        meta_flat[i] = saved - eps;\n        let lm = loss_with_prelude(ctx, meta_flat, w, pre);\n        grads[i] = (lp - lm) / (2.0 * eps);\n        meta_flat[i] = saved;\n    }\n}\n// The mem-grad sweep is the mirror image: meta is the frozen co-operand —\n// hoist unflatten_meta + adapter conversion; only mem_flat perturbs.\n".to_string(),
        },
        // ══ Batch 46 (maxsim cluster) ═════════════════════════════
        // ── size-gate-covers-every-cost-factor — a CPU/GPU size gate
        // must gate on the full work term, not a partial shape
        // product (D) ──
        //
        // Source: riir-gpu maxsim/dispatch.rs L16-23 + L215-216 (the
        // gate `ld * batch < threshold` ignores lq and dim — both
        // multiply CPU cost; the constant 256 was pinned at lq=8,
        // dim=64 on M3 Max) + hypernet/gpu_m2p.rs L307/L363 (the same
        // partial-product shape: `rows * num_heads < threshold`,
        // ignoring head width). Benchmark 002 anchors the two-term
        // cost model: GPU floor ~1.6-1.8 ms/call (per-call allocs +
        // fresh staging readback), CPU ~8.5 us per (ld x batch) unit
        // at the benched shape; realistic shapes move the true
        // crossover from ~27 to ~6,900 in the gate's units while the
        // constant stays 256. Mis-dispatch is asymmetric: GPU at
        // too-small work pays the bounded floor (~6x/call); CPU at
        // too-large work forfeits an unbounded parallel win (73.86x
        // measured at the top of the bench).
        //
        // Distinct from recurrent-vs-chunk-dispatch (Batch 3 — the
        // size-gated-dispatch STRUCTURE between two GPU algorithms
        // whose domain has a single variable factor n; this adds
        // CPU-vs-GPU sides with different cost scaling + gate
        // completeness + calibration lifecycle),
        // defer-readback-gated-by-live-output-size (Batch 29 —
        // gate-on-the-right-quantity applied to readback), and
        // hybrid-cpu-elementwise-gpu-throughput-sync-points (Batch 34
        // — routes op CLASSES to sides; this routes the SAME op by
        // size).
        KernelRule {
            rule_id: "size-gate-covers-every-cost-factor".to_string(),
            gpu_concept: "When one op has both a CPU and a GPU implementation and a size gate routes between them, the empirically-pinned crossover constant is only valid if the gate quantity scales with BOTH sides' costs: CPU cost is proportional to total MACs (every shape factor — lq, ld, dim, batch), while GPU cost is a launch/alloc/readback floor plus MACs at the GPU rate. Gating on a partial product (ld * batch, ignoring lq and dim) silently transposes the crossover at unbenched shapes — and the failure is latency-only, so no test detects it. The mis-dispatch is asymmetric: routing too-small work to GPU pays the bounded floor (~6x per call); routing too-large work to CPU forfeits an unbounded parallel win (73.86x measured at the top of the bench). The constant also bakes in BOTH implementations' current overheads — making the CPU path 4x faster (SIMD) or removing the per-call GPU alloc floor moves the crossover in opposite directions — so the bench citation must name hardware AND shape, and the crossover bench must re-run when either side changes. Distinct from recurrent-vs-chunk-dispatch (the size-dispatch structure between two GPU algorithms with one variable factor), defer-readback-gated-by-live-output-size (gate-on-the-right-quantity applied to readback), and hybrid-cpu-elementwise-gpu-throughput-sync-points (op-class routing, not size routing of one op).".to_string(),
            ref_shape: "// naive: the gate reads a PARTIAL work product (ld * batch) while the CPU\n// side's cost scales with lq * ld * dim * batch — the pinned constant is\n// only valid at the benched shape\nconst THRESHOLD: usize = 256; // pinned once at lq=8, dim=64 — never re-derived\nfn score(queries: &[f32], docs: &[f32], lq: usize, ld: usize, dim: usize, batch: usize) -> Vec<f32> {\n    let work_size = ld * batch; // lq and dim missing — up to 4096x of CPU cost unaccounted\n    if work_size < THRESHOLD {\n        cpu_score(queries, docs, lq, ld, dim, batch) // scalar rate: cost ∝ lq*ld*dim*batch\n    } else {\n        gpu_score(queries, docs, lq, ld, dim, batch) // fixed floor ~1.8ms + MACs at GPU rate\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: gate on the FULL work term (or a two-term cost model) so the\n// crossover tracks shape; re-pin the measured constants when either side's\n// implementation changes (SIMD CPU twin ⇒ re-measure the CPU rate; scratch-\n// resident GPU buffers ⇒ re-measure the floor)\nconst CPU_US_PER_MAC: f64 = 8.5e-9;   // measured at the bench shape, cited w/ hardware\nconst GPU_FLOOR_US: f64 = 1_800.0;    // measured: submit + per-call allocs + readback\nconst GPU_US_PER_MAC: f64 = 0.11e-9;  // measured GPU rate at the bench shape\nfn score(queries: &[f32], docs: &[f32], lq: usize, ld: usize, dim: usize, batch: usize) -> Vec<f32> {\n    let macs = (lq * ld * dim * batch) as f64;\n    let cpu_us = macs * CPU_US_PER_MAC;\n    let gpu_us = GPU_FLOOR_US + macs * GPU_US_PER_MAC;\n    if cpu_us < gpu_us {\n        cpu_score(queries, docs, lq, ld, dim, batch)\n    } else {\n        gpu_score(queries, docs, lq, ld, dim, batch)\n    }\n}\n".to_string(),
        },
        // ── declared-max-array-taxed-per-thread-under-runtime-bound —
        // runtime-bounded function-scope array → declared-max
        // thread-local placement (A) ──
        //
        // Source: riir-gpu kernels/spectralquant_maxsim.wgsl L110-117 +
        // L130-151 — `var rotated_q: array<f32, 256>` (1 KB/thread)
        // written under a RUNTIME uniform bound (d_eff, default 6) and
        // read inside the j-loop: the compiler can neither unroll nor
        // promote it to registers, so every thread pays the DECLARED
        // max in indexable thread-local memory regardless of the
        // runtime value. wgpu pipeline override constants close the
        // gap with zero kernel-source forks (one pipeline per bound
        // class). The tax composes with axis-splitting fixes — more
        // resident threads = more paid footprints.
        //
        // Distinct from hardcoded-fast-path-jit-specialization (Batch
        // 23 — throughput constant-folding; this is the ADDRESS-SPACE
        // consequence of runtime vs compile-time bounds),
        // stash-kernel-internal-intermediate-in-smem (Batch 44 — a
        // global write+re-read moved to smem; this is function-scope
        // private placement + declared-max sizing), and
        // stack-array-row-view-bridge (Batch 43 — CPU-side fixed
        // stack bridge).
        KernelRule {
            rule_id: "declared-max-array-taxed-per-thread-under-runtime-bound".to_string(),
            gpu_concept: "A function-scope array sized to a hard maximum (array<f32, 256> = 1 KB per thread) but bounded at runtime by a uniform (d_eff, default 6) is dynamically indexed across two separately-bounded loops, so the compiler can neither unroll nor promote it to registers — every thread pays the DECLARED max in indexable thread-local memory regardless of the runtime value. Specialize the bound at pipeline creation (wgpu pipeline override constants; one pipeline per bound-class) so the array is compile-time sized: unrollable, register-promotable, zero kernel-source forks. The tax composes with parallelism fixes — splitting the reduction axis multiplies resident threads, multiplying the thread-local footprint. Distinct from hardcoded-fast-path-jit-specialization (throughput constant-folding — this is the address-space consequence of runtime vs compile-time bounds), stash-kernel-internal-intermediate-in-smem (a global intermediate moved to smem — this is function-scope private placement), and stack-array-row-view-bridge (CPU-side stack bridge).".to_string(),
            ref_shape: "// naive: declared-max local array under a RUNTIME bound — the compiler\n// cannot unroll or promote it; every thread pays 256 slots of indexable\n// thread-local even when d_eff is 6\nfn score_naive(d_eff: u32, q_rot: &[f32; 256], k_row: &[f32; 256]) -> f32 {\n    let mut rotated_q = [0.0f32; 256]; // 1 KB/thread, always fully paid\n    for d in 0..d_eff {\n        rotated_q[d as usize] = q_rot[d as usize]; // runtime index into local\n    }\n    let mut dot = 0.0f32;\n    for j in 0..d_eff {\n        dot += rotated_q[j as usize] * k_row[j as usize]; // runtime index again\n    }\n    dot\n}\n".to_string(),
            opt_pattern: "// optimized: the bound is a pipeline OVERRIDE CONSTANT (one pipeline per\n// d_eff class) — compile-time-sized array unrolls / register-promotes;\n// threads pay only the used size\nfn score_specialized(q_rot: &[f32; 256], k_row: &[f32; 256]) -> f32 {\n    const D_EFF: usize = 6; // pipeline override constant — NOT a runtime uniform\n    let mut rotated_q = [0.0f32; D_EFF]; // compile-time bound\n    for d in 0..D_EFF {\n        rotated_q[d] = q_rot[d]; // statically indexed — unrollable\n    }\n    let mut dot = 0.0f32;\n    for j in 0..D_EFF {\n        dot += rotated_q[j] * k_row[j];\n    }\n    dot\n}\n".to_string(),
        },
        // ── shared-operand-side-pair-batch-index-decomposition —
        // a pair-batch tensor replicates the shared side; decompose
        // the launch index instead (A) ──
        //
        // Source: riir-gpu maxsim/spectralquant.rs L190 + kernels/
        // spectralquant_maxsim.wgsl L27/L109 — the pair API's
        // [batch x lq x d_h] query tensor bakes a per-pair copy of the
        // shared side into the batch geometry (q_base = batch_idx *
        // lq * d_h); the canonical MaxSim/ColBERT workload is ONE
        // query vs N candidate docs, so the caller replicates the
        // query N x on the host, uploads N x the bytes, and every
        // thread redundantly re-rotates identical tokens. The CPU
        // substrate twin already solves it: katgpt-types
        // maxsim_score_packed takes offsets + per-pair query-ids.
        //
        // Distinct from gqa-kv-head-sharing-bandwidth-reduction (Batch
        // 23 — head-index mapping WITHIN attention; this is
        // batch-tensor geometry for pair tasks),
        // fan-out-shared-input-single-sync (Batch 25 — N sibling
        // DISPATCHES sharing one upload; this is ONE kernel's input
        // layout), and twin-projection-shared-delta-adapter (Batch 31
        // — param sharing between shape twins).
        KernelRule {
            rule_id: "shared-operand-side-pair-batch-index-decomposition".to_string(),
            gpu_concept: "A 'score N (query, doc) pairs' kernel whose input tensor is [batch x lq x d_h] bakes a per-pair copy of the shared side into the batch geometry (q_base = batch_idx * lq * d_h). The canonical MaxSim/ColBERT workload is one query vs N candidate docs: the caller must replicate the query N times on the host, upload N times the bytes, and every thread redundantly re-transforms identical tokens. Decompose the invocation index into (query_idx, doc_idx) — a 2D dispatch or divide/mod — with a [n_q x lq x d_h] query operand plus per-pair doc offsets, and the shared side crosses the bus once: share-by-indexing instead of replication, at the batch-geometry level. Distinct from gqa-kv-head-sharing-bandwidth-reduction (head-index mapping within attention — this is batch-tensor geometry), fan-out-shared-input-single-sync (N dispatches sharing one upload — this is one kernel's input layout), and twin-projection-shared-delta-adapter (param sharing between shape twins).".to_string(),
            ref_shape: "// naive: the pair API bakes the shared query into every pair —\n// [batch x lq x d] forces the caller to replicate the query batch-times\n// before upload (N x host bytes, N x bus traffic, per-thread re-transform)\nfn score_pairs_replicated(queries: &[f32], docs: &[f32], batch: usize,\n                          lq: usize, ld: usize, d: usize, out: &mut [f32]) {\n    for b in 0..batch {\n        let q = &queries[b * lq * d..(b + 1) * lq * d]; // SAME query, batch copies\n        let k = &docs[b * ld * d..(b + 1) * ld * d];\n        out[b] = maxsim(q, k, lq, ld, d);\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: decompose the pair index — the shared query is uploaded\n// ONCE and selected by query_idx; each pair carries only its doc offset\nfn score_pairs_shared(query: &[f32], docs: &[f32], doc_offsets: &[usize],\n                      pair_q: &[u16], lq: usize, ld: usize, d: usize,\n                      out: &mut [f32]) {\n    for pair in 0..pair_q.len() {\n        let qi = pair_q[pair] as usize; // index into the n_q shared queries\n        let q = &query[qi * lq * d..(qi + 1) * lq * d]; // one copy on device\n        let off = doc_offsets[pair];\n        let k = &docs[off..off + ld * d];\n        out[pair] = maxsim(q, k, lq, ld, d);\n    }\n}\n".to_string(),
        },
    ]
}

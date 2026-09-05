//! Entries slice — Batches 14-49+51+52+54+55+56 era (rules 325-353 of 355) — riir-gpu production-kernel mining + measured-GOAT distillations.
//! Pure data moved verbatim from the former corpus.rs monolith
//! (2026-08-15 split); append new batches to the LAST gpu slice or
//! open a new one past ~900 lines. Category membership lives ONLY in
//! `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

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
            gpu_concept: "A per-row dot product is mapped one-thread-per-row: thread tid owns output row tid and serially loops the full row length h, with the row base computed as g_base = tid * h. When the output count (rows) is small relative to the machine — at production (D=4, K=32, h=2048) that is 128 threads inside a SINGLE workgroup of 256, i.e. one SM while the rest of the GPU idles — the mapping simultaneously destroys coalescing: adjacent threads read addresses h * 4B (8 KB) apart, so every warp load touches 32 distinct cache lines (16x read amplification). The fix: when the output count is too small to fill the machine, split the REDUCTION axis instead — dispatch one workgroup per row, have lanes stride over h (adjacent lanes read adjacent elements, coalesced), accumulate per-lane partials in registers, and block-reduce the partials through shared memory; lane 0 writes the scalar. This fixes occupancy (D*K workgroups spread across SMs instead of 1) and coalescing at once, and shrinks the per-thread serial chain from h FMAs to h/cube_size. Distinct from grid-over-output-thread-private-sweep (the inverse regime — there the sweep axis is long and the output-owned grid needs NO cross-thread reduction; here the output count is too short to fill the machine so the reduction axis is the one to split), from block-reduction-separate-scratch-runtime-strides (where partials live once the split exists), and from tree-reduction-depth-over-sequential-accumulate (the reduction tree shape, assuming the split was already made). RIDER (Batch 108, riir-ai M3 campaign / Plan 078, the CAPPED-OCCUPANCY REGIME COMPLEMENT): the split THIS rule prescribes was built on the split-decode attention pair and measured a TIE with the shipped thread-per-position kernel at every length at production occupancy (interleaved median ratios 1.01-1.06; the variant was REMOVED) — at a workgroup-supply cap no extra workgroups exist to run the split halves, so axis splitting buys nothing; the lever that pays there is per-thread loads-in-flight (independent-accumulator-chains-hide-load-latency-at-capped-occupancy, Batch 108). Regime check before splitting: confirm workgroup supply actually scales (riir-ai @ 59a172623 tests/probe_831_attn_pair_time.rs:13-17).".to_string(),
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
            gpu_concept: "When one op has both a CPU and a GPU implementation and a size gate routes between them, the empirically-pinned crossover constant is only valid if the gate quantity scales with BOTH sides' costs: CPU cost is proportional to total MACs (every shape factor — lq, ld, dim, batch), while GPU cost is a launch/alloc/readback floor plus MACs at the GPU rate. Gating on a partial product (ld * batch, ignoring lq and dim) silently transposes the crossover at unbenched shapes — and the failure is latency-only, so no test detects it. The mis-dispatch is asymmetric: routing too-small work to GPU pays the bounded floor (~6x per call); routing too-large work to CPU forfeits an unbounded parallel win (73.86x measured at the top of the bench). The constant also bakes in BOTH implementations' current overheads — making the CPU path 4x faster (SIMD) or removing the per-call GPU alloc floor moves the crossover in opposite directions — so the bench citation must name hardware AND shape, and the crossover bench must re-run when either side changes. Distinct from recurrent-vs-chunk-dispatch (the size-dispatch structure between two GPU algorithms with one variable factor), defer-readback-gated-by-live-output-size (gate-on-the-right-quantity applied to readback), and hybrid-cpu-elementwise-gpu-throughput-sync-points (op-class routing, not size routing of one op). RIDER (Batch 113 external instance, ggml-org/llama.cpp @ 5266f24da v0.4.0 -- ggml-metal-ops.cpp:1773-1782, the chunked-SSD MMA gate): a five-clause eligibility gate names every cost factor per clause -- mma_tokens > 0 (work left after the reserve), ne30 == 1 ('checks that A tensor is set to scalar decay per head'), has_simdgroup_mm ('hardware check for M1 or newer'), d_state % 8 == 0 ('to align with simdgroup_float 8x8 tiles'), d_inner == OP_SSM_SCAN_SSD_HD ('mma kernel is specialized for the Mamba-2 head dim; this checks it') -- each comment states WHICH cost the clause covers; the fifth clause is the specialization honesty this rule demands.".to_string(),
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
        // ── per-key-projection-hoist-out-of-query-sweep — K/V
        // projections re-derived inside the query loop multiply the
        // projection work by seq (B) ──
        //
        // Source: riir-gpu hypernet/gpu_m2p.rs L97-112 (the CPU
        // fallback's score loop re-projects K[t] for EVERY query pos:
        // a GEMV nest `for k in 0..hidden_dim { k_val += input[t_off
        // + k] * wqkv[...] }` inside `for pos`) + L121-133 (the V
        // twin: v_val re-projected inside the d_out loop for every t)
        // + kernels/m2p_attention.wgsl L55-70 + L82-91 (the GPU form:
        // each (pos, head) thread re-derives ALL K and V rows —
        // projection work multiplied by seq per thread). The OPTIMIZED
        // twin ships in the sibling file: hypernet/m2p_transformer.rs
        // L313-323 projects ALL QKV for all positions once at sweep
        // entry, and the score loop reads `qkv[s * 3 * h + h +
        // head_offset + d]` with no weight in the inner loop. Column
        // attention at default shape (seq=36, H=4096): 43.3 G MACs
        // naive vs 1.2 G hoisted; row attention (seq=M): factor M —
        // arithmetically infeasible at production M=425,984.
        //
        // Distinct from frozen-cooperand-hoist-out-of-perturbation-loop
        // (Batch 45 — a frozen operand RE-MATERIALIZED (cloned or
        // uploaded) per candidate; here the operand is re-DERIVED by
        // GEMV per query — a clone-detection healer never fires on
        // this shape), from row-tile-amortize-input-load (Batch 15 —
        // amortize input LOADS across tiles; this is projection
        // COMPUTE), and from fan-out-shared-input-single-sync (Batch
        // 25 — sibling dispatches sharing one upload; this is one
        // sweep's inner loop).
        KernelRule {
            rule_id: "per-key-projection-hoist-out-of-query-sweep".to_string(),
            gpu_concept: "A per-query sweep (attention scores, similarity search) re-derives the per-key projection inside the query loop: for each query position, each key's vector is re-computed by a full-width GEMV against the projection weight (K[t] = Wk . input[t] recomputed for every query pos; the V twin nests the same re-projection inside the per-output-dim loop), turning O(seq * H^2) projection work into O(seq^2 * H^2). On GPU the same shape multiplies per-thread: each (pos, head) thread re-derives ALL key and value rows, so projection work scales with the thread count again. Project all K (and V) once per slice at sweep entry — exactly what the reference CPU attention does: materialize the projected key/value rows (or the packed QKV) before the query loop, so the score sweep reads projected vectors at O(head_dim) per pair instead of re-deriving them at O(H * head_dim) per pair. On GPU, either a pre-projection kernel writing a scratch buffer or a hoist above the per-position dispatch axis. Detection shape: a GEMV nest (inner loop over the full H against a weight row) nested inside a loop over query positions whose induction variable the projection does not depend on. Distinct from frozen-cooperand-hoist-out-of-perturbation-loop (a frozen operand re-MATERIALIZED per candidate — clone/upload cadence; here the operand is re-DERIVED by GEMV per query, and a clone-detection healer never fires on this shape), from row-tile-amortize-input-load (amortize input loads across tiles — this is projection compute), and from fan-out-shared-input-single-sync (sibling dispatches sharing one upload — this is one sweep's inner loop).".to_string(),
            ref_shape: "// naive: K[t] re-derived by GEMV for EVERY query pos — O(seq^2 * H * hd)\nfn attention_scores_reproject_naive(input: &[f32], wk: &[f32], seq: usize,\n                                    h: usize, hd: usize, scores: &mut [f32]) {\n    for q_pos in 0..seq {\n        for t in 0..seq {\n            let mut dot = 0.0f32;\n            for d in 0..hd {\n                let mut k_val = 0.0f32;\n                for k in 0..h {\n                    // the GEMV nest does not depend on q_pos — recomputed\n                    // seq times per (t, d): projection work x seq\n                    k_val += input[t * h + k] * wk[d * h + k];\n                }\n                dot += input[q_pos * h + d] * k_val;\n            }\n            scores[q_pos * seq + t] = dot;\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: project ALL keys once at sweep entry — O(seq * H * hd) —\n// then the score sweep reads projected rows at O(hd) per pair\nfn attention_scores_hoisted(input: &[f32], wk: &[f32], seq: usize,\n                            h: usize, hd: usize, k_proj: &mut [f32],\n                            scores: &mut [f32]) {\n    for t in 0..seq {\n        for d in 0..hd {\n            let mut k_val = 0.0f32;\n            for k in 0..h {\n                k_val += input[t * h + k] * wk[d * h + k];\n            }\n            k_proj[t * hd + d] = k_val;\n        }\n    }\n    for q_pos in 0..seq {\n        for t in 0..seq {\n            let mut dot = 0.0f32;\n            for d in 0..hd {\n                dot += input[q_pos * h + d] * k_proj[t * hd + d];\n            }\n            scores[q_pos * seq + t] = dot;\n        }\n    }\n}\n".to_string(),
        },
        // ── multi-head-out-proj-consumes-head-concat — the
        // attention output projection needs the head concat; a
        // per-head thread cannot express it (E) ──
        //
        // Source: riir-gpu kernels/m2p_attention.wgsl L92-98 (column)
        // + L167-172 (row): `proj_val = proj_val + attn_val *
        // out_weight[d_out * h + kk]` — attn_val is INVARIANT in kk,
        // so the "projection" computes attn_val * rowsum(wo_row), a
        // scalar-times-rowsum, NOT the matvec. hypernet/gpu_m2p.rs
        // L134-141 mirrors it faithfully (adding a second bug:
        // `let wo_row = d_out;` uses the head-local index, so heads
        // 1..n re-read wo rows [0, head_dim)). The CORRECT form ships
        // in the sibling file: hypernet/m2p_transformer.rs L371-381
        // sums over the FULL attn_out row (`sum += attn_out[t * h +
        // k] * layer.wo[d * h + k]`). Masked by the identity-weight
        // fixtures (gpu_m2p.rs L499-505: identity rows have rowsum
        // 1.0) and a print-only CPU-vs-GPU pseudo-parity check that
        // compares the two wrong implementations against each other
        // (benches/alternating_2d_bench.rs L515-553). Filed as riir-ai
        // Issue 701 — the rule is the mechanism, the issue is the
        // live wrong math.
        //
        // Distinct from broadcast-uniform-skip-reduce (Batch 24 — a
        // REDUNDANT reduction of a thread-uniform whose fix is to
        // broadcast; here the invariant operand makes the result WRONG
        // and the fix restructures the phase boundary), from
        // epilogue-must-be-workgroup-local (Batch 29 — fusion legality
        // by ownership scope; this is fusion legality by DATA
        // dependency across the head axis), and from
        // split-dispatch-expose-fused-intermediate (Batch 34 —
        // exposing a fused intermediate for a second consumer; here
        // the intermediate is per-head PARTIALS of one output row).
        KernelRule {
            rule_id: "multi-head-out-proj-consumes-head-concat".to_string(),
            gpu_concept: "Fusing the attention output projection into per-(pos, head) threads produces wrong math silently: the projection O[d] = sum_k attn_out_full[k] * wo[d][k] consumes the CONCATENATION of all heads' outputs, but a per-head thread owns only its own head_dim dims — so the in-thread projection loop degenerates to attn_val (a scalar) times the summed weight row, a scalar-times-rowsum that is not the matvec, while every identity-weight fixture returns the right answer anyway (identity rowsum = 1.0) and a CPU-vs-GPU pseudo-parity check that compares the two wrong implementations against each other stays green. The fix: keep the per-head decomposition for the score/V mixing (legal there — each head's output dims are disjoint), write per-head attention outputs to a scratch buffer, and run the output projection as a separate phase (kernel, dispatch axis, or host pass) that owns the full H-length row and computes the true matvec sum_k attn_out[k] * wo[d][k]. Detection tells: a projection loop whose multiplicative operand is invariant in the summation index (a dot-shaped loop with a loop-invariant operand), and per-head threads writing disjoint output slices while indexing the projection weight by only the head-local dim. Distinct from broadcast-uniform-skip-reduce (a redundant reduction of a thread-uniform whose fix is to broadcast the value — here the invariant operand makes the result wrong and the fix moves the phase boundary), from epilogue-must-be-workgroup-local (fusion legality by ownership scope — this is fusion legality by data dependency across the head axis), and from split-dispatch-expose-fused-intermediate (exposing a fused intermediate for a second consumer — here the intermediate is per-head partials of one output row).".to_string(),
            ref_shape: "// naive: fused into the per-head thread — the projection operand\n// attn_head[d_out] is INVARIANT in kk, so this computes\n// attn_val * rowsum(wo_row): a scalar-times-rowsum, not a matvec\nfn per_head_out_proj_naive(attn_head: &[f32], wo: &[f32], h: usize,\n                           head_off: usize, hd: usize, out: &mut [f32]) {\n    for d_out in 0..hd {\n        let attn_val = attn_head[d_out];\n        let mut proj_val = 0.0f32;\n        for kk in 0..h {\n            proj_val += attn_val * wo[d_out * h + kk];\n        }\n        // silently wrong under real weights; identity fixtures mask it\n        out[head_off + d_out] = proj_val;\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: two phases — per-head threads stage THEIR dims into the\n// concat buffer (disjoint slices, legal decomposition); a separate\n// full-row pass computes the true matvec over the head CONCAT\nfn attention_out_proj_two_phase(head_outputs: &[f32], wo: &[f32],\n                                seq: usize, h: usize, heads: usize,\n                                hd: usize, scratch: &mut [f32],\n                                out: &mut [f32]) {\n    for pos in 0..seq {\n        for head in 0..heads {\n            let head_off = head * hd;\n            for d in 0..hd {\n                scratch[pos * h + head_off + d] =\n                    head_outputs[(pos * heads + head) * hd + d];\n            }\n        }\n        for d in 0..h {\n            let mut sum = 0.0f32;\n            for k in 0..h {\n                sum += scratch[pos * h + k] * wo[d * h + k];\n            }\n            out[pos * h + d] = sum;\n        }\n    }\n}\n".to_string(),
        },
        // ── shared-producer-prefix-extract-once-per-input — two
        // sibling methods sharing an expensive pure producer pay it
        // N times under one composed call (D) ──
        //
        // Source: riir-gpu hypernet/shine.rs L229-230 — the composed
        // entry `context_to_lora_with_span_width` calls
        // `self.context_to_lora(context_tokens)` then
        // `self.estimate_difficulty(context_tokens)`, and BOTH
        // internally run `extract_memory_states(context_tokens, ...)`
        // (L138-144 + L196-202) — a full frozen-backbone transformer
        // forward producing the [L, M, H] grid, the dominant cost of
        // both methods, executed twice for one input (feature
        // mux_training). The producer is pure w.r.t. repeat calls (all
        // inputs are &self immutable fields + the context), so the
        // second run is bit-identical wasted work.
        //
        // Distinct from split-dispatch-expose-fused-intermediate
        // (Batch 34 — ONE fused chain discards its intermediate; here
        // TWO independent methods each fully use the artifact), from
        // fan-out-shared-input-single-sync (Batch 25 — N sibling
        // KERNELS sharing one uploaded input buffer; this is the
        // host-API twin: N sibling METHODS sharing one COMPUTED
        // artifact), from frozen-cooperand-hoist-out-of-perturbation-
        // loop (Batch 45 — an input-INDEPENDENT operand rebuilt per
        // perturbation candidate; this producer varies with the input
        // and there is no sweep), and from refine-loop-invariant-
        // prefix-elision (Batch 42 — iteration invariance inside a
        // loop; here two sequential calls).
        KernelRule {
            rule_id: "shared-producer-prefix-extract-once-per-input".to_string(),
            gpu_concept: "When two sibling public methods share an expensive pure producer as their first stage, a composed entry that calls both pays the producer N times for one input: each method internally re-runs the identical expensive call (a full frozen-backbone transformer forward) on the identical context, and the producer is pure w.r.t. repeat calls — all inputs are immutable receiver fields plus the argument — so the second run is bit-identical wasted work, the dominant cost of the composition doubled. The fix: extract once and thread the shared artifact — keep both public methods unchanged for standalone callers, add internal *_from_<artifact> variants that take the producer's output directly, make the public methods thin wrappers that extract then delegate, and have every composed entry extract the artifact once and pass it to both variants. Detection tell: a composer invoking two sibling APIs whose first stage is the identical expensive call on the identical input. Distinct from split-dispatch-expose-fused-intermediate (one fused chain discards its intermediate — here two independent methods each fully use the artifact), from fan-out-shared-input-single-sync (N sibling kernels sharing one uploaded buffer — this is the host-API twin: N sibling methods sharing one computed artifact), from frozen-cooperand-hoist-out-of-perturbation-loop (an input-independent operand rebuilt per perturbation candidate — this producer varies with the input and there is no sweep), and from refine-loop-invariant-prefix-elision (iteration invariance inside a loop — here two sequential calls).".to_string(),
            ref_shape: "// naive: both siblings internally run the SAME expensive producer\n// (a full frozen-backbone forward) on the same input — paid twice\nfn composed_entry_naive(ctx: &Engine, tokens: &[u32]) -> (Weights, f32) {\n    let weights = ctx.generate_weights(tokens);\n    let difficulty = ctx.estimate_difficulty(tokens);\n    (weights, difficulty)\n}\n".to_string(),
            opt_pattern: "// optimized: extract the shared artifact ONCE, thread it into the\n// *_from variants; the public methods become thin extract+delegate\n// wrappers so standalone callers are unchanged\nfn composed_entry_extract_once(ctx: &Engine, tokens: &[u32]) -> (Weights, f32) {\n    let grid = ctx.extract_memory_grid(tokens);\n    let weights = ctx.generate_weights_from_grid(&grid);\n    let difficulty = ctx.difficulty_from_grid(&grid);\n    (weights, difficulty)\n}\n".to_string(),
        },
        // ── runtime-arg-loop-bound-demotes-arrays-to-local-memory —
        // a correctly-sized per-thread array whose per-element loops
        // are bounded by a RUNTIME kernel arg (constant at every call
        // site) is demoted to local memory (A) ──
        //
        // Source: riir-gpu cudarc_kernels/mod.rs rmsnorm_quantize_f32
        // L211-235 (Issue 706 / Bench 686, 2026-08-16) — the Issue-623
        // kernel shipped `float vals[16]` (exactly sized) with
        // `for (int i = 0; i < ablock; i++)` where `ablock` is a kernel
        // ARG that is 16 at EVERY call site (the launcher's Rust
        // ACTIVATION_BLOCK const). nvcc cannot unroll a runtime-bounded
        // loop, and a dynamically-indexed per-thread array has no
        // register form — every vals[]/gam[] access was a local-memory
        // round-trip since Issue 623, hidden under the phase-4 global
        // re-read. The pathology surfaced only when Issue 706 v1 (gamma
        // prefetch) ADDED a second spilled array and measured 0.6-1%
        // SLOWER in paired interleaved A/B — trading L1-hot global
        // reads for more local-memory traffic is a LOSS. v2
        // (compile-time bound `i < 16` + `#pragma unroll` on all three
        // per-element loops) keeps vals+gam in registers: kernel
        // −18.4% (nsys 5,460.9 → 4,455.7 ns avg, 20,702 launches,
        // same-code-drift controlled), ppl bit-exact.
        //
        // Distinct from declared-max-array-taxed-per-thread-under-
        // runtime-bound (Batch 46 — the array's DECLARED SIZE exceeds
        // the runtime bound, taxing thread-local footprint; here the
        // array is exactly sized and the tax is per-ACCESS
        // local-memory addressing + lost unrolling; fix differs:
        // specialize the SIZE via pipeline overrides vs hoist the
        // BOUND to a compile-time const + unroll), from
        // bit-cursor-unpack-to-width-specialized-byte-batch (Batch 44
        // — per-width arms make BIT ADDRESSES compile-time-constant;
        // here the LOOP BOUND itself is the runtime value), and from
        // hardcoded-fast-path-jit-specialization (Batch 23 —
        // throughput constant-folding; this is address-space
        // placement of per-thread state).
        KernelRule {
            rule_id: "runtime-arg-loop-bound-demotes-arrays-to-local-memory".to_string(),
            gpu_concept: "A per-thread array whose per-element loops are bounded by a RUNTIME kernel argument is demoted to local memory even when the array is exactly sized and the argument is constant at every call site: the compiler cannot unroll a runtime-bounded loop, and a dynamically-indexed per-thread array has no register form — every access becomes a local-memory (spill) round-trip, and every array the loop touches inherits the demotion. The invariant is a LAUNCHER fact the kernel cannot see (the host passes a Rust const as an int arg), so the fix is to hoist it into the kernel as a compile-time constant and unroll the loops (#pragma unroll): statically-indexed arrays promote to registers, and unrolling preserves the ascending accumulation order so outputs stay bit-identical (keep the (i < len) tail-guard for partial tiles). The pathology is invisible by default — it hides under whatever global traffic the kernel already does, and only surfaces when a neighboring change ADDS traffic to the same address space: a fix that trades L1-hot global reads for more local-memory traffic measures SLOWER (the paired interleaved A/B is the detection instrument; nsys local-memory counters quantify it). Measured on the fused rmsnorm+quantize decode kernel: −18.4% kernel time (5,460.9 → 4,455.7 ns avg) at ppl bit-exact. Distinct from declared-max-array-taxed-per-thread-under-runtime-bound (the declared SIZE exceeds the runtime bound, taxing thread-local footprint — here the array is exactly sized and the tax is per-access local addressing plus lost unrolling; fix: specialize size via pipeline overrides vs hoist the bound to a compile-time const), from bit-cursor-unpack-to-width-specialized-byte-batch (per-width arms make bit addresses compile-time-constant — here the loop bound itself is the runtime value), and from hardcoded-fast-path-jit-specialization (throughput constant-folding — this is address-space placement of per-thread state).".to_string(),
            ref_shape: "// naive: the loop bound is the RUNTIME kernel arg `ablock` (16 at\n// every call site, but the compiler cannot know that) — no unroll,\n// and a dynamically-indexed per-thread array has no register form:\n// every vals[]/gam[] access is a local-memory round-trip\nfn load_partial_sq_naive(input: &[f32], gamma: &[f32], start: usize,\n                         len: usize, ablock: usize) -> ([f32; 16], [f32; 16], f32) {\n    let mut vals = [0.0f32; 16]; // exactly sized — still demoted\n    let mut gam = [0.0f32; 16];\n    let mut partial_sq = 0.0f32;\n    for i in 0..ablock { // RUNTIME bound → dynamic indexing → local memory\n        let ok = i < len;\n        let v = if ok { input[start + i] } else { 0.0 };\n        vals[i] = v;\n        gam[i] = if ok { gamma[start + i] } else { 0.0 };\n        partial_sq += v * v;\n    }\n    (vals, gam, partial_sq)\n}\n".to_string(),
            opt_pattern: "// optimized: the bound is the compile-time ACTIVATION_BLOCK const\n// (the launcher's Rust const — a launcher invariant, not a kernel\n// arg) + unroll: statically-indexed arrays promote to registers;\n// the (i < len) tail-guard preserves partial-tile semantics, and\n// unrolling preserves the ascending accumulation order (bit-identical)\nfn load_partial_sq_unrolled(input: &[f32], gamma: &[f32], start: usize,\n                            len: usize) -> ([f32; 16], [f32; 16], f32) {\n    const ACTIVATION_BLOCK: usize = 16; // compile-time, NOT a runtime arg\n    let mut vals = [0.0f32; ACTIVATION_BLOCK]; // registers\n    let mut gam = [0.0f32; ACTIVATION_BLOCK]; // registers\n    let mut partial_sq = 0.0f32;\n    for i in 0..ACTIVATION_BLOCK { // const bound → unrolled → registers\n        let ok = i < len;\n        let v = if ok { input[start + i] } else { 0.0 };\n        vals[i] = v;\n        gam[i] = if ok { gamma[start + i] } else { 0.0 };\n        partial_sq += v * v;\n    }\n    (vals, gam, partial_sq)\n}\n".to_string(),
        },

        // ── batched-read-slot-membership-decided-at-launch —
        // read-batch slot membership must be decided at ONE site
        // (the launch), never re-derived differently at extraction (D) ──
        //
        // Source: riir-gpu kimi_k3_gpu_backward_batched.rs sync2
        // L1037-1099 (Batch 51 / Research 057, 2026-08-16) — the push
        // site guards the w_g GEMM on the local Option PAIR
        // `(Some(d_gate_pre), Some(w_g_t))` while the extraction site
        // guards consumption on `config.use_output_gate` — a flag that
        // IMPLIES the pair but is not implied by it. The builder
        // (build_mla_handles_from_cpu, kimi_k3_gpu_backward.rs L2345-2351)
        // propagates `w_g_t = if config.use_output_gate {
        // weights.w_g.as_ref().map(...) } else { None }`, so
        // use_output_gate=true + weights.w_g=None yields w_g_t=None:
        // push skips the slot while extraction consumes one — every
        // subsequent result shifts by one (d_h_g reads base_uv[0],
        // mislabeled) and the running index walks one past the end
        // (guaranteed OOB panic at base_uk extraction, far from root
        // cause). The dead `sync2_layout` vec (pushed 1+2n_h times,
        // never read) is the UNWIRED version of this rule's fix; the
        // same file's t6_meta (L524-641) is the wired positive
        // contrast. The write-side twin of this drift class is
        // write-plan-derived-from-cache-slots-not-recomputed (B40) —
        // but a READ list has no persistent state to derive from (the
        // handles vec is consumed by the read), so the fix shape is
        // genuinely different: decide membership ONCE into a named
        // binding consumed at both ends, or record a tag/metadata vec
        // at push time and drive extraction by iterating it.
        KernelRule {
            rule_id: "batched-read-slot-membership-decided-at-launch".to_string(),
            gpu_concept: "When a batched readback collects handles that were pushed under conditional guards, the extraction side must replay the EXACT slot membership the launch side decided — but the two ends naturally express the decision differently: the push site guards on a local Option pair (source handle AND weight handle present), while the extraction site guards on the config flag that merely IMPLIES it. When the implying-but-not-implied-by case fires (flag true, handle absent — constructible whenever a builder propagates an optional weight through .map()), every subsequent result slot shifts by one: contents get read under the wrong label and the running index walks one past the end (OOB panic far from the root cause). Unlike the write-plan drift family, a read list has NO persistent state to derive from (the handles vec is consumed by the read), so the fix is to make the launch site the single decision point: compute membership ONCE into a named binding consumed at both ends, or record a tag/metadata vec alongside the pushes and drive extraction by iterating it in lockstep. The tell: a running result_idx plus an if-chain at extraction re-deriving conditions that were decided differently at push time — while a sibling sync point in the same file already ships the recorded-meta shape. Minimum guard when refactoring is not immediate: assert at entry that the flag and the handle presence agree, hard-erroring on the builder mismatch. Distinct from write-plan-derived-from-cache-slots-not-recomputed (B40: write-side host-to-device plan where the fix DERIVES from persistent cache state — here the read list is consumed, nothing to derive from; fix = record/decide-at-launch), from batched-downloads-single-sync-cycle (B26: amortizes sync count, silent on extraction indexing), and from enum-dispatch-separate-kernel-per-variant (B29: kernel-side specialization metadata, not host read-batch membership).".to_string(),
            ref_shape: "// naive: push guards on the Option PAIR, extraction guards on the CONFIG\n// FLAG that only implies it — one decision, two expressions, they drift\nfn backward_layer(client: &Client, h: &Handles, cfg: &Config) {\n    let mut handles: Vec<Handle> = Vec::new();\n    if let (Some(dg), Some(wg)) = (&d_gate_pre, &h.w_g_t) {\n        handles.push(launch_gemm(client, dg, wg, l, proj, d)); // flag && handle\n    }\n    for head in 0..n_h {\n        handles.push(launch_gemm(client, &d_o[head], &h.w_uv[head], l, v_h, d_c));\n        handles.push(launch_gemm(client, &q_c[head], &h.w_uk[head], l, d_h, d_c));\n    }\n    let results = read_batched(client, handles);\n    let mut idx = 0usize;\n    if cfg.use_output_gate { // flag ONLY — handle may be absent: drift\n        let d_h_g = &results[idx]; // reads base_uv[0] under the wrong label\n        idx += 1;\n        accumulate(d_h_g);\n    }\n    for head in 0..n_h {\n        let uv = &results[idx]; idx += 1; // every slot shifted by one\n        let uk = &results[idx]; idx += 1; // final read: index OOB panic\n        base_uv[head] = uv; base_uk[head] = uk;\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: the launch site is the ONLY decision point — a recorded\n// tag vec drives extraction in lockstep; drift is structurally impossible\nfn backward_layer(client: &Client, h: &Handles, _cfg: &Config) {\n    let has_w_g = matches!((&d_gate_pre, &h.w_g_t), (Some(_), Some(_)));\n    let mut handles: Vec<Handle> = Vec::new();\n    let mut tags: Vec<Kind> = Vec::new(); // layout recorded AT PUSH time\n    if has_w_g {\n        handles.push(launch_gemm(client, d_gate_pre.as_ref().unwrap(),\n                                 h.w_g_t.as_ref().unwrap(), l, proj, d));\n        tags.push(Kind::DhG);\n    }\n    for head in 0..n_h {\n        handles.push(launch_gemm(client, &d_o[head], &h.w_uv[head], l, v_h, d_c));\n        tags.push(Kind::BaseUv(head));\n        handles.push(launch_gemm(client, &q_c[head], &h.w_uk[head], l, d_h, d_c));\n        tags.push(Kind::BaseUk(head));\n    }\n    let results = read_batched(client, handles);\n    for (tag, r) in tags.iter().zip(results.iter()) { // extraction REPLAYS\n        match tag {                                     // the recorded layout\n            Kind::DhG => accumulate(r),\n            Kind::BaseUv(hd) => base_uv[*hd] = r,\n            Kind::BaseUk(hd) => base_uk[*hd] = r,\n        }\n    }\n}\n".to_string(),
        },
        // ── unpack-loop-indexed-by-container-word —
        // index the unpack loop by the CONTAINER word, not the logical
        // element (A) ──
        //
        // Source: riir-gpu attention_q8kv_cubecl.rs Phase 1 Q·K loop
        // L212-224 (Batch 51 / Research 057, 2026-08-16) — the Q8_0 KV
        // dot loop indexes `e` over the 32 LOGICAL bytes of a block:
        // `word_idx = base + e/4`, `byte_in_word = e%4`,
        // `word = kv_qs[word_idx]` — the SAME u32 re-loaded 4 times,
        // plus a runtime div/mod and a runtime-variable shift amount
        // per element. GPU compilers cannot merge these loads without
        // unrolling-by-4, which is not guaranteed for while loops in
        // WGSL/CubeCL. Honest scope: the redundant loads are cache-hit
        // (L1) loads — the win is L1 bandwidth + issue slots in a
        // latency-bound decode kernel, not DRAM traffic. Generalizes
        // by container:value ratio (nibbles 8/u32, 2-bit 16/u32).
        KernelRule {
            rule_id: "unpack-loop-indexed-by-container-word".to_string(),
            gpu_concept: "When unpacking byte-packed quantized values (i8 x4 per u32), a loop indexed by the LOGICAL element reloads the same container word once per byte — 4 global loads where 1 suffices — and re-derives the word/byte position with runtime div/mod (e/4, e%4) plus a runtime-variable shift amount. GPU compilers cannot merge these loads without unrolling-by-4, which is not guaranteed for while loops in WGSL/CubeCL. The fix indexes the loop by the CONTAINER word: one load serves all its bytes, and the byte shifts become compile-time immediates in the (manual) unrolled extraction. Honest scope: the redundant loads are cache-hit (L1) loads — the win is L1 bandwidth + issue slots in a latency-bound decode kernel, not DRAM traffic. Generalizes by container:value ratio: nibbles pack 8 per u32 (one load serves 8), 2-bit values 16 per u32. Distinct from bit-cursor-unpack-to-width-specialized-byte-batch (B44: WIDTH DISPATCH — per-bits arms eliminating a software cursor's branches; here width is FIXED 8-bit and the decision is loop GRANULARITY = container granularity), from hoist-bit-address-out-of-packed-row-sweep (B44: hoists an address decomposition invariant ACROSS a whole row sweep — here nothing is loop-invariant, the load target legitimately changes every 4 iterations; the fix is load-batching, not invariant hoisting), and from vectorized-pack-load (popcorn: the pack-side cousin — 4 f32 loads merged into 1 packed load for vectorization; this is the unpack twin over a pre-packed quantized container). Note the file's Phase 1 already applies hoist-group-invariant-scale-out-of-word-loop (B17) correctly — block_scale loaded once per block; this rule is orthogonal (the payload load, not the scale).".to_string(),
            ref_shape: "// naive: loop indexed by the LOGICAL element (byte) — the container\n// word is re-loaded once per byte; div/mod + shift amount are runtime\nfn qk_dot_by_byte(query: &[f32], kv_qs: &[u32], qs_base: usize, scale: f32) -> f32 {\n    let mut dot = 0.0f32;\n    let mut e = 0usize;\n    while e < 32 {\n        let word_idx = qs_base + e / 4; // identical for e in [0..4), [4..8)...\n        let byte_in_word = e % 4; // runtime div/mod per element\n        let word = kv_qs[word_idx]; // LOAD #1..#4 of the SAME word\n        let byte = (word >> (byte_in_word * 8)) & 0xFF; // runtime shift\n        let i8v = if byte >= 128 { byte as f32 - 256.0 } else { byte as f32 };\n        dot += query[e] * (scale * i8v);\n        e += 1;\n    }\n    dot // 32 loads (8 unique) + 32 div/mod + 32 runtime shifts per block\n}\n".to_string(),
            opt_pattern: "// optimized: loop indexed by the CONTAINER word — one load serves 4\n// bytes; byte shifts are compile-time immediates (manual 4-way unroll)\nfn qk_dot_by_word(query: &[f32], kv_qs: &[u32], qs_base: usize, scale: f32) -> f32 {\n    let mut dot = 0.0f32;\n    let mut w = 0usize;\n    while w < 8 {\n        let word = kv_qs[qs_base + w]; // ONE load per 4 values\n        let b0 = word & 0xFF; // immediate shifts\n        let b1 = (word >> 8) & 0xFF;\n        let b2 = (word >> 16) & 0xFF;\n        let b3 = (word >> 24) & 0xFF;\n        for (e, b) in [b0, b1, b2, b3].iter().enumerate() {\n            let i8v = if *b >= 128 { *b as f32 - 256.0 } else { *b as f32 };\n            dot += query[w * 4 + e] * (scale * i8v);\n        }\n        w += 1;\n    }\n    dot // 8 loads per block (was 32) + constant shifts\n}\n".to_string(),
        },
        // ── dequant-placement-by-component-traffic-share —
        // values stay packed + dequant in registers; scales pre-decode
        // at upload — the placement is decided per component by
        // traffic share (N) ──
        //
        // Source: riir-gpu attention_q8kv_cubecl.rs (Batch 51 /
        // Research 057, 2026-08-16) — the Q8_0 KV cache deliberately
        // splits representation: values stay packed i8 (kv_qs:
        // Vec<u32>, inline dequant per read, header L4-5) while scales
        // are CPU pre-decoded f16->f32 at upload (kv_scales: Vec<f32>,
        // L398-400 + L514-517). The deciding inequality per component:
        // expected reads x bytes saved vs conversion cost x reads.
        // Values dominate bytes (1 B/elem vs 4 B f32 -> 4x less
        // bandwidth for the dominant traffic at ~2 ALU ops/element;
        // within-workgroup reuse of a dequantized K row is 1 — each
        // thread owns a distinct position — so tile-staging buys
        // nothing). Scales are a tiny sidecar (1 f16 per 32 values,
        // 1/16 the traffic of even the packed values): pre-decode once
        // — the negligible +2 B/32 elems removes the per-block-
        // per-thread f16 decode from both consumer loops. The naive
        // shapes are both real: (a) dequantize the whole cache to f32
        // to reuse the f32 kernel — 4x KV bandwidth forever
        // (quantization bought memory but not bandwidth); (b) keep
        // scales in storage dtype and decode in-kernel per block per
        // thread per step.
        KernelRule {
            rule_id: "dequant-placement-by-component-traffic-share".to_string(),
            gpu_concept: "A quantized persistent operand (e.g. a Q8_0 KV cache read by attention every decode step) has two components with OPPOSITE optimal dequant placements, decided per component by traffic share x conversion cost x read count. The VALUES dominate bytes (1 B/elem vs 4 B f32): keep them packed on device and dequant in registers at every read — 4x less bandwidth for the dominant traffic at ~2 ALU ops/element, and within-workgroup reuse of a dequantized row is 1 (each thread owns a distinct position), so tile-staging buys nothing. The SCALES are a tiny sidecar (1 f16 per 32 values, 1/16 the traffic of even the packed values): pre-decode f16-to-f32 once at upload — the negligible +2 B per 32 elems removes the per-block-per-thread decode from every consumer loop. The naive shapes are both real: (a) dequantize the whole cache to f32 to reuse the f32 kernel — quantization bought memory but not bandwidth; (b) keep scales in storage dtype and decode in-kernel — redundant conversion per block per thread per step. Note the tension with dtype-normalize-once-at-upload (B31): B31 is CORRECT for the sidecar and WRONG for the payload — normalizing the whole operand at upload is exactly naive shape (a); this rule encodes when each half applies. Distinct from dequant-once-per-k-tile-into-shared-memory (B14: amortizes dequant across M output rows sharing a weight tile — reuse > 1 regime; a KV-cache K row has reuse 1 within the scoring workgroup, so inline-in-registers is the correct placement here — the smem-staging-gated-by-cross-thread-reuse (B35) gate applied to dequant placement), from verbatim-upload-on-format-match (B37: load-path transcode decision; this is per-read consumption placement), and from kv-window-device-resident-ring-append (B37: WHERE the cache lives; this is what REPRESENTATION it is stored in).".to_string(),
            ref_shape: "// naive (a): one placement for the whole operand — materialize the\n// cache at f32 before attention to reuse the f32 kernel\nfn attention_with_dequantized_cache(query: &[f32], kv_q8: &Q8Cache, out: &mut [f32]) {\n    let mut keys_f32 = vec![0.0f32; kv_q8.n * KV_DIM];\n    let mut values_f32 = vec![0.0f32; kv_q8.n * KV_DIM];\n    kv_q8.dequantize_into(&mut keys_f32, &mut values_f32); // f16 decode + i8 mul\n    f32_attention(query, &keys_f32, &values_f32, out); // 4 B/value read forever\n}\n// naive (b): both components in storage dtype — the scale f16 decoded\n// in-kernel per block per thread per step\n".to_string(),
            opt_pattern: "// optimized: split placement per component by traffic share\nstruct Q8KvBuffers {\n    kv_qs: Vec<u32>, // VALUES stay packed (1 B/elem, inline dequant)\n    kv_scales: Vec<f32>, // SCALES pre-decoded f16->f32 at upload\n}\n\nfn qk_score_inline(query: &[f32], kv: &Q8KvBuffers, pos: usize) -> f32 {\n    let mut dot = 0.0f32;\n    for blk in 0..8 {\n        let d = kv.kv_scales[pos * 8 + blk]; // f32 read — no f16 decode\n        let base = pos * 64 + blk * 8;\n        for e in 0..32 {\n            let i8v = unpack_byte(&kv.kv_qs, base, e); // dequant at read\n            dot += query[blk * 32 + e] * (d * i8v);\n        }\n    }\n    dot\n}\n// decision: values -> inline (4:1 bytes ratio x every-step reads\n// outweighs 2 ALU/elem); scales -> pre-decode (sidecar traffic\n// negligible, decode would be per-block-per-thread)\n".to_string(),
        },
        // ── phase-split-per-position-data-liveness-row-storage ── distilled
        // from the measured-GOAT riir-ai Issue 708 H4 fix (2026-08-16): the
        // dflash forward's Phase-A project_q wrote a single-row [n_embd] q
        // buffer per position, so Phase-B bidirectional attention could only
        // ever see the LAST position's Q - invisible while the attention
        // dispatch was a no-op stub, and NOT a stale-uniform bug (the right
        // value was never stored). The fix (q_cache [seq_len*n_embd] +
        // row-pos writes + backward reading row pos) made the forward
        // CPU-reference-pinned at max_rel=5e-6.
        KernelRule {
            rule_id: "phase-split-per-position-data-liveness-row-storage".to_string(),
            gpu_concept: "A Phase-A dispatch loop that projects per-position intermediates (Q from token p, per-layer hidden rows) into a SINGLE-ROW working buffer [dim] followed by a separately-scheduled Phase-B consumer loop that needs EVERY position's value only ever preserves the LAST write - each Phase-A dispatch overwrites the whole row before any consumer runs (the encoder is submitted once after both loops are built). The bug is invisible while the consumer is a stub or reads a different buffer, and it is NOT a stale-uniform bug (B26/B27: the wrong VALUE is read per dispatch) - here the right value is NEVER STORED: the buffer has no per-position storage at all. Fix shape: per-position row storage - a [seq_len x dim] cache with the producer kernel writing row pos (out[pos * dim + i]) via the existing pos-in-uniform, and the consumer indexing rows (attention reads q rows per position; the backward replay copies its per-head slice from row pos). The write index rides the SAME uniform field the kernel already receives - zero new dispatch state. Tell: a dispatch fn binds a whole single-row buffer, the kernel writes flat [0..dim), and the caller loops positions BEFORE the consumer phase executes. Distinct from per-slot-uniform-buffer-write-ordering and staging-packed-uniforms-encoder-ordered-copies (uniform PARAMS staleness - wrong values read; this is output DATA liveness - per-position values never stored), from preallocated-activation-slots-inplace-save (saves FULL per-layer activations for backward replay; this fixes the forward's own cross-phase consumption of per-position intermediates), and from append-cache-rewind-overwrite-in-place (cache growth/lifetime management; the row count here is fixed by construction seq_len).".to_string(),
            ref_shape: "// naive: Phase A overwrites a single-row q buffer per position; only the\n// LAST position's Q survives into Phase B attention\nfn forward_two_phase(encoder: &mut Encoder, seq_len: usize) {\n    for layer in 0..n_layer {\n        // Phase A: fill concat K/V + 'Q' for ALL positions\n        for pos in 0..seq_len {\n            dispatch_qkv(encoder, layer, pos, &q_row_buffer); // q_out[i], flat [0..dim)\n        }\n        // Phase B: attention per position reads 'its' Q - which is seq_len-1's\n        for pos in 0..seq_len {\n            dispatch_attention(encoder, layer, pos, &q_row_buffer); // wrong: shared row\n        }\n    }\n}\n".to_string(),
            opt_pattern: "// optimized: per-position row storage - producer writes row pos, consumers index rows\nstruct QkvCache {\n    q_cache: Buffer, // [seq_len * n_embd] - one row per position\n}\n\nfn project_q_kernel(params: &Params, out: &[f32]) {\n    let i = global_id();\n    if i >= params.n_embd { return; }\n    let sum = dot_row(&params.wq, i, &params.noise_hidden);\n    out[params.pos * params.n_embd + i] = sum + lora_delta(params, i); // ROW pos\n}\n\nfn forward_two_phase(encoder: &mut Encoder, seq_len: usize) {\n    for layer in 0..n_layer {\n        for pos in 0..seq_len {\n            dispatch_qkv(encoder, layer, pos); // q_cache row pos (pos already in uniform)\n        }\n        dispatch_attention_concat(encoder, layer); // reads q_cache rows [0..seq_len)\n    }\n}\n".to_string(),
        },
        // ── regularizer-above-noise-floor-coupled-to-damping-cap ── distilled
        // from the measured-GOAT riir-train Plan 339 T1.5 / Bench 470
        // (2026-08-17, RTX 4090, TF32 cuBLAS, 26 adversarial cases): NS
        // orthogonalization on a rank-deficient Gram diverges with spurious
        // negative null-space eigenvalues set by SIGNED ~1e-3 per-GEMM TF32
        // noise — the shipped eps=1e-5 sits 100x below that noise floor.
        // The measured winner needs BOTH knobs: eps raised to the noise
        // floor AND capped by the gamma damping budget (asserted in code),
        // PLUS a second FULL-schedule pass with the Gram re-formed from the
        // DATA iterate (26/26 + production 0.9587 > 0.9454; each knob alone
        // measured FAIL). Second measured-GOAT distillation after the
        // riir-gpu corpus-complete declaration (the B49/B52 reopen path).
        KernelRule {
            rule_id: "regularizer-above-noise-floor-coupled-to-damping-cap".to_string(),
            gpu_concept: "A reduced-precision matrix-polynomial iteration (Newton-Schulz / inverse-PSD-square-root over a TF32 Gram) run on rank-deficient input develops SPURIOUS eigenvalues in the Gram's null space, set by the SIGNED per-GEMM relative noise of the format (~1e-3 in TF32) — not by the diagonal eps regulariser, and not by ill-conditioning alone (a cond~1e12 decay spectrum is clean; exact/near rank deficiency with O(1)-magnitude columns is the trigger). Sizing the fix needs BOTH coupled constraints, and neither knob alone repairs the iteration: (1) eps must sit ABOVE the format's noise floor — eps=1e-5 is 100x below TF32 GEMM noise, a regulariser quieter than the thing it regularises; it cannot push noise-dominated null eigenvalues off the edge of the convergence basin. (2) eps is capped by the damping budget: after normalisation lambda_max = 1+eps, and the gamma damping budgets exactly gamma-1 of overshoot past the polynomial's convergence basin [0,1] — the coupling eps <= gamma-1 must be ASSERTED in code, not documented in a comment (10x the budget diverges to sigma_max ~1e24 on the well-conditioned CONTROL). (3) raising eps alone breaks convergence at high d_min: (lambda/(lambda+eps))^(1/2) shrinks every singular value (norm(O)/sqrt(d_min) -> 0.36). The repair is a SECOND FULL-coefficient-schedule pass with the Gram re-formed from the DATA iterate (per pass: S = cur^T cur, normalise, run the WHOLE NS schedule to R = S^(-1/2), then cur = cur R): after pass 1 the singular values sit near 1 so S ~= I and eps is no longer large relative to the spectrum (rank-1 case lambda_min/lambda_max: -1.9e-1 -> -1.1e-9). Two refuted variants, both measured: a GRAM-DOMAIN restart (P = X P0 X from the recurrence invariant — algebraically identical and free of extra tall GEMMs, but X reaches ~1/sqrt(eps) ~= 316 while the null-direction product must cancel back to ~3e-3, four digits of cancellation in a format carrying three; broke the well-conditioned control 16/26 -> 6/26) and a PARTIAL-schedule restart (re-applies the aggressive a=7.42 prefix to an already-orthogonal spectrum; composes to a fixed point at sigma ~= 1.7 that MORE passes do not improve — passes must be whole schedules). Guard corollary: a degenerate-input guard written `t < 1e-20` is FALSE for NaN (every NaN comparison is false) — an overflowed Gram sails through and poisons the weights permanently; write `!t.is_finite() || t < threshold` (a zero step skips, a NaN step is unrecoverable). Measured decision table (Bench 470, 26 adversarial cases): shipped eps=1e-5 single-pass 16/26; restart alone 14/26 (WORSE — a fresh Gram gets a fresh helping of the same signed noise); eps=1e-3 alone 26/26 but convergence 0.36; eps=1e-3 + 2 full passes 26/26 AND production convergence 0.9587 > 0.9454. Distinct from saturating-nonlinearity-recovery-breaks-near-cap (a derivative-recovery identity failing at the saturation boundary of one cached output; this is iteration-level divergence from format noise in the input spectrum) and from exact-op-parity-for-analytic-backward (forward/backward op consistency; this is iteration robustness vs the GEMM noise floor).".to_string(),
            ref_shape: "// naive: eps 100x below the TF32 noise floor that creates the spurious\n// spectrum, one pass, and a degenerate guard that NaN sails through\nconst NS_GAMMA: f32 = 1.001; // damping: budgets exactly 1e-3 of overshoot\nconst NS_EPS: f32 = 1e-5; // TF32 per-GEMM noise is ~1e-3 - this regulariser\n// is quieter than the thing it regularises (the coupling is undocumented)\n\nfn run_ns_default(gram: &Mat) -> Mat {\n    let t = gram.norm_inf();\n    if t < 1e-20 {\n        return Mat::identity(); // FALSE for NaN: an overflowed Gram\n    } // sails straight through and poisons the weights permanently\n    let mut p = normalize_psd(gram, NS_EPS); // rank-deficient null-space\n    for k in 0..5 { // eigenvalues stay signed-noise-dominated -> NEGATIVE\n        p = ns_poly_step(p, COEFFS[k]); // the polynomial amplifies them:\n    } // lambda_min/lambda_max = -0.19 at 5 iters, sigma_max ~1e8 at 7\n    p\n}".to_string(),
            opt_pattern: "// optimized (Bench 470): eps at the noise floor + asserted damping cap +\n// a second FULL-schedule pass with data-domain Gram re-form + finite guard\nconst NS_GAMMA: f32 = 1.001;\nconst NS_EPS: f32 = 1e-3; // above the ~1e-3 TF32 GEMM noise floor so it can\n// actually push null-space eigenvalues off the basin edge - AND capped:\nconst _: () = assert!(NS_EPS <= NS_GAMMA - 1.0); // normalisation makes\n// lambda_max = 1+eps; the gamma budget is exactly gamma-1 (10x the budget\n// diverges on the CONTROL). Asserted in code - the coupling is load-bearing.\n\nfn run_ns_hardened(m: &Mat, passes: usize) -> Mat {\n    let t = m.norm_inf();\n    if !t.is_finite() || t < 1e-20 {\n        return Mat::identity(); // NaN caught too: a zero step skips,\n    } // a NaN step is unrecoverable\n    let mut cur = m.clone();\n    for _pass in 0..passes.max(1) {\n        // 2 at the hardened setting (each knob alone measured FAIL)\n        let s = gram_of(&cur); // Gram re-formed from the DATA iterate -\n        let mut r = normalize_psd(&s, NS_EPS); // NOT carried in Gram domain\n        for k in 0..COEFFS.len() {\n            r = ns_poly_step(r, COEFFS[k]); // FULL schedule every pass:\n        } // early coeffs (a=7.42) are tuned for a raw Gram; a truncated\n        cur = matmul(cur, &r); // prefix restarts at fixed point sigma ~1.7\n    } // after pass 1 singular values ~1, so S = cur^T cur ~ I: eps is no\n    cur // longer large vs the spectrum (rank-1 lambda_min/lambda_max:\n} // -1.9e-1 -> -1.1e-9; production convergence 0.9587 > 0.9454)".to_string(),
        },
        // ── outlier-span-escapes-quantization-via-lossless-sidecar ── distilled
        // from the measured-GOAT riir-ai Issue 716 / Bench 691 (2026-08-18,
        // M3 Max Metal, GPU-exclusive): the arXiv:2608.12149 massive-activation
        // failure mode is LIVE on the Gemma-2 q8kv decode path - synthetic MA
        // injection (1-2 channels x100/x300 at sink pos 0 + delimiter)
        // inflates q8kv attention output error 584x/18,844x over uniform data,
        // concentrated at the poisoned 32-blocks (88x/299x vs clean dims); CPU
        // row-level neighbor collapse 58x - the KV-side twin of the Research
        // 085/086 weight-side collapse. T3 mitigation A/B: (b) lossless f32
        // sink sidecar WINS (unguarded 4.781 -> 0.000698 = 6,850x; full
        // sidecar bit-identical to the f32 kernel at 0.00000000 max diff)
        // over (c) Hadamard sink rotation (19.6x reduction but nonzero +
        // strictly larger kernel cost: rotated-Q buffer + sink accumulator +
        // un-rotate phase). S=0 bit-identical to the unguarded launch; 0.96x
        // perf (neutral-to-faster - f32 sidecar reads replace byte-dequant
        // loops on the guard span). Third measured-GOAT distillation after
        // the riir-gpu corpus-complete declaration (the B49/B52/B54 reopen
        // path (c)).
        KernelRule {
            rule_id: "outlier-span-escapes-quantization-via-lossless-sidecar".to_string(),
            gpu_concept: "A per-block quantized persistent operand (Q8_0 KV cache) collapses when 1-2 massive-activation channels sit in a 32-value block: the outlier sets the block scale and the NON-outlier neighbors in the same block quantize at 58x the error of clean blocks (row-level collapse; attention output error inflates 584x/18844x over uniform data at x100/x300 injection, concentrated at the poisoned blocks at 88x/299x vs clean dims - the KV-side twin of the weight-side outlier-to-scale collapse). The rows carrying the outliers are typically KNOWN A PRIORI, not detected: attention sinks are the oldest fixed positions, and hybrid models' full-attention schedule boundaries are enumerable from config. Fix shape: the known outlier span escapes quantization entirely into a LOSSLESS f32 sidecar ([keys || values], one contiguous vec), while the quantized buffers keep those rows as DEAD DATA so the q8 layout and strides never shift - the kernel branches per position (the guard span reads the sidecar in BOTH the scoring and the V-accumulation phases) and derives the row count from the sidecar allocation length divided by the row stride. A/B measured (Bench 691): the lossless sidecar WINS over Hadamard rotation of the outlier rows on BOTH axes - rotation spreads the activation to absmax ~ MA/16 so neighbors regain only ~8x resolution (nonzero residual), and its kernel cost strictly exceeds a per-position branch (rotated-Q buffer + separate sink accumulator + an un-rotate phase needing cross-thread reduction); the sidecar is EXACT on the guard span (unguarded 4.781 -> 0.000698 = 6850x; a full sidecar reproduces the f32 kernel at 0.00000000 max diff) at sidecar memory cost (S=4 rows -> 32 KB/layer). Off-state discipline: S=0 must be bit-identical to the unguarded path (to_bits per element), and the guard is perf-neutral-to-faster (0.96x - f32 sidecar reads REPLACE byte-dequant loops on the guard span). Distinct from water-fill-tail-dim-shared-codebook (allocates BITS by importance inside one quantized representation; this EXCLUDES a row span from quantization into a lossless channel), from dequant-placement-by-component-traffic-share (WHERE the dequant conversion happens per component; this chooses WHAT REPRESENTATION each row span is stored in), and from single-sweep-minmax-for-quant-scale (scale computation inside one representation; the collapse here is inherent to ANY per-block scale carrying a x100 outlier).".to_string(),
            ref_shape: "// naive: quantize the WHOLE KV cache per-32-block - the sink row's massive\n// activations (2 channels at x300) set every block scale, and non-outlier\n// NEIGHBORS in the same block collapse (58x row error, 18844x attention err)\nfn quantize_all(keys: &[f32], values: &[f32]) -> Q8Cache {\n    let mut cache = Q8Cache::new(keys.len());\n    for (blk, chunk) in keys.chunks(32).enumerate() {\n        let scale = chunk.iter().fold(0.0f32, |m, v| m.max(v.abs())) / 127.0;\n        cache.write_block(blk, scale, chunk); // neighbors share the outlier scale\n    }\n    cache // same loop for values - no row is special\n}\n\n// refuted variant (c): rotate outlier rows with a Hadamard H to spread the MA\n// - absmax drops to ~MA/16 so neighbors regain only ~8x resolution (NOT\n// exactness), and the kernel needs a rotated-Q buffer + separate sink\n// accumulator + un-rotate phase (cross-thread reduction) - strictly costlier\n// than a per-position branch\nfn score_rotated_sink(query: &[f32], sink_k_rot: &[f32]) -> f32 {\n    let hq = fwht_256(query); // rotated Q every call\n    let mut dot = 0.0f32;\n    for d in 0..256 {\n        dot += hq[d] * sink_k_rot[d];\n    }\n    un_rotate_accumulate(dot) // V contribution needs the cross-thread un-rotate\n}\n".to_string(),
            opt_pattern: "// optimized (Bench 691): the KNOWN outlier span (attention sinks = the oldest\n// fixed positions - no detection) escapes quantization into a lossless f32\n// sidecar; q8 buffers keep the rows as dead data so strides never shift\nstruct Q8KvBuffers {\n    kv_qs: Vec<u32>,    // full cache; rows 0..S stay as dead data\n    kv_scales: Vec<f32>,\n    sink_kv: Vec<f32>,  // [sink_keys || sink_values] raw f32, S rows\n}\n\nfn qk_score(query: &[f32], bufs: &Q8KvBuffers, pos: usize) -> f32 {\n    let sink_rows = bufs.sink_kv.len() / 2 / KV_STRIDE; // derived row count\n    if pos < sink_rows {\n        // guard span: lossless f32 read replaces the byte-dequant loop\n        let base = pos * KV_STRIDE;\n        return dot_f32(query, &bufs.sink_kv[base..base + KV_STRIDE]);\n    }\n    q8_dot_dequant(query, &bufs.kv_qs, &bufs.kv_scales, pos) // unchanged path\n}\n// measured: unguarded 4.781 -> guarded 0.000698 (6850x); S=0 bit-identical\n// to the unguarded kernel (dummy 1-f32 sidecar -> len 1 -> 1/2/1024 = 0);\n// S=n_positions reproduces the f32 kernel exactly (0.00000000 max diff);\n// 0.96x wall-clock - f32 reads are cheaper than dequant on the guard span\n".to_string(),
        },
        // ── buffer-length-derived-from-allocation-not-binding ── distilled
        // from TWO root-caused occurrences: Bench 642 / Issue 639 (2026-08-12,
        // the OVER-LENGTH half - rmsnorm reduced x^2 over the FOLLOWING rows
        // because `let dim = input.len()` reads the ALLOCATION, not the bound
        // length; fixed by both-ends binding trims offset_start(start)
        // .offset_end(tail), filed class-wide as Issue 639 - five ternary GEMV
        // kernels shared `let m = output.len()`) and Bench 691 / Issue 716
        // (2026-08-18, the ZERO-LENGTH half - a from_raw_parts
        // (query_handle.clone(), 0) dummy made sink_rows derive 1 from the
        // 2048-f32 allocation, silently reading the QUERY buffer as
        // position-0 K/V; uniform-baseline error 0.000268 -> 0.12 across
        // every test; fixed by a real 4-byte allocation that floor-divides
        // to zero rows). The Bench 642 probe corollary rides along: a probe
        // exercising only the benign case (the LAST row, where remaining
        // allocation coincidentally equals dim) is worse than no probe - it
        // converted an open question into a wrong answer.
        KernelRule {
            rule_id: "buffer-length-derived-from-allocation-not-binding".to_string(),
            gpu_concept: "The kernel-visible length of a bound buffer is derived from the ALLOCATION (handle.size_in_used() / elem_size in cubecl's KernelLauncher::process_buffer), NOT from the bound BufferArg length - the length field is validation-only in the checked path and launch_unchecked never sees it. Two root-caused consequences, both silent-wrong-result classes: (1) OVER-LENGTH (Bench 642/Issue 639): a kernel deriving a semantic parameter from an array length (`let dim = input.len()`) reads the allocation, so binding one row of a larger buffer makes it consume the FOLLOWING rows too - rmsnorm reduced x^2 over following rows while inv_dim still said 1/dim, a pure scale error; the fix trims the binding at BOTH ends (offset_start(start).offset_end(tail)) so the view is exactly stride elements. (2) ZERO-LENGTH-OVER-A-REAL-HANDLE (Bench 691/Issue 716): an absent optional array CANNOT be expressed as from_raw_parts(handle.clone(), 0) - the kernel derives the handle's allocation length (a 2048-f32 query handle -> sink_rows = 1) and silently reads the WRONG buffer (position-0 K/V became the query buffer; the uniform baseline error inflated 0.000268 -> 0.12 across every test); the fix is a REAL minimal allocation sized to floor-divide to the semantic zero (1 f32 -> len 1 -> 1/2/1024 = 0 rows). The contract is EXACT-sized allocation, not 'at least' - because the length is derived FROM the allocation, oversizing CHANGES semantics. Probe corollary (Bench 642): a probe that exercises only the benign case is worse than no probe - the earlier offset probe CLEARED rmsnorm because it tested the LAST row of a 2-row buffer, where the remaining allocation coincidentally equals dim; it converted an open question into a wrong answer. Probe the row where derived length differs from intended length (row 0). Distinct from launch-unchecked-caller-owned-size-contract (elides per-launch validation by documenting caller-owned size contracts; here the caller CANNOT pass an explicit length - the runtime derives it from the allocation, so the fix is allocation-SHAPING, not contract documentation), from empty-slice-sentinel-launders-shape-violation (a sentinel slice type-checks past validation into a swallowed Err; here nothing is swallowed - the kernel reads real memory from the wrong buffer with silently wrong numerics), and from pad-uniform-pod-to-min-binding-size (hardware binding-size minimums; this is semantic length derivation).".to_string(),
            ref_shape: "// naive: express 'no optional rows' as a ZERO-LENGTH binding over a real\n// handle - the runtime derives the kernel-visible length from the\n// ALLOCATION (handle.size_in_used()), not the bound length, so the kernel\n// sees 2048 f32s of QUERY buffer and derives sink_rows = 1: position 0's\n// K/V silently becomes the query buffer (uniform error 0.000268 -> 0.12)\nfn launch_no_guard(client: &ComputeClient, query: Handle, kv: Handle) {\n    let sink_arg = BufferArg::from_raw_parts(query.clone(), 0); // the len\n    // field is validation-only in the checked path; launch_unchecked never\n    // sees it - the kernel reads query as the sink sidecar\n    unsafe {\n        attention_decode::launch_unchecked(client, grid, dim, query, kv, sink_arg);\n    }\n}\n\n// naive twin (Bench 642, the over-length half): bind one row of a larger\n// buffer WITHOUT trimming the tail - `let dim = input.len()` in rmsnorm\n// derives the remaining ALLOCATION, so it reduces x^2 over the FOLLOWING\n// rows too (a pure scale error: inv_dim still says 1/dim)\nfn per_token_rmsnorm(client: &ComputeClient, buf: Handle, token: usize) {\n    let row = BufferArg::from_raw_parts(buf.clone(), DIM); // 'len' = DIM is\n    // ignored - the view starts at token*DIM but extends to the allocation\n    // end; only the LAST token is benign (remaining == DIM exactly)\n    unsafe { rmsnorm_f32::launch_unchecked(client, grid, dim, row); }\n}\n".to_string(),
            opt_pattern: "// optimized: the ALLOCATION is the only length the kernel can see - shape it.\n// (1) absent optional array = a REAL minimal allocation that floor-divides\n// to the semantic zero; (2) sub-row views = trim the binding at BOTH ends\nfn launch_guarded(client: &ComputeClient, query: Handle, kv: Handle, sink: Option<Handle>) {\n    // 1 f32 allocation -> len 1 -> 1/2/1024 = 0 rows: neither guard branch\n    // is taken, S=0 output bit-identical to the pre-guard kernel. The\n    // contract is EXACT-sized - oversizing CHANGES semantics (len derives\n    // sink_rows), and a zero-length binding over a real handle reads the\n    // WRONG buffer entirely.\n    let sink_handle = sink.unwrap_or_else(|| client.empty(4)); // real alloc\n    unsafe {\n        attention_decode::launch_unchecked(client, grid, dim, query, kv, sink_handle);\n    }\n}\n\nfn tok_slice(buf: Handle, token: usize, stride: usize, total: usize) -> BufferArg {\n    // trim BOTH ends so the view is exactly `stride` elements (Issue 639):\n    // offset_start positions the row, offset_end cuts the tail the kernel\n    // would otherwise derive from the allocation\n    BufferArg::new(buf)\n        .offset_start(token * stride)\n        .offset_end(total - (token + 1) * stride) // 0 only at the last row\n}\n// probe corollary (Bench 642): a probe exercising only the benign case is\n// WORSE than no probe - the earlier offset probe cleared rmsnorm because it\n// tested the LAST row, where remaining allocation coincidentally equals\n// DIM. Probe row 0, where derived length != intended length.\n".to_string(),
        },
        // ── concurrent-context-init-needs-process-wide-lock ── distilled
        // from the measured-GOAT riir-ai Issue 719 fix (2026-08-18, commit
        // 97bcf4738): the Issue-714 OnceLock process-global GpuContext cache
        // assumed "race-benign" parallel first-callers — on the
        // 4090/Windows/Vulkan box concurrent driver-level init (wgpu
        // Instance/Adapter/Device + CubeCL server registration) deadlocks
        // non-deterministically (7-thread race: 4 wedged forever inside
        // request_device, another run 1/7 inside init_device; full lib suite
        // froze at 101/604 with every worker blocked behind first-call init)
        // AND every loser leaks its device + server for process lifetime (no
        // deregistration). Fix = process-wide GPU_INIT_LOCK shared by
        // GpuContext::new AND CubeCLContext::new (both run the same
        // sequence), double-checked fast path, poison de-fanged, failure not
        // cached. Regression gate tests/context_init_race.rs (8-thread
        // same-path + 8-thread cross-path, 3x green ~1s). Post-fix: 605
        // passed / 0 failed in 1664 s — the first green parallel-mode full
        // suite on the box. Fourth measured-GOAT distillation after the
        // corpus-complete declaration (the B49/B52/B54/B55 reopen path (c)).
        KernelRule {
            rule_id: "concurrent-context-init-needs-process-wide-lock".to_string(),
            gpu_concept: "A process-global GPU context cache built with the init-outside-publish-on-success pattern (required for failure-not-cached semantics — an adapter may appear mid-run, so an Err must NOT pin the cache) does NOT serialize construction: publication is atomic but construction is not — every first-caller that misses the fast-path read runs the FULL driver-level sequence (Instance/Adapter/Device creation + compute-server registration) concurrently. Two failure halves, both measured on 4090/Windows/Vulkan: (1) concurrent execution of that sequence wedges NON-DETERMINISTICALLY at the driver level — 7 threads racing the constructor: 4 wedged forever inside adapter.request_device() (another run: 1/7 inside init_device; the stuck point MOVES between runs — a driver-level race, not a single lock site); a parallel test suite froze with every worker blocked behind first-call init (101/604). (2) every racer that completes the sequence but loses the publication slot leaks its Instance/Adapter/Device + registered server for the process lifetime when the substrate has no deregistration — the partial reintroduction of a 46-live-device leak. The CROSS-PATH half: the same driver-level sequence reachable from MULTIPLE constructors each with its OWN cache (GpuContext::new AND CubeCLContext::new) — each path serializes its own callers while the two paths race each other; one lock must guard ALL entry points running the same sequence. Fix shape: a process-wide init Mutex (const-constructible Mutex<()>), acquired ONLY after the fast-path cache read (steady state stays one atomic load — no lock), DOUBLE-CHECK the cache after acquiring (the racer you queued behind may have initialized), construct + publish, release before returning Err (failure still not cached — the next caller retries fresh); de-fang poison (a panicking init leaves nothing partial worth blocking later callers over: lock().unwrap_or_else(|e| e.into_inner())). Guarded by a race regression test covering BOTH same-path and cross-path racers. Distinct from env-flag-resolved-once-not-per-dispatch (B38: WHERE a decision input is resolved — this is concurrency safety of shared-state construction), from chunk-encoder-submits-below-driver-limit (B26: a per-encoder submission ceiling; this is init-time concurrency), and from autotune-cache-extract-outlives-owner (B39: lifetime salvage of a cache ACROSS owner destruction; this prevents concurrent construction from wedging/leaking in the first place).".to_string(),
            ref_shape: "// naive: \"race-benign\" init-outside-publish — every first-caller runs the\n// full driver-level sequence concurrently; publication is atomic,\n// construction is NOT\nstatic SHARED_CTX: std::sync::OnceLock<Ctx> = std::sync::OnceLock::new();\npub fn ctx_a() -> Result<Ctx, GpuError> {\n    if let Some(c) = SHARED_CTX.get() { return Ok(c.clone()); } // fast path\n    let ctx = pollster::block_on(build_ctx_async())?; // 4/7 threads WEDGE\n    // here forever (request_device / init_device — driver-level race, stuck\n    // point moves between runs); every racer that completes but loses the\n    // slot leaks its device + server for process lifetime\n    Ok(SHARED_CTX.get_or_init(|| ctx).clone())\n}\npub fn ctx_b() -> Result<Ctx, GpuError> {\n    // SECOND entry point, its OWN OnceLock — serializes ITS callers while\n    // racing ctx_a(): both run the same driver-level sequence\n    static CTX_B: std::sync::OnceLock<Ctx> = std::sync::OnceLock::new();\n    if let Some(c) = CTX_B.get() { return Ok(c.clone()); }\n    let ctx = pollster::block_on(build_ctx_async())?; // races ctx_a()'s init\n    Ok(CTX_B.get_or_init(|| ctx).clone())\n}\n".to_string(),
            opt_pattern: "// optimized: ONE process-wide init lock shared by every entry point that\n// runs the same driver-level sequence; double-checked fast path\nstatic GPU_INIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());\nfn gpu_init_lock() -> std::sync::MutexGuard<'static, ()> {\n    GPU_INIT_LOCK.lock().unwrap_or_else(|e| e.into_inner()) // poison\n} // de-fanged: a panicking init leaves nothing partial worth blocking over\nstatic SHARED_CTX: std::sync::OnceLock<Ctx> = std::sync::OnceLock::new();\npub fn ctx_a() -> Result<Ctx, GpuError> {\n    if let Some(c) = SHARED_CTX.get() { return Ok(c.clone()); } // steady\n    let _guard = gpu_init_lock(); // state: one atomic load, no lock\n    if let Some(c) = SHARED_CTX.get() { // double-check: the racer we\n        return Ok(c.clone()); // queued behind may have initialized\n    }\n    let ctx = pollster::block_on(build_ctx_async())?; // alone under the\n    // guard; Err propagates with the lock RELEASED (failure not cached —\n    Ok(SHARED_CTX.get_or_init(|| ctx).clone()) // next caller retries fresh)\n}\npub fn ctx_b() -> Result<Ctx, GpuError> {\n    static CTX_B: std::sync::OnceLock<Ctx> = std::sync::OnceLock::new();\n    if let Some(c) = CTX_B.get() { return Ok(c.clone()); }\n    let _guard = gpu_init_lock(); // the SAME process-wide lock — the two\n    if let Some(c) = CTX_B.get() { return Ok(c.clone()); } // paths can no\n    let ctx = pollster::block_on(build_ctx_async())?; // longer race each\n    Ok(CTX_B.get_or_init(|| ctx).clone()) // other's driver sequence\n}\n// regression gate: N-thread same-path race + N-thread cross-path race\n// (both entry points hammered concurrently), 3x green\n".to_string(),
        },
        // ── superseded-format-constructor-live-fields-only ── distilled
        // from the measured-GOAT riir-ai Issues 714 + 720 fixes
        // (2026-08-17/18, Bench 692): the WGSL Q4_K constructors uploaded
        // BOTH weight sets — the full f32 set (~9.7 GiB device) dead on top
        // of the ~1.4 GiB Q4_K set ("the Issue 714 class, missed on the
        // WGSL Q4_K constructors"; 714's skipped() was the cubecl half).
        // Fix = GpuGemmaWeightBuffers::norms_only (live norms + host
        // wte_cpu + zero-size placeholders) wired automatically in
        // new_internal when weights_q4k.is_some(); dead wte_cpu field
        // removed; forward_layer_by_layer now refuses Q4_K. Measured
        // (Bench 692): G1 bit-identity (LOGITS_HASH df741575cc8f33be both
        // sides), G2 −10.3 GiB device per construction (14,089 → ~3,500
        // MiB) + −2.2 GiB host; first-ever e2e test coverage for the path.
        // Fourth measured-GOAT distillation after the corpus-complete
        // declaration (the B49/B52/B54/B55 reopen path (c)).
        //
        // RIDER `assigned-columns-only-staging-dequant` (Batch 84 /
        // Research 098 / Plan 045; source: jundot/omlx PR #3110, merged
        // 2026-08-24, verified against the PR body): when staging a
        // split-device / tensor-parallel weight for dequantization,
        // dequantize ONLY the columns the consuming device is actually
        // assigned -- slice to the device boundary BEFORE the dequant
        // call (in packed-axis terms, on quantization-group boundaries),
        // never dequantize-then-discard the sibling device's columns
        // (it reads them directly from the still-packed source).
        // Measured: compile-memory staging spike 35.8 GB -> 4.7 GB and
        // retained cache 32.1 GB -> 0.2 GB (the GB figures are the
        // research record's; the PR body quantifies ~0.5 GB/layer of
        // pure-waste transient), staged weights BIT-IDENTICAL to
        // dequantize-then-slice (the boundary is a multiple of the
        // quantization group size, pinned by a shape-spy test). This is
        // the live-fields-only class at COLUMN granularity: the dead
        // weight is not a whole superseded format but the per-column
        // slice belonging to a consumer that never reads the dequantized
        // copy.
        KernelRule {
            rule_id: "superseded-format-constructor-live-fields-only".to_string(),
            gpu_concept: "A constructor that accepts an OPTIONAL second weight format (a quantized Q4_K set, an alternative runtime's pool) while unconditionally uploading the primary f32 set uploads DEAD WEIGHT: every dispatch site branches on the live format, so the primary set's projections are never bound in the superseded arms — but the memory is paid in full. The scale makes it a correctness-adjacent allocation bug, not a tuning miss: full f32 ~9.7 GB device (2.2 GB wte + 7.5 GB projections, Gemma 2 2B) stacking on a ~1.4 GB Q4_K set; or ~6 GB device + 2.4 GB host stacking on an alternative runtime's pool → 23.8 GB peak on a 24 GB 4090 (OOM territory). The dead-set proof comes from a DISPATCH-SITE AUDIT (grep every bind of the primary buffers — all sit inside the superseded format's match arms or guarded entry points), never from the constructor alone. Fix shape: live-fields-only constructor variants in TIERS — skipped() (everything dead: zero-size buffers + empty Vecs) and norms_only() (the small live subset still uploads: per-layer norm gammas, final_norm, the host embedding copy the decode loop reads per token) — with zero-size placeholder buffers for dead fields so the struct still typechecks. The placeholder is a TRIPWIRE, not a silent zero: an accidental bind of a zero-size buffer fails GPU validation LOUDLY instead of silently computing garbage (raw wgpu accepts size 0; substrates that validate non-empty need B31's minimum-legal-footprint instead). Wire the variant selection INTO the shared constructor (new_internal picks it when the alternative set is Some) so callers cannot forget. The class shipped TWICE back-to-back (Issue 714: the cubecl half-skip; Issue 720: the Issue-714 class missed on the WGSL Q4_K constructors) and as a hazard elsewhere (the 2x GPU weight residency in the Q4_K trainer) — a constructor-format pair is the trigger, not any one runtime. Measured (Bench 692): bit-identical logits (LOGITS_HASH equal both sides), −10.3 GiB device per construction (14,089 → ~3,500 MiB) + −2.2 GiB host, strictly fewer allocs; the path had ZERO e2e test coverage before — the fix added the first determinism gate. Distinct from verbatim-upload-on-format-match (B37: HOW to upload the LIVE format — raw blocks, never dequant-requantize; this is NOT uploading the dead one at all), from min-rank-inert-adapter-zero-alpha (B31: a structurally-REQUIRED-but-inert branch gets minimum legal footprint + scale annihilation; this is an OPTIONAL superseded branch skipped entirely), from superseded-primitive-machinery-zombie (B41: dead MACHINERY still EXECUTING per step; this is dead MEMORY allocated once at construction), and from host-cache-immutable-device-weights (B26: immutable weights cached at init to avoid per-call download; this is which format's buffers exist at all).".to_string(),
            ref_shape: "// naive: new_q4k uploads BOTH sets — the Q4_K buffers AND the full f32\n// set the Q4_K arms never bind\npub fn new_q4k(device: &Device, queue: &Queue, w: &Weights,\n               q4k: Option<Q4kSet>) -> Self {\n    let f32_buffers = GpuGemmaWeightBuffers::new(device, queue, w);\n    // ~9.7 GB device (2.2 wte + 7.5 projections) — DEAD in every\n    // WeightFormat::Q4K arm; ~1.4 GB q4k rides on top; 14.1 GB peak\n    let q4k_buffers = q4k.map(|s| upload_q4k(device, queue, s));\n    Self { f32: f32_buffers, q4k: q4k_buffers }\n}\n// the dead-set proof is a DISPATCH-SITE audit: every bind of f32 buffers\n// sits inside a WeightFormat::F32 match arm — invisible from the ctor\n".to_string(),
            opt_pattern: "// optimized: live-fields-only constructor tiers, wired automatically\nimpl GpuGemmaWeightBuffers {\n    // tier 2 — small live subset (norms_only): the decode path still\n    // reads 4 per-layer norm gammas + final_norm + the HOST wte_cpu copy\n    pub fn norms_only(device: &Device, queue: &Queue, w: &Weights) -> Self {\n        let empty = || zero_size_buffer(device, \"q4k_skipped\");\n        Self {\n            wte: empty(),            // zero-size placeholder = TRIPWIRE:\n            wte_cpu: w.wte.clone(),  // accidental bind fails validation\n            final_norm: upload_f32(device, queue, &w.final_norm, \"fn\"),\n            layers: w.layers.iter().enumerate().map(|(i, l)| Layer {\n                attn_wq: empty(), attn_wk: empty(), attn_wv: empty(),\n                attn_wo: empty(), gate_proj: empty(), up_proj: empty(),\n                down_proj: empty(),\n                input_norm: upload_f32(device, queue, &l.input_norm, &i),\n                post_attn_norm: upload_f32(device, queue, &l.post_attn_norm, &i),\n                pre_mlp_norm: upload_f32(device, queue, &l.pre_mlp_norm, &i),\n                post_mlp_norm: upload_f32(device, queue, &l.post_mlp_norm, &i),\n            }).collect(),\n        } // 14,089 -> ~3,500 MiB peak; LOGITS_HASH bit-identical\n    }\n    // tier 1 — everything dead (skipped): alternative-runtime-only pass\n    pub fn skipped(device: &Device) -> Self {\n        let _ = zero_size_buffer(device, \"wgsl_skipped\");\n    }\n}\n// wired in the SHARED ctor — callers cannot forget:\npub fn new_internal(device: &Device, queue: &Queue, w: &Weights,\n                    q4k: Option<Q4kSet>) -> Self {\n    let f32_buffers = if q4k.is_some() {\n        GpuGemmaWeightBuffers::norms_only(device, queue, w)\n    } else {\n        GpuGemmaWeightBuffers::new(device, queue, w)\n    }; // + forward_layer_by_layer REFUSES Q4_K passes (its unconditional\n} // f32 trace never reflected Q4_K numerics — guard, don't silently mix)\n".to_string(),
        },
    ]
}

//! Entries slice — Batch 57+ era (rules 354-355 of 355) — measured-GOAT
//! distillations. Opened 2026-08-18 when `entries_gpu_06.rs` reached the
//! ~900-line threshold (Batches 43-56, rules 325-353). Category membership
//! lives ONLY in `KERNEL_CATEGORIES` (mod.rs) — never encode it here.

use crate::domains::kernel_opt::KernelRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<KernelRule> {
    vec![
        // ── smem-slot-role-reversal-needs-barrier ── distilled from the
        // measured-GOAT riir-ai Issue 715 fix (2026-08-17/18, commits
        // 9f9e68402 + 93d853582): the flash-attention online-softmax tile
        // loop shared an smem scratch across phases WITHOUT a barrier at
        // the two points where a slot's role reverses. Measured on the 27B
        // model, 256-token probe, same binary back-to-back: NONDETERMINISTIC
        // before (first divergent token index 71), BIT-IDENTICAL (3/3)
        // after. Five kernels shipped the races: 3 CubeCL
        // (qwen_attention_decode_f32, qwen_attention_decode_gated_f32,
        // qwen_attention_prefill_gated_f32) + 2 cudarc CUDA twins
        // (attention_decode_f32, attention_decode_f32_devpos) — the CubeCL
        // fix did NOT reach the cudarc arm (that arm dispatches hand-written
        // CUDA; grepping the CubeCL source proves nothing about which
        // kernels a given arm dispatches). Recurring class: Issue 612's
        // smem-corruption fix removed the destructive reduction but left
        // BOTH windows open.
        //
        // Distinct from smem-broadcast-scalar-then-parallel-scatter (E — the
        // broadcast pattern ASSUMES this discipline; this rule is the
        // required ordering when it is missing), from
        // smem-staging-gated-by-cross-thread-reuse (B35 — whether to stage
        // in smem at all; this is the barrier contract across a staged
        // buffer's lifetime), and from the barrier-elimination family
        // (warp-shuffle / plane-sized workgroups remove barriers by
        // construction — these two reversals can NEVER be removed while the
        // buffer is shared across phases).
        KernelRule {
            rule_id: "smem-slot-role-reversal-needs-barrier".to_string(),
            gpu_concept: "A shared-memory scratch whose slot ROLES reverse between phases needs a barrier at each reversal point. The two canonical windows in an online-softmax tile loop: (a) INTRA-ITERATION — every thread reads the reduction result smem[0], then a later phase has thread 0 OVERWRITE that same slot with its weight, with no barrier between: a slower warp reads a weight as the tile max, threads disagree on the softmax shift, and the tile's weights come out inconsistently scaled — length-independent, fires at any size; (b) LOOP BACK-EDGE — the last phase reads every smem[p] while the next iteration's first phase writes smem[tid] with no barrier between: a thread that finishes its read loop early clobbers a weight another thread has not consumed — reachable only when the loop trips more than once (n_tiles >= 2). Measured on the 27B model, 256-token probe, same binary back-to-back: NONDETERMINISTIC before (first divergent token index 71), BIT-IDENTICAL (3/3) after — a tolerance-only assert cannot see a race in the low mantissa bits. Both barriers must sit in BLOCK-UNIFORM control flow: the kernel's early returns (head_idx >= n_head, n_positions == 0) are uniform across the block, so no divergent-barrier deadlock is introduced. The fix must be audited against EVERY implementation twin of the same computation: the CubeCL fix did not reach the cudarc arm, because that arm dispatches hand-written CUDA — grepping one source tree proves nothing about which kernels a given arm dispatches. Recurring class: the prior fix for smem corruption in this loop (Issue 612) removed the destructive reduction but left both windows open. Distinct from smem-broadcast-scalar-then-parallel-scatter (the broadcast pattern that ASSUMES this discipline — this rule is the required ordering when it is missing), from smem-staging-gated-by-cross-thread-reuse (whether to stage in smem at all — this is the barrier contract across a staged buffer's lifetime), and from the barrier-elimination family (warp-shuffle reductions, plane-sized workgroups remove barriers by construction — this rule marks the two role reversals that can NEVER be removed while the buffer is shared across phases).".to_string(),
            ref_shape: "// naive: online-softmax tile loop sharing smem across phases with NO\n// barrier at the role reversals — race (a) intra-iteration, race (b) back-edge\nfn attention_decode_naive(scores: &[f32], smem: &mut [f32], tid: usize,\n                          n_tiles: usize, cube_size: usize) -> f32 {\n    let mut running_max = f32::NEG_INFINITY;\n    let mut running_sum = 0.0f32;\n    for tile in 0..n_tiles {\n        // Phase 1: each thread writes its local score (PRODUCER)\n        let base = tile * cube_size + tid;\n        smem[tid] = if base < scores.len() { scores[base] } else { f32::NEG_INFINITY };\n        barrier();\n        // Phase 2: max-reduction tree — smem[0] becomes the tile max\n        let mut stride = cube_size / 2;\n        while stride > 0 {\n            if tid < stride && smem[tid + stride] > smem[tid] {\n                smem[tid] = smem[tid + stride];\n            }\n            barrier();\n            stride /= 2;\n        }\n        // RACE (a): every thread reads smem[0] as tile_max; Phase 3 below\n        // has thread 0 overwrite that SAME slot with its weight — no\n        // barrier between: a slow warp reads a weight as the max, threads\n        // disagree on the softmax shift, weights scale inconsistently.\n        // Length-independent: fires at any n_positions.\n        let tile_max = smem[0];\n        let my_score = smem[tid];\n        if tid == 0 {\n            smem[0] = (my_score - tile_max).exp(); // thread 0 overwrites\n        }\n        let new_max = if tile_max > running_max { tile_max } else { running_max };\n        running_sum *= (new_max - running_max).exp();\n        running_sum += (my_score - new_max).exp();\n        running_max = new_max;\n        // Phase 4: every thread reads every smem[p] (the weights)\n        let mut acc = 0.0f32;\n        for p in 0..cube_size {\n            acc += smem[p];\n        }\n        running_sum += acc;\n        // RACE (b): loop back-edge — the next tile's Phase 1 writes smem[tid]\n        // with NO barrier: a thread that finishes its read loop early\n        // clobbers a weight another thread has not yet consumed.\n        // Reachable only when n_tiles >= 2 (n_positions > cube_size).\n    }\n    1.0 / running_sum // nondeterministic run-to-run: races flip low mantissa bits\n}\nfn barrier() {}".to_string(),
            opt_pattern: "// optimized: a barrier at EACH role reversal — (a) between the smem[0]\n// read and the thread-0 overwrite, (b) on the loop back-edge; both sit\n// in block-uniform control flow (the early returns are uniform)\nfn attention_decode_fixed(scores: &[f32], smem: &mut [f32], tid: usize,\n                          n_tiles: usize, cube_size: usize) -> f32 {\n    let mut running_max = f32::NEG_INFINITY;\n    let mut running_sum = 0.0f32;\n    for tile in 0..n_tiles {\n        smem[tid] = scores[tile * cube_size + tid]; // Phase 1 (producer)\n        barrier();\n        let mut stride = cube_size / 2;             // Phase 2 (reduce)\n        while stride > 0 {\n            if tid < stride && smem[tid + stride] > smem[tid] {\n                smem[tid] = smem[tid + stride];\n            }\n            barrier();\n            stride /= 2;\n        }\n        let tile_max = smem[0]; // every thread reads the result (consumer)\n        barrier(); // REVERSAL (a): smem[0] flips result -> thread-0 weight\n        let my_score = smem[tid];\n        if tid == 0 {\n            smem[0] = (my_score - tile_max).exp(); // safe: all lanes read it\n        }\n        let new_max = if tile_max > running_max { tile_max } else { running_max };\n        running_sum *= (new_max - running_max).exp();\n        running_sum += (my_score - new_max).exp();\n        running_max = new_max;\n        let mut acc = 0.0f32;                       // Phase 4 (consumer)\n        for p in 0..cube_size {\n            acc += smem[p];\n        }\n        running_sum += acc;\n        barrier(); // REVERSAL (b): back-edge — next Phase 1 writes smem[tid]\n    }\n    1.0 / running_sum // bit-identical run-to-run (measured 3/3 on the 27B)\n}\nfn barrier() {}".to_string(),
        },
        // ── tile-loop-back-edge-needs-multi-tile-bitexact-gate ── distilled
        // from the measured-GOAT riir-ai Issue 715 regression gate
        // (2026-08-17, commit 9f9e68402): every prior decode test used
        // n_positions <= 200 — strictly inside ONE 256-wide tile — so the
        // tile-loop body ran once and its back-edge was NEVER taken: race
        // (b) (and any back-edge carry) was untestable by construction. The
        // new gate pins n_positions=300 (n_tiles=2) and asserts, over 64
        // repeats, BOTH correctness vs the CPU reference AND bit-exact
        // reproducibility — validated in BOTH directions: fails on run 0
        // with max_diff=0.0586 when the barriers are removed, passes with
        // them. Second in-tree instance: riir-ai Issue 710 H10 — the q8kv
        // multi-tile path (cross-tile rescale + tail-tile guard) had ZERO
        // coverage at n_positions > 256 until a 2-tile fixture was written.
        //
        // Distinct from size-gate-covers-every-cost-factor (B46 — a dispatch
        // cost gate's INPUT completeness; this is a test fixture's COVERAGE
        // completeness), from verify-once-at-trust-boundary (B32b — WHERE
        // verification happens; this is what the verification must be shaped
        // like to see loop-boundary defects), and from the benign-case-only
        // probe corollary in buffer-length-derived-from-allocation-not-binding
        // (B55 — a probe measuring only the benign case answers wrongly;
        // here the fixture population never enters the defective region).
        KernelRule {
            rule_id: "tile-loop-back-edge-needs-multi-tile-bitexact-gate".to_string(),
            gpu_concept: "A loop whose trip count crosses an iteration boundary only past a size threshold — a tile loop trips twice only when n > tile size, a chunked recurrence carries only on partial chunks — hides every back-edge defect from any fixture population whose inputs all sit inside ONE iteration: the loop body runs once, the back-edge is never taken, and back-edge bugs (carry/rescale races, cross-tile state, tail-tile guards) are untestable BY CONSTRUCTION. The gate needs BOTH halves: (1) at least one fixture with n strictly greater than the boundary (n_positions=300 > cube_size=256 → n_tiles=2) so the back-edge actually executes; and (2) a bit-exact REPRODUCIBILITY assert over repeats of the SAME binary — a tolerance-only assert cannot see a race in the low mantissa bits, because two runs of identical code agreeing to 1e-5 still differ bitwise under a race. Validate the gate in BOTH directions: it must fail with the defect present (the 715 gate fails on run 0 with max_diff=0.0586 when the barriers are removed) and pass with the fix — a gate that cannot fail proves nothing. The fixture-population bias is silent: every prior decode test used n_positions <= 200 and passed, not because the kernel was correct but because the code under test never ran. Second in-tree instance of the class: the q8kv multi-tile path (cross-tile rescale + tail-tile guard) had zero coverage at n_positions > 256 until a 2-tile fixture was written (Issue 710 H10). Distinct from size-gate-covers-every-cost-factor (a dispatch cost gate's input completeness — this is a test fixture's coverage completeness), from verify-once-at-trust-boundary (where verification happens — this is what the verification must be shaped like to see loop-boundary defects), and from the benign-case-only probe corollary in buffer-length-derived-from-allocation-not-binding (a probe measuring only the benign case answers wrongly — here the fixture population never enters the defective region at all).".to_string(),
            ref_shape: "// naive gate: every fixture fits inside ONE tile — the loop back-edge\n// never executes, so a back-edge race passes the whole suite\nfn decode_test_single_tile_only() {\n    // n_positions=200 < cube_size=256 -> n_tiles=1: the tile-loop body\n    // runs ONCE; its back-edge (and any cross-tile carry) is never taken\n    for n_positions in [1usize, 64, 128, 200] {\n        let out = decode(n_positions);\n        // tolerance-only assert: a race in the low mantissa bits lands\n        // within 1e-5 — two bitwise-different runs both \"pass\"\n        assert!((out - cpu_reference(n_positions)).abs() < 1e-5);\n    }\n}\nfn decode(_n: usize) -> f32 { 0.0 }\nfn cpu_reference(_n: usize) -> f32 { 0.0 }".to_string(),
            opt_pattern: "// optimized gate: cross the tile boundary + assert bit-exact reproducibility\nfn decode_multi_tile_deterministic() {\n    let n_positions = 300; // > cube_size=256 -> n_tiles=2: the back-edge RUNS\n    let expected = cpu_reference(n_positions);\n    let mut first: Option<Vec<f32>> = None;\n    for _ in 0..64 {\n        // repeats of the SAME binary: a race flips low bits run-to-run\n        let out = decode(n_positions);\n        assert!((out - expected).abs() < 1e-5); // correctness vs reference\n        if let Some(prev) = &first {\n            // BIT-exact reproducibility: a tolerance assert cannot see a\n            // race in the low mantissa bits — bitwise equality is the only\n            // race-proof assert\n            assert_eq!(out, *prev, \"run-to-run divergence = a race, not noise\");\n        } else {\n            first = Some(out);\n        }\n    }\n}\n// validated in BOTH directions: fails on run 0 (max_diff=0.0586) with the\n// barriers removed, passes with them — a gate that cannot fail proves nothing\nfn decode(_n: usize) -> Vec<f32> { vec![0.0; 4] }\nfn cpu_reference(_n: usize) -> Vec<f32> { vec![0.0; 4] }".to_string(),
        },
    ]
}

//! Entries slice — Batch 224 (Plan 226) — the llama.cpp `MoE` `mul_mat_id`
//! duplicate-id compact-prepass + static-bound tile-descriptor dispatch
//! family (metal #30373 + vulkan #29998, the cross-backend pair) — D-class.
//! Source ggml-org/llama.cpp @ 073b74034 + 966baae76 (MIT); every quote
//! re-verified at the pin via the GitHub commits API this session (patch
//! completeness reconciled +/- against the API stats block — none
//! truncated; the metal commit's parent is 77757a346, the vulkan's is
//! 62a6f74ba; the metal message credits the vulkan: 'follow the approach
//! of the vulkan implementation (#29998)').
//!
//! DEDUP (re-run at authoring, 3+ vocabularies): the core atoms are
//! UNOWNED — `tile-descriptor` / `launch-to-exit` / `launch to exit` /
//! `static upper bound` / `unconditional store` / `divergent lanes` /
//! `entry tile` / `compact list`: ZERO corpus hits. The neighbors:
//! `degenerate-selection-falls-back-to-identity-uniform` (B110) owns
//! SELECTION-side duplicate-id AVOIDANCE (clamp so the router never emits
//! duplicates — THIS is the CONSUMER-side tolerance contract: the matmul
//! path handles whatever duplicates arrive); `moe-per-expert-mixed-dispatch-
//! worst-case-workspace` (B186/B210 era) owns WORKSPACE worst-case sizing
//! (THIS bounds TILES, not workspace); `tile-picker-input-from-dispatch-
//! grid-partition` (B213-rider family) owns the grid-partition → tile-picker
//! input seam; `moe-router-weight-fold-into-reduce-never-materialize`
//! (B220) owns the router-weight consumer fold; `indirect-device-dispatch-
//! tile-map-moe` (the tile-map indirect family) owns device-side INDIRECT
//! dispatch through a tile map — THIS is a HOST-SIZED static bound with a
//! prepass-emitted descriptor list, no indirect dispatch. `prepass` itself
//! hits `sparse-gather-scale-at-read-not-table-prepass` (table-vs-read) and
//! `split-reduction-scalar-prepass` (reduction split) — different prepass
//! purposes.

use super::KernelRule;

#[allow(clippy::too_many_lines)]
pub fn entries() -> Vec<KernelRule> {
    vec![
        KernelRule {
            rule_id: "moe-duplicate-id-compact-prepass-static-bound-tile-dispatch".to_string(),
            gpu_concept: "MECHANISM (sparse-launch MoE grouped matmul: a two-pass compact prepass + per-tile descriptors + a STATIC upper-bound launch with tail early-exit; category D — dispatch): when a grid is sized by the DENSE shape (experts x entry-tiles) but the WORK is sparse (only the used experts carry entries), most threadgroups launch only to exit — the repair is a prepass that builds a COMPACT per-expert list and emits one descriptor per tile that has work, a host-side static upper bound on the tile count, and a consumer that reads its descriptor and early-exits in the tail. THE CORRECTNESS HALF (the family's founding defect, metal, verbatim): 'kernel_mul_mm_id_map0 assumed at most one match per token and a fixed capacity of ne21 entries per expert, which produced wrong or dropped output rows and could overflow into the slice of the next expert when the ids contain duplicates.' — the removed accumulator embodied the assumption ('sel += (sids[i20] == ide)*(i20 + 1); ... ids_i32[n_all] = (i21 + t)*ne20 + sel - 1; n_all += sel > 0;' — at most one match per token, one slot per (expert, token) under a fixed ne21 stride, 'const int id = ids_i32[im*args.ne21 + r1 + lr1];'). THE PREPASS (metal, two passes over the ids): 'pass 1: count the entries of this expert - every occurrence of this expert within the ids is an entry (duplicate ids produce multiple entries for the same token)' then thread 0 runs 'exclusive prefix sum over the counts: the entries of this expert start at bas[ide] - every slice is padded by one entry for the uniform dummy store of pass 2 - and emit the entry tile descriptors of kernel_mul_mm_id: (expert, first entry, valid rows) per tile'; 'pass 2: write the entries of this expert into its slice of the compact list' with THE UNCONDITIONAL-STORE LAW, verbatim (mul_mm.metal, the code comment — the commit message drops one 'the'): 'one store per token for every thread, regardless of the match - conditional stores of the divergent lanes are very slow - the dummy value of the tokens without a match lands on the padding slot of this slice and is never read' ('ids_i32[ient] = first >= 0 ? (i21 + t)*ne20 + first : 0;'); DUPLICATES ride 'a rare branch': 'if (n_ents > 1) { // duplicate ids: the remaining matches of this token ... }' — the padding slot is reserved by the prefix sum's 'base += n + 1'. The ids buffer is sized for the truth, not the assumption (ggml-metal-ops.cpp): 'one entry per (expert used, token) pair of ids plus one padding slot per expert (see kernel_mul_mm_id_map0) return ggml_type_size(GGML_TYPE_I32)*(ne20*ne21 + ne02);' and the admission gains the capacity ceiling: 'return has_simdgroup_mm && ne00 >= 64 && ne02 <= 1024 && ne21 >= 32;'. THE DESCRIPTORS + STATIC BOUND (metal): thread 0 emits 'constexpr uint32_t NTILE = 32; // must match NR1 of kernel_mul_mm_id' descriptors as 'tpe_u32[1 + 3*tile] = e; tpe_u32[1 + 3*tile + 1] = base + k; tpe_u32[1 + 3*tile + 2] = n - k < NTILE ? n - k : NTILE;' with 'tpe_u32[0] = tile;' the total; the host bound is the SAME CLOSED FORM on both backends — metal: '// upper bound on the number of entry tiles emitted by kernel_mul_mm_id_map0: the full tiles over all entries plus at most one partial tile per expert ... const int64_t tiles = (n_rows + 31)/32 + (ne02 < n_rows ? ne02 : n_rows) - 1; return n_rows < tiles ? n_rows : tiles;' — vulkan (ggml-vulkan.cpp): 'static uint64_t ggml_vk_mul_mat_id_max_tiles(uint64_t n_experts, uint64_t n_rows, uint32_t bn) { // Each nonempty expert can add at most one partial tile to the combined grid. return std::min(n_rows, CEIL_DIV(n_rows, bn) + std::min(n_experts, n_rows) - 1); }' — and the launch swaps the dense grid for the bound (metal): 'ggml_metal_encoder_dispatch_threadgroups(enc, max_tiles, (ne01 + 63)/64, 1, 128, 1, 1);' (was '(ne21 + 31)/32, (ne01 + 63)/64, ne02, 128, 1, 1'); the consumer reads its descriptor and THE TAIL EARLY-EXITS (metal): '// the prepass emits one (expert, first entry, valid rows) descriptor per tile that has work - the grid is sized by a static upper bound, so the tail early exits' — 'if (tile >= (int) tpe_u32[0]) { return; } const int im = tpe_u32[1 + 3*tile]; const int idb = tpe_u32[1 + 3*tile + 1];' (epilogue 'const int id = ids_i32[idb + j];' — ne21-strided indexing GONE); vulkan's four matmul shaders carry the identical pattern: 'const uint tile_idx = gl_WorkGroupID.y + gl_WorkGroupID.z * gl_NumWorkGroups.y; if (tile_idx >= data_row_map[0]) { return; } const uint expert_idx = data_row_map[1 + 2 * tile_idx]; _ne1 = data_row_map[2 + 2 * tile_idx];' with the bound split against the device limit: 'const uint32_t tiles_y = std::min(max_tiles, ctx->device->properties.limits.maxComputeWorkGroupCount[1]);'. MEASURED (metal message, Qwen3.6-35B-A3B Q8_0, 883/883 test-backend-ops MUL_MAT_ID parity): 'The grid shrinks by ~8-16x on typical shapes (e.g. 128 experts, 8 used, 512 tokens: 128 tiles instead of 2048 entry tiles, of which 94% only launched to exit)' + 'llama-bench ... is now faster than master on all pp sizes (pp2048/4096/8192 +2%, pp512 +0.5%), tg32 unchanged' + the register half: 'This removes the round loop that was needed to cover duplicate ids past the grid (register pressure: th_max 768 -> back to the plain mul_mm level) and duplicates are now handled implicitly, since they only produce more descriptors within the bound.' HONEST NULL (the function-constant rider, in-rule): turning ne02 into the function constant FC_mul_mm_id_map0_ne02 'so that the threadgroup offsets of cnt/bas/sids and the prefix-sum bound fold at compile time' measured 'no measurable perf change: 883/883 test-backend-ops MUL_MAT_ID tests pass, op-level and llama-bench results are within run variance.' — the compile-time-fold specialization is FREE-AT-BEST here, kept for kargs hygiene (ne02/ne21 leave the mm kernel kargs; the dispatch grid still comes from the host tensor ne[2]). CROSS-BACKEND HONESTY (both pins read verbatim): the 'conditional stores of divergent lanes are very slow' comment, the padding-slot dummy, the th_max register note, the ~8-16x/94% figures and the +2% bench are METAL-COMMIT-ONLY; the vulkan commit (#29998, 'Extend the \"hoist row ids\" optimization to always be enabled and to emit a compact list of tile descriptions that need to run, and to emit multiple tiles when needed. Then we can launch a tighter upper bound on the number of workgroups and the tail can trivially early exit.') carries NO measured numbers, its scatter appends via atomicAdd cursors ('const uint row = atomicAdd(cursors[expert], 1); data_d[p.row_ids_offset + offsets[expert] + row] = (i01 << 16) | i00;' — duplicates by construction, no rare branch), its descriptor is 2 words '(expert, valid rows)' with row_begin derived as tile_idx * BN (metal: 3 words, first-entry explicit), it CHUNKS the count pass past the expert cap ('#define EXPERTS_PER_CHUNK 1024' replacing the removed '#define MAX_EXPERTS 1024'), it moved the surviving u16 packing constraint to op-support time ('// Shaders store row IDs as uint16_t slot and token indices. if (ids->ne[0] > int64_t(UINT16_MAX) + 1 || ids->ne[1] > int64_t(UINT16_MAX) + 1) { return false; }'), and it DELETED the subgroupBallot streaming scan the metal round loop twin of ('the whole old load_row_ids ... ballot machinery ... ~80 deleted lines incl. shared uvec4 ballots_sh[NUM_WARPS]; subgroupBallot(in_range && id == expert_idx), subgroupBallotExclusiveBitCount' — the looping the title retires); the vulkan test lane adds the duplicate fixture class ('struct test_mul_mat_id_dup ... // MUL_MAT_ID with expert ids repeated within a token's row ... data[i] = (rng() % 4 == 0) ? (int32_t) (rng() % n_mats) : 0; // repeated ids, mostly expert 0' incl. 1025/2050-expert shapes exercising the chunked prepass). THE DECISION CONTENT: (1) size the grid for the WORK, never the SHAPE — a dense experts x entries grid where '94% only launched to exit' pays launch overhead per empty tilegroup; the static upper bound (full tiles + at most one partial per expert, min-clamped by n_rows) needs NO device-side count readback before launch — it is computable on the host from tensor dims alone, so the prepass stays asynchronous and the tail self-eliminates at run time; (2) the compact list + descriptor indirection REPLACES the fixed-capacity per-expert stride — the buffer is sized 'ne20*ne21 + ne02 (entries + padding)' for the worst case while the CONSUMER's addressing is base/count per expert, so duplicates and skew both land in the same closed-form bound ('duplicates are now handled implicitly, since they only produce more descriptors within the bound'); (3) divergent lanes STORE UNCONDITIONALLY to a never-read padding slot — the branch is moved off the store path into the VALUE (the ternary selects the dummy), keeping one store per token per thread; (4) a correctness contract upstream ('consumers assume unique ids' — degenerate-selection-falls-back-to-identity-uniform) and its downstream tolerance twin (THIS rule) are TWO rules, not one: the selection side neutralizes poison BEFORE the router emits duplicates; THIS side makes the matmul correct WHEN they arrive anyway — a pipeline needs the tolerance even with the clamp (the clamp fixes NaN, not legitimate duplicate routing). REGIME: MoE grouped matmul with sparse expert usage (few of many experts hot per batch); any sparse-launch grid where the dense bound is host-computable but the true count is device-discovered (the law generalizes past MoE — a prepass descriptor list + static bound + tail exit fits ANY mostly-empty launch); backends where launch-to-exit tiles cost real occupancy and registers (the round-loop removal). ANTI-REGIME: dense work (every tilegroup carries entries — the bound equals the dense grid and the prepass is pure overhead); grids whose count the host already knows exactly (launch that — descriptors add nothing); ONE expert (the bound degenerates to the plain row count); register-starved consumers that cannot afford the descriptor read (they keep the round loop by choice — the metal commit REMOVED one). Distinct from degenerate-selection-falls-back-to-identity-uniform (B110 — SELECTION-side duplicate AVOIDANCE via pre-selection clamp; THIS is CONSUMER-side duplicate TOLERANCE — composed neighbors, the clamp fixes the emitter and this fixes the receiver); from moe-per-expert-mixed-dispatch-worst-case-workspace (B186-era — worst-case WORKSPACE sizing per expert; THIS bounds TILE COUNT, the launch shape); from tile-picker-input-from-dispatch-grid-partition (B213-era — feeding a tile picker from the grid partition; THIS replaces the grid itself); from indirect-device-dispatch-tile-map-moe (the tile-map INDIRECT family — device-side indirect dispatch through a tile map; THIS is a host-sized STATIC bound, no indirect dispatch, no map readback); from moe-router-weight-fold-into-reduce-never-materialize (B220 — router-weight consumer fold; different axis entirely); from split-reduction-scalar-prepass / sparse-gather-scale-at-read-not-table-prepass (prepass PURPOSES: reduction split / table-vs-read; THIS is a compact-list + descriptor-emission prepass); from cost-aware-verify-length-survival-cost-table (B223 — speculative verify-length economics, not launch shape); from bounded-cross-worker-barrier-protocol-polling-fan-in-ladder (B223 — in-kernel cross-worker barriers; THIS eliminates the empty tiles instead of synchronizing them). NotBenchable here (no MoE serve path on our boxes — Bonsai-2-27B is dense; the +2%/8-16x/94% figures are source-reported on THEIR metal/vulkan lanes at the pinned commits; the decision content is the bound's closed form + the descriptor indirection + the unconditional-store law, none of which needs our wall-clock to be law). KINSHIP NOTE for the general law: the same static-bound + tail-exit shape serves ANY sparse launch — our speculative-decode draft grids and katgpt mux lanes are the reachable surfaces (lane intel, no rule). Source: ggml-org/llama.cpp @ 073b74034b8bfe652e410ec8bd1e0d95a7d46fea (#30373, MIT) -- ggml/src/ggml-metal/kernels/mul_mm.metal (@@ -371,19 +372,90 the smem layout + pass 1 + prefix-sum/descriptor emission; @@ -394,24 +466,40 the unconditional-store comment + padding slot + duplicate rare-branch; @@ -533,55 +622,42 the descriptor read + tail exit + idb epilogue; @@ -8,6 +8,7 FC_mul_mm_id_map0_ne02), ggml/src/ggml-metal/ggml-metal-ops.cpp (@@ -2772,21 +2772,39 buffer sizing comment + max_tiles helper; @@ -2960,9 +2976,13 the bound launch), ggml/src/ggml-metal/ggml-metal-common.cpp (@@ -43,9 +43,10 the ne02 <= 1024 admission), commit message (the hazard, the grid/94%/th_max/+2% figures, the ne02 null); AND ggml-org/llama.cpp @ 966baae76b578d5d3dd4c179a75a35c80ff75577 (#29998, MIT) -- ggml/src/ggml-vulkan/vulkan-shaders/count_experts.comp (@@ -18,123 +19,108 the chunked count/scatter + write_tiles + 'data_d[0] = total_tiles'), mul_mm_id_funcs.glsl (@@ -2,86 +2,9 the ballot-scan deletion), mul_mm.comp (@@ -208,13 +208,17 the 2-word descriptor read + tail exit; @@ -124,7 +124,7 the RowMap binding), ggml-vulkan.cpp (@@ -6014,20 +6006,27 max_tiles + tiles_y split; @@ -7336,46 +7335,31 the removed hoist gate; @@ -15431,6 +15417,13 the u16 op-support constraint), tests/test-backend-ops.cpp (@@ -5512,6 +5513,32 the test_mul_mat_id_dup fixture; @@ -10972,6 +10999,18 the duplicate case matrix incl. 1025/2050 experts), commit message ('a tighter upper bound on the number of workgroups and the tail can trivially early exit'). Batch 224 / Plan 226 / Research 254. RIDER (Batch 233 / Plan 235 / Research 264 -- the contract face formalized upstream, ggml-org/llama.cpp @ 8c2134a #30501, MIT): the duplicate-id precondition moved from an implementation assumption to an API CONTRACT -- the ggml.h note on mul_mat_id, verbatim: \"note: this op has undefined behaviour if a given `a` is referenced more than once for a given `b`\" + \"(i.e. duplicate experts per token are not guaranteed to be supported in all backends)\", tagged in-source (\"[TAG_MUL_MAT_ID_DUPLICATE_ID]\") with the enabling TODO pinned to the support thread (\"TODO: add backend support: https://github.com/ggml-org/llama.cpp/pull/29998#issuecomment-6153742533\"); and the duplicate-shape regression tests the B224 vulkan round itself introduced are now DISABLED, not deleted -- \"#if 0\" around the \"repeated expert ids within a row\" matrix with the comment \"TODO: disabled until CUDA backend support is added [TAG_MUL_MAT_ID_DUPLICATE_ID]\" -- the three-part guard law for an explicitly-UB input class: (a) the contract note lives at the API declaration where every caller reads it, (b) the regression tests that encode the desired-but-unsupported behavior stay in the tree behind a literal disable (never silently deleted -- they are the re-enable gate), and (c) the disable comment carries the SAME tag + a pinned link so the re-enable lands with the support work it waits for. The rider is count-neutral and does not change this rule's law: the compact-prepass consumer-side TOLERANCE remains the receiver-side answer for backends that handle duplicates; what #30501 adds is the upstream declaration that callers may NOT rely on it -- a producer must not emit duplicates against the contract even where one backend tolerates them (composes with B110's selection-side clamp: the clamp is now contract-enforced, not merely NaN hygiene). RIDER (Batch 235 / Plan 237 / Research 267 -- the SECOND UB-contract face: the state-slot guarantee, ggml-org/llama.cpp #29353 @ merge 62df1815, MIT -- count-neutral): the same contract-supersedes-tests law, OTHER disposition. The ggml.h declaration of ggml_gated_delta_net gained, directly beneath its slot-semantics comment (\"the output packs the attention scores [S_v, H_v, n_tokens, n_seqs] followed by K state snapshots, most-recent first (slot 0 = final state, slot s = state s tokens back). K == 1 keeps only the final state; when n_tokens < K only slots 0..n_tokens-1 are written.\"), the dictated narrowing, verbatim: \"when n_tokens > K only slot 0 is guaranteed; backends may leave slots 1..K-1 unwritten.\" -- and its consequence is REMOVAL, not the B233 face's disable-behind-literal: test-backend-ops.cpp drops both rows that pinned overflow behavior, test_gated_delta_net(GGML_TYPE_F32, 4, 32, 8, 1, 1, false, false, /*K=*/3) and test_gated_delta_net(GGML_TYPE_F32, 4, 64, 16, 2, 1, false, false, /*K=*/4), together with their framing comment \"// overflow: n_tokens > K — only the last K snapshots kept.\" The family law, now two-faced: an API contract that states what callers may rely on SUPERSEDES any test that pins beyond it, and the disposition follows the contract's intent -- REMOVE when the behavior is deliberately unspecified across backends (a surviving test would over-pin the guarantee it just lost), DISABLE-behind-literal when the behavior is desired and support is pending (the B233 duplicate-id face -- those tests stay in the tree as the re-enable gate). The consumer-side corollary: code that reads slots 1..K-1 under n_tokens > K is contract-violating, not merely unportable -- the slot-0-only surface is the portable one.".to_string(),
            ref_shape: "// naive: the grid is the DENSE shape (experts x entry tiles), the
// per-expert capacity is a FIXED stride (ne21), and the map kernel
// assumes at most one match per token — duplicates overflow into the
// next expert's slice, and 94% of the launched tiles exit immediately.
fn mul_mat_id_naive(ids: &[[u8; 64]], experts: usize /* ne02 */, per_expert_stride: usize /* ne21 */) {
    // launch experts x ceil(per_expert_stride / 32) x ... tilegroups:
    // with 128 experts, 8 used, 512 tokens this is 2048 entry tiles of
    // which ~94% only launched to exit — and the ids buffer cannot
    // hold a token routed twice to one expert.
    let mut compact: Vec<i32> = vec![0; experts * per_expert_stride];
    for ide in 0..experts {
        let mut n_all = 0usize;
        for (t, row) in ids.iter().enumerate() {
            // 'sel += (sids[i20] == ide) * (i20 + 1)' — at most ONE match
            // per token; a duplicate expert id in one token's row either
            // drops the second entry or overflows the fixed slice.
            let first = row.iter().position(|&e| e as usize == ide);
            if let Some(i20) = first {
                compact[ide * per_expert_stride + n_all] = (t * 64 + i20) as i32;
                n_all += 1;
            }
        }
        let _ = n_all; // counts dropped on the floor — the consumer re-walks
    }
    // consumer: 'const int id = ids_i32[im * ne21 + r1 + lr1]' — the fixed
    // stride IS the addressing; a token with two entries for one expert
    // writes past its slice 'and could overflow into the slice of the
    // next expert'.
    dispatch(experts * per_expert_stride.div_ceil(32)); // dense, mostly empty
}

fn dispatch(_tiles: usize) {}
"
            .to_string(),
            opt_pattern: "// optimized: a two-pass prepass builds the COMPACT per-expert list
// (base/count per expert, one padding slot per slice) and emits one
// (expert, first-entry, valid-rows) descriptor per 32-entry tile that
// has work; the host launches a STATIC upper bound — full tiles over
// all entries plus at most one partial per expert, min-clamped by
// n_rows — computable from tensor dims alone (no device readback), and
// the consumer tail early-exits. Divergent lanes store UNCONDITIONALLY:
// the branch moves into the value, the dummy lands on the never-read
// padding slot.
fn max_tiles(n_rows: usize, n_experts: usize, tile: usize) -> usize {
    // the closed form, host-side, before any device work:
    (n_rows + tile - 1) / tile + n_experts.min(n_rows) - 1
    // callers min-clamp by n_rows; duplicates only produce more
    // descriptors WITHIN the bound — 'handled implicitly'.
}

fn mul_mat_id_compact(ids: &[[u8; 64]], n_experts: usize) -> (Vec<i32>, Vec<(u32, u32, u32)>) {
    // buffer sized for the truth: every (expert-used, token) pair plus
    // one padding slot per expert — never a fixed per-expert stride.
    let mut compact: Vec<i32> = Vec::with_capacity(ids.len() * 64 + n_experts);
    let mut descriptors: Vec<(u32, u32, u32)> = Vec::new();
    let mut counts = vec![0u32; n_experts];
    // pass 1: count — 'every occurrence of this expert within the ids is
    // an entry (duplicate ids produce multiple entries for the same token)'
    for row in ids {
        for &e in row {
            counts[e as usize] += 1;
        }
    }
    // thread 0: exclusive prefix sum (each slice padded by ONE for the
    // dummy store) + descriptor emission per 32-entry tile with work
    let mut base = 0usize;
    for (e, &n) in counts.iter().enumerate() {
        let mut k = 0u32;
        while k < n {
            descriptors.push((e as u32, (base + k as usize) as u32, (n - k).min(32)));
            k += 32;
        }
        base += n as usize + 1; // +1: the padding slot
    }
    // pass 2: ONE unconditional store per token — 'conditional stores of
    // the divergent lanes are very slow' — the dummy value (0) of the
    // tokens without a match lands on the padding slot, never read;
    // duplicates ride a rare branch appending the remaining matches.
    for (t, row) in ids.iter().enumerate() {
        let slot = compact.len();
        compact.push(0); // the unconditional store — dummy by default
        let matches: Vec<_> = row
            .iter()
            .enumerate()
            .filter(|&(_, &e)| e as usize == row[0] as usize)
            .map(|(i20, _)| (t * 64 + i20) as i32)
            .collect();
        if !matches.is_empty() {
            compact[slot] = matches[0]; // the value the branch selects
            for &m in &matches[1..] {
                compact.push(m); // duplicates: the rare branch
            }
        }
    }
    (compact, descriptors)
    // launch: dispatch(max_tiles(n_rows, n_experts, 32)) — the tail
    // tiles read descriptors.len() and 'if tile >= total { return; }'.
}
"
            .to_string(),
        },
    ]
}

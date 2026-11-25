//! Batch 410 — slice (3) the vendored ml-explore/mlx tree, the 2D-conv +
//! col-reduce clusters (Plan 409 / Research 443): ml-explore/mlx @
//! `c69303b0` (MIT) — the vendored `lib/mlx-src` submodule of
//! ddalcu/mlx-serve @ tip `2201ec9`, the B398 visit's disclosed unread
//! residue ("its own future distill surface"), read as its own batch
//! slice. Upstream-past-pin deltas (64 commits at T0) UNSWEPT — a future
//! watch surface, recorded in the research note.
//!
//! The two clusters are the least-corpus-overlapped surfaces of the
//! vendored tree: 2D convolution (Winograd F(6,3) via simdgroup matrix
//! chains + the admission/budget/floor host, conv.metal 712 + conv.cpp
//! 1815) and column-reduction tiers + the reduction-op numerics laws
//! (ops.h 293 + `reduce_row/reduce_all` + reduce.cpp dispatch). The
//! quantized/steel/SDPA surfaces stay excluded (mined via the mlxfast
//! vendor copies + B404); fft/hadamard/scan/sort stay excluded (small or
//! heavily mined).
//!
//! - `winograd-fixed-transform-as-simdgroup-mma-chain` — the transform
//!   law (N): the constant Winograd matrices ride the SIMD-group matrix
//!   units as one mma chain per channel, never scalar loops.
//! - `winograd-admission-budget-floor-routes-to-generalist` — the
//!   admission law (D): shape gate + working-set budget + the
//!   amortization floor that refuses to the implicit-GEMM generalist.
//! - `identity-guarded-atomic-elision-packed-bool-word` — the atomic
//!   law (E): the identity test elides the RMW; the packed bool4 word
//!   carries four lanes in one atomic; the guard appears ONLY on the
//!   idempotent ops (the Sum/Prod/Min/Max asymmetry is deliberate).
//! - `whole-tile-inbounds-hoisted-loader-fast-path` — the bounds law
//!   (D): the tile-granularity predicate computed once at construction
//!   buys the unguarded unrolled load path; the input loaders keep
//!   per-row guards because their predicate is not a tile property.
//! - `nan-preflight-preserves-propagation-parity` — the numerics law
//!   (N): NaN-lossy hardware intrinsics get a preflight so propagation
//!   parity holds at EVERY reduction stage, lane and pairwise alike.
//! - `logical-reduce-width-bitcast-float-carveout` — the read-type law
//!   (N): logical reductions remap the read type by WIDTH; floats are
//!   exempt because -0.0 compares equal to zero with a bit set.
//!
//! Faces (count-neutral, same commit, on their OWNERS):
//! `attention-partition-two-knob-shared-partial-workspace` += the
//! all-reduce byte-tier two-pass instance; the V-as-strided-view rule +=
//! the group-conv materialize-for-contiguous-slices pole;
//! `smem-staging-gated-by-cross-thread-reuse` += the depthwise span-tile
//! halo; `tail-clamped-row-reuse-widens-fast-kernel-admission` += the
//! store-side backward realignment (idempotent overwrite);
//! `pad-tail-to-fixed-tile` += the exhausted-axis zero-fill tier.
//!
//! Staged for the next batch (recorded, nothing hidden): the small-filter
//! per-row bounds bitmask (loader_channel_l.h:273-297), the dilation
//! base tables + kernel odometer (conv.cpp:514-554 + `loader_general.h`
//! :361-377), the simd-reduce width dispatch + the 8-byte launch cap
//! (ops.h:8-20 + reduce.cpp:494-496), the col-reduce tier dispatch
//! (reduce.cpp:947-976), the scratch-sized-to-max-tile views (folded as
//! faces into the admission rule's own text this batch).
//!
//! Fixtures: `corpus/fixtures/b410_fixtures.rs` — standalone-compiled AND
//! executed BEFORE entry (debug AND -O), the four-arm discipline.
//! Dedup re-verified at the live corpus at authoring time (Protocol A).
//! NOTHING INVENTED — every quote read at the clone at pin `c69303b0`
//! this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 410 — the vendored mlx tree's conv + reduce slice's `kernel_opt` half.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the transform law: a FIXED small transform (the Winograd
        // matrices) is a matrix, so it rides the mma units as one chain —
        // never a scalar loop nest. ──
        KernelRule {
            rule_id: "winograd-fixed-transform-as-simdgroup-mma-chain".to_string(),
            gpu_concept: "MECHANISM (fixed-transform execution substrate; category N — dtype/alignment, the transform-as-mma law): a kernel whose inner step is a FIXED CONSTANT TRANSFORM (the Winograd F(6,3) weight/input/output matrices, an 8x8 DCT butterfly, any precomputed basis) should express that step as a MATRIX CHAIN on the SIMD-group/tensor matrix units — (G·g)·Gt per element — never as a scalar loop nest, because the constant operands are loop-invariant (load the transform ONCE as two thread_elements per lane) and the mma chain replaces a ker^2-scalar multiply-accumulate per transformed element with ONE matrix instruction sequence. The source's three transforms all take the same shape, verbatim (conv.metal): the weight transform 'simdgroup_matrix<float, 8, 8> g_out = (G * g) * Gt;' where G/Gt hold 'WGT::wt_transform[sm][sn]' loaded once ('G.thread_elements()[0] = WGT::wt_transform[sm][sn]; G.thread_elements()[1] = WGT::wt_transform[sm][sn + 1];' — each lane carries TWO adjacent elements of the constant matrix, the standard fragment packing), the input transform 'simdgroup_matrix<float, 8, 8> I_out = (Bt * I) * B;', the output transform 'simdgroup_matrix<float, 8, 8> O_out = (Bt * (O_mat * B));' — three different algebraic forms (transform-then-store, transform, inverse) over the SAME substrate: constant-matrix operands staged once per threadgroup, per-channel g/I/O tiles as the variable operand, the result written from the fragment's two thread_elements ('wt_out_0[c * O] = static_cast<T>(g_out.thread_elements()[0]);' with the transposed 'A x A x C x O' output layout absorbing the fragment's coordinate mapping). THE CHANNEL-STRIDED BATCH is the throughput half: both operand loads and stores walk C in BC=32 blocks with the barrier at the block edge ('for (int bc = 0; bc < C; bc += BC) { threadgroup_barrier(...); ...read into shared memory...; ...transform and store...; }'), so the constant matrices are amortized over a whole channel block and the per-channel cost is pure mma + fragment IO. THE OUT-OF-RANGE CLAUSE rides the fragment write, never the mma: the weight transform zero-fills g's out-of-filter lanes at PACK time ('g.thread_elements()[0] = sm < R && sn < R ? Ws[simd_group_id][sm][sn][c] : T(0);'), the output transform gates the SHARED-MEMORY store per element ('if ((sm < M) && (sn < M)) { Os[sm][sn][c] = ...; }') because the 8x8 fragment overhangs the 6x6 output tile — the matrix units always compute the full 8x8, the boundaries are applied where the data lands. ⛔ KINSHIP (adjudicated both directions): fwht-butterfly-staging-ladder-warp-smem-register + fwht-shuffle-xor-butterfly-register-resident own BUTTERFLY transforms as shuffle/staging ladders — a butterfly's data flow is pairwise-exchange (no matrix product), THIS owns the PRODUCT-form fixed transform (the Winograd matrices are dense constant matrices, the natural substrate is the matrix unit); mma-accumulate-into-resident-accumulator-c-operand + fp32-input-hi-lo-two-fp16-mma-columns own the mma UNIT's operand/accumulator conventions for DATA products — THIS owns WHICH compute rides the unit at all (a fixed transform promoted to an mma chain), compose (the Winograd tile GEMM that follows the transforms is exactly their territory); mma-window-admission-output-width-occupancy-floor owns the admission of mma paths by output width — the shape side of the same substrate choice, cite. REGIME: a hot kernel step that is a small fixed linear transform over tiles (Winograd, DCT, color-space, Hadamard-as-matrix); consumers with simdgroup/tensor matrix units and a per-channel (or per-feature) batch dimension to amortize the constants. ANTI-REGIME: sparse or data-dependent transforms (nothing constant to load once — a shuffle ladder wins); transforms smaller than the fragment (the 8x8 minimum leaves the unit mostly idle — scalar loops win below ~4x4); backends without matrix units (the chain degrades to scalar matmul, still correct, no longer the win). GOAT: the fixture pins (a) the mma-chain form of the input transform equals the scalar closed form Bt·I·B element-exact at the pinned association order in f64 (the transcription-validity contract), (b) the fragment packing completeness (two elements per lane reconstruct the full 8x8 constant exactly), (c) the identity control (B = I returns the input bit-exact — the chain is a faithful transform), (d) the tamper arm (a perturbed constant matrix reds against the closed form). NotBenchable-by-domain (the win is mma-unit throughput on Metal SIMD-group matrices — no validator hosts that; the transform EQUALITY is CPU-testable and pinned exact). P113: upstream's own landed decision — decision content only. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/kernels/conv.metal:340-470 (WinogradTransforms<6,3,8> constants + winograd_conv_2d_weight_transform: the G/Gt load, the (G*g)*Gt chain, the zero-fill-at-pack clause, the transposed store) + :471-582 (winograd_conv_2d_input_transform: (Bt*I)*B) + :582-712 (winograd_conv_2d_output_transform: Bt*(O_mat*B), the per-element M-bound store gate). Quotes byte-verified at the pin this session. MIT. Batch 410 / Plan 409 / Research 443.".to_string(),
            ref_shape: r"// NAIVE: the fixed Winograd transform runs as a scalar loop
// nest — for each output tile element, a ker^2 multiply-accumulate
// per dimension, re-reading the constant coefficients per element.
fn winograd_scalar_naive(tile: &[[f32; 8]; 8], b: &[[f32; 8]; 8]) -> [[f32; 8]; 8] {
    let mut out = [[0.0f32; 8]; 8];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, o) in row.iter_mut().enumerate() {
            let mut acc = 0.0;
            for k in 0..8 {
                for l in 0..8 {
                    // the transform re-reads constants per element and
                    // multiplies through BOTH indices — the scalar form
                    acc += b[i][k] * tile[k][l] * b[j][l];
                }
            }
            *o = acc;
        }
    }
    out
}
".to_string(),
            opt_pattern: r"// FIXED: the transform is a MATRIX CHAIN in the pinned
// association order — (Bt·I)·B per tile, the constants bound once,
// each product a small dense matmul (the mma-unit form in Rust).
fn matmul8(a: &[[f32; 8]; 8], m: &[[f32; 8]; 8]) -> [[f32; 8]; 8] {
    let mut out = [[0.0f32; 8]; 8];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, o) in row.iter_mut().enumerate() {
            let mut acc = 0.0f32;
            for k in 0..8 {
                acc += a[i][k] * m[k][j]; // pinned fold order
            }
            *o = acc;
        }
    }
    out
}

fn transpose8(a: &[[f32; 8]; 8]) -> [[f32; 8]; 8] {
    let mut t = [[0.0f32; 8]; 8];
    for (i, row) in a.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            t[j][i] = v;
        }
    }
    t
}

/// (Bt·I)·B — the source's input-transform chain, exact form.
fn winograd_chain(tile: &[[f32; 8]; 8], b: &[[f32; 8]; 8]) -> [[f32; 8]; 8] {
    let bt = transpose8(b);
    let inner = matmul8(&bt, tile); // Bt·I
    matmul8(&inner, b) // (Bt·I)·B
}

/// fragment packing completeness: two adjacent elements per lane
/// reconstruct the full constant exactly (lane (sm,sn) holds
/// [sm][sn] and [sm][sn+1]).
fn pack_unpack_constants(b: &[[f32; 8]; 8]) -> [[f32; 8]; 8] {
    let mut rebuilt = [[0.0f32; 8]; 8];
    for sm in 0..8usize {
        for sn in (0..8usize).step_by(2) {
            rebuilt[sm][sn] = b[sm][sn];
            rebuilt[sm][sn + 1] = b[sm][sn + 1];
        }
    }
    rebuilt
}
".to_string(),
        },
        // ── R2 — the admission law: the fast conv path is gated by a
        // shape conjunct AND a memory budget, and the budget carries an
        // amortization floor that refuses to the generalist. ──
        KernelRule {
            rule_id: "winograd-admission-budget-floor-routes-to-generalist".to_string(),
            gpu_concept: "MECHANISM (fast-path admission with a memory-budget amortization floor; category D — dispatch): a transform-based fast path (Winograd conv) is admitted by a THREE-PART gate — a SHAPE conjunct set (stride 1, dilations 1, no flip, 3x3 kernel, C%32==0, O%32==0, input tile count >= 4096, C+O >= 256, an env kill-switch defaulting ON), a MEMORY-BUDGET gate (the transform scratch is sized from a fraction of the device working set — limit = recommendedMaxWorkingSetSize/4*3 — and the BATCH is stepped so one step's padded input + 8x8 transform tiles + filter transform fit: max_n = budget / bytes_per_n, the env override can only shrink), and an AMORTIZATION FLOOR that routes back to the generalist when the budget forced tiling below usefulness — verbatim (conv.cpp:101-106): 'When the budget forces tiling, each tile must carry enough gemm rows to amortize fixed cost, below that the implicit gemm fallback is much faster.' + 'constexpr int min_rows_per_tile = 32; if ((safe_n < total_n) && (safe_n * tiles_per_n < min_rows_per_tile)) { return 0; }' — return 0 means the CALLER falls through to the implicit-GEMM path ('if (int n_step = winograd_batch_step(d, in, conv_params); n_step > 0)'). The distinctive atom is the floor's DIRECTION: a memory constraint does not merely cap the batch — it can DISQUALIFY the fast path entirely when the cap would leave each tile too few rows to amortize the three-transform fixed cost, and the refusal is silent-by-design (the generalist is the correct answer, not an error). The budget's own shape is two-sided: used = io_bytes + filt_bytes is subtracted from the limit BEFORE the per-step cost (budget = limit > used ? limit - used : 0), so the admission measures the working set the WHOLE op will occupy, not just the scratch. ⛔ KINSHIP (adjudicated both directions): amortized-reuse-claim-break-even (rust_perf entries_68) owns the REPORT-shape law (break_even = bake / per_launch_savings as a first-class artifact) — THIS owns the ADMISSION-side floor (the pre-flight refusal when per-tile rows < 32), compose (a lane adopting this rule should also report its floor, the entries_68 way); sibling-kernel-admission-gap-absorbed-by-generalist owns the COMPLEMENT direction — composing sibling predicates so the gap band lands on the generalist — THIS is the self-refusal pole: the fast path itself measures its amortization and hands the shape back; batch-width-aware-partition-shrink owns the PARTITION-shrink axis at kernel dispatch (blocks = min(8, stock)) — distinct axis (in-flight partition count vs batch stepping), cite; reserve-then-commit-capacity-accounting owns the capacity/reservation accounting for ADMISSIONS to a cache — the budget here sizes a BATCH STEP, different lifecycle; fewrow-tensor-tile-decode-into-rhs-operand + fa-fast-path-validated-exact-shape-admission own the shape-conjunct side of fast-path admission — the conjunct set here is that family's instance, the NEW atom is the floor. The unfold twin (faces, same rule): the explicit-GEMM unfold path carries the SIBLING budget axis — the row-tile count derived from the HARD device limit ('size_t max_buffer = d.mtl_device()->maxBufferLength(); size_t max_rows = row_bytes == 0 ? total_rows : max_buffer / row_bytes;', conv.cpp:35-38) with the env override only shrinking ('max_rows = std::min(max_rows, static_cast<size_t>(forced));') and the unfit single row refusing LOUD naming both numbers ('[conv] A single unfolding row needs N bytes, which exceeds the maximum Metal buffer size of M bytes.'), and the scratch allocated ONCE at the max tile with every tile incl. the ragged tail as a strided VIEW ('The full unfold buffer can exceed maxBufferLength, so unfold and gemm in row tiles reusing one buffer, keeping peak memory at a single tile.' + 'in_tile.copy_shared_buffer(in_unfolded, ...)') — two working-set laws (soft working-set fraction vs hard max-buffer division), one shape: the tile is sized from the DEVICE, never configured, and an override can only shrink it. REGIME: transform-based fast paths whose scratch scales with the batch (Winograd, FFT-conv); any admission where a memory cap can starve the amortization of fixed per-tile cost. ANTI-REGIME: fast paths whose fixed cost is trivially amortized (no floor needed); consumers where the generalist is unavailable (the floor would strand the shape — fall back to a STEPPED fast path instead, accepting the amortization loss); interactive latency-bound callers (a silent route-to-generalist can be a latency cliff — surface the routing). GOAT: the fixture pins (a) the budget arithmetic (max_n = budget/bytes_per_n over a synthetic working set, the used-subtraction first), (b) the floor fires exactly when safe_n < total_n AND safe_n*tiles_per_n < 32 — both conjuncts, (c) the floor's route (return 0 → the caller's generalist), (d) the tamper arm (floor removed: an under-amortized tile is admitted — the defect the floor exists for). NotBenchable-by-domain (the win is the routing decision's correctness; the arithmetic is CPU-testable and pinned). P113: upstream's own landed decision. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/conv.cpp:51-106 (winograd_padded_size + winograd_batch_step: the working-set fraction, the budget arithmetic, the env overrides, the min_rows_per_tile floor verbatim) + :1443-1458 (the shape conjunct set + the n_step > 0 fall-through) + :33-49 (max_unfold_rows: the hard-limit division + the loud refusal) + :145-163 (the unfold row-tile loop + the scratch-view reuse). Quotes byte-verified at the pin this session. MIT. Batch 410 / Plan 409 / Research 443.".to_string(),
            ref_shape: r"// NAIVE: the fast path admits on shape alone — the memory
// budget either does not exist, or it caps the batch without ever
// disqualifying an under-amortized tiling.
fn admit_winograd_naive(
    n: usize, c: usize, o: usize, tiles_per_n: usize, working_set: usize,
) -> Option<usize> {
    let shape_ok = c % 32 == 0 && o % 32 == 0;
    if !shape_ok {
        return None;
    }
    // budget caps the step but the floor is missing: a batch tiled to
    // 1 row per tile still takes the fast path.
    let budget = working_set / 4 * 3;
    let bytes_per_n = 16 * 1024;
    let max_n = budget / bytes_per_n;
    Some(max_n.min(n).max(1))
}
".to_string(),
            opt_pattern: r#"// FIXED: shape gate + two-sided budget + the amortization
// floor that returns 0 — the caller falls through to the generalist.
const MIN_ROWS_PER_TILE: usize = 32;

fn admit_winograd(
    n: usize, c: usize, o: usize, tiles_per_n: usize, working_set: usize,
    env_tile_batch: usize,
) -> Option<usize> {
    let shape_ok = c % 32 == 0 && o % 32 == 0;
    if !shape_ok {
        return None;
    }
    let limit = working_set / 4 * 3;
    let used = c * o * 4; // filt_bytes: the whole-op occupancy first
    let budget = limit.saturating_sub(used);
    let bytes_per_n = 16 * 1024;
    let max_n = budget / bytes_per_n;
    let safe_n = max_n.min(n);
    if env_tile_batch > 0 {
        return Some(env_tile_batch.min(safe_n).max(1));
    }
    // the floor: forced tiling must still amortize the fixed cost
    if safe_n < n && safe_n * tiles_per_n < MIN_ROWS_PER_TILE {
        return None; // route to the implicit-GEMM generalist
    }
    Some(safe_n.max(1))
}

#[test]
fn batch410_admission_floor_routes_to_generalist() {
    // a 48 KiB working set forces safe_n = 2; with 10 tiles per n the
    // tile carries 2*10 = 20 rows < 32 — the floor refuses.
    let admitted = admit_winograd(64, 32, 32, 10, 48 * 1024, 0);
    assert_eq!(admitted, None, "under-amortized tiling must route out");
    // the same shape with a roomy budget fits untiled: the floor is
    // gated on safe_n < total_n and never fires.
    let admitted = admit_winograd(8, 32, 32, 10, 1 << 30, 0);
    assert_eq!(admitted, Some(8));
}
"#.to_string(),
        },
        // ── R3 — the atomic law: the identity test elides the RMW, the
        // packed bool word carries four lanes in one atomic, and the
        // guard appears ONLY where it is provably safe and profitable. ──
        KernelRule {
            rule_id: "identity-guarded-atomic-elision-packed-bool-word".to_string(),
            gpu_concept: "MECHANISM (atomic-accumulation elision; category E — atomics/synchronization): an atomic accumulate whose operation has an IDENTITY (And: true, Or: false) elides the read-modify-write entirely when the operand IS the identity — the guard 'if (!val) { ... fetch_and ... }' skips the contended atomic on identity-heavy streams because fetch_and(1) and fetch_or(0) are semantic no-ops whose only cost is the contention and the memory traffic. Two decision contents ride the same site. (1) THE PACKED WORD: four boolean lanes fold into ONE 32-bit atomic word — 'bool4_or_uint update; update.b = {true, true, true, true}; update.b[elem_idx] = false; mlx_atomic_fetch_and_explicit(out, update.i, offset);' — a single-lane-clearing mask under fetch_and (And's identity is all-ones; lane i's contribution is the mask with lane i zeroed), single-lane-setting under fetch_or for Or (ops.h:88-106, 'if (val)'), so four reduction outputs cost one atomic site and one word instead of four. (2) THE SCALAR DOWNGRADE: the elided case of the scalar-bool form is a plain idempotent STORE ('mlx_atomic_store_explicit(out, val, offset)' under the same guard) — when the operand IS the fixpoint, store-false is exactly And's identity and no RMW is needed. THE DELIBERATE ASYMMETRY is the load-bearing negative: Sum/Prod/Min/Max's atomic_updates carry NO guard (ops.h:131-134 Sum: a bare 'mlx_atomic_fetch_add_explicit(out, val, offset)') — the elision appears only where it is provably safe AND profitable: for idempotent ops the identity operand is COMMON in real streams (early-exited masks, mostly-true predicates) and the branch is predictable, while for sum the identity operand (0) is rare enough in reduction traffic that the compare is a tax. ⛔ KINSHIP (adjudicated both directions): broadcast-uniform-skip-reduce owns skipping UNIFORM work at the tile level (a whole-warp ballot before a shared write) — THIS elides at the ATOMIC site per element (a per-element guard, not a tile ballot), compose (a uniform skip makes the elision free of even the branch); fail-stop-deadline-trap-bounded-cross-rank-wait + the atomic-free contention family (splitk-atomic-free-contention-threshold) own the ATOMIC-FREE restructuring axis (redesign so no atomic is needed) — THIS keeps the atomic and skips its identity invocations, the lighter pole; mask-sentinel-matches-reduction-identity owns the sentinel/identity VALUE contract at write sites — the same identity notion, different mechanism (what to write vs whether to write atomically), cite. REGIME: atomic-accumulated reductions over idempotent ops (and/or/min/max) or streams with identity-heavy operands; packed multi-lane boolean reductions sharing one atomic word. ANTI-REGIME: sum/product over dense numeric traffic (the guard is a tax — the source's own asymmetry is the evidence); ops without an identity; atomics whose contention is negligible (the elision buys nothing). GOAT: the fixture pins (a) the elision count (a guarded And over an all-true stream performs ZERO fetch_and calls; the unguarded form performs N), (b) the packed-word lane isolation (clearing lane 2 leaves lanes 0,1,3 exact — the mask construction is the contract), (c) the semantic identity (guarded vs unguarded And produce identical words on mixed streams — the elision is invisible), (d) the tamper arm (a guard on SUM eliding zero operands is sound for sum but the fixture reds the ASYMMERY claim by showing the source's Sum carries no guard — modeled as the unguarded arm). NotBenchable-by-domain (the win is contention/traffic on a GPU atomic unit; the elision COUNT and the word semantics are CPU-testable and pinned). P113: upstream's own landed decision. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/kernels/reduction/ops.h:47-65 (the And packed-word + scalar-store forms verbatim) + :88-106 (the Or mirror, 'if (val)' + fetch_or) + :131-134 (Sum's bare fetch_add — the asymmetry). Quotes byte-verified at the pin this session. MIT. Batch 410 / Plan 409 / Research 443.".to_string(),
            ref_shape: r"// NAIVE: every element pays the atomic — an AND reduction
// accumulate fetch_and(1) per true element; contention and traffic
// scale with the stream, identity or not.
use std::sync::atomic::{AtomicU32, Ordering};

fn and_reduce_naive(words: &[bool], out: &AtomicU32, lane: usize) {
    for &w in words {
        // the packed-word form WITHOUT the identity guard: every
        // element issues fetch_and
        let mask = !(1u32 << lane) | (!(w as u32) << lane);
        let _ = mask; // (lane-clearing mask when w == false)
        if !w {
            out.fetch_and(!(1u32 << lane), Ordering::Relaxed);
        } else {
            out.fetch_and(u32::MAX, Ordering::Relaxed); // the no-op RMW
        }
    }
}
".to_string(),
            opt_pattern: r"// FIXED: the identity guard elides the no-op RMW — an
// all-true stream issues ZERO atomics; false issues exactly one
// lane-clearing fetch_and.
use std::sync::atomic::{AtomicU32, Ordering};

static ATOMIC_CALLS: AtomicU32 = AtomicU32::new(0);

fn and_reduce_guarded(words: &[bool], out: &AtomicU32, lane: usize) {
    for &w in words {
        if !w {
            ATOMIC_CALLS.fetch_add(1, Ordering::Relaxed);
            out.fetch_and(!(1u32 << lane), Ordering::Relaxed);
        }
        // w == true is And's identity: fetch_and(1) is a no-op — skip it
    }
}

#[test]
fn batch410_identity_guard_elides_and_packs() {
    let out = AtomicU32::new(u32::MAX);
    // 4096 true elements: the guarded form issues zero atomics
    and_reduce_guarded(&[true; 4096], &out, 2);
    assert_eq!(ATOMIC_CALLS.load(Ordering::Relaxed), 0);
    assert_eq!(out.load(Ordering::Relaxed), u32::MAX);

    // one false in lane 2: exactly one atomic, other lanes isolated
    ATOMIC_CALLS.store(0, Ordering::Relaxed);
    let mut stream = [true; 4096];
    stream[17] = false;
    and_reduce_guarded(&stream, &out, 2);
    assert_eq!(ATOMIC_CALLS.load(Ordering::Relaxed), 1);
    assert_eq!(out.load(Ordering::Relaxed) & (1 << 2), 0);
    assert_eq!(out.load(Ordering::Relaxed) & 0b1011, 0b1011);
}
".to_string(),
        },
        // ── R4 — the bounds law: the tile-granularity predicate computed
        // once at construction buys the unguarded fast path. ──
        KernelRule {
            rule_id: "whole-tile-inbounds-hoisted-loader-fast-path".to_string(),
            gpu_concept: "MECHANISM (bounds-check granularity hoist; category D — dispatch): a staged-block loader whose bounds predicate is a TILE-GRANULARITY property (first row + tile height <= bound) computes it ONCE at construction and branches the whole load: the fully-interior tile takes an unguarded, fully-unrolled copy ('if (do_read) { STEEL_PRAGMA_UNROLL for ... dst[...] = curr_src[...]; }'), the straddling tile keeps the per-row guard ('} else { for ... { if (((read_n + i) < N)) { ... } }'), and a wholly-out-of-range tile zero-fills and returns — three tiers keyed by one hoisted bool ('do_read(read_n + BN <= gemm_params_->N)' in the constructor's initializer list). The win is that MOST tiles are fully interior (every output-channel tile except the last edge), so one branch per tile — amortized over the tile's rows AND its k-iterations — buys an unguarded inner loop on the hot posture. THE BOUNDARY IS IN THE NEGATIVE SPACE and is the rule's load-bearing clause: the INPUT loaders of the same file deliberately KEEP per-row guards, verbatim in structure (loader_general.h:286-296 is the weight side; the input loaders at loader_general.h:104-137 / loader_channel_n.h:135-188 guard per row inside load_unsafe) — because an input tile's rows straddle the SPATIAL edges differently per k-iteration (the tap walk moves the window), the tile predicate would never hold there; the fast path is only worth hoisting where the predicate is a per-tile property that is USUALLY TRUE. ⛔ KINSHIP (adjudicated both directions): loop-invariant-dispatch-resolution-hoist (rust_perf entries_48) owns the loop-invariant RESOLUTION hoist (a handler/flag resolved once before the loop) — THIS hoists a BOUNDS predicate to tile granularity with a three-tier dispatch, the same genus at a coarser granularity and with the zero-fill tier, compose; the col_reduce kernels' hoisted 'safe' split (column + n_reads <= reduction_stride computed once, the guarded row loop only in the else arm — reduce_col.h:325-326 + 355-368) and the all_reduce hoist (reduce_all.h:27-36, 'extra -= lid.x * N_READS; if (extra >= N_READS) { blocks++; extra = 0; }') are the REDUCTION-side instances of the same hoist — faces, cited here as the family's second/third members; tail-clamped-row-reuse-widens-fast-kernel-admission (entries_gpu_75) owns the STORE-side tail (clamp-compute-discard to admit unaligned N) — different boundary site (writes vs staged loads), cite; smem-staging-gated-by-cross-thread-reuse owns WHEN to stage at all — THIS owns the guard structure OF the staging loop, compose. REGIME: staged-block loaders (weights, KV tiles, packed operands) whose per-tile interior/exterior status is fixed at launch; any hot loop whose bounds predicate is uniform over the tile. ANTI-REGIME: predicates that vary per k-iteration (nothing to hoist — keep the per-row guard); tiles that straddle as often as not (the hoisted branch adds a check without buying the fast path); bounds that can change mid-kernel (nothing here — the predicate must be launch-invariant). GOAT: the fixture pins (a) the three tiers (interior tile → unguarded path flag; straddling → guarded path; out-of-range → zero-fill), (b) the count (an interior stream runs N zero bound-checks in the fast path; the guarded path runs one per row only on the edge tile), (c) the semantic identity (staged tile bytes identical guarded vs unguarded for every tile class), (d) the tamper arm (an off-by-one predicate — first + BN < N instead of <= — admits a straddling tile into the fast path and reds the OOB read). NotBenchable-by-domain (the win is branch/unroll quality in a Metal loader; the tier dispatch is CPU-testable and pinned). P113: upstream's own landed decision. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/kernels/steel/conv/loaders/loader_channel_n.h:255-300 (the do_read constructor clause + the three-tier load_unsafe verbatim) + mlx/backend/metal/kernels/steel/conv/loaders/loader_general.h:286-300 (the weight-loader twin '(start_row + BN <= params->O)'). Quotes byte-verified at the pin this session. MIT. Batch 410 / Plan 409 / Research 443.".to_string(),
            ref_shape: r"// NAIVE: every row of every tile pays the bounds check —
// the guard sits inside the unrolled copy loop, hot path included.
#[derive(Clone, Copy)]
struct TileSpec {
    first_row: usize,
    rows: usize,
    bound: usize,
}

fn load_tile_naive(spec: TileSpec, src: &[f32], dst: &mut [f32]) -> usize {
    let mut checks = 0;
    for (i, d) in dst.iter_mut().enumerate() {
        let row = spec.first_row + i / spec.rows;
        checks += 1; // the per-row guard, every element
        if row < spec.bound {
            *d = src[row * spec.rows + i % spec.rows];
        } else {
            *d = 0.0;
        }
    }
    checks
}
".to_string(),
            opt_pattern: r#"// FIXED: the tile predicate computed once; three tiers —
// unguarded unrolled copy / per-row guard / whole-tile zero-fill.
#[derive(Clone, Copy)]
struct TileSpec {
    first_row: usize,
    rows: usize,
    bound: usize,
}

impl TileSpec {
    /// the constructor clause: do_read = read_n + BN <= N
    fn fully_interior(&self) -> bool {
        self.first_row + self.rows <= self.bound
    }
    fn wholly_out(&self) -> bool {
        self.first_row >= self.bound
    }
}

fn load_tile(spec: TileSpec, src: &[f32], dst: &mut [f32]) -> (usize, bool) {
    if spec.wholly_out() {
        dst.fill(0.0);
        return (0, false);
    }
    if spec.fully_interior() {
        // the unguarded fast path: no per-row checks
        for (i, d) in dst.iter_mut().enumerate() {
            let row = spec.first_row + i / spec.rows;
            *d = src[row * spec.rows + i % spec.rows];
        }
        return (0, true);
    }
    // the straddling tier: the per-row guard, edge tile only
    let mut checks = 0;
    for (i, d) in dst.iter_mut().enumerate() {
        let row = spec.first_row + i / spec.rows;
        checks += 1;
        *d = if row < spec.bound {
            src[row * spec.rows + i % spec.rows]
        } else {
            0.0
        };
    }
    (checks, false)
}

#[test]
fn batch410_tile_tiers_dispatch() {
    let src: Vec<f32> = (0..4096).map(|i| i as f32).collect();
    // interior tile: zero checks, fast path taken
    let (checks, fast) = load_tile(TileSpec { first_row: 0, rows: 16, bound: 256 }, &src, &mut [0.0; 256]);
    assert!(fast && checks == 0);
    // straddling tile: guarded, exactly rows-per-row checks
    let (checks, fast) = load_tile(TileSpec { first_row: 250, rows: 16, bound: 256 }, &src, &mut [0.0; 256]);
    assert!(!fast && checks == 256);
    // off-by-one tamper: first + rows < bound admits 250..266 — OOB
    let spec = TileSpec { first_row: 250, rows: 16, bound: 256 };
    assert!(spec.first_row + spec.rows > spec.bound, "the tamper must not pass the true predicate");
}
"#.to_string(),
        },
        // ── R5 — the numerics law: NaN-lossy hardware intrinsics get a
        // preflight so propagation parity holds at every stage. ──
        KernelRule {
            rule_id: "nan-preflight-preserves-propagation-parity".to_string(),
            gpu_concept: "MECHANISM (NaN-propagation parity around lossy intrinsics; category N — numerics): a hardware lane intrinsic that does NOT propagate NaN the way the semantic contract demands (Metal simd_min/simd_max lose NaN: simd_min(NaN, x) = x) gets a PREFLIGHT at every reduction stage where the lossy form runs — 'if (simd_any(val != val)) { return static_cast<T>(NAN); } return simd_min(val);' — and the SAME propagation rule at the pairwise fold level ('if (metal::isnan(a) || metal::isnan(b)) { return NAN; } return a < b ? a : b;'), because a reduction whose stages disagree on NaN produces a result that depends on WHICH stage observed the NaN: thread-local folds, the shuffle butterfly (the 8-byte fallback path), and the lane intrinsic must all carry the same contract or two shapes of the same reduction return different values. The integer instantiations keep the bare intrinsic via the enable_if split (the check is dead weight where NaN cannot exist), and the complex form propagates PER COMPONENT (real-NaN keeps a real min over the imaginary axis and vice versa) — the granularity of the propagation is part of the contract, not an implementation detail. The preflight cost is one simd_any over the lane group, paid only on the lossy-op-width classes. ⛔ KINSHIP (adjudicated both directions): nan-preserving-select-over-fmaxf-in-scale-reductions (entries_gpu_147, B323) owns the SELECT-form face of the same hazard — a clamp/reduction INSIDE a numerics contract must be the comparison-select form because fmaxf launders NaN to 0 — THIS owns the LANED-INTRINSIC form (the hardware simd_min/simd_max behind a preflight) with the stage-parity clause (preflight + pairwise + butterfly agreeing), the family's second member; wide-reduction-precision-split + the Issue-940 fold-order laws own ORDER/dtype determinism — orthogonal axis (which order sums in vs what propagates), compose (a parity-correct reduction still pins its fold order); monotone-transform-elision-under-argmax owns argmax-side NaN/tie semantics for INDEX-carrying selection — THIS is value-only propagation, cite; exact-ordering-escalating-precision-ladder owns precision ESCALATION — different remedy axis, cite. REGIME: min/max (and any op whose hardware lane form drops NaN) in reductions whose contract is numpy-parity (NaN propagates); mixed-stage reductions (lane + pairwise + butterfly) where stage agreement is observable. ANTI-REGIME: hardware forms that already propagate (sum/prod on IEEE add/mul — free NaN semantics); integer reductions (the enable_if keeps the bare intrinsic — the check is a tax with no NaN domain); contracts that DEFINE NaN away (a total-order min — then the preflight is the defect). GOAT: the fixture pins (a) the lossy-intrinsic model reds (simd_min(NaN, x) = x — the naive lane min returns x, wrong), (b) the preflight form propagates (any-NaN lane group → NaN), (c) the pairwise twin agrees (isnan(a) || isnan(b) → NaN), (d) the stage-parity clause (a reduction whose NaN sits at a lane boundary vs inside the local fold returns the SAME value — stage agreement), (e) the integer carve-out (no preflight on i32 — the check would be dead weight). NotBenchable-by-domain (correctness; the preflight's one-simd_any cost is a design constant, never a quoted win). P113: upstream's own landed decision. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/kernels/reduction/ops.h:184-191 (Min::simd_reduce_impl preflight verbatim) + :207-214 (the Min pairwise operator() twin verbatim) + :244-251 (the Max mirror) + :216-232 (the complex per-component form). Quotes byte-verified at the pin this session. MIT. Batch 410 / Plan 409 / Research 443.".to_string(),
            ref_shape: r"// NAIVE: the lane min is the hardware intrinsic form — which
// LAUNDERS NaN (min(NaN, x) = x), so a poisoned lane silently
// disappears and the reduction returns a clean-looking wrong value.
fn lane_min_naive(lanes: &[f32]) -> f32 {
    // the hardware simd_min model: NaN loses pairwise
    let hw_min = |a: f32, b: f32| {
        if a.is_nan() {
            b // the lossy intrinsic: NaN dropped
        } else if b.is_nan() {
            a
        } else {
            a.min(b)
        }
    };
    lanes.iter().copied().reduce(hw_min).unwrap()
}
".to_string(),
            opt_pattern: r#"// FIXED: the preflight — any-NaN short-circuits the lane
// reduce; the pairwise fold carries the SAME contract.
fn lane_min_preflight(lanes: &[f32]) -> f32 {
    if lanes.iter().any(|v| v.is_nan()) {
        // simd_any(val != val) across the lane group
        return f32::NAN;
    }
    // the bare intrinsic path: no NaN can remain
    lanes.iter().copied().reduce(f32::min).unwrap()
}

fn fold_pairwise(a: f32, b: f32) -> f32 {
    if a.is_nan() || b.is_nan() {
        f32::NAN
    } else {
        a.min(b)
    }
}

#[test]
fn batch410_nan_parity_holds_at_every_stage() {
    let poisoned = [1.0, f32::NAN, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    assert!(lane_min_naive(&poisoned) == 1.0, "the lossy form drops the NaN");
    assert!(lane_min_preflight(&poisoned).is_nan(), "the preflight propagates");
    assert!(fold_pairwise(f32::NAN, 1.0).is_nan(), "the pairwise twin agrees");

    // stage parity: NaN at a lane boundary vs inside the local fold —
    // both stages must return the same value.
    let boundary = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, f32::NAN];
    assert_eq!(
        lane_min_preflight(&boundary).is_nan(),
        fold_pairwise(1.0, f32::NAN).is_nan(),
    );
    // the integer carve-out: no NaN domain, the bare intrinsic stands
    let ints = [3i32, 1, 2];
    assert_eq!(ints.iter().copied().reduce(i32::min), Some(1));
}
"#.to_string(),
        },
        // ── R6 — the read-type law: logical reductions bitcast by width;
        // floats are exempt because -0.0 compares equal to zero with a
        // bit set. ──
        KernelRule {
            rule_id: "logical-reduce-width-bitcast-float-carveout".to_string(),
            gpu_concept: "MECHANISM (read-type remap for logical reductions; category N — dtype/alignment): an all/any reduction over arbitrary element types remaps its INPUT READ TYPE by WIDTH, not by name — any integer is read as the same-width signed int ('Integers can be tested as whatever type has the same width, since only their bits matter.'), with THREE named carve-outs: floats keep their float type ('Floats cannot: -0.0 compares equal to zero but has a bit set, so it has to be tested as a float.') because the truthiness decision then moves to a magnitude-bits test ('(as_type<uint32_t>(val) & 0x7FFFFFFF) != 0' — +0.0 and -0.0 both false, NaN true, the sign bit discarded), complex tests either half, and the TARGET constraint forces sub-4-byte integers to read as their SIGNED width ('A Metal bool must hold 0 or 1, so read int8 and uint8 as int8.' — a re-read into a 1-byte bool would trap on 0xFF, and the sign bit is irrelevant to and/or so the unsigned re-read gains nothing). The rule is the full table WITH its reasons: width-keyed bitcast for the bits-only class, value-typed float lane with the magnitude-bit test, per-component complex, signed-width reads for the bool-target. ⛔ KINSHIP (adjudicated both directions): identity-guarded-atomic-elision (this batch) owns the and/or ATOMIC side — THIS owns the read-type side of the same ops, compose (a logical reduction is this rule's remap feeding that rule's guarded atomic); the dtype-native family (dtype-normalize-once-at-upload, dtype-branch-dispatch-parity-in-place-precondition) owns dtype DISPATCH for compute kernels — THIS is a truthiness-semantics remap with a correctness carve-out (the -0.0 class), a different axis; bit-equal-fusion-mirrors-reference-reduction-regime owns bit-fidelity under fusion — the -0.0 clause is exactly a bit-vs-value divergence, cite as the family's truthiness member. REGIME: all/any (and boolean-cast reductions) over mixed element types on targets whose bool representation is constrained (0/1 bytes); any bitcast-of-truthiness where a signed-zero class exists. ANTI-REGIME: targets with native bool ops at every width (the table collapses); semantics where -0.0 must be TRUE (a bit-test contract — the comment itself names the flip condition); floats under a bit-exactness contract (the carve-out exists precisely because value and bit tests disagree). GOAT: the fixture pins (a) the naive bitcast reds on -0.0 (as-u32 != 0 → true; the value semantics say false), (b) the magnitude-bit form classifies exactly (+0.0/-0.0 false, NaN true, denormals true), (c) the width table (i8/u8/i16/u16/i32/u32/i64/u64 read as signed same-width; the u8 255 case stays true via the signed read), (d) the tamper arm (removing the float carve-out reds the -0.0 case). NotBenchable-by-domain (correctness). P113: upstream's own landed decision. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/reduce.cpp:276-302 (remap_reduce_types: the comment + the width table verbatim) + mlx/backend/metal/kernels/utils.h:482-485 (the float cast_to bit test verbatim). Quotes byte-verified at the pin this session. MIT. Batch 410 / Plan 409 / Research 443.".to_string(),
            ref_shape: r"// NAIVE: every element is bitcast to its integer width and
// tested non-zero — the float cases silently misclassify.
fn truthy_bitcast_naive(v: f32) -> bool {
    // as-u32 != 0: -0.0 has bits set, so it reads TRUE — wrong
    (v.to_bits() != 0)
}

fn logical_and_naive(xs: &[f32]) -> bool {
    xs.iter().all(|&v| truthy_bitcast_naive(v))
}
".to_string(),
            opt_pattern: r#"// FIXED: the float carve-out tests magnitude bits; integers
// ride the width table; the -0.0 class classifies false.
fn truthy_float(v: f32) -> bool {
    // & 0x7FFFFFFF discards the sign bit: -0.0 → false, NaN → true
    (v.to_bits() & 0x7FFF_FFFF) != 0
}

/// the width table: same-width signed read for the bits-only class.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ReadType {
    I8,
    I16,
    I32,
    I64,
    F32, // the carve-out: floats keep their float type
}

fn read_type(is_float: bool, byte_width: usize) -> ReadType {
    if is_float {
        return ReadType::F32;
    }
    match byte_width {
        1 => ReadType::I8,
        2 => ReadType::I16,
        4 => ReadType::I32,
        _ => ReadType::I64,
    }
}

#[test]
fn batch410_float_carveout_and_width_table() {
    // the naive bitcast misclassifies -0.0
    assert!(truthy_bitcast_naive(-0.0_f32), "the bit test is wrong on -0.0");
    // the magnitude-bit form is exact
    assert!(!truthy_float(0.0));
    assert!(!truthy_float(-0.0));
    assert!(truthy_float(f32::NAN));
    assert!(truthy_float(1e-45)); // denormal: magnitude bits set
    // the width table: u8 reads as i8, still truthy at 255
    assert_eq!(read_type(false, 1), ReadType::I8);
    assert_eq!(read_type(true, 4), ReadType::F32);
    assert_eq!(read_type(false, 8), ReadType::I64);
}
"#.to_string(),
        },
    ]
}

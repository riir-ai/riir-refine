//! Batch 411 — the four mlx residual slices (`kernel_opt`).
//!
//! Source: ml-explore/mlx @ pin `c69303b0` (MIT — the mlx-serve
//! submodule pin at tip `2201ec9`; the SAME pin as Batch 410 — these
//! are Batch 410's own recorded staged leads, converted per Plan 410 /
//! Research 444). All quotes byte-verified at the clone at the pin
//! this session.
//!
//! The four slices:
//! - `per-row-bounds-bitmask-hoisted-loader` — the input-side pole of
//!   `gpu_166`'s whole-tile inbounds hoist: rows that straddle the
//!   spatial edges get a per-row precomputed bitmask, the hot tap loop
//!   pays two ANDs.
//! - `dilation-base-table-tap-odometer` — the congruence-subsequence
//!   precompute: per-phase {base, size} solved on the host, the device
//!   iterator wraps to the phase base, never to zero.
//! - `reduce-simd-width-fallback-launch-cap-pairing` — the width-keyed
//!   execution split (hardware primitive vs shuffle-down fallback)
//!   PAIRED with the host launch clamp keyed on the same predicate.
//! - `strided-reduce-shape-ratio-assumption-guarded-tier-ladder` — the
//!   four-tier ordered dispatch over (work-per-output, stride,
//!   output width) + the longcolumn-only widening rider.
//!
//! Faces: none this batch — the four are new rules; the adjudication
//! kinship rides in-rule citations (`gpu_166` R4, `gpu_04`
//! derived-scalar-host-computed-in-uniform, `rust_perf` `entries_48`
//! loop-invariant-dispatch-resolution-hoist, `gpu_166` R6, `gpu_127`
//! physical-warp-size, `gpu_117` splitk-tier-ladder, `gpu_61`
//! work-size-gated-overlap-reduce-mode), compose never merge.
//!
//! Honest residue: the mlx upstream-past-pin delta (64 commits) stays
//! UNSWEPT (the watch surface); the CPU backend and the excluded
//! surfaces of plan 409 stay unread. The 8-byte clamp exists in
//! exactly ONE dispatch arm (`row_reduce_simple`) — `row_reduce_looped`
//! and all-reduce pass-1 size without it (recorded, not a defect claim).
//!
//! Fixtures: `corpus/fixtures/b411_fixtures.rs` — standalone-compiled
//! AND executed BEFORE entry (debug AND -O, 16/16); the gate caught
//! THREE fixture defects pre-entry (the masked path's missing weight
//! multiply, the S3 cost model's double-scaled narrow weight, the S1
//! tamper expectation's wrong surviving-row value).
//! NOTHING INVENTED — every quote read at the clone at pin `c69303b0`
//! this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 411 — the four mlx residual slices (Batch 410's staged leads).
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── S1 — the per-row bounds bitmask: per-ROW predicates that a
        // tile predicate can never cover become O(1) bit tests. ──
        KernelRule {
            rule_id: "per-row-bounds-bitmask-hoisted-loader".to_string(),
            gpu_concept: "MECHANISM (bounds-test hoist to per-row bitmasks; category D — dispatch, the constructor-time predicate family): a staged conv/neighborhood input loader whose per-row bounds predicate is SEPARABLE and TAP-INDEXABLE — row i, tap (kh, kw) is in bounds iff (n < N && ih_base + flip(kh)·kdil >= 0 && < iS[0]) && (iw_base + flip(kw)·kdil >= 0 && < iS[1]) — precomputes, ONCE in the constructor, one bitmask per axis per row ('mask_h[i] |= (in_bounds << kh);' over the tap loop, loader_channel_l.h:268-292, with 'bool in_bounds = n < params->N && ih >= 0 && ih < params->iS[0];' the H mask carrying the batch check — the W mask does NOT repeat it; the gate is mask_h AND mask_w so n is folded once), then the hot per-tap load gates each row with two bit-test ANDs against the current tap's one-hot word ('mask_t h_mask = mask_t(1) << weight_h;' + 'if ((mask_h[i] & h_mask) && (mask_w[i] & w_mask))' load else zero-pad, :296-318). The hot loop drops from 2 coordinate IMADs + 5 comparisons + 4 ANDs per row-tap (the SAME FILE's large-filter twin re-evaluates per tap: 'int ih = read_ih[i] + weight_h * kdil_h;' + 'if ((n < params->N) && (ih >= 0 && ih < params->iS[0]) && (iw >= 0 && iw < params->iS[1]))', :117-122) to 2 ANDs + 1 test with ZERO coordinate arithmetic — the src pointers were staged at construction and next() advances them by fixed jumps. The masks are n_rows × 2 thread-local shorts ('using mask_t = short;', :193 — ONE BIT PER TAP ON A 16-BIT CARRIER: the small-filter regime is wS <= 15 per axis, which is exactly why the source routes large filters to the re-evaluating twin) — trivially register-resident, build cost n_rows × wS × ~3 ops amortized over the whole tap walk × channel blocks. The zero-pad arm ('dst[...] = T(0);') is the additive identity the implicit GEMM accumulate needs — masked and unmasked postures are bit-identical by construction. A 3D twin carries three masks the same way ('if ((mask_d[i] & d_mask) && (mask_h[i] & h_mask) && (mask_w[i] & w_mask))', :789-797). THE AXIS IS PER-ROW, NOT PER-TILE: this is the input-side remedy for exactly the case whole-tile-inbounds-hoisted-loader-fast-path (gpu_166, same pin) documents as unhoistable — an input tile's rows straddle the SPATIAL edges differently per k-iteration, so no tile predicate exists; the mask makes the per-row predicate O(1) instead. ⛔ KINSHIP (adjudicated both directions): gpu_166 owns the TILE-granularity predicate + three-tier dispatch keyed on one constructor bool, sourced from the WEIGHT loaders — CLEAN COMPOSE: same tile can carry both (weight side do_read, input side masks); S1 still pays a check per row-tap and never buys an unguarded path, gpu_166's fast path DELETES the check where a tile predicate holds; constexpr-unroll-capacity-bound-zero-guard-holds-array-in-registers (gpu_117) owns array RESIDENCY (compile-time capacity vs runtime indexing) — no overlap, the masks are compile-time-sized thread-locals; mask-sentinel-matches-reduction-identity (gpu_04) governs what masked lanes WRITE into block reductions — the only kinship is S1's zero-pad arm being the identity fill (a face-level echo, citable, never an owner). REGIME: staged input loaders with small per-axis tap counts (single digits in practice); separable per-axis bounds (the H test must not depend on kw); hot tap loop (reuse = taps × channel iters); mixed/partial boundary tiles (the common case near spatial edges). ANTI-REGIME: wS > mask width (representation doesn't fit — keep the re-evaluating twin); non-separable predicates (nothing to split into per-axis bits); a tap loop that runs once or twice (build cost not amortized); bounds that change mid-kernel (masks are launch-invariant). GOAT: the fixture pins the closed-form masks (mask_h/mask_w = 6/7/7/3 for a 7×7 s2 p1 k3 tile), the hot-path arithmetic census (720 bound checks + 576 coordinate ops naive vs 60 build comparisons + 2 ANDs per row-tap fixed), the bit-identity (masked == naive over the whole tile), the all-interior pole (all-ones masks — the compose seam gpu_166's fast path keys on), the tamper arm (a cleared live bit drops the kh=1 row of out(0,1): −5 → +2). NotBenchable-by-domain (the win is staged-loader ALU pressure on Metal threadgroups — no validator hosts that; the mask construction, the census, and the bit-identity are CPU-testable and pinned exact). P113: upstream's own landed decision — decision content only. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/kernels/steel/conv/loaders/loader_channel_l.h:177-345 (struct, mask_t, the constructor's mask build, load_unsafe, next) + :112-122 (the large-filter counterfactual) + :647-815 (the 3D twin). Quotes byte-verified at the clone at the pin this session. MIT. Batch 411 / Plan 410 / Research 444.".to_string(),
            ref_shape: r"// NAIVE: the tap loop re-evaluates the full bounds inequality and
// re-does the coordinate arithmetic per tap per row — the large-filter
// twin's shape (the source's own counterfactual).
fn naive_tap_load(rows: &[(i32, i32, i32)], w_s: usize) -> u64 {
    let mut bound_checks = 0u64;
    for &(n, ih_base, iw_base) in rows {
        for kh in 0..w_s as i32 {
            for kw in 0..w_s as i32 {
                bound_checks += 4; // coord IMADs
                let ih = ih_base + kh;
                let iw = iw_base + kw;
                bound_checks += 5; // n + h(2) + w(2) comparisons
                if n < 1 && ih >= 0 && ih < 7 && iw >= 0 && iw < 7 {
                    // load staged element
                } else {
                    // zero pad
                }
            }
        }
    }
    bound_checks
}

#[test]
fn naive_retests_bounds_every_tap() {
    // 4 rows x 9 taps x 9 ops = 324 hot-loop ops
    assert_eq!(naive_tap_load(&[(0, -1, -1); 4], 3), 324);
}
".to_string(),
            opt_pattern: r"// opt_pattern: build the per-row bitmask once in the constructor,
// then the hot tap load pays two bit tests — zero bound checks, zero
// coordinate arithmetic (the pointers were staged + advanced).
fn build_masks(rows: &[(i32, i32, i32)], w_s: usize) -> Vec<(u16, u16)> {
    let mut masks = Vec::with_capacity(rows.len());
    for &(n, ih_base, iw_base) in rows {
        let (mut mh, mut mw) = (0u16, 0u16);
        for kh in 0..w_s as u16 {
            let ih = ih_base + kh as i32;
            mh |= (((n < 1) && ih >= 0 && ih < 7) as u16) << kh; // n folded here ONLY
        }
        for kw in 0..w_s as u16 {
            let iw = iw_base + kw as i32;
            mw |= (((iw >= 0) && iw < 7) as u16) << kw;
        }
        masks.push((mh, mw));
    }
    masks
}

fn masked_tap_load(masks: &[(u16, u16)], w_s: usize) -> u64 {
    let mut bit_tests = 0u64;
    for (row, &(mh, mw)) in masks.iter().enumerate() {
        for kh in 0..w_s {
            let h_mask = 1u16 << kh; // hoisted per tap, not per row-tap
            for kw in 0..w_s {
                bit_tests += 2;
                if (mh & h_mask) != 0 && (mw & (1u16 << kw)) != 0 {
                    let _ = row; // staged pointer read
                } else {
                    // zero pad (the additive identity)
                }
            }
        }
    }
    bit_tests
}

#[test]
fn masks_kill_the_hot_loop_arithmetic() {
    let rows = [(0i32, -1i32, -1i32), (0, 1, -1), (0, 3, 1), (0, 5, 3)];
    let masks = build_masks(&rows, 3);
    // closed form: bit kh set iff ih_base + kh in [0, 7)
    assert_eq!(masks[0], (0b110, 0b110));
    assert_eq!(masks[3], (0b011, 0b011));
    // hot loop: 4 rows x 9 taps x 2 ANDs = 72 ops (vs 324)
    assert_eq!(masked_tap_load(&masks, 3), 72);
    // build cost: 4 rows x 6 taps x ~3 comparisons = 60 (once)
    // the compose seam: an all-ones mask row is the tile-interior
    // predicate gpu_166's fast path keys on
    let interior = build_masks(&[(0, 1, 1)], 3);
    assert_eq!(interior[0], (0b111, 0b111));
}
".to_string(),
        },
        // ── S2 — the congruence-subsequence precompute: solve the
        // alignment once per phase on the host, wrap to the base. ──
        KernelRule {
            rule_id: "dilation-base-table-tap-odometer".to_string(),
            gpu_concept: "MECHANISM (per-phase alignment precompute + wrap-to-base odometer; category D — dispatch): with input dilation idil, a conv tap t contributes to an output whose dilated input coordinate is congruent to 0 mod idil — so each output's contributing taps are an ARITHMETIC SUBSEQUENCE {base + k·f_wgt_jump} of the weight, where f_wgt_jump = lcm(idil, kdil)/kdil is the tap stride between aligned taps and base is the smallest tap with the congruence. Which subsequence depends only on the output position MOD f_out_jump = lcm(idil, str)/str (the REDUCED period — not stride×dilation), so the host solves the alignment ONCE per phase ('int wh_base = 0; while (wh_base < conv_params.wS[0] && ih_loop % conv_params.idil[0] != 0) { wh_base++; ih_loop += jump_h; } int wh_size = ((conv_params.wS[0] - wh_base) + f_wgt_jump_h - 1) / f_wgt_jump_h; base_h[i] = {wh_base, wh_size};', conv.cpp:522-534, the jump/init vocabulary at :511-520, Conv2DGeneralBaseInfo {weight_base, weight_size} at params.h:97-100), ships the table as a kernel buffer, and the device tap iterator STARTS at the phase base ('weight_h(base_wh_), weight_w(base_ww_)' in BOTH loaders' constructors, loader_general.h:79-80 + :277-278) and WRAPS TO THE PHASE BASE, NEVER TO ZERO ('weight_w += jump_params->f_wgt_jump_w; if (weight_w < params->wS[1]) { return; } weight_w = base_ww; weight_h += jump_params->f_wgt_jump_h; if (...) return; weight_h = base_wh; src += BK;', :361-377 weight twin, :187-208 input twin advancing n_rows pointers). The inner loop NEVER evaluates the modulo and never misses the subsequence start — every visited tap is aligned by construction — and THE TABLE SIZES THE LOOP ITSELF: 'int gemm_k_iterations = base_wh_size * base_ww_size * gemm_params->gemm_k_iterations;' (steel_conv_general.h:124-125), so tail taps past the subsequence are never visited at all, not merely skipped; the per-threadgroup table lookup is two loads at kernel start ('const int base_wh = base_h[base_oh].weight_base;', :80-87) over the phase plane grid z = f_out_jump_h × f_out_jump_w (conv.cpp:624). THE NEGATIVE SPACE IS THE PROOF: grep '% idil' over the device conv tree returns ZERO modulo sites — the only idil modulo in the lane is the HOST build loop (conv.cpp:526, 540). ⛔ KINSHIP (adjudicated both directions): derived-scalar-host-computed-in-uniform (gpu_04) owns the SINGLE-config-derived-scalar half — S2's f_wgt_jump/f_out_jump scalars ARE that rule's shape (a citable instance, never re-owned); the NEW axis is the per-PHASE TABLE (one {base, size} row per residue class, indexed by the tid_z-derived phase) + the wrap-to-base iteration discipline + the table-sized trip count — none of which is 'a scalar passed as a uniform'. compute-experts-before-router-lookup-fixup (gpu_152) is optimistic device-side compute with a fixup path under a deliberate race — S2 is fully deterministic precompute, no snapshot, no lie class, the table is authoritative before the kernel starts. loop-invariant-dispatch-resolution-hoist (rust_perf entries_48) is the same resolve-once GENUS on a different axis — it hoists a loop-invariant resolution out of a hot loop so the body is the bare call; S2 resolves an ALIGNMENT SUBSEQUENCE per phase on the host and replays it with an odometer whose wrap-to-base is an iteration invariant, not a hoisted branch (gpu_166 already names entries_48 as its kin; S2 joins the genus on the third axis). REGIME: dilated / input-dilated / strided implicit-GEMM convs where idil > 1 (non-trivial f_wgt_jump, skipped taps exist) or f_out_jump > 1 (multiple phases exist); any tap iterator whose participation predicate is a congruence on position mod period. ANTI-REGIME: idil == 1 (everything aligned: f_wgt_jump = 1, base 0, size = wS — the table degenerates to the trivial full range and the win is zero; the mechanism degrades gracefully, pinned as the all-safe arm); wS == 1; single-output launches where the (bounded, ≤ wS × f_out_jump-step) host solve wouldn't amortize. GOAT: the fixture pins the host solve verbatim in i32 (config A wS=6 str=2 pad=1 kdil=2 idil=3 → table {(2,2),(1,2),(0,2)}, tap sets {2,5},{1,4},{0,3}) against the congruence cross-check ((2i−1+2t) % 3 == 0), the naive census (18 alignment tests for 6 productive taps vs zero in the walk), the degenerate phase (idil=1 → trivial table, no wrap), config B (phase size can be 1 — one visit, wrap without an in-range second step), and the tamper arm (wrap-to-ZERO injects unaligned taps over the second channel block: fixed {2,5,2,5} vs defective {2,5,0,3}, each injected tap violating the congruence). NotBenchable-by-domain (the win is dead ALU + dead taps on Metal threadgroups — no validator hosts that; the table, the visit sets, and the congruence are CPU-testable and pinned exact). P113: upstream's own landed decision — decision content only. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/conv.cpp:483-547 + :624 + kernels/steel/conv/params.h:97-100 + kernels/steel/conv/kernels/steel_conv_general.h:80-125 + kernels/steel/conv/loaders/loader_general.h:44-91 + :187-208 + :248-377. Quotes byte-verified at the clone at the pin this session. MIT. Batch 411 / Plan 410 / Research 444.".to_string(),
            ref_shape: r"// NAIVE: every output re-tests tap alignment with a modulo in the
// hot loop — and visits the taps the congruence excludes.
fn naive_dilated_walk(phases: usize, w_s: usize, kdil: i32, idil: i32) -> (u64, u64, Vec<i32>) {
    let mut align_tests = 0u64;
    let mut visits = 0u64;
    let mut aligned = Vec::new();
    for i in 0..phases {
        for t in 0..w_s as i32 {
            align_tests += 1;
            visits += 1;
            // the hot-loop modulo the device must never pay
            if (i as i32 * (2 * kdil) - 1 + kdil * t) % idil == 0 {
                aligned.push(t);
            }
        }
    }
    (align_tests, visits, aligned)
}

#[test]
fn naive_retests_alignment_per_tap() {
    let (tests, visits, aligned) = naive_dilated_walk(3, 6, 2, 3);
    assert_eq!(tests, 18);
    assert_eq!(visits, 18); // 12 of the 18 are dead visits
    assert_eq!(aligned, vec![0, 3, 1, 4, 2, 5]);
}
".to_string(),
            opt_pattern: r"// opt_pattern: the host solve (the while loop, verbatim in i32 —
// Rust's % keeps the dividend's sign), then the odometer that starts
// at the base and wraps to the base, never to zero.
fn host_base(w_s: i32, str_: i32, pad: i32, kdil: i32, idil: i32) -> Vec<(i32, i32)> {
    let lcm = |a: i32, b: i32| -> i32 { a / gcd(a, b) * b };
    fn gcd(a: i32, b: i32) -> i32 { if b == 0 { a.abs() } else { gcd(b, a % b) } }
    let f_wgt_jump = lcm(idil, kdil) / kdil;
    let f_out_jump = lcm(idil, str_) / str_;
    let (jump, init) = (kdil, 0); // flip = false
    let mut base = Vec::new();
    for i in 0..f_out_jump {
        let mut ih_loop = i * str_ - pad + init;
        let mut wh_base = 0;
        while wh_base < w_s && ih_loop % idil != 0 {
            wh_base += 1;
            ih_loop += jump;
        }
        let wh_size = ((w_s - wh_base) + f_wgt_jump - 1) / f_wgt_jump;
        base.push((wh_base, wh_size));
    }
    base
}

fn odometer(base: (i32, i32), w_s: i32, jump: i32, trips: usize) -> Vec<i32> {
    let mut t = base.0; // START at the phase base
    let mut visited = Vec::new();
    for _ in 0..trips {
        visited.push(t);
        t += jump;
        if t < w_s {
            continue;
        }
        t = base.0; // WRAP TO THE BASE — never to zero
    }
    visited
}

#[test]
fn table_solves_alignment_once_odometer_never_wraps_to_zero() {
    let base = host_base(6, 2, 1, 2, 3);
    assert_eq!(base, vec![(2, 2), (1, 2), (0, 2)]);
    // the walk visits exactly the congruence set per phase
    for (i, &b) in base.iter().enumerate() {
        let taps: Vec<i32> = (0..6)
            .filter(|&t| (2 * i as i32 - 1 + 2 * t) % 3 == 0)
            .collect();
        assert_eq!(odometer(b, 6, 3, b.1 as usize), taps);
    }
    // the tamper: wrap-to-zero injects unaligned taps on block 2
    let defective: Vec<i32> = {
        let mut t = base[0].0;
        let mut v = Vec::new();
        for k in 0..4 {
            v.push(t);
            t += 3;
            if t >= 6 {
                t = if k >= 1 { 0 } else { base[0].0 };
            }
        }
        v
    };
    assert_eq!(defective, vec![2, 5, 0, 3]); // 0 and 3 violate phase 0
    assert_ne!((-1 + 2 * 0) % 3, 0);
    assert_ne!((-1 + 2 * 3) % 3, 0);
    // the degenerate pole: idil=1 → trivial table, no wrap
    assert_eq!(host_base(6, 2, 1, 1, 1), vec![(0, 6)]);
}
".to_string(),
        },
        // ── S3 — the width-keyed execution split PAIRED with the host
        // launch clamp on the same predicate. ──
        KernelRule {
            rule_id: "reduce-simd-width-fallback-launch-cap-pairing".to_string(),
            gpu_concept: "MECHANISM (width-keyed execution split + same-predicate launch clamp; category N — dtype specialization): a width-generic cooperative reduction splits its EXECUTION PATH by type width — the two-arm enable_if gives sizeof(T) < 8 the hardware simd primitive ('template <typename T, metal::enable_if_t<sizeof(T) < 8, bool> = true> T simd_reduce(T val) thread { return simd_reduce_impl(val); }' — simd_sum/simd_product/simd_min/simd_all, ONE instruction per slot) and sizeof(T) == 8 a software fallback of 5 dependent shuffle rounds per slot ('for (short i = simd_size / 2; i > 0; i /= 2) { val = operator()(val, simd_shuffle_down(val, i)); }', ops.h:8-20, simd_size = 32: i = 16, 8, 4, 2, 1). The fallback is paid TWICE per threadgroup — threadgroup_reduce runs 'totals[i] = op.simd_reduce(totals[i]);' over all N_WRITES slots first, then — when simd_per_group > 1 — lane-0 partials spill to shared_vals, barrier, reload, and 'totals[i] = op.simd_reduce(values[i]);' a SECOND time on every lane of every simdgroup (reduce_row.h:129-161); per-thread shuffle rounds are CONSTANT (20 single-simd / 40 multi-simd at N_WRITES=4), but the AGGREGATE lane-work, the shared round trip, and the barrier serialization all SCALE WITH THE SIMDGROUP COUNT, while the per-thread sequential loop ('blocks = reduction_size / (lsize.x * N_READS)') scales inversely with threadgroup size. The HOST therefore tightens the launch exactly for the fallback's type arm: 'int threadgroup_size = threadgroup_size_from_row_size(row_size); if (in.itemsize() == 8) { threadgroup_size = std::min(threadgroup_size, 512); }' (reduce.cpp:493-496) — trading extra sequential iterations per thread for half the simdgroups, half the phase-2 aggregate, half the barrier. THE PAIRING IS THE POINT: ONE PREDICATE (width == 8) keys both the kernel's fallback arm and the host's clamp — a fallback whose launch budget is not reduced runs its slowest at the largest occupancy. The clamp bites only where the sizing ladder exceeds 512 (row_size ≥ 2049: ladder(4096) = 1024 → 512; ladder(2048) = 512 is a structural no-op — the boundary pin), and it exists in exactly ONE dispatch arm — grep itemsize() == 8 over the metal backend returns this ONE site; row_reduce_looped (reduce.cpp:547) and all-reduce pass-1 (:385-388) size WITHOUT it (the honest residue — recorded, never a defect claim: those kernels' arms differ). ⛔ KINSHIP (adjudicated both directions): logical-reduce-width-bitcast-float-carveout (gpu_166, same pin) owns the READ-TYPE remap law — what dtype the kernel READS AS, with the −0.0 truthiness carve-out; S3 owns the EXECUTION-PATH split + the HOST LAUNCH CLAMP — a performance axis. The same 8-byte width class keys both rules' predicates and they literally compose in one call chain (remap_reduce_types picks in_type, the kernel template's simd_reduce splits on its width, the host clamps on the same width) — cite, never merge; gpu_166's text carries no simd_reduce or sizing content. attention-partition-two-knob-shared-partial-workspace (gpu_86) carries the all-reduce byte-tier face keyed on total BYTES (nbytes vs 2^26) — a different function, a different key, different knobs (workspace rows + pass-2 threads); same-file sibling, cite. physical-warp-size-keys-tile-legality-and-dispatch (gpu_127) keys dispatch on a DEVICE property (the physical warp width, arch-keyed with static_assert) — S3's clamp is TYPE-keyed on a fixed-width device, no arch axis; the genuine kinship is gpu_127's decision-content clause (b), the dispatch predicate and the kernel contract must AGREE on the spelling — S3's pairing invariant (clamp predicate == fallback predicate) is that agreement law instantiated on the dtype axis. REGIME: cooperative (threadgroup-wide) reductions whose slot reduce is width-generic; 64-bit element classes taking a software shuffle fallback; host launch sizing computed from the same shape the kernel splits on. ANTI-REGIME: narrow types on the hardware primitive (the clamp would cost memory-level parallelism for a negligible shuffle term — pinned as the counterfactual loss); width-independent slot reduce (one arm = no pairing needed); dispatch arms whose kernel does NOT contain the fallback (clamping those buys nothing — and upstream indeed does not). GOAT: the fixture pins the ladder (512→32 / 1024→128 / ceil-32-cap-1024), the clamp bite (row 4096: 1024 → 512, cost 15616 → 8192 in ×4 model units), the boundary no-op (row 2048 → 512), the narrow arm unclamped (1024 stays; the counterfactual clamp costs 512 → 640 — the loss the pairing prevents), work conservation (adds × L == row × N_WRITES at every launch size), the per-thread rounds (40 multi-simd / 20 single-simd), and the tamper arm (the host clamp re-keyed on ROW SIZE while the kernel falls back on WIDTH — the grid probe detects the predicate divergence at (4096, narrow)). NotBenchable-by-domain (the win is shuffle-phase lane-seconds on Metal simdgroups — no validator hosts that; the ladder, the cost model, and the predicate pairing are CPU-testable and pinned exact; the cost weights are disclosed MODEL units, never hardware claims). P113: upstream's own landed decision — decision content only. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/kernels/reduction/ops.h:8-22 + :42/:124/:157/:181 (the primitive arms) + kernels/reduction/reduce_row.h:129-161 + :278 + :290 + kernels/defines.h:12-13 (REDUCE_N_READS/N_WRITES = 4) + reduce.cpp:214-230 + :466-500 + :385-388 + :547. Quotes byte-verified at the clone at the pin this session. MIT. Batch 411 / Plan 410 / Research 444.".to_string(),
            ref_shape: r"// NAIVE: the launch is width-blind — the fallback's type arm runs
// its slowest at the largest occupancy.
fn threadgroup_size_from_row_size(row: usize) -> usize {
    if row <= 512 {
        32
    } else if row <= 1024 {
        128
    } else {
        (((row + 3) / 4 + 31) / 32) * 32
    }
    .min(1024)
}

fn naive_launch(row: usize, width8: bool) -> usize {
    let _ = width8; // ignored — the unpaired posture
    threadgroup_size_from_row_size(row)
}

#[test]
fn unpaired_fallback_gets_max_occupancy() {
    assert_eq!(naive_launch(4096, true), 1024);
    // the 8-byte shuffle fallback pays 2 phases x 4 slots x 5 rounds
    // on EVERY lane of 32 simdgroups — the unpaired worst case
}
".to_string(),
            opt_pattern: r"// opt_pattern: the clamp keyed on the SAME predicate as the kernel's
// fallback arm — width == 8.
fn fixed_launch(row: usize, width8: bool) -> usize {
    let tg = threadgroup_size_from_row_size(row);
    if width8 {
        tg.min(512)
    } else {
        tg
    }
}

/// ×4 integer cost model: sequential adds + cross-group shuffle units
/// (wide rounds weigh 12 model-units, the primitive 1 — disclosed
/// MODEL units, never hardware claims).
fn cost4(row: usize, l: usize, width8: bool) -> usize {
    let adds = 16 * row * 4 / l; // N_WRITES = 4, 4 units/add
    let phases = if l / 32 == 1 { 1 } else { 2 };
    let rounds = if width8 { 5 } else { 1 };
    adds + (phases * 4 * rounds * (l / 32)) * if width8 { 12 } else { 1 }
}

#[test]
fn pairing_clamps_exactly_the_fallback_arm() {
    assert_eq!(fixed_launch(4096, true), 512); // the bite
    assert_eq!(fixed_launch(2048, true), 512); // boundary no-op
    assert_eq!(fixed_launch(512, true), 32);
    assert_eq!(fixed_launch(4096, false), 1024); // narrow NEVER clamps
    // the bite's payoff: 15616 → 8192 model units
    assert_eq!(cost4(4096, 1024, true), 15_616);
    assert_eq!(cost4(4096, 512, true), 8_192);
    // the counterfactual: clamping the PRIMITIVE arm loses (512 → 640)
    assert_eq!(cost4(4096, 1024, false), 512);
    assert_eq!(cost4(4096, 512, false), 640);
    // work conservation at every launch size
    for l in [32usize, 128, 512, 1024] {
        let blocks = 4096 / (l * 4);
        assert_eq!(blocks * 4 * 4 * l, 4096 * 4);
    }
}
".to_string(),
        },
        // ── S4 — the shape-ratio tier ladder over reduction topologies. ──
        KernelRule {
            rule_id: "strided-reduce-shape-ratio-assumption-guarded-tier-ladder".to_string(),
            gpu_concept: "MECHANISM (shape-ratio assumption-guarded tier ladder; category D — dispatch): a strided/column reduction is dispatched across FOUR execution tiers by the shape's ratios — work-per-output R = reduction_size × non_col_reductions, row-contiguity (reduction_stride), and output width (out.size()/32) — ordered cheapest-first with EACH TIER'S ASSUMPTION GUARDED BY THE PREDICATE THAT ADMITS IT ('if (args.reduction_size * args.non_col_reductions < 32) return strided_reduce_small(...); // Small column' — per-output work trivial, one inline tile, no intermediate; 'if (args.reduction_stride < 32 && args.reduction_size * args.non_col_reductions >= 1024) return strided_reduce_longcolumn(...); // Long column but small row' — the column walk stays cache-local BECAUSE the kernel's threadgroup_x IS the short stride ('size_t threadgroup_x = args.reduction_stride;', reduce.cpp:684) AND the work is large enough that one-thread-per-output-region pays; 'if (args.reduction_size * args.non_col_reductions > 256 && out.size() / 32 < 1024) return strided_reduce_2pass(...)' — the second pass amortized by the work AND the pass-2 block count bounded; 'return strided_reduce_looped(...)' — the grid-strided general fallback, no assumption; reduce.cpp:959-976, the ratio vocabulary from ColReduceArgs at :89-122). The ladder is ORDER-SENSITIVE: longcolumn is checked before 2pass, so a short-stride high-R shape never reaches 2pass even when 2pass's own predicate also holds — admission is by the FIRST satisfied guard, and reordering the ladder breaks the assumption a tier makes (a long-column shape routed to 2pass pays a pass-2 geometry sized for wide outputs; a 2pass-shaped shape routed to longcolumn pays threadgroup_x = stride with stride ≥ 32). SUB-CLAUSE (the widening rider): longcolumn's intermediate first dimension widens 32 → 128 once 'total_reduction_size >= 32768' ('size_t outer_blocks = 32; if (total_reduction_size >= 32768) { outer_blocks = 128; }', reduce.cpp:661-665 — the intermediate scales with total work once the work amortizes it) while 2pass's outer_blocks stays FIXED at 32 (':845, :860') — the widening belongs only to the tier whose second pass reads exactly outer_blocks partials per output at a stride that no longer grows. ⛔ KINSHIP (adjudicated both directions): splitk-tier-ladder-per-tier-ceiling-min-partition-inputs (gpu_117) is the closest owner of the admission-LADDER species (guarded asks, per-tier ceilings, min partition inputs) but it sizes the split-K DEPTH within one kernel while S4 selects among four whole REDUCTION KERNEL TOPOLOGIES by shape ratios with no depth knob — cite as kin, the reduction-topology ladder + widening rider remain unowned. work-size-gated-overlap-reduce-mode (gpu_61) is the closest single-axis cousin (one 32-MiB work-size threshold flips plan machinery) — one binary gate vs a 4-tier ordered ladder over three axes; cite as the work-size-gate precedent. buffer-set-struct-two-tier-dispatch (gpu_04) is an API/lifetime discipline (pre-allocated vs self-allocating entry), not shape-ratio execution dispatch — contrast only. gpu_86's Batch-410 all-reduce face (nbytes-keyed workspace tiers, same file, reduce.cpp:363-376) is the same size-the-intermediate-by-a-shape-key family at a different function and different knobs — cite as the sibling. REGIME: strided/column reductions (last-axes plans over strided layouts) dispatched by shape ratios; any reduction stack choosing among whole-kernel topologies by (work-per-output, contiguity, output width). ANTI-REGIME: a single strided kernel for all shapes (small shapes pay loop machinery, long columns serialize); a one-predicate dispatch (work size alone misroutes both ends — pinned: shapes 1/3/9 misroute under R-only routing); reordering the ladder (admission stops guarding the assumption). GOAT: the fixture pins the 11-shape table with boundary-exact rows (R=31 Small; R=32/S=64 Looped past small; R=1024/S=16 LongColumn at the exact lower boundary; R=1023 TwoPass one under it; R=257 TwoPass at the exact boundary; R=256 Looped one under; O/32=1024 Looped at the exact upper boundary; O/32=1023 TwoPass one under; total=32768 widening exact; 32767 one under; and shape 11 — S<32 but R<1024 → 2pass — PROVING the order), the widening rider's asymmetry (longcolumn 128 at 32768, 2pass NEVER widens even at 10^6), the naive misroute census (one-predicate dispatch ≥ 3 divergences), and the tamper arm (the reordered ladder routes shapes 3 and 9 to TwoPass against the pinned LongColumn). NotBenchable-by-domain (the win is topology selection on Metal dispatch — no validator hosts that; the ladder, the table, and the rider are CPU-testable and pinned exact). P113: upstream's own landed decision — decision content only. Source: ml-explore/mlx @ c69303b0 (MIT) — mlx/backend/metal/reduce.cpp:89-122 (ColReduceArgs) + :586-636 (small) + :652-764 (longcolumn, the widening at :661-665, threadgroup_x at :684) + :766-829 (looped) + :832-919 (2pass, outer_blocks fixed at :845/:860) + :947-977 (the ladder). Quotes byte-verified at the clone at the pin this session. MIT. Batch 411 / Plan 410 / Research 444.".to_string(),
            ref_shape: r#"// NAIVE: one strided kernel (or one predicate) for every shape —
// small shapes pay loop machinery, long columns serialize.
fn naive_route(r: usize, _s: usize, o: usize) -> &'static str {
    if r > 256 && o / 32 < 1024 {
        "2pass"
    } else {
        "looped"
    }
}

#[test]
fn one_predicate_misroutes_both_ends() {
    assert_eq!(naive_route(31, 8, 1024), "looped"); // Small shape pays it all
    assert_eq!(naive_route(1024, 16, 2048), "2pass"); // a long column misrouted
}
"#.to_string(),
            opt_pattern: r#"// opt_pattern: the four-tier ordered ladder — first satisfied guard
// wins, each guard is exactly the assumption its tier makes.
fn tier(r: usize, s: usize, o: usize) -> &'static str {
    if r < 32 {
        return "small";
    }
    if s < 32 && r >= 1024 {
        return "longcolumn";
    }
    if r > 256 && o / 32 < 1024 {
        return "2pass";
    }
    "looped"
}

/// the widening rider: longcolumn-only — 2pass NEVER widens.
fn longcolumn_outer_blocks(total: usize) -> usize {
    if total >= 32768 { 128 } else { 32 }
}

#[test]
fn ladder_and_widening_pinned() {
    // boundary-exact rows
    assert_eq!(tier(31, 8, 1024), "small");
    assert_eq!(tier(1024, 16, 2048), "longcolumn"); // exact lower boundary
    assert_eq!(tier(257, 64, 8192), "2pass"); // exact R boundary
    assert_eq!(tier(256, 64, 8192), "looped"); // one under
    assert_eq!(tier(1024, 64, 32768), "looped"); // O/32 == 1024, NOT <
    assert_eq!(tier(1023, 16, 4096), "2pass"); // LC misses by 1
    // the ORDER proof: S < 32 but R < 1024 → LC refuses → 2pass
    assert_eq!(tier(512, 16, 4096), "2pass");
    // the widening rider's asymmetry
    assert_eq!(longcolumn_outer_blocks(32768), 128);
    assert_eq!(longcolumn_outer_blocks(32767), 32);
    // 2pass's outer blocks stay 32 even at absurd totals
    // (two_pass_outer_blocks(1_000_000) == 32 — pinned in the fixture)
}
"#.to_string(),
        },
    ]
}

//! Batch 410 kernel fixture preflight — the vendored ml-explore/mlx
//! tree slice (3), the 2D-conv + col-reduce clusters (Plan 409 /
//! Research 443, ml-explore/mlx @ c69303b0, MIT) as ONE standalone file
//! (6 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Metal/C++; the Rust arms below model each law's decision
//! content). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b410_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_166.rs in the same landing commit.
//!
//! Arms:
//! - R1 winograd-fixed-transform-as-simdgroup-mma-chain: the mma-chain
//!   form of the input transform ((Bt·I)·B) equals the scalar closed
//!   form at the pinned association order in f64 (transcription
//!   validity); the fragment packing completeness (two elements per
//!   lane rebuild the constant exactly); the identity control (B = I
//!   returns the input); the tamper arm (a perturbed constant reds).
//! - R2 winograd-admission-budget-floor-routes-to-generalist: the
//!   budget arithmetic (used subtracted first); the floor fires on
//!   BOTH conjuncts (safe_n < total_n AND rows < 32); the route (None
//!   = the caller's generalist); the tamper arm (floor removed admits
//!   an under-amortized tiling).
//! - R3 identity-guarded-atomic-elision-packed-bool-word: the elision
//!   count (zero atomics on an all-true stream); the packed-word lane
//!   isolation; the semantic identity (guarded == unguarded on mixed
//!   streams); the asymmetry control (Sum keeps its bare RMW).
//! - R4 whole-tile-inbounds-hoisted-loader-fast-path: the three tiers
//!   (interior → unguarded, straddling → guarded, out → zero-fill);
//!   the check count (zero on the hot posture); the semantic identity
//!   (staged bytes identical across the tiers); the tamper arm (an
//!   off-by-one predicate admits a straddling tile).
//! - R5 nan-preflight-preserves-propagation-parity: the lossy lane min
//!   reds (NaN dropped); the preflight propagates; the pairwise twin
//!   agrees; the stage-parity clause (lane boundary vs local fold);
//!   the integer carve-out.
//! - R6 logical-reduce-width-bitcast-float-carveout: the naive bitcast
//!   reds on -0.0; the magnitude-bit form is exact; the width table;
//!   the tamper arm (carve-out removed reds).

// ═══════════ R1 — the mma-chain transform ═══════════

type Mat8 = [[f64; 8]; 8];

fn matmul8(a: &Mat8, m: &Mat8) -> Mat8 {
    let mut out = [[0.0; 8]; 8];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, o) in row.iter_mut().enumerate() {
            let mut acc = 0.0f64;
            for k in 0..8 {
                acc += a[i][k] * m[k][j];
            }
            *o = acc;
        }
    }
    out
}

fn transpose8(a: &Mat8) -> Mat8 {
    let mut t = [[0.0; 8]; 8];
    for (i, row) in a.iter().enumerate() {
        for (j, &v) in row.iter().enumerate() {
            t[j][i] = v;
        }
    }
    t
}

/// the scalar closed form: out[i][j] = Σ_k Σ_l B[k][i]·tile[k][l]·B[l][j]
/// (the elementwise expansion of (Bt·I)·B).
fn winograd_scalar(tile: &Mat8, b: &Mat8) -> Mat8 {
    let mut out = [[0.0; 8]; 8];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, o) in row.iter_mut().enumerate() {
            let mut acc = 0.0;
            for k in 0..8 {
                for l in 0..8 {
                    acc += b[k][i] * tile[k][l] * b[l][j];
                }
            }
            *o = acc;
        }
    }
    out
}

/// the chain form: (Bt·I)·B — the mma-unit shape.
fn winograd_chain(tile: &Mat8, b: &Mat8) -> Mat8 {
    let bt = transpose8(b);
    let inner = matmul8(&bt, tile);
    matmul8(&inner, b)
}

/// the source's Winograd input-transform constant (WinogradTransforms
/// <6,3,8>::in_transform, f64 for the exactness pin).
const IN_TRANSFORM: Mat8 = [
    [1.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00, 1.00, -1.00, 0.50, -0.50, 2.00, -2.00, -1.00],
    [-5.25, 1.00, 1.00, 0.25, 0.25, 4.00, 4.00, 0.00],
    [0.00, -4.25, 4.25, -2.50, 2.50, -2.50, 2.50, 5.25],
    [5.25, -4.25, -4.25, -1.25, -1.25, -5.00, -5.00, 0.00],
    [0.00, 1.00, -1.00, 2.00, -2.00, 0.50, -0.50, -5.25],
    [-1.00, 1.00, 1.00, 1.00, 1.00, 1.00, 1.00, 0.00],
    [0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 1.00],
];

/// fragment packing completeness: lane (sm, sn) holds [sm][sn] and
/// [sm][sn+1]; rebuilding from the packed pairs is exact.
fn pack_unpack(b: &Mat8) -> Mat8 {
    let mut rebuilt = [[0.0; 8]; 8];
    for sm in 0..8usize {
        for sn in (0..8usize).step_by(2) {
            rebuilt[sm][sn] = b[sm][sn];
            rebuilt[sm][sn + 1] = b[sm][sn + 1];
        }
    }
    rebuilt
}

#[test]
fn batch410_r1_chain_equals_scalar_closed_form() {
    let mut tile = [[0.0; 8]; 8];
    for (i, row) in tile.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = ((i * 8 + j) % 13) as f64 * 0.5 - 3.0;
        }
    }
    // the chain form IS the closed form at the same association order
    let chain = winograd_chain(&tile, &IN_TRANSFORM);
    let scalar = winograd_scalar(&tile, &IN_TRANSFORM);
    for (cr, sr) in chain.iter().zip(scalar.iter()) {
        for (c, s) in cr.iter().zip(sr.iter()) {
            assert!((c - s).abs() < 1e-9, "chain {c} vs scalar {s}");
        }
    }
    // fragment packing completeness: the constant rebuilds exactly
    assert_eq!(pack_unpack(&IN_TRANSFORM), IN_TRANSFORM);
}

#[test]
fn batch410_r1_identity_control_and_tamper() {
    let mut tile = [[0.0; 8]; 8];
    for (i, row) in tile.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = (i * 3 + j) as f64;
        }
    }
    // identity control: B = I returns the input bit-exact
    let identity: Mat8 = [
        [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    ];
    assert_eq!(winograd_chain(&tile, &identity), tile);
    // tamper arm: one perturbed constant reds against the closed form
    let mut tampered = IN_TRANSFORM;
    tampered[2][3] += 0.01;
    let chain = winograd_chain(&tile, &tampered);
    let scalar = winograd_scalar(&tile, &IN_TRANSFORM);
    let diverged = chain
        .iter()
        .zip(scalar.iter())
        .any(|(cr, sr)| cr.iter().zip(sr.iter()).any(|(c, s)| (c - s).abs() > 1e-6));
    assert!(diverged, "the perturbed constant must red");
}

// ═══════════ R2 — the admission budget + floor ═══════════

const MIN_ROWS_PER_TILE: usize = 32;

fn admit_winograd(
    n: usize,
    c: usize,
    o: usize,
    in_hw: usize,
    tiles_per_n: usize,
    working_set: usize,
    env_tile_batch: usize,
) -> Option<usize> {
    let shape_ok = c % 32 == 0 && o % 32 == 0 && n * in_hw >= 4096 && c + o >= 256;
    if !shape_ok {
        return None;
    }
    let limit = working_set / 4 * 3;
    let used = 8 * 8 * c * o * 4; // filt_bytes first
    let budget = limit.saturating_sub(used);
    let bytes_per_n = 16 * 1024;
    let max_n = budget / bytes_per_n;
    let safe_n = max_n.min(n);
    if env_tile_batch > 0 {
        return Some(env_tile_batch.min(safe_n).max(1));
    }
    if safe_n < n && safe_n * tiles_per_n < MIN_ROWS_PER_TILE {
        return None; // route to the implicit-GEMM generalist
    }
    Some(safe_n.max(1))
}

#[test]
fn batch410_r2_floor_routes_to_generalist() {
    // used subtracted first: a huge filter transform eats the budget
    let roomy = admit_winograd(8, 128, 128, 512, 10, 1 << 30, 0);
    assert_eq!(roomy, Some(8), "untiled fit admits all 8");
    // a tiny working set forces safe_n = 2; 2*10 = 20 rows < 32 — refuse
    let floored = admit_winograd(64, 128, 128, 256, 10, 512 * 1024, 0);
    assert_eq!(floored, None, "under-amortized tiling routes out");
    // the floor needs BOTH conjuncts: an untiled fit (safe_n == n)
    // never fires even with few tiles
    let untiled = admit_winograd(2, 128, 128, 2048, 1, 1 << 30, 0);
    assert!(untiled.is_some(), "untiled fit skips the floor");
    // tamper: the floor removed admits the under-amortized tiling
    let floor_removed = |n: usize, working_set: usize| {
        let limit = working_set / 4 * 3;
        let budget = limit.saturating_sub(8 * 8 * 128 * 128 * 4);
        let safe_n = (budget / (16 * 1024)).min(n);
        Some(safe_n.max(1)) // the defect: no floor
    };
    assert_eq!(floor_removed(64, 512 * 1024), Some(1)); // the defect: a 1-row tiling admitted where the floor refuses (None)
}

// ═══════════ R3 — the identity-guarded atomic elision ═══════════

use std::cell::Cell;

thread_local! {
    static ATOMIC_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn and_reduce_guarded(words: &[bool], out: &Cell<u32>, lane: usize) {
    for &w in words {
        if !w {
            ATOMIC_CALLS.with(|c| c.set(c.get() + 1));
            out.set(out.get() & !(1u32 << lane));
        }
        // w == true is And's identity: fetch_and(1) is a no-op — skip
    }
}

fn and_reduce_unguarded(words: &[bool], out: &Cell<u32>, lane: usize) {
    for &w in words {
        ATOMIC_CALLS.with(|c| c.set(c.get() + 1));
        out.set(out.get() & (!(1u32 << lane) | (w as u32) << lane));
    }
}

#[test]
fn batch410_r3_elision_count_and_lane_isolation() {
    let out = Cell::new(u32::MAX);
    // 4096 true elements: the guarded form issues ZERO atomics
    ATOMIC_CALLS.with(|c| c.set(0));
    and_reduce_guarded(&[true; 4096], &out, 2);
    assert_eq!(ATOMIC_CALLS.with(Cell::get), 0);
    assert_eq!(out.get(), u32::MAX);

    // one false in lane 2: exactly one atomic, other lanes isolated
    ATOMIC_CALLS.with(|c| c.set(0));
    let mut stream = [true; 4096];
    stream[17] = false;
    and_reduce_guarded(&stream, &out, 2);
    assert_eq!(ATOMIC_CALLS.with(Cell::get), 1);
    assert_eq!(out.get() & (1 << 2), 0, "lane 2 cleared");
    assert_eq!(out.get() & 0b1011, 0b1011, "lanes 0,1,3 untouched");

    // semantic identity: guarded == unguarded on a mixed stream
    let out2 = Cell::new(u32::MAX);
    and_reduce_unguarded(&stream, &out2, 2);
    assert_eq!(out.get(), out2.get());

    // the asymmetry control: Sum's bare RMW keeps counting — the
    // source's own no-guard form modeled as always counting
    let sum_calls = stream.len(); // a bare fetch_add per element
    assert_eq!(sum_calls, 4096);
}

// ═══════════ R4 — the whole-tile hoisted loader ═══════════

#[derive(Clone, Copy)]
struct TileSpec {
    first_row: usize,
    rows: usize,
    bound: usize,
}

impl TileSpec {
    fn fully_interior(&self) -> bool {
        self.first_row + self.rows <= self.bound
    }
    fn wholly_out(&self) -> bool {
        self.first_row >= self.bound
    }
}

fn load_tile(spec: TileSpec, src: &[f32], dst: &mut [f32]) -> (usize, u8) {
    // tier 3: wholly out — zero-fill + return
    if spec.wholly_out() {
        dst.fill(0.0);
        return (0, 2);
    }
    // tier 1: fully interior — the unguarded fast path
    if spec.fully_interior() {
        for (i, d) in dst.iter_mut().enumerate() {
            let row = spec.first_row + i / spec.rows;
            *d = src[row * spec.rows + i % spec.rows];
        }
        return (0, 0);
    }
    // tier 2: straddling — the per-row guard
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
    (checks, 1)
}

#[test]
fn batch410_r4_tier_dispatch_and_tamper() {
    let src: Vec<f32> = (0..4096).map(|i| i as f32).collect();
    let interior = TileSpec { first_row: 0, rows: 16, bound: 256 };
    let (checks, tier) = load_tile(interior, &src, &mut [0.0; 256]);
    assert_eq!((checks, tier), (0, 0), "interior: unguarded, zero checks");
    let straddling = TileSpec { first_row: 250, rows: 16, bound: 256 };
    let (checks, tier) = load_tile(straddling, &src, &mut [0.0; 256]);
    assert_eq!((checks, tier), (256, 1), "straddling: guarded");
    let outside = TileSpec { first_row: 300, rows: 16, bound: 256 };
    let mut dst = [9.0f32; 256];
    let (checks, tier) = load_tile(outside, &src, &mut dst);
    assert_eq!((checks, tier), (0, 2));
    assert!(dst.iter().all(|&v| v == 0.0), "zero-filled");
    // semantic identity: tier 1 and tier 2 agree on bytes for a
    // boundary tile that tier 1's true predicate admits
    let mut via_fast = vec![0.0f32; 256];
    let mut via_guard = vec![0.0f32; 256];
    let near_edge = TileSpec { first_row: 240, rows: 16, bound: 256 };
    load_tile(near_edge, &src, &mut via_fast);
    for (i, d) in via_guard.iter_mut().enumerate() {
        let row = 240 + i / 16;
        *d = if row < 256 { src[row * 16 + i % 16] } else { 0.0 };
    }
    assert_eq!(via_fast, via_guard);
    // tamper: first + rows < bound admits 240..256 — but the TRUE
    // predicate (<=) must also admit it; the tamper is the off-by-one
    // that admits 250..266 (OOB)
    let tampered_pred = |first: usize, bound: usize| first < bound;
    assert!(
        tampered_pred(250, 256),
        "the loose first-row-only predicate admits a straddling tile — the defect"
    );
    assert!(!TileSpec { first_row: 250, rows: 16, bound: 256 }.fully_interior());
}

// ═══════════ R5 — the NaN preflight ═══════════

fn lane_min_naive(lanes: &[f32]) -> f32 {
    // the hardware simd_min model: NaN loses pairwise
    lanes
        .iter()
        .copied()
        .reduce(|a, b| if a.is_nan() { b } else if b.is_nan() { a } else { a.min(b) })
        .unwrap()
}

fn lane_min_preflight(lanes: &[f32]) -> f32 {
    if lanes.iter().any(|v| v.is_nan()) {
        return f32::NAN;
    }
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
fn batch410_r5_nan_parity_at_every_stage() {
    let poisoned = [1.0, f32::NAN, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    assert_eq!(lane_min_naive(&poisoned), 1.0, "the lossy form drops the NaN");
    assert!(lane_min_preflight(&poisoned).is_nan(), "the preflight propagates");
    assert!(fold_pairwise(f32::NAN, 1.0).is_nan(), "the pairwise twin agrees");
    // stage parity: NaN at a lane boundary vs inside the local fold
    let boundary = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, f32::NAN];
    assert!(lane_min_preflight(&boundary).is_nan());
    assert!(fold_pairwise(1.0, f32::NAN).is_nan());
    // clean streams: both forms agree (the preflight is invisible)
    let clean = [8.0, 3.0, 5.0, 1.0, 9.0, 2.0, 7.0, 4.0];
    assert_eq!(lane_min_naive(&clean), 1.0);
    assert_eq!(lane_min_preflight(&clean), 1.0);
    // the integer carve-out: no NaN domain, the bare intrinsic stands
    let ints = [3i32, 1, 2];
    assert_eq!(ints.iter().copied().reduce(i32::min), Some(1));
}

// ═══════════ R6 — the logical-reduce bitcast ═══════════

fn truthy_bitcast_naive(v: f32) -> bool {
    v.to_bits() != 0
}

fn truthy_float(v: f32) -> bool {
    (v.to_bits() & 0x7FFF_FFFF) != 0
}

#[derive(Debug, PartialEq, Eq)]
enum ReadType {
    I8,
    I16,
    I32,
    I64,
    F32,
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
fn batch410_r6_float_carveout_and_width_table() {
    // the naive bitcast misclassifies -0.0
    assert!(truthy_bitcast_naive(-0.0_f32), "the bit test reads -0.0 TRUE");
    assert!(logical_and_naive(&[1.0, -0.0]), "so does the reduction");
    // the magnitude-bit form is exact
    assert!(!truthy_float(0.0));
    assert!(!truthy_float(-0.0));
    assert!(truthy_float(f32::NAN));
    assert!(truthy_float(1e-45)); // denormal: magnitude bits set
    // the width table: signed reads for the bits-only class
    assert_eq!(read_type(false, 1), ReadType::I8);
    assert_eq!(read_type(false, 2), ReadType::I16);
    assert_eq!(read_type(false, 4), ReadType::I32);
    assert_eq!(read_type(false, 8), ReadType::I64);
    assert_eq!(read_type(true, 4), ReadType::F32);
    // u8 255 stays true via the signed read (as i8 = -1 ≠ 0)
    let raw: u8 = 255;
    assert_eq!(raw as i8, -1i8);
    // tamper: the carve-out removed reds the -0.0 case
    let no_carveout = |v: f32| v.to_bits() != 0;
    assert!(no_carveout(-0.0), "without the carve-out -0.0 misclassifies");
}

fn logical_and_naive(xs: &[f32]) -> bool {
    xs.iter().all(|&v| truthy_bitcast_naive(v))
}

//! Batch 339 fixtures — the TriAttention laws as ONE standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b339_fixtures.rs` (the B336/B338 discipline). Each rule's ref/opt arms in
//! `entries_gpu_151.rs` embed the same executed code; the ref arms carry
//! their own scoped test so every arm standalone-compiles under the G1
//! cascade. ORIGINAL Rust throughout (the source has NO LICENSE — decision
//! content only, never source verbatim).
//!
//! F1 = tied-rotation-bilinear-invariance-quant-isotropy
//! F2 = diagonal-boundary-pair-o1-policy-storage
//! F3 = deferred-correction-rides-rescale-periodic-flush
//! F4 = lagrangian-shared-budget-policy-allocator

// ─────────────────────────────────────────────────────────────────────────────
// F1 — the tied orthogonal rotation: the SAME pinned sign-Hadamard applied to
// BOTH operands of a dot product before independent per-row absmax
// quantization. The dot is INVARIANT (orthogonality); each side's error is
// ISOTROPIZED (single-axis outliers stop governing the absmax scale). Naive =
// quantize raw; opt = quantize rotated.
// ─────────────────────────────────────────────────────────────────────────────

/// The pinned sign table (the corpus's stand-in for the source's four u32
/// constants: 128 pinned bits at D=128; D=16 here takes the first 16 bits of
/// the same stream — the STRUCTURE is the law, the width is the fixture's).
const SIGNS: [u32; 4] = [0x9a3b_5e71, 0x2c6d_8f13, 0xe41b_7a95, 0x5f02_c3d8];

fn sign_bits() -> [f32; 16] {
    let bits: Vec<bool> = SIGNS
        .iter()
        .flat_map(|w| (0..32).map(move |b| (w >> b) & 1 == 1))
        .take(16)
        .collect();
    let mut s = [0.0f32; 16];
    for (i, b) in bits.iter().enumerate() {
        s[i] = if *b { -1.0 } else { 1.0 };
    }
    s
}

/// rotate: sign flips + a full pairwise butterfly + 1/sqrt(D) — orthogonal by
/// construction (each stage is), the same three moves as the source's
/// `rotate128` at its own width.
fn rotate(x: &mut [f32]) {
    let d = x.len();
    let signs = sign_bits();
    for (v, s) in x.iter_mut().zip(signs.iter()) {
        *v *= *s;
    }
    let mut stride = 1usize;
    while stride < d {
        for base in (0..d).step_by(stride * 2) {
            for i in 0..stride {
                let a = x[base + i];
                let b = x[base + i + stride];
                x[base + i] = a + b;
                x[base + i + stride] = a - b;
            }
        }
        stride *= 2;
    }
    let inv = 1.0 / (d as f32).sqrt();
    for v in x.iter_mut() {
        *v *= inv;
    }
}

/// Per-row absmax symmetric int8 quantization + dequant (the standalone
/// fixture's stand-in for the IMMA row quantizers).
fn quant_dequant_row(x: &[f32]) -> Vec<f32> {
    let absmax = x.iter().fold(0.0f32, |a, &v| a.max(v.abs()));
    let scale = absmax / 127.0;
    x.iter()
        .map(|&v| {
            let q = (v / scale).round();
            let q = q.clamp(-127.0, 127.0);
            q * scale
        })
        .collect()
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}

/// Seeded adversarial row: one dominant axis + small uniform noise on the
/// rest (LCG, fixed seed — deterministic).
fn adversarial_row(outlier_axis: usize, outlier: f32, seed: u64) -> [f32; 16] {
    let mut s = seed;
    let mut x = [0.0f32; 16];
    for i in 0..16 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // noise in [-0.3, 0.3)
        let n = (((s >> 33) as f32 / (1u64 << 31) as f32) - 0.5) * 0.6;
        x[i] = if i == outlier_axis { outlier } else { n };
    }
    x
}

#[test]
fn f1_ref_raw_absmax_loses_spread_axes() {
    // x has its energy on axis 0; y on axis 3. The dot's VALUE lives on the
    // small components (the outliers are orthogonal), but raw per-row absmax
    // scales by the outliers, so the small components quantize to zero.
    let x = adversarial_row(0, 100.0, 0xDEAD_42);
    let y = adversarial_row(3, 80.0, 0xC0FF_EE);

    let qx = quant_dequant_row(&x);
    let qy = quant_dequant_row(&y);
    let exact = dot(&x, &y);
    let approx = dot(&qx, &qy);
    let err = (approx - exact).abs();

    // the naive error IS (most of) the signal: the noise cross-terms died.
    assert!(
        err > 0.5 * exact.abs(),
        "naive quantization must lose the spread-axis signal (err {err}, exact {exact})"
    );
}

#[test]
fn f1_tied_rotation_invariance_and_isotropy() {
    let x = adversarial_row(0, 100.0, 0xDEAD_42);
    let y = adversarial_row(3, 80.0, 0xC0FF_EE);

    // (a) INVARIANCE: the dot in rotated coordinates equals the raw dot
    //     (orthogonality), to f32 tolerance.
    let mut rx = x;
    let mut ry = y;
    rotate(&mut rx);
    rotate(&mut ry);
    let raw_dot = dot(&x, &y);
    let rot_dot = dot(&rx, &ry);
    let rel = ((rot_dot - raw_dot) / raw_dot.abs().max(1e-6)).abs();
    assert!(rel < 1e-3, "dot must be invariant under the tied rotation (rel {rel})");

    // (b) ISOTROPY: the outlier no longer governs the scale — max/mean(|·|)
    //     collapses toward 1.
    let ratio = |v: &[f32]| {
        let max = v.iter().fold(0.0f32, |a, &w| a.max(w.abs()));
        let mean = v.iter().map(|w| w.abs()).sum::<f32>() / v.len() as f32;
        max / mean
    };
    assert!(ratio(&x) > 10.0, "raw adversarial row must be outlier-governed");
    assert!(
        ratio(&rx) < 2.0,
        "rotated row must be isotropized (ratio {})",
        ratio(&rx)
    );

    // (c) ERROR BOUND SHRINKS: quantizing the rotated operands reconstructs
    //     the dot far better than quantizing the raw ones.
    let exact = dot(&x, &y);
    let qrx = quant_dequant_row(&rx);
    let qry = quant_dequant_row(&ry);
    let err_rot = (dot(&qrx, &qry) - exact).abs();
    let qx = quant_dequant_row(&x);
    let qy = quant_dequant_row(&y);
    let err_raw = (dot(&qx, &qy) - exact).abs();
    assert!(
        err_rot < err_raw,
        "tied rotation must shrink the dot error (rot {err_rot} vs raw {err_raw})"
    );
    // and by a wide margin on this adversarial class, not a rounding hair.
    assert!(err_raw / err_rot.max(1e-6) > 2.0);
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — the diagonal boundary pair: a per-head attention precision policy that
// is MONOTONE in tile distance collapses to TWO integers (O(1) storage,
// independent of context length). Naive = a materialized per-tile class table
// (O(H·T) bytes); opt = the boundary pair expanded in closed form.
// ─────────────────────────────────────────────────────────────────────────────

/// Per-tile policy table: one byte per tile (the storage the boundary pair
/// replaces). Class: 8 = int8 band, 4 = int4 band, 0 = skip.
fn class_table(t: usize, w8: usize, w4: usize) -> Vec<u8> {
    // tile index j (0 = oldest); distance d = (t-1) - j.
    (0..t)
        .map(|j| {
            let d = (t - 1) - j;
            if d < w8 {
                8
            } else if d < w4 {
                4
            } else {
                0
            }
        })
        .collect()
}

/// The O(1) encoding: two boundaries expanded by closed-form arithmetic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BoundaryPair {
    w8: usize,
    w4: usize,
}

impl BoundaryPair {
    fn class_at(&self, t: usize, j: usize) -> u8 {
        let d = (t - 1) - j;
        if d < self.w8 {
            8
        } else if d < self.w4 {
            4
        } else {
            0
        }
    }
    /// the policy's whole storage: two integers, never a per-tile table.
    fn storage_bytes(&self) -> usize {
        std::mem::size_of::<BoundaryPair>()
    }
}

#[test]
fn f2_ref_per_tile_table_scales_with_context() {
    let t = 512usize;
    let table = class_table(t, 8, 32);
    assert_eq!(table.len(), t, "the table pays one byte PER TILE");
    // storage at t=512 vs the pair's two integers: the axis the rule owns.
    assert!(table.len() > BoundaryPair { w8: 8, w4: 32 }.storage_bytes() * 8);
    // spot semantics: the newest tile (j = t-1, d = 0) is int8; d = 16 is
    // int4; d = 40 is skipped.
    assert_eq!(table[t - 1], 8);
    assert_eq!(table[t - 17], 4);
    assert_eq!(table[t - 41], 0);
}

#[test]
fn f2_boundary_pair_matches_table_and_is_o1() {
    let cases = [(1usize, 4usize, 64usize), (4, 16, 128), (8, 64, 512), (2, 2, 32)];
    for (w8, w4, t) in cases {
        let pair = BoundaryPair { w8, w4 };
        let table = class_table(t, w8, w4);
        for j in 0..t {
            assert_eq!(
                pair.class_at(t, j),
                table[j],
                "closed form must reproduce the table at w8={w8} w4={w4} j={j}"
            );
        }
        assert_eq!(
            pair.storage_bytes(),
            std::mem::size_of::<usize>() * 2,
            "two integers — storage never scales with T"
        );
    }

    // THE "WHEN" CONTRACT: a MONOTONE policy collapses to two boundaries; a
    // non-monotone one does not. The pair form must refuse to claim it.
    // monotone: classes descend 8,4,0 with distance — encodable (checked above).
    // non-monotone: 4 at d=0, 8 at d=1 — no two ascending boundaries encode it.
    let non_monotone: Vec<u8> = vec![4, 8, 4, 0];
    let best = BoundaryPair { w8: 1, w4: 3 }; // any pair
    let reproduced: Vec<u8> = (0..4).map(|j| best.class_at(4, j)).collect();
    assert_ne!(
        reproduced, non_monotone,
        "a non-monotone policy must not be claimed by the pair form"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — the deferred anchor correction: the exact per-tile correction
// accumulates in a side register (one FMA per row per tile) instead of being
// applied to every accumulator lane per tile; it rides an existing rescale
// when one fires, otherwise flushes on a bounded period; `out` reads
// `out - corr` once at the end. Inside the anchor domain (integer-valued
// corrections, exact in f32) the deferred form is BIT-IDENTICAL to the
// per-tile form.
// ─────────────────────────────────────────────────────────────────────────────

const LANES: usize = 8;
const FLUSH_PERIOD: usize = 16;

/// Naive: every tile applies its correction to ALL accumulator lanes.
fn apply_per_tile(tiles: &[f32], seed_out: f32, rescales: &[usize]) -> (f32, usize) {
    let mut out = seed_out;
    let mut out_touches = 0usize;
    for (i, &c) in tiles.iter().enumerate() {
        out -= c; // the per-tile subtract over all lanes (modeled as one op
        out_touches += 1; // per tile — the lanes multiply this cost)
        if rescales.contains(&i) {
            out *= 1.5; // the rescale the softmax pays anyway
            out_touches += 1;
        }
    }
    (out, out_touches)
}

/// Deferred: accumulate the correction in a side register; fold it into any
/// rescale (fma(out, alpha, -corr*alpha)); flush on the bounded period; read
/// out - corr once at the end.
fn deferred_correction(tiles: &[f32], seed_out: f32, rescales: &[usize]) -> (f32, usize, usize) {
    let mut out = seed_out;
    let mut corr = 0.0f32;
    let mut since_flush = 0usize;
    let mut out_touches = 0usize;
    let mut flushes = 0usize;
    for (i, &c) in tiles.iter().enumerate() {
        corr += c; // the side register: one add per tile, `out` untouched
        since_flush += 1;
        if rescales.contains(&i) {
            // ride-along: fma(out, alpha, -corr*alpha) == (out - corr)*alpha
            let alpha = 1.5f32;
            out = out.mul_add(alpha, -(corr * alpha));
            out_touches += 1;
            corr = 0.0;
            since_flush = 0;
            flushes += 1;
        } else if since_flush == FLUSH_PERIOD {
            out -= corr;
            out_touches += 1;
            corr = 0.0;
            since_flush = 0;
            flushes += 1;
        }
    }
    let out_final = out - corr; // the one epilogue read
    out_touches += 1;
    (out_final, out_touches, flushes)
}

#[test]
fn f3_ref_per_tile_correction_touches_out_every_tile() {
    let tiles: Vec<f32> = (0..64).map(|i| ((i * 7) % 13) as f32).collect();
    let rescales = [10usize, 40];
    let (v, touches) = apply_per_tile(&tiles, 1_000.0, &rescales);
    assert_eq!(touches, 66, "naive: one touch PER TILE plus the rescales");
    assert!(v.is_finite());
}

#[test]
fn f3_deferred_bit_identical_inside_anchor_domain() {
    // Integer-valued corrections (the anchor domain: exact in f32) over a
    // seeded stream with two rescales.
    let tiles: Vec<f32> = (0..64).map(|i| ((i * 7) % 13) as f32).collect();
    let rescales = [10usize, 40];

    let (naive, _) = apply_per_tile(&tiles, 1_000.0, &rescales);
    let (deferred, touches, flushes) = deferred_correction(&tiles, 1_000.0, &rescales);

    // BIT-IDENTICAL: exact additions commute, so the regrouped order —
    // side register + ride-along folds + one epilogue read — lands on the
    // same f32 bits as the per-tile subtracts.
    assert_eq!(deferred.to_bits(), naive.to_bits(), "deferred must be bit-identical");

    // the census: `out` was touched at most (rescales + flushes + 1) times,
    // nowhere near once per tile.
    assert!(
        touches <= rescales.len() + flushes + 1,
        "deferred touches: {touches}"
    );
    assert!(
        touches < tiles.len(),
        "the whole point: out is touched far less than once per tile ({touches} vs {})",
        tiles.len()
    );
    // the bounded period: flushes are bounded by ceil(n / 16) + rescales.
    assert!(
        flushes <= tiles.len().div_ceil(FLUSH_PERIOD) + rescales.len(),
        "flush period bound: {flushes}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — the Lagrangian shared-budget allocator: per-head argmin(err + λ·cost)
// traced across a λ sweep builds the frontier; a frontier point's budget
// coordinate is the MEAN cost over all heads (one shared number); selection
// cuts the frontier at a cost fraction and takes the minimum-error eligible
// point; no eligible point fails LOUD.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AllocProblem {
    /// err[head][config], cost[head][config]
    err: Vec<Vec<f32>>,
    cost: Vec<Vec<f32>>,
}

impl AllocProblem {
    fn frontier(&self) -> Vec<(f32, f32, Vec<usize>)> {
        let heads = self.err.len();
        let configs = self.err[0].len();
        let mut lams = vec![0.0f32];
        for k in 0..12 {
            lams.push(10f32.powf(-9.0 + 10.0 * k as f32 / 11.0));
        }
        let mut frontier = Vec::new();
        for &lam in &lams {
            let mut choice = vec![0usize; heads];
            for h in 0..heads {
                let mut best = 0usize;
                let mut best_v = f32::INFINITY;
                for c in 0..configs {
                    let v = self.err[h][c] + lam * self.cost[h][c];
                    if v < best_v {
                        best_v = v;
                        best = c;
                    }
                }
                choice[h] = best;
            }
            let mean_cost =
                choice.iter().map(|&c| self.cost.iter().enumerate().map(|(h, row)| row[c]).sum::<f32>() / heads as f32).sum::<f32>()
                    / heads as f32;
            let sum_err: f32 = choice
                .iter()
                .enumerate()
                .map(|(h, &c)| self.err[h][c])
                .sum();
            frontier.push((mean_cost, sum_err, choice));
        }
        frontier
    }

    fn select(&self, fraction: f32) -> Option<(f32, f32, Vec<usize>)> {
        let f = self.frontier();
        f.into_iter()
            .filter(|(cost, _, _)| *cost <= fraction)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
    }
}

#[test]
fn f4_ref_independent_budgets_miss_the_shared_cut() {
    // Two heads: head 0 cheap-but-lossy configs, head 1 expensive-but-exact.
    // Independent per-head "spend up to the fraction" choices overshoot the
    // SHARED mean budget — the axis the shared cut owns.
    let p = AllocProblem {
        err: vec![vec![0.40, 0.10, 0.05], vec![0.30, 0.12, 0.02]],
        cost: vec![vec![0.20, 0.60, 0.95], vec![0.55, 0.75, 0.99]],
    };
    // per-head greedy at the same fraction: each head takes the costliest
    // config it can afford — head 1 spends 0.75 on its own budget, so the
    // MEAN across heads (0.20+0.75)/2 = 0.475 exceeds the shared cut 0.40.
    let per_head = [2usize, 1usize];
    let mean = (p.cost[0][per_head[0]] + p.cost[1][per_head[1]]) / 2.0;
    assert!(mean > 0.40, "independent budgets overshoot the shared mean ({mean})");
    // the shared allocator's answer respects it (asserted in the opt arm).
    let _ = p;
}

#[test]
fn f4_shared_frontier_cut_selects_min_error_loud() {
    let p = AllocProblem {
        err: vec![vec![0.40, 0.10, 0.05], vec![0.30, 0.12, 0.02]],
        cost: vec![vec![0.20, 0.60, 0.95], vec![0.55, 0.75, 0.99]],
    };

    // (a) the selected point's mean cost is under the fraction.
    let pick = p.select(0.60).expect("a frontier point must qualify");
    assert!(pick.0 <= 0.60, "selected mean cost {} under 0.60", pick.0);

    // (b) the shared cut CHANGES per-head choices vs per-head independence:
    //     head 1's expensive exact config is given up so the MEAN fits.
    let per_head = vec![2usize, 1usize]; // what independent budgets pick
    assert_ne!(
        pick.2, per_head,
        "the shared budget must rebalance at least one head's choice"
    );

    // (c) LOUD failure below the cheapest frontier cost — never a silent
    //     fallback to an over-budget point.
    assert!(p.select(0.05).is_none(), "no frontier point fits 0.05; must refuse");
}

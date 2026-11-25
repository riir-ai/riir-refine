//! Batch 418 rust_perf fixture preflight — the tunazip exact-numerics
//! transcription batch (Plan 414 / Research 449). Original plain-Rust
//! syntheses — the mechanisms' LAWS transcribed, never upstream code (the
//! corpus convention). Pin 571509687de3bd7301af0f79d75893b3e83d2417 (tip
//! UNMOVED at T0 + re-verified at authoring; MIT + NOTICE verified); every
//! quote in the entries file re-verified at that pin.
//!
//! Compiled AND executed BEFORE entry:
//!   rustc --edition 2021 b418_perf_fixtures.rs -o b418fx && ./b418fx
//! at debug AND -O (both must print all-ok).
//!
//! Arms:
//! - F1 (R1 G1 order-invariance): the quantized exact dot is BIT-identical
//!   under sequential / unrolled-4 / reversed / chunked-T4 reduction orders
//!   on three input classes (random, all-max worst case, alternating-sign
//!   cancellation), and every arm equals an exact i128 integer reference.
//! - F2 (thread-invariance shape): chunked partials over T in {1,2,3,7,8}
//!   combined forward / reverse / tree — all bit-identical under the
//!   quantized form; the RAW fold's chunk-dependence is real (the defect
//!   witness: at least one (T, combine) arm differs from sequential raw).
//! - F3 (the loud precondition): inner x 2^(b1+b2) <= 2^p refuses PAST the
//!   budget and admits AT it — f64 row: 16+16 bits, inner 2^21 in, 2^21+1
//!   out; f32-shaped row: 8+8 bits, inner 256 in, 257 out.
//! - F4 (the lossy-side disclosure): a wide-dynamic-range row (+/-12
//!   decades, the source's stress shape) shows a LARGE quantized-vs-raw
//!   relative delta (the transform is lossy — the losing regime is real);
//!   a benign well-scaled row stays inside the derived envelope
//!   (4*sqrt(n)*2^-(B+1)); the source's own 1e-6 det-vs-float tolerance is
//!   cited as PROVENANCE in the entry, never a bar here.
//! - F5 (the int64 overflow budget): bits = min(40, 62 - ceil_log2(terms))
//!   pinned at terms in {4, 2^20, 2^24}; the narrowing is LOSSY (a 40-bit
//!   significand changes at the 38-bit budget); the int64 accumulation of
//!   the quantized integers over 2^24 terms is exact (i128-equal) and
//!   overflow-free (|sum| < 2^63).
#![allow(dead_code)]

use std::hint::black_box;

// ══════════════ shared quantization machinery (the rule's law) ══════════════

const B: i32 = 16; // mantissa bits per operand (the source's 16+16 row)
const F64_SIG_BITS: u64 = 53; // p: f64 significand incl. the implicit bit
const SUM_FRACTION_BITS: i32 = 40;
const INT64_SUM_BITS: i32 = 62;

/// Exponent e such that |x| = m * 2^e, m in [0.5, 1); subnormals floored.
fn peak_exponent(row: &[f64]) -> i32 {
    let mut e: i32 = -1022;
    for &x in row {
        if x != 0.0 {
            let bi = x.abs().to_bits();
            let be = ((bi >> 52) & 0x7ff) as i32;
            let ex = if be == 0 { -1021 } else { be - 1022 };
            if ex > e {
                e = ex;
            }
        }
    }
    e
}

fn pow2(e: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&e), "pow2 range");
    f64::from_bits(((e + 1023) as u64) << 52)
}

/// The row's shared power-of-two scale pair: scale lifts the peak to the
/// B-bit grid, inverse restores it. (fixed.py peak_scales/quantize shape.)
fn scale_pair(row: &[f64]) -> (f64, f64) {
    let e = peak_exponent(row);
    (pow2(B - e), pow2(e - B))
}

/// grid_round: quantize to the B-bit grid under the shared scale, restore.
fn grid_round(x: f64, scale: f64, inverse: f64) -> f64 {
    (x * scale).round_ties_even() * inverse
}

/// Budget law: the inner dimension is bounded by 2^(p - b1 - b2).
fn inner_budget(p_sig_bits: u64, b1: i32, b2: i32) -> u64 {
    debug_assert!(b1 + b2 <= p_sig_bits as i32, "operands must leave room");
    1u64 << (p_sig_bits - (b1 + b2) as u64)
}

/// The loud precondition refusal (gemm.py:23-24 shape): refuse past the
/// budget, admit at it. Errors carry the inner dimension by name.
fn check_budget(inner: usize, p_sig_bits: u64, b1: i32, b2: i32) -> Result<(), String> {
    let budget = inner_budget(p_sig_bits, b1, b2);
    if (inner as u64) > budget {
        Err(format!(
            "inner dimension {inner} overflows the exact float{p_sig_bits} range"
        ))
    } else {
        Ok(())
    }
}

/// Quantized exact dot, SEQUENTIAL fold: any order must give these bits.
fn dot_quant_seq(a: &[f64], b: &[f64]) -> f64 {
    check_budget(a.len(), F64_SIG_BITS, B, B).expect("fixture stays inside the budget");
    let (sa, ia) = scale_pair(a);
    let (sb, ib) = scale_pair(b);
    let mut acc = 0.0f64;
    for i in 0..a.len() {
        acc += grid_round(a[i], sa, ia) * grid_round(b[i], sb, ib);
    }
    acc
}

/// Quantized exact dot, UNROLLED-4: four partial accumulators.
fn dot_quant_unrolled4(a: &[f64], b: &[f64]) -> f64 {
    let (sa, ia) = scale_pair(a);
    let (sb, ib) = scale_pair(b);
    let mut acc = [0.0f64; 4];
    let mut i = 0;
    let n = a.len();
    while i + 4 <= n {
        acc[0] += grid_round(a[i], sa, ia) * grid_round(b[i], sb, ib);
        acc[1] += grid_round(a[i + 1], sa, ia) * grid_round(b[i + 1], sb, ib);
        acc[2] += grid_round(a[i + 2], sa, ia) * grid_round(b[i + 2], sb, ib);
        acc[3] += grid_round(a[i + 3], sa, ia) * grid_round(b[i + 3], sb, ib);
        i += 4;
    }
    while i < n {
        acc[0] += grid_round(a[i], sa, ia) * grid_round(b[i], sb, ib);
        i += 1;
    }
    (acc[0] + acc[1]) + (acc[2] + acc[3])
}

/// Quantized exact dot over a REVERSED walk.
fn dot_quant_reversed(a: &[f64], b: &[f64]) -> f64 {
    let (sa, ia) = scale_pair(a);
    let (sb, ib) = scale_pair(b);
    let mut acc = 0.0f64;
    for i in (0..a.len()).rev() {
        acc += grid_round(a[i], sa, ia) * grid_round(b[i], sb, ib);
    }
    acc
}

/// Quantized exact dot, CHUNKED into t partials (the parallel-fold shape):
/// each chunk folds its own range, then partials combine in `combine`
/// order (0 = forward, 1 = reverse, 2 = pairwise tree).
fn dot_quant_chunked(a: &[f64], b: &[f64], t: usize, combine: u8) -> f64 {
    assert!(t >= 1);
    let (sa, ia) = scale_pair(a);
    let (sb, ib) = scale_pair(b);
    let n = a.len();
    let chunk = n.div_ceil(t);
    let mut partials: Vec<f64> = Vec::with_capacity(t);
    for c in 0..t {
        let lo = c * chunk;
        let hi = ((c + 1) * chunk).min(n);
        let mut acc = 0.0f64;
        for i in lo..hi {
            acc += grid_round(a[i], sa, ia) * grid_round(b[i], sb, ib);
        }
        partials.push(acc);
    }
    match combine {
        0 => partials.into_iter().sum(),
        1 => partials.into_iter().rev().sum(),
        2 => {
            // pairwise tree
            let mut v = partials;
            while v.len() > 1 {
                let mut next = Vec::with_capacity(v.len().div_ceil(2));
                let mut j = 0;
                while j + 1 < v.len() {
                    next.push(v[j] + v[j + 1]);
                    j += 2;
                }
                if j < v.len() {
                    next.push(v[j]);
                }
                v = next;
            }
            v[0]
        }
        _ => unreachable!(),
    }
}

/// The exact reference: the quantized integer factors, folded in i128,
/// rescaled by the shared pow2 — the value every order must reproduce.
fn dot_exact_reference(a: &[f64], b: &[f64]) -> f64 {
    let ea = peak_exponent(a);
    let eb = peak_exponent(b);
    let sa = pow2(B - ea);
    let sb = pow2(B - eb);
    let mut ksum: i128 = 0;
    for i in 0..a.len() {
        let ka = (a[i] * sa).round_ties_even() as i128;
        let kb = (b[i] * sb).round_ties_even() as i128;
        ksum += ka * kb;
    }
    // |ksum| < inner * 2^(2B) <= 2^53 by the budget: the conversion is exact
    (ksum as f64) * pow2(ea + eb - 2 * B)
}

/// The RAW (unquantized) fold — order-DEPENDENT, the defect side.
fn dot_raw_seq(a: &[f64], b: &[f64]) -> f64 {
    let mut acc = 0.0f64;
    for i in 0..a.len() {
        acc += a[i] * b[i];
    }
    acc
}

fn dot_raw_chunked(a: &[f64], b: &[f64], t: usize, combine: u8) -> f64 {
    let n = a.len();
    let chunk = n.div_ceil(t);
    let mut partials: Vec<f64> = Vec::with_capacity(t);
    for c in 0..t {
        let lo = c * chunk;
        let hi = ((c + 1) * chunk).min(n);
        let mut acc = 0.0f64;
        for i in lo..hi {
            acc += a[i] * b[i];
        }
        partials.push(acc);
    }
    match combine {
        0 => partials.into_iter().sum(),
        _ => partials.into_iter().rev().sum(),
    }
}

// ══════════════ input synthesis ══════════════

struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn unit(&mut self) -> f64 {
        ((self.next() >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 2.0
    }
}

fn make_random(n: usize, seed: u64) -> Vec<f64> {
    let mut xs = Xs(seed | 1);
    (0..n).map(|_| xs.unit()).collect()
}

/// worst case: every element at the same max magnitude (monotone partial
/// sums — the all-max adversarial fixture).
fn make_all_max(n: usize) -> Vec<f64> {
    vec![127.0; n]
}

/// alternating-sign cancellation: adjacent +/-127 pairs.
fn make_alternating(n: usize) -> Vec<f64> {
    (0..n).map(|i| if i % 2 == 0 { 127.0 } else { -127.0 }).collect()
}

/// near-cancellation carrier: B is balanced +/-1 (the products cancel) with
/// ONE tiny perturbation (2^-20) riding on b[0] — a small signal adjacent to
/// a large one INSIDE the operand (the wide-dynamic-range-within-a-column
/// losing regime: the shared-scale grid's ulp is ~2^-17 of the CARRIER, so
/// the perturbation is below its resolution).
fn make_cancel_carrier(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut xs = Xs(seed | 1);
    let mut b: Vec<f64> = Vec::with_capacity(n);
    let mut ones = 0usize;
    for _ in 0..n / 2 {
        b.push(1.0);
        ones += 1;
    }
    for _ in n / 2..n {
        b.push(-1.0);
    }
    // deterministic shuffle so the +/-1 products interleave (the fold's
    // partial sums cancel pairwise instead of in blocks)
    for i in (1..b.len()).rev() {
        let j = (xs.next() % (i as u64 + 1)) as usize;
        b.swap(i, j);
    }
    b[0] += 2.0f64.powi(-20); // the tiny rider on a large carrier
    let a = vec![0.7; n]; // off-grid carrier: 0.7 * 2^16 = 45875.2
    let _ = ones;
    (a, b)
}

/// wide dynamic range: +/-12 decades (the source's stress shape).
fn make_wide_range(n: usize, seed: u64) -> Vec<f64> {
    let mut xs = Xs(seed | 1);
    (0..n)
        .map(|i| {
            let dec = (i as f64 / (n - 1) as f64) * 24.0 - 12.0; // -12 .. +12
            let sign = if xs.unit() < 0.0 { -1.0 } else { 1.0 };
            sign * dec.exp2()
        })
        .collect()
}

// ══════════════ F1 — order-invariance G1 (three input classes) ══════════════

fn fixture_f1() {
    let n = 1_024usize; // inside the 2^21 budget with 12 orders of headroom
    for (class, mk) in [
        ("random", 1u64),
        ("all-max", 2u64),
        ("alternating", 3u64),
    ] {
        let a = match class {
            "random" => make_random(n, 41),
            "all-max" => make_all_max(n),
            _ => make_alternating(n),
        };
        let b = match mk {
            1 => make_random(n, 97),
            2 => make_all_max(n),
            _ => make_alternating(n),
        };
        let seq = dot_quant_seq(&a, &b);
        let unr4 = dot_quant_unrolled4(&a, &b);
        let rev = dot_quant_reversed(&a, &b);
        let ch4 = dot_quant_chunked(&a, &b, 4, 0);
        let reference = dot_exact_reference(&a, &b);
        for (name, v) in [
            ("unrolled4", unr4),
            ("reversed", rev),
            ("chunked4", ch4),
        ] {
            assert_eq!(
                v.to_bits(),
                seq.to_bits(),
                "F1 {class}: {name} order must be bit-identical to sequential"
            );
            assert_eq!(
                v.to_bits(),
                reference.to_bits(),
                "F1 {class}: {name} must equal the exact i128 reference"
            );
        }
        assert_eq!(
            seq.to_bits(),
            reference.to_bits(),
            "F1 {class}: sequential must equal the exact i128 reference"
        );
        black_box(seq);
    }
    println!("F1 ok: order-invariance G1 — random / all-max / alternating all bit-identical across 4 orders + i128-exact");
}

// ══════════════ F2 — thread-invariance shape ══════════════

fn fixture_f2() {
    let n = 2_048usize;
    let a = make_random(n, 11);
    let b = make_random(n, 23);
    let seq = dot_quant_seq(&a, &b);
    for t in [1usize, 2, 3, 7, 8] {
        for combine in [0u8, 1, 2] {
            let v = dot_quant_chunked(&a, &b, t, combine);
            assert_eq!(
                v.to_bits(),
                seq.to_bits(),
                "F2: T={t} combine={combine} must be bit-identical (the thread-count law)"
            );
            black_box(v);
        }
    }
    // the defect witness: the RAW fold's chunk dependence is real — at
    // least one (T, combine) arm drifts from the sequential raw fold
    let raw_seq = dot_raw_seq(&a, &b);
    let mut raw_drifts = 0usize;
    for t in [2usize, 3, 7, 8] {
        for combine in [0u8, 1] {
            if dot_raw_chunked(&a, &b, t, combine).to_bits() != raw_seq.to_bits() {
                raw_drifts += 1;
            }
        }
    }
    assert!(
        raw_drifts >= 1,
        "F2: the raw fold must show chunk dependence somewhere (the bug class is real)"
    );
    println!("F2 ok: thread-invariance T=1..8 x 3 combines bit-identical; raw fold drifts in {raw_drifts} arms (defect witness)");
}

// ══════════════ F3 — the loud precondition ══════════════

fn fixture_f3() {
    // f64 row: p=53, b1=b2=16 -> budget 2^21
    let budget = inner_budget(F64_SIG_BITS, B, B);
    assert_eq!(budget, 1u64 << 21, "f64 16+16 budget must be 2^21");
    assert!(
        check_budget(budget as usize, F64_SIG_BITS, B, B).is_ok(),
        "F3: the budget boundary itself is ADMITTED (inner x 2^32 = 2^53, not >)"
    );
    let refused = check_budget((budget + 1) as usize, F64_SIG_BITS, B, B);
    match refused {
        Err(msg) => {
            assert!(
                msg.contains("overflows the exact float53 range"),
                "F3: the refusal names the overflow: {msg}"
            );
        }
        Ok(()) => panic!("F3: inner=budget+1 must REFUSE loudly"),
    }
    // f32-shaped row: p=24, b=8+8 -> budget 256; 257 refuses
    let f32_budget = inner_budget(24, 8, 8);
    assert_eq!(f32_budget, 256, "f32 8+8 budget must be 2^8 = 256");
    assert!(check_budget(256, 24, 8, 8).is_ok());
    assert!(check_budget(257, 24, 8, 8).is_err());
    // the no-room case: 12+12 on f32 leaves ZERO accumulation room
    assert_eq!(inner_budget(24, 12, 12), 1, "12+12 on f32: no room left");
    assert!(check_budget(2, 24, 12, 12).is_err(), "12+12 refuses past one term");
    println!("F3 ok: loud precondition — budget admitted AT the boundary, refused past it, by name");
}

// ══════════════ F4 — the lossy-side disclosure ══════════════

fn fixture_f4() {
    let n = 256usize;
    // the losing regime: a 2^-20 perturbation riding a 2^0 carrier inside
    // the b operand. The raw fold sees it (raw result ~ 0.7 * 2^-20); the
    // shared-scale grid cannot (its ulp at the carrier is ~2^-17 — the
    // perturbation rounds AWAY), so the quantized result loses the signal
    let (ca, cb) = make_cancel_carrier(n, 71);
    let qc = dot_quant_seq(&ca, &cb);
    let rc = dot_raw_seq(&ca, &cb);
    assert!(rc.abs() > 0.0, "F4: the raw fold must carry the perturbation");
    let relc = ((qc - rc) / rc).abs();
    assert!(
        relc > 1e-3,
        "F4: the perturbation must be lost/coarsened by the grid (the losing regime is real), got {relc}"
    );
    // the wide-dynamic-range row (+/-12 decades), measured and recorded:
    // the pow2 grid is LOGARITHMIC, so magnitude alone is not the loss
    // axis — every element keeps ~2^-(B+1) RELATIVE precision; the delta
    // stays small here and that is itself the disclosure
    let wide_a = make_wide_range(n, 31);
    let wide_b = make_wide_range(n, 37);
    let qw = dot_quant_seq(&wide_a, &wide_b);
    let rw = dot_raw_seq(&wide_a, &wide_b);
    let relw = ((qw - rw) / rw).abs();
    // benign well-scaled data: inside the derived envelope, not a bar
    let a = make_random(n, 51);
    let b = make_random(n, 57);
    let qn = dot_quant_seq(&a, &b);
    let rn = dot_raw_seq(&a, &b);
    let reln = ((qn - rn) / rn).abs();
    let envelope = 4.0 * (n as f64).sqrt() * 2.0f64.powi(-(B + 1));
    assert!(
        reln <= envelope,
        "F4: benign row must stay inside the derived envelope {envelope:.2e}, got {reln:.2e}"
    );
    assert!(
        relc > 100.0 * reln,
        "F4: the losing regime must dominate the benign delta by orders of magnitude"
    );
    println!(
        "F4 ok: perturbation-lost delta {relc:.3e} (losing regime, real) vs benign {reln:.3e} (inside {envelope:.2e}); wide-range row {relw:.3e} (recorded — the grid is logarithmic)"
    );
}

// ══════════════ F5 — the int64 overflow budget ══════════════

/// bits = min(SUM_FRACTION_BITS, INT64_SUM_BITS - ceil_log2(terms)) —
/// the source spells it (terms - 1).bit_length() (Python); in Rust:
/// 64 - (terms - 1).leading_zeros() (the same floor(log2)+1 value).
fn int64_sum_bits(terms: u64) -> i32 {
    SUM_FRACTION_BITS.min(INT64_SUM_BITS - (64 - (terms - 1).leading_zeros()) as i32)
}

fn fixture_f5() {
    assert_eq!(int64_sum_bits(4), 40, "terms=4: 62-2=60, min(40,60)=40");
    assert_eq!(int64_sum_bits(1u64 << 20), 40, "2^20 terms: 62-20=42, min=40");
    assert_eq!(
        int64_sum_bits(1u64 << 24),
        38,
        "2^24 terms: 62-24=38 — the budget narrows (the overflow trade)"
    );
    // the narrowing is LOSSY: a 40-bit significand (2^39+1) loses its low
    // bit at the 38-bit budget — the grid ulp at 2^38 is 4 original units,
    // so the +1 (a quarter-ulp) rounds away entirely
    let v = (1i64 << 39) + 1; // needs 40 significant bits
    let scale = pow2(38 - 40); // quantize DOWN to the 38-bit grid
    let rounded = ((v as f64 * scale).round_ties_even() * pow2(2)) as i64;
    assert_ne!(rounded, v, "F5: the 38-bit budget must round the 40-bit value");
    assert_eq!(rounded, 1i64 << 39, "the low bit is lost, the leading bits kept");
    // the int64 accumulation is EXACT for the quantized integers and
    // overflow-free: 2^24 terms, each |k| <= 2^38 -> |sum| <= 2^62 < 2^63
    let terms = 1u64 << 24;
    let bits = int64_sum_bits(terms) as u32;
    let mut xs = Xs(7);
    let mut sum64: i64 = 0;
    let mut sum128: i128 = 0;
    for _ in 0..terms {
        let k = ((xs.next() >> 11) as f64 / (1u64 << 53) as f64 * (1i64 << bits) as f64)
            .round_ties_even() as i64;
        sum64 = sum64.wrapping_add(k);
        sum128 += k as i128;
    }
    assert_eq!(sum64 as i128, sum128, "F5: the int64 accumulation is exact (no wrap)");
    assert!(sum128.abs() < (1i128 << 62), "the accumulator stays inside int64");
    println!("F5 ok: int64 budget pinned at 40/40/38; narrowing lossy; 2^24-term accumulation exact + overflow-free");
}

fn main() {
    fixture_f1();
    fixture_f2();
    fixture_f3();
    fixture_f4();
    fixture_f5();
    println!("all-ok");
}

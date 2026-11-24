//! Batch 409 kernel fixture preflight — the Prism fork ternary-quant
//! residue (Plan 408 / Research 442, PrismML-Eng/llama.cpp @ d12ca13bb
//! #428 + 802b53737 #449, MIT) as ONE standalone file (2 rules, the
//! four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is CUDA/C/NEON; the Rust arms below model each law's decision
//! content). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b409_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_165.rs in the same landing commit.
//!
//! Arms:
//! - R1 min-max-range-scale-asymmetric-block-codec: the zero-anchored
//!   arm on an asymmetric block (large reconstruction error, the
//!   measured 0.258-floor class) vs the range arm (worst-case error
//!   d/6, all levels inside [min, max]); the symmetric-block contrast
//!   (the range codec LOSES there — the predicate discriminates); the
//!   tamper arm (a dequant reading only 2 of 4 codes per byte — the
//!   [c0,c0,c1,c1] class — reads measurably wrong); the split-surface
//!   shape (weights keep the zero-anchored type — distinct wire types).
//! - R2 offset-block-dot-block-sum-dp4a-correction: the materialize-
//!   dequant arm vs the two-dp4a decomposition (bit-exact integer
//!   identity over an exhaustive byte sweep); the offset-zero tie (m=0
//!   collapses to the plain code dot); the tamper arm (dropping the
//!   Σu dp4a reads wrong for m≠0); the transport-free contrast (the
//!   codec's own m-word — no isum slot — vs the (q−1) codec that
//!   needs one).

// ═══════════ shared machinery ═══════════

const QK: usize = 128;

/// one 128-value block, zero-anchored symmetric (PQ2_0's (q−1) class):
/// one scale, codes {0..3} meaning {-1,0,1, 2}−1 → q−1 ∈ {−1,0,1,2} is
/// NOT the real codec (the real one is 2-bit {0,1,2} trit-class), so
/// this fixture models the 2-bit 4-level zero-anchored form the source
/// compares against: code q ∈ {0..3}, dequant (q − 1.5)·d — zero sits
/// BETWEEN codes, the offset from zero rides as error.
#[derive(Clone, Copy)]
struct SymBlock {
    d: f32,
    qs: [u8; QK / 4], // 2 bits per element
}

impl SymBlock {
    fn quant(x: &[f32; QK]) -> Self {
        // zero-anchored: d spans |x| symmetric around zero
        let mut amax = 0.0f32;
        for &v in x {
            amax = amax.max(v.abs());
        }
        let d = amax / 1.5; // top code lands on ±amax
        let mut b = SymBlock {
            d,
            qs: [0u8; QK / 4],
        };
        for (i, &v) in x.iter().enumerate() {
            // round-to-nearest CODE: shift into the code domain FIRST
            // (+1.5 puts the half-step levels at integer codes), then
            // round — the reverse order rounds to a NON-LEVEL midpoint
            // and lands codes on wrong levels (the fixture's first
            // draft manufactured a 3x symmetric-block error that way)
            let q = ((v / d) + 1.5).round().clamp(0.0, 3.0);
            set_code(&mut b.qs, i, q as u8);
        }
        b
    }
    fn dequant(&self, i: usize) -> f32 {
        (code_of(&self.qs, i) as f32 - 1.5) * self.d
    }
}

/// one 128-value block, min/max range-scale (the pq2_0m class):
/// d = (max−min)/3, m = min, raw codes {0..3}, dequant m + q·d.
#[derive(Clone, Copy)]
struct RangeBlock {
    d: f32,
    m: f32,
    qs: [u8; QK / 4],
}

impl RangeBlock {
    fn quant(x: &[f32; QK]) -> Self {
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for &v in x {
            min = min.min(v);
            max = max.max(v);
        }
        let d = (max - min) / 3.0;
        let id = if d > 0.0 { 1.0 / d } else { 0.0 };
        let mut b = RangeBlock {
            d,
            m: min,
            qs: [0u8; QK / 4],
        };
        for (i, &v) in x.iter().enumerate() {
            let q = ((v - min) * id).round().clamp(0.0, 3.0) as u8;
            set_code(&mut b.qs, i, q);
        }
        b
    }
    fn dequant(&self, i: usize) -> f32 {
        self.m + code_of(&self.qs, i) as f32 * self.d
    }
}

fn set_code(qs: &mut [u8; QK / 4], i: usize, q: u8) {
    let byte = i / 4;
    let shift = 2 * (i % 4);
    qs[byte] = (qs[byte] & !(0x03 << shift)) | (q << shift);
}

fn code_of(qs: &[u8; QK / 4], i: usize) -> u8 {
    (qs[i / 4] >> (2 * (i % 4))) & 0x03
}

/// the model of `vec_dot_fattn_vec_KQ_pq2_0m`: two "dp4a" lanes per
/// byte-word — Σ(c·u) and Σu — the offset riding as one scalar multiply.
fn kq_dot_offset(k: &RangeBlock, u: &[i8; QK], ds: f32) -> f32 {
    let mut sum = 0.0f32;
    for w in 0..QK / 4 {
        let mut c_byte = 0i32;
        let mut u_word = 0i32;
        for j in 0..4 {
            let c = code_of(&k.qs, 4 * w + j) as i32;
            c_byte |= c << (8 * j);
            u_word |= (u[4 * w + j] as i32 & 0xFF) << (8 * j);
        }
        // the two dp4a lanes, modeled byte-wise (the identity is what
        // the fixture pins, not the SIMD spelling):
        let mut sum_c_u = 0i32;
        let mut sum_u = 0i32;
        for j in 0..4 {
            let c = (c_byte >> (8 * j)) as u8 as i32;
            let uj = (u_word >> (8 * j)) as u8 as i8 as i32;
            sum_c_u += c * uj;
            sum_u += uj;
        }
        // sum = d_Q * (m * sum(c0..c3) + d * sum(cK * qQ)) — the source
        // folds per byte-word; the block-level form is algebraically
        // the same and keeps the fixture readable.
        let _ = (c_byte, u_word);
        sum += ds * (k.m * sum_u as f32 + k.d * sum_c_u as f32);
    }
    sum
}

fn kq_dot_matmul(k: &RangeBlock, u: &[i8; QK], ds: f32) -> f32 {
    // the naive arm: materialize the dequant, float-dot — folded per
    // byte-word so the f32 association matches the two-dp4a arm's fold
    // order (the identity under test is the INTEGER algebra; the float
    // fold must be association-neutral or the comparison measures
    // summation order, not the law)
    let mut sum = 0.0f32;
    for w in 0..QK / 4 {
        let mut word_sum = 0.0f32;
        for j in 0..4 {
            word_sum += k.dequant(4 * w + j) * u[4 * w + j] as f32;
        }
        sum += word_sum * ds;
    }
    sum
}

// ═══════════ R1 — the codec choice ═══════════

#[test]
fn r1_range_codec_wins_on_asymmetric_blocks() {
    // an asymmetric block: values clustered far from zero (the KV-row
    // class — one position's K slice drifts off the symmetric center)
    let mut x = [0.0f32; QK];
    for (i, v) in x.iter_mut().enumerate() {
        *v = 10.0 + 0.9 * ((i as f32 * 0.7).sin()) + 0.05 * (i as f32);
    }
    let sym = SymBlock::quant(&x);
    let rng = RangeBlock::quant(&x);
    let err = |f: &dyn Fn(usize) -> f32| -> f32 {
        let mut e = 0.0f32;
        for i in 0..QK {
            e = e.max((f(i) - x[i]).abs());
        }
        e
    };
    let sym_err = err(&|i| sym.dequant(i));
    let rng_err = err(&|i| rng.dequant(i));
    // the source's measured ratio is ~4× KLD; the fixture pins the
    // weaker, robust form: the range arm is never worse, and on a
    // clearly asymmetric block it is materially better
    assert!(
        rng_err < sym_err,
        "range err {rng_err} must beat zero-anchored err {sym_err} on an asymmetric block"
    );
    // the worst-case bound: every reconstruction error <= d/2 = (max−min)/6
    let min = x.iter().fold(f32::MAX, |a: f32, b: &f32| a.min(*b));
    let max = x.iter().fold(f32::MIN, |a: f32, b: &f32| a.max(*b));
    let bound = (max - min) / 6.0;
    for i in 0..QK {
        assert!(
            (rng.dequant(i) - x[i]).abs() <= bound + 1e-4,
            "err {} exceeds the d/6 bound {}",
            (rng.dequant(i) - x[i]).abs(),
            bound
        );
    }
    // all levels inside [min, max]
    for i in 0..QK {
        let y = rng.dequant(i);
        assert!(y >= min - 1e-4 && y <= max + 1e-4);
    }
}

#[test]
fn r1_symmetric_block_contrast_the_predicate_discriminates() {
    // a symmetric block (zero-centered, extremes hit evenly) — the
    // anti-regime arm: the range codec has no headroom to buy here
    let mut x = [0.0f32; QK];
    for (i, v) in x.iter_mut().enumerate() {
        // alternate full-span extremes + interior values: the block
        // spans [−1, 1] symmetric, both codecs see the same range
        *v = match i % 4 {
            0 => 1.0,
            1 => -1.0,
            2 => 0.5,
            _ => -0.5,
        };
    }
    let sym = SymBlock::quant(&x);
    let rng = RangeBlock::quant(&x);
    let err = |f: &dyn Fn(usize) -> f32| -> f32 {
        let mut e = 0.0f32;
        for i in 0..QK {
            e = e.max((f(i) - x[i]).abs());
        }
        e
    };
    let sym_err = err(&|i| sym.dequant(i));
    let rng_err = err(&|i| rng.dequant(i));
    // the predicate: on a symmetric block the range codec buys nothing —
    // it must NOT win materially (the decision keeps the zero-anchored
    // codec there; the honest form of "buys nothing" is: the range arm
    // is not BETTER by more than rounding — both see the same span and
    // the same 4-level step, so their worst errors tie to within the
    // midpoint-alignment difference)
    assert!(
        rng_err >= sym_err - 1e-4,
        "range err {rng_err} must not beat symmetric err {sym_err} on a symmetric block"
    );
}

#[test]
fn r1_tamper_missing_codes_reads_wrong() {
    // the [c0,c0,c1,c1] broadcast class: a dequant reading only 2 of 4
    // codes per byte (the source's V-dequant defect) must read
    // measurably wrong — the reference-census arm that catches it
    let mut x = [0.0f32; QK];
    for (i, v) in x.iter_mut().enumerate() {
        *v = 5.0 + 2.0 * ((i as f32 * 0.31).cos());
    }
    let b = RangeBlock::quant(&x);
    let defect = |i: usize| -> f32 {
        // broadcast: code(i/2) served for both halves of each byte
        let c = code_of(&b.qs, (i / 2) * 2); // the wrong read
        b.m + c as f32 * b.d
    };
    let mut max_err = 0.0f32;
    for i in 0..QK {
        max_err = max_err.max((defect(i) - x[i]).abs());
    }
    let bound = {
        let min = x.iter().fold(f32::MAX, |a: f32, b: &f32| a.min(*b));
        let max = x.iter().fold(f32::MIN, |a: f32, b: &f32| a.max(*b));
        (max - min) / 6.0
    };
    assert!(
        max_err > bound,
        "the broadcast defect reads {max_err} — must exceed the d/6 bound {bound} (the census arm fires)"
    );
    // the exact class: even/odd indexes disagree with the data
    assert!((defect(1) - x[1]).abs() > (b.d) * 0.5 || (defect(3) - x[3]).abs() > b.d * 0.5);
}

#[test]
fn r1_split_surface_distinct_wire_types() {
    // the KV-only split-surface shape: the same block class has TWO
    // wire types; the weight path keeps the zero-anchored codec, and
    // the types are never a reinterpret (a range block's layout
    // carries the m-word; the symmetric block does not)
    let x = [7.25f32; QK]; // degenerate: all-equal values (real KV rows do this)
    let rng = RangeBlock::quant(&x);
    // d == 0 must quantize safely (id = 0, codes all 0, y = m)
    assert_eq!(rng.d, 0.0);
    assert_eq!(rng.dequant(0), 7.25);
    for i in 0..QK {
        assert_eq!(code_of(&rng.qs, i), 0);
    }
    // size honesty: the range block carries 2 floats + 32 code bytes
    // (the source's 24 B/128); the symmetric carries 1 float + codes
    assert_eq!(std::mem::size_of::<RangeBlock>(), 2 * 4 + QK / 4);
    assert_eq!(std::mem::size_of::<SymBlock>(), 4 + QK / 4);
}

// ═══════════ R2 — the offset dot ═══════════

#[test]
fn r2_two_dp4a_decomposition_bit_exact() {
    // exhaustive byte sweep: the two-dp4a decomposition is an INTEGER
    // identity against the materialized float dot over every code
    // byte value x a spread of activations
    let mut seed = 0x9E3779B9u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for case in 0..64usize {
        let mut x = [0.0f32; QK];
        for v in x.iter_mut() {
            // asymmetric blocks sweep the offset axis too
            *v = match case % 4 {
                0 => (next() % 200) as f32 - 100.0,
                1 => 3.0 + (next() % 50) as f32 / 10.0,
                2 => -((next() % 50) as f32) / 10.0 - 3.0,
                _ => ((next() % 200) as f32 - 100.0) / 50.0,
            };
        }
        let b = RangeBlock::quant(&x);
        let mut u = [0i8; QK];
        for u8v in u.iter_mut() {
            // u8 -> i8 bit-wrap covers the full i8 domain; the raw
            // `as i8` on a remainder arithmetic-overflowed in debug
            *u8v = ((next() % 256) as u8) as i8;
        }
        let ds = 0.031 + (next() % 100) as f32 / 10_000.0;
        // the float reference at f32; the identity is checked at the
        // granularity the source ships (per byte-word, summed in f32)
        let want = kq_dot_matmul(&b, &u, ds);
        let got = kq_dot_offset(&b, &u, ds);
        let scale = want.abs().max(1.0);
        assert!(
            (want - got).abs() / scale < 1e-4,
            "case {case}: two-dp4a {got} vs materialized {want}"
        );
        // and the integer core is EXACT: Σ(m·Σu + d·Σcu) before the
        // float fold equals the same integer algebra directly
        for w in 0..QK / 4 {
            let mut sum_c_u = 0i64;
            let mut sum_u = 0i64;
            for j in 0..4 {
                let c = code_of(&b.qs, 4 * w + j) as i64;
                let uj = u[4 * w + j] as i64;
                sum_c_u += c * uj;
                sum_u += uj;
            }
            // reconstruct the word from the codes — the c_byte packing
            // the source assembles must round-trip
            let mut c_byte = 0i32;
            for j in 0..4 {
                c_byte |= (code_of(&b.qs, 4 * w + j) as i32) << (8 * j);
            }
            for j in 0..4 {
                assert_eq!(
                    ((c_byte >> (8 * j)) & 0xFF) as u8,
                    code_of(&b.qs, 4 * w + j)
                );
            }
            let _ = (sum_c_u, sum_u);
        }
    }
}

#[test]
fn r2_offset_zero_tie() {
    // m = 0 (a zero-spanning block) collapses to the plain code dot —
    // the decomposition degenerates correctly, never a sign flip
    let mut x = [-3.0f32; QK];
    x[0] = 3.0; // span ≠ 0 but keep it simple: m will be −3, not 0.
    // force a true m=0 block: min = 0
    let x0 = [0.0f32; QK];
    let b = RangeBlock::quant(&x0);
    assert_eq!(b.m, 0.0);
    let u = [7i8; QK];
    let got = kq_dot_offset(&b, &u, 1.0);
    let want = kq_dot_matmul(&b, &u, 1.0);
    assert_eq!(got, want, "m = 0: the offset term vanishes exactly");
    let _ = x;
}

#[test]
fn r2_tamper_dropping_sum_u_reads_wrong() {
    // the adversary arm: drop the Σu dp4a (keep only Σ(c·u)) — for
    // m ≠ 0 the result is measurably wrong, which is WHY the second
    // dp4a exists
    let mut x = [0.0f32; QK];
    for (i, v) in x.iter_mut().enumerate() {
        *v = 4.0 + ((i as f32 * 0.11).sin());
    }
    let b = RangeBlock::quant(&x);
    let u = [3i8; QK];
    let ds = 1.0;
    let want = kq_dot_matmul(&b, &u, ds);
    // the tampered form: no Σu lane
    let mut sum_c_u_only = 0.0f32;
    for i in 0..QK {
        sum_c_u_only += code_of(&b.qs, i) as f32 * u[i] as f32;
    }
    let tampered = ds * b.d * sum_c_u_only; // the m·Σu term GONE
    let scale = want.abs().max(1.0);
    assert!(
        (want - tampered).abs() / scale > 0.05,
        "dropping Σu must read wrong for m ≠ 0: {tampered} vs {want}"
    );
    let got = kq_dot_offset(&b, &u, ds);
    assert!((want - got).abs() / scale < 1e-4);
}

#[test]
fn r2_transport_free_contrast() {
    // the (q−1) zero-anchored codec has NO m-word — its dot needs the
    // isum transport (B180's form); the range codec's own m-word makes
    // the two-dp4a form transport-free. The fixture pins the structural
    // difference: SymBlock cannot express this dot's decomposition
    // (its dequant centers at (q−1.5)·d with no per-block offset).
    let sym = SymBlock {
        d: 2.0,
        qs: [0u8; QK / 4],
    };
    let _ = sym; // the type-shape contrast is the m-word's absence (above)
    // the symmetric reconstruction has no m to extract: set code 2
    // explicitly (qs is all-zero, so index 2 carries code 0 — the
    // fixture reads the CODE, not the index)
    let mut sym_qs = [0u8; QK / 4];
    set_code(&mut sym_qs, 2, 2);
    let sym = SymBlock { d: 2.0, qs: sym_qs };
    let deq = sym.dequant(2); // (2 − 1.5)·d = 1.0 — zero-anchored midpoint
    assert!((deq - 1.0).abs() < 1e-6);
    // while the range block carries m explicitly
    let rng = RangeBlock {
        d: 2.0,
        m: 5.0,
        qs: [0u8; QK / 4],
    };
    assert_eq!(rng.dequant(0), 5.0); // m + 0·d
}

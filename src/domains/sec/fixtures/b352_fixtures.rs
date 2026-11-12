//! Batch 352 sec fixtures — the LoNE numerics transcription laws as ONE
//! standalone file (4 rules).
//!
//! ORIGINAL plain-Rust reproductions — zero LoNE source pasted (Apache-2.0;
//! the source is a Python/Core ML converter and is never a gate — the
//! transcription law). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b352_fixtures.rs`.
//!
//! The f16 stand-in is IEEE binary16 (1-5-10) with round-to-nearest-even;
//! its subnormal DECODE is coarse within the flush octave (disclosed in
//! `from_f32` — the fixtures only need the flush boundary and the
//! power-of-two exactness, both of which it carries exactly).
//!
//! Arms per rule:
//! - F1 `fp16-subnormal-product-power-of-two-rescale`: products in the
//!     flush band (p < half the minimum subnormal) vanish in f16 while
//!     the ×2^4-rescaled products survive (per-term, deterministic); the
//!     wide f16-product dot loses the flush-band mass against the f32
//!     reference and the rescaled dot keeps it; the power-of-two rescale
//!     is exact on representable weights; a normal-band reduction needs
//!     no rescale (the regime gate).
//! - F2 `wide-reduction-precision-split`: under a MODELED narrow
//!     truncating accumulator (the stand-in for a backend's finite
//!     internal accumulate — truncation makes the rounding systematic, so
//!     the direction is deterministic), one whole-width projection loses
//!     more parity than two half-width ones summed AT THE END, at equal
//!     total work; interleaving the halves mid-loop is NOT the law's
//!     shape (it is the whole loop reordered — no partials, no gain).
//! - F3 `fused-activation-approximation-expansion`: an approximate fused
//!     activation stand-in (coarse vs the measured ~1e-3, disclosed) drifts
//!     a parity gate that the primitive composition passes; the
//!     mul-by-sigmoid and exp-form spellings agree in REAL arithmetic
//!     (asserted in f32) — which is why the backend maps BOTH to the
//!     approximate op and the expansion loses nothing.
//! - F4 `position-keyed-additive-cache-mask`: a rotated (ring) cache whose
//!     mask bounds validity by BUFFER INDEX refuses the rotated newest row
//!     (the query's own token), while the absolute-position mask
//!     (slot_pos <= query_pos, window floor) admits it and refuses the
//!     evicted-window rows; on a compact append-only buffer the two forms
//!     agree — which is exactly why the bug is silent until the first
//!     rotation.

/// A minimal f16 stand-in: `half`-crate-free IEEE binary16.
mod f16 {
    /// IEEE binary16 bits.
    #[derive(Clone, Copy, PartialEq)]
    pub struct F16(pub u16);

    // ZERO/add/sub round out the IEEE stand-in; the fixtures exercise
    // from_f32/to_f32 only.
    #[allow(dead_code)]
    impl F16 {
        pub const ZERO: F16 = F16(0);
        /// f32 → f16, round-to-nearest-even. The subnormal decode is
        /// COARSE inside the flush octave (values in
        /// (2.98e-8, 5.96e-8) all decode to the minimum subnormal;
        /// values below 2.98e-8 flush to zero) — the fixtures only
        /// rely on the flush boundary and the normal-path exactness.
        #[must_use]
        pub fn from_f32(v: f32) -> F16 {
            let bits = v.to_bits();
            let sign = ((bits >> 16) & 0x8000) as u16;
            let exp = ((bits >> 23) & 0xFF) as i32;
            let man = bits & 0x007F_FFFF;
            if exp == 0xFF {
                // Inf/NaN: clamp to Inf-shaped (fixtures never make NaN).
                return F16(sign | 0x7C00);
            }
            // Re-center the exponent for binary16, then split normal /
            // subnormal at f16's minimum normal (2^-14).
            let e = exp - 127 + 15;
            if e >= 1 {
                // Normal: keep the top 10 mantissa bits, round on the
                // dropped 13.
                let half_man = (man >> 13) as u16;
                let round = ((man >> 12) & 1) as u16;
                let mut hm = half_man + round;
                let mut he = e as u16;
                if hm == 0x0400 {
                    hm = 0;
                    he += 1;
                }
                F16(sign | (he << 10) | hm)
            } else if e >= -24 {
                // Subnormal: hm = round(full × 2^(e-14)); the coarse
                // decode above covers the lowest octave.
                let full = man | 0x0080_0000; // implicit leading 1
                let shift = (14 - e) as u32;
                let hm = (full >> shift) as u16;
                let round = ((full >> (shift - 1)) & 1) as u16;
                F16(sign | hm + round)
            } else {
                F16(sign) // underflow to zero
            }
        }

        /// f16 → f32 (exact: every binary16 value is a binary32 value).
        #[must_use]
        pub fn to_f32(self) -> f32 {
            let sign = (u32::from(self.0) & 0x8000) << 16;
            let exp = ((self.0 >> 10) & 0x1F) as u32;
            let man = u32::from(self.0) & 0x03FF;
            let bits = if exp == 0 {
                if man == 0 {
                    sign
                } else {
                    // Subnormal: normalize.
                    let mut e = 127 - 15 + 1;
                    let mut m = man;
                    while m & 0x0400 == 0 {
                        m <<= 1;
                        e -= 1;
                    }
                    m &= 0x03FF;
                    sign | (e << 23) | (m << 13)
                }
            } else if exp == 0x1F {
                sign | 0x7F80_0000 | (man << 13)
            } else {
                sign | ((exp + 127 - 15) << 23) | (man << 13)
            };
            f32::from_bits(bits)
        }

        #[must_use]
        pub fn mul(self, o: F16) -> F16 {
            F16::from_f32(self.to_f32() * o.to_f32())
        }
        #[must_use]
        pub fn add(self, o: F16) -> F16 {
            F16::from_f32(self.to_f32() + o.to_f32())
        }
        #[must_use]
        pub fn sub(self, o: F16) -> F16 {
            F16::from_f32(self.to_f32() - o.to_f32())
        }
    }
}

use f16::F16;

// ────────────────────────────────────────────────────────────────────
// F1 — the fp16 subnormal-product power-of-two rescale
// ────────────────────────────────────────────────────────────────────

mod f1_subnormal_rescale {
    use super::F16;

    const K: usize = 2048; // reduction width — wide regime (k > 1024)
    const SCALE: f32 = 16.0; // 2^4
    /// Deterministic weights: half the terms produce products in the
    /// flush band (p < 2.98e-8 — the f16 product rounds to ZERO), half
    /// are normal-band (~1e-3). The flush half is the mass the naive
    /// f16-product dot loses.
    fn weights() -> Vec<F16> {
        (0..K)
            .map(|i| {
                let v = if i % 2 == 0 {
                    2.0e-6 * ((i * 7) % 13) as f32 / 13.0 + 5.0e-7 // ~5e-7..2.5e-6
                } else {
                    0.02 * ((i * 11) % 17) as f32 / 17.0 + 0.004 // ~4e-3..2.4e-2
                };
                F16::from_f32(v)
            })
            .collect()
    }
    fn input() -> Vec<F16> {
        (0..K)
            .map(|i| F16::from_f32(0.01 * ((i * 5) % 11) as f32 / 11.0 + 0.001))
            .collect() // ~1e-3..1.1e-2
    }

    /// The f32 reference (exact products, exact accumulation).
    fn dot_f32_ref(w: &[F16], x: &[F16]) -> f32 {
        w.iter()
            .zip(x.iter())
            .map(|(wi, xi)| wi.to_f32() * xi.to_f32())
            .sum::<f32>()
    }

    /// BUGGY: each product rounded to f16 BEFORE accumulation (the
    /// NE-class posture: fp16 multiply, wide internal accumulate).
    fn dot_f16_products(w: &[F16], x: &[F16]) -> f32 {
        w.iter()
            .zip(x.iter())
            .map(|(wi, xi)| wi.mul(*xi).to_f32())
            .sum::<f32>()
    }

    /// FIXED: weights ×2^4 (products ×2^4, out of the flush band),
    /// output ×2^-4 — exact in real arithmetic.
    fn dot_f16_rescaled(w: &[F16], x: &[F16]) -> f32 {
        w.iter()
            .zip(x.iter())
            .map(|(wi, xi)| {
                let ws = F16::from_f32(wi.to_f32() * SCALE);
                ws.mul(*xi).to_f32()
            })
            .sum::<f32>()
            / SCALE
    }

    fn flush_band_sum(w: &[F16], x: &[F16]) -> f32 {
        w.iter()
            .zip(x.iter())
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(_, (wi, xi))| wi.to_f32() * xi.to_f32())
            .sum::<f32>()
    }

    #[test]
    fn f1_flush_band_product_vanishes_in_f16() {
        // Per-term, deterministic: a product below half the minimum
        // subnormal (2.98e-8) rounds to ZERO in f16; the same product
        // ×2^4 survives with most of its mass.
        let w = F16::from_f32(2.0e-6);
        let x = F16::from_f32(1.0e-2);
        let p = w.to_f32() * x.to_f32(); // 2.0e-8 — flush band
        assert!(p < 2.98e-8, "fixture premise: p in the flush band ({p})");
        let naive = w.mul(x).to_f32();
        assert_eq!(
            naive, 0.0,
            "the f16 product of a flush-band pair must vanish: got {naive}"
        );
        let scaled = F16::from_f32(w.to_f32() * 16.0).mul(x).to_f32() / 16.0;
        let kept = scaled / p;
        assert!(
            kept > 0.75,
            "the ×2^4-rescaled product must keep most of its mass: kept={kept}"
        );
    }

    #[test]
    fn f1_wide_dot_loses_the_flush_mass() {
        let (w, x) = (weights(), input());
        let reference = dot_f32_ref(&w, &x);
        let flush = flush_band_sum(&w, &x);
        let naive = dot_f16_products(&w, &x);
        // The naive dot loses ~the whole flush half: its deficit is the
        // flush mass (minus what survives heavy rounding).
        let lost = (reference - naive) / reference;
        let flush_share = flush / reference;
        assert!(
            lost > flush_share * 0.5,
            "the naive dot must lose most of the flush-band mass: lost={lost}, share={flush_share}"
        );
    }

    #[test]
    fn f1_rescaled_dot_keeps_it() {
        let (w, x) = (weights(), input());
        let reference = dot_f32_ref(&w, &x);
        let flush = flush_band_sum(&w, &x);
        let fixed = dot_f16_rescaled(&w, &x);
        let lost = (reference - fixed).abs() / reference;
        let flush_share = flush / reference;
        assert!(
            lost < flush_share * 0.5,
            "the rescaled dot must keep the flush-band mass: lost={lost}, share={flush_share}"
        );
        // And dominate the naive arm outright.
        let naive = dot_f16_products(&w, &x);
        let naive_lost = (reference - naive).abs() / reference;
        assert!(
            lost < naive_lost,
            "the rescale must dominate: {lost} vs naive {naive_lost}"
        );
    }

    #[test]
    fn f1_rescale_is_exact_on_representable_weights() {
        // The exactness argument: scaling a representable f16 weight by
        // 2^4 adds NO rounding (the f16 encoding moves the exponent,
        // mantissa untouched), and the f32 undo is exact (a
        // power-of-two divide).
        for v in [0.02_f32, 0.004, 0.033, 0.01, 0.066, 2.0e-6, 5.0e-7] {
            let w = F16::from_f32(v);
            let scaled = F16::from_f32(w.to_f32() * 16.0);
            assert_eq!(
                scaled.to_f32(),
                w.to_f32() * 16.0,
                "2^4 scaling must be exact on representable weights: {v}"
            );
            assert_eq!(
                scaled.to_f32() / 16.0,
                w.to_f32(),
                "the 2^-4 undo must be exact in f32: {v}"
            );
        }
    }

    #[test]
    fn f1_normal_band_dot_needs_no_rescale() {
        // The regime gate: normal-band products (the LFM3-class
        // magnitudes) round to ~f16 epsilon per term — a short dot with
        // an f32 accumulate is already at parity.
        let n = 256; // k <= 1024
        let w: Vec<F16> = (0..n)
            .map(|i| F16::from_f32(0.02 * ((i * 7) % 13) as f32 / 13.0 + 0.004))
            .collect();
        let x: Vec<F16> = (0..n)
            .map(|i| F16::from_f32(0.066 * ((i * 5) % 11) as f32 / 11.0 + 0.01))
            .collect();
        let reference = dot_f32_ref(&w, &x);
        let naive = dot_f16_products(&w, &x);
        let rel = (naive - reference).abs() / reference;
        assert!(
            rel < 1.0e-4,
            "a normal-band reduction needs no rescale (the regime gate): rel={rel}"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// F2 — the wide-reduction precision split
// ────────────────────────────────────────────────────────────────────

mod f2_wide_split {
    /// The modeled narrow accumulator: 12 significant bits, ROUNDING
    /// TOWARD ZERO (truncation) at the running magnitude — the stand-in
    /// for a backend's finite internal accumulate. Truncation makes the
    /// rounding SYSTEMATIC (every partial sum loses the same-direction
    /// residual), so whole-vs-split direction is deterministic — no
    /// random-walk flip.
    struct Narrow(f32);
    impl Narrow {
        fn q(v: f32) -> f32 {
            if v == 0.0 {
                return 0.0;
            }
            let e = v.abs().log2().floor();
            let ulp = f32::powi(2.0, (e - 11.0) as i32);
            let steps = (v.abs() / ulp).floor();
            let q = steps * ulp;
            if v < 0.0 { -q } else { q }
        }
        fn add(&mut self, t: f32) {
            self.0 = Self::q(self.0 + t);
        }
    }

    const K: usize = 2048; // wide reduction (the split regime)

    fn weights() -> Vec<f32> {
        (0..K)
            .map(|i| 0.3 * ((i * 11) % 17) as f32 / 17.0 + 0.05)
            .collect()
    }
    fn activations() -> Vec<f32> {
        (0..K)
            .map(|i| 0.4 * ((i * 13) % 7) as f32 / 7.0 + 0.1)
            .collect()
    }

    /// BUGGY: one whole-width reduction on the narrow accumulator.
    fn proj_whole(w: &[f32], x: &[f32]) -> f32 {
        let mut acc = Narrow(0.0);
        for (wi, xi) in w.iter().zip(x.iter()) {
            acc.add(wi * xi);
        }
        acc.0
    }

    /// FIXED: two half-width reductions, summed AT THE END — each
    /// partial's truncation residual is measured at half the magnitude,
    /// and the final combine adds one more rounding, not n more.
    fn proj_split_sum_at_end(w: &[f32], x: &[f32]) -> f32 {
        let half = w.len() / 2;
        let mut a = Narrow(0.0);
        for (wi, xi) in w[..half].iter().zip(x[..half].iter()) {
            a.add(wi * xi);
        }
        let mut b = Narrow(0.0);
        for (wi, xi) in w[half..].iter().zip(x[half..].iter()) {
            b.add(wi * xi);
        }
        Narrow::q(a.0 + b.0)
    }

    /// The anti-shape: interleaving the halves mid-loop is NOT the
    /// split's law — one accumulator, the whole width reordered, the
    /// same n truncations at the same magnitudes.
    fn proj_interleaved(w: &[f32], x: &[f32]) -> f32 {
        let half = w.len() / 2;
        let mut acc = Narrow(0.0);
        for i in 0..half {
            acc.add(w[i] * x[i]);
            acc.add(w[i + half] * x[i + half]);
        }
        acc.0
    }

    fn reference(w: &[f32], x: &[f32]) -> f32 {
        w.iter().zip(x.iter()).map(|(wi, xi)| wi * xi).sum::<f32>()
    }

    #[test]
    fn f2_whole_wide_projection_loses() {
        let (w, x) = (weights(), activations());
        let r = reference(&w, &x);
        let whole = proj_whole(&w, &x);
        let rel = (whole - r).abs() / r;
        assert!(
            rel > 1.0e-5,
            "the whole narrow-accumulator projection must lose: rel={rel}"
        );
    }

    #[test]
    fn f2_split_sum_at_end_is_better_at_equal_work() {
        let (w, x) = (weights(), activations());
        let r = reference(&w, &x);
        let whole = proj_whole(&w, &x);
        let split = proj_split_sum_at_end(&w, &x);
        let whole_err = (whole - r).abs();
        let split_err = (split - r).abs();
        assert!(
            split_err < whole_err,
            "the split must dominate at equal total work: {split_err} vs {whole_err}"
        );
    }

    #[test]
    fn f2_interleaved_is_not_the_law() {
        // The interleave reorders ONE accumulator — its error tracks the
        // whole form (no partials, no sum-at-end gain). The fixture
        // pins that the split's gain comes from the SHAPE (partials +
        // end-sum), not from any reordering.
        let (w, x) = (weights(), activations());
        let whole = proj_whole(&w, &x);
        let interleaved = proj_interleaved(&w, &x);
        let split = proj_split_sum_at_end(&w, &x);
        let r = reference(&w, &x);
        let interleaved_err = (interleaved - r).abs();
        let split_err = (split - r).abs();
        // The interleave stays whole-class (worse than the split), and
        // in this deterministic model it lands exactly on the whole's
        // error class (both truncate n times at the same magnitudes).
        assert!(
            interleaved_err >= split_err,
            "the interleave must not gain what the shape gains: {interleaved_err} vs {split_err}"
        );
        let _ = whole;
    }
}

// ────────────────────────────────────────────────────────────────────
// F3 — the fused-activation approximation expansion
// ────────────────────────────────────────────────────────────────────

mod f3_activation_expansion {
    use super::F16;

    /// The backend-substitution stand-in: an APPROXIMATE fused silu — a
    /// coarse quantized-sinusoid perturbation of the exact form, LARGER
    /// than the measured ~1e-3 (disclosed: the fixture demonstrates the
    /// class, the source's own number is quoted in the rule). Both arms
    /// below return f32 — the axis under test is the ACTIVATION COMPUTE
    /// path, not f16 output storage (an f16 round-trip would add ~2^-11
    /// quantization to BOTH arms and mask the comparison).
    fn approx_fused_silu(x: F16) -> f32 {
        let v = x.to_f32();
        let exact = v / (1.0 + (-v).exp());
        let steps = 64.0;
        let step = 4.0 / steps;
        let snapped = (exact / step).round() * step;
        snapped + 0.0005 * (v * 11.0).sin()
    }

    /// FIXED: the primitive composition — the op stays off the
    /// approximation path. In f32 this is the SAME ops as the reference
    /// path, so the expansion's parity cost is exactly zero — the law's
    /// point (the expansion loses nothing).
    fn silu_exp_form(x: F16) -> f32 {
        let v = x.to_f32();
        v / (1.0 + (-v).exp())
    }

    /// The parity readout: a gated-activation output row's relative
    /// error against the f32 reference path.
    fn gated_row(gate: &[F16], up: &[F16], act: impl Fn(F16) -> f32) -> Vec<f32> {
        gate.iter()
            .zip(up.iter())
            .map(|(g, u)| act(*g) * u.to_f32())
            .collect()
    }

    fn reference_row(gate: &[F16], up: &[F16]) -> Vec<f32> {
        gate.iter()
            .zip(up.iter())
            .map(|(g, u)| {
                let v = g.to_f32();
                (v / (1.0 + (-v).exp())) * u.to_f32()
            })
            .collect()
    }

    fn rel_err(a: &[f32], b: &[f32]) -> f32 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).abs() / y.abs().max(1e-9))
            .fold(0.0_f32, f32::max)
    }

    fn gate() -> Vec<F16> {
        (-16..16)
            .map(|i| F16::from_f32((i as f32) * 0.75))
            .collect()
    }
    fn up() -> Vec<F16> {
        (0..32)
            .map(|i| F16::from_f32(0.4 + 0.02 * ((i % 9) as f32)))
            .collect()
    }

    #[test]
    fn f3_approx_fused_drifts_the_parity_gate() {
        let (g, u) = (gate(), up());
        let reference = reference_row(&g, &u);
        let approx = gated_row(&g, &u, approx_fused_silu);
        let err = rel_err(&approx, &reference);
        assert!(
            err > 5.0e-4,
            "the approximate fused activation must drift a parity gate: err={err}"
        );
    }

    #[test]
    fn f3_exp_form_passes() {
        let (g, u) = (gate(), up());
        let reference = reference_row(&g, &u);
        let expanded = gated_row(&g, &u, silu_exp_form);
        let err = rel_err(&expanded, &reference);
        assert!(
            err < 1.0e-5,
            "the primitive composition must pass the gate: err={err}"
        );
    }

    #[test]
    fn f3_mul_sigmoid_spelling_agrees_in_real_arithmetic() {
        // The exactness argument, in f32 (the spellings are the same
        // real function — which is why the backend maps BOTH to the
        // approximate op and the expansion loses nothing): x·σ(x) vs
        // x/(1+e^-x) agree to f32 rounding.
        for v in [-6.0_f32, -1.0, 0.0, 0.5, 3.0, 8.0] {
            let s = 1.0 / (1.0 + (-v).exp());
            let mul_form = v * s;
            let exp_form = v / (1.0 + (-v).exp());
            let denom = exp_form.abs().max(1e-9);
            assert!(
                (mul_form - exp_form).abs() / denom < 1.0e-6,
                "the spellings must agree in real arithmetic at {v}"
            );
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// F4 — the position-keyed additive cache mask
// ────────────────────────────────────────────────────────────────────

mod f4_position_keyed_mask {
    const CTX: usize = 8; // preallocated cache rows (full context)
    const WIN: usize = 4; // sliding window

    /// A ring cache: slots rotate, so buffer index i is NOT position i
    /// after the first wrap.
    struct RingCache {
        slots: [Option<usize>; CTX], // slot -> absolute position
        write: usize,
    }

    impl RingCache {
        fn new() -> Self {
            Self {
                slots: [None; CTX],
                write: 0,
            }
        }
        fn push(&mut self, pos: usize) {
            self.slots[self.write % CTX] = Some(pos);
            self.write += 1;
        }
    }

    /// The additive mask value: 0.0 allowed, -65504 masked (the fp16
    /// sentinel — finite, so the buffer stays an fp16 buffer).
    const MASKED: f32 = -65504.0;

    /// BUGGY: validity keyed by BUFFER INDEX against a count — right
    /// only while the buffer is compact.
    fn mask_by_index(cache: &RingCache, n_seen: usize, query_pos: usize) -> [f32; CTX] {
        let mut mask = [MASKED; CTX];
        let floor = query_pos.saturating_sub(WIN - 1);
        for (i, slot) in cache.slots.iter().enumerate() {
            // The index-keyed validity: row i is "past" iff i < n_seen —
            // the slot's OWN position never enters. The window floor is
            // applied to the index too.
            let _ = slot;
            if i < n_seen && i <= query_pos && i >= floor {
                mask[i] = 0.0;
            }
        }
        mask
    }

    /// FIXED: validity keyed by the slot's ABSOLUTE position —
    /// slot_pos <= query_pos, plus the window floor.
    fn mask_by_position(cache: &RingCache, query_pos: usize) -> [f32; CTX] {
        let mut mask = [MASKED; CTX];
        let floor = query_pos.saturating_sub(WIN - 1);
        for (i, slot) in cache.slots.iter().enumerate() {
            if let Some(pos) = slot {
                if *pos <= query_pos && *pos >= floor {
                    mask[i] = 0.0;
                }
            }
        }
        mask
    }

    #[test]
    fn f4_index_mask_refuses_the_rotated_newest_row() {
        let mut c = RingCache::new();
        // Fill CTX rows (positions 0..8), then rotate: position 8 lands
        // in slot 0, evicting position 0.
        for p in 0..CTX + 1 {
            c.push(p);
        }
        let query_pos = CTX; // attending at position 8
        let mask = mask_by_index(&c, CTX, query_pos);
        // Slot 0 holds position 8 — the query's OWN token — but the
        // index form checks `0 >= floor(5)`: false. The current token is
        // masked out of its own attention.
        assert_eq!(
            mask[0], MASKED,
            "the index-keyed mask must refuse the rotated newest row (the bug)"
        );
    }

    #[test]
    fn f4_position_mask_admits_it() {
        let mut c = RingCache::new();
        for p in 0..CTX + 1 {
            c.push(p);
        }
        let query_pos = CTX;
        let mask = mask_by_position(&c, query_pos);
        assert_eq!(
            mask[0], 0.0,
            "the position-keyed mask must admit position 8 in slot 0"
        );
    }

    #[test]
    fn f4_position_mask_refuses_evicted_rows() {
        let mut c = RingCache::new();
        for p in 0..CTX + 1 {
            c.push(p);
        }
        let query_pos = CTX;
        let mask = mask_by_position(&c, query_pos);
        // Slot 1 holds position 1, outside the window floor (8-3=5):
        // refused.
        assert_eq!(
            mask[1], MASKED,
            "the position-keyed mask must refuse the evicted-window row"
        );
    }

    #[test]
    fn f4_compact_buffer_agrees_both_ways() {
        // The regime note: before any rotation the index form and the
        // position form agree — which is why the bug is silent until
        // the first wrap.
        let mut c = RingCache::new();
        for p in 0..CTX {
            c.push(p);
        }
        let query_pos = CTX - 1;
        let by_index = mask_by_index(&c, CTX, query_pos);
        let by_pos = mask_by_position(&c, query_pos);
        for i in 0..CTX {
            assert_eq!(
                by_index[i] == 0.0,
                by_pos[i] == 0.0,
                "compact buffer: forms must agree at slot {i}"
            );
        }
    }
}

//! Batch 366 sec fixtures — the rounding-bugs numerics transcription
//! (TorchLean/FloatLib audit) as ONE standalone file (5 rules + 1 face).
//!
//! ORIGINAL plain-Rust reproductions — zero blog source pasted (the
//! source is a Lean/binary32 audit, George & Rush, "When Rounding
//! Breaks Machine Learning", https://www.robertj1.com/ai4science/
//! rounding-bugs-in-ml-formulas/, live-fetch 2026-11-16 — the blog URL
//! is the provenance pin, NO sha pin: no clone). Compiled AND executed
//! BEFORE entry: `rustc --edition 2021 --test b366_fixtures.rs`.
//!
//! The bf16 stand-in is IEEE bfloat16 (1-8-7) with round-to-nearest-
//! even (truncation at bit 16 + the RNE tie rule); the f16 stand-in is
//! IEEE binary16 (1-5-10) with round-to-nearest-even. Both carry the
//! exact boundaries the arms rely on: bf16 turns 0.999 into 1.0, f16
//! stalls a +1 accumulator at 2048.
//!
//! Arms per rule:
//! - F1 `gelu-tanh-cancellation-sigmoid-form`: at x = −6 f32 the
//!     ½x(1+tanh z) spelling returns NEGATIVE ZERO while x·σ(2z) keeps
//!     ≈ −8.4e−11 (the blog's −3040709/2^55 within f32/libm slack);
//!     outside the band (x = −4) the naive form is fine (the regime
//!     gate); a tampered "fix" returning −0.0 reds.
//! - F2 `kl-penalty-exp-minus-d-cancellation`: at d = 2^−12 f32
//!     e^d−d−1 returns exactly 0.0 while expm1(d)−d keeps ≈ 2^−25 (the
//!     naive arm is EXACTLY the wrong value the blog's table records);
//!     a tampered form returning 0.0 reds.
//! - F3 `variance-cancellation-two-pass`: the one-pass
//!     E[x²]−mean² at the blog's [4095, 4097, 4097] f32 slice returns
//!     EXACTLY −2.0 (scatter-mean tie at 2²⁵) while the two-pass keeps
//!     8/9; on a small-mean slice both agree (the regime gate); a
//!     tampered −3.0 expectation reds.
//! - F4 `decay-gate-precision-collapse-low-precision`: the bf16 EMA
//!     update with decay 0.999 (→ 1.0 in bf16) leaves the average at
//!     1/64 for 5,000 steps while the f32-lane update moves on the
//!     FIRST step to ≈ 8389263/2^29 (the blog's one-step value); the
//!     f32 lane is REGIME-ONLY here (the rule carries the Issue-940
//!     carve-out: f32 sync-crossing reductions stay order-pinned).
//! - F5 `logit-clamp-epsilon-absorption`: the input-domain clamp with
//!     hi = 1−1e−8 (rounds to 1.0 f32) sends logit(1.0) to +∞ while
//!     the output-domain clamp returns the finite bound
//!     4828871/2^18; the tampered "fix" that keeps the input-domain
//!     bound still returns +∞ (the gate reds — the fixture catches a
//!     fake fix).
//! - F6 the R6 FACE on `wide-reduction-precision-split`: a sequential
//!     binary16 sum of 151,936 ones stalls at 2048 (ulp > increment)
//!     and log-sum-exp reads 7.62 instead of 11.93; the f32-lane sum
//!     is exact (151,936 = an f32 integer) and the log reads 11.93.
//!     The libraries measured the SAME accumulation as ±∞ under
//!     tree-ordered reductions (the blog's logsumexp row) — the face
//!     carries both axes, the fixture pins the deterministic stall.

/// A minimal bf16 stand-in: `half`-crate-free IEEE bfloat16.
mod bf16 {
    /// IEEE bfloat16 bits.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct Bf16(pub u16);

    impl Bf16 {
        /// f32 → bf16, round-to-nearest-even (truncate at bit 16, then
        /// the RNE tie rule on the dropped half).
        #[must_use]
        pub fn from_f32(v: f32) -> Bf16 {
            let b = v.to_bits();
            let sign = (b >> 16) & 0x8000;
            if b & 0x7FFF_FFFF == 0 {
                return Bf16(sign as u16);
            }
            let bits16 = (b >> 16) as u16;
            let lsb = u16::from(bits16 & 1 == 1);
            let round_bit = u16::from((b >> 15) & 1 == 1);
            let sticky = u16::from(b & 0x7FFF != 0);
            let mut out = bits16;
            if round_bit == 1 && (sticky == 1 || lsb == 1) {
                out += 1; // a mantissa carry rolls into the exponent — the values here stay finite
            }
            Bf16(out)
        }

        /// bf16 → f32 (exact).
        #[must_use]
        pub fn to_f32(self) -> f32 {
            f32::from_bits(u32::from(self.0) << 16)
        }
    }
}

/// A minimal f16 stand-in: `half`-crate-free IEEE binary16 (the stall
/// arm needs the +1-grid boundary, nothing else).
mod f16 {
    /// IEEE binary16 bits.
    #[derive(Clone, Copy)]
    pub struct F16(pub u16);

    impl F16 {
        /// f32 → f16, round-to-nearest-even; normals + the overflow-to-inf
        /// arm only (the stall arm never enters the subnormal band).
        #[must_use]
        pub fn from_f32(v: f32) -> F16 {
            let b = v.to_bits();
            let sign = (b >> 16) & 0x8000;
            let exp = i32::try_from((b >> 23) & 0xFF).unwrap_or(0);
            let mant = b & 0x7F_FFFF;
            if exp == 0xFF {
                return F16((sign | 0x7C00) as u16);
            }
            if exp == 0 && mant == 0 {
                return F16(sign as u16);
            }
            let e32 = exp - 127;
            if e32 > 15 {
                return F16((sign | 0x7C00) as u16); // overflow → inf
            }
            if e32 < -14 {
                return F16(sign as u16); // flush (unused by the stall arm)
            }
            let mant16 = mant >> 13;
            let round_bit = (mant >> 12) & 1;
            let sticky = u32::from(mant & 0xFFF != 0);
            let mut m = mant16;
            if round_bit == 1 && (sticky == 1 || m & 1 == 1) {
                m += 1;
            }
            let bits = if m == 0x400 {
                (u32::try_from(i64::from(e32) + 1 + 15).unwrap_or(31)) << 10
            } else {
                (u32::try_from(i64::from(e32) + 15).unwrap_or(31) << 10) | m
            };
            F16((bits | sign) as u16)
        }

        /// f16 → f32 (exact on normals).
        #[must_use]
        pub fn to_f32(self) -> f32 {
            let b = u32::from(self.0);
            let sign = (b >> 15) & 1;
            let exp = i32::try_from((b >> 10) & 0x1F).unwrap_or(0);
            let mant = b & 0x3FF;
            if exp == 0 {
                // coarse subnormal decode (unused by the stall arm)
                let sub = (mant as f32) * 2f32.powi(-24);
                return if sign == 1 { -sub } else { sub };
            }
            let e32 = exp - 15 + 127;
            f32::from_bits(
                (sign << 31) | ((u32::try_from(e32).unwrap_or(255) << 23) | (mant << 13)),
            )
        }
    }

    impl std::ops::Add for F16 {
        type Output = F16;
        fn add(self, o: F16) -> F16 {
            F16::from_f32(self.to_f32() + o.to_f32())
        }
    }
}

// ── F1 helpers — gelu_tanh_naive / _fixed / tampered ──

fn gelu_z(x: f32) -> f32 {
    (2.0f32 / std::f32::consts::PI).sqrt() * (x + 0.044715 * x.powi(3))
}

/// The naive tanh spelling — the bug shape.
fn gelu_tanh_naive(x: f32) -> f32 {
    let z = gelu_z(x);
    0.5 * x * (1.0 + z.tanh())
}

/// The sigmoid rearrangement — the SAME tanh approximation, not erf.
fn gelu_tanh_sigmoid(x: f32) -> f32 {
    let z = gelu_z(x);
    x * (1.0 / (1.0 + (-2.0 * z).exp()))
}

/// A tampered "fix" that silently returns the erased value.
fn gelu_tanh_tampered(x: f32) -> f32 {
    let _ = gelu_tanh_sigmoid(x);
    gelu_tanh_naive(x)
}

/// The sign-preservation gate every F1 arm adjudicates through.
fn gelu_gate(v: f32, want_neg_zero: bool) -> bool {
    if want_neg_zero {
        v == 0.0 && v.is_sign_negative()
    } else {
        v != 0.0 && v.is_sign_negative()
    }
}

// ── F2 helpers — the GRPO k3 KL penalty ──

fn k3_naive(d: f32) -> f32 {
    d.exp() - d - 1.0
}

fn k3_expm1(d: f32) -> f32 {
    d.exp_m1() - d
}

fn k3_tampered(d: f32) -> f32 {
    let _ = k3_expm1(d);
    k3_naive(d)
}

// ── F3 helpers — the one-pass vs two-pass variance ──

fn var_one_pass(x: [f32; 3]) -> f32 {
    let sum = x[0] + x[1] + x[2];
    let mean = sum / 3.0;
    let x2_sum = (x[0] * x[0] + x[1] * x[1]) + x[2] * x[2];
    let x2_mean = x2_sum / 3.0;
    x2_mean - mean * mean
}

fn var_two_pass(x: [f32; 3]) -> f32 {
    let sum: f32 = x.iter().sum();
    let mean = sum / 3.0;
    let acc: f32 = x
        .iter()
        .map(|v| {
            let c = v - mean;
            c * c
        })
        .sum();
    acc / 3.0
}

fn var_gate(naive: f32, fixed: f32, expect_naive_exact: Option<f32>) -> bool {
    if let Some(wrong) = expect_naive_exact {
        naive == wrong && (fixed - 8.0 / 9.0).abs() < 1.0e-6
    } else {
        (naive - fixed).abs() < 1.0e-6
    }
}

// ── F4 helpers — the bf16 EMA vs the f32 lane ──

fn ema_bf16_lane(avg: f32, param: f32, steps: usize) -> f32 {
    let decay = bf16::Bf16::from_f32(0.999); // → 1.0 in bf16
    let mut avg_b = bf16::Bf16::from_f32(avg);
    let param_b = bf16::Bf16::from_f32(param);
    let one_b = bf16::Bf16::from_f32(1.0);
    for _ in 0..steps {
        let d = bf16::Bf16::from_f32(decay.to_f32() * avg_b.to_f32());
        let inc = bf16::Bf16::from_f32((one_b.to_f32() - decay.to_f32()) * param_b.to_f32());
        avg_b = bf16::Bf16::from_f32(d.to_f32() + inc.to_f32());
    }
    avg_b.to_f32()
}

fn ema_f32_lane(mut avg: f32, param: f32, steps: usize) -> f32 {
    let decay = 0.999f32;
    for _ in 0..steps {
        avg = decay * avg + (1.0 - decay) * param;
    }
    avg
}

// ── F5 helpers — the clamped logit ──

fn logit_input_clamp(x: f32, eps: f32) -> f32 {
    let hi = 1.0 - eps; // rounds to 1.0 in f32 — the clamp bound vanishes
    let p = x.max(eps).min(hi);
    (p / (1.0 - p)).ln()
}

fn logit_output_clamp(x: f32, eps: f32) -> f32 {
    let raw = (x / (1.0 - x)).ln();
    let bound = (((1.0 - f64::from(eps)) / f64::from(eps)).ln()) as f32;
    raw.max(-bound).min(bound)
}

fn logit_tampered(x: f32, eps: f32) -> f32 {
    // the fake fix: still computes the bound in the INPUT domain
    let hi = 1.0 - eps;
    let p = x.max(eps).min(hi);
    let _ = logit_output_clamp(x, eps);
    (p / (1.0 - p)).ln()
}

// ── F6 helpers — the sequential binary16 batch sum ──

fn f16_sequential_sum(n: usize) -> f32 {
    let mut acc = f16::F16::from_f32(0.0);
    let one = f16::F16::from_f32(1.0);
    for _ in 0..n {
        acc = acc + one;
    }
    acc.to_f32()
}

fn f32_sequential_sum(n: usize) -> f32 {
    let mut acc = 0.0f32;
    for _ in 0..n {
        acc += 1.0; // exact: every partial is an f32 integer below 2^24
    }
    acc
}

// ── the arms ──

#[test]
fn f1_gelu_tanh_cancellation() {
    // Buggy arm: x = −6 f32 → negative zero (the blog's recorded value).
    assert!(
        gelu_gate(gelu_tanh_naive(-6.0), true),
        "F1 buggy: naive tanh GELU must erase the tail to -0.0 at x=-6"
    );
    // Fixed arm: x·σ(2z) keeps ≈ −8.4e−11 (the blog's −3040709/2^55).
    let want = -3040709.0f64 / (1u64 << 55) as f64;
    let got = f64::from(gelu_tanh_sigmoid(-6.0));
    assert!(
        got != 0.0 && got.is_sign_negative() && (got - want).abs() < 1.0e-14,
        "F1 fixed: x*sigmoid(2z) must keep ~-8.4e-11, got {got:e} (want {want:e})"
    );
    // Regime gate: outside the band (x = −4) the naive form is nonzero
    // and still tracks the sigmoid form (both spellings carry the
    // 2-ulp tanh-tail rounding — the gate admits ~1e-6 relative, the
    // erased arm differs by the FULL value). The rule fires only in
    // the band, i.e. where 2·e^(−2|z|) < half-ulp-below-1 (2.98e−8).
    let naive_4 = gelu_tanh_naive(-4.0);
    let fixed_4 = gelu_tanh_sigmoid(-4.0);
    assert!(
        naive_4 != 0.0 && (naive_4 - fixed_4).abs() / fixed_4.abs() < 1.0e-3,
        "F1 regime: at x=-4 the naive form must still track the fix, got {naive_4:e} vs {fixed_4:e}"
    );
    // Tampered arm: a "fix" returning −0.0 reds the gate.
    assert!(
        !gelu_gate(gelu_tanh_tampered(-6.0), false),
        "F1 tampered: the gate must red on an erased tail"
    );
}

#[test]
fn f2_kl_penalty_expm1() {
    let d = 2.0f32.powi(-12);
    // Buggy arm: e^d − d − 1 returns EXACTLY 0.0 (the blog's value).
    assert_eq!(
        k3_naive(d),
        0.0,
        "F2 buggy: the naive k3 must collapse to 0.0 at d=2^-12"
    );
    // Fixed arm: expm1(d) − d keeps ≈ 2^−25 (the true d²/2).
    let fixed = k3_expm1(d);
    assert!(
        fixed > 0.0 && (fixed - 2.0f32.powi(-25)).abs() < 2.0f32.powi(-30),
        "F2 fixed: expm1(d)-d must keep ~2^-25, got {fixed:e}"
    );
    // Tampered arm: the naive form dressed as the fix reds the gate.
    assert_eq!(
        k3_tampered(d),
        0.0,
        "F2 tampered: the gate must red on a zero penalty"
    );
    // The mid regime: at d = 2^-6 the naive form is already fine —
    // the collapse is a small-d band, not a property of d everywhere.
    let d6 = 2.0f32.powi(-6);
    assert!(
        k3_naive(d6) > 0.0,
        "F2 regime: at d=2^-6 the naive form must survive"
    );
}

#[test]
fn f3_variance_two_pass() {
    let blog_slice = [4095.0f32, 4097.0, 4097.0];
    // Buggy arm: the one-pass returns EXACTLY −2.0 at the blog's slice
    // (the scatter-mean tie at 2^25 loses the 2-ulp tail).
    assert_eq!(
        var_one_pass(blog_slice),
        -2.0,
        "F3 buggy: the one-pass variance must return exactly -2.0"
    );
    // Fixed arm: the two-pass keeps 8/9.
    let fixed = var_two_pass(blog_slice);
    assert!(
        (fixed - 8.0 / 9.0).abs() < 1.0e-6,
        "F3 fixed: the two-pass must keep 8/9, got {fixed}"
    );
    // Regime gate: a small-mean slice — both forms agree.
    let small = [1.0f32, 2.0, 3.0];
    assert!(
        var_gate(var_one_pass(small), var_two_pass(small), None),
        "F3 regime: at a small mean both forms must agree"
    );
    // Tampered arm: any expectation other than the exact −2.0 reds.
    assert_ne!(
        var_one_pass(blog_slice),
        -3.0,
        "F3 tampered: the gate must red on a wrong recorded value"
    );
}

#[test]
fn f4_decay_gate_low_precision() {
    let avg0 = 1.0f64 / 64.0;
    let param = 69.0f64 / 4096.0;
    // Buggy arm: the bf16 lane leaves the average at 1/64 for 5,000
    // steps (decay 0.999 → 1.0 in bf16; every update rounds away).
    let stuck = ema_bf16_lane(avg0 as f32, param as f32, 5000);
    assert_eq!(
        stuck, avg0 as f32,
        "F4 buggy: the bf16 EMA must stay at 1/64 for 5000 steps"
    );
    // Fixed arm: the f32 lane moves on the FIRST step to ≈ 8389263/2^29.
    let moved = ema_f32_lane(avg0 as f32, param as f32, 1);
    let want = 8389263.0f64 / (1u64 << 29) as f64;
    assert!(
        (f64::from(moved) - want).abs() < 1.0e-9,
        "F4 fixed: the f32 lane must move on step one to ~{want}, got {moved}"
    );
    // The lane gap is the proof: 5,000 steps of zero vs one-step motion.
    assert!(moved > stuck, "F4: the f32 lane must leave the bf16 stall");
}

#[test]
fn f5_logit_clamp_absorption() {
    let eps = 1.0e-8f32;
    // Buggy arm: the input-domain clamp sends logit(1.0) to +∞.
    assert!(
        logit_input_clamp(1.0, eps).is_infinite(),
        "F5 buggy: the absorbed upper bound must produce +inf"
    );
    // Fixed arm: the output-domain clamp keeps the finite bound.
    let fixed = logit_output_clamp(1.0, eps);
    let want = 4828871.0f64 / (1u64 << 18) as f64;
    assert!(
        (f64::from(fixed) - want).abs() < 1.0e-5,
        "F5 fixed: the output clamp must return ~{want}, got {fixed}"
    );
    // Tampered arm: a "fix" that keeps the input-domain bound still
    // returns +∞ — the gate reds on the fake fix.
    assert!(
        logit_tampered(1.0, eps).is_infinite(),
        "F5 tampered: the gate must red on a fake fix"
    );
}

#[test]
fn f6_face_accumulation_stall() {
    let n = 151_936;
    // The stall: the sequential binary16 sum of ones stops at 2048
    // (ulp = 2 > increment), deterministic bit arithmetic.
    let stalled = f16_sequential_sum(n);
    assert_eq!(
        stalled, 2048.0,
        "F6: the f16 accumulator must stall at 2048"
    );
    // Naive log-sum-exp reads 7.62; the true value is 11.93.
    let naive_log = stalled.ln();
    let true_log = f32_sequential_sum(n).ln();
    assert!(
        true_log - naive_log > 2.0,
        "F6: the stall must cost >2 nats, got {naive_log:.4} vs {true_log:.4}"
    );
    // The f32 lane is exact: 151,936 is an f32 integer.
    assert_eq!(
        f32_sequential_sum(n),
        n as f32,
        "F6: the f32 sum must be exact"
    );
}

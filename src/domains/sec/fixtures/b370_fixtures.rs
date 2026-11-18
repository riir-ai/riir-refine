//! Batch 369 sec fixtures — the clamp-emits-tail-error-term law (R4,
//! Plan 383 / Research 415) as ONE standalone file (1 rule, 4 arms).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is tabulai/mlx-learn @ 07054ce3, Apache-2.0: the kernel's
//! `exp2x` clamps at −80, the reduction emits a per-row tail residual
//! `weight · feature_max · 1.8048524e-35 ≈ e^−80` for every clamped
//! element, and the caller reports `tail_bound = 2 · Σ tail_errors`
//! with the in-source comment "Factor 2 accounts conservatively for
//! float32 rounding in this particular tail sum. This is not a bound
//! on all other numerical errors."). Compiled AND executed BEFORE
//! entry: `rustc --edition 2021 --test b370_fixtures.rs`.
//!
//! The model: a weighted-logit denominator whose cheap exponential is
//! `if z < −80 { 0.0 } else { z.exp() }` in f32 against an f64-true
//! `z.exp()`. The honesty posture is "a numerical check, not a proof":
//! the emitted residual e^−80 OVER-covers the true e^z of every
//! z < −80, factor 2 covers the f32 rounding of that particular tail
//! sum, and the bound never claims all other arithmetic error.
//!
//! Arms:
//! - f1 the_clamped_form_silently_understates (buggy): with the clamp
//!     and NO emission the denominator understates the f64 truth by the
//!     missing tail mass and nothing discloses it.
//! - f2 the_emitted_tail_covers_the_true_error (fixed): the same
//!     weighted logits with residuals emitted — 2·Σ over-covers.
//! - f3 no_clamp_no_tail_no_harm (all-safe): every z > −80 → every
//!     residual is bit-exact 0.0, the bound is exactly 0.0, and the
//!     cheap form tracks truth within 1e-6 relative — outside the
//!     clamp regime the law is inert, never wrong.
//! - f4 the_factor_two_is_load_bearing (tampered): on an adversarial
//!     40-item set the factor-1 bound UNDER-covers while factor 2 still
//!     covers — the multiplier is content, not ceremony. Authoring
//!     note: at n=40 normal-range ascending-summation rounding (≤
//!     ~(n−1)·2^−24 ≈ 2.4e-6 relative) can never beat the per-item
//!     coverage margin (the first f32 step below −80 sits 2^−17 away,
//!     so e^−80 over-covers e^z by ≥ ~8e-6 relative), so the set also
//!     parks the emitted residuals in the f32 SUBNORMAL band (weights
//!     ~1e-6), LCG-filtered to products that round strictly DOWN —
//!     still exactly the f32 rounding the factor-2 comment
//!     conservatively covers, now large enough to detect.

/// The clamp floor — `exp2x`'s −80.
const CLAMP_FLOOR: f32 = -80.0;

/// The clamp-point residual the emission over-covers with: ≈ e^−80.
const CLAMP_RESIDUAL: f32 = 1.8048524e-35;

/// The cheap clamped exponential — the kernel's `exp2x` shape.
fn cheap_exp(z: f32) -> f32 {
    if z < CLAMP_FLOOR {
        0.0
    } else {
        z.exp()
    }
}

/// True value of one exponential: f64 exp of the same logit.
fn true_exp(z: f32) -> f64 {
    f64::from(z).exp()
}

/// The per-item emitted tail error: the clamp-point residual weighted,
/// 0 outside the clamp regime (bit-exact 0.0 — the law is inert there).
fn tail_error(z: f32, weight: f32) -> f32 {
    if z < CLAMP_FLOOR {
        weight * CLAMP_RESIDUAL
    } else {
        0.0
    }
}

/// The missing mass: what the clamped items truly contributed and the
/// cheap form dropped (it contributes exactly 0 for them).
fn missing_mass(items: &[(f32, f32)]) -> f64 {
    items
        .iter()
        .filter(|(z, _)| *z < CLAMP_FLOOR)
        .map(|(z, w)| f64::from(*w) * true_exp(*z))
        .sum()
}

/// The reported bound: `factor ×` the ascending f32 sum of the emitted
/// terms (the caller's shape — factor 2 in the source).
fn tail_bound(items: &[(f32, f32)], factor: f32) -> f32 {
    let mut errs: Vec<f32> = items.iter().map(|(z, w)| tail_error(*z, *w)).collect();
    errs.sort_by(f32::total_cmp); // ascending
    let mut acc = 0.0f32;
    for e in &errs {
        acc += *e;
    }
    factor * acc
}

/// Cheap denominator: weighted logits through the clamped exponential.
fn cheap_den(items: &[(f32, f32)]) -> f32 {
    items.iter().map(|(z, w)| w * cheap_exp(*z)).sum()
}

/// True denominator: the same weights against f64 truth.
fn true_den(items: &[(f32, f32)]) -> f64 {
    items.iter().map(|(z, w)| f64::from(*w) * true_exp(*z)).sum()
}

/// Deterministic LCG (Knuth) — no rand; the adversarial set is fixed
/// by the seed.
struct Lcg(u32);

impl Lcg {
    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
}

/// The adversarial set: 40 clamped items at the FIRST f32 step below
/// the clamp (shallowest per-item margin) whose weights park the
/// emitted residuals in the f32 subnormal band, LCG-filtered to
/// products `fl32(w·C)` that round strictly DOWN — the worst rounding
/// direction the factor-2 comment conservatively covers.
fn adversarial_clamped_items() -> Vec<(f32, f32)> {
    let z = CLAMP_FLOOR - 2.0f32.powi(-17); // first f32 value below −80
    let mut rng = Lcg(0x3690_3690);
    let mut items = Vec::new();
    for _ in 0..100_000 {
        let r = f64::from(rng.next_u32()) / f64::from(u32::MAX);
        let w = (5.0e-7 + 1.0e-6 * r) as f32;
        let exact = f64::from(w) * f64::from(CLAMP_RESIDUAL);
        let emitted = w * CLAMP_RESIDUAL; // the f32 product the kernel emits
        if f64::from(emitted) < exact {
            items.push((z, w));
            if items.len() == 40 {
                return items;
            }
        }
    }
    panic!("adversarial set: the seeded LCG must yield 40 down-rounding weights");
}

// ── the arms ──

#[test]
fn f1_clamped_form_silently_understates() {
    // Weighted logits including two deep-clamped items; NO emission.
    let items = [(0.0f32, 1.0f32), (1.0, 1.5), (-90.0, 40.0), (-110.0, 25.0)];
    // The cheap form contributes EXACTLY zero for the clamped items.
    for (z, w) in items.iter().filter(|(z, _)| *z < CLAMP_FLOOR) {
        assert_eq!(*w * cheap_exp(*z), 0.0, "F1 buggy: clamped item must weigh 0");
    }
    // The shortfall vs the f64-true denominator is strictly positive.
    let shortfall = (true_den(&items) - f64::from(cheap_den(&items))).abs();
    assert!(
        shortfall > 0.0,
        "F1 buggy: the clamped form must understate, got {shortfall:e}"
    );
    // The clamp-attributable part of that shortfall — the missing mass.
    let missing = missing_mass(&items);
    assert!(
        missing > 0.0,
        "F1 buggy: the clamped tail mass must be missing, got {missing:e}"
    );
    // The silent mode: with no emission there is no bound to disclose it.
    let disclosed_bound = 0.0f32; // the law-absent path never forms one
    assert!(
        f64::from(disclosed_bound) < missing,
        "F1 silent: an absent emission cannot cover the missing mass"
    );
}

#[test]
fn f2_emitted_tail_covers_true_error() {
    // The SAME weighted logits, now with the residual emitted per item.
    let items = [(0.0f32, 1.0f32), (1.0, 1.5), (-90.0, 40.0), (-110.0, 25.0)];
    let bound = tail_bound(&items, 2.0);
    let missing = missing_mass(&items);
    // A numerical check, not a proof: the bound covers the clamped tail.
    assert!(
        missing <= f64::from(bound),
        "F2 fixed: tail_bound {bound:e} must cover the missing mass {missing:e}"
    );
}

#[test]
fn f3_no_clamp_no_tail_no_harm() {
    // Every logit above the clamp: the law must be inert, never wrong.
    let items = [(-5.0f32, 2.0f32), (-1.0, 0.5), (0.0, 1.0), (1.5, 0.75)];
    for (z, w) in &items {
        assert_eq!(tail_error(*z, *w), 0.0, "F3 safe: residual must be bit-exact 0.0");
    }
    assert_eq!(
        tail_bound(&items, 2.0),
        0.0,
        "F3 safe: the bound must be exactly 0.0"
    );
    let cheap = f64::from(cheap_den(&items));
    let true_d = true_den(&items);
    let rel = (cheap - true_d).abs() / true_d;
    assert!(rel < 1.0e-6, "F3 safe: cheap must track truth, rel err {rel:e}");
}

#[test]
fn f4_factor_two_is_load_bearing() {
    let items = adversarial_clamped_items();
    let missing = missing_mass(&items);
    // Factor 1 UNDER-covers on the adversarial set...
    let bound_f1 = f64::from(tail_bound(&items, 1.0));
    assert!(
        bound_f1 < missing,
        "F4 tampered: the factor-1 bound must under-cover, {bound_f1:e} vs {missing:e}"
    );
    // ...while factor 2 still covers — the multiplier is the content.
    let bound_f2 = f64::from(tail_bound(&items, 2.0));
    assert!(
        bound_f2 >= missing,
        "F4 tampered: the factor-2 bound must still cover, {bound_f2:e} vs {missing:e}"
    );
}

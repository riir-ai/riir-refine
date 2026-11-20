//! Issue 251 — the multiplicative (soft-AND) composition challenger for
//! candidate selection (Research 424; lineage Issue 033 / `selection_tether`).
//!
//! The incumbent composes the two selection legs ADDITIVELY:
//! `W_EVO·evolution + W_RATE·reliability` (0.6 / 0.4, hand-pinned in
//! `self_evolve.rs`, bit-pinned as the cold-start identity by
//! `selection_tether.rs`). Additive forgives a weak leg when the other leg is
//! strong. The challenger composes them as a PRODUCT in probability space,
//! scored in log space: `Σ ln pᵢ = Σ log σ(logit pᵢ)` (ITERLOGIC's
//! `log p₁ + log p₂`) — a candidate resting on one poorly-supported leg scores
//! low, full stop.
//!
//! ## Input law (verdict round 1)
//!
//! Every leg must be sigmoid-bounded: a probability in `(0, 1]`. Never a raw
//! logit, never a value clamped to zero — a leg that reads exactly `0` would
//! send the log-product to `-∞` and erase every other leg's information.
//! [`ProductLeg::new`] REFUSES such a value at construction; the caller
//! (`HealPipeline::select_best_candidate`) degrades the whole pool to the
//! additive incumbent, loudly, when any leg is refused — a pool is never
//! scored under two laws at once.
//!
//! Determinism: the fold is sequential, in leg order (Issue 940 law). House
//! law: sigmoid-bounded legs, never softmax.
//!
//! The additive blend remains the DEMOTE and the cold-start identity: with
//! this feature OFF nothing here compiles, and with it ON the challenger is
//! reached only through the opt-in `SelectionMode::ProductComposition`.

use crate::self_evolve::{W_EVO, W_RATE};

/// One selection leg admitted to the product: a probability in `(0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProductLeg(f32);

impl ProductLeg {
    /// Admit `p` as a product leg, or refuse it.
    ///
    /// Refuses non-finite values, `p ≤ 0` (a zero leg would zero the whole
    /// product — the input law's named failure) and `p > 1` (not a
    /// probability; a raw logit or an unbounded score).
    #[inline]
    #[must_use]
    pub fn new(p: f32) -> Option<Self> {
        (p.is_finite() && p > 0.0 && p <= 1.0).then_some(Self(p))
    }

    /// The admitted probability.
    #[inline]
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

/// `Σ ln pᵢ` — the log of the product, folded sequentially in leg order.
///
/// Always finite and `≤ 0` for admitted legs. Equal to `Σ log σ(logit pᵢ)`.
#[inline]
#[must_use]
pub fn log_product(legs: &[ProductLeg]) -> f32 {
    let mut acc = 0.0f32;
    for leg in legs {
        acc += leg.0.ln();
    }
    acc
}

/// The two-leg selection score under the product law: `ln evo + ln rel`, or
/// `None` when either leg is refused by [`ProductLeg::new`].
#[inline]
#[must_use]
pub fn product_score(evo: f32, reliability: f32) -> Option<f32> {
    Some(log_product(&[
        ProductLeg::new(evo)?,
        ProductLeg::new(reliability)?,
    ]))
}

/// The incumbent's two-leg score, `W_EVO·evo + W_RATE·rel` — the same f32
/// expression `select_best_candidate` evaluates (bit-identical), exposed so
/// the paired replay (`examples/composition_product_replay.rs`) scores both
/// laws from one definition of each.
#[inline]
#[must_use]
pub fn additive_score(evo: f32, reliability: f32) -> f32 {
    W_EVO * evo + W_RATE * reliability
}

/// One-time loud degradation notice: a pool held a refused leg and was scored
/// by the additive incumbent instead (exactly one line per process).
pub(crate) fn warn_product_degraded_once() {
    use std::sync::Once;
    static NOTICE: Once = Once::new();
    NOTICE.call_once(|| {
        eprintln!(
            "riir-refine: SelectionMode::ProductComposition met a leg outside (0, 1] — \
             pool scored by the additive incumbent (Issue 251 input law)"
        );
    });
}

#[cfg(test)]
// Exactness is the property under test (bit-identical incumbent expression).
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn refuses_zero_negative_nonfinite_and_above_one() {
        for bad in [0.0, -0.0, -0.1, 1.000_001, f32::NAN, f32::INFINITY] {
            assert!(ProductLeg::new(bad).is_none(), "{bad} must be refused");
        }
        for ok in [f32::MIN_POSITIVE, 0.5, 1.0] {
            assert!(ProductLeg::new(ok).is_some(), "{ok} must be admitted");
        }
    }

    #[test]
    fn log_product_is_sum_of_logs_in_order() {
        let legs = [ProductLeg::new(0.6).unwrap(), ProductLeg::new(0.3).unwrap()];
        assert_eq!(log_product(&legs), 0.6f32.ln() + 0.3f32.ln());
        assert_eq!(log_product(&[]), 0.0);
        // log σ(logit p) == ln p (the issue's stated form), to f32 rounding.
        let p = 0.3f32;
        let logit = (p / (1.0 - p)).ln();
        let log_sig = -(1.0 + (-logit).exp()).ln();
        assert!((log_sig - p.ln()).abs() < 1e-6);
    }

    #[test]
    fn product_punishes_the_weak_leg_additive_forgives() {
        // Strong evo, weak reliability vs two middling legs.
        let (lopsided, balanced) = ((0.95, 0.05), (0.5, 0.5));
        assert!(
            additive_score(lopsided.0, lopsided.1) > additive_score(balanced.0, balanced.1),
            "additive ranks the lopsided candidate first"
        );
        assert!(
            product_score(lopsided.0, lopsided.1) < product_score(balanced.0, balanced.1),
            "product ranks the balanced candidate first"
        );
    }

    #[test]
    fn product_score_refuses_a_zero_leg() {
        assert_eq!(product_score(0.9, 0.0), None);
        assert_eq!(product_score(0.0, 0.9), None);
    }

    #[test]
    fn additive_score_is_the_incumbent_expression() {
        let (e, r) = (0.37f32, 0.81f32);
        assert_eq!(additive_score(e, r), 0.6 * e + 0.4 * r);
    }
}

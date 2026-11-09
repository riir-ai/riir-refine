//! Vendored-from-`riir-neuron-db` scalar seam (sha `a020ace2a6ee`, vendored
//! 2026-11-09, issue 221 P2b — the publish posture's modelless retrieval
//! substrate; the canonical copy stays in the private sibling, drift caught
//! by `scripts/vendor_ragplane.sh --check` — never hand-edit one side).
//!
//! The two scalar primitives the modelless embedder needs, routed through
//! katgpt-core.
//!
//! UPSTREAM SHAPE, SIMPLIFIED FOR THE REFINED POSTURE (issue 221 P2b): the
//! sibling routes `sigmoid` through a katgpt-optional seam (a transcription
//! mirror arm) because the game editor's graph admits no katgpt package.
//! Refine's dependency law is the opposite — katgpt-core is THE one foreign
//! code dep, present in every build (the publish posture included) — so the
//! mirror arm is dead weight here and the delegation is DIRECT. The
//! committed embedder model id (`modelless-blake3-dft-sigmoid@8d-v1`) is
//! unchanged: same function, same vectors.

/// Logistic sigmoid — the Cephes-polynomial APPROXIMATION `katgpt_core::sigmoid`
/// ships (≤ 1.2e-7 off libm; saturates to exactly 0/1 past ±40).
#[inline]
#[must_use]
pub fn sigmoid(x: f32) -> f32 {
    katgpt_core::sigmoid(x)
}

/// Descending total-order f32 comparator (NaN last, `-0.0` ties `+0.0`).
#[inline]
#[must_use]
pub fn desc(a: f32, b: f32) -> core::cmp::Ordering {
    katgpt_core::float_order::desc(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The embedder's model-id contract: deterministic, saturating, and
    /// bit-stable across calls (the vector is the model).
    #[test]
    fn sigmoid_deterministic_and_saturating() {
        #[allow(clippy::cast_precision_loss)] // the sweep is the point
        for i in -400..=400 {
            let x = i as f32 / 10.0;
            assert_eq!(sigmoid(x), sigmoid(x), "not deterministic at {x}");
        }
        assert_eq!(sigmoid(100.0), 1.0);
        assert_eq!(sigmoid(-100.0), 0.0);
    }

    /// The NaN-last total order the candidate sort relies on.
    #[test]
    fn desc_orders_nan_last_and_ties_zero() {
        assert_eq!(desc(1.0, 0.5), core::cmp::Ordering::Less);
        assert_eq!(desc(f32::NAN, 1.0), core::cmp::Ordering::Greater);
        assert_eq!(desc(0.0, -0.0), core::cmp::Ordering::Equal);
    }
}

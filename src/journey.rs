//! Issue 105 — the journey-credit factor (PC-ALM's dual accumulator,
//! katgpt-rs Research 554 / riir-refine Issue 105 — issue file removed per
//! the noise-reduction rule; the durable record is `HISTORY.md`, landed at
//! `e58ff9b9`).
//!
//! Credit for a kept fix is the **integral of its run's residual history
//! plus the terminal residual** — never the terminal verdict alone. A keep
//! that ends its run at a typeck-masked E0614 (the "strand-keep", the
//! Bench 089/090 poison) has a legal per-attempt lexicographic delta but a
//! poisoned journey, and the per-key success counters cannot tell the two
//! apart. The driver-side accumulator lives in
//! [`crate::domains::rustc_errors::driver::JourneyCredit`] (the
//! `rustc_errors` domain); THIS module is the ungated consumption half —
//! the per-key discount the selection layers apply to success evidence
//! that accumulated inside tainted journeys:
//!
//! - [`crate::self_evolve`] — the reliability axis of
//!   `select_best_candidate` (the generic pipeline, memory.json rows);
//! - [`crate::traj_store::shape_conditional_score`] — the fix-compile
//!   lane's learned ordering (`latent_retrieval` builds).
//!
//! The factor is **exactly `1.0` with no journey data** (`journey_runs ==
//! 0`) **or all-clean journeys** (`journey_taint_sum == 0`) — pre-105
//! stores and memories decode as `(0, 0)` via `#[serde(default)]`, so
//! every existing read is bit-identical. Failures are never discounted:
//! taint reprices SUCCESS evidence only (a strategy that fails in poisoned
//! runs was already fully counted as failing).

/// The per-key journey discount (Issue 105 T2).
///
/// `clamp01(1 − (poverty − 0.5)·2)` where `poverty =
/// sigmoid(journey_taint_sum / (256 · journey_runs))` — the mean ×256
/// fixed-point tainted composite over the runs the key kept an attempt in
/// (each tainted run stamps `α·λ_T + ρ·r_T` in ×256 units; clean runs bump
/// the denominator only). Monotone: more taint ⇒ smaller factor; more
/// clean journeys at fixed taint ⇒ factor recovers toward 1.
#[must_use]
pub fn journey_factor(journey_runs: u32, journey_taint_sum: u32) -> f32 {
    if journey_runs == 0 || journey_taint_sum == 0 {
        return 1.0;
    }
    #[allow(clippy::cast_precision_loss)] // counts are small (< 2^24); f32 is exact
    let mean_composite = journey_taint_sum as f32 / (256.0 * journey_runs as f32);
    let poverty = katgpt_core::sigmoid(mean_composite);
    (1.0 - (poverty - 0.5) * 2.0).clamp(0.0, 1.0)
}

#[cfg(test)]
// Exactness IS the design under test: the neutral/clean points must be
// BIT-IDENTICAL to 1.0 (the legacy-behavior guarantee), not approximately
// equal.
#[allow(clippy::float_cmp)]
mod tests {
    use super::journey_factor;

    /// No journey data ⇒ EXACTLY neutral (bit-identical legacy behavior).
    #[test]
    fn no_data_is_exactly_neutral() {
        assert_eq!(journey_factor(0, 0), 1.0);
        assert_eq!(journey_factor(0, 4_000_000), 1.0);
    }

    /// All-clean journeys ⇒ EXACTLY 1.0 — proven-clean keys are never
    /// penalized relative to no-data keys.
    #[test]
    fn clean_journeys_are_exactly_one() {
        assert_eq!(journey_factor(12, 0), 1.0);
        assert_eq!(journey_factor(1_000, 0), 1.0);
    }

    /// Monotone in taint at fixed runs; recovers with clean journeys at
    /// fixed taint; the deep-strand regime saturates toward 0.
    #[test]
    fn monotone_taint_and_recovery() {
        let f_low = journey_factor(10, 256); // mean composite 0.1
        let f_high = journey_factor(10, 2_560); // mean composite 1.0
        let f_deep = journey_factor(10, 11_520); // the strand ring: mean 4.5
        assert!(f_low > f_high, "{f_low} > {f_high}");
        assert!(f_high > f_deep, "{f_high} > {f_deep}");
        assert!(f_deep < 0.05, "deep strand saturates toward 0: {f_deep}");
        // Clean journeys dilute the same absolute taint back toward 1.
        let f_recovered = journey_factor(100, 2_560);
        assert!(f_recovered > f_high, "dilution must recover: {f_recovered}");
        assert!(f_recovered < 1.0);
        // Bound checks.
        assert!((0.0..=1.0).contains(&f_low));
        assert!((0.0..=1.0).contains(&f_deep));
    }
}

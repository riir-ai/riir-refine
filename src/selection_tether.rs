//! TETHER-adaptive selection blend — the closed-form outcome-fit mixing
//! weight (Issue 033).
//!
//! Source: arXiv:2608.16739 "Le Critique: Privileged Value Functions for LLM
//! Reinforcement Learning" (the TETHER baseline), distilled at
//! `riir-train/.research/426_Le_Critique_PVF_TETHER.md`. The primitive's
//! canonical home is katgpt-core `tether` (katgpt-rs Issue 675 T1-T2); until
//! that lands this module is the **local inline** the issue sanctions — three
//! scalar accumulators and an EMA, deleted in favour of a re-export shim (the
//! [`crate::selection_entropic`] precedent) the moment the primitive ships.
//!
//! ## The defect being fixed
//!
//! [`select_best_candidate`](crate::self_evolve::HealPipeline) scores
//! `W_EVO·evo + W_RATE·reliability` with `W_EVO = 0.6`, `W_RATE = 0.4`. That
//! is exactly the paper's baseline shape `b(ρ) = (1−ρ)·p1 + ρ·p2` with **ρ
//! hand-pinned at 0.4 and never swept** — p1 = the evolved-direction score,
//! p2 = the per-candidate reliability. A fixed ρ cannot express the fact that
//! the two axes carry different amounts of information at different points of
//! the memory lifecycle (cold memory: reliability is noise; hot memory:
//! reliability is the signal).
//!
//! ## The mechanism
//!
//! Fit ρ per window by ordinary least squares against realized outcomes:
//!
//! ```text
//! A_B = R − p1            (the residual the p1-only baseline leaves)
//! Δ   = p2 − p1           (the direction the p2 endpoint moves in)
//! ρ*  = clip[0,1]( Σ A_B·Δ / Σ Δ² )
//! ```
//!
//! with the paper's **exact in-sample guarantee**
//! `SSE(ρ*) ≤ min(SSE(p1-only), SSE(p2-only))` — unconditional, because SSE is
//! convex in ρ and both endpoints (ρ = 0, ρ = 1) are feasible points of the
//! clipped domain. [`fit_rho`] + [`sse`] make that assertable, and
//! `tests/t33_tether_selection_ab.rs` pins it.
//!
//! ## The lag law, encoded as API shape
//!
//! Fitting ρ on a window and applying it to the SAME window makes the blend a
//! function of its own realized returns — bias by construction, the paper's
//! admissibility violation. So the type makes same-window application
//! **unrepresentable**: [`observe`](TetherFit::observe) only ever touches the
//! accumulators, and [`rho`](TetherFit::rho) only ever reads the value
//! published by a *closed* window. The window closes inside `observe` AFTER
//! the sample is folded in, so the new ρ is visible only to later selections.
//! There is no method that applies an open window's fit.
//!
//! Admissibility on the healer's own stream: the realized outcome of a past
//! heal is not caused by the current selection decision, so the lagged fit is
//! admissible de facto (Research 426 §3 channel rule). Outcomes are only
//! *trustworthy* with the `clippy_verify` oracle on — ρ fitting is the
//! learning half of the Issue 018 oracle split and is gated the same way the
//! EMA direction update is.
//!
//! ## Verdict for THIS consumer (Bench 042 — read before adopting)
//!
//! **Measured NEGATIVE on the selection blend; the mode is opt-in and
//! `BetaPosterior` remains the default.** The math is sound (the in-sample
//! guarantee holds exactly, ρ drift is large and reproducible, zero alloc,
//! bit-exact determinism) but ρ* minimizes a *prediction* loss while selection
//! needs a *ranking*, and the reliability axis is deliberately biased low —
//! so the fit downweights it correctly for the loss and wrongly for the metric.
//! The fixed-ρ sweep in the same bench shows discovery is flat over
//! ρ ∈ [0, 0.6] anyway: the pinned 0.4 was not a live lever. Anyone reaching
//! for this type for a NEW consumer: fit ρ against the metric that consumer
//! actually optimizes, or expect this result.
//!
//! ## G4 (allocation)
//!
//! Three `f64` accumulators plus counters, all inline scalars. Zero heap, ever
//! — no warm-up phase to amortize.

/// Default mixing weight — the historical `W_RATE`.
///
/// A fresh [`TetherFit`] therefore reproduces the pinned blend
/// bit-identically until its first window closes: the adaptive mode's
/// cold-start behaviour IS the fixed-weight behaviour (no-regression by
/// construction, pinned by test).
pub const DEFAULT_RHO: f32 = 0.4;

/// Default EMA retention on the per-window fit (`d` in the paper's
/// `ρ_k = d·ρ_{k−1} + (1−d)·ρ̂_k`).
///
/// 0.95 is the paper's own smoothing constant: a single noisy window moves the
/// applied ρ by 5% of its distance, so a starved window cannot swing selection.
pub const DEFAULT_EMA_DECAY: f32 = 0.95;

/// Default observations per window.
///
/// The window is the fit's sample size, so it trades tracking speed against
/// fit variance. 16 is chosen for THIS consumer: a `cargo heal --fix` run
/// applies fixes in the tens-to-hundreds, so 16 gives several windows per run
/// (ρ can move within a run) while keeping `ΣΔ²` far enough from the
/// degenerate guard to fit anything at all. Per-run windows were the
/// alternative the issue offered; they were rejected because a single-window
/// run never applies its own fit — every run would serve the cold-start ρ and
/// the mechanism would be inert without cross-run persistence.
pub const DEFAULT_WINDOW: u32 = 16;

/// `ΣΔ²` floor below which a window is **degenerate** — the two axes agreed
/// on every sample in it, so the data contains no information about their
/// mixing weight and the OLS quotient is `0/0`.
///
/// The guard holds the previous ρ rather than resetting it: worst case the
/// adaptive mode converges to fixed-blend behaviour, never worse.
pub const DEGENERATE_EPS: f64 = 1e-9;

/// Telemetry snapshot of a [`TetherFit`] — the ρ-drift signature (Issue 033
/// T3), which is part of the gate rather than a debugging nicety: a ρ that
/// never moves means the mechanism is inert on this stream, and that is a
/// finding, not a pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TetherStats {
    /// The applied (published, EMA-smoothed) mixing weight.
    pub rho: f32,
    /// The last CLOSED window's raw OLS fit, before EMA smoothing. `None`
    /// until a window closes.
    pub last_rho_hat: Option<f32>,
    /// Observations folded into the currently OPEN window.
    pub open_observations: u32,
    /// Windows closed so far.
    pub windows_closed: u32,
    /// How many of those were degenerate (`ΣΔ² < ε` — previous ρ held).
    pub degenerate_windows: u32,
}

/// Online closed-form fit of the blend weight ρ (three accumulators + EMA).
///
/// See the module docs for the mechanism, the in-sample guarantee, and the
/// lag law this type's shape enforces.
#[derive(Debug, Clone, Copy)]
pub struct TetherFit {
    /// `Σ A_B·Δ` over the open window.
    num: f64,
    /// `Σ Δ²` over the open window.
    den: f64,
    /// Observations in the open window.
    obs: u32,
    /// The applied weight — written ONLY by [`Self::close_window`].
    rho: f32,
    /// Last closed window's raw fit (telemetry).
    last_rho_hat: Option<f32>,
    decay: f32,
    window: u32,
    windows_closed: u32,
    degenerate_windows: u32,
}

impl Default for TetherFit {
    fn default() -> Self {
        Self::new()
    }
}

impl TetherFit {
    /// Fresh fit at [`DEFAULT_RHO`] with the default decay + window.
    #[must_use]
    pub fn new() -> Self {
        Self::with_params(DEFAULT_RHO, DEFAULT_EMA_DECAY, DEFAULT_WINDOW)
    }

    /// Fresh fit with explicit parameters (the A/B knob).
    ///
    /// `rho0` is clamped to `[0, 1]`, `decay` to `[0, 1]`, and `window` is
    /// floored at 1 — a zero-length window would close on every observation
    /// and fit ρ from a single sample.
    #[must_use]
    pub fn with_params(rho0: f32, decay: f32, window: u32) -> Self {
        Self {
            num: 0.0,
            den: 0.0,
            obs: 0,
            rho: rho0.clamp(0.0, 1.0),
            last_rho_hat: None,
            decay: decay.clamp(0.0, 1.0),
            window: if window == 0 { 1 } else { window },
            windows_closed: 0,
            degenerate_windows: 0,
        }
    }

    /// The applied mixing weight — fit on CLOSED windows only (the lag law).
    #[must_use]
    pub const fn rho(&self) -> f32 {
        self.rho
    }

    /// Blend the two estimator endpoints at the applied ρ:
    /// `(1−ρ)·p1 + ρ·p2`.
    #[must_use]
    pub fn blend(&self, p1: f32, p2: f32) -> f32 {
        (1.0 - self.rho) * p1 + self.rho * p2
    }

    /// Fold one realized outcome into the open window.
    ///
    /// `r` is the realized return (1.0 success / 0.0 fail for this consumer),
    /// `p1` / `p2` the two endpoint predictions **as they were at selection
    /// time** — passing post-outcome values would leak the return into its own
    /// baseline.
    ///
    /// Returns `true` iff this observation closed the window (and therefore
    /// republished ρ for *subsequent* selections).
    pub fn observe(&mut self, r: f32, p1: f32, p2: f32) -> bool {
        let a_b = f64::from(r - p1);
        let delta = f64::from(p2 - p1);
        self.num += a_b * delta;
        self.den += delta * delta;
        self.obs += 1;
        // Close AFTER folding: the sample can never influence the ρ that
        // selected it.
        let boundary = self.obs >= self.window;
        if boundary {
            self.close_window();
        }
        boundary
    }

    /// Close the open window: fit ρ̂, EMA it into the applied ρ, reset the
    /// accumulators.
    ///
    /// Called automatically by [`Self::observe`] at the window boundary;
    /// public for consumers that want to flush a short tail (a `cargo heal`
    /// run ending mid-window).
    pub fn close_window(&mut self) {
        self.windows_closed += 1;
        if self.den < DEGENERATE_EPS {
            // Degenerate: the axes never disagreed, so nothing was learned.
            // Hold the previous ρ — the fixed-blend fallback.
            self.degenerate_windows += 1;
        } else {
            let rho_hat = clip_unit(self.num / self.den);
            self.last_rho_hat = Some(rho_hat);
            self.rho = self.decay * self.rho + (1.0 - self.decay) * rho_hat;
        }
        self.num = 0.0;
        self.den = 0.0;
        self.obs = 0;
    }

    /// The OPEN window's raw fit — **telemetry only**, never applied.
    ///
    /// Exposed so the drift signature can be watched at sub-window
    /// resolution; `None` while the open window is degenerate. Applying this
    /// value would be the same-window bias the type exists to prevent, which
    /// is why no method on this type consumes it.
    #[must_use]
    pub fn open_rho_hat(&self) -> Option<f32> {
        if self.den < DEGENERATE_EPS {
            return None;
        }
        Some(clip_unit(self.num / self.den))
    }

    /// Telemetry snapshot (Issue 033 T3).
    #[must_use]
    pub const fn stats(&self) -> TetherStats {
        TetherStats {
            rho: self.rho,
            last_rho_hat: self.last_rho_hat,
            open_observations: self.obs,
            windows_closed: self.windows_closed,
            degenerate_windows: self.degenerate_windows,
        }
    }
}

/// Batch OLS fit of ρ over a recorded `(r, p1, p2)` stream — the same closed
/// form [`TetherFit`] accumulates online, for tests and offline analysis.
///
/// Returns [`DEFAULT_RHO`]'s caller-supplied fallback (`prev`) on a degenerate
/// stream, mirroring the online guard.
#[must_use]
pub fn fit_rho(samples: &[(f32, f32, f32)], prev: f32) -> f32 {
    let (mut num, mut den) = (0.0f64, 0.0f64);
    for &(r, p1, p2) in samples {
        let a_b = f64::from(r - p1);
        let delta = f64::from(p2 - p1);
        num += a_b * delta;
        den += delta * delta;
    }
    if den < DEGENERATE_EPS {
        return prev;
    }
    clip_unit(num / den)
}

/// `clip[0,1]` of the OLS quotient, narrowed to f32.
///
/// The clip is the paper's, not a safety net: ρ outside `[0, 1]` would be an
/// extrapolation past both endpoints, which the never-worse guarantee does not
/// cover. Truncation is intentional — the accumulators are f64 for numerical
/// stability, the applied weight is an f32 blend coefficient.
#[inline]
#[allow(clippy::cast_possible_truncation)]
fn clip_unit(q: f64) -> f32 {
    q.clamp(0.0, 1.0) as f32
}

/// In-sample sum of squared errors of the blend at `rho` over a recorded
/// stream — the quantity the never-worse guarantee is stated in.
#[must_use]
pub fn sse(rho: f32, samples: &[(f32, f32, f32)]) -> f64 {
    samples
        .iter()
        .map(|&(r, p1, p2)| {
            let b = f64::from(1.0 - rho) * f64::from(p1) + f64::from(rho) * f64::from(p2);
            let e = f64::from(r) - b;
            e * e
        })
        .sum()
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)] // loop-index fixtures, exact at these magnitudes
mod tests {
    use super::*;

    /// The cold-start no-regression contract: before any window closes, the
    /// adaptive blend IS the pinned `W_EVO/W_RATE` blend, bit-identically.
    #[test]
    fn cold_start_is_bit_identical_to_the_fixed_blend() {
        let t = TetherFit::new();
        for &(p1, p2) in &[(0.5f32, 0.5f32), (0.62, 0.13), (1.0, 0.0), (0.0, 1.0)] {
            let fixed = 0.6 * p1 + 0.4 * p2;
            assert_eq!(
                t.blend(p1, p2).to_bits(),
                fixed.to_bits(),
                "cold start must reproduce W_EVO·p1 + W_RATE·p2 exactly"
            );
        }
    }

    /// The paper's guarantee, asserted rather than trusted: the fitted ρ never
    /// loses to either endpoint in-sample. No conditions — SSE is convex in ρ
    /// and both endpoints are feasible.
    #[test]
    fn fit_is_never_worse_than_either_endpoint() {
        // Three regimes: p2 informative, p1 informative, and both partly so.
        let streams: [Vec<(f32, f32, f32)>; 3] = [
            (0..64)
                .map(|i| {
                    let p2 = (i % 8) as f32 / 8.0;
                    (p2, 0.5, p2) // R tracks p2 exactly
                })
                .collect(),
            (0..64)
                .map(|i| {
                    let p1 = (i % 8) as f32 / 8.0;
                    (p1, p1, 0.5) // R tracks p1 exactly
                })
                .collect(),
            (0..64)
                .map(|i| {
                    let p1 = (i % 5) as f32 / 5.0;
                    let p2 = (i % 7) as f32 / 7.0;
                    (0.5 * p1 + 0.5 * p2, p1, p2) // both half-informative
                })
                .collect(),
        ];

        for (idx, s) in streams.iter().enumerate() {
            let rho = fit_rho(s, DEFAULT_RHO);
            let (fit, p1_only, p2_only) = (sse(rho, s), sse(0.0, s), sse(1.0, s));
            assert!(
                fit <= p1_only.min(p2_only) + 1e-6,
                "stream {idx}: SSE(rho*={rho}) = {fit} must not exceed \
                 min(p1-only {p1_only}, p2-only {p2_only})"
            );
        }
    }

    /// The recovered ρ is the planted one, not merely "no worse".
    #[test]
    fn fit_recovers_the_planted_weight() {
        for &planted in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let s: Vec<(f32, f32, f32)> = (0..128)
                .map(|i| {
                    let p1 = (i % 11) as f32 / 11.0;
                    let p2 = (i % 13) as f32 / 13.0;
                    ((1.0 - planted) * p1 + planted * p2, p1, p2)
                })
                .collect();
            let rho = fit_rho(&s, DEFAULT_RHO);
            assert!(
                (rho - planted).abs() < 1e-3,
                "planted rho {planted}, recovered {rho}"
            );
        }
    }

    /// Online accumulation must agree with the batch closed form — the same
    /// math, one pass.
    #[test]
    fn online_window_fit_matches_the_batch_fit() {
        let s: Vec<(f32, f32, f32)> = (0..16)
            .map(|i| {
                let p1 = (i % 5) as f32 / 5.0;
                let p2 = (i % 3) as f32 / 3.0;
                (if i % 2 == 0 { 1.0 } else { 0.0 }, p1, p2)
            })
            .collect();

        let mut t = TetherFit::with_params(DEFAULT_RHO, 0.0, 16); // decay 0 ⇒ ρ = ρ̂
        for &(r, p1, p2) in &s {
            t.observe(r, p1, p2);
        }
        let batch = fit_rho(&s, DEFAULT_RHO);
        assert_eq!(t.stats().windows_closed, 1);
        assert!(
            (t.rho() - batch).abs() < 1e-6,
            "online {} vs batch {batch}",
            t.rho()
        );
    }

    /// The lag law: within a window ρ is frozen, and the sample that closes a
    /// window cannot have been selected by the ρ it produces.
    #[test]
    fn rho_is_frozen_inside_a_window() {
        let mut t = TetherFit::with_params(DEFAULT_RHO, 0.0, 4);
        let before = t.rho();
        for i in 0..3 {
            let closed = t.observe(1.0, 0.1, 0.9);
            assert!(!closed, "window must not close early at i={i}");
            assert_eq!(
                t.rho().to_bits(),
                before.to_bits(),
                "rho must not move mid-window"
            );
        }
        assert!(t.observe(1.0, 0.1, 0.9), "the 4th observation closes it");
        assert!(t.rho() > before, "a p2-favouring window must raise rho");
    }

    /// The degenerate guard: identical endpoints carry no mixing information,
    /// so ρ is held rather than fit from `0/0`.
    #[test]
    fn degenerate_window_holds_the_previous_rho() {
        let mut t = TetherFit::with_params(0.4, 0.0, 4);
        for _ in 0..4 {
            t.observe(1.0, 0.7, 0.7); // Δ = 0 every sample
        }
        let st = t.stats();
        assert_eq!(st.rho.to_bits(), 0.4f32.to_bits(), "rho held");
        assert_eq!(st.degenerate_windows, 1);
        assert_eq!(st.last_rho_hat, None);
        assert_eq!(t.open_rho_hat(), None);
    }

    /// Known-answer EMA: a constant ρ̂ converges geometrically from ρ₀.
    #[test]
    fn ema_converges_geometrically_on_constant_fits() {
        let mut t = TetherFit::with_params(0.4, 0.5, 2);
        // Each window plants ρ̂ = 1.0 (R == p2, p1 uninformative).
        let mut expected = 0.4f32;
        for _ in 0..6 {
            t.observe(0.0, 0.5, 0.0);
            t.observe(1.0, 0.5, 1.0);
            expected = 0.5 * expected + 0.5 * 1.0;
            assert!(
                (t.rho() - expected).abs() < 1e-5,
                "EMA {} vs expected {expected}",
                t.rho()
            );
        }
        assert_eq!(t.stats().last_rho_hat, Some(1.0));
    }

    /// Determinism: identical streams give bit-identical ρ.
    #[test]
    fn identical_streams_give_bit_identical_rho() {
        let s: Vec<(f32, f32, f32)> = (0..64)
            .map(|i| {
                let p1 = (i % 7) as f32 / 7.0;
                let p2 = (i % 5) as f32 / 5.0;
                (if i % 3 == 0 { 1.0 } else { 0.0 }, p1, p2)
            })
            .collect();
        let run = || {
            let mut t = TetherFit::new();
            for &(r, p1, p2) in &s {
                t.observe(r, p1, p2);
            }
            t.rho()
        };
        assert_eq!(run().to_bits(), run().to_bits());
    }
}

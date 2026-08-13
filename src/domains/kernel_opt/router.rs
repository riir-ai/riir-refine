//! Kernel expert router — latent-MoE expert selection via `pick_domain`.
//!
//! Wraps [`katgpt_core::variable_rank_domain_expert::pick_domain`] (Plan 558,
//! Research 453) for the kernel optimization domain. The activity vector is
//! the kernel signature; the domain directions are per-expert centroids.
//!
//! ## Placeholder caveat
//!
//! The domain directions are currently **identity vectors** (each expert's
//! direction is the unit vector along its own axis). This makes `pick_domain`
//! degenerate to "pick the expert whose axis has the highest activity value" —
//! a deterministic placeholder. Real per-expert centroids from popcorn
//! distillation land in a follow-up plan (Proposal 012 §Phase 0).
//!
//! ## Domain classification
//!
//! - **Activity vector** = raw physical (kernel signature: M, N, K, dtype
//!   enum, hw enum). Deterministic + replayable.
//! - **Domain directions** = latent (learned centroids, projected to a scalar
//!   expert index via dot-product + argmax). NOT synced — only the resulting
//!   expert index crosses the sync boundary.
//! - **Expert index** = raw categorical (usize, 1 word). Synced.

use katgpt_core::variable_rank_domain_expert::pick_domain;

/// Number of kernel optimization experts. Placeholder = 4 (coalesce, tile,
/// fuse, vectorize). Real expert count from popcorn distillation may differ.
pub const NUM_KERNEL_EXPERTS: usize = 4;

/// Activity vector dimension. The kernel signature: `[M, N, K, dtype, hw]`
/// (normalized to `[0.0, 1.0]` per axis). 5 dims covers the common GEMM /
/// matvec / pointwise shapes.
pub const ACTIVITY_DIM: usize = 5;

/// The kernel optimization experts. Placeholder names matching the 4 axes of
/// the identity direction matrix below.
pub const KERNEL_EXPERT_NAMES: [&str; NUM_KERNEL_EXPERTS] =
    ["coalesce", "tile", "fuse", "vectorize"];

/// Placeholder domain directions — identity vectors (each expert's direction is
/// the unit vector along its own axis).
///
/// This makes `pick_domain` degenerate to "pick the expert whose axis has the
/// highest activity value". Real per-expert centroids from popcorn distillation
/// replace this const in a follow-up plan.
///
/// # Layout
///
/// `domain_directions[e][i]` = direction component `i` for expert `e`.
/// Identity layout: `domain_directions[e][e] == 1.0`, all others `0.0`.
pub const IDENTITY_KERNEL_DIRECTIONS: [[f32; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS] = [
    [1.0, 0.0, 0.0, 0.0, 0.0], // coalesce ← M axis
    [0.0, 1.0, 0.0, 0.0, 0.0], // tile ← N axis
    [0.0, 0.0, 1.0, 0.0, 0.0], // fuse ← K axis
    [0.0, 0.0, 0.0, 1.0, 0.0], // vectorize ← dtype axis
];

/// The kernel expert router — selects ONE expert from `NUM_KERNEL_EXPERTS`
/// candidates based on the kernel signature.
///
/// Wraps [`pick_domain`] (Plan 558). Modelless: pure argmax on
/// `activity · domain_directions`. Ties broken by lowest index (deterministic).
#[derive(Debug, Clone, Copy, Default)]
pub struct KernelExpertRouter {
    /// The domain direction matrix. Defaults to [`IDENTITY_KERNEL_DIRECTIONS`]
    /// (placeholder). Real consumers inject per-expert centroids from popcorn
    /// distillation.
    pub directions: [[f32; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS],
}

impl KernelExpertRouter {
    /// Create a router with the placeholder identity directions.
    #[must_use]
    pub fn new() -> Self {
        Self {
            directions: IDENTITY_KERNEL_DIRECTIONS,
        }
    }

    /// Create a router with explicit domain directions (for when popcorn
    /// distillation lands real centroids).
    #[must_use]
    pub fn with_directions(directions: [[f32; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS]) -> Self {
        Self { directions }
    }

    /// Pick the best expert for the given kernel signature.
    ///
    /// Modelless: pure argmax on `activity · directions`. Zero allocation
    /// (scores stay on the stack in a fixed `[f32; N]` array inside
    /// [`pick_domain`]).
    ///
    /// # Returns
    ///
    /// The expert index in `[0, NUM_KERNEL_EXPERTS)`. Deterministic — same
    /// activity + directions → same expert.
    #[must_use]
    pub fn route(&self, activity: &[f32; ACTIVITY_DIM]) -> usize {
        pick_domain::<NUM_KERNEL_EXPERTS, ACTIVITY_DIM>(activity, &self.directions)
    }

    /// Convenience: route + return the expert's display name.
    ///
    /// Uses [`KERNEL_EXPERT_NAMES`]. Returns `"unknown"` if the index is out
    /// of range (shouldn't happen with the placeholder directions, but guards
    /// against future direction matrices that produce unexpected indices).
    #[must_use]
    pub fn route_named(&self, activity: &[f32; ACTIVITY_DIM]) -> &'static str {
        let idx = self.route(activity);
        KERNEL_EXPERT_NAMES.get(idx).copied().unwrap_or("unknown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_directions_route_to_highest_activity_axis() {
        let router = KernelExpertRouter::new();
        // M axis highest → coalesce (index 0)
        assert_eq!(router.route(&[0.9, 0.1, 0.0, 0.0, 0.0]), 0);
        // N axis highest → tile (index 1)
        assert_eq!(router.route(&[0.1, 0.9, 0.0, 0.0, 0.0]), 1);
        // K axis highest → fuse (index 2)
        assert_eq!(router.route(&[0.0, 0.0, 1.0, 0.0, 0.0]), 2);
        // dtype axis highest → vectorize (index 3)
        assert_eq!(router.route(&[0.0, 0.0, 0.0, 1.0, 0.0]), 3);
    }

    #[test]
    fn route_is_deterministic() {
        let router = KernelExpertRouter::new();
        let activity = [0.5, 0.3, 0.8, 0.1, 0.2];
        let r1 = router.route(&activity);
        let r2 = router.route(&activity);
        assert_eq!(r1, r2);
    }

    #[test]
    fn ties_broken_by_lowest_index() {
        let router = KernelExpertRouter::new();
        // M and N both 0.5 → tie → lowest index (0 = coalesce) wins.
        assert_eq!(router.route(&[0.5, 0.5, 0.0, 0.0, 0.0]), 0);
    }

    #[test]
    fn route_named_returns_correct_name() {
        let router = KernelExpertRouter::new();
        assert_eq!(router.route_named(&[0.9, 0.1, 0.0, 0.0, 0.0]), "coalesce");
        assert_eq!(router.route_named(&[0.1, 0.9, 0.0, 0.0, 0.0]), "tile");
        assert_eq!(router.route_named(&[0.0, 0.0, 1.0, 0.0, 0.0]), "fuse");
        assert_eq!(router.route_named(&[0.0, 0.0, 0.0, 1.0, 0.0]), "vectorize");
    }

    #[test]
    fn with_directions_overrides_identity() {
        // Custom directions where expert 2 dominates all axes.
        let custom = [[0.1, 0.1, 0.1, 0.1, 0.1], [0.1, 0.1, 0.1, 0.1, 0.1],
            [0.9, 0.9, 0.9, 0.9, 0.9], [0.1, 0.1, 0.1, 0.1, 0.1]];
        let router = KernelExpertRouter::with_directions(custom);
        let activity = [0.5, 0.5, 0.5, 0.5, 0.5];
        // Expert 2 has the highest dot product (0.9*5 = 4.5 vs 0.1*5 = 0.5).
        assert_eq!(router.route(&activity), 2);
    }

    #[test]
    fn zero_activity_routes_to_lowest_index() {
        // All-zero activity → all scores 0 → tie → lowest index wins.
        let router = KernelExpertRouter::new();
        assert_eq!(router.route(&[0.0; ACTIVITY_DIM]), 0);
    }

    #[test]
    fn nan_activity_routes_deterministically() {
        // NaN in the activity vector should not panic. pick_domain uses
        // partial_cmp which returns Equal on NaN → ties → lowest index.
        let router = KernelExpertRouter::new();
        let _ = router.route(&[f32::NAN, 0.0, 0.0, 0.0, 0.0]);
        // No panic = pass.
    }

    #[test]
    fn num_experts_and_activity_dim_are_stable() {
        // Pin the const generics so future changes are deliberate.
        assert_eq!(NUM_KERNEL_EXPERTS, 4);
        assert_eq!(ACTIVITY_DIM, 5);
        assert_eq!(KERNEL_EXPERT_NAMES.len(), NUM_KERNEL_EXPERTS);
        assert_eq!(IDENTITY_KERNEL_DIRECTIONS.len(), NUM_KERNEL_EXPERTS);
        assert_eq!(
            IDENTITY_KERNEL_DIRECTIONS[0].len(),
            ACTIVITY_DIM
        );
    }
}

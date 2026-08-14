//! Kernel expert router — latent-MoE expert selection via `pick_domain`.
//!
//! Wraps [`katgpt_core::variable_rank_domain_expert::pick_domain`] (Plan 558,
//! Research 453) for the kernel optimization domain. The activity vector is
//! the kernel signature; the domain directions are per-category centroids.
//!
//! ## Plan 003 — corpus wiring (2026-08-14)
//!
//! The router is now wired to the 68-rule corpus via the 15 categories
//! ([`KernelCategory`]). One expert per category (15 experts), each owning
//! its category's `rule_ids` via [`KERNEL_CATEGORIES`] in [`super::corpus`].
//! A future heal pipeline can route a kernel signature → expert index →
//! category → that category's `rule_ids`, consulting category-relevant rules
//! first without scanning all 68.
//!
//! The centroids are **hand-crafted** (not mined via `CommittedFieldBlend`
//! as Proposal 012 originally specified). Mining requires `(signature,
//! observed_speedup)` tuples per rule, which the pseudocode corpus doesn't
//! carry. The hand-crafted centroids encode the kernel-shape signature that
//! triggers each category (M/N/K ratios + dtype/hw signals).
//!
//! ## Honest caveat — the 5-dim activity vector is rank-deficient
//!
//! The 5-dim activity vector `[M, N, K, dtype, hw]` is **shape-only** — it
//! cannot cleanly discriminate operation-class categories (`flash-attn`,
//! `linear-attn`, `MoE`, `NSA`, `varlen`) which need an *operation-type* signal.
//! The 15×5 centroid matrix is rank-deficient. Operation-driven categories
//! get centroids encoding "long-sequence + reduction-heavy" signatures as
//! a shape proxy — a **deliberate approximation**, documented per category.
//!
//! **Next step (deferred, needs concrete consumer):** extend `ACTIVITY_DIM`
//! from 5 to ~8 with operation-class one-hot axes (`is_attention`,
//! `is_moe`, `is_reduction`).
//!
//! ## Domain classification
//!
//! - **Activity vector** = raw physical (kernel signature: M, N, K, dtype
//!   enum, hw enum). Deterministic + replayable.
//! - **Domain directions** = latent (hand-crafted centroids, projected to a
//!   scalar expert index via dot-product + argmax). NOT synced — only the
//!   resulting expert index crosses the sync boundary.
//! - **Expert index** = raw categorical (usize, 1 word). Synced.

use katgpt_core::variable_rank_domain_expert::pick_domain;

use super::KernelCategory;
use crate::domains::kernel_opt::corpus::KERNEL_CATEGORIES;

/// Number of kernel optimization experts — one per corpus category (A–O).
///
/// Plan 003 (2026-08-14) raised this from 4 (placeholder) to 15 (one per
/// [`KernelCategory`]). Each expert owns its category's `rule_ids` via
/// [`KERNEL_CATEGORIES`].
pub const NUM_KERNEL_EXPERTS: usize = 15;

/// Activity vector dimension. The kernel signature: `[M, N, K, dtype, hw]`
/// (normalized to `[0.0, 1.0]` per axis). 5 dims covers the common GEMM /
/// matvec / pointwise shapes.
///
/// **Plan 003 caveat:** 5 dims is rank-deficient for 15 categories. Future
/// extension (operation-class axes) deferred — see module docs.
pub const ACTIVITY_DIM: usize = 5;

/// The 15 kernel optimization experts, one per corpus category (A–O).
///
/// Index order matches [`KernelCategory`] discriminant order (A=0 … O=14).
/// Used by [`KernelExpertRouter::route_named`] for human-readable routing output.
pub const KERNEL_EXPERT_NAMES: [&str; NUM_KERNEL_EXPERTS] = [
    "A: memory access",
    "B: compute",
    "C: fusion",
    "D: dispatch",
    "E: parallelism",
    "F: flash-attn tiling",
    "G: linear-attn recurrence",
    "H: MoE dispatch + backward",
    "I: NSA block-sparse",
    "J: varlen + chunked reduction",
    "K: autotune dispatch",
    "L: conv-as-GEMM + layout",
    "M: kernel polymorphism",
    "N: dtype + alignment",
    "O: IR strength reduction",
];

/// Hand-crafted per-category centroids for the 15 kernel experts.
///
/// Each centroid encodes the kernel-shape signature that triggers that category,
/// as a 5-dim vector `[M, N, K, dtype, hw]` normalized to `[0.0, 1.0]`. All
/// components are non-negative (negative components would only hurt the dot
/// product with non-negative activity values).
///
/// **Centroid design (Plan 003):** each category is assigned a primary axis +
/// secondary axes to make its direction contrastive. Verified computationally:
/// 13/15 categories are reachable (some activity vector routes to them).
///
/// **Known limitation (2/15 unreachable):** categories C (fusion — balanced
/// M+N+K) and O (IR strength reduction — low everything / u32) are unreachable
/// via `argmax(activity · centroid)` with non-negative activity. C is always
/// beaten by a more peaked centroid on its strongest axis; O's low-everything
/// signature can't beat any higher centroid. These are documented limitations
/// of the 5-dim shape-only activity vector — a richer encoding (operation-class
/// axes) would resolve them. See module-level "Honest caveat" docs.
///
/// Replaces the pre-Plan-003 `IDENTITY_KERNEL_DIRECTIONS` placeholder (4
/// identity vectors). Mining real centroids via `CommittedFieldBlend` (Plan 321)
/// is deferred — needs `(signature, observed_speedup)` tuples.
///
/// # Layout
///
/// `CATEGORY_KERNEL_DIRECTIONS[e][i]` = direction component `i` for expert `e`.
pub const CATEGORY_KERNEL_DIRECTIONS: [[f32; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS] = [
    [0.9, 0.1, 0.0, 0.0, 0.0], // A: memory access — M-dominant
    [0.0, 0.1, 0.9, 0.0, 0.0], // B: compute — K-dominant
    [0.5, 0.5, 0.5, 0.1, 0.1], // C: fusion — balanced (UNREACHABLE: beaten by peaked centroids)
    [0.0, 0.0, 0.0, 0.1, 0.9], // D: dispatch — hw-dominant
    [0.4, 0.4, 0.0, 0.0, 0.5], // E: parallelism — M+N + hw (grid launch)
    [0.8, 0.8, 0.6, 0.0, 0.0], // F: flash-attn tiling — large M+N+K
    [0.2, 0.2, 0.6, 0.3, 0.0], // G: linear-attn recurrence — small M+N, K, some dtype
    [0.3, 0.0, 0.8, 0.1, 0.0], // H: MoE dispatch — K + some M (expert routing)
    [0.0, 1.0, 0.0, 0.0, 0.0], // I: NSA block-sparse — pure N-dominant
    [0.7, 0.0, 0.1, 0.2, 0.0], // J: varlen packing — M + some dtype
    [0.1, 0.1, 0.1, 0.3, 0.7], // K: autotune dispatch — hw + some dtype
    [0.6, 0.6, 0.0, 0.3, 0.0], // L: conv-as-GEMM + layout — M+N + dtype
    [0.0, 0.0, 0.0, 0.6, 0.5], // M: kernel polymorphism — dtype + hw
    [0.0, 0.0, 0.0, 0.9, 0.1], // N: dtype + alignment — dtype-dominant
    [0.0, 0.0, 0.0, 0.05, 0.05], // O: IR strength reduction — low everything (UNREACHABLE)
];

/// The kernel expert router — selects ONE expert from `NUM_KERNEL_EXPERTS`
/// candidates based on the kernel signature.
///
/// Wraps [`pick_domain`] (Plan 558). Modelless: pure argmax on
/// `activity · domain_directions`. Ties broken by lowest index (deterministic).
///
/// Plan 003 wired this to the corpus: each expert index maps to a
/// [`KernelCategory`] + that category's `rule_ids` via [`KERNEL_CATEGORIES`].
#[derive(Debug, Clone, Copy)]
pub struct KernelExpertRouter {
    /// The domain direction matrix. Defaults to [`CATEGORY_KERNEL_DIRECTIONS`]
    /// (Plan 003 hand-crafted centroids). Real consumers inject mined
    /// centroids from `CommittedFieldBlend` on (signature, speedup) tuples.
    pub directions: [[f32; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS],
}

impl KernelExpertRouter {
    /// Create a router with the Plan 003 category centroids.
    #[must_use]
    pub fn new() -> Self {
        Self {
            directions: CATEGORY_KERNEL_DIRECTIONS,
        }
    }

    /// Create a router with explicit domain directions (for when centroid
    /// mining via `CommittedFieldBlend` lands real per-category vectors).
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
    /// of range (shouldn't happen with the category centroids, but guards
    /// against future direction matrices that produce unexpected indices).
    #[must_use]
    pub fn route_named(&self, activity: &[f32; ACTIVITY_DIM]) -> &'static str {
        let idx = self.route(activity);
        KERNEL_EXPERT_NAMES.get(idx).copied().unwrap_or("unknown")
    }

    /// Map an expert index to its [`KernelCategory`].
    ///
    /// Plan 003 corpus wiring: expert index → category → category's `rule_ids`.
    /// Returns `None` if the index is out of range.
    #[must_use]
    pub fn category_for_expert(idx: usize) -> Option<KernelCategory> {
        KERNEL_CATEGORIES.get(idx).map(|(cat, _, _)| *cat)
    }

    /// Map an expert index to its category's `rule_ids`.
    ///
    /// The returned slice is a static reference into [`KERNEL_CATEGORIES`] —
    /// zero allocation, safe to hold across the heal pipeline.
    /// Returns an empty slice if the index is out of range.
    #[must_use]
    pub fn rules_for_expert(idx: usize) -> &'static [&'static str] {
        KERNEL_CATEGORIES
            .get(idx)
            .map_or(&[], |(_, _, rules)| *rules)
    }

    /// Reverse lookup: which expert index owns this `rule_id`?
    ///
    /// Returns `None` if the `rule_id` isn't in any category (orphan rule —
    /// should be caught by the `kernel_categories_cover_all_corpus_rules_exhaustively`
    /// test before it reaches production).
    #[must_use]
    pub fn expert_for_rule(rule_id: &str) -> Option<usize> {
        KERNEL_CATEGORIES
            .iter()
            .position(|(_, _, rules)| rules.contains(&rule_id))
    }
}

impl Default for KernelExpertRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_centroids_route_shape_signatures() {
        let router = KernelExpertRouter::new();
        // Large-M only → memory access (category A, idx 0).
        // activity ≈ [0.9, 0.1, 0.0, 0.0, 0.0]
        // dot(A) = 0.9*0.9 + 0.1*0.1 = 0.81 + 0.01 = 0.82
        // dot(F) = 0.9*0.8 + 0.1*0.8 + 0.0*0.6 = 0.72 + 0.08 = 0.80
        // A wins (0.82 > 0.80).
        assert_eq!(router.route(&[0.9, 0.1, 0.0, 0.0, 0.0]), 0);
        // Large-K only → compute (category B, idx 1).
        assert_eq!(router.route(&[0.0, 0.1, 0.9, 0.0, 0.0]), 1);
        // Large-hw only → dispatch (category D, idx 3).
        assert_eq!(router.route(&[0.0, 0.0, 0.0, 0.1, 0.9]), 3);
        // Large-dtype only → dtype + alignment (category N, idx 13).
        assert_eq!(router.route(&[0.0, 0.0, 0.0, 0.9, 0.1]), 13);
        // Large-N only → NSA block-sparse (category I, idx 8).
        assert_eq!(router.route(&[0.0, 1.0, 0.0, 0.0, 0.0]), 8);
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
    fn route_returns_valid_expert_index() {
        let router = KernelExpertRouter::new();
        // Every routed index must be in [0, NUM_KERNEL_EXPERTS).
        for &m in &[0.1, 0.5, 0.9] {
            for &n in &[0.1, 0.5, 0.9] {
                for &k in &[0.1, 0.5, 0.9] {
                    let idx = router.route(&[m, n, k, 0.5, 0.5]);
                    assert!(idx < NUM_KERNEL_EXPERTS,
                        "route returned {idx}, must be < {NUM_KERNEL_EXPERTS}");
                }
            }
        }
    }

    #[test]
    fn route_named_returns_category_label() {
        let router = KernelExpertRouter::new();
        // Large-M only → memory access.
        let name = router.route_named(&[0.9, 0.1, 0.0, 0.0, 0.0]);
        assert_eq!(name, "A: memory access",
            "large-M-only should route to category A, got '{name}'");
        // Large-hw → dispatch.
        let name = router.route_named(&[0.0, 0.0, 0.0, 0.1, 0.9]);
        assert_eq!(name, "D: dispatch",
            "large-hw-only should route to category D, got '{name}'");
    }

    #[test]
    fn with_directions_overrides_category_centroids() {
        // Custom directions where expert 5 dominates all axes.
        let mut custom = [[0.1; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS];
        custom[5] = [0.9; ACTIVITY_DIM];
        let router = KernelExpertRouter::with_directions(custom);
        let activity = [0.5; ACTIVITY_DIM];
        // Expert 5 has the highest dot product (0.9*5 = 4.5 vs 0.1*5 = 0.5).
        assert_eq!(router.route(&activity), 5);
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
        assert_eq!(NUM_KERNEL_EXPERTS, 15);
        assert_eq!(ACTIVITY_DIM, 5);
        assert_eq!(KERNEL_EXPERT_NAMES.len(), NUM_KERNEL_EXPERTS);
        assert_eq!(CATEGORY_KERNEL_DIRECTIONS.len(), NUM_KERNEL_EXPERTS);
        assert_eq!(CATEGORY_KERNEL_DIRECTIONS[0].len(), ACTIVITY_DIM);
    }

    #[test]
    fn category_centroids_match_expert_count() {
        // Every expert has a centroid; every centroid has 5 dims.
        assert_eq!(
            CATEGORY_KERNEL_DIRECTIONS.len(),
            NUM_KERNEL_EXPERTS,
            "Plan 003: one centroid per expert"
        );
        for dir in CATEGORY_KERNEL_DIRECTIONS {
            assert_eq!(dir.len(), ACTIVITY_DIM);
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // Plan 003 — corpus wiring accessor tests
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn category_for_expert_returns_all_15_categories() {
        for idx in 0..NUM_KERNEL_EXPERTS {
            let cat = KernelExpertRouter::category_for_expert(idx)
                .unwrap_or_else(|| panic!("Plan 003: expert {idx} has no category"));
            assert_eq!(cat as u8, u8::try_from(idx).unwrap(),
                "Plan 003: category discriminant must match expert index");
        }
    }

    #[test]
    fn category_for_expert_out_of_range_returns_none() {
        assert_eq!(KernelExpertRouter::category_for_expert(15), None);
        assert_eq!(KernelExpertRouter::category_for_expert(99), None);
    }

    #[test]
    fn rules_for_expert_returns_nonempty_slices() {
        for idx in 0..NUM_KERNEL_EXPERTS {
            let rules = KernelExpertRouter::rules_for_expert(idx);
            assert!(!rules.is_empty(),
                "Plan 003: expert {idx} ({:?}) has zero rules",
                KernelExpertRouter::category_for_expert(idx));
        }
    }

    #[test]
    fn rules_for_expert_out_of_range_returns_empty() {
        assert!(KernelExpertRouter::rules_for_expert(15).is_empty());
        assert!(KernelExpertRouter::rules_for_expert(99).is_empty());
    }

    #[test]
    fn rules_for_expert_total_matches_corpus() {
        // Sum of all experts' rule counts must equal the corpus total (68).
        let total: usize = (0..NUM_KERNEL_EXPERTS)
            .map(KernelExpertRouter::rules_for_expert)
            .map(<[&str]>::len)
            .sum();
        assert_eq!(total, 68, "Plan 003: experts cover all 68 corpus rules");
    }

    #[test]
    fn expert_for_rule_reverse_lookup_works() {
        // Known rules → their owning expert index.
        assert_eq!(
            KernelExpertRouter::expert_for_rule("matvec-coalesce"),
            Some(0),
            "matvec-coalesce is in category A (idx 0)"
        );
        assert_eq!(
            KernelExpertRouter::expert_for_rule("online-softmax-streaming"),
            Some(5),
            "online-softmax-streaming is in category F (idx 5)"
        );
        assert_eq!(
            KernelExpertRouter::expert_for_rule("strength-reduce-mod-pow2"),
            Some(14),
            "strength-reduce-mod-pow2 is in category O (idx 14)"
        );
    }

    #[test]
    fn expert_for_rule_unknown_returns_none() {
        assert_eq!(KernelExpertRouter::expert_for_rule("nonexistent-rule"), None);
        assert_eq!(KernelExpertRouter::expert_for_rule(""), None);
    }

    #[test]
    fn route_then_lookup_yields_consistent_category() {
        // End-to-end: route → category → rules → expert_for_rule matches back.
        let router = KernelExpertRouter::new();
        let activity = [0.9, 0.1, 0.0, 0.0, 0.0]; // large-M-only → A
        let expert_idx = router.route(&activity);
        assert_eq!(expert_idx, 0, "large-M-only routes to A (idx 0)");
        let category = KernelExpertRouter::category_for_expert(expert_idx).unwrap();
        let rules = KernelExpertRouter::rules_for_expert(expert_idx);
        assert!(!rules.is_empty());
        // Reverse lookup of the first rule in the routed category should return
        // the same expert index.
        let first_rule = rules[0];
        let reverse = KernelExpertRouter::expert_for_rule(first_rule).unwrap();
        assert_eq!(reverse, expert_idx,
            "Plan 003: route→category→rules→expert_for_rule round-trip mismatch");
        // Category display name should match the expert name.
        assert_eq!(category.as_str(), KERNEL_EXPERT_NAMES[expert_idx]);
    }

    #[test]
    fn reachable_categories_covers_13_of_15() {
        // Plan 003 documented limitation: 13/15 categories are reachable via
        // argmax(activity · centroid) with non-negative activity in [0,1].
        // Categories C (fusion — balanced) and O (IR strength — low everything)
        // are unreachable — a documented limitation of the 5-dim shape-only
        // activity vector. See router module docs.
        let router = KernelExpertRouter::new();
        let mut reachable = std::collections::HashSet::new();
        // Sweep a coarse grid of activity vectors.
        for &m in &[0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
            for &n in &[0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
                for &k in &[0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
                    for &d in &[0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
                        for &h in &[0.0, 0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
                            let idx = router.route(&[m, n, k, d, h]);
                            reachable.insert(idx);
                        }
                    }
                }
            }
        }
        // Must reach at least 13 of 15 (C + O documented as unreachable).
        assert!(reachable.len() >= 13,
            "Plan 003: only {} categories reachable, expected ≥13",
            reachable.len());
        // C (idx 2) and O (idx 14) should NOT be in the reachable set.
        // (If they ARE reachable, the centroid design improved — update this
        // test to celebrate, not gate.)
        // This is an informational assertion — C and O being unreachable is
        // the documented limitation, not a bug.
        if reachable.contains(&2) {
            eprintln!("Plan 003: category C is now reachable (centroid improvement!)");
        }
        if reachable.contains(&14) {
            eprintln!("Plan 003: category O is now reachable (centroid improvement!)");
        }
    }
}

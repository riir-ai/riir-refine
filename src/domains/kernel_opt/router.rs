//! Kernel expert router — latent-MoE expert selection via `pick_domain`.
//!
//! Wraps [`katgpt_core::variable_rank_domain_expert::pick_domain`] (Plan 558,
//! Research 453) for the kernel optimization domain. The activity vector is
//! the kernel signature; the domain directions are per-category centroids.
//!
//! ## Plan 003 — corpus wiring (2026-08-14)
//!
//! The router is now wired to the 411-rule corpus via the 15 categories
//! ([`KernelCategory`]). One expert per category (15 experts), each owning
//! its category's `rule_ids` via [`KERNEL_CATEGORIES`] in [`super::corpus`].
//! A future heal pipeline can route a kernel signature → expert index →
//! category → that category's `rule_ids`, consulting category-relevant rules
//! first without scanning all 364.
//!
//! The centroids are **hand-crafted** (not mined via `CommittedFieldBlend`
//! as Proposal 012 originally specified). Mining requires `(signature,
//! observed_speedup)` tuples per rule, which the pseudocode corpus doesn't
//! carry. The hand-crafted centroids encode the kernel-shape signature that
//! triggers each category (M/N/K ratios + dtype/hw signals).
//!
//! ## Plan 004 — 8-dim activity vector (2026-08-14)
//!
//! Plan 003 shipped 5-dim shape-only activity vectors + documented that
//! categories C (fusion) and O (IR strength reduction) were unreachable — a
//! rank-deficiency of the 5-dim encoding. Plan 004 extends to **8 dims** by
//! adding 3 operation-class one-hot axes (`is_attention`, `is_fusion`,
//! `is_ir_transform`). C gets a unique `is_fusion=1.0` signal; O gets a
//! unique `is_ir_transform=1.0` signal; F/G/I get `is_attention=1.0`.
//! **Result: 15/15 categories reachable** (was 13/15).
//!
//! ## Domain classification
//!
//! - **Activity vector** = raw physical (kernel signature: M, N, K, dtype
//!   enum, hw enum, + 3 operation-class one-hot flags). Deterministic +
//!   replayable. See [`build_activity`] for the consumer-facing constructor.
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

/// Activity vector dimension. The kernel signature:
/// `[M, N, K, dtype, hw, is_attention, is_fusion, is_ir_transform]`
/// (normalized to `[0.0, 1.0]` per axis).
///
/// - Axes 0-4 (shape): M, N, K, dtype, hw — cover the common GEMM / matvec /
///   pointwise shapes.
/// - Axes 5-7 (operation class): `is_attention`, `is_fusion`, `is_ir_transform`
///   — one-hot flags set by the consumer based on the kernel's operation type.
///   These resolve the Plan 003 rank-deficiency (C + O were unreachable with
///   shape-only axes).
///
/// See [`build_activity`] for the consumer-facing constructor + the named
/// axis-index constants ([`AXIS_M`] … [`AXIS_IS_IR_TRANSFORM`]).
pub const ACTIVITY_DIM: usize = 8;

// ─── Named axis indices ───────────────────────────────────────────────────
// Consumers should use these constants instead of hardcoding array indices.

/// Axis 0: M dimension (rows / output elements). Normalized to `[0, 1]`.
pub const AXIS_M: usize = 0;
/// Axis 1: N dimension (cols / output elements). Normalized to `[0, 1]`.
pub const AXIS_N: usize = 1;
/// Axis 2: K dimension (reduction inner). Normalized to `[0, 1]`.
pub const AXIS_K: usize = 2;
/// Axis 3: dtype enum (0.0 = f32, 0.5 = f16, 1.0 = i8/u4/etc.). Normalized.
pub const AXIS_DTYPE: usize = 3;
/// Axis 4: hardware enum (0.0 = CPU, 0.5 = Metal, 1.0 = CUDA). Normalized.
pub const AXIS_HW: usize = 4;
/// Axis 5: `is_attention` — one-hot flag for attention-family kernels
/// (flash-attn, linear-attn, NSA). Set by the consumer based on operation type.
pub const AXIS_IS_ATTENTION: usize = 5;
/// Axis 6: `is_fusion` — one-hot flag for multi-op fused kernels
/// (norm+act, quant+matmul, masked softmax). Set by the consumer.
pub const AXIS_IS_FUSION: usize = 6;
/// Axis 7: `is_ir_transform` — one-hot flag for compile-time IR strength
/// reduction (mul/div/mod by pow2 → shift/mask). Set by the consumer.
pub const AXIS_IS_IR_TRANSFORM: usize = 7;

/// Build an activity vector from shape + operation-class flags.
///
/// This is the consumer-facing constructor. A future heal pipeline that
/// receives a kernel signature + operation class calls this then
/// [`KernelExpertRouter::route`].
///
/// # Arguments
///
/// - `m`, `n`, `k`: kernel shape dimensions, normalized to `[0.0, 1.0]`
///   (e.g., `m / MAX_M` where `MAX_M` is the largest expected dim).
/// - `dtype`: dtype signal in `[0.0, 1.0]` (0.0 = f32, 0.5 = f16, 1.0 = low-bit).
/// - `hw`: hardware signal in `[0.0, 1.0]` (0.0 = CPU, 0.5 = Metal, 1.0 = CUDA).
/// - `is_attention`: true for attention-family kernels (flash-attn, linear-attn, NSA).
/// - `is_fusion`: true for multi-op fused kernels.
/// - `is_ir_transform`: true for compile-time IR strength reduction.
///
/// # Zero allocation
///
/// Returns a stack `[f32; ACTIVITY_DIM]` — no heap.
#[must_use]
#[allow(clippy::too_many_arguments)] // 8 args = 8 activity-vector axes
pub fn build_activity(
    m: f32,
    n: f32,
    k: f32,
    dtype: f32,
    hw: f32,
    is_attention: bool,
    is_fusion: bool,
    is_ir_transform: bool,
) -> [f32; ACTIVITY_DIM] {
    [
        m,
        n,
        k,
        dtype,
        hw,
        f32::from(is_attention),
        f32::from(is_fusion),
        f32::from(is_ir_transform),
    ]
}

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
/// Each centroid encodes the kernel-shape signature + operation-class flags
/// that trigger that category, as an 8-dim vector
/// `[M, N, K, dtype, hw, is_attention, is_fusion, is_ir_transform]`
/// normalized to `[0.0, 1.0]`. All components are non-negative (negative
/// components would only hurt the dot product with non-negative activity
/// values).
///
/// **Centroid design (Plan 003 + Plan 004):**
/// - Plan 003 assigned each category a primary shape axis + secondary axes.
///   Verified computationally: 13/15 categories reachable.
/// - Plan 004 added 3 operation-class one-hot axes (`is_attention`,
///   `is_fusion`, `is_ir_transform`) to resolve the 2 unreachable categories
///   (C fusion, O IR strength reduction). **15/15 now reachable.**
///
/// **How C + O became reachable (Plan 004):**
/// - C (fusion) gets `is_fusion=1.0` — no other centroid sets it, so any
///   activity vector with `is_fusion>0` routes to C.
/// - O (IR strength) gets `is_ir_transform=1.0` — same uniqueness argument.
///
/// Mining real centroids via `CommittedFieldBlend` (Plan 321) is deferred —
/// needs `(signature, observed_speedup)` tuples.
///
/// # Layout
///
/// `CATEGORY_KERNEL_DIRECTIONS[e][i]` = direction component `i` for expert `e`.
pub const CATEGORY_KERNEL_DIRECTIONS: [[f32; ACTIVITY_DIM]; NUM_KERNEL_EXPERTS] = [
    [0.9, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], // A: memory access — M-dominant
    [0.0, 0.1, 0.9, 0.0, 0.0, 0.0, 0.0, 0.0], // B: compute — K-dominant
    [0.5, 0.5, 0.5, 0.1, 0.1, 0.0, 1.0, 0.0], // C: fusion — balanced + is_fusion
    [0.0, 0.0, 0.0, 0.1, 0.9, 0.0, 0.0, 0.0], // D: dispatch — hw-dominant
    [0.5, 0.5, 0.0, 0.0, 0.7, 0.0, 0.0, 0.0], // E: parallelism — M+N + hw (grid launch)
    [0.8, 0.8, 0.6, 0.0, 0.0, 1.0, 0.0, 0.0], // F: flash-attn tiling — large M+N+K + is_attention
    [0.2, 0.2, 0.6, 0.3, 0.0, 1.0, 0.0, 0.0], // G: linear-attn recurrence — small M+N, K + is_attention
    [0.2, 0.0, 0.5, 0.5, 0.0, 0.0, 0.0, 0.0], // H: MoE dispatch — moderate M+K + dtype (expert routing)
    [0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0], // I: NSA block-sparse — pure N + is_attention
    [0.85, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0], // J: varlen packing — peaked M + some dtype
    [0.0, 0.0, 0.0, 0.5, 0.8, 0.0, 0.0, 0.0], // K: autotune dispatch — dtype + hw (config sweep)
    [0.6, 0.6, 0.0, 0.3, 0.0, 0.0, 0.0, 0.0], // L: conv-as-GEMM + layout — M+N + dtype
    [0.2, 0.2, 0.0, 0.6, 0.3, 0.0, 0.0, 0.0], // M: kernel polymorphism — small shape + dtype + hw (op-family trait)
    [0.0, 0.0, 0.0, 0.9, 0.1, 0.0, 0.0, 0.0], // N: dtype + alignment — dtype-dominant
    [0.0, 0.0, 0.0, 0.05, 0.05, 0.0, 0.0, 1.0], // O: IR strength reduction — is_ir_transform
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
#[allow(clippy::float_cmp)] // exact comparisons of known f32 values in tests
mod tests {
    use super::*;

    #[test]
    fn category_centroids_route_shape_signatures() {
        let router = KernelExpertRouter::new();
        // Large-M only → memory access (category A, idx 0).
        let activity = build_activity(0.9, 0.0, 0.0, 0.0, 0.0, false, false, false);
        assert_eq!(router.route(&activity), 0);
        // Large-K only → compute (category B, idx 1).
        let activity = build_activity(0.0, 0.0, 0.9, 0.0, 0.0, false, false, false);
        assert_eq!(router.route(&activity), 1);
        // Large-hw only → dispatch (category D, idx 3).
        let activity = build_activity(0.0, 0.0, 0.0, 0.0, 0.9, false, false, false);
        assert_eq!(router.route(&activity), 3);
        // Large-dtype only → dtype + alignment (category N, idx 13).
        let activity = build_activity(0.0, 0.0, 0.0, 0.9, 0.0, false, false, false);
        assert_eq!(router.route(&activity), 13);
        // Large-N + is_attention → NSA block-sparse (category I, idx 8).
        let activity = build_activity(0.0, 1.0, 0.0, 0.0, 0.0, true, false, false);
        assert_eq!(router.route(&activity), 8);
    }

    #[test]
    fn route_is_deterministic() {
        let router = KernelExpertRouter::new();
        let activity = build_activity(0.5, 0.3, 0.8, 0.1, 0.2, false, false, false);
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
                    let activity = build_activity(m, n, k, 0.5, 0.5, false, false, false);
                    let idx = router.route(&activity);
                    assert!(
                        idx < NUM_KERNEL_EXPERTS,
                        "route returned {idx}, must be < {NUM_KERNEL_EXPERTS}"
                    );
                }
            }
        }
    }

    #[test]
    fn route_named_returns_category_label() {
        let router = KernelExpertRouter::new();
        // Large-M only → memory access.
        let activity = build_activity(0.9, 0.0, 0.0, 0.0, 0.0, false, false, false);
        let name = router.route_named(&activity);
        assert_eq!(
            name, "A: memory access",
            "large-M-only should route to category A, got '{name}'"
        );
        // Large-hw → dispatch.
        let activity = build_activity(0.0, 0.0, 0.0, 0.0, 0.9, false, false, false);
        let name = router.route_named(&activity);
        assert_eq!(
            name, "D: dispatch",
            "large-hw-only should route to category D, got '{name}'"
        );
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
        let mut activity = [0.0f32; ACTIVITY_DIM];
        activity[0] = f32::NAN;
        let _ = router.route(&activity);
        // No panic = pass.
    }

    #[test]
    fn num_experts_and_activity_dim_are_stable() {
        // Pin the const generics so future changes are deliberate.
        // Plan 004: ACTIVITY_DIM raised 5 → 8 (operation-class axes).
        assert_eq!(NUM_KERNEL_EXPERTS, 15);
        assert_eq!(ACTIVITY_DIM, 8);
        assert_eq!(KERNEL_EXPERT_NAMES.len(), NUM_KERNEL_EXPERTS);
        assert_eq!(CATEGORY_KERNEL_DIRECTIONS.len(), NUM_KERNEL_EXPERTS);
        assert_eq!(CATEGORY_KERNEL_DIRECTIONS[0].len(), ACTIVITY_DIM);
    }

    #[test]
    fn category_centroids_match_expert_count() {
        // Every expert has a centroid; every centroid has ACTIVITY_DIM dims.
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
            assert_eq!(
                cat as u8,
                u8::try_from(idx).unwrap(),
                "Plan 003: category discriminant must match expert index"
            );
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
            assert!(
                !rules.is_empty(),
                "Plan 003: expert {idx} ({:?}) has zero rules",
                KernelExpertRouter::category_for_expert(idx)
            );
        }
    }

    #[test]
    fn rules_for_expert_out_of_range_returns_empty() {
        assert!(KernelExpertRouter::rules_for_expert(15).is_empty());
        assert!(KernelExpertRouter::rules_for_expert(99).is_empty());
    }

    #[test]
    fn rules_for_expert_total_matches_corpus() {
        // Sum of all experts' rule counts must equal the corpus total.
        let total: usize = (0..NUM_KERNEL_EXPERTS)
            .map(KernelExpertRouter::rules_for_expert)
            .map(<[&str]>::len)
            .sum();
        assert_eq!(total, 734, "Plan 003: experts cover all 734 corpus rules (Batch 197 +6, the mlx VJP distill — E +2 the composed-backward lead + the threadgroup-scope single-writer, G +3 the entry-state checkpoint + the forward-save cache + the chain-rule postprocess, A +1 the GQA zero-stride broadcast; plus the operand-typed-mma rider on B192; re-pinned 734 with the honest label)");
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
        assert_eq!(
            KernelExpertRouter::expert_for_rule("nonexistent-rule"),
            None
        );
        assert_eq!(KernelExpertRouter::expert_for_rule(""), None);
    }

    #[test]
    fn route_then_lookup_yields_consistent_category() {
        // End-to-end: route → category → rules → expert_for_rule matches back.
        let router = KernelExpertRouter::new();
        let activity = build_activity(0.9, 0.0, 0.0, 0.0, 0.0, false, false, false); // → A
        let expert_idx = router.route(&activity);
        assert_eq!(expert_idx, 0, "large-M-only routes to A (idx 0)");
        let category = KernelExpertRouter::category_for_expert(expert_idx).unwrap();
        let rules = KernelExpertRouter::rules_for_expert(expert_idx);
        assert!(!rules.is_empty());
        // Reverse lookup of the first rule in the routed category should return
        // the same expert index.
        let first_rule = rules[0];
        let reverse = KernelExpertRouter::expert_for_rule(first_rule).unwrap();
        assert_eq!(
            reverse, expert_idx,
            "Plan 003: route→category→rules→expert_for_rule round-trip mismatch"
        );
        // Category display name should match the expert name.
        assert_eq!(category.as_str(), KERNEL_EXPERT_NAMES[expert_idx]);
    }

    // ════════════════════════════════════════════════════════════════════
    // Plan 004 — 8-dim activity vector + 15/15 reachability
    // ════════════════════════════════════════════════════════════════════

    #[test]
    fn build_activity_sets_flags_correctly() {
        // All shape axes + all flags off.
        let a = build_activity(0.5, 0.3, 0.8, 0.1, 0.2, false, false, false);
        assert_eq!(a, [0.5, 0.3, 0.8, 0.1, 0.2, 0.0, 0.0, 0.0]);
        // All flags on.
        let a = build_activity(0.0, 0.0, 0.0, 0.0, 0.0, true, true, true);
        assert_eq!(a, [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        // Mixed flags.
        let a = build_activity(0.9, 0.1, 0.0, 0.0, 0.0, false, true, false);
        assert_eq!(a[AXIS_M], 0.9);
        assert_eq!(a[AXIS_IS_FUSION], 1.0);
        assert_eq!(a[AXIS_IS_ATTENTION], 0.0);
        assert_eq!(a[AXIS_IS_IR_TRANSFORM], 0.0);
    }

    #[test]
    fn build_activity_dimension_matches_const() {
        let a = build_activity(0.0, 0.0, 0.0, 0.0, 0.0, false, false, false);
        assert_eq!(a.len(), ACTIVITY_DIM);
    }

    #[test]
    fn all_15_categories_reachable() {
        // Plan 004: with the 8-dim activity vector (operation-class axes added),
        // ALL 15 categories should be reachable. Plan 003 had 13/15 (C + O
        // unreachable); the is_fusion + is_ir_transform axes resolve both.
        let router = KernelExpertRouter::new();
        let mut reachable = std::collections::HashSet::new();
        // Coarse grid sweep: 5 values per dim × 8 dims = 390,625 points.
        // Covers shape axes [0..4] + operation-class axes [5..7].
        for &m in &[0.0, 0.25, 0.5, 0.75, 1.0] {
            for &n in &[0.0, 0.25, 0.5, 0.75, 1.0] {
                for &k in &[0.0, 0.25, 0.5, 0.75, 1.0] {
                    for &d in &[0.0, 0.25, 0.5, 0.75, 1.0] {
                        for &h in &[0.0, 0.25, 0.5, 0.75, 1.0] {
                            for &attn in &[0.0, 1.0] {
                                for &fus in &[0.0, 1.0] {
                                    for &ir in &[0.0, 1.0] {
                                        let idx = router.route(&[m, n, k, d, h, attn, fus, ir]);
                                        reachable.insert(idx);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            reachable.len(),
            NUM_KERNEL_EXPERTS,
            "Plan 004: expected all {NUM_KERNEL_EXPERTS} categories reachable, \
             got {}: missing = {:?}",
            reachable.len(),
            (0..NUM_KERNEL_EXPERTS)
                .filter(|i| !reachable.contains(i))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn per_category_canonical_routing() {
        // Each category has a canonical activity vector that routes to it.
        // This proves the router can distinguish all 15 categories — not just
        // that some grid point hits each (which could be coincidental).
        let router = KernelExpertRouter::new();
        let cases: [(KernelCategory, [f32; ACTIVITY_DIM]); NUM_KERNEL_EXPERTS] = [
            // Canonical activity vectors found via fine-grid max-margin search.
            // Each maximizes (target_score - max_other_score) over a 21^5×2^3 grid.
            (
                KernelCategory::MemoryAccess,
                build_activity(1.0, 0.05, 0.0, 0.0, 0.0, false, false, false),
            ),
            (
                KernelCategory::Compute,
                build_activity(0.0, 0.0, 1.0, 0.0, 0.0, false, false, false),
            ),
            (
                KernelCategory::Fusion,
                build_activity(0.0, 0.0, 0.0, 0.0, 0.0, false, true, false),
            ),
            (
                KernelCategory::Dispatch,
                build_activity(0.0, 0.0, 0.0, 0.0, 1.0, false, false, false),
            ),
            (
                KernelCategory::Parallelism,
                build_activity(0.3, 0.85, 0.0, 0.0, 1.0, false, false, false),
            ),
            (
                KernelCategory::FlashAttnTiling,
                build_activity(1.0, 1.0, 1.0, 0.0, 0.0, true, false, false),
            ),
            (
                KernelCategory::LinearAttnRecurrence,
                build_activity(0.0, 0.0, 0.05, 1.0, 0.0, true, false, false),
            ),
            (
                KernelCategory::MoeDispatchBackward,
                build_activity(0.5, 0.0, 0.85, 1.0, 0.0, false, false, false),
            ),
            (
                KernelCategory::NsaBlockSparse,
                build_activity(0.0, 1.0, 0.0, 0.0, 0.0, true, false, false),
            ),
            (
                KernelCategory::VarlenChunkedReduction,
                build_activity(1.0, 0.0, 0.0, 1.0, 0.0, false, false, false),
            ),
            (
                KernelCategory::AutotuneDispatch,
                build_activity(0.0, 0.0, 0.0, 1.0, 1.0, false, false, false),
            ),
            (
                KernelCategory::ConvGemmLayout,
                build_activity(0.25, 0.9, 0.0, 1.0, 0.0, false, false, false),
            ),
            (
                KernelCategory::KernelPolymorphism,
                build_activity(0.15, 0.9, 0.0, 0.95, 0.55, false, false, false),
            ),
            (
                KernelCategory::DtypeAlignment,
                build_activity(0.0, 0.0, 0.0, 1.0, 0.0, false, false, false),
            ),
            (
                KernelCategory::IrStrengthReduction,
                build_activity(0.0, 0.0, 0.0, 0.0, 0.0, false, false, true),
            ),
        ];
        for (expected_cat, activity) in cases {
            let expert_idx = router.route(&activity);
            let actual_cat = KernelExpertRouter::category_for_expert(expert_idx)
                .expect("routed expert must have a category");
            assert_eq!(actual_cat, expected_cat,
                "canonical activity for {expected_cat:?} routed to {actual_cat:?} (expert {expert_idx}), \
                 activity = {activity:?}");
        }
    }

    #[test]
    fn routed_heal_narrows_candidates_and_drafts_valid_fix() {
        // Verify the Plan 003/004 composition: route → narrow → draft.
        // This is the automated test behind the `kernel_opt_demo` example —
        // it asserts the composition works, not just that it prints.
        use crate::domains::kernel_opt::corpus::build_kernel_seed_corpus;
        use crate::drafter::TernaryDrafter;

        let router = KernelExpertRouter::new();
        let corpus = build_kernel_seed_corpus();

        // Route a flash-attention signature → expert F (idx 5).
        let activity = build_activity(1.0, 1.0, 1.0, 0.0, 0.0, true, false, false);
        let expert = router.route(&activity);
        let category = KernelExpertRouter::category_for_expert(expert)
            .expect("routed expert must have a category");
        assert_eq!(
            category,
            KernelCategory::FlashAttnTiling,
            "flash-attn signature should route to F, got {category:?}"
        );

        // Narrow candidates to the routed category.
        let rule_ids = KernelExpertRouter::rules_for_expert(expert);
        let candidates: Vec<_> = corpus
            .entries
            .iter()
            .filter(|e| rule_ids.contains(&e.rule_id.as_str()))
            .cloned()
            .collect();

        // The narrowed set must be smaller than the full corpus.
        assert!(
            candidates.len() < corpus.entries.len(),
            "narrowed set ({}) should be smaller than full corpus ({})",
            candidates.len(),
            corpus.entries.len()
        );
        assert!(!candidates.is_empty(), "narrowed set should not be empty");

        // Draft a fix from the narrowed set.
        let mut drafter = TernaryDrafter::fallback();
        let rule_id = &candidates[0].rule_id;
        let buggy = &candidates[0].ref_shape;
        let fix = drafter.draft_fix(rule_id, buggy, &candidates);
        assert!(
            fix.is_some(),
            "drafter should produce a fix from narrowed candidates"
        );

        // The drafted fix must come from the narrowed category.
        let result = fix.unwrap();
        let matching = candidates.iter().any(|c| c.opt_pattern == result.code);
        assert!(
            matching,
            "drafted fix must come from the narrowed candidate set"
        );
    }

    #[test]
    fn routed_heal_narrowing_is_category_scoped() {
        // Verify that routing to different categories produces different
        // candidate sets — the router actually discriminates.
        use crate::domains::kernel_opt::corpus::build_kernel_seed_corpus;

        let router = KernelExpertRouter::new();
        let corpus = build_kernel_seed_corpus();

        // Route to category A (memory access) — high M only.
        let activity_a = build_activity(0.9, 0.05, 0.0, 0.0, 0.0, false, false, false);
        let expert_a = router.route(&activity_a);
        let rules_a = KernelExpertRouter::rules_for_expert(expert_a);

        // Route to category O (IR strength reduction) — is_ir_transform flag.
        let activity_o = build_activity(0.0, 0.0, 0.0, 0.0, 0.0, false, false, true);
        let expert_o = router.route(&activity_o);
        let rules_o = KernelExpertRouter::rules_for_expert(expert_o);

        // Different experts.
        assert_ne!(
            expert_a, expert_o,
            "memory-access vs IR-strength-reduction should route to different experts"
        );

        // Different rule sets.
        assert_ne!(
            rules_a, rules_o,
            "different experts should have different rule sets"
        );

        // Both must have non-empty rule sets in the corpus.
        let candidates_a: Vec<_> = corpus
            .entries
            .iter()
            .filter(|e| rules_a.contains(&e.rule_id.as_str()))
            .collect();
        let candidates_o: Vec<_> = corpus
            .entries
            .iter()
            .filter(|e| rules_o.contains(&e.rule_id.as_str()))
            .collect();
        assert!(
            !candidates_a.is_empty(),
            "category A should have corpus rules"
        );
        assert!(
            !candidates_o.is_empty(),
            "category O should have corpus rules"
        );

        // No overlap — categories partition the corpus.
        let overlap: Vec<_> = rules_a.iter().filter(|r| rules_o.contains(r)).collect();
        assert!(
            overlap.is_empty(),
            "categories A and O should not share rules (found: {overlap:?})"
        );
    }
}

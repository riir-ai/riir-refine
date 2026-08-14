//! Cross-domain router — which healing domain should own a code span
//! (Plan 005 T1.4).
//!
//! Routing signal: the code chunk's [`ModellessEmbedder`] direction (8-D
//! structural fingerprint) scored against per-domain **centroid directions**,
//! each the L2-normalized mean of that domain's corpus pattern embeddings.
//! Deterministic, modelless (no trained weights — centroids are derived from
//! the corpora at build time).
//!
//! ## Why not `katgpt_core::pick_domain::<3, 8>` directly
//!
//! `pick_domain` is `const`-generic over the domain count `N`, but the
//! enabled-domain set here is **feature-dependent** (clippy+perf by default;
//! +kernel with `kernel_heal`) — a runtime-sized direction list. `route()`
//! therefore implements the identical dot-product argmax (ties → lowest
//! index) over `Vec` directions, mirroring
//! `katgpt_core::variable_rank_domain_expert::pick_domain` (Plan 558) — the
//! same routing primitive the kernel-15 expert router consumes at
//! compile-time N.

use riir_rag::{query::LATENT_DIM, Embedder, ModellessEmbedder};

/// The healing domains — index into the router's direction list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainId {
    /// `clippy_lints` — lint-shaped style/complexity fixes.
    Clippy,
    /// `rust_perf` — perf-anti-pattern fixes.
    Perf,
    /// `kernel_opt` — GPU kernel optimization patterns.
    Kernel,
}

impl DomainId {
    /// All domains, in router order.
    pub const ALL: [DomainId; 3] = [DomainId::Clippy, DomainId::Perf, DomainId::Kernel];

    /// Stable label (feature-independent).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            DomainId::Clippy => "clippy",
            DomainId::Perf => "perf",
            DomainId::Kernel => "kernel",
        }
    }
}

/// Cross-domain centroid router.
///
/// Built from the ENABLED domains' corpus pattern texts. A disabled domain
/// simply has no direction — it can never be routed to.
pub struct DomainRouter {
    directions: Vec<[f32; LATENT_DIM]>,
    ids: Vec<DomainId>,
}

impl DomainRouter {
    /// Build from `(domain, pattern_texts)` — one entry per enabled domain.
    ///
    /// Each domain's direction = L2-normalized mean of its pattern
    /// embeddings.
    ///
    /// # Panics
    ///
    /// Panics if any domain has an empty pattern list (a centroid of nothing
    /// is undefined).
    #[must_use]
    pub fn new(domains: Vec<(DomainId, Vec<String>)>) -> Self {
        let mut directions = Vec::with_capacity(domains.len());
        let mut ids = Vec::with_capacity(domains.len());
        let embedder = ModellessEmbedder;
        for (id, patterns) in domains {
            assert!(
                !patterns.is_empty(),
                "DomainRouter::new: domain {} has no patterns",
                id.label()
            );
            let mut centroid = [0.0f32; LATENT_DIM];
            for pattern in &patterns {
                let emb = embedder.embed(pattern);
                for (c, v) in centroid.iter_mut().zip(emb) {
                    *c += v;
                }
            }
            #[allow(clippy::cast_precision_loss)] // corpus sizes are ≤ ~10²; f32 mantissa is plenty
            let n = patterns.len() as f32;
            let mut norm = 0.0f32;
            for c in &mut centroid {
                *c /= n;
                norm += *c * *c;
            }
            let norm = norm.sqrt();
            if norm > f32::EPSILON {
                for c in &mut centroid {
                    *c /= norm;
                }
            }
            directions.push(centroid);
            ids.push(id);
        }
        Self { directions, ids }
    }

    /// Which domains this router knows.
    #[must_use]
    pub fn domains(&self) -> &[DomainId] {
        &self.ids
    }

    /// Route a code chunk to its owning domain: argmax over
    /// `dot(embedding, domain_centroid)`; ties → lowest domain index
    /// (deterministic, mirroring `pick_domain`).
    ///
    /// Returns `None` if the router was built with no domains.
    #[must_use]
    pub fn route(&self, code: &str) -> Option<DomainId> {
        if self.ids.is_empty() {
            return None;
        }
        let activity = ModellessEmbedder.embed(code);
        self.route_activity(&activity)
    }

    /// Route a pre-computed embedding (avoids re-embedding when the caller
    /// already has the direction).
    #[must_use]
    pub fn route_activity(&self, activity: &[f32; LATENT_DIM]) -> Option<DomainId> {
        let mut best = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for (i, dir) in self.directions.iter().enumerate() {
            let mut s = 0.0f32;
            for (a, d) in activity.iter().zip(dir) {
                s += a * d;
            }
            // partial_cmp: NaN ties → keeps the earlier index (same as
            // pick_domain's tie semantics).
            if s.partial_cmp(&best_score) == Some(std::cmp::Ordering::Greater) {
                best_score = s;
                best = i;
            }
        }
        self.ids.get(best).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_domains() -> Vec<(DomainId, Vec<String>)> {
        vec![
            (
                DomainId::Clippy,
                vec![
                    // Signature: short 1-line fn bodies with `return expr;`.
                    "fn a(x: i32) -> i32 { return x; }".to_string(),
                    "fn b() -> u8 { return 3; }".to_string(),
                    "fn c(v: u64) -> u64 { return v + 1; }".to_string(),
                    // + closure shape.
                    "let v = xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();".to_string(),
                ],
            ),
            (
                DomainId::Perf,
                vec![
                    // Signature: loop-shaped accumulation / scanning WITHOUT
                    // short-fn wrappers.
                    "let mut out = Vec::new(); for x in xs { out.push(f(x)); }".to_string(),
                    "let mut total = 0; for v in vals { total += v; }".to_string(),
                    "let mut found = false; for u in users { if u.id == id { found = true; } }".to_string(),
                ],
            ),
            (
                DomainId::Kernel,
                vec![
                    // Signature: nested float-accumulation loops + GPU
                    // position intrinsics.
                    "for col in 0..N { let mut acc = 0.0; for row in 0..N { acc += a[row][col] * x[row]; } y[col] = acc; }".to_string(),
                    "let tile = tile_group.thread_position_in_group();".to_string(),
                ],
            ),
        ]
    }

    #[test]
    fn centroids_are_unit_norm_and_distinct() {
        let router = DomainRouter::new(fixture_domains());
        assert_eq!(router.domains().len(), 3);
        for (i, dir) in router.directions.iter().enumerate() {
            let norm: f32 = dir.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-4,
                "centroid {i} not unit-norm: {norm}"
            );
        }
        // Clippy vs kernel centroids must not be identical (sanity —
        // separation quality is measured by the Phase 3 G1 gate).
        let d: f32 = router.directions[0]
            .iter()
            .zip(&router.directions[2])
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt();
        assert!(d > 1e-3, "clippy/kernel centroids nearly identical (d={d})");
    }

    #[test]
    fn routes_pattern_shaped_snippets_to_their_domain() {
        let router = DomainRouter::new(fixture_domains());
        // needless-return shape → clippy
        assert_eq!(router.route("fn f() -> u8 { return 7; }"), Some(DomainId::Clippy));
        // naive matvec strided-read shape → kernel
        assert_eq!(
            router.route(
                "for c in 0..M { let mut acc = 0.0; for r in 0..K { acc += a[r][c] * x[r]; } y[c] = acc; }"
            ),
            Some(DomainId::Kernel)
        );
    }

    #[test]
    fn routing_is_deterministic() {
        let router = DomainRouter::new(fixture_domains());
        let a = router.route("fn f() -> u8 { return 7; }");
        let b = router.route("fn f() -> u8 { return 7; }");
        assert_eq!(a, b);
    }

    #[test]
    fn empty_router_routes_none() {
        let router = DomainRouter::new(Vec::new());
        assert_eq!(router.route("fn f() {}"), None);
    }

    #[test]
    #[should_panic(expected = "no patterns")]
    fn empty_domain_patterns_rejected() {
        let _ = DomainRouter::new(vec![(DomainId::Perf, Vec::new())]);
    }
}

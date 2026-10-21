//! Issue 134 T2 — the pool-depth **allocation gate** (feature
//! `demonstrability_gate`, default off).
//!
//! T1 ([`crate::demonstrability`]) measures the per-domain constant; THIS
//! module consumes it as the allocation principle the paper states:
//! **candidate-pool depth pays in proportion to measured selection
//! discriminability** — where selection reliably lands the correct
//! candidate when present, generation breadth pays (deeper pools); where
//! it does not, generation is wasted at any depth and the budget belongs
//! to discrimination work instead.
//!
//! ## What scales
//!
//! The suggest lane's two depth choices, per domain, in
//! [`LatentMatcher::suggest_all_domains`](crate::draft::LatentMatcher):
//!
//! - the **retrieval pool** (fan-out width, `top_k.max(RERANK_POOL)`), and
//! - the **truncation K** (rerank truncation).
//!
//! Both scale by the same per-domain factor, so `pool >= k` is preserved
//! (a max is preserved under positive scaling) and the rerank probe still
//! sees everything the fan-out retrieved.
//!
//! ## Budget-conserving by construction
//!
//! The gate is an ALLOCATION gate, not an inflation gate: raw factors are
//! normalized to unit mean **over the measured set only**. What a
//! low-demonstrability domain gives up, the high-demonstrability domains
//! receive — total fan-out work is conserved across the measured set.
//! Unmeasured domains stay at EXACTLY `1.0`: absence of a measurement is
//! never a low measurement (the frontier-report law one layer up), and
//! the gate never punishes a domain for lacking a labeled set.
//!
//! Two honest consequences, both deliberate:
//!
//! - **A single measured domain is identity** — with nothing to
//!   reallocate against, normalization returns the raw/mean = 1.0. The
//!   gate only ever acts on measured DIFFERENCES between domains.
//! - **The standing constants are inert after rounding**: clippy 0.935 /
//!   `rust_perf` 0.962 give factors 0.990/1.011 — pool 28 → 28, K 3 → 3.
//!   Two high nearly-equal constants mean no reallocation, which is the
//!   correct answer, not a defect. The gate earns teeth as more domains
//!   carry labeled sets (T3's ≥5-domain power floor is the same shape).
//!
//! ## Byte-identity
//!
//! - Feature OFF: the field, builder and call-site scaling compile away —
//!   byte-identical by construction.
//! - Feature ON, no policy installed (or an identity policy): [`Row`]
//!   scaling returns `base` **exactly** (the identity fast path) —
//!   byte-identical, and required by the A/B control.
//!
//! ## Kill-switch
//!
//! `RIIR_REFINE_DEMONSTRABILITY=0` guards the AUTO-arm path ([`standing`])
//! — the loader returns `None` and the run stays at identity. Explicit
//! injection ([`crate::draft::LatentMatcher::with_depth_policy`]) is the
//! A/B override and is deliberately not env-guarded: the operator who
//! injects a policy explicitly can simply not inject one (the
//! `set_rerank` precedent checks env at the use site; the suggest lane's
//! per-span hot path forbids a per-call env lookup, so the check rides
//! the loader).
//!
//! ## Out of scope (recorded, not hidden)
//!
//! - `pool_and_ranked` (the Issue-036 read surface) does NOT scale — it
//!   is the T1 measurement itself; scaling it would contaminate the
//!   column the gate consumes.
//! - The L4 fallback seam has NO numeric arming threshold in the shipped
//!   contract (consultation is structural: only on a genuine miss —
//!   `l4_fallback`'s four invariants). A numeric threshold is the future
//!   surface here if one ever ships.
//! - The trajectory-memory lane (`collapse_monitor`'s `TOP_K = 5`) is a
//!   different substrate ([`FixTrajectory`] retrieval, not the rule corpus).
//!
//! [`Row`]: crate::demonstrability::Row
//! [`FixTrajectory`]: crate::memory::FixTrajectory

// Count arithmetic: usize→f32 for pool/K scaling — the pedantic cast
// lints are noise, allowed with cause (the demonstrability.rs precedent).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

use std::collections::BTreeMap;
use std::path::Path;

use crate::demonstrability::{canonical_domain, Row};

/// The runtime kill-switch (guards [`standing`], the auto-arm path).
/// Deliberately exact: only the literal `"0"` disarms — a typo must not
/// silently disarm the gate (the auto-oracle's truthy-spelling law).
pub const KILL_SWITCH_ENV: &str = "RIIR_REFINE_DEMONSTRABILITY";

/// The raw-multiplier floor (a low-demonstrability domain still retrieves
/// HALF depth — it keeps a pool, just not a deep one).
pub const MIN_FACTOR: f32 = 0.5;
/// The raw-multiplier ceiling (a high-demonstrability domain at most
/// doubles its depth).
pub const MAX_FACTOR: f32 = 2.0;

/// Where the standing constants live, relative to the healer repo root.
pub const STANDING_RELPATH: &str = ".benchmarks/score_history.jsonl";

/// The per-domain depth policy — a factor per measured domain, mean 1.0
/// over the measured set, `1.0` for everything else.
///
/// Deterministic in its input (a `BTreeMap` — iteration order is part of
/// the debug render). Pure: no I/O, no env reads. The loader ([`standing`])
/// is the only I/O-bearing entry point.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DepthPolicy {
    /// Canonical domain id → factor (the `demonstrability::Row` spelling).
    factors: BTreeMap<String, f32>,
}

impl DepthPolicy {
    /// The do-nothing policy: every domain scales at exactly 1.0.
    #[must_use]
    pub fn identity() -> Self {
        Self::default()
    }

    /// Build from the standing-column rows. Only QUALIFYING rows enter
    /// ([`Row::qualifies`] — domain aggregate, non-empty labeled set, not
    /// `"other"`); per-rule rows, `"other"` rows and empty sets are
    /// ignored (attribution is membership, never a guess).
    ///
    /// Raw factor = `MIN + (MAX − MIN) · constant`, normalized to unit
    /// mean over the measured set, then clamped back into
    /// `[MIN_FACTOR, MAX_FACTOR]`. With fewer than two measured domains
    /// the result is identity (see the module doc).
    #[must_use]
    pub fn from_rows(rows: &[Row]) -> Self {
        // (canonical domain, raw factor) — canonicalized AT INSERT so a
        // row keyed on a router spelling still lands on the one key both
        // halves of the column share.
        let measured: Vec<(String, f32)> = rows
            .iter()
            .filter(|r| r.qualifies())
            .filter_map(|r| {
                r.constant().map(|c| {
                    let raw = MIN_FACTOR + (MAX_FACTOR - MIN_FACTOR) * c;
                    (canonical_domain(&r.domain).to_string(), raw)
                })
            })
            .collect();
        if measured.len() < 2 {
            // One measured domain normalizes to 1.0 by arithmetic; zero
            // has nothing to scale. Both are identity — recorded, not
            // special-cased away, so the invariant stays visible.
            return Self::identity();
        }
        let mean = measured.iter().map(|(_, raw)| raw).sum::<f32>() / measured.len() as f32;
        let factors = measured
            .into_iter()
            .map(|(domain, raw)| (domain, (raw / mean).clamp(MIN_FACTOR, MAX_FACTOR)))
            .collect();
        Self { factors }
    }

    /// The factor for a domain, taking either spelling (router label or
    /// canonical id — translated through [`canonical_domain`], the one
    /// translation point). Unmeasured domains read exactly `1.0`.
    #[must_use]
    pub fn factor(&self, domain_label: &str) -> f32 {
        self.factors
            .get(canonical_domain(domain_label))
            .copied()
            .unwrap_or(1.0)
    }

    /// Scale one depth choice (pool or K) for a domain. Identity factors
    /// return `base` EXACTLY (the byte-identity fast path); non-identity
    /// results are clamped to `[1, base·2]` — a depth is never zeroed and
    /// never more than doubled.
    #[must_use]
    pub fn scale(&self, domain_label: &str, base: usize) -> usize {
        let f = self.factor(domain_label);
        if (f - 1.0).abs() < 1e-6 {
            return base;
        }
        let scaled = (base as f32 * f).round();
        let ceiling = base.saturating_mul(2).max(1);
        (scaled.clamp(1.0, ceiling as f32)) as usize
    }

    /// True when no domain moves (empty map, or every factor at 1.0) —
    /// the caller's "do not arm" signal.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        self.factors.values().all(|f| (f - 1.0).abs() < 1e-6)
    }

    /// One-line debug render (the arming line's payload).
    #[must_use]
    pub fn render(&self) -> String {
        if self.is_identity() {
            return "identity (no reallocation)".to_string();
        }
        self.factors
            .iter()
            .map(|(d, f)| format!("{d}×{f:.3}"))
            .collect::<Vec<_>>()
            .join("  ")
    }

    /// The AUTO-arm path: read the standing constants from the healer
    /// repo's score history and build the policy. `None` when the
    /// kill-switch is set, the file is absent/unreadable, no row carries
    /// the field yet, or the built policy is identity (nothing to arm).
    ///
    /// Read errors degrade LOUDLY (one stderr line, the
    /// `active-set`-precedent posture): a gate that silently fails to arm
    /// is indistinguishable from one that measured nothing.
    #[must_use]
    pub fn standing(repo_root: &Path) -> Option<Self> {
        if env_kills(std::env::var(KILL_SWITCH_ENV).ok().as_deref()) {
            return None;
        }
        let path = repo_root.join(STANDING_RELPATH);
        match crate::demonstrability::standing_from_jsonl(&path) {
            Ok(Some((_, rows))) => {
                let policy = Self::from_rows(&rows);
                // Arm ONLY a non-identity policy — an identity result (fewer
                // than two measured domains, or uniform constants) leaves
                // the run at the default posture and the caller silent.
                (!policy.is_identity()).then_some(policy)
            }
            Ok(None) => None,
            Err(e) => {
                eprintln!(
                    "demonstrability gate: standing history at {} unreadable ({e}) — \
                     continuing at identity (no reallocation)",
                    path.display()
                );
                None
            }
        }
    }
}

/// The kill-switch predicate, pure over its input for testability: only
/// the exact literal `"0"` disarms.
#[must_use]
pub fn env_kills(value: Option<&str>) -> bool {
    value == Some("0")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demonstrability::{bench_row, Source};

    fn row(domain: &str, selected: u32, rejected: u32) -> Row {
        Row {
            domain: domain.to_string(),
            rule: None,
            source: Source::ScoreBench,
            selected,
            rejected,
            selected_partial: 0,
            excluded_generation: 0,
        }
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        // The shared-temp-path law: never a FIXED temp filename — a
        // concurrent process would truncate it mid-test.
        std::env::temp_dir().join(format!(
            "riir_demonstrability_gate_{}_{}",
            std::process::id(),
            name
        ))
    }

    #[test]
    fn identity_policy_scales_nothing() {
        let p = DepthPolicy::identity();
        for label in ["clippy", "perf", "kernel", "rust_perf", "unknown"] {
            assert!((p.factor(label) - 1.0).abs() < 1e-6, "{label}");
        }
        // Exact passthrough — byte-identity requires ==, not approximately.
        for base in [0usize, 1, 3, 28, 100] {
            assert_eq!(p.scale("clippy", base), base);
        }
        assert!(p.is_identity());
    }

    #[test]
    fn from_rows_ignores_non_qualifying_rows() {
        let mut per_rule = row("clippy", 90, 10);
        per_rule.rule = Some("needless_return".to_string());
        let rows = [
            per_rule,
            row("other", 50, 50),
            row("docker", 0, 0), // empty labeled set
        ];
        assert!(DepthPolicy::from_rows(&rows).is_identity());
    }

    #[test]
    fn single_measured_domain_is_identity() {
        // One measured domain: nothing to reallocate against (module doc).
        let p = DepthPolicy::from_rows(&[row("clippy", 935, 65)]);
        assert!(p.is_identity());
    }

    #[test]
    fn two_measured_domains_normalize_to_unit_mean() {
        // constants: clippy 0.2, rust_perf 1.0 → raw 0.8 / 2.0, mean 1.4.
        let p = DepthPolicy::from_rows(&[row("clippy", 20, 80), row("rust_perf", 100, 0)]);
        let f_clippy = p.factor("clippy");
        let f_perf = p.factor("rust_perf");
        assert!(f_clippy < 1.0 && f_perf > 1.0);
        // Unit mean over the measured set (budget conservation).
        assert!((f_clippy + f_perf - 2.0).abs() < 1e-5);
        // Unmeasured domains stay at exactly 1.0 — never punished.
        assert!((p.factor("docker") - 1.0).abs() < 1e-6);
        assert!(!p.is_identity());
    }

    #[test]
    fn spread_constants_shallow_and_deepen() {
        let p = DepthPolicy::from_rows(&[row("clippy", 20, 80), row("rust_perf", 100, 0)]);
        // raw 0.8/2.0, mean 1.4 → factors 4/7 ≈ 0.5714 and 10/7 ≈ 1.4286.
        // pool 28 → 16 / 40; K 3 → round(1.71)=2 / round(4.29)=4.
        assert_eq!(p.scale("clippy", 28), 16);
        assert_eq!(p.scale("rust_perf", 28), 40);
        assert_eq!(p.scale("clippy", 3), 2);
        assert_eq!(p.scale("rust_perf", 3), 4);
    }

    #[test]
    fn standing_constants_round_to_inert() {
        // The honest inertness pin: the T1 live readings (clippy 0.935,
        // rust_perf 0.962) give factors 0.990/1.011 — pool 28 → 28, K 3 → 3.
        // The gate arms only when domains DIVERGE; two high nearly-equal
        // constants mean no reallocation.
        let p = DepthPolicy::from_rows(&[row("clippy", 935, 65), row("rust_perf", 962, 38)]);
        assert!(!p.is_identity(), "factors differ — armed but inert:");
        assert_eq!(p.scale("clippy", 28), 28);
        assert_eq!(p.scale("rust_perf", 28), 28);
        assert_eq!(p.scale("clippy", 3), 3);
        assert_eq!(p.scale("rust_perf", 3), 3);
    }

    #[test]
    fn factor_translates_router_labels() {
        let p = DepthPolicy::from_rows(&[row("clippy", 20, 80), row("rust_perf", 100, 0)]);
        // Compared through `scale` (usize) — the factors themselves are f32
        // and direct == on them trips the pedantic float_cmp lint; equal
        // SCALING is the assertion that matters (the translation is exact
        // iff the scaled depths agree for a non-identity factor).
        assert_eq!(p.scale("perf", 28), p.scale("rust_perf", 28));
        assert_eq!(p.scale("kernel", 28), p.scale("kernel_opt", 28));
        assert_eq!(p.scale("shader", 28), p.scale("shader_perf", 28));
        // And the untranslated pair actually differs from the untranslated
        // clippy pair (the policy is non-identity — the equalities above are
        // meaningful, not vacuous).
        assert_ne!(p.scale("perf", 28), p.scale("clippy", 28));
    }

    #[test]
    fn clamp_bounds_hold_after_normalization() {
        // 0.0 / 1.0: raw 0.5 / 2.0, mean 1.25 → 0.4 (clamped to 0.5) /
        // 1.6. The clamp is the second line of defense, this pins it.
        let p = DepthPolicy::from_rows(&[row("clippy", 0, 100), row("rust_perf", 100, 0)]);
        for f in [p.factor("clippy"), p.factor("rust_perf")] {
            assert!(
                (MIN_FACTOR..=MAX_FACTOR).contains(&f),
                "factor {f} outside [{MIN_FACTOR},{MAX_FACTOR}]"
            );
        }
        assert!((p.factor("clippy") - MIN_FACTOR).abs() < 1e-6);
    }

    #[test]
    fn scale_never_zeroes_non_identity() {
        let p = DepthPolicy::from_rows(&[row("clippy", 0, 100), row("rust_perf", 100, 0)]);
        for base in [1usize, 2, 3, 28] {
            assert!(p.scale("clippy", base) >= 1);
        }
        // Double ceiling: factor <= 2.0 → scale <= 2*base.
        assert!(p.scale("rust_perf", 28) <= 56);
    }

    #[test]
    fn bench_row_rows_feed_from_rows_directly() {
        // The T1 producer shape (bench_row) flows through unchanged.
        let rows = [
            bench_row("clippy", None, 20, 80, 0, 0),
            bench_row("rust_perf", None, 100, 0, 0, 0),
        ];
        let p = DepthPolicy::from_rows(&rows);
        assert_eq!(p.scale("clippy", 28), 16);
    }

    #[test]
    fn env_kills_only_on_exact_zero() {
        assert!(env_kills(Some("0")));
        assert!(!env_kills(Some("1")));
        assert!(!env_kills(Some("false")));
        assert!(!env_kills(Some("00")));
        assert!(!env_kills(None));
    }

    #[test]
    fn standing_reads_jsonl_and_respects_the_kill_switch() {
        let rows = serde_json::to_string(&[row("clippy", 20, 80), row("rust_perf", 100, 0)])
            .expect("serialize rows");
        let line = format!("{{\"ts\":\"2026-09-24T00:00:00Z\",\"demonstrability\":{rows}}}");
        let tmp_root = temp_path("standing_root");
        let _ = std::fs::create_dir_all(tmp_root.join(".benchmarks"));
        std::fs::write(tmp_root.join(STANDING_RELPATH), format!("{line}\n"))
            .expect("write temp jsonl");

        // Kill-switch set → None, BEFORE any file read matters.
        std::env::set_var(KILL_SWITCH_ENV, "0");
        assert!(DepthPolicy::standing(&tmp_root).is_none());
        std::env::remove_var(KILL_SWITCH_ENV);

        // Armed: the two measured, diverging constants build a policy.
        let armed = DepthPolicy::standing(&tmp_root);
        assert!(armed.is_some(), "diverging constants must arm");
        assert_eq!(armed.unwrap().scale("clippy", 28), 16);

        // Absent file → None (not yet measured), quietly.
        let empty_root = temp_path("standing_absent");
        let _ = std::fs::create_dir_all(&empty_root);
        assert!(DepthPolicy::standing(&empty_root).is_none());

        let _ = std::fs::remove_file(tmp_root.join(STANDING_RELPATH));
        let _ = std::fs::remove_dir(tmp_root.join(".benchmarks"));
        let _ = std::fs::remove_dir(&tmp_root);
        let _ = std::fs::remove_dir(&empty_root);
    }

    #[test]
    fn standing_returns_none_when_the_policy_would_be_identity() {
        // One measured row → identity → standing() declines to arm (the
        // caller's arming line would be a lie).
        let rows = serde_json::to_string(&[row("clippy", 935, 65)]).expect("serialize rows");
        let line = format!("{{\"ts\":\"t\",\"demonstrability\":{rows}}}");
        let tmp_root = temp_path("standing_identity_root");
        let _ = std::fs::create_dir_all(tmp_root.join(".benchmarks"));
        std::fs::write(tmp_root.join(STANDING_RELPATH), format!("{line}\n"))
            .expect("write temp jsonl");

        assert!(DepthPolicy::standing(&tmp_root).is_none());

        let _ = std::fs::remove_file(tmp_root.join(STANDING_RELPATH));
        let _ = std::fs::remove_dir(tmp_root.join(".benchmarks"));
        let _ = std::fs::remove_dir(&tmp_root);
    }

    #[test]
    fn render_names_the_factors() {
        let p = DepthPolicy::from_rows(&[row("clippy", 20, 80), row("rust_perf", 100, 0)]);
        let r = p.render();
        assert!(r.contains("clippy") && r.contains("rust_perf"));
        assert_eq!(
            DepthPolicy::identity().render(),
            "identity (no reallocation)"
        );
    }
}

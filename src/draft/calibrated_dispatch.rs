//! Issue 233 T3 — the calibrated acquisition-config gate for the
//! span-shape dispatch.
//!
//! The promoted dispatch ([`super::latent_matcher::span_shape`]) narrows
//! `rust_perf` rules by hand-registered shape annotations. This module gates
//! the SAME retain on measured evidence: per (domain × span-shape) group,
//! narrowing applies only where oracle-labeled evidence clears the floor —
//! Wilson 95% lower confidence bound ≥ [`TAU`] over ≥ [`MIN_LABELED`]
//! labeled spans. Every other group falls back to the FULL fan-out (the
//! paper's law: a config under the floor is never silently relaxed), and
//! groups below [`MIN_LABELED`] use the default (no dispatch — the small-
//! group rule).
//!
//! Postures:
//! - feature OFF: this module does not compile; the promoted hand-registered
//!   dispatch stands byte-identical.
//! - feature ON with the pinned [`EVIDENCE_GROUPS`]: narrowing arms only on
//!   Perf spans whose EVERY carried shape bit is [`GroupVerdict::Calibrated`]
//!   — the conservative reading of the group law (one Sparse/Failing group
//!   on a span falls the whole span back to the full fan-out; the dispatch
//!   narrows, it never widens). At the 2026-11-14 landing that is exactly
//!   the {`FnDecl`, `HotLoop`} spans; `TypeDecl` (n=10, Sparse), `ImplBlock`,
//!   `TraitDecl` and every unevidenced shape keep the full fan-out — the
//!   measured evidence DEMOTES the unproven narrowing, it never assumes it.
//!
//! Evidence provenance: [`EVIDENCE_GROUPS`] rows join ONLY with oracle-
//! labeled census output (`examples/calibrated_dispatch_census.rs` — the
//! re-run lane) and a dated provenance note. A row never joins run-to-pass:
//! the census prints the table, this file pins it, and the GOAT gate
//! (recall@k ≥ full at the floor + the G2 cost leg) owns any promotion.

use crate::domain::{SpanShape, SpanShapeSet};
use crate::draft::DomainId;

/// The paper's fixed pre-evaluation floor (Issue 233 T3: tau = 0.6 first).
pub const TAU: f64 = 0.6;

/// The small-group rule: fewer labeled spans than this use the default
/// (no dispatch) — the evidence cannot distinguish the group from noise.
pub const MIN_LABELED: usize = 20;

/// The Wilson 95% two-sided z (the issue pins the 95% lower end).
pub const WILSON_Z: f64 = 1.959_964;

/// Raw labeled evidence for ONE (domain × span-shape) group: `labeled`
/// oracle-labeled spans classified into the group, `successes` where the
/// narrowed posture still served the gold rule in top-k. The pinned table
/// carries the raw counts (never the derived verdict) so a floor change
/// re-derives without re-mining.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupStats {
    pub labeled: u32,
    pub successes: u32,
}

/// The derived per-group verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupVerdict {
    /// `labeled >= MIN_LABELED` and the Wilson LCB clears [`TAU`] — the
    /// narrowing may arm for spans carrying this shape.
    Calibrated,
    /// `labeled >= MIN_LABELED` and the LCB is under [`TAU`] — a MEASURED
    /// refutation: the group falls back to the full fan-out.
    Fallback,
    /// `labeled < MIN_LABELED` (or no row at all) — the small-group rule:
    /// default, no dispatch.
    Sparse,
}

/// The Wilson score 95% lower bound on the success rate. `None` on an
/// empty group (nothing to bound).
#[must_use]
pub fn wilson_lcb(successes: u32, labeled: u32) -> Option<f64> {
    if labeled == 0 {
        return None;
    }
    debug_assert!(
        successes <= labeled,
        "wilson_lcb: {successes} successes over {labeled} labeled spans — caller bug"
    );
    let n = f64::from(labeled);
    let p = f64::from(successes) / n;
    let z2 = WILSON_Z * WILSON_Z;
    let center = p + z2 / (2.0 * n);
    let spread = WILSON_Z * ((p * (1.0 - p) / n) + (z2 / (4.0 * n * n))).sqrt();
    let denom = 1.0 + z2 / n;
    Some((center - spread) / denom)
}

/// Derive the group verdict from raw counts: the small-group rule first,
/// then the floor on the Wilson LCB.
#[must_use]
pub fn verdict_of(stats: GroupStats) -> GroupVerdict {
    if (stats.labeled as usize) < MIN_LABELED {
        return GroupVerdict::Sparse;
    }
    match wilson_lcb(stats.successes, stats.labeled) {
        Some(lcb) if lcb >= TAU => GroupVerdict::Calibrated,
        _ => GroupVerdict::Fallback,
    }
}

/// Every categorical shape, in declaration order — the per-bit walk the
/// consult folds over. `SpanShapeSet` deliberately carries no bits iterator
/// (the hand-rolled flags type grows only at a registration), so the fold
/// tests each bit by `intersects`.
const ALL_SHAPES: [SpanShape; 10] = [
    SpanShape::FnDecl,
    SpanShape::TestFn,
    SpanShape::AsyncFn,
    SpanShape::ImplBlock,
    SpanShape::TraitDecl,
    SpanShape::TypeDecl,
    SpanShape::ModBlock,
    SpanShape::MacroItem,
    SpanShape::HotLoop,
    SpanShape::MatchBlock,
];

fn carries(verdict: SpanShapeSet, shape: SpanShape) -> bool {
    verdict.intersects(SpanShapeSet::empty().with(shape))
}

/// The pinned evidence table (oracle-labeled census provenance per row —
/// never paste a row run-to-pass: re-run
/// `examples/calibrated_dispatch_census.rs` first, and the GOAT gate —
/// recall@k ≥ full at the floor + the G2 cost leg — owns any widening,
/// per the Issue 216 registration discipline).
///
/// Census 2026-11-14 (amo, zed-amo-i233-t3; the static eval fixtures at the
/// dev posture — 180 labeled spans, 131 grouped; counts are posture-stable,
/// deterministic fixtures + deterministic retrieval): perf `FnDecl` 49/49
/// narrowed-posture successes (Wilson 95% LCB 0.927) and perf `HotLoop` 24/24
/// (LCB 0.862), both clearing tau = 0.6. GOAT recall half PASS on the cell
/// (46/46 Perf spans narrowed, zero full→narrowed recall losses); the G2
/// cost leg (quiet-box window) still owns promotion — the feature stays
/// opt-in, never in the release set until it passes. perf `TypeDecl` measured
/// n=10 → Sparse (below the small-group rule): the three `TYPE_DECL_ONLY`
/// annotations stay UNNARROWED under this feature until real evidence lands.
static EVIDENCE_GROUPS: &[(DomainId, SpanShape, GroupStats)] = &[
    (
        DomainId::Perf,
        SpanShape::FnDecl,
        GroupStats {
            labeled: 49,
            successes: 49,
        },
    ),
    (
        DomainId::Perf,
        SpanShape::HotLoop,
        GroupStats {
            labeled: 24,
            successes: 24,
        },
    ),
];

/// The consult the dispatch retain reads: narrowing applies to `verdict`
/// only when EVERY shape bit the span carries is [`GroupVerdict::Calibrated`]
/// in `rows`. Fail-open by contract:
/// - `ALL` (parse skipped/failed) and `empty` never narrow — the caller's
///   own guard keeps `ALL` out; `empty` is refused here defensively (an
///   empty verdict would drop every annotated rule — narrowing hard).
/// - an absent row is [`GroupVerdict::Sparse`] — no evidence, no dispatch.
#[must_use]
pub fn narrowing_applies_with(
    rows: &[(DomainId, SpanShape, GroupStats)],
    verdict: SpanShapeSet,
) -> bool {
    if verdict == SpanShapeSet::ALL || verdict == SpanShapeSet::empty() {
        return false;
    }
    ALL_SHAPES.iter().all(|&shape| {
        if !carries(verdict, shape) {
            return true;
        }
        rows.iter()
            .find(|(d, s, _)| *d == DomainId::Perf && *s == shape)
            .is_some_and(|(_, _, stats)| verdict_of(*stats) == GroupVerdict::Calibrated)
    })
}

/// The runtime consult over the pinned [`EVIDENCE_GROUPS`] table.
#[must_use]
pub fn narrowing_applies(verdict: SpanShapeSet) -> bool {
    narrowing_applies_with(EVIDENCE_GROUPS, verdict)
}

#[cfg(all(test, feature = "calibrated_dispatch"))]
mod tests {
    use super::{
        GroupStats, GroupVerdict, MIN_LABELED, TAU, WILSON_Z, narrowing_applies_with, verdict_of,
        wilson_lcb,
    };
    use crate::domain::{SpanShape, SpanShapeSet};
    use crate::draft::DomainId;

    const fn stats(labeled: u32, successes: u32) -> GroupStats {
        GroupStats { labeled, successes }
    }

    const fn set(bits: &[SpanShape]) -> SpanShapeSet {
        let mut acc = SpanShapeSet::empty();
        let mut i = 0;
        while i < bits.len() {
            acc = acc.with(bits[i]);
            i += 1;
        }
        acc
    }

    /// Hand-computed Wilson 95% LCB values (z = 1.959964, independent
    /// arithmetic — the test pins the FORMULA against outside math, never
    /// against its own output).
    #[test]
    fn wilson_lcb_matches_hand_computed_pins() {
        let cases: &[(u32, u32, f64)] = &[
            (20, 20, 0.8389),
            (19, 20, 0.7639),
            (17, 20, 0.6395),
            (16, 20, 0.5840),
            (12, 20, 0.3866),
        ];
        for &(succ, n, expected) in cases {
            let lcb = wilson_lcb(succ, n).expect("nonempty group");
            assert!(
                (lcb - expected).abs() < 1.0e-3,
                "wilson_lcb({succ}, {n}) = {lcb}, expected ~{expected}"
            );
        }
        assert_eq!(wilson_lcb(0, 0), None);
    }

    /// The floor semantics at the small-group boundary: at n = 20 the 95%
    /// LCB demands ~85% observed success (0.6 raw does NOT clear), and
    /// n < 20 is Sparse regardless of a perfect rate.
    #[test]
    fn verdict_pins_the_floor_and_the_small_group_rule() {
        assert_eq!(verdict_of(stats(20, 17)), GroupVerdict::Calibrated);
        assert_eq!(verdict_of(stats(20, 16)), GroupVerdict::Fallback);
        assert_eq!(verdict_of(stats(20, 12)), GroupVerdict::Fallback);
        assert_eq!(verdict_of(stats(19, 19)), GroupVerdict::Sparse);
        assert_eq!(verdict_of(stats(0, 0)), GroupVerdict::Sparse);
        assert_eq!(
            verdict_of(stats(u32::try_from(MIN_LABELED).expect("const fits"), 12)),
            GroupVerdict::Fallback
        );
        // the floor constant is the paper's first value, pinned
        assert!((TAU - 0.6).abs() < f64::EPSILON);
        // z is the two-sided 95% normal quantile
        assert!((WILSON_Z - 1.959_964).abs() < 1.0e-6);
    }

    /// The consult: fail-open on ALL/empty and on any absent (Sparse) bit;
    /// conservative per-bit — one non-Calibrated group on a span falls the
    /// whole span back to the full fan-out.
    #[test]
    fn narrowing_applies_is_conservative_per_bit_and_fail_open() {
        let fn_only = set(&[SpanShape::FnDecl]);
        let fn_hot = set(&[SpanShape::FnDecl, SpanShape::HotLoop]);
        let all = SpanShapeSet::ALL;

        // no rows: nothing narrows (the landing posture).
        assert!(!narrowing_applies_with(&[], fn_only));
        assert!(!narrowing_applies_with(&[], fn_hot));
        // the caller guards ALL, but the consult refuses it too (and empty).
        let calibrated_fn = &[(DomainId::Perf, SpanShape::FnDecl, stats(40, 40))];
        assert!(!narrowing_applies_with(calibrated_fn, all));
        assert!(!narrowing_applies_with(
            calibrated_fn,
            SpanShapeSet::empty()
        ));

        // a calibrated FnDecl arms a FnDecl-only span...
        assert!(narrowing_applies_with(calibrated_fn, fn_only));
        // ...but a span ALSO carrying an unevidenced HotLoop falls back.
        assert!(!narrowing_applies_with(calibrated_fn, fn_hot));

        // a measured Fallback row never arms, even with a perfect co-bit.
        let fallback_fn = &[(DomainId::Perf, SpanShape::FnDecl, stats(40, 24))];
        assert_eq!(verdict_of(fallback_fn[0].2), GroupVerdict::Fallback);
        assert!(!narrowing_applies_with(fallback_fn, fn_only));

        // both bits calibrated arms the multi-bit span.
        let both = &[
            (DomainId::Perf, SpanShape::FnDecl, stats(40, 40)),
            (DomainId::Perf, SpanShape::HotLoop, stats(25, 24)),
        ];
        assert!(narrowing_applies_with(both, fn_hot));

        // a Sparse row (under the small-group floor) does not arm.
        let sparse_fn = &[(DomainId::Perf, SpanShape::FnDecl, stats(10, 10))];
        assert!(!narrowing_applies_with(sparse_fn, fn_only));

        // non-Perf rows are inert (the dispatch only narrows Perf).
        let clippy_row = &[(DomainId::Clippy, SpanShape::FnDecl, stats(40, 40))];
        assert!(!narrowing_applies_with(clippy_row, fn_only));
    }

    /// The PINNED table's posture: the runtime consult arms exactly the
    /// calibrated groups ({`FnDecl`, `HotLoop`}) and nothing else —
    /// `TypeDecl` is measured Sparse (n=10), so the `TYPE_DECL_ONLY`
    /// annotations stay unnarrowed under this feature. A census re-run that
    /// moves a group's verdict updates this test WITH the table (same
    /// family).
    #[test]
    fn pinned_table_arms_exactly_the_calibrated_groups() {
        use super::narrowing_applies;
        let fn_only = set(&[SpanShape::FnDecl]);
        let fn_hot = set(&[SpanShape::FnDecl, SpanShape::HotLoop]);
        let type_only = set(&[SpanShape::TypeDecl]);
        let impl_only = set(&[SpanShape::ImplBlock]);
        let fn_type = set(&[SpanShape::FnDecl, SpanShape::TypeDecl]);
        assert!(narrowing_applies(fn_only));
        assert!(narrowing_applies(fn_hot));
        assert!(!narrowing_applies(type_only));
        assert!(!narrowing_applies(impl_only));
        // one Sparse bit on a span falls the WHOLE span back to full.
        assert!(!narrowing_applies(fn_type));
        assert!(!narrowing_applies(SpanShapeSet::ALL));
        assert!(!narrowing_applies(SpanShapeSet::empty()));
    }
}

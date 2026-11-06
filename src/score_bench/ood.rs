//! Entropy-gap OOD axis (Issue 077 T1+T4) — the consumer half of
//! katgpt-rs Issue 740's [`katgpt_core::regime_probe`] primitives.
//!
//! One regime question per score-bench process: **does the rule substrate
//! respond to held-out code the same way it responds to its home ground?**
//! The substrate here is the proposer rule table (the SAME per-rule
//! `collect_edits` calls [`super::heal_fixpoint_with`] makes on pass one).
//! For every fixture span, the per-rule edit counts form a score vector over
//! the rule vocabulary; the `regime_probe` entropy kernel reads it as logits
//! and emits the rule-selection entropy in nats — sharp firing (one rule
//! dominates) → low entropy, diffuse firing (no rule stands out) → high
//! entropy.
//!
//! Two sets, both frozen in-tree:
//!
//! | set | source | role |
//! |---|---|---|
//! | `v1_frozen` | [`super::fixtures::fixtures`] (frozen 2026-08-19, the corpus-era home ground) | reference |
//! | `gen6_real_repo` | [`super::generations::generation_v6`] (real spans carved from pre-heal sibling states, frozen 2026-09-06 — mined AFTER the v1 freeze; held-out by construction) | OOD |
//!
//! The v2–v5 ladders are corpus-derived composites (in-distribution by
//! construction) and v7 is authored-overlap — neither is an OOD set; gen 6
//! is the only real-code set mined outside the reference era. Zero oracle
//! involvement: both sets are static, so the axis is generation-independent
//! and rides EVERY score-bench row (`HistoryRecord::ood`).
//!
//! Regime semantics follow Bench 702 G1 exactly (gap = `mean_ood − mean_ref`):
//! a **positive** gap ⇒ the substrate fires measurably sharper on seen
//! material than on held-out material — the memorization signature; a gap
//! within ±0.5 nats ⇒ the distributions have converged (the generalized
//! regime); a strongly **negative** gap is not a paper regime at all and is
//! reported as anomalous rather than folded into either neighbor. The
//! secondary overfit reading (corpus so sharp its entropy is near-zero on
//! BOTH sets) is read directly off the two absolute means — see
//! `.docs/08_benchmarks/entropy_gap_ood_axis.md`.
//!
//! Bit-determinism: fixtures are iterated in sorted-id order per set (the
//! artifact contract), the entropy kernel is the shared katgpt-core one, and
//! the BLAKE3 artifact is [`katgpt_core::regime_probe::entropy_gap_into`]'s
//! canonical encoding — no entropy arithmetic lives in this repo (T1).
//!
//! Provenance surface (issue 077, provenance half): each set also reports
//! its distinct provenance FAMILIES — the `repo@sha` prefix of every
//! fixture's `provenance` string ([`provenance_family`]). This is the
//! "which sibling states does each set actually cover" answer that reads
//! the `anomalous_negative` verdict: `v1_frozen` is mostly authored
//! composites plus one katgpt-rs state, while `gen6_real_repo` is carved
//! entirely from pre-heal sibling states — the substrate firing SHARPER on
//! the real-code set is a set-composition fact, not a substrate fact.
//! Deliberately NOT part of the BLAKE3 artifact: the artifact pins the
//! entropy MEASUREMENT (values in sorted-id order); family membership is a
//! static in-tree property of the frozen sets (every fixture row carries
//! its provenance verbatim in source), so folding it into the hash would
//! rotate every future artifact without changing a single measurement — a
//! presentational change masquerading as a new one. The families ride the
//! row as data ([`OodRow::ref_provenance_families`] /
//! [`OodRow::ood_provenance_families`]) and the console report instead.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use katgpt_core::regime_probe::{conditional_entropy_nats, entropy_gap_into, EntropyGapReport};
use serde::{Deserialize, Serialize};

use super::BenchFixture;

/// Bench 702 G1 regime threshold (nats): |gap| above this classifies the
/// run; at or below it the two entropy distributions count as converged.
/// The GOAT separated memorizer from generalizer at 300× this threshold.
pub const GAP_REGIME_THRESHOLD_NATS: f32 = 0.5;

/// The reference set id (the frozen v1 fixture set).
pub const REF_SET: &str = "v1_frozen";
/// The OOD set id (the generation-6 real-repo span set).
pub const OOD_SET: &str = "gen6_real_repo";

/// Regime classification of the entropy-gap axis (Bench 702 G1 semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OodRegime {
    /// gap > +0.5 nats — held-out spans fire measurably more diffusely than
    /// corpus-era spans: the substrate distinguishes seen from unseen
    /// (memorization signature).
    Memorizing,
    /// |gap| ≤ 0.5 nats — the rule-firing distributions have converged; the
    /// substrate treats held-out shapes like home-ground shapes.
    Converged,
    /// gap < −0.5 nats — held-out spans fire MORE sharply than home-ground
    /// spans. Not a paper regime; recorded loudly rather than folded into
    /// either neighbor (an instrument-level flag).
    AnomalousNegative,
    /// Either sample ended up empty (every fixture failed to parse) — no
    /// verdict is classified; the raw gap fields keep the primitive's
    /// one-sided measurement (consistent with the artifact) and the
    /// console report reads the regime, not the bare gap.
    Insufficient,
}

/// One OOD-axis measurement — the additive `ood` field of a
/// score-history JSONL row (Issue 077 T4). Always present on rows written
/// by `score_bench` builds carrying this module; rows predating the axis
/// deserialize with `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OodRow {
    /// The reference set id (see [`REF_SET`]).
    pub ref_set: String,
    /// The OOD set id (see [`OOD_SET`]).
    pub ood_set: String,
    /// Reference fixtures that produced an entropy (parsed + scored).
    pub n_ref: usize,
    /// OOD fixtures that produced an entropy.
    pub n_ood: usize,
    /// Reference fixtures skipped (failed to parse) — loud in the console
    /// report, honest in the row.
    pub n_ref_skipped: usize,
    /// OOD fixtures skipped (failed to parse).
    pub n_ood_skipped: usize,
    /// Mean rule-selection entropy over the reference set (nats).
    pub mean_ref_entropy: f32,
    /// Mean rule-selection entropy over the OOD set (nats).
    pub mean_ood_entropy: f32,
    /// `mean_ood − mean_ref`. Positive ⇒ memorization signature (Bench 702
    /// G1); ≈ 0 ⇒ converged/generalized.
    pub mean_gap: f32,
    /// Cohen's-d-style standardized gap (the primitive's field) — the
    /// scale-free axis for thresholding across vocabulary-size changes.
    pub standardized_gap: f32,
    /// The regime classification ([`GAP_REGIME_THRESHOLD_NATS`]).
    pub regime: OodRegime,
    /// 64-hex BLAKE3 artifact — the primitive's canonical little-endian
    /// encoding over both entropy samples (fixtures id-sorted per set), so
    /// an audit can re-verify the exact measurement behind a verdict.
    /// Provenance families are deliberately NOT hashed in — see the module
    /// doc's provenance-surface note.
    pub artifact: String,
    /// Distinct provenance families of the reference set ([`REF_SET`]) —
    /// sorted `repo@sha` prefixes. Empty on rows written before the
    /// surface landed (`serde(default)` keeps old JSONL rows readable).
    #[serde(default)]
    pub ref_provenance_families: Vec<String>,
    /// Distinct provenance families of the OOD set ([`OOD_SET`]).
    #[serde(default)]
    pub ood_provenance_families: Vec<String>,
}

/// The provenance FAMILY of one fixture — the `repo@sha` prefix, i.e.
/// everything up to the first `:` of the `provenance` string
/// (`katgpt-rs@9e0e6a57`, `authored-composite`, `riir-train@a1197e06~1`).
/// A string with no `:` is its own family.
#[must_use]
pub fn provenance_family(provenance: &str) -> &str {
    match provenance.split_once(':') {
        Some((family, _)) => family,
        None => provenance,
    }
}

/// The OOD set's MINING provenance family for one fixture. The frozen
/// conversion in [`super::generations::generation_v6`] stamps
/// `construction:gen6:…` provenance onto the bench rows (freeze-gate
/// metadata), so the family question reads the real `repo@sha` provenance
/// off the raw span table by id; ids not on that table keep their own
/// fixture provenance.
#[must_use]
pub fn ood_set_family(fixture: &BenchFixture) -> &str {
    for s in super::generations::RAW_SPANS_V6 {
        if s.id == fixture.id {
            return provenance_family(s.provenance);
        }
    }
    provenance_family(fixture.provenance)
}

/// Distinct provenance families of a fixture set, sorted — the set-level
/// coverage summary. Counts (which family contributes how many spans) ride
/// [`family_counts`]; the row carries the distinct names.
#[must_use]
pub fn provenance_families(
    fixture_list: &[BenchFixture],
    family_of: fn(&BenchFixture) -> &str,
) -> Vec<String> {
    let mut families: Vec<&str> = fixture_list.iter().map(family_of).collect();
    families.sort_unstable();
    families.dedup();
    families.into_iter().map(ToString::to_string).collect()
}

/// Per-family span counts of a fixture set — most-contributing first, name
/// asc on ties. The console report's `families:` line.
// Count→ratio-free census over fixture populations (≤ 10³); allowed.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn family_counts(
    fixture_list: &[BenchFixture],
    family_of: fn(&BenchFixture) -> &str,
) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for fx in fixture_list {
        *counts.entry(family_of(fx).to_string()).or_insert(0) += 1;
    }
    let mut pairs: Vec<(String, usize)> = counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    pairs
}

/// Per-span rule score vector over the production fixpoint's rule table —
/// the number of bounded edits each proposer emits on the span, pass one.
///
/// `fxhashmap` is SKIPPED exactly as [`super::heal_fixpoint_with`] skips it
/// (the documented divergence), so the vector mirrors what the production
/// fixpoint lane actually consumes: [`crate::domains::rust_perf::proposers::FIXABLE_RULES`]
/// minus one, then [`crate::domains::clippy_lints::proposers::FIXABLE_LINTS`],
/// in static-table order. `None` = the span does not parse (no vector
/// exists).
#[must_use]
// Count→logit casts: edit counts (< 10³) — far below f32's mantissa;
// allowed with cause.
#[allow(clippy::cast_precision_loss)]
pub fn rule_score_vector(code: &str) -> Option<Vec<f32>> {
    let parsed = syn::parse_file(code).ok()?;
    let mut scores = Vec::with_capacity(
        crate::domains::rust_perf::proposers::FIXABLE_RULES.len()
            + crate::domains::clippy_lints::proposers::FIXABLE_LINTS.len(),
    );
    for rule in crate::domains::rust_perf::proposers::FIXABLE_RULES {
        if *rule == "fxhashmap" {
            continue;
        }
        scores.push(
            crate::domains::rust_perf::proposers::collect_edits(rule, code, &parsed).len() as f32,
        );
    }
    for lint in crate::domains::clippy_lints::proposers::FIXABLE_LINTS {
        scores.push(
            crate::domains::clippy_lints::proposers::collect_edits(lint, code, &parsed).len()
                as f32,
        );
    }
    Some(scores)
}

/// Entropies (nats) for one fixture set, in SORTED-id order — the artifact
/// contract's fixed iteration order. Unparseable fixtures are skipped loud
/// and counted. Offline path (once per process, [`ood_axis`]-cached): the
/// Vec here is the same offline convenience the primitive's own
/// `entropy_gap` wrapper takes; the kernels stay zero-alloc.
fn fixture_entropies(fixture_list: &[BenchFixture], skipped: &mut usize) -> Vec<f32> {
    let mut sorted: Vec<&BenchFixture> = fixture_list.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(b.id));
    let mut ents = Vec::with_capacity(sorted.len());
    for fx in sorted {
        if let Some(v) = rule_score_vector(fx.code) {
            ents.push(conditional_entropy_nats(&v));
        } else {
            *skipped += 1;
            eprintln!("score_bench ood: skipping unparseable fixture {}", fx.id);
        }
    }
    ents
}

/// Assemble the row from two entropy samples (the classification + artifact
/// half — unit-testable without the fixture sets, the Bench 702 G1 shape in
/// miniature).
// Count→ratio casts over fixture counts (< 10⁴); allowed with cause.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn row_from_entropies(
    ref_entropies: &[f32],
    ood_entropies: &[f32],
    skipped: (usize, usize),
) -> OodRow {
    use std::fmt::Write as _;

    let mut report = EntropyGapReport::default();
    entropy_gap_into(ref_entropies, ood_entropies, &mut report);
    let regime = if ref_entropies.is_empty() || ood_entropies.is_empty() {
        OodRegime::Insufficient
    } else if report.mean_gap > GAP_REGIME_THRESHOLD_NATS {
        OodRegime::Memorizing
    } else if report.mean_gap < -GAP_REGIME_THRESHOLD_NATS {
        OodRegime::AnomalousNegative
    } else {
        OodRegime::Converged
    };
    let mut artifact = String::with_capacity(64);
    for b in &report.artifact {
        // Hex into an owned String cannot fail.
        let _ = write!(artifact, "{b:02x}");
    }
    OodRow {
        ref_set: REF_SET.to_string(),
        ood_set: OOD_SET.to_string(),
        n_ref: ref_entropies.len(),
        n_ood: ood_entropies.len(),
        n_ref_skipped: skipped.0,
        n_ood_skipped: skipped.1,
        mean_ref_entropy: report.mean_ref,
        mean_ood_entropy: report.mean_gen,
        mean_gap: report.mean_gap,
        standardized_gap: report.standardized_gap,
        regime,
        artifact,
        // The offline miniature constructor leaves the families empty —
        // they are a SET-level property, filled by [`ood_axis`] from the
        // real fixture tables (the samples here may be synthetic).
        ref_provenance_families: Vec::new(),
        ood_provenance_families: Vec::new(),
    }
}

static AXIS: OnceLock<OodRow> = OnceLock::new();

/// The process-wide OOD axis (computed once, cached — the console report and
/// the appended JSONL row share one measurement by construction).
#[must_use]
pub fn ood_axis() -> &'static OodRow {
    AXIS.get_or_init(|| {
        let mut ref_skipped = 0usize;
        let mut ood_skipped = 0usize;
        let ref_set = super::fixtures::fixtures();
        let ood_set = super::generations::generation_v6();
        let ref_ents = fixture_entropies(&ref_set, &mut ref_skipped);
        let ood_ents = fixture_entropies(&ood_set, &mut ood_skipped);
        let mut row = row_from_entropies(&ref_ents, &ood_ents, (ref_skipped, ood_skipped));
        row.ref_provenance_families =
            provenance_families(&ref_set, |fx| provenance_family(fx.provenance));
        row.ood_provenance_families = provenance_families(&ood_set, ood_set_family);
        row
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Near-one-hot logits (one dominant rule) — the memorizer-like shape.
    fn peaked_logits() -> Vec<f32> {
        let mut v = vec![0.0f32; 8];
        v[2] = 40.0;
        v
    }

    /// Uniform logits (no rule stands out) — the diffuse shape.
    fn flat_logits() -> Vec<f32> {
        vec![0.0f32; 8]
    }

    fn sample(kind: &SampleKind, n: usize) -> Vec<f32> {
        let logits = match kind {
            SampleKind::Peaked => peaked_logits(),
            SampleKind::Flat => flat_logits(),
        };
        (0..n).map(|_| conditional_entropy_nats(&logits)).collect()
    }

    enum SampleKind {
        Peaked,
        Flat,
    }

    /// Bench 702 G1 in miniature: a peaked reference (near-zero entropy —
    /// the substrate is decisive on its home ground) vs a flat held-out set
    /// (finite entropy) must classify `Memorizing` with a large positive gap;
    /// mirrored samples must classify `AnomalousNegative`; identical samples
    /// must classify `Converged` with an exactly-zero gap.
    #[test]
    fn g1_miniature_gap_sign_and_classification() {
        // float_cmp allowed: the identical-samples gap is exactly 0.0 by
        // construction (same buffer, same kernel) — the equality IS the pin.
        #![allow(clippy::float_cmp)]
        // Reference peaked (memorized home ground), OOD flat (diffuse on
        // unseen) → gap = ln 8 ≈ +2.08 > 1.0 → Memorizing.
        let r = row_from_entropies(
            &sample(&SampleKind::Peaked, 16),
            &sample(&SampleKind::Flat, 16),
            (0, 0),
        );
        assert!(
            r.mean_gap > 1.0,
            "memorization gap must be large: {}",
            r.mean_gap
        );
        assert!(r.mean_ref_entropy < 0.01);
        assert_eq!(r.regime, OodRegime::Memorizing);
        assert_eq!(r.ref_set, REF_SET);
        assert_eq!(r.ood_set, OOD_SET);

        // Mirrored: held-out sharper than home ground → anomalous, not a
        // paper regime, never folded into Memorizing/Converged.
        let r2 = row_from_entropies(
            &sample(&SampleKind::Flat, 16),
            &sample(&SampleKind::Peaked, 16),
            (0, 0),
        );
        assert!(r2.mean_gap < -1.0);
        assert_eq!(r2.regime, OodRegime::AnomalousNegative);

        // Identical distributions → converged, exact zero gap, d = 0.
        let same = sample(&SampleKind::Flat, 16);
        let r3 = row_from_entropies(&same, &same, (0, 0));
        assert_eq!(r3.mean_gap, 0.0);
        assert_eq!(r3.standardized_gap, 0.0);
        assert_eq!(r3.regime, OodRegime::Converged);
    }

    /// Threshold edges stay off the exact ±0.5 boundary (no float flakes):
    /// ±0.4 nats converges, ±0.6 nats classifies.
    #[test]
    fn regime_threshold_edges() {
        // cast_possible_truncation allowed: i ≤ 7 · 0.1 is exact fixture
        // arithmetic; float_cmp allowed: the regime boundary is a >=
        // comparison the test pins from BOTH sides (no boundary value).
        #![allow(clippy::cast_possible_truncation, clippy::float_cmp)]
        let tenth: Vec<f32> = (0..8_u16).map(|i| f32::from(i) * 0.1).collect();
        let mk = |delta: f32| {
            let b: Vec<f32> = tenth.iter().map(|&x| x + delta).collect();
            row_from_entropies(&tenth, &b, (0, 0))
        };
        assert_eq!(mk(0.4).regime, OodRegime::Converged);
        assert_eq!(mk(0.6).regime, OodRegime::Memorizing);
        assert_eq!(mk(-0.6).regime, OodRegime::AnomalousNegative);
    }

    /// An empty sample is `Insufficient` — no verdict is classified over no
    /// data. The raw gap fields keep the primitive's one-sided measurement
    /// (`mean_ref` of an empty sample is 0.0) so the row stays consistent
    /// with its own artifact — never a fabricated zero.
    #[test]
    fn empty_sample_is_insufficient() {
        // float_cmp allowed: the empty-sample mean is EXACTLY 0.0 (the
        // primitive's documented convention, not a rounded value) and the
        // one-sided gap equals the ood mean by that same construction.
        #![allow(clippy::float_cmp)]
        let r = row_from_entropies(&[], &sample(&SampleKind::Flat, 4), (3, 0));
        assert_eq!(r.regime, OodRegime::Insufficient);
        assert_eq!(
            r.mean_ref_entropy, 0.0,
            "empty-sample mean is 0.0 (the primitive's convention)"
        );
        assert_eq!(
            r.mean_gap, r.mean_ood_entropy,
            "one-sided measurement, artifact-consistent"
        );
        assert_eq!(r.n_ref, 0);
        assert_eq!(r.n_ref_skipped, 3);
    }

    /// Same inputs → bit-identical artifacts; different inputs → different
    /// artifacts (the audit can re-verify the exact measurement).
    #[test]
    fn artifact_determinism() {
        let a = sample(&SampleKind::Peaked, 8);
        let b = sample(&SampleKind::Flat, 8);
        let r1 = row_from_entropies(&a, &b, (0, 0));
        let r2 = row_from_entropies(&a, &b, (0, 0));
        assert_eq!(r1.artifact, r2.artifact);
        assert_eq!(r1.artifact.len(), 64, "32-byte BLAKE3 → 64 hex");
        let c: Vec<f32> = b.iter().map(|x| x + 1.0).collect();
        let r3 = row_from_entropies(&a, &c, (0, 0));
        assert_ne!(r1.artifact, r3.artifact);
    }

    /// The score vector: right width (the fixpoint's table minus the
    /// skipped `fxhashmap`), deterministic, positive on a known-healable
    /// span, `None` on garbage.
    #[test]
    fn rule_score_vector_shape_and_known_firing() {
        let expected_len = crate::domains::rust_perf::proposers::FIXABLE_RULES.len()
            + crate::domains::clippy_lints::proposers::FIXABLE_LINTS.len()
            - 1; // fxhashmap skipped, mirroring the fixpoint
        let code = "pub fn greet(name: &str) -> String {\n    format!(\"{}\", name)\n}\n";
        let v = rule_score_vector(code).expect("known-parseable span");
        assert_eq!(v.len(), expected_len);
        let v2 = rule_score_vector(code).expect("second run");
        assert_eq!(v, v2, "deterministic");
        assert!(v.iter().sum::<f32>() > 0.0, "uninlined_format_args fires");
        // The fired rule is the decisive entry (the peaked shape the axis
        // reads as low entropy).
        let max = v.iter().copied().fold(0.0f32, f32::max);
        assert!(max >= 1.0, "at least one rule fired: {v:?}");
        assert!(rule_score_vector("this is not rust").is_none());
    }

    /// The full pipeline over the in-tree sets: no skips (both sets are
    /// parse-tested), counts reconcile, regime consistent with the row's
    /// own gap, artifact stable across a recomputation.
    #[test]
    fn ood_axis_over_the_real_sets_reconciles() {
        let axis = ood_axis();
        assert_eq!(
            axis.n_ref + axis.n_ref_skipped,
            super::super::fixtures::fixtures().len()
        );
        assert_eq!(
            axis.n_ood + axis.n_ood_skipped,
            super::super::generations::generation_v6().len()
        );
        assert_eq!((axis.n_ref_skipped, axis.n_ood_skipped), (0, 0));
        assert_eq!(axis.regime, {
            let g = axis.mean_gap;
            if g > GAP_REGIME_THRESHOLD_NATS {
                OodRegime::Memorizing
            } else if g < -GAP_REGIME_THRESHOLD_NATS {
                OodRegime::AnomalousNegative
            } else {
                OodRegime::Converged
            }
        });
        // Cached → the same measurement every reader in the process sees.
        assert_eq!(ood_axis(), axis);
    }

    /// The `repo@sha` prefix is the family — everything up to the first
    /// `:`, including the `~1` sweep-revision suffix; a string with no `:`
    /// is its own family.
    #[test]
    fn provenance_family_parses_the_repo_at_sha_prefix() {
        assert_eq!(
            provenance_family("katgpt-rs@9e0e6a57:bench_ptrm_width_scaling.rs"),
            "katgpt-rs@9e0e6a57"
        );
        assert_eq!(
            provenance_family("riir-train@a1197e06~1:crates/riir-train-engine/src/svr.rs"),
            "riir-train@a1197e06~1"
        );
        assert_eq!(
            provenance_family("authored-composite:T3.5.5:return_plus_identity_op"),
            "authored-composite"
        );
        assert_eq!(provenance_family("no-colon-string"), "no-colon-string");
    }

    /// The real axis row carries each set's distinct families: v1 is the
    /// authored-composite set plus its katgpt-rs mining state; gen6 is
    /// carved entirely from real pre-heal sibling states. The overlap at
    /// the REPO level (katgpt-rs appears in both, different shas) with the
    /// composition difference (authored vs all-real) is the documented
    /// `anomalous_negative` reading.
    #[test]
    fn ood_axis_row_carries_provenance_families() {
        let axis = ood_axis();
        assert_ne!(axis.ref_provenance_families, [] as [String; 0]);
        assert_ne!(axis.ood_provenance_families, [] as [String; 0]);
        // Sorted + distinct by construction.
        let mut sorted = axis.ref_provenance_families.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted, axis.ref_provenance_families);
        // v1: the composite era + exactly one real mining state family.
        assert!(axis
            .ref_provenance_families
            .iter()
            .any(|f| f == "authored-composite"));
        assert_eq!(
            axis.ref_provenance_families
                .iter()
                .filter(|f| f.starts_with("authored"))
                .count(),
            1
        );
        // gen6: no authored family — every span is a real repo@sha carve.
        assert!(
            !axis
                .ood_provenance_families
                .iter()
                .any(|f| f.starts_with("authored")),
            "gen6 must be all-real"
        );
        assert!(axis.ood_provenance_families.iter().all(|f| f.contains('@')));
    }
}

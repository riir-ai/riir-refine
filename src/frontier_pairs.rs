//! Issue 072 T1 — the construction-labeled pair factory (J-Zero M1).
//!
//! Every case in [`crate::domains::rustc_errors::corpus_gen`] plants a known
//! truth: `seed` compiles (and passes its `#[cfg(test)]` behavior test),
//! `broken` fails with `expect_code`, and the repair is the inverse of the
//! planted mutator. This module turns those truths into PREFERENCE PAIRS —
//! (fix reproducing the planted truth) ≻ (fix disagreeing with it) — the
//! soft-judge evidence that densifies the candidate rating pools T39/Research
//! 135 measured as starved (98.5% of candidates unseen; "the bottleneck is
//! evidence volume, not selection math").
//!
//! Construction labeling (the issue's unifying principle): the label comes
//! from the GENERATOR's intent, never from any evaluator output. Zero cargo
//! invocations here — the fence ([`fence_verdicts`]) is the separate,
//! sampled, real-oracle check that catches a buggy generator (the Issue-022
//! corpus-drift class) BEFORE pairs are allowed to feed ratings.
//!
//! ## Pair semantics (resolution pairs)
//!
//! For case `i`, good = case `i`'s own canonical repair (`seed_i`), bad = the
//! canonical repair of a deterministic partner case `j` (cross-code, hash-
//! rotated). The repair approach of `j` does not resolve case `i` — a
//! strategy-shape mismatch — so the pair teaches "matching-inverse ≻
//! non-matching-inverse" at `(error_code, strategy)` granularity, exactly the
//! bucket the `--fix-compile` driver's candidate selection consumes.
//!
//! In-family preference pairs (both arms compile, canonical ≻ non-canonical,
//! e.g. clone-out vs deref-removal on a Copy element) are deliberately NOT
//! emitted by this `PoC`: they need behavior-test arbitration to stay truthful,
//! which is post-PoC plan material (issue 072 T5's composite ladder).
//!
//! ## Two feature halves (the 2026-09-10 each-feature sweep fix)
//!
//! The TYPE half ([`ConstructionPair`], [`PairVerdict`], [`FenceReport`],
//! [`fence_verdicts`], [`fence_sample`], the provenance vocabulary +
//! [`provenance_weight`]) is dependency-free and UNGATED (Issue 089 T5:
//! [`crate::frontier_report`] consumes the vocabulary from every feature build).
//! The CORPUS half — [`strategy_for`] + [`construction_pairs`], which import
//! `rustc_errors::corpus_gen::generate_corpus` + `floor::MutatorKind` — is
//! `cfg(feature = "rustc_errors")` inside the file. The latent-only arm used
//! to import the corpus unconditionally: every latent-without-`rustc_errors`
//! combo (`latent_retrieval`, `tpr_structure`, `bonsai_retrieval`,
//! `cf_workers_ai`, `fix_verify`, `margin_mining`, `pav_data`,
//! `score_bench`) failed E0433 from the day 072 landed until the sweep
//! caught it.
//!
//! ## Strategy ids
//!
//! Construction truth names the playbook strategy that performs the inverse
//! where one EXISTS (`e0597-scope-hoist` is the doc-quoted classic NLL fix
//! for the narrow-scope escape; `e0499-swap-second-decl` re-swaps the
//! flattened block; `e0502-deref-removal`/`e0502-clone-out` remove the added
//! borrow per Copy-ness; `e0505-clone-at-move` re-inserts the dropped
//! clone). Codes no playbook owns (E0596/E0382/E0507/E0373) use
//! generator-native `gen:{CODE}:{inverse}` ids — NEVER a playbook id whose
//! gate would reject the diagnostic (a mislabeled player would poison the
//! very table the pairs feed).

#[cfg(feature = "rustc_errors")]
use crate::domains::rustc_errors::corpus_gen::generate_corpus;
#[cfg(feature = "rustc_errors")]
use crate::domains::rustc_errors::floor::MutatorKind;

/// The construction-provenance tag every row from this module carries
/// (the pair path must never be confusable with outcome evidence).
pub const PROVENANCE: &str = "construction";

/// Issue 089 T5 — the organic provenance class: pairs mined from real
/// feature churn (an observed kept-fix ≻ reverted-fix on a real repo),
/// never planted by a generator. No organic miner ships yet; the
/// vocabulary + weighting land first so the day one lands, its rows are
/// priced correctly and fenced correctly (report-first — the weight
/// gates nothing today). Never inject failures on purpose to make
/// organic-looking rows: the class exists to MEASURE churn, and a
/// fabricated row priced 1.0 is the worst lie in the table.
pub const PROVENANCE_ORGANIC: &str = "organic";

/// Issue 089 T5 — ε, the synthetic/injected pair weight (organic = 1.0).
/// Prices construction pairs in weighted-yield arithmetic ONLY. The paper
/// effect (+2.06 organic vs +0.00 injected; TaskPilot/FrogNano) prices
/// injected near zero but NOT zero: construction truth is verified by the
/// fence and bootstraps pools with no organic evidence yet (this store's
/// whole BT fit). The value is a report-time pricing knob — measurable
/// once organic rows exist, and never an admission gate (the T4 law:
/// report-first, gate-later).
pub const SYNTHETIC_PAIR_WEIGHT: f32 = 0.05;

/// Issue 089 T5 — the provenance weight: organic feature-churn = 1.0,
/// construction/synthetic = ε, anything else = 0.0 (fail-closed — an
/// unrecognized provenance is priced out of weighted yield entirely, and
/// [`crate::traj_store::TrajStore::rate_pairs`] fences it out of rating
/// outright).
#[must_use]
pub fn provenance_weight(provenance: &str) -> f32 {
    match provenance {
        PROVENANCE_ORGANIC => 1.0,
        PROVENANCE => SYNTHETIC_PAIR_WEIGHT,
        _ => 0.0,
    }
}

/// One construction-labeled preference pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionPair {
    /// The planted truth's error code (`expect_code`).
    pub code: &'static str,
    /// The generator case the pair is anchored to (`gen_...` name).
    pub case_name: String,
    /// Winner: the strategy whose repair reproduces the planted truth.
    pub good_strategy: String,
    /// Loser: the partner case's strategy (does not resolve this case).
    pub bad_strategy: String,
    /// The partner's own error code — the cross-code premise is data, not
    /// convention, so the fence and the A/B can assert it.
    pub bad_code: &'static str,
    /// Winner fix text (the case's own `seed` — self-contained).
    pub good_fix: String,
    /// Loser fix text (the partner's `seed` — plausible, wrong shape here).
    pub bad_fix: String,
    /// The case's shape key (whole `broken` text; hashed at ingest).
    pub span_shape: String,
    /// Always [`PROVENANCE`] — the non-circularity marker.
    pub provenance: &'static str,
}

/// The canonical repair strategy for one planted mutator, per the mapping
/// table in the module docs. `None` never happens for the shipped corpus
/// (the match is total) — the `Option` keeps the fn total against future
/// kinds and the factory `expect`s, loudly, at construction.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn strategy_for(code: &str, kind: MutatorKind) -> String {
    match (code, kind) {
        // The playbook's own doc: "the classic NLL fix for the narrow-scope
        // escape (`let r; { let x = …; r = &x; } *r`)".
        ("E0597", MutatorKind::NarrowScope) => "e0597-scope-hoist".into(),
        // The playbook re-swaps the second decl out of the flattened block.
        ("E0499", MutatorKind::FlattenScope) => "e0499-swap-second-decl".into(),
        // Copy-typed element read by value through the added reference.
        ("E0502", MutatorKind::AddBorrow) => "e0502-deref-removal".into(),
        // The playbook inserts `.clone()` at the move site.
        ("E0505", MutatorKind::DropCloneCall) => "e0505-clone-at-move".into(),
        // No playbook owns these codes — generator-native ids (the playbook
        // gates reject foreign diagnostics; a borrowed id would be a lie).
        ("E0596", MutatorKind::DropMutBinding) => "gen:E0596:add-mut".into(),
        ("E0382", MutatorKind::DropCloneCall) => "gen:E0382:clone-at-move".into(),
        ("E0507", MutatorKind::DropCloneCall) => "gen:E0507:clone-at-move".into(),
        ("E0373", MutatorKind::DropMoveKeyword) => "gen:E0373:add-move".into(),
        _ => format!("gen:{code}:{}", kind.label()),
    }
}

/// FNV-1a over bytes — the deterministic partner rotator (no RNG, the
/// `corpus_gen` house style: byte-stable per build).
#[cfg(feature = "rustc_errors")]
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The full construction-pair corpus: one resolution pair per generated
/// case. Deterministic (partner = hash-rotated cross-code case) — the
/// byte-stability is pinned by test.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn construction_pairs() -> Vec<ConstructionPair> {
    let cases = generate_corpus();
    let n = cases.len();
    let mut out = Vec::with_capacity(n);
    for (i, case) in cases.iter().enumerate() {
        // Deterministic cross-code partner: rotate forward through the
        // corpus until the expect_code differs (the next cross-code case
        // is at most one full lap away; the corpus has 8 codes over 200
        // cases, so the lap terminates). The hash offsets the start so
        // adjacent cases do not all pair with the same partner — but any
        // FIXED rule is byte-stable, which is the actual contract.
        let mut j = (i + 1 + (fnv1a(case.name.as_bytes()) % 64) as usize) % n;
        let mut laps = 0;
        while cases[j].expect_code == case.expect_code && laps < n {
            j = (j + 1) % n;
            laps += 1;
        }
        let partner = &cases[j];
        debug_assert_ne!(partner.expect_code, case.expect_code);
        out.push(ConstructionPair {
            code: case.expect_code,
            case_name: case.name.clone(),
            good_strategy: strategy_for(case.expect_code, case.kind),
            bad_strategy: strategy_for(partner.expect_code, partner.kind),
            bad_code: partner.expect_code,
            good_fix: case.seed.clone(),
            bad_fix: partner.seed.clone(),
            span_shape: case.broken.clone(),
            provenance: PROVENANCE,
        });
    }
    out.shrink_to_fit();
    out
}

/// One sampled pair's fence outcome — what the real oracle said about the
/// pair's construction premise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairVerdict {
    /// The good arm (`seed_i`) compiled with NO `expect_code` diagnostic.
    pub good_resolves: bool,
    /// The bad premise held: the case under no-repair still fails with
    /// `expect_code` (the pair is a valid resolution instance).
    pub bad_premise_holds: bool,
}

impl PairVerdict {
    /// The pair agrees with its construction label.
    #[must_use]
    pub fn agrees(self) -> bool {
        self.good_resolves && self.bad_premise_holds
    }
}

/// The fence report — arithmetic + verdict, pure (the compile outcomes are
/// handed in; the e2e runner that produces them is `#[ignore]`d).
#[derive(Debug, Clone)]
pub struct FenceReport {
    pub pairs_checked: usize,
    pub agreements: usize,
    /// The disagreeing `case_name`s (the loud block names them).
    pub disagreements: Vec<String>,
}

impl FenceReport {
    /// Agreement rate in `[0, 1]`.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)] // sampled-K counts < 2^24 — exact in both casts
    pub fn rate(&self) -> f64 {
        if self.pairs_checked == 0 {
            return 0.0;
        }
        f64::from(self.agreements as u32) / f64::from(self.pairs_checked as u32)
    }

    /// The fence verdict: ≥95% agreement passes (T2's threshold).
    #[must_use]
    pub fn passes(&self, threshold: f64) -> bool {
        self.pairs_checked > 0 && self.rate() >= threshold
    }
}

/// Fold sampled verdicts into the fence report.
#[must_use]
pub fn fence_verdicts(verdicts: &[(&str, PairVerdict)]) -> FenceReport {
    let mut report = FenceReport {
        pairs_checked: verdicts.len(),
        agreements: 0,
        disagreements: Vec::new(),
    };
    for (name, v) in verdicts {
        if v.agrees() {
            report.agreements += 1;
        } else {
            report.disagreements.push((*name).to_string());
        }
    }
    report
}

/// Deterministic stride sample of `k` pairs (T2: "sample K pairs per
/// batch"). Stride = `n / k` rounded down, min 1 — every sampled index is
/// distinct, spread across the corpus, and the selection is byte-stable.
#[must_use]
pub fn fence_sample(pairs: &[ConstructionPair], k: usize) -> Vec<&ConstructionPair> {
    if pairs.is_empty() || k == 0 {
        return Vec::new();
    }
    let stride = (pairs.len() / k).max(1);
    let mut out = Vec::with_capacity(k.min(pairs.len()));
    let mut idx = 0;
    while out.len() < k && idx < pairs.len() {
        out.push(&pairs[idx]);
        idx += stride;
    }
    out.shrink_to_fit();
    out
}

#[cfg(test)]
mod fence_tests {
    // The fence-arithmetic half needs no corpus generator — it runs in every
    // feature state that compiles the module (latent_retrieval-only included).
    use super::*;

    #[test]
    fn provenance_weight_prices_the_three_classes() {
        // Issue 089 T5: organic churn = 1.0, construction/synthetic = ε,
        // anything else = 0.0 (fail-closed — an unrecognized tag is priced
        // out entirely, and rate_pairs fences it out of rating outright).
        // Exact f32 equality IS the contract here — the prices are the pin.
        #![allow(clippy::float_cmp)]
        assert_eq!(provenance_weight(PROVENANCE_ORGANIC), 1.0);
        assert_eq!(provenance_weight(PROVENANCE), SYNTHETIC_PAIR_WEIGHT);
        const { assert!(SYNTHETIC_PAIR_WEIGHT > 0.0 && SYNTHETIC_PAIR_WEIGHT < 1.0) };
        assert_eq!(provenance_weight(""), 0.0);
        assert_eq!(provenance_weight("Organic"), 0.0, "case-sensitive on purpose");
        assert_eq!(provenance_weight("injected"), 0.0);
    }

    #[test]
    fn fence_verdicts_agree_arithmetic() {
        let v = vec![
            (
                "a",
                PairVerdict {
                    good_resolves: true,
                    bad_premise_holds: true,
                },
            ),
            (
                "b",
                PairVerdict {
                    good_resolves: true,
                    bad_premise_holds: false,
                },
            ),
        ];
        let r = fence_verdicts(&v);
        assert_eq!(r.pairs_checked, 2);
        assert_eq!(r.agreements, 1);
        assert!((r.rate() - 0.5).abs() < 1e-12);
        assert!(!r.passes(0.95));
        assert_eq!(r.disagreements, vec!["b".to_string()]);
    }

    #[test]
    fn fence_passes_at_threshold() {
        let verdicts: Vec<(&str, PairVerdict)> = (0..20)
            .map(|i| {
                let name: &'static str = Box::leak(format!("p{i}").into_boxed_str());
                (
                    name,
                    PairVerdict {
                        good_resolves: true,
                        bad_premise_holds: i != 0, // 19/20 = 95%
                    },
                )
            })
            .collect();
        let r = fence_verdicts(&verdicts);
        assert!((r.rate() - 0.95).abs() < 1e-12);
        assert!(r.passes(0.95));
    }
}

#[cfg(all(test, feature = "rustc_errors"))]
mod corpus_tests {
    // The corpus half: needs `generate_corpus`/`construction_pairs` — runs
    // wherever rustc_errors is on (default-on, so every guard layer sees it).
    use super::*;

    #[test]
    fn corpus_yields_one_pair_per_case() {
        let cases = generate_corpus();
        let pairs = construction_pairs();
        assert_eq!(pairs.len(), cases.len());
        assert!(
            pairs.len() >= 200,
            "corpus contract N>=200, got {}",
            pairs.len()
        );
    }

    #[test]
    fn pairs_are_deterministic_across_calls() {
        let a = construction_pairs();
        let b = construction_pairs();
        assert_eq!(a, b);
    }

    #[test]
    fn every_partner_is_cross_code() {
        for p in construction_pairs() {
            assert_ne!(
                p.code, p.bad_code,
                "pair {} has a same-code partner",
                p.case_name
            );
        }
    }

    #[test]
    fn provenance_is_always_construction() {
        for p in construction_pairs() {
            assert_eq!(p.provenance, PROVENANCE);
        }
    }

    #[test]
    fn strategy_mapping_covers_every_corpus_kind() {
        for case in generate_corpus() {
            let s = strategy_for(case.expect_code, case.kind);
            assert!(!s.is_empty());
            assert!(
                s.starts_with("e0") || s.starts_with("gen:"),
                "unmapped strategy for {} / {:?}",
                case.expect_code,
                case.kind
            );
        }
    }

    #[test]
    fn fence_sample_is_spread_and_deterministic() {
        let pairs = construction_pairs();
        let s1 = fence_sample(&pairs, 20);
        let s2 = fence_sample(&pairs, 20);
        assert_eq!(s1.len(), 20);
        assert_eq!(s1, s2);
        // Spread: first and last sampled indices differ by more than 1.
        let first = pairs
            .iter()
            .position(|p| p.case_name == s1[0].case_name)
            .unwrap();
        let last = pairs
            .iter()
            .position(|p| p.case_name == s1[19].case_name)
            .unwrap();
        assert!(last > first);
    }

    #[test]
    fn fence_sample_handles_tiny_inputs() {
        let pairs = construction_pairs();
        assert!(fence_sample(&pairs, 0).is_empty());
        assert_eq!(fence_sample(&[], 5).len(), 0);
        let one = fence_sample(&pairs, 1);
        assert_eq!(one.len(), 1);
    }
}

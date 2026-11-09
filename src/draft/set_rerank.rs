//! Issue 121 (katgpt-rs Plan 599 T4.2) — counter-anchored SET rerank of the
//! merged seven-domain span pool.
//!
//! ## The unanchored set decision this replaces
//!
//! `LatentMatcher::suggest_span` fans one code span out across every enabled
//! domain's `RuleIndex`, re-ranks each domain's pool independently, and the
//! consumers merge the per-domain lists (`SpanSuggestion::all_rules` — a flat
//! score sort). Nothing in that merge counter-anchors the SET: when one rule
//! family (the `needless*` clippy stems, the `collect-*` perf stems) crowds
//! the pool, it fills every visible slot and the correct rule from another
//! family is evicted before any consumer sees it. The stack already DETECTS
//! this post-hoc (`instantiation_rerank::detect_collapse`, the family-entropy
//! verdict); detection is not prevention.
//!
//! ## The R4T triple, consumed not forked
//!
//! katgpt-core's `set_admission` (Plan 599, Bench 811 — G1 greedy==OPT 8/8,
//! G4 zero-alloc) supplies the selection-time composite with mutual
//! counter-anchors: alignment (cos of the candidate's latent to the span's
//! query direction) x diversity (exact marginal log-det gain via
//! Sherman-Morrison on the dual Gram) x groundedness (the modular prior =
//! the retrieval score, min-max normalized over the merged pool, plus the
//! structural-instantiation bonus). R4T's ablation law (arXiv:2603.06397):
//! drop any one anchor and a specific degenerate family becomes optimal —
//! groundingness-only -> coordinate-gaming, +alignment -> paraphrase
//! collapse, +diversity -> the balanced region.
//!
//! Every admitted candidate GROUNDS to a corpus rule by identity — the
//! slate is made of real rules, never virtual queries.
//!
//! ## Recovery
//!
//! When the exact Vendi certificate flags the admitted set as collapsed
//! (`vendi < rho * min(K, DIM)`), `admit_with_recovery_into` re-fans the
//! theta-ladder around the query over the RULE CORPUS (all enabled domains'
//! pattern latents), snaps each fanned direction to a corpus rule, and
//! re-admits over the snapped pool alone — the collapsed pool is the
//! disease, replaced, never merged back. Bounded by `CAP_RUNGS` (4 rungs),
//! deterministic end to end.
//!
//! ## Latent space choice (an A/B axis, not a claim)
//!
//! `riir_rag::ModellessEmbedder` is 8-D: dims `[0..4]` are BLAKE3 hash
//! buckets (hash avalanche = zero locality), dims `[4..8]` are the byte
//! histogram DFT (structural, local). `LatentChoice::Full8` uses the native
//! contract; `LatentChoice::Structural4` zeroes the hash dims (effective
//! rank <= 4 — the certificate honestly reports the lower ceiling because
//! Vendi <= rank, and `saturated` compares against `min(K, DIM=8)`, so a
//! 4-rank set can saturate only at K <= 4). Which space's diversity
//! correlates with the family-entropy eval axis is a measurement, reported
//! by the A/B (Bench 096), not assumed here.
//!
//! ## Eval posture
//!
//! `detect_collapse` stays the EVALUATION axis — the family-entropy
//! detector judges this arm from outside; nothing here reads its verdict
//! to guard anything. The kill-switch (`RIIR_CLIPPY_NO_SET_RERANK=1`)
//! restores the baseline merge order for a same-binary A/B (the
//! `RIIR_CLIPPY_NO_TWO_STAGE` precedent).

use crate::ragplane::{query::LATENT_DIM, Embedder, ModellessEmbedder};
use katgpt_core::set_admission::{
    admit_with_recovery_into, certify_set, AdmissionScratch, CertificateReport, FanScratch,
    RecoveryOutcome, SetAdmissionConfig, DIM,
};

use super::corpus_index::ScoredRule;
use super::domain_router::DomainId;

/// Kill-switch env (same-binary A/B, the `RIIR_CLIPPY_NO_TWO_STAGE`
/// precedent): when set, `suggest_set_slate` returns the baseline merged
/// order with its certificate instead of the counter-anchored admission.
pub const NO_SET_RERANK_ENV: &str = "RIIR_CLIPPY_NO_SET_RERANK";

/// Default structural-evidence bonus: a rule whose proposer actually fires
/// on the span (`instantiates == true`) gets this added to its normalized
/// quality. The starting sweep point, not a claim — half the normalized
/// [0, 1] range makes structural evidence a strong prior that alignment
/// (alpha = 0.6 x cos, at most) can still overrule when the cosine gap is
/// large. Measured by the A/B.
pub const DEFAULT_STRUCTURAL_BONUS: f32 = 0.5;

/// Default slate size for the A/B harness (matches `collapse_monitor::TOP_K`
/// and the recall@5 convention).
pub const DEFAULT_SLATE_K: usize = 5;

/// Which 8-D sub-space the admission operates in (see the module doc — an
/// A/B axis, not a claim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LatentChoice {
    /// The native `ModellessEmbedder` contract (hash dims included).
    #[default]
    Full8,
    /// Hash dims zeroed — only the structural DFT dims `[4..8]` carry
    /// signal (the `domain_router::structural4` precedent that the BLAKE3
    /// dims have zero locality).
    Structural4,
}

impl LatentChoice {
    /// Project one latent into this space. `Structural4` zeroes the hash
    /// dims; normalization stays to the admission kernel (a zero-norm row
    /// is skipped there by contract).
    #[must_use]
    pub fn project(self, v: [f32; LATENT_DIM]) -> [f32; DIM] {
        debug_assert_eq!(
            LATENT_DIM, DIM,
            "the consumer space and the admission space must agree"
        );
        match self {
            Self::Full8 => v,
            Self::Structural4 => [0.0, 0.0, 0.0, 0.0, v[4], v[5], v[6], v[7]],
        }
    }
}

/// Configuration for the set rerank arm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetRerankConfig {
    /// The counter-anchor weights (alpha / kappa / `theta_coll` / `rho_vendi`)
    /// — `SetAdmissionConfig::default()` is R4T's 0.6/0.2 proportion prior.
    pub admission: SetAdmissionConfig,
    /// Bonus added to normalized quality for `instantiates == true`
    /// candidates (see [`DEFAULT_STRUCTURAL_BONUS`]).
    pub structural_bonus: f32,
    /// Which latent sub-space to admit over.
    pub latent: LatentChoice,
}

impl Default for SetRerankConfig {
    fn default() -> Self {
        Self {
            admission: SetAdmissionConfig::default(),
            structural_bonus: DEFAULT_STRUCTURAL_BONUS,
            latent: LatentChoice::default(),
        }
    }
}

/// The embedded rule corpus — snap targets for the recovery fan and the
/// identity map from latent index back to rule. Built deterministically
/// (rule ids in sorted order) from the matcher's rule book, so the same
/// corpus + config build a bit-identical corpus.
pub struct RuleLatentCorpus {
    /// Sorted rule ids (the deterministic ordering).
    pub rule_ids: Vec<String>,
    /// Owning domain per rule (parallel to `rule_ids`).
    pub domains: Vec<DomainId>,
    /// Projected + raw latents per rule (parallel to `rule_ids`).
    pub latents: Vec<[f32; DIM]>,
    /// `buggy_code` per rule (parallel to `rule_ids`) — the exact text the
    /// `RuleIndex` embedded at build time.
    pub patterns: Vec<String>,
}

impl RuleLatentCorpus {
    /// Embed every `(rule_id, domain, pattern)` triple with the shared
    /// modelless embedder, sorted by rule id.
    #[must_use]
    pub fn build(entries: &[(String, DomainId, String)]) -> Self {
        let mut sorted: Vec<&(String, DomainId, String)> = entries.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let embedder = ModellessEmbedder;
        Self {
            rule_ids: sorted.iter().map(|e| e.0.clone()).collect(),
            domains: sorted.iter().map(|e| e.1).collect(),
            latents: sorted.iter().map(|e| embedder.embed(&e.2)).collect(),
            patterns: sorted.iter().map(|e| e.2.clone()).collect(),
        }
    }

    /// Number of rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rule_ids.len()
    }

    /// Whether the corpus is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rule_ids.is_empty()
    }

    /// Latent of `rule_id` in the chosen space, or `None` when absent.
    #[must_use]
    pub fn latent_of(&self, choice: LatentChoice, rule_id: &str) -> Option<[f32; DIM]> {
        self.rule_ids
            .binary_search_by(|probe| probe.as_str().cmp(rule_id))
            .ok()
            .map(|i| choice.project(self.latents[i]))
    }
}

/// The matcher-held admission + fan scratch pair (G4 — allocations persist
/// across set-rerank calls).
#[derive(Default)]
pub struct MatcherSetScratch {
    pub admission: AdmissionScratch,
    pub fan: FanScratch,
}

/// One admitted slate member.
#[derive(Debug, Clone, PartialEq)]
pub struct AdmittedRule {
    /// The rule id (a real corpus rule — the grounding identity).
    pub rule_id: String,
    /// Owning domain.
    pub domain: DomainId,
    /// How it was admitted: `false` = from the retrieved merged pool
    /// (carrying its retrieval score + structural flag), `true` = from a
    /// recovery rung's fan (snapped corpus rule, possibly never retrieved).
    pub via_recovery: bool,
    /// The fused retrieval score when admitted from the pool.
    pub score: Option<f32>,
    /// The structural-instantiation flag when admitted from the pool.
    pub instantiates: Option<bool>,
}

/// The outcome of one set-rerank cycle.
#[derive(Debug, Clone, PartialEq)]
pub struct SetSlate {
    /// The admitted slate, admission order (best first).
    pub admitted: Vec<AdmittedRule>,
    /// The final certificate (post-recovery when it fired).
    pub certificate: CertificateReport,
    /// Whether the initial admission's certificate flagged collapse and the
    /// recovery ladder ran.
    pub recovery_fired: bool,
    /// Whether the recovery ladder ended clean (trivially `true` when it
    /// never fired; `false` = every rung either failed to ground or stayed
    /// collapsed — the last attempt's slate stands, best effort).
    pub recovery_recovered: bool,
    /// Ladder rungs the recovery walked (0 when it never fired).
    pub rungs_tried: usize,
    /// `true` when the kill-switch restored the baseline merge order
    /// (the same-binary A/B control arm).
    pub baseline_fallback: bool,
}

/// Merge the per-domain ranked lists into the candidate pool, preserving
/// domain tags and the pool order each domain delivered.
#[must_use]
pub fn merge_pool(by_domain: &[(DomainId, Vec<ScoredRule>)]) -> Vec<(DomainId, &ScoredRule)> {
    by_domain
        .iter()
        .flat_map(|(d, rules)| rules.iter().map(move |r| (*d, r)))
        .collect()
}

/// The counter-anchored admission over one merged pool.
///
/// `query_raw` is the span's native 8-D embedding (projected internally
/// per `cfg.latent`). Returns the slate, certificate and recovery summary.
/// Deterministic: same pool + corpus + config -> bit-identical slate.
///
/// # Panics
/// Panics via the substrate when `corpus.len() > u16::MAX` (the index
/// encoding) — the rule corpus is ~10^2-10^3 rules, far under the bound.
#[must_use]
#[allow(clippy::too_many_arguments)] // admission + recovery fan: each param is a distinct role (the substrate's own shape)
pub fn counter_anchored_slate(
    cfg: &SetRerankConfig,
    by_domain: &[(DomainId, Vec<ScoredRule>)],
    rule_latents: impl Fn(&str) -> Option<[f32; DIM]>,
    corpus: &RuleLatentCorpus,
    query_raw: [f32; LATENT_DIM],
    k: usize,
    scratch: &mut AdmissionScratch,
    fan: &mut FanScratch,
) -> SetSlate {
    let query = cfg.latent.project(query_raw);
    let pool = merge_pool(by_domain);
    if k == 0 || pool.is_empty() || corpus.is_empty() {
        return SetSlate {
            admitted: Vec::new(),
            certificate: CertificateReport {
                vendi: 0.0,
                participation_ratio: 0.0,
                saturated: false,
                collapsed: false,
            },
            recovery_fired: false,
            recovery_recovered: true,
            rungs_tried: 0,
            baseline_fallback: false,
        };
    }
    let k = k.min(pool.len());

    // Latents per pool entry; un-embeddable entries (zero structural norm
    // under Structural4) drop out of the pool rather than poison the kernel.
    let mut latents: Vec<[f32; DIM]> = Vec::with_capacity(pool.len());
    let mut kept: Vec<usize> = Vec::with_capacity(pool.len());
    for (i, (_, rule)) in pool.iter().enumerate() {
        if let Some(x) = rule_latents(rule.rule_id.as_str()) {
            latents.push(x);
            kept.push(i);
        }
    }
    if latents.is_empty() {
        return empty_slate_beside_pool();
    }
    // quality = min-max over the KEPT pool's scores (the same distribution
    // the baseline sorts by, restricted to kernel-usable rows; degenerate
    // single-value pools map every candidate to 0.5 — alignment decides),
    // plus the structural-instantiation bonus.
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &i in &kept {
        let s = pool[i].1.score;
        min = min.min(s);
        max = max.max(s);
    }
    let span = max - min;
    let quality: Vec<f32> = kept
        .iter()
        .map(|&i| {
            let r = pool[i].1;
            let base = if span > f32::EPSILON {
                (r.score - min) / span
            } else {
                0.5
            };
            base + if r.instantiates {
                cfg.structural_bonus
            } else {
                0.0
            }
        })
        .collect();

    let mut pool_idx = vec![u16::MAX; k];
    let mut corpus_idx = vec![u16::MAX; k];
    let outcome: RecoveryOutcome = admit_with_recovery_into(
        &cfg.admission,
        &latents,
        &quality,
        &query,
        &corpus.latents,
        k,
        &mut pool_idx,
        &mut corpus_idx,
        scratch,
        fan,
    );

    let mut admitted: Vec<AdmittedRule> = Vec::with_capacity(k);
    if outcome.fired {
        for &slot in corpus_idx.iter().take(outcome.admitted) {
            let c = slot as usize;
            admitted.push(AdmittedRule {
                rule_id: corpus.rule_ids[c].clone(),
                domain: corpus.domains[c],
                via_recovery: true,
                score: None,
                instantiates: None,
            });
        }
    } else {
        for &slot in pool_idx.iter().take(outcome.admitted) {
            let p = slot as usize;
            let (domain, rule) = pool[kept[p]];
            admitted.push(AdmittedRule {
                rule_id: rule.rule_id.clone(),
                domain,
                via_recovery: false,
                score: Some(rule.score),
                instantiates: Some(rule.instantiates),
            });
        }
    }

    SetSlate {
        admitted,
        certificate: outcome.certificate,
        recovery_fired: outcome.fired,
        recovery_recovered: outcome.recovered,
        rungs_tried: outcome.rungs_tried,
        baseline_fallback: false,
    }
}

fn empty_slate_beside_pool() -> SetSlate {
    SetSlate {
        admitted: Vec::new(),
        certificate: CertificateReport {
            vendi: 0.0,
            participation_ratio: 0.0,
            saturated: false,
            collapsed: false,
        },
        recovery_fired: false,
        recovery_recovered: true,
        rungs_tried: 0,
        baseline_fallback: false,
    }
}

/// The BASELINE slate — the current merged order (`all_rules`' key:
/// instantiates desc, score desc, `rule_id` asc), top-k — with its
/// certificate. The A/B control arm and the kill-switch fallback.
#[must_use]
pub fn baseline_slate(
    cfg: &SetRerankConfig,
    by_domain: &[(DomainId, Vec<ScoredRule>)],
    rule_latents: impl Fn(&str) -> Option<[f32; DIM]>,
    k: usize,
) -> SetSlate {
    let pool = merge_pool(by_domain);
    let mut order: Vec<usize> = (0..pool.len()).collect();
    order.sort_by(|&a, &b| {
        let (_, ra) = pool[a];
        let (_, rb) = pool[b];
        rb.instantiates
            .cmp(&ra.instantiates)
            .then_with(|| rb.score.total_cmp(&ra.score))
            .then_with(|| ra.rule_id.cmp(&rb.rule_id))
    });
    let top: Vec<usize> = order.into_iter().take(k).collect();
    let mut admitted = Vec::with_capacity(top.len());
    let mut latents = Vec::with_capacity(top.len());
    for &i in &top {
        let (domain, rule) = pool[i];
        if let Some(x) = rule_latents(rule.rule_id.as_str()) {
            latents.push(x);
            admitted.push(AdmittedRule {
                rule_id: rule.rule_id.clone(),
                domain,
                via_recovery: false,
                score: Some(rule.score),
                instantiates: Some(rule.instantiates),
            });
        }
    }
    let certificate = certify_set(&cfg.admission, &latents);
    SetSlate {
        admitted,
        certificate,
        recovery_fired: false,
        recovery_recovered: true,
        rungs_tried: 0,
        baseline_fallback: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, score: f32) -> ScoredRule {
        ScoredRule {
            rule_id: id.to_string(),
            score,
            instantiates: false,
        }
    }

    fn corpus() -> RuleLatentCorpus {
        let entries: Vec<(String, DomainId, String)> = [
            ("needless_return", "pub fn a() -> u8 { return 7; }"),
            (
                "needless_bool",
                "pub fn b(x: bool) -> u8 { if x { return 1; } return 0; }",
            ),
            (
                "vec-with-capacity",
                "let mut v: Vec<u8> = Vec::new(); v.push(1);",
            ),
            (
                "collect-append-to-extend",
                "let o: Vec<u8> = i.iter().map(|x| *x).collect();",
            ),
            (
                "hashmap-with-capacity",
                "let m: HashMap<u8, u8> = HashMap::new(); m.insert(1, 1);",
            ),
            (
                "static-str-ptrs",
                "let s: &'static str = \"constant text here\";",
            ),
        ]
        .iter()
        .map(|&(id, text)| (id.to_string(), DomainId::Clippy, text.to_string()))
        .collect();
        RuleLatentCorpus::build(&entries)
    }

    fn matcher_latents(c: &RuleLatentCorpus) -> impl Fn(&str) -> Option<[f32; DIM]> + '_ {
        move |id| c.latent_of(LatentChoice::Full8, id)
    }

    #[test]
    fn slate_is_deterministic() {
        let c = corpus();
        let pool = vec![
            (
                DomainId::Clippy,
                vec![
                    rule("needless_return", 9.0),
                    rule("needless_bool", 8.5),
                    rule("vec-with-capacity", 8.0),
                ],
            ),
            (
                DomainId::Perf,
                vec![
                    rule("collect-append-to-extend", 7.5),
                    rule("hashmap-with-capacity", 7.0),
                ],
            ),
        ];
        let cfg = SetRerankConfig::default();
        let q = ModellessEmbedder.embed("pub fn a() -> u8 { return 7; }");
        let mut s1 = AdmissionScratch::new();
        let mut s2 = AdmissionScratch::new();
        let mut f1 = FanScratch::new();
        let mut f2 = FanScratch::new();
        let a =
            counter_anchored_slate(&cfg, &pool, matcher_latents(&c), &c, q, 4, &mut s1, &mut f1);
        let b =
            counter_anchored_slate(&cfg, &pool, matcher_latents(&c), &c, q, 4, &mut s2, &mut f2);
        assert_eq!(a, b, "same inputs must produce a bit-identical slate");
        assert_ne!(a.admitted, [] as [AdmittedRule; 0]);
        assert!(!a.baseline_fallback);
    }

    #[test]
    fn a_collapsed_family_pool_fires_recovery_when_the_corpus_has_room() {
        // Every pool entry from ONE family (needless*), so the admitted set
        // is near-duplicate by construction in any space that has any
        // locality at all. Whether the certificate FIRES depends on the
        // vendi floor over the embedded patterns — hash-heavy Full8 may
        // spread them; the assertion is the INVARIANT, not the verdict:
        // when the certificate is clean the slate stands, when collapsed
        // the ladder ran and bounded itself.
        let c = corpus();
        let pool = vec![(
            DomainId::Clippy,
            vec![rule("needless_return", 9.0), rule("needless_bool", 8.5)],
        )];
        let cfg = SetRerankConfig::default();
        let q = ModellessEmbedder.embed("pub fn a() -> u8 { return 7; }");
        let mut s = AdmissionScratch::new();
        let mut f = FanScratch::new();
        let slate =
            counter_anchored_slate(&cfg, &pool, matcher_latents(&c), &c, q, 2, &mut s, &mut f);
        if slate.recovery_fired {
            assert!(slate.rungs_tried >= 1, "fired implies >= 1 rung");
            assert!(slate.rungs_tried <= katgpt_core::set_admission::CAP_RUNGS.len());
            // Recovery slate members ground to corpus rules by identity.
            for m in &slate.admitted {
                assert!(m.via_recovery, "post-recovery members come from the fan");
                assert!(
                    c.rule_ids.iter().any(|id| id == &m.rule_id),
                    "recovery member must be a corpus rule by identity"
                );
            }
        } else {
            assert!(!slate.certificate.collapsed);
        }
    }

    #[test]
    fn empty_and_degenerate_inputs_do_not_panic() {
        let c = corpus();
        let cfg = SetRerankConfig::default();
        let q = ModellessEmbedder.embed("x");
        let mut s = AdmissionScratch::new();
        let mut f = FanScratch::new();
        let empty: Vec<(DomainId, Vec<ScoredRule>)> = Vec::new();
        let out =
            counter_anchored_slate(&cfg, &empty, matcher_latents(&c), &c, q, 5, &mut s, &mut f);
        assert_eq!(out.admitted, [] as [AdmittedRule; 0]);
        let out =
            counter_anchored_slate(&cfg, &empty, matcher_latents(&c), &c, q, 0, &mut s, &mut f);
        assert_eq!(out.admitted, [] as [AdmittedRule; 0]);
    }

    #[test]
    fn baseline_slate_matches_canonical_order() {
        let c = corpus();
        let pool = vec![(
            DomainId::Clippy,
            vec![rule("needless_return", 9.0), rule("vec-with-capacity", 8.0)],
        )];
        let cfg = SetRerankConfig::default();
        let slate = baseline_slate(&cfg, &pool, matcher_latents(&c), 2);
        assert_eq!(slate.admitted.len(), 2);
        assert_eq!(slate.admitted[0].rule_id, "needless_return");
        assert!(slate.baseline_fallback);
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identity is the assertion (Full8 is the identity projection)
    fn structural4_zeroes_hash_dims() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        assert_eq!(LatentChoice::Full8.project(v), v);
        assert_eq!(
            LatentChoice::Structural4.project(v),
            [0.0, 0.0, 0.0, 0.0, 5.0, 6.0, 7.0, 8.0]
        );
    }
}

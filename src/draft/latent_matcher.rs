//! The composed `--suggest` pipeline (Plan 005 T1.3 + T1.5).
//!
//! ```text
//! real .rs file → AstChunker (fn/impl/struct spans)
//!              → ModellessEmbedder → DomainRouter (which domain owns it)
//!              → that domain's RuleIndex.retrieve → ranked rules
//! ```
//!
//! ZERO proposer code — the corpus `buggy_code`/`ref_shape` +
//! `fixed_code`/`opt_pattern` texts ARE the suggestion (`--fix` needs the
//! Phase 4/5 proposers). Composes riir-rag as-is; nothing here reimplements
//! chunking, embedding, or retrieval.

use riir_rag::ast_chunker::AstChunker;
use riir_rag::Chunker;

// dead under thin feature sets (its consumers live in the clippy_lints +
// rust_perf lanes and the gated tests module); gated to match
#[cfg_attr(
    not(all(feature = "clippy_lints", feature = "rust_perf")),
    allow(unused_imports)
)]
use crate::domain::{HealDomain, HealEntry};

use super::clippy_feed::ClippyDiagnostic;
use super::corpus_index::{RetrievalConfig, RuleIndex, ScoredRule, SuggestScratch};
use super::domain_router::{DomainId, DomainRouter, RoutingEmbedding, SpectralOrder};

/// A leased epoch's per-domain corpus overlay (T1.6): domain → retrieval
/// triples. Mirrors [`crate::kat_lease_client::DomainTriples`] one layer up
/// (that module is `kat_service`-only; this alias keeps the matcher's
/// surface self-contained under `latent_retrieval` alone).
pub type DomainOverlay = [(DomainId, Vec<(String, String, String)>)];

/// One suggestion bundle for one code span.
#[derive(Debug, Clone, PartialEq)]
pub struct SpanSuggestion {
    /// Source file the span came from.
    pub path: String,
    /// 1-based inclusive line range of the span.
    pub start_line: usize,
    pub end_line: usize,
    /// The router's domain verdict for the span (ANNOTATION — retrieval is
    /// fan-out, not gated; see `LatentMatcher::suggest_span`).
    pub routed_domain: DomainId,
    /// Per-enabled-domain ranked rules (best first), one entry per domain
    /// that has an index.
    pub by_domain: Vec<(DomainId, Vec<ScoredRule>)>,
    /// Oracle-anchored rules (the clippy-fed mode, 2026-09-13 intake):
    /// rules KNOWN to fire at this span — real clippy found them —
    /// force-included ahead of similarity ranking so the top-N gate
    /// cannot eat a true positive. `(domain, rule_id)` pairs, domain from
    /// the rule book. Empty on the plain (feed-less) path.
    pub anchored_rules: Vec<(DomainId, String)>,
    /// Oracle lint names at this span with NO corpus rule in this build —
    /// a live corpus-gap signal, surfaced instead of swallowed (the
    /// mining intake's input).
    pub anchored_missing: Vec<String>,
    /// True only when an oracle feed covered this file and NO diagnostic
    /// fell inside this span: every ranked rule below is then
    /// heuristic-only (similarity without an oracle signal — the intake's
    /// false-positive datum). False when no feed covered the file (the
    /// plain path claims nothing either way) and on anchored spans.
    pub heuristic_only: bool,
    /// Measured-hot annotations covering this span (Issue 146 T0.2, the
    /// hotpath-fed mode): the profile's ranked entries whose `(file, line)`
    /// falls inside. Timing (`calls × avg`) is the RANKING signal only
    /// (B129 — instrumented attribution, never a verification target);
    /// `alloc_bytes`/`alloc_calls` ride as the DETERMINISTIC verification
    /// axis. Empty on the plain (feed-less) path. A hot-marked span is an
    /// oracle-signal span: it is never `heuristic_only`.
    pub hot: Vec<crate::hotpath_report::HotSpan>,
}

impl SpanSuggestion {
    /// All suggested rules across domains, canonical order best-first
    /// (`instantiates` desc, then score desc, then `rule_id` asc).
    ///
    /// Issue 121 (Bench 096): the key previously sorted by score ALONE,
    /// contradicting this method's own doc ("same canonical order as
    /// `rank_into`/`rerank`/`canonical_rank`") and silently DISCARDING the
    /// per-domain instantiation re-rank at merge time — a clippy rule whose
    /// proposer fired on the span could be buried under higher-BM25 perf
    /// rules. Measured on the composite rig: +55.6pp top-1 / +40.7pp
    /// recall@5 for the canonical key vs the score-only merge.
    #[must_use]
    pub fn all_rules(&self) -> Vec<&ScoredRule> {
        let mut all: Vec<&ScoredRule> = self
            .by_domain
            .iter()
            .flat_map(|(_, rules)| rules.iter())
            .collect();
        // MUST match `instantiation_rerank::rerank`'s key order —
        // instantiation first, then similarity, then rule_id. Sorting by
        // score alone silently undoes the T3.5.4 re-rank at the merge
        // (the exact defect the Issue 121 A/B caught).
        all.sort_unstable_by(|a, b| {
            b.instantiates
                .cmp(&a.instantiates)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });
        all
    }
}

/// A corpus rule's displayable essence — what `--suggest` prints for manual
/// review (Plan 005 T2.3).
#[derive(Debug, Clone, PartialEq)]
pub struct RuleInfo {
    /// Owning domain.
    pub domain: DomainId,
    /// The slow/buggy pattern (what the span was matched against).
    pub buggy_code: String,
    /// The fixed/optimized pattern (the manual-review suggestion).
    pub fixed_code: String,
}

/// The kernel domain's index: the flat `RuleIndex`, or — under the opt-in
/// `kernel_route_narrow` feature (Issue 196) — the route-narrowed index of
/// 15 per-category slices with a bounded global tail. Every consumer
/// dispatches through this type's methods, so the OFF posture is the flat
/// path byte-identical.
pub(crate) enum KernelIndex {
    /// The flat full-corpus index (the pre-196 shape, the default posture).
    /// Never constructed under the narrow feature (the build arm selects
    /// `Narrowed` there) — the lint is feature-gated, not a defect. Boxed
    /// beside `Narrowed` so neither variant inflates the enum (the matcher
    /// struct rides by value).
    #[cfg_attr(feature = "kernel_route_narrow", allow(dead_code))]
    Flat(Box<RuleIndex>),
    /// The route-narrowed index (Issue 196; the feature selects it at
    /// build — see [`Self::build`]). Boxed: 16 sub-`RuleIndex`es dwarf the
    /// flat variant.
    #[cfg(feature = "kernel_route_narrow")]
    Narrowed(Box<crate::domains::kernel_opt::route_narrow::NarrowedKernelIndex>),
}

// The whole type is dead when the kernel domain is feature-off (the field
// stays None; the enum exists only so the field type is posture-stable).
#[cfg_attr(not(feature = "kernel_opt"), allow(dead_code))]
impl KernelIndex {
    /// Build the kernel index — flat by default, route-narrowed under the
    /// feature. The single posture-selection point. The flat arm copies the
    /// slice into `build_with`'s owned shape (build-time, once per process);
    /// the narrowed arm partitions by reference — no by-value split to
    /// annotate per posture.
    fn build(patterns: &[(String, String)], config: RetrievalConfig) -> Result<Self, String> {
        #[cfg(feature = "kernel_route_narrow")]
        return Ok(Self::Narrowed(Box::new(
            crate::domains::kernel_opt::route_narrow::NarrowedKernelIndex::build(patterns, config)?,
        )));
        #[cfg(not(feature = "kernel_route_narrow"))]
        Ok(Self::Flat(Box::new(RuleIndex::build_with(
            patterns.to_vec(),
            config,
        )?)))
    }

    /// Dispatch the multi-scale suggest — flat self, narrowed via the
    /// slice+tail retrieval (Issue 196). The FLAT arm honors the kernel
    /// pregate's per-span line-query cap (Bench 134, issue 208 T6): when
    /// `RIIR_KERNEL_PREGATE_LINE_CAP` arms (pinned literals 4/2), per-span
    /// line queries are stride-capped — the whole-span query always runs.
    /// `None` = the exact pre-cap behavior. The narrowed arm is NOT in the
    /// Bench-134 registration (the Issue-196 measured-negative lane) and
    /// stays uncapped.
    fn suggest_deep_into<'s>(
        &self,
        code: &str,
        top_k: usize,
        scratch: &'s mut SuggestScratch,
    ) -> &'s [ScoredRule] {
        match self {
            Self::Flat(index) => match crate::kernel_pregate::line_cap() {
                Some(cap) => index.suggest_deep_into_capped(code, top_k, scratch, cap),
                None => index.suggest_deep_into(code, top_k, scratch),
            },
            #[cfg(feature = "kernel_route_narrow")]
            Self::Narrowed(narrowed) => narrowed.suggest_deep_into(code, top_k, scratch),
        }
    }

    /// Forward the differential-anchor rider (flat: the index's own;
    /// narrowed: every slice + tail — the per-domain rider semantics).
    #[cfg(feature = "differential_anchor")]
    fn set_differential_anchor(&mut self, lambda: f32) {
        match self {
            Self::Flat(index) => index.set_differential_anchor(lambda),
            #[cfg(feature = "kernel_route_narrow")]
            Self::Narrowed(narrowed) => narrowed.set_differential_anchor(lambda),
        }
    }

    /// Forward the Issue-126 trained-term rider — FLAT only (the trained
    /// term is a flat-index rider; the narrowed posture falls back to the
    /// untrained ranking — the disclosed interaction, the `route_narrow`
    /// module docs).
    #[cfg(feature = "rule_embed_poc")]
    fn set_rule_embed(
        &mut self,
        tables: &std::sync::Arc<super::rule_embed::TrainedTables>,
        weight: f32,
    ) {
        if let Self::Flat(index) = self {
            index.set_rule_embed(tables, weight);
        }
    }
}

/// The cross-domain latent matcher — owns the router + one index per enabled
/// domain.
///
/// Construction indexes every enabled domain's corpus (clippy ~10, perf ~46,
/// kernel ~155 rules — one modelless embedding each; milliseconds).
pub struct LatentMatcher {
    router: DomainRouter,
    /// How much proposer evidence counts when ordering candidates (T3.5.4).
    rerank_mode: super::instantiation_rerank::RerankMode,
    clippy: Option<RuleIndex>,
    perf: Option<RuleIndex>,
    kernel: Option<KernelIndex>,
    sec: Option<RuleIndex>,
    docker: Option<RuleIndex>,
    dist: Option<RuleIndex>,
    shader_perf: Option<RuleIndex>,
    rule_book: std::collections::HashMap<String, RuleInfo>,
    /// G4 scratch — reused across every span + domain query.
    scratch: super::corpus_index::SuggestScratch,
    /// Issue 121: the embedded rule corpus (snap targets for the recovery
    /// fan + the latent lookup for pool entries), built from `rule_book`
    /// deterministically at construction.
    #[cfg(feature = "set_rerank")]
    rule_latents: super::set_rerank::RuleLatentCorpus,
    /// Issue 121: the admission + fan scratch, reused across set-rerank
    /// calls (the G4 discipline — allocation-free steady state).
    #[cfg(feature = "set_rerank")]
    set_scratch: super::set_rerank::MatcherSetScratch,
    /// Issue 121: the set-rerank weights (A/B knob).
    #[cfg(feature = "set_rerank")]
    set_cfg: super::set_rerank::SetRerankConfig,
    /// Issue 102 T2/T3: the corpus active-set exclusions — demoted rule ids
    /// per domain, dropped from every similarity fan-out before ranking.
    /// Default-empty (byte-identical posture): nothing demotes
    /// automatically — the fade-predictive gate stays closed, and only the
    /// operator-written marker file (`.refine/active_set.json`) demotes.
    active_exclusions: crate::active_set::ActiveSet,
    /// Issue 134 T2: the pool-depth allocation policy — per-domain factors
    /// over the retrieval pool + truncation K, built from the standing
    /// demonstrability constants. Default identity (byte-identical
    /// posture); the ONLY writer is [`Self::with_depth_policy`] (the
    /// auto-arm path lives in the CLI).
    #[cfg(feature = "demonstrability_gate")]
    depth_policy: crate::demonstrability_gate::DepthPolicy,
}

impl LatentMatcher {
    /// Build from the corpora of every feature-enabled domain.
    ///
    /// Each enabled domain contributes (a) its patterns to the router's
    /// centroid and (b) a [`RuleIndex`].
    ///
    /// # Errors
    ///
    /// Returns an error `String` if no domain is feature-enabled, or if any
    /// enabled domain's [`RuleIndex::build`] fails.
    #[allow(clippy::too_many_lines)]
    pub fn from_enabled_domains() -> Result<Self, String> {
        Self::from_enabled_domains_with(super::corpus_index::RetrievalConfig::default())
    }

    /// Build with a per-domain corpus OVERLAY (Proposal 005 T1.6, Plan 040 —
    /// the corpus-lease consumer's install point).
    ///
    /// An epoch is a FULL-corpus state per covered domain: a domain with a
    /// non-empty overlay entry REPLACES its compiled-in seed corpus with the
    /// leased entries (deterministic — the same epoch + same entries build a
    /// bit-identical index); a domain with NO overlay entry keeps the
    /// compiled-in seed. Overlay entries for a feature-OFF domain are ignored
    /// (that domain compiles no index at all). This is what makes the leased
    /// epoch swappable at a run boundary + pinnable (`--corpus-epoch`) for CI
    /// reproducibility.
    ///
    /// # Errors
    ///
    /// Returns an error `String` if no domain is feature-enabled, or if any
    /// enabled domain's [`RuleIndex::build_with`] fails.
    #[allow(clippy::too_many_lines)]
    pub fn from_enabled_domains_with_overlay(overlay: &DomainOverlay) -> Result<Self, String> {
        Self::build_with_config(
            super::corpus_index::RetrievalConfig::default(),
            Some(overlay),
        )
    }

    /// [`from_enabled_domains`](Self::from_enabled_domains) with an explicit
    /// retrieval config — the Plan 005 T3.5 A/B constructor.
    ///
    /// # Errors
    ///
    /// Returns an error `String` if no domain is feature-enabled, or if any
    /// enabled domain's [`RuleIndex::build_with`] fails.
    #[allow(clippy::too_many_lines)]
    pub fn from_enabled_domains_with(
        config: super::corpus_index::RetrievalConfig,
    ) -> Result<Self, String> {
        Self::build_with_config(config, None)
    }

    /// The shared constructor core. `overlay` (when `Some`) replaces the
    /// compiled-in corpus per covered domain — see
    /// [`from_enabled_domains_with_overlay`].
    ///
    /// # Errors
    ///
    /// Returns an error `String` if no domain is feature-enabled, or if any
    /// enabled domain's [`RuleIndex::build_with`] fails.
    ///
    /// [`from_enabled_domains_with_overlay`]: `Self::from_enabled_domains_with_overlay`
    #[allow(clippy::too_many_lines)]
    fn build_with_config(
        config: super::corpus_index::RetrievalConfig,
        overlay: Option<&DomainOverlay>,
    ) -> Result<Self, String> {
        // Overlay entries for a domain, when present + non-empty (else None
        // → the compiled-in seed stands).
        let overlay_for = |d: DomainId| -> Option<Vec<(String, String, String)>> {
            overlay
                .and_then(|o| o.iter().find(|(od, _)| *od == d))
                .map(|(_, v)| v.clone())
                .filter(|v| !v.is_empty())
        };
        // under thin feature sets most domains are cfg'd out and neither
        // binding sees its push — the mut stays for the enabled-domain path
        #[cfg_attr(not(feature = "clippy_lints"), allow(unused_mut))]
        let mut router_domains = Vec::new();
        #[cfg_attr(not(feature = "clippy_lints"), allow(unused_mut))]
        let mut rule_book: std::collections::HashMap<String, RuleInfo> =
            std::collections::HashMap::new();

        // same lane discipline: unused when every consumer domain is off
        #[cfg_attr(not(feature = "clippy_lints"), allow(unused_macros))]
        macro_rules! index_domain {
            ($domain:expr, $corpus:expr, $index_ctor:expr) => {{
                let entries: Vec<_> = $corpus;
                let mut pattern_texts = Vec::with_capacity(entries.len());
                for (rule_id, buggy, fixed) in &entries {
                    pattern_texts.push(buggy.clone());
                    rule_book.insert(
                        rule_id.clone(),
                        RuleInfo {
                            domain: $domain,
                            buggy_code: buggy.clone(),
                            fixed_code: fixed.clone(),
                        },
                    );
                }
                router_domains.push(($domain, pattern_texts));
                let patterns: Vec<(String, String)> = entries
                    .into_iter()
                    .map(|(rule_id, buggy, _fixed)| (rule_id, buggy))
                    .collect();
                // The constructor tail is per-domain: `RuleIndex::build_with`
                // (owned patterns) for the six flat domains, a borrowing
                // closure over `KernelIndex::build` for kernel — which
                // selects flat vs route-narrowed under Issue 196's feature.
                Some(($index_ctor)(patterns, config)?)
            }};
        }

        #[cfg(feature = "clippy_lints")]
        let clippy = index_domain!(
            DomainId::Clippy,
            overlay_for(DomainId::Clippy).unwrap_or_else(|| {
                crate::domains::clippy_lints::ClippyLintDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            RuleIndex::build_with
        );
        #[cfg(not(feature = "clippy_lints"))]
        let clippy = None;

        #[cfg(feature = "rust_perf")]
        let perf = index_domain!(
            DomainId::Perf,
            overlay_for(DomainId::Perf).unwrap_or_else(|| {
                crate::domains::rust_perf::RustPerfDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            RuleIndex::build_with
        );
        #[cfg(not(feature = "rust_perf"))]
        let perf = None;

        #[cfg(feature = "kernel_opt")]
        let kernel = index_domain!(
            DomainId::Kernel,
            overlay_for(DomainId::Kernel).unwrap_or_else(|| {
                crate::domains::kernel_opt::KernelOptDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            |patterns: Vec<(String, String)>, config| KernelIndex::build(&patterns, config)
        );
        #[cfg(not(feature = "kernel_opt"))]
        let kernel = None;

        #[cfg(feature = "sec")]
        let sec = index_domain!(
            DomainId::Sec,
            overlay_for(DomainId::Sec).unwrap_or_else(|| {
                crate::domains::sec::SecDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            RuleIndex::build_with
        );
        #[cfg(not(feature = "sec"))]
        let sec = None;

        #[cfg(feature = "docker")]
        let docker = index_domain!(
            DomainId::Docker,
            overlay_for(DomainId::Docker).unwrap_or_else(|| {
                crate::domains::docker::DockerDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            RuleIndex::build_with
        );
        #[cfg(not(feature = "docker"))]
        let docker = None;

        #[cfg(feature = "dist")]
        let dist = index_domain!(
            DomainId::Dist,
            overlay_for(DomainId::Dist).unwrap_or_else(|| {
                crate::domains::dist::DistDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            RuleIndex::build_with
        );
        #[cfg(not(feature = "dist"))]
        let dist = None;

        #[cfg(feature = "shader_perf")]
        let shader_perf = index_domain!(
            DomainId::ShaderPerf,
            overlay_for(DomainId::ShaderPerf).unwrap_or_else(|| {
                crate::domains::shader_perf::ShaderPerfDomain::build_corpus()
                    .into_iter()
                    .map(|e| {
                        (
                            e.rule_id().to_string(),
                            e.buggy_code().to_string(),
                            e.fixed_code().to_string(),
                        )
                    })
                    .collect::<Vec<_>>()
            }),
            RuleIndex::build_with
        );
        #[cfg(not(feature = "shader_perf"))]
        let shader_perf = None;

        if router_domains.is_empty() {
            // Single-domain-less feature combos (latent_retrieval alone):
            // the config + overlay are only consumed inside `index_domain!`
            // — reference them so the no-domain build stays warning-clean.
            let _ = (&config, &overlay_for);
            return Err(
                "LatentMatcher: no domains enabled (need at least one of clippy_lints/rust_perf/kernel_opt/sec/docker/dist/shader_perf)"
                    .to_string(),
            );
        }

        #[cfg(feature = "set_rerank")]
        let rule_latents = {
            let entries: Vec<(String, DomainId, String)> = rule_book
                .iter()
                .map(|(id, info)| (id.clone(), info.domain, info.buggy_code.clone()))
                .collect();
            super::set_rerank::RuleLatentCorpus::build(&entries)
        };

        Ok(Self {
            // Issue 020 verdict (Bench 025): the annotation router uses the
            // token-frequency cosine block (69.3%/68.6% 2-domain/3-domain vs
            // the BLAKE3-baseline's 53.4%/48.3%, at 0.96µs vs 3.31µs per
            // span). GATING stays killed — 70% < the 85% kill-bar — so this
            // remains an annotation; retrieval stays fan-out.
            router: DomainRouter::with_embedding(
                &router_domains,
                RoutingEmbedding::TfCosine {
                    order: SpectralOrder::GlobalFreq,
                },
            ),
            rerank_mode: super::instantiation_rerank::RerankMode::default(),
            clippy,
            perf,
            kernel,
            sec,
            docker,
            dist,
            shader_perf,
            rule_book,
            scratch: super::corpus_index::SuggestScratch::default(),
            #[cfg(feature = "set_rerank")]
            rule_latents,
            #[cfg(feature = "set_rerank")]
            set_scratch: super::set_rerank::MatcherSetScratch::default(),
            #[cfg(feature = "set_rerank")]
            set_cfg: super::set_rerank::SetRerankConfig::default(),
            active_exclusions: crate::active_set::ActiveSet::empty(),
            #[cfg(feature = "demonstrability_gate")]
            depth_policy: crate::demonstrability_gate::DepthPolicy::identity(),
        })
    }

    /// katgpt-rs Issue 882 P0 rider: set the differential-anchor strength λ
    /// on EVERY enabled domain's index (each carries its own mean-corpus
    /// anchor). `0.0` is the uncorrected path, byte-identical.
    #[cfg(feature = "differential_anchor")]
    #[must_use]
    pub fn with_differential_anchor(mut self, lambda: f32) -> Self {
        for index in [
            &mut self.clippy,
            &mut self.perf,
            &mut self.sec,
            &mut self.docker,
            &mut self.dist,
            &mut self.shader_perf,
        ]
        .into_iter()
        .flatten()
        {
            index.set_differential_anchor(lambda);
        }
        // Kernel dispatches through [`KernelIndex`] (flat or route-narrowed,
        // Issue 196) — the rider forwards to the flat index's own anchor or
        // to every slice + tail (the per-domain rider semantics).
        #[cfg(feature = "kernel_opt")]
        if let Some(kernel_index) = self.kernel.as_mut() {
            kernel_index.set_differential_anchor(lambda);
        }
        self
    }

    /// Look up a suggested rule's display info (buggy + fixed patterns).
    ///
    /// `--suggest` prints the `fixed_code` snippet for manual review
    /// (Plan 005 T2.3); `--fix` (Phase 4/5) will consume the same book.
    #[must_use]
    /// Set the instantiation re-rank policy — the Plan 005 T3.5.4 A/B knob.
    /// Defaults to
    /// [`RerankMode::Structural`](super::instantiation_rerank::RerankMode::Structural)
    /// — promoted on the T3.5.5 uncontaminated measurement (+12.0pp top-1).
    pub fn with_rerank_mode(mut self, mode: super::instantiation_rerank::RerankMode) -> Self {
        self.rerank_mode = mode;
        self
    }

    /// Issue 102 T2/T3: attach a corpus active-set — the operator marker's
    /// demotions, dropped from every similarity fan-out below. Default is
    /// [`crate::active_set::ActiveSet::empty`] (byte-identical posture); a
    /// set loaded from the marker file (`crate::active_set::ACTIVE_SET_FILE`
    /// under the repo root) is the ONLY way anything demotes — the
    /// fade-predictive gate stays closed.
    #[must_use]
    pub fn with_active_set(mut self, set: crate::active_set::ActiveSet) -> Self {
        self.active_exclusions = set;
        self
    }

    /// Read-only view of the attached active set (what is demoted, what
    /// caps are armed) — for callers and tests that need to know without
    /// re-reading the marker file.
    #[must_use]
    pub fn active_exclusions(&self) -> &crate::active_set::ActiveSet {
        &self.active_exclusions
    }

    /// Issue 134 T2: attach a pool-depth allocation policy — the
    /// demonstrability-scaled per-domain factors over the retrieval pool and
    /// the truncation K in [`Self::suggest_span`]. Default is identity
    /// (byte-identical posture, the A/B control); an armed policy is built
    /// either explicitly here (the A/B override — NOT env-guarded) or through
    /// [`DepthPolicy::standing`](crate::demonstrability_gate::DepthPolicy::standing)
    /// (the auto-arm path, guarded by `RIIR_REFINE_DEMONSTRABILITY=0`).
    #[cfg(feature = "demonstrability_gate")]
    #[must_use]
    pub fn with_depth_policy(mut self, policy: crate::demonstrability_gate::DepthPolicy) -> Self {
        self.depth_policy = policy;
        self
    }

    /// Issue 134 T2: read-only view of the attached depth policy (the
    /// wiring tests' identity pin reads it without re-deriving).
    #[cfg(feature = "demonstrability_gate")]
    #[must_use]
    pub fn depth_policy(&self) -> &crate::demonstrability_gate::DepthPolicy {
        &self.depth_policy
    }

    pub fn rule_book(&self, rule_id: &str) -> Option<&RuleInfo> {
        self.rule_book.get(rule_id)
    }

    /// The full rule book (id → buggy/fixed/domain) — Issue 126's trainer
    /// reads THIS as its one corpus source (the same enumeration the matcher
    /// indexes; never a second copy that can drift from it).
    #[must_use]
    pub fn rule_book_map(&self) -> &std::collections::HashMap<String, RuleInfo> {
        &self.rule_book
    }

    /// Attach the frozen two-tower tables + the trained-term weight to every
    /// built domain's index — the Issue-126 A/B posture knob (opt-in
    /// feature). Absent ψ rows degrade per-rule to the untrained baseline.
    #[cfg(feature = "rule_embed_poc")]
    #[must_use]
    pub fn with_rule_embed(
        mut self,
        tables: &std::sync::Arc<super::rule_embed::TrainedTables>,
        weight: f32,
    ) -> Self {
        for index in [
            &mut self.clippy,
            &mut self.perf,
            &mut self.sec,
            &mut self.docker,
            &mut self.dist,
            &mut self.shader_perf,
        ]
        .into_iter()
        .flatten()
        {
            index.set_rule_embed(tables, weight);
        }
        // Kernel dispatches through [`KernelIndex`] — the trained term is a
        // FLAT-index rider: the narrowed posture (Issue 196) does not apply
        // it (the disclosed interaction, route_narrow module docs).
        #[cfg(feature = "kernel_opt")]
        if let Some(kernel_index) = self.kernel.as_mut() {
            kernel_index.set_rule_embed(tables, weight);
        }
        self
    }

    /// Issue 121: the embedded rule corpus (pool-latent lookups + recovery
    /// snap targets). Feature-gated with the set-rerank arm.
    #[cfg(feature = "set_rerank")]
    #[must_use]
    pub fn rule_latents(&self) -> &super::set_rerank::RuleLatentCorpus {
        &self.rule_latents
    }

    /// Issue 121: certify an arbitrary delivered slate (rule ids) with the
    /// same exact-Vendi kernel the admission arm uses — the eval axis for
    /// BASELINE slates the A/B compares against. Measurement only.
    #[cfg(feature = "set_rerank")]
    #[must_use]
    pub fn certify_slate_ids(&self, ids: &[&str]) -> katgpt_core::set_admission::CertificateReport {
        let latents: Vec<[f32; katgpt_core::set_admission::DIM]> = ids
            .iter()
            .filter_map(|id| self.rule_latents.latent_of(self.set_cfg.latent, id))
            .collect();
        katgpt_core::set_admission::certify_set(&self.set_cfg.admission, &latents)
    }

    /// The domains this matcher can route to.
    #[must_use]
    pub fn domains(&self) -> &[DomainId] {
        self.router.domains()
    }

    /// Whether a domain's index is built (feature-gated at construction).
    #[must_use]
    pub fn has_domain(&self, domain: DomainId) -> bool {
        match domain {
            DomainId::Clippy => self.clippy.is_some(),
            DomainId::Perf => self.perf.is_some(),
            DomainId::Kernel => self.kernel.is_some(),
            DomainId::Sec => self.sec.is_some(),
            DomainId::Docker => self.docker.is_some(),
            DomainId::Dist => self.dist.is_some(),
            DomainId::ShaderPerf => self.shader_perf.is_some(),
        }
    }

    /// G4 observability: the scratch buffers' current capacities (stability
    /// across queries is the alloc-free steady-state signal).
    #[must_use]
    pub fn scratch_capacities(&self) -> (usize, usize) {
        (
            self.scratch.retrieved_capacity(),
            self.scratch.ranked_capacity(),
        )
    }

    /// Suggest rules for a single code span (already-chunked text).
    ///
    /// **Fan-out retrieval, not gated** (Phase 1 empirical finding, re-gated
    /// by the Issue 020 POC / Bench 025): the best token-frequency routing
    /// variant reaches ~70% annotation accuracy — well under the 85% bar a
    /// gate would need — so retrieval queries EVERY enabled domain and the
    /// router verdict is returned as an annotation (now via the TF-cosine
    /// block, the measured-best variant). A semantic-signal router (768-D
    /// embedder or similar) is the only plausible future re-gate path; the
    /// lexical branch is measured closed.
    ///
    /// Multi-scale per domain: whole span + per-line queries.
    ///
    /// Takes `&mut self` — the internal G4 scratch is reused across calls.
    pub fn suggest_span(
        &mut self,
        code: &str,
        top_k: usize,
    ) -> (DomainId, Vec<(DomainId, Vec<ScoredRule>)>) {
        self.suggest_span_gated(code, top_k, true)
    }

    /// [`Self::suggest_span`] with the kernel domain's participation decided
    /// by the caller (the kernel pregate's per-span verdict, Issue 208 /
    /// Bench 131). `kernel_allowed == true` is byte-identical to
    /// [`Self::suggest_span`]; `false` skips the kernel index ENTIRELY —
    /// the kernel-disabled `by_domain` shape — every other domain untouched.
    fn suggest_span_gated(
        &mut self,
        code: &str,
        top_k: usize,
        kernel_allowed: bool,
    ) -> (DomainId, Vec<(DomainId, Vec<ScoredRule>)>) {
        let routed = self.router.route(code).unwrap_or(DomainId::Clippy);
        let by_domain = self.suggest_all_domains(code, top_k, kernel_allowed);
        (routed, by_domain)
    }

    /// Retrieve `top_k` rules from EVERY enabled domain's index (fan-out),
    /// reusing the shared G4 scratch across domains. Field-direct index
    /// access keeps the borrow checker happy alongside the scratch borrow.
    fn suggest_all_domains(
        &mut self,
        code: &str,
        top_k: usize,
        kernel_allowed: bool,
    ) -> Vec<(DomainId, Vec<ScoredRule>)> {
        // One parse per span, shared across every domain's instantiation
        // probe (Issue 063): the parse-fail contract — no parse, no marking,
        // every flag stays `false` — is preserved by skipping the marking
        // call entirely when `code` does not parse. `Off` mode needs no AST
        // at all (the per-domain fn's own early return), so it skips the
        // parse too.
        let parsed = if self.rerank_mode == super::instantiation_rerank::RerankMode::Off {
            None
        } else {
            syn::parse_file(code).ok()
        };
        let mut out = Vec::with_capacity(DomainId::ALL.len());
        for domain in DomainId::ALL {
            // Issue 134 T2 (cfg(demonstrability_gate)): scale the
            // fan-out width by the domain's measured demonstrability
            // factor — identity returns the base EXACTLY (the
            // byte-identity fast path; the default posture).
            // Retrieve a POOL deeper than the caller's top_k: re-ranking
            // can only promote a rule that was retrieved, and the residual
            // T3.5.2 misses sat at rank 2-3 (T3.5.4).
            #[cfg(feature = "demonstrability_gate")]
            let mut pool = top_k.max(super::instantiation_rerank::RERANK_POOL);
            #[cfg(not(feature = "demonstrability_gate"))]
            let pool = top_k.max(super::instantiation_rerank::RERANK_POOL);
            #[cfg(feature = "demonstrability_gate")]
            {
                pool = self.depth_policy.scale(domain.label(), pool);
            }
            // Field-direct index access (not a helper) so the immutable
            // index borrow + the mutable scratch borrow stay provably
            // disjoint. Kernel dispatches through [`KernelIndex`] — flat or
            // route-narrowed (Issue 196) — with the same scratch.
            let mut rules = if domain == DomainId::Kernel {
                // Issue 208 / Bench 131: the kernel pregate — a denied span
                // skips the kernel index ENTIRELY (no `by_domain` entry: the
                // kernel-disabled shape, the same one `None => continue`
                // produces below). Every other domain is untouched. The
                // pregate resolves per FILE (`KernelPlan::for_file` in
                // [`Self::suggest_file_inner`]); this is its per-span verdict.
                if !kernel_allowed {
                    continue;
                }
                match self.kernel.as_ref() {
                    Some(kernel_index) => kernel_index
                        .suggest_deep_into(code, pool, &mut self.scratch)
                        .to_vec(),
                    None => continue,
                }
            } else {
                let index = match domain {
                    DomainId::Clippy => self.clippy.as_ref(),
                    DomainId::Perf => self.perf.as_ref(),
                    DomainId::Sec => self.sec.as_ref(),
                    DomainId::Docker => self.docker.as_ref(),
                    DomainId::Dist => self.dist.as_ref(),
                    DomainId::ShaderPerf => self.shader_perf.as_ref(),
                    DomainId::Kernel => unreachable!("handled above"),
                };
                let Some(index) = index else {
                    continue;
                };
                index
                    .suggest_deep_into(code, pool, &mut self.scratch)
                    .to_vec()
            };
            // Issue 102 T2/T3: the active-set honor — demoted rules
            // leave the fan-out BEFORE instantiation marking + rerank +
            // truncate, so the surviving pool fills top_k. Oracle
            // ANCHORS pass through untouched (`push_anchor` reads the
            // rule book, not this list): an anchor is real-clippy
            // ground truth, and demotion is a similarity-fan-out
            // lifecycle operation, never an oracle override — the fix
            // lane's proposer honor + auto re-admit carry the demotion
            // there (coordinator-wired). The is_empty guard keeps the
            // default posture byte-identical (zero work, zero
            // reallocation).
            if !self.active_exclusions.is_empty() {
                let key = active_set_domain_key(domain);
                rules.retain(|r| !self.active_exclusions.excludes(key, &r.rule_id));
            }
            if let Some(file) = &parsed {
                super::instantiation_rerank::mark_instantiation_with_file(
                    domain,
                    code,
                    file,
                    &mut rules,
                    self.rerank_mode,
                );
            }
            // No parse (or Off): no evidence in any domain — identical to
            // the per-domain `mark_instantiation` short-circuit.
            super::instantiation_rerank::rerank(&mut rules);
            // Issue 134 T2 (cfg(demonstrability_gate)): the truncation K
            // scales by the SAME factor as the pool above (a max is
            // preserved under positive scaling, so pool >= k holds); K is
            // clamped to >= 1 — a depth is never zeroed.
            #[cfg(feature = "demonstrability_gate")]
            let top_k = self.depth_policy.scale(domain.label(), top_k).min(pool);
            rules.truncate(top_k);
            out.push((domain, rules));
        }
        out
    }

    /// Issue 036 T1 surface: the pre-rerank retrieval pool AND the
    /// post-rerank ranking for one domain, replicating `suggest_all_domains`'
    /// per-domain steps on a copy. Read-only with respect to every existing
    /// entry point — the ranked copy is a fresh `Vec`, never the truncated
    /// `top_k` slice the suggest path returns.
    ///
    /// `None` when the domain is not enabled.
    pub fn pool_and_ranked(
        &mut self,
        domain: DomainId,
        code: &str,
    ) -> Option<(Vec<ScoredRule>, Vec<ScoredRule>)> {
        // Kernel dispatches through [`KernelIndex`] — flat or route-narrowed
        // (Issue 196); the other six are flat `RuleIndex` fields. Field-
        // direct access keeps the index borrow disjoint from the scratch.
        let mut pool = if domain == DomainId::Kernel {
            self.kernel
                .as_ref()?
                .suggest_deep_into(
                    code,
                    super::instantiation_rerank::RERANK_POOL,
                    &mut self.scratch,
                )
                .to_vec()
        } else {
            let index = match domain {
                DomainId::Clippy => self.clippy.as_ref(),
                DomainId::Perf => self.perf.as_ref(),
                DomainId::Sec => self.sec.as_ref(),
                DomainId::Docker => self.docker.as_ref(),
                DomainId::Dist => self.dist.as_ref(),
                DomainId::ShaderPerf => self.shader_perf.as_ref(),
                DomainId::Kernel => unreachable!("handled above"),
            }?;
            index
                .suggest_deep_into(
                    code,
                    super::instantiation_rerank::RERANK_POOL,
                    &mut self.scratch,
                )
                .to_vec()
        };
        // Issue 102 T2/T3: same honor as `suggest_all_domains` — the
        // retrieved pool drops demoted rules before the ranked copy, so
        // pool + ranked agree on what the fan-out serves.
        if !self.active_exclusions.is_empty() {
            let key = active_set_domain_key(domain);
            pool.retain(|r| !self.active_exclusions.excludes(key, &r.rule_id));
        }
        let mut ranked = pool.clone();
        super::instantiation_rerank::mark_instantiation(
            domain,
            code,
            &mut ranked,
            self.rerank_mode,
        );
        super::instantiation_rerank::rerank(&mut ranked);
        Some((pool, ranked))
    }

    /// Issue 121 (katgpt-rs Plan 599 T4.2): the counter-anchored SET slate
    /// for one span — fan out at `RERANK_POOL` depth, merge the per-domain
    /// pools, admit `k` under alignment × diversity × groundedness, certify
    /// (exact Vendi), and re-fan the θ-ladder when the certificate flags
    /// collapse. `detect_collapse` stays the eval axis — nothing here reads
    /// its verdict.
    ///
    /// Kill-switch: `RIIR_CLIPPY_NO_SET_RERANK=1` returns the BASELINE
    /// merged order with its certificate (the same-binary A/B control).
    #[cfg(feature = "set_rerank")]
    pub fn suggest_set_slate(&mut self, code: &str, k: usize) -> super::set_rerank::SetSlate {
        use riir_rag::Embedder as _;
        let (_, by_domain) = self.suggest_span(code, super::instantiation_rerank::RERANK_POOL);
        let corpus = &self.rule_latents;
        let cfg = self.set_cfg;
        let latents = &corpus.latents;
        let ids = &corpus.rule_ids;
        let lookup = move |id: &str| {
            ids.binary_search_by(|p| p.as_str().cmp(id))
                .ok()
                .map(|i| cfg.latent.project(latents[i]))
        };
        let query = riir_rag::ModellessEmbedder.embed(code);
        let killed = std::env::var(super::set_rerank::NO_SET_RERANK_ENV).is_ok();
        if killed {
            super::set_rerank::baseline_slate(&cfg, &by_domain, lookup, k)
        } else {
            let super::set_rerank::MatcherSetScratch { admission, fan } = &mut self.set_scratch;
            super::set_rerank::counter_anchored_slate(
                &cfg, &by_domain, lookup, corpus, query, k, admission, fan,
            )
        }
    }

    /// Issue 121: override the set-rerank weights (the A/B knob).
    #[cfg(feature = "set_rerank")]
    #[must_use]
    pub fn with_set_config(mut self, cfg: super::set_rerank::SetRerankConfig) -> Self {
        self.set_cfg = cfg;
        self
    }

    /// Suggest rules for every chunk of a real `.rs` file.
    ///
    /// Uses [`AstChunker`] (fn/impl/struct spans with 1-based line ranges).
    /// Order: file order. Retrieval fans out across all enabled domains
    /// (see [`Self::suggest_span`]); the router verdict annotates each span.
    /// Takes `&mut self` — the G4 scratch is reused across every span.
    pub fn suggest_file(&mut self, src: &str, path: &str, top_k: usize) -> Vec<SpanSuggestion> {
        self.suggest_file_inner(src, path, top_k, None, None)
    }

    /// [`Self::suggest_file`] with oracle anchoring (the clippy-fed mode,
    /// 2026-09-13 retrieval-miss intake): every anchor's lint is
    /// force-included on the span containing its line — the rule book
    /// decides `anchored_rules` (rule exists, domain known) vs
    /// `anchored_missing` (gap signal) — and every span of the file with
    /// no anchor is marked `heuristic_only`. Ranking itself is untouched:
    /// anchoring is additive recall, never a re-ranker.
    ///
    /// Anchors in CHUNK GAPS get a synthesized line-span: not every line
    /// sits inside an item chunk (blank separators, module-level spans,
    /// macro-generated regions), so a real diagnostic can fall in NO chunk;
    /// an oracle finding is never dropped for a coverage gap. (The
    /// 2026-09-13 fn-tail gap — chunk ends scanned from the NORMALIZED
    /// signature length — was measured here and fixed at the substrate,
    /// riir-rag Issue 942; gap synthesis remains correct behavior for the
    /// legitimate gap classes.) Synthetic spans carry no retrieval
    /// (`by_domain` empty) — the oracle IS the signal.
    pub fn suggest_file_anchored(
        &mut self,
        src: &str,
        path: &str,
        top_k: usize,
        anchors: &[ClippyDiagnostic],
    ) -> Vec<SpanSuggestion> {
        self.suggest_file_inner(src, path, top_k, Some(anchors), None)
    }

    /// Issue 146 T0.2: [`Self::suggest_file_anchored`] with the hotpath
    /// oracle beside the clippy one. Hot spans (measured at runtime, keyed
    /// to this tree by the report's git SHA) attach to the chunk containing
    /// their line — same mapping machinery as clippy anchors, including
    /// gap-span synthesis (an oracle finding is never dropped for a chunk
    /// coverage gap; a synthetic hot span routes to [`DomainId::Perf`], the
    /// clippy-synthetic `DomainId::Clippy` precedent — hotness IS a perf
    /// signal). Either feed may be empty: the clippy-only call is exactly
    /// [`Self::suggest_file_anchored`], and a hot-only call anchors spans
    /// the profile measured with no lint knowledge at all. Anchoring is
    /// ADDITIVE recall — ranking itself is untouched (the hot spans' own
    /// order, by `time_share_ns`, is a display/scheduling concern of the
    /// caller, not a re-rank).
    pub fn suggest_file_hot_anchored(
        &mut self,
        src: &str,
        path: &str,
        top_k: usize,
        anchors: &[ClippyDiagnostic],
        hot: &[crate::hotpath_report::HotSpan],
    ) -> Vec<SpanSuggestion> {
        self.suggest_file_inner(src, path, top_k, Some(anchors), Some(hot))
    }

    /// The shared file walk. `anchors` (when `Some`) carries this file's
    /// real clippy findings; `hot` (when `Some`) carries this file's
    /// measured-hot spans; `None` is the plain feed-less path.
    fn suggest_file_inner(
        &mut self,
        src: &str,
        path: &str,
        top_k: usize,
        anchors: Option<&[ClippyDiagnostic]>,
        hot: Option<&[crate::hotpath_report::HotSpan]>,
    ) -> Vec<SpanSuggestion> {
        let chunker = AstChunker;
        let chunks = chunker.chunk(src, path);
        // Issue 208 / Bench 131: the kernel pregate — ONE signal read per
        // file (the run-level index the walk builds as it reads), resolved
        // to a per-chunk verdict against the chunk's line range. `Off` (the
        // feature-off posture and the explicit `--domain kernel` bypass)
        // allows every span: byte-identical pre-gate behavior.
        let kernel_plan = crate::kernel_pregate::KernelPlan::for_file(src, path);
        let mut out = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let kernel_allowed =
                kernel_plan.span_allowed(chunk.source.start_line, chunk.source.end_line);
            let (routed_domain, by_domain) =
                self.suggest_span_gated(&chunk.content, top_k, kernel_allowed);
            out.push(SpanSuggestion {
                path: chunk.source.path.clone(),
                start_line: chunk.source.start_line,
                end_line: chunk.source.end_line,
                routed_domain,
                by_domain,
                anchored_rules: Vec::new(),
                anchored_missing: Vec::new(),
                heuristic_only: false,
                hot: Vec::new(),
            });
        }
        let Some(anchors) = anchors else {
            // The clippy-less path still anchors hot spans when present
            // (hot-only feed: `Some(&[])` for clippy).
            if let Some(hot) = hot {
                attach_hot_spans(&mut out, path, hot);
                for span in &mut out {
                    span.heuristic_only = span.hot.is_empty();
                }
            }
            return out;
        };
        for anchor in anchors {
            // Chunks are disjoint + file-ordered, so the first containing
            // span is THE span; `position` over a linear scan is honest at
            // file scale (hundreds of chunks, not a hot path).
            if let Some(span) = out
                .iter_mut()
                .find(|s| anchor.line >= s.start_line && anchor.line <= s.end_line)
            {
                push_anchor(span, &self.rule_book, anchor);
            } else {
                // The diagnostic falls in a CHUNK GAP (a legitimately
                // unchunked line — see `suggest_file_anchored`):
                // synthesize a line-span carrying the anchor, inserted in
                // line order.
                let mut synthetic = SpanSuggestion {
                    path: path.to_string(),
                    start_line: anchor.line,
                    end_line: anchor.line,
                    // A clippy-fed anchor is a clippy-domain finding by
                    // construction; the [oracle] marker carries the truth.
                    routed_domain: DomainId::Clippy,
                    by_domain: Vec::new(),
                    anchored_rules: Vec::new(),
                    anchored_missing: Vec::new(),
                    heuristic_only: false,
                    hot: Vec::new(),
                };
                push_anchor(&mut synthetic, &self.rule_book, anchor);
                let pos = out
                    .iter()
                    .position(|s| s.start_line > anchor.line)
                    .unwrap_or(out.len());
                out.insert(pos, synthetic);
            }
        }
        if let Some(hot) = hot {
            attach_hot_spans(&mut out, path, hot);
        }
        for span in &mut out {
            span.heuristic_only = span.anchored_rules.is_empty()
                && span.anchored_missing.is_empty()
                && span.hot.is_empty();
        }
        out
    }
}

/// Attach the file's measured-hot spans to their containing chunks
/// (Issue 146 T0.2 — the clippy-anchor mapping machinery applied to the
/// profile's entries). A hot line falling in a CHUNK GAP synthesizes a
/// line-span routed to [`DomainId::Perf`] — hotness is a perf signal by
/// construction (the `DomainId::Clippy` synthetic precedent) — inserted in
/// line order. Multiple entries may cover one chunk (nested/adjacent
/// instrumented items): each attaches; the span's `hot` vec carries them
/// in the feed's ranked order (time share desc).
fn attach_hot_spans(
    out: &mut Vec<SpanSuggestion>,
    path: &str,
    hot: &[crate::hotpath_report::HotSpan],
) {
    for hs in hot {
        let line = usize::try_from(hs.line).unwrap_or(usize::MAX);
        if let Some(span) = out
            .iter_mut()
            .find(|s| line >= s.start_line && line <= s.end_line)
        {
            span.hot.push(hs.clone());
        } else {
            let synthetic = SpanSuggestion {
                path: path.to_string(),
                start_line: line,
                end_line: line,
                routed_domain: DomainId::Perf,
                by_domain: Vec::new(),
                anchored_rules: Vec::new(),
                anchored_missing: Vec::new(),
                heuristic_only: false,
                hot: vec![hs.clone()],
            };
            let pos = out
                .iter()
                .position(|s| s.start_line > line)
                .unwrap_or(out.len());
            out.insert(pos, synthetic);
        }
    }
}

/// The active-set domain key for a [`DomainId`] — the marker file's
/// vocabulary (Issue 102 T2/T3). One translation point, delegating to
/// [`DomainId::label`] so the marker keys cannot drift from the router's
/// labels; the clippy spelling is pinned against
/// [`crate::active_set::CLIPPY_DOMAIN_KEY`] by the test
/// `active_set_domain_key_matches_the_marker_vocabulary`.
fn active_set_domain_key(domain: DomainId) -> &'static str {
    domain.label()
}

/// Attach one anchor's lint to a span: rule-book hit → `anchored_rules`
/// (domain from the book), miss → `anchored_missing` (the corpus-gap
/// signal). Same-lint duplicates anchor once.
fn push_anchor(
    span: &mut SpanSuggestion,
    rule_book: &std::collections::HashMap<String, RuleInfo>,
    anchor: &ClippyDiagnostic,
) {
    if let Some(info) = rule_book.get(&anchor.lint) {
        if !span.anchored_rules.iter().any(|(_, id)| *id == anchor.lint) {
            span.anchored_rules.push((info.domain, anchor.lint.clone()));
        }
    } else if !span.anchored_missing.contains(&anchor.lint) {
        span.anchored_missing.push(anchor.lint.clone());
    }
}

// Issue 090: these tests need clippy_lints AND rust_perf — the overlay
// test asserts the uncovered PERF seed stands, the fixtures retrieve
// clippy corpus rules, and `matcher_builds_from_enabled_domains` asserts
// >= 2 domains. A latent-only build (the matcher refuses a domain-less
// build) correctly drops them.
#[cfg(all(
    test,
    feature = "latent_retrieval",
    feature = "clippy_lints",
    feature = "rust_perf"
))]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
pub fn area(w: f64, h: f64) -> f64 {
    return w * h;
}

pub fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {
    for u in users {
        if u.id == id {
            return Some(u);
        }
    }
    None
}
";

    #[test]
    fn matcher_builds_from_enabled_domains() {
        // Default features: clippy_lints + rust_perf → 2 domains.
        let m = LatentMatcher::from_enabled_domains().expect("build");
        assert!(
            m.domains().len() >= 2,
            "expected ≥2 enabled domains, got {:?}",
            m.domains()
        );
    }

    #[test]
    fn overlay_replaces_the_covered_domain_and_keeps_the_rest() {
        // T1.6 (Plan 040): a leased epoch is a full-corpus state per
        // COVERED domain — the overlay replaces clippy's seed entirely
        // (the leased rule is retrievable, a seed rule is NOT), while the
        // uncovered perf domain keeps its compiled-in seed.
        let overlay = vec![(
            DomainId::Clippy,
            vec![(
                "lease-fixture-rule".to_string(),
                "fn f() -> u8 { return 7; }".to_string(),
                "fn f() -> u8 { 7 }".to_string(),
            )],
        )];
        let mut m = LatentMatcher::from_enabled_domains_with_overlay(&overlay).expect("build");
        let (_, by_domain) = m.suggest_span("fn f() -> u8 { return 7; }", 3);
        let clippy = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .expect("clippy index enabled");
        let ids: Vec<&str> = clippy.1.iter().map(|r| r.rule_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["lease-fixture-rule"],
            "the overlay REPLACES the seed — exactly the leased rule is retrievable"
        );
        assert_eq!(
            m.rule_book("lease-fixture-rule")
                .expect("in book")
                .fixed_code,
            "fn f() -> u8 { 7 }"
        );
        // The uncovered domain keeps its seed.
        let perf = by_domain.iter().find(|(d, _)| *d == DomainId::Perf);
        assert!(
            perf.is_some_and(|(_, rules)| !rules.is_empty()),
            "perf seed stands"
        );
    }

    #[test]
    fn empty_overlay_entries_make_no_statement() {
        // An EMPTY entries list is "no statement" — the seed stands (the
        // pack contract: empty ≠ remove-everything).
        let overlay = vec![(DomainId::Clippy, vec![])];
        let mut m = LatentMatcher::from_enabled_domains_with_overlay(&overlay).expect("build");
        let (_, by_domain) = m.suggest_span("fn f() -> u8 { return 7; }", 3);
        let clippy = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .expect("clippy index enabled");
        assert!(
            clippy
                .1
                .iter()
                .take(3)
                .any(|r| r.rule_id == "needless_return"),
            "the seed corpus still serves the span"
        );
    }

    #[test]
    fn span_suggestion_routes_and_retrieves() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let (routed, by_domain) = m.suggest_span("fn f() -> u8 { return 7; }", 3);
        assert_ne!(by_domain, [] as [(DomainId, std::vec::Vec<ScoredRule>); 0]);
        // Fan-out: the clippy domain's list must exist + top-1 needless_return
        // (empirically the routed annotation may differ — see the mis-routing
        // finding in suggest_span's doc).
        let clippy = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .expect("clippy index enabled by default");
        assert_ne!(clippy.1, [] as [ScoredRule; 0]);
        // Top-3 membership, not top-1: corpus growth shifts rankings (the
        // Issue 016 expansion 10→40 entries made `manual_unwrap_or`'s fixture
        // — `fn f(x: Option<u8>) -> u8 { match .. 7 }` — a stronger BM25
        // token match for this query than needless_return's `fn foo() -> i32
        // { return 42; }`). Precision@1 over curated fixtures is the Phase 3
        // G1 gate's job (Bench 018); this test pins routing + retrievability.
        assert!(
            clippy
                .1
                .iter()
                .take(3)
                .any(|r| r.rule_id == "needless_return"),
            "needless_return not in top-3: {:?}",
            clippy
                .1
                .iter()
                .map(|r| r.rule_id.clone())
                .collect::<Vec<_>>()
        );
        let _ = routed;
    }

    /// Issue 121 (Bench 096): the merge MUST carry the per-domain rerank's
    /// structural key — a fired proposer outranks a higher-BM25 rival from
    /// another domain, exactly as `instantiation_rerank::rerank` orders
    /// within one domain. The score-only merge measured -55.6pp top-1 on
    /// the composite rig.
    #[test]
    fn all_rules_keeps_the_canonical_key_across_domains() {
        use crate::draft::ScoredRule;
        let mk = |id: &str, score: f32, fires: bool| ScoredRule {
            rule_id: id.to_string(),
            score,
            instantiates: fires,
        };
        let s = SpanSuggestion {
            path: "x.rs".to_string(),
            start_line: 1,
            end_line: 2,
            routed_domain: DomainId::Clippy,
            by_domain: vec![
                (
                    DomainId::Clippy,
                    vec![
                        mk("needless_return", 8.0, true),
                        mk("needless_bool", 8.5, false),
                    ],
                ),
                (DomainId::Perf, vec![mk("vec-with-capacity", 9.0, false)]),
            ],
            anchored_rules: Vec::new(),
            anchored_missing: Vec::new(),
            heuristic_only: false,
            hot: Vec::new(),
        };
        let order: Vec<&str> = s.all_rules().iter().map(|r| r.rule_id.as_str()).collect();
        assert_eq!(
            order,
            vec!["needless_return", "vec-with-capacity", "needless_bool"],
            "the fired proposer must outrank higher-BM25 rivals across domains"
        );
    }

    #[test]
    fn file_suggestion_covers_all_chunks_with_lines() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file(SAMPLE, "sample.rs", 3);
        assert!(
            sugg.len() >= 2,
            "expected ≥2 chunks (2 fns), got {}",
            sugg.len()
        );
        for s in &sugg {
            assert_eq!(s.path, "sample.rs");
            assert!(s.start_line >= 1 && s.end_line >= s.start_line);
            assert_ne!(
                s.by_domain,
                [] as [(DomainId, std::vec::Vec<ScoredRule>); 0]
            );
            assert!(
                !s.all_rules().is_empty(),
                "chunk {} suggested no rules",
                s.start_line
            );
        }
        // Fan-out: the needless_return-shaped chunk must surface that rule
        // via the CLIPPY domain list (in top-3 — recall; precision@1 over a
        // real fixture is the Phase 3 G1 gate's job).
        let ret = sugg
            .iter()
            .find(|s| {
                s.by_domain.iter().any(|(d, rules)| {
                    *d == DomainId::Clippy && rules.iter().any(|r| r.rule_id == "needless_return")
                })
            })
            .expect("some chunk must surface needless_return via the clippy list");
        assert_eq!(ret.start_line, 1);
    }

    // ── the clippy-fed anchored mode (2026-09-13 retrieval-miss intake) ──

    fn anchor(
        file: &str,
        line: usize,
        lint_id: &str,
    ) -> super::super::clippy_feed::ClippyDiagnostic {
        super::super::clippy_feed::ClippyDiagnostic {
            span: None,
            file: file.to_string(),
            line,
            lint: lint_id.to_string(),
            suggestion: None,
        }
    }

    #[test]
    fn anchored_feed_force_includes_the_rule_at_its_line() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[anchor("sample.rs", 2, "needless_return")],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 2 && s.end_line >= 2)
            .expect("a chunk covers line 2");
        assert!(
            span.anchored_rules
                .iter()
                .any(|(d, id)| *d == DomainId::Clippy && id == "needless_return"),
            "the oracle finding is force-included regardless of its ranked position"
        );
        assert!(!span.heuristic_only, "the anchored span is not heuristic");
        // The FP datum: a feed-covered file's un-flagged chunks are marked.
        assert!(
            sugg.iter().any(|s| s.heuristic_only),
            "some other chunk carries the heuristic_only marker"
        );
    }

    #[test]
    fn anchored_missing_lints_surface_as_gap_signal() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[anchor("sample.rs", 2, "definitely_not_a_corpus_rule")],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 2 && s.end_line >= 2)
            .expect("a chunk covers line 2");
        assert_eq!(
            span.anchored_missing,
            vec!["definitely_not_a_corpus_rule".to_string()]
        );
        assert_eq!(
            span.anchored_rules,
            [] as [(DomainId, std::string::String); 0]
        );
        assert!(!span.heuristic_only, "a gap signal is still an oracle hit");
    }

    #[test]
    fn plain_suggest_file_leaves_anchor_fields_default() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file(SAMPLE, "sample.rs", 3);
        assert_ne!(sugg, [] as [SpanSuggestion; 0]);
        for s in &sugg {
            assert_eq!(s.anchored_rules, [] as [(DomainId, std::string::String); 0]);
            assert_eq!(s.anchored_missing, [] as [std::string::String; 0]);
            assert!(!s.heuristic_only, "the plain path claims nothing");
        }
    }

    #[test]
    fn duplicate_anchor_lints_in_one_chunk_dedup() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        // Lines 6+7 sit inside the `find_user` chunk (5-12 under the
        // fixed substrate spans, riir-rag Issue 942 — they also sat
        // inside the pre-fix truncated 5-10 chunk).
        let sugg = m.suggest_file_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[
                anchor("sample.rs", 6, "needless_return"),
                anchor("sample.rs", 7, "needless_return"),
            ],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 6 && s.end_line >= 7)
            .expect("a chunk covers both lines");
        assert_eq!(span.anchored_rules.len(), 1, "same lint anchors once");
    }

    #[test]
    fn gap_anchor_gets_a_synthesized_line_span() {
        // Line 4 (the blank line between the fns) is covered by NO chunk
        // — a blank separator is never item-spanned, so it is a pure gap
        // under the fixed substrate spans too (the fn-tail quirk class
        // itself was fixed at the substrate, riir-rag Issue 942). The
        // oracle finding must still surface: a synthesized line-span
        // carrying the anchor, inserted in line order.
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[anchor("sample.rs", 4, "needless_return")],
        );
        let pos = sugg
            .iter()
            .position(|s| s.start_line == 4 && s.end_line == 4)
            .expect("synthesized line-span at the gap");
        let span = &sugg[pos];
        assert!(span
            .anchored_rules
            .iter()
            .any(|(d, id)| *d == DomainId::Clippy && id == "needless_return"));
        assert!(
            span.by_domain.is_empty(),
            "no retrieval on a synthetic span"
        );
        assert!(!span.heuristic_only);
        // Line-order insertion: every earlier span starts before 4, every
        // later one starts after 4.
        for (i, s) in sugg.iter().enumerate() {
            if i < pos {
                assert!(s.start_line <= 4);
            } else if i > pos {
                assert!(s.start_line > 4);
            }
        }
    }

    // ── Issue 146 T0.2: the hotpath-fed anchored mode ──────────────────

    fn hot(
        file: &str,
        line: u32,
        name: &str,
        time_share_ns: u64,
    ) -> crate::hotpath_report::HotSpan {
        crate::hotpath_report::HotSpan {
            file: file.to_string(),
            line,
            name: name.to_string(),
            calls: 100,
            avg_ns: Some(50),
            time_share_ns,
            percent_total: Some(10.0),
            alloc_bytes: Some(4096),
            alloc_calls: Some(7),
        }
    }

    #[test]
    fn hot_anchored_marks_the_containing_chunk_and_never_heuristic() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_hot_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[],
            &[hot("sample.rs", 2, "app::area", 5_000)],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 2 && s.end_line >= 2)
            .expect("a chunk covers line 2");
        assert_eq!(span.hot.len(), 1, "the hot mark attaches");
        assert_eq!(span.hot[0].name, "app::area");
        assert_eq!(span.hot[0].alloc_bytes, Some(4096));
        assert!(!span.heuristic_only, "a hot span has an oracle signal");
        // The FP datum mirrors the clippy lane: a feed-covered file's
        // un-marked chunks are heuristic-only.
        assert!(
            sugg.iter().any(|s| s.hot.is_empty() && s.heuristic_only),
            "some other chunk carries the heuristic_only marker"
        );
    }

    #[test]
    fn hot_anchor_in_a_chunk_gap_synthesizes_a_perf_routed_span() {
        // Line 4 (blank separator) sits in NO chunk — the clippy gap
        // precedent: synthesize a line-span, routed to Perf (hotness IS a
        // perf signal; the clippy-synthetic Clippy precedent).
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_hot_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[],
            &[hot("sample.rs", 4, "app::orphan", 1)],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line == 4 && s.end_line == 4)
            .expect("synthesized line-span at the gap");
        assert_eq!(span.routed_domain, DomainId::Perf);
        assert!(
            span.by_domain.is_empty(),
            "no retrieval on a synthetic span"
        );
        assert_eq!(span.hot.len(), 1);
    }

    #[test]
    fn hot_and_clippy_anchors_compose_on_one_span() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_hot_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[anchor("sample.rs", 2, "needless_return")],
            &[hot("sample.rs", 2, "app::area", 5_000)],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 2 && s.end_line >= 2)
            .expect("a chunk covers line 2");
        assert!(span
            .anchored_rules
            .iter()
            .any(|(d, id)| *d == DomainId::Clippy && id == "needless_return"));
        assert_eq!(span.hot.len(), 1, "both oracles attach independently");
        assert!(!span.heuristic_only);
    }

    // ── Issue 102 T2/T3: the active-set fan-out honor ──────────────────

    #[test]
    fn active_set_domain_key_matches_the_marker_vocabulary() {
        // The marker file's domain keys ARE the router label spellings;
        // the clippy key MUST agree with active_set::CLIPPY_DOMAIN_KEY or
        // the auto re-admit (clippy-only) would silently key a domain no
        // marker can name — the demotions would then be un-undoable.
        assert_eq!(
            active_set_domain_key(DomainId::Clippy),
            crate::active_set::CLIPPY_DOMAIN_KEY
        );
        // Every domain translates to a non-empty stable label AND is a
        // key the ungated marker module knows — the drift pin for the
        // KNOWN_DOMAIN_KEYS table (which cannot see `DomainId` from behind
        // its feature gate): a domain added to the router without a row
        // there fails HERE, not silently at the operator CLI.
        for domain in DomainId::ALL {
            let key = active_set_domain_key(domain);
            assert_ne!(key, "");
            assert!(
                crate::active_set::KNOWN_DOMAIN_KEYS.contains(&key),
                "router label {key:?} missing from active_set::KNOWN_DOMAIN_KEYS — add it there (same commit)"
            );
        }
    }

    #[test]
    fn active_set_empty_is_byte_identical() {
        let query = "fn f() -> u8 { return 7; }";
        let mut plain = LatentMatcher::from_enabled_domains().expect("build");
        let mut marked = LatentMatcher::from_enabled_domains()
            .expect("build")
            .with_active_set(crate::active_set::ActiveSet::empty());
        let a = plain.suggest_span(query, 5);
        let b = marked.suggest_span(query, 5);
        assert_eq!(a, b, "an empty active set is the default posture");
        // The pool_and_ranked path agrees too.
        let pa = plain
            .pool_and_ranked(DomainId::Clippy, query)
            .expect("clippy index enabled");
        let pb = marked
            .pool_and_ranked(DomainId::Clippy, query)
            .expect("clippy index enabled");
        assert_eq!(pa, pb);
    }

    #[test]
    fn demoted_rule_exits_the_fanout() {
        let query = "fn f() -> u8 { return 7; }";
        // Retrieve first; demote whatever tops the clippy list, and keep
        // the runners-up as the survival witnesses.
        let mut probe = LatentMatcher::from_enabled_domains().expect("build");
        let (_, by_domain) = probe.suggest_span(query, 3);
        let clippy = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .expect("clippy index enabled");
        let top = clippy
            .1
            .first()
            .expect("clippy serves the span")
            .rule_id
            .clone();
        let others: Vec<String> = clippy.1.iter().skip(1).map(|r| r.rule_id.clone()).collect();
        assert!(!others.is_empty(), "need a runner-up to prove survival");

        let mut set = crate::active_set::ActiveSet::empty();
        set.demote(crate::active_set::CLIPPY_DOMAIN_KEY, &top);
        let mut m = LatentMatcher::from_enabled_domains()
            .expect("build")
            .with_active_set(set);
        assert!(m
            .active_exclusions()
            .excludes(crate::active_set::CLIPPY_DOMAIN_KEY, &top));
        let (_, by_domain) = m.suggest_span(query, 3);
        // The domain ENTRY is not dropped — only the rule is gone, and the
        // freed top slot backfills from deeper ranks.
        let clippy = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .expect("the clippy entry survives demotion of one rule");
        assert!(
            !clippy.1.iter().any(|r| r.rule_id == top),
            "the demoted top rule left the fan-out"
        );
        for other in &others {
            assert!(
                clippy.1.iter().any(|r| &r.rule_id == other),
                "runner-up {other} still served after the demotion"
            );
        }
        // The pool_and_ranked path honors the same set.
        let (pool, ranked) = m.pool_and_ranked(DomainId::Clippy, query).expect("clippy");
        assert!(
            !pool.iter().any(|r| r.rule_id == top),
            "pool drops the demoted rule"
        );
        assert!(
            !ranked.iter().any(|r| r.rule_id == top),
            "ranked copy drops it too"
        );
    }

    #[test]
    fn demotion_never_overrides_oracle_anchors() {
        // The anchor path (`push_anchor`) reads the RULE BOOK, not the
        // active set: an oracle finding is real-clippy ground truth, and
        // demotion is a similarity-fan-out lifecycle operation — never an
        // oracle override. (The fix lane's proposer honor + auto re-admit
        // carry the demotion there; coordinator-wired.)
        let mut set = crate::active_set::ActiveSet::empty();
        set.demote(crate::active_set::CLIPPY_DOMAIN_KEY, "needless_return");
        let mut m = LatentMatcher::from_enabled_domains()
            .expect("build")
            .with_active_set(set);
        let sugg = m.suggest_file_anchored(
            SAMPLE,
            "sample.rs",
            3,
            &[anchor("sample.rs", 2, "needless_return")],
        );
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 2 && s.end_line >= 2)
            .expect("a chunk covers line 2");
        assert!(
            span.anchored_rules
                .iter()
                .any(|(d, id)| *d == DomainId::Clippy && id == "needless_return"),
            "the demoted rule still anchors — oracle truth is not demotable"
        );
        // ...while the same rule left the similarity list of that span.
        assert!(!span
            .by_domain
            .iter()
            .any(|(_, rules)| rules.iter().any(|r| r.rule_id == "needless_return")));
    }
}

// Issue 208 / Bench 131: the kernel pregate's scan-path wiring — the gate
// DENIES the kernel domain on non-signaling files and KEEPS it on signaling
// ones, through the real `suggest_file` walk. Gated: the gate only arms
// under `kernel_pregate`; the tests need the matcher (latent_retrieval) and
// the kernel index (implied).
#[cfg(all(test, feature = "kernel_pregate", feature = "latent_retrieval"))]
mod kernel_pregate_tests {
    use super::*;

    #[test]
    fn the_gate_denies_kernel_on_non_signaling_files() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build matcher");
        let src = "fn main() { println!(\"hi\"); }\nstruct S { a: u32 }\n";
        let spans = m.suggest_file(src, "plain.rs", 3);
        assert!(!spans.is_empty());
        for span in &spans {
            assert!(
                !span.by_domain.iter().any(|(d, _)| *d == DomainId::Kernel),
                "kernel must be ABSENT (the kernel-disabled shape) on a non-signaling file"
            );
        }
    }

    #[test]
    fn the_gate_keeps_kernel_on_signaling_files() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build matcher");
        let src = "use cubecl::prelude::*;\nfn kernel_driver() { }\n";
        let spans = m.suggest_file(src, "gpu.rs", 3);
        assert!(
            spans
                .iter()
                .any(|s| s.by_domain.iter().any(|(d, _)| *d == DomainId::Kernel)),
            "kernel must be PRESENT on a signaling file"
        );
    }

    #[test]
    fn the_public_suggest_span_is_ungated_allow_all() {
        // The public per-span entry keeps the exact pre-gate behavior (the
        // gate is a per-FILE scan-path posture, never a span-API one).
        let mut m = LatentMatcher::from_enabled_domains().expect("build matcher");
        let (_, by_domain) = m.suggest_span("let x = 1;", 3);
        assert!(by_domain.iter().any(|(d, _)| *d == DomainId::Kernel));
    }
}

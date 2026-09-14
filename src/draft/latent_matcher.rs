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

use crate::domain::{HealDomain, HealEntry};

use super::clippy_feed::ClippyDiagnostic;
use super::corpus_index::{RuleIndex, ScoredRule};
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
}

impl SpanSuggestion {
    /// All suggested rules across domains, best-score-first.
    #[must_use]
    pub fn all_rules(&self) -> Vec<&ScoredRule> {
        let mut all: Vec<&ScoredRule> = self
            .by_domain
            .iter()
            .flat_map(|(_, rules)| rules.iter())
            .collect();
        // rule_id tie-break: cross-domain concatenation is deterministic
        // (DomainId::ALL order), but equal scores from DIFFERENT domains
        // must still resolve identically every run — same canonical order
        // as `rank_into`/`rerank`/`canonical_rank`.
        all.sort_unstable_by(|a, b| {
            b.score
                .total_cmp(&a.score)
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
    kernel: Option<RuleIndex>,
    sec: Option<RuleIndex>,
    docker: Option<RuleIndex>,
    dist: Option<RuleIndex>,
    rule_book: std::collections::HashMap<String, RuleInfo>,
    /// G4 scratch — reused across every span + domain query.
    scratch: super::corpus_index::SuggestScratch,
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
    /// [`from_enabled_domains_with_overlay`]: Self::from_enabled_domains_with_overlay
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
        let mut router_domains = Vec::new();
        let mut rule_book = std::collections::HashMap::new();

        macro_rules! index_domain {
            ($var:ident, $domain:expr, $corpus:expr) => {{
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
                Some(RuleIndex::build_with(patterns, config)?)
            }};
        }

        #[cfg(feature = "clippy_lints")]
        let clippy = index_domain!(
            clippy,
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
            })
        );
        #[cfg(not(feature = "clippy_lints"))]
        let clippy = None;

        #[cfg(feature = "rust_perf")]
        let perf = index_domain!(
            perf,
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
            })
        );
        #[cfg(not(feature = "rust_perf"))]
        let perf = None;

        #[cfg(feature = "kernel_heal")]
        let kernel = index_domain!(
            kernel,
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
            })
        );
        #[cfg(not(feature = "kernel_heal"))]
        let kernel = None;

        #[cfg(feature = "sec")]
        let sec = index_domain!(
            sec,
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
            })
        );
        #[cfg(not(feature = "sec"))]
        let sec = None;

        #[cfg(feature = "docker")]
        let docker = index_domain!(
            docker,
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
            })
        );
        #[cfg(not(feature = "docker"))]
        let docker = None;

        #[cfg(feature = "dist")]
        let dist = index_domain!(
            dist,
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
            })
        );
        #[cfg(not(feature = "dist"))]
        let dist = None;

        if router_domains.is_empty() {
            // Single-domain-less feature combos (latent_retrieval alone):
            // the config + overlay are only consumed inside `index_domain!`
            // — reference them so the no-domain build stays warning-clean.
            let _ = (&config, &overlay_for);
            return Err(
                "LatentMatcher: no domains enabled (need at least one of clippy_lints/rust_perf/kernel_heal/sec/docker/dist)"
                    .to_string(),
            );
        }

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
            rule_book,
            scratch: super::corpus_index::SuggestScratch::default(),
        })
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

    pub fn rule_book(&self, rule_id: &str) -> Option<&RuleInfo> {
        self.rule_book.get(rule_id)
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
        let routed = self.router.route(code).unwrap_or(DomainId::Clippy);
        let by_domain = self.suggest_all_domains(code, top_k);
        (routed, by_domain)
    }

    /// Retrieve `top_k` rules from EVERY enabled domain's index (fan-out),
    /// reusing the shared G4 scratch across domains. Field-direct index
    /// access keeps the borrow checker happy alongside the scratch borrow.
    fn suggest_all_domains(
        &mut self,
        code: &str,
        top_k: usize,
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
            // Direct field access (not `index_for`) so the immutable index
            // borrow + the mutable scratch borrow stay provably disjoint.
            let index = match domain {
                DomainId::Clippy => self.clippy.as_ref(),
                DomainId::Perf => self.perf.as_ref(),
                DomainId::Kernel => self.kernel.as_ref(),
                DomainId::Sec => self.sec.as_ref(),
                DomainId::Docker => self.docker.as_ref(),
                DomainId::Dist => self.dist.as_ref(),
            };
            if let Some(index) = index {
                // Retrieve a POOL deeper than the caller's top_k: re-ranking
                // can only promote a rule that was retrieved, and the residual
                // T3.5.2 misses sat at rank 2-3 (T3.5.4).
                let pool = top_k.max(super::instantiation_rerank::RERANK_POOL);
                let mut rules = index
                    .suggest_deep_into(code, pool, &mut self.scratch)
                    .to_vec();
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
                rules.truncate(top_k);
                out.push((domain, rules));
            }
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
        let index = match domain {
            DomainId::Clippy => self.clippy.as_ref(),
            DomainId::Perf => self.perf.as_ref(),
            DomainId::Kernel => self.kernel.as_ref(),
            DomainId::Sec => self.sec.as_ref(),
            DomainId::Docker => self.docker.as_ref(),
            DomainId::Dist => self.dist.as_ref(),
        }?;
        let pool = index
            .suggest_deep_into(
                code,
                super::instantiation_rerank::RERANK_POOL,
                &mut self.scratch,
            )
            .to_vec();
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

    /// Suggest rules for every chunk of a real `.rs` file.
    ///
    /// Uses [`AstChunker`] (fn/impl/struct spans with 1-based line ranges).
    /// Order: file order. Retrieval fans out across all enabled domains
    /// (see [`Self::suggest_span`]); the router verdict annotates each span.
    /// Takes `&mut self` — the G4 scratch is reused across every span.
    pub fn suggest_file(&mut self, src: &str, path: &str, top_k: usize) -> Vec<SpanSuggestion> {
        self.suggest_file_inner(src, path, top_k, None)
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
        self.suggest_file_inner(src, path, top_k, Some(anchors))
    }

    /// The shared file walk. `anchors` (when `Some`) carries this file's
    /// real clippy findings; `None` is the plain feed-less path.
    fn suggest_file_inner(
        &mut self,
        src: &str,
        path: &str,
        top_k: usize,
        anchors: Option<&[ClippyDiagnostic]>,
    ) -> Vec<SpanSuggestion> {
        let chunker = AstChunker;
        let chunks = chunker.chunk(src, path);
        let mut out = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let (routed_domain, by_domain) = self.suggest_span(&chunk.content, top_k);
            out.push(SpanSuggestion {
                path: chunk.source.path.clone(),
                start_line: chunk.source.start_line,
                end_line: chunk.source.end_line,
                routed_domain,
                by_domain,
                anchored_rules: Vec::new(),
                anchored_missing: Vec::new(),
                heuristic_only: false,
            });
        }
        let Some(anchors) = anchors else {
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
                };
                push_anchor(&mut synthetic, &self.rule_book, anchor);
                let pos = out
                    .iter()
                    .position(|s| s.start_line > anchor.line)
                    .unwrap_or(out.len());
                out.insert(pos, synthetic);
            }
        }
        for span in &mut out {
            span.heuristic_only = span.anchored_rules.is_empty() && span.anchored_missing.is_empty();
        }
        out
    }
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
        if !span
            .anchored_rules
            .iter()
            .any(|(_, id)| *id == anchor.lint)
        {
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
#[cfg(all(test, feature = "latent_retrieval", feature = "clippy_lints", feature = "rust_perf"))]
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
        assert!(!by_domain.is_empty());
        // Fan-out: the clippy domain's list must exist + top-1 needless_return
        // (empirically the routed annotation may differ — see the mis-routing
        // finding in suggest_span's doc).
        let clippy = by_domain
            .iter()
            .find(|(d, _)| *d == DomainId::Clippy)
            .expect("clippy index enabled by default");
        assert!(!clippy.1.is_empty());
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
            assert!(!s.by_domain.is_empty());
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

    fn anchor(file: &str, line: usize, lint_id: &str) -> super::super::clippy_feed::ClippyDiagnostic {
        super::super::clippy_feed::ClippyDiagnostic {
            file: file.to_string(),
            line,
            lint: lint_id.to_string(),
            suggestion: None,
        }
    }

    #[test]
    fn anchored_feed_force_includes_the_rule_at_its_line() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file_anchored(SAMPLE, "sample.rs", 3, &[anchor(
            "sample.rs",
            2,
            "needless_return",
        )]);
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
        let sugg = m.suggest_file_anchored(SAMPLE, "sample.rs", 3, &[anchor(
            "sample.rs",
            2,
            "definitely_not_a_corpus_rule",
        )]);
        let span = sugg
            .iter()
            .find(|s| s.start_line <= 2 && s.end_line >= 2)
            .expect("a chunk covers line 2");
        assert_eq!(span.anchored_missing, vec!["definitely_not_a_corpus_rule".to_string()]);
        assert!(span.anchored_rules.is_empty());
        assert!(!span.heuristic_only, "a gap signal is still an oracle hit");
    }

    #[test]
    fn plain_suggest_file_leaves_anchor_fields_default() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file(SAMPLE, "sample.rs", 3);
        assert!(!sugg.is_empty());
        for s in &sugg {
            assert!(s.anchored_rules.is_empty());
            assert!(s.anchored_missing.is_empty());
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
        let sugg = m.suggest_file_anchored(SAMPLE, "sample.rs", 3, &[anchor(
            "sample.rs",
            4,
            "needless_return",
        )]);
        let pos = sugg
            .iter()
            .position(|s| s.start_line == 4 && s.end_line == 4)
            .expect("synthesized line-span at the gap");
        let span = &sugg[pos];
        assert!(span
            .anchored_rules
            .iter()
            .any(|(d, id)| *d == DomainId::Clippy && id == "needless_return"));
        assert!(span.by_domain.is_empty(), "no retrieval on a synthetic span");
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
}

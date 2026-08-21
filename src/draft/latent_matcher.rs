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

use super::corpus_index::{RuleIndex, ScoredRule};
use super::domain_router::{DomainId, DomainRouter, RoutingEmbedding, SpectralOrder};

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
        all.sort_unstable_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
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
        );
        #[cfg(not(feature = "clippy_lints"))]
        let clippy = None;

        #[cfg(feature = "rust_perf")]
        let perf = index_domain!(
            perf,
            DomainId::Perf,
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
        );
        #[cfg(not(feature = "rust_perf"))]
        let perf = None;

        #[cfg(feature = "kernel_heal")]
        let kernel = index_domain!(
            kernel,
            DomainId::Kernel,
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
        );
        #[cfg(not(feature = "kernel_heal"))]
        let kernel = None;

        #[cfg(feature = "sec")]
        let sec = index_domain!(
            sec,
            DomainId::Sec,
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
        );
        #[cfg(not(feature = "sec"))]
        let sec = None;

        if router_domains.is_empty() {
            return Err(
                "LatentMatcher: no domains enabled (need at least one of clippy_lints/rust_perf/kernel_heal)"
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
    fn suggest_all_domains(&mut self, code: &str, top_k: usize) -> Vec<(DomainId, Vec<ScoredRule>)> {
        let mut out = Vec::with_capacity(4);
        for domain in [
            DomainId::Clippy,
            DomainId::Perf,
            DomainId::Kernel,
            DomainId::Sec,
        ] {
            // Direct field access (not `index_for`) so the immutable index
            // borrow + the mutable scratch borrow stay provably disjoint.
            let index = match domain {
                DomainId::Clippy => self.clippy.as_ref(),
                DomainId::Perf => self.perf.as_ref(),
                DomainId::Kernel => self.kernel.as_ref(),
                DomainId::Sec => self.sec.as_ref(),
            };
            if let Some(index) = index {
                // Retrieve a POOL deeper than the caller's top_k: re-ranking
                // can only promote a rule that was retrieved, and the residual
                // T3.5.2 misses sat at rank 2-3 (T3.5.4).
                let pool = top_k.max(super::instantiation_rerank::RERANK_POOL);
                let mut rules = index.suggest_deep_into(code, pool, &mut self.scratch).to_vec();
                super::instantiation_rerank::mark_instantiation(
                    domain,
                    code,
                    &mut rules,
                    self.rerank_mode,
                );
                super::instantiation_rerank::rerank(&mut rules);
                rules.truncate(top_k);
                out.push((domain, rules));
            }
        }
        out
    }

    /// Suggest rules for every chunk of a real `.rs` file.
    ///
    /// Uses [`AstChunker`] (fn/impl/struct spans with 1-based line ranges).
    /// Order: file order. Retrieval fans out across all enabled domains
    /// (see [`Self::suggest_span`]); the router verdict annotates each span.
    /// Takes `&mut self` — the G4 scratch is reused across every span.
    pub fn suggest_file(&mut self, src: &str, path: &str, top_k: usize) -> Vec<SpanSuggestion> {
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
            });
        }
        out
    }
}

#[cfg(test)]
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
            clippy.1.iter().take(3).any(|r| r.rule_id == "needless_return"),
            "needless_return not in top-3: {:?}",
            clippy.1.iter().map(|r| r.rule_id.clone()).collect::<Vec<_>>()
        );
        let _ = routed;
    }

    #[test]
    fn file_suggestion_covers_all_chunks_with_lines() {
        let mut m = LatentMatcher::from_enabled_domains().expect("build");
        let sugg = m.suggest_file(SAMPLE, "sample.rs", 3);
        assert!(sugg.len() >= 2, "expected ≥2 chunks (2 fns), got {}", sugg.len());
        for s in &sugg {
            assert_eq!(s.path, "sample.rs");
            assert!(s.start_line >= 1 && s.end_line >= s.start_line);
            assert!(!s.by_domain.is_empty());
            assert!(!s.all_rules().is_empty(), "chunk {} suggested no rules", s.start_line);
        }
        // Fan-out: the needless_return-shaped chunk must surface that rule
        // via the CLIPPY domain list (in top-3 — recall; precision@1 over a
        // real fixture is the Phase 3 G1 gate's job).
        let ret = sugg
            .iter()
            .find(|s| {
                s.by_domain.iter().any(|(d, rules)| {
                    *d == DomainId::Clippy
                        && rules.iter().any(|r| r.rule_id == "needless_return")
                })
            })
            .expect("some chunk must surface needless_return via the clippy list");
        assert_eq!(ret.start_line, 1);
    }
}

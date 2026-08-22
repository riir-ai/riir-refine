//! Latent retrieval draft layer — Plan 005 (real-file span-level healing).
//!
//! The `--suggest` MVP: index the corpus rules via the riir-rag substrate,
//! embed real code chunks, route to the owning domain, retrieve ranked
//! rules. ZERO proposer code — suggestions ARE the corpus
//! `buggy_code`/`ref_shape` + `fixed_code`/`opt_pattern` texts.
//!
//! Feature-gated behind `latent_retrieval` (pulls riir-rag + riir-neuron-db
//! via path deps — heavy, opt-in; the default build stays dependency-light).
//!
//! Layer map (Plan 005):
//! - [`corpus_index`] — T1.2: per-domain rule index (`ShardIndex` KNN + BM25).
//! - [`op_tokenizer`] — T3.5: operator-aware BM25 tokenizer (the
//!   tiny-token-crowding fix; clippy top-1 54.2% -> 91.7%, Bench 027).
//! - [`clippy_oracle`] — T3.5.5: real-`cargo clippy` labeling oracle, the
//!   uncontaminated ground truth the re-ranker must be measured against.
//! - [`instantiation_rerank`] — T3.5.4: proposer-visitor re-ranking over the
//!   retrieved top-k (rule instantiation, not text similarity).
//! - [`domain_router`] — T1.4: centroid `MoE` routing across enabled domains.
//! - [`latent_matcher`] — T1.3 + T1.5: the composed file/span → rules pipeline.
//! - [`retrieval_eval`] — Issue 017: scaled precision eval (≥100 labeled
//!   spans, top-1/3/5 recall per domain, regression-gate floors) that
//!   re-measures on every corpus batch.
//! - [`switch_cost`] + [`eval_fixtures_cross_domain`] — Issue 024: the
//!   directed cross-domain (clippy × perf) co-presence measurement — does
//!   an anti-pattern retrieve its rule worse when another domain's
//!   anti-pattern shares the span? (Research 484's switch-cost analog.)

pub mod corpus_index;
pub mod domain_router;
pub mod eval_fixtures_clippy;
pub mod eval_fixtures_composite;
pub mod eval_fixtures_cross_domain;
pub mod eval_fixtures_perf;
pub mod headroom_probe;
pub mod latent_matcher;
#[cfg(feature = "clippy_verify")]
pub mod clippy_oracle;
pub mod instantiation_rerank;
#[cfg(feature = "clippy_verify")]
pub mod jury_disagreement;
pub mod op_tokenizer;
pub mod retrieval_eval;
pub mod switch_cost;

pub use corpus_index::{IndexTokenizer, RetrievalConfig, RuleIndex, ScoredRule, SuggestScratch};
pub use domain_router::{
    DomainId, DomainRouter, RoutingEmbedding, SpectralOrder, SpectralVocab, SPECTRAL_DIM,
    SPECTRAL_K, SPECTRAL_VOCAB_N,
};
pub use latent_matcher::{LatentMatcher, RuleInfo, SpanSuggestion};
#[cfg(feature = "clippy_verify")]
pub use clippy_oracle::{ClippyOracle, OracleVerdict};
pub use headroom_probe::{
    decompose_headroom, print_headroom_decomposition, print_headroom_report, run_headroom_probe,
    run_headroom_probe_with, HeadroomDecomposition, HeadroomMetrics, HeadroomMiss, HeadroomReport,
    HeadroomVerdict, MissKind, ResidualCause, HEADROOM_DEPTHS, HEADROOM_STOP_BAR_PP,
};
pub use instantiation_rerank::{RerankMode, RERANK_POOL};
pub use op_tokenizer::{RustOpBigramTokenizer, RustOpTokenizer, BIGRAM_SEP, OP_PREFIX};
pub use retrieval_eval::{run_eval, run_eval_with, run_retrieval_sweep, run_router_eval, EvalReport, DomainMetrics as EvalDomainMetrics};
pub use switch_cost::{cross_domain_spans, run_switch_cost_eval, print_switch_cost_report, SwitchCostReport};

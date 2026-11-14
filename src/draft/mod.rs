//! Latent retrieval draft layer — Plan 005 (real-file span-level healing).
//!
//! The `--suggest` MVP: index the corpus rules via the vendored
//! [`crate::ragplane`] substrate, embed real code chunks, route to the
//! owning domain, retrieve ranked rules. ZERO proposer code — suggestions
//! ARE the corpus `buggy_code`/`ref_shape` + `fixed_code`/`opt_pattern`
//! texts.
//!
//! Issue 221 P2b: the MODELLESS CORE of this layer (the matcher pipeline:
//! `clippy_feed` / `span_source` / `corpus_index` / `domain_router` /
//! `latent_matcher` / `op_tokenizer` / `instantiation_rerank` / the in-repo
//! `retriever` seam) is UNGATED — it runs on the vendored substrate with
//! zero private deps, so the crates.io publish posture carries the full
//! suggest product. The retrieval-lane EXTRAS (eval fixtures beyond the
//! clippy set, the sweeps, the measurement lanes, the Warm-tier store
//! consumers) stay behind `latent_retrieval`, whose dep rows
//! (`riir-rag` / `riir-neuron-db`) now serve only those lanes plus the
//! `bonsai_retrieval` dense arm.
//!
//! Layer map (Plan 005):
//! - [`corpus_index`] — T1.2: per-domain rule index (exact-cosine KNN + BM25).
//! - [`retriever`] — issue 221 P2b: the in-repo fused retrieval seam (the
//!   ONE retrieval path; replaced the substrate `NeuronDbRetriever`).
//! - [`op_tokenizer`] — T3.5: operator-aware BM25 tokenizer (the
//!   tiny-token-crowding fix; clippy top-1 54.2% -> 91.7%, Bench 027).
//! - [`clippy_oracle`] — T3.5.5: real-`cargo clippy` labeling oracle, the
//!   uncontaminated ground truth the re-ranker must be measured against.
//! - [`instantiation_rerank`] — T3.5.4: proposer-visitor re-ranking over the
//!   retrieved top-k (rule instantiation, not text similarity).
//! - [`domain_router`] — T1.4: centroid `MoE` routing across enabled domains.
//! - [`latent_matcher`] — T1.3 + T1.5: the composed file/span → rules pipeline.
//! - [`clippy_feed`] — the 2026-09-13 retrieval-miss intake: a real-clippy
//!   `--message-format=json` stream parsed into diagnostic anchors that
//!   force-include their spans/rules AHEAD of similarity ranking (the
//!   clippy-fed mode).
//! - [`retrieval_eval`] — Issue 017: scaled precision eval (≥100 labeled
//!   spans, top-1/3/5 recall per domain, regression-gate floors) that
//!   re-measures on every corpus batch.
//! - [`switch_cost`] + [`eval_fixtures_cross_domain`] — Issue 024: the
//!   directed cross-domain (clippy × perf) co-presence measurement — does
//!   an anti-pattern retrieve its rule worse when another domain's
//!   anti-pattern shares the span? (Research 484's switch-cost analog.)
//! - [`set_rerank`] — Issue 121 (katgpt-rs Plan 599 T4.2): the
//!   counter-anchored SET rerank of the merged seven-domain pool — the
//!   first consumer of katgpt-core's `set_admission` primitive.

#[cfg(feature = "choice_scorer_poc")]
pub mod choice_scorer;
pub mod clippy_feed;
#[cfg(feature = "clippy_verify")]
pub mod clippy_oracle;
pub mod corpus_index;
pub mod domain_router;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_ane;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_clippy;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_composite;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_cross_domain;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_dist;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_docker;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_perf;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_sec;
#[cfg(feature = "latent_retrieval")]
pub mod eval_fixtures_shader;
#[cfg(feature = "index_cache")]
pub mod index_cache;
// Issue 217 T0: the size-band jaggedness axis — the ordered OOD cut beside
// the withheld-pair axis (leave-one-BAND-out; the preregistered floor is in
// the issue's Status line, never re-tuned).
#[cfg(feature = "latent_retrieval")]
pub mod band_ood;
// Issue 186: the structure-grounded synth fixture family — tier-1
// deterministic mutations of the corpus's own material (gold = the owning
// rule), the withheld-pair population feeder + the robustness floor.
#[cfg(feature = "latent_retrieval")]
pub mod eval_synth;
#[cfg(feature = "latent_retrieval")]
pub mod headroom_probe;
pub mod instantiation_rerank;
#[cfg(feature = "clippy_verify")]
pub mod jury_disagreement;
pub mod latent_matcher;
// Issue 077 T2+T3: the margin law's shared substrate — pure math + the
// matcher access pattern, no oracle. Gated latent_retrieval (its consumers
// are the gated eval lanes).
#[cfg(feature = "latent_retrieval")]
pub mod margin_weight;
// Issue 169 T0 (Research 309 FLINT): the multi-vector kill-test — per-op-token
// hashed embeddings + the shipped maxsim_score vs the single-vector matcher.
// Measurement-only (examples/multivector_poc.rs); implies latent_retrieval.
#[cfg(feature = "multivector_poc")]
pub mod multivector;
// Issue 126 lane (a): the sigmoid-NCE two-tower span↔rule embeddings + the
// frozen i8 artifact. PoC posture (implies choice_scorer_poc — shared Rng +
// schedule); the shipped suggest lane stays modelless until the gates pass.
#[cfg(feature = "rule_embed_poc")]
pub mod rule_embed;
// Issue 133 T1: the Matryoshka-k randomized-budget supervision arms
// (span-structured BCE over `katgpt_core::exact_mass_admit` masks) —
// measurement-only, same towers/artifact wire as `rule_embed`. Gated on the
// FULL feature (it alone carries the katgpt-core/exact_mass_admit forward).
pub mod op_tokenizer;
// Issue 221 P2b: the in-repo fused retrieval seam — the matcher path's ONE
// retrieval primitive (replaced the substrate `NeuronDbRetriever`). Ungated:
// the publish posture's matcher runs on it.
#[cfg(feature = "latent_retrieval")]
pub mod retrieval_eval;
// Issue 233 T3: the evidence-gated variant of the span-shape dispatch —
// narrowing arms only where the pinned per-group evidence clears the
// Wilson-LCB floor; empty table = fail-open to the full fan-out.
#[cfg(feature = "calibrated_dispatch")]
pub mod calibrated_dispatch;
pub mod retriever;
#[cfg(feature = "mattr_k_poc")]
pub mod rule_embed_mattr;
#[cfg(feature = "set_rerank")]
pub mod set_rerank;
pub mod span_source;
#[cfg(feature = "latent_retrieval")]
pub mod switch_cost;
#[cfg(all(feature = "tpr_structure", feature = "latent_retrieval"))]
pub mod tpr_fit;
#[cfg(all(feature = "tpr_structure", feature = "latent_retrieval"))]
pub mod tpr_t4a_withheld;
#[cfg(all(feature = "tpr_structure", feature = "latent_retrieval"))]
pub mod tpr_t6a_counterfactual;
#[cfg(feature = "latent_retrieval")]
pub mod withheld_pair_ood;

pub use clippy_feed::{ClippyDiagnostic, parse_clippy_feed, paths_ref_match};
#[cfg(feature = "clippy_verify")]
pub use clippy_oracle::{ClippyOracle, OracleVerdict};
pub use corpus_index::{IndexTokenizer, RetrievalConfig, RuleIndex, ScoredRule, SuggestScratch};
pub use domain_router::{
    DomainId, DomainMask, DomainRouter, RoutingEmbedding, SPECTRAL_DIM, SPECTRAL_K,
    SPECTRAL_VOCAB_N, SpectralOrder, SpectralVocab,
};
#[cfg(feature = "latent_retrieval")]
pub use headroom_probe::{
    HEADROOM_DEPTHS, HEADROOM_STOP_BAR_PP, HeadroomDecomposition, HeadroomMetrics, HeadroomMiss,
    HeadroomReport, HeadroomVerdict, MissKind, ResidualCause, decompose_headroom,
    decompose_headroom_with_spans, print_headroom_decomposition, print_headroom_report,
    run_headroom_probe, run_headroom_probe_with, run_headroom_probe_with_spans,
};
pub use instantiation_rerank::{
    CollapseVerdict, RERANK_POOL, RerankMode, detect_collapse, detect_collapse_rule_ids,
    rule_family,
};
pub use latent_matcher::{LatentMatcher, RuleInfo, SpanSuggestion};
#[cfg(feature = "latent_retrieval")]
pub use margin_weight::{
    SpanMargin, concentration_fraction, contested_margin, margin_weight, span_margin,
    span_min_margin,
};
pub use op_tokenizer::{BIGRAM_SEP, OP_PREFIX, RustOpBigramTokenizer, RustOpTokenizer};
#[cfg(feature = "set_rerank")]
pub use retrieval_eval::VendiAxis;
#[cfg(feature = "latent_retrieval")]
pub use retrieval_eval::{
    ChoiceTaxonomy, DomainMetrics as EvalDomainMetrics, EvalReport, OwnedEvalSpan, run_eval,
    run_eval_with, run_retrieval_sweep, run_router_eval,
};
#[cfg(feature = "set_rerank")]
pub use set_rerank::{
    AdmittedRule, DEFAULT_SLATE_K, LatentChoice, NO_SET_RERANK_ENV, RuleLatentCorpus,
    SetRerankConfig, SetSlate, baseline_slate, counter_anchored_slate, merge_pool,
};
pub use span_source::{collect_source_files, collect_source_files_sorted};
#[cfg(feature = "latent_retrieval")]
pub use switch_cost::{
    SwitchCostReport, cross_domain_spans, print_switch_cost_report, run_switch_cost_eval,
};
#[cfg(feature = "set_rerank")]
pub use withheld_pair_ood::run_withheld_pair_set_admission;
#[cfg(feature = "latent_retrieval")]
pub use withheld_pair_ood::{
    ArmRow, AtomicNull, Binding, ShapeClass, WithheldReport, atomic_keys, classify_rustc_shape,
    classify_shape, normalized_tokens, run_withheld_pair,
};

//! Issue 196 — the route-narrowed kernel retrieval (opt-in
//! `kernel_route_narrow`, implies `kernel_opt`).
//!
//! The kernel domain's matcher index, built as **15 per-category
//! sub-indices** (one per `KernelCategory`, via
//! [`KernelExpertRouter::expert_for_rule`]) **+ one bounded global-tail
//! index**, instead of the single flat `RuleIndex` over the full ~1051-rule
//! corpus. This is the G5 cost-leg repair (Bench 125: adding the kernel
//! domain to the scan costed p50 38.8 ms/file added against the
//! pre-registered 1.0 ms ceiling, 39× over — the flat index fans every
//! significant-line query out to the full corpus, and the line-query loop
//! is ~20× the span mass).
//!
//! ## Design (the issue's option (b) — substrate-first)
//!
//! - **Routing is LEXICAL, span-level, once per suggest call.** The span's
//!   own TF-cosine token vector dotted against 15 category centroids (the
//!   Issue-020-proven routing primitive, [`SpectralVocab`]/`DomainRouter`
//!   machinery at category granularity, `Discriminative` vocab order);
//!   ties break to the lowest index. Measured pivot: the issue's option (b)
//!   (route on the existing 8-D `ModellessEmbedder` direction) was
//!   implemented first and REFUTED by the retrieval-precision gate — kernel
//!   top-1 100% → 16.7% (~random: 1/15 slice + 12% tail) — because the 8-D
//!   embedding is a hash-space signal, near-orthogonal across distinct
//!   texts. The record: Bench 126 / the Issue-196 landing note.
//!   Routing ignores the differential-anchor correction (a per-slice
//!   retrieval rider, not a routing concern).
//! - **Line queries inherit the containing span's expert.** The line
//!   queries exist to refine the span's own scoring (the multi-scale merge
//!   law), so a per-line re-route would re-embed every line for nothing.
//!   One routing decision per `suggest_deep_into` call.
//! - **The global tail + the top-K set.** A stride subsample of the full
//!   corpus in build order ([`ROUTE_TAIL_RULES`], pinned) rides EVERY query
//!   beside the span's TOP-K routed slices ([`ROUTE_TOP_K_EXPERTS`],
//!   pinned at the measured sweep point) — a routing miss is bounded by
//!   (K−1) sibling slices + the tail rather than lost. Tail rules also
//!   appear in their own category slices; the per-rule best-score merge
//!   makes the duplicate harmless.
//! - **Scoring is per-slice.** Each sub-index carries its own BM25
//!   statistics (IDF over its slice, not the full corpus), so cross-slice
//!   score comparison is approximate. This IS the disclosed composition
//!   change: the ON posture's rank-k candidates differ from the flat
//!   posture's, the census re-run (Bench 125's harness, unchanged)
//!   quantifies the recall delta, and the G5 utility floor (15/15 experts
//!   owning rank-1) must still hold.
//!
//! ## Disclosed feature interactions (both opt-in)
//!
//! - `rule_embed_poc` (the Issue-126 trained two-tower term) is a
//!   FLAT-index rider: the narrowed posture does not apply the trained term
//!   (`with_rule_embed` forwards to the flat index only). Both features on
//!   simultaneously is undefined territory — the narrowed kernel domain
//!   falls back to the untrained ranking, honestly.
//! - `differential_anchor` forwards to every sub-index (each carries its
//!   own mean-corpus anchor; the correction applies at each slice's
//!   retrieval, exactly the per-domain rider semantics).
//! - An overlay-leased epoch (rule ids unknown to `KERNEL_CATEGORIES`)
//!   rides the tail only — narrowed narrowing, disclosed, still functional.
//!
//! ## G4 discipline
//!
//! The narrowed path reuses the caller's `SuggestScratch` (retrieved/best/
//! ranked) — capacities stabilize across calls, and the per-query
//! allocations shrink with the scored mass. The OFF posture is the flat
//! index byte-identical (the G3 disclosure boundary).

use std::collections::HashMap;

use super::router::{KernelExpertRouter, NUM_KERNEL_EXPERTS};
use crate::draft::corpus_index::{
    finalize_ranked, merge_retrieved_best, significant_lines, RetrievalConfig, RuleIndex,
    ScoredRule, SuggestScratch,
};
use crate::draft::domain_router::{SpectralOrder, SpectralVocab, SPECTRAL_VOCAB_N};

/// The pinned global-tail size (rules, not chunks): a stride subsample of
/// the full corpus in build order, retrieved by EVERY query beside the
/// routed slices. ~12% of the standing 1051-rule corpus. Pinned constant,
/// disclosed here, never env-tuned (a knob would be an unmeasured A/B
/// surface).
pub const ROUTE_TAIL_RULES: usize = 128;

/// The pinned top-K: EVERY query (span + line) retrieves its span's TOP-K
/// routed category slices + the global tail. Measured K-sweep on the
/// retrieval-precision gate's own 30 kernel fixture spans (the R1
/// diagnostic, 2026-11-02): end-to-end top-1 13/30 (K=1) → 20/30 (K=3) →
/// 27/30 (K=5) → **29/30 (K=6)**. K=6 is the FIRST point that holds the
/// gate's own floors WITHOUT touching a pin — kernel top-1 96.7%, blended
/// 89.5%/93.0%/94.7% vs floors 87.2%/92.2%/93.6% (measured at the R1
/// landing; the gate runs UNCHANGED at the narrow posture). K=5 missed the
/// blended top-3 floor by ONE span (157/171 vs 130/141), so the sweep's
/// next point is the design point, not a preference. Rule mass per query
/// ≈ 52% of the flat corpus (6 slices ~40% + tail ~12%), 7 retrieve calls
/// per query — the measured quality/cost point, not an assumed 15× (the
/// single-expert design the issue sketched was refuted by this sweep; see
/// [`CategoryRouter`]).
pub const ROUTE_TOP_K_EXPERTS: usize = 6;

/// The category router — TF-cosine over 15 category centroids (the
/// Issue-020-proven lexical routing primitive, the SAME machinery the
/// matcher's own `DomainRouter` runs, one granularity down).
///
/// **Measured pivot (Issue 196 R1, 2026-11-02):** the issue's option (b) —
/// route on the span's existing 8-D `ModellessEmbedder` direction dotted
/// against category centroids — was implemented first and REFUTED by the
/// retrieval-precision gate: kernel-domain top-1 fell 100% → 16.7% (5/30),
/// blended 90.1% → 75.4%. The 8-D embedder is BLAKE3+DFT+sigmoid — a
/// hash-space signal, near-orthogonal across distinct texts (the Issue 020
/// record's own finding), so 15-way category routing on it selects
/// categories ~randomly: 1/15 slice hit + the 12% tail ≈ the observed
/// 16.7%. The lexical TF-cosine block is the signal the repo's own routing
/// verdict trusts (~70% at DOMAIN level, Issue 020 / Bench 025); at sibling-
/// category granularity the vocab is built over the 15 categories' own
/// pattern texts with the `Discriminative` order (max–min TF-share spread),
/// which upweights category-DISTINCTIVE tokens across otherwise-CUDA-
/// homogeneous siblings.
struct CategoryRouter {
    /// The shared 128-slot vocab, built over the 15 categories' pattern texts.
    vocab: SpectralVocab,
    /// Per-category mean-of-TF-embeddings centroids, L2-normalized
    /// (mirrors `DomainRouter`'s `TfCosine` posture exactly).
    centroids: Vec<[f32; SPECTRAL_VOCAB_N]>,
}

impl CategoryRouter {
    /// Build from each category's pattern texts (group index = category
    /// index = expert index).
    fn build(group_texts: &[Vec<String>]) -> Self {
        let vocab = SpectralVocab::build_from_patterns(group_texts, SpectralOrder::Discriminative);
        let mut centroids = Vec::with_capacity(group_texts.len());
        for texts in group_texts {
            let mut centroid = [0.0f32; SPECTRAL_VOCAB_N];
            if !texts.is_empty() {
                for text in texts {
                    let vec = vocab.tf_embed(text);
                    for (c, x) in centroid.iter_mut().zip(vec) {
                        *c += x;
                    }
                }
                // Mean-of-embeddings → unit centroid (the DomainRouter
                // normalize_into semantics; /n is a positive scale the L2
                // normalization absorbs — kept for parity, not for effect).
                let n = texts.len();
                #[allow(clippy::cast_precision_loss)] // corpus sizes are ≤ ~10²
                let scale = 1.0 / n as f32;
                for c in &mut centroid {
                    *c *= scale;
                }
                let norm: f32 = centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 1e-10 {
                    let inv = 1.0 / norm;
                    for c in &mut centroid {
                        *c *= inv;
                    }
                }
            }
            centroids.push(centroid);
        }
        Self { vocab, centroids }
    }

    /// Route a span to its category index — argmax over
    /// `dot(tf_embed(code), centroid)`; ties → lowest index (deterministic,
    /// the `argmax_tf` semantics). Test/diagnostic surface; the suggest
    /// path takes the top-K set ([`Self::route_top`]).
    #[cfg(test)]
    fn route(&self, code: &str) -> usize {
        self.route_top(code, 1)[0]
    }

    /// The top-K category indices for a span, score desc — ties broken to
    /// the lower index (deterministic, mirroring the corpus ranking
    /// convention). K is clamped to the category count.
    fn route_top(&self, code: &str, k: usize) -> Vec<usize> {
        let vec = self.vocab.tf_embed(code);
        let mut scored: Vec<(usize, f32)> = self
            .centroids
            .iter()
            .enumerate()
            .map(|(i, centroid)| {
                let mut s = 0.0f32;
                for (a, d) in vec.iter().zip(centroid.iter()) {
                    s += a * d;
                }
                (i, s)
            })
            .collect();
        scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        scored.truncate(k.min(scored.len()));
        scored.into_iter().map(|(i, _)| i).collect()
    }
}

/// The route-narrowed kernel domain index — see the module docs.
pub(crate) struct NarrowedKernelIndex {
    /// 15 per-category sub-indices (`KernelCategory` discriminant order).
    /// Empty categories build empty indices (the substrate's
    /// `retrieve_candidates` returns no candidates for an empty index).
    slices: Vec<RuleIndex>,
    /// The bounded deterministic global tail (stride subsample).
    tail: RuleIndex,
    /// The lexical category router (TF-cosine; the measured pivot — the
    /// 8-D hash-space dot is refuted, see [`CategoryRouter`]).
    router: CategoryRouter,
    /// Merged `zone_hash → rule_id` over the WHOLE corpus in build order —
    /// row-for-row identical to the flat index's `by_zone_hash` (both walk
    /// `significant_lines`).
    zone_lookup: HashMap<[u8; 32], String>,
    /// The scoring weights this index was built with (forwarded to the
    /// shared merge core).
    config: RetrievalConfig,
}

impl NarrowedKernelIndex {
    /// Build the 15 category slices + tail from the kernel corpus entries
    /// (the same `(rule_id, pattern)` list `RuleIndex::build_with` takes —
    /// the overlay contract applies unchanged: non-empty overlay entries
    /// replace the seed corpus wholesale).
    ///
    /// # Errors
    ///
    /// Returns the substrate's indexing error if any sub-index build fails.
    pub(crate) fn build(
        patterns: &[(String, String)],
        config: RetrievalConfig,
    ) -> Result<Self, String> {
        // The merged zone lookup, walked in corpus order through the SAME
        // row definition the flat build uses — identical semantics.
        let mut zone_lookup: HashMap<[u8; 32], String> =
            HashMap::with_capacity(patterns.len() * 2);
        for (rule_id, text) in patterns {
            for content in significant_lines(text) {
                zone_lookup.insert(
                    *blake3::hash(content.as_bytes()).as_bytes(),
                    rule_id.clone(),
                );
            }
        }
        // Partition by category (the category router builds from the same
        // partition's texts, after it).
        let mut slice_patterns: Vec<Vec<(String, String)>> =
            (0..NUM_KERNEL_EXPERTS).map(|_| Vec::new()).collect();
        for (rule_id, text) in patterns {
            let Some(expert) = KernelExpertRouter::expert_for_rule(rule_id) else {
                // Unknown to the category table (an overlay-leased rule):
                // rides the tail only — disclosed in the module docs.
                continue;
            };
            slice_patterns[expert].push((rule_id.clone(), text.clone()));
        }
        let group_texts: Vec<Vec<String>> = slice_patterns
            .iter()
            .map(|sp| sp.iter().map(|(_, t)| t.clone()).collect())
            .collect();
        let router = CategoryRouter::build(&group_texts);
        // The tail: stride subsample of the full corpus in build order.
        let stride = (patterns.len().div_ceil(ROUTE_TAIL_RULES)).max(1);
        let tail_patterns: Vec<(String, String)> = patterns
            .iter()
            .enumerate()
            .filter(|(i, _)| i % stride == 0)
            .map(|(_, p)| p.clone())
            .collect();
        let mut slices = Vec::with_capacity(NUM_KERNEL_EXPERTS);
        for sp in &slice_patterns {
            slices.push(RuleIndex::build_with(sp.clone(), config)?);
        }
        let tail = RuleIndex::build_with(tail_patterns, config)?;
        Ok(Self {
            slices,
            tail,
            router,
            zone_lookup,
            config,
        })
    }

    /// Multi-scale suggest over the top-K routed slices + tail: route ONCE
    /// on the span text (lexical TF-cosine), take the TOP-K categories
    /// ([`ROUTE_TOP_K_EXPERTS`]), then query those slices and the tail for
    /// the whole span AND each significant line (the line queries inherit
    /// the span's routed set — the module docs). Ranks through the shared
    /// merge core with the merged zone lookup.
    pub(crate) fn suggest_deep_into<'s>(
        &self,
        code: &str,
        top_k: usize,
        scratch: &'s mut SuggestScratch,
    ) -> &'s [ScoredRule] {
        scratch.retrieved.clear();
        let routed = self.router.route_top(code, ROUTE_TOP_K_EXPERTS);
        for &expert in &routed {
            let slice = &self.slices[expert];
            if slice.is_empty() {
                continue;
            }
            slice.retrieve_scored_into(code, &mut scratch.retrieved);
        }
        self.tail.retrieve_scored_into(code, &mut scratch.retrieved);
        for line in code.lines() {
            let trimmed = line.trim();
            // The same two skips as the flat multi-scale path (Issue 063):
            // insignificant lines contribute nothing; a single-line span's
            // only line IS the whole-span query, already merged.
            if trimmed.len() <= 8 || trimmed == code {
                continue;
            }
            for &expert in &routed {
                let slice = &self.slices[expert];
                if slice.is_empty() {
                    continue;
                }
                slice.retrieve_scored_into(trimmed, &mut scratch.retrieved);
            }
            self.tail.retrieve_scored_into(trimmed, &mut scratch.retrieved);
        }
        merge_retrieved_best(
            &self.zone_lookup,
            &self.config,
            &scratch.retrieved,
            &mut scratch.best,
        );
        finalize_ranked(&mut scratch.best, &mut scratch.ranked, top_k)
    }

    /// Test observable: total rules across the category slices (the tail
    /// overlaps by construction and is not counted).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slices.iter().map(RuleIndex::len).sum::<usize>()
    }

    /// Forward the differential-anchor rider to every sub-index (each
    /// carries its own mean-corpus anchor; the correction applies at each
    /// slice's retrieval — the per-domain rider semantics preserved).
    #[cfg(feature = "differential_anchor")]
    pub(crate) fn set_differential_anchor(&mut self, lambda: f32) {
        for slice in &mut self.slices {
            slice.set_differential_anchor(lambda);
        }
        self.tail.set_differential_anchor(lambda);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A few REAL rule ids across three categories (the first of each) —
    /// `expert_for_rule` must resolve them for the partition to bite.
    fn seed_patterns() -> Vec<(String, String)> {
        let mut out = Vec::new();
        for expert in [0_usize, 1, 5] {
            let rules = KernelExpertRouter::rules_for_expert(expert);
            let id = rules.first().expect("category has rules").to_string();
            let text = format!(
                "// {id} pattern\nfn kernel_step(p: *mut f32, n: usize) {{\n    let tile = p.add(n);\n    *tile = 0.0;\n}}"
            );
            out.push((id, text));
        }
        out
    }

    #[test]
    fn build_partitions_and_routes_deterministically() {
        let patterns = seed_patterns();
        let n = patterns.len();
        let idx = NarrowedKernelIndex::build(&patterns, RetrievalConfig::default())
            .expect("narrowed index builds");
        // Every rule landed in its category slice (len counts slice rules;
        // the tail overlaps by construction and is not counted).
        assert_eq!(idx.len(), n);

        let mut scratch = SuggestScratch::default();
        let code = "fn kernel_step(p: *mut f32, n: usize) {\n    let tile = p.add(n);\n    *tile = 0.0;\n}";
        let first = idx
            .suggest_deep_into(code, 5, &mut scratch)
            .to_vec();
        assert!(!first.is_empty(), "narrowed retrieval returns candidates");
        // Deterministic: a fresh call (same scratch) replays byte-identical.
        let second = idx.suggest_deep_into(code, 5, &mut scratch).to_vec();
        assert_eq!(first, second);
    }

    #[test]
    fn lexical_router_self_routes_a_category_pattern() {
        // The exact corpus text of a category-0 rule routes to category 0 —
        // the TF-cosine self-routing sanity the 8-D hash-space router
        // could not hold (the measured pivot, `CategoryRouter` docs).
        let expert = 0_usize;
        let id = KernelExpertRouter::rules_for_expert(expert)[0].to_string();
        let text = format!(
            "// {id} pattern\nfn kernel_step(p: *mut f32, n: usize) {{\n    let tile = p.add(n);\n    *tile = 0.0;\n}}"
        );
        let idx = NarrowedKernelIndex::build(&[(id, text.clone())], RetrievalConfig::default())
            .expect("narrowed index builds");
        assert_eq!(idx.router.route(&text), expert);
    }

    #[test]
    fn scratch_capacities_stabilize_across_calls() {
        let idx =
            NarrowedKernelIndex::build(&seed_patterns(), RetrievalConfig::default())
                .expect("narrowed index builds");
        let mut scratch = SuggestScratch::default();
        let code = "fn kernel_step(p: *mut f32, n: usize) {\n    let tile = p.add(n);\n    *tile = 0.0;\n}";
        let _ = idx.suggest_deep_into(code, 5, &mut scratch);
        let (r1, k1) = (scratch.retrieved_capacity(), scratch.ranked_capacity());
        for _ in 0..3 {
            let _ = idx.suggest_deep_into(code, 5, &mut scratch);
        }
        let (r2, k2) = (scratch.retrieved_capacity(), scratch.ranked_capacity());
        assert_eq!((r1, k1), (r2, k2), "G4: capacities stabilize after warmup");
    }

    #[test]
    fn tail_constant_is_pinned_and_sane() {
        assert_eq!(ROUTE_TAIL_RULES, 128);
    }

    /// Diagnostic (run with --nocapture): the category router's accuracy on
    /// the retrieval-precision gate's own 30 kernel fixture spans (the same
    /// step-by selection + `mutate_shape` renames), plus the end-to-end
    /// top-1 split — routed-right vs rescued-by-tail. Decides the slice
    /// budget (top-K experts) the R1 record pins. The K-sweep lives in
    /// [`diagnostic_k_sweep`] (this test keeps the router-accuracy half).
    #[test]
    fn diagnostic_routing_accuracy_on_gate_fixture_spans() {
        let (idx, spans) = gate_fixture_index();

        let mut route_top1 = 0;
        let mut route_top3 = 0;
        let mut end_top1 = 0;
        let mut scratch = SuggestScratch::default();
        for (rule_id, code) in &spans {
            let true_expert = KernelExpertRouter::expert_for_rule(rule_id).expect("labeled rule");
            let rank = router_rank(&idx.router, code, true_expert);
            if rank == 0 {
                route_top1 += 1;
            }
            if rank < 3 {
                route_top3 += 1;
            }
            let top = idx.suggest_deep_into(code, 8, &mut scratch);
            if top.first().is_some_and(|r| r.rule_id == *rule_id) {
                end_top1 += 1;
            }
        }
        println!(
            "routing on 30 gate spans: top-1 {route_top1}/30, top-3 {route_top3}/30 | end-to-end top-1 {end_top1}/30"
        );
    }

    /// The K-sweep half of the diagnostic: top-K routed slices + tail,
    /// merged through the same fns the real path uses. The sweep that chose
    /// [`ROUTE_TOP_K_EXPERTS`] (K=6: the first point holding the gate's own
    /// floors untouched).
    #[test]
    fn diagnostic_k_sweep() {
        let (idx, spans) = gate_fixture_index();
        for k in 1..=6_usize {
            let mut hits = 0;
            for (rule_id, code) in &spans {
                let routed = idx.router.route_top(code, k);
                let mut retrieved: Vec<riir_rag::RetrievedChunk> = Vec::new();
                for slice_i in &routed {
                    let slice = &idx.slices[*slice_i];
                    if !slice.is_empty() {
                        slice.retrieve_scored_into(code, &mut retrieved);
                    }
                }
                idx.tail.retrieve_scored_into(code, &mut retrieved);
                // Multi-scale: the line queries ride the same K slices.
                for line in code.lines() {
                    let trimmed = line.trim();
                    if trimmed.len() <= 8 || trimmed == code {
                        continue;
                    }
                    for slice_i in &routed {
                        let slice = &idx.slices[*slice_i];
                        if !slice.is_empty() {
                            slice.retrieve_scored_into(trimmed, &mut retrieved);
                        }
                    }
                    idx.tail.retrieve_scored_into(trimmed, &mut retrieved);
                }
                let mut best = std::collections::HashMap::new();
                crate::draft::corpus_index::merge_retrieved_best(
                    &idx.zone_lookup,
                    &idx.config,
                    &retrieved,
                    &mut best,
                );
                let mut ranked = Vec::new();
                let top = crate::draft::corpus_index::finalize_ranked(&mut best, &mut ranked, 8);
                if top.first().is_some_and(|r| r.rule_id == *rule_id) {
                    hits += 1;
                }
            }
            println!("K={k}: end-to-end top-1 {hits}/30 (slices {k} + tail)");
        }
    }

    /// The diagnostic fixture set: the narrowed index over the FULL corpus
    /// + the gate's 30 renamed self-variant spans (the same step-by
    ///   selection `retrieval_eval::kernel_fixture_spans` uses).
    fn gate_fixture_index() -> (NarrowedKernelIndex, Vec<(String, String)>) {
        use crate::domains::kernel_opt::corpus::build_kernel_seed_corpus;
        use crate::draft::retrieval_eval::mutate_shape;

        let entries = build_kernel_seed_corpus().entries;
        let step = (entries.len() / 30).max(1);
        let spans: Vec<(String, String)> = entries
            .iter()
            .step_by(step)
            .take(30)
            .map(|e| (e.rule_id.clone(), mutate_shape(&e.ref_shape)))
            .collect();
        let patterns: Vec<(String, String)> = entries
            .iter()
            .map(|e| (e.rule_id.clone(), e.ref_shape.clone()))
            .collect();
        let idx = NarrowedKernelIndex::build(&patterns, RetrievalConfig::default())
            .expect("narrowed index builds");
        (idx, spans)
    }

    /// The true category's router rank (0 = top) for a span.
    fn router_rank(router: &CategoryRouter, code: &str, true_expert: usize) -> usize {
        router
            .route_top(code, NUM_KERNEL_EXPERTS)
            .iter()
            .position(|i| *i == true_expert)
            .expect("category present in the routed set")
    }
}

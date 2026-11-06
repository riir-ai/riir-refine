//! Issue 169 T0 — the FLINT-style multi-vector kill-test (modelless).
//!
//! Arm (e) of Issue 169's G1 battery: score a span against every rule of its
//! domain with the SHIPPED [`maxsim_score`](katgpt_core::simd::maxsim_score)
//! (Plan 080) over per-op-token hashed embeddings — NO pooling, NO learned
//! weights. Arm (a) is the shipped [`LatentMatcher`](super::latent_matcher::LatentMatcher)
//! pipeline measured by `retrieval_eval` in the same binary. If (e) cannot
//! beat (a) at the widest D ∈ {8, 32, 64}, no deterministic pooler can
//! (pooling only removes information from the token set), and Issue 169's T1
//! is never built — that is the kill-test the issue pins.
//!
//! ## The per-token object
//!
//! [`hash_embed`]`::<D>(token)`: BLAKE3 XOF over the token's UTF-8 bytes → D
//! little-endian u32 words → f32 in [-1, 1] → L2-normalized. Deterministic,
//! allocation-light, BLAKE3-committable (a corpus digest rides every build).
//! The tokenizer is the shipped [`RustOpTokenizer`] (identifiers + maximal
//! punctuation runs) — the same token vocabulary arm (a)'s BM25 half indexes.
//! Both sides share ONE vector space per Issue 169's T1 rule-side spec: query
//! tokens = the span's op-tokens, document tokens = the rule's `buggy_code`
//! op-tokens — NEVER the prose description, never the single 8-D span vector
//! repeated.
//!
//! ## Exact document-side dedup
//!
//! `maxsim_score` takes a max over document tokens per query token. The hash
//! carries no position, so duplicate document tokens produce IDENTICAL rows
//! and the max over the multiset equals the max over its support — document
//! dedup is EXACT, not an approximation. The query side is the design axis:
//! [`QueryMode::All`] keeps every token (count-weighted — the literal arm (e)
//! the issue pins), [`QueryMode::Unique`] gives one vote per type. Both are
//! measured because the kill-test must not die on a weighting choice that T1's
//! salience work would revisit.
//!
//! ## Two integration postures
//!
//! - [`Integration::Raw`] — pure `MaxSim` ranking (the representation-only read).
//! - [`Integration::Full`] — the shipped pipeline shape with ONLY the score
//!   channel swapped: pool = max(`top_k`, `RERANK_POOL`) by `MaxSim`, then the
//!   same `mark_instantiation_with_file` + `rerank` + truncate the incumbent
//!   fan-out runs. A Raw loss does not prove pooling cannot help THROUGH the
//!   shipped pipeline (the incumbent also rides BM25 + instantiation rerank);
//!   Full is the posture a promotion would ship.
//!
//! Measurement-only: nothing here joins the shipped matcher path. The module
//! sits behind `multivector_poc` (implies `latent_retrieval`, forwards
//! `katgpt-core/maxsim` — never default, Issue 169 T2 discipline).

use std::time::{Duration, Instant};

use blake3::Hasher;

use super::corpus_index::ScoredRule;
use super::domain_router::DomainId;
use super::instantiation_rerank::{mark_instantiation_with_file, rerank, RerankMode, RERANK_POOL};
use super::op_tokenizer::RustOpTokenizer;
use super::retrieval_eval::{canonical_rank, domain_index, labeled_eval_spans, CLEAN_SPANS};
use riir_neuron_db::bm25::Tokenizer;

/// Probe depth — mirrors `retrieval_eval::PROBE_TOP_K` (private there;
/// duplicated deliberately, one line, the `linking_confusion` precedent).
const PROBE_TOP_K: usize = 8;

/// Digest domain separator — bump on ANY change to the scheme (hash layout,
/// normalization, tokenizer, dedup rule) so pinned digests cannot silently
/// survive a semantics change.
const SCHEME_TAG: &str = "mv-t0-v1";

/// The token-embedding identity string (G3 discipline): a multi-vector space
/// never mixes vectors with the shipped 8-D index, and every width is its own
/// space. One string per D.
#[must_use]
pub fn token_model_id(dim: usize) -> String {
    format!("{SCHEME_TAG}/op-token-blake3xof-d{dim}")
}

/// The per-op-token hashed embedding — Issue 169 T0's per-token object.
///
/// BLAKE3 XOF (extendable output) over the token bytes → D u32 words →
/// [-1, 1] → L2-normalized. Deterministic across runs and platforms (integer
/// bytes → f32 casts, no transcendendentals), BLAKE3-committable.
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)] // [-1, 1] f32 embeddings; the f64 step only bounds the u32 division
pub fn hash_embed<const D: usize>(token: &str) -> [f32; D] {
    let mut reader = Hasher::new().update(token.as_bytes()).finalize_xof();
    let mut v = [0.0f32; D];
    let mut word = [0u8; 4];
    for slot in &mut v {
        reader.fill(&mut word);
        let bits = u32::from_le_bytes(word);
        *slot = ((f64::from(bits) / f64::from(u32::MAX)) * 2.0 - 1.0) as f32;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-10 {
        return v; // unreachable for real tokens; a zero row scores 0.0 honestly
    }
    for x in &mut v {
        *x /= norm;
    }
    v
}

/// A packed `[n, D]` row-major token matrix (query or document side).
#[derive(Debug, Clone)]
pub struct TokenMatrix<const D: usize> {
    data: Vec<f32>,
    n: usize,
}

impl<const D: usize> TokenMatrix<D> {
    fn from_tokens(tokens: &[String]) -> Self {
        let mut data = Vec::with_capacity(tokens.len() * D);
        for t in tokens {
            data.extend_from_slice(&hash_embed::<D>(t));
        }
        Self {
            n: tokens.len(),
            data,
        }
    }

    /// Sorted-dedup token set — canonical order pins the f32 summation order
    /// of `maxsim_score`'s outer sum (a `HashMap` would not).
    fn from_unique(tokens: &[String]) -> Self {
        let mut sorted: Vec<&String> = tokens.iter().collect();
        sorted.sort();
        sorted.dedup();
        let mut data = Vec::with_capacity(sorted.len() * D);
        for t in &sorted {
            data.extend_from_slice(&hash_embed::<D>(t));
        }
        let n = sorted.len();
        Self { data, n }
    }

    /// The shipped late-interaction kernel: `Σ_i max_j dot(q_i, d_j)`.
    #[inline]
    #[must_use]
    pub fn maxsim(&self, query: &Self) -> f32 {
        katgpt_core::simd::maxsim_score(&query.data, &self.data, query.n, self.n, D)
    }

    fn digest_into(&self, hasher: &mut Hasher) {
        hasher.update(&(self.n as u64).to_le_bytes());
        for v in &self.data {
            hasher.update(&v.to_le_bytes());
        }
    }
}

fn tokenize(code: &str, out: &mut Vec<String>) {
    out.clear();
    RustOpTokenizer.tokenize_into(code, out);
}

/// The per-domain corpus of hashed rule documents.
#[derive(Debug, Clone)]
struct DomainDocs<const D: usize> {
    domain: DomainId,
    /// rule_id-sorted (canonical build order → deterministic digests).
    rules: Vec<(String, TokenMatrix<D>)>,
}

impl<const D: usize> DomainDocs<D> {
    fn build(domain: DomainId, mut entries: Vec<(String, String)>) -> Self {
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut toks = Vec::new();
        let rules = entries
            .into_iter()
            .map(|(id, buggy)| {
                tokenize(&buggy, &mut toks);
                // Document side: dedup is EXACT under MaxSim (module docs).
                (id, TokenMatrix::<D>::from_unique(&toks))
            })
            .collect();
        Self { domain, rules }
    }
}

/// The multi-vector index over the enabled domains' rule corpora — the SAME
/// enumeration the shipped matcher indexes (arm (a)'s `rule_book_map`), never
/// a second corpus source that could drift from it.
#[derive(Debug, Clone)]
pub struct MultivectorIndex<const D: usize> {
    domains: Vec<DomainDocs<D>>,
    /// BLAKE3 over (scheme tag, D, every rule id + its matrix bytes) in
    /// canonical order — the determinism pin the record quotes.
    pub corpus_digest: String,
    pub model_id: String,
}

impl<const D: usize> MultivectorIndex<D> {
    /// `rules`: (`rule_id`, domain, `buggy_code`) triples — the same corpus
    /// enumeration as arm (a). Duplicated rule ids within a domain are a
    /// caller bug and are refused loud (the shipped matcher's index would
    /// silently key-collide; here the harness must not).
    ///
    /// # Panics
    /// On a duplicate (domain, `rule_id`) pair.
    #[must_use]
    pub fn from_rules(rules: &[(String, DomainId, String)]) -> Self {
        let mut by_domain: Vec<(DomainId, Vec<(String, String)>)> = Vec::new();
        for (id, domain, buggy) in rules {
            match by_domain.iter_mut().find(|(d, _)| d == domain) {
                Some((_, entries)) => entries.push((id.clone(), buggy.clone())),
                None => by_domain.push((*domain, vec![(id.clone(), buggy.clone())])),
            }
        }
        let mut domains: Vec<DomainDocs<D>> = by_domain
            .into_iter()
            .map(|(domain, entries)| DomainDocs::<D>::build(domain, entries))
            .collect();
        domains.sort_by_key(|d| domain_index(d.domain));

        let mut hasher = Hasher::new();
        hasher.update(SCHEME_TAG.as_bytes());
        hasher.update(&(D as u64).to_le_bytes());
        for dd in &domains {
            hasher.update(&(dd.domain as u8).to_le_bytes());
            for (id, matrix) in &dd.rules {
                hasher.update(id.as_bytes());
                matrix.digest_into(&mut hasher);
            }
        }
        Self {
            corpus_digest: hasher.finalize().to_hex()[..16].to_string(),
            model_id: token_model_id(D),
            domains,
        }
    }

    #[must_use]
    pub fn has_domain(&self, domain: DomainId) -> bool {
        self.domains.iter().any(|d| d.domain == domain)
    }

    #[must_use]
    pub fn rule_count(&self, domain: DomainId) -> usize {
        self.domains
            .iter()
            .find(|d| d.domain == domain)
            .map_or(0, |d| d.rules.len())
    }

    /// The enabled domains in canonical (`domain_index`) order — the harness's
    /// per-domain table iterates THIS, never a hand-typed list.
    #[must_use]
    pub fn domain_list(&self) -> Vec<DomainId> {
        self.domains.iter().map(|d| d.domain).collect()
    }

    fn query_matrix(code: &str, mode: QueryMode) -> TokenMatrix<D> {
        let mut toks = Vec::new();
        tokenize(code, &mut toks);
        match mode {
            QueryMode::All => TokenMatrix::from_tokens(&toks),
            QueryMode::Unique => TokenMatrix::from_unique(&toks),
        }
    }

    /// Arm (e) Raw: every rule of `domain` scored by full-token `MaxSim`,
    /// canonically ordered (score desc, `rule_id` asc — `canonical_rank`'s key
    /// order with every `instantiates` false).
    #[must_use]
    pub fn rank_raw(&self, domain: DomainId, code: &str, mode: QueryMode) -> Vec<ScoredRule> {
        let Some(docs) = self.domains.iter().find(|d| d.domain == domain) else {
            return Vec::new();
        };
        let query = Self::query_matrix(code, mode);
        let mut out: Vec<ScoredRule> = docs
            .rules
            .iter()
            .map(|(id, doc)| ScoredRule {
                rule_id: id.clone(),
                score: doc.maxsim(&query),
                instantiates: false,
            })
            .collect();
        out.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.rule_id.cmp(&b.rule_id))
        });
        out
    }

    /// Arm (e) Full: the shipped pipeline shape with only the score channel
    /// swapped — MaxSim-ranked pool of `max(top_k, RERANK_POOL)`, the SAME
    /// instantiation marking + `rerank` + truncate the incumbent fan-out runs
    /// (parse-fail contract included: no parse, no marking, flags stay false).
    #[must_use]
    pub fn rank_full(
        &self,
        domain: DomainId,
        code: &str,
        mode: QueryMode,
        top_k: usize,
    ) -> Vec<ScoredRule> {
        let pool_k = top_k.max(RERANK_POOL);
        let mut pool: Vec<ScoredRule> = self
            .rank_raw(domain, code, mode)
            .into_iter()
            .take(pool_k)
            .collect();
        if pool.is_empty() {
            return pool;
        }
        if let Ok(file) = syn::parse_file(code) {
            mark_instantiation_with_file(domain, code, &file, &mut pool, RerankMode::default());
        }
        rerank(&mut pool);
        pool.truncate(top_k);
        pool
    }
}

/// Query-side token weighting (module docs — the design axis T1 inherits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryMode {
    /// Every op-token, duplicates kept — count-weighted. The literal arm (e).
    #[default]
    All,
    /// One vote per token type.
    Unique,
}

/// How the `MaxSim` ranking integrates with the shipped pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Integration {
    /// Pure `MaxSim` ranking — the representation-only read.
    #[default]
    Raw,
    /// Pool + instantiation marking + rerank + truncate, incumbent-shaped.
    Full,
}

/// Per-domain hit counts (the eval's hits@1/3/5 shape).
#[derive(Debug, Clone, Copy, Default)]
pub struct Hits {
    pub labeled: usize,
    pub at_1: usize,
    pub at_3: usize,
    pub at_5: usize,
}

impl Hits {
    fn note(&mut self, rank: Option<usize>) {
        self.labeled += 1;
        if rank.is_some_and(|r| r < 1) {
            self.at_1 += 1;
        }
        if rank.is_some_and(|r| r < 3) {
            self.at_3 += 1;
        }
        if rank.is_some_and(|r| r < 5) {
            self.at_5 += 1;
        }
    }

    /// Per-span top-1 outcome — the paired read's unit.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // display metric — the retrieval_eval precedent, one module over
    pub fn rate(&self, k: usize) -> f64 {
        let denom = self.labeled.max(1) as f64;
        let num = match k {
            1 => self.at_1,
            3 => self.at_3,
            _ => self.at_5,
        };
        num as f64 / denom
    }
}

#[derive(Debug, Clone)]
pub struct MissRow {
    pub expected: String,
    pub domain: DomainId,
    pub rank: Option<usize>,
    pub top1: Option<String>,
}

/// One arm's measurement over the labeled fixture battery.
#[derive(Debug, Clone)]
pub struct ArmReport {
    pub per_domain: [Hits; 7],
    pub blended: Hits,
    /// Clean-excerpt top-1 per enabled domain (the false-positive context).
    pub clean_top1: Vec<(String, Vec<(DomainId, String)>)>,
    pub misses: Vec<MissRow>,
    /// Per-span top-1 hits in `labeled_eval_spans` order — the paired read.
    pub span_top1: Vec<(String, bool)>,
    /// Index build wall time (tokenize + embed the whole rule corpus).
    pub build: Duration,
    /// Span scoring wall time (labeled battery; clean profile excluded).
    pub score: Duration,
    /// Scored (span, rule) pairs — the per-pair µs denominator.
    pub pairs: usize,
}

impl ArmReport {
    fn new() -> Self {
        Self {
            per_domain: [Hits::default(); 7],
            blended: Hits::default(),
            clean_top1: Vec::new(),
            misses: Vec::new(),
            span_top1: Vec::new(),
            build: Duration::ZERO,
            score: Duration::ZERO,
            pairs: 0,
        }
    }

    #[must_use]
    pub fn domain_hits(&self, domain: DomainId) -> Hits {
        self.per_domain[domain_index(domain)]
    }
}

/// Run one arm of the kill-test over the labeled fixture battery (the
/// `retrieval_eval` spans, own-domain ranking, hits@1/3/5) + optionally the
/// clean-excerpt profile.
#[must_use]
pub fn run_arm<const D: usize>(
    idx: &MultivectorIndex<D>,
    integration: Integration,
    mode: QueryMode,
    with_clean: bool,
) -> ArmReport {
    let mut rep = ArmReport::new();
    let spans = labeled_eval_spans();
    let t0 = Instant::now();
    for span in &spans {
        let domain = span.domain;
        if !idx.has_domain(domain) {
            continue;
        }
        let ranked = match integration {
            Integration::Raw => idx.rank_raw(domain, &span.code, mode),
            Integration::Full => idx.rank_full(domain, &span.code, mode, PROBE_TOP_K),
        };
        rep.pairs += idx.rule_count(domain);
        let canonical = canonical_rank(&ranked);
        let rank = canonical.iter().position(|r| r.rule_id == span.label);
        let top1_hit = rank.is_some_and(|r| r < 1);
        rep.per_domain[domain_index(domain)].note(rank);
        rep.blended.note(rank);
        rep.span_top1.push((span.label.clone(), top1_hit));
        if rank.is_none_or(|r| r >= 3) {
            rep.misses.push(MissRow {
                expected: span.label.clone(),
                domain,
                rank,
                top1: canonical.first().map(|r| r.rule_id.clone()),
            });
        }
    }
    rep.score = t0.elapsed();
    if with_clean {
        for (tag, code) in CLEAN_SPANS {
            let mut tops = Vec::new();
            for dd in &idx.domains {
                let ranked = match integration {
                    Integration::Raw => idx.rank_raw(dd.domain, code, mode),
                    Integration::Full => idx.rank_full(dd.domain, code, mode, PROBE_TOP_K),
                };
                if let Some(r) = canonical_rank(&ranked).first() {
                    tops.push((dd.domain, r.rule_id.clone()));
                }
            }
            rep.clean_top1.push((tag.to_string(), tops));
        }
    }
    rep
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // The comparisons are deliberate BIT-exactness pins on a deterministic
    // hash-derived vector: float equality IS the assertion (any drift means
    // the scheme moved).
    #[allow(clippy::float_cmp)]
    fn hash_embed_is_deterministic_and_unit() {
        let a = hash_embed::<64>("op:&*");
        let b = hash_embed::<64>("op:&*");
        assert_eq!(a, b, "same token must hash identically");
        let norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "unit norm, got {norm}");
        let c = hash_embed::<64>("op:->");
        assert_ne!(a, c, "distinct tokens must embed distinctly");
    }

    /// The scheme pin: `SCHEME_TAG` + layout + this digest must move together.
    /// Any change to `hash_embed`'s byte layout, normalization, or the tag
    /// REFUSES to look like the old scheme — bump `SCHEME_TAG` and re-pin here.
    #[test]
    fn hash_embed_digest_pin_d8() {
        let mut h = Hasher::new();
        h.update(SCHEME_TAG.as_bytes());
        h.update(&8u64.to_le_bytes());
        for v in hash_embed::<8>("op:&*") {
            h.update(&v.to_le_bytes());
        }
        let digest = h.finalize().to_hex()[..16].to_string();
        assert_eq!(digest, "3466d6ed36f96a0c", "hash_embed scheme moved");
    }

    #[test]
    // Same bit-exactness pin: MaxSim over a multiset must equal MaxSim over
    // its support, exactly.
    #[allow(clippy::float_cmp)]
    fn document_dedup_is_exact_under_maxsim() {
        // max over a multiset with duplicate rows == max over its support.
        let q: Vec<String> = vec!["fn".into(), "op:&*".into()];
        let doc_dup: Vec<String> = vec!["op:&*".into(), "op:&*".into(), "x".into()];
        let doc_uniq: Vec<String> = vec!["op:&*".into(), "x".into()];
        let qm = TokenMatrix::<32>::from_tokens(&q);
        let dm_dup = TokenMatrix::<32>::from_tokens(&doc_dup);
        let dm_uniq = TokenMatrix::<32>::from_unique(&doc_uniq);
        let a = dm_dup.maxsim(&qm);
        let b = dm_uniq.maxsim(&qm);
        assert_eq!(a, b, "duplicate doc rows must not change MaxSim");
    }

    #[test]
    fn token_model_ids_are_width_distinct() {
        assert_ne!(token_model_id(8), token_model_id(64));
    }
}

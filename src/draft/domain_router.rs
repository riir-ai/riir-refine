//! Cross-domain router — which healing domain should own a code span
//! (Plan 005 T1.4).
//!
//! Routing signal: the code chunk's [`ModellessEmbedder`] direction (8-D
//! structural fingerprint) scored against per-domain **centroid directions**,
//! each the L2-normalized mean of that domain's corpus pattern embeddings.
//! Deterministic, modelless (no trained weights — centroids are derived from
//! the corpora at build time).
//!
//! ## Why not `katgpt_core::pick_domain::<3, 8>` directly
//!
//! `pick_domain` is `const`-generic over the domain count `N`, but the
//! enabled-domain set here is **feature-dependent** (clippy+perf by default;
//! +kernel with `kernel_opt`) — a runtime-sized direction list. `route()`
//! therefore implements the identical dot-product argmax (ties → lowest
//! index) over `Vec` directions, mirroring
//! `katgpt_core::variable_rank_domain_expert::pick_domain` (Plan 558) — the
//! same routing primitive the kernel-15 expert router consumes at
//! compile-time N.

//! ## Issue 020 POC: the spectral routing block
//!
//! The baseline embedding (`ModellessEmbedder`, 8-D) carries 4 BLAKE3
//! hash-bucket dims — hash avalanche gives them **zero locality** (similar
//! code → dissimilar embedding), which is why the measured routing accuracy
//! is 53.4% (2-domain) / 46.6% (3-domain) (Bench 022). The POC adds a
//! **router-local spectral block**: k DFT coefficient pairs computed over a
//! **token-frequency vector** (top-N tokens shared across the enabled
//! domains' corpora). Token distributions have locality by construction —
//! similar code shares tokens — which is exactly what BLAKE3 lacks.
//!
//! Router-local by design: riir-rag's 8-D `ModellessEmbedder` contract is
//! NOT touched (it is pinned by `ShardIndex::retrieve_diverse(&[f32; 8])`
//! and by Bench 018's G2 numbers). Everything here lives + dies inside the
//! router.
//!
//! Two vocab orderings are measured (the axis the DFT runs over):
//! - `GlobalFreq` — tokens ranked by global corpus frequency (the naive
//!   reading of "top-N tokens").
//! - `Discriminative` — tokens ranked by cross-domain TF range
//!   (`max_d p_d − min_d p_d`), so domain-separating tokens sit at low
//!   indices where the low-k DFT coefficients can see them.
//!
//! The `TfCosine` mode is the **ablation reference** (full-N TF dot, no DFT
//! compression) — it upper-bounds what any projection of the token-frequency
//! signal can achieve and splits a FAIL into "lexical signal insufficient"
//! vs "the k-coefficient projection loses it".

use crate::ragplane::{query::LATENT_DIM, Embedder, ModellessEmbedder};

/// The healing domains — index into the router's direction list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainId {
    /// `clippy_lints` — lint-shaped style/complexity fixes.
    Clippy,
    /// `rust_perf` — perf-anti-pattern fixes.
    Perf,
    /// `kernel_opt` — GPU kernel optimization patterns.
    Kernel,
    /// `sec` — security-pattern fixes (FFI/Drop/pointer hygiene, Plan 018).
    Sec,
    /// `docker` — Dockerfile bounded-fix patterns (Plan 030).
    Docker,
    /// `dist` — shipped-artifact build-config advice (Issue 085).
    Dist,
    /// `shader_perf` — WGSL render-pipeline bounded-fix patterns (Plan 134
    /// / Proposal 014; `@compute` stays `kernel_opt` — the entry-point
    /// routing split).
    ShaderPerf,
}

/// A set of [`DomainId`]s (one bit per domain, `ALL` order) — the per-span
/// fan-out allow set (Issue 213). Copy, zero-allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomainMask(u8);

impl DomainMask {
    /// Every domain allowed — the path-less / pre-213 fan-out.
    pub const ALL: Self = Self((1 << DomainId::ALL.len()) - 1);
    /// No domain allowed.
    pub const NONE: Self = Self(0);

    const fn bit(domain: DomainId) -> u8 {
        1 << domain as u8
    }

    /// Whether `domain` is in the set.
    #[must_use]
    pub const fn allows(self, domain: DomainId) -> bool {
        self.0 & Self::bit(domain) != 0
    }

    /// The set with `domain` added.
    #[must_use]
    pub const fn with(self, domain: DomainId) -> Self {
        Self(self.0 | Self::bit(domain))
    }

    /// The set with `domain` removed.
    #[must_use]
    pub const fn without(self, domain: DomainId) -> Self {
        Self(self.0 & !Self::bit(domain))
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The domains among `present` whose rules can touch a file at `path`
    /// ([`DomainId::applies_to_path`], the Issue 145 T4 contract).
    #[must_use]
    pub fn applying_to_path(path: &str, present: impl Fn(DomainId) -> bool) -> Self {
        DomainId::ALL
            .into_iter()
            .filter(|&d| present(d) && d.applies_to_path(path))
            .fold(Self::NONE, Self::with)
    }
}

impl DomainId {
    /// All domains, in router order.
    pub const ALL: [Self; 7] = [
        Self::Clippy,
        Self::Perf,
        Self::Kernel,
        Self::Sec,
        Self::Docker,
        Self::Dist,
        Self::ShaderPerf,
    ];

    /// Stable label (feature-independent).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Clippy => "clippy",
            Self::Perf => "perf",
            Self::Kernel => "kernel",
            Self::Sec => "sec",
            Self::Docker => "docker",
            Self::Dist => "dist",
            Self::ShaderPerf => "shader",
        }
    }

    /// Whether this domain's rules can apply to a file at `path` (Issue
    /// 145 T4): Rust-source domains to `.rs`, `docker` to Dockerfiles,
    /// `dist` to Cargo manifests, shaders to `.wgsl` (`kernel` also covers
    /// `@compute` WGSL and CUDA/Metal sources). Display-side only — a span's
    /// suggestion list drops domains that cannot touch its file kind.
    #[must_use]
    pub fn applies_to_path(self, path: &str) -> bool {
        let p = path.replace('\\', "/");
        let name = p.rsplit('/').next().unwrap_or(&p);
        let ext = |e: &str| name.ends_with(e);
        match self {
            Self::Clippy | Self::Perf | Self::Sec => ext(".rs"),
            Self::Kernel => ext(".rs") || ext(".wgsl") || ext(".cu") || ext(".metal"),
            Self::ShaderPerf => ext(".wgsl"),
            Self::Docker => {
                name.starts_with("Dockerfile") || name == "Containerfile" || ext(".dockerfile")
            }
            Self::Dist => name == "Cargo.toml" || p.ends_with(".cargo/config.toml"),
        }
    }

    /// Inverse of [`DomainId::label`] (`None` for an unknown label).
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|d| d.label() == label)
    }

    /// Whether this domain's corpus feature is compiled into THIS build —
    /// the same gates `LatentMatcher` indexes under, so a CLI can list the
    /// domains it can actually run without building a matcher.
    #[must_use]
    pub const fn is_compiled(self) -> bool {
        match self {
            Self::Clippy => cfg!(feature = "clippy_lints"),
            Self::Perf => cfg!(feature = "rust_perf"),
            Self::Kernel => cfg!(feature = "kernel_opt"),
            Self::Sec => cfg!(feature = "sec"),
            Self::Docker => cfg!(feature = "docker"),
            Self::Dist => cfg!(feature = "dist"),
            Self::ShaderPerf => cfg!(feature = "shader_perf"),
        }
    }

    /// The compiled domains, in router order.
    pub fn compiled() -> impl Iterator<Item = Self> {
        Self::ALL.into_iter().filter(|d| d.is_compiled())
    }
}

/// Cross-domain centroid router.
///
/// Built from the ENABLED domains' corpus pattern texts. A disabled domain
/// simply has no direction — it can never be routed to.
pub struct DomainRouter {
    /// Baseline 8-D centroids (materialized only in `Baseline` mode).
    directions: Vec<[f32; LATENT_DIM]>,
    /// DFT-block centroids (materialized only in `SpectralDft` mode).
    spectral: Vec<[f32; SPECTRAL_DIM]>,
    /// Full-N TF centroids (materialized only in `TfCosine` mode).
    tf: Vec<[f32; SPECTRAL_VOCAB_N]>,
    /// Structural-4 suffix centroids (`SpectralDft { with_struct: true }`).
    struct4: Vec<[f32; 4]>,
    /// The shared spectral vocab + twiddle tables (spectral modes only).
    vocab: Option<SpectralVocab>,
    mode: RoutingEmbedding,
    ids: Vec<DomainId>,
}

impl DomainRouter {
    /// Build from `(domain, pattern_texts)` — one entry per enabled domain.
    ///
    /// Each domain's direction = L2-normalized mean of its pattern
    /// embeddings.
    ///
    /// # Panics
    ///
    /// Panics if any domain has an empty pattern list (a centroid of nothing
    /// is undefined).
    #[must_use]
    pub fn new(domains: &[(DomainId, Vec<String>)]) -> Self {
        Self::with_embedding(domains, RoutingEmbedding::Baseline)
    }

    /// Build with an explicit routing embedding (the Issue 020 POC
    /// constructor; [`DomainRouter::new`] delegates with `Baseline`).
    ///
    /// # Panics
    ///
    /// Panics if any domain has an empty pattern list.
    #[must_use]
    pub fn with_embedding(domains: &[(DomainId, Vec<String>)], mode: RoutingEmbedding) -> Self {
        let mut ids = Vec::with_capacity(domains.len());
        for (id, patterns) in domains {
            assert!(
                !patterns.is_empty(),
                "DomainRouter::new: domain {} has no patterns",
                id.label()
            );
            ids.push(*id);
        }

        // Baseline centroids (8-D) — only materialized in Baseline mode.
        let mut directions = Vec::new();
        if mode == RoutingEmbedding::Baseline {
            let embedder = ModellessEmbedder;
            for (_id, patterns) in domains {
                let mut centroid = [0.0f32; LATENT_DIM];
                for pattern in patterns {
                    let emb = embedder.embed(pattern);
                    for (c, v) in centroid.iter_mut().zip(emb) {
                        *c += v;
                    }
                }
                normalize_into(&mut centroid, patterns.len());
                directions.push(centroid);
            }
        }

        // Spectral block: shared vocab from ALL domains' pattern texts.
        let mut spectral = Vec::new();
        let mut tf = Vec::new();
        let mut struct4 = Vec::new();
        let mut vocab = None;
        match mode {
            RoutingEmbedding::Baseline => {}
            RoutingEmbedding::SpectralDft { order, with_struct } => {
                let v = SpectralVocab::build(domains, order);
                for (_id, patterns) in domains {
                    let mut centroid = [0.0f32; SPECTRAL_DIM];
                    for pattern in patterns {
                        let vec = v.dft_embed(pattern);
                        for (c, x) in centroid.iter_mut().zip(vec) {
                            *c += x;
                        }
                    }
                    normalize_into(&mut centroid, patterns.len());
                    spectral.push(centroid);

                    if with_struct {
                        let mut sc = [0.0f32; 4];
                        for pattern in patterns {
                            let s = structural4(pattern);
                            for (c, x) in sc.iter_mut().zip(s) {
                                *c += x;
                            }
                        }
                        normalize_into(&mut sc, patterns.len());
                        struct4.push(sc);
                    }
                }
                vocab = Some(v);
            }
            RoutingEmbedding::TfCosine { order } => {
                let v = SpectralVocab::build(domains, order);
                for (_id, patterns) in domains {
                    let mut centroid = [0.0f32; SPECTRAL_VOCAB_N];
                    for pattern in patterns {
                        let vec = v.tf_embed(pattern);
                        for (c, x) in centroid.iter_mut().zip(vec) {
                            *c += x;
                        }
                    }
                    normalize_into(&mut centroid, patterns.len());
                    tf.push(centroid);
                }
                vocab = Some(v);
            }
        }

        Self {
            directions,
            spectral,
            tf,
            struct4,
            vocab,
            mode,
            ids,
        }
    }

    /// Which domains this router knows.
    #[must_use]
    pub fn domains(&self) -> &[DomainId] {
        &self.ids
    }

    /// Route a code chunk to its owning domain: argmax over
    /// `dot(embedding, domain_centroid)`; ties → lowest domain index
    /// (deterministic, mirroring `pick_domain`).
    ///
    /// In spectral modes the embedding is the token-frequency block (see the
    /// module docs) — stack-resident scratch, zero allocation.
    ///
    /// Returns `None` if the router was built with no domains.
    ///
    /// # Panics
    ///
    /// Panics if a spectral mode was constructed without its vocab (a
    /// construction-time invariant — cannot happen via
    /// [`DomainRouter::with_embedding`]).
    #[must_use]
    pub fn route(&self, code: &str) -> Option<DomainId> {
        if self.ids.is_empty() {
            return None;
        }
        match self.mode {
            RoutingEmbedding::Baseline => {
                let activity = ModellessEmbedder.embed(code);
                self.route_activity(&activity)
            }
            RoutingEmbedding::SpectralDft { with_struct, .. } => {
                let vocab = self.vocab.as_ref().expect("spectral mode implies a vocab");
                let spec = vocab.dft_embed(code);
                let struc = if with_struct {
                    Some(structural4(code))
                } else {
                    None
                };
                self.argmax_spectral(&spec, struc.as_ref())
            }
            RoutingEmbedding::TfCosine { .. } => {
                let vocab = self.vocab.as_ref().expect("tf mode implies a vocab");
                let vec = vocab.tf_embed(code);
                self.argmax_tf(&vec)
            }
        }
    }

    /// Argmax over `dot(spec, spectral[i]) + dot(struc, struct4[i])`.
    fn argmax_spectral(
        &self,
        spec: &[f32; SPECTRAL_DIM],
        struc: Option<&[f32; 4]>,
    ) -> Option<DomainId> {
        let mut best = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for i in 0..self.ids.len() {
            let mut s = 0.0f32;
            for (a, d) in spec.iter().zip(&self.spectral[i]) {
                s += a * d;
            }
            if let (Some(struc), Some(sc)) = (struc, self.struct4.get(i)) {
                for (a, d) in struc.iter().zip(sc) {
                    s += a * d;
                }
            }
            if s.partial_cmp(&best_score) == Some(std::cmp::Ordering::Greater) {
                best_score = s;
                best = i;
            }
        }
        self.ids.get(best).copied()
    }

    /// Argmax over `dot(vec, tf[i])`.
    fn argmax_tf(&self, vec: &[f32; SPECTRAL_VOCAB_N]) -> Option<DomainId> {
        let mut best = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for i in 0..self.ids.len() {
            let mut s = 0.0f32;
            for (a, d) in vec.iter().zip(&self.tf[i]) {
                s += a * d;
            }
            if s.partial_cmp(&best_score) == Some(std::cmp::Ordering::Greater) {
                best_score = s;
                best = i;
            }
        }
        self.ids.get(best).copied()
    }

    /// Route a pre-computed embedding (avoids re-embedding when the caller
    /// already has the direction).
    #[must_use]
    pub fn route_activity(&self, activity: &[f32; LATENT_DIM]) -> Option<DomainId> {
        let mut best = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for (i, dir) in self.directions.iter().enumerate() {
            let mut s = 0.0f32;
            for (a, d) in activity.iter().zip(dir) {
                s += a * d;
            }
            // partial_cmp: NaN ties → keeps the earlier index (same as
            // pick_domain's tie semantics).
            if s.partial_cmp(&best_score) == Some(std::cmp::Ordering::Greater) {
                best_score = s;
                best = i;
            }
        }
        self.ids.get(best).copied()
    }
}

/// Divide `v` by `n` and L2-normalize in place (mean-of-embeddings → unit
/// centroid). Zero-norm centroids stay zero.
#[allow(clippy::cast_precision_loss)] // corpus sizes are ≤ ~10²
fn normalize_into(v: &mut [f32], n: usize) {
    let n = n.max(1) as f32;
    let mut norm = 0.0f32;
    for c in v.iter_mut() {
        *c /= n;
        norm += *c * *c;
    }
    let norm = norm.sqrt();
    if norm > f32::EPSILON {
        for c in v.iter_mut() {
            *c /= norm;
        }
    }
}

/// The 4 structural dims of the modelless embedding (dims 4-7: byte-DFT
/// rhythm, symbol density, nesting depth, line-length entropy),
/// re-normalized to unit length — the BLAKE3 hash-bucket dims (0-3) are
/// dropped (they are the locality killer this POC exists to fix).
fn structural4(text: &str) -> [f32; 4] {
    let emb = ModellessEmbedder.embed(text);
    let mut s = [emb[4], emb[5], emb[6], emb[7]];
    let norm = s.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in &mut s {
            *x /= norm;
        }
    }
    s
}

// ── Issue 020 POC: spectral routing block ─────────────────────────────

/// Vocab size — the token-frequency vector axis length.
pub const SPECTRAL_VOCAB_N: usize = 128;

/// DFT coefficient pairs kept (k = 1..=8). The DC term (k=0) is skipped —
/// it is constant for L2-normalized frequency vectors.
pub const SPECTRAL_K: usize = 8;

/// DFT block dimensionality (re + im per coefficient).
pub const SPECTRAL_DIM: usize = 2 * SPECTRAL_K;

/// Minimum token length (single-char tokens are near-universal noise).
const MIN_TOKEN_LEN: usize = 2;

/// How the shared vocab (the frequency-vector axis) is ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralOrder {
    /// Tokens ranked by global corpus frequency (count desc, token asc).
    GlobalFreq,
    /// Tokens ranked by cross-domain TF range (`max_d p_d − min_d p_d`
    /// desc, token asc) — domain-separating tokens sit at low indices where
    /// the low-k DFT coefficients can see them.
    Discriminative,
}

/// Which embedding the router scores spans against centroids with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingEmbedding {
    /// The shipped 8-D `ModellessEmbedder` centroids (BLAKE3 dims included —
    /// locality-blind, the measured 53.4%/46.6% baseline).
    Baseline,
    /// k-pair token-frequency DFT block, optionally suffixed with the
    /// 4 structural dims (`with_struct`) — each part unit-normalized, the
    /// score is the sum of the two dots (equal weight).
    SpectralDft {
        order: SpectralOrder,
        with_struct: bool,
    },
    /// Ablation reference: full-N token-frequency cosine (no DFT
    /// compression). Upper-bounds every projection of the lexical signal.
    TfCosine { order: SpectralOrder },
}

/// The shared token vocabulary + precomputed DFT twiddle tables.
///
/// Deterministic by construction: vocab order uses (score desc, token asc)
/// tie-breaks; the lookup map is lookup-only (never iterated).
pub struct SpectralVocab {
    tokens: Vec<Box<str>>,
    lookup: std::collections::HashMap<Box<str>, u16>,
    /// `(cos θ, sin θ)` for θ = 2π·k·n/N, indexed `[k-1][n]`.
    tables: [[(f32, f32); SPECTRAL_VOCAB_N]; SPECTRAL_K],
}

impl SpectralVocab {
    /// Build the shared vocab from every enabled domain's pattern texts.
    ///
    /// Tokens = runs of `[A-Za-z0-9_]` with length ≥ 2 (case-sensitive —
    /// Rust keywords + API casing carry signal).
    #[must_use]
    pub fn build(domains: &[(DomainId, Vec<String>)], order: SpectralOrder) -> Self {
        let texts: Vec<Vec<String>> = domains.iter().map(|(_, p)| p.clone()).collect();
        Self::build_from_patterns(&texts, order)
    }

    /// [`SpectralVocab::build`] over RAW pattern groups — no `DomainId`
    /// labels. The Issue-196 category router reuses the vocab machinery at
    /// kernel-CATEGORY granularity (15 sibling groups of one domain), which
    /// `DomainId` cannot type; group INDEX is the identity here.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // token counts/totals are ≤ ~10⁵; f32 ranks them exactly
    pub fn build_from_patterns(group_texts: &[Vec<String>], order: SpectralOrder) -> Self {
        use std::collections::BTreeMap;

        // Per-group token counts (BTreeMap: deterministic iteration + no
        // hash-order leakage into the vocab ranking). Vec-per-token so the
        // group count is not hardcoded (4 domains under `sec`; Plan 018).
        let ndomains = group_texts.len();
        let mut counts: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        let mut totals = vec![0u64; ndomains];
        for (di, patterns) in group_texts.iter().enumerate() {
            for pattern in patterns {
                for_each_token(pattern, &mut |tok| {
                    counts.entry(tok).or_insert(vec![0; ndomains])[di] += 1;
                    totals[di] += 1;
                });
            }
        }

        // Rank tokens: score desc, then token asc — both deterministic.
        #[allow(clippy::cast_possible_truncation)] // vocab ≤ 128 « u16::MAX
        let mut ranked: Vec<(f32, &str)> = counts
            .iter()
            .map(|(&tok, per)| {
                let score = match order {
                    SpectralOrder::GlobalFreq => per.iter().map(|c| *c as f32).sum::<f32>(),
                    SpectralOrder::Discriminative => {
                        let mut lo = f32::INFINITY;
                        let mut hi = f32::NEG_INFINITY;
                        for (di, c) in per.iter().enumerate() {
                            // Per-domain TF share (domain totals are nonzero —
                            // each domain asserts non-empty patterns).
                            let p = *c as f32 / totals[di].max(1) as f32;
                            lo = lo.min(p);
                            hi = hi.max(p);
                        }
                        hi - lo
                    }
                };
                (score, tok)
            })
            .collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        ranked.truncate(SPECTRAL_VOCAB_N);

        let tokens: Vec<Box<str>> = ranked.into_iter().map(|(_, t)| t.into()).collect();
        #[allow(clippy::cast_possible_truncation)] // vocab ≤ 128 « u16::MAX
        let lookup = tokens
            .iter()
            .enumerate()
            .map(|(i, t)| (t.clone(), i as u16))
            .collect();

        // Precompute twiddle tables — embed stays pure FMAs (the <10µs/span
        // cost gate; naive per-pair cos/sin is ~2K transcendentals ≈ 40µs).
        let mut tables = [[(0.0f32, 0.0f32); SPECTRAL_VOCAB_N]; SPECTRAL_K];
        for (ki, table) in tables.iter_mut().enumerate() {
            let kf = (ki + 1) as f32;
            for (n, slot) in table.iter_mut().enumerate() {
                let theta = 2.0 * std::f32::consts::PI * (kf * n as f32 / SPECTRAL_VOCAB_N as f32);
                *slot = (theta.cos(), theta.sin());
            }
        }

        Self {
            tokens,
            lookup,
            tables,
        }
    }

    /// The vocab in axis order (deterministic; tests pin it).
    #[must_use]
    pub fn tokens(&self) -> &[Box<str>] {
        &self.tokens
    }

    /// Tokenize `text`, counting occurrences per vocab slot. Returns the
    /// TOTAL token count (vocab + non-vocab — non-vocab tokens dilute the
    /// TF distribution, keeping short spans comparable).
    fn count_into(&self, text: &str, counts: &mut [u32; SPECTRAL_VOCAB_N]) -> u32 {
        let mut total = 0u32;
        for_each_token(text, &mut |tok| {
            total += 1;
            if let Some(&i) = self.lookup.get(tok) {
                counts[i as usize] += 1;
            }
        });
        total
    }

    /// k-pair DFT of the token-frequency vector (L2-normalized). Skips zero
    /// counts — typical spans touch ≪ N distinct vocab tokens.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // token counts ≤ ~10³ per span
    pub fn dft_embed(&self, text: &str) -> [f32; SPECTRAL_DIM] {
        let mut counts = [0u32; SPECTRAL_VOCAB_N];
        let total = self.count_into(text, &mut counts);
        let mut out = [0.0f32; SPECTRAL_DIM];
        if total == 0 {
            return out;
        }
        let inv = 1.0 / total as f32;
        for (ki, table) in self.tables.iter().enumerate() {
            let mut re = 0.0f32;
            let mut im = 0.0f32;
            for (n, &c) in counts.iter().enumerate() {
                if c == 0 {
                    continue;
                }
                let x = c as f32 * inv;
                let (cos, sin) = table[n];
                re += x * cos;
                im -= x * sin;
            }
            out[2 * ki] = re;
            out[2 * ki + 1] = im;
        }
        let norm = out.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > f32::EPSILON {
            for x in &mut out {
                *x /= norm;
            }
        }
        out
    }

    /// Full-N token-frequency vector (L2-normalized) — the `TfCosine`
    /// ablation embedding.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // token counts ≤ ~10³ per span
    pub fn tf_embed(&self, text: &str) -> [f32; SPECTRAL_VOCAB_N] {
        let mut counts = [0u32; SPECTRAL_VOCAB_N];
        let total = self.count_into(text, &mut counts);
        let mut out = [0.0f32; SPECTRAL_VOCAB_N];
        if total == 0 {
            return out;
        }
        let inv = 1.0 / total as f32;
        for (o, c) in out.iter_mut().zip(counts) {
            *o = c as f32 * inv;
        }
        let norm = out.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > f32::EPSILON {
            for x in &mut out {
                *x /= norm;
            }
        }
        out
    }
}

/// Visit every token (run of `[A-Za-z0-9_]`, length ≥ 2) in `text`.
fn for_each_token<'a>(text: &'a str, f: &mut impl FnMut(&'a str)) {
    let bytes = text.as_bytes();
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        let tok_byte = b.is_ascii_alphanumeric() || b == b'_';
        if tok_byte {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            if i - s >= MIN_TOKEN_LEN {
                f(&text[s..i]);
            }
        }
    }
    if let Some(s) = start {
        if bytes.len() - s >= MIN_TOKEN_LEN {
            f(&text[s..]);
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn domains_apply_to_their_file_kinds_only() {
        use DomainId::*;
        for (path, yes, no) in [
            (
                "./src/lib.rs",
                &[Clippy, Perf, Sec, Kernel][..],
                &[Docker, Dist, ShaderPerf][..],
            ),
            (
                "deploy/Dockerfile",
                &[Docker][..],
                &[Clippy, Perf, Dist][..],
            ),
            ("Dockerfile.release", &[Docker][..], &[Dist][..]),
            ("crates/x/Cargo.toml", &[Dist][..], &[Clippy, Docker][..]),
            (".cargo/config.toml", &[Dist][..], &[Clippy][..]),
            (
                "shaders/blur.wgsl",
                &[ShaderPerf, Kernel][..],
                &[Clippy, Docker][..],
            ),
            ("C:\\repo\\src\\main.rs", &[Clippy][..], &[Docker][..]),
        ] {
            for d in yes {
                assert!(d.applies_to_path(path), "{d:?} applies to {path}");
            }
            for d in no {
                assert!(!d.applies_to_path(path), "{d:?} must not apply to {path}");
            }
        }
    }

    use super::*;

    #[test]
    fn label_round_trips_and_compiled_matches_features() {
        for d in DomainId::ALL {
            assert_eq!(DomainId::from_label(d.label()), Some(d));
        }
        assert_eq!(DomainId::from_label("auto"), None);
        // Clippy's arm reads the feature, not a constant: flip-checked
        // against the build's own cfg so a hard-coded `true` reds here.
        assert_eq!(
            DomainId::Clippy.is_compiled(),
            cfg!(feature = "clippy_lints")
        );
        assert_eq!(DomainId::Sec.is_compiled(), cfg!(feature = "sec"));
        assert!(DomainId::compiled().all(DomainId::is_compiled));
    }

    fn fixture_domains() -> Vec<(DomainId, Vec<String>)> {
        vec![
            (
                DomainId::Clippy,
                vec![
                    // Signature: short 1-line fn bodies with `return expr;`.
                    "fn a(x: i32) -> i32 { return x; }".to_string(),
                    "fn b() -> u8 { return 3; }".to_string(),
                    "fn c(v: u64) -> u64 { return v + 1; }".to_string(),
                    // + closure shape.
                    "let v = xs.iter().map(|x| x.to_string()).collect::<Vec<_>>();".to_string(),
                ],
            ),
            (
                DomainId::Perf,
                vec![
                    // Signature: loop-shaped accumulation / scanning WITHOUT
                    // short-fn wrappers.
                    "let mut out = Vec::new(); for x in xs { out.push(f(x)); }".to_string(),
                    "let mut total = 0; for v in vals { total += v; }".to_string(),
                    "let mut found = false; for u in users { if u.id == id { found = true; } }".to_string(),
                ],
            ),
            (
                DomainId::Kernel,
                vec![
                    // Signature: nested float-accumulation loops + GPU
                    // position intrinsics.
                    "for col in 0..N { let mut acc = 0.0; for row in 0..N { acc += a[row][col] * x[row]; } y[col] = acc; }".to_string(),
                    "let tile = tile_group.thread_position_in_group();".to_string(),
                ],
            ),
        ]
    }

    #[test]
    fn centroids_are_unit_norm_and_distinct() {
        let router = DomainRouter::new(&fixture_domains());
        assert_eq!(router.domains().len(), 3);
        for (i, dir) in router.directions.iter().enumerate() {
            let norm: f32 = dir.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-4,
                "centroid {i} not unit-norm: {norm}"
            );
        }
        // Clippy vs kernel centroids must not be identical (sanity —
        // separation quality is measured by the Phase 3 G1 gate).
        let d: f32 = router.directions[0]
            .iter()
            .zip(&router.directions[2])
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt();
        assert!(d > 1e-3, "clippy/kernel centroids nearly identical (d={d})");
    }

    #[test]
    fn routes_pattern_shaped_snippets_to_their_domain() {
        let router = DomainRouter::new(&fixture_domains());
        // needless-return shape → clippy
        assert_eq!(
            router.route("fn f() -> u8 { return 7; }"),
            Some(DomainId::Clippy)
        );
        // naive matvec strided-read shape → kernel
        assert_eq!(
            router.route(
                "for c in 0..M { let mut acc = 0.0; for r in 0..K { acc += a[r][c] * x[r]; } y[c] = acc; }"
            ),
            Some(DomainId::Kernel)
        );
    }

    #[test]
    fn routing_is_deterministic() {
        let router = DomainRouter::new(&fixture_domains());
        let a = router.route("fn f() -> u8 { return 7; }");
        let b = router.route("fn f() -> u8 { return 7; }");
        assert_eq!(a, b);
    }

    #[test]
    fn empty_router_routes_none() {
        let router = DomainRouter::new(&[]);
        assert_eq!(router.route("fn f() {}"), None);
    }

    #[test]
    #[should_panic(expected = "no patterns")]
    fn empty_domain_patterns_rejected() {
        let _ = DomainRouter::new(&[(DomainId::Perf, Vec::new())]);
    }

    // ── Issue 020: spectral block unit gates ────────────────────────────

    #[test]
    fn spectral_vocab_is_deterministic_and_bounded() {
        let a = SpectralVocab::build(&fixture_domains(), SpectralOrder::GlobalFreq);
        let b = SpectralVocab::build(&fixture_domains(), SpectralOrder::GlobalFreq);
        assert_eq!(a.tokens(), b.tokens());
        assert!(a.tokens().len() <= SPECTRAL_VOCAB_N);
        assert_ne!(a.tokens(), []);
        // Single-char tokens never enter the vocab.
        assert!(a.tokens().iter().all(|t| t.len() >= 2));
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identical determinism check — same input must produce identical bytes
    fn dft_embed_is_deterministic_and_unit_norm() {
        let vocab = SpectralVocab::build(&fixture_domains(), SpectralOrder::Discriminative);
        let a = vocab.dft_embed("for x in xs { acc += x; }");
        let b = vocab.dft_embed("for x in xs { acc += x; }");
        assert_eq!(a, b);
        let norm = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "dft embed not unit-norm: {norm}");
        // Empty + token-less text → zero vector (no direction).
        assert_eq!(vocab.dft_embed(""), [0.0; SPECTRAL_DIM]);
        assert_eq!(vocab.dft_embed("; : , . ( )"), [0.0; SPECTRAL_DIM]);
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identical determinism check
    fn tf_embed_is_deterministic_and_unit_norm() {
        let vocab = SpectralVocab::build(&fixture_domains(), SpectralOrder::GlobalFreq);
        let a = vocab.tf_embed("let out = Vec::new(); for x in xs { out.push(f(x)); }");
        let b = vocab.tf_embed("let out = Vec::new(); for x in xs { out.push(f(x)); }");
        assert_eq!(a, b);
        let norm = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "tf embed not unit-norm: {norm}");
    }

    #[test]
    fn spectral_router_routes_pattern_shapes() {
        for mode in [
            RoutingEmbedding::SpectralDft {
                order: SpectralOrder::GlobalFreq,
                with_struct: false,
            },
            RoutingEmbedding::SpectralDft {
                order: SpectralOrder::Discriminative,
                with_struct: true,
            },
            RoutingEmbedding::TfCosine {
                order: SpectralOrder::Discriminative,
            },
        ] {
            let router = DomainRouter::with_embedding(&fixture_domains(), mode);
            assert_eq!(
                router.route("fn f() -> u8 { return 7; }"),
                Some(DomainId::Clippy),
                "needless-return shape must route to clippy under {mode:?}"
            );
            assert_eq!(
                router.route(
                    "for c in 0..M { let mut acc = 0.0; for r in 0..K { acc += a[r][c] * x[r]; } y[c] = acc; }"
                ),
                Some(DomainId::Kernel),
                "naive matvec shape must route to kernel under {mode:?}"
            );
        }
    }

    #[test]
    fn spectral_router_is_deterministic() {
        let router = DomainRouter::with_embedding(
            &fixture_domains(),
            RoutingEmbedding::SpectralDft {
                order: SpectralOrder::Discriminative,
                with_struct: true,
            },
        );
        for _ in 0..3 {
            assert_eq!(
                router.route("let mut total = 0; for v in vals { total += v; }"),
                Some(DomainId::Perf)
            );
        }
    }

    /// Issue 020 cost gate: routing must stay < 10µs/span (it runs per-span
    /// in `--suggest`). Wall-clock in TEST only — the routing fn itself is
    /// clock-free. Release-only: debug builds pay ~10× on the transcendental
    /// paths (the baseline `ModellessEmbedder`'s byte-histogram DFT included)
    /// — same class as the `g2_*` perf gates.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "debug pays ~10× on the transcendental paths — release-only gate (the g2_* pattern)"
    )]
    #[allow(clippy::cast_precision_loss)] // timing arithmetic
    fn spectral_route_cost_under_10us() {
        let domains = vec![
            (
                DomainId::Clippy,
                (0..40)
                    .map(|i| format!("fn f{i}(x: u32) -> u32 {{ return x + {i}; }}"))
                    .collect::<Vec<_>>(),
            ),
            (
                DomainId::Perf,
                (0..40)
                    .map(|i| {
                        format!("let mut out{i} = Vec::new(); for x in xs {{ out{i}.push(f(x)); }}")
                    })
                    .collect::<Vec<_>>(),
            ),
        ];
        let span = "pub fn find_user(users: &Vec<User>, id: u32) -> Option<&User> {\n    for u in users {\n        if u.id == id {\n            return Some(u);\n        }\n    }\n    None\n}";
        for mode in [
            RoutingEmbedding::Baseline,
            RoutingEmbedding::SpectralDft {
                order: SpectralOrder::Discriminative,
                with_struct: true,
            },
            RoutingEmbedding::TfCosine {
                order: SpectralOrder::Discriminative,
            },
        ] {
            let router = DomainRouter::with_embedding(&domains, mode);
            // Warm up (first route pays lazy page faults etc.).
            let _ = router.route(span);
            let n = 2000usize;
            let t0 = std::time::Instant::now();
            for _ in 0..n {
                let _ = router.route(span);
            }
            let per = t0.elapsed().as_nanos() as f64 / n as f64 / 1000.0;
            println!("{mode:?} route cost: {per:.2}µs/span");
            assert!(
                per < 10.0,
                "{mode:?} route cost {per:.2}µs/span exceeds the 10µs gate"
            );
        }
    }
}

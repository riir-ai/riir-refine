//! Issue 224 T1/T2/T3 — the content-addressed on-disk matcher index cache.
//!
//! The rulewright's matcher ([`super::latent_matcher::LatentMatcher`])
//! rebuilds its whole multi-domain retrieval index from the corpus on EVERY
//! process — a CLI boots per invocation, so boot = per run. After Issue
//! 224's `index_batch` root-cause fix the build stands at ~250 ms of a
//! ~340 ms warm run; this module removes the remaining rebuild: the first
//! run stores the built per-domain index state here, every later run on
//! identical inputs restores it (deserialize ≈ tens of ms — never
//! re-tokenize, never re-embed).
//!
//! **Content addressing.** The cache key is
//! `blake3(per-domain resolved corpus entries + retrieval config + embedder
//! model id + cache/ranking/bm25-snapshot format versions)`. The leased
//! epoch NUMBER is deliberately not a key input: identical corpus content
//! under a fresh lease is an identical index, so a re-lease that changes no
//! entry hits (the `--corpus-delta` freshness plane reads epochs from the
//! lease store, never from here).
//!
//! **Corruption is loud (T2).** A cache entry carries a blake3 digest of
//! everything before it; [`load`] verifies the digest BEFORE parsing, then
//! bounds-checks and structurally validates every field. Any mismatch is a
//! typed [`LoadOutcome::Corrupt`] naming the file — the caller prints one
//! loud line and rebuilds from the corpus (never a wrong index). Format
//! upgrades (any version const bump) surface the same way: the stale entry
//! is named, rebuilt, and overwritten atomically.
//!
//! **Trust model.** The cache lives in the user's own cache dir (the same
//! trust class as the trajectory store / sccache): the digest detects
//! corruption, not tampering — a local attacker who can write this dir can
//! already write `~/.cargo/bin`. Not a secrets surface: the index derives
//! from the compiled-in + leased public corpus.
//!
//! **Env seams.**
//! - [`CACHE_DIR_ENV`] (`RIIR_REFINE_INDEX_CACHE_DIR`) — moves the cache
//!   dir (tests + A/B measurement).
//! - [`DEMOTE_ENV`] (`RIIR_REFINE_INDEX_CACHE`) — the exact-literal `0`
//!   demote (DEMOTE-ONLY; at the opt-in stage the compile-time feature is
//!   the only arm, per the env-switch law).
//! - [`MAX_MB_ENV`] (`RIIR_REFINE_INDEX_CACHE_MAX_MB`) — the prune's byte
//!   ceiling (issue 226 T3; unset = [`DEFAULT_MAX_CACHE_MB`], `0` =
//!   uncapped, junk = the default).
//!
//! **Scope guards.** The lane disarms (never stores, never restores, the
//! build path stays byte-identical) under `kernel_route_narrow` (16
//! sub-indexes do not fit the flat snapshot shape) and under the Bonsai
//! dense posture (the dense embedder is not serialized — restoring without
//! it would change query-side ranking).
//!
//! **T3** rides the same dir: [`auto_corpus_stamp_path`] moves the 12 h
//! auto-corpus attempt stamp from the crate's `.refine/` to the per-user
//! cache, so a new crate on a machine that already checked the current
//! epoch skips the ~0.6–1.3 s network fetch.

use std::path::{Path, PathBuf};

use super::corpus_index::{IndexTokenizer, RetrievalConfig};
use super::domain_router::DomainId;
use crate::ragplane::bm25::SNAPSHOT_FORMAT_VERSION;
use crate::ragplane::embedder::MODELLESS_MODEL_ID;
use crate::ragplane::query::LATENT_DIM;

/// The container layout version. Bump on ANY wire change to the on-disk
/// entry (header, payload fields, ordering).
pub const CACHE_FORMAT_VERSION: u32 = 1;

/// The ranking-semantics version. Bump when anything that maps inputs to
/// rankings changes in a way the content key cannot see — embedder math
/// (beyond its own model id), tokenizer behavior under the same
/// discriminant, retrieval scoring. A stale entry then refuses loud and
/// rebuilds.
pub const RANKING_SEMANTICS_VERSION: u32 = 1;

/// Env override for the cache dir (tests + A/B measurement).
pub const CACHE_DIR_ENV: &str = "RIIR_REFINE_INDEX_CACHE_DIR";

/// The exact-literal demote. `RIIR_REFINE_INDEX_CACHE=0` disarms the lane;
/// every other value (including unset) leaves the compile-time feature in
/// charge. DEMOTE-ONLY by the env-switch law — it can never arm the lane
/// in a build without the feature.
pub const DEMOTE_ENV: &str = "RIIR_REFINE_INDEX_CACHE";

/// LRU keep count: the newest [`KEEP_ENTRIES`] entries survive a prune.
/// At ~20 MiB per entry this alone caps the dir at a few hundred MiB;
/// the byte ceiling ([`MAX_MB_ENV`]) bounds it independently of count.
pub const KEEP_ENTRIES: usize = 8;

/// Issue 226 T3: the byte-ceiling env (`RIIR_REFINE_INDEX_CACHE_MAX_MB`) —
/// when the surviving entries exceed this many MiB, [`prune`] evicts
/// oldest-first until they fit. Unset = [`DEFAULT_MAX_CACHE_MB`]; `0` =
/// uncapped (the count cap alone); junk or MiB overflow = the default.
pub const MAX_MB_ENV: &str = "RIIR_REFINE_INDEX_CACHE_MAX_MB";

/// The default byte ceiling ([`MAX_MB_ENV`] unset) — 256 MiB, ~12 entries
/// at the measured 18.8–20 MiB.
pub const DEFAULT_MAX_CACHE_MB: u64 = 256;

/// Maximum domains a container may carry (the [`DomainId`] set's size).
const MAX_DOMAINS: u32 = 7;

/// Container magic (`b"RRIXIDC1"`).
const MAGIC: &[u8; 8] = b"RRIXIDC1";

/// Fixed header length: magic(8) + 3×u32 versions + config(9) + u32 count.
const HEADER_LEN: usize = 8 + 12 + 9 + 4;

/// blake3 digest trailer length.
const DIGEST_LEN: usize = 32;

/// Minimum entry size: header + digest (an empty-domain container).
const MIN_LEN: usize = HEADER_LEN + DIGEST_LEN;

/// Minimum byte footprint of one serialized row: the 32-byte zone hash +
/// the LATENT_DIM-wide direction. (The Bonsai dense arm adds fields on top;
/// the cap is a plausibility bound, every real read bounds-checks.)
const ROW_MIN_RECORD: usize = 32 + LATENT_DIM * 4;

/// One resolved domain corpus: `(rule_id, buggy, fixed)` triples.
pub type CorpusEntries = Vec<(String, String, String)>;

/// One indexed chunk, snapshot form (the retriever's private `IndexedChunk`
/// exposed for the cache wire only).
#[derive(Debug)]
pub struct IndexRow {
    /// BLAKE3 of the chunk content — the retriever's upsert/dedup key.
    pub zone_hash: [u8; 32],
    /// The 8-D modelless embedding of the chunk.
    pub dir: [f32; LATENT_DIM],
    /// The dense direction, when the dense path indexed one (feature-gated
    /// the same way the retriever's `IndexedChunk` is; the cache lane
    /// disarms under the dense posture, so this is always `None` in
    /// practice — carried for wire completeness).
    #[cfg(feature = "dense_embed")]
    pub dense: Option<Vec<f32>>,
}

/// One domain's fully-restorable index state.
#[derive(Debug)]
pub struct DomainSnapshot {
    pub domain: DomainId,
    /// Rows in build order — retrieval iterates this `Vec`, and the
    /// ranker's total order keeps tied scores in row order, so preserving
    /// it preserves bit-identical rankings.
    pub rows: Vec<IndexRow>,
    /// The sidecar lookup: `zone_hash` → `rule_id` (unique keys, build
    /// order).
    pub by_zone_hash: Vec<([u8; 32], String)>,
    /// Distinct rule ids (build order, deduped).
    pub rule_ids: Vec<String>,
    /// Distinct rules (NOT chunk count).
    pub n_rules: usize,
    /// The [`crate::ragplane::bm25::Bm25Index`] snapshot container
    /// (self-digested by the primitive — corrupt bytes refuse twice).
    pub bm25: Vec<u8>,
    /// The fitted mean-corpus anchor (feature-gated; the correction
    /// strength λ is env/rider state, re-derived at restore).
    #[cfg(feature = "differential_anchor")]
    pub anchor_mean: [f32; LATENT_DIM],
}

/// A domain captured at build time, finished into a [`DomainSnapshot`] on
/// the writer thread (issue 226). The capture is the cheap half (rows +
/// sidecars copied, the BM25 index shared by handle); [`DomainCapture::finish`]
/// is the expensive half — the canonical BM25 encode and the zone-map sort
/// — and runs off the critical path. The index is immutable after build,
/// so the handle reads exactly what was built.
pub struct DomainCapture {
    /// Every field final except `bm25` (empty until [`DomainCapture::finish`])
    /// and `by_zone_hash` (unsorted until then).
    snap: DomainSnapshot,
    bm25: std::sync::Arc<crate::ragplane::bm25::Bm25Index>,
}

impl DomainCapture {
    pub(crate) fn new(
        snap: DomainSnapshot,
        bm25: std::sync::Arc<crate::ragplane::bm25::Bm25Index>,
    ) -> Self {
        Self { snap, bm25 }
    }

    /// Serialize the shared BM25 index and canonicalize the zone map.
    ///
    /// # Errors
    ///
    /// The BM25 snapshot refusing (e.g. facet state it has no encoding for).
    pub fn finish(self) -> Result<DomainSnapshot, String> {
        let Self { mut snap, bm25 } = self;
        snap.bm25 = bm25.to_snapshot().map_err(|e| e.to_string())?;
        // The zone map comes from a HashMap — sort it so one content key
        // always encodes to the same bytes. Restore rebuilds the map, so
        // order never changed an answer; unsorted, every cold store was a
        // different file (measured: ~2 MB of the 18.8 MiB entry differed
        // run to run). Hash keys are unique, so the order is total.
        snap.by_zone_hash.sort_unstable_by_key(|(h, _)| *h);
        Ok(snap)
    }
}

/// Why a cache read produced no index. `Miss` is the quiet fast path; the
/// other two non-hit arms are loud (the caller prints the reason) and fall
/// through to a full rebuild + rewrite.
#[derive(Debug)]
pub enum LoadOutcome {
    /// Restored cleanly; carries the snapshots + the entry's byte size.
    Hit(Vec<DomainSnapshot>, u64),
    /// No entry under the key (the ordinary cold path).
    Miss,
    /// The entry exists but could not be trusted — corrupt, stale, or
    /// structurally invalid. LOUD: the message names the file + the reason.
    Corrupt(String),
    /// The entry exists but could not be READ (permissions, ...). LOUD —
    /// a silent rebuild-every-run would be a silent perf loss.
    ReadFailed(String),
}

/// The cache lane for one matcher build: armed with a resolved dir + key,
/// or off (byte-identical build, no I/O).
pub enum Lane {
    Armed(LaneArmed),
    Off,
}

pub struct LaneArmed {
    pub dir: PathBuf,
    pub key: [u8; 32],
    /// The key's hex (the on-disk name) — precomputed for the log lines.
    pub hex: String,
}

/// The compile-time posture checks that disarm the lane.
const NARROWED_POSTURE: bool = cfg!(feature = "kernel_route_narrow");

impl Lane {
    /// Resolve the lane for one build. Pure + infallible: every refusal is
    /// a documented posture, never an error path.
    #[must_use]
    pub fn resolve(config: &RetrievalConfig, domains: &[(DomainId, &CorpusEntries)]) -> Self {
        if NARROWED_POSTURE {
            return Lane::Off;
        }
        #[cfg(feature = "bonsai_retrieval")]
        if config.dense_weight > 0.0 {
            return Lane::Off;
        }
        if !armed() {
            return Lane::Off;
        }
        let key = cache_key(config, domains);
        Lane::Armed(LaneArmed {
            dir: cache_dir(),
            hex: hex(&key),
            key,
        })
    }
}

/// The runtime demote check (the exact literal `0`).
#[must_use]
pub fn armed() -> bool {
    std::env::var(DEMOTE_ENV).as_deref() != Ok("0")
}

/// Issue 226 T3: the effective byte ceiling in bytes (`None` = uncapped),
/// resolved from [`MAX_MB_ENV`]. Forgiving by design — this is a tunable,
/// not an arm switch: junk parses to the default, never to a surprise.
fn max_cache_bytes() -> Option<u64> {
    let parsed = match std::env::var(MAX_MB_ENV) {
        Err(_) => return Some(DEFAULT_MAX_CACHE_MB * 1024 * 1024),
        Ok(v) => v.trim().parse::<u64>(),
    };
    match parsed {
        Ok(0) => None,
        Ok(mb) => mb
            .checked_mul(1024 * 1024)
            .or(Some(DEFAULT_MAX_CACHE_MB * 1024 * 1024)),
        Err(_) => Some(DEFAULT_MAX_CACHE_MB * 1024 * 1024),
    }
}

/// The cache base dir: `RIIR_REFINE_INDEX_CACHE_DIR` override, else the
/// platform cache chain (`$XDG_CACHE_HOME` / `~/.cache` /
/// `%LOCALAPPDATA%` / temp fallback) + `riir-refine` — the same resolution
/// the self-evolve memory file uses.
#[must_use]
fn base_dir() -> PathBuf {
    if let Some(p) = std::env::var_os(CACHE_DIR_ENV) {
        return PathBuf::from(p);
    }
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("riir-refine")
}

/// The index-cache dir (`<base>/index`).
#[must_use]
pub fn cache_dir() -> PathBuf {
    base_dir().join("index")
}

/// T3: the per-user auto-corpus attempt stamp (`<base>/corpus_auto.json`)
/// — the 12 h window becomes machine-wide, so a new crate on a machine at
/// the current epoch skips the fetch. (`None` never happens today — the
/// chain ends at the temp dir — but the `Option` shape keeps callers
/// honest about a no-dir world.)
#[must_use]
pub fn auto_corpus_stamp_path() -> Option<PathBuf> {
    Some(base_dir().join("corpus_auto.json"))
}

pub(crate) fn domain_tag(domain: DomainId) -> u8 {
    match domain {
        DomainId::Clippy => 0,
        DomainId::Perf => 1,
        DomainId::Kernel => 2,
        DomainId::Sec => 3,
        DomainId::Docker => 4,
        DomainId::Dist => 5,
        DomainId::ShaderPerf => 6,
        DomainId::Ane => 7,
    }
}

fn domain_from_tag(tag: u8) -> Option<DomainId> {
    match tag {
        0 => Some(DomainId::Clippy),
        1 => Some(DomainId::Perf),
        2 => Some(DomainId::Kernel),
        3 => Some(DomainId::Sec),
        4 => Some(DomainId::Docker),
        5 => Some(DomainId::Dist),
        6 => Some(DomainId::ShaderPerf),
        7 => Some(DomainId::Ane),
        _ => None,
    }
}

/// The 9-byte config fingerprint carried in the header (and hashed into
/// the key): tokenizer discriminant + both weight bit patterns.
fn config_bytes(config: &RetrievalConfig) -> [u8; 9] {
    let mut out = [0u8; 9];
    out[0] = match config.tokenizer {
        IndexTokenizer::CodeDefault => 0,
        IndexTokenizer::OpAware => 1,
        IndexTokenizer::OpAwareBigram => 2,
    };
    out[1..5].copy_from_slice(&config.latent_weight.to_bits().to_le_bytes());
    out[5..9].copy_from_slice(&config.dense_weight.to_bits().to_le_bytes());
    out
}

/// The content-addressed cache key: blake3 over everything that determines
/// the built index's rankings. Deterministic — identical inputs, identical
/// key.
#[must_use]
pub fn cache_key(config: &RetrievalConfig, domains: &[(DomainId, &CorpusEntries)]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(b"riir-refine/matcher-index-cache/v1");
    h.update(&CACHE_FORMAT_VERSION.to_le_bytes());
    h.update(&RANKING_SEMANTICS_VERSION.to_le_bytes());
    h.update(&SNAPSHOT_FORMAT_VERSION.to_le_bytes());
    h.update(MODELLESS_MODEL_ID.as_bytes());
    h.update(&config_bytes(config));
    for (domain, entries) in domains {
        // One deref: the slice carries shared refs, so iterating the pair
        // binds `&&CorpusEntries` — bind it as `&CorpusEntries` for the
        // per-entry loop (and satisfy the explicit-iter lint).
        let entries: &CorpusEntries = entries;
        h.update(&[domain_tag(*domain)]);
        h.update(&(entries.len() as u64).to_le_bytes());
        for (rule_id, buggy, fixed) in entries {
            for s in [rule_id, buggy, fixed] {
                h.update(&(s.len() as u64).to_le_bytes());
                h.update(s.as_bytes());
            }
        }
    }
    *h.finalize().as_bytes()
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(64);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Bytes → MiB for the disclosure lines (display-only; the cast never
/// feeds an index or a decision).
#[must_use]
pub(crate) fn mib(bytes: u64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    {
        bytes as f64 / (1024.0 * 1024.0)
    }
}

/// A `usize` length on the u32 wire. Vec lengths are ≤ `isize::MAX`; an
/// actual overflow of the u32 prefix would need a >4 GiB single section —
/// refused LOUD here rather than silently truncated (the wire keeps no
/// other length bound).
fn wire_len(n: usize) -> u32 {
    u32::try_from(n).expect("index-cache: section length exceeds the u32 wire prefix")
}

// ---------------------------------------------------------------------------
// Wire encoding
// ---------------------------------------------------------------------------

/// Single-buffer encoder (issue 226 T2). Every section is written straight
/// into ONE pre-sized buffer — a section's length prefix is reserved and
/// patched in place once its payload lands — and the digest is ONE
/// `blake3` pass over the finished buffer. (The 224 shape built each
/// section in a temp buffer, hashed it there, then memcpy'd + re-hashed it
/// into the outer one: two copies and two hashes of the ~20 MiB entry.)
/// The wire bytes are unchanged.
struct Enc {
    buf: Vec<u8>,
}

impl Enc {
    fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }
    fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }
    /// Reserve a u32 length prefix; returns its offset for [`Enc::patch_len`].
    fn reserve_len(&mut self) -> usize {
        let at = self.buf.len();
        self.buf.extend_from_slice(&[0; 4]);
        at
    }
    /// Patch the prefix reserved at `at` with the byte count written since.
    fn patch_len(&mut self, at: usize) {
        let len = wire_len(self.buf.len() - at - 4);
        self.buf[at..at + 4].copy_from_slice(&len.to_le_bytes());
    }
    fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }
    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(wire_len(s.len()));
        self.bytes(s.as_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }
    fn f32s(&mut self, v: &[f32]) {
        for x in v {
            self.f32(*x);
        }
    }
}

fn encode_row(enc: &mut Enc, row: &IndexRow) {
    enc.bytes(&row.zone_hash);
    enc.f32s(&row.dir);
    #[cfg(feature = "dense_embed")]
    match &row.dense {
        Some(v) => {
            enc.u8(1);
            // The dense vector is LATENT_DIM-bounded by construction —
            // the cast cannot truncate on any 64-bit target.
            #[allow(clippy::cast_possible_truncation)]
            enc.u32(v.len() as u32);
            enc.f32s(v);
        }
        None => enc.u8(0),
    }
}

fn encode_domain(enc: &mut Enc, snap: &DomainSnapshot) {
    enc.u8(domain_tag(snap.domain));
    // The self-describing frame's length prefix is patched in place after
    // the payload lands (issue 226 T2) — no temp buffer, no second copy.
    let len_at = enc.reserve_len();
    enc.u32(wire_len(snap.n_rules));
    enc.u32(wire_len(snap.rows.len()));
    for row in &snap.rows {
        encode_row(enc, row);
    }
    enc.u32(wire_len(snap.by_zone_hash.len()));
    for (hash, rule_id) in &snap.by_zone_hash {
        enc.bytes(hash);
        enc.str(rule_id);
    }
    enc.u32(wire_len(snap.rule_ids.len()));
    for id in &snap.rule_ids {
        enc.str(id);
    }
    enc.u32(wire_len(snap.bm25.len()));
    enc.bytes(&snap.bm25);
    #[cfg(feature = "differential_anchor")]
    enc.f32s(&snap.anchor_mean);
    enc.patch_len(len_at);
}

/// An upper-ish estimate of the encoded size, so the one buffer is sized
/// once (a doubling `Vec` would re-copy the ~20 MiB entry ~log2 times).
/// Only a capacity hint — an under-estimate just grows the buffer.
fn estimated_len(snapshots: &[DomainSnapshot]) -> usize {
    let per_domain: usize = snapshots
        .iter()
        .map(|s| {
            let strs: usize = s
                .by_zone_hash
                .iter()
                .map(|(_, id)| 32 + 4 + id.len())
                .chain(s.rule_ids.iter().map(|id| 4 + id.len()))
                .sum();
            1 + 4 * 6 + s.rows.len() * (ROW_MIN_RECORD + 1) + strs + s.bm25.len() + LATENT_DIM * 4
        })
        .sum();
    HEADER_LEN + per_domain + DIGEST_LEN
}

/// Encode a full container (header + domains + digest trailer).
fn encode(config: &RetrievalConfig, snapshots: &[DomainSnapshot]) -> Vec<u8> {
    let mut enc = Enc::with_capacity(estimated_len(snapshots));
    enc.bytes(MAGIC);
    enc.u32(CACHE_FORMAT_VERSION);
    enc.u32(RANKING_SEMANTICS_VERSION);
    enc.u32(SNAPSHOT_FORMAT_VERSION);
    enc.bytes(&config_bytes(config));
    enc.u32(wire_len(snapshots.len()));
    for s in snapshots {
        encode_domain(&mut enc, s);
    }
    let digest = blake3::hash(&enc.buf);
    enc.buf.extend_from_slice(digest.as_bytes());
    enc.buf
}

// ---------------------------------------------------------------------------
// Wire decoding
// ---------------------------------------------------------------------------

/// Bounds-checked cursor over a byte slice. Every read refuses a overrun as
/// a typed message; counts are plausibility-capped against the remaining
/// bytes BEFORE any allocation.
struct Dec<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Dec<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, p: 0 }
    }
    fn remaining(&self) -> usize {
        self.b.len() - self.p
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.remaining() < n {
            return Err("truncated payload".to_string());
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn read_u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes(b.try_into().expect("4 bytes")))
    }
    fn read_f32(&mut self) -> Result<f32, String> {
        Ok(f32::from_bits(self.read_u32()?))
    }
    /// A count whose record footprint must fit in the remaining bytes.
    fn capped(&mut self, min_record: usize, what: &str) -> Result<usize, String> {
        let n = self.read_u32()? as usize;
        if min_record == 0 || n > self.remaining() / min_record {
            return Err(format!("implausible {what} count {n}"));
        }
        Ok(n)
    }
    fn read_str(&mut self) -> Result<String, String> {
        let n = self.read_u32()? as usize;
        if n > self.remaining() {
            return Err("truncated string".to_string());
        }
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| "invalid utf8 in snapshot".to_string())
    }
    fn array32(&mut self) -> Result<[u8; 32], String> {
        let b = self.take(32)?;
        Ok(b.try_into().expect("32 bytes"))
    }
}

fn parse_row(dec: &mut Dec) -> Result<IndexRow, String> {
    let zone_hash = dec.array32()?;
    let mut dir = [0.0_f32; LATENT_DIM];
    for slot in &mut dir {
        *slot = dec.read_f32()?;
    }
    // The dense direction, when the dense path indexed one (feature-gated
    // the same way the retriever's `IndexedChunk` is; the cache lane
    // disarms under the dense posture, so this is always `None` in
    // practice — carried for wire completeness).
    #[cfg(feature = "dense_embed")]
    let dense = {
        let has = dec.read_u8()?;
        if has == 0 {
            None
        } else {
            let n = dec.capped(4, "dense vector")?;
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                v.push(dec.read_f32()?);
            }
            Some(v)
        }
    };
    Ok(IndexRow {
        zone_hash,
        dir,
        #[cfg(feature = "dense_embed")]
        dense,
    })
}

fn parse_domain(dec: &mut Dec) -> Result<DomainSnapshot, String> {
    let domain = domain_from_tag(dec.read_u8()?).ok_or("unknown domain tag")?;
    let payload_len = dec.read_u32()? as usize;
    if payload_len > dec.remaining() {
        return Err("truncated domain payload".to_string());
    }
    let body = dec.take(payload_len)?;
    let mut d = Dec::new(body);
    let n_rules = d.read_u32()? as usize;
    let rows_len = d.capped(ROW_MIN_RECORD, "rows")?;
    let mut rows = Vec::with_capacity(rows_len);
    for _ in 0..rows_len {
        rows.push(parse_row(&mut d)?);
    }
    let bzh_len = d.capped(36, "zone-map")?;
    let mut by_zone_hash = Vec::with_capacity(bzh_len);
    for _ in 0..bzh_len {
        let hash = d.array32()?;
        let rule_id = d.read_str()?;
        by_zone_hash.push((hash, rule_id));
    }
    let ids_len = d.capped(4, "rule ids")?;
    let mut rule_ids = Vec::with_capacity(ids_len);
    for _ in 0..ids_len {
        rule_ids.push(d.read_str()?);
    }
    let bm_len = d.read_u32()? as usize;
    if bm_len > d.remaining() {
        return Err("truncated bm25 snapshot".to_string());
    }
    let bm25 = d.take(bm_len)?.to_vec();
    #[cfg(feature = "differential_anchor")]
    let anchor_mean = {
        let mut a = [0.0_f32; LATENT_DIM];
        for slot in &mut a {
            *slot = d.read_f32()?;
        }
        a
    };
    if d.remaining() != 0 {
        return Err("trailing bytes in domain payload".to_string());
    }
    Ok(DomainSnapshot {
        domain,
        rows,
        by_zone_hash,
        rule_ids,
        n_rules,
        bm25,
        #[cfg(feature = "differential_anchor")]
        anchor_mean,
    })
}

// ---------------------------------------------------------------------------
// Store / load / prune
// ---------------------------------------------------------------------------

/// Write the entry atomically (tmp + rename). The caller owns the
/// all-or-nothing rule: a partial [`DomainSnapshot`] list never reaches
/// here.
///
/// # Errors
///
/// Directory creation, the tmp write, or the rename failing — surfaced to
/// the caller, which prints one loud line and continues uncached (the
/// build path is never gated on the cache store).
pub fn store(
    dir: &Path,
    key: &[u8; 32],
    config: &RetrievalConfig,
    snapshots: &[DomainSnapshot],
) -> std::io::Result<u64> {
    std::fs::create_dir_all(dir)?;
    let buf = encode(config, snapshots);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tmp = dir.join(format!(".tmp-{}-{nanos}", std::process::id()));
    std::fs::write(&tmp, &buf)?;
    std::fs::rename(&tmp, dir.join(format!("{}.idx", hex(key))))?;
    Ok(buf.len() as u64)
}

/// Read + validate the entry under `key`. Digest first, then structure —
/// corrupt bytes never become [`DomainSnapshot`]s.
#[must_use]
pub fn load(dir: &Path, key: &[u8; 32], config: &RetrievalConfig) -> LoadOutcome {
    let path = dir.join(format!("{}.idx", hex(key)));
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LoadOutcome::Miss,
        Err(e) => {
            return LoadOutcome::ReadFailed(format!(
                "[index-cache] read failed: {} ({e}) — rebuilding",
                path.display()
            ));
        }
    };
    let corrupt = |m: String| {
        LoadOutcome::Corrupt(format!(
            "[index-cache] corrupt entry {}: {m} — rebuilding",
            path.display()
        ))
    };
    if bytes.len() < MIN_LEN {
        return corrupt(format!("too small ({} bytes)", bytes.len()));
    }
    let mut d = Dec::new(&bytes);
    let ok = (|| -> Result<(), String> {
        if d.take(8)? != MAGIC {
            return Err("bad magic (not a matcher index entry)".to_string());
        }
        let ver = d.read_u32()?;
        if ver != CACHE_FORMAT_VERSION {
            return Err(format!("stale container v{ver} ≠ v{CACHE_FORMAT_VERSION}"));
        }
        let rank_ver = d.read_u32()?;
        if rank_ver != RANKING_SEMANTICS_VERSION {
            return Err(format!(
                "stale ranking semantics v{rank_ver} ≠ v{RANKING_SEMANTICS_VERSION}"
            ));
        }
        let bm_ver = d.read_u32()?;
        if bm_ver != SNAPSHOT_FORMAT_VERSION {
            return Err(format!(
                "stale bm25 snapshot format v{bm_ver} ≠ v{SNAPSHOT_FORMAT_VERSION}"
            ));
        }
        let cfg = d.take(9)?;
        if cfg != config_bytes(config) {
            return Err("config drift".to_string());
        }
        Ok(())
    })();
    if let Err(m) = ok {
        return corrupt(m);
    }
    // Digest over everything before the trailer — BEFORE any parse.
    let payload_end = bytes.len() - DIGEST_LEN;
    if blake3::hash(&bytes[..payload_end]).as_bytes() != &bytes[payload_end..] {
        return corrupt("digest mismatch — corrupt or truncated".to_string());
    }
    let domain_count = match d.read_u32() {
        Ok(n) if n <= MAX_DOMAINS => n,
        Ok(n) => return corrupt(format!("implausible domain count {n}")),
        Err(m) => return corrupt(m),
    };
    let mut snaps = Vec::with_capacity(domain_count as usize);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..domain_count {
        match parse_domain(&mut d) {
            Ok(s) => {
                if !seen.insert(s.domain) {
                    return corrupt(format!("duplicate domain {:?}", s.domain));
                }
                snaps.push(s);
            }
            Err(m) => return corrupt(m),
        }
    }
    // The cursor stops at the digest trailer — exactly DIGEST_LEN bytes
    // must remain. More (junk between payload and trailer) or fewer
    // (payload overran the trailer) is corrupt.
    if d.remaining() != DIGEST_LEN {
        return corrupt("trailing bytes".to_string());
    }
    LoadOutcome::Hit(snaps, bytes.len() as u64)
}

/// A `.tmp-*` file older than this is a dead writer's litter (a live store
/// of the ~20 MiB entry takes well under a second).
const STALE_TMP_AGE: std::time::Duration = std::time::Duration::from_secs(3600);

// ---------------------------------------------------------------------------
// Issue 226 T1 — store after print
// ---------------------------------------------------------------------------

/// One deferred store: a running writer thread, or the loud line of a
/// writer that could not be spawned (the entry is skipped; the next miss
/// rewrites it).
enum PendingStore {
    Thread(std::thread::JoinHandle<String>),
    Failed(String),
}

/// The process's deferred stores. Not a hot path (≤ 1 push per matcher
/// build, 1 drain per process) — a plain `Mutex` is the right tool.
static PENDING: std::sync::Mutex<Vec<PendingStore>> = std::sync::Mutex::new(Vec::new());

/// Store + prune, returning the one disclosure line (G4's size report).
fn store_and_prune(
    dir: &Path,
    key: &[u8; 32],
    hex: &str,
    config: &RetrievalConfig,
    snapshots: &[DomainSnapshot],
) -> String {
    let t_store = std::time::Instant::now();
    let line = match store(dir, key, config, snapshots) {
        Ok(len) => format!(
            "[index-cache] stored {hex} ({:.1} MiB, {} domains, write {:.0}ms, after results)",
            mib(len),
            snapshots.len(),
            t_store.elapsed().as_secs_f64() * 1e3
        ),
        Err(e) => format!("[index-cache] store failed ({e}) — continuing uncached"),
    };
    prune(dir);
    line
}

/// Issue 226 T1: hand the store OFF the critical path. Encoding, the
/// digest, the atomic write and the prune run on a writer thread while the
/// run scans and prints; [`finish_pending_stores`] joins it at process end,
/// so the results never wait on the ~20 MiB write. Atomicity is unchanged
/// (tmp then rename inside [`store`]); a run that dies before the join
/// loses only the entry (the next miss rewrites it) and leaves tmp litter
/// that [`prune`] sweeps.
pub fn store_deferred(armed: &LaneArmed, config: RetrievalConfig, captures: Vec<DomainCapture>) {
    let (dir, key, hex) = (armed.dir.clone(), armed.key, armed.hex.clone());
    let pending = match std::thread::Builder::new()
        .name("index-cache-store".into())
        .spawn(move || {
            // All-or-nothing: one domain refusing skips the whole entry.
            let snapshots: Result<Vec<DomainSnapshot>, String> =
                captures.into_iter().map(DomainCapture::finish).collect();
            match snapshots {
                Ok(s) => store_and_prune(&dir, &key, &hex, &config, &s),
                Err(e) => format!("[index-cache] snapshot failed ({e}) — cache store skipped"),
            }
        }) {
        Ok(h) => PendingStore::Thread(h),
        Err(e) => PendingStore::Failed(format!(
            "[index-cache] store skipped (writer thread: {e}) — continuing uncached"
        )),
    };
    PENDING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(pending);
}

/// Join every deferred store and print its disclosure line to stderr.
/// Call once at the end of a run, AFTER the results and the roll-up.
/// Returns the number of stores finished. Idempotent (a second call finds
/// nothing pending).
pub fn finish_pending_stores() -> usize {
    let drained: Vec<PendingStore> = std::mem::take(
        &mut *PENDING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    let n = drained.len();
    for p in drained {
        let line = match p {
            PendingStore::Thread(h) => h.join().unwrap_or_else(|_| {
                "[index-cache] store thread panicked — continuing uncached".to_string()
            }),
            PendingStore::Failed(line) => line,
        };
        eprintln!("{line}");
    }
    n
}

/// Touch an entry's mtime (the LRU is read-order, best-effort).
pub fn touch(path: &Path) {
    let _ = std::fs::File::options()
        .write(true)
        .open(path)
        .and_then(|f| f.set_modified(std::time::SystemTime::now()));
}

/// Keep the newest [`KEEP_ENTRIES`] cache entries (by mtime), delete the
/// rest. Only files named `<64 hex>.idx` are considered — nothing else in
/// the dir (including operator files) is ever touched. Best-effort.
pub fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let now = std::time::SystemTime::now();
    let mut idx: Vec<(std::time::SystemTime, PathBuf, u64)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?;
            // Issue 226 T1: a deferred store killed mid-write (a run that
            // exits through `process::exit` before
            // [`finish_pending_stores`]) leaves its `.tmp-<pid>-<nanos>`
            // behind — never a corrupt ENTRY (the rename is the commit),
            // only litter. Swept once it is older than any live write.
            if name.starts_with(".tmp-") {
                let stale = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| now.duration_since(t).ok())
                    .is_some_and(|age| age >= STALE_TMP_AGE);
                if stale {
                    let _ = std::fs::remove_file(&path);
                }
                return None;
            }
            if name.len() != 68 {
                return None;
            }
            // Case-sensitive BY DESIGN: cache names are lowercase hex —
            // a `.IDX` look-alike must never be treated as an entry.
            #[allow(clippy::case_sensitive_file_extension_comparisons)]
            if !name.ends_with(".idx") {
                return None;
            }
            if !name.as_bytes()[..64].iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            let meta = e.metadata().ok()?;
            let mtime = meta.modified().ok()?;
            Some((mtime, path, meta.len()))
        })
        .collect();
    // The count cap first: the newest [`KEEP_ENTRIES`] survive.
    if idx.len() > KEEP_ENTRIES {
        idx.sort_by_key(|&(t, _, _)| std::cmp::Reverse(t));
        let excess = idx.split_off(KEEP_ENTRIES);
        for (_, path, _) in &excess {
            let _ = std::fs::remove_file(path);
        }
    }
    // Issue 226 T3: the byte cap, independent of count.
    let Some(cap) = max_cache_bytes() else {
        return;
    };
    for path in byte_cap_evictions(&idx, cap) {
        let _ = std::fs::remove_file(path);
    }
}

/// Issue 226 T3(a): the paths to evict when the surviving entries exceed
/// `cap` bytes — oldest mtime first until they fit. LRU BY DECISION
/// (issue 226 T3(b), recorded at implementation): mtime is last USE
/// ([`touch`] bumps it on every hit), so the stale tail dies first and an
/// entry pinned by constant use carries a fresh mtime and survives.
/// LFU was REFUSED: it pins a hot-but-stale corpus state past its content
/// epoch (the content key moved on; the old entry is worthless however
/// often it was hit early) and needs a use-counter sidecar the wire does
/// not carry. A fresh corpus state can never be starved: it arrives with
/// the newest mtime, so under pressure it is the LAST candidate — and the
/// newest entry is never evicted at all. A lone over-cap entry stays
/// (raising [`MAX_MB_ENV`] is the operator's lever); evicting it would
/// thrash (re-store → prune → re-store every run).
fn byte_cap_evictions(idx: &[(std::time::SystemTime, PathBuf, u64)], cap: u64) -> Vec<PathBuf> {
    let mut total: u64 = idx.iter().map(|(_, _, len)| len).sum();
    if idx.len() < 2 || total <= cap {
        return Vec::new();
    }
    let mut order: Vec<usize> = (0..idx.len()).collect();
    order.sort_by_key(|&i| idx[i].0);
    let mut out = Vec::with_capacity(order.len());
    for &i in &order {
        if idx.len() - out.len() <= 1 {
            break; // never the last survivor (the newest, oldest-first walk)
        }
        out.push(idx[i].1.clone());
        total = total.saturating_sub(idx[i].2);
        if total <= cap {
            break;
        }
    }
    out.shrink_to_fit();
    out
}

#[cfg(all(test, feature = "index_cache"))]
mod tests {
    use super::*;
    use crate::draft::latent_matcher::{DomainOverlay, LatentMatcher};

    /// The two env-mutating tests (demote + G1) share this lock — the env
    /// is process-global and a concurrent `=0` demote would disarm the
    /// G1 lane mid-build (a false failure, not a false pass).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_dir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("riir-refine-i224-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp cache dir");
        d
    }

    fn sample_snapshot(domain: DomainId) -> DomainSnapshot {
        sample_capture(domain).finish().expect("finish capture")
    }

    /// The build-time capture shape: BM25 shared by handle, the zone map
    /// deliberately UNSORTED (finish canonicalizes it).
    fn sample_capture(domain: DomainId) -> DomainCapture {
        let bm25 = crate::ragplane::bm25::Bm25Index::new();
        bm25.index(
            b"doc-a",
            "fn process_payment(amount: u32) { let x = amount + 1; }",
        );
        bm25.index(b"doc-b", "match value { Some(v) => v, None => 0 }");
        bm25.index(b"doc-c", "");
        let snap = DomainSnapshot {
            domain,
            rows: vec![
                IndexRow {
                    zone_hash: [1u8; 32],
                    dir: [0.5, -1.25, 0.0, 3.5, 1.0, -2.0, 0.125, 8.0],
                    #[cfg(feature = "bonsai_retrieval")]
                    dense: None,
                },
                IndexRow {
                    zone_hash: [2u8; 32],
                    dir: [-0.5, 1.0, 2.0, -3.0, 4.0, 5.0, -6.0, 0.25],
                    #[cfg(feature = "bonsai_retrieval")]
                    dense: None,
                },
            ],
            by_zone_hash: vec![
                ([2u8; 32], "rule_two".to_string()),
                ([1u8; 32], "rule_one".to_string()),
            ],
            rule_ids: vec!["rule_one".to_string(), "rule_two".to_string()],
            n_rules: 2,
            bm25: Vec::new(),
            #[cfg(feature = "differential_anchor")]
            anchor_mean: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        };
        DomainCapture::new(snap, std::sync::Arc::new(bm25))
    }

    fn sample_snapshots() -> Vec<DomainSnapshot> {
        vec![
            sample_snapshot(DomainId::Clippy),
            sample_snapshot(DomainId::Perf),
        ]
    }

    fn sample_entries() -> Vec<(DomainId, CorpusEntries)> {
        vec![
            (
                DomainId::Clippy,
                vec![
                    (
                        "i224_a".to_string(),
                        "fn f() -> u8 { return 7; }".to_string(),
                        "fn f() -> u8 { 7 }".to_string(),
                    ),
                    (
                        "i224_b".to_string(),
                        "if x == true {}".to_string(),
                        "if x {}".to_string(),
                    ),
                ],
            ),
            (
                DomainId::Perf,
                vec![(
                    "i224_p1".to_string(),
                    "v.iter().map(|x| x.clone()).collect::<Vec<_>>()".to_string(),
                    "v.clone()".to_string(),
                )],
            ),
        ]
    }

    fn row_bit_key(rows: &[IndexRow]) -> Vec<([u8; 32], Vec<u32>)> {
        rows.iter()
            .map(|r| {
                (
                    r.zone_hash,
                    r.dir.iter().map(|f| f.to_bits()).collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    #[test]
    fn round_trip_preserves_every_field() {
        let dir = temp_dir("roundtrip");
        let config = RetrievalConfig::default();
        let entries = sample_entries();
        let refs: Vec<(DomainId, &CorpusEntries)> = entries.iter().map(|(d, e)| (*d, e)).collect();
        let key = cache_key(&config, &refs);
        let snapshots = sample_snapshots();
        store(&dir, &key, &config, &snapshots).expect("store");
        match load(&dir, &key, &config) {
            LoadOutcome::Hit(mut got, size) => {
                assert!(size >= MIN_LEN as u64);
                assert_eq!(got.len(), snapshots.len());
                got.sort_by_key(|s| domain_tag(s.domain));
                for (want, have) in snapshots.iter().zip(got.iter()) {
                    assert_eq!(want.domain, have.domain);
                    assert_eq!(want.n_rules, have.n_rules);
                    assert_eq!(want.rule_ids, have.rule_ids);
                    assert_eq!(want.by_zone_hash, have.by_zone_hash);
                    assert_eq!(row_bit_key(&want.rows), row_bit_key(&have.rows));
                    assert_eq!(want.bm25, have.bm25);
                    #[cfg(feature = "differential_anchor")]
                    assert_eq!(want.anchor_mean, have.anchor_mean);
                }
            }
            other => panic!("expected Hit, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_entry_refuses_and_never_hits() {
        let dir = temp_dir("corrupt");
        let config = RetrievalConfig::default();
        let snapshots = sample_snapshots();
        let key = [7u8; 32];
        let path = dir.join(format!("{}.idx", hex(&key)));
        store(&dir, &key, &config, &snapshots).expect("store");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Hit(_, _)));

        let original = std::fs::read(&path).expect("read entry");
        // (a) a flipped payload byte — the digest must catch it.
        let mut flipped = original.clone();
        let mid = original.len() / 2;
        flipped[mid] ^= 0xff;
        std::fs::write(&path, &flipped).expect("write");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Corrupt(_)));

        // (b) truncated mid-payload.
        std::fs::write(&path, &original[..original.len() - 10]).expect("write");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Corrupt(_)));

        // (c) smaller than a header + digest.
        std::fs::write(&path, &original[..MIN_LEN - 1]).expect("write");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Corrupt(_)));

        // (d) bad magic — refuses before the digest is even consulted.
        let mut magic = original.clone();
        magic[0] = b'X';
        std::fs::write(&path, &magic).expect("write");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Corrupt(_)));

        // (e) stale container version — the post-upgrade rebuild path.
        let mut ver = original.clone();
        ver[8..12].copy_from_slice(&99u32.to_le_bytes());
        std::fs::write(&path, &ver).expect("write");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Corrupt(_)));

        // (f) a config-drift entry (key collision defense in depth).
        let mut drift = original.clone();
        drift[20] = (drift[20] + 1) % 3;
        std::fs::write(&path, &drift).expect("write");
        assert!(matches!(load(&dir, &key, &config), LoadOutcome::Corrupt(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_changes_when_inputs_change() {
        let config = RetrievalConfig::default();
        let entries = sample_entries();
        let refs: Vec<(DomainId, &CorpusEntries)> = entries.iter().map(|(d, e)| (*d, e)).collect();
        let base = cache_key(&config, &refs);
        assert_eq!(base, cache_key(&config, &refs), "key must be deterministic");

        // One entry byte changes the key.
        let mut mutated = entries.clone();
        mutated[0].1[0].1.push(' ');
        let mrefs: Vec<(DomainId, &CorpusEntries)> = mutated.iter().map(|(d, e)| (*d, e)).collect();
        assert_ne!(base, cache_key(&config, &mrefs));

        // The tokenizer discriminant changes the key.
        let tok_config = RetrievalConfig {
            tokenizer: IndexTokenizer::OpAware,
            ..config
        };
        assert_ne!(base, cache_key(&tok_config, &refs));

        // A weight bit changes the key.
        let w_config = RetrievalConfig {
            latent_weight: 0.5,
            ..config
        };
        assert_ne!(base, cache_key(&w_config, &refs));
    }

    #[test]
    fn prune_keeps_newest_and_strict_names() {
        let dir = temp_dir("prune");
        // Ten entries with FORCED distinct mtimes (i = age in seconds); the
        // two oldest must go, the fake.txt bystander must survive.
        for i in 0..10u32 {
            let name = format!("{i:064x}.idx");
            let path = dir.join(&name);
            std::fs::write(&path, b"x").expect("write");
            let f = std::fs::File::options()
                .write(true)
                .open(&path)
                .expect("open");
            f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(i.into()))
                .expect("mtime");
        }
        std::fs::write(dir.join("fake.txt"), b"keep me").expect("write");
        std::fs::write(dir.join(".tmp-999-1"), b"transient").expect("write");
        prune(&dir);
        let mut remaining: Vec<String> = std::fs::read_dir(&dir)
            .expect("readdir")
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        remaining.sort();
        assert_eq!(
            remaining.len(),
            KEEP_ENTRIES + 2,
            "8 newest + fake.txt + tmp"
        );
        assert!(remaining.contains(&"fake.txt".to_string()));
        for i in 0..KEEP_ENTRIES {
            let newest = 9 - i;
            let kept = format!("{newest:064x}.idx");
            assert!(remaining.contains(&kept), "newest {kept} must survive");
        }
        for i in 0..2u32 {
            let gone = format!("{i:064x}.idx");
            assert!(!remaining.contains(&gone), "oldest {gone} must be pruned");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn demote_env_literal_is_exact_zero() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var(DEMOTE_ENV);
        assert!(
            armed(),
            "unset must arm (the compile-time feature owns arming)"
        );
        std::env::set_var(DEMOTE_ENV, "0");
        assert!(!armed(), "the exact literal 0 is THE demote");
        for not_demote in ["00", "false", "1", "on", " "] {
            std::env::set_var(DEMOTE_ENV, not_demote);
            assert!(armed(), "{not_demote:?} must never demote");
        }
        std::env::remove_var(DEMOTE_ENV);
        assert!(armed());
    }

    // ---- Issue 226 T3: the byte ceiling -------------------------------

    #[test]
    fn max_cache_mb_env_semantics() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var(MAX_MB_ENV);
        assert_eq!(
            max_cache_bytes(),
            Some(DEFAULT_MAX_CACHE_MB * 1024 * 1024),
            "unset = the default ceiling"
        );
        std::env::set_var(MAX_MB_ENV, "0");
        assert_eq!(max_cache_bytes(), None, "the exact 0 = uncapped");
        std::env::set_var(MAX_MB_ENV, "16");
        assert_eq!(max_cache_bytes(), Some(16 * 1024 * 1024));
        std::env::set_var(MAX_MB_ENV, " 16 ");
        assert_eq!(
            max_cache_bytes(),
            Some(16 * 1024 * 1024),
            "trim is friendly"
        );
        for junk in ["abc", "-4", "1.5", ""] {
            std::env::set_var(MAX_MB_ENV, junk);
            assert_eq!(
                max_cache_bytes(),
                Some(DEFAULT_MAX_CACHE_MB * 1024 * 1024),
                "junk {junk:?} = the default, never a silent 0 or a panic"
            );
        }
        std::env::set_var(MAX_MB_ENV, "18446744073709551615");
        assert_eq!(
            max_cache_bytes(),
            Some(DEFAULT_MAX_CACHE_MB * 1024 * 1024),
            "MiB overflow = the default"
        );
        std::env::remove_var(MAX_MB_ENV);
    }

    /// T3(a): oldest-first eviction order, fit stops the walk, mtime order
    /// (not vec order) decides, and the newest entry is never a candidate.
    #[test]
    fn byte_cap_evicts_oldest_first_never_the_newest() {
        let mk = |i: usize, age: u64, len: u64| {
            (
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(age),
                PathBuf::from(format!("e{i}.idx")),
                len,
            )
        };
        // Four 100-unit entries, cap 250: e0 (→300), e1 (→200 ≤ 250).
        let idx: Vec<_> = (0..4usize).map(|i| mk(i, i as u64, 100)).collect();
        assert_eq!(
            byte_cap_evictions(&idx, 250),
            [PathBuf::from("e0.idx"), PathBuf::from("e1.idx")]
        );
        // Fits: nothing goes. (assert_eq! not assert!(is_empty) — the
        // clippy::assert_is_empty law; Vec==Vec, never the broken
        // Vec-vs-array [] spelling the rulewright once emitted, issue 223.)
        assert_eq!(byte_cap_evictions(&idx, 400), Vec::<PathBuf>::new());
        // mtime order, not vec order: same four entries, shuffled vec.
        let mut shuffled: Vec<_> = (0..4usize).map(|i| mk(3 - i, i as u64, 100)).collect();
        shuffled.reverse();
        let evicted = byte_cap_evictions(&shuffled, 250);
        assert!(evicted.contains(&PathBuf::from("e2.idx")));
        assert!(evicted.contains(&PathBuf::from("e3.idx")));
        assert!(
            !evicted.contains(&PathBuf::from("e0.idx")),
            "newest survives"
        );
        // A lone entry over the cap is never evicted (no thrash).
        let lone = vec![mk(0, 0, 10_000)];
        assert_eq!(byte_cap_evictions(&lone, 1), Vec::<PathBuf>::new());
        // Two entries over the cap evict only the older one.
        let two: Vec<_> = [(0u64, 9_000u64), (1, 9_000)]
            .iter()
            .enumerate()
            .map(|(i, &(age, len))| mk(i, age, len))
            .collect();
        assert_eq!(byte_cap_evictions(&two, 1), [PathBuf::from("e0.idx")]);
    }

    /// T3(b), the verification the issue asks for: an entry pinned by
    /// constant use ([`touch`] keeps its mtime fresh) survives byte-cap
    /// pressure while staler entries die, and a fresh corpus state always
    /// lands (newest mtime = last candidate). LRU over LFU is the recorded
    /// decision (see [`byte_cap_evictions`]).
    #[test]
    fn pinned_by_use_survives_and_fresh_state_lands_under_byte_cap() {
        let now = std::time::SystemTime::now();
        let entry = |name: &str, age_secs: u64, len: u64| {
            (
                now - std::time::Duration::from_secs(age_secs),
                PathBuf::from(format!("{name}.idx")),
                len,
            )
        };
        // hot = created old but touched seconds ago; the tail much staler.
        let idx = vec![
            entry("hot", 3_600, 100),
            entry("stale_a", 3_600 * 24 * 30, 100),
            entry("stale_b", 3_600 * 24 * 20, 100),
            entry("fresh", 1, 100),
        ];
        // Cap forces two evictions: both stale entries die, hot + fresh stay.
        let evicted = byte_cap_evictions(&idx, 250);
        assert_eq!(evicted.len(), 2, "two evictions to fit 200 ≤ 250");
        assert!(evicted.contains(&PathBuf::from("stale_a.idx")));
        assert!(evicted.contains(&PathBuf::from("stale_b.idx")));
        assert!(
            !evicted.contains(&PathBuf::from("hot.idx")),
            "pinned by use survives"
        );
        assert!(
            !evicted.contains(&PathBuf::from("fresh.idx")),
            "fresh state lands"
        );
    }

    /// Integration: real files, real sizes, the env knob armed — prune
    /// enforces the ceiling end to end. 4 × ~1.2 MiB vs a 2 MiB cap: the
    /// three oldest go, only the newest survives (and the count cap is not
    /// what bit: 4 ≤ [`KEEP_ENTRIES`]).
    #[test]
    fn prune_enforces_the_byte_cap() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = temp_dir("byte_cap");
        std::env::set_var(MAX_MB_ENV, "2");
        std::fs::write(dir.join("fake.txt"), b"bystander").expect("write");
        let body = vec![0u8; 1_258_291]; // ~1.2 MiB
        for i in 0..4u32 {
            let path = dir.join(format!("{i:064x}.idx"));
            std::fs::write(&path, &body).expect("write");
            let f = std::fs::File::options()
                .write(true)
                .open(&path)
                .expect("open");
            f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(i.into()))
                .expect("mtime");
        }
        prune(&dir);
        let mut remaining: Vec<String> = std::fs::read_dir(&dir)
            .expect("readdir")
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        remaining.sort();
        assert_eq!(
            remaining,
            [format!("{:064x}.idx", 3u32), "fake.txt".to_string()],
            "only the newest entry + the bystander survive the cap"
        );
        std::env::remove_var(MAX_MB_ENV);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// G1 (issue 224 T4): the cache path and the rebuild path must answer
    /// `suggest_span` with BIT-IDENTICAL outputs. Three runs over the same
    /// overlay in one sandboxed cache dir: (a) cold miss → build + store,
    /// (b) warm hit → restore, (c) wiped dir → rebuild again. The equality
    /// (a) == (b) is the gate; (c) pins that the store path itself changed
    /// nothing observable.
    #[test]
    fn g1_cached_restore_suggestions_are_bit_identical_to_rebuild() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = temp_dir("g1");
        std::env::set_var(CACHE_DIR_ENV, &dir);
        let overlay_vec: Vec<(DomainId, CorpusEntries)> = sample_entries();
        let overlay: &DomainOverlay = &overlay_vec;
        let probe_spans = [
            "fn f() -> u8 { return 7; }",
            "if x == true { panic!() }",
            "let v: Vec<_> = items.iter().map(|x| x.clone()).collect();",
            "match opt { Some(v) => v, None => String::new() }",
        ];
        let digests = |m: &mut LatentMatcher| -> Vec<String> {
            let mut out = Vec::new();
            for span in probe_spans {
                let (_top, by_domain) = m.suggest_span(span, 3);
                let mut rows: Vec<_> = by_domain
                    .into_iter()
                    .map(|(d, rules)| (domain_tag(d), format!("{rules:?}")))
                    .collect();
                rows.sort();
                out.push(format!("{span}|{rows:?}"));
            }
            out
        };

        // (a) cold miss → build + store. `cache_dir()` appends `/index`
        // under the env-set base — the entry lands in `dir/index/`.
        let idx_dir = dir.join("index");
        let mut m =
            LatentMatcher::from_enabled_domains_with_overlay(overlay).expect("build (miss)");
        let cold = digests(&mut m);
        // Issue 226 T1: the store is deferred — join it before reading the
        // dir. Posture-aware (issue 228 D-d): under `kernel_route_narrow`
        // the kernel index narrows and the store arm is skipped BY DESIGN
        // (the Narrowed arm nulls `snapshots_to_store`), so the posture
        // asserts the skip itself.
        #[cfg(not(feature = "kernel_route_narrow"))]
        assert!(
            finish_pending_stores() >= 1,
            "the cold miss deferred a store"
        );
        #[cfg(feature = "kernel_route_narrow")]
        assert_eq!(
            finish_pending_stores(),
            0,
            "the narrowed-arm posture skips the store (the design)"
        );
        // The narrow posture never created the dir (no store ran) — the
        // listing is the stored-posture's instrument.
        #[cfg(not(feature = "kernel_route_narrow"))]
        let stored: Vec<PathBuf> = std::fs::read_dir(&idx_dir)
            .expect("cache dir")
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("idx")))
            .collect();
        #[cfg(feature = "kernel_route_narrow")]
        let stored: Vec<PathBuf> = Vec::new();
        #[cfg(not(feature = "kernel_route_narrow"))]
        assert_eq!(stored.len(), 1, "exactly one entry after the cold run");
        #[cfg(feature = "kernel_route_narrow")]
        assert_eq!(
            stored.len(),
            0,
            "the narrowed-arm posture stores nothing (the design)"
        );

        // (b) warm hit → restore. The whole point: identical answers.
        let mut m =
            LatentMatcher::from_enabled_domains_with_overlay(overlay).expect("restore (hit)");
        let warm = digests(&mut m);
        assert_eq!(cold, warm, "restored matcher must answer bit-identically");

        // (c) wiped dir → the plain rebuild path again.
        for p in &stored {
            std::fs::remove_file(p).expect("wipe entry");
        }
        let mut m =
            LatentMatcher::from_enabled_domains_with_overlay(overlay).expect("rebuild (wiped)");
        let rebuilt = digests(&mut m);
        finish_pending_stores();
        assert_eq!(
            cold, rebuilt,
            "the store path must change nothing observable"
        );

        std::env::remove_var(CACHE_DIR_ENV);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Issue 226 T1: the deferred store lands the SAME entry the inline
    /// store would (byte-identical file), only after the join — and the
    /// join is idempotent.
    #[test]
    fn deferred_store_lands_identical_entry_after_join() {
        // PENDING is process-global: serialize with the G1 lane (which also
        // drains it) so neither test joins the other's writer mid-read.
        let _g = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let inline_dir = temp_dir("i226-inline");
        let deferred_dir = temp_dir("i226-deferred");
        let config = RetrievalConfig::default();
        let snapshots = sample_snapshots();
        let key = [7u8; 32];
        store(&inline_dir, &key, &config, &snapshots).expect("inline store");
        let armed = LaneArmed {
            dir: deferred_dir.clone(),
            key,
            hex: hex(&key),
        };
        let captures = sample_snapshots()
            .iter()
            .map(|s| sample_capture(s.domain))
            .collect();
        store_deferred(&armed, config, captures);
        assert!(finish_pending_stores() >= 1, "one store was pending");
        let name = format!("{}.idx", hex(&key));
        let a = std::fs::read(inline_dir.join(&name)).expect("inline entry");
        let b = std::fs::read(deferred_dir.join(&name)).expect("deferred entry");
        assert_eq!(a, b, "deferred store must write the identical entry");
        assert!(matches!(
            load(&deferred_dir, &key, &config),
            LoadOutcome::Hit(..)
        ));
        let _ = std::fs::remove_dir_all(&inline_dir);
        let _ = std::fs::remove_dir_all(&deferred_dir);
    }

    /// Issue 226: one content key must encode to ONE byte string — the
    /// HashMap-ordered zone map is canonicalized by `finish` (the measured
    /// pre-fix drift was ~2 MB of the 18.8 MiB entry between cold runs).
    #[test]
    fn capture_finish_canonicalizes_zone_map_order() {
        let snap = sample_capture(DomainId::Clippy).finish().expect("finish");
        let hashes: Vec<[u8; 32]> = snap.by_zone_hash.iter().map(|(h, _)| *h).collect();
        assert_eq!(
            hashes,
            vec![[1u8; 32], [2u8; 32]],
            "zone map sorted by hash"
        );
        let config = RetrievalConfig::default();
        let a = encode(&config, &[snap]);
        let b = encode(&config, &[sample_snapshot(DomainId::Clippy)]);
        assert_eq!(a, b, "same content, same bytes");
    }

    /// Issue 226 T1: a dead writer's `.tmp-*` is swept once stale; a fresh
    /// one (a live concurrent writer) is never touched.
    #[test]
    fn prune_sweeps_only_stale_tmp_litter() {
        let dir = temp_dir("i226-tmp");
        std::fs::create_dir_all(&dir).expect("dir");
        let stale = dir.join(".tmp-1-1");
        let fresh = dir.join(".tmp-2-2");
        std::fs::write(&stale, b"x").expect("stale");
        std::fs::write(&fresh, b"x").expect("fresh");
        let old = std::time::SystemTime::now() - STALE_TMP_AGE - std::time::Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&stale)
            .and_then(|f| f.set_modified(old))
            .expect("age the stale tmp");
        prune(&dir);
        assert!(!stale.exists(), "stale tmp litter is swept");
        assert!(fresh.exists(), "a live writer's tmp is never touched");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

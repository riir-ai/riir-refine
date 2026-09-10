//! The corpus-lease CLIENT half — Proposal 005 Phase 1 T1.6 (Plan 040),
//! per §"Corpus sync-back lifecycle" (normative) + the update-first phase +
//! delta-scoped boundary reheat + the QR/out-of-KAT warn UX + the
//! `update`/`sync`/`claim` commands. Gated by the proposal's **G10**
//! (client halves a–e).
//!
//! ## The wire contract is a MIROR, not a dependency
//!
//! riir-clippy may NOT depend on riir-dapps (the boundary allowlist). The
//! lease wire format is a **versioned serialization contract**: this module
//! mirrors riir-dapps `src/kat/lease.rs` BYTE-FOR-BYTE — the domain
//! separator `b"kat-lease-manifest-v1"`, [`LeaseManifest::canonical_body`]
//! (fixed-width LE, fixed field order), `to_bytes`, `decode`, and the
//! commitment `BLAKE3(domain ‖ canonical body)` — pinned by **golden
//! vectors derived from the dapps side** (see `golden` in the test module).
//! A drift on either side breaks the pinned vectors; that IS the drift
//! detector (the C# `WireProtocol` golden-fixture precedent, riir-unity
//! Issue 003).
//!
//! ## Layout
//!
//! - §1 the wire mirror ([`LeaseManifest`]/[`LeaseSegmentDesc`]/[
//!   `LeaseError`]/[`LeaseVerdict`]/[`LeaseStage`])
//! - §3 the corpus pack — the segment payload's semantic contract ([[
//!   `CorpusPack`]])
//! - §4 the local lease store + the lease-charge projection ([`LeaseStore`],
//!   [`LeaseCharge`])
//! - §5 the transport + the bounded update phase (`kat_transport`)
//! - §6 the reheat planner — delta-scoped re-billing ([`plan_reheat`])
//! - §7 the payments UX — QR/payment URL/claim queue ([`queue_claim`])
//!
//! `kat_service` alone gets §1/§3/§4/§6/§7 (std + blake3 + serde only — the
//! feature stays dep-light); §5's network leg + the QR render need
//! `kat_transport` (`dep:ureq` + `dep:qrcode`).
//!
//! ## G3 posture (offline is a mode, never a failure)
//!
//! Every network path here is: bounded, silent on failure, and never blocks
//! healing — the update phase prints ONLY when an epoch actually applied
//! (or when the user ran an explicit command). With kat features ON and
//! offline (and no service URL resolved at run start), behavior is
//! byte-identical to features-off: the store is empty, the compiled-in
//! corpus stands, nothing prints.

// The wire format is MIRRORED from dapps byte-for-byte + golden-pinned: its
// `usize as u32` field encodings are the contract (a >4 GiB corpus segment
// is not a real shape — the format encodes len as u32 on BOTH sides), so
// the truncation lint is answered here once, not per site.
#![allow(unknown_lints)]
#![allow(clippy::cast_possible_truncation)]

use std::path::{Path, PathBuf};

// ── §1 the wire mirror ─────────────────────────────────────────────────────

/// The manifest format version — MUST equal dapps' `MANIFEST_VERSION`. A
/// bump on either side is a new domain separator + new commitment space
/// (never recycle).
pub const LEASE_MANIFEST_VERSION: u32 = 1;

/// The commitment domain separator — MUST equal dapps'
/// `MANIFEST_DOMAIN` byte-for-byte (pinned by the golden vectors).
const MANIFEST_DOMAIN: &[u8] = b"kat-lease-manifest-v1";

/// Failures of the lease channel (the dapps `KatLeaseError` mirror + the
/// client-side transport/compat failures). `ManifestTamper` is the G10c
/// verdict: a body that does not hash to its own commitment — surfaced as
/// an error so the caller falls back to local, NEVER as a usable manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseError {
    Store(String),
    Corrupt,
    /// G10c: the body does not hash to its own commitment.
    ManifestTamper,
    ImmutableEpoch(u64),
    EpochWentBackwards {
        current: u64,
        tried: u64,
    },
    SegmentHashMismatch,
    SegmentLengthMismatch {
        expected: u32,
        actual: u32,
    },
    ForeignSegment([u8; 32]),
    MissingSegment([u8; 32]),
    /// Swap-or-nothing: commit refused before verification.
    NotVerified,
    NoEpochPublished,
    BadVersion(u32),
    /// The server's epoch needs a newer client (compat matrix).
    Incompatible {
        min_client: u32,
        client: u32,
    },
}

impl std::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(e) => write!(f, "store io: {e}"),
            Self::Corrupt => write!(f, "corrupt lease row"),
            Self::ManifestTamper => write!(
                f,
                "manifest tamper: body does not hash to its own commitment"
            ),
            Self::ImmutableEpoch(e) => {
                write!(
                    f,
                    "epoch {e} is already published — manifests are immutable"
                )
            }
            Self::EpochWentBackwards { current, tried } => {
                write!(f, "epoch went backwards: current {current}, tried {tried}")
            }
            Self::SegmentHashMismatch => {
                write!(f, "segment hash mismatch: id does not name these bytes")
            }
            Self::SegmentLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "segment length mismatch: manifest says {expected}, got {actual}"
                )
            }
            Self::ForeignSegment(id) => {
                write!(f, "segment {} is not in the staged manifest", hex32(id))
            }
            Self::MissingSegment(id) => {
                write!(f, "manifest segment {} missing from the stage", hex32(id))
            }
            Self::NotVerified => write!(f, "stage not verified — commit refused (swap-or-nothing)"),
            Self::NoEpochPublished => write!(f, "no epoch published yet"),
            Self::BadVersion(v) => write!(f, "unsupported manifest version {v}"),
            Self::Incompatible { min_client, client } => write!(
                f,
                "epoch needs client >= {min_client} (this client is {client}) — staying on local"
            ),
        }
    }
}

impl std::error::Error for LeaseError {}

/// One content-addressed segment: its BLAKE3 id + byte length (the dapps
/// `SegmentDesc` mirror).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeaseSegmentDesc {
    pub id: [u8; 32],
    pub len: u32,
}

impl LeaseSegmentDesc {
    /// Describe `bytes` content-addressed (the only sanctioned constructor).
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            id: *blake3::hash(bytes).as_bytes(),
            len: bytes.len() as u32,
        }
    }
}

/// A versioned, self-committed epoch manifest — the `/epoch/<n>` body and
/// the source of the `ETag` (the dapps `EpochManifest` mirror, byte-identical
/// serialization).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeaseManifest {
    pub version: u32,
    pub epoch: u64,
    /// `BLAKE3(MANIFEST_DOMAIN ‖ canonical body)` — **the `ETag` value**.
    pub commitment: [u8; 32],
    /// Compat-matrix entry: the minimum client version that may apply this
    /// epoch (clients below it stay on local + warn).
    pub min_client_version: u32,
    pub segments: Vec<LeaseSegmentDesc>,
}

impl LeaseManifest {
    /// Canonical body bytes — everything the commitment covers (the whole
    /// manifest except the commitment field). Fixed-width LE, fixed field
    /// order: deterministic by construction. MUST stay byte-identical to
    /// dapps `EpochManifest::canonical_body` (golden-pinned).
    #[must_use]
    pub fn canonical_body(
        version: u32,
        epoch: u64,
        min_client_version: u32,
        segments: &[LeaseSegmentDesc],
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(20 + 36 * segments.len());
        out.extend_from_slice(&version.to_le_bytes());
        out.extend_from_slice(&epoch.to_le_bytes());
        out.extend_from_slice(&min_client_version.to_le_bytes());
        out.extend_from_slice(&(segments.len() as u32).to_le_bytes());
        for s in segments {
            out.extend_from_slice(&s.id);
            out.extend_from_slice(&s.len.to_le_bytes());
        }
        out
    }

    /// Derive the commitment over a canonical body (domain-separated).
    #[must_use]
    pub fn commitment_over(body: &[u8]) -> [u8; 32] {
        let mut h = blake3::Hasher::new();
        h.update(MANIFEST_DOMAIN);
        h.update(body);
        *h.finalize().as_bytes()
    }

    /// Build a manifest, deriving its commitment (the only sanctioned
    /// constructor for a trustworthy manifest).
    #[must_use]
    pub fn build(epoch: u64, min_client_version: u32, segments: Vec<LeaseSegmentDesc>) -> Self {
        let body =
            Self::canonical_body(LEASE_MANIFEST_VERSION, epoch, min_client_version, &segments);
        let commitment = Self::commitment_over(&body);
        Self {
            version: LEASE_MANIFEST_VERSION,
            epoch,
            commitment,
            min_client_version,
            segments,
        }
    }

    /// Full row bytes (canonical body ‖ commitment) — the served/stored form.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Self::canonical_body(
            self.version,
            self.epoch,
            self.min_client_version,
            &self.segments,
        );
        out.extend_from_slice(&self.commitment);
        out
    }

    /// Byte length of one full row for `n` segments (decode validation).
    #[must_use]
    pub fn row_len(n: usize) -> usize {
        20 + 36 * n + 32
    }

    /// Decode + **verify** a full row: wrong length, unsupported version, or
    /// a commitment that does not cover the body are all tamper-class
    /// refusals — the decoded value is never returned on mismatch (G10c).
    ///
    /// # Panics
    ///
    /// Never in practice: the `try_into().expect()` slicing is guarded by the
    /// exact-length check above it (the wire format is fixed-width).
    ///
    /// # Errors
    ///
    /// [`LeaseError::BadVersion`] for an unknown format version;
    /// [`LeaseError::ManifestTamper`] for any structurally-invalid or
    /// commitment-mismatched body.
    pub fn decode(bytes: &[u8]) -> Result<Self, LeaseError> {
        if bytes.len() < Self::row_len(0) {
            return Err(LeaseError::ManifestTamper);
        }
        let version = u32::from_le_bytes(bytes[0..4].try_into().expect("4 byte slice"));
        if version != LEASE_MANIFEST_VERSION {
            return Err(LeaseError::BadVersion(version));
        }
        let epoch = u64::from_le_bytes(bytes[4..12].try_into().expect("8 byte slice"));
        let min_client_version =
            u32::from_le_bytes(bytes[12..16].try_into().expect("4 byte slice"));
        let n = u32::from_le_bytes(bytes[16..20].try_into().expect("4 byte slice")) as usize;
        if bytes.len() != Self::row_len(n) {
            return Err(LeaseError::ManifestTamper);
        }
        let mut segments = Vec::with_capacity(n);
        for i in 0..n {
            let base = 20 + 36 * i;
            let mut id = [0u8; 32];
            id.copy_from_slice(&bytes[base..base + 32]);
            let len = u32::from_le_bytes(
                bytes[base + 32..base + 36]
                    .try_into()
                    .expect("4 byte slice"),
            );
            segments.push(LeaseSegmentDesc { id, len });
        }
        let mut commitment = [0u8; 32];
        commitment.copy_from_slice(&bytes[20 + 36 * n..]);
        let manifest = Self {
            version,
            epoch,
            commitment,
            min_client_version,
            segments,
        };
        // The self-verification: the body must hash to its own tag.
        let derived = Self::commitment_over(&bytes[..20 + 36 * n]);
        if derived != commitment {
            return Err(LeaseError::ManifestTamper);
        }
        Ok(manifest)
    }

    /// Re-derive this manifest's commitment from its own fields.
    #[must_use]
    pub fn self_commitment(&self) -> [u8; 32] {
        let body = Self::canonical_body(
            self.version,
            self.epoch,
            self.min_client_version,
            &self.segments,
        );
        Self::commitment_over(&body)
    }

    /// True iff the manifest is internally consistent.
    #[must_use]
    pub fn is_self_consistent(&self) -> bool {
        self.self_commitment() == self.commitment
    }
}

/// Verify a served/stored manifest body (decode + self-commitment check).
/// The G10c entry point: tamper surfaces as `Err`, never as a manifest.
///
/// # Errors
///
/// See [`LeaseManifest::decode`].
pub fn verify_manifest(bytes: &[u8]) -> Result<LeaseManifest, LeaseError> {
    LeaseManifest::decode(bytes)
}

/// The compat-matrix check: may a client at `client_version` apply this epoch?
#[must_use]
pub fn compat_ok(manifest: &LeaseManifest, client_version: u32) -> bool {
    client_version >= manifest.min_client_version
}

/// The manifest DIFF — which segments a client at `local` must fetch, in
/// manifest order (deterministic: same manifest + same local set →
/// bit-identical list). The dapps `KatLease::manifest_diff` mirror.
#[must_use]
pub fn manifest_diff(manifest: &LeaseManifest, local: &[[u8; 32]]) -> Vec<[u8; 32]> {
    manifest
        .segments
        .iter()
        .filter(|s| !local.contains(&s.id))
        .map(|s| s.id)
        .collect()
}

/// The `/lease` conditional endpoint's answer: `NotModified` = the 304 case
/// (no body, **no charge**); `Manifest` = the 200 case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaseVerdict {
    NotModified,
    Manifest(LeaseManifest),
}

/// A staged atomic swap: download-to-temp → verify → commit. In-memory
/// until [`LeaseStage::commit`]; any interrupt (drop, timeout) before
/// commit is a **no-op** on the local corpus — there is no
/// partially-applied state (G10b). The dapps `StagedSwap` mirror.
#[derive(Clone, Debug)]
pub struct LeaseStage {
    manifest: LeaseManifest,
    staged: Vec<([u8; 32], std::sync::Arc<[u8]>)>,
}

impl LeaseStage {
    /// Begin staging a fetched manifest. Refuses a manifest that is not
    /// self-consistent before anything is staged (tamper never enters the
    /// pipeline).
    ///
    /// # Errors
    ///
    /// [`LeaseError::ManifestTamper`] when the manifest's commitment does
    /// not cover its body.
    pub fn begin(manifest: LeaseManifest) -> Result<Self, LeaseError> {
        if !manifest.is_self_consistent() {
            return Err(LeaseError::ManifestTamper);
        }
        Ok(Self {
            manifest,
            staged: Vec::new(),
        })
    }

    /// Stage one fetched segment: verifies BLAKE3 (content-addressed) and
    /// the manifest's recorded length. A bad segment never joins the stage.
    ///
    /// # Errors
    ///
    /// [`LeaseError::ForeignSegment`] when the id is not in the manifest;
    /// [`LeaseError::SegmentHashMismatch`] / [`LeaseError::SegmentLengthMismatch`]
    /// on content disagreement.
    pub fn stage_segment(&mut self, id: &[u8; 32], bytes: &[u8]) -> Result<(), LeaseError> {
        let Some(desc) = self.manifest.segments.iter().find(|s| &s.id == id) else {
            return Err(LeaseError::ForeignSegment(*id));
        };
        if blake3::hash(bytes).as_bytes() != id {
            return Err(LeaseError::SegmentHashMismatch);
        }
        if bytes.len() as u32 != desc.len {
            return Err(LeaseError::SegmentLengthMismatch {
                expected: desc.len,
                actual: bytes.len() as u32,
            });
        }
        if self.staged.iter().any(|(sid, _)| sid == id) {
            return Ok(()); // idempotent restage
        }
        self.staged.push((*id, std::sync::Arc::from(bytes)));
        Ok(())
    }

    /// The staged manifest (read-only).
    #[must_use]
    pub fn manifest(&self) -> &LeaseManifest {
        &self.manifest
    }

    /// True when every manifest segment is staged + verified.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.manifest
            .segments
            .iter()
            .all(|s| self.staged.iter().any(|(id, _)| id == &s.id))
    }

    /// Full verification: the epoch commitment covers the manifest + every
    /// segment present (per-segment hashes were checked at stage time).
    ///
    /// # Errors
    ///
    /// [`LeaseError::ManifestTamper`] / [`LeaseError::MissingSegment`].
    pub fn verify(&self) -> Result<(), LeaseError> {
        if !self.manifest.is_self_consistent() {
            return Err(LeaseError::ManifestTamper);
        }
        for s in &self.manifest.segments {
            if !self
                .staged
                .iter()
                .any(|(id, b)| id == &s.id && b.len() as u32 == s.len)
            {
                return Err(LeaseError::MissingSegment(s.id));
            }
        }
        Ok(())
    }

    /// The atomic handoff: consumes the stage and returns the verified
    /// bundle. Refused until [`Self::verify`] passes — swap-or-nothing.
    ///
    /// # Errors
    ///
    /// Whatever [`Self::verify`] returns; the stage is consumed either way.
    pub fn commit(self) -> Result<LeasedEpoch, LeaseError> {
        self.verify()?;
        Ok(LeasedEpoch {
            manifest: self.manifest,
            segments: self
                .staged
                .into_iter()
                .map(|(id, b)| (id, b.to_vec()))
                .collect(),
        })
    }
}

/// A fully verified epoch bundle — the artifact the store installs.
#[derive(Clone, Debug)]
pub struct LeasedEpoch {
    pub manifest: LeaseManifest,
    pub segments: Vec<([u8; 32], Vec<u8>)>,
}

/// Lower-case hex of a 32-byte id (URL/`ETag`/path spelling).
#[must_use]
pub fn hex32(id: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(64);
    for b in id {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Parse 64-char lower-case hex into a 32-byte id.
#[must_use]
pub fn unhex32(s: &str) -> Option<[u8; 32]> {
    let b = s.as_bytes();
    if b.len() != 64 || !b.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

// ── §3 the corpus pack — the segment payload's semantic contract ───────────

/// The corpus-pack schema version. A segment's wire bytes are opaque
/// (content-addressed); this is the SEMANTIC contract for what a leased
/// segment carries — an evolved corpus pack for ONE domain. The service
/// side produces these when the Worker shell lands; the format is pinned
/// here because the client defines the install semantics.
pub const CORPUS_PACK_VERSION: u32 = 1;

/// The domain spellings a pack may carry (mapped to `DomainId` one layer
/// up, in the CLI glue).
pub const PACK_DOMAIN_CLIPPY: &str = "clippy";
pub const PACK_DOMAIN_PERF: &str = "perf";
pub const PACK_DOMAIN_KERNEL: &str = "kernel";
pub const PACK_DOMAIN_SEC: &str = "sec";
pub const PACK_DOMAIN_DOCKER: &str = "docker";
/// Issue 085 — the dist domain's pack spelling. No server packs it yet; the
/// client decodes it so an older binary never rejects a newer pack loudly.
pub const PACK_DOMAIN_DIST: &str = "dist";

/// One evolved corpus entry (the retrieval triple).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PatternEntry {
    pub rule_id: String,
    pub buggy_code: String,
    pub fixed_code: String,
}

/// A corpus pack — one domain's corpus state inside one segment. An EMPTY
/// entries list makes no statement (the domain keeps its compiled-in
/// corpus).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CorpusPack {
    pub pack_version: u32,
    pub domain: String,
    pub entries: Vec<PatternEntry>,
}

/// Parse + validate a corpus pack.
///
/// # Errors
///
/// A `String` message when the bytes are not valid pack JSON, the version
/// is unknown, or the domain is empty.
pub fn parse_pack(bytes: &[u8]) -> Result<CorpusPack, String> {
    let pack: CorpusPack =
        serde_json::from_slice(bytes).map_err(|e| format!("corpus pack is not valid JSON: {e}"))?;
    if pack.pack_version != CORPUS_PACK_VERSION {
        return Err(format!(
            "corpus pack carries version {} (this client defines {CORPUS_PACK_VERSION})",
            pack.pack_version
        ));
    }
    if pack.domain.is_empty() {
        return Err("corpus pack carries an empty domain".to_string());
    }
    Ok(pack)
}

/// Render a pack to its canonical bytes (the producer side of the
/// contract — used by tests + the future local mining flow).
#[must_use]
pub fn render_pack(pack: &CorpusPack) -> Vec<u8> {
    serde_json::to_vec(pack).unwrap_or_default()
}

/// The per-domain retrieval triples a corpus pack yields: domain name →
/// merged, deterministically-ordered `(rule_id, buggy, fixed)` entries.
pub type DomainTriples = Vec<(String, Vec<(String, String, String)>)>;

/// The per-domain overlay an installed epoch yields: domain name → merged,
/// deterministically-ordered retrieval triples (entries from ALL of the
/// epoch's packs for that domain, sorted + deduplicated by `rule_id`).
/// Returns `None` when the epoch is unusable (missing/corrupt segment or
/// pack) — the caller then stays on the compiled-in corpus.
#[must_use]
pub fn overlay_from_epoch(store: &LeaseStore, epoch: Option<u64>) -> Option<(u64, DomainTriples)> {
    let manifest = match epoch {
        Some(e) => store.manifest_at(e)?,
        None => store.current()?,
    };
    let mut by_domain: std::collections::BTreeMap<String, Vec<PatternEntry>> =
        std::collections::BTreeMap::new();
    for desc in &manifest.segments {
        let bytes = store.segment_bytes(&desc.id)?;
        let pack = parse_pack(&bytes).ok()?;
        if pack.entries.is_empty() {
            continue; // no statement for the domain
        }
        by_domain
            .entry(pack.domain)
            .or_default()
            .extend(pack.entries);
    }
    let overlay = by_domain
        .into_iter()
        .map(|(domain, mut entries)| {
            entries.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
            entries.dedup_by(|a, b| a.rule_id == b.rule_id);
            (
                domain,
                entries
                    .into_iter()
                    .map(|e| (e.rule_id, e.buggy_code, e.fixed_code))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    Some((manifest.epoch, overlay))
}

// ── §4 the local lease store + charge projection ───────────────────────────

/// The store directory under `.heal` (env-overridable for tests).
pub const STORE_DIR_NAME: &str = "corpus-lease";

/// The pointer-file shape version.
pub const POINTER_VERSION: u32 = 1;

/// The on-disk pointer: which epoch is current + its full manifest shape.
/// Reconstructed into a [`LeaseManifest`] (and self-consistency-checked —
/// a tampered pointer is not a usable epoch).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LeasePointer {
    pub pointer_version: u32,
    pub epoch: u64,
    pub commitment_hex: String,
    pub min_client_version: u32,
    pub segments: Vec<PointerSegment>,
}

/// One segment row inside [`LeasePointer`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PointerSegment {
    pub id_hex: String,
    pub len: u32,
}

impl LeasePointer {
    /// Build from a manifest.
    #[must_use]
    pub fn of(manifest: &LeaseManifest) -> Self {
        Self {
            pointer_version: POINTER_VERSION,
            epoch: manifest.epoch,
            commitment_hex: hex32(&manifest.commitment),
            min_client_version: manifest.min_client_version,
            segments: manifest
                .segments
                .iter()
                .map(|s| PointerSegment {
                    id_hex: hex32(&s.id),
                    len: s.len,
                })
                .collect(),
        }
    }

    /// Reconstruct the manifest (version + commitment verified against the
    /// canonical body). Returns `None` for any corrupt/tampered pointer.
    #[must_use]
    pub fn to_manifest(&self) -> Option<LeaseManifest> {
        let mut segments = Vec::with_capacity(self.segments.len());
        for s in &self.segments {
            segments.push(LeaseSegmentDesc {
                id: unhex32(&s.id_hex)?,
                len: s.len,
            });
        }
        let m = LeaseManifest {
            version: LEASE_MANIFEST_VERSION,
            epoch: self.epoch,
            commitment: unhex32(&self.commitment_hex)?,
            min_client_version: self.min_client_version,
            segments,
        };
        m.is_self_consistent().then_some(m)
    }
}

/// The local lease store: content-addressed segments + per-epoch manifests
/// + the current-epoch pointer, all under `<root>/.heal/corpus-lease/`.
///
/// **Atomicity invariant (G10b)**: [`Self::install_segments`] writes
/// content-addressed segment files + the epoch manifest (each tmp+rename,
/// idempotent); [`Self::publish_pointer`] is THE swap (one rename). Any
/// interrupt before the pointer rename leaves the previous epoch fully
/// intact — there is no partially-applied corpus state.
#[derive(Debug, Clone)]
pub struct LeaseStore {
    dir: PathBuf,
}

impl LeaseStore {
    /// Open at `<root>/.heal/corpus-lease` (honors `RIIR_HEAL_KAT_LEASE_DIR`).
    #[must_use]
    pub fn open(root: &Path) -> Self {
        if let Ok(d) = std::env::var("RIIR_HEAL_KAT_LEASE_DIR") {
            return Self::at_dir(PathBuf::from(d));
        }
        Self::at_dir(root.join(".heal").join(STORE_DIR_NAME))
    }

    /// Open at an exact directory (tests).
    #[must_use]
    pub fn at_dir(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The store directory (diagnostics).
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn seg_path(&self, id: &[u8; 32]) -> PathBuf {
        self.dir.join("seg").join(format!("{}.seg", hex32(id)))
    }

    fn man_path(&self, epoch: u64) -> PathBuf {
        self.dir.join("man").join(format!("{epoch}.bin"))
    }

    fn ptr_path(&self) -> PathBuf {
        self.dir.join("current.json")
    }

    /// The current (installed) epoch's manifest, verified — `None` when no
    /// epoch is installed or the pointer is corrupt/tampered (fall back to
    /// compiled-in).
    #[must_use]
    pub fn current(&self) -> Option<LeaseManifest> {
        let text = std::fs::read_to_string(self.ptr_path()).ok()?;
        let p: LeasePointer = serde_json::from_str(&text).ok()?;
        p.to_manifest()
    }

    /// A stored epoch's manifest (for `--corpus-epoch` pinning), verified.
    #[must_use]
    pub fn manifest_at(&self, epoch: u64) -> Option<LeaseManifest> {
        let bytes = std::fs::read(self.man_path(epoch)).ok()?;
        LeaseManifest::decode(&bytes).ok()
    }

    /// The current epoch number, if installed.
    #[must_use]
    pub fn current_epoch(&self) -> Option<u64> {
        self.current().map(|m| m.epoch)
    }

    /// The local segment ids of the CURRENT epoch that exist on disk (the
    /// manifest-diff input).
    #[must_use]
    pub fn local_segment_ids(&self) -> Vec<[u8; 32]> {
        self.current()
            .map(|m| {
                m.segments
                    .iter()
                    .map(|s| s.id)
                    .filter(|id| self.seg_path(id).exists())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A segment's bytes by content id.
    #[must_use]
    pub fn segment_bytes(&self, id: &[u8; 32]) -> Option<Vec<u8>> {
        std::fs::read(self.seg_path(id)).ok()
    }

    /// Install step 1 (pre-swap): write every segment + the epoch manifest.
    /// Content-addressed + idempotent — safe to re-run after an interrupt.
    ///
    /// # Errors
    ///
    /// [`LeaseError::Store`] on filesystem failure.
    pub fn install_segments(&self, le: &LeasedEpoch) -> Result<(), LeaseError> {
        for (id, bytes) in &le.segments {
            let final_path = self.seg_path(id);
            if final_path.exists()
                && blake3::hash(
                    &std::fs::read(&final_path).map_err(|e| LeaseError::Store(e.to_string()))?,
                )
                .as_bytes()
                    == id.as_slice()
            {
                continue; // already installed + verified
            }
            write_file_atomic(&final_path, bytes, false)
                .map_err(|e| LeaseError::Store(e.to_string()))?;
        }
        write_file_atomic(
            &self.man_path(le.manifest.epoch),
            &le.manifest.to_bytes(),
            false,
        )
        .map_err(|e| LeaseError::Store(e.to_string()))
    }

    /// Install step 2 — **THE pointer swap** (one rename; atomic on every
    /// platform for same-directory tmp files).
    ///
    /// # Errors
    ///
    /// [`LeaseError::Store`] on filesystem failure.
    pub fn publish_pointer(&self, manifest: &LeaseManifest) -> Result<(), LeaseError> {
        let text = serde_json::to_string(&LeasePointer::of(manifest))
            .map_err(|e| LeaseError::Store(e.to_string()))?;
        write_file_atomic(&self.ptr_path(), text.as_bytes(), true)
            .map_err(|e| LeaseError::Store(e.to_string()))
    }

    /// The full atomic install: segments → pointer (swap-or-nothing).
    ///
    /// # Errors
    ///
    /// [`LeaseError::Store`] on filesystem failure; any interrupt before
    /// the pointer rename is a no-op on the installed epoch.
    pub fn install_atomic(&self, le: &LeasedEpoch) -> Result<(), LeaseError> {
        self.sweep_stale_tmp();
        self.install_segments(le)?;
        self.publish_pointer(&le.manifest)
    }

    /// Best-effort removal of stale `*.tmp-*` files left by an interrupted
    /// install (harmless either way — content-addressed files re-write
    /// identically; the pointer tmp is never a pointer).
    pub fn sweep_stale_tmp(&self) {
        let mut dirs = vec![self.dir.clone(), self.dir.join("seg"), self.dir.join("man")];
        for d in &mut dirs {
            let Ok(entries) = std::fs::read_dir(&d) else {
                continue;
            };
            for e in entries.flatten() {
                let name = e.file_name();
                let Some(name) = name.to_str() else { continue };
                if name.contains(".tmp-") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }
}

/// Write `bytes` to `path` atomically: tmp in the same directory (same
/// filesystem) → rename. `sync` additionally fsyncs the file before the
/// rename (used for the pointer — the one write where losing a power
/// cycle mid-swap would matter).
fn write_file_atomic(path: &Path, bytes: &[u8], sync: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let tmp = path.with_file_name(format!(
        "{}.tmp-{}-{nanos}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        std::process::id()
    ));
    {
        use std::io::Write as _;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        if sync {
            f.sync_all()?;
        }
    }
    std::fs::rename(&tmp, path)
}

/// The lease-charge model version. Independent of [`crate::kat_meter`]'s
/// `CHARGE_VERSION` on purpose: that versions the per-token burn; this
/// versions the flat per-epoch lease fee. Either change is a reprice with
/// an audit-trail entry, never silent.
pub const LEASE_CHARGE_VERSION: u32 = 1;

/// The flat lease fee per applied epoch — the proposal §Account figure
/// (`corpus lease epoch N: −1,000 KAT`). Accounting-only projection (G9d);
/// the server ledger is the authority + is idempotent per (account, epoch).
pub const LEASE_FEE_KAT: u64 = 1_000;

/// Versioned deltas of the lease-charge model (the reprice audit trail).
pub const LEASE_CHARGE_CHANGELOG: &[(u32, &str)] = &[(
    1,
    "initial lease charge (T1.6, Plan 040): flat 1,000 KAT per applied epoch \
         (the proposal §Account mock figure); idempotent per epoch; a 304 never \
         charges (the charge only composes on the accepted-epoch path).",
)];

/// The persisted lease-charge projection.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LeaseChargeState {
    pub lease_charge_version: u32,
    pub charged_kat: u64,
    /// Epochs already charged — the idempotence set (the same epoch
    /// transition never re-charges).
    pub charged_epochs: Vec<u64>,
}

impl Default for LeaseChargeState {
    fn default() -> Self {
        Self {
            lease_charge_version: LEASE_CHARGE_VERSION,
            charged_kat: 0,
            charged_epochs: Vec::new(),
        }
    }
}

/// The lease-charge ledger (local projection; the server ledger
/// reconciles at sync and is the authority — caveat #9).
#[derive(Debug)]
pub struct LeaseCharge {
    path: PathBuf,
    state: LeaseChargeState,
    session_kat: u64,
}

impl LeaseCharge {
    /// Open (or start) the ledger at `<store>/charge.json`. A corrupt or
    /// unknown-version file starts fresh — bookkeeping never aborts
    /// anything (the meter precedent).
    #[must_use]
    pub fn open(store: &LeaseStore) -> Self {
        let path = store.dir.join("charge.json");
        let state = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .filter(|s: &LeaseChargeState| s.lease_charge_version == LEASE_CHARGE_VERSION)
            .unwrap_or_default();
        Self {
            path,
            state,
            session_kat: 0,
        }
    }

    /// Charge the flat fee for `epoch` — **idempotent per epoch** (a
    /// re-apply of an already-charged epoch charges 0; a 304 never reaches
    /// here at all). Returns the KAT charged.
    pub fn charge_epoch(&mut self, epoch: u64) -> u64 {
        if self.state.charged_epochs.contains(&epoch) {
            return 0;
        }
        self.state.charged_epochs.push(epoch);
        self.state.charged_kat = self.state.charged_kat.saturating_add(LEASE_FEE_KAT);
        self.session_kat = self.session_kat.saturating_add(LEASE_FEE_KAT);
        LEASE_FEE_KAT
    }

    /// Total KAT charged across all epochs (the projection).
    #[must_use]
    pub fn charged_kat(&self) -> u64 {
        self.state.charged_kat
    }

    /// KAT charged in this process (diagnostics).
    #[must_use]
    pub fn session_kat(&self) -> u64 {
        self.session_kat
    }

    /// Persist. Save failure is lost bookkeeping, never a failed heal.
    ///
    /// # Errors
    ///
    /// The underlying `io::Error`.
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(
            &self.path,
            serde_json::to_string_pretty(&self.state).unwrap_or_default(),
        )
    }

    /// The visible charge line (KAT only — no `$`, caveats #12/#15).
    #[must_use]
    pub fn line(epoch: u64, kat: u64) -> String {
        if kat == 0 {
            format!("corpus lease epoch {epoch}: 0 KAT (already charged this epoch)")
        } else {
            format!("corpus lease epoch {epoch}: -{kat} KAT")
        }
    }
}

// ── §6 the reheat planner — delta-scoped re-billing ────────────────────────

/// The span identity across passes: `BLAKE3(chunk text)` — stable while the
/// chunk is byte-identical, changes when a fix (or the corpus) changes it.
#[must_use]
pub fn span_key(text: &str) -> [u8; 32] {
    *blake3::hash(text.as_bytes()).as_bytes()
}

/// One span's retrieval verdict under one epoch (the pass-2 input).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassSpans {
    pub key: [u8; 32],
    /// Word tokens of the span's text (the re-bill unit — G9's counter).
    pub tokens: u64,
    /// The top-k candidate rule ids, sorted + deduplicated (the compared
    /// set).
    pub rules: Vec<String>,
}

/// The delta-scoped reheat plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReheatPlan {
    /// Changed spans (key + tokens), sorted by key — deterministic.
    pub changed: Vec<([u8; 32], u64)>,
    /// Spans re-glanced with an identical candidate set — NOT re-billed
    /// (retrieval-only; "a re-glanced span with no new candidates was not
    /// re-processed").
    pub unchanged: usize,
    /// Sum of the changed spans' tokens — exactly what re-bills.
    pub reheat_tokens: u64,
}

/// Plan the delta: compare every pass-2 span against pass-1's candidate
/// sets. A span is CHANGED when its key is absent from pass 1 (new/edited
/// chunk — a fix changed the tree) or its candidate set differs under
/// E+1. Only changed spans re-enter propose/verify + re-bill.
///
/// Deterministic: the changed list is key-sorted and the token sum is
/// order-independent — same passes → bit-identical plan (the G9
/// discipline, extended to the reheat).
#[allow(clippy::type_complexity)] // the pass-1 map is the natural shape
#[allow(clippy::implicit_hasher)] // a hasher generic buys nothing at one call-site family
#[must_use]
pub fn plan_reheat(
    pass1: &std::collections::HashMap<[u8; 32], Vec<String>>,
    pass2: &[PassSpans],
) -> ReheatPlan {
    let mut changed = Vec::new();
    let mut unchanged = 0usize;
    for s in pass2 {
        match pass1.get(&s.key) {
            Some(prev) if *prev == s.rules => unchanged += 1,
            _ => changed.push((s.key, s.tokens)),
        }
    }
    changed.sort_unstable();
    let reheat_tokens = changed.iter().map(|(_, t)| t).sum();
    ReheatPlan {
        changed,
        unchanged,
        reheat_tokens,
    }
}

/// The run-end reheat report — the itemized output's source (G10e).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReheatReport {
    pub pass1_epoch: u64,
    pub pass2_epoch: u64,
    pub pass1_tokens: u64,
    pub pass2_tokens: u64,
    pub changed_spans: usize,
    pub new_fixes: usize,
}

impl ReheatReport {
    /// The itemized line: pass-1/pass-2 token counts + the new-fix count,
    /// separated — no silent aggregates.
    #[must_use]
    pub fn itemized_line(&self) -> String {
        let e1 = self.pass1_epoch;
        let t1 = self.pass1_tokens;
        let e2 = self.pass2_epoch;
        let t2 = self.pass2_tokens;
        let n = self.new_fixes;
        format!("pass 1 (epoch {e1}): {t1} tokens · pass 2 (epoch {e2}): {t2} tokens, +{n} fixes")
    }
}

// ── §7 the payments UX — QR / payment URL / claim queue ────────────────────

/// The service plane's default base URL (the payment page rides the same
/// host: `<base>/pay?acct=<account_id>`).
pub const DEFAULT_SERVICE_URL: &str = "https://ai.gist.rs";

/// The failover PAIR of [`DEFAULT_SERVICE_URL`] — the other known service
/// host, REPOINTED 2026-09-10 from the retired `heal.gist.rs` to the devnet
/// subdomain (riir-dapps Plan 023 S2): after the domain swap the apex IS
/// mainnet, and a DEV client must never fail over onto MAINNET — the pair's
/// direction is default→devnet only. A configured (`cargo heal config set`)
/// or env-set URL has NO pair — the shipped "never custom" rule — so no
/// cross-network failover can fire for an opted-in dev machine. Born
/// 2026-09-09 when release binaries shipped a default with no public DNS
/// record; the three HTTP clients (lease / payreq / wetrun-read) retry a
/// HOST-level failure once against the pair (see [`failover_base`]) and
/// stick to it for the process lifetime.
pub const FALLBACK_SERVICE_URL: &str = "https://devnet.ai.gist.rs";

/// The process-wide failover memo (see [`failover_base`]): once a host-level
/// failure flips the CLI to the paired host, every later request in the same
/// process starts there — one flip line, ever, per run.
#[cfg(feature = "kat_transport")]
static FAILED_OVER: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// The failover pair for a service base URL: the two known service hosts
/// fall back to EACH OTHER; any other (operator-set / test-double) URL is
/// respected literally and falls back to nothing.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn fallback_pair_for(base: &str) -> Option<String> {
    match base.trim_end_matches('/') {
        DEFAULT_SERVICE_URL => Some(FALLBACK_SERVICE_URL.to_string()),
        FALLBACK_SERVICE_URL => Some(DEFAULT_SERVICE_URL.to_string()),
        _ => None,
    }
}

/// Host-level = the request provably never reached a server (DNS resolution,
/// connection refused). Timeouts and TLS failures are deliberately NOT
/// failover triggers: a timed-out push may have landed (every money write
/// here is server-side idempotent, but an unneeded retry is still a retry),
/// and a TLS failure names a proxy/operator problem the paired host cannot
/// fix.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn is_host_level_error(e: &ureq::Error) -> bool {
    is_host_level_kind(e.kind())
}

/// The [`is_host_level_error`] classification over the public `ureq::ErrorKind`
/// enum — a separate fn because transport errors themselves are
/// `pub(crate)`-constructed and so untestable from outside the crate.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn is_host_level_kind(kind: ureq::ErrorKind) -> bool {
    matches!(
        kind,
        ureq::ErrorKind::Dns | ureq::ErrorKind::ConnectionFailed
    )
}

/// The base URL the next request should use: the failover memo when one is
/// recorded, else the client's own base (trailing slash normalized).
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn request_base(base: &str) -> String {
    FAILED_OVER
        .get()
        .cloned()
        .unwrap_or_else(|| base.trim_end_matches('/').to_string())
}

/// One-shot failover: on a HOST-level error against a base with a paired
/// host, record + return the pair (the caller retries against it). Server
/// statuses, timeouts, custom URLs, and anything after the first flip answer
/// `None` — not ours to retry.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn failover_base(tried: &str, e: &ureq::Error) -> Option<String> {
    if !is_host_level_error(e) {
        return None;
    }
    let next = failover_flip(tried)?;
    eprintln!("service: {tried} unreachable ({e}) — falling back to {next}");
    Some(next)
}

/// The state half of [`failover_base`] (memo check + pair + set), error-free
/// so the same-module tests can drive it deterministically.
#[cfg(feature = "kat_transport")]
fn failover_flip(tried: &str) -> Option<String> {
    if FAILED_OVER.get().is_some() {
        return None;
    }
    let next = fallback_pair_for(tried)?;
    FAILED_OVER.set(next.clone()).ok()?;
    Some(next)
}

/// The payment-page URL for an account (the QR payload).
#[must_use]
pub fn payment_url(service_base: &str, account_id: &str) -> String {
    format!(
        "{}/pay?acct={account_id}",
        service_base.trim_end_matches('/')
    )
}

/// The claim-request schema version.
pub const CLAIM_VERSION: u32 = 1;

/// A payment claim: the client composes + submits (the five-check
/// verification is service-side); on the Worker-deferred reality it is
/// validated + queued locally — **never a client-side mint** (G9d).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClaimRequest {
    pub claim_version: u32,
    pub account_id: String,
    /// `"solana"` (tx signature) or `"stripe"` (event id).
    pub rail: String,
    /// The payment proof reference (the Solana tx signature here).
    pub reference: String,
    pub queued_unix: u64,
}

/// Solana signatures are base58 (Bitcoin alphabet), 64..=88 chars.
#[must_use]
pub fn validate_solana_signature(sig: &str) -> bool {
    (64..=88).contains(&sig.len())
        && sig
            .bytes()
            .all(|b| matches!(b, b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z'))
}

/// Queue a claim locally (`.heal/claims/pending.jsonl`, append). The queued
/// claims submit when the Worker shell lands; nothing is minted here.
///
/// # Errors
///
/// The underlying `io::Error` from directory creation or the append.
pub fn queue_claim(root: &Path, req: &ClaimRequest) -> std::io::Result<PathBuf> {
    use std::io::Write as _;
    let path = root.join(".heal").join("claims").join("pending.jsonl");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "{}", serde_json::to_string(req).unwrap_or_default())?;
    Ok(path)
}

/// Queue a sync payload locally (`.heal/outbox/`, the Worker-deferred
/// egress) — returns the queued file path. Deterministic name from the
/// batch commitment so a re-queue of the same batch is one file.
///
/// # Errors
///
/// The underlying `io::Error`.
pub fn queue_outbox(root: &Path, ndjson: &str, commitment: &str) -> std::io::Result<PathBuf> {
    let short: String = commitment.chars().take(12).collect();
    let path = root
        .join(".heal")
        .join("outbox")
        .join(format!("{short}.ndjson"));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, ndjson)?;
    Ok(path)
}

// ── §5 the transport + the bounded update phase (kat_transport) ────────────

/// This client's compat-matrix version (bumped when the lease consumer's
/// semantics change in a way a manifest must gate on).
pub const LEASE_CLIENT_VERSION: u32 = 1;

/// The transport-level response (the raw HTTP verdict, before any lease
/// interpretation).
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpResponse {
    NotModified,
    Ok(Vec<u8>),
    Status(u16),
    Error(String),
}

/// The lease channel transport — one implementation per protocol (the
/// ureq HTTP impl here; the std-TCP mock in tests drives the same trait).
#[cfg(feature = "kat_transport")]
pub trait LeaseTransport {
    /// `GET /lease` with `If-None-Match: <local epoch commitment hex>` →
    /// 304 (one RTT, no body) or 200 manifest body.
    fn get_lease(&self, if_none_match: Option<&str>) -> HttpResponse;
    /// `GET /segment/<hex>` → 200 segment body.
    fn get_segment(&self, id: &[u8; 32]) -> HttpResponse;
    /// `POST /sync` (NDJSON body) → 200 accepted.
    fn post_sync(&self, ndjson: &str) -> HttpResponse;
    /// `POST /claim` (JSON body) → 200 queued.
    fn post_claim(&self, json: &str) -> HttpResponse;
}

/// The ureq HTTP transport (the production impl; TLS default features).
#[cfg(feature = "kat_transport")]
pub struct HttpLeaseTransport {
    agent: ureq::Agent,
    base: String,
}

#[cfg(feature = "kat_transport")]
impl HttpLeaseTransport {
    /// Build with a per-request timeout (seconds; the update phase's
    /// bounded window). Every request — connect, headers, AND body read —
    /// is covered (a stalled server cannot outlive the budget).
    #[must_use]
    pub fn new(base: &str, timeout_secs: u64) -> Self {
        let t = std::time::Duration::from_secs(timeout_secs.max(1));
        let agent = ureq::AgentBuilder::new()
            .timeout(t)
            .timeout_connect(t)
            .timeout_read(t)
            .build();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
        }
    }

    fn get(&self, path: &str, if_none_match: Option<&str>) -> HttpResponse {
        let mut base = request_base(&self.base);
        loop {
            let url = format!("{base}{path}");
            let mut req = self.agent.get(&url);
            if let Some(v) = if_none_match {
                req = req.set("If-None-Match", v);
            }
            match req.call() {
                Ok(r) => return Self::body(r),
                // ureq surfaces some non-2xx as Status errors — 304 is the one
                // we care about (both shapes handled defensively).
                Err(ureq::Error::Status(304, _)) => return HttpResponse::NotModified,
                Err(ureq::Error::Status(c, _)) => return HttpResponse::Status(c),
                Err(e) => match failover_base(&base, &e) {
                    Some(next) => base = next,
                    None => return HttpResponse::Error(e.to_string()),
                },
            }
        }
    }

    fn post(&self, path: &str, body: &str) -> HttpResponse {
        let mut base = request_base(&self.base);
        loop {
            let url = format!("{base}{path}");
            match self
                .agent
                .post(&url)
                .set("Content-Type", "application/x-ndjson")
                .send_string(body)
            {
                Ok(r) => return Self::body(r),
                Err(ureq::Error::Status(c, _)) => return HttpResponse::Status(c),
                Err(e) => match failover_base(&base, &e) {
                    Some(next) => base = next,
                    None => return HttpResponse::Error(e.to_string()),
                },
            }
        }
    }

    fn body(r: ureq::Response) -> HttpResponse {
        use std::io::Read as _;
        match r.status() {
            304 => HttpResponse::NotModified,
            200 => {
                let mut buf = Vec::new();
                match r.into_reader().read_to_end(&mut buf) {
                    Ok(_) => HttpResponse::Ok(buf),
                    Err(e) => HttpResponse::Error(e.to_string()),
                }
            }
            c => HttpResponse::Status(c),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl LeaseTransport for HttpLeaseTransport {
    fn get_lease(&self, if_none_match: Option<&str>) -> HttpResponse {
        self.get("/lease", if_none_match)
    }

    fn get_segment(&self, id: &[u8; 32]) -> HttpResponse {
        self.get(&format!("/segment/{}", hex32(id)), None)
    }

    fn post_sync(&self, ndjson: &str) -> HttpResponse {
        self.post("/sync", ndjson)
    }

    fn post_claim(&self, json: &str) -> HttpResponse {
        // The claim endpoint takes one JSON object (not NDJSON).
        let url = format!("{}/claim", self.base);
        match self
            .agent
            .post(&url)
            .set("Content-Type", "application/json")
            .send_string(json)
        {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }
}

/// The update phase's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// The 304 case — no body, **no charge**, proceed instantly.
    NotModified,
    /// The 200 case — applied + installed; `charge_kat` is the idempotent
    /// lease fee (0 when this epoch was already charged).
    Applied { epoch: u64, charge_kat: u64 },
    /// A benign non-application (out-of-KAT pause, compat, no URL).
    Skipped(String),
    /// A transport/verification failure — **always** falls back to local;
    /// never blocks healing (G3).
    Failed(String),
}

/// The update phase's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOutcome {
    pub status: UpdateStatus,
    pub elapsed: std::time::Duration,
}

impl UpdateOutcome {
    /// Whether the run-start phase should print NOTHING for this outcome
    /// (G3 silence: only an applied epoch — or an explicit command — is
    /// worth a line mid-run).
    #[must_use]
    pub fn silent(&self) -> bool {
        !matches!(self.status, UpdateStatus::Applied { .. })
    }
}

/// The bounded update phase — the T1.6 flow end-to-end (proposal rule
/// chain): lease check → 304 fast path | 200 → decode+verify (tamper →
/// reject + fall back local) → compat matrix → segment diff → fetch →
/// stage (per-segment BLAKE3+len) → verify → **atomic swap** → idempotent
/// lease charge.
///
/// `deadline` bounds the multi-segment case across fetches (each transport
/// call is additionally bounded by the transport's own timeout). Any
/// failure returns [`UpdateStatus::Failed`] with the local corpus
/// untouched — the caller proceeds on local (G3/G10a).
#[cfg(feature = "kat_transport")]
pub fn run_update_phase(
    transport: &dyn LeaseTransport,
    store: &LeaseStore,
    charge: &mut LeaseCharge,
    out_of_kat: bool,
    deadline: Option<std::time::Instant>,
) -> UpdateOutcome {
    let start = std::time::Instant::now();
    let outcome = update_phase_inner(transport, store, charge, out_of_kat, deadline, start);
    UpdateOutcome {
        status: outcome,
        elapsed: start.elapsed(),
    }
}

#[cfg(feature = "kat_transport")]
#[allow(clippy::too_many_lines)] // the phase is one linear contract chain (the proposal's rule sequence); splitting it would hide the flow
fn update_phase_inner(
    transport: &dyn LeaseTransport,
    store: &LeaseStore,
    charge: &mut LeaseCharge,
    out_of_kat: bool,
    deadline: Option<std::time::Instant>,
    start: std::time::Instant,
) -> UpdateStatus {
    // Warn-and-pay (§KAT): at 0 remaining the evolved-corpus hot-push
    // pauses at epoch end; mining accrual/settlement continue.
    if out_of_kat {
        return UpdateStatus::Skipped(
            "out-of-KAT — evolved-corpus hot-push paused at epoch end (mining continues; \
             warn-and-pay, never brick)"
                .to_string(),
        );
    }
    let current = store.current();
    let etag = current.as_ref().map(|m| hex32(&m.commitment));
    let body = match transport.get_lease(etag.as_deref()) {
        HttpResponse::NotModified => return UpdateStatus::NotModified,
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => {
            return UpdateStatus::Failed(format!("lease endpoint returned HTTP {c}"))
        }
        HttpResponse::Error(e) => {
            return UpdateStatus::Failed(format!("lease endpoint unreachable: {e}"))
        }
    };
    // G10c: a body that does not hash to its own commitment never becomes
    // a manifest — decode+verify refuses, we fall back to local.
    let manifest = match LeaseManifest::decode(&body) {
        Ok(m) => m,
        Err(LeaseError::BadVersion(v)) => {
            return UpdateStatus::Failed(format!("lease manifest carries version {v}"))
        }
        Err(e) => return UpdateStatus::Failed(format!("{e} — falling back to the local corpus")),
    };
    if !compat_ok(&manifest, LEASE_CLIENT_VERSION) {
        return UpdateStatus::Skipped(format!(
            "epoch {} needs client >= {} (this client is {LEASE_CLIENT_VERSION}) — staying local",
            manifest.epoch, manifest.min_client_version
        ));
    }
    if let Some(cur) = &current {
        if manifest.epoch < cur.epoch {
            return UpdateStatus::Failed(format!(
                "server epoch went backwards (current {}, tried {}) — refusing",
                cur.epoch, manifest.epoch
            ));
        }
    }
    let fail = |m: String| UpdateStatus::Failed(m);
    let mut stage = match LeaseStage::begin(manifest.clone()) {
        Ok(s) => s,
        Err(e) => return fail(format!("stage refused: {e}")),
    };
    let local_ids = store.local_segment_ids();
    // Fetch only what changed (content-addressed diff — rule 3).
    for id in manifest_diff(&manifest, &local_ids) {
        if let Some(d) = deadline {
            if std::time::Instant::now() >= d {
                return fail(format!(
                    "update window expired after {} — proceeding on the local corpus",
                    start.elapsed().as_secs()
                ));
            }
        }
        match transport.get_segment(&id) {
            HttpResponse::Ok(b) => {
                if let Err(e) = stage.stage_segment(&id, &b) {
                    return fail(format!("segment {} rejected: {e}", hex32(&id)));
                }
            }
            HttpResponse::NotModified => {
                return fail(format!(
                    "segment endpoint returned 304 for {} — malformed service",
                    hex32(&id)
                ))
            }
            HttpResponse::Status(c) => {
                return fail(format!("segment {} returned HTTP {c}", hex32(&id)))
            }
            HttpResponse::Error(e) => {
                return fail(format!("segment {} unreachable: {e}", hex32(&id)))
            }
        }
    }
    // Unchanged segments join the stage from the local store (they were
    // verified at install time + content-addressed on disk).
    for desc in &manifest.segments {
        if stage.is_complete() {
            break;
        }
        if local_ids.contains(&desc.id) {
            let Some(bytes) = store.segment_bytes(&desc.id) else {
                return fail(format!(
                    "local segment {} vanished mid-update — retrying fetches next run",
                    hex32(&desc.id)
                ));
            };
            if let Err(e) = stage.stage_segment(&desc.id, &bytes) {
                return fail(format!("local segment {} rejected: {e}", hex32(&desc.id)));
            }
        }
    }
    let leased = match stage.commit() {
        Ok(le) => le,
        Err(e) => return fail(format!("verification failed: {e} — local corpus untouched")),
    };
    let epoch = leased.manifest.epoch;
    if let Err(e) = store.install_atomic(&leased) {
        return fail(format!("install failed: {e} — local corpus untouched"));
    }
    let charge_kat = charge.charge_epoch(epoch);
    UpdateStatus::Applied { epoch, charge_kat }
}

/// The run-start phase's service-URL resolution — env ONLY, so the ambient
/// lease phase goes online exactly when the operator EXPLICITLY configured
/// a service URL (a persisted `cargo heal config set` record re-points the
/// EXPLICIT commands via [`service_url_for_command`], never the ambient
/// run-start phase). Delegates the read to `kat_service_config` so the var
/// spelling has one owner.
#[must_use]
pub fn service_url_from_env() -> Option<String> {
    crate::kat_service_config::service_url_from_env()
}

/// The explicit-command URL resolution (riir-dapps Plan 023):
/// `RIIR_HEAL_KAT_SERVICE_URL` > the persisted config (`cargo heal config
/// set`) > [`DEFAULT_SERVICE_URL`] — which never moves (the QR goldens pin
/// it; the Solana-CLI model: default mainnet, devnet is opt-in).
#[must_use]
pub fn service_url_for_command() -> String {
    crate::kat_service_config::resolved_service_url(DEFAULT_SERVICE_URL).0
}

/// Whether the whole lease phase (update + reheat) is kill-switched off
/// (`RIIR_HEAL_KAT_LEASE=0|off|false|no` — the `RIIR_HEAL_KAT_METER`
/// family's spelling; CI never touches the network).
#[must_use]
pub fn lease_kill_switched() -> bool {
    std::env::var("RIIR_HEAL_KAT_LEASE")
        .is_ok_and(|v| matches!(v.as_str(), "0" | "off" | "false" | "no"))
}

// ── the QR asset: render params, commitment, and the caveat-#3 pins ─────
//
// Proposal 011 T1.4 (Plan 064). **Caveat #3 restated: QR determinism is a
// PINNED DEPENDENCY, not a property of QR.** A byte-exact re-encode holds
// only while the `qrcode` crate version AND the render params (ECC level,
// quiet zone) stay put. So the pin must be sensitive to BOTH axes — a pin
// that silently survived a `qrcode` patch bump inside the `"0.14"`
// requirement range, or a change of ECC level, would be doing nothing.
//
// Two independent mechanisms, because neither alone is enough:
//
// 1. **Structural** — the crate version and both render params ride IN the
//    commitment preimage ([`qr_commitment_stamped`]), so changing any of
//    them MOVES the digest by construction. Divergence probes in the test
//    module assert each axis actually moves (an in-preimage field that no
//    test varies is an assumption, not a pin).
// 2. **Measured** — [`QRCODE_PINNED_VERSION`] is a hand-written declaration
//    of provenance, and a hand-written constant cannot notice a `cargo
//    update`. `qrcode_resolved_version_is_the_pinned_one` reads the
//    RESOLVED version out of `Cargo.lock` and fails on drift, which is the
//    half that catches `0.14.1 → 0.14.2`.
//
// Mechanism 1 without 2 lets a lock bump change every stored asset's
// commitment while every golden still passes (the constant did not move,
// so neither did the preimage). Mechanism 2 without 1 leaves the
// commitment version-blind. Keep both.

/// The pinned QR render parameters (caveat #3's "ECC, quiet zone").
/// Both members ride in the [`qr_commitment`] preimage, so a change here
/// moves every commitment — deliberately, since it also changes every
/// rendered asset.
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QrRenderParams {
    /// The error-correction level. `qrcode::QrCode::new` uses `M`
    /// implicitly; naming it here makes the param visible to review and to
    /// the commitment.
    pub ec_level: qrcode::EcLevel,
    /// Whether the render carries the 4-module quiet zone.
    pub quiet_zone: bool,
}

/// The shipped render params — the values every stored KAT QR asset was
/// encoded under. Changing either member is a **re-pin event**: the
/// goldens in this module's tests move, and so does every stored
/// invoice's commitment (caveat #3's re-encode-on-read fallback is what
/// absorbs the pre-change assets).
#[cfg(feature = "kat_transport")]
pub const QR_RENDER_PARAMS: QrRenderParams = QrRenderParams {
    ec_level: qrcode::EcLevel::M,
    quiet_zone: true,
};

/// The **resolved** `qrcode` version the QR goldens were captured against,
/// and the version stamped into every [`qr_commitment`] preimage.
///
/// Pinned by `qrcode_resolved_version_is_the_pinned_one`, which reads
/// `Cargo.lock` — not `Cargo.toml`. The manifest only asks for `"0.14"`, so
/// the requirement is satisfied by any `0.14.x`; only the lock knows which
/// one actually compiled.
#[cfg(feature = "kat_transport")]
pub const QRCODE_PINNED_VERSION: &str = "0.14.1";

/// The `Cargo.toml` requirement string for `qrcode`, pinned separately so
/// widening it (`"0.14"` → `"0.15"`) fails a test *before* the lock moves.
#[cfg(feature = "kat_transport")]
pub const QRCODE_PINNED_REQUIREMENT: &str = "0.14";

/// Domain separator for the QR asset commitment (bump the `v1` on any
/// preimage-layout change — a layout change is not a param change).
#[cfg(feature = "kat_transport")]
pub const QR_COMMITMENT_DOMAIN: &[u8] = b"riir-kat-qr-v1:";

/// The ASCII QR render of `url` under the pinned [`QR_RENDER_PARAMS`] (the
/// `qrcode` crate's dense unicode block renderer — terminal-friendly).
/// Errors surface as an empty string (the QR is a convenience, never a
/// gate).
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn render_qr_ascii(url: &str) -> String {
    render_qr_ascii_with(url, QR_RENDER_PARAMS)
}

/// [`render_qr_ascii`] with explicit params — exists so the divergence
/// probes can vary a param without editing the shipped constant.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn render_qr_ascii_with(url: &str, params: QrRenderParams) -> String {
    use qrcode::render::unicode::Dense1x2;
    use qrcode::QrCode;
    match QrCode::with_error_correction_level(url, params.ec_level) {
        Ok(code) => code
            .render::<Dense1x2>()
            .quiet_zone(params.quiet_zone)
            .build(),
        Err(_) => String::new(),
    }
}

/// The QR asset commitment (Proposal 011 §1's "bit-exact recheck before
/// display", the payee/CLI half): BLAKE3 over a canonical serialization of
/// the QR **module matrix** — the encoder's own output, not a rendering.
///
/// Committing the matrix rather than a rendered image is what lets this
/// repo participate at all: `qrcode` ships here with
/// `default-features = false` (no `image`), so there is no PNG to hash —
/// and both a PNG and the ASCII render are pure functions of the matrix, so
/// the matrix is the *stronger* commitment, not a weaker stand-in.
///
/// `None` when the URL cannot be encoded (too long for any version) — the
/// same never-a-gate posture as [`render_qr_ascii`].
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn qr_commitment(url: &str) -> Option<[u8; 32]> {
    qr_commitment_with(url, QR_RENDER_PARAMS)
}

/// [`qr_commitment`] with explicit params (the render-param divergence
/// axis of the caveat-#3 pin).
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn qr_commitment_with(url: &str, params: QrRenderParams) -> Option<[u8; 32]> {
    qr_commitment_stamped(url, params, QRCODE_PINNED_VERSION)
}

/// [`qr_commitment`] with explicit params AND an explicit crate-version
/// stamp (the crate-version divergence axis).
///
/// Preimage layout ([`QR_COMMITMENT_DOMAIN`] `v1`):
/// `domain` ‖ `version_len` u8 ‖ `crate_version` bytes ‖ `ec_level` u8 ‖
/// `quiet_zone` u8 ‖ `width` u32 le ‖ one byte per module (row-major,
/// `1` = dark, `0` = light).
///
/// The length prefix is not decoration: without it a version/param
/// boundary shift could collide two different pins.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn qr_commitment_stamped(
    url: &str,
    params: QrRenderParams,
    crate_version: &str,
) -> Option<[u8; 32]> {
    use qrcode::{Color, QrCode};
    let code = QrCode::with_error_correction_level(url, params.ec_level).ok()?;
    let width = u32::try_from(code.width()).ok()?;
    let version_len = u8::try_from(crate_version.len()).ok()?;
    let mut h = blake3::Hasher::new();
    h.update(QR_COMMITMENT_DOMAIN);
    h.update(&[version_len]);
    h.update(crate_version.as_bytes());
    h.update(&[params.ec_level as u8]);
    h.update(&[u8::from(params.quiet_zone)]);
    h.update(&width.to_le_bytes());
    for c in code.to_colors() {
        h.update(&[match c {
            Color::Dark => 1u8,
            Color::Light => 0u8,
        }]);
    }
    Some(*h.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex digit"))
            .collect()
    }

    // ── §2 the golden vectors (the cross-repo drift detector) ────────────
    //
    // PROVENANCE: captured 2026-08-27 against riir-dapps @ 84dd049
    // (`feat(kat): T1.5 corpus lease substrate + T2.3 reward schedule`) by
    // running dapps' `EpochManifest::build` on the exact test shapes from
    // its `src/kat/lease.rs` test module (`manifest_commitment_is_
    // deterministic_and_content_sensitive` + `decode_verifies_and_round_
    // trips`) via a temporary dump test (deleted after capture; the dapps
    // tree was left clean). A serialization change on EITHER side breaks
    // these vectors — that IS the drift detector (the C# WireProtocol
    // golden-fixture precedent).

    /// dapps `manifest(1, vec![seg(b"alpha")])` — build(1, 3, [alpha]).
    const GOLDEN_M1_BYTES: &str = "0100000001000000000000000300000001000000644a9bc57c6063e2ba4028fa73ed585170ae7db8ac7723d32be49c021a0225f5050000001835e35ec6a4870bf943c4a1174fd2db2cefaaf6e567979b2dc3b07e88489d63";
    const GOLDEN_M1_COMMIT: &str =
        "1835e35ec6a4870bf943c4a1174fd2db2cefaaf6e567979b2dc3b07e88489d63";
    /// blake3(b"alpha") — the first segment id of M1.
    const GOLDEN_SEG_ALPHA: &str =
        "644a9bc57c6063e2ba4028fa73ed585170ae7db8ac7723d32be49c021a0225f5";

    /// dapps `manifest(7, vec![seg(b"x"), seg(b"yy")])` — the decode-test
    /// shape.
    const GOLDEN_M2_BYTES: &str = "01000000070000000000000003000000020000003ae7d805f6789a6402acb70ad4096a85a56bf6804eaf25c0493ac697548d30b5010000001d3665edb47f75a2d5d956ee1331e464c38c5108852b182d0259a7e3f20374ae020000001cdde30d6bd152f61a925c235060b3360b9aab1db2bf24e4da01ab56ed129790";
    const GOLDEN_M2_COMMIT: &str =
        "1cdde30d6bd152f61a925c235060b3360b9aab1db2bf24e4da01ab56ed129790";

    /// dapps `manifest(0, vec![])` — the zero-segment edge.
    const GOLDEN_M3_BYTES: &str = "0100000000000000000000000300000000000000552e3f85fb634eb9bb310a95e0661e113826cb85b64705365cef81a27125a72a";

    /// `build(42, 5, [seg(b""), seg(b"zzz"), seg(b"x")])` — three segments
    /// incl. the empty-bytes edge + a different `min_client_version`. Note
    /// `seg("")`'s id is the canonical `blake3("")` =
    /// af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f326.
    const GOLDEN_M4_BYTES: &str = "010000002a000000000000000500000003000000af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f326200000000c35fed1498b44f7f3f8012abedd4e2540a8442dbe09aab9b0c803a658f33d50b030000003ae7d805f6789a6402acb70ad4096a85a56bf6804eaf25c0493ac697548d30b5010000009629869b5492220ee02ff2e1cd07aea3a90f269eba25696d585f7e57cb6753b3";

    #[test]
    fn golden_m1_matches_dapps_bytes() {
        let m = LeaseManifest::build(1, 3, vec![LeaseSegmentDesc::of(b"alpha")]);
        assert_eq!(hex32(&m.commitment), GOLDEN_M1_COMMIT);
        assert_eq!(m.to_bytes(), unhex(GOLDEN_M1_BYTES));
        assert_eq!(hex32(&LeaseSegmentDesc::of(b"alpha").id), GOLDEN_SEG_ALPHA);
        // And the mirror decodes the dapps bytes back.
        assert_eq!(LeaseManifest::decode(&unhex(GOLDEN_M1_BYTES)).unwrap(), m);
    }

    #[test]
    fn golden_m2_matches_dapps_bytes() {
        let m = LeaseManifest::build(
            7,
            3,
            vec![LeaseSegmentDesc::of(b"x"), LeaseSegmentDesc::of(b"yy")],
        );
        assert_eq!(hex32(&m.commitment), GOLDEN_M2_COMMIT);
        assert_eq!(m.to_bytes(), unhex(GOLDEN_M2_BYTES));
        assert_eq!(LeaseManifest::decode(&unhex(GOLDEN_M2_BYTES)).unwrap(), m);
    }

    #[test]
    fn golden_m3_zero_segments() {
        let m = LeaseManifest::build(0, 3, vec![]);
        assert_eq!(m.to_bytes(), unhex(GOLDEN_M3_BYTES));
        assert!(LeaseManifest::decode(&unhex(GOLDEN_M3_BYTES))
            .unwrap()
            .is_self_consistent());
    }

    #[test]
    fn golden_m4_empty_segment_bytes_and_order_sensitivity() {
        let m = LeaseManifest::build(
            42,
            5,
            vec![
                LeaseSegmentDesc::of(b""),
                LeaseSegmentDesc::of(b"zzz"),
                LeaseSegmentDesc::of(b"x"),
            ],
        );
        assert_eq!(m.to_bytes(), unhex(GOLDEN_M4_BYTES));
        // Segment order is part of the commitment (the dapps property).
        let reordered = LeaseManifest::build(
            42,
            5,
            vec![
                LeaseSegmentDesc::of(b"x"),
                LeaseSegmentDesc::of(b"zzz"),
                LeaseSegmentDesc::of(b""),
            ],
        );
        assert_ne!(m.commitment, reordered.commitment);
    }

    #[test]
    fn golden_tamper_is_refused_not_decoded() {
        // The G10c case against the pinned bytes: flip one body byte (the
        // epoch field) → the body no longer hashes to its own commitment.
        let mut tampered = unhex(GOLDEN_M2_BYTES);
        tampered[4] ^= 0xFF;
        assert!(matches!(
            LeaseManifest::decode(&tampered),
            Err(LeaseError::ManifestTamper)
        ));
        // Truncation + extension are tamper too.
        let bytes = unhex(GOLDEN_M2_BYTES);
        assert!(LeaseManifest::decode(&bytes[..bytes.len() - 1]).is_err());
        let mut extended = bytes.clone();
        extended.push(0);
        assert!(LeaseManifest::decode(&extended).is_err());
        // An unsupported version is a loud version error.
        let mut bad_version = bytes;
        bad_version[0..4].copy_from_slice(&99u32.to_le_bytes());
        assert!(matches!(
            LeaseManifest::decode(&bad_version),
            Err(LeaseError::BadVersion(99))
        ));
    }

    // ── wire-mirror properties ────────────────────────────────────────────

    #[test]
    fn commitment_is_deterministic_and_content_sensitive() {
        let a = LeaseManifest::build(1, 3, vec![LeaseSegmentDesc::of(b"alpha")]);
        let b = LeaseManifest::build(1, 3, vec![LeaseSegmentDesc::of(b"alpha")]);
        assert_eq!(a.commitment, b.commitment);
        assert_ne!(
            a.commitment,
            LeaseManifest::build(1, 3, vec![LeaseSegmentDesc::of(b"beta")]).commitment
        );
        assert_ne!(
            a.commitment,
            LeaseManifest::build(2, 3, vec![LeaseSegmentDesc::of(b"alpha")]).commitment
        );
        assert_ne!(
            a.commitment,
            LeaseManifest::build(1, 4, vec![LeaseSegmentDesc::of(b"alpha")]).commitment
        );
    }

    #[test]
    fn manifest_diff_is_deterministic_and_order_stable() {
        let m = LeaseManifest::build(
            1,
            1,
            vec![
                LeaseSegmentDesc::of(b"a"),
                LeaseSegmentDesc::of(b"b"),
                LeaseSegmentDesc::of(b"c"),
            ],
        );
        let ids: Vec<[u8; 32]> = m.segments.iter().map(|s| s.id).collect();
        // Nothing missing.
        assert!(manifest_diff(&m, &ids).is_empty());
        // First + third missing → manifest order (b first? no: a, c).
        assert_eq!(manifest_diff(&m, &ids[1..2]), vec![ids[0], ids[2]]);
    }

    #[test]
    fn compat_matrix_gates_old_clients() {
        let m = LeaseManifest::build(1, 5, vec![]);
        assert!(!compat_ok(&m, 4), "client below min stays local");
        assert!(compat_ok(&m, 5));
        assert!(compat_ok(&m, 6));
    }

    #[test]
    fn stage_refuses_tampered_foreign_and_short_segments() {
        let good = LeaseSegmentDesc::of(b"payload");
        let m = LeaseManifest::build(1, 1, vec![good]);
        let mut stage = LeaseStage::begin(m.clone()).unwrap();
        // Foreign segment.
        assert!(matches!(
            stage.stage_segment(&LeaseSegmentDesc::of(b"other").id, b"other"),
            Err(LeaseError::ForeignSegment(_))
        ));
        // Hash mismatch (bytes don't match the id).
        assert!(matches!(
            stage.stage_segment(&good.id, b"tampered"),
            Err(LeaseError::SegmentHashMismatch)
        ));
        // Commit before complete is refused (swap-or-nothing) — commit
        // consumes, so a fresh identical stage proves the refusal.
        let incomplete = LeaseStage::begin(m.clone()).unwrap();
        assert!(matches!(
            incomplete.commit(),
            Err(LeaseError::MissingSegment(_))
        ));
        // Good stage → complete → commit.
        stage.stage_segment(&good.id, b"payload").unwrap();
        assert!(stage.is_complete());
        let leased = stage.commit().unwrap();
        assert_eq!(leased.manifest.epoch, 1);
        assert_eq!(leased.segments[0].1, b"payload");
    }

    #[test]
    fn stage_begin_refuses_inconsistent_manifest() {
        let mut m = LeaseManifest::build(1, 1, vec![LeaseSegmentDesc::of(b"x")]);
        m.commitment = [9u8; 32]; // tamper in memory
        assert!(matches!(
            LeaseStage::begin(m),
            Err(LeaseError::ManifestTamper)
        ));
    }

    // ── §3 corpus packs ───────────────────────────────────────────────────

    #[test]
    fn pack_round_trips_and_validates() {
        let pack = CorpusPack {
            pack_version: CORPUS_PACK_VERSION,
            domain: PACK_DOMAIN_CLIPPY.to_string(),
            entries: vec![PatternEntry {
                rule_id: "vec-with-capacity".to_string(),
                buggy_code: "let mut v = Vec::new();".to_string(),
                fixed_code: "let mut v = Vec::with_capacity(n);".to_string(),
            }],
        };
        let bytes = render_pack(&pack);
        assert_eq!(parse_pack(&bytes).unwrap(), pack);
        // Unknown version is a loud refusal.
        let mut bad = pack.clone();
        bad.pack_version = 99;
        assert!(parse_pack(&render_pack(&bad)).is_err());
        // Empty domain is a loud refusal.
        let mut nod = pack;
        nod.domain = String::new();
        assert!(parse_pack(&render_pack(&nod)).is_err());
    }

    // ── §4 the store + charge ledger ──────────────────────────────────────

    fn temp_store(tag: &str) -> LeaseStore {
        let dir = std::env::temp_dir().join(format!(
            "riir-clippy-lease-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        LeaseStore::at_dir(dir)
    }

    fn leased(epoch: u64, payloads: &[&[u8]]) -> LeasedEpoch {
        let descs: Vec<LeaseSegmentDesc> =
            payloads.iter().copied().map(LeaseSegmentDesc::of).collect();
        let manifest = LeaseManifest::build(epoch, 1, descs);
        let mut stage = LeaseStage::begin(manifest).unwrap();
        for p in payloads {
            stage.stage_segment(&LeaseSegmentDesc::of(p).id, p).unwrap();
        }
        stage.commit().unwrap()
    }

    #[test]
    fn store_install_is_atomic_and_pointer_is_the_swap() {
        let store = temp_store("atomic");
        // Nothing installed.
        assert!(store.current().is_none());
        // Pre-state: epoch 1 installed.
        store.install_atomic(&leased(1, &[b"one", b"two"])).unwrap();
        assert_eq!(store.current_epoch(), Some(1));
        let seg_e1 = store.current().unwrap().segments.clone();

        // G10b — the fault seam: install E+1's segments WITHOUT the
        // pointer swap (the interrupt case). The next run still sees E1.
        let e2 = leased(2, &[b"one", b"two", b"three"]);
        store.install_segments(&e2).unwrap();
        assert_eq!(
            store.current_epoch(),
            Some(1),
            "an interrupt before the pointer swap leaves the old epoch intact"
        );
        // Complete the swap.
        store.publish_pointer(&e2.manifest).unwrap();
        assert_eq!(store.current_epoch(), Some(2));
        // Content-addressed dedup: E1's segments still on disk + readable.
        for s in &seg_e1 {
            assert!(store.segment_bytes(&s.id).is_some());
        }
        // The new segment is there + the old epoch's manifest is pinnable.
        assert!(store.manifest_at(1).is_some());
        assert!(store.manifest_at(2).is_some());
        assert!(store.manifest_at(3).is_none());
    }

    #[test]
    fn store_current_rejects_a_tampered_pointer() {
        let store = temp_store("tampered-pointer");
        store.install_atomic(&leased(5, &[b"x"])).unwrap();
        // Flip the pointer's commitment (in-memory tamper simulation).
        let ptr_path = store.dir().join("current.json");
        let mut p: LeasePointer =
            serde_json::from_str(&std::fs::read_to_string(&ptr_path).unwrap()).unwrap();
        p.commitment_hex = "00".repeat(32);
        std::fs::write(&ptr_path, serde_json::to_string(&p).unwrap()).unwrap();
        assert!(
            store.current().is_none(),
            "a pointer whose commitment does not cover its body is not a usable epoch"
        );
    }

    #[test]
    fn local_segment_ids_only_reports_on_disk_ids() {
        let store = temp_store("ids");
        store.install_atomic(&leased(1, &[b"a", b"b"])).unwrap();
        assert_eq!(store.local_segment_ids().len(), 2);
        let m = store.current().unwrap();
        let gone = m.segments[0].id;
        std::fs::remove_file(store.dir.join("seg").join(format!("{}.seg", hex32(&gone)))).unwrap();
        assert_eq!(store.local_segment_ids(), vec![m.segments[1].id]);
    }

    #[test]
    fn lease_charge_is_idempotent_per_epoch() {
        let store = temp_store("charge");
        let mut c = LeaseCharge::open(&store);
        assert_eq!(c.charge_epoch(7), LEASE_FEE_KAT);
        assert_eq!(c.charge_epoch(7), 0, "the same epoch never re-charges");
        assert_eq!(c.charge_epoch(8), LEASE_FEE_KAT);
        assert_eq!(c.charged_kat(), LEASE_FEE_KAT * 2);
        assert_eq!(c.session_kat(), LEASE_FEE_KAT * 2);
        c.save().unwrap();
        let mut reopened = LeaseCharge::open(&store);
        assert_eq!(reopened.charged_kat(), LEASE_FEE_KAT * 2);
        assert_eq!(reopened.charge_epoch(7), 0, "idempotence survives reopen");
        // The visible line carries no $ figure (caveats #12/#15).
        let line = LeaseCharge::line(7, LEASE_FEE_KAT);
        assert!(line.contains("epoch 7"));
        assert!(line.contains("KAT"));
        assert!(!line.contains('$'));
    }

    #[test]
    fn lease_charge_corrupt_file_starts_fresh() {
        let store = temp_store("charge-corrupt");
        std::fs::create_dir_all(store.dir()).unwrap();
        std::fs::write(store.dir.join("charge.json"), "not json").unwrap();
        let c = LeaseCharge::open(&store);
        assert_eq!(c.charged_kat(), 0);
    }

    // ── §6 the reheat planner ─────────────────────────────────────────────

    #[test]
    fn reheat_plan_bills_zero_when_no_candidate_set_changed() {
        // G10d: a straddle that changes nothing costs ~retrieval-only.
        let mut pass1 = std::collections::HashMap::new();
        pass1.insert(span_key("fn a() {}"), vec!["rule-a".to_string()]);
        pass1.insert(span_key("fn b() {}"), vec!["rule-b".to_string()]);
        let pass2 = vec![
            PassSpans {
                key: span_key("fn a() {}"),
                tokens: 100,
                rules: vec!["rule-a".to_string()],
            },
            PassSpans {
                key: span_key("fn b() {}"),
                tokens: 200,
                rules: vec!["rule-b".to_string()],
            },
        ];
        let plan = plan_reheat(&pass1, &pass2);
        assert_eq!(plan.reheat_tokens, 0, "no changed set → no re-bill");
        assert_eq!(plan.unchanged, 2);
        assert!(plan.changed.is_empty());
    }

    #[test]
    fn reheat_plan_bills_exactly_the_changed_spans_deterministically() {
        // G10d: N changed spans → exactly those spans' tokens, and the same
        // inputs plan bit-identically (the G9 discipline).
        let mut pass1 = std::collections::HashMap::new();
        pass1.insert(span_key("span-1"), vec!["old".to_string()]);
        pass1.insert(span_key("span-2"), vec!["same".to_string()]);
        pass1.insert(span_key("span-3"), vec!["x".to_string()]);
        let pass2 = vec![
            // Changed candidate set under E+1.
            PassSpans {
                key: span_key("span-1"),
                tokens: 111,
                rules: vec!["new".to_string()],
            },
            // Identical set → not billed.
            PassSpans {
                key: span_key("span-2"),
                tokens: 222,
                rules: vec!["same".to_string()],
            },
            // Absent from pass 1 (edited chunk) → changed.
            PassSpans {
                key: span_key("span-4"),
                tokens: 44,
                rules: vec!["y".to_string()],
            },
        ];
        let plan = plan_reheat(&pass1, &pass2);
        assert_eq!(plan.changed.len(), 2);
        assert_eq!(plan.reheat_tokens, 111 + 44);
        assert_eq!(plan.unchanged, 1);
        // Determinism: same inputs → bit-identical plan.
        let again = plan_reheat(&pass1, &pass2);
        assert_eq!(plan, again);
    }

    #[test]
    fn reheat_itemized_line_separates_the_passes() {
        // G10e: no silent aggregates.
        let r = ReheatReport {
            pass1_epoch: 3,
            pass2_epoch: 4,
            pass1_tokens: 12_500,
            pass2_tokens: 400,
            changed_spans: 14,
            new_fixes: 14,
        };
        let line = r.itemized_line();
        assert_eq!(
            line,
            "pass 1 (epoch 3): 12500 tokens · pass 2 (epoch 4): 400 tokens, +14 fixes"
        );
    }

    // ── §7 payments UX ────────────────────────────────────────────────────

    #[test]
    fn payment_url_shape() {
        assert_eq!(
            payment_url("https://ai.gist.rs", "9f3a"),
            "https://ai.gist.rs/pay?acct=9f3a"
        );
        assert_eq!(
            payment_url("https://x.example/", "ab"),
            "https://x.example/pay?acct=ab"
        );
    }

    #[test]
    fn solana_signature_validation() {
        let good64: String = "a".repeat(64);
        let good88: String = "A".repeat(88);
        assert!(validate_solana_signature(&good64));
        assert!(validate_solana_signature(&good88));
        assert!(!validate_solana_signature(&"a".repeat(63)), "too short");
        assert!(!validate_solana_signature(&"a".repeat(89)), "too long");
        // Base58 excludes 0, I, O, l.
        assert!(!validate_solana_signature(&"0".repeat(64)));
        assert!(!validate_solana_signature(&"I".repeat(64)));
        assert!(!validate_solana_signature(&"O".repeat(64)));
        assert!(!validate_solana_signature(&"l".repeat(64)));
    }

    #[test]
    fn claim_queues_locally_and_never_mints() {
        let root = std::env::temp_dir().join(format!(
            "riir-clippy-claim-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let req = ClaimRequest {
            claim_version: CLAIM_VERSION,
            account_id: "ab".repeat(32),
            rail: "solana".to_string(),
            reference: "b".repeat(64),
            queued_unix: 1,
        };
        let path = queue_claim(&root, &req).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let back: ClaimRequest = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn hex_round_trips() {
        let id = LeaseSegmentDesc::of(b"roundtrip").id;
        assert_eq!(unhex32(&hex32(&id)), Some(id));
        assert_eq!(unhex32("zz"), None);
        assert_eq!(unhex32(&"0".repeat(63)), None);
    }

    // ── T1.4 / caveat #3: the QR re-encode pin ──────────────────────────
    //
    // These are the "the pin must actually be sensitive to the crate
    // version AND the render params" tests. Read the module comment above
    // `QrRenderParams` for why it takes two mechanisms.

    /// The canonical golden subject: a real payreq capability URL on the
    /// production service base (the T0.3 golden request id).
    #[cfg(feature = "kat_transport")]
    const QR_GOLDEN_URL: &str = "https://ai.gist.rs/payreq/71b5cbf13626d2554aba7d9b06e63474";

    /// Axis 2 (MEASURED): the **resolved** `qrcode` version in `Cargo.lock`
    /// is the one [`QRCODE_PINNED_VERSION`] declares, and the `Cargo.toml`
    /// requirement is the one [`QRCODE_PINNED_REQUIREMENT`] declares.
    ///
    /// Why the lock and not the manifest: the manifest asks for `"0.14"`,
    /// which `0.14.1`, `0.14.2` and `0.14.99` all satisfy. A pin that read
    /// only the manifest would sit green through a `cargo update` that
    /// silently re-rendered every stored QR asset — the precise failure
    /// caveat #3 names. Reading the lock is what makes a PATCH bump fail.
    ///
    /// The test also refuses to pass when it cannot FIND what it audits
    /// (missing lock, missing `qrcode` entry, missing manifest line): an
    /// auditor that vacuously passes on an empty read is worse than none.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn qrcode_resolved_version_is_the_pinned_one() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        // (a) the resolved version, out of Cargo.lock.
        let lock = std::fs::read_to_string(root.join("Cargo.lock")).expect(
            "Cargo.lock must be present and readable — the pin has nothing to read without it",
        );
        let mut lines = lock.lines();
        let mut resolved: Option<String> = None;
        while let Some(line) = lines.next() {
            if line.trim() != "name = \"qrcode\"" {
                continue;
            }
            let v = lines
                .next()
                .expect("a Cargo.lock [[package]] block always carries `version` after `name`");
            resolved = Some(
                v.trim()
                    .strip_prefix("version = \"")
                    .and_then(|s| s.strip_suffix('"'))
                    .expect("the line after `name = \"qrcode\"` must be the version line")
                    .to_string(),
            );
            break;
        }
        let resolved = resolved.expect(
            "no `name = \"qrcode\"` package in Cargo.lock — either the dep was dropped (then \
             delete this pin deliberately) or the lock is stale; the pin does not get to guess",
        );
        assert_eq!(
            resolved, QRCODE_PINNED_VERSION,
            "the RESOLVED `qrcode` version moved ({QRCODE_PINNED_VERSION} → {resolved}). \
             Caveat #3: every stored QR asset's commitment moves with it. Re-capture the QR \
             goldens in this module, bump QRCODE_PINNED_VERSION, and note the re-pin — do NOT \
             just edit the constant to make this pass."
        );

        // (b) the requirement, out of Cargo.toml — so widening the range
        //     fails BEFORE a lock bump has a chance to happen.
        let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("Cargo.toml");
        let req_line = manifest
            .lines()
            .find(|l| l.trim_start().starts_with("qrcode = "))
            .expect("no `qrcode = ` dependency line in Cargo.toml — the pin has nothing to audit");
        assert!(
            req_line.contains(&format!("version = \"{QRCODE_PINNED_REQUIREMENT}\"")),
            "the `qrcode` requirement is no longer \"{QRCODE_PINNED_REQUIREMENT}\": {req_line}"
        );
        assert!(
            resolved.starts_with(&format!("{QRCODE_PINNED_REQUIREMENT}.")),
            "the resolved version {resolved} is not inside the pinned requirement \
             {QRCODE_PINNED_REQUIREMENT} — the two halves of the pin disagree"
        );

        // (c) `default-features = false` is load-bearing: the default set
        //     drags `image` in, and a PNG renderer appearing would invite a
        //     second, divergent commitment surface.
        assert!(
            req_line.contains("default-features = false"),
            "qrcode must stay `default-features = false`: {req_line}"
        );
    }

    /// The shipped render params are the ones the goldens were captured
    /// under. A deliberate change edits BOTH this and the goldens; an
    /// accidental one fails here first.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn qr_render_params_are_the_pinned_ones() {
        assert_eq!(
            QR_RENDER_PARAMS,
            QrRenderParams {
                ec_level: qrcode::EcLevel::M,
                quiet_zone: true,
            }
        );
        // The discriminant rides in the preimage as a byte — pin it so a
        // reordering of the upstream enum is visible here.
        assert_eq!(QR_RENDER_PARAMS.ec_level as u8, 1, "EcLevel::M == 1");
    }

    /// The commitment golden itself (deterministic across runs and hosts),
    /// plus an INDEPENDENT recompute of the preimage layout — the
    /// "pin every reimplemented derivation" discipline. A golden that only
    /// records whatever the function emits cannot catch a preimage-layout
    /// regression; this half can.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn qr_commitment_golden() {
        let c = qr_commitment(QR_GOLDEN_URL).expect("the golden URL encodes");
        assert_eq!(hex32(&c), QR_COMMITMENT_GOLDEN);

        // Independent preimage recompute (built here, not via the fn).
        {
            use qrcode::{Color, QrCode};
            let code =
                QrCode::with_error_correction_level(QR_GOLDEN_URL, QR_RENDER_PARAMS.ec_level)
                    .expect("encodes");
            let mut pre: Vec<u8> = Vec::new();
            pre.extend_from_slice(b"riir-kat-qr-v1:");
            pre.push(u8::try_from(QRCODE_PINNED_VERSION.len()).unwrap());
            pre.extend_from_slice(QRCODE_PINNED_VERSION.as_bytes());
            pre.push(1); // EcLevel::M
            pre.push(1); // quiet_zone = true
            pre.extend_from_slice(&u32::try_from(code.width()).unwrap().to_le_bytes());
            pre.extend(
                code.to_colors()
                    .into_iter()
                    .map(|c| u8::from(c == Color::Dark)),
            );
            assert_eq!(
                blake3::hash(&pre).as_bytes(),
                &c,
                "the preimage layout drifted from the documented one"
            );
        }
        // Determinism: same input, same digest, twice.
        assert_eq!(qr_commitment(QR_GOLDEN_URL), Some(c));
        // Content sensitivity: one flipped hex nibble in the request id
        // moves the commitment (the asset is bound to the request).
        let other = QR_GOLDEN_URL.replace("71b5", "71b6");
        assert_ne!(qr_commitment(&other), Some(c));
    }

    /// Captured 2026-09-09 against [`QR_GOLDEN_URL`] (the CURRENT
    /// [`DEFAULT_SERVICE_URL`] host) — blake3 of [`qr_commitment`]'s
    /// canonical matrix serialization under [`QR_RENDER_PARAMS`].
    ///
    /// Provenance: `dc9e179` (2026-09-02) captured these pins against the
    /// then-current `kat.heal.gist.rs` host; `f7cbe31` flipped the host to
    /// `heal.gist.rs` but its "QR golden … follow" claim never landed — the
    /// pins kept encoding the retired host's QR and both golden tests ran
    /// red until issue 078 re-derived them. `qr_golden_url_tracks_the_default_service_url`
    /// is the guard that fails at the URL line next time.
    #[cfg(feature = "kat_transport")]
    const QR_COMMITMENT_GOLDEN: &str =
        "4332593b3c38be168b11b8cd12d9737d0bdeb808b151360f689edff7828c68c5";

    /// Axis 1 (STRUCTURAL), and the anti-vacuity half: the commitment must
    /// MOVE on every pinned axis. An in-preimage field that no test varies
    /// is an assumption, not a pin — so vary all of them and require
    /// divergence.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn qr_commitment_moves_on_every_pinned_axis() {
        use qrcode::EcLevel;
        let base = qr_commitment(QR_GOLDEN_URL).expect("encodes");

        // (i) ECC level — all three alternatives diverge.
        for level in [EcLevel::L, EcLevel::Q, EcLevel::H] {
            let c = qr_commitment_with(
                QR_GOLDEN_URL,
                QrRenderParams {
                    ec_level: level,
                    quiet_zone: QR_RENDER_PARAMS.quiet_zone,
                },
            )
            .expect("encodes");
            assert_ne!(
                c, base,
                "the commitment did NOT move for EcLevel::{level:?} — the ECC axis of the \
                 caveat-#3 pin is vacuous"
            );
        }

        // (ii) quiet zone.
        let c = qr_commitment_with(
            QR_GOLDEN_URL,
            QrRenderParams {
                ec_level: QR_RENDER_PARAMS.ec_level,
                quiet_zone: !QR_RENDER_PARAMS.quiet_zone,
            },
        )
        .expect("encodes");
        assert_ne!(
            c, base,
            "the commitment did NOT move on the quiet zone — that axis of the pin is vacuous"
        );

        // (iii) the crate version. This is the axis a golden digest alone
        //       can never cover: an upstream bump that happens to produce
        //       the same matrix would leave a matrix-only digest green
        //       while every stored asset's provenance had changed.
        for v in ["0.14.2", "0.15.0", ""] {
            let c = qr_commitment_stamped(QR_GOLDEN_URL, QR_RENDER_PARAMS, v).expect("encodes");
            assert_ne!(
                c, base,
                "the commitment did NOT move for a {v:?} crate-version stamp — the version \
                 axis of the caveat-#3 pin is vacuous"
            );
        }

        // And the stamped form at the PINNED version is the shipped one
        // (so the three-arg fn is the same function, not a parallel path).
        assert_eq!(
            qr_commitment_stamped(QR_GOLDEN_URL, QR_RENDER_PARAMS, QRCODE_PINNED_VERSION),
            Some(base)
        );
    }

    /// The param lift is behaviour-preserving: rendering through
    /// [`QR_RENDER_PARAMS`] is byte-identical to the pre-T1.4 call shape
    /// (`QrCode::new` + `quiet_zone(true)`), so no stored asset moved when
    /// the params became explicit.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn render_qr_ascii_is_byte_identical_to_the_pre_t14_call_shape() {
        use qrcode::render::unicode::Dense1x2;
        use qrcode::QrCode;
        let before = QrCode::new(QR_GOLDEN_URL)
            .map(|c| c.render::<Dense1x2>().quiet_zone(true).build())
            .expect("encodes");
        assert_eq!(render_qr_ascii(QR_GOLDEN_URL), before);
        assert_eq!(
            hex32(blake3::hash(before.as_bytes()).as_bytes()),
            QR_ASCII_GOLDEN
        );
    }

    /// Re-derived 2026-09-09 against [`QR_GOLDEN_URL`] (the CURRENT
    /// [`DEFAULT_SERVICE_URL`] host) — blake3 of the ASCII render of
    /// [`QR_GOLDEN_URL`] under [`QR_RENDER_PARAMS`]. Same provenance as
    /// [`QR_COMMITMENT_GOLDEN`] below: the original pins encoded the
    /// pre-`f7cbe31` host's render.
    #[cfg(feature = "kat_transport")]
    const QR_ASCII_GOLDEN: &str =
        "a14d9f506952767acc65a2b4c8eda51f53d90eb7ffdcb89053d46bb0593d789d";

    /// Provenance tie (issue 078): the two goldens above are only meaningful
    /// for the URL the service actually issues. This fails at the URL line —
    /// with the re-derive instruction — when [`DEFAULT_SERVICE_URL`] moves,
    /// instead of letting two digest mismatches be the first signal (the
    /// exact gap that let the `f7cbe31` host flip run red for a week).
    #[cfg(feature = "kat_transport")]
    #[test]
    fn fallback_pair_covers_both_known_hosts_and_never_custom() {
        assert_eq!(
            fallback_pair_for(DEFAULT_SERVICE_URL).as_deref(),
            Some(FALLBACK_SERVICE_URL)
        );
        assert_eq!(
            fallback_pair_for(FALLBACK_SERVICE_URL).as_deref(),
            Some(DEFAULT_SERVICE_URL)
        );
        // An operator override / test-double URL is respected literally.
        assert_eq!(fallback_pair_for("https://example.test"), None);
        // Trailing slash normalized.
        assert_eq!(
            fallback_pair_for("https://ai.gist.rs/").as_deref(),
            Some(FALLBACK_SERVICE_URL)
        );
    }

    /// Plan 023 S2 (re-derived 2026-09-10): the failover pair is the DEVNET
    /// subdomain — a DEV client must never fail over onto MAINNET, and the
    /// retired `heal.gist.rs` (no DNS) must not come back here. If this
    /// fails, the pair moved: re-read Plan 023 S2 and move the pin in the
    /// SAME commit (the issue-078 class — URL pins move consciously, or the
    /// digests lie).
    #[test]
    fn fallback_pair_is_the_devnet_subdomain() {
        assert_eq!(FALLBACK_SERVICE_URL, "https://devnet.ai.gist.rs");
        assert_eq!(
            FALLBACK_SERVICE_URL,
            crate::kat_service_config::DEVNET_SERVICE_URL
        );
    }

    /// The const-never-moves premise's GATED half (Plan 023): the baked
    /// default IS the pinned mainnet host — the ungated `kat_service_config`
    /// module passes its `MAINNET_SERVICE_URL` const as the default mirror,
    /// and together the two pins keep the QR goldens from ever tripping at
    /// the domain swap.
    #[test]
    fn default_service_url_is_the_pinned_mainnet_host() {
        assert_eq!(
            DEFAULT_SERVICE_URL,
            crate::kat_service_config::MAINNET_SERVICE_URL
        );
    }

    #[cfg(feature = "kat_transport")]
    #[test]
    fn host_level_kinds_are_dns_and_connect_only() {
        assert!(is_host_level_kind(ureq::ErrorKind::Dns));
        assert!(is_host_level_kind(ureq::ErrorKind::ConnectionFailed));
        // An IO timeout is NOT host-level (the push may have landed).
        assert!(!is_host_level_kind(ureq::ErrorKind::Io));
        assert!(!is_host_level_kind(ureq::ErrorKind::InvalidUrl));
    }

    #[cfg(feature = "kat_transport")]
    #[test]
    fn failover_flips_once_to_the_pair_and_request_base_follows() {
        // Process-global memo: this is the only test that flips it, and the
        // assertions tolerate either host being the first writer.
        if let Some(next) = failover_flip(DEFAULT_SERVICE_URL) {
            assert_eq!(next, FALLBACK_SERVICE_URL);
        } else {
            assert!(FAILED_OVER.get().is_some());
        }
        // One-shot: every later flip is None, either direction.
        assert_eq!(failover_flip(FALLBACK_SERVICE_URL), None);
        assert_eq!(failover_flip(DEFAULT_SERVICE_URL), None);
        // And request_base now answers the memo for ANY client base.
        assert_eq!(
            request_base("https://unrelated.test"),
            FAILED_OVER.get().unwrap().as_str()
        );
    }

    #[cfg(feature = "kat_transport")]
    #[test]
    fn qr_golden_url_tracks_the_default_service_url() {
        assert_eq!(
            QR_GOLDEN_URL,
            format!("{DEFAULT_SERVICE_URL}/payreq/71b5cbf13626d2554aba7d9b06e63474"),
            "QR_GOLDEN_URL no longer tracks DEFAULT_SERVICE_URL — QR_COMMITMENT_GOLDEN \
             and QR_ASCII_GOLDEN were captured against the golden URL: re-derive BOTH \
             at the new host in the same commit as the flip"
        );
    }

    /// The ASCII render digest moves on the render params too — same
    /// anti-vacuity discipline applied to the surface a user actually sees.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn render_qr_ascii_digest_moves_on_the_render_params() {
        use qrcode::EcLevel;
        let base = render_qr_ascii(QR_GOLDEN_URL);
        assert!(!base.is_empty());
        let no_quiet = render_qr_ascii_with(
            QR_GOLDEN_URL,
            QrRenderParams {
                ec_level: QR_RENDER_PARAMS.ec_level,
                quiet_zone: false,
            },
        );
        assert_ne!(no_quiet, base, "the quiet zone must change the render");
        let ec_h = render_qr_ascii_with(
            QR_GOLDEN_URL,
            QrRenderParams {
                ec_level: EcLevel::H,
                quiet_zone: QR_RENDER_PARAMS.quiet_zone,
            },
        );
        assert_ne!(ec_h, base, "the ECC level must change the render");
        // An unencodable payload stays the never-a-gate empty string.
        assert!(render_qr_ascii(&"x".repeat(8000)).is_empty());
        assert_eq!(qr_commitment(&"x".repeat(8000)), None);
    }
}

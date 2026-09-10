//! Wire types for lease — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).
//!
//! Extracted verbatim from [`crate::kat_lease_client`] (which re-exports
//! everything below, so every existing path is unchanged): the §1 wire
//! mirror, the §3 corpus-pack contract, the on-disk pointer shape, and the
//! claim frame. Only pure items moved — the store, the charge ledger, the
//! reheat planner, the transport, the URL seam, and the QR asset stayed in
//! the client module.
//!
//! ## The wire contract is a MIRROR, not a dependency
//!
//! riir-clippy may NOT depend on riir-dapps (the boundary allowlist). The
//! lease wire format is a **versioned serialization contract**: this module
//! mirrors riir-dapps `src/kat/lease.rs` BYTE-FOR-BYTE — the domain
//! separator [`MANIFEST_DOMAIN`], [`LeaseManifest::canonical_body`]
//! (fixed-width LE, fixed field order), `to_bytes`, `decode`, and the
//! commitment `BLAKE3(domain ‖ canonical body)` — pinned by **golden
//! vectors derived from the dapps side** (see the test module). A drift on
//! either side breaks the pinned vectors; that IS the drift detector (the
//! C# `WireProtocol` golden-fixture precedent, riir-unity Issue 003).

// The wire format is MIRRORED from dapps byte-for-byte + golden-pinned: its
// `usize as u32` field encodings are the contract (a >4 GiB corpus segment
// is not a real shape — the format encodes len as u32 on BOTH sides), so
// the truncation lint is answered here once, not per site.
#![allow(unknown_lints)]
#![allow(clippy::cast_possible_truncation)]

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

/// This client's compat-matrix version (bumped when the lease consumer's
/// semantics change in a way a manifest must gate on).
pub const LEASE_CLIENT_VERSION: u32 = 1;

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

// ── the on-disk pointer (the pointer-file wire shape) ──────────────────────

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

// ── the claim frame (the payment-claim wire payload) ───────────────────────

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

    #[test]
    fn hex_round_trips() {
        let id = LeaseSegmentDesc::of(b"roundtrip").id;
        assert_eq!(unhex32(&hex32(&id)), Some(id));
        assert_eq!(unhex32("zz"), None);
        assert_eq!(unhex32(&"0".repeat(63)), None);
    }
}

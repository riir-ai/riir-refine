//! KAT sync payload schema — Proposal 005 Phase 0 T0.3 (Plan 027).
//!
//! The **versioned serialization contract** for shipping [`TrajStore`] rows to
//! the KAT service (the proposal's words: the Worker consumes it *as data*).
//! NDJSON — one JSON object per line — matching the `.refine/` jsonl store
//! convention: streamable, debuggable, append-friendly.
//!
//! ```text
//! line 1    PayloadHeader   (versions, origin, counts, batch_commitment)
//! line 2..  PayloadRow      (one per GATE-PASSED row, sorted by (lint_key, shape_id))
//! ```
//!
//! ## The load-bearing rule: no row is serialized without passing the gate
//!
//! Every text field of every row crosses the wire only through
//! [`RedactionGate::check`](crate::kat_redact::RedactionGate::check) — `shape`,
//! `fix`, `lint_key`, `last_run_id`, each gated independently. A `Blocked`
//! verdict on ANY field excludes the whole row (a half-row on the server is
//! worse than no row); with `allow_redacted: true` each field redacts in place
//! and the row's [`RowGateMeta`] records the finding **fingerprints** (kind +
//! which field + `blake3(finding)[..12]` — never bytes, never byte ranges;
//! ranges are meaningless across fields). This is the consumer T0.2's gate was
//! built waiting for.
//!
//! ## The batch commitment + the account signature (Issue 008)
//!
//! [`PayloadHeader::batch_commitment`] = `blake3` (full 64-hex) over the exact
//! row-line bytes (each line + `\n`). Same rows + same policy + same
//! `created_unix` → byte-identical payload (rows sort by `(lint_key,
//! shape_id)` — the store's row identity, one derivation). The commitment
//! cannot cover its own header — the SIGNATURE does: every payload is signed
//! by the KAT account key (the `login` Ed25519 keypair) over the canonical
//! [`PayloadHeader::signing_message`] (domain ‖ every header scalar ‖ the
//! commitment). The receiver verifies the signature BEFORE its
//! `redact_version` floor, so the floor gates an ATTESTED version, not a
//! claimed one, and the sync submitter is DERIVED from the signed pubkey —
//! never trusted from the wire. The v1 (unsigned) schema is refused outright:
//! a hard cut, no deployed client fleet to migrate.
//!
//! ## Versions (G9b)
//!
//! [`PAYLOAD_VERSION`] versions the SCHEMA; the header additionally stamps
//! `redact_version` from the compiled [`REDACT_VERSION`] so the receiver knows
//! which gate definition produced the verdicts. The two move independently by
//! design (a detection-rule change is not a wire-shape change), and the
//! alignment rule is: the builder stamps the constant, input is never trusted.
//!
//! ## The training-source designation (Issue 148 / Proposal 021 §3, 2026-10-05)
//!
//! The gate-passed rows this module ships are the DESIGNATED training corpus
//! for any future refine-rule suite mint (the artifact that would un-refute
//! the decision-scorer class, Bench 098's negative prior). Designation, not
//! new behavior: the lane already produces exactly what a mint consumes —
//! `shape → fix → lint_key`, redaction-gated, BLAKE3-committed, signed,
//! mining-consented. The corpus-side kill that closed the Decision-CPI tier
//! (Issue 147's T0′: 533 credited fleet contexts = 19× off the 10⁴ target,
//! converged since 2026-09-16/18) is the recorded bar for any re-open: growth
//! needs NEW contexts (machines/repos/rules — never-pushed local stores, new
//! corpus batches, fleet growth), all of which already flow through the
//! per-repo `.refine-sync-policy` + mining-consent mechanism — no new
//! consent surface is owed. decstat/fixstat CANNOT carry contexts (their
//! stats-only redaction law); this lane is the one carrier. Any label-field
//! additions (e.g. the rejected candidate set per span) are `PAYLOAD_VERSION`
//! decisions — see [`PAYLOAD_CHANGELOG`] + the receiver's floor posture.
//!
//! ## Honest scope
//!
//! - The builder is the SANCTIONED egress path: a valid payload (commitment
//!   over gated rows + the account-key signature) is only producible via
//!   [`SyncPayloadBuilder`]. The wire types are `pub` for the consumer side
//!   (the server parses them as data) and tests. Since v2 the signature makes
//!   a hand-written payload attributable rather than anonymous — a hostile
//!   local process can still sign garbage WITH ITS OWN KEY, but it can no
//!   longer claim an arbitrary gate version under someone else's identity
//!   (the receiver credits/attests the key-holder; governance can Ban it).
//! - Blocked rows appear only as the header's `rows_blocked` count + the
//!   builder's [`RowOutcome`]s (for caller-side [`RedactionLog`]
//!   recording — the builder stays pure; logging policy is the caller's, the
//!   `fixer_regate` precedent). Blocked details never cross the wire.
//! - No transport / retry / resume cursor (Phase 1 T1.6). The payload is a
//!   snapshot batch; `shape_id` is the upsert key a delta design will need.
//! - Scanner limits are inherited from T0.2 (line-based; no layer-stripping;
//!   committed hash literals trip the entropy rule fail-closed). Real run ids
//!   (`run-{hex≤16}-{pid}`) are entropy-safe by length. A secret split across
//!   two lines within ONE field inherits the scanner's limit; across fields it
//!   cannot reassemble in serialized form (JSON structure interleaves).
//! - Re-scanning a REDACTED payload may re-flag a placeholder site (T0.2's
//!   terminal-state rule): assert bytes-gone, not zero findings.

use crate::kat_redact::{
    BlockReason, Finding, GateDecision, RedactionGate, SpanOrigin, REDACT_VERSION,
};
use crate::traj_store::{shape_hash, TrajRowDoc};
use ed25519_dalek::{Signer as _, Verifier as _};

/// The payload SCHEMA version (G9b). Bump ONLY when the wire shape changes
/// (field added/removed/renamed, gating scope, commitment domain), and record
/// the delta in [`PAYLOAD_CHANGELOG`] in the same change.
pub const PAYLOAD_VERSION: u32 = 3;

/// Versioned deltas of the payload schema (G9b — "never a silent re-shape").
pub const PAYLOAD_CHANGELOG: &[(u32, &str)] = &[
    (
        1,
        "initial definition: NDJSON envelope (PayloadHeader + PayloadRow lines) \
         over TrajRowDoc; per-field gate check on shape/fix/lint_key/last_run_id \
         (first block excludes the row); gate fingerprints (kind + field + \
         blake3_12) in per-row metadata; rows sorted by (lint_key, shape_id) \
         with shape_id = the store's shape_hash; full-64-hex BLAKE3 batch \
         commitment over the exact row-line bytes; blocked rows cross as a \
         count only.",
    ),
    (
        2,
        "the envelope is SIGNED (Issue 008): the header gains account_pubkey_hex \
         + signature_hex — Ed25519 by the kat account key over the canonical \
         signing message (SYNC_SIGNING_DOMAIN ‖ payload_version ‖ \
         redact_version ‖ created_unix ‖ row counts ‖ origin ‖ the batch \
         commitment). The receiver verifies the signature BEFORE the \
         redact_version floor, so MIN_REDACT_VERSION gates an ATTESTED \
         version, not a claimed one; the sync submitter is derived \
         blake3(pubkey)[..16] on the receiver, never trusted from the wire. \
         v1 is refused outright (hard cut — no deployed client fleet).",
    ),
    (
        3,
        "each row gains `domain` (Issue 103): the emitting domain's id from the \
         client's COMPILED rule→domain table (clippy / rust_perf / kernel_opt / \
         sec / docker / dist by corpus membership, rustc_errors by playbook \
         code; empty string = unknown, which the receiver buckets as `other`). \
         Kills the server-side seed mapping's staleness for attributed rules — \
         a new domain's rules land in their exact slot the moment a v3 client \
         ships, instead of degrading to `other` until the worker's seed export \
         is re-run. NOT a gated field (a generated vocabulary id, never user \
         bytes). The receiver still ingests v2 (domain = absent → seed \
         mapping) — never a hard cutover; mixed fleets are the normal state. \
         SYNC_SIGNING_DOMAIN keeps its historical `-v2` spelling: old clients \
         verify against those exact bytes, so moving it WOULD be a hard cut.",
    ),
];

/// The gated text fields of a row, in check order. Every string that crosses
/// the wire passes the gate; numeric fields cannot carry secrets.
const GATED_FIELDS: [&str; 4] = ["shape", "fix", "lint_key", "last_run_id"];

// ── Wire types ─────────────────────────────────────────────────────────

/// NDJSON line 1 — batch metadata + the commitment + the account signature.
/// The commitment covers the row lines (never the header); the Ed25519
/// signature covers the header itself (via [`Self::signing_message`]) —
/// together the whole document is tamper-evident and attributable.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadHeader {
    /// The schema version ([`PAYLOAD_VERSION`] at build).
    pub payload_version: u32,
    /// The gate definition that produced the verdicts ([`REDACT_VERSION`] at
    /// build — stamped from the compiled constant, never trusted from input).
    /// **Signed** — the receiver's floor gates this ATTESTED value.
    pub redact_version: u32,
    /// The origin every row was gated under (policy spelling). Signed.
    pub origin: String,
    /// Builder-stamped creation time (unix seconds; deterministic via
    /// [`SyncPayloadBuilder::build_at`]). Signed — a backdated stamp no longer
    /// passes undetected, though no freshness window is enforced (the batch is
    /// content-addressed + ingest-idempotent; freshness is a deferred policy).
    pub created_unix: u64,
    /// Rows that passed the gate and are present below. Signed.
    pub rows_included: u32,
    /// Rows excluded by the gate (count only — details stay in the local
    /// `.refine/redaction_log.jsonl`). Signed.
    pub rows_blocked: u32,
    /// `blake3` full 64-hex over the exact row-line bytes (each + `\n`). Signed.
    pub batch_commitment: String,
    /// The signer's Ed25519 verifying key, 64 lowercase-hex chars. The sync
    /// account id the receiver attributes this batch to is DERIVED from it
    /// (`blake3(pubkey)[..16]` — the receiver's `submitter_for`), never taken
    /// from a query param.
    pub account_pubkey_hex: String,
    /// Ed25519 over [`Self::signing_message`] by the account key, 128
    /// lowercase-hex chars. Produced only by [`SyncPayloadBuilder::build_at`].
    pub signature_hex: String,
}

/// The sync-envelope signing domain — the mirror of riir-dapps
/// `kat::mining::SYNC_SIGNING_DOMAIN` (no cargo dep between the repos; the
/// dual-pinned golden payload is the byte-level drift tripwire).
///
/// ⚠ The `-v2` spelling is HISTORICAL and load-bearing (Issue 103): it is
/// domain SEPARATION for the signature, not a schema-version marker — the
/// schema version rides the signed `payload_version` field. Old (v2) clients
/// sign with these exact bytes, so renaming it would refuse every
/// pre-v3 payload at the signature check: a hard cut, exactly what the
/// mixed-fleet rule forbids.
pub const SYNC_SIGNING_DOMAIN: &[u8] = b"riir-kat-sync-payload-v2";

impl PayloadHeader {
    /// The canonical signed message — every field a receiver gates or
    /// attributes on, in fixed little-endian layout:
    ///
    /// ```text
    /// SYNC_SIGNING_DOMAIN ‖ payload_version le32 ‖ redact_version le32 ‖
    /// created_unix le64 ‖ rows_included le32 ‖ rows_blocked le32 ‖
    /// origin_len le32 ‖ origin utf8 ‖ batch_commitment (32 raw bytes)
    /// ```
    ///
    /// The signature is deliberately OUTSIDE the message (it cannot sign
    /// itself); the row LINES are covered transitively through the commitment.
    /// `None` only when `batch_commitment` is not 64 hex chars (builder output
    /// always is; the receiver refuses the header before this matters).
    #[must_use]
    pub fn signing_message(&self) -> Option<Vec<u8>> {
        let commitment = decode_hex32(&self.batch_commitment)?;
        let mut m = Vec::with_capacity(
            SYNC_SIGNING_DOMAIN.len() + 4 + 4 + 8 + 4 + 4 + 4 + self.origin.len() + 32,
        );
        m.extend_from_slice(SYNC_SIGNING_DOMAIN);
        m.extend_from_slice(&self.payload_version.to_le_bytes());
        m.extend_from_slice(&self.redact_version.to_le_bytes());
        m.extend_from_slice(&self.created_unix.to_le_bytes());
        m.extend_from_slice(&self.rows_included.to_le_bytes());
        m.extend_from_slice(&self.rows_blocked.to_le_bytes());
        m.extend_from_slice(&(u32::try_from(self.origin.len()).unwrap_or(u32::MAX)).to_le_bytes());
        m.extend_from_slice(self.origin.as_bytes());
        m.extend_from_slice(&commitment);
        Some(m)
    }
}

/// One gate-passed trajectory row. Constructed ONLY inside
/// [`SyncPayloadBuilder`] (`from_gated`) — `pub` fields are for the consumer
/// side (the server parses rows as data) and tests.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadRow {
    /// `blake3(shape)[..12]` of the ORIGINAL (unredacted) shape — the store's
    /// own row identity (`traj_store::shape_hash`), so the server can key
    /// rows against the store's keyspace. A fingerprint, not a verifier: for
    /// redacted rows the receiver cannot recompute it (by design — the
    /// original bytes are gone).
    pub shape_id: String,
    pub lint_key: String,
    /// The emitting domain's id (v3 — Issue 103): attributed client-side from
    /// the COMPILED rule→domain table (corpus membership; `rustc_errors` by
    /// playbook code). Empty string = unknown — the receiver buckets it as
    /// `other`. NOT a gated field: this is a generated vocabulary id, never
    /// user bytes. A pre-v3 receiver ignores it; a v3 receiver treats absent
    /// (v2 rows) and empty identically.
    pub domain: String,
    /// The trigger text, gated (redacted in place when the policy allows).
    pub shape: String,
    /// The replacement text, gated.
    pub fix: String,
    /// The store row's 8-D direction (derived, numeric — never gated).
    pub direction: Vec<f32>,
    pub success_count: u32,
    pub fail_count: u32,
    pub consecutive_fails: u32,
    /// `EvidenceTier` as the store's stable u8.
    pub tier: u8,
    pub last_run_id: String,
    pub last_seen_at: u64,
    pub elo: f32,
    pub span_elo: f32,
    /// The gate's verdict metadata for THIS row.
    pub gate: RowGateMeta,
}

// ── The compiled rule→domain table (v3 — Issue 103) ─────────────────────
// Issue 134 T1: the table moved to its own UNGATED module
// (`crate::rule_domains`) so the demonstrability column and the
// score-bench lane project through the SAME table without the
// kat_service dep tree. Re-exported here — every historical path
// (`kat_sync_payload::RuleDomains`, the sync builder, the pending
// estimator, the fixstats fold) resolves unchanged.
pub use crate::rule_domains::{RuleDomains, RUSTC_ERRORS_DOMAIN};

/// Per-row gate metadata — the "gate fingerprints are recorded in payload
/// metadata" contract. Fingerprints only; never bytes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RowGateMeta {
    /// `"allow"` (zero findings across all gated fields) or `"redacted"`
    /// (≥1 finding, policy allowed redacted payloads).
    pub decision: String,
    /// Every finding that produced a placeholder, across all gated fields.
    pub findings: Vec<FindingMeta>,
}

/// One recorded finding fingerprint.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FindingMeta {
    /// Which gated field tripped: one of `shape` / `fix` / `lint_key` /
    /// `last_run_id`.
    pub field: String,
    /// The kebab kind tag (`aws-access-key`, …).
    pub kind: String,
    /// `blake3(finding_bytes)[..12]` — identifies WHAT was found without
    /// carrying it.
    pub fingerprint: String,
}

// ── The builder ─────────────────────────────────────────────────────────────

/// Builds SIGNED gated sync payloads (v3 — the schema is Issue 008's signed
/// envelope plus each row's `domain`, Issue 103). The sanctioned egress
/// path: a valid payload (commitment over gate-passed rows + the account-key
/// signature) is only producible through here.
///
/// By default the builder attributes each row's `domain` from the COMPILED
/// rule→domain table ([`RuleDomains::compiled`]) — the same table the
/// pending estimator projects through, so watermark digests agree by
/// construction. Tests needing feature-independent bytes (the cross-repo
/// golden) override with [`Self::with_rule_domains`].
///
/// ```
/// # #[cfg(all(feature = "kat_service", feature = "latent_retrieval"))] {
/// # use riir_refine::kat_redact::{RedactionGate, SpanOrigin};
/// # use riir_refine::kat_sync_payload::SyncPayloadBuilder;
/// let gate = RedactionGate::deny_all(); // or discover/load a policy
/// let signer = ed25519_dalek::SigningKey::from_bytes(&[42; 32]); // a test key
/// let builder = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer);
/// let payload = builder.build(&[]); // header-only: 0 rows, 0 blocked
/// assert!(riir_refine::kat_sync_payload::verify_payload(&payload.ndjson).is_ok());
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SyncPayloadBuilder<'g, 'k> {
    gate: &'g RedactionGate,
    origin: SpanOrigin,
    /// The account key every payload is signed with (v2 — Issue 008). The
    /// receiver derives the sync account id from its public half.
    signer: &'k ed25519_dalek::SigningKey,
    /// The row→domain attribution table (v3 — Issue 103). Defaults to the
    /// compiled table; overridden only where bytes must be feature-stable.
    domains: RuleDomains,
}

/// A built payload + its per-input-row outcomes (input order — zip with the
/// input slice to map back; the payload itself sorts rows).
#[derive(Debug, Clone)]
pub struct SyncPayload {
    /// The complete NDJSON document (header line + row lines).
    pub ndjson: String,
    /// The header's `batch_commitment` (also inside `ndjson`).
    pub commitment: String,
    pub rows_included: usize,
    pub rows_blocked: usize,
    /// One per input row, input order — for caller-side `RedactionLog`
    /// recording (the builder stays pure; logging is the caller's policy).
    pub outcomes: Vec<RowOutcome>,
}

/// What happened to one input row.
#[derive(Debug, Clone, PartialEq)]
pub enum RowOutcome {
    /// Row included; `redacted` = at least one field carries placeholders.
    Included { shape_id: String, redacted: bool },
    /// Row excluded; the FIRST blocking field's reason + that decision's
    /// findings (`shape_id` identifies the row in the local log).
    Blocked {
        shape_id: String,
        reason: BlockReason,
        findings: Vec<Finding>,
    },
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn decode_hex32(s: &str) -> Option<[u8; 32]> {
    decode_hex_n(s)
}

fn decode_hex64(s: &str) -> Option<[u8; 64]> {
    decode_hex_n(s)
}

/// Decode `N` raw bytes from `2N` lowercase/uppercase hex chars (None on any
/// bad char or wrong length).
#[allow(clippy::cast_possible_truncation)] // `hi`/`lo` are 4-bit nibbles; the OR is ≤ 0xFF by construction
fn decode_hex_n<const N: usize>(s: &str) -> Option<[u8; N]> {
    let b = s.as_bytes();
    if b.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, chunk) in b.chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out[i] = ((hi << 4) | lo) as u8;
    }
    Some(out)
}

impl<'g, 'k> SyncPayloadBuilder<'g, 'k> {
    /// A builder gating every row under `origin` through `gate`, signing the
    /// envelope with the KAT account key (v2 — Issue 008; `cargo refine login`
    /// provisions the key, [`crate::kat_account::KatAccount::signing_key`]).
    /// Rows are domain-attributed from the COMPILED table.
    #[must_use]
    pub fn new(
        gate: &'g RedactionGate,
        origin: SpanOrigin,
        signer: &'k ed25519_dalek::SigningKey,
    ) -> Self {
        Self {
            gate,
            origin,
            signer,
            domains: RuleDomains::compiled(),
        }
    }

    /// Override the row→domain table (Issue 103). Production callers stay on
    /// the compiled default; the cross-repo golden fixture uses an explicit
    /// table so its pinned bytes never move with the reader's feature flags.
    #[must_use]
    pub fn with_rule_domains(mut self, domains: RuleDomains) -> Self {
        self.domains = domains;
        self
    }

    /// Build with the current time stamp. See [`Self::build_at`] for the
    /// deterministic form (tests / rebuild-verification).
    #[must_use]
    pub fn build(&self, rows: &[TrajRowDoc]) -> SyncPayload {
        self.build_at(rows, now_unix())
    }

    /// Build with an explicit creation stamp — the deterministic form: same
    /// rows + same policy + same `created_unix` → byte-identical `ndjson`
    /// (rows sort by `(lint_key, shape_id)`; JSON field order is struct
    /// order; floats round-trip via serde's shortest form).
    ///
    /// # Panics
    ///
    /// Only if a [`PayloadRow`] or [`PayloadHeader`] fails to serialize —
    /// unreachable for these field types (strings/ints/floats; serde's
    /// infallible shapes), pinned by the round-trip tests.
    #[must_use]
    pub fn build_at(&self, rows: &[TrajRowDoc], created_unix: u64) -> SyncPayload {
        let mut outcomes = Vec::with_capacity(rows.len());
        let mut included: Vec<PayloadRow> = Vec::new();
        for row in rows {
            let shape_id = shape_hash(&row.shape);
            match gate_row(self.gate, self.origin, row, &self.domains) {
                Ok(pr) => {
                    let redacted = pr.gate.decision == "redacted";
                    outcomes.push(RowOutcome::Included { shape_id, redacted });
                    included.push(pr);
                }
                Err((reason, findings)) => {
                    outcomes.push(RowOutcome::Blocked {
                        shape_id,
                        reason,
                        findings,
                    });
                }
            }
        }
        // Deterministic order for the commitment (input order is preserved
        // only in `outcomes`).
        included.sort_by(|a, b| (&a.lint_key, &a.shape_id).cmp(&(&b.lint_key, &b.shape_id)));

        let mut row_bytes = String::new();
        for r in &included {
            // serde_json::to_string is infallible for these field types.
            let line = serde_json::to_string(r).expect("payload row serializes");
            row_bytes.push_str(&line);
            row_bytes.push('\n');
        }
        let commitment = blake3_hex(row_bytes.as_bytes());
        let blocked = rows.len() - included.len();
        // Row counts saturate rather than truncate (a >4G-row batch is
        // already absurd; the header must never undercount).
        let rows_included = u32::try_from(included.len()).unwrap_or(u32::MAX);
        let rows_blocked = u32::try_from(blocked).unwrap_or(u32::MAX);
        let pubkey = self.signer.verifying_key().to_bytes();
        let mut header = PayloadHeader {
            payload_version: PAYLOAD_VERSION,
            redact_version: REDACT_VERSION,
            origin: self.origin.as_str().to_string(),
            created_unix,
            rows_included,
            rows_blocked,
            batch_commitment: commitment.clone(),
            account_pubkey_hex: hex_encode(&pubkey),
            signature_hex: String::new(),
        };
        // The signing message reads only fields already set — the signature
        // itself is outside it.
        let msg = header
            .signing_message()
            .expect("builder commitment is canonical 64-hex");
        header.signature_hex = hex_encode(&self.signer.sign(&msg).to_bytes());
        let head = serde_json::to_string(&header).expect("header serializes");
        let mut ndjson = String::with_capacity(row_bytes.len() + head.len() + 1);
        ndjson.push_str(&head);
        ndjson.push('\n');
        ndjson.push_str(&row_bytes);
        SyncPayload {
            ndjson,
            commitment,
            rows_included: included.len(),
            rows_blocked: blocked,
            outcomes,
        }
    }
}

/// The one gated-row projection (free fn so the pending-record estimator can
/// project rows without constructing a signed builder — signing is envelope
/// level, not row level; same code path as [`SyncPayloadBuilder::build_at`],
/// no second projection to drift). The row's `domain` comes from `domains`
/// (the caller's table — the builder passes its own; the estimator passes
/// the compiled one) and is NOT gated: a generated vocabulary id, never
/// user bytes.
///
/// ## The source path (riir-dapps Issue 119 T2)
///
/// Every field is checked WITH the row's [`TrajRowDoc::src_path`], so the
/// policy's `allow_paths` actually binds (the `--mine` promise: snippets
/// only from the allowed source globs). A row with NO recorded path (legacy
/// rows, path-less writers) FAILS CLOSED whenever `allow_paths` is
/// non-empty — blocked as [`BlockReason::PathNotAllowed`]`("")`, the same
/// verdict an out-of-allowlist path gets; it syncs once a heal run
/// re-records it with its file. An empty `allow_paths` means the policy
/// does not restrict paths, so the unknown path passes there. The path is
/// a gate INPUT only — never copied into the [`PayloadRow`].
pub(crate) fn gate_row(
    gate: &RedactionGate,
    origin: SpanOrigin,
    row: &TrajRowDoc,
    domains: &RuleDomains,
) -> Result<PayloadRow, (BlockReason, Vec<Finding>)> {
    let inputs = [
        (GATED_FIELDS[0], row.shape.as_str()),
        (GATED_FIELDS[1], row.fix.as_str()),
        (GATED_FIELDS[2], row.lint_key.as_str()),
        (GATED_FIELDS[3], row.last_run_id.as_str()),
    ];
    let mut texts: [String; GATED_FIELDS.len()] =
        [String::new(), String::new(), String::new(), String::new()];
    let mut findings: Vec<FindingMeta> = Vec::new();
    let path = (!row.src_path.is_empty()).then_some(row.src_path.as_str());
    for (i, (field, text)) in inputs.iter().enumerate() {
        match gate.check(origin, path, text) {
            GateDecision::Allow => texts[i] = (*text).to_string(),
            GateDecision::Redacted {
                payload,
                findings: fs,
            } => {
                for f in fs {
                    findings.push(FindingMeta {
                        field: (*field).to_string(),
                        kind: f.kind.tag().to_string(),
                        fingerprint: f.fingerprint.clone(),
                    });
                }
                texts[i] = payload;
            }
            GateDecision::Blocked {
                reason,
                findings: fs,
            } => {
                return Err((reason, fs));
            }
        }
    }
    // Fail closed on the unknown path (see the doc above). Checked after the
    // field loop so the gate's own earlier verdicts (sync disabled, origin)
    // keep their reasons; nothing has been emitted yet either way.
    if path.is_none() && !gate.policy().allow_paths.is_empty() {
        return Err((BlockReason::PathNotAllowed(String::new()), Vec::new()));
    }
    let [shape, fix, lint_key, last_run_id] = texts;
    let decision = if findings.is_empty() {
        "allow"
    } else {
        "redacted"
    };
    Ok(PayloadRow {
        shape_id: shape_hash(&row.shape),
        lint_key,
        domain: domains.domain_for(row.lint_key.as_str()).to_string(),
        shape,
        fix,
        direction: row.direction.clone(),
        success_count: row.success_count,
        fail_count: row.fail_count,
        consecutive_fails: row.consecutive_fails,
        tier: row.tier,
        last_run_id,
        last_seen_at: row.last_seen_at,
        elo: row.elo,
        span_elo: row.span_elo,
        gate: RowGateMeta {
            decision: decision.to_string(),
            findings,
        },
    })
}

// ── Verification + parsing (the consumer side) ─────────────────────────

/// Parse the header + verify the Ed25519 account signature over the canonical
/// message, then the BLAKE3 batch commitment over the row lines. The mirror
/// of riir-dapps `verify_sync_payload`'s ordering: schema version → signature
/// → (the receiver then gates the ATTESTED `redact_version`) → commitment.
///
/// # Errors
///
/// `Err(reason)` when there is no header line, the header does not decode,
/// the schema version is not [`PAYLOAD_VERSION`], the account pubkey or
/// signature hex is malformed, the signature does not verify against the
/// header's pubkey, or the recomputed commitment disagrees with the header's
/// (tampered rows / truncated batch).
pub fn verify_payload(ndjson: &str) -> Result<PayloadHeader, String> {
    let (head, rest) = ndjson
        .split_once('\n')
        .ok_or("payload has no header line")?;
    let header: PayloadHeader =
        serde_json::from_str(head).map_err(|e| format!("unparseable header: {e}"))?;
    // v3 (Issue 103) is the current schema; v2 (the signed envelope without
    // per-row domains) stays accepted for the fleet's transition window — the
    // receiver-side mirror ingests BOTH. Never a hard cutover.
    if header.payload_version != PAYLOAD_VERSION && header.payload_version != PAYLOAD_VERSION - 1 {
        return Err(format!(
            "payload version {} (this build defines {PAYLOAD_VERSION})",
            header.payload_version
        ));
    }
    // Issue 008: the signature binds every header scalar + the batch
    // commitment to the account key — verify it before anything downstream
    // consults the (attested) values.
    let pubkey = decode_hex32(&header.account_pubkey_hex)
        .ok_or("account_pubkey_hex must be 64 hex chars")?;
    let sig = decode_hex64(&header.signature_hex).ok_or("signature_hex must be 128 hex chars")?;
    let msg = header
        .signing_message()
        .ok_or("batch_commitment must be 64 hex chars")?;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&pubkey)
        .map_err(|_| "account_pubkey_hex is not a valid Ed25519 verifying key")?;
    if vk
        .verify(&msg, &ed25519_dalek::Signature::from_bytes(&sig))
        .is_err()
    {
        return Err("payload signature does not verify against account_pubkey_hex".into());
    }
    let computed = blake3_hex(rest.as_bytes());
    if computed != header.batch_commitment {
        return Err(format!(
            "batch commitment mismatch: header {} vs computed {computed}",
            header.batch_commitment
        ));
    }
    Ok(header)
}

/// Parse the row lines (skipping the header). No commitment check — pair with
/// [`verify_payload`] when the batch must be authenticated.
///
/// # Errors
///
/// `Err(line_no, err)` on the first row line that does not decode.
pub fn parse_rows(ndjson: &str) -> Result<Vec<PayloadRow>, (usize, String)> {
    let mut rows = Vec::new();
    for (i, line) in ndjson.split('\n').skip(1).enumerate() {
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok(r) => rows.push(r),
            Err(e) => return Err((i + 2, format!("row line {}: {e}", i + 2))),
        }
    }
    Ok(rows)
}

// ── Tests ──────────────────────────────────────────────────────────────────

// ── Tests ──────────────────────────────────────────────────────────────

/// A throwaway signing key for a LOCAL export (`cargo refine sync --export`,
/// riir-dapps Issue 112 T4). The exported batch is the push's exact shape and
/// verifies under [`verify_payload`], but no one holds this key once the
/// process exits — the file attributes to no account a person can claim, and
/// producing it needs no login. Seeded from the std hasher's per-thread OS
/// randomness plus the clock, mixed through blake3: the key only has to be
/// unguessable, never recoverable.
#[must_use]
pub fn ephemeral_signer() -> ed25519_dalek::SigningKey {
    use std::hash::BuildHasher as _;
    let mut h = blake3::Hasher::new_derive_key("riir-refine sync-export ephemeral key v1");
    let state = std::collections::hash_map::RandomState::new();
    for i in 0u64..4 {
        h.update(&state.hash_one(i).to_le_bytes());
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    h.update(&nanos.to_le_bytes());
    h.update(&std::process::id().to_le_bytes());
    ed25519_dalek::SigningKey::from_bytes(h.finalize().as_bytes())
}

/// The cross-repo GOLDEN signing seed — the deterministic test key both
/// repos' fixture payloads are signed with (the `[42; 32]` answer-seed;
/// distinct from every real-key convention in the worker shell). NOT for
/// production. Mirrored in riir-dapps' `tests/kat_mining.rs` + the worker
/// shell fixtures; shared with `kat_pending_estimator`'s tests.
#[cfg(test)]
pub(crate) const GOLDEN_SIGNING_SEED: [u8; 32] = [42u8; 32];

/// The golden seed's signing key.
#[cfg(test)]
pub(crate) fn golden_signer() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&GOLDEN_SIGNING_SEED)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kat_redact::SyncPolicy;

    fn allow_policy() -> SyncPolicy {
        SyncPolicy {
            version: REDACT_VERSION,
            sync_enabled: true,
            allow_origins: vec!["own-repo".into()],
            allow_paths: Vec::new(),
            max_span_bytes: 65_536,
            allow_redacted: false,
        }
    }

    fn row(lint_key: &str, shape: &str, fix: &str) -> TrajRowDoc {
        TrajRowDoc {
            lint_key: lint_key.into(),
            shape: shape.into(),
            fix: fix.into(),
            direction: vec![0.1, -0.2, 0.3, 0.0, 0.5, -0.5, 0.7, 0.9],
            success_count: 3,
            fail_count: 1,
            consecutive_fails: 0,
            tier: 0,
            last_run_id: "run-abc123-42".into(),
            last_seen_at: 1_700_000_000,
            elo: 1215.5,
            span_elo: 1188.25,
            component: None,
            journey_runs: 0,
            journey_taint_sum: 0,
            src_path: "src/lib.rs".into(),
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identity IS the field-parity gate here
    fn clean_rows_round_trip() {
        let gate = RedactionGate::from_policy(allow_policy());
        let signer = golden_signer();
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer);
        let rows = vec![
            row(
                "uninlined_format_args",
                "fn f() { println!(\"{}\", x); }",
                "fn f() { println!(\"{x}\"); }",
            ),
            row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
        ];
        let p = b.build_at(&rows, 1_234_567);
        assert_eq!(p.rows_included, 2);
        assert_eq!(p.rows_blocked, 0);
        let h = verify_payload(&p.ndjson).unwrap();
        assert_eq!(h.payload_version, PAYLOAD_VERSION);
        assert_eq!(h.redact_version, REDACT_VERSION);
        assert_eq!(h.rows_included, 2);
        assert_eq!(h.origin, "own-repo");
        assert_eq!(h.created_unix, 1_234_567);
        // The signature identity: the header carries the golden key's pubkey,
        // and the account id the receiver will derive from it.
        assert_eq!(
            h.account_pubkey_hex,
            hex_encode(&golden_signer().verifying_key().to_bytes())
        );
        assert_eq!(h.signature_hex.len(), 128);
        let parsed = parse_rows(&p.ndjson).unwrap();
        assert_eq!(parsed.len(), 2);
        // Field parity with the source rows (sorted by lint_key: needless_return first).
        assert_eq!(parsed[0].lint_key, "needless_return");
        assert_eq!(parsed[0].shape, "fn g() { return 1; }");
        assert_eq!(parsed[0].fix, "fn g() { 1 }");
        assert_eq!(parsed[0].direction, rows[1].direction);
        assert_eq!(parsed[0].success_count, 3);
        assert_eq!(parsed[0].tier, 0);
        assert_eq!(parsed[0].elo, 1215.5);
        assert_eq!(parsed[0].gate.decision, "allow");
        assert!(parsed[0].gate.findings.is_empty());
        assert_eq!(parsed[0].shape_id, shape_hash("fn g() { return 1; }"));
    }

    #[test]
    fn deterministic_same_stamp_same_bytes() {
        let gate = RedactionGate::from_policy(allow_policy());
        let signer = golden_signer();
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer);
        let rows = vec![
            row("a", "let x = 1;", "let x = 1;"),
            row("b", "let y = 2;", "let y = 2;"),
        ];
        let p1 = b.build_at(&rows, 99);
        let p2 = b.build_at(&rows, 99);
        assert_eq!(p1.ndjson, p2.ndjson);
        // Input order must not matter (sort by (lint_key, shape_id)).
        let mut shuffled = rows.clone();
        shuffled.reverse();
        let p3 = b.build_at(&shuffled, 99);
        assert_eq!(p3.ndjson, p1.ndjson);
    }

    #[test]
    fn commitment_detects_tampering() {
        let gate = RedactionGate::from_policy(allow_policy());
        let signer = golden_signer();
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer);
        let rows = vec![row("a", "let x = 1;", "let x = 1;")];
        let p = b.build_at(&rows, 1);
        assert!(verify_payload(&p.ndjson).is_ok());
        // Flip one byte inside the row line.
        let mut lines: Vec<&str> = p.ndjson.split('\n').collect();
        let tampered_row = lines[1].replace("success_count\":3", "success_count\":4");
        let orig = lines[1].to_string();
        lines[1] = &tampered_row;
        let bad = lines.join("\n");
        assert_ne!(verify_payload(&bad).unwrap_err(), "");
        // Restore sanity: original still verifies.
        lines[1] = &orig;
        assert!(verify_payload(&lines.join("\n")).is_ok());
    }

    /// riir-dapps Issue 119 T2: the `--mine` default policy's source globs
    /// BIND — a row's recorded file decides whether its snippet may leave.
    fn source_globs_policy() -> SyncPolicy {
        SyncPolicy {
            allow_paths: vec![
                "src/**/*.rs".into(),
                "tests/**/*.rs".into(),
                "benches/**/*.rs".into(),
                "examples/**/*.rs".into(),
            ],
            ..allow_policy()
        }
    }

    fn row_at(lint_key: &str, shape: &str, src_path: &str) -> TrajRowDoc {
        TrajRowDoc {
            src_path: src_path.into(),
            ..row(lint_key, shape, "fixed")
        }
    }

    /// riir-dapps Issue 119 T2 — (a) a path outside `allow_paths` is not in
    /// the payload, (b) a `src/` path is, (c) a legacy row with no recorded
    /// path is dropped (fail-closed) while `allow_paths` is non-empty. The
    /// pending estimator projects through the same `gate_row`, so it must
    /// count the identical split.
    #[test]
    fn allow_paths_bind_by_recorded_src_path_and_unknown_fails_closed() {
        let gate = RedactionGate::from_policy(source_globs_policy());
        let rows = vec![
            row_at("needless_return", "fn a() { return 1; }", "src/deep/a.rs"),
            row_at("needless_return", "fn b() { return 2; }", "build.rs"),
            row_at(
                "needless_return",
                "fn c() { return 3; }",
                "vendor/x/src/lib.rs",
            ),
            row_at("needless_return", "fn d() { return 4; }", ""),
        ];
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &golden_signer())
            .build_at(&rows, 1);
        assert_eq!((p.rows_included, p.rows_blocked), (1, 3));
        let parsed = parse_rows(&p.ndjson).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].shape, "fn a() { return 1; }");
        // The path is a gate input only — never a payload byte.
        assert!(!p.ndjson.contains("src/deep/a.rs"), "{}", p.ndjson);
        assert!(!p.ndjson.contains("src_path"), "{}", p.ndjson);
        let reasons: Vec<&BlockReason> = p
            .outcomes
            .iter()
            .filter_map(|o| match o {
                RowOutcome::Blocked { reason, .. } => Some(reason),
                RowOutcome::Included { .. } => None,
            })
            .collect();
        assert_eq!(
            reasons,
            [
                &BlockReason::PathNotAllowed("build.rs".into()),
                &BlockReason::PathNotAllowed("vendor/x/src/lib.rs".into()),
                &BlockReason::PathNotAllowed(String::new()),
            ]
        );
        let est = crate::kat_pending_estimator::estimate_pending(
            &rows,
            &gate,
            SpanOrigin::OwnRepo,
            &crate::kat_pending_estimator::SyncWatermark::default(),
        );
        assert_eq!((est.pending_unsynced, est.blocked), (1, 3));
    }

    /// The unknown path only fails closed against a path-RESTRICTING policy:
    /// with `allow_paths` empty the policy names no source globs, so a
    /// legacy row passes exactly as before the field existed.
    #[test]
    fn unknown_src_path_passes_when_allow_paths_is_empty() {
        let gate = RedactionGate::from_policy(allow_policy());
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &golden_signer())
            .build_at(&[row_at("needless_return", "fn d() { return 4; }", "")], 1);
        assert_eq!((p.rows_included, p.rows_blocked), (1, 0));
    }

    #[test]
    fn missing_policy_denies_all_rows() {
        let deny = RedactionGate::deny_all();
        let signer = golden_signer();
        let b = SyncPayloadBuilder::new(&deny, SpanOrigin::OwnRepo, &signer);
        let rows = vec![row("a", "let x = 1;", "let x = 1;")];
        let p = b.build_at(&rows, 1);
        assert_eq!(p.rows_included, 0);
        assert_eq!(p.rows_blocked, 1);
        match &p.outcomes[0] {
            RowOutcome::Blocked { reason, .. } => {
                assert!(matches!(reason, BlockReason::SyncDisabled));
            }
            RowOutcome::Included { .. } => panic!("deny-all must block"),
        }
        // Header-only payload still verifies (commitment over empty rows).
        let h = verify_payload(&p.ndjson).unwrap();
        assert_eq!(h.rows_blocked, 1);
        assert!(parse_rows(&p.ndjson).unwrap().is_empty());
    }

    #[test]
    fn external_origin_blocked() {
        let gate = RedactionGate::from_policy(allow_policy());
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::External, &golden_signer())
            .build_at(&[row("a", "let x = 1;", "let x = 1;")], 1);
        assert_eq!(p.rows_blocked, 1);
        match &p.outcomes[0] {
            RowOutcome::Blocked { reason, .. } => {
                assert!(matches!(reason, BlockReason::OriginNotAllowed(_)));
            }
            RowOutcome::Included { .. } => panic!("external must block"),
        }
    }

    #[test]
    fn changelog_covers_every_version() {
        for v in 1..=PAYLOAD_VERSION {
            assert!(
                PAYLOAD_CHANGELOG.iter().any(|(cv, _)| *cv == v),
                "version {v} has no changelog row"
            );
        }
    }

    #[test]
    fn version_fields_stamped_from_constants() {
        let gate = RedactionGate::from_policy(allow_policy());
        let signer = golden_signer();
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer);
        let p = b.build_at(&[row("a", "let x = 1;", "let x = 1;")], 5);
        let h = verify_payload(&p.ndjson).unwrap();
        assert_eq!(h.payload_version, PAYLOAD_VERSION);
        assert_eq!(h.redact_version, REDACT_VERSION);
    }

    #[test]
    fn shape_id_is_the_store_derivation() {
        // The pin: payload identity == store row identity (one derivation).
        assert_eq!(shape_hash("fn q() {}"), {
            let mut h = blake3::Hasher::new();
            h.update(b"fn q() {}");
            h.finalize().to_hex().to_string()[..12].to_string()
        });
    }

    // (The four rule→domain table tests moved WITH the table to
    // `crate::rule_domains` — Issue 134 T1. The builder tests below stay:
    // they exercise the payload, through the re-exported path.)

    /// v3 (Issue 103): a row built WITHOUT an override carries the COMPILED
    /// table's attribution — the same default the estimator projects, which
    /// is the estimator/payload agreement contract.
    #[cfg(feature = "clippy_lints")]
    #[test]
    fn builder_default_attributes_from_the_compiled_table() {
        let gate = RedactionGate::from_policy(allow_policy());
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &golden_signer())
            .build_at(&[row("needless_return", "fn g() { return 1; }", "fn g() { 1 }")], 1);
        let parsed = parse_rows(&p.ndjson).unwrap();
        assert_eq!(parsed[0].domain, "clippy");
        // Unknown rules cross as the empty string (the receiver's `other`).
        let p2 = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &golden_signer())
            .build_at(&[row("no_such_rule_anywhere", "fn h() {}", "fn h() {}")], 1);
        assert_eq!(parse_rows(&p2.ndjson).unwrap()[0].domain, "");
    }

    /// The CROSS-REPO GOLDEN PAYLOAD — the exact NDJSON bytes the
    /// `clean_rows_round_trip` fixture (2 rows, stamp `1_234_567`, own-repo
    /// allow policy, the golden signing seed) produces. Pinned IDENTICALLY on
    /// the consumer side: riir-dapps `tests/kat_mining.rs` (`GOLDEN_PAYLOAD`,
    /// feature `kat_mining`) parses these exact bytes as the ingest contract
    /// — the `wire_golden.bytes` precedent (a mirror nobody checks is not a
    /// mirror; serialization drift on EITHER side fails a gate here).
    ///
    /// Since v2 the SIGNATURE covers `redact_version` (unlike the commitment,
    /// which covers the row lines only) — a `REDACT_VERSION` bump re-pins
    /// this golden on BOTH sides. That re-pin is the attestation contract
    /// working as designed (Issue 008), not churn.
    ///
    /// Since v3 (Issue 103) each row also carries `domain`. The fixture pins
    /// an EXPLICIT table (both rules → `clippy`) rather than the compiled
    /// default, so the canonical bytes never move with the reader's feature
    /// flags (a `--no-default-features` build would attribute `""` and the
    /// golden would drift). The value `"clippy"` is the receiver-registry
    /// name — the same string the corpus-view slot law keys on.
    #[test]
    fn golden_payload_bytes_are_pinned() {
        let gate = RedactionGate::from_policy(allow_policy());
        let signer = golden_signer();
        let domains = RuleDomains {
            rules: [
                ("needless_return".to_string(), "clippy".to_string()),
                ("uninlined_format_args".to_string(), "clippy".to_string()),
            ]
            .into_iter()
            .collect(),
            rustc_codes: std::collections::BTreeSet::new(),
        };
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer)
            .with_rule_domains(domains);
        let rows = vec![
            row(
                "uninlined_format_args",
                "fn f() { println!(\"{}\", x); }",
                "fn f() { println!(\"{x}\"); }",
            ),
            row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
        ];
        let p = b.build_at(&rows, 1_234_567);
        assert_eq!(
            p.commitment,
            "bab3d27b5adee7427f696b8d8e66a52eb9690660b371c989d43507ecdf1dea1f"
        );
        let expected = concat!(
            // payload_version 3 (Issue 103). redact_version tracks
            // REDACT_VERSION (5 since the Issue-002 AI-intel pass: the AQ.
            // Gemini row; the v4 consolidation rode the same window). The
            // batch_commitment is over the ROW lines only; the SIGNATURE
            // covers the header — so a redact_version bump re-pins this
            // golden (coordinated, both repos, same window). Rows now carry
            // `domain` (the fixture's EXPLICIT table — both rules → clippy),
            // placed between lint_key and shape in the serde field order.
            "{\"payload_version\":3,\"redact_version\":5,\"origin\":\"own-repo\",",
            "\"created_unix\":1234567,\"rows_included\":2,\"rows_blocked\":0,",
            "\"batch_commitment\":\"bab3d27b5adee7427f696b8d8e66a52eb9690660b371c989d43507ecdf1dea1f\",",
            "\"account_pubkey_hex\":\"197f6b23e16c8532c6abc838facd5ea789be0c76b2920334039bfa8b3d368d61\",",
            "\"signature_hex\":\"fff18caad80604fb0613b9a618a22d76c34396063a4c5a617303df182f4b78d31ee87a615e2721f30af146007048d7d215ef18db5151c588ee95f021f077310d\"}\n",
            "{\"shape_id\":\"241e0429984b\",\"lint_key\":\"needless_return\",\"domain\":\"clippy\",",
            "\"shape\":\"fn g() { return 1; }\",\"fix\":\"fn g() { 1 }\",",
            "\"direction\":[0.1,-0.2,0.3,0.0,0.5,-0.5,0.7,0.9],\"success_count\":3,\"fail_count\":1,",
            "\"consecutive_fails\":0,\"tier\":0,\"last_run_id\":\"run-abc123-42\",",
            "\"last_seen_at\":1700000000,\"elo\":1215.5,\"span_elo\":1188.25,",
            "\"gate\":{\"decision\":\"allow\",\"findings\":[]}}\n",
            "{\"shape_id\":\"15ca779abbbc\",\"lint_key\":\"uninlined_format_args\",\"domain\":\"clippy\",",
            "\"shape\":\"fn f() { println!(\\\"{}\\\", x); }\",\"fix\":\"fn f() { println!(\\\"{x}\\\"); }\",",
            "\"direction\":[0.1,-0.2,0.3,0.0,0.5,-0.5,0.7,0.9],\"success_count\":3,\"fail_count\":1,",
            "\"consecutive_fails\":0,\"tier\":0,\"last_run_id\":\"run-abc123-42\",",
            "\"last_seen_at\":1700000000,\"elo\":1215.5,\"span_elo\":1188.25,",
            "\"gate\":{\"decision\":\"allow\",\"findings\":[]}}\n",
        );
        assert_eq!(p.ndjson, expected);
    }

    /// Issue 008 gates (the T3 set — mirrored receiver-side):
    /// a FORGED signature (right shape, wrong bytes) is refused, a
    /// signature made by a DIFFERENT key is refused, and a header field
    /// mutated after signing (the `redact_version` flip — the exact attack the
    /// issue closes) is refused by the signature even though the floor would
    /// have accepted the flipped value.
    #[test]
    fn forged_or_tampered_signatures_are_refused() {
        use ed25519_dalek::Signer as _;
        let gate = RedactionGate::from_policy(allow_policy());
        let signer = golden_signer();
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer)
            .build_at(&[row("a", "let x = 1;", "let x = 1;")], 1);
        assert!(verify_payload(&p.ndjson).is_ok());

        // All-zero signature bytes, right shape.
        let lines: Vec<String> = p.ndjson.split('\n').map(str::to_string).collect();
        let mut header: PayloadHeader = serde_json::from_str(&lines[0]).unwrap();
        header.signature_hex = "0".repeat(128);
        let forged_sig = format!(
            "{}\n{}",
            serde_json::to_string(&header).unwrap(),
            lines[1..].join("\n")
        );
        let err = verify_payload(&forged_sig).unwrap_err();
        assert!(err.contains("signature"), "got: {err}");

        // A DIFFERENT key's signature over the same message.
        let other = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let mut header: PayloadHeader = serde_json::from_str(&lines[0]).unwrap();
        let msg = header.signing_message().unwrap();
        header.signature_hex = hex_encode(&other.sign(&msg).to_bytes());
        let wrong_key = format!(
            "{}\n{}",
            serde_json::to_string(&header).unwrap(),
            lines[1..].join("\n")
        );
        let err = verify_payload(&wrong_key).unwrap_err();
        assert!(err.contains("signature"), "got: {err}");

        // Post-signing header mutation: claim redact_version 99 under the
        // golden key's signature — the signature refuses (this is the exact
        // hostile-client move the issue closes: the version is no longer a
        // free-text claim).
        let mut header: PayloadHeader = serde_json::from_str(&lines[0]).unwrap();
        header.redact_version = 99;
        let tampered = format!(
            "{}\n{}",
            serde_json::to_string(&header).unwrap(),
            lines[1..].join("\n")
        );
        let err = verify_payload(&tampered).unwrap_err();
        assert!(err.contains("signature"), "got: {err}");
    }

    /// The golden payload's account id, as the receiver derives it
    /// (`blake3(pubkey)[..16]` — mirrored from riir-dapps
    /// `kat::mining::submitter_for`). Pinned here so BOTH repos can assert
    /// the same submitter attribution for the golden payload.
    #[test]
    fn golden_submitter_derivation_is_pinned() {
        use std::fmt::Write as _;
        let pubkey = golden_signer().verifying_key().to_bytes();
        let account: [u8; 16] = blake3::hash(pubkey.as_slice()).as_bytes()[..16]
            .try_into()
            .unwrap();
        let mut hex = String::with_capacity(account.len() * 2);
        for b in account {
            write!(hex, "{b:02x}").expect("String writes are infallible");
        }
        // The value the dapps-side test pins as GOLDEN_SUBMITTER_HEX.
        assert_eq!(hex.len(), 32);
        assert_eq!(hex, hex_encode(&account));
    }

    /// Issue 112 T4: two ephemeral keys differ (the export is attributable to
    /// no stable identity), and a batch signed with one verifies like a push.
    #[test]
    fn ephemeral_signer_is_fresh_and_verifies() {
        let a = ephemeral_signer();
        let b = ephemeral_signer();
        assert_ne!(a.verifying_key().to_bytes(), b.verifying_key().to_bytes());
        assert_ne!(
            a.verifying_key().to_bytes(),
            golden_signer().verifying_key().to_bytes()
        );
        let gate = RedactionGate::from_policy(allow_policy());
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &a)
            .build_at(&[row("a", "let x = 1;", "let x = 1;")], 1);
        assert_eq!(p.rows_included, 1);
        assert!(verify_payload(&p.ndjson).is_ok());
    }
}

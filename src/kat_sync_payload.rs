//! KAT sync payload schema — Proposal 005 Phase 0 T0.3 (Plan 027).
//!
//! The **versioned serialization contract** for shipping [`TrajStore`] rows to
//! the KAT service (the proposal's words: the Worker consumes it *as data*).
//! NDJSON — one JSON object per line — matching the `.heal/` jsonl store
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
//! ## The batch commitment
//!
//! [`PayloadHeader::batch_commitment`] = `blake3` (full 64-hex) over the exact
//! row-line bytes (each line + `\n`). Same rows + same policy + same
//! `created_unix` → byte-identical payload (rows sort by `(lint_key,
//! shape_id)` — the store's row identity, one derivation). The header cannot
//! cover itself; end-to-end authentication is Phase 1 transport's job — this
//! commitment makes the ROW SET tamper-evident and rebuild-verifiable via
//! [`verify_payload`].
//!
//! ## Versions (G9b)
//!
//! [`PAYLOAD_VERSION`] versions the SCHEMA; the header additionally stamps
//! `redact_version` from the compiled [`REDACT_VERSION`] so the receiver knows
//! which gate definition produced the verdicts. The two move independently by
//! design (a detection-rule change is not a wire-shape change), and the
//! alignment rule is: the builder stamps the constant, input is never trusted.
//!
//! ## Honest scope
//!
//! - The builder is the SANCTIONED egress path: a valid payload (commitment
//!   over gated rows) is only producible via [`SyncPayloadBuilder`]. The wire
//!   types are `pub` for the consumer side (the server parses them as data)
//!   and tests. A hostile local process hand-writing NDJSON is not this
//!   module's threat model (T0.2's contract: accidental leakage through OUR
//!   surfaces).
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

/// The payload SCHEMA version (G9b). Bump ONLY when the wire shape changes
/// (field added/removed/renamed, gating scope, commitment domain), and record
/// the delta in [`PAYLOAD_CHANGELOG`] in the same change.
pub const PAYLOAD_VERSION: u32 = 1;

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
];

/// The gated text fields of a row, in check order. Every string that crosses
/// the wire passes the gate; numeric fields cannot carry secrets.
const GATED_FIELDS: [&str; 4] = ["shape", "fix", "lint_key", "last_run_id"];

// ── Wire types ──────────────────────────────────────────────────────────────

/// NDJSON line 1 — batch metadata + the commitment. Cannot cover itself;
/// the commitment binds the row lines only.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PayloadHeader {
    /// The schema version ([`PAYLOAD_VERSION`] at build).
    pub payload_version: u32,
    /// The gate definition that produced the verdicts ([`REDACT_VERSION`] at
    /// build — stamped from the compiled constant, never trusted from input).
    pub redact_version: u32,
    /// The origin every row was gated under (policy spelling).
    pub origin: String,
    /// Builder-stamped creation time (unix seconds; deterministic via
    /// [`SyncPayloadBuilder::build_at`]).
    pub created_unix: u64,
    /// Rows that passed the gate and are present below.
    pub rows_included: u32,
    /// Rows excluded by the gate (count only — details stay in the local
    /// `.heal/redaction_log.jsonl`).
    pub rows_blocked: u32,
    /// `blake3` full 64-hex over the exact row-line bytes (each + `\n`).
    pub batch_commitment: String,
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

/// Builds gated sync payloads. The sanctioned egress path: a valid payload
/// (commitment over gate-passed rows) is only producible through here.
///
/// ```
/// # #[cfg(all(feature = "kat_service", feature = "latent_retrieval"))] {
/// # use riir_clippy::kat_redact::{RedactionGate, SpanOrigin};
/// # use riir_clippy::kat_sync_payload::SyncPayloadBuilder;
/// let gate = RedactionGate::deny_all(); // or discover/load a policy
/// let builder = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo);
/// let payload = builder.build(&[]); // header-only: 0 rows, 0 blocked
/// assert!(riir_clippy::kat_sync_payload::verify_payload(&payload.ndjson).is_ok());
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct SyncPayloadBuilder<'g> {
    gate: &'g RedactionGate,
    origin: SpanOrigin,
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
    Blocked { shape_id: String, reason: BlockReason, findings: Vec<Finding> },
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

impl<'g> SyncPayloadBuilder<'g> {
    /// A builder gating every row under `origin` through `gate`.
    #[must_use]
    pub fn new(gate: &'g RedactionGate, origin: SpanOrigin) -> Self {
        Self { gate, origin }
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
            match self.gate_row(row) {
                Ok(pr) => {
                    let redacted = pr.gate.decision == "redacted";
                    outcomes.push(RowOutcome::Included { shape_id, redacted });
                    included.push(pr);
                }
                Err((reason, findings)) => {
                    outcomes.push(RowOutcome::Blocked { shape_id, reason, findings });
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
        let header = PayloadHeader {
            payload_version: PAYLOAD_VERSION,
            redact_version: REDACT_VERSION,
            origin: self.origin.as_str().to_string(),
            created_unix,
            rows_included,
            rows_blocked,
            batch_commitment: commitment.clone(),
        };
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

    /// Gate every text field of one row (single pass — text + metadata
    /// harvested together); first block excludes the row.
    ///
    /// `pub(crate)`: the pending-record estimator (T0.4) projects rows through
    /// THIS path so estimator/payload agreement is by construction — no second
    /// projection to drift (the one-derivation rule).
    pub(crate) fn gate_row(
        &self,
        row: &TrajRowDoc,
    ) -> Result<PayloadRow, (BlockReason, Vec<Finding>)> {
        let inputs = [
            (GATED_FIELDS[0], row.shape.as_str()),
            (GATED_FIELDS[1], row.fix.as_str()),
            (GATED_FIELDS[2], row.lint_key.as_str()),
            (GATED_FIELDS[3], row.last_run_id.as_str()),
        ];
        let mut texts: [String; GATED_FIELDS.len()] = [
            String::new(), String::new(), String::new(), String::new(),
        ];
        let mut findings: Vec<FindingMeta> = Vec::new();
        for (i, (field, text)) in inputs.iter().enumerate() {
            match self.gate.check(self.origin, None, text) {
                GateDecision::Allow => texts[i] = (*text).to_string(),
                GateDecision::Redacted { payload, findings: fs } => {
                    for f in fs {
                        findings.push(FindingMeta {
                            field: (*field).to_string(),
                            kind: f.kind.tag().to_string(),
                            fingerprint: f.fingerprint.clone(),
                        });
                    }
                    texts[i] = payload;
                }
                GateDecision::Blocked { reason, findings: fs } => {
                    return Err((reason, fs));
                }
            }
        }
        let [shape, fix, lint_key, last_run_id] = texts;
        let decision = if findings.is_empty() { "allow" } else { "redacted" };
        Ok(PayloadRow {
            shape_id: shape_hash(&row.shape),
            lint_key,
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
            gate: RowGateMeta { decision: decision.to_string(), findings },
        })
    }
}

// ── Verification + parsing (the consumer side) ─────────────────────────────

/// Parse the header + verify the BLAKE3 batch commitment over the row lines.
///
/// # Errors
///
/// `Err(reason)` when there is no header line, the header does not decode,
/// the schema version is not [`PAYLOAD_VERSION`], or the recomputed
/// commitment disagrees with the header's (tampered rows / truncated batch).
pub fn verify_payload(ndjson: &str) -> Result<PayloadHeader, String> {
    let (head, rest) = ndjson.split_once('\n').ok_or("payload has no header line")?;
    let header: PayloadHeader =
        serde_json::from_str(head).map_err(|e| format!("unparseable header: {e}"))?;
    if header.payload_version != PAYLOAD_VERSION {
        return Err(format!(
            "payload version {} (this build defines {PAYLOAD_VERSION})",
            header.payload_version
        ));
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
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // bit-identity IS the field-parity gate here
    fn clean_rows_round_trip() {
        let gate = RedactionGate::from_policy(allow_policy());
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo);
        let rows = vec![
            row("uninlined_format_args", "fn f() { println!(\"{}\", x); }", "fn f() { println!(\"{x}\"); }"),
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
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo);
        let rows = vec![row("a", "let x = 1;", "let x = 1;"), row("b", "let y = 2;", "let y = 2;")];
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
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo);
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

    #[test]
    fn missing_policy_denies_all_rows() {
        let deny = RedactionGate::deny_all();
        let b = SyncPayloadBuilder::new(&deny, SpanOrigin::OwnRepo);
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
        let p = SyncPayloadBuilder::new(&gate, SpanOrigin::External)
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
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo);
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

    /// The CROSS-REPO GOLDEN PAYLOAD — the exact NDJSON bytes the
    /// `clean_rows_round_trip` fixture (2 rows, stamp `1_234_567`, own-repo
    /// allow policy) produces. Pinned IDENTICALLY on the consumer side:
    /// riir-dapps `tests/kat_mining.rs` (`GOLDEN_PAYLOAD`, feature
    /// `kat_mining`) parses these exact bytes as the ingest contract — the
    /// `wire_golden.bytes` precedent (a mirror nobody checks is not a mirror;
    /// serialization drift on EITHER side fails a gate here).
    #[test]
    fn golden_payload_bytes_are_pinned() {
        let gate = RedactionGate::from_policy(allow_policy());
        let b = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo);
        let rows = vec![
            row("uninlined_format_args", "fn f() { println!(\"{}\", x); }", "fn f() { println!(\"{x}\"); }"),
            row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
        ];
        let p = b.build_at(&rows, 1_234_567);
        assert_eq!(p.commitment, "d6395af4a5df9460d859f3de6ebc7f1372d55e4e11625a8bf1aa41622bf0401d");
        let expected = concat!(
            // redact_version tracks REDACT_VERSION (3 since the Bench 053
            // declaration-scope bump; 2 was the Bench 052 hexish rule). The
            // batch_commitment below is over the ROW lines only, so it is
            // unchanged by a header-field bump — which is why only this line
            // moved, and that immobility is the point: a gate-version bump
            // must not silently re-key committed batches.
            "{\"payload_version\":1,\"redact_version\":3,\"origin\":\"own-repo\",",
            "\"created_unix\":1234567,\"rows_included\":2,\"rows_blocked\":0,",
            "\"batch_commitment\":\"d6395af4a5df9460d859f3de6ebc7f1372d55e4e11625a8bf1aa41622bf0401d\"}\n",
            "{\"shape_id\":\"241e0429984b\",\"lint_key\":\"needless_return\",",
            "\"shape\":\"fn g() { return 1; }\",\"fix\":\"fn g() { 1 }\",",
            "\"direction\":[0.1,-0.2,0.3,0.0,0.5,-0.5,0.7,0.9],\"success_count\":3,\"fail_count\":1,",
            "\"consecutive_fails\":0,\"tier\":0,\"last_run_id\":\"run-abc123-42\",",
            "\"last_seen_at\":1700000000,\"elo\":1215.5,\"span_elo\":1188.25,",
            "\"gate\":{\"decision\":\"allow\",\"findings\":[]}}\n",
            "{\"shape_id\":\"15ca779abbbc\",\"lint_key\":\"uninlined_format_args\",",
            "\"shape\":\"fn f() { println!(\\\"{}\\\", x); }\",\"fix\":\"fn f() { println!(\\\"{x}\\\"); }\",",
            "\"direction\":[0.1,-0.2,0.3,0.0,0.5,-0.5,0.7,0.9],\"success_count\":3,\"fail_count\":1,",
            "\"consecutive_fails\":0,\"tier\":0,\"last_run_id\":\"run-abc123-42\",",
            "\"last_seen_at\":1700000000,\"elo\":1215.5,\"span_elo\":1188.25,",
            "\"gate\":{\"decision\":\"allow\",\"findings\":[]}}\n",
        );
        assert_eq!(p.ndjson, expected);
    }
}

//! The pending-record estimator (Proposal 005 Phase 0 T0.4) — the local,
//! deterministic answer to "how much unproven heal data is sitting in this
//! store?", rendered under caveat #15's service-credit framing.
//!
//! ## What "pending" means here (and what it cannot mean)
//!
//! Proof status is SERVER-SIDE (Phase 2's trainer quorum) — locally
//! unknowable. The honest local partition of every store row is exactly
//! three classes, and the estimator is that partition:
//!
//! - **`blocked`** — [`RedactionGate`] would exclude the row from the next
//!   payload. It can NEVER be proven until the policy changes, so it is
//!   counted separately and EXCLUDED from the accrual estimate (a blocked
//!   row's "value" is money that cannot exist — counting it would violate
//!   caveat #15 harder than the mock it fixes).
//! - **`pending_unsynced`** — gate-passed, current projection NOT covered by
//!   the last-synced watermark (new row, or changed since sync: verdict
//!   counts, tier, or text moved).
//! - **`synced_awaiting_proof`** — gate-passed, current projection already
//!   covered (the server holds this exact row). Can overcount post-proof
//!   rows until settlement data flows back (Phase 2's channel) — locally
//!   undecidable, documented here rather than papered over.
//!
//! `records_pending_proof = pending_unsynced + synced_awaiting_proof`: BOTH
//! classes are unproven from the local seat, and the accrual estimate covers
//! that sum — it must NOT drop when a sync moves rows between the two
//! classes. Value only leaves the accrual when the server proves or rejects.
//!
//! ## Coverage = per-row digest of the PAYLOAD projection
//!
//! The watermark maps `"{lint_key}:{shape_id}"` (the store's own identity —
//! [`crate::traj_store::shape_hash`]) to `blake3_12` of the gated
//! [`PayloadRow`]'s serde JSON. The payload row — not the raw
//! [`TrajRowDoc`] — is what the server holds, so it is what "already sent"
//! is measured against: a policy change that alters redaction re-pends every
//! affected row (the server's copy is stale), and a verdict-count change
//! re-pends the row (new evidence the server lacks).
//!
//! The estimator projects through the BUILDER'S OWN
//! [`SyncPayloadBuilder::gate_row`] — same code path as the next real
//! payload, so estimator/payload agreement is by construction (no second
//! projection to drift).
//!
//! ## Pure by construction
//!
//! [`estimate_pending`] is a pure function of `(rows, gate, origin,
//! watermark)`: no wall-clock, no network, no store mutation — identical
//! inputs → identical output (the Decision's constraint; the
//! [`SyncPayloadBuilder::build_at`] determinism precedent). Only
//! [`SyncWatermark::load`] touches the filesystem, and it never aborts a
//! run (the [`crate::kat_redact::LoadedPolicy`] pattern): missing → empty
//! silently (bootstrap), corrupt / version mismatch → empty + surfaced
//! warning. Recovery-to-empty is deliberately NOT salvage — unlike a frozen
//! `charged_kat` history, row digests from another scheme are
//! uninterpretable, and the failure direction is over-sync (re-send
//! everything: cheap, safe), never under-sync (silent data loss).
//!
//! ## The estimate is a projection, not a promise (caveat #15)
//!
//! The real per-record value is epoch-pool + novelty-class dependent and is
//! frozen at T2.3 (reward schedule) — Phase 0 cannot know it.
//! [`PROVISIONAL_KAT_PER_PENDING_RECORD`] is a version-stamped placeholder
//! (the §Account mock's own implied ratio) that will be WRONG by design
//! until T2.3; the render says "provisional" on the same line as the number
//! — the disclaimer travels with the figure. No render in this module ever
//! emits `$` (the `$`-saved line refers to consumed healing — the
//! charge/meter surface, never an accrual); pinned by
//! `no_dollar_sign_in_any_render`.
//!
//! ```
//! # #[cfg(all(feature = "kat_service", feature = "latent_retrieval"))] {
//! # use riir_refine::kat_redact::{RedactionGate, SpanOrigin};
//! # use riir_refine::kat_pending_estimator::{estimate_pending, SyncWatermark};
//! let gate = RedactionGate::deny_all();
//! let est = estimate_pending(&[], &gate, SpanOrigin::OwnRepo, &SyncWatermark::empty());
//! assert_eq!(est.records_pending_proof(), 0);
//! assert_eq!(est.est_kat_service_credit(), 0);
//! assert!(!est.pending_line().contains('$'));
//! # }
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::kat_redact::{RedactionGate, SpanOrigin};
use crate::kat_sync_payload::PayloadRow;
use crate::traj_store::{shape_hash, TrajRowDoc};

// ── Version stamps (G9b — no silent shape/model change) ─────────────────────

/// The watermark-file schema version. Bump ONLY when the persisted shape of
/// [`SyncWatermark`] or the row-digest SCHEME changes (a scheme change makes
/// old digests uninterpretable → recovery re-pends everything — the
/// fail-safe direction), and record the delta in
/// [`SYNC_STATE_CHANGELOG`] in the same change. Independent of
/// [`ESTIMATE_VERSION`] on purpose: a rate change must not invalidate the
/// watermark, and a watermark scheme change must not read as a reprice (the
/// meter/charge two-axis discipline).
pub const SYNC_STATE_VERSION: u32 = 2;

/// Versioned deltas of the watermark shape (the migration audit trail).
pub const SYNC_STATE_CHANGELOG: &[(u32, &str)] = &[
    (
        1,
        "initial watermark (T0.4, 2026-08-26): {lint_key}:{shape_id} → \
             blake3_12(PayloadRow JSON), + last_batch_commitment + payload_version. \
             Recovery-to-empty on version mismatch is deliberate: digests from \
             another scheme are uninterpretable and over-sync is the safe direction.",
    ),
    (
        2,
        "payload v3 (Issue 103): PayloadRow gains `domain`, so EVERY row digest \
             moves — old digests stay uninterpretable-by-scheme, and bumping the \
             version makes that explicit: one recovery-to-empty re-pend (the safe \
             direction) instead of a silent whole-store mismatch read.",
    ),
];

/// The accrual-estimate model version. Bump ONLY when the estimated KAT for
/// the same pending-record set changes (a re-estimate), and record the delta
/// in [`ESTIMATE_CHANGELOG`] in the same change. The parameters this
/// placeholder stands in for freeze at T2.3 (reward schedule) — until then
/// every figure this model produces is provisional by construction.
pub const ESTIMATE_VERSION: u32 = 1;

/// Versioned deltas of the estimate model (the re-estimate audit trail).
pub const ESTIMATE_CHANGELOG: &[(u32, &str)] = &[(
    1,
    "initial estimate model (T0.4, 2026-08-26): provisional flat 35 KAT per \
         pending record — a PLACEHOLDER aligned with Proposal 005 §Account's \
         illustrative ratio (~42,000 KAT / 1,204 records ≈ 34.9). The real \
         per-record value is epoch-pool + novelty-class dependent (×3 \
         first-discoverer, documented-miss-class floor, decay) and freezes at \
         T2.3. Figures render ONLY as 'service credit, if proven' — never \
         beside a $ amount (caveat #15).",
)];

/// Provisional KAT per pending record (see [`ESTIMATE_CHANGELOG`]). A
/// placeholder until T2.3 freezes the reward schedule; any change is an
/// [`ESTIMATE_VERSION`] bump + changelog entry, never a silent re-estimate.
pub const PROVISIONAL_KAT_PER_PENDING_RECORD: u64 = 35;

// ── The sync watermark ──────────────────────────────────────────────────────

/// What the last successful sync pushed — the estimator's comparison
/// surface. Built post-push by Phase 1's `sync` command via
/// [`SyncWatermark::from_payload`]; until then [`SyncWatermark::empty`]
/// (bootstrap: every gate-passed row is pending).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SyncWatermark {
    /// The watermark shape + digest-scheme version ([`SYNC_STATE_VERSION`]).
    pub sync_state_version: u32,
    /// The pushed payload's `batch_commitment` (full 64-hex) — provenance,
    /// not coverage (coverage is the per-row map).
    pub last_batch_commitment: String,
    /// The pushed payload's schema version (provenance).
    pub payload_version: u32,
    /// `"{lint_key}:{shape_id}"` → `blake3_12` of the gated `PayloadRow`
    /// JSON — the rows the server holds, at the projection last sent.
    /// `BTreeMap` for deterministic serialization.
    pub synced_rows: BTreeMap<String, String>,
}

impl Default for SyncWatermark {
    fn default() -> Self {
        Self::empty()
    }
}

impl SyncWatermark {
    /// The watermark file name under the `.refine` root.
    pub const FILE_NAME: &'static str = "kat_sync_state.json";

    /// The never-synced watermark: everything gate-passed is pending.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            sync_state_version: SYNC_STATE_VERSION,
            last_batch_commitment: String::new(),
            payload_version: 0,
            synced_rows: BTreeMap::new(),
        }
    }

    /// The watermark a successful push of `payload` leaves behind: every
    /// INCLUDED row's digest, keyed by the store's identity. Phase 1's
    /// `sync` calls this after the server accepts the batch — one
    /// derivation for both the write side (here) and the read side
    /// ([`estimate_pending`]).
    #[must_use]
    pub fn from_payload(payload: &crate::kat_sync_payload::SyncPayload) -> Self {
        // Rows come from the payload itself (the gated projection the server
        // holds), not from a caller-supplied row list — parse our own bytes.
        // A parse failure here is a can't-happen (we serialized these lines;
        // round-trip pinned by tests) and degrades fail-safe: an empty map
        // re-syncs everything next time, never under-syncs.
        let rows = crate::kat_sync_payload::parse_rows(&payload.ndjson).unwrap_or_default();
        let mut map = BTreeMap::new();
        for r in &rows {
            map.insert(row_key(&r.lint_key, &r.shape_id), row_digest(r));
        }
        Self {
            sync_state_version: SYNC_STATE_VERSION,
            last_batch_commitment: payload.commitment.clone(),
            payload_version: crate::kat_sync_payload::PAYLOAD_VERSION,
            synced_rows: map,
        }
    }

    /// Load the watermark from `<root>/.refine/kat_sync_state.json`. Never
    /// aborts: missing → empty (bootstrap, no warning); corrupt / version
    /// mismatch → empty + warning (re-pend everything — over-sync is the
    /// fail-safe direction). The estimator stays pure; surfacing the
    /// warning belongs to the CLI.
    #[must_use]
    pub fn load(root: &Path) -> (Self, Option<String>) {
        Self::load_at(&root.join(".refine").join(Self::FILE_NAME))
    }

    /// The PER-NET watermark path (riir-kat Issue 003 T1): the watermark
    /// belongs to the net it was synced against — a fresh net re-pends
    /// everything by construction (the file simply is not there).
    #[must_use]
    pub fn net_path(root: &Path, url: &str) -> PathBuf {
        riir_kat::kat_state_stamp::net_state_dir_for_url(root, url).join(Self::FILE_NAME)
    }

    /// Exact-path loader (tests + the env-override pattern).
    #[must_use]
    pub fn load_at(path: &Path) -> (Self, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Self>(&text) {
                Ok(w) if w.sync_state_version == SYNC_STATE_VERSION => (w, None),
                Ok(w) => {
                    let warn = format!(
                        "kat-sync: {} carries watermark version {} (this build \
                         defines {}); re-pending everything (over-sync is the \
                         safe direction) — see SYNC_STATE_CHANGELOG",
                        path.display(),
                        w.sync_state_version,
                        SYNC_STATE_VERSION
                    );
                    (Self::empty(), Some(warn))
                }
                Err(e) => {
                    let warn = format!(
                        "kat-sync: ignoring unreadable watermark ({}): {e}; \
                         re-pending everything",
                        path.display()
                    );
                    (Self::empty(), Some(warn))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::empty(), None),
            Err(e) => {
                let warn = format!(
                    "kat-sync: ignoring unreadable watermark ({}): {e}; \
                     re-pending everything",
                    path.display()
                );
                (Self::empty(), Some(warn))
            }
        }
    }

    /// Persist the watermark (the post-push write — Phase 1's `sync` calls
    /// this; shipped now so the derivation is shared).
    ///
    /// # Errors
    ///
    /// Filesystem write failure (the caller decides how loud to be).
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text =
            serde_json::to_string_pretty(self).map_err(|e| format!("watermark serialize: {e}"))?;
        std::fs::write(path, text).map_err(|e| format!("watermark write: {e}"))
    }

    /// Does the watermark cover this row at this digest?
    #[must_use]
    pub fn covers(&self, lint_key: &str, shape_id: &str, digest: &str) -> bool {
        self.synced_rows
            .get(&row_key(lint_key, shape_id))
            .is_some_and(|d| d == digest)
    }

    /// Rows the watermark covers (diagnostics).
    #[must_use]
    pub fn len(&self) -> usize {
        self.synced_rows.len()
    }

    /// Is the coverage set empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.synced_rows.is_empty()
    }
}

/// The watermark key for one row: the store's own identity pair.
fn row_key(lint_key: &str, shape_id: &str) -> String {
    format!("{lint_key}:{shape_id}")
}

/// `blake3_12` of the gated `PayloadRow`'s serde JSON — the digest both the
/// write side ([`SyncWatermark::from_payload`]) and the read side
/// ([`estimate_pending`]) derive. Struct field order is fixed; `gate_row`'s
/// findings order is fixed (field order × fixed rule list) — deterministic.
fn row_digest(row: &PayloadRow) -> String {
    let text = serde_json::to_string(row).expect("payload row serializes");
    let hex = blake3::hash(text.as_bytes()).to_hex().to_string();
    hex[..12].to_string()
}

// ── The estimate ────────────────────────────────────────────────────────────

/// The local partition of the store's rows by proof reachability. Pure data
/// — see the module doc for what each class means and cannot mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PendingEstimate {
    /// Rows the gate blocks — never synced, never provable until the policy
    /// changes. Count only (details stay in the local redaction log).
    pub blocked: u64,
    /// Gate-passed rows the watermark does NOT cover at their current
    /// projection — the next `sync` pushes these.
    pub pending_unsynced: u64,
    /// Gate-passed rows the watermark covers — on the server, quorum outcome
    /// unknown locally (can overcount post-proof rows until Phase 2's
    /// settlement channel exists).
    pub synced_awaiting_proof: u64,
}

impl PendingEstimate {
    /// Records pending proof — everything that could still be proven EXCEPT
    /// blocked rows. Both unsynced and synced-awaiting count: proof status
    /// is server-side, and the accrual must not drop when a sync moves rows
    /// between the two classes.
    #[must_use]
    pub fn records_pending_proof(&self) -> u64 {
        self.pending_unsynced
            .saturating_add(self.synced_awaiting_proof)
    }

    /// The provisional service-credit estimate (caveat #15: a prospective
    /// usage discount, not income — renders ONLY as "service credit, if
    /// proven", never beside a `$` figure). Saturates rather than overflows.
    #[must_use]
    pub fn est_kat_service_credit(&self) -> u64 {
        self.records_pending_proof()
            .saturating_mul(PROVISIONAL_KAT_PER_PENDING_RECORD)
    }

    /// The accrual line — the caveat #15 render. Carries "service credit" +
    /// "if proven" + the provisional marker; carries NO `$` figure.
    #[must_use]
    pub fn pending_line(&self) -> String {
        format!(
            "Records pending proof: {} (est. {} KAT service credit if proven — \
             provisional rate, freezes at reward schedule v{})",
            thousands(self.records_pending_proof()),
            thousands(self.est_kat_service_credit()),
            ESTIMATE_VERSION
        )
    }

    /// The blocked line — count + policy hint, NO KAT figure (blocked rows
    /// can never be proven; estimating their value would print money that
    /// cannot exist).
    #[must_use]
    pub fn blocked_line(&self) -> String {
        format!(
            "Records blocked by redaction policy: {} (never synced: a secret \
             finding, a file outside .refine-sync-policy, or a row recorded \
             before its file was tracked — a later heal run re-records it)",
            thousands(self.blocked)
        )
    }
}

/// The estimator — a pure function of `(rows, gate, origin, watermark)`.
/// Projects every row through the payload builder's own gated-row projection
/// ([`crate::kat_sync_payload::gate_row`] — same code path as the next real
/// payload; no signed builder needed, row gating is envelope-independent)
/// and compares the gated projection's digest against the watermark. The
/// domain table is the COMPILED one (v3 — Issue 103) — the same default the
/// real builder uses, so estimator/payload digests agree by construction.
#[must_use]
pub fn estimate_pending(
    rows: &[TrajRowDoc],
    gate: &RedactionGate,
    origin: SpanOrigin,
    watermark: &SyncWatermark,
) -> PendingEstimate {
    let domains = crate::kat_sync_payload::RuleDomains::compiled();
    let mut est = PendingEstimate::default();
    for row in rows {
        let shape_id = shape_hash(&row.shape);
        match crate::kat_sync_payload::gate_row(gate, origin, row, &domains) {
            Ok(pr) => {
                let digest = row_digest(&pr);
                if watermark.covers(&pr.lint_key, &shape_id, &digest) {
                    est.synced_awaiting_proof += 1;
                } else {
                    est.pending_unsynced += 1;
                }
            }
            Err(_) => est.blocked += 1,
        }
    }
    est
}

/// Deterministic thousands separation (`1204` → `"1,204"`), the §Account
/// mock's display convention. Plain integer math — no allocation beyond the
/// result.
fn thousands(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let first_sep = bytes.len() % 3;
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && i % 3 == first_sep {
            out.push(',');
        }
        out.push(char::from(*b));
    }
    out
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kat_redact::{RedactionGate, SpanOrigin, SyncPolicy, REDACT_VERSION};
    use crate::kat_sync_payload::{golden_signer, SyncPayloadBuilder};
    use crate::traj_store::TrajRowDoc;

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

    /// `allow_redacted: true` — the redact-in-place arm.
    fn redact_policy() -> SyncPolicy {
        SyncPolicy {
            allow_redacted: true,
            ..allow_policy()
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
            rejected: Vec::new(),
            b0_attempts: 0,
            b0_successes: 0,
            bd_attempts: 0,
            bd_successes: 0,
            rescue_depth: 0,
        }
    }

    fn two_rows() -> Vec<TrajRowDoc> {
        vec![
            row(
                "uninlined_format_args",
                "fn f() { println!(\"{}\", x); }",
                "fn f() { println!(\"{x}\"); }",
            ),
            row("needless_return", "fn g() { return 1; }", "fn g() { 1 }"),
        ]
    }

    /// The watermark a sync of `rows` under `policy` would leave.
    fn watermark_after_sync(rows: &[TrajRowDoc], policy: &SyncPolicy) -> SyncWatermark {
        let gate = RedactionGate::from_policy(policy.clone());
        let payload =
            SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &golden_signer()).build_at(rows, 0);
        SyncWatermark::from_payload(&payload)
    }

    #[test]
    fn empty_store_all_zero() {
        let gate = RedactionGate::from_policy(allow_policy());
        let est = estimate_pending(&[], &gate, SpanOrigin::OwnRepo, &SyncWatermark::empty());
        assert_eq!(est, PendingEstimate::default());
        assert_eq!(est.records_pending_proof(), 0);
        assert_eq!(est.est_kat_service_credit(), 0);
    }

    #[test]
    fn bootstrap_no_watermark_everything_unsynced() {
        let gate = RedactionGate::from_policy(allow_policy());
        let est = estimate_pending(
            &two_rows(),
            &gate,
            SpanOrigin::OwnRepo,
            &SyncWatermark::empty(),
        );
        assert_eq!(est.pending_unsynced, 2);
        assert_eq!(est.synced_awaiting_proof, 0);
        assert_eq!(est.blocked, 0);
        assert_eq!(est.records_pending_proof(), 2);
        assert_eq!(est.est_kat_service_credit(), 70);
    }

    #[test]
    fn full_coverage_estimate_holds_not_drops() {
        let rows = two_rows();
        let policy = allow_policy();
        let wm = watermark_after_sync(&rows, &policy);
        let gate = RedactionGate::from_policy(policy);
        let est = estimate_pending(&rows, &gate, SpanOrigin::OwnRepo, &wm);
        // All covered — but STILL pending proof (proof is server-side).
        assert_eq!(est.pending_unsynced, 0);
        assert_eq!(est.synced_awaiting_proof, 2);
        assert_eq!(est.records_pending_proof(), 2);
        assert_eq!(est.est_kat_service_credit(), 70);
    }

    #[test]
    fn changed_row_re_pends() {
        let rows = two_rows();
        let policy = allow_policy();
        let wm = watermark_after_sync(&rows, &policy);
        let mut changed = rows.clone();
        changed[0].success_count += 1; // new evidence the server lacks
        let gate = RedactionGate::from_policy(policy);
        let est = estimate_pending(&changed, &gate, SpanOrigin::OwnRepo, &wm);
        assert_eq!(est.pending_unsynced, 1);
        assert_eq!(est.synced_awaiting_proof, 1);
    }

    #[test]
    fn new_row_pends() {
        let rows = two_rows();
        let policy = allow_policy();
        let wm = watermark_after_sync(&rows, &policy);
        let mut grown = rows.clone();
        grown.push(row("identity_op", "let y = x * 1;", "let y = x;"));
        let gate = RedactionGate::from_policy(policy);
        let est = estimate_pending(&grown, &gate, SpanOrigin::OwnRepo, &wm);
        assert_eq!(est.pending_unsynced, 1);
        assert_eq!(est.synced_awaiting_proof, 2);
    }

    #[test]
    fn coverage_follows_the_projection_not_the_row() {
        // A secret-bearing row the REDACTING policy includes (placeholders in
        // place): synced + covered. Under the ALLOW policy the same row is
        // BLOCKED — coverage evaporates when the projection changes class
        // (the server holds a row we would no longer send). Deterministic:
        // the planted AKIA value is the payload G4 fixture shape.
        let dirty = row(
            "needless_return",
            "let k = \"AKIAIOSFODNN7EXAMPLE\"; return k;",
            "let k = \"AKIAIOSFODNN7EXAMPLE\"; k",
        );
        let wm = watermark_after_sync(std::slice::from_ref(&dirty), &redact_policy());
        let redacting = RedactionGate::from_policy(redact_policy());
        let e = estimate_pending(
            std::slice::from_ref(&dirty),
            &redacting,
            SpanOrigin::OwnRepo,
            &wm,
        );
        assert_eq!(e.synced_awaiting_proof, 1);
        assert_eq!(e.blocked, 0);

        let allowing = RedactionGate::from_policy(allow_policy());
        let e2 = estimate_pending(
            std::slice::from_ref(&dirty),
            &allowing,
            SpanOrigin::OwnRepo,
            &wm,
        );
        assert_eq!(e2.blocked, 1);
        assert_eq!(e2.synced_awaiting_proof, 0);

        // The no-spurious-churn side: policies that produce IDENTICAL
        // projections (clean row, no findings) do not re-pend.
        let clean = two_rows();
        let wm_clean = watermark_after_sync(&clean, &allow_policy());
        let e3 = estimate_pending(&clean, &redacting, SpanOrigin::OwnRepo, &wm_clean);
        assert_eq!(e3.synced_awaiting_proof, 2);
        assert_eq!(e3.pending_unsynced, 0);
    }

    #[test]
    fn all_blocked_accrual_stays_zero() {
        // deny-all gate: every row blocked, nothing can ever be proven.
        let gate = RedactionGate::deny_all();
        let est = estimate_pending(
            &two_rows(),
            &gate,
            SpanOrigin::OwnRepo,
            &SyncWatermark::empty(),
        );
        assert_eq!(est.blocked, 2);
        assert_eq!(est.records_pending_proof(), 0);
        assert_eq!(est.est_kat_service_credit(), 0);
        assert!(!est.blocked_line().contains("KAT"));
    }

    #[test]
    fn no_dollar_sign_in_any_render() {
        let gate = RedactionGate::from_policy(allow_policy());
        let est = estimate_pending(
            &two_rows(),
            &gate,
            SpanOrigin::OwnRepo,
            &SyncWatermark::empty(),
        );
        for line in [est.pending_line(), est.blocked_line()] {
            assert!(!line.contains('$'), "caveat #15: {line}");
        }
    }

    #[test]
    fn accrual_line_language_gates() {
        let gate = RedactionGate::from_policy(allow_policy());
        let est = estimate_pending(
            &two_rows(),
            &gate,
            SpanOrigin::OwnRepo,
            &SyncWatermark::empty(),
        );
        let line = est.pending_line();
        assert!(line.contains("service credit"), "{line}");
        assert!(line.contains("if proven"), "{line}");
        assert!(line.contains("provisional"), "{line}");
        assert!(line.contains("KAT"), "{line}");
    }

    #[test]
    fn determinism_same_inputs_identical_output() {
        let gate = RedactionGate::from_policy(allow_policy());
        let wm = SyncWatermark::empty();
        let a = estimate_pending(&two_rows(), &gate, SpanOrigin::OwnRepo, &wm);
        let b = estimate_pending(&two_rows(), &gate, SpanOrigin::OwnRepo, &wm);
        assert_eq!(a, b);
        assert_eq!(a.pending_line(), b.pending_line());
        assert_eq!(a.blocked_line(), b.blocked_line());
    }

    #[test]
    fn watermark_round_trip() {
        let rows = two_rows();
        let wm = watermark_after_sync(&rows, &allow_policy());
        let dir = std::env::temp_dir().join(format!("kat_wm_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(SyncWatermark::FILE_NAME);
        wm.save(&path).unwrap();
        let (loaded, warn) = SyncWatermark::load_at(&path);
        assert!(warn.is_none());
        assert_eq!(loaded, wm);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_watermark_recovers_empty_with_warning() {
        let dir = std::env::temp_dir().join(format!("kat_wm_bad_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kat_sync_state.json");
        std::fs::write(&path, "not json").unwrap();
        let (wm, warn) = SyncWatermark::load_at(&path);
        assert_eq!(wm, SyncWatermark::empty());
        assert!(warn.is_some());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn version_mismatch_recovers_empty_with_warning() {
        let dir = std::env::temp_dir().join(format!("kat_wm_ver_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kat_sync_state.json");
        let future = r#"{"sync_state_version": 99, "last_batch_commitment": "", "payload_version": 1, "synced_rows": {}}"#;
        std::fs::write(&path, future).unwrap();
        let (wm, warn) = SyncWatermark::load_at(&path);
        assert_eq!(wm, SyncWatermark::empty());
        assert!(warn.is_some());
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn from_payload_covers_exactly_included_rows() {
        let clean = row("needless_return", "fn g() { return 1; }", "fn g() { 1 }");
        let dirty = row(
            "map_unwrap_or",
            "let k = \"AKIAIOSFODNN7EXAMPLE\";",
            "let k = String::new();",
        );
        let rows = vec![clean, dirty];
        let gate = RedactionGate::from_policy(allow_policy());
        let payload = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &golden_signer())
            .build_at(&rows, 0);
        assert_eq!(payload.rows_included, 1);
        assert_eq!(payload.rows_blocked, 1);
        let wm = SyncWatermark::from_payload(&payload);
        assert_eq!(wm.len(), 1, "only the included row is covered");
        let est = estimate_pending(&rows, &gate, SpanOrigin::OwnRepo, &wm);
        assert_eq!(est.synced_awaiting_proof, 1);
        assert_eq!(est.blocked, 1);
        assert_eq!(est.pending_unsynced, 0);
    }

    #[test]
    fn thousands_separator() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_204), "1,204");
        assert_eq!(thousands(42_000), "42,000");
        assert_eq!(thousands(1_000_000), "1,000,000");
    }

    #[test]
    fn estimate_saturates() {
        let est = PendingEstimate {
            blocked: 0,
            pending_unsynced: u64::MAX,
            synced_awaiting_proof: 1,
        };
        assert_eq!(est.records_pending_proof(), u64::MAX);
        assert_eq!(est.est_kat_service_credit(), u64::MAX);
    }

    #[test]
    fn changelog_covers_every_version() {
        for (v, _) in SYNC_STATE_CHANGELOG {
            assert!(*v <= SYNC_STATE_VERSION);
        }
        assert!(SYNC_STATE_CHANGELOG
            .iter()
            .any(|(v, _)| *v == SYNC_STATE_VERSION));
        for (v, _) in ESTIMATE_CHANGELOG {
            assert!(*v <= ESTIMATE_VERSION);
        }
        assert!(ESTIMATE_CHANGELOG
            .iter()
            .any(|(v, _)| *v == ESTIMATE_VERSION));
    }
}

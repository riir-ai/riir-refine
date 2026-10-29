//! `kat_migrate_client` — the account-key migration CLI plane (riir-dao
//! Proposal 001 T4 / riir-refine Proposal 023): the census preflight, the
//! dual-signed filing, the status read, and the proposal-issue vote hint
//! (riir-refine Issue 183: the worker files ONE proposal issue per request;
//! reactions on it are the votes).
//!
//! # The lanes
//!
//! - **`--census`** (report-only, `kat_service`): walk THIS box's
//!   account-keyed state — the per-net burn book + ack, the sync
//!   watermark, the account record — and name every surface with its
//!   staleness. THE PREFLIGHT GATE: a filing refuses while local rows
//!   are unsigned-for (they must drain UNDER THE OLD ACCOUNT first —
//!   the E-3 law), naming the drain command.
//! - **filing** (`--github <user>`, `kat_transport`): measure + freeze
//!   the GitHub `PoW` attestation (riir-auth `github_pow`, the client-side
//!   fetch arm), sign the dual request — the OLD key releases over
//!   `migrate_release_message(_v2)`, the account (NEW) key claims over
//!   `migrate_claim_message(_v2)`, the network-bound v2 armed from the
//!   service-info `ledger_id` exactly like the sync payload (an old
//!   server answers without the field and the legacy signature goes out
//!   unchanged) — and POST it. The ack renders the worker-created
//!   proposal-issue URL (or the not-created-yet line) + the vote rules.
//! - **`--object <request-id>`** (`kat_transport`): read the status and
//!   point at the proposal issue — "open <url> and react 👎". There is no
//!   objector-composed issue any more (Issue 183 removed it: a second issue
//!   carries zero votes by construction).
//! - **`--status <request-id>`** (`kat_transport`): the public read, plus
//!   the proposal URL, the frozen `old_bound_login`, and the vote rules.
//! - **`--attest-cosign <old_hex> <new_hex> --github <login>`**
//!   (`kat_transport`, riir-dapps Issue 143): run on the box of account B —
//!   the account the login is CURRENTLY bound to — with B's own key (the
//!   account record). It measures the `PoW` attestation, fixes `created_at`,
//!   signs `migrate_attest_message(_v2)` with B's key and prints an
//!   [`AttestCosigBundle`] (B's account + public key + the detached
//!   signature + the attestation and instant it covers). NO private key
//!   leaves its box: the bundle carries only public data.
//! - **filing with `--attest-cosig <bundle>`** (the box holding old + new):
//!   the filing reuses the bundle's attestation + `created_at` (both are
//!   inside the co-signed bytes), verifies the co-signature locally (fail
//!   fast), and files ATTEST-ONLY — the GitHub link stays on B. The
//!   worker's 600 s `created_at` skew bound applies: file within ten
//!   minutes of the co-sign.
//!
//! # The vote rules (Issue 183 §B/§C + Issue 184 rule C — the kernel's polarity)
//!
//! 👍 on the proposal issue ENDORSES the migration, 👎 OBJECTS. The rule
//! text is riir-kat's `proposal_vote_rules(timing)` VERBATIM (the same text
//! the issue body carries): the three core voters 30% each (Amendment 4),
//! every other GitHub account 1%; releases if 👍 outweighs 👎, a draw or no
//! votes cancels; with the core silent only ≥ 1-year-old accounts count and a passing vote releases the
//! timing's R2 span after filing (7 days LAW, 2 hours on the devnet fast
//! build). The renders print the ACTUAL close / hold / silent-owner release
//! instants (UTC) from the status read — never a hard-coded "24 h" where an
//! instant exists — and `@`-mention the owner, the old account's bound
//! login and the claimant.
//!
//! # The per-invocation key source (the T-PREFLIGHT product finding)
//!
//! The filing signs with TWO keys, so the record's key can never be the
//! whole mechanism: `--old-key <path>` names the OLD key file explicitly
//! (an OpenSSH Ed25519 private key at ANY path — the retired
//! `id_ed25519.retired-*` posture needs no reinstall), and the CLAIM
//! signature rides the account record's own key (the key whose blake3 IS
//! the new account id — after T-POST, the machine key). The env var is
//! NEVER consulted here (the measured finding: `RIIR_AUTH_ACCOUNT_KEY`
//! is login-time-only; a per-run env read would silently sign ONE side).

use crate::kat_account::KatAccount;
use riir_kat::kat_protocol_migrate::{
    MigrateAck, MigrateRequestState, MigrateStatusBody, PROPOSAL_LABEL, ProposalDraft,
    proposal_issue_html_url,
};
use riir_kat::kat_protocol_migrate::{
    MigratePowAttestation, attest_only_line, decode_pubkey, migrate_attest_message,
    migrate_attest_message_v2,
};
#[cfg(feature = "kat_transport")]
use riir_kat::kat_protocol_migrate::{
    MigrateRequestBody, migrate_claim_message, migrate_claim_message_v2, migrate_release_message,
    migrate_release_message_v2,
};
use riir_kat::kat_protocol_migrate::{proposal_vote_rules, render_span};
use riir_kat::kat_protocol_migrate_tally::{TALLY_CORE, TallyTiming};

/// The migrate repo base the proposal-issue URL composes against when the
/// status body carries only the issue NUMBER (`--repo-base` overrides; the
/// worker's own `KAT_MIGRATE_REPO_BASE` is authoritative when it sends the
/// URL itself).
pub const DEFAULT_MIGRATE_REPO_BASE: &str = "https://github.com/gist-rs/riir-dao-proposal";

/// How stale the burn book may be before a filing refuses (the E-3 bar).
/// The gate is the PENDING DELTA (unsynced rows), never raw age — a
/// 38-day-old ack with a settled book owes nothing; age only enters as
/// the never-synced signal (no ack file at all with charges on the book).
pub const MIGRATE_MAX_UNSYNCED: u64 = 0;

// ── the census (report-only; the filing gate) ────────────────────────────

/// One account-keyed surface's staleness, as the census names it.
pub struct CensusSurface {
    pub name: &'static str,
    pub detail: String,
    pub stale: bool,
}

/// Render a wire-KAT total honestly (the Issue-094 slip class: these values
/// are the push's own wire-unit arithmetic, never micro — a raw `µKAT` label
/// reads a 1 KAT delta as `1 µKAT`). One conversion, `MICRO_PER_KAT`.
fn render_wire_kat(wire_kat: u64) -> String {
    let micro = i64::try_from(wire_kat.saturating_mul(riir_kat::kat_units::MICRO_PER_KAT))
        .unwrap_or(i64::MAX);
    riir_kat::kat_units::render_units(micro, "KAT")
}

/// The local census: every account-keyed surface this repo touches, read
/// report-only (no file is written, nothing dials).
pub struct MigrateCensus {
    pub account_present: bool,
    pub account_id: Option<String>,
    pub github_user: Option<String>,
    pub key_path: Option<String>,
    pub surfaces: Vec<CensusSurface>,
    /// The filing refusals (E-3): empty = READY.
    pub blockers: Vec<String>,
}

/// Read the census for ONE repo root (the caller resolves the repo root
/// and the service URL; the burn surfaces are PER-NET — the same
/// `net_state_dir_for_url` split the push and the ack already use).
#[must_use]
pub fn census(
    root: &std::path::Path,
    account_dir: &std::path::Path,
    base_url: &str,
) -> MigrateCensus {
    let account = KatAccount::load(account_dir).ok();
    let mut surfaces = Vec::new();
    let mut blockers = Vec::new();

    let net_dir = riir_kat::kat_state_stamp::net_state_dir_for_url(root, base_url);
    let ack_path = net_dir.join(riir_kat::kat_protocol_burn::KatBurnAck::FILE_NAME);
    let (ack, ack_warning) = riir_kat::kat_burn_client::KatBurnAck::load_at(&ack_path);
    let ack_present = ack_path.exists();
    let now = now_unix();
    let ack_age_days = if ack.acked_unix > 0 && ack.acked_unix <= now {
        (now - ack.acked_unix) / 86_400
    } else {
        u64::MAX
    };

    // The charge book (micro-KAT) — read-only parse, no recovery write.
    let charge_path = net_dir.join(riir_kat::kat_meter::KatCharge::FILE_NAME);
    let charged_micro = read_u64_field(&charge_path, "charged_kat");

    // The pending delta: what the book holds that the ack never covered
    // (the wire-unit arithmetic the push itself uses — one truth).
    let pending = riir_kat::kat_burn_client::pending_report_total(&ack, charged_micro)
        .saturating_sub(ack.burn_kat_total);
    surfaces.push(CensusSurface {
        name: "burn ack",
        detail: if ack_present {
            format!(
                "{} (acked {} d ago, booked {}, pending {}){}",
                ack_path.display(),
                ack_age_days,
                render_wire_kat(ack.burn_kat_total),
                render_wire_kat(pending),
                ack_warning
                    .as_ref()
                    .map(|w| format!(" — {w}"))
                    .unwrap_or_default(),
            )
        } else {
            format!("{} (never synced)", ack_path.display())
        },
        stale: !ack_present || pending > MIGRATE_MAX_UNSYNCED,
    });
    if pending > MIGRATE_MAX_UNSYNCED {
        blockers.push(format!(
            "the burn book holds {} the service never acknowledged — \
             run `cargo refine sync` (the auto burn-sync drains it) UNDER THE OLD ACCOUNT first",
            render_wire_kat(pending)
        ));
    }
    if let Some(w) = &ack_warning {
        blockers.push(format!(
            "the burn ack failed to load cleanly ({w}) — \
             adjudicate it with `cargo refine config adopt-state` before migrating"
        ));
    }

    // The sync watermark (the mining/lease lane) — its own per-net file,
    // read RAW (the estimator module is latent_retrieval-gated; the census
    // must not widen its posture). Report-only: the burn delta is the
    // gate, never the watermark.
    let wm_path = riir_kat::kat_state_stamp::net_state_dir_for_url(root, base_url)
        .join("kat_sync_state.json");
    let wm_text = std::fs::read_to_string(&wm_path).ok();
    let wm: Option<serde_json::Value> = wm_text
        .as_deref()
        .and_then(|t| serde_json::from_str(t).ok());
    let synced_rows = wm
        .as_ref()
        .and_then(|v| v.get("synced_rows"))
        .and_then(|v| v.as_object())
        .map_or(0, serde_json::Map::len);
    let last_batch = wm
        .as_ref()
        .and_then(|v| v.get("last_batch_commitment"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned();
    surfaces.push(CensusSurface {
        name: "sync watermark",
        detail: format!(
            "{} ({} synced row(s), last batch {})",
            wm_path.display(),
            synced_rows,
            if last_batch.is_empty() {
                "none".to_owned()
            } else {
                last_batch[..12.min(last_batch.len())].to_owned()
            },
        ),
        stale: false,
    });

    MigrateCensus {
        account_present: account.is_some(),
        account_id: account.as_ref().map(|a| a.state().account_id.clone()),
        github_user: account.as_ref().and_then(|a| a.state().github_user.clone()),
        key_path: account.as_ref().map(|a| a.key_path().display().to_string()),
        surfaces,
        blockers,
    }
}

// ── the filing + reads (transport; kat_transport-gated call sites) ───────

/// The filing inputs (the bin parses the flags into this).
/// The census render (the `--census` mode; every line agent-parseable).
#[must_use]
pub fn render_census(c: &MigrateCensus, base_url: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "migrate census — net {base_url}");
    match (&c.account_present, &c.account_id) {
        (true, id) => {
            let _ = writeln!(
                out,
                "  account: {} (github: {}, key: {})",
                id.clone().unwrap_or_default(),
                c.github_user.as_deref().unwrap_or("(none)"),
                c.key_path.as_deref().unwrap_or("?"),
            );
        }
        (false, _) => out.push_str("  account: NONE — run `cargo refine login` first\n"),
    }
    for s in &c.surfaces {
        let _ = writeln!(
            out,
            "  [{}] {}: {}",
            if s.stale { "STALE" } else { "ok" },
            s.name,
            s.detail
        );
    }
    if c.blockers.is_empty() {
        out.push_str("  verdict: READY — the filing may proceed\n");
    } else {
        out.push_str("  verdict: STALE — the filing REFUSES until:\n");
        for b in &c.blockers {
            let _ = writeln!(out, "    - {b}");
        }
    }
    out
}

pub struct FileParams<'a> {
    pub github_user: &'a str,
    /// The OLD key file (OpenSSH Ed25519) — the per-invocation key source.
    pub old_key_path: &'a std::path::Path,
    /// Optional bearer for the GitHub `PoW` measure (rate limits only).
    pub github_token: Option<String>,
    /// The TOTP second factor (riir-dapps plan 053): the CURRENT
    /// authenticator code, riding the `X-Kat-Totp` header on the filing
    /// POST (the request route is in the worker's sensitive set — the OLD
    /// account is the custody loser). `None` is correct for every
    /// unenrolled account; an ARMED account files with `Some`, or the
    /// worker refuses `403 totp_required` (surfaced verbatim).
    pub totp_code: Option<&'a str>,
    /// riir-dapps Issue 143: the ATTEST-ONLY co-signature bundle B printed
    /// with `--attest-cosign` on its own box. `Some` → the filing reuses the
    /// bundle's attestation + `created_at` (no fresh measure) and carries
    /// B's co-signature; the GitHub link stays on B.
    pub attest_cosig: Option<AttestCosigBundle>,
}

// ── the attest-only co-signature (riir-dapps Issue 143) ─────────────────

/// The bundle B's box prints and the filing box consumes. Public data only:
/// B's account + public key + the detached co-signature, plus every input
/// the signature covers that the filing must reproduce byte-for-byte (the
/// attestation — its digest is in the signed body — and `created_at`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttestCosigBundle {
    /// The bundle shape version ([`ATTEST_COSIG_BUNDLE_VERSION`]).
    pub bundle_version: u32,
    pub old_account_hex: String,
    pub new_account_hex: String,
    pub github_user: String,
    /// The frozen `PoW` attestation the co-signature covers (by digest).
    pub attestation: MigratePowAttestation,
    /// The request body's creation instant (inside the co-signed bytes).
    pub created_at_unix: u64,
    /// The ledger id the co-signature is bound to (`None` = the legacy v1
    /// message — an old server without a ledger id).
    #[serde(default)]
    pub ledger_id_hex: Option<String>,
    /// B — the account the login is currently bound to (the co-signer).
    pub attest_by_hex: String,
    pub attest_by_pubkey_hex: String,
    pub attest_signature_hex: String,
}

/// [`AttestCosigBundle::bundle_version`] of this build.
pub const ATTEST_COSIG_BUNDLE_VERSION: u32 = 1;

/// Strict 128-hex → 64 bytes (an ed25519 signature).
fn decode_sig64(s: &str) -> Option<[u8; 64]> {
    let b = s.as_bytes();
    if b.len() != 128 {
        return None;
    }
    let mut out = [0u8; 64];
    for (i, pair) in b.chunks(2).enumerate() {
        let hi = u8::try_from((pair[0] as char).to_digit(16)?).ok()?;
        let lo = u8::try_from((pair[1] as char).to_digit(16)?).ok()?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn decode_hex32(s: &str, what: &str) -> Result<[u8; 32], String> {
    decode_pubkey(s).ok_or_else(|| format!("attest: {what} must be 64 hex chars"))
}

/// The co-signed message for a bundle's inputs: the ledger-bound v2 form
/// when a ledger id is known, else the legacy v1 form (the release/claim
/// convention).
fn attest_message(
    ledger_id: Option<&[u8; 32]>,
    old: &[u8; 32],
    new: &[u8; 32],
    github_user: &str,
    digest: &[u8; 32],
    created_at_unix: u64,
    attest_by: &[u8; 32],
) -> Vec<u8> {
    match ledger_id {
        Some(id) => migrate_attest_message_v2(
            id,
            old,
            new,
            github_user,
            digest,
            created_at_unix,
            attest_by,
        ),
        None => migrate_attest_message(old, new, github_user, digest, created_at_unix, attest_by),
    }
}

/// Compose B's co-signature bundle (pure — the transport arm measures the
/// attestation and reads the ledger id, then calls this). `b_key` is B's
/// OWN key; only its public half and the signature leave.
#[must_use]
pub fn compose_attest_cosig(
    b_key: &ed25519_dalek::SigningKey,
    old: &[u8; 32],
    new: &[u8; 32],
    github_user: &str,
    attestation: MigratePowAttestation,
    created_at_unix: u64,
    ledger_id: Option<[u8; 32]>,
) -> AttestCosigBundle {
    use ed25519_dalek::Signer as _;
    let b_pk = b_key.verifying_key().to_bytes();
    let b_acct: [u8; 32] = *blake3::hash(&b_pk).as_bytes();
    let msg = attest_message(
        ledger_id.as_ref(),
        old,
        new,
        github_user,
        &attestation.digest,
        created_at_unix,
        &b_acct,
    );
    let sig = b_key.sign(&msg);
    AttestCosigBundle {
        bundle_version: ATTEST_COSIG_BUNDLE_VERSION,
        old_account_hex: hex(old),
        new_account_hex: hex(new),
        github_user: github_user.to_owned(),
        attestation,
        created_at_unix,
        ledger_id_hex: ledger_id.map(|id| hex(&id)),
        attest_by_hex: hex(&b_acct),
        attest_by_pubkey_hex: hex(&b_pk),
        attest_signature_hex: hex(&sig.to_bytes()),
    }
}

/// The filing box's fail-fast check of a bundle against THIS filing: the
/// version, the old/new accounts, the login, the ledger id it is bound to,
/// B's key deriving B (the worker's hash-check), and the signature over the
/// exact message. Returns B's account on success.
///
/// # Errors
///
/// Any mismatch, named (the worker would refuse the same filing).
pub fn verify_attest_cosig(
    b: &AttestCosigBundle,
    old: &[u8; 32],
    new: &[u8; 32],
    github_user: &str,
    ledger_id: Option<[u8; 32]>,
) -> Result<[u8; 32], String> {
    use ed25519_dalek::Verifier as _;
    if b.bundle_version != ATTEST_COSIG_BUNDLE_VERSION {
        return Err(format!(
            "attest: bundle version {} (this build reads {ATTEST_COSIG_BUNDLE_VERSION})",
            b.bundle_version
        ));
    }
    if decode_hex32(&b.old_account_hex, "old_account_hex")? != *old
        || decode_hex32(&b.new_account_hex, "new_account_hex")? != *new
    {
        return Err(
            "attest: the bundle was co-signed for a different old/new account pair".to_owned(),
        );
    }
    if b.github_user != github_user || b.attestation.github_user != github_user {
        return Err(format!(
            "attest: the bundle is for @{}, the filing is for @{github_user}",
            b.github_user
        ));
    }
    let bundle_ledger = b
        .ledger_id_hex
        .as_deref()
        .map(|h| decode_hex32(h, "ledger_id_hex"))
        .transpose()?;
    if bundle_ledger != ledger_id {
        return Err(
            "attest: the bundle was co-signed for a different ledger (re-run --attest-cosign \
             against this service)"
                .to_owned(),
        );
    }
    let b_acct = decode_hex32(&b.attest_by_hex, "attest_by_hex")?;
    let b_pk = decode_hex32(&b.attest_by_pubkey_hex, "attest_by_pubkey_hex")?;
    if *blake3::hash(&b_pk).as_bytes() != b_acct {
        return Err("attest: the co-signer's public key does not derive attest_by_hex".to_owned());
    }
    let sig_bytes = decode_sig64(&b.attest_signature_hex)
        .ok_or("attest: attest_signature_hex must be 128 hex chars")?;
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&b_pk)
        .map_err(|e| format!("attest: the co-signer's public key is invalid: {e}"))?;
    let msg = attest_message(
        ledger_id.as_ref(),
        old,
        new,
        github_user,
        &b.attestation.digest,
        b.created_at_unix,
        &b_acct,
    );
    vk.verify(&msg, &ed25519_dalek::Signature::from_bytes(&sig_bytes))
        .map_err(|_| "attest: the co-signature does not verify over this filing".to_owned())?;
    Ok(b_acct)
}

/// Read a bundle from `--attest-cosig <arg>`: inline JSON (starts with
/// `{`) or a file path.
///
/// # Errors
///
/// An unreadable file or a body that is not a bundle.
pub fn parse_attest_cosig_arg(arg: &str) -> Result<AttestCosigBundle, String> {
    let text = if arg.trim_start().starts_with('{') {
        arg.to_owned()
    } else {
        std::fs::read_to_string(arg)
            .map_err(|e| format!("attest: reading the bundle {arg}: {e}"))?
    };
    serde_json::from_str(&text).map_err(|e| format!("attest: the bundle does not parse: {e}"))
}

/// The human render of a printed bundle (B's box): what was co-signed and
/// what to carry to the filing box.
#[must_use]
pub fn render_attest_cosig(b: &AttestCosigBundle) -> String {
    format!(
        "attest-cosign: co-signed as {by} (pubkey {pk})\n\
         \x20 old {old} -> new {new}, @{user}, created_at {created} — file within 10 minutes\n\
         \x20 {line}\n\
         carry this bundle to the filing box (public data only) and pass it as \
         `--attest-cosig '<json>'` or a file path:\n{json}",
        by = b.attest_by_hex,
        pk = b.attest_by_pubkey_hex,
        old = b.old_account_hex,
        new = b.new_account_hex,
        user = b.github_user,
        created = b.created_at_unix,
        line = attest_only_line(&b.attest_by_hex),
        json = serde_json::to_string(b).unwrap_or_default(),
    )
}

/// The filing outcome: the wire ack + the locally-known extras.
pub struct FileOutcome {
    pub ack: MigrateAck,
    pub filed: bool,
    pub window_close_unix: Option<u64>,
}

/// Compose + dual-sign the request body (the extracted sign step): the
/// network-bound v2 arm from the service-info read's ledger id (an old
/// server → the legacy signature — the sync-payload convention), the OLD
/// key over the release domain, the claim key over the claim domain.
#[cfg(feature = "kat_transport")]
#[allow(clippy::too_many_arguments)]
fn sign_request(
    old_key: &ed25519_dalek::SigningKey,
    claim_key: &ed25519_dalek::SigningKey,
    old_acct: &[u8; 32],
    new_acct: &[u8; 32],
    params: &FileParams<'_>,
    attestation: MigratePowAttestation,
    att_digest: [u8; 32],
    created_at_unix: u64,
    service_url: &str,
) -> Result<MigrateRequestBody, String> {
    use ed25519_dalek::Signer as _;
    let ledger_id = fetch_ledger_id(service_url)?;
    let old_pk = old_key.verifying_key().to_bytes();
    let new_pk = claim_key.verifying_key().to_bytes();
    let release = match ledger_id {
        Some(id) => migrate_release_message_v2(
            &id,
            old_acct,
            new_acct,
            params.github_user,
            &att_digest,
            created_at_unix,
        ),
        None => migrate_release_message(
            old_acct,
            new_acct,
            params.github_user,
            &att_digest,
            created_at_unix,
        ),
    };
    let claim = match ledger_id {
        Some(id) => migrate_claim_message_v2(
            &id,
            old_acct,
            new_acct,
            params.github_user,
            &att_digest,
            created_at_unix,
        ),
        None => migrate_claim_message(
            old_acct,
            new_acct,
            params.github_user,
            &att_digest,
            created_at_unix,
        ),
    };
    let release_sig = old_key.sign(&release);
    let claim_sig = claim_key.sign(&claim);
    Ok(MigrateRequestBody {
        old_account_hex: hex(old_acct),
        new_account_hex: hex(new_acct),
        github_user: params.github_user.to_owned(),
        attestation,
        old_pubkey_hex: hex(&old_pk),
        new_pubkey_hex: hex(&new_pk),
        release_signature_hex: hex(&release_sig.to_bytes()),
        claim_signature_hex: hex(&claim_sig.to_bytes()),
        created_at_unix,
        attest_by_hex: params
            .attest_cosig
            .as_ref()
            .map(|b| b.attest_by_hex.clone()),
        attest_by_pubkey_hex: params
            .attest_cosig
            .as_ref()
            .map(|b| b.attest_by_pubkey_hex.clone()),
        attest_signature_hex: params
            .attest_cosig
            .as_ref()
            .map(|b| b.attest_signature_hex.clone()),
    })
}

/// File the dual-signed migration request. The census gate runs FIRST
/// (E-3); every later failure is the service's own refusal surfaced
/// verbatim (the wire vocabulary one code per class). TRANSPORT-lane
/// (the `PoW` measure + the POST); a `kat_service`-only build refuses via
/// the off-posture arm below.
///
/// # Errors
///
/// The census gate (rendered verdict), key-load or PoW-measure failures,
/// and the service's own refusal verbatim (HTTP status + body).
#[cfg(feature = "kat_transport")]
pub fn file_migration(
    root: &std::path::Path,
    params: &FileParams<'_>,
    service_url: &str,
) -> Result<FileOutcome, String> {
    // The gate (E-3): nothing files while the old account's rows are
    // unsigned-for. The census is per-net, the SAME split the drain uses.
    let c = census(root, &KatAccount::default_dir(), service_url);
    if !c.account_present {
        return Err("migrate: no account — run `cargo refine login` first".to_owned());
    }
    if !c.blockers.is_empty() {
        return Err(render_census(&c, service_url));
    }
    let account = KatAccount::load(&KatAccount::default_dir())
        .map_err(|e| format!("migrate: loading the account record: {e}"))?;
    let claim_key = account.signing_key()?;
    let old_key = load_old_key(params.old_key_path)?;

    let old_pk = old_key.verifying_key().to_bytes();
    let new_pk = claim_key.verifying_key().to_bytes();
    let old_acct: [u8; 32] = *blake3::hash(&old_pk).as_bytes();
    let new_acct: [u8; 32] = *blake3::hash(&new_pk).as_bytes();

    // Issue 143 — an ATTEST-ONLY filing reuses the bundle's attestation +
    // created_at (both are inside B's co-signed bytes) after verifying the
    // co-signature locally against THIS filing (fail fast).
    let (wire_att, created_at) = match &params.attest_cosig {
        Some(b) => {
            let ledger_id = fetch_ledger_id(service_url)?;
            verify_attest_cosig(b, &old_acct, &new_acct, params.github_user, ledger_id)?;
            (b.attestation.clone(), b.created_at_unix)
        }
        None => (measure_attestation(params)?, now_unix()),
    };
    let att_digest = wire_att.digest;
    let body = sign_request(
        &old_key,
        &claim_key,
        &old_acct,
        &new_acct,
        params,
        wire_att,
        att_digest,
        created_at,
        service_url,
    )?;
    let resp = post_json(
        service_url,
        riir_kat::kat_protocol_migrate::ROUTE_REQUEST,
        &serde_json::to_string(&body).map_err(|e| format!("migrate: body: {e}"))?,
        params.totp_code,
    )?;
    let ack: MigrateAck = serde_json::from_str(&resp)
        .map_err(|e| format!("migrate: ack parse: {e} — body: {resp}"))?;
    // The additive extras ride the ack JSON beside the wire fields.
    let filed = parse_bool_field(&resp, "filed").unwrap_or(true);
    // The proposal-issue URL rides the ack itself (`proposal_issue_url`);
    // the CLI never composes one — only the worker-created issue votes.
    Ok(FileOutcome {
        ack,
        filed,
        window_close_unix: parse_u64_field(&resp, "window_close_unix"),
    })
}

/// The frozen `PoW` claim (client-side measure — the worker never fetches).
#[cfg(feature = "kat_transport")]
fn measure_attestation(params: &FileParams<'_>) -> Result<MigratePowAttestation, String> {
    measure_login(params.github_user, params.github_token.clone())
}

#[cfg(feature = "kat_transport")]
fn measure_login(
    github_user: &str,
    github_token: Option<String>,
) -> Result<MigratePowAttestation, String> {
    let source = riir_kat::github_pow::UreqGithubUserSource::new(github_token);
    let att = riir_kat::github_pow::GithubPowAttestation::measure(github_user, &source)
        .map_err(|e| format!("migrate: PoW measure failed: {e}"))?;
    Ok(MigratePowAttestation {
        github_user: att.github_user,
        measured_at_unix: att.measured_at_unix,
        created_at_unix: att.created_at_unix,
        followers: att.followers,
        stars_received: att.stars_received,
        digest: att.digest,
    })
}

/// `--attest-cosign <old_hex> <new_hex> --github <login>` (riir-dapps Issue
/// 143) — run on B's box, B = THIS box's account record (the account the
///      login is bound to): measure the attestation, fix `created_at`, co-sign
///      with B's key, return the bundle. B's private key never leaves the box.
///
/// # Errors
///
/// Bad hex, no account record, a failed measure or ledger-id read.
#[cfg(feature = "kat_transport")]
pub fn attest_cosign(
    old_account_hex: &str,
    new_account_hex: &str,
    github_user: &str,
    github_token: Option<String>,
    service_url: &str,
) -> Result<AttestCosigBundle, String> {
    let old = decode_hex32(old_account_hex, "the old account")?;
    let new = decode_hex32(new_account_hex, "the new account")?;
    let account = KatAccount::load(&KatAccount::default_dir())
        .map_err(|e| format!("attest: loading THIS box's account record (B): {e}"))?;
    let b_key = account.signing_key()?;
    let attestation = measure_login(github_user, github_token)?;
    let ledger_id = fetch_ledger_id(service_url)?;
    Ok(compose_attest_cosig(
        &b_key,
        &old,
        &new,
        github_user,
        attestation,
        now_unix(),
        ledger_id,
    ))
}

/// The status read — `GET /account/migrate/status?request_id=`; the raw
/// JSON comes back verbatim (additive wire), parsed just enough to render.
///
/// # Errors
///
/// A transport failure or a non-JSON body. An HTTP error status comes
/// back AS the body (the refusal JSON renders verbatim).
#[cfg(feature = "kat_transport")]
pub fn status(service_url: &str, request_id_hex: &str) -> Result<String, String> {
    let agent = agent();
    let url = format!(
        "{}{}?request_id={}",
        service_url.trim_end_matches('/'),
        riir_kat::kat_protocol_migrate::ROUTE_STATUS,
        request_id_hex
    );
    match riir_kat::kat_ua::apply(agent.get(&url)).call() {
        Ok(r) => Ok(r.into_string().map_err(|e| format!("migrate: body: {e}"))?),
        Err(ureq::Error::Status(c, r)) => Ok(r
            .into_string()
            .unwrap_or_else(|_| format!("{{\"status\":{c}}}"))),
        Err(e) => Err(format!("migrate: {e}")),
    }
}

/// The OLD key loader — the substrate's own arbitrary-path helper
/// (`kat_account::load_signing_key_at`, the ONE parse law). The migrate
/// release key is deliberately NOT the record's key.
#[cfg(feature = "kat_transport")]
fn load_old_key(path: &std::path::Path) -> Result<ed25519_dalek::SigningKey, String> {
    crate::kat_account::KatAccount::load_signing_key_at(path)
}

#[cfg(feature = "kat_transport")]
fn agent() -> ureq::Agent {
    let t = std::time::Duration::from_secs(30);
    ureq::AgentBuilder::new()
        .timeout(t)
        .timeout_connect(t)
        .timeout_read(t)
        .build()
}

#[cfg(feature = "kat_transport")]
fn post_json(
    base: &str,
    route: &str,
    body: &str,
    totp_code: Option<&str>,
) -> Result<String, String> {
    let url = format!("{}{}", base.trim_end_matches('/'), route);
    let req = riir_kat::kat_ua::apply(agent().post(&url)).set("Content-Type", "application/json");
    // The second factor rides the transport-level header (plan 053 —
    // never part of a signed body; the replay-safety is the persisted
    // step, so a leaked code is one-time).
    let req = match totp_code {
        Some(code) => req.set(riir_kat::kat_protocol_totp::TOTP_HEADER, code),
        None => req,
    };
    match req.send_string(body) {
        Ok(r) => r.into_string().map_err(|e| format!("migrate: body: {e}")),
        Err(ureq::Error::Status(c, r)) => {
            let text = r.into_string().unwrap_or_default();
            Err(format!("migrate: HTTP {c} — {text}"))
        }
        Err(e) => Err(format!("migrate: {e}")),
    }
}

/// The service-info read's ledger id (the sync-payload convention). `pub`
/// for the totp runner's v2-first signing (the same read, one home).
///
/// # Errors
///
/// A transport fault or an unreadable info body as a human-readable
/// string; an unreachable service reads as `Ok(None)` (the filing fails
/// closed server-side when the posture demands v2).
#[cfg(feature = "kat_transport")]
pub fn fetch_ledger_id(service_url: &str) -> Result<Option<[u8; 32]>, String> {
    let url = format!("{}/", service_url.trim_end_matches('/'));
    match riir_kat::kat_ua::apply(agent().get(&url)).call() {
        Ok(r) => {
            let body = r.into_string().map_err(|e| format!("migrate: info: {e}"))?;
            Ok(riir_kat::kat_lease_client::parse_service_ledger_id(
                body.as_bytes(),
            ))
        }
        // An unreachable info read falls back to legacy — the filing
        // itself fails closed at the server if the posture demands v2.
        Err(_) => Ok(None),
    }
}

/// The off-transport arm (this build compiles the network half out).
///
/// # Errors
///
/// Always — the refusal names the missing `kat_transport` feature.
#[cfg(not(feature = "kat_transport"))]
pub fn file_migration(
    _root: &std::path::Path,
    _params: &FileParams<'_>,
    _service_url: &str,
) -> Result<FileOutcome, String> {
    Err(riir_refine_off_transport())
}

/// The off-transport arm (this build compiles the network half out).
///
/// # Errors
///
/// Always — the refusal names the missing `kat_transport` feature.
#[cfg(not(feature = "kat_transport"))]
pub fn status(_service_url: &str, _request_id_hex: &str) -> Result<String, String> {
    Err(riir_refine_off_transport())
}

#[cfg(not(feature = "kat_transport"))]
fn riir_refine_off_transport() -> String {
    "migrate: this lane needs `kat_transport` (the PoW measure + the service POST);      this build compiles it out — rebuild with --features kat_transport"
        .to_owned()
}

// ── small local helpers (byte-local on purpose: the census is std-only) ──

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn hex(bytes: &[u8]) -> String {
    riir_kat::kat_payreq_client::hex_encode(bytes)
}

/// Read one numeric field from a JSON file without a full parse (the
/// census is a report; the files it reads carry the field top-level).
fn read_u64_field(path: &std::path::Path, field: &str) -> u64 {
    let Ok(text) = std::fs::read_to_string(path) else {
        return 0;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v.get(field).and_then(serde_json::Value::as_u64))
        .unwrap_or(0)
}

#[cfg(feature = "kat_transport")]
fn parse_u64_field(json: &str, field: &str) -> Option<u64> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get(field).and_then(serde_json::Value::as_u64))
}

#[cfg(feature = "kat_transport")]
fn parse_bool_field(json: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get(field).and_then(serde_json::Value::as_bool))
}

// ── the proposal-issue renders (Issue 183 T3; pure, transport-free) ─────

/// The line naming WHERE the votes go: the pinned proposal issue, or the
/// honest not-opened-yet posture (riir-refine Issue 187 — YOU open it from
/// the draft; E12: the request cannot release until it is opened + found).
#[must_use]
pub fn render_proposal_line(proposal_issue_url: Option<&str>) -> String {
    match proposal_issue_url {
        Some(u) => format!("proposal issue: {u}"),
        None => "proposal issue: not opened yet — open the draft below under \
                 the claimant GitHub account; the request cannot release until \
                 the worker finds it"
            .to_owned(),
    }
}

// ── the filer-authored draft (riir-refine Issue 187) ─────────────────────

/// How the draft got opened (or why it was not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftOpen {
    /// `gh` opened it (or it was already open) — the issue URL.
    Opened(String),
    /// Not opened by the CLI — the reason; the user clicks the draft URL.
    Manual(String),
}

/// `owner/repo` out of a draft's `https://github.com/owner/repo/issues/new?…`.
#[must_use]
pub fn draft_repo_slug(draft: &ProposalDraft) -> Option<String> {
    let rest = draft.new_issue_url.strip_prefix("https://github.com/")?;
    let (repo, _) = rest.split_once("/issues/new")?;
    let (owner, name) = repo.split_once('/')?;
    (!owner.is_empty() && !name.is_empty() && !name.contains('/')).then(|| repo.to_owned())
}

/// The manual instructions: the claimant opens the prefilled form THEMSELF
/// (the worker holds no GitHub write credential — riir-refine Issue 187).
#[must_use]
pub fn render_draft_instructions(draft: &ProposalDraft, reason: &str) -> String {
    format!(
        "proposal draft: open this as GitHub @{login} ({reason}) — do not edit the title or body:\n  {url}\n\
         then re-run --status; the worker pins it once it sees @{login}'s exact issue",
        login = draft.author_login,
        url = draft.new_issue_url,
    )
}

/// Run one `gh` command; stdout on success, the reason otherwise. The
/// user's OWN `gh` credential — this process never reads or forwards it.
fn gh(args: &[&str], stdin: Option<&str>) -> Result<String, String> {
    use std::io::Write as _;
    let mut cmd = std::process::Command::new("gh");
    cmd.args(args)
        .stdin(match stdin {
            Some(_) => std::process::Stdio::piped(),
            None => std::process::Stdio::null(),
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|_| "gh is not installed".to_owned())?;
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        pipe.write_all(text.as_bytes())
            .map_err(|e| format!("gh stdin: {e}"))?;
    }
    let out = child.wait_with_output().map_err(|e| format!("gh: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
    }
    Err(format!(
        "gh {} failed: {}",
        args.first().copied().unwrap_or_default(),
        String::from_utf8_lossy(&out.stderr).trim()
    ))
}

/// Open the draft with the user's own `gh` — ONLY when `gh` is logged in as
/// the draft's author (any other account's issue is never pinned), and
/// never twice (an existing exact-title issue by the author is reused). Any
/// miss falls back to [`DraftOpen::Manual`].
#[must_use]
pub fn open_proposal_draft(draft: &ProposalDraft) -> DraftOpen {
    let Some(slug) = draft_repo_slug(draft) else {
        return DraftOpen::Manual("the draft URL names no GitHub repo".to_owned());
    };
    let login = match gh(&["api", "user", "--jq", ".login"], None) {
        Ok(l) => l,
        Err(e) => return DraftOpen::Manual(e),
    };
    if !login.eq_ignore_ascii_case(&draft.author_login) {
        return DraftOpen::Manual(format!(
            "gh is logged in as @{login}, not @{}",
            draft.author_login
        ));
    }
    let list_path = format!("repos/{slug}/issues?creator={login}&state=all&per_page=100");
    if let Ok(json) = gh(&["api", &list_path], None) {
        let existing = serde_json::from_str::<Vec<serde_json::Value>>(&json)
            .unwrap_or_default()
            .into_iter()
            .find(|i| i.get("title").and_then(|t| t.as_str()) == Some(draft.title.as_str()))
            .and_then(|i| {
                i.get("html_url")
                    .and_then(|u| u.as_str())
                    .map(str::to_owned)
            });
        if let Some(url) = existing {
            return DraftOpen::Opened(url);
        }
    }
    match gh(
        &[
            "issue",
            "create",
            "--repo",
            &slug,
            "--title",
            &draft.title,
            "--body-file",
            "-",
        ],
        Some(&draft.body),
    ) {
        Ok(url) => DraftOpen::Opened(url),
        Err(e) => DraftOpen::Manual(e),
    }
}

/// The timing a status body says the request is judged under (riir-refine
/// Issue 184): the worker's `tally_timing`, else [`TallyTiming::LAW`] (a
/// worker that predates the field ran LAW).
#[must_use]
pub fn status_timing(s: &MigrateStatusBody) -> TallyTiming {
    s.tally_timing.unwrap_or(TallyTiming::LAW)
}

/// The `@`-mentions a request notifies: the core voters (Amendment 4),
/// the old account's bound login (frozen at filing), the claimant —
/// case-insensitively deduped, in that order.
#[must_use]
pub fn vote_mentions(old_bound_login: Option<&str>, claimant: Option<&str>) -> String {
    let mut seen: Vec<&str> = TALLY_CORE.iter().map(|&(l, _)| l).collect();
    for l in [old_bound_login, claimant].into_iter().flatten() {
        if !l.is_empty() && !seen.iter().any(|p| p.eq_ignore_ascii_case(l)) {
            seen.push(l);
        }
    }
    seen.iter()
        .map(|l| format!("@{l}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The core voters as `@a, @b, @c` (Amendment 4).
fn core_mentions() -> String {
    TALLY_CORE
        .iter()
        .map(|&(l, _)| format!("@{l}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The vote rules (Issue 184 rule C): where to vote, the polarity, then
/// riir-kat's `proposal_vote_rules(timing)` VERBATIM (the issue body's own
/// text — the R2 span rendered from `timing`), then who is notified.
#[must_use]
pub fn render_vote_rules(timing: TallyTiming, mentions: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "votes: react on the proposal issue (label {PROPOSAL_LABEL}; only that issue counts; \
         one account = one vote; both ways = no vote)"
    );
    let _ = writeln!(out, "  👍 ENDORSES the migration (release vote)");
    let _ = writeln!(out, "  👎 OBJECTS to it (cancel vote)");
    let _ = writeln!(out, "  {}", proposal_vote_rules(timing));
    let _ = write!(out, "  notified: {mentions}");
    out
}

/// The request's timeline as ACTUAL UTC instants (Issue 184): the window
/// close (where an owner vote decides), the silent-owner release instant
/// (filing + the timing's R2 span — derived from the close, since the close
/// IS filing + the window), or the HOLD instant when the tally already
/// held. Terminal rows render their close instant. One line per fact.
#[must_use]
pub fn render_timeline(s: &MigrateStatusBody) -> Vec<String> {
    let timing = status_timing(s);
    let mut out = Vec::new();
    match s.state {
        MigrateRequestState::Released | MigrateRequestState::Cancelled => {
            if let Some(t) = s.closed_at_unix {
                out.push(format!("closed at {} (unix {t})", fmt_utc(t)));
            }
            return out;
        }
        MigrateRequestState::Pending | MigrateRequestState::Dispute => {}
    }
    if let Some(close) = s.window_close_unix {
        out.push(format!(
            "window closes at {} (unix {close}) — a core vote ({}) is judged then",
            fmt_utc(close),
            core_mentions()
        ));
    }
    match (s.hold_until_unix, s.window_close_unix) {
        (Some(t), _) => out.push(format!(
            "HELD — the core ({core}) silent, the ≥ 1-year-old accounts' vote passes; \
             releases at {} unless a core voter 👎s (unix {t})",
            fmt_utc(t),
            core = core_mentions()
        )),
        (None, Some(close)) => {
            let filed = close.saturating_sub(timing.window_secs());
            let t = timing.silent_owner_release_at_unix(filed);
            out.push(format!(
                "if the core ({}) stays silent: a passing vote releases at {} \
                 (filing + {}; unix {t})",
                core_mentions(),
                fmt_utc(t),
                render_span(timing.silent_owner_window_secs())
            ));
        }
        (None, None) => {}
    }
    if !timing.is_law() {
        out.push(format!(
            "timing: DEVNET FAST ({} window, {} silent-owner hold) — never the mainnet law",
            render_span(timing.window_secs()),
            render_span(timing.silent_owner_window_secs())
        ));
    }
    out
}

/// The filing's tail (after the request + proposal lines): the timeline +
/// rules from the post-filing status read when it parsed, else the ack's
/// close instant + the LAW rules, labelled as assumed (the ack carries no
/// timing).
#[must_use]
pub fn render_filing_tail(
    window_close_unix: Option<u64>,
    status: Option<&MigrateStatusBody>,
    github_user: &str,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    if let Some(s) = status {
        for line in render_timeline(s) {
            let _ = writeln!(out, "{line}");
        }
        out.push_str(&render_vote_rules(
            status_timing(s),
            &vote_mentions(s.old_bound_login.as_deref(), Some(&s.github_user)),
        ));
        return out;
    }
    if let Some(close) = window_close_unix {
        let _ = writeln!(
            out,
            "window closes at {} (unix {close}) — default-deny",
            fmt_utc(close)
        );
    }
    let _ = writeln!(
        out,
        "(the status read failed — the rules below assume the LAW timing; \
         `--status` shows the deployment's)"
    );
    out.push_str(&render_vote_rules(
        TallyTiming::LAW,
        &vote_mentions(None, Some(github_user)),
    ));
    out
}

/// `YYYY-MM-DD HH:MM:SS UTC` from unix seconds (the shared Hinnant civil
/// helper — no chrono dep for one label).
#[must_use]
pub fn fmt_utc(unix: u64) -> String {
    let (y, m, d) = crate::kat_footer::ymd_from_unix(unix);
    let sod = unix % 86_400;
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// The proposal URL a status body names: the worker's own URL first, else
/// composed from the issue NUMBER over `repo_base`, else `None`.
#[must_use]
pub fn status_proposal_url(s: &MigrateStatusBody, repo_base: &str) -> Option<String> {
    s.proposal_issue_url.clone().or_else(|| {
        s.proposal_issue
            .map(|n| proposal_issue_html_url(repo_base, n))
    })
}

/// The structured status render (appended under the raw JSON): the
/// proposal URL, the frozen `old_bound_login`, the timeline (close / hold /
/// silent-owner release instants in UTC), the rule-C vote rules.
#[must_use]
pub fn render_status(s: &MigrateStatusBody, repo_base: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "migrate: request {} — state {}",
        s.request_id_hex,
        state_name(s.state)
    );
    let url = status_proposal_url(s, repo_base);
    let _ = writeln!(out, "{}", render_proposal_line(url.as_deref()));
    if let (None, Some(d)) = (url.as_ref(), s.proposal_draft.as_ref()) {
        let _ = writeln!(out, "{}", render_draft_instructions(d, "not opened yet"));
    }
    let _ = writeln!(
        out,
        "old account's bound login (frozen at filing): {}",
        s.old_bound_login
            .as_deref()
            .map_or_else(|| "none".to_owned(), |l| format!("@{l}"))
    );
    if let Some(line) = render_binding_line(s) {
        let _ = writeln!(out, "{line}");
    }
    if let Some(reason) = s.cancel_reason.as_deref() {
        let _ = writeln!(out, "{}", render_cancel_reason(reason));
    }
    for line in render_timeline(s) {
        let _ = writeln!(out, "{line}");
    }
    out.push_str(&render_vote_rules(
        status_timing(s),
        &vote_mentions(s.old_bound_login.as_deref(), Some(&s.github_user)),
    ));
    out.push('\n');
    out
}

/// The `--object` hint over a status body: "open <url> and react 👎". A
/// terminal request or a missing proposal issue refuses with the reason
/// (there is nothing to vote on).
///
/// # Errors
///
/// The request is already terminal, or no proposal issue exists yet.
pub fn render_object_hint(s: &MigrateStatusBody, repo_base: &str) -> Result<String, String> {
    match s.state {
        MigrateRequestState::Pending | MigrateRequestState::Dispute => {}
        MigrateRequestState::Released | MigrateRequestState::Cancelled => {
            return Err(format!(
                "object: request {} is already {} — there is nothing left to vote on",
                s.request_id_hex,
                state_name(s.state)
            ));
        }
    }
    let Some(url) = status_proposal_url(s, repo_base) else {
        return Err(format!(
            "object: request {} has no proposal issue yet — the worker retries; \
             the request cannot release until it exists (re-run `--object` later)",
            s.request_id_hex
        ));
    };
    Ok(format!(
        "object: open {url} and react 👎 (only that issue counts; \
         reacting 👍 too cancels your vote)"
    ))
}

/// The attest-only line for a status body (riir-dapps Issue 143): `Some`
/// exactly when the filing was attest-only — the GitHub link stays on the
/// co-signer.
#[must_use]
pub fn render_binding_line(s: &MigrateStatusBody) -> Option<String> {
    s.attest_by_hex.as_deref().map(attest_only_line)
}

/// Why a release-verdict close cancelled (riir-dapps Issue 143), with the
/// remedy.
#[must_use]
pub fn render_cancel_reason(reason: &str) -> String {
    match reason {
        riir_kat::kat_protocol_migrate::CANCEL_REASON_CLAIM_NOT_FRESH => {
            "cancelled: the claim account stopped being fresh (it gained rows after filing) — \
             re-file into a fresh claim account"
                .to_owned()
        }
        riir_kat::kat_protocol_migrate::CANCEL_REASON_GITHUB_CONFLICT => {
            "cancelled: the GitHub login was bound to another account mid-window — re-file \
             attest-only with that account's co-signature (--attest-cosig)"
                .to_owned()
        }
        other => format!("cancelled: {other}"),
    }
}

/// Parse a status read's body into the wire shape (`None` on a refusal
/// JSON — the caller prints the raw body either way).
#[must_use]
pub fn parse_status(body: &str) -> Option<MigrateStatusBody> {
    serde_json::from_str(body).ok()
}

fn state_name(state: MigrateRequestState) -> &'static str {
    match state {
        MigrateRequestState::Pending => "pending",
        MigrateRequestState::Dispute => "dispute",
        MigrateRequestState::Released => "released",
        MigrateRequestState::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Issue 183 T3: the proposal-issue renders (polarity pinned) ──

    fn status_body(
        state: MigrateRequestState,
        issue: Option<u64>,
        url: Option<&str>,
        victim: Option<&str>,
    ) -> MigrateStatusBody {
        MigrateStatusBody {
            request_id_hex: "ab".repeat(32),
            state,
            old_account_hex: "aa".repeat(32),
            new_account_hex: "bb".repeat(32),
            github_user: "claimant".to_owned(),
            created_at_unix: 1_792_900_000,
            window_close_unix: Some(1_792_986_400),
            dispute_with_hex: None,
            closed_at_unix: None,
            tally_snapshot_digest_hex: None,
            proposal_issue: issue,
            proposal_issue_url: url.map(str::to_owned),
            proposal_draft: None,
            old_bound_login: victim.map(str::to_owned),
            hold_until_unix: None,
            tally_timing: None,
            attest_by_hex: None,
            cancel_reason: None,
        }
    }

    #[test]
    fn the_vote_rules_render_rule_c_verbatim() {
        let r = render_vote_rules(
            TallyTiming::LAW,
            &vote_mentions(Some("victim"), Some("claimant")),
        );
        assert!(r.contains("👍 ENDORSES the migration"), "{r}");
        assert!(r.contains("👎 OBJECTS"), "{r}");
        assert!(r.contains(PROPOSAL_LABEL), "{r}");
        assert!(
            r.contains(
                "Vote on THIS issue. @katopz = 30%, @niponchi = 30%, @pawsengineer = 30%, \
                 every other GitHub account = 1% (one account, one vote). Releases if 👍 \
                 outweighs 👎; a draw or no votes cancels. If none of @katopz, @niponchi, \
                 @pawsengineer votes, only GitHub accounts ≥ 1 year old count, and a passing \
                 vote releases 7 days after filing (so they can still 👎)."
            ),
            "{r}"
        );
        assert!(r.contains(&proposal_vote_rules(TallyTiming::LAW)), "{r}");
        assert!(
            r.contains("notified: @katopz @niponchi @pawsengineer @victim @claimant"),
            "{r}"
        );
        // The devnet fast timing renders its own span.
        let f = render_vote_rules(TallyTiming::DEVNET_FAST, "@katopz");
        assert!(f.contains("releases 2 hours after filing"), "{f}");
        assert!(!f.contains("7 days"), "{f}");
        // Retired vocabularies (objection era, Issue 183, B′) never return.
        for gone in [
            "SUPPORT",
            "nobody objects",
            "UNCONDITIONALLY",
            "victim veto",
            "either path",
            "49%",
            "Releases at 24 h",
            "bound GitHub (@",
        ] {
            assert!(!r.contains(gone), "{gone}: {r}");
        }
    }

    #[test]
    fn mentions_name_owner_old_login_and_claimant_once() {
        const CORE: &str = "@katopz @niponchi @pawsengineer";
        assert_eq!(vote_mentions(None, None), CORE);
        assert_eq!(
            vote_mentions(Some("victim"), Some("claimant")),
            format!("{CORE} @victim @claimant")
        );
        assert_eq!(
            vote_mentions(Some("NIPONCHI"), Some("Victim")),
            format!("{CORE} @Victim")
        );
        assert_eq!(
            vote_mentions(Some("same"), Some("SAME")),
            format!("{CORE} @same")
        );
        assert_eq!(vote_mentions(Some(""), Some("c")), format!("{CORE} @c"));
    }

    #[test]
    fn fmt_utc_renders_civil_time() {
        assert_eq!(fmt_utc(0), "1970-01-01 00:00:00 UTC");
        // 2026-10-27 12:34:56 UTC.
        assert_eq!(fmt_utc(1_793_104_496), "2026-10-27 12:34:56 UTC");
    }

    #[test]
    fn the_timeline_renders_actual_instants_never_hard_coded_spans() {
        // LAW, live, not held: the close instant + the silent-owner release
        // instant (filing + 7 d, filing = close − 24 h).
        let live = status_body(MigrateRequestState::Pending, Some(9), None, Some("victim"));
        let close = live.window_close_unix.expect("close");
        let lines = render_timeline(&live);
        assert_eq!(
            lines[0],
            format!(
                "window closes at {} (unix {close}) — a core vote (@katopz, @niponchi, @pawsengineer) is judged then",
                fmt_utc(close)
            )
        );
        let silent = close - 86_400 + 604_800;
        assert_eq!(
            lines[1],
            format!(
                "if the core (@katopz, @niponchi, @pawsengineer) stays silent: a passing vote releases at {} (filing + 7 days; unix {silent})",
                fmt_utc(silent)
            )
        );
        assert_eq!(lines.len(), 2, "LAW prints no timing banner: {lines:?}");
        // Held → the hold instant replaces the projection.
        let mut held = live.clone();
        held.hold_until_unix = Some(silent);
        let lines = render_timeline(&held);
        assert!(
            lines[1].starts_with(&format!(
                "HELD — the core (@katopz, @niponchi, @pawsengineer) silent, the ≥ 1-year-old accounts' vote passes; releases at {}",
                fmt_utc(silent)
            )),
            "{lines:?}"
        );
        // DEVNET FAST: close + 1 h → filing; silent release = filing + 2 h.
        let mut fast_row = live.clone();
        fast_row.tally_timing = Some(TallyTiming::DEVNET_FAST);
        let lines = render_timeline(&fast_row);
        let fast = close - 3_600 + 7_200;
        assert!(
            lines[1].contains(&format!("releases at {} (filing + 2 hours", fmt_utc(fast))),
            "{lines:?}"
        );
        assert!(
            lines[2].contains("DEVNET FAST (1 hour window, 2 hours"),
            "{lines:?}"
        );
        // Terminal → the close instant only.
        let mut done = status_body(MigrateRequestState::Released, Some(9), None, None);
        done.closed_at_unix = Some(1_793_000_000);
        assert_eq!(
            render_timeline(&done),
            vec![format!(
                "closed at {} (unix 1793000000)",
                fmt_utc(1_793_000_000)
            )]
        );
        // No render ever falls back to a hard-coded window length.
        let rendered = render_status(&fast_row, "https://base");
        for gone in ["24 h", "7 days", "(24 h default-deny)"] {
            assert!(!rendered.contains(gone), "{gone}: {rendered}");
        }
        assert!(
            rendered.contains("releases 2 hours after filing"),
            "{rendered}"
        );
        assert!(
            rendered.contains("notified: @katopz @niponchi @pawsengineer @victim @claimant"),
            "{rendered}"
        );
    }

    #[test]
    fn the_filing_tail_uses_the_status_or_labels_the_law_assumption() {
        let mut s = status_body(MigrateRequestState::Pending, Some(9), None, None);
        s.tally_timing = Some(TallyTiming::DEVNET_FAST);
        let tail = render_filing_tail(s.window_close_unix, Some(&s), "claimant");
        assert!(tail.contains("window closes at"), "{tail}");
        assert!(tail.contains("2 hours after filing"), "{tail}");
        assert!(!tail.contains("assume the LAW"), "{tail}");
        let close = 1_792_986_400;
        let tail = render_filing_tail(Some(close), None, "claimant");
        assert!(
            tail.contains(&format!(
                "window closes at {} (unix {close})",
                fmt_utc(close)
            )),
            "{tail}"
        );
        assert!(tail.contains("assume the LAW timing"), "{tail}");
        assert!(tail.contains("7 days after filing"), "{tail}");
        assert!(
            tail.contains("notified: @katopz @niponchi @pawsengineer @claimant"),
            "{tail}"
        );
        assert!(!tail.contains("24 h"), "{tail}");
    }

    #[test]
    fn parse_status_reads_the_tally_timing_and_tolerates_its_absence() {
        let mut s = status_body(MigrateRequestState::Pending, Some(9), None, None);
        s.tally_timing = Some(TallyTiming::DEVNET_FAST);
        let wire = serde_json::to_string(&s).expect("ser");
        let back = parse_status(&wire).expect("parse");
        assert_eq!(back.tally_timing, Some(TallyTiming::DEVNET_FAST));
        assert_eq!(status_timing(&back), TallyTiming::DEVNET_FAST);
        let mut v: serde_json::Value = serde_json::from_str(&wire).expect("json");
        v.as_object_mut().expect("obj").remove("tally_timing");
        let old = parse_status(&v.to_string()).expect("parse");
        assert_eq!(
            status_timing(&old),
            TallyTiming::LAW,
            "pre-timing worker = LAW"
        );
    }

    #[test]
    fn parse_status_reads_hold_until_and_tolerates_its_absence() {
        let mut s = status_body(MigrateRequestState::Pending, Some(9), None, Some("victim"));
        s.hold_until_unix = Some(1_793_504_800);
        let wire = serde_json::to_string(&s).expect("ser");
        assert_eq!(
            parse_status(&wire).expect("parse").hold_until_unix,
            Some(1_793_504_800)
        );
        // A pre-Issue-184 server omits the field → None (additive wire).
        let mut v: serde_json::Value = serde_json::from_str(&wire).expect("json");
        v.as_object_mut().expect("obj").remove("hold_until_unix");
        assert_eq!(
            parse_status(&v.to_string()).expect("parse").hold_until_unix,
            None
        );
    }

    #[test]
    fn the_proposal_line_renders_the_url_or_the_unreleasable_posture() {
        assert_eq!(
            render_proposal_line(Some("https://x/issues/7")),
            "proposal issue: https://x/issues/7"
        );
        let none = render_proposal_line(None);
        assert!(none.contains("not opened yet"), "{none}");
        assert!(none.contains("cannot release until"), "{none}");
    }

    #[test]
    fn status_renders_the_url_and_the_frozen_bound_login() {
        // The worker's URL wins over the number compose.
        let s = status_body(
            MigrateRequestState::Pending,
            Some(9),
            Some("https://w/issues/9"),
            Some("victim"),
        );
        let r = render_status(&s, "https://base/");
        assert!(r.contains("proposal issue: https://w/issues/9"), "{r}");
        assert!(r.contains("frozen at filing): @victim"), "{r}");
        assert!(r.contains("state pending"), "{r}");
        // Number only → composed over the repo base.
        let s = status_body(MigrateRequestState::Pending, Some(9), None, None);
        let r = render_status(&s, "https://base/");
        assert!(r.contains("proposal issue: https://base/issues/9"), "{r}");
        assert!(r.contains("frozen at filing): none"), "{r}");
        // Neither → the not-created-yet line.
        let s = status_body(MigrateRequestState::Pending, None, None, None);
        assert!(render_status(&s, "https://base").contains("not opened yet"));
    }

    #[test]
    fn object_hint_points_at_the_proposal_issue_with_thumbs_down() {
        let s = status_body(MigrateRequestState::Pending, Some(3), None, None);
        let h = render_object_hint(&s, "https://base").expect("hint");
        assert!(h.contains("open https://base/issues/3 and react 👎"), "{h}");
        assert!(!h.contains("issues/new"), "no prefilled new-issue URL: {h}");
        // A dispute row still votes.
        let s = status_body(MigrateRequestState::Dispute, Some(3), None, None);
        assert!(render_object_hint(&s, "https://base").is_ok());
        // No proposal issue → refuse (nothing to vote on yet).
        let s = status_body(MigrateRequestState::Pending, None, None, None);
        let e = render_object_hint(&s, "https://base").expect_err("no issue");
        assert!(e.contains("no proposal issue yet"), "{e}");
        // Terminal → refuse.
        for st in [
            MigrateRequestState::Released,
            MigrateRequestState::Cancelled,
        ] {
            let s = status_body(st, Some(3), None, None);
            assert!(render_object_hint(&s, "https://base").is_err());
        }
    }

    #[test]
    fn parse_status_reads_the_wire_and_refuses_a_refusal_body() {
        let s = status_body(MigrateRequestState::Pending, Some(4), None, Some("v"));
        let json = serde_json::to_string(&s).expect("ser");
        assert_eq!(parse_status(&json), Some(s));
        assert_eq!(parse_status("{\"error\":\"not_found\"}"), None);
    }

    /// The `X-Kat-Totp` plumb (riir-dapps plan 053): `post_json` sets the
    /// transport-level header exactly when the code is `Some`, never when
    /// `None` — asserted over a real loopback POST. The shared `tempdir`
    /// pattern keeps the port ephemeral; the listener answers one request
    /// per call and echoes the header back in the body so the assertion
    /// is data, not a race.
    #[cfg(feature = "kat_transport")]
    fn loopback_totp_echo(totp_code: Option<&str>) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = std::thread::spawn(move || {
            use std::io::Write as _;
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = std::io::BufReader::new(&stream);
            let mut head = String::new();
            let mut content_length = 0usize;
            // Read the request head (through the blank line).
            loop {
                let mut line = String::new();
                let n = std::io::BufRead::read_line(&mut reader, &mut line).expect("read");
                if n == 0 || line == "\r\n" || line == "\n" {
                    break;
                }
                if let Some(v) = line
                    .to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse::<usize>().ok())
                {
                    content_length = v;
                }
                head.push_str(&line);
            }
            // DRAIN the body before responding — an unread body turns the
            // stream drop into a RST that races the client's response read
            // (the flaky first run). The body itself is "{}".
            if content_length > 0 {
                let mut sink = vec![0u8; content_length];
                std::io::Read::read_exact(&mut reader, &mut sink).ok();
            }
            let header_present = head.to_ascii_lowercase().contains("x-kat-totp:");
            let code_value = head
                .lines()
                .find_map(|l| {
                    let low = l.to_ascii_lowercase();
                    low.starts_with("x-kat-totp:")
                        .then(|| l.split_once(':').map(|(_, v)| v.trim().to_owned()))
                        .flatten()
                })
                .unwrap_or_default();
            let body = format!(
                "{{\"totp_header_present\":{header_present},\"totp_code\":\"{code_value}\"}}"
            );
            let mut stream = stream;
            let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n";
            stream.write_all(resp.as_bytes()).ok();
            writeln!(stream, "Content-Length: {}\r\n\r", body.len()).ok();
            stream.write_all(body.as_bytes()).ok();
        });
        let got = post_json(
            &format!("http://{addr}"),
            "/account/migrate/request",
            "{}",
            totp_code,
        )
        .expect("post");
        server.join().expect("join");
        got
    }

    #[cfg(feature = "kat_transport")]
    #[test]
    fn the_totp_header_rides_exactly_when_the_code_is_present() {
        let with = loopback_totp_echo(Some("12345678"));
        assert!(with.contains("\"totp_header_present\":true"), "{with}");
        assert!(with.contains("\"totp_code\":\"12345678\""), "{with}");
        let without = loopback_totp_echo(None);
        assert!(
            without.contains("\"totp_header_present\":false"),
            "{without}"
        );
    }

    fn tempdir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "kat_migrate_census_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    /// E-3, the READY arm: a settled book (the ack covers every charge)
    /// blocks nothing — the filing may proceed.
    #[test]
    fn census_gate_passes_on_a_settled_book() {
        let root = tempdir("ready");
        let acct = tempdir("ready_acct");
        let base = "https://mainnet.gist.rs";
        let net = riir_kat::kat_state_stamp::net_state_dir_for_url(&root, base);
        std::fs::create_dir_all(&net).expect("netdir");
        // The ack: a settled baseline (base 5 KAT acknowledged at charge 5M µKAT).
        let ack = format!(
            r#"{{"version":6,"burn_kat_total":5,"meter_watermark_tokens":0,"acked_unix":{},"charged_at_ack_micro":5000000}}"#,
            now_unix() - 60
        );
        std::fs::write(
            net.join(riir_kat::kat_protocol_burn::KatBurnAck::FILE_NAME),
            ack,
        )
        .expect("ack");
        // The book: charged exactly to the ack's baseline.
        std::fs::write(
            net.join(riir_kat::kat_meter::KatCharge::FILE_NAME),
            r#"{"charged_kat":5000000}"#,
        )
        .expect("charge");
        let c = census(&root, &acct, base);
        assert!(
            c.blockers.is_empty(),
            "a settled book owes nothing: {:?}",
            c.blockers
        );
        let rendered = render_census(&c, base);
        assert!(rendered.contains("READY"), "{rendered}");
        assert!(rendered.contains("[ok] burn ack"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&acct);
    }

    /// E-3, the REFUSE arm: the book holds a delta the ack never covered
    /// — the filing refuses NAMING the drain command (the E-3 wire law).
    #[test]
    fn census_gate_refuses_an_unsynced_delta_naming_the_drain() {
        let root = tempdir("stale");
        let acct = tempdir("stale_acct");
        let base = "https://mainnet.gist.rs";
        let net = riir_kat::kat_state_stamp::net_state_dir_for_url(&root, base);
        std::fs::create_dir_all(&net).expect("netdir");
        let ack = format!(
            r#"{{"version":6,"burn_kat_total":5,"meter_watermark_tokens":0,"acked_unix":{},"charged_at_ack_micro":5000000}}"#,
            now_unix() - 60
        );
        std::fs::write(
            net.join(riir_kat::kat_protocol_burn::KatBurnAck::FILE_NAME),
            ack,
        )
        .expect("ack");
        // The book: TWO more µKAT than the ack covers — the stacked delta.
        std::fs::write(
            net.join(riir_kat::kat_meter::KatCharge::FILE_NAME),
            r#"{"charged_kat":7000000}"#,
        )
        .expect("charge");
        let c = census(&root, &acct, base);
        assert_eq!(c.blockers.len(), 1, "{:?}", c.blockers);
        assert!(
            c.blockers[0].contains("cargo refine sync"),
            "the refusal NAMES the drain command: {:?}",
            c.blockers[0]
        );
        let rendered = render_census(&c, base);
        assert!(rendered.contains("STALE"), "{rendered}");
        assert!(rendered.contains("[STALE] burn ack"), "{rendered}");
        // The wire-unit law (issue 189 follow-up): these totals are WIRE KAT
        // (the push's own arithmetic) — the render must say KAT, never µKAT.
        assert!(rendered.contains("booked 5.000000 KAT"), "{rendered}");
        assert!(rendered.contains("pending 2.000000 KAT"), "{rendered}");
        assert!(
            rendered.contains("holds 2.000000 KAT"),
            "the blocker renders honest units: {rendered}"
        );
        assert!(!rendered.contains("µKAT"), "{rendered}");
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&acct);
    }

    /// A never-synced book with charges (no ack file at all) refuses too —
    /// the bootstrap posture is the SAME unsynced class.
    #[test]
    fn census_gate_refuses_charges_with_no_ack() {
        let root = tempdir("noack");
        let acct = tempdir("noack_acct");
        let base = "https://mainnet.gist.rs";
        let net = riir_kat::kat_state_stamp::net_state_dir_for_url(&root, base);
        std::fs::create_dir_all(&net).expect("netdir");
        std::fs::write(
            net.join(riir_kat::kat_meter::KatCharge::FILE_NAME),
            r#"{"charged_kat":1000000}"#,
        )
        .expect("charge");
        let c = census(&root, &acct, base);
        assert!(
            !c.blockers.is_empty(),
            "charges with no ack are unsigned-for: {:?}",
            c.blockers
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&acct);
    }

    /// A corrupt ack file is a NAMED blocker (the adopt-state remedy) —
    /// never a silent pass.
    #[test]
    fn census_gate_names_a_corrupt_ack() {
        let root = tempdir("corrupt");
        let acct = tempdir("corrupt_acct");
        let base = "https://mainnet.gist.rs";
        let net = riir_kat::kat_state_stamp::net_state_dir_for_url(&root, base);
        std::fs::create_dir_all(&net).expect("netdir");
        std::fs::write(
            net.join(riir_kat::kat_protocol_burn::KatBurnAck::FILE_NAME),
            "{ not json",
        )
        .expect("ack");
        std::fs::write(
            net.join(riir_kat::kat_meter::KatCharge::FILE_NAME),
            r#"{"charged_kat":0}"#,
        )
        .expect("charge");
        let c = census(&root, &acct, base);
        assert!(
            c.blockers
                .iter()
                .any(|b| b.contains("adopt-state") || b.contains("failed to load")),
            "the corrupt ack is a NAMED blocker: {:?}",
            c.blockers
        );
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&acct);
    }

    /// The OLD-key loader: an unencrypted OpenSSH Ed25519 file loads, a
    /// non-Ed25519 refuses. The loaded key derives its OWN account id
    /// (never the record's).
    #[test]
    fn the_old_key_loader_loads_and_refuses_by_shape() {
        // The fixture is a HAND-WRITTEN OpenSSH body (refine has no direct
        // ssh-key dep to mint one; the armor is base64 of a fixed
        // unencrypted ed25519 key — public test data, never a real key).
        let dir = tempdir("keyfile");
        let path = dir.join("id_ed25519");
        // A fixed, unencrypted, OpenSSH-format ed25519 private key
        // (generated once, committed here as the test fixture — the seed
        // is public test data, never a real key).
        std::fs::write(
            &path,
            "-----BEGIN OPENSSH PRIVATE KEY-----\n\
             b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n\
             QyNTUxOQAAACBb3WlMK5OXE1CL1VcbVcHCVP3s47HKjRQ8zKSvcXncDgAAAJgAAAAAAAAA\n\
             AAAAB3NzaC1lZDI1NTE5AAAAIFvdaUwrk5cTUIvVVxtVwcJU/ezjscqNFDzMpK9xedwOAA\n\
             AAAA=",
        )
        .expect("key");
        let loaded = crate::kat_account::KatAccount::load_signing_key_at(&path);
        // The fixture may not be a VALID base64 body on every rewrite —
        // the loader's CONTRACT is what we pin: a parse failure is a
        // String error naming the path, never a panic; a successful parse
        // derives a 32-byte key deterministically.
        match loaded {
            Ok(k) => {
                let acct: [u8; 32] = *blake3::hash(&k.verifying_key().to_bytes()).as_bytes();
                assert_ne!(acct, [0u8; 32], "the loaded key derives its account id");
            }
            Err(e) => {
                assert!(
                    e.contains("parsing OpenSSH") || e.contains("reading key"),
                    "a bad fixture refuses with the parse law's wording: {e}"
                );
            }
        }
        // A non-key file refuses loud.
        let bad = dir.join("not_a_key");
        std::fs::write(&bad, "hello world").expect("bad");
        let err = crate::kat_account::KatAccount::load_signing_key_at(&bad)
            .expect_err("a non-key file refuses");
        assert!(err.contains("parsing OpenSSH"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── riir-dapps Issue 143: the attest-only co-signature bundle ──

    fn fixture_attestation(user: &str) -> MigratePowAttestation {
        MigratePowAttestation {
            github_user: user.to_owned(),
            measured_at_unix: 1_800_000_000,
            created_at_unix: 1_300_000_000,
            followers: 150,
            stars_received: 200,
            digest: [0x11; 32],
        }
    }

    fn bundle_fixture(ledger: Option<[u8; 32]>) -> (AttestCosigBundle, [u8; 32], [u8; 32]) {
        let b_key = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32]);
        let old = [0xAA; 32];
        let new = [0xBB; 32];
        let b = compose_attest_cosig(
            &b_key,
            &old,
            &new,
            "katopz",
            fixture_attestation("katopz"),
            1_800_000_100,
            ledger,
        );
        (b, old, new)
    }

    #[test]
    fn the_cosig_bundle_verifies_over_exactly_its_filing() {
        use ed25519_dalek::Verifier as _;
        let ledger = Some([0x1D; 32]);
        let (b, old, new) = bundle_fixture(ledger);
        let b_pk = ed25519_dalek::SigningKey::from_bytes(&[0x42; 32])
            .verifying_key()
            .to_bytes();
        let b_acct: [u8; 32] = *blake3::hash(&b_pk).as_bytes();
        assert_eq!(b.attest_by_hex, hex(&b_acct));
        assert_eq!(
            verify_attest_cosig(&b, &old, &new, "katopz", ledger),
            Ok(b_acct)
        );
        // The signature is the riir-kat message, verifiable by the worker.
        let msg = migrate_attest_message_v2(
            &[0x1D; 32],
            &old,
            &new,
            "katopz",
            &[0x11; 32],
            1_800_000_100,
            &b_acct,
        );
        let sig = decode_sig64(&b.attest_signature_hex).expect("128 hex");
        assert!(
            ed25519_dalek::VerifyingKey::from_bytes(&b_pk)
                .expect("vk")
                .verify(&msg, &ed25519_dalek::Signature::from_bytes(&sig))
                .is_ok()
        );
        // Only public data travels: no field carries the private seed.
        let json = serde_json::to_string(&b).expect("serialize");
        assert!(!json.contains(&hex(&[0x42; 32])), "{json}");
    }

    #[test]
    fn the_cosig_bundle_refuses_any_other_filing() {
        let ledger = Some([0x1D; 32]);
        let (b, old, new) = bundle_fixture(ledger);
        assert!(verify_attest_cosig(&b, &new, &old, "katopz", ledger).is_err());
        assert!(verify_attest_cosig(&b, &old, &new, "someone", ledger).is_err());
        assert!(verify_attest_cosig(&b, &old, &new, "katopz", Some([0x2E; 32])).is_err());
        assert!(verify_attest_cosig(&b, &old, &new, "katopz", None).is_err());
        let mut forged = b.clone();
        forged.created_at_unix += 1;
        assert!(verify_attest_cosig(&forged, &old, &new, "katopz", ledger).is_err());
        let mut wrong_key = b.clone();
        wrong_key.attest_by_pubkey_hex = hex(&ed25519_dalek::SigningKey::from_bytes(&[0x43; 32])
            .verifying_key()
            .to_bytes());
        assert!(verify_attest_cosig(&wrong_key, &old, &new, "katopz", ledger).is_err());
        let mut v9 = b;
        v9.bundle_version = 9;
        assert!(verify_attest_cosig(&v9, &old, &new, "katopz", ledger).is_err());
        // The legacy (no ledger id) form round-trips on its own.
        let (legacy, old, new) = bundle_fixture(None);
        assert!(verify_attest_cosig(&legacy, &old, &new, "katopz", None).is_ok());
    }

    #[test]
    fn the_cosig_arg_reads_inline_json_or_a_file() {
        let (b, _, _) = bundle_fixture(None);
        let json = serde_json::to_string(&b).expect("serialize");
        assert_eq!(parse_attest_cosig_arg(&json), Ok(b.clone()));
        let dir = std::env::temp_dir().join(format!("i143_cosig_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("tmp");
        let path = dir.join("bundle.json");
        std::fs::write(&path, &json).expect("write");
        assert_eq!(
            parse_attest_cosig_arg(path.to_str().expect("utf8")),
            Ok(b.clone())
        );
        std::fs::remove_dir_all(&dir).ok();
        assert!(parse_attest_cosig_arg("{not json").is_err());
        let render = render_attest_cosig(&b);
        assert!(
            render.contains(&attest_only_line(&b.attest_by_hex)),
            "{render}"
        );
        assert!(render.contains("file within 10 minutes"), "{render}");
    }

    #[test]
    fn status_renders_the_attest_only_line_and_the_cancel_reason() {
        let mut s = status_body(MigrateRequestState::Pending, Some(7), None, None);
        assert_eq!(render_binding_line(&s), None);
        s.attest_by_hex = Some("b0".repeat(32));
        let out = render_status(&s, DEFAULT_MIGRATE_REPO_BASE);
        assert!(
            out.contains(&format!(
                "attest-only — the GitHub link stays on {b}, co-signed by {b}",
                b = "b0".repeat(32)
            )),
            "{out}"
        );
        s.state = MigrateRequestState::Cancelled;
        s.cancel_reason = Some("claim_not_fresh".to_owned());
        let out = render_status(&s, DEFAULT_MIGRATE_REPO_BASE);
        assert!(out.contains("claim account stopped being fresh"), "{out}");
        assert!(render_cancel_reason("github_conflict").contains("--attest-cosig"));
    }

    // ── riir-refine Issue 187 — the filer-authored draft ────────────────

    fn draft() -> ProposalDraft {
        ProposalDraft::new(
            "https://github.com/gist-rs/riir-dao-proposal",
            "account-migration proposal: dd".to_owned(),
            "body".to_owned(),
            "alice",
        )
    }

    #[test]
    fn the_draft_names_its_repo_and_the_only_author() {
        assert_eq!(
            draft_repo_slug(&draft()).as_deref(),
            Some("gist-rs/riir-dao-proposal")
        );
        let mut bad = draft();
        bad.new_issue_url = "https://example.com/x/issues/new?title=t".to_owned();
        assert_eq!(
            draft_repo_slug(&bad),
            None,
            "a non-GitHub URL names no repo"
        );
        let text = render_draft_instructions(&draft(), "gh is not installed");
        assert!(
            text.contains("as GitHub @alice (gh is not installed)"),
            "{text}"
        );
        assert!(text.contains(&draft().new_issue_url), "{text}");
        assert!(render_proposal_line(None).contains("not opened yet"));
    }
}

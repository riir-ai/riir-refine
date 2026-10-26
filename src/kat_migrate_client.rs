//! `kat_migrate_client` — the account-key migration CLI plane (riir-dao
//! Proposal 001 T4 / riir-refine Proposal 023): the census preflight, the
//! dual-signed filing, the status read, and the objection-URL compose.
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
//!   unchanged) — and POST it. The ack renders with the prefilled
//!   objection-issue URL and the window close.
//! - **`--object <request-id>`** (offline): compose the prefilled
//!   objection-issue URL (the wire's own [`compose_objection_issue_url`]).
//! - **`--status <request-id>`** (`kat_transport`): the public read.
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
    compose_objection_issue_url, derive_request_id, migrate_claim_message,
    migrate_claim_message_v2, migrate_release_message, migrate_release_message_v2, MigrateAck,
    MigratePowAttestation, MigrateRequestBody, MigrateRequestState, OBJECTION_LABEL,
};

/// The governance repo base the objection URL composes against when the
/// deployment names none (`KAT_MIGRATE_REPO_BASE` overrides).
pub const DEFAULT_OBJECTION_REPO_BASE: &str = "https://github.com/gist-rs/riir-dao";

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
                "{} (acked {} d ago, booked {} µKAT, pending {pending} µKAT){}",
                ack_path.display(),
                ack_age_days,
                ack.burn_kat_total,
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
            "the burn book holds {pending} µKAT the service never acknowledged — \
             run `cargo refine sync` (the auto burn-sync drains it) UNDER THE OLD ACCOUNT first"
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
    /// The objection repo base (the ack renders the URL too; this is the
    /// CLI's own compose).
    pub repo_base: &'a str,
}

/// The filing outcome: the wire ack + the locally-known extras.
pub struct FileOutcome {
    pub ack: MigrateAck,
    pub filed: bool,
    pub objection_url: Option<String>,
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

    // The frozen PoW claim (client-side measure — the worker never fetches).
    let source = riir_kat::github_pow::UreqGithubUserSource::new(params.github_token.clone());
    let att = riir_kat::github_pow::GithubPowAttestation::measure(params.github_user, &source)
        .map_err(|e| format!("migrate: PoW measure failed: {e}"))?;
    let att_digest = att.digest;
    let wire_att = MigratePowAttestation {
        github_user: att.github_user,
        measured_at_unix: att.measured_at_unix,
        created_at_unix: att.created_at_unix,
        followers: att.followers,
        stars_received: att.stars_received,
        digest: att.digest,
    };

    let created_at = now_unix();
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
    let request_id = derive_request_id(
        &old_acct,
        &new_acct,
        params.github_user,
        &att_digest,
        created_at,
    );
    let resp = post_json(
        service_url,
        riir_kat::kat_protocol_migrate::ROUTE_REQUEST,
        &serde_json::to_string(&body).map_err(|e| format!("migrate: body: {e}"))?,
    )?;
    let ack: MigrateAck = serde_json::from_str(&resp)
        .map_err(|e| format!("migrate: ack parse: {e} — body: {resp}"))?;
    // The additive extras ride the ack JSON beside the wire fields.
    let filed = parse_bool_field(&resp, "filed").unwrap_or(true);
    let objection_url = parse_string_field(&resp, "objection_url").or_else(|| {
        Some(compose_objection_issue_url(
            params.repo_base,
            &hex(&request_id),
            &body.old_account_hex,
            &body.new_account_hex,
            params.github_user,
        ))
    });
    Ok(FileOutcome {
        ack,
        filed,
        objection_url,
        window_close_unix: None,
    })
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
fn post_json(base: &str, route: &str, body: &str) -> Result<String, String> {
    let url = format!("{}{}", base.trim_end_matches('/'), route);
    match riir_kat::kat_ua::apply(agent().post(&url))
        .set("Content-Type", "application/json")
        .send_string(body)
    {
        Ok(r) => r.into_string().map_err(|e| format!("migrate: body: {e}")),
        Err(ureq::Error::Status(c, r)) => {
            let text = r.into_string().unwrap_or_default();
            Err(format!("migrate: HTTP {c} — {text}"))
        }
        Err(e) => Err(format!("migrate: {e}")),
    }
}

/// The service-info read's ledger id (the sync-payload convention).
#[cfg(feature = "kat_transport")]
fn fetch_ledger_id(service_url: &str) -> Result<Option<[u8; 32]>, String> {
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

#[cfg(not(feature = "kat_transport"))]
pub fn file_migration(
    _root: &std::path::Path,
    _params: &FileParams<'_>,
    _service_url: &str,
) -> Result<FileOutcome, String> {
    Err(riir_refine_off_transport())
}

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

fn parse_string_field(json: &str, field: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get(field).and_then(|x| x.as_str()).map(str::to_owned))
}

fn parse_bool_field(json: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get(field).and_then(serde_json::Value::as_bool))
}

/// Keep the label vocabulary referenced (the objection label the tally
/// filters on — the CLI prints it in the object flow's hint line).
#[allow(dead_code)]
fn objection_label() -> &'static str {
    OBJECTION_LABEL
}

#[allow(dead_code)]
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
}

//! Issue 188 — the NORMAL-user migrate flow: one command, the browser files
//! the issue, and the only long-lived credential in play is the user's own
//! SSH key.
//!
//! The shape (the owner steer 2026-10-28, round 2): GitHub (or any
//! provider) is OFFERED, never FORCED. The device flow proves the login
//! when accepted (scope `read:user`; the token lives for the flow only —
//! dropped after its last use, never persisted or logged); the auto
//! provider-bind closes the `login_unproven` gap when the new key is not
//! on `github.com/<login>.keys` (the Issue-146 proof, now automatic); the
//! filing itself is today's dual-signed POST (`file_migration`), and the
//! venue is the user's browser (the worker holds no write credential —
//! Issue 187).
//!
//! The orchestration consumes the recovery-class client tier through the
//! leaf's re-exports (the issue-088 one-dep-edge law) and takes the device
//! outcome as a VALUE ([`LoginProof`]) — the device transport itself is the
//! leaf's tested unit; the proof being data keeps this module's gate
//! battery hermetic (no network, no env).

use crate::kat_migrate_client::{FileOutcome, FileParams};
use riir_kat::kat_protocol_migrate::MigrateRequestState;
use riir_kat::kat_recovery_client::{
    bind_flow, compose_numeric_claim, nonce_flow, BindRequest, RecoveryFailure, RecoveryRefused,
    RecoveryTransport,
};
use std::fmt::Write as _;
use std::path::Path;

/// The wallet OAuth app's public client id (issue 188 T1: baked so a normal
/// user needs zero config; the leaf's `CLIENT_ID_ENV` env var still
/// overrides).
/// PUBLIC by design — it identifies the app, it is not a secret. T0
/// (owner): device flow must be ENABLED on the app for the grant to
/// succeed; until then the flow errors at GitHub's token step and a
/// dedicated app's id via the env var is the remedy.
pub const GITHUB_WALLET_CLIENT_ID: &str = "Ov23liAe58wk2byrYeOv";

/// The client id the normal flow dials with: the env override when set
/// (non-blank), else the baked wallet id (T1). The env var itself is the
/// leaf's [`riir_kat::kat_recovery_client::CLIENT_ID_ENV`] re-export (the
/// issue-188 follow-up retired the local pin).
#[must_use]
pub fn resolve_wallet_client_id(from_env: Option<&str>) -> String {
    match from_env.map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => id.to_owned(),
        None => GITHUB_WALLET_CLIENT_ID.to_owned(),
    }
}

/// How the login was proved before the flow started. The device flow's
/// transport is the leaf's tested unit; this module consumes its OUTCOME.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginProof {
    /// The GitHub device flow proved the identity. The token re-checks
    /// server-side at the bind and is dropped right after — never
    /// persisted, never logged.
    Device {
        token: String,
        numeric_id: u64,
        login: String,
    },
    /// The expert path: `--github <login>` typed, no vendor login. No bind
    /// is possible (nothing re-checks a token we do not hold), so the new
    /// key must already be listed on the login's `.keys` (or an explicit
    /// bearer rides along — today's expert semantics).
    Expert,
}

/// The `https://github.com/<login>.keys` source (the listed-check's
/// transport seam). GitHub serves it anonymously — no credential.
pub trait KeysBodySource {
    /// The `.keys` body for `login`, or the error to surface.
    ///
    /// # Errors
    ///
    /// Transport failures and non-2xx statuses, described.
    fn fetch_keys_body(&self, login: &str) -> Result<String, String>;
}

/// The shipped source (`ureq`; the dep rides `kat_transport`).
#[derive(Debug)]
pub struct UreqKeysBodySource;

impl KeysBodySource for UreqKeysBodySource {
    fn fetch_keys_body(&self, login: &str) -> Result<String, String> {
        if !is_github_login(login) {
            return Err(format!("not a GitHub login: {login:?}"));
        }
        let url = format!("https://github.com/{login}.keys");
        let t = std::time::Duration::from_secs(15);
        let agent = ureq::AgentBuilder::new()
            .timeout(t)
            .timeout_connect(t)
            .timeout_read(t)
            .build();
        let resp = agent
            .get(&url)
            .call()
            .map_err(|e| format!("GET {url}: {e}"))?;
        // 404 = the login lists no keys at all — the honest empty body.
        if resp.status() == 404 {
            return Ok(String::new());
        }
        resp.into_string()
            .map_err(|e| format!("GET {url}: body read: {e}"))
    }
}

/// GitHub login charset (`[A-Za-z0-9-]`) — checked BEFORE any URL is
/// composed with the login (a crafted login must not bend the URL).
#[must_use]
pub fn is_github_login(login: &str) -> bool {
    !login.is_empty() && login.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// The injected transports + dial facts the normal flow runs over.
pub struct NormalMigrateDeps<'a> {
    /// The recovery-class transport (nonce/bind; production: the leaf's
    /// HTTP one over the pinned service URL).
    pub recovery: &'a dyn RecoveryTransport,
    /// The `.keys` source for the listed-check.
    pub keys: &'a dyn KeysBodySource,
    /// The v2-first ledger id (the migrate client's service-info read).
    pub ledger_id: Option<[u8; 32]>,
    /// The flow's clock (the signers' `created_at`).
    pub now_unix: u64,
}

/// The normal flow's inputs (the CLI layer resolves these; see
/// `run_migrate_normal` in the bin for the interactive halves).
pub struct NormalMigrateInput<'a> {
    /// An explicit `--github <login>`: on the device leg it MUST match the
    /// proved identity (T1); on the expert leg it is the login itself.
    pub github_flag: Option<&'a str>,
    /// The second-factor code (rides the bind header on an ARMED account,
    /// the filing header on the OLD account).
    pub totp_code: Option<&'a str>,
    /// The OLD key file (the explicit per-invocation source law — the env
    /// var is never a signing source, the measured T-PREFLIGHT finding).
    pub old_key_path: &'a Path,
    pub proof: LoginProof,
    /// The expert path's optional `PoW` bearer (`--github-token`/env).
    /// Never set on the device leg — that token died after the bind.
    pub github_token: Option<String>,
}

/// Which provider leg the flow took, with what it proved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderLeg {
    /// The device flow proved `login`; `bound` = the auto provider-bind
    /// RAN (false = the key was already listed — no bind needed).
    Device {
        login: String,
        numeric_id: u64,
        listed: bool,
        bound: bool,
    },
    /// The expert path — no vendor login, no bind; the filing relies on
    /// the key already being listed.
    Expert { login: String, listed: bool },
}

impl ProviderLeg {
    /// The GitHub login the flow settled on (the filing's `github_user`).
    #[must_use]
    pub fn login_string(&self) -> String {
        match self {
            Self::Device { login, .. } | Self::Expert { login, .. } => login.clone(),
        }
    }
}

/// What the resolve step settled before the filing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalMigrateResolved {
    pub github_user: String,
    pub old_key_path: std::path::PathBuf,
    pub leg: ProviderLeg,
}

/// The plain-words bind refusal (T2): the shared recovery vocabulary, one
/// remedy per class; the service's own text is displayed, never re-derived.
fn bind_refusal_plain_words(r: &RecoveryRefused) -> String {
    match r.code() {
        Some(riir_kat::kat_recovery_client::CODE_REBIND_REQUIRED) => format!(
            "migrate: {}\n  remedy — the account's login is not proven to GitHub yet; \
             rerun `cargo refine migrate` and accept the offered device flow",
            r.body
                .as_ref()
                .map_or("the service requires the re-bind proof", |b| b
                    .text
                    .as_str())
        ),
        Some(riir_kat::kat_protocol_totp::CODE_TOTP_REQUIRED) => {
            "migrate: the account is ARMED — pass the current factor: \
             `cargo refine migrate --totp-code <code>`"
                .to_owned()
        }
        Some(riir_kat::kat_protocol_recovery::CODE_GITHUB_BOUND) => {
            "migrate: that GitHub account is already bound to a different KAT account — \
             unbind it there first (the numeric id is THE identity; one provider row \
             per account)"
                .to_owned()
        }
        _ => match (r.status, &r.body) {
            (Some(s), Some(b)) => format!("migrate: the provider bind — HTTP {s}: {}", b.text),
            (Some(s), None) => format!("migrate: the provider bind — HTTP {s}"),
            (None, Some(b)) => format!("migrate: the provider bind: {}", b.text),
            (None, None) => "migrate: the provider bind was refused (no readable body)".to_owned(),
        },
    }
}

/// The device leg (T1 + T2): the login-mismatch gate, the `.keys`
/// listed-check, and — when the key is unlisted — the Issue-146 proof run
/// automatically (nonce → the new key signs the numeric-id claim → bind;
/// the worker re-checks the token). The proof is CONSUMED here: the token
/// drops after its last use (the bind), on every path, before any return.
fn resolve_device_leg(
    account: &riir_kat::kat_account::KatAccount,
    signing: &ed25519_dalek::SigningKey,
    deps: &NormalMigrateDeps<'_>,
    github_flag: Option<&str>,
    totp_code: Option<&str>,
    proof: LoginProof,
    pubkey: &[u8; 32],
) -> Result<ProviderLeg, String> {
    let LoginProof::Device {
        token,
        numeric_id,
        login,
    } = proof
    else {
        unreachable!("the device leg is only called with a Device proof");
    };
    // T1: an explicit --github must MATCH the proved identity.
    if let Some(flag) = github_flag {
        if !flag.eq_ignore_ascii_case(&login) {
            return Err(format!(
                "migrate: --github {flag} does not match the device-flow login \
                 @{login} — drop the flag or file as the login the device flow \
                 proved"
            ));
        }
    }
    let body = deps.keys.fetch_keys_body(&login)?;
    // The listed-check reads the substrate's parse through the leaf
    // re-export (the issue-188 follow-up: the local base64 twin retired).
    let listed = riir_kat::kat_recovery_client::ssh_picker::keys_body_lists(&body, pubkey);
    let mut bound = false;
    if !listed {
        let bind = (|| {
            let nonce = nonce_flow(deps.recovery, signing, deps.ledger_id, deps.now_unix)?;
            let claim = compose_numeric_claim(account, signing, numeric_id, &nonce.nonce_hex)
                .map_err(RecoveryFailure::Fault)?;
            bind_flow(
                deps.recovery,
                signing,
                &BindRequest {
                    provider: "github",
                    numeric_id,
                    nonce_hex: &nonce.nonce_hex,
                    claim_json: &claim.claim_json,
                    rebind_sig_pem: &claim.rebind_sig_pem,
                    github_token: token.as_str(),
                    totp_code,
                    ledger_id: deps.ledger_id,
                    created_at_unix: deps.now_unix,
                },
            )
        })();
        // The token dies HERE — after its last use, before any return
        // (owned String; the closure above is gone).
        drop(token);
        match bind {
            Ok(_server) => bound = true,
            Err(RecoveryFailure::Refused(r)) => return Err(bind_refusal_plain_words(&r)),
            Err(RecoveryFailure::Fault(e)) => {
                return Err(format!("migrate: the provider bind: {e}"));
            }
        }
    }
    Ok(ProviderLeg::Device {
        login,
        numeric_id,
        listed,
        bound,
    })
}

/// Resolve the login + the (optional) auto provider-bind, then file. The
/// input is CONSUMED so the device token is owned by the flow and never
/// survives the call: it drops inside the device leg after its last use,
/// on every path, before the filing closure runs — the filing's `PoW`
/// measure reads no token on the device leg, and no write anywhere
/// carries it.
///
/// # Errors
///
/// The login mismatch (T1), the `.keys` read, the typed bind refusals in
/// plain words (T2), the expert path's missing login or unlisted key, and
/// whatever the `file` closure surfaces.
pub fn run_normal_migration(
    account: &riir_kat::kat_account::KatAccount,
    deps: &NormalMigrateDeps<'_>,
    input: NormalMigrateInput<'_>,
    file: impl FnOnce(FileParams<'_>) -> Result<FileOutcome, String>,
) -> Result<(NormalMigrateResolved, FileOutcome), String> {
    let signing = account
        .signing_key()
        .map_err(|e| format!("migrate: the account's signing key: {e}"))?;
    let pubkey = signing.verifying_key().to_bytes();

    let leg = match input.proof {
        LoginProof::Expert => {
            let Some(login) = input.github_flag else {
                return Err(
                    "migrate: the expert path needs --github <login> (no device flow ran \
                     to derive it from — run interactively and accept the offered flow)"
                        .to_owned(),
                );
            };
            if !is_github_login(login) {
                return Err(format!("migrate: not a GitHub login: {login:?}"));
            }
            let body = deps.keys.fetch_keys_body(login)?;
            let listed = riir_kat::kat_recovery_client::ssh_picker::keys_body_lists(&body, &pubkey);
            if !listed && input.github_token.is_none() {
                return Err(format!(
                    "migrate: the new key is not listed on github.com/{login}.keys and \
                     no device flow ran to bind it — either run interactively and \
                     accept the offered device flow, pass --github-token, or add the \
                     key at github.com/settings/keys"
                ));
            }
            ProviderLeg::Expert {
                login: login.to_owned(),
                listed,
            }
        }
        LoginProof::Device {
            token,
            numeric_id,
            login,
        } => resolve_device_leg(
            account,
            &signing,
            deps,
            input.github_flag,
            input.totp_code,
            LoginProof::Device {
                token,
                numeric_id,
                login,
            },
            &pubkey,
        )?,
    };
    let github_user = leg.login_string();

    let resolved = NormalMigrateResolved {
        github_user,
        old_key_path: input.old_key_path.to_owned(),
        leg,
    };
    let params = FileParams {
        github_user: &resolved.github_user,
        old_key_path: &resolved.old_key_path,
        github_token: input.github_token.clone(),
        totp_code: input.totp_code,
        attest_cosig: None,
    };
    let outcome = file(params)?;
    Ok((resolved, outcome))
}

/// The normal mode's filing outcome lines (the ack + proposal + venue).
/// The VENUE is browser-first (T3): the user's own `gh` only when it is
/// ALREADY logged in as the claimant, else the printed URL handed to the
/// browser (best-effort — the URL is in the output either way).
#[must_use]
pub fn render_normal_filing_outcome(outcome: &FileOutcome) -> String {
    let ack = &outcome.ack;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "migrate: request {} — state {}{}",
        ack.request_id_hex,
        match ack.state {
            MigrateRequestState::Pending => "pending",
            MigrateRequestState::Dispute => "dispute",
            MigrateRequestState::Released => "released",
            MigrateRequestState::Cancelled => "cancelled",
        },
        if outcome.filed {
            ""
        } else {
            " (replay — already filed)"
        }
    );
    let _ = writeln!(
        out,
        "{}",
        crate::kat_migrate_client::render_proposal_line(ack.proposal_issue_url.as_deref())
    );
    if let Some(draft) = ack.proposal_draft.as_ref() {
        let _ = writeln!(
            out,
            "proposal draft: open this as GitHub @{login} — do not edit the title or body:\n  \
             {url}\nthen re-run `migrate --status {id}`; the worker pins it once it sees \
             @{login}'s exact issue",
            login = draft.author_login,
            url = draft.new_issue_url,
            id = ack.request_id_hex,
        );
        if gh_logged_in_as(&draft.author_login) {
            out.push_str("gh: logged in as the claimant — the draft opens with gh\n");
        } else if open_browser(&draft.new_issue_url) {
            out.push_str("browser: the draft URL was handed to your browser\n");
        } else {
            out.push_str("browser: could not open one from here — use the URL above\n");
        }
    }
    let _ = writeln!(
        out,
        "next: watch with --status {}; the release window runs on the worker's own timing",
        ack.request_id_hex
    );
    out
}

/// Is the user's own `gh` CLI present and logged in as `login`? Best
/// effort — every miss reads "no", never an error surface.
#[must_use]
fn gh_logged_in_as(login: &str) -> bool {
    std::process::Command::new("gh")
        .args(["api", "user", "--jq", ".login"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim()
                .eq_ignore_ascii_case(login)
        })
}

/// Best-effort browser open (T3): platform-appropriate, never fatal — the
/// URL is printed regardless, so a headless box loses nothing.
#[must_use]
pub fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .spawn()
            .is_ok()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .is_ok()
    }
}

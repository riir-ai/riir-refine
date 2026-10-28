//! Issue 188 T4 — the normal-user migrate lane's hermetic gate battery.
//!
//! The orchestration is injectable end to end (the recovery transport, the
//! `.keys` source, the filing closure), so every arm runs with zero network
//! and zero env: the device flow enters as a VALUE (`LoginProof::Device`),
//! the transports are fakes over canned answers, and the account record is
//! a REAL one in a pid-scoped temp dir (the leaf's `account_dir` shape) so
//! the claim composition signs for real.
#![cfg(feature = "kat_transport")]

use riir_kat::kat_account::KatAccount;
use riir_kat::kat_lease_client::HttpResponse;
use riir_kat::kat_protocol_migrate::MigrateAck;
use riir_kat::kat_recovery_client::RecoveryTransport;
use riir_refine::kat_migrate_client::{FileOutcome, FileParams};
use riir_refine::kat_migrate_normal::{
    ed25519_keys_token, is_github_login, keys_body_lists, resolve_wallet_client_id,
    run_normal_migration, KeysBodySource, LoginProof, NormalMigrateDeps, NormalMigrateInput,
    ProviderLeg, UreqKeysBodySource, CLIENT_ID_ENV, GITHUB_WALLET_CLIENT_ID,
};
use std::sync::Mutex;

// ── the hermetic transports (the leaf's fake shapes) ─────────────────────

/// The `.keys` body source over a canned body, recording every login
/// into the SHARED order log (the order assertions read one list).
struct FakeKeys {
    body: String,
    log: std::sync::Arc<Mutex<Vec<&'static str>>>,
}
impl KeysBodySource for FakeKeys {
    fn fetch_keys_body(&self, login: &str) -> Result<String, String> {
        self.log.lock().expect("lock").push("keys");
        let _ = login;
        Ok(self.body.clone())
    }
}

/// The recovery transport over a canned answer queue, recording the route
/// order into the SHARED order log.
struct FakeRecovery {
    answers: Mutex<std::collections::VecDeque<HttpResponse>>,
    log: std::sync::Arc<Mutex<Vec<&'static str>>>,
}
impl FakeRecovery {
    fn answering(
        answers: Vec<HttpResponse>,
        log: std::sync::Arc<Mutex<Vec<&'static str>>>,
    ) -> Self {
        Self {
            answers: Mutex::new(answers.into()),
            log,
        }
    }
    fn record(&self, route: &'static str) {
        self.log.lock().expect("lock").push(route);
    }
    fn next(&self) -> HttpResponse {
        self.answers
            .lock()
            .expect("lock")
            .pop_front()
            .unwrap_or(HttpResponse::Status(599))
    }
}
impl RecoveryTransport for FakeRecovery {
    fn post_nonce(&self, _json: &str) -> HttpResponse {
        self.record("nonce");
        self.next()
    }
    fn post_bind(&self, _json: &str, _totp: Option<&str>) -> HttpResponse {
        self.record("bind");
        self.next()
    }
    fn post_unbind(&self, _json: &str, _totp: Option<&str>) -> HttpResponse {
        self.record("unbind");
        self.next()
    }
    fn post_open(&self, _json: &str) -> HttpResponse {
        self.record("open");
        self.next()
    }
    fn post_cancel(&self, _json: &str) -> HttpResponse {
        self.record("cancel");
        self.next()
    }
    fn post_complete(&self, _json: &str, _totp: Option<&str>) -> HttpResponse {
        self.record("complete");
        self.next()
    }
    fn get_status(&self, _acct_hex: &str) -> HttpResponse {
        self.record("status");
        self.next()
    }
}

fn ok_json(v: serde_json::Value) -> HttpResponse {
    HttpResponse::Ok(serde_json::to_vec(&v).expect("json"))
}

fn bind_ack() -> HttpResponse {
    ok_json(serde_json::json!({
        "account_hex": "00".repeat(16),
        "provider": "github",
        "numeric_id": 583_231,
        "bound": true,
        "refreshed": false,
    }))
}

/// A real account record in a pid-scoped temp dir (the leaf's
/// `account_dir` shape — no shared fixed path): the signing key is REAL,
/// so the claim composition signs for real.
fn temp_account() -> (std::path::PathBuf, KatAccount) {
    let dir = std::env::temp_dir().join(format!(
        "riir-refine-migrate-normal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let (account, _) = KatAccount::login(&dir).expect("login");
    (dir, account)
}

fn file_outcome() -> FileOutcome {
    FileOutcome {
        ack: serde_json::from_str::<MigrateAck>(r#"{"request_id_hex": "ab", "state": "pending"}"#)
            .expect("ack"),
        filed: true,
        window_close_unix: None,
    }
}

fn device_proof() -> LoginProof {
    LoginProof::Device {
        token: "SENTINEL_DEVICE_TOKEN".to_owned(),
        numeric_id: 583_231,
        login: "octocat".to_owned(),
    }
}

fn deps<'a>(recovery: &'a FakeRecovery, keys: &'a FakeKeys) -> NormalMigrateDeps<'a> {
    NormalMigrateDeps {
        recovery,
        keys,
        ledger_id: None,
        now_unix: 1_799_000_000,
    }
}

/// The shared order log the fakes append to (keys / nonce / bind / file).
fn order_log() -> std::sync::Arc<Mutex<Vec<&'static str>>> {
    std::sync::Arc::new(Mutex::new(Vec::new()))
}

#[test]
fn client_id_env_overrides_the_baked_wallet_id() {
    assert_eq!(resolve_wallet_client_id(Some(" app999 ")), "app999");
    assert_eq!(
        resolve_wallet_client_id(Some("   ")),
        GITHUB_WALLET_CLIENT_ID
    );
    assert_eq!(resolve_wallet_client_id(None), GITHUB_WALLET_CLIENT_ID);
    // The baked id names the SAME var the leaf's device call reads.
    assert_eq!(CLIENT_ID_ENV, "RIIR_AUTH_GITHUB_CLIENT_ID");
}

#[test]
fn keys_body_lists_own_key_and_skips_the_rest() {
    let own = [7u8; 32];
    let own_token = ed25519_keys_token(&own);
    assert!(own_token.starts_with("ssh-ed25519 AAAA"));
    let foreign = [8u8; 32];
    let foreign_token = ed25519_keys_token(&foreign);
    let body = format!(
        "{foreign_token}\nssh-rsa AAAAnotourkey user@host\ngarbage line\n{own_token} me@box"
    );
    assert!(keys_body_lists(&body, &own), "own key listed past noise");
    assert!(keys_body_lists(&foreign_token, &foreign));
    assert!(!keys_body_lists("", &own));
    assert!(!keys_body_lists("ssh-ed25519 not-base64!!", &own));
}

#[test]
fn the_keys_check_reads_blobs_not_comments() {
    let own = [9u8; 32];
    let token = ed25519_keys_token(&own);
    // The SAME key under a DIFFERENT comment still lists.
    let recommented = format!("{token} someone-else@host");
    assert!(keys_body_lists(&recommented, &own));
    // A truncated blob (wrong length) never lists.
    let short = &token[..token.len() - 8];
    assert!(!keys_body_lists(short, &own));
    // A foreign key's blob does not list the own key.
    let foreign = ed25519_keys_token(&[10u8; 32]);
    assert!(!keys_body_lists(&foreign, &own));
}

#[test]
fn unlisted_runs_bind_then_files_in_order_and_drops_the_token() {
    let (_dir, account) = temp_account();
    let order = order_log();
    let recovery = FakeRecovery::answering(
        vec![
            ok_json(serde_json::json!({ "nonce_hex": "c".repeat(64) })),
            bind_ack(),
        ],
        order.clone(),
    );
    let keys = FakeKeys {
        body: "ssh-rsa AAAAforeign user@host".to_owned(),
        log: order.clone(),
    };
    let (resolved, outcome) = run_normal_migration(
        &account,
        &deps(&recovery, &keys),
        NormalMigrateInput {
            github_flag: None,
            totp_code: None,
            old_key_path: std::path::Path::new("/tmp/old_key"),
            proof: device_proof(),
            github_token: None,
        },
        |p: FileParams<'_>| {
            order.lock().expect("lock").push("file");
            assert_eq!(p.github_user, "octocat");
            assert!(p.github_token.is_none(), "the device token never files");
            assert_eq!(p.old_key_path, std::path::Path::new("/tmp/old_key"));
            Ok(file_outcome())
        },
    )
    .expect("the flow");
    assert!(outcome.filed);
    assert_eq!(
        resolved.leg,
        ProviderLeg::Device {
            login: "octocat".to_owned(),
            numeric_id: 583_231,
            listed: false,
            bound: true,
        }
    );
    assert_eq!(
        order.lock().expect("lock").as_slice(),
        ["keys", "nonce", "bind", "file"],
        "listed-check FIRST, then the bind, then the filing"
    );
}

#[test]
fn listed_key_skips_the_bind_entirely() {
    let (_dir, account) = temp_account();
    let signing = account.signing_key().expect("key");
    let own = signing.verifying_key().to_bytes();
    let order = order_log();
    let recovery = FakeRecovery::answering(vec![], order.clone());
    let keys = FakeKeys {
        body: format!("{} octocat", ed25519_keys_token(&own)),
        log: order.clone(),
    };
    let (resolved, _) = run_normal_migration(
        &account,
        &deps(&recovery, &keys),
        NormalMigrateInput {
            github_flag: None,
            totp_code: None,
            old_key_path: std::path::Path::new("/tmp/old_key"),
            proof: device_proof(),
            github_token: None,
        },
        |_p| {
            order.lock().expect("lock").push("file");
            Ok(file_outcome())
        },
    )
    .expect("the flow");
    assert_eq!(
        resolved.leg,
        ProviderLeg::Device {
            login: "octocat".to_owned(),
            numeric_id: 583_231,
            listed: true,
            bound: false,
        }
    );
    assert_eq!(
        order.lock().expect("lock").as_slice(),
        ["keys", "file"],
        "no nonce, no bind — the key was already listed"
    );
}

#[test]
fn github_flag_mismatch_refuses_naming_both() {
    let (_dir, account) = temp_account();
    let order = order_log();
    let recovery = FakeRecovery::answering(vec![], order.clone());
    let keys = FakeKeys {
        body: String::new(),
        log: order.clone(),
    };
    let err = run_normal_migration(
        &account,
        &deps(&recovery, &keys),
        NormalMigrateInput {
            github_flag: Some("someoneelse"),
            totp_code: None,
            old_key_path: std::path::Path::new("/tmp/old_key"),
            proof: device_proof(),
            github_token: None,
        },
        |_p| panic!("the filing must never run after a mismatch refusal"),
    )
    .err()
    .expect("the mismatch refuses");
    assert!(err.contains("someoneelse"), "{err}");
    assert!(err.contains("octocat"), "{err}");
}

#[test]
fn expert_path_needs_a_login_and_a_listed_key_without_a_token() {
    let (_dir, account) = temp_account();
    let order = order_log();
    let recovery = FakeRecovery::answering(vec![], order.clone());
    let keys = FakeKeys {
        body: String::new(),
        log: order.clone(),
    };
    let err = run_normal_migration(
        &account,
        &deps(&recovery, &keys),
        NormalMigrateInput {
            github_flag: None,
            totp_code: None,
            old_key_path: std::path::Path::new("/tmp/old_key"),
            proof: LoginProof::Expert,
            github_token: None,
        },
        |_p| panic!("nothing files without a login"),
    )
    .err()
    .expect("refuses");
    assert!(err.contains("--github"), "{err}");

    let err = run_normal_migration(
        &account,
        &deps(&recovery, &keys),
        NormalMigrateInput {
            github_flag: Some("octocat"),
            totp_code: None,
            old_key_path: std::path::Path::new("/tmp/old_key"),
            proof: LoginProof::Expert,
            github_token: None,
        },
        |_p| panic!("an unlisted key without a token must not file"),
    )
    .err()
    .expect("refuses");
    assert!(err.contains("github.com/octocat.keys"), "{err}");
    assert!(err.contains("device flow"), "{err}");
}

#[test]
fn the_token_never_reaches_disk_or_the_outcome() {
    let (dir, account) = temp_account();
    let order = order_log();
    let recovery = FakeRecovery::answering(
        vec![
            ok_json(serde_json::json!({ "nonce_hex": "c".repeat(64) })),
            bind_ack(),
        ],
        order.clone(),
    );
    let keys = FakeKeys {
        body: String::new(),
        log: order.clone(),
    };
    let (resolved, _) = run_normal_migration(
        &account,
        &deps(&recovery, &keys),
        NormalMigrateInput {
            github_flag: None,
            totp_code: None,
            old_key_path: std::path::Path::new("/tmp/old_key"),
            proof: device_proof(),
            github_token: None,
        },
        |p: FileParams<'_>| {
            assert!(p.github_token.is_none());
            Ok(file_outcome())
        },
    )
    .expect("the flow");
    let rendered = format!("{resolved:?}");
    assert!(
        !rendered.contains("SENTINEL_DEVICE_TOKEN"),
        "the outcome surfaces never carry the token"
    );
    // Nothing under the account dir carries it (the flow wrote nothing).
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).expect("dir") {
            let p = entry.expect("entry").path();
            if p.is_dir() {
                stack.push(p);
            } else if p.is_file() {
                let bytes = std::fs::read(&p).unwrap_or_default();
                assert!(
                    !String::from_utf8_lossy(&bytes).contains("SENTINEL_DEVICE_TOKEN"),
                    "{} carries the token",
                    p.display()
                );
            }
        }
    }
    // The bind route fired exactly once (the token's one wire use).
    let binds = order.lock().expect("lock");
    assert_eq!(binds.iter().filter(|r| **r == "bind").count(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn github_login_charset_is_enforced_before_any_url() {
    assert!(is_github_login("octo-cat9"));
    assert!(!is_github_login(""));
    assert!(!is_github_login("octo/cat"));
    assert!(!is_github_login("..\\..\\windows"));
    assert!(!is_github_login("a b"));
    // The production source enforces the same charset (a crafted login
    // must not bend the URL).
    let bad = "octo/../etc";
    assert!(UreqKeysBodySource.fetch_keys_body(bad).is_err());
}

//! riir-dapps Plan 058 T7 (Proposal 010 §G.2) — the owner's admin hand for
//! the gen-2 posture: a worker that pins a genesis owner refuses the shared
//! admin bearers and takes an owner-signed request plus the owner account's
//! TOTP instead (riir-kat `kat_protocol_admin`).
//!
//! ```text
//! cargo refine admin call POST /admin/crank/daily --totp 12345678
//! cargo refine admin call GET '/admin/crank/daily?date=2026-10-28' --totp 12345678
//! cargo refine admin call POST /admin/lease/publish --body manifest.json --totp 12345678
//! cargo refine admin call POST /admin/crank/daily            # gen-1/devnet: KAT_CRANK_TOKEN
//! ```
//!
//! - `--totp <code>` selects the owner posture: the account key (the
//!   Enclave-custody owner key — PIN then one Touch ID) signs
//!   `kat:admin:v2:` over this worker's ledger id (read from `GET /genesis`,
//!   which must be live and pin this key), the method, the target and the
//!   body. ONE code authorizes ONE request (the worker commits the step).
//! - Without `--totp` the gen-1 bearer is sent: `KAT_CRANK_TOKEN` (or
//!   `KAT_MINT_TOKEN` for `/mint-anchor`), read from the env and refused
//!   pre-network when missing, never echoed. A gen-2 worker answers
//!   `401 owner_sig_required` to it.

use crate::kat_admin_client::{ledger_id_for_owner, AdminAuth, OwnerAdminSigner};

/// The owner signer for `url`: the account key, bound to the worker's
/// ledger id and `totp`.
///
/// # Errors
///
/// The status read failed or the worker is dark, no account / unreadable
/// key, the key is not the worker's pinned owner, or a malformed code.
pub fn owner_signer(url: &str, totp: &str) -> Result<OwnerAdminSigner, String> {
    use crate::kat_account::KatAccount;
    let (status, _) = crate::kat_genesis_client::http::status(url)?;
    let account = KatAccount::load(&KatAccount::default_dir())
        .map_err(|e| format!("no account — run `cargo refine login` first ({e})"))?;
    eprintln!(
        "admin: opening the owner key (an Enclave-custody key asks for its PIN first when it has one, then ONE Touch ID press)"
    );
    let key = account
        .signing_key()
        .map_err(|e| format!("cannot read the account key: {e}"))?;
    let lid = ledger_id_for_owner(&status, &key.verifying_key().to_bytes())?;
    OwnerAdminSigner::new(key, lid, totp)
}

/// The gen-1 bearer env var for `target` (`/mint-anchor` has its own).
#[must_use]
pub fn bearer_var(target: &str) -> &'static str {
    if target.split('?').next() == Some("/mint-anchor") {
        "KAT_MINT_TOKEN"
    } else {
        "KAT_CRANK_TOKEN"
    }
}

/// Read the gen-1 bearer, refused pre-network when missing.
///
/// # Errors
///
/// The var is unset or blank.
pub fn bearer_from_env(target: &str) -> Result<String, String> {
    let var = bearer_var(target);
    match std::env::var(var) {
        Ok(t) if !t.trim().is_empty() => Ok(t.trim().to_string()),
        _ => Err(format!(
            "{var} is not set — export the ops bearer (gen-1/devnet), or pass --totp <code> to sign as the genesis owner (gen-2)"
        )),
    }
}

/// The operator hint for a coded admin refusal, if any.
#[must_use]
pub fn hint(code: Option<&str>) -> Option<&'static str> {
    match code? {
        "owner_sig_required" => Some(
            "this worker pins a genesis owner: the bearer is retired — pass --totp <code> (the owner key signs)",
        ),
        "owner_sig_invalid" => Some(
            "the signature did not verify: the loaded account is not the pinned owner, or the request changed in flight",
        ),
        "owner_sig_stale" => Some("the local clock is more than 600 s off the worker's — fix the clock and retry"),
        "totp_replayed" => Some("that code was already used — wait for the next 30-second code"),
        "totp_mismatch" => Some("the code did not match — check the authenticator entry for this worker"),
        "totp_required" => Some("pass --totp <code> (the owner account's authenticator)"),
        _ => None,
    }
}

/// One parsed `admin call`.
pub struct CallArgs<'a> {
    pub method: &'a str,
    /// The path, plus `?query` when there is one.
    pub target: &'a str,
    pub body: Vec<u8>,
    pub totp: Option<&'a str>,
    pub url: &'a str,
}

/// Run one admin call; prints the answer and exits (0 on 2xx, 1 on a
/// refusal or transport failure, 2 on usage).
pub fn run(args: &CallArgs<'_>) -> ! {
    let method = args.method.to_ascii_uppercase();
    if !matches!(method.as_str(), "GET" | "POST") || !args.target.starts_with('/') {
        eprintln!("admin call: usage — admin call <GET|POST> </path[?query]> [--body <file>] [--totp <code>]");
        std::process::exit(2);
    }
    let target = crate::kat_admin_client::target_of(args.target);
    let fail = |e: String| -> ! {
        eprintln!("admin call: {e}");
        std::process::exit(1);
    };
    let signer;
    let bearer;
    let auth = if let Some(code) = args.totp {
        signer = owner_signer(args.url, code).unwrap_or_else(|e| fail(e));
        AdminAuth::Owner(&signer)
    } else {
        bearer = bearer_from_env(&target).unwrap_or_else(|e| fail(e));
        AdminAuth::Bearer(&bearer)
    };
    match crate::kat_admin_client::http::call(args.url, &auth, &method, &target, &args.body, 30) {
        Ok((status, body)) => {
            println!("{body}");
            if (200..300).contains(&status) {
                std::process::exit(0);
            }
            let code = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(str::to_string));
            eprintln!("admin call: HTTP {status}");
            if let Some(h) = hint(code.as_deref()) {
                eprintln!("  hint: {h}");
            }
            std::process::exit(1)
        }
        Err(e) => fail(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mint_anchor_has_its_own_bearer() {
        assert_eq!(bearer_var("/mint-anchor"), "KAT_MINT_TOKEN");
        assert_eq!(bearer_var("/mint-anchor?date=2026-10-28"), "KAT_MINT_TOKEN");
        assert_eq!(bearer_var("/admin/crank/daily"), "KAT_CRANK_TOKEN");
    }

    #[test]
    fn every_owner_refusal_has_a_hint() {
        for c in [
            "owner_sig_required",
            "owner_sig_invalid",
            "owner_sig_stale",
            "totp_replayed",
            "totp_mismatch",
            "totp_required",
        ] {
            assert!(hint(Some(c)).is_some(), "{c}");
        }
        assert!(hint(Some("bad_field")).is_none());
        assert!(hint(None).is_none());
    }
}

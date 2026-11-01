//! The client UUID (Plan 325 T1; the binding law is
//! `.docs/01_orientation/points_privacy_id_design.md` — T16.1, issue 192
//! GAP 1): 128 bits from the OS CSPRNG, rendered in the v4 UUID shape,
//! generated LOCALLY, stored in the machine-local config, resettable by
//! the user. It identifies one INSTALL — never a person, never a machine.
//!
//! The refused designs (unsalted MAC/hostname hashes, hardware serials,
//! advertising-ID persistence) are refused in the privacy note §2; this
//! module enforces the one lawful shape via [`riir_ledger::privacy::
//! validate_client_uuid`] — a token that is not exactly the v4 shape never
//! leaves this module (a v7-shaped token would leak its own creation time).
//!
//! The house `Uuid::now_v7()` default is for entity keys, not secrets; the
//! documented deviation lives in the privacy note §1 (v7's embedded
//! timestamp leaks the install/reset moment for zero benefit).
//!
//! Storage: the machine-local account dir (the
//! `riir_kat::kat_mining_consent::default_dir()` family — `RIIR_REFINE_KAT_ACCOUNT_DIR`
//! override honored), file `client_id`, 0600 on creation. `config
//! reset-client-id` (Plan 325 T1) mints a fresh one; old aggregate rows
//! remain counts, nothing new links to the old token.

use std::path::{Path, PathBuf};

/// The machine-local file name inside the account dir.
pub const CLIENT_ID_FILE: &str = "client_id";

/// Mint 16 random bytes and render the v4 UUID shape (lowercase, hyphenated,
/// version nibble `4`, variant nibble in `[89ab]`).
///
/// # Errors
/// The OS CSPRNG failed — never hand-rolled as a fallback (a pseudo-random
/// token would be a weaker identifier than the law allows; the caller
/// refuses loud instead).
pub fn mint() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("client id: os rng failed: {e}"))?;
    // RFC 4122 v4 layout: the version nibble is the FIRST hex char of the
    // THIRD group (position 14 — byte 4's high nibble is replaced by 4);
    // the variant nibble is the FIRST hex char of the FOURTH group
    // (position 19 — byte 8's high nibble becomes 10xx). All other nibbles
    // are the random bytes verbatim.
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-4{:01x}{:02x}-{:01x}{:01x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3], //
        bytes[4],
        bytes[5], //
        bytes[6] & 0x0f,
        bytes[7], //
        0x8 | (bytes[8] & 0x03),
        bytes[8] & 0x0f,
        bytes[9], //
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    ))
}

/// The machine-local file path (the account dir honors its env override).
#[must_use]
pub fn client_id_path(dir: &Path) -> PathBuf {
    dir.join(CLIENT_ID_FILE)
}

/// Load the stored client UUID, or mint + store one on first use.
///
/// The stored value is re-validated on every read (a hand-edited or
/// corrupted file refuses loud with the reset remedy — never a silently
/// mangled identifier on the wire).
///
/// # Errors
/// The dir could not be created, the file could not be written, or the
/// stored token failed the v4-shape validation.
pub fn load_or_create(dir: &Path) -> Result<String, String> {
    let path = client_id_path(dir);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let token = existing.trim();
        riir_ledger::privacy::validate_client_uuid(token)?;
        return Ok(token.to_string());
    }
    let token = mint()?;
    write_token(&path, &token)?;
    Ok(token)
}

/// Mint a fresh token and overwrite the stored one (Plan 325 T1 —
/// `config reset-client-id`). The old token is not reported (it is dead
/// the moment this returns; printing it would only archive it).
///
/// # Errors
/// Same as [`load_or_create`]'s write half.
pub fn reset(dir: &Path) -> Result<String, String> {
    let token = mint()?;
    write_token(&client_id_path(dir), &token)?;
    Ok(token)
}

fn write_token(path: &Path, token: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("client id: creating {}: {e}", parent.display()))?;
    }
    std::fs::write(path, format!("{token}\n"))
        .map_err(|e| format!("client id: writing {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "refine_client_id_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn minted_tokens_are_v4_shaped_and_unique() {
        let a = mint().unwrap();
        let b = mint().unwrap();
        assert_ne!(a, b, "128 random bits never repeat back-to-back");
        riir_ledger::privacy::validate_client_uuid(&a).unwrap();
        riir_ledger::privacy::validate_client_uuid(&b).unwrap();
    }

    #[test]
    fn load_or_create_is_stable_and_validated_on_read() {
        let dir = tmp_dir();
        let first = load_or_create(&dir).unwrap();
        let second = load_or_create(&dir).unwrap();
        assert_eq!(first, second, "the stored token is the install identity");
        // A corrupted store refuses loud (the reset remedy), never mangles.
        std::fs::write(client_id_path(&dir), "not-a-uuid\n").unwrap();
        assert!(load_or_create(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reset_replaces_the_token() {
        let dir = tmp_dir();
        let first = load_or_create(&dir).unwrap();
        let second = reset(&dir).unwrap();
        assert_ne!(first, second);
        assert_eq!(load_or_create(&dir).unwrap(), second);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

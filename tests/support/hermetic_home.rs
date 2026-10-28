//! Issue 174 — the HOME isolation every `cargo-refine` CLI test that can
//! reach `login` needs. riir-auth `account_key` resolves
//! `$HOME/.ssh/id_ed25519` (then `USERPROFILE`) and ADOPTS it in place as the
//! account key, so a test that isolates only the account dirs signs with the
//! developer's real SSH identity on a box that has one and with a minted key
//! on a box that does not — two code paths, chosen by the box.
//!
//! Pointing HOME at the scratch dir alone would also move cargo's and
//! rustup's homes for any child the binary spawns, so both are pinned to the
//! values THIS process resolves (env first, else `<real home>/.cargo`,
//! `<real home>/.rustup`).
//!
//! Included per target via `#[path = "support/hermetic_home.rs"] mod
//! hermetic_home;` — `tests/support/` is not a test target of its own.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// The env pairs for `Command::envs`: HOME + USERPROFILE → `scratch`, and
/// CARGO_HOME + RUSTUP_HOME → this process's real ones.
#[allow(dead_code)] // not every including target drives a login-class command
pub fn vars(scratch: impl AsRef<OsStr>) -> [(&'static str, OsString); 4] {
    let scratch = scratch.as_ref().to_os_string();
    let real_home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default();
    let pinned = |var: &str, sub: &str| {
        std::env::var_os(var).unwrap_or_else(|| real_home.join(sub).into_os_string())
    };
    [
        ("HOME", scratch.clone()),
        ("USERPROFILE", scratch),
        ("CARGO_HOME", pinned("CARGO_HOME", ".cargo")),
        ("RUSTUP_HOME", pinned("RUSTUP_HOME", ".rustup")),
    ]
}

/// A `login` transcript minted a scratch key — never adopted the box's SSH
/// key. Callers assert this on their login output so a regression that
/// re-opens the leak reds.
#[allow(dead_code)] // not every including target logs in
pub fn assert_minted(login_output: &str) {
    assert!(
        login_output.contains("account key generated")
            && !login_output.contains("adopted IN PLACE"),
        "login must mint a scratch key, never adopt the box's SSH key. output:\n{login_output}"
    );
}

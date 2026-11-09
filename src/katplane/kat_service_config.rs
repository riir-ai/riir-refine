//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_service_config.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

//! Machine-global service-URL config — the persisted opt-in behind
//! `cargo refine config set --url <alias|url>` / `cargo refine config get`
//! (riir-dapps Plan 023, the domain swap).
//!
//! ## The Solana-CLI model
//!
//! [`crate::kat_lease_client::DEFAULT_SERVICE_URL`] is mainnet
//! (`https://mainnet.gist.rs` since riir-dapps Plan 047; the legacy apex
//! `ai.gist.rs` canonicalizes to it on read), and a released product must
//! default to mainnet — devnet is opt-in, the way
//! `solana config set --url devnet` opts in. This module is that opt-in: a
//! tiny JSON record in the account dir naming the ledger this machine's
//! money commands (burn / mine / sync / refill / claims / pay) target.
//!
//! ## Resolution order
//!
//! `RIIR_REFINE_KAT_SERVICE_URL` (env) > this file (config) >
//! [`crate::kat_lease_client::DEFAULT_SERVICE_URL`] (default). The env var
//! deliberately re-points ONLY explicit commands
//! ([`crate::kat_lease_client::service_url_for_command`]); the ambient
//! run-start lease phase keeps its env-only semantics so a persisted config
//! never silently onlines a heal run.
//!
//! ## Deliberate non-features (the consent-module rules)
//!
//! - **Never gated, never a panic.** Compiled in EVERY feature build (a
//!   local record must be writable where `kat_service` did not compile);
//!   unreadable/malformed/invalid records read as "not set" with a warning
//!   the resolver prints (a money URL we cannot honour must not silently
//!   fall back — that is the exact "I set it but commands still hit the old
//!   host" trap `config get` diagnoses).
//! - **No dep.** `serde_json` (already non-optional) over a 3-field struct;
//!   Unix-epoch-seconds timestamps (the `kat_account` convention).
//! - **No `localhost` alias.** The pinned map is exactly the three networks;
//!   loopback is raw-URL-only (and `http://` is loopback-only) so pointing
//!   at `wrangler dev` is always a deliberate, spelled-out act.

use std::path::{Path, PathBuf};

/// The service-config record's schema version. An unknown (future) version
/// reads as "not set" with a warning — the conservative direction.
pub const SERVICE_CONFIG_VERSION: u32 = 1;

/// The service-config record's file name inside the account dir.
pub const SERVICE_CONFIG_FILE: &str = "service_url.json";

/// The env var that overrides everything (Plan 023 precedence: env >
/// config > default). One spelling, owned here.
pub const SERVICE_URL_ENV_VAR: &str = "RIIR_REFINE_KAT_SERVICE_URL";

// issue 088: the host strings moved to the protocol tier's `kat_service_urls`
// (one source shared with the transports) — re-exported so every existing
// path (`crate::kat_service_config::MAINNET_SERVICE_URL`) resolves.
#[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
pub use crate::kat_service_urls::DEVNET_SERVICE_URL;
pub use crate::kat_service_urls::{
    canonical_service_url, LEGACY_SERVICE_URLS, MAINNET_SERVICE_URL,
};

/// The PINNED alias map (Plan 023 g1; testnet retired by riir-dapps Plan
/// 047): exactly two entries, full-word
/// aliases only — no `localhost` alias, no partial-word matching, so a typo
/// can never resolve to a network. `devnet`/`mainnet` are
/// case-insensitive; a custom host is always spelled out as a URL.
pub const PINNED_NETWORKS: &[(&str, &str)] = &[
    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    ("devnet", DEVNET_SERVICE_URL),
    ("mainnet", MAINNET_SERVICE_URL),
];

/// The env half of the resolution — the read lives HERE (ungated) so the
/// ungated `config` command and the gated lease client cannot drift on the
/// var's spelling.
#[must_use]
pub fn service_url_from_env() -> Option<String> {
    std::env::var(SERVICE_URL_ENV_VAR)
        .ok()
        .filter(|u| !u.is_empty())
}

/// Resolve a pinned alias (case-insensitive, full word) to its URL.
#[must_use]
pub fn resolve_alias(alias: &str) -> Option<&'static str> {
    PINNED_NETWORKS
        .iter()
        .find(|(a, _)| a.eq_ignore_ascii_case(alias.trim()))
        .map(|(_, u)| *u)
}

/// The pinned alias a URL IS (if any) — trailing-slash and case tolerant.
/// `None` for a custom/loopback host, which is what the CLI's loud
/// not-a-pinned-host warning keys on.
#[must_use]
pub fn pinned_alias_for(url: &str) -> Option<&'static str> {
    // a legacy name (riir-dapps Plan 047) IS its canonical network
    let canonical = canonical_service_url(url);
    let normalized = canonical.trim().trim_end_matches('/');
    PINNED_NETWORKS
        .iter()
        .find(|(_, u)| normalized.eq_ignore_ascii_case(u))
        .map(|(a, _)| *a)
}

/// The host of an absolute URL (no new dep — RFC 3986-shaped, deliberately
/// stricter than a full parser): scheme, authority up to the first
/// `/`/`?`/`#`, userinfo stripped, IPv6 brackets stripped, port stripped.
/// Pub so the net-pin tier parses hosts through the SAME parser — two
/// host parsers in one crate is drift waiting to happen.
#[must_use]
pub fn host_of(absolute: &str) -> Option<&str> {
    let rest = absolute.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    if let Some(stripped) = host_port.strip_prefix('[') {
        let inner = stripped.split_once(']')?.0;
        return Some(inner);
    }
    let host = host_port.split_once(':').map_or(host_port, |(h, _)| h);
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// Loopback = where plain `http://` is allowed (TLS is meaningless there —
/// the `wrangler dev` case, kept possible explicitly).
#[must_use]
pub fn is_loopback_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}

/// Whether an ABSOLUTE url's host is loopback — the URL-shaped half of
/// [`is_loopback_host`]. The host parse is this module's, so a caller
/// asking "is this a local `wrangler dev` stand-in?" never re-derives one
/// (riir-dapps Issue 094 D4: the refill posture gate is the first caller).
#[must_use]
pub fn is_loopback_url(url: &str) -> bool {
    host_of(url.trim()).is_some_and(is_loopback_host)
}

/// Validate a `config set --url` value (Plan 023 g1): a pinned alias, or an
/// ABSOLUTE url — `https://` required unless the host is loopback. Returns
/// the normalized stored form (trimmed, no trailing slash).
///
/// # Errors
///
/// A human-readable rejection for every non-URL, wrong-scheme, or
/// plain-http-on-a-real-host input — the message names the rule so the CLI
/// can print it verbatim.
pub fn validate_service_url(raw: &str) -> Result<String, String> {
    let input = raw.trim();
    if input.is_empty() {
        return Err(
            "the service url is empty — want an alias (devnet | mainnet) \
             or an absolute url"
                .to_string(),
        );
    }
    if let Some(pinned) = resolve_alias(input) {
        return Ok(pinned.to_string());
    }
    let lower = input.to_ascii_lowercase();
    let scheme_is_https = if lower.starts_with("https://") {
        true
    } else if lower.starts_with("http://") {
        false
    } else {
        return Err(format!(
            "{input:?} is not an alias (devnet | mainnet) and carries no \
             https:// — absolute urls must be https:// unless the host is loopback \
             (localhost / 127.0.0.1 / [::1])"
        ));
    };
    if input.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!(
            "{input:?} contains whitespace or control characters"
        ));
    }
    let host = host_of(input).ok_or_else(|| format!("{input:?} has no host after the scheme"))?;
    if !scheme_is_https && !is_loopback_host(host) {
        return Err(format!(
            "http:// is allowed only for loopback hosts (localhost / 127.0.0.1 / [::1]) — \
             {host} is not loopback; use https://"
        ));
    }
    Ok(input.trim_end_matches('/').to_string())
}

/// Where the active service URL came from (`config get`'s diagnosis line —
/// Plan 023 g3: "I set it but commands still hit the old host" is answered
/// by naming the source, not guessing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceUrlSource {
    /// `RIIR_REFINE_KAT_SERVICE_URL` — wins over everything while set.
    Env,
    /// The persisted `service_url.json` (`cargo refine config set`).
    Config,
    /// [`crate::kat_lease_client::DEFAULT_SERVICE_URL`] — never moves.
    Default,
}

impl ServiceUrlSource {
    /// One short word for the `config get` source line.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Config => "config",
            Self::Default => "default",
        }
    }
}

/// The pure resolution order (env > config > default), split from the I/O
/// so the order is testable without touching the process environment
/// (tests share it — the consent-module rule). `default_url` is supplied
/// by the caller so this ungated module never names the gated lease
/// client: the gated side passes
/// [`crate::kat_lease_client::DEFAULT_SERVICE_URL`], the ungated CLI side
/// passes [`MAINNET_SERVICE_URL`], and a pin test on the gated side
/// asserts the two are the same string.
#[must_use]
pub fn resolved_from(
    env_url: Option<&str>,
    config_url: Option<&str>,
    default_url: &str,
) -> (String, ServiceUrlSource) {
    // Plan 047: a legacy API name (stored before the host move, or set in
    // an old shell profile) resolves to its canonical host — same worker,
    // same ledger — so an upgraded install moves without user action.
    if let Some(u) = env_url.filter(|u| !u.is_empty()) {
        return (canonical_service_url(u), ServiceUrlSource::Env);
    }
    if let Some(u) = config_url {
        return (canonical_service_url(u), ServiceUrlSource::Config);
    }
    (default_url.to_string(), ServiceUrlSource::Default)
}

/// The account dir this record lives in — the ONE resolution in
/// [`crate::kat_mining_consent::default_dir`] (which `kat_account` also
/// delegates to), so all three surfaces cannot drift.
#[must_use]
pub fn account_dir() -> PathBuf {
    crate::kat_mining_consent::default_dir()
}

/// The config record's full path inside `dir`.
#[must_use]
pub fn service_config_path(dir: &Path) -> PathBuf {
    dir.join(SERVICE_CONFIG_FILE)
}

/// The on-disk record. Deliberately tiny and forward-readable.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ServiceUrlDoc {
    /// [`SERVICE_CONFIG_VERSION`] at write time.
    version: u32,
    /// The normalized service URL (the [`validate_service_url`] output —
    /// a stored value that no longer validates reads as "not set").
    url: String,
    /// Unix epoch seconds (UTC) at write time; `0` when unreadable.
    recorded_unix_secs: u64,
}

/// Read the configured URL, plus one honest warning line when a record
/// EXISTS and could not be honoured (missing/unreadable/malformed/
/// future-versioned/no-longer-invalid all read as "not set" — the warning
/// is what keeps that from being silent).
///
/// `None` warning means either "no record" or a clean read.
#[must_use]
pub fn read_with_warning(dir: &Path) -> (Option<String>, Option<String>) {
    let path = service_config_path(dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (None, None),
        Err(e) => {
            return (
                None,
                Some(format!(
                    "service url config: {} is unreadable ({e}) — treating as not set; \
                     `cargo refine config set` rewrites it",
                    path.display()
                )),
            );
        }
    };
    let doc = match serde_json::from_str::<ServiceUrlDoc>(&text) {
        Ok(d) => d,
        Err(e) => {
            return (
                None,
                Some(format!(
                    "service url config: {} is malformed ({e}) — treating as not set; \
                     `cargo refine config set` rewrites it",
                    path.display()
                )),
            );
        }
    };
    if doc.version != SERVICE_CONFIG_VERSION {
        return (
            None,
            Some(format!(
                "service url config: {} is version {} (this build understands \
                 {SERVICE_CONFIG_VERSION}) — treating as not set; re-run \
                 `cargo refine config set`",
                path.display(),
                doc.version
            )),
        );
    }
    match validate_service_url(&doc.url) {
        Ok(valid) => (Some(valid), None),
        Err(e) => (
            None,
            Some(format!(
                "service url config: {} holds an invalid url ({e}) — treating as not \
                 set; re-run `cargo refine config set`",
                path.display()
            )),
        ),
    }
}

/// Read the configured URL, discarding the diagnostic (the read side of
/// [`read_with_warning`]).
#[must_use]
pub fn read(dir: &Path) -> Option<String> {
    read_with_warning(dir).0
}

/// Persist the service URL (already validated — the CLI validates first so
/// the rejection is loud BEFORE any write). The record is rewritten
/// atomically-enough for a one-file config: create/truncate write, then
/// `0600` on unix (the file names the ledger this machine's money commands
/// hit; a permissive mode would let a sibling user repoint them).
///
/// # Errors
///
/// A string when the account dir cannot be created or the file cannot be
/// written (read-only `$HOME`, full disk). The CLI treats that as fatal —
/// a config set we failed to persist is not a set.
pub fn store(dir: &Path, url: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("could not create the account dir {}: {e}", dir.display()))?;
    let doc = ServiceUrlDoc {
        version: SERVICE_CONFIG_VERSION,
        url: url.to_string(),
        recorded_unix_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    };
    let json = serde_json::to_string_pretty(&doc)
        .map_err(|e| format!("could not serialize the service-url record: {e}"))?;
    let path = service_config_path(dir);
    std::fs::write(&path, format!("{json}\n"))
        .map_err(|e| format!("could not write {}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut perms = std::fs::metadata(&path)
            .map_err(|e| format!("could not stat {}: {e}", path.display()))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("could not chmod 0600 {}: {e}", path.display()))?;
    }
    Ok(path)
}

/// The active service URL + where it came from — the resolver every
/// explicit money command goes through. `default_url` is the baked default
/// (the gated caller passes [`crate::kat_lease_client::DEFAULT_SERVICE_URL`];
/// the ungated CLI passes [`MAINNET_SERVICE_URL`] — the pin test on the
/// gated side keeps the two the same string). A corrupt/invalid CONFIG
/// record prints its warning (never silently falls back to the default;
/// the fallback-to-default of a record we MISREAD is the one direction this
/// surface must not be quiet about).
///
/// ⛔ **The stored config row no longer steers the dial** (issue 126
/// follow-up 3, owner law 2026-10-14: "all should target mainnet without
/// any flag, flag is for devnet"): a stale `config set --url devnet` row
/// silently re-pointed EVERY bare command at devnet — the owner ran bare
/// `cargo refine account` and read the DEVNET ledger believing it was the
/// position. The dialing precedence is [`dial_service_url`] — env >
/// mainnet — and the config row survives only as `config get` diagnosis.
#[must_use]
pub fn resolved_service_url(default_url: &str) -> (String, ServiceUrlSource) {
    let dir = account_dir();
    let (_config, warning) = read_with_warning(&dir);
    if let Some(w) = warning {
        eprintln!("config: {w}");
    }
    dial_service_url(service_url_from_env().as_deref(), default_url)
}

/// The DIALING precedence (owner law 2026-10-14): env > default. The
/// stored config row is deliberately ABSENT from this chain —
/// [`resolved_from`] keeps the three-source shape for `config get`'s
/// diagnosis, and this thin seam is what every dial goes through, so the
/// law has one home and a test.
#[must_use]
pub fn dial_service_url(env_url: Option<&str>, default_url: &str) -> (String, ServiceUrlSource) {
    resolved_from(env_url, None, default_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch dir that never touches the real account dir.
    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "riir_heal_service_config_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    /// The pinned map is EXACTLY the two networks — a third entry (or a
    /// `localhost` alias) must fail here, loudly (Plan 023 g1; testnet
    /// retired by riir-dapps Plan 047).
    #[test]
    fn the_alias_map_is_exactly_two_networks() {
        // mainnet-only pin: the devnet row compiles out — the map is ONE
        // network, and the devnet alias must not resolve.
        if cfg!(all(feature = "net-mainnet", not(feature = "net-devnet"))) {
            assert_eq!(PINNED_NETWORKS.len(), 1);
            assert_eq!(resolve_alias("devnet"), None);
        } else {
            assert_eq!(PINNED_NETWORKS.len(), 2);
            assert_eq!(resolve_alias("devnet"), Some("https://devnet.gist.rs"));
        }
        assert_eq!(resolve_alias("mainnet"), Some("https://mainnet.riir.ai"));
        assert_eq!(
            resolve_alias("testnet"),
            None,
            "the testnet deployment is retired"
        );
        // The g1 pin: no localhost alias — loopback is raw-URL-only.
        assert_eq!(resolve_alias("localhost"), None);
        assert_eq!(resolve_alias("lo"), None);
        assert_eq!(resolve_alias("dev"), None);
        assert_eq!(resolve_alias("dev network"), None);
        // Full words, case-insensitive; partial words never match.
        #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
        {
            assert_eq!(resolve_alias("DEVNET"), Some(DEVNET_SERVICE_URL));
        }
        assert_eq!(resolve_alias(" Mainnet "), Some(MAINNET_SERVICE_URL));
    }

    /// The const-never-moves premise's UNGATED half: the mirror const is
    /// the apex. The gated twin of this pin
    /// (`MAINNET_SERVICE_URL == DEFAULT_SERVICE_URL`) lives in
    /// `kat_lease_client`'s tests — this module cannot name that path in a
    /// default build, by construction. Together they keep the QR goldens
    /// from ever tripping at the domain swap (Plan 023's core claim).
    #[test]
    fn mainnet_const_is_the_ungated_default_mirror() {
        assert_eq!(MAINNET_SERVICE_URL, "https://mainnet.riir.ai");
        // Plan 059 C1: the gen-2 apex and the gen-1 API host are the mainnet alias
        assert_eq!(pinned_alias_for("https://riir.ai"), Some("mainnet"));
        assert_eq!(pinned_alias_for("https://mainnet.gist.rs"), Some("mainnet"));
        assert_eq!(pinned_alias_for(MAINNET_SERVICE_URL), Some("mainnet"));
        // Plan 047: the legacy names ARE their networks, and a stored or
        // env-set legacy URL resolves to the canonical host
        assert_eq!(pinned_alias_for("https://ai.gist.rs"), Some("mainnet"));
        #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
        {
            assert_eq!(
                pinned_alias_for("https://devnet.ai.gist.rs/"),
                Some("devnet")
            );
        }
        assert_eq!(pinned_alias_for("https://testnet.ai.gist.rs"), None);
        #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
        {
            let (u, src) =
                resolved_from(None, Some("https://devnet.ai.gist.rs"), MAINNET_SERVICE_URL);
            assert_eq!(
                (u.as_str(), src),
                (DEVNET_SERVICE_URL, ServiceUrlSource::Config)
            );
        }
        let (u, src) = resolved_from(Some("https://ai.gist.rs/"), None, MAINNET_SERVICE_URL);
        assert_eq!(
            (u.as_str(), src),
            (MAINNET_SERVICE_URL, ServiceUrlSource::Env)
        );
    }

    #[test]
    fn pinned_alias_for_round_trips_and_distinguishes_custom() {
        for (alias, url) in PINNED_NETWORKS {
            assert_eq!(pinned_alias_for(url), Some(*alias));
            assert_eq!(pinned_alias_for(&format!("{url}/")), Some(*alias));
            assert_eq!(resolve_alias(alias), Some(*url));
        }
        assert_eq!(pinned_alias_for("http://localhost:8787"), None);
        assert_eq!(pinned_alias_for("https://kat.example.dev"), None);
    }

    #[test]
    fn validation_accepts_https_and_aliases_and_rejects_the_rest() {
        // Aliases pass through to the pinned URL.
        #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
        {
            assert_eq!(validate_service_url("devnet").unwrap(), DEVNET_SERVICE_URL);
        }
        // https anywhere.
        assert_eq!(
            validate_service_url("https://ai.gist.rs").unwrap(),
            "https://ai.gist.rs"
        );
        assert_eq!(
            validate_service_url("  https://ai.gist.rs/  ").unwrap(),
            "https://ai.gist.rs"
        );
        // http ONLY on loopback (the wrangler dev case).
        assert_eq!(
            validate_service_url("http://localhost:8787").unwrap(),
            "http://localhost:8787"
        );
        assert_eq!(
            validate_service_url("http://127.0.0.1").unwrap(),
            "http://127.0.0.1"
        );
        assert_eq!(
            validate_service_url("http://[::1]:8787").unwrap(),
            "http://[::1]:8787"
        );
        // Rejections: plain http off loopback; no scheme; wrong scheme;
        // empty; bare scheme; whitespace.
        for (bad, needle) in [
            ("http://ai.gist.rs", "https://"),
            ("http://kat.example.dev", "loopback"),
            ("ai.gist.rs", "https://"),
            ("devnet.ai.gist.rs", "alias"),
            ("ftp://ai.gist.rs", "https://"),
            ("", "empty"),
            ("   ", "empty"),
            ("https://", "no host"),
            ("https://hos t", "whitespace"),
        ] {
            let err = validate_service_url(bad).unwrap_err();
            assert!(
                err.contains(needle),
                "validate({bad:?}) = {err:?}, expected to mention {needle:?}"
            );
        }
    }

    #[test]
    fn host_of_survives_userinfo_ports_and_ipv6() {
        assert_eq!(
            host_of("https://user:pw@kat.example.dev/x"),
            Some("kat.example.dev")
        );
        assert_eq!(host_of("http://localhost:8787"), Some("localhost"));
        assert_eq!(host_of("https://[::1]:8787/p?k=v#f"), Some("::1"));
        assert_eq!(host_of("https://AI.gist.rs"), Some("AI.gist.rs"));
        assert_eq!(host_of("https://"), None);
    }

    #[test]
    fn loopback_detection_is_exact_and_case_insensitive() {
        for yes in ["localhost", "LOCALHOST", "127.0.0.1", "::1"] {
            assert!(is_loopback_host(yes), "{yes} must be loopback");
        }
        for no in [
            "localhost.example.dev",
            "127.0.0.2",
            "::2",
            "ai.gist.rs",
            "",
        ] {
            assert!(!is_loopback_host(no), "{no} must NOT be loopback");
        }
    }

    #[test]
    fn the_url_shaped_loopback_half_agrees_with_the_host_one() {
        for yes in [
            "http://localhost:8787",
            "http://127.0.0.1:8787/burn",
            "https://[::1]/pay",
            "  http://LOCALHOST  ",
        ] {
            assert!(is_loopback_url(yes), "{yes} must be loopback");
        }
        for no in [
            MAINNET_SERVICE_URL,
            #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
            DEVNET_SERVICE_URL,
            "https://localhost.example.dev",
            "not-a-url",
            "",
        ] {
            assert!(!is_loopback_url(no), "{no} must NOT be loopback");
        }
    }

    #[test]
    fn resolution_order_is_env_then_config_then_default() {
        const DEFAULT: &str = "https://default.example";
        // Pure — no process env is touched (tests share it).
        let (url, src) = resolved_from(
            Some("https://env.example"),
            Some("https://cfg.example"),
            DEFAULT,
        );
        assert_eq!(url, "https://env.example");
        assert_eq!(src, ServiceUrlSource::Env);
        let (url, src) = resolved_from(None, Some("https://cfg.example"), DEFAULT);
        assert_eq!(url, "https://cfg.example");
        assert_eq!(src, ServiceUrlSource::Config);
        let (url, src) = resolved_from(None, None, DEFAULT);
        assert_eq!(url, DEFAULT);
        assert_eq!(src, ServiceUrlSource::Default);
        // An EMPTY env value is not a configuration (the shipped filter).
        let (url, src) = resolved_from(Some(""), Some("https://cfg.example"), DEFAULT);
        assert_eq!(src, ServiceUrlSource::Config);
        assert_eq!(url, "https://cfg.example");
    }

    /// The DIALING law (issue 126 follow-up 3, owner 2026-10-14: "all
    /// should target mainnet without any flag, flag is for devnet"): the
    /// dialing seam takes NO config row — a stale `config set --url
    /// devnet` can never re-point a bare command again. env wins, the
    /// baked default (mainnet) is the floor.
    #[test]
    fn dial_service_url_never_sees_the_config_row() {
        const DEFAULT: &str = "https://mainnet.gist.rs";
        // No env → MAINNET, whatever a config file may hold (the seam has
        // no config parameter — that IS the law; the compile-time shape is
        // the pin).
        let (url, src) = dial_service_url(None, DEFAULT);
        assert_eq!(url, DEFAULT);
        assert_eq!(src, ServiceUrlSource::Default);
        // An explicit env still steers (operators + the hermetic rigs).
        let (url, src) = dial_service_url(Some("https://devnet.gist.rs"), DEFAULT);
        assert_eq!(url, "https://devnet.gist.rs");
        assert_eq!(src, ServiceUrlSource::Env);
        // The I/O entry composes the seam: resolved_service_url passes the
        // env through and DROPS the config row (pinned by construction —
        // the fn body has no read of the config value for the dial).
    }

    #[test]
    fn source_labels_are_distinct() {
        let labels = [
            ServiceUrlSource::Env.label(),
            ServiceUrlSource::Config.label(),
            ServiceUrlSource::Default.label(),
        ];
        assert!(labels[0] != labels[1] && labels[1] != labels[2] && labels[0] != labels[2]);
    }

    #[test]
    #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
    fn store_read_round_trip_and_permissions() {
        let dir = scratch("roundtrip");
        let url = validate_service_url("devnet").unwrap();
        let path = store(&dir, &url).expect("store");
        assert_eq!(path, service_config_path(&dir));
        assert_eq!(read(&dir).as_deref(), Some(DEVNET_SERVICE_URL));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(&path).expect("stat").permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "the record must be 0600");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_and_future_records_are_not_set_with_a_warning() {
        let dir = scratch("malformed");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = service_config_path(&dir);
        std::fs::write(&path, "{ not json").expect("plant");
        let (url, warning) = read_with_warning(&dir);
        assert_eq!(url, None);
        let w = warning.expect("malformed MUST warn");
        assert!(w.contains("malformed"), "warning was: {w:?}");
        std::fs::write(
            &path,
            r#"{"version":99,"url":"https://ai.gist.rs","recorded_unix_secs":0}"#,
        )
        .expect("plant");
        let (url, warning) = read_with_warning(&dir);
        assert_eq!(url, None);
        let w = warning.expect("future version MUST warn");
        assert!(w.contains("version 99"), "warning was: {w:?}");
        // A record whose URL no longer validates is not a config either.
        std::fs::write(
            &path,
            r#"{"version":1,"url":"http://ai.gist.rs","recorded_unix_secs":0}"#,
        )
        .expect("plant");
        let (url, warning) = read_with_warning(&dir);
        assert_eq!(url, None);
        let w = warning.expect("invalid stored url MUST warn");
        assert!(w.contains("http://"), "warning was: {w:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn absent_record_is_none_without_a_warning() {
        let dir = scratch("absent");
        let (url, warning) = read_with_warning(&dir);
        assert_eq!(url, None);
        assert!(warning.is_none(), "an absent record is ordinary");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

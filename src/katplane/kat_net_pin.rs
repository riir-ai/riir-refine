//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_net_pin.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

//! Net-pinned builds (riir-kat Issue 003 T9) — the CLIENT-side compile-time
//! restriction of which KAT service hosts a binary may talk to. Feature
//! flags are RESTRICTIONS with OVERLAP semantics, never `compile_error!`
//! (the `--all-features` lanes this workspace runs routinely must compile):
//!
//! | build | allowed host set |
//! |---|---|
//! | no pin features (the shipped bin) | unrestricted — multi-net, default `mainnet` |
//! | `net-mainnet` | the mainnet canonical host + its legacy aliases (+ loopback with `allow-local`) |
//! | `net-devnet` | the devnet canonical host + its legacy aliases (+ loopback with `allow-local`) |
//! | BOTH pins | the overlap = the EMPTY set — every HOSTED host refuses at runtime, loud; loopback still follows `allow-local` |
//!
//! `allow-local` only WIDENS (adds loopback hosts — the `wrangler dev`
//! lane); it never names a network. A pin also refuses CUSTOM hosts: today
//! `validate_service_url` accepts any `https://` URL, so a custom proxy URL
//! would route a "devnet" build at mainnet — under a pin, everything
//! outside the pinned host + its legacy aliases refuses naming the rebuild.
//!
//! The stamp law (the issue's round-3 verdict): [`NET_PIN`] is exported by
//! THE CRATE THAT ENFORCES THE PIN — a consumer's `--version` cannot see
//! another crate's feature list, so the label must come from here. The
//! shipped (unpinned) bin prints `multi (default mainnet)` — NEVER
//! `net-mainnet`: an unpinned build that labeled itself pinned would
//! misreport its own posture.
//!
//! Foreign hosts compile to NOTHING in a pinned build (the
//! compile-to-nothing discipline): the allowed set is a compile-time
//! constant — there is no runtime branch that could fall back onto another
//! network, only this refusal.

use crate::kat_service_config::{host_of, is_loopback_host, validate_service_url};
#[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
use crate::kat_service_urls::DEVNET_SERVICE_URL;
use crate::kat_service_urls::{canonical_service_url, LEGACY_SERVICE_URLS, MAINNET_SERVICE_URL};

/// The devnet host URL, posture-adaptive: `None` at a mainnet-only
/// compilation (the const is cfg'd out there — compile-to-nothing), the
/// URL at every other posture. The net-pin code that must typecheck at ALL
/// postures goes through this, never the const directly.
#[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
#[must_use]
// The Option is the posture contract, not a wrap: the cfg twin arm below
// returns None at the mainnet-only compile (issue 221 vendored allow).
#[allow(clippy::unnecessary_wraps)]
fn devnet_service_url() -> Option<String> {
    Some(DEVNET_SERVICE_URL.to_string())
}

#[cfg(all(feature = "net-mainnet", not(feature = "net-devnet")))]
#[must_use]
fn devnet_service_url() -> Option<String> {
    None
}

/// The compile-time net posture, printed by `--version` (via the consumer's
/// build stamp). Derived from the pin features — never hand-set.
pub const NET_PIN: &str = if cfg!(all(feature = "net-mainnet", feature = "net-devnet")) {
    "conflict: net-mainnet+net-devnet (every hosted host refuses; loopback follows allow-local)"
} else if cfg!(feature = "net-mainnet") {
    "mainnet"
} else if cfg!(all(feature = "net-devnet", not(feature = "net-mainnet"))) {
    "devnet"
} else {
    "multi (default mainnet)"
};

/// The hosts a pinned build may talk to, or `None` when the build carries
/// NO pin (the shipped multi-net bin — unrestricted by this module).
///
/// The set is the OVERLAP of the enabled pins (a pin restricts; two pins
/// intersect), each widened by `allow-local`'s loopback entries. Legacy
/// aliases are listed through their canonical form — callers canonicalize
/// first ([`ensure_net_allowed`]) so `https://ai.gist.rs` and
/// `https://mainnet.gist.rs` are one host.
#[must_use]
pub fn pinned_allowed_hosts() -> Option<Vec<String>> {
    let mainnet = cfg!(feature = "net-mainnet");
    let devnet = cfg!(feature = "net-devnet");
    if !mainnet && !devnet {
        return None; // the shipped multi-net bin
    }
    let mut set = Vec::new();
    if mainnet && devnet {
        // the overlap of two disjoint host sets — empty by construction
    } else if mainnet {
        set.push(MAINNET_SERVICE_URL.to_string());
        // The points plane's own host (the gen-2 worker's custom domain;
        // riir-refine `points_client::DEFAULT_SERVICE_URL`). Same worker +
        // ledger as MAINNET_SERVICE_URL, so the mainnet pin admits it —
        // the owner's mainnet-only directive (2026-11-04, the shipped
        // release set pins net-mainnet) must not break join/rank/info.
        set.push("https://tuna.riir.ai".to_string());
    } else {
        // The devnet-only pin: the devnet host is the allowed set. The
        // accessor is posture-adaptive — at a mainnet-only compilation the
        // const does not exist (compile-to-nothing) and this branch is
        // logically unreachable anyway.
        if let Some(u) = devnet_service_url() {
            set.push(u);
        }
    }
    Some(set)
}

/// The legacy alias URLs of the pinned network(s) — kept beside the allowed
/// set so a refusal message can name every spelling that WOULD have been
/// accepted ([`ensure_net_allowed`] canonicalizes, so aliases are accepted
/// through the canonical form; this is diagnostics, not a second gate).
#[must_use]
pub fn pinned_legacy_aliases() -> Vec<&'static str> {
    let mut out = Vec::new();
    for (legacy, canonical) in LEGACY_SERVICE_URLS {
        let pinned = match pinned_allowed_hosts() {
            None => true, // unrestricted: name every alias for diagnosis
            Some(set) => set.iter().any(|h| h == canonical),
        };
        if pinned {
            out.push(*legacy);
        }
    }
    out
}

/// Enforce the compiled pin against a service URL. `Ok` carries the
/// CANONICAL form (aliases and legacy spellings resolve to the pinned
/// host); `Err` is a human-readable refusal naming the rebuild — the CLI
/// prints it verbatim and exits nonzero.
///
/// Unpinned builds never refuse (this function is their no-op guard).
///
/// Order law (riir-refine Issue 163): the loopback/`allow-local` branch is
/// checked BEFORE the empty-set refusal, so the both-pins build widened by
/// `allow-local` — exactly what `--all-features` compiles — keeps its
/// wrangler-dev/hermetic-test lane. Hosted refusals are unchanged: the
/// conflict posture still refuses every hosted net (the anti-spoof law),
/// and the EMPTY check stays ahead of the allowed-set match so a hosted
/// refusal names the conflict, not a misleading "outside your pin".
///
/// # Errors
///
/// The pinned-host verdict as a string when the URL's host sits outside
/// the build's allowed set (a hosted net at a single-pin build, or the
/// both-pins conflict posture); `Ok` carries the canonicalized URL.
pub fn ensure_net_allowed(raw: &str) -> Result<String, String> {
    let Some(allowed) = pinned_allowed_hosts() else {
        return Ok(canonical_service_url(raw.trim()));
    };
    let input = raw.trim();
    // aliases resolve to a pinned host (or the other net's — then refused);
    // legacy URLs canonicalize; loopback stays loopback.
    let resolved = validate_service_url(input);
    let conflict_hint = if allowed.is_empty() {
        " — this build enables BOTH net-mainnet and net-devnet; rebuild with \
         at most one pin"
    } else {
        ""
    };
    let candidate = match resolved {
        Ok(normalized) => canonical_service_url(&normalized),
        Err(rule) => return Err(format!("{rule}{conflict_hint} (net: {NET_PIN})")),
    };
    let Some(host) = host_of(&candidate) else {
        return Err(format!(
            "could not parse a host from {input:?}{conflict_hint} (net: {NET_PIN})"
        ));
    };
    // loopback follows allow-local, never the pin — ahead of the empty-set
    // refusal so both pins + allow-local keeps the local lane (Issue 163).
    if is_loopback_host(host) {
        if cfg!(feature = "allow-local") {
            return Ok(candidate);
        }
        return Err(format!(
            "loopback {host} is outside this build's pin — rebuild with the \
             allow-local feature for the wrangler-dev lane (net: {NET_PIN})"
        ));
    }
    if allowed.is_empty() {
        return Err(format!(
            "this build enables BOTH net-mainnet and net-devnet — the allowed \
             host set is EMPTY by overlap semantics; rebuild with at most one \
             pin (net: {NET_PIN})"
        ));
    }
    if allowed.iter().any(|a| a == &candidate) {
        return Ok(candidate);
    }
    let aliases = pinned_legacy_aliases().join(", ");
    Err(format!(
        "service host {candidate} is outside this build's net pin — allowed: \
         {}{}; rebuild without the pin for a multi-net bin (net: {NET_PIN})",
        allowed.join(", "),
        if aliases.is_empty() {
            String::new()
        } else {
            format!(" (legacy aliases: {aliases})")
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The label the CURRENT compilation must carry — derived from the same
    /// cfg!() the const uses, so the test is correct at EVERY feature
    /// posture (default, each pin, --all-features conflict).
    fn expected_pin() -> &'static str {
        match (cfg!(feature = "net-mainnet"), cfg!(feature = "net-devnet")) {
            (true, true) => {
                "conflict: net-mainnet+net-devnet (every hosted host refuses; loopback follows allow-local)"
            }
            (true, false) => "mainnet",
            // posture-adaptive STRING, never a cfg'd-out ARM (a gated arm
            // makes the match non-exhaustive at the other test postures):
            // at mainnet-only this arm is unreachable (net-devnet is off),
            // so its label only ever matters at the devnet-only posture.
            (false, true) => {
                if cfg!(all(feature = "net-devnet", not(feature = "net-mainnet"))) {
                    "devnet"
                } else {
                    "mainnet-only (the devnet label cannot occur)"
                }
            }
            (false, false) => "multi (default mainnet)",
        }
    }

    #[test]
    fn net_pin_label_matches_the_compiled_features() {
        assert_eq!(NET_PIN, expected_pin());
    }

    #[test]
    fn unpinned_builds_are_unrestricted() {
        if pinned_allowed_hosts().is_some() {
            return; // posture guard: this arm is about the shipped bin
        }
        for url in [
            crate::kat_service_urls::MAINNET_SERVICE_URL,
            #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
            crate::kat_service_urls::DEVNET_SERVICE_URL,
            "https://testnet.ai.gist.rs",
            "http://localhost:8787",
        ] {
            assert!(ensure_net_allowed(url).is_ok(), "{url}");
        }
    }

    #[test]
    fn pinned_builds_accept_only_the_pinned_net_and_its_aliases() {
        let Some(allowed) = pinned_allowed_hosts() else {
            return; // posture guard: runs only under a single pin
        };
        if allowed.is_empty() {
            return; // the conflict posture has its own arm below
        }
        let pinned = &allowed[0];
        // the pinned host, its short alias, and its legacy spelling all pass
        let alias = if pinned.contains("devnet") {
            "devnet"
        } else {
            "mainnet"
        };
        for url in [pinned.as_str(), alias] {
            let ok = ensure_net_allowed(url);
            assert_eq!(ok.as_deref(), Ok(pinned.as_str()), "{url}: {ok:?}");
        }
        for legacy in pinned_legacy_aliases() {
            assert_eq!(
                ensure_net_allowed(legacy).as_deref(),
                Ok(pinned.as_str()),
                "{legacy}"
            );
        }
        // the OTHER net, a custom https host, and a retired host all refuse
        let other = if pinned.contains("devnet") {
            crate::kat_service_urls::MAINNET_SERVICE_URL
        } else {
            // posture-adaptive: at a mainnet-only compile the devnet const
            // does not exist; the test asserts against a custom host then
            // (the custom-host refusal arm below still fires).
            &match devnet_service_url() {
                Some(u) => u,
                None => "https://proxy.example.com".to_string(),
            }
        };
        for url in [
            other,
            "https://testnet.ai.gist.rs",
            "https://proxy.example.com",
        ] {
            let err = ensure_net_allowed(url).unwrap_err();
            assert!(err.contains("net pin"), "{url}: {err}");
            assert!(err.contains("rebuild"), "{url}: {err}");
        }
        // loopback follows allow-local, never the pin
        let loopback = ensure_net_allowed("http://localhost:8787");
        if cfg!(feature = "allow-local") {
            assert!(loopback.is_ok());
        } else {
            assert!(loopback.unwrap_err().contains("allow-local"));
        }
    }

    /// Issue 163 (split from the old `both_pins_refuse_every_host_loudly`):
    /// at the conflict posture every HOSTED host still refuses EMPTY — the
    /// anti-spoof law is untouched by the loopback reorder.
    #[test]
    fn both_pins_refuse_every_hosted_host_loudly() {
        if !matches!(pinned_allowed_hosts(), Some(set) if set.is_empty()) {
            return; // posture guard: the --all-features conflict cell only
        }
        assert_eq!(NET_PIN, expected_pin());
        for url in [
            crate::kat_service_urls::MAINNET_SERVICE_URL,
            #[cfg(not(all(feature = "net-mainnet", not(feature = "net-devnet"))))]
            crate::kat_service_urls::DEVNET_SERVICE_URL,
            "https://testnet.ai.gist.rs",
            "https://proxy.example.com",
        ] {
            let err = ensure_net_allowed(url).unwrap_err();
            assert!(err.contains("EMPTY"), "{url}: {err}");
            assert!(err.contains("at most one pin"), "{url}: {err}");
        }
    }

    /// Issue 163: at the conflict posture loopback follows `allow-local`,
    /// never the pin. ONE test with both arms so every conflict posture
    /// (with and without `allow-local`) executes a real assertion — two
    /// separately-gated tests would report a green zero at either posture.
    #[test]
    fn both_pins_loopback_follows_allow_local() {
        if !matches!(pinned_allowed_hosts(), Some(set) if set.is_empty()) {
            return; // posture guard: the conflict cell only
        }
        let loopback = ensure_net_allowed("http://localhost:8787");
        if cfg!(feature = "allow-local") {
            assert!(
                loopback.is_ok(),
                "both pins + allow-local must keep the loopback lane: {loopback:?}"
            );
        } else {
            let err = loopback.unwrap_err();
            assert!(err.contains("allow-local"), "{err}");
        }
    }

    /// Issue 163: the conflict hint survives a parse failure — an invalid
    /// URL at the both-pins posture must still name the misconfiguration,
    /// not just the syntax rule.
    #[test]
    fn conflict_posture_parse_failures_still_name_the_conflict() {
        if !matches!(pinned_allowed_hosts(), Some(set) if set.is_empty()) {
            return; // posture guard: the conflict cell only
        }
        // "" and a non-alias scheme both die in the parse-rules branch (not
        // the loopback branch), so each refusal must still carry the hint.
        for url in ["", "ftp://x.example.com"] {
            let err = ensure_net_allowed(url).unwrap_err();
            assert!(err.contains("at most one pin"), "{url:?}: {err}");
        }
    }

    #[test]
    fn host_of_strips_ports_and_paths() {
        assert_eq!(host_of("https://mainnet.gist.rs"), Some("mainnet.gist.rs"));
        assert_eq!(host_of("http://127.0.0.1:8787"), Some("127.0.0.1"));
        assert_eq!(host_of("http://[::1]:8787"), Some("::1"));
        assert_eq!(host_of("https://x.io/pay?a=b"), Some("x.io"));
        assert_eq!(host_of("devnet"), None);
    }
}

//! Batch 398 sec fixture preflight — the DiscoveryEgress family seed
//! (Plan 398 / Research 432, ddalcu/mlx-serve @ 96311b26 MIT) as ONE
//! standalone file (1 rule, 4 arms).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Zig: `src/host/lan/lan.zig`; the Rust arms below model the
//! law's decision content). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b398_sec_fixtures.rs`
//! at debug AND -O.
//!
//! Arms (the b393 discipline — every classifier deterministic, every
//! assert pair names the honest and the adversary half):
//! - 398a the defect arm: an unauthenticated mDNS advertisement plants
//!   BOTH the inventory and the egress destination; the next request
//!   naming a planted model exfiltrates its body to the attacker's
//!   listener (the self-recognition token stops nothing — it only
//!   lets the host skip its OWN advertisement).
//! - 398b the fixed arm: the pairing vocabulary — a PSK challenge the
//!   peer must ECHO on its discovered surface (unproven peers are
//!   listed display-only), the first-use consent seat for an
//!   allowlisted stranger, the allowlist arm that lists but never
//!   forwards.
//! - 398c the all-safe arms: a display-only inventory (the human dials
//!   by hand — the human is the control) and a discovery plane inside
//!   an authenticated overlay both run without the control.
//! - 398d the tampered arm: a "verify" that accepts any advertiser —
//!   the exfiltration path must stay open under it (the check that
//!   cannot fail is not a control).

use std::collections::HashMap;

// ────────────────────────── the model ──────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct Peer {
    name: String,
    addr: String,
    models: Vec<String>,
    proven: bool,
}

struct Discovery {
    self_token: [u8; 8],
    pairing_psk: [u8; 8],
    allowlist: Vec<String>,
    consented: Vec<String>,
    peers: Vec<Peer>,
}

impl Discovery {
    fn new() -> Self {
        Discovery {
            self_token: [7; 8],
            pairing_psk: [42; 8],
            allowlist: vec!["workstation".to_string()],
            consented: Vec::new(),
            peers: Vec::new(),
        }
    }

    /// the advertisement handler. `txt_token` is the advertiser's TXT
    /// record; `psk_echo` says whether the peer answered the PSK
    /// challenge on its discovered surface.
    fn on_advertisement(
        &mut self,
        name: &str,
        txt_token: &[u8],
        addr: &str,
        models: Vec<String>,
        psk_echo: bool,
    ) {
        if txt_token == self.self_token {
            return; // self_ad: skip our own advertisement
        }
        let proven = psk_echo; // the ONLY ownership proof
        self.peers.push(Peer {
            name: name.to_string(),
            addr: addr.to_string(),
            models,
            proven,
        });
    }

    /// request routing for a model name — the egress decision.
    /// Buggy posture: ANY peer listing the model receives the body.
    fn route_buggy(&self, model: &str) -> Option<String> {
        self.peers
            .iter()
            .find(|p| p.models.iter().any(|m| m == model))
            .map(|p| p.addr.clone())
    }

    /// request routing, the fixed posture: proven peers only; an
    /// unproven ALLOWLISTED peer asks consent ONCE; an unproven
    /// stranger is display-only forever.
    fn route_fixed(&mut self, model: &str, consent: bool) -> Option<String> {
        let idx = self
            .peers
            .iter()
            .position(|p| p.models.iter().any(|m| m == model))?;
        let p = &self.peers[idx];
        if p.proven {
            return Some(p.addr.clone());
        }
        if self.allowlist.iter().any(|a| a == &p.name) {
            if self.consented.iter().any(|c| c == &p.name) {
                return Some(p.addr.clone());
            }
            if consent {
                self.consented.push(p.name.clone());
                return Some(p.addr.clone());
            }
        }
        None // unproven + unconsented: list-only, never a destination
    }
}

fn attacker_advertises(d: &mut Discovery) {
    // the attacker answers the browse with a planted model list; the
    // only check the buggy posture carries is the self-recognition
    // token, which authenticates nothing about the attacker.
    let attacker_token = [9; 8]; // anything but the host's own
    d.on_advertisement(
        "fast-local-model",
        &attacker_token,
        "10.0.0.66:11234",
        vec!["steal-me-70b".to_string()],
        false, // no PSK echo — it is not the owner
    );
}

// ────────────────────────── the arms ──────────────────────────

#[test]
fn a398_the_advertisement_plants_inventory_and_destination() {
    let mut d = Discovery::new();
    attacker_advertises(&mut d);
    // the inventory mirrors the attacker's self-description
    assert!(d
        .peers
        .iter()
        .any(|p| p.models.contains(&"steal-me-70b".to_string())));
    // the egress decision: the planted model routes to the attacker's
    // listener — the body follows the advertisement.
    let dest = d.route_buggy("steal-me-70b");
    assert_eq!(dest.as_deref(), Some("10.0.0.66:11234"));
    // the self-recognition token stops nothing: it only matches the
    // host's OWN token; the attacker's advertisement sails through.
    assert_ne!([9u8; 8], [7u8; 8], "the token check cannot fire on a stranger");
}

#[test]
fn a398_the_pairing_vocabulary_controls_the_destination() {
    let mut d = Discovery::new();
    // the legitimate peer ECHOES the minted PSK on its surface: proven
    d.on_advertisement(
        "workstation",
        &[9; 8],
        "10.0.0.7:11234",
        vec!["legit-70b".to_string()],
        true,
    );
    // the attacker does not: listed, never proven
    attacker_advertises(&mut d);
    // the proven peer serves bodies
    assert_eq!(
        d.route_fixed("legit-70b", false).as_deref(),
        Some("10.0.0.7:11234")
    );
    // the attacker's planted model is display-only — no body ever
    // routes to an unproven stranger, consent or not (not allowlisted).
    assert_eq!(d.route_fixed("steal-me-70b", true), None);
    // an ALLOWLISTED but unproven peer needs first-use consent: refused
    // until the operator answers, then granted once.
    d.on_advertisement(
        "workstation",
        &[9; 8],
        "10.0.0.7:11234",
        vec!["m2".to_string()],
        false,
    );
    assert_eq!(d.route_fixed("m2", false), None, "no consent yet");
    assert_eq!(
        d.route_fixed("m2", true).as_deref(),
        Some("10.0.0.7:11234"),
        "first-use consent granted once"
    );
    assert_eq!(
        d.route_fixed("m2", false).as_deref(),
        Some("10.0.0.7:11234"),
        "the consent persists"
    );
}

#[test]
fn a398_the_all_safe_postures_need_no_extra_control() {
    // DISPLAY-ONLY inventory: the human dials by hand — the human is
    // the control; the router is never consulted.
    let mut d = Discovery::new();
    attacker_advertises(&mut d);
    let display: Vec<&String> = d.peers.iter().map(|p| &p.name).collect();
    assert!(!display.is_empty(), "the inventory may mirror freely");
    // no route call — nothing egresses by advertisement alone.
    // AUTHENTICATED-OVERLAY discovery: the advertisement rides an
    // already-authenticated plane, so `proven` is structural.
    struct OverlayDiscovery {
        peers: HashMap<String, String>, // mTLS-attested name -> addr
    }
    let overlay = OverlayDiscovery {
        peers: HashMap::from([("workstation".to_string(), "10.0.0.7:1".to_string())]),
    };
    // an advertisement for a name the overlay never attested cannot route
    assert_eq!(overlay.peers.get("attacker"), None);
}

#[test]
fn a398_the_tampered_verify_that_cannot_fail_is_no_control() {
    // the TAMPERED arm: a "verify" that accepts any advertiser keeps the
    // exfiltration path open — the fixed router with a lying psk_echo.
    let mut d = Discovery::new();
    // the attacker DOES echo (a tampered check marks it proven)
    d.on_advertisement(
        "fast-local-model",
        &[9; 8],
        "10.0.0.66:11234",
        vec!["steal-me-70b".to_string()],
        true, // the tampered verify's verdict
    );
    let dest = d.route_fixed("steal-me-70b", false);
    assert_eq!(
        dest.as_deref(),
        Some("10.0.0.66:11234"),
        "a check that cannot fail is not a control: the body routes"
    );
    // the honest half of the pair: the SAME advertisement WITHOUT the
    // echo stays display-only — the control is the challenge, not the
    // listing.
    let mut honest = Discovery::new();
    attacker_advertises(&mut honest);
    assert_eq!(honest.route_fixed("steal-me-70b", true), None);
}

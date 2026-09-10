//! The account SERVICE-plane client — Proposal 011 T0.2b (Issue 061, Plan 064).
//!
//! T0.2 landed `cargo heal account` as a purely LOCAL view and left two
//! things explicitly undone, with the reason written into the source
//! ([`crate::kat_account::welcome_line`]): the welcome gate was a local
//! heuristic (`no local burn yet` ⇒ fresh) rather than 011 §8.3's
//! *"all-synced + nodes healthy"* condition, and the history read the
//! per-repo charge ledger rather than the service ledger. This module is
//! that missing half.
//!
//! # The three real reads
//!
//! | read | route | what it answers |
//! |---|---|---|
//! | ledger health | `GET /health` | the ledger plane is up AND has no durable write still pending |
//! | mining health | `GET /mining/health` | the mining plane is up (and how many records await proof) |
//! | the account row | `GET /balance?acct=` | the ledger's authoritative balance + whether the grant landed |
//!
//! Those are the health/sync facts the service plane actually serves today
//! (riir-dapps `cloudflare/kat-service/src/routes.rs`). The per-NODE
//! reachability/height/sync% table is a different instrument on a different
//! host — the `node-sync` probe, Issue 061 **T3.2**, which this deliberately
//! does not pretend to be.
//!
//! One more read is DISPLAY-ONLY: `GET /` service info (dapps Issue 029
//! Rider 1) renders `service: devnet (virtual clock)` from the
//! deployment's self-declared `env` label + `tick_source`. It is never a
//! gate input, and the probe reads it LAST — the fail-fast rule below
//! still bounds an offline run at ONE timeout.
//!
//! # Three states, never two
//!
//! [`WelcomeGate`] separates *"read the network, it is not settled"*
//! ([`WelcomeGate::Withheld`]) from *"could not read the network at all"*
//! ([`WelcomeGate::Unknown`]). Folding `Unknown` into `Open` would print a
//! sync-health claim the CLI cannot see, which is exactly what T0.2 refused
//! to do; folding it into `Withheld` would tell an offline user their
//! account is out of sync, which is a different lie.
//!
//! # Feature shape
//!
//! `kat_service` gets the parses, the gate and the renders (std +
//! `serde_json` only — pure functions, testable with no transport).
//! [`AccountTransport`] and the probe live behind `kat_transport`, the
//! `kat_payreq_client` / `kat_vote_client` convention — one trait per
//! channel, one ureq impl, a scripted mock in tests driving the same trait.
//!
//! The wire types + pure wire decoders live in [`crate::kat_protocol_account`]
//! (the issue-088 protocol tier) and are re-exported below — every existing
//! path unchanged.

#[cfg(feature = "kat_transport")]
use crate::kat_lease_client::HttpResponse;

// The wire types + pure wire decoders moved to the protocol tier (issue
// 088); the re-export keeps every `kat_account_client::X` path resolving
// unchanged. Moved: the account-id derivation the service indexes by, the
// parsed service reads, and the §8.3 welcome gate + its evaluation.
pub use crate::kat_protocol_account::{
    evaluate_welcome, parse_balance, parse_ledger_health, parse_mining_health,
    parse_pending_durable, service_account_hex, service_info_line, LedgerHealth, MiningHealth,
    SERVICE_ACCOUNT_HEX_LEN, ServiceBalance, WelcomeGate,
};

/// The rendered welcome block for a verdict.
///
/// Caveat-#15 copy on every branch: no `$`, KAT figures labelled service
/// credit, and the two non-`Open` branches state plainly that no health
/// claim is being made — pinned by
/// [`tests::welcome_gate_lines_obey_the_copy_rule_in_every_state`].
#[must_use]
pub fn welcome_gate_lines(gate: &WelcomeGate) -> Vec<String> {
    match gate {
        WelcomeGate::Open {
            balance_micro,
            epoch,
            unproven,
        } => vec![
            "welcome to the KAT network — this account is live on the service ledger".to_string(),
            format!(
                "  network:   all planes healthy, ledger settled at epoch {epoch} \
                 ({unproven} record(s) awaiting proof)"
            ),
            format!(
                "  balance:   {} service credit (service ledger — authoritative)",
                crate::kat_payreq_client::render_kat(*balance_micro)
            ),
        ],
        WelcomeGate::Withheld { reason } => vec![
            "welcome held back — the network was read and it is not ready yet:".to_string(),
            format!("  {reason}"),
        ],
        WelcomeGate::Unknown { reason } => vec![
            "welcome held back — the service plane could not be read, so no claim is made about \
             sync or node health:"
                .to_string(),
            format!("  {reason}"),
            "  the lines below are this machine's LOCAL view only".to_string(),
        ],
    }
}

/// The §8.5 history, served by the service ledger instead of the local
/// projection (T0.2's `kat_account::history_lines` is the offline
/// fallback).
///
/// **What `/balance` can and cannot say, said out loud.** The route serves
/// `{account_id, balance_micro, granted}` — an authoritative *position*,
/// not an event log. So the earned line reports the grant STATE, the spent
/// line stays the local per-repo meter (the only place a burn is recorded
/// today), and the copy does not imply a server-side transaction list
/// exists. Fabricating one would be the G9d violation T0.2 avoided.
///
/// Unit discipline: `balance_micro` is micro-units, `spent_kat` is whole
/// KAT. They are rendered by their own renderers and never added.
#[must_use]
pub fn service_history_lines(
    balance: &ServiceBalance,
    spent_kat: u64,
    metered_tokens: u64,
) -> Vec<String> {
    vec![
        "history (service ledger — authoritative; the burn column is this repo's meter):"
            .to_string(),
        format!(
            "  earned:    {} (free-tier grant, once per account key)",
            if balance.granted {
                "grant applied"
            } else {
                "grant not applied yet"
            }
        ),
        format!(
            "  spent:     {} service credit ({} tokens metered, this repo)",
            crate::kat_payreq_client::render_kat(i64::try_from(spent_kat).unwrap_or(i64::MAX)),
            metered_tokens
        ),
        format!(
            "  balance:   {} service credit (account {})",
            crate::kat_payreq_client::render_kat(balance.balance_micro),
            balance.account_id
        ),
        "  note:      /balance serves a position, not an event log — per-event earn/spend \
         history needs a service route that does not exist yet"
            .to_string(),
    ]
}

// ── the transport + the probe (kat_transport) ────────────────────────────

/// The account-channel transport (one impl per protocol — the
/// [`crate::kat_payreq_client::PayreqTransport`] pattern; the ureq HTTP
/// impl and the in-memory scripted mock in tests drive the same trait).
#[cfg(feature = "kat_transport")]
pub trait AccountTransport {
    /// `GET /health` — the ledger plane.
    fn get_ledger_health(&self) -> HttpResponse;
    /// `GET /mining/health` — the mining plane.
    fn get_mining_health(&self) -> HttpResponse;
    /// `GET /balance?acct=<32 hex>`.
    fn get_balance(&self, acct_hex: &str) -> HttpResponse;
    /// `GET /` — the service-info body (dapps Issue 029 Rider 1,
    /// display-only). The DEFAULT is a 404: "this channel does not serve
    /// the service-info route" — which renders no label, the honest
    /// degradation for transports composed from narrower planes (the
    /// wetrun adapter spans three plane reads; `/` is not one of them).
    /// [`HttpAccountTransport`] overrides with the real read.
    fn get_service_info(&self) -> HttpResponse {
        HttpResponse::Status(404)
    }
}

/// The ureq HTTP transport over the shared service base URL (the
/// `HttpPayreqTransport` shape: one bounded agent, GET-only here).
#[cfg(feature = "kat_transport")]
pub struct HttpAccountTransport {
    agent: ureq::Agent,
    base: String,
}

#[cfg(feature = "kat_transport")]
impl HttpAccountTransport {
    /// Build with a per-request timeout in seconds (connect, headers, AND
    /// body read).
    ///
    /// Keep this SHORT. `account` is an offline-capable command that now
    /// makes up to three network reads, and the probe's fail-fast rule
    /// ([`account_service_view`]) bounds the offline cost at ONE timeout —
    /// but only if the timeout itself is small.
    #[must_use]
    pub fn new(base: &str, timeout_secs: u64) -> Self {
        let t = std::time::Duration::from_secs(timeout_secs.max(1));
        let agent = ureq::AgentBuilder::new()
            .timeout(t)
            .timeout_connect(t)
            .timeout_read(t)
            .build();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
        }
    }

    fn get(&self, path: &str) -> HttpResponse {
        use std::io::Read as _;
        let url = format!("{}{path}", self.base);
        match self.agent.get(&url).call() {
            Ok(r) => match r.status() {
                304 => HttpResponse::NotModified,
                200 => {
                    let mut buf = Vec::new();
                    match r.into_reader().read_to_end(&mut buf) {
                        Ok(_) => HttpResponse::Ok(buf),
                        Err(e) => HttpResponse::Error(e.to_string()),
                    }
                }
                c => HttpResponse::Status(c),
            },
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl AccountTransport for HttpAccountTransport {
    fn get_ledger_health(&self) -> HttpResponse {
        self.get("/health")
    }

    fn get_mining_health(&self) -> HttpResponse {
        self.get("/mining/health")
    }

    fn get_balance(&self, acct_hex: &str) -> HttpResponse {
        self.get(&format!("/balance?acct={acct_hex}"))
    }

    fn get_service_info(&self) -> HttpResponse {
        self.get("/")
    }
}

/// One `GET /` read → the display label, or `None` (the service answered
/// unhappily, or the body carried nothing labelable). Display-only by
/// contract: a missing label must never read as a probe failure.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn service_line_via(transport: &dyn AccountTransport) -> Option<String> {
    match transport.get_service_info() {
        HttpResponse::Ok(b) => service_info_line(&b),
        _ => None,
    }
}

/// What the service plane said about this account (the probe's product).
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountServiceView {
    /// The §8.3 verdict.
    pub gate: WelcomeGate,
    /// The authoritative row, when it read.
    pub balance: Option<ServiceBalance>,
    /// The `GET /` deployment label (dapps Issue 029 Rider 1), when the
    /// service answered with a body that carries one. Display-only.
    pub service_line: Option<String>,
}

/// Probe the service plane for one account.
///
/// **Fail-fast on transport death.** A `HttpResponse::Error` from the FIRST
/// read means the service is unreachable, and the remaining two reads would
/// each burn a full timeout to learn the same thing — an offline `account`
/// would take 3× the timeout to print a local view. So a transport error
/// short-circuits; an HTTP *status* (the service answered, just not 200)
/// does not, because that is per-route information.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn account_service_view(
    transport: &dyn AccountTransport,
    acct_hex: &str,
) -> AccountServiceView {
    fn read(resp: HttpResponse, what: &str) -> Result<Vec<u8>, String> {
        match resp {
            HttpResponse::Ok(b) => Ok(b),
            HttpResponse::Status(c) => Err(format!("{what} answered HTTP {c}")),
            HttpResponse::NotModified => Err(format!("{what} answered 304")),
            HttpResponse::Error(e) => Err(format!("{what} is unreachable: {e}")),
        }
    }

    let ledger_body = read(transport.get_ledger_health(), "the ledger plane");
    // The short-circuit: unreachable, not merely unhappy.
    if let Err(e) = &ledger_body {
        if e.contains("unreachable") {
            return AccountServiceView {
                gate: WelcomeGate::Unknown {
                    reason: format!("the ledger plane's health did not read: {e}"),
                },
                balance: None,
                // No second timeout on a dead service: the label read
                // would answer the same `unreachable`.
                service_line: None,
            };
        }
    }
    let ledger = ledger_body.and_then(|b| parse_ledger_health(&b));
    let mining = read(transport.get_mining_health(), "the mining plane")
        .and_then(|b| parse_mining_health(&b));
    let balance =
        read(transport.get_balance(acct_hex), "the balance route").and_then(|b| parse_balance(&b));
    AccountServiceView {
        gate: evaluate_welcome(&ledger, &mining, &balance),
        balance: balance.ok(),
        // LAST (dapps Issue 029 Rider 1): an offline run still costs the
        // short-circuit's ONE timeout, never a second for the label.
        service_line: service_line_via(transport),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger_health_json(pending: u64) -> Vec<u8> {
        serde_json::json!({
            "plane": "ledger", "tick": 7, "epoch": 3,
            "pending_durable": pending, "next_segment_seq": 1
        })
        .to_string()
        .into_bytes()
    }

    // (the LIVE wire fixture + the bool/count pin + the cross-derivation pin
    // + the service-info label shapes moved to crate::kat_protocol_account —
    // they exclusively exercise the pure wire decoders)

    fn mining_health_json(unproven: u64) -> Vec<u8> {
        serde_json::json!({"plane": "mining", "unproven": unproven, "next_segment_seq": 1})
            .to_string()
            .into_bytes()
    }

    fn balance_json(acct: &str, micro: i64, granted: bool) -> Vec<u8> {
        serde_json::json!({"account_id": acct, "balance_micro": micro, "granted": granted})
            .to_string()
            .into_bytes()
    }

    /// The parses check the plane identity and refuse partial rows — an
    /// unreadable health is never "healthy".
    #[test]
    fn health_parses_check_the_plane_and_refuse_partials() {
        assert_eq!(
            parse_ledger_health(&ledger_health_json(0)).unwrap(),
            LedgerHealth {
                tick: 7,
                epoch: 3,
                pending_durable: 0
            }
        );
        assert_eq!(
            parse_mining_health(&mining_health_json(12)).unwrap(),
            MiningHealth { unproven: 12 }
        );
        // The worker fans /health out to three DOs; a lease-plane answer
        // must not be read as ledger health.
        let lease = serde_json::json!({"plane": "lease", "next_segment_seq": 4}).to_string();
        let err = parse_ledger_health(lease.as_bytes()).expect_err("refused");
        assert!(err.contains("`lease` plane"), "{err}");
        assert!(parse_mining_health(&ledger_health_json(0)).is_err());
        // Missing field → error.
        let partial = serde_json::json!({"plane": "ledger", "tick": 1, "epoch": 1}).to_string();
        assert!(parse_ledger_health(partial.as_bytes()).is_err());
        assert!(parse_ledger_health(b"{").is_err());
        // Balance parse.
        let b = parse_balance(&balance_json(&"ab".repeat(16), 1_500_000, true)).unwrap();
        assert_eq!(b.balance_micro, 1_500_000);
        assert!(b.granted);
        assert!(parse_balance(br#"{"error":"acct must be 32 hex chars"}"#).is_err());
        let no_granted = serde_json::json!({"account_id": "ab", "balance_micro": 1}).to_string();
        assert!(parse_balance(no_granted.as_bytes()).is_err());
    }

    /// The §8.3 decision table, exhaustively — including that each of the
    /// three unreadable inputs lands in `Unknown` rather than `Withheld`.
    #[test]
    fn welcome_gate_decision_table() {
        let ok_ledger = || parse_ledger_health(&ledger_health_json(0));
        let ok_mining = || parse_mining_health(&mining_health_json(5));
        let ok_balance = || parse_balance(&balance_json(&"ab".repeat(16), 2_000_000, true));

        // Open: all read, ledger settled, grant applied.
        assert_eq!(
            evaluate_welcome(&ok_ledger(), &ok_mining(), &ok_balance()),
            WelcomeGate::Open {
                balance_micro: 2_000_000,
                epoch: 3,
                unproven: 5
            }
        );

        // Withheld: the ledger has writes in flight (NOT all-synced).
        let busy = parse_ledger_health(&ledger_health_json(4));
        let g = evaluate_welcome(&busy, &ok_mining(), &ok_balance());
        assert!(matches!(g, WelcomeGate::Withheld { .. }), "{g:?}");
        assert!(welcome_gate_lines(&g)
            .join("\n")
            .contains("4 durable write"));

        // Withheld: the account has no grant row yet.
        let ungranted = parse_balance(&balance_json(&"ab".repeat(16), 0, false));
        assert!(matches!(
            evaluate_welcome(&ok_ledger(), &ok_mining(), &ungranted),
            WelcomeGate::Withheld { .. }
        ));

        // A mining backlog is NOT a sync failure — gating on it would
        // withhold the welcome permanently.
        let backlog = parse_mining_health(&mining_health_json(9_999));
        assert!(matches!(
            evaluate_welcome(&ok_ledger(), &backlog, &ok_balance()),
            WelcomeGate::Open { .. }
        ));

        // Unknown, once per unreadable input.
        let bad_ledger: Result<LedgerHealth, String> = Err("offline".to_string());
        let bad_mining: Result<MiningHealth, String> = Err("offline".to_string());
        let bad_balance: Result<ServiceBalance, String> = Err("offline".to_string());
        for g in [
            evaluate_welcome(&bad_ledger, &ok_mining(), &ok_balance()),
            evaluate_welcome(&ok_ledger(), &bad_mining, &ok_balance()),
            evaluate_welcome(&ok_ledger(), &ok_mining(), &bad_balance),
        ] {
            assert!(
                matches!(g, WelcomeGate::Unknown { .. }),
                "an unreadable plane must be Unknown, never Withheld and never Open: {g:?}"
            );
        }
    }

    /// Caveat-#15 copy rule over every gate state and the history — no
    /// `$` anywhere, and the KAT figures are labelled service credit.
    /// The two non-`Open` states must also SAY no health claim is made.
    #[test]
    fn welcome_gate_lines_obey_the_copy_rule_in_every_state() {
        let states = [
            WelcomeGate::Open {
                balance_micro: 1_500_000,
                epoch: 3,
                unproven: 0,
            },
            WelcomeGate::Withheld {
                reason: "not synced".to_string(),
            },
            WelcomeGate::Unknown {
                reason: "offline".to_string(),
            },
        ];
        for s in &states {
            for line in welcome_gate_lines(s) {
                assert!(!line.contains('$'), "no dollars beside KAT: {line}");
            }
        }
        let open = welcome_gate_lines(&states[0]).join("\n");
        assert!(open.contains("1.500000 KAT"), "{open}");
        assert!(open.contains("service credit"), "{open}");
        assert!(open.contains("welcome to the KAT network"), "{open}");
        for s in &states[1..] {
            let held = welcome_gate_lines(s).join("\n");
            assert!(
                !held.contains("welcome to the KAT network"),
                "a held-back gate must not print the welcome: {held}"
            );
        }
        assert!(welcome_gate_lines(&states[2])
            .join("\n")
            .contains("no claim is made"));

        let bal = parse_balance(&balance_json(&"ab".repeat(16), 42_000_000, true)).unwrap();
        let hist = service_history_lines(&bal, 7, 1_234);
        for line in &hist {
            assert!(!line.contains('$'), "no dollars beside KAT: {line}");
        }
        let joined = hist.join("\n");
        assert!(joined.contains("42.000000 KAT service credit"), "{joined}");
        assert!(
            joined.contains("7 KAT service credit (1234 tokens"),
            "{joined}"
        );
        assert!(joined.contains("grant applied"), "{joined}");
        // The honesty clause: do not imply an event log exists.
        assert!(joined.contains("not an event log"), "{joined}");
        let ungranted = ServiceBalance {
            granted: false,
            ..bal
        };
        assert!(service_history_lines(&ungranted, 0, 0)
            .join("\n")
            .contains("grant not applied yet"));
    }

    /// The probe against a scripted mock: three reads, the gate opens, the
    /// balance rides out. The mock drives the SAME trait the ureq
    /// transport implements.
    #[cfg(feature = "kat_transport")]
    #[test]
    fn the_probe_reads_all_four_routes_and_opens_the_gate() {
        use std::cell::RefCell;

        struct Mock {
            seen: RefCell<Vec<String>>,
        }
        impl AccountTransport for Mock {
            fn get_ledger_health(&self) -> HttpResponse {
                self.seen.borrow_mut().push("/health".into());
                HttpResponse::Ok(ledger_health_json(0))
            }
            fn get_mining_health(&self) -> HttpResponse {
                self.seen.borrow_mut().push("/mining/health".into());
                HttpResponse::Ok(mining_health_json(2))
            }
            fn get_balance(&self, acct_hex: &str) -> HttpResponse {
                self.seen
                    .borrow_mut()
                    .push(format!("/balance?acct={acct_hex}"));
                HttpResponse::Ok(balance_json(acct_hex, 5_000_000, true))
            }
            fn get_service_info(&self) -> HttpResponse {
                self.seen.borrow_mut().push("/".into());
                HttpResponse::Ok(
                    serde_json::json!({"service":"kat","env":"devnet","tick_source":"virtual"})
                        .to_string()
                        .into_bytes(),
                )
            }
        }
        let acct = "ab".repeat(16);
        let mock = Mock {
            seen: RefCell::new(Vec::new()),
        };
        let view = account_service_view(&mock, &acct);
        assert_eq!(
            view.gate,
            WelcomeGate::Open {
                balance_micro: 5_000_000,
                epoch: 3,
                unproven: 2
            }
        );
        assert_eq!(
            view.balance.as_ref().map(|b| b.balance_micro),
            Some(5_000_000)
        );
        assert_eq!(
            view.service_line.as_deref(),
            Some("service: devnet (virtual clock)"),
            "the probe carries the display label through"
        );
        assert_eq!(
            *mock.seen.borrow(),
            vec![
                "/health".to_string(),
                "/mining/health".to_string(),
                format!("/balance?acct={acct}"),
                "/".to_string(),
            ]
        );
    }

    /// Fail-fast: an unreachable service costs ONE read, not four. A
    /// three-timeout `account` is the regression this pins (an offline
    /// user waits 1× the budget, not 3×).
    #[cfg(feature = "kat_transport")]
    #[test]
    fn an_unreachable_service_short_circuits_after_one_read() {
        use std::cell::Cell;

        // Both mocks are declared up front (`items_after_statements`):
        // `Dead` answers as an unreachable host, `Refusing` answers with a
        // status — the distinction the fail-fast rule turns on.
        struct Dead {
            reads: Cell<u32>,
        }
        struct Refusing {
            reads: Cell<u32>,
        }
        impl AccountTransport for Dead {
            fn get_ledger_health(&self) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Error("connection refused".to_string())
            }
            fn get_mining_health(&self) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Error("connection refused".to_string())
            }
            fn get_balance(&self, _acct: &str) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Error("connection refused".to_string())
            }
            fn get_service_info(&self) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Error("connection refused".to_string())
            }
        }
        impl AccountTransport for Refusing {
            fn get_ledger_health(&self) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Status(500)
            }
            fn get_mining_health(&self) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Status(500)
            }
            fn get_balance(&self, _acct: &str) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Status(500)
            }
            fn get_service_info(&self) -> HttpResponse {
                self.reads.set(self.reads.get() + 1);
                HttpResponse::Status(500)
            }
        }

        let dead = Dead {
            reads: Cell::new(0),
        };
        let view = account_service_view(&dead, &"ab".repeat(16));
        assert!(
            matches!(view.gate, WelcomeGate::Unknown { .. }),
            "{:?}",
            view.gate
        );
        assert!(view.balance.is_none());
        assert!(
            view.service_line.is_none(),
            "a dead service renders no label"
        );
        assert_eq!(
            dead.reads.get(),
            1,
            "an unreachable service must cost ONE timeout"
        );

        // A non-200 STATUS is different: the service answered, so the
        // other routes still carry information and are still read — the
        // label among them, and an unhappy `/` still renders no label.
        let refusing = Refusing {
            reads: Cell::new(0),
        };
        let view = account_service_view(&refusing, &"ab".repeat(16));
        assert!(matches!(view.gate, WelcomeGate::Unknown { .. }));
        assert!(view.service_line.is_none());
        assert_eq!(refusing.reads.get(), 4);
    }
}

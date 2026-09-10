//! The refill flow (Plan 117) — `cargo heal --refill`, the dev/test faucet
//! refill: the one command a depleted account runs to get fuel again.
//!
//! **Not a new money surface.** Every wire byte it sends is a shipped
//! primitive, composed:
//!
//! ```text
//! --refill = POST /payreq/faucet (the `faucet --request` wire, golden-pinned)
//!          → POST /admin/crank/daily  (ONLY when RIIR_HEAL_KAT_CRANK_TOKEN is set)
//!          → GET /balance poll (bounded by --refill-wait, default: one read)
//!          → the 075 soft-gate input refresh (the burn ack's server balance)
//! ```
//!
//! The last step is the actual point: the Issue 075 gate reads the burn
//! ack's `last_balance_micro` — a refill by any other path left that stale,
//! so the gate stayed red until the next burn push even after the ledger
//! credited the account. Writing the server truth into the ack here lifts
//! the gate the moment the fuel lands.
//!
//! # Posture (dev/test, prod untouched)
//!
//! * The faucet + balance legs run against whatever the service URL says —
//!   the `faucet --request` precedent. The client adds no host gate: the
//!   faucet's gate is the DEPLOYMENT's `faucet_daily_cap_kat` (a zero-cap —
//!   read: production — deployment has no faucet at all; riir-dapps
//!   `a_zero_cap_deployment_has_no_faucet_at_all`), and the four ledger
//!   refusals surface verbatim.
//! * The crank leg is an ADMIN action: it fires only when the operator set
//!   `RIIR_HEAL_KAT_CRANK_TOKEN`, and **the bearer IS the authorization** —
//!   the client does not second-guess an operator who deliberately
//!   distributed a crank token to this machine. Production never
//!   distributes one, which is the whole barrier. Fail-soft: a failed or
//!   refused crank never fails the refill (the row still settles at the
//!   next hourly alarm).
//! * Mainnet is not special-cased client-side because the client cannot
//!   name it: the wetrun's host-word classifier leaves the production
//!   default Unclassifiable, so nothing auto-runs against it. The server
//!   config is the authority (Proposal 011 caveat #2; riir-dapps Plan 023
//!   moves the apex to mainnet and names devnet `devnet.ai.gist.rs`).
//!
//! # The exit contract
//!
//! The command's exit code IS its outcome (an explicit command — the burn
//! push's never-blocks contract does not apply): **0 iff the final synced
//! server balance is positive**. A refused faucet with fuel already in the
//! account (e.g. "daily cap used" after an earlier settle) is exit 0 — the
//! goal was fuel, and the fuel is there.
//!
//! # Feature shape
//!
//! `kat_service` gets the pure halves that don't name transport types (the
//! ack refresh); the exit decision + renders + flow + transport ride
//! `kat_transport` (the vote/payreq-client convention) — `RefillOutcome`
//! carries the transport-gated leg types (`FaucetRequest` / `CrankLeg` /
//! `ServiceBalance`), so its consumers gate with it. The transport COMPOSES
//! the two shipped HTTP transports —
//! [`crate::kat_payreq_client::HttpPayreqTransport`] (faucet + balance, with
//! its refusal-body rule) and [`crate::kat_wetrun::HttpWetrunTransport`]
//! (the bearer crank POST, with its fail-closed no-token posture) — so zero
//! HTTP is re-derived here.

/// The settle poll cadence for `--refill-wait` (the pay-command's 5 s).
pub const REFILL_POLL_INTERVAL_SECS: u64 = 5;

// ── the pure halves (kat_service) ────────────────────────────────────────

/// Write a server-truth balance into the soft-gate input (Plan 117 T5).
///
/// The 075 gate reads [`crate::kat_burn_client::KatBurnAck`]
/// `.last_balance_micro` — the last SYNCED server truth, never the local
/// projection. A confirmed balance read is written here verbatim, whatever
/// the value: a 0 or negative read gates the run (the account IS depleted —
/// issue 080's "server truth wins" lesson, applied to the balance leg).
/// Every other ack field (the burn watermark, the sticky-grace truth) is
/// untouched.
///
/// Returns whether the file CHANGED — the caller skips the save on `false`.
#[must_use]
pub fn record_server_balance(
    ack: &mut crate::kat_burn_client::KatBurnAck,
    balance_micro: i64,
) -> bool {
    if ack.last_balance_micro == Some(balance_micro) {
        return false;
    }
    ack.last_balance_micro = Some(balance_micro);
    true
}

/// The exit decision — **0 iff the final synced balance is positive** (the
/// module doc's contract; total function so the table stays pinnable).
/// kat_transport-gated because `RefillOutcome` carries the transport-gated
/// leg types (`FaucetRequest` / `CrankLeg` / `ServiceBalance`) — the "pure
/// half" split below the type level, not at it.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn refill_exit_code(outcome: &RefillOutcome) -> i32 {
    match &outcome.final_balance {
        Some(b) if b.balance_micro > 0 => 0,
        _ => 1,
    }
}

// ── the flow + transport (kat_transport) ─────────────────────────────────

#[cfg(feature = "kat_transport")]
use crate::kat_lease_client::HttpResponse;

/// The refill channel's transport — exactly the three routes the flow
/// drives (interface segregation: a test mock implements three methods,
/// not the wetrun's nine). The HTTP impl composes the shipped transports.
#[cfg(feature = "kat_transport")]
pub trait RefillTransport {
    /// `POST /payreq/faucet` (signed JSON body) — the payreq channel's
    /// refusal-body rule rides the composed HTTP impl (a non-200 with
    /// `{"error": …}` arrives as [`HttpResponse::Ok`] of that body).
    fn post_faucet(&self, json: &str) -> HttpResponse;
    /// `GET /balance?acct=` — the server-truth read.
    fn get_balance(&self, acct_hex: &str) -> HttpResponse;
    /// `POST /admin/crank/daily` — bearer-gated, fail-closed on the client
    /// side too (the wetrun posture: no token ⇒ an honest 401, never a
    /// request that would read like a service problem).
    fn post_crank(&self) -> HttpResponse;
}

/// The ureq HTTP transport — a composition, not a re-derivation: the
/// payreq half owns faucet + balance (and its refusal-body rule), the
/// wetrun half owns the bearer crank POST.
#[cfg(feature = "kat_transport")]
pub struct HttpRefillTransport {
    payreq: crate::kat_payreq_client::HttpPayreqTransport,
    crank: crate::kat_wetrun::HttpWetrunTransport,
}

#[cfg(feature = "kat_transport")]
impl HttpRefillTransport {
    /// Build over the shared service base URL. `crank_bearer: None` makes
    /// [`RefillTransport::post_crank`] answer 401 without dialing (the
    /// wetrun fail-closed posture).
    #[must_use]
    pub fn new(
        base: &str,
        timeout_secs: u64,
        key: ed25519_dalek::SigningKey,
        crank_bearer: Option<String>,
    ) -> Self {
        Self {
            payreq: crate::kat_payreq_client::HttpPayreqTransport::new(base, timeout_secs),
            crank: crate::kat_wetrun::HttpWetrunTransport::new(
                base,
                timeout_secs,
                key,
                crank_bearer,
            ),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl RefillTransport for HttpRefillTransport {
    fn post_faucet(&self, json: &str) -> HttpResponse {
        crate::kat_payreq_client::PayreqTransport::post_faucet(&self.payreq, json)
    }

    fn get_balance(&self, acct_hex: &str) -> HttpResponse {
        crate::kat_payreq_client::PayreqTransport::get_balance(&self.payreq, acct_hex)
    }

    fn post_crank(&self) -> HttpResponse {
        crate::kat_wetrun::WetrunTransport::post_crank(&self.crank)
    }
}

/// The authorized crank leg's outcome (`None` = no token configured — the
/// normal dev shape; the row settles at the next hourly alarm).
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrankLeg {
    /// No `RIIR_HEAL_KAT_CRANK_TOKEN` — not attempted.
    Skipped,
    /// The pass ran; the report's `faucet_settled` / `skipped_treasury`
    /// counters are the honest answer (a treasury-floor skip is the
    /// Issue-010/025 signal, surfaced not hidden).
    Ran(crate::kat_wetrun::CrankReport),
    /// The pass was attempted and did not answer with a report.
    Failed(String),
}

/// What a refill concluded. Every leg is carried, refusals included — the
/// caller renders and decides the exit code via [`refill_exit_code`].
#[cfg(feature = "kat_transport")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefillOutcome {
    /// The faucet-request leg: `Ok` (the row opened) or the refusal /
    /// transport error, verbatim. The link composes from the id + the
    /// caller's service base (the render's job — the flow stays
    /// base-free like the payreq trait).
    pub faucet: Result<crate::kat_payreq_client::FaucetRequest, String>,
    /// The authorized crank leg.
    pub crank: CrankLeg,
    /// The last successful server-truth balance read (before or after the
    /// legs — whichever is fresher).
    pub final_balance: Option<crate::kat_account_client::ServiceBalance>,
}

/// One server-truth balance read; `Err` carries the human reason.
#[cfg(feature = "kat_transport")]
fn read_balance(
    transport: &dyn RefillTransport,
    acct_hex: &str,
) -> Result<crate::kat_account_client::ServiceBalance, String> {
    let body = match transport.get_balance(acct_hex) {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => return Err(format!("the balance read answered HTTP {c}")),
        HttpResponse::NotModified => return Err("the balance read answered 304".to_string()),
        HttpResponse::Error(e) => return Err(format!("the service is unreachable ({e})")),
    };
    crate::kat_account_client::parse_balance(&body)
}

/// The refill flow (Plan 117): faucet request → optional authorized crank
/// → bounded balance poll. Signs the faucet POST with the local account
/// key; prints nothing itself (the caller renders — the link-first copy
/// rule stays testable without a transport).
///
/// `trigger_crank` is the CALLER's token decision (the bin reads the env;
/// this module stays environment-free — the wetrun caveat #2 rule). The
/// poll is a MONOTONE budget like the pay flow's: `wait_secs == 0` does one
/// after-read, no spinning; `poll_interval` keeps the test honest.
#[cfg(feature = "kat_transport")]
pub fn refill_flow(
    transport: &dyn RefillTransport,
    key: &ed25519_dalek::SigningKey,
    acct_hex: &str,
    trigger_crank: bool,
    wait_secs: u64,
    poll_interval: std::time::Duration,
) -> RefillOutcome {
    // The baseline read: a cap-used / already-open refusal with fuel already
    // in the account is a SUCCESS (exit 0), and the poll needs a baseline
    // to detect the credit against. One public GET, zero cost.
    let mut final_balance = read_balance(transport, acct_hex).ok();

    // Leg 1: the faucet request. A refusal is recorded verbatim and the
    // flow CONTINUES — the crank may still settle an earlier open row, and
    // the balance read still refreshes the gate.
    let faucet = match transport.post_faucet(&crate::kat_payreq_client::faucet_request_body(key)) {
        HttpResponse::Ok(b) => crate::kat_payreq_client::parse_faucet_request(&b).and_then(|r| {
            // The pinned-amount rule (the `faucet --request` flow's):
            // a 0 pinned amount is an unusable row, not a free pass.
            if r.amount_micro == 0 {
                Err("the ledger pinned an amount of 0 — this deployment's \
                         `faucet_daily_cap_kat` is unset; there is nothing to credit"
                    .to_string())
            } else {
                Ok(r)
            }
        }),
        HttpResponse::Status(c) => Err(format!("the service refused the request (HTTP {c})")),
        HttpResponse::NotModified => {
            Err("the service answered 304 for a faucet request".to_string())
        }
        HttpResponse::Error(e) => Err(format!("service unreachable: {e}")),
    };

    // Leg 2: the authorized crank (the bearer is the authorization — the
    // module doc's posture). Fail-soft: a failure is a rendered line, never
    // a bail; the row still settles at the next hourly alarm.
    let crank = if trigger_crank {
        match transport.post_crank() {
            HttpResponse::Ok(b) => match crate::kat_wetrun::parse_crank_report(&b) {
                Ok(report) => CrankLeg::Ran(report),
                Err(e) => CrankLeg::Failed(e),
            },
            HttpResponse::Status(c) => CrankLeg::Failed(format!(
                "HTTP {c} — the crank is bearer-gated fail-closed service-side; \
                 the row settles at the next hourly alarm instead"
            )),
            HttpResponse::NotModified => CrankLeg::Failed("the crank answered 304".to_string()),
            HttpResponse::Error(e) => CrankLeg::Failed(e),
        }
    } else {
        CrankLeg::Skipped
    };

    // Leg 3: the bounded settle poll. Breaks early the moment the balance
    // moves above the baseline (the credit landed) — a same-instant settle
    // (localnet crank leg) costs zero waits.
    let started = std::time::Instant::now();
    let budget = std::time::Duration::from_secs(wait_secs);
    loop {
        if let Ok(b) = read_balance(transport, acct_hex) {
            let above_baseline = final_balance
                .as_ref()
                .is_none_or(|base| b.balance_micro > base.balance_micro);
            final_balance = Some(b);
            if above_baseline {
                break;
            }
        }
        if started.elapsed() >= budget {
            break;
        }
        std::thread::sleep(poll_interval);
    }

    RefillOutcome {
        faucet,
        crank,
        final_balance,
    }
}

/// The `--refill` render — the refusals verbatim, the crank counters
/// honest (a `skipped_treasury` row IS the diagnosis), the balance as
/// server truth. Caveat-#15 copy: no `$`, KAT labelled service credit.
#[cfg(feature = "kat_transport")]
#[must_use]
pub fn refill_display_lines(outcome: &RefillOutcome, service_base: &str) -> Vec<String> {
    let mut lines = Vec::new();
    match &outcome.faucet {
        Ok(request) => {
            let url =
                crate::kat_payreq_client::request_url_from_id_hex(service_base, &request.id_hex);
            lines.push(format!("refill: faucet request {} opened", request.id_hex));
            lines.push(format!("  link:     {url}"));
            lines.push(format!(
                "  amount:   {} service credit — PINNED by the deployment's daily faucet cap",
                crate::kat_payreq_client::render_kat(request.amount_micro)
            ));
        }
        Err(e) => {
            lines.push(format!("refill: the faucet request did not open — {e}"));
        }
    }
    match &outcome.crank {
        CrankLeg::Skipped => lines.push(
            "  crank:    skipped (set RIIR_HEAL_KAT_CRANK_TOKEN to force a settle pass now; \
             otherwise the row settles at the next hourly alarm)"
                .to_string(),
        ),
        CrankLeg::Ran(report) => {
            lines.push(format!(
                "  crank:    ran — faucet_settled {}, credited {}, skipped_treasury {} \
                 (skipped_faucet_cap {})",
                report.faucet_settled,
                report.credited,
                report.skipped_treasury,
                report.skipped_faucet_cap
            ));
            if report.skipped_treasury > 0 || report.needs_alert {
                lines.push(
                    "  crank:    the treasury is at/below its grant floor — nothing credits \
                     until it is funded (riir-dapps Issue 010/025)"
                        .to_string(),
                );
            }
        }
        CrankLeg::Failed(e) => {
            lines.push(format!("  crank:    failed — {e}"));
        }
    }
    match &outcome.final_balance {
        Some(b) => lines.push(format!(
            "  kat-fuel: {} (server truth, account {})",
            crate::kat_payreq_client::render_kat(b.balance_micro),
            b.account_id
        )),
        None => lines.push(
            "  kat-fuel: UNREADABLE — the balance read failed; `cargo heal account` \
             retries once the service answers"
                .to_string(),
        ),
    }
    lines
}

#[cfg(all(test, feature = "kat_transport"))]
mod tests {
    use super::*;
    use crate::kat_account_client::ServiceBalance;
    use std::cell::RefCell;

    /// A scripted transport: one faucet answer, one crank answer, and a
    /// balance QUEUE the flow drains (the scripted-mock convention — the
    /// vote/stake tests' shape, narrowed to the three refill routes).
    struct Mock {
        faucet: RefCell<Vec<HttpResponse>>,
        crank: RefCell<Vec<HttpResponse>>,
        balances: RefCell<Vec<HttpResponse>>,
    }

    impl Mock {
        fn balance(micro: i64) -> HttpResponse {
            HttpResponse::Ok(
                serde_json::json!({
                    "account_id": "a".repeat(32),
                    "balance_micro": micro,
                    "granted": true,
                })
                .to_string()
                .into_bytes(),
            )
        }

        fn faucet_ok() -> HttpResponse {
            HttpResponse::Ok(
                serde_json::json!({
                    "request_id": "b".repeat(32),
                    "amount_micro": 10_000_000,
                    "expiry": 1234,
                })
                .to_string()
                .into_bytes(),
            )
        }

        fn crank_ok() -> HttpResponse {
            HttpResponse::Ok(
                serde_json::json!({
                    "date": "2026-09-09", "credited": 1, "faucet_settled": 1,
                    "granted_micro": 10_000_000, "expired": 0,
                    "skipped_already": 0, "skipped_dormant": 0, "skipped_budget": 0,
                    "skipped_treasury": 0, "skipped_faucet_cap": 0, "needs_alert": false,
                })
                .to_string()
                .into_bytes(),
            )
        }
    }

    impl RefillTransport for Mock {
        fn post_faucet(&self, _json: &str) -> HttpResponse {
            self.faucet
                .borrow_mut()
                .pop()
                .unwrap_or(HttpResponse::Status(500))
        }
        fn get_balance(&self, _acct: &str) -> HttpResponse {
            let mut q = self.balances.borrow_mut();
            if q.len() > 1 {
                q.remove(0)
            } else {
                // Exhausted script → keep answering with the LAST value (a
                // steady server) so the poll loop terminates on budget.
                q.first().cloned().unwrap_or_else(|| Self::balance(0))
            }
        }
        fn post_crank(&self) -> HttpResponse {
            self.crank
                .borrow_mut()
                .pop()
                .unwrap_or(HttpResponse::Status(500))
        }
    }

    fn key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[7u8; 32])
    }

    #[test]
    fn record_server_balance_writes_the_gate_input_and_nothing_else() {
        let mut ack = crate::kat_burn_client::KatBurnAck {
            version: crate::kat_burn_client::BURN_ACK_VERSION,
            burn_kat_total: 42,
            meter_watermark_tokens: 7,
            acked_unix: 99,
            last_balance_micro: Some(0),
            contributed: Some(false),
            first_commitment_hex: None,
        };
        assert!(record_server_balance(&mut ack, 10_000_000));
        assert_eq!(ack.last_balance_micro, Some(10_000_000));
        assert_eq!(ack.burn_kat_total, 42);
        assert_eq!(ack.contributed, Some(false));
        assert_eq!(ack.version, crate::kat_burn_client::BURN_ACK_VERSION);
        // Idempotent: the same truth again changes nothing (no save).
        assert!(!record_server_balance(&mut ack, 10_000_000));
        // And a depleted read is WRITTEN — server truth wins (issue 080).
        assert!(record_server_balance(&mut ack, 0));
        assert_eq!(ack.last_balance_micro, Some(0));
    }

    #[test]
    fn the_exit_code_is_the_fuel_verdict() {
        let ok = |micro: i64| RefillOutcome {
            faucet: Ok(crate::kat_payreq_client::FaucetRequest {
                id_hex: "b".repeat(32),
                amount_micro: 1,
                expiry: 1,
            }),
            crank: CrankLeg::Skipped,
            final_balance: Some(ServiceBalance {
                account_id: "a".repeat(32),
                balance_micro: micro,
                granted: true,
            }),
        };
        assert_eq!(refill_exit_code(&ok(1)), 0);
        assert_eq!(refill_exit_code(&ok(10_000_000)), 0);
        // Depleted / unreadable — never an exit 0.
        assert_eq!(refill_exit_code(&ok(0)), 1);
        assert_eq!(refill_exit_code(&ok(-5)), 1);
        let unreadable = RefillOutcome {
            final_balance: None,
            ..ok(1)
        };
        assert_eq!(refill_exit_code(&unreadable), 1);
    }

    #[test]
    fn the_happy_path_opens_sets_and_confirms() {
        let t = Mock {
            faucet: RefCell::new(vec![Mock::faucet_ok()]),
            crank: RefCell::new(vec![Mock::crank_ok()]),
            balances: RefCell::new(vec![Mock::balance(0), Mock::balance(10_000_000)]),
        };
        let out = refill_flow(
            &t,
            &key(),
            &"a".repeat(32),
            true,
            0,
            std::time::Duration::from_millis(1),
        );
        assert!(out.faucet.is_ok(), "faucet leg: {:?}", out.faucet);
        assert_eq!(
            out.crank,
            CrankLeg::Ran(crate::kat_wetrun::CrankReport {
                credited: 1,
                faucet_settled: 1,
                granted_micro: 10_000_000,
                ..crate::kat_wetrun::CrankReport::default()
            })
        );
        assert_eq!(
            out.final_balance.as_ref().map(|b| b.balance_micro),
            Some(10_000_000)
        );
        assert_eq!(refill_exit_code(&out), 0);
    }

    #[test]
    fn a_refused_faucet_still_reads_the_balance_and_decides() {
        let t = Mock {
            faucet: RefCell::new(vec![HttpResponse::Ok(
                br#"{"error":"the daily faucet cap is already used for today"}"#.to_vec(),
            )]),
            crank: RefCell::new(vec![]),
            balances: RefCell::new(vec![Mock::balance(0), Mock::balance(9_000_000)]),
        };
        let out = refill_flow(
            &t,
            &key(),
            &"a".repeat(32),
            false, // no token → the crank leg is skipped, not failed
            0,
            std::time::Duration::from_millis(1),
        );
        assert!(
            out.faucet
                .as_ref()
                .is_err_and(|e| e.contains("daily faucet cap")),
            "the ledger's words must survive verbatim: {:?}",
            out.faucet
        );
        assert_eq!(out.crank, CrankLeg::Skipped);
        assert_eq!(
            out.final_balance.as_ref().map(|b| b.balance_micro),
            Some(9_000_000)
        );
        // Fuel already in the account → the goal IS met despite the refusal.
        assert_eq!(refill_exit_code(&out), 0);
    }

    #[test]
    fn a_zero_pinned_amount_is_not_a_free_pass() {
        let t = Mock {
            faucet: RefCell::new(vec![HttpResponse::Ok(
                serde_json::json!({
                    "request_id": "b".repeat(32),
                    "amount_micro": 0,
                    "expiry": 1,
                })
                .to_string()
                .into_bytes(),
            )]),
            crank: RefCell::new(vec![]),
            balances: RefCell::new(vec![Mock::balance(0)]),
        };
        let out = refill_flow(
            &t,
            &key(),
            &"a".repeat(32),
            false,
            0,
            std::time::Duration::from_millis(1),
        );
        assert!(
            out.faucet
                .as_ref()
                .is_err_and(|e| e.contains("amount of 0")),
            "the pinned-zero rule must hold: {:?}",
            out.faucet
        );
        assert_eq!(refill_exit_code(&out), 1);
    }

    #[test]
    fn the_poll_breaks_early_when_the_credit_lands() {
        let t = Mock {
            faucet: RefCell::new(vec![Mock::faucet_ok()]),
            crank: RefCell::new(vec![Mock::crank_ok()]),
            // Baseline 0, then two depleted reads, then the credit.
            balances: RefCell::new(vec![
                Mock::balance(0),
                Mock::balance(0),
                Mock::balance(0),
                Mock::balance(10_000_000),
            ]),
        };
        let started = std::time::Instant::now();
        let out = refill_flow(
            &t,
            &key(),
            &"a".repeat(32),
            true,
            60, // a budget a spinning loop would burn through
            std::time::Duration::from_millis(5),
        );
        assert_eq!(refill_exit_code(&out), 0);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(30),
            "the poll must break on the credit, not on the budget: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn the_display_lines_obey_the_copy_rule_and_surface_the_treasury_signal() {
        let ok = RefillOutcome {
            faucet: Ok(crate::kat_payreq_client::FaucetRequest {
                id_hex: "b".repeat(32),
                amount_micro: 10_000_000,
                expiry: 1234,
            }),
            crank: CrankLeg::Ran(crate::kat_wetrun::CrankReport {
                faucet_settled: 0,
                skipped_treasury: 1,
                needs_alert: true,
                ..crate::kat_wetrun::CrankReport::default()
            }),
            final_balance: Some(ServiceBalance {
                account_id: "a".repeat(32),
                balance_micro: 0,
                granted: true,
            }),
        };
        let lines = refill_display_lines(&ok, "https://heal.example");
        let text = lines.join("\n");
        assert!(!text.contains('$'), "no $ beside a KAT figure:\n{text}");
        assert!(text.contains("faucet request"), "the leg is named:\n{text}");
        assert!(
            text.contains("skipped_treasury 1") && text.contains("grant floor"),
            "the Issue-010/025 signal must surface, not hide:\n{text}"
        );
        assert!(
            text.contains("0.000000 KAT"),
            "the balance renders in KAT:\n{text}"
        );
        assert!(
            text.contains("https://heal.example/payreq/"),
            "the link composes from the caller's base:\n{text}"
        );
    }
}

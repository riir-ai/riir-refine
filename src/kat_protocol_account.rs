//! Wire types for account — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).
//!
//! Extracted verbatim from [`crate::kat_account_client`] (which re-exports
//! everything below, so every existing path is unchanged): the account-id
//! derivation the service indexes by, the parsed service-read DTOs and
//! their pure wire decoders, and the three-state §8.3 welcome verdict.
//! Only pure items moved — the rendered copy (`welcome_gate_lines` /
//! `service_history_lines`), the transport trait, the ureq impl, and the
//! probe stayed in the client module.

// ── the account-id derivation the service actually indexes by ────────────

/// The number of hex chars the service's `/balance?acct=` accepts: the
/// ledger's `AccountId` is **16 bytes**.
pub const SERVICE_ACCOUNT_HEX_LEN: usize = 32;

/// The service-side account id for a local 64-hex account id.
///
/// **This is a real trap, not a formality.** [`crate::kat_account::account_id_for`]
/// is the FULL 32-byte BLAKE3 of the pubkey (64 hex chars), while the KAT
/// ledger — and therefore `/balance`, `/payreq` and the sync plane's
/// `submitter_for` — keys on `blake3(pubkey)[..16]` (32 hex chars). Passing
/// the 64-hex form to `/balance` is a 400, and inventing a different
/// truncation would query an account no route can settle from.
///
/// Truncating the hex string is equivalent to hex-encoding the truncated
/// digest, and that equivalence is pinned against
/// [`crate::kat_payreq_client::account_for_pubkey`] by
/// [`tests::the_service_account_id_is_the_payreq_derivation`].
///
/// `None` when the input is not at least [`SERVICE_ACCOUNT_HEX_LEN`] chars
/// of lowercase hex (a silent truncation of junk would produce a
/// well-formed query for a nonexistent account).
#[must_use]
pub fn service_account_hex(account_id: &str) -> Option<&str> {
    let head = account_id.get(..SERVICE_ACCOUNT_HEX_LEN)?;
    head.bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        .then_some(head)
}

// ── the parsed service reads ─────────────────────────────────────────────

/// The ledger's authoritative account row — `GET /balance?acct=`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceBalance {
    /// The account id the service echoed back (32 hex chars).
    pub account_id: String,
    /// The balance in **micro-units** (1 KAT = 10⁶ micro). Note the unit
    /// difference from the local projections, which are whole KAT.
    pub balance_micro: i64,
    /// Whether the once-per-key free-tier grant has been applied.
    pub granted: bool,
}

/// Parse `GET /balance?acct=`.
///
/// # Errors
///
/// A human-readable string when the body is not JSON, carries an `error`,
/// or is missing a field.
pub fn parse_balance(body: &[u8]) -> Result<ServiceBalance, String> {
    let v: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| "the balance response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the service refused the balance read — {err}"));
    }
    Ok(ServiceBalance {
        account_id: v
            .get("account_id")
            .and_then(serde_json::Value::as_str)
            .ok_or("the balance carried no `account_id`")?
            .to_string(),
        balance_micro: v
            .get("balance_micro")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the balance carried no `balance_micro`")?,
        granted: v
            .get("granted")
            .and_then(serde_json::Value::as_bool)
            .ok_or("the balance carried no `granted`")?,
    })
}

/// The `GET /` service-info label line — dapps Issue 029 Rider 1.
///
/// The wire (pinned by the kat-service contract test
/// `service_info_carries_tick_source_and_optional_env_label`):
/// `{"service":"kat",…[,"env":"<KAT_ENV_LABEL>"],"tick_source":"virtual"|"wall"}`
/// — `tick_source` always rides, `env` only when the deployment declares a
/// label. The client renders ONE display line and nothing else; the fields
/// are never a gate input, and a body that does not identify as the kat
/// service (or carries neither field) renders NOTHING rather than a
/// guessed label.
#[must_use]
pub fn service_info_line(body: &[u8]) -> Option<String> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    if v.get("service").and_then(serde_json::Value::as_str) != Some("kat") {
        return None;
    }
    let env = v.get("env").and_then(serde_json::Value::as_str);
    let tick = v.get("tick_source").and_then(serde_json::Value::as_str);
    match (env, tick) {
        (Some(env), Some(tick)) => Some(format!("service: {env} ({tick} clock)")),
        (Some(env), None) => Some(format!("service: {env}")),
        (None, Some(tick)) => Some(format!("service: ({tick} clock)")),
        (None, None) => None,
    }
}

/// The ledger plane's health — `GET /health`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerHealth {
    /// The durable tick row.
    pub tick: u64,
    /// The epoch derived from that tick.
    pub epoch: u64,
    /// Durable writes not yet flushed. **This is the sync half**: a
    /// nonzero count means the plane has state in flight.
    ///
    /// The wire carries a **bool** today (`riir-neuron-db`'s WAL exposes
    /// `pending_durable() -> bool` and the service serialises it straight
    /// through), and this stays a `u64` so a future counting service needs no
    /// consumer change. `true` reads as `1`. See [`parse_pending_durable`].
    pub pending_durable: u64,
}

/// The mining plane's health — `GET /mining/health`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiningHealth {
    /// Records ingested but not yet quorum-proven. Informational — a
    /// healthy plane routinely carries a backlog, so this does NOT gate
    /// the welcome (gating on it would withhold the welcome forever).
    pub unproven: u64,
}

/// Parse the ledger plane's `GET /health`.
///
/// The `plane` field is checked, not ignored: the worker fans `/health`
/// out to three different DOs, and reading a lease-plane answer as ledger
/// health would report a plane that was never asked.
///
/// # Errors
///
/// A human-readable string on bad JSON, a plane mismatch, or a missing
/// field. Any of those is "health unreadable", never "healthy".
pub fn parse_ledger_health(body: &[u8]) -> Result<LedgerHealth, String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "the health response was not JSON".to_string())?;
    let plane = v
        .get("plane")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if plane != "ledger" {
        return Err(format!(
            "the answer identified itself as the `{plane}` plane, not `ledger`"
        ));
    }
    let n = |k: &str| -> Result<u64, String> {
        v.get(k)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("the ledger health carried no `{k}`"))
    };
    Ok(LedgerHealth {
        tick: n("tick")?,
        epoch: n("epoch")?,
        pending_durable: parse_pending_durable(v.get("pending_durable"))?,
    })
}

/// `pending_durable` off the wire, as **either** a bool or a count.
///
/// **This function exists because the deployed path was unreachable.** The
/// service serialises `riir-neuron-db`'s `DoWal::pending_durable() -> bool`
/// verbatim, so `GET /health` answers `"pending_durable":true`. This consumer
/// declared a `u64` and parsed it with `as_u64`, which returns `None` for a
/// JSON bool — so the read failed with *"the ledger health carried no
/// `pending_durable`"*, the welcome gate could never reach `Open` against a
/// real service, and `wetrun`'s step-1 preflight therefore **always failed,
/// so steps 2–6 never ran at all**. Every mock-driven test passed throughout,
/// because the mock (`ledger_health_json`) serves a *number*. Found 2026-09-04
/// by the first run against a live `wrangler dev` instance (Issue 061 T4.4a);
/// same shape as T1.8's `refusal_body_or_status` hole, one route over.
///
/// Both spellings are accepted rather than swapping one for the other: `bool`
/// is what ships, a count is what the field is *named* for, and accepting both
/// means neither side has to move first. The mapping is the obvious one —
/// `true` is "state in flight", i.e. `1`.
///
/// # Errors
///
/// Absent, or a JSON type that is neither bool nor unsigned number. That stays
/// an error rather than defaulting to `0`, because `0` is the value that reads
/// as *settled*: a silent default here would turn an unreadable plane into a
/// confident "all synced", which is exactly the false `Open` the three-state
/// [`WelcomeGate`] exists to prevent.
pub fn parse_pending_durable(v: Option<&serde_json::Value>) -> Result<u64, String> {
    match v {
        Some(serde_json::Value::Bool(b)) => Ok(u64::from(*b)),
        Some(n) => n.as_u64().ok_or_else(|| {
            "`pending_durable` was neither a bool nor an unsigned count".to_string()
        }),
        None => Err("the ledger health carried no `pending_durable`".to_string()),
    }
}

/// Parse the mining plane's `GET /mining/health`.
///
/// # Errors
///
/// A human-readable string on bad JSON, a plane mismatch, or a missing
/// field.
pub fn parse_mining_health(body: &[u8]) -> Result<MiningHealth, String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "the health response was not JSON".to_string())?;
    let plane = v
        .get("plane")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if plane != "mining" {
        return Err(format!(
            "the answer identified itself as the `{plane}` plane, not `mining`"
        ));
    }
    Ok(MiningHealth {
        unproven: v
            .get("unproven")
            .and_then(serde_json::Value::as_u64)
            .ok_or("the mining health carried no `unproven`")?,
    })
}

// ── the welcome gate (011 §8.3) ──────────────────────────────────────────

/// The §8.3 welcome verdict — evaluated from the three real service reads,
/// never from a local guess. See the client module's docs for why this is
/// three-state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WelcomeGate {
    /// Every plane answered, the ledger is settled, and the account has
    /// its grant row: print the welcome.
    Open {
        /// The service ledger's balance, micro-units.
        balance_micro: i64,
        /// The epoch the ledger reported.
        epoch: u64,
        /// Records awaiting proof (informational, carried into the copy).
        unproven: u64,
    },
    /// The network WAS read and it does not meet the condition.
    Withheld {
        /// Which condition failed, in the operator's words.
        reason: String,
    },
    /// The network could not be read. No claim is made in either
    /// direction.
    Unknown {
        /// What failed to read.
        reason: String,
    },
}

/// Evaluate 011 §8.3's *"all-synced + nodes healthy"* gate.
///
/// A pure function of three already-parsed reads, so the whole decision
/// table is testable without a transport (the `kat_payreq_client` split).
/// Each argument is the outcome of one read: `Err` means that read failed,
/// which is [`WelcomeGate::Unknown`] — an unreadable plane is never
/// evidence of health.
#[must_use]
pub fn evaluate_welcome(
    ledger: &Result<LedgerHealth, String>,
    mining: &Result<MiningHealth, String>,
    balance: &Result<ServiceBalance, String>,
) -> WelcomeGate {
    let ledger = match ledger {
        Ok(l) => l,
        Err(e) => {
            return WelcomeGate::Unknown {
                reason: format!("the ledger plane's health did not read: {e}"),
            }
        }
    };
    let mining = match mining {
        Ok(m) => m,
        Err(e) => {
            return WelcomeGate::Unknown {
                reason: format!("the mining plane's health did not read: {e}"),
            }
        }
    };
    let balance = match balance {
        Ok(b) => b,
        Err(e) => {
            return WelcomeGate::Unknown {
                reason: format!("this account's ledger row did not read: {e}"),
            }
        }
    };
    // "all-synced": the ledger plane has nothing in flight. The mining
    // plane's `unproven` backlog is NOT a sync failure — records awaiting
    // quorum proof are the steady state, and gating on that would withhold
    // the welcome permanently.
    if ledger.pending_durable != 0 {
        return WelcomeGate::Withheld {
            reason: format!(
                "the ledger plane still has {} durable write(s) in flight — not all-synced yet",
                ledger.pending_durable
            ),
        };
    }
    // The welcome says "this account is live", so the account must exist
    // on the authoritative ledger. `granted == false` is a real, common
    // pre-state (logged in locally, never synced), not an error.
    if !balance.granted {
        return WelcomeGate::Withheld {
            reason: "the service ledger has no free-tier grant row for this account yet — \
                     run `cargo heal account` (or `cargo heal sync`) once — either \
                     claims the once-per-key free-tier grant"
                .to_string(),
        };
    }
    WelcomeGate::Open {
        balance_micro: balance.balance_micro,
        epoch: ledger.epoch,
        unproven: mining.unproven,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The bytes a real service actually sent**, captured 2026-09-04 from a
    /// live `wrangler dev` kat-service on loopback (Issue 061 T4.4a). Every
    /// other fixture in this module is hand-written, and that is precisely how
    /// the bool/count divergence survived: the mock served the type the
    /// consumer wanted. Keep this literal — do not "tidy" it into a
    /// generated helper, because its whole value is that nobody in this
    /// repo chose its shape.
    const LIVE_LEDGER_HEALTH: &[u8] =
        br#"{"plane":"ledger","tick":0,"epoch":0,"pending_durable":true,"next_segment_seq":1,"crank":null}"#;

    #[test]
    fn the_live_service_wire_parses_and_pending_durable_takes_bool_or_count() {
        // The captured body. Before the fix this returned Err and the welcome
        // gate could never reach Open on the deployed path — which made
        // wetrun's step-1 preflight fail every time, so steps 2-6 never ran.
        let h = parse_ledger_health(LIVE_LEDGER_HEALTH).expect("the LIVE wire must parse");
        assert_eq!(h.tick, 0);
        assert_eq!(h.epoch, 0);
        assert_eq!(h.pending_durable, 1, "`true` means state IS in flight");

        // `false` is the settled reading, and must be 0 — the value the
        // all-synced check compares against.
        let body = String::from_utf8(LIVE_LEDGER_HEALTH.to_vec())
            .unwrap()
            .replace("\"pending_durable\":true", "\"pending_durable\":false");
        let h = parse_ledger_health(body.as_bytes()).expect("false must parse");
        assert_eq!(h.pending_durable, 0);

        // A count still works — neither side has to move first.
        assert_eq!(parse_pending_durable(Some(&serde_json::json!(0))), Ok(0));
        assert_eq!(parse_pending_durable(Some(&serde_json::json!(7))), Ok(7));
        assert_eq!(parse_pending_durable(Some(&serde_json::json!(true))), Ok(1));
        assert_eq!(
            parse_pending_durable(Some(&serde_json::json!(false))),
            Ok(0)
        );

        // …and an unreadable value stays an ERROR, never a default 0. A 0
        // reads as "settled", so defaulting would turn an unreadable plane
        // into a confident all-synced — the false `Open` the three-state gate
        // exists to prevent.
        for bad in [
            serde_json::json!("1"),
            serde_json::json!(-1),
            serde_json::json!(null),
        ] {
            assert!(
                parse_pending_durable(Some(&bad)).is_err(),
                "{bad} must not read as a count"
            );
        }
        assert!(parse_pending_durable(None).is_err());
    }

    /// THE cross-derivation pin: the id this module sends to `/balance` is
    /// byte-for-byte the id the payreq/sync planes attribute a signature
    /// to (`blake3(pubkey)[..16]`). Computed independently on both sides —
    /// a drift here would query an account no route can settle from.
    #[test]
    fn the_service_account_id_is_the_payreq_derivation() {
        for seed in [0u8, 7, 0xC5, 0xFF] {
            let pubkey = [seed; 32];
            let local = crate::kat_account::account_id_for(&pubkey);
            let service = service_account_hex(&local).expect("64-hex id truncates");
            let payreq = crate::kat_payreq_client::hex_encode(
                &crate::kat_payreq_client::account_for_pubkey(&pubkey),
            );
            assert_eq!(service, payreq, "seed {seed}");
            assert_eq!(service.len(), SERVICE_ACCOUNT_HEX_LEN);
            // And the local id really is the LONGER form — if these ever
            // became equal, this pin would be vacuous.
            assert_eq!(local.len(), 64);
            assert_ne!(local.as_str(), service);
        }
        // Junk does not get silently truncated into a well-formed query.
        assert_eq!(service_account_hex("short"), None);
        assert_eq!(service_account_hex(&"Z".repeat(64)), None);
        assert_eq!(
            service_account_hex(&"AB".repeat(32)),
            None,
            "uppercase is not the wire form"
        );
    }

    /// The `GET /` display label (dapps Issue 029 Rider 1): every shape
    /// the service may send, and the bodies that must render NOTHING.
    #[test]
    fn service_info_label_renders_what_the_service_declares_and_nothing_else() {
        // The full labeled body (the deployed shape — wrangler sets
        // KAT_ENV_LABEL = "devnet").
        let labeled = serde_json::json!({
            "service": "kat", "planes": ["ledger", "lease", "mining"],
            "leaderboard": "/leaderboard", "env": "devnet", "tick_source": "virtual"
        })
        .to_string();
        assert_eq!(
            service_info_line(labeled.as_bytes()).as_deref(),
            Some("service: devnet (virtual clock)")
        );

        // The unlabeled deployment omits `env` — the tick is still a fact
        // the service itself declared (absence is honest, so no guess).
        let unlabeled = serde_json::json!({"service": "kat", "tick_source": "wall"}).to_string();
        assert_eq!(
            service_info_line(unlabeled.as_bytes()).as_deref(),
            Some("service: (wall clock)")
        );

        // Degenerate shapes render NOTHING — never a guessed label.
        assert_eq!(service_info_line(br#"{"service":"kat"}"#), None);
        // A body that does not identify as the kat service is not ours to
        // label (a proxy answering `/` with its own page, say).
        assert_eq!(service_info_line(br#"{"service":"other","env":"x"}"#), None);
        assert_eq!(service_info_line(b"not json at all"), None);
        assert_eq!(service_info_line(b""), None);

        // env-only still renders — the fields are independent facts.
        assert_eq!(
            service_info_line(br#"{"service":"kat","env":"localnet"}"#).as_deref(),
            Some("service: localnet")
        );
    }
}

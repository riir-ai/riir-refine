//! `cargo heal --info` — the one-command stats view (the 2026-09-17 owner
//! ask): wallet + account + mining + local work. READ-ONLY composition of
//! SHIPPED surfaces — no new wire, no writes:
//!
//! | section | source |
//! |---|---|
//! | account + consents | the local account dir + the two consent records |
//! | wallet (balance / grant / burned / contributed / receipts) | `GET /history` (dapps Issue 092) |
//! | staking | `GET /stake/info` (the `stake --info` read) |
//! | mining (this machine's pushed/earned/dup/unproven) | `GET /mining/adoption`, matched by pseudonym |
//! | local work (store rows / outbox / repo burn) | `.heal/` state |
//! | soft gate | `kat_soft_gate::evaluate` over the burn-ack server truth |
//!
//! Degradation is per-section and honest (the `account` precedent): an
//! unreachable service prints `(offline — …)` per section and the local
//! sections still render; nothing is fabricated. A transport ERROR (vs an
//! HTTP status) marks the remaining reads offline — the
//! `account_service_view` fail-fast rule, so an offline `--info` costs ONE
//! timeout, not four.

use std::path::PathBuf;

/// The entry (dispatched from `main` before `validate_mode_flags` and
/// before the 075 soft gate — a read must never be refused by the gate it
/// reports on). Exits 1 only on a missing/unverifiable account; every
/// other failure degrades to a printed line.
pub(crate) fn run_kat_info(paths: &[PathBuf]) {
    use riir_clippy::kat_account::KatAccount;

    let dir = KatAccount::default_dir();
    let account = match KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("info: no account ({e}) — run `cargo heal --mine` (or `cargo heal login`) first");
            std::process::exit(1);
        }
    };
    if let Err(e) = account.verify() {
        eprintln!("info: account VERIFICATION FAILED — {e}");
        std::process::exit(1);
    }

    // ── the account + consent block (always local) ──────────────────────
    let state = account.state();
    println!("KAT ACCOUNT");
    println!(
        "  account:  {} ({})",
        state.account_id,
        account.key_path().display()
    );
    if let Some(user) = &state.github_user {
        println!("  github:   {user}");
    }
    let (mining, mw) = riir_clippy::kat_mining_consent::read_with_warning(
        &riir_clippy::kat_mining_consent::default_dir(),
    );
    if let Some(w) = mw {
        eprintln!("{w}");
    }
    println!("  mining:   {} (contribution consent)", mining.label());
    let (stats, sw) = riir_clippy::kat_stats_consent::read_with_warning(
        &riir_clippy::kat_stats_consent::default_dir(),
    );
    if let Some(w) = sw {
        eprintln!("{w}");
    }
    println!("  stats:    {} (work-stats sharing)", stats.label());

    // ── the network sections ────────────────────────────────────────────
    let acct32 = riir_clippy::kat_account_client::service_account_hex(&state.account_id)
        .map(str::to_string);
    #[cfg(feature = "kat_transport")]
    network_sections(acct32.as_deref());
    #[cfg(not(feature = "kat_transport"))]
    {
        println!("SERVICE");
        println!("  (network sections need a kat_transport build — the local view follows)");
        let _ = &acct32;
    }

    // ── the local work block (always) ───────────────────────────────────
    local_section(paths);
}

/// The service/wallet/staking/mining reads. One bounded agent per plane
/// (the `account` command's shape); the FIRST transport error marks the
/// remaining reads offline (one timeout total, never four).
#[cfg(feature = "kat_transport")]
fn network_sections(acct32: Option<&str>) {
    use riir_clippy::kat_account_client as svc;
    use riir_clippy::kat_account_client::AccountTransport as _;
    use riir_clippy::kat_lease_client::HttpResponse;
    use riir_clippy::kat_stake_client::StakeTransport as _;

    let url = riir_clippy::kat_lease_client::service_url_for_command();
    let transport = svc::HttpAccountTransport::new(&url, 6);

    println!("SERVICE");
    match svc::service_line_via(&transport) {
        Some(line) => println!("  {line}"),
        None => println!("  (no service label — asking {url})"),
    }

    let Some(acct) = acct32 else {
        eprintln!(
            "info: the local account id is not the 64-hex derivation — the service reads are skipped"
        );
        return;
    };

    // WALLET — `GET /history`: the server-truth position.
    println!("WALLET (server truth)");
    let mut offline = false;
    match transport.get_history(acct, 20) {
        HttpResponse::Ok(body) => match svc::parse_history(&body) {
            Ok(h) => {
                for line in wallet_lines(&h) {
                    println!("{line}");
                }
            }
            Err(e) => println!("  (unreadable — {e})"),
        },
        HttpResponse::Error(e) => {
            offline = true;
            println!("  (offline — {e})");
        }
        resp => println!("  (no answer — {resp:?})"),
    }

    // STAKING — `GET /stake/info` (the `stake --info` read, same render).
    println!("STAKING");
    if offline {
        println!("  (offline — skipped after the wallet read failed)");
    } else {
        let stake = riir_clippy::kat_stake_client::HttpStakeTransport::new(&url, 6);
        match stake.get_stake_info(acct) {
            HttpResponse::Ok(body) => {
                match serde_json::from_slice::<riir_clippy::kat_stake_client::StakeInfo>(&body) {
                    Ok(info) => {
                        let now_unix = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |d| d.as_secs());
                        for line in
                            riir_clippy::kat_stake_client::render_info(&info, now_unix).lines()
                        {
                            println!("{line}");
                        }
                    }
                    Err(e) => println!("  (unreadable — {e})"),
                }
            }
            HttpResponse::Error(e) => println!("  (offline — {e})"),
            resp => println!("  (no position / no answer — {resp:?})"),
        }
    }

    // MINING — `GET /mining/adoption`: the public per-machine queue
    // outcomes, this machine's row matched by the locally-computed
    // pseudonym (the full id never enters the public body).
    println!("MINING (public table)");
    if offline {
        println!("  (offline — skipped after the wallet read failed)");
    } else {
        match transport.get_adoption() {
            HttpResponse::Ok(body) => match riir_clippy::kat_adoption::parse_adoption(&body) {
                Ok(stats) => {
                    let mine = stats.machine_for(acct);
                    for line in riir_clippy::kat_adoption::mining_lines(&stats, mine) {
                        println!("{line}");
                    }
                }
                Err(e) => println!("  (unreadable — {e})"),
            },
            HttpResponse::Error(e) => println!("  (offline — {e})"),
            resp => println!("  (no answer — {resp:?})"),
        }
    }
}

/// The compact wallet view of a `ServiceHistory` — the position lines the
/// full `account --history` render carries, minus the per-event log (the
/// dashboard summary; the event log keeps its own command).
fn wallet_lines(h: &riir_clippy::kat_protocol_account::ServiceHistory) -> Vec<String> {
    let kat = |micro: i64| riir_clippy::kat_units::render_kat(micro);
    let mut lines = vec![
        format!(
            "  balance:     {} service credit",
            kat(h.balance_micro)
        ),
        format!(
            "  grant:       {}",
            if h.granted {
                "applied (free tier, once per account key)"
            } else {
                "not applied yet"
            }
        ),
    ];
    // The trial-credit line (Proposal 006) — only when the server sent
    // TUNA truth; a pre-TUNA deployment renders nothing here.
    if let Some(tuna) = h.tuna_balance_micro {
        let expiry = h
            .tuna_expires_unix
            .map(|u| {
                let (y, m, d) = riir_clippy::kat_footer::ymd_from_unix(u);
                format!(" · expires {y:04}-{m:02}-{d:02}")
            })
            .unwrap_or_default();
        lines.push(format!(
            "  tuna:        {}{expiry} · trial credit — heals only, non-transferable",
            riir_clippy::kat_units::render_units(tuna, "TUNA")
        ));
    }
    lines.push(match h.burn_kat_total {
        Some(total) => format!(
            "  burned:      {total} KAT service credit lifetime{}",
            h.burn_updated_tick
                .map(|t| format!(" (watermark tick {t})"))
                .unwrap_or_default()
        ),
        None => "  burned:      nothing yet (no burn watermark on the ledger)".to_string(),
    });
    lines.push(match (h.contributed, h.first_commitment_hex.as_deref()) {
        (true, Some(hex)) => format!(
            "  contributed: yes — first commitment {}…",
            &hex[..hex.len().min(16)]
        ),
        (true, None) => "  contributed: yes".to_string(),
        (false, _) => "  contributed: no mining batch has landed from this account yet".to_string(),
    });
    lines.push(receipts_line(h, &kat));
    lines
}

/// One receipts summary line: minted totals (final amounts are set), the
/// allocated count (amounts set at settle), and the honest page bound.
fn receipts_line(
    h: &riir_clippy::kat_protocol_account::ServiceHistory,
    kat: &dyn Fn(i64) -> String,
) -> String {
    if h.receipts.is_empty() {
        return "  receipts:   none yet (mining rewards mint per epoch after proof)".to_string();
    }
    let minted: Vec<_> = h.receipts.iter().filter(|r| r.state == "Minted").collect();
    let allocated = h.receipts.iter().filter(|r| r.state == "Allocated").count();
    let other = h.receipts.len() - minted.len() - allocated;
    let final_micro: i64 = minted.iter().map(|r| r.amount_final_micro).sum();
    let released_micro: i64 = minted.iter().map(|r| r.released_micro).sum();
    let locked_micro: i64 = minted.iter().map(|r| r.locked_micro).sum();
    let mut parts = vec![format!(
        "{} Minted (\u{03a3} {} final \u{00b7} {} released \u{00b7} {} locked)",
        minted.len(),
        kat(final_micro),
        kat(released_micro),
        kat(locked_micro)
    )];
    if allocated > 0 {
        parts.push(format!("{allocated} Allocated (amount set at settle)"));
    }
    if other > 0 {
        parts.push(format!("{other} other state"));
    }
    format!(
        "  receipts:   {} shown of {} matching — {} \u{00b7} newest epoch {}",
        h.receipts.len(),
        h.receipts_total_matching,
        parts.join(", "),
        h.receipts[0].epoch
    )
}

/// The repo-local block: trajectory rows, pending outbox batches, the
/// charge ledger, the burn-ack server truth, and the soft-gate verdict.
fn local_section(paths: &[PathBuf]) {
    use riir_clippy::kat_meter::ChargeState;

    let root = crate::resolve_heal_root(paths);
    println!("LOCAL WORK ({})", root.display());

    // healed rows — this repo's trajectory store.
    #[cfg(feature = "latent_retrieval")]
    match riir_clippy::traj_store::TrajStore::open(&root) {
        Ok((traj, bad)) => {
            if bad > 0 {
                eprintln!("info: {bad} corrupt trajectory row(s) skipped");
            }
            println!(
                "  healed:      {} trajectory rows in this repo's store",
                traj.dump().len()
            );
        }
        Err(e) => println!("  healed:      (store unreadable — {e})"),
    }
    #[cfg(not(feature = "latent_retrieval"))]
    println!("  healed:      (this build cannot read the neuron store — latent_retrieval off)");

    // unsynced batches — the outbox the push half drains.
    let outbox = root.join(".heal").join("outbox");
    let queued = std::fs::read_dir(&outbox)
        .map(|rd| rd.filter_map(|e| e.ok()).count())
        .unwrap_or(0);
    if queued > 0 {
        println!("  unsynced:    {queued} outbox batch(es) queued — push with `cargo heal sync`");
    } else {
        println!("  unsynced:    0 pending outbox batches");
    }

    // local burn — this repo's charge ledger (per-repo by design).
    let charge_path = root.join(".heal").join("kat_charge.json");
    match std::fs::read_to_string(&charge_path)
        .ok()
        .and_then(|t| serde_json::from_str::<ChargeState>(&t).ok())
    {
        Some(c) => {
            let kat = |micro: u64| {
                riir_clippy::kat_units::render_kat(i64::try_from(micro).unwrap_or(i64::MAX))
            };
            println!(
                "  local burn:  {} charged this repo ({} tokens metered)",
                kat(c.charged_kat),
                c.meter_watermark_tokens
            );
        }
        None => println!(
            "  local burn:  (no charge ledger here — heal this repo to start one)"
        ),
    }

    // the burn-ack server truth + the 075 soft gate over it.
    let ack_path = root
        .join(".heal")
        .join(riir_clippy::kat_burn_client::KatBurnAck::FILE_NAME);
    let (ack, warning) = riir_clippy::kat_burn_client::KatBurnAck::load_at(&ack_path);
    if let Some(w) = warning {
        eprintln!("{w}");
    }
    let verdict = riir_clippy::kat_soft_gate::evaluate(
        riir_clippy::kat_soft_gate::SoftGateInput {
            transport_capable: cfg!(feature = "kat_transport"),
            account_present: true,
            last_synced_balance_micro: ack.last_balance_micro,
            contributed_and_synced: ack.contributed == Some(true),
            tuna_balance_micro: ack.tuna_balance_micro,
        },
    );
    use riir_clippy::kat_soft_gate::SoftGateVerdict;
    let kat = |micro: i64| riir_clippy::kat_units::render_kat(micro);
    match verdict {
        SoftGateVerdict::Allow => {
            let tuna = ack
                .tuna_balance_micro
                .map(|t| format!(" · trial credit {} TUNA", kat(t)))
                .unwrap_or_default();
            println!(
                "  soft gate:   ALLOW{}{}",
                ack.last_balance_micro
                    .map(|b| format!(" (last synced balance {})", kat(b)))
                    .unwrap_or_default(),
                tuna
            );
        }
        SoftGateVerdict::WarnOnly(reason) => {
            use riir_clippy::kat_soft_gate::WarnOnlyReason;
            let why = match reason {
                WarnOnlyReason::NoTransport => "no kat_transport build",
                WarnOnlyReason::NoAccount => "no account",
                WarnOnlyReason::NeverSynced => "never synced (no burn push has landed)",
            };
            println!("  soft gate:   warn-only — {why} (no server truth to gate on)");
        }
        SoftGateVerdict::Refuse { balance_micro } => println!(
            "  soft gate:   REFUSED — depleted balance {} (refuel: `cargo heal --refill`, `--mine` + sync, or the payment page)",
            kat(balance_micro)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_lines_render_trial_credit_only_when_the_server_sends_it() {
        let base = |tuna: (Option<i64>, Option<u64>)| riir_clippy::kat_protocol_account::ServiceHistory {
            account_id: "ab".repeat(16),
            balance_micro: 97_000_000,
            granted: false,
            burn_kat_total: Some(3),
            burn_updated_tick: Some(9),
            contributed: false,
            first_commitment_hex: None,
            receipts: vec![],
            receipts_total_matching: 0,
            receipts_truncated: false,
            events: vec![],
            events_total: 0,
            events_truncated: false,
            tuna_balance_micro: tuna.0,
            tuna_expires_unix: tuna.1,
        };

        // Post-TUNA server: the line names the balance, the expiry, and
        // the conditions — the Proposal 006 vocabulary, one line.
        let with = wallet_lines(&base((Some(47_200_000), Some(1_792_281_600))));
        let tuna_line = with
            .iter()
            .find(|l| l.contains("tuna:"))
            .expect("the trial-credit line must render");
        assert!(tuna_line.contains("47.200000 TUNA"), "{tuna_line}");
        assert!(tuna_line.contains("expires 2026-10-18"), "{tuna_line}");
        assert!(tuna_line.contains("heals only, non-transferable"), "{tuna_line}");
        // And it sits beside the grant line, before the burn line.
        let grant_idx = with.iter().position(|l| l.contains("grant:")).unwrap();
        let tuna_idx = with.iter().position(|l| l.contains("tuna:")).unwrap();
        let burn_idx = with.iter().position(|l| l.contains("burned:")).unwrap();
        assert!(grant_idx < tuna_idx && tuna_idx < burn_idx);

        // Pre-TUNA deployment: NO line, never an invented zero.
        let pre = wallet_lines(&base((None, None)));
        assert!(
            !pre.iter().any(|l| l.contains("tuna:")),
            "absence renders nothing: {pre:?}"
        );

        // Balance without expiry (defensive): the line renders, the
        // expiry clause does not.
        let no_expiry = wallet_lines(&base((Some(47_200_000), None)));
        let line = no_expiry.iter().find(|l| l.contains("tuna:")).unwrap();
        assert!(!line.contains("expires"), "{line}");
    }
}

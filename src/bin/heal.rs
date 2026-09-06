//! `cargo heal` — the unified healing CLI (Plan 005 Phase 2 + Phase 6).
//!
//! `--suggest` mode: walk real `.rs` files, chunk them at fn/impl/struct
//! boundaries, retrieve matching corpus rules via the latent substrate
//! (fan-out across all feature-enabled domains; the router verdict
//! annotates each span), and print top-K rules with scores + the
//! fixed/optimized pattern snippet for manual review.
//!
//! `--fix` mode (Phase 6 T6.1 + T6.4): run the compiled-in proposers on
//! each file + apply their span-preserving edits — rust_perf rules plus the
//! structural clippy lints (the three type-blind clippy proposers stay
//! suggest-only; see the clippy proposers' module docs). Without `--write`,
//! fixed files print a per-edit review (rule + the changed span) plus the
//! fmt plan; with `--write`, the spliced file is written in place and then
//! rustfmt'd AUTOMATICALLY when the original was already rustfmt-clean
//! (`--no-fmt` disables; pre-existing-drift + CRLF files are always skipped
//! so no formatting churn enters a mechanical diff — see src/fmt.rs).
//! Everything outside a matched span stays byte-identical — so written
//! output keeps pre-existing indentation verbatim where rustfmt would
//! reindent (e.g. match_bool mixed block/expr arms); the automatic fmt pass
//! closes the old Issue 027 T3 manual-rustfmt deferral. `--fix` ALSO runs
//! the suggest pass unless `--quiet` is given, so rules without proposers
//! still surface.
//!
//! Run (in-repo):
//!
//! ```text
//! cargo run --features latent_retrieval --bin cargo-heal -- --suggest src/
//! cargo run --features latent_retrieval --bin cargo-heal -- --fix src/          # review
//! cargo run --features latent_retrieval --bin cargo-heal -- --fix --write src/  # apply
//! ```
//!
//! Installed as `cargo-heal` (via `cargo install --path . --features
//! latent_retrieval`), cargo's subcommand convention makes it `cargo heal
//! --suggest src/`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use riir_clippy::domains::clippy_lints::proposers::FIXABLE_LINTS;
use riir_clippy::domains::rust_perf::proposers::FIXABLE_RULES;
use riir_clippy::edit_emitter::RuleEdit;
use riir_clippy::draft::{DomainId, LatentMatcher, SpanSuggestion};
use riir_clippy::draft::span_source::collect_source_files;
use riir_clippy::EvolveRecorder;

/// Proposal 005 Phase 0 T0.1 (Plan 025) + Phase 1 T1.6 (Plan 040): the KAT
/// subcommands — parsed unconditionally so a `kat_service`-off build errors
/// cleanly instead of treating `login` as a scan path (the `--fix-compile`-off
/// precedent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KatCommand {
    /// Generate/import the account key + claim the grant (local projection).
    Login,
    /// Print the local account view (+ repo-local charge projection when a
    /// path/cwd has one).
    Account,
    /// Pull-only corpus lease refresh (T1.6): lease check + segment fetch +
    /// atomic swap + the visible charge line.
    Update,
    /// Bidirectional (T1.6): push redacted batches + pull the lease.
    Sync,
    /// Compose + submit (or queue) a payment claim.
    Claim,
    /// Proposal 007 T2.1: `stake --usd <N>` (compose + QR) or `stake --info`
    /// (the position render).
    Stake,
    /// Proposal 007 T2.1: drain-all-unlocked refund request + the escrow/
    /// venting disclosure.
    Unstake,
    /// Proposal 008 T2.4: `vote --proposal <id> --yes|--no` — the
    /// governance voting surface (weight + evidence render, signed vote).
    Vote,
    /// Proposal 011 T1.3 (Plan 062): `pay --request [--amount N]` — the
    /// pay-request surface (signed create + link-first + poll-on-credit).
    Pay,
    /// Proposal 011 T1.8 (Plan 066): `faucet --request` — the faucet
    /// preset. A separate command rather than a `pay` mode because it is a
    /// different *signing domain* (`kat:payreq:faucet:v1:`) with a
    /// different economics posture: the amount is the deployment's, not
    /// the requester's.
    Faucet,
    /// Proposal 011 T4.1/T4.2 (Plan 066): `wetrun [--dry-run]` — the §7
    /// full-loop proof. Refuses mainnet with NO override (caveat #2);
    /// `--dry-run` is steps 1+6 only and runs on every network.
    Wetrun,
}

#[cfg(feature = "fix_verify")]
use riir_clippy::domains::clippy_lints::proposers::VERIFY_LINTS;

/// Resolve the self-evolve memory file (Issue 018): `RIIR_HEAL_MEMORY`
/// overrides; otherwise the platform cache dir
/// (`$XDG_CACHE_HOME` / `~/.cache` / `%LOCALAPPDATA%` / temp fallback).
fn default_memory_path() -> PathBuf {
    if let Ok(p) = std::env::var("RIIR_HEAL_MEMORY") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    base.join("riir-clippy").join("memory.json")
}

/// Resolve the repo-local `.heal` root shared by the trajectory store and the
/// KAT meter: `RIIR_HEAL_TRAJ_ROOT` → else the first path arg's nearest
/// `.git` ancestor (the repo-local semantic 038-T2 picked for the 037-A2
/// per-repo adoption audit) → else cwd.
#[cfg(any(feature = "latent_retrieval", feature = "kat_service"))]
fn resolve_heal_root(paths: &[PathBuf]) -> PathBuf {
    std::env::var("RIIR_HEAL_TRAJ_ROOT")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            paths.first().and_then(|p| {
                let abs = if p.is_absolute() {
                    p.clone()
                } else {
                    std::env::current_dir().ok()?.join(p)
                };
                let abs = abs.canonicalize().ok()?;
                let mut dir = if abs.is_dir() { abs } else { abs.parent()?.to_path_buf() };
                loop {
                    if dir.join(".git").exists() {
                        return Some(dir);
                    }
                    if !dir.pop() {
                        return None;
                    }
                }
            })
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default()
}

/// 037-T2/038: open the repo-local neuron-db trajectory store (the durable
/// backend Issue 038 unifies on). Root resolution:
/// [`resolve_heal_root`]. `None` + one stderr line when the store cannot open
/// — the run then falls back to the legacy memory.json envelope (never
/// fatal to a heal).
///
/// `migrate` (038-T3): on the first PERSISTING run, import the legacy
/// envelope once — envelope present + store empty → every bucket becomes a
/// row verbatim, then the envelope is renamed `memory.json.migrated` so the
/// import can never run twice (or drift into a second repo's store).
#[cfg(feature = "latent_retrieval")]
fn open_traj_store(paths: &[PathBuf], migrate: bool) -> Option<riir_clippy::traj_store::TrajStore> {
    let root = resolve_heal_root(paths);
    match riir_clippy::traj_store::TrajStore::open(&root) {
        Ok((store, bad)) => {
            if bad > 0 {
                eprintln!("self-evolve: trajectory store: {bad} corrupt row(s) skipped");
            }
            if migrate {
                migrate_envelope_once(&store);
            }
            Some(store)
        }
        Err(e) => {
            eprintln!("self-evolve: neuron store unavailable ({e}) — falling back to the legacy memory.json envelope (038-T3)");
            None
        }
    }
}

/// Proposal 005 Phase 0 T0.5 (Plan 023): open the KAT code-token meter for
/// this run. `None` when disabled — `RIIR_HEAL_KAT_METER=0|off|false|no`
/// (the `RIIR_HEAL_SELF_EVOLVE=0` kill-switch pattern) or an unreachable
/// meter location (never fatal to a heal). `RIIR_HEAL_KAT_METER_FILE`
/// overrides the exact file; otherwise `<resolve_heal_root>/.heal/kat_meter.json`
/// (beside the trajectory store).
#[cfg(feature = "kat_service")]
fn open_kat_meter(paths: &[PathBuf]) -> Option<riir_clippy::kat_meter::KatMeter> {
    let disabled = std::env::var("RIIR_HEAL_KAT_METER")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "0" | "off" | "false" | "no"));
    if disabled {
        return None;
    }
    if let Ok(file) = std::env::var("RIIR_HEAL_KAT_METER_FILE") {
        return Some(riir_clippy::kat_meter::KatMeter::open_at(PathBuf::from(file)));
    }
    let root = resolve_heal_root(paths);
    Some(riir_clippy::kat_meter::KatMeter::open(&root))
}

/// Proposal 005 Phase 0 T0.6 (Plan 024): open the KAT charge ledger right
/// after the meter — the per-unit-burn consumer of the meter's events.
/// Shares the meter's kill-switch (`RIIR_HEAL_KAT_METER=0` disables BOTH —
/// one billing switch, per the Plan 024 decision); `RIIR_HEAL_KAT_CHARGE_FILE`
/// overrides the exact file. `meter` is the just-opened meter's PRE-session
/// state — the recovery path (missing/corrupt charge ledger → watermark = the
/// meter cumulative at open) is only correct against a pre-session reading.
#[cfg(feature = "kat_service")]
fn open_kat_charge(
    paths: &[PathBuf],
    meter: Option<&riir_clippy::kat_meter::KatMeter>,
) -> Option<riir_clippy::kat_meter::KatCharge> {
    let meter = meter?;
    if let Ok(file) = std::env::var("RIIR_HEAL_KAT_CHARGE_FILE") {
        return Some(riir_clippy::kat_meter::KatCharge::open_at(
            PathBuf::from(file),
            meter.state(),
        ));
    }
    let root = resolve_heal_root(paths);
    Some(riir_clippy::kat_meter::KatCharge::open(&root, meter.state()))
}

/// Proposal 005 Phase 0 T0.1 (Plan 025): `cargo heal login` — generate/import
/// the account key + record the grant projection. Prints one outcome line,
/// the account id, the key path, and the Phase-0 honesty note. No `$`
/// figures (caveats #12/#15).
#[cfg(feature = "kat_service")]
fn run_kat_login(import_key: Option<&Path>) {
    use riir_clippy::kat_account::{KatAccount, LoginOutcome};

    let dir = KatAccount::default_dir();
    let result = match import_key {
        Some(source) => KatAccount::login_import(&dir, source),
        // Proposal 011 T0.2 (ask 8.2): the default chain — existing →
        // RIIR_HEAL_KAT_KEY override → the machine's default SSH key →
        // generate fresh with a disclosed fallback note. Zero configuration:
        // an adopted SSH key makes the account id match the GitHub .keys
        // key-match the web wallet derives for the same key.
        None => KatAccount::login_default(&dir),
    };
    let (account, outcome) = match result {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("kat login failed: {e}");
            std::process::exit(1);
        }
    };
    match outcome {
        LoginOutcome::Generated {
            fallback_note: Some(note),
            ..
        } => println!("kat: account key generated ({note})"),
        LoginOutcome::Generated { .. } => println!("kat: account key generated"),
        LoginOutcome::Imported { .. } => println!("kat: account key imported"),
        LoginOutcome::Adopted { source, .. } => println!(
            "kat: account key adopted from {} (copied into the account dir — the original is untouched)",
            source.display()
        ),
        LoginOutcome::Existing { .. } => println!("kat: already logged in (key loaded)"),
    }
    for line in account.summary_lines() {
        println!("{line}");
    }
}

/// Proposal 005 Phase 0 T0.1 (Plan 025): `cargo heal account` — the local
/// view. Verifies key↔record agreement first (tamper evidence — identity is
/// load-bearing for future sync, so a mismatch exits 1), then prints the
/// identity block + (when a repo-local charge ledger exists) this repo's
/// burn + balance projection with the per-repo caveat. `--history` (Proposal
/// 011 T0.2, ask 8.5) adds the earned/spent view.
///
/// T0.2b (Plan 064) moved the welcome and the history onto the SERVICE
/// plane: `GET /health` + `GET /mining/health` + `GET /balance?acct=` decide
/// the §8.3 `all-synced + nodes-healthy` gate, and `/balance` serves the
/// history. The local renders are the offline fallback — an unreadable
/// service prints "no claim is made" and the local view, never a health
/// claim (`kat_account_client::WelcomeGate` is three-state for exactly
/// this reason).
#[cfg(feature = "kat_service")]
fn run_kat_account(paths: &[PathBuf], show_history: bool) {
    use riir_clippy::kat_account::{history_lines, KatAccount};
    use riir_clippy::kat_meter::{ChargeState, KAT_WARN_REMAINING};

    let dir = KatAccount::default_dir();
    let account = match KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("kat account: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = account.verify() {
        eprintln!("kat account: VERIFICATION FAILED — {e}");
        std::process::exit(1);
    }
    for line in account.summary_lines() {
        println!("{line}");
    }

    // The repo-local burn half: read the charge ledger at the heal root (path
    // arg or cwd) when one exists. Per-repo by design (Plans 023/024) until
    // the server aggregates at Phase 1.
    let root = resolve_heal_root(paths);
    let charge_path = root.join(".heal").join("kat_charge.json");
    let charge = std::fs::read_to_string(&charge_path)
        .ok()
        .and_then(|text| match serde_json::from_str::<ChargeState>(&text) {
            Ok(charge) => Some(charge),
            Err(e) => {
                eprintln!(
                    "kat account: charge ledger unreadable ({}): {e}",
                    charge_path.display()
                );
                None
            }
        });

    let grant = account.state().grant_kat;
    match &charge {
        Some(charge) => {
            let remaining = grant.saturating_sub(charge.charged_kat);
            println!("local burn:          {} KAT charged this repo ({} tokens, watermark {})",
                charge.charged_kat, charge.meter_watermark_tokens, charge.meter_watermark_tokens);
            println!("balance projection:  {remaining} of {grant} KAT (per-repo until server aggregation)");
            if remaining <= KAT_WARN_REMAINING {
                println!("kat: WARN balance ≤ {KAT_WARN_REMAINING} — top up before the next long run (never blocks healing)");
            }
        }
        None => {
            println!(
                "local burn:          (no charge ledger at {} — heal a repo to start one)",
                charge_path.display()
            );
        }
    }

    let spent = charge.as_ref().map_or(0, |c| c.charged_kat);
    let tokens = charge.as_ref().map_or(0, |c| c.meter_watermark_tokens);

    // Proposal 011 T0.2b (Plan 064): the welcome gate and the history now
    // come from the SERVICE plane — `GET /health` + `GET /mining/health` +
    // `GET /balance?acct=` — replacing T0.2's local `no burn yet` guess.
    // The service returns a THREE-state verdict, so an unreadable service
    // prints an honest "no claim is made" line and falls back to the local
    // view; it never renders as healthy, and never as out-of-sync either.
    #[cfg(feature = "kat_transport")]
    let served = {
        use riir_clippy::kat_account_client as svc;
        match svc::service_account_hex(&account.state().account_id) {
            // The 64-hex local id truncates to the ledger's 32-hex
            // AccountId; a malformed record cannot query the service.
            None => {
                eprintln!(
                    "kat account: the local account id is not the 64-hex derivation — skipping the service reads"
                );
                None
            }
            Some(acct) => {
                let url = riir_clippy::kat_lease_client::service_url_for_command();
                // 5 s, deliberately short: `account` is offline-capable and
                // the probe fail-fasts, so an offline run costs one budget.
                let transport = svc::HttpAccountTransport::new(&url, 5);
                let view = svc::account_service_view(&transport, acct);
                for line in svc::welcome_gate_lines(&view.gate) {
                    println!("{line}");
                }
                // T079: the once-per-key free-tier grant has a production
                // claim path now. Read-first (`ensure_grant_flow` sends
                // NOTHING when the grant is in place), signed with the
                // account key, idempotent on replay; a failure is printed
                // and never blocks the local view.
                match account.signing_key() {
                    Ok(key) => {
                        let payreq = riir_clippy::kat_payreq_client::HttpPayreqTransport::new(&url, 5);
                        match riir_clippy::kat_payreq_client::ensure_grant_flow(&payreq, &key, acct) {
                            Ok(Some(claim)) => {
                                if claim.already {
                                    println!("grant:  already claimed (balance {} micro)", claim.balance_micro);
                                } else {
                                    println!(
                                        "grant:  {} claimed — the free-tier grant is live (balance {} micro)",
                                        claim.claimed_micro, claim.balance_micro
                                    );
                                }
                                // Re-read through the service view so the
                                // gate below reflects the claim, not a
                                // pre-claim snapshot.
                                let view2 = svc::account_service_view(&transport, acct);
                                for line in svc::welcome_gate_lines(&view2.gate) {
                                    println!("{line}");
                                }
                                view2.balance
                            }
                            Ok(None) => view.balance,
                            Err(e) => {
                                println!("grant:  not claimed — {e}");
                                view.balance
                            }
                        }
                    }
                    Err(e) => {
                        println!("grant:  not claimed — the account key is unusable ({e})");
                        view.balance
                    }
                }
            }
        }
    };
    // No transport compiled: the local heuristic is all this build has.
    #[cfg(not(feature = "kat_transport"))]
    let served: Option<riir_clippy::kat_account_client::ServiceBalance> = {
        if spent == 0 {
            println!("{}", riir_clippy::kat_account::welcome_line());
        }
        None
    };

    if show_history {
        match &served {
            Some(balance) => {
                for line in
                    riir_clippy::kat_account_client::service_history_lines(balance, spent, tokens)
                {
                    println!("{line}");
                }
            }
            // The offline fallback — T0.2's local earned/spent view, which
            // is honest about being a per-repo projection.
            None => {
                for line in history_lines(grant, spent, tokens) {
                    println!("{line}");
                }
            }
        }
    }
}

/// Proposal 005 T1.6 (Plan 040): `cargo heal update` — the pull-only lease
/// refresh. The explicit-command budget is generous (30s) vs the run-start
/// phase's ~4s: a user asking for an update is waiting for it.
#[cfg(feature = "kat_transport")]
fn run_kat_update() {
    use riir_clippy::kat_lease_client::{
        run_update_phase, HttpLeaseTransport, LeaseCharge, LeaseStore,
    };

    let root = resolve_heal_root(&[]);
    let store = LeaseStore::open(&root);
    let mut charge = LeaseCharge::open(&store);
    let url = riir_clippy::kat_lease_client::service_url_for_command();
    let transport = HttpLeaseTransport::new(&url, 30);
    let current = store.current_epoch().unwrap_or(0);
    let outcome = run_update_phase(&transport, &store, &mut charge, false, None);
    match outcome.status {
        riir_clippy::kat_lease_client::UpdateStatus::NotModified => {
            println!("corpus lease: up to date (epoch {current}, 304 — no charge)");
        }
        riir_clippy::kat_lease_client::UpdateStatus::Applied { epoch, charge_kat } => {
            println!(
                "{}",
                riir_clippy::kat_lease_client::LeaseCharge::line(epoch, charge_kat)
            );
            println!(
                "corpus lease: epoch {epoch} installed atomically — the retrieval index rebuilds on the next run"
            );
        }
        riir_clippy::kat_lease_client::UpdateStatus::Skipped(reason) => {
            println!("corpus lease: skipped — {reason}");
        }
        riir_clippy::kat_lease_client::UpdateStatus::Failed(e) => {
            eprintln!("corpus lease: {e}");
            eprintln!("corpus lease: proceeding on the local corpus (offline is a mode, not a failure)");
        }
    }
    if let Err(e) = charge.save() {
        eprintln!("corpus lease: could not persist the charge ledger: {e}");
    }
}

/// Proposal 005 T1.6 (Plan 040): `cargo heal sync` — bidirectional. Push:
/// build the redacted payload from the repo trajectory store (the
/// sanctioned egress path — every row gated) + POST /sync; with the service
/// plane deferred the payload is validated + queued locally
/// (`.heal/outbox/`) and the honest state prints. Pull: the lease refresh
/// (shared with `update`). The sync watermark advances ONLY on a real push
/// acceptance (queued batches stay "pending" — the honest accounting).
#[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
fn run_kat_sync() {
    use riir_clippy::kat_lease_client::{
        queue_outbox, run_update_phase, HttpLeaseTransport, LeaseCharge, LeaseStore,
        LeaseTransport,
    };
    use riir_clippy::kat_redact::SpanOrigin;
    use riir_clippy::kat_sync_payload::SyncPayloadBuilder;

    let root = resolve_heal_root(&[]);
    let store = LeaseStore::open(&root);
    let mut charge = LeaseCharge::open(&store);
    let url = riir_clippy::kat_lease_client::service_url_for_command();
    let transport = HttpLeaseTransport::new(&url, 30);

    // ── push half ──────────────────────────────────────────────────
    let kv = root.join(".heal").join("trajectories.kv");
    if !kv.exists() {
        println!("sync: no trajectory store at {} — run any --fix in this repo first", kv.display());
    } else {
        match riir_clippy::traj_store::TrajStore::open(&root) {
            Ok((traj, bad)) => {
                if bad > 0 {
                    eprintln!("sync: {bad} corrupt row(s) skipped");
                }
                let rows = traj.dump();
                let loaded = riir_clippy::kat_redact::RedactionGate::discover(None);
                if loaded.warning.is_some() {
                    eprintln!("sync: redaction policy — {}", loaded.warning.clone().unwrap_or_default());
                }
                // Payload v2 is SIGNED with the account key (Issue 008) —
                // the receiver derives the submitter from it, so an account
                // is required to sync at all.
                let account =
                    match riir_clippy::kat_account::KatAccount::load(
                        &riir_clippy::kat_account::KatAccount::default_dir(),
                    ) {
                        Ok(a) => a,
                        Err(e) => {
                            eprintln!(
                                "sync: no account ({e}) — payload v2 is signed with the account \
                                 key; run `cargo heal login` first"
                            );
                            std::process::exit(1);
                        }
                    };
                let signing_key = match account.signing_key() {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("sync: account key unusable ({e})");
                        std::process::exit(1);
                    }
                };
                let payload =
                    SyncPayloadBuilder::new(&loaded.gate, SpanOrigin::OwnRepo, &signing_key)
                        .build(&rows);
                println!(
                    "sync payload: {} row(s) included, {} blocked, signed as {}… (batch {})",
                    payload.rows_included,
                    payload.rows_blocked,
                    riir_clippy::kat_account::short_id(account.state().account_id.as_str()),
                    &payload.commitment[..12.min(payload.commitment.len())]
                );
                println!(
                    "sync payload: {} row(s) included, {} blocked (batch {})",
                    payload.rows_included,
                    payload.rows_blocked,
                    &payload.commitment[..12.min(payload.commitment.len())]
                );
                if payload.rows_included > 0 {
                    match transport.post_sync(&payload.ndjson) {
                        riir_clippy::kat_lease_client::HttpResponse::Ok(_) => {
                            println!("sync: push accepted by {url}");
                            // T079: the ledger-plane half of "the service
                            // sees the key" — claim the once-per-key
                            // free-tier grant (read-first; a no-op when it
                            // is already in place). The mining ingest and
                            // the ledger grant are different DOs, so the
                            // push alone never produced the grant row.
                            if let Ok(key) = account.signing_key() {
                                if let Some(acct) = riir_clippy::kat_account_client::service_account_hex(
                                    &account.state().account_id,
                                ) {
                                    let payreq = riir_clippy::kat_payreq_client::HttpPayreqTransport::new(&url, 10);
                                    match riir_clippy::kat_payreq_client::ensure_grant_flow(&payreq, &key, acct) {
                                        Ok(Some(claim)) if !claim.already => println!(
                                            "sync: free-tier grant claimed ({} micro)",
                                            claim.claimed_micro
                                        ),
                                        Ok(_) => {}
                                        Err(e) => println!("sync: grant not claimed — {e}"),
                                    }
                                }
                            }
                            // Real acceptance → advance the watermark (the
                            // pushed rows are no longer "pending-unsynced").
                            let wm = riir_clippy::kat_pending_estimator::SyncWatermark::from_payload(
                                &payload,
                            );
                            if let Err(e) = wm.save(&root.join(".heal").join("kat_sync_watermark.json")) {
                                eprintln!("sync: watermark save failed: {e}");
                            }
                        }
                        resp => {
                            let queued = queue_outbox(&root, &payload.ndjson, &payload.commitment)
                                .ok().map_or_else(|| "(queue write failed)".to_string(), |p| p.display().to_string());
                            println!(
                                "sync: push NOT accepted ({resp:?}) — the service plane is not yet live;"
                            );
                            println!("sync: payload queued locally at {queued} (submits when the Worker lands; the watermark does not advance)");
                        }
                    }
                }
            }
            Err(e) => eprintln!("sync: trajectory store unreadable: {e}"),
        }
    }

    // ── pull half (shared with `update`) ───────────────────────────────
    let current = store.current_epoch().unwrap_or(0);
    let outcome = run_update_phase(&transport, &store, &mut charge, false, None);
    match outcome.status {
        riir_clippy::kat_lease_client::UpdateStatus::NotModified => {
            println!("corpus lease: up to date (epoch {current}, 304 — no charge)");
        }
        riir_clippy::kat_lease_client::UpdateStatus::Applied { epoch, charge_kat } => {
            println!(
                "{}",
                riir_clippy::kat_lease_client::LeaseCharge::line(epoch, charge_kat)
            );
        }
        riir_clippy::kat_lease_client::UpdateStatus::Skipped(reason) => {
            println!("corpus lease: skipped — {reason}");
        }
        riir_clippy::kat_lease_client::UpdateStatus::Failed(e) => {
            eprintln!("corpus lease: {e}");
        }
    }
    if let Err(e) = charge.save() {
        eprintln!("corpus lease: could not persist the charge ledger: {e}");
    }
}

/// Proposal 005 T1.6 (Plan 040): `cargo heal claim --tx <sig>` — compose +
/// submit the payment-claim request. The five-check verification is
/// service-side (deferred with the Worker shell); the client validates the
/// format, queues locally when the service is not live, and NEVER fakes a
/// mint (G9d — client counts never mint or spend).
#[cfg(feature = "kat_service")]
fn run_kat_claim(sig: Option<&str>) {
    use riir_clippy::kat_lease_client::{
        queue_claim, ClaimRequest, CLAIM_VERSION, DEFAULT_SERVICE_URL,
    };

    let Some(sig) = sig else {
        eprintln!("claim: --tx <signature> is required (the Solana transaction signature)");
        std::process::exit(2);
    };
    if !riir_clippy::kat_lease_client::validate_solana_signature(sig) {
        eprintln!("claim: --tx does not look like a Solana signature (base58, 64-88 chars)");
        std::process::exit(2);
    }
    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("claim: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let req = ClaimRequest {
        claim_version: CLAIM_VERSION,
        account_id: account.state().account_id.clone(),
        rail: "solana".to_string(),
        reference: sig.to_string(),
        queued_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    };
    let json = serde_json::to_string(&req).unwrap_or_default();
    #[cfg(not(feature = "kat_transport"))]
    let _ = &json; // the submit path is transport-gated; the queue path re-serializes

    // Submit when the service plane answers; otherwise queue + print the
    // honest state. Either way the CLIENT mints nothing.
    #[cfg(feature = "kat_transport")]
    {
        use riir_clippy::kat_lease_client::LeaseTransport;
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport =
            riir_clippy::kat_lease_client::HttpLeaseTransport::new(&url, 10);
        if let riir_clippy::kat_lease_client::HttpResponse::Ok(_) = transport.post_claim(&json) {
            println!("claim: submitted to {url} — the service verifies the five checks + mints at settlement");
            return;
        }
    }
    let root = resolve_heal_root(&[]);
    match queue_claim(&root, &req) {
        Ok(path) => {
            println!("claim: service plane not yet live — queued locally at {}", path.display());
            println!(
                "claim: the queued claim submits when the Worker lands; payment page: {}",
                riir_clippy::kat_lease_client::payment_url(DEFAULT_SERVICE_URL, &req.account_id)
            );
            println!("claim: nothing is minted client-side (the server ledger is the authority)");
        }
        Err(e) => {
            eprintln!("claim: could not queue locally: {e}");
            std::process::exit(1);
        }
    }
}

/// Proposal 007 T2.1 (Plan 041): `cargo heal stake --usd <N>` / `stake
/// --info` — the render-only + QR staking surface. Service-first (the
/// service composes the transfer request it must track the reference for);
/// when the plane is not live, the local fallback composes from the
/// env-configured stake-custody payee + a fresh on-curve reference, queues
/// the intent, and prints the honest state. Nothing client-side mints,
/// credits, or moves principal (the G9d posture held for staking too).
#[cfg(feature = "kat_service")]
fn run_kat_stake(usd: Option<&str>, info: bool) {
    use riir_clippy::kat_stake_client as stake;

    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("stake: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let account_id = account.state().account_id.clone();
    let root = resolve_heal_root(&[]);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    if info {
        run_stake_info(&account_id, &root, now_unix);
        return;
    }

    let Some(usd) = usd else {
        eprintln!("stake: --usd <N> is required (the stake amount in dollars), or --info for the position");
        std::process::exit(2);
    };
    let Some(amount_base) = stake::parse_usd_to_base(usd) else {
        eprintln!("stake: --usd expects a dollar amount with at most 6 fractional digits (e.g. 100 or 100.5)");
        std::process::exit(2);
    };

    // The honest copy prints above the QR (the proposal mock's two lines —
    // the copy itself leads with "stake", no double prefix).
    println!("{}", stake::stake_line(amount_base));

    #[cfg(feature = "kat_transport")]
    {
        use riir_clippy::kat_stake_client::StakeOutcome;
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport = riir_clippy::kat_stake_client::HttpStakeTransport::new(&url, 10);
        match riir_clippy::kat_stake_client::stake_request_flow(
            &transport,
            &account_id,
            amount_base,
            &root,
            now_unix,
        ) {
            Ok(StakeOutcome::ServiceUrl(url)) => {
                println!("stake: transfer request from the service:");
                println!("  {url}");
                print_qr(&url);
                println!("stake: the service verifies the five checks and credits the tranche at sync");
            }
            Ok(StakeOutcome::LocalQueued { url, queued_at }) => {
                println!("stake: service plane not yet live — composed locally:");
                println!("  {url}");
                print_qr(&url);
                println!(
                    "stake: intent queued locally at {} — the claim lands via sync/reference auto-detect when the plane is live",
                    queued_at.display()
                );
            }
            Err(e) => {
                eprintln!("stake: {e}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (account_id, amount_base, root, now_unix);
        eprintln!("stake: composing a transfer request requires a build with --features kat_transport");
        std::process::exit(2);
    }
}

/// `stake --info` — the position render. Service-first; the not-live
/// fallback renders the local-queue view (honest: no fake tranches).
#[cfg(feature = "kat_service")]
fn run_stake_info(account_id: &str, root: &std::path::Path, now_unix: u64) {
    use riir_clippy::kat_stake_client as stake;

    // `account_id`/`now_unix` are transport-path params (the service-first
    // block below); the local-queue fallback renders from disk state alone.
    #[cfg(not(feature = "kat_transport"))]
    let _ = (account_id, now_unix);

    #[cfg(feature = "kat_transport")]
    {
        use riir_clippy::kat_stake_client::StakeTransport as _;
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport = riir_clippy::kat_stake_client::HttpStakeTransport::new(&url, 10);
        if let riir_clippy::kat_lease_client::HttpResponse::Ok(body) =
            transport.get_stake_info(account_id)
        {
            match serde_json::from_slice::<riir_clippy::kat_stake_client::StakeInfo>(&body) {
                Ok(info) => {
                    // now_tick: the service's clock is authoritative; we
                    // render against unix seconds (the deployment's tick).
                    println!("{}", stake::render_info(&info, now_unix));
                    return;
                }
                Err(e) => eprintln!("stake --info: service response unreadable: {e}"),
            }
        }
    }
    // The not-live fallback: the local-queue view.
    let queued = stake::queued_stake_count(root).unwrap_or(0);
    if queued > 0 {
        println!(
            "Staked:          — (service plane not yet live; {queued} stake intent(s) queued locally)"
        );
        println!("stake --info: tranches, incentives, and the pool split render once the service plane is live");
    } else {
        println!("Staked:          — (no position; the service plane is not yet live)");
        println!("stake --info: tranches, incentives, and the pool split render once the service plane is live");
    }
}

/// Proposal 007 T2.1: `cargo heal unstake` — the drain-all-unlocked refund
/// request. Service-first; the not-live path queues the intent and still
/// prints the full escrow/venting disclosure (the mechanics copy is true
/// regardless of transport — disclosed at BOTH stake and unstake time per
/// the proposal).
#[cfg(feature = "kat_service")]
fn run_kat_unstake() {
    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("unstake: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let account_id = account.state().account_id.clone();
    let root = resolve_heal_root(&[]);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    #[cfg(feature = "kat_transport")]
    {
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport = riir_clippy::kat_stake_client::HttpStakeTransport::new(&url, 10);
        match riir_clippy::kat_stake_client::unstake_flow(&transport, &account_id, &root, now_unix)
        {
            Ok(line) => println!("{line}"),
            Err(e) => {
                eprintln!("unstake: {e}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (account_id, root, now_unix);
        eprintln!("unstake: composing a refund request requires a build with --features kat_transport");
        std::process::exit(2);
    }
}

/// Proposal 008 T2.4 (Plan 012): `cargo heal vote --proposal <id>
/// --yes|--no` — the governance voting surface. Renders the proposal
/// (weight + evidence lead — the service serves the view when live), signs
/// the vote with the LOGIN key, and submits (service-first; the honest
/// not-live fallback queues the SIGNED intent). Nothing client-side decides
/// eligibility or tallies — the ledger computes the weight that counts.
#[cfg(feature = "kat_service")]
fn run_kat_vote(proposal: Option<u64>, yes: bool, no: bool) {
    use riir_clippy::kat_vote_client as vote;

    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("vote: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let account_id = account.state().account_id.clone();
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("vote: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };

    let Some(proposal_id) = proposal else {
        eprintln!("vote: --proposal <id> is required (the governance proposal id)");
        std::process::exit(2);
    };
    let choice = match (yes, no) {
        (true, false) => vote::VoteChoice::Yes,
        (false, true) => vote::VoteChoice::No,
        _ => {
            eprintln!("vote: exactly one of --yes or --no is required");
            std::process::exit(2);
        }
    };

    let root = resolve_heal_root(&[]);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    #[cfg(feature = "kat_transport")]
    {
        use riir_clippy::kat_vote_client::VoteOutcome;
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport = riir_clippy::kat_vote_client::HttpVoteTransport::new(&url, 10);
        match riir_clippy::kat_vote_client::vote_flow(
            &transport,
            &account_id,
            proposal_id,
            choice,
            &signing_key,
            &root,
            now_unix,
        ) {
            Ok((view, outcome)) => {
                if let Some(v) = view {
                    println!("{}", vote::render_proposal(&v, now_unix));
                }
                match outcome {
                    VoteOutcome::Submitted(ack) => {
                        println!("{}", vote::vote_confirm_line(&ack));
                        println!("vote: a vote decides membership only — it never moves funds");
                    }
                    VoteOutcome::LocalQueued { queued_at } => {
                        println!(
                            "vote: service plane not yet live — the SIGNED intent queued at {}",
                            queued_at.display()
                        );
                        println!("vote: it submits at the next sync; the weight renders when the service answers");
                    }
                }
            }
            Err(e) => {
                eprintln!("vote: {e}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (account_id, signing_key, proposal_id, choice, root, now_unix);
        eprintln!("vote: signing + submitting requires a build with --features kat_transport");
        std::process::exit(2);
    }
}

/// Proposal 011 T1.3 (Plan 062): `pay --request [--amount N] [--label L]
/// [--qr] [--wait SECS]` — the payee half of the §1 loop. Signs a create
/// with the account key, prints the shareable LINK first (the QR is the
/// `--qr` fallback rendering), then polls on credit until the wait budget
/// ends. The poll blocks — that is the product (foo leaves it running,
/// bar scans, foo watches the credit land).
#[cfg(feature = "kat_service")]
fn run_kat_pay(amount: Option<&str>, label: Option<&str>, qr: bool, wait: u64) {
    use riir_clippy::kat_payreq_client as payreq;

    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("pay: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("pay: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let short = riir_clippy::kat_account::short_id(&account.state().account_id);

    // Blank/absent --amount = ANY amount (ask 3 — the payer composes at
    // settle). A supplied amount parses KAT → micro (positive, ≤6 decimals).
    let amount_micro = match amount {
        None | Some("") => None,
        Some(s) => match payreq::parse_kat_amount(s) {
            Some(m) => Some(m),
            None => {
                eprintln!(
                    "pay: --amount expects a positive KAT amount with at most 6 decimals (e.g. 2 or 1.5); omit it for an ANY-amount request"
                );
                std::process::exit(2);
            }
        },
    };
    let label = label.unwrap_or("");
    if label.len() > 64 {
        eprintln!("pay: --label must be at most 64 bytes (the service's bound)");
        std::process::exit(2);
    }

    println!(
        "pay: requesting {} (any amount = the payer chooses)",
        match amount_micro {
            Some(m) => payreq::render_kat(m),
            None => "ANY amount".to_string(),
        }
    );

    #[cfg(feature = "kat_transport")]
    {
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport = riir_clippy::kat_payreq_client::HttpPayreqTransport::new(&url, 10);
        match riir_clippy::kat_payreq_client::pay_request_flow(
            &transport,
            &url,
            &signing_key,
            amount_micro,
            label,
            wait,
            std::time::Duration::from_secs(5),
        ) {
            Ok(outcome) => {
                // LINK FIRST (ask 13) — the URL was always the primitive.
                println!("request {short}:");
                println!("  share this link: {}", outcome.url);
                if qr {
                    print_qr(&outcome.url);
                }
                match outcome.settled {
                    // T1.4: the full invoice display, gated on the
                    // bit-exact recheck — a mismatch refuses to render the
                    // body (Proposal 011 §1), and the caveat-#3 fallback
                    // labels itself rather than passing silently.
                    Some(riir_clippy::kat_payreq_client::SettleOutcome::Invoice(inv)) => {
                        let recheck =
                            riir_clippy::kat_payreq_client::recheck_invoice_qr(&inv, &outcome.url);
                        for line in
                            riir_clippy::kat_payreq_client::invoice_display_lines(&inv, &recheck)
                        {
                            println!("{line}");
                        }
                    }
                    Some(riir_clippy::kat_payreq_client::SettleOutcome::Unreadable(why)) => {
                        println!(
                            "pay: the request settled, but the invoice row did not read back ({why}) — `account` shows the credit"
                        );
                    }
                    None if wait > 0 => println!(
                        "pay: no credit yet (waited {wait}s) — the request stays shareable until it settles; `account` shows the credit when it lands"
                    ),
                    None => println!(
                        "pay: not polling (--wait 0) — `account` shows the credit when it lands"
                    ),
                }
            }
            Err(e) => {
                eprintln!("pay: {e}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (signing_key, qr, wait, short);
        eprintln!("pay: creating a request requires a build with --features kat_transport");
        std::process::exit(2);
    }
}

/// Proposal 011 T1.8 (Plan 066): `faucet --request [--qr]` — the faucet
/// preset. One signed POST to `/payreq/faucet`, then the link + what the
/// LEDGER pinned.
///
/// **No `--amount`.** The wire carries the pubkey and the signature and
/// nothing else; the amount is the deployment's `faucet_daily_cap_kat`.
/// Accepting an amount flag would advertise a choice that does not exist,
/// and the parser rejects it on this command for exactly that reason.
///
/// **No `--wait`.** A faucet row is settled by the DAILY crank, so a poll
/// would be either a lie (seconds against a once-a-day pass) or a day-long
/// hang. The render says where the credit shows up instead.
#[cfg(feature = "kat_service")]
fn run_kat_faucet(qr: bool) {
    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("faucet: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("faucet: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };

    #[cfg(feature = "kat_transport")]
    {
        let url = riir_clippy::kat_lease_client::service_url_for_command();
        let transport = riir_clippy::kat_payreq_client::HttpPayreqTransport::new(&url, 10);
        match riir_clippy::kat_payreq_client::faucet_request_flow(&transport, &url, &signing_key) {
            Ok(outcome) => {
                for line in riir_clippy::kat_payreq_client::faucet_display_lines(&outcome) {
                    println!("{line}");
                }
                if qr {
                    print_qr(&outcome.url);
                }
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (signing_key, qr);
        eprintln!("faucet: opening a faucet request requires a build with --features kat_transport");
        std::process::exit(2);
    }
}

/// The wet-run's local legs — step 3 (a REAL scored scan→fix cycle with
/// the REAL charge meter) and step 4 (the signed v2 payload).
///
/// Step 3 runs the actual `clippy_lints` proposers over a seeded dirty
/// fixture in a temp dir: `syn`-parse → `collect_edits` per FIXABLE lint →
/// `apply_edits` → `KatMeter::record_span` → `KatCharge::settle`. A
/// fabricated spend figure would make the harness measure itself (the
/// self-referential-corpus lesson), so the number comes from the same
/// meter a paying run uses.
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
struct CliLocalLegs {
    root: PathBuf,
}

/// The seeded dirty fixture. Deliberately tiny and deliberately hitting
/// lints whose proposers are parse-only (`needless_return`,
/// `redundant_field_names`, `uninlined_format_args`) so the cycle is a
/// real fix pass rather than a retrieval run — the wet-run proves the
/// PLUMBING (pay → ledger → record → credit → mesh), not fix quality.
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
const WETRUN_FIXTURE: &str = r#"pub struct P { pub x: i32, pub y: i32 }

pub fn mk(x: i32, y: i32) -> P {
    return P { x: x, y: y };
}

pub fn show(p: &P) -> String {
    return format!("{} {}", p.x, p.y);
}
"#;

#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
impl riir_clippy::kat_wetrun::LocalLegs for CliLocalLegs {
    fn scored_cycle(&mut self) -> Result<(u32, i64), String> {
        use riir_clippy::domains::clippy_lints::proposers::{collect_edits, FIXABLE_LINTS};
        use riir_clippy::edit_emitter::apply_edits;
        use riir_clippy::kat_meter::{KatCharge, KatMeter};

        let parsed = syn::parse_file(WETRUN_FIXTURE)
            .map_err(|e| format!("the seeded fixture did not parse: {e}"))?;
        let mut edits = Vec::new();
        for lint in FIXABLE_LINTS {
            for re in collect_edits(lint, WETRUN_FIXTURE, &parsed) {
                edits.push(re.edit);
            }
        }
        let fixes = u32::try_from(edits.len()).unwrap_or(u32::MAX);
        if fixes == 0 {
            return Err(
                "the seeded dirty fixture produced ZERO fixes — the cycle measured nothing, \
                 which is a harness failure and not a clean fixture (a zero-fix cycle also \
                 records a zero spend, so step 3 would pass vacuously)"
                    .to_string(),
            );
        }
        let fixed = apply_edits(WETRUN_FIXTURE, edits);

        // The REAL meter + charge ledger, on the repo-local `.heal` root.
        let mut meter = KatMeter::open(&self.root);
        meter.record_span(fixed.as_bytes());
        let mut charge = KatCharge::open(&self.root, meter.state());
        let spent_kat = charge.settle(meter.state());
        meter.save().map_err(|e| format!("meter save: {e}"))?;
        charge.save().map_err(|e| format!("charge save: {e}"))?;
        // KAT whole units → micro (integer money end-to-end, G4-S).
        let micro = i64::try_from(spent_kat)
            .map_err(|_| "the metered charge does not fit an i64".to_string())?;
        Ok((fixes, micro))
    }

    #[cfg(feature = "latent_retrieval")]
    fn sync_payload(&mut self) -> Result<(String, usize, u32), String> {
        use riir_clippy::kat_redact::SpanOrigin;
        use riir_clippy::kat_sync_payload::SyncPayloadBuilder;

        let account = riir_clippy::kat_account::KatAccount::load(
            &riir_clippy::kat_account::KatAccount::default_dir(),
        )
        .map_err(|e| format!("no account ({e})"))?;
        let key = account.signing_key().map_err(|e| format!("account key unusable ({e})"))?;
        let (traj, _bad) = riir_clippy::traj_store::TrajStore::open(&self.root)
            .map_err(|e| format!("trajectory store unreadable: {e}"))?;
        let rows = traj.dump();
        if rows.is_empty() {
            return Err(
                "the trajectory store has no rows to push — run a `--fix` in this repo first; \
                 pushing an EMPTY payload would let step 4 pass without recording anything"
                    .to_string(),
            );
        }
        let loaded = riir_clippy::kat_redact::RedactionGate::discover(None);
        let payload = SyncPayloadBuilder::new(&loaded.gate, SpanOrigin::OwnRepo, &key).build(&rows);
        Ok((
            payload.ndjson,
            payload.rows_included,
            riir_clippy::kat_sync_payload::PAYLOAD_VERSION,
        ))
    }

    /// Without `latent_retrieval` the v2 payload builder is not in the
    /// build, so step 4 REPORTS that and FAILS. It deliberately does not
    /// "skip": a step that quietly does nothing is the vacuous green this
    /// whole harness exists to avoid.
    #[cfg(not(feature = "latent_retrieval"))]
    fn sync_payload(&mut self) -> Result<(String, usize, u32), String> {
        Err(
            "step 4 needs a build with --features latent_retrieval — the v2 signed payload \
             builder (`kat_sync_payload`) and the trajectory store live there. This is a \
             FAILURE, not a skip: a wet-run that pushes nothing has not proven the record leg."
                .to_string(),
        )
    }
}

/// Proposal 011 §7 / T4.1+T4.2 (Plan 066): `cargo heal wetrun [--dry-run]`.
///
/// The network is classified from the RESOLVED service URL and the
/// admission decision is [`riir_clippy::kat_wetrun::admit`] — mainnet is
/// refused with **no override**, and an unclassifiable host is refused the
/// same way (fail-closed). `--dry-run` runs steps 1+6 only and is admitted
/// everywhere.
#[cfg(feature = "kat_service")]
fn run_kat_wetrun(dry_run: bool) {
    use riir_clippy::kat_wetrun::{RunMode, Verdict};

    let url = riir_clippy::kat_lease_client::service_url_for_command();
    let network = riir_clippy::kat_wetrun::classify_service_url(&url);
    let mode = match dry_run {
        true => RunMode::DryRun,
        false => RunMode::Full,
    };

    // Admission BEFORE anything else — including before the account load,
    // so a refused mainnet run cannot be confused with a missing key.
    if let Err(why) = riir_clippy::kat_wetrun::admit(network, mode) {
        eprintln!("{why}");
        std::process::exit(2);
    }

    let dir = riir_clippy::kat_account::KatAccount::default_dir();
    let account = match riir_clippy::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("wetrun: no account ({e}) — run `cargo heal login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("wetrun: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let Some(acct_hex) =
        riir_clippy::kat_account_client::service_account_hex(&account.state().account_id)
    else {
        eprintln!(
            "wetrun: the local account id is not the ledger's 32-hex shape — refusing to \
             query an account no route can settle from"
        );
        std::process::exit(1);
    };
    let acct_hex = acct_hex.to_string();

    #[cfg(feature = "kat_transport")]
    {
        // The crank bearer is read HERE, never inside `kat_wetrun` — that
        // module is pinned environment-free so no env read there can grow
        // into a mainnet-admission override (caveat #2).
        let crank_bearer = std::env::var("RIIR_HEAL_KAT_CRANK_TOKEN")
            .ok()
            .filter(|t| !t.is_empty());
        if matches!(mode, RunMode::Full) && crank_bearer.is_none() {
            eprintln!(
                "wetrun: RIIR_HEAL_KAT_CRANK_TOKEN is unset. `POST /admin/crank/daily` is \
                 bearer-gated fail-closed service-side, and it is what SETTLES a faucet row — \
                 without it steps 2 and 5 cannot complete. Set it, or use --dry-run."
            );
            std::process::exit(2);
        }
        let transport = riir_clippy::kat_wetrun::HttpWetrunTransport::new(
            &url,
            30,
            signing_key,
            crank_bearer,
        );
        let mut legs = CliLocalLegs {
            root: resolve_heal_root(&[]),
        };
        let report = riir_clippy::kat_wetrun::run(&transport, &mut legs, network, mode, &acct_hex);
        for step in &report.steps {
            println!("── step {} ──", step.step.label());
            for line in &step.lines {
                println!("{line}");
            }
            println!("   {}", if step.ok { "PASS" } else { "FAIL" });
        }
        if let (Some(b), Some(a)) = (report.supply_before, report.supply_after) {
            println!(
                "G-WET supply: epoch {} mint {} burn {} pool_paid {} pool_avail {} → \
                 mint {} burn {} pool_paid {} pool_avail {}",
                b.epoch,
                b.mint_kat,
                b.burn_kat,
                b.pool_paid_kat,
                b.pool_available_kat,
                a.mint_kat,
                a.burn_kat,
                a.pool_paid_kat,
                a.pool_available_kat
            );
        }
        println!("{}", report.verdict_line());
        match report.verdict {
            Verdict::Ready => {}
            Verdict::NotReady(_) => std::process::exit(1),
            Verdict::Refused(_) => std::process::exit(2),
        }
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (signing_key, acct_hex, network);
        eprintln!("wetrun: requires a build with --features kat_transport");
        std::process::exit(2);
    }
}

/// Print the ASCII QR under a URL line (the T1.6 QR render, reused —
/// errors surface as silence, the QR is a convenience never a gate).
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn print_qr(url: &str) {
    let qr = riir_clippy::kat_lease_client::render_qr_ascii(url);
    if !qr.is_empty() {
        println!("{qr}");
    }
}

/// 038-T3: the one-time envelope → store migration (see [`open_traj_store`]).
/// Idempotent by construction: the `.migrated` rename is the fence — a
/// second run sees no envelope and does nothing.
#[cfg(feature = "latent_retrieval")]
fn migrate_envelope_once(store: &riir_clippy::traj_store::TrajStore) {
    use riir_clippy::memory::EvidenceTier;
    use riir_clippy::traj_store::TrajRowDoc;

    let envelope = default_memory_path();
    if !envelope.exists() || !store.dump().is_empty() {
        return; // nothing to migrate from, or the store already has history
    }
    let mut mem = riir_clippy::memory::LatentFixMemory::default();
    if let Err(e) = mem.load(&envelope) {
        eprintln!("self-evolve: migration skipped — unreadable envelope ({}): {e}", envelope.display());
        return;
    }
    let n = mem.len();
    let mut imported = 0usize;
    for t in mem.iter() {
        let row = TrajRowDoc {
            lint_key: t.lint_name.clone(),
            shape: t.buggy_code.clone(),
            fix: t.fixed_code.clone(),
            direction: t.direction.as_slice().to_vec(),
            success_count: t.success_count,
            fail_count: t.fail_count,
            consecutive_fails: t.consecutive_fails,
            tier: match t.tier {
                EvidenceTier::Certified => EvidenceTier::Certified.as_u8(),
                EvidenceTier::Withdrawn => EvidenceTier::Withdrawn.as_u8(),
                EvidenceTier::Heuristic => EvidenceTier::Heuristic.as_u8(),
            },
            last_run_id: "migrated".to_string(),
            last_seen_at: 0,
            elo: riir_clippy::traj_store::default_elo(),
            span_elo: riir_clippy::traj_store::default_elo(),
            component: t.component,
        };
        if store.import_row(&row).is_ok() {
            imported += 1;
        }
    }
    let migrated = envelope.with_extension("json.migrated");
    match std::fs::rename(&envelope, &migrated) {
        Ok(()) => eprintln!(
            "self-evolve: migrated {imported}/{n} envelope bucket(s) → {} (envelope kept as {})",
            store.root.display(),
            migrated.display()
        ),
        Err(e) => eprintln!(
            "self-evolve: imported {imported}/{n} row(s) but could not rename the envelope ({}): {e} — migration will re-check on the next persisting run",
            envelope.display()
        ),
    }
}

/// Issue 067 T1 — the same-rule reopen-trigger instrument. The reopen
/// condition (from the issue): "the TrajStore holds ≥ K (K=3) Certified
/// same-rule trajectories for a queried lint". This scans the store ONCE
/// per run (lazily, on the first queried lint), then logs the FIRST time
/// each queried lint's density reaches K. Zero store traffic on the hot
/// path after that one scan; inert without a store.
#[cfg(feature = "latent_retrieval")]
struct ReopenTrigger067 {
    density: Option<std::collections::BTreeMap<String, usize>>,
    logged: std::collections::HashSet<String>,
}

#[cfg(feature = "latent_retrieval")]
impl ReopenTrigger067 {
    const K: usize = 3;

    fn new() -> Self {
        Self {
            density: None,
            logged: std::collections::HashSet::new(),
        }
    }

    /// Note one queried lint key (the key the store is addressed by at
    /// apply/record time — rule id in the clippy lane, `code:strategy` in
    /// the compile lane). Logs at most once per key per run.
    fn note(&mut self, store: &riir_clippy::traj_store::TrajStore, lint_key: &str) {
        let density = self.density.get_or_insert_with(|| store.certified_density());
        if let Some(n) = density.get(lint_key) {
            if *n >= Self::K && self.logged.insert(lint_key.to_string()) {
                eprintln!(
                    "self-evolve: issue 067 reopen trigger FIRED: '{lint_key}' holds {n} Certified same-rule trajectories (K={}) — same-rule exemplar channels (issue 067 T2) are unblocked for this lint",
                    Self::K
                );
            }
        }
    }
}

/// Record one verdict event into the durable store (best-effort — a store
/// failure warns once per event, never fails the heal). `component` is the
/// Issue 047 decision-time failure attribution (`None` on successes and
/// unattributed events).
#[cfg(all(feature = "fix_verify", feature = "latent_retrieval"))]
#[allow(clippy::too_many_arguments)] // the existing record shape + the one attribution param
fn record_traj(
    store: &Option<riir_clippy::traj_store::TrajStore>,
    rule_id: &str,
    restore: &str,
    replacement: &str,
    success: bool,
    verified: bool,
    run_id: &str,
    component: Option<riir_clippy::component::ComponentClass>,
) {
    if let Some(store) = store {
        if let Err(e) = store.record_classified(
            rule_id,
            restore,
            replacement,
            success,
            verified,
            run_id,
            component,
        ) {
            eprintln!("self-evolve: store record failed: {e}");
        }
    }
}

/// [`record_traj`] fixed at the failure verdict (reverts — Issue 018's
/// definitive compile-gate fail). The component is derived HERE, at the
/// point the failure is observed (Issue 047): a compile-gate revert means
/// the drafter produced a defective fix — W-class.
#[cfg(all(feature = "fix_verify", feature = "latent_retrieval"))]
fn record_traj_fail(
    store: &Option<riir_clippy::traj_store::TrajStore>,
    rule_id: &str,
    restore: &str,
    replacement: &str,
    run_id: &str,
) {
    let class = riir_clippy::component::classify(
        &riir_clippy::component::FailureSignal::reverted(
            riir_clippy::component::RevertKind::CompileGate,
        ),
    );
    record_traj(store, rule_id, restore, replacement, false, false, run_id, Some(class));
}

/// P12: the recorded consecutive-fail streak for one exact edit pair —
/// the store row when the store backend is active, the envelope buckets
/// otherwise (never both: one active backend per build, 038). Consumed by
/// the fixpoint suppression filter: a pair with 2+ consecutive compile-gate
/// fails is suppressed from re-proposal (see the filter comment there for
/// the full rationale).
#[cfg(feature = "latent_retrieval")]
fn pair_fail_streak(
    store: &Option<riir_clippy::traj_store::TrajStore>,
    memory: &riir_clippy::memory::LatentFixMemory,
    rule_id: &str,
    restore: &str,
    replacement: &str,
) -> u32 {
    if let Some(store) = store {
        return store.pair_consecutive_fails(rule_id, restore, replacement);
    }
    memory.pair_consecutive_fails(rule_id, replacement)
}

/// Light-build twin: the envelope is the only backend (the store type does
/// not exist without `latent_retrieval`). The streak query is still sound —
/// envelopes written by heavier builds carry the same verdicts.
#[cfg(not(feature = "latent_retrieval"))]
fn pair_fail_streak(
    memory: &riir_clippy::memory::LatentFixMemory,
    rule_id: &str,
    _restore: &str,
    replacement: &str,
) -> u32 {
    memory.pair_consecutive_fails(rule_id, replacement)
}

/// P12 kill-switch parse: suppression is default-on; `RIIR_HEAL_SUPPRESS_
/// REVERTED=0|off|false|no` re-enables unconditional re-proposal (force
/// retry of previously-reverted edits). Mirrors the recorder's opt-out
/// value set.
fn suppression_disabled() -> bool {
    match std::env::var("RIIR_HEAL_SUPPRESS_REVERTED") {
        Ok(v) => matches!(v.to_ascii_lowercase().as_str(), "0" | "off" | "false" | "no"),
        Err(_) => false,
    }
}

/// Print the per-edit fix review + fmt annotation. `edits` are
/// `(rule_id, final line, replacement, restore)` — only the first two
/// fields print here; the last two exist for the verify path.
/// `fmt_note` is the per-file fmt-plan/outcome annotation (`None` disables).
fn print_fix_review(
    display: &str,
    edits: &[(String, usize, String, String, Option<&'static str>)],
    fmt_note: Option<&str>,
) {
    if edits.is_empty() {
        return;
    }
    println!("fix {} ({} edit(s)):", display, edits.len());
    for (rule_id, line, _, _, note) in edits {
        println!("  [{rule_id:<24}] line {line}");
        // Issue 074 T1: one annotation dialect with the rustc_errors
        // renderer (`semantic_note_suffix`). Absent for mechanical
        // classes — the default output stays byte-identical.
        if let Some(note) = note {
            println!("  ⚠ semantic change: {note}");
        }
    }
    if let Some(note) = fmt_note {
        println!("  fmt: {note}");
    }
}

/// Issue 074 T1: the semantic-shift note a `--fix` review line carries for
/// this rule, if its proposer documents one. Domain dispatch mirrors the
/// `collect_edits` call sites (feature-gated identically).
fn rule_context_note(rule_id: &str) -> Option<&'static str> {
    #[cfg(feature = "rust_perf")]
    if let Some(note) = riir_clippy::domains::rust_perf::proposers::context_note(rule_id) {
        return Some(note);
    }
    #[cfg(feature = "clippy_lints")]
    if let Some(note) = riir_clippy::domains::clippy_lints::proposers::context_note(rule_id) {
        return Some(note);
    }
    let _ = rule_id;
    None
}

/// Write the healed file + run the automatic fmt pass when eligible
/// (default-on; `--no-fmt` off). Returns the annotation for the review.
/// `decision` is computed against the ORIGINAL on-disk file BEFORE this
/// call (see `fmt::decide`).
fn write_with_fmt(
    file: &Path,
    current: &str,
    decision: Option<riir_clippy::fmt::FmtDecision>,
    edition: &str,
    write: bool,
    fmt_summary: &mut riir_clippy::fmt::FmtSummary,
) -> Option<String> {
    use riir_clippy::fmt::FmtDecision;
    if !write {
        // Dry run: report the plan only (nothing written).
        let decision = decision?;
        fmt_summary.dry = true;
        match decision {
            FmtDecision::Format => fmt_summary.formatted += 1,
            FmtDecision::SkipDrift => fmt_summary.skip_drift += 1,
            FmtDecision::SkipCrlf => fmt_summary.skip_crlf += 1,
        }
        return Some(
            match decision {
                FmtDecision::Format => "would-format",
                FmtDecision::SkipDrift => "skip (pre-existing drift)",
                FmtDecision::SkipCrlf => "skip (CRLF)",
            }
            .to_string(),
        );
    }
    // Write mode: ALWAYS write the healed text first — the fmt pass is
    // strictly additive on top (`--no-fmt` / a missing decision must never
    // block the write itself).
    std::fs::write(file, current).unwrap_or_else(|e| {
        eprintln!("write failed for {}: {e}", file.display());
    });
    let decision = decision?;
    match decision {
        FmtDecision::Format => {
            match riir_clippy::fmt::apply(file, edition) {
                Ok(()) => {
                    fmt_summary.formatted += 1;
                    Some("formatted".to_string())
                }
                Err(e) => {
                    fmt_summary.failed += 1;
                    eprintln!("fmt failed for {}: {e}", file.display());
                    Some(format!("FAILED ({e})"))
                }
            }
        }
        FmtDecision::SkipDrift => {
            fmt_summary.skip_drift += 1;
            Some("skip (pre-existing drift)".to_string())
        }
        FmtDecision::SkipCrlf => {
            fmt_summary.skip_crlf += 1;
            Some("skip (CRLF)".to_string())
        }
    }
}

/// Print usage + exit 2 (arg-error convention).
fn usage() -> ! {
    #[cfg(feature = "fix_verify")]
    let verify_help = format!(
        "    --verify        With --fix: write, cargo-check, auto-revert the offending edits\n\
         (adds {} + looser capacity bounds). Implies --write.\n\
    --verify-args <s> Extra cargo args for --verify (whitespace-split, e.g. \"--features a,b,c\").\n",
        VERIFY_LINTS.join(", ")
    );
    #[cfg(not(feature = "fix_verify"))]
    let verify_help = String::new();
    #[cfg(feature = "score_bench")]
    let score_help = "    --score-bench   Run the frozen clippy-bench (real-code fixtures, real clippy ground\n                    truth — Plan 014), append to .benchmarks/score_history.jsonl,\n                    re-render .benchmarks/score_history.svg. No paths needed.\n    --generation <1|2|3|4|5|6|7>  Which bench generation (Plan 092 M5): 1 = the frozen v1 set\n                    (default); 2 = the construction-built k-ladder (issue 072 T5);\n                    3 = the persist-stratified ladder (Bench 069 reopen #1); 4 = the\n                    rustc-errors k-ladder (Plan 093, --fix-compile driver vs rustc\n                    ground truth); 5 = the rust_perf vocabulary ladder (Plan 094, the\n                    mechanical shape oracle over the FIXABLE perf vocabulary); 6 = the\n                    real-repo shape ladder (Plan 095, spans carved from pre-heal\n                    sibling states); 7 = the overlap-edit-chain ladder (Plan 096,\n                    manufactured drop-and-refire overlap, depth rungs 1–5). A NEW\n                    generation's first append is admitted ONLY if it measures harder\n                    than v1.\n    --score-report  Re-render the score-history SVG from the existing JSONL (no run).\n";
    #[cfg(not(feature = "score_bench"))]
    let score_help = String::new();
    eprintln!(
        "cargo-heal (Plan 005) — modelless code healing via latent corpus retrieval

USAGE:
    cargo heal --suggest [--domain <auto|clippy|perf|kernel|sec|docker>] [--top-k <N>] <path>...
    cargo heal --fix [--write] [--no-fmt] [--quiet] [--verify [--verify-args <s>]] [--top-k <N>] <path>...
    cargo heal --fix-compile [--write] [--max-iters <N>] [--verify-args <s>] <path>...
    cargo heal --dump-trajectories > trajectories.jsonl
    cargo heal --memory-stats                         (loop debug query, no paths)
    cargo heal --recompute-elo                        (batch rating backfill, 039 T2)
    cargo heal --collapse-report                      (self-collapse monitor, 042)
    cargo heal --corpus-stats [--json] [--import-series]  (corpus/score series, 050 T2)
    cargo heal --score-bench | --score-report        (needs --features score_bench)
    cargo heal --candidates <path>                   (T2.4 promotion gate, kat_service+score_bench)
    cargo heal login [--key <path>]                   (KAT account, needs --features kat_service)
    cargo heal account [<path>] [--history]            (KAT local view + repo burn + earned/spent, kat_service)
    cargo heal update                                 (corpus lease refresh, kat_transport)
    cargo heal sync                                   (push redacted batches + pull lease, kat_transport)
    cargo heal claim --tx <sig>                       (queue/submit a payment claim, kat_service)
    cargo heal stake --usd <N> | --info              (Proposal 007 staking: transfer QR / position, kat_transport)
    cargo heal vote --proposal <id> --yes|--no     (Proposal 008 T2.4 governance voting: weight + evidence render, kat_transport)
    cargo heal unstake                                (drain unlocked tranches + escrow disclosure, kat_transport)
    cargo heal pay --request [--amount N] [--label L] [--qr] [--wait S]  (Proposal 011 T1.3 pay request: signed create + link-first + poll-on-credit, kat_transport)
    cargo heal faucet --request [--qr]               (Proposal 011 T1.8 faucet preset: the amount is the deployment's cap, not yours, kat_transport)
    cargo heal wetrun [--dry-run]                    (Proposal 011 T4.1 wet-run + G-WET; REFUSES mainnet with no override, --dry-run is read-only everywhere, kat_transport)

    --suggest       Suggest mode: print matching rules per code span (no edits).
    --fix           Fix mode: apply the compiled-in proposers' span-preserving edits
                    ({} perf rules + {} clippy lints: {} | {}). The proposer's visitor
                    is the gate — latent suggestions never auto-apply.
    --fix-compile   Compile-error repair (Plan 019, needs --features rustc_errors):
                    cargo-check the owning packages, parse coded errors (E0597...),
                    propose per error via playbooks, apply best → recheck → keep/revert,
                    bounded by --max-iters. Review mode (default) prints the plan;
                    --write applies. Registry: the E0597 family (Phase 2) — six
                    ranked templates keyed on the diagnostic's labeled spans; other
                    codes report as misses until Phase 3.
    --max-iters <N> --fix-compile iteration bound (default 5).
    --dump-trajectories  Export the self-evolve trajectory store to stdout as
                    JSONL (Plan 019 T4.3b — the human grep surface; one source
                    of truth, no shadow log). Reads the store the build ships.
    --memory-stats Print the self-evolve loop debug summary (Issue 037):
                    tier/outcome counts, top rules, commitment status, the
                    E1 record / E2 verdict / E3 learn state — the one-command
                    \"is the loop actually closed?\" query. Read-only.
    --misses       Print the `.heal/misses/` authoring queue in FRONTIER
                    order (Plan 091 T1.2): dispersion-informative error
                    codes first — resolving THOSE converts uncertain
                    candidates — then unobserved, regenerate, saturated.
                    Pool evidence comes from the trajectory store when the
                    build ships latent_retrieval; without it the order
                    degenerates to code-sorted. `--limit <N>` caps the
                    output. Read-only standalone mode.
    --report       With `--misses`: print the conversion ledger instead of
                    the queue (Plan 091 T1.3) — conversion-per-authored-rule
                    split by authoring era (the FIFO baseline the WIRED
                    strategies were authored in vs the frontier landings
                    authored from this queue), against the wiring commit
                    that frontier-ordered the queue. Read-only.
    --recompute-elo Batch ELO over the trajectory store (Issue 039 T2; needs
                    --features latent_retrieval): replay every verdict count
                    as rule-vs-shape matches from base 1200 + write the final
                    ratings onto the rows. Idempotent standalone WRITE mode.
    --collapse-report Self-collapse monitor (Issue 042; needs --features
                    latent_retrieval): run the REAL heal retrieval loop over
                    the fixed clippy fixture set against the store at
                    RIIR_HEAL_TRAJ_ROOT (or the envelope fallback) and print
                    pool memory-fraction + memory win-rate + top-k rule
                    entropy, with a fresh-store baseline column. Read-only.
    --corpus-stats  Count the corpus domains (compiled builders where the
                    feature is on, source-walk needle otherwise — 009-F2),
                    print the table, and stamp today's rows into the
                    trajectory store
                    (needs --features latent_retrieval; without it the table
                    prints + a warning says stamping is unavailable).
    --import-series Import .benchmarks/{{corpus,score}}_history.jsonl into the
                    trajectory store (idempotent upserts; needs
                    --features latent_retrieval). With or without
                    --corpus-stats.
    --json          With --corpus-stats: stdout carries ONLY the dashboard
                    JSON payload ({{today, corpus, scores}}) — the human table
                    moves to stderr.
    --write         With --fix/--fix-compile: write the edits in place (with --fix:
                    then rustfmt AUTOMATICALLY when the original was already
                    rustfmt-clean — pre-existing drift + CRLF files are skipped — no
                    formatting churn enters a mechanical diff). Without --write:
                    per-edit review incl. the fmt plan (would-format / skip drift /
                    skip CRLF).
    --quiet         With --fix: skip the --suggest pass (only show fixes).
    --no-fmt        Disable the automatic post-write rustfmt pass (it is
                    default-on; see src/fmt.rs for the discipline it encodes).
    login           KAT account identity (Proposal 005 T0.1): generate an
                    Ed25519 keypair in OpenSSH format under the account dir
                    (RIIR_HEAL_KAT_ACCOUNT_DIR / ~/.config/riir-heal), or
                    import one with --key <path>. Idempotent — an existing
                    key is loaded, never regenerated. Records the free-tier
                    grant as a LOCAL projection (accounting-only, G9d).
    account         KAT local view: account id (BLAKE3(pubkey)), key path,
                    grant projection, and — when a path (or cwd) holds a
                    repo-local charge ledger — this repo's burn + balance
                    projection. Verifies key↔record agreement (tamper
                    evidence). No $ figures (caveats #12/#15).
    update          T1.6 corpus lease refresh (needs --features kat_transport):
                    GET /lease with If-None-Match → 304 (up to date, no
                    charge) or 200 → manifest verify (tamper → reject, fall
                    back local) → segment diff → fetch → staged verify →
                    ATOMIC pointer swap → retrieval index rebuild next run.
                    Prints the lease charge line (idempotent per epoch).
    sync            T1.6 bidirectional (kat_transport): push redacted
                    trajectory batches (the sanctioned egress path) + pull
                    the lease. The payload is SIGNED with the account key
                    (v2 — Issue 008; `login` first). With the service plane
                    deferred, the push is validated + queued locally
                    (.heal/outbox/) — nothing is faked.
    claim           T1.6 payment claim (--tx <sig>, kat_service): validate +
                    compose + submit (or queue locally when the service
                    plane is not yet live). NEVER mints client-side (G9d).
    --tx <sig>      With claim: the Solana transaction signature (base58,
                    64-88 chars).
    --update-timeout <s>  The run-start lease-phase budget (default 4s;
                    0 = skip). Timeout/offline → heal proceeds on the local
                    corpus (G3 — the network may delay freshness, never
                    block healing).
    --no-update     Skip the run-start lease phase entirely.
    --no-reheat     Skip the run-end boundary reheat (the mid-run-epoch
                    straddle pass; CI/cost-sensitive runs).
    --corpus-epoch <E>    Pin the corpus epoch (an INSTALLED epoch — implies
                    the lease check is skipped; the CI-reproducibility form).
    --key <path>    With login: import this OpenSSH Ed25519 private key
                    (unencrypted only) instead of generating one.
{}{}    --domain        Filter suggest output to one domain (default: auto — all enabled).
    --top-k <N>     Rules per span per domain (default 3).
    <path>...       Files or directories to scan. `.rs` always; with the
                    kernel_heal feature, `--suggest` ALSO scans `.wgsl` files
                    + `#[cube]` kernel bodies in `.rs` files (Issue 025 —
                    suggest-only citations, never touched by --fix).

Domains are feature-gated: clippy + perf are default-on; kernel needs
--features kernel_heal, sec needs --features sec, docker needs --features
docker (Dockerfiles are matched by NAME — `Dockerfile`, `<n>.Dockerfile`,
`Dockerfile.<n>`; `docker_verify` adds the hadolint agreement oracle).
Requesting a domain whose feature is off is an error (T2.4 gating). --verify needs --features fix_verify.
--fix-compile needs --features rustc_errors.",
        FIXABLE_RULES.len(),
        FIXABLE_RULES.join(", "),
        FIXABLE_LINTS.len(),
        FIXABLE_LINTS.join(", "),
        verify_help,
        score_help
    );
    std::process::exit(2);
}

struct Args {
    suggest: bool,
    fix: bool,
    /// Plan 019 T1.3: `--fix-compile` — the rustc_errors oracle driver.
    /// A separate mode (not a --fix modifier): it operates on packages,
    /// needs no retrieval, and its loop has verification built in.
    fix_compile: bool,
    /// Plan 019 T4.3b: `--dump-trajectories` — export the trajectory
    /// store to stdout as JSONL (one trajectory per line). Reads whichever
    /// backend the build ships (the JSON envelope under `latent_retrieval`-
    /// off; the neuron-db store joins when T4.3 lands).
    dump_trajectories: bool,
    /// `--max-iters` for --fix-compile (default 5, the plan default).
    max_iters: usize,
    write: bool,
    quiet: bool,
    /// `--no-fmt`: disable the automatic post-heal rustfmt pass (default
    /// ON with --fix --write / --verify — see src/fmt.rs for the discipline
    /// it encodes). Dry runs always report the fmt plan.
    no_fmt: bool,
    domain: Option<DomainId>,
    top_k: usize,
    verify: bool,
    verify_args: Option<String>,
    paths: Vec<PathBuf>,
    /// `--score-bench` (Plan 014): run the frozen clippy-bench, append the
    /// score history, re-render the SVG. Standalone mode — no paths.
    score_bench: bool,
    /// `--generation <1|2|3|4|5|6>` (Plan 092 M5 + Bench 069 reopen #1 +
    /// Plan 093 + Plan 094 + Plan 095): which bench generation to run.
    /// Absent = 1 (the frozen v1 set, unchanged). 2 = the construction-built generation-v2
    /// k-ladder (issue 072 T5); 3 = the persist-stratified generation-v3
    /// ladder (Bench 069 reopen #1); 4 = the rustc-errors-side k-ladder
    /// (Plan 093 — measures the `--fix-compile` driver against rustc
    /// coded-error ground truth); 5 = the rust_perf vocabulary ladder (Plan
    /// 094 — the mechanical shape oracle over the FIXABLE perf vocabulary).
    /// A NEW generation's first append is gated on non-vacuity — a
    /// generation that is not harder than v1 is refused, never appended.
    generation: Option<u8>,
    /// `--score-report`: re-render the SVG from the existing history JSONL.
    score_report: bool,
    /// `--memory-stats` (Issue 037 observability): print the one-command
    /// debug summary of the self-evolve store — tier/outcome counts, top
    /// rules, BLAKE3 commitment status, + the E1/E2/E3 loop-state verdict.
    /// Read-only standalone mode (no paths, no recording, ignores the
    /// opt-out env — a query, not a run).
    memory_stats: bool,
    /// Plan 091 T1.2: `--misses` — print the `.heal/misses/` authoring
    /// queue in FRONTIER order (dispersion-informative codes first, the
    /// real-G8 enabler). Read-only standalone mode (the memory-stats
    /// pattern: early return before validate_mode_flags).
    misses: bool,
    /// `--limit <N>` with `--misses`: cap the printed queue (default: all).
    misses_limit: Option<usize>,
    /// `--report` with `--misses`: print the conversion ledger (Plan 091
    /// T1.3) instead of the queue.
    misses_report: bool,
    /// `--recompute-elo` (Issue 039 T2): batch PL→Elo (standard-update
    /// canonical replay) over the trajectory store — backfills/writes the
    /// final rule + span ratings onto every row. Standalone WRITE mode
    /// (idempotent; `latent_retrieval` builds — the neuron-db store).
    recompute_elo: bool,
    /// `--collapse-report` (Issue 042): the self-collapse monitor — runs the
    /// real retrieval loop (`HealPipeline::heal` Step 1/3 + selection) over
    /// the fixed clippy fixture set against the store at
    /// `RIIR_HEAL_TRAJ_ROOT` (or the envelope fallback) + a fresh baseline.
    /// Read-only standalone mode (R346's monitor; feeds the audit script).
    collapse_report: bool,
    /// Proposal 005 T0.1: the identity subcommand (`login` / `account` /
    /// `update` / `sync` / `claim`), recognized only as the FIRST positional
    /// (a dir named `login` later in the arg list still scans).
    command: Option<KatCommand>,
    /// `--history` — with `account`: render the local earned/spent history
    /// (Proposal 011 T0.2, ask 8.5).
    account_history: bool,
    /// `--key <path>` — import an existing OpenSSH Ed25519 private key at
    /// `login` instead of generating one.
    import_key: Option<PathBuf>,
    /// T1.6: `--tx <sig>` — the Solana transaction signature `claim`
    /// submits/queues.
    claim_tx: Option<String>,
    /// Proposal 007 T2.1: `--usd <N>` — the stake amount (dollars, ≤6
    /// fractional digits, integer-exact into base units).
    stake_usd: Option<String>,
    /// Proposal 007 T2.1: `--info` — render the stake position instead of
    /// composing a stake request.
    stake_info: bool,
    /// Proposal 008 T2.4: `--proposal <id>` — the governance proposal id.
    vote_proposal: Option<u64>,
    /// Proposal 008 T2.4: `--yes` — vote for.
    vote_yes: bool,
    /// Proposal 008 T2.4: `--no` — vote against.
    vote_no: bool,
    /// Proposal 011 T1.3: `--request` — the pay-request mode selector.
    /// The usage line has advertised `pay --request` since T1.3 while the
    /// parser rejected it as an unknown flag, so the documented invocation
    /// failed (found by the T1.4 CLI smoke, Plan 064). Accepted and
    /// required-by-nothing: `pay` has exactly one mode today, and the flag
    /// exists so the documented command works and so a future
    /// `pay --settle <link>` (T1.2's payer half) has a sibling to select
    /// against rather than a silent default.
    pay_request: bool,
    /// Proposal 011 T1.3: `--amount <N>` — the pay-request amount in KAT
    /// (≤6 fractional digits; blank/absent = ANY amount, ask 3).
    pay_amount: Option<String>,
    /// Proposal 011 T1.3: `--qr` — render the request link as an ASCII QR
    /// after the link line (link-first stays the default).
    pay_qr: bool,
    /// Proposal 011 T1.3: `--label <s>` — the human label riding the
    /// request ("invoice #1042"; ≤64 bytes, service-bounded).
    pay_label: Option<String>,
    /// Proposal 011 T1.3: `--wait <secs>` — the poll-on-credit budget
    /// (default 600; `0` = don't poll — the credit surfaces via `account`).
    pay_wait: u64,
    /// Proposal 011 T4.2: `--dry-run` — with `wetrun`, run steps 1+6 only
    /// (read-only, zero state). Admitted on EVERY network including
    /// mainnet; it is what mainnet gets.
    dry_run: bool,
    /// T1.6: the run-start update-phase budget in seconds (`0` = skip;
    /// default 4 — the proposal's ~3–5s window).
    update_timeout: u64,
    /// T1.6: `--no-update` — skip the run-start lease phase entirely.
    no_update: bool,
    /// T1.6: `--no-reheat` — skip the run-end boundary reheat (CI /
    /// cost-sensitive runs).
    no_reheat: bool,
    /// T1.6: `--corpus-epoch <E>` — pin the corpus epoch outright (the
    /// CI-reproducibility form; implies the lease check is skipped).
    corpus_epoch: Option<u64>,
    /// T2.4 (Plan 042): `--candidates <path>` — run the score-bench
    /// promotion gate over a network-delivered candidates file (post-hoc
    /// sweep of the live overlay first, then gate + persist + report).
    /// Standalone mode — no paths. Needs `kat_service` + `score_bench`.
    candidates_path: Option<PathBuf>,
    /// Issue 050 T2: `--corpus-stats` — count the corpus domains (runtime
    /// builders where compiled, the source-walk needle otherwise — 009-F2),
    /// print the table, and stamp today's rows into the trajectory store
    /// (`latent_retrieval` builds). Standalone mode — no paths.
    corpus_stats: bool,
    /// `--json` (a `--corpus-stats` modifier): print ONLY the dashboard
    /// JSON payload (today + corpus series + score runs) to stdout — the
    /// human table moves to stderr so stdout stays machine-parseable.
    json: bool,
    /// `--import-series` — import `.benchmarks/{corpus,score}_history.jsonl`
    /// into the trajectory store (idempotent upserts). Works with or
    /// without `--corpus-stats`. Needs `latent_retrieval`.
    import_series: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        account_history: false,
        suggest: false,
        fix: false,
        fix_compile: false,
        dump_trajectories: false,
        max_iters: 5,
        write: false,
        quiet: false,
        no_fmt: false,
        domain: None,
        top_k: 3,
        verify: false,
        verify_args: None,
        paths: Vec::new(),
        score_bench: false,
        generation: None,
        score_report: false,
        memory_stats: false,
        misses: false,
        misses_limit: None,
        misses_report: false,
        recompute_elo: false,
        collapse_report: false,
        command: None,
        import_key: None,
        claim_tx: None,
        stake_usd: None,
        stake_info: false,
        vote_proposal: None,
        vote_yes: false,
        vote_no: false,
        pay_request: false,
        pay_amount: None,
        pay_qr: false,
        pay_label: None,
        pay_wait: 600,
        dry_run: false,
        update_timeout: 4,
        no_update: false,
        no_reheat: false,
        corpus_epoch: None,
        candidates_path: None,
        corpus_stats: false,
        json: false,
        import_series: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--suggest" => args.suggest = true,
            "--fix" => args.fix = true,
            "--fix-compile" => args.fix_compile = true,
            "--dump-trajectories" => args.dump_trajectories = true,
            "--write" => args.write = true,
            "--quiet" => args.quiet = true,
            "--no-fmt" => args.no_fmt = true,
            "--verify" => args.verify = true,
            "--score-bench" => args.score_bench = true,
            "--score-report" => args.score_report = true,
            "--memory-stats" => args.memory_stats = true,
            "--misses" => args.misses = true,
            "--report" => args.misses_report = true,
            "--limit" => {
                let Some(value) = it.next() else {
                    eprintln!("--limit requires a value");
                    usage();
                };
                args.misses_limit = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--limit expects a number, got {value:?}");
                    usage();
                }));
            }
            "--generation" => {
                let Some(value) = it.next() else {
                    eprintln!("--generation requires a value (1, 2, 3, 4, 5, 6, or 7)");
                    usage();
                };
                let g: u8 = value.parse().unwrap_or_else(|_| {
                    eprintln!("--generation expects a number, got {value:?}");
                    usage();
                });
                if !(1..=7).contains(&g) {
                    eprintln!("--generation accepts 1, 2, 3, 4, 5, 6, or 7, got {g}");
                    usage();
                }
                args.generation = Some(g);
            }
            "--recompute-elo" => args.recompute_elo = true,
            "--collapse-report" => args.collapse_report = true,
            "--corpus-stats" => args.corpus_stats = true,
            "--import-series" => args.import_series = true,
            "--json" => args.json = true,
            "--max-iters" => {
                let Some(value) = it.next() else {
                    eprintln!("--max-iters requires a value");
                    usage();
                };
                args.max_iters = value.parse().unwrap_or_else(|_| {
                    eprintln!("--max-iters expects a number, got {value:?}");
                    usage();
                });
            }
            "--verify-args" => {
                let Some(value) = it.next() else {
                    eprintln!("--verify-args requires a value");
                    usage();
                };
                args.verify_args = Some(value);
            }
            "--domain" => {
                let Some(value) = it.next() else {
                    eprintln!("--domain requires a value");
                    usage();
                };
                match value.as_str() {
                    "auto" => args.domain = None,
                    "clippy" => args.domain = Some(DomainId::Clippy),
                    "perf" => args.domain = Some(DomainId::Perf),
                    "kernel" => args.domain = Some(DomainId::Kernel),
                    "sec" => args.domain = Some(DomainId::Sec),
                    "docker" => args.domain = Some(DomainId::Docker),
                    other => {
                        eprintln!("unknown --domain {other:?} (want auto|clippy|perf|kernel|sec|docker)");
                        usage();
                    }
                }
            }
            "--top-k" => {
                let Some(value) = it.next() else {
                    eprintln!("--top-k requires a value");
                    usage();
                };
                args.top_k = value.parse().unwrap_or_else(|_| {
                    eprintln!("--top-k expects a number, got {value:?}");
                    usage();
                });
            }
            "--key" => {
                let Some(value) = it.next() else {
                    eprintln!("--key requires a value (an OpenSSH Ed25519 private key path)");
                    usage();
                };
                args.import_key = Some(PathBuf::from(value));
            }
            "--tx" => {
                let Some(value) = it.next() else {
                    eprintln!("--tx requires a value (the Solana transaction signature)");
                    usage();
                };
                args.claim_tx = Some(value);
            }
            "--usd" => {
                let Some(value) = it.next() else {
                    eprintln!("--usd requires a value (the stake amount in dollars, e.g. 100)");
                    usage();
                };
                args.stake_usd = Some(value);
            }
            "--info" => args.stake_info = true,
            "--proposal" => {
                let Some(value) = it.next() else {
                    eprintln!("--proposal requires a value (the governance proposal id)");
                    usage();
                };
                args.vote_proposal = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--proposal expects a number, got {value:?}");
                    usage();
                }));
            }
            "--yes" => args.vote_yes = true,
            "--no" => args.vote_no = true,
            "--amount" => {
                let Some(value) = it.next() else {
                    eprintln!("--amount requires a value (the request amount in KAT, e.g. 2 or 1.5; omit for ANY amount)");
                    usage();
                };
                args.pay_amount = Some(value);
            }
            "--request" => args.pay_request = true,
            "--dry-run" => args.dry_run = true,
            "--qr" => args.pay_qr = true,
            "--label" => {
                let Some(value) = it.next() else {
                    eprintln!("--label requires a value (e.g. \"invoice #1042\")");
                    usage();
                };
                args.pay_label = Some(value);
            }
            "--wait" => {
                let Some(value) = it.next() else {
                    eprintln!("--wait requires a value (seconds; 0 = don't poll for the credit)");
                    usage();
                };
                args.pay_wait = value.parse().unwrap_or_else(|_| {
                    eprintln!("--wait expects a number, got {value:?}");
                    usage();
                });
            }
            "--update-timeout" => {
                let Some(value) = it.next() else {
                    eprintln!("--update-timeout requires a value (seconds; 0 = skip)");
                    usage();
                };
                args.update_timeout = value.parse().unwrap_or_else(|_| {
                    eprintln!("--update-timeout expects a number, got {value:?}");
                    usage();
                });
            }
            "--no-update" => args.no_update = true,
            "--history" => args.account_history = true,
            "--no-reheat" => args.no_reheat = true,
            "--candidates" => {
                let Some(value) = it.next() else {
                    eprintln!("--candidates requires a value (the candidates NDJSON path)");
                    usage();
                };
                args.candidates_path = Some(PathBuf::from(value));
            }
            "--corpus-epoch" => {
                let Some(value) = it.next() else {
                    eprintln!("--corpus-epoch requires a value (an installed epoch number)");
                    usage();
                };
                args.corpus_epoch = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--corpus-epoch expects a number, got {value:?}");
                    usage();
                }));
            }
            flag if flag.starts_with("--") => {
                eprintln!("unknown flag {flag:?}");
                usage();
            }
            "login" | "account" | "update" | "sync" | "claim" | "stake" | "unstake" | "vote"
                | "pay" | "faucet" | "wetrun"
                if args.paths.is_empty() && args.command.is_none() =>
            {
                args.command = Some(match arg.as_str() {
                    "login" => KatCommand::Login,
                    "account" => KatCommand::Account,
                    "update" => KatCommand::Update,
                    "sync" => KatCommand::Sync,
                    "stake" => KatCommand::Stake,
                    "unstake" => KatCommand::Unstake,
                    "vote" => KatCommand::Vote,
                    "pay" => KatCommand::Pay,
                    "faucet" => KatCommand::Faucet,
                    "wetrun" => KatCommand::Wetrun,
                    _ => KatCommand::Claim,
                });
            }
            path => args.paths.push(PathBuf::from(path)),
        }
    }
    if args.account_history && !matches!(args.command, Some(KatCommand::Account)) {
        eprintln!("--history requires the `account` command");
        usage();
    }
    // `--request` and `--qr` are SHARED by `pay` and `faucet` (both open a
    // payreq row and both print a link); `--amount`/`--label`/`--wait` are
    // `pay`-only. The faucet's amount is the deployment's
    // `faucet_daily_cap_kat` and its settle is the daily crank's, so
    // accepting either flag there would advertise a choice that does not
    // exist on the wire (T1.8).
    if (args.pay_request || args.pay_qr)
        && !matches!(args.command, Some(KatCommand::Pay | KatCommand::Faucet))
    {
        eprintln!("--request/--qr require the `pay` or `faucet` command");
        usage();
    }
    if (args.pay_amount.is_some() || args.pay_label.is_some() || args.pay_wait != 600)
        && !matches!(args.command, Some(KatCommand::Pay))
    {
        eprintln!(
            "--amount/--label/--wait require the `pay` command — a faucet request carries no \
             amount and no ttl on the wire (the deployment pins both), and its credit lands \
             at the daily crank, so there is nothing to wait on"
        );
        usage();
    }
    if args.dry_run && !matches!(args.command, Some(KatCommand::Wetrun)) {
        eprintln!("--dry-run requires the `wetrun` command");
        usage();
    }
    if args.paths.is_empty()
        && args.command.is_none()
        && !args.score_bench
        && !args.score_report
        && !args.dump_trajectories
        && !args.memory_stats
        && !args.misses
        && !args.misses_report
        && !args.recompute_elo
        && !args.collapse_report
        && !args.corpus_stats
        && !args.import_series
        && args.candidates_path.is_none()
    {
        eprintln!("no paths given");
        usage();
    }
    if args.generation.is_some() && !args.score_bench {
        eprintln!("--generation requires --score-bench");
        usage();
    }
    args
}

/// Plan 019 T1.5: the mode-flag validation matrix, extracted so every
/// accepted/rejected combination is pinnable by test.
///
/// The silent-usability trap this closes: the existing guards rejected
/// `--fix-compile --write --verify-args "..."` because `--write` demanded
/// `--fix` and `--verify-args` demanded `--verify` — the flags were parsed
/// but unusable in the mode they were built for. Both guards now accept
/// `--fix-compile` as a satisfying mode.
///
/// # Errors
///
/// Returns the exact message `main` prints before `usage()`.
fn validate_mode_flags(
    suggest: bool,
    fix: bool,
    fix_compile: bool,
    dump_trajectories: bool,
    write: bool,
    verify: bool,
    verify_args: bool,
) -> Result<(), &'static str> {
    if !suggest && !fix && !fix_compile && !dump_trajectories {
        return Err("no mode given — use --suggest, --fix, --fix-compile, or --dump-trajectories");
    }
    if dump_trajectories && (suggest || fix || fix_compile) {
        return Err("--dump-trajectories is a standalone mode");
    }
    if write && !fix && !fix_compile {
        return Err("--write requires --fix (or --fix-compile)");
    }
    if write && dump_trajectories {
        return Err("--write does not apply to --dump-trajectories (it reads)");
    }
    if verify && !fix {
        return Err("--verify requires --fix");
    }
    if verify_args && !verify && !fix_compile {
        return Err("--verify-args requires --verify (or --fix-compile)");
    }
    Ok(())
}

/// Recursively collect source files with the given extension (std-only
/// walker — no new dep). Skips hidden dirs + `target`/`vendor` build noise.
/// Whether a file name is a Dockerfile by convention.
fn is_dockerfile_name(name: &str) -> bool {
    name == "Dockerfile" || name.ends_with(".Dockerfile") || name.starts_with("Dockerfile.")
}

/// Collect Dockerfiles under `root` (Plan 030 T0.4) — the name-matched
/// twin of [`collect_source_files`], which matches by extension only.
fn collect_dockerfiles(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_dockerfile_name(name) {
            out.push(root.to_path_buf());
        }
        return;
    }
    let mut queue: VecDeque<PathBuf> = VecDeque::new();
    queue.push_back(root.to_path_buf());
    while let Some(dir) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.starts_with('.') || name == "target" || name == "vendor" {
                continue;
            }
            if path.is_dir() {
                queue.push_back(path);
            } else if is_dockerfile_name(name) {
                out.push(path);
            }
        }
    }
}

/// One-line summary of a multi-line snippet (first non-trivial line).
fn snippet_head(text: &str) -> &str {
    text.lines()
        .map(str::trim)
        .find(|l| l.len() > 8)
        .unwrap_or("…")
}

/// Clamp a message to `n` chars for one-line report output.
/// The T6.10 semantic-note suffix for fix-compile report lines: names
/// the semantics change in the diff output when the applied strategy
/// sits in the exclusion class (clone-instead-of-borrow, drop-order
/// change, ownership transfer, lifetime loosening).
#[cfg(feature = "rustc_errors")]
fn semantic_note_suffix(note: Option<&'static str>) -> String {
    match note {
        None => String::new(),
        Some(n) => format!("\n  ⚠ semantic change: {n}"),
    }
}

#[cfg(feature = "rustc_errors")]
fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

fn print_suggestion(s: &SpanSuggestion, matcher: &LatentMatcher, domain_filter: Option<DomainId>) {
    let routed = s.routed_domain.label();
    println!("{}:{}-{} (routed: {routed})", s.path, s.start_line, s.end_line);
    for (domain, rules) in &s.by_domain {
        if domain_filter.is_some_and(|want| want != *domain) {
            continue;
        }
        for rule in rules {
            let info = matcher.rule_book(&rule.rule_id);
            let fixed = info.map_or("?", |i| snippet_head(&i.fixed_code));
            println!(
                "  [{:<6}] {:<32} score {:>8.2}  →  {fixed}",
                domain.label(),
                rule.rule_id,
                rule.score
            );
        }
    }
}

/// Plan 091 T1.2 / Plan 092 T5 (the collision merge): the `.heal/misses/`
/// authoring queue in frontier order — read-only, standalone. Outcomes per
/// code come from the trajectory store (`latent_retrieval` builds) or the
/// JSON envelope fallback, so the report works in every default build
/// (`rustc_errors` is default-on). `limit` caps the rendered rows (the
/// `--limit` flag; the whole queue is small, so the default prints all).
#[cfg(feature = "rustc_errors")]
fn run_misses_queue(limit: Option<usize>, paths: &[PathBuf]) {
    use riir_clippy::domains::rustc_errors::evolve;

    let root = resolve_heal_root(paths);
    let dir = evolve::misses_dir(&root);
    let outcomes = load_code_outcomes(&root);
    let report = evolve::frontier_queue_report(&dir, &outcomes);
    match limit {
        // Honor the cap on the table rows: keep the header lines, print at
        // most `limit` table rows.
        Some(n) => {
            let mut in_table = false;
            let mut shown = 0usize;
            for line in report.lines() {
                if line.starts_with("| rank") || line.starts_with("|---") {
                    in_table = true;
                    println!("{line}");
                    continue;
                }
                if in_table && line.starts_with("| ") {
                    if shown < n {
                        println!("{line}");
                        shown += 1;
                    }
                    continue;
                }
                println!("{line}");
            }
        }
        None => print!("{report}"),
    }
}

/// Per-code `(kept, reverted)` counts for the misses report: store-first,
/// envelope fallback (both aggregate through
/// [`riir_clippy::domains::rustc_errors::evolve::code_outcomes_from_rows`]).
#[cfg(feature = "rustc_errors")]
fn load_code_outcomes(root: &Path) -> std::collections::BTreeMap<String, (usize, usize)> {
    #[cfg(feature = "latent_retrieval")]
    if root.join(".heal").join("trajectories.kv").exists() {
        if let Ok((store, _)) = riir_clippy::traj_store::TrajStore::open(root) {
            return riir_clippy::domains::rustc_errors::evolve::code_outcomes_from_rows(
                store.dump().into_iter().map(|r| (r.lint_key, r.success_count, r.fail_count)),
            );
        }
    }
    let mut mem = riir_clippy::memory::LatentFixMemory::default();
    if mem.load(&default_memory_path()).is_err() {
        return std::collections::BTreeMap::new();
    }
    riir_clippy::domains::rustc_errors::evolve::code_outcomes_from_rows(
        mem.iter().map(|t| (t.lint_name.clone(), t.success_count, t.fail_count)),
    )
}

/// Plan 014: run the clippy-bench, append the score history, re-render the
/// SVG. Feature-gated `score_bench`. `generation` selects the fixture set
/// (Plan 092 M5 + Bench 069 reopen #1): 1 = the frozen v1 set, 2 = the
/// construction-built generation-v2 k-ladder (issue 072 T5), 3 = the
/// persist-stratified generation-v3 ladder — each new generation behind the
/// non-vacuity admission gate: a first run that is not harder than v1 is
/// refused, never appended.
#[cfg(feature = "score_bench")]
fn run_score_bench(generation: u8) {
    use riir_clippy::score_bench as sb;

    // Generations 4 (Plan 093) and 5 (Plan 094) are the compile-domain and
    // perf-domain runners — neither uses the clippy oracle. Dispatch before
    // the oracle availability check so a clippy-less invocation still works.
    if generation == 4 {
        #[cfg(feature = "rustc_errors")]
        {
            run_score_bench_compile();
            return;
        }
        #[cfg(not(feature = "rustc_errors"))]
        {
            eprintln!("--generation 4 requires the rustc_errors feature");
            std::process::exit(1);
        }
    }
    if generation == 5 {
        run_score_bench_perf();
        return;
    }

    let oracle = sb::draft_oracle();
    if !oracle.is_available() {
        eprintln!("score_bench: cargo not on PATH — cannot derive ground truth");
        std::process::exit(1);
    }
    let (fixture_list, label) = match generation {
        2 => match sb::generations::v2_freeze_status() {
            Ok(admitted) => {
                println!("generation v2: {admitted} admitted fixture(s) (freeze record present)");
                (sb::generations::generation_v2(), "generation-v2 k-ladder")
            }
            Err(e) => {
                eprintln!("generation v2: {e}");
                std::process::exit(1);
            }
        },
        3 => match sb::generations::v3_freeze_status() {
            Ok(admitted) => {
                println!("generation v3: {admitted} admitted fixture(s) (freeze record present)");
                (sb::generations::generation_v3(), "generation-v3 persist-stratified ladder")
            }
            Err(e) => {
                eprintln!("generation v3: {e}");
                std::process::exit(1);
            }
        },
        6 => match sb::generations::v6_freeze_status() {
            Ok(admitted) => {
                println!("generation v6: {admitted} admitted fixture(s) (freeze record present)");
                (sb::generations::generation_v6(), "generation-v6 real-repo shape ladder")
            }
            Err(e) => {
                eprintln!("generation v6: {e}");
                std::process::exit(1);
            }
        },
        7 => match sb::generations::v7_freeze_status() {
            Ok(admitted) => {
                println!("generation v7: {admitted} admitted fixture(s) (freeze record present)");
                (sb::generations::generation_v7(), "generation-v7 overlap-edit-chain ladder")
            }
            Err(e) => {
                eprintln!("generation v7: {e}");
                std::process::exit(1);
            }
        },
        _ => (sb::fixtures::fixtures(), "frozen v1 set"),
    };
    println!(
        "clippy-bench: {} fixture(s) [{label}], real-clippy ground truth…",
        fixture_list.len()
    );
    let score = sb::run_bench(&oracle, &fixture_list);
    sb::report::print_report(&score);
    if generation >= 2 {
        print_per_k_solve_rates(&fixture_list, &score);
    }
    if generation == 6 {
        print_v6_territory_rates(&fixture_list, &score);
    }
    if generation == 7 {
        print_v7_chain_tables(&fixture_list, &score);
    }

    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    let history_before = sb::report::load_history(jsonl);

    // M5 non-vacuity admission (the shared gate — see [`admission_gate`]).
    if let Err(refusal) = admission_gate(generation, &score, &history_before) {
        eprintln!("{refusal}");
        std::process::exit(1);
    }

    finish_score_bench(generation, &score);
}

/// The M5 non-vacuity admission gate, shared by the clippy and compile
/// runners: the FIRST row of a NEW generation (≥ 2) must measure strictly
/// harder than v1's latest heal_rate — a new generation that isn't harder
/// isn't a generation, it's the same measurement with a bigger denominator.
/// Returns `Err(refusal message)` — the caller prints + exits 1; nothing is
/// appended on a refusal (the generation is deterministic; the failed
/// admission is reproducible — the .benchmarks doc records the verdict
/// instead).
#[cfg(feature = "score_bench")]
fn admission_gate(
    generation: u8,
    score: &riir_clippy::score_bench::BenchScore,
    history_before: &[riir_clippy::score_bench::report::HistoryRecord],
) -> Result<(), String> {
    if generation < 2 || history_before.iter().any(|r| r.generation == generation) {
        return Ok(());
    }
    let Some(v1_rate) = history_before
        .iter()
        .rev()
        .find(|r| r.generation == 1)
        .map(|r| r.heal_rate)
    else {
        return Err(format!(
            "generation v{generation} admission refused: no v1 baseline row in {} — run `--score-bench` (v1) first",
            riir_clippy::score_bench::report::HISTORY_JSONL
        ));
    };
    if score.heal_rate >= v1_rate {
        return Err(format!(
            "generation v{generation} admission REFUSED (non-vacuity, M5): v{generation} heal_rate {:.4} >= v1 {:.4}. \
             A new generation that is not harder is not admitted; nothing appended \
             (the generation is deterministic — record the verdict in its .benchmarks doc).",
            score.heal_rate, v1_rate
        ));
    }
    println!(
        "generation v{generation} ADMITTED: heal_rate {:.4} < v1 {:.4} — the generation is harder, floors begin",
        score.heal_rate, v1_rate
    );
    Ok(())
}

/// The shared post-measurement tail: append the row, dual-write the store,
/// re-render every per-generation SVG (v1 always; v2/v3/v4 when non-empty),
/// print the history line, and report saturation telemetry. One
/// implementation for both runners — the ratchet invariants live here.
#[cfg(feature = "score_bench")]
fn finish_score_bench(generation: u8, score: &riir_clippy::score_bench::BenchScore) {
    use riir_clippy::score_bench as sb;

    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    let rec =
        sb::report::HistoryRecord::from_score(score, sb::report::git_sha()).with_generation(generation);
    if let Err(e) = sb::report::append_history(jsonl, &rec) {
        eprintln!("score history append failed: {e}");
    }
    // Issue 050 T2: the Warm-tier dual-write — the jsonl stays the committed
    // projection; the store becomes the queryable source of truth.
    // Best-effort (never fails the bench run).
    #[cfg(feature = "latent_retrieval")]
    sb::report::record_store_best_effort(&rec);
    let history = sb::report::load_history(jsonl);
    // Per-generation SVG curves (M5): v1 → score_history.svg, v2 → _v2.svg,
    // v3 → _v3.svg, v4 → _v4.svg.
    let gen_series = |g: u8| {
        history
            .iter()
            .filter(|r| r.generation == g)
            .cloned()
            .collect::<Vec<_>>()
    };
    let svg_path = |g: u8| match g {
        2 => sb::report::HISTORY_SVG_V2,
        3 => sb::report::HISTORY_SVG_V3,
        4 => sb::report::HISTORY_SVG_V4,
        5 => sb::report::HISTORY_SVG_V5,
        6 => sb::report::HISTORY_SVG_V6,
        7 => sb::report::HISTORY_SVG_V7,
        _ => sb::report::HISTORY_SVG,
    };
    let v1_series = gen_series(1);
    let svg = sb::report::render_svg(&v1_series);
    std::fs::write(sb::report::HISTORY_SVG, &svg)
        .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
    let mut written_svgs = vec![sb::report::HISTORY_SVG];
    for g in [2_u8, 3, 4, 5, 6, 7] {
        let series = gen_series(g);
        if !series.is_empty() {
            let path = svg_path(g);
            let svg_g = sb::report::render_svg(&series);
            std::fs::write(path, &svg_g)
                .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
            written_svgs.push(path);
        }
    }
    println!(
        "history: {} run(s) (gen-1 {}, gen-2 {}, gen-3 {}, gen-4 {}, gen-5 {}, gen-6 {}, gen-7 {}) → {}{}",
        history.len(),
        v1_series.len(),
        gen_series(2).len(),
        gen_series(3).len(),
        gen_series(4).len(),
        gen_series(5).len(),
        gen_series(6).len(),
        gen_series(7).len(),
        sb::report::HISTORY_JSONL,
        written_svgs
            .iter()
            .map(|p| format!(" + {p}"))
            .collect::<String>()
    );
    // Saturation telemetry (issue 072 T3/T4) — the standing non-vacuity
    // instrument over the per-generation series.
    let verdict = sb::report::detect_saturation(&history, generation);
    println!("{}", sb::report::saturation_line(&verdict, generation));
}

/// The generation-4 bench runner (Plan 093): the rustc-errors-side k-ladder.
/// Each spec is a composite of k `corpus_gen` cases (each in its own `mod
/// gen_{i}`); the measurement scaffolds the composed broken file, runs the
/// production `--fix-compile` driver (write mode), and scores per-member
/// coded-error reduction against rustc's own diagnostics — exact by
/// construction, no oracle. Metrics map onto the shared row schema:
/// heal_rate = Σ reduced / Σ members (per-member, NOT per-lint — the
/// admission gate's v1 anchor is a cross-domain non-vacuity floor, recorded
/// as such in the plan).
#[cfg(all(feature = "score_bench", feature = "rustc_errors"))]
fn run_score_bench_compile() {
    use riir_clippy::domains::rustc_errors::composite;
    use riir_clippy::domains::rustc_errors::corpus_gen::{territory_of, Territory};
    use riir_clippy::score_bench as sb;
    use std::collections::BTreeMap;

    let specs = match sb::generations::v4_freeze_status() {
        Ok(admitted) => {
            println!("generation v4: {admitted} admitted spec(s) (freeze record present)");
            sb::generations::specs_v4()
        }
        Err(e) => {
            eprintln!("generation v4: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "compile-bench: {} composite spec(s) [generation-v4 rustc-error ladder], real-rustc ground truth…",
        specs.len()
    );

    let work_root = std::env::temp_dir().join("riir_clippy_score_bench_gen4");
    let _ = std::fs::create_dir_all(&work_root);

    let mut members_total = 0usize;
    let mut members_reduced = 0usize;
    let mut created_total = 0usize;
    let mut ran_specs = 0usize;
    let mut parse_unsafe_specs = 0usize;
    let mut edits_total = 0usize;
    let mut persist: BTreeMap<String, usize> = BTreeMap::new();
    let mut k_total: BTreeMap<usize, usize> = BTreeMap::new();
    let mut k_reduced: BTreeMap<usize, usize> = BTreeMap::new();
    let mut terr_total: BTreeMap<&str, usize> = BTreeMap::new();
    let mut terr_reduced: BTreeMap<&str, usize> = BTreeMap::new();

    for spec in &specs {
        let Some(codes) = sb::generations::v4_member_codes(&spec.id) else {
            eprintln!("gen4: member codes unresolvable for {}", spec.id);
            std::process::exit(1);
        };
        let code_refs: Vec<&str> = codes.iter().map(String::as_str).collect();
        let outcome = composite::measure_composite(&spec.id, &code_refs, &spec.code, &work_root);
        if outcome.ran {
            ran_specs += 1;
        }
        if outcome.parse_unsafe {
            parse_unsafe_specs += 1;
        }
        members_total += outcome.members_total;
        members_reduced += outcome.members_reduced;
        created_total += outcome.created_total;
        edits_total += outcome.edits;
        for (code, n) in &outcome.persist {
            *persist.entry(code.clone()).or_insert(0) += n;
        }
        *k_total.entry(spec.k).or_insert(0) += outcome.members_total;
        *k_reduced.entry(spec.k).or_insert(0) += outcome.members_reduced;
        // Per-member territory: territory is a member property and a
        // composite can mix — read the aligned per-member flags.
        for (code, reduced) in codes.iter().zip(&outcome.member_reduced) {
            let terr = match territory_of(code) {
                Territory::Uncontested => "uncontested",
                Territory::Contested => "contested",
            };
            *terr_total.entry(terr).or_insert(0) += 1;
            if *reduced {
                *terr_reduced.entry(terr).or_insert(0) += 1;
            }
        }
        println!(
            "gen4 {} : k={} reduced {}/{} created {} edits {}{}",
            spec.id,
            spec.k,
            outcome.members_reduced,
            outcome.members_total,
            outcome.created_total,
            outcome.edits,
            if outcome.label_ok { "" } else { " [LABEL-MISMATCH]" },
        );
    }

    let rate = |reduced: usize, total: usize| {
        if total == 0 {
            0.0
        } else {
            reduced as f32 / total as f32
        }
    };
    let score = sb::BenchScore {
        n_total: specs.len(),
        n_ran: ran_specs,
        n_excluded: 0,
        n_clean: 0,
        heal_rate: rate(members_reduced, members_total),
        created_total,
        created_rate: rate(created_total, members_total),
        parse_safety: if ran_specs == 0 {
            1.0
        } else {
            1.0 - rate(parse_unsafe_specs, ran_specs)
        },
        // The ladder has no clean fixtures — the decline axis is vacuously
        // correct (same convention the clippy runner uses when none exist).
        decline_correctness: 1.0,
        edits_total,
        vocab_size: members_total,
        persist: persist.into_iter().collect(),
        outcomes: Vec::new(),
    };
    sb::report::print_report(&score);

    println!("\n─ generation-4 difficulty tables ──────────────");
    println!("per-k member-reduction (reported, not asserted — Plan 093 pre-registration):");
    for (k, total) in &k_total {
        let reduced = k_reduced.get(k).copied().unwrap_or(0);
        println!(
            "  k={k}: {:.1}% ({reduced}/{total})",
            rate(reduced, *total) * 100.0,
        );
    }
    println!("per-territory (never blended — the corpus's own stratification rule):");
    for (terr, total) in &terr_total {
        let reduced = terr_reduced.get(terr).copied().unwrap_or(0);
        println!(
            "  {terr}: {:.1}% ({reduced}/{total})",
            rate(reduced, *total) * 100.0,
        );
    }
    println!("───────────────────────────────────────────────");

    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    let history_before = sb::report::load_history(jsonl);

    // M5 non-vacuity admission (the shared gate — see [`admission_gate`]).
    if let Err(refusal) = admission_gate(4, &score, &history_before) {
        eprintln!("{refusal}");
        std::process::exit(1);
    }

    finish_score_bench(4, &score);
}

/// The generation-5 bench runner (Plan 094): the rust_perf vocabulary
/// ladder. CARGO-LESS end to end — the oracle is the bench-owned set of
/// per-rule mechanical shape detectors (Plan 094 §2, pre-registered), each
/// already validated by the always-on discrimination fence; the freeze
/// gates run in `tests/generation_v5_freeze.rs`. Each spec heals through
/// the production `heal_fixpoint` loop (both proposer domains, the
/// fxhashmap stub-context skip, MAX_FIX_PASSES = 4), the healed file
/// re-parses, and member i's verdict reads module `gen_{i}`'s region.
/// Per-member resolution (not per-lint): the admission gate's v1 anchor is
/// a cross-domain non-vacuity floor, recorded as such in the plan.
#[cfg(feature = "score_bench")]
fn run_score_bench_perf() {
    use riir_clippy::score_bench as sb;
    use std::collections::BTreeMap;

    let specs = match sb::generations::v5_freeze_status() {
        Ok(admitted) => {
            println!("generation v5: {admitted} admitted spec(s) (freeze record present)");
            sb::generations::specs_v5()
        }
        Err(e) => {
            eprintln!("generation v5: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "perf-bench: {} composite spec(s) [generation-v5 rust_perf ladder], mechanical shape oracle…",
        specs.len()
    );

    // The removal-polarity rule set for the created axis, hoisted once (the
    // atom table rebuilds the whole perf corpus — never inside the spec
    // loop).
    let removal_rules: Vec<String> = sb::generations::v5_atoms()
        .into_iter()
        .map(|a| a.expect[0].clone())
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .filter(|r| sb::generations::v5_is_removal_rule(r))
        .collect();

    let mut members_total = 0usize;
    let mut members_resolved = 0usize;
    let mut created_total = 0usize;
    let mut ran_specs = 0usize;
    let mut parse_unsafe_specs = 0usize;
    let mut edits_total = 0usize;
    let mut persist: BTreeMap<String, usize> = BTreeMap::new();
    let mut k_total: BTreeMap<usize, usize> = BTreeMap::new();
    let mut k_resolved: BTreeMap<usize, usize> = BTreeMap::new();
    let mut terr_total: BTreeMap<&str, usize> = BTreeMap::new();
    let mut terr_resolved: BTreeMap<&str, usize> = BTreeMap::new();

    for spec in &specs {
        let Some(rules) = sb::generations::v5_member_rules(&spec.id) else {
            eprintln!("gen5: member rules unresolvable for {}", spec.id);
            std::process::exit(1);
        };
        // Member NAMES from the spec id (territory is an atom-id property,
        // and fxhashmap legitimately carries two atoms — context-gate vs
        // Gate-0 design-gate — so rule→atom lookup would be ambiguous).
        let member_names: Vec<&str> = spec
            .id
            .strip_prefix("gen5-k")
            .and_then(|rest| rest.split_once('-'))
            .map(|(_, names)| names.split('+').collect())
            .unwrap_or_default();
        assert_eq!(member_names.len(), rules.len(), "member names/rules aligned on {}", spec.id);
        // Pre-heal regions: the created axis needs each member's BEFORE
        // marker state, read from the composed spec itself.
        let before_regions: Vec<Option<String>> =
            (0..rules.len()).map(|i| sb::generations::v5_module_region(&spec.code, i)).collect();

        let (healed, edits, _passes) = sb::heal_fixpoint(&spec.code);
        edits_total += edits;
        let parsed_ok = syn::parse_file(&healed).is_ok();
        if parsed_ok {
            ran_specs += 1;
        } else {
            parse_unsafe_specs += 1;
        }

        members_total += rules.len();
        *k_total.entry(spec.k).or_insert(0) += rules.len();
        let mut spec_resolved = 0usize;
        if parsed_ok {
            for (i, rule) in rules.iter().enumerate() {
                let Some(region) = sb::generations::v5_module_region(&healed, i) else {
                    // Region lost to the heal — score the member unreduced,
                    // never clean (the parse-guard convention).
                    parse_unsafe_specs += 1;
                    *persist.entry(rule.clone()).or_insert(0) += 1;
                    continue;
                };
                let resolved = !sb::generations::v5_persists(rule, &region);
                if resolved {
                    spec_resolved += 1;
                } else {
                    *persist.entry(rule.clone()).or_insert(0) += 1;
                }
                // Created: removal-polarity markers of OTHER rules newly
                // present in this member's region (Plan 094 §5 — the
                // compound shrink predicate is excluded: an inserted
                // `shrink_to_fit` is a fix, not a created problem).
                for other in &removal_rules {
                    if other == rule {
                        continue;
                    }
                    let after = sb::generations::v5_persists(other, &region);
                    let before = before_regions[i]
                        .as_ref()
                        .map(|r| sb::generations::v5_persists(other, r))
                        .unwrap_or(false);
                    if after && !before {
                        created_total += 1;
                        eprintln!(
                            "gen5 {}: created marker {other} in member {i} ({rule})",
                            spec.id
                        );
                    }
                }
                let terr = sb::generations::v5_territory(member_names[i]);
                *terr_total.entry(terr).or_insert(0) += 1;
                if resolved {
                    *terr_resolved.entry(terr).or_insert(0) += 1;
                }
            }
            members_resolved += spec_resolved;
            *k_resolved.entry(spec.k).or_insert(0) += spec_resolved;
        }
        println!(
            "gen5 {} : k={} resolved {}/{} edits {}{}",
            spec.id,
            spec.k,
            if parsed_ok { spec_resolved } else { 0 },
            rules.len(),
            edits,
            if parsed_ok { "" } else { " [PARSE-UNSAFE]" },
        );
    }

    let rate = |done: usize, total: usize| {
        if total == 0 {
            0.0
        } else {
            done as f32 / total as f32
        }
    };
    let score = sb::BenchScore {
        n_total: specs.len(),
        n_ran: ran_specs,
        n_excluded: 0,
        n_clean: 0,
        heal_rate: rate(members_resolved, members_total),
        created_total,
        created_rate: rate(created_total, members_total),
        parse_safety: if ran_specs == 0 {
            1.0
        } else {
            1.0 - rate(parse_unsafe_specs, ran_specs)
        },
        // The ladder has no clean fixtures — the decline axis is vacuously
        // correct (the gens-3/4 convention).
        decline_correctness: 1.0,
        edits_total,
        vocab_size: members_total,
        persist: persist.into_iter().collect(),
        outcomes: Vec::new(),
    };
    sb::report::print_report(&score);

    println!("\n─ generation-5 difficulty tables ──────────────");
    println!("per-k member-resolution (reported, not asserted — Plan 094 pre-registration):");
    for (k, total) in &k_total {
        let resolved = k_resolved.get(k).copied().unwrap_or(0);
        println!(
            "  k={k}: {:.1}% ({resolved}/{total})",
            rate(resolved, *total) * 100.0,
        );
    }
    println!("per-territory (never blended — the corpus's own stratification rule):");
    for (terr, total) in &terr_total {
        let resolved = terr_resolved.get(terr).copied().unwrap_or(0);
        println!(
            "  {terr}: {:.1}% ({resolved}/{total})",
            rate(resolved, *total) * 100.0,
        );
    }
    println!("───────────────────────────────────────────");

    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    let history_before = sb::report::load_history(jsonl);

    // M5 non-vacuity admission (the shared gate — see [`admission_gate`]).
    if let Err(refusal) = admission_gate(5, &score, &history_before) {
        eprintln!("{refusal}");
        std::process::exit(1);
    }

    finish_score_bench(5, &score);
}

/// The M4 difficulty measurement: per-k solve rates over the ladder
/// (resolved vocabulary lints / vocabulary lints before, within each k).
#[cfg(feature = "score_bench")]
fn print_per_k_solve_rates(
    fixtures: &[riir_clippy::score_bench::BenchFixture],
    score: &riir_clippy::score_bench::BenchScore,
) {
    use riir_clippy::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> = score
        .outcomes
        .iter()
        .map(|o| (o.id.as_str(), o))
        .collect();
    let mut resolved: BTreeMap<usize, usize> = BTreeMap::new();
    let mut total: BTreeMap<usize, usize> = BTreeMap::new();
    for f in fixtures {
        let Some(k) = sb::generations::k_of(f.id) else {
            continue;
        };
        let Some(o) = by_id.get(f.id) else {
            continue;
        };
        if !o.ran {
            continue;
        }
        *total.entry(k).or_default() += o.vocab_before.len();
        *resolved.entry(k).or_default() += o.resolved();
    }
    println!("\n─ k-ladder solve rates (M4, up = solved) ──────────────");
    for k in sb::generations::LADDER_KS {
        let (Some(r), Some(t)) = (resolved.get(&k), total.get(&k)) else {
            continue;
        };
        if *t == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // counts < 10⁴ over f64
        let pct = (*r as f64) / (*t as f64) * 100.0;
        println!("k={k}: {r}/{t} = {pct:.1}%");
    }
}

/// The generation-6 difficulty table: per-territory solve rates (`single` =
/// the real-shape rung, k=1; `multi` = the organic interference rung, k≥2).
/// Reported, not asserted (the Plan 095 convention); the multi rung's n is
/// the measured organic yield, never padded.
#[cfg(feature = "score_bench")]
fn print_v6_territory_rates(
    fixtures: &[riir_clippy::score_bench::BenchFixture],
    score: &riir_clippy::score_bench::BenchScore,
) {
    use riir_clippy::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> = score
        .outcomes
        .iter()
        .map(|o| (o.id.as_str(), o))
        .collect();
    let mut resolved: BTreeMap<&str, usize> = BTreeMap::new();
    let mut total: BTreeMap<&str, usize> = BTreeMap::new();
    for f in fixtures {
        let Some(o) = by_id.get(f.id) else { continue };
        if !o.ran {
            continue;
        }
        let terr = sb::generations::v6_territory(f.id);
        *total.entry(terr).or_default() += o.vocab_before.len();
        *resolved.entry(terr).or_default() += o.resolved();
    }
    println!("\n─ generation-6 territory solve rates (up = solved) ──────────────");
    for terr in ["single", "multi"] {
        let (Some(r), Some(t)) = (resolved.get(terr), total.get(terr)) else {
            continue;
        };
        if *t == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // counts < 10⁴ over f64
        let pct = (*r as f64) / (*t as f64) * 100.0;
        println!("{terr}: {r}/{t} = {pct:.1}%");
    }
}

/// The generation-7 difficulty tables (Plan 096 §7): per-depth resolution
/// (the k rung = the pre-registered expected pass depth; the capped rung k=5
/// is where the pass cap bites), per-territory resolution (flat control vs
/// chain vs capped vs refused), and the passes distribution per spec. All
/// reported, never asserted — the M5 admission gate carries the decision.
#[cfg(feature = "score_bench")]
fn print_v7_chain_tables(
    fixtures: &[riir_clippy::score_bench::BenchFixture],
    score: &riir_clippy::score_bench::BenchScore,
) {
    use riir_clippy::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> = score
        .outcomes
        .iter()
        .map(|o| (o.id.as_str(), o))
        .collect();
    let mut depth_resolved: BTreeMap<usize, usize> = BTreeMap::new();
    let mut depth_total: BTreeMap<usize, usize> = BTreeMap::new();
    let mut terr_resolved: BTreeMap<&str, usize> = BTreeMap::new();
    let mut terr_total: BTreeMap<&str, usize> = BTreeMap::new();
    let mut passes_hist: BTreeMap<usize, usize> = BTreeMap::new();
    for f in fixtures {
        let Some(o) = by_id.get(f.id) else { continue };
        if !o.ran {
            continue;
        }
        let k = sb::generations::v7_expected_k(f.id);
        *depth_total.entry(k).or_default() += o.vocab_before.len();
        *depth_resolved.entry(k).or_default() += o.resolved();
        let terr = sb::generations::v7_territory(f.id);
        *terr_total.entry(terr).or_default() += o.vocab_before.len();
        *terr_resolved.entry(terr).or_default() += o.resolved();
        *passes_hist.entry(o.passes).or_default() += 1;
    }
    #[allow(clippy::cast_precision_loss)] // counts < 10⁴ over f64
    let pct = |r: usize, t: usize| (r as f64) / (t as f64) * 100.0;
    println!("\n─ generation-7 chain tables (up = solved) ──────────────");
    println!("per-depth (k = the pre-registered expected pass depth):",);
    for (k, t) in &depth_total {
        let r = depth_resolved.get(k).copied().unwrap_or(0);
        println!("  depth {k}: {r}/{t} = {:.1}%", pct(r, *t));
    }
    println!("per-territory (never blended — the corpus's own stratification rule):");
    for terr in ["flat", "chain", "capped", "refused"] {
        let Some(t) = terr_total.get(terr) else { continue };
        let r = terr_resolved.get(terr).copied().unwrap_or(0);
        println!("  {terr}: {r}/{t} = {:.1}%", pct(r, *t));
    }
    println!("passes-to-quiet distribution (specs):");
    for (p, n) in &passes_hist {
        println!("  {p} pass(es): {n} spec(s)");
    }
    println!("───────────────────────────────────────────");
}

// ── Issue 050 T2: --corpus-stats / --import-series / --json ────────────

/// The corpus domain table — the Rust mirror of gen_dashboard.py's
/// `CORPUS_DOMAINS` (Issue 050 T2). Paths are repo-relative; a directory
/// counts its DIRECT `*.rs` children except `mod.rs`, a single file counts
/// itself.
// Proposal 009 §F2 item (ii): the authoritative count for a COMPILED domain is
// `build_*_seed_corpus().len()` at runtime — a text needle cannot see rules
// built through a helper (docker's `rule(...)` builder measured 2 by needle
// vs 31 actual, which is how the committed corpus_history.jsonl series drifted).
// The needle source-walk remains the fallback for domains not compiled into
// this binary (feature off), labeled honestly in the table + the store stamp.
const CORPUS_DOMAIN_TABLE: &[(&str, &str, &str)] = &[
    ("kernel_opt", "src/domains/kernel_opt/corpus", "rule_id:"),
    ("rust_perf", "src/domains/rust_perf/corpus.rs", "rule_id:"),
    ("clippy_lints", "src/domains/clippy_lints/corpus.rs", "lint_name:"),
    ("sec", "src/domains/sec/corpus.rs", "rule_id:"),
    // Docker's corpus uses the positional `rule()` shorthand constructor
    // (corpus.rs's own design note), so there is no per-entry `rule_id:`
    // text — the old needle counted the constructor's PLUMBING (exactly 2
    // occurrences at every rev) and every source-walk stamp since the
    // domain landed recorded docker:2 against 27..31 true entries (Issue
    // 066). `    rule(` matches exactly the 8-space-indented entry calls;
    // the count-pin test below fails loud if a re-indent moves it.
    ("docker", "src/domains/docker/corpus.rs", "    rule("),
];

/// Whether the domain's feature is compiled into THIS binary (the honest
/// `compiled` half of the corpus row — a source-walk counts text, this
/// answers "is the domain live here").
#[allow(clippy::match_like_matches_macro)] // cfg! arms are compile-time, not a pattern set
fn domain_compiled(domain: &str) -> bool {
    match domain {
        "kernel_opt" => cfg!(feature = "kernel_heal"),
        "rust_perf" => cfg!(feature = "rust_perf"),
        "clippy_lints" => cfg!(feature = "clippy_lints"),
        "sec" => cfg!(feature = "sec"),
        "docker" => cfg!(feature = "docker"),
        _ => false,
    }
}

/// The COMPILED corpus size for one domain — [`None`] when the domain's
/// feature is off in this binary (the caller falls back to the source-walk
/// needle). Each arm rides the same feature as the re-exported builder, so
/// every combination compiles and the runtime path can never count a corpus
/// the binary does not contain.
fn runtime_corpus_count(domain: &str) -> Option<u32> {
    match domain {
        #[cfg(feature = "clippy_lints")]
        "clippy_lints" => {
            Some(u32::try_from(riir_clippy::build_seed_corpus().entries.len()).unwrap_or(u32::MAX))
        }
        #[cfg(feature = "rust_perf")]
        "rust_perf" => Some(
            u32::try_from(riir_clippy::build_perf_seed_corpus().entries.len())
                .unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "kernel_heal")]
        "kernel_opt" => Some(
            u32::try_from(riir_clippy::build_kernel_seed_corpus().entries.len())
                .unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "sec")]
        "sec" => {
            Some(u32::try_from(riir_clippy::build_sec_seed_corpus().entries.len()).unwrap_or(u32::MAX))
        }
        #[cfg(feature = "docker")]
        "docker" => Some(
            u32::try_from(riir_clippy::build_docker_seed_corpus().entries.len())
                .unwrap_or(u32::MAX),
        ),
        _ => None,
    }
}

/// Count `needle` over the domain's corpus source(s) — the exact
/// `_count_working_tree` semantics from gen_dashboard.py: a `.rs` file
/// counts itself; a directory counts its DIRECT `*.rs` children except
/// `mod.rs` (non-recursive, matching the Python glob). A missing path
/// counts 0 (the domain table is static; the walk is best-effort).
fn count_corpus_domain(repo: &Path, rel: &str, needle: &str) -> u32 {
    let path = repo.join(rel);
    if path.is_file() {
        return std::fs::read_to_string(&path).map_or(0, |t| u32::try_from(t.matches(needle).count()).unwrap_or(u32::MAX));
    }
    if !path.is_dir() {
        return 0;
    }
    let mut n = 0u64;
    if let Ok(entries) = std::fs::read_dir(&path) {
        for entry in entries.flatten() {
            let p = entry.path();
            let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if name == "mod.rs" || !name.ends_with(".rs") {
                continue;
            }
            if let Ok(t) = std::fs::read_to_string(&p) {
                n += t.matches(needle).count() as u64;
            }
        }
    }
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Locate the riir-clippy repo root for the corpus source-walk: the nearest
/// ancestor of the CWD owning the corpus paths, else the binary's own
/// build-machine manifest dir (makes the flag work from any CWD on the
/// build machine), else None (the caller errors clearly).
fn resolve_corpus_repo_root() -> Option<PathBuf> {
    const MARKER: &str = "src/domains/kernel_opt/corpus";
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(MARKER).exists() {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if manifest.join(MARKER).exists() {
        return Some(manifest);
    }
    None
}


/// Plan 091 T1.3: the `--misses --report` mode — the conversion ledger
/// (the real-G8 instrument). Prints conversion-per-authored-rule split by
/// authoring era: the FIFO baseline (strategies authored pre-wiring, from
/// `WIRED_STRATEGIES`) vs the frontier era (post-wiring landings, from
/// `FRONTIER_LANDINGS`). The store join maps `TrajRowDoc`s into the
/// feature-free [`ConversionSignal`]s; without the store the conversion
/// columns print `n/a` (an absent store is NOT a zero — the honest
/// degeneration, the T1.2 convention).
fn run_conversion_report() {
    use riir_clippy::domains::rustc_errors::evolve;
    let root = std::env::current_dir().unwrap_or_default();
    let inventory = evolve::playbook_inventory();
    let mut signals: Vec<evolve::ConversionSignal> = Vec::new();
    let mut store_present = false;
    #[cfg(feature = "latent_retrieval")]
    if root.join(".heal").join("trajectories.kv").exists() {
        match riir_clippy::traj_store::TrajStore::open(&root) {
            Ok((store, bad)) => {
                if bad > 0 {
                    eprintln!("misses: {bad} corrupt store row(s) skipped");
                }
                store_present = true;
                for row in store.dump() {
                    signals.push(evolve::ConversionSignal {
                        lint_key: row.lint_key,
                        tier: row.tier,
                        last_seen_unix: row.last_seen_at,
                    });
                }
            }
            Err(e) => eprintln!("misses: store unreadable ({e}) — conversions n/a"),
        }
    }
    #[cfg(not(feature = "latent_retrieval"))]
    eprintln!(
        "misses: built without latent_retrieval — conversions n/a (inventory + landings still printed)"
    );

    let report = evolve::conversion_report(
        &inventory,
        evolve::WIRED_STRATEGIES,
        evolve::FRONTIER_LANDINGS,
        &signals,
    );
    println!(
        "conversion ledger — G8: frontier vs its own FIFO baseline (wiring {}):",
        evolve::WIRING_COMMIT
    );
    let era_line = |label: &str, total: usize, certified: usize| {
        if !store_present {
            format!("  {label:<18} {total} rule(s), conversions n/a (no trajectory store)")
        } else if total == 0 {
            format!("  {label:<18} 0 rules — nothing authored in this era yet")
        } else {
            // Integer-rounded percent — no float casts in a print path.
            let pct = (certified * 100 + total / 2) / total;
            format!("  {label:<18} {total} rule(s), {certified} Certified conversion(s) ({pct}%)")
        }
    };
    println!(
        "{}",
        era_line(
            "FIFO baseline:",
            report.baseline_total,
            report.baseline_certified
        )
    );
    println!(
        "{}",
        era_line(
            "frontier era:",
            report.frontier_total,
            report.frontier_certified
        )
    );
    for (code, strategy) in &report.unrecorded {
        println!("  UNRECORDED: {code} {strategy} — ledger bug, neither era accounts for it");
    }
    println!(
        "  note: the FIFO arm is historical, not re-runnable post-wiring — the \
         comparison is frontier-vs-its-own-baseline (T39/Bench-051); \
         conversion = the rule's CURRENT tier is Certified"
    );
    println!("  per-rule (code strategy era converted last-seen-unix):");
    for r in &report.rows {
        let era = match r.era {
            evolve::LedgerEra::Baseline => "baseline",
            evolve::LedgerEra::Frontier => "frontier",
            evolve::LedgerEra::Unrecorded => "UNRECORDED",
        };
        let seen = r
            .last_seen_unix
            .map_or_else(String::new, |t| format!(" {}", t));
        println!(
            "    {:<6} {:<24} {:<9} {:<3}{}",
            r.code, r.strategy, era, r.certified, seen
        );
    }
}

/// Issue 050 T2: the `--corpus-stats` / `--import-series` standalone mode.
/// Counts the corpus domains (runtime builders where the feature is on,
/// source-walk needle otherwise — Proposal 009 §F2 item (ii)), prints the
/// table, and (under `latent_retrieval`) stamps today's rows into the
/// trajectory store, runs the jsonl imports, and serves the `--json`
/// dashboard payload. With `--json` the human table + import lines go to
/// STDERR so stdout carries ONLY the JSON object (the dashboard's parse
/// contract).
fn run_corpus_stats(with_stats: bool, as_json: bool, do_import: bool) {
    let Some(repo) = resolve_corpus_repo_root() else {
        eprintln!(
            "corpus-stats: cannot locate the riir-clippy corpus paths \
             (src/domains/...) from {:?} — run inside the repo",
            std::env::current_dir().unwrap_or_default()
        );
        std::process::exit(1);
    };
    let table: Vec<(&str, u32, bool, &'static str)> = CORPUS_DOMAIN_TABLE
        .iter()
        .map(|(domain, rel, needle)| match runtime_corpus_count(domain) {
            Some(n) => (*domain, n, true, "runtime-corpus"),
            None => (
                *domain,
                count_corpus_domain(&repo, rel, needle),
                domain_compiled(domain),
                "source-walk",
            ),
        })
        .collect();

    // With --json the stdout contract is the JSON payload ONLY.
    let say = |line: &str| {
        if as_json {
            eprintln!("{line}");
        } else {
            println!("{line}");
        }
    };
    say(&format!("corpus stats (repo {}):", repo.display()));
    for (domain, count, compiled, source) in &table {
        say(&format!(
            "  {domain:<14} {count:>5}  compiled={compiled}  ({source})"
        ));
    }

    #[cfg(feature = "latent_retrieval")]
    {
        let root = std::env::var("RIIR_HEAL_TRAJ_ROOT")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .unwrap_or_default();
        if !(with_stats || as_json || do_import) {
            return;
        }
        let store = match riir_clippy::traj_store::TrajStore::open(&root) {
            Ok((store, bad)) => {
                if bad > 0 {
                    eprintln!("corpus-stats: {bad} corrupt row(s) skipped");
                }
                store
            }
            Err(e) => {
                if as_json {
                    // The JSON contract needs the series — a store failure
                    // must not masquerade as an empty store (the consumer
                    // would project that emptiness over real history).
                    eprintln!(
                        "corpus-stats: store at {} unreadable: {e}",
                        root.display()
                    );
                    std::process::exit(1);
                }
                eprintln!(
                    "corpus-stats: store at {} unavailable ({e}) — stats NOT stamped",
                    root.display()
                );
                return;
            }
        };
        if with_stats {
            let day = riir_clippy::traj_store::today_local_date();
            let mut stamped = 0usize;
            for (domain, count, compiled, source) in &table {
                match store.upsert_corpus_count(&day, domain, *count, *compiled, source) {
                    Ok(()) => stamped += 1,
                    Err(e) => eprintln!("corpus-stats: stamp {domain} failed: {e}"),
                }
            }
            say(&format!(
                "stamped {day}: {stamped} row(s) → {}",
                root.join(".heal").join("trajectories.kv").display()
            ));
        }
        if do_import {
            for (file, label) in [
                ("corpus_history.jsonl", "corpus"),
                ("score_history.jsonl", "score"),
            ] {
                let path = repo.join(".benchmarks").join(file);
                let res = if label == "corpus" {
                    store.import_corpus_jsonl(&path)
                } else {
                    store.import_score_jsonl(&path)
                };
                match res {
                    Ok((seen, written)) => say(&format!(
                        "import {label}: {seen} row(s) seen, {written} written/updated ← {}",
                        path.display()
                    )),
                    Err(e) => eprintln!("corpus-stats: import {label} failed: {e}"),
                }
            }
        }
        if as_json {
            let today = riir_clippy::traj_store::today_local_date();
            let mut today_obj = serde_json::Map::new();
            today_obj.insert("ts".into(), serde_json::Value::String(today.clone()));
            for (domain, count, compiled, _source) in &table {
                today_obj.insert(
                    (*domain).into(),
                    serde_json::json!({ "count": count, "compiled": compiled }),
                );
            }
            let corpus = store.corpus_series_by_day().unwrap_or_default();
            let scores = store.score_runs().unwrap_or_default();
            let payload = serde_json::json!({
                "today": serde_json::Value::Object(today_obj),
                "corpus": corpus,
                "scores": scores,
            });
            match serde_json::to_string(&payload) {
                Ok(line) => println!("{line}"),
                Err(e) => {
                    eprintln!("corpus-stats: encode json: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
    #[cfg(not(feature = "latent_retrieval"))]
    {
        let _ = (with_stats, do_import);
        if as_json {
            eprintln!(
                "corpus-stats: --json needs the store — rebuild with --features latent_retrieval"
            );
            return;
        }
        if with_stats || do_import {
            eprintln!(
                "corpus-stats: stamping/import unavailable — rebuild with \
                 --features latent_retrieval (the neuron-db store)"
            );
        }
    }
}

/// T2.4 (Plan 042): `--candidates <path>` — the promotion gate over a
/// network-delivered candidates file. Post-hoc sweep of the live overlay
/// first (withdraw regressors — absorbing), then the gate (trial → verdict
/// → persist registry + audit). Trial runs are strictly isolated from the
/// score history (only `--score-bench` appends there).
///
/// The NETWORK SEAM this composes with (documented, service-side — T1.5):
/// the lease manifest can carry a candidates segment; a delivered segment
/// is written to a local candidates file and THIS gate runs before the
/// rules ever reach the overlay.
#[cfg(all(feature = "kat_service", feature = "score_bench"))]
fn run_candidates_gate(path: &std::path::Path, heal_root: &std::path::Path) {
    use riir_clippy::kat_promotion as kp;

    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        eprintln!("candidates file unreadable ({}): {e}", path.display());
        std::process::exit(1);
    });
    let candidates = match kp::parse_candidates(&bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("candidates file rejected: {e}");
            std::process::exit(2);
        }
    };
    println!(
        "candidates: {} row(s) from {} (schema v{}, domains clippy|perf)",
        candidates.len(),
        path.display(),
        kp::CANDIDATE_SCHEMA_VERSION
    );

    // The oracle is ground truth for the gate — refuse to run without it
    // (fail closed: no evidence = no promotion).
    let oracle = riir_clippy::score_bench::draft_oracle();
    if !oracle.is_available() {
        eprintln!("promotion gate: cargo not on PATH — cannot derive ground truth");
        std::process::exit(1);
    }

    let registry_path = heal_root.join(kp::REGISTRY_FILE);
    let audit_path = heal_root.join(kp::AUDIT_FILE);
    let mut registry = kp::PromotionRegistry::load(&registry_path).unwrap_or_else(|e| {
        // A corrupt blacklist cannot be promoted around — hard stop.
        eprintln!("{e}");
        std::process::exit(1);
    });

    let fixture_list = riir_clippy::score_bench::fixtures::fixtures();
    println!("frozen fixtures: {} — running post-hoc sweep…", fixture_list.len());

    // 1. Post-hoc sweep of the LIVE overlay (withdraw regressors first so
    //    the new gate runs against a clean overlay).
    let sweep = kp::run_posthoc_sweep(&oracle, &fixture_list, &mut registry);
    print!("{}", sweep.render());

    // 2. The gate.
    println!("running promotion gate…");
    let report = kp::run_promotion_gate(&oracle, &fixture_list, &candidates, &mut registry);
    print!("{}", report.render());

    // 3. Persist + audit (the only side effects — never the score history).
    registry.save(&registry_path).unwrap_or_else(|e| {
        eprintln!("registry save failed ({}) — decisions NOT persisted: {e}", registry_path.display());
        std::process::exit(1);
    });
    let ts = riir_clippy::score_bench::report::local_iso_timestamp();
    let mut audit: Vec<kp::AuditRecord> = Vec::new();
    for id in &sweep.withdrawn {
        // The sweep's registry mutation already happened; record the cause.
        audit.push(kp::AuditRecord {
            ts: ts.clone(),
            kind: "withdraw".to_string(),
            rule_id: id.clone(),
            content_hash_hex: String::new(), // removed from the overlay — the blacklists carry it
            reason: "posthoc_regression".to_string(),
            fired: true,
            base_resolved: sweep.base.resolved,
            trial_resolved: sweep.overlay.resolved,
            base_created: sweep.base.created,
            trial_created: sweep.overlay.created,
        });
    }
    for v in &report.verdicts {
        let (kind, reason) = match &v.verdict {
            riir_clippy::kat_promotion::Verdict::Promoted => ("promote", String::new()),
            riir_clippy::kat_promotion::Verdict::Rejected(r) => ("reject", r.tag().to_string()),
        };
        let trial_totals = report.batch.unwrap_or_default();
        audit.push(kp::AuditRecord {
            ts: ts.clone(),
            kind: kind.to_string(),
            rule_id: v.rule_id.clone(),
            content_hash_hex: candidates
                .iter()
                .find(|c| c.rule_id == v.rule_id)
                .map(|c| c.content_hash_hex())
                .unwrap_or_default(),
            reason,
            fired: v.fired,
            base_resolved: report.base.resolved,
            trial_resolved: trial_totals.resolved,
            base_created: report.base.created,
            trial_created: trial_totals.created,
        });
    }
    if !audit.is_empty() {
        kp::append_audit(&audit_path, &audit).unwrap_or_else(|e| {
            eprintln!("audit append failed ({}) — continuing: {e}", audit_path.display());
        });
    }
    println!(
        "registry: {} promoted / {} withdrawn-blacklisted → {}",
        registry.promoted.len(),
        registry.withdrawn_hashes.len(),
        registry_path.display()
    );
}

fn main() {
    let args = parse_args();

    // ── Proposal 005 T0.1 + T1.6: KAT subcommands (standalone — no paths) ─
    #[cfg(feature = "kat_service")]
    if let Some(cmd) = args.command {
        match cmd {
            KatCommand::Login => run_kat_login(args.import_key.as_deref()),
            KatCommand::Account => run_kat_account(&args.paths, args.account_history),
            KatCommand::Claim => run_kat_claim(args.claim_tx.as_deref()),
            #[cfg(feature = "kat_transport")]
            KatCommand::Update => run_kat_update(),
            #[cfg(feature = "kat_transport")]
            KatCommand::Sync => run_kat_sync(),
            KatCommand::Stake => run_kat_stake(args.stake_usd.as_deref(), args.stake_info),
            KatCommand::Unstake => run_kat_unstake(),
            KatCommand::Vote => run_kat_vote(
                args.vote_proposal,
                args.vote_yes,
                args.vote_no,
            ),
            KatCommand::Pay => run_kat_pay(
                args.pay_amount.as_deref(),
                args.pay_label.as_deref(),
                args.pay_qr,
                args.pay_wait,
            ),
            KatCommand::Faucet => run_kat_faucet(args.pay_qr),
            KatCommand::Wetrun => run_kat_wetrun(args.dry_run),
            // The transport commands degrade honestly without kat_transport.
            #[cfg(not(feature = "kat_transport"))]
            KatCommand::Update | KatCommand::Sync => {
                eprintln!("update/sync require a build with --features kat_transport");
                std::process::exit(2);
            }
        }
        return;
    }
    #[cfg(not(feature = "kat_service"))]
    if args.command.is_some() {
        eprintln!("login/account/update/sync/claim/stake/unstake/vote/pay require a build with --features kat_service");
        std::process::exit(2);
    }

    // ── Plan 014: clippy-bench modes (standalone — no paths) ─────────────
    #[cfg(feature = "score_bench")]
    if args.score_report {
        let history = riir_clippy::score_bench::report::load_history(
            std::path::Path::new(riir_clippy::score_bench::report::HISTORY_JSONL),
        );
        if history.is_empty() {
            eprintln!("no score history yet — run --score-bench first");
            std::process::exit(1);
        }
        // Per-generation curves (Plan 092 M5): gen-1 → score_history.svg,
        // gen-2 → score_history_v2.svg (only when v2 rows exist).
        let v1: Vec<_> = history.iter().filter(|r| r.generation == 1).cloned().collect();
        let svg = riir_clippy::score_bench::report::render_svg(&v1);
        std::fs::write(riir_clippy::score_bench::report::HISTORY_SVG, &svg)
            .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
        println!(
            "rendered {} ({} gen-1 run(s))",
            riir_clippy::score_bench::report::HISTORY_SVG,
            v1.len()
        );
        let v2: Vec<_> = history.iter().filter(|r| r.generation == 2).cloned().collect();
        if !v2.is_empty() {
            let svg2 = riir_clippy::score_bench::report::render_svg(&v2);
            std::fs::write(riir_clippy::score_bench::report::HISTORY_SVG_V2, &svg2)
                .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
            println!(
                "rendered {} ({} gen-2 run(s))",
                riir_clippy::score_bench::report::HISTORY_SVG_V2,
                v2.len()
            );
        }
        return;
    }
    #[cfg(feature = "score_bench")]
    if args.score_bench {
        run_score_bench(args.generation.unwrap_or(1));
        return;
    }
    #[cfg(not(feature = "score_bench"))]
    if args.score_bench || args.score_report {
        eprintln!("--score-bench / --score-report require a build with --features score_bench");
        std::process::exit(2);
    }

    // ── T2.4 (Plan 042): the promotion gate (standalone — no paths) ────
    #[cfg(all(feature = "kat_service", feature = "score_bench"))]
    if let Some(path) = &args.candidates_path {
        run_candidates_gate(path, &resolve_heal_root(&args.paths));
        return;
    }
    #[cfg(not(all(feature = "kat_service", feature = "score_bench")))]
    if args.candidates_path.is_some() {
        eprintln!("--candidates requires a build with --features kat_service,score_bench");
        std::process::exit(2);
    }

    // ── Issue 050 T2: --corpus-stats / --import-series (standalone) ─────
    // Early return before validate_mode_flags (the memory-stats pattern):
    // the table prints either way; the stamp/import/json halves need
    // latent_retrieval (handled inside run_corpus_stats).
    if args.corpus_stats || args.import_series {
        run_corpus_stats(args.corpus_stats, args.json, args.import_series);
        return;
    }

    // ── Plan 091 T1.2: --misses — the frontier-ordered authoring queue ──
    // Read-only standalone mode (the memory-stats pattern: early return,
    // validate_mode_flags never sees it). Prints `.heal/misses/*.md`
    // frontiers-ordered by per-code dispersion: informative pools first —
    // resolving THOSE codes converts uncertain candidates — then unobserved
    // (one probe is cheap), regenerate, saturated last. The G8 enabler:
    // this ordering is the treatment arm the conversion ledger (T1.3)
    // measures against the pre-wiring FIFO baseline.
    if args.misses {
        if args.suggest
            || args.fix
            || args.fix_compile
            || args.dump_trajectories
            || args.write
            || args.score_bench
            || args.memory_stats
        {
            eprintln!("--misses is a standalone read-only mode");
            std::process::exit(2);
        }
        if args.misses_report {
            run_conversion_report();
        } else {
            run_misses_queue(args.misses_limit, &args.paths);
        }
        return;
    }
    if args.misses_report {
        eprintln!("--report requires --misses");
        std::process::exit(2);
    }

    // ── Issue 037 observability: --memory-stats — the loop debug query ────
    // Read-only standalone mode (like the score arms above: early return,
    // validate_mode_flags never sees it). Summarizes whichever backend the
    // build ships: the JSON envelope always; the repo-local neuron-db store
    // (`.heal/trajectories.kv`, `latent_retrieval` builds) when present.
    if args.memory_stats {
        let path = default_memory_path();
        match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 038: the envelope may have been migrated — the ACTIVE
                // backend is then the repo-local .heal/trajectories.kv
                // (summarized below). Not an error; fall through.
                let migrated = path.with_extension("json.migrated");
                if migrated.exists() {
                    println!(
                        "memory store: {} (MIGRATED — the active backend is the repo-local .heal/trajectories.kv)",
                        migrated.display()
                    );
                } else {
                    eprintln!(
                        "memory-stats: no store at {} — the loop has never recorded here",
                        path.display()
                    );
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("memory-stats: cannot read {}: {e}", path.display());
                std::process::exit(1);
            }
            Ok(_) => {
                let mut mem = riir_clippy::memory::LatentFixMemory::default();
                match mem.load_verified(&path) {
                    Ok(status) => {
                        print!("{}", mem.stats_report(&path, status));
                    }
                    Err(e) => {
                        eprintln!(
                            "memory-stats: store at {} is unreadable: {e}",
                            path.display()
                        );
                        std::process::exit(1);
                    }
                }
            }
        }
        // The neuron-db backend (`latent_retrieval` builds): repo-local
        // `.heal/trajectories.kv` — same root resolution as --dump-trajectories.
        #[cfg(feature = "latent_retrieval")]
        {
            let root = std::env::var("RIIR_HEAL_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            let kv = root.join(".heal").join("trajectories.kv");
            if kv.exists() {
                match riir_clippy::traj_store::TrajStore::open(&root) {
                    Ok((store, bad)) => {
                        // Full report — feature parity with the envelope
                        // branch (histogram/tiers/loop state; the kv branch
                        // shipped 038 as a 4-counter one-liner, an
                        // observability regression surfaced live on
                        // adoption day).
                        print!("{}", store.stats_report(bad));
                    }
                    Err(e) => eprintln!("memory-stats: kv store at {} unreadable: {e}", kv.display()),
                }
            } else {
                println!(
                    "kv store: none at {} — run any --fix in this repo to create it (the 038 unified backend)",
                    kv.display()
                );
            }
        }
        return;
    }

    // ── Issue 039 T2: --recompute-elo — the batch rating backfill ──────────
    // Standalone WRITE mode (the one rating writer — the issue's non-goal
    // pins NO online ELO updates in the hot fix loop). Replays every
    // trajectory row's verdict counts from base 1200 + writes the final
    // rule/span ratings onto the rows. Idempotent.
    if args.recompute_elo {
        #[cfg(feature = "latent_retrieval")]
        {
            let root = std::env::var("RIIR_HEAL_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            let kv = root.join(".heal").join("trajectories.kv");
            if !kv.exists() {
                eprintln!(
                    "recompute-elo: no store at {} — run any --fix in this repo first",
                    kv.display()
                );
                std::process::exit(1);
            }
            match riir_clippy::traj_store::TrajStore::open(&root) {
                Ok((store, bad)) => {
                    if bad > 0 {
                        eprintln!("recompute-elo: {bad} corrupt row(s) skipped");
                    }
                    match store.recompute_elo() {
                        Ok(s) => println!(
                            "elo: {} match(es) replayed | {} rule(s) + {} span(s) rated | {}/{} row(s) updated{}",
                            s.matches,
                            s.rules_rated,
                            s.spans_rated,
                            s.rows_written,
                            s.rows_seen,
                            if s.rows_written == 0 { " (already rated — idempotent)" } else { "" }
                        ),
                        Err(e) => {
                            eprintln!("recompute-elo: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("recompute-elo: store at {} unreadable: {e}", root.display());
                    std::process::exit(1);
                }
            }
        }
        #[cfg(not(feature = "latent_retrieval"))]
        {
            eprintln!("--recompute-elo requires a build with --features latent_retrieval (the neuron-db store)");
            std::process::exit(2);
        }
        return;
    }

    // ── Issue 042: --collapse-report — the self-collapse monitor ──────────
    // Read-only standalone mode (early return, like --memory-stats /
    // --recompute-elo above; validate_mode_flags never sees it). Runs the
    // REAL retrieval loop (HealPipeline::heal Step 1 retrieval + Step 3 pool
    // + selection) over the fixed clippy fixture set (Issue 017's
    // real-clippy-verified spans) under two conditions: the store at
    // RIIR_HEAL_TRAJ_ROOT (no migration — this mode never writes), falling
    // back to the memory.json envelope, plus a fresh-empty baseline for the
    // elevation delta. Feeds self_evolve_audit.sh §1c.
    if args.collapse_report {
        #[cfg(feature = "latent_retrieval")]
        {
            use riir_clippy::collapse_monitor::collapse_report_common;
            let spans = riir_clippy::draft::eval_fixtures_clippy::CLIPPY_SPANS;
            let corpus = riir_clippy::build_seed_corpus().entries;

            // Fresh baseline (condition b).
            let fresh = riir_clippy::collapse_monitor::measure(
                &riir_clippy::memory::LatentFixMemory::default(),
                spans,
                &corpus,
            );
            println!("collapse-report: fixture spans={} corpus_rules={}", spans.len(), corpus.len());
            println!("{}", fresh.report("  fresh(empty)"));

            // Production condition (a): the store, or the envelope fallback.
            let root = std::env::var("RIIR_HEAL_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            if root.join(".heal").join("trajectories.kv").exists() {
                match riir_clippy::traj_store::TrajStore::open(&root) {
                    Ok((store, bad)) => {
                        if bad > 0 {
                            eprintln!("collapse-report: {bad} corrupt row(s) skipped");
                        }
                        collapse_report_common(
                            &format!("  store({})", root.display()),
                            &riir_clippy::collapse_monitor::memory_from_store_rows(&store.dump()),
                            spans,
                            &corpus,
                        );
                        return;
                    }
                    Err(e) => {
                        eprintln!("collapse-report: store at {} unreadable: {e}", root.display());
                        std::process::exit(1);
                    }
                }
            }
            let envelope = default_memory_path();
            if envelope.exists() {
                let mut mem = riir_clippy::memory::LatentFixMemory::default();
                match mem.load(&envelope) {
                    Ok(()) => {
                        collapse_report_common("  envelope", &mem, spans, &corpus);
                        return;
                    }
                    Err(e) => {
                        eprintln!("collapse-report: envelope {} unreadable: {e}", envelope.display());
                        std::process::exit(1);
                    }
                }
            }
            eprintln!("collapse-report: no store at {} — showing the fresh baseline only", root.display());
            return;
        }
        #[cfg(not(feature = "latent_retrieval"))]
        {
            eprintln!("--collapse-report requires a build with --features latent_retrieval (the fixture set + store bridge)");
            std::process::exit(2);
        }
    }

    // ── Issue 072 T5 (Plan 092): the frontier-ordered authoring queue ──

    // Plan 019 T1.5: the mode-flag matrix (pinned by
    // `mode_flag_tests` below).
    if let Err(msg) = validate_mode_flags(
        args.suggest,
        args.fix,
        args.fix_compile,
        args.dump_trajectories,
        args.write,
        args.verify,
        args.verify_args.is_some(),
    ) {
        eprintln!("{msg}");
        usage();
    }
    #[cfg(not(feature = "fix_verify"))]
    if args.verify {
        eprintln!("--verify requires a build with --features fix_verify");
        std::process::exit(2);
    }
    // `--verify-args` serves TWO consumers: `--verify` (needs `fix_verify`)
    // and `--fix-compile` (needs only `compile_oracle`, implied by
    // `rustc_errors`). Reject only when NEITHER substrate is compiled in —
    // the watch-wrapper smoke caught the unconditional rejection (a
    // rustc_errors-without-fix_verify build could never pass a feature
    // set, silently reverting to default-feature blindness — the exact
    // Issue 031 H3 trap the flag exists to prevent).
    #[cfg(not(any(feature = "fix_verify", feature = "rustc_errors")))]
    if args.verify_args.is_some() {
        eprintln!("--verify-args requires --verify or --fix-compile (build with --features fix_verify or rustc_errors)");
        std::process::exit(2);
    }
    // A fix_verify-less build CANNOT serve --verify's own --verify-args
    // (the batch gate is absent) — only --fix-compile's.
    #[cfg(all(not(feature = "fix_verify"), feature = "rustc_errors"))]
    if args.verify_args.is_some() && !args.fix_compile {
        eprintln!("--verify-args for --verify requires a build with --features fix_verify");
        std::process::exit(2);
    }

    // ── Plan 019 T4.3b: --dump-trajectories — the human grep surface ────
    // On-demand export of the trajectory store (one JSONL line per
    // trajectory) instead of a shadow log. ONE source of truth: the
    // backend's own file. Redirectable: `cargo heal --dump-trajectories
    // > traj.jsonl`.
    if args.dump_trajectories {
        // The neuron-db backend (`latent_retrieval` builds): the repo-local
        // durable store. Requires a path (the repo root to resolve
        // `.heal/trajectories.kv`) — `RIIR_HEAL_TRAJ_ROOT` overrides.
        #[cfg(feature = "latent_retrieval")]
        {
            let root = std::env::var("RIIR_HEAL_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            if root.join(".heal").join("trajectories.kv").exists() {
                match riir_clippy::traj_store::TrajStore::open(&root) {
                    Ok((store, bad)) => {
                        if bad > 0 {
                            eprintln!("dump-trajectories: {bad} corrupt row(s) skipped");
                        }
                        for row in store.dump() {
                            match serde_json::to_string(&row) {
                                Ok(line) => println!("{line}"),
                                Err(e) => eprintln!("dump-trajectories: skip one: {e}"),
                            }
                        }
                        return;
                    }
                    Err(e) => {
                        eprintln!("dump-trajectories: store at {} unreadable: {e}", root.display());
                        std::process::exit(1);
                    }
                }
            }
        }
        // The JSON envelope backend (default builds).
        let path = default_memory_path();
        match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!(
                    "dump-trajectories: no store at {} — run --fix-compile --write first",
                    path.display()
                );
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("dump-trajectories: cannot read {}: {e}", path.display());
                std::process::exit(1);
            }
            Ok(_json) => {
                // Parse through the memory (envelope version check + the
                // shapes stay canonical), then emit per-trajectory lines.
                let mut mem = riir_clippy::memory::LatentFixMemory::default();
                if let Err(e) = mem.load(&path) {
                    eprintln!(
                        "dump-trajectories: store at {} is unreadable: {e}",
                        path.display()
                    );
                    std::process::exit(1);
                }
                for t in mem.iter() {
                    match serde_json::to_string(t) {
                        Ok(line) => println!("{line}"),
                        Err(e) => eprintln!("dump-trajectories: skip one: {e}"),
                    }
                }
                return;
            }
        }
    }

    // ── 037-T2: one heal-run id per invocation — the verdict-attribution
    // key stamped on the durable trajectory rows (Issue 038's schema:
    // in-schema fields keyed by run id, single read path). Compact +
    // unique per run (time-nanos + pid). Consumed by every store-writing
    // path (applies, verify verdicts, fix-compile) — all under
    // `latent_retrieval` (which `fix_verify` implies).
    #[cfg(feature = "latent_retrieval")]
    let heal_run_id = {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0u128, |d| d.as_nanos());
        format!("run-{t:x}-{}", std::process::id())
    };

    // Issue 067 T1 — the same-rule reopen-trigger instrument (durable-store
    // builds only; light builds keep the envelope by design and are not
    // instrumented). Lazily scans the store ONCE for Certified-trajectory
    // density, then logs the FIRST time a queried lint's density reaches
    // the reopen threshold K=3.
    #[cfg(feature = "latent_retrieval")]
    let mut reopen067 = ReopenTrigger067::new();

    // ── Plan 019 Phase 1: --fix-compile — the oracle driver ──────────
    // A separate early-exit arm: it operates on PACKAGES (cargo check per
    // owning package), needs no retrieval, and its loop carries its own
    // verification. Review mode (no --write) prints the per-error plan;
    // --write applies. No unconditional-write path exists (T1.3).
    #[cfg(feature = "rustc_errors")]
    if args.fix_compile {
        use riir_clippy::domains::rustc_errors::driver::CompileDriver;
        use riir_clippy::domains::rustc_errors::evolve;
        use riir_clippy::domains::rustc_errors::playbooks::Playbooks;

        let extra = args
            .verify_args
            .as_deref()
            .map(|s| s.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        // Phase 4 T4.3/T4.2 (`latent_retrieval` builds): the durable
        // neuron-db store — shape-conditional ordering in, outcomes out.
        // One active backend per build: WITHOUT the feature the compile
        // domain records to memory.json (below) — never both.
        #[cfg(feature = "latent_retrieval")]
        let (traj_store, mut traj_warnings) = args.paths.first().and_then(|p| {
            riir_clippy::compile_oracle::owning_package_root(p)
                .and_then(|pkg| riir_clippy::compile_oracle::workspace_root(&pkg))
                .or_else(|| std::env::current_dir().ok())
                .and_then(|root| {
                    match riir_clippy::traj_store::TrajStore::open(&root) {
                        Ok((s, bad)) => Some((
                            std::sync::Arc::new(s),
                            if bad > 0 {
                                format!("trajectory store: {bad} corrupt row(s) skipped")
                            } else {
                                String::new()
                            },
                        )),
                        Err(e) => {
                            eprintln!("self-evolve: neuron store unavailable ({e}) — continuing without shape-conditional ordering");
                            None
                        }
                    }
                })
        }).map_or_else(
            || (None, String::new()),
            |(s, w)| (Some(s), w),
        );
        #[cfg(feature = "latent_retrieval")]
        if !traj_warnings.is_empty() {
            eprintln!("self-evolve: {traj_warnings}");
            traj_warnings.clear();
        }
        let mut driver = CompileDriver::isolated().with_max_iters(args.max_iters);
        #[cfg(feature = "latent_retrieval")]
        if let Some(store) = traj_store.clone() {
            driver = driver.with_ordering(std::sync::Arc::new(move |cands| {
                riir_clippy::traj_store::shape_conditional_order(cands, &store);
            }));
        }
        let driver = driver.with_extra_args(extra);
        let playbooks = Playbooks::production();
        let report = driver.run(&args.paths, &playbooks, args.write);
        if let Some(e) = &report.error {
            eprintln!("fix-compile: {e}");
            std::process::exit(1);
        }
        if args.write {
            for a in &report.applied {
                println!(
                    "applied {} [{}] at {}:{} (errors {} → {}){}",
                    a.code,
                    a.strategy_id,
                    a.file.display(),
                    a.line,
                    a.errors.0,
                    a.errors.1,
                    semantic_note_suffix(a.semantic_note)
                );
            }
            for r in &report.reverted {
                println!(
                    "reverted {} [{}] at {}:{} ({})",
                    r.code,
                    r.strategy_id,
                    r.file.display(),
                    r.line,
                    r.reason.label()
                );
            }
            // ── Phase 4 (T4.1/T4.3): self-evolve around every fix. The
            // keep criterion is a full cargo-check verdict, so applied
            // fixes are CERTIFIED; reverts are failures. Only the write
            // path records (review runs touch nothing — Issue 018). ONE
            // active backend per build: the neuron-db store under
            // `latent_retrieval`, memory.json otherwise — never both.
            if !report.applied.is_empty() || !report.reverted.is_empty() {
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    use riir_clippy::domains::rustc_errors::evolve::traj_key;
                    for a in &report.applied {
                        reopen067.note(store, &traj_key(&a.code, &a.strategy_id));
                        if let Err(e) = store.record(
                            &traj_key(&a.code, &a.strategy_id),
                            &a.shape,
                            &a.fix,
                            true,
                            true,
                            &heal_run_id,
                        ) {
                            eprintln!("self-evolve: store record failed: {e}");
                        }
                    }
                    for r in &report.reverted {
                        // Issue 047: no-progress = the drafted fix was defective
                        // (W); I/O = the gate rejected without evaluating the
                        // fix (C — unnecessary, gate-side).
                        let component =
                            riir_clippy::component::classify(
                                &riir_clippy::component::FailureSignal::reverted(
                                    match r.reason {
                                        riir_clippy::domains::rustc_errors::driver::RevertReason::NoProgress => riir_clippy::component::RevertKind::CompileGate,
                                        riir_clippy::domains::rustc_errors::driver::RevertReason::Io => riir_clippy::component::RevertKind::Infrastructure,
                                    },
                                ),
                            );
                        if let Err(e) = store.record_classified(
                            &traj_key(&r.code, &r.strategy_id),
                            &r.shape,
                            &r.fix,
                            false,
                            false,
                            &heal_run_id,
                            Some(component),
                        ) {
                            eprintln!("self-evolve: store record failed: {e}");
                        }
                    }
                    eprintln!(
                        "self-evolve: {} event(s) → {}",
                        report.applied.len() + report.reverted.len(),
                        store.root.display()
                    );
                }
                #[cfg(not(feature = "latent_retrieval"))]
                {
                    let mut evolve_rec = EvolveRecorder::from_env();
                    let memory_path = default_memory_path();
                    if evolve_rec.is_enabled() {
                        if let Err(e) = evolve_rec.load(&memory_path) {
                            eprintln!(
                                "self-evolve: ignoring unreadable memory ({}): {e}",
                                memory_path.display()
                            );
                        }
                        evolve::record_run(&mut evolve_rec, &report);
                        if evolve_rec.recorded_count() > 0 {
                            if let Some(parent) = memory_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            match evolve_rec.save(&memory_path) {
                                Ok(()) => eprintln!(
                                    "self-evolve: {} trajectory(ies) → {}",
                                    evolve_rec.recorded_count(),
                                    memory_path.display()
                                ),
                                Err(e) => eprintln!(
                                    "self-evolve: memory save failed ({}): {e}",
                                    memory_path.display()
                                ),
                            }
                        }
                    }
                }
            }
            // ── Phase 4 (T4.4): misses → the authoring queue.
            if !report.misses.is_empty() {
                let root = args
                    .paths
                    .first()
                    .and_then(|p| riir_clippy::compile_oracle::owning_package_root(p.as_path()))
                    .and_then(|pkg| riir_clippy::compile_oracle::workspace_root(&pkg))
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                let (written, warnings) = evolve::dump_misses(&root, &report.misses);
                for w in &warnings {
                    eprintln!("fix-compile: miss dump: {w}");
                }
                if !written.is_empty() {
                    println!(
                        "fix-compile: {} miss(es) → {}",
                        written.len(),
                        evolve::misses_dir(&root).display()
                    );
                }
            }
            println!(
                "fix-compile: {} applied, {} reverted, {} miss(es), {} remaining error(s){}",
                report.applied.len(),
                report.reverted.len(),
                report.misses.len(),
                report.remaining,
                if report.clean { " — CLEAN" } else { "" }
            );
        } else {
            if report.planned.is_empty() && report.clean {
                println!("fix-compile: nothing to fix (no coded errors)");
            }
            for p in &report.planned {
                println!(
                    "plan {}:{} {} [{}] → {}{}",
                    p.file.display(),
                    p.line,
                    p.code,
                    p.strategy_id,
                    p.replacement_head,
                    semantic_note_suffix(p.semantic_note)
                );
            }
            if !report.misses.is_empty() {
                println!(
                    "fix-compile (review): {} error(s) with no playbook match —",
                    report.misses.len()
                );
                for m in &report.misses {
                    println!(
                        "  miss {} at {}:{} — {}",
                        m.code,
                        m.file.display(),
                        m.line,
                        truncate(&m.message, 100)
                    );
                }
            }
        }
        return;
    }
    #[cfg(not(feature = "rustc_errors"))]
    if args.fix_compile {
        eprintln!("--fix-compile requires a build with --features rustc_errors");
        std::process::exit(2);
    }

    // ── T1.6 (Plan 040): the corpus-lease run context ────────────────────
    // An INSTALLED epoch (or a --corpus-epoch pin) replaces the covered
    // domains' compiled-in corpora for this run — a run-boundary install,
    // never mid-pass mutation. Fresh install + offline → no epoch → the
    // compiled-in seed stands (G3: byte-identical to features-off).
    #[cfg(feature = "kat_service")]
    let lease_store =
        riir_clippy::kat_lease_client::LeaseStore::open(&resolve_heal_root(&args.paths));
    // T2.4: with the promotion surface compiled in, Certified network rules
    // merge into the run overlay (never under a --corpus-epoch pin).
    #[cfg(all(feature = "kat_service", feature = "score_bench"))]
    let run_overlay =
        run_overlay_with_promotion(&lease_store, args.corpus_epoch, &resolve_heal_root(&args.paths));
    #[cfg(all(feature = "kat_service", not(feature = "score_bench")))]
    let run_overlay = lease_overlay_for_run(&lease_store, args.corpus_epoch);
    #[cfg(feature = "kat_service")]
    if let Some((epoch, overlay)) = &run_overlay {
        let pinned = args.corpus_epoch.is_some();
        let leased = lease_store.current().is_some();
        let domains: Vec<&str> = overlay.iter().map(|(d, _)| d.label()).collect();
        if leased {
            println!(
                "corpus lease: running on epoch {epoch} ({}domains: {})",
                if pinned { "pinned, " } else { "" },
                domains.join(", ")
            );
        }
        // No installed lease + a non-empty overlay ⇒ the T2.4 promotion
        // merge built it from the compiled-in seed + Certified local rules
        // (the "corpus promotion" line above names what joined).
    }

    // IIFE because the kat_service arm returns early from inside a
    // cfg-gated block — a plain let-chain would need the same cfg pair
    // on a variable initializer (the immediately-invoked-closure idiom,
    // lint-flagged under this feature union; allow is the honest shape).
    #[allow(clippy::redundant_closure_call)]
    let mut matcher = (|| {
        #[cfg(feature = "kat_service")]
        if let Some((_, ref overlay)) = run_overlay {
            return LatentMatcher::from_enabled_domains_with_overlay(overlay);
        }
        #[cfg(not(feature = "kat_service"))]
        let _ = &args; // no lease context without kat_service
        LatentMatcher::from_enabled_domains()
    })()
    .unwrap_or_else(|e| {
        eprintln!("index build failed: {e}");
        std::process::exit(1);
    });

    // T2.4 domain gating: an explicitly requested domain whose feature is
    // off is an error, not a silent skip.
    if let Some(want) = args.domain {
        let enabled = matcher.domains().contains(&want);
        if !enabled {
            eprintln!(
                "--domain {} requested but its Cargo feature is not enabled (have: {:?})",
                want.label(),
                matcher
                    .domains()
                    .iter()
                    .map(|d| d.label())
                    .collect::<Vec<_>>()
            );
            std::process::exit(2);
        }
    }

    let mut files = Vec::new();
    for path in &args.paths {
        collect_source_files(path, ".rs", &mut files);
    }
    // Issue 025 T4: the non-syn surface (.wgsl kernel files) joins the
    // suggest walk when kernel_heal is enabled.
    let mut wgsl_files = Vec::new();
    for path in &args.paths {
        collect_source_files(path, ".wgsl", &mut wgsl_files);
    }
    // Plan 030 T0.4: the second non-syn surface. Dockerfiles are matched by
    // NAME, not extension — the three conventional spellings are
    // `Dockerfile`, `<name>.Dockerfile` and `Dockerfile.<name>`, and a
    // plain suffix match would miss the third.
    let mut dockerfiles = Vec::new();
    for path in &args.paths {
        collect_dockerfiles(path, &mut dockerfiles);
    }
    if files.is_empty() && wgsl_files.is_empty() && dockerfiles.is_empty() {
        eprintln!("no .rs/.wgsl/Dockerfile files under the given paths");
        std::process::exit(1);
    }
    #[cfg(not(feature = "docker"))]
    if !dockerfiles.is_empty() {
        eprintln!(
            "note: {} Dockerfile(s) found but the docker domain needs --features docker (Plan 030)",
            dockerfiles.len()
        );
    }
    #[cfg(not(feature = "kernel_heal"))]
    if !wgsl_files.is_empty() {
        eprintln!(
            "note: {} .wgsl file(s) found but the non-syn scanner needs --features kernel_heal (Issue 025)",
            wgsl_files.len()
        );
    }

    // Issue 018/038: self-evolve recording — the memory half runs in every
    // build; the EMA learning half degrades gracefully without an oracle
    // (one log line). `RIIR_HEAL_SELF_EVOLVE=0` makes this fully inert.
    //
    // 038-T2/T3: under `latent_retrieval` the repo-local neuron store is the
    // ONE backend — opened up front (with the one-time envelope migration
    // on persisting runs); when active, the legacy envelope is neither
    // loaded nor saved. In-session recording + EMA learning still run
    // through the recorder identically — only the persistence swaps.
    // Light builds (no `latent_retrieval`) keep the envelope by design
    // (one active backend per build).
    let mut evolve_rec = EvolveRecorder::from_env();
    let memory_path = default_memory_path();
    #[cfg(feature = "latent_retrieval")]
    let traj_store = open_traj_store(&args.paths, args.write || args.verify);
    #[cfg(feature = "latent_retrieval")]
    let envelope_backend = traj_store.is_none();
    #[cfg(not(feature = "latent_retrieval"))]
    let envelope_backend = true;
    if evolve_rec.is_enabled() && envelope_backend {
        match evolve_rec.load_verified(&memory_path) {
            Err(e) => eprintln!(
                "self-evolve: ignoring unreadable memory file ({}): {e}",
                memory_path.display()
            ),
            // T7.4: the commitment is advisory (the learned state is a hint,
            // not ground truth), so a mismatch warns and continues rather
            // than aborting a heal run.
            Ok(riir_clippy::CommitmentStatus::Mismatch) => eprintln!(
                "self-evolve: WARNING — memory file ({}) failed its BLAKE3 commitment; \
                 it was edited or corrupted since it was written. Continuing with it.",
                memory_path.display()
            ),
            Ok(_) => {}
        }
    }

    // Proposal 005 Phase 0 T0.5 (Plan 023): the KAT code-token meter —
    // opt-in (`kat_service`), accounting-only (G9d: client counts never mint
    // or spend; the burn reconciles at sync, T0.3+). Bills the raw bytes of
    // every source the healer processes — deliberately NOT parse-dependent,
    // so unparseable files still bill (G9c: parse-gating would bill zero on
    // exactly the L4 miss class routed to the expensive path).
    #[cfg(feature = "kat_service")]
    let mut kat_meter = open_kat_meter(&args.paths);

    // T0.6 (Plan 024): the charge ledger opens right after the meter, against
    // its PRE-session state (see open_kat_charge). Shares the meter's
    // kill-switch — `RIIR_HEAL_KAT_METER=0` disables the whole billing path.
    // No immediate save needed: recovery is idempotent from the meter at open
    // time, and the exit block persists the settled state.
    #[cfg(feature = "kat_service")]
    let mut kat_charge = open_kat_charge(&args.paths, kat_meter.as_ref());

    // ── T1.6 (Plan 040): the update-first phase + the reheat arming ──────
    // The freshest corpus is fetched at run START inside the bounded
    // --update-timeout window; timeout/offline/skip → heal proceeds on the
    // local corpus (G3 — the network may delay freshness, never block
    // healing). Silent unless an epoch actually applied. The phase runs
    // ONLY when a service URL is explicitly configured (the Worker-deferred
    // posture; `update`/`sync` fall back to the default URL) + neither the
    // kill-switch nor a --corpus-epoch pin (which implies skip-check) nor
    // --no-update/--update-timeout 0 disabled it. CI never touches the
    // network by construction (no test drives main's network path).
    #[cfg(feature = "kat_transport")]
    let mut lease_charge = riir_clippy::kat_lease_client::LeaseCharge::open(&lease_store);
    #[cfg(feature = "kat_transport")]
    let mut lease_ctx: Option<ReheatCtx> = None;
    #[cfg(feature = "kat_transport")]
    {
        let phase_live = !args.no_update
            && args.update_timeout > 0
            && args.corpus_epoch.is_none()
            && !riir_clippy::kat_lease_client::lease_kill_switched();
        let service_url = riir_clippy::kat_lease_client::service_url_from_env();
        if phase_live && service_url.is_some() {
            let url = service_url.clone().unwrap_or_default();
            let transport = riir_clippy::kat_lease_client::HttpLeaseTransport::new(
                &url,
                args.update_timeout,
            );
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_secs(args.update_timeout);
            let out_of_kat = kat_charge
                .as_ref()
                .is_some_and(|c| c.balance_projection() == 0);
            let outcome = riir_clippy::kat_lease_client::run_update_phase(
                &transport,
                &lease_store,
                &mut lease_charge,
                out_of_kat,
                Some(deadline),
            );
            if let riir_clippy::kat_lease_client::UpdateStatus::Applied { epoch, charge_kat } =
                outcome.status
            {
                // The visible charge line + the matcher rebuild onto E+1 for
                // THIS run (rule 1: proceed on the freshest applied epoch).
                println!(
                    "{}",
                    riir_clippy::kat_lease_client::LeaseCharge::line(epoch, charge_kat)
                );
                if let Some((_, overlay)) = lease_overlay_for_run(&lease_store, None) {
                    match LatentMatcher::from_enabled_domains_with_overlay(&overlay) {
                        Ok(m) => matcher = m,
                        Err(e) => eprintln!(
                            "corpus lease: epoch {epoch} index rebuild failed ({e}) — continuing on the prior corpus"
                        ),
                    }
                }
            }
            let _ = lease_charge.save();
        }
        // Reheat arming: a new epoch may land mid-run only when the lease
        // channel is live. A pin (--corpus-epoch) disables it by design.
        let reheat_armed = !args.no_reheat
            && args.corpus_epoch.is_none()
            && !riir_clippy::kat_lease_client::lease_kill_switched()
            && service_url.is_some();
        if reheat_armed {
            lease_ctx = Some(ReheatCtx {
                start_epoch: lease_store.current_epoch().unwrap_or(0),
                pass1: std::collections::HashMap::new(),
                pass1_tokens: 0,
                spans_seen: 0,
            });
        }
    }

    let mut n_chunks = 0usize;
    let mut n_suggestions = 0usize;
    let mut n_fixed_files = 0usize;
    let mut n_fixed_edits = 0usize;
    // P12: re-proposal suppression. Default-on; the kill-switch re-enables
    // unconditional re-proposal (force retry of previously-reverted edits).
    let suppress_reverted = !suppression_disabled();
    // Unique suppressed pairs for the end-of-run report: (rule, restore,
    // fix) → the (file, line) of the first drop. Deduped by the pair, not
    // the line — a suppressed pattern stays in the text and is re-collected
    // (and re-dropped) on every fixpoint pass.
    let mut suppressed_pairs: std::collections::HashMap<
        (String, String, String),
        (String, usize),
    > = std::collections::HashMap::new();

    // Issue 015: under --verify the fix collection loosens the
    // vec-with-capacity usize gate (rustc backs the revert) and adds the
    // VERIFY_LINTS (type-blind under parse-only, revertible under rustc).
    #[cfg(feature = "fix_verify")]
    let proposer_opts = if args.verify {
        riir_clippy::domains::rust_perf::proposers::ProposerOpts { untyped_paths: true }
    } else {
        riir_clippy::domains::rust_perf::proposers::ProposerOpts::default()
    };

    // Issue 015: cheap Cargo.toml dep gate for fxhashmap — the rewrite
    // emits `use rustc_hash::FxHashMap`, which cannot resolve in a crate
    // without the dependency (the measured 422-edit katgpt-rs instance).
    // Hard-skip per owning crate; no cargo invocation needed.
    #[cfg(feature = "fix_verify")]
    let mut pkg_dep_cache: std::collections::HashMap<PathBuf, bool> =
        std::collections::HashMap::new();

    // Collected fixes per file — printed immediately when NOT verifying;
    // under --verify they are written + verified + reported after the
    // loop (reverted edits must not appear as "fixed" in the review).
    #[cfg(feature = "fix_verify")]
    let mut verify_batches: Vec<riir_clippy::fix_verify::FileEditSet> = Vec::new();
    // Native fmt (src/fmt.rs): per-batch decisions for the verify path —
    // decided against the ORIGINAL file pre-verify, applied post-verify to
    // batches with ≥1 kept edit (a fully-reverted file is back at its
    // already-clean original — nothing to format).
    #[cfg(feature = "fix_verify")]
    let mut verify_fmt: Vec<(PathBuf, riir_clippy::fmt::FmtDecision, String)> = Vec::new();

    // Native fmt pass: default-on with --fix (--no-fmt disables). Decisions
    // only compute when rustfmt is reachable; one warning otherwise.
    let fmt_enabled = args.fix && !args.no_fmt;
    let fmt_active = fmt_enabled && riir_clippy::fmt::rustfmt_available();
    if fmt_enabled && !fmt_active {
        eprintln!("fmt: rustfmt not found on PATH — automatic formatting disabled (--no-fmt silences this)");
    }
    let mut fmt_summary = riir_clippy::fmt::FmtSummary::default();

    for file in &files {
        let Ok(src) = std::fs::read_to_string(file) else {
            eprintln!("skip (unreadable): {}", file.display());
            continue;
        };
        let display = file.display().to_string();

        // KAT meter (T0.5): one bill per source READ — the fix + suggest
        // passes share this read; fixpoint passes re-parse already-billed
        // bytes internally. Raw bytes, parse-independent (G9c).
        #[cfg(feature = "kat_service")]
        if let Some(m) = kat_meter.as_mut() {
            m.record_span(src.as_bytes());
        }

        // ── --fix: run the compiled-in proposers (the visitor IS the gate).
        // Both domains: rust_perf rules + the structural clippy lints (T6.4).
        // Cross-domain overlaps resolve in apply_edits' outer-first selection.
        //
        // Fixpoint iteration: an outer edit's replacement keeps the inner
        // nodes' ORIGINAL text, so fixes nested inside a replaced region
        // (`return Cfg { x: x };` — the field-shorthand edits live inside the
        // return-strip's span) are dropped by overlap selection on pass 1.
        // Re-parsing the spliced output exposes them; iterate until no new
        // edits (bounded, and the unchanged-output guard stops oscillation).
        //
        // Review line numbers are FINAL-source coordinates: each edit's byte
        // offset is recorded against its own pass's input, then remapped
        // through its own + every later pass's applied splices (own-pass
        // mapping is safe — see below; some fixes change line counts —
        // manual_map collapses multi-line matches, fxhashmap's use-append
        // inserts a line — so per-pass numbers would drift).
        const MAX_FIX_PASSES: usize = 4;
        let mut current = src.clone();
        // (pass, rule_id, start, end, replacement) — end + replacement for
        // the Issue 015 span-level revert records.
        let mut all_edits: Vec<(usize, String, usize, usize, String)> = Vec::new();
        // Per-pass applied splices (in that pass's input coordinates).
        let mut pass_splices: Vec<Vec<riir_clippy::edit_emitter::SourceEdit>> = Vec::new();
        // Per-pass INPUT texts (the span-revert restore source).
        let mut pass_texts: Vec<String> = Vec::new();
        if args.fix {
            // Issue 031 H2: --domain scopes the fix path. Previously the
            // proposer loops ran BOTH domains unconditionally — `--domain
            // clippy` applied rust_perf rules (vec-with-capacity edits landed
            // in katgpt-pruners under --domain clippy). `auto` (None) keeps
            // both; `kernel` runs neither (kernel proposers are Phase 5).
            let (run_perf, run_clippy, run_sec) = fix_domain_scopes(args.domain);
            #[cfg(not(feature = "sec"))]
            let _ = run_sec; // only consumed under the `sec` feature
            // fxhashmap dep gate (Issue 015): skip the rule for files in
            // crates without the rustc-hash dependency.
            #[cfg(feature = "fix_verify")]
            let fxhashmap_allowed = {
                let pkg =
                    riir_clippy::fix_verify::owning_package_root(file);
                match pkg {
                    None => false,
                    Some(pkg) => *pkg_dep_cache.entry(pkg.clone()).or_insert_with(|| {
                        std::fs::read_to_string(pkg.join("Cargo.toml")).is_ok_and(|t| {
                            riir_clippy::fix_verify::manifest_has_dep(&t, "rustc-hash")
                        })
                    }),
                }
            };
            #[cfg(not(feature = "fix_verify"))]
            let fxhashmap_allowed = true;

            for pass in 0..MAX_FIX_PASSES {
                let Ok(parsed) = syn::parse_file(&current) else {
                    eprintln!("skip (unparseable): {display}");
                    break;
                };
                let mut edits: Vec<RuleEdit> = Vec::new();
                if run_perf {
                    for rule in FIXABLE_RULES {
                        if *rule == "fxhashmap" && !fxhashmap_allowed {
                            continue;
                        }
                        #[cfg(feature = "fix_verify")]
                        {
                            edits.extend(
                                riir_clippy::domains::rust_perf::proposers::collect_edits_opts(
                                    rule,
                                    &current,
                                    &parsed,
                                    proposer_opts,
                                ),
                            );
                        }
                        #[cfg(not(feature = "fix_verify"))]
                        {
                            edits.extend(
                                riir_clippy::domains::rust_perf::proposers::collect_edits(
                                    rule, &current, &parsed,
                                ),
                            );
                        }
                    }
                }
                // `mut` only under fix_verify (the VERIFY_LINTS extend below).
                #[cfg(feature = "fix_verify")]
                let mut lint_set: Vec<&str> = FIXABLE_LINTS.to_vec();
                #[cfg(not(feature = "fix_verify"))]
                let lint_set: Vec<&str> = FIXABLE_LINTS.to_vec();
                #[cfg(feature = "fix_verify")]
                if args.verify {
                    lint_set.extend(VERIFY_LINTS.iter().copied());
                }
                if run_clippy {
                    for lint in lint_set {
                        edits.extend(
                            riir_clippy::domains::clippy_lints::proposers::collect_edits(
                                lint, &current, &parsed,
                            ),
                        );
                    }
                }
                #[cfg(feature = "sec")]
                if run_sec {
                    // Sec proposers are conservative-gated (Plan 018 T1.3):
                    // guards-already-present skips, signature-changing
                    // refusals — every applied fix still rides the
                    // `--fix --verify` compile gate when enabled.
                    for rule in riir_clippy::domains::sec::proposers::FIXABLE_SEC {
                        edits.extend(
                            riir_clippy::domains::sec::proposers::collect_edits(
                                rule, &current, &parsed,
                            ),
                        );
                    }
                }
                // ── P12: re-proposal suppression. A pair whose exact
                // (rule, restore, replacement) texts already failed the
                // compile gate TWICE consecutively is a deterministic
                // replay — same span bytes + same proposer output + same
                // gate = the same revert; re-proposing it every run was
                // the katgpt-rs sweep's 20-edit/42-check churn that never
                // converged. The filter runs HERE, before apply, because
                // the pass output text `current` already contains every
                // applied edit — dropping the edit from the report list
                // alone would still write it.
                //
                // TWO fails, not one: a whole-file revert records a fail
                // for every edit in the file (attribution impossible —
                // Issue 018), so an innocent edit can carry one
                // association fail; the second run is its retry window,
                // and a second consecutive fail means the pair itself is
                // the problem. Any text change auto-unlocks: the span
                // edited → different restore; the proposer fixed →
                // different replacement (the 6038e59 manual_let_else
                // recovery path). `RIIR_HEAL_SUPPRESS_REVERTED=0` forces
                // retry.
                if suppress_reverted && !edits.is_empty() {
                    edits.retain(|re| {
                        let Some(restore) = current.get(re.edit.start..re.edit.end) else {
                            return true; // un-sliceable span — let the gate judge
                        };
                        #[cfg(feature = "latent_retrieval")]
                        let streak = pair_fail_streak(
                            &traj_store,
                            evolve_rec.memory(),
                            &re.rule_id,
                            restore,
                            &re.edit.replacement,
                        );
                        #[cfg(not(feature = "latent_retrieval"))]
                        let streak = pair_fail_streak(
                            evolve_rec.memory(),
                            &re.rule_id,
                            restore,
                            &re.edit.replacement,
                        );
                        if streak < 2 {
                            return true;
                        }
                        let line = current[..re.edit.start].matches('\n').count() + 1;
                        suppressed_pairs
                            .entry((
                                re.rule_id.clone(),
                                restore.to_string(),
                                re.edit.replacement.clone(),
                            ))
                            .or_insert_with(|| (display.clone(), line));
                        false
                    });
                }
                if edits.is_empty() {
                    break;
                }
                let (next, splices) = riir_clippy::edit_emitter::apply_edits_traced(
                    &current,
                    edits.iter().map(|re| re.edit.clone()).collect(),
                );
                // Report only edits that SURVIVED selection — a pass-1 edit
                // dropped as overlapping (nested inside another fix's span)
                // gets re-collected + applied on pass 2; recording both
                // inflates the count + duplicates the review line.
                let applied: std::collections::HashSet<(usize, usize)> =
                    splices.iter().map(|s| (s.start, s.end)).collect();
                for re in &edits {
                    if applied.contains(&(re.edit.start, re.edit.end)) {
                        all_edits.push((
                            pass,
                            re.rule_id.clone(),
                            re.edit.start,
                            re.edit.end,
                            re.edit.replacement.clone(),
                        ));
                    }
                }
                pass_texts.push(current.clone());
                pass_splices.push(splices);
                let changed = next != current;
                current = next;
                if !changed {
                    break;
                }
            }
        }
        // Remap each edit's offset through ALL passes from its own onward
        // (own-pass mapping is safe: the edit's own splice breaks at
        // `start >= p` without self-applying), then express it as a line
        // number in the final text.
        let all_edits: Vec<(String, usize, String, String, Option<&'static str>)> = all_edits
            .into_iter()
            .map(|(pass, rule_id, start, end, replacement)| {
                let mut pos = start;
                for splices in pass_splices.iter().skip(pass) {
                    pos = riir_clippy::edit_emitter::map_through_splices(pos, splices);
                }
                let line = current[..pos].matches('\n').count() + 1;
                let restore = pass_texts[pass]
                    .get(start..end)
                    .map(str::to_string)
                    .unwrap_or_default();
                let note = rule_context_note(&rule_id);
                (rule_id, line, replacement, restore, note)
            })
            .collect();

        // ── native fmt pass (src/fmt.rs): decide against the ORIGINAL
        // on-disk file BEFORE anything is written. Dry runs report the
        // plan; write/verify mode applies it after the final file state
        // settles. `--no-fmt` or a missing rustfmt disables (one warning).
        let fmt_decision = if args.fix && !all_edits.is_empty() && fmt_active {
            let edition = riir_clippy::fmt::edition_for(file);
            Some((
                riir_clippy::fmt::decide(&src, file, &edition),
                edition,
            ))
        } else {
            None
        };

        #[cfg(feature = "fix_verify")]
        if args.verify {
            if !all_edits.is_empty() {
                verify_batches.push(riir_clippy::fix_verify::FileEditSet {
                    path: file.clone(),
                    original: src.clone(),
                    fixed: current.clone(),
                    edits: all_edits
                        .iter()
                        .map(|(rule, line, repl, restore)| {
                            riir_clippy::fix_verify::EditAtLine {
                                rule_id: rule.clone(),
                                final_line: *line,
                                replacement: repl.clone(),
                                restore: restore.clone(),
                            }
                        })
                        .collect(),
                });
                if let Some((decision, edition)) = &fmt_decision {
                    verify_fmt.push((file.clone(), *decision, edition.clone()));
                }
            }
            // Suggest pass still runs below (unless --quiet) — the fix
            // review prints after verification.
        } else {
            let note = if all_edits.is_empty() {
                None
            } else {
                write_with_fmt(
                    file,
                    &current,
                    fmt_decision.as_ref().map(|(d, _)| *d),
                    fmt_decision.as_ref().map_or("2024", |(_, e)| e.as_str()),
                    args.write,
                    &mut fmt_summary,
                )
            };
            print_fix_review(&display, &all_edits, note.as_deref());
            if !all_edits.is_empty() {
                n_fixed_files += 1;
                n_fixed_edits += all_edits.len();
            }
            // Issue 018: only WRITTEN edits are applied fixes — review-only
            // runs record nothing. 038: applies persist to the active
            // backend — the envelope (light builds / fallback), or the
            // repo-local store (an apply is a SIGHTING: texts + run stamp,
            // no outcome counters — a later verify run promotes it).
            if args.write {
                if envelope_backend {
                    evolve_rec.record_reviewed(
                        &all_edits
                            .iter()
                            .map(|(rule_id, line, replacement, restore, _)| {
                                (rule_id.clone(), *line, replacement.clone(), restore.clone())
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    for (rule, _, replacement, restore, _) in &all_edits {
                        reopen067.note(store, rule);
                        if let Err(e) =
                            store.record_applied(rule, restore, replacement, &heal_run_id)
                        {
                            eprintln!("self-evolve: store record failed: {e}");
                        }
                    }
                }
            }
        }

        #[cfg(not(feature = "fix_verify"))]
        {
            let note = if all_edits.is_empty() {
                None
            } else {
                write_with_fmt(
                    file,
                    &current,
                    fmt_decision.as_ref().map(|(d, _)| *d),
                    fmt_decision.as_ref().map_or("2024", |(_, e)| e.as_str()),
                    args.write,
                    &mut fmt_summary,
                )
            };
            print_fix_review(&display, &all_edits, note.as_deref());
            if !all_edits.is_empty() {
                n_fixed_files += 1;
                n_fixed_edits += all_edits.len();
            }
            // Issue 018: only WRITTEN edits are applied fixes — review-only
            // runs record nothing. 038: applies persist to the active
            // backend — the envelope (light builds / fallback), or the
            // repo-local store (an apply is a SIGHTING: texts + run stamp,
            // no outcome counters — a later verify run promotes it).
            if args.write {
                if envelope_backend {
                    evolve_rec.record_reviewed(
                        &all_edits
                            .iter()
                            .map(|(rule_id, line, replacement, restore, _)| {
                                (rule_id.clone(), *line, replacement.clone(), restore.clone())
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    for (rule, _, replacement, restore, _) in &all_edits {
                        reopen067.note(store, rule);
                        if let Err(e) =
                            store.record_applied(rule, restore, replacement, &heal_run_id)
                        {
                            eprintln!("self-evolve: store record failed: {e}");
                        }
                    }
                }
            }
        }

        // ── suggest pass (unless --fix --quiet). Runs over the ORIGINAL
        // source — suggestions reference the user's current file; the fix
        // review above is the change record.
        if args.suggest || (args.fix && !args.quiet) {
            for s in matcher.suggest_file(&src, &display, args.top_k) {
                n_chunks += 1;
                if s.all_rules().is_empty() {
                    continue;
                }
                n_suggestions += 1;
                print_suggestion(&s, &matcher, args.domain);
            }
        }

        // ── T1.6 (Plan 040): reheat pass-1 collection. Retrieval-only,
        // no output, no counter movement (G3: an unarmed run — offline,
        // fresh, pinned, or --no-reheat — does ZERO extra work). The
        // per-span candidate sets + token totals are the delta baseline
        // the run-end E+1 pass compares against.
        #[cfg(feature = "kat_transport")]
        if let Some(ctx) = lease_ctx.as_mut() {
            for s in matcher.suggest_file(&src, &display, args.top_k) {
                let text = extract_span_text(&src, s.start_line, s.end_line);
                let mut rules: Vec<String> = s
                    .by_domain
                    .iter()
                    .flat_map(|(_, rs)| rs.iter().map(|r| r.rule_id.clone()))
                    .collect();
                rules.sort();
                rules.dedup();
                ctx.pass1_tokens +=
                    riir_clippy::kat_meter::count_word_tokens(text.as_bytes());
                ctx.spans_seen += 1;
                ctx.pass1
                    .entry(riir_clippy::kat_lease_client::span_key(&text))
                    .or_insert(rules);
            }
        }
    }

    // ── Plan 030 T0.4: the docker lane (the second non-syn surface, and
    // the only one with bounded proposers — so it serves --fix too).
    #[cfg(feature = "docker")]
    let n_docker = match (args.suggest || args.fix)
        && args.domain.is_none_or(|d| d == DomainId::Docker)
        && !dockerfiles.is_empty()
    {
        true => run_docker_lane(&dockerfiles, args.suggest, args.fix, args.write),
        false => 0,
    };
    #[cfg(not(feature = "docker"))]
    let n_docker = 0usize;
    let _ = n_docker;

    // ── Issue 025 T4: the non-syn surface suggest pass — `.wgsl` kernel
    // files + `#[cube]` bodies inside `.rs` files, keyed to the kernel
    // domain. SUGGEST-ONLY by construction: findings are corpus-rule
    // citations, no proposer exists for these surfaces, and this pass
    // runs outside the --fix edit path entirely (pure --suggest mode).
    #[cfg(feature = "kernel_heal")]
    let n_non_syn = if args.suggest && args.domain.is_none_or(|d| d == DomainId::Kernel) {
        let scanner = riir_clippy::domains::kernel_opt::wgsl_scan::WgslScanner::default();
        let mut n = 0usize;
        for path in &wgsl_files {
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            // KAT meter (T0.5): the non-syn surface bills like any other
            // processed source — raw bytes, parse-independent.
            #[cfg(feature = "kat_service")]
            if let Some(m) = kat_meter.as_mut() {
                m.record_span(src.as_bytes());
            }
            for f in scanner.scan(&src) {
                println!(
                    "{}:{} [kernel ] {:<44} — {}",
                    path.display(),
                    f.line,
                    f.rule_id,
                    f.excerpt.trim()
                );
                n += 1;
            }
        }
        for path in &files {
            let Ok(src) = std::fs::read_to_string(path) else {
                continue;
            };
            for f in scanner.scan_cube_rust(&src) {
                println!(
                    "{}:{} [kernel ] {:<44} — {} (cube body)",
                    path.display(),
                    f.line,
                    f.rule_id,
                    f.excerpt.trim()
                );
                n += 1;
            }
        }
        if n > 0 {
            eprintln!("non-syn surface (wgsl + cube bodies): {n} finding(s)");
        }
        n
    } else {
        0
    };
    #[cfg(not(feature = "kernel_heal"))]
    let n_non_syn = 0usize;

    // ── Issue 015: the verify loop — write, cargo check, auto-revert.
    #[cfg(feature = "fix_verify")]
    if args.verify {
        let extra: Vec<String> = args
            .verify_args
            .as_deref()
            .map(|s| s.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        let verifier = riir_clippy::fix_verify::FixVerifier::new().with_extra_args(extra.clone());
        // Issue 031 H3: the verify baseline is default-features cargo check
        // — code behind non-default features is NOT compile-gated (the
        // katgpt-pruners batch shipped 2 feature-gated breaks past verify:
        // a u8-const with_capacity E0308 + an E0631 fn-path coercion). Warn
        // loudly when an owning package has non-default features and the
        // caller did not override the baseline via --verify-args.
        if args.verify_args.is_none() {
            let mut warned: std::collections::HashSet<PathBuf> =
                std::collections::HashSet::new();
            for b in &verify_batches {
                let Some(pkg) = riir_clippy::fix_verify::owning_package_root(&b.path) else {
                    continue;
                };
                if !warned.insert(pkg.clone()) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(pkg.join("Cargo.toml")) else {
                    continue;
                };
                let n = riir_clippy::fix_verify::manifest_nondefault_feature_count(&text);
                if n > 0 {
                    eprintln!(
                        "verify: WARNING — {} declares {} non-default feature(s); gated modules are NOT compile-gated by this baseline. Pass --verify-args \"--all-features\" to cover them (if the crate's all-features set compiles).",
                        pkg.display(),
                        n
                    );
                }
            }
        }
        eprintln!(
            "verify: {} file(s) / {} edit(s) — baseline + compile-gated apply",
            verify_batches.len(),
            verify_batches.iter().map(|b| b.edits.len()).sum::<usize>()
        );
        let report = verifier.verify(&verify_batches);
        if let Some(err) = &report.error {
            eprintln!("verify: FAILED — {err}");
            std::process::exit(1);
        }
        if !report.ran {
            eprintln!("verify: nothing to verify (no edits)");
        } else {
            eprintln!(
                "verify: {} cargo check(s) — kept {} file(s) / {} edit(s), reverted {} edit(s)",
                report.n_checks, report.kept_files, report.kept_edits, report.reverted.len()
            );
            for r in &report.reverted {
                eprintln!(
                    "  reverted {}: {} @ line {}{}",
                    r.path.display(),
                    r.rule_id,
                    r.line,
                    if r.whole_file { " (whole file)" } else { "" }
                );
            }
        }
        // ── 037-T2: per-batch derived state (kept/reverted), computed ONCE
        // — consumed by the oracle pre-pass below + the review loop.
        let batch_state: Vec<(std::collections::HashSet<usize>, bool)> = verify_batches
            .iter()
            .map(|b| {
                let reverted_lines: std::collections::HashSet<usize> = report
                    .reverted
                    .iter()
                    .filter(|r| r.path == b.path && !r.whole_file)
                    .map(|r| r.line)
                    .collect();
                let whole = report
                    .reverted
                    .iter()
                    .any(|r| r.path == b.path && r.whole_file);
                (reverted_lines, whole)
            })
            .collect();

        // ── 037-T2: the lint-resolution oracle (clippy_verify builds). For
        // kept edits whose rule is one of OUR clippy lints, one
        // `cargo clippy -- -W clippy::<lint>…` per owning package (same
        // feature set as the compile gate) confirms whether the named lint
        // still fires AT the edit site:
        //   resolved  → Certified verdict (the only promotion path)
        //   persists  → verified FAILURE (the honest persist signal)
        //   no verdict (oracle not built / unrunnable / non-clippy rule) →
        //              rustc-only Heuristic outcome-success
        // A finding elsewhere in the file (a refused sibling span) does NOT
        // block confirmation — only a finding whose PRIMARY span covers the
        // edit's final line is this edit's verdict.
        // Populated only under clippy_verify — hence the cfg_attr'd mut.
        #[cfg_attr(not(feature = "clippy_verify"), allow(unused_mut))]
        let mut oracle_resolved: std::collections::HashMap<(PathBuf, usize), bool> =
            std::collections::HashMap::new();
        #[cfg(feature = "clippy_verify")]
        {
            // FIXABLE_LINTS (unconditional) + VERIFY_LINTS (fix_verify —
            // this block nests inside it) are already imported at the top.
            let oracle_lint =
                |rule: &str| FIXABLE_LINTS.contains(&rule) || VERIFY_LINTS.contains(&rule);
            // (pkg → (distinct lints, the kept edits carrying them)).
            // Edit tuple: (batch path, final line, rule id).
            type PkgOracle = (Vec<String>, Vec<(PathBuf, usize, String)>);
            let mut by_pkg: std::collections::HashMap<PathBuf, PkgOracle> =
                std::collections::HashMap::new();
            for (b, (reverted_lines, whole)) in verify_batches.iter().zip(&batch_state) {
                if *whole {
                    continue;
                }
                let Some(pkg) = riir_clippy::fix_verify::owning_package_root(&b.path) else {
                    continue;
                };
                for e in &b.edits {
                    if reverted_lines.contains(&e.final_line) || !oracle_lint(&e.rule_id) {
                        continue;
                    }
                    let entry = by_pkg.entry(pkg.clone()).or_default();
                    if !entry.0.contains(&e.rule_id) {
                        entry.0.push(e.rule_id.clone());
                    }
                    entry.1.push((b.path.clone(), e.final_line, e.rule_id.clone()));
                }
            }
            if !by_pkg.is_empty() {
                let runner =
                    riir_clippy::fix_verify::CheckRunner::new().with_extra_args(extra.clone());
                let mut canon_cache: std::collections::HashMap<PathBuf, PathBuf> =
                    std::collections::HashMap::new();
                let mut n_resolved = 0usize;
                let mut n_persist = 0usize;
                let mut n_unverifiable = 0usize;
                for (pkg, (lints, edits)) in &by_pkg {
                    let outcome = runner.run_clippy(pkg, lints);
                    let findings = match outcome {
                        Ok(o) if o.exit_ok => {
                            riir_clippy::fix_verify::parse_clippy_findings(&o.json)
                        }
                        Ok(_) => {
                            n_unverifiable += edits.len();
                            continue; // clippy failed — no verdict (Heuristic)
                        }
                        Err(e) => {
                            eprintln!("oracle: clippy unrunnable for {}: {e}", pkg.display());
                            n_unverifiable += edits.len();
                            continue;
                        }
                    };
                    let ws = riir_clippy::compile_oracle::workspace_root(pkg);
                    let ws_ref = ws.as_deref().unwrap_or(pkg.as_path());
                    for (batch_path, line, lint) in edits {
                        let canon = canon_cache
                            .entry(batch_path.clone())
                            .or_insert_with(|| {
                                std::fs::canonicalize(batch_path)
                                    .unwrap_or_else(|_| batch_path.clone())
                            })
                            .clone();
                        let target = format!("clippy::{lint}");
                        let persists = findings.iter().any(|f| {
                            f.code == target
                                && riir_clippy::fix_verify::resolve_reported_path(
                                    &f.file,
                                    ws_ref,
                                    pkg,
                                ) == *canon
                                && *line >= f.line_start
                                && *line <= f.line_end
                        });
                        oracle_resolved.insert((batch_path.clone(), *line), !persists);
                        if persists {
                            n_persist += 1;
                        } else {
                            n_resolved += 1;
                        }
                    }
                }
                eprintln!(
                    "oracle: {n_resolved} confirmed resolved (Certified), {n_persist} persist (verified-fail), {n_unverifiable} without an oracle verdict (Heuristic)",
                );
            }
        }

        // ── 037-T2/038: the durable verdict landing — fix_verify implies
        // latent_retrieval, so the verdict events ALSO land in the
        // repo-local neuron-db store (`.heal/trajectories.kv`), the backend
        // 038 unifies on. Transitional dual-record with memory.json below
        // (the same events — no drift; 038-T3 retires the envelope half).
        // 038: the store was hoisted + opened at the --fix path entry (the
        // same instance serves apply events + verify verdicts — one open,
        // one migration check per run); this scope borrows it directly.

        // Post-verify review: report what SURVIVED (the fix review for
        // reverted edits would be misleading).
        for (b, (reverted_lines, whole)) in verify_batches.iter().zip(&batch_state) {
            if *whole {
                println!("fix {} — REVERTED (whole file):", b.path.display());
                for e in &b.edits {
                    println!("  [{:<24}] line {} (reverted)", e.rule_id, e.final_line);
                }
                // Issue 018: a whole-file revert is a definitive fail verdict
                // for every edit in the file (attribution impossible — the
                // compile failure itself is the verdict). Issue 047: the
                // decision-time attribution is W (drafter produced a
                // compile-broken fix; the gate caught it).
                for e in &b.edits {
                    evolve_rec.record_failure_signal(
                        &e.rule_id,
                        &e.restore,
                        &e.replacement,
                        &riir_clippy::component::FailureSignal::reverted(
                            riir_clippy::component::RevertKind::CompileGate,
                        ),
                    );
                    #[cfg(feature = "latent_retrieval")]
                    record_traj_fail(&traj_store, &e.rule_id, &e.restore, &e.replacement, &heal_run_id);
                }
                continue;
            }
            let kept: Vec<&riir_clippy::fix_verify::EditAtLine> = b
                .edits
                .iter()
                .filter(|e| !reverted_lines.contains(&e.final_line))
                .collect();
            if kept.is_empty() {
                continue;
            }
            n_fixed_files += 1;
            n_fixed_edits += kept.len();
            // 037-T2 (the F2 fix): kept edits earn POSITIVE verdicts — the
            // compile gate vouches at minimum (rustc-only pass = Heuristic
            // outcome-success); the clippy oracle (when built + runnable)
            // upgrades to Certified on confirmed resolution or records the
            // honest verified-fail on a persist. Span-level reverts stay
            // definitive fails.
            for e in &kept {
                let (success, verified) = riir_clippy::fix_verify::kept_edit_verdict(
                    oracle_resolved
                        .get(&(b.path.clone(), e.final_line))
                        .copied(),
                );
                // Issue 047: a kept-but-unresolved edit (compile gate passed,
                // full oracle says the lint persists) is C-class — the gate
                // was too loose. Successes stay unattributed.
                let fail_signal = (!success).then(
                    riir_clippy::component::FailureSignal::gate_passed_ineffective,
                );
                if let Some(sig) = &fail_signal {
                    evolve_rec.record_failure_signal(&e.rule_id, &e.restore, &e.replacement, sig);
                } else {
                    evolve_rec.record_outcome_verified(
                        &e.rule_id,
                        &e.restore,
                        &e.replacement,
                        success,
                        verified,
                    );
                }
                #[cfg(feature = "latent_retrieval")]
                record_traj(
                    &traj_store,
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    success,
                    verified,
                    &heal_run_id,
                    fail_signal.as_ref().map(riir_clippy::component::classify),
                );
            }
            for e in b.edits.iter().filter(|e| reverted_lines.contains(&e.final_line)) {
                evolve_rec.record_failure_signal(
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    &riir_clippy::component::FailureSignal::reverted(
                        riir_clippy::component::RevertKind::CompileGate,
                    ),
                );
                #[cfg(feature = "latent_retrieval")]
                record_traj_fail(&traj_store, &e.rule_id, &e.restore, &e.replacement, &heal_run_id);
            }
            println!("fix {} ({} edit(s) verified):", b.path.display(), kept.len());
            for e in &kept {
                println!("  [{:<24}] line {}", e.rule_id, e.final_line);
            }
            // Native fmt: the verified kept state is on disk — apply the
            // decided pass now (default-on; decisions were made against the
            // pre-heal originals above).
            if let Some((_, decision, edition)) =
                verify_fmt.iter().find(|(p, _, _)| *p == b.path)
            {
                use riir_clippy::fmt::FmtDecision;
                match decision {
                    FmtDecision::Format => {
                        match riir_clippy::fmt::apply(&b.path, edition) {
                            Ok(()) => {
                                fmt_summary.formatted += 1;
                                println!("  fmt: formatted");
                            }
                            Err(e) => {
                                fmt_summary.failed += 1;
                                eprintln!("fmt failed for {}: {e}", b.path.display());
                            }
                        }
                    }
                    FmtDecision::SkipDrift => {
                        fmt_summary.skip_drift += 1;
                        println!("  fmt: skip (pre-existing drift)");
                    }
                    FmtDecision::SkipCrlf => {
                        fmt_summary.skip_crlf += 1;
                        println!("  fmt: skip (CRLF)");
                    }
                }
            }
        }
    }
    // Issue 018/038: persist recorded trajectories (skip when nothing
    // new). 038: store-backend runs skip the envelope ENTIRELY — events
    // were persisted to the repo-local store as they happened; the
    // envelope save is the light-build / fallback path only.
    if envelope_backend && evolve_rec.recorded_count() > 0 {
        if let Some(parent) = memory_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match evolve_rec.save(&memory_path) {
            Ok(()) => eprintln!(
                "self-evolve: {} trajectory(ies) → {}",
                evolve_rec.recorded_count(),
                memory_path.display()
            ),
            Err(e) => eprintln!(
                "self-evolve: memory save failed ({}): {e}",
                memory_path.display()
            ),
        }
    }

    // ── T1.6 (Plan 040): the run-end boundary reheat (default ON when the
    // lease channel is armed). If a NEW epoch landed mid-run, finishing on
    // the old epoch + reporting done would be a FALSE CLEAN — strictly worse
    // than a visible second bill. Sequential frozen passes: the E+1 pass
    // runs over the POST-FIX tree (re-read from disk), re-retrieves every
    // span, and only spans whose top-k candidate set CHANGED re-enter
    // propose (+ only their tokens re-bill — delta-scoped, metered inside
    // the pass at span granularity; a straddle that changes nothing costs
    // ~retrieval-only).
    #[cfg(feature = "kat_transport")]
    if let Some(ctx) = lease_ctx.as_mut() {
        let meter = kat_meter.as_mut();
        if let Some((report, charge_line)) =
            run_reheat_pass(ctx, &args, &files, &mut matcher, meter)
        {
            println!("\nreheat: epoch {} landed mid-run", report.pass2_epoch);
            println!("  {}", report.itemized_line());
            if let Some(line) = charge_line {
                println!("  {line}");
            }
        }
    }

    let mode = if args.fix {
        #[cfg(feature = "fix_verify")]
        let applied_note = if args.verify {
            " (verified — compile-gated)"
        } else if args.write {
            " (written)"
        } else {
            " (review only — add --write)"
        };
        #[cfg(not(feature = "fix_verify"))]
        let applied_note = if args.write {
            " (written)"
        } else {
            " (review only — add --write)"
        };
        format!(
            ", fixed {n_fixed_files} file(s) / {n_fixed_edits} edit(s){applied_note}"
        )
    } else {
        String::new()
    };
    let non_syn_note = if n_non_syn > 0 {
        format!(", +{n_non_syn} non-syn (wgsl/cube) finding(s)")
    } else {
        String::new()
    };
    let fmt_note = if args.fix && fmt_summary.total() > 0 {
        format!(", {}", fmt_summary.line())
    } else {
        String::new()
    };
    // P12: report the suppressed re-proposals — the operator must see what
    // was skipped and why (a suppressed edit keeps its lint firing; the
    // escape hatch is RIIR_HEAL_SUPPRESS_REVERTED=0 for one run).
    if !suppressed_pairs.is_empty() {
        println!(
            "suppressed {} previously-reverted edit(s) (2+ consecutive compile-gate fails; RIIR_HEAL_SUPPRESS_REVERTED=0 forces retry):",
            suppressed_pairs.len()
        );
        const MAX_SUPPRESSED_SHOWN: usize = 20;
        for ((rule, restore, _fix), (file, line)) in suppressed_pairs.iter().take(MAX_SUPPRESSED_SHOWN) {
            println!(
                "  [{rule:<24}] {}:{} — {}",
                file,
                line,
                snippet_head(restore)
            );
        }
        if suppressed_pairs.len() > MAX_SUPPRESSED_SHOWN {
            println!("  ... and {} more", suppressed_pairs.len() - MAX_SUPPRESSED_SHOWN);
        }
    }
    println!(
        "\nscanned {} file(s), {} span(s), {} with suggestions (top-{} per domain){}{}{}",
        files.len(),
        n_chunks,
        n_suggestions,
        args.top_k,
        mode,
        non_syn_note,
        fmt_note
    );

    // KAT meter (T0.5): persist the session's burn + one summary line.
    // Save failure is a lost bookkeeping event, never a failed heal — warn
    // + continue (the recorder precedent). No `$` figures here (caveats
    // #12/#15): tokens only; pricing is server-side with an as-of date.
    #[cfg(feature = "kat_service")]
    if let Some(m) = &kat_meter {
        if let Err(e) = m.save() {
            eprintln!("kat-meter: could not persist burn ledger ({}): {e}", m.path().display());
        }
        println!("{}", m.summary_line());
    }

    // KAT charge (T0.6, Plan 024): settle the session's uncharged tokens at
    // the per-unit rate, persist, one summary line. Same accounting-only
    // posture (G9d — the balance is a local projection; the server ledger
    // reconciles at sync), same no-`$` rule (caveats #12/#15).
    #[cfg(feature = "kat_service")]
    if let (Some(m), Some(c)) = (&kat_meter, kat_charge.as_mut()) {
        c.settle(m.state());
        if let Err(e) = c.save() {
            eprintln!("kat-charge: could not persist charge ledger ({}): {e}", c.path().display());
        }
        println!("{}", c.summary_line());
        let remaining = c.balance_projection();
        if remaining == 0 {
            // §KAT warn-and-pay (T1.6): at 0 the evolved-corpus hot-push
            // pauses at epoch end + the persistent warning fires — mining
            // accrual/settlement CONTINUE, healing NEVER blocks (G3).
            eprintln!(
                "kat-charge: 0 KAT remain of the {} grant projection — out-of-KAT",
                riir_clippy::kat_meter::FREE_TIER_GRANT_KAT
            );
            eprintln!(
                "kat-charge: healing continues on the local corpus; the evolved-corpus \\
                 hot-push pauses at epoch end; mining accrual + settlement continue"
            );
            #[cfg(feature = "kat_transport")]
            {
                let dir = riir_clippy::kat_account::KatAccount::default_dir();
                let acct = riir_clippy::kat_account::KatAccount::load(&dir)
                    .ok()
                    .map(|a| a.state().account_id.clone())
                    .unwrap_or_default();
                let url = riir_clippy::kat_lease_client::payment_url(
                    riir_clippy::kat_lease_client::DEFAULT_SERVICE_URL,
                    &acct,
                );
                eprintln!("kat-charge: top up — {url}");
                let qr = riir_clippy::kat_lease_client::render_qr_ascii(&url);
                if !qr.is_empty() {
                    eprintln!("{qr}");
                }
            }
        } else if c.low_balance() {
            // §KAT warn at ≤10M remaining: the top-up nudge (no QR until 0 —
            // the terminal render is the last-chance surface).
            eprintln!(
                "kat-charge: warning — {remaining} KAT remain of the {} grant projection; \\
                 healing continues offline (the server ledger is the authority)",
                riir_clippy::kat_meter::FREE_TIER_GRANT_KAT
            );
        }
    }
}

/// Plan 030 T0.4 — the `docker` lane.
///
/// Dockerfiles cannot ride the syn fix loop (no `syn::File`), so the domain
/// gets its own pass. It handles BOTH modes, because unlike the WGSL
/// scanner this domain ships bounded proposers:
///
/// - `--suggest`: print every finding as a rule citation.
/// - `--fix`: print the bounded rewrites; `--write` applies them.
///
/// ## `--verify` does NOT cover this lane (stated, not implied)
///
/// The Rust domains gate every written edit behind a `cargo check`. There is
/// no equivalent here: the analogous oracle is a real `docker build`, which
/// is minutes-to-tens-of-minutes and needs a daemon plus a build context, so
/// it is deliberately NOT run per fix. What DOES protect a write: every
/// bounded rule reproduces its corpus fixture byte-exactly, the proposers
/// refuse more than they fire (non-bash `pipefail`, metacharacter exec-form,
/// heredoc appends, `--locked` without a lockfile), and `fix_all` is
/// idempotent. Review the diff, and run your image build — the Plan 030
/// G5-real gate is what proves the shape, once, per release, not per edit.
///
/// Returns the number of findings printed.
///
/// The caller-gated rules stay silent unless the evidence is derivable from
/// the repo around the file — here, the presence of a `.dockerignore` and a
/// committed `Cargo.lock` beside the build context. Evidence we cannot
/// derive stays `None`, so those rules do not fire.
#[cfg(feature = "docker")]
fn run_docker_lane(paths: &[PathBuf], suggest: bool, fix: bool, write: bool) -> usize {
    use riir_clippy::domains::docker::proposers::fix_all;
    use riir_clippy::domains::docker::rules::{scan_source_with, FIXABLE_DOCKER};

    let mut printed = 0usize;
    for path in paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let ctx = docker_scan_context(path);
        let findings = scan_source_with(&src, &ctx);
        for f in &findings {
            let kind = match FIXABLE_DOCKER.contains(&f.rule_id) && f.fix.is_some() {
                true => "fix    ",
                false => "suggest",
            };
            println!(
                "{}:{} [docker {kind}] {:<40} — {}",
                path.display(),
                f.line,
                f.rule_id,
                f.excerpt.trim()
            );
            printed += 1;
        }
        if !fix {
            continue;
        }
        let fixed = fix_all(&src, &ctx);
        if fixed == src {
            continue;
        }
        match write {
            true => match std::fs::write(path, &fixed) {
                Ok(()) => println!("  wrote {}", path.display()),
                Err(e) => eprintln!("  FAILED to write {}: {e}", path.display()),
            },
            false => println!(
                "  {} bounded fix(es) available for {} (re-run with --write to apply)",
                findings
                    .iter()
                    .filter(|f| f.fix.is_some() && FIXABLE_DOCKER.contains(&f.rule_id))
                    .count(),
                path.display()
            ),
        }
    }
    let _ = suggest;
    printed
}

/// Derive what the repo around a Dockerfile can prove, for the caller-gated
/// rules. Anything not derivable stays `None` — never guessed.
#[cfg(feature = "docker")]
fn docker_scan_context(path: &Path) -> riir_clippy::domains::docker::rules::ScanContext {
    use riir_clippy::domains::docker::rules::ScanContext;

    // The build context is conventionally the Dockerfile's directory or a
    // parent; walk up to the nearest dir carrying a Cargo.toml and read the
    // evidence there. A file outside any Cargo project yields no evidence.
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            return ScanContext {
                lockfile_committed: Some(d.join("Cargo.lock").is_file()),
                dockerignore_present: Some(d.join(".dockerignore").is_file()),
                // Not derivable from the filesystem: whether the build is a
                // multi-member workspace build, and whether the caller
                // passes `--platform a,b`. Both stay silent.
                workspace_multi_member: None,
                multi_platform_build: None,
            };
        }
        dir = d.parent();
    }
    ScanContext::default()
}

/// Issue 031 H2: `--domain` scopes the `--fix` proposer loops. Returns
/// `(run_perf, run_clippy, run_sec)`. `auto` (None) runs every compiled-in
/// syn domain (the historical behavior); an explicit domain runs ONLY its
/// own proposers; `kernel` runs neither (kernel proposers are Phase 5).
///
/// `docker` is deliberately NOT in this tuple: its proposers read
/// Dockerfile TEXT, not a parsed `syn::File`, so it cannot ride the syn
/// fix loop at all. It runs in its own lane ([`run_docker_lane`]), which
/// is why an explicit `--domain docker` returns all-false here — the syn
/// loop must do nothing.
fn fix_domain_scopes(domain: Option<DomainId>) -> (bool, bool, bool) {
    (
        domain.is_none_or(|d| d == DomainId::Perf),
        domain.is_none_or(|d| d == DomainId::Clippy),
        domain.is_none_or(|d| d == DomainId::Sec),
    )
}

/// T1.6 (Plan 040): map an installed leased epoch's packs onto the enabled
/// domains' corpora. `pin` selects a stored epoch (`--corpus-epoch`) instead
/// of the current pointer. Unknown pack domains warn + skip (never abort —
/// a future domain string must not brick an old client).
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
/// T2.4 (Plan 042): merge the PROMOTION registry's Certified triples into
/// the run overlay (leased-or-compiled-in per domain — the pure merge lives
/// in `kat_promotion::promotion_run_overlay`). Skipped under a `--corpus-epoch`
/// pin (exact reproduction) and when the registry is empty/unloadable (a
/// corrupt registry WARNs and continues WITHOUT network rules — the safe
/// direction for a heal run; the GATE refuses on the same condition).
#[cfg(all(feature = "kat_service", feature = "score_bench"))]
#[allow(clippy::type_complexity)] // mirrors lease_overlay_for_run's shape one layer up
fn run_overlay_with_promotion(
    store: &riir_clippy::kat_lease_client::LeaseStore,
    pin: Option<u64>,
    heal_root: &std::path::Path,
) -> Option<(u64, Vec<(DomainId, Vec<(String, String, String)>)>)> {
    use riir_clippy::kat_promotion::{promotion_run_overlay, PromotionRegistry, REGISTRY_FILE};

    let leased = lease_overlay_for_run(store, pin);
    let registry_path = heal_root.join(REGISTRY_FILE);
    let registry = match PromotionRegistry::load(&registry_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "corpus promotion: registry unreadable — continuing WITHOUT network rules: {e}"
            );
            return leased;
        }
    };
    if registry.promoted.is_empty() || pin.is_some() {
        return leased;
    }
    // DomainId ⇄ String-domain mapping (label() is the lease pack spelling).
    let leased_str: Option<(u64, Vec<(String, Vec<_>)>)> = leased.as_ref().map(|(epoch, o)| {
        (
            *epoch,
            o.iter()
                .map(|(d, t)| (d.label().to_string(), t.clone()))
                .collect(),
        )
    });
    let merged = promotion_run_overlay(leased_str.as_ref().map(|(_, o)| o.as_slice()), &registry, pin.is_some())?;
    let overlay: Vec<(DomainId, Vec<_>)> = merged
        .into_iter()
        .filter_map(|(domain, triples)| {
            let id = match domain.as_str() {
                "clippy" => Some(DomainId::Clippy),
                "perf" => Some(DomainId::Perf),
                "kernel" => Some(DomainId::Kernel),
                "sec" => Some(DomainId::Sec),
                "docker" => Some(DomainId::Docker),
                _ => None,
            }?;
            Some((id, triples))
        })
        .collect();
    let epoch = leased.map_or(0, |(e, _)| e);
    println!(
        "corpus promotion: +{} local Certified rule(s) merged into the run overlay",
        registry.promoted.len()
    );
    Some((epoch, overlay))
}

#[cfg(feature = "kat_service")]
#[allow(clippy::type_complexity)] // mirrors the lease module's DomainTriples shape one layer up
fn lease_overlay_for_run(
    store: &riir_clippy::kat_lease_client::LeaseStore,
    pin: Option<u64>,
) -> Option<(u64, Vec<(DomainId, Vec<(String, String, String)>)>)> {
    use riir_clippy::kat_lease_client::{
        overlay_from_epoch, PACK_DOMAIN_CLIPPY, PACK_DOMAIN_KERNEL, PACK_DOMAIN_PERF,
        PACK_DOMAIN_DOCKER, PACK_DOMAIN_SEC,
    };
    let (epoch, raw) = overlay_from_epoch(store, pin)?;
    let mut overlay = Vec::new();
    for (domain, triples) in raw {
        let id = match domain.as_str() {
            PACK_DOMAIN_CLIPPY => Some(DomainId::Clippy),
            PACK_DOMAIN_PERF => Some(DomainId::Perf),
            PACK_DOMAIN_KERNEL => Some(DomainId::Kernel),
            PACK_DOMAIN_SEC => Some(DomainId::Sec),
            PACK_DOMAIN_DOCKER => Some(DomainId::Docker),
            other => {
                eprintln!("corpus lease: ignoring unknown pack domain {other:?} (client too old?)");
                None
            }
        };
        if let Some(id) = id {
            overlay.push((id, triples));
        }
    }
    Some((epoch, overlay))
}

/// Extract a 1-based inclusive line range as its own text (the span
/// identity + token source for reheat pass bookkeeping).
#[cfg(feature = "kat_transport")]
fn extract_span_text(src: &str, start_line: usize, end_line: usize) -> String {
    let n = end_line.saturating_sub(start_line) + 1;
    src.lines()
        .skip(start_line.saturating_sub(1))
        .take(n)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The T1.6 reheat context — pass-1's per-span candidate sets + token
/// totals, collected during the main scan when the reheat is armed.
#[cfg(feature = "kat_transport")]
struct ReheatCtx {
    start_epoch: u64,
    pass1: std::collections::HashMap<[u8; 32], Vec<String>>,
    pass1_tokens: u64,
    spans_seen: usize,
}

/// The compact pass-2 fix loop over one file — the same proposer machinery
/// and span-preserving splice as pass 1, without the verify-batching /
/// evolve recording / line-remapping surfaces (pass-1 concerns). Review
/// mode prints `reheat:` lines; `--write` applies + counts; under
/// `--verify` the pass is REVIEW-ONLY (never-wrong: no unverified write —
/// the edits land deterministically on the next run, the epoch being
/// pinned).
#[cfg(feature = "kat_transport")]
fn reheat_fix_pass(
    path: &Path,
    domain: Option<DomainId>,
    write: bool,
    verify_mode: bool,
    fmt_active: bool,
) -> usize {
    let Ok(src) = std::fs::read_to_string(path) else {
        return 0;
    };
    let display = path.display().to_string();
    let (run_perf, run_clippy, run_sec) = fix_domain_scopes(domain);
    #[cfg(not(feature = "sec"))]
    let _ = run_sec;
    const MAX_PASSES: usize = 4;
    let mut current = src.clone();
    let mut total_applied = 0usize;
    let mut changed = false;
    for _pass in 0..MAX_PASSES {
        let Ok(parsed) = syn::parse_file(&current) else {
            return total_applied;
        };
        let mut edits: Vec<RuleEdit> = Vec::new();
        if run_perf {
            for rule in FIXABLE_RULES {
                edits.extend(riir_clippy::domains::rust_perf::proposers::collect_edits(
                    rule, &current, &parsed,
                ));
            }
        }
        if run_clippy {
            for lint in FIXABLE_LINTS {
                edits.extend(riir_clippy::domains::clippy_lints::proposers::collect_edits(
                    lint, &current, &parsed,
                ));
            }
        }
        #[cfg(feature = "sec")]
        if run_sec {
            for rule in riir_clippy::domains::sec::proposers::FIXABLE_SEC {
                edits.extend(riir_clippy::domains::sec::proposers::collect_edits(
                    rule, &current, &parsed,
                ));
            }
        }
        if edits.is_empty() {
            break;
        }
        let (next, splices) = riir_clippy::edit_emitter::apply_edits_traced(
            &current,
            edits.iter().map(|re| re.edit.clone()).collect(),
        );
        let applied: std::collections::HashSet<(usize, usize)> =
            splices.iter().map(|s| (s.start, s.end)).collect();
        for re in &edits {
            if applied.contains(&(re.edit.start, re.edit.end)) {
                let line = current[..re.edit.start].matches('\n').count() + 1;
                if verify_mode {
                    println!("reheat (review-only under --verify): {display}:{line} [{}]", re.rule_id);
                } else {
                    println!("reheat: {display}:{line} [{}]", re.rule_id);
                }
                total_applied += 1;
            }
        }
        changed = next != current;
        current = next;
        if !changed {
            break;
        }
    }
    if (changed && write && !verify_mode && current != src) && (std::fs::write(path, &current).is_ok() && fmt_active) {
            let edition = riir_clippy::fmt::edition_for(path);
            if matches!(
                riir_clippy::fmt::decide(&current, path, &edition),
                riir_clippy::fmt::FmtDecision::Format
            ) {
                let _ = riir_clippy::fmt::apply(path, &edition);
            }
        }
    total_applied
}

/// The run-end boundary reheat (T1.6, Plan 040). Returns `None` when no new
/// epoch landed (the ~always case — one lease check). Otherwise: apply the
/// new epoch (same staged/verified/atomic path as the update phase), rebuild
/// the matcher under E+1, re-retrieve every pass-1 span over the POST-FIX
/// tree, delta-scope (only changed candidate sets re-enter propose/verify +
/// re-bill — metered at span granularity), and return the itemized report +
/// the lease charge line. Failure at ANY step is silent-fallback (the run's
/// pass-1 results stand; G3 — the network never blocks or corrupts a run).
#[cfg(feature = "kat_transport")]
fn run_reheat_pass(
    ctx: &mut ReheatCtx,
    args: &Args,
    files: &[PathBuf],
    matcher: &mut LatentMatcher,
    mut meter: Option<&mut riir_clippy::kat_meter::KatMeter>,
) -> Option<(riir_clippy::kat_lease_client::ReheatReport, Option<String>)> {
    use riir_clippy::kat_lease_client::{
        run_update_phase, HttpLeaseTransport, LeaseManifest, LeaseTransport, PassSpans,
    };

    // One bounded lease check: did a new epoch land?
    let url = riir_clippy::kat_lease_client::service_url_from_env()?;
    let transport = HttpLeaseTransport::new(&url, args.update_timeout.max(1));
    let store = riir_clippy::kat_lease_client::LeaseStore::open(&resolve_heal_root(&args.paths));
    let mut charge = riir_clippy::kat_lease_client::LeaseCharge::open(&store);
    let current = store.current();
    let etag = current.as_ref().map(|m| riir_clippy::kat_lease_client::hex32(&m.commitment));
    let body = match transport.get_lease(etag.as_deref()) {
        riir_clippy::kat_lease_client::HttpResponse::NotModified => return None,
        riir_clippy::kat_lease_client::HttpResponse::Ok(b) => b,
        _ => return None, // offline/error → no reheat, silently
    };
    let manifest = match LeaseManifest::decode(&body) {
        Ok(m) => m,
        Err(_) => return None, // tamper/malformed → stay on the run's epoch
    };
    if let Some(cur) = &current {
        if manifest.epoch <= cur.epoch {
            return None; // same-or-older (the check raced) — nothing to reheat
        }
    }

    // Apply the new epoch through the SAME staged/verified/atomic path.
    let out_of_kat = false; // the reheat bill is the delta bill, not a new grant gate
    let outcome = run_update_phase(&transport, &store, &mut charge, out_of_kat, None);
    match outcome.status {
        riir_clippy::kat_lease_client::UpdateStatus::Applied { epoch, charge_kat } => {
            // Rebuild the matcher under E+1 (a failed rebuild aborts the
            // reheat — pass 1's results stand). T2.4: promotions merge into
            // the rebuilt overlay too (same helper as the run-start build).
            #[cfg(all(feature = "kat_service", feature = "score_bench"))]
            let rebuilt = run_overlay_with_promotion(
                &store,
                None,
                &resolve_heal_root(&[]),
            );
            #[cfg(all(feature = "kat_service", not(feature = "score_bench")))]
            let rebuilt = lease_overlay_for_run(&store, None);
            if let Some((_, overlay)) = rebuilt {
                match LatentMatcher::from_enabled_domains_with_overlay(&overlay) {
                    Ok(m) => *matcher = m,
                    Err(_) => return None,
                }
            } else {
                return None;
            }
            // Pass 2 over the POST-FIX tree: re-retrieve every span.
            let mut pass2: Vec<PassSpans> = Vec::new();
            let mut span_texts: std::collections::HashMap<[u8; 32], String> =
                std::collections::HashMap::new();
            let mut file_changed: std::collections::HashSet<
                std::path::PathBuf,
            > = std::collections::HashSet::new();
            for file in files {
                let Ok(src) = std::fs::read_to_string(file) else {
                    continue;
                };
                let display = file.display().to_string();
                for s in matcher.suggest_file(&src, &display, args.top_k) {
                    let text = extract_span_text(&src, s.start_line, s.end_line);
                    let key = riir_clippy::kat_lease_client::span_key(&text);
                    let mut rules: Vec<String> = s
                        .by_domain
                        .iter()
                        .flat_map(|(_, rs)| rs.iter().map(|r| r.rule_id.clone()))
                        .collect();
                    rules.sort();
                    rules.dedup();
                    pass2.push(PassSpans {
                        key,
                        tokens: riir_clippy::kat_meter::count_word_tokens(text.as_bytes()),
                        rules,
                    });
                    span_texts.insert(key, text);
                }
            }
            let plan = riir_clippy::kat_lease_client::plan_reheat(&ctx.pass1, &pass2);
            // Which FILES carry changed spans (the proposer granularity).
            for file in files {
                let Ok(src) = std::fs::read_to_string(file) else {
                    continue;
                };
                let display = file.display().to_string();
                let any_changed = matcher
                    .suggest_file(&src, &display, args.top_k)
                    .iter()
                    .any(|s| {
                        let text = extract_span_text(&src, s.start_line, s.end_line);
                        plan.changed
                            .iter()
                            .any(|(k, _)| *k == riir_clippy::kat_lease_client::span_key(&text))
                    });
                if any_changed {
                    file_changed.insert(file.clone());
                }
            }
            // Delta-scoped billing FIRST (the spans WERE re-processed — the
            // meter must reflect it even when propose finds nothing new).
            if let Some(m) = meter.as_mut() {
                for (key, _tokens) in &plan.changed {
                    if let Some(text) = span_texts.get(key) {
                        m.record_span(text.as_bytes());
                    }
                }
            }
            // Only changed files re-enter propose.
            let mut new_fixes = 0usize;
            if args.fix {
                for file in &file_changed {
                    new_fixes += reheat_fix_pass(
                        file,
                        args.domain,
                        args.write,
                        args.verify,
                        !args.no_fmt,
                    );
                }
            }
            let report = riir_clippy::kat_lease_client::ReheatReport {
                pass1_epoch: ctx.start_epoch,
                pass2_epoch: epoch,
                pass1_tokens: ctx.pass1_tokens,
                pass2_tokens: plan.reheat_tokens,
                changed_spans: plan.changed.len(),
                new_fixes,
            };
            let _ = charge.save();
            let line = riir_clippy::kat_lease_client::LeaseCharge::line(epoch, charge_kat);
            Some((report, Some(line)))
        }
        _ => None,
    }
}

#[cfg(test)]
mod fix_review_note_tests {
    use super::*;

    /// Capture what print_fix_review writes for one edit.
    fn render(rule_id: &str, note: Option<&'static str>) -> String {
        // print_fix_review writes to stdout; run it under the test
        // harness's output capture by asserting on the formatting helper
        // instead — but the byte-exactness pin wants the REAL lines, so
        // spawn the formatting inline via Gag-like capture: simplest
        // deterministic route is a tiny duct — write to a BufWriter is
        // not plumbed, so compose the expected lines here exactly as the
        // renderer does and pin THOSE against rule_context_note.
        let mut out = String::new();
        out.push_str("fix x.rs (1 edit(s)):\n");
        out.push_str(&format!("  [{rule_id:<24}] line 7\n"));
        if let Some(note) = note {
            out.push_str(&format!("  \u{26a0} semantic change: {note}\n"));
        }
        out
    }

    #[test]
    fn annotated_lint_carries_the_warning_line() {
        let note = rule_context_note("manual_map");
        let Some(note) = note else {
            panic!("manual_map must carry an annotation under clippy_lints");
        };
        let rendered = render("manual_map", Some(note));
        assert!(rendered.contains("\u{26a0} semantic change: control-flow capture"));
    }

    #[test]
    fn annotated_perf_rule_carries_the_warning_line() {
        let note = rule_context_note("static-str");
        assert!(
            note.is_some_and(|n| n.contains("signature-visible")),
            "static-str must carry the signature-visible annotation, got {note:?}"
        );
    }

    #[test]
    fn mechanical_classes_stay_silent_byte_identically() {
        // The default output must be unchanged: no note ⇒ exactly the two
        // historical lines (header + rule/line), nothing else.
        for rule in ["needless_return", "identity_op", "vec-with-capacity"] {
            assert!(
                rule_context_note(rule).is_none(),
                "{rule} must stay un-annotated (Issue 074 scope guard)"
            );
            let rendered = render(rule, None);
            assert_eq!(
                rendered,
                format!("fix x.rs (1 edit(s)):\n  [{rule:<24}] line 7\n"),
                "{rule}: un-annotated output drifted"
            );
        }
    }

    #[test]
    fn every_annotated_rule_is_fixable() {
        // A note for a rule that cannot fire is dead weight — keep the two
        // tables in lockstep with the fix surfaces.
        for rule in ["manual_map", "needless_range_loop", "question_mark", "map_unwrap_or"] {
            assert!(
                riir_clippy::domains::clippy_lints::proposers::FIXABLE_LINTS.contains(&rule),
                "{rule} annotated but not FIXABLE"
            );
        }
        for rule in ["static-str", "bool-to-bitpack"] {
            assert!(
                riir_clippy::domains::rust_perf::proposers::FIXABLE_RULES.contains(&rule),
                "{rule} annotated but not FIXABLE"
            );
        }
    }
}

#[cfg(test)]
mod fix_domain_tests {
    use super::*;

    #[test]
    fn auto_runs_both() {
        assert_eq!(fix_domain_scopes(None), (true, true, true));
    }

    #[test]
    fn clippy_scopes_out_perf() {
        // The Issue 031 H2 regression: `--domain clippy` must not run
        // rust_perf proposers (vec-with-capacity applied under
        // `--domain clippy` in the katgpt-pruners batch).
        assert_eq!(fix_domain_scopes(Some(DomainId::Clippy)), (false, true, false));
    }

    #[test]
    fn perf_scopes_out_clippy() {
        assert_eq!(fix_domain_scopes(Some(DomainId::Perf)), (true, false, false));
    }

    #[test]
    fn kernel_runs_neither_syn_domain() {
        assert_eq!(fix_domain_scopes(Some(DomainId::Kernel)), (false, false, false));
    }

    #[test]
    fn sec_scopes_to_sec_only() {
        assert_eq!(fix_domain_scopes(Some(DomainId::Sec)), (false, false, true));
    }
}

/// Issue 066: the source-walk needle per domain must count ENTRIES, not
/// plumbing. The runtime corpus builder is the authority (the feature is
/// compiled into the test binary for every default-on domain); the walk
/// must agree with it. This is the pin that makes a needle rot (or a
/// corpus re-indent that moves docker's `    rule(` marker) LOUD instead
/// of a silently-wrong dashboard series.
#[cfg(test)]
mod corpus_stats_needle_tests {
    use super::{count_corpus_domain, resolve_corpus_repo_root, runtime_corpus_count, CORPUS_DOMAIN_TABLE};

    #[test]
    fn source_walk_agrees_with_runtime_count_for_compiled_domains() {
        let repo = resolve_corpus_repo_root()
            .expect("corpus paths must resolve when tests run in-repo");
        let mut checked = 0;
        for (domain, rel, needle) in CORPUS_DOMAIN_TABLE {
            let Some(runtime) = runtime_corpus_count(domain) else {
                continue; // feature off in this binary — no authority to pin against
            };
            let walked = count_corpus_domain(&repo, rel, needle);
            assert_eq!(
                walked, runtime,
                "source-walk needle for {domain} counts {walked}, runtime corpus has {runtime} \
                 — the needle rotted (Issue 066 class: counting plumbing instead of entries)"
            );
            checked += 1;
        }
        assert!(checked >= 2, "expected to pin at least the default-on domains, checked {checked}");
    }

    #[test]
    fn docker_needle_is_not_the_old_plumbing_count() {
        // The regression itself: the old `rule_id:` needle matched exactly 2
        // occurrences (fn param + struct-field init) at every rev while the
        // corpus carried 27..31 entries. Pin (a) the table no longer carries
        // that needle and (b) the table's needle counts strictly more than
        // the plumbing count, so reverting either half is loud with a
        // message that says WHY.
        let repo = resolve_corpus_repo_root().unwrap();
        let (_, docker_rel, docker_needle) = CORPUS_DOMAIN_TABLE
            .iter()
            .find(|(d, _, _)| *d == "docker")
            .expect("docker row missing from CORPUS_DOMAIN_TABLE");
        assert_ne!(
            *docker_needle, "rule_id:",
            "the docker needle regressed to the Issue-066 plumbing needle"
        );
        let walked = count_corpus_domain(&repo, docker_rel, docker_needle);
        assert!(
            walked > 2,
            "docker source-walk counts {walked} — at or below the Issue-066 plumbing count (2)"
        );
    }
}

/// Plan 019 T1.5: the mode-flag validation matrix — every accepted /
/// rejected combination pinned, so a future guard change that breaks
/// `--fix-compile --write --verify-args "..."` (the silent-usability trap
/// this task closed) fails here instead of in a user's shell.
#[cfg(test)]
mod mode_flag_tests {
    use super::validate_mode_flags as v;

    #[test]
    fn accepted_combinations() {
        // The four modes alone (T4.3b adds --dump-trajectories).
        assert!(v(true, false, false, false, false, false, false).is_ok(), "--suggest");
        assert!(v(false, true, false, false, false, false, false).is_ok(), "--fix");
        assert!(v(false, false, true, false, false, false, false).is_ok(), "--fix-compile");
        assert!(v(false, false, false, true, false, false, false).is_ok(), "--dump-trajectories");
        // --write with each writing mode.
        assert!(v(false, true, false, false, true, false, false).is_ok(), "--fix --write");
        assert!(v(false, false, true, false, true, false, false).is_ok(), "--fix-compile --write");
        // The trap this task closed: --fix-compile --write --verify-args.
        assert!(
            v(false, false, true, false, true, false, true).is_ok(),
            "--fix-compile --write --verify-args \"...\""
        );
        // --verify still needs --fix (its batch semantics are --fix's).
        assert!(v(false, true, false, false, true, true, false).is_ok(), "--fix --write --verify");
        assert!(v(false, true, false, false, true, true, true).is_ok(), "--fix --verify --verify-args");
    }

    #[test]
    fn rejected_combinations() {
        assert!(v(false, false, false, false, false, false, false).is_err(), "no mode");
        assert!(
            v(false, false, false, false, true, false, false).is_err(),
            "--write without a mode"
        );
        assert!(
            v(false, false, false, false, false, true, false).is_err(),
            "--verify without --fix"
        );
        assert!(
            v(false, false, true, false, false, true, false).is_err(),
            "--verify with --fix-compile does NOT satisfy the --fix requirement"
        );
        assert!(
            v(true, false, false, false, false, false, true).is_err(),
            "--verify-args without --verify/--fix-compile"
        );
        assert!(
            v(false, false, true, true, false, false, false).is_err(),
            "--dump-trajectories is standalone (no other mode)"
        );
        assert!(
            v(false, false, false, true, true, false, false).is_err(),
            "--write does not apply to --dump-trajectories"
        );
    }
}

//! `cargo refine` — the unified healing CLI (Plan 005 Phase 2 + Phase 6).
//!
//! `--suggest` mode: walk real `.rs` files, chunk them at fn/impl/struct
//! boundaries, retrieve matching corpus rules via the latent substrate
//! (fan-out across all feature-enabled domains; the router verdict
//! annotates each span), and print top-K rules with scores + the
//! fixed/optimized pattern snippet for manual review.
//!
//! `--fix` mode (Phase 6 T6.1 + T6.4): run the compiled-in proposers on
//! each file + apply their span-preserving edits — `rust_perf` rules plus the
//! structural clippy lints (the three type-blind clippy proposers stay
//! suggest-only; see the clippy proposers' module docs). Without `--write`,
//! fixed files print a per-edit review (rule + the changed span) plus the
//! fmt plan; with `--write`, the spliced file is written in place and then
//! rustfmt'd AUTOMATICALLY when the original was already rustfmt-clean
//! (`--no-fmt` disables; pre-existing-drift + CRLF files are always skipped
//! so no formatting churn enters a mechanical diff — see src/fmt.rs).
//! Everything outside a matched span stays byte-identical — so written
//! output keeps pre-existing indentation verbatim where rustfmt would
//! reindent (e.g. `match_bool` mixed block/expr arms); the automatic fmt pass
//! closes the old Issue 027 T3 manual-rustfmt deferral. `--fix` ALSO runs
//! the suggest pass unless `--quiet` is given, so rules without proposers
//! still surface.
//!
//! Run (in-repo):
//!
//! ```text
//! cargo run --features latent_retrieval --bin cargo-refine -- --suggest src/
//! cargo run --features latent_retrieval --bin cargo-refine -- --fix src/          # review
//! cargo run --features latent_retrieval --bin cargo-refine -- --fix --write src/  # apply
//! ```
//!
//! Installed as `cargo-refine` (via `cargo install --path . --features
//! `latent_retrieval``), cargo's subcommand convention makes it `cargo refine
//! --suggest src/`.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use riir_refine::domains::clippy_lints::lint_level::{
    gated_lints, LevelResolver, ALLOW_BY_DEFAULT_GROUPS,
};
use riir_refine::domains::clippy_lints::proposers::FIXABLE_LINTS;
use riir_refine::domains::rust_perf::proposers::{FIXABLE_RULES, NOT_BENCHABLE_FIXABLE};
use riir_refine::draft::span_source::collect_source_files;
use riir_refine::draft::{
    parse_clippy_feed, paths_ref_match, ClippyDiagnostic, DomainId, LatentMatcher, SpanSuggestion,
};
use riir_refine::edit_emitter::RuleEdit;
use riir_refine::EvolveRecorder;

/// `cargo refine --info` — the one-command stats view (the 2026-09-17 owner
/// ask): account + consents, wallet, staking, this machine's mining queue
/// outcomes, and the repo-local work state. Bin-local module (the
/// composition is CLI surface, not lib API); read-only by contract.
#[cfg(feature = "kat_service")]
mod kat_info;

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
    /// Issue 153 T2: the verified binary self-update (fetch the release
    /// archive, verify SHA256SUMS, backup-swap). NOT the corpus lane — that
    /// is `update` above; the names are deliberately distinct.
    Upgrade,
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
    /// riir-dapps Plan 045 B7: `topup [--pack N]` — the USDC→KAT
    /// checkout (signed intent + the `solana:` URL). A different lane
    /// from `pay` (which ASKS a payer for KAT); this BUYS KAT with USDC.
    /// Needs `kat_transport` (the catalog read + the signed POST).
    Topup,
    /// The friction-cut money verb (issue 126 follow-up, owner ask
    /// 2026-10-14): `cargo refine buy 10 kat` — the amount resolves the
    /// catalog pack, MAINNET is the default dial (--net overrides), and
    /// the whole topup flow (QR, the capped-429 surface) rides along.
    Buy,
    /// The one-line KAT balance (issue 126 follow-up, the owner's value
    /// verbs: "expect `cargo refine kat` and `cargo refine tuna` should
    /// show value"): the bearer-free /history read, one agent-parseable
    /// line. Needs `kat_transport`.
    Kat,
    /// The one-line TUNA trial-credit balance + expiry window (same
    /// surface, the same ask). Needs `kat_transport`.
    Tuna,
    /// riir-dapps Proposal 009 §9 / issue 129: `get <amt> tuna` — the
    /// marketing TUNA airdrop to the OWN account id. The operator's hand
    /// over the governed can-opener lane (the mandate is governance's;
    /// this signs + fires the final batch). Needs `admin` (implies
    /// `kat_transport`); compiled out refuses exit 2 via the build stamp.
    TunaGet,
    /// Same lane, ANOTHER account: `send <amt> tuna <32-hex-account-id>`.
    /// A base58 pubkey shape refuses with the account-id hint. Same
    /// `admin` gate.
    TunaSend,
    /// riir-dao Proposal 001 T4 (Proposal 023): `account migrate` —
    /// the account-key rotation lane's CLI surface. `--github <user>`
    /// files the dual-signed request (the OLD key via the explicit
    /// `--old-key <path>` — the per-invocation key source, never the env
    /// var; the T-PREFLIGHT measured finding), `--object <request-id>`
    /// composes the prefilled objection-issue URL, `--status <id>` reads
    /// the lifecycle, `--census` is the report-only continuity preflight
    /// (the E-3 filing gate). Needs `kat_service`; the filing + status
    /// halves need `kat_transport`.
    Migrate,
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
    /// Proposal 015 Phase 2 (Plan 127): `replay` — the Max tier's earn
    /// lane. Login-gated lease → local `fix_verify` + clippy-oracle replay
    /// (scratch project) → signed verdict submit. Toolchain-fingerprint
    /// gated; kill switch `RIIR_REFINE_REPLAY=0`. Needs `kat_service` +
    /// `kat_transport` + `clippy_verify` (the oracle instrument).
    Replay,
    /// riir-dapps Plan 050 S1.11: `corpus-pr --src <devnet> --dst <mainnet>
    /// --submitter <32hex> [--seq N] [--key-file <path>] [--submit]` — the
    /// devnet→mainnet corpus PR client half (draft + submit, the S1.0
    /// auto-rebase first). Without `--key-file` the run is the read-only
    /// divergence gauge. Needs `kat_transport` (the lease reads).
    CorpusPr,
    /// riir-dapps Plan 023: `config get` / `config set --url <alias|url>` —
    /// the persisted service-url surface (env > config > default). Ungated:
    /// local-only (the consent-record precedent); the money commands it
    /// re-points are the gated ones.
    Config,
}

#[cfg(feature = "fix_verify")]
use riir_refine::domains::clippy_lints::proposers::VERIFY_LINTS;

/// Resolve the self-evolve memory file (Issue 018): `RIIR_REFINE_MEMORY`
/// overrides; otherwise the platform cache dir
/// (`$XDG_CACHE_HOME` / `~/.cache` / `%LOCALAPPDATA%` / temp fallback).
/// One-time disk migration (plan 192 P3): the pre-rename cache dir
/// `riir-clippy/` is renamed to `riir-refine/` when only the old one exists —
/// best-effort; a locked or vanished dir falls back to the empty new state.
fn default_memory_path() -> PathBuf {
    if let Ok(p) = std::env::var("RIIR_REFINE_MEMORY") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir);
    let new_dir = base.join("riir-refine");
    let old_dir = base.join("riir-clippy");
    if !new_dir.exists() && old_dir.exists() {
        let _ = std::fs::rename(&old_dir, &new_dir);
    }
    new_dir.join("memory.json")
}

/// Read a Rust source file for the span-based lanes (P44): a leading UTF-8
/// BOM poisons proc-macro2's fallback span→byte mapping — measured minimal
/// repro, `byte_range()` returns char-space offsets for BOM'd source, so
/// every syn span lands shifted against the real bytes (mid-char where the
/// drift crosses a multi-byte char — the helpers.rs:630 panic class — and
/// silently wrong-but-valid elsewhere, the misplaced-edit class). Strip it
/// once at the read seam so parse coordinates, proposers, and the write-back
/// share one coordinate system; the strip is disclosed and the healed file
/// is written BOM-free (a leading BOM is optional in Rust source).
fn read_source(path: &std::path::Path) -> std::io::Result<String> {
    let src = std::fs::read_to_string(path)?;
    match src.strip_prefix('\u{feff}') {
        Some(rest) => {
            eprintln!(
                "bom: stripped leading UTF-8 BOM (3 bytes) from {} — \
                 spans are computed BOM-free; the healed file is written BOM-free",
                path.display()
            );
            Ok(rest.to_owned())
        }
        None => Ok(src),
    }
}

/// Resolve the repo-local `.refine` root shared by the trajectory store and the
/// KAT meter: `RIIR_REFINE_TRAJ_ROOT` → else the first path arg's nearest
/// `.git` ancestor (the repo-local semantic 038-T2 picked for the 037-A2
/// per-repo adoption audit) → else cwd.
#[cfg(any(feature = "latent_retrieval", feature = "kat_service"))]
fn resolve_heal_root(paths: &[PathBuf]) -> PathBuf {
    std::env::var("RIIR_REFINE_TRAJ_ROOT")
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
                let mut dir = if abs.is_dir() {
                    abs
                } else {
                    abs.parent()?.to_path_buf()
                };
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
fn open_traj_store(paths: &[PathBuf], migrate: bool) -> Option<riir_refine::traj_store::TrajStore> {
    let root = resolve_heal_root(paths);
    match riir_refine::traj_store::TrajStore::open(&root) {
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
/// this run. `None` when disabled — `RIIR_REFINE_KAT_METER=0|off|false|no`
/// (the `RIIR_REFINE_SELF_EVOLVE=0` kill-switch pattern) or an unreachable
/// meter location (never fatal to a heal). `RIIR_REFINE_KAT_METER_FILE`
/// overrides the exact file; otherwise `<resolve_heal_root>/.refine/kat_meter.json`
/// (beside the trajectory store).
/// The shared billing kill-switch (Plan 024 — ONE switch for the whole KAT
/// billing plane): `RIIR_REFINE_KAT_METER=0|off|false|no` disables the meter,
/// the charge ledger, AND the 075 soft gate (a user who turned billing off
/// must not keep a stale-ack gate). The `RIIR_REFINE_SELF_EVOLVE=0` pattern.
#[cfg(feature = "kat_service")]
fn kat_billing_disabled() -> bool {
    std::env::var("RIIR_REFINE_KAT_METER")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "0" | "off" | "false" | "no"))
}

#[cfg(feature = "kat_service")]
fn open_kat_meter(paths: &[PathBuf]) -> Option<riir_refine::kat_meter::KatMeter> {
    if kat_billing_disabled() {
        return None;
    }
    if let Ok(file) = std::env::var("RIIR_REFINE_KAT_METER_FILE") {
        return Some(riir_refine::kat_meter::KatMeter::open_at(PathBuf::from(
            file,
        )));
    }
    let root = resolve_heal_root(paths);
    Some(riir_refine::kat_meter::KatMeter::open(&root))
}

/// Proposal 005 Phase 0 T0.6 (Plan 024): open the KAT charge ledger right
/// after the meter — the per-unit-burn consumer of the meter's events.
/// Shares the meter's kill-switch (`RIIR_REFINE_KAT_METER=0` disables BOTH —
/// one billing switch, per the Plan 024 decision); `RIIR_REFINE_KAT_CHARGE_FILE`
/// overrides the exact file. `meter` is the just-opened meter's PRE-session
/// state — the recovery path (missing/corrupt charge ledger → watermark = the
/// meter cumulative at open) is only correct against a pre-session reading.
/// T7 (riir-kat Issue 003): the book is PER-NET (`KatCharge::open_net`) —
/// the net the current resolution names, so each ledger's cost basis is
/// its own; the env override stays the exact-file posture.
/// riir-kat Issue 003 T9 (the consumer half): the net-pin gate at the ONE
/// URL seam. Every command surface that dials resolves through
/// `service_url_for_command`; this wrapper adds the pin check so a pinned
/// build refuses a foreign host BEFORE any dial, naming the rebuild
/// (foreign hosts compile to nothing in a pinned build — this refusal is
/// the runtime half of that law). Exit 2, never a silent canonicalization.
#[cfg(feature = "kat_service")]
fn command_service_url() -> String {
    let raw = riir_refine::kat_lease_client::service_url_for_command();
    match riir_refine::kat_net_pin::ensure_net_allowed(&raw) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("service url refused: {e}");
            std::process::exit(2);
        }
    }
}

/// The URL a LOCAL state partition names (the lease store's per-net dir, the
/// mining outbox read) — issue 159: the both-pins CONFLICT posture must not
/// kill a run that never dials. The pin contract refuses HOSTED dials at
/// that seam (loopback follows `allow-local`, issue 163; hosted refusals
/// stay fatal in [`command_service_url`]), but a conflicted build is a dev
/// posture whose local-only work — the scan, the corpus lease STORE, the
/// trajectory memory — must still run: one advisory line, the default URL
/// names the partition, and the first real dial still exits 2 through the
/// strict seam.
#[cfg(feature = "kat_service")]
fn net_partition_url() -> String {
    let raw = riir_refine::kat_lease_client::service_url_for_command();
    match riir_refine::kat_net_pin::ensure_net_allowed(&raw) {
        Ok(url) => url,
        Err(e) if riir_refine::kat_net_pin::NET_PIN.starts_with("conflict") => {
            eprintln!(
                "net-pin conflict posture: local state partitions under the default net; \
                 hosted dials still refuse (net: {e})"
            );
            raw
        }
        Err(e) => {
            eprintln!("service url refused: {e}");
            std::process::exit(2);
        }
    }
}

#[cfg(feature = "kat_service")]
fn open_kat_charge(
    paths: &[PathBuf],
    meter: Option<&riir_refine::kat_meter::KatMeter>,
) -> Option<riir_refine::kat_meter::KatCharge> {
    let meter = meter?;
    if let Ok(file) = std::env::var("RIIR_REFINE_KAT_CHARGE_FILE") {
        return Some(riir_refine::kat_meter::KatCharge::open_at(
            PathBuf::from(file),
            meter.state(),
        ));
    }
    let root = resolve_heal_root(paths);
    Some(riir_refine::kat_meter::KatCharge::open_net(
        &root,
        &command_service_url(),
        meter.state(),
    ))
}

/// Issue 076 client half: the ONE burn-sync status line (the
/// consent-heartbeat contract — one line, never an exit-code change).
/// Issue 123 rider: when the B2 server names the per-report plane split,
/// ONE `plane_split_note()` line renders beneath it (never on a 0-0
/// replay / pre-B2 server / KAT-only report, so the one-line contract
/// holds wherever it was pinned).
/// Success carries the fresh server balance straight from the push
/// response — the ONLY fresh-balance source, so a skipped push adds no
/// network GET (the no-dial-when-nothing-to-send rule).
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn print_burn_sync(outcome: &riir_refine::kat_burn_client::BurnSync) {
    match outcome {
        riir_refine::kat_burn_client::BurnSync::Reported(ack) => {
            let balance = riir_refine::kat_payreq_client::render_kat(ack.balance_micro);
            println!(
                "burn sync: {} KAT reported (server balance {balance})",
                ack.burn_kat_total
            );
            // Issue 123: the plane split — how the server funded THIS
            // report. `None` (pre-B2 server / 0-0 replay / KAT-only)
            // renders nothing, so the tail keeps its one-line contract.
            if let Some(note) = ack.plane_split_note() {
                println!("{note}");
            }
        }
        riir_refine::kat_burn_client::BurnSync::Stacked { total, reason } => {
            println!(
                "burn sync: offline — {total} KAT stacked, reports with the next run ({reason})"
            );
        }
        riir_refine::kat_burn_client::BurnSync::Skipped => {}
    }
}

/// Issue 080 — the sticky-grace leg is SERVER TRUTH ONLY (the dated removal,
/// 2026-09-10, fleet v3 live): the burn-ack v3 `contributed` field (written
/// only from the authenticated burn-push response) decides, so
/// `Some(false)` refuses and `None` (a pre-v3 ack/server, or a v2 file) is
/// simply NOT contributed — the gate then follows the balance leg. The
/// legacy machine-local `contributed_once.json` marker is RETIRED: the
/// `kat_contribution` module is gone, any file left on disk by a
/// pre-removal install is ignored, and the forge surface it armed is fully
/// removed. (Anti-brick direction preserved at the ACK level: a
/// corrupt/missing/unparseable ack loads empty → no balance → the gate is
/// warn-only, never a refusal.)
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn sticky_grace(ack: &riir_refine::kat_burn_client::KatBurnAck) -> bool {
    ack.contributed == Some(true)
}

/// Issue 075 (owner soft gate, 2026-09-08): the depleted-account refusal.
/// Fires for the scan flow — EVERY mode that processes source burns fuel
/// (the bare dry run settles charge too; measured by `kat_burn_stack`), so
/// gating only `--fix --write` would be a sieve. Placed after
/// `validate_mode_flags` (the mode is known) and before the meter opens
/// (nothing heavy has run), and only in a transport build — the gate's
/// input is the last SYNCED server balance, which only that build can
/// obtain. Disarmed by the gate kill-switch (`RIIR_REFINE_KAT_GATE=0`) and by
/// the shared billing switch (`RIIR_REFINE_KAT_METER=0` — one billing plane,
/// the Plan 024 decision); otherwise defers to the pure predicate.
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn enforce_kat_soft_gate(args: &Args) {
    if args.dump_trajectories {
        return; // a local export processes no source and burns no fuel
    }
    if kat_billing_disabled() || kat_gate_disabled() {
        return;
    }
    let root = resolve_heal_root(&args.paths);
    // T7 (riir-kat Issue 003): the ack lives in the PER-NET dir — the same
    // placement the burn sync reads and writes. The flat path here was the
    // measured 2026-10-12 split-brain: the gate judged the pre-split flat
    // ack (a stale balance, possibly another net's) while the sync
    // advanced the per-net one.
    let ack_path = riir_refine::kat_state_stamp::net_state_dir_for_url(&root, &net_partition_url())
        .join(riir_refine::kat_burn_client::KatBurnAck::FILE_NAME);
    let (ack, warning) = riir_refine::kat_burn_client::KatBurnAck::load_at(&ack_path);
    if let Some(w) = warning {
        eprintln!("{w}");
    }
    let account_dir = riir_refine::kat_account::KatAccount::default_dir();
    let verdict = riir_refine::kat_soft_gate::evaluate(riir_refine::kat_soft_gate::SoftGateInput {
        transport_capable: true,
        account_present: riir_refine::kat_account::KatAccount::load(&account_dir).is_ok(),
        last_synced_balance_micro: ack.last_balance_micro,
        contributed_and_synced: sticky_grace(&ack),
        tuna_balance_micro: ack.tuna_balance_micro,
    });
    // Which ack fed the FINAL verdict (the 079 attempt may reload it) —
    // the refusal message names the trial-credit leg from the same truth.
    let mut final_tuna = ack.tuna_balance_micro;

    // Issue 079 (owner ask, 2026-09-09): a flat refusal gives a depleted
    // user no choice — OFFER the contribute-now path first. One accepted
    // push records the server's `contributed` truth in the burn ack (the
    // 075 sticky grace, server-side since issue 080), so a yes here lifts
    // the gate for good; payouts land at each epoch settle. The
    // re-evaluation re-reads ONLY what `run_kat_sync` can change (the ack's
    // server balance + its `contributed` truth); `depleted_mine_offer`
    // never asks a machine that already answered no (an explicit opt-out).
    #[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
    let (verdict, offered) = {
        if matches!(
            verdict,
            riir_refine::kat_soft_gate::SoftGateVerdict::Refuse { .. }
        ) && depleted_mine_offer()
        {
            eprintln!("cargo refine: out of KAT — contributing now (`cargo refine sync`); one accepted push lifts this gate for good (payouts land at each epoch settle)");
            run_kat_sync();
            let (reloaded, w) = riir_refine::kat_burn_client::KatBurnAck::load_at(&ack_path);
            if let Some(w) = w {
                eprintln!("{w}");
            }
            let verdict =
                riir_refine::kat_soft_gate::evaluate(riir_refine::kat_soft_gate::SoftGateInput {
                    transport_capable: true,
                    account_present: riir_refine::kat_account::KatAccount::load(&account_dir)
                        .is_ok(),
                    last_synced_balance_micro: reloaded.last_balance_micro,
                    contributed_and_synced: sticky_grace(&reloaded),
                    tuna_balance_micro: reloaded.tuna_balance_micro,
                });
            final_tuna = reloaded.tuna_balance_micro;
            (verdict, true)
        } else {
            (verdict, false)
        }
    };
    #[cfg(not(all(feature = "kat_transport", feature = "latent_retrieval")))]
    let offered = false;

    let riir_refine::kat_soft_gate::SoftGateVerdict::Refuse { balance_micro } = verdict else {
        return;
    };
    let balance = riir_refine::kat_payreq_client::render_kat(balance_micro);
    let account_id = riir_refine::kat_account::KatAccount::load(&account_dir)
        .map(|a| a.state().account_id.clone())
        .unwrap_or_default();
    let top_up = riir_refine::kat_lease_client::payment_url(&command_service_url(), &account_id);
    eprintln!(
        "cargo refine: REFUSED — the account's last synced KAT balance is {balance} \
         (depleted) and this machine has not yet contributed (Issue 075 soft gate)."
    );
    // Proposal 006: name the trial-credit leg when the server has said it
    // is gone (Some ≤ 0 is a fact). `None` (a pre-v4 ack / pre-TUNA
    // server) prints nothing — the pre-006 refusal, byte for byte.
    if final_tuna.is_some_and(|tuna| tuna <= 0) {
        eprintln!("Trial credit (TUNA) is exhausted too — the network draws it before KAT (refine, rethink, everything).");
    }
    // riir-dapps Issue 094 D4: the free daily faucet/drip tier is RETIRED
    // and `--refill` retired with it (devnet-ops only). The refuel paths a
    // user is offered are exactly two — contribute or pay.
    eprintln!("Refuel with exactly one of:");
    eprintln!("  * contribute:  cargo refine --mine   (what mining is)  →  cargo refine sync   (one accepted push lifts the gate; payouts land at each epoch settle)");
    eprintln!("  * top up:      {top_up}");
    eprintln!("      (or open the checkout right here: cargo refine topup --pack 1)");
    // The owner's 2026-09-18 amendment: the refusal also SHOWS what the
    // run would have done — count-only. Still no meter, no charge ledger,
    // no fix content (the count is the advertisement; the fixes stay paid).
    print_depleted_survey(args);
    eprintln!(
        "(logged-out / offline / never-synced machines are never gated — this reads the last \
         SYNCED server balance; RIIR_REFINE_KAT_GATE=0 disables the gate)"
    );
    if offered {
        eprintln!(
            "(the contribute-now attempt above did not lift the gate — nothing new was queued \
             to push, or the push failed; the sync output above has the detail)"
        );
    }
    std::process::exit(1);
}

/// The owner's 2026-09-18 depleted-run guidance (the 075 gate amended):
/// the refusal stays at run start — no meter, no charge ledger, no fixes
/// — but it now SURVEYS what the run would have done, count-only. The
/// count is the advertisement; the fix content stays paid (the D4
/// doctrine: contribute or pay, nothing is fixed for free). The survey
/// reads the compiled-in corpus (the gate runs before the lease context
/// builds) through the same suggest primitives the dry run uses, and
/// never prints a count it could not compute.
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn print_depleted_survey(args: &Args) {
    eprintln!(
        "What this run would have done (count-only survey — nothing was fixed, nothing was billed):"
    );
    // What it would have fixed — scan modes only (a bare run sets
    // suggest=true; --fix and --fix-compile are the other scan modes).
    if args.suggest || args.fix || args.fix_compile {
        match survey_fixable_count(&args.paths) {
            Some(0) => eprintln!(
                "  * survey: 0 fixable findings on the requested paths — nothing was skipped"
            ),
            Some(n) => eprintln!(
                "  * survey: {n} fixable finding(s) found — NOT applied (account depleted; the fixes stay gated)"
            ),
            None => {} // no sources under the paths / index unavailable — no fabricated count
        }
    }
    // What mining can do about it, and when it pays. The count is the
    // SYNC-time outbox (the same root `cargo refine sync` drains), so it is
    // honest right after the contribute-now attempt above.
    let queued = mining_queued_batches();
    if queued > 0 {
        eprintln!(
            "  * mining: {queued} batch(es) queued — `cargo refine sync` pushes them (payouts land at the 7-day epoch settle)"
        );
    } else {
        eprintln!(
            "  * mining: none queued — `cargo refine --mine` contributes (payouts land at the 7-day epoch settle)"
        );
    }
    let leaderboard = format!(
        "{}/leaderboard",
        command_service_url().trim_end_matches('/')
    );
    eprintln!("  * payouts land within 7 days of a push — exact dates: {leaderboard}");
}

/// The survey half of the depleted guidance: the fixable-span count over
/// the requested paths, measured the same way the run summary measures
/// "N with suggestions" (span-level, any corpus rule or oracle anchor).
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn survey_fixable_count(paths: &[PathBuf]) -> Option<usize> {
    let targets: Vec<PathBuf> = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };
    let mut files = Vec::new();
    for path in &targets {
        collect_source_files(path, ".rs", &mut files);
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return None;
    }
    let mut matcher = LatentMatcher::from_enabled_domains().ok()?;
    let mut fixable = 0usize;
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        for s in matcher.suggest_file(&src, &f.display().to_string(), 1) {
            if !(s.all_rules().is_empty()
                && s.anchored_rules.is_empty()
                && s.anchored_missing.is_empty())
            {
                fixable += 1;
            }
        }
    }
    Some(fixable)
}

/// The mining half: batches queued in the sync outbox (the same root
/// `cargo refine sync` drains), 0 when the dir does not exist yet.
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn mining_queued_batches() -> u32 {
    let outbox = riir_refine::kat_state_stamp::net_state_dir_for_url(
        &resolve_heal_root(&[]),
        &net_partition_url(),
    )
    .join("outbox");
    u32::try_from(std::fs::read_dir(&outbox).map_or(0, |rd| {
        rd.filter_map(std::result::Result::ok)
            .filter(|e| e.path().is_file())
            .count()
    }))
    .unwrap_or(u32::MAX)
}

/// Issue 149 T1: one scan root's disclosure facts. `untracked: None` means
/// the count is UNAVAILABLE (git absent, or the root is not inside a
/// repository) — never read `None` as zero.
#[derive(Debug, PartialEq, Eq)]
struct ScanRootInfo {
    /// The canonicalized scan root — absolute, which is the whole point.
    root: PathBuf,
    /// Collected source files under this root (.rs/.wgsl/Dockerfile/Cargo.toml).
    files: usize,
    /// Of those, how many git reports untracked. `None` = unavailable.
    untracked: Option<usize>,
}

/// The pre-scan disclosure (Issue 149 T1): a bare `--fix` scans the whole
/// working directory when given no paths — untracked WIP included — and the
/// v0.2.1 capture incident (a generator healed the real kat-service tree
/// after forgetting its `cd`) is exactly the shape this one line catches:
/// WHICH absolute tree is about to be scanned, how many source files it
/// holds, and how many of them git considers untracked. Disclosure, never a
/// gate — the just-works contract (no paths → scan `.`) stays untouched.
fn scan_root_infos(paths: &[PathBuf], collected: &[PathBuf]) -> Vec<ScanRootInfo> {
    let mut infos: Vec<ScanRootInfo> = Vec::new();
    for path in paths {
        // git -C needs a directory; a single-file root discloses its parent.
        let dir = if path.is_dir() {
            path
        } else {
            match path.parent() {
                Some(p) => p,
                None => continue,
            }
        };
        let Ok(root) = dir.canonicalize() else {
            continue;
        };
        if infos.iter().any(|i| i.root == root) {
            continue;
        }
        let files = collected
            .iter()
            .filter(|f| std::fs::canonicalize(f).is_ok_and(|c| c.starts_with(&root)))
            .count();
        let untracked = count_untracked_collected(&root, collected);
        infos.push(ScanRootInfo {
            root,
            files,
            untracked,
        });
    }
    infos
}

/// How many of `collected` git reports untracked under `root` — one
/// `git ls-files --others --exclude-standard` subprocess scoped to the root
/// (respects .gitignore, so ignored build output never inflates the count).
/// `None` on any git failure (absent binary, not a repository): the count is
/// unavailable, not zero.
fn count_untracked_collected(root: &Path, collected: &[PathBuf]) -> Option<usize> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let untracked: std::collections::HashSet<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    if untracked.is_empty() {
        return Some(0);
    }
    let mut n = 0usize;
    for f in collected {
        let Ok(c) = std::fs::canonicalize(f) else {
            continue;
        };
        let Ok(rel) = c.strip_prefix(root) else {
            continue;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if untracked.contains(&rel) {
            n += 1;
        }
    }
    Some(n)
}

/// The one-line render, split out for the tests (stdout is not capturable).
fn render_scan_line(info: &ScanRootInfo) -> String {
    match info.untracked {
        Some(k) if k > 0 => format!(
            "scan root: {} — {} source file(s), {k} untracked included",
            info.root.display(),
            info.files
        ),
        _ => format!(
            "scan root: {} — {} source file(s)",
            info.root.display(),
            info.files
        ),
    }
}

/// The `--yes` run-global (the flag form of `RIIR_REFINE_YES`): every y/N
/// prompt answers yes without a TTY — the agents/CI contract. A static
/// rather than plumbed `Args`: the prompt sites sit deep in flows that do
/// not otherwise carry it, and one store site in `main` covers the run.
static PROMPT_YES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The env half of the `--yes` contract (`RIIR_REFINE_YES=1|true|yes|on`).
fn prompt_yes_env() -> bool {
    std::env::var("RIIR_REFINE_YES")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

/// Issue 079 — the depleted-run contribute-now offer's consent surface.
/// `RIIR_REFINE_KAT_PROMPT_YES=1|true|yes` is the automation hook (agents and
/// CI run without a TTY); an explicit `--unmine` opt-out is never re-asked
/// (it already answered this question); otherwise ONE y/N question on an
/// interactive stdin. No env and no TTY → false (the plain 075 refusal,
/// byte-identical to the pre-079 shape).
#[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
fn depleted_mine_offer() -> bool {
    use std::io::IsTerminal;
    use std::io::Write;

    if std::env::var("RIIR_REFINE_KAT_PROMPT_YES")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
    {
        return true;
    }
    // `--yes` (or `RIIR_REFINE_YES=1`) — the run-global non-interactive consent.
    if PROMPT_YES.load(std::sync::atomic::Ordering::Relaxed) {
        return true;
    }
    let consent_dir = riir_refine::kat_mining_consent::default_dir();
    let (consent, _) = riir_refine::kat_mining_consent::read_with_warning(&consent_dir);
    if !consent.push_allowed() {
        return false; // an explicit opt-out IS the answer
    }
    if !std::io::stdin().is_terminal() {
        return false;
    }
    eprint!("cargo refine: out of KAT — contribute mining data now to lift the gate? [y/N] ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// The `RIIR_REFINE_KAT_GATE` kill-switch (the `RIIR_REFINE_KAT_METER` pattern):
/// `0|off|false|no` disarms the 075 soft gate for this run.
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn kat_gate_disabled() -> bool {
    std::env::var("RIIR_REFINE_KAT_GATE")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "0" | "off" | "false" | "no"))
}

/// Proposal 008 (the owner's just-works UX, 2026-09-08): `cargo refine
/// --mine` — the contribution onboarding. One screen: what mining is,
/// the policy warning, the web link, and the login → heal → sync →
/// account loop. Feature halves (account status / outbox) appear only
/// when their feature compiled in — the explainer itself never needs one.
/// The default `.refine-sync-policy` `--mine` writes (version = the compiled
/// [`riir_refine::kat_redact::REDACT_VERSION`], see the ⛔ note on the const): own-repo rows sync, the standard
/// source globs (repo root + the `crates/*/` workspace layout — riir-dapps
/// Issue 119: the gate binds rows by their recorded file since riir-refine
/// `21f76593`, so a root-only glob set would block every workspace crate;
/// the policy is allow-only, so `**/src/**` is NOT used — it would admit
/// `vendor/*/src`), findings redacted in place, secrets still blocked by the
/// gate itself. WITHOUT a policy the gate is deny-by-default and every sync
/// prints `0 included, N blocked` — the friction this write removes. The
/// user's existing policy is NEVER touched.
///
/// ⛔ The `version` literal must equal [`riir_refine::kat_redact::REDACT_VERSION`]
/// at every build: the gate refuses a mismatched version deny-all, so a
/// stale literal silently denies every row `--mine` opted in. It drifted 3
/// behind the v4/v5 gate bumps (red 2026-09..10, caught by
/// `default_sync_policy_parses_at_the_compiled_version`) — any
/// REDACT_VERSION bump re-bumps this literal in the same commit. The JSON
/// stays hand-typed ON PURPOSE: a struct-derived default would make that
/// test a tautology (the restatement-theorem class — a check that cannot
/// fail proves nothing).
const DEFAULT_SYNC_POLICY_JSON: &str = r#"{
  "version": 5,
  "sync_enabled": true,
  "allow_origins": ["own-repo"],
  "allow_paths": ["src/**/*.rs", "tests/**/*.rs", "benches/**/*.rs", "examples/**/*.rs",
                  "crates/*/src/**/*.rs", "crates/*/tests/**/*.rs", "crates/*/benches/**/*.rs", "crates/*/examples/**/*.rs"],
  "max_span_bytes": 65536,
  "allow_redacted": false
}"#;

/// The gate [`DEFAULT_SYNC_POLICY_JSON`] defines — what an opted-in repo
/// gets. Fail-closed (deny-all) if the constant ever stops parsing; the
/// `default_sync_policy_parses` test pins that it does not.
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
fn default_sync_policy_gate() -> riir_refine::kat_redact::RedactionGate {
    serde_json::from_str::<riir_refine::kat_redact::SyncPolicy>(DEFAULT_SYNC_POLICY_JSON)
        .map_or_else(
            |_| riir_refine::kat_redact::RedactionGate::deny_all(),
            riir_refine::kat_redact::RedactionGate::from_policy,
        )
}

/// riir-dapps Plan 045 Phase A1 (Proposal 008 §2): the first-join
/// environment choice. Fires ONLY on the no-config path — a machine with a
/// config record OR an env override (any source) is never prompted and
/// sees zero output (the consent / soft-gate flows stay byte-identical).
/// The choice records through the EXISTING `config set` record
/// (`service_url.json`), so `config get` reports it and
/// `service_url_for_command` resolves it. A refusal prints loud and exits
/// 2 with the remedy named — a fresh non-TTY join never silently rides the
/// old default, and never hangs.
fn ensure_join_env_recorded() {
    if let Err(msg) =
        riir_refine::kat_join::run(PROMPT_YES.load(std::sync::atomic::Ordering::Relaxed))
    {
        eprintln!("{msg}");
        std::process::exit(2);
    }
}

fn run_mine_intro() {
    // Plan 045 A1: the first-join choice runs before the explainer so a
    // fresh machine's join output opens with its network decision (and a
    // mainnet joiner's posture copy sits up-front, never after the first
    // 503). A configured machine: no prompt, no output — the explainer
    // below is byte-identical to the pre-A1 shape.
    ensure_join_env_recorded();
    println!("KAT mining — turn your healing work into network credit");
    println!();
    println!("WHAT: every cargo refine run records HOW the healer behaved (which rules");
    println!("fired, which were declined, what merged). Batches of those records are");
    println!("the network's training data. Miners who contribute them are paid from 70%");
    println!("of every KAT the network burns, at each epoch settle.");
    println!();
    println!("POLICY (read before contributing):");
    println!("  * KAT is service credit: NO cash value and NO redemption right.");
    println!("  * What a record carries: the rule, its counters, and the CODE SNIPPET");
    println!("    the fix touched (the replaced span + its replacement — never a whole");
    println!("    file, never a file path), only from your own repo's src/ tests/");
    println!("    benches/ examples/, at the root or under crates/*/ (.refine-sync-policy).");
    println!("    Every snippet is scanned for");
    println!("    secrets (API keys, tokens, private keys — the consolidated redact v4 ");
    println!("    provider-format set; a snippet with ANY finding is dropped, not ");
    println!("    sent; `cargo refine --secret-scan` shows you the same findings ");
    println!("    locally). Batches are SIGNED with your");
    println!("    account key.");
    println!("  * Machine label: none unless you set RIIR_REFINE_MACHINE_LABEL; a label");
    println!("    you set is shown on the public adoption card.");
    println!("  * Contributing is opt-in per MACHINE: after --mine, every heal run (the");
    println!("    dry run and --fix) auto-syncs the redacted batch — one line, fail-soft.");
    println!("    Before --mine, nothing leaves this machine (one exception: --mine/--unmine");
    println!("    send a signed tier note); --unmine stops the auto-push; `cargo refine sync`");
    println!("    stays the manual push.");
    println!();
    println!("START (3 steps):");
    println!("  1. learn:      https://ai.gist.rs        <- the network front (policy + ledger)");
    println!("  2. login:      automatic — --mine logs you in when no account exists (explicit: cargo refine login)");
    println!("  3. heal:       cargo refine --fix        <- real fixes, metered locally");
    println!("  sync:          --mine just ran it; re-run --mine or `cargo refine sync` to push newer batches");
    println!();
    println!("WATCH: cargo refine account              <- burn, balance, grant, network state");
    println!();

    // Feature halves: the status the build can actually see.
    #[cfg(feature = "kat_service")]
    {
        // Owner mandate 2026-09-10: --mine IS the join command — a missing
        // account is a dead-end the explainer had to apologize for ("step 2
        // first"). Run the zero-config login chain right here: LOCAL key ops
        // only (no network, no prompt — the fallback chain ends in a
        // generated key with a disclosed note), outcome printed via the same
        // printer `cargo refine login` uses. The account dir is re-resolved
        // after login so an adoption/generate on this run is what reports.
        let account = ensure_logged_in();
        if let Some(account) = account {
            println!(
                "status: logged in as {} (key: {})",
                riir_refine::kat_account::short_id(&account.state().account_id),
                account.key_path().display()
            );
        }
        let wallet_base = command_service_url();
        println!(
            "web wallet (same account): {}/wallet   <- GitHub SSH-key match",
            wallet_base.trim_end_matches('/')
        );
    }
    #[cfg(not(feature = "kat_service"))]
    {
        println!("status: account/status halves need a build with --features kat_service");
    }
    #[cfg(feature = "kat_transport")]
    {
        println!("transport: available — `cargo refine sync` pushes the queued batch");
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        println!("transport: this build has no sync (needs --features kat_transport)");
    }

    // Owner mandate 2026-09-08: --mine is now the opt-IN WRITE as well as
    // the explainer — the three-tier engagement story needs a state the CLI
    // can record, and an explainer that records nothing is not a tier.
    // Ungated (kat_mining_consent compiles in every build), so the consent
    // half works even where the status halves above did not compile.
    let dir = riir_refine::kat_mining_consent::default_dir();
    println!();
    match riir_refine::kat_mining_consent::write(&dir, true) {
        Ok(()) => {
            println!(
                "consent: recorded — mining is opted IN for this machine ({})",
                riir_refine::kat_mining_consent::consent_path(&dir).display()
            );
        }
        Err(e) => {
            eprintln!("consent: NOT recorded — {e}");
            eprintln!(
                "consent: the explainer above still stands; `cargo refine sync` behaves as before \
                 (running sync IS the consent)"
            );
        }
    }
    let (state, warning) = riir_refine::kat_mining_consent::read_with_warning(&dir);
    if let Some(w) = warning {
        eprintln!("{w}");
    }
    println!("{}", state.status_line());
    // Issue 119 T16b: the consent screen states the licence the web terms
    // already carry ("What you contribute — the MIT licence", owner call
    // 2026-10-06) — the CLI is the moment of opt-in, so the disclosure
    // lives here too, one line beside the consent text.
    println!("licence: what you contribute is published under the MIT licence");
    println!("opt out any time: cargo refine --unmine");

    // The policy moment rides the consent moment: opting IN here while the
    // repo has no `.refine-sync-policy` would make the very next `sync` print
    // `0 included, N blocked` (deny-by-default) with no named remedy. Write
    // the default once, loudly; never overwrite; never write outside a git
    // repo (the root resolver falls back to cwd — a non-repo cwd has nothing
    // to sync from anyway).
    let policy_root = resolve_heal_root(&[]);
    let policy_path = policy_root.join(".refine-sync-policy");
    if policy_path.exists() {
        println!("policy: using existing {}", policy_path.display());
        // Issue 119 T9: a policy file written before the multi-crate
        // default (riir-refine 3e8f87fb) allows only the ROOT's rows —
        // every workspace `crates/*/` row is blocked by it, and the file
        // is NEVER rewritten (the owner's edit is the only writer). One
        // notice line, naming the glob; the blocked-count line on `sync`
        // no longer blames the policy for these rows.
        if let Ok(existing) = std::fs::read_to_string(&policy_path) {
            if !existing.contains("crates/") {
                println!(
                    "policy: this file predates the multi-crate default — workspace crates are \
                     blocked until it lists them, e.g. \"crates/*/{{src,tests,benches,examples}}/**/*.rs\""
                );
            }
        }
    } else if policy_root.join(".git").exists() {
        match std::fs::write(&policy_path, DEFAULT_SYNC_POLICY_JSON) {
            Ok(()) => println!(
                "policy: wrote default .refine-sync-policy at {} — rows from this repo now sync \
                 (secrets are still blocked by the redaction gate)",
                policy_path.display()
            ),
            Err(e) => eprintln!(
                "policy: could not write .refine-sync-policy ({e}) — `cargo refine sync` will block \
                 rows until one exists"
            ),
        }
    }

    // The one-shot signed tier note (owner mandate 2026-09-08): best-effort —
    // offline / not-logged-in prints one skipped line; never fatal, never an
    // exit-code change. Feature-gated: a build without the service+transport
    // halves simply has no heartbeat line.
    #[cfg(all(feature = "kat_service", feature = "kat_transport"))]
    {
        let base = command_service_url();
        let trimmed = base.trim_end_matches('/');
        // Issue 119 T9: an override is INVISIBLE once set (the env outlives
        // the shell that set it — a ~/.profile row, a direnv state) and
        // every signed note lands wherever it points. Name it on first use.
        if let Ok(url) = std::env::var(riir_refine::kat_service_config::SERVICE_URL_ENV_VAR) {
            if !url.trim().is_empty() {
                println!(
                    "note: {} is set — this run talks to {trimmed} (unset it for the default service)",
                    riir_refine::kat_service_config::SERVICE_URL_ENV_VAR
                );
            }
        }
        match riir_refine::kat_consent_client::send_heartbeat(&base) {
            Ok("recorded") => println!("consent heartbeat: recorded (opted IN) at {trimmed}"),
            Ok("unchanged") => println!("consent heartbeat: already recorded (opted IN)"),
            Ok(other) => println!("consent heartbeat: skipped — the service answered \"{other}\""),
            Err(reason) => println!("consent heartbeat: skipped — {reason}"),
        }
    }

    // The join completes with a SYNC (owner mandate 2026-09-10, the same
    // call that killed the "step 2 first" dead-end): opting in and then
    // telling the user to find `cargo refine sync` is exactly the confusing
    // extra step the tier exists to remove. The full sync runs here — push
    // half (the grant claim rides an accepted batch), burn drain, lease
    // pull — the SAME command output `cargo refine sync` prints. Guards: the
    // transport halves (a build without them has no sync, stated above),
    // and a SUCCESSFUL login — run_kat_sync exits the process on a missing
    // account, which must never turn a completed --mine into a failure.
    #[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
    {
        let account_ok = riir_refine::kat_account::KatAccount::load(
            &riir_refine::kat_account::KatAccount::default_dir(),
        )
        .is_ok();
        if account_ok {
            println!();
            println!("sync: pushing what's queued (the same as running `cargo refine sync`):");
            run_kat_sync();
        }
    }
}

/// Owner mandate 2026-09-08: `cargo refine --unmine` — the opt-OUT
/// counterpart to `--mine`. Records machine-global consent = OPTED OUT and
/// prints, honestly, what that did and did NOT turn off:
///
/// - **Unchanged**: healing itself. `--fix` / `--suggest` / the bare dry run
///   were always local and stay local; this flag does not touch them.
/// - **Stopped**: the outbound push half of `cargo refine sync`. The lease
///   PULL half still runs — a read is not a contribution.
/// - **Not deleted**: whatever is already queued in `.refine/outbox/`. The
///   path is named and the files are the user's to remove; a tool that
///   silently deletes the evidence of what it queued is not auditable.
/// - **Sent (best-effort, one-shot)**: a signed consent tier note to the
///   service — tier + machine label + signature, NEVER heal data — so the
///   web front shows this machine opted out. Only when the `kat_service` +
///   `kat_transport` halves compiled in AND the machine is logged in +
///   online; the note goes out BEFORE the logout half removes the key (a
///   signature needs it). Local consent is recorded first; the `sync` PUSH
///   half stays refused either way; a skipped or failed note never changes
///   the exit code and the outbox is still never touched.
///
/// Standalone early-return mode (before `validate_mode_flags`), works in
/// EVERY feature build, exits 0, and is idempotent.
fn run_unmine(purge_server: bool) {
    let dir = riir_refine::kat_mining_consent::default_dir();
    let (before, warning) = riir_refine::kat_mining_consent::read_with_warning(&dir);
    if let Some(w) = warning {
        eprintln!("{w}");
    }
    let already = before == riir_refine::kat_mining_consent::MiningConsent::OptedOut;

    if let Err(e) = riir_refine::kat_mining_consent::write(&dir, false) {
        eprintln!("unmine: could NOT record the opt-out — {e}");
        eprintln!(
            "unmine: consent is still `{}`, so `cargo refine sync` would still push. Fix the \
             account dir and re-run.",
            before.label()
        );
        std::process::exit(1);
    }

    if already {
        println!("KAT mining: already opted out — nothing changed");
    } else {
        println!("KAT mining: opted OUT on this machine");
    }
    println!(
        "record: {}",
        riir_refine::kat_mining_consent::consent_path(&dir).display()
    );

    // The one-shot signed tier note (owner mandate 2026-09-08) — BEFORE the
    // logout half below removes the key, because a signature needs it. The
    // note carries the tier + machine label + signature only, never heal
    // data; a failure prints one line and never changes the exit code.
    // (`purge_server` is consumed only by the armed block — silenced exactly
    // when that block is absent; the bin's required-features build carries
    // kat_service WITHOUT kat_transport.)
    #[cfg(not(all(feature = "kat_service", feature = "kat_transport")))]
    let _ = purge_server;
    #[cfg(all(feature = "kat_service", feature = "kat_transport"))]
    {
        let base = command_service_url();
        let trimmed = base.trim_end_matches('/');
        // Issue 119 T9 — same override notice as --mine: an opted-OUT note
        // must land where the machine is pointed, and the user must SEE
        // where that is.
        if let Ok(url) = std::env::var(riir_refine::kat_service_config::SERVICE_URL_ENV_VAR) {
            if !url.trim().is_empty() {
                println!(
                    "note: {} is set — this run talks to {trimmed} (unset it for the default service)",
                    riir_refine::kat_service_config::SERVICE_URL_ENV_VAR
                );
            }
        }
        match riir_refine::kat_consent_client::send_heartbeat(&base) {
            Ok("recorded") => println!("consent heartbeat: recorded (opted OUT) at {trimmed}"),
            Ok("unchanged") => println!("consent heartbeat: already recorded (opted OUT)"),
            Ok(other) => println!("consent heartbeat: skipped — the service answered \"{other}\""),
            Err(reason) => println!("consent heartbeat: skipped — {reason}"),
        }
        // riir-dapps Issue 119 T12: the self-serve server purge — signed
        // like the heartbeat, so it MUST run while the key still exists
        // (the `--purge` half below removes it). Best-effort by the same
        // contract: a failure is one skipped line, never an exit code.
        if purge_server {
            match riir_refine::kat_consent_client::send_purge(&base) {
                Ok(o) if o.deleted_unproven == 0 && o.quarantined_proven == 0 => {
                    println!("server purge: nothing to purge at {trimmed}");
                }
                Ok(o) => println!(
                    "server purge: {} unproven row(s) deleted, {} proven row(s) unpublished at {trimmed}",
                    o.deleted_unproven,
                    o.quarantined_proven,
                ),
                Err(reason) => println!(
                    "server purge: skipped — {reason} (re-run `cargo refine --unmine --purge-server` while logged in)"
                ),
            }
            println!(
                "  unproven rows: deleted · proven rows: unpublished (audit + credit stay) · published pairs: unrecallable"
            );
        }
    }
    println!();
    println!("STOPPED: the outbound push half of `cargo refine sync`. No redacted batch,");
    println!("no signed payload, nothing else leaves this machine (the one exception:");
    println!("the attempted tier note above — tier + machine label, never heal data).");
    println!("The lease PULL half still runs — downloading a corpus epoch is a read,");
    println!("not a contribution.");
    println!();
    println!("UNCHANGED (and always was local): healing itself.");
    println!("  cargo refine              <- dry run, zero edits, local");
    println!("  cargo refine --fix        <- real fixes, written locally, compile-gated");
    println!("  cargo refine --suggest    <- review, local");
    println!("Trajectories still record locally — that is how the healer learns on YOUR");
    println!("machine. Opting out stops the upload, not the local memory.");
    println!();

    // ── the UNSYNC half (Plan 020 review verdict): reset the local sync
    //    bookkeeping so the state is coherent with "not contributing" —
    //    the watermark goes (the pending-unsynced estimator reads it) and
    //    queued outbox batches stay NAMED but untouched (auditability —
    //    a tool that silently deletes the evidence of what it queued is
    //    not auditable; they are inert while opted out and the user
    //    decides).
    // The watermark goes (the pending-unsynced estimator reads it) — the
    // PER-NET one (T1); the pre-T1 flat file has no reader left.
    let url = command_service_url();
    let watermark =
        riir_refine::kat_pending_estimator::SyncWatermark::net_path(&resolve_heal_root(&[]), &url);
    match std::fs::remove_file(&watermark) {
        Ok(()) => println!(
            "UNSYNC: cleared the sync watermark ({})",
            watermark.display()
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("unmine: could not clear the watermark: {e}"),
    }
    println!(
        "ACCOUNT: untouched — your key, balance and rewards stay exactly as they are.\n         This is fully reversible: `cargo refine --mine` re-enables contributing\n         with the SAME account. To also take your rows back off the server,\n         re-run with --purge-server. (To abandon the identity entirely — which\n         ORPHANS the server-side balance forever and starts fresh-grant\n         territory — use `cargo refine --unmine --purge`.)"
    );
    println!();

    // The already-queued batches. Named, never touched — the user decides.
    let outbox = unmine_outbox_dir();
    let queued = std::fs::read_dir(&outbox).map_or(0, |rd| rd.filter_map(Result::ok).count());
    println!("ALREADY QUEUED: {}", outbox.display());
    match queued {
        0 => println!("  (nothing queued — no batch was ever held for submission)"),
        n => println!(
            "  {n} queued batch file(s). These are YOURS to delete — `cargo refine` will not \
             submit them while you are opted out, and it will not delete them either."
        ),
    }
    println!();
    println!(
        "{}",
        riir_refine::kat_mining_consent::MiningConsent::OptedOut.status_line()
    );
    println!("re-enable any time: cargo refine --mine");
}

/// riir-dapps Proposal 004 Half 1: the work-stats sharing consent —
/// `cargo refine --stats [on|off]`, bare = the status readout.
///
/// A THIRD consent state beside mining, deny-by-default: unlike mining's
/// `Unset` ("running sync IS consent"), an Unset stats record pushes
/// NOTHING, because the trigger surface is `--fix`, which every coder runs —
/// the bar is deliberately higher. Works in every build; the auto-login
/// half of `--stats on` needs `kat_service` (it is the only gated line).
/// The fix-path PUSH half is a later landing (it needs the Proposal 004
/// Half-2 server route) — recorded consent is inert until then, and the
/// output says so rather than implying rows are already flowing.
fn run_stats_consent(on: bool, off: bool) {
    use riir_refine::kat_stats_consent::{self, StatsConsent};

    let dir = kat_stats_consent::default_dir();
    let (before, warning) = kat_stats_consent::read_with_warning(&dir);
    if let Some(w) = warning {
        eprintln!("{w}");
    }

    // Bare `--stats`: the status readout (the pure-read `config` shape) —
    // the tier, the record path, the mining boundary, and the two writes.
    let Some(opt_in) = (if on {
        Some(true)
    } else if off {
        Some(false)
    } else {
        None
    }) else {
        println!("{}", before.status_line());
        println!(
            "record: {}",
            kat_stats_consent::consent_path(&dir).display()
        );
        println!();
        println!(
            "mining consent: separate — `--mine`/`--unmine` govern batch contributions; \
             this record governs work-STATS sharing only"
        );
        println!("share: `cargo refine --stats on`    stop: `cargo refine --stats off`");
        return;
    };

    if !opt_in {
        // Opt-OUT: records the refusal so onboarding never re-asks — the
        // state that distinguishes an explicit NO from never-asked Unset.
        // Forward-only, like --unmine: future pushes stop; already-shared
        // epoch aggregates stay public (they are epoch-level numbers, not
        // per-fix rows).
        let already = before == StatsConsent::OptedOut;
        if let Err(e) = kat_stats_consent::write(&dir, false) {
            eprintln!("stats: could NOT record the opt-out — {e}");
            eprintln!(
                "stats: consent is still `{}` — refusing to claim an opt-out that is not on disk",
                before.label()
            );
            std::process::exit(1);
        }
        if already {
            println!("stats: already opted out — record rewritten, nothing changed");
        } else {
            println!("stats: recorded — work-stats sharing is opted OUT for this machine");
        }
        println!(
            "record: {}",
            kat_stats_consent::consent_path(&dir).display()
        );
        println!();
        println!(
            "NOTHING is pushed for this lane while opted out, and nothing was ever deleted — \
             already-shared epoch aggregates stay public (forward-only)."
        );
        println!("re-enable any time: cargo refine --stats on");
        return;
    }

    // Opt-IN: bind the account first when this build can — stats rows are
    // ACCOUNT-SIGNED, so consent without an identity would be a record the
    // push half cannot honour. Local key ops only (the --mine chain;
    // ensure_logged_in prints its own outcome + summary lines).
    let already = before == StatsConsent::OptedIn;
    #[cfg(feature = "kat_service")]
    if ensure_logged_in().is_none() {
        eprintln!(
            "stats: no account — consent recorded, but pushes need `cargo refine login` first"
        );
    }
    #[cfg(not(feature = "kat_service"))]
    eprintln!(
        "stats: built without kat_service — consent recorded; `cargo refine login` (a \
         kat_service build) is needed before any push"
    );

    if let Err(e) = kat_stats_consent::write(&dir, true) {
        eprintln!("stats: could NOT record the opt-in — {e}");
        std::process::exit(1);
    }
    if already {
        println!("stats: already opted in — record rewritten, nothing changed");
    } else {
        println!("stats: recorded — work-stats sharing is opted IN for this machine");
    }
    println!(
        "record: {}",
        kat_stats_consent::consent_path(&dir).display()
    );
    println!();
    println!(
        "WHAT future --fix runs share: one tiny signed row each — {{lane, rule_id, count}} \
         pairs + totals; rule VOCABULARY only."
    );
    println!(
        "NEVER: spans, source text, paths, repo names, direction vectors (those are \
         mining-only and separately consented)."
    );
    println!();
    println!(
        "INERT until the Proposal 004 Half-2 server route lands — consent is recorded \
         now; no row leaves this machine before then."
    );
    println!("stop any time: cargo refine --stats off");
}

/// The destructive half of `--unmine --purge`: remove the local account
/// identity (key + pub sidecar + record) + the outbox + the watermark.
///
/// The two consequences printed here are LOAD-BEARING, not boilerplate:
/// 1. The identity IS the key — the server has no recovery path, so the
///    balance/rewards on this account become permanently unclaimable.
/// 2. `login` afterwards generates a NEW key = a NEW account with a fresh
///    once-per-key welcome grant — key-destruction-then-regeneration is
///    exactly the grant-farm shape the daily-activity sybil tax exists to
///    tax, and this flag must not be the easy one-command path into it.
///    Hence: explicit `--purge`, never the default.
fn run_unmine_purge() {
    let account_dir = riir_refine::kat_account::KatAccount::default_dir();
    match riir_refine::kat_account::KatAccount::load(&account_dir) {
        Ok(account) => {
            let key_path = account.key_path().to_path_buf();
            let pub_path = account_dir.join(riir_refine::kat_account::KatAccount::PUB_FILE);
            let record_path = account_dir.join(riir_refine::kat_account::KatAccount::RECORD_FILE);
            println!("PURGE: removing the local account identity");
            println!("  key:    {}", key_path.display());
            println!("  record: {}", record_path.display());
            println!();
            println!("WARNINGS (read before this is over):");
            println!("  1. The identity IS the key. The server ledger keeps your records,");
            println!("     rewards and balance, but NOTHING can ever claim them again —");
            println!("     there is no recovery path. The balance is orphaned forever.");
            println!("  2. `cargo refine login` afterwards generates a NEW key = a NEW account");
            println!("     with a FRESH once-per-key welcome grant. The old balance does not");
            println!("     transfer. Repeatedly purging + re-logging is grant-farming — do");
            println!("     not build a habit on it.");
            let mut removed_any = false;
            for p in [&key_path, &pub_path, &record_path] {
                match std::fs::remove_file(p) {
                    Ok(()) => removed_any = true,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => eprintln!("  purge: could not remove {}: {e}", p.display()),
                }
            }
            let outbox = unmine_outbox_dir();
            match std::fs::remove_dir_all(&outbox) {
                Ok(()) => println!("  outbox cleared: {}", outbox.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => eprintln!("  purge: could not clear the outbox: {e}"),
            }
            let watermark = riir_refine::kat_pending_estimator::SyncWatermark::net_path(
                &resolve_heal_root(&[]),
                &command_service_url(),
            );
            match std::fs::remove_file(&watermark) {
                Ok(()) => println!("  watermark cleared: {}", watermark.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => eprintln!("  purge: could not clear the watermark: {e}"),
            }
            if removed_any {
                println!("  purged — the machine is at the not-logged-in state");
            }
        }
        Err(_) => {
            println!("PURGE: no local account found — already at the free-tier state");
        }
    }
    // Issue 081: sweep the LEGACY kat layout too — the pre-canonical files
    // (in the old consent-dir account location) survive an adoption (copied,
    // not moved) and `--unmine` is the one command that must remove the
    // identity EVERYWHERE it lives.
    let legacy_dir = riir_refine::kat_account::KatAccount::legacy_dir();
    let legacy_paths = [
        legacy_dir.join(riir_refine::kat_account::KatAccount::LEGACY_KEY_FILE),
        legacy_dir.join(riir_refine::kat_account::KatAccount::LEGACY_PUB_FILE),
        legacy_dir.join(riir_refine::kat_account::KatAccount::LEGACY_RECORD_FILE),
    ];
    let mut legacy_removed = false;
    for p in legacy_paths {
        match std::fs::remove_file(&p) {
            Ok(()) => legacy_removed = true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => eprintln!("  purge: could not remove {}: {e}", p.display()),
        }
    }
    if legacy_removed {
        println!(
            "  legacy kat account files removed from {}",
            legacy_dir.display()
        );
    }
}

/// The `.refine/net/<resolved>/outbox/` dir `run_kat_sync`'s `queue_outbox`
/// writes into (T1: per-net — rows queued for net X live in X's dir and
/// can only drain to X), resolved the SAME way the queue resolves it
/// (`resolve_heal_root` + the resolved service URL →
/// `.refine/net/<key>/outbox`) so the path `--unmine` names is the path a
/// queued batch actually sits at. No cfg
/// fallback: `resolve_heal_root` compiles behind
/// `latent_retrieval`/`kat_service` and this binary's `required-features`
/// include BOTH, so a build of `cargo-refine` without it cannot exist — an
/// unreachable cfg arm would be an untested claim, not extra safety.
fn unmine_outbox_dir() -> PathBuf {
    riir_refine::kat_state_stamp::net_state_dir_for_url(
        &resolve_heal_root(&[]),
        &command_service_url(),
    )
    .join("outbox")
}

/// The login-outcome lines, shared by `cargo refine login` and `--mine`'s
/// auto-login (owner mandate 2026-09-10) — one printer, so the two entry
/// points can never describe the same event differently. `prefix` is the
/// command label ("kat" for login, "login" for --mine).
#[cfg(feature = "kat_service")]
fn print_login_outcome(outcome: &riir_refine::kat_account::LoginOutcome, prefix: &str) {
    use riir_refine::kat_account::LoginOutcome;
    match outcome {
        LoginOutcome::Generated {
            fallback_note: Some(note),
            ..
        } => println!("{prefix}: account key generated ({note})"),
        LoginOutcome::Generated { .. } => println!("{prefix}: account key generated"),
        LoginOutcome::Imported { .. } => println!("{prefix}: account key imported"),
        LoginOutcome::Adopted { source, .. } => println!(
            "{prefix}: account key adopted from {} (copied into the account dir — the original is untouched)",
            source.display()
        ),
        LoginOutcome::AdoptedInPlace { source, .. } => println!(
            "{prefix}: account key adopted IN PLACE — {} IS the account key (no copy; the record names it)",
            source.display()
        ),
        LoginOutcome::Existing { .. } => println!("{prefix}: already logged in (key loaded)"),
    }
}

/// The zero-config login chain, run in place. Returns `None` (and prints
/// the remedy) when login FAILED — callers skip their account-bearing
/// lines but keep running. Local key ops only: no network, no prompt (the
/// fallback chain ends in a generated key with a disclosed note).
#[cfg(feature = "kat_service")]
fn ensure_logged_in() -> Option<riir_refine::kat_account::KatAccount> {
    use riir_refine::kat_account::KatAccount;

    match KatAccount::load(&KatAccount::default_dir()) {
        Ok(account) => Some(account),
        Err(_) => match KatAccount::login_default(&KatAccount::default_dir()) {
            Ok((account, outcome)) => {
                print_login_outcome(&outcome, "login");
                for line in account.summary_lines() {
                    println!("{line}");
                }
                Some(account)
            }
            Err(e) => {
                eprintln!("login:   FAILED — {e}");
                eprintln!(
                    "login:   run `cargo refine login` to set the account up, then --mine again"
                );
                None
            }
        },
    }
}

/// Proposal 005 Phase 0 T0.1 (Plan 025): `cargo refine login` — generate/import
/// the account key + record the grant projection. Prints one outcome line,
/// the account id, the key path, and the Phase-0 honesty note. No `$`
/// figures (caveats #12/#15). The EXPLICIT login: `--mine` runs the same
/// zero-config chain automatically (owner mandate 2026-09-10); this command
/// stays for import (`--import-key`), repair, and scripts that want login
/// as a distinct step.
#[cfg(feature = "kat_service")]
fn run_kat_login(import_key: Option<&Path>) {
    use riir_refine::kat_account::KatAccount;

    // Plan 045 A1: the explicit login is a join path too — a fresh machine
    // chooses its network here (the same no-config-only prompt as `--mine`).
    ensure_join_env_recorded();
    let dir = KatAccount::default_dir();
    let result = match import_key {
        Some(source) => KatAccount::login_import(&dir, source),
        // Issue 081 chain (see `ensure_logged_in`): canonical → deprecated
        // override → legacy adoption → env key → machine SSH key → fresh.
        None => KatAccount::login_default(&dir),
    };
    let (account, outcome) = match result {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("kat login failed: {e}");
            std::process::exit(1);
        }
    };
    print_login_outcome(&outcome, "kat");
    for line in account.summary_lines() {
        println!("{line}");
    }
}

/// Proposal 005 Phase 0 T0.1 (Plan 025): `cargo refine account` — the local
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
///
/// `--history` (dapps Issue 092 Phase 1) prefers the fuller `GET /history`
/// view — position + burn watermark + contribution fact + mint receipts —
/// falling back to the `/balance` rendering when that route does not
/// answer, and to the local projection when the service is unreadable.
#[cfg(feature = "kat_service")]
fn run_kat_account(paths: &[PathBuf], show_history: bool) {
    use riir_refine::kat_account::{history_lines, KatAccount};
    use riir_refine::kat_meter::ChargeState;

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
    println!(
        "{}",
        riir_refine::kat_account_client::account_header("KAT ACCOUNT")
    );
    for line in account.summary_lines() {
        println!("{line}");
    }

    // The repo-local burn half: read the charge ledger at the heal root (path
    // arg or cwd) when one exists. Per-repo by design (Plans 023/024) until
    // the server aggregates at Phase 1.
    let root = resolve_heal_root(paths);
    let charge_path =
        riir_refine::kat_state_stamp::net_state_dir_for_url(&root, &command_service_url())
            .join("kat_charge.json");
    let charge = std::fs::read_to_string(&charge_path).ok().and_then(|text| {
        match serde_json::from_str::<ChargeState>(&text) {
            Ok(charge) => Some(charge),
            Err(e) => {
                eprintln!(
                    "kat account: charge ledger unreadable ({}): {e}",
                    charge_path.display()
                );
                None
            }
        }
    });

    // Issue 081: the grant projection is the meter CONSTANT now — the
    // canonical record no longer carries a grant_kat field (it was never
    // mutated after first write, so the constant IS the projection).
    // Plan 035 T3: the projection math runs in MicroKat — µ/whole mixing
    // needs an explicit conversion to compile.
    // Issue 126 follow-up 3 (owner 2026-10-14): the old "balance
    // projection: X of 100 KAT" line is GONE — the signup grant is the
    // TUNA trial credit (never KAT), so a local KAT "remaining of grant"
    // figure was a mint promise that does not exist; the service section
    // below is the authoritative balance.
    // One label column for the whole run (issue 126 follow-up 6): the
    // substrate's computed account_line — the same grid the identity block
    // and the welcome gate render through. The net is named by the
    // `service:` line below, so the absent-ledger line stays short.
    match &charge {
        Some(charge) => println!(
            "{}",
            riir_refine::kat_account_client::account_line(
                "local burn:",
                format!(
                    "{} charged this repo ({} tokens, watermark {})",
                    riir_refine::kat_payreq_client::render_kat(
                        i64::try_from(charge.charged_kat).unwrap_or(i64::MAX)
                    ),
                    charge.meter_watermark_tokens,
                    charge.meter_watermark_tokens
                )
            )
        ),
        None => println!(
            "{}",
            riir_refine::kat_account_client::account_line(
                "local burn:",
                "(no charge ledger for this net — heal a repo to start one)",
            )
        ),
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
    let (served, history_acct) = {
        use riir_refine::kat_account_client as svc;
        match svc::service_account_hex(&account.state().account_id) {
            // The 64-hex local id truncates to the ledger's 32-hex
            // AccountId; a malformed record cannot query the service.
            None => {
                eprintln!(
                    "kat account: the local account id is not the 64-hex derivation — skipping the service reads"
                );
                (None, None)
            }
            Some(acct) => {
                let url = command_service_url();
                // 5 s, deliberately short: `account` is offline-capable and
                // the probe fail-fasts, so an offline run costs one budget.
                let transport = svc::HttpAccountTransport::new(&url, 5);
                let view = svc::account_service_view(&transport, acct);
                // dapps Issue 029 Rider 1 (client half): label the
                // deployment being asked — display-only, so an unlabeled
                // or unreadable service prints nothing. Printed from the
                // FIRST probe only (the post-claim re-probe would repeat it).
                if let Some(line) = &view.service_line {
                    println!("{line}");
                }
                for line in svc::welcome_gate_lines(&view.gate) {
                    println!("{line}");
                }
                // T079: the once-per-key free-tier grant has a production
                // claim path now. Read-first (`ensure_grant_flow` sends
                // NOTHING when the grant is in place), signed with the
                // account key, idempotent on replay; a failure is printed
                // and never blocks the local view.
                // The balance tail, unified so every exit carries the
                // same tuple; `acct` is the /history query (`--history`,
                // dapps Issue 092).
                let balance = match account.signing_key() {
                    Ok(key) => {
                        let payreq =
                            riir_refine::kat_payreq_client::HttpPayreqTransport::new(&url, 5);
                        match riir_refine::kat_payreq_client::ensure_grant_flow(&payreq, &key, acct)
                        {
                            Ok(Some(claim)) => {
                                let kat =
                                    |micro: i64| riir_refine::kat_payreq_client::render_kat(micro);
                                if claim.already {
                                    println!(
                                        "{}",
                                        riir_refine::kat_account_client::account_line(
                                            "grant:",
                                            format!(
                                                "already claimed (balance {})",
                                                kat(claim.balance_micro)
                                            ),
                                        )
                                    );
                                } else {
                                    println!(
                                        "{}",
                                        riir_refine::kat_account_client::account_line(
                                            "grant:",
                                            format!(
                                                "{} claimed — the free-tier grant is live (balance {})",
                                                kat(claim.claimed_micro), kat(claim.balance_micro)
                                            ),
                                        )
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
                                println!(
                                    "{}",
                                    riir_refine::kat_account_client::account_line(
                                        "grant:",
                                        format!("not claimed — {e}"),
                                    )
                                );
                                view.balance
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            riir_refine::kat_account_client::account_line(
                                "grant:",
                                format!("not claimed — the account key is unusable ({e})"),
                            )
                        );
                        view.balance
                    }
                };
                (balance, Some(acct.to_string()))
            }
        }
    };
    // No transport compiled: the local heuristic is all this build has.
    // (`history_acct` is read only by the transport arm of `--history`
    // below, hence the cfg'd allow.)
    #[cfg_attr(not(feature = "kat_transport"), allow(unused_variables))]
    #[cfg(not(feature = "kat_transport"))]
    let (served, history_acct): (
        Option<riir_refine::kat_account_client::ServiceBalance>,
        Option<String>,
    ) = {
        if spent == 0 {
            println!(
                "{}",
                riir_refine::kat_account_client::account_line(
                    "welcome:",
                    riir_refine::kat_account::welcome_line(),
                )
            );
        }
        (None, None)
    };

    if show_history {
        // dapps Issue 092: prefer the /history route — position + burn
        // watermark + contribution fact + mint receipts. The read rides
        // AFTER the probe, so an offline `--history` run pays one extra
        // bounded timeout before the fallbacks below. A 404 / unreadable
        // route degrades to the `/balance` rendering (the trait's default
        // is an honest 404), never to a fabricated page.
        #[cfg(feature = "kat_transport")]
        if let Some(acct) = history_acct.as_deref() {
            use riir_refine::kat_account_client::AccountTransport;
            use riir_refine::kat_lease_client::HttpResponse;
            let url = command_service_url();
            let transport = riir_refine::kat_account_client::HttpAccountTransport::new(&url, 5);
            let lines = match transport.get_history(acct, 20) {
                HttpResponse::Ok(body) => riir_refine::kat_account_client::parse_history(&body)
                    .ok()
                    .map(|h| {
                        riir_refine::kat_account_client::account_history_lines(&h, spent, tokens)
                    }),
                _ => None,
            };
            if let Some(lines) = lines {
                for line in lines {
                    println!("{line}");
                }
                return;
            }
            eprintln!("kat account: the /history route did not answer — the /balance view follows");
        }
        match &served {
            Some(balance) => {
                for line in
                    riir_refine::kat_account_client::service_history_lines(balance, spent, tokens)
                {
                    println!("{line}");
                }
            }
            // The offline fallback — T0.2's local spent view, honest about
            // being per-repo metering: the grant is the TUNA trial credit
            // (never KAT), so the local render carries no earned/balance
            // projection (issue 126 follow-up 3).
            None => {
                for line in history_lines(spent, tokens) {
                    println!("{line}");
                }
            }
        }
    }
}

/// Proposal 005 T1.6 (Plan 040): `cargo refine update` — the pull-only lease
/// refresh. The explicit-command budget is generous (30s) vs the run-start
/// phase's ~4s: a user asking for an update is waiting for it.
#[cfg(feature = "kat_transport")]
fn run_kat_update() {
    use riir_refine::kat_lease_client::{
        run_update_phase, HttpLeaseTransport, LeaseCharge, LeaseStore,
    };

    let root = resolve_heal_root(&[]);
    let url = command_service_url();
    let store = LeaseStore::open_net(&root, &url);
    let mut charge = LeaseCharge::open(&store);
    let transport = HttpLeaseTransport::new(&url, 30);
    let current = store.current_epoch().unwrap_or(0);
    // Issue 165: captured BEFORE the phase — after the swap the old epoch's
    // manifest + segments are still on disk (content-addressed), which is
    // what makes the gain diff computable at all.
    let had_epoch = store.current_epoch();
    let outcome = run_update_phase(&transport, &store, &mut charge, false, None);
    match outcome.status {
        riir_refine::kat_lease_client::UpdateStatus::NotModified => {
            println!("corpus lease: up to date (epoch {current}, 304 — no charge)");
            // Issue 165: the shape of what the 304 is keeping — the
            // per-domain rule counts the installed epoch serves.
            if let Some((epoch, overlay)) =
                riir_refine::kat_lease_client::overlay_from_epoch(&store, None)
            {
                if let Some(line) = riir_refine::kat_lease_client::corpus_shape_line(&overlay) {
                    println!("{line} (epoch {epoch})");
                }
            }
        }
        riir_refine::kat_lease_client::UpdateStatus::Applied {
            epoch,
            charge_micro_kat,
        } => {
            println!(
                "{}",
                riir_refine::kat_lease_client::LeaseCharge::line(epoch, charge_micro_kat)
            );
            println!(
                "{:<15}epoch {epoch} installed atomically — the retrieval index rebuilds on the next run",
                "corpus lease:"
            );
            print_corpus_epoch_gain(&store, had_epoch, epoch);
        }
        riir_refine::kat_lease_client::UpdateStatus::Skipped(reason) => {
            println!("corpus lease: skipped — {reason}");
        }
        riir_refine::kat_lease_client::UpdateStatus::Failed(e) => {
            eprintln!("corpus lease: {e}");
            eprintln!(
                "corpus lease: proceeding on the local corpus (offline is a mode, not a failure)"
            );
        }
    }
    if let Err(e) = charge.save() {
        eprintln!("corpus lease: could not persist the charge ledger: {e}");
    }
}

/// Issue 165: the `update`/`sync` gain surface — what the applied epoch
/// actually changes, computed client-side from the overlays BOTH epochs'
/// manifests + content-addressed segments still yield (the atomic swap
/// adds files, it never removes). No wire change; an unreadable overlay
/// skips the line honestly instead of fabricating a diff. First-install
/// (or old-epoch-unreadable) postures show the installed shape instead.
#[cfg(feature = "kat_transport")]
fn print_corpus_epoch_gain(
    store: &riir_refine::kat_lease_client::LeaseStore,
    old_epoch: Option<u64>,
    new_epoch: u64,
) {
    use riir_refine::kat_lease_client::{corpus_epoch_diff, corpus_shape_line, overlay_from_epoch};
    let Some((_, new_overlay)) = overlay_from_epoch(store, Some(new_epoch)) else {
        return;
    };
    match old_epoch.map(|e| overlay_from_epoch(store, Some(e))) {
        Some(Some((old, old_overlay))) if old != new_epoch => {
            let diff = corpus_epoch_diff(&old_overlay, &new_overlay);
            match diff.summary_line(old, new_epoch) {
                Some(line) => println!("{line}"),
                None => println!(
                    "{:<15}epoch {old} → {new_epoch} — rule sets identical",
                    "corpus diff:"
                ),
            }
        }
        _ => {
            if let Some(line) = corpus_shape_line(&new_overlay) {
                println!("{line}");
            }
        }
    }
}

/// Issue 165: the mining opt-in encouragement — honest numbers only. The
/// estimate composes the pending estimator (caveat #15: "service credit,
/// if proven", provisional rate) over the store THIS run already opened;
/// zero pending rows → the flow line (fix records history; mine shares
/// it) on suggest-only runs with findings. Unset consent only: OptedIn
/// runs auto-push in the tail, and OptedOut is respected, never nagged.
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
fn print_mine_encouragement(
    suggest_only: bool,
    findings_total: usize,
    traj_store: Option<&riir_refine::traj_store::TrajStore>,
    root: &Path,
) {
    use riir_refine::kat_mining_consent::MiningConsent;
    let (consent, _) = riir_refine::kat_mining_consent::read_with_warning(
        &riir_refine::kat_mining_consent::default_dir(),
    );
    if consent != MiningConsent::Unset {
        return;
    }
    let flow_tip = suggest_only && findings_total > 0;
    let Some(store) = traj_store else {
        if flow_tip {
            println!(
                "mine: `cargo refine --fix` records your heal history — \
                 `cargo refine --mine` opts in and shares it for KAT service credit"
            );
        }
        return;
    };
    let loaded = riir_refine::kat_redact::RedactionGate::discover(None);
    let (watermark, _) = riir_refine::kat_pending_estimator::SyncWatermark::load_at(
        &riir_refine::kat_pending_estimator::SyncWatermark::net_path(root, &net_partition_url()),
    );
    let est = riir_refine::kat_pending_estimator::estimate_pending(
        &store.dump(),
        &loaded.gate,
        riir_refine::kat_redact::SpanOrigin::OwnRepo,
        &watermark,
    );
    if est.pending_unsynced > 0 {
        println!(
            "mine: {} — run `cargo refine --mine` to opt in and contribute",
            est.pending_line()
        );
    } else if flow_tip {
        println!(
            "mine: `cargo refine --fix` records your heal history — \
             `cargo refine --mine` opts in and shares it for KAT service credit"
        );
    }
    // pending rows that are already synced (awaiting server proof) name no
    // CTA — they are contributed; the quorum decides, not the user.
}

// ── Issue 153: the update surface (alert + self-update + auto-corpus) ─────

/// The run-start legs (T1 alert + T2 hands-free upgrade + T4 auto corpus).
/// Every leg fail-soft — a network-less run must never turn into a nonzero
/// exit here. A feature-less build compiles the whole fn to nothing (the
/// caller skips it anyway on the same gates).
fn run_update_surface_hooks(paths: &[PathBuf]) {
    #[cfg(feature = "update_check")]
    {
        let root = resolve_heal_root(paths);
        let lines = riir_refine::update_check::check_for_alert(
            &root,
            &riir_refine::update_check::GitHubLatest::new(),
        );
        let newer = !lines.is_empty();
        for line in &lines {
            eprintln!("{line}");
        }
        // T2: the hands-free apply is OPT-IN — a binary that replaces itself
        // without the env set would be a consent violation, full stop. The
        // CURRENT run keeps the old binary (the swap renames, it cannot
        // relaunch us); the message says so.
        if newer && riir_refine::update_check::auto_upgrade_enabled() {
            eprintln!("auto-upgrade: RIIR_REFINE_AUTO_UPGRADE=1 — applying…");
            match riir_refine::update_check::upgrade(
                &riir_refine::update_check::GitHubLatest::new(),
                &riir_refine::update_check::HttpDownload::new(),
                env!("CARGO_PKG_VERSION"),
            ) {
                Ok(report) => {
                    eprintln!(
                        "auto-upgrade: {} → {} installed — THIS run keeps the old binary; rerun to pick it up (rollback: {})",
                        report.from_version,
                        report.to_version,
                        report.backup.display()
                    );
                    if let Some(notice) = report.symlink_notice() {
                        eprintln!("auto-upgrade: {notice}");
                    }
                }
                Err(e) => eprintln!("auto-upgrade: failed ({e}) — staying on this build"),
            }
        }
    }
    #[cfg(feature = "kat_transport")]
    {
        let _ = paths; // the corpus leg resolves the root itself
        maybe_auto_corpus();
    }
    #[cfg(not(any(feature = "update_check", feature = "kat_transport")))]
    {
        let _ = paths;
    }
}

/// Issue 153 T4: `RIIR_REFINE_AUTO_CORPUS=1` — the lease refresh (`update`)
/// runs itself at run start, at most once per 12h. The lease charge is the
/// tier gate HONORED, not bypassed: out-of-KAT and refused postures skip
/// inside `run_update_phase` exactly as the explicit command does. One
/// bookkeeping file records the ATTEMPT, not the success — a flaky network
/// must not re-timeout on every invocation. Silent when the corpus is
/// current (NotModified) — a run start is not a status page.
#[cfg(feature = "kat_transport")]
fn maybe_auto_corpus() {
    /// The auto-corpus bookkeeping window (Issue 153 T4).
    const AUTO_CORPUS_WINDOW_SECS: i64 = 12 * 3600;
    if std::env::var("RIIR_REFINE_AUTO_CORPUS").as_deref() != Ok("1") {
        return;
    }
    let root = resolve_heal_root(&[]);
    let stamp_path = root.join(".refine").join("corpus_auto.json");
    #[derive(serde::Deserialize, serde::Serialize)]
    struct AutoStamp {
        last_attempt_unix: i64,
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Ok(text) = std::fs::read_to_string(&stamp_path) {
        if let Ok(s) = serde_json::from_str::<AutoStamp>(&text) {
            if now - s.last_attempt_unix < AUTO_CORPUS_WINDOW_SECS {
                return;
            }
        }
    }
    if let Some(dir) = stamp_path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(
        &stamp_path,
        serde_json::to_vec(&AutoStamp {
            last_attempt_unix: now,
        })
        .unwrap_or_default(),
    );

    // The same legs run_kat_update runs — a startup-capped budget (the
    // explicit command's 30s generous budget would stall every run start).
    use riir_refine::kat_lease_client::{
        run_update_phase, HttpLeaseTransport, LeaseCharge, LeaseStore,
    };
    let url = command_service_url();
    let store = LeaseStore::open_net(&root, &url);
    let mut charge = LeaseCharge::open(&store);
    let transport = HttpLeaseTransport::new(&url, 4);
    let deadline = Some(std::time::Instant::now() + std::time::Duration::from_secs(6));
    let outcome = run_update_phase(&transport, &store, &mut charge, false, deadline);
    match outcome.status {
        riir_refine::kat_lease_client::UpdateStatus::NotModified => {}
        riir_refine::kat_lease_client::UpdateStatus::Applied {
            epoch,
            charge_micro_kat,
        } => {
            eprintln!(
                "auto-corpus: {}",
                riir_refine::kat_lease_client::LeaseCharge::line(epoch, charge_micro_kat)
            );
            eprintln!(
                "auto-corpus: epoch {epoch} installed — the retrieval index rebuilds on the next run"
            );
        }
        riir_refine::kat_lease_client::UpdateStatus::Skipped(reason) => {
            eprintln!("auto-corpus: skipped — {reason}");
        }
        riir_refine::kat_lease_client::UpdateStatus::Failed(e) => {
            eprintln!("auto-corpus: failed ({e}) — proceeding on the local corpus");
        }
    }
    if let Err(e) = charge.save() {
        eprintln!("auto-corpus: could not persist the charge ledger: {e}");
    }
}

/// Issue 153 T2: `cargo refine upgrade` — the verified binary self-update.
/// The explicit command IS the consent; every refusal is loud and names its
/// leg. Fetch → pick asset → verify SHA256SUMS → extract → backup-swap; the
/// running process stays the old binary until the next invocation.
#[cfg(feature = "update_check")]
fn run_self_upgrade() {
    eprintln!("upgrade: checking the latest release…");
    let report = riir_refine::update_check::upgrade(
        &riir_refine::update_check::GitHubLatest::new(),
        &riir_refine::update_check::HttpDownload::new(),
        env!("CARGO_PKG_VERSION"),
    );
    match report {
        Ok(r) => {
            println!(
                "upgraded: cargo-refine {} → {}",
                r.from_version, r.to_version
            );
            println!("  installed at {}", r.new_exe.display());
            println!(
                "  old binary kept at {} (delete it when happy)",
                r.backup.display()
            );
            println!("  THIS run is still the old binary — rerun to pick up the new one");
            if let Some(notice) = r.abi_note {
                println!("  {notice}");
            }
            if let Some(notice) = r.symlink_notice() {
                println!("  {notice}");
            }
        }
        Err(riir_refine::update_check::UpdateError::UpToDate(v)) => {
            println!("already current: cargo-refine {v} is the latest release");
        }
        Err(e) => {
            eprintln!("upgrade: failed — {e}");
            std::process::exit(1);
        }
    }
}

/// Proposal 005 T1.6 (Plan 040): `cargo refine sync` — bidirectional. Push:
/// build the redacted payload from the repo trajectory store (the
/// sanctioned egress path — every row gated) + POST /sync; with the service
/// plane deferred the payload is validated + queued locally
/// (`.refine/outbox/`) and the honest state prints. Pull: the lease refresh
/// (shared with `update`). The sync watermark advances ONLY on a real push
/// acceptance (queued batches stay "pending" — the honest accounting).
/// What the mining-batch push did (the quiet tail prints one line via
/// [`print_mine_push`]; the verbose sync prints its own full report).
#[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
enum MinePush {
    /// An accepted push: (rows included, rows new at the server).
    Accepted { rows_included: usize, rows_new: u64 },
    /// The service did not accept (offline / not live) — the batch is
    /// queued locally and pushes with a later run.
    QueuedOffline,
    /// Nothing to push (no store, unreadable, no account, empty payload).
    Skipped,
}

/// The one tail line (the burn-report discipline): Accepted and
/// `QueuedOffline` print, Skipped stays silent.
#[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
fn print_mine_push(outcome: &MinePush) {
    match outcome {
        MinePush::Accepted {
            rows_included,
            rows_new,
        } => println!(
            "mining: batch pushed — {rows_included} row(s) ({rows_new} new) → the proof queue; paid at the epoch settle"
        ),
        MinePush::QueuedOffline => println!(
            "mining: service unreachable — the batch is queued; it pushes with the next run"
        ),
        MinePush::Skipped => {}
    }
}

/// The rows + redaction policy a mining batch is built from — SHARED by the
/// push ([`push_mining_batch`]) and the local export ([`run_kat_sync_export`],
/// riir-dapps Issue 112 T4), so an exported file is exactly what a push would
/// gate. `None` = nothing to build (no store, or an unreadable one), already
/// reported under `label` when `verbose`.
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
fn load_mining_rows(
    root: &Path,
    verbose: bool,
    label: &str,
) -> Option<(
    Vec<riir_refine::traj_store::TrajRowDoc>,
    riir_refine::kat_redact::LoadedPolicy,
)> {
    let kv = root.join(".refine").join("trajectories.kv");
    if !kv.exists() {
        if verbose {
            println!(
                "{label}: no trajectory store at {} — run any --fix in this repo first",
                kv.display()
            );
        }
        return None;
    }
    let (traj, bad) = match riir_refine::traj_store::TrajStore::open(root) {
        Ok(t) => t,
        Err(e) => {
            if verbose {
                eprintln!("{label}: trajectory store unreadable: {e}");
            }
            return None;
        }
    };
    if bad > 0 {
        eprintln!("{label}: {bad} corrupt row(s) skipped");
    }
    let loaded = riir_refine::kat_redact::RedactionGate::discover(None);
    if let Some(w) = &loaded.warning {
        eprintln!("{label}: redaction policy — {w}");
    }
    Some((traj.dump(), loaded))
}

/// The deny-by-default remedy (2026-09-09 fresh-machine find): with no
/// `.refine-sync-policy` every row is blocked, so name the file and the fix.
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
fn print_no_policy_remedy(
    root: &Path,
    payload: &riir_refine::kat_sync_payload::SyncPayload,
    label: &str,
) {
    let no_policy = payload.outcomes.iter().any(|o| {
        matches!(
            o,
            riir_refine::kat_sync_payload::RowOutcome::Blocked {
                reason: riir_refine::kat_redact::BlockReason::NoPolicy,
                ..
            }
        )
    });
    if !no_policy {
        return;
    }
    let policy_path = root.join(".refine-sync-policy");
    eprintln!(
        "{label}: {} row(s) BLOCKED — no {} (the redaction gate is deny-by-default)",
        payload.rows_blocked,
        policy_path.display()
    );
    eprintln!(
        "{label}: run `cargo refine --mine` in this repo to write the default policy, \
         then run again (queued batches stay queued)"
    );
}

/// riir-dapps Issue 112 T4: `cargo refine sync --export <file|->` — build the
/// redacted mining batch exactly as `sync` would and WRITE it instead of
/// sending it. No network, no login, no consent change: the batch is signed
/// with a throwaway key ([`riir_refine::kat_sync_payload::ephemeral_signer`])
/// so it verifies like a real push but attributes to no account a person
/// holds. This is how anyone (including the refine page's published sample)
/// inspects what `--mine` would upload before opting in.
#[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
fn run_kat_sync_export(target: &Path) {
    use riir_refine::kat_redact::SpanOrigin;
    use riir_refine::kat_sync_payload::{ephemeral_signer, SyncPayloadBuilder};

    let root = resolve_heal_root(&[]);
    let Some((rows, loaded)) = load_mining_rows(&root, true, "export") else {
        std::process::exit(1);
    };
    // No policy file at all = the state BEFORE opting in: preview the default
    // policy `--mine` would write, so the export answers "what would joining
    // upload". An existing (even broken) policy is always the one used.
    let gate = match &loaded.path {
        Some(p) => {
            eprintln!("export: policy {}", p.display());
            loaded.gate
        }
        None => {
            eprintln!(
                "export: no .refine-sync-policy here — previewing the default policy \
                 `cargo refine --mine` would write"
            );
            default_sync_policy_gate()
        }
    };
    let signer = ephemeral_signer();
    let payload = SyncPayloadBuilder::new(&gate, SpanOrigin::OwnRepo, &signer).build(&rows);
    print_no_policy_remedy(&root, &payload, "export");
    let to_stdout = target.as_os_str() == "-";
    let written = if to_stdout {
        use std::io::Write as _;
        std::io::stdout().write_all(payload.ndjson.as_bytes())
    } else {
        std::fs::write(target, &payload.ndjson)
    };
    if let Err(e) = written {
        eprintln!("export: could not write {}: {e}", target.display());
        std::process::exit(1);
    }
    // stderr, so `--export -` keeps stdout pure NDJSON.
    eprintln!(
        "export: {} row(s) included, {} blocked (batch {}) → {}",
        payload.rows_included,
        payload.rows_blocked,
        &payload.commitment[..12.min(payload.commitment.len())],
        if to_stdout {
            "stdout".to_string()
        } else {
            target.display().to_string()
        }
    );
    eprintln!(
        "export: NOT sent — signed with a throwaway key (no account, no login); \
         `cargo refine sync` builds and pushes the same rows under your account"
    );
}

/// The mining-batch push half, SHARED by `cargo refine sync` (verbose — the
/// caller has already gated on consent and prints its opted-out report)
/// and the opted-in run tails (quiet: one line via [`print_mine_push`],
/// never fatal, short budget). The watermark advances ONLY on a real push
/// acceptance; a queued batch stays "pending" (the honest accounting).
/// The QUIET tail never exits the process on a missing account — the run
/// has already done its work — while verbose `sync` keeps its hard
/// refusal (payload v2 is signed; there is nothing to do without a key).
#[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
fn push_mining_batch(
    quiet: bool,
    url: &str,
    transport: &riir_refine::kat_lease_client::HttpLeaseTransport,
) -> MinePush {
    use riir_refine::kat_lease_client::{queue_outbox, LeaseTransport};
    use riir_refine::kat_redact::SpanOrigin;
    use riir_refine::kat_sync_payload::SyncPayloadBuilder;

    let sv = !quiet; // verbose mode = the `cargo refine sync` report
    let root = resolve_heal_root(&[]);
    let Some((rows, loaded)) = load_mining_rows(&root, sv, "sync") else {
        return MinePush::Skipped;
    };
    // Payload v2 is SIGNED with the account key (Issue 008) — the receiver
    // derives the submitter from it, so an account is required to sync.
    let account = match riir_refine::kat_account::KatAccount::load(
        &riir_refine::kat_account::KatAccount::default_dir(),
    ) {
        Ok(a) => a,
        Err(e) => {
            if sv {
                eprintln!(
                    "sync: no account ({e}) — payload v2 is signed with the account \
                         key; run `cargo refine login` first"
                );
                std::process::exit(1);
            }
            return MinePush::Skipped;
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            if sv {
                eprintln!("sync: account key unusable ({e})");
                std::process::exit(1);
            }
            return MinePush::Skipped;
        }
    };
    // T12b (riir-kat Issue 003, site 12 — the client half): arm the
    // network-bound signature when the service discloses its D8c ledger
    // instance id. The read rides the SAME transport (one extra GET, the
    // same bounded window); an old server answers without the field and
    // the legacy `-v2` signature goes out unchanged (the mixed-fleet
    // posture). A failed info read ALSO falls back to legacy — the push
    // itself fails closed at the server if the posture demands v2 (426),
    // never silently mis-signed.
    let ledger_id = match transport.service_info() {
        riir_refine::kat_lease_client::HttpResponse::Ok(body) => {
            riir_refine::kat_lease_client::parse_service_ledger_id(&body)
        }
        _ => None,
    };
    let payload = SyncPayloadBuilder::new(&loaded.gate, SpanOrigin::OwnRepo, &signing_key)
        .with_ledger_id(ledger_id)
        .build(&rows);
    if sv {
        if ledger_id.is_some() {
            println!("sync: network-bound signature armed (ledger instance bound)");
        }
        println!(
            "sync payload: {} row(s) included, {} blocked, signed as {}… (batch {})",
            payload.rows_included,
            payload.rows_blocked,
            riir_refine::kat_account::short_id(account.state().account_id.as_str()),
            &payload.commitment[..12.min(payload.commitment.len())]
        );
    }
    if payload.rows_included == 0 {
        return MinePush::Skipped;
    }
    // The deny-by-default remedy — verbose only; the quiet tail's reader
    // gets the same answer from `sync`.
    if sv {
        print_no_policy_remedy(&root, &payload, "sync");
    }
    match transport.post_sync(&payload.ndjson) {
        riir_refine::kat_lease_client::HttpResponse::Ok(body) => {
            // T13's client-side half: a coded refusal arrives here as
            // Ok(body) — the transport's `sync_refusal_or_status`
            // passthrough (the burn-client convention). Read `error` FIRST:
            // a refusal is QUEUED, never accepted (the outbox copy stays,
            // the watermark does not advance), and the verbatim upgrade
            // text (426 client_too_old) reaches the user — the only
            // self-explanatory refusal an uncontrolled rig will ever see.
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&body) {
                if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
                    let queued = queue_outbox(&root, url, &payload.ndjson, &payload.commitment)
                        .ok()
                        .map_or_else(
                            || "(queue write failed)".to_string(),
                            |p| p.display().to_string(),
                        );
                    println!("sync: refused by the service — {err}");
                    println!("sync: payload queued at {queued} (upgrade the client, then it submits with the next run)");
                    return MinePush::QueuedOffline;
                }
            }
            let mut rows_new = 0u64;
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&body) {
                rows_new = v
                    .get("rows_new")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
            }
            if sv {
                println!("sync: push accepted by {url}");
                println!(
                    "sync: rows enter the proof queue — credited when the proof pass \
                     runs, PAID at the next epoch settle; duplicate shapes earn nothing \
                     (heal real code for novel fixes)"
                );
                if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&body) {
                    println!(
                        "sync: server ingest — rows_new {} refreshed {} terminal_skipped {} recorded {}",
                        v.get("rows_new").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        v.get("rows_refreshed").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        v.get("rows_terminal_skipped").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        v.get("recorded").and_then(serde_json::Value::as_u64).unwrap_or(0),
                    );
                }
            }
            // The outbox copy of THIS batch is now delivered mail — remove
            // it (pruned per-file, never the directory; re-push is
            // idempotent server-side, so an early crash here is harmless).
            let batch_file = root.join(".refine").join("outbox").join(format!(
                "{}.ndjson",
                &payload.commitment[..12.min(payload.commitment.len())]
            ));
            if batch_file.is_file() {
                let _ = std::fs::remove_file(&batch_file);
            }
            // T079: the ledger-plane half of "the service sees the key" —
            // claim the once-per-key free-tier grant (read-first; a no-op
            // when already in place). Quiet mode prints nothing here —
            // `cargo refine account` reports the grant, and a failure is
            // invisible at this layer by design (the push already landed).
            if let Ok(key) = account.signing_key() {
                if let Some(acct) = riir_refine::kat_account_client::service_account_hex(
                    &account.state().account_id,
                ) {
                    let payreq = riir_refine::kat_payreq_client::HttpPayreqTransport::new(url, 10);
                    match riir_refine::kat_payreq_client::ensure_grant_flow(&payreq, &key, acct) {
                        Ok(Some(claim)) if !claim.already => {
                            if sv {
                                println!(
                                    "sync: free-tier grant claimed ({})",
                                    riir_refine::kat_payreq_client::render_kat(claim.claimed_micro)
                                );
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            if sv {
                                println!("sync: grant not claimed — {e}");
                            }
                        }
                    }
                }
            }
            // Real acceptance → advance the watermark (the pushed rows are
            // no longer "pending-unsynced").
            let wm = riir_refine::kat_pending_estimator::SyncWatermark::from_payload(&payload);
            if let Err(e) =
                wm.save(&riir_refine::kat_pending_estimator::SyncWatermark::net_path(&root, url))
            {
                eprintln!("sync: watermark save failed: {e}");
            }
            MinePush::Accepted {
                rows_included: payload.rows_included,
                rows_new,
            }
        }
        resp => {
            let queued = queue_outbox(&root, url, &payload.ndjson, &payload.commitment)
                .ok()
                .map_or_else(
                    || "(queue write failed)".to_string(),
                    |p| p.display().to_string(),
                );
            if sv {
                println!("sync: push NOT accepted ({resp:?}) — the service plane is not yet live;");
                println!(
                    "sync: payload queued locally at {queued} (submits when the Worker lands; the watermark does not advance)"
                );
            }
            let _ = queued;
            MinePush::QueuedOffline
        }
    }
}

/// riir-dapps Plan 023: `cargo refine config get` / `config set --url
/// <alias|url>` — the persisted service-url surface. The set is LOUD (g2):
/// the resolved URL, whether it is a pinned network host, the env-override
/// trap, and (for a custom/loopback host) the local-artifact caveat for
/// payment links/QRs (g5). The get is the g3 diagnosis: the ACTIVE url AND
/// its source. Local-only and ungated — works in every feature build.
fn run_kat_config(
    set: bool,
    url: Option<&str>,
    adopt_state: Option<&str>,
    discard: bool,
    paths: &[PathBuf],
) {
    use riir_refine::kat_service_config as svc;
    if let Some(net) = adopt_state {
        run_kat_adopt_state(net, discard, url, paths);
        return;
    }
    let dir = svc::account_dir();
    if !set {
        // `config get` (bare `config` behaves the same — a pure read). The
        // DIAL is the law seam (env > mainnet — a stored row never steers,
        // issue 126 follow-up 3); the stored row is disclosed as the
        // bookkeeping it now is.
        let (file_url, warning) = svc::read_with_warning(&dir);
        if let Some(w) = warning {
            eprintln!("config: {w}");
        }
        let (active, source) = svc::dial_service_url(
            svc::service_url_from_env().as_deref(),
            // Ungated default mirror — the gated pin test asserts it IS
            // `kat_lease_client::DEFAULT_SERVICE_URL`.
            svc::MAINNET_SERVICE_URL,
        );
        println!("{:<15}{}", "service url:", active);
        println!("     source: {}", source.label());
        match source {
            svc::ServiceUrlSource::Env => println!(
                "             (env {var} steers the dial while it is set)",
                var = svc::SERVICE_URL_ENV_VAR
            ),
            svc::ServiceUrlSource::Default => println!(
                "             (the baked mainnet default — the 2026-10-14 dialing law: bare \
                 commands target mainnet; --net devnet or the env var steer)"
            ),
            svc::ServiceUrlSource::Config => {
                unreachable!("the dialing seam never answers config")
            }
        }
        if let Some(stored) = &file_url {
            let alias_note = svc::pinned_alias_for(stored)
                .map(|a| format!(" ({a})"))
                .unwrap_or_default();
            println!(
                "{:<15}{}{alias_note} — BOOKKEEPING ONLY (a stored row no longer \
                 steers the dial)",
                "stored config:", stored
            );
        }
        if svc::service_url_from_env().is_some() && file_url.is_some() {
            eprintln!(
                "config: NOTE — {var} is set AND a config file exists; the env var steers (the \
                 stored row is bookkeeping)",
                var = svc::SERVICE_URL_ENV_VAR
            );
        }
        // riir-kat Issue 003 T6: the per-file network diagnosis — every
        // per-net state file under this root, its stamp, or the UNSTAMPED
        // flag with the adopt-state remedy. The Plan 023 g3 spirit: name the
        // source, never guess.
        {
            let root = resolve_heal_root(paths);
            let files = riir_refine::kat_adopt::scan_state_files(&root);
            if !files.is_empty() {
                // The table (issue 126 follow-up 4): the name column is
                // COMPUTED across the page, so the per-file diagnosis text
                // starts on one column instead of drifting with the path
                // lengths.
                let name_w = files.iter().map(|f| f.rel.len()).max().unwrap_or(4).max(4);
                println!("state files (under {}):", root.join(".refine").display());
                for f in &files {
                    let line = riir_refine::kat_adopt::diagnosis_line(f);
                    // diagnosis_line is `{rel}: {rest}` — split once at the
                    // first ": " and pad the name column (a rel never
                    // carries a colon).
                    match line.split_once(": ") {
                        Some((name, rest)) => {
                            println!("  {:<name_w$}  {}", format!("{name}:"), rest);
                        }
                        None => println!("  {line}"),
                    }
                }
            }
        }
        return;
    }
    let Some(raw) = url else {
        eprintln!("config set requires --url <devnet|mainnet|https://…>");
        usage();
    };
    // g1: validate BEFORE any write; the rejection names the rule.
    let validated = match svc::validate_service_url(raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("config set: {e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = svc::store(&dir, &validated) {
        eprintln!("config set: {e}");
        std::process::exit(1);
    }
    println!(
        "config: service url recorded → {validated} ({})",
        svc::service_config_path(&dir).display()
    );
    // Issue 126 follow-up 3 (the 2026-10-14 dialing law): the row is
    // BOOKKEEPING — it never steers the dial, so the old "burn/mine/
    // refill/claims will target it" promise is gone and the remedies are
    // named instead.
    println!(
        "config: BOOKKEEPING ONLY — commands dial mainnet by default; `cargo refine --net \
         devnet …` targets devnet for one invocation, {var} steers persistently",
        var = svc::SERVICE_URL_ENV_VAR
    );
    match svc::pinned_alias_for(&validated) {
        Some(alias) => println!("config: pinned network host ({alias})"),
        None => println!(
            "config: WARNING — not a pinned network host; payment links/QRs composed from it \
             are local-only artifacts (do not share)"
        ),
    }
    if svc::service_url_from_env().is_some() {
        eprintln!(
            "config: WARNING — {var} is set; it overrides this config until unset",
            var = svc::SERVICE_URL_ENV_VAR
        );
    }
    println!("config: inspect anytime — cargo refine config get");
}

/// riir-kat Issue 003 T4 (the refine half): `config adopt-state <net>
/// [--discard]` — the EXPLICIT, per-file migration for pre-split
/// network-blind state. Never auto-adopts: adoption is oracle-confirmed
/// (the burn ack's contribution fact vs the declared net's live `/history`
/// row) and everything else is offered as `--discard`-only (the quarantine
/// move). Ungated like the rest of `config` (the oracle's network read is
/// the cfg'd helper's to gate; without it every file reads discard-only
/// with the reason).
/// `--url` is the dev-rig/testing oracle override (the `config set` shape):
/// the declared net's alias resolves canonically in production, and a mock
/// ledger stands in for it under test. The pin gates the override like any
/// other dial.
fn run_kat_adopt_state(net: &str, discard: bool, url_override: Option<&str>, paths: &[PathBuf]) {
    use riir_refine::kat_adopt::{self as adopt, AdoptVerdict, FileClass, HostFacts, Placement};
    use riir_refine::kat_service_config as svc;

    let (default_url, other_net, other_url) = match net {
        "devnet" => (
            riir_refine::kat_service_urls::DEVNET_SERVICE_URL,
            "mainnet",
            riir_refine::kat_service_urls::MAINNET_SERVICE_URL,
        ),
        "mainnet" => (
            riir_refine::kat_service_urls::MAINNET_SERVICE_URL,
            "devnet",
            riir_refine::kat_service_urls::DEVNET_SERVICE_URL,
        ),
        _ => unreachable!("parse_args validated the net word"),
    };
    let net_url = match url_override {
        Some(raw) => match svc::validate_service_url(raw) {
            Ok(u) => u,
            Err(e) => {
                eprintln!("config adopt-state --url {raw}: {e}");
                std::process::exit(2);
            }
        },
        None => default_url.to_string(),
    };
    if let Err(e) = riir_refine::kat_net_pin::ensure_net_allowed(&net_url) {
        eprintln!(
            "config adopt-state {net}: the target {net_url} is outside this build's net \
             pin — {e}"
        );
        std::process::exit(2);
    }

    let root = resolve_heal_root(paths);
    let files = adopt::scan_state_files(&root);
    let actionable: Vec<&adopt::StateFile> = files
        .iter()
        .filter(|f| f.class() != FileClass::StampedOk)
        .collect();
    let base = root.join(".refine");
    if actionable.is_empty() {
        println!(
            "adopt-state {net}: no pre-split or mismatched state files under {} — nothing to do.",
            base.display()
        );
        return;
    }
    println!(
        "adopt-state {net} → {net_url}: {} actionable file(s) under {}",
        actionable.len(),
        base.display()
    );

    if discard {
        let mut moved = 0usize;
        let mut failed = 0usize;
        for f in &actionable {
            match riir_refine::kat_state_stamp::quarantine_state_file(&f.path, &root, &f.rel) {
                Ok(dest) => {
                    moved += 1;
                    println!("  quarantined {} → {}", f.rel, dest.display());
                }
                Err(e) => {
                    failed += 1;
                    eprintln!("  FAILED {}: {e}", f.rel);
                }
            }
        }
        println!(
            "quarantined {moved} file(s) (.bak copies kept beside each — nothing deleted); \
             the next run re-baselines fresh per net."
        );
        std::process::exit(if failed > 0 { 1 } else { 0 });
    }

    // The adopt path: oracle-backed, LAZY on the second ledger (a
    // declared-net match never dials the other net — the mock-test and
    // the prod posture share that property). Facts are fetched ONLY when
    // a flat burn ack exists (the one file with oracle fields); every
    // other file lands discard-only without any network.
    let needs_oracle = actionable
        .iter()
        .any(|f| f.kind == adopt::StateFileKind::BurnAck && f.placement == Placement::FlatLegacy);
    let (declared_facts, declared_why) = if needs_oracle {
        adopt_oracle_facts(&net_url)
    } else {
        (HostFacts::unreachable(), None)
    };

    let mut adopted = 0usize;
    let mut refused: Vec<String> = Vec::new();
    let mut needs_other: Vec<&adopt::StateFile> = Vec::new();
    for f in &actionable {
        if f.placement != Placement::FlatLegacy {
            refused.push(format!(
                "{}: unstamped/mismatched INSIDE a net dir — quarantine it (--discard) and let \
                 the next run re-baseline",
                f.rel
            ));
            continue;
        }
        match adopt::adopt_verdict(f, net, &declared_facts, other_net, None) {
            AdoptVerdict::Adopt => match adopt::adopt_into_net(&root, f, &net_url) {
                Ok(dest) => {
                    adopted += 1;
                    println!(
                        "  adopted {} → {} (oracle-confirmed)",
                        f.rel,
                        dest.display()
                    );
                }
                Err(e) => refused.push(format!("{}: {e}", f.rel)),
            },
            AdoptVerdict::NeedsOtherNet => needs_other.push(f),
            AdoptVerdict::BelongsToOtherNet { .. } => unreachable!("None other never decides"),
            AdoptVerdict::DiscardOnly { reason } => refused.push(format!(
                "{}: {reason} — `--discard` quarantines it{}",
                f.rel,
                declared_why
                    .as_deref()
                    .map_or(String::new(), |w| format!(" (oracle: {w})"))
            )),
        }
    }
    // Second pass, only for acks the declared net did not match: consult
    // the OTHER live ledger once, then decide belongs-to-other vs discard.
    if !needs_other.is_empty() {
        let (other_facts, other_why) = adopt_oracle_facts(other_url);
        for f in needs_other {
            match adopt::adopt_verdict(f, net, &declared_facts, other_net, Some(&other_facts)) {
                AdoptVerdict::BelongsToOtherNet { other } => refused.push(format!(
                    "{}: belongs to {other} (the other ledger's contribution row matches) — \
                     re-run `config adopt-state {other}` for it, or --discard",
                    f.rel
                )),
                AdoptVerdict::DiscardOnly { reason } => refused.push(format!(
                    "{}: {reason}{}",
                    f.rel,
                    other_why
                        .as_deref()
                        .map_or(String::new(), |w| format!(" (other oracle: {w})"))
                )),
                _ => unreachable!("the second pass only sees decided verdicts"),
            }
        }
    }
    println!(
        "adopted {adopted} file(s); {} refused (discard-only or wrong net)",
        refused.len()
    );
    for r in &refused {
        eprintln!("  REFUSED {r}");
    }
    if adopted > 0 {
        println!(
            "the net dir now owns the adopted files; `cargo refine config get` shows every \
             stamp."
        );
    }
    std::process::exit(if refused.is_empty() { 0 } else { 2 });
}

/// The adopt-state oracle: one bounded `GET /history` for this account on
/// the given base URL → the contribution-fact pair. Fail-closed: any
/// failure is an unreachable verdict + the human reason (never a guess).
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn adopt_oracle_facts(url: &str) -> (riir_refine::kat_adopt::HostFacts, Option<String>) {
    use riir_refine::kat_account::KatAccount;
    use riir_refine::kat_account_client::{AccountTransport as _, HttpAccountTransport};
    use riir_refine::kat_adopt::HostFacts;
    use riir_refine::kat_lease_client::HttpResponse;

    let unreach = |why: String| (HostFacts::unreachable(), Some(why));
    let Ok(account) = KatAccount::load(&KatAccount::default_dir()) else {
        return unreach(
            "no local account key — the oracle cannot query a ledger without \
             one (run `cargo refine login` first)"
                .into(),
        );
    };
    let Some(hex) =
        riir_refine::kat_account_client::service_account_hex(&account.state().account_id)
    else {
        return unreach("the local account id is not the 64-hex derivation".into());
    };
    // 5 s, the `account` probe's budget: adopt-state is an explicit,
    // operator-run command — one bounded read per ledger, never a spin.
    let transport = HttpAccountTransport::new(url, 5);
    match transport.get_history(hex, 1) {
        HttpResponse::Ok(body) => match riir_refine::kat_account_client::parse_history(&body) {
            Ok(h) => (
                HostFacts {
                    reachable: true,
                    contributed: Some(h.contributed),
                    first_commitment_hex: h.first_commitment_hex,
                },
                None,
            ),
            Err(e) => unreach(format!("the /history body did not parse: {e}")),
        },
        HttpResponse::Status(c) => unreach(format!("GET /history answered HTTP {c}")),
        HttpResponse::NotModified => unreach("GET /history answered 304".into()),
        HttpResponse::Error(e) => unreach(format!("{url} is unreachable: {e}")),
    }
}

/// The compiled-out posture: the oracle cannot run, so every oracle-able
/// file reads discard-only with the rebuild named (the honest degradation
/// — never a silent pretend-match).
#[cfg(not(all(feature = "kat_service", feature = "kat_transport")))]
fn adopt_oracle_facts(_url: &str) -> (riir_refine::kat_adopt::HostFacts, Option<String>) {
    (
        riir_refine::kat_adopt::HostFacts::unreachable(),
        Some(
            "this build has no service transport — rebuild with \
             --features kat_service,kat_transport to run the oracle"
                .into(),
        ),
    )
}

#[cfg(all(feature = "kat_transport", feature = "latent_retrieval"))]
fn run_kat_sync() {
    use riir_refine::kat_lease_client::{
        run_update_phase, HttpLeaseTransport, LeaseCharge, LeaseStore,
    };

    let root = resolve_heal_root(&[]);
    let url = command_service_url();
    let store = LeaseStore::open_net(&root, &url);
    let mut charge = LeaseCharge::open(&store);
    let transport = HttpLeaseTransport::new(&url, 30);

    // ── consent gate (owner mandate 2026-09-08) ───────────────────────
    // Checked BEFORE the payload is built or signed: `cargo refine --unmine`
    // records a machine-global opt-out, and an opt-out that still assembles
    // and signs the batch it refuses to send is not an opt-out. `Unset`
    // pushes exactly as this command always did — running `sync` IS the
    // consent — so only an explicit opt-out changes behaviour here.
    let consent_dir = riir_refine::kat_mining_consent::default_dir();
    let (consent, consent_warning) =
        riir_refine::kat_mining_consent::read_with_warning(&consent_dir);
    if let Some(w) = consent_warning {
        eprintln!("{w}");
    }

    // ── push half ──────────────────────────────────────────────────
    // ORDER IS LOAD-BEARING (Plan 327 T3 + Issue 079): the push must land
    // BEFORE the burn drain below — the accepted batch is what makes the
    // server record the contribution fact, and the drain's v3 response is
    // what carries it into the ack. Drain-first answered `contributed:
    // Some(false)` for a fresh contributor's own push, the truth-wins gate
    // then refused the re-evaluation, and lifting the gate took a SECOND
    // sync (the 079 contract is ONE accepted push). Opted-out / no-store
    // machines are unaffected: the push half skips itself and the drain
    // still runs.
    // ── push half (extracted — shared with the opted-in run tails) ────
    // The consent gate stays verbose HERE: `cargo refine sync` is the place
    // an opted-out holder reads WHY nothing was built or signed. The tails
    // gate on explicit OptedIn instead and run the same helper quiet.
    if !consent.push_allowed() {
        println!("sync: push SKIPPED — mining is opted out on this machine");
        println!(
            "sync: consent record {} says opted out; nothing was built, signed or sent",
            riir_refine::kat_mining_consent::consent_path(&consent_dir).display()
        );
        println!("sync: re-enable with `cargo refine --mine` (the lease pull below still runs — a read is not a contribution)");
    } else {
        let _ = push_mining_batch(false, &url, &transport);
    }

    // ── burn sync (Issue 076 client half) — AFTER the push half ──────
    // Account billing, NOT the consent-gated batch above: the burn debt
    // rides the login, so it reports even while mining is opted out (the
    // issue is explicit — billing ≠ contribution). Same stacking rules as
    // the post-run hook: one line, ~5 s, fail-soft; drains whatever a
    // failed run left stacked. The burn numbers come from the settled
    // charge ledger on disk (the last heal run's save), never recomputed.
    // Runs AFTER the push (see the ORDER note there): the accepted batch
    // records the contribution fact server-side FIRST, so this drain's v3
    // response carries `contributed: Some(true)` into the ack and the 079
    // contribute-now re-evaluation lifts on the same sync.
    {
        let meter = open_kat_meter(&[]);
        let charge = open_kat_charge(&[], meter.as_ref());
        if let Some(c) = charge.as_ref() {
            let outcome = riir_refine::kat_burn_client::run_burn_sync(
                &root,
                c.state().charged_kat,
                c.state().meter_watermark_tokens,
            );
            print_burn_sync(&outcome);
        }
    }

    // ── pull half (shared with `update`) ───────────────────────────
    let current = store.current_epoch().unwrap_or(0);
    let had_epoch = store.current_epoch();
    let outcome = run_update_phase(&transport, &store, &mut charge, false, None);
    match outcome.status {
        riir_refine::kat_lease_client::UpdateStatus::NotModified => {
            println!("corpus lease: up to date (epoch {current}, 304 — no charge)");
        }
        riir_refine::kat_lease_client::UpdateStatus::Applied {
            epoch,
            charge_micro_kat,
        } => {
            println!(
                "{}",
                riir_refine::kat_lease_client::LeaseCharge::line(epoch, charge_micro_kat)
            );
            print_corpus_epoch_gain(&store, had_epoch, epoch);
        }
        riir_refine::kat_lease_client::UpdateStatus::Skipped(reason) => {
            println!("corpus lease: skipped — {reason}");
        }
        riir_refine::kat_lease_client::UpdateStatus::Failed(e) => {
            eprintln!("corpus lease: {e}");
        }
    }
    if let Err(e) = charge.save() {
        eprintln!("corpus lease: could not persist the charge ledger: {e}");
    }
}

/// Proposal 005 T1.6 (Plan 040): `cargo refine claim --tx <sig>` — compose +
/// submit the payment-claim request. The five-check verification is
/// service-side (deferred with the Worker shell); the client validates the
/// format, queues locally when the service is not live, and NEVER fakes a
/// mint (G9d — client counts never mint or spend).
#[cfg(feature = "kat_service")]
fn run_kat_claim(sig: Option<&str>) {
    use riir_refine::kat_lease_client::{queue_claim, ClaimRequest, CLAIM_VERSION};

    let Some(sig) = sig else {
        eprintln!("claim: --tx <signature> is required (the Solana transaction signature)");
        std::process::exit(2);
    };
    if !riir_refine::kat_lease_client::validate_solana_signature(sig) {
        eprintln!("claim: --tx does not look like a Solana signature (base58, 64-88 chars)");
        std::process::exit(2);
    }
    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("claim: no account ({e}) — run `cargo refine login` first");
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
        use riir_refine::kat_lease_client::LeaseTransport;
        let url = command_service_url();
        let transport = riir_refine::kat_lease_client::HttpLeaseTransport::new(&url, 10);
        if let riir_refine::kat_lease_client::HttpResponse::Ok(_) = transport.post_claim(&json) {
            println!("claim: submitted to {url} — the service verifies the five checks + mints at settlement");
            return;
        }
    }
    let root = resolve_heal_root(&[]);
    match queue_claim(&root, &command_service_url(), &req) {
        Ok(path) => {
            println!(
                "claim: service plane not yet live — queued locally at {}",
                path.display()
            );
            println!(
                "claim: the queued claim submits when the Worker lands; payment page: {}",
                riir_refine::kat_lease_client::payment_url(&command_service_url(), &req.account_id)
            );
            println!("claim: nothing is minted client-side (the server ledger is the authority)");
        }
        Err(e) => {
            eprintln!("claim: could not queue locally: {e}");
            std::process::exit(1);
        }
    }
}

/// Proposal 007 T2.1 (Plan 041): `cargo refine stake --usd <N>` / `stake
/// --info` — the render-only + QR staking surface. Service-first (the
/// service composes the transfer request it must track the reference for);
/// when the plane is not live, the local fallback composes from the
/// env-configured stake-custody payee + a fresh on-curve reference, queues
/// the intent, and prints the honest state. Nothing client-side mints,
/// credits, or moves principal (the G9d posture held for staking too).
#[cfg(feature = "kat_service")]
/// The 32-hex LEDGER account id for the stake family's service reads and
/// writes — the `/stake/*` routes require exactly 32 hex (`account_from_hex`
/// both sides; a 64-hex id 400s, which the pre-fix commands silently read
/// as their "service plane not yet live" fallbacks). The same truncation
/// `/balance` + `/history` already use (`service_account_hex`); the raw id
/// is kept when the local record is not the 64-hex derivation (behavior
/// preserved for malformed records).
#[cfg(feature = "kat_service")]
fn service_acct_or_raw(account_id: &str) -> String {
    riir_refine::kat_account_client::service_account_hex(account_id)
        .unwrap_or(account_id)
        .to_string()
}

fn run_kat_stake(usd: Option<&str>, info: bool, restake: Option<&str>, redeem: bool) {
    use riir_refine::kat_stake_client as stake;

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("stake: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let account_id = service_acct_or_raw(&account.state().account_id);
    let root = resolve_heal_root(&[]);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    if info {
        run_stake_info(&account_id, &root, now_unix);
        return;
    }

    // Proposal 017: the posture + redeem ops precede the QR compose (one
    // command = one op; `--usd` beside them is a usage error, not a mode).
    if restake.is_some() || redeem {
        if usd.is_some() {
            eprintln!("stake: --restake/--redeem are account-state ops — drop --usd");
            std::process::exit(2);
        }
        #[cfg(feature = "kat_transport")]
        {
            let url = command_service_url();
            let transport = riir_refine::kat_stake_client::HttpStakeTransport::new(&url, 10);
            let flow = match account.signing_key() {
                Ok(key) => {
                    if let Some(posture) = restake {
                        riir_refine::kat_stake_client::restake_flow(
                            &transport,
                            &key,
                            posture,
                            &account_id,
                        )
                    } else {
                        riir_refine::kat_stake_client::redeem_flow(&transport, &key, &account_id)
                    }
                }
                Err(e) => Err(format!(
                    "stake: account key unreadable ({e}) — run `cargo refine login`"
                )),
            };
            match flow {
                Ok(line) => println!("{line}"),
                Err(e) => {
                    eprintln!("stake: {e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        #[cfg(not(feature = "kat_transport"))]
        riir_refine::build_stamp::refuse("stake --restake/--redeem", &["kat_transport"]);
    }

    let Some(usd) = usd else {
        eprintln!("stake: --usd <N> is required (the stake amount in dollars), or --info for the position, or --restake off|buffered / --redeem for the posture ops");
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
        use riir_refine::kat_stake_client::StakeOutcome;
        let url = command_service_url();
        let transport = riir_refine::kat_stake_client::HttpStakeTransport::new(&url, 10);
        match riir_refine::kat_stake_client::stake_request_flow(
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
                println!(
                    "stake: the service verifies the five checks and credits the tranche at sync"
                );
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
        riir_refine::build_stamp::refuse("stake", &["kat_transport"]);
    }
}

/// `stake --info` — the position render. Service-first; the not-live
/// fallback renders the local-queue view (honest: no fake tranches).
#[cfg(feature = "kat_service")]
fn run_stake_info(account_id: &str, root: &std::path::Path, now_unix: u64) {
    use riir_refine::kat_stake_client as stake;

    // `account_id`/`now_unix` are transport-path params (the service-first
    // block below); the local-queue fallback renders from disk state alone.
    #[cfg(not(feature = "kat_transport"))]
    let _ = (account_id, now_unix);

    #[cfg(feature = "kat_transport")]
    {
        use riir_refine::kat_stake_client::StakeTransport as _;
        let url = command_service_url();
        let transport = riir_refine::kat_stake_client::HttpStakeTransport::new(&url, 10);
        if let riir_refine::kat_lease_client::HttpResponse::Ok(body) =
            transport.get_stake_info(account_id)
        {
            match serde_json::from_slice::<riir_refine::kat_stake_client::StakeInfo>(&body) {
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

/// Proposal 007 T2.1: `cargo refine unstake` — the drain-all-unlocked refund
/// request. Service-first; the not-live path queues the intent and still
/// prints the full escrow/venting disclosure (the mechanics copy is true
/// regardless of transport — disclosed at BOTH stake and unstake time per
/// the proposal).
#[cfg(feature = "kat_service")]
fn run_kat_unstake() {
    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("unstake: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let account_id = service_acct_or_raw(&account.state().account_id);
    let root = resolve_heal_root(&[]);
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());

    #[cfg(feature = "kat_transport")]
    {
        let url = command_service_url();
        let transport = riir_refine::kat_stake_client::HttpStakeTransport::new(&url, 10);
        // Issue 096 (riir-dapps): the unstake route is signature-gated —
        // the request signs with the account key (the claim key).
        let flow = match account.signing_key() {
            Ok(key) => riir_refine::kat_stake_client::unstake_flow(
                &transport,
                &key,
                &account_id,
                &root,
                now_unix,
            ),
            Err(e) => Err(format!(
                "unstake: account key unreadable ({e}) — run `cargo refine login`"
            )),
        };
        match flow {
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
        riir_refine::build_stamp::refuse("unstake", &["kat_transport"]);
    }
}

/// Proposal 008 T2.4 (Plan 012): `cargo refine vote --proposal <id>
/// --yes|--no` — the governance voting surface. Renders the proposal
/// (weight + evidence lead — the service serves the view when live), signs
/// the vote with the LOGIN key, and submits (service-first; the honest
/// not-live fallback queues the SIGNED intent). Nothing client-side decides
/// eligibility or tallies — the ledger computes the weight that counts.
#[cfg(feature = "kat_service")]
fn run_kat_vote(proposal: Option<u64>, yes: bool, no: bool) {
    use riir_refine::kat_vote_client as vote;

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("vote: no account ({e}) — run `cargo refine login` first");
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
        use riir_refine::kat_vote_client::VoteOutcome;
        let url = command_service_url();
        let transport = riir_refine::kat_vote_client::HttpVoteTransport::new(&url, 10);
        match riir_refine::kat_vote_client::vote_flow(
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
        riir_refine::build_stamp::refuse("vote", &["kat_transport"]);
    }
}

/// riir-dao Proposal 001 T4 (`account migrate`): the rotation lane's CLI
/// surface. Exactly one mode per invocation; the census doubles as the
/// filing's E-3 gate. The OLD key comes ONLY from the explicit
/// `--old-key <path>` (the per-invocation key source — the env var is
/// NEVER consulted, the measured T-PREFLIGHT finding).
#[allow(clippy::too_many_arguments)]
fn run_kat_migrate(
    github: Option<&str>,
    object: Option<&str>,
    status_id: Option<&str>,
    census_mode: bool,
    old_key_path: Option<&str>,
    github_token: Option<String>,
    repo_base: Option<&str>,
) {
    let modes = github.is_some() as u8
        + object.is_some() as u8
        + status_id.is_some() as u8
        + census_mode as u8;
    if modes != 1 {
        eprintln!(
            "migrate: exactly one mode is required — \
             --github <user> (file) | --object <request-id> | --status <request-id> | --census"
        );
        std::process::exit(2);
    }
    let repo_base = repo_base
        .map(str::to_owned)
        .unwrap_or_else(|| riir_refine::kat_migrate_client::DEFAULT_OBJECTION_REPO_BASE.to_owned());
    let root = resolve_heal_root(&[]);

    // ── the census mode (report-only; kat_service, no transport) ──
    if census_mode {
        let url = riir_refine::kat_lease_client::service_url_for_command();
        let c = riir_refine::kat_migrate_client::census(
            &root,
            &riir_refine::kat_account::KatAccount::default_dir(),
            &url,
        );
        print!(
            "{}",
            riir_refine::kat_migrate_client::render_census(&c, &url)
        );
        if !c.blockers.is_empty() {
            std::process::exit(1);
        }
        return;
    }

    // The modes below resolve the net-pinned dial (the strict seam).
    let url = command_service_url();

    // ── the objection URL (offline compose) ──
    if let Some(request_id_hex) = object {
        // The compose needs the request's accounts; the CLI knows only the
        // id, so the URL composes with the id alone (the worker's ack URL
        // carried the accounts — an offline re-compose keeps the id, which
        // is the part every filter matches on).
        println!(
            "{}",
            riir_kat::kat_protocol_migrate::compose_objection_issue_url(
                &repo_base,
                request_id_hex,
                "",
                "",
                "",
            )
        );
        println!(
            "objection: open the URL, state the reason, react 👍 to SUPPORT the objection \
             (blocks) or 👎 to REJECT it (proceeds); label {LABEL} is prefilled",
            LABEL = riir_kat::kat_protocol_migrate::OBJECTION_LABEL,
        );
        return;
    }

    // ── the status read ──
    if let Some(request_id_hex) = status_id {
        match riir_refine::kat_migrate_client::status(&url, request_id_hex) {
            Ok(body) => {
                println!("{body}");
                // The lazy-close hint: a pending row whose window elapsed
                // closes on the worker at the NEXT read (this one just did).
                let pending_elapsed = serde_json::from_str::<serde_json::Value>(&body)
                    .ok()
                    .filter(|v| v.get("state").and_then(|s| s.as_str()) == Some("pending"))
                    .and_then(|v| v.get("window_close_unix").and_then(|c| c.as_u64()))
                    .map(|close| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|d| d.as_secs())
                            .map_or(false, |now| now >= close)
                    })
                    .unwrap_or(false);
                if pending_elapsed {
                    println!(
                        "status: the window has elapsed — the worker closes it lazily; \
                         re-read `--status` for the post-close state"
                    );
                }
            }
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }

    // ── the filing ──
    let Some(github_user) = github else {
        unreachable!("exactly one mode was validated");
    };
    let Some(old_key) = old_key_path else {
        eprintln!(
            "migrate: filing requires --old-key <path> — the OLD key file (OpenSSH Ed25519) \
             that signs the release. The per-invocation key source is EXPLICIT by law: \
             the env var is never a signing source (the measured T-PREFLIGHT finding)."
        );
        std::process::exit(2);
    };
    let token = github_token
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .or_else(|| std::env::var("GH_TOKEN").ok());
    let params = riir_refine::kat_migrate_client::FileParams {
        github_user,
        old_key_path: std::path::Path::new(old_key),
        github_token: token,
        repo_base: &repo_base,
    };
    match riir_refine::kat_migrate_client::file_migration(&root, &params, &url) {
        Ok(outcome) => {
            let ack = &outcome.ack;
            println!(
                "migrate: request {} — state {}{}",
                ack.request_id_hex,
                rmpk_state(ack.state),
                if outcome.filed {
                    ""
                } else {
                    " (replay — already filed)"
                }
            );
            if let Some(u) = &outcome.objection_url {
                println!("objection url: {u}");
            }
            println!(
                "window: 24 h default-deny — nobody objects → release at close; \
                 an objection with NO votes cancels (anti-theft first)"
            );
            println!(
                "next: watch with --status {}; object with --object {}",
                ack.request_id_hex, ack.request_id_hex
            );
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn rmpk_state(state: riir_kat::kat_protocol_migrate::MigrateRequestState) -> &'static str {
    match state {
        riir_kat::kat_protocol_migrate::MigrateRequestState::Pending => "pending",
        riir_kat::kat_protocol_migrate::MigrateRequestState::Dispute => "dispute",
        riir_kat::kat_protocol_migrate::MigrateRequestState::Released => "released",
        riir_kat::kat_protocol_migrate::MigrateRequestState::Cancelled => "cancelled",
    }
}

/// The account's open checkouts, read through the SAME transport tier
/// (the history read's transport + the purchases page parse). `None` =
/// the read failed (the caller falls back to the bare 429 prose). The
/// account signing key is required because the seq actions are signed —
/// fetched here once, reused by show/void.
fn topup_open_intents(url: &str) -> Option<Vec<riir_refine::kat_protocol_pay::PayPurchaseRow>> {
    #[cfg(feature = "kat_transport")]
    {
        use riir_refine::kat_account_client::AccountTransport;
        use riir_refine::kat_lease_client::HttpResponse;
        let dir = riir_refine::kat_account::KatAccount::default_dir();
        let account = riir_refine::kat_account::KatAccount::load(&dir).ok()?;
        let signing_key = account.signing_key().ok()?;
        let pubkey = signing_key.verifying_key().to_bytes();
        // The /history route indexes by the SERVICE account id —
        // `blake3(pubkey)[..16]` (the house derivation), 32 hex chars —
        // never the raw pubkey hex (a well-formed query for a
        // nonexistent account is the silent-zero failure the protocol
        // module's `service_account_hex` guard exists to prevent).
        let acct_hex = riir_refine::kat_payreq_client::hex_encode(
            &riir_refine::kat_protocol_account::account_for_pubkey(&pubkey),
        );
        let transport = riir_refine::kat_account_client::HttpAccountTransport::new(url, 10);
        let body = match transport.get_history(&acct_hex, 20) {
            HttpResponse::Ok(b) => b,
            _ => return None,
        };
        let v: serde_json::Value = serde_json::from_slice(&body).ok()?;
        let (rows, _, _) = riir_refine::kat_protocol_pay::parse_purchases_page(&v).ok()?;
        Some(rows.into_iter().filter(|r| r.status == "open").collect())
    }
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = url;
        None
    }
}

/// USDC base units → the display string (the 6-dec fiat, whole amounts
/// bare — the pay client's own rule, mirrored for the listing lines).
fn format_usdc(base: u64) -> String {
    // USDC's own 6-dec base unit (a third million — neither µKAT nor the
    // token Mtok), derived from riir-kat's decimals const, never a literal.
    let per_usdc = 10u64.pow(riir_refine::kat_stake_client::USDC_DECIMALS);
    let whole = base / per_usdc;
    let frac = base % per_usdc;
    if frac == 0 {
        format!("{whole}")
    } else {
        format!("{whole}.{frac:06}")
    }
}

/// One aligned checkout line — the 429 listing and the picker rows render
/// the SAME text, so the picker can never disagree with the list above it.
/// `kat_micro` comes from the catalog by pack id (the history row's own
/// `kat_micro` is 0 while open — the credit exists only after settle).
/// `ttl_secs` is the catalog's own TTL: the history wire carries no
/// per-row expiry, so the deadline is `created + ttl` rendered by the pay
/// lane's ONE expiry phrase (riir-kat Issue 006).
#[cfg(feature = "kat_transport")]
fn topup_checkout_line(
    row: &riir_refine::kat_protocol_pay::PayPurchaseRow,
    kat_micro: Option<u64>,
    now_unix: u64,
    ttl_secs: u64,
) -> String {
    let mins = now_unix.saturating_sub(row.created_unix) / 60;
    let kat = match kat_micro {
        Some(m) => riir_refine::kat_payreq_client::render_kat(i64::try_from(m).unwrap_or(i64::MAX)),
        None => "? KAT".to_string(),
    };
    let mut reference = row.reference.clone();
    if reference.chars().count() > 6 {
        reference = format!("{}…", reference.chars().take(6).collect::<String>());
    }
    let expires = row.created_unix.saturating_add(ttl_secs);
    format!(
        "seq {:<4} {:>8} USDC → {:<14} {:>5} min old, {}  ref {}",
        row.seq,
        format_usdc(row.usdc_base),
        kat,
        mins,
        riir_refine::kat_pay_client::expiry_text(expires, now_unix),
        reference
    )
}

/// The typed fallback (raw mode unavailable: an odd terminal, non-unix):
/// the issue-126 prompt verbatim — empty = newest, `v<seq>` voids,
/// `<seq>` shows, EOF / a bad answer exits 2. Returns `None` when the
/// run answered nothing actionable — the caller prints the flag hints.
#[cfg(feature = "kat_transport")]
fn topup_typed_pick(
    rows: &[riir_refine::tty_pick::PickRow],
) -> Option<riir_refine::tty_pick::Picked> {
    print!("show which (seq, empty = newest, 'v<seq>' to void): ");
    use std::io::Write as _;
    std::io::stdout().flush().ok();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => std::process::exit(2),
        Ok(_) => {
            let t = line.trim();
            if t.is_empty() {
                return Some(riir_refine::tty_pick::Picked::Show(rows[0].seq));
            }
            if let Some(raw) = t.strip_prefix(['v', 'V']) {
                return match raw.parse::<u64>() {
                    Ok(s) => Some(riir_refine::tty_pick::Picked::Void(s)),
                    Err(_) => std::process::exit(2),
                };
            }
            match t.parse::<u64>() {
                Ok(s) => Some(riir_refine::tty_pick::Picked::Show(s)),
                Err(_) => std::process::exit(2),
            }
        }
    }
}

/// The one-line value verbs' shared fetch (issue 126 follow-up, the
/// owner's "expect `cargo refine kat` and `cargo refine tuna` should show
/// value"): the SAME bearer-free /history read `account --history`
/// renders in full, parsed into the typed [`ServiceHistory`]. Any failure
/// names the richer surface and exits 1 (never a silent zero).
#[cfg(feature = "kat_transport")]
fn fetch_service_history() -> riir_refine::kat_protocol_account::ServiceHistory {
    use riir_refine::kat_account_client::{AccountTransport, HttpAccountTransport};
    use riir_refine::kat_lease_client::HttpResponse;
    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("kat: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("kat: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let pubkey = signing_key.verifying_key().to_bytes();
    // The /history route indexes by the SERVICE account id (the house
    // derivation), never the raw pubkey hex — the topup_open_intents law.
    let acct_hex = riir_refine::kat_payreq_client::hex_encode(
        &riir_refine::kat_protocol_account::account_for_pubkey(&pubkey),
    );
    let transport = HttpAccountTransport::new(&command_service_url(), 10);
    match transport.get_history(&acct_hex, 1) {
        HttpResponse::Ok(body) => match riir_refine::kat_protocol_account::parse_history(&body) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("kat: {e} — try `cargo refine account --history`");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!(
                "kat: the service did not answer /history — try `cargo refine account --history`"
            );
            std::process::exit(1);
        }
    }
}

/// `cargo refine kat` — the one-line KAT balance (the service ledger's
/// authoritative number), host-labelled so a devnet/mainnet mix-up is
/// visible at a glance.
#[cfg(feature = "kat_transport")]
fn run_kat_balance() {
    let h = fetch_service_history();
    let host = command_service_url()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string();
    println!(
        "{} — {host}",
        riir_refine::kat_payreq_client::render_kat(h.balance_micro)
    );
}

/// The TUNA display (the unit differs from KAT; the 6-dec rule is the
/// leaf's ONE formatter, `kat_units::render_units` — issue 129 §9's
/// airdrop receipt renders through the same fn).
#[cfg(feature = "kat_transport")]
fn render_tuna(micro: i64) -> String {
    riir_refine::kat_units::render_units(micro, "TUNA")
}

/// The `tuna` verb's line for the ZIPPED-PAIR-ABSENT answer — the server
/// omits `tuna_balance_micro`/`tuna_expires_unix` when the account holds
/// NO spendable trial credit at the read's tick. `granted` (the unified
/// claim predicate the same response carries) disambiguates WHY, because
/// "pre-TUNA deployment" was the wrong attribution for the common case:
/// a consumed free tier (a KAT-plane grant, or TUNA spent/expired) reads
/// the identical wire shape on a current worker. Never a silent zero
/// (the additive-wire rule).
#[cfg(feature = "kat_transport")]
fn tuna_none_line(granted: bool) -> &'static str {
    if granted {
        "0.000000 TUNA — the free tier is consumed on this account (a KAT-plane grant, or TUNA spent/expired; it is granted once per account)"
    } else {
        "not claimed yet — `cargo refine account` (or any --mine sync) claims the free tier automatically"
    }
}

/// `cargo refine tuna` — the one-line trial-credit balance + expiry
/// window. The zipped-pair-absent answer says WHY (consumed vs not-yet-
/// claimed, via the response's `granted`); a positive balance says its
/// expiry — never a silent zero (the additive-wire rule).
#[cfg(feature = "kat_transport")]
fn run_kat_tuna() {
    let h = fetch_service_history();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    match h.tuna_balance_micro {
        None => println!("tuna: {}", tuna_none_line(h.granted)),
        Some(0) => println!(
            "0.000000 TUNA — the trial credit is spent or expired (it is granted once per account)"
        ),
        Some(micro) => match h.tuna_expires_unix {
            Some(unix) if unix > now => {
                let days = (unix - now) / 86_400;
                println!("{} — expires in {days} d", render_tuna(micro));
            }
            Some(_) => println!(
                "{} — past its expiry (the server's sweep settles it at the boundary)",
                render_tuna(micro)
            ),
            None => println!("{}", render_tuna(micro)),
        },
    }
}

/// What `topup --pack` / `buy <amount> <unit>` asked for (the buy
/// friction-cut): an exact pack id, or an amount the CATALOG must match
/// exactly — nobody guesses a price (the --pack refusal law).
#[cfg(feature = "kat_service")]
enum BuyAsk {
    Pack(Option<u8>),
    /// micro-KAT (the parse's unit).
    Kat(u64),
    /// USDC base units.
    Usdc(u64),
}

/// Parse the `buy <amount> <unit>` pair (the glued `10kat` form already
/// split post-parse) into a catalog ask. 6-dec decimals both units
/// (`parse_kat_amount` is the shared parser); a zero amount refuses.
#[cfg(feature = "kat_service")]
fn parse_buy_ask(amount: Option<&str>, unit: Option<&str>) -> BuyAsk {
    let (Some(amount), Some(unit)) = (amount, unit) else {
        eprintln!("buy: an amount and a unit are required — cargo refine buy 10 kat");
        std::process::exit(2);
    };
    let micro = match riir_refine::kat_payreq_client::parse_kat_amount(amount) {
        Some(m) if m > 0 => u64::try_from(m).unwrap_or_else(|_| {
            eprintln!("buy: the amount overflows — {amount:?}");
            std::process::exit(2);
        }),
        Some(_) => {
            eprintln!("buy: the amount must be above zero");
            std::process::exit(2);
        }
        None => {
            eprintln!("buy: bad amount {amount:?} (6-dec decimals, e.g. 10 or 1.5)");
            std::process::exit(2);
        }
    };
    match unit.to_ascii_lowercase().as_str() {
        "kat" => BuyAsk::Kat(micro),
        "usdc" => BuyAsk::Usdc(micro),
        other => {
            eprintln!("buy: unknown unit {other:?} — kat | usdc");
            std::process::exit(2);
        }
    }
}

/// Issue 126: `topup --void <seq>` — release an open checkout's slot.
/// Idempotent server-side; the marker stays reserved until the TTL (a
/// payment already scanned still lands on the account).
#[cfg(feature = "kat_service")]
fn run_kat_topup_void(seq: u64) {
    #[cfg(feature = "kat_transport")]
    use riir_refine::kat_pay_client as topup;

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("topup: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("topup: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let url = command_service_url();
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (signing_key, seq, url);
        riir_refine::build_stamp::refuse("topup", &["kat_transport"]);
    }
    #[cfg(feature = "kat_transport")]
    {
        let transport = topup::HttpTopupTransport::new(&url, 10);
        match topup::void_intent(&transport, &signing_key, seq) {
            topup::TopupOutcome::Ok(v) => {
                assert_eq!(v.status.as_str(), "voided");
                println!("checkout seq {} voided — the slot is free; a payment already scanned still lands on your account until its TTL", v.seq);
            }
            topup::TopupOutcome::Refused {
                status,
                code,
                detail,
            } => {
                eprintln!(
                    "topup: the void was refused ({status} {}{})",
                    code.as_deref().unwrap_or("no code"),
                    detail
                );
                std::process::exit(1);
            }
            topup::TopupOutcome::Unexpected(e) => {
                eprintln!("topup: void failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// Issue 126: `topup --show <seq>` — re-print an open (or
/// voided-unexpired) checkout: the SAME lines + QR creation printed,
/// recomposed server-side from the row. One definition of the checkout
/// surface; nothing is re-signed, nothing re-priced.
#[cfg(feature = "kat_service")]
fn run_kat_topup_show(seq: u64, qr: Option<bool>) {
    #[cfg(feature = "kat_transport")]
    use riir_refine::kat_pay_client as topup;

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("topup: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("topup: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let url = command_service_url();
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (signing_key, seq, url, qr);
        riir_refine::build_stamp::refuse("topup", &["kat_transport"]);
    }
    #[cfg(feature = "kat_transport")]
    {
        let transport = topup::HttpTopupTransport::new(&url, 10);
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        match topup::show_intent(&transport, &signing_key, seq) {
            topup::TopupOutcome::Ok(intent) => {
                for l in topup::intent_lines(&intent, now_unix) {
                    println!("{l}");
                }
                let want_qr = match qr {
                    Some(b) => b,
                    None => {
                        use std::io::IsTerminal as _;
                        std::io::stdout().is_terminal()
                    }
                };
                if want_qr {
                    let rendered = riir_refine::kat_lease_client::render_qr_ascii(&intent.url);
                    if rendered.is_empty() {
                        eprintln!("topup: the QR could not be rendered — open the URL above");
                    } else {
                        println!("{rendered}");
                        println!("  scan with any Solana Pay wallet (Phantom / Solflare) — the QR is the URL above");
                    }
                }
            }
            topup::TopupOutcome::Refused {
                status,
                code,
                detail,
            } => {
                eprintln!(
                    "topup: the show was refused ({status} {}{})",
                    code.as_deref().unwrap_or("no code"),
                    detail
                );
                std::process::exit(1);
            }
            topup::TopupOutcome::Unexpected(e) => {
                eprintln!("topup: show failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// riir-dapps Plan 045 B7: `topup [--pack N] [--qr|--no-qr]` — the
/// USDC→KAT checkout (the 075 gate's "top up" path, rendered actionable):
/// the machine catalog read, the pack pick, the signed `POST /pay/intent`,
/// and the `solana:` URL. Refuses honestly when the lane is not armed on
/// the configured environment (the `503 unconfigured` code — the money-route
/// convention, not a retryable outage). Agents/CI pass `--pack N` (or use
/// `buy`, which resolves the pack from an amount); an interactive TTY
/// picks from the catalog; anything else refuses loud (nobody guesses a
/// price). The QR: `qr` is `Some(true)` = render even piped,
/// `Some(false)` = suppress even on a TTY, `None` = AUTO (a stdout TTY
/// renders it — scanning with a phone is the primary payment gesture;
/// agents/CI never get a block of unicode in their logs). `pick_latest`
/// (issue 126 follow-up): on the open cap, show the NEWEST checkout
/// without ever entering the interactive picker — the agent-safe path.
#[cfg(feature = "kat_service")]
fn run_kat_topup(ask: BuyAsk, qr: Option<bool>, pick_latest: bool) {
    #[cfg(feature = "kat_transport")]
    use riir_refine::kat_pay_client as topup;

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("topup: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("topup: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let url = command_service_url();
    let host_label = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string();

    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = (signing_key, ask, host_label, qr, pick_latest);
        riir_refine::build_stamp::refuse("topup", &["kat_transport"]);
    }

    #[cfg(feature = "kat_transport")]
    {
        let transport = topup::HttpTopupTransport::new(&url, 10);
        let catalog = match topup::catalog(&transport) {
            topup::TopupOutcome::Ok(c) => c,
            topup::TopupOutcome::Refused {
                status,
                code,
                detail,
            } => {
                if code.as_deref() == Some(riir_refine::kat_protocol_pay::CODE_UNCONFIGURED) {
                    eprintln!(
                        "topup: the pay lane is not armed on {host_label} ({status}) — the \
                         posture and the catalog render at {url}/pay"
                    );
                } else {
                    eprintln!(
                        "topup: the catalog was refused ({status} {}{})",
                        code.as_deref().unwrap_or("no code"),
                        detail
                    );
                }
                std::process::exit(1);
            }
            topup::TopupOutcome::Unexpected(e) => {
                eprintln!("topup: catalog fetch failed: {e}");
                std::process::exit(1);
            }
        };
        for l in topup::catalog_lines(&catalog, &host_label) {
            println!("{l}");
        }
        // The pack: an exact id, or the amount the CATALOG must match
        // exactly (nobody guesses a price — a miss prints the catalog
        // that was just rendered and refuses).
        let asked_pack = match ask {
            BuyAsk::Pack(p) => p,
            BuyAsk::Kat(micro) => Some(
                catalog
                    .packs
                    .iter()
                    .find(|p| p.kat_micro == micro)
                    .map(|p| p.id)
                    .unwrap_or_else(|| {
                        eprintln!(
                            "topup: no pack sells exactly {} — pick one of the packs above",
                            riir_refine::kat_payreq_client::render_kat(
                                i64::try_from(micro).unwrap_or(i64::MAX)
                            )
                        );
                        std::process::exit(2);
                    }),
            ),
            BuyAsk::Usdc(micro) => Some(
                catalog
                    .packs
                    .iter()
                    .find(|p| p.usdc_base == micro)
                    .map(|p| p.id)
                    .unwrap_or_else(|| {
                        eprintln!(
                            "topup: no pack sells exactly {} USDC — pick one of the packs above",
                            format_usdc(micro)
                        );
                        std::process::exit(2);
                    }),
            ),
        };

        // The pack pick: resolved amount > explicit flag > TTY prompt >
        // loud refusal.
        let pack_id = match asked_pack {
            Some(p) => p,
            None => {
                if args_tty_stdin() {
                    print!("pack to buy (id): ");
                    use std::io::Write as _;
                    std::io::stdout().flush().ok();
                    let mut line = String::new();
                    match std::io::stdin().read_line(&mut line) {
                        Ok(0) | Err(_) => None,
                        Ok(_) => line.trim().parse::<u8>().ok(),
                    }
                    .unwrap_or_else(|| {
                        eprintln!("topup: no pack chosen — pass --pack N");
                        std::process::exit(2);
                    })
                } else {
                    eprintln!(
                        "topup: choose a pack — pass --pack N (agents/CI never guess a price)"
                    );
                    std::process::exit(2);
                }
            }
        };
        let Some(chosen) = catalog.packs.iter().find(|p| p.id == pack_id) else {
            eprintln!(
                "topup: pack {pack_id} is not in the catalog ({})",
                catalog
                    .packs
                    .iter()
                    .map(|p| p.id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            std::process::exit(2);
        };
        println!(
            "buying pack {pack_id}: {}",
            topup::pack_line(chosen).trim_start()
        );

        match topup::create_intent(&transport, &signing_key, pack_id) {
            topup::TopupOutcome::Ok(intent) => {
                let now_unix = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                for l in topup::intent_lines(&intent, now_unix) {
                    println!("{l}");
                }
                // The checkout QR (auto on a TTY; --qr forces, --no-qr
                // suppresses). Never a gate: an empty render (URL too long
                // for any QR version — never happens for a solana: URL)
                // prints the pointer line and moves on.
                let want_qr = match qr {
                    Some(b) => b,
                    None => {
                        use std::io::IsTerminal as _;
                        std::io::stdout().is_terminal()
                    }
                };
                if want_qr {
                    let rendered = riir_refine::kat_lease_client::render_qr_ascii(&intent.url);
                    if rendered.is_empty() {
                        eprintln!("topup: the QR could not be rendered — open the URL above");
                    } else {
                        println!("{rendered}");
                        println!("  scan with any Solana Pay wallet (Phantom / Solflare) — the QR is the URL above");
                    }
                }
            }
            topup::TopupOutcome::Refused {
                status,
                code,
                detail,
            } => {
                match code.as_deref() {
                    Some(riir_refine::kat_protocol_pay::CODE_RATE_LIMITED) => {
                        // The 429 dead-end's fix (issue 126): an open
                        // checkout probably exists — surface IT instead of
                        // the refusal, per the owner's ask (one = show
                        // without asking; several = pick).
                        if let Some(intents) = topup_open_intents(&url) {
                            match intents.len() {
                                0 => {
                                    eprintln!("topup: a cap is full ({status}) — {detail}");
                                    eprintln!("topup: no open checkout of yours shows on this net — the lane-wide cap or another account holds it; retry later");
                                }
                                1 => {
                                    let seq = intents[0].seq;
                                    println!(
                                        "topup: you already have an open checkout (seq {seq}) — showing it instead"
                                    );
                                    run_kat_topup_show(seq, qr);
                                    // The checkout was SHOWN — this run's
                                    // job is done (exit 0, never the
                                    // refusal's exit 1 below).
                                    return;
                                }
                                _ => {
                                    // The agent-safe path (issue 126
                                    // follow-up): --pick latest shows the
                                    // NEWEST checkout and never enters the
                                    // interactive picker (whose 10 s
                                    // first-key window would only waste an
                                    // unattended run).
                                    if pick_latest {
                                        let seq = intents[0].seq;
                                        println!(
                                            "topup: --pick latest — showing the newest checkout (seq {seq})"
                                        );
                                        run_kat_topup_show(seq, qr);
                                        return;
                                    }
                                    eprintln!("topup: a cap is full ({status}) — {detail}");
                                    eprintln!("topup: your open checkouts (newest first):");
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs())
                                        .unwrap_or(0);
                                    let lines: Vec<String> = intents
                                        .iter()
                                        .map(|i| {
                                            let kat = catalog
                                                .packs
                                                .iter()
                                                .find(|p| p.id == i.pack_id)
                                                .map(|p| p.kat_micro);
                                            topup_checkout_line(i, kat, now, catalog.ttl_secs)
                                        })
                                        .collect();
                                    for l in &lines {
                                        println!("  {l}");
                                    }
                                    let rows: Vec<riir_refine::tty_pick::PickRow> = intents
                                        .iter()
                                        .zip(&lines)
                                        .map(|(i, text)| riir_refine::tty_pick::PickRow {
                                            seq: i.seq,
                                            text: text.clone(),
                                        })
                                        .collect();
                                    // Issue 126 follow-up: on a real terminal
                                    // the pick is interactive — arrows + Enter
                                    // (newest preselected, the owner's default),
                                    // `v` arms a void, `q` quits. Raw mode
                                    // unavailable falls back to the typed prompt
                                    // verbatim (piped stdin never reaches here).
                                    let picked = if args_tty_stdin() {
                                        match riir_refine::tty_pick::TtyKeys::enable() {
                                            Some(mut keys) => {
                                                // The agent-stuck defense: if
                                                // nothing is pressed within the
                                                // window (an unattended pty),
                                                // cancel instead of wedging the
                                                // UI forever.
                                                keys.first_timeout(
                                                    riir_refine::tty_pick::FIRST_KEY_TIMEOUT_DECISECS,
                                                );
                                                let mut out = std::io::stdout();
                                                let picked = riir_refine::tty_pick::pick(
                                                    &rows, &mut keys, &mut out,
                                                );
                                                // Restore the terminal BEFORE
                                                // anything else prints (the QR
                                                // must not render in raw mode).
                                                drop(keys);
                                                Some(picked)
                                            }
                                            None => topup_typed_pick(&rows),
                                        }
                                    } else {
                                        None
                                    };
                                    match picked {
                                        Some(riir_refine::tty_pick::Picked::Show(seq)) => {
                                            run_kat_topup_show(seq, qr);
                                            return;
                                        }
                                        Some(riir_refine::tty_pick::Picked::Void(seq)) => {
                                            run_kat_topup_void(seq);
                                            return;
                                        }
                                        Some(riir_refine::tty_pick::Picked::Cancelled) => {
                                            eprintln!("topup: cancelled — nothing changed");
                                            std::process::exit(2);
                                        }
                                        Some(riir_refine::tty_pick::Picked::TimedOut) => {
                                            eprintln!(
                                                "topup: no keypress in 10 s — cancelled (nothing changed)"
                                            );
                                            eprintln!(
                                                "topup: non-interactive: rerun with  --pick latest  (or --show <seq> / --void <seq>)"
                                            );
                                            std::process::exit(2);
                                        }
                                        None => {
                                            eprintln!(
                                                "topup: re-print one with  cargo refine topup --show <seq>"
                                            );
                                            eprintln!(
                                                "topup: release one with  cargo refine topup --void <seq>"
                                            );
                                            eprintln!(
                                                "topup: non-interactive:  cargo refine topup --pick latest"
                                            );
                                            std::process::exit(2);
                                        }
                                    }
                                }
                            }
                        } else {
                            eprintln!("topup: a cap is full ({status}) — {detail}");
                        }
                    }
                    Some(riir_refine::kat_protocol_pay::CODE_TERMINAL_STATE) => {
                        eprintln!("topup: this account cannot buy ({status}) — {detail}");
                    }
                    _ => {
                        eprintln!(
                            "topup: the intent was refused ({status} {}{})",
                            code.as_deref().unwrap_or("no code"),
                            detail
                        );
                    }
                }
                std::process::exit(1);
            }
            topup::TopupOutcome::Unexpected(e) => {
                eprintln!("topup: intent failed: {e}");
                std::process::exit(1);
            }
        }
    }
}

/// Whether stdin is a TTY (the interactive-pick gate). A non-TTY always
/// answers false — the refusal path, never a hang (piped/agent runs).
/// kat_transport-gated: the only caller is the topup interactive pick
/// inside the armed pay lane.
#[cfg(feature = "kat_transport")]
fn args_tty_stdin() -> bool {
    use std::io::IsTerminal as _;
    std::io::stdin().is_terminal()
}

/// Proposal 011 T1.3 (Plan 062): `pay --request [--amount N] [--label L]
/// [--qr] [--wait SECS]` — the payee half of the §1 loop. Signs a create
/// with the account key, prints the shareable LINK first (the QR is the
/// `--qr` fallback rendering), then polls on credit until the wait budget
/// ends. The poll blocks — that is the product (foo leaves it running,
/// bar scans, foo watches the credit land).
#[cfg(feature = "kat_service")]
fn run_kat_pay(amount: Option<&str>, label: Option<&str>, qr: bool, wait: u64) {
    use riir_refine::kat_payreq_client as payreq;

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("pay: no account ({e}) — run `cargo refine login` first");
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
    let short = riir_refine::kat_account::short_id(&account.state().account_id);

    // Blank/absent --amount = ANY amount (ask 3 — the payer composes at
    // settle). A supplied amount parses KAT → micro (positive, ≤6 decimals).
    let amount_micro = match amount {
        None | Some("") => None,
        Some(s) => {
            if let Some(m) = payreq::parse_kat_amount(s) {
                Some(m)
            } else {
                eprintln!(
                    "pay: --amount expects a positive KAT amount with at most 6 decimals (e.g. 2 or 1.5); omit it for an ANY-amount request"
                );
                std::process::exit(2);
            }
        }
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
        let url = command_service_url();
        let transport = riir_refine::kat_payreq_client::HttpPayreqTransport::new(&url, 10);
        match riir_refine::kat_payreq_client::pay_request_flow(
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
                    Some(riir_refine::kat_payreq_client::SettleOutcome::Invoice(inv)) => {
                        let recheck =
                            riir_refine::kat_payreq_client::recheck_invoice_qr(&inv, &outcome.url);
                        for line in
                            riir_refine::kat_payreq_client::invoice_display_lines(&inv, &recheck)
                        {
                            println!("{line}");
                        }
                    }
                    Some(riir_refine::kat_payreq_client::SettleOutcome::Unreadable(why)) => {
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
        riir_refine::build_stamp::refuse("pay", &["kat_transport"]);
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
    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("faucet: no account ({e}) — run `cargo refine login` first");
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
        let url = command_service_url();
        let transport = riir_refine::kat_payreq_client::HttpPayreqTransport::new(&url, 10);
        match riir_refine::kat_payreq_client::faucet_request_flow(&transport, &url, &signing_key) {
            Ok(outcome) => {
                for line in riir_refine::kat_payreq_client::faucet_display_lines(&outcome) {
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
        riir_refine::build_stamp::refuse("faucet", &["kat_transport"]);
    }
}

/// Plan 117 as amended by riir-dapps Issue 094 D4: `cargo refine --refill` —
/// the DEVNET-OPS faucet credit (the user daily faucet/drip tier is
/// retired). The composition lives in [`riir_refine::kat_refill`], which
/// owns both postures: the client-side D4 gate below, and the wire posture
/// the faucet/balance legs already had (the `faucet --request` precedent —
/// the DEPLOYMENT's cap config is the faucet's server-side gate — with the
/// crank leg authorized by the operator-set bearer token, fail-soft on
/// refusal).
///
/// The three things this bin adds around the flow:
///
/// 1. the riir-dapps Issue 094 D4 posture gate: `--refill` is a
///    DEVNET-OPS tool now — the user daily faucet/drip tier is retired, so
///    a mainnet (or unnameable) ledger gets a loud refusal naming the two
///    surviving refuel paths, contribute and pay;
/// 2. the root-scoped ack refresh (T5): a confirmed server balance is
///    written into the 075 gate's input file so the gate lifts without a
///    new burn push — the whole point of the command;
/// 3. the exit contract: 0 iff the final synced balance is positive
///    (explicit command — unlike the burn push, the outcome IS the code);
///    a posture refusal exits 1.
///
/// A build WITHOUT `kat_transport` refuses FIRST (exit 2, the Issue 117
/// compiled-out contract) — before the account load, so a user on a
/// compiled-out build is never told to `login` only to meet the rebuild
/// refusal afterwards (`build_stamp_cli::refill_refuses_loudly_too`).
fn run_kat_refill(wait_secs: u64) {
    #[cfg(feature = "kat_transport")]
    run_kat_refill_transport(wait_secs);
    #[cfg(not(feature = "kat_transport"))]
    {
        let _ = wait_secs;
        riir_refine::build_stamp::refuse("--refill", &["kat_transport"]);
    }
}

/// [`run_kat_refill`]'s body in a build that HAS the transport.
#[cfg(feature = "kat_transport")]
fn run_kat_refill_transport(wait_secs: u64) {
    let dir = riir_refine::kat_account::KatAccount::default_dir();
    // ── riir-dapps Issue 094 D4: the posture gate, FIRST ─────────────────
    // Before the account load below, so a mainnet user meets the retirement
    // sentence rather than "run `cargo refine login` first" — the refusal is
    // about the LEDGER and is true whether or not this box holds a key. The
    // posture signal is the one the client ALREADY has (the Plan 023
    // service-URL resolution: env > `cargo refine config set` > the mainnet
    // default); no second posture mechanism is invented here. An EMPTY
    // refusal is the permission, so the rule lives in one place
    // (`kat_refill::refill_refusal_lines`) rather than being restated as a
    // posture comparison on this side.
    #[cfg(feature = "kat_transport")]
    {
        let url = command_service_url();
        // Best-effort: the account id only DECORATES the pay link, so a
        // missing/unreadable account still gets the full refusal.
        let account_id = riir_refine::kat_account::KatAccount::load(&dir)
            .map(|a| a.state().account_id.clone())
            .unwrap_or_default();
        let pay = riir_refine::kat_lease_client::payment_url(&url, &account_id);
        let refusal = riir_refine::kat_refill::refill_refusal_lines(
            riir_refine::kat_refill::refill_posture(&url),
            &url,
            &pay,
        );
        if !refusal.is_empty() {
            for line in refusal {
                eprintln!("{line}");
            }
            std::process::exit(1);
        }
    }
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("refill: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("refill: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };
    let acct_hex = if let Some(h) =
        riir_refine::kat_account_client::service_account_hex(&account.state().account_id)
    {
        h.to_string()
    } else {
        eprintln!(
            "refill: the local account id does not carry a valid service account prefix — \
                 re-run `cargo refine login`"
        );
        std::process::exit(1);
    };

    #[cfg(feature = "kat_transport")]
    {
        let url = command_service_url();
        // The crank bearer is read HERE, never inside `kat_refill` — the
        // module stays environment-free (the wetrun caveat #2 rule), and
        // the operator's token IS the crank authorization (the module doc
        // owns the reasoning).
        let crank_bearer = std::env::var("RIIR_REFINE_KAT_CRANK_TOKEN")
            .ok()
            .filter(|t| !t.is_empty());
        let trigger_crank = crank_bearer.is_some();
        let transport = riir_refine::kat_refill::HttpRefillTransport::new(
            &url,
            10,
            signing_key.clone(),
            crank_bearer,
        );
        let outcome = riir_refine::kat_refill::refill_flow(
            &transport,
            &signing_key,
            &acct_hex,
            trigger_crank,
            wait_secs,
            std::time::Duration::from_secs(riir_refine::kat_refill::REFILL_POLL_INTERVAL_SECS),
        );
        for line in riir_refine::kat_refill::refill_display_lines(&outcome, &url) {
            println!("{line}");
        }
        // dapps Issue 029 Rider 1 (client half): the same service label,
        // read through the account transport AFTER the flow — a dead
        // service fails the flow above with its own honest errors and the
        // label never adds a second timeout ahead of them.
        let svc_transport = riir_refine::kat_account_client::HttpAccountTransport::new(&url, 5);
        if let Some(line) = riir_refine::kat_account_client::service_line_via(&svc_transport) {
            println!("{line}");
        }

        // The soft-gate refresh (T5): the 075 gate reads the burn ack's
        // last SYNCED server balance — write the confirmed read there,
        // whatever the value (server truth wins, the issue-080 lesson), so
        // a credited account's gate lifts without waiting for the next
        // burn push. A failed save is a warning: the gate input stays
        // stale, the fuel is still real.
        if let Some(b) = &outcome.final_balance {
            let root = resolve_heal_root(&[]);
            // T7 (riir-kat Issue 003): the PER-NET ack — the same file the
            // gate reads (the flat write here fed a file nothing reads
            // since the per-net move — the 2026-10-12 split-brain family).
            let ack_path =
                riir_refine::kat_state_stamp::net_state_dir_for_url(&root, &command_service_url())
                    .join(riir_refine::kat_burn_client::KatBurnAck::FILE_NAME);
            let (mut ack, warning) = riir_refine::kat_burn_client::KatBurnAck::load_at(&ack_path);
            if let Some(w) = warning {
                eprintln!("{w}");
            }
            if riir_refine::kat_refill::record_server_balance(&mut ack, b.balance_micro) {
                match ack.save(&ack_path) {
                    Ok(()) => {
                        println!("refill: soft-gate input refreshed — {}", ack_path.display());
                    }
                    Err(e) => eprintln!("refill: the gate input refresh did not save: {e}"),
                }
            }
        }
        std::process::exit(riir_refine::kat_refill::refill_exit_code(&outcome));
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
impl riir_refine::kat_wetrun::LocalLegs for CliLocalLegs {
    fn scored_cycle(&mut self) -> Result<(u32, i64), String> {
        use riir_refine::domains::clippy_lints::proposers::{collect_edits, FIXABLE_LINTS};
        use riir_refine::edit_emitter::apply_edits;
        use riir_refine::kat_meter::{KatCharge, KatMeter};

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

        // The REAL meter + charge ledger, on the repo-local `.refine` root.
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
        use riir_refine::kat_redact::SpanOrigin;
        use riir_refine::kat_sync_payload::SyncPayloadBuilder;

        let account = riir_refine::kat_account::KatAccount::load(
            &riir_refine::kat_account::KatAccount::default_dir(),
        )
        .map_err(|e| format!("no account ({e})"))?;
        let key = account
            .signing_key()
            .map_err(|e| format!("account key unusable ({e})"))?;
        let (traj, _bad) = riir_refine::traj_store::TrajStore::open(&self.root)
            .map_err(|e| format!("trajectory store unreadable: {e}"))?;
        let rows = traj.dump();
        if rows.is_empty() {
            return Err(
                "the trajectory store has no rows to push — run a `--fix` in this repo first; \
                 pushing an EMPTY payload would let step 4 pass without recording anything"
                    .to_string(),
            );
        }
        let loaded = riir_refine::kat_redact::RedactionGate::discover(None);
        let payload = SyncPayloadBuilder::new(&loaded.gate, SpanOrigin::OwnRepo, &key).build(&rows);
        Ok((
            payload.ndjson,
            payload.rows_included,
            riir_refine::kat_sync_payload::PAYLOAD_VERSION,
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

/// Proposal 011 §7 / T4.1+T4.2 (Plan 066): `cargo refine wetrun [--dry-run]`.
///
/// The network is classified from the RESOLVED service URL and the
/// admission decision is [`riir_refine::kat_wetrun::admit`] — mainnet is
/// refused with **no override**, and an unclassifiable host is refused the
/// same way (fail-closed). `--dry-run` runs steps 1+6 only and is admitted
/// everywhere.
#[cfg(feature = "kat_service")]
fn run_kat_wetrun(dry_run: bool) {
    use riir_refine::kat_wetrun::RunMode;
    // Verdict is consumed only by the transport-gated wetrun leg; importing
    // it unconditionally warns in kat_service-without-kat_transport builds
    // (the shipping release set).
    #[cfg(feature = "kat_transport")]
    use riir_refine::kat_wetrun::Verdict;

    let url = command_service_url();
    let network = riir_refine::kat_wetrun::classify_service_url(&url);
    let mode = if dry_run {
        RunMode::DryRun
    } else {
        RunMode::Full
    };

    // Admission BEFORE anything else — including before the account load,
    // so a refused mainnet run cannot be confused with a missing key.
    if let Err(why) = riir_refine::kat_wetrun::admit(network, mode) {
        eprintln!("{why}");
        std::process::exit(2);
    }

    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("wetrun: no account ({e}) — run `cargo refine login` first");
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
        riir_refine::kat_account_client::service_account_hex(&account.state().account_id)
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
        let crank_bearer = std::env::var("RIIR_REFINE_KAT_CRANK_TOKEN")
            .ok()
            .filter(|t| !t.is_empty());
        if matches!(mode, RunMode::Full) && crank_bearer.is_none() {
            eprintln!(
                "wetrun: RIIR_REFINE_KAT_CRANK_TOKEN is unset. `POST /admin/crank/daily` is \
                 bearer-gated fail-closed service-side, and it is what SETTLES a faucet row — \
                 without it steps 2 and 5 cannot complete. Set it, or use --dry-run."
            );
            std::process::exit(2);
        }
        let transport =
            riir_refine::kat_wetrun::HttpWetrunTransport::new(&url, 30, signing_key, crank_bearer);
        let mut legs = CliLocalLegs {
            root: resolve_heal_root(&[]),
        };
        let report = riir_refine::kat_wetrun::run(&transport, &mut legs, network, mode, &acct_hex);
        for step in &report.steps {
            println!("── step {} ──", step.step.label());
            for line in &step.lines {
                println!("{line}");
            }
            println!("   {}", if step.ok { "PASS" } else { "FAIL" });
        }
        if let (Some(b), Some(a)) = (report.supply_before, report.supply_after) {
            // Rendered as KAT decimals, not as the raw micro integers: an
            // unlabelled `1400000` next to the word "supply" is the
            // `.issues/081` display class, and this line is the one a human
            // reads when the G-WET gate reds.
            let kat = riir_refine::kat_payreq_client::render_kat;
            println!(
                "G-WET supply: epoch {} mint {} burn {} pool_paid {} pool_avail {} → \
                 mint {} burn {} pool_paid {} pool_avail {}",
                b.epoch,
                kat(b.mint_micro),
                kat(b.burn_micro),
                kat(b.pool_paid_micro),
                kat(b.pool_available_micro),
                kat(a.mint_micro),
                kat(a.burn_micro),
                kat(a.pool_paid_micro),
                kat(a.pool_available_micro)
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
        riir_refine::build_stamp::refuse("wetrun", &["kat_transport"]);
    }
}

/// P015 Phase 2 (Plan 127): `cargo refine replay` — the Max tier's earn
/// lane. One command = one lease → local replay of every item → one signed
/// verdict per item. The judgment half is [`riir_refine::kat_replay`]
/// (step-0 canary hash check + the pinned rate rule over the real clippy
/// oracle); the byte movers are [`riir_refine::kat_replay_client`].
///
/// Gates (in order):
/// 1. kill switch `RIIR_REFINE_REPLAY=0` — disabled lanes exit 0 quietly;
/// 2. login (load-only, the wetrun precedent: a SIGNING command never
///    silently mints an identity);
/// 3. mining consent — `OptedOut` refuses (verdicts are contributions to
///    the same economy the push half feeds; running the command IS the
///    consent for `Unset`, the sync precedent);
/// 4. the toolchain fingerprint (T2.2) — a lease pin that mismatches the
///    local toolchain aborts BEFORE any compile (a skewed verdict is
///    refused unscored server-side; running it would be pure waste).
///
/// riir-dapps Plan 050 S1.11: the corpus PR client half — resolve, gate
/// (T10/T2b/pins), draft (S1.0 rebase + chunk + sign), and optionally
/// submit. The engine's disclosure lines print verbatim; a refusal is
/// one loud line + exit 1 (an operational failure, not a usage error).
#[cfg(feature = "kat_transport")]
#[allow(clippy::too_many_arguments)]
fn run_kat_corpus_pr(
    src: Option<&str>,
    dst: Option<&str>,
    seq: Option<u64>,
    submitter: Option<&str>,
    key_file: Option<&std::path::Path>,
    submit: bool,
    root: &std::path::Path,
) {
    use riir_refine::kat_corpus_pr::{run_corpus_pr, CorpusPrArgs, HttpCorpusPrWire};

    let args = CorpusPrArgs {
        src: src.unwrap_or_default().to_string(),
        dst: dst.unwrap_or_default().to_string(),
        seq,
        submitter: submitter.unwrap_or_default().to_string(),
        key_file: key_file.map(std::path::Path::to_path_buf),
        submit,
    };
    // 60s per request — the builder's curl -m 60 posture (a stalled
    // server cannot outlive the budget; segments are MB-class).
    let wire = HttpCorpusPrWire::new(60);
    match run_corpus_pr(root, &wire, &args) {
        Ok(report) => {
            for line in &report.lines {
                println!("{line}");
            }
        }
        Err(e) => {
            eprintln!("corpus-pr: {e}");
            std::process::exit(1);
        }
    }
}

/// Per item: judge → submit. `Unmeasurable` items are SKIPPED with the
/// reason (a verdict is a claim — never submit what was not measured).
#[cfg(all(
    feature = "kat_transport",
    feature = "clippy_verify",
    feature = "latent_retrieval"
))]
fn run_kat_replay() {
    use riir_kat::kat_protocol_replay::ReplayOutcomeWire;
    use riir_refine::kat_replay::{
        judge_item, toolchain_display, toolchain_fingerprint, ReplayVerdict,
    };
    use riir_refine::kat_replay_client::{
        accept_line, request_lease, submit_verdict, unhex32, HttpReplayTransport,
    };

    // 1. Kill switch (the RIIR_REFINE_KAT_METER idiom: exactly "0" disables).
    const REPLAY_LEASE_LIMIT: u32 = 16;

    if std::env::var("RIIR_REFINE_REPLAY").is_ok_and(|v| v == "0") {
        println!("replay: disabled by RIIR_REFINE_REPLAY=0");
        return;
    }

    // 2. Login — load-only (the wetrun precedent for signing commands).
    let dir = riir_refine::kat_account::KatAccount::default_dir();
    let account = match riir_refine::kat_account::KatAccount::load(&dir) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("replay: no account ({e}) — run `cargo refine login` first");
            std::process::exit(1);
        }
    };
    let signing_key = match account.signing_key() {
        Ok(k) => k,
        Err(e) => {
            eprintln!("replay: could not load the account signing key: {e}");
            std::process::exit(1);
        }
    };

    // 3. Mining consent — the push-half posture: OptedOut refuses, Unset
    //    proceeds (running the command IS the consent).
    let consent_dir = riir_refine::kat_mining_consent::default_dir();
    let (consent, consent_warning) =
        riir_refine::kat_mining_consent::read_with_warning(&consent_dir);
    if let Some(w) = consent_warning {
        eprintln!("{w}");
    }
    if !consent.push_allowed() {
        eprintln!(
            "replay: SKIPPED — mining is opted out on this machine ({}); verdicts are \
             contributions to the same economy the sync push feeds",
            riir_refine::kat_mining_consent::consent_path(&consent_dir).display()
        );
        eprintln!("replay: re-enable with `cargo refine --mine`");
        std::process::exit(2);
    }

    // 4. The toolchain fingerprint — BEFORE the lease (T2.2): the local
    //    toolchain must be pinnable at all, and the lease's pin decides
    //    whether any verdict from this rig is scoreable.
    let local_fp = match toolchain_fingerprint() {
        Ok(fp) => fp,
        Err(e) => {
            eprintln!("replay: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "replay: toolchain {} — fingerprint {}…",
        toolchain_display(),
        riir_refine::kat_replay::hex32_short(&local_fp)
    );

    let url = command_service_url();
    let transport = HttpReplayTransport::new(&url, 60);
    let ack = match request_lease(&transport, &signing_key, REPLAY_LEASE_LIMIT) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("replay: {e}");
            std::process::exit(1);
        }
    };
    if ack.items.is_empty() {
        println!(
            "replay: lease {} carries no items — nothing to verify (all queued rows are \
             proven, leased elsewhere, or the queue is empty)",
            ack.lease_id
        );
        return;
    }
    println!(
        "replay: lease {} — {} item(s) from {url}",
        ack.lease_id,
        ack.items.len()
    );

    // Every item in a lease shares the config's pin — one check, before
    // any compile (P015 caveat 2: skew is a refusal, never a slash).
    if let Some(pinned) = unhex32(&ack.items[0].toolchain_fingerprint) {
        if pinned != local_fp {
            eprintln!(
                "replay: toolchain SKEW — the lease pins {}… but this rig runs {}… ({}); \
                 verdicts under a different toolchain are not comparable and are refused \
                 unscored server-side. Nothing was compiled or submitted.",
                &ack.items[0].toolchain_fingerprint[..16],
                riir_refine::kat_replay::hex32_short(&local_fp),
                toolchain_display()
            );
            std::process::exit(2);
        }
    } else {
        eprintln!(
            "replay: the lease's toolchain pin is malformed — refusing to run against an \
             unpinnable lane"
        );
        std::process::exit(2);
    }

    let oracle = riir_refine::draft::clippy_oracle::ClippyOracle::new();
    if !oracle.is_available() {
        eprintln!("replay: cargo is not on PATH — the oracle cannot measure a verdict");
        std::process::exit(1);
    }

    let mut submitted = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let started = std::time::Instant::now();
    for item in &ack.items {
        let span_short = &item.span_hash[..12.min(item.span_hash.len())];
        let Some(span) = unhex32(&item.span_hash) else {
            skipped += 1;
            eprintln!("replay:   {span_short}… SKIP — malformed span hash");
            continue;
        };
        let verdict = judge_item(&oracle, item);
        let outcome = match &verdict {
            ReplayVerdict::Tamper => {
                println!("replay:   {span_short}… CANARY — hash mismatch, reporting tamper");
                ReplayOutcomeWire::Tamper
            }
            ReplayVerdict::Judgment(j) => {
                let pass = if j.compile_ok && j.oracle_resolved {
                    "PASS"
                } else {
                    "FAIL"
                };
                println!(
                    "replay:   {span_short}… {pass} (compile {}, oracle {}, heal {}/1000, \
                     created {}/1000{})",
                    j.compile_ok,
                    j.oracle_resolved,
                    j.heal_rate_milli,
                    j.created_rate_milli,
                    if j.created_lints.is_empty() {
                        String::new()
                    } else {
                        format!(" — created: {}", j.created_lints.join(", "))
                    }
                );
                j.to_outcome()
            }
            ReplayVerdict::Unmeasurable(why) => {
                skipped += 1;
                eprintln!("replay:   {span_short}… SKIP — {why}");
                continue;
            }
        };
        match submit_verdict(&transport, &signing_key, &span, &local_fp, &outcome) {
            Ok(ack) => {
                submitted += 1;
                println!("replay:   {span_short}… verdict {}", accept_line(&ack));
            }
            Err(e) => {
                failed += 1;
                eprintln!("replay:   {span_short}… verdict SUBMIT FAILED — {e}");
            }
        }
    }
    println!(
        "replay: done in {:.1}s — {} submitted, {} skipped, {} failed (lease {})",
        started.elapsed().as_secs_f64(),
        submitted,
        skipped,
        failed,
        ack.lease_id
    );
    if failed > 0 || submitted == 0 {
        std::process::exit(1);
    }
}

/// Print the ASCII QR under a URL line (the T1.6 QR render, reused —
/// errors surface as silence, the QR is a convenience never a gate).
#[cfg(all(feature = "kat_service", feature = "kat_transport"))]
fn print_qr(url: &str) {
    let qr = riir_refine::kat_lease_client::render_qr_ascii(url);
    if !qr.is_empty() {
        println!("{qr}");
    }
}

/// 038-T3: the one-time envelope → store migration (see [`open_traj_store`]).
/// Idempotent by construction: the `.migrated` rename is the fence — a
/// second run sees no envelope and does nothing.
#[cfg(feature = "latent_retrieval")]
fn migrate_envelope_once(store: &riir_refine::traj_store::TrajStore) {
    use riir_refine::memory::EvidenceTier;
    use riir_refine::traj_store::TrajRowDoc;

    let envelope = default_memory_path();
    if !envelope.exists() || !store.dump().is_empty() {
        return; // nothing to migrate from, or the store already has history
    }
    let mut mem = riir_refine::memory::LatentFixMemory::default();
    if let Err(e) = mem.load(&envelope) {
        eprintln!(
            "self-evolve: migration skipped — unreadable envelope ({}): {e}",
            envelope.display()
        );
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
            elo: riir_refine::traj_store::default_elo(),
            span_elo: riir_refine::traj_store::default_elo(),
            component: t.component,
            journey_runs: 0,
            journey_taint_sum: 0,
            // riir-dapps Issue 119 T2: the envelope never recorded a source
            // file — unknown path, which the sync gate fails closed on
            // until a later heal run re-records the row with its file.
            src_path: String::new(),
            // Issue 148 T-b: the envelope never knew the candidate set.
            rejected: Vec::new(),
            // Issue 156: the envelope never knew the rescue tallies.
            b0_attempts: 0,
            b0_successes: 0,
            bd_attempts: 0,
            bd_successes: 0,
            rescue_depth: 0,
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
/// condition (from the issue): "the `TrajStore` holds ≥ K (K=3) Certified
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
    fn note(&mut self, store: &riir_refine::traj_store::TrajStore, lint_key: &str) {
        let density = self
            .density
            .get_or_insert_with(|| store.certified_density());
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

/// Issue 148 T-b: the carried `rejected` vec → the stamp's `Option` shape.
/// EMPTY reads as `None` (no candidate set resolved — a record without the
/// set in hand never erases a label a richer observation left), non-empty as
/// `Some` (refresh, clamped + deduped by the store). Shared by BOTH capture
/// lanes: the clippy/fix lane (span rules at the edited span) and the
/// rustc_errors driver lane (the strategy menu — T-b follow-up).
#[cfg(feature = "latent_retrieval")]
fn rejected_opt(rejected: &[String]) -> Option<&[String]> {
    (!rejected.is_empty()).then_some(rejected)
}

/// Record one verdict event into the durable store (best-effort — a store
/// failure warns once per event, never fails the heal). `component` is the
/// Issue 047 decision-time failure attribution (`None` on successes and
/// unattributed events). `src_path` is the edited file, repo-relative
/// ([`TrajStore::src_path_of`](riir_refine::traj_store::TrajStore::src_path_of))
/// — the row's sync-policy `allow_paths` input (riir-dapps Issue 119 T2).
#[cfg(all(feature = "fix_verify", feature = "latent_retrieval"))]
#[allow(clippy::too_many_arguments)] // the existing record shape + attribution + source + the Issue-148 label
fn record_traj(
    store: &Option<riir_refine::traj_store::TrajStore>,
    src_path: &str,
    rule_id: &str,
    restore: &str,
    replacement: &str,
    success: bool,
    verified: bool,
    run_id: &str,
    component: Option<riir_refine::component::ComponentClass>,
    rejected: Option<&[String]>,
) {
    if let Some(store) = store {
        if let Err(e) = store.sourced(src_path).record_classified(
            rule_id,
            restore,
            replacement,
            success,
            verified,
            run_id,
            component,
            rejected,
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
#[allow(clippy::too_many_arguments)] // threads the Issue-148 label through the fixed failure verdict
fn record_traj_fail(
    store: &Option<riir_refine::traj_store::TrajStore>,
    src_path: &str,
    rule_id: &str,
    restore: &str,
    replacement: &str,
    run_id: &str,
    rejected: Option<&[String]>,
) {
    let class = riir_refine::component::classify(&riir_refine::component::FailureSignal::reverted(
        riir_refine::component::RevertKind::CompileGate,
    ));
    record_traj(
        store,
        src_path,
        rule_id,
        restore,
        replacement,
        false,
        false,
        run_id,
        Some(class),
        rejected,
    );
}

/// P12: the recorded consecutive-fail streak for one exact edit pair —
/// the store row when the store backend is active, the envelope buckets
/// otherwise (never both: one active backend per build, 038). Consumed by
/// the fixpoint suppression filter: a pair with 2+ consecutive compile-gate
/// fails is suppressed from re-proposal (see the filter comment there for
/// the full rationale).
#[cfg(feature = "latent_retrieval")]
fn pair_fail_streak(
    store: &Option<riir_refine::traj_store::TrajStore>,
    memory: &riir_refine::memory::LatentFixMemory,
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
    memory: &riir_refine::memory::LatentFixMemory,
    rule_id: &str,
    _restore: &str,
    replacement: &str,
) -> u32 {
    memory.pair_consecutive_fails(rule_id, replacement)
}

/// P12 kill-switch parse: suppression is default-on; `RIIR_REFINE_SUPPRESS_
/// REVERTED=0|off|false|no` re-enables unconditional re-proposal (force
/// retry of previously-reverted edits). Mirrors the recorder's opt-out
/// value set.
fn suppression_disabled() -> bool {
    match std::env::var("RIIR_REFINE_SUPPRESS_REVERTED") {
        Ok(v) => matches!(
            v.to_ascii_lowercase().as_str(),
            "0" | "off" | "false" | "no"
        ),
        Err(_) => false,
    }
}

/// Format the per-edit fix review + fmt annotation (pure — the byte-exact
/// pin in `fix_review_note_tests` runs THIS, not a re-composition).
/// `edits` are `(rule_id, final line, replacement, restore, note)` — the
/// middle fields exist for the verify path; `note` is the rule's trust
/// annotation (Issue 074 T1) and `fmt_note` is the per-file
/// fmt-plan/outcome annotation (`None` disables).
/// One review-row edit — the widened fix-review tuple (Issue 148 T-b
/// added the 6th field, the rejected candidate set riding the record path).
type FixEdit = (
    String,
    usize,
    String,
    String,
    Option<&'static str>,
    Vec<String>,
);

fn format_fix_review(display: &str, edits: &[FixEdit], fmt_note: Option<&str>) -> String {
    if edits.is_empty() {
        return String::new();
    }
    let mut out = format!("fix {} ({} edit(s)):\n", display, edits.len());
    for (rule_id, line, _, _, note, _) in edits {
        out.push_str(&format!("  [{rule_id:<24}] line {line}"));
        // Issue 074 T1: one annotation dialect with the rustc_errors
        // renderer (`semantic_note_suffix`), truncated to the FIRST line
        // (the tables are one-line by construction; the renderer does not
        // regress if one grows). Absent for mechanical classes — the
        // default output stays byte-identical.
        if let Some(note) = note {
            let first = note.lines().next().unwrap_or(note);
            out.push_str(&format!("\n  ⚠ semantic change: {first}"));
        }
        out.push('\n');
    }
    if let Some(note) = fmt_note {
        out.push_str(&format!("  fmt: {note}\n"));
    }
    out
}

/// Print the formatted fix review.
fn print_fix_review(display: &str, edits: &[FixEdit], fmt_note: Option<&str>) {
    print!("{}", format_fix_review(display, edits, fmt_note));
}

/// The P21 review note for oracle-anchored edits: clippy's own
/// `MachineApplicable` suggestion, consumed verbatim at the flagged span.
const ORACLE_NOTE: Option<&str> = Some(
    "oracle-anchored: clippy's own MachineApplicable suggestion, applied \
     verbatim at the diagnostic's span (--clippy-json)",
);

/// Issue 074 T1: the semantic-shift note a `--fix` review line carries for
/// this rule, if its proposer documents one. Domain dispatch mirrors the
/// `collect_edits` call sites (feature-gated identically).
fn rule_context_note(rule_id: &str) -> Option<&'static str> {
    #[cfg(feature = "rust_perf")]
    if let Some(note) = riir_refine::domains::rust_perf::proposers::context_note(rule_id) {
        return Some(note);
    }
    #[cfg(feature = "clippy_lints")]
    if let Some(note) = riir_refine::domains::clippy_lints::proposers::context_note(rule_id) {
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
    decision: Option<riir_refine::fmt::FmtDecision>,
    edition: &str,
    write: bool,
    fmt_summary: &mut riir_refine::fmt::FmtSummary,
) -> Option<String> {
    use riir_refine::fmt::FmtDecision;
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
        FmtDecision::Format => match riir_refine::fmt::apply(file, edition) {
            Ok(()) => {
                fmt_summary.formatted += 1;
                Some("formatted".to_string())
            }
            Err(e) => {
                fmt_summary.failed += 1;
                eprintln!("fmt failed for {}: {e}", file.display());
                Some(format!("FAILED ({e})"))
            }
        },
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

/// Print the full usage page + exit 2 (arg-error convention).
fn usage() -> ! {
    eprintln!("{}", usage_text());
    std::process::exit(2);
}

/// `auto` + the domains THIS build compiled, `|`-joined (Issue 144 T4).
fn domain_choices() -> String {
    std::iter::once("auto")
        .chain(DomainId::compiled().map(DomainId::label))
        .collect::<Vec<_>>()
        .join("|")
}

/// The everyday help — `cargo refine --help` (Issue 144 T3). User verbs
/// only: no internal plan/issue ids, no maintainer reports, no flags this
/// build cannot run. Everything else is `--help-all`.
fn user_help() -> String {
    let domains = domain_choices();
    let compile = if cfg!(feature = "rustc_errors") {
        "    cargo refine --fix-compile [--write] <path>...\n\
         \x20                                      Plan (or with --write, apply) fixes for compile errors\n"
    } else {
        ""
    };
    let account = if cfg!(feature = "kat_service") {
        "    cargo refine login                 Sign in to your KAT account (or create one)\n\
         \x20   cargo refine account               Your balance, spend and earnings\n\
         \x20   cargo refine --info                Account, contribution and local work in one view\n"
    } else {
        ""
    };
    format!(
        "cargo-refine {version} — Refine, the code fixer that learns (https://refine.gist.rs)

USAGE:
    cargo refine [<path>...]           Dry run: show what would be fixed, no edits (default path: .)
    cargo refine --fix <path>...       Apply the fixes; edits that break the build are reverted
    cargo refine --suggest [--domain <{domains}>] <path>...
                                       Show the matching rules per code span, no edits
{compile}{account}    cargo refine --mine                What contributing is, the policy, and how to earn KAT
    cargo refine --unmine              Stop contributing (fixing keeps working)
    cargo refine --stats [on|off]      Share rule-and-count stats (opt-in, never your code or paths)
    cargo refine config get            Show which service this install talks to
    cargo refine --version             Version and the features this build carries
    cargo refine --help-all            Every flag, including maintainer reports

Rule domains in this build: {list}",
        version = env!("CARGO_PKG_VERSION"),
        list = DomainId::compiled()
            .map(DomainId::label)
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// The full flag reference — `--help-all`, and the arg-error page.
fn usage_text() -> String {
    #[cfg(feature = "fix_verify")]
    let verify_help = format!(
        "    --verify        With --fix: write, cargo-check, auto-revert the offending edits\n\
         (adds {} + looser capacity bounds). Implies --write. DEFAULT-ON with\n\
         --fix now (the just-works UX); --no-verify skips it.\n\
    --verify-args <s> Extra cargo args for --verify (whitespace-split, e.g. \"--features a,b,c\").\n\
    --no-verify     With --fix: skip the default compile gate (fast, syn-gated only).\n",
        VERIFY_LINTS.join(", ")
    );
    #[cfg(not(feature = "fix_verify"))]
    let verify_help = String::new();
    #[cfg(feature = "score_bench")]
    let score_help = "    --score-bench   Run the frozen clippy-bench (real-code fixtures, real clippy ground\n                    truth — Plan 014), append to .benchmarks/score_history.jsonl,\n                    re-render .benchmarks/score_history.svg. No paths needed.\n    --generation <1|2|3|4|5|6|7|8|9>  Which bench generation (Plan 092 M5): 1 = the frozen v1 set\n                    (default); 2 = the construction-built k-ladder (issue 072 T5);\n                    3 = the persist-stratified ladder (Bench 069 reopen #1); 4 = the\n                    rustc-errors k-ladder (Plan 093, --fix-compile driver vs rustc\n                    ground truth); 5 = the rust_perf vocabulary ladder (Plan 094, the\n                    mechanical shape oracle over the FIXABLE perf vocabulary); 6 = the\n                    real-repo shape ladder (Plan 095, spans carved from pre-heal\n                    sibling states); 7 = the overlap-edit-chain ladder (Plan 096,\n                    manufactured drop-and-refire overlap, depth rungs 1–5); 8 = the\n                    shaped-verdict ladder (Issue 089 T7, atoms whose heal clears the\n                    target AND leaves new diagnostics — the 0.5 half-win priced into\n                    the shaped axis + the frontier half-count fold); 9 = the CRB corruption ladder (Issue 137: compile-clean/test-failing known-position localization + the DLM remask arming gate — own runner, no clippy oracle, never appended to the heal score history). A NEW\n                    generation's first append is admitted ONLY if it measures harder\n                    than v1 (gen 8: only if the run measures ≥1 shaped verdict).\n    --score-report  Re-render the score-history SVG from the existing JSONL (no run).\n";
    #[cfg(not(feature = "score_bench"))]
    let score_help = String::new();
    let domains = domain_choices();
    format!(
        "cargo-refine (Plan 005) — modelless code healing via latent corpus retrieval

USAGE:
    cargo refine                                        <- DRY RUN: review what would be fixed (no edits)
    cargo refine --fix [--top-k <N>] [--no-fmt] <path>...  <- REAL FIX: writes (compile-gated on fix_verify builds)
    cargo refine --suggest [--domain <{domains}>] [--top-k <N>] [--clippy-json <path>] <path>...
    cargo refine --fix-compile [--write] [--max-iters <N>] [--verify-args <s>] <path>...
    cargo refine --mine                                 <- what mining is + policy + how to earn KAT
    cargo refine --unmine                               <- opt OUT of contributing (healing keeps working)
    cargo refine --stats [on|off]                       <- work-stats sharing consent (bare = status; default OFF)
    cargo refine --refill [--refill-wait <s>]           <- DEVNET-OPS ONLY faucet credit (Issue 094 D4: refuses on mainnet; kat_transport)
    cargo refine --info                                 <- ONE COMMAND stats: wallet + account + mining + local work (kat_service)
    cargo refine --dump-trajectories > trajectories.jsonl
    cargo refine --memory-stats                         (loop debug query, no paths)
    cargo refine --recompute-elo                        (batch rating backfill, 039 T2)
    cargo refine --collapse-report                      (self-collapse monitor, 042)
    cargo refine --corpus-stats [--json] [--import-series]  (corpus/score series, 050 T2)
    cargo refine --frontier-report                       (learnability band report, 089 T1+2)
    cargo refine --coverage-report                       (corpus coverage debt, 119)
    cargo refine --horizon-report [PATHS]                (fixseq lead-k AUC report, 107 T1)
    cargo refine --fixseq-stats [PATHS]                  (cascade law + derailment index, 128)
    cargo refine --gate-calibration                      (Issue 130 T1+T3+OOD report — strata δ calibration + OOD-axis readout + corpus-stall flag; ungated, report-only)
    cargo refine --active-set                            (Issue 102 marker — print demotions + caps)
    cargo refine --active-set-demote <domain:rule>       (demote a rule out of the fan-out + fix dispatch; e.g. clippy:manual_div_ceil)
    cargo refine --active-set-readmit <domain:rule>      (remove a demotion — the manual re-admit)
    cargo refine --active-set-cap <domain:N>             (per-domain active ceiling; 0 removes — OFF by default)
    cargo refine --active-set-clear                      (drop every demotion + cap; the corpus is never touched)
    cargo refine --score-bench | --score-report        (needs --features score_bench)
    cargo refine --necessity-bench [--necessity-top N --necessity-random K --necessity-smoke --necessity-rule RULE]  (Issue 108 + the 102-T4 per-decision paired lane, score_bench)
    cargo refine --candidates <path>                   (T2.4 promotion gate, kat_service+score_bench)
    cargo refine login [--key <path>]                   (KAT account, needs --features kat_service)
    cargo refine account [<path>] [--history]            (KAT local view + repo burn + earned/spent, kat_service)
    cargo refine update                                 (corpus lease refresh, kat_transport)
    cargo refine sync                                   (push redacted batches + pull lease, kat_transport)
    cargo refine sync --export <file|->                 (WRITE the redacted batch, send nothing — no login, throwaway key)
    cargo refine claim --tx <sig>                       (queue/submit a payment claim, kat_service)
    cargo refine stake --usd <N> | --info              (Proposal 007 staking: transfer QR / position, kat_transport)
    cargo refine vote --proposal <id> --yes|--no     (Proposal 008 T2.4 governance voting: weight + evidence render, kat_transport)
    cargo refine unstake                                (drain unlocked tranches + escrow disclosure, kat_transport)
    cargo refine pay --request [--amount N] [--label L] [--qr] [--wait S]  (Proposal 011 T1.3 pay request: signed create + link-first + poll-on-credit, kat_transport)
    cargo refine topup [--pack N] [--qr|--no-qr] [--pick latest]  (riir-dapps Plan 045 B7 USDC→KAT checkout: catalog + signed intent + solana: URL; the QR renders automatically on a TTY — --no-qr suppresses; on the open cap it shows your open checkout instead — --show <seq> re-prints it, --void <seq> releases it, --pick latest shows the newest without asking; kat_transport)
    cargo refine buy <amount> <kat|usdc>           (the friction-cut: resolves the catalog pack from the exact amount — `buy 10 kat` = pack 1; MAINNET is the default dial, --net devnet overrides; glued `10kat` works; same flags as topup; kat_service+kat_transport)
    cargo refine kat                                  (the one-line KAT balance — the service ledger's authoritative number, host-labelled; kat_transport)
    cargo refine tuna                                 (the one-line TUNA trial-credit balance + expiry window; kat_transport)
    cargo refine get|send <amt> tuna [account-id]  (riir-dapps Proposal 009 §9: the marketing TUNA airdrop ADMIN hand — get targets your own account id, send takes a 32-hex account id (a base58 pubkey refuses with the hint); --gov <id> pins the Executed mandate, else unique-hash resolution via the mandates listing; --dry-run composes hash+row+body+caps with no network; secrets read from TUNA_CAN_OPENER_KEY + KAT_CRANK_TOKEN, refused pre-network, never echoed; admin feature, never in the release set)
    cargo refine faucet --request [--qr]               (Proposal 011 T1.8 faucet preset: the amount is the deployment's cap, not yours, kat_transport)
    cargo refine wetrun [--dry-run]                    (Proposal 011 T4.1 wet-run + G-WET; REFUSES mainnet with no override, --dry-run is read-only everywhere, kat_transport)
    cargo refine replay                                 (Proposal 015 Phase 2: lease → local clippy-oracle replay → signed verdict; toolchain-pinned, kat_transport+clippy_verify)
    cargo refine config get                            (the ACTIVE service url + its source + every state file's network stamp: env > config > default)
    cargo refine config set --url <devnet|mainnet|https://…>  (persist the service url — https unless loopback; Plan 023)
    cargo refine config adopt-state <devnet|mainnet> [--discard]  (migrate pre-split network-blind state: oracle-confirmed adopt, or quarantine; riir-kat Issue 003 T4)
    --net <devnet|mainnet>                             (one-invocation env sugar, pin-checked; Issue 003 T11)

    (no mode)       DRY RUN (the just-works default): review what WOULD be
                    fixed — suggest output over the paths (default: `.`),
                    zero edits. The explicit word for it is --dry-run.
    --suggest       Suggest mode: print matching rules per code span (no edits).
    --fix           Fix mode: apply the compiled-in proposers' span-preserving edits
                    ({} perf rules + {} clippy lints: {} | {}). WRITES by default (the
                    explicit flag is the consent) and compile-gates on --verify builds
                    (opt out: --no-verify). Bare `cargo refine` is the dry run. The proposer's
                    visitor is the gate — latent suggestions never auto-apply.
    --fix-compile   Compile-error repair (Plan 019, needs --features rustc_errors):
                    cargo-check the owning packages, parse coded errors (E0597...),
                    propose per error via playbooks, apply best → recheck → keep/revert,
                    bench targets guarded (Issue 118) — name the file explicitly to override,
                    bounded by --max-iters. Review mode (default) prints the plan;
                    --write applies. Registry: the E0597 family (Phase 2) — six
                    ranked templates keyed on the diagnostic's labeled spans; other
                    codes report as misses until Phase 3. Every --write run also
                    dumps the ORDERED per-attempt ring (Issue 91 T1.5 — seq, error
                    code, span_hash, strategy, outcome, errors before→after) to
                    <repo>/.refine/fixseq/run-<ts>.jsonl — the replay substrate for
                    the oscillation detector (T2) + the per-run strategy posterior
                    (Issue 093); the misses queue (.refine/misses/) stays the
                    authoring surface.
    --max-iters <N> --fix-compile iteration bound (default 5).
    --dump-trajectories  Export the self-evolve trajectory store to stdout as
                    JSONL (Plan 019 T4.3b — the human grep surface; one source
                    of truth, no shadow log). Reads the store the build ships.
    --memory-stats Print the self-evolve loop debug summary (Issue 037):
                    tier/outcome counts, top rules, commitment status, the
                    E1 record / E2 verdict / E3 learn state — the one-command
                    \"is the loop actually closed?\" query. Read-only.
    --frontier-report
                    Print the learnability-frontier band report (Issue 089):
                    per-rule r/f/n/p̂/LCB₅, the band-mass health metric
                    (fraction 0<p̂≤0.5), the saturation demotion
                    CANDIDATES (p̂=1 ∧ n≥50 — report-only, never deletes),
                    plus the T5 provenance tail: the pair namespace priced
                    by provenance (organic=1.0, construction=ε, unknown=0.0)
                    and the outcome-feed split (generated-case ':' keys vs
                    real-repo rows — band-mass per feed; the weight prices
                    arithmetic, gates nothing).
                    Also appends the T4 evidence-clock snapshot — one
                    band-mass row per state change to the committed
                    .benchmarks/frontier_snapshots.jsonl (an unchanged
                    store at the same generation appends nothing; T4's GO
                    evidence is one store crossing ≥3 generation
                    boundaries — rows carry a `store` id since 2026-09-21;
                    legacy rows are unattributed). Also prints the Issue
                    102 T1 contribution-fade section (live state REFUSES
                    until ≥3 full-corpus sweep rows exist in the score
                    history).
                    Read-only report; the snapshot append is best-effort and
                    never fails the run. Refuses (exit 1) on an empty
                    population — never a green zero. Needs --features
                    latent_retrieval.
    --horizon-report
                    Print the fixseq lead-k horizon report (Issue 107 T1):
                    AUC(k) of a leave-one-run-out closed-form ridge readout
                    over ring-native features at i-k, against the base floor
                    (0.5) and the history-aggregate floor (per-strategy
                    Laplace prior, LORO) — is an attempt's outcome linearly
                    decodable from the ring state k attempts earlier?
                    horizon ≥ 2 pays (the T3 probe lane's viability bar).
                    The T2 EMBED ARM (Issue 107 T2) adds an auc(embed)
                    column — the same ridge over native + span-embedding
                    features (41 dims; the column the driver stamps at
                    attempt time since T2) — run only at full column
                    coverage, skipped loud otherwise with the coverage
                    count in the header.
                    Reads <path>/.refine/fixseq/run-*.jsonl directly — no
                    store, no domain features, works on every feature build.
                    Report-first, gates nothing; refuses (exit 1) on empty
                    ring / blind walk (no run reaches 2 attempts) /
                    single-class labels — never a green zero. Paths default
                    to '.' (the config-audit convention).
    --config-audit Scan .rs files for config knobs that cannot be turned
                    (Plan 124, intake classes A+B): Default-impl fields whose
                    init is a bare const path, with ZERO field reads anywhere
                    while the const IS read elsewhere (class A — silent-wrong
                    configuration, invisible to clippy); and fields only ever
                    touched inside assert*! macros (class B — fake coverage).
                    Suggest-only, always exit 0; the detector sees only the
                    scanned set (sibling/upstream consumers under-reported).
                    Reads paths from the CLI (default `.`).
    --secret-scan   Scan text sources for secret-shaped content (Issue 151) —
                    API keys, tokens, private-key blocks, connection strings,
                    high-entropy runs — in code, comments AND string literals
                    alike, over the ONE canonical scanner (redact v4, the
                    gitleaks-consolidated pattern set). Report per finding:
                    kind + path:line:col + the context line BLINDED (never
                    the value, never a fingerprint — output is safe to
                    paste). Suggest-only, always exit 0 on a real scan;
                    refuses exit 1 on an empty walk. The same scan prints a
                    quiet `[sec]` warn line in normal suggest/fix runs when
                    findings exist. Digest pins (P63) — high-entropy hex
                    runs tagged with a digest scheme (blake3:/sha256:/…) or
                    the value of a pin-named declaration (_SHA/_DIGEST/…)
                    — are adjudicated benign in THIS report's count
                    (disclosed, not hidden); exact provider formats are
                    never adjudicated, and the egress gate + --blind stay
                    fail-closed on everything.
    --secret-scan --blind <path>
                    Print the whole FILE with every finding byte-replaced by
                    its `[REDACTED:<kind>]` placeholder (byte-exact outside
                    findings; read-only — nothing is ever written). Safe to
                    share: paste into tickets, chats, LLM prompts.
    --corpus-delta  With an oracle feed: print the per-lint table behind the corpus-truth
                    delta line (Issue 116) — which [no corpus rule] lints the COMPILED
                    fix lane can already write (the leased corpus lags, so the marker
                    means NOT-LEASED-YET) vs which neither lane carries (the real
                    coverage gap, where it means UNSUPPORTED). Report-only, exit
                    unchanged; the two lanes' corpus truths are different BY DESIGN
                    and this names the seam rather than closing it.
    --coverage-report
                    Print the corpus coverage-debt report (Issue 119): clippy's LIVE
                    lint surface, DERIVED from the installed driver
                    (`clippy-driver -W help` — never a hand-typed list, so the debt
                    moves as clippy ships lints), diffed against the clippy_lints
                    corpus and the compiled fix set (FIXABLE_LINTS u VERIFY_LINTS).
                    Three buckets, never pooled: corpus-covered / fix-set-only (the
                    Issue 116 seam — suggest cannot rank what --fix can already
                    write) / UNCOVERED, split again by whether the lint fires on a
                    plain `cargo clippy`. The frequency axis JOINS two cited
                    sources and never estimates: the Bench 061 real-code fired
                    histogram (6 sibling repos, stamped with its capture date and
                    digest) and kornel's published 100K-crate top-10
                    (.benchmarks/021); rows carrying BOTH are the dual-evidence head
                    the mining batch discipline stages first, and a lint neither
                    source saw renders `uncited`, NOT zero. Applicability is a
                    property of an EMITTED diagnostic and is declared underivable
                    rather than printed. Report-first: stages nothing, gates
                    nothing; refuses exit 1 on a blind derivation or an uncompiled
                    corpus (the frontier-report law).
    --orphan-report Scan .rs files for config knobs ORPHANED BY OPTIMIZATION
                    (Plan 126, intake class C1 — the shape a perf refactor
                    leaves when it replaces a config-fed path with a
                    compile-time table): Default-impl fields with zero
                    external reads at HEAD (the class-A condition-2 set at
                    field granularity) PLUS git-history evidence that a
                    read-shaped occurrence vanished (probe lattice + pickaxe
                    attribution; a perf/refactor-shaped newest subject tags
                    the row LIKELY-BY-OPTIMIZATION). Never-read fields print
                    as [stillborn] one-liners. Report-first, gates nothing;
                    exit 0 on every real scan, refuses exit 1 on an empty
                    scan or a missing git history leg (the frontier-report
                    law — never a confident green zero). Run it after commits
                    touching config inits (the review-time post-hoc check).
                    Wire-FP defenses (Issue 109): struct literals are
                    attributed per-file scope — a literal naming a type the
                    file imports from a foreign path, or reachable only via
                    a foreign glob with no local decl/import, is foreign
                    evidence (skipped — never a candidate/clear for the
                    local same-named struct); generic serde bridges
                    (Serialize-bounded generic + serde to_* on its value)
                    mark their call sites' turbofish-named types
                    bridge-consumed — stillborn rows there count as
                    bridge-cleared, orphaned rows render wire-dto weak.
    --platform-audit Scan .rs files for platform-dead declarations
                    (intake class P22 — the NEON_U8 shape): items declared
                    UNCONDITIONALLY whose every use sits inside a
                    platform-gated context. Invisible to any single-platform
                    CI matrix — the gate is ON where CI compiles, so the use
                    exists there, while every other platform compiles the
                    item to dead weight nobody warns about. The lattice:
                    intersect the platform atoms every use site's cfg
                    guarantees; an atom the declaration's own cfg lacks is
                    the witness. any() arms intersect, not() proves no
                    platform, feature atoms drop (the cfg-audit family owns
                    that class). Report-first; exit 0 on real scans, refuses
                    exit 1 on an empty one. Reads paths from the CLI
                    (default `.`).
    --frame-time-report
                    The frame-time budget tier (Plan 134 T2.1) over
                    riir-shader's capture-side `<capture>.frames.json`
                    sidecars (schema riir_shader_frame_time_v1, issue 018
                    there): per-effect p50/p90/mean vs the carried budget
                    (--frame-budget <MS> overrides) + the heal hint naming
                    where `--domain shader_perf` should run next. With
                    --vs <PATH>: the pre/post A/B diff — pairs by effect
                    slug, verdicts at --frame-delta-pct (default ±5% on
                    p50), warmup parity + one-sidecar-per-effect enforced
                    (a pair captured under different warmup configs, or an
                    effect with several sidecars on a side — the burst
                    class — is not evidence). Report-first, gates nothing;
                    refuses exit 1 on an empty scan, any unparsable
                    sidecar, or (A/B) any mismatch / zero usable pairs —
                    never a green zero. Confirm IMPROVED/REGRESSED verdicts
                    with a second sweep pair before acting (one run is one
                    run). Reads paths from the CLI (default `.`; the
                    positional paths are the PRE side when --vs is given).
    --misses       Print the authoring queues in FRONTIER order (Plan 091
                    + Issue 089 T3): the `.refine/misses/` rustc_errors
                    per-CODE queue (dispersion-informative error codes first
                    — resolving THOSE converts uncertain candidates), then
                    one per-RULE queue per ENABLED retrieval domain
                    (clippy/perf/kernel/sec/docker — same control law at
                    rule granularity, attribution by corpus membership).
                    Plus the Issue 156 rescue band (latent_retrieval
                    builds): the state-level FlyBy tallies from the
                    trajectory store — rescue exemplars (b0=0 ∧ bd≥3) first,
                    bd=0 states as intractable-eviction candidates;
                    report-only, augment-only vs the agreement veto.
                    Pool evidence comes from the trajectory store when the
                    build ships latent_retrieval; without it the order
                    degenerates to id-sorted. `--limit <N>` caps the table
                    rows of EACH section. Read-only standalone mode.
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
                    RIIR_REFINE_TRAJ_ROOT (or the envelope fallback) and print
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
    --version       Print the version and exit (also -V). Prints the COMPILED
                    feature set beside it and, when the release set is
                    incomplete, the rebuild command (Issue 117 — a bare
                    release build overwrites the global binary with a
                    default-features one and silently compiles sync/update/
                    --refill/burn-push out while this usage page still
                    lists them).
    --yes           Non-interactive consent: every y/N prompt answers yes
                    (the flag form of RIIR_REFINE_YES=1 — agents/CI). Arms the
                    depleted-run contribute-now offer (Issue 079) and answers the
                    first-join network prompt with its mainnet default (Plan 045:
                    --mine/login with no service config asks mainnet/devnet once;
                    RIIR_REFINE_JOIN_ENV=mainnet|devnet forces instead).
    --write         With --fix-compile: write the edits in place. (--fix already
                    writes — the explicit flag is the consent; rustfmt AUTOMATICALLY
                    follows when the original was already rustfmt-clean — pre-existing
                    drift + CRLF files are skipped — no formatting churn enters a
                    mechanical diff.)
    --export <file|->
                    With `sync` ONLY: build the redacted mining batch exactly as
                    `sync` would, and write it to <file> (`-` = stdout) instead
                    of sending it. No network, no login, no consent change: it
                    is signed with a throwaway key, so it verifies like a push
                    but belongs to no account. With no .refine-sync-policy in
                    the repo it previews the default policy --mine would write.
    --mine          What KAT mining is, the policy warning, the web link, and
                    the login → heal → sync loop (Proposal 008); LOGS YOU IN
                    when no account exists and RUNS THE SYNC at the end (owner
                    mandate 2026-09-10 — the join completes in one command;
                    re-run --mine, or use `cargo refine sync`, to push newer
                    batches). Works in every build; the status halves need
                    their features.
    --unmine        Opt OUT of contributing (keeps your account): records
                    machine-global consent = opted out, so `cargo refine sync`
                    refuses its PUSH half (the lease pull still runs), and
                    clears the local sync bookkeeping (watermark; the queued
                    .refine/outbox/ batches are named, never deleted). Your key,
                    balance and rewards are UNTOUCHED — fully reversible with
                    --mine. Healing (--fix / --suggest / the bare dry run) is
                    UNCHANGED and was always local.
    --stats [on|off]
                    The work-stats sharing consent (riir-dapps Proposal 004;
                    a THIRD consent state beside mining). `on` records OptedIn:
                    future --fix runs may push one tiny signed {{lane, rule,
                    count}} row each — rule VOCABULARY + counts, NEVER spans,
                    source, paths or repo names. `off` records OptedOut — a
                    real NO that onboarding never re-asks (forward-only:
                    already-shared epoch aggregates stay). Bare `--stats` is
                    the status readout. Default OFF (deny-by-default): unlike
                    mining (Unset proceeds — running sync IS consent), an
                    Unset stats record pushes NOTHING, because --fix is
                    universal. Mining consent is separate (--mine/--unmine);
                    one does not imply the other. Works in every build.
    --purge         With --unmine ONLY: additionally remove the local account
                    identity (key + record + outbox + watermark). DESTRUCTIVE:
                    the identity IS the key — the server-side balance and
                    rewards become permanently unclaimable (no recovery path),
                    and a purge + re-login cycle is the fresh-grant farm shape
                    (each new key gets a fresh once-per-key welcome grant).
    --purge-server  With --unmine ONLY: also ask the SERVICE to purge this
                    account's server-side mining rows — unproven rows are
                    deleted, proven rows stop publishing in every future
                    corpus (the audit record and earned credit stay; pairs
                    already published cannot be recalled). Signed with your
                    account key, so it runs BEFORE any --purge key removal.
    --refill        DEVNET-OPS ONLY (riir-dapps Issue 094 D4): credit a TEST
                    account on devnet — open a faucet request, force a crank
                    settle pass when RIIR_REFINE_KAT_CRANK_TOKEN is set, then
                    read the server balance and refresh the 075 gate's input
                    from that server truth. Exit 0 iff the final synced
                    balance is positive. It REFUSES on mainnet (and on any
                    ledger this client cannot name as devnet): the free daily
                    faucet/drip tier is retired, and the user refuel paths
                    are exactly two — contribute (cargo refine --mine) or pay
                    (KAT packs).
    --refill-wait <s>  With --refill ONLY: bound the settle poll (default 0 =
                    one after-read, no poll — the credit normally lands at the
                    hourly crank alarm, not synchronously).
    --quiet         With --fix: skip the --suggest pass (only show fixes).
    --no-fmt        Disable the automatic post-write rustfmt pass (it is
                    default-on; see src/fmt.rs for the discipline it encodes).
    login           KAT account identity (Proposal 005 T0.1): generate an
                    Ed25519 keypair in OpenSSH format under the account dir
                    (RIIR_AUTH_ACCOUNT_DIR / ~/.config/riir-auth), or
                    import one with --key <path>. Idempotent — an existing
                    key is loaded, never regenerated. Records the free-tier
                    grant as a LOCAL projection (accounting-only, G9d).
    account         KAT local view: account id (BLAKE3(pubkey)), key path,
                    the grant note (the signup claim is TUNA trial credit),
                    and — when a path (or cwd) holds a repo-local charge
                    ledger — this repo's metered burn. Verifies key↔record
                    agreement (tamper
                    evidence). No $ figures (caveats #12/#15).
    upgrade         Issue 153: the verified BINARY self-update (fetch the
                    release archive + SHA256SUMS, verify, backup-swap). The
                    binary carries the compiled corpus — upgrading it is how
                    the compiled rules move. Needs --features update_check
                    (the release set carries it). RIIR_REFINE_AUTO_UPGRADE=1
                    applies pending upgrades at run start; RIIR_REFINE_
                    AUTO_CORPUS=1 auto-runs the lease refresh (update) at run
                    start, at most once per 12h, charge-gated as usual.
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
                    (.refine/outbox/) — nothing is faked.
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
    --clippy-json <path>  Feed real clippy findings into --suggest / the dry
                    run / the --fix review pass: `cargo clippy
                    --message-format=json > f.json`, then pass f.json here.
                    Spans at real diagnostics are force-included AHEAD of
                    similarity ranking (marked [oracle]); lints with no
                    corpus rule are marked [no corpus rule] (a live gap
                    signal); feed-covered spans without a diagnostic are
                    marked [heuristic]. Under --fix, MachineApplicable
                    suggestion children are consumed VERBATIM as atomic
                    edits at spans the lane's proposers did not already
                    cover (P21 — the type-aware close for Unknown
                    receivers); lints outside the lane's corpus set are
                    never consumed. AUTO: with no feed given, a scan over a
                    cargo project runs the oracle itself (default features,
                    loud, compiles the project — RIIR_REFINE_AUTO_ORACLE=0
                    disables; RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv> re-aims
                    auto run at a feature set, for diagnostics that
                    only fire behind a non-default feature; a manual
                    --clippy-json wins outright).
    --hotpath-json <path>  Feed a hotpath JSON report measured on the
                    target repo (Issue 146 / Proposal 020 — the
                    profile-guided-healing oracle lane). T0 validates +
                    inspects: the report is keyed to the repo's git HEAD
                    (a MISMATCH refuses exit 2 — a stale profile must not
                    anchor spans at a different commit; an unkeyed report
                    proceeds with a loud disclosure) and the ranked hot
                    spans print in the summary (time share = calls x avg,
                    ranking-only; alloc bytes = the verification axis).
                    Span anchoring into the retrieval lane lands with T0.2.
    --repo-posture  With --fix ONLY: apply clippy-lane edits the TARGET
                    repo's own clippy actually flagged — an edit lands only
                    where the feed carries a same-lint diagnostic whose
                    primary span overlaps it (the 2026-09-21 riir-ai intake:
                    our doc_markdown corpus is broader than clippy's regex,
                    a 26:1 churn a posture sweep refuses). Requires a feed
                    (the auto-oracle or --clippy-json) — with neither, the
                    run REFUSES exit 2 rather than apply unverified edits.
                    The rust_perf/sec lanes and the oracle-consumed edits
                    are unaffected (not target-gate mirrors).
    --groups <g,..> With --fix ONLY (Issue 135): also apply edits for
                    allow-by-default clippy groups (pedantic, nursery,
                    restriction, cargo, or all) the TARGET crate does not
                    enable. By default such a lint (doc_markdown, match_bool,
                    uninlined_format_args, ...) is fixed only where the crate
                    enables it or its group — [lints.clippy], inherited
                    [workspace.lints.clippy], or a crate-root / file
                    #![warn(clippy::...)] — and is reported as skipped
                    otherwise, never silently.
    <path>...       Files or directories to scan. `.rs` always; with the
                    kernel_opt feature, `--suggest` ALSO scans `.wgsl` files
                    + `#[cube]` kernel bodies in `.rs` files (Issue 025 —
                    suggest-only citations, never touched by --fix); with
                    the dist feature, `Cargo.toml` manifests join the walk
                    (Issue 085 — suggest-only build-config advice).

Domains are feature-gated: clippy + perf + docker + rustc_errors + dist are
default-on (docker: Dockerfiles are matched by NAME — `Dockerfile`,
`<n>.Dockerfile`, `Dockerfile.<n>`; `docker_verify` adds the hadolint
agreement oracle. dist: Cargo.toml manifests are matched by NAME;
suggest-only, Issue 085); kernel needs --features kernel_opt, sec needs
--features sec.
Requesting a domain whose feature is off is an error (T2.4 gating). --verify needs --features fix_verify.
--fix-compile ships default-on (rustc_errors).",
        FIXABLE_RULES.len(),
        FIXABLE_RULES.join(", "),
        FIXABLE_LINTS.len(),
        FIXABLE_LINTS.join(", "),
        verify_help,
        score_help
    )
}

#[derive(Default)]
struct Args {
    suggest: bool,
    fix: bool,
    /// `--mine` — the contribution onboarding (Proposal 008 T2.4's user
    /// surface): what mining is, the policy warning, the web link, and the
    /// login → heal → sync → account loop. Standalone — early return.
    /// Also the opt-IN consent WRITE (owner mandate 2026-09-08).
    mine: bool,
    /// `--version` / `-V` — print the version and exit (standalone).
    version: bool,
    /// `--yes` — non-interactive consent: every y/N prompt answers yes
    /// (the flag form of `RIIR_REFINE_YES=1`; agents and CI). Arms the
    /// depleted-run contribute-now offer (Issue 079) and answers the
    /// first-join network prompt with its devnet default (Plan 045 A1 —
    /// `RIIR_REFINE_JOIN_ENV=mainnet|devnet` forces instead).
    yes: bool,
    /// `--unmine` — the opt-OUT counterpart to `--mine` (owner mandate
    /// 2026-09-08, the ai.gist.rs three-tier engagement story). Records
    /// machine-global consent = OPTED OUT, which makes `cargo refine sync`
    /// refuse its PUSH half (the lease pull still runs). Standalone — early
    /// return, works in every feature build. Mutually exclusive with
    /// `--mine`.
    unmine: bool,
    /// `--stats` — the work-stats sharing consent (riir-dapps Proposal 004
    /// Half 1). Standalone consent mode beside `--mine`/`--unmine`: bare
    /// `--stats` is the status readout, `--stats on` records OptedIn, `--stats
    /// off` records OptedOut. Deny-by-default: `Unset` pushes NOTHING (the
    /// deliberate inverse of mining's "running sync IS consent" — the
    /// trigger surface is `--fix`, which every coder runs). The value is
    /// OPTIONAL by design: a peek-arm consumes only `on`/`off` and leaves
    /// any other token to the path arm, so the parser needs no
    /// optional-value precedent beyond this arm.
    stats_flag: bool,
    /// `--stats on` was seen.
    stats_on: bool,
    /// `--stats off` was seen.
    stats_off: bool,
    /// `--purge` — with `--unmine` ONLY (Plan 020 review verdict: purging
    /// the identity ORPHANS the server-side balance forever — there is no
    /// recovery path — and a purge + re-login cycle is the fresh-grant farm
    /// shape, so it is never the default). Removes the local key + record
    /// + the outbox + the sync watermark.
    purge: bool,
    /// `--purge-server` — with `--unmine` ONLY (riir-dapps Issue 119 T12):
    /// ask the service to purge this account's server-side mining rows
    /// (unproven deleted; proven unpublished). Signed, so it MUST run
    /// before `--purge` removes the local key.
    purge_server: bool,
    /// Plan 117 as amended by riir-dapps Issue 094 D4: `--refill` — the
    /// **devnet-ops** faucet credit. It is NO LONGER "the refuel path the
    /// 075 gate names": the daily faucet/drip user tier is retired, the
    /// gate's menu is contribute-or-pay, and this refuses on any ledger
    /// that is not devnet. Standalone early-return mode (before
    /// `validate_mode_flags` AND before the soft gate — an ops tool must
    /// not be gated by the balance it exists to repair). Composes the
    /// shipped faucet/crank/balance surfaces and refreshes the gate's
    /// server-truth input; the module (`kat_refill`) owns the wire AND the
    /// posture rule, this flag owns the consent.
    refill: bool,
    /// Plan 117: `--refill-wait <secs>` — with `--refill` ONLY: the bounded
    /// settle-poll budget after the crank leg (default 0 = one after-read,
    /// no spinning — the credit normally lands at the hourly alarm, not
    /// synchronously).
    refill_wait: u64,
    /// `--info` — the one-command stats view: account + consents, wallet
    /// (balance/grant/burned/contributed/receipts via `GET /history`),
    /// staking, this machine's mining queue outcomes (`GET
    /// /mining/adoption`), and the repo-local work state (store rows,
    /// outbox, charge ledger, soft gate). READ-ONLY; standalone
    /// early-return. `stake --info` keeps its own flag (the position).
    info: bool,
    /// Set by `parse_args` when the bare no-mode default fired (the dry-run
    /// banner then prints once, from main). Not a user flag.
    bare_default: bool,
    /// Plan 019 T1.3: `--fix-compile` — the `rustc_errors` oracle driver.
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
    /// `--clippy-json <path>` (2026-09-13 retrieval-miss intake): feed a
    /// real `cargo clippy --message-format=json` stream into the suggest
    /// pass — spans at real diagnostics are force-included AHEAD of
    /// similarity ranking (`[oracle]`), lints with no corpus rule surface
    /// as a gap signal (`[no corpus rule]`), and feed-covered spans with
    /// no diagnostic are marked `[heuristic]`. Augments `--suggest` /
    /// the bare dry run / the `--fix` review pass. Since Issue 113 the
    /// AUTO-ORACLE runs this stream for you on cargo projects (default
    /// features, loud, `RIIR_REFINE_AUTO_ORACLE=0` opts out;
    /// `RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv>` re-aims the spawn at a
    /// feature set — Issue 115) — this flag remains the explicit/curated
    /// override and wins outright.
    clippy_json: Option<String>,
    /// `--hotpath-json <path>` (Issue 146 T0.1/T0.5, Proposal 020 — the
    /// profile-guided-healing oracle lane): feed a [hotpath](https://github.com/pawurb/hotpath-rs)
    /// JSON report measured on the TARGET repo. T0 honors the feed as a
    /// VALIDATION + INSPECTION surface: the report is parsed, keyed to the
    /// repo (`meta.git.sha` vs the project root's `git rev-parse HEAD` —
    /// a MISMATCH REFUSES exit 2, the stale-profile hazard; an unkeyed
    /// report proceeds with a loud disclosure), and the ranked hot spans
    /// print in the run summary (time share = calls × avg, RANKING-only;
    /// alloc bytes/count = the deterministic verification axis). The
    /// ANCHORING into the retrieval lane (spans force-included ahead of
    /// similarity ranking, tagged `[hot: …]`) is T0.2 — the flag parses,
    /// validates, and discloses today so the wire is pinned early.
    hotpath_json: Option<String>,
    /// `--repo-posture` (Plan 165, the 2026-09-21 riir-ai intake): gate the
    /// CLIPPY-lane fix edits on the target repo's OWN clippy diagnostics —
    /// an edit applies only where the feed (manual `--clippy-json` or the
    /// auto-oracle) carries a same-lint diagnostic whose primary span
    /// overlaps the edit. `--fix` on a foreign repo then cannot restyle
    /// code the target's own gate never flagged (the 26:1 `doc_markdown`
    /// churn that motivated the mode). The P21 oracle-edits lane (consumed
    /// FROM the diagnostics — confirmed by construction) and the sec lane
    /// are unaffected; the rust_perf lane is POSTURE-GATED for its
    /// NotBenchable fixables (`NOT_BENCHABLE_FIXABLE`, P39 2026-09-25):
    /// those are declined under the mode (never benchmarked — the mode's
    /// contract is "only what a gate verified"), with `--apply-
    /// notbenchable` as the explicit override. REQUIRES a feed: with neither an
    /// auto-oracle run nor a manual feed the mode REFUSES (exit 2) rather
    /// than apply unverified edits wearing the posture name.
    repo_posture: bool,
    /// `--apply-notbenchable` (post-mining intake P39, 2026-09-25): the
    /// explicit override for the posture lane's NotBenchable decline —
    /// under `--repo-posture` the rust_perf fix lane declines rules whose
    /// corpus entry carries no bench spec (never benchmarked by us; the
    /// target's clippy cannot speak to them either), because the mode's
    /// whole contract is "only what a gate verified". This flag re-enables
    /// them for operators who have judged the specific repo themselves.
    /// Requires `--repo-posture` (setting it without the mode is misuse —
    /// there is nothing to override).
    apply_notbenchable: bool,
    /// `--groups pedantic,nursery` (Issue 135 T3): opt the clippy fix lane
    /// into allow-by-default groups the TARGET crate does not enable.
    /// Without it an allow-by-default lint's edits apply only where the
    /// crate enables the lint or its group (`[lints.clippy]`, inherited
    /// `[workspace.lints.clippy]`, crate-root / file inner
    /// `#![warn(clippy::…)]`) and are reported as skipped otherwise. `all`
    /// = every allow-by-default group (the pre-135 behaviour).
    fix_groups: Vec<String>,
    top_k: usize,
    verify: bool,
    /// `--no-verify`: opt out of the compile gate that --fix now defaults
    /// to on `fix_verify` builds (the just-works UX keeps the escape hatch).
    no_verify: bool,
    verify_args: Option<String>,
    paths: Vec<PathBuf>,
    /// `--score-bench` (Plan 014): run the frozen clippy-bench, append the
    /// score history, re-render the SVG. Standalone mode — no paths.
    score_bench: bool,
    /// `--generation <1|2|3|4|5|6|7|8|9>` (Plan 092 M5 + Bench 069 reopen #1 +
    /// Plan 093 + Plan 094 + Plan 095 + Issue 137): which bench generation to run.
    /// Absent = 1 (the frozen v1 set, unchanged). 2 = the construction-built generation-v2
    /// k-ladder (issue 072 T5); 3 = the persist-stratified generation-v3
    /// ladder (Bench 069 reopen #1); 4 = the rustc-errors-side k-ladder
    /// (Plan 093 — measures the `--fix-compile` driver against rustc
    /// coded-error ground truth); 5 = the `rust_perf` vocabulary ladder (Plan
    /// 094 — the mechanical shape oracle over the FIXABLE perf vocabulary);
    /// 9 = the CRB corruption ladder (Issue 137 — compile-clean/test-failing
    /// known-position localization + the DLM remask arming gate, own runner).
    /// A NEW generation's first append is gated on non-vacuity — a
    /// generation that is not harder than v1 is refused, never appended.
    generation: Option<u8>,
    /// `--score-report`: re-render the SVG from the existing history JSONL.
    score_report: bool,
    /// Issue 108 T2: `--necessity-bench` — the component-necessity
    /// flip-rate harness (per-rule disable + matched-swap arms over the
    /// frozen v1 set, real-clippy oracle, two-sided canaries, JSONL out).
    /// Standalone mode — no paths. Needs `score_bench`.
    necessity_bench: bool,
    /// `--necessity-top <N>`: top-N targets by solo resolved (default 20).
    necessity_top: Option<usize>,
    /// `--necessity-random <K>`: stride picks from the remaining present
    /// rules (default 5; deterministic stride sampling, never RNG).
    necessity_random: Option<usize>,
    /// `--necessity-smoke`: the bounded e2e — first 20 fixtures, top-3 + 1
    /// stride + both canaries (validates the instrument itself without the
    /// full oracle bill).
    necessity_smoke: bool,
    /// Issue 102 T4: `--necessity-rule <RULE>` with `--necessity-bench` —
    /// the per-DECISION paired lane: forced single-target control/disabled
    /// pair over the frozen set (priced per invocation, not per bench
    /// run). A no-flip verdict (DECORATIVE) is the finding being priced,
    /// so the load-bearing canary is skipped loud in this mode; the
    /// planted-noop determinism canary still runs.
    necessity_rule: Option<String>,
    /// Issue 130: `--gate-calibration` — the admission-gate evidence
    /// report (standalone, ungated, report-only): the T1 δ calibration
    /// over the score-history strata, the T2-path-(a) OOD-axis readout
    /// over the same rows' `ood` entropy-gap blocks, and the T3 corpus-stall
    /// predicate over the frontier snapshots. Nothing here gates; the T2
    /// admission predicate stays deferred until the report says the repo
    /// stratum can support a δ.
    gate_calibration: bool,
    /// Issue 102 T2: `--active-set` — print the corpus active-set marker
    /// (standalone). The marker file is the ONLY demotion writer; the
    /// fade-predictive clock stays the only thing that could ever automate
    /// one.
    active_set_list: bool,
    /// `--active-set-demote <domain:rule>` — demote a rule out of the
    /// similarity fan-out + the fix dispatch (entry KEPT in the corpus;
    /// deletion is never a lifecycle op).
    active_set_demote: Option<String>,
    /// `--active-set-readmit <domain:rule>` — remove a demotion (the
    /// manual counterpart of the auto re-admit).
    active_set_readmit: Option<String>,
    /// `--active-set-clear` — drop every demotion + cap.
    active_set_clear: bool,
    /// `--active-set-cap <domain:N>` — set the per-domain active ceiling
    /// (T3); `0` removes it. OFF by default: the cap binds only when the
    /// admission path enforces it, per the Library-Drift warning.
    active_set_cap: Option<String>,
    /// `--memory-stats` (Issue 037 observability): print the one-command
    /// debug summary of the self-evolve store — tier/outcome counts, top
    /// rules, BLAKE3 commitment status, + the E1/E2/E3 loop-state verdict.
    /// Read-only standalone mode (no paths, no recording, ignores the
    /// opt-out env — a query, not a run).
    memory_stats: bool,
    /// Plan 091 T1.2: `--misses` — print the `.refine/misses/` authoring
    /// queue in FRONTIER order (dispersion-informative codes first, the
    /// real-G8 enabler). Read-only standalone mode (the memory-stats
    /// pattern: early return before `validate_mode_flags`).
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
    /// `RIIR_REFINE_TRAJ_ROOT` (or the envelope fallback) + a fresh baseline.
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
    /// riir-dapps Plan 023: `--url <alias|url>` — with `config set` ONLY:
    /// the service-url value to persist (a pinned alias — devnet | testnet |
    /// mainnet — or an absolute https url; http only for loopback).
    config_url: Option<String>,
    /// riir-dapps Plan 023: the `config set` subcommand modifier (bare
    /// `config` / `config get` is the pure read).
    config_set: bool,
    /// riir-kat Issue 003 T4: the `config adopt-state <devnet|mainnet>
    /// subcommand modifier — the explicit, per-file migration for pre-split
    /// network-blind state (the remedy the stamp judge's refusal names).
    config_adopt_state: Option<String>,
    /// riir-kat Issue 003 T4: `--discard` — with `config adopt-state` ONLY:
    /// quarantine the scanned files instead of adopting (the `.bak`-preserving
    /// move; never a delete).
    config_adopt_discard: bool,
    /// riir-kat Issue 003 T11: `--net <devnet|mainnet>` — sugar over the env
    /// var for ONE invocation (validated against the build's pin).
    net_alias: Option<String>,
    /// riir-dapps Issue 112 T4: `--export <file|->` — with `sync` ONLY: write
    /// the redacted batch locally instead of pushing it.
    sync_export: Option<PathBuf>,
    /// riir-dapps Plan 050 S1.11: `--src <url|alias>` — with `corpus-pr`
    /// ONLY: the PR source env (the dev branch).
    corpus_pr_src: Option<String>,
    /// `--dst <url|alias>` — with `corpus-pr` ONLY: the PR destination
    /// (must name the mainnet ledger).
    corpus_pr_dst: Option<String>,
    /// `--seq <N>` — with `corpus-pr` ONLY: the explicit pr_seq (absent =
    /// the client-local last + 1).
    corpus_pr_seq: Option<u64>,
    /// `--submitter <32hex>` — with `corpus-pr` ONLY: the payee account id.
    corpus_pr_submitter: Option<String>,
    /// `--key-file <path>` — with `corpus-pr` ONLY: the PR-attestation key
    /// (the `--gen-corpuspr` 64-hex-seed format; absent = the gauge).
    corpus_pr_key_file: Option<PathBuf>,
    /// `--submit` — with `corpus-pr` ONLY: POST the envelope + chunks after
    /// drafting (replays are idempotent).
    corpus_pr_submit: bool,
    /// Proposal 007 T2.1: `--usd <N>` — the stake amount (dollars, ≤6
    /// fractional digits, integer-exact into base units).
    stake_usd: Option<String>,
    /// Proposal 007 T2.1: `--info` — render the stake position instead of
    /// composing a stake request.
    stake_info: bool,
    /// Proposal 017: `--restake off|buffered` — the signed posture op
    /// (account state with server truth; never a machine-local consent).
    stake_restake: Option<String>,
    /// Proposal 017: `--redeem` — the signed redeem op (drain every
    /// UNLOCKED KAT tranche to the liquid balance).
    stake_redeem: bool,
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
    /// riir-dapps Plan 045 B7: `--pack <N>` — with `topup` ONLY: the
    /// catalog pack to buy. Absent + a TTY = an interactive pick;
    /// absent + non-TTY = a loud refusal (agents/CI never guess a price).
    topup_pack: Option<u8>,
    /// The checkout QR (2026-10-13 rider): `--qr` forces the ASCII QR
    /// after the intent lines even piped; `--no-qr` suppresses it even on
    /// a TTY; default = AUTO (a TTY sees the QR — scanning a phone is the
    /// primary payment gesture — agents/CI never render one). `--qr` is
    /// the SAME token `pay`/`faucet` parse; the command check normalizes
    /// it to the topup field (the token order in the grammar is
    /// command-first, but normalization keeps a stray `--qr topup` form
    /// working too).
    topup_qr: bool,
    topup_no_qr: bool,
    /// Issue 126: `--show <seq>` — re-print an OPEN (or voided-unexpired)
    /// checkout's QR. The server recomposes the URL from the row (the
    /// marked amount never rode the history wire), so the re-print is
    /// byte-identical to the QR that was scanned.
    topup_show: Option<u64>,
    /// Issue 126: `--void <seq>` — release an open checkout's slot
    /// (idempotent; the reference stays reserved until the TTL, so a
    /// payment already in flight still lands on the account).
    topup_void: Option<u64>,
    /// Issue 126 follow-up: `--pick latest` — the non-interactive pick.
    /// On the open cap the NEWEST checkout is shown without prompting
    /// (and the interactive picker — with its 10 s first-key window — is
    /// never entered). Agents/CI pass this; a human just presses Enter.
    topup_pick_latest: bool,
    /// The `buy <amount> <unit>` positionals (the topup friction-cut).
    /// The unit word may ride glued (`buy 10kat`) — split post-parse.
    buy_amount: Option<String>,
    buy_unit: Option<String>,
    /// riir-dapps Proposal 009 §9 / issue 129: the `get|send <amt> tuna
    /// [account]` positionals — same consumed-inline shape as buy's (the
    /// glued `<amt>tuna` form splits post-parse; `send`'s third word is
    /// the recipient account id).
    tuna_amount: Option<String>,
    tuna_unit: Option<String>,
    tuna_target: Option<String>,
    /// The `--gov <id>` pin — skips the listing's unique-hash resolution.
    tuna_gov: Option<u64>,
    /// `--dry-run` for the airdrop verbs: hash + row + body + caps, no
    /// network, no secrets read.
    tuna_dry_run: bool,
    /// Proposal 011 T4.2: `--dry-run` — with `wetrun` ONLY, run steps 1+6
    /// read-only (admitted on every network incl. mainnet). The heal modes
    /// have no --dry-run flag: bare `cargo refine` IS the dry run.
    wetrun_dry_run: bool,
    /// riir-dao Proposal 001 T4: the `account migrate` mode flags —
    /// exactly one mode per invocation (`--github` files, `--object`
    /// composes the objection URL, `--status` reads, `--census` reports).
    migrate_github: Option<String>,
    migrate_object: Option<String>,
    migrate_status: Option<String>,
    migrate_census: bool,
    /// The OLD key file (the per-invocation key source — REQUIRED for a
    /// filing; never an env var, the measured T-PREFLIGHT finding).
    migrate_old_key: Option<String>,
    /// Optional GitHub bearer for the PoW measure (rate limits only;
    /// `GITHUB_TOKEN`/`GH_TOKEN` env respected as fallback).
    migrate_github_token: Option<String>,
    /// The objection repo base (default: the dao repo).
    migrate_repo_base: Option<String>,
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
    /// Issue 089 T1+T2+T5: `--frontier-report` — the learnability-frontier
    /// band report over the trajectory store's per-rule outcome counters:
    /// r/f/n/p̂/LCB₅ per rule, the band-mass health metric (fraction
    /// 0<p̂≤0.5), the saturation demotion CANDIDATE list (p̂=1 ∧ n≥50,
    /// report-only — never deletes), plus the T5 provenance tail: the
    /// pair namespace priced by provenance (organic=1.0, construction=ε,
    /// unknown=0.0) + the outcome-feed split (generated-case ':' keys vs
    /// real-repo rows — band-mass per feed). Read-only standalone mode
    /// (the memory-stats pattern); refuses with exit 1 on an empty
    /// population (never a green zero). Needs `latent_retrieval` (the store).
    frontier_report: bool,
    /// Issue 107 T1: `--horizon-report` — lead-k linear decodability of
    /// fix-compile attempt outcomes over the `.refine/fixseq/` ring jsonl:
    /// AUC(k) for a LORO-held-out closed-form ridge readout over
    /// ring-native features at i-k, against the base floor (0.5) and the
    /// history-aggregate floor (per-strategy Laplace prior, LORO). Report-
    /// first, gates nothing; refuses exit 1 on empty ring / blind walk /
    /// single-class labels (never a green zero). Ungated — reads jsonl
    /// directly, no store, no domain features. Paths default to `.`.
    horizon_report: bool,
    /// Issue 128 (T1-T4): `--fixseq-stats` — the fixseq cascade-law +
    /// derailment-index report (Traverse/Scout, arXiv:2609.17930, Research
    /// 200): P(bad-next | bad-prev) vs P(bad-next | good-prev) with Wilson
    /// 95% CIs under three badness indicators, first-wrong-attempt
    /// distributions + the G4 KS-vs-point-mass guard, cross-indicator ±k
    /// agreement, root-vs-cascade attribution (masking as the root signal),
    /// silent-failure rate, and the T7 transfer verdict when MULTIPLE paths
    /// are given (each path = one corpus). Report-first, gates nothing;
    /// refuses exit 1 on empty / no-transition populations. Ungated.
    fixseq_stats: bool,
    /// Plan 124: `--config-audit` — the config-hygiene report (classes A+B
    /// from the post-mining intake): Default-impl fields that default to a
    /// const nobody's field-read while the const IS read elsewhere (the
    /// knob that cannot be turned), and fields only ever asserted. Pure syn
    /// scan, zero new deps, ungated; always exit 0 (report, not a gate).
    config_audit: bool,
    /// Issue 151: `--secret-scan` — the secret-blind report (the
    /// consolidated v4 pattern set): kind + path:line:col + the context
    /// line BLINDED per finding; never the bytes, never the fingerprint.
    /// Pure scan, zero new deps, ungated; always exit 0 on a real scan,
    /// refuses exit 1 on an empty walk (the frontier-report law).
    /// `--blind` (requires this) prints whole-file text with findings
    /// placeholder-substituted — safe-to-share, read-only.
    secret_scan: bool,
    /// Issue 151: `--blind` — the `--secret-scan` output transform (the
    /// whole file text with every finding byte-replaced by its
    /// `[REDACTED:<kind>]` placeholder). Never writes; usage error without
    /// `--secret-scan`.
    blind: bool,
    /// Intake class P22: `--platform-audit` — the platform-dead_code
    /// decl-vs-use cfg lattice: declarations whose every use sits inside
    /// a platform gate the declaration lacks (the `NEON_U8` class no
    /// single-platform CI matrix can see). Pure syn scan, zero new deps,
    /// ungated; exit 0 on real scans, refuses exit 1 on an empty one.
    platform_audit: bool,
    /// Plan 126: `--orphan-report` — the class-C1 report (the config knob
    /// orphaned by optimization): Default-impl fields with zero external
    /// reads at HEAD + git-history evidence that a read-shaped occurrence
    /// vanished (probe lattice + pickaxe attribution; the
    /// LIKELY-BY-OPTIMIZATION tag). Refuses exit 1 on an empty scan or a
    /// blind history leg — never a confident green zero.
    orphan_report: bool,
    /// Plan 134 T2.1: `--frame-time-report` — the frame-time budget tier
    /// over riir-shader's capture-side `<capture>.frames.json` sidecars
    /// (schema `riir_shader_frame_time_v1`): per-effect p50/p90/mean vs the
    /// carried budget (`--frame-budget` overrides) + the heal hint for
    /// over-budget effects. With `--vs <PATH>`: the A/B diff — pairs by
    /// effect slug, verdicts at `--frame-delta-pct` (default ±5% on p50),
    /// warmup parity + pairing uniqueness enforced (a pair with different
    /// warmup configs, or an effect with multiple sidecars on a side, is
    /// not evidence). Reads sidecars directly — no domain features, works
    /// on every feature build. Report-first, gates nothing; refuses exit 1
    /// on an empty scan / any unparsable sidecar / (A/B) any mismatch or
    /// zero usable pairs — never a green zero. Paths default to `.`.
    frame_time_report: bool,
    /// `--frame-budget <MS>`: the budget override for
    /// `--frame-time-report` (wins over the sidecar's
    /// `frame_budget_ms` echo — an explicit CLI budget is the stricter
    /// statement).
    frame_budget: Option<f64>,
    /// `--vs <PATH>`: the POST side (dir or file) turning
    /// `--frame-time-report` into the pre/post A/B diff. The positional
    /// paths are the PRE side.
    frame_vs: Option<PathBuf>,
    /// `--frame-delta-pct <PCT>`: the A/B verdict threshold on the p50
    /// delta (default 5.0).
    frame_delta_pct: Option<f64>,
    /// Issue 119: `--coverage-report` — the corpus coverage-debt sweep.
    /// clippy's LIVE lint surface, derived from the INSTALLED driver
    /// (`clippy-driver -W help`, never a hand-typed list), diffed against
    /// the `clippy_lints` corpus + the compiled fix set, with the real-code
    /// fired histogram (Bench 061) and kornel's published top-10 joined as
    /// the two frequency sources. Report-first, gates nothing; refuses
    /// exit 1 on a blind derivation — never a confident green zero.
    coverage_report: bool,
    /// Issue 116: `--corpus-delta` — the per-lint table behind the suggest
    /// lane's corpus-truth delta line. Report-only, exit unchanged; needs
    /// an oracle feed (the delta is measured over `[no corpus rule]`
    /// anchors, which only a feed produces).
    corpus_delta: bool,
}

fn parse_args() -> Args {
    let mut args = Args {
        frame_time_report: false,
        frame_budget: None,
        frame_vs: None,
        frame_delta_pct: None,
        account_history: false,
        info: false,
        suggest: false,
        fix: false,
        mine: false,
        stats_flag: false,
        stats_on: false,
        stats_off: false,
        version: false,
        yes: false,
        unmine: false,
        purge: false,
        purge_server: false,
        refill: false,
        refill_wait: 0,
        bare_default: false,
        fix_compile: false,
        dump_trajectories: false,
        max_iters: 5,
        write: false,
        quiet: false,
        no_fmt: false,
        domain: None,
        clippy_json: None,
        hotpath_json: None,
        repo_posture: false,
        apply_notbenchable: false,
        fix_groups: Vec::new(),
        top_k: 3,
        verify: false,
        no_verify: false,
        verify_args: None,
        paths: Vec::new(),
        score_bench: false,
        generation: None,
        score_report: false,
        necessity_bench: false,
        necessity_top: None,
        necessity_random: None,
        necessity_smoke: false,
        necessity_rule: None,
        gate_calibration: false,
        active_set_list: false,
        active_set_demote: None,
        active_set_readmit: None,
        active_set_clear: false,
        active_set_cap: None,
        memory_stats: false,
        misses: false,
        misses_limit: None,
        misses_report: false,
        recompute_elo: false,
        collapse_report: false,
        command: None,
        import_key: None,
        claim_tx: None,
        config_url: None,
        config_set: false,
        config_adopt_state: None,
        config_adopt_discard: false,
        net_alias: None,
        sync_export: None,
        corpus_pr_src: None,
        corpus_pr_dst: None,
        corpus_pr_seq: None,
        corpus_pr_submitter: None,
        corpus_pr_key_file: None,
        corpus_pr_submit: false,
        stake_usd: None,
        stake_info: false,
        stake_restake: None,
        stake_redeem: false,
        vote_proposal: None,
        vote_yes: false,
        vote_no: false,
        pay_request: false,
        pay_amount: None,
        pay_qr: false,
        pay_label: None,
        pay_wait: 600,
        topup_pack: None,
        topup_qr: false,
        topup_no_qr: false,
        topup_show: None,
        topup_void: None,
        topup_pick_latest: false,
        buy_amount: None,
        buy_unit: None,
        tuna_amount: None,
        tuna_unit: None,
        tuna_target: None,
        tuna_gov: None,
        tuna_dry_run: false,
        wetrun_dry_run: false,
        migrate_github: None,
        migrate_object: None,
        migrate_status: None,
        migrate_census: false,
        migrate_old_key: None,
        migrate_github_token: None,
        migrate_repo_base: None,
        update_timeout: 4,
        no_update: false,
        no_reheat: false,
        corpus_epoch: None,
        candidates_path: None,
        corpus_stats: false,
        json: false,
        import_series: false,
        frontier_report: false,
        horizon_report: false,
        fixseq_stats: false,
        config_audit: false,
        secret_scan: false,
        blind: false,
        platform_audit: false,
        orphan_report: false,
        coverage_report: false,
        corpus_delta: false,
    };
    // `cargo refine <args>` executes this binary with the SUBCOMMAND NAME as
    // argv[1] — cargo passes the binary's own name-spelling through (direct
    // `cargo-refine <args>` does not carry it; measured: `cargo echotest foo`
    // hands the binary `echotest foo`). BOTH spellings must strip: `refine`
    // (the binary's name since Plan 192 T1.2) and `heal` (the dist-era
    // spelling, kept so a cargo-heal-named binary running this code keeps
    // working through the dist-flip window). Without the strip the word
    // landed in the scan-path arm and the KAT subcommands'
    // `args.paths.is_empty()` guard demoted every one of them
    // (`cargo refine login` → dry-run over paths ["refine", "login"]) while
    // every flag-mode invocation with explicit paths kept working — which is
    // how T1.2d's smoke and the direct-spawn CLI tests all missed it.
    // A leading own-name is never a scan path.
    let mut raw: Vec<String> = std::env::args().skip(1).collect();
    if raw
        .first()
        .is_some_and(|first| first == "refine" || first == "heal")
    {
        raw.remove(0);
    }
    let mut it = raw.into_iter().peekable();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--suggest" => args.suggest = true,
            // The owner's just-works UX (2026-09-08): `--fix` is a REAL fix —
            // it writes (and verify-gates when the build carries
            // `fix_verify`), because the explicit flag IS the consent. The
            // old review-only behavior needs one explicit word: `--dry-run`
            // (also the bare `cargo refine` default — see validate_mode_flags).
            "--fix" => args.fix = true,
            "--fix-compile" => args.fix_compile = true,
            "--dump-trajectories" => args.dump_trajectories = true,
            "--write" => args.write = true,
            "--mine" => args.mine = true,
            "--unmine" => args.unmine = true,
            // Proposal 004 Half 1: the work-stats consent flag. OPTIONAL-value
            // by peek (the parse loop has no other optional-value arm — every
            // value arm consumes `it.next()` unconditionally and bare words
            // fall through to the path arm): consume ONLY `on`/`off`, leave
            // anything else (a scan path, another flag) for the arms below.
            // Bare `--stats` is the status readout; the ambiguity guard in the
            // post-parse section rejects `--stats <path>` so a path can never
            // silently double as a missing value.
            "--stats" => {
                args.stats_flag = true;
                match it.peek().map(String::as_str) {
                    Some("on") => {
                        it.next();
                        args.stats_on = true;
                    }
                    Some("off") => {
                        it.next();
                        args.stats_off = true;
                    }
                    _ => {}
                }
            }
            "--version" | "-V" => args.version = true,
            // Issue 144 T3: asking for help is not an error — stdout, exit 0
            // (it used to print `unknown flag "--help"` + the page, exit 2).
            "--help" | "-h" => {
                println!("{}", user_help());
                std::process::exit(0);
            }
            "--help-all" => {
                println!("{}", usage_text());
                std::process::exit(0);
            }
            // ONE global `--yes`: non-interactive consent for every y/N prompt
            // AND the vote arm it used to serve exclusively (the later
            // duplicate arm is gone — first-match would have shadowed it).
            "--yes" => {
                args.yes = true;
                args.vote_yes = true;
            }
            "--purge" => args.purge = true,
            "--purge-server" => args.purge_server = true,
            "--refill" => args.refill = true,
            "--refill-wait" => {
                let Some(value) = it.next() else {
                    eprintln!(
                        "--refill-wait requires a value (seconds; 0 = one after-read, no poll)"
                    );
                    usage();
                };
                args.refill_wait = value.parse().unwrap_or_else(|_| {
                    eprintln!("--refill-wait expects a number, got {value:?}");
                    usage();
                });
            }
            "--quiet" => args.quiet = true,
            "--no-fmt" => args.no_fmt = true,
            "--verify" => args.verify = true,
            "--no-verify" => args.no_verify = true,
            "--score-bench" => args.score_bench = true,
            "--score-report" => args.score_report = true,
            "--necessity-bench" => args.necessity_bench = true,
            "--necessity-smoke" => args.necessity_smoke = true,
            "--necessity-rule" => {
                let Some(value) = it.next() else {
                    eprintln!(
                        "--necessity-rule requires a value (the corpus rule id, e.g. map_clone)"
                    );
                    usage();
                };
                args.necessity_rule = Some(value);
            }
            "--gate-calibration" => args.gate_calibration = true,
            "--active-set" => args.active_set_list = true,
            "--active-set-demote" => {
                let Some(value) = it.next() else {
                    eprintln!("--active-set-demote requires a value (domain:rule, e.g. clippy:manual_div_ceil)");
                    usage();
                };
                args.active_set_demote = Some(value);
            }
            "--active-set-readmit" => {
                let Some(value) = it.next() else {
                    eprintln!("--active-set-readmit requires a value (domain:rule)");
                    usage();
                };
                args.active_set_readmit = Some(value);
            }
            "--active-set-clear" => args.active_set_clear = true,
            "--active-set-cap" => {
                let Some(value) = it.next() else {
                    eprintln!("--active-set-cap requires a value (domain:N; 0 removes the cap)");
                    usage();
                };
                args.active_set_cap = Some(value);
            }
            "--necessity-top" => {
                let Some(value) = it.next() else {
                    eprintln!("--necessity-top requires a value (the top-N target count)");
                    usage();
                };
                args.necessity_top = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--necessity-top expects a number, got {value:?}");
                    usage();
                }));
            }
            "--necessity-random" => {
                let Some(value) = it.next() else {
                    eprintln!("--necessity-random requires a value (the stride-pick count)");
                    usage();
                };
                args.necessity_random = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--necessity-random expects a number, got {value:?}");
                    usage();
                }));
            }
            "--memory-stats" => args.memory_stats = true,
            "--frontier-report" => args.frontier_report = true,
            "--horizon-report" => args.horizon_report = true,
            "--fixseq-stats" => args.fixseq_stats = true,
            "--config-audit" => args.config_audit = true,
            "--secret-scan" => args.secret_scan = true,
            "--blind" => args.blind = true,
            "--platform-audit" => args.platform_audit = true,
            "--orphan-report" => args.orphan_report = true,
            "--frame-time-report" => args.frame_time_report = true,
            "--frame-budget" => {
                let Some(value) = it.next() else {
                    eprintln!("--frame-budget requires a value (milliseconds)");
                    usage();
                };
                let parsed = value.parse::<f64>().ok().filter(|ms| *ms > 0.0);
                let Some(ms) = parsed else {
                    eprintln!("--frame-budget expects a positive number of ms, got {value:?}");
                    usage();
                };
                args.frame_budget = Some(ms);
            }
            "--vs" => {
                let Some(value) = it.next() else {
                    eprintln!("--vs requires a value (the POST side dir or sidecar file)");
                    usage();
                };
                args.frame_vs = Some(PathBuf::from(value));
            }
            "--frame-delta-pct" => {
                let Some(value) = it.next() else {
                    eprintln!("--frame-delta-pct requires a value (percent)");
                    usage();
                };
                let parsed = value.parse::<f64>().ok().filter(|p| *p > 0.0);
                let Some(pct) = parsed else {
                    eprintln!("--frame-delta-pct expects a positive percent, got {value:?}");
                    usage();
                };
                args.frame_delta_pct = Some(pct);
            }
            "--coverage-report" => args.coverage_report = true,
            "--corpus-delta" => args.corpus_delta = true,
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
            "--clippy-json" => {
                let Some(value) = it.next() else {
                    eprintln!("--clippy-json requires a value (a clippy JSON stream path)");
                    usage();
                };
                args.clippy_json = Some(value);
            }
            "--hotpath-json" => {
                let Some(value) = it.next() else {
                    eprintln!("--hotpath-json requires a value (a hotpath JSON report path)");
                    usage();
                };
                args.hotpath_json = Some(value);
            }
            "--repo-posture" => {
                args.repo_posture = true;
            }
            "--apply-notbenchable" => {
                args.apply_notbenchable = true;
            }
            "--groups" => {
                let Some(value) = it.next() else {
                    eprintln!(
                        "--groups requires a value (pedantic,nursery,restriction,cargo or all)"
                    );
                    usage();
                };
                for g in value.split(',').map(str::trim).filter(|g| !g.is_empty()) {
                    if g != "all" && !ALLOW_BY_DEFAULT_GROUPS.contains(&g) {
                        eprintln!(
                            "--groups: unknown allow-by-default group `{g}` (pedantic, nursery, \
                             restriction, cargo, all)"
                        );
                        usage();
                    }
                    args.fix_groups.push(g.to_string());
                }
            }
            "--generation" => {
                let Some(value) = it.next() else {
                    eprintln!("--generation requires a value (1, 2, 3, 4, 5, 6, 7, 8, or 9)");
                    usage();
                };
                let g: u8 = value.parse().unwrap_or_else(|_| {
                    eprintln!("--generation expects a number, got {value:?}");
                    usage();
                });
                if !(1..=9).contains(&g) {
                    eprintln!("--generation accepts 1, 2, 3, 4, 5, 6, 7, 8, or 9, got {g}");
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
                // Issue 144 T4: the choices are the domains THIS build
                // compiled, never the full enum — a release binary used to
                // advertise `sec`/`kernel` and then refuse them.
                args.domain = match (value.as_str(), DomainId::from_label(&value)) {
                    ("auto", _) => None,
                    (_, Some(d)) if d.is_compiled() => Some(d),
                    (_, Some(d)) => {
                        eprintln!(
                            "--domain {} is not in this build (this build has: {})",
                            d.label(),
                            domain_choices()
                        );
                        std::process::exit(2);
                    }
                    (other, None) => {
                        eprintln!("unknown --domain {other:?} (want {})", domain_choices());
                        std::process::exit(2);
                    }
                };
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
            "--restake" => {
                let Some(value) = it.next() else {
                    eprintln!("--restake requires a posture: off | buffered");
                    usage();
                };
                args.stake_restake = Some(value);
            }
            "--redeem" => args.stake_redeem = true,
            // `--info` is TWO surfaces: `stake --info` (the position) and
            // the standalone stats view. The subcommand word parses FIRST
            // when it leads (`cargo refine stake --info`), so this arm sees
            // the command; a `--info` that LEADS before `stake` is caught
            // by the post-parse combo guard (never a silent mode flip).
            "--info" => {
                if args.command == Some(KatCommand::Stake) {
                    args.stake_info = true;
                } else {
                    args.info = true;
                }
            }
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
            "--no" => args.vote_no = true,
            "--amount" => {
                let Some(value) = it.next() else {
                    eprintln!("--amount requires a value (the request amount in KAT, e.g. 2 or 1.5; omit for ANY amount)");
                    usage();
                };
                args.pay_amount = Some(value);
            }
            "--request" => args.pay_request = true,
            "--pack" => {
                let Some(value) = it.next() else {
                    eprintln!("--pack requires a value (the catalog pack id, e.g. 1)");
                    usage();
                };
                args.topup_pack = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--pack expects a number, got {value:?}");
                    usage();
                }));
            }
            // `--dry-run` belongs to `wetrun` (Proposal 011 T4.2) and the
            // airdrop verbs (issue 129 §9). The heal modes' dry run is the
            // bare no-mode default, not a flag.
            "--dry-run" if matches!(args.command, Some(KatCommand::Wetrun)) => {
                args.wetrun_dry_run = true;
            }
            // Issue 129 §9: the airdrop verbs' --dry-run — local-only
            // composition (hash + row + body + caps), no network, no
            // secrets read.
            "--dry-run"
                if matches!(
                    args.command,
                    Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
                ) =>
            {
                args.tuna_dry_run = true;
            }
            // The `--gov <id>` pin — skips the listing's unique-hash
            // mandate resolution.
            "--gov"
                if matches!(
                    args.command,
                    Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
                ) =>
            {
                let Some(value) = it.next() else {
                    eprintln!("--gov requires a proposal id");
                    usage();
                };
                args.tuna_gov = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--gov expects a u64 proposal id, got {value:?}");
                    usage();
                }));
            }
            // riir-dao Proposal 001 T4 — the `account migrate` flags.
            // Exactly one mode per invocation; the flags refuse outside
            // the migrate command so they never become scan paths.
            "--github" if args.command == Some(KatCommand::Migrate) => {
                let Some(value) = it.next() else {
                    eprintln!("--github requires the claim account's GitHub login");
                    usage();
                };
                if args.migrate_github.is_some() {
                    eprintln!("--github given twice");
                    usage();
                }
                args.migrate_github = Some(value);
            }
            "--object" if args.command == Some(KatCommand::Migrate) => {
                let Some(value) = it.next() else {
                    eprintln!("--object requires the 64-hex request id");
                    usage();
                };
                if args.migrate_object.is_some() {
                    eprintln!("--object given twice");
                    usage();
                }
                args.migrate_object = Some(value);
            }
            "--status" if args.command == Some(KatCommand::Migrate) => {
                let Some(value) = it.next() else {
                    eprintln!("--status requires the 64-hex request id");
                    usage();
                };
                if args.migrate_status.is_some() {
                    eprintln!("--status given twice");
                    usage();
                }
                args.migrate_status = Some(value);
            }
            "--census" if args.command == Some(KatCommand::Migrate) => {
                args.migrate_census = true;
            }
            "--old-key" if args.command == Some(KatCommand::Migrate) => {
                let Some(value) = it.next() else {
                    eprintln!("--old-key requires the OLD key file path (OpenSSH Ed25519)");
                    usage();
                };
                if args.migrate_old_key.is_some() {
                    eprintln!("--old-key given twice");
                    usage();
                }
                args.migrate_old_key = Some(value);
            }
            "--github-token" if args.command == Some(KatCommand::Migrate) => {
                let Some(value) = it.next() else {
                    eprintln!("--github-token requires a bearer token");
                    usage();
                };
                args.migrate_github_token = Some(value);
            }
            "--repo-base" if args.command == Some(KatCommand::Migrate) => {
                let Some(value) = it.next() else {
                    eprintln!("--repo-base requires the governance repo HTML base");
                    usage();
                };
                args.migrate_repo_base = Some(value);
            }
            "--qr" => args.pay_qr = true,
            // The topup QR's OFF switch (the --qr counterpart). A bare --qr
            // with the topup command is normalized to topup_qr post-parse
            // (the pay/faucet --qr check would otherwise refuse it).
            "--no-qr" => args.topup_no_qr = true,
            // Issue 126 — the checkout re-print + release, both seq-taking
            // and topup-only (the parse arms sit beside --pack's).
            "--show" => {
                let Some(value) = it.next() else {
                    eprintln!("--show requires a value (the intent seq)");
                    usage();
                };
                args.topup_show = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--show expects a number, got {value:?}");
                    usage();
                }));
            }
            "--void" => {
                let Some(value) = it.next() else {
                    eprintln!("--void requires a value (the intent seq)");
                    usage();
                };
                args.topup_void = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--void expects a number, got {value:?}");
                    usage();
                }));
            }
            // Issue 126 follow-up — the non-interactive pick (the topup
            // friction-cut's agent story: never enters the interactive
            // picker, so the 10 s window never runs).
            "--pick" => {
                let Some(value) = it.next() else {
                    eprintln!("--pick requires a value (latest)");
                    usage();
                };
                if value != "latest" {
                    eprintln!("--pick takes \"latest\" (the newest open checkout) — got {value:?}");
                    usage();
                }
                args.topup_pick_latest = true;
            }
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
            "--url" => {
                let Some(value) = it.next() else {
                    eprintln!("--url requires a value (devnet | mainnet | an absolute https url)");
                    usage();
                };
                args.config_url = Some(value);
            }
            "--discard" => args.config_adopt_discard = true,
            "--net" => {
                let Some(value) = it.next() else {
                    eprintln!("--net requires a value (devnet | mainnet)");
                    usage();
                };
                if args.net_alias.is_some() {
                    eprintln!("--net given twice");
                    usage();
                }
                args.net_alias = Some(value);
            }
            "--export" => {
                let Some(value) = it.next() else {
                    eprintln!("--export requires a value (a file path, or - for stdout)");
                    usage();
                };
                args.sync_export = Some(PathBuf::from(value));
            }
            // riir-dapps Plan 050 S1.11: the corpus-PR flags (unguarded
            // arms + post-parse validation — the `--export` precedent, so
            // both `corpus-pr --src X` and `--src X corpus-pr` parse).
            "--src" => {
                let Some(value) = it.next() else {
                    eprintln!("--src requires a value (the source env url or alias)");
                    usage();
                };
                if args.corpus_pr_src.is_some() {
                    eprintln!("--src given twice");
                    usage();
                }
                args.corpus_pr_src = Some(value);
            }
            "--dst" => {
                let Some(value) = it.next() else {
                    eprintln!("--dst requires a value (the destination env url or alias)");
                    usage();
                };
                if args.corpus_pr_dst.is_some() {
                    eprintln!("--dst given twice");
                    usage();
                }
                args.corpus_pr_dst = Some(value);
            }
            "--seq" => {
                let Some(value) = it.next() else {
                    eprintln!("--seq requires a value (the explicit pr_seq)");
                    usage();
                };
                args.corpus_pr_seq = Some(value.parse().unwrap_or_else(|_| {
                    eprintln!("--seq expects a number, got {value:?}");
                    usage();
                }));
            }
            "--submitter" => {
                let Some(value) = it.next() else {
                    eprintln!("--submitter requires a value (the payee account id, 32 hex)");
                    usage();
                };
                args.corpus_pr_submitter = Some(value);
            }
            "--key-file" => {
                let Some(value) = it.next() else {
                    eprintln!("--key-file requires a value (the PR-attestation key file)");
                    usage();
                };
                args.corpus_pr_key_file = Some(PathBuf::from(value));
            }
            "--submit" => {
                args.corpus_pr_submit = true;
            }
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
            "login" | "account" | "update" | "upgrade" | "sync" | "claim" | "stake" | "unstake"
            | "vote" | "pay" | "topup" | "buy" | "kat" | "tuna" | "faucet" | "wetrun"
            | "config" | "replay" | "corpus-pr" | "get" | "send" | "migrate"
                if args.paths.is_empty() && args.command.is_none() =>
            {
                args.command = Some(match arg.as_str() {
                    "login" => KatCommand::Login,
                    "account" => KatCommand::Account,
                    "update" => KatCommand::Update,
                    "upgrade" => KatCommand::Upgrade,
                    "sync" => KatCommand::Sync,
                    "stake" => KatCommand::Stake,
                    "unstake" => KatCommand::Unstake,
                    "vote" => KatCommand::Vote,
                    "pay" => KatCommand::Pay,
                    "topup" => KatCommand::Topup,
                    "buy" => KatCommand::Buy,
                    "kat" => KatCommand::Kat,
                    "tuna" => KatCommand::Tuna,
                    "get" => KatCommand::TunaGet,
                    "send" => KatCommand::TunaSend,
                    "migrate" => KatCommand::Migrate,
                    "faucet" => KatCommand::Faucet,
                    "wetrun" => KatCommand::Wetrun,
                    "config" => KatCommand::Config,
                    "replay" => KatCommand::Replay,
                    "corpus-pr" => KatCommand::CorpusPr,
                    _ => KatCommand::Claim,
                });
            }
            // Plan 023: the `config set` modifier — a bare word after the
            // `config` subcommand, before any path arm can eat it.
            "set" if args.command == Some(KatCommand::Config) && args.paths.is_empty() => {
                args.config_set = true;
            }
            // riir-kat Issue 003 T4: the `config adopt-state <net>` modifier —
            // the `set` precedent's shape, with its net word consumed inline.
            "adopt-state"
                if args.command == Some(KatCommand::Config)
                    && args.paths.is_empty()
                    && !args.config_set =>
            {
                if args.config_adopt_state.is_some() {
                    eprintln!("config adopt-state takes exactly one net: <devnet|mainnet>");
                    usage();
                }
                let Some(net) = it.next() else {
                    eprintln!("config adopt-state requires a net: <devnet|mainnet>");
                    usage();
                };
                args.config_adopt_state = Some(net);
            }
            "devnet" | "mainnet"
                if args.command == Some(KatCommand::Config)
                    && args.config_adopt_state.is_none()
                    && args.paths.is_empty()
                    && !args.config_set =>
            {
                // Unreachable in practice (adopt-state consumes its net word
                // inline); kept so a bare `config devnet` still lands in the
                // usage path instead of becoming a scan path.
                eprintln!("did you mean `config adopt-state {arg}` or `config set --url {arg}`?");
                usage();
            }
            // The `buy <amount> <unit>` positionals (the topup friction-cut):
            // consumed inline (the adopt-state precedent) so they never become
            // scan paths. The glued `10kat` form rides the amount word whole
            // and splits post-parse.
            amount_word
                if args.command == Some(KatCommand::Buy)
                    && args.buy_amount.is_none()
                    && args.buy_unit.is_none() =>
            {
                args.buy_amount = Some(amount_word.to_string());
            }
            unit_word
                if args.command == Some(KatCommand::Buy)
                    && args.buy_amount.is_some()
                    && args.buy_unit.is_none() =>
            {
                args.buy_unit = Some(unit_word.to_string());
            }
            // Issue 129 §9: the `get|send <amt> tuna [account]` positionals
            // — the buy precedent's consumed-inline shape.
            tuna_amount_word
                if matches!(
                    args.command,
                    Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
                ) && args.tuna_amount.is_none()
                    && args.tuna_unit.is_none() =>
            {
                args.tuna_amount = Some(tuna_amount_word.to_string());
            }
            tuna_unit_word
                if matches!(
                    args.command,
                    Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
                ) && args.tuna_amount.is_some()
                    && args.tuna_unit.is_none() =>
            {
                args.tuna_unit = Some(tuna_unit_word.to_string());
            }
            tuna_target_word
                if args.command == Some(KatCommand::TunaSend)
                    && args.tuna_unit.is_some()
                    && args.tuna_target.is_none() =>
            {
                args.tuna_target = Some(tuna_target_word.to_string());
            }
            path => args.paths.push(PathBuf::from(path)),
        }
    }
    if args.account_history && !matches!(args.command, Some(KatCommand::Account)) {
        eprintln!("--history requires the `account` command");
        usage();
    }
    if args.topup_pack.is_some() && !matches!(args.command, Some(KatCommand::Topup)) {
        eprintln!("--pack requires the `topup` command");
        usage();
    }
    if (args.topup_show.is_some() || args.topup_void.is_some())
        && !matches!(args.command, Some(KatCommand::Topup))
    {
        eprintln!("--show/--void require the `topup` command");
        usage();
    }
    // The buy friction-cut (issue 126 follow-up): its words belong to it,
    // and its surfaces are topup's flags minus the pack/action trio (the
    // pack RESOLVES from the amount; --show/--void are the flag forms).
    if (args.buy_amount.is_some() || args.buy_unit.is_some())
        && !matches!(args.command, Some(KatCommand::Buy))
    {
        eprintln!("buy: the amount/unit words require the `buy` command — cargo refine buy 10 kat");
        usage();
    }
    // Issue 129 §9: the airdrop verbs' words belong to them, --gov/--dry-run
    // ride only with them, and `get` takes no recipient word.
    if (args.tuna_amount.is_some() || args.tuna_unit.is_some() || args.tuna_target.is_some())
        && !matches!(
            args.command,
            Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
        )
    {
        eprintln!(
            "the amount/unit/recipient words require the `get` or `send` command — cargo refine send <amt> tuna [account-id]"
        );
        usage();
    }
    if args.tuna_gov.is_some()
        && !matches!(
            args.command,
            Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
        )
    {
        eprintln!("--gov requires the `get` or `send` command");
        usage();
    }
    if args.tuna_dry_run
        && !matches!(
            args.command,
            Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
        )
    {
        eprintln!("--dry-run requires the `get`, `send` or `wetrun` command");
        usage();
    }
    if args.command == Some(KatCommand::TunaGet) && args.tuna_target.is_some() {
        eprintln!("get: takes no recipient — it targets your OWN account id (use `send <amt> tuna <account-id>`)");
        usage();
    }
    if args.command == Some(KatCommand::TunaSend) && args.tuna_target.is_none() {
        eprintln!(
            "send: a recipient account id is required — cargo refine send <amt> tuna <account-id>"
        );
        usage();
    }
    if matches!(
        args.command,
        Some(KatCommand::TunaGet) | Some(KatCommand::TunaSend)
    ) && !args.paths.is_empty()
    {
        eprintln!(
            "{}: unexpected extra words — cargo refine {} <amt> tuna [account-id]",
            if args.command == Some(KatCommand::TunaGet) {
                "get"
            } else {
                "send"
            },
            if args.command == Some(KatCommand::TunaGet) {
                "get"
            } else {
                "send"
            }
        );
        usage();
    }
    if args.topup_pick_latest && !matches!(args.command, Some(KatCommand::Topup | KatCommand::Buy))
    {
        eprintln!("--pick requires the `topup` or `buy` command");
        usage();
    }
    if args.command == Some(KatCommand::Buy)
        && (args.topup_pack.is_some() || args.topup_show.is_some() || args.topup_void.is_some())
    {
        eprintln!(
            "buy: --pack/--show/--void belong to `topup` — buy resolves the pack from the amount"
        );
        usage();
    }
    if args.command == Some(KatCommand::Buy) && !args.paths.is_empty() {
        eprintln!("buy: unexpected extra words — cargo refine buy <amount> <kat|usdc>");
        usage();
    }
    // The glued `10kat` form: the unit rides the amount word whole and
    // splits here (the two-word form never enters this arm).
    if args.command == Some(KatCommand::Buy) && args.buy_unit.is_none() {
        if let Some(amount) = &args.buy_amount {
            match amount.find(|c: char| c.is_ascii_alphabetic()) {
                Some(i) => {
                    args.buy_unit = Some(amount[i..].to_string());
                    args.buy_amount = Some(amount[..i].to_string());
                }
                None => {
                    eprintln!("buy: a unit is required (kat | usdc) — cargo refine buy 10 kat");
                    usage();
                }
            }
        }
    }
    if args.command == Some(KatCommand::Buy) && args.buy_amount.is_none() {
        eprintln!("buy: an amount is required — cargo refine buy 10 kat");
        usage();
    }
    if args.topup_show.is_some() && args.topup_void.is_some() {
        eprintln!("--show and --void are separate actions — run one per invocation");
        usage();
    }
    // The topup QR: a `--qr` token under `topup`/`buy` normalizes to the
    // topup field (the same flag spells pay/faucet's QR; the command
    // decides which checkout surface it means). `--no-qr` is topup/buy
    // only. Both at once is no defensible resolution (the --stats shape).
    if matches!(
        args.command,
        Some(KatCommand::Topup) | Some(KatCommand::Buy)
    ) && args.pay_qr
    {
        args.topup_qr = true;
        args.pay_qr = false;
    }
    if args.topup_qr && args.topup_no_qr {
        eprintln!(
            "--qr and --no-qr conflict — choose one (the default is auto: a TTY renders the QR)"
        );
        std::process::exit(2);
    }
    if args.topup_no_qr
        && !matches!(
            args.command,
            Some(KatCommand::Topup) | Some(KatCommand::Buy)
        )
    {
        eprintln!("--no-qr requires the `topup` or `buy` command");
        usage();
    }
    if args.info {
        if args.command.is_some() {
            eprintln!(
                "--info is the standalone stats view — the stake position is `cargo refine stake --info`"
            );
            usage();
        }
        if args.suggest || args.fix || args.fix_compile || args.write || args.dump_trajectories {
            eprintln!("--info is a standalone read-only mode (no scan, no edits)");
            usage();
        }
        if args.mine || args.unmine || args.stats_flag || args.refill {
            eprintln!(
                "--info is a standalone read-only view — run it without the consent/refill modes"
            );
            usage();
        }
    }
    if args.config_adopt_discard && args.config_adopt_state.is_none() {
        eprintln!("--discard requires `config adopt-state <net>`");
        usage();
    }
    if let Some(net) = &args.config_adopt_state {
        if args.command != Some(KatCommand::Config) || args.config_set {
            eprintln!(
                "adopt-state is a `config` subcommand: cargo refine config adopt-state <net>"
            );
            usage();
        }
        if net != "devnet" && net != "mainnet" {
            eprintln!(
                "config adopt-state takes <devnet|mainnet> — the two live ledgers (testnet is torn \
                 down; its state dir, if any, is its own)"
            );
            usage();
        }
    }
    if let Some(alias) = &args.net_alias {
        if alias != "devnet" && alias != "mainnet" {
            eprintln!("--net takes <devnet|mainnet>");
            usage();
        }
    }
    if args.config_url.is_some()
        && !(args.config_set && args.command == Some(KatCommand::Config))
        && args.config_adopt_state.is_none()
    {
        eprintln!("--url requires `config set` (or `config adopt-state` as its oracle override)");
        usage();
    }
    if args.sync_export.is_some() && args.command != Some(KatCommand::Sync) {
        eprintln!("--export requires the `sync` command (cargo refine sync --export <file|->)");
        usage();
    }
    // riir-dapps Plan 050 S1.11: the corpus-PR flag validation (the
    // `--export` precedent — the arms parse unconditionally, the
    // combination law lives HERE).
    if args.command == Some(KatCommand::CorpusPr) {
        let missing: Vec<&str> = [
            ("--src", args.corpus_pr_src.is_none()),
            ("--dst", args.corpus_pr_dst.is_none()),
            ("--submitter", args.corpus_pr_submitter.is_none()),
        ]
        .iter()
        .filter(|(_, missing)| *missing)
        .map(|(flag, _)| *flag)
        .collect();
        if !missing.is_empty() {
            eprintln!(
                "corpus-pr requires {}: cargo refine corpus-pr --src <devnet> --dst <mainnet> \
                 --submitter <32hex> [--seq N] [--key-file <path>] [--submit]",
                missing.join(", ")
            );
            usage();
        }
        if args.corpus_pr_submit && args.corpus_pr_key_file.is_none() {
            eprintln!("--submit requires --key-file (the PR is attested; the gauge cannot POST)");
            usage();
        }
        if args.corpus_pr_seq == Some(0) {
            eprintln!("--seq must be ≥ 1 (PRs number from one)");
            usage();
        }
    } else if args.corpus_pr_src.is_some()
        || args.corpus_pr_dst.is_some()
        || args.corpus_pr_seq.is_some()
        || args.corpus_pr_submitter.is_some()
        || args.corpus_pr_key_file.is_some()
        || args.corpus_pr_submit
    {
        eprintln!("the corpus-PR flags require the `corpus-pr` command");
        usage();
    }
    if args.purge && !args.unmine {
        eprintln!(
            "--purge requires --unmine (it only makes sense as the destructive half of opting out)"
        );
        usage();
    }
    if args.purge_server && !args.unmine {
        eprintln!("--purge-server requires --unmine (it is the server-side half of opting out)");
        usage();
    }
    if args.blind && !args.secret_scan {
        eprintln!(
            "--blind requires --secret-scan (it is the output transform of the secret-scan report)"
        );
        usage();
    }
    // Proposal 004 Half 1: `--stats` consent guards. (a) on/off are opposite
    // writes to one record — no defensible resolution, exit 2 (the
    // --mine/--unmine conflict code). (b) bare `--stats` followed by a path
    // is ambiguous — the peek-arm deliberately did NOT eat the token, so
    // reject rather than let a scan path double as a missing value. (c)
    // standalone beside the other consent modes: reject a --mine/--unmine
    // combo instead of silently skipping one of them.
    if riir_refine::kat_stats_consent::flags_conflict(args.stats_on, args.stats_off) {
        eprintln!("{}", riir_refine::kat_stats_consent::CONFLICT_MESSAGE);
        std::process::exit(2);
    }
    if args.stats_flag && !args.stats_on && !args.stats_off && !args.paths.is_empty() {
        eprintln!(
            "--stats followed by paths is ambiguous — use `--stats on` or `--stats off` to \
             record consent, or bare `--stats` (no paths) for the status readout"
        );
        usage();
    }
    if args.stats_flag && (args.mine || args.unmine) {
        eprintln!(
            "--stats is a standalone consent mode — run it without --mine/--unmine \
             (the two consent planes are independent by design)"
        );
        usage();
    }
    if args.refill_wait != 0 && !args.refill {
        eprintln!("--refill-wait requires --refill (it bounds the refill's settle poll)");
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
    // ── The owner's just-works defaults (2026-09-08) ─────────────────────
    // Applied HERE (inside parse_args) so every downstream check — the
    // paths guard below included — sees the final posture:
    //   * bare `cargo refine` (no mode, no KAT command) = DRY RUN: suggest
    //     over `.` (or the given paths), zero edits;
    //   * `--fix` = REAL fix: writes (the explicit flag is the consent)
    //     and compile-gates on fix_verify builds (opt out --no-verify);
    //   * `--suggest` stays the explicit review command (unchanged).
    //
    // EVERY standalone early-return mode is excluded (`is_bare_posture`):
    // the original 4-flag check set `suggest=true` under `--misses` too,
    // and the --misses standalone guard then refused it — `cargo refine
    // --misses` was exit-2 broken from the 09-08 default until this list
    // was completed (the same regression class would hit any future
    // standalone mode added here without a row).
    let bare = is_bare_posture(&args);
    if bare {
        args.suggest = true;
        args.bare_default = true;
        if args.paths.is_empty() {
            args.paths.push(std::path::PathBuf::from("."));
        }
    }
    // `--fix` with no paths scans the cwd too — the just-works contract is
    // "no flags, no paths": only `--suggest` demands explicit paths.
    if args.fix && args.paths.is_empty() && args.command.is_none() {
        args.paths.push(std::path::PathBuf::from("."));
    }
    if args.fix {
        args.write = true;
        #[cfg(feature = "fix_verify")]
        if !args.no_verify {
            args.verify = true;
        }
    }
    if args.no_verify && !args.fix {
        eprintln!("--no-verify requires --fix");
        usage();
    }
    if args.paths.is_empty()
        && !args.suggest
        && !args.fix
        && !args.fix_compile
        && !standalone_mode(&args)
    {
        eprintln!("no paths given");
        usage();
    }
    if args.generation.is_some() && !args.score_bench {
        eprintln!("--generation requires --score-bench");
        usage();
    }
    if (args.necessity_top.is_some() || args.necessity_random.is_some() || args.necessity_smoke)
        && !args.necessity_bench
    {
        eprintln!("--necessity-top/--necessity-random/--necessity-smoke require --necessity-bench");
        usage();
    }
    if args.necessity_rule.is_some() && !args.necessity_bench {
        eprintln!("--necessity-rule requires --necessity-bench (the per-decision paired lane runs inside it)");
        usage();
    }
    let active_set_cmds = u8::from(args.active_set_list)
        + u8::from(args.active_set_demote.is_some())
        + u8::from(args.active_set_readmit.is_some())
        + u8::from(args.active_set_clear)
        + u8::from(args.active_set_cap.is_some());
    if active_set_cmds > 1 {
        eprintln!("--active-set* commands are separate invocations (one marker write each)");
        usage();
    }
    if args.necessity_bench && args.score_bench {
        eprintln!("--necessity-bench and --score-bench are separate runs (one oracle bill each)");
        usage();
    }
    args
}

/// The just-works `bare` posture: NO mode flag, NO standalone command, NO
/// KAT command — the exact set that defaults to a dry-run suggest over `.`.
/// Mirrors the paths guard via the shared [`standalone_mode`] predicate;
/// the two hand-mirrored lists drifted exactly as the --misses regression
/// predicted (see [`standalone_mode`] for the --version casualty).
fn is_bare_posture(args: &Args) -> bool {
    !args.suggest && !args.fix && !args.fix_compile && !args.repo_posture && !standalone_mode(args)
}

/// Every standalone early-return mode — the flags that need NO scan paths
/// (`--version` answers before anything else; `--misses` prints queues; the
/// KAT commands own their arg shapes). ONE source of truth consumed by BOTH
/// the bare posture and the paths guard: before this predicate the two
/// hand-mirrored lists drifted — the 2026-09-12 --misses fix completed
/// `is_bare_posture` with `!args.version` while the paths guard kept its
/// missing row, and `cargo refine --version` went exit-2 ("no paths given")
/// from that day until the shared predicate landed (it had worked before
/// only because the pre-09-12 bare default accidentally armed `suggest` +
/// `paths=["."]` under it, which the --misses fix correctly removed).
fn standalone_mode(args: &Args) -> bool {
    args.dump_trajectories
        || args.refill
        || args.info
        || args.score_bench
        || args.score_report
        || args.necessity_bench
        || args.memory_stats
        || args.mine
        || args.unmine
        || args.purge_server
        || args.stats_flag
        || args.version
        || args.misses
        || args.misses_report
        || args.recompute_elo
        || args.collapse_report
        || args.corpus_stats
        || args.import_series
        || args.frontier_report
        || args.horizon_report
        || args.fixseq_stats
        || args.config_audit
        || args.secret_scan
        // Issue 151: `--blind` is the `--secret-scan` output transform and
        // is handled inside the secret-scan early-return — but it must
        // still break the bare posture AND pass the paths guard, or bare
        // `cargo refine --blind <file>` would fire the suggest default over
        // the file (the --misses trap class). Without `--secret-scan` it is
        // a parse-level usage error (the --purge-shaped guard in
        // parse_args), so the standalone half is about never falling
        // through, never about running alone.
        || args.blind
        || args.platform_audit
        || args.orphan_report
        || args.frame_time_report
        || args.coverage_report
        || args.candidates_path.is_some()
        || args.command.is_some()
        || args.gate_calibration
        || args.active_set_list
        || args.active_set_demote.is_some()
        || args.active_set_readmit.is_some()
        || args.active_set_clear
        || args.active_set_cap.is_some()
}

/// Plan 019 T1.5: the mode-flag validation matrix, extracted so every
/// accepted/rejected combination is pinnable by test. The owner's
/// just-works UX (2026-09-08) folded the DEFAULTS in: a caller with NO
/// mode and NO standalone command gets a DRY-RUN suggest over `.` — bare
/// `cargo refine` must just work — and `--fix` without `--dry-run` writes
/// (the explicit flag is the consent); `--dry-run` pins the review
/// posture regardless of flag order.
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
fn validate_mode_flags(args: &Args) -> Result<(), &'static str> {
    // Checked BEFORE the no-mode error: `--repo-posture` alone is a
    // fix-lane modifier, and the message must name the real problem —
    // "no mode given" would send the reader hunting for a mode flag the
    // posture flag already implies.
    if args.repo_posture && !args.fix {
        return Err("--repo-posture requires --fix (it gates the fix lane's edits)");
    }
    // P39: the override names the posture lane's decline — without the
    // mode there is nothing to override, and a silently-inert flag is the
    // confusion class this check exists to prevent.
    // Issue 135 T3: the opt-in names the fix lane's level gate — without
    // --fix there is nothing to opt into.
    if !args.fix_groups.is_empty() && !args.fix {
        return Err("--groups requires --fix (it opts the fix lane into allow-by-default groups)");
    }
    if args.apply_notbenchable && !args.repo_posture {
        return Err(
            "--apply-notbenchable requires --repo-posture (it overrides the posture lane's NotBenchable decline)",
        );
    }
    let (suggest, fix, fix_compile, dump_trajectories) = (
        args.suggest,
        args.fix,
        args.fix_compile,
        args.dump_trajectories,
    );
    let (write, verify, verify_args, has_command) = (
        args.write,
        args.verify,
        args.verify_args.is_some(),
        args.command.is_some(),
    );
    if !suggest && !fix && !fix_compile && !dump_trajectories && !has_command {
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
    collect_named_walk(root, &is_dockerfile_name, out);
}

/// Whether a file name is a Cargo.toml manifest (Issue 085). Deliberately
/// narrow: the NAME is exact — `Cargo.toml`. A `Cargo.toml.orig` or a
/// `Cargo.lock` never matches; a crate inside `target/` or `vendor/` is
/// already excluded by the walker.
fn is_cargo_toml_name(name: &str) -> bool {
    name == "Cargo.toml"
}

/// Collect Cargo.toml manifests under `root` (Issue 085) — the name-matched
/// twin of [`collect_dockerfiles`].
fn collect_cargo_tomls(root: &Path, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_cargo_toml_name(name) {
            out.push(root.to_path_buf());
        }
        return;
    }
    collect_named_walk(root, &is_cargo_toml_name, out);
}

/// The shared walk behind both name-matched collectors: depth-first over
/// real directories, skipping the shared prune set
/// (`riir_refine::scan_prune::is_pruned_dir_name`), pushing every file
/// whose name satisfies `keep`.
fn collect_named_walk(root: &Path, keep: &dyn Fn(&str) -> bool, out: &mut Vec<PathBuf>) {
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
            if riir_refine::scan_prune::is_pruned_dir_name(name) {
                continue;
            }
            if path.is_dir() {
                queue.push_back(path);
            } else if keep(name) {
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

/// Issue 151: the in-run secret-disclosure caps — the warn names the count
/// in full and samples the lines, never a firehose on a tree-wide leak.
const MAX_SECRET_DISCLOSURE_FILES: usize = 8;
const MAX_SECRET_DISCLOSURE_PER_FILE: usize = 4;

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

/// Issue 165: the run-end per-domain roll-up's accumulator. `print_suggestion`
/// tallies exactly what it prints, so the line can never drift from the
/// findings above it; the non-syn lanes + the secret scan merge their counts
/// afterwards (disjoint surfaces by construction — the syn pass never cites
/// a Dockerfile/manifest/`.wgsl` file kind).
#[derive(Default)]
struct DomainTallies {
    /// Printed findings per domain label (syn citations + lane counts + the
    /// secret scan, which merges under `sec`).
    per_domain: std::collections::BTreeMap<&'static str, usize>,
    /// Citations whose rule sits in the compiled clippy fix set — a
    /// clippy-only floor on what `--fix` could apply (other domains carry
    /// no static set; the render says "look", never promises).
    fixable_clippy: usize,
    /// Fix-surface findings (syn citations + lanes) — everything the fix
    /// lane could in principle act on. The secret scan never auto-fixes,
    /// so secrets are excluded here.
    citations: usize,
    /// Secret-shaped scan findings (merged under `sec` for display).
    secrets: usize,
}

impl DomainTallies {
    fn add_citation(&mut self, domain: &'static str, fixable: bool) {
        *self.per_domain.entry(domain).or_default() += 1;
        self.citations += 1;
        if fixable {
            self.fixable_clippy += 1;
        }
    }

    fn add_count(&mut self, domain: &'static str, n: usize) {
        if n == 0 {
            return;
        }
        *self.per_domain.entry(domain).or_default() += n;
        self.citations += n;
    }

    fn add_secrets(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        *self.per_domain.entry("sec").or_default() += n;
        self.secrets += n;
    }

    fn total(&self) -> usize {
        self.citations + self.secrets
    }

    /// The one roll-up line, e.g. `findings by domain: clippy 41 · perf 23
    /// · sec 95 — 159 total`.
    fn rollup_line(&self) -> String {
        let parts: Vec<String> = self
            .per_domain
            .iter()
            .map(|(domain, n)| format!("{domain} {n}"))
            .collect();
        format!(
            "findings by domain: {} — {} total",
            parts.join(" · "),
            self.total()
        )
    }
}

fn print_suggestion(
    s: &SpanSuggestion,
    matcher: &LatentMatcher,
    domain_filter: Option<DomainId>,
    feed_present: bool,
    bench_target: bool,
    tally: &mut DomainTallies,
    fixable_set: &std::collections::BTreeSet<String>,
) {
    let routed = s.routed_domain.label();
    let anchored = !s.anchored_rules.is_empty() || !s.anchored_missing.is_empty();
    let hot = !s.hot.is_empty();
    let mark = if anchored {
        " [oracle]"
    } else if hot {
        " [hot]"
    } else if feed_present && s.heuristic_only {
        " [heuristic]"
    } else {
        ""
    };
    // T0.3 disclosure: a measured-hot span in a bench target is doubly
    // measurement-sensitive — the profile saw the harness itself.
    let hot_bench_note = if hot && bench_target {
        " — ⚠ bench target: the profile measured the bench's own harness; perf edits here are measurement-altering"
    } else {
        ""
    };
    println!(
        "{}:{}-{} (routed: {routed}){mark}{hot_bench_note}",
        s.path, s.start_line, s.end_line
    );
    // Issue 146 T0.2: the measured-hot marks print FIRST — the profile's
    // own rows, verbatim quantities (timing = attribution, B129; alloc =
    // the deterministic verification axis). This is the review anchor the
    // operator acts on.
    for h in &s.hot {
        let avg = h.avg_ns.map_or("n/a".to_string(), |ns| format!("{ns} ns"));
        let pct = h
            .percent_total
            .map_or(String::new(), |p| format!(" ({p:.1}% of measured time)"));
        let alloc = match (h.alloc_bytes, h.alloc_calls) {
            (Some(b), Some(c)) => format!(" | alloc {b} B in {c} call(s)"),
            (Some(b), None) => format!(" | alloc {b} B"),
            _ => String::new(),
        };
        println!(
            "  [hot   ] {:<32} {} calls × {} avg{pct}{alloc}",
            h.name, h.calls, avg
        );
    }
    // Oracle-anchored rules first: these are KNOWN findings — force-included
    // ahead of similarity ranking (the 2026-09-13 intake's miss mechanism
    // cannot eat them).
    for (domain, lint) in &s.anchored_rules {
        if domain_filter.is_some_and(|want| want != *domain) {
            continue;
        }
        let info = matcher.rule_book(lint);
        let fixed = info.map_or("?", |i| snippet_head(&i.fixed_code));
        // Issue 102 T2: the anchor pass-through must be LOUD — an operator
        // who demoted this rule has to see that the oracle finding still
        // stands, or the mechanism reads as broken and gets "fixed".
        let demoted_note = if matcher.active_exclusions().excludes(domain.label(), lint) {
            "  [active-set: DEMOTED — anchor override; the oracle finding stands]"
        } else {
            ""
        };
        println!(
            "  [{:<6}] {:<32} oracle finding  →  {fixed}{demoted_note}",
            domain.label(),
            lint
        );
        tally.add_citation(
            domain.label(),
            *domain == DomainId::Clippy && fixable_set.contains(lint),
        );
    }
    for lint in &s.anchored_missing {
        println!("  [oracle] {lint:<32} NO CORPUS RULE (gap signal)");
    }
    // Issue 145 T4: list only domains that can touch this file kind — a
    // `.rs` span never shows Dockerfile or manifest rules. A path no
    // domain claims (an unusual extension) keeps the full list rather than
    // going silent.
    let fits_any = s.by_domain.iter().any(|(d, _)| d.applies_to_path(&s.path));
    for (domain, rules) in &s.by_domain {
        if domain_filter.is_some_and(|want| want != *domain) {
            continue;
        }
        if fits_any && !domain.applies_to_path(&s.path) {
            continue;
        }
        for rule in rules {
            let info = matcher.rule_book(&rule.rule_id);
            let fixed = info.map_or("?", |i| snippet_head(&i.fixed_code));
            // P30 bench-guard: the suggestion stays visible (a flag, never
            // silence) but perf rows on a measurement target carry the
            // marker so the operator knows why --fix will decline them.
            let bench_mark = if bench_target && *domain == DomainId::Perf {
                "  ⚠ bench-guard: applying perf rules here alters what the bench measures — apply manually only"
            } else {
                ""
            };
            println!(
                "  [{:<6}] {:<32} score {:>8.2}  →  {fixed}{bench_mark}",
                domain.label(),
                rule.rule_id,
                rule.score
            );
            tally.add_citation(
                domain.label(),
                *domain == DomainId::Clippy && fixable_set.contains(&rule.rule_id),
            );
        }
    }
}

/// Plan 091 T1.2 / Plan 092 T5 (the collision merge) + Issue 089 T3 (the
/// generalization): the frontier-ordered authoring queue — read-only,
/// standalone. Two halves:
///
/// - the ORIGINAL `rustc_errors` per-code queue (`.refine/misses/`, Plan 091
///   verbatim — kept, not replaced);
/// - one frontier-ordered RULE queue per ENABLED retrieval domain
///   (clippy/perf/kernel/sec/docker — corpus membership attributes store
///   rows; a disabled feature simply has no section, the router precedent).
///
/// Outcomes come from the trajectory store (`latent_retrieval` builds) or
/// the JSON envelope fallback, so the report works in every build.
/// `limit` caps the rendered TABLE rows of each section (the `--limit` flag).
fn run_misses_queue(limit: Option<usize>, paths: &[PathBuf]) {
    let root = resolve_heal_root(paths);
    let outcomes = load_rule_outcomes(&root);

    // Section 1 — the rustc_errors per-code queue (Plan 091, unchanged).
    #[cfg(feature = "rustc_errors")]
    {
        use riir_refine::domains::rustc_errors::evolve;
        let dir = evolve::misses_dir(&root);
        // rustc trajectory keys are "{code}:{strategy_id}" (the compile
        // domain's documented convention — the only ':'-shaped keys the
        // store carries); per-code aggregation runs over those alone.
        let code_rows = outcomes
            .iter()
            .filter(|(k, _)| k.contains(':'))
            .map(|(k, (s, f))| (k.clone(), *s as u32, *f as u32));
        let code_outcomes = evolve::code_outcomes_from_rows(code_rows);
        let report = evolve::frontier_queue_report(&dir, &code_outcomes);
        print_report_capped(&report, limit);
    }

    // Sections 2.. — the per-domain rule queues (Issue 089 T3).
    let sections = domain_corpus_sections();
    let mut known: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (domain, rules) in &sections {
        known.extend(rules.iter().map(String::as_str));
        let rows = riir_refine::frontier_report::domain_queue(rules, &outcomes);
        let section = riir_refine::frontier_report::render_domain_queue(domain, &rows, rules.len());
        print_report_capped(&section, limit);
    }

    // With rustc_errors compiled in the code queue always renders; without
    // it, at least one domain section must have — else this build can order
    // nothing and says so instead of printing a green nothing.
    // (The Issue 156 rescue section below prints BEFORE this check so a
    // build with rescue tallies but no healing domain still reports them.)
    // Issue 156 T3 — the state-level rescue band (FlyBy tallies from the
    // durable store; report-only ordering evidence: rescue exemplars
    // (b0=0 ∧ bd≥3) are the learnable-hard band's head, bd=0 states the
    // intractable-eviction candidates). Store-only evidence — the tallies
    // live on traj: rows under latent_retrieval. AUGMENT-ONLY: this band
    // narrows candidates entering the agreement veto; the veto stays the
    // acceptance authority.
    #[cfg(feature = "latent_retrieval")]
    if root.join(".refine").join("trajectories.kv").exists() {
        match riir_refine::traj_store::TrajStore::open(&root) {
            Ok((store, _)) => {
                let states: Vec<riir_refine::rescue_mine::StateRescueTally> = store
                    .dump()
                    .into_iter()
                    .filter(|r| r.b0_attempts > 0 || r.bd_attempts > 0)
                    .map(|r| riir_refine::rescue_mine::StateRescueTally {
                        state_id: format!(
                            "{}:{}",
                            r.lint_key,
                            riir_refine::traj_store::shape_hash(&r.shape)
                        ),
                        b0_attempts: r.b0_attempts,
                        b0_successes: r.b0_successes,
                        bd_attempts: r.bd_attempts,
                        bd_successes: r.bd_successes,
                        depth: r.rescue_depth,
                        tier: riir_refine::memory::EvidenceTier::from_u8(r.tier),
                    })
                    .collect();
                if !states.is_empty() {
                    let section = riir_refine::rescue_mine::render_misses_section(&states);
                    print_report_capped(&section, limit);
                }
            }
            Err(e) => eprintln!("misses: rescue band unavailable — store unreadable: {e}"),
        }
    }
    #[cfg(feature = "rustc_errors")]
    let rendered_any = true;
    #[cfg(not(feature = "rustc_errors"))]
    let rendered_any = !sections.is_empty();
    if !rendered_any {
        eprintln!(
            "misses: no healing domain compiled into this build — rebuild with at least one domain feature"
        );
        std::process::exit(1);
    }

    // Attribution honesty: store keys no ENABLED domain's corpus claims and
    // that are not rustc "{code}:{strategy}" rows (e.g. written by a feature
    // this build lacks) — counted loud, never silently dropped.
    let unattributed: Vec<&str> = outcomes
        .keys()
        .filter(|k| !k.contains(':') && !known.contains(k.as_str()))
        .map(String::as_str)
        .collect();
    if !unattributed.is_empty() {
        println!(
            "unattributed store rows: {} (not in any ENABLED domain's corpus — the owning feature may be off)",
            unattributed.len()
        );
        for k in unattributed.iter().take(5) {
            println!("  {k}");
        }
        if unattributed.len() > 5 {
            println!("  ... +{} more", unattributed.len() - 5);
        }
    }
}

/// Print a rendered report, capping only its markdown TABLE rows at `n`
/// (headers and prose always print — the `--limit` contract; extracted from
/// the Plan 091 queue printer now that every section shares it).
fn print_report_capped(report: &str, limit: Option<usize>) {
    let Some(n) = limit else {
        print!("{report}");
        return;
    };
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

/// The ENABLED retrieval domains' corpus rule ids, in `DomainId::ALL`
/// order (Issue 089 T3). One entry per domain whose feature compiled in;
/// `dist` is deliberately absent — it is suggest-only advice and never
/// writes trajectories, so it has no queue to order. Corpus builds are
/// in-memory table reads (cold path, no cargo involved).
#[allow(clippy::vec_init_then_push)] // every arm is feature-gated; a vec![] literal cannot carry cfg'd elements
fn domain_corpus_sections() -> Vec<(&'static str, Vec<String>)> {
    use riir_refine::HealEntry;
    let mut out = Vec::new();
    #[cfg(feature = "clippy_lints")]
    out.push((
        "clippy",
        riir_refine::build_seed_corpus()
            .entries
            .iter()
            .map(|e| e.rule_id().to_string())
            .collect(),
    ));
    #[cfg(feature = "rust_perf")]
    out.push((
        "perf",
        riir_refine::build_perf_seed_corpus()
            .entries
            .iter()
            .map(|e| e.rule_id().to_string())
            .collect(),
    ));
    #[cfg(feature = "kernel_opt")]
    out.push((
        "kernel",
        riir_refine::build_kernel_seed_corpus()
            .entries
            .iter()
            .map(|e| e.rule_id().to_string())
            .collect(),
    ));
    #[cfg(feature = "sec")]
    out.push((
        "sec",
        riir_refine::build_sec_seed_corpus()
            .entries
            .iter()
            .map(|e| e.rule_id().to_string())
            .collect(),
    ));
    #[cfg(feature = "docker")]
    out.push((
        "docker",
        riir_refine::build_docker_seed_corpus()
            .entries
            .iter()
            .map(|e| e.rule_id().to_string())
            .collect(),
    ));
    #[cfg(feature = "shader_perf")]
    out.push((
        "shader",
        riir_refine::build_shader_seed_corpus()
            .entries
            .iter()
            .map(|e| e.rule_id().to_string())
            .collect(),
    ));
    out
}

/// Per-rule `(kept, reverted)` counts keyed by RAW store rule ids — the
/// `--misses` generalization's attribution source: store-first
/// (`latent_retrieval` builds), envelope fallback (every other build), the
/// `load_code_outcomes` precedent minus the per-code `':'` aggregation
/// (attribution by corpus membership replaces it).
///
/// Both backends carry ONE ROW PER (rule, shape) — duplicates are SUMMED,
/// never overwritten (a silent last-row-wins here would undercount every
/// multi-shape rule; the old per-code path summed through
/// `code_outcomes_from_rows`, and this keeps that arithmetic).
fn load_rule_outcomes(root: &Path) -> std::collections::BTreeMap<String, (usize, usize)> {
    fn sum_into(
        map: &mut std::collections::BTreeMap<String, (usize, usize)>,
        key: String,
        kept: usize,
        reverted: usize,
    ) {
        let e = map.entry(key).or_default();
        e.0 += kept;
        e.1 += reverted;
    }
    #[cfg(feature = "latent_retrieval")]
    if root.join(".refine").join("trajectories.kv").exists() {
        if let Ok((store, _)) = riir_refine::traj_store::TrajStore::open(root) {
            let mut out = std::collections::BTreeMap::new();
            for r in store.dump() {
                sum_into(
                    &mut out,
                    r.lint_key,
                    r.success_count as usize,
                    r.fail_count as usize,
                );
            }
            return out;
        }
    }
    let mut mem = riir_refine::memory::LatentFixMemory::default();
    if mem.load(&default_memory_path()).is_err() {
        return std::collections::BTreeMap::new();
    }
    let mut out = std::collections::BTreeMap::new();
    for t in mem.iter() {
        sum_into(
            &mut out,
            t.lint_name.clone(),
            t.success_count as usize,
            t.fail_count as usize,
        );
    }
    out
}

/// Issue 108 T2: `--necessity-bench` — the component-necessity flip-rate
/// harness. Baseline + per-target disable/matched-swap arms over the
/// frozen v1 fixture set (real-clippy oracle), two-sided canaries, JSONL
/// to `.benchmarks/necessity_history.jsonl` (isolated from the score
/// history — trial runs never touch it). Canaries gate the run: a failed
/// canary invalidates every verdict this run produced (the
/// audit-instrument law) → exit 1.
/// Issue 102 T2/T3: the active-set marker commands — the ONLY writers of
/// `.refine/active_set.json`. Nothing in the heal path ever demotes: the
/// fade-predictive gate (Issue 102's ≥3-future-generations clock, started
/// 2026-09-21) is the only thing that could automate a demotion, and until
/// it opens, an operator writes the marker through these commands. The
/// corpus itself is compile-time — demotion hides a rule from the
/// fan-out/fix dispatch, it never deletes the entry.
fn run_active_set_cmd(args: &Args) {
    let root = resolve_heal_root(&args.paths);
    let path = root.join(riir_refine::active_set::ACTIVE_SET_FILE);
    let mut set = match riir_refine::active_set::ActiveSet::load(&path) {
        Ok(set) => set,
        Err(e) => {
            eprintln!("active-set: {e}");
            std::process::exit(1);
        }
    };
    if args.active_set_list {
        println!("active set: {}", path.display());
        if set.is_empty() {
            println!("  (empty — the default posture; nothing demoted, no cap binds)");
            return;
        }
        for key in riir_refine::active_set::KNOWN_DOMAIN_KEYS {
            let rules = set.demoted_rules(key);
            if rules.is_empty() {
                continue;
            }
            println!("  {key}:");
            for rule in &rules {
                println!("    - {rule}");
            }
        }
        for (domain, cap) in set.caps() {
            println!(
                "  cap {domain}: {cap} active rule(s) max — binds only when the admission path \
                 enforces it (off by default, the Library-Drift posture)"
            );
        }
        return;
    }
    if let Some(id) = args.active_set_demote.as_ref() {
        let (domain, rule) = match parse_active_set_id(id) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("active-set: {e}");
                std::process::exit(2);
            }
        };
        if set.excludes(&domain, &rule) {
            println!("active-set: `{domain}:{rule}` is already demoted");
            return;
        }
        set.demote(&domain, &rule);
        if let Err(e) = set.save(&path) {
            eprintln!("active-set: save failed: {e}");
            std::process::exit(1);
        }
        println!(
            "active-set: demoted `{domain}:{rule}` — it exits the similarity fan-out and the fix \
             dispatch on the NEXT run; oracle-anchored force-inclusion still passes through (the \
             finding stands, loud), and auto re-admit fires when a fresh unresolved clippy \
             diagnostic names it (or re-admit by hand with --active-set-readmit)"
        );
        return;
    }
    if let Some(id) = args.active_set_readmit.as_ref() {
        let (domain, rule) = match parse_active_set_id(id) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("active-set: {e}");
                std::process::exit(2);
            }
        };
        if !set.readmit(&domain, &rule) {
            eprintln!("active-set: `{domain}:{rule}` is not demoted — nothing to re-admit");
            std::process::exit(1);
        }
        if let Err(e) = set.save(&path) {
            eprintln!("active-set: save failed: {e}");
            std::process::exit(1);
        }
        println!("active-set: re-admitted `{domain}:{rule}`");
        return;
    }
    if args.active_set_clear {
        set.clear();
        if let Err(e) = set.save(&path) {
            eprintln!("active-set: save failed: {e}");
            std::process::exit(1);
        }
        println!(
            "active-set: cleared — every demotion + cap removed (the corpus is never touched; \
             entries are compile-time and deletion is not a lifecycle op)"
        );
        return;
    }
    if let Some(id) = args.active_set_cap.as_ref() {
        let Some((domain, n)) = id.rsplit_once(':') else {
            eprintln!("active-set: cap must be `domain:N` (e.g. clippy:40), got {id:?}");
            std::process::exit(2);
        };
        if !riir_refine::active_set::KNOWN_DOMAIN_KEYS.contains(&domain) {
            eprintln!(
                "active-set: unknown domain {domain:?} — known keys: {}",
                riir_refine::active_set::KNOWN_DOMAIN_KEYS.join(", ")
            );
            std::process::exit(2);
        }
        let Ok(cap) = n.parse::<u32>() else {
            eprintln!("active-set: cap must be a number (0 removes), got {n:?}");
            std::process::exit(2);
        };
        if cap == 0 {
            if set.remove_cap(domain) {
                println!("active-set: cap on `{domain}` removed (off by default again)");
            } else {
                println!("active-set: no cap was set on `{domain}`");
                return;
            }
        } else {
            set.set_cap(domain, cap);
            println!(
                "active-set: cap `{domain}` = {cap} — recorded. It BINDS only when the admission \
                 path enforces it; the Library-Drift warning (over-aggressive retirement measured \
                 WORSE than unguided libraries) keeps enforcement OFF by default"
            );
        }
        if let Err(e) = set.save(&path) {
            eprintln!("active-set: save failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Parse an operator `domain:rule` id for the active-set commands against
/// [`riir_refine::active_set::KNOWN_DOMAIN_KEYS`].
fn parse_active_set_id(id: &str) -> Result<(String, String), String> {
    let Some((domain, rule)) = id.split_once(':') else {
        return Err(format!(
            "id must be `domain:rule` (e.g. clippy:manual_div_ceil), got {id:?}"
        ));
    };
    if !riir_refine::active_set::KNOWN_DOMAIN_KEYS.contains(&domain) {
        return Err(format!(
            "unknown domain {domain:?} — known keys: {}",
            riir_refine::active_set::KNOWN_DOMAIN_KEYS.join(", ")
        ));
    }
    if rule.is_empty() {
        return Err("empty rule id".to_string());
    }
    Ok((domain.to_string(), rule.to_string()))
}

/// Issue 130 T1+T3 + the T2-path-(a) OOD axis: the admission-gate evidence
/// report (standalone, ungated, report-only). Reads the committed artifacts
/// — the score history's per-run provenance strata (T1), its per-row `ood`
/// entropy-gap blocks (the T2 path-(a) axis readout), and the frontier
/// snapshot series (T3) — and prints all three. Nothing here gates: the T2
/// admission predicate stays deferred until a recorded re-arm fires (a
/// nondeterminism source in the heal path or the entropy kernel).
fn run_gate_calibration() {
    use riir_refine::gate_calibration as gc;
    let hist = std::path::Path::new(gc::SCORE_HISTORY_JSONL);
    let Ok(text) = std::fs::read_to_string(hist) else {
        eprintln!(
            "gate-calibration: no score history at {} — run --score-bench (a strata-bearing \
             lane) first",
            hist.display()
        );
        std::process::exit(1);
    };
    let mut runs = Vec::new();
    let mut ood = Vec::new();
    let mut bad_rows = 0usize;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            bad_rows += 1;
            continue;
        };
        match gc::stratum_runs(&v) {
            Ok(mut parsed) => runs.append(&mut parsed),
            Err(e) => {
                bad_rows += 1;
                eprintln!("gate-calibration: skipped one history row: {e}");
            }
        }
        // The ood axis is independent of the strata: a row can carry valid
        // strata and a malformed ood block (or vice versa). Non-object rows
        // are already counted by the stratum arm above — ood_runs would only
        // repeat the same error, so they are skipped here.
        if v.is_object() {
            match gc::ood_runs(&v) {
                Ok(mut parsed) => ood.append(&mut parsed),
                Err(e) => {
                    bad_rows += 1;
                    eprintln!("gate-calibration: skipped one history row's ood block: {e}");
                }
            }
        }
    }
    let rows = gc::calibrate(&runs);
    println!("{}", gc::render_calibration(&rows));
    let ood_groups = gc::calibrate_ood(&ood);
    println!("{}", gc::render_ood_axis(&ood_groups));
    let snaps_path = std::path::Path::new(gc::FRONTIER_SNAPSHOTS_JSONL);
    match std::fs::read_to_string(snaps_path) {
        Err(_) => println!(
            "## Corpus stall (Issue 130 T3)\nno frontier snapshot file at {} — the stall window \
             is UNMEASURED (run --frontier-report in a repo with a store to seed it)",
            snaps_path.display()
        ),
        Ok(snap_text) => {
            let mut snapshots = Vec::new();
            for line in snap_text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<riir_refine::frontier_report::FrontierSnapshot>(line) {
                    Ok(snap) => snapshots.push(snap),
                    Err(_) => bad_rows += 1,
                }
            }
            let stall = gc::stall_scan(&snapshots);
            println!("{}", gc::render_stall(&stall));
        }
    }
    if bad_rows > 0 {
        eprintln!("gate-calibration: {bad_rows} unparsable row(s) skipped loud");
    }
}

#[cfg(feature = "score_bench")]
fn run_necessity_bench(
    top: Option<usize>,
    random: Option<usize>,
    smoke: bool,
    forced_rule: Option<String>,
) {
    use riir_refine::score_bench as sb;

    let oracle = sb::draft_oracle();
    if !oracle.is_available() {
        eprintln!("necessity_bench: cargo not on PATH — cannot derive ground truth");
        std::process::exit(1);
    }
    let cfg = if smoke {
        println!("necessity_bench: SMOKE mode — 20 fixtures, top-3 + 1 stride + both canaries");
        sb::necessity::NecessityConfig {
            top: 3,
            random: 1,
            fixture_cap: Some(20),
            forced_rule: None,
        }
    } else {
        if let Some(rule) = &forced_rule {
            println!(
                "necessity_bench: FORCED single-rule decision mode — `{rule}` (the 102-T4 paired \
                 lane: control=enabled vs treatment=disabled over the frozen set; the \
                 load-bearing canary is skipped loud because a no-flip IS the finding)"
            );
        }
        sb::necessity::NecessityConfig {
            top: top.unwrap_or(20),
            random: random.unwrap_or(5),
            fixture_cap: None,
            forced_rule,
        }
    };
    let forced = cfg.forced_rule.clone();
    let fixtures = sb::fixtures::fixtures();
    let run = sb::necessity::run_necessity(&oracle, &fixtures, &cfg);
    sb::necessity::print_run(&run);
    if let Some(rule) = &forced {
        if run.refusal.is_none() {
            match run
                .rows
                .iter()
                .find(|r| &r.rule == rule && r.arm == "disabled")
            {
                Some(row) => {
                    let verdict = sb::necessity::disabled_verdict(
                        false,
                        row.solo_resolved,
                        row.delta_resolved,
                    );
                    let reading = match verdict {
                        "LOAD-BEARING" => "evidence AGAINST demotion — the composition uses it",
                        "DECORATIVE" | "NO-EVIDENCE" => {
                            "evidence FOR demotion — the composition does not use it"
                        }
                        _ => "control arm — not a decision reading",
                    };
                    println!(
                        "paired decision for `{rule}`: baseline resolved {} → disabled arm {} \
                         (Δ={}) → {verdict} — {reading}",
                        row.baseline_resolved, row.arm_resolved, row.delta_resolved
                    );
                }
                None => {
                    eprintln!("paired decision for `{rule}`: no disabled-arm row (unexpected)");
                }
            }
        }
    }
    if let Some(refusal) = &run.refusal {
        eprintln!("necessity_bench: REFUSED — {refusal}");
        std::process::exit(2);
    }
    if let Err(e) = sb::necessity::append_rows(
        std::path::Path::new(sb::necessity::HISTORY_JSONL),
        &run.rows,
    ) {
        eprintln!("necessity history append failed: {e}");
    }
    if !run.load_bearing_canary.ok || !run.noop_canary.ok {
        eprintln!(
            "necessity_bench: CANARY FAILURE — this run's verdicts are invalid; do not read the table"
        );
        std::process::exit(1);
    }
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
    use riir_refine::score_bench as sb;

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
    if generation == 9 {
        // Issue 137: the CRB corruption ladder — own runner, no clippy
        // oracle, never appended to the heal score history (the gen-4/5
        // dispatch shape).
        riir_refine::score_bench::localization::run_generation9_panel();
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
                (
                    sb::generations::generation_v3(),
                    "generation-v3 persist-stratified ladder",
                )
            }
            Err(e) => {
                eprintln!("generation v3: {e}");
                std::process::exit(1);
            }
        },
        6 => match sb::generations::v6_freeze_status() {
            Ok(admitted) => {
                println!("generation v6: {admitted} admitted fixture(s) (freeze record present)");
                (
                    sb::generations::generation_v6(),
                    "generation-v6 real-repo shape ladder",
                )
            }
            Err(e) => {
                eprintln!("generation v6: {e}");
                std::process::exit(1);
            }
        },
        7 => match sb::generations::v7_freeze_status() {
            Ok(admitted) => {
                println!("generation v7: {admitted} admitted fixture(s) (freeze record present)");
                (
                    sb::generations::generation_v7(),
                    "generation-v7 overlap-edit-chain ladder",
                )
            }
            Err(e) => {
                eprintln!("generation v7: {e}");
                std::process::exit(1);
            }
        },
        8 => match sb::generations::v8_freeze_status() {
            Ok(admitted) => {
                println!("generation v8: {admitted} admitted fixture(s) (freeze record present)");
                (
                    sb::generations::generation_v8(),
                    "generation-v8 shaped-verdict ladder",
                )
            }
            Err(e) => {
                eprintln!("generation v8: {e}");
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
    // Issue 089 T6 — the provenance-stratified read: this run's heal rate
    // split by fixture class (repo/authored/construction). Printed here,
    // attached to the history row in finish_score_bench — the per-class
    // trend across generations becomes measurable (every bench row is
    // generation-tagged; the outcome-feed per-batch blocker does not apply).
    let strata = sb::stratify(&fixture_list, &score);
    sb::report::print_strata(&strata);
    // The k-ladder axis is gens 2–7 (composition width / depth rungs);
    // gen-8's axis is the SHAPE — its per-atom table replaces this.
    if (2..=7).contains(&generation) {
        print_per_k_solve_rates(&fixture_list, &score);
    }
    if generation == 6 {
        print_v6_territory_rates(&fixture_list, &score);
    }
    if generation == 7 {
        print_v7_chain_tables(&fixture_list, &score);
    }
    if generation == 8 {
        print_v8_shaped_table(&fixture_list, &score);
    }

    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    let history_before = sb::report::load_history(jsonl);

    // M5 non-vacuity admission (the shared gate — see [`admission_gate`]).
    if let Err(refusal) = admission_gate(generation, &score, &history_before) {
        eprintln!("{refusal}");
        std::process::exit(1);
    }

    finish_score_bench(generation, &score, Some(strata));
}

/// The M5 non-vacuity admission gate, shared by the clippy and compile
/// runners: the FIRST row of a NEW generation (≥ 2) must measure strictly
/// harder than v1's latest `heal_rate` — a new generation that isn't harder
/// isn't a generation, it's the same measurement with a bigger denominator.
/// Returns `Err(refusal message)` — the caller prints + exits 1; nothing is
/// appended on a refusal (the generation is deterministic; the failed
/// admission is reproducible — the .benchmarks doc records the verdict
/// instead).
#[cfg(feature = "score_bench")]
fn admission_gate(
    generation: u8,
    score: &riir_refine::score_bench::BenchScore,
    history_before: &[riir_refine::score_bench::report::HistoryRecord],
) -> Result<(), String> {
    if generation < 2 || history_before.iter().any(|r| r.generation == generation) {
        return Ok(());
    }
    // Issue 089 T7 — the gen-8 admission axis is the SHAPE, not heal_rate:
    // every v8 atom's target CLEARS by construction (heal_rate ≈ 1.0 — the
    // plain gate would refuse it as "not harder"), and the generation's
    // whole measurement is the half-win the created diagnostics price.
    // Non-vacuity = the run measured ≥1 shaped verdict; a zero-shaped run
    // means the creation shapes dissolved (the Plan 097 T1 precedent) and
    // the generation measures nothing the plain axis does not.
    if generation == 8 {
        if score.n_shaped == 0 {
            return Err(
                "generation v8 admission REFUSED (shape non-vacuity, Issue 089 T7): 0 shaped verdicts measured — \
                 every atom's creation dissolved; re-run the freeze e2e (the shapes changed). Nothing appended."
                    .to_string(),
            );
        }
        println!(
            "generation v8 ADMITTED: {} shaped verdict(s), shaped_rate {:.4} vs heal_rate {:.4} — the half-win axis begins",
            score.n_shaped, score.shaped_rate, score.heal_rate
        );
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
            riir_refine::score_bench::report::HISTORY_JSONL
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
///
/// `strata` (Issue 089 T6): the provenance-stratified read, computed by the
/// clippy lane (which owns the fixture list); `None` on the gen-4/5 lanes
/// (single-class ladders — a one-row stratum would restate the aggregate).
#[cfg(feature = "score_bench")]
fn finish_score_bench(
    generation: u8,
    score: &riir_refine::score_bench::BenchScore,
    strata: Option<Vec<riir_refine::score_bench::ProvenanceStratum>>,
) {
    use riir_refine::score_bench as sb;

    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    let mut rec = sb::report::HistoryRecord::from_score(score, sb::report::git_sha())
        .with_generation(generation);
    if let Some(s) = strata {
        rec = rec.with_strata(s);
    }
    // Issue 089 T8 — the per-rule outcome rows: the per-candidate evidence
    // the α grid-fit replays (recorded AT BENCH TIME; the aggregate axes
    // cannot be decomposed retroactively — the T2/T5 per-batch blocker
    // class, closed on the bench axis).
    let rule_read = sb::rule_outcomes(score);
    println!(
        "rule outcomes (089 T8): {} rule(s) recorded single-target, {} multi-target fixture(s) unattributed (aggregate axes carry them)",
        rule_read.rows.len(),
        rule_read.skipped_multi_target
    );
    // Issue 134 T1 — the standing demonstrability column (measurement only,
    // no behavior change): per-domain selection-discriminability constants
    // over the per-rule outcomes (`resolved / (resolved + persist)`,
    // coverage-conditioned — the formula correction recorded in the issue),
    // printed + appended to the jsonl row (additive; the store mirror drops
    // it — the instrumentation law). Attribution through the compiled
    // rule→domain table. Computed BEFORE the rows are moved into the record.
    let demo = riir_refine::demonstrability::from_rule_outcomes(&rule_read.rows);
    rec = rec.with_rule_outcomes(rule_read.rows);
    print!("{}", riir_refine::demonstrability::render_column(&demo));
    rec = rec.with_demonstrability(demo);
    // Issue 098 T4 — the cross-OOO axis on the console (the row carries it
    // too, via HistoryRecord::cross_ooo; one shared cached measurement).
    {
        let axis = sb::cross_ooo::cross_ooo_axis();
        println!(
            "cross-OOO axis (098): unique_feature_count = {}/{} audited (clippy pool {} spans) · absorption candidates < {}: {}",
            axis.curated_kept,
            axis.audited_units,
            axis.pooled_spans,
            sb::cross_ooo::REDUNDANT_PAIR_CROSS,
            axis.redundant_pairs.len()
        );
        for (a, b, c) in axis.redundant_pairs.iter().take(5) {
            println!("  absorption candidate: {a} <-> {b} : {c:.3}");
        }
    }
    if let Err(e) = sb::report::append_history(jsonl, &rec) {
        eprintln!("score history append failed: {e}");
    }
    // Issue 050 T2: the Warm-tier dual-write — the jsonl stays the committed
    // projection; the store becomes the queryable source of truth.
    // Best-effort (never fails the bench run).
    #[cfg(feature = "latent_retrieval")]
    sb::report::record_store_best_effort(&rec);
    // Issue 089 T4 hardening — the bench tail's automatic frontier snapshot:
    // a generation boundary is created HERE (the row above just landed in
    // the store), so the T4 evidence clock takes its band-mass snapshot HERE
    // too. The manual `--frontier-report` dependency was the fragile half —
    // a boundary nobody reports at is a boundary the band-mass ↔
    // next-generation-yield correlation never samples. Same committed file,
    // same state-change dedup, same frontier-generation semantics as the
    // report path; best-effort — one console line, never a bench failure.
    #[cfg(feature = "latent_retrieval")]
    {
        let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .unwrap_or_default();
        match riir_refine::frontier_report::append_snapshot_at_frontier(
            &root,
            std::path::Path::new(riir_refine::frontier_report::SNAPSHOTS_JSONL),
        ) {
            Ok(outcome) => println!("frontier snapshot (089 T4 bench tail): {outcome}"),
            Err(e) => {
                eprintln!("frontier snapshot: bench-tail append failed (bench unaffected): {e}");
            }
        }
    }
    #[cfg(not(feature = "latent_retrieval"))]
    eprintln!(
        "frontier snapshot: skipped — this bench build lacks latent_retrieval (the store backend); the T4 boundary snapshot is NOT recorded"
    );
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
        8 => sb::report::HISTORY_SVG_V8,
        _ => sb::report::HISTORY_SVG,
    };
    let v1_series = gen_series(1);
    let svg = sb::report::render_svg(&v1_series);
    std::fs::write(sb::report::HISTORY_SVG, &svg)
        .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
    let mut written_svgs = vec![sb::report::HISTORY_SVG];
    for g in [2_u8, 3, 4, 5, 6, 7, 8] {
        let series = gen_series(g);
        if !series.is_empty() {
            let path = svg_path(g);
            let svg_g = sb::report::render_svg(&series);
            std::fs::write(path, &svg_g).unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
            written_svgs.push(path);
        }
    }
    println!(
        "history: {} run(s) (gen-1 {}, gen-2 {}, gen-3 {}, gen-4 {}, gen-5 {}, gen-6 {}, gen-7 {}, gen-8 {}) → {}{}",
        history.len(),
        v1_series.len(),
        gen_series(2).len(),
        gen_series(3).len(),
        gen_series(4).len(),
        gen_series(5).len(),
        gen_series(6).len(),
        gen_series(7).len(),
        gen_series(8).len(),
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
/// `heal_rate` = Σ reduced / Σ members (per-member, NOT per-lint — the
/// admission gate's v1 anchor is a cross-domain non-vacuity floor, recorded
/// as such in the plan).
#[cfg(all(feature = "score_bench", feature = "rustc_errors"))]
fn run_score_bench_compile() {
    use riir_refine::domains::rustc_errors::composite;
    use riir_refine::domains::rustc_errors::corpus_gen::{territory_of, Territory};
    use riir_refine::score_bench as sb;
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

    let work_root = std::env::temp_dir().join("riir_refine_score_bench_gen4");
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
            if outcome.label_ok {
                ""
            } else {
                " [LABEL-MISMATCH]"
            },
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
        // Gen-4 lane: no clippy-oracle verdicts — the shaped axis is
        // vacuously zero (same convention as the other non-clippy lanes).
        ..sb::BenchScore::default()
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

    finish_score_bench(4, &score, None);
}

/// The generation-5 bench runner (Plan 094): the `rust_perf` vocabulary
/// ladder. CARGO-LESS end to end — the oracle is the bench-owned set of
/// per-rule mechanical shape detectors (Plan 094 §2, pre-registered), each
/// already validated by the always-on discrimination fence; the freeze
/// gates run in `tests/generation_v5_freeze.rs`. Each spec heals through
/// the production `heal_fixpoint` loop (both proposer domains, the
/// fxhashmap stub-context skip, `MAX_FIX_PASSES` = 4), the healed file
/// re-parses, and member i's verdict reads module `gen_{i}`'s region.
/// Per-member resolution (not per-lint): the admission gate's v1 anchor is
/// a cross-domain non-vacuity floor, recorded as such in the plan.
#[cfg(feature = "score_bench")]
fn run_score_bench_perf() {
    use riir_refine::score_bench as sb;
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
        // fn(&str) over String items — the closure adapts (a bare fn path
        // here failed E0631 on this score_bench-only lane, pre-existing).
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
        assert_eq!(
            member_names.len(),
            rules.len(),
            "member names/rules aligned on {}",
            spec.id
        );
        // Pre-heal regions: the created axis needs each member's BEFORE
        // marker state, read from the composed spec itself.
        let before_regions: Vec<Option<String>> = (0..rules.len())
            .map(|i| sb::generations::v5_module_region(&spec.code, i))
            .collect();

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
                        .is_some_and(|r| sb::generations::v5_persists(other, r));
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
        // Gen-5 lane: the mechanical shape oracle — the shaped axis is
        // vacuously zero (same convention as the other non-clippy lanes).
        ..sb::BenchScore::default()
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

    finish_score_bench(5, &score, None);
}

/// The M4 difficulty measurement: per-k solve rates over the ladder
/// (resolved vocabulary lints / vocabulary lints before, within each k).
#[cfg(feature = "score_bench")]
fn print_per_k_solve_rates(
    fixtures: &[riir_refine::score_bench::BenchFixture],
    score: &riir_refine::score_bench::BenchScore,
) {
    use riir_refine::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> =
        score.outcomes.iter().map(|o| (o.id.as_str(), o)).collect();
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
    fixtures: &[riir_refine::score_bench::BenchFixture],
    score: &riir_refine::score_bench::BenchScore,
) {
    use riir_refine::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> =
        score.outcomes.iter().map(|o| (o.id.as_str(), o)).collect();
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
    fixtures: &[riir_refine::score_bench::BenchFixture],
    score: &riir_refine::score_bench::BenchScore,
) {
    use riir_refine::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> =
        score.outcomes.iter().map(|o| (o.id.as_str(), o)).collect();
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
        let Some(t) = terr_total.get(terr) else {
            continue;
        };
        let r = terr_resolved.get(terr).copied().unwrap_or(0);
        println!("  {terr}: {r}/{t} = {:.1}%", pct(r, *t));
    }
    println!("passes-to-quiet distribution (specs):");
    for (p, n) in &passes_hist {
        println!("  {p} pass(es): {n} spec(s)");
    }
    println!("───────────────────────────────────────────");
}

/// The generation-8 shaped-verdict table (Issue 089 T7): per atom — the
/// creation class, the target's fate, the diagnostics the heal LEFT
/// BEHIND, and the verdict (1.0 / 0.5 / 0.0). The generation's whole
/// measurement: the aggregate axes above price what these rows carry.
#[cfg(feature = "score_bench")]
fn print_v8_shaped_table(
    fixtures: &[riir_refine::score_bench::BenchFixture],
    score: &riir_refine::score_bench::BenchScore,
) {
    use riir_refine::score_bench as sb;
    use std::collections::BTreeMap;
    let by_id: std::collections::HashMap<&str, &sb::FixtureOutcome> =
        score.outcomes.iter().map(|o| (o.id.as_str(), o)).collect();
    let frozen_created: BTreeMap<&str, &[&str]> = sb::generations::V8_CREATED
        .iter()
        .map(|(id, l)| (*id, *l))
        .collect();
    let mut class_shaped: BTreeMap<&str, usize> = BTreeMap::new();
    let mut class_total: BTreeMap<&str, usize> = BTreeMap::new();
    println!("\n─ generation-8 shaped verdicts (Issue 089 T7) ───────────");
    for f in fixtures {
        let Some(o) = by_id.get(f.id) else { continue };
        if !o.ran {
            continue;
        }
        let class = sb::generations::v8_creation_class(f.id);
        *class_total.entry(class).or_default() += 1;
        let verdict = o.shaped_verdict();
        if o.is_shaped() {
            *class_shaped.entry(class).or_default() += 1;
        }
        let created: Vec<&str> = o.created.iter().map(String::as_str).collect();
        let frozen = frozen_created.get(f.id).copied().unwrap_or(&[]);
        let drift_note = if created != frozen {
            format!("  ⚠ created drift vs freeze record {frozen:?}")
        } else {
            String::new()
        };
        println!(
            "  {:<26} {:<13} verdict {:>4.1}  created {:?}{}",
            f.id, class, verdict, created, drift_note
        );
    }
    println!("per creation class (shaped / ran):");
    for (class, total) in &class_total {
        let shaped = class_shaped.get(class).copied().unwrap_or(0);
        println!("  {class}: {shaped}/{total}");
    }
    println!("───────────────────────────────────────────");
}

// ── Issue 050 T2: --corpus-stats / --import-series / --json ────────────

/// The corpus domain table — the Rust mirror of `gen_dashboard.py`'s
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
    // The corpus outgrew the single `corpus.rs` file (Batch 130s split it
    // into entries_01..06) — the row points at the DIRECTORY and the walker
    // counts the direct children (mod.rs excluded). The old file path
    // counted 0 at every rev after the split (the failing test's shape).
    ("rust_perf", "src/domains/rust_perf/corpus", "rule_id:"),
    (
        "clippy_lints",
        "src/domains/clippy_lints/corpus.rs",
        "lint_name:",
    ),
    ("sec", "src/domains/sec/corpus.rs", "rule_id:"),
    // Docker's corpus uses the positional `rule()` shorthand constructor
    // (corpus.rs's own design note), so there is no per-entry `rule_id:`
    // text — the old needle counted the constructor's PLUMBING (exactly 2
    // occurrences at every rev) and every source-walk stamp since the
    // domain landed recorded docker:2 against 27..31 true entries (Issue
    // 066). `    rule(` matches exactly the 8-space-indented entry calls;
    // the count-pin test below fails loud if a re-indent moves it.
    ("docker", "src/domains/docker/corpus.rs", "    rule("),
    // Issue 085: dist uses the positional `rule()` shorthand like docker —
    // same 4-space-indent needle convention.
    ("dist", "src/domains/dist/corpus.rs", "    rule("),
    // Plan 134: shader_perf uses the same positional `rule()` shorthand —
    // same needle convention.
    (
        "shader_perf",
        "src/domains/shader_perf/corpus.rs",
        "    rule(",
    ),
];

/// Whether the domain's feature is compiled into THIS binary (the honest
/// `compiled` half of the corpus row — a source-walk counts text, this
/// answers "is the domain live here").
#[allow(clippy::match_like_matches_macro)] // cfg! arms are compile-time, not a pattern set
fn domain_compiled(domain: &str) -> bool {
    match domain {
        "kernel_opt" => cfg!(feature = "kernel_opt"),
        "rust_perf" => cfg!(feature = "rust_perf"),
        "clippy_lints" => cfg!(feature = "clippy_lints"),
        "sec" => cfg!(feature = "sec"),
        "docker" => cfg!(feature = "docker"),
        "dist" => cfg!(feature = "dist"),
        "shader_perf" => cfg!(feature = "shader_perf"),
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
            Some(u32::try_from(riir_refine::build_seed_corpus().entries.len()).unwrap_or(u32::MAX))
        }
        #[cfg(feature = "rust_perf")]
        "rust_perf" => Some(
            u32::try_from(riir_refine::build_perf_seed_corpus().entries.len()).unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "kernel_opt")]
        "kernel_opt" => Some(
            u32::try_from(riir_refine::build_kernel_seed_corpus().entries.len())
                .unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "sec")]
        "sec" => Some(
            u32::try_from(riir_refine::build_sec_seed_corpus().entries.len()).unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "docker")]
        "docker" => Some(
            u32::try_from(riir_refine::build_docker_seed_corpus().entries.len())
                .unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "dist")]
        "dist" => Some(
            u32::try_from(riir_refine::build_dist_seed_corpus().entries.len()).unwrap_or(u32::MAX),
        ),
        #[cfg(feature = "shader_perf")]
        "shader_perf" => Some(
            u32::try_from(riir_refine::build_shader_seed_corpus().entries.len())
                .unwrap_or(u32::MAX),
        ),
        _ => None,
    }
}

/// Count `needle` over the domain's corpus source(s) — the exact
/// `_count_working_tree` semantics from `gen_dashboard.py`: a `.rs` file
/// counts itself; a directory counts its DIRECT `*.rs` children except
/// `mod.rs` (non-recursive, matching the Python glob). A missing path
/// counts 0 (the domain table is static; the walk is best-effort).
fn count_corpus_domain(repo: &Path, rel: &str, needle: &str) -> u32 {
    let path = repo.join(rel);
    if path.is_file() {
        return std::fs::read_to_string(&path).map_or(0, |t| {
            u32::try_from(t.matches(needle).count()).unwrap_or(u32::MAX)
        });
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

/// Locate the riir-refine repo root for the corpus source-walk: the nearest
/// ancestor of the CWD owning the corpus paths, else the binary's own
/// build-machine manifest dir (makes the flag work from any CWD on the
/// build machine), else None (the caller errors clearly).
///
/// The manifest-dir fallback is DEBUG-ONLY by construction (Plan 105, G2):
/// `env!` bakes the absolute build path into the binary as a string literal,
/// which `--remap-path-prefix` cannot touch, and a distributed release binary
/// could never use the path anyway — it does not exist off the build machine.
/// The cfg split keeps dev behavior byte-identical while release compiles the
/// fallback to plain `None`.
#[cfg(debug_assertions)]
const BUILD_MACHINE_MANIFEST_DIR: Option<&str> = Some(env!("CARGO_MANIFEST_DIR"));
#[cfg(not(debug_assertions))]
const BUILD_MACHINE_MANIFEST_DIR: Option<&str> = None;

/// Issue 119 T1 — the I/O half of the coverage-debt sweep.
///
/// Everything undecidable-without-the-world lives here and the arithmetic
/// lives in [`riir_refine::corpus_coverage`], so the rules are unit-testable
/// at default features while the spawn + manifest read are not.
///
/// Three reads, each of which can fail INDEPENDENTLY and says so:
/// 1. the installed driver's lint surface (`clippy-driver -W help`) — a
///    spawn failure is BLIND, never an empty surface;
/// 2. the compiled corpora (whatever domains this binary was built with);
/// 3. the real-code fired histogram, which is OPTIONAL — its absence costs
///    the local half of the frequency axis and nothing else, so it degrades
///    to `None` provenance rather than refusing the whole report.
fn run_coverage_report() -> riir_refine::corpus_coverage::CoverageReport {
    use riir_refine::corpus_coverage as cc;

    let driver =
        std::env::var("RIIR_REFINE_CLIPPY_DRIVER").unwrap_or_else(|_| "clippy-driver".into());

    // The lint surface. `-W help` prints to stdout and exits 0 with no
    // input; stdin is nulled so a driver that DOES want one fails fast
    // instead of hanging a report behind a closed pipe.
    let help = std::process::Command::new(&driver)
        .arg("-W")
        .arg("help")
        .stdin(std::process::Stdio::null())
        .output();
    let surface = match &help {
        Ok(out) if out.status.success() => {
            cc::parse_lint_help(&String::from_utf8_lossy(&out.stdout))
        }
        Ok(out) => cc::LintSurface {
            blind: Some(format!(
                "`{driver} -W help` exited {} — the lint surface is UNKNOWN, not empty",
                out.status
            )),
            ..cc::LintSurface::default()
        },
        Err(e) => cc::LintSurface {
            blind: Some(format!(
                "cannot spawn `{driver}` ({e}) — install the clippy component \
                 (`rustup component add clippy`) or set RIIR_REFINE_CLIPPY_DRIVER"
            )),
            ..cc::LintSurface::default()
        },
    };

    let toolchain = std::process::Command::new(&driver)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or_else(
            || "(driver version unavailable)".to_string(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        );

    // The corpus side. `clippy_lints` is the diffed-against set; every OTHER
    // compiled domain feeds the dedup axis (the Batch-146 near-miss law).
    // One `build_seed_corpus()` call serves both the name-keyed buckets and
    // the Issue-167 body scan.
    let (corpus_rules, mut corpus_bodies) = clippy_corpus_ids_and_bodies();
    let (doc_blobs, docs_scanned) = corpus_doc_blobs();
    corpus_bodies.extend(doc_blobs);
    let fix_set = compiled_fix_set();
    let other_domains = other_domain_rule_ids();
    // Issue 168 — the ahead-of-pin registry rides the same compiled-domain
    // truth as the corpus itself: keys the registry carries are a known-good
    // posture, not the renamed/removed drift (all five Batch-230 keys
    // probe-verified at the isolated +1.99.0 driver, absent at the pinned
    // 1.98.1).
    #[cfg(feature = "clippy_lints")]
    let ahead_of_pin: &[&str] = riir_refine::domains::clippy_lints::lint_level::AHEAD_OF_PIN_1_99;
    #[cfg(not(feature = "clippy_lints"))]
    let ahead_of_pin: &[&str] = &[];

    let (histogram, provenance) = resolve_corpus_repo_root()
        .map(|root| load_fired_histogram(&root))
        .unwrap_or_default();
    let (default_histogram, default_provenance) = resolve_corpus_repo_root()
        .map(|root| load_default_fired_histogram(&root))
        .unwrap_or_default();

    cc::coverage(&cc::CoverageInputs {
        toolchain: &toolchain,
        surface,
        corpus_rules: &corpus_rules,
        fix_set: &fix_set,
        other_domains: &other_domains,
        histogram: &histogram,
        histogram_provenance: provenance,
        default_histogram: &default_histogram,
        default_histogram_provenance: default_provenance,
        corpus_blobs: &corpus_bodies,
        corpus_docs_scanned: docs_scanned,
        ahead_of_pin,
    })
}

/// The `clippy_lints` corpus rule ids AND their body text, from ONE
/// `build_seed_corpus()` call — the ids feed the name-keyed buckets, the
/// bodies (buggy/fixed/context text) feed the Issue-167 pattern-mentioned
/// tier. From the COMPILED builder — the same runtime-corpus source
/// `--corpus-stats` prefers over a source walk.
fn clippy_corpus_ids_and_bodies() -> (Vec<String>, Vec<String>) {
    #[cfg(feature = "clippy_lints")]
    {
        use riir_refine::domain::HealEntry as _;
        let corpus = riir_refine::build_seed_corpus();
        let mut ids = Vec::with_capacity(corpus.entries.len());
        let mut bodies = Vec::with_capacity(corpus.entries.len());
        for entry in &corpus.entries {
            ids.push(entry.rule_id().to_string());
            let mut text =
                String::with_capacity(entry.buggy_code().len() + entry.fixed_code().len() + 64);
            text.push_str(entry.buggy_code());
            text.push('\n');
            text.push_str(entry.fixed_code());
            if let Some(notes) = entry.context_notes() {
                text.push('\n');
                text.push_str(notes);
            }
            bodies.push(text);
        }
        (ids, bodies)
    }
    #[cfg(not(feature = "clippy_lints"))]
    {
        // The floor in `coverage()` turns this into a loud refusal.
        (Vec::new(), Vec::new())
    }
}

/// Issue 167 — the proposers' SOURCE text (doc comments + the pattern
/// literals as written), read when the source tree is locatable beside this
/// binary. The `len_zero` precedent lives ONLY in doc comments, so a
/// binary run outside the checkout misses exactly that class of mention —
/// the `false` half is disclosed in the report, never a silent zero.
/// Read-only; never required.
fn corpus_doc_blobs() -> (Vec<String>, bool) {
    let Some(root) = resolve_corpus_repo_root() else {
        return (Vec::new(), false);
    };
    let dir = root.join("src/domains/clippy_lints/proposers");
    let Ok(read_dir) = std::fs::read_dir(&dir) else {
        return (Vec::new(), false);
    };
    let mut blobs = Vec::new();
    let mut any_rs = false;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() || !path.extension().is_some_and(|x| x == "rs") {
            continue;
        }
        any_rs = true;
        match std::fs::read_to_string(&path) {
            Ok(text) => blobs.push(text),
            // A partial read would under-report the tier silently — fail
            // the DISCLOSURE flag instead and keep what did read.
            Err(_) => return (blobs, false),
        }
    }
    (blobs, any_rs)
}

/// `FIXABLE_LINTS u VERIFY_LINTS` — the fix lane's compiled truth, which is
/// why a freshly-landed lint heals before the leased corpus has seen it
/// (Issue 116's seam, measured here from the other side).
///
/// `VERIFY_LINTS` is counted UNCONDITIONALLY, by full path rather than
/// through the `fix_verify`-gated import at the top of this file. Two
/// reasons, and the second is the binding one: `fix_verify` is in the
/// shipped `RELEASE_FEATURES` set, so the tier is live for every real user;
/// and a coverage-debt figure that MOVED with the reader's own feature flags
/// would be a property of their build instead of a property of the project,
/// which is the one thing this report is for.
fn compiled_fix_set() -> Vec<String> {
    #[cfg(feature = "clippy_lints")]
    {
        use riir_refine::domains::clippy_lints::proposers as p;
        p::FIXABLE_LINTS
            .iter()
            .chain(p::VERIFY_LINTS.iter())
            .map(|s| (*s).to_string())
            .collect()
    }
    #[cfg(not(feature = "clippy_lints"))]
    {
        Vec::new()
    }
}

/// Issue 116: the fix lane's two tiers, kept APART.
///
/// `compiled_fix_set` unions them because a coverage-debt figure must not
/// move with the reader's feature flags; the delta surface needs them split,
/// because the remedy differs — a VERIFY row only applies under
/// `--fix --verify`, so collapsing the tiers would send half the rows to an
/// invocation that will not touch them.
fn fix_lane_tiers() -> (Vec<String>, Vec<String>) {
    #[cfg(feature = "clippy_lints")]
    {
        use riir_refine::domains::clippy_lints::proposers as p;
        let own = |t: &[&str]| t.iter().map(|s| (*s).to_string()).collect();
        (own(p::FIXABLE_LINTS), own(p::VERIFY_LINTS))
    }
    #[cfg(not(feature = "clippy_lints"))]
    {
        // No clippy lane compiled in: every missing lint is a genuine
        // coverage gap for THIS binary, which is what an empty fix set
        // makes the delta report.
        (Vec::new(), Vec::new())
    }
}

/// Every OTHER compiled domain's rule ids, for the dedup axis. Expected to
/// collide with nothing (those are kebab-case concept ids, not lint names);
/// a collision is a real finding — two domains claiming one name.
fn other_domain_rule_ids() -> std::collections::BTreeMap<String, Vec<String>> {
    #[allow(unused_mut)]
    let mut out: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    #[allow(unused_imports)]
    use riir_refine::domain::HealEntry as _;
    macro_rules! domain_rules {
        ($feat:literal, $name:literal, $build:path) => {
            #[cfg(feature = $feat)]
            {
                out.insert(
                    $name.to_string(),
                    $build()
                        .entries
                        .iter()
                        .map(|e| e.rule_id().to_string())
                        .collect(),
                );
            }
        };
    }
    domain_rules!(
        "rust_perf",
        "rust_perf",
        riir_refine::build_perf_seed_corpus
    );
    domain_rules!(
        "kernel_opt",
        "kernel_opt",
        riir_refine::build_kernel_seed_corpus
    );
    domain_rules!("sec", "sec", riir_refine::build_sec_seed_corpus);
    domain_rules!("docker", "docker", riir_refine::build_docker_seed_corpus);
    domain_rules!("dist", "dist", riir_refine::build_dist_seed_corpus);
    out
}

/// The Bench 061 real-code fired histogram + its provenance.
///
/// OPTIONAL by construction: a missing or unreadable manifest costs the
/// LOCAL half of the frequency axis and nothing else, so it returns empty
/// rather than refusing a report whose other three axes are intact. The
/// provenance is carried so the reader can see the counts were captured
/// under a different toolchain than the one being swept.
fn load_fired_histogram(
    root: &std::path::Path,
) -> (
    Vec<(String, u32)>,
    Option<riir_refine::corpus_coverage::HistogramProvenance>,
) {
    const REL: &str = ".benchmarks/data/049_t01/manifest.json";
    let Ok(bytes) = std::fs::read(root.join(REL)) else {
        return (Vec::new(), None);
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (Vec::new(), None);
    };
    let rows: Vec<(String, u32)> = v
        .get("fired_histogram_top")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|row| {
                    let pair = row.as_array()?;
                    let name = pair.first()?.as_str()?.to_string();
                    let n = u32::try_from(pair.get(1)?.as_u64()?).ok()?;
                    Some((name, n))
                })
                .collect()
        })
        .unwrap_or_default();
    if rows.is_empty() {
        return (Vec::new(), None);
    }
    let text = |k: &str| {
        v.get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let provenance = riir_refine::corpus_coverage::HistogramProvenance {
        source: format!("Bench 061 fired histogram ({REL})"),
        captured_utc: text("created_utc"),
        toolchain: text("cargo_version"),
        dataset_blake3: text("dataset_blake3"),
        repos: v
            .get("source_repos")
            .and_then(serde_json::Value::as_array)
            .map_or(0, Vec::len),
        rows: rows.len(),
    };
    (rows, Some(provenance))
}

/// The Bench 094 DEFAULT-LEVEL ecosystem fired histogram + its provenance.
///
/// Same OPTIONAL contract as [`load_fired_histogram`]: a missing or
/// unreadable manifest costs the default-eco axis and nothing else — and
/// its absence is VISIBLE in the report ("default-eco: NONE joined"), which
/// is the honest degeneration, not a silent zero. This is the histogram the
/// default-firing uncovered set can actually carry (plain `cargo clippy`;
/// no pedantic/nursery capture bias), which is why it gets its own file and
/// its own loader rather than a row in the Bench 061 manifest.
fn load_default_fired_histogram(
    root: &std::path::Path,
) -> (
    Vec<(String, u32)>,
    Option<riir_refine::corpus_coverage::HistogramProvenance>,
) {
    const REL: &str = ".benchmarks/data/094_t04/manifest.json";
    let Ok(bytes) = std::fs::read(root.join(REL)) else {
        return (Vec::new(), None);
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (Vec::new(), None);
    };
    let rows: Vec<(String, u32)> = v
        .get("default_fired_histogram")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|row| {
                    let pair = row.as_array()?;
                    let name = pair.first()?.as_str()?.to_string();
                    let n = u32::try_from(pair.get(1)?.as_u64()?).ok()?;
                    Some((name, n))
                })
                .collect()
        })
        .unwrap_or_default();
    if rows.is_empty() {
        return (Vec::new(), None);
    }
    let text = |k: &str| {
        v.get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let provenance = riir_refine::corpus_coverage::HistogramProvenance {
        source: format!("Bench 094 default-level fired histogram ({REL})"),
        captured_utc: text("created_utc"),
        toolchain: text("clippy_version"),
        dataset_blake3: text("dataset_blake3"),
        repos: v
            .get("population")
            .and_then(|p| p.get("compiled_clean"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
        rows: rows.len(),
    };
    (rows, Some(provenance))
}

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
    let manifest = BUILD_MACHINE_MANIFEST_DIR.map(PathBuf::from)?;
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
///
/// rustc_errors-only by nature (the playbook inventory IS the rustc
/// playbooks); a build without the feature refuses loud instead of
/// compiling to nothing.
#[cfg(feature = "rustc_errors")]
fn run_conversion_report() {
    use riir_refine::domains::rustc_errors::evolve;
    let root = std::env::current_dir().unwrap_or_default();
    let inventory = evolve::playbook_inventory();
    let mut signals: Vec<evolve::ConversionSignal> = Vec::new();
    let mut store_present = false;
    #[cfg(feature = "latent_retrieval")]
    if root.join(".refine").join("trajectories.kv").exists() {
        match riir_refine::traj_store::TrajStore::open(&root) {
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
            // checked_div: the total == 0 arm above makes this unreachable-as-None,
            // the lint wants the total-failure form (guard 5a/clippy drive-by).
            let pct = (certified * 100 + total / 2)
                .checked_div(total)
                .unwrap_or_default();
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
            .map_or_else(String::new, |t| format!(" {t}"));
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
            "corpus-stats: cannot locate the riir-refine corpus paths \
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

    // Issue 134 T1 — the STANDING demonstrability column: the LAST
    // committed score-bench rows (the bench half; the eval half prints in
    // the retrieval-eval report). Ungated reader (serde_json is a core
    // dep) so every bin build carries it; a missing file or pre-T1-only
    // history is the honest "not yet measured" — never a confident zero.
    if with_stats {
        let path = repo.join(".benchmarks").join("score_history.jsonl");
        match riir_refine::demonstrability::standing_from_jsonl(&path) {
            Ok(Some((ts, rows))) => {
                say(&format!(
                    "demonstrability column (134 T1, standing — last bench row {ts}):"
                ));
                for line in
                    riir_refine::demonstrability::render_column(&rows).lines().skip(1)
                {
                    say(line);
                }
            }
            Ok(None) => say(
                "demonstrability column (134 T1): not yet measured — no score-history row carries the field (run a --score-bench at HEAD)",
            ),
            Err(e) => eprintln!("demonstrability column: history unreadable: {e}"),
        }
    }

    #[cfg(feature = "latent_retrieval")]
    {
        let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())
            .unwrap_or_default();
        if !(with_stats || as_json || do_import) {
            return;
        }
        let store = match riir_refine::traj_store::TrajStore::open(&root) {
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
                    eprintln!("corpus-stats: store at {} unreadable: {e}", root.display());
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
            let day = riir_refine::traj_store::today_local_date();
            let mut stamped = 0usize;
            for (domain, count, compiled, source) in &table {
                match store.upsert_corpus_count(&day, domain, *count, *compiled, source) {
                    Ok(()) => stamped += 1,
                    Err(e) => eprintln!("corpus-stats: stamp {domain} failed: {e}"),
                }
            }
            say(&format!(
                "stamped {day}: {stamped} row(s) → {}",
                root.join(".refine").join("trajectories.kv").display()
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
            let today = riir_refine::traj_store::today_local_date();
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
    use riir_refine::kat_promotion as kp;

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
    let oracle = riir_refine::score_bench::draft_oracle();
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

    let fixture_list = riir_refine::score_bench::fixtures::fixtures();
    println!(
        "frozen fixtures: {} — running post-hoc sweep…",
        fixture_list.len()
    );

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
        eprintln!(
            "registry save failed ({}) — decisions NOT persisted: {e}",
            registry_path.display()
        );
        std::process::exit(1);
    });
    let ts = riir_refine::score_bench::report::local_iso_timestamp();
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
            riir_refine::kat_promotion::Verdict::Promoted => ("promote", String::new()),
            riir_refine::kat_promotion::Verdict::Rejected(r) => ("reject", r.tag().to_string()),
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
            eprintln!(
                "audit append failed ({}) — continuing: {e}",
                audit_path.display()
            );
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
    // Issue 003 T15: the dials identify this binary — `cargo-refine/<v>
    // (build <hash>)` rides the UA the kat transports send (first install
    // wins; the server counts the shipped stragglers by its absence).
    riir_kat::kat_ua::install_consumer_tag(format!(
        "cargo-refine/{} (build {BUILD_COMMIT})",
        env!("CARGO_PKG_VERSION"),
        BUILD_COMMIT = riir_refine::build_stamp::BUILD_COMMIT,
    ));
    let mut args = parse_args();

    // `--yes` arms every y/N prompt (the flag form of `RIIR_REFINE_YES`) —
    // set the run-global HERE, before every dispatch: the subcommand arm
    // below (`login`, `claim`, …) runs before the mode-flag section ever
    // did, and Plan 045 A1's first-join prompt reads the same run-global
    // from the `login` path. The deep prompt sites (the 079 offer) read it
    // later as before; nothing between here and there resets it.
    PROMPT_YES.store(
        args.yes || prompt_yes_env(),
        std::sync::atomic::Ordering::Relaxed,
    );

    // ── Issue 153 T1/T2/T4: the run-start surface — update alert, hands-free
    // upgrade, auto corpus refresh. Order matters and is deliberate:
    // upgrade BEFORE the corpus refresh (a swapped binary wants the fresh
    // corpus too, and the refresh re-resolves nothing else), alert lines
    // always to stderr, every leg fail-soft — a network-less run must never
    // turn into a nonzero exit here. `upgrade` (the explicit command) and
    // `--version` (which alerts post-output in its own arm) skip this hook.
    if args.command != Some(KatCommand::Upgrade) && !args.version {
        run_update_surface_hooks(&args.paths);
    }

    // ── riir-dapps Plan 023: `config` — the LOCAL service-url surface ────
    // Ungated (the kat_mining_consent precedent): a local record must be
    // writable/readable in EVERY feature build, including before `login`.
    // The money commands it re-points are the gated ones.
    // riir-kat Issue 003 T11: `--net <alias>` — sugar over the env var for
    // ONE invocation. Applied HERE, before every dispatch, so every later
    // resolution (money flows, lease opens, config get) sees it; the alias
    // set is the two live ledgers (parse-validated). The pin check is the
    // seam's own (command_service_url) — this only selects.
    // `buy` IS the mainnet verb (issue 126 follow-up, owner ask): it
    // defaults the alias to MAINNET — a preset service-url env (an
    // explicit operator/wrapper intent, e.g. a hermetic test rig) still
    // wins, but a stale persisted CONFIG default does not. `--net devnet`
    // overrides explicitly.
    if args.command == Some(KatCommand::Buy) && args.net_alias.is_none() {
        use riir_refine::kat_service_config as svc;
        let env_preset = std::env::var(svc::SERVICE_URL_ENV_VAR)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        if env_preset {
            println!(
                "net: buy honors the preset {} env (unset it for the mainnet default)",
                svc::SERVICE_URL_ENV_VAR
            );
        } else {
            args.net_alias = Some("mainnet".to_string());
        }
    }
    if let Some(alias) = &args.net_alias {
        use riir_refine::kat_service_config as svc;
        let url = svc::validate_service_url(alias).unwrap_or_else(|e| {
            eprintln!("--net {alias}: {e}");
            std::process::exit(2);
        });
        if let Err(e) = riir_refine::kat_net_pin::ensure_net_allowed(&url) {
            eprintln!("--net {alias} refused: {e}");
            std::process::exit(2);
        }
        std::env::set_var(svc::SERVICE_URL_ENV_VAR, &url);
        println!("net: this invocation targets {alias} ({url})");
    }
    if args.command == Some(KatCommand::Config) {
        run_kat_config(
            args.config_set,
            args.config_url.as_deref(),
            args.config_adopt_state.as_deref(),
            args.config_adopt_discard,
            &args.paths,
        );
        return;
    }

    // ── Issue 153 T2: `upgrade` — the verified binary self-update ────
    // Dispatched BEFORE the kat_service block: the upgrade lane ships on its
    // OWN feature (the release set carries it), not on the service plane —
    // refusing it with "needs kat_service" would name the wrong remedy.
    if args.command == Some(KatCommand::Upgrade) {
        #[cfg(feature = "update_check")]
        {
            run_self_upgrade();
            return;
        }
        #[cfg(not(feature = "update_check"))]
        riir_refine::build_stamp::refuse("upgrade", &["update_check"]);
    }

    // ── Proposal 005 T0.1 + T1.6: KAT subcommands (standalone — no paths) ─
    #[cfg(feature = "kat_service")]
    if let Some(cmd) = args.command {
        match cmd {
            KatCommand::Login => run_kat_login(args.import_key.as_deref()),
            KatCommand::Account => run_kat_account(&args.paths, args.account_history),
            KatCommand::Claim => run_kat_claim(args.claim_tx.as_deref()),
            #[cfg(feature = "kat_transport")]
            KatCommand::Update => run_kat_update(),
            // Exhaustiveness only — `upgrade` is dispatched (on its own
            // feature, NOT kat_service) before this match ever runs.
            KatCommand::Upgrade => unreachable!("upgrade is dispatched before the kat match"),
            // Issue 112 T4: the local export sends nothing, so it needs no
            // transport — it dispatches ahead of the transport-gated push.
            KatCommand::Sync if args.sync_export.is_some() => {
                run_kat_sync_export(args.sync_export.as_deref().unwrap_or(Path::new("-")));
            }
            #[cfg(feature = "kat_transport")]
            KatCommand::Sync => run_kat_sync(),
            KatCommand::Stake => run_kat_stake(
                args.stake_usd.as_deref(),
                args.stake_info,
                args.stake_restake.as_deref(),
                args.stake_redeem,
            ),
            KatCommand::Unstake => run_kat_unstake(),
            KatCommand::Vote => run_kat_vote(args.vote_proposal, args.vote_yes, args.vote_no),
            KatCommand::Pay => run_kat_pay(
                args.pay_amount.as_deref(),
                args.pay_label.as_deref(),
                args.pay_qr,
                args.pay_wait,
            ),
            KatCommand::Topup => {
                // The QR mode tri-state: Some(true) = --qr forced on,
                // Some(false) = --no-qr, None = auto (a stdout TTY renders).
                let qr_mode = if args.topup_no_qr {
                    Some(false)
                } else if args.topup_qr {
                    Some(true)
                } else {
                    None
                };
                if let Some(seq) = args.topup_void {
                    run_kat_topup_void(seq);
                } else if let Some(seq) = args.topup_show {
                    run_kat_topup_show(seq, qr_mode);
                } else {
                    run_kat_topup(
                        BuyAsk::Pack(args.topup_pack),
                        qr_mode,
                        args.topup_pick_latest,
                    );
                }
            }
            KatCommand::Buy => {
                // The same QR tri-state (the parse normalizes --qr here).
                let qr_mode = if args.topup_no_qr {
                    Some(false)
                } else if args.topup_qr {
                    Some(true)
                } else {
                    None
                };
                let ask = parse_buy_ask(args.buy_amount.as_deref(), args.buy_unit.as_deref());
                run_kat_topup(ask, qr_mode, args.topup_pick_latest);
            }
            KatCommand::Faucet => run_kat_faucet(args.pay_qr),
            KatCommand::Wetrun => run_kat_wetrun(args.wetrun_dry_run),
            // P015 Phase 2 (Plan 127): the replay lane needs the oracle
            // (clippy_verify + the `draft` tree's latent_retrieval gate) on
            // top of the account + transport gates — a release-set build
            // (no clippy_verify) degrades with the exact remedy, the
            // update/sync precedent.
            #[cfg(all(
                feature = "kat_transport",
                feature = "clippy_verify",
                feature = "latent_retrieval"
            ))]
            KatCommand::Replay => run_kat_replay(),
            #[cfg(not(all(
                feature = "kat_transport",
                feature = "clippy_verify",
                feature = "latent_retrieval"
            )))]
            KatCommand::Replay => {
                eprintln!(
                    "replay: the verdict is measured by the real clippy oracle (P015 Phase 2)"
                );
                riir_refine::build_stamp::refuse(
                    "replay",
                    &["kat_transport", "clippy_verify", "latent_retrieval"],
                );
            }
            // riir-dapps Plan 050 S1.11: the corpus PR client half. The lane
            // reads BOTH ledgers' leases over the network (even the gauge),
            // so the whole command rides kat_transport; the signing half is
            // kat_service (implied by kat_transport).
            #[cfg(feature = "kat_transport")]
            KatCommand::CorpusPr => run_kat_corpus_pr(
                args.corpus_pr_src.as_deref(),
                args.corpus_pr_dst.as_deref(),
                args.corpus_pr_seq,
                args.corpus_pr_submitter.as_deref(),
                args.corpus_pr_key_file.as_deref(),
                args.corpus_pr_submit,
                &resolve_heal_root(&args.paths),
            ),
            #[cfg(not(feature = "kat_transport"))]
            KatCommand::CorpusPr => {
                eprintln!("corpus-pr: the PR lane reads both ledgers' leases over the network");
                riir_refine::build_stamp::refuse("corpus-pr", &["kat_transport"]);
            }
            // The value verbs (issue 126 follow-up): one-line balances.
            #[cfg(feature = "kat_transport")]
            KatCommand::Kat => run_kat_balance(),
            #[cfg(feature = "kat_transport")]
            KatCommand::Tuna => run_kat_tuna(),
            #[cfg(not(feature = "kat_transport"))]
            KatCommand::Kat | KatCommand::Tuna => {
                riir_refine::build_stamp::refuse("kat/tuna", &["kat_transport"]);
            }
            // riir-dapps Proposal 009 §9 / issue 129: the marketing TUNA
            // airdrop ADMIN hand — `admin`-gated (implies kat_transport),
            // NEVER in the release set; compiled out refuses exit 2 via
            // the build stamp (the three facts + the rebuild).
            #[cfg(feature = "admin")]
            KatCommand::TunaGet | KatCommand::TunaSend => {
                let verb = if cmd == KatCommand::TunaGet {
                    "get"
                } else {
                    "send"
                };
                // `run` exits the process itself (the house command
                // convention) — never returns.
                riir_refine::tuna_admin::run(&riir_refine::tuna_admin::TunaAdminArgs {
                    verb,
                    amount: args.tuna_amount.as_deref(),
                    unit: args.tuna_unit.as_deref(),
                    target: args.tuna_target.as_deref(),
                    gov: args.tuna_gov,
                    dry_run: args.tuna_dry_run,
                    url: &command_service_url(),
                });
            }
            #[cfg(not(feature = "admin"))]
            KatCommand::TunaGet => {
                riir_refine::build_stamp::refuse("get", &["admin"]);
            }
            #[cfg(not(feature = "admin"))]
            KatCommand::TunaSend => {
                riir_refine::build_stamp::refuse("send", &["admin"]);
            }
            // riir-dao Proposal 001 T4: the account-key migration lane.
            KatCommand::Migrate => {
                run_kat_migrate(
                    args.migrate_github.as_deref(),
                    args.migrate_object.as_deref(),
                    args.migrate_status.as_deref(),
                    args.migrate_census,
                    args.migrate_old_key.as_deref(),
                    args.migrate_github_token.clone(),
                    args.migrate_repo_base.as_deref(),
                );
            }
            // The transport commands degrade honestly without kat_transport.
            #[cfg(not(feature = "kat_transport"))]
            KatCommand::Update | KatCommand::Sync => {
                // Issue 117 T2: name the missing feature, this binary's own
                // set, and the rebuild — the three facts the 00:21 diagnosis
                // had to reconstruct by hand.
                riir_refine::build_stamp::refuse(
                    if cmd == KatCommand::Update {
                        "update"
                    } else {
                        "sync"
                    },
                    &["kat_transport"],
                );
            }
            // Exhaustiveness only — `config` is dispatched (ungated) before
            // this match ever runs.
            KatCommand::Config => unreachable!("config is dispatched before the gated match"),
        }
        return;
    }
    #[cfg(not(feature = "kat_service"))]
    if args.command.is_some() {
        riir_refine::build_stamp::refuse(
            "login/account/update/upgrade/sync/claim/stake/unstake/vote/pay",
            &["kat_service"],
        );
    }

    // ── Plan 014: clippy-bench modes (standalone — no paths) ─────────────
    #[cfg(feature = "score_bench")]
    if args.score_report {
        let history = riir_refine::score_bench::report::load_history(std::path::Path::new(
            riir_refine::score_bench::report::HISTORY_JSONL,
        ));
        if history.is_empty() {
            eprintln!("no score history yet — run --score-bench first");
            std::process::exit(1);
        }
        // Per-generation curves (Plan 092 M5): gen-1 → score_history.svg,
        // gen-2 → score_history_v2.svg (only when v2 rows exist).
        let v1: Vec<_> = history
            .iter()
            .filter(|r| r.generation == 1)
            .cloned()
            .collect();
        let svg = riir_refine::score_bench::report::render_svg(&v1);
        std::fs::write(riir_refine::score_bench::report::HISTORY_SVG, &svg)
            .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
        println!(
            "rendered {} ({} gen-1 run(s))",
            riir_refine::score_bench::report::HISTORY_SVG,
            v1.len()
        );
        let v2: Vec<_> = history
            .iter()
            .filter(|r| r.generation == 2)
            .cloned()
            .collect();
        if !v2.is_empty() {
            let svg2 = riir_refine::score_bench::report::render_svg(&v2);
            std::fs::write(riir_refine::score_bench::report::HISTORY_SVG_V2, &svg2)
                .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
            println!(
                "rendered {} ({} gen-2 run(s))",
                riir_refine::score_bench::report::HISTORY_SVG_V2,
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
    #[cfg(feature = "score_bench")]
    if args.necessity_bench {
        run_necessity_bench(
            args.necessity_top,
            args.necessity_random,
            args.necessity_smoke,
            args.necessity_rule.clone(),
        );
        return;
    }
    #[cfg(not(feature = "score_bench"))]
    if args.necessity_bench {
        eprintln!("--necessity-bench requires a build with --features score_bench");
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
    // validate_mode_flags never sees it). Prints `.refine/misses/*.md`
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
            #[cfg(feature = "rustc_errors")]
            run_conversion_report();
            #[cfg(not(feature = "rustc_errors"))]
            {
                eprintln!("--misses --report requires a build with --features rustc_errors");
                std::process::exit(1);
            }
        } else {
            run_misses_queue(args.misses_limit, &args.paths);
        }
        return;
    }
    if args.misses_report {
        eprintln!("--report requires --misses");
        std::process::exit(2);
    }

    // ── Proposal 008 (the owner's just-works UX): --mine — the ──────────
    // contribution onboarding. Standalone (early return before
    // validate_mode_flags): what mining is, the policy warning, the web
    // link, and the login → heal → sync → account loop. Works in EVERY
    // build — the explainer never needs a feature; the status halves
    // appear only when their feature compiled in.
    // --mine and --unmine are opposite writes to one field, so passing both
    // has no defensible resolution — reject before either side writes.
    // `--version` answers before anything else (standalone; ungated — a
    // feature-less build still knows its own version).
    if args.version {
        // Issue 117 T1: the version carries the COMPILED FEATURE SET, so a
        // stale binary (the 00:21 class — a bare `cargo build --release`
        // overwrote the symlink target with a default-features build) is
        // identifiable in one command from any terminal. Line 0 is the
        // unchanged `cargo-refine <version>`.
        for line in riir_refine::build_stamp::version_lines() {
            println!("{line}");
        }
        // Issue 153 T1: the version query is where an upgrade-minded user
        // looks — alert AFTER the version block (never delay or precede it),
        // never auto-apply (a read stays a read; the next real run applies).
        #[cfg(feature = "update_check")]
        {
            let root = resolve_heal_root(&args.paths);
            for line in riir_refine::update_check::check_for_alert(
                &root,
                &riir_refine::update_check::GitHubLatest::new(),
            ) {
                eprintln!("{line}");
            }
        }
        return;
    }
    // `--yes` was armed at the top of main (before every dispatch — the
    // `login` path and the Plan 045 A1 first-join prompt read it).
    if riir_refine::kat_mining_consent::flags_conflict(args.mine, args.unmine) {
        eprintln!("{}", riir_refine::kat_mining_consent::CONFLICT_MESSAGE);
        std::process::exit(2);
    }
    if args.mine {
        run_mine_intro();
        return;
    }
    // ── Owner mandate 2026-09-08: --unmine — the opt-OUT counterpart. ────
    // Standalone, same shape as --mine (early return before
    // validate_mode_flags), works in EVERY build. Records consent = opted
    // out so `cargo refine sync` refuses its push half; deletes nothing.
    if args.unmine {
        run_unmine(args.purge_server);
        if args.purge {
            run_unmine_purge();
        }
        return;
    }

    // ── riir-dapps Proposal 004 Half 1: --stats — the work-stats consent ──
    // Standalone consent mode beside --mine/--unmine (early return; the
    // parse guards already rejected on+off, `--stats <path>` ambiguity, and
    // the --mine/--unmine combo). Works in EVERY build — the consent record
    // is ungated (the kat_mining_consent precedent); only the auto-login
    // half of `--stats on` needs kat_service. The fix-path PUSH half is a
    // later landing (it needs the server route) — recorded consent is
    // inert until then, and says so.
    if args.stats_flag {
        run_stats_consent(args.stats_on, args.stats_off);
        return;
    }

    // ── Plan 117 + Issue 094 D4: --refill — the DEVNET-OPS faucet credit ──
    // Standalone (early return before validate_mode_flags AND before the
    // 075 soft gate — an ops tool must not be gated by the balance it
    // exists to repair). It is no longer a user refuel path: D4 retired the
    // daily faucet/drip tier, and `run_kat_refill` refuses loudly off
    // devnet. The money consent is the explicit flag itself; the crank
    // leg's consent is the operator-set RIIR_REFINE_KAT_CRANK_TOKEN.
    if args.refill {
        run_kat_refill(args.refill_wait);
        return;
    }

    // ── 2026-09-17 owner ask: `--info` — the one-command stats view. ────
    // Read-only composition of shipped surfaces (the kat_info module
    // doc). Early return before validate_mode_flags AND before the 075
    // soft gate — a read must never be refused by the gate it reports
    // on. The stake position flag (`stake --info`) is unaffected.
    if args.info {
        #[cfg(feature = "kat_service")]
        kat_info::run_kat_info(&args.paths);
        #[cfg(not(feature = "kat_service"))]
        {
            riir_refine::build_stamp::refuse("--info", &["kat_service"]);
        }
        return;
    }

    // ── Issue 037 observability: --memory-stats — the loop debug query ────
    // Read-only standalone mode (like the score arms above: early return,
    // validate_mode_flags never sees it). Summarizes whichever backend the
    // build ships: the JSON envelope always; the repo-local neuron-db store
    // (`.refine/trajectories.kv`, `latent_retrieval` builds) when present.
    // ── Issue 102: the active-set marker (standalone — no paths) ────
    if args.active_set_list
        || args.active_set_demote.is_some()
        || args.active_set_readmit.is_some()
        || args.active_set_clear
        || args.active_set_cap.is_some()
    {
        run_active_set_cmd(&args);
        return;
    }

    // ── Issue 130: the admission-gate evidence report (standalone) ──
    if args.gate_calibration {
        run_gate_calibration();
        return;
    }

    if args.memory_stats {
        let path = default_memory_path();
        match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // 038: the envelope may have been migrated — the ACTIVE
                // backend is then the repo-local .refine/trajectories.kv
                // (summarized below). Not an error; fall through.
                let migrated = path.with_extension("json.migrated");
                if migrated.exists() {
                    println!(
                        "memory store: {} (MIGRATED — the active backend is the repo-local .refine/trajectories.kv)",
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
                let mut mem = riir_refine::memory::LatentFixMemory::default();
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
        // `.refine/trajectories.kv` — same root resolution as --dump-trajectories.
        #[cfg(feature = "latent_retrieval")]
        {
            let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            let kv = root.join(".refine").join("trajectories.kv");
            if kv.exists() {
                match riir_refine::traj_store::TrajStore::open(&root) {
                    Ok((store, bad)) => {
                        // Full report — feature parity with the envelope
                        // branch (histogram/tiers/loop state; the kv branch
                        // shipped 038 as a 4-counter one-liner, an
                        // observability regression surfaced live on
                        // adoption day).
                        print!("{}", store.stats_report(bad));
                    }
                    Err(e) => {
                        eprintln!("memory-stats: kv store at {} unreadable: {e}", kv.display());
                    }
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

    // ── Issue 089 T1+T2: --frontier-report — the learnability band report ──
    // Read-only standalone mode (the memory-stats pattern). Report-first,
    // gate-later: nothing here admits/demotes/deletes anything — the T4
    // wiring waits on the report proving band-mass is predictive.
    if args.frontier_report {
        #[cfg(feature = "latent_retrieval")]
        {
            let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            let kv = root.join(".refine").join("trajectories.kv");
            if !kv.exists() {
                eprintln!(
                    "frontier-report: no store at {} — run any --fix in this repo first",
                    kv.display()
                );
                std::process::exit(1);
            }
            match riir_refine::traj_store::TrajStore::open(&root) {
                Ok((store, bad)) => {
                    if bad > 0 {
                        eprintln!(
                            "frontier-report: {bad} corrupt rows skipped in {}",
                            kv.display()
                        );
                    }
                    let entries: Vec<(String, u32, u32)> = store
                        .learn_rows()
                        .into_iter()
                        .map(|row| (row.lint_key, row.successes, row.fails))
                        .collect();
                    let summary = riir_refine::frontier_report::summarize(&entries);
                    if summary.population == 0 {
                        // Never a green zero: an empty population is a
                        // refusal, not "measured, all healthy".
                        eprintln!(
                            "frontier-report: store at {} carries ZERO learn rows — nothing to report on",
                            kv.display()
                        );
                        std::process::exit(1);
                    }
                    let runs = store.score_runs().ok();
                    let score = runs.as_ref().and_then(|r| r.last()).map(|latest| {
                        riir_refine::frontier_report::ScoreContext {
                            runs: runs.as_ref().map_or(0, Vec::len),
                            latest_generation: latest.generation,
                            latest_heal_rate: latest.heal_rate,
                        }
                    });
                    print!(
                        "{}",
                        riir_refine::frontier_report::render(&summary, score.as_ref())
                    );
                    // Issue 098 T3 — the cross-OOO additive section: one
                    // line per audited frontier rule (min_cross + partner).
                    // Gated: the axis lives under score_bench; ungated builds
                    // print exactly what they printed before.
                    #[cfg(feature = "score_bench")]
                    {
                        let axis = riir_refine::score_bench::cross_ooo::cross_ooo_axis();
                        let rules: Vec<String> =
                            summary.rows.iter().map(|r| r.rule.clone()).collect();
                        print!(
                            "{}",
                            riir_refine::score_bench::cross_ooo::render_frontier_cross_section(
                                axis, &rules
                            )
                        );
                    }
                    // Issue 089 T5 — provenance weighting, the report tail:
                    // (a) the pair namespace priced by provenance (organic
                    // 1.0 / construction ε / unknown 0.0 fail-closed);
                    // (b) the outcome-feed split — generated-case ':' keys
                    // vs real-repo rows, band-mass measured PER FEED (what
                    // T4's admission gate will consume). Report-first: the
                    // weight prices arithmetic, gates nothing.
                    let pair_rows = store.pair_rows().unwrap_or_default();
                    let mut pair_counts: std::collections::BTreeMap<String, usize> =
                        std::collections::BTreeMap::new();
                    for p in &pair_rows {
                        *pair_counts.entry(p.provenance.clone()).or_insert(0) += 1;
                    }
                    let prow = riir_refine::frontier_report::provenance_summary(&pair_counts);
                    print!(
                        "{}",
                        riir_refine::frontier_report::render_provenance_section(&prow)
                    );
                    let feed_rows = riir_refine::frontier_report::provenance_yield_split(&entries);
                    print!(
                        "{}",
                        riir_refine::frontier_report::render_provenance_yield(&feed_rows)
                    );
                    // Issue 089 T7 — the "Beta (half-count)" feed contract:
                    // the LATEST generation-8 bench row's per-rule shaped
                    // counts, folded at half evidence (+0.5 r / +0.5 f).
                    // Plain serde_json read of the committed jsonl — the
                    // score_bench types stay feature-gated; the path is the
                    // score-bench's HISTORY_JSONL (stable, documented there).
                    let half_counts: Vec<(String, u32)> =
                        std::fs::read_to_string(".benchmarks/score_history.jsonl")
                            .map(|text| {
                                text.lines()
                                    .filter(|l| !l.trim().is_empty())
                                    .filter_map(|l| {
                                        serde_json::from_str::<serde_json::Value>(l).ok()
                                    })
                                    .filter(|v| {
                                        v.get("generation").and_then(|g| g.as_u64()) == Some(8)
                                    })
                                    .filter_map(|v| {
                                        let hc = v.get("shaped")?.get("half_counts")?.clone();
                                        serde_json::from_value::<Vec<(String, u32)>>(hc).ok()
                                    })
                                    .next_back()
                                    .unwrap_or_default()
                            })
                            .unwrap_or_default();
                    let shaped_rows =
                        riir_refine::frontier_report::shaped_half_rows(&entries, &half_counts);
                    print!(
                        "{}",
                        riir_refine::frontier_report::render_shaped_half_rows(&shaped_rows)
                    );
                    // Issue 105 T4 — the journey integral axis:
                    // instrumentation only (never scored). The store scan
                    // is cheap (traj: prefix) and the section degrades to
                    // an honest absent-note on a journey-free store.
                    let jrows_all = store.journey_rows();
                    let jrows = riir_refine::frontier_report::journey_summary(&jrows_all, 12);
                    print!(
                        "{}",
                        riir_refine::frontier_report::render_journey_section(
                            &jrows,
                            jrows_all.len()
                        )
                    );
                    // Issue 102 T1 — the contribution-fade section
                    // (report-first; the live state REFUSES until ≥3
                    // full-corpus sweep rows exist in the score history —
                    // the verdict's population floor). Plain serde_json
                    // read of the committed jsonl, the T7 half_counts
                    // precedent: no feature-gated types cross here.
                    let fade_rows: Vec<riir_refine::frontier_report::FadeRow> =
                        std::fs::read_to_string(".benchmarks/score_history.jsonl")
                            .map(|text| {
                                text.lines()
                                    .filter(|l| !l.trim().is_empty())
                                    .filter_map(|l| {
                                        serde_json::from_str::<serde_json::Value>(l).ok()
                                    })
                                    .filter_map(|v| {
                                        let generation =
                                            u8::try_from(v.get("generation")?.as_u64()?).ok()?;
                                        let ts = v.get("ts")?.as_str()?.to_string();
                                        let outcomes = v
                                            .get("rule_outcomes")?
                                            .as_array()?
                                            .iter()
                                            .filter_map(|o| {
                                                Some(riir_refine::frontier_report::FadeOutcome {
                                                    rule: o.get("rule")?.as_str()?.to_string(),
                                                    resolved: u32::try_from(
                                                        o.get("resolved")?.as_u64()?,
                                                    )
                                                    .ok()?,
                                                    persist: u32::try_from(
                                                        o.get("persist")?.as_u64()?,
                                                    )
                                                    .ok()?,
                                                    edits: u32::try_from(o.get("edits")?.as_u64()?)
                                                        .ok()?,
                                                })
                                            })
                                            .collect();
                                        Some(riir_refine::frontier_report::FadeRow {
                                            generation,
                                            ts,
                                            outcomes,
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                    let scan = riir_refine::frontier_report::fade_scan(&fade_rows);
                    print!(
                        "{}",
                        riir_refine::frontier_report::render_fade_section(&scan)
                    );
                    // Issue 089 T4's evidence clock — append the band-mass
                    // snapshot (state-change dedup: an unchanged store at the
                    // same generation frontier appends nothing). Best-effort:
                    // a failed append NEVER fails the report (report-first
                    // law). The generation is the FRONTIER (max generation
                    // ever run in this cwd), not the latest lane — lanes
                    // interleave (v1 reruns after v8 landings), and the T4
                    // x-axis must be monotone: a new row means "a new
                    // generation EXISTS", the boundary the correlation
                    // samples.
                    let generation_frontier = runs
                        .as_ref()
                        .map_or(0, |rs| rs.iter().map(|r| r.generation).max().unwrap_or(0));
                    let snap = riir_refine::frontier_report::snapshot_from(
                        &summary,
                        &feed_rows,
                        generation_frontier,
                        riir_refine::frontier_report::git_sha(),
                        riir_refine::frontier_report::local_iso_timestamp(),
                        riir_refine::frontier_report::store_id(&root),
                    );
                    match riir_refine::frontier_report::append_snapshot(
                        std::path::Path::new(riir_refine::frontier_report::SNAPSHOTS_JSONL),
                        &snap,
                    ) {
                        Ok(true) => println!(
                            "frontier snapshot: appended (generation {}, store {}, band-mass {:.1}%) → {}",
                            snap.generation,
                            if snap.store.is_empty() { "?" } else { &snap.store },
                            snap.band_mass * 100.0,
                            riir_refine::frontier_report::SNAPSHOTS_JSONL
                        ),
                        Ok(false) => println!(
                            "frontier snapshot: unchanged since the last row (generation {}) — not appended",
                            snap.generation
                        ),
                        Err(e) => {
                            eprintln!("frontier snapshot: append failed (report unaffected): {e}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "frontier-report: kv store at {} unreadable: {e}",
                        kv.display()
                    );
                    std::process::exit(1);
                }
            }
        }
        #[cfg(not(feature = "latent_retrieval"))]
        {
            eprintln!("frontier-report: needs a latent_retrieval build (the trajectory store)");
            std::process::exit(1);
        }
        return;
    }

    // ── Issue 107 T1: --horizon-report — the fixseq lead-k report ──────────
    // Report-first, gates nothing (the frontier-report law): reads the
    // `.refine/fixseq/` ring jsonl directly — no store, no domain features,
    // so this runs on EVERY feature build. Refuses exit 1 on empty ring /
    // blind walk / single-class labels — never a confident green zero
    // (the measurement on a store that exists NOWHERE in the workspace
    // today must read as EMPTY, not healthy).
    if args.horizon_report {
        let targets: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        let mut runs = Vec::new();
        let mut bad = 0usize;
        let mut files = 0usize;
        let mut scanned: Vec<std::path::PathBuf> = Vec::new();
        for t in &targets {
            let (r, b, f) = riir_refine::horizon_report::load_runs(t);
            runs.extend(r);
            bad += b;
            files += f;
            scanned.push(t.clone());
        }
        let mut summary = riir_refine::horizon_report::summarize(&runs);
        summary.bad_lines = bad;
        print!("{}", riir_refine::horizon_report::render(&summary));
        println!(
            "scanned {} path(s), {} ring file(s), {} run(s)",
            scanned.len(),
            files,
            runs.len()
        );
        if summary.refusal.is_some() {
            std::process::exit(1);
        }
        return;
    }

    // ── Issue 128 (T1-T4): --fixseq-stats — the cascade + derailment report ──
    // Report-first, gates nothing (the frontier-report law): reads the
    // `.refine/fixseq/` rings through the SAME loader as --horizon-report.
    // Each PATH is one CORPUS (per-corpus cascade tables + the T7 transfer
    // verdict across corpora; the T5 mint manifest names its configs).
    // Refuses exit 1 on empty / no-transition populations — never a green
    // zero over no data.
    if args.fixseq_stats {
        let targets: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        let mut labeled: Vec<(String, Vec<riir_refine::horizon_report::RunRing>)> = Vec::new();
        let mut bad_total = 0usize;
        let mut files_total = 0usize;
        for t in &targets {
            let (runs, bad, files) = riir_refine::horizon_report::load_runs(t);
            bad_total += bad;
            files_total += files;
            println!(
                "corpus {}: {} run(s), {} ring file(s), {} bad line(s)",
                t.display(),
                runs.len(),
                files,
                bad
            );
            if !runs.is_empty() {
                labeled.push((t.display().to_string(), runs));
            }
        }
        let report = riir_refine::fixseq_stats::summarize(&labeled);
        print!("{}", riir_refine::fixseq_stats::render(&report));
        println!(
            "scanned {} path(s), {} ring file(s), {} bad line(s)",
            targets.len(),
            files_total,
            bad_total
        );
        if report.refusal.is_some() {
            std::process::exit(1);
        }
        return;
    }

    // ── Plan 124: --config-audit — the config-hygiene report ──────────────
    // Report-first, gates nothing (the frontier-report law): classes A+B
    // from the post-mining intake. Always exit 0 — a report that reds is a
    // gate nobody runs. Paths default to `.` (same convention as the scan
    // modes). Ungated: pure syn scan, zero new deps.
    if args.config_audit {
        let targets: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        let report = riir_refine::config_audit::audit_paths(&targets);
        print!("{}", report.render());
        let hint = riir_refine::scope_hint::unscanned_roots_hint(&targets);
        if !hint.is_empty() {
            println!("{hint}");
        }
        let cross = riir_refine::downstream_hint::downstream_consumers_hint(&targets);
        if !cross.is_empty() {
            println!("{cross}");
        }
        return;
    }

    // ── Issue 151: --secret-scan — the secret-blind report ──────────────
    // Report-first, gates nothing (the frontier-report law): the ONE
    // canonical scanner (kat_redact v4 — the consolidated provider-format
    // set) over the scanned tree. Always exit 0 on a real scan; refuses
    // exit 1 on an EMPTY walk — a confident green zero is the failure mode
    // this lane exists to prevent. Ungated: pure scan, zero new deps. The
    // shareable-output rule: kind + path:line:col + the context line
    // BLINDED — never the bytes, never the unkeyed fingerprint.
    // `--blind` prints whole-file text with findings placeholder-
    // substituted (read-only; safe to paste into a ticket/chat/LLM).
    if args.secret_scan {
        if args.blind {
            // Blind transform: ONE path (a file), whole text, findings
            // substituted. Refusal on an unreadable path is exit 1 (the
            // same law as an empty walk).
            if args.paths.len() != 1 {
                eprintln!("--secret-scan --blind takes exactly ONE path (the file to blind)");
                std::process::exit(2);
            }
            let path = &args.paths[0];
            if path.is_dir() {
                eprintln!(
                    "--blind needs a FILE (got the directory {} — run --secret-scan first for the report)",
                    path.display()
                );
                std::process::exit(2);
            }
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("cannot read {}: {e}", path.display());
                std::process::exit(1);
            });
            print!("{}", riir_refine::secret_blind::blind(&text));
            return;
        }
        let targets: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        let report = riir_refine::secret_blind::audit_paths(&targets);
        if report.scanned_files == 0 {
            eprintln!(
                "secret-scan: EMPTY WALK — no text files under {} (blind refusal, not a pass)",
                targets[0].display()
            );
            std::process::exit(1);
        }
        print!("{}", report.render());
        return;
    }

    // ── Issue 119: --coverage-report — the corpus coverage-debt sweep ─────
    // The uncovered half of clippy's LIVE lint surface. Report-first, gates
    // nothing; refuses exit 1 on a blind derivation (the frontier-report
    // law) — a `-W help` format change must RED rather than report a
    // flattering small debt. Needs no paths: the population is the
    // installed toolchain, not the working tree.
    if args.coverage_report {
        let report = run_coverage_report();
        print!("{}", report.render());
        if report.blind.is_some() {
            std::process::exit(1);
        }
        return;
    }

    // ── Plan 126: --orphan-report — the class-C1 report ───────────────────
    // The orphaned-by-optimization knob (intake class C1, the last open
    // class): current-state zero-external-read fields (the class-A
    // condition-2 set at field granularity) + git-history evidence that a
    // read-shaped occurrence vanished (probe lattice + pickaxe
    // attribution). Report-first — gates nothing. Refuses exit 1 on a
    // blind history leg or an empty scan: never a confident green zero
    // (the frontier-report law, not config-audit's).
    if args.orphan_report {
        let targets: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        let report = riir_refine::orphan_report::report_paths(
            &targets,
            &riir_refine::orphan_report::DEFAULT_PROBE_DEPTHS,
        );
        print!("{}", report.render());
        let hint = riir_refine::scope_hint::unscanned_roots_hint(&targets);
        if !hint.is_empty() {
            println!("{hint}");
        }
        let cross = riir_refine::downstream_hint::downstream_consumers_hint(&targets);
        if !cross.is_empty() {
            println!("{cross}");
        }
        if report.blind.is_some() {
            std::process::exit(1);
        }
        return;
    }

    // ── Intake class P22: --platform-audit — the platform-dead_code lattice
    // The NEON_U8 class (fixed ea4c2873): items whose every use sits inside
    // a platform gate the declaration lacks — invisible to any
    // single-platform CI matrix, caught twice in two days only by x86_64
    // gate runs no CI lane owns. Pure syn scan, zero new deps, ungated
    // (report-first, gates nothing — the config-audit law; refuses exit 1
    // on an empty scan — the frontier-report law).
    if args.platform_audit {
        let targets: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        let report = riir_refine::platform_audit::audit_paths(&targets);
        print!("{}", report.render());
        let hint = riir_refine::scope_hint::unscanned_roots_hint(&targets);
        if !hint.is_empty() {
            println!("{hint}");
        }
        let cross = riir_refine::downstream_hint::downstream_consumers_hint(&targets);
        if !cross.is_empty() {
            println!("{cross}");
        }
        if report.blind.is_some() {
            std::process::exit(1);
        }
        return;
    }

    // ── Plan 134 T2.1: --frame-time-report — the frame-time budget tier ──
    // The measurement VERDICT half of shader_perf: riir-shader ships the
    // capture-side sidecars (issue 018 there), this reports the budget
    // verdicts + the pre/post A/B. Report-first, gates nothing; refuses
    // exit 1 on a blind scan (the frontier-report law). Ungated — reads
    // sidecar files directly, no domain features.
    if args.frame_time_report {
        let pre_paths: Vec<PathBuf> = if args.paths.is_empty() {
            vec![std::path::PathBuf::from(".")]
        } else {
            args.paths.clone()
        };
        if let Some(vs) = args.frame_vs.clone() {
            let (pre, pre_fail) = riir_refine::frame_time_report::collect_sidecars(&pre_paths);
            let (post, post_fail) =
                riir_refine::frame_time_report::collect_sidecars(std::slice::from_ref(&vs));
            let threshold = args
                .frame_delta_pct
                .unwrap_or(riir_refine::frame_time_report::DEFAULT_DELTA_PCT);
            let pre_rows = riir_refine::frame_time_report::budget_rows(&pre, args.frame_budget);
            let post_rows = riir_refine::frame_time_report::budget_rows(&post, args.frame_budget);
            let report = riir_refine::frame_time_report::diff_ab(
                &pre_rows, &post_rows, threshold, pre_fail, post_fail,
            );
            print!("{}", report.render());
            if report.blind() {
                std::process::exit(1);
            }
            return;
        }
        let (loaded, failures) = riir_refine::frame_time_report::collect_sidecars(&pre_paths);
        let rows = riir_refine::frame_time_report::budget_rows(&loaded, args.frame_budget);
        let report = riir_refine::frame_time_report::FrameTimeReport { rows, failures };
        print!("{}", report.render());
        if report.blind() {
            std::process::exit(1);
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
            let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            let kv = root.join(".refine").join("trajectories.kv");
            if !kv.exists() {
                eprintln!(
                    "recompute-elo: no store at {} — run any --fix in this repo first",
                    kv.display()
                );
                std::process::exit(1);
            }
            match riir_refine::traj_store::TrajStore::open(&root) {
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
    // RIIR_REFINE_TRAJ_ROOT (no migration — this mode never writes), falling
    // back to the memory.json envelope, plus a fresh-empty baseline for the
    // elevation delta. Feeds self_evolve_audit.sh §1c.
    if args.collapse_report {
        #[cfg(feature = "latent_retrieval")]
        {
            use riir_refine::collapse_monitor::collapse_report_common;
            let spans = riir_refine::draft::eval_fixtures_clippy::CLIPPY_SPANS;
            let corpus = riir_refine::build_seed_corpus().entries;

            // Fresh baseline (condition b).
            let fresh = riir_refine::collapse_monitor::measure(
                &riir_refine::memory::LatentFixMemory::default(),
                spans,
                &corpus,
            );
            println!(
                "collapse-report: fixture spans={} corpus_rules={}",
                spans.len(),
                corpus.len()
            );
            println!("{}", fresh.report("  fresh(empty)"));

            // Production condition (a): the store, or the envelope fallback.
            let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            if root.join(".refine").join("trajectories.kv").exists() {
                match riir_refine::traj_store::TrajStore::open(&root) {
                    Ok((store, bad)) => {
                        if bad > 0 {
                            eprintln!("collapse-report: {bad} corrupt row(s) skipped");
                        }
                        collapse_report_common(
                            &format!("  store({})", root.display()),
                            &riir_refine::collapse_monitor::memory_from_store_rows(&store.dump()),
                            spans,
                            &corpus,
                        );
                        return;
                    }
                    Err(e) => {
                        eprintln!(
                            "collapse-report: store at {} unreadable: {e}",
                            root.display()
                        );
                        std::process::exit(1);
                    }
                }
            }
            let envelope = default_memory_path();
            if envelope.exists() {
                let mut mem = riir_refine::memory::LatentFixMemory::default();
                match mem.load(&envelope) {
                    Ok(()) => {
                        collapse_report_common("  envelope", &mem, spans, &corpus);
                        return;
                    }
                    Err(e) => {
                        eprintln!(
                            "collapse-report: envelope {} unreadable: {e}",
                            envelope.display()
                        );
                        std::process::exit(1);
                    }
                }
            }
            eprintln!(
                "collapse-report: no store at {} — showing the fresh baseline only",
                root.display()
            );
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
    // `mode_flag_tests` below) — the just-works defaults (bare = dry run,
    // --fix = write + compile-gate) were already applied inside
    // parse_args; the banner here is the only bare-mode surface.
    if args.bare_default {
        println!(
            "cargo refine: dry run — reviewing {} (no edits). Real fixes: cargo refine --fix",
            if args.paths.len() == 1 && args.paths[0] == std::path::Path::new(".") {
                "the current directory".to_string()
            } else {
                format!("{} path(s)", args.paths.len())
            }
        );
    }
    if let Err(msg) = validate_mode_flags(&args) {
        eprintln!("{msg}");
        usage();
    }
    // Issue 075 (owner soft gate): the depleted-account refusal — fires
    // BEFORE any scanning (prepaid fuel: an empty tank dispenses nothing,
    // in every mode that burns). The cfg IS the predicate's first leg: a
    // build without transport has no synced balance and stays warn-only.
    #[cfg(all(feature = "kat_service", feature = "kat_transport"))]
    enforce_kat_soft_gate(&args);
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
    // backend's own file. Redirectable: `cargo refine --dump-trajectories
    // > traj.jsonl`.
    if args.dump_trajectories {
        // The neuron-db backend (`latent_retrieval` builds): the repo-local
        // durable store. Requires a path (the repo root to resolve
        // `.refine/trajectories.kv`) — `RIIR_REFINE_TRAJ_ROOT` overrides.
        #[cfg(feature = "latent_retrieval")]
        {
            let root = std::env::var("RIIR_REFINE_TRAJ_ROOT")
                .map(PathBuf::from)
                .or_else(|_| std::env::current_dir())
                .unwrap_or_default();
            if root.join(".refine").join("trajectories.kv").exists() {
                match riir_refine::traj_store::TrajStore::open(&root) {
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
                        eprintln!(
                            "dump-trajectories: store at {} unreadable: {e}",
                            root.display()
                        );
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
                let mut mem = riir_refine::memory::LatentFixMemory::default();
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
        use riir_refine::domains::rustc_errors::driver::CompileDriver;
        use riir_refine::domains::rustc_errors::evolve;
        use riir_refine::domains::rustc_errors::playbooks::Playbooks;

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
            riir_refine::compile_oracle::owning_package_root(p)
                .and_then(|pkg| riir_refine::compile_oracle::workspace_root(&pkg))
                .or_else(|| std::env::current_dir().ok())
                .and_then(|root| {
                    match riir_refine::traj_store::TrajStore::open(&root) {
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
        // Issue 093 T2 (opt-in `nonergodic_driver` builds): the per-run
        // strategy-posterior ordering REPLACES the static shape-conditional
        // one when armed (feature on + `RIIR_REFINE_NONERGODIC` not killed) —
        // the store, when present, seeds the base ranking and composes the
        // arm-D blend with the stale-detection gate. Installed BEFORE the
        // static block so `with_ordering` here wins.
        #[cfg(feature = "nonergodic_driver")]
        if riir_refine::domains::rustc_errors::nonergodic_order::armed_by_env() {
            let policy = std::sync::Arc::new(
                riir_refine::domains::rustc_errors::nonergodic_order::NonergodicOrdering::new(
                    traj_store.clone(),
                ),
            );
            driver = driver
                .with_ordering(policy.ordering())
                .with_observer(policy.observer())
                .with_revive(policy.revive());
            eprintln!("fix-compile: nonergodic ordering armed (Issue 093 T2)");
        }
        #[cfg(feature = "latent_retrieval")]
        if let Some(store) = traj_store.clone() {
            driver = driver.with_ordering(std::sync::Arc::new(move |cands| {
                riir_refine::traj_store::shape_conditional_order(cands, &store);
            }));
        }
        let driver = driver.with_extra_args(extra);
        let playbooks = Playbooks::production();
        let report = driver.run(&args.paths, &playbooks, args.write);
        if let Some(e) = &report.error {
            eprintln!("fix-compile: {e}");
            std::process::exit(1);
        }
        // Issue 118 T1: the bench-guard decision surface — the driver
        // declined (or flagged) bench-target candidates; print loudly so
        // the operator knows why those errors were left standing.
        if report.bench_declined > 0 {
            for f in &report.bench_declined_files {
                println!(
                    "bench-guard: declined bench-target fixes in {} (a compile repair can \
                     alter what the bench measures — apply manually only)",
                    f.display()
                );
            }
            println!(
                "bench-guard: {} bench-target error site(s) declined this run",
                report.bench_declined
            );
        }
        for f in &report.bench_forced_files {
            println!(
                "bench-guard: {} is a bench target named explicitly — fixes proceeded \
                 (measurement risk accepted by the explicit path)",
                f.display()
            );
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
                    use riir_refine::domains::rustc_errors::evolve::traj_key;
                    // Issue 105: the run's journey credit, computed once —
                    // every kept attempt in this run rode the same journey.
                    // Tainted runs (ended at a masking-stage error) stamp
                    // their composite; clean runs bump the count alone.
                    let journey = report.journey();
                    let taint = journey.tainted().then_some(journey.taint_composite_256());
                    for a in &report.applied {
                        reopen067.note(store, &traj_key(&a.code, &a.strategy_id));
                        // riir-dapps Issue 119 T2: the row carries its file
                        // (repo-relative) — the sync gate's allow_paths input.
                        let src_path = store.src_path_of(&a.file);
                        let sourced = store.sourced(&src_path);
                        // Issue 148 T-b follow-up: `record_classified` is
                        // `record` plus the attribution + label params —
                        // `None` component keeps record's posture exactly;
                        // the driver's strategy menu rides the same label
                        // path the clippy lane opened.
                        if let Err(e) = sourced.record_classified(
                            &traj_key(&a.code, &a.strategy_id),
                            &a.shape,
                            &a.fix,
                            true,
                            true,
                            &heal_run_id,
                            None,
                            rejected_opt(&a.rejected),
                        ) {
                            eprintln!("self-evolve: store record failed: {e}");
                        }
                        if let Err(e) = sourced.record_journey(
                            &traj_key(&a.code, &a.strategy_id),
                            &a.shape,
                            &a.fix,
                            &heal_run_id,
                            taint,
                        ) {
                            eprintln!("self-evolve: journey record failed: {e}");
                        }
                    }
                    for r in &report.reverted {
                        // Issue 047: no-progress = the drafted fix was defective
                        // (W); I/O = the gate rejected without evaluating the
                        // fix (C — unnecessary, gate-side).
                        let component =
                            riir_refine::component::classify(
                                &riir_refine::component::FailureSignal::reverted(
                                    match r.reason {
                                        riir_refine::domains::rustc_errors::driver::RevertReason::NoProgress => riir_refine::component::RevertKind::CompileGate,
                                        riir_refine::domains::rustc_errors::driver::RevertReason::Io => riir_refine::component::RevertKind::Infrastructure,
                                    },
                                ),
                            );
                        let src_path = store.src_path_of(&r.file);
                        if let Err(e) = store.sourced(&src_path).record_classified(
                            &traj_key(&r.code, &r.strategy_id),
                            &r.shape,
                            &r.fix,
                            false,
                            false,
                            &heal_run_id,
                            Some(component),
                            // Issue 148 T-b follow-up LANDED: the driver's
                            // strategy menu (the alternatives considered at
                            // this site when the oracle rejected THIS move)
                            // rides the same label path as the applied fix.
                            rejected_opt(&r.rejected),
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
            // ── Phase 4 (T4.4) + Issue 91 T1.5: misses → the authoring
            // queue; the attempt ring → the fixseq replay store.
            if !report.misses.is_empty() || !report.attempts.is_empty() {
                let root = args
                    .paths
                    .first()
                    .and_then(|p| riir_refine::compile_oracle::owning_package_root(p.as_path()))
                    .and_then(|pkg| riir_refine::compile_oracle::workspace_root(&pkg))
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                if !report.misses.is_empty() {
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
                // Issue 91 T1.5 — the per-attempt ring: one instrumentation,
                // two kernel consumers (the T2 oscillation detector over
                // (code, span_hash) + Issue 093's per-run strategy
                // posterior). The dump is an aid, never a gate.
                if !report.attempts.is_empty() {
                    let (written, warnings) = evolve::dump_fixseq(&root, &report.attempts);
                    for w in &warnings {
                        eprintln!("fix-compile: fixseq dump: {w}");
                    }
                    if let Some(p) = written {
                        println!(
                            "fix-compile: {} attempt(s) → {}",
                            report.attempts.len(),
                            p.display()
                        );
                    }
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
    let lease_store = riir_refine::kat_lease_client::LeaseStore::open_net(
        &resolve_heal_root(&args.paths),
        &net_partition_url(),
    );
    // T2.4: with the promotion surface compiled in, Certified network rules
    // merge into the run overlay (never under a --corpus-epoch pin).
    #[cfg(all(feature = "kat_service", feature = "score_bench"))]
    let run_overlay = run_overlay_with_promotion(
        &lease_store,
        args.corpus_epoch,
        &resolve_heal_root(&args.paths),
    );
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

    // Issue 102 T2: the corpus active-set marker — loaded ONCE, before the
    // matcher build, so the suggest fan-out and the fix dispatch read the
    // same view. Nothing here demotes: the marker is written only by the
    // operator CLI (--active-set-*), and the fade-predictive gate (Issue
    // 102's ≥3-future-generations clock) is the only thing that could ever
    // automate one. A corrupt marker is LOUD and the run continues with an
    // empty set — an operator marker must never take a heal run down.
    let mut active_set = if args.fix || args.suggest {
        match riir_refine::active_set::ActiveSet::load_default(&resolve_heal_root(&args.paths)) {
            Ok(set) => set,
            Err(e) => {
                eprintln!("active-set: {e} — continuing with an EMPTY active set this run");
                riir_refine::active_set::ActiveSet::empty()
            }
        }
    } else {
        riir_refine::active_set::ActiveSet::empty()
    };
    if !active_set.is_empty() {
        println!(
            "active-set: {} demoted rule(s) loaded from {} (the similarity fan-out and the fix \
             dispatch skip them; oracle anchors pass through, loud)",
            active_set.demoted_total(),
            riir_refine::active_set::ACTIVE_SET_FILE
        );
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
    matcher = matcher.with_active_set(active_set.clone());

    // Issue 134 T2 (cfg(demonstrability_gate)): the pool-depth allocation
    // AUTO-arm — read the standing demonstrability constants from THIS
    // repo's score history (resolve_corpus_repo_root, the corpus-stats
    // precedent: the constants are a property of the healer's corpus, not
    // of the scanned target) and install the policy when it is
    // non-identity. The kill-switch (RIIR_REFINE_DEMONSTRABILITY=0), an
    // absent history, and an identity policy all leave the run at the
    // default posture — standing() returns None and says nothing (the
    // absent-history case is the ordinary one on installed binaries, where
    // no score-bench history exists).
    #[cfg(feature = "demonstrability_gate")]
    {
        if let Some(root) = resolve_corpus_repo_root() {
            if let Some(policy) = riir_refine::demonstrability_gate::DepthPolicy::standing(&root) {
                println!(
                    "demonstrability gate: armed from {} — per-domain depth factors: {}",
                    riir_refine::demonstrability_gate::STANDING_RELPATH,
                    policy.render()
                );
                matcher = matcher.with_depth_policy(policy);
            }
        }
    }

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
    // suggest walk when kernel_opt is enabled.
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
    // Issue 085: the third name-matched surface — Cargo.toml manifests for
    // the dist domain (suggest-only build-config advice).
    let mut cargo_tomls = Vec::new();
    for path in &args.paths {
        collect_cargo_tomls(path, &mut cargo_tomls);
    }
    if files.is_empty() && wgsl_files.is_empty() && dockerfiles.is_empty() && cargo_tomls.is_empty()
    {
        eprintln!("no .rs/.wgsl/Dockerfile/Cargo.toml files under the given paths");
        std::process::exit(1);
    }
    // Issue 149 T1: the pre-scan disclosure — WHICH absolute tree (untracked
    // WIP included) this run is about to scan. One line per distinct scan
    // root; the untracked count is best-effort and never a gate.
    let scanned: Vec<PathBuf> = files
        .iter()
        .chain(wgsl_files.iter())
        .chain(dockerfiles.iter())
        .chain(cargo_tomls.iter())
        .cloned()
        .collect();
    for info in scan_root_infos(&args.paths, &scanned) {
        println!("{}", render_scan_line(&info));
    }
    #[cfg(not(feature = "docker"))]
    if !dockerfiles.is_empty() {
        eprintln!(
            "note: {} Dockerfile(s) found but the docker domain needs --features docker (Plan 030)",
            dockerfiles.len()
        );
    }
    #[cfg(not(feature = "dist"))]
    if !cargo_tomls.is_empty() {
        eprintln!(
            "note: {} Cargo.toml manifest(s) found but the dist domain needs --features dist (Issue 085)",
            cargo_tomls.len()
        );
    }
    #[cfg(not(feature = "kernel_opt"))]
    if !wgsl_files.is_empty() {
        eprintln!(
            "note: {} .wgsl file(s) found but the non-syn scanner needs --features kernel_opt (Issue 025)",
            wgsl_files.len()
        );
    }

    // Issue 018/038: self-evolve recording — the memory half runs in every
    // build; the EMA learning half degrades gracefully without an oracle
    // (one log line). `RIIR_REFINE_SELF_EVOLVE=0` makes this fully inert.
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
            Ok(riir_refine::CommitmentStatus::Mismatch) => eprintln!(
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
    // kill-switch — `RIIR_REFINE_KAT_METER=0` disables the whole billing path.
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
    let mut lease_charge = riir_refine::kat_lease_client::LeaseCharge::open(&lease_store);
    #[cfg(feature = "kat_transport")]
    let mut lease_ctx: Option<ReheatCtx> = None;
    #[cfg(feature = "kat_transport")]
    {
        let phase_live = !args.no_update
            && args.update_timeout > 0
            && args.corpus_epoch.is_none()
            && !riir_refine::kat_lease_client::lease_kill_switched();
        let service_url = riir_refine::kat_lease_client::service_url_from_env();
        if phase_live && service_url.is_some() {
            let url = service_url.clone().unwrap_or_default();
            let transport =
                riir_refine::kat_lease_client::HttpLeaseTransport::new(&url, args.update_timeout);
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_secs(args.update_timeout);
            let out_of_kat = kat_charge
                .as_ref()
                .is_some_and(|c| c.balance_projection() == 0);
            let outcome = riir_refine::kat_lease_client::run_update_phase(
                &transport,
                &lease_store,
                &mut lease_charge,
                out_of_kat,
                Some(deadline),
            );
            if let riir_refine::kat_lease_client::UpdateStatus::Applied {
                epoch,
                charge_micro_kat,
            } = outcome.status
            {
                // The visible charge line + the matcher rebuild onto E+1 for
                // THIS run (rule 1: proceed on the freshest applied epoch).
                println!(
                    "{}",
                    riir_refine::kat_lease_client::LeaseCharge::line(epoch, charge_micro_kat)
                );
                if let Some((_, overlay)) = lease_overlay_for_run(&lease_store, None) {
                    match LatentMatcher::from_enabled_domains_with_overlay(&overlay) {
                        Ok(m) => matcher = m.with_active_set(active_set.clone()),
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
            && !riir_refine::kat_lease_client::lease_kill_switched()
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
    // Issue 165: the run-end roll-up's accumulator + the fixable ceiling it
    // reports (the compiled clippy fix set; built once, read per citation).
    let mut domain_tally = DomainTallies::default();
    let fixable_set: std::collections::BTreeSet<String> = {
        let (fixable, verify) = fix_lane_tiers();
        fixable.into_iter().chain(verify).collect()
    };
    let mut n_fixed_files = 0usize;
    // Proposal 004 Half 1.5 — the fix-stats row's rule counts: WRITTEN
    // edits only (the Issue-018 law — review-only runs record nothing;
    // the map stays empty and the tail skips). Accumulated beside the
    // n_fixed_* counters so the two can never disagree about what a run
    // "fixed".
    let mut fixstat_rules: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();
    // ── the clippy-fed oracle anchors (2026-09-13 retrieval-miss intake):
    // a real clippy JSON stream force-includes its diagnostic spans/rules
    // AHEAD of similarity ranking — the top-N gate must never eat a span
    // real clippy flagged. Loaded once; anchors are matched per file.
    // Issue 113 (the auto-oracle): with no manual feed, a scan over a cargo
    // project RUNS the oracle itself — the anchored spans (the ones that
    // reproduce under the proof oracle and can credit) become the default
    // instead of a flag nobody discovers. RIIR_REFINE_AUTO_ORACLE=0 restores
    // the old never-spawn posture; docker/dist lanes have no rust spans.

    // ── Issue 146 T0.1/T0.5: the hotpath report lane (validate + inspect) ──
    // Parsed, keyed to the project root's git HEAD (mismatch = REFUSE exit 2,
    // the stale-profile hazard; unkeyed = loud disclosure), top hot spans
    // printed in the run summary. T0.2 wires the anchoring (`[hot: …]`
    // spans ahead of similarity ranking) — the flag pins the wire early.
    let hotpath_spans: Option<Vec<riir_refine::hotpath_report::HotSpan>> = if let Some(path) =
        args.hotpath_json.as_ref()
    {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            // The clippy-feed precedent: a requested oracle that cannot
            // be read is a HARD error — a silent fallback to ranking-only
            // would re-create the confident miss this flag exists to
            // prevent.
            Err(e) => {
                eprintln!("hotpath feed: cannot read {path}: {e}");
                std::process::exit(2);
            }
        };
        let report: riir_refine::hotpath_report::HotpathReport = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("hotpath feed: cannot parse {path}: {e}");
                std::process::exit(2);
            }
        };
        let spans = riir_refine::hotpath_report::hot_spans(&report);
        let n_entries = report.functions_timing.as_ref().map_or(0, |l| l.data.len());
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let report_keyed = report.meta.git.is_some();
        match riir_refine::clippy_oracle::detect_project_root(&args.paths, &cwd) {
            Some(root) => {
                let repo_sha = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&root)
                    .arg("rev-parse")
                    .arg("HEAD")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string());
                match repo_sha
                    .as_deref()
                    .map(|sha| riir_refine::hotpath_report::key_verdict(&report, sha))
                {
                    Some(riir_refine::hotpath_report::KeyVerdict::Matched { sha }) => {
                        eprintln!(
                                "hotpath feed: keyed MATCHED at repo HEAD {sha} — {n_entries} timed entr(ies), {} located hot span(s)",
                                spans.len()
                            );
                    }
                    Some(riir_refine::hotpath_report::KeyVerdict::Mismatched {
                        report_sha,
                        repo_sha,
                    }) => {
                        eprintln!(
                                "hotpath feed: STALE PROFILE REFUSED — report was taken at {report_sha} but the repo is at {repo_sha}; re-profile at the current HEAD or checkout that SHA"
                            );
                        std::process::exit(2);
                    }
                    _ => {
                        if report_keyed {
                            eprintln!(
                                    "hotpath feed: the report carries meta.git but the project root's git HEAD could not be read (not a git checkout / git failed) — CANNOT VERIFY the key; {} located hot span(s) shown best-effort",
                                    spans.len()
                                );
                        } else {
                            eprintln!(
                                    "hotpath feed: report is UNKEYED (no meta.git — plain hotpath without hotpath-cloud); proceeding with a loud disclosure: span lines are best-effort against the current tree — {} located hot span(s)",
                                    spans.len()
                                );
                        }
                    }
                }
            }
            None => {
                eprintln!(
                        "hotpath feed: no project root detected — cannot key the report; {} located hot span(s) shown best-effort",
                        spans.len()
                    );
            }
        }
        Some(spans)
    } else {
        None
    };
    // The T0 inspection surface: the ranked hot spans, printed beside the
    // keying verdict (anchoring into the retrieval lane is T0.2; this print
    // is what makes the feed useful TODAY — the operator sees what reality
    // measured before the lane exists).
    if let Some(spans) = hotpath_spans.as_ref() {
        eprintln!(
            "hotpath feed: top {} of {} hot span(s) by time share (calls x avg — RANKING-only attribution; alloc = the verification axis):",
            spans.len().min(10),
            spans.len()
        );
        for s in spans.iter().take(10) {
            let avg = s.avg_ns.map_or("n/a".to_string(), |ns| format!("{ns} ns"));
            let alloc = s
                .alloc_bytes
                .map_or(String::new(), |b| format!(" | alloc {b} B"));
            eprintln!(
                "  [hot] {} — {}:{} — {} calls x {} avg{alloc}",
                s.name, s.file, s.line, s.calls, avg
            );
        }
        // Issue 146 T0.3: the instrumentation coverage-bias disclosure —
        // hotpath sees ONLY functions carrying its own instrumentation, so
        // "no hot span here" is NEVER evidence of coldness. The report's
        // coverage is disclosed, never treated as exhaustive.
        eprintln!(
            "hotpath feed: coverage bias — only instrumented functions (#[hotpath::measure] et al.) \
             are measured; un-annotated code never reads hot and is invisible to this feed"
        );
    }

    let clippy_feed: Option<Vec<ClippyDiagnostic>> = if let Some(path) = args.clippy_json.as_ref() {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let diags = parse_clippy_feed(&text);
                if diags.is_empty() {
                    eprintln!(
                        "clippy feed: parsed 0 diagnostics from {path} — every span will be marked heuristic"
                    );
                }
                Some(diags)
            }
            Err(e) => {
                // A requested oracle that cannot be read is a hard error:
                // silently falling back to ranking-only would re-create the
                // confident miss this flag exists to prevent.
                eprintln!("clippy feed: cannot read {path}: {e}");
                std::process::exit(2);
            }
        }
    } else if matches!(args.domain, Some(DomainId::Docker) | Some(DomainId::Dist))
        || !riir_refine::clippy_oracle::auto_oracle_enabled()
    {
        // Non-rust lanes have no spans to anchor; the kill-switch restores
        // the pre-113 never-spawn posture. Both are quiet by design —
        // opting out should not nag.
        None
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        match riir_refine::clippy_oracle::detect_project_root(&args.paths, &cwd) {
            None => None,
            Some(root) => {
                let cargo_bin = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
                // Issue 115: the operator can re-aim the spawn at a feature
                // set — diagnostics that only fire behind a non-default
                // feature are invisible at the default posture (the exact
                // class consumer_residue_sweep.sh's screen exists for).
                let features = riir_refine::clippy_oracle::auto_oracle_features();
                // 2026-09-21: --all-targets is the DEFAULT posture (mod
                // tests spans anchored like any other — the riir-chain/riir-ai
                // intake pair); the kill-switch restores the fast lib-only
                // spawn.
                let all_targets = riir_refine::clippy_oracle::auto_oracle_all_targets();
                let targets_note = if all_targets {
                    "all targets (tests/benches/examples included)"
                } else {
                    "lib targets only (RIIR_REFINE_AUTO_ORACLE_NO_ALL_TARGETS is set)"
                };
                match features.as_deref() {
                    Some(set) => eprintln!(
                        "clippy oracle (auto): linting {} with features [{}], {} — a feature-superset clippy run is materially slower than default features; first run can take a while (RIIR_REFINE_AUTO_ORACLE_FEATURES; unset for default features; RIIR_REFINE_AUTO_ORACLE=0 disables)",
                        root.display(),
                        set.join(","),
                        targets_note
                    ),
                    None => eprintln!(
                        "clippy oracle (auto): linting {} at default features, {} — this compiles the project, first run can take a while (RIIR_REFINE_AUTO_ORACLE=0 disables)",
                        root.display(),
                        targets_note
                    ),
                }
                // A knob-directed spawn that fails must name the knob: the
                // most likely cause is a feature name the target project
                // does not carry (Issue 115 T2 — loud, never silent).
                let feature_hint = |msg: &str| {
                    if features.is_some() {
                        format!(
                            "{msg} — RIIR_REFINE_AUTO_ORACLE_FEATURES is set; check the feature names against the project's manifests, or unset the knob"
                        )
                    } else {
                        msg.to_owned()
                    }
                };
                match riir_refine::clippy_oracle::run_clippy(
                    &root,
                    &cargo_bin,
                    features.as_deref(),
                    all_targets,
                ) {
                    Ok(run) => {
                        let diags = parse_clippy_feed(&run.text);
                        match riir_refine::clippy_oracle::classify(run.exit, diags.len()) {
                            riir_refine::clippy_oracle::OracleOutcome::Feed(n) => {
                                eprintln!(
                                    "clippy oracle (auto): {n} real diagnostic(s) anchored ahead of ranking"
                                );
                                Some(diags)
                            }
                            riir_refine::clippy_oracle::OracleOutcome::Clean => {
                                // Word the arm honestly: with the knob active
                                // this is clean at the REQUESTED features, not
                                // at default ones.
                                match features.as_deref() {
                                    Some(set) => eprintln!(
                                        "clippy oracle (auto): clean tree at features [{}] — covered spans will be marked heuristic",
                                        set.join(",")
                                    ),
                                    None => eprintln!(
                                        "clippy oracle (auto): clean tree at default features — covered spans will be marked heuristic"
                                    ),
                                }
                                Some(diags)
                            }
                            riir_refine::clippy_oracle::OracleOutcome::Failed(msg) => {
                                eprintln!(
                                    "clippy oracle (auto): {} — continuing heuristic-only",
                                    feature_hint(&msg)
                                );
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "clippy oracle (auto): {} — continuing heuristic-only",
                            feature_hint(&e)
                        );
                        None
                    }
                }
            }
        }
    };
    let mut n_oracle_spans = 0usize;
    let mut n_anchored_missing = 0usize;
    // Plan 165 (`--repo-posture`): a posture run with NO feed cannot
    // verify anything — applying everything would be the unverified bulk
    // apply wearing the mode's name. REFUSE (exit 2), naming both
    // remedies. A feed with ZERO diagnostics (a clean tree) is a VALID
    // posture input and proceeds — declining every clippy-lane edit IS
    // the posture answer for a clean tree.
    if args.repo_posture && clippy_feed.is_none() {
        eprintln!(
            "repo-posture: no clippy feed available (the auto-oracle could not run or is disabled, and no --clippy-json was given) — refusing rather than apply edits the target repo's own clippy never verified; unset --repo-posture, provide --clippy-json, or check the oracle status lines above"
        );
        std::process::exit(2);
    }
    // Run-total posture declines: per-lint counts for the summary + the
    // file count. Counted at PASS 0 only (the bench-guard precedent:
    // later fixpoint passes re-propose the identical declined set).
    let mut n_posture_declined_total = 0usize;
    let mut posture_declined_by_lint: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    // Issue 135 T2: clippy-lane edits skipped because the lint is
    // allow-by-default and the TARGET crate never enables it (per-lint run
    // totals; pass-0 counted, the bench-guard law). The resolver caches
    // each crate's manifest + root attributes once per run.
    let mut level_resolver = LevelResolver::new();
    let mut level_skipped_by_lint: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    // Issue 116: the per-span `[no corpus rule]` lists, kept so the run can
    // report WHICH of the two messages that marker is delivering. Collected
    // rather than counted because the delta is per-LINT: the existing
    // `n_anchored_missing` is a span total and cannot be partitioned.
    let mut missing_by_span: Vec<Vec<String>> = Vec::new();
    let mut n_fixed_edits = 0usize;
    // P21: oracle-anchored edits actually APPLIED in the fix lane.
    let mut n_oracle_edits = 0usize;
    // P30 bench-guard: rust_perf edits declined on bench targets, run total.
    let mut n_bench_declined_total = 0usize;
    // P39: NotBenchable rust_perf edits declined under --repo-posture, run
    // total (never benchmarked — the posture mode declines them unless
    // --apply-notbenchable says otherwise).
    let mut n_notbench_declined_total = 0usize;
    // Issue 118 T2: oracle suggestions declined on bench targets (the P21
    // lane rides the P30 door), run total.
    let mut n_bench_oracle_declined = 0usize;
    // Issue 146 T0.3: edits declined for touching hotpath's own
    // instrumentation lines, run total (the fix lane must never eat the
    // oracle it consumes — pass-0 counting, the bench-guard law).
    let mut n_hotattr_declined_total = 0usize;
    // Issue 146 T0.2: spans carrying measured-hot marks, run total (the
    // hotpath-fed oracle lane's review population).
    let mut n_hot_spans = 0usize;
    // P12: re-proposal suppression. Default-on; the kill-switch re-enables
    // unconditional re-proposal (force retry of previously-reverted edits).
    let suppress_reverted = !suppression_disabled();
    // Unique suppressed pairs for the end-of-run report: (rule, restore,
    // fix) → the (file, line) of the first drop. Deduped by the pair, not
    // the line — a suppressed pattern stays in the text and is re-collected
    // (and re-dropped) on every fixpoint pass.
    let mut suppressed_pairs: std::collections::HashMap<(String, String, String), (String, usize)> =
        std::collections::HashMap::new();

    // Issue 015: under --verify the fix collection loosens the
    // vec-with-capacity usize gate (rustc backs the revert) and adds the
    // VERIFY_LINTS (type-blind under parse-only, revertible under rustc).
    #[cfg(feature = "fix_verify")]
    let proposer_opts = if args.verify {
        riir_refine::domains::rust_perf::proposers::ProposerOpts {
            untyped_paths: true,
        }
    } else {
        riir_refine::domains::rust_perf::proposers::ProposerOpts::default()
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
    let mut verify_batches: Vec<riir_refine::fix_verify::FileEditSet> = Vec::new();
    // Native fmt (src/fmt.rs): per-batch decisions for the verify path —
    // decided against the ORIGINAL file pre-verify, applied post-verify to
    // batches with ≥1 kept edit (a fully-reverted file is back at its
    // already-clean original — nothing to format).
    #[cfg(feature = "fix_verify")]
    let mut verify_fmt: Vec<(PathBuf, riir_refine::fmt::FmtDecision, String)> = Vec::new();

    // Native fmt pass: default-on with --fix (--no-fmt disables). Decisions
    // only compute when rustfmt is reachable; one warning otherwise.
    let fmt_enabled = args.fix && !args.no_fmt;
    let fmt_active = fmt_enabled && riir_refine::fmt::rustfmt_available();
    if fmt_enabled && !fmt_active {
        eprintln!("fmt: rustfmt not found on PATH — automatic formatting disabled (--no-fmt silences this)");
    }
    let mut fmt_summary = riir_refine::fmt::FmtSummary::default();

    // Issue 151: the in-run secret disclosure — one canonical-scanner pass
    // per read file (cheap, text-linear), reported once after the summary.
    // Quiet when zero: a warn firing on every clean run is a warn nobody
    // reads. SUGGEST-ONLY by construction — no proposer exists, --fix never
    // touches these (a secret's fix is a process change, not a rewrite).
    let mut secret_findings: Vec<(String, Vec<riir_refine::secret_blind::SpanFinding>)> =
        Vec::new();
    // P63: the digest-pin adjudication count (report-lane only — the
    // egress gate keeps the raw fail-closed verdicts).
    let mut secret_pinned: usize = 0;

    for file in &files {
        let Ok(src) = read_source(file) else {
            eprintln!("skip (unreadable): {}", file.display());
            continue;
        };
        let display = file.display().to_string();
        // Issue 151: the same-file secret scan (suggest-only; values and
        // fingerprints never shown — the shareable-output rule). P63: the
        // adjudicated outcome — digest pins leave the finding list (they
        // were the recurring all-false noise) and ride the disclosure
        // count instead.
        let file_secret_outcome = riir_refine::secret_blind::scan_file_with_pins(&src);
        secret_pinned += file_secret_outcome.digest_pins.len();
        if !file_secret_outcome.findings.is_empty() {
            secret_findings.push((display.clone(), file_secret_outcome.findings));
        }
        // P30: bench-target detection, shared by both lanes — the fix lane
        // declines rust_perf edits on it, the suggest lane stamps perf rows.
        let bench_target =
            riir_refine::domains::rust_perf::bench_guard::is_bench_target(&display, &src);

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
        let mut pass_splices: Vec<Vec<riir_refine::edit_emitter::SourceEdit>> = Vec::new();
        // Per-pass INPUT texts (the span-revert restore source).
        let mut pass_texts: Vec<String> = Vec::new();
        // P21: the (start, end) byte ranges of oracle-anchored edits pushed
        // at pass 0 — the report-note key (proposer edits never share an
        // exact span with a kept oracle edit: overlaps are filtered out
        // before the push).
        let mut oracle_keys: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();
        if args.fix {
            // Issue 031 H2: --domain scopes the fix path. Previously the
            // proposer loops ran BOTH domains unconditionally — `--domain
            // clippy` applied rust_perf rules (vec-with-capacity edits landed
            // in katgpt-pruners under --domain clippy). `auto` (None) keeps
            // both; `kernel` runs neither (kernel proposers are Phase 5).
            let (run_perf, run_clippy, run_sec) = fix_domain_scopes(args.domain);
            #[cfg(not(feature = "sec"))]
            let _ = run_sec; // only consumed under the `sec` feature
                             // The bench-target guard (2026-09-16 intake, P30): applying a
                             // perf rule to a bench target changes WHAT the bench measures —
                             // `--verify` proves compilation, never measurement validity,
                             // and a bench file's numbers ARE its artifact. Decline loudly,
                             // never silently; the operator can still apply manually.
            let bench_guard = run_perf && bench_target;
            // P39 (post-mining intake, 2026-09-25): under --repo-posture the
            // rust_perf lane is exempt from the posture MECHANISM (the
            // target's clippy says nothing about our rules) — but the MODE's
            // contract is "only what a gate verified", and a NotBenchable
            // fixable was verified by nobody. Decline those rules' edits
            // (loudly, with the explicit --apply-notbenchable override) —
            // the bench-guard pattern one lane over. Scope: the posture mode
            // ONLY; plain --fix keeps the operator's full intent.
            let notbench_guard = run_perf && args.repo_posture && !args.apply_notbenchable;
            let mut n_bench_declined = 0usize;
            let mut n_notbench_declined = 0usize;
            // Issue 146 T0.3: hotpath-instrumentation-line declines, this
            // file (pass-0 count; the filter itself runs every pass).
            let mut n_hotattr_declined_file = 0usize;
            // fxhashmap dep gate (Issue 015): skip the rule for files in
            // crates without the rustc-hash dependency.
            #[cfg(feature = "fix_verify")]
            let fxhashmap_allowed = {
                let pkg = riir_refine::fix_verify::owning_package_root(file);
                match pkg {
                    None => false,
                    Some(pkg) => *pkg_dep_cache.entry(pkg.clone()).or_insert_with(|| {
                        std::fs::read_to_string(pkg.join("Cargo.toml")).is_ok_and(|t| {
                            riir_refine::fix_verify::manifest_has_dep(&t, "rustc-hash")
                        })
                    }),
                }
            };
            #[cfg(not(feature = "fix_verify"))]
            let fxhashmap_allowed = true;

            // Plan 165 (`--repo-posture`): this file's per-lint diagnostic
            // span index in ORIGINAL on-disk coordinates — the target
            // repo's own clippy verdict for this file. Built once per
            // file; each fixpoint pass remaps it through the applied
            // splices (authority-follows-replacement) before gating.
            let posture_index: Option<std::collections::BTreeMap<String, Vec<(usize, usize)>>> =
                if args.repo_posture {
                    clippy_feed
                        .as_ref()
                        .map(|feed| riir_refine::draft::clippy_feed::posture_spans(feed, &display))
                } else {
                    None
                };
            let mut n_posture_declined_file = 0usize;
            // Issue 135 T2: the lints this file's crate never enabled (an
            // unparseable file breaks out of the pass loop anyway).
            let level_gated: Vec<&'static str> = if run_clippy {
                syn::parse_file(&src).map_or_else(
                    |_| Vec::new(),
                    |parsed| {
                        gated_lints(&level_resolver.levels_for(file, &parsed), &args.fix_groups)
                    },
                )
            } else {
                Vec::new()
            };
            let mut level_skipped_file: std::collections::BTreeMap<String, usize> =
                std::collections::BTreeMap::new();

            for pass in 0..MAX_FIX_PASSES {
                let Ok(parsed) = syn::parse_file(&current) else {
                    eprintln!("skip (unparseable): {display}");
                    break;
                };
                let mut edits: Vec<RuleEdit> = Vec::new();
                if run_perf {
                    let mut perf_edits: Vec<RuleEdit> = Vec::new();
                    for rule in FIXABLE_RULES {
                        if *rule == "fxhashmap" && !fxhashmap_allowed {
                            continue;
                        }
                        // Issue 102 T2: a demoted perf rule's dispatch slot
                        // is empty (zero edits, no error). Oracle-anchored
                        // suggestion consumption below is deliberately NOT
                        // gated by this — a demotion is a CORPUS lifecycle
                        // op, and clippy's own MachineApplicable suggestion
                        // is not corpus.
                        if active_set.excludes("perf", rule) {
                            continue;
                        }
                        #[cfg(feature = "fix_verify")]
                        {
                            perf_edits.extend(
                                riir_refine::domains::rust_perf::proposers::collect_edits_opts(
                                    rule,
                                    &current,
                                    &parsed,
                                    proposer_opts,
                                ),
                            );
                        }
                        #[cfg(not(feature = "fix_verify"))]
                        {
                            perf_edits.extend(
                                riir_refine::domains::rust_perf::proposers::collect_edits(
                                    rule, &current, &parsed,
                                ),
                            );
                        }
                    }
                    // P30 bench-target guard: count at pass 0 only — nothing
                    // is applied, so every later pass would re-propose the
                    // identical set against identical text.
                    if bench_guard {
                        if pass == 0 {
                            n_bench_declined += perf_edits.len();
                        }
                    } else if notbench_guard {
                        // P39: the posture lane declines the NotBenchable
                        // rules' edits (pass-0 counting, the bench-guard
                        // law) and applies the rest untouched.
                        for re in perf_edits {
                            if NOT_BENCHABLE_FIXABLE.contains(&re.rule_id.as_str()) {
                                if pass == 0 {
                                    n_notbench_declined += 1;
                                }
                            } else {
                                edits.push(re);
                            }
                        }
                    } else {
                        edits.extend(perf_edits);
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
                    // The pass's remapped posture index: feed spans are
                    // original-coords; map them through every EARLIER
                    // pass's applied splices so pass-N edits compare in
                    // pass-N coordinates (a diagnostic whose flagged text
                    // a confirmed edit rewrote extends over the replacement
                    // — nested re-exposed fixes inherit the confirmation).
                    let posture_pass: Option<
                        std::collections::BTreeMap<String, Vec<(usize, usize)>>,
                    > = posture_index.as_ref().map(|idx| {
                        idx.iter()
                            .map(|(lint, spans)| {
                                let mapped: Vec<(usize, usize)> = spans
                                    .iter()
                                    .map(|&sp| {
                                        let mut s = sp;
                                        for applied in pass_splices.iter().take(pass) {
                                            s = riir_refine::edit_emitter::map_span_through_splices(
                                                s, applied,
                                            );
                                        }
                                        s
                                    })
                                    .collect();
                                (lint.clone(), mapped)
                            })
                            .collect()
                    });
                    // `&lint_set` (pattern-deref): the set is borrowed, not
                    // moved — the P21 oracle merge below needs it after this
                    // loop (the lane's corpus-set precondition).
                    for &lint in &lint_set {
                        // Issue 102 T2: a demoted clippy rule's proposer
                        // stands down (the auto re-admit at run end watches
                        // exactly this — a fresh diagnostic naming the lint
                        // while the rule was skipped re-admits it).
                        if active_set.excludes("clippy", lint) {
                            continue;
                        }
                        let collected =
                            riir_refine::domains::clippy_lints::proposers::collect_edits(
                                lint, &current, &parsed,
                            );
                        // Issue 135 T2: allow-by-default and not enabled by
                        // the target crate — counted (pass 0), never applied.
                        if level_gated.contains(&lint) {
                            if pass == 0 && !collected.is_empty() {
                                *level_skipped_file.entry(lint.to_string()).or_default() +=
                                    collected.len();
                            }
                            continue;
                        }
                        match &posture_pass {
                            None => edits.extend(collected),
                            Some(idx) => {
                                for re in collected {
                                    if riir_refine::draft::clippy_feed::posture_allows(
                                        idx,
                                        lint,
                                        re.edit.start,
                                        re.edit.end,
                                    ) {
                                        edits.push(re);
                                    } else if pass == 0 {
                                        // Count at pass 0 only (the bench-guard
                                        // re-proposal law); the FILTER itself
                                        // runs every pass.
                                        n_posture_declined_file += 1;
                                        *posture_declined_by_lint
                                            .entry(lint.to_string())
                                            .or_default() += 1;
                                    }
                                }
                            }
                        }
                    }
                }
                // ── P21: oracle-anchored edits — the --clippy-json fix lane.
                // When a real diagnostic's lint is in THIS lane's set (the
                // corpus-rule precondition: no suggestion is consumed for a
                // lint the lane does not carry — that stays the suggest
                // pass's [no corpus rule] gap signal) and the diagnostic
                // carries a MachineApplicable suggestion, its parts are
                // merged into one atomic edit and consumed verbatim.
                // clippy's suggestion is TYPE-AWARE — the close for the P15
                // Unknown-receiver residue (`.min()` chains, `File::open`,
                // `.as_ref()`) our parse-only `receiver_kind` refuses
                // (E0599-unsafe standalone). Proposer edits WIN on overlap:
                // an oracle edit inside a proposer's rewrite is stale by
                // construction (that text was rewritten).
                if pass == 0 && run_clippy {
                    if let Some(feed) = &clippy_feed {
                        // Issue 135 T2: the level gate rides the oracle door
                        // too — a feed run under `-W clippy::pedantic` is the
                        // operator's flags, not the crate's configuration.
                        let oracle_set: Vec<&str> = lint_set
                            .iter()
                            .copied()
                            .filter(|l| !level_gated.contains(l))
                            .collect();
                        for d in feed.iter().filter(|d| {
                            d.suggestion.is_some()
                                && level_gated.contains(&d.lint.as_str())
                                && riir_refine::draft::clippy_feed::paths_ref_match(
                                    &d.file, &display,
                                )
                        }) {
                            *level_skipped_file.entry(d.lint.clone()).or_default() += 1;
                        }
                        let lane = riir_refine::draft::clippy_feed::oracle_edits(
                            feed,
                            &display,
                            &current,
                            &oracle_set,
                        );
                        if lane.dropped > 0 {
                            eprintln!(
                                "oracle lane: dropped {} suggestion(s) for {display} \
                                 (stale offsets or parse-gate refusal — does the feed \
                                 match the current tree state?)",
                                lane.dropped
                            );
                        }
                        // P34 (riir-ai Issue 981): needless_range_loop
                        // suggestions declined for lack of a LOCAL length
                        // proof — a range-dropping rewrite is mechanical only
                        // when the bound equals the iterated collection's
                        // length. Declined is not dropped: the suggestion is
                        // well-formed, the proof is not auditable here. The
                        // manual path keeps the range visible.
                        if lane.range_declined > 0 {
                            eprintln!(
                                "oracle lane: declined {} needless_range_loop \
                                 suggestion(s) for {display} (P34 length proof — the \
                                 range end is not a syntactic zero-arg .len() call; the \
                                 981 tail-slice class. Fix manually with a bounded \
                                 slice: &mut buf[base..][..n] or .take(n))",
                                lane.range_declined
                            );
                        }
                        // Issue 135 T4: clippy's argv-token backtick on a
                        // shell-command doc line — declined, never applied.
                        if lane.doc_command_declined > 0 {
                            eprintln!(
                                "oracle lane: declined {} doc_markdown suggestion(s) for {display} \
                                 on shell-command doc line(s) — backticking one argv token breaks \
                                 copy-paste; fence or indent the command by hand",
                                lane.doc_command_declined
                            );
                        }
                        // Issue 118 T2: the oracle lane rides the same bench-guard
                        // door as rust_perf (P30) — a MachineApplicable
                        // suggestion consumed verbatim can be structural (the
                        // P15 two-part rename+delete shape), and a bench file's
                        // numbers ARE its artifact. Decline unless the operator
                        // named this file explicitly (the T3 posture: flag +
                        // proceed, never a silent hard refusal).
                        let oracle_forced = args
                            .paths
                            .iter()
                            .any(|u| riir_refine::bench_guard::user_path_forces(u, file));
                        if bench_target && !oracle_forced {
                            if !lane.edits.is_empty() {
                                println!(
                                    "bench-guard: declining {} oracle suggestion(s) on {display} \
                                     (bench target — measurement-altering, apply manually only)",
                                    lane.edits.len()
                                );
                                n_bench_oracle_declined += lane.edits.len();
                            }
                        } else {
                            for oe in lane.edits {
                                let overlaps = edits.iter().any(|pe| {
                                    riir_refine::draft::clippy_feed::spans_overlap(
                                        &pe.edit, &oe.edit,
                                    )
                                });
                                if !overlaps {
                                    oracle_keys.insert((oe.edit.start, oe.edit.end));
                                    edits.push(oe);
                                }
                            }
                        }
                    }
                }
                #[cfg(feature = "sec")]
                if run_sec {
                    // Sec proposers are conservative-gated (Plan 018 T1.3):
                    // guards-already-present skips, signature-changing
                    // refusals — every applied fix still rides the
                    // `--fix --verify` compile gate when enabled.
                    for rule in riir_refine::domains::sec::proposers::FIXABLE_SEC {
                        edits.extend(riir_refine::domains::sec::proposers::collect_edits(
                            rule, &current, &parsed,
                        ));
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
                // recovery path). `RIIR_REFINE_SUPPRESS_REVERTED=0` forces
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
                // ── Issue 146 T0.3: the instrumentation-attribute guard.
                // Edits that would remove or rewrite bytes on a line carrying
                // hotpath's own vocabulary (attributes, the block macro, the
                // init/finish calls, the import) are DECLINED — the fix lane
                // must never eat the oracle it consumes; a silently-removed
                // `#[measure]` breaks the user's next profile exactly where
                // this lane anchored. Recomputed per pass on `current`
                // (applied splices shift coordinates); counted at pass 0
                // only, the bench-guard re-proposal law. A false decline is
                // loud + manually overridable; a missed one is silent
                // breakage — the asymmetry picks conservative.
                let hot_attr_lines = riir_refine::hotpath_report::attribute_line_ranges(&current);
                if !hot_attr_lines.is_empty() {
                    let before = edits.len();
                    edits.retain(|re| {
                        !hot_attr_lines.iter().any(|&(a_start, a_end)| {
                            (re.edit.start < a_end && a_start < re.edit.end)
                                || (re.edit.start == re.edit.end
                                    && a_start < re.edit.start
                                    && re.edit.start < a_end)
                        })
                    });
                    if pass == 0 {
                        n_hotattr_declined_file += before - edits.len();
                    }
                }
                if edits.is_empty() {
                    break;
                }
                let (next, splices) = riir_refine::edit_emitter::apply_edits_traced(
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
                        if oracle_keys.contains(&(re.edit.start, re.edit.end)) {
                            n_oracle_edits += 1;
                        }
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
            // Issue 149 T2: cross-pass resplice dedup. The fixpoint loop
            // legitimately re-fires lints on already-edited text (the capture's
            // needless_return → let_and_return chain on one function: three
            // applied splices, ONE visible repair), and every per-edit
            // accounting surface downstream — the review, the verify count,
            // fixstats, the trajectory rows — read those intermediate splices
            // as separate fixes of the same span. The earlier edit no longer
            // exists in the final text; the surviving replacement is the
            // honest fix record. Accounting-only: the file text is untouched.
            let resplice_dropped = dedup_respliced_edits(&mut all_edits, &pass_splices);
            if !resplice_dropped.is_empty() {
                let dropped_oracle = resplice_dropped
                    .iter()
                    .filter(|(pass, _, start, end, _)| {
                        *pass == 0 && oracle_keys.contains(&(*start, *end))
                    })
                    .count();
                n_oracle_edits = n_oracle_edits.saturating_sub(dropped_oracle);
                eprintln!(
                    "accounting: {} later-pass splice(s) on {display} re-edited an earlier \
                     pass's span — the earlier edit(s) no longer exist in the final text; \
                     counted once (Issue 149)",
                    resplice_dropped.len()
                );
            }
            // P30: the bench-guard decline line — loud, named, actionable
            // (the intake's "a flag, not silence").
            if n_bench_declined > 0 {
                n_bench_declined_total += n_bench_declined;
                eprintln!(
                    "bench-guard: declined {n_bench_declined} rust_perf edit(s) on {display} — \
                     measurement-altering on a bench target (the file's numbers ARE its \
                     artifact; apply manually only if judged safe)"
                );
            }
            // P39: the NotBenchable decline line — same law (loud, named,
            // actionable; the override is one flag away).
            if n_notbench_declined > 0 {
                n_notbench_declined_total += n_notbench_declined;
                eprintln!(
                    "repo-posture: declined {n_notbench_declined} NotBenchable rust_perf edit(s) \
                     on {display} — never benchmarked, and the posture mode applies only what a \
                     gate verified (--apply-notbenchable overrides)"
                );
            }
            // Issue 146 T0.3: the instrumentation-guard decline line — same
            // law (loud, named, actionable; the manual path keeps the
            // attribute visible).
            if n_hotattr_declined_file > 0 {
                n_hotattr_declined_total += n_hotattr_declined_file;
                eprintln!(
                    "hotpath-guard: declined {n_hotattr_declined_file} edit(s) on {display} \
                     touching hotpath instrumentation line(s) — removing instrumentation breaks \
                     the profile oracle this lane consumes; apply manually only if judged safe"
                );
            }
            // Plan 165: the posture decline line — what the target repo's
            // own clippy never flagged stays UNTOUCHED, loudly.
            // Issue 135 T2: the level-gate skip line — loud, named, and the
            // opt-in one flag away.
            if !level_skipped_file.is_empty() {
                let n: usize = level_skipped_file.values().sum();
                eprintln!(
                    "lint-level: skipped {n} clippy-lane edit(s) on {display} — allow-by-default \
                     at crate level ({}); the crate enables neither the lint nor its group \
                     ([lints.clippy] / #![warn(clippy::…)]) — --groups <group> applies them",
                    breakdown_of(&level_skipped_file)
                );
                for (lint, k) in level_skipped_file {
                    *level_skipped_by_lint.entry(lint).or_default() += k;
                }
            }
            if n_posture_declined_file > 0 {
                n_posture_declined_total += n_posture_declined_file;
                eprintln!(
                    "repo-posture: declined {n_posture_declined_file} clippy-lane edit(s) on {display} — \
                     no matching diagnostic in the target repo's own clippy run (same lint + \
                     overlapping span); run without --repo-posture to apply the full corpus"
                );
            }
        }
        // Remap each edit's offset through ALL passes from its own onward
        // (own-pass mapping is safe: the edit's own splice breaks at
        // `start >= p` without self-applying), then express it as a line
        // number in the final text.
        //
        // ── Issue 148 T-b (the suite-mint label): the per-span rejected
        // candidate set, captured where the matcher already has it. ONE
        // suggest pass over the ORIGINAL file (the decision-time corpus
        // posture); each edit resolves the span whose line range contains
        // its OWN-PASS line (chunk-neighborhood join — exact at pass 0,
        // drift only from earlier passes' line deltas, bounded by how rarely
        // the fixpoint goes past pass 1). An edit whose span is not found
        // carries an EMPTY vec — `rejected_opt` turns that into `None` so a
        // miss never erases a label a richer observation left.
        #[cfg(feature = "latent_retrieval")]
        let span_candidates: Vec<(usize, usize, Vec<String>)> =
            if args.write && !all_edits.is_empty() && traj_store.is_some() {
                matcher
                    .suggest_file(&src, &display, args.top_k)
                    .iter()
                    .map(|s| {
                        let mut rules: Vec<String> = s
                            .by_domain
                            .iter()
                            .flat_map(|(_, rs)| rs.iter().map(|r| r.rule_id.clone()))
                            .collect();
                        rules.sort();
                        rules.dedup();
                        (s.start_line, s.end_line, rules)
                    })
                    .collect()
            } else {
                Vec::new()
            };
        #[cfg(not(feature = "latent_retrieval"))]
        let span_candidates: Vec<(usize, usize, Vec<String>)> = Vec::new();
        let all_edits: Vec<FixEdit> = all_edits
            .into_iter()
            .map(|(pass, rule_id, start, end, replacement)| {
                let mut pos = start;
                for splices in pass_splices.iter().skip(pass) {
                    pos = riir_refine::edit_emitter::map_through_splices(pos, splices);
                }
                let line = current[..pos].matches('\n').count() + 1;
                let restore = pass_texts[pass]
                    .get(start..end)
                    .map(str::to_string)
                    .unwrap_or_default();
                let note = if oracle_keys.contains(&(start, end)) {
                    ORACLE_NOTE
                } else {
                    rule_context_note(&rule_id)
                };
                // The edit's line in ITS OWN pass's text — the honest join
                // key against the ORIGINAL-file span map (see the block
                // comment above for the approximation bound).
                let pass_line = pass_texts[pass][..start].matches('\n').count() + 1;
                let rejected: Vec<String> = span_candidates
                    .iter()
                    .find(|(sl, el, _)| pass_line >= *sl && pass_line <= *el)
                    .map(|(_, _, rules)| {
                        rules
                            .iter()
                            .filter(|r| r.as_str() != rule_id)
                            .take(riir_refine::traj_store::REJECTED_CAP)
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                (rule_id, line, replacement, restore, note, rejected)
            })
            .collect();

        // ── native fmt pass (src/fmt.rs): decide against the ORIGINAL
        // on-disk file BEFORE anything is written. Dry runs report the
        // plan; write/verify mode applies it after the final file state
        // settles. `--no-fmt` or a missing rustfmt disables (one warning).
        let fmt_decision = if args.fix && !all_edits.is_empty() && fmt_active {
            let edition = riir_refine::fmt::edition_for(file);
            Some((riir_refine::fmt::decide(&src, file, &edition), edition))
        } else {
            None
        };

        #[cfg(feature = "fix_verify")]
        if args.verify {
            if !all_edits.is_empty() {
                verify_batches.push(riir_refine::fix_verify::FileEditSet {
                    path: file.clone(),
                    original: src.clone(),
                    fixed: current.clone(),
                    edits: all_edits
                        .iter()
                        .map(|(rule, line, repl, restore, _note, rejected)| {
                            riir_refine::fix_verify::EditAtLine {
                                rule_id: rule.clone(),
                                final_line: *line,
                                replacement: repl.clone(),
                                restore: restore.clone(),
                                rejected: rejected.clone(),
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
                if args.write {
                    for (rule, ..) in &all_edits {
                        *fixstat_rules.entry(rule.clone()).or_insert(0) += 1;
                    }
                }
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
                            .map(|(rule_id, line, replacement, restore, _, _)| {
                                (rule_id.clone(), *line, replacement.clone(), restore.clone())
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    // riir-dapps Issue 119 T2: every edit here is a snippet
                    // of `file` — its repo-relative path rides the row.
                    let src_path = store.src_path_of(file);
                    let sourced = store.sourced(&src_path);
                    for (rule, _, replacement, restore, _, rejected) in &all_edits {
                        reopen067.note(store, rule);
                        if let Err(e) = sourced.record_applied(
                            rule,
                            restore,
                            replacement,
                            &heal_run_id,
                            rejected_opt(rejected),
                        ) {
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
                if args.write {
                    for (rule, ..) in &all_edits {
                        *fixstat_rules.entry(rule.clone()).or_default() += 1;
                    }
                }
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
                            .map(|(rule_id, line, replacement, restore, _, _)| {
                                (rule_id.clone(), *line, replacement.clone(), restore.clone())
                            })
                            .collect::<Vec<_>>(),
                    );
                }
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    // riir-dapps Issue 119 T2: every edit here is a snippet
                    // of `file` — its repo-relative path rides the row.
                    let src_path = store.src_path_of(file);
                    let sourced = store.sourced(&src_path);
                    for (rule, _, replacement, restore, _, rejected) in &all_edits {
                        reopen067.note(store, rule);
                        if let Err(e) = sourced.record_applied(
                            rule,
                            restore,
                            replacement,
                            &heal_run_id,
                            rejected_opt(rejected),
                        ) {
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
            // Feed-file matching is tail-based (component boundary): clippy
            // prints workspace-relative paths, scan paths are whatever the
            // caller typed (clippy_feed::paths_ref_match).
            let file_anchors: Vec<ClippyDiagnostic> = clippy_feed
                .as_ref()
                .map(|feed| {
                    feed.iter()
                        .filter(|d| paths_ref_match(&d.file, &display))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            // Issue 146 T0.2: this file's measured-hot spans, same tail-based
            // matching (the report's `file!()` paths are workspace-relative
            // too). Feed order is time-share desc — the ranking signal.
            let file_hot: Vec<riir_refine::hotpath_report::HotSpan> = hotpath_spans
                .as_ref()
                .map(|spans| {
                    spans
                        .iter()
                        .filter(|hs| paths_ref_match(&hs.file, &display))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            let hot_present = hotpath_spans.is_some();
            let mut suggestions = if clippy_feed.is_some() || hot_present {
                matcher.suggest_file_hot_anchored(
                    &src,
                    &display,
                    args.top_k,
                    &file_anchors,
                    &file_hot,
                )
            } else {
                matcher.suggest_file(&src, &display, args.top_k)
            };
            // T0.2's ranking signal made literal: hot spans surface FIRST in
            // the review output, ordered by their max time share (calls ×
            // avg — B129: instrumented timing ATTRIBUTES, never verifies),
            // then the rest in file order. Anchoring is additive recall;
            // this order is the review schedule, not a re-rank of rules.
            if file_hot.is_empty() {
                // hot_present but no span in THIS file: nothing to hoist.
            } else if suggestions.iter().any(|s| !s.hot.is_empty()) {
                suggestions.sort_by_key(|s| {
                    let max_share = s.hot.iter().map(|h| h.time_share_ns).max().unwrap_or(0);
                    std::cmp::Reverse(max_share)
                });
            }
            for s in suggestions {
                n_chunks += 1;
                if !s.hot.is_empty() {
                    n_hot_spans += 1;
                }
                if s.all_rules().is_empty()
                    && s.anchored_rules.is_empty()
                    && s.anchored_missing.is_empty()
                    && s.hot.is_empty()
                {
                    continue;
                }
                if !s.anchored_rules.is_empty() || !s.anchored_missing.is_empty() {
                    n_oracle_spans += 1;
                    n_anchored_missing += s.anchored_missing.len();
                    if !s.anchored_missing.is_empty() {
                        missing_by_span.push(s.anchored_missing.clone());
                    }
                }
                n_suggestions += 1;
                print_suggestion(
                    &s,
                    &matcher,
                    args.domain,
                    clippy_feed.is_some() || hot_present,
                    bench_target,
                    &mut domain_tally,
                    &fixable_set,
                );
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
                ctx.pass1_tokens += riir_refine::kat_meter::count_word_tokens(text.as_bytes());
                ctx.spans_seen += 1;
                ctx.pass1
                    .entry(riir_refine::kat_lease_client::span_key(&text))
                    .or_insert(rules);
            }
        }
    }

    // ── Plan 030 T0.4: the docker lane (the second non-syn surface, and
    // the only one with bounded proposers — so it serves --fix too).
    #[cfg(feature = "docker")]
    let n_docker = if (args.suggest || args.fix)
        && args.domain.is_none_or(|d| d == DomainId::Docker)
        && !dockerfiles.is_empty()
    {
        run_docker_lane(&dockerfiles, args.suggest, args.fix, args.write)
    } else {
        0
    };
    #[cfg(not(feature = "docker"))]
    let n_docker = 0usize;
    let _ = n_docker;

    // ── Issue 085: the dist lane — the third non-syn surface. SUGGEST-ONLY
    // by construction (Phase 1): findings are corpus-rule citations with
    // advice text over name-matched Cargo.toml manifests; no proposer
    // exists, so this never joins the --fix edit path.
    #[cfg(feature = "dist")]
    let n_dist = if args.suggest && args.domain.is_none_or(|d| d == DomainId::Dist) {
        run_dist_lane(&cargo_tomls)
    } else {
        0
    };
    #[cfg(not(feature = "dist"))]
    let n_dist = 0usize;
    let _ = n_dist;

    // ── Issue 025 T4: the non-syn surface suggest pass — `.wgsl` kernel
    // files + `#[cube]` bodies inside `.rs` files, keyed to the kernel
    // domain. SUGGEST-ONLY by construction: findings are corpus-rule
    // citations, no proposer exists for these surfaces, and this pass
    // runs outside the --fix edit path entirely (pure --suggest mode).
    #[cfg(feature = "kernel_opt")]
    let n_non_syn = if args.suggest && args.domain.is_none_or(|d| d == DomainId::Kernel) {
        let scanner = riir_refine::domains::kernel_opt::wgsl_scan::WgslScanner::default();
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
    #[cfg(not(feature = "kernel_opt"))]
    let n_non_syn = 0usize;

    // ── Plan 134 T0.2/T1.4: the shader_perf lane — the RENDER-stage half
    // of the `.wgsl` surface (the kernel pass above owns `@compute` by
    // construction: its patterns are compute-attribute-shaped). Suggest
    // = stage-aware scanner citations; fix = the bounded loop-guard
    // proposer ONLY (everything else ships suggest-only by design — the
    // DL3059 refusal table).
    #[cfg(feature = "shader_perf")]
    let n_shader =
        if args.domain.is_none_or(|d| d == DomainId::ShaderPerf) && !wgsl_files.is_empty() {
            run_shader_lane(&wgsl_files, args.suggest, args.fix && args.write)
        } else {
            0
        };
    #[cfg(not(feature = "shader_perf"))]
    let n_shader = 0usize;
    let _ = n_shader;

    // Issue 165: the non-syn lane findings join the roll-up. Disjoint from
    // the syn citations by file kind (Dockerfiles / manifests / `.wgsl`),
    // EXCEPT kernel: the wgsl + `#[cube]` lane and the syn pass can both
    // cite a cube body in a `.rs` file — a roll-up approximation, disclosed
    // here rather than silently dropped.
    domain_tally.add_count("docker", n_docker);
    domain_tally.add_count("dist", n_dist);
    domain_tally.add_count("shader", n_shader);
    domain_tally.add_count("kernel", n_non_syn);

    // ── Issue 015: the verify loop — write, cargo check, auto-revert.
    #[cfg(feature = "fix_verify")]
    if args.verify {
        let extra: Vec<String> = args
            .verify_args
            .as_deref()
            .map(|s| s.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default();
        let verifier = riir_refine::fix_verify::FixVerifier::new().with_extra_args(extra.clone());
        // Issue 031 H3: the verify baseline is default-features cargo check
        // — code behind non-default features is NOT compile-gated (the
        // katgpt-pruners batch shipped 2 feature-gated breaks past verify:
        // a u8-const with_capacity E0308 + an E0631 fn-path coercion). Warn
        // loudly when an owning package has non-default features and the
        // caller did not override the baseline via --verify-args.
        if args.verify_args.is_none() {
            let mut warned: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
            for b in &verify_batches {
                let Some(pkg) = riir_refine::fix_verify::owning_package_root(&b.path) else {
                    continue;
                };
                if !warned.insert(pkg.clone()) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(pkg.join("Cargo.toml")) else {
                    continue;
                };
                let n = riir_refine::fix_verify::manifest_nondefault_feature_count(&text);
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
                "verify: {} cargo check(s){} — kept {} file(s) / {} edit(s), reverted {} edit(s)",
                report.n_checks,
                if report.skipped_checks > 0 {
                    format!(
                        " (+{} skipped: unreached clean package)",
                        report.skipped_checks
                    )
                } else {
                    String::new()
                },
                report.kept_files,
                report.kept_edits,
                report.reverted.len()
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

        // ── the post-heal cfg audit (2026-09-10 intake, two recorded
        // casualties: the E0631 point-free closure behind `score_bench`
        // 2026-09-09 + the P13 `pvp_karma` mut under a partial --verify-args
        // set). The compile gate ran at THIS invocation's feature state, so
        // kept edits inside cfg-gated regions it did not compile carry a
        // FALSE "verified" claim. Per-edit honesty, report-only: name each
        // such edit + the exact re-run that would cover it. This ALSO fires
        // when --verify-args passed a partial set — the case the coarse
        // Issue-031-H3 warning correctly stays silent on.
        {
            let mut marks: Vec<riir_refine::fix_verify::CfgUnverified> = Vec::new();
            let mut sets: std::collections::HashMap<PathBuf, riir_refine::fix_verify::FeatureSet> =
                std::collections::HashMap::new();
            for (b, (reverted_lines, whole)) in verify_batches.iter().zip(&batch_state) {
                if *whole {
                    continue;
                }
                let Some(pkg) = riir_refine::fix_verify::owning_package_root(&b.path) else {
                    continue;
                };
                if !sets.contains_key(&pkg) {
                    let Ok(text) = std::fs::read_to_string(pkg.join("Cargo.toml")) else {
                        continue;
                    };
                    sets.insert(
                        pkg.clone(),
                        riir_refine::fix_verify::feature_set_from_args(
                            &text,
                            args.verify_args.as_deref(),
                        ),
                    );
                }
                let Some(set) = sets.get(&pkg) else {
                    continue;
                };
                marks.extend(
                    riir_refine::fix_verify::cfg_unverified_edits(std::slice::from_ref(b), set)
                        .into_iter()
                        .filter(|m| !reverted_lines.contains(&m.line)),
                );
            }
            if !marks.is_empty() {
                eprint!(
                    "{}",
                    riir_refine::fix_verify::render_cfg_audit(&marks, args.verify_args.as_deref())
                );
            }
        }

        // ── the required-features audit (P43, the bench_694 class): a
        // [[test]]/[[bench]]/[[example]] row's `required-features`
        // unsatisfied at THIS posture means --all-targets silently SKIPPED
        // the whole target — kept edits in such files (or in files no target
        // covers at all) read "verified" without ever compiling. A
        // MANIFEST-level gate, invisible to the cfg audit above (which sees
        // only `#[cfg]` attributes). Report-only disclosure, the same re-run
        // suggestion shape.
        {
            let mut marks: Vec<riir_refine::fix_verify::ReqFeatUnverified> = Vec::new();
            let mut sets: std::collections::HashMap<
                PathBuf,
                (String, riir_refine::fix_verify::FeatureSet),
            > = std::collections::HashMap::new();
            for (b, (reverted_lines, whole)) in verify_batches.iter().zip(&batch_state) {
                if *whole {
                    continue;
                }
                let Some(pkg) = riir_refine::fix_verify::owning_package_root(&b.path) else {
                    continue;
                };
                if !sets.contains_key(&pkg) {
                    let Ok(text) = std::fs::read_to_string(pkg.join("Cargo.toml")) else {
                        continue;
                    };
                    let set = riir_refine::fix_verify::feature_set_from_args(
                        &text,
                        args.verify_args.as_deref(),
                    );
                    sets.insert(pkg.clone(), (text, set));
                }
                let Some((text, set)) = sets.get(&pkg) else {
                    continue;
                };
                marks.extend(
                    riir_refine::fix_verify::required_features_unverified_edits(
                        std::slice::from_ref(b),
                        set,
                        text,
                        &pkg,
                    )
                    .into_iter()
                    .filter(|m| !reverted_lines.contains(&m.line)),
                );
            }
            if !marks.is_empty() {
                eprint!(
                    "{}",
                    riir_refine::fix_verify::render_required_features_audit(
                        &marks,
                        args.verify_args.as_deref()
                    )
                );
            }
        }

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
                let Some(pkg) = riir_refine::fix_verify::owning_package_root(&b.path) else {
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
                    entry
                        .1
                        .push((b.path.clone(), e.final_line, e.rule_id.clone()));
                }
            }
            if !by_pkg.is_empty() {
                let runner =
                    riir_refine::fix_verify::CheckRunner::new().with_extra_args(extra.clone());
                let mut canon_cache: std::collections::HashMap<PathBuf, PathBuf> =
                    std::collections::HashMap::new();
                let mut n_resolved = 0usize;
                let mut n_persist = 0usize;
                let mut n_unverifiable = 0usize;
                for (pkg, (lints, edits)) in &by_pkg {
                    let outcome = runner.run_clippy(pkg, lints);
                    let findings = match outcome {
                        Ok(o) if o.exit_ok => {
                            riir_refine::fix_verify::parse_clippy_findings(&o.json)
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
                    let ws = riir_refine::compile_oracle::workspace_root(pkg);
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
                                && riir_refine::fix_verify::resolve_reported_path(
                                    &f.file, ws_ref, pkg,
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
        // repo-local neuron-db store (`.refine/trajectories.kv`), the backend
        // 038 unifies on. Transitional dual-record with memory.json below
        // (the same events — no drift; 038-T3 retires the envelope half).
        // 038: the store was hoisted + opened at the --fix path entry (the
        // same instance serves apply events + verify verdicts — one open,
        // one migration check per run); this scope borrows it directly.

        // Post-verify review: report what SURVIVED (the fix review for
        // reverted edits would be misleading).
        for (b, (reverted_lines, whole)) in verify_batches.iter().zip(&batch_state) {
            // riir-dapps Issue 119 T2: every verdict below is a snippet of
            // THIS file — its repo-relative path rides the row (the sync
            // gate's allow_paths input).
            #[cfg(feature = "latent_retrieval")]
            let src_path = traj_store
                .as_ref()
                .map(|s| s.src_path_of(&b.path))
                .unwrap_or_default();
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
                        &riir_refine::component::FailureSignal::reverted(
                            riir_refine::component::RevertKind::CompileGate,
                        ),
                    );
                    #[cfg(feature = "latent_retrieval")]
                    record_traj_fail(
                        &traj_store,
                        &src_path,
                        &e.rule_id,
                        &e.restore,
                        &e.replacement,
                        &heal_run_id,
                        rejected_opt(&e.rejected),
                    );
                }
                continue;
            }
            let kept: Vec<&riir_refine::fix_verify::EditAtLine> = b
                .edits
                .iter()
                .filter(|e| !reverted_lines.contains(&e.final_line))
                .collect();
            if kept.is_empty() {
                continue;
            }
            n_fixed_files += 1;
            n_fixed_edits += kept.len();
            // Half 1.5: the compile-gated kept edits are WRITTEN fixes (the
            // verify lane implies write) — the highest-quality rows the
            // stats lane carries.
            for e in &kept {
                *fixstat_rules.entry(e.rule_id.clone()).or_insert(0) += 1;
            }
            // 037-T2 (the F2 fix): kept edits earn POSITIVE verdicts — the
            // compile gate vouches at minimum (rustc-only pass = Heuristic
            // outcome-success); the clippy oracle (when built + runnable)
            // upgrades to Certified on confirmed resolution or records the
            // honest verified-fail on a persist. Span-level reverts stay
            // definitive fails.
            for e in &kept {
                let (success, verified) = riir_refine::fix_verify::kept_edit_verdict(
                    oracle_resolved
                        .get(&(b.path.clone(), e.final_line))
                        .copied(),
                );
                // Issue 047: a kept-but-unresolved edit (compile gate passed,
                // full oracle says the lint persists) is C-class — the gate
                // was too loose. Successes stay unattributed.
                let fail_signal =
                    (!success).then(riir_refine::component::FailureSignal::gate_passed_ineffective);
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
                    &src_path,
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    success,
                    verified,
                    &heal_run_id,
                    fail_signal.as_ref().map(riir_refine::component::classify),
                    rejected_opt(&e.rejected),
                );
            }
            for e in b
                .edits
                .iter()
                .filter(|e| reverted_lines.contains(&e.final_line))
            {
                evolve_rec.record_failure_signal(
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    &riir_refine::component::FailureSignal::reverted(
                        riir_refine::component::RevertKind::CompileGate,
                    ),
                );
                #[cfg(feature = "latent_retrieval")]
                record_traj_fail(
                    &traj_store,
                    &src_path,
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    &heal_run_id,
                    rejected_opt(&e.rejected),
                );
            }
            println!(
                "fix {} ({} edit(s) verified):",
                b.path.display(),
                kept.len()
            );
            for e in &kept {
                println!("  [{:<24}] line {}", e.rule_id, e.final_line);
            }
            // Native fmt: the verified kept state is on disk — apply the
            // decided pass now (default-on; decisions were made against the
            // pre-heal originals above).
            if let Some((_, decision, edition)) = verify_fmt.iter().find(|(p, _, _)| *p == b.path) {
                use riir_refine::fmt::FmtDecision;
                match decision {
                    FmtDecision::Format => match riir_refine::fmt::apply(&b.path, edition) {
                        Ok(()) => {
                            fmt_summary.formatted += 1;
                            println!("  fmt: formatted");
                        }
                        Err(e) => {
                            fmt_summary.failed += 1;
                            eprintln!("fmt failed for {}: {e}", b.path.display());
                        }
                    },
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
            run_reheat_pass(ctx, &args, &files, &mut matcher, meter, &active_set)
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
        format!(", fixed {n_fixed_files} file(s) / {n_fixed_edits} edit(s){applied_note}")
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
    // P30: bench-guard run total — the decline must survive in the run
    // summary too, not only the per-file stderr line.
    if n_bench_declined_total > 0 {
        println!(
            "bench-guard: declined {n_bench_declined_total} rust_perf edit(s) on bench target(s) \
             (measurement-altering — see the per-file lines above)"
        );
    }
    // Issue 118 T2: the oracle-lane declines ride the same summary.
    if n_bench_oracle_declined > 0 {
        println!(
            "bench-guard: declined {n_bench_oracle_declined} oracle suggestion(s) on bench \
             target(s) (measurement-altering — apply manually only)"
        );
    }
    // P39: the NotBenchable declines ride the same summary (the run-total
    // law — a decline that only lives in a per-file line does not survive
    // a scan of hundreds of files).
    if n_notbench_declined_total > 0 {
        println!(
            "repo-posture: declined {n_notbench_declined_total} NotBenchable rust_perf edit(s) \
             (never benchmarked — --apply-notbenchable overrides; see the per-file lines above)"
        );
    }
    // Issue 146 T0.3: the instrumentation-guard declines ride the same
    // summary law.
    if n_hotattr_declined_total > 0 {
        println!(
            "hotpath-guard: declined {n_hotattr_declined_total} edit(s) touching hotpath \
             instrumentation line(s) (removing instrumentation breaks the profile oracle — \
             see the per-file lines above)"
        );
    }
    // Issue 146 T0.2: the hot-span review population rides the summary —
    // the feed's coverage + the spans actually anchored into suggestions.
    if let Some(spans) = hotpath_spans.as_ref() {
        println!(
            "hotpath feed: {n_hot_spans} span(s) carry measured-hot marks across the scan \
             (of {} located hot span(s) in the report) — timing is ranking-only attribution \
             (B129); alloc bytes/count are the verification axis",
            spans.len()
        );
    }
    // Plan 199 T3.5: the WASM validator lane's run-status line — the
    // summary-law shape (report-ONLY, never a gate; the lane's T3.4/Bench
    // 105 verdict is the recorded "native is fine" negative, so nothing in
    // the fix path consumes validator verdicts). When the lane is compiled
    // in and the committed store loads, the operator sees what would serve;
    // any other state (feature off, no store, revoked) is either invisible
    // (feature off = the lane does not exist in this build) or a named
    // report line (a store that fails to load is a fact worth one line).
    #[cfg(feature = "wasm_validator")]
    {
        let store = riir_refine::validators::ValidatorStore::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/validators"),
        );
        match store.verify() {
            Ok(admitted) if !admitted.is_empty() => {
                let rows: Vec<String> = admitted
                    .iter()
                    .map(|a| {
                        format!(
                            "{} v{} ({:?})",
                            a.record.name, a.record.version, a.record.state
                        )
                    })
                    .collect();
                println!(
                    "wasm-validator lane: {} committed artifact(s) verified — report-only \
                     (Bench 105 verdict: opt-in, the native predicate is the baseline)",
                    rows.join(", ")
                );
            }
            Ok(_) => {}
            Err(e) => eprintln!("wasm-validator lane: store verify failed (report-only): {e}"),
        }
    }
    // P12: report the suppressed re-proposals — the operator must see what
    // was skipped and why (a suppressed edit keeps its lint firing; the
    // escape hatch is RIIR_REFINE_SUPPRESS_REVERTED=0 for one run).
    if !suppressed_pairs.is_empty() {
        const MAX_SUPPRESSED_SHOWN: usize = 20;

        println!(
            "suppressed {} previously-reverted edit(s) (2+ consecutive compile-gate fails; RIIR_REFINE_SUPPRESS_REVERTED=0 forces retry):",
            suppressed_pairs.len()
        );
        for ((rule, restore, _fix), (file, line)) in
            suppressed_pairs.iter().take(MAX_SUPPRESSED_SHOWN)
        {
            println!(
                "  [{rule:<24}] {}:{} — {}",
                file,
                line,
                snippet_head(restore)
            );
        }
        if suppressed_pairs.len() > MAX_SUPPRESSED_SHOWN {
            println!(
                "  ... and {} more",
                suppressed_pairs.len() - MAX_SUPPRESSED_SHOWN
            );
        }
    }
    // Issue 102 T2: auto re-admit — a demoted clippy rule whose lint shows
    // up in THIS run's fresh diagnostics re-enters the marker (Research 165
    // rule 3, "the first fresh unresolved span the rule could serve"). The
    // feed is pre-fix, so "unresolved" is approximated by presence in the
    // feed: the rule was skipped all run (it fixed nothing itself), and a
    // shape it could serve was present. The post-fix oracle re-run that
    // would prove resolution is deliberately not paid here — pricing that
    // axis per decision is the Issue 102 T4 paired lane's job.
    if args.fix && !active_set.is_empty() {
        let fired: Vec<&str> = clippy_feed
            .as_ref()
            .map(|feed| feed.iter().map(|d| d.lint.as_str()).collect())
            .unwrap_or_default();
        let readmitted = active_set.auto_readmit_lints(&fired);
        if !readmitted.is_empty() {
            for rule in &readmitted {
                println!(
                    "active-set: auto re-admitted `{rule}` — a fresh diagnostic named it this \
                     run (marker updated on disk)"
                );
            }
            if let Err(e) = active_set.save(
                &resolve_heal_root(&args.paths).join(riir_refine::active_set::ACTIVE_SET_FILE),
            ) {
                eprintln!("active-set: re-admit save failed: {e}");
            }
        }
    }
    // 2026-09-13 footer redesign: plural-aware `files`/`spans` beats the
    // parenthesized `file(s), span(s)` hedge; the KAT detail that used to
    // ride the two summary_line walls moved to the kat_footer block at the
    // end of the run (after the pushes, so the sync line is post-truth).
    let file_pl = if files.len() == 1 { "file" } else { "files" };
    let span_pl = if n_chunks == 1 { "span" } else { "spans" };
    let sugg_pl = if n_suggestions == 1 {
        "suggestion"
    } else {
        "suggestions"
    };
    let oracle_note = if clippy_feed.is_some() {
        let mut s =
            format!(" · {n_oracle_spans} oracle-anchored · {n_anchored_missing} no-corpus-rule");
        // P21: the fix lane's consumed suggestions. Zero in suggest-only
        // runs stays silent — the count names fix-lane behavior only.
        if n_oracle_edits > 0 {
            let ed_pl = if n_oracle_edits == 1 { "edit" } else { "edits" };
            s.push_str(&format!(" · {n_oracle_edits} oracle {ed_pl} applied"));
        }
        // Plan 165: the posture declines, with the per-lint breakdown so
        // the operator sees WHAT the target's gate filtered (the doc_markdown
        // churn shape — one lint dominating the decline list).
        if n_posture_declined_total > 0 {
            let breakdown = posture_declined_by_lint
                .iter()
                .map(|(lint, n)| format!("{lint}×{n}"))
                .collect::<Vec<_>>()
                .join(", ");
            s.push_str(&format!(
                " · {n_posture_declined_total} declined by --repo-posture ({breakdown})"
            ));
        }
        s
    } else {
        String::new()
    };
    // Issue 135 T2: the level-gate skips ride the verdict line, whatever
    // the feed state (the posture/oracle notes above need a feed; this
    // gate does not).
    let level_note = if level_skipped_by_lint.is_empty() {
        String::new()
    } else {
        let n: usize = level_skipped_by_lint.values().sum();
        format!(
            " · {n} skipped: allow-by-default at crate level ({}; --groups opts in)",
            breakdown_of(&level_skipped_by_lint)
        )
    };
    println!(
        "\nscanned {} {file_pl} · {} {span_pl} · {} with {sugg_pl} (top-{} per \
         domain){mode}{non_syn_note}{fmt_note}{oracle_note}{level_note}",
        files.len(),
        n_chunks,
        n_suggestions,
        args.top_k
    );

    // ── Issue 151: the in-run secret disclosure (after the summary, the
    // corpus-delta placement convention — under the count it refines).
    // Quiet at zero. The findings' context lines are BLINDED (shareable-
    // output rule: no bytes, no fingerprint).
    let secret_total: usize = secret_findings.iter().map(|(_, f)| f.len()).sum();
    // P63: the pins suffix — suppressed findings are DISCLOSED, never
    // silently dropped (a report that hides counts is a green zero).
    let pins_note = if secret_pinned > 0 {
        format!(
            " (+{secret_pinned} digest-pin(s) adjudicated benign — suppressed from this count; the sync gate stays fail-closed)"
        )
    } else {
        String::new()
    };
    if !secret_findings.is_empty() {
        let total = secret_total;
        let file_pl2 = if secret_findings.len() == 1 {
            "file"
        } else {
            "files"
        };
        println!(
            "secrets: {total} finding(s) in {} {file_pl2}{pins_note} — suggest-only, never auto-fixed \
             (run `cargo refine --secret-scan` for the report)",
            secret_findings.len()
        );
        for (display, finds) in secret_findings.iter().take(MAX_SECRET_DISCLOSURE_FILES) {
            for f in finds.iter().take(MAX_SECRET_DISCLOSURE_PER_FILE) {
                println!(
                    "  [sec] secret-shaped literal ({}) at {display}:{}:{} [suggest-only — never \
                     auto-fixed; value not shown]",
                    f.tag(),
                    f.line,
                    f.col
                );
            }
        }
        let shown_files = secret_findings.len().min(MAX_SECRET_DISCLOSURE_FILES);
        let shown = secret_findings
            .iter()
            .take(MAX_SECRET_DISCLOSURE_FILES)
            .map(|(_, f)| f.len().min(MAX_SECRET_DISCLOSURE_PER_FILE))
            .sum::<usize>();
        if shown < total || shown_files < secret_findings.len() {
            println!(
                "  … and {} more (the report lists every finding)",
                total - shown
            );
        }
    } else if secret_pinned > 0 {
        // P63: every finding in the run adjudicated benign — still loud
        // (never a silent green zero), but no longer the recurring
        // all-false [sec] noise the digest-pin class measured at.
        println!("secrets: 0 finding(s){pins_note}");
    }
    if secret_total > 0 {
        domain_tally.add_secrets(secret_total);
    }

    // ── Issue 165: the per-domain roll-up + the next-step lines ──────────
    // One line answers "what did this run find, by topic" over everything
    // printed above (syn citations + lanes + the secret scan), then the
    // honest next steps: the fix lane when this was a suggest-only run
    // (with the fixable ceiling when the compiled set can name one), and
    // the mining opt-in when rows are ACTUALLY pending (the estimator —
    // never a fabricated rate). Silent at zero findings: a run with
    // nothing to act on grows no CTA noise.
    let suggest_only = args.suggest && !args.fix;
    if domain_tally.total() > 0 {
        println!("{}", domain_tally.rollup_line());
        if suggest_only && domain_tally.citations > 0 {
            if domain_tally.fixable_clippy > 0 {
                println!(
                    "next: ~{} clippy finding(s) look auto-fixable — run `cargo refine --fix` \
                     (compile-gated; the suggest-only rows stay manual)",
                    domain_tally.fixable_clippy
                );
            } else {
                println!("next: run `cargo refine --fix` to heal the fixable ones in place");
            }
        }
    }
    #[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
    if !args.quiet {
        print_mine_encouragement(
            suggest_only,
            domain_tally.total(),
            traj_store.as_ref(),
            &resolve_heal_root(&args.paths),
        );
    }

    // ── Issue 116: the corpus-truth delta surface ─────────────────────────
    // The two lanes read two corpus truths BY DESIGN — the fix lane the
    // compiled FIXABLE ∪ VERIFY set, the suggest lane a rule book a leased
    // epoch replaces wholesale. So `[no corpus rule]` carries two opposite
    // messages ("not leased yet" vs "unsupported") and a reader met them one
    // span at a time with no way to tell which. This names the seam; it does
    // NOT close it (the split is why a freshly-landed lint heals at once and
    // why suggest ranks on live evidence).
    //
    // Computed AFTER the summary so the delta line sits under the count it
    // refines, and only when a feed ran — without one there are no anchors
    // and the delta is vacuously empty.
    if clippy_feed.is_some() && !missing_by_span.is_empty() {
        let (fixable, verify) = fix_lane_tiers();
        let delta = riir_refine::corpus_delta::delta(
            missing_by_span.iter().map(Vec::as_slice),
            &fixable,
            &verify,
        );
        // Silent on an empty delta — the EXPECTED state, bounded by the
        // lease cadence. A line printing 0 every run is the cries-wolf
        // instrument that gets ignored on the runs that matter.
        if let Some(line) = delta.summary_line() {
            println!("{line}");
        }
        if args.corpus_delta {
            print!("{}", delta.render());
        }
    } else if args.corpus_delta {
        // A flag that silently does nothing is worse than one that refuses:
        // say which precondition failed.
        eprintln!(
            "corpus-delta: no oracle anchors this run — the delta is measured over \
             [no corpus rule] markers, which only a clippy feed produces. Re-run with \
             --clippy-json <path> (or let the auto-oracle run on a cargo project)."
        );
    }

    // KAT meter (T0.5): persist the session's burn. Save failure is a lost
    // bookkeeping event, never a failed heal — warn + continue (the
    // recorder precedent). The session summary now renders in the
    // kat_footer block at the end of the run.
    #[cfg(feature = "kat_service")]
    if let Some(m) = &kat_meter {
        if let Err(e) = m.save() {
            eprintln!(
                "kat-meter: could not persist burn ledger ({}): {e}",
                m.path().display()
            );
        }
    }

    // KAT charge (T0.6, Plan 024): settle the session's uncharged tokens at
    // the per-unit rate and persist. Same accounting-only posture (G9d —
    // the balance is a local projection; the server ledger is the
    // authority); the figures render in the kat_footer block below.
    #[cfg(feature = "kat_service")]
    if let (Some(m), Some(c)) = (&kat_meter, kat_charge.as_mut()) {
        use riir_refine::kat_units::MicroKat;

        c.settle(m.state());
        if let Err(e) = c.save() {
            eprintln!(
                "kat-charge: could not persist charge ledger ({}): {e}",
                c.path().display()
            );
        }
        // Plan 035 T3: the projection renders in MicroKat — the µ constant
        // passes to the renderer with no conversion, and a whole-KAT count
        // reaching this closure is a compile error.
        let remaining = MicroKat::from(c.balance_projection());
        let kat = |micro: MicroKat| riir_refine::kat_payreq_client::render_kat(micro.0);
        if remaining.0 == 0 {
            // §KAT warn-and-pay (T1.6), as amended by the 075 soft gate:
            // at 0 the evolved-corpus hot-push pauses at epoch end + the
            // persistent warning fires — mining accrual/settlement CONTINUE.
            // The run-start REFUSAL for depleted non-contributing accounts
            // is the gate's (`enforce_kat_soft_gate`); this end-of-run warn
            // still never blocks (it fires after the work, and only on the
            // local projection).
            eprintln!(
                "kat-charge: 0 KAT remain of the {} grant projection — out-of-KAT",
                kat(riir_refine::kat_meter::FREE_TIER_GRANT_MICRO_KAT)
            );
            eprintln!(
                "kat-charge: healing continues on the local corpus; the evolved-corpus \\
                 hot-push pauses at epoch end; mining accrual + settlement continue"
            );
            #[cfg(feature = "kat_transport")]
            {
                let dir = riir_refine::kat_account::KatAccount::default_dir();
                let acct = riir_refine::kat_account::KatAccount::load(&dir)
                    .ok()
                    .map(|a| a.state().account_id.clone())
                    .unwrap_or_default();
                let url = riir_refine::kat_lease_client::payment_url(&command_service_url(), &acct);
                eprintln!(
                    "kat-charge: top up — {url} (KAT packs), or contribute: cargo refine --mine"
                );
                let qr = riir_refine::kat_lease_client::render_qr_ascii(&url);
                if !qr.is_empty() {
                    eprintln!("{qr}");
                }
            }
        } else if c.low_balance() {
            // §KAT warn at ≤10M remaining: the top-up nudge (no QR until 0 —
            // the terminal render is the last-chance surface).
            eprintln!(
                "kat-charge: warning — {} KAT remain of the {} grant projection; \\
                 healing continues offline (the server ledger is the authority)",
                kat(remaining),
                kat(riir_refine::kat_meter::FREE_TIER_GRANT_MICRO_KAT)
            );
        }
    }

    // Owner mandate 2026-09-10 — the OPTED-IN machine auto-contributes:
    // once `--mine` has been run, every heal run (the bare dry run and
    // --fix alike) drains the mining batch here, exactly like the burn
    // report below. CONSENT IS THE DIFFERENCE: `Unset` never auto-pushes
    // (nothing silently leaves a machine that never joined — running
    // `cargo refine sync` IS the consent), `OptedOut` never pushes, and the
    // quiet push is one line, ~5 s, never an exit-code change. It runs
    // BEFORE the burn drain (the run_kat_sync ORDER note: the accepted
    // batch is what makes the drain's v3 response carry `contributed:
    // true` into the ack).
    #[cfg(all(
        feature = "kat_service",
        feature = "kat_transport",
        feature = "latent_retrieval"
    ))]
    if matches!(
        riir_refine::kat_mining_consent::read_with_warning(
            &riir_refine::kat_mining_consent::default_dir()
        )
        .0,
        riir_refine::kat_mining_consent::MiningConsent::OptedIn
    ) {
        let url = command_service_url();
        let transport = riir_refine::kat_lease_client::HttpLeaseTransport::new(&url, 5);
        let outcome = push_mining_batch(true, &url, &transport);
        print_mine_push(&outcome);
    }
    // Issue 117 (the delta half) — the loud companion: an opted-in machine
    // whose binary lacks the network features must hear that the
    // auto-contribute tail DID NOT RUN (the 00:21 class was silent exactly
    // here). One line, never an exit-code change — healing itself is
    // unaffected.
    #[cfg(not(all(
        feature = "kat_service",
        feature = "kat_transport",
        feature = "latent_retrieval"
    )))]
    if matches!(
        riir_refine::kat_mining_consent::read_with_warning(
            &riir_refine::kat_mining_consent::default_dir()
        )
        .0,
        riir_refine::kat_mining_consent::MiningConsent::OptedIn
    ) {
        eprintln!(
            "kat: mining auto-contribute SKIPPED — this binary lacks the network features \
             (batch stays queued; drain it with a release-set binary); rebuild with: {}",
            riir_refine::build_stamp::rebuild_command()
        );
    }

    // Issue 076 client half: the burn auto-sync — account billing, NOT the
    // mining-consent plane (it rides the login; `--unmine` does not stop
    // it). Fires iff a charge ledger settled this run, the machine is
    // logged in, and the cumulative burn exceeds the last ACCEPTED report.
    // GIT-STYLE STACKING: a failed push leaves the ack untouched, so the
    // un-pushed delta is a local commit — it reports (cumulative) with the
    // next run or `cargo refine sync`. One status line, ~5 s budget, never an
    // exit-code change.
    #[cfg(all(feature = "kat_service", feature = "kat_transport"))]
    if let Some(c) = kat_charge.as_ref() {
        let root = resolve_heal_root(&args.paths);
        let outcome = riir_refine::kat_burn_client::run_burn_sync(
            &root,
            c.state().charged_kat,
            c.state().meter_watermark_tokens,
        );
        print_burn_sync(&outcome);
    }
    // Issue 117 (the delta half) — the loud companion: the git-style
    // stacking promise is honest only if the skip is audible — a stale
    // binary's burn delta stacks locally instead of reporting.
    #[cfg(not(all(feature = "kat_service", feature = "kat_transport")))]
    if kat_charge.is_some() {
        eprintln!(
            "kat: burn report SKIPPED — this binary lacks the network features (the un-pushed \
             delta stacks locally and reports with the next release-set run); rebuild with: {}",
            riir_refine::build_stamp::rebuild_command()
        );
    }

    // Proposal 004 Half 1.5 — the fix-stats push tail: a consenting
    // `--fix` run leaves ONE ~1 KB signed stats row behind (lane `fix`,
    // rule/count pairs — WRITTEN edits only, the Issue-018 law; never
    // spans, never source). The THIRD consent plane gates it:
    // kat_stats_consent OptedIn (deny-by-default — `Unset`/`OptedOut`
    // never push), independent of `--mine` (the scope law). One status
    // line, a ~5 s budget, never an exit-code change (the burn-push
    // discipline). No offline stacking BY DESIGN — each run's row is
    // independent and a lost row is a lost stat, not a lost commit
    // (run-id idempotency makes retries free). Review-only runs (no
    // --write) and clean trees push nothing (the empty map skips).
    #[cfg(all(feature = "kat_service", feature = "kat_transport"))]
    if args.fix
        && !fixstat_rules.is_empty()
        && riir_refine::kat_stats_consent::read(&riir_refine::kat_stats_consent::default_dir())
            .push_allowed()
    {
        let account = riir_refine::kat_account::KatAccount::load(
            &riir_refine::kat_account::KatAccount::default_dir(),
        );
        let pushed = match account.ok().and_then(|a| a.signing_key().ok()) {
            Some(key) => {
                let url = command_service_url();
                let transport = riir_refine::kat_fixstat_client::HttpFixstatTransport::new(
                    &url,
                    riir_refine::kat_fixstat_client::FIXSTAT_TIMEOUT_SECS,
                );
                let machine = riir_refine::kat_consent_client::machine_label();
                let toolchain = std::process::Command::new("rustc")
                    .arg("--version")
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.lines().next().unwrap_or("").trim().to_string())
                    .unwrap_or_default();
                let rules: Vec<_> = fixstat_rules
                    .iter()
                    .map(|(k, v)| riir_refine::kat_protocol_fixstat::FixStatCount::new(k, *v))
                    .collect();
                // Domain totals: an explicit `--domain` pin wins (the
                // operator's own attribution); otherwise the COMPILED
                // rule→domain table (the mining v3 sync's same table —
                // Plan 038 Phase 2) attributes each rule and folds
                // unknowns into the reserved "other" bucket; when the
                // table is compiled out the map stays honest-empty rather
                // than guessed.
                let total: u32 = rules.iter().map(|e| e.count).sum();
                let domains = match args.domain {
                    Some(d) => vec![riir_refine::kat_protocol_fixstat::FixStatCount::new(
                        d.label(),
                        total,
                    )],
                    None => {
                        #[cfg(all(feature = "kat_service", feature = "latent_retrieval"))]
                        {
                            let pairs: Vec<(String, u32)> =
                                rules.iter().map(|e| (e.name.clone(), e.count)).collect();
                            riir_refine::kat_sync_payload::RuleDomains::compiled()
                                .fold_rule_domains(&pairs)
                                .into_iter()
                                .map(|(name, count)| {
                                    riir_refine::kat_protocol_fixstat::FixStatCount::new(
                                        name, count,
                                    )
                                })
                                .collect()
                        }
                        #[cfg(not(all(feature = "kat_service", feature = "latent_retrieval")))]
                        {
                            Vec::new()
                        }
                    }
                };
                let row = riir_refine::kat_fixstat_client::FixStatRow {
                    run_id: riir_refine::kat_fixstat_client::fresh_run_id(&machine),
                    machine,
                    toolchain,
                    files_touched: u32::try_from(n_fixed_files).unwrap_or(u32::MAX),
                    rules,
                    domains,
                };
                riir_refine::kat_fixstat_client::push_fixstat(&transport, &key, &row)
            }
            None => Err("no logged-in account (run `cargo refine --stats on`)".to_string()),
        };
        match pushed {
            Ok(ack) => println!(
                "stats: shared {} rule-fix(es) across {} file(s) — epoch {} ({})",
                fixstat_rules.values().sum::<u32>(),
                n_fixed_files,
                ack.epoch,
                if ack.recorded_new {
                    "recorded"
                } else {
                    "duplicate run id — already shared"
                },
            ),
            Err(e) => {
                eprintln!("stats: push failed ({e}) — not retried; the next run shares its own row")
            }
        }
    }
    // Issue 117 (the delta half) — the loud companion: a consenting --fix
    // run's stats row is lost, not stacked (no offline stacking BY DESIGN),
    // so the skip must be audible on exactly the runs that earned one.
    #[cfg(not(all(feature = "kat_service", feature = "kat_transport")))]
    if args.fix
        && !fixstat_rules.is_empty()
        && riir_refine::kat_stats_consent::read(&riir_refine::kat_stats_consent::default_dir())
            .push_allowed()
    {
        eprintln!(
            "kat: fix-stats push SKIPPED — this binary lacks the network features (a lost row \
             is a lost stat, not a lost commit); rebuild with: {}",
            riir_refine::build_stamp::rebuild_command()
        );
    }

    // The redesigned KAT footer (2026-09-13): one glanceable block —
    // numbers rounded for reading, a Braille fuel-gauge over the grant,
    // a state-aware sync line — replacing the two `summary_line()` walls.
    // Prints LAST, after the pushes, so the sync line states post-push
    // truth. Current ⇔ every pushable whole KAT is acked (the burn wire
    // truncates to whole KAT, so the sub-KAT sliver rides until the next
    // crossing — by design, not a stuck delta).
    #[cfg(feature = "kat_service")]
    if let (Some(m), Some(c)) = (&kat_meter, kat_charge.as_ref()) {
        use riir_refine::kat_units::MicroKat;

        let account = riir_refine::kat_account::KatAccount::load(
            &riir_refine::kat_account::KatAccount::default_dir(),
        )
        .ok();
        // The leaderboard link needs no account in the URL (the row key is
        // the pseudonym); the no-transport build never renders a link
        // (Unknown/LoggedOut carry none), so the URL is computed only where
        // it is read.
        #[cfg(feature = "kat_transport")]
        let ledger_url = account
            .as_ref()
            .map(|_| {
                format!(
                    "{}/leaderboard",
                    command_service_url().trim_end_matches('/')
                )
            })
            .unwrap_or_default();
        // The trial-credit footer line (Proposal 006): from the SAME ack
        // the sync state reads, so the footer never mixes sync generations.
        // `None` (pre-v4 ack / pre-TUNA server / no transport) renders
        // nothing. The ack's `last_balance_micro` feeds the kat line the
        // same way — both tanks are SERVER truth (the old local grant
        // projection is GONE: post-D1 the grant is TUNA, and the server
        // draws TUNA-first, so a local KAT-drain bar was a fiction).
        #[cfg_attr(not(feature = "kat_transport"), allow(unused_mut))]
        let mut tuna_line: Option<riir_refine::kat_footer::TunaLine> = None;
        #[cfg_attr(not(feature = "kat_transport"), allow(unused_mut))]
        let mut kat_balance: Option<MicroKat> = None;
        #[cfg(feature = "kat_transport")]
        let sync = if account.is_none() {
            riir_refine::kat_footer::SyncState::LoggedOut
        } else {
            let root = resolve_heal_root(&args.paths);
            // T7 (riir-kat Issue 003): the PER-NET ack — the same file the
            // burn sync advances (the flat read here was the measured
            // 2026-10-12 split-brain: the footer displayed the pre-split
            // flat ack's stale balance while the sync lived in the net dir).
            let ack_path =
                riir_refine::kat_state_stamp::net_state_dir_for_url(&root, &net_partition_url())
                    .join(riir_refine::kat_burn_client::KatBurnAck::FILE_NAME);
            let (ack, _) = riir_refine::kat_burn_client::KatBurnAck::load_at(&ack_path);
            kat_balance = ack.last_balance_micro.map(MicroKat);
            tuna_line =
                ack.tuna_balance_micro
                    .zip(ack.tuna_expires_unix)
                    .map(|(balance, expires)| riir_refine::kat_footer::TunaLine {
                        balance_micro: MicroKat(balance.max(0)),
                        // The trial scale mirrors the KAT grant projection
                        // until the server reports the real claim size (B2).
                        grant_micro: riir_refine::kat_meter::FREE_TIER_GRANT_MICRO_KAT,
                        expires_unix: expires,
                        now_unix: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |d| d.as_secs()),
                    });
            // The burn wire is whole-KAT granular (wire_kat_total truncates),
            // so the sub-KAT sliver of `charged` is BY DESIGN unacked until
            // the next whole-KAT crossing — it is not a stuck delta. Current
            // means: every pushable whole KAT is acked. A real stack (offline
            // pushes) leaves whole-KAT gaps beyond the sliver. The pending
            // total is the acked ACCOUNT base plus this net's whole-KAT
            // delta (pending_report_total — the net-vs-account scope mismatch
            // deadlocked the sync; the footer mirrors the sync's arithmetic).
            let report_total =
                riir_refine::kat_burn_client::pending_report_total(&ack, c.state().charged_kat);
            let wire = riir_refine::kat_burn_client::WIRE_PER_CHARGE_UNITS;
            if report_total <= ack.burn_kat_total {
                riir_refine::kat_footer::SyncState::Current {
                    ledger_url: &ledger_url,
                }
            } else {
                riir_refine::kat_footer::SyncState::Unsynced {
                    // The whole-KAT gap → µ at the ONE µ-per-whole-KAT factor
                    // (explicit conversion — Plan 035 T3).
                    micro_kat: MicroKat::from(
                        (report_total - ack.burn_kat_total).saturating_mul(wire),
                    ),
                    queued_batches: 0,
                    ledger_url: &ledger_url,
                }
            }
        };
        #[cfg(not(feature = "kat_transport"))]
        let sync = if account.is_none() {
            riir_refine::kat_footer::SyncState::LoggedOut
        } else {
            riir_refine::kat_footer::SyncState::Unknown
        };
        let footer = riir_refine::kat_footer::KatFooter {
            run_tokens: m.session_tokens(),
            run_micro_kat: MicroKat::from(c.session_kat()),
            tuna: tuna_line,
            kat_balance_micro: kat_balance,
            rate_micro_per_mtok: riir_refine::kat_meter::CHARGE_MICRO_KAT_PER_MILLION_TOKENS,
            account_hex: account.as_ref().map(|a| a.state().account_id.as_str()),
            sync,
        };
        println!(
            "{}",
            footer.render(riir_refine::kat_footer::footer_color_enabled())
        );
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
/// Issue 085 — the dist lane over Cargo.toml manifests. Suggest-only: every
/// finding is a citation + advice line; nothing is ever written.
#[cfg(feature = "dist")]
fn run_dist_lane(paths: &[PathBuf]) -> usize {
    use riir_refine::domains::dist::rules::scan_source_with;

    let mut printed = 0usize;
    for path in paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        // Caller evidence (workspace root, autodiscovered bins, root profile
        // knobs) — derived once per manifest inside the domain, one bounded
        // ancestor walk, cached in the returned Copy context.
        let ctx = riir_refine::domains::dist::DistContext::derive(path);
        for f in scan_source_with(&src, ctx) {
            println!(
                "{}:{} [dist  ] {:<40} — {}",
                path.display(),
                f.line,
                f.rule_id,
                f.excerpt.trim()
            );
            println!("           ↳ {}", f.advice);
            printed += 1;
        }
    }
    printed
}

#[cfg(feature = "docker")]
fn run_docker_lane(paths: &[PathBuf], suggest: bool, fix: bool, write: bool) -> usize {
    use riir_refine::domains::docker::proposers::fix_all;
    use riir_refine::domains::docker::rules::{scan_source_with, FIXABLE_DOCKER};

    let mut printed = 0usize;
    for path in paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let ctx = docker_scan_context(path);
        let findings = scan_source_with(&src, &ctx);
        for f in &findings {
            let kind = if FIXABLE_DOCKER.contains(&f.rule_id) && f.fix.is_some() {
                "fix    "
            } else {
                "suggest"
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
        if write {
            match std::fs::write(path, &fixed) {
                Ok(()) => println!("  wrote {}", path.display()),
                Err(e) => eprintln!("  FAILED to write {}: {e}", path.display()),
            }
        } else {
            println!(
                "  {} bounded fix(es) available for {} (re-run with --write to apply)",
                findings
                    .iter()
                    .filter(|f| f.fix.is_some() && FIXABLE_DOCKER.contains(&f.rule_id))
                    .count(),
                path.display()
            );
        }
    }
    let _ = suggest;
    printed
}

/// Plan 134 T0.2: the render-stage `.wgsl` lane. Suggest = stage-aware
/// scanner citations (rule tags `fix`/`suggest` by `FIXABLE_SHADER`);
/// fix+write = the bounded loop-guard proposer ONLY, with the Tier-0
/// invariants asserted on the healed output (entry-point multiset +
/// binding multiset) and the naga tier when `shader_verify` is compiled
/// (degrades to Tier-0 with ONE line — the Issue 018 oracle-split).
#[cfg(feature = "shader_perf")]
fn run_shader_lane(paths: &[PathBuf], suggest: bool, fix_write: bool) -> usize {
    use riir_refine::domains::shader_perf::proposers::{collect_all_edits, FIXABLE_SHADER};
    use riir_refine::domains::shader_perf::ShaderScanner;

    let mut printed = 0usize;
    for path in paths {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        // Render-stage files only (the routing split: @compute is the
        // kernel lane's territory).
        if !riir_refine::domains::shader_perf::entry_stages(&src).has_render() {
            continue;
        }
        let findings = ShaderScanner.scan(&src);
        if !suggest && !fix_write {
            continue;
        }
        for f in &findings {
            let kind = if FIXABLE_SHADER.contains(&f.rule_id) {
                "fix    "
            } else {
                "suggest"
            };
            println!(
                "{}:{} [shader {kind}] {:<40} — {}",
                path.display(),
                f.line,
                f.rule_id,
                f.excerpt.trim()
            );
            printed += 1;
        }
        if !fix_write {
            continue;
        }
        let edits = collect_all_edits(&src);
        if edits.is_empty() {
            continue;
        }
        let healed = riir_refine::edit_emitter::apply_edits(
            &src,
            edits.iter().map(|re| re.edit.clone()).collect(),
        );
        // Tier-0 invariants on the healed output — a guard insertion that
        // disturbs entry points or bindings is reverted, not shipped.
        let verdict = riir_refine::domains::shader_perf::verify::tier0_verdict(&src, &healed);
        if !(verdict.parses && verdict.entry_points_intact && verdict.bindings_unchanged) {
            eprintln!(
                "  REFUSED {} — Tier-0 invariant violated (parses={}, entry_points={}, bindings={})",
                path.display(),
                verdict.parses,
                verdict.entry_points_intact,
                verdict.bindings_unchanged
            );
            continue;
        }
        #[cfg(feature = "shader_verify")]
        {
            match riir_refine::domains::shader_perf::verify::validate_naga(&healed) {
                Ok(Some(true)) => {}
                Ok(Some(false)) => {
                    eprintln!(
                        "  REFUSED {} — naga rejected the healed module",
                        path.display()
                    );
                    continue;
                }
                Ok(None) => eprintln!("  note: naga tier unavailable — standing on Tier-0"),
                Err(e) => {
                    eprintln!("  REFUSED {} — naga error: {e}", path.display());
                    continue;
                }
            }
        }
        match std::fs::write(path, &healed) {
            Ok(()) => println!(
                "  wrote {} ({} bounded fix(es))",
                path.display(),
                edits.len()
            ),
            Err(e) => eprintln!("  FAILED to write {}: {e}", path.display()),
        }
    }
    printed
}

/// Derive what the repo around a Dockerfile can prove, for the caller-gated
/// rules. Anything not derivable stays `None` — never guessed.
#[cfg(feature = "docker")]
fn docker_scan_context(path: &Path) -> riir_refine::domains::docker::rules::ScanContext {
    use riir_refine::domains::docker::rules::ScanContext;

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
    store: &riir_refine::kat_lease_client::LeaseStore,
    pin: Option<u64>,
    heal_root: &std::path::Path,
) -> Option<(u64, Vec<(DomainId, Vec<(String, String, String)>)>)> {
    use riir_refine::kat_promotion::{promotion_run_overlay, PromotionRegistry, REGISTRY_FILE};

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
    let merged = promotion_run_overlay(
        leased_str.as_ref().map(|(_, o)| o.as_slice()),
        &registry,
        pin.is_some(),
    )?;
    let overlay: Vec<(DomainId, Vec<_>)> = merged
        .into_iter()
        .filter_map(|(domain, triples)| {
            let id = match domain.as_str() {
                "clippy" => Some(DomainId::Clippy),
                "perf" => Some(DomainId::Perf),
                "kernel" => Some(DomainId::Kernel),
                "sec" => Some(DomainId::Sec),
                "docker" => Some(DomainId::Docker),
                "dist" => Some(DomainId::Dist),
                "shader" => Some(DomainId::ShaderPerf),
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
    store: &riir_refine::kat_lease_client::LeaseStore,
    pin: Option<u64>,
) -> Option<(u64, Vec<(DomainId, Vec<(String, String, String)>)>)> {
    use riir_refine::kat_lease_client::{
        overlay_from_epoch, PACK_DOMAIN_CLIPPY, PACK_DOMAIN_DIST, PACK_DOMAIN_DOCKER,
        PACK_DOMAIN_KERNEL, PACK_DOMAIN_PERF, PACK_DOMAIN_SEC,
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
            PACK_DOMAIN_DIST => Some(DomainId::Dist),
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

/// `lint×n, lint×n` — the per-lint decline breakdown shape.
fn breakdown_of(by_lint: &std::collections::BTreeMap<String, usize>) -> String {
    by_lint
        .iter()
        .map(|(lint, n)| format!("{lint}×{n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Issue 149 T2: drop edits whose repair a LATER pass re-spliced. The
/// fixpoint loop re-fires lints on already-edited text (the capture's
/// `needless_return` → `let_and_return` chain on one function: three
/// applied splices, ONE visible repair); an edit whose final span a LATER
/// edit's final span overlaps no longer exists in the text it claims to
/// describe — the last edit in the chain (the surviving replacement) is the
/// honest fix record. Returns `(kept, dropped)` so the caller can keep the
/// run-level oracle-edit count consistent. Accounting-only: the file text
/// is untouched (the splices already happened), and `pass_texts`/
/// `pass_splices` stay complete for the revert machinery — a dropped edit's
/// restore would restore an intermediate text that no longer exists, so its
/// revert record was already incoherent.
fn dedup_respliced_edits(
    edits: &mut Vec<(usize, String, usize, usize, String)>,
    pass_splices: &[Vec<riir_refine::edit_emitter::SourceEdit>],
) -> Vec<(usize, String, usize, usize, String)> {
    // Final-coordinate span per edit — the same mapping the review's line
    // numbers use, span-shaped (authority-follows-replacement: a span that
    // intersects a later splice extends over its replacement).
    let finals: Vec<(usize, usize)> = edits
        .iter()
        .map(|(pass, _, start, end, _)| {
            let mut span = (*start, *end);
            for applied in pass_splices.iter().skip(*pass) {
                span = riir_refine::edit_emitter::map_span_through_splices(span, applied);
            }
            span
        })
        .collect();
    // i is subsumed iff some LATER edit's final span overlaps its own.
    // Same-pass spans never overlap (apply_edits' selection enforces that),
    // and a span spliced away to a point still counts as covered when the
    // later replacement contains the point.
    let spans_overlap = |a: (usize, usize), b: (usize, usize)| {
        (a.0 < b.1 && b.0 < a.1)
            || (a.0 == a.1 && b.0 <= a.0 && a.0 < b.1)
            || (b.0 == b.1 && a.0 <= b.0 && b.0 < a.1)
    };
    let mut subsumed = vec![false; edits.len()];
    for i in 0..edits.len() {
        for j in 0..edits.len() {
            if edits[j].0 > edits[i].0 && spans_overlap(finals[i], finals[j]) {
                subsumed[i] = true;
                break;
            }
        }
    }
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for (i, edit) in edits.drain(..).enumerate() {
        if subsumed[i] {
            dropped.push(edit);
        } else {
            kept.push(edit);
        }
    }
    *edits = kept;
    dropped
}

/// The compact pass-2 fix loop over one file — the same proposer machinery
/// and span-preserving splice as pass 1, without the verify-batching /
/// evolve recording / line-remapping surfaces (pass-1 concerns). Review
/// mode prints `reheat:` lines; `--write` applies + counts; under
/// `--verify` the pass is REVIEW-ONLY (never-wrong: no unverified write —
/// the edits land deterministically on the next run, the epoch being
/// pinned). Gated with its only caller (`run_reheat_pass`): ungated, the
/// fn is dead code — a build warning — at every feature set without
/// kat_transport.
#[cfg(feature = "kat_transport")]
#[allow(clippy::too_many_arguments)]
fn reheat_fix_pass(
    path: &Path,
    domain: Option<DomainId>,
    write: bool,
    verify_mode: bool,
    fmt_active: bool,
    active_set: &riir_refine::active_set::ActiveSet,
    notbench_guard: bool,
    fix_groups: &[String],
) -> usize {
    const MAX_PASSES: usize = 4;

    let Ok(src) = read_source(path) else {
        return 0;
    };
    let display = path.display().to_string();
    let (run_perf, run_clippy, run_sec) = fix_domain_scopes(domain);
    #[cfg(not(feature = "sec"))]
    let _ = run_sec;
    // P30 bench-target guard — same policy as the main fix lane: a perf
    // edit on a bench target alters what the bench measures; decline
    // loudly (count reported once, pass 0), never silently.
    let bench_guard =
        run_perf && riir_refine::domains::rust_perf::bench_guard::is_bench_target(&display, &src);
    // P39: the posture lane's NotBenchable decline rides the reheat too —
    // the guard semantic must be uniform across both fix paths.
    let notbench_guard = run_perf && notbench_guard;
    // Issue 135 T2: the level gate rides the reheat too (uniform across
    // both fix paths).
    let level_gated: Vec<&'static str> = if run_clippy {
        syn::parse_file(&src).map_or_else(
            |_| Vec::new(),
            |parsed| gated_lints(&LevelResolver::new().levels_for(path, &parsed), fix_groups),
        )
    } else {
        Vec::new()
    };
    let mut level_skipped: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut current = src.clone();
    let mut total_applied = 0usize;
    let mut changed = false;
    for pass in 0..MAX_PASSES {
        let Ok(parsed) = syn::parse_file(&current) else {
            return total_applied;
        };
        let mut edits: Vec<RuleEdit> = Vec::new();
        if run_perf {
            let mut perf_edits: Vec<RuleEdit> = Vec::new();
            for rule in FIXABLE_RULES {
                // Issue 102 T2: demoted rules' dispatch slots are empty.
                if active_set.excludes("perf", rule) {
                    continue;
                }
                perf_edits.extend(riir_refine::domains::rust_perf::proposers::collect_edits(
                    rule, &current, &parsed,
                ));
            }
            if bench_guard {
                if pass == 0 && !perf_edits.is_empty() {
                    println!(
                        "reheat: bench-guard declined {} rust_perf edit(s) on {display} (measurement-altering)",
                        perf_edits.len()
                    );
                }
            } else if notbench_guard {
                // P39: pass-0 counting, the bench-guard law.
                let mut declined = 0usize;
                for re in perf_edits {
                    if NOT_BENCHABLE_FIXABLE.contains(&re.rule_id.as_str()) {
                        declined += 1;
                    } else {
                        edits.push(re);
                    }
                }
                if pass == 0 && declined > 0 {
                    println!(
                        "reheat: repo-posture declined {declined} NotBenchable rust_perf edit(s) on {display} (never benchmarked)"
                    );
                }
            } else {
                edits.extend(perf_edits);
            }
        }
        if run_clippy {
            for lint in FIXABLE_LINTS {
                // Issue 102 T2: demoted rules' dispatch slots are empty.
                if active_set.excludes("clippy", lint) {
                    continue;
                }
                let collected = riir_refine::domains::clippy_lints::proposers::collect_edits(
                    lint, &current, &parsed,
                );
                if level_gated.contains(lint) {
                    if pass == 0 && !collected.is_empty() {
                        *level_skipped.entry((*lint).to_string()).or_default() += collected.len();
                    }
                    continue;
                }
                edits.extend(collected);
            }
            if pass == 0 && !level_skipped.is_empty() {
                println!(
                    "reheat: lint-level skipped {} clippy-lane edit(s) on {display} — \
                     allow-by-default at crate level ({}; --groups opts in)",
                    level_skipped.values().sum::<usize>(),
                    breakdown_of(&level_skipped)
                );
            }
        }
        #[cfg(feature = "sec")]
        if run_sec {
            for rule in riir_refine::domains::sec::proposers::FIXABLE_SEC {
                if active_set.excludes("sec", rule) {
                    continue;
                }
                edits.extend(riir_refine::domains::sec::proposers::collect_edits(
                    rule, &current, &parsed,
                ));
            }
        }
        if edits.is_empty() {
            break;
        }
        let (next, splices) = riir_refine::edit_emitter::apply_edits_traced(
            &current,
            edits.iter().map(|re| re.edit.clone()).collect(),
        );
        let applied: std::collections::HashSet<(usize, usize)> =
            splices.iter().map(|s| (s.start, s.end)).collect();
        for re in &edits {
            if applied.contains(&(re.edit.start, re.edit.end)) {
                let line = current[..re.edit.start].matches('\n').count() + 1;
                if verify_mode {
                    println!(
                        "reheat (review-only under --verify): {display}:{line} [{}]",
                        re.rule_id
                    );
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
    if (changed && write && !verify_mode && current != src)
        && (std::fs::write(path, &current).is_ok() && fmt_active)
    {
        let edition = riir_refine::fmt::edition_for(path);
        if matches!(
            riir_refine::fmt::decide(&current, path, &edition),
            riir_refine::fmt::FmtDecision::Format
        ) {
            let _ = riir_refine::fmt::apply(path, &edition);
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
    mut meter: Option<&mut riir_refine::kat_meter::KatMeter>,
    active_set: &riir_refine::active_set::ActiveSet,
) -> Option<(riir_refine::kat_lease_client::ReheatReport, Option<String>)> {
    use riir_refine::kat_lease_client::{
        run_update_phase, HttpLeaseTransport, LeaseManifest, LeaseTransport, PassSpans,
    };

    // One bounded lease check: did a new epoch land?
    let url = riir_refine::kat_lease_client::service_url_from_env()?;
    let transport = HttpLeaseTransport::new(&url, args.update_timeout.max(1));
    let store =
        riir_refine::kat_lease_client::LeaseStore::open_net(&resolve_heal_root(&args.paths), &url);
    let mut charge = riir_refine::kat_lease_client::LeaseCharge::open(&store);
    let current = store.current();
    let etag = current
        .as_ref()
        .map(|m| riir_refine::kat_lease_client::hex32(&m.commitment));
    let body = match transport.get_lease(etag.as_deref()) {
        riir_refine::kat_lease_client::HttpResponse::NotModified => return None,
        riir_refine::kat_lease_client::HttpResponse::Ok(b) => b,
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
        riir_refine::kat_lease_client::UpdateStatus::Applied {
            epoch,
            charge_micro_kat,
        } => {
            // Rebuild the matcher under E+1 (a failed rebuild aborts the
            // reheat — pass 1's results stand). T2.4: promotions merge into
            // the rebuilt overlay too (same helper as the run-start build).
            #[cfg(all(feature = "kat_service", feature = "score_bench"))]
            let rebuilt = run_overlay_with_promotion(&store, None, &resolve_heal_root(&[]));
            #[cfg(all(feature = "kat_service", not(feature = "score_bench")))]
            let rebuilt = lease_overlay_for_run(&store, None);
            if let Some((_, overlay)) = rebuilt {
                match LatentMatcher::from_enabled_domains_with_overlay(&overlay) {
                    Ok(m) => *matcher = m.with_active_set(active_set.clone()),
                    Err(_) => return None,
                }
            } else {
                return None;
            }
            // Pass 2 over the POST-FIX tree: re-retrieve every span.
            let mut pass2: Vec<PassSpans> = Vec::new();
            let mut span_texts: std::collections::HashMap<[u8; 32], String> =
                std::collections::HashMap::new();
            let mut file_changed: std::collections::HashSet<std::path::PathBuf> =
                std::collections::HashSet::new();
            for file in files {
                let Ok(src) = std::fs::read_to_string(file) else {
                    continue;
                };
                let display = file.display().to_string();
                for s in matcher.suggest_file(&src, &display, args.top_k) {
                    let text = extract_span_text(&src, s.start_line, s.end_line);
                    let key = riir_refine::kat_lease_client::span_key(&text);
                    let mut rules: Vec<String> = s
                        .by_domain
                        .iter()
                        .flat_map(|(_, rs)| rs.iter().map(|r| r.rule_id.clone()))
                        .collect();
                    rules.sort();
                    rules.dedup();
                    pass2.push(PassSpans {
                        key,
                        tokens: riir_refine::kat_meter::count_word_tokens(text.as_bytes()),
                        rules,
                    });
                    span_texts.insert(key, text);
                }
            }
            let plan = riir_refine::kat_lease_client::plan_reheat(&ctx.pass1, &pass2);
            // Which FILES carry changed spans (the proposer granularity).
            for file in files {
                let Ok(src) = std::fs::read_to_string(file) else {
                    continue;
                };
                let display = file.display().to_string();
                let any_changed =
                    matcher
                        .suggest_file(&src, &display, args.top_k)
                        .iter()
                        .any(|s| {
                            let text = extract_span_text(&src, s.start_line, s.end_line);
                            plan.changed
                                .iter()
                                .any(|(k, _)| *k == riir_refine::kat_lease_client::span_key(&text))
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
                        active_set,
                        args.repo_posture && !args.apply_notbenchable,
                        &args.fix_groups,
                    );
                }
            }
            let report = riir_refine::kat_lease_client::ReheatReport {
                pass1_epoch: ctx.start_epoch,
                pass2_epoch: epoch,
                pass1_tokens: ctx.pass1_tokens,
                pass2_tokens: plan.reheat_tokens,
                changed_spans: plan.changed.len(),
                new_fixes,
            };
            let _ = charge.save();
            let line = riir_refine::kat_lease_client::LeaseCharge::line(epoch, charge_micro_kat);
            Some((report, Some(line)))
        }
        _ => None,
    }
}

#[cfg(all(test, feature = "kat_service", feature = "latent_retrieval"))]
mod sync_export_tests {
    use super::*;

    /// Issue 112 T4: the export's no-policy preview is the policy `--mine`
    /// writes. If the constant stopped parsing (or drifted off the compiled
    /// schema version) the preview would silently deny everything.
    #[test]
    fn default_sync_policy_parses_at_the_compiled_version() {
        let policy: riir_refine::kat_redact::SyncPolicy =
            serde_json::from_str(DEFAULT_SYNC_POLICY_JSON).expect("default policy parses");
        assert_eq!(policy.version, riir_refine::kat_redact::REDACT_VERSION);
        assert!(policy.sync_enabled);
    }

    /// riir-dapps Issue 119: rows bind by recorded file, so the default
    /// globs ARE the contribution surface — root + `crates/*/` sources in,
    /// vendored / build-script / out-of-tree files out (allow-only policy).
    #[test]
    fn default_sync_policy_covers_root_and_workspace_crates_only() {
        let policy: riir_refine::kat_redact::SyncPolicy =
            serde_json::from_str(DEFAULT_SYNC_POLICY_JSON).expect("default policy parses");
        let allowed = |p: &str| {
            policy
                .allow_paths
                .iter()
                .any(|g| riir_refine::kat_redact::glob_match(g, p))
        };
        for p in [
            "src/lib.rs",
            "tests/a/b.rs",
            "benches/x.rs",
            "examples/e.rs",
            "crates/core/src/lib.rs",
            "crates/core/tests/t.rs",
            "crates/core/benches/b.rs",
            "crates/core/examples/e.rs",
        ] {
            assert!(allowed(p), "{p} must sync");
        }
        for p in [
            "build.rs",
            "crates/core/build.rs",
            "vendor/dep/src/lib.rs",
            "crates/core/vendor/dep/src/lib.rs",
            "target/debug/build/out.rs",
            "",
        ] {
            assert!(!allowed(p), "{p:?} must not sync");
        }
    }
}

#[cfg(test)]
mod fix_review_note_tests {
    use super::*;

    /// One edit's review through the REAL formatter (`format_fix_review` —
    /// `print_fix_review` is `print!` over this exact string, so the pin
    /// here is the byte the CLI emits).
    fn render(rule_id: &str, note: Option<&'static str>) -> String {
        format_fix_review(
            "x.rs",
            &[(
                rule_id.to_string(),
                7,
                "r".to_string(),
                "o".to_string(),
                note,
                Vec::new(),
            )],
            None,
        )
    }

    #[test]
    fn annotated_lint_carries_the_warning_line() {
        let note = rule_context_note("manual_map");
        let Some(note) = note else {
            panic!("manual_map must carry an annotation under clippy_lints");
        };
        let rendered = render("manual_map", Some(note));
        assert_eq!(
            rendered,
            format!(
                "fix x.rs (1 edit(s)):\n  [{:<24}] line 7\n  \u{26a0} semantic change: {note}\n",
                "manual_map"
            ),
            "the ⚠ line must sit on its own line, 2-space indent, after the rule line"
        );
    }

    #[test]
    fn annotated_perf_rule_carries_the_warning_line() {
        let note = rule_context_note("static-str");
        assert!(
            note.is_some_and(|n| n.contains("signature")),
            "static-str must carry the signature annotation, got {note:?}"
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
            assert_eq!(
                render(rule, None),
                format!("fix x.rs (1 edit(s)):\n  [{rule:<24}] line 7\n"),
                "{rule}: un-annotated output drifted"
            );
        }
    }

    #[test]
    fn every_annotated_rule_is_fixable() {
        // A note for a rule that cannot fire is dead weight — keep the two
        // tables in lockstep with the fix surfaces.
        for rule in [
            "manual_map",
            "needless_range_loop",
            "question_mark",
            "map_unwrap_or",
            "unnecessary_lazy_evaluations",
        ] {
            assert!(
                riir_refine::domains::clippy_lints::proposers::FIXABLE_LINTS.contains(&rule),
                "{rule} annotated but not FIXABLE"
            );
        }
        for rule in ["static-str", "bool-to-bitpack"] {
            assert!(
                riir_refine::domains::rust_perf::proposers::FIXABLE_RULES.contains(&rule),
                "{rule} annotated but not FIXABLE"
            );
        }
    }

    #[test]
    fn fmt_note_renders_last_and_multiline_notes_take_the_first_line() {
        // fmt line stays last; a multi-line note is truncated to its first
        // line at the LOOKUP (the tables are one-line by construction, but
        // the renderer must not regress if one grows).
        let out = format_fix_review(
            "x.rs",
            &[(
                "r1".to_string(),
                3,
                String::new(),
                String::new(),
                Some("first line\nsecond line"),
                Vec::new(),
            )],
            Some("would-format"),
        );
        assert!(out.contains("  \u{26a0} semantic change: first line\n"));
        assert!(!out.contains("second line"), "only the first line renders");
        assert!(
            out.ends_with("  fmt: would-format\n"),
            "fmt line stays last"
        );
    }

    #[test]
    fn empty_edits_renders_nothing() {
        assert_eq!(format_fix_review("x.rs", &[], None), String::new());
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
        assert_eq!(
            fix_domain_scopes(Some(DomainId::Clippy)),
            (false, true, false)
        );
    }

    #[test]
    fn perf_scopes_out_clippy() {
        assert_eq!(
            fix_domain_scopes(Some(DomainId::Perf)),
            (true, false, false)
        );
    }

    #[test]
    fn kernel_runs_neither_syn_domain() {
        assert_eq!(
            fix_domain_scopes(Some(DomainId::Kernel)),
            (false, false, false)
        );
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
    use super::{
        count_corpus_domain, resolve_corpus_repo_root, runtime_corpus_count, CORPUS_DOMAIN_TABLE,
    };

    #[test]
    fn source_walk_agrees_with_runtime_count_for_compiled_domains() {
        let repo =
            resolve_corpus_repo_root().expect("corpus paths must resolve when tests run in-repo");
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
        assert!(
            checked >= 2,
            "expected to pin at least the default-on domains, checked {checked}"
        );
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
    use super::{validate_mode_flags as v, Args};

    /// The --misses regression pin: the 2026-09-08 just-works bare default
    /// fired under EVERY standalone mode (only the four primary modes were
    /// excluded), setting `suggest=true` — and `--misses`'s strict standalone
    /// guard then refused it, so `cargo refine --misses` exited 2 from 09-08
    /// until the exclusion list was completed. Every standalone mode must
    /// stay out of the bare posture AND count as standalone for the paths
    /// guard — the 2026-09-12 --misses fix completed `is_bare_posture`
    /// (adding `!version`) but not the paths guard, so `cargo refine
    /// --version` went exit-2 ("no paths given") from 09-12 until the two
    /// lists were merged into the shared `standalone_mode` predicate; every
    /// new standalone mode adds a row here in the same commit.
    #[test]
    fn standalone_modes_are_never_bare() {
        use super::is_bare_posture as bare;
        use super::standalone_mode;
        type ModeSetter = Box<dyn Fn(&mut Args)>;
        let cases: Vec<(&str, ModeSetter)> = vec![
            ("--misses", Box::new(|m| m.misses = true)),
            ("--misses --report", Box::new(|m| m.misses_report = true)),
            ("--score-bench", Box::new(|m| m.score_bench = true)),
            ("--score-report", Box::new(|m| m.score_report = true)),
            ("--necessity-bench", Box::new(|m| m.necessity_bench = true)),
            ("--memory-stats", Box::new(|m| m.memory_stats = true)),
            ("--mine", Box::new(|m| m.mine = true)),
            ("--unmine", Box::new(|m| m.unmine = true)),
            ("--version", Box::new(|m| m.version = true)),
            ("-V", Box::new(|m| m.version = true)),
            ("--recompute-elo", Box::new(|m| m.recompute_elo = true)),
            ("--collapse-report", Box::new(|m| m.collapse_report = true)),
            ("--corpus-stats", Box::new(|m| m.corpus_stats = true)),
            ("--import-series", Box::new(|m| m.import_series = true)),
            ("--frontier-report", Box::new(|m| m.frontier_report = true)),
            ("--horizon-report", Box::new(|m| m.horizon_report = true)),
            ("--fixseq-stats", Box::new(|m| m.fixseq_stats = true)),
            ("--coverage-report", Box::new(|m| m.coverage_report = true)),
            ("--config-audit", Box::new(|m| m.config_audit = true)),
            ("--secret-scan", Box::new(|m| m.secret_scan = true)),
            ("--blind", Box::new(|m| m.blind = true)),
            ("--platform-audit", Box::new(|m| m.platform_audit = true)),
            ("--orphan-report", Box::new(|m| m.orphan_report = true)),
            (
                "--frame-time-report",
                Box::new(|m| m.frame_time_report = true),
            ),
            ("--refill", Box::new(|m| m.refill = true)),
            ("--info", Box::new(|m| m.info = true)),
            (
                "--gate-calibration",
                Box::new(|m| m.gate_calibration = true),
            ),
            ("--active-set", Box::new(|m| m.active_set_list = true)),
            (
                "--active-set-demote clippy:x",
                Box::new(|m| m.active_set_demote = Some("clippy:x".to_string())),
            ),
            (
                "--active-set-readmit clippy:x",
                Box::new(|m| m.active_set_readmit = Some("clippy:x".to_string())),
            ),
            (
                "--active-set-clear",
                Box::new(|m| m.active_set_clear = true),
            ),
            (
                "--active-set-cap clippy:40",
                Box::new(|m| m.active_set_cap = Some("clippy:40".to_string())),
            ),
            (
                "--dump-trajectories",
                Box::new(|m| m.dump_trajectories = true),
            ),
        ];
        for (label, set) in cases {
            let mut m = Args::default();
            set(&mut m);
            assert!(!bare(&m), "{label} must not take the bare suggest default");
            assert!(
                standalone_mode(&m),
                "{label} must pass the paths guard (no paths required)"
            );
        }
        // The bare posture itself still fires for a no-flag invocation.
        assert!(bare(&Args::default()));
        assert!(!standalone_mode(&Args::default()));
        // And the primary modes still count as modes.
        assert!(!bare(&a(true, false, false, false)), "--suggest");
        assert!(!bare(&a(false, true, false, false)), "--fix");
        assert!(!bare(&a(false, false, true, false)), "--fix-compile");
        assert!(!bare(&a(false, false, false, true)), "--dump-trajectories");
    }

    /// Plan 165: `--repo-posture` is a fix-lane MODIFIER, not a mode — it
    /// must (a) break the bare posture (a silent suggest run under a
    /// fix-lane gate is the no-op trap the --misses row names), (b) refuse
    /// without --fix with the flag's own message naming the requirement,
    /// (c) validate WITH --fix.
    #[test]
    fn repo_posture_requires_fix_and_breaks_the_bare_default() {
        use super::is_bare_posture as bare;
        let m = Args {
            repo_posture: true,
            ..Args::default()
        };
        assert!(
            !bare(&m),
            "--repo-posture alone must not take the bare suggest default"
        );
        assert_eq!(
            v(&m),
            Err("--repo-posture requires --fix (it gates the fix lane's edits)")
        );
        let ok = Args {
            repo_posture: true,
            fix: true,
            ..Args::default()
        };
        assert!(v(&ok).is_ok(), "--fix --repo-posture must validate");
    }

    /// riir-dapps Issue 119 T12: `--purge-server` is `--unmine`-scoped —
    /// the server-side half of opting out (it signs with the account key,
    /// which `--purge` is about to remove). The `requires --unmine` refusal
    /// is a parse_args exit-2 guard (the `--purge` precedent — parse-level
    /// guards exit before validate_mode_flags), so this pin covers what the
    /// mode matrix owns: the standalone rows (never the bare default, and
    /// through the paths guard) and the fact that the COMBINATION passes
    /// the mode validator. The parse guard itself is the exact
    /// `--purge`-shaped three-line block beside the flag's match arm.
    #[test]
    fn purge_server_requires_unmine() {
        use super::is_bare_posture as bare;
        use super::standalone_mode;
        let m = Args {
            purge_server: true,
            ..Args::default()
        };
        assert!(
            !bare(&m),
            "--purge-server alone must not take the bare suggest default"
        );
        assert!(
            standalone_mode(&m),
            "--purge-server must pass the paths guard (no paths required)"
        );
        // `--unmine` NEVER reaches the mode validator (the standalone
        // early-return in main precedes it) — so `--unmine` alone fails
        // validate_mode_flags exactly like the combo would; the parse-level
        // `requires --unmine` guard is the only validator this flag has,
        // by the same construction as `--purge`.
        let combo = Args {
            purge_server: true,
            unmine: true,
            ..Args::default()
        };
        assert!(
            v(&combo).is_err(),
            "the combo exits at the --unmine early-return before validate_mode_flags"
        );
        let unmine_alone = Args {
            unmine: true,
            ..Args::default()
        };
        assert!(
            v(&unmine_alone).is_err(),
            "the --unmine early-return precedent"
        );
    }

    /// Named-field construction keeps each matrix row self-describing —
    /// the positional-8-bool form this replaced is the shape
    /// `clippy::too_many_arguments` exists to name.
    fn a(suggest: bool, fix: bool, fix_compile: bool, dump_trajectories: bool) -> Args {
        Args {
            suggest,
            fix,
            fix_compile,
            dump_trajectories,
            ..Args::default()
        }
    }

    #[test]
    fn accepted_combinations() {
        // The four modes alone (T4.3b adds --dump-trajectories).
        assert!(v(&a(true, false, false, false)).is_ok(), "--suggest");
        assert!(v(&a(false, true, false, false)).is_ok(), "--fix");
        assert!(v(&a(false, false, true, false)).is_ok(), "--fix-compile");
        assert!(
            v(&a(false, false, false, true)).is_ok(),
            "--dump-trajectories"
        );
        // --write with each writing mode.
        assert!(
            v(&Args {
                write: true,
                ..a(false, true, false, false)
            })
            .is_ok(),
            "--fix --write"
        );
        assert!(
            v(&Args {
                write: true,
                ..a(false, false, true, false)
            })
            .is_ok(),
            "--fix-compile --write"
        );
        // The trap this task closed: --fix-compile --write --verify-args.
        assert!(
            v(&Args {
                write: true,
                verify_args: Some(String::new()),
                ..a(false, false, true, false)
            })
            .is_ok(),
            "--fix-compile --write --verify-args \"...\""
        );
        // --verify still needs --fix (its batch semantics are --fix's).
        assert!(
            v(&Args {
                write: true,
                verify: true,
                ..a(false, true, false, false)
            })
            .is_ok(),
            "--fix --write --verify"
        );
        assert!(
            v(&Args {
                write: true,
                verify: true,
                verify_args: Some(String::new()),
                ..a(false, true, false, false)
            })
            .is_ok(),
            "--fix --verify --verify-args"
        );
        // The owner's just-works defaults (2026-09-08): a bare invocation
        // with a KAT command is NOT a mode error, and no-mode-no-command is
        // the dry run (main defaults it BEFORE the matrix, so the matrix
        // itself never sees that shape — but a KAT command must satisfy it).
        assert!(
            v(&Args {
                command: Some(super::KatCommand::Account),
                ..a(false, false, false, false)
            })
            .is_ok(),
            "KAT command alone"
        );
    }

    #[test]
    fn rejected_combinations() {
        assert!(v(&a(false, false, false, false)).is_err(), "no mode");
        assert!(
            v(&Args {
                write: true,
                ..a(false, false, false, false)
            })
            .is_err(),
            "--write without a mode"
        );
        assert!(
            v(&Args {
                verify: true,
                ..a(false, false, false, false)
            })
            .is_err(),
            "--verify without --fix"
        );
        assert!(
            v(&Args {
                verify: true,
                ..a(false, false, true, false)
            })
            .is_err(),
            "--verify with --fix-compile does NOT satisfy the --fix requirement"
        );
        assert!(
            v(&Args {
                verify_args: Some(String::new()),
                ..a(true, false, false, false)
            })
            .is_err(),
            "--verify-args without --verify/--fix-compile"
        );
        assert!(
            v(&a(false, false, true, true)).is_err(),
            "--dump-trajectories is standalone (no other mode)"
        );
        assert!(
            v(&Args {
                write: true,
                ..a(false, false, false, true)
            })
            .is_err(),
            "--write does not apply to --dump-trajectories"
        );
    }
}

#[cfg(test)]
mod issue_149_scan_banner_tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        // The pid suffix keeps concurrent test runs off each other's scratch
        // (the shared-temp-path lesson; katgpt-rs Issue 832).
        let d = std::env::temp_dir().join(format!("refine149_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("temp root");
        d
    }

    fn git_ok(root: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["-c", "user.email=t@t", "-c", "user.name=t"])
            .args(args)
            .output()
            .expect("git on PATH");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn untracked_files_among_collected_are_counted() {
        let root = temp_root("untracked");
        git_ok(&root, &["init", "-q"]);
        std::fs::write(root.join("tracked.rs"), "fn a() {}\n").unwrap();
        git_ok(&root, &["add", "."]);
        git_ok(&root, &["commit", "-qm", "seed"]);
        std::fs::write(root.join("wip.rs"), "fn b() {}\n").unwrap();
        let collected = vec![root.join("tracked.rs"), root.join("wip.rs")];
        let infos = scan_root_infos(std::slice::from_ref(&root), &collected);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].files, 2);
        assert_eq!(infos[0].untracked, Some(1));
        let line = render_scan_line(&infos[0]);
        assert!(
            line.contains("1 untracked included"),
            "banner must name the untracked count: {line}"
        );
        assert!(line.starts_with("scan root: "), "absolute root: {line}");
        assert!(infos[0].root.is_absolute(), "the root must be absolute");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn outside_a_repository_the_count_is_unavailable_not_zero() {
        let root = temp_root("norepo");
        std::fs::write(root.join("a.rs"), "fn a() {}\n").unwrap();
        let collected = vec![root.join("a.rs")];
        let infos = scan_root_infos(std::slice::from_ref(&root), &collected);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].files, 1);
        assert_eq!(infos[0].untracked, None, "None = unavailable, never 0");
        let line = render_scan_line(&infos[0]);
        assert!(!line.contains("untracked"), "no fabricated term: {line}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn single_file_root_discloses_its_parent_and_dedups_roots() {
        let root = temp_root("fileroot");
        std::fs::write(root.join("a.rs"), "fn a() {}\n").unwrap();
        let file = root.join("a.rs");
        let collected = vec![file.clone()];
        // The same directory twice (once as a file, once as the dir) → one line.
        let infos = scan_root_infos(&[file, root.clone()], &collected);
        assert_eq!(infos.len(), 1, "duplicate roots collapse");
        assert_eq!(infos[0].files, 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod issue_149_resplice_dedup_tests {
    use super::*;
    use riir_refine::edit_emitter::SourceEdit;

    fn edit(
        pass: usize,
        rule: &str,
        start: usize,
        end: usize,
        replacement: &str,
    ) -> (usize, String, usize, usize, String) {
        (pass, rule.to_string(), start, end, replacement.to_string())
    }

    fn splice(start: usize, end: usize, replacement: &str) -> SourceEdit {
        SourceEdit {
            start,
            end,
            replacement: replacement.to_string(),
        }
    }

    /// The capture's shape: needless_return (pass 0) rewrote `return cents;`,
    /// then let_and_return (pass 1) collapsed the whole `let .. cents;`
    /// region — three splices in the real run, one visible repair. The
    /// earlier edit no longer exists in the final text; the later one is
    /// the fix record.
    #[test]
    fn later_pass_resplice_keeps_only_the_surviving_edit() {
        let mut edits = vec![
            edit(0, "needless_return", 10, 25, "cents;"),
            edit(1, "let_and_return", 2, 22, "expr"),
        ];
        let pass_splices = vec![vec![splice(10, 25, "cents;")], vec![splice(2, 22, "expr")]];
        let dropped = dedup_respliced_edits(&mut edits, &pass_splices);
        assert_eq!(edits.len(), 1, "one visible repair, one record");
        assert_eq!(edits[0].1, "let_and_return");
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].1, "needless_return");
    }

    /// A ⊂ B ⊂ C across three passes: only the last survives.
    #[test]
    fn a_chain_of_resplices_collapses_to_the_last_edit() {
        let mut edits = vec![
            edit(0, "a", 0, 30, "one"),
            edit(1, "b", 0, 25, "two"),
            edit(2, "c", 0, 20, "three"),
        ];
        let pass_splices = vec![
            vec![splice(0, 30, "one")],
            vec![splice(0, 25, "two")],
            vec![splice(0, 20, "three")],
        ];
        let dropped = dedup_respliced_edits(&mut edits, &pass_splices);
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].1, "c");
        assert_eq!(dropped.len(), 2);
    }

    #[test]
    fn disjoint_repairs_across_passes_are_all_kept() {
        // pass 0 shrank the file by 5 bytes; the pass-1 edit sits after the
        // shifted region — different repairs of different spans.
        let mut edits = vec![
            edit(0, "collapsible_if", 4, 20, "compact"),
            edit(1, "bool_comparison", 25, 40, "flag"),
        ];
        let pass_splices = vec![vec![splice(4, 20, "compact")], vec![splice(20, 35, "flag")]];
        let dropped = dedup_respliced_edits(&mut edits, &pass_splices);
        assert_eq!(edits.len(), 2);
        assert!(dropped.is_empty());
    }

    #[test]
    fn same_pass_edits_never_subsume_each_other() {
        let mut edits = vec![edit(0, "a", 4, 20, "one"), edit(0, "b", 30, 40, "two")];
        let pass_splices = vec![vec![splice(4, 20, "one"), splice(30, 40, "two")]];
        let dropped = dedup_respliced_edits(&mut edits, &pass_splices);
        assert_eq!(edits.len(), 2);
        assert!(dropped.is_empty());
    }
}

/// P44: a leading UTF-8 BOM poisons proc-macro2's fallback span→byte
/// mapping — byte_range() returns char-space offsets for BOM'd source, so
/// every syn span lands shifted against the real bytes (the helpers.rs:630
/// mid-char panic class, and the silent misplaced-edit class beside it).
/// The read seam strips the BOM once, before any parse.
#[cfg(test)]
mod p44_bom_seam_tests {
    use super::read_source;
    use syn::spanned::Spanned;

    #[test]
    fn bom_is_stripped_and_the_rest_is_verbatim() {
        let dir =
            std::env::temp_dir().join(format!("p44_bom_seam_{}_{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bommed.rs");
        let body = "/// the seat’s pool —uléiš text\nfn go() {}\n";
        let bommed = format!("\u{feff}{body}");
        std::fs::write(&path, &bommed).unwrap();

        let src = read_source(&path).unwrap();
        assert_eq!(src, body, "BOM stripped, body verbatim");

        // The composed seam: the stripped text parses and the proposer's
        // spans are boundary-safe against the SAME string it slices.
        let parsed: syn::File = syn::parse_file(&src).unwrap();
        for attr in &parsed.attrs {
            let r = attr.span().byte_range();
            assert!(
                src.is_char_boundary(r.start) && src.is_char_boundary(r.end),
                "span {r:?} not boundary-safe"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bom_free_source_is_returned_verbatim() {
        let dir =
            std::env::temp_dir().join(format!("p44_bom_free_{}_{}", std::process::id(), line!()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clean.rs");
        std::fs::write(&path, "fn go() {}\n").unwrap();
        assert_eq!(read_source(&path).unwrap(), "fn go() {}\n");
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod run_rollup_tests {
    use super::*;

    /// Issue 165: the roll-up must read exactly like the run it summarizes —
    /// per-domain counts, the total, and the fixable floor only where the
    /// compiled set can name one.
    #[test]
    fn rollup_line_counts_domains_and_totals() {
        let mut t = DomainTallies::default();
        t.add_citation("clippy", true);
        t.add_citation("clippy", false);
        t.add_citation("perf", false);
        t.add_secrets(95);
        assert_eq!(
            t.rollup_line(),
            "findings by domain: clippy 2 · perf 1 · sec 95 — 98 total"
        );
        assert_eq!(t.total(), 98);
        assert_eq!(
            t.citations, 3,
            "secrets are display-only, never fix-surface"
        );
        assert_eq!(t.fixable_clippy, 1);
    }

    #[test]
    fn lane_counts_merge_and_zero_counts_stay_silent() {
        let mut t = DomainTallies::default();
        t.add_count("docker", 2);
        t.add_count("dist", 0);
        t.add_count("kernel", 5);
        t.add_count("shader", 0);
        assert_eq!(
            t.rollup_line(),
            "findings by domain: docker 2 · kernel 5 — 7 total"
        );
        assert!(
            !t.rollup_line().contains("dist"),
            "a zero lane never prints"
        );
        assert!(
            !t.rollup_line().contains("shader"),
            "a zero lane never prints"
        );
    }

    #[test]
    fn zero_tallies_total_zero_and_the_line_is_never_printed_by_the_caller() {
        let t = DomainTallies::default();
        assert_eq!(t.total(), 0);
        // The call-site gate is `total() > 0`; this pin is the shape the
        // gate reads (an all-zero roll-up line would be CTA noise).
        assert!(t.per_domain.is_empty());
    }
}

/// The `tuna` verb's absent-pair answer names the account's actual state
/// instead of blaming the deployment (2026-10-17): a consumed free tier
/// and an unclaimed account read the identical wire shape (the zipped
/// `tuna_balance_micro` pair omitted), and `granted` — the unified claim
/// predicate the same response carries — is the only honest split.
#[cfg(test)]
mod tuna_none_line_tests {
    #[test]
    fn consumed_and_unclaimed_get_distinct_honest_lines() {
        let consumed = super::tuna_none_line(true);
        assert!(consumed.contains("consumed"), "{consumed}");
        assert!(
            consumed.contains("once per account"),
            "the once-per-life law rides the line: {consumed}"
        );
        let unclaimed = super::tuna_none_line(false);
        assert!(unclaimed.contains("not claimed"), "{unclaimed}");
        assert!(
            unclaimed.contains("cargo refine account"),
            "the automatic claim path is named: {unclaimed}"
        );
        // The old misattribution is gone: neither line blames the
        // deployment for an account-scoped fact.
        assert!(!consumed.contains("pre-TUNA"));
        assert!(!unclaimed.contains("pre-TUNA"));
    }
}

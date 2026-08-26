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
use riir_clippy::EvolveRecorder;

/// Proposal 005 Phase 0 T0.1 (Plan 025): the identity subcommands — parsed
/// unconditionally so a `kat_service`-off build errors cleanly instead of
/// treating `login` as a scan path (the `--fix-compile`-off precedent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KatCommand {
    /// Generate/import the account key + claim the grant (local projection).
    Login,
    /// Print the local account view (+ repo-local charge projection when a
    /// path/cwd has one).
    Account,
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
        None => KatAccount::login(&dir),
    };
    let (account, outcome) = match result {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("kat login failed: {e}");
            std::process::exit(1);
        }
    };
    match outcome {
        LoginOutcome::Generated { .. } => println!("kat: account key generated"),
        LoginOutcome::Imported { .. } => println!("kat: account key imported"),
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
/// burn + balance projection with the per-repo caveat.
#[cfg(feature = "kat_service")]
fn run_kat_account(paths: &[PathBuf]) {
    use riir_clippy::kat_account::KatAccount;
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
    if let Ok(text) = std::fs::read_to_string(&charge_path) {
        match serde_json::from_str::<ChargeState>(&text) {
            Ok(charge) => {
                let grant = account.state().grant_kat;
                let remaining = grant.saturating_sub(charge.charged_kat);
                println!("local burn:          {} KAT charged this repo ({} tokens, watermark {})",
                    charge.charged_kat, charge.meter_watermark_tokens, charge.meter_watermark_tokens);
                println!("balance projection:  {remaining} of {grant} KAT (per-repo until server aggregation)");
                if remaining <= KAT_WARN_REMAINING {
                    println!("kat: WARN balance ≤ {KAT_WARN_REMAINING} — top up before the next long run (never blocks healing)");
                }
            }
            Err(e) => eprintln!(
                "kat account: charge ledger unreadable ({}): {e}",
                charge_path.display()
            ),
        }
    } else {
        println!(
            "local burn:          (no charge ledger at {} — heal a repo to start one)",
            charge_path.display()
        );
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

/// Record one verdict event into the durable store (best-effort — a store
/// failure warns once per event, never fails the heal).
#[cfg(all(feature = "fix_verify", feature = "latent_retrieval"))]
fn record_traj(
    store: &Option<riir_clippy::traj_store::TrajStore>,
    rule_id: &str,
    restore: &str,
    replacement: &str,
    success: bool,
    verified: bool,
    run_id: &str,
) {
    if let Some(store) = store {
        if let Err(e) = store.record(rule_id, restore, replacement, success, verified, run_id) {
            eprintln!("self-evolve: store record failed: {e}");
        }
    }
}

/// [`record_traj`] fixed at the failure verdict (reverts — Issue 018's
/// definitive compile-gate fail).
#[cfg(all(feature = "fix_verify", feature = "latent_retrieval"))]
fn record_traj_fail(
    store: &Option<riir_clippy::traj_store::TrajStore>,
    rule_id: &str,
    restore: &str,
    replacement: &str,
    run_id: &str,
) {
    record_traj(store, rule_id, restore, replacement, false, false, run_id);
}

/// Print the per-edit fix review + fmt annotation. `edits` are
/// `(rule_id, final line, replacement, restore)` — only the first two
/// fields print here; the last two exist for the verify path.
/// `fmt_note` is the per-file fmt-plan/outcome annotation (`None` disables).
fn print_fix_review(
    display: &str,
    edits: &[(String, usize, String, String)],
    fmt_note: Option<&str>,
) {
    if edits.is_empty() {
        return;
    }
    println!("fix {} ({} edit(s)):", display, edits.len());
    for (rule_id, line, _, _) in edits {
        println!("  [{rule_id:<24}] line {line}");
    }
    if let Some(note) = fmt_note {
        println!("  fmt: {note}");
    }
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
    let score_help = "    --score-bench   Run the frozen clippy-bench (real-code fixtures, real clippy ground\n                    truth — Plan 014), append to .benchmarks/score_history.jsonl,\n                    re-render .benchmarks/score_history.svg. No paths needed.\n    --score-report  Re-render the score-history SVG from the existing JSONL (no run).\n";
    #[cfg(not(feature = "score_bench"))]
    let score_help = String::new();
    eprintln!(
        "cargo-heal (Plan 005) — modelless code healing via latent corpus retrieval

USAGE:
    cargo heal --suggest [--domain <auto|clippy|perf|kernel|sec>] [--top-k <N>] <path>...
    cargo heal --fix [--write] [--no-fmt] [--quiet] [--verify [--verify-args <s>]] [--top-k <N>] <path>...
    cargo heal --fix-compile [--write] [--max-iters <N>] [--verify-args <s>] <path>...
    cargo heal --dump-trajectories > trajectories.jsonl
    cargo heal --memory-stats                         (loop debug query, no paths)
    cargo heal --recompute-elo                        (batch rating backfill, 039 T2)
    cargo heal --collapse-report                      (self-collapse monitor, 042)
    cargo heal --score-bench | --score-report        (needs --features score_bench)
    cargo heal login [--key <path>]                   (KAT account, needs --features kat_service)
    cargo heal account [<path>]                       (KAT local view + repo burn, kat_service)

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
    --key <path>    With login: import this OpenSSH Ed25519 private key
                    (unencrypted only) instead of generating one.
{}{}    --domain        Filter suggest output to one domain (default: auto — all enabled).
    --top-k <N>     Rules per span per domain (default 3).
    <path>...       Files or directories to scan. `.rs` always; with the
                    kernel_heal feature, `--suggest` ALSO scans `.wgsl` files
                    + `#[cube]` kernel bodies in `.rs` files (Issue 025 —
                    suggest-only citations, never touched by --fix).

Domains are feature-gated: clippy + perf are default-on; kernel needs
--features kernel_heal. Requesting a domain whose feature is off is an
error (T2.4 gating). --verify needs --features fix_verify.
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
    /// `--score-report`: re-render the SVG from the existing history JSONL.
    score_report: bool,
    /// `--memory-stats` (Issue 037 observability): print the one-command
    /// debug summary of the self-evolve store — tier/outcome counts, top
    /// rules, BLAKE3 commitment status, + the E1/E2/E3 loop-state verdict.
    /// Read-only standalone mode (no paths, no recording, ignores the
    /// opt-out env — a query, not a run).
    memory_stats: bool,
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
    /// Proposal 005 T0.1: the identity subcommand (`login` / `account`),
    /// recognized only as the FIRST positional (a dir named `login` later in
    /// the arg list still scans).
    command: Option<KatCommand>,
    /// `--key <path>` — import an existing OpenSSH Ed25519 private key at
    /// `login` instead of generating one.
    import_key: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut args = Args {
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
        score_report: false,
        memory_stats: false,
        recompute_elo: false,
        collapse_report: false,
        command: None,
        import_key: None,
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
            "--recompute-elo" => args.recompute_elo = true,
            "--collapse-report" => args.collapse_report = true,
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
                    other => {
                        eprintln!("unknown --domain {other:?} (want auto|clippy|perf|kernel|sec)");
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
            flag if flag.starts_with("--") => {
                eprintln!("unknown flag {flag:?}");
                usage();
            }
            "login" | "account" if args.paths.is_empty() && args.command.is_none() => {
                args.command = Some(if arg == "login" {
                    KatCommand::Login
                } else {
                    KatCommand::Account
                });
            }
            path => args.paths.push(PathBuf::from(path)),
        }
    }
    if args.paths.is_empty()
        && args.command.is_none()
        && !args.score_bench
        && !args.score_report
        && !args.dump_trajectories
        && !args.memory_stats
        && !args.recompute_elo
        && !args.collapse_report
    {
        eprintln!("no paths given");
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
fn collect_source_files(root: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    if root.is_file() {
        let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(ext) {
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
            } else if name.ends_with(ext) {
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

/// Plan 014: run the frozen clippy-bench, append the score history,
/// re-render the SVG. Feature-gated `score_bench`.
#[cfg(feature = "score_bench")]
fn run_score_bench() {
    use riir_clippy::score_bench as sb;

    let oracle = sb::draft_oracle();
    if !oracle.is_available() {
        eprintln!("score_bench: cargo not on PATH — cannot derive ground truth");
        std::process::exit(1);
    }
    let fixture_list = sb::fixtures::fixtures();
    println!(
        "clippy-bench: {} frozen fixture(s), real-clippy ground truth…",
        fixture_list.len()
    );
    let score = sb::run_bench(&oracle, &fixture_list);
    sb::report::print_report(&score);

    let rec = sb::report::HistoryRecord::from_score(&score, sb::report::git_sha());
    let jsonl = std::path::Path::new(sb::report::HISTORY_JSONL);
    if let Err(e) = sb::report::append_history(jsonl, &rec) {
        eprintln!("score history append failed: {e}");
    }
    let history = sb::report::load_history(jsonl);
    let svg = sb::report::render_svg(&history);
    std::fs::write(sb::report::HISTORY_SVG, &svg)
        .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
    println!(
        "history: {} run(s) → {} + {}",
        history.len(),
        sb::report::HISTORY_JSONL,
        sb::report::HISTORY_SVG
    );
}

fn main() {
    let args = parse_args();

    // ── Proposal 005 T0.1: identity subcommands (standalone — no paths) ──
    #[cfg(feature = "kat_service")]
    if let Some(cmd) = args.command {
        match cmd {
            KatCommand::Login => run_kat_login(args.import_key.as_deref()),
            KatCommand::Account => run_kat_account(&args.paths),
        }
        return;
    }
    #[cfg(not(feature = "kat_service"))]
    if args.command.is_some() {
        eprintln!("login/account require a build with --features kat_service");
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
        let svg = riir_clippy::score_bench::report::render_svg(&history);
        std::fs::write(riir_clippy::score_bench::report::HISTORY_SVG, &svg)
            .unwrap_or_else(|e| eprintln!("svg write failed: {e}"));
        println!(
            "rendered {} ({} run(s))",
            riir_clippy::score_bench::report::HISTORY_SVG,
            history.len()
        );
        return;
    }
    #[cfg(feature = "score_bench")]
    if args.score_bench {
        run_score_bench();
        return;
    }
    #[cfg(not(feature = "score_bench"))]
    if args.score_bench || args.score_report {
        eprintln!("--score-bench / --score-report require a build with --features score_bench");
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
                        if let Err(e) = store.record(
                            &traj_key(&r.code, &r.strategy_id),
                            &r.shape,
                            &r.fix,
                            false,
                            false,
                            &heal_run_id,
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
                    .and_then(|p| riir_clippy::compile_oracle::owning_package_root(p))
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

    let mut matcher = LatentMatcher::from_enabled_domains().unwrap_or_else(|e| {
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
    if files.is_empty() && wgsl_files.is_empty() {
        eprintln!("no .rs/.wgsl files under the given paths");
        std::process::exit(1);
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

    let mut n_chunks = 0usize;
    let mut n_suggestions = 0usize;
    let mut n_fixed_files = 0usize;
    let mut n_fixed_edits = 0usize;

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
                        std::fs::read_to_string(pkg.join("Cargo.toml")).map_or(false, |t| {
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
        let all_edits: Vec<(String, usize, String, String)> = all_edits
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
                (rule_id, line, replacement, restore)
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
                    evolve_rec.record_reviewed(&all_edits);
                }
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    for (rule, _, replacement, restore) in &all_edits {
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
                    evolve_rec.record_reviewed(&all_edits);
                }
                #[cfg(feature = "latent_retrieval")]
                if let Some(store) = &traj_store {
                    for (rule, _, replacement, restore) in &all_edits {
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
    }

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
                // compile failure itself is the verdict).
                for e in &b.edits {
                    evolve_rec.record_outcome(&e.rule_id, &e.restore, &e.replacement, false);
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
                evolve_rec.record_outcome_verified(
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    success,
                    verified,
                );
                #[cfg(feature = "latent_retrieval")]
                record_traj(
                    &traj_store,
                    &e.rule_id,
                    &e.restore,
                    &e.replacement,
                    success,
                    verified,
                    &heal_run_id,
                );
            }
            for e in b.edits.iter().filter(|e| reverted_lines.contains(&e.final_line)) {
                evolve_rec.record_outcome(&e.rule_id, &e.restore, &e.replacement, false);
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
        if c.low_balance() {
            // §KAT out-of-KAT warn: the QR/top-up half is T1.6. Healing never
            // blocks on this (G3 — warn-and-pay, never brick).
            eprintln!(
                "kat-charge: warning — {} KAT remain of the {} grant projection; \
                 healing continues offline (the server ledger is the authority)",
                c.balance_projection(),
                riir_clippy::kat_meter::FREE_TIER_GRANT_KAT
            );
        }
    }
}

/// Issue 031 H2: `--domain` scopes the `--fix` proposer loops. Returns
/// `(run_perf, run_clippy)`. `auto` (None) runs both compiled-in domains
/// (the historical behavior); an explicit domain runs ONLY its own
/// proposers; `kernel` runs neither (kernel proposers are Phase 5).
fn fix_domain_scopes(domain: Option<DomainId>) -> (bool, bool, bool) {
    (
        domain.is_none_or(|d| d == DomainId::Perf),
        domain.is_none_or(|d| d == DomainId::Clippy),
        domain.is_none_or(|d| d == DomainId::Sec),
    )
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

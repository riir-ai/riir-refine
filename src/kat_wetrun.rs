//! Proposal 011 §7 / T4.1+T4.2 (Plan 066): `cargo heal wetrun` — the
//! full-loop proof, and the **G-WET** gate that decides whether it passed.
//!
//! ```text
//! 1. preflight (read-only): nodes healthy, ledger settled, my /balance fetched
//! 2. faucet tops MY account +X KAT              (real ledger rows)
//! 3. one scored scan→fix cycle on a seeded dirty fixture (the meter records the spend)
//! 4. signed sync push (redact ≥ floor) → service records → self-evolve dry-run queued
//! 5. faucet refund credit X arrives             (labeled the wet-run refund)
//! 6. mesh readiness: planes healthy + settled → "ready" verdict printed
//! ```
//!
//! # The three things this module exists to make impossible
//!
//! **1. A money printer with a test wrapper (caveat #2).** Steps 2 and 5 are
//! FAUCET legs — a grant funded from the treasury — and never mint or burn-pool
//! draws. The gate is [`SupplyCounters`]: read before step 2, read again after
//! step 5, and compared as **bytes** via a total-function encoding, so
//! "mint/burn counters bit-unchanged" is a literal claim rather than a
//! hand-picked field subset. The riir-dapps G-CRANK `Meta` discipline, mirrored
//! on the wire side: a counter added later is covered automatically, and
//! [`tests::the_supply_encoding_covers_every_field`] reds if a field is added
//! without extending the encoding.
//!
//! **2. A silent skip read as a pass.** riir-dapps **Issue 010** —
//! `TREASURY_GRANT_FLOOR_DEFAULT == MAX_SUPPLY_PLACEHOLDER` (both 10¹²) — makes
//! the treasury-debit grant path *structurally unreachable at genesis defaults*,
//! so the daily crank skips every account every day while emitting a below-floor
//! alert. The crank is deliberately treasury-ONLY (a mint fallback on an
//! unattended recurring path is exactly what caveat #2 forbids), so the skip is
//! the CORRECT behavior of a MISCONFIGURED deployment. This harness therefore
//! reads the crank report the service already serves and
//! [`classify_faucet_leg`] names that state
//! ([`FaucetLegVerdict::TreasuryFloorUnreachable`]) — a hard, diagnostic FAIL
//! that prints the issue number and both constants. It never retries, never
//! waits it out, and never lets it read as a pass.
//!
//! **3. A wet-run on mainnet.** [`admit`] refuses it, and there is **no
//! override** — per caveat #2 a `RIIR_ALLOW_MAINNET_WETRUN` would be "an attack
//! on our own bookkeeping". The refusal reads nothing from the environment at
//! all, which [`tests::the_mainnet_refusal_reads_no_environment`] pins at the
//! source level. Classification is **fail-closed**: a host this module cannot
//! place is refused exactly like mainnet, because a run that cannot name its
//! network must not move money.
//!
//! [`RunMode::DryRun`] (steps 1 + 6, read-only) is allowed on **every** network
//! including mainnet, and "touches nothing" is proven by the recorded call log,
//! not asserted in prose ([`tests::a_dry_run_issues_only_reads`]).
//!
//! # What this module deliberately does NOT do
//!
//! The per-node reachability / height / sync% table is **riir-deployer's
//! `node-sync`** (Proposal 011 §6, landed there as `VerifySpec::NodeSync`). This
//! harness reads the service planes it can reach and points at that gate for the
//! fleet view rather than forking a third implementation of it.
//!
//! Steps 1 and 6 reuse T0.2b's landed reads verbatim
//! ([`crate::kat_account_client`]): `/health`, `/mining/health`, `/balance`, and
//! the three-state [`crate::kat_account_client::WelcomeGate`] — an unreadable
//! plane is `Unknown`, never "healthy".

#[cfg(feature = "kat_transport")]
use crate::kat_lease_client::HttpResponse;

// ── the network boundary (T4.2) ─────────────────────────────────────────

/// Which network the resolved service URL points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// A local / devnet host — the wet-run's home.
    Devnet,
    /// A testnet host.
    Testnet,
    /// Mainnet. Refused for a money run, with no override.
    Mainnet,
    /// This module cannot place the host. **Treated as mainnet** for
    /// admission: a run that cannot name its network must not move money.
    Unclassifiable,
}

impl Network {
    /// The operator-facing name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Devnet => "devnet",
            Self::Testnet => "testnet",
            Self::Mainnet => "mainnet",
            Self::Unclassifiable => "unclassifiable",
        }
    }
}

/// Classify the RESOLVED service base URL.
///
/// **Substring on the host, not a prefix** — the riir-deployer `is_mainnet`
/// rule: `mainnet`, `chain-mainnet` and `mainnet-eu` must all trip it, and a
/// prefix match would let `chain-mainnet` through. `mainnet` is tested FIRST
/// so a host carrying both words is refused rather than admitted.
///
/// Loopback and RFC1918 hosts are devnet; anything else unnamed is
/// [`Network::Unclassifiable`] and therefore refused. The production default
/// (`heal.gist.rs`) carries no network word, so it lands there — which is
/// the safe answer, not an oversight: it is the live service, and a wet-run
/// against it must be an explicit, named choice.
#[must_use]
pub fn classify_service_url(url: &str) -> Network {
    let host = url
        .split_once("://")
        .map_or(url, |(_, rest)| rest)
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match () {
        // Order matters: a host naming BOTH is mainnet.
        () if host.contains("mainnet") => Network::Mainnet,
        () if host.contains("testnet") => Network::Testnet,
        () if host.contains("devnet") || is_local_host(&host) => Network::Devnet,
        () => Network::Unclassifiable,
    }
}

/// Loopback + RFC1918 + `.local` — a host only reachable from the operator's
/// own network.
fn is_local_host(host: &str) -> bool {
    let bare = host.split(':').next().unwrap_or(host);
    match bare {
        "localhost" | "127.0.0.1" | "[::1]" | "::1" | "0.0.0.0" => true,
        // The last label, not `ends_with(".local")` — clippy reads the
        // latter as a file-extension comparison, and `rsplit_once` says
        // what is actually meant (mDNS / loopback-alias TLDs).
        _ => {
            matches!(bare.rsplit_once('.'), Some((_, "local" | "localhost")))
                || bare.starts_with("192.168.")
                || bare.starts_with("10.")
                || bare.starts_with("172.16.")
        }
    }
}

/// Full money run vs the read-only rehearsal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// All six steps. Devnet/testnet only.
    Full,
    /// Steps 1 + 6 only — read-only, zero state. Allowed everywhere.
    DryRun,
}

/// The admission decision.
///
/// # Errors
///
/// The refusal sentence for a [`RunMode::Full`] run on
/// [`Network::Mainnet`]/[`Network::Unclassifiable`].
///
/// **Reads nothing from the environment.** There is no override and no
/// escape hatch — see the module docs' point 3. A `--dry-run` is admitted on
/// every network, which is what mainnet gets.
pub fn admit(network: Network, mode: RunMode) -> Result<(), String> {
    match (mode, network) {
        // Two independent reasons to admit, merged only because clippy
        // rejects identical arms: a DRY-RUN is admitted on EVERY network
        // (it is what mainnet gets), and a FULL run is admitted on
        // devnet/testnet.
        (RunMode::DryRun, _) | (RunMode::Full, Network::Devnet | Network::Testnet) => Ok(()),
        (RunMode::Full, Network::Mainnet) => Err(
            "wetrun: REFUSED on mainnet. The wet-run moves real ledger rows, and a faucet \
             loop on mainnet would put a test harness inside the supply bookkeeping \
             (Proposal 011 caveat #2). There is deliberately NO override — an env var here \
             would be a standing invitation. Use `wetrun --dry-run` (steps 1+6, read-only) \
             on mainnet, or point at devnet/testnet for the full loop."
                .to_string(),
        ),
        (RunMode::Full, Network::Unclassifiable) => Err(
            "wetrun: REFUSED — this service host cannot be placed on a network, and a run \
             that cannot name its network must not move money (fail-closed). Point \
             RIIR_HEAL_KAT_SERVICE_URL at a host whose name carries `devnet`/`testnet`, or a \
             loopback/RFC1918 address, or use `wetrun --dry-run`."
                .to_string(),
        ),
    }
}

// ── the money-printer guard: the supply counters on the wire ────────────

/// The supply-bearing counters the service exposes at
/// `GET /telemetry?epoch=<n>` — the wire view of riir-dapps' `Meta` row.
///
/// Integer money only (G4-S): every field is `i64` micro/KAT as the service
/// serves it. A float anywhere in here would be review-blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SupplyCounters {
    /// The epoch this row belongs to. Carried IN the snapshot so a
    /// comparison across two different epochs cannot masquerade as a
    /// bit-unchanged claim — it changes the bytes and reds the gate.
    pub epoch: u64,
    /// Mint accrued in this epoch.
    pub mint_kat: i64,
    /// Burn accrued in this epoch.
    pub burn_kat: i64,
    /// Mining credits paid from the pool.
    pub pool_paid_kat: i64,
    /// Pool backing available to mining credits.
    pub pool_available_kat: i64,
}

impl SupplyCounters {
    /// The encoded width. Pinned as a constant so
    /// [`tests::the_supply_encoding_covers_every_field`] can cross it against
    /// `size_of` — that cross is what makes a silently-added field a test
    /// failure instead of a hole.
    pub const ENCODED_LEN: usize = 40;

    /// A **total function** of every field, in declaration order.
    ///
    /// This is the whole point of the type: G-WET's claim is "mint/burn
    /// counters **bit**-unchanged", so the comparison is over bytes, and the
    /// encoding covering every field is what makes struct equality *be* byte
    /// equality (the riir-dapps `Meta::to_bytes` discipline). Asserting a
    /// hand-picked subset of fields is the failure mode this replaces.
    #[must_use]
    pub fn to_bytes(self) -> [u8; Self::ENCODED_LEN] {
        let mut out = [0u8; Self::ENCODED_LEN];
        out[0..8].copy_from_slice(&self.epoch.to_le_bytes());
        out[8..16].copy_from_slice(&self.mint_kat.to_le_bytes());
        out[16..24].copy_from_slice(&self.burn_kat.to_le_bytes());
        out[24..32].copy_from_slice(&self.pool_paid_kat.to_le_bytes());
        out[32..40].copy_from_slice(&self.pool_available_kat.to_le_bytes());
        out
    }

    /// The all-zero row for an epoch with no telemetry row yet.
    ///
    /// A 404 from `/telemetry` means *nothing has been minted, burned or paid
    /// in this epoch* — so it is a legitimate snapshot, not an unreadable one.
    /// Encoding it as zeros lets an absent-then-still-absent run compare
    /// bit-equal, while a row that APPEARS carrying any nonzero counter moves
    /// the bytes and reds the gate.
    #[must_use]
    pub fn absent(epoch: u64) -> Self {
        Self {
            epoch,
            ..Self::default()
        }
    }
}

/// Parse a `GET /telemetry?epoch=` body into the supply projection.
///
/// # Errors
///
/// A human-readable string on bad JSON, a service `error`, an epoch that is
/// not the one asked for (reading epoch B's row as epoch A's would be the
/// `parse_ledger_health` plane-mismatch defect in another costume), or a
/// missing counter. A missing counter is an ERROR, never a zero — a zeroed
/// mint counter is exactly the reading that would make a money printer look
/// frozen.
pub fn parse_supply(asked_epoch: u64, body: &[u8]) -> Result<SupplyCounters, String> {
    let v: serde_json::Value = serde_json::from_slice(body)
        .map_err(|_| "the telemetry response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the telemetry read was refused — {err}"));
    }
    let epoch = v
        .get("epoch")
        .and_then(serde_json::Value::as_u64)
        .ok_or("the telemetry row carried no `epoch`")?;
    if epoch != asked_epoch {
        return Err(format!(
            "asked for epoch {asked_epoch} and got epoch {epoch} — refusing to read one \
             epoch's counters as another's"
        ));
    }
    let n = |k: &str| -> Result<i64, String> {
        v.get(k)
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| format!("the telemetry row carried no `{k}`"))
    };
    Ok(SupplyCounters {
        epoch,
        mint_kat: n("mint_kat")?,
        burn_kat: n("burn_kat")?,
        pool_paid_kat: n("pool_paid_kat")?,
        pool_available_kat: n("pool_available_kat")?,
    })
}

/// **The G-WET money-printer assertion.** Whole-struct, byte-level.
///
/// # Errors
///
/// A diagnostic naming every field that moved. The message deliberately
/// leads with caveat #2 rather than with the numbers: a nonzero delta here
/// means the loop found an emission channel, which is a design failure, not
/// a tolerance to widen.
pub fn assert_supply_unchanged(
    before: SupplyCounters,
    after: SupplyCounters,
) -> Result<(), String> {
    if before.to_bytes() == after.to_bytes() {
        return Ok(());
    }
    let mut moved = Vec::new();
    let mut note = |name: &str, a: i64, b: i64| {
        if a != b {
            moved.push(format!("{name} {a} → {b}"));
        }
    };
    note(
        "epoch",
        i64::try_from(before.epoch).unwrap_or(i64::MAX),
        i64::try_from(after.epoch).unwrap_or(i64::MAX),
    );
    note("mint_kat", before.mint_kat, after.mint_kat);
    note("burn_kat", before.burn_kat, after.burn_kat);
    note("pool_paid_kat", before.pool_paid_kat, after.pool_paid_kat);
    note(
        "pool_available_kat",
        before.pool_available_kat,
        after.pool_available_kat,
    );
    Err(format!(
        "G-WET FAILED — the wet-run moved a supply counter. The refund is FAUCET-funded by \
         design (Proposal 011 caveat #2: if it ever routes from mint or the burn pool, \
         `wetrun` is a money printer with a test wrapper). Moved: {}",
        moved.join(", ")
    ))
}

// ── the crank report + the riir-dapps Issue 010 diagnostic ──────────────

/// The `POST /admin/crank/daily` report, as the service serves it.
///
/// Every counter is carried, not a subset: `skipped_treasury` is the Issue-010
/// signal and would have been the field a "just read `credited`" parse threw
/// away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CrankReport {
    /// Accounts credited this pass.
    pub credited: u64,
    /// Faucet rows settled this pass.
    pub faucet_settled: u64,
    /// Rows expired this pass.
    pub expired: u64,
    /// Total granted, micro-units.
    pub granted_micro: i64,
    /// Already credited today (the idempotent re-run).
    pub skipped_already: u64,
    /// Outside the activity window.
    pub skipped_dormant: u64,
    /// The daily budget cap was reached.
    pub skipped_budget: u64,
    /// **The Issue-010 signal**: treasury surplus below
    /// `treasury_grant_floor`, so the treasury-ONLY crank skipped.
    pub skipped_treasury: u64,
    /// The per-account daily faucet cap was already used.
    pub skipped_faucet_cap: u64,
    /// The service's own alert verdict.
    pub needs_alert: bool,
}

/// Parse the crank report.
///
/// # Errors
///
/// A human-readable string on bad JSON, a service `error`, or a missing
/// counter. Missing is an error rather than a zero: a zeroed
/// `skipped_treasury` is precisely the reading that would hide Issue 010.
pub fn parse_crank_report(body: &[u8]) -> Result<CrankReport, String> {
    let v: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| "the crank response was not JSON".to_string())?;
    if let Some(err) = v.get("error").and_then(serde_json::Value::as_str) {
        return Err(format!("the crank pass was refused — {err}"));
    }
    let n = |k: &str| -> Result<u64, String> {
        v.get(k)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("the crank report carried no `{k}`"))
    };
    Ok(CrankReport {
        credited: n("credited")?,
        faucet_settled: n("faucet_settled")?,
        expired: n("expired")?,
        granted_micro: v
            .get("granted_micro")
            .and_then(serde_json::Value::as_i64)
            .ok_or("the crank report carried no `granted_micro`")?,
        skipped_already: n("skipped_already")?,
        skipped_dormant: n("skipped_dormant")?,
        skipped_budget: n("skipped_budget")?,
        skipped_treasury: n("skipped_treasury")?,
        skipped_faucet_cap: n("skipped_faucet_cap")?,
        needs_alert: v
            .get("needs_alert")
            .and_then(serde_json::Value::as_bool)
            .ok_or("the crank report carried no `needs_alert`")?,
    })
}

/// What one crank pass did to the faucet leg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaucetLegVerdict {
    /// The leg landed. `settled` faucet rows, `granted_micro` moved from the
    /// treasury.
    Settled {
        /// Faucet rows the pass settled.
        settled: u64,
        /// Micro-units the treasury paid.
        granted_micro: i64,
    },
    /// **riir-dapps Issue 010.** The treasury-ONLY crank found the surplus
    /// below `treasury_grant_floor` and skipped. Under genesis defaults this
    /// is EVERY account, EVERY day.
    TreasuryFloorUnreachable {
        /// How many accounts the pass skipped for this reason.
        skipped: u64,
    },
    /// The pass ran and skipped this account for some other stated reason.
    SkippedOther {
        /// The reason, in the report's own counters.
        reason: String,
    },
}

/// Classify a crank pass's faucet leg — the thing that must never read as a
/// pass when it is a skip.
///
/// `skipped_treasury` is checked BEFORE the other skip reasons: it is the one
/// that indicates a *misconfigured deployment* rather than a *correctly
/// refused account*, and conflating the two is how "the faucet does nothing"
/// looks like "this account was not eligible".
#[must_use]
pub fn classify_faucet_leg(r: &CrankReport) -> FaucetLegVerdict {
    match () {
        () if r.skipped_treasury > 0 => FaucetLegVerdict::TreasuryFloorUnreachable {
            skipped: r.skipped_treasury,
        },
        () if r.faucet_settled > 0 => FaucetLegVerdict::Settled {
            settled: r.faucet_settled,
            granted_micro: r.granted_micro,
        },
        () if r.skipped_budget > 0 => FaucetLegVerdict::SkippedOther {
            reason: format!(
                "the daily grant budget was reached ({} skipped)",
                r.skipped_budget
            ),
        },
        () if r.skipped_faucet_cap > 0 => FaucetLegVerdict::SkippedOther {
            reason: format!(
                "the per-account daily faucet cap was already used ({} skipped)",
                r.skipped_faucet_cap
            ),
        },
        () if r.skipped_already > 0 => FaucetLegVerdict::SkippedOther {
            reason: format!("already credited today ({} skipped)", r.skipped_already),
        },
        () if r.skipped_dormant > 0 => FaucetLegVerdict::SkippedOther {
            reason: format!(
                "outside the activity window ({} skipped)",
                r.skipped_dormant
            ),
        },
        () => FaucetLegVerdict::SkippedOther {
            reason: "the pass settled no faucet row and named no skip reason".to_string(),
        },
    }
}

/// The diagnostic the harness prints — and FAILS on — when the faucet skips
/// because of riir-dapps Issue 010.
///
/// Named as its own function so the wording is pinned by test rather than
/// buried in a format string: this is the message that has to stop a reader
/// from concluding "the faucet is broken, retry later".
#[must_use]
pub fn issue_010_diagnostic(skipped: u64) -> String {
    format!(
        "wetrun step FAILED (not a transient) - the daily crank SKIPPED {skipped} account(s)          with `skipped_treasury`, so no faucet row settled and steps 2/5 cannot complete.
           cause: the treasury-debit grant gate refused - one of TWO distinct states:
           (a) riir-dapps Issue 010 - TREASURY_GRANT_FLOOR_DEFAULT >= MAX_SUPPLY_PLACEHOLDER          (the 2026-09-03 resolution set the floor to 1 000 bp of supply, so this is now the          LEGACY defect; it recurs only if the floor constants regress), or
           (b) an UNFUNDED TREASURY - a freshly stood-up deployment has a zero treasury float,          so `treasury - grant >= floor` cannot hold at any grant size. The crank is          deliberately treasury-ONLY (a mint fallback on an unattended recurring path is what          caveat #2 forbids). Seed the float with the ops bearer: POST /admin/treasury/fund          (exactly-once per event_id_hex), then re-run.
           distinguish them from /health's crank report after ONE pass: a config defect skips          EVERY account on EVERY deployment; an unfunded float skips on THIS deployment only          and `granted_micro` stays 0 while the floor arithmetic checks out."
    )
}

// ── the transport ───────────────────────────────────────────────────────

/// Every service call the wet-run makes, split by side effect.
///
/// The read set (`get_*`) is what a [`RunMode::DryRun`] is allowed to touch;
/// the write set (`post_*`) is what it must not. Splitting them at the TRAIT
/// is what makes "dry-run touches nothing" checkable by a recorded call log
/// instead of by reading the code.
#[cfg(feature = "kat_transport")]
pub trait WetrunTransport {
    /// `GET /health` — the ledger plane.
    fn get_ledger_health(&self) -> HttpResponse;
    /// `GET /mining/health`.
    fn get_mining_health(&self) -> HttpResponse;
    /// `GET /balance?acct=<32 hex>`.
    fn get_balance(&self, acct_hex: &str) -> HttpResponse;
    /// `GET /telemetry?epoch=<n>` — the supply counters. A 404 is the
    /// legitimate "no row in this epoch yet" state.
    fn get_telemetry(&self, epoch: u64) -> HttpResponse;
    /// `POST /payreq/faucet` — open one faucet request.
    ///
    /// Takes NO body: the **implementation** signs it, because the
    /// implementation is what holds the account key. The harness never sees
    /// key material, so it cannot accidentally log, reuse or re-sign it, and
    /// the scripted gate does not have to carry a key to exercise the branch.
    fn open_faucet_request(&self) -> HttpResponse;
    /// `POST /admin/crank/daily` — the pass that SETTLES faucet rows.
    /// Bearer-gated fail-closed on the service; a 401/503 here is an honest
    /// "this operator cannot settle the row", not a harness bug.
    fn post_crank(&self) -> HttpResponse;
    /// `POST /sync` — the signed, redacted push (step 4).
    fn post_sync(&self, ndjson: &str) -> HttpResponse;
}

// ── the run ─────────────────────────────────────────────────────────────

/// The six §7 steps, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// 1 — preflight, read-only.
    Preflight,
    /// 2 — the faucet top-up.
    FaucetTopUp,
    /// 3 — one scored scan→fix cycle, the meter recording the spend.
    ScoredCycle,
    /// 4 — the signed sync push.
    SyncPush,
    /// 5 — the faucet refund credit.
    RefundCredit,
    /// 6 — the mesh readiness verdict.
    MeshReadiness,
}

impl Step {
    /// The §7 label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Preflight => "1 preflight (read-only)",
            Self::FaucetTopUp => "2 faucet top-up",
            Self::ScoredCycle => "3 scored scan→fix cycle",
            Self::SyncPush => "4 signed sync push",
            Self::RefundCredit => "5 faucet refund credit",
            Self::MeshReadiness => "6 mesh readiness",
        }
    }

    /// The two steps a [`RunMode::DryRun`] runs.
    #[must_use]
    pub fn in_dry_run(self) -> bool {
        matches!(self, Self::Preflight | Self::MeshReadiness)
    }
}

/// One step's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRecord {
    /// Which step.
    pub step: Step,
    /// Whether it met its own condition.
    pub ok: bool,
    /// The operator-facing lines.
    pub lines: Vec<String>,
}

/// The run's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every executed step met its condition and the supply is frozen.
    Ready,
    /// The run executed and something did not hold. Carries the reason.
    NotReady(String),
    /// The run never started — [`admit`] said no.
    Refused(String),
}

/// The whole run's record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WetrunReport {
    /// Full or dry-run.
    pub mode: RunMode,
    /// Where it pointed.
    pub network: Network,
    /// Per-step records, in execution order.
    pub steps: Vec<StepRecord>,
    /// The supply snapshot taken before the money legs (`None` in a
    /// dry-run — a dry-run reads no telemetry because it moves nothing).
    pub supply_before: Option<SupplyCounters>,
    /// The snapshot after.
    pub supply_after: Option<SupplyCounters>,
    /// The verdict.
    pub verdict: Verdict,
}

impl WetrunReport {
    /// The G-WET one-liner.
    #[must_use]
    pub fn verdict_line(&self) -> String {
        match &self.verdict {
            Verdict::Ready => format!(
                "wetrun: READY — {} step(s) passed on {}, supply counters bit-unchanged",
                self.steps.len(),
                self.network.label()
            ),
            Verdict::NotReady(why) => {
                format!("wetrun: NOT READY on {} — {why}", self.network.label())
            }
            Verdict::Refused(why) => why.clone(),
        }
    }
}

/// The local legs the harness cannot do over HTTP: the scored cycle (step 3)
/// and the sync payload (step 4).
///
/// A trait rather than concrete code because the CLI wires the REAL meter +
/// proposers + payload builder, and the gate drives scripted ones — and
/// because a harness that fabricated a spend figure would be measuring
/// itself (the self-referential-corpus lesson).
pub trait LocalLegs {
    /// Run one scored scan→fix cycle over the seeded dirty fixture and
    /// return `(fixes_applied, kat_micro_spent)`.
    ///
    /// # Errors
    /// The reason the cycle could not run.
    fn scored_cycle(&mut self) -> Result<(u32, i64), String>;

    /// Compose the signed, redacted sync payload for step 4, returning
    /// `(ndjson, rows, redact_version)`. The redact version is returned so
    /// the harness — not the composer — enforces the `≥ floor` rule.
    ///
    /// # Errors
    /// The reason the payload could not be composed.
    fn sync_payload(&mut self) -> Result<(String, usize, u32), String>;
}

/// The v2 sync envelope's redact floor (§7 step 4 — "redact ≥ floor").
/// Mirrors the service-side `MIN_REDACT_VERSION`; a payload below it is
/// refused rather than pushed.
pub const MIN_REDACT_VERSION: u32 = 2;

/// Run the wet-run.
///
/// The whole function is a straight line of steps with early returns —
/// there is no retry anywhere in it. Every failure is reported with its own
/// reason and stops the run, because each of the six steps is a
/// precondition for the next and "keep going and see" on a money loop is
/// how a skip becomes a pass.
#[cfg(feature = "kat_transport")]
pub fn run(
    transport: &dyn WetrunTransport,
    legs: &mut dyn LocalLegs,
    network: Network,
    mode: RunMode,
    acct_hex: &str,
) -> WetrunReport {
    let mut steps = Vec::new();
    if let Err(why) = admit(network, mode) {
        return WetrunReport {
            mode,
            network,
            steps,
            supply_before: None,
            supply_after: None,
            verdict: Verdict::Refused(why),
        };
    }

    // ── step 1: preflight, read-only ─────────────────────────────────
    let pre = read_planes(transport, acct_hex);
    let pre_ok = matches!(
        pre.gate,
        crate::kat_account_client::WelcomeGate::Open { .. }
    );
    let mut lines = vec![format!(
        "network:   {} (classified from the service URL)",
        network.label()
    )];
    lines.extend(crate::kat_account_client::welcome_gate_lines(&pre.gate));
    lines.push(
        "  fleet:     per-node reachability/height/sync% is `riir-deploy verify --network \
         <net>` (node-sync) — not re-derived here"
            .to_string(),
    );
    steps.push(StepRecord {
        step: Step::Preflight,
        ok: pre_ok,
        lines,
    });
    if !pre_ok {
        return finish(mode, network, steps, None, None);
    }
    let epoch = match &pre.gate {
        crate::kat_account_client::WelcomeGate::Open { epoch, .. } => *epoch,
        _ => 0,
    };

    if mode == RunMode::DryRun {
        // Steps 2–5 are the money legs; a dry-run skips straight to 6 and
        // has by construction issued nothing but the plane reads above.
        steps.push(readiness_step(transport, acct_hex, epoch));
        return finish(mode, network, steps, None, None);
    }

    // The supply snapshot BEFORE any money leg.
    let supply_before = match read_supply(transport, epoch) {
        Ok(s) => s,
        Err(e) => {
            steps.push(StepRecord {
                step: Step::FaucetTopUp,
                ok: false,
                lines: vec![format!(
                    "the supply counters did not read ({e}) — refusing to run the money legs \
                     without the baseline G-WET compares against"
                )],
            });
            return finish(mode, network, steps, None, None);
        }
    };

    // ── step 2: the faucet top-up ────────────────────────────────────
    steps.push(faucet_leg(
        transport,
        Step::FaucetTopUp,
        "the wet-run top-up",
    ));
    if !steps.last().is_some_and(|s| s.ok) {
        return finish(mode, network, steps, Some(supply_before), None);
    }

    // ── step 3: one scored scan→fix cycle ────────────────────────────
    let cycle = legs.scored_cycle();
    steps.push(match &cycle {
        Ok((fixes, spend)) => StepRecord {
            step: Step::ScoredCycle,
            ok: true,
            lines: vec![format!(
                "{fixes} fix(es) applied; the charge meter recorded {} of spend",
                crate::kat_payreq_client::render_kat(*spend)
            )],
        },
        Err(e) => StepRecord {
            step: Step::ScoredCycle,
            ok: false,
            lines: vec![e.clone()],
        },
    });
    if !steps.last().is_some_and(|s| s.ok) {
        return finish(mode, network, steps, Some(supply_before), None);
    }

    // ── step 4: the signed sync push ─────────────────────────────────
    steps.push(sync_leg(transport, legs));
    if !steps.last().is_some_and(|s| s.ok) {
        return finish(mode, network, steps, Some(supply_before), None);
    }

    // ── step 5: the faucet refund credit ─────────────────────────────
    steps.push(faucet_leg(
        transport,
        Step::RefundCredit,
        "the wet-run refund (FAUCET-funded — never mint, never the burn pool)",
    ));
    if !steps.last().is_some_and(|s| s.ok) {
        return finish(mode, network, steps, Some(supply_before), None);
    }

    // ── step 6: mesh readiness + the G-WET supply assertion ──────────
    steps.push(readiness_step(transport, acct_hex, epoch));
    let supply_after = read_supply(transport, epoch).ok();
    finish(mode, network, steps, Some(supply_before), supply_after)
}

/// Steps 1 and 6's read — T0.2b's landed probe, unchanged.
#[cfg(feature = "kat_transport")]
fn read_planes(
    transport: &dyn WetrunTransport,
    acct_hex: &str,
) -> crate::kat_account_client::AccountServiceView {
    struct Adapter<'t>(&'t dyn WetrunTransport);
    impl crate::kat_account_client::AccountTransport for Adapter<'_> {
        fn get_ledger_health(&self) -> HttpResponse {
            self.0.get_ledger_health()
        }
        fn get_mining_health(&self) -> HttpResponse {
            self.0.get_mining_health()
        }
        fn get_balance(&self, acct_hex: &str) -> HttpResponse {
            self.0.get_balance(acct_hex)
        }
    }
    crate::kat_account_client::account_service_view(&Adapter(transport), acct_hex)
}

/// Read the supply snapshot; a 404 is [`SupplyCounters::absent`], not a
/// failure.
#[cfg(feature = "kat_transport")]
fn read_supply(transport: &dyn WetrunTransport, epoch: u64) -> Result<SupplyCounters, String> {
    match transport.get_telemetry(epoch) {
        HttpResponse::Ok(b) => parse_supply(epoch, &b),
        HttpResponse::Status(404) => Ok(SupplyCounters::absent(epoch)),
        HttpResponse::Status(c) => Err(format!("the telemetry read answered HTTP {c}")),
        HttpResponse::NotModified => Err("the telemetry read answered 304".to_string()),
        HttpResponse::Error(e) => Err(format!("the telemetry read failed: {e}")),
    }
}

/// One faucet leg: open the signed request, then run the crank pass that
/// settles it, then classify. `label` names which leg for the operator.
#[cfg(feature = "kat_transport")]
fn faucet_leg(transport: &dyn WetrunTransport, step: Step, label: &str) -> StepRecord {
    let fail = |lines: Vec<String>| StepRecord {
        step,
        ok: false,
        lines,
    };
    // The transport impl signs it (it holds the key) — see the trait doc.
    let opened = match transport.open_faucet_request() {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(c) => {
            return fail(vec![format!("the faucet request was refused (HTTP {c})")]);
        }
        HttpResponse::NotModified => {
            return fail(vec!["the faucet request answered 304".to_string()]);
        }
        HttpResponse::Error(e) => return fail(vec![format!("the faucet request failed: {e}")]),
    };
    let request = match crate::kat_payreq_client::parse_faucet_request(&opened) {
        Ok(r) => r,
        Err(e) => return fail(vec![format!("the faucet request did not read back — {e}")]),
    };
    // Early return, not a `match`: clippy's `single_match` fires on a
    // two-arm match with an empty `_` arm, and a zero pinned amount is a
    // guard rather than a classification.
    if request.amount_micro == 0 {
        return fail(vec![
            "the ledger pinned a faucet amount of 0 — this deployment's \
             `faucet_daily_cap_kat` is unset, so there is nothing to credit"
                .to_string(),
        ]);
    }

    // The crank is what SETTLES it. A skip here is the Issue-010 path.
    let report_body = match transport.post_crank() {
        HttpResponse::Ok(b) => b,
        HttpResponse::Status(401 | 403) => {
            return fail(vec![
                "the crank pass was refused (unauthorized) — the faucet row is OPEN and \
                 unsettled. `POST /admin/crank/daily` is bearer-gated fail-closed; a wet-run \
                 needs the deployment's crank token. This is not retryable without it."
                    .to_string(),
            ]);
        }
        HttpResponse::Status(503) => {
            return fail(vec![
                "the crank is unconfigured at this deployment (503) — `KAT_CRANK_TOKEN` is \
                 unset, so nothing settles faucet rows and steps 2/5 cannot complete."
                    .to_string(),
            ]);
        }
        HttpResponse::Status(c) => return fail(vec![format!("the crank pass answered HTTP {c}")]),
        HttpResponse::NotModified => return fail(vec!["the crank pass answered 304".to_string()]),
        HttpResponse::Error(e) => return fail(vec![format!("the crank pass failed: {e}")]),
    };
    let report = match parse_crank_report(&report_body) {
        Ok(r) => r,
        Err(e) => return fail(vec![format!("the crank report did not read — {e}")]),
    };
    match classify_faucet_leg(&report) {
        FaucetLegVerdict::Settled {
            settled,
            granted_micro,
        } => StepRecord {
            step,
            ok: true,
            lines: vec![
                format!(
                    "{label}: {} pinned by the deployment cap, request {}",
                    crate::kat_payreq_client::render_kat(request.amount_micro),
                    request.id_hex
                ),
                format!(
                    "  crank settled {settled} faucet row(s); the TREASURY paid {} \
                     (grant, not mint)",
                    crate::kat_payreq_client::render_kat(granted_micro)
                ),
            ],
        },
        // The load-bearing branch: loud, diagnostic, terminal.
        FaucetLegVerdict::TreasuryFloorUnreachable { skipped } => {
            fail(vec![issue_010_diagnostic(skipped)])
        }
        FaucetLegVerdict::SkippedOther { reason } => fail(vec![format!(
            "the crank settled no faucet row — {reason}. The row is OPEN and unsettled; this \
             is the ledger's answer, not a timeout."
        )]),
    }
}

/// Step 4: compose the signed payload, enforce the redact floor HERE, push.
#[cfg(feature = "kat_transport")]
fn sync_leg(transport: &dyn WetrunTransport, legs: &mut dyn LocalLegs) -> StepRecord {
    let fail = |lines: Vec<String>| StepRecord {
        step: Step::SyncPush,
        ok: false,
        lines,
    };
    let (ndjson, rows, redact) = match legs.sync_payload() {
        Ok(t) => t,
        Err(e) => return fail(vec![format!("the sync payload did not compose — {e}")]),
    };
    // The floor is checked by the HARNESS, not by the composer: a composer
    // that both builds the payload and vouches for its redaction level is
    // one structure agreeing with itself.
    if redact < MIN_REDACT_VERSION {
        return fail(vec![format!(
            "the payload's redact version {redact} is below the floor {MIN_REDACT_VERSION} — \
             refusing to push (§7 step 4: redact ≥ floor)"
        )]);
    }
    match transport.post_sync(&ndjson) {
        HttpResponse::Ok(b) => {
            // The service's ingest answer (kat-service `mining_route`):
            // `{"accepted":true,"rows_new":N,"rows_refreshed":M,
            //   "rows_terminal_skipped":K}`. `recorded` is accepted as a
            // legacy synonym for rows_new. "The service recorded what I
            // pushed" = new + refreshed == the payload's rows; a skip is
            // surfaced, never hidden.
            let parsed = serde_json::from_slice::<serde_json::Value>(&b).ok();
            let accepted = parsed
                .as_ref()
                .and_then(|v| v.get("accepted").and_then(serde_json::Value::as_bool))
                .unwrap_or(false);
            let n_new = parsed
                .as_ref()
                .and_then(|v| v.get("rows_new").and_then(serde_json::Value::as_u64))
                .or_else(|| {
                    parsed
                        .as_ref()
                        .and_then(|v| v.get("recorded").and_then(serde_json::Value::as_u64))
                });
            let n_refreshed = parsed
                .as_ref()
                .and_then(|v| v.get("rows_refreshed").and_then(serde_json::Value::as_u64))
                .unwrap_or(0);
            let n_skipped = parsed
                .as_ref()
                .and_then(|v| {
                    v.get("rows_terminal_skipped")
                        .and_then(serde_json::Value::as_u64)
                })
                .unwrap_or(0);
            match (accepted, n_new) {
                (true, Some(n)) if usize::try_from(n + n_refreshed) == Ok(rows) => {
                    StepRecord {
                        step: Step::SyncPush,
                        ok: true,
                        lines: vec![format!("pushed {rows} row(s) at redact v{redact}; the service recorded {n} new + {n_refreshed} refreshed ({n_skipped} terminal-skipped) - the self-evolve dry-run is queued service-side")],
                    }
                }
                (true, Some(n)) => fail(vec![format!("pushed {rows} row(s); the service recorded {n} new + {n_refreshed} refreshed ({n_skipped} terminal-skipped) - a partial record is not a successful push")]),
                (true, None) => fail(vec!["the sync answered 200 as accepted but carried no per-row counts - refusing to read that as a successful push".to_string()]),
                (false, _) => fail(vec!["the sync answered 200 without `accepted:true` - refusing to read that as a successful push".to_string()]),
            }
        }
        HttpResponse::Status(c) => fail(vec![format!("the sync push was refused (HTTP {c})")]),
        HttpResponse::NotModified => fail(vec!["the sync push answered 304".to_string()]),
        HttpResponse::Error(e) => fail(vec![format!("the sync push failed: {e}")]),
    }
}

/// Step 6 — the same three reads as step 1, re-run.
#[cfg(feature = "kat_transport")]
fn readiness_step(transport: &dyn WetrunTransport, acct_hex: &str, epoch: u64) -> StepRecord {
    let post = read_planes(transport, acct_hex);
    let ok = match &post.gate {
        crate::kat_account_client::WelcomeGate::Open { epoch: now, .. } => *now >= epoch,
        _ => false,
    };
    let mut lines = crate::kat_account_client::welcome_gate_lines(&post.gate);
    lines.push(
        "  mesh:      run `riir-deploy verify --network <net>` for the per-node \
         reachability/height/sync% table (node-sync, Proposal 011 §6)"
            .to_string(),
    );
    StepRecord {
        step: Step::MeshReadiness,
        ok,
        lines,
    }
}

/// Assemble the verdict: every executed step ok AND (for a full run) the
/// supply frozen.
///
/// Gated with [`run`], its only caller. Without the gate a
/// `kat_service`-without-`kat_transport` build warns `finish is never used`
/// — found by a feature-isolation sweep, which is the only axis that sees
/// it (`--all-features` and the default build are both silent).
#[cfg(feature = "kat_transport")]
fn finish(
    mode: RunMode,
    network: Network,
    steps: Vec<StepRecord>,
    supply_before: Option<SupplyCounters>,
    supply_after: Option<SupplyCounters>,
) -> WetrunReport {
    let failed: Vec<&str> = steps
        .iter()
        .filter(|s| !s.ok)
        .map(|s| s.step.label())
        .collect();
    // Make [`Step::in_dry_run`] load-bearing rather than decorative.
    //
    // Measured: a canary that widened `in_dry_run` to include the faucet
    // top-up changed NOTHING, because `run`'s dry-run branch is a straight
    // line that never consulted it — the declaration and the mechanism
    // could drift silently in both directions. (The `--dry-run` proof
    // itself does not rest on this: the gate's real evidence is the
    // recorded CALL LOG, and a canary that ran a money leg inside the
    // dry-run branch did red it.) This check closes the other direction.
    let illegal: Vec<&str> = match mode {
        RunMode::Full => Vec::new(),
        RunMode::DryRun => steps
            .iter()
            .filter(|s| !s.step.in_dry_run())
            .map(|s| s.step.label())
            .collect(),
    };
    let verdict = match () {
        () if !illegal.is_empty() => Verdict::NotReady(format!(
            "the harness executed money leg(s) inside a --dry-run: {} — a dry-run is steps \
             1+6, read-only, zero state (Proposal 011 §7)",
            illegal.join(", ")
        )),
        () if !failed.is_empty() => {
            Verdict::NotReady(format!("step(s) failed: {}", failed.join(", ")))
        }
        () if mode == RunMode::DryRun => Verdict::Ready,
        // A full run that reached the end MUST have both snapshots; a
        // missing one is a failure to verify, never a pass.
        () => match (supply_before, supply_after) {
            (Some(b), Some(a)) => match assert_supply_unchanged(b, a) {
                Ok(()) => Verdict::Ready,
                Err(e) => Verdict::NotReady(e),
            },
            _ => Verdict::NotReady(
                "the supply counters could not be read on both sides of the run, so the \
                 G-WET money-printer assertion cannot be made — that is a FAIL, not a pass"
                    .to_string(),
            ),
        },
    };
    WetrunReport {
        mode,
        network,
        steps,
        supply_before,
        supply_after,
        verdict,
    }
}

// ── the ureq transport ──────────────────────────────────────────────────

/// The live `WetrunTransport` — one bounded agent over the service base URL
/// (the `HttpPayreqTransport` / `HttpAccountTransport` shape).
///
/// **The crank bearer arrives as a constructor argument, never from an
/// environment read here.** `POST /admin/crank/daily` is bearer-gated
/// fail-closed service-side, so the wet-run needs the deployment's token —
/// but this module is pinned environment-free by
/// [`tests::the_mainnet_refusal_reads_no_environment`] precisely so no
/// future env read can grow into an admission override. Resolving the token
/// is the CLI's job.
///
/// **The account key lives here** so the harness never handles key material:
/// [`WetrunTransport::open_faucet_request`] signs the body itself.
#[cfg(feature = "kat_transport")]
pub struct HttpWetrunTransport {
    agent: ureq::Agent,
    base: String,
    key: ed25519_dalek::SigningKey,
    crank_bearer: Option<String>,
}

#[cfg(feature = "kat_transport")]
impl HttpWetrunTransport {
    /// Build with a per-request timeout in seconds (connect, headers, AND
    /// body read).
    #[must_use]
    pub fn new(
        base: &str,
        timeout_secs: u64,
        key: ed25519_dalek::SigningKey,
        crank_bearer: Option<String>,
    ) -> Self {
        let t = std::time::Duration::from_secs(timeout_secs.max(1));
        let agent = ureq::AgentBuilder::new()
            .timeout(t)
            .timeout_connect(t)
            .timeout_read(t)
            .build();
        Self {
            agent,
            base: base.trim_end_matches('/').to_string(),
            key,
            crank_bearer,
        }
    }

    fn body(resp: ureq::Response) -> HttpResponse {
        use std::io::Read as _;
        match resp.status() {
            304 => HttpResponse::NotModified,
            200 => {
                let mut buf = Vec::new();
                match resp.into_reader().read_to_end(&mut buf) {
                    Ok(_) => HttpResponse::Ok(buf),
                    Err(e) => HttpResponse::Error(e.to_string()),
                }
            }
            c => HttpResponse::Status(c),
        }
    }

    fn get(&self, path: &str) -> HttpResponse {
        match self.agent.get(&format!("{}{path}", self.base)).call() {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }
}

#[cfg(feature = "kat_transport")]
impl WetrunTransport for HttpWetrunTransport {
    fn get_ledger_health(&self) -> HttpResponse {
        self.get("/health")
    }

    fn get_mining_health(&self) -> HttpResponse {
        self.get("/mining/health")
    }

    fn get_balance(&self, acct_hex: &str) -> HttpResponse {
        self.get(&format!("/balance?acct={acct_hex}"))
    }

    fn get_telemetry(&self, epoch: u64) -> HttpResponse {
        self.get(&format!("/telemetry?epoch={epoch}"))
    }

    fn open_faucet_request(&self) -> HttpResponse {
        // The payreq transport already carries the refusal-body rule (a
        // non-200 with `{"error":...}` is the operator's answer, not an
        // opaque status), so the faucet POST goes through it rather than
        // re-deriving that here.
        use crate::kat_payreq_client::PayreqTransport as _;
        crate::kat_payreq_client::HttpPayreqTransport::new(&self.base, 10)
            .post_faucet(&crate::kat_payreq_client::faucet_request_body(&self.key))
    }

    fn post_crank(&self) -> HttpResponse {
        // Fail-closed on the CLIENT side too: no token, no request. A POST
        // without the bearer would come back 401 and read like a service
        // problem; saying it here names the missing input instead.
        let Some(bearer) = self.crank_bearer.as_deref() else {
            return HttpResponse::Status(401);
        };
        match self
            .agent
            .post(&format!("{}/admin/crank/daily", self.base))
            .set("Authorization", &format!("Bearer {bearer}"))
            .set("Content-Type", "application/json")
            .send_string("{}")
        {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }

    fn post_sync(&self, ndjson: &str) -> HttpResponse {
        match self
            .agent
            .post(&format!("{}/sync", self.base))
            .set("Content-Type", "application/x-ndjson")
            .send_string(ndjson)
        {
            Ok(r) => Self::body(r),
            Err(ureq::Error::Status(c, _)) => HttpResponse::Status(c),
            Err(e) => HttpResponse::Error(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── the network boundary ─────────────────────────────────────────

    #[test]
    fn classify_service_url_places_hosts_and_fails_closed() {
        for (url, want) in [
            ("http://127.0.0.1:8787", Network::Devnet),
            ("http://localhost:8787/", Network::Devnet),
            ("http://192.168.1.33:8787", Network::Devnet),
            ("http://kat.devnet.gist.rs", Network::Devnet),
            ("https://kat-testnet.gist.rs", Network::Testnet),
            ("https://kat.testnet.gist.rs/payreq", Network::Testnet),
            ("https://kat-mainnet.gist.rs", Network::Mainnet),
            // Substring, not prefix — the riir-deployer `is_mainnet` rule.
            ("https://chain-mainnet.gist.rs", Network::Mainnet),
            ("https://MAINNET-eu.gist.rs", Network::Mainnet),
            // A host naming BOTH is mainnet: refuse, never admit.
            ("https://testnet-mainnet.gist.rs", Network::Mainnet),
            // The production default carries no network word → refused.
            ("https://heal.gist.rs", Network::Unclassifiable),
            ("", Network::Unclassifiable),
        ] {
            assert_eq!(classify_service_url(url), want, "{url}");
        }
    }

    #[test]
    fn a_full_run_is_refused_off_devnet_testnet_and_a_dry_run_is_admitted_everywhere() {
        for net in [Network::Devnet, Network::Testnet] {
            assert!(admit(net, RunMode::Full).is_ok(), "{net:?}");
        }
        for net in [Network::Mainnet, Network::Unclassifiable] {
            let err = admit(net, RunMode::Full).expect_err("must refuse");
            assert!(err.contains("REFUSED"), "{err}");
            assert!(
                err.contains("--dry-run"),
                "the refusal must name the way out: {err}"
            );
        }
        // The mainnet refusal states that no override exists, so an
        // operator does not go looking for one.
        let err = admit(Network::Mainnet, RunMode::Full).expect_err("refused");
        assert!(err.contains("NO override"), "{err}");
        // Every network admits a dry-run — including mainnet.
        for net in [
            Network::Devnet,
            Network::Testnet,
            Network::Mainnet,
            Network::Unclassifiable,
        ] {
            assert!(admit(net, RunMode::DryRun).is_ok(), "{net:?}");
        }
    }

    /// **There is no mainnet override, and this pins that structurally.**
    ///
    /// Caveat #2: a `RIIR_ALLOW_MAINNET_WETRUN` would be "an attack on our
    /// own bookkeeping". A behavioural test cannot prove the absence of an
    /// env hatch (you cannot enumerate every variable name), so the pin is
    /// a source-level invariant over the WHOLE module: it reads the
    /// environment nowhere at all. That also covers a hatch added to
    /// [`classify_service_url`], which would be the sneakier place for one.
    ///
    /// Canaried: adding `std::env::var("X")` anywhere in this file reds it.
    /// The forbidden spellings. `ALLOW_MAINNET` is in the list on purpose:
    /// the hatch would be *named* before it was *read*.
    const NO_ENV_NEEDLES: &[&str] = &["std::env", "env::var", "var_os", "ALLOW_MAINNET", "getenv"];

    /// Strip line comments so a **mention** of the forbidden hatch in the
    /// module docs is not read as a **use** of it. (This module's docs name
    /// `RIIR_ALLOW_MAINNET_WETRUN` explicitly, as the thing that must not
    /// exist — and the first cut of this audit failed on its own
    /// documentation, which is the right kind of false positive to have
    /// found before shipping.)
    fn code_only(src: &str) -> String {
        src.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_mainnet_refusal_reads_no_environment() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/kat_wetrun.rs");
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        // The instrument checks itself first: it must be reading the real
        // module, and the comment stripper must not have eaten the code.
        let body = code_only(&src[..src.find("\nmod tests {").unwrap_or(src.len())]);
        assert!(
            body.contains("pub fn admit(")
                && body.contains("Network::Mainnet")
                && body.len() > 4_000,
            "the source audit did not find this module's code — a scan over nothing is a \
             false green (found {} bytes)",
            body.len()
        );
        // And the predicate must actually fire on a real code use: without
        // this, a regressed stripper would report a confident clean.
        assert!(
            NO_ENV_NEEDLES
                .iter()
                .any(|n| code_only("let hatch = std::env::var(\"X\");").contains(n)),
            "the instrument cannot detect an environment read it is meant to forbid"
        );
        assert!(
            !NO_ENV_NEEDLES
                .iter()
                .any(|n| code_only("//! never set RIIR_ALLOW_MAINNET_WETRUN").contains(n)),
            "the instrument must not read a doc MENTION as a code use"
        );

        for forbidden in NO_ENV_NEEDLES {
            assert!(
                !body.contains(forbidden),
                "`{forbidden}` appears in kat_wetrun's non-test CODE — the mainnet refusal \
                 has NO override by design (Proposal 011 caveat #2). If a genuine \
                 environment read is needed, it must not be able to widen `admit`."
            );
        }
    }

    // ── the money-printer guard ──────────────────────────────────────

    /// The encoding is a TOTAL function of the struct, and this is what
    /// makes that checkable rather than asserted.
    ///
    /// `size_of` is crossed against [`SupplyCounters::ENCODED_LEN`]: add a
    /// field without extending `to_bytes` and `size_of` moves while the
    /// encoded length does not, so this reds instead of the new counter
    /// silently escaping the G-WET comparison (the riir-dapps `Meta`
    /// lesson, applied to the wire view).
    #[test]
    fn the_supply_encoding_covers_every_field() {
        assert_eq!(
            std::mem::size_of::<SupplyCounters>(),
            SupplyCounters::ENCODED_LEN,
            "a field was added to SupplyCounters without extending to_bytes — the new counter \
             would escape the G-WET bit-comparison. Extend to_bytes AND ENCODED_LEN together."
        );
        // Every field must MOVE the bytes: an encoded field no test varies
        // is an assumption, not a pin.
        let base = SupplyCounters {
            epoch: 3,
            mint_kat: 5,
            burn_kat: 7,
            pool_paid_kat: 11,
            pool_available_kat: 13,
        };
        let probes = [
            SupplyCounters { epoch: 4, ..base },
            SupplyCounters {
                mint_kat: 6,
                ..base
            },
            SupplyCounters {
                burn_kat: 8,
                ..base
            },
            SupplyCounters {
                pool_paid_kat: 12,
                ..base
            },
            SupplyCounters {
                pool_available_kat: 14,
                ..base
            },
        ];
        assert_eq!(
            probes.len(),
            SupplyCounters::ENCODED_LEN / 8,
            "one divergence probe per encoded 8-byte field"
        );
        for (i, p) in probes.iter().enumerate() {
            assert_ne!(
                p.to_bytes(),
                base.to_bytes(),
                "field {i} does not move the bytes"
            );
            assert!(
                assert_supply_unchanged(base, *p).is_err(),
                "field {i} escapes the gate"
            );
        }
        assert!(assert_supply_unchanged(base, base).is_ok());
    }

    #[test]
    fn the_gwet_failure_leads_with_caveat_2_and_names_the_moved_counter() {
        let before = SupplyCounters {
            epoch: 3,
            mint_kat: 0,
            burn_kat: 0,
            pool_paid_kat: 0,
            pool_available_kat: 0,
        };
        let after = SupplyCounters {
            mint_kat: 10_000_000,
            ..before
        };
        let err = assert_supply_unchanged(before, after).expect_err("a mint moved");
        assert!(err.starts_with("G-WET FAILED"), "{err}");
        assert!(err.contains("money printer with a test wrapper"), "{err}");
        assert!(err.contains("mint_kat 0 → 10000000"), "{err}");
        // The refund is faucet-funded; the message must say so rather than
        // read as a tolerance to widen.
        assert!(err.contains("FAUCET-funded"), "{err}");
    }

    /// An absent telemetry row is a legitimate all-zero snapshot; a row
    /// that APPEARS carrying a counter is a change.
    #[test]
    fn an_absent_row_is_zeros_and_a_row_appearing_with_a_counter_reds_the_gate() {
        let absent = SupplyCounters::absent(9);
        assert_eq!(absent.mint_kat, 0);
        assert_eq!(absent.epoch, 9);
        assert!(assert_supply_unchanged(absent, SupplyCounters::absent(9)).is_ok());
        assert!(
            assert_supply_unchanged(
                absent,
                SupplyCounters {
                    mint_kat: 1,
                    ..absent
                }
            )
            .is_err()
        );
        // And a snapshot from a DIFFERENT epoch cannot masquerade as
        // unchanged — the epoch rides in the bytes.
        assert!(assert_supply_unchanged(absent, SupplyCounters::absent(10)).is_err());
    }

    #[test]
    fn parse_supply_refuses_a_foreign_epoch_a_refusal_and_a_missing_counter() {
        let row = |epoch: u64| {
            serde_json::json!({
                "epoch": epoch, "burn_kat": 0, "mint_kat": 0,
                "pool_paid_kat": 0, "pool_available_kat": 0
            })
            .to_string()
            .into_bytes()
        };
        assert_eq!(
            parse_supply(4, &row(4)).expect("reads"),
            SupplyCounters::absent(4)
        );
        // Reading epoch 5's row as epoch 4's would be the plane-mismatch
        // defect in another costume.
        let err = parse_supply(4, &row(5)).expect_err("epoch mismatch");
        assert!(
            err.contains("refusing to read one epoch's counters as another's"),
            "{err}"
        );
        // A missing counter is an ERROR, never a zero — a zeroed mint is
        // exactly the reading that makes a printer look frozen.
        let partial = serde_json::json!({"epoch": 4, "burn_kat": 0}).to_string();
        let err = parse_supply(4, partial.as_bytes()).expect_err("partial row");
        assert!(err.contains("mint_kat"), "{err}");
        let err = parse_supply(4, b"{\"error\":\"nope\"}").expect_err("refusal");
        assert!(err.contains("refused"), "{err}");
        assert!(parse_supply(4, b"not json").is_err());
    }

    // ── the riir-dapps Issue 010 path ────────────────────────────────

    fn crank_json(fields: &serde_json::Value) -> Vec<u8> {
        let mut base = serde_json::json!({
            "date": "2026-09-02", "credited": 0, "faucet_settled": 0, "expired": 0,
            "granted_micro": 0, "skipped_already": 0, "skipped_dormant": 0,
            "skipped_budget": 0, "skipped_treasury": 0, "skipped_faucet_cap": 0,
            "needs_alert": false
        });
        for (k, v) in fields.as_object().expect("object") {
            base[k] = v.clone();
        }
        base.to_string().into_bytes()
    }

    /// `skipped_treasury` is classified BEFORE every other skip reason.
    ///
    /// It marks a MISCONFIGURED DEPLOYMENT (riir-dapps Issue 010), while
    /// the others mark a correctly-refused ACCOUNT. Conflating them is how
    /// "the faucet does nothing, forever" reads as "this account was not
    /// eligible today".
    #[test]
    fn the_treasury_floor_skip_outranks_every_other_skip_reason() {
        let both = parse_crank_report(&crank_json(&serde_json::json!({
            "skipped_treasury": 3, "skipped_dormant": 5, "skipped_already": 2,
            "faucet_settled": 1, "needs_alert": true
        })))
        .expect("reads");
        assert_eq!(
            classify_faucet_leg(&both),
            FaucetLegVerdict::TreasuryFloorUnreachable { skipped: 3 },
            "a treasury skip must not be masked by a settle or another skip in the same pass"
        );

        // The healthy pass.
        let ok = parse_crank_report(&crank_json(&serde_json::json!({
            "faucet_settled": 1, "credited": 1, "granted_micro": 10_000_000
        })))
        .expect("reads");
        assert_eq!(
            classify_faucet_leg(&ok),
            FaucetLegVerdict::Settled {
                settled: 1,
                granted_micro: 10_000_000
            }
        );

        // Each other reason gets its own sentence, and NONE of them is
        // reported as a settle.
        for (field, needle) in [
            ("skipped_budget", "daily grant budget"),
            ("skipped_faucet_cap", "daily faucet cap"),
            ("skipped_already", "already credited today"),
            ("skipped_dormant", "activity window"),
        ] {
            let r =
                parse_crank_report(&crank_json(&serde_json::json!({ field: 1 }))).expect("reads");
            match classify_faucet_leg(&r) {
                FaucetLegVerdict::SkippedOther { reason } => {
                    assert!(reason.contains(needle), "{field}: {reason}");
                }
                other => panic!("{field} classified as {other:?}"),
            }
        }
        // A pass that did nothing and named nothing is still NOT a settle.
        let silent = parse_crank_report(&crank_json(&serde_json::json!({}))).expect("reads");
        assert!(matches!(
            classify_faucet_leg(&silent),
            FaucetLegVerdict::SkippedOther { .. }
        ));
    }

    /// The diagnostic has to stop a reader from concluding "retry later".
    #[test]
    fn the_issue_010_diagnostic_names_the_issue_the_constants_and_the_owner_act() {
        let d = issue_010_diagnostic(7);
        assert!(d.contains("Issue 010"), "{d}");
        assert!(d.contains("TREASURY_GRANT_FLOOR_DEFAULT"), "{d}");
        assert!(d.contains("skipped_treasury"), "{d}");
        assert!(d.contains("7 account(s)"), "{d}");
        // The two-cause split (T4.4a live finding): a config defect vs an
        // unfunded treasury on a freshly stood-up deployment — the fix is
        // different (owner supply constants vs the ops fund route).
        assert!(d.contains("UNFUNDED TREASURY"), "{d}");
        assert!(d.contains("/admin/treasury/fund"), "{d}");
        assert!(d.contains("treasury-ONLY"), "{d}");
        assert!(d.contains("caveat #2"), "{d}");
    }

    #[test]
    fn parse_crank_report_refuses_a_partial_report() {
        // Dropping `skipped_treasury` must be an error, not a zero — a
        // zeroed treasury-skip counter is precisely what would hide 010.
        let mut v: serde_json::Value =
            serde_json::from_slice(&crank_json(&serde_json::json!({}))).expect("json");
        v.as_object_mut().expect("obj").remove("skipped_treasury");
        let err = parse_crank_report(v.to_string().as_bytes()).expect_err("partial");
        assert!(err.contains("skipped_treasury"), "{err}");
        let err = parse_crank_report(b"{\"error\":\"crank unauthorized\"}").expect_err("refusal");
        assert!(err.contains("crank unauthorized"), "{err}");
    }

    // ── step bookkeeping ─────────────────────────────────────────────

    #[test]
    fn exactly_two_of_the_six_steps_run_in_a_dry_run() {
        let all = [
            Step::Preflight,
            Step::FaucetTopUp,
            Step::ScoredCycle,
            Step::SyncPush,
            Step::RefundCredit,
            Step::MeshReadiness,
        ];
        let dry: Vec<Step> = all.into_iter().filter(|s| s.in_dry_run()).collect();
        assert_eq!(
            dry,
            [Step::Preflight, Step::MeshReadiness],
            "§7: steps 1+6 only"
        );
        // The four money legs are exactly the complement.
        assert_eq!(all.len() - dry.len(), 4);
        for s in all {
            assert!(!s.label().is_empty());
        }
    }

    /// The floor is the v2 envelope's version. Gated on
    /// `latent_retrieval` because that is where `kat_sync_payload` lives —
    /// and stated here rather than left implicit, since a cfg'd-out test is
    /// a green that measured nothing (the "disbelieve a cfg-gated pass"
    /// rule). The `kat_transport`-only build simply does not have the
    /// module to compare against.
    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn the_redact_floor_matches_the_service_envelope_version() {
        assert_eq!(
            MIN_REDACT_VERSION,
            crate::kat_sync_payload::PAYLOAD_VERSION,
            "§7 step 4's floor is the v2 sync envelope's version — if the payload version \
             moves, this floor is the thing that decides whether an OLD payload is still \
             pushable, so the drift must be a deliberate edit here"
        );
    }
}

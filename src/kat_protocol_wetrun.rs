//! Wire types for wetrun — pure serde, no transport, no policy (riir-clippy issue 088 protocol tier).
//!
//! The report/supply/step shapes moved verbatim out of `crate::kat_wetrun`,
//! which re-exports every name here so all existing `crate::kat_wetrun::X`
//! paths resolve unchanged. No imports: the JSON parsers
//! (`parse_supply` / `parse_crank_report`), the admission policy
//! (`admit` / `classify_service_url`), the G-WET assertion, the transport
//! traits and the run driver stay with the client — this tier owns only the
//! shapes they produce and consume. (The wetrun wire views are hand-parsed
//! JSON, so no serde derives ride on these types.)

// ── the network boundary ─────────────────────────────────────────────────

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

/// Full money run vs the read-only rehearsal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// All six steps. Devnet/testnet only.
    Full,
    /// Steps 1 + 6 only — read-only, zero state. Allowed everywhere.
    DryRun,
}

// ── the money-printer guard: the supply counters on the wire ────────────

/// The supply-bearing counters the service exposes at
/// `GET /telemetry?epoch=<n>` — the wire view of riir-dapps' `Meta` row.
///
/// Integer money only (G4-S): every amount is `i64` **micro-KAT**
/// (1 KAT = `1_000_000` micro), the unit the service actually serves — the
/// `_micro` suffix is load-bearing, because these keys spent their whole
/// life named `*_kat` while carrying micro (riir-dapps `.issues/033`). A
/// float anywhere in here would be review-blocking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SupplyCounters {
    /// The epoch this row belongs to. Carried IN the snapshot so a
    /// comparison across two different epochs cannot masquerade as a
    /// bit-unchanged claim — it changes the bytes and reds the gate.
    pub epoch: u64,
    /// Mint accrued in this epoch, in micro-KAT.
    pub mint_micro: i64,
    /// Burn accrued in this epoch, in micro-KAT.
    pub burn_micro: i64,
    /// Mining credits paid from the pool, in micro-KAT.
    pub pool_paid_micro: i64,
    /// Pool backing available to mining credits, in micro-KAT.
    pub pool_available_micro: i64,
}

impl SupplyCounters {
    /// The encoded width. Pinned as a constant so the
    /// `the_supply_encoding_covers_every_field` test can cross it against
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
        out[8..16].copy_from_slice(&self.mint_micro.to_le_bytes());
        out[16..24].copy_from_slice(&self.burn_micro.to_le_bytes());
        out[24..32].copy_from_slice(&self.pool_paid_micro.to_le_bytes());
        out[32..40].copy_from_slice(&self.pool_available_micro.to_le_bytes());
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

// ── the crank report + the riir-dapps Issue 010 verdict ─────────────────

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

// ── the run's step bookkeeping ───────────────────────────────────────────

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
    /// The run never started — `admit` said no.
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

/// The v2 sync envelope's redact floor (§7 step 4 — "redact ≥ floor").
/// Mirrors the service-side `MIN_REDACT_VERSION`; a payload below it is
/// refused rather than pushed.
pub const MIN_REDACT_VERSION: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

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
}

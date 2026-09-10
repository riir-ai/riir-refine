//! The 075 soft gate — the depleted-account refusal for write/heal runs.
//!
//! Owner decision (2026-09-08, `.issues/075` §"Zero-balance decision"):
//! **prepaid fuel, not an interactive gate** — burn the free grant run by
//! run; when it runs out, plain runs REFUSE until the user contributes
//! (`--mine` + sync) or tops up. Logged-out / offline / never-synced
//! machines stay warn-only: no server authority, no honest gate.
//!
//! # The predicate (all four required — 075 implementation prerequisite 2)
//!
//! 1. **transport-capable build** — the input is the last SYNCED server
//!    balance (carried on the burn-push ack), which only a `kat_transport`
//!    build can obtain; every other build is warn-only by construction.
//! 2. **account configured** — the gate is an account-state fact; anonymous
//!    runs are never gated (the funnel stays open).
//! 3. **last-synced balance ≤ 0** — SERVER truth from the burn ack, never
//!    the local projection (the projection models neither the daily faucet
//!    drip nor mining credits — 075 prerequisite 1). `None` (never synced)
//!    is NOT ≤ 0: it is warn-only.
//! 4. **not yet contributed-and-synced** — SERVER TRUTH ONLY (issue 080,
//!    dated removal 2026-09-10): the burn-ack v3 `contributed` field
//!    (written only from the authenticated burn-push response) is the sole
//!    input — `Some(true)` lifts the gate, and `None`/`Some(false)` are
//!    simply NOT contributed (the balance leg then decides). The legacy
//!    machine-local `contributed_once.json` marker that once filled in
//!    while the ack said `None` is RETIRED: the `kat_contribution` module
//!    is deleted and any file a pre-removal install left on disk has ZERO
//!    effect — the forge surface is fully removed. The sticky-grace
//!    semantics are unchanged: this IS the `OptedIn` grace — mining payouts
//!    land at the 7-day epoch settle (`pay_due_rewards` rides
//!    `settle_epoch`, NOT the hourly burn bucket), so a time-boxed grace
//!    would brick contributing users mid-epoch — the exact failure 075
//!    forbids. Once the server has stated `true`, the gate stays lifted
//!    (sticky); the balance leg alone re-arms it for fresh depleted
//!    accounts.
//!
//! An env kill-switch (`RIIR_HEAL_KAT_GATE=0`) and the shared billing
//! kill-switch (`RIIR_HEAL_KAT_METER=0`) both disarm the gate at the call
//! site — the caller decides; this module is the pure predicate.

/// Everything the predicate reads, gathered by the caller (the bin owns the
/// feature cfg + the file paths; this module owns the logic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoftGateInput {
    /// `kat_transport` compiled in — without it there is no synced balance.
    pub transport_capable: bool,
    /// The account loaded (the machine is logged in).
    pub account_present: bool,
    /// The last-synced server balance, micro-units (`None` = never synced).
    pub last_synced_balance_micro: Option<i64>,
    /// The sticky-grace leg, pre-bridged by the caller (issue 080, dated
    /// removal 2026-09-10): the burn-ack v3 `contributed` server truth —
    /// `Some(true)` only. A planted/leftover `contributed_once.json` has
    /// zero effect (the `kat_contribution` reader is deleted); `None` and
    /// `Some(false)` both arrive here as `false`.
    pub contributed_and_synced: bool,
}

/// The verdict. `Refuse` carries the depleted balance for the message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftGateVerdict {
    /// Run proceeds (the normal case).
    Allow,
    /// No honest gate input — warn-only posture (the pre-075 behavior).
    WarnOnly(WarnOnlyReason),
    /// All four predicate legs held: the run is refused.
    Refuse {
        /// The depleted synced balance (≤ 0), micro-units.
        balance_micro: i64,
    },
}

/// Why the gate has no authority here (the warn-only taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarnOnlyReason {
    /// No `kat_transport` — no synced balance can exist.
    NoTransport,
    /// No account — anonymous runs are never gated.
    NoAccount,
    /// The account never completed a burn push — no server truth yet.
    NeverSynced,
}

/// The pure predicate. Total function; no I/O, no env, no clock.
#[must_use]
pub fn evaluate(input: SoftGateInput) -> SoftGateVerdict {
    if !input.transport_capable {
        return SoftGateVerdict::WarnOnly(WarnOnlyReason::NoTransport);
    }
    if !input.account_present {
        return SoftGateVerdict::WarnOnly(WarnOnlyReason::NoAccount);
    }
    let Some(balance) = input.last_synced_balance_micro else {
        return SoftGateVerdict::WarnOnly(WarnOnlyReason::NeverSynced);
    };
    if input.contributed_and_synced {
        return SoftGateVerdict::Allow; // the sticky OptedIn grace
    }
    if balance > 0 {
        return SoftGateVerdict::Allow;
    }
    SoftGateVerdict::Refuse {
        balance_micro: balance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(
        transport: bool,
        account: bool,
        balance: Option<i64>,
        contributed: bool,
    ) -> SoftGateInput {
        SoftGateInput {
            transport_capable: transport,
            account_present: account,
            last_synced_balance_micro: balance,
            contributed_and_synced: contributed,
        }
    }

    #[test]
    fn the_four_legs_of_the_predicate() {
        // All four held → refuse.
        assert_eq!(
            evaluate(input(true, true, Some(0), false)),
            SoftGateVerdict::Refuse { balance_micro: 0 }
        );
        assert_eq!(
            evaluate(input(true, true, Some(-5), false)),
            SoftGateVerdict::Refuse { balance_micro: -5 }
        );

        // Each leg broken → allow or warn-only.
        assert_eq!(
            evaluate(input(false, true, Some(0), false)),
            SoftGateVerdict::WarnOnly(WarnOnlyReason::NoTransport)
        );
        assert_eq!(
            evaluate(input(true, false, Some(0), false)),
            SoftGateVerdict::WarnOnly(WarnOnlyReason::NoAccount)
        );
        assert_eq!(
            evaluate(input(true, true, None, false)),
            SoftGateVerdict::WarnOnly(WarnOnlyReason::NeverSynced)
        );
        assert_eq!(
            evaluate(input(true, true, Some(0), true)),
            SoftGateVerdict::Allow
        );
        assert_eq!(
            evaluate(input(true, true, Some(1), false)),
            SoftGateVerdict::Allow
        );
    }

    #[test]
    fn the_sticky_contribution_grace_outlives_a_zero_balance() {
        // The 075 grace: contributed + depleted mid-epoch (payouts land at
        // the 7-day settle) must NOT refuse — any balance, including the
        // warn-and-pay floor and a deep negative clamp.
        for balance in [Some(0), Some(-1), Some(i64::MIN)] {
            assert_eq!(
                evaluate(input(true, true, balance, true)),
                SoftGateVerdict::Allow,
                "contributed accounts are never gated (balance {balance:?})"
            );
        }
    }

    #[test]
    fn warn_only_is_the_missing_truth_direction_not_the_depleted_direction() {
        // A never-synced account with a DEPLETED local projection must still
        // be warn-only: the projection is not the gate's input (075
        // prerequisite 1 — it models neither the faucet nor mining credits).
        assert_eq!(
            evaluate(input(true, true, None, false)),
            SoftGateVerdict::WarnOnly(WarnOnlyReason::NeverSynced)
        );
    }
}

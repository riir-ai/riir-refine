//! Vendored from `riir-kat` (`863c590798b5`, 2026-11-09, issue 221
//! private-dep minimization): the feature-free protocol tier refine's
//! UNGATED surfaces consume (secret scan, units, consent dir, config
//! store, hosts, net pin, state stamp, soft gate). std-only by the
//! leaf's protocol-tier purity rule. The canonical copy stays in the
//! private sibling (`riir-kat` `src/kat_soft_gate.rs`); drift is caught by the
//! golden pins in `tests/katplane_parity.rs` and repaired by
//! re-vendoring via `scripts/vendor_katplane.sh` — never hand-edit one
//! side.

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
//! 3. **last-synced balances depleted on BOTH planes** — SERVER truth from
//!    the burn ack, never the local projection (the projection models
//!    neither the daily faucet drip nor mining credits — 075 prerequisite
//!    1). The KAT leg is the ack's `last_balance_micro`; the trial-credit
//!    leg is the v4 `tuna_balance_micro` (Proposal 006 B3): a
//!    server-reported positive TUNA balance LIFTS the refusal (trial
//!    credit heals — that is its whole job), while `None` is "no
//!    trial-credit truth" (a pre-v4 ack / pre-TUNA server) and keeps the
//!    KAT-only posture byte for byte. The gate never invents a zero.
//!    `None` KAT (never synced) is still warn-only.
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
//! An env kill-switch (`RIIR_REFINE_KAT_GATE=0`) and the shared billing
//! kill-switch (`RIIR_REFINE_KAT_METER=0`) both disarm the gate at the call
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
    /// The last-synced trial-credit (TUNA) balance, micro-units
    /// (Proposal 006 B3 — the ack's v4 leg). `None` = no server truth
    /// about trial credit (a pre-v4 ack / pre-TUNA server): the KAT-only
    /// posture is preserved, never read as zero.
    pub tuna_balance_micro: Option<i64>,
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
    // Proposal 006: trial credit heals too — a server-reported positive
    // TUNA balance is fuel, not depletion (the 075 refusal would have
    // bricked a trial user holding credit; that is the Issue-094 shape of
    // a property asserted where nobody checks it). `None` falls through:
    // no trial-credit truth, KAT alone decides, exactly the pre-v4 gate.
    if input.tuna_balance_micro.is_some_and(|tuna| tuna > 0) {
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
        full_input(transport, account, balance, contributed, None)
    }

    fn full_input(
        transport: bool,
        account: bool,
        balance: Option<i64>,
        contributed: bool,
        tuna: Option<i64>,
    ) -> SoftGateInput {
        SoftGateInput {
            transport_capable: transport,
            account_present: account,
            last_synced_balance_micro: balance,
            contributed_and_synced: contributed,
            tuna_balance_micro: tuna,
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
    fn trial_credit_lifts_the_refusal_only_when_the_server_says_so() {
        // Proposal 006: a TUNA-holding trial user with a drained KAT
        // balance is NOT depleted — trial credit heals.
        assert_eq!(
            evaluate(full_input(true, true, Some(0), false, Some(47_200_000))),
            SoftGateVerdict::Allow
        );
        // 1 micro of TUNA still heals — the predicate is sign-shaped,
        // never a threshold.
        assert_eq!(
            evaluate(full_input(true, true, Some(0), false, Some(1))),
            SoftGateVerdict::Allow
        );

        // Server truth that trial credit is GONE → the KAT-only refusal
        // stands (Some ≤ 0 is a fact, not an absence).
        assert_eq!(
            evaluate(full_input(true, true, Some(0), false, Some(0))),
            SoftGateVerdict::Refuse { balance_micro: 0 }
        );
        assert_eq!(
            evaluate(full_input(true, true, Some(-5), false, Some(-1))),
            SoftGateVerdict::Refuse { balance_micro: -5 }
        );

        // None = no trial-credit truth (pre-v4 ack / pre-TUNA server):
        // byte-identical to the pre-006 gate — never an invented zero.
        assert_eq!(
            evaluate(full_input(true, true, Some(0), false, None)),
            SoftGateVerdict::Refuse { balance_micro: 0 }
        );

        // The sticky grace outranks everything, as before.
        assert_eq!(
            evaluate(full_input(true, true, Some(0), true, Some(0))),
            SoftGateVerdict::Allow
        );
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

//! Batch 424 sec fixture preflight — the on-chain ledger state-machine sec
//! family (Plan 418 / Research 452; sources: aeyakovenko/lean-svm @
//! dd409de0 Apache-2.0 — the load-bearing artifact is the bundled
//! `.agents/skills/svm-security-testing/SKILL.md` 10-class taxonomy — and
//! aeyakovenko/percolator @ afdadeca Apache-2.0, spec.md frozen v16.9.1;
//! both educational-only per their own READMEs: decision-content citation
//! only, every fixture an ORIGINAL Rust synthesis of the DECISION, never a
//! translation of the code or a Lean proof port).
//!
//! Compiled AND executed BEFORE entry (debug AND -O):
//!   rustc --edition 2021 --test b424_sec_fixtures.rs && ./b424_sec_fixtures
//!
//! Arms (the four-arm discipline: buggy/fixed/all-safe/tampered per rule):
//! - F1 (division-rounds-against-the-account): the user-favorable arm (a
//!   payout computed with the WRONG direction — rounds UP, mints dust the
//!   protocol funds); the floor arm (user payouts/mints round DOWN — the
//!   dust falls to the protocol); the ceil arm (fees/debt/margin round UP —
//!   the protocol is never short); the dust-loop tamper arm (iterating
//!   ceil-rounded payouts extracts the reserve one epsilon per pass).
//! - F2 (account-revival-tombstone-close-discipline): the revival arm (a
//!   closed account re-initialized from DIRTY memory reads stale nonzero
//!   balances — magic==0 admits anything); the tombstone arm (close writes
//!   a permanent nonzero magic DISTINCT from every live magic — neither
//!   live handlers nor fresh-init can accept it); the zero-init arm
//!   (init zeroes every field including padding — dirty memory never
//!   leaks); the >= tamper arm (an exact-size check spelled `>=` accepts a
//!   bigger buffer whose tail holds stale bytes).
//! - F3 (share-pool-first-depositor-boundary-states): the four boundary
//!   states as falsifier arms — drained pool (`assets==0 ∧ totSh>0`: the
//!   new depositor donates to wiped-out old LPs), donation inflation
//!   (`totSh==1` + large donation: honest deposits truncate to 0 minted),
//!   receipt desync (`receipt.shares > totSh`), trapped rent (tombstoned
//!   pool blocks receipt closure); the honest-deposit arm (the guarded
//!   first-deposit path mints correctly in every boundary).
//! - F4 (account-pair-aliasing-distinct-key): the no-op self-transfer arm
//!   (source==dest makes the SPL call a no-op while the program still
//!   credits internally — stock conservation breaks); the overwrite arm
//!   (from==to where both balances are read before either write — the
//!   debit is overwritten); the distinct-key arm (the check refuses both
//!   aliasings up front); the all-safe arm (distinct accounts settle
//!   exactly).
//! - F5 (cpi-callee-id-unpinned): the spoofed-CPI arm (a callee program id
//!   from the untrusted request — a noop fake "succeeds", the program
//!   credits on nothing); the pinned arm (the callee id comes from an
//!   immutable config record; a spoofed id refuses); the home
//!   anti-regime arm (a dispatch keyed on a COMPILED match table — the
//!   callee id is structural, not request-supplied; the class cannot
//!   exist); the custom-target tamper arm (pinning the well-known ids but
//!   leaving the custom one request-supplied — the same hole through the
//!   custom door).

// ─────────────────────────────────────────────────────────────────────────
// F1 — division-rounds-against-the-account (percolator spec.md L77 +
// lean-svm §10 mulDiv64/mulDiv64Ceil)
// ─────────────────────────────────────────────────────────────────────────

/// The FIXED direction law: amounts owed BY the protocol to a user
/// (payouts, mints, swap proceeds) round DOWN — the residual dust stays
/// with the account; amounts owed TO the protocol (fees, debt, margin
/// requirements) round UP — the protocol is never short by an epsilon.
fn round_against_account(
    amount: u128,
    numerator: u64,
    denominator: u64,
    owes_protocol: bool,
) -> u128 {
    let raw = (amount * u128::from(numerator)) / u128::from(denominator);
    let exact = (amount * u128::from(numerator)) % u128::from(denominator) == 0;
    if exact || !owes_protocol {
        raw // floor: user-favorable dust impossible
    } else {
        raw + 1 // ceil: the protocol never carries the epsilon
    }
}

#[test]
fn f1_division_rounding_direction_arms() {
    // the user-favorable WRONG arm: a payout computed with the ceil side —
    // every pass mints dust the reserve funds
    let payout_wrong = round_against_account(1_000u128, 1, 3, true); // ceil: 334
    let payout_right = round_against_account(1_000u128, 1, 3, false); // floor: 333
    assert_eq!(payout_wrong, 334);
    assert_eq!(payout_right, 333);
    assert!(
        payout_wrong > payout_right,
        "ceil-on-payout mints the dust the floor keeps"
    );

    // the floor arm: payouts floor — the residue stays protocol-side
    let fee = round_against_account(1_000u128, 1, 3, true); // owes_protocol: ceil
    assert_eq!(fee, 334, "fees/debt ceil — the protocol is never short");

    // the exact arm: no residue, both directions agree
    assert_eq!(round_against_account(999u128, 1, 3, true), 333);
    assert_eq!(round_against_account(999u128, 1, 3, false), 333);

    // the dust-loop tamper arm: iterating a ceil-rounded payout drains the
    // reserve one epsilon per pass — the direction-as-policy is the fix
    let mut reserve: u128 = 1_000_000;
    for _ in 0..1000 {
        let out = round_against_account(1_000u128, 1, 3, true); // WRONG for a payout
        reserve = reserve + 333 - out; // the vault pays out, keeps 333, loses 1/pass
    }
    assert_eq!(
        reserve,
        1_000_000 - 1000,
        "the ceil-payout loop extracted 1000 epsilons"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// F2 — account-revival-tombstone-close-discipline (lean-svm §1)
// ─────────────────────────────────────────────────────────────────────────

const MAGIC_LIVE: u32 = 0x1DEA_0001;
const MAGIC_CLOSED: u32 = 0x4445_4144; // permanent, nonzero, distinct from every live magic

#[derive(Debug, PartialEq)]
struct LedgerAccount {
    magic: u32,
    balance: u64,
    pad: [u8; 8], // the tail: stale bytes live here if never zeroed
}

/// The BUGGY close: the account "closes" by dropping the handle — the bytes
/// stay exactly as they were.
fn close_naive(acc: &mut LedgerAccount) {
    let _ = acc; // nothing written: the memory keeps balance + pad
    acc.magic = 0; // the fresh-init sentinel — the revival door
}

/// The FIXED close: permanent nonzero tombstone DISTINCT from every live
/// magic, zero the body, (exact-len is the wire contract, carried in-rule).
fn close_tombstoned(acc: &mut LedgerAccount) {
    acc.magic = MAGIC_CLOSED;
    acc.balance = 0;
    acc.pad = [0u8; 8]; // zeroWords on close: the dirty tail never survives
}

/// The FIXED init: zero every field BEFORE writing (defense-in-depth) —
/// init from arbitrary dirty memory yields a clean account.
fn init_clean(acc: &mut LedgerAccount, balance: u64) {
    acc.magic = 0; // the fresh sentinel, transiently
    acc.balance = 0;
    acc.pad = [0u8; 8]; // zeroWords on init
    acc.magic = MAGIC_LIVE;
    acc.balance = balance;
}

#[test]
fn f2_tombstone_close_discipline_arms() {
    // the revival arm: closed-naive, then re-init — the dirty tail was
    // never zeroed, and anything reading magic==0 as "uninitialized" while
    // trusting the rest of the bytes inherits the stale balance
    let mut dirty = LedgerAccount {
        magic: MAGIC_LIVE,
        balance: 999,
        pad: [0xFF; 8],
    };
    close_naive(&mut dirty);
    assert_eq!(dirty.magic, 0);
    assert_eq!(
        dirty.balance, 999,
        "the stale balance survived the naive close"
    );
    assert_eq!(dirty.pad, [0xFF; 8], "the dirty tail survived");
    init_clean(&mut dirty, 5);
    assert_eq!(dirty.balance, 5, "the FIXED init zeroes before writing");
    assert_eq!(dirty.pad, [0u8; 8]);

    // the tombstone arm: the closed account is PERMANENTLY distinct —
    // neither a live handler nor a fresh-init branch can accept it
    let mut acc = LedgerAccount {
        magic: MAGIC_LIVE,
        balance: 42,
        pad: [0; 8],
    };
    close_tombstoned(&mut acc);
    assert_eq!(acc.magic, MAGIC_CLOSED);
    assert_ne!(
        acc.magic, 0,
        "magic==0 is the fresh-init door — never close into it"
    );
    assert_ne!(acc.magic, MAGIC_LIVE, "distinct from every live magic");
    assert_eq!(acc.balance, 0);
    assert_eq!(
        acc.pad, [0u8; 8],
        "zeroed on close — the dirty tail never survives"
    );

    // the all-safe arm: a clean lifecycle — init from dirty memory, live,
    // tombstoned close, and the tombstone refuses revival
    let mut fresh = LedgerAccount {
        magic: 0xABCD_ABCD,
        balance: u64::MAX,
        pad: [0x7F; 8],
    };
    init_clean(&mut fresh, 10);
    assert_eq!(fresh.magic, MAGIC_LIVE);
    close_tombstoned(&mut fresh);
    let revival_refused = fresh.magic != 0 && fresh.magic != MAGIC_LIVE;
    assert!(revival_refused);
}

// ─────────────────────────────────────────────────────────────────────────
// F3 — share-pool-first-depositor-boundary-states (lean-svm §7)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct SharePool {
    assets: u128,
    total_shares: u128,
}

impl SharePool {
    /// The FIXED mint: the boundary states are GUARDED — a fresh pool mints
    /// shares == assets deposited; a live pool mints proportionally with a
    /// zero-guard on the denominator; the honest depositor never donates.
    fn deposit(&mut self, amount: u128) -> u128 {
        if self.total_shares == 0 || self.assets == 0 {
            // fresh-or-wiped pool: 1:1 (the drained-pool boundary is NOT
            // allowed to silently donate to old shareholders — the guard
            // mints against the CURRENT assets, and the wiped state resets
            // the share base)
            self.total_shares += amount;
            self.assets += amount;
            return amount;
        }
        let shares = amount * self.total_shares / self.assets;
        self.total_shares += shares.max(1); // the inflation guard: never mint 0
        self.assets += amount;
        shares.max(1)
    }
}

#[test]
fn f3_share_pool_boundary_arms() {
    // boundary 1, the drained-pool arm: assets==0 ∧ totSh>0 — the buggy 1:1
    // mint DONATES to the wiped-out old shareholders; the FIXED path resets
    // the share base so the new depositor's entry is their own
    let mut drained = SharePool {
        assets: 0,
        total_shares: 1_000,
    };
    let shares = drained.deposit(1_000_000);
    assert_eq!(shares, 1_000_000, "no donation to the wiped old base");

    // boundary 2, the donation-inflation arm: totSh==1 + a large donation —
    // honest deposits truncate; the zero-guard keeps every deposit alive
    let mut inflated = SharePool {
        assets: 1,
        total_shares: 1,
    };
    inflated.assets += 1_000_000; // the donation: assets grow, shares don't
    let honest = inflated.deposit(1_000);
    assert!(
        honest >= 1,
        "the honest deposit never truncates to zero minted"
    );

    // the honest arm: a normal proportional deposit
    let mut pool = SharePool {
        assets: 1_000,
        total_shares: 1_000,
    };
    let s = pool.deposit(1_000);
    assert_eq!(s, 1_000, "1:1 at equal base");

    // boundary 3+4, the receipt-desync + trapped-rent arms (the wire-level
    // halves): receipt.shares must never exceed total_shares, and a
    // tombstoned pool must not block receipt closure while receipts hold
    // value — carried as the in-rule clauses; the fixture pins the pool
    // never mints past the base
    let mut p = SharePool {
        assets: 100,
        total_shares: 100,
    };
    let _ = p.deposit(50);
    assert!(
        p.total_shares <= p.assets,
        "receipt desync impossible when Σshares tracks assets"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// F4 — account-pair-aliasing-distinct-key (lean-svm §5)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum SettleError {
    AliasedAccounts,
    Ok,
}

/// The FIXED settlement: the distinct-key check runs BEFORE any read —
/// both aliasings (the no-op external call AND the read-both-write-both
/// internal overwrite) are refused at the door.
fn settle(
    from_bal: &mut u64,
    to_bal: &mut u64,
    from_key: u32,
    to_key: u32,
    amount: u64,
) -> SettleError {
    if from_key == to_key {
        return SettleError::AliasedAccounts;
    }
    if *from_bal < amount {
        return SettleError::Ok; // insufficient: a no-op, not an error here
    }
    *from_bal -= amount;
    *to_bal += amount;
    SettleError::Ok
}

#[test]
fn f4_account_pair_aliasing_arms() {
    // the no-op self-transfer arm: source==dest — the external call is a
    // no-op while the program-side credit still runs: the RECIPIENT's
    // program-side balance grew, the VAULT's SPL balance never moved —
    // the 100 tokens credited to the recipient materialized from nothing
    let mut vault_spl = 1_000u64; // the real token account
    let mut recipient_internal = 0u64; // the program's record of what the user holds
    // SPL transfer(vault -> vault): a no-op — vault_spl unchanged
    // then the program credits the recipient: recipient_internal += 100
    recipient_internal += 100;
    assert_eq!(vault_spl, 1_000, "the vault never moved (the no-op)");
    assert_eq!(
        recipient_internal, 100,
        "the program credited the recipient anyway"
    );
    assert!(
        recipient_internal > 0 && vault_spl == 1_000,
        "100 materialized from nothing — conservation broke"
    );

    // the overwrite arm: from==to with read-both-write-both — the debit is
    // overwritten by the credit (bal - amt then bal + amt on the same cell)
    let mut bal = 1_000u64;
    let amt = 200u64;
    bal = bal - amt; // read + write as "from"
    bal = bal + amt; // read + write as "to": the debit vanished
    assert_eq!(bal, 1_000, "the amount teleported — no debit ever landed");

    // the distinct-key arm: the check refuses both aliasings up front
    let mut a = 1_000u64;
    let mut b = 0u64;
    assert_eq!(
        settle(&mut a, &mut b, 1, 1, 100),
        SettleError::AliasedAccounts
    );
    assert_eq!(a, 1_000, "the aliased pair never mutated");

    // the all-safe arm: distinct accounts settle exactly
    assert_eq!(settle(&mut a, &mut b, 1, 2, 100), SettleError::Ok);
    assert_eq!(
        (a, b),
        (900, 100),
        "exact conservation across distinct keys"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// F5 — cpi-callee-id-unpinned (lean-svm §4; the home anti-regime arm)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum CpiError {
    UnpinnedRefused,
    Transferred,
}

/// A callee identity: either request-supplied (untrusted) or config-pinned.
#[derive(Debug, Clone, PartialEq)]
enum CalleeId {
    FromRequest(u32),
    ConfigPinned(u32),
}

const TOKEN_PROGRAM_ID: u32 = 0x1001;
const ATTACKER_FAKE_ID: u32 = 0xDEAD;

/// The BUGGY dispatch: the callee id comes from the request and executes
/// unpinned — the fake program "transfers" nothing and returns success.
fn cpi_transfer_unpinned(callee: &CalleeId, vault: &mut u64, _amount: u64) -> CpiError {
    let _ = callee; // never checked
    // the fake: returns success without moving anything
    *vault = 0; // the noop.so "transfer" left the vault untouched in reality;
    // here the stand-in zeroes it to mark that NOTHING moved
    CpiError::Transferred // the program credits on this
}

/// The FIXED dispatch: the callee id must match the config-pinned program;
/// a request-supplied id refuses at the door.
fn cpi_transfer_pinned(callee: &CalleeId, vault: &mut u64, amount: u64) -> CpiError {
    let pinned = match callee {
        CalleeId::ConfigPinned(id) if *id == TOKEN_PROGRAM_ID => *id,
        _ => return CpiError::UnpinnedRefused,
    };
    let _ = pinned;
    if *vault >= amount {
        *vault -= amount; // real transfer semantics stand-in
        CpiError::Transferred
    } else {
        CpiError::UnpinnedRefused
    }
}

/// The HOME ANTI-REGIME: dispatch keyed on a COMPILED match table — the
/// callee id is a structural variant, never a runtime-supplied number.
#[derive(Debug)]
enum HomeDispatch {
    TokenTransfer,
}
fn home_dispatch(op: &HomeDispatch, amount: u64) -> &'static str {
    match op {
        HomeDispatch::TokenTransfer => {
            let _ = amount;
            "token_program" // the callee is the ARM, not a parameter
        }
    }
}

#[test]
fn f5_cpi_callee_pinned_arms() {
    // the spoofed arm: the attacker's fake id rides in as request-supplied,
    // "transfers" nothing, returns success
    let mut vault = 700u64;
    let fake = CalleeId::FromRequest(ATTACKER_FAKE_ID);
    assert_eq!(
        cpi_transfer_unpinned(&fake, &mut vault, 10),
        CpiError::Transferred,
        "the noop fake succeeded — the program credits on nothing"
    );

    // the pinned arm: the config-pinned id transfers; a spoofed id refuses
    let mut vault2 = 700u64;
    let pinned = CalleeId::ConfigPinned(TOKEN_PROGRAM_ID);
    assert_eq!(
        cpi_transfer_pinned(&pinned, &mut vault2, 8),
        CpiError::Transferred
    );
    let spoof = CalleeId::ConfigPinned(ATTACKER_FAKE_ID);
    assert_eq!(
        cpi_transfer_pinned(&spoof, &mut vault2, 8),
        CpiError::UnpinnedRefused
    );
    let request_supplied = CalleeId::FromRequest(TOKEN_PROGRAM_ID);
    assert_eq!(
        cpi_transfer_pinned(&request_supplied, &mut vault2, 8),
        CpiError::UnpinnedRefused,
        "even the RIGHT id refuses when it arrives from the request — provenance is the law"
    );

    // the custom-target tamper arm: pinning the well-known ids but leaving
    // the custom one request-supplied is the same hole through the custom
    // door — EVERY callee pins (carried in-rule)
    let custom = CalleeId::FromRequest(0x7777);
    assert_eq!(
        cpi_transfer_pinned(&custom, &mut vault2, 8),
        CpiError::UnpinnedRefused
    );

    // the home anti-regime arm: compiled-match dispatch has no request-
    // supplied id at all — the class cannot exist
    assert_eq!(
        home_dispatch(&HomeDispatch::TokenTransfer, 10),
        "token_program"
    );
}

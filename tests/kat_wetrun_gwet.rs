//! **G-WET** — Proposal 011's wet-run gate (Issue 061 T4.1 + T4.2, Plan 066).
//!
//! The proposal's G-WET bullets and where each is pinned here:
//!
//! | Bullet | Test |
//! |---|---|
//! | full wet-run passes | [`the_full_wet_run_passes_on_devnet`] |
//! | **mint/burn counters bit-unchanged** | [`assert_frozen`], called by every money test |
//! | faucet refund lands exactly X once | [`the_refund_lands_exactly_once_and_is_faucet_funded`] |
//! | mainnet refusal fires | [`a_full_run_on_mainnet_is_refused_and_touches_nothing`] |
//! | dry-run touches nothing | [`a_dry_run_issues_only_reads_on_every_network`] |
//!
//! plus the thing most likely to be papered over:
//!
//! | riir-dapps **Issue 010** skip fails LOUDLY | [`an_issue_010_treasury_skip_fails_loudly_and_never_retries`] |
//!
//! # Two disciplines this file is built on
//!
//! **1. The claim is over the WHOLE counter struct.** `assert_frozen`
//! compares `SupplyCounters::to_bytes()`, a total function of every field, so
//! "bit-unchanged" is literal and a counter added later is covered without
//! anyone remembering to extend this file. Asserting a hand-picked subset is
//! the failure mode it replaces (the riir-dapps G-CRANK `Meta` lesson).
//!
//! **2. Every gate is canaried.** A gate that cannot fail is not a gate:
//! [`the_supply_gate_can_actually_fail`] runs the identical happy path with
//! one mint micro moved and requires the verdict to flip. Without it, a
//! `Verdict::Ready` here would prove only that the harness runs.
//!
//! # The Issue-010 override, said out loud
//!
//! The happy-path tests script a service whose `treasury_grant_floor` has
//! been **explicitly lowered**. Against a GENESIS-DEFAULT riir-dapps
//! deployment they could not pass at all:
//! `TREASURY_GRANT_FLOOR_DEFAULT == MAX_SUPPLY_PLACEHOLDER` (both 10¹²), so
//! the treasury-ONLY crank skips every account every day (riir-dapps Issue
//! 010, pinned there by `tests/kat_grant_reachability.rs`). The override is
//! what makes the faucet legs reachable, and
//! [`an_issue_010_treasury_skip_fails_loudly_and_never_retries`] is the test
//! for what a default deployment actually does.
//!
//! Requires `--features kat_transport`.
#![cfg(feature = "kat_transport")]

use std::cell::RefCell;

use riir_clippy::kat_lease_client::HttpResponse;
use riir_clippy::kat_wetrun::{
    self, LocalLegs, Network, RunMode, SupplyCounters, Verdict, WetrunReport, WetrunTransport,
};

const ACCT: &str = "ab12cd34ef56ab12cd34ef56ab12cd34";
/// The deployment's `faucet_daily_cap_kat`, micro-units. Integer money only
/// (G4-S) — a float anywhere on this path is review-blocking.
const CAP_MICRO: i64 = 10_000_000;
const EPOCH: u64 = 3;

// ── the scripted service ────────────────────────────────────────────────

/// What one crank pass does, per leg. Scripted so the Issue-010 branch is a
/// first-class case rather than a fault injection.
#[derive(Clone, Copy)]
enum CrankScript {
    /// The floor was explicitly lowered: the pass settles the faucet row
    /// from the TREASURY.
    SettlesFromTreasury,
    /// **Genesis defaults**: `skipped_treasury`, nothing settled, ever.
    SkipsOnTreasuryFloor,
    /// The bearer was not accepted.
    Unauthorized,
}

struct MockService {
    /// Every call, in order, with its path. This is what makes "dry-run
    /// touches nothing" a measurement instead of a claim.
    log: RefCell<Vec<String>>,
    crank: CrankScript,
    /// Faucet rows the scripted ledger has settled — the refund
    /// exactly-once counter.
    settled: RefCell<u64>,
    /// The supply counters the scripted ledger serves. A test may move one
    /// to canary the gate.
    supply: RefCell<SupplyCounters>,
    /// Treasury and user balances, so "the treasury paid every micro" is
    /// checkable rather than asserted.
    treasury_micro: RefCell<i64>,
    user_micro: RefCell<i64>,
    /// `pending_durable` on `/health` — nonzero withholds the welcome.
    pending_durable: u64,
    /// Rows the sync route claims to have recorded (`None` = answer 200
    /// with no count at all).
    sync_recorded: Option<u64>,
}

impl MockService {
    fn new(crank: CrankScript) -> Self {
        Self {
            log: RefCell::new(Vec::new()),
            crank,
            settled: RefCell::new(0),
            supply: RefCell::new(SupplyCounters {
                epoch: EPOCH,
                mint_micro: 0,
                burn_micro: 0,
                pool_paid_micro: 0,
                pool_available_micro: 0,
            }),
            treasury_micro: RefCell::new(1_000_000_000),
            user_micro: RefCell::new(0),
            pending_durable: 0,
            sync_recorded: Some(2),
        }
    }

    fn note(&self, what: &str) {
        self.log.borrow_mut().push(what.to_string());
    }

    fn calls(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    /// Calls that mutate service state. The split is on the METHOD, not on
    /// a hand-kept list, so a write added later shows up here automatically.
    fn writes(&self) -> Vec<String> {
        self.log
            .borrow()
            .iter()
            .filter(|c| c.starts_with("POST "))
            .cloned()
            .collect()
    }

    fn ok(body: &serde_json::Value) -> HttpResponse {
        HttpResponse::Ok(body.to_string().into_bytes())
    }
}

impl WetrunTransport for MockService {
    fn get_ledger_health(&self) -> HttpResponse {
        self.note("GET /health");
        Self::ok(&serde_json::json!({
            "plane": "ledger", "tick": 42, "epoch": EPOCH,
            "pending_durable": self.pending_durable
        }))
    }

    fn get_mining_health(&self) -> HttpResponse {
        self.note("GET /mining/health");
        Self::ok(&serde_json::json!({"plane": "mining", "unproven": 7}))
    }

    fn get_balance(&self, acct_hex: &str) -> HttpResponse {
        self.note("GET /balance");
        assert_eq!(acct_hex, ACCT, "the probe must query THIS account");
        Self::ok(&serde_json::json!({
            "account_id": acct_hex,
            "balance_micro": *self.user_micro.borrow(),
            "granted": true
        }))
    }

    fn get_telemetry(&self, epoch: u64) -> HttpResponse {
        self.note("GET /telemetry");
        assert_eq!(epoch, EPOCH, "the snapshot must be pinned to one epoch");
        let s = *self.supply.borrow();
        Self::ok(&serde_json::json!({
            "epoch": s.epoch, "burn_micro": s.burn_micro, "mint_micro": s.mint_micro,
            "pool_paid_micro": s.pool_paid_micro, "pool_available_micro": s.pool_available_micro
        }))
    }

    fn open_faucet_request(&self) -> HttpResponse {
        self.note("POST /payreq/faucet");
        Self::ok(&serde_json::json!({
            "request_id": "71b5cbf13626d2554aba7d9b06e63474",
            "created": true, "kind": "faucet",
            // The AMOUNT is the deployment's, echoed back. Nothing on the
            // wire proposed it.
            "amount_micro": CAP_MICRO,
            "expiry": 86_400
        }))
    }

    fn post_crank(&self) -> HttpResponse {
        self.note("POST /admin/crank/daily");
        match self.crank {
            CrankScript::Unauthorized => HttpResponse::Status(401),
            CrankScript::SkipsOnTreasuryFloor => Self::ok(&serde_json::json!({
                "date": "2026-09-02", "credited": 0, "faucet_settled": 0, "expired": 0,
                "granted_micro": 0, "skipped_already": 0, "skipped_dormant": 0,
                "skipped_budget": 0,
                // THE Issue-010 signal.
                "skipped_treasury": 1,
                "skipped_faucet_cap": 0, "needs_alert": true
            })),
            CrankScript::SettlesFromTreasury => {
                // Treasury debit == user credit, to the micro. A grant, not
                // a mint: no supply counter is touched here, which is the
                // whole point of caveat #2.
                *self.settled.borrow_mut() += 1;
                *self.treasury_micro.borrow_mut() -= CAP_MICRO;
                *self.user_micro.borrow_mut() += CAP_MICRO;
                Self::ok(&serde_json::json!({
                    "date": "2026-09-02", "credited": 1, "faucet_settled": 1, "expired": 0,
                    "granted_micro": CAP_MICRO, "skipped_already": 0, "skipped_dormant": 0,
                    "skipped_budget": 0, "skipped_treasury": 0, "skipped_faucet_cap": 0,
                    "needs_alert": false
                }))
            }
        }
    }

    fn post_sync(&self, ndjson: &str) -> HttpResponse {
        self.note("POST /sync");
        assert!(!ndjson.is_empty(), "an empty payload is not a push");
        // The REAL service's ingest answer (kat-service `mining_route`, its
        // contract test is the authority): `accepted:true` + `recorded` (the
        // legacy synonym) + `rows_new`. The tightened sync-leg gate refuses a
        // 200 without `accepted:true`, so the healthy arm must carry the
        // full real shape — `recorded` alone is the stale-contract shape the
        // gate exists to catch.
        match self.sync_recorded {
            Some(n) => Self::ok(&serde_json::json!({
                "accepted": true, "recorded": n, "rows_new": n,
                "rows_refreshed": 0, "rows_terminal_skipped": 0
            })),
            None => Self::ok(&serde_json::json!({"accepted": true})),
        }
    }
}

// ── the scripted local legs ─────────────────────────────────────────────

/// Step 3 + step 4, scripted. The CLI wires the REAL proposers + meter +
/// payload builder (`CliLocalLegs` in `heal.rs`); this file's job is the
/// SERVICE-side gate, so the local legs are held fixed and counted.
struct ScriptedLegs {
    cycle: Result<(u32, i64), String>,
    payload: Result<(String, usize, u32), String>,
    cycles_run: u32,
}

impl ScriptedLegs {
    fn healthy() -> Self {
        Self {
            cycle: Ok((3, 1_000)),
            payload: Ok(("{\"row\":1}\n{\"row\":2}\n".to_string(), 2, 2)),
            cycles_run: 0,
        }
    }
}

impl LocalLegs for ScriptedLegs {
    fn scored_cycle(&mut self) -> Result<(u32, i64), String> {
        self.cycles_run += 1;
        self.cycle.clone()
    }

    fn sync_payload(&mut self) -> Result<(String, usize, u32), String> {
        self.payload.clone()
    }
}

// ── the money-printer assertion, applied by every money test ────────────

/// **The G-WET money-printer guard.** Whole-struct, byte-level.
///
/// Compares `to_bytes()` rather than any field subset: the encoding is a
/// total function of `SupplyCounters`, so this is literally the
/// "bit-unchanged" claim, and a counter added to the type later is covered
/// here without an edit. `SupplyCounters`'s own unit test crosses
/// `size_of` against the encoded width, so a field that skips the encoding
/// reds there rather than passing silently here.
fn assert_frozen(report: &WetrunReport, what: &str) {
    let (Some(before), Some(after)) = (report.supply_before, report.supply_after) else {
        panic!(
            "{what}: a full run MUST carry both supply snapshots — without them the \
             money-printer assertion cannot be made, and an unmade assertion is not a pass \
             (got before={:?} after={:?})",
            report.supply_before, report.supply_after
        );
    };
    assert_eq!(
        before.to_bytes(),
        after.to_bytes(),
        "{what}: the WHOLE supply counter struct must be BIT-unchanged — the wet-run refund \
         is FAUCET-funded (a treasury grant), and a moved mint/burn/pool counter means the \
         loop found an emission channel (Proposal 011 caveat #2)"
    );
    // And the harness's own verdict must agree with this file's — two
    // independent statements of the same rule.
    assert!(
        kat_wetrun::assert_supply_unchanged(before, after).is_ok(),
        "{what}: the harness and this gate disagree about the supply"
    );
}

fn run(mock: &MockService, legs: &mut ScriptedLegs, net: Network, mode: RunMode) -> WetrunReport {
    kat_wetrun::run(mock, legs, net, mode, ACCT)
}

// ── G-WET: the full run ─────────────────────────────────────────────────

/// The headline: all six steps, on devnet, against a deployment whose
/// `treasury_grant_floor` was **explicitly lowered** (see the module docs —
/// against genesis defaults this cannot pass, by riir-dapps Issue 010).
#[test]
fn the_full_wet_run_passes_on_devnet() {
    let mock = MockService::new(CrankScript::SettlesFromTreasury);
    let mut legs = ScriptedLegs::healthy();
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);

    assert_eq!(report.verdict, Verdict::Ready, "{:#?}", report.steps);
    assert_eq!(report.steps.len(), 6, "all six §7 steps ran");
    for s in &report.steps {
        assert!(s.ok, "step {} failed: {:?}", s.step.label(), s.lines);
    }
    assert_frozen(&report, "G-WET full run");
    assert_eq!(legs.cycles_run, 1, "exactly one scored cycle");
    assert!(report.verdict_line().contains("READY"));
    assert!(report.verdict_line().contains("bit-unchanged"));

    // The exact call sequence — pinned so a step silently dropping its
    // service call cannot pass. Reads bracket the money legs on both sides.
    assert_eq!(
        mock.calls(),
        vec![
            "GET /health",
            "GET /mining/health",
            "GET /balance",
            "GET /telemetry",
            "POST /payreq/faucet",
            "POST /admin/crank/daily",
            "POST /sync",
            "POST /payreq/faucet",
            "POST /admin/crank/daily",
            "GET /health",
            "GET /mining/health",
            "GET /balance",
            "GET /telemetry",
        ]
    );
}

/// Step 5's bullet: the refund lands exactly X, exactly ONCE, and the
/// TREASURY pays every micro of it.
#[test]
fn the_refund_lands_exactly_once_and_is_faucet_funded() {
    let mock = MockService::new(CrankScript::SettlesFromTreasury);
    let mut legs = ScriptedLegs::healthy();
    let treasury_0 = *mock.treasury_micro.borrow();
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);
    assert_eq!(report.verdict, Verdict::Ready);

    // Two faucet legs (step 2 top-up + step 5 refund), each settling once.
    assert_eq!(
        *mock.settled.borrow(),
        2,
        "one settle per leg, never a retry-doubled credit"
    );
    assert_eq!(
        mock.writes()
            .iter()
            .filter(|c| c.contains("payreq/faucet"))
            .count(),
        2,
        "exactly two faucet requests were opened"
    );
    // Exactly X per leg, and Σ conserved: the treasury paid what the user
    // received, to the micro. Integer money throughout.
    assert_eq!(*mock.user_micro.borrow(), 2 * CAP_MICRO);
    assert_eq!(*mock.treasury_micro.borrow(), treasury_0 - 2 * CAP_MICRO);
    assert_eq!(
        *mock.treasury_micro.borrow() + *mock.user_micro.borrow(),
        treasury_0,
        "a grant moves money; it does not create any"
    );
    assert_frozen(&report, "the refund leg");

    // The refund step must LABEL itself as faucet-funded — the copy is part
    // of caveat #2, not decoration.
    let refund = report
        .steps
        .iter()
        .find(|s| s.step == riir_clippy::kat_wetrun::Step::RefundCredit)
        .expect("step 5 ran");
    let joined = refund.lines.join("\n");
    assert!(joined.contains("FAUCET-funded"), "{joined}");
    assert!(joined.contains("never mint"), "{joined}");
    assert!(joined.contains("grant, not mint"), "{joined}");
    assert!(!joined.contains('$'), "caveat #15 copy rule: {joined}");
}

/// **The canary.** The identical happy path with ONE mint micro moved must
/// flip the verdict. Without this, `Verdict::Ready` above would prove only
/// that the harness runs to completion.
#[test]
fn the_supply_gate_can_actually_fail() {
    let mock = MockService::new(CrankScript::SettlesFromTreasury);
    let mut legs = ScriptedLegs::healthy();

    // Same script, except the ledger mints one micro somewhere in the run.
    // (Scripted at the last telemetry read, which is where a real emission
    // would surface.)
    struct Minting<'m>(&'m MockService);
    impl WetrunTransport for Minting<'_> {
        fn get_ledger_health(&self) -> HttpResponse {
            self.0.get_ledger_health()
        }
        fn get_mining_health(&self) -> HttpResponse {
            self.0.get_mining_health()
        }
        fn get_balance(&self, a: &str) -> HttpResponse {
            self.0.get_balance(a)
        }
        fn get_telemetry(&self, e: u64) -> HttpResponse {
            let first = !self.0.log.borrow().iter().any(|c| c == "GET /telemetry");
            let r = self.0.get_telemetry(e);
            if first {
                self.0.supply.borrow_mut().mint_micro += 1;
            }
            r
        }
        fn open_faucet_request(&self) -> HttpResponse {
            self.0.open_faucet_request()
        }
        fn post_crank(&self) -> HttpResponse {
            self.0.post_crank()
        }
        fn post_sync(&self, n: &str) -> HttpResponse {
            self.0.post_sync(n)
        }
    }

    let report = kat_wetrun::run(
        &Minting(&mock),
        &mut legs,
        Network::Devnet,
        RunMode::Full,
        ACCT,
    );
    match &report.verdict {
        Verdict::NotReady(why) => {
            assert!(why.contains("G-WET FAILED"), "{why}");
            assert!(why.contains("mint_micro 0 → 1"), "{why}");
            assert!(why.contains("money printer with a test wrapper"), "{why}");
        }
        other => panic!("one minted micro must fail G-WET, got {other:?}"),
    }
    // Every step still PASSED — which is exactly why the supply gate has to
    // be separate from the step verdicts.
    for s in &report.steps {
        assert!(s.ok, "step {} failed for another reason", s.step.label());
    }
}

// ── the riir-dapps Issue 010 path (the thing most likely papered over) ──

/// **Genesis defaults.** The crank skips on the treasury floor, so no
/// faucet row settles. The run must FAIL, name riir-dapps Issue 010, say it
/// is not retryable, and issue **exactly one** crank POST — no retry loop,
/// no wait-it-out, no pass.
#[test]
fn an_issue_010_treasury_skip_fails_loudly_and_never_retries() {
    let mock = MockService::new(CrankScript::SkipsOnTreasuryFloor);
    let mut legs = ScriptedLegs::healthy();
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);

    // NOT ready, and the failing step is the faucet top-up (step 2) — the
    // run stops there rather than carrying on to "see if it works out".
    match &report.verdict {
        Verdict::NotReady(why) => assert!(why.contains("2 faucet top-up"), "{why}"),
        other => panic!("a treasury skip must not be READY, got {other:?}"),
    }
    let step2 = report
        .steps
        .iter()
        .find(|s| s.step == riir_clippy::kat_wetrun::Step::FaucetTopUp)
        .expect("step 2 ran");
    assert!(!step2.ok);
    let d = step2.lines.join("\n");
    assert!(d.contains("riir-dapps Issue 010"), "{d}");
    assert!(d.contains("TREASURY_GRANT_FLOOR_DEFAULT"), "{d}");
    assert!(d.contains("MAX_SUPPLY_PLACEHOLDER"), "{d}");
    assert!(d.contains("skipped_treasury"), "{d}");
    // The T4.4a two-cause split vocabulary (f5cce1e): the legacy phrases
    // (STRUCTURALLY UNREACHABLE / NOT retryable / OWNER act) were replaced —
    // the diagnostic now names BOTH causes (config defect vs unfunded
    // treasury) and the ops fund route that fixes the second.
    assert!(d.contains("not a transient"), "{d}");
    assert!(d.contains("UNFUNDED TREASURY"), "{d}");
    assert!(d.contains("/admin/treasury/fund"), "{d}");
    assert!(d.contains("treasury-ONLY"), "{d}");

    // ONE crank attempt, ONE faucet request. A retry loop here would burn
    // the per-account daily cap and still never settle.
    assert_eq!(
        mock.writes(),
        vec!["POST /payreq/faucet", "POST /admin/crank/daily"],
        "no retry, and the run stopped before the sync + refund legs"
    );
    assert_eq!(
        *mock.settled.borrow(),
        0,
        "nothing settled, and nothing pretended to"
    );
    // The local legs never ran: a failed money leg must not be followed by
    // a metered spend.
    assert_eq!(legs.cycles_run, 0);
    // Supply untouched — the failure mode is "no credit", never "a mint".
    assert_eq!(mock.supply.borrow().mint_micro, 0);
    assert_eq!(mock.supply.borrow().burn_micro, 0);
}

/// The other honest crank refusal: the bearer. Named as a missing input,
/// not as a service fault, and equally terminal.
#[test]
fn a_crank_the_operator_cannot_authorize_fails_with_the_row_left_open() {
    let mock = MockService::new(CrankScript::Unauthorized);
    let mut legs = ScriptedLegs::healthy();
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);
    assert!(matches!(report.verdict, Verdict::NotReady(_)));
    let d = report
        .steps
        .iter()
        .find(|s| s.step == riir_clippy::kat_wetrun::Step::FaucetTopUp)
        .expect("step 2 ran")
        .lines
        .join("\n");
    assert!(d.contains("unauthorized"), "{d}");
    assert!(d.contains("OPEN and unsettled"), "{d}");
    assert!(d.contains("not retryable"), "{d}");
    // A 401 must NOT be mistaken for the Issue-010 skip: different cause,
    // different fix.
    assert!(!d.contains("Issue 010"), "{d}");
    assert_eq!(
        mock.writes().len(),
        2,
        "one open + one crank attempt, then stop"
    );
}

// ── T4.2: mainnet refusal + the dry-run proof ───────────────────────────

/// A full run on mainnet is refused BEFORE anything is touched. Not
/// "refused after the preflight" — the call log must be EMPTY.
#[test]
fn a_full_run_on_mainnet_is_refused_and_touches_nothing() {
    for net in [Network::Mainnet, Network::Unclassifiable] {
        let mock = MockService::new(CrankScript::SettlesFromTreasury);
        let mut legs = ScriptedLegs::healthy();
        let report = run(&mock, &mut legs, net, RunMode::Full);
        match &report.verdict {
            Verdict::Refused(why) => {
                assert!(why.contains("REFUSED"), "{why}");
                assert!(
                    why.contains("--dry-run"),
                    "the refusal names the way out: {why}"
                );
            }
            other => panic!("{net:?} must be refused for a full run, got {other:?}"),
        }
        assert!(report.steps.is_empty(), "no step may run");
        assert!(
            mock.calls().is_empty(),
            "a refused run must touch NOTHING — not even a read ({:?})",
            mock.calls()
        );
        assert_eq!(legs.cycles_run, 0);
        assert!(report.supply_before.is_none() && report.supply_after.is_none());
    }
    // And the mainnet sentence states that no override exists, so nobody
    // goes looking for one (caveat #2).
    let mock = MockService::new(CrankScript::SettlesFromTreasury);
    let mut legs = ScriptedLegs::healthy();
    let report = run(&mock, &mut legs, Network::Mainnet, RunMode::Full);
    assert!(
        report.verdict_line().contains("NO override"),
        "{}",
        report.verdict_line()
    );
}

/// **Dry-run touches nothing — measured, not asserted.** Steps 1+6 only,
/// zero POSTs, and no telemetry read either (a dry-run moves nothing, so
/// there is nothing to snapshot). Admitted on EVERY network, mainnet
/// included — that is what mainnet gets.
#[test]
fn a_dry_run_issues_only_reads_on_every_network() {
    for net in [
        Network::Devnet,
        Network::Testnet,
        Network::Mainnet,
        Network::Unclassifiable,
    ] {
        let mock = MockService::new(CrankScript::SkipsOnTreasuryFloor);
        let mut legs = ScriptedLegs::healthy();
        let report = run(&mock, &mut legs, net, RunMode::DryRun);

        assert_eq!(
            report.verdict,
            Verdict::Ready,
            "{net:?}: {:#?}",
            report.steps
        );
        assert_eq!(report.steps.len(), 2, "{net:?}: §7 steps 1+6 only");
        for s in &report.steps {
            assert!(
                s.step.in_dry_run(),
                "{net:?}: {} is a money leg",
                s.step.label()
            );
            assert!(s.ok);
        }
        // THE proof: every recorded call is a GET, and none of them is a
        // state-bearing route.
        assert!(
            mock.writes().is_empty(),
            "{net:?}: a dry-run issued writes: {:?}",
            mock.writes()
        );
        assert_eq!(
            mock.calls(),
            vec![
                "GET /health",
                "GET /mining/health",
                "GET /balance",
                "GET /health",
                "GET /mining/health",
                "GET /balance",
            ],
            "{net:?}: the dry-run read set is pinned — a route added to it is a review event"
        );
        // Zero local work: no metered spend, no payload composed.
        assert_eq!(legs.cycles_run, 0, "{net:?}");
        // And no supply snapshot, because nothing could have moved.
        assert!(report.supply_before.is_none(), "{net:?}");
        assert!(report.supply_after.is_none(), "{net:?}");
        // The scripted ledger is bit-identical to its start state.
        assert_eq!(*mock.settled.borrow(), 0, "{net:?}");
        assert_eq!(*mock.user_micro.borrow(), 0, "{net:?}");
        assert_eq!(
            mock.supply.borrow().to_bytes(),
            SupplyCounters {
                epoch: EPOCH,
                mint_micro: 0,
                burn_micro: 0,
                pool_paid_micro: 0,
                pool_available_micro: 0
            }
            .to_bytes(),
            "{net:?}"
        );
    }
}

// ── the remaining honest-failure branches ───────────────────────────────

/// An unsettled ledger withholds the run at step 1 — before any money leg,
/// and without a supply snapshot (there is nothing to compare yet).
#[test]
fn an_unsettled_preflight_stops_the_run_before_any_money_moves() {
    let mut mock = MockService::new(CrankScript::SettlesFromTreasury);
    mock.pending_durable = 4;
    let mut legs = ScriptedLegs::healthy();
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);
    assert!(matches!(report.verdict, Verdict::NotReady(_)));
    assert_eq!(report.steps.len(), 1, "the run stops at the preflight");
    assert!(mock.writes().is_empty(), "{:?}", mock.writes());
    assert_eq!(legs.cycles_run, 0);
}

/// Step 4's floor is enforced by the HARNESS. A composer that both builds
/// the payload and vouches for its redaction level is one structure
/// agreeing with itself.
#[test]
fn a_payload_below_the_redact_floor_is_refused_rather_than_pushed() {
    let mock = MockService::new(CrankScript::SettlesFromTreasury);
    let mut legs = ScriptedLegs::healthy();
    legs.payload = Ok((
        "{\"row\":1}\n".to_string(),
        1,
        kat_wetrun::MIN_REDACT_VERSION - 1,
    ));
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);
    assert!(matches!(report.verdict, Verdict::NotReady(_)));
    let d = report
        .steps
        .iter()
        .find(|s| s.step == riir_clippy::kat_wetrun::Step::SyncPush)
        .expect("step 4 ran")
        .lines
        .join("\n");
    assert!(d.contains("below the floor"), "{d}");
    assert!(
        !mock.writes().contains(&"POST /sync".to_string()),
        "a below-floor payload must never reach the wire: {:?}",
        mock.writes()
    );
}

/// A 200 that cannot confirm WHAT was recorded (no `accepted:true`, no
/// per-row counts, or a partial record) is NOT a successful push.
#[test]
fn a_sync_that_cannot_confirm_the_record_is_not_a_pass() {
    for (recorded, needle) in [
        (None, "carried no per-row counts"),
        (Some(1), "partial record"),
    ] {
        let mut mock = MockService::new(CrankScript::SettlesFromTreasury);
        mock.sync_recorded = recorded;
        let mut legs = ScriptedLegs::healthy();
        let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);
        assert!(
            matches!(report.verdict, Verdict::NotReady(_)),
            "{recorded:?}"
        );
        let d = report
            .steps
            .iter()
            .find(|s| s.step == riir_clippy::kat_wetrun::Step::SyncPush)
            .expect("step 4 ran")
            .lines
            .join("\n");
        assert!(d.contains(needle), "{recorded:?}: {d}");
        // The refund leg must NOT have run on an unconfirmed record.
        assert_eq!(*mock.settled.borrow(), 1, "only step 2's settle");
    }
}

/// A zero-fix scored cycle is a HARNESS failure, not a clean fixture — it
/// would record a zero spend and pass vacuously.
#[test]
fn a_scored_cycle_that_measured_nothing_fails_the_run() {
    let mock = MockService::new(CrankScript::SettlesFromTreasury);
    let mut legs = ScriptedLegs::healthy();
    legs.cycle = Err("the seeded dirty fixture produced ZERO fixes".to_string());
    let report = run(&mock, &mut legs, Network::Devnet, RunMode::Full);
    assert!(matches!(report.verdict, Verdict::NotReady(_)));
    assert!(
        !mock.writes().contains(&"POST /sync".to_string()),
        "nothing to record means nothing to push"
    );
}

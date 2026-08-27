# Proposal 007 — KAT staking: USDC in, lockup, burn-pool incentives, authority share

Status: **draft** (filed 2026-08-27). Extends [Proposal 005](005_kat_heal_service_network.md) — the third demand-side rail beside payments (T1.3/T1.4) and mining (Phase 2). **Amends Proposal 005 caveat #7** (the "no yield" closed-loop posture): refundable USDC custody + KAT incentives changes the legal surface — counsel sign-off is a hard launch gate (§Honest caveats #1), with a custody-free KAT-stake fallback arm that ships under the existing posture. Blocked by nothing at the proposal layer; execution order: after 005 T2.3 (the reward schedule this plugs into).
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 005 §Payments/§Economics (the rail + the pool this extends) × riir-chain `StakeProgram` **semantics** (`lockup_ticks` / `LockupNotExpired` / withdraw-after-lockup — cite, NOT the vehicle: KAT is ledger-side) × `KatLedger` row/queue discipline (exact-once intents, `CommitBatch` per op) × the Beanstalk snapshot defense (weight fixed BEFORE payout — prior art, §References)
Related: [Proposal 005](005_kat_heal_service_network.md), [riir-dapps Plan 005](../../riir-dapps/.plans/005_kat_solana_payments.md) (the five-check Solana rail this reuses), [Proposal 008](008_adaptive_tokenomics_riir_dao.md) (the agent that will steer the staking lever)

## TL;DR

Users **stake USDC** through the exact Solana rail payments already use (per-stake reference key, the same five verification checks), lock it for **7 days**, and earn **KAT incentives drawn from the existing epoch burn pool** — never from new supply. Unstake is all-or-nothing after the lockup, queued into the exact-once settlement ring, and paid back in a daily ops-signed batch. The **authority node takes a genesis-configured share of the incentive emission initially (e.g. 20%), decaying linearly to a floor (e.g. 5%)** — the operator is paid for bootstrapping, not forever. The CLI shows stake position, unlock countdown, floating-rate incentives (service credit, no promised APY), and the unstake rules. **This is composition over shipped rails** (T1.4's verification machinery is ~90% reusable) **plus one honest new construction**: outbound USDC signing — nothing in the stack signs Solana transactions today.

## The problem this solves

Proposal 005's economy has two demand-side rails — pay-to-mint (one-shot) and mine-to-earn (work-gated) — but no **commitment surface**: nothing lets a user say "I'm in for the medium term" and get paid for it. Concretely:

| Gap | Consequence today |
|---|---|
| No committed demand | Every KAT purchase is terminal; there is no working-capital float to budget ops against and no user skin-in-the-game. |
| The burn pool pays only miners | The 70% re-mint has one outlet; a demand shock has no counter-cyclical sink. |
| No TVL signal | Proposal 008's tokenomics agent would lack a demand-side actuator to steer. |
| Proposal 005 T2.3 freezes the reward schedule with no staker class at all | The split is mining-only by construction. |

What goes wrong without this: the economy stays spot-market-only, and the "keep miners and users running" loop (the flywheel's retention half) has no lock-in mechanism at all.

## The proposed design

### Stake (in) — the payment rail, verbatim

- `cargo heal stake --usd <N>` → the service emits a **Solana Pay transfer request** to a **dedicated stake-custody payee** (a SEPARATE wallet from the revenue payee — refundable principal must not commingle with revenue) with a fresh per-stake **reference key**.
- Verification = the T1.4 five checks unchanged (finalized; SPL-USDC **to the stake-custody payee**; amount ≥ minimum; reference present; signature unseen → `blake3("solana-tx:"‖sig64)` dedupe row, the `kat:mintevt:` pattern). Only the claim kind differs. **This is the "same as payment" the owner asked for — genuinely the same machinery.**
- New ledger rows (all integer, little-endian, one `CommitBatch` per op — the `ledger.rs` discipline):

| Key | Value |
|---|---|
| `kat:stake:<16B account>` | 40B: `principal_usdc_base u64` · `staked_at_tick u64` · `unlock_at_tick u64` · `accrued_kat i64` · `last_claim_epoch u64` |
| `kat:stakesig:<sig-key>` | 8B tick (replay dedupe) |
| `Meta` extension | `staked_total_usdc u64` · `unstake_pending u64` (float + queue counters) |

- **One stake position per account** (v1). A top-up **resets the whole position's lockup clock** to `now + LOCKUP` — punitive for late top-ups, but it is the only single-position rule that is never *less* locked than before (anti-churn is a feature here; the CLI warns). Per-tranche lockups (FIFO) are the named plan-time alternative if this reads too harsh in UX testing.

### Incentives (the earn) — burn-pool share, snapshot-gated

- The epoch reward pool (the existing 70% burn re-mint) splits by **`staking_share_bp`** (genesis config, e.g. 1000 = 10% of the pool to stakers, 90% to miners). Staking incentives are **strictly pool-funded**: when burn is zero, incentives are zero. **No new supply channel** — the Proposal 005 invariant ("free compute opens no new emission channel") extends to staking: *capital opens no new emission channel either.*
- Distribution: pro-rata by principal **among stakes that were already locked-and-old at the previous epoch boundary** — a stake counts for epoch E iff
  `staked_at_tick ≤ boundary(E−1) − LOCKUP_TICKS`.
  This is the Beanstalk lesson applied with margin: the weight snapshot precedes the payout, AND the capital must have been at risk for a full lockup before the snapshot even sees it. A flash-borrowed stake cannot farm one epoch — it must survive ≥1 week + 1 boundary. **Flash loans are time-attacks; the lockup is the clock they don't have.**
- **Authority share:** of each epoch's staking slice, `authority_share_bp` (genesis, e.g. **2000 = 20%**) routes to the authority operator account, decaying by `authority_decay_bp`/epoch to a floor (e.g. **500 = 5%**). This implements "the authority node will have xx% initially": the operator is compensated for bootstrapping demand, treasury carry, and ops — with a schedule that hands the network to stakers as it matures. All three numbers are `KatConfig` fields, CLI-visible.
- Rewards accrue as KAT service credit — **no $ figure ever renders beside a KAT accrual** (Proposal 005 caveat #15 rule, inherited verbatim), and the rate is honestly floating: it is a share of whatever the burn pool did, which can be zero. **No APY is promised anywhere.**
- Concentration cap: a single account's principal is capped at `max_stake_share_bp` of total float (e.g. 20%) — no whale owns the incentive stream.

### Unstake (out) — the one new construction, honestly named

- `cargo heal unstake` refuses while `now < unlock_at_tick`, printing remaining time. After lockup, it queues an **exact-once settlement intent** (the `queue.rs` discipline — the measured ring) for the full principal; incentives stop accruing at request time.
- **Outbound USDC signing does not exist anywhere in the stack** (payments are watch-only). v1: an ops-run **signer binary** (local, capped key) drains the unstake queue as a **daily batch** — Glacial cadence, per-day outflow cap (float protection), receipts written back with the 64-byte signature (the same no-truncation rule). Custody posture: the stake-custody wallet holds user principal and **does nothing with it** — no lending, no yield-farming of principal (§Honest caveats #2).
- **Counsel-fallback arm (design hedge):** if counsel flags USDC custody, the SAME lockup/snapshot/incentive machinery runs as **KAT-stake** — lock an existing KAT balance (no custody, no outbound transfers, no new legal surface at all). Both arms share every ledger mechanism; only the rail differs. The proposal is written so this swap is a plan-time toggle, not a redesign.

### CLI surface

```
$ cargo heal stake --usd 100
  [QR] stake 100.00 USDC → 7-day lockup → earn KAT from the burn pool
       (custody wallet, refundable after unlock; no APY promise)

$ cargo heal stake --info
  Staked:          100.00 USDC (refundable)
  Unlock:          in 4d 03h   (top-ups restart the 7-day clock)
  Incentives:      +2,431 KAT accrued — service credit, rate floats with burn
  Epoch pool:      mining 90% · staking 10% · authority share 20%→5% over 52 epochs

$ cargo heal unstake
  Queued: full refund in the next daily batch (exact-once).
  Incentive accrual stopped at request. Next unlock check: passed.
```

### Ledger + chain placement (unchanged doctrine)

No new chain program. Staking ops are user-cadence request-driven writes to the DO ledger; the chain sees only epoch-batched intents through the existing TokenRegistry composition — **the three-test rule would REJECT a staking program** (test 3: user-cadence writes). riir-chain's `StakeProgram` is cited as the semantic precedent (`lockup_ticks`, `LockupNotExpired(ticks)`, reward-accumulator cells) — the KAT design reuses the semantics in `KatLedger`, deliberately not the vehicle.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Legal posture — this amends Proposal 005 caveat #7, and counsel is a hard launch gate.** 005's closed-loop posture explicitly said "non-transferable, non-redeemable, **no yield**" to stay outside money-transmission/securities territory. Staking introduces (a) **custody of user USDC** (holding others' funds — money-transmission surface) and (b) **rewards for committing capital** (staking-as-a-service-adjacent; the SEC/Kraken-2023 class). Mitigations built into the design: principal fully refundable (not a purchase), rewards in non-transferable service credit (not money), no promised rate (pool share, can be zero), no principal deployment. **Ship NOTHING user-facing before counsel signs the structure** — and the KAT-stake fallback arm exists precisely so the feature can ship under the old posture if counsel says no.
2. **Custody risk is real and ours.** The stake-custody wallet is a hot-ish operational wallet. v1 mitigations: dedicated wallet (no commingling), per-day outflow cap, min principal, cold-backup of keys, refund-runbook. Future: multisig/FROST on the custody key (the 005 T4.3 mint-key substrate is the same upgrade). Say "operator-custodied" in user-facing copy, not "decentralized".
3. **The float is a liability, not revenue.** Staked USDC appears on no income line and funds nothing. The USD invariant of 005's G7 is unchanged; staking adds a **liquidity invariant**: `custody_balance ≥ Σ principal − daily_drain_cap`, checked per epoch (a run on unstakes drains at the cap rate and the queue holds — by design, honestly disclosed as a queue delay).
4. **Top-up clock reset is a product decision, not a law of nature.** It favors safety over UX fairness. If UX testing hates it, the fix is per-tranche lockups (FIFO unstake) — a bounded change, decided at plan time, not silently.
5. **Pool dependence means thin early incentives.** At launch burn is small; staking rewards will be symbolic. That is honest (the incentive is a share, not a rate) but it must be messaged — do not let the CLI imply earnings the pool doesn't have.
6. **The authority share is a governance-lightning rod.** 20% initial to the operator will be called extraction by someone. The defense is the schedule: published at genesis, decaying, floor-bounded, CLI-visible, and earning nothing when burn is zero. Parameter freeze happens once at genesis (G7-S sensitivity) — re-pinning mid-flight is a breach of trust, not a config change.

## Fusion lineage

1. **[Proposal 005](005_kat_heal_service_network.md) §Payments + §Economics** — the Solana rail (five checks, reference keys, sig-width rule) and the 70/20/10 burn pool this extends with a staking slice.
2. **riir-chain `StakeProgram`** (`src/programs/stake.rs`: `lockup_ticks`, `LockupNotExpired`, withdraw-after-lockup, reward-accumulator cells) — the lockup semantics, transplanted into the ledger tier the rate rule requires.
3. **`KatLedger` discipline** (`riir-dapps/src/kat/ledger.rs`: one `CommitBatch` per logical op, exact-once intents via `queue.rs`, chaos-proven zero-loss) — staking inherits the whole invariant set rather than inventing a parallel one.
4. **Beanstalk post-mortem (prior art)** — snapshot-before-payout as the flash-loan defense, strengthened here by the lockup-age rule (capital must predate the snapshot by a full lockup).

## GOAT gate (extends 005's; service gates again — the healer's offline gates untouched)

- **G1-S (money e2e, exact-once):** stake-claim → ledger rows → epoch distribution (deterministic integer math, no float money) → unstake request → batch payout; chaos sweep (fault-injected WAL, the 16-seed × 4-fault standard) — zero loss, zero double-pay; replayed stake signatures rejected.
- **G2-S (perf/placement):** zero per-op chain writes (epoch-batched intents only — grep-pinned); CLI render < 50 ms; signer drain batch wall-bounded.
- **G3-S (no-regression):** with `kat_staking` off, the riir-dapps default build and riir-clippy default binary are bit-identical to today.
- **G4-S (alloc/typing):** ledger ops ride the existing `CommitBatch` per-op discipline; USDC base units u64 + KAT `MicroUnits` i64 end-to-end — `f32/f64` in any money path is a review-blocking finding (the `BridgeEvent.fractional_cents` anti-precedent, named in 005).
- **G7-S (economics, extends G7):** the 005 simulator gains the staking axis — pool never overdrawn by mining+staking combined; incentive strictly zero when burn zero (self-funding invariant); authority share decays within bounds; a sybil-arm of many minimum stakes earns no more than one honest stake of equal principal.
- **G8-S (adversarial, the flash-loan gate):** unstake-before-unlock refused with exact remaining ticks; a stake younger than `LOCKUP + 1 boundary` is excluded from distribution (flash-stake fixture); top-up cannot backdate the snapshot age; concentration cap rejects at `max_share + 1`; unstake-then-restake within one epoch cannot collect twice.

## What ships now vs deferred

### Ships now — after 005 T2.3
- riir-dapps `src/kat/staking.rs` (feature `kat_staking`, forwards `kat_solana` + `kat_ledger`): claim handling, rows, epoch distribution, authority-share schedule, unstake queue, liquidity invariant. The ops signer binary (`examples/` or a small bin crate — plan-time call, riir-dapps-side).
- riir-clippy: `stake`/`stake --info`/`unstake` CLI under `kat_service` (render-only + QR; the caveat-#15 copy rule enforced by test).

### Deferred — gated
- KAT-stake fallback arm implementation (both arms share the machinery; build only if counsel picks it).
- Per-tranche lockups (only if UX testing rejects the clock reset).
- Multisig/FROST custody key (T4.3 substrate).
- Staking-weight as governance signal for [Proposal 008](008_adaptive_tokenomics_riir_dao.md) (telemetry only in 008 v1).

### Explicitly NOT shipped by this proposal
- Any yield on principal, any principal deployment/lending, any USDC-out beyond refunds.
- Transferability/redeemability of KAT rewards (005 caveat #7 posture holds for the token itself).
- A chain staking program (three-test rule rejects it; `StakeProgram` stays validator-native).

## Phased rollout (sketch — a plan would expand this)

### Phase 0 — ledger substrate
- [ ] T0.1 `kat:stake:*` rows + Meta extension + stake claim via the five-check machinery (custody payee configurable)
- [ ] T0.2 Lockup clock, top-up reset, unstake refusal + queue intent (exact-once)
- [ ] T0.3 Liquidity invariant + concentration cap (G1-S/G8-S local fixtures)

### Phase 1 — economics
- [ ] T1.1 Epoch staking slice: snapshot-age rule, pro-rata distribution, integer math (G1-S)
- [ ] T1.2 Authority-share schedule (genesis config, decay, floor)
- [ ] T1.3 G7-S simulator extension + parameter freeze (incl. authority triple sensitivity)

### Phase 2 — rail + CLI
- [ ] T2.1 Stake QR/claim CLI; `stake --info` render (caveat-#15 gate by test)
- [ ] T2.2 Ops signer binary + daily batch drain + per-day cap + receipt rows
- [ ] T2.3 Chaos/soak on the full stake→earn→unstake loop (G1-S full)

### Phase 3 — counsel gate
- [ ] T3.1 Counsel review of custody + incentive structure → pick USDC arm or KAT-stake arm (launch blocker)
- [ ] T3.2 User-facing copy review (queue-delay disclosure, no-APY language)

## Risks

1. **Regulatory (highest):** misclassified staking converts a checkout flow into a regulated activity — mitigated by the design posture + the fallback arm + the counsel gate (caveat #1).
2. **Custody incident:** hot-key compromise drains the float — mitigated by caps, dedicated wallet, cold backup; residual accepted and disclosed.
3. **Run dynamics:** a demand shock queues unstakes at the drain cap — by design, but a communications problem if undisclosed; the CLI shows queue depth.
4. **Incentive starvation:** thin early pool makes staking decorative — honest, and the 008 agent's staking-share lever exists precisely to respond.
5. **Architectural:** none new — no repo boundary moves; everything lands in already-owned planes.

## Out of scope

- Governance/voting rights for stakers (008 v1 keeps governance bounded-agent; staking is economic, not political).
- Liquid staking tokens, delegated staking, secondary market for locked positions.
- Any change to KAT transferability.

## References

### In-house (substrates this composes — grep-verified 2026-08-27)
1. [riir-dapps Plan 005 / `src/kat/solana.rs`](../../riir-dapps/src/kat/solana.rs) — the five-check claim machinery (finalized, payee, amount, reference, sig-dedupe) this reuses with a new claim kind; the `[u8; 64]` signature-width rule.
2. [riir-dapps `src/kat/ledger.rs`](../../riir-dapps/src/kat/ledger.rs) — the row layout, `CommitBatch`-per-op discipline, `Meta` accumulators, exact-once epoch settle this extends.
3. `riir-chain/src/programs/stake.rs` — `StakeDeposit { amount, lockup_ticks }`, `LockupNotExpired(remaining)`, withdraw-after-lockup, reward cells: **semantic precedent only** (KAT stays ledger-side per the rate rule).
4. `riir-chain/src/economics/inflation.rs` — `InflationSchedule` + uptime-weighted `distribute_rewards`: the "weights at snapshot" family the pro-rata rule belongs to.
5. [Proposal 005](005_kat_heal_service_network.md) — caveats #7/#15 (amended/inherited), G7 (extended), T2.3 (the schedule this plugs into).

### External (cited-only)
6. **Beanstalk flash-loan governance attack post-mortems** (Veridise, Cyfrin glossary, 2023) — snapshot-before-payout as the flash-loan defense; our lockup-age rule is that defense plus a week of time the attacker does not have.
7. **EIP-7265 (Circuit Breaker Standard)** — the per-day outflow cap follows the same "bound the outflow" doctrine (relevant to 008's guards too).
8. **FinCEN prepaid-access guidance** (already cited by 005 caveat #7) — the custody question this proposal adds to that review.

## TL;DR

**Ship it (phased, counsel-gated) — the rail is ~90% composition, the economics are pool-funded with no new supply channel, and the flash-loan defense is structural (lockup + snapshot-age) rather than vigilance.** The one new construction is outbound USDC signing, and the one amended posture is 005 caveat #7 — both are named, both are gated. Next action: 005 T2.3 lands the reward schedule → open the 007 plan starting at T0.1.

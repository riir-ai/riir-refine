# Proposal 007 — KAT staking: USDC in, lockup, burn-pool incentives, authority share

Status: **draft** (filed 2026-08-27). **EXECUTED 2026-08-27/28 (subagent campaign): Phase 0 + Phase 1 COMPLETE (dapps `7a18690`) + T2.1 CLI (clippy `6ee000e`) + **T2.3 chaos/soak COMPLETE 2026-08-28 (dapps `b4b56c0`, Plan 011 / Bench 004)**; T2.2 (ops signer — outbound USDC tx construction) deferred pending ops decisions; Phase 3 counsel gate unchanged (launch blocker).** **Cross-ref 2026-08-30 — Proposal 010 Phase 4 ACTIVATED staking** (`staking_share_bp` 0→500 in `EpochParams::DEFAULT`, `yield_ceiling_bp` 22 armed in the same default, the ordering constraint enforced; Proposal 011 keeps live USDC mock-tested until this proposal's Phase-3 counsel gate): the `[mock: … default is 0 …]` CLI note below and 008's "first lever move pinned to 500" bootstrap rider are HISTORICAL — the first governance move on the lever is now Δ-bounded from 500. Extends [Proposal 005](005_kat_heal_service_network.md) — the third demand-side rail beside payments (T1.3/T1.4) and mining (Phase 2). **Amends Proposal 005 caveat #7** (the "no yield" closed-loop posture): refundable USDC custody + KAT incentives changes the legal surface — counsel sign-off is a hard launch gate (§Honest caveats #1), with a custody-free KAT-stake fallback arm that ships under the existing posture. Blocked by nothing at the proposal layer; execution order: after 005 T2.3 (the reward schedule this plugs into). **Amended 2026-08-27 (owner design review, same day):** (a) mining rewards gain **opt-in auto-restaking** (default OFF — 005's never-brick rule; compounding as commitment escalation, not a forced lock), (b) positions unify on **per-tranche lockup rows** (each deposit owns its own 7-day clock — supersedes the single-position clock-reset design and resolves caveat #4), (c) unstake refunds are **refund-to-source-bound** (the funding address recorded at first stake; the service key can trigger a refund but never redirect one — zero principal blast radius on KAT-key compromise). **Amended 2026-08-27 (2nd owner review):** (d) unstake refunds pass through a **genesis-frozen escrow window with optional per-account venting** (unbonding-queue semantics — run smoothing + an anomaly window before principal leaves custody; exit mechanics are constitutionally NEVER an [008](008_adaptive_tokenomics_riir_dao.md) lever), (e) ban/unban **voting** moves to [Proposal 008](008_adaptive_tokenomics_riir_dao.md) as the first governance primitive (agent-proposes/stakers-vote/authority-executes — this proposal's voting out-of-scope row is superseded).
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
| `kat:stakeacct:<16B account>` | 45B account head: `funding_solana_pubkey [u8;32]` · `restake u8` · `next_tranche_seq u32` · `accrued_kat i64` |
| `kat:staketranche:<16B>:<seq>` | 25B: `kind u8` (usdc/kat) · `amount u64` (base units) · `staked_at_tick u64` · `unlock_at_tick u64` |
| `kat:stakesig:<sig-key>` | 8B tick (replay dedupe) |
| `Meta` extension | `staked_total_usdc u64` · `unstake_pending u64` (float + queue counters) |

- **Tranche-based positions (amended 2026-08-27).** An account's stake is a FIFO list of tranches, each owning its **own** 7-day clock — a top-up never re-locks earlier tranches (the original single-position clock-reset design is superseded; see caveat #4). USDC tranches share **one funding address per account** (§Unstake); KAT tranches come from reward restaking (next subsection). `unstake` drains all currently-unlocked tranches in one request — one button, no amount picker, simplicity preserved.

### Incentives (the earn) — burn-pool share, snapshot-gated

- The epoch reward pool (the existing 70% burn re-mint) splits by **`staking_share_bp`** (genesis config, e.g. 1000 = 10% of the pool to stakers, 90% to miners). Staking incentives are **strictly pool-funded**: when burn is zero, incentives are zero. **No new supply channel** — the Proposal 005 invariant ("free compute opens no new emission channel") extends to staking: *capital opens no new emission channel either.*
- Distribution: pro-rata **per tranche** among tranches that were already locked-and-old at the previous epoch boundary — a tranche counts for epoch E iff
  `staked_at_tick ≤ boundary(E−1) − LOCKUP_TICKS`.
  This is the Beanstalk lesson applied with margin: the weight snapshot precedes the payout, AND the capital must have been at risk for a full lockup before the snapshot even sees it. A flash-borrowed stake cannot farm one epoch — it must survive ≥1 week + 1 boundary. **Flash loans are time-attacks; the lockup is the clock they don't have.**
- **Authority share:** of each epoch's staking slice, `authority_share_bp` (genesis, e.g. **2000 = 20%**) routes to the authority operator account, decaying by `authority_decay_bp`/epoch to a floor (e.g. **500 = 5%**). This implements "the authority node will have xx% initially": the operator is compensated for bootstrapping demand, treasury carry, and ops — with a schedule that hands the network to stakers as it matures. All three numbers are `KatConfig` fields, CLI-visible.
- Rewards accrue as KAT service credit — **no $ figure ever renders beside a KAT accrual** (Proposal 005 caveat #15 rule, inherited verbatim), and the rate is honestly floating: it is a share of whatever the burn pool did, which can be zero. **No APY is promised anywhere.**
- Concentration cap: a single account's principal is capped at `max_stake_share_bp` of total float (e.g. 20%) — no whale owns the incentive stream.

### Reward restaking (opt-in compounding — amended 2026-08-27)

The owner's ask: should mining rewards auto-stake into the same lockup? **Opt-in, default OFF.** The default must preserve 005's never-brick rule (*"mining accrual and settlement CONTINUE at balance 0 — earning it back by syncing data is the loop that keeps a broke user running"*): default-locked rewards would brick exactly that miner for up to 7 days, and delayed burns cut against the sink that gives KAT value. The retention design agrees — 008's variable-ratio schedule needs **liquid** base rewards for instant reinforcement; compounding is the *commitment escalation* layer a believer opts into.

- `cargo heal stake --restake on|off` (account flag, enforced server-side at credit time): **on** redirects epoch-settled mining credits into **KAT tranches** (same FIFO rows, `kind = kat`, same 7-day clock each). KAT-tranche redemption after unlock is a pure ledger op — no custody, no outbound batch — so it is instant; only USDC principal rides the daily refund batch.
- The staking slice now serves two classes — USDC-principal tranches and KAT-reward tranches — split by a **fixed sub-share** (`usdc_stake_share`/`kat_stake_share`, genesis config; a 008 lever). **Never pro-rata across units**: USDC base units and KAT micro-units are different denominations, and a cross-unit pro-rata is a category error that would price an arbitrage between the two stake kinds.
- **Compounding is structurally bounded**: payouts are a partition of a fixed burn-funded slice (percentages of the pool), never a promised rate — no spiral even at 100% opt-in. The G7-S simulator models the loop explicitly.
- This is the KAT-stake fallback arm's machinery shipped **anyway** — if counsel picks the fallback, the USDC arm is what drops, not this.
- Securities note: yield on KAT you hold is the same class as the fallback arm — inside 007's single counsel review (caveat #1), no new gate.

### Unstake (out) — the one new construction, honestly named

- `cargo heal unstake` refuses while any targeted tranche is younger than its `unlock_at_tick`, printing remaining time. After lockup, it queues an **exact-once settlement intent** (the `queue.rs` discipline — the measured ring) covering all unlocked tranches with `releasable_at = request_tick + ESCROW_TICKS`; incentives stop accruing on them at request time.
- **Refund-to-source binding (amended 2026-08-27):** the **funding Solana address** — captured from the verified incoming transfer's token-balance delta — is recorded at the account's first USDC tranche, every later USDC tranche must originate from the same address or the claim is **rejected**, and refunds go **only** to that address. The KAT account key can *trigger* a refund but can never *redirect* one: **principal's only path is `funding wallet → custody → funding wallet`** — a compromised service key has zero principal blast radius, and address-switching through staking (stake dirty, unstake to a clean address) is structurally impossible. A new wallet means a new KAT account; re-binding an existing position is deliberately NOT self-service in v1 (caveat #7).
- **Escrow + venting (2nd owner review, 2026-08-27):** the refund rides the intent through a **genesis-frozen escrow window** (`ESCROW_TICKS`, default ~48 h) before the daily batch may move it, and refunds above `VENT_THRESHOLD` drain in **equal integer daily slices** over `VENT_DAYS` (remainder on the final slice — Sablier-lite streaming). The semantic substrate ships: riir-chain's `UnbondingQueue` (`UnbondingEntry { claimable_at_tick }` + `drain_ready(current_tick)`, `slashing.rs`) is exactly this shape — cited, ledger-side again. Why the friction is worth it: (a) an **anomaly window** — a run is visible in the escrow queue *before* principal leaves custody, and [008](008_adaptive_tokenomics_riir_dao.md)'s breaker can pause the drain inside it; (b) **predictable outflow** — cold→hot pre-funding, and escrow depth doubles as the agent's *leading exit signal* (exits appear in telemetry before they appear in the treasury); (c) whale venting smooths concentration. Honest cost: the lockup already covers flash-loan defense, so escrow exists for treasury/ops protection only — it must stay **short** and is disclosed at BOTH stake and unstake time, or it reads as hostage-keeping. **Exit mechanics (escrow length, venting thresholds, drain caps) are constitutionally frozen — never a 008 lever**: an agent that could lengthen exits could trap funds.
- **Outbound USDC signing does not exist anywhere in the stack** (payments are watch-only). v1: an ops-run **signer binary** (local, capped key) drains **matured** escrow intents as a **daily batch** (only `releasable_at ≤ now`, under the global per-day cap + venting schedule), receipts written back with the 64-byte signature (the same no-truncation rule). Custody posture: the stake-custody wallet holds user principal and **does nothing with it** — no lending, no yield-farming of principal (§Honest caveats #2).
- **Counsel-fallback arm (design hedge):** if counsel flags USDC custody, the SAME lockup/snapshot/incentive machinery runs as **KAT-stake** — lock an existing KAT balance (no custody, no outbound transfers, no new legal surface at all). Both arms share every ledger mechanism; only the rail differs. The proposal is written so this swap is a plan-time toggle, not a redesign.

### CLI surface

```
$ cargo heal stake --usd 100
  [QR] stake 100.00 USDC → 7-day lockup → earn KAT from the burn pool
       (custody wallet, refundable after unlock; no APY promise)

$ cargo heal stake --info
  Staked:          100.00 USDC (refundable)
  Unlock:          in 4d 03h   (oldest tranche; each deposit owns its own 7-day clock)
  Incentives:      +2,431 KAT accrued — service credit, rate floats with burn
  Epoch pool:      mining 90% · staking 10% (usdc 70% / kat-reward 30% of the slice)
                   [updated 2026-08-30: `staking_share_bp` is ACTIVATED at 500 bp (010
                    Phase 4 — carved from mining, ceiling 22 armed, ordering constraint
                    enforced); the original 0-bp bootstrap posture is historical]
                   · authority share 20%→5% over 60 epochs
  Reward restake:  ON — mining credits auto-lock as KAT tranches (7-day clock each;
                   redeem instantly after unlock; opt back out anytime)

$ cargo heal unstake
  Queued: refund enters a 48h escrow, then drains in the next daily batch
  (exact-once; refunds over $10,000 vent in equal daily slices — 5 days).
  Refund returns to the funding wallet (bound at first stake — never changeable
  by the service key). Incentive accrual stopped at request.
```

### Ledger + chain placement (unchanged doctrine)

No new chain program. Staking ops are user-cadence request-driven writes to the DO ledger; the chain sees only epoch-batched intents through the existing TokenRegistry composition — **the three-test rule would REJECT a staking program** (test 3: user-cadence writes). riir-chain's `StakeProgram` is cited as the semantic precedent (`lockup_ticks`, `LockupNotExpired(ticks)`, reward-accumulator cells) — the KAT design reuses the semantics in `KatLedger`, deliberately not the vehicle.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **Legal posture — this amends Proposal 005 caveat #7, and counsel is a hard launch gate.** 005's closed-loop posture explicitly said "non-transferable, non-redeemable, **no yield**" to stay outside money-transmission/securities territory. Staking introduces (a) **custody of user USDC** (holding others' funds — money-transmission surface) and (b) **rewards for committing capital** (staking-as-a-service-adjacent; the SEC/Kraken-2023 class). Mitigations built into the design: principal fully refundable (not a purchase), rewards in non-transferable service credit (not money), no promised rate (pool share, can be zero), no principal deployment. **Ship NOTHING user-facing before counsel signs the structure** — and the KAT-stake fallback arm exists precisely so the feature can ship under the old posture if counsel says no.
2. **Custody risk is real and ours.** The stake-custody wallet is a hot-ish operational wallet. v1 mitigations: dedicated wallet (no commingling), per-day outflow cap, min principal, cold-backup of keys, refund-runbook. Future: multisig/FROST on the custody key (the 005 T4.3 mint-key substrate is the same upgrade). Say "operator-custodied" in user-facing copy, not "decentralized".
3. **The float is a liability, not revenue.** Staked USDC appears on no income line and funds nothing. The USD invariant of 005's G7 is unchanged; staking adds a **liquidity invariant**: `custody_balance ≥ Σ principal − daily_drain_cap`, checked per epoch (a run on unstakes drains at the cap rate and the queue holds — by design, honestly disclosed as a queue delay). The escrow window (2nd review) is the early-warning half: a run is visible in the queue *before* it drains.
4. **Lockup granularity is per-tranche (amended 2026-08-27).** The original draft reset the whole position's clock on top-up — safe but punitive; the reward-restaking amendment forced the tranche model (each deposit owns its 7-day clock, FIFO unstake), which resolves the harshness as a side effect. Residual: tranche rows need a compaction discipline (drained tranches pruned, seq high-water bounded) — plan-time detail.
5. **Pool dependence means thin early incentives.** At launch burn is small; staking rewards will be symbolic. That is honest (the incentive is a share, not a rate) but it must be messaged — do not let the CLI imply earnings the pool doesn't have.
6. **The authority share is a governance-lightning rod.** 20% initial to the operator will be called extraction by someone. The defense is the schedule: published at genesis, decaying, floor-bounded, CLI-visible, and earning nothing when burn is zero. Parameter freeze happens once at genesis (G7-S sensitivity) — re-pinning mid-flight is a breach of trust, not a config change.
7. **The funding-wallet binding strands principal on wallet loss.** Refund-to-source (§Unstake) means a lost/locked Solana wallet strands the principal until an ops-assisted identity recovery — a manual process, not a feature. Deliberately NO self-service refund-address change in v1: a changeable destination reopens the redirect attack the binding kills. The named future upgrade is a delay-and-notice change flow (exchange-style withdrawal-address discipline) if real users hit this. Exchange-deposit-address quirks get a CLI warning ("stake from a wallet you control").
8. **Reward restaking is yield-on-KAT — same counsel envelope.** Opt-in compounding on KAT you already hold is the same securities class as the KAT-stake fallback arm (caveat #1); one review covers both. Default-OFF is also legal hygiene: the default experience promises nothing.

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
- **G7-S (economics, extends G7):** the 005 simulator gains the staking axis — pool never overdrawn by mining+staking combined; incentive strictly zero when burn zero (self-funding invariant); authority share decays within bounds; a sybil-arm of many minimum stakes earns no more than one honest stake of equal principal; the reward-restake compounding loop is modeled explicitly (partition-bounded — no emission spiral at 100% opt-in); a **mass-unstake run scenario** — outflow bounded by escrow + cap + venting, custody never overdrawn.
- **G8-S (adversarial, the flash-loan gate):** unstake-before-unlock refused with exact remaining ticks; a tranche younger than `LOCKUP + 1 boundary` is excluded from distribution (flash-stake fixture); a USDC top-up from a different funding address is rejected at claim; top-up cannot backdate the snapshot age; concentration cap rejects at `max_share + 1`; unstake-then-restake within one epoch cannot collect twice; reward tranches obey the same age rule while non-restaked credits stay liquid at balance 0 (the never-brick fixture); a compromised account key cannot redirect a refund (destination-pinned fixture); **nothing leaves custody before `releasable_at`** (escrow maturation fixture); venting splits are exact integers with the remainder on the final slice; a paused breaker (008) holds the drain with the queue intact.

## What ships now vs deferred

### Ships now — after 005 T2.3
- riir-dapps `src/kat/staking.rs` (feature `kat_staking`, forwards `kat_solana` + `kat_ledger`): claim handling, rows, epoch distribution, authority-share schedule, unstake queue **+ escrow/venting**, liquidity invariant. The ops signer binary (`examples/` or a small bin crate — plan-time call, riir-dapps-side).
- riir-clippy: `stake`/`stake --info`/`unstake` CLI under `kat_service` (render-only + QR; the caveat-#15 copy rule enforced by test).

### Deferred — gated
- KAT-stake-**only** fallback deployment (the tranche + incentive machinery ships regardless via reward restaking; only the USDC rail drops if counsel picks the fallback).
- Self-service refund-address change with delay-and-notice (only if real users hit caveat #7's lost-wallet case).
- Multisig/FROST custody key (T4.3 substrate).
- Staking-weight as governance signal for [Proposal 008](008_adaptive_tokenomics_riir_dao.md) (telemetry only in 008 v1).

### Explicitly NOT shipped by this proposal
- Any yield on principal, any principal deployment/lending, any USDC-out beyond refunds.
- Transferability/redeemability of KAT rewards (005 caveat #7 posture holds for the token itself).
- A chain staking program (three-test rule rejects it; `StakeProgram` stays validator-native).

## Phased rollout (sketch — a plan would expand this)

### Phase 0 — ledger substrate
- [x] T0.1 `kat:stake:*` rows + Meta extension + stake claim via the five-check machinery (custody payee configurable) — **DONE 2026-08-28, riir-dapps `7a18690`** ([Plan 008](../../riir-dapps/.plans/008_kat_staking_phase0_1.md)): 45B head / 25B tranche / sig-dedupe rows, the claim via the extracted shared `verify_checks` core against a SEPARATE custody payee, refund-to-source binding (unique negative-delta sender; ambiguous ⇒ reject)
- [x] T0.2 Lockup clock (per-tranche — supersedes the top-up-reset design, amendment (b)), unstake refusal + queue intent (exact-once) — **DONE (tranche model per amendment (b))**: per-tranche 7-day clocks, FIFO, top-up never re-locks earlier tranches, crash-safe drains (durable intent before deletes, covered-seq exclusion ⇒ double-refund structurally impossible), escrow + venting (constitutionally frozen, commented in-source)
- [x] T0.3 Liquidity invariant + concentration cap (G1-S/G8-S local fixtures) — **DONE**: custody ≥ Σ principal − drain cap per epoch; cap bootstrap-aware (a lone staker holds 100% by construction); all 23 G8-S fixtures green

### Phase 1 — economics
- [x] T1.1 Epoch staking slice: snapshot-age rule, pro-rata distribution, integer math (G1-S) — **DONE**: `staked_at + lockup ≤ boundary(E−1)` (the Beanstalk snapshot+age defense), fixed usdc/kat class sub-shares (never pro-rata across units), u128 pro-rata floors, full-or-defer pool room, exact-once marker
- [x] T1.2 Authority-share schedule (genesis config, decay, floor) — **DONE**: integer-exact decay to the 500bp floor over 60 epochs, monotone, CLI-visible KatConfig fields
- [x] T1.3 G7-S simulator extension + parameter freeze (incl. authority triple sensitivity) — **DONE**: 6 sim arms green (pool never overdrawn, zero-burn⇒zero-incentive, authority decay, sybil ≤ honest, restake partition-bounded, 20-account mass-unstake run bounded by escrow+cap+venting)

### Phase 2 — rail + CLI
- [x] T2.1 Stake QR/claim CLI; `stake --info` render (caveat-#15 gate by test) — **DONE 2026-08-28, riir-clippy `6ee000e`+`ba262a8`** ([Plan 041](../.plans/041_kat_007_t21_staking_cli.md)): `src/kat_stake_client.rs` (mirrored base58 pinned by dapps' math-matrix goldens, transfer-URL composer, fresh on-curve reference keys — zero new deps, `RIIR_HEAL_KAT_STAKE_PAYEE` custody config), the three commands + QR + honest not-live fallbacks, the copy gate PASS (no $ beside KAT, service-credit labeling, all four unstake disclosures pinned). The proposal mock's "top-ups restart the clock" line superseded by amendment (b) — the render ships the amended truth
- [ ] T2.2 Ops signer binary + daily batch drain + per-day cap + receipt rows — **DEFERRED**: the escrow/venting/vent-slice LEDGER semantics landed in Phase 0; the outbound USDC transaction construction (the one genuinely new construction) needs ops decisions (RPC provider, key custody) — re-open as its own unit
- [x] T2.3 Chaos/soak on the full stake→earn→unstake loop (G1-S full) — **DONE 2026-08-28, riir-dapps `b4b56c0`** ([Plan 011](../../riir-dapps/.plans/011_t25_g7_econ_sim_and_007_t23_staking_chaos.md) COMPLETE / [Bench 004](../../riir-dapps/.benchmarks/004_g7_econ_sim_and_staking_chaos.md)): `tests/kat_staking_chaos.rs` — 26 tests, the 16×4 full-loop sweep + composed negatives + the `#[ignore]` deep soak `g1s_soak_chained_faults_and_crashes`. The soak harness is committed and the sweep is green; the deep-depth run itself is an operator step (it is `#[ignore]`d by design)

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

- Governance/voting rights for stakers — **superseded 2026-08-27 by the [008](008_adaptive_tokenomics_riir_dao.md) amendment**: ban/unban voting (agent-proposes/stakers-vote) is now 008 scope as the first governance primitive; staking remains otherwise economic, not political.
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

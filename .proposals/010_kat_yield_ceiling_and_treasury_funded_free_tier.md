# Proposal 010 — KAT yield ceiling + treasury-funded free tier (two ledger budget rules)

Status: **COMPLETE — Phases 1–3 LANDED (riir-dapps `c27300b`, 2026-08-29); Phase 4 (the delegated perf/sec activation call) + the dao-side mirror LANDED 2026-08-30** (filed 2026-08-29). G16 PASS: `riir-dao Bench 003`. Enters through the re-open path [Proposal 008](008_adaptive_tokenomics_riir_dao.md) T1.4 names verbatim: *"Re-open levers (v2 candidates incl. decay_factor/Inflate, **or a ledger budget-rule change**) require a NEW plan."* This is the ledger-budget-rule half. It does **not** re-roll the G5 campaign and does **not** ask for agent autonomy — the agent stays advisory-only. **Consumer cross-ref 2026-09-01 — Proposal 011** restates both rules at genesis and builds on them: ask 10's daily free-tier crank rides the M2 treasury-first grant (`free_tier_daily_kat`, never-lever family), and the ceremony yaml carries `yield_ceiling_bp` 22 + the armed 6500/500/2000/1000 split as ratified defaults (drift ⇒ ceremony FAILS).

## TL;DR

Two independent, aggregate-only budget rules for `riir-dapps`'s `KatLedger`:

- **M1 — `yield_ceiling_bp`**: cap the **KAT-class** staking payout at a per-epoch yield rate on the eligible staked base. Excess is simply never drawn, so it stays in the mining pool. Closes a live **thin-base yield exploit** (below). Genesis saturates at the bound's safe end — the agent has zero upward headroom, exactly as `decay_factor` does after Issue 006.
- **M2 — treasury-first free grant**: fund `KatLedger::grant` from the treasury balance down to a genesis reserve floor, minting only the shortfall. The grant amount, eligibility and once-per-account rule are **untouched** — the free grant stays a never-lever; only its funding source moves.

Neither rule is a per-account rule. Both are aggregate invariants, so **neither can be gamed by splitting an account** — the defence is structural, not a heuristic.

## The problem this solves

### 1. Thin-base yield is unbounded today (the security half — this is the load-bearing reason)

`pay_due_staking` (`riir-dapps/src/kat/staking.rs:1276-1330`) computes:

```
slice   = burn_total × staking_share_bp / 10⁴      # bounded by the lever + pool_room
kat_sub = (slice − authority_cut) × kat_stake_share_bp / 10⁴
share_i = kat_sub × amount_i / kat_total           # pro-rata, floor per tranche
```

`slice` is bounded on the **numerator** side (`STAKING_SHARE` ≤ 3 000 bp, and full-or-defer against `pool_room`). Nothing bounds the **denominator**. The realized yield rate is `kat_sub / kat_total`, and `kat_total` is the eligible staked base — an unbounded-from-below quantity.

Concretely: at the moment staking activates, one staker holding a single eligible KAT tranche receives the **entire** `kat_sub` for that epoch. With `staking_share_bp` at its first legal value (500 bp) and a healthy `burn_total`, that is an arbitrarily large multiple of principal. It is not a bug in the split — the split is correct and conservation holds — it is a **missing rate bound**. The failure mode is an early-staker jackpot that is fully legal under every shipped guard.

This is exactly the class 008's safety inversion exists to catch (*the ledger enforces bounds at apply time*), and it is currently unenforced because the lever set bounds **shares of the burn**, never **rate on capital**.

It is dormant only because `staking_share_bp` ships at `0` (`compose.rs:47`). It arms itself the first epoch governance moves that lever to 500 bp. **Fixing it before activation is strictly cheaper than after.**

### 2. The free grant is pure mint pressure

`grant` (`ledger.rs:729`) unconditionally adds `cfg.free_grant` to `meta.mint_demand`. The treasury accrues a 20 % leg of every settled epoch (`compose.rs:48`, `MintTo { dest: cfg.treasury }` at `compose.rs:195`) plus the staking authority cut (`staking.rs:1329`), and today has **no consumer at all** beyond the (unimplemented) governance stipend. Free-tier onboarding mints new supply while a funded treasury sits idle. Routing grants through the treasury first converts idle treasury into onboarding runway and removes that mint pressure one-for-one.

## What the original sketch got wrong (recorded, not smoothed over)

Three parts of the informal design dissolved on contact with the code. Recording them so the same ideas do not get re-proposed:

1. **A progressive tax on per-staker yield is a no-op.** The payout is pro-rata within a class, so **every tranche in a class realizes the identical rate**. There is no per-staker yield dispersion to tax. Progressive brackets would place every staker in the same bracket every epoch — a global cap wearing a costume. M1 is that global cap, stated honestly.
2. **The "bell shape / water-fill to the next-highest" redistribution solved a non-problem.** It targeted yield-rate concentration, which pro-rata already precludes. Concentration exists only in *absolute* KAT, which is proportional to capital — that is what staking is. Dropped entirely.
3. **√stake weighting was a sybil trap.** N accounts of S/N each carry total weight √(N·S) > √S. Concave weighting *rewards* splitting. Never ship it. M1 avoids the whole family by being aggregate-only.

**Arithmetic correction (matters — it changes the constants):** the per-epoch rate for a 7-day epoch (`ticks_per_epoch = 604_800`, `staking.rs:190`) is `(1+x)^(365/7) = 1+APY`, i.e. **21.8 bp/epoch for 12 % APY, not 218 bp**. An earlier informal note carried a 10× error. Every constant below is recomputed at 52.142857 epochs/year.

| `yield_ceiling_bp` (per epoch) | APY |
|---|---|
| 9 | 4.80 % |
| 12 | 6.45 % |
| 15 | 8.13 % |
| 18 | 9.83 % |
| 20 | 10.98 % |
| **22 (genesis = `max_bp`)** | **12.14 %** |

1 bp of per-epoch resolution ≈ 0.55 pp of APY near the ceiling. **Decision: keep the unit uniform (bp/epoch) across every lever rather than introduce a finer unit for one row.** Mixed units in a governance layer are a silent-catastrophe footgun; 12.14 % vs an exact 12.00 % is immaterial for a guard. No floats anywhere — the check is integer-only, so it is bit-deterministic under replay.

## M1 — `yield_ceiling_bp`

### Rule

At `pay_due_staking`, after the eligible classes are built (`staking.rs:1294-1304`) and **before** the payout loop (`staking.rs:1319`):

```
kat_cap = kat_total × yield_ceiling_bp / 10⁴      # u128, floor
kat_sub = min(kat_sub, kat_cap)
```

The excess is **never drawn**: `meta.pool_paid` only advances by what was actually paid (`staking.rs:1332`), so unclaimed slice stays as pool room for mining. **No new accounting, no new sink, no `mint_demand` change, no conservation risk** — the strongest reason to prefer this over routing the excess anywhere explicitly.

### Scope limit: KAT class only (deliberate)

The USDC class stakes USDC-denominated principal and is paid in KAT (`staking.rs:1289`). Its "rate" is KAT-per-USDC — a **cross-denomination emission**, not a yield percentage. Capping it in bp would silently assume a 1:1 KAT/USDC price, and per 008's never-lever list KAT↔USD pricing is operator-owned and off-ledger. **The ledger must not infer a price it is not allowed to hold.** So M1 binds the KAT class only, and this is a documented scope limit rather than an oversight. A USDC-class emission bound is a separate, price-aware decision and is out of scope here.

### Bound, and why genesis sits at `max_bp`

```rust
/// `yield_ceiling`: max KAT-class staking yield, bp per 7-day epoch.
/// 9 bp (4.80% APY) – 22 bp (12.14% APY), Δ ≤ 2 bp.
pub const YIELD_CEILING: LeverBound = LeverBound { min_bp: 9, max_bp: 22, max_delta_bp: 2 };
```

Higher bp = looser cap = more emission = the **inflationary** extreme. Genesis is pinned at `max_bp` exactly, so — identically to `DECAY_FACTOR` after Issue 006 — **the agent holds zero upward headroom and every in-bounds move it can make tightens yield.** One-directional by construction, with no special case in the guard and no reliance on the agent behaving well. Loosening past 12.14 % APY is a bound widening: an owner-reviewed ledger upgrade, never an agent action. Pinned by a test that is verified to *fire*, not merely to pass (`yield_ceiling_genesis_saturates_lever_safe_end`).

### Row v2 migration

`EpochParams` is a fixed 16-byte row, 4 × `u32` LE, `from_row_bytes` rejecting any other length (`compose.rs:66-92`). M1 adds a 5th field → 20 bytes. Decisions:

- `to_row_bytes` emits **20 bytes** always.
- `from_row_bytes` accepts **both**: 20 → v2; 16 → v1 with `yield_ceiling_bp = YIELD_CEILING_DISABLED` (`u32::MAX`, an explicit sentinel — `0` would read as "cap at 0 %" and silently zero every payout). Any other length stays `None`, never a panic.
- `valid()` is **unchanged**: the ceiling is a rate, not a share, so it is excluded from the 10 000 bp partition sum. No interaction with the `apply_params` step-1 sum rule.
- This is the `:v2` widening already declared PENDING for `USDC_KAT_SUB_SPLIT` and `DECAY_FACTOR` (`governance.rs:26`, `:180`). **M1 rides that one migration; it does not invent a second.** Landing all three row fields together is the cheaper and safer sequencing, and is the reason this proposal touches `governance.rs` at all.
- `bounds_violation` gains a 5th check, taking the apply-time-enforced lever count from 4 to 5 (and to 7 if the two pending rows land in the same migration).

## M2 — treasury-funded free tier

### Rule (in `KatLedger::grant`, `ledger.rs:729`)

```
if treasury_balance − free_grant ≥ treasury_grant_floor:
    debit treasury, credit account, DO NOT touch mint_demand
else:
    today's path (credit account, mint_demand += free_grant)
```

Once-per-account, amount, and eligibility are untouched. The `granted` flag, the `AlreadySettled` refusal, and the returned value are byte-identical.

### The reserve floor, and who may move it

`treasury_grant_floor: MicroUnits`, a **genesis constant in `KatConfig`** — deployment-frozen, **not an agent lever, and deliberately not votable**. Rationale: the floor guards the governance stipend's runway, and the stipend is *the agent's own compensation* (008 §5). An agent able to move the floor that protects its own pay is a direct conflict of interest. Genesis value `FREE_GRANT_KAT × 10_000` = 10¹² micro (1 000 000 KAT) — ten thousand grants of runway held in reserve.

This also answers the priority question 008 left open: **stipend first, grants from the surplus above the floor.** Stateless — one subtraction and one comparison, no per-epoch budget row, no new epoch coupling.

### Sybil consideration

Mass account creation to farm grants is a pre-existing exposure; M2 changes *which* resource it drains (treasury instead of supply). The floor bounds that drain absolutely, and the fallback path is exactly today's behavior — so **M2 is strictly no worse than the status quo at every treasury level**, and better above the floor. Grant eligibility hardening (proof-of-personhood, registration cost) is a separate concern and is explicitly out of scope.

> Cross-ref (2026-09-08): `Issue 075` — why the free tier never demands a burn at heal time: the burn is the existing local meter (`src/kat_meter.rs`) settled at epoch cadence, and M2 exists precisely for users with nothing to burn.

## Levers table (additions to 008 §The levers)

| Lever | Parameter | Bounds (per epoch) | Enforcement |
|---|---|---|---|
| **yield ceiling** | `EpochParams::yield_ceiling_bp` | 9–22 bp, Δ ≤ 2 bp; **genesis = 22 = `max_bp`** | LIVE at the `:v2` row widening; one-directional (agent may only tighten) |
| (funding) | `KatConfig::treasury_grant_floor` | genesis constant, 10¹² micro | **NOT a lever** — never-lever by conflict-of-interest; owner-only |

Unchanged: every never-lever in 008 §Never levers. The free grant itself, `max_supply`, pack prices, USDC-out paths, transferability, exit mechanics and ban semantics are all untouched by this proposal.

## Performance

- **M1:** `kat_total` is already computed at `staking.rs:1304`. The rule adds one `u128` multiply, one divide, one `min` — **O(1) per epoch, zero allocation, no new scan, no new row read.** It cannot regress the hot path because it is not on one; `pay_due_staking` is epoch-cadence (7 days).
- **M2:** one extra `account_row` read for the treasury and one extra `CommitBatch` push (capacity 2 → 3) on the grant path. Grant is once-per-account-lifetime — the least hot write in the ledger.
- **Row v2:** 16 → 20 bytes on `kat:params:<epoch>`, one row per epoch. 4 bytes × 52 epochs/year. Immaterial.
- No change to any per-op or per-tick path. Nothing here touches the 20 Hz tier, and nothing here adds a chain write.

## Security

| Vector | Disposition |
|---|---|
| Account splitting / sybil | **Structurally immune** — both rules are aggregate invariants. M1 reads `kat_total`; M2 reads the treasury balance. Neither has a per-account term |
| Shrinking the base to raise the rate | Self-defeating: smaller `kat_total` ⇒ lower `kat_cap` ⇒ the attacker's own payout is cut. Gaming pushes against the attacker |
| Inflating the base to dodge the cap | Self-defeating: dilutes the attacker's own pro-rata share |
| Flash-stake | Already dead via the snapshot+age rule (`staked_at + lockup ≤ boundary(E−1)`, `staking.rs:1293`). M1 reuses that exact eligible set — **no new eligibility surface** |
| Integer overflow | `u128` intermediates throughout, matching the existing `i128` discipline at `staking.rs:1276` |
| Determinism / replay | Integer-only, no floats, `BTreeMap` iteration order preserved. Bit-identical replay is gate G10 |
| Conservation | M1 never mints and never creates a sink — excess is *undrawn*, so `pool_paid ≤ pool_available` is preserved by construction rather than by a new check |
| Agent self-dealing | `treasury_grant_floor` is owner-only precisely because it guards the agent's own stipend |
| Row-format confusion | `u32::MAX` sentinel, not `0` — a v1 row can never be misread as "cap everything to zero" |

## GOAT gate (G10–G16 — numbered to avoid collision with 008's G1–G9-GOV)

| Gate | Criterion |
|---|---|
| **G10** correctness | Capped payouts sum exactly; `pool_paid ≤ pool_available` holds; replay is bit-identical over 10⁴ epochs |
| **G11** sybil-invariance | Splitting any account into N produces a **bit-identical** aggregate outcome. Asserted mechanically, not argued in prose |
| **G12** perf / alloc | Zero added allocation in `pay_due_staking`; added work is O(1) per epoch; benched against the pre-change baseline |
| **G13** back-compat | v1 16-byte rows decode with the ceiling disabled; **every existing pinned test passes unmodified** (`kat_governance.rs`, `dapps_contract_pins.rs`) |
| **G14** mint pressure | With M2, cumulative `mint_demand` is **strictly ≤** baseline at every epoch; the reserve floor is never breached; stipend runway preserved |
| **G15** saturation pin | Genesis `yield_ceiling_bp == YIELD_CEILING.max_bp`, and the agent cannot reach any looser value. Pin **verified to fire**, per the Issue 006 discipline |
| **G16** simulator | riir-dao sim, 10⁴ epochs × 5 seeds vs frozen baseline: thin-base exploit blocked; staker participation **not** reduced beyond tolerance; treasury-funded grant fraction ↑ |

**G16 is the ship gate. No ledger change lands before it passes** — the simulator-first discipline 008 established, applied to its own re-open path.

*Not UQ-bearing:* neither rule claims a distribution, interval, or calibrated confidence, so the "Report the Floor" conformal-floor requirement does not apply. Stated explicitly so the next re-gate does not have to re-derive it.

## The honest limit

**This proposal does not address axis-E.** Bench 004's finding — no agent-reachable `decay_factor` keeps mining alive to a 1 000-epoch horizon at launch scale under any cap — is untouched here and remains the larger open economic risk. Closing it is a cap/initial-supply decision: an owner act, a different plan.

One nuance recorded so the synergy is not overclaimed: M1's undrawn excess stays as mining pool room, but Bench 004 measured mining death to be **schedule-bound, not pool-bound** (a cap raise changed nothing at decay 9 900). So M1 **does not revive mining** — it only stops the staking slice from leaking pool room that mining could otherwise use. That is a real but modest effect, and it is not a fix for axis-E.

## What ships now vs deferred

**Ships now (phased below):** M1 rule + bound + row v2 migration; M2 rule + floor; the pins; the riir-dao sim arm; G10–G16.

**Deferred — gated:** USDC-class emission bound (needs an on-ledger price the never-lever list forbids); grant eligibility hardening (separate concern); making `yield_ceiling_bp` agent-movable *upward* (a bound widening — owner act, never this proposal).

**Explicitly NOT shipped:** any change to agent autonomy (advisory-only stands), any change to the free grant's amount or eligibility, any new sink or mint path, any per-account rule of any kind.

## Phased rollout

### Phase 0 — simulator arm (riir-dao, no ledger change) — **COMPLETE, G16 PASS** (`Bench 003`, 2026-08-29)
- [x] T0.1 Model the thin-base exploit in the riir-dao sim; **reproduce it against today's rules** — reproduced at 38–42 M bp/epoch on the eligible base, early birds 717×–1 813× principal, 5/5 seeds
- [x] T0.2 Implement M1 as a sim-side rule; run G16 (10⁴ epochs × 5 seeds vs frozen baseline) — PASS: blocked at exactly 22 bp (cap bound 156–180 epochs/seed, verified to fire); participation delta **0 bp** under the pinned 1 000 bp tolerance; replay bit-identical
- [x] T0.3 Sim M2: cumulative `mint_demand` and treasury trajectory vs baseline (G14 shape) — main regime 100 % treasury-funded (zero fallback mint); stress regime (3× inflow drain) 34 % treasury-funded with the fallback firing every epoch and the floor never breached

### Phase 1 — row v2 migration (riir-dapps) — **G16 passed → LANDED (riir-dapps `c27300b`)**
- [x] T1.1 `EpochParams` → **28 bytes** (three appended fields — T1.2's two levers ride the same migration, not 20), dual-length decode, `u32::MAX` sentinels, `valid()` unchanged (G13); the `kat:params` integrity scan accepts 16-or-28 B rows
- [x] T1.2 The two PENDING levers landed in the same migration — `usdc_kat_sub_split_bp` (genesis 7 000 = `StakingParams::DEFAULT`, pinned) + `decay_factor_bp` (**genesis = the DISABLED sentinel, deliberately: the no-row fallback must track `KatConfig::reward_decay_bp` — a baked 9 900 silently overrode decay-10 000 deployments, caught live by `tests/kat_mining.rs` and root-caused via a pristine-HEAD worktree A/B**)
- [x] T1.3 `LeverBound::YIELD_CEILING { 9, 22, Δ2 }` + the three new `bounds_violation`/`delta_violation` checks (sentinel-refusing proposals; a sentinel reference gets the free first move — the staking 0→500 bootstrap)
- [x] T1.4 Saturation pin verified to fire (`yield_ceiling_genesis_saturates_lever_safe_end` + the v2 back-compat pin, `tests/kat_governance.rs` 29/29); `GovernanceIntent` binding widened to 28 B (GOVAPPLIED row 113→125 B). **The dao-side mirror (dao `params.rs` + `PARAMS_HASH_DOMAIN` v1→v2 + the dao pin) is DEFERRED to the dao-side session** — the dapps-side landing kept the no-interference line on riir-dao

### Phase 2 — M1 in the ledger — **LANDED**
- [x] T2.1 The cap in `pay_due_staking` (after the `kat_total` fold, before the payout loop); excess undrawn + honestly counted in `StakingPayoutReport::ceiling_deferred_kat`; the sub-split now reads the epoch row's lever (v1 sentinel → the frozen `StakingParams` value)
- [x] T2.2 G10/G11 — `tests/kat_staking_yield_ceiling.rs` 6/6 (incl. T0.1's LEDGER twin: the uncapped control pays the whole class slice to a lone tranche)
- [x] T2.3 G12 — O(1) integer cap on the already-computed `kat_total`; zero added allocation (by construction; epoch-cadence path, not hot)

### Phase 3 — M2 in the ledger — **LANDED**
- [x] T3.1 `treasury_grant_floor` in `KatConfig` (genesis `FREE_GRANT_KAT × 10_000` = 1 M KAT runway; never a lever) — the feature-independent core row widened **128 → 136 B** (the 4 reserved bytes cannot hold an i64; the staking/governance widening precedent)
- [x] T3.2 Treasury-first `grant`; G14 `tests/kat_grant_treasury.rs` 7/7 (mint delta 10 M legacy vs **0** funded over the same 10-account window; floor never breached; `granted`/refusal/return byte-identical). Fixture discoveries recorded in the test doc: `mint_demand` is a per-epoch transient (`settle_epoch` flushes it → measure windowed deltas); the treasury's in-fixture depth source is the mining stipend path

### Phase 4 — activation — **LANDED 2026-08-30** (the delegated perf/sec call)
- [x] T4.1 `staking_share_bp` 0 → 500 in `EpochParams::DEFAULT` (carved from mining), the
      ceiling already armed in the SAME default — and the ordering constraint upgraded from
      conventional to ENFORCED: `KatLedger::set_epoch_params` refuses staking > 0 with the
      disabled-ceiling sentinel, so the thin-base window is unwritable by any path. The
      dao-side mirror landed in the same window (`PARAMS_HASH_DOMAIN` v1→v2, the ceiling
      field, `LeverBounds::YIELD_CEILING`, armed `frozen_baseline`; G5/g16 re-ran green on
      the flipped control arm). Record: riir-dapps Plan 014 T4.1/T4.2.

## Risks

- **Row migration touches signed-intent binding.** `EpochParams::to_row_bytes` is the single codec shared by the ledger write, the ledger read, and the `kat_governance` intent binding (`compose.rs:60-65`). A 16→20 byte change is a coordinated change across all three. Mitigated by G13 (every existing pin passes unmodified) and by landing all three pending row fields in one migration instead of three.
- **The ceiling could bind too tightly and suppress staking participation.** That is precisely what G16 measures; a participation regression beyond tolerance fails the gate and the proposal does not ship.
- **M2 shifts a sybil drain from supply to treasury.** Bounded by the floor, and the fallback is today's exact behavior — strictly no worse at any treasury level.
- **Both rules are dormant until staking activates**, so production evidence accrues only post-Phase 4. Recorded rather than papered over; the sim is the only pre-activation evidence available.

## Out of scope

Agent autonomy (008 Phase 4 stays blocked). Axis-E / supply-cap decisions. USDC-class emission bounds. Grant eligibility hardening. Any per-account or vote-driven fund movement — 008's "a vote NEVER moves funds" is unamended.

## References

1. [Proposal 008](008_adaptive_tokenomics_riir_dao.md) — the lever set, the never-lever contract, the safety inversion, and the T1.4 re-open path this proposal enters through
2. [Proposal 007](007_kat_usdc_staking.md) — the staking slice, tranches, snapshot+age eligibility
3. [Proposal 005](005_kat_heal_service_network.md) — the epoch settle, supply discipline, the `kat:params` row seam
4. `riir-dapps/src/kat/staking.rs:1230-1340` — `pay_due_staking`, the M1 insertion point
5. `riir-dapps/src/kat/compose.rs:29-140` — `EpochParams`, its codec, `split_burn_with`
6. `riir-dapps/src/kat/ledger.rs:729-747` — `grant`, the M2 insertion point
7. `riir-dapps/src/kat/governance.rs:100-200` — `LeverBound`, `bounds_violation`, the PENDING rows
8. riir-dapps `Issue 006` — the genesis-saturates-the-safe-end discipline M1 replicates
9. riir-dapps [Bench 004](../../riir-dapps/.benchmarks/004_g7_econ_sim_and_staking_chaos.md) — axis-E, and the measurement that mining death is schedule-bound not pool-bound

## TL;DR

**Two aggregate-only ledger budget rules, sybil-immune by construction, simulator-gated before any ledger change.** M1 closes a live unbounded-yield hole on a thin staked base, with genesis saturated at the lever's safe end so the agent can only ever tighten it. M2 converts an idle treasury into free-tier onboarding runway and removes that mint pressure, with a floor that the agent may not move because it guards the agent's own stipend. Neither touches a never-lever; neither asks for autonomy; neither claims to fix axis-E.

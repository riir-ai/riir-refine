# Proposal 017 — Buffered reward restake: burn-adaptive auto-stake + the two missing surfaces

**Status:** IMPLEMENTED (2026-09-29, owner GO recorded — plan 186 Phase C); default stays OFF (the soak clock + the dormancy conjunct are wall-time/code prerequisites, not approvals). C1–C4 + C5b landed across riir-dapps/riir-kat/riir-clippy; C5 (devnet deploy + soak start) is the remaining step

## TL;DR

Reward restake (Proposal 007 §Reward restaking) ships ledger-side but is **unreachable by any user surface** — and it is missing BOTH halves of its lifecycle: nothing sets the flag (`set_restake` has no route, the CLI has no `--restake`) and nothing redeems a restaked tranche (`redeem_kat_tranches` has no route and no command; `/stake/unstake` covers USDC principal only). Shipping the setter alone would brick restaked rewards. This proposal:

1. **Ships both surfaces** — `POST /stake/restake` + `POST /stake/redeem` (signature-gated, replay-defended) and `cargo heal stake --restake off|buffered` + `cargo heal stake --redeem`.
2. **Adds the `buffered` posture** — auto-stake each epoch reward credit **above a burn-adaptive liquid buffer**: `buffer = max(FLOOR, trailing-7d *requested* burn × 1.25)`, top-up-to-buffer, integer micro only. The buffer share is never tranche-locked.
3. **Encodes the posture in a side row, not the head** — the 45-byte `StakeAccountHead` bool stays 0/1 forever; a rolled-back binary reads a Buffered account as **Off** (the safe direction: liquid, foregone yield — never locked).
4. **Exposes only `off | buffered`** — lock-all (`on`) is *retired before it ever shipped* (the flag is user-unreachable today, so it has zero accounts): routing 100% of income away from the burnable balance widens the free-heal leak (`burn_report` clamps the debit at balance and advances the watermark — unfunded heals are forgiven).
5. **Touches nothing else** — default stays OFF, no new supply channel, no dao lever, D4a untouched, counsel envelope unchanged (same yield-on-KAT class 007 already covers).

Answers the owner's tokenomics question (2026-09-29): "auto-stake if paid from system" — per-account, applies to **every** lane that earns epoch rewards (miner / fixer / trainer ride the single `pay_due_rewards` leg), with the buffer replacing role-keyed constants.

## The problem

1. **The flag is dead code for users — and so is its exit.** `set_restake` (riir-dapps `kat/staking.rs`) is test-only; `redeem_kat_tranches` likewise. 007's rollout T2.2 sketched `stake --restake on|off`; neither it nor a redeem command landed.
2. **Lock-all asks a believer to lock 100% of every payout** for a 7-day clock each — and (found in review) it starves the burnable balance: `burn_report` debits are clamped at balance while the watermark still advances, so a lock-all miner's heals get served unpaid forever while principal compounds. The never-brick rule (005) plus this leak are why the answer is a *buffer*, and why lock-all is not exposed at all.
3. **Idle reward capital.** Non-believers' rewards sit liquid and earn nothing; a middle posture captures compounding without the brick risk or the leak.

## Design

### The two surfaces (the actually-missing pieces)

- **`POST /stake/restake`** — body `{"posture":"off"|"buffered", "seq":<u64>, ...}`, signed by the **account key** — the verified arm (`/stake/unstake` accepts both an account-key signature and a session token, `contract.rs:3280–3317`; 017 uses the account-key arm, which is what miners without a funding wallet have). **Replay defense is mandatory because a posture is a toggle:** `/burn` is replay-safe only because its watermark is monotone; a captured signed `buffered` replayed after the user goes `off` would force re-locking. The signed message is prefix-scoped per route (`kat:stake:restake:v1:` ‖ body) and carries a **per-account monotone operation sequence** (`kat:stakeopseq:<16B acct>` u64 side row, shared by posture and redeem); the route refuses `seq ≤ current` (409 `already_settled`-class) and advances it in the same `CommitBatch`. Error codes per the money-route table: unknown posture `bad_field`, bad signature `bad_signature`, stale seq `already_settled`.
- **`POST /stake/redeem`** — drains all unlocked KAT tranches to the liquid balance (`redeem_kat_tranches`'s existing semantics; instant, pure-ledger, no custody). Account-key signed with the **same op-sequence defense** and its own prefix (`kat:stake:redeem:v1:` ‖ body). "Naturally idempotent" is **rejected** as a design: it is only true immediately — a captured replay would force-redeem every tranche that unlocks *later*, stripping yield the user chose to stake (griefing, not theft — still refused). CLI: `cargo heal stake --redeem`. ⚠ The existing `unstake_message` is a **static signature** (`b"kat:stake:unstake:v1:" ‖ pubkey`, riir-kat `kat_protocol_stake.rs`) replayable forever — a pre-existing defect outside 017's scope, filed as riir-dapps Issue 106; 017's routes do not copy that shape.
- **`/stake/info` returns the current op sequence** — the CLI needs it to sign the next posture/redeem request; and because one op sequence is shared, a posture and a redeem sent from two machines at once race (one gets 409): the CLI **re-fetches `/stake/info` and retries once** instead of surfacing a raw `already_settled` on the first attempt.
- **No ambient env var.** Money-adjacent behavior changes by explicit command only. This deliberately does NOT reuse the `--stats on|off` consent-plane pattern: restake posture is **account state with server truth**, not machine-local consent.

### Posture encoding — side row, never the head byte

`StakeAccountHead` (45 B, exact-width decode) is **frozen at bool**; the buffered mode lives in a side row:

| Posture | head.restake (byte 32) | `kat:restakemode:<acct>` |
|---|---|---|
| Off (default) | 0 | absent |
| Buffered | **0** | `1` |
| LockAll (legacy decode only, unreachable by 017 surfaces) | 1 | ignored |

- **Rollback safety is the design driver:** a pre-017 binary reads a Buffered account's head as `restake = false` → **Off** → rewards stay liquid. The failure direction is foregone yield, never an unintended lock. (An in-head u8 `{0,1,2}` encoding was considered and rejected in review: only 017 code writes byte 2, so the real exposure is a *rollback* — and every head write by a rolled-back binary (the restake hook, `set_restake`, stake requests) re-encodes 2 → 1, permanently converting Buffered to LockAll. That is the never-brick harm, not a cosmetic transient.)
- The head byte therefore only ever carries 0/1 — the "unknown posture byte" whole-DO decode hazard never arises. The mode row is **not** added to `validate_rows` (no exact-width boot gate); it is read at posture-resolution time, an invalid value degrades to the head-bool path, and `stake --info` renders the **resolved** posture so the user always sees actual behavior.
- `set_restake(on: bool)` stays as a wrapper over a new `set_posture`; existing callers/tests do not churn.

### Buffered semantics — the payout-leg hook

`restake_credit_into_tranche` (the only caller is `pay_due_rewards`) splits under `Buffered`:

```text
liquid_available = acct_balance + locked_balance(account)        # KAT only
buffer_needed    = max(FLOOR_LIQUID_MICRO,
                       requested_burn_trailing(LOCKUP_TICKS) × (1_000 + HEADROOM_MILLI) / 1_000)
staked           = amount − min(amount, max(0, buffer_needed − liquid_available))
if staked < MIN_RESTAKE_MICRO → all liquid (dust floor; no tranche row)
```

- **`requested_burn_trailing` = total demand, not applied burn, not KAT-only.** `BurnEvent.applied_micro` is recorded *after* the balance clamp — a heavy user whose burns were clamped at balance 0 would show almost no history and their buffer would collapse to the floor, permanently. The trailing sum is therefore the **cumulative burn watermark delta**: `latest total_kat − total_kat at the first event older than the window`, found by **binary search over seq by tick** (O(log n) point reads — a capped newest-first scan would silently under-count exactly the heavy miners this buffer exists for). The delta is used **as-is, TUNA-paid heals included**: the watermark counts a heal's full size regardless of which currency paid, `BurnEvent` does not record the TUNA part per event (a KAT-only burn figure is not even computable), and counting total demand is the conservative direction for a buffer. Convert whole-KAT → micro (×10⁶). No burn history → buffer = `FLOOR_LIQUID_MICRO` (a brand-new contributor still gets the never-brick floor). TUNA's exclusion belongs on the *availability* side, not the demand side.
- **`liquid_available` = the whole locked KAT balance, one row read — KAT only.** Under `kat_mint_nft`, the non-restaked share of a payout goes to `kat:locked:` (the D4a vest clock), not `kat:acct:` — sizing against the spendable row alone would recompute the same shortfall every epoch and quietly disable buffering. The term is deliberately NOT a releasing-within-window scan (receipt keys are epoch-prefixed, so a per-account receipt scan is O(all receipts)): `VEST_DAYS_MAX = 7` equals the default `LOCKUP_TICKS` (604 800 = 7 d), so every vest-locked KAT releases within the window **by construction**. A config-install refusal fires if `vest_ticks > lockup_ticks` (the over-count direction). TUNA never counts as *available* KAT — it expires. Disclosed optimism: a receipt stuck unfinalized can hold locked KAT past its vest clock, so counting the locked balance as available is slightly optimistic in exactly that case (caveat 9).
- **Never-brick claim, stated per clock:** the **buffer share is never tranche-locked** (7-day stake clock). The liquid remainder still rides the receipt **vest clock** (1–7 d, `KAT_MINT_VEST_DAYS` — D4a owner config) — the same clock every non-restaking user's rewards already ride. One amount never carries two unlock clocks (Plan 020): the **split** gives each share exactly one — staked → tranche clock, remainder → vest clock.
- **Receipt split accounting (the Plan-020 half) — ONE split function, not a composition.** The per-receipt locked part is `amount_final − released_micro` (`locked_part()`); `locked_micro` is the global `MintMeta` counter, not a receipt field. Composing today's two functions (`settle_restake_stamp(staked)` then `settle_apply(remainder)`) is **refused by design, two ways**: both add `rows_resolved` (double count), and stamping the staked share first finds `locked_part() == 0`, so the receipt lands **Minted** — and `release_due_receipts` only picks up `Finalized`, stranding the remainder forever. The split is one function: `rows_resolved += n` added **once** (n = the receipt rows this payout resolves for the (epoch, account) — the existing settle functions add the per-receipt row count, not 1; a literal `+1` would never reach `rows_total`, the receipt would never finalize, and the remainder would strand — the exact failure this function exists to prevent) and `amount_final += amount` once; `released += staked`; `kat:locked: += remainder`; `MintMeta` minted/released/locked updated so `conserved()` holds; the receipt state decided **after** both shares applied — `remainder > 0 ⇒ Finalized`, never Minted. Golden rows pin: full restake (remainder 0 → Minted, today's shape, unchanged), partial (remainder > 0 → Finalized), and **multi-row** (a receipt spanning ≥ 2 rows → Finalized with `rows_total` reached).
- **Top-up-to-buffer, not refill-every-payout:** only what the burn history says was consumed is kept back; the rest compounds. A liquid-rich account restakes everything.
- **Constants are code consts:** `FLOOR_LIQUID_MICRO` (calibrated plan-time from the measured charge ledger — one typical heal run's burn; a measurement, not a guess), `HEADROOM_MILLI = 250` (25 %), `MIN_RESTAKE_MICRO` (dust floor — buffered otherwise mints one tiny tranche row per epoch). **Not `KatConfig` fields** (exact-width decode fingerprint — the `MintMeta` widening outage lesson) and **not dao levers** (per-account UX, not economy-level; the `NEVER_LEVERS` bookkeeping stays untouched).
- Integer micro arithmetic only (`u64`/`i64`) — f32/f64 in a money path is review-blocking (G4-S, inherited).

### Why no per-role buffer config

The trailing burn rate **is** the role signal (a heavy scanning miner carries a large buffer; a light-usage fixer a small one; the internal trainer lane ~floor). Role constants would be a second config surface with no added safety, prone to staleness. And the buffer has **zero gaming surface**: it is the user's own foregone yield — inflating one's burn history to grow one's buffer costs the inflator compounding.

### What explicitly does not change

- Default posture **OFF**; 017 surfaces accept only `off | buffered` (lock-all is retired — no surface ever exposed it; the zero-accounts claim is **verified at deploy time, not asserted**: count heads whose byte 32 is 1 before the 017 deploy and record the number).
- The vest window (D4a, owner deployment config), the `NEVER_LEVERS` contract, the dao `Lever` set, the pool split, the yield ceiling, the authority decay schedule: untouched. Buffer constants are not levers.
- **No new supply channel**: the staked slice remains a partition of the burn pool (007 invariant). 100 % buffered opt-in cannot spiral — partition-bounded, asserted in the simulator (G7-S).
- USDC purchase mints (`record_mint_demand`) still never restake — purchases are not earnings.
- Staking yield (`claim_accrued`) still claims to the liquid balance — compounding the *yield* is phase 2 (owner call, same counsel envelope).
- The explorer's `restake` field gains the posture spelling (read-only; already renders today).

## Honest caveats — READ BEFORE IMPLEMENTING

1. **A buffer is foregone yield.** The `--info` copy says so plainly ("buffer KAT stays liquid instead of compounding"). The user trades liquidity for growth; nothing hides it.
2. **The free-heal leak bounds what restake may safely lock.** Burn debits are clamped at balance and the watermark advances regardless — unfunded usage is forgiven *forever* (the watermark is the replay defense, so it can never be un-advanced). Buffered keeps the buffer spendable precisely so ordinary usage stays paid; this is also why lock-all is not exposed. A bench arm compares forgiven-burn volume under the postures at heavy burn (G7-S).
3. **Restake makes dead accounts *more* expensive, not less.** A restaked abandoned account keeps drawing its share of the staking slice forever at live stakers' expense. The owner's dead/lost-wallet concern is therefore **not** answered by auto-stake — the dormant-stake question (a dormancy sweep is a confiscation decision, owner-gated) must be decided **before** any default-ON flip (§Phases).
4. **Counsel gates more than the default.** Restake is yield-on-KAT — 007 caveat #8's class, inside 007's single counsel review. Two additional counsel-touching surfaces: (a) any **default-ON** flip breaks the "default experience promises nothing" hygiene, so a green soak alone is not the gate — counsel sign-off is a conjunct; (b) the **first-payout nudge** toward `buffered` is promotion of a yield product — it ships **devnet-only** until counsel signs.
5. **Rollback direction is safe by construction** (side-row design): a rolled-back binary reads Buffered as Off — liquid, foregone yield, never locked. The reverse (an 017 binary reading legacy rows) is the 007 decode, unchanged.
6. **Burn-window lookup is O(log n)** (watermark delta via seq/tick binary search) — bounded even for the heaviest accounts, unlike a capped scan. The payout leg pays it once per Buffered account per epoch settle.
7. **FLOOR calibration is a measurement:** plan-time read of the charge ledger. A mis-calibrated floor mis-sizes a user's own liquidity — it cannot break a ledger invariant (conservation is tranche-vs-credit, independent of the split).
8. **Formula-bug blast radius is one user, one payout:** over-restake is bounded by the dust floor + buffer assertion (test-asserted); under-restake costs only yield. Neither touches pool accounting (007's "identical either way" property).
9. **`liquid_available` is slightly optimistic on stuck receipts:** a receipt stuck unfinalized can hold locked KAT past its vest clock, and the formula counts the whole locked balance as within-window available. The stuck population is visible via the release leg's own counters, so the soak readout can size the optimism instead of guessing.
10. **Scale reality (live devnet, 2026-09-29) — the economy is treasury-subsidized and emission-capped, not burn-circulating.** Lifetime: 119.70 KAT emitted against a 100,000 KAT `reward_max_emission` cap; 171 KAT burned; staking slice UNEXERCISED (0 staked, 0 accrued — 007 caveat 5 confirmed live: at fleet scale the staking yield is exactly zero, and the CLI's honest-zero copy is load-bearing). Per-epoch burn telemetry reads 0 because the burn-bucket reader is pending (Plan 036 P1) — 017's `requested_burn_trailing` is unaffected (per-account `kat:burnevt:` watermark, not the buckets), but the **G7-S run and the soak verdict both depend on that reader landing first**; a soak read during the reader gap would measure a fake zero. Extrapolated to 1,000 m3-profile users (~30 KAT earned / ~5 KAT burned per user-epoch): the emission cap binds within ~3 epochs and rewards flip to burn-funded at ~10× lower per-user (mining slice of a 5,000 KAT burn ≈ 2.9 KAT/user vs ~30 treasury-funded); the staking slice becomes ~225 KAT/epoch, where the 22 bp/epoch yield ceiling sits within ±10 % of the available slice at realistic stake levels — the thin-base guard is correctly sized at that scale. Two soak counters are therefore mandatory before any default-ON read: **emission-cap headroom** (`reward_emitted` vs `reward_max_emission` — the default-ON decision must not land during the treasury→burn transition) and the **burn-reader liveness** marker. The first real FLOOR calibration datum is also on record: fleet lifetime burn 171 KAT (m3 alone 21 KAT).

## Fusion lineage

1. **Proposal 007** §Reward restaking — this extends it additively: the buffered posture + the setter/redeem surfaces 007's rollout sketched but never landed.
2. `restake_credit_into_tranche` / `set_restake` / `redeem_kat_tranches` (riir-dapps `src/kat/staking.rs`) — the exact hook points; `pay_due_rewards` restake arm + `settle_restake_stamp` (Plan 020 precedence) carry over with the split accounting added.
3. **The never-brick rule** (005) — the buffer is its mechanical answer at opt-in scale; the burn-clamp leak is its boundary condition.
4. **`KAT_MINT_VEST_DAYS`** — the refuse-not-clamp deployment-config pattern, cited for any future env knob; v1 ships code consts instead.
5. **The consent planes** (`--stats on|off`) — deliberately *not* reused; posture is account state, not machine consent.
6. **The `/burn` watermark** — the monotone-counter replay-defense pattern this proposal generalizes to the posture toggle (posture seq).
7. riir-chain `StakeProgram` lockup semantics — cited by 007, unchanged here.

## GOAT gate (extends 007's G*-S set; the healer's offline gates untouched)

- **G1-S (money e2e):** posture `buffered` → payout: `need ≥ amount` → all liquid, head unchanged, no tranche row; `need < amount` → exact split, `liquid_available + (amount − staked) ≥ FLOOR` asserted; dust → all liquid, no tranche row; **redeem round trip** (restake → unlock clock advances → redeem → balance credited, stale-seq redeem refused); **`kat_mint_nft` × buffered golden rows via the ONE split function** (`released += staked`, `kat:locked: += remainder`, `rows_resolved += n` once, `MintMeta.conserved()` holds; state decided after both shares — `remainder > 0 ⇒ Finalized`, never Minted; incl. a **multi-row receipt** arm); `off` accounts byte-identical to today (golden).
- **G8-S (adversarial):** forged `/stake/restake` signature → 401; **stale/replayed op-seq → refused on BOTH routes** (the toggle-replay arm: capture signed `buffered`, replay after `off`, assert refusal; the same arm for redeem — the force-redeem-future-tranches grief is unreachable); unknown posture string → 400 `bad_field`; zero-burn history → FLOOR; **burn-clamp comparison** (requested vs applied burn fixtures — a clamped-at-zero user's buffer must NOT collapse); **multi-epoch catch-up** (`pay_due_rewards` settling several epochs in one crank — `liquid_available` freshness across per-epoch batches); **rollback arm** (decode a Buffered account with the legacy head decode → Off; re-encode → posture survives the side row); LockAll legacy decode still resolves (unreachable but decodable); **`vest_ticks > lockup_ticks` config-install refusal**; integer-only math asserted by construction.
- **G7-S (economics):** simulator run with 100 % buffered opt-in — pool never overdrawn, compounding partition-bounded (no spiral), thin-base yield ceiling still binds, authority schedule unaffected; **forgiven-burn volume** under buffered vs lock-all at heavy burn (the leak quantified); **dormant-restaked-share** counter feeding the dormancy decision.
- **G3-S (no regression):** `kat_staking` off → riir-dapps default build + riir-clippy default binary bit-identical.
- **G2-S (perf/placement):** zero per-op chain writes preserved; the burn-watermark delta lookup is **O(log n)** and `locked_balance` is a single row read (never a receipt-prefix scan), with the per-account cost printed in the payout report.
- **G4-S (typing):** micro-u64/i64 end-to-end; posture resolution is an exhaustive match over (mode row, head byte).

## Ships now vs deferred

**Ships now:** posture side row + buffered split + ONE receipt-split function + op-seq row + the `vest_ticks ≤ lockup_ticks` install refusal (riir-dapps) · `/stake/restake` + `/stake/redeem` routes (op-seq replay defense, per-route message prefixes) + error codes + contract tests (worker) · CLI `stake --restake off|buffered` + `stake --redeem` + `--info` render (resolved posture, buffer, projected split, current op seq — no APY, no $ figures, caveat-#15 rule) + wire client rows (riir-clippy + riir-kat) · explorer posture spelling · deploy-time zero-lock-all count · soak telemetry counters (posture counts, split sums, redemption waits, post-restake liquid ≈ 0 incidents, forgiven-burn volume, dormant-restaked share, stuck-receipt population, **emission-cap headroom**, **burn-reader liveness**).

**Deferred — gated:** default-ON flip — **requires ALL of**: green soak of **≥ 2 × `LOCKUP_TICKS`** (one epoch cannot observe a full lockup + vest cycle), a **dormancy decision** for restaked abandoned accounts (caveat 3), and **counsel sign-off** (caveat 4 — the soak alone is not the gate). Also deferred: the first-payout nudge (devnet-only until counsel — caveat 4b); `claim_accrued` yield compounding (phase 2, same counsel envelope, owner call); per-account buffer override (`--buffer <N>`) only if real users ask (v1 formula is the only posture).

## Phased rollout (sketch — a plan would expand this)

0. Measure: charge-ledger read → `FLOOR_LIQUID_MICRO` calibration (a number with its box state, the G2 box-state rule).
1. Ledger: posture side row + buffered split + ONE receipt-split function + op-seq row + the vest/lockup install refusal + tests (riir-dapps, `kat_staking`).
2. Worker: `/stake/restake` + `/stake/redeem` + contract pins (riir-dapps `cloudflare/kat-service`).
3. Client: CLI flag + redeem + info render (riir-clippy; riir-kat wire rows).
4. Devnet deploy + soak (≥ 2 × lockup, counters above) → **owner verdict** on the default-ON question with the dormancy + counsel conjuncts. Mainnet posture: same counsel gate as 007 — nothing user-facing before counsel signs.

## Risks

- **Posture-resolution bug = mis-locked rewards.** Mitigation: golden fixtures for all resolved postures + the side-row rollback arm + `--info` rendering the resolved posture.
- **Buffer formula bug.** Bounded per caveat 8; the dust floor + buffer assertion are the hard floors in tests.
- **Replay toggle attack.** Mitigated structurally (monotone posture seq); the G8-S arm proves it.
- **Feature creep toward "auto-optimizing users' money."** This proposal stops at one posture byte, one formula, and one redeem command. Anything that moves money on the user's behalf beyond an explicit account flag (auto-unstake, auto-claim, auto-redeem) is out of scope and needs its own owner-gated proposal.

## Out of scope

Any USDC-side change · any dao lever or telemetry-schema change beyond soak counters · vest timing (D4a) · transferability · **stake-weighted work assignment** (the fixer/trainer "assign ratio" idea is refused: queue priority is content-value `miss_freq × rule_gap` and stake-corrupting it degrades the corpus the economy pays for; throughput-side weighting, if ever, is its own proposal) · **a miner burn rebate / "tax discount"** (refused: no transfer tax exists — KAT is non-transferable by contract, a NEVER_LEVER — so the only thing to discount is the usage burn itself, which would be a new dao lever whose benefit drains the very pool that funds the staking yield; circular. The existing staking slice + RewardVariance variance-shaping are the honest stake bonus) · **dormancy sweeps** (a confiscation decision; owner-gated, and a *precondition* for any default-ON flip — caveat 3, not a feature here) · yield-on-yield compounding (phase 2) · **the pre-existing static `unstake_message` replay defect** (riir-dapps Issue 106 — outside 017's scope; 017's own routes carry the op-seq defense from day one).

## References

- In-house: `riir-clippy/.proposals/007_kat_usdc_staking.md` (parent), `005_kat_heal_service_network.md` (never-brick, caveat #15), `008_adaptive_tokenomics_riir_dao.md` (levers/never-levers) · `riir-dapps/src/kat/{staking,burn,ledger,mint_receipt,crank}.rs` · `riir-dapps/cloudflare/kat-service/src/{routes,contract}.rs` (`/stake/*` family) · `riir-clippy/src/bin/heal.rs` (`run_kat_stake`/`run_kat_unstake`/`run_stake_info`) · `riir-dao/crates/riir-dao/src/strategy.rs` (`NEVER_LEVERS`, disjointness tests).
- Prior art: Beanstalk snapshot-before-payout (via 007); exchange withdrawal-address discipline (007 caveat #7) for why posture changes are explicit commands; the `/burn` monotone watermark as the replay-defense pattern generalized to toggles.
- Review: Claude verdict round 1 (2026-09-29) — REVISE; blockers addressed in r2 (redeem surface, vest-lock accounting, Plan-020 receipt split, requested-vs-applied burn, replay defense, side-row rollback safety, lock-all retirement, dust floor, soak length, counsel+nudge gating, dormancy conjunct). Round 2 — REVISE (4 defects); addressed in r3: ONE receipt-split function, not a composition (`remainder > 0 ⇒ Finalized`, never Minted); burn sized from total demand (TUNA exclusion moved to `liquid_available`, where it is computable and conservative); redeem carries the op-seq defense with its own message prefix (the pre-existing static `unstake_message` replay filed as riir-dapps Issue 106); `locked_balance` single-row form + the `vest_ticks ≤ lockup_ticks` install refusal + the stuck-receipt optimism caveat; `/stake/info` returns the op sequence; the zero-lock-all claim is deploy-time counted, not asserted; the auth arm verified (account-key signature — unstake accepts both arms).

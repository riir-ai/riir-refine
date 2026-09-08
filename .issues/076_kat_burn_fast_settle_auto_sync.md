# Issue 076 — KAT burn fast-settle: per-run auto-sync + hourly burn bucket (decoupled from the 7-day staking epoch)

**Status:** OPEN — design recorded 2026-09-08 (owner decisions in 075 round 2); not started

## Why

075's cadence table mapped the burn settle leg onto the 7-day staking
epoch. The owner called it: the 7 days is the staking lockup/yield epoch
(`lockup_ticks: 604_800`, "the proposal's 7 days" — flash-farming
defense — plus the ~48 h unstake escrow and the per-7-day yield
ceiling), never a burn-settlement requirement. Waiting 7 days to see a
heal run's burn committed is a pigeon-grade UX and was an implementation
accident, not economics.

## The three decisions (owner, 2026-09-08)

1. **Sync the burn debt right after the run finishes** — per-run,
   fail-soft, background. Burn reporting is account accounting (rides
   the login); the unheal telemetry batch remains the consent-gated
   part and still waits for explicit `sync`.
2. **The service ledger commits burn hourly** — its own bucket,
   decoupled from `StakingParams.ticks_per_epoch`.
3. **Staking stays 7-day** — lockup, escrow/venting, yield ceiling,
   flash-vote defense are the anti-farm mechanics and the ONLY 7-day
   waits in the system.

## Tasks

- [ ] Client (riir-clippy): auto-sync post-run — after `KatCharge::settle`, fire the burn-debt push in the background (fail-soft, offline-safe; a failed push retries at the next run or explicit `sync`, never blocks or delays the run's exit). Burn-debt payload ONLY — the unheal outbox still waits for explicit `sync` per the consent contract
- [ ] Client (riir-clippy): the `kat-charge` summary line shows the last-synced server balance when fresher than the projection (075's soft gate reads THIS, never the projection)
- [ ] Service (riir-dapps `kat-service`): burn-debt ingest endpoint — idempotent per (account, meter watermark segment), replay-safe, exactly-once credit to the burn liability
- [ ] Ledger (riir-dapps `kat`): hourly burn-settle bucket — new bucket key space, settle runs per bucket on the existing hourly alarm; `BurnFrom(liability)` → pool/staking/treasury/true-burn composition per bucket, burn-first ordering preserved (crash-mid-bucket = under-supply, never inflation)
- [ ] Decouple check: grep every `Epoch::from_chain_height` / settle-leg caller and classify STAKING (keeps `ticks_per_epoch`) vs BURN (moves to the hourly bucket) — the split must be by consumer, not by editing the shared epoch counter
- [ ] Cross-repo pin: riir-dao `tests/dapps_contract_pins.rs` hard-codes the reward schedule — re-derive per bucket or amend the pin in the same commit; run the dao gate after
- [ ] Staking untouched: a test pinning `lockup_ticks = 604_800`, `escrow_ticks = 172_800`, vent windows, and the per-7-day yield ceiling against accidental change
- [ ] Chain anchor stays batched (anchor-leg cadence unchanged; Glacial applies to chain/quorum writes, never to service-ledger internal buckets)
- [ ] GOAT gate: (G2) hourly settle-leg cost bench — composition per bucket must stay O(burn rows in bucket), never O(history); (G1/G3) chaos/replay soak — a replayed alarm pass moves zero, a crash mid-bucket leaves under-supply never inflation
- [ ] Feature flag `kat_fast_settle` (opt-in) until the GOAT gate passes; promote to default only on G1–G4 pass

## Honest caveats

- Nothing of the sync→debt path is landed today (T0.3+ in 008); the
  cadence is designable precisely because of that — this is wiring, not
  migration.
- The soft gate (075) depends on this issue's client + server legs; the
  gate lands after, reading server truth.
- Hourly buckets multiply composition events 168× vs the weekly epoch.
  Per-event cost is trivial (pure ledger math over a WAL) but the bench
  must prove it — that is the GOAT gate's job, not an assumption.
- The 7-day figure was never "the chain being slow" — the L1 chain was
  never in the per-run path at all. Do not re-litigate chain latency
  here; the chain only sees batched anchors.

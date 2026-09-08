# Issue 076 — KAT burn fast-settle: per-run auto-sync + hourly burn bucket (decoupled from the 7-day staking epoch)

**Status:** CORE LANDED + E2E VERIFIED LIVE 2026-09-08 — burn + contribution both move on ai.gist.rs; open: G2 bench, soft gate, release feature set, riir-dao pin, grant-units (riir-dapps 025)

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

- [x] Client (riir-clippy): auto-sync post-run — `f3a7cf2`: `src/kat_burn_client.rs` + the hook after the charge block in `main()` + the same push atop `run_kat_sync`; fail-soft ~5 s budget, never an exit-code change, anonymous runs never dial, billing ≠ consent (the unheal outbox still waits for explicit `sync`)
- [x] Client (riir-clippy): fresh server balance — carried on the push response (`burn sync: N KAT reported (server balance …)`); no GET when nothing to send
- [x] Service (riir-dapps `kat-service`): burn-debt ingest — `a2648c6`: `POST /burn` (route-table contract pinned), per-account watermark `kat:burnwm:<acct>` monotone max + warn-and-pay debit in ONE CommitBatch, replay = 200 no-op
- [x] Ledger (riir-dapps `kat`): hourly burn-settle bucket — `settle_burn_bucket` under `kat:burnsettle:<bucket>`, distinct `riir-dapps.kat.burnsettle.v1` intent domain, burn-first, pool backing applied once on receipts, zero buckets complete in place; `KAT_FAST_SETTLE` flag, `CrankRunRow` v3 lenient widening; chaos/conservation green
- [x] Decouple check — split by consumer: staking callers keep `ticks_per_epoch` untouched; only the burn bucket moved; the epoch-shrink coupling is documented in-code (staking funded hourly by each bucket's 5% leg)
- [ ] Cross-repo pin: riir-dao `tests/dapps_contract_pins.rs` — per-epoch `EpochRow.burn_total` reads ≈0 with the flag armed; re-derive or amend the pin (OPEN — riir-dao side)
- [x] Staking untouched: kat_staking suite green post-change (`lockup_ticks`/escrow/vent/yield-ceiling paths unmodified; green as part of `a2648c6` gates)
- [x] Chain anchor stays batched — anchor leg untouched (`anchor_seq` advanced per pass, cadence unchanged)
- [ ] GOAT gate: (G2) hourly settle-leg cost bench; (G1/G3) chaos/replay soak is DONE (shipped in `a2648c6`: replayed pass moves zero, crash-mid-bucket = under-supply) — the G2 bench itself still owed before any default promotion
- [x] Feature flag: `KAT_FAST_SETTLE` env var, fail-off, inert when unset — `13950c5` arms it `"1"` in prod vars (owner mandate: hourly NOW; promotion-to-default question dissolves into keeping the var)

## Added by owner, same day (2026-09-08)

- [x] **Stacking test (the git analogy):** `tests/kat_burn_stack.rs` 3/3 (verified independently by the coordinator): closed port → "stacked" line + ack untouched + exit 0; next run → ONE request carrying the FULL stacked total, signature verified over the documented message, ack advanced; zero-delta run → zero dials. Server-side monotone (replay ≤ stored no-op, higher = increment) pinned in the burn_contract tier
- [x] **Docs task:** README "Fuel model" section + AGENTS.md CLI-UX bullet (riir-clippy, `f3a7cf2`, docs_gate 14/14) + front-page burn-stats copy + auto-report/stacking sentence on the `--mine` tier (riir-dapps, `a2648c6`)
- [x] **Manual deploy:** wrangler 4.129.0 from the workstation, twice (initial `3d9bf574` + corpus refresh `94975071`), both verified via `deployments list` newer-than-T0 + `/health` probe; pushes carried `[skip ci]` — the CI deploy lane (billing-failed ~4 s per run) never relied on
- [x] **E2E on live ai.gist.rs (2026-09-08):** Batch 135 RustForge distill (`84d0d73`, rust_perf 114→117, all three rules measured on M3 Max) + real heal run → `burn sync: 8666 KAT reported` → crank pass shows `"burn_settle":"settled"`, `burn_bucket:496914`, `burn_settled_micro:100000000` → site: **"100 KAT burned to date"** (was 0), **"10 batches · 179 rows contributed"** (was 9/177), **rust_perf 117** on the corpus graph (snapshot refreshed + redeployed, `7e7c0be`)
- [x] Client build constraint (RESOLVED AS A GAP, LOUDLY): `kat_service` alone has NO HTTP client — every network surface including the burn push rides ureq behind `kat_transport`. The shipped release set (`default + latent_retrieval + kat_service`) compiles the push to nothing, exactly as it already does for `sync` + the consent heartbeat. Recorded in module docs + AGENTS bullet. **Follow-up owed:** add `kat_transport` to the release feature set (Plan 107 amendment) or a ureq-free transport

## Honest caveats

- The LIVE tank is 10⁶× smaller than the story: genesis grants 100 KAT
  (`FREE_GRANT_KAT: MicroUnits = 100_000_000`, `riir-dapps/src/kat/mod.rs:262`)
  vs the client projection's + the copy's 100M KAT. Found live when the
  first report clamped at balance. Filed as riir-dapps `.issues/025`
  (owner economics call: raise grant + supply cap, shrink the story, or
  devnet-small + server-derived display). m3 is at balance 0 until then —
  further burn reports clamp to no-ops.
- The soft gate (075's zero-balance decision) is NOT wired yet — now it
  has the server-truth input it was waiting on; needs the depletion
  predicate over `balance_micro` + the OptedIn-grace decision.
- `paid to miners` stays 0 µKAT until the prove leg pays from the pool —
  the pool got its first backing this hour (100 KAT burn → 70%), payouts
  ride the next passes.
- Pre-existing (not ours, reported): 2 wasm32 clippy failures in
  kat-service (identical at pre-change HEAD), 1 `unused_mut` + clippy
  findings in riir-clippy heal.rs outside the new hunks.
- The 7-day figure was never "the chain being slow" — the L1 chain was
  never in the per-run path at all. Do not re-litigate chain latency
  here; the chain only sees batched anchors.

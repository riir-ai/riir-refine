# Plan 079 — the account-claim grant route (T4.4a's onboarding gap)

**Status:** COMPLETE (2026-09-05) — two production surfaces landed + live-proven; the
wet-run reaches steps 1–4 PASS; step 5 is a named harness-design decision (owner).

## The finding

`cargo heal wetrun --dry-run` against a fresh devnet kat-service failed at step 1 with
`WelcomeGate::Withheld { "the service ledger has no free-tier grant row for this
account yet — run cargo heal sync once so the service sees the key" }` — but **no
production path could ever produce the grant**:

- `KatLedger::grant()` (the once-per-key FREE_GRANT_KAT mint, the `granted` flag) was
  called from **contract tests only** — zero production call sites.
- The crank deliberately does NOT call `grant()` (061 T1.7: the flag refuses repeats,
  so it cannot serve a *daily* credit) — and its free-tier leg walks only EXISTING
  `accounts()` rows, which a new account does not have. `note_activity()` was also
  test-only.
- `POST /sync` ingests into the MINING DO — it never touches the ledger DO's account
  rows.
- The payreq settle path preserves `granted: false`.

So the welcome gate required a state (`granted == true`) the deployed service could
not produce, and the CLI hint named a command that did not fix it. The T2.0 session
stopped exactly here (2d6e06a fixed the bool/count wire shape; the gate then withheld
on the missing grant, and the session ended).

**Three more findings followed once the first was fixed** (each found live, in order):
1. **No treasury-funding surface.** The crank and the free grant are treasury-FIRST;
   a freshly stood-up ledger has a ZERO treasury, so every credit skipped with
   `skipped_treasury`. `record_mint_demand` (the designed plug point) had no route.
2. **The wet-run sync-leg wire contract drifted.** The harness parsed `recorded`
   (Plan 066's assumed shape); the service answers
   `{"accepted":true,"rows_new":N,"rows_refreshed":M,"rows_terminal_skipped":K}`
   (the Plan 015 shape its own e2e drives). The contract authority is the service.
3. **Step 5 vs the daily faucet cap is structural.** `faucet_request_create` pins the
   FULL cap (`request_put_with_ttl(.., cap, ..)`), so the top-up draw always consumes
   the whole per-account daily cap and the same-day refund draw is always refused
   ("cap already used") — proven live at cap 10, 20 AND 40 (pin == cap ⇒ one draw
   empties it; no cap value fits two draws). The refund leg's MEANING is a harness
   design decision on the money surface: (a) verify the settled invoice + balance
   instead of a second draw, or (b) ride the crank's free-tier daily grant leg
   (needs the harness to push a burn for the activity marker). Both are owner-gated.

## Landed (with the live proof)

- [x] T1 `claim_message(pubkey)` in `src/kat/payreq.rs` — domain
      `kat:payreq:claim:v1:` ‖ pubkey, mirroring `faucet_message`.
- [x] T2 `POST /account/claim` (ledger DO) — signed-route pattern
      (`payreq_faucet_route` mirror): verify → derive → `ledger.grant` (its
      `AlreadySettled` IS the once-per-key sybil bound) → drain → balance JSON with
      `already` (replay = idempotent 200, not an error).
- [x] T3 dapps tests — `account_claim_mints_the_free_grant_exactly_once` (mint +
      flip + forged-401 + replay-idempotent + empty-treasury mint-demand posture),
      `account_claim_survives_a_cold_reboot` (drain → reopen → replay), the claim
      message domain-separation test + the cross-repo golden digest.
- [x] T4 `POST /admin/treasury/fund` (the second surface) — ops-bearer (the CRANK
      token, fail-closed 503) → `record_mint_demand(&event, &treasury, amount, now)`
      (exactly-once per `event_id_hex`, the Tier-2 rails' designed plug point) →
      drain. Contract test covers 401/503/exactly-once/non-positive/second-event.
      NOT an unattended mint: one bearer-gated ops action per event id (caveat #2).
- [x] T5 `KAT_FAUCET_DAILY_CAP_KAT` dev knob (`dev_config(env)`, the
      `KAT_CRANK_ALARM_SECS` posture) — deployment-scoped faucet-cap sizing;
      unset/garbage/0 = genesis default. (Useful for rehearsal sizing; does NOT
      resolve finding 3 — pin == cap regardless.)
- [x] T6 clippy client — `claim_message`/`claim_request_body`/`ClaimResponse`/
      `parse_claim_response`/`ensure_grant_flow` (read-first: a granted account
      costs ZERO posts) in `kat_payreq_client.rs`; `get_balance`/`post_claim` on
      `PayreqTransport` (no default impls — money-adjacent surface, the
      `post_faucet` posture); the golden digest pinned BOTH repos; bin wiring in
      `account` + `sync` (claim never blocks the local view on failure); the
      welcome-gate hint now names a command that works.
- [x] T7 harness sync-leg parse fix — `accepted` + `rows_new`(+refreshed, skipped),
      `recorded` as a legacy synonym; "recorded what I pushed" = new + refreshed ==
      rows. The stale Issue-010 diagnostic now names BOTH skip causes (config
      defect vs unfunded treasury) and points at the fund route; the pinning test
      updated to the two-cause contract.

## Gates

- [x] G1 dapps — kat-service `--lib` 47/0 (was 44; +2 claim +1 fund),
      `kat_payreq` 20/0 (was 18; +2), `kat_ledger` 14/0 count-identical;
      clippy 0 in touched files.
- [x] G2 clippy — lib 1372/0/1i under `kat_transport,kat_service,latent_retrieval`
      (+4 claim tests); clippy 0 touched.
- [x] G3 live devnet proof — wrangler dev (portable Node 22; wrangler 4 needs ≥22,
      box has 20) on 127.0.0.1:8813, crank-warm → funded (600.5M micro via the fund
      route) → claimed (100 KAT, gate Open) → **steps 1–4 PASS** (preflight /
      treasury-paid faucet top-up / real scored scan→fix cycle with a metered spend /
      6942-row sync recorded new=6942). Step 5 refuses on the structural cap
      conflict (finding 3) — verdict NOT READY, honest.
- [x] G4 records — this plan + the 061 tracker row (T4.4a `- [-]` with the named
      design decision) + both repos committed/pushed.

## Remaining (owner-gated — the step-5 semantics call)

- [-] Pick the refund-leg meaning: (a) settled-invoice readback + balance assertion
      (pure harness, my recommendation — proves "the credit arrives" without a second
      cap draw), or (b) ride the free-tier daily grant leg (needs the harness to push
      a burn for the activity marker — new client surface). Either unblocks the full
      6-step READY verdict; then T4.4a (testnet wet-run) proceeds behind T2.2/T2.3.
- [-] wasm-opt on this box: worker-build's wasm-opt stage stalls ~25 min (and died
      twice mid-optimize). The temp `wasm-opt = false` manifest block was REVERTED
      before commit (production/M3 keep optimization); re-add it locally for future
      4090-box service builds, or pre-warm and accept the 25 min.

## Composition notes (second independent run, 2026-09-05)

An independent session re-ran the full composition (boot → fund → claim →
wetrun) and confirmed G3 end to end, including the `--dry-run` arm GREEN
(steps 1 + 6 PASS, supply counters bit-unchanged) once the claim+fund
surfaces existed. Two operational findings the recipe needs, recorded so
the third run doesn't rediscover them:

1. **Fresh `--persist-to` state is MANDATORY when `KAT_FAUCET_DAILY_CAP_KAT`
   (or any config-affecting var) changes** — the ledger DO cold-boots with
   `ConfigMismatch` over a warm store authored under different config bytes.
   Symptom: `GET /health` → 500 `ledger cold boot: config mismatch`; fix:
   wipe the state dir and re-fund/re-claim.
2. **The faucet DATE is real UTC wall time even under the virtual tick** —
   `utc_today(wall_unix)` ignores `KAT_TICK_SOURCE=virtual` (by design, the
   Issue-004 rule: the host picks the date; `/admin/tick` is unauthenticated
   on workerd dev, so a tick-derived date would be a caller-advancable
   faucet-day roller). Consequence for rehearsal: the per-account daily cap
   rolls at UTC midnight ONLY — a same-day fix-and-rerun of the step-5 leg
   stays cap-blocked no matter what the tick says. Plan the refund-leg
   re-run after the design decision lands accordingly.

Also independently confirmed: the dapps `/sync` route now carries BOTH
shapes' fields (`recorded` + `accepted`/`rows_*`), so the T7 harness parse
fix and the route-side additive field are compatible, not competing fixes.

**Third run (2026-09-06, deployer T4.4a — the ceremony-driven composition):
[Bench 005](../riir-deployer/.benchmarks/005_t44a_devnet_wetrun.md).** Two
additions to this recipe: (3) **the treasury floor is a RESERVE** —
funding to exactly `treasury_grant_floor_micro` leaves zero grant headroom
(`crank_treasury_debit` requires `treasury − grant ≥ floor`), so the fund
MUST be floor + float (the Bench-005 recipe: floor + one faucet draw + one
free-tier grant, all registry-json numbers); a floor-exact fund skips every
credit with `skipped_treasury`; and (4) a FAILED pass leaves an open faucet
request that blocks re-creation until UTC midnight — wipe the persist dir
and re-boot (re-claim re-grants honestly on a fresh devnet ledger; the
once-per-key bound is per-ledger).

# Issue 175 — teardown-and-migrate the LIVE mainnet refine account (71615e8d…) onto the dao rotation lane

**Status:** OPEN — ASAP (the account is live; the 4090 identity already
forked locally 2026-10-26). Owner-directed.
**Claimed:** zed-amo-175-preflight, 1793021421, expires 2026-10-28T13:30:21Z
(scope: the T-PREFLIGHT 4090 leg only — dual-sign + drain under the OLD
account; T-WIRE/T4 stays the dao lane's, no code changes in this lease) Executes the refine slice of
[riir-dao Proposal 001](../riir-dao/.proposals/001_kat_account_key_rotation_social_recovery.md)
(ACCEPTED, T0–T3 landed) per
[Proposal 023](../.proposals/023_refine_mainnet_account_migration.md).
NOT a wetrun.
**Claimed:** zed-shikuwa-4090-idle, epoch=2026-10-26T20:30:00+07:00, expires 2026-10-28T20:30:00+07:00 — T-PREFLIGHT (4090 leg) drain.
**YIELDED to zed-amo-175-preflight** (fc846efd landed on origin first — the live foreign claim is the yield signal; shikuwa stands down, no wire ops from this session).
**Progress 2026-10-26 (doctor sweep):** M3 preflight leg DONE — the
account dir was missing under the post-uid-502 HOME (`~/.config/riir-auth/
did not exist → every run read "not logged in"`); `cargo refine login`
adopted the fresh rotation-target account `d54e39a8…742aaa` IN PLACE
(machine key = account key), sync push ACCEPTED by mainnet (54 rows,
free-tier 100-KAT grant claimed, corpus lease 2964→2970), TUNA 100
(29 d). The 4090 leg is measured WORSE than expected — see the posture
block below: its KAT plane is FAIL-CLOSED BROKEN today (record names a
key path that now holds the new key), so the migration is not just ASAP,
it is the unblock.

## The posture (2026-10-26)

- Live mainnet account `71615e8d…91303` = blake3(pubkey) of the OLD 4090
  SSH key — retired on-box the same day (`id_ed25519.retired-20261026`,
  + the byte-identical `riir-heal` legacy copy). The old key still signs;
  `KEY_OVERRIDE_ENV` signs from the retired path without re-installing it.
- The 4090 now boots `katop@4090-20261026` — the next unguarded refine op
  there derives a FRESH account id. Uncoordinated ops strand the old
  balance/history behind a tombstone-to-be. **No new-identity ops on the
  4090 until the migration files or the loss is explicitly accepted.**
- Local sync staleness (measured on the M3): `kat_burn_ack.json`
  2026-09-18 (38 d), `kat_sync_state.json` 2026-10-06 (20 d) — offline
  rows must drain UNDER THE OLD ACCOUNT first.

## Tasks

- [x] **T-PREFLIGHT (M3 leg — DONE 2026-10-26):** the account state never
  moved with the 2026-10-25 uid-502 HOME switch (`/Users/kat/.config/
  riir-auth/` absent → "not logged in", rows stacking offline). Fixed by
  fresh login as `d54e39a8` (the census rotation target, adopted in
  place) + sync green. The OLD home (`/Users/katopz`, permission-denied
  to the new user) keeps the retired `c79c93b8` account state as history
  — deliberately NOT copied (watermark-reset rule); the dao sweep covers it.
- [ ] **T-PREFLIGHT (4090 leg):** measured 2026-10-26 —
  `C:\Users\katop\.config\riir-auth\account.json` = `71615e8d…91303`,
  `key_path = C:\Users\katop\.ssh\id_ed25519` — **which now holds the
  NEW `katop@4090-20261026` key**; the old key (71615e8d's binding)
  survives as `id_ed25519.retired-20261026` (+ the `riir-heal` legacy
  copy). Signing as 71615e8d therefore MISMATCHES → the plane refuses
  loud (no silent fork — inference from the record/key mismatch, not yet
  a measured wire refusal). Remedy = the migration itself: dual-sign via
  `KEY_OVERRIDE_ENV=C:\Users\katop\.ssh\id_ed25519.retired-20261026`.
  The per-repo burn acks live in the repos' `.refine/` dirs (E:\git\riir-*),
  not the home dir — drain whichever have offline rows once signing works.
- [ ] **T-WIRE:** verify/land the dao chain this lane waits on: T3d
  (tally + snapshot verifier) → T4 (THIS repo's `account migrate` /
  `--object` / `--status` / `--census` CLI) → T6 (battery) → T7 (devnet
  rehearsal).
- [ ] **T-E2E:** the Proposal-023 battery — E-1 no-objection release +
  tombstone refusal; E-2 objection-cancel + claimant-only cancel; E-3
  preflight refuses stale burn ack; E-4 watermark reset post-release; E-5
  released-id re-file refusal.
- [ ] **T-MIGRATE:** file the real rotation (`account migrate --github
  katopz`, old key via `KEY_OVERRIDE_ENV`), ride the 24 h default-deny
  window, release (sweep + tombstone + github binding move).
- [ ] **T-POST:** census reads zero live old-key bindings; 4090 runs
  new-identity permanently; delete `id_ed25519.retired-20261026(.pub)`
  and the `riir-heal` legacy copy (tombstoned account = nothing to
  recover) — closes the 2026-10-26 deferred cleanup. On the 4090 the
  post state also repairs `account.json` (re-login under the new
  identity so `key_path` names a key whose blake3 IS the account id).
- [ ] **T-KEYS:** the API-key freshness sweep around refine + riir-chain
  (CF deploy/ops/prod tokens, GH PATs, HF tokens, wrangler OAuth,
  chain ops bearers) — inventory + steps in
  `../riir-chain/.issues/169_api_key_rotation_sweep.md`; CF half verified
  via the deployer sibling guard `--live` after the roll.

## Acceptance

- [ ] E-1..E-5 green on the devnet rehearsal posture.
- [ ] Census zero after release; old account tombstone proven by refusal.
- [ ] Burn ack + lease + mining-consent continuity: no drained row lost,
  no watermark replayed onto the new account.

# Issue 175 — teardown-and-migrate the LIVE mainnet refine account (71615e8d…) onto the dao rotation lane

**Status:** OPEN — ASAP (the account is live; the 4090 identity already
forked locally 2026-10-26). Owner-directed.
**Claimed:** zed-amo-175-preflight, 1793021421, expires 2026-10-28T13:30:21Z
(scope: the T-PREFLIGHT 4090 leg only — dual-sign + drain under the OLD
account; T-WIRE/T4 stays the dao lane's, no code changes in this lease)
**Claimed:** zed-amo-175-twire, 1793024781, expires 2026-10-28T21:06:21Z
(scope: T-WIRE code — T3d riir-dapps tally + close, T4 this repo's CLI,
T6 E1–E11 battery; T5 riir-dao weight artifact rides a follow-up unit;
T7 devnet rehearsal + the real rotation stay owner-adjacent, untouched) Executes the refine slice of
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
(29 d). **4090 leg DONE 2026-10-26 ~20:4x +07 (zed-amo-175-preflight)**:
identity restored to `71615e8d…` via the record projection (the env-var
remedy measured broken — see the task block); the 9-repo drain measured
a burn no-op + ONE real mining drain (riir-refine, 76 rows ACCEPTED by
mainnet under the old account); leases pulled to 2970. The plane is
CONTINUOUS under the old identity — T-MIGRATE files whenever dao T4
lands, no drain owed first.

## The posture (2026-10-26)

- Live mainnet account `71615e8d…91303` = blake3(pubkey) of the OLD 4090
  SSH key — retired on-box the same day (`id_ed25519.retired-20261026`,
  + the byte-identical `riir-heal` legacy copy). The old key still signs;
  `KEY_OVERRIDE_ENV` signs from the retired path without re-installing it.
- The 4090 record now sits ON the old identity (`key_path` → the retired
  file) — unguarded refine ops there bill the OLD account (the safe
  direction) until the migration files. A deliberate re-login under the
  new identity is T-POST's move, never an accident now.
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
- [x] **T-PREFLIGHT (4090 leg):** DONE 2026-10-26 ~20:3x +07 (zed-amo-175-preflight
  via ssh; full narrative in this block + the snapshot row). Measured chain:
  (1) the written remedy is WRONG-BY-MEASUREMENT — `KEY_OVERRIDE_ENV`
  (`RIIR_AUTH_ACCOUNT_KEY`) is consulted ONLY at `login_default` (riir-auth
  `account_key.rs` line 567, the no-record adoption chain); the per-run
  `load_with_home` path never reads it, so a run with the env set signed
  NOTHING — it took the record's `key_path` (now the NEW key), derived the
  fresh id `4f4623f3…`, and the "key is the authority" recovery REWROTE the
  record (loud, not a refusal — the predicted mismatch-refusal does not
  exist as coded). That fresh-id derivation is the issue's own warned
  class; it happened BEFORE the migration filed. Disclosure: one service
  read fired under `4f4623f3`; the once-per-key free-tier grant state there
  is UNVERIFIED (the render carried both "no grant row yet" and "claimed"
  lines; balance 0; no funds can move — grants are not transfers). The
  T-POST re-login makes the same identity the deliberate end state, so the
  drift is timing, not substance. (2) The working remedy is the RECORD
  PROJECTION: `account.json` is a projection ("the record is a projection,
  the key is the authority") — one `key_path` edit to
  `id_ed25519.retired-20261026` (backup at `account.json.pre-175-preflight.bak`;
  PS 5.1 `Set-Content -Encoding UTF8` writes a BOM serde_json rejects —
  the P44 class, rewritten BOM-free via .NET `WriteAllText`) rebinds the
  plane to `71615e8d…91303` on the next load (one recovery line, expected).
  The record is LEFT on the old identity — "no new-identity ops until the
  migration files" holds structurally now. (3) THE DRAIN MEASURED A NO-OP
  by construction: `sync` across all 9 E:\git repos signed `7161…`
  everywhere; the burn channel skips silently per-net (the current code
  reads `<root>/.refine/net/<host>/kat_charge.json` — fresh-born zeros —
  the September ROOT books are legacy pre-split files, account-scoped ack
  burn 42 KAT ≥ every local total, and non-ack legacy files are
  `config adopt-state --discard`-class anyway — NOT touched, the
  dao/migration lane decides); the mining push carried 0 rows on 8 repos
  (blocked-by-policy, not stacks) and **76 rows on riir-refine — PUSHED
  AND ACCEPTED by mainnet** (rows enter the proof queue; server ingest
  rows_new 0 recorded 0 = known shapes, the contribution fact + batch
  commitment landed under the OLD account before any tombstone — the
  drain's actual payload). All 9 repos' corpus leases pulled to epoch
  2970 (9 × 0.001 KAT charged to the old account). The old account's
  service ledger reads 0.000000 KAT credit at epoch 2964 (the September
  ack balances 85–97 KAT are server-side history — spent/decayed there,
  nothing local to recover; the free-tier grant row for the old account
  shows claimed-and-live). Consequence for T-MIGRATE: the account is
  CONTINUOUS (signed, synced, contributed) — file the rotation whenever
  the dao T4 CLI lands; no drain is owed first.
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

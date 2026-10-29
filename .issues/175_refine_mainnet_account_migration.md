# Issue 175 — teardown-and-migrate the LIVE mainnet refine account (71615e8d…) onto the dao rotation lane

**Status:** OPEN — ASAP (the account is live; the 4090 identity already
forked locally 2026-10-26). Owner-directed.
**Lease released:** zed-amo-175-te2e (claimed 2026-10-29, epoch 1793265834, expiry 2026-10-31T09:23:54Z) — T-E2E RAN; the assembled battery measured green on every reachable cell and is recorded with the remaining tails in `.issues/189_te2e_battery_results_issue175_plan302_t1.md` (plan-302's T1 tick, the E-5 live attempt on the M3, the T3b + promote owner calls). Prior claim zed-m3-175-t7 expired 2026-10-29T07:11:08Z with no landing — reclaimed per the 48 h/expiry rule.
**Claimed:** zed-m3-175-t7, 1793085068, expires 2026-10-29T07:11:08Z
(scope: T-E2E + the T7 devnet rehearsal only — the E-battery against a live
devnet worker per `.plans/302` T1; T-MIGRATE filing stays owner-adjacent)
**Lease closed:** zed-amo-175-preflight (claimed 2026-10-26, epoch
1793021421) — its scoped leg (T-PREFLIGHT 4090) is DONE per the checkbox
below; released 2026-10-27 by owner call ("sibling is done or leftover")
to open the rehearsal lane.
**Lease closed:** zed-amo-175-twire (claimed 2026-10-26, epoch 1793024781) —
T-WIRE landed same day across five repos (T3d/T4/T5/T6; the commits in
the T-WIRE row); T7 + T-E2E/T-MIGRATE/T-POST/T-KEYS remain, owner-adjacent Executes the refine slice of
[riir-dao Proposal 001](../riir-dao/.proposals/001_kat_account_key_rotation_social_recovery.md)
(ACCEPTED, T0–T3 landed) per
[Proposal 023](../.proposals/023_refine_mainnet_account_migration.md).
NOT a wetrun.
**Lease released:** zed-shikuwa-4090-idle (claimed 2026-10-26T20:30:00+07:00, would have expired 2026-10-28T20:30:00+07:00) — T-PREFLIGHT (4090 leg) drain; reshaped per the riir-infer-1109 malformed-line precedent — the old `**Claimed:**` shape could not parse (`epoch=` + ISO instead of epoch digits), and an unparseable claim-shaped line exits claim_lease_check BLIND for every future lane.
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

**Sequencing verdict 2026-10-27 (`.plans/302_issue175_migration_then_secret_reset.md`):** the full secret reset (T-KEYS + the origin-vault wetrun) does NOT replace this migration — disjoint credential planes; order is MIGRATE FIRST, the owner mints land inside the 24 h window, the wetrun (the fingerprint e2e with fresh secrets) AFTER release so it doubles as the post-roll verifier. No re-genesis is required by any current plan (dao T0 item 14: no located genesis/validator binding); corpus + contribution live WORKER-side (KatLeaseDo / MiningDo / the ledger families) and survive both the sweep (one-batch re-key, per-family conservation) and any chain re-genesis. Full analysis + the data-survival table: plan 302.

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
- [x] **T-WIRE (LANDED 2026-10-26, zed-amo-175-twire):** the dao chain
  this lane waits on is code-complete: T3d (riir-kat `8e25fa0` kernel +
  riir-dapps `5d0a231` readers/close/lazy + `6091af5` route row) → T4
  (THIS repo's `account migrate` CLI, `97c987d0`; riir-kat `f3a264b` +
  riir-auth `5a24e73` carry the key loader + PoW measure) → T5 (riir-dao
  `c7b9caa` weight artifact + quorum + snapshot audit) → T6 (the battery
  assembled: wire cells E1–E11 across migrate_contract.rs 17 migrate
  tests + the T3b/T3c store arms + the dao re-pins). **POLARITY FINDING
  (product):** the T2 composer's "React with 👍 to APPROVE the migration"
  inverted the ratified E3/E4 (katopz 👍 → cancelled; 👎 → released);
  fixed in the composer + pinned in the kernel — the owner would have
  flipped their own release vote at the window. **T7 (devnet rehearsal +
  the real rotation) remains OPEN, owner-adjacent** — the only leg
  between here and `T-MIGRATE`.
- [ ] **T-E2E:** the Proposal-023 battery — E-1 no-objection release +
  tombstone refusal; E-2 objection-cancel + claimant-only cancel; E-3
  preflight refuses stale burn ack; E-4 watermark reset post-release; E-5
  released-id re-file refusal.
- [ ] **T-MIGRATE:** file the real rotation (`account migrate --github
  katopz --old-key <path>` — the T4 product law: the env var is NEVER a
  signing source; the dual-sign needs BOTH keys on ONE box, see plan 302
  T2 for the key-crossing decision), ride the 24 h default-deny window,
  release (sweep + tombstone + github binding move).
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
  - [x] **T-KEYS pre-roll baseline MEASURED 2026-10-26 ~15:0xZ**
    (zed-m3-tkeys-preroll; full record: riir-chain
    [169 addendum](../../riir-chain/.issues/169_api_key_rotation_sweep.md)
    @ `4f433d35`). Guard `--static` 0 red / 6 advisory + `--live` 0 red
    / 0 advisory (all refusal legs hold with TODAY's tokens — any
    post-roll red is attributable). Resolved from "unknown": wrangler
    OAuth session stale-live (access token minted ~10:10Z today, refresh
    present, drop still preferred; real path
    `~/Library/Preferences/.wrangler/config/default.toml`). **The
    load-bearing finding: the riir-train/.env HF WRITE token (the M3
    publisher) is INVALID** — whoami-v2 refuses it; the HF push lane
    (`data_backup.sh push*`) fails auth until the owner mints (the
    refine/.env read token is VALID, role read). Items 6/7/9/10 stay
    owner mints (not locally resolvable; CI-dispatch verifiers held
    until post-roll per the spending convention). KAT-plane verifier
    baseline recorded (d54e39a8: ledger 0 KAT @ epoch 2964, TUNA 100
    / 29 d, planes healthy).

## Acceptance

- [ ] E-1..E-5 green on the devnet rehearsal posture.
- [ ] Census zero after release; old account tombstone proven by refusal.
- [ ] Burn ack + lease + mining-consent continuity: no drained row lost,
  no watermark replayed onto the new account.

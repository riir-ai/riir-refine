# Issue 175 — teardown-and-migrate the LIVE mainnet refine account (71615e8d…) onto the dao rotation lane

**Status:** OPEN — ASAP (the account is live; the 4090 identity already
forked locally 2026-10-26). Owner-directed. Executes the refine slice of
[riir-dao Proposal 001](../riir-dao/.proposals/001_kat_account_key_rotation_social_recovery.md)
(ACCEPTED, T0–T3 landed) per
[Proposal 023](../.proposals/023_refine_mainnet_account_migration.md).
NOT a wetrun.

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

- [ ] **T-PREFLIGHT (runnable today, no wire needed):** drain local sync
  state under the OLD account on both boxes (M3 + 4090) — `cargo refine
  sync` + a burn-sync pass; re-check ack watermarks advance (200-accepted
  contract). Freeze new-identity ops on the 4090.
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
  recover) — closes the 2026-10-26 deferred cleanup.
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

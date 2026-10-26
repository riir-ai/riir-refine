# Proposal 023 — refine mainnet account migration: the 4090 heal-network account onto the rotation lane (ASAP — the account is LIVE)

**Status:** PROPOSED — owner-directed 2026-10-26 ("riir-refine is live on
mainnet, no issue to teardown current and do full migration yet, migration
ASAP"). Executes the riir-refine slice of
[riir-dao Proposal 001](../riir-dao/.proposals/001_kat_account_key_rotation_social_recovery.md)
(ACCEPTED — the GitHub-attested key-rotation lane; T0–T3 LANDED, T3d
claimed, T4 = this repo's CLI slice, T7 = rehearsal + real rotation).
NOT a wetrun: the account is live TODAY, and the 4090's 2026-10-26 SSH
regen already forked the local identity posture — every further
uncoordinated op risks stranding value behind a tombstone-to-be.

Companion issue: `.issues/175_refine_mainnet_account_migration.md`.

## The live posture (measured, 2026-10-26)

| Fact | Value | Consequence |
|---|---|---|
| Live mainnet account | `71615e8d…91303` (blake3(pubkey) of the OLD 4090 SSH key) | all KAT credit / mining history / lease rows live here |
| Old key | retired on-box 2026-10-26 as `id_ed25519.retired-20261026` (+ `.pub`); the `riir-heal` legacy copy is byte-identical (verified) | the old key STILL SIGNS — the dual-sign migration can run without any key-file shuffle (`KEY_OVERRIDE_ENV` names the retired path) |
| 4090 current key | `katop@4090-20261026` (regen, gitsync-verified) | the next unguarded `cargo refine` op on the 4090 derives a FRESH account id — local state forks; the old balance/history orphan behind a tombstone-to-be |
| M3 `.refine` state | `kat_burn_ack.json` 2026-09-18, `kat_sync_state.json` 2026-10-06 | burn-sync + lease sync are STALE — offline-stacked rows must drain UNDER THE OLD ACCOUNT before any tombstone |

**Why ASAP:** the rotation lane's tombstone is one-way and the migration
needs the old key to sign. Today that key exists and the value is small;
the longer the fork posture stands, the more burn/mining/lease rows
accumulate on an identity whose signing key is a retired file.

## What this repo owns (the dao routing row, expanded)

riir-dao Proposal 001 §Routing assigns riir-refine the **CLI surface**.
This proposal pins the refine-local deliverables that make THIS account's
migration safe:

1. **T4 CLI slice** (`account migrate` / `--object <request-id>` /
   `--status` / `--census`) — the dao flow verbatim: one line to file the
   dual-signed request, prefilled-URL objection flow, 24 h default-deny
   window, census-before-release refusal.
2. **Continuity preflight** — a `--census` subcommand mode (report-only)
   that walks THIS box's `.refine` state and names every account-keyed
   surface with its staleness: burn ack watermark + offline-stacked rows
   (Issue 076), sync watermark, corpus-lease rows, mining-consent records.
   The preflight is the "sync first" gate: the operator cannot FILE a
   migration while local rows are unsigned-for (they must drain under the
   old account first — `cargo refine sync` + the auto burn-sync path).
3. **E2E battery** (see below) — refine-side tests feeding the dao T6
   battery and T7 devnet rehearsal.

## The continuity surfaces the sweep must cover (census seed)

Every account-keyed row family this repo touches — the sweep re-keys
them, the census must read zero live old-key bindings after release:

- `kat:burn:…` burn-report watermarks + any offline-stacked report rows
  (the `kat_burn_ack.json` ack contract: watermark advances ONLY on
  200-accepted).
- corpus-lease rows (`kat:lease:…` / the `corpus lease` epoch plane) — a
  live lease under the old account must re-key or the corpus plane
  dead-locks at tombstone.
- mining-consent / pending-proof rows (the `--mine` quorum plane) —
  consent is an account property.
- `kat_sync_state.json` / `kat_sync_watermark` — machine-local, but the
  migration must RESET them against the new account (never replay old
  watermarks onto a fresh ledger position).

## E2E battery (the user requirement: "e2e test needed")

Runs on devnet rehearsal data + the real-wire shapes; both paths of the
dao flow, plus the refine-local continuity class:

- **E-1 no-objection release:** file dual-signed request (old key
  `KEY_OVERRIDE_ENV` → retired path) → 24 h window expires clean →
  release → census reads ZERO live old-key bindings → every signature
  route on the old account refuses `dead_account` (the CLI's own calls
  prove the tombstone).
- **E-2 objection-cancel:** objection issue → zero votes → cancelled;
  claimant-only cancel proven (old-key-only cancel REFUSES — the
  thief-shape test).
- **E-3 continuity preflight refuses stale:** with a deliberately stale
  burn ack, `account migrate` refuses naming the drain command; after
  drain it passes.
- **E-4 watermark reset:** post-release, the new account's sync state
  starts clean (no old watermarks replayed).
- **E-5 idempotence:** a released request id can never re-file (dao rule).

E-1/E-2 ride the dao battery's fixtures; E-3/E-4 are refine-local and new.

## Ops sequence (the ASAP runbook, once T3d+T4 are on the wire)

1. **Preflight (today, no wire needed):** on BOTH boxes, sync under the
   OLD account while it still signs — `cargo refine sync` + a `--fix`
   burn-sync drain (or the explicit sync verb). M3's ack is 38 days
   stale; the 4090's is its own posture. No new-identity ops on the 4090
   until the migration files (or explicitly acknowledged as fork-loss).
2. Land/verify the wire chain: dao T3d (tally) → T4 (this CLI) → T6
   (battery) → T7 (devnet rehearsal).
3. File the REAL rotation as the drill (dao directive 3 — signing works,
   token valueless, cheapest day): `cargo refine account migrate
   --github katopz` from the box that can load the old key.
4. 24 h window; owner votes 👎 on any challenge (default-deny) or lets it
   release clean.
5. Release: sweep + tombstone + github binding move + census zero.
6. Post: 4090 runs new-identity permanently (already regen'd); delete
   `id_ed25519.retired-20261026(.pub)` ONLY after census zero (resolves
   the 2026-10-26 deferred cleanup; the riir-heal legacy copy dies with
   it — the old account is tombstoned, there is nothing left to recover).

## Key-freshness sweep (the broader owner ask)

Beyond the account key: all API credentials around refine + riir-chain
rotate to maximum freshness — CF deploy/ops/prod tokens, GH PATs
(`SIBLING_REPOS_TOKEN`, `RELEASE_PAT`), HF tokens, wrangler OAuth
sessions, chain ops bearers. Inventory + steps: `.issues/169_api_key_rotation_sweep.md`
in riir-chain; the CF half verifies through THIS repo's sibling guard
(`riir-deployer/scripts/deploy_token_guard.sh --live`) after the roll.
Many mints are dashboard acts (owner); the issue carries the exact scope
tables so the owner act is copy-paste, then the verification is runnable.

## Honest caveats

- The lane is only as done as the dao status line: T3d is CLAIMED, not
  LANDED — this proposal cannot execute before the wire exists. The
  preflight (step 1) is the only runnable piece today and it is also the
  most time-critical (staleness grows daily).
- `KEY_OVERRIDE_ENV` signing from the retired path must be proven in the
  T4 tests (a retired key file with 0600 perms is just a file — but the
  tests prove the CLI never needs it INSTALLED).
- If the owner chooses to keep serving from a NEW account WITHOUT
  migration (fork-and-forfeit), the tombstone step becomes optional —
  but mining history, lease continuity, and burn history are LOST; this
  proposal assumes migration (owner said "full migration").

# Plan 302 — Issue 175 sequencing: migration FIRST, then the full secret reset (T-KEYS + origin-vault wetrun); no re-genesis required

**Status:** ACTIVE — verdict + runbook for the owner's 2026-10-27 questions
(readiness, the plan recollection, corpus/contribution survival across
"re-genesis", rotation-vs-migration order). Companions:
`.issues/175_refine_mainnet_account_migration.md`,
`../riir-chain/.issues/169_api_key_rotation_sweep.md`,
riir-deployer Plan 008 (origin-vault), riir-dao Proposal 001 (the rotation lane).

## Verdict (the three questions)

1. **Ready to file the real rotation? NOT yet — exactly one leg remains.**
   The wire chain is LANDED (dao T3d tally + T4 CLI + T5 artifact + T6
   battery, 2026-10-26) and the plane is CONTINUOUS (both T-PREFLIGHT legs
   done; the 76-row drain accepted; no drain owed). What blocks T-MIGRATE
   is T7: the devnet rehearsal of BOTH paths (quiet release +
   objection-cancel) against a live worker, carrying the refine E-battery.
   After T7 is green → file the same day. The old key must stay loadable
   until filing; every day of delay extends the retired-key fork window.

2. **The owner's recollection is correct, and it is mostly ALREADY LANDED
   as code** — what remains are owner acts + verifiers:
   - "e2e deployment with all-new SECRETs generated in container" =
     riir-deployer **origin-vault** (Plan 008): T0–T11 ALL LANDED —
     `scripts/vault_e2e_lane.sh` 10/10; the T4b genesis engine generates
     the seed INSIDE the digest-pinned docker carrier (`--rm`; the private
     half never persists); G2 unlock 16.3 ms cold. The OPEN piece is
     owner-gated: **the WETRUN itself** — the Developer-ID-signed binary +
     a mainnet manifest + the ONE real Touch-ID ceremony.
   - "fingerprint via passkey or keyring" = origin-vault's **Touch-ID
     biometric primary** (SEP-gated keychain `BiometryCurrentSet` item) +
     riir-auth Plan 001 `biometric_login` (keychain custody via
     security-framework; signed-binary law MEASURED — ad-hoc binaries fail
     -34018; `scripts/codesign_biometric.sh` is the recipe).
   - "full clean rotate all secret/env" = **T-KEYS**, riir-chain issue 169:
     items 2/3/4/6/7/9/10 are OPEN owner dashboard mints (copy-paste scope
     tables in the issue); item 5 (wrangler OAuth) RESOLVED (dropped);
     item 8 (HF write token) REPAIRED (minted 2026-10-27). Verifiers are
     runnable post-roll; re-baseline behind riir-deployer `.issues/016`
     (the zed-upstream scan-noise marker) so the zed reds are out of the
     signal.

3. **The rotation does NOT replace the migration; corpus + contribution
   survive everything currently planned.** The two are disjoint credential
   planes (table below). NO current plan requires a chain re-genesis or a
   chain "master key" re-issue: the dao T0 census (item 14) found NO
   located genesis/validator authority binding in `riir-chain/src`;
   chainspec ceremony keys are devnet-ephemeral (169 item 12); the account
   keys ride the dao lane (169 item 1); chaind gm/roll signing follows the
   account rotation via dao T8 `RotateAccount` — a separate DEFERRED plan,
   not a re-genesis.

## Where the value lives (what survives what)

| Datum | Home | Account migration (sweep re-key) | Chain re-genesis (NOT planned) | Worker-ledger wipe (NEVER planned) |
|---|---|---|---|---|
| Corpus lease epochs + manifest + segments | worker `KatLeaseDo` (content-addressed, additive) | KEPT (re-keyed; per-family conservation) | KEPT (different plane) | LOST — re-pushable via `corpus-pr` |
| Contribution rows + batch commitments (the 76 drained) | worker ledger + `MiningDo` | KEPT (ledger families move in the ONE-batch sweep; MiningDo pending rows are TTL'd ephemera — the T3b owner call, surface at T7) | KEPT | LOST — no export lane exists today |
| Burn/sync watermarks, free grant, TUNA | worker ledger, per-account families | KEPT + reset cleanly (E-4: no watermark replayed onto the new account) | KEPT | LOST |
| GitHub binding, tombstone, request rows | worker ledger | MOVED by design (census-zero gate) | KEPT | LOST |
| Chain tx/settlement history | riir-chain state | UNTOUCHED | **LOST** — the only datum a chain re-genesis costs | n/a |
| Chain-bound worker rows (stakesig/stakedist/queue receipts) | worker, keyed to chain ids | SKIPPED by the sweep (chain-bound, by design) | ORPHANED (chain ids reset) | n/a |

## Order (and why)

**MIGRATION FIRST; the rotation woven through; re-genesis = not needed
(a separate owner proposal only if ever wanted).**

- The migration is the ASAP item and the old key must still sign; the
  T-KEYS rotation has no dependency on it and vice versa (169 item 1
  explicitly routes the account keys to the dao lane, out of the sweep).
- The 24 h objection window is dead time: the T-KEYS dashboard mints
  happen THERE. NO worker deploys mid-window — keep the one-way window's
  moving parts at zero; worker secret re-binds land after release.
- The origin-vault wetrun comes AFTER release + after the rotated
  credentials are installed, so it doubles as the post-roll e2e verifier:
  ONE Touch-ID ceremony exercising the FRESH credential set end to end —
  exactly the "e2e deployment with all-new secrets" test the owner wants.

## Tasks

- [x] **T1-pre — the live venue (2026-10-27):** proposal repo = PUBLIC
      `gist-rs/riir-dao-proposal` (riir-dao is private; owner-created);
      `KAT_MIGRATE_ISSUES_TOKEN` set on the APEX by the owner. riir-dapps
      `db9e6dc` (repo-base var apex+devnet; label-free find-or-create — a
      silently dropped label would have duplicated issues per retry; reader
      falls back to the issues token; `[env.devnet] build` compiles
      `kat_migrate`) + refine `e4084013` (CLI default repo). DEVNET DEPLOYED
      version `5f77c6e0` — `/account/migrate/*` live (404 → `unknown_request`).
      Mainnet still has NO migrate routes (apex build unchanged — promotion is
      the post-rehearsal owner call; setup.sh --mainnet runs the R8 gate).
- [ ] **T1-secret (owner):** `npx wrangler secret put KAT_MIGRATE_ISSUES_TOKEN
      --env devnet` (same token) — devnet cannot file the proposal issue
      without it (filings stay E12-unreleasable).
- [ ] **T1 — T7 devnet rehearsal (agent):** run the assembled battery
      against a live devnet worker at the compressed window: dao E1–E11
      (`migrate_contract.rs` 17 tests + the sweep battery + 3 spec-match
      arms) AND the refine Proposal-023 cells (E-1 no-objection release +
      tombstone refusal; E-2 objection-cancel + claimant-only cancel;
      E-3 census preflight refuses a stale burn ack; E-4 watermark reset;
      E-5 released-id re-file refusal — note the two E-numberings are
      distinct taxonomies; E-1/E-2 ride the dao fixtures, E-3/E-4 are
      refine-local). Drill pair `c79c93b8… → 1bb4b7b5…` (devnet, valueless).
      SURFACE the T3b owner call: the mining-plane tombstone gate gap
      (pending rows are TTL'd ephemera) — accept the ephemera or gate at
      T7. GOAT verdict → `kat_migrate` promote = owner call.
- [ ] **T2 — T-MIGRATE prep (agent):** the key-crossing decision — the
      dual-sign needs BOTH keys on ONE box (the CLI loads the old key via
      `--old-key <path>`; the claim side rides the record's own key).
      ⛔ **CORRECTED 2026-10-27 (measured):** the 4090's
      `id_ed25519.retired-20261026` derives to account `71615e8d…` (the
      4090's OWN identity — `ssh-keygen -y` on the private file, blake3 of
      the raw key; its riir-auth `account.json` + riir-heal legacy copy say
      the same), NOT `c79c93b8…`. Filing with it would migrate the WRONG
      account. `c79c93b8` is the OLD M3 key, presumed at
      `/Users/katopz/.ssh/id_ed25519` (the pre-switch macOS user; mode 700,
      unreadable to `kat` — the OWNER copies it: `sudo install -m 600 -o kat
      /Users/katopz/.ssh/id_ed25519 /tmp/kat_old_c79c93b8`; verify
      `blake3(pubkey)` starts `c79c93b8` before use). File on the M3 (the
      claimant record `d54e39a8`'s home) with that TEMP 0600 copy, shredded in
      the same session — prefer exposing the old key (it dies at tombstone
      anyway) over spreading the M3's long-lived identity key to the 4090.
      Pre-file: `account migrate --census` green on BOTH boxes.
- [ ] **T3 — FILE + the window (owner-adjacent):**
      `cargo refine account migrate --github katopz --old-key <temp>`;
      record the request id. DURING the window: owner mints T-KEYS items
      2/3/4/6/7/9/10 per 169's scope tables. Owner votes 👎 on any
      objection (polarity FIXED 2026-10-26: 👎 = REJECT the objection →
      release proceeds). NO worker deploys mid-window.
- [ ] **T4 — release + T-POST (agent):** close (lazy status read or the
      explicit `POST /account/migrate/close`) → sweep → tombstone →
      github binding move; census zero; `account --history` + a ledger
      snapshot as the continuity receipt; 4090 re-login under `d54e39a8`
      (deliberate — repairs the record so `key_path` names a key whose
      blake3 IS the account id); owner removes the old key from the 4090's
      `administrators_authorized_keys`; delete
      `id_ed25519.retired-20261026(.pub)` + the `riir-heal` legacy copy;
      shred the T2 temp copy (verify gone).
- [ ] **T5 — T-KEYS finish (agent):** install the minted set into the
      stores (`.env` rows, `wrangler secret put` via stdin for worker
      bindings, `gh secret set`, HF `.env` — 169's store list); land
      riir-deployer `.issues/016` so the post-roll re-baseline is clean;
      run the verifiers in 169's order: guard `--static` (0 red) →
      `--live` (all refusal legs hold) → `cargo refine account` + `tuna`
      reads → the item-6/7 CI dispatches (feature-guard, release dry-run);
      record every roll date in 169's table (the freshness bar).
- [ ] **T6 — the e2e deployment test with all-new secrets (owner + agent):**
      the origin-vault WETRUN: `scripts/codesign_origin_vault.sh`
      (Developer ID) → a mainnet manifest → `origin-vault --dest <mainnet>
      --reason` → boundary gate → TOTP → Touch ID → the digest-pinned
      carrier execs `wrangler@4` (credential on STDIN, never argv) → the
      completion line names the wetrun teardown (`riir-deploy down`).
      This is the ONE real Touch-ID acceptance row. Optional same-day:
      the riir-auth `biometric_login` custody rehearsal (codesign first —
      the -34018 law).
- [-] **T7 — chain "master key" / re-genesis: NOT REQUIRED — DEFERRED to
      an explicit owner proposal.** Nothing in the rotation inventory or
      the migration needs it (dao T0 item 14: no located genesis/validator
      binding; 169 items 11/12). If ever wanted: file the proposal FIRST;
      export chain tx history BEFORE (the one datum it costs; no export
      lane exists today — the proposal must build one); expect the
      chain-bound worker rows (stakesig/stakedist/queue receipts) to
      orphan; schedule it BEFORE any future account migration, never
      after one. A WORKER-ledger wipe is the only surface that destroys
      corpus + contribution — never planned; if ever contemplated, the
      corpus re-push rides `corpus-pr` and contribution history needs an
      export lane built first.

## Acceptance

- [ ] T1 green on devnet; both owner calls recorded (mining ephemera,
      `kat_migrate` promote).
- [ ] T-MIGRATE filed → window clean (or objections 👎'd) → release →
      census zero.
- [ ] 169's table fully dated; every post-roll verifier green (after the
      016 re-baseline).
- [ ] The wetrun completion line recorded (the fingerprint acceptance row).
- [ ] No drained row lost, no watermark replayed (issue 175's continuity
      acceptance); corpus lease + contribution planes intact post-release
      (the per-family conservation receipt).

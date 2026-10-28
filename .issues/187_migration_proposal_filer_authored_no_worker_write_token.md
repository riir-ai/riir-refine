# Issue 187 — the migration proposal issue is FILER-authored; the worker holds NO GitHub write token

**Status:** OPEN — filed 2026-10-28 (owner steer + Claude verdict AGREE). Blocks the mainnet migrate promotion (plan 302 T2). SUPERSEDES the "mint a BOT issues token" item in plan 302 (no bot needed once the worker cannot write).

## Problem (measured, 2026-10-28)

Issue 183 made the owner's 👍 on the proposal issue the release AUTHORIZATION (owner weight 51% — decisive). Today the worker CREATES that issue with `KAT_MIGRATE_ISSUES_TOKEN`, which is the OWNER's PAT (plan 302 T1-live: "author = katopz → the issues token is the OWNER's PAT"), installed as a Cloudflare secret on BOTH the apex and devnet (`wrangler secret list`, 2026-10-28).

1. **The deciding vote sits in a worker secret.** GitHub's issue-reaction write rides the same `Issues: write` permission the token needs to create issues (not independently verified here — owner check below), so the secret can add katopz's 👍 — whoever reads the worker's secrets, or steals the token, holds the 51% vote. Repo-scoping the token does NOT help: the vote is cast ON that repo.
2. **Notification gap:** a stranger's migration request is authored by katopz, so GitHub never notifies the owner (no self-notification, no self-mention) — fail-closed (E12, no release without 👍) but user migrations stall unseen.
3. A bot account (the previous plan) fixes (2) but only MOVES the write secret; it still sits in the worker.

## Decision

The worker holds NO GitHub write credential. The filer authors the proposal issue with their OWN GitHub account; the worker only READS.

- **CLI** (`refine kat migrate`, `src/bin/heal.rs::run_kat_migrate` + `src/kat_migrate_client.rs`): signs the filing (old + new ssh keys, unchanged), POSTs it, receives `request_id` + the request digest, then opens the issue **as the user**: with the user's own `gh` auth when present (`gh issue create` — the CLI never stores or forwards that credential), else it PRINTS a prefilled `https://github.com/gist-rs/riir-dao-proposal/issues/new?title=…&body=…` URL for the user to click. Then `attach <issue#>` (or the worker's own discovery) binds the issue to the request.
- **Worker verifies, never creates** (`migrate_tally_reader.rs`, `migrate_routes.rs`, `recovery_tally.rs`): exactly one issue whose body carries the request digest (re-read at release time — an edited body that drops the digest refuses); created after the filing; **author == the OLD account's bound GitHub login when a binding exists** (E11) — a key thief without that GitHub account cannot file; no binding → author recorded, not trusted (the owner 👍 remains the gate; stated gap). Duplicates → refuse (no first-wins guessing). Owner 👍 / old-login 👎 semantics (E3–E3d, E12) UNCHANGED.
- **Read token only:** keep `KAT_MIGRATE_GITHUB_TOKEN` as a READ-ONLY token (unauthenticated reads are 60/h per IP and CF egress IPs are shared — too flaky). REMOVE the reader's fallback to the issues token. A stolen read token writes nothing.
- **Factor ledger — what the SERVER can verify vs what only protects a laptop:**
  - server-verified: ssh dual signature · issue author == bound login · owner 👍 (owner login). GitHub's own login guards (passkey + TOTP 2FA on the owner and filer accounts) are what make the last two strong — the fingerprint/OTP factors live THERE, enforced by GitHub.
  - client-only (protects key custody, NOT the protocol): Touch ID / TOTP gating the CLI (origin-vault / riir-auth Proposal 001 `biometric_login`). Never claimed as a protocol factor.
  - NOT doing: a worker-side TOTP (it needs a shared secret in the worker — the very thing this issue removes).
  - Later (deferred, own issue when picked): a WebAuthn/passkey assertion folded into the filing makes the fingerprint server-verifiable (substrate: riir-chain plan 022 passkey custody, riir-auth Proposal 001).
- **Residual, stated:** a stolen GitHub SESSION (cookie) bypasses passkey/TOTP; the owner's GitHub account is the root — keep its 2FA on passkey + TOTP, sessions short.

## Tasks

- [ ] T0 Owner check (2 min, no agent can do it — CF secrets are write-only): GitHub → Settings → Developer settings → Personal access tokens → find the token in `KAT_MIGRATE_ISSUES_TOKEN`; record its type (classic `ghp_` / fine-grained `github_pat_`), repository access, permissions. Optional probe that creates NOTHING: `curl -s -o /dev/null -w '%{http_code}\n' -X POST -H "Authorization: Bearer $TOKEN" https://api.github.com/repos/<another-repo>/issues -d '{}'` → `422` = it CAN write there (missing title rejected), `403`/`404` = it cannot.
- [ ] T1 CLI: after the filing, open the issue as the user (`gh` if authed) else print the prefilled URL; `attach <issue#>`; tests for both paths + the URL encoding (body carries `request_id` + digest).
- [ ] T2 Worker (riir-dapps `cloudflare/kat-service`): delete the issue CREATOR (`create`, `list_own`, `resolve_bot_login`, `KAT_MIGRATE_ISSUES_TOKEN`, `KAT_MIGRATE_ISSUES_BOT_LOGIN`) and the reader's issues-token fallback; add the attach/verify path (digest, created-after, author == bound login, single match); E-matrix cells: wrong author (bound) → refuse, digest edited away → refuse, duplicate → refuse, unbound → author recorded + owner 👍 still required. Recovery lane (issue 142) gets the same treatment.
- [ ] T3 dao/kat contract text (riir-dao Proposal 001, riir-kat `kat_protocol_migrate`) — "the worker files the issue" → "the filer files; the worker verifies".
- [ ] T4 Devnet rehearsal: one filing via `gh`, one via the printed URL; owner 👍 releases; a wrong-author issue refuses.
- [ ] T5 Owner, AFTER T4 ships: `npx wrangler secret delete KAT_MIGRATE_ISSUES_TOKEN` (apex + `--env devnet`) and REVOKE the PAT on GitHub; set `KAT_MIGRATE_GITHUB_TOKEN` to a read-only token if not already.
- [x] T6 Plan 302: the bot-token item marked SUPERSEDED → this issue; mainnet promotion waits on T5 (same commit as the filing).

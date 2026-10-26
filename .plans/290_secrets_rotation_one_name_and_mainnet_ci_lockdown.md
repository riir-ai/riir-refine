# Plan 290 — Secrets rotation, one-name CF token, and mainnet CI-only lockdown

**Status:** IN PROGRESS — Phase 0+1 COMPLETE, Phase 2 ~70% (deferrals named), Phases 3–5 open. Session record 2026-10-25 (2): ops token wired end-to-end (secret push + deploy `07046fa0` + backfill verify), riir-refine bearers re-minted, `GH_PAT` retired for `SIBLING_REPOS_TOKEN` (`9c66ce72`).

**⚠ Credential-home sweep measured 2026-10-26T22:3xZ (riir-refine-idle-amo, Protocol H + Helius-cleanup attempt):** the local credential picture changed since the 10-25/26 baselines — `seal-remake/.env` (the CF deploy+account pair's home, the guard --live source) MISSING; `riir-deployer/.env`, `riir-dapps/.env`, `riir-reflex/.env` MISSING; `riir-refine/.env` present but 0 lines (held READ_TOKEN/DASHBOARD_TOKEN/GROQ/INFRON/HF-read per Phase 2); `~/Library/Preferences/.wrangler/config/default.toml` (the OAuth session, seen 10-26 ~15:0xZ in the 169 baseline with a refresh token) GONE. Values never read; names only. Consequences: NO Cloudflare credential of any scope is currently reachable from this box — the Helius `HELIUS_WEBHOOK_SECRET_PREVIOUS` cleanup (due: cutover was 10-25, the ≥24 h retry horizon has passed) is BLOCKED on re-homing a credential, and every CF-surface op (guard --live, secret pushes, deploys) fails at auth. Cause unknown (deliberate rotation-lane move vs third-actor wipe); queried the active overflow lane, no answer at record time. Owner decision needed: re-home the CF deploy+account pair (and the OAuth posture — token-only per issue 169 item 5 remains the preferred end state) OR confirm the sweep was deliberate and point the lanes at the new home.

## Context (the audit, 2026-10-25)

Owner directive: all SSH already revoked/rotated; now revoke + roll every API
token and security-critical TOKEN. Rules set by the owner:

1. **One name per credential.** The housekeeping Worker binding `CF_API_TOKEN`
   vs the local var `CLOUDFLARE_OPS_API_TOKEN` was a bug factory — the same
   credential under two names on two sides of a deploy. One name everywhere now.
2. **Any API that can read, or deploy code that can read secrets, is itself a
   secret** and must never reach mainnet. Mainnet deploys come from GitHub
   CI/CD only (protected branch + protected environment), so prod-capable
   tokens exist ONLY as GitHub environment secrets.
3. **Keys/keypairs/private keys are generated inside the Cloudflare service**
   and land in secrets directly — nobody (owner, CI, LLM) ever sees or holds
   them, even in transit through a shell.
4. Already-new (do NOT touch): `GROQ_API_KEY`, `INFRON_API_KEY`,
   `HF_TOKEN` (read-only). Everything else rolls.

### Audit findings (liveness-tested 2026-10-25, values never printed)

| Credential on disk | Where | Verdict |
|---|---|---|
| `CLOUDFLARE_OPS_API_TOKEN` | seal-remake/.env | **DEAD** (verify → error 1000) — line removed |
| `CLOUDFLARE_API_TOKEN` | seal-remake/.env | **DEAD** — line removed |
| `CLOUDFLARE_OPS_API_TOKEN` | riir-reflex/.env | **DEAD** — line removed |
| `CLOUDFLARE_API_TOKEN` | riir-reflex/.env | **DEAD** — line removed |
| `CLOUDFLARE_API_TOKEN` | riir-reflexer/.env | **DEAD** — line removed |
| `CLOUDFLARE_API_TOKEN` | jup-tool/.env | **DEAD** — line removed |
| `GH_RAW_TOKEN` | riir-dapps/.env | **LIVE as `katopz`** — owner must revoke + re-mint |
| `GROQ_API_KEY` / `INFRON_API_KEY` / `HF_TOKEN` | riir-refine/.env, riir-train/.env | NEW per owner — keep |
| `HF_READ_API_KEY` | riir-ai/.env | **consumerless** (no tracked reader) — remove after HF-side check |
| `GLM_API_KEY` | seal-game-editor/.env | roll (not tested — zhipu endpoint) |
| `HELIUS_API_KEY` / `WEBHOOK_TOKEN` / `TEMP_PRI_KEY_1/2` / `TEMP_BURNER_KEY` | jup-tool/.env | roll / delete (rescue keys are exposed by definition) |
| `KAT_CRANK_TOKEN`, `KAT_MINT_TOKEN`, `HELIUS_WEBHOOK_SECRET`, `KAT_WAL_PASSPHRASE`, `KAT_WALLET_SESSION_SECRET`, `KAT_WALLET_OAUTH_CLIENT_SECRET` | riir-dapps/.env, kat-service/.env | roll (self-minted ones re-mint 64-hex; OAuth secret rotates at provider) |
| `KAT_ANCHOR_SEED_HEX`, `CEGEL_DEPOSIT_PAYEE_KEYFILE` | kat-service/.env | **private keys on disk** — Phase 3 |
| `TUNA_CAN_OPENER_KEY`, `TEST_WALLET_KEYPAIR` | riir-dapps/.env | owner-held spender key (decision), devnet test key (delete) |
| `DECISION_TOKEN` | riir-rethink/.env | roll |
| `READ_TOKEN`, `DASHBOARD_TOKEN` | riir-refine/.env | roll (self-minted) |
| `WALLET_SSH_KEY`, `WALLET_SSH_KEY_2` | riir-chain edge-wallet worker secrets **and container env vars** | worst class: private keys readable from container deployment config — Phase 3 |
| `SEAL_META_SEED` | seal-remake authority worker secret | generated locally via `node -e` then pushed — seed transits the machine — Phase 3 |
| `SIBLING_REPOS_TOKEN`, `GH_PAT`, `RELEASE_PAT`, `CF_DEPLOY_API_TOKEN` | GitHub repo/org secrets | roll — fine-grained, ~90 d expiry, loud-fail pattern stays |

GitHub Actions-side note: `ops_dashboard.yml` repo secret `CF_API_TOKEN`
(mapped onto env `CLOUDFLARE_OPS_API_TOKEN`) is renamed in this commit to the
same-name secret — the owner deletes the old repo secret and creates
`CLOUDFLARE_OPS_API_TOKEN` with the analytics token's value.

## Phase 0 — confirm revocation ✅ (this audit)

- [x] All 6 CF tokens found on local disk verified DEAD (tokens/verify → error 1000)
- [x] DEAD token lines removed from seal-remake/.env, riir-reflex/.env,
      riir-reflexer/.env, jup-tool/.env (gitignored local files)
- [x] `riir-refine/.env` already carries no CF tokens (consistent with the revoke)
- [ ] Owner: CF dashboard → API Tokens → confirm the token LIST shows zero
      ACTIVE tokens besides the fresh `CLOUDFLARE_OPS_API_TOKEN` (minted
      2026-10-25) — i.e. nothing stale-active from other boxes

## Phase 1 — one name (LANDED this commit)

- [x] Worker binding renamed `CF_API_TOKEN` → `CLOUDFLARE_OPS_API_TOKEN`
      (worker.js, worker.test.mjs, wrangler.toml comment, verify_token.mjs,
      README, AGENTS.md, root README, .env.example, ops_dashboard.yml,
      gen_dashboard.py disclosure text) — 48/48 housekeeping tests pass
- [x] Owner: `wrangler secret put CLOUDFLARE_OPS_API_TOKEN` (housekeeping)
      with the NEW analytics token — DONE 2026-10-25 (2): value minted by the
      owner (scopes verified live: Analytics GraphQL + Workers AI read),
      pushed via the documented pipe, deployed same window (version
      `07046fa0`) — binding rename ACTIVE; stale `CF_API_TOKEN` binding
      confirmed GONE from the worker (secret list = exactly the two names);
      `/backfill?days=1` completed 18 GraphQL subrequests clean = the token
      works end-to-end (the pre-fix dead hours stay `ok:false` by the
      never-overwrite design; the next hourly cron writes live rows)
- [x] Owner: GitHub riir-refine → Secrets: delete `CF_API_TOKEN`, create
      `CLOUDFLARE_OPS_API_TOKEN` — DONE 2026-10-25 (2): owner set the repo
      secret (timestamp verified); no `CF_API_TOKEN` repo secret existed to
      delete; `GH_PAT` repo secret deleted in the same window (see Phase 2)

## Phase 2 — roll every live credential (owner, dashboards)

- [x] **GH_RAW_TOKEN (LIVE)** — owner regenerated 2026-10-25 (1) as
      `riir-kat-service-score-read` (fine-grained score-history read); wired
      into BOTH kat-service CF envs (devnet + apex) + riir-dapps/.env
- [x] GLM_API_KEY (seal-game-editor) — owner rolled it at the provider;
      new value in seal-game-editor/.env (2026-10-25 (3)). Sole consumer is
      that local .env — no worker secret, no further wiring
- [x] HELIUS_API_KEY — DONE 2026-10-25 (4). The live consumer was never
      jup-tool (archived; empty .env, zero code refs) — it is the kat-service
      `SOLANA_RPC_URL`, which EMBEDS the key: owner minted the new key;
      verified shaped + probed `getHealth` OK on BOTH networks BEFORE any
      push; then rebuilt both URLs (hosts preserved) and pushed the
      `SOLANA_RPC_URL` worker secret to kat-service apex + devnet (2/2
      rc=0); `riir-dapps/.env` + `kat-service/.env` mirrors synced. Old key
      dies with the Helius dashboard revocation
- [x] HELIUS_WEBHOOK_SECRET — OWNER half DONE 2026-10-25 (3): rotated at the
      Helius dashboard (provider-first flip), new value → riir-dapps/.env.
      WORKER half DONE same hour: both kat-service deployments (apex + devnet)
      got the documented secret-first cutover pair —
      `HELIUS_WEBHOOK_SECRET_PREVIOUS` = old (from kat-service/.env, len 29)
      THEN `HELIUS_WEBHOOK_SECRET` = new (len 39), 4/4 pushes rc=0;
      kat-service/.env synced to the new value. The provider-first order had
      webhooks 401ing into retry until the worker push landed — the RPC poll
      + 60 s alarm fallback kept settlement alive through the window.
      FOLLOW-UP: after the Helius retry horizon passes (≥ 24 h), delete
      `HELIUS_WEBHOOK_SECRET_PREVIOUS` on BOTH deployments — the toml's
      "set ONLY during a cutover" rule
- [x] Self-minted bearers re-mint (64-hex, `wrangler secret put` + matching
      local .env) — DONE 2026-10-25 (2) for the five with no cross-service
      pairing: READ_TOKEN + DASHBOARD_TOKEN (riir-refine housekeeping +
      dashboard workers; new READ_TOKEN verified 200 on `/hours`; dashboard
      gate is DORMANT — `AUTH_MODE=public` — so no consumer breaks),
      AUDIT_DRAIN_TOKEN (riir-chain-audit-drain worker; NO client configured
      anywhere on this box — verified deployer/dapps/seal-remake/sdk clean),
      DECISION_TOKEN (riir-rethink/.env only — `wrangler containers list` is
      EMPTY: no live stage exists, the next stage deploy carries the fresh
      `wrangler secret put` per deploy.yaml's own comment). OWNER VERDICT
      2026-10-25 (3): WEBHOOK_TOKEN (jup-tool) is OBSOLETE — SKIP, no
      rotation, strike it from every roll list. DEFERRED with reason:
      KAT_CRANK_TOKEN + KAT_MINT_TOKEN ride the kat-service lane — another
      agent's KAT account-key rotation (`riir-dao/.proposals/001`) is in
      flight on that surface; rotate after it lands. Correction while
      executing: housekeeping does NOT carry KAT_MINT_TOKEN (secret roster =
      exactly `CLOUDFLARE_OPS_API_TOKEN` + `READ_TOKEN`; the mint-anchor POST
      lane is inert there) — only kat-service holds the KAT bearers
- [ ] KAT_WALLET_SESSION_SECRET + KAT_WAL_PASSPHRASE rotate;
      KAT_WALLET_OAUTH_CLIENT_SECRET rotates at the OAuth provider
- [x] GitHub secrets: SIBLING_REPOS_TOKEN (regen 2026-10-25, ~90-day expiry,
      read-only posture — owner done) · RELEASE_PAT (`cargo-heal-release-bot`
      regen 2026-10-25, value → riir-refine repo secret) · maxion-riir-* trio
      DELETED (obsolete project, never used) · duplicate `riir-chain CI sibling
      read` deleted after the sibling regen · **GH_PAT RESOLVED 2026-10-25
      (2), structurally**: the nightly's 8 private sibling checkouts +
      preflight retargeted to the LIVE `SIBLING_REPOS_TOKEN` (one read-only
      sibling token everywhere — the same secret rust.yml already uses;
      `9c66ce72`), the dead `GH_PAT` repo secret DELETED, no new PAT minted —
      the owner's token-page "done" on `riir-clippy-ops-dashboard-nightly`
      lands as a retirement, not a rotation · CF_DEPLOY_API_TOKEN
      (riir-reflexer) still open — owner must mint a fresh deploy-scoped CF
      token for the repo secret
- [ ] HF reconcile: owner says new HF_TOKEN is READ-ONLY, but M3 is the
      publisher for riir-train `data_backup.sh push` (needs WRITE scoped to
      the gist-rs dataset repos). Either the M3 token keeps a narrow write
      scope or pushes move to CI. Owner decision; rigs stay read-only
- [x] riir-ai/.env `HF_READ_API_KEY` — line REMOVED 2026-10-25 (2); workspace
      grep confirmed zero tracked consumers

## Phase 3 — private keys never on disk or in plain env (the "generated on CF" law)

- [ ] **riir-chain edge-wallet** (worst): WALLET_SSH_KEY/WALLET_SSH_KEY_2 are
      worker secrets AND forwarded into CONTAINER env vars
      (edge-wallet-container/index.js) — container env is readable from
      deployment config. Fix: the wallet wasm generates/holds its keypair
      internally (identity_from_seed path exists); worker provisions a seed
      into DO storage on first boot (worker-side keygen, RFC 8032 via
      WebCrypto or the wasm), publishes ONLY NODE_PUBKEY_HEX; the SSH-key
      transport + container env passthrough are removed
- [ ] **seal-remake SEAL_META_SEED**: today generated locally (`node -e
      crypto.getRandomValues`) then `wrangler secret put` — the seed transits
      the owner's shell. Fix: directory worker generates its Ed25519 keypair
      on first boot (WebCrypto, PKCS8 → DO storage), serves/publishes the
      public half for the SEAL_META_PUBKEY pin; `signer_key_id` bumps
      (dir-v1 → dir-v2) so clients re-pin once; README recipe for local
      generation is deleted
- [ ] **kat-service KAT_ANCHOR_SEED_HEX**: same worker-side keygen pattern for
      the anchor signer
- [ ] **TUNA_CAN_OPENER_KEY**: owner decision — it is a SPENDING authority
      (the can-opener signs tuna admin batches). Moving it into a worker
      makes the WORKER the spender, which breaks the owner-explicit-act law;
      recommended: keep owner-held but out of any repo .env (macOS Keychain
      or 0600 file outside the tree), never in CI
- [ ] **CEGEL_DEPOSIT_PAYEE_KEYFILE + ~/.kat/rail_seed_mainnet.hex**: MAINNET
      payee private keys on the M3 disk. Owner decision: sweep the balances
      and rotate to a worker-held deposit wallet (kat-service holding the key
      as a worker secret used only server-side) vs accept owner-held. Money
      migration is required either way — owner-gated, never automated here
- [x] jup-tool TEMP_PRI_KEY_1/2 + TEMP_BURNER_KEY — MOOT per owner verdict
      2026-10-25 (4): jup-tool is ARCHIVED to `/git/obsolete/jup-tool` (the
      rescue is complete); the keys live only in the obsolete copy's .env and
      fold into Phase 5's obsolete-purge (assume every credential there is
      burned; nothing live reads them)
- [ ] TEST_WALLET_KEYPAIR (riir-dapps/.env): devnet-only — delete after
      confirming no remaining consumer
- [ ] AccountKey lineage: after the SSH rotation, confirm
      riir-auth account.json `key_path` still resolves to the new
      ~/.ssh/id_ed25519; re-adopt if the key file was replaced

## Phase 4 — mainnet deploys: CI-only (GitHub environment protection)

- [ ] GitHub Environment `mainnet` per deploy repo (required reviewer = owner,
      restricted to the protected default branch): riir-dapps (kat-service
      apex), seal-remake (authority-container), riir-reflexer
      (reflexer-worker), riir-refine (housekeeping + dashboard workers)
- [ ] Prod deploy token exists ONLY as that environment's secret; deploy jobs
      declare `environment: mainnet` and run on protected-branch push or
      owner-approved dispatch
- [ ] Local machines hold ZERO mainnet-capable tokens (already true — the only
      deploy tokens on disk were the revoked ones); if local dev deploys are
      still wanted, mint a DEVNET-scoped token only
- [ ] seal-remake authority_deploy.sh: devnet keeps local deploy on a
      devnet-scoped token; the mainnet path requires CI provenance (or owner
      + the existing multi-gate refusal chain)
- [ ] Known tension: Actions-minutes budget (the standing spending-limit
      convention) — dispatch-only deploys cost a few minutes each; a
      self-hosted runner is the later owner call if volume grows

## Phase 5 — hygiene

- [ ] obsolete/* .env files (mu-maxage, sxxx-maxage, riir-burner, maxion-*)
      and gist/* .env files (reev, gslides-tools, anypod, anyrag,
      hello-genkit): purge stale secrets from dead projects on disk
      (owner call, low priority — assume every credential in them is burned)
- [ ] One-secret-file law restated: the CF ops plane's tokens live in
      riir-refine/.env only; scripts consume it via the
      `../riir-refine/.env` fallback (cf_billing_split.py already does).
      Never re-duplicate tokens into other repos' .env
- [ ] Re-run this audit's liveness sweep after Phase 2 (every CF token on
      disk must answer DEAD-or-fresh, every live token accounted for)

## Validation

- [x] housekeeping worker tests 48/48 pass post-rename
- [x] gen_dashboard.py parses; ops_nightly.sh `bash -n` clean
- [x] repo-wide residue scan: `CF_API_TOKEN` survives only as the documented
      historical mention of the rename (README L122, ops_nightly.sh unset
      ladder comment) — the unset ladder deliberately keeps `-u CF_API_TOKEN`
      as a defensive strip of the deprecated alias name

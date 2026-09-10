# Issue 088 — spin the kat SERVICE PLANE out of riir-clippy into a NEW PRIVATE REPO (`riir-kat`)

**Status:** OPEN — REVISED v4 2026-09-10: renamed `riir-kat-client` → **`riir-kat`** (owner challenge: "nothing usable server-side?") + wire DTOs & units promoted into a feature-free PROTOCOL tier so server-side Rust consumers never depend on a crate named "client" or enable transport to read their own wire format. Move-set analysis (v2) + new-repo decision (v3) carry over. Implementation not started.

## Naming + the two-sided story (v3 → v4)

v3 named the repo `riir-kat-client`. Wrong on two counts:

1. **The plane carries two-sided vocabulary.** The wire DTOs and units/constants are the KAT network's
   shared language, not client furniture — they exist TODAY in duplicate: client-side Rust here,
   server-side Rust in riir-dapps (burn ingest parses the same `kat:burn:v1:` the client emits),
   synchronized by symmetric golden pins. That golden-pinned duplication is precisely the DRY residue
   this extraction should retire — and a server must not need a crate named "client" for it.
2. **v3's tier model buried the DTOs inside `transport`** — a server wanting its own wire types would
   have to compile ureq/qrcode. Absurd.

| one-sided (client behavior — behind `client`) | two-sided (protocol — feature-free) |
|---|---|
| `kat_meter` (G9d accounting-only), `kat_mining_consent`, `kat_service_config`, `kat_soft_gate`, `kat_redact` policy, `kat_account` facade, 9 HTTP transports, `kat_refill` | wire DTOs: burn ack v3 (`contributed`), payreq frames, lease wire (the MIRRORED versioned contract), wetrun reports, vote intents, faucet request · units: `render_kat`, `FREE_TIER_GRANT_KAT`, charge-per-million price · `kat_service_urls` (host constants), schema/version stamps |
| server NEVER runs these | riir-dapps ledger plane, dao-audit KAT rendering, any Rust verifier consume these |

Identity stays where the fleet already put it: `riir-auth` is THE substrate (both sides); `kat_account`
remains a thin kat-facing facade over it in the `client` tier.

## Repo shape (v4)

`gist-rs/riir-kat` — PRIVATE (Research 003 secrecy holds: private under gist-rs + `publish = false`),
single crate, leaf: **riir-auth (optional) is the only sibling dep — zero katgpt-core, zero game/chain/dapps.**

Features:
- **feature-free = PROTOCOL tier**: `kat_protocol` (wire DTO types extracted per-area from the client
  modules — the types move, the logic stays; goldens move WITH the types), `kat_units` (`render_kat`
  + `FREE_TIER_GRANT_KAT` + pricing const), `kat_service_urls` (`DEFAULT_SERVICE_URL`/`FALLBACK_SERVICE_URL`
  lifted from `kat_lease_client`), version stamps. Server consumer: `default-features = false` →
  protocol only, no optional dep compiled.
- `account_key = ["dep:riir-auth", "dep:ed25519-dalek"]` → `kat_account`.
- `client = ["account_key"]` → `kat_meter`, `kat_mining_consent`, `kat_service_config`, `kat_soft_gate`,
  `kat_redact` (ungated-in-root today; root maps them to non-default so root's default build stays clean).
- `transport = ["client", "dep:ureq", "dep:qrcode", "dep:ssh-key", "dep:ed25519-dalek"]` → the 9 HTTP
  clients (`kat_payreq_client`, `kat_burn_client`, `kat_stake_client`, `kat_vote_client`,
  `kat_lease_client`, `kat_account_client`, `kat_consent_client`, `kat_wetrun`, `kat_refill`).
- `default = ["client"]`.

Client-only modules (machine-local, ungated in root today): `kat_mining_consent`, `kat_service_config`,
`kat_soft_gate` — they land in the `client` tier.

## Stays in riir-clippy (verified healer imports — v2 grep)

`kat_promotion` (domain/draft/memory/score_bench — the score-bench gate), `kat_sync_payload` +
`kat_pending_estimator` (traj_store). Their `use crate::kat_redact::…` paths keep resolving via
root re-exports.

## Landing steps

1. Owner creates private `gist-rs/riir-kat` (manual, free tier); local init, `develop` default.
2. Fleet scaffold: root `BOUNDARY.md` (owns: KAT protocol + client plane; does-not-own: the LEDGER/settlement
   SERVER semantics = riir-dapps, the healer pipeline, identity substrate = riir-auth), `AGENTS.md`,
   `HISTORY.md`, `.issues/` + `.highwater`, `ci_feature_guard.sh` + `standalone_dep_gate.sh`
   (riir-auth templates), `rust.yml` MAIN-ONLY + dispatch (2026-09-09 spending convention, no schedules).
3. Move modules into the tier/feature model; **extract wire DTO types from the client modules into
   `kat_protocol`** (logic stays; golden vectors move with the types); lift `render_kat`/constants into
   `kat_units`; lift URL consts into `kat_service_urls` (lease re-exports for path compat inside the crate).
4. riir-clippy becomes a consumer: `riir-kat = { path = "../riir-kat", default-features = false }`
   non-optional (ungated-in-root modules need `client`… DECISION: root default build must stay free of
   them? NO — they are ungated in root TODAY; map root's non-gated modules to `riir_kat/client` being
   ON in root default via `kat_service`… resolves to: root `default` does NOT enable `client`; the three
   ungated root modules become `#[cfg(feature = "kat_service")]` re-exports — an ACCEPTED, documented
   behavior change (they are service-plane modules; compile-to-nothing in default matches every other
   kat module). Update any default-build test expectations.
   `kat_service = ["riir_kat/client"]` (dep-light preserved), `kat_transport = ["riir_kat/transport", "kat_service"]`.
   Root lib.rs re-exports every moved module at its existing path — `bin/heal.rs` (~40 refs), keepers,
   `tests/*_cli.rs`, examples compile unchanged. Root dep rows removed: `riir-auth`, `ssh-key`, `qrcode`;
   kept: `ureq` (cf_workers_ai/groq_first), `ed25519-dalek` (kat_sync_payload signs).
5. Boundary bookkeeping: riir-clippy BOUNDARY.md allowlist +1 row (`riir-kat`, kat_service/kat_transport
   only) + Owns updated; `standalone_dep_gate.sh` set +1; katgpt-rs `repo_set.txt` + AGENTS.md counts
   17 → 18 (load-bearing pins — edit by hand); 4090 box joins the sync set.
6. Validation: `CARGO_TARGET_DIR=/tmp/riir-kat-spinout` — NEVER bare `cargo build --release` in
   riir-clippy (00:21 lesson). New repo: check/test/clippy per tier INCLUDING the server-consumption
   lane (`--no-default-features`: protocol-only, proves zero optional deps compiled). riir-clippy:
   default + kat_service + kat_transport + clippy + keeper paths. Fresh-`git worktree` commit-state
   compile in both repos. Pre-flight `git status -sb` (Plan-023 WIP landed as cf236f1a; re-verify);
   lib.rs blob-commit if dirty; NAMED files only.
7. Docs: both AGENTS.md notes; katgpt-rs AGENTS.md §Repo count + BOUNDARY.md workspace list; ref hashes.

## Explicit follow-ups (NOT this landing)

- **riir-dapps may adopt `riir-kat` protocol** for its server-side burn/payreq/lease Rust types,
  retiring the symmetric golden duplication — needs a dapps BOUNDARY.md allowlist row (dapps is
  chain-side-only by contract; owner decision). Until then goldens keep both sides honest.
- dao-audit (riir-dao) may swap its hand-pinned units/goldens for `riir-kat` protocol — same
  owner-decision pattern (dao is deliberately dep-light).

## Tasks

- [ ] Owner: create private `gist-rs/riir-kat` on GitHub
- [ ] Local init + fleet scaffold (BOUNDARY/AGENTS/HISTORY/.issues/guards/CI MAIN-ONLY)
- [ ] Protocol tier: extract wire DTO types + units + URLs from the client modules (goldens move with types)
- [ ] Client tier: move the behavior modules; feature graph per spec above
- [ ] riir-clippy consumer rewiring (dep rows, feature defs, lib.rs re-exports, delete moved files, default-build expectation updates)
- [ ] Boundary bookkeeping (both BOUNDARY.md rows, standalone_dep_gate, repo_set.txt, counts 17→18, 4090 sync)
- [ ] Full validation per tier incl. `--no-default-features` server lane + fresh-worktree commit-state compiles
- [ ] Commit `feat: issue 088 ...` both repos on develop, push, ref hashes here

## Non-goals

- No healer-coupled moves (kat_promotion / kat_sync_payload / kat_pending_estimator stay root).
- No account_key absorption (riir-auth stays THE identity substrate).
- No forced riir-dapps/riir-dao adoption (follow-ups above — goldens remain the contract until the owner flips them).
- No public publishing ever; no katgpt-core dep; no API change at the riir-clippy surface.

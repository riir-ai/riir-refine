# Issue 088 — spin the kat CLIENT PLANE out of riir-clippy into a NEW PRIVATE REPO (`riir-kat-client`)

**Status:** OPEN — REVISED v3 2026-09-10: workspace-member plan replaced by new-repo spin-out (owner: consumers must not clone the whole healer repo). v2's move-set + tier analysis carries over unchanged; destination + bookkeeping change. Implementation not started.

## Why a repo, not a member crate (v1 → v2 → v3 history)

- v1 (minimal slice): superseded — owner confirmed multiple consumers; per-module asks repeat the same churn.
- v2 (workspace member `riir-clippy/crates/kat-client`): superseded — a consumer path-depping into riir-clippy must clone the ENTIRE healer repo (corpus tables, healing domains, workers). Compile isolation ≠ source isolation. Also flips riir-clippy's boundary role from developer-tooling to substrate, which its BOUNDARY.md domain test forbids.
- v3 (this): **one new private repo** holding the whole 15-module client plane. Precedent: `riir-auth` (spun out when multiple parties needed account_key; riir-clippy became a consumer, Issue 081). `kat_account` is a facade over riir-auth/account_key — the plane belongs in a leaf beside its substrate, not inside its first customer.

## The move set (15 modules → `riir-kat-client`, one repo)

Healer-coupled and STAY in riir-clippy (verified grep, v2):

| stays (root) | healer imports |
|---|---|
| `kat_promotion` | `domain::HealDomain/HealEntry`, `draft::clippy_oracle`, `memory::EvidenceTier`, `score_bench::*` — it IS the score-bench promotion gate |
| `kat_sync_payload` | `traj_store::{shape_hash, TrajRowDoc}` + `kat_redact` |
| `kat_pending_estimator` | `traj_store` + `kat_redact` + `kat_sync_payload` |

Moves (zero healer imports — only each other + std/serde/serde_json/blake3/ureq/qrcode/ssh-key/ed25519-dalek):

- Tier 0 feature-free (UNGATED in root today, stay so): `kat_mining_consent`, `kat_service_config`, `kat_soft_gate`, + NEW `kat_service_urls` (`DEFAULT_SERVICE_URL`/`FALLBACK_SERVICE_URL` lifted out of `kat_lease_client`; lease re-exports them — paths unchanged).
- `service` feature: `kat_units` (render_kat + FREE_TIER_GRANT_KAT lifted from kat_payreq_client/kat_meter), `kat_meter`, `kat_redact`.
- `account_key` feature: `kat_account` (dep riir-auth/account_key — the facade keeps delegating; NO absorption of account_key itself).
- `transport` feature: the nine HTTP clients — `kat_payreq_client`, `kat_burn_client`, `kat_stake_client`, `kat_vote_client`, `kat_lease_client`, `kat_account_client`, `kat_consent_client`, `kat_wetrun`, `kat_refill` (dep ureq, qrcode, ssh-key, ed25519-dalek; implies service + account_key).

New repo dep graph: **riir-auth (optional, account_key) is the ONLY sibling dep — zero katgpt-core, zero game/chain/dapps**. Pure leaf. Downstream: riir-clippy + the owner's other app(s) + any future consumer. No cycles.

## Landing steps

1. **New repo** `gist-rs/riir-kat-client` (PRIVATE, under gist-rs — source secrecy per Research 003 holds: "anything riir-* is internal"; also `publish = false`). Owner creates the GitHub repo (manual — free-tier limit); local `git init` + `develop` default branch.
2. Scaffold the fleet-standard set: root `BOUNDARY.md` (owns: kat service client plane; allowlist: riir-auth/account_key optional ONLY; does-not-own: healer pipeline, wire SERVER half = riir-dapps, promotion/payload machinery), `AGENTS.md`, `HISTORY.md`, `.issues/` + `.highwater`, `scripts/ci_feature_guard.sh` + `scripts/standalone_dep_gate.sh` (riir-auth's are the template), `.github/workflows/rust.yml` MAIN-ONLY + workflow_dispatch (the 2026-09-09 spending-limit convention; no schedules).
3. Move the 15 modules + their tests into the 4-tier feature model; add `kat_service_urls`; the golden wire vectors (lease §golden, faucet, burn ack) move WITH their modules.
4. riir-clippy becomes a CONSUMER: sibling path dep `riir-kat-client = { path = "../riir-kat-client", default-features = false }` (non-optional — tier 0 is ungated in every root build); `kat_service = ["riir_kat_client/service", "riir_kat_client/account_key"]`; `kat_transport = ["riir_kat_client/transport", "kat_service"]`; root `lib.rs` re-exports every moved module at its existing path — `bin/heal.rs` (~40 refs), the 3 keeper modules (`use crate::kat_redact` resolves via re-export), `tests/*_cli.rs`, examples: ALL compile unchanged. Remove root dep rows: `riir-auth`, `ssh-key`, `qrcode` (verified unused post-move); KEEP `ureq` (cf_workers_ai/groq_first) + `ed25519-dalek` (kat_sync_payload signs).
5. Boundary bookkeeping, same commit: riir-clippy `BOUNDARY.md` allowlist +1 row (riir-kat-client, condition: kat_service/kat_transport only) + Owns updated; riir-kat-client's own BOUNDARY.md is the new repo's contract. `scripts/standalone_dep_gate.sh` (riir-clippy) escaping-dep set +1. NOTE the measured-graph check: ci_boundary_contract.sh derives from BOUNDARY.md + .git — the new repo joins the population automatically on first run.
6. Workspace count pins (load-bearing — a count is a claim): katgpt-rs `scripts/repo_set.txt` + AGENTS.md repo-count prose (17 → 18) + any `*_floors.txt` membership rows that pin the workspace set (the drift-sweep family derives population, but membership pins must be edited by hand). Same for the 4090 box: the new repo joins the both-boxes sync set (no divergence).
7. Validation: `CARGO_TARGET_DIR=/tmp/kat-client-spinout` — NEVER bare `cargo build --release` in riir-clippy (00:21 global-binary-symlink lesson). New repo: check/test per feature tier (tier0 / +service / +account_key / +transport) + clippy `-D warnings` + standalone_dep_gate + guard. riir-clippy: default + kat_service + kat_transport checks, member-path tests, clippy; keeper-module paths resolve. Final: commit-state compile in a fresh `git worktree` per repo.
8. Pre-flight (sibling agents): `git status -sb` in riir-clippy before touching move-set files (`kat_lease_client.rs`, `kat_refill.rs` were Plan-023 WIP — now landed as cf236f1a, re-verify); lib.rs hunks committed via the blob technique if the tree is dirty again; stage NAMED files only.
9. Docs: riir-clippy AGENTS.md feature-list note (kat_service/kat_transport now activate the sibling); katgpt-rs AGENTS.md §Repo count + BOUNDARY.md workspace list +1 (18 repos, product/distillation set stays 7); ref the spin-out commit hash here.

## Tasks

- [ ] Owner: create private `gist-rs/riir-kat-client` on GitHub (manual; free tier)
- [ ] Local init + fleet scaffold (BOUNDARY/AGENTS/HISTORY/.issues/guards/CI MAIN-ONLY)
- [ ] Move 15 modules + tests + goldens into the 4-tier model + kat_service_urls
- [ ] riir-clippy consumer rewiring (dep rows, feature defs, lib.rs re-exports, delete moved files)
- [ ] Boundary bookkeeping (both BOUNDARY.md rows, standalone_dep_gate, repo_set.txt, AGENTS counts 17→18, 4090 sync)
- [ ] Full validation per tier (isolated target dirs) + fresh-worktree commit-state compiles
- [ ] Commit `feat: issue 088 ...` both repos on develop, push, ref hashes here

## Non-goals

- No healer-coupled module moves (kat_promotion / kat_sync_payload / kat_pending_estimator stay root — table above).
- No account_key absorption (riir-auth stays THE identity substrate; the facade keeps delegating).
- No public publishing ever (private repo, publish = false); no katgpt-core dep (the leaf stays clean).
- No API change at the riir-clippy surface — re-export contract keeps every existing path bit-identical.

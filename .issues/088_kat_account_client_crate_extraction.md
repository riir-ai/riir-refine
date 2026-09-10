# Issue 088 — extract the kat CLIENT PLANE into a workspace member crate (`crates/kat-client`)

**Status:** OPEN — REVISED 2026-09-10 (same day): scope widened from the minimal `kat_account` slice to the full client plane (15 of 18 modules) after owner challenge "why one by one?". Analysis complete; implementation not started.

## Why

Second Rust consumers for the kat service plane are REAL (owner-confirmed), and slicing per-module
would repeat the same wiring churn (workspace, features, CI lanes, BOUNDARY.md) N times. Measured
coupling supports one pass: 15 of 18 `kat_*` modules import ZERO healer internals — only each other
plus std/serde/serde_json/ureq/qrcode/ssh-key/ed25519-dalek. Three modules are genuinely
healer-coupled and STAY (moving them would drag the healer pipeline into the client crate — the
exact anti-goal):

| stays (root) | healer imports (verified grep) |
|---|---|
| `kat_promotion` | `crate::domain::{HealDomain, HealEntry}`, `crate::draft::clippy_oracle::ClippyOracle`, `crate::memory::EvidenceTier`, `crate::score_bench::{run_bench, run_bench_with, BenchFixture, BenchScore}` — it IS the score-bench promotion gate |
| `kat_sync_payload` | `crate::traj_store::{shape_hash, TrajRowDoc}` (the healer's trajectory store) + `kat_redact` |
| `kat_pending_estimator` | `crate::traj_store` + `kat_redact` + `kat_sync_payload` |

## The move set (15 modules → `crates/kat-client`, package `kat-client`, lib `kat_client`)

Tier 0 — feature-free, always compiled (these are UNGATED in root today and must stay so):
`kat_mining_consent`, `kat_service_config`, `kat_soft_gate`, + NEW `kat_service_urls`
(`DEFAULT_SERVICE_URL` + `FALLBACK_SERVICE_URL` consts lifted out of `kat_lease_client` so the
ungated config tier can name the default without transport; the lease module re-exports the consts
so every existing path is unchanged).

`service` feature — `kat_units` (render_kat + FREE_TIER_GRANT_KAT, lifted from
kat_payreq_client/kat_meter), `kat_meter`, `kat_redact`.

`account_key` feature — `kat_account` (`dep:riir-auth` account_key substrate + `dep:ed25519-dalek`).

`transport` feature — the nine HTTP clients: `kat_payreq_client`, `kat_burn_client`,
`kat_stake_client`, `kat_vote_client`, `kat_lease_client`, `kat_account_client`,
`kat_consent_client`, `kat_wetrun`, `kat_refill` (`dep:ureq`, `dep:qrcode`, `dep:ssh-key` —
stake's `fresh_reference` uses it, `dep:ed25519-dalek`, implies `service` + `account_key`).

Root feature rewiring:
- dep `kat-client = { path = "crates/kat-client", default-features = false }` — NON-optional (tier 0
  is ungated in every root build; zero new external packages: serde/serde_json already non-optional).
- `kat_service = ["kat_client/service", "kat_client/account_key"]` — root's own `riir-auth` row and
  `ssh-key` row are REMOVED (verified: after the move nothing in root uses `ssh_key::`; the
  ed25519-dalek row STAYS — `kat_sync_payload` signs).
- `kat_transport = ["kat_client/transport", "kat_service"]` — root `qrcode` row REMOVED; root `ureq`
  row STAYS (`cf_workers_ai` + `groq_first` use it; same version unifies with the member's).

Path contract — re-export, don't duplicate: root `lib.rs` re-exports every moved module at its
existing path (`pub use kat_client::kat_mining_consent;` ungated; the rest behind their current
`kat_service`/`kat_transport` cfg). Every existing call site — `bin/heal.rs` (~40 refs),
`kat_promotion`/`kat_sync_payload`/`kat_pending_estimator` (`use crate::kat_redact::…` still
resolves), `tests/*_cli.rs`, examples with `required-features` — compiles unchanged.

Root keeps: `kat_promotion`, `kat_sync_payload`, `kat_pending_estimator` (table above).

## Same-commit housekeeping

- `BOUNDARY.md`: Owns gains the member-crate line; riir-auth row condition becomes "via member
  crate `kat-client` (account_key), `kat_service` only"; allowlist unchanged (same sibling edges).
- AGENTS.md: one note under the feature list (kat_service/kat_transport now activate member features).
- CI (Gate Coverage rule — a suite nobody runs is documentation): `scripts/ci_feature_guard.sh` gains
  `cargo test -p kat-client --lib` (tier 0) + a `--features service,account_key,transport` lib lane;
  verify `scripts/standalone_dep_gate.sh` tolerates the member manifest (adds `../../../riir-auth` +
  `../../../riir-ai`? NO — member deps are riir-auth only among siblings; same escaping-edge set).
- Re-pin any root lib-test count floors (a large test population moves: account 11, consent 9, plus
  meter/redact/config/stake/lease-goldens/burn/vote/payreq/wetrun/soft_gate suites).
- Validation with `CARGO_TARGET_DIR=/tmp/kat-client-extract` — NEVER a bare `cargo build --release`
  (the 00:21 global-binary-symlink lesson): root check default + `--features kat_service` +
  `--features kat_transport`; member checks per tier; clippy both; then a commit-state compile in a
  fresh `git worktree` to prove the committed tree (sibling WIP is live in this worktree:
  `heal.rs`, `kat_lease_client.rs` (!), `kat_refill.rs`, `kat_wetrun.rs`, `lib.rs` + untracked
  `kat_service_config.rs` — CONFLICT WARNING: the sibling's Plan 023 WIP edits `kat_lease_client.rs`
  and `kat_refill.rs`, two files in the move set. Before landing: re-check `git status -sb`; either
  wait for their commit, or rebase-scope this landing after theirs; lib.rs commits via the blob
  technique (HEAD content + only my hunks); stage NAMED files only.

## Tasks

- [ ] Pre-flight: `git status -sb` + `git --no-pager diff` on the sibling-WIP move-set files; coordinate or defer to their landing
- [ ] Create `crates/kat-client` (Cargo.toml with the four-tier feature model + lib.rs + 15 module moves + the new `kat_service_urls`)
- [ ] Root Cargo.toml: `[workspace]` + member dep + feature rewiring + remove root `riir-auth`/`ssh-key`/`qrcode` rows (keep ureq, ed25519-dalek)
- [ ] Root lib.rs re-exports (blob-commit protocol for lib.rs); delete moved files from `src/`
- [ ] Root keepers (`kat_promotion`, `kat_sync_payload`, `kat_pending_estimator`): confirm `use crate::kat_redact` paths resolve via re-exports
- [ ] BOUNDARY.md + AGENTS.md updates
- [ ] ci_feature_guard.sh member lanes + standalone_dep_gate.sh check + count-floor re-pins
- [ ] Isolated-target-dir validation (root default / kat_service / kat_transport; member per-tier; clippy; fresh-worktree commit-state compile)
- [ ] Commit `feat: issue 088 ...` on develop, push, ref hash here

## Non-goals

- No healer-coupled module moves (table above) — they follow only if their substrate (traj_store/score_bench) ever moves.
- No new repo, no public publishing (source secrecy, Proposal 003); member is `publish = false`.
- No API changes — the re-export contract keeps every existing path bit-identical.

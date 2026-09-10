# Issue 088 — extract the kat account plane into a workspace member crate (`crates/kat-account`)

**Status:** OPEN — analysis + scope complete (2026-09-10 session); implementation paused by owner. No code moved yet.

## Why (the trigger that fired)

The standing verdict on extracting the `kat_*` service plane was DEFER-until-a-real-second-consumer
(consulted 2026-09-10: single-crate repo, all kat modules opt-in behind `kat_service`/`kat_transport`,
zero second Rust consumers, cross-repo contracts pinned by goldens — riir-dao/dapps precedent).
The owner then named a real second application that needs `src/kat_account.rs`. That is the documented
trigger, so the minimal account-plane slice is now sanctioned for extraction. Full-plane extraction
(lease/burn/vote/stake/payreq/consent/wetrun clients) stays deferred until each has its own consumer.

## Measured dependency surface (2026-09-10, this tree)

| module | deps outside itself |
|---|---|
| `kat_mining_consent` | std + serde/serde_json ONLY (ungated in every feature build today) |
| `kat_account` | `riir_auth::account_key` (AccountKey + re-exports `account_id_for`, `short_id`, `AccountState`, `LoginOutcome`, `ACCOUNT_CHANGELOG`, `ACCOUNT_VERSION`) · `ed25519_dalek::SigningKey` (`signing_key()`) · `crate::kat_meter::FREE_TIER_GRANT_KAT` · `crate::kat_payreq_client::render_kat` (×3 incl. tests) · `crate::kat_mining_consent::default_dir` (legacy dir) · `ssh_key` in TESTS only (`signing_key_matches_the_raw_seed_route`) |
| `render_kat` (kat_payreq_client L361) | 5-line pure formatter — movable |
| `FREE_TIER_GRANT_KAT` (kat_meter L293) | single `pub const u64 = 100_000_000` — movable |

Healer-coupled modules that STAY in the root: `kat_promotion` (imports `domain`/`draft::clippy_oracle`/`memory`/`score_bench`),
`kat_pending_estimator` + `kat_sync_payload` (import `traj_store`), `kat_soft_gate`, `kat_refill`, `kat_meter` logic, all HTTP clients.

## Plan (the split line)

1. New workspace member `crates/kat-account` (package `kat-account`, lib `kat_account`), modules:
   - `kat_mining_consent` — verbatim move (tests ride along).
   - `kat_units` — NEW small module: `render_kat` + `FREE_TIER_GRANT_KAT` (the one shared KAT presentation/unit pair).
   - `kat_account` — moved; `crate::kat_meter::` → `crate::kat_units::`, `crate::kat_payreq_client::` → `crate::kat_units::`; `crate::kat_mining_consent::` unchanged.
   - Member features: `default = ["account_key"]`; `account_key = ["dep:riir-auth", "dep:ed25519-dalek"]` (same profiles as root today: riir-auth `default-features = false`, ed25519-dalek `default-features = false`); dev-dep `ssh-key = "0.6"` for the signing pin test.
2. Root `Cargo.toml`: add `[workspace]` + `members = ["crates/kat-account"]`; dep `kat-account = { path = "crates/kat-account", default-features = false }` — NON-optional, because the mining-consent module must stay ungated (consent-module rule: compiled in EVERY feature build). Root's own optional `riir-auth` dep row is REPLACED by the member edge: `kat_service` gains `"kat_account/account_key"` and drops `"riir-auth/account_key"` + `"dep:ssh-key"`... CAREFUL: root's `ssh-key` optional row may still serve other root modules — grep `ssh_key::` across root src before removing anything. Default build must gain ZERO new external packages (serde/serde_json already non-optional; riir-auth stays out of default — preserves the BOUNDARY.md dep-light invariant).
3. Root `src/lib.rs`: `pub use kat_account::kat_mining_consent;` (ungated) and `#[cfg(feature = "kat_service")] pub use kat_account::kat_account;` — re-export, don't duplicate: every existing path (`crate::kat_account::*`, `riir_clippy::kat_account::*`, `crate::kat_mining_consent::*`) keeps resolving; ~40 call sites in `bin/heal.rs` etc. unchanged.
4. Root `kat_meter.rs` / `kat_payreq_client.rs`: replace the local const/fn with `pub use kat_account::kat_units::{FREE_TIER_GRANT_KAT}` / `{render_kat}` — root paths unchanged; existing root tests of `render_kat` keep compiling (they now test the re-export too).
5. Update in the SAME commit: `BOUNDARY.md` (Owns mentions the member crate; riir-auth row condition now "via member crate kat-account, kat_service only") + AGENTS.md feature-list note.
6. CI lanes (the Gate Coverage rule — a suite nobody runs is documentation): add `cargo test -p kat-account --lib` (20 tests move out of the root lib count: 11 kat_account + 9 kat_mining_consent) to `scripts/ci_feature_guard.sh`; verify `scripts/standalone_dep_gate.sh` tolerates the new member manifest (it pins the escaping-dep set — the member adds `../../../riir-auth`, the SAME sibling edge, possibly re-counted).
7. Validation with `CARGO_TARGET_DIR=/tmp/kat-account-extract` isolation — NEVER a bare `cargo build --release` in this repo (the 00:21 lesson: it overwrites the global binary symlink target with a default-features build): `cargo check` (default) · `cargo check --features kat_service` · `cargo test -p kat-account --lib` · root lib test count re-pinned if any floor exists · clippy both crates.
8. Release engineering: release feature set unchanged (root features); same resolved external crates → `THIRD_PARTY_LICENSES.md` + `binary_size_gate.sh` unaffected; member is `publish = false`, source secrecy holds.

## Shared-worktree commit protocol (sibling WIP live)

At scope time the worktree carried sibling WIP: `M src/bin/heal.rs kat_lease_client.rs kat_refill.rs kat_wetrun.rs lib.rs` + untracked `src/kat_service_config.rs` (riir-dapps Plan 023 domain swap). Protocol:
- Touch NOTHING in that set except `src/lib.rs`.
- lib.rs: edit on top of their working file, but commit a BLOB built from HEAD's lib.rs + only my hunks (`git hash-object -w` + `git update-index --cacheinfo`) so their `pub mod kat_service_config;` line stays uncommitted.
- Stage NAMED files only (never `-A`); run `scripts/staged_set_audit.py` before the commit; re-scan `.highwater` + `git status -sb` together at write time (a rebase changes the timeline).

## Tasks

- [ ] Create `crates/kat-account` member (Cargo.toml + lib.rs + the three modules, tests moved with files)
- [ ] Root Cargo.toml: `[workspace]` + member dep + `kat_service` rewiring (`kat_account/account_key`); grep-and-settle the root `ssh-key` row
- [ ] Root lib.rs re-exports (blob-commit protocol); kat_meter/kat_payreq_client re-export the moved units
- [ ] BOUNDARY.md + AGENTS.md same-commit updates
- [ ] ci_feature_guard.sh member test lane + standalone_dep_gate.sh check
- [ ] Isolated-target-dir validation (check default, check kat_service, member lib tests, clippy) + count-pin re-pin if the guard pins lib test counts
- [ ] Commit `feat: issue 088 ...` on develop, push, ref hash here

## Non-goals

- No full kat-client plane extraction (lease/burn/vote/stake/payreq/wetrun/consent clients stay root-gated until each has a consumer).
- No new repo, no public publishing (source secrecy, Proposal 003).
- No API changes: the re-export contract keeps every existing path bit-identical.

# Plan 025 — KAT Phase 0 T0.1: account key + `login`/`account` (local view)

Executes [Proposal 005](../.proposals/005_kat_heal_service_network.md) **Phase 0 T0.1** — the account-key
substrate + the two identity CLI commands. Opened 2026-08-26 after [Plan 024](024_kat_phase0_charge.md)
landed the charge consumer. Phase 0 scope is explicit in the rollout sketch: **local view, no cloud** —
the server half of the grant (the actual mint) arrives with Phase 1 (T1.2).

## The decision (T0.1 — recorded)

**Identity = an Ed25519 keypair in OpenSSH format; account id = BLAKE3(pubkey), full 32 bytes, hex.**

- The proposal names the exact pattern ("shipped three times": `riir-chain/src/ssh_key.rs`, `GmKeySetup`,
  `avatar_crypto`'s registry). Those live in repos riir-clippy must NOT depend on (boundary: the allowlist
  has no riir-chain/riir-game-sdk/riir-mmorpg-examples edge), so T0.1 takes the *shape* and the *crate*
  (`ssh-key`, the RustCrypto crate all three use transitively) — not a code dep. Third-party crates are not
  boundary-gated (the `ureq` precedent, Cargo.toml L46-48).
- **One new dep, one feature edge:** `ssh-key = "0.6"` (`ed25519` + `std` + `getrandom`) under the existing
  `kat_service` feature. ssh-key's `ed25519` feature already implies `dep:ed25519-dalek` + `rand_core`, so
  generation (`PrivateKey::random(&mut OsRng, Algorithm::Ed25519)`), OpenSSH serialization, parsing, and
  pubkey extraction all come from the one crate. A DIRECT `ed25519-dalek` dep (for `SigningKey` — sync
  payload signatures) arrives with T0.3, not before (minimal dep surface per repo discipline).
- **Account id derivation pinned against an independent recompute** (the riir-dapps lesson: "pin every
  reimplemented derivation against its source") — the test computes `blake3::hash(pubkey).to_hex()` inline
  and asserts equality with `account_id_for`.

## Design

1. **Location — machine-GLOBAL, not repo-local.** An account is a user identity shared by every repo this
   host heals; the meter/charge ledgers stay repo-local (Plans 023/024 — per-repo burn). Resolution mirrors
   the `default_memory_path` cache-dir precedent at CONFIG tier (a key is identity, not disposable cache):
   `RIIR_HEAL_KAT_ACCOUNT_DIR` → `$XDG_CONFIG_HOME/riir-heal` → `~/.config/riir-heal` →
   `%APPDATA%\riir-heal` → temp fallback.
2. **Files in the account dir:** `kat_id_ed25519` (OpenSSH private key, unencrypted, `0600` on unix),
   `kat_id_ed25519.pub` (OpenSSH public key), `kat_account.json` (the versioned record).
3. **Record** (`AccountState`): `{account_version, account_id, created_at_unix, grant_kat}` — versioned
   with `ACCOUNT_VERSION`/`ACCOUNT_CHANGELOG` (the meter/charge discipline: no silent shape change). The
   record is a PROJECTION of the key: missing/corrupt/unknown-version → re-derive from the key file + warn
   (the charge-recovers-from-the-meter precedent — the key is the authority, exactly as the meter is the
   token authority for the charge ledger).
4. **Login semantics** (idempotent, three outcomes):
   - key exists → `Existing`: load, re-derive id, recover the record if needed. Never regenerates a key.
   - `--key <path>` given + no key yet → `Imported`: parse the OpenSSH private key at `<path>` (encrypted
     keys REFUSED with the riir-chain `SshKeyError::Encrypted` remedy text), copy into the account dir,
     derive the account id from the imported pubkey.
   - neither → `Generated`: fresh Ed25519 keypair, written with the pub sidecar.
   - Grant: `grant_kat = FREE_TIER_GRANT_KAT` recorded at first record write — the LOCAL PROJECTION of the
     §KAT free tier ("granted once per account key at login"). Accounting-only (G9d): nothing here mints
     or spends; the server mints at Phase 1 and "once per key" is server-enforced there.
5. **`account` command (local view):** load → **verify** (recompute BLAKE3(pubkey-from-key-file) ==
   record's `account_id`; mismatch = tamper evidence, loud error + exit 1) → print account id (short
   `9f3a…`-style + full hex), key path, grant projection, the Phase-0 honesty line (server ledger arrives
   with T1.2). When a repo-local charge ledger exists (path arg or cwd via `resolve_heal_root`), ALSO
   print this repo's local burn + `grant − charged` balance projection + the ≤10M warn — with the explicit
   caveat that the projection is per-repo until the server aggregates. No `$` figures (caveats #12/#15).
6. **CLI shape:** first-positional subcommands (`cargo heal login [--key <path>]`,
   `cargo heal account [path]`) parsed unconditionally; a `kat_service`-off build errors cleanly (the
   `--fix-compile`-off precedent) instead of treating `login` as a scan path.

## Tasks

- [x] T1 `Cargo.toml`: `ssh-key` optional dep + `kat_service = ["dep:ssh-key"]` (+ comment update — the
  T0.5 comment anticipated "qrcode/HTTP arrive with the later account/sync tasks"; this is the account one)
- [x] T2 `src/kat_account.rs`: `ACCOUNT_VERSION`/`ACCOUNT_CHANGELOG` + `account_id_for` + `AccountState` +
  `KatAccount` (default_dir/login/login_import/load/verify/summary) + `LoginOutcome` + unit tests
- [x] T3 `src/lib.rs`: module decl + `kat_service` re-export extension
- [x] T4 `src/bin/heal.rs`: subcommand parsing (`login`/`account` first-positional + `--key`) + dispatch
  (feature-gated handler + clean off-error) + `run_kat_login`/`run_kat_account` + usage lines
- [x] T5 `tests/kat_account_g9.rs`: G9-style fixtures — id purity (G9a), version stamp on the record
  (G9b), grant projection accounting-only + idempotent re-login (G9d), the account-grant × repo-charge
  balance-projection composition (what `account` prints), key↔record tamper evidence (3 distinct
  properties: held-state-vs-swapped-key, fresh-load-follows-key, stale-sidecar)
- [x] T6 validation: see the record below — all green, incl. bin e2e on M3
- [x] T7 `cargo heal --fix --write --verify` over the touched files: 2 mechanical edits landed
  compile-gated in `src/kat_account.rs` (single_match_else + map_unwrap_or→map_or, Certified);
  1 manual fix (unnecessary_debug_formatting — newer lint, not in the corpus); 2 needless_borrow
  manual fixes in the new test file
- [x] T8 `.plans/.highwater` 24 → 25; Proposal 005 T0.1 checkbox + DONE note

## Deferred `[-]`

- [-] Signing (T0.3): the sync payload signature needs a direct `ed25519-dalek` dep for `SigningKey` —
  arrives with the sync schema, not before.
- [-] Server reconciliation of the grant (Phase 1 T1.2): the local grant projection is designed so the
  server can mint exactly once per key and the account view flips to server-authoritative.
- [-] `--key` passphrase support: unencrypted keys only at v1 (the riir-chain refusal precedent); encrypted
  import fails with the ssh-keygen remedy text.

## Validation record

(2026-08-26, M3)

- `cargo test --features kat_service --lib` — **705/0** (Plan 024's 696 + 9 account unit tests:
  id-derivation pin vs inline blake3, generate+record shape incl. 0600 + version stamp, idempotent
  re-login preserving the record, import adoption + overwrite refusal, key↔sidecar round-trip,
  record-tamper recovery, corrupt/unknown-version record recovery, no-key remedy text, no-`$
  summary)
- `cargo test --features kat_service --test kat_account_g9` — **6/6** (G9a purity ×100 + distinct-key,
  G9b changelog coverage + version-on-file, G9d grant-never-moves across re-logins + same-key-same-
  grant across DIRS, the grant×charge balance composition through the persisted JSON in the bin's
  exact open order, the 3-property tamper fixture, record JSON round-trip)
- `cargo test --features kat_service --test kat_meter_g9` — **11/11** held; `--test kat_meter_g2_alloc`
  — **1/1** held
- `cargo test --lib` (default) — **677/0**, baseline held (G3: account code not compiled)
- `cargo clippy --features kat_service --all-targets` — **0 warnings**; `cargo clippy --all-targets`
  (default) — 0; `cargo check --all-features --lib` — green
- Bin e2e (M3, `--features latent_retrieval,rust_perf,clippy_lints,kat_service`): first login →
  `kat: account key generated` + identity block (silent — no spurious recovery line); second login →
  `already logged in (key loaded)`, same id, record untouched; `--key` import → adopts source identity
  (41e0…), second import → refusal with the overwrite-never message; `account` with no ledger →
  "(no charge ledger at … — heal a repo to start one)"; after a real `--fix --write` run (11 tokens
  billed/charged) → `local burn: 11 KAT charged this repo` + `balance projection: 99999989 of
  100000000 KAT (per-repo until server aggregation)`; key-swap tamper → `kat account: VERIFICATION
  FAILED` + **exit 1**; kat_service-off build → `login/account require a build with --features
  kat_service` + **exit 2** (the `--fix-compile`-off precedent)

## Honest notes

- **One UX bug caught by the e2e itself:** the first `login` printed the record-recovery warning —
  `generate_into` round-tripped through `load()`, whose missing-record branch IS the recovery path.
  Fixed by giving the generators their own `with_fresh_record` (they know the record is new; the
  warning is for a key that LOST its record, not one that just got one). The exact class Plan 024's
  honest notes flag for the bin paths — e2e-before-commit catches it.
- **`load()` follows the key, always.** A swapped key file re-derives a NEW account id on the next
  load (identity = the key, not the directory). That is the designed semantics, but it means the
  record's `account_id` is tamper-evidence only when the KEY is intact — the real integrity chain
  for Phase 1 is the server's once-per-key grant registry. `verify()` covers the mid-process swap
  + the stale-sidecar cases (both exit 1 in the CLI).
- **The healer's fmt skip on a NEW file:** the fmt-clean-at-HEAD check compares against git HEAD —
  an untracked file has no HEAD version, so the healer conservatively skips fmt. Hand-formatted
  instead; the healer's 2 span-preserving edits were re-indented manually (the Plan 024 precedent).
- **Dep weight:** `ssh-key 0.6.7` (+ ed25519-dalek 2.2.0, curve25519-dalek, sha2, zeroize, … — ~15
  crates) lands ONLY under `kat_service` (opt-in). The default build's dep tree is unchanged —
  verified by the 677 default-lib baseline holding.
- **Concurrent sibling:** `dac4b71` (LLM cost tracking) landed on origin/develop between threads,
  on top of Plan 024's `2512ef4`. My working tree applies cleanly on top (zero overlap with the
  files I touched — the sibling's commit predates my edits and touches different files).

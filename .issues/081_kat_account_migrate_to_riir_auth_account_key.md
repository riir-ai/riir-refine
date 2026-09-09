# Issue 081 — migrate `kat_account` onto riir-auth's `account_key` substrate

**Status:** OPEN — design complete + substrate-ready verdict recorded; execution queued behind the shared-worktree conflict (`Cargo.toml` mid-edit by a sibling session, 2026-09-09).

## Context

Plan 327's DRY call landed the SSH-key account substrate in riir-auth
(`account_key` feature, commit `c2274ca`) as the consolidation home for the
`kat_account`-family pattern the workspace ships four times (riir-auth
BOUNDARY: "riir-clippy `kat_account` migration pending — 4-copy consolidation
is the end state"). riir-clippy is consumer #2 after neuron-db-cli's `ndb`.
This issue is the riir-clippy side: file first (the boundary sequence), fix
after.

## Substrate-readiness verdict (verified 2026-09-09): GO — zero riir-auth changes required

Audited `riir-auth/src/account_key.rs` at `c2274ca` against every riir-clippy
call site:

| riir-clippy need | riir-auth substrate | Verdict |
|---|---|---|
| `account_id_for` (BLAKE3(pubkey) hex) | same function, bit-identical (documented in its doc comment) | ✅ |
| `signing_key()` → `ed25519_dalek::SigningKey` | `AccountKey::seed()` → 32-byte Ed25519 seed; `SigningKey::from_bytes(&seed)` locally (`ed25519-dalek` already a `kat_service` dep here) | ✅ no new riir-auth dep |
| `login` / `login_default` (4-step chain) / `login_import` / `load` / `verify` | same names, same semantics | ✅ |
| `LoginOutcome::{Generated,Imported,Adopted,Existing}` | same variants; `Generated`/`Adopted` carry an extra `github_user: Option<String>` — use sites already pattern with `..` (`heal.rs` L724-733) | ✅ |
| `state()` / `dir()` / `key_path()` / `short_id` | same | ✅ |
| `KEY_FILE` / `PUB_FILE` / `RECORD_FILE` consts | exist (canonical names — see the layout decision) | ✅ intended change |

## Design decision: End-state A — canonical identity (not a parameterized layout)

`cargo heal` joins `ndb` on the ONE machine account: `~/.config/riir-auth` +
`id_ed25519` / `account.json` + `RIIR_AUTH_ACCOUNT_KEY` / `RIIR_AUTH_ACCOUNT_DIR`.
One machine, one identity — `ndb`'s private-row sealing and `cargo heal`'s
payload/burn/vote signing share the same key file and the same BLAKE3 account
id. A parameterized layout (kat file names + kat env names forever) would
consolidate the CODE but keep two on-disk identities — the weaker end state,
rejected.

`src/kat_account.rs` becomes a thin kat-facing shim (~870 → ~300 lines):

- `pub struct KatAccount(AccountKey)` newtype + `Deref` (a type alias cannot
  carry `impl`), forwarding associated consts (`KEY_FILE` etc. = the canonical
  names) + `signing_key()` via `seed()`.
- Kat-specific presentation stays local: `history_lines`, `welcome_line`,
  `summary_lines` composition with the grant line.
- Re-exports keep `lib.rs`'s public list compiling unchanged:
  `account_id_for, short_id, AccountState, LoginOutcome, ACCOUNT_VERSION,
  ACCOUNT_CHANGELOG`.

### One-time legacy adoption (the shim's `login_default` chain)

1. canonical key exists → load (`Existing`);
2. `RIIR_HEAL_KAT_KEY` set (deprecated) → loud warn + honor via
   `login_import` (explicit configuration must not silently change meaning);
3. legacy kat dir (`kat_mining_consent::default_dir()`) holds
   `kat_id_ed25519` → adopt into the canonical dir + loud disclosure line
   (preserves the user's existing KAT-ledger identity — generated kat
   accounts do NOT derive from any SSH key, so machine-SSH adoption would
   silently re-bind them);
4. else → substrate `login_default` (`RIIR_AUTH_ACCOUNT_KEY` → machine SSH →
   generate).

The legacy dir is scanned BEFORE the machine SSH key for exactly that reason:
a legacy account adopted from `~/.ssh/id_ed25519` reproduces the same identity
either way, but a GENERATED legacy account only survives via step 3.

### What deliberately changes

- **`grant_kat` leaves the record.** It was always `FREE_TIER_GRANT_KAT`
  (never mutated by any code path — the module's own docs say so). Read sites
  (`run_kat_account` L792, `summary_lines`) take the meter constant. The old
  `kat_account.json` in the legacy dir is abandoned (the key is the authority;
  the record is a projection). riir-clippy's local
  `ACCOUNT_VERSION`/`ACCOUNT_CHANGELOG` retire — the canonical record shape
  (with `github_user`) is riir-auth's, version 1, different
  `RECORD_FILE` name so the two never read each other's files.
- **`RIIR_HEAL_KAT_ACCOUNT_DIR` deprecated for the ACCOUNT only** (warn, not
  honored — pointing the account at the consent dir is what the consolidation
  removes). `kat_mining_consent` is untouched: consent keeps its dir, its
  ungated resolution, and its env semantics.
- **`run_unmine_purge` must sweep BOTH layouts** during the deprecation
  window: canonical files + leftover legacy `kat_id_ed25519{,.pub}`,
  `kat_account.json` if present.

## Touchpoints (all in this repo)

1. `Cargo.toml` — `riir-auth = { path = "../riir-auth", optional = true,
   default-features = false }`; `kat_service` gains `"riir-auth/account_key"`
   (NOT `github_login` — no `--github` flag on this CLI). Net-new tree weight
   ≈ zero: `ssh-key` + `blake3` + `serde_json` already in the `kat_service`
   graph.
2. `src/kat_account.rs` — rewrite as the shim (above).
3. `src/lib.rs` — module decl + re-export list unchanged (names preserved).
4. `src/bin/heal.rs` — `run_kat_account` grant read → meter constant;
   `run_unmine_purge` dual-layout sweep; `run_kat_login` chain comes from the
   shim unchanged; audit remaining `kat_account::` uses.
5. `BOUNDARY.md` — the planned row (added with this issue) flips to landed.
6. `scripts/standalone_dep_gate.sh` — pinned inventory 9 → 10 entries, three
   → four siblings (+ `riir-auth`, private).
7. `.github/workflows/rust.yml` — feature-guard job's private-sibling
   checkout list + `riir-auth` (already token-gated via
   `SIBLING_REPOS_TOKEN`; the job fails loud there today, no status change).
8. `AGENTS.md` (§CLI UX contract) + README env table — the two deprecation
   notes (`RIIR_HEAL_KAT_KEY`, `RIIR_HEAL_KAT_ACCOUNT_DIR`).
9. riir-auth BOUNDARY drift note — flip "migration pending" → consumed (that
   repo's commit).

## Compat test suite (the migration's proof obligations)

- legacy adoption reproduces the SAME account_id (bit-compat pin:
  `account_id_for` over the adopted key == what the old module derived);
- `RIIR_HEAL_KAT_KEY` honored + warned;
- canonical `RIIR_AUTH_ACCOUNT_DIR`/`RIIR_AUTH_ACCOUNT_KEY` honored (test
  isolation rides the dir override);
- signing: seed-route signature verifies against the pubkey AND equals the
  old `signing_key_of` route's signature (bit-identical, not just valid);
- purge sweeps both layouts;
- login outcome print lines unchanged for all four variants.

## Verification gates before landing

`cargo clippy -D warnings` (default + `--features kat_service,kat_transport` +
`--all-features`), `cargo test --lib` at the same states (compat suite
included), `./scripts/standalone_dep_gate.sh`, boundary contract script.

## Hazard note (why this issue exists before the code)

2026-09-09: `Cargo.toml` is mid-edit by a sibling session (score_bench /
regime_probe routing, blob-commit planned) and `.plans/.highwater` is dirty —
this issue (not a plan) is the vehicle, and the code phase MUST start only
after the sibling's `Cargo.toml` work lands, or with strict blob-commit
discipline (`HEAD + my hunks` only, `git hash-object -w` +
`update-index --cacheinfo`). Never `-A` stage in this state.

## Tasks

- [ ] Wait out the sibling `Cargo.toml` edit (poll `git status -sb`) or adopt blob-commit discipline
- [ ] Cargo.toml: optional `riir-auth` dep + `kat_service` feature wiring
- [ ] Rewrite `src/kat_account.rs` as the shim (newtype + legacy adoption chain + presentation + re-exports)
- [ ] `bin/heal.rs`: grant read → meter constant; purge dual-layout sweep; use-site audit
- [ ] Compat test suite (6 obligations above)
- [ ] `standalone_dep_gate.sh` 9 → 10 entries; BOUNDARY row → landed
- [ ] `rust.yml` sibling checkout list + AGENTS.md/README deprecation notes
- [ ] riir-auth BOUNDARY drift note flip (that repo's commit)
- [ ] Full gate: clippy x3 states + lib tests + standalone gate + boundary contract

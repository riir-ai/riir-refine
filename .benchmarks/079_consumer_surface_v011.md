# Bench 079 — consumer public surface: v0.1.1 ships the KAT account client

**Status:** RECORD — Plan 107 T1 + T2 executed 2026-09-07; T3/T4 (leaderboard + service route) filed as riir-dapps issues 019/020; transport deferred to v0.1.2.

## What shipped

- **T1 GO — `kat_service` joined the release feature set** (`c15c2c3`): bin
  `required-features = ["latent_retrieval", "rust_perf", "clippy_lints",
  "kat_service"]`; `release.yml` build + `cargo about generate` pass the SAME
  set (`latent_retrieval,kat_service`) — license-doc parity is the rule: the
  doc must resolve the same graph the binary links or it silently misses
  crates.
- **Dry run [`34087690447`](https://github.com/gist-rs/riir-clippy/actions/runs/34087690447)
  ALL GREEN** (dispatch): licenses G4 accepted `ssh-key` (MIT/Apache; the
  ed25519 tree was already attributed in v0.1.0's doc — 101 → 102 crates),
  5/5 builds, package; publish skipped as designed.
- **v0.1.1 tagged** at `c15c2c3` → run
  [`34088624161`](https://github.com/gist-rs/riir-clippy/actions/runs/34088624161).
- **Consumer README live** on `gist-rs/cargo-heal` (`a11c73a`): TL;DR install
  (4 channels), feature table with counts measured by `--corpus-stats`
  (clippy 49 · rust_perf 112 · docker 31 · kernel_opt 559-not-shipped · sec
  12-not-shipped), "what it is NOT" (no LLM, no heal-time network), KAT
  economics + solid/dashed mermaid flywheel (solid = live today, dashed =
  mining loop), 60-second join (`login`/`account`), privacy posture, verify +
  update/uninstall, FAQ. `.md`-only push — the zero-`.rs` guard unaffected.
- **AGENTS.md hygiene bullet** now names the real set (was
  `default + latent_retrieval + rust_perf`, itself stale — `clippy_lints` is
  in default) and warns about the license-parity trap.

## Measured (local, shipping feature set, CARGO_TARGET_DIR=/tmp/p107)

- `cargo build --release --features latent_retrieval,kat_service --bin
  cargo-heal` → 1m21s warm-ish; clippy CLEAN at both
  `latent_retrieval,kat_service` AND `+kat_transport` combos (one
  pre-existing unused-import warning surfaced only in the shipping combo —
  `Verdict` imported in a `kat_service` fn but consumed only in the
  `kat_transport` block; fixed with a matching cfg, `c15c2c3` — the
  feature-gate trap class).
- `--corpus-stats --json`: `{"clippy_lints":49,"docker":31,"kernel_opt":559,
  "rust_perf":112,"sec":12}` — pinned into the README; AGENTS.md's
  feature-list "105 rules" corrected to the measured 112.
- Offline smoke (`HOME=/tmp/p107_home`): `cargo heal login` → Ed25519 key
  generated, grant projection rendered; `cargo heal account` → account id +
  grant + the repo-local burn (134 KAT from this session's own heal runs —
  the meter works). NOTE: the subcommands are TOP-LEVEL (`cargo heal login`),
  not `cargo heal kat login` — the `KatCommand` enum name is not the CLI
  surface.

## Honest deferrals (each with its reason)

- **`kat_transport` stays out of v0.1.1**: pulls `ureq` whose TLS tree
  (rustls → ring/aws-lc) would need human license-reads on the about.toml
  accept list (the config reserves that act) + an `aws-lc-sys` Windows build
  risk. The mining quickstart's `sync`/`claim` legs say "next release" in the
  README. v0.1.2 scope.
- **Service plane not publicly reachable** (measured): workers.dev URL → CF
  error 1042 (worker's workers.dev route disabled);
  `kat.heal.gist.rs` (the client `DEFAULT_SERVICE_URL`) → DNS unresolved.
  **Filed: riir-dapps Issue 020** (owner ops act — dashboard toggle or custom
  domain). README status table says "bring-up in progress" until then.
- **Leaderboard**: no public contribution view exists (`/balance` is
  per-account, `/admin/claims` is bearer-gated). **Filed: riir-dapps Issue
  019** — `GET /leaderboard?epoch=&limit=` projection over the ledger DO,
  pseudonymous prefixes, cache headers, consumer-web-only (never enters the
  client transport contract unless the CLI grows the command).
- G1 rows for brew/scoop/Linux/Windows install paths still ride on hash
  verification (structural) — same note as Bench 078.

## Gate evidence

- G1 ✓ (macOS arm64 installer → `login`/`account` offline smoke, this bench).
- G2 ✓ — CI leak scan green on the kat_service build (run 34087690447 rows).
- G3 ✓ — default build untouched: `kat_service` is opt-in; the DEFAULT build
  stays bit-identical with zero KAT code compiled (the feature's own G3 rule).
- G4 ✓ — licenses job green with `ssh-key` added; about-features == build
  features enforced in the same commit.

# Issue 153 — CLI version alert + binary self-update + the stale/new corpus surface

**Status:** OPEN — T1–T4 in flight (this session, owner ask 2026-10-08); T5/T6 deferred

## The ask (owner, 2026-10-08)

"alert user via cli for new version with short detail is needed, the old version
not have this yet" + "info about stale and new corpus including auto update
option for bin and option for auto update corpus if staked tier".

## Verdict (GO/GO/GO/GO + one deferred)

| Ask | Verdict | Shape |
|---|---|---|
| New-version alert | **GO, default-on** | One stderr line, cached release check, fail-soft, `RIIR_REFINE_UPDATE_CHECK=0` opts out, `CI` env skips. |
| Auto update option for bin | **GO, opt-in** | `cargo refine upgrade` (explicit, verified, atomic, rollback) + `RIIR_REFINE_AUTO_UPGRADE=1` for hands-free apply at startup. |
| Stale/new corpus info | **GO** | `--info` gains a corpus section: compiled-in rule counts per compiled domain + the lease-corpus local epoch + bin staleness vs the cached latest release. |
| Auto corpus update if staked tier | **GO, opt-in env** | Rides the EXISTING lease lane (`cargo refine update` = pull-only corpus lease refresh): `RIIR_REFINE_AUTO_CORPUS=1` auto-runs the update phase at most once per 12 h, **the lease charge IS the tier gate** (out-of-KAT → skip; the soft-gate posture). No new service contract. |
| Richer tier vocabulary for corpus packs | **DEFERRED (T6)** | Today's entitlement = the lease charge. A per-tier corpus-pack store (freeze/thaw overlay of compiled rules) needs a dapps service contract + a drafter overlay seam — filed, not built here. |

## Substrate-first (what already exists — consumed, not rebuilt)

- `kat_lease_client::run_update_phase` (riir-kat) — the pull-only corpus
  refresh `cargo refine update` runs; idempotent per `charged_epochs`; the
  charge ledger is the entitlement ("staked tier" = can pay the lease).
- `LeaseStore::current_epoch` — the locally-installed corpus epoch (--info).
- `kat_soft_gate` — the Allow/WarnOnly/Refuse posture the auto-corpus gate reads.
- `build_stamp::RELEASE_FEATURES` + the `release_set_matches_workflow` test —
  the release-set pairing this issue must extend in the SAME commit.
- `cf_workers_ai::CfClient` — the ureq AgentBuilder pattern (timeout +
  timeout_connect) mirrored here.
- `.github/workflows/release.yml` package job — the asset contract this issue's
  downloader consumes: `cargo-refine-${TAG}-${target}.tar.gz|.zip` +
  `SHA256SUMS` over every dist file; targets = aarch64-apple-darwin,
  x86_64-apple-darwin, x86_64-unknown-linux-musl, aarch64-unknown-linux-musl,
  x86_64-pc-windows-gnu.

## Not this (name collision, recorded)

`cargo refine update` is TAKEN — the T1.6 pull-only **corpus lease refresh**
(lease check + segment fetch + atomic swap). The binary self-update is therefore
the subcommand **`upgrade`**. A `--update`-style flag would collide with the
lease lane's mental model; `upgrade` reads as rustup's does.

## Plan

- [x] T1 — the alert: `src/update_check.rs` behind `update_check = ["dep:ureq", "dep:sha2"]`;
      GET `api.github.com/repos/gist-rs/cargo-refine/releases/latest`; cache
      `.refine/update_check.json` (ok → 12 h window, failed → 45 min); one
      stderr line `update available: <ver> (installed <ver>) — <short detail>`
      + the release URL; short detail = release title else first body line,
      truncated; semver-numeric compare, ragged → silent; wired at `--version`
      (post-output) and at main start (pre-run) for real runs only.
- [x] T2 — `upgrade`: fetch → pick the asset for `target_triple()` (unknown
      triple = loud refusal, not a guess) → download archive + `SHA256SUMS` →
      verify sha256 → `tar` extraction (system tar; bsdtar reads the Windows
      zip) → chmod +x → backup-swap (`cargo-refine.bak-<old>`) → print new
      version + rerun hint. Refuses: dev builds (exe under a `target/` dir),
      unknown triple, checksum mismatch, tar missing (install.sh fallback
      named). `RIIR_REFINE_AUTO_UPGRADE=1` applies it automatically at startup
      when the alert finds a newer release.
- [x] T3 — `--info` corpus section (`kat_info::local_section`): compiled rule
      counts per compiled domain (clippy_lints, rust_perf; "not compiled"
      otherwise) + the lease-corpus local epoch (`LeaseStore::current_epoch`)
      + bin staleness from the update cache (cache read ONLY — no network in
      a read command) + the `upgrade` hint when stale.
- [x] T4 — auto-corpus-refresh: `RIIR_REFINE_AUTO_CORPUS=1` → at startup,
      once per 12 h (`last_attempt_unix` bookkeeping file), call the SAME
      `run_update_phase` the `update` command runs, deadline-capped, fail-soft
      (any error = one stderr line, never a nonzero exit); out-of-KAT /
      refused posture skips with a named line — the lease charge is the tier
      gate, honored, not bypassed.
- [ ] T5 (docs tail, after the sibling's AGENTS.md edit lands) — AGENTS.md
      feature-table row for `update_check` + the release-set line + the
      release-note convention: every release body carries a `corpus:` line
      (per-domain rule deltas) so the alert quotes real numbers.
- [-] T6 — the per-tier corpus-pack lane: service contract (dapps issue) +
      freeze/thaw overlay seam in the drafter + a committed-artifact manifest
      (the `wasm_validator` store precedent). Deferred until the tier
      vocabulary exists server-side; the lease charge covers the need today.

## Tests

- version compare: newer/older/equal/ragged matrix.
- release JSON parse: tag/title/body extraction, truncation, malformed → Err.
- cache decide: fresh-ok silent, stale-ok fetch, failed backoff, disabled env.
- alert text: newer → line + URL; same/older → None.
- asset pick: 5 triples map to the release.yml names; unknown refused.
- checksum verify: good/bad fixture vectors (sha2).
- swap dry-run: temp-dir staging, backup naming, dev-build refusal.
- non-vacuity by name: `cargo test -p riir-refine --features update_check --lib update_check`.

# Issue 225 — the crates.io build's `--version` calls itself STALE and prescribes a rebuild it cannot do

**Status:** fixed in source (`bbcf719e`) — ships with the next crates.io release (0.5.1); the 0.5.0 crate on the registry keeps the wrong lines (versions are immutable). Filed 2026-11-11 from the cargo-refine 0.5.0 crates.io install smoke (issue 221 P3).

## What happens

`cargo install cargo-refine` (crates.io 0.5.0, verified on amo 2026-11-11) then `cargo-refine --version` prints:

```
build: unknown
release set: STALE — missing latent_retrieval,points,net-mainnet (the network surfaces — sync/update/--refill/burn-push — are compiled out of this binary)
rebuild: cargo build --release --features latent_retrieval,points,net-mainnet,fix_verify,update_check,kernel_pregate,shader_perf --bin cargo-refine
```

Both lines are wrong FOR THIS LANE: the crates.io crate is the intended code-fixing posture (issue 221 lane design — retrieval/points/net are binaries-only), and the prescribed features do not exist in the published manifest (the surgery removed them), so following the advice fails. `build: unknown` — no commit stamp in a registry build.

## Fix

- [x] The publish surgery (`scripts/cratesio_publish.sh`) sets a build-time lane marker (e.g. a `cratesio` cfg/env read by the stamp) so `--version` prints `release set: crates.io (code-fixing; join/leaderboard/latent retrieval ship in the prebuilt binaries — brew or install.sh)` instead of STALE + a rebuild line.
- [x] Stamp the published version's tag/commit (the surgery knows `--ref`) instead of `build: unknown`.
- [x] Test: the stamp's lane branch, both postures.
- [ ] Ships in the next crates.io release (0.5.1) — 👤 publish lane. Issue 221 P4 step 5's alert/upgrade messaging rides the same lane marker (landed `bbcf719e`).

## Landed (`bbcf719e`, 2026-11-11)

- **Mechanism:** `scripts/cratesio_publish.sh` writes `cratesio_lane.stamp` (the `--ref` resolved to its 12-hex commit at surgery time) into the stage root; the include list ships it (`/cratesio_lane.stamp`). `build.rs` → `build_commit::read_lane_marker` exports `RIIR_REFINE_BUILD_LANE` (`crates.io` | `source`) and, on the crates.io lane, the marker's commit as `RIIR_REFINE_BUILD_COMMIT` (malformed content keeps the lane, degrades the commit to `unknown`). Marker absent (the dev tree, the prebuilt binaries) = the source lane, byte-identical output. A file, not an env var: a registry `cargo install` runs none of our scripts and has no git.
- **Output:** `build_stamp::version_lines_for` / `refusal_lines_for` (pure, lane as a parameter). Crates.io lane prints `release set: crates.io (code-fixing build — join/leaderboard/latent retrieval ship in the prebuilt binaries: brew or install.sh)`, NO STALE and NO rebuild line; refusals there point at the prebuilt binaries instead of a rebuild naming removed features.
- **Gate:** the publish script now asserts the built bin prints `build: <ref commit>` + the crates.io lane line and no STALE/rebuild, and that the marker is in `cargo package --list`. Ran `--kc-source registry --ref HEAD --version 0.5.1 --kc-version 0.5.0 --keep-stage` from a detached worktree at `bbcf719e`: **VERDICT green** (zero path deps, release build, `--version` checks, 662-entry package list incl. the marker, `cargo publish --dry-run --locked` ✓; publish refused, not run).
- **Tests:** `build_commit` `lane_marker_absent_and_present`; `build_stamp` `source_lane_block_is_unchanged`, `cratesio_lane_block_names_the_lane_and_never_prescribes_a_rebuild`, `refusal_remedy_follows_the_lane`, `build_lane_is_a_known_value`; `tests/build_stamp_cli.rs` 7/7 (source lane, release-ish posture).
- **Disclosed, NOT fixed here:** `cargo-refine --version` prints `cargo-refine <ver>` TWICE on both lanes — the always-on header (`heal.rs` "The header" block, ungated by `args.version`) runs before the `--version` arm returns. Pre-existing; one-line fix candidate (`&& !args.version` on the header condition).

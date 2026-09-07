# Bench 078 — binary distribution pipeline: CI dry-run gate record

**Status:** RECORD — pipeline proven end-to-end on dispatch; G1 (clean-machine install) remains gated on the first real tag, which is gated on T1.3 (`RELEASE_PAT`).

Proposal 003 / Plan 105. Three `workflow_dispatch` dry runs of
`.github/workflows/release.yml` on 2026-09-07, each after a fix. Dispatch
proves everything a tag run would prove except the publish job itself
(tag-gated by design; publish additionally fails loud without `RELEASE_PAT`).

## Runs

| Run | Result | Finding → fix (commit) |
|---|---|---|
| [`34072493969`](https://github.com/gist-rs/riir-clippy/actions/runs/34072493969) | licenses ✓ · macOS ×2 ✓ · Windows ✓ · musl ×2 ✗ | `mlugg/setup-zig` default download 404'd on runners → `taiki-e/setup-cross-toolchain-action@v1` + uniform plain `cargo build` (`64604af`) |
| [`34073335938`](https://github.com/gist-rs/riir-clippy/actions/runs/34073335938) | 5/5 builds ✓ · package ✗ AT THE LEAK SCAN | Gate caught a REAL leak: `C:\Users\runneradmin\.cargo\registry\src\…` panic-Location paths on Windows (workspace `D:\a` and CARGO_HOME sit on **different drives**) → remap extended (`ad71a85`). Also one scan false positive: `n:/usr/bin` is a substring of the musl default-PATH constant (`…/bin:/bin:/usr/bin`) → drive-letter arm now uppercase-only |
| [`34074402607`](https://github.com/gist-rs/riir-clippy/actions/runs/34074402607) | **ALL GREEN** — licenses, 5 builds, package (stage + archive + checksum + scan ×5) | Final artifact verified: 5 archives (2.2–3.2 MB), `SHA256SUMS` `sha256sum -c` → 5× OK, `THIRD_PARTY_LICENSES.md` in every archive |

The Windows leak was the gate working as designed: registry crates' panic
`Location` strings bake their compile paths **regardless of stripped
debuginfo**, and the pre-fix remap (`D:\a` only) could not cover
`C:\Users\runneradmin`. Post-fix exe re-scanned clean from the released
artifact. Note the boundary lesson measured en route: a preceding-char
boundary in the scan regex would have REJECTED the real leak (it occurs
mid-string: `` `Err` valueC:\Users\runneradmin``) — uppercase-drive + two
segments is the discriminator that keeps both true and false positives right.

## Gate evidence (pre-release)

- **G2** ✓ — leak scan green on all 5 CI-built binaries in run `34074402607` (defined symbols: 0 — dyld imports + `__mh_execute_header` allowlisted; machine paths: 0; secret shapes: 0).
- **G3** ✓ (partials) — `publish = false` intact (Cargo.toml L7); `gist-rs/riir-clippy` PRIVATE verified via API the same day; repo suite green (bin tests 15/15, install-copy parity 3/3, clippy clean on the shipping feature set).
- **G4** ✓ (partials) — `SHA256SUMS` verifies for all 5 archives (`sha256sum -c` OK ×5, on the downloaded artifact); `THIRD_PARTY_LICENSES.md` present in every archive (tar/zip listings checked; 96 crates, 5 license families, cargo-about@0.9.2 pinned).
- **G1** ✅ **PASS (first real tag, 2026-09-07, T3.2 chain complete)** — macOS arm64 clean-prefix install: `CARGO_HEAL_INSTALL_DIR=/tmp/g1_cargo_heal sh -c 'curl -fsSL …/install.sh | sh'` → installer fetched `cargo-heal-v0.1.0-aarch64-apple-darwin.tar.gz`, installed, binary executes (full usage render; note `--version`/`--help` are not flags — usage-on-unknown-flag is the binary's behavior). Consumer hash verification proven: downloaded the release's `SHA256SUMS` + the archive from the PUBLIC release → `shasum -a 256 -c` → `aarch64-apple-darwin.tar.gz: OK` (the other 4 rows not-downloaded, expected). Untested-live here: `brew`/`scoop`/Linux/Windows install paths — formula + manifest hashes verified against `SHA256SUMS` structurally (all 5 match), PRs merged; Linux/Windows G1 rows remain for a box that boots them.

## T3.2 chain record (first real tag, 2026-09-07)

| Run | Result | Finding → fix (commit) |
|---|---|---|
| [`34081938353`](https://github.com/gist-rs/riir-clippy/actions/runs/34081938353) | builds ✓ package ✓ · publish ✗ AT THE EMPTY-TOKEN GUARD | **The guard worked as designed.** `RELEASE_PAT` was set as an ORG secret (Plan 105 T1.3's original text) — GitHub's FREE plan silently resolves org secrets to EMPTY for private repos even when the repo is selected and the UI lists it (community discussion 25428). Fix: `RELEASE_PAT` moved to a REPO secret on `gist-rs/riir-clippy`; the empty org secret deleted; T1.3 amended. |
| same run, publish rerun | release ✓ tap-branch ✓ · `gh pr create` ✗ | `GraphQL: The release/v0.1.0 branch has no history in common with main` — the bump steps did `git init` (orphan history) then PR'd into `main`; GitHub rejects unrelated-history PRs. Fix: clone `main` shallow, branch off it, `cp` the formula/manifest in, force-push the branch (`0740daa`). |
| [`34085254189`](https://github.com/gist-rs/riir-clippy/actions/runs/34085254189) | **ALL GREEN** — licenses, 5 builds, package, publish: release live (5 archives + `SHA256SUMS` + `THIRD_PARTY_LICENSES.md`, `isDraft: false`), homebrew-tap#1 + scoop-bucket#1 opened by the bot, hashes verified 5/5 vs `SHA256SUMS`, both merged. G1 above. | The pipeline is DONE: tag → public release → tap/bucket PRs in one green run. |

Two secrets-level lessons now on record: (1) free-plan org secrets never reach private repos — silent-empty, fail at USE time not SET time, hence the fail-loud guard earning its keep; (2) `gh pr create` needs pull_requests:write on a fine-grained PAT (contents RW alone is insufficient — T1.3's original text underspecified).

## Durations (per-ovens, 2026-09-07, GH-hosted runners)

licenses 1m10s · aarch64-apple 2m36s→3m39s · x86_64-apple 3m30s→5m9s ·
x86_64-musl 4m28s→5m16s · aarch64-musl 3m45s→4m27s · windows 8m34s→12m31s ·
package 15–19s. Total wall per dry run ≈ 13 min (parallel matrix), ≈ 45
billable runner-minutes per iteration — the reason dry runs, not tags, own
the debugging loop.

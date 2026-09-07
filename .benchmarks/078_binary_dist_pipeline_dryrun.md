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
- **G1** ☐ — clean-machine install on all five assets; requires a published release → first tag → T1.3.

## Durations (per-ovens, 2026-09-07, GH-hosted runners)

licenses 1m10s · aarch64-apple 2m36s→3m39s · x86_64-apple 3m30s→5m9s ·
x86_64-musl 4m28s→5m16s · aarch64-musl 3m45s→4m27s · windows 8m34s→12m31s ·
package 15–19s. Total wall per dry run ≈ 13 min (parallel matrix), ≈ 45
billable runner-minutes per iteration — the reason dry runs, not tags, own
the debugging loop.

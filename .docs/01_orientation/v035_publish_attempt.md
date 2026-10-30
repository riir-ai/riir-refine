# v0.3.5 publish attempt — 2026-10-30 (BLOCKED, one step left)

## What happened

"do remains" item 2 (publish the v0.3.5 release so `cargo refine upgrade` has
something newer than v0.3.4 to hand out). The `scripts/manual_release.sh`
lane was followed exactly (the standing manual mode, per its own header — the
Actions billing block refuses the tag-triggered pipeline):

1. ✅ Notes exist (`release/notes-v0.3.5.md`, `corpus:` line present).
2. ✅ Licenses generated (cargo-about, accepted-list gate).
3. ✅ All 5 targets built at the v0.3.5 tag commit `0542da6c` (`--profile dist`,
   pinned release feature set, `--remap-path-prefix $HOME=/build`).
4. ✅ G2 leak scan PASS on all 5 raw binaries (no user paths, no secret-shaped
   strings, no drive letters).
5. ✅ Size gate PASS (9.32–11.17 MiB ≤ 12.00 ceiling).
6. ✅ Archives + SHA256SUMS assembled in `/tmp/manual_release_dist/`.
7. ❌ **`gh release create` FAILED — no GitHub API token.** The build/publish
   split means the M3 has SSH git auth (pushes work) but gh CLI is
   unauthenticated (`gh auth status`: no hosts; hosts.yml `{}`), and no
   RELEASE_PAT anywhere this session could find (env, .env, keychain-not-probed).

## The one command that finishes this (owner, with a token)

```sh
GH_TOKEN=<a PAT with gist-rs/cargo-refine contents:write> \
  sh scripts/manual_release.sh v0.3.5
```

⚠ Re-running rebuilds (fine, ~15 min warm). To publish WITHOUT rebuilding,
the packaged set is still in `/tmp/manual_release_dist/` (reboot loses it):

```sh
gh release create v0.3.5 -R gist-rs/cargo-refine \
  --title "cargo-refine v0.3.5 — corpus: kernel_opt 1022 (+2 since the v0.3.4 build) — rust_perf 278 (=) · sec 31 (=) · clippy_lints 112 (=) · dist 7 (=)" \
  --notes-file release/notes-v0.3.5.md \
  /tmp/manual_release_dist/*
```

The title carries the corpus line per the Issue-153 T5 convention (the
update alert renders the title).

## REVERTED — the tap/scoop bumps were pushed and pulled back (VERIFIED clean)

The formula + scoop manifest bumps were generated from the release
SHA256SUMS and pushed to BOTH public repos — then **reverted (reset + force
push back to the 0.3.4 commits)** when the release itself could not be
created: a formula/manifest pointing at a 404 asset would break every brew
install / scoop update. **Verified clean by fresh clone** (raw.githubusercontent
served a stale cache for ~2 min; the git truth is authoritative):

- gist-rs/homebrew-tap `Formula/cargo-refine.rb` → **0.3.4** (`6ab4a3f`, `main = origin/main`)
- gist-rs/scoop-bucket `bucket/cargo-refine.json` → **0.3.4** (`b9cdfcb`, `main = origin/main`)

When the release lands, re-apply: same edits (version 0.3.5, the five
URLs → `download/v0.3.5/`, hashes from the release's SHA256SUMS — the exact
values this attempt computed are in this file's git-history twin of
`/tmp/manual_release_dist/SHA256SUMS`):

| asset | sha256 (first 16) |
|---|---|
| aarch64-apple-darwin.tar.gz | `5ea293900d532743` |
| x86_64-apple-darwin.tar.gz | `cba0134b2deca997` |
| aarch64-unknown-linux-musl.tar.gz | `8eafd6d5b410af4e` |
| x86_64-unknown-linux-musl.tar.gz | `39d71827c4edf54e` |
| x86_64-pc-windows-gnu.zip | `91322fc56f0ea06c` |

⚠ These hashes are for THE BINARIES THIS SESSION BUILT at `0542da6c`. If the
finishing run re-runs `manual_release.sh`, it rebuilds and its OWN SHA256SUMS
is the truth — use those, not these (build determinism across hosts/toolchain
states is not pinned).

## Why the tag's own CI didn't publish

`release.yml` triggers on `v*` tag pushes — `v0.3.5` was pushed
(`git ls-remote` confirms `0542da6c` at the remote tag) but no release
appeared: the Actions billing block (the same 2026-09-08 block the manual
script's header names) refused the run. `workflow_dispatch` would cost the
same blocked budget — not retried.

## Pre-staged same evening — the finishing run is now 4 commands

1. **SHA256SUMS regenerated** (`/tmp/manual_release_dist/SHA256SUMS`):
   the staged manifest carried a self-referential entry (`SHA256SUMS` →
   `e3b0c44…`, the empty-string hash — a manual re-hash of the dir after
   the failed publish truncated the file before `sha256sum` read it).
   Every PAYLOAD hash was already correct; only the self-entry was
   poisoned. Regenerated with an explicit file list and now
   `shasum -a 256 --check` passes clean. Root cause fixed at source:
   `scripts/manual_release.sh` `e448c147` (explicit glob, no self-entry
   possible).
2. **Tap/scoop bumps PRE-COMMITTED, NOT PUSHED** (pushing would point the
   manifests at 404 assets — the exact mistake reverted earlier today):
   - gist-rs/homebrew-tap `9d457be` (fresh clone at `/tmp/homebrew-tap-stage`, 1 ahead of `origin/main`)
   - gist-rs/scoop-bucket `96482af` (fresh clone at `/tmp/scoop-bucket-stage`, 1 ahead of `origin/main`)
   - All 5 recorded hashes verified against the staged archives
     (`OK` × 5) — these are the hashes of THE BINARIES AT `0542da6c`
     uploaded as-is; no rebuild planned, so the table above stays true.
3. **Token**: owner seals `gh-release-pat` (fine-grained PAT, repo
   `gist-rs/cargo-refine`, Contents: Read and write) via
   `kat-secret seal gh-release-pat` in their own terminal.

Finishing run once sealed (agent runs; Touch ID prompt for the owner):

```sh
~/.kat/bin/kat-secret exec gh-release-pat --env GH_TOKEN \
  --why "publish cargo-refine v0.3.5 release (tag 0542da6c)" -- \
  gh release create v0.3.5 -R gist-rs/cargo-refine \
    --title "cargo-refine v0.3.5 — corpus: kernel_opt 1022 (+2 since the v0.3.4 build) — rust_perf 278 (=) · sec 31 (=) · clippy_lints 112 (=) · dist 7 (=)" \
    --notes-file release/notes-v0.3.5.md \
    /tmp/manual_release_dist/*
git -C /tmp/homebrew-tap-stage push origin main
git -C /tmp/scoop-bucket-stage push origin main
```

Then: verify the asset URL returns 200 + the release API `latest` =
v0.3.5; smoke-test the shipped aarch64 binary (`cargo refine --version`);
flip this doc BLOCKED → PUBLISHED; commit + push riir-refine; add the
`gh-release-pat` row to the touch-deploy skill's Known-tokens table.

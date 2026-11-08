# Issue 219 — gist-rs → riir-ai org migration: the owner-gated residue + the 4090 disk-full incident found en route

**Status:** OPEN — §A items are owner acts (PAT/secret console + CF Touch-ID lane + 4090 disk approval); §B records what session zed-m3 2026-11-08 already landed and closed. No claim lease (nothing here is agent-implementable without owner auth).

The org move (24 riir-* + riir-dao-proposal + the public dist family reflex/cargo-refine/homebrew-tap/reflex-site) landed 2026-11-07 (pass 1 `5cce78ed`/pass 2 `8f033e4b` local, pushed as `e8385410` here). GitHub redirects keep every old URL working at both HTTP (308) and git-protocol (SSH) level — verified by a full 26-repo `SYNC DONE` on the 4090 over the old SSH paths AFTER the transfer (13:23Z). Nothing user-facing is broken today; this issue exists so the residue doesn't rot silent.

## A. Owner acts (blocking, not agent-executable)

- **A1. PAT / secret rescoping.** The Actions secrets still scope gist-rs repos while every migrated workflow now clones `github.com/riir-ai/...`:
  - `SIBLING_REPOS_TOKEN` — the sibling-clone lane in every repo's `rust.yml` (fine-grained PAT, ~90-day expiry noted in release.yml); rescope to the riir-ai org repos or the next sibling job 404s.
  - `gh-release-pat` — cargo-refine RW for riir-refine `release.yml`'s dist push + tap/bucket bump PRs.
  - `GH_RAW_TOKEN` — riir-refine raw read used by kat-service (riir-dapps).
  - Deploy keys on the boxes: the 4090's SSH key is ALREADY riir-ai-authorized (proven 2026-11-08 — the flipped remote negotiated + received pack data over `git@github.com:riir-ai/riir-ai.git` before dying on disk-full), so box keys are NOT part of this item.
- **A2. Prod redeploys** (CF auth law = owner Touch-ID lane; dev-lane token unaffected): kat-service worker (refine.riir.ai still serves pre-migration copy/legal links), reflex-site, rethink.riir.ai.
- **A3. 4090 E: drive is FULL — 0 GB free of 1863 GB** (C: 76 GB free, F: 51 GB). Every gitsync fetch now fails `No space left on device` (loose-object + index.lock writes); the lane self-heals nothing and burns ~2 min per repo. Assessment recorded 2026-11-08 (read-only):
  - C: safe-class caches (regenerable, NOT the constraint but the easy win): `AppData\Local\Temp` 20.4 GB, `uv\cache` 9.7 GB, `pip\cache` 2.7 GB, `.cache\huggingface` 21.1 GB (owner decides — model re-downloads), `.cargo` 4.9 GB, `.rustup` 4.6 GB.
  - E: holds the real mass: `E:\git` (target dirs = regenerable; `riir-train/data` GGUFs = owner decides per the disk-clean priority list), plus `E:\public`, `E:\pics`, `E:\MaxGame (Dev)`, `E:\t980`, `E:\rel`, `E:\pplx`, `E:\clef`, `E:\d3`, `E:\scratch`, `E:\plan471`, `E:\tmp` (4.45 GB scratch) — unmeasured (full walk times out at 5 min).
  - Cleanup follows the `disk-clean` skill contract (list → classify → owner approves → delete). Until space is freed, gitsync on the 4090 is DOWN (old gist-rs remotes would NOT have saved it — the failure is object writes, not auth).

## B. Landed + closed by this session (2026-11-08, zed-m3)

- **Dist repos content follows the move** (self-references updated; redirects keep old links alive meanwhile):
  - `riir-ai/reflex` `bec68fe4c` — install.sh/install.ps1 REPO vars + usage one-liners, README brew tap + engine-source + releases links.
  - `riir-ai/cargo-refine` `afb2348bb` — installers, README (home line now canonical `refine.riir.ai`; `refine.gist.rs` 308s there), LICENSE.md + CHANGELOG.md links.
  - `riir-ai/homebrew-tap` `bf3a018eb` — tap renamed `riir-ai/tap` in README; formula homepages + release download URLs (agree with release.yml's `DIST_REPO`/`TAP_REPO=riir-ai/*` generators, so the next automated bump won't revert).
  - `gist-rs/scoop-bucket` `f876207db` — manifest download/checkbadger URLs → riir-ai releases (releases moved WITH the repos); bucket itself stays gist-rs, matching release.yml's `BUCKET_REPO: gist-rs/scoop-bucket` pin.
  - Deliberate keeps: both dist READMEs' `scoop bucket add gist-rs …/scoop-bucket` lines; `hf://gist-rs/*` artifact lanes (HF org untouched).
- **riir-refine** `c15cabe6` — two stale workflow comments (release.yml's `gist-rs/cargo-heal` → `riir-ai/cargo-refine`; ops_dashboard.yml's `gist-rs/*` → `riir-ai/*` siblings).
- **4090 hygiene:** 24 repo remotes flipped gist-rs → riir-ai (the moved set: riir-* + reflex-site; keps: seal-remake/ndb/scoop-bucket stay gist-rs, katgpt-rs/seal-game-editor stay katopz/maxion-game) — flipped via textual `.git/config` edit after `git remote set-url` failed on the full disk; 13 git + 6 ssh zombie processes (11-04…11-06 hangs, the Windows OpenSSH teardown family) killed; helper scripts removed from the box.
- **gitsync verification posture:** remotes flipped + access proven (pack negotiation succeeds over the new URLs); the lane's remaining failure is purely A3 disk space. One transient `E:\git\riir-dapps\.git\index.lock` seen mid-run is the live self-heal's own lock, not stale.

## C. Observations for the record

- **Debugging lesson (misdiagnosis corrected):** `git remote set-url` failing `failed to write new configuration file … config.lock` on every repo looked like an app-level write block (a manual zero-byte `[IO.File]::Open` lock-create succeeded) — root cause was the FULL DISK all along: create-succeeds/write-content-fails is the ENOSPC signature. Check disk before blaming AV/processes/ACLs on a wedged box.
- `refine.gist.rs` → `refine.riir.ai` 308 redirect confirmed live; canonical updated in the cargo-refine dist README.
- Two stale 0-byte `index-*.tmp.lock` files sit in `E:\git\seal\.git` (non-fleet repo, harmless; sweep when the box next gets a hygiene pass).
- The local riir-refine checkout on m3 stays intentionally diverged (fleet convention; sibling corpus WIP in the worktree) — this issue's commit rides origin/develop via the detached-worktree cherry-pick, self-healing on next `pull --rebase`.

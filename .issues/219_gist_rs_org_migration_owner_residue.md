# Issue 219 — gist-rs → riir-ai org migration: the owner-gated residue + the 4090 disk-full incident found en route

**Status:** OPEN — **A2 rethink + reflex-site redeploys LANDED 2026-11-08 (§D)**; remaining owner acts: A1 PAT/secret console rescoping + the kat-service prod ceremony (worker code ready at dapps `49a59d0`). A3 executed on the m3 local checkout (`2fca79279`, unpushed — that session pushes it). No claim lease (the residue here is owner-auth work; §D's agent lanes are landed and pushed).

The org move (24 riir-* + riir-dao-proposal + the public dist family reflex/cargo-refine/homebrew-tap/reflex-site) landed 2026-11-07 (pass 1 `5cce78ed`/pass 2 `8f083e4b` local, pushed as `e8385410` here). GitHub redirects keep every old URL working at both HTTP (308) and git-protocol (SSH) level — verified by a full 26-repo `SYNC DONE` on the 4090 over the old SSH paths AFTER the transfer (13:23Z). Nothing user-facing is broken today; this issue exists so the residue doesn't rot silent.

## A. Owner acts (blocking, not agent-executable)

- **A1. PAT / secret rescoping.** The Actions secrets still scope gist-rs repos while every migrated workflow now clones `github.com/riir-ai/...`:
  - `SIBLING_REPOS_TOKEN` — the sibling-clone lane in every repo's `rust.yml` (fine-grained PAT, ~90-day expiry noted in release.yml); rescope to the riir-ai org repos or the next sibling job 404s.
  - `gh-release-pat` — cargo-refine RW for riir-refine `release.yml`'s dist push + tap/bucket bump PRs.
  - `GH_RAW_TOKEN` — riir-refine raw read used by kat-service (riir-dapps).
  - Deploy keys on the boxes: the 4090's SSH key is ALREADY riir-ai-authorized (proven 2026-11-08 — the flipped remote negotiated + received pack data over `git@github.com:riir-ai/riir-ai.git` before dying on disk-full), so box keys are NOT part of this item.
- **A2. Prod redeploys** (CF auth law = owner Touch-ID lane; dev-lane token unaffected): kat-service worker (refine.riir.ai still serves pre-migration copy/legal links), reflex-site, rethink.riir.ai.
- **A3. 4090 E: drive was FULL — 0 GB free of 1863 GB. RESOLVED 2026-11-08 ~19:30Z (owner approved the clean):** E: → **154.6 GB free**, C: 76.1 → **129.3 GB**, F: 51.1 → 52.7. Deleted (SAFE-class only): per-repo `target\debug`/`incremental`/`wasm32*` on every E:\git repo **except `riir-refine`** (a live sibling cargo test held it; release dirs kept everywhere per the load-bearing rule — note `zed\target` 43.6 GB is release-ONLY, kept); stale (>7d) plan-target dirs `p614-target` 5.0, `p614-check` 1.1, `plan416_ab` 0.3, `F:\tgt-1066` 1.7 (`rt_t673_target` KEPT — newest write that morning, an active lane); C: Temp 20.4, uv 9.7, pip 2.7, recycle bin 9.1, HF cache 21.1 (models re-download on demand), CrashDumps 0.5, NVIDIA shader caches 0.8. Stale `.git` locks swept; **gitsync re-run → full `SYNC DONE` 19:44Z, zero errors in the run segment, over the flipped riir-ai remotes** — the lane and the migration are end-to-end verified. Cold-rebuild cost: every touched repo's first debug build is cold (release warm builds untouched). **Still owner-decides (measured, untouched):** `F:\models` 154.8 GB (league models — highest-value), `F:\vllm-381-ref` 31.9 (league opponent build), `E:\pplx` 48.6 + `E:\clef` 24.5 (27B-class models + venvs), `E:\plan471` 5.0 (models+venv, plan 471), `E:\scratch` 8.9 + `E:\tmp` 6.3 (recent sibling scratch, incl. today's `b342`), `E:\git\zed\target` 43.6 (release-only warm build), `F:\Hive` 9.9, `E:\git\riir-refine\target` 18.3 (in use at write time), VSS shadow storage 12.7 on C:, `.cargo` ~5.3 + rustup 4.6 (in use / pinned). **Never-touch:** `E:\pics` 128.7 + `E:\public` 16.6 (personal photos/media).

## B. Landed + closed by this session (2026-11-08, zed-m3)

- **Dist repos content follows the move** (self-references updated; redirects keep old links alive meanwhile):
  - `riir-ai/reflex` `bec68fe4c` — install.sh/install.ps1 REPO vars + usage one-liners, README brew tap + engine-source + releases links.
  - `riir-ai/cargo-refine` `afb2348bb` — installers, README (home line now canonical `refine.riir.ai`; `refine.gist.rs` 308s there), LICENSE.md + CHANGELOG.md links.
  - `riir-ai/homebrew-tap` `bf3a018eb` — tap renamed `riir-ai/tap` in README; formula homepages + release download URLs (agree with release.yml's `DIST_REPO`/`TAP_REPO=riir-ai/*` generators, so the next automated bump won't revert).
  - `gist-rs/scoop-bucket` `f876207db` — manifest download/checkbadger URLs → riir-ai releases (releases moved WITH the repos); bucket itself stays gist-rs, matching release.yml's `BUCKET_REPO: gist-rs/scoop-bucket` pin.
  - Deliberate keeps: both dist READMEs' `scoop bucket add gist-rs …/scoop-bucket` lines; `hf://gist-rs/*` artifact lanes (HF org untouched).
- **riir-refine** `c15cabe6` — two stale workflow comments (release.yml's `gist-rs/cargo-heal` → `riir-ai/cargo-refine`; ops_dashboard.yml's `gist-rs/*` → `riir-ai/*` siblings).
- **4090 hygiene:** 24 repo remotes flipped gist-rs → riir-ai (the moved set: riir-* + reflex-site; keeps: seal-remake/ndb/scoop-bucket stay gist-rs, katgpt-rs/seal-game-editor stay katopz/maxion-game) — flipped via textual `.git/config` edit after `git remote set-url` failed on the full disk; 13 git + 6 ssh zombie processes (11-04…11-06 hangs, the Windows OpenSSH teardown family) killed; helper scripts removed from the box.
- **gitsync verification posture:** remotes flipped + access proven (pack negotiation succeeds over the new URLs); the lane's remaining failure is purely A3 disk space. One transient `E:\git\riir-dapps\.git\index.lock` seen mid-run is the live self-heal's own lock, not stale.

## C. Observations for the record

- **Debugging lesson (misdiagnosis corrected):** `git remote set-url` failing `failed to write new configuration file … config.lock` on every repo looked like an app-level write block (a manual zero-byte `[IO.File]::Open` lock-create succeeded) — root cause was the FULL DISK all along: create-succeeds/write-content-fails is the ENOSPC signature. Check disk before blaming AV/processes/ACLs on a wedged box.
- `refine.gist.rs` → `refine.riir.ai` 308 redirect confirmed live; canonical updated in the cargo-refine dist README.
- Two stale 0-byte `index-*.tmp.lock` files in `E:\git\seal\.git` (non-fleet repo) — REMOVED during the 11-08 cleanup.
- The local riir-refine checkout on m3 stays intentionally diverged (fleet convention; sibling corpus WIP in the worktree) — this issue's commit rides origin/develop via the detached-worktree cherry-pick, self-healing on next `pull --rebase`.

## D. Landed 2026-11-08, session zed-m3 (the agent-executable halves of A1/A2)

- **The scoop-bucket move to riir-ai (owner, same day) dissolved the RELEASE_PAT two-org span** — one fine-grained PAT (resource owner `riir-ai`) now covers all three dist repos. The stale pins it left behind all landed:
  - riir-refine `3c0f8f94` — release.yml `BUCKET_REPO: gist-rs/scoop-bucket` → `riir-ai/scoop-bucket` (detached-worktree commit; the local checkout stays diverged by fleet convention).
  - riir-dapps `49a59d0` — kat-service front-page scoop one-liner + front.rs assertion + the copy-contract exemption widened to org vocabulary (org name + tap + bucket URL — same public-org class as `riir-ai/tap`, private-repo-name law unchanged). 625 lib tests + clippy `-D warnings` green, `--locked` (sibling Cargo.lock untouched). **Prod deploy still pending the owner ceremony** — this commit is the worker code the ceremony ships.
  - reflex-site `8b7d8ca` — index.html install one-liner (text + data-copy) + the served `skills/reflex-integration/SKILL.md`.
  - dist READMEs: `cargo-refine e3ffc3a` · `reflex 54192b8` · `scoop-bucket e10eedc`.
- **A2 storefront redeploys LANDED + probed:**
  - **reflex-site** — Touch-ID `cf-deploy-reflex`, clean detached worktree at origin/main `8b7d8ca`, Version `84730a68-e01e-44db-921b-8dee3b8e9c00`. Probe: reflex.riir.ai went 20 gist-rs refs → **0**; all repo links `github.com/riir-ai/*`; `scoop bucket add riir-ai` ×2.
  - **rethink** — Touch-ID `cf-deploy-rethink`, detached at `2fed0c6`, Version `2108dbd0-bbbc-499e-88fe-3d27cc5fa8e7`. The morning's `01bd70a` deploy predated org-move commit `047744c`, so the live page still named `gist-rs/riir-instinct` — probe now reads `github.com/riir-ai/riir-instinct`, **0 gist-rs refs** (foxfox twin: 0). Also carries the `2fed0c6` feed column (banking77 composed macro-F1; the 094-rerun gold-bearing cell folds when its lane lands).
  - **kat-service** — NOT deployed (mainnet = owner ceremony only, the auth law). refine.riir.ai still serves the pre-migration copy (probed: 7× gist-rs/cargo-refine + 4× raw gist-rs URLs + 1 scoop gist-rs) until the owner runs the vault net-leg.
- **Pre-existing flag (not this lane's):** reflex-site `scripts/public_copy_gate.cjs` FAILS on `/bench/ @390` sideways scroll (scrollWidth 422 vs 390, every `<details>` open) — fails at HEAD^ without the scoop change; belongs to the bench-feed lane (`1f029d1` or earlier).

# Issue 150 — the Homebrew-installed cargo-refine shadows the maintained symlink on PATH

**Status:** RESOLVED 2026-10-08 — **option 2 APPLIED as `brew unlink cargo-refine`** (the Claude-verdict REVISE, session `fc0f0611`: AGREE with option 2's goal, unlink over uninstall — reversible, keeps the Cellar files). Verified post-unlink: `which -a cargo-refine` = `~/.cargo/bin/cargo-refine` ONLY; bare `cargo refine --version` resolves the maintained binary (update_check present, `build:` line present). Option 3 remains landed (the build-commit stamp, `0a4c010c`/`3a9d1e96`) and is now the BACKSTOP: if a later `brew upgrade`/reinstall re-creates the link, the standing stamp-vs-HEAD check catches it. **Option 3 LANDED 2026-10-07** (the build-commit stamp: `--version` prints `build: <hash>[-dirty]|unknown`; commit `0a4c010c`; the standing shadow check is the hash-vs-HEAD comparison below). Found 2026-10-06 by the dapps plan-045 verdict round 2 during the B7 privacy-cut live smoke · Machine: the M3 box. **STANDING CHECK FIRED 2026-10-08 (M3, idle-loop hygiene): the shadow is LIVE and the maintained binary was STALE — both halves in one reading.** Bare `cargo refine --version` resolved `/opt/homebrew/bin/cargo-refine` (Cellar 0.2.1 tap build) and printed NO `build:` line — the exact detector; `which -a` shows Homebrew FIRST, `~/.cargo/bin` second. The absolute-path read showed the maintained binary at `build: 6ee548ca3cfd` ≠ HEAD `32d0ac3458b6` (stale maintained binary — the shadow's danger WITHOUT the tap: even the symlink path was behind). Remedy applied per the issue's own law: rebuilt in place at the pinned release set (`latent_retrieval,kat_service,kat_transport,fix_verify,update_check`), stamp now `32d0ac3458b6` = HEAD ✓. Session hygiene note: every healer invocation this session used the absolute path. The tap build's silence is now a POSITIVE detector: no `build:` line ⇒ pre-2026-10 binary ⇒ not the maintained one.

**STANDING CHECK FIRED 2026-10-08 (4090 box, session hygiene, post-T8-CMMA):** the maintained
binary `~/.cargo/bin/cargo-refine.exe` was a manual COPY (Windows — no symlink convention) at
`07:43` today, pre-dating the build stamp (printed NO `build:` line) and missing `update_check`
(release set read 4 of 5). Remedy per the same law: `git pull --ff-only` to `c3cf1865`, rebuild
at the pinned release set (7m05s), COPY over the PATH binary. Verified by direct invocation:
`build: c3cf186526dc` = HEAD ✓, full 5-feature set. ⚠ Cosmetic git-bash quirk on this box:
`cargo refine --version` (the cargo SUBCOMMAND form) prints only the last line of the block —
read the stamp via the DIRECT binary path (`~/.cargo/bin/cargo-refine.exe --version`), which
prints all four lines. No tap/shadow on this box (Homebrew is macOS-only); the failure class
here was pure staleness of the manual copy.

## Resolution — 2026-10-08, the M3 box (option 2 as unlink, owner call via Claude verdict)

The Claude verdict (session `fc0f0611`, 2026-10-08) REVISEd option 2 to `brew unlink cargo-refine` — AGREE with the goal (the maintained binary becomes the only cargo-refine on PATH), unlink over uninstall because it answers the one caution on record (the Cellar install may be the owner's deliberate check of the public release lane) while reversing with one command. Applied and verified same day: `brew unlink cargo-refine` (1 symlinks removed) → `which -a cargo-refine` lists `~/.cargo/bin/cargo-refine` ONLY → bare `cargo refine --version` reads the maintained binary (full release set incl. `update_check`, `build:` stamp present).

- **Undo (restore the tap lane on PATH):** `brew link cargo-refine`.
- **Test the release lane WITHOUT relinking:** run the Cellar binary directly — `/opt/homebrew/Cellar/cargo-refine/0.2.1/bin/cargo-refine --version` (it stays installed, just unlinked).
- **Standing caveat:** a later `brew upgrade cargo-refine` or a tap reinstall re-creates the link silently — the option-3 stamp-vs-HEAD standing check is the backstop that catches it (no `build:` line / hash ≠ HEAD ⇒ run the check, then `brew unlink` again or rebuild).
- The 4090 half is unaffected (no Homebrew there; its failure class was pure staleness of the manual copy — rebuilt same day, `c3cf1865`).
- The stale-maintained-binary half is a separate, recurring problem with an existing rule: rebuild at the pinned release set when the stamp trails HEAD (this session ends with one).

## Option 3 — landed (2026-10-07, the Claude-verdict REVISE designs, rounds 1+2)

- `build.rs` + `build_commit.rs` (shared module — build scripts cannot host tests; `tests/build_commit_logic.rs` compiles it): `capture_commit` = `git rev-parse --short=12 HEAD` + a `-dirty` suffix scoped to **compiled paths only** (`src`, `build.rs`, `build_commit.rs`, `Cargo.toml`, `Cargo.lock`, plus the embedded-asset dirs `assets` and `tests/fixtures` — both are `include_bytes!`/`include_str!`'d into the binary; `--untracked-files=no` — a bare status in this shared worktree is always dirty from sibling `.plans`/`.issues` edits and would say nothing). Any git failure → the honest `unknown`, never a build error.
- **The rerun contract (round-1 premise error, the verdict caught):** the first `cargo:rerun-if-changed` line disables cargo's any-file default, and the OLD build.rs already emitted two — so "emit no rerun line" was impossible. The list is now explicit and covers every stamp-moving input: the compiled paths + the git state files resolved via `git rev-parse --git-path` (worktree-safe — `.git` is a FILE in a `git worktree` checkout): HEAD, the reflog `logs/HEAD`, the `refs/heads` DIRECTORY (the one watch that fires even when the branch's own loose ref is packed away), packed-refs, and the branch's ref file; detached-HEAD CI has no branch ref — HEAD + logs/HEAD still cover it.
- **⛔ Round-2 blocking fix — every watched path is EXISTENCE-FILTERED:** cargo treats a MISSING `rerun-if-changed` path as perpetually dirty (probed by the reviewer: `Dirty p: the file 'x' is missing` on EVERY build), so an unconditional `--git-path` result would have recompiled the crate on every build the moment `git gc` packs the loose branch ref away. A path that appears/vanishes costs ONE absorbing re-run, never a perpetual one — pinned by `noop_rebuild_reports_fresh_even_after_pack_refs`.
- **MEASURED (the round-1 verdict's ask, 2026-10-07, dev profile):** a build-script re-run with byte-identical output DOES recompile the crate — `touch Cargo.toml` (mtime-only) → build → **1 × `Compiling riir-refine`**; the identical re-run → 0. So the rerun watches make every commit re-stamp AND recompile the lib on the next build, docs-only commits included — the correct cost: after a commit the binary corresponds to a new commit, and a stamp that did not move would be the stale-lie class this fix guards.
- `--version` block: lines 0/1 byte-stable (`cargo-refine <version>`, `features: …`), then `build: <stamp>`. The shipped 0.2.1 Cellar binary predates the line, so a MISSING line is a detector for pre-2026-10 binaries — but only once; **the standing check is the comparison**: `cargo refine --version` → strip `-dirty` → must equal `git rev-parse --short=12 HEAD` in the repo you meant to run; missing line / `unknown` / mismatch = the shadow **or a stale maintained binary** (same remedy: absolute path + rebuild; `which -a cargo-refine` tells the two apart). ⚠ Scope: the stamp names THIS repo's commit only — sibling path-dep commits compile in and are outside it. Pinned as `tests/build_stamp_cli.rs::version_names_the_build_commit_and_it_matches_this_checkout`.
- Gates: lib 2469/0 (default) · `build_commit_logic` 11/11 (temp-git fixtures + the e2e arms: broad commit→rebuild, the ISOLATED arm `committing_without_touching_files_moves_the_stamp_by_itself` (only the ref watches can move that stamp), Fresh-after-pack-refs, every-emitted-path-exists incl. the real checkout) · `build_stamp_cli` 6/6 at `latent_retrieval,kat_service` · clippy `-D warnings` clean at default, all-features, and the release-set postures (lib+all-targets). **Verdict round 3: AGREE** (reviewer re-ran the suite independently; landed `3a9d1e96`). Known edge, accepted: a repo on git's reftable format has no loose `refs/heads` files — only HEAD + `logs/HEAD` fire there, degrading to a stale stamp, never a build error (this workspace is on the default ref format).

## The shape

`/opt/homebrew/bin/cargo-refine` (a Homebrew Cellar install from the public
tap, `../Cellar/cargo-refine/0.2.1/bin/cargo-refine`, installed 2026-10-05
23:30) precedes `~/.cargo/bin` on this box's PATH — so bare `cargo refine`
runs the RELEASED 0.2.1 build, not the maintained
`~/.cargo/bin/cargo-refine → ../git/riir-refine/target/release/cargo-refine`
symlink. Every post-release fix in this repo's working tree is invisible to
the subcommand that ran: measured live, the Cellar binary rendered NO
purchases section for `account --history` (it predates the B7 render,
`c80a5794`) while the maintained binary parsed and rendered the same
deployed worker's page correctly. `--version` reports the same 0.2.1 +
release-set-complete stamp on BOTH — the build stamp cannot distinguish
them (it stamps the compiled feature set, not the commit), so the shadow is
silent.

Scope of the exposure: it is not just convenience — the global lint-heal
rule (`cargo refine` as the dev-time default across the workspace) resolves
through PATH, so heals run the release build. Nothing breaks today (both
binaries report the same version + feature set; neither carries
`clippy_verify` — consistent with C1 not having landed), but fixes made
after the release are invisible until the next tag.

## The verdict's interim ruling (2026-10-06, session `1123c233`)

- **Do NOT `brew unlink` this session** — machine-wide state shared with
  sibling sessions, and the Cellar install may be the owner's deliberate
  check of the public release lane.
- **Until decided: smokes and gates that care call the binary by ABSOLUTE
  PATH** (`~/.cargo/bin/cargo-refine`). Recorded precedent: the dapps 018
  deploy-ledger smoke note.

## Options for the owner

1. Keep both; document the precedence (the tap install is the "stable"
   lane, the symlink the dev lane) — smokes/gates pin the absolute path.
2. `brew uninstall cargo-refine` on this box (the symlink becomes the only
   cargo-refine) — the cleanest dev posture; re-install via the tap is one
   command when the release lane is being tested.
3. Ship the build stamp with the commit (build.rs already derives the
   feature set from cargo's own exports; adding
   `CARGO_GIT_HASH`-class env would make `--version` name the commit and
   the shadow detectable) — helps every box, not just this one.

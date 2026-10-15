# Issue 164 — the install-probe plain-copy hazard: install.sh/ps1 (gist-rs/cargo-refine) clobbers the developer symlink silently

**Status:** OPEN — checklist-convention half LANDED 2026-10-14 (`.plans/223` §R6 + `.plans/105` Notes); remaining = the dist-repo installer edit (owner-adjacent lane, rides the next release cut or a deliberate dist-repo pass) + the AGENTS.md UNLANDED-line update once it lands
**Claimed:** zed-m3-idle, 1792087500, expires 2026-10-17T18:05Z
the in-repo optional polish (post-swap symlink courtesy notice, `upgrade` path) — **LANDED 2026-10-15** (the optional polish arm, done): `UpgradeReport.replaced_symlink_target` captured pre-swap (`read_link`), `symlink_notice()` names the preserved `.bak` link + the platform restore command (`ln -sf` unix / re-copy Windows), printed by both consumers (`upgrade` + the auto-upgrade hook); unit test `symlink_notice_names_target_backup_and_restore`; clippy `-D warnings` green lib + bin at the release-shaped feature set

## The hazard (measured twice, both boxes, 2026-10-13)

The documented developer convention (AGENTS.md §Production adoption): the global
`~/.cargo/bin/cargo-refine` is a **symlink** to the local checkout's
`target/release/cargo-refine`, so every rebuild propagates automatically.

The public installers (`install.sh` / `install.ps1` in **gist-rs/cargo-refine**, the
Plan-105 source-free dist repo) **copy** the release tarball binary to the same path.
When a developer box runs an install probe (testing the installer, or the v0.3.x cut's
probe run), the copy **silently replaces the symlink with a plain file**:

- The global path then serves the RELEASE-CUT bytes (stale corpus, missing new
  commands — the compiled-out class) while `--version` still prints a plausible stamp.
- A subsequent local `cargo build --release` propagates NOTHING to the global path —
  the rebuild runs green while the global binary stays frozen at the release cut.
- The tell (caught on the M3 2026-10-13 08:0x, confirmed on the 4090 2026-10-13 ~21:xx
  in the cycle-60 post-mining sweep): audit bin-age says STALE while the developer
  believes they rebuilt; `ls -la` shows `-rwxr-xr-x` where `lrwxrwxrwx` is expected.

Evidence rows: `.distill/001_mining_queue_snapshot.md` cycle 60 (the 4090 finding —
global copy served B224 bytes while HEAD carried B225-B228 + the `corpus-pr` command;
discharged by release-set rebuild + explicit `cp`) and the M3 10-13 08:0x idle pass
(the plain-copy root cause + symlink restore).

## The Windows compounding (why "just re-symlink" is not the remedy here)

On a NON-ELEVATED Windows shell the symlink is **unenforceable**:
`MSYS=winsymlinks:nativestrict ln -s` → `Operation not permitted` (measured, this box).
The Windows developer-box posture is perforce a COPY that must be re-copied after every
rebuild (`cp target/release/cargo-refine.exe ~/.cargo/bin/cargo-refine.exe`) — recorded
in AGENTS.md §Production adoption. The installer hazard still applies (it re-freezes
the copy at release-cut bytes), but the detector is the same either way:
`scripts/self_evolve_audit.sh` §0 bin-age (global-path mtime vs last src commit).

## Fix spec (the dist-repo edit — gist-rs/cargo-refine)

The installers should become **symlink-aware** at the destination, preserving the
developer convention without changing normal-user behavior:

1. Before writing `~/.cargo/bin/cargo-refine` (`.exe` on Windows): if the destination
   is a symlink (test -L / `[IO.FileInfo]::LinkType`), resolve and print its target,
   then MOVE the symlink aside (`cargo-refine.symlink.bak`) — never silently overwrite.
   The install proceeds (the user asked for it); the notice names the restore command
   (`ln -s <old-target> ~/.cargo/bin/cargo-refine`).
2. If the destination is a plain file: current behavior unchanged (normal users).
3. `install.ps1` additionally: after install, print the bin-age detector hint for
   developer boxes (one line, gated on the destination having been a symlink).

Also check the `cargo refine upgrade` swap path (src/build_stamp.rs / update_check
lane, THIS repo): the backup-swap (`mv` → `.bak-<old>`, write new) — verify whether a
symlinked global path is moved-aside cleanly (preserving the symlink in `.bak`) or
clobbered; if clobbered, the same awareness belongs there (this half IS in-repo).

## Why filed here (not edited directly in the dist repo)

- The dist repo is the PUBLIC product surface; its install scripts ship to users
  directly. The edit is owner-adjacent (the release lane / a deliberate dist pass),
  not an idle-loop unilateral change.
- The in-repo half (`upgrade` swap path verification + any fix) can proceed
  independently when this issue is picked up.

## Acceptance

- [ ] install.sh/ps1 (gist-rs/cargo-refine) symlink-aware per the spec; probe re-run
      on a developer box leaves the convention intact (or loudly preserved-as-.bak)
- [x] the `cargo refine upgrade` swap path verified against a symlinked destination
      (this repo) — **VERIFIED 2026-10-13: NOT clobbering.** `StagedUpgrade::install`
      (`src/update_check.rs` L723-730) uses `rename(exe, .bak-<version>)` then
      `rename(new_exe, exe)`; rename moves the SYMLINK ITSELF (both platforms), so the
      dev link is preserved in `.bak-<version>` and the failure path restores it.
      Residual polish (optional, not owed): post-swap courtesy notice when the old exe
      was a symlink (name the preserved `.bak` link + the restore command) — the
      semantics still flip destination to a real file after upgrade, but with a
      recoverable trail. — **LANDED 2026-10-15** (`zed-m3-idle`): the courtesy notice
      now ships — `UpgradeReport.replaced_symlink_target` + `symlink_notice()` (one
      formatter, both consumers), the unix notice names `ln -sf '<target>' '<exe>'`,
      the Windows notice names the re-copy/mklink posture; test-pinned in-module.
- [ ] AGENTS.md §Production adoption hazard line updated to name the fixed installer
      versions (remove "UNLANDED" once the dist edit lands)
- [x] the R6 release-checklist convention line gains the re-link /
      re-copy step for developer boxes — **LANDED 2026-10-14:** `.plans/223` §R6
      now carries the standing DEVELOPER-BOX CONVENTION (after any probe:
      re-link unix / re-copy Windows, verify `lrwxrwxrwx` + build-stamp==HEAD;
      `self_evolve_audit.sh` §0 bin-age named the standing detector) + a
      one-line hazard pointer in `.plans/105` Notes.

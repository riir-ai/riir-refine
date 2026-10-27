# Issue 181 — the hourly `\gitsync` task wedges repos when `git pull` dies mid-checkout on a locked file (2 incidents 2026-10-27)

**Status:** OPEN — root cause IDENTIFIED with log evidence; self-heal patch APPLIED to `E:\git\sync-repos.cmd` (backup `sync-repos.cmd.bak-20261027`); closes after the first real wedge self-repairs in sync-repos.log, or 30 days incident-free.

## The class

`E:\git\sync-repos.cmd` (scheduled task `\gitsync`, registered 2026-10-01 by SHIKUWA\katop, ~hourly; observed 5 runs on 10-27: 06:29 / 07:53 / 09:02 / 09:52 / 10:11 +07) runs `git fetch` + `git pull --ff-only` over 27 repos. When the pull's checkout cannot create a file — on Windows, any open reader handle (a hung Zed agent stream's `cat`/`rg` on the file, AV scan, search indexer) makes the create fail with `Permission denied` — git dies MID-CHECKOUT: the other files are already rewritten (content from origin materialized in the worktree at identical mtimes) while **HEAD is never moved**. Every subsequent `git pull --ff-only` then aborts ("Please move or remove them before you merge") and the repo stays wedged until a manual zero-loss repair. The old script had no recovery: the wedge persisted across runs.

## Evidence (both incidents the same file, same error — `sync-repos.log`)

- 07:53:09 run: `Updating 689a69b1..0d8f3791` → `error: unable to create file .distill/001_mining_queue_snapshot.md: Permission denied` → "Updating files: 100% (15/15), done." — the **shikuwa-4090-binage** incident (B295 content materialized, HEAD unmoved at 689a69b1; repaired manually that morning via the update-ref path after byte-identity verification).
- 09:52:03 run: `Updating 518237b6..b7c89b1a` → same error, same file → second incident (9 files written at mtimes 09:53:20, ref unmoved, the 1.88 MB snapshot left missing/D; repaired manually this session, `git status` clean at `2d9cd070`).

The target is always `.distill/001_mining_queue_snapshot.md` — the 1.88 MB rolling resume state, the most recently-modified large file in the repo at almost any moment (every idle cycle appends a row), and the file every idle cycle READS at startup — so it is simultaneously the most-likely-locked file and the one a mid-checkout death hurts most.

## The patch (applied 2026-10-27, additive, safe-by-construction)

In the per-repo loop, after `git pull --ff-only`: `if errorlevel 1 call :selfheal %%R`. `:selfheal` gates on the EXACT wedge signature — `rev-list --count @{u}..HEAD` == 0 (nothing ahead) AND `HEAD..@{u}` > 0 (behind) — anything else (real divergence, ahead commits, fetch failure) is logged and left untouched exactly as before. On the wedge shape:

1. `git reset @{u}` — mixed reset: branch ref + index move to upstream, **worktree bytes never touched** (sibling WIP stays as M rows; untracked stays untracked).
2. `git checkout HEAD -- <file>` for every file `git diff --name-only --diff-filter=D HEAD` reports — restores ONLY tracked files missing from the worktree (the snapshot, in both incidents). Modified/untracked files are never read.

Recipe verified: semantically on a throwaway clone (reset left worktree bytes intact, D-file restored, final status clean, HEAD==upstream) and in situ on the real 09:52 wedge (the manual repair this session used the same sequence after verifying all 9 materialized files byte-identical to origin; `git status` clean immediately after).

## Notes

- The script is unversioned (E:\git root, not a repo) — this issue + the snapshot cycle row are the record. Revert = `cp E:\git\sync-repos.cmd.bak-20261027 E:\git\sync-repos.cmd`.
- Verdict lane: `request_verdict` returned `Authentication required` (claude_code reviewer auth unavailable from this session). Applied under the owner rule's fallback ("ask Claude for verdict **and make decision**") — decision: apply, with backup + this record, because the patch only fires on a failure shape that today burned two manual repairs, and the both-boxes-synced mandate is the script's own purpose. Flagging for the owner in case the auth posture should change the call.
- Residual hardening (NOT done, owner-side if wanted): reduce the lock window itself (e.g. idle cycles reading the snapshot through a non-handle-holding path), or a second sync pass retry ~60 s after any self-heal.
- The script file needs CRLF (written via unix2dos; verified `DOS batch file, ASCII text, with CRLF line terminators`). A full test pass ran clean at 10:47 +07 (rc=0, every repo processed, batch syntax valid — the self-heal arm is gated on a real failure and has not fired yet; that first real fire is the close condition).

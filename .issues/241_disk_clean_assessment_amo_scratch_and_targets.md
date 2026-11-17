# Issue 241: disk-clean assessment — amo scratch + target survey (2026-11-17)

**Status:** Open — housekeeping assessment (list-first, deletions per the disk-clean contract).
**Claimed:** zed-c0c35a68@amo, 1794946800, expires 2026-11-19T02:20:00Z

## Trigger

Protocol H step 4 tripwire fired: `df -h /` avail **96 Gi** (< 100 GB) at session start
(2026-11-17 ~02:05Z). The protocol's named zero-risk pick (`*/target/debug/incremental`)
was executed this session: **18.9 GB reclaimed → 113 Gi avail** (18 main-checkout
incremental dirs; the active session's `scratch/sge014gate` deliberately skipped).
Incremental caches are regenerable and carry no work product — no approval class touched.

## Measurement trap disclosed (the a3t2 double-count)

`scratch/a3t2/*` and `scratch/s011b/*` are **symlink farms** — the "mirror" repos are
symlinks into `git/*`. A `du` that TRAVERSES a symlinked path (e.g.
`du scratch/a3t2/seal-remake/target`) reads the real repo's target and double-counts;
`du scratch/a3t2` (no traversal) shows the truth (~58M). Any future disk survey of
`scratch/` must resolve symlinks first (`readlink -f`) or it will report ~700 GB that
does not exist. First survey this session double-counted exactly this way; corrected in
the same session (no deletion was based on the inflated numbers).

## Assessment list (deletions need per-item approval per the disk-clean contract)

| Item | Size | Class | Note |
|---|---|---|---|
| `git/zed/target` | 154G | regenerable targets | release halves may serve source builds; confirm nothing runs from it before any prune |
| `git/seal-remake/target` | 187G | mixed | `target/release/seal-viewer` + `seal-bot-host` are LOAD-BEARING (the editor's RUN press + e2e lanes resolve them); only a `debug/`-half prune is admissible |
| `git/riir-ai/target` | 73G | regenerable targets | sibling lanes active in repo this window — defer to a quiet window |
| `git/riir-infer/target` | 31G | regenerable targets | sibling lanes active (the f16b retention walk rig cell ran 09:11Z) — defer |
| `scratch/sge014gate` | 59G | ACTIVE SESSION | captures modified 2026-11-17 — never touch while active; re-check next cycle (its `target/` 23G + two `target-clean*` 2.9G each are teardown-law items once the lane closes) |
| `scratch/t2wt-target`, `s011w-target`, `a3t2-target`, `t2wt-allf`, `t2base-allf`, `s011-target`, `t2wt`, `t2base` | ~21G total | stale landed-lane plan targets (teardown law: per-plan target dirs are rm'd when the plan closes) | lanes landed (s011 worktree HEAD reachable from develop); approval per item |
| `scratch/a3t2`, `s011b`, `s011` | small | symlink farms + registered worktrees | `a3t2/seal-game-editor` is a REGISTERED worktree with source WIP (`feat-pet/src/lib.rs` dirty) — adjudicate that WIP's owner before any worktree removal |

## Post-sweep state

`df -h /` → **113 Gi avail** (from 96 Gi). Tripwire satisfied; the table above is the
standing list for the next deep clean (owner approval or a closed-lane re-scan).

## Cleanup note

This session's own footprint: no new target dirs, no worktrees, no /tmp plans.
The incremental sweep's recompile tax lands on the next dev-profile builds (expected).

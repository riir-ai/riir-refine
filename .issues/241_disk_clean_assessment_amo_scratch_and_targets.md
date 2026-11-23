# Issue 241: disk-clean assessment — amo scratch + target survey (2026-11-17)

**Status:** Open — housekeeping assessment (list-first, deletions per the disk-clean contract).
**Claimed:** zed-133t1@amo, 1795456100, expires 2026-11-24T17:28Z
**Lease closed:** zed-c0c35a68@amo, 1794946800, was expires 2026-11-19T02:20:00Z (expired 2026-11-19T02:20Z, no landing - folded 2026-11-19 by the re-claim below; scope passed through unchanged)
**Lease closed:** zed-6addfb44@amo, 1795060553, was expires 2026-11-21T04:40:00+00:00 (scope discharged same session - the assessment landed 6e618b1c; the issue remains the standing deep-clean list for the next low-disk event)

## 2026-11-19 re-claim verdict (zed-6addfb44@amo, epoch 1795060553) — SAFE-class executed autonomously per the 2026-11-17 decree, 98 GB net recovered; the standing table re-measured with two stale rows dissolved

Running-process guard held at every step (one conservative false-positive self-match abort, re-checked clean on raw ps, zero cargo/rustc through the whole batch). df before -> after: **373 Gi -> 471 Gi avail**.

**Deleted (regenerable, measured per-path deltas):**
- /private/tmp stale attempt/plan scratch ×19 (63.4G): i932_fix_target 11.9G, i932_118i_tgt 8.0G, i932_mirror_target 7.7G, i932_mirror_118g 4.2G, i932_118f_amo 4.1G, i932_bisect_target 4.0G, i932_118e2 3.7G, i932_t18e3_repro 3.4G, sr1197-{ctrl,e2e} 3.6G, i1226f2{,b} 3.7G, i692_target 2.7G, i1208mint 1.6G, i1221t3d 1.6G, i1201_fixgate 1.5G, i377g2 1.4G, i1208_{fkl_smoke,corpus} ~0. Attempts 6-8 concluded RED-environmental (ledger landed); receipts live in the issues/logs — only the cargo targets went. **i701_* KEPT entirely (artifact + cap + target + logs, mtimes TODAY — possibly-active verdict lane, the same-day-mtime protection).**
- */target/debug/incremental ×13 (37.4G measured deltas: riir-ai 11.4G, seal-remake 4.2G, refine 3.7G, reflex 3.1G, mmorpg-examples 2.6G, train 1.8G, game-sdk 1.6G, katgpt-rs 1.6G, dapps 0.8G, neuron-db 0.6G, rethink 0.5G, auth 0.3G, reflexer 0.1G). The recompile tax lands on the next dev builds (expected, disclosed).
- rustup toolchains 1.91.0 + 1.94.1 (2.2G) — zero live references (grep over every rust-toolchain.toml; zed pins 1.98.1, the stack pins 1.99.0, 1.97.x kept at low-confidence/low-size).
- Post-touch verify: ~/.cargo/bin/cargo-refine resolves (0.5.0, build 1f2fdb6983df) — refine's release binary untouched.

**Standing-table re-measure (corrections):**
- git/zed/target — ROW DISSOLVED: the dir is ABSENT (zed checkout now 569M total; a prior session already cleaned it). 154G row was stale.
- scratch/sge014gate (59G) + the stale plan targets (t2wt-target, s011w-target, a3t2-target, t2wt-allf, t2base-allf, s011-target, t2wt, t2base ~21G) — ROWS DISSOLVED: all absent; scratch/ is now the pure symlink farm (0B per entry, the a3t2 trap discipline holding). The issue's 21G approval row is discharged by teardown, not deletion-authority.
- git/seal-remake/target 162G (debug 8.7G / release 21G) — still MIXED; release LOAD-BEARING (seal-viewer + bot-host); the debug-half prune stays admissible-but-deferred (active seal lanes, no disk pressure). incremental half executed above (4.2G).
- git/riir-ai/target 89G — sibling quest_combat WIP lane active IN the repo (11 dirty files); debug 67G stays deferred to a quiet window (incremental half executed, 11.4G).
- riir-train/data 42G — IRREPLACEABLE (GGUF pins), untouched.
- ~/.cargo registry src 4.6G + git checkouts 4.0G + registry src-side cache 664M — SAFE-class headroom, kept this run (healthy box; re-download cost not worth it at 471 Gi).

**Verdict:** tripwire not armed (373 Gi at start, healthy); the assessment fired via the 4090 lane's lease-expiry routing instead. The list-first contract is satisfied: SAFE class executed measured + reported, LOAD-BEARING/IRREPLACEABLE stay listed above. No approval class touched.
(claim scope: finish the list-first assessment - fresh census incl. the scratch/ symlink-trap discipline + per-item verdicts on the standing table; SAFE-class deletions per the disk-clean contract autonomous and measured, everything else stays listed for approval)

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

## 2026-11-20 re-survey (amo idle housekeeping lane, epoch 1795148454) — healthy box (356 Gi avail, tripwire NOT armed); six merge-gate worktrees found as ~703 G of UNLISTED candidate rows; list-only this cycle (today-mtime protection)

Fresh `du` census + per-worktree adjudication. The 11-19 table had none of these rows:
the `git/wt-mg-*` / `git/wt2-mg-*` merge-gate worktrees (ownership inferred — `mg-N` naming + the
sprint!083 gate landing today; the mtimes govern, never the inference). All six carry SOURCE
mtimes from this morning 07:06–08:30 → the never-delete-today law holds on every row:

| Path (registered wt of) | target | HEAD | today's activity | verdict |
|---|---|---|---|---|
| `git/wt-mg-045` (seal-remake) | 160G | `3654e0d9` MERGED (on origin) | `wrangler.jsonc` 07:06 | teardown-law leftover — WAIT for a quiet window (sprint!083 landed seal-sprints `5b30fd1` today) |
| `git/wt2-mg-045` (seal-remake) | 162G | `3654e0d9` MERGED | Cargo.lock churn 08:26 | same |
| `git/wt-mg-075` (seal-remake) | 160G | `32b7fdc39` **NOT-ON-ANY-BRANCH** | Cargo.lock churn 07:08 | ANCHORED: `hold/wt-mg-075` created in seal-remake; adjudicate the unlanded commit (land it or let the hold branch carry it) before any removal |
| `git/wt2-mg-075` (seal-remake) | 162G | `f7ff677dc` **NOT-ON-ANY-BRANCH** | `src/harness.rs` mtime 08:28, tree clean | ANCHORED: `hold/wt2-mg-075` created; same adjudication + the today-mtime hold |
| `git/wt-mg-065` (seal-game-editor) | 24G | `91283c500` MERGED | `feat-interaction/src/lib.rs` 07:09, tree clean | leftover once quiet (gate-lane fmt/build touch) |
| `git/wt2-mg-065` (seal-game-editor) | 35G | `91283c500` MERGED | same, 08:30 | same |

Candidate reclaim ≈ **703 G** (all `target/` trees; removal = `git worktree remove` + the target
dir per the teardown law, never while fresh source mtimes exist). Correction to the 11-19 row:
`git/seal-remake/target` measured **205G today** (was 162G — ~43G of release rebuilds + gate
lanes since). Deep-clean window = a day with NO fresh source mtimes under `wt-mg-*`/`wt2-mg-*`
AND the two `-075` commits adjudicated. The -045/-065 quartet alone is ~381G of pure leftover
build cache once quiet.

Same-session Protocol-H findings (context for the next lane): the seal-sprints checkout was
jammy — an unpushed local commit `600848e` ("fix(i067)" lease reshape) turned out to be a
STALE-TREE WHOLESALE REVERT (deleted live issues 039/084/085 + plan 069, stripped live claims
in 050/061/063/067, un-closed DONE plan 033, reverted HISTORY/TASKS); its legitimate intent
(UUID-blind lease reshape) was already landed properly on origin (the `d498106` note in
`.plans/067`). develop re-set to origin/develop `7daa9bc` (ff refused — diverged; hold branch
`hold/i067-stale-revert-600848e` preserves the commit for adjudication). seal-sprints now
synced + clean. riir-refine itself: clean, up to date, nothing to sweep.

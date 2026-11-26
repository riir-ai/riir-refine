# Issue 241: disk-clean assessment — amo scratch + target survey (2026-11-17)

**Status:** Open — housekeeping assessment (list-first, deletions per the disk-clean contract).
**Lease closed:** zed-1feab5f6@amo, 1796199000, was expires 2026-11-28T16:00:00+00:00 — the 11-26 pass executed SAFE-class autonomous (see its section below; scope discharged same session, issue stays the standing list).
**Lease closed:** zed-133t1@amo, 1795456100, was expires 2026-11-24T17:28Z — folded at the 11-23 pass's landing (scope: pre-large-run hygiene before the queued 6h realesrgan render + mist-bars exclusive; the wt-mg quartet rows dissolved — see below).
**Lease closed:** zed-c0c35a68@amo, 1794946800, was expires 2026-11-19T02:20:00Z (expired 2026-11-19T02:20Z, no landing - folded 2026-11-19 by the re-claim below; scope passed through unchanged)
**Lease closed:** zed-6addfb44@amo, 1795060553, was expires 2026-11-21T04:40:00+00:00 (scope discharged same session - the assessment landed 6e618b1c; the issue remains the standing deep-clean list for the next low-disk event)

## 2026-11-26 pass (zed-1feab5f6@amo, epoch 1796199000) — the standing `gate/` teardown row EXECUTED; ~120 GB SAFE-class deleted (185 → 211 GB avail under concurrent lane traffic)

Trigger: pre-large-run hygiene — amo carries 4 i1244 shard cells + a queued sprint!235 r4 full-guard landing (19 h est) + the holder's i1186 4090 staging; no sub-100 GB event (185 GB avail), the deletions ride the standing-row obligation, not the event trigger.

- **Deleted (SAFE-class: regenerable, 2+ days dead, zero lsof holders, no active/queued owner):** `seal-remake/target/gate` 111 G (zero inner files newer than 11-25; the 11-23 row "teardown when the gate cadence allows a rebuild window" — no gate in flight, the queued sprint!235 r4 lane rebuilds from scratch as part of its own 19 h run; cost disclosed, not hidden) + `/private/tmp/s085probe-gate` 5.2 G (dead 11-24 17:36; sprint!085 landed, p084 window closed) + `/private/tmp/sge084-target` 4.1 G (dead 11-23; plan 084 CLOSED + lease folded per seal-sprints TASKS.md — the teardown law's landed-ticket leftover).
- **Skipped (never-delete law held, per-dir freshness verified):** `/private/tmp/i1241` 4.5 G (stale amo-side build dir but the census lane is MID-FLIGHT on bmo — the name belongs to active sibling work, conservative skip); `/private/tmp/plan470` 5.0 G (ACTIVE riir-train plan's per-plan dir, Phase 1 not started — the plan-dir convention keeps it warm for its owner); `/private/tmp/s134-target` 4.5 G (sprint!134 queued-state uncertain — was queued 11-24, absent from today's queue, ownership unresolved); `/private/tmp/s235-*` 42 G + everything else fresh (the queued landing lane's own prep, touched today).
- **df reconciliation (the honest form):** gross deleted du ≈ 120 G; avail moved 185 → 211 GB (+26 G net) because concurrent lane traffic wrote ~big during the same window (i1244 measurement cells, s235-merge-target growth, the holder's `/private/tmp/riir-infer` staging — which ANOTHER lane shrank 57 → 14 G mid-pass). All three deleted paths verified absent post-rm, zero open handles, seal-remake/target 156 → 45 G measured. No missed deletion; the delta is traffic, not residue.
- **Standing-table update:** the `gate/` KEEP row DISSOLVED (executed); future merge-gate lanes rebuild it from scratch — the teardown-first cadence is now the standing expectation whenever no gate is in flight and the tree is >48 h stale.

## 2026-11-23 pass (zed-133t1@amo, epoch 1795456100) — quartet rows DISSOLVED by another lane; the gate-profile tree measured; 127 GB SAFE-class executed (201 → 321 GB avail)

Trigger: pre-large-run hygiene (the skill's "before large builds" class) — amo carries a queued 6 h 720p→5K realesrgan render + a 45 m exclusive mist-bars job; df was 201 GB and down from 356 Gi on 11-20.

- **Quartet rows DISSOLVED (not deleted here):** the four `wt-mg-045/wt2-mg-045/wt-mg-065/wt2-mg-065` merge-gate worktrees from the 11-20 survey (~703 G candidate) are GONE — removed by another lane between 11-20 and now (also explains most of the 356→201 drift being *net* growth, not gross). `git worktree list` shows only `/private/tmp/s111-mirror-seal/seal-remake` + `/Users/katopz/git/i1256-mirror/seal-remake` (both FOREIGN — s111 sprint mirror + the live i1256 lease; untouched).
- **The growth is the gate-profile tree** (`seal-remake/target/gate*`, measured this pass): `gate` 111 G + `gate-lane1` 49 G + `gate-lane2` 36 G + `gate-lane3` 28 G + `gate-exclusive` 14 G = 238 G — the guard_parallel one-target-per-lane discipline writing full gate targets into the MAIN checkout's target dir. **Deleted (SAFE, autonomous):** `gate-lane1/2/3` + `gate-exclusive` — 127 GB, newest inner file 2026-11-21T14:23Z (2+ days dead), no flock holders (lsof per `.cargo-lock`), no process names them, recreated automatically by the next `guard_parallel.sh` run. **Kept:** `gate/` (111 G, 16 h fresh merge-gate cache — deleting burns a full gate rebuild for the next merge-gate lane; new standing row: teardown when the gate cadence allows a rebuild window), `release/` 29 G (LOAD-BEARING viewer+bot-host, verified present post-touch), `debug/` 22 G (active seal lanes defer, standing), editor-run/dev-vibe/soak (lane-owned, small), wasm32+tmp (empty).
- **Unchanged verdicts:** `riir-ai/target` 95 G — sibling riir-games-mmorpg WIP STILL dirty (12+ files: quest_combat/authority_sim/genesis/strategy_grammar) → debug-half (73 G incl. 15 G incremental) stays deferred to a quiet window; `zed/target` 24 G — owner-built TODAY (zed-remote-server .gz 10:10 local) → never listed; `riir-train/data` IRREPLACEABLE (GGUF pins); cargo registries kept (healthy box).
- **2026-11-24 survey (zed-3d94a4e1@amo, read-only): 209 GB avail — no low-disk event, ZERO deletions.** The 321→209 GB regrowth (~112 G over 2 days) is explained by source: `/tmp` holds ~144 G across 12 lane/plan target dirs and **every one is touched within 48 h** (mtime or fresh inner files, verified per dir): `s020-w3-gate-target` 57 G (11-24 12:56), `zed-issue080-verify` 13 G (11-24 21:16), `i257` 13 G, `i1256` 13 G (live lease mirror), `s134-sr` 8.1 G + `s134-target` 4.5 G (sprint!134, queued on amo), `seal-e2e-matrix-20261124` 7 G (today's run), `sv125-target` 6.9 G, `sensanova-probe` 6.5 G (live rethink-068 lane), `s085probe-gate` 5.2 G, `wp3p2` 5 G, `plan470` 5 G. All modified by other agents today/yesterday — the never-delete law holds; the regrowth is ACTIVE LANE SCRATCH, not junk. Standing list unchanged; re-survey at the next sub-100 GB event.
- **Post-touch verify:** df 201 → 321 GB; seal-viewer + seal-bot-host resolve; `cargo-refine --version` fine (0.6.0).
- Housekeeping compaction note for the standing table: the **gate-profile tree is the new recurring consumer** — every `guard_parallel.sh` run writes 3–4 × 30–50 G lane targets. The build-hygiene law's isolated-target-dir discipline says gate lanes belong in `/tmp/<plan>`-class dirs, not the shared checkout target; routing future gate lanes to overflow dirs (or rm-ing lane targets at gate close, the teardown law) is the structural fix — a build-hygiene-skill item, not deletions each cycle.

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

## 2026-11-24 pass (amo hygiene lane, post-leftover-sweep) — survey current; SAFE-class partial (6.5G); the big rows are LIVE lanes

df before 187G avail (post-sweep). The 11-23 pass's quartet rows dissolved; the current big trees:

**LOAD-BEARING / LIVE (never delete):**
- seal-remake/target 177G + riir-ai/target 115G + i131-riir-ai/target 115G — the two product workspaces + the live sprint!131 mirror clone (its managed job runs from i131-sib/worktree). Profiling-invalidation law: profiling data is never invalidation data; these are the shipping postures. Next deep-clean candidate ONLY via a measured sweep pass with the lanes quiet.
- zed/target 24G (the box's Zed build), riir-refine/target 17G (the release-set binary's home — the LOAD-BEARING class), riir-train/target 16G, riir-infer/target 12G (bmo's i1241 census lane pairs with it), seal-game-editor/target 6.7G (m3's wp3p2 checkout rides this clone), seal-render 6.6G + i131-seal-render 6.6G.

**/tmp scratch (dated-run class):**
- sprint117-sibs 69G (mtime TODAY 16:04 — sprint117 lane LIVE today, gate sibs) — KEEP, lane's teardown.
- s020-w3-gate-target 57G (mtime TODAY 12:56 — sprint!020 wave-3 mist gate, lane live) — KEEP.
- i257 13G (today 20:09), i1256 13G (Nov 23 — the i1256-mirror's gate target; mirror itself torn down this pass, target kept one more day pending the lane's close), seal-view-084 9.2G + sge084-target 4.1G (sprint084 window OPEN — mist-bars merge pending), plan470 5.0G (today), i1241 4.5G (today, census), wp3p2 5.0G (the live wp3p2 lane's target), sv125-target 6.9G (fingerprints moved TODAY 19:41 — another agent's today-work; KEEP despite sprint125 having landed), s085probe-gate 5.2G (today 17:36).

**SAFE-class executed this pass:** seal-e2e-matrix-20261122-235251 6.5G removed (2-day-dated matrix run dir, receipts in .benchmarks). Plus the earlier sweep's i1221t4samp+_v2 2.56G + i1256-mirror 175M. df after 193G avail (+6G this pass, +9G session total).

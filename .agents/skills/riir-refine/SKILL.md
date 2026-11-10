---
name: riir-refine
description: Auto-continue idle loop + riir-refine batch mining/heal workflow for the riir-* workspace. Activate when the user says "continue", "auto continue", "keep going", "nothing to do", "idle", "next", "next batch", "Batch 32+", or when a session summary shows remaining riir-refine batches; when asked to fix clippy across repos, mine new rust/kernel perf rules, dry-run `cargo refine --suggest/--fix` on workspace repos and adapt proposers when suggestions are wrong, find/finish issues or plans, run boundary-guard/substrate-first routine checks, run benchmark regression sweeps, search online to unblock, or find related papers. Chains doc-sync + research skills at the right steps. EVERY activation starts with Protocol H housekeeping — sync git both boxes, kill orphaned git/ssh zombies, prune leftover worktrees, disk check — before the queue. On an auto_prompt context-overflow handoff the summary's (3) What remains section decides — Nothing means fall through to this loop, listed work means resume it first.
---

# riir-refine — the auto-continue idle loop

You are the idle-time worker for the workspace. When activated without a
specific task, run **Protocol H — housekeeping** below FIRST (sync, zombies,
worktrees, disk), then the priority queue, **one unit per cycle**, re-scanning
after each completed unit. **Commit + push each completed unit before starting
the next.**

**Not a hijack — auto_prompt handoffs (precedence).** A context-overflow
handoff opens a fresh thread whose message carries a `## 1. Thread Summary`
block and a `## 3. Decision` template saying "Continue remaining work...".
That wording alone is NOT the trigger — the summary's **(3) What remains**
section is. "Nothing for this ask" (+ at most optional "if wanted" items)
= genuinely nothing to resume → the Decision's own "check other repos for
unfinished tasks or do housekeeping" clause fires and THIS loop is the
correct continuation (the sanctioned auto-continue path, not a hijack).
Listed remaining work (unchecked `- [ ]`, `- [-]` defers, named
plan/issue filenames in the (4) active-plan section) → RESUME that first;
the queue only after the resume closes with nothing left. Never decide
from trigger words alone. Symmetric etiquette mid-loop: when the overflow
handler requests a summary, stop after the current unit and emit the
exact 4-part `## Summary` (Discipline below) — never open a new queue
unit past that request.

**Rule 0 — session context outranks the queue.** User-reported
regressions/bugs/questions in the current conversation always go first.
The queue is for idle time only.

**Rule 0b — REMOVED (owner's call, 2026-08-15 evening).** The former
00:00–11:59 overnight-window gate is deleted — the idle queue now runs
at **any hour**. Collision protection shifts from time-gating to the
standing discipline (re-scan folders at WRITE time, commit only your own
hunks, isolate builds under `/tmp`, never `git stash` sibling WIP — see
Discipline below). The Plan 337 measurement carve-out (Discipline below)
still applies whenever a measurement saturates the box, at any hour.

## Protocol H — housekeeping (EVERY activation, before the queue — owner mandate 2026-09-27)

Sync BEFORE anything else; then clear the process/worktree/disk debris that
past sessions left behind. Each step is read-only until a finding is
adjudicated. Findings are reported in the cycle's summary — deletions only
after the safety checks named per step.

1. **Sync git — both boxes, all repos.** M3: per repo under
   `/Users/katopz/git/*` (top-level dirs with a `.git` dir): `git fetch origin`
   + `git status -sb`; ff-only merge (`git merge --ff-only origin/<branch>`)
   CLEAN behind repos only. Dirty repos: fetch + report, never pull through a
   dirty tree (adjudicate the dirt first — see 3). katgpt-web + reflex-site
   ride `main`; obsolete/ repos are lineage-only — skip.
   4090 (`katop@100.85.179.44`, Tailscale fallback from `192.168.1.33`; LAN
   port 22 is often firewalled): ⛔ **never run `git fetch` directly inside a
   nested ssh session — git's ssh child inherits the session's open stdin
   pipe and hangs forever, leaving orphaned git.exe+ssh.exe pairs** (measured
   2026-09-27: 46+24 orphans from one sweep). Use DETACHED execution:
   `Start-Process -FilePath "C:\Program Files\Git\cmd\git.exe" -ArgumentList
   @("-C","E:\git\<repo>","fetch","origin") -RedirectStandardError
   E:\git\_sync\<repo>.err -WindowStyle Hidden`, then poll statuses in a
   SECOND ssh call. ff-only clean behind repos; discard machine-local
   Cargo.lock churn (`git checkout -- Cargo.lock`) only when the incoming
   commits touch it — it is build noise, not work.
2. **Kill process zombies — VERIFY before killing.** M3: `ps aux | grep -E
   "git|ssh|cargo" | grep -v grep` — a wedged pair is a git process whose
   child ssh holds a fetch with no CPU and no progress. Check parentage +
   start time BEFORE killing: a process whose parent is a live PowerShell /
   agent session belongs to that session — LEAVE IT and report; only kill
   confirmed ORPHANS (parent gone) and stuck-since-«minutes» pairs (a fetch
   takes seconds; 25 min stuck = dead). Never kill another session's active
   work — one wipe of "suspicious" processes once deleted a sibling's
   in-flight fetches mid-run.
3. **Worktree clean.** Per repo: `git worktree list` + `git worktree prune
   --dry-run` (prunable = stale registration — prune immediately). For each
   non-main worktree: `git -C <wt> status --porcelain` — CLEAN + detached →
   ⛔ ANCHOR-BEFORE-REMOVE (incident 2026-10-27: the sweep removed a staging
   worktree whose detached HEAD held the unlanded issue-099 fmt drain — the
   KEEP clause below covers branch-holding worktrees, and a detached HEAD
   with an unlanded commit is the same class of work): if the worktree's
   HEAD commit is reachable from NO branch and NO origin
   (`git branch --all --contains HEAD` empty + `git merge-base --is-ancestor
   HEAD origin/<default>` failing), create a holding branch in the MAIN repo
   first (`git branch hold/<wt-name> <sha>`) or KEEP the worktree — only
   then `git worktree remove --force`; DIRTY → report the dirt, remove only
   when it is plainly build churn (Cargo.lock), never source WIP. Also
   sweep `.wNNN`-style dirs beside the repos and temp-path worktrees
   (`/private/tmp/*`, `E:\git\wt*`, `C:\t980`). KEEP: anything holding ops
   credentials (the 4090's `E:\git\kat141` carries live signing keys),
   branch-holding worktrees whose branch has unmerged commits, and another
   session's checked-out worktree (parent = live session).
   ⛔ Shell discipline in this step (and everywhere): never chain a mutating
   git command after a `cd` in one line without `&&`-asserting the cd (a
   failed cd silently runs the rest in the WRONG cwd — measured: a misfired
   chain reset a sibling's WIP in a shared tree); never pipe a mutating git
   command through a filter (`| tail/head` masks the exit); verify `pwd`
   before any `reset --hard` in a shared worktree.
4. **Disk check → clean only when needed.** `df -h /` and `df -h
   /Volumes/SDXC1TB` — internal < 100Gi free OR SD < 100Gi free → invoke the
   `disk-clean` skill (its contract: LIST first, delete only user-approved
   items). Healthy disk → report the two numbers, delete nothing. The
   incremental-dir slice (`*/target/debug/incremental`, ~30G when dirty) is
   the standing zero-risk pick. LOAD-BEARING, never delete:
   `riir-refine/target/release` (the `~/.cargo/bin/cargo-refine` symlink
   target) and every repo's `target/release`.
5. **Build-hygiene quick pass (report-only).** Invoke the `build-hygiene`
   skill's §6 quick pass: per repo `du -sh target` (report the number; the
   step-4 LOAD-BEARING rule applies), the profile spot-check (`grep -c
   '^\[profile' Cargo.toml` vs the repo's own documented set — drift is a
   finding), binary sizes vs the last recorded, and a `cargo refine` DRY RUN
   on dirty files (mechanical findings before any perf claim). A finding
   files an `.issues/` row — this step NEVER fixes inline and never deletes.

Housekeeping findings (hung processes killed, worktrees removed, repos
ff'd) are reportable work — name them in the cycle summary — but they are
NOT a queue unit and never substitute for one: after housekeeping, proceed
to queue item 1.

## The idle priority queue

| # | Item | How |
|---|------|-----|
| 1 | **Fix open issues — ISSUE-FIRST GATE** | Scan `.issues/` in THIS repo first, then every sibling repo (scan order below), oldest first, skip deferred. Items 5/11/12 are FORBIDDEN while any actionable issue remains anywhere. Fix → validate → doc → remove file (noise-reduction rule). |
| 2 | **Finish in-flight plans** | `.plans/` with unchecked `- [ ]` — complete the next task group, honor `- [-]` defers. Rank plans from ORIGIN, never the working tree (Discipline: plan_read_origin). |
| 3 | **Clippy sweep, all repos** | Fix-driven where possible (see fix protocol below), manual `cargo clippy --all-targets` otherwise. **GATE (2026-10-28, the plan-306 census finding): a clean sweep is NOT evidence over gated code — plain clippy silently skips every `required-features`-gated target. After any clean sweep of a repo, run `cargo refine --oracle-census <repo>` — non-empty output = the sweep was BLIND to those targets: either re-arm the rulewright there (its gap-arming now covers up to 3 distinct sets by default) or run the repo's own all-features posture (`RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv from the census>`). The oracle-census binary must be fresh (bin-age law). |
| 4 | **Resolved-issue hygiene** | Find issue files whose work is verifiably landed but file remains → doc at `.docs`/AGENTS.md → `rm` the file. |
| 5 | **Batch mining + fix dry-run — GATED on 1–4 exhausted across ALL repos** | The centerpiece — see the two protocols below. If issues/plans exist anywhere, mining is a mispick, not a valid unit. |
| 6 | **boundary-guard routine check** | Run `scripts/ci_boundary_guard.sh` where present (riir-mmorpg-examples); otherwise the 7-surface grep table from the boundary-guard skill. File violations as `.issues/` — do NOT fix in the same commit as detection. Record the run in the skill's Run log. |
| 7 | **substrate-first drift audit** | Mode 2 of the substrate-first skill: inventory substrate primitives, grep consumers for parallel systems (3+ vocabulary variants!), classify (false positive / POC simplification / DRY violation / architectural), file issues. |
| 8 | **Benchmark regression sweep** | GPU-exclusivity check FIRST (see discipline), then pinned suites; compare vs `.benchmarks/` baselines; drift → bisect → root-cause fix → new bench file. |
| 9 | **Online unblock search** | For each blocked/`[-]` item: web search the error/blocker, fetch via `https://r.jina.ai/...` per global Search rules. |
| 10 | **Defer-marking sweep** | Plans/issues with silently-stalled tasks → mark `- [-]` WITH a reason + unblock condition; file a follow-up issue when the blocker is external. |
| 11 | **Online rust/kernel perf search** | rust-optimize checklist, nnethercote/perf-book, bevy, CubeCL/burn, wgpu, CUDA best practices → candidate rules → into the batch pipeline. |
| 12 | **Related-paper research** | Hand off to the `research` skill (its pre-flight + prior-art gate are mandatory — ad-hoc "search online" is the #1 false-PASS mode). |
| 13 | **doc-sync pass** | Invoke the doc-sync skill against repos with landed-but-undocumented work. |
| 14 | **Post-mining sweep** | Mine improvement candidates from the rulewright's OWN usage artifacts (Protocol C below) — misses, created-warnings, verify reverts, memory.json tier stats, score-bench delta. Contrast with items 5/11/12 which are PRE-mining (external sources). |
| 15 | **Build-hygiene gate** | Invoke the `build-hygiene` skill (canonical home `katgpt-rs/.agents/skills/build-hygiene/`, the PUBLIC shareable copy): the five-profile discipline (dev line-tables-only + opt-3 deps, gate opt-0 shared cache, dist fat-LTO strip), target-dir size, binary size, the dev-loop driver posture (plain cargo vs framework dynamic-linking vs blaze), and the fix-before-perf law (`cargo refine --fix` where installed, else `cargo clippy --fix`). Quick pass rides Protocol H step 5; THIS item is the FULL pass — quarterly, or after any profile/feature-set/crate-count change. Run BEFORE item 8's benchmarks — a bloated target tree or a stale profile invalidates every timing row; re-run the workspace's dev-loop A/B harness (create one before adopting any driver) and re-time the quick gate layers vs the last recorded baseline. |

Items 6 + 7 run BEFORE item 8 (benchmarks) by design — boundary/DRY
findings are cheaper to fix before you benchmark around them. Item 15's
full pass also runs BEFORE item 8 whenever it fires — build hygiene first,
benchmarks second.

**Scan order + racing partition (why: 2026-09-04, three idle sessions
all raced to mining while zed had 6 open issues — "across repos" was
never grounded in actual repo roots).** The workspace spans TWO roots;
enumerate both, never guess from cwd alone:
1. **This repo** — the checkout your session runs in: `ls .issues/`.
2. **Sibling repos, both roots** — `ls -d /Users/katopz/git/*/.issues`
   AND, when this session's root differs, `ls -d <session-parent>/*/.issues`
   (e.g. `/Volumes/SDXC1TB/git` holds zed + the zed-mcp-server-* extensions).
   Oldest issue first across the merged list.
3. Item 2 (plans) follows the SAME order; items 3–4 next.
4. Item 5 (mining) + 11/12 (pre-mining research) ONLY when 1–4 are
   exhausted across ALL repos.

**One repo per session** — N idle sessions must not pile on one repo
then all fall through to mining. Before picking, partition: if `ls -lat`
or `git status` shows a sibling session mid-flight in repo X (Issue-665
discipline), take repo Y's issues, not X, and never mining as the
diversion. If ALL repos' issues/plans are claimed by live sessions,
claim an uncontended item from 3–4/6–14 instead — mining stays gated
until the claims clear.

## Protocol A — batch mining (queue item 5) — PRE-mining (external sources)

0. **Create the batch plan FIRST** — `riir-refine/.plans/NNN_*.md` (next
   number per `.plans/.highwater`, re-scan the folder at write time) with
   `- [ ]` checkbox tasks for: dedup, entry authoring, standing gates,
   count pins, GOAT (G1/G2/G3 + retrieval), research note, docs/AGENTS/
   queue-snapshot updates, commit+push. Mark `[x]` as tasks complete,
   `- [-]` for defers with a reason. Adopted 2026-08-18 (owner call;
   canonical retroactive plan: 006 for Batch 60 — every batch after it is
   plan-FIRST).
1. Read the **mining queue snapshot** at `.distill/001_mining_queue_snapshot.md`
   (repo root) + `ls` the target tree. Pick
   the next unmined cluster (1–2 files, ~8–12 rules). NEVER mine a
   sibling-blocked file.
2. **Dedup grep BEFORE proposing** — grep the existing corpus
   (`src/domains/kernel_opt/`, `src/domains/rust_perf/`) for each
   candidate concept. Batch 30 dropped 13 candidates as
   duplicates/adjacent; that filter is load-bearing. **Upstream-state
   re-check at authoring (P113):** a fix-shaped lead ("the repo should
   do X") re-runs its one-line proof — `git log -S<mechanism-token>` /
   blame at the pin: ABSENT = open fix space (outbound-PR eligible),
   PRESENT = their landed decision (mineable decision content ONLY,
   never a PR) — even when the distill verdict already carried it (the
   verdict and the authoring can be days apart).
3. Spawn **2 parallel subagents** to read the target file(s) + propose
   rules (pattern, bounded fix-space, category, rationale, source
   quote). **Re-verify every quoted line against the real file** —
   Batch 31's full-quote re-verification caught drift before it landed.
4. Add corpus entries + extend the standing syn-parse gate
   (`batchNN_pseudocode_parses`).
5. **GOAT** — a batch is NOT done at corpus-add time:
   - G1: parse/syn check (all new rules)
   - G2: lib tests pass
   - G3: no-regression — clippy 0 in BOTH feature states + CI guard
   - **freeze-table regeneration (clippy-corpus batches, plan 137 T7.5):**
     any `build_seed_corpus()` delta moves the deterministic stride windows —
     regenerate the gen2/gen3 freeze tables in the SAME batch (the
     `#[ignore]`d `generation_v2_freeze`/`generation_v3_freeze` e2e, which
     spawn real clippy; labels/exclusions land as data) — the freeze-status
     gates red on develop by design until then. Record paragraphs:
     `src/score_bench/generations.rs`.
   - **retrieval-precision re-measure (Issue 017, kernel batches):** run
     `CARGO_TARGET_DIR=/tmp/riir-refine-bNN cargo test --features
     kernel_opt,latent_retrieval --lib retrieval -- --nocapture` (~1 min) —
     the gate floors (re-pinned by Issue 022/Bench 024: kernel self-variants
     100%, blended 3-domain 77.1/86.4/92.4) must hold; a decay FAILS the
     batch → fix retrieval
     quality, or re-pin the floors + record the delta in the Research note
     (never weaken silently).
   - **retrieval regression gate at the DEFAULT posture (Issue 170 T3,
     corpus batches — the lane that was missing):** any corpus delta (any
     domain) also runs Layer 5b2's command at the batch's landing —
     `cargo test --lib --features latent_retrieval retrieval_` (~2 min
     debug; the gate + the fixture-label verifies + determinism). The
     router/blended/clippy/perf floors are pinned at the DEFAULT state (the
     `router_annotation_floor()` `(false, false, true)` arm), and this is
     the lane that catches their decay — the B248–B271 waves sat RED here
     for 11 days because every standing lane compiled the gate out. A decay
     FAILS the batch → re-measure + re-pin with the delta recorded (the
     Issue-020 contract), or fix retrieval quality. The per-span dump +
     vocab probe (`tests/router_per_span_dump.rs --nocapture`, diffed
     against a worktree at the last pin) is the attribution instrument.
   - rust_perf rules: the Pareto bench (`rust_perf_bench` feature) +
     `context_notes` annotations on regime-fragile/Mixed verdicts;
     demote/annotate losers. Never promote a perf gain on a wrong result.
6. Write the Research note (`.research/NNN_*.md` — next free number),
   bump the corpus count in `AGENTS.md`, update `.docs/11_domains/`.
7. Commit `feat: batch NN: ...` + push. Update the mining queue snapshot
   at `.distill/001_mining_queue_snapshot.md` (same commit). **If the batch
   touched rulewright source, the maintained binary must be rebuilt BEFORE the
   push** (the Issue-153 release feature set; verify with §0 of
   `scripts/self_evolve_audit.sh` or `cargo refine --version`'s `build:`
   hash == HEAD) — the B248 lesson: a landed corpus the global
   `~/.cargo/bin/cargo-refine` does not serve silently fixes with the OLD
   rules until the next audit catches the bin-age.

## Protocol B — fix dry-run on workspace repos (queue items 3 + 5)

```bash
cd /Users/katopz/git/riir-refine
cargo run --features latent_retrieval --bin cargo-refine -- --suggest <target-repo>/src/
cargo run --features latent_retrieval --bin cargo-refine -- --fix <target-repo>/src/          # ⚠ WRITES — --fix implies --write (the just-works UX, 2026-09-08)
cargo run --features latent_retrieval --bin cargo-refine -- <target-repo>/src/                # the DRY RUN (bare default, zero edits)
```

⛔ **`--fix` is a REAL fix — the "preview" reading cost a measured incident
(2026-10-30, amo idle lane):** a fix run at the all-features posture
believed the old preview text and invoked `--fix --verify`; the fix phase
WROTE 83 files before any gate, and the verify's baseline guard (the
tree was broken at that posture) then refused to classify or revert
anything — unverified edits in the tree until the gates were run by hand
(they were green; landed `aec6c9a9`). The bare command is the dry run;
`--suggest` is the explicit review; `--fix` commits to writing. The
cargo-refine skill's Usage block has said this since 2026-09-08 — this
protocol was the stale copy.

- Validate in the TARGET repo: `cargo clippy --all-targets` +
  `cargo test --lib`. Use `CARGO_TARGET_DIR=/tmp/<repo>-fix` if the
  repo's target dir is locked by a sibling build; clean it up after.
- **The auto-oracle gap-arms by default (2026-10-28):** a fix dry-run's
  oracle now runs up to 3 extra feature postures over the
  required-features census — read its stderr lines: `N gated target(s)
  invisible` = what the base posture cannot see; `M gap pass(es) armed`
  = what ran anyway; `L remain UNCOVERED … RIIR_REFINE_AUTO_ORACLE_FEATURES=<csv>`
  = arm the remaining sets by hand when the findings matter. A fix-verify
  on gated edits still needs the matching `--verify-args` (the P43 tail
  names it).
- **Feature-aware validation (the bevy_shared lesson, 2026-08-15):** a
  cfg-gated edit site is INVISIBLE to default `cargo check`/`--all-targets`
  when the gate is a non-test feature — an E0432 sat green in
  `riir-game-sdk/src/backend/bevy_shared.rs` until validated with
  `--features backend_bevy`. After a fix, compile the edited files under
  the features that actually include them (grep the `#[cfg(...)]` on the
  enclosing `mod`); a passing default check proves nothing about gated
  code. Same class as the count-assertion rule below.
- cfg-gated test files: assert COUNTS, not exit codes — a green run can
  mean "ran nothing" (riir-chain lesson).
- **Wrong suggestion → fix the proposer at root cause** (rule/pattern in
  `src/domains/*/proposers/`), add a fixture regression test, re-run.
  NEVER suppress the lint, NEVER hand-edit past it without a fixture —
  the rulewright must adapt each iteration, not accumulate suppressed lints.
- Commit separately: `fix: refine <repo> ...` (target repo) and
  `fix: proposer ...` (riir-refine).

## Protocol C — post-mining sweep (queue item 14 — usage artifacts)

Pre-mining (Protocol A) distills rules from external sources BEFORE usage.
Post-mining distills proposer extensions + new rules from artifacts that only
exist AFTER usage. Intake lives in `.distill/001_mining_queue_snapshot.md`
§"Post-mining intake" — appended by dev-time fix sessions (the `cargo-refine`
skill points agents there). Per sweep:

1. Gather (one command: `scripts/self_evolve_audit.sh` — Issue 037's
   instrument): `~/.cache/riir-refine/memory.json` (trajectory counts by lint,
  tier/outcome distribution), `.benchmarks/score_history.jsonl` delta,
  `.refine/misses/`, verify-gate revert reports, grep target-repo session
  records for `divergence|backlog|miss`.
2. Triage each open class: **probe against real clippy FIRST** (temp crate,
  JSON output — the established discipline; a rule without probe evidence
  does not land, Batch 33's rejected seed is the precedent).
3. Fix the proposer at root cause + fixture regression test + re-run
   `--score-bench` (the history accumulates; never weaken floors silently).
4. Highest-value recurring classes are pre-ranked in the intake file —
   consume top-down, mark each DONE with its commit hash.

## Discipline (non-negotiable)

- **Routing restriction (owner, 2026-09-04): do NOT trigger riir-train for
  other tasks — only `riir-refine`-related units are allowed.** Read-only
  citations of riir-train records stay fine; never initiate fixes, gates,
  benches, or doc edits there. Sibling-owned riir-train lanes belong to
  their owning sessions.
- **Commit + push per unit** — `feat:`/`fix:`/`docs:` prefixes; stay on
  `develop` (every repo — riir-train flipped from `main` to develop + default
  on 2026-09-04); rebase non-interactive only.
- **Numbering**: read `.highwater`, +1, AND re-scan the folder at WRITE
  time — concurrent sessions dual-allocated 675 AND 677 in one night.
- **Queue-item dual-allocation (katgpt-rs Issue 665, 2026-08-17)**: two idle sessions
  can pick the SAME queue-1 issue within minutes. Before writing any code,
  `git status` + `ls -lat` the target crate for uncommitted hunks / fresh
  mtimes that match your chosen issue (a feature line, a module gate, a new
  file with different design names = someone is mid-flight). If sibling
  hunks appear AFTER you started (a duplicate feature key materializes
  beside yours), YIELD immediately: revert ONLY your hunks, delete your
  untracked files, verify the tree is 100% theirs, pick a different unit.
  Never commit through a duplicate (two `feature = []` keys = broken
  manifest for both sessions).
- **Claim-marker protocol (RATIFIED 2026-10-11 via the verdict reviewer; trial:
  Batch 221's claim `80327d3f`; root cause: the three-lane collision record —
  the dual-allocation gate cannot see another box's unfetched commits):**
  before authoring a batch, append a one-line claim row to
  `.distill/001_mining_queue_snapshot.md` naming the queue head + your batch
  number, COMMIT + PUSH it BEFORE authoring begins; every lane re-fetches
  before staging and yields to an existing claim on its target. **The push is
  the arbiter**: a rejected push = re-fetch, re-adjudicate, yield. **Claim
  expiry:** a claim with no follow-up landing within 48h is EXPIRED — any
  lane may claim past it (a crashed lane must not block the queue head forever).
  **Extend the same claim to OWNER-GATED ADJUDICATIONS**: a lane about to run a
  verdict session on a named gate posts the claim row first — a gate ruling is
  as yield-worthy as a batch (the 2026-10-11 dual gate-ruling: two lanes ran
  owner-delegated verdicts on reflex 085's T0 in the same hour and got OPPOSITE
  outcomes; landed `89769e0` won, the other lane reset unpushed). **Extend it
  to WALK EXECUTIONS too**: the due arxiv-walk close-out is claimed the same
  way before its lane fetches begin — a walk is a multi-hour grading session,
  exactly the claim-worthy waste class (the 2026-10-13 dual walk: two sessions
  executed arxiv walk #14 in the same hour; the pushed close-out `9027f028`
  kept Research 258, the later lane yielded unpushed). **The same lease now
  covers `.issues/`/`.plans/` items (katgpt-rs Issue 943):** before
  implementing any filed issue/plan, `git fetch origin && py
  ../katgpt-rs/scripts/claim_lease_check.py --item <NNN>` — a foreign live
  `Claimed:` line is the yield signal; to claim, append
  `**Claimed:** <session>, <epoch>, expires <ISO-UTC>` under the item's
  Status line and push BEFORE implementing (48 h expiry; advisory — the
  push stays the arbiter). **Closing a lease is a RESHAPE, not an append**
  (measured 2026-10-20, three BLIND repairs): when the claimed work lands
  or the lease is released, rewrite the line to a NON-claim form
  (`**Lease closed:**` / `**Lease released:**` + the provenance in prose)
  — never leave prose after the expiry token on a `Claimed:`-shaped line,
  never put an ISO date in the epoch slot (digits only): an unparseable
  claim-shaped line makes `claim_lease_check` exit 1 (BLIND) — the
  read-time check reads no verdict for every lane in that repo.
  Repaired this date: riir-refine `.plans/225` (`f58f194b7`),
  riir-ai `.issues/971` + `.plans/638` (`6b379b3e6`); riir-ai `.issues/1122` reshaped 2026-10-25
  (`dcf0262af` — trailing prose after the expiry token; claim preserved).
- **Plan selection reads ORIGIN, not the working tree** (2026-08-27, the
  Plan-346 stale-copy trap): before ranking/dispatching any `.plans/` work,
  use `riir-refine/scripts/plan_read_origin.sh <repo> (--list | <file>)` —
  fetches origin, resolves the default branch from the remote (`develop`
  everywhere since the 2026-09-04 riir-train flip),
  reads `git show origin/<default>:...`, logs the commit hash (cite it when
  acting on a plan). The shared checkout can ride a SIBLING branch for days
  (riir-train rode `sync/p348-*` while its mainline was `main`) whose plans
  are stale vs origin — a working-tree read once dispatched a plan already
  closed there. Guard: `--self-test` (dirty non-default-branch checkout must
  still yield origin contents).
- **Sibling safety**: never `git stash` sibling WIP; commit only your own
  hunks; isolate builds under `/tmp`; before blaming CI on riir-refine,
  check whether `src/domains/*/bench.rs` WIP is the sibling's.
- **Stale `rebase-merge` blocks rebases (2026-08-15, Batch 42)**: a
  previous session's leftover `.git/rebase-merge/autostash` (the marker
  file only, stash already applied back) makes `git status` say "currently
  rebasing" + every rebase refuse with a misleading message. Verify the
  autostash SHA's diff is byte-identical to the current WIP
  (`git diff <sha>^ <sha> -- <files>` vs `git diff HEAD -- <files>`),
  then `rm -rf .git/rebase-merge`.
- **When remote diverged AND sibling WIP overlaps the remote commits**:
  do NOT `git rebase --autostash` (the pop can conflict on their files).
  Push via a temp worktree instead: `git worktree add /tmp/<wt>
  origin/develop --detach` → `git -C /tmp/<wt> cherry-pick <my-sha>` →
  `git -C /tmp/<wt> push origin HEAD:develop` → `git worktree remove` →
  `git fetch` + `git update-ref refs/heads/develop origin/develop` (moves
  the ref WITHOUT touching the tree) → `git reset --mixed HEAD` (clears
  phantom staged changes) + `git checkout HEAD -- <committed-but-missing
  files>` (restores files the sibling committed from another clone —
  disjoint from the WIP set). Disk content of WIP files: untouched.
- **GPU exclusivity before benches/gates**: `ps aux | grep -E
  "cargo|riir|katgpt" | grep -v grep` — no second compute consumer
  (Unity Editor + Zed are exempt). Bench 649: 12.3× correctness swing
  under contention.
- **Long-running measurement on this box → grep/doc/web units only** (the
  Plan 337 lesson, 2026-08-15): while a measurement saturates the machine
  (e.g. a 32K precompute at ~13 cores + 13 GB RSS with the measurement
  previously pushed into swap), run ONLY zero-contention queue units —
  items 4 (hygiene), 6 (boundary-guard), 7 (substrate-first), 9-13 (web /
  defer-marking / research / doc-sync). Defer build-dependent units (3, 5,
  8 — sweeps, mining GOAT, benches) until it completes; a cargo build both
  steals CPU from the measurement AND risks re-triggering the swap tax.
  The GPU-exclusivity check above covers benches; this extends it to
  CPU/memory-heavy measurements.
- Don't fix unrelated bugs — note them in the final summary.
- Low confidence → STOP, emit the 4-part `## Summary` (the auto_prompt
  harness depends on that exact format), let the user re-aim.
- **All queues empty → idle report** (what was checked, all clean). Do
  NOT invent work.

## Skill homes (canonical-in-repo + global symlink — never copies)

| Skill | Canonical home | Global |
|---|---|---|
| `riir-refine` (this) | `riir-refine/.agents/skills/riir-refine/` | symlink ✓ |
| `boundary-guard` | `katgpt-rs/.agents/skills/boundary-guard/` | symlink ✓ |
| `substrate-first` | `katgpt-rs/.agents/skills/substrate-first/` | symlink ✓ |
| `doc-sync` | `katgpt-rs/.agents/skills/doc-sync/` | symlink ✓ |
| `build-hygiene` | `katgpt-rs/.agents/skills/build-hygiene/` | symlink ✓ |
| `rust-optimize` | `katgpt-rs/.agents/skills/rust-optimize/` | no symlink (manual `/optimize` invocation only) |
| `feature-gate-audit`, `goat-audit`, `proposal`, `research` | `katgpt-rs/.agents/skills/` (tracked) | no symlink yet |

Copies drift — the substrate-first global duplicate (identical today,
diff-verified 2026-08-15) was removed for exactly this reason. When
creating a new skill: canonical home = the repo that owns the workflow;
global = symlink only.

## Mining queue snapshot (resume state — lives OUTSIDE this file)

The resume state (next batch targets, sibling-blocked files, corpus count,
standing generator lessons) lives at **`.distill/001_mining_queue_snapshot.md`** in this
repo — read it at session start, update it in the same commit as each batch.

**Never append batch history to this SKILL.md.** It broke Zed once already (104KB,
2026-08-18). Batch detail goes to `.research/NNN_*.md`; the rolling queue goes to the
`.distill/` file. This file stays instructions-only, under ~10KB.

## Manual invocation

In Zed: `/riir-refine` in the agent panel, or ask "run the riir-refine
skill". From a terminal, the underlying rulewright is `cargo-refine`
(see Protocol B).

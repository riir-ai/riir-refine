# Issue 019 — Auto-continue idle loop + the `riir-clippy` skill

**Filed:** 2026-08-15, after Batch 31 (`b60b15b`, corpus 209 → 217).
**Request:** the user wants idle sessions to auto-continue the workspace
work instead of only picking `doc-sync`. They proposed a 10-item idle
loop and asked (a) what it misses, (b) to create the `riir-clippy` skill
(project-local + global symlink) so it can be picked up when there is
nothing to do and also run manually.

## Mechanism verdict (honest)

- Zed's agent has **no background scheduler** — it runs when a prompt
  arrives (the user or their `auto_prompt` harness). Skill selection is
  **description-driven**: `doc-sync` gets picked because its description
  matches "after landing work"; nothing matched "continue batches"
  because no such skill existed.
- Fix = one new skill with trigger-rich description. `doc-sync` already
  exists globally — no `create-skill` needed for it; the new skill
  *chains* it at the right step instead.
- Yes, `riir-clippy` can already run + fix: `cargo-heal --suggest` /
  `--fix [--write]` (Plan 005 T6.1/T6.4, Bench 019). The skill drives
  the existing CLI; the new discipline it adds is the **proposer
  adaptation loop** (fix the proposer when a suggestion is wrong, add a
  fixture, re-run — the healer self-improves each iteration).

## The user's 10-item loop

1. find issue to fix
2. find plan to finish
3. fix clippy all repos
4. rm done issue after doc at `.docs`
5. batch-explore benchmark → new rust/kernel perf rules + dry-run the
   fix on our repos (the `riir-clippy` skill itself)
6. run benchmark, find regression, fix
7. search online to unblock
8. mark `- [-]` in plan/issue if deferred or blocked
9. search online for more rust/kernel perf
10. search online for related papers (research)

**Amendment (same day):** the user added `boundary-guard` + `substrate-first`
routine checks to the queue BEFORE item 6 (benchmarks), and asked for the
skill homes to be canonicalized (repo copy + global symlink, never
_duplicates_ — the substrate-first global copy was drift risk). Final queue
shape lives in the SKILL.md (13 items: the 10 above + the 2 checks inserted
at 6/7 + doc-sync as the closing pass).

## Gap analysis — what it missed (folded into the skill)

| # | Gap | Why it matters |
|---|-----|----------------|
| G1 | **Commit + push per completed unit** (`feat:`/`fix:`/`docs:`, stay on `develop`; `main` for riir-train) | the loop's units are worthless if they pile up uncommitted; sibling-WIP conflicts grow |
| G2 | **Validation gates after every heal/fix** — `cargo clippy --all-targets` + `cargo test` + repo CI scripts (`ci_feature_guard.sh`, wasm32 guard); cfg-gated files need count assertions, not exit codes | a green exit code can mean "ran nothing" (riir-chain lesson) |
| G3 | **Proposer adaptation loop as a hard rule** — wrong suggestion → fix the proposer at root cause + add fixture regression + re-run the G5 real-file compile gate; never suppress a lint or hand-edit past it | otherwise the healer accumulates suppressed lints instead of adapting |
| G4 | **GOAT for new rules** — rust_perf rules need the Pareto bench (`rust_perf_bench`) + `context_notes` on regime-fragile/Mixed; demote/annotate losers | a batch is NOT done at corpus-add time (Bench 015/016/017 pattern) |
| G5 | **Dedup grep before adding rules** | Batch 30 dropped 13 candidates as duplicates/adjacent |
| G6 | **Sibling-agent safety** — no `git stash` of sibling WIP; `CARGO_TARGET_DIR=/tmp/...` isolation; commit only own hunks; re-check sibling `bench.rs` WIP before blaming CI | the 2 CI-guard failures after Batch 31 were the sibling's WIP, not ours |
| G7 | **GPU exclusivity check before benches/gates** (`ps aux \| grep -E "cargo\|riir\|katgpt"`) | Bench 649: 12.3× correctness swing under contention (Unity/Zed exempt) |
| G8 | **Numbering discipline** — read `.highwater`, +1, AND re-scan the folder at write time | Bench 675/677 same-night dual-allocation collisions |
| G9 | **Defer requires reason + unblock condition** (+ follow-up issue when the blocker is external) | `- [-]` without a reason is silent rot |
| G10 | **Research items go through the `research` skill protocol** (pre-flight, prior-art gate, PASS-Redirects) | ad-hoc "search online" is the #1 false-PASS mode |
| G11 | **Terminal condition** — all queues empty → idle report, do NOT invent work; low confidence → stop + 4-part `## Summary` | the `auto_prompt` harness depends on the Summary format |
| G12 | **Session context outranks the queue** — user-reported regressions/bugs in the current conversation always go first | the queue is for idle time only |
| G13 | **One unit per cycle, re-scan after each** | priorities shift between units; a stale plan rots |
| G14 | **Mining-queue snapshot lives in the skill** (Batch 32 targets, blocked files, corpus count) — updated per batch | this is the state-carrying mechanism for cross-session resume |
| G15 | **Skill homes canonical** — repo copy is canonical, global is a symlink; duplicated copies drift (substrate-first was duplicated identical; boundary-guard had no repo home) | unversioned global-only skills silently rot + can collide |

## Tasks

- [x] T1 Create the skill at `.agents/skills/riir-clippy/SKILL.md` — the 10-item idle queue + G1-G14 discipline + the batch/heal/bench workflows
- [x] T2 Symlink global: `~/.agents/skills/riir-clippy` → this repo's skill dir (manual `/riir-clippy` invocation + cross-project discovery)
- [x] T3 Reference the skill from `AGENTS.md` so repo-local sessions see it
- [x] T3b Canonicalize skill homes: `boundary-guard` moved global → `katgpt-rs/.agents/skills/` (git-tracked); `substrate-first` global duplicate removed → symlink to the katgpt-rs canonical (copies diff-verified identical first); `riir-clippy` → repo-local + global symlink. `doc-sync` + `rust-optimize` remain global-only real dirs — no repo home yet (pending user call, see G15)
- [x] T4 Smoke run: activate the skill, execute queue item 3 (clippy sweep) on one small repo via `cargo-heal --suggest` → `--fix` → validate → adapt proposer if any suggestion is wrong (proves G3 end-to-end)
  - **DONE 2026-08-15** (riir-viewbridge → riir-game-sdk target). Full loop proven:
    `--suggest` (76 spans, recall-fuzzy by design) → `--fix` preview
    (77 edits across 10 files) → `--fix --write` (2 edits to
    `bevy_shared.rs`) → **validation caught E0432** (`rustc_hash`
    unresolved — game-sdk has no `rustc-hash` dep) → **proposer adapted
    at root cause** (Gate 0: fxhashmap self-gates on an existing
    `rustc_hash` import; splice cannot add a Cargo.toml dep) → fixtures
    rewritten + regression added (`no_rustc_hash_import_unchanged`,
    `aliased_rustc_hash_import_enables` — the alias form caught a real
    gate blind spot) → suite 196 pass / clippy 0 → **re-proof: 77
    proposed edits → 2** (75 were dep-breaking fxhashmap rewrites).
    The 2 surviving `vec-with-capacity` edits landed in game-sdk
    (`0b96f67`, example self-checks 8/8 PASS). Proposer fix:
    riir-clippy `4c4dd8c`.
  - **Bonus finding (validation must be feature-aware):** the E0432 was
    INVISIBLE to default `cargo check` — `bevy_shared.rs` is cfg-gated
    behind `backend_bevy`. Recorded in SKILL.md Protocol B.
  - Sibling-WIP surgical commit used for `proposers/mod.rs` (fixture in
    their WIP file): saved their copy → committed base + my 3 edits →
    re-applied their WIP with my fixture fix merged. Their diff vs new
    HEAD = exactly their 2 hunks.
- [x] T5 Batch 32 via the skill: mine `riir-ai/crates/riir-gpu/src/edge_lora/` + `kernels/` wrappers → Research 034, corpus 217 → ~227, full GOAT + docs + push. **32a DONE (2026-08-15, commit 154025c):** core cluster (edge_module + types + training + adaptive_width), 11 rules, corpus 217→228, Research 034, GOAT G1/G2/G3 PASS, bug yield filed as riir-ai Issue 683. **32b-1 DONE (2026-08-15, commit e6bbe81):** training half (topology_trainer + chain_commit + sigmoid_gate + mod), 10 rules, corpus 228→238, Research 035, GOAT G1/G2/G3 PASS (3 new batch32b tests), bug yield filed as riir-ai Issue 684. **32c DONE (2026-08-15, landed as "32b-2"):** self-play/arena/consolidation half (sleep_consolidation, arena_eval, {go,bomber,fft}_self_play, cross_game_{edge,real} — no kernels/ subdir existed, the snapshot note was stale), 13 rules, corpus 238→251, Research 036, GOAT G1/G2/G3 PASS (312 kernel_heal + 201 default, 3 new batch32c tests, clippy 0 both states), bug yield filed as riir-ai Issue 685 (quadratic centroid storm + adapter-skeleton triplication). The edge_lora module is now FULLY MINED — Batch 32 COMPLETE across 3 sessions (34 rules total, 217→251).
- [ ] T6 Sweep remaining idle-queue items in order (issues → plans → benches → unblock → defer-marking) at least one full cycle; record friction back into the skill
- [ ] T8 First boundary-guard routine check via the idle queue (item 6, before bench regression): `ci_boundary_guard.sh` + 7-surface greps, file violations, log the run
- [ ] T9 First substrate-first drift audit via the idle queue (item 7): Mode 2 parallel-system scan, classify findings, file issues
- [-] T7 Mine `ternary_deltanet_gpu_forward.rs` — BLOCKED: sibling agent actively working there (Batch 31 note); mine only after their WIP lands

## Resolution note

T1-T3 landed same day (skill + symlink + AGENTS.md section + this issue).
T4-T6 are the skill's first real cycles — expected to be executed by
follow-up sessions via "continue"-style prompts. Close this issue once
T4-T6 are done and the loop has run one full clean cycle; then doc + rm
per the noise-reduction rule.

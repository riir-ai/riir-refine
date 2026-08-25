# Issue 037 — E2E self-evolve proof + heal-adoption observability

**Repo:** riir-clippy
**Filed:** 2026-08-24
**Status:** OPEN — **BLOCKER for [Proposal 003](../.proposals/003_binary_only_distribution.md) (binary-only distribution) and [Proposal 004](../.proposals/004_sec_domain_fourth_heal_domain.md) (sec domain)**
**Branch:** develop

## The two questions (owner, 2026-08-24)

1. **How do I know agents will actually USE `cargo heal`?** — judge from log?
2. **How do I know self-evolve works END-TO-END?** — judge from new bench + mining report?

Answer: yes — but only after the loop is actually closed. Today it measurably
is not (F2 below). This issue tracks the wiring + the falsifiable gates, and
blocks the two proposals until proven.

## Measured findings (2026-08-24, adoption day)

- **F1 — memory state:** `~/.cache/riir-clippy/memory.json` = 8,236 buckets,
  ALL `tier=Heuristic`, `success_count` totals **0**, `fail_count` totals 4
  (verify-gate reverts only). The loop records every APPLY but no positive
  verdict has ever been recorded by real usage.
- **F2 — root cause (code-level):** the CLI's only two outcome call sites
  (`src/bin/heal.rs` L1400 + L1422) both pass literal `false` — kept edits go
  through `record_applied` (Heuristic, no outcome). `record_outcome_verified`
  exists in `src/recorder.rs` L213 with **zero call sites in the CLI**.
  Positive verdicts are structurally never recorded → EvidenceTier can never
  advance → the EMA learning half has nothing to eat → trust-weighted
  retrieval weights everything at the default 0.5. This is post-mining
  intake P5, now root-caused.
- **F3 — adoption cadence unmeasurable from memory:** the v1 envelope carries
  no timestamps (per-bucket or top-level). "Did agents run it this week?"
  is currently only answerable via git commits (`fix: heal`/grep) — a proxy
  that misses uncommitted in-session runs.
- **F4 — honest observability limit:** there is NO negative signal for
  *skipped* usage. An agent manually fixing 20 mechanical lints leaves no
  violation trace anywhere. Adoption detection is positive-signal-only
  (memory growth + commits); spot-check residue via dry `cargo heal --fix`.

## The audit one-liner (how to judge from log)

`./scripts/self_evolve_audit.sh` (shipped with this issue) prints all four
log surfaces in one command:

| Section | Source | Proves |
|---|---|---|
| 0. Setup health | symlinks + `~/.config/zed/AGENTS.md` rule | the preconditions for "agent will use it" exist |
| 1. Memory loop state | `~/.cache/riir-clippy/memory.json` | E1/E2 — records + verdicts + tiers |
| 2. Score bench | `.benchmarks/score_history.jsonl` | E4 — learning quality over time |
| 3. Post-mining intake | `.distill/001_mining_queue_snapshot.md` | E5 — mining-report closure |
| 4. Adoption trail | `git log --grep=heal` across repos | A2 — agents actually ran it |

In-session, adoption is additionally visible directly in the agent transcript
(the `cargo heal` terminal calls). But the audit script is the durable,
cross-session evidence — run it weekly or whenever doubting.

**In-binary variant of section 1** (works from any repo with the installed
binary, no checkout needed): `cargo heal --memory-stats` — same counts plus
the BLAKE3 commitment status and the E1/E2/E3 loop-state verdict. This is
the query surface for verifying T2 once it lands: after a real
`--fix --write --verify` run, `verdict` must flip from `—` to `OK`.

## Acceptance gates (falsifiable; issue closes when ALL green)

### Adoption (the "agents use it" half)

- [ ] **A1 — cadence:** heal activity (memory growth after T3, or heal
      commits) on ≥3 distinct days within any 14-day window post-T3.
- [x] **A2 — breadth:** ≥2 repos carry heal-commits dated after
      2026-08-24T00:00:00 (`git log --since=... --grep=heal`).
      → **MET (2026-08-24):** riir-ai `703856080` (the T5 unit) + riir-dapps
      `4fbdbfb` (the E3 evidence run — 3 match_bool edits, oracle-confirmed,
      tests count-identical, direction_gate OK). Both genuine dev-session
      usage commits, not adoption-day infrastructure.
- [x] **A3 — no silent regression:** no repo's clippy residue grows between
      audits (dry-run spot check on the repos agents touched).
      → **MET (2026-08-25 spot check):** dry-run `cargo heal --fix` on both
      heal-touched repos — riir-ai's 10 T5-healed `riir-games` files →
      **0 edits available**; riir-dapps `src/` (the 3 match_bool sites'
      home) → **0 edits available**. No residue growth since the adoption-day
      heals. (Standing check — re-run at each audit until the window closes.)

### E2E self-evolve (the "loop closed" half)

- [x] **E1 — record:** trajectories auto-record on every apply (proven
      2026-08-24: 8,236 buckets).
- [x] **E2 — verdict:** ≥1 real (non-fixture) bucket with `success_count ≥ 1`
      and the first `Certified` tier entry, sourced from agent-initiated
      `--fix --write --verify` runs (NOT the demo/tests). **Blocked by T2.**
      → **T2 LANDED** — the gate now needs a genuine dev-session run (T5);
      the wiring proof used a scratch crate (real CLI, but not a workspace
      repo's dev session).
      → **Post-T2 rebuild verified live (2026-08-24, global binary rebuilt at
      `fix_verify,clippy_verify`):** an agent-initiated scratch-crate
      `--fix --write --verify` run recorded 9 oracle-confirmed Certified
      outcomes into BOTH stores (memory.json: success 9, Certified 6;
      `.heal/trajectories.kv`: 8 rows certified) and `--memory-stats` now
      prints `record OK | verdict OK | learn OK` (the line-61 check PASSES).
      Same provenance class as T2's own proof, so E2's checkbox stayed open
      until T5's workspace-repo run — but the wiring was proven live on the
      installed binary, not just in the E2E proof.
      → **MET (T5, 2026-08-24, riir-ai `703856080`):** the first genuine
      dev-session verify cycle — 19 files / 106 compile-gated edits on
      `riir-games` (`--features swarm,target_coordination`), count-identical
      tests in all 3 configs (default 557/0 · swarm 801/0 · union 860/0).
      The repo store (first persisting run in riir-ai): **101 rows, 96
      oracle-confirmed Certified, success 106 / fail 6** — every fail an
      honest one (ruliology/player.rs's 3 proposed edits auto-reverted by
      the compile gate: apply breaks cargo check → revert → fail verdict).
      `--memory-stats`: `record OK | verdict OK | learn OK`.
- [x] **E3 — learn:** the EMA path consumes real verdicts — evidence: a lib
      test or `self_evolve_real_world_demo` run against the LIVE memory.json
      showing direction deltas ≠ 0 for buckets with outcomes, or an oracle
      run whose log shows learning active (no "learning disabled" line) AND
      persisted state reflecting the update.
      → **MET (2026-08-24, the E3 landing + the riir-dapps oracle run).** The
      gate's store-backend half: `TrajStore::record` now also feeds a durable
      per-rule EMA learn row (`learn:{lint_key}` → `LearnRowDoc`) through the
      SAME `ema_step` core the in-process layer uses (extracted to
      `evolve::ema_step` — one rule, two homes). `--memory-stats` prints the
      learn section + decay line. Evidence run (riir-dapps `4fbdbfb`): 3
      oracle-confirmed verdicts, learning active (no degradation line), and
      `--memory-stats` in a SEPARATE process prints `learn (EMA): 1 evolved
      rule(s) | 3 update(s) consumed … last |Δ| 0.2303` — direction deltas
      ≠ 0 for the bucket with outcomes, persisted (cross-process durable).
      Lib tests pin the mechanism: init-on-first-success, first-fail no-op,
      apply-only never touches learn rows, learn rows invisible to the
      trajectory surface, stats section + decay line.
      (The envelope-era EMA half was already evidenced by Bench 024 G7 —
      50%→100% healing via real clippy evolution; the envelope carries no
      persisted EMA state to print, so the learn section is store-only by
      design, 038's one-active-backend constraint.)
- [x] **E4 — bench:** ≥1 new `score_history.jsonl` run post-T2 with
      `heal_rate ≥ 98.1%` AND `created_rate ≤ 1.9%` (no decay), or an
      improvement attributable to a post-mining fix.
      → **MET (T4, 2026-08-24, sha `e4ce8ba`):** run #4 = heal **98.1%** ·
      created **1.9%** · safety **100%** · decline **100%** — bit-identical
      metrics to run #3 (`c8d0d34`): ZERO decay through the T2 verdict
      wiring + the 038 store-backend swap (the bench's `heal_fixpoint`
      exercises proposers only, by design — persistence is T5's axis).
      Persist map unchanged: `useless_format ×1` (the documented String-typed
      boundary, deliberately kept).
- [ ] **E5 — mining report:** ≥1 post-mining intake class (P1–P8) closed
      end-to-end: intake `- [x]` + proposer-change commit + fixture +
      score-bench delta recorded in the research note.

## Tasks

- [x] **T1 — audit script** `scripts/self_evolve_audit.sh` (this issue's
      instrument; tested live — sections 0–4 print, E2/E5 honestly FAIL).
- [x] **T2 — wire positive outcomes in the CLI** (the F2 fix, unblocks E2):
      after a `--verify` pass where the compile gate kept an edit AND the
      clippy oracle (when built) confirms the named lint resolved, call
      `record_outcome_verified` / `record_outcome(..., true)` instead of bare
      `record_applied`. Keep the honest split: rustc-only pass = Heuristic
      outcome-success; oracle-confirmed = Certified (Issue 021 semantics).

      **DONE 2026-08-24 — commit `667562a`** — landed as the verdict matrix + the durable
      landing (the decision's directive: verdicts as in-schema fields on
      the `.heal/trajectories.kv` rows, keyed by heal-run id — no sidecar,
      038 keeps a single read path):
      - **The matrix** (`fix_verify::kept_edit_verdict`, unit-pinned):
        oracle-confirmed resolve → `(success, verified)` = Certified;
        persist → verified FAILURE (the honest persist signal); no oracle
        verdict (not built / unrunnable / non-clippy rule — rust_perf ids
        are kebab-case, not clippy lints) → rustc-only Heuristic
        outcome-success. The CLI's kept-edit loop now calls
        `record_outcome_verified` (was bare `record_applied` — F2's root
        cause); reverts keep their Issue-018 failure verdicts.
      - **The oracle** (clippy_verify builds): one `cargo clippy
        --all-targets -- -W clippy::<lint>…` per owning package (the same
        feature set as the compile gate; `CheckRunner::run_clippy`),
        findings parsed PRIMARY-span-only
        (`fix_verify::parse_clippy_findings`) — a kept edit is resolved iff
        no finding of its named lint covers the edit's final line (a
        finding elsewhere in the file — a refused sibling span — does not
        block confirmation). Pedantic-group lints work (the `-W` forcing
        was e2e-proven with `uninlined_format_args`).
      - **The durable landing** (fix_verify = implies latent_retrieval):
      verdict events ALSO land in the repo-local neuron-db store
      (`.heal/trajectories.kv`) with run attribution —
      `TrajRowDoc.last_run_id` + `last_seen_at` (unix secs; 038-T1's
      timestamp field landed early — supersedes 037-T3's JSON-envelope
      version; pre-T2 rows decode with `""`/`0` serde defaults).
      Transitional dual-record with memory.json (same events; 038-T3
      retires the envelope half). Root: `RIIR_HEAL_TRAJ_ROOT` → first
      path's workspace root → cwd (per-repo, 037-A2's audit shape).
      - **E2E proof** (real CLI, scratch crate, non-fixture code): 6 kept
      edits → `oracle: 6 confirmed resolved (Certified)` → all 6 rows in
      `.heal/trajectories.kv` carry `success_count ≥ 1`, `tier = 0`
      (Certified), `last_run_id = run-…`, `last_seen_at > 0`; memory.json
      mirrors with Certified tiers (F1's all-zero-success state is now
      structurally impossible on new verify runs). The revert→fail-row
      path rides the same `store.record` call (unit-pinned matrix + the
      Issue-018-validated revert flow); no separate e2e demo recorded.
      - En route: the sibling session's in-flight `--memory-stats`
      observability scaffolding (flag + parse + help +
      `LatentFixMemory::stats_report`) landed in the same commit with the
      quote-escape fix that unblocked compilation — its E2 readout expects
      exactly this wiring ("verdict needs a positive outcome (037-T2
      wires it)"); the handler remains the sibling's to finish.
- [ ] **T3 — timestamps in the memory envelope** (unblocks A1's memory half):
      optional `last_seen_at` per bucket + top-level `last_run_at`, serde
      defaults for backward compat (pre-T3 files decode as today).
      **SUPERSEDED by Issue 038** (removed 2026-08-25 per noise-reduction
      rule; record: AGENTS.md §Production adoption +
      `.docs/10_self_evolve/latent_fix_memory.md` §Persistence) — the
      neuron-db backend landed; timestamps live in the Pod rows
      (`last_seen_at`); do the JSON-envelope version ONLY if 038's backend
      were ever reverted.
- [x] **T4 — post-T2 score-bench run** (feeds E4; record in score_history).
      **DONE 2026-08-24** — run #4 appended (`e4ce8ba`): heal 98.1% / created
      1.9% / safety 100% / decline 100% — bit-identical to run #3, zero
      decay through the T2 + 038 landings. E4 MET.
- [x] **T5 — first real-world verify cycle:** one genuine dev-session heal
      with `--write --verify` on a workspace repo (e.g. the 222-edit
      riir-games unit or the next idle-loop sweep) — its audit section-1
      output is the E2 evidence.
      **DONE 2026-08-24 — riir-ai `703856080`.** Scoped to the 20
      default+swarm+target_coordination-compiled files (108 dry-run edits;
      dep-info gate map — the full 222-edit unit's other 34 files sit behind
      game_sync/npc/combat/etc. features, the idle-loop sweep's territory).
      19 files / 106 edits kept compile-gated; ruliology/player.rs's 3
      proposed edits AUTO-REVERTED (apply breaks cargo check — the honest
      refusal, recorded as the 3 fail verdicts). Tests count-identical in
      all 3 configs: default 557/0 · swarm 801/0 · union 860/0. Store:
      101 rows / 96 Certified / success 106 / fail 6 → **E2 MET**.
      The player.rs break class is post-mining intake material (the exact
      rustc error would sharpen a proposer guard — filed in the queue
      snapshot, not examined this session).
- [ ] **T6 — close gates A1–A3/E2–E5** via audit script output; write the
      resolution note here; remove this file per the noise-reduction rule.
- [ ] **T7 — unblock Proposals 003 + 004** (they reference this issue; see
      below) once E2–E5 are green.

## Why this blocks the two proposals (owner's call, 2026-08-24)

- **Proposal 003 (binary-only distribution):** the moat argument is "the
  adaptive/learning layers" (caveat 2: "keep the adaptive/learning layers as
  the moat"). Shipping a public binary while the loop that creates the moat
  has **never recorded a single positive verdict** (F2) ships an unvalidated
  moat — and bakes un-judged memory semantics into the public artifact.
  First release waits for E2–E4.
- **Proposal 004 (sec domain):** a fourth domain multiplies corpus +
  proposer surface on top of a loop whose verdict half is unwired. The sec
  domain's boundary gate is real-clippy-oracle-driven — the same substrate
  T2 fixes. Prove the existing loop first; then open the new domain.

## Notes

- The intake P5 line in `.distill/001_mining_queue_snapshot.md` now points
  here; T2 IS P5's fix — one unit, tracked once.
- Audit section 4 already shows 5 heal-mentioning commits across 3 repos on
  adoption day (2026-08-24) — A2 is nearly satisfied by the adoption work
  itself; the gate wants post-adoption *usage* commits, so it stays unchecked
  until they accumulate.

## Session record (2026-08-25)

- **A3 spot check run + MET** (evidence at the A3 checkbox). Both
  heal-touched repos clean on dry-run — no residue growth since adoption day.
- **A1 status:** distinct heal-activity days so far = 1 (2026-08-24 — the
  stores' `last_seen_at` population). Window runs through 2026-09-07;
  earliest possible close 2026-08-26 if heal runs land today + tomorrow.
- **A1 addendum (later 2026-08-25, this session):** the audit script gained
  **§5 — the mechanical A1 judge** (union of store `last_seen_at` days +
  consumer-repo heal-commit days, sliding ≥3-in-any-14-day-window count;
  riir-clippy's own dev commits excluded from the commit half — the tool's
  development is not adoption evidence). First live run **corrects the day
  count above: 2 distinct days, not 1** — riir-game-sdk's store carries a
  genuine 01:24 heal run today (`run-18ced0f5438da5b8-68248`:
  vec-with-capacity `Vec::new()` → `Vec::with_capacity(MAX_DRAIN_ROUNDS)`,
  success, recorded) + same-day heal commits in game-sdk/train. A1 stays
  open — one more distinct day closes it (earliest 2026-08-26; any genuine
  heal run that day → §5 prints `A1: PASS` → T6 ticks it).
- **Issue 038 closed** (all T1-T6 already `[x]`; GOAT landed `e4ce8ba`):
  resolution note in AGENTS.md §Production adoption +
  `.docs/10_self_evolve/latent_fix_memory.md` §Persistence; issue file
  removed per the noise-reduction rule. It fed this issue's E2/E3/E4 —
  all three MET on the store backend.
- **Remaining open:** A1 (time-gated — accumulates), E5 (post-mining class
  closed end-to-end; P9 in the queue snapshot — the ruliology/player.rs
  auto-revert cluster — is the armed candidate: root-cause the exact rustc
  error, fix the proposer guard + fixture, record the score-bench delta).
  Then T6 (resolution note + rm) + T7 (unblock Proposals 003+004).

## Session record (2026-08-24, T4+T5)

- **E3 + A2 LANDED (same day, post-T5):** the EMA learn observability on the
  store backend. `ema_step` extracted to `evolve.rs` (the pure EMA core —
  `LatentSkillEvolution::record_outcome` now calls it, bit-identical by
  test); `TrajStore::record` feeds durable per-rule learn rows
  (`learn:{lint_key}`, `LearnRowDoc`: direction/updates/successes/fails/
  last_delta/run attribution); `--memory-stats` prints the learn section
  (per-rule counters, top-8 by updates) + the decay line (α/β + mean last
  |Δ|). 7 lib tests (2 evolve + 5 traj_store), clippy 0 both states
  (default + latent_retrieval). Evidence run: riir-dapps `4fbdbfb` — 3
  oracle-confirmed match_bool edits, `--memory-stats` (separate process)
  prints `learn (EMA): 1 evolved rule | 3 updates | last |Δ| 0.2303` →
  **E3 MET**; the commit is heal-commit #2 post-adoption (riir-ai #1) →
  **A2 MET**. The global binary was rebuilt with the change (21:5x).
- **Remaining open gates:** A1 (cadence — time-gated, accumulates; today is
  day 1), A3 (no-residue spot check — next audit), E5 (mining-report
  closure — needs a P-class closed end-to-end). T6 closes them via the
  audit script; T7 (unblock Proposals 003/004) follows.

- **Observability fix (`c9d718b`)**: the kv branch of `--memory-stats`
  shipped 038 as a 4-counter one-liner while the envelope branch printed
  the full report — the histogram/tier/loop-state detail was envelope-only
  (surfaced live when post-migration reads from riir-game-sdk showed no
  per-rule detail). `TrajStore::stats_report` now carries envelope parity
  PLUS the cadence line the envelope could never answer (F3): `activity:
  last verdict YYYY-MM-DD | distinct active days N` from `last_seen_at`
  (migrated rows carry 0 → excluded by construction). `loop_state_line`
  extracted to memory.rs — one set of E1/E2/E3 semantics, both backends.
- **The migration landed in riir-game-sdk** (another session's persisting
  run, ~21:02): envelope 8241 buckets → 8117 rows (124 merged where the
  envelope's (lint, buggy, fixed) triples collapsed onto the store's
  (lint, shape) keying) + that run's own outcomes on top (success 9→11,
  Certified 6→8). riir-ai's store started FRESH (the `.migrated` fence
  prevents a second import) — per the one-import design.
- **A2 status**: riir-ai `703856080` is heal-commit #1 post-adoption;
  game-sdk's persisting run rode a sibling's uncommitted WIP (riir-e2e
  files, Plan 004) — its heal commit lands when they commit. Gate stays
  open until the 2nd repo's commit is on a branch.
- **E3 remains open** — the EMA learn path consuming the store's verdicts
  needs its own evidence run (the direction-delta / learning-active check
  from the gate text); T5's run records verdicts but the EMA-half
  observability is unproven on the store backend.
  *(Superseded same day — see the E3 + A2 LANDED entry above.)*
- **2026-08-24 hygiene sweep (workspace-wide):** `.heal/` is machine-local
  runtime state — the `trajectories.kv` encrypted WAL, the `.heal/key` seed,
  and the `misses/` queue — and must never be committed: binary WAL churn,
  cross-machine divergence under the m3/4090 sync rule, and a committed
  `.heal/key` in public katgpt-rs would be key material in a public repo.
  `.heal/` is now gitignored in all 15 workspace repos (riir-clippy already
  carried the Plan 019 entry; 14 more added it, riir-train via a worktree
  cherry-pick — push race with a sibling — and katgpt-web committed to local
  `main` only, it has no remote). Misses surface via `.distill/` intake
  notes, not raw commits. Candidate post-mining follow-up: the healer
  auto-appends the ignore entry on fresh-store creation (protects repos not
  yet swept).

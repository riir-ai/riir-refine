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

## Acceptance gates (falsifiable; issue closes when ALL green)

### Adoption (the "agents use it" half)

- [ ] **A1 — cadence:** heal activity (memory growth after T3, or heal
      commits) on ≥3 distinct days within any 14-day window post-T3.
- [ ] **A2 — breadth:** ≥2 repos carry heal-commits dated after
      2026-08-24T00:00:00 (`git log --since=... --grep=heal`).
- [ ] **A3 — no silent regression:** no repo's clippy residue grows between
      audits (dry-run spot check on the repos agents touched).

### E2E self-evolve (the "loop closed" half)

- [x] **E1 — record:** trajectories auto-record on every apply (proven
      2026-08-24: 8,236 buckets).
- [ ] **E2 — verdict:** ≥1 real (non-fixture) bucket with `success_count ≥ 1`
      and the first `Certified` tier entry, sourced from agent-initiated
      `--fix --write --verify` runs (NOT the demo/tests). **Blocked by T2.**
- [ ] **E3 — learn:** the EMA path consumes real verdicts — evidence: a lib
      test or `self_evolve_real_world_demo` run against the LIVE memory.json
      showing direction deltas ≠ 0 for buckets with outcomes, or an oracle
      run whose log shows learning active (no "learning disabled" line) AND
      persisted state reflecting the update.
- [ ] **E4 — bench:** ≥1 new `score_history.jsonl` run post-T2 with
      `heal_rate ≥ 98.1%` AND `created_rate ≤ 1.9%` (no decay), or an
      improvement attributable to a post-mining fix.
- [ ] **E5 — mining report:** ≥1 post-mining intake class (P1–P8) closed
      end-to-end: intake `- [x]` + proposer-change commit + fixture +
      score-bench delta recorded in the research note.

## Tasks

- [x] **T1 — audit script** `scripts/self_evolve_audit.sh` (this issue's
      instrument; tested live — sections 0–4 print, E2/E5 honestly FAIL).
- [ ] **T2 — wire positive outcomes in the CLI** (the F2 fix, unblocks E2):
      after a `--verify` pass where the compile gate kept an edit AND the
      clippy oracle (when built) confirms the named lint resolved, call
      `record_outcome_verified` / `record_outcome(..., true)` instead of bare
      `record_applied`. Keep the honest split: rustc-only pass = Heuristic
      outcome-success; oracle-confirmed = Certified (Issue 021 semantics).
- [ ] **T3 — timestamps in the memory envelope** (unblocks A1's memory half):
      optional `last_seen_at` per bucket + top-level `last_run_at`, serde
      defaults for backward compat (pre-T3 files decode as today).
      **SUPERSEDED by [Issue 038](038_self_evolve_memory_neuron_db_backend.md)**
      (neuron-db backend — timestamps land in the Pod rows there); do the
      JSON-envelope version ONLY if 038 slips past the A1 measurement window.
- [ ] **T4 — post-T2 score-bench run** (feeds E4; record in score_history).
- [ ] **T5 — first real-world verify cycle:** one genuine dev-session heal
      with `--write --verify` on a workspace repo (e.g. the 222-edit
      riir-games unit or the next idle-loop sweep) — its audit section-1
      output is the E2 evidence.
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

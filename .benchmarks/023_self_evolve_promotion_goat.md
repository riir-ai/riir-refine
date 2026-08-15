# Bench 023 — Issue 018: `self_evolve` promotion with graceful oracle degradation

**Date:** 2026-08-16.
**Issue:** [018](../.issues/022_corpus_real_clippy_drift.md) (resolved + removed; the OPEN follow-up found during this GOAT is Issue 022 — corpus drift).
**Scope:** promote the self-evolution layer (memory + EMA learning) from
opt-in to default-on, splitting the oracle-free memory half from the
oracle-gated learning half. Promotion + degradation plumbing only — no new
learning mechanisms (per the issue's non-goals).

## What shipped

1. **Unconditional modules** — `evolve`, `memory`, `self_evolve`,
   `recorder` (new) compile in every build (incl.
   `--no-default-features`). The `self_evolve` feature stays as a
   default-on back-compat alias (benches'/examples' `required-features`).
2. **`EvolveRecorder`** (`src/recorder.rs`) — the oracle split:
   - `record_applied` / `record_reviewed`: memory only, no verdict —
     runs in EVERY build.
   - `record_outcome`: memory stats always; EMA learning only when the
     oracle is present (`cfg!(feature = "clippy_verify")` or explicit
     runtime enable); otherwise a no-op with **exactly ONE** degradation
     log line per recorder lifetime.
   - `disable_learning(reason)`: runtime oracle failure path (memory
     keeps working).
   - `RIIR_HEAL_SELF_EVOLVE=0|off|false|no`: pure-loop opt-out — fully
     inert (no recording, no persistence, no log).
3. **Persistence** (`LatentFixMemory::to_json/load_json/save/load`) — a
   CLI process is short-lived; an in-RAM memory would stay permanently
   starved (the exact problem the issue fixes). v1 JSON envelope,
   merge-on-load semantics (identical triples keep live stats; new
   triples append with persisted stats), bit-identical `direction`
   round-trip (ryu shortest form), `InvalidData` on corrupt/unknown-version
   files (never aborts a heal run), `NotFound` = fresh.
4. **`cargo heal --fix` wiring** — non-verify mode records WRITTEN edits
   (`record_reviewed`); `--verify` (fix_verify) mode records kept edits as
   memory-only (a rustc pass proves non-breakage, not lint resolution) and
   reverted edits as definitive `success=false` verdicts (sound to learn
   from when an oracle build enabled learning); review-only runs record
   nothing. Memory path: `RIIR_HEAL_MEMORY` override, else
   `$XDG_CACHE_HOME`/`~/.cache`/`%LOCALAPPDATA%`/temp →
   `riir-clippy/memory.json`. Persisted once per run when anything was
   recorded.
5. Drive-by: fixed the pre-existing cross-feature `unused_mut` warning in
   `heal.rs` (`lint_set` is only mutated under `fix_verify` — cfg-split
   declaration).

## GOAT

| Gate | Result |
|---|---|
| G1 (contracts) | ✅ Default build: `oracle_absent_learning_noops_memory_works_one_line` (learning no-op, memory works, one-line flag sticky) + `memory_accumulates_after_fix_run` (the production proposer→emitter→`record_reviewed` path). clippy_verify build: `oracle_present_learning_records_and_no_degradation_line`. Persistence: lossless round-trip, cross-run accumulation (3 lifetimes), corrupt→`InvalidData` + memory untouched, merge keeps live stats. |
| G2 (e2e, real bin) | ✅ `cargo heal --fix --write` on a 2-file fixture: 1→3 trajectories across runs, exactly one degradation line per run, memory file correct (`buggy_code`/`fixed_code`/`direction`). Opt-out run: 0 self-evolve lines, memory byte-untouched. |
| G3 (no-regression) | ✅ 405 default / 419 clippy_verify / **652 combined** (CI guard, incl. the Issue 017 retrieval regression gate) lib tests green; clippy **0 warnings** in 5 states (default all-targets, no-default-features, latent_retrieval, clippy_verify, fix_verify bin); `ci_feature_guard.sh` ALL LAYERS PASS; both self_evolve examples compile. |
| G4 (dep surface) | ✅ `cargo tree -e normal` diff old-default vs new-default: **EMPTY** (self_evolve adds no deps; persistence uses the existing serde/serde_json). |
| G7 (oracle-present unchanged) | ✅ by regression test + untouched code: `HealPipeline::report_outcome` + `tune_against_fixture` are byte-identical; all 14 pre-existing `self_evolve::tests` pass unchanged; recorder-level `oracle_present` test reproduces the verdict→memory+EMA mechanism. The full demo re-run is blocked PRE-EXISTING (see below). |

## Honest findings

- **The G7 demo re-run aborts at G6 (35/40) — PRE-EXISTING, proven by
  baseline worktree run at `a532b7c`** (identical 5 failures). The Issue
  016 corpus expansion's GOAT verified corpus buggy shapes against our
  parse-only proposers, not real clippy; 5 entries' `buggy_code` doesn't
  trigger under `-W clippy::<name>` (`comparison_to_empty` — the Bench 022
  known drift — `iter_next_loop`, `iter_nth_zero`, `single_match_else`,
  `while_let_on_iterator`). Filed as **Issue 022** (fix sketch + acceptance
  there; fixing it restores the demo as the oracle-present evidence path).
- The verifier-based oracle canNOT soundly verify span-level bin edits
  standalone (`ClippyVerifier` wraps code as a whole temp-crate lib — a
  span replacement referencing outer-scope names won't compile, and a
  non-compiling crate reports "lint absent" = false success). This is why
  the bin's learning hooks ride the fix_verify compile gate (sound verdicts
  only) instead of the clippy verifier.
- `--all-features` CI layer passes (the previously-broken sibling state in
  `rust_perf/bench.rs` was fixed when Issue 015's work landed).

## Floors

No retrieval-precision impact (corpus untouched) — the Issue 017 regression
gate passed unchanged inside the CI guard's combined suite.

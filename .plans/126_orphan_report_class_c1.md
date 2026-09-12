# Plan 126 — Orphan-report: intake class C1, the config knob orphaned by optimization

**Status:** COMPLETE — 2026-09-12 (all tasks `[x]`; G1 37/37 fixtures, G2 positive control FIRES + 0 FP in every sampled adjudication, G3 1241 lib green + clippy clean on touched files, G4 zero new deps)

**Intake:** `.distill/001_mining_queue_snapshot.md` §"Post-mining intake" class C,
sub-shape **C1** (the last open class). Measured once: riir-games-civ
`BehaviorConfig.alarm_radius` — a 20-arm `match` duplicated all 22 per-species
values after a const-table optimization; fixed by wiring to the SpeciesRegistry
(riir-ai `88c51824b`).

**The intake's own detection mandate:** "Detection needs HISTORY, not just
syntax — the field had a reader that vanished — so this is a review-time +
sweep-time heuristic, not a per-compile lint: on any commit touching a
`Default`/constructor init of a `pub` config field, check the field still has a
non-test reader at the new HEAD (the class-A lint's condition 2 reused as a
post-hoc check)."

## The instrument (report-first law — gates nothing)

`cargo heal --orphan-report [PATHS]` — a standalone report mode, sibling lane
to `--config-audit` (ungated, zero new deps, pure syn + `git` subprocess).

Two legs:

1. **Current leg (syn, reused + extended):** the class-A condition-2 set at
   field granularity — Default-impl **or serde-backed-literal** candidate
   fields where `external(f) = total − tests − own_impl(T, f) == 0` (the C2
   formula, per-field). Extracted from `config_audit` as a shared `scan_core`
   + `pub fn zero_external_read_fields` (DRY — no second visitor).
2. **History leg (git, name-global):** count read-shaped occurrences (POSIX
   ERE `\.name([^[:alnum:]_]|$)` — literal inits and bare defs excluded by
   the dot; destructuring under-counted, documented) at HEAD and at probe
   commits (default depths 10/50/200/1000, clamped). **Orphan verdict =
   delta**: any probe count > HEAD count ⇒ read-shaped occurrences vanished.
   Survives own-impl reads (validate then AND now ⇒ delta 0 ⇒ silent) and
   catches the consumer-removed shape. Removal attribution via
   `git log --pickaxe-regex -S` (newest 8); the newest hit's subject matching
   the optimization vocabulary (perf/optimi/refact/fast/inline/table/cache/
   simd, case-insensitive) tags the row `LIKELY-BY-OPTIMIZATION`.

Verdict classes: `ORPHANED` rows (with evidence), `[stillborn]` one-liners
(zero read-shaped occurrences at HEAD and every probe — never read within the
window; fills the single-dead-field gap C2's whole-struct rule misses),
silent (read-shaped occurrences exist now, no vanished evidence — counted in
the header).

Exit contract (the frontier-report law, not config-audit's): exit 0 for every
real scan; **refuse exit 1** on an empty population or a blind history leg —
never a confident green zero.

## Tasks

- [x] T1 `config_audit`: shared `scan_core` (parse → candidates → tally) +
      `pub fn zero_external_read_fields` + `ZeroReadField`/`ZeroReadScan`;
      existing 23 fixtures unregressed.
- [x] T2 `src/orphan_report.rs`: types + `HistoryProbe` trait (prod `GitCli`
      — rev-resolve, grep-count, pickaxe-log; test `FakeHistory`),
      delta classification (orphaned/stillborn/silent), render,
      `report_paths` / `classify_with_probe`.
- [x] T3 `src/lib.rs` module + `src/bin/heal.rs` wiring (`--orphan-report`:
      Args field, init, parse arm, paths-guard + bare-posture exclusions,
      dispatch + blind exit-1, `standalone_modes_are_never_bare` row, usage
      text).
- [x] T4 Fixtures (14 total incl. the 4 serde-literal arms): orphaned fires,
      stillborn, alive silent, own-impl delta-0 silent, reader+validate delta
      fires, perf tag + case-insensitivity, name-masking FN direction,
      serde-literal candidate (the canonical shape), serde-literal orphaned,
      Serialize-only excluded, `Self`-literal not duplicated — + one REAL-git
      e2e (tempdir repo, init→wire→optimize→orphan). 37/37 green.
- [x] T5 Validate (G2) — numbers below.
- [x] T6 Docs: AGENTS.md CLI bullet; intake row class C1 → DONE (with the
      narrative correction); HISTORY row; `.plans/.highwater` 125 → 126.
- [x] T7 Gates + commit (heal.rs via the my-blob technique — the sibling
      093-T2 WIP shares the file; disjoint hunks) + push.

## The two findings that reshaped the instrument (the honest record)

1. **The canonical case forced the serde-literal candidate extension.**
   `BehaviorConfig` has NO `impl Default` at the pre-fix tree — the designer
   surface is `#[serde(default = …)]` + handwritten literals
   (`hardcoded_config()` tables). The A/B/C2 candidate net (Default-impl
   only) cannot see the intake's OWN canonical case. The extension:
   constructor-init candidates from serde-backed structs (`#[serde(…)]`
   anywhere on the item, or a derive naming Deserialize; Serialize-only
   excluded — the derive reads those fields invisibly). Classes A/B/C2 keep
   the narrower net (their plans' measured rates stand).
2. **The intake's "reader that vanished" narrative is syntactically false for
   the canonical case.** Repo-wide pickaxe over ALL riir-ai history:
   `.alarm_radius` member-read occurrences NEVER changed before the fix —
   the const-table fn was born 2026-06-06 (Plan 225 T2), the config field
   2026-06-08 (Plan 245) — two days LATER, with the values already copied
   into the table at birth. The knob was born unwired, not orphaned. The
   instrument's honest verdict for it is `[stillborn]` — which it fires, as
   the first row. The ORPHANED machinery stays for real vanished readers
   (the current-tree `server_hash` row below is one).

## Validation gates (all measured 2026-09-12, binary built at the release
feature set)

G1: 37/37 fixture tests green (23 prior + 14 new; the e2e over a real
    throwaway git repository exercises init→wire→optimize→orphan).

G2 (hand-adjudicated, every sampled row):
- **Self-scan** (riir-clippy `src/`, 234 files): 0 orphaned, 1 stillborn —
  `PavDataConfig.ground_truth` VERIFIED TRUE (every grep hit is the decl, a
  literal init, or the free *function* of the same name). 1 TP / 0 FP.
- **Positive control** (pre-fix worktree of riir-ai `88c51824b^`, civ):
  `BehaviorConfig.alarm_radius` FIRES — first stillborn row. Sampled 9/9
  rows TRUE repo-wide (`.alarm_radius/.boids/.initial_goal/.hunted_by/
  .use_predator_weights/…` all 0 member-reads at that tree — the designer
  surface was broadly unwired, matching the fix narrative "editing the
  config did nothing").
- **Current riir-ai** (2613 files, 2164 candidates, 222 zero-external-read):
  exactly **1 ORPHANED row** — `ReplayVerificationResponse.server_hash`
  (6→1 read-shaped occurrences across `23389c801` "refactor(570): dissolve
  riir-ffi"; the surviving occurrence is a test assert) — VERIFIED TRUE
  (no reader anywhere at HEAD; the consumer died with the sidecar, not
  moved). `alarm_radius` correctly ABSENT (post-fix it is read). 83
  stillborn: 11 sampled — 8 TRUE unwired knobs (incl. `boids`,
  `initial_goal`, `CombatConfig.use_predator_weights` — the post-fix
  remainder of the same civ designer surface), 3 benign-true
  (`AssetViewState._pad*` repr(C) Pod padding — intentional no-read layout
  fields, a documented benign subclass). **0 FP in every sample.**

G3: 1241 lib tests green; clippy clean on every touched file (healed first —
    3 edits by `cargo heal --fix --write --verify`, then manual for the
    corpus-miss class); the repo-wide `-D warnings` sweep's remaining errors
    are PRE-EXISTING in `frontier_pairs.rs`/`frontier_report.rs`/
    `rustc_errors/evolve.rs` (committed develop state, the 093/frontier
    lane's territory — not this plan's).

G4: zero new deps; `Cargo.toml` untouched (the sibling WIP file).

## Post-landing residue (the guard-landing protocol, applied to findings)

The current-tree stillborn rows that name REAL unwired knobs (civ
`boids`/`initial_goal`/`use_predator_weights`, engine
`BeliefDeltaStepLocalizer.default_tau_reliable`,
`CommittedBlendConfig.summary_kind`, rest `server_hash`) are findings for
their OWNING lanes (riir-ai Issue-888 lineage) — recorded here + in the
intake; deletion/wiring is the owning lane's call, not the instrument's.

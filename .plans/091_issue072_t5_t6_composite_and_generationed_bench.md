# Plan 091 — Issue 072 T5+T6: composite exactness schedule + generationed bench ratchet (+ the `.heal/misses/` frontier-order wiring)

**Status:** PLANNED — ready to execute (unblocked; CPU-only; no owner gate)

**Source:** riir-clippy `.issues/072_frontier_evidence_challenger.md` T5/T6 (the PoC's
designated post-exit plan — PoC exit criterion MET, Plan 090); Research 450 (riir-train)
§4.1 M4/M5. The verdict line in the issue names this unit: "The next unit is the PLAN for
T5/T6 (+ wiring `frontier_order` into the `.heal/misses/` authoring queue, where the real
G8 becomes measurable)."

## Grounding (read at plan time, 2026-09-06)

- `GeneratedCase { name, expect_code, kind, seed, broken }` — `seed` is the compiling
  ground truth, `broken` the planted defect (`src/domains/rustc_errors/corpus_gen.rs:53`).
  A composite's gold = composition of atomic seeds, so M4's "exact, no oracle" is
  structural, not asserted.
- Misses: `evolve.rs::write_miss` → `.heal/misses/{code}-{ts}.md` (FIFO by filename
  timestamp; no ordering surface exists; `misses_dir()` is the only reader-facing API).
  The real G8 (frontier vs FIFO as Certified-conversion per authored rule) becomes
  measurable once the queue is frontier-ordered and conversions are attributable.
- Score bench: `score_bench/report.rs::{HistoryRecord, append_history, load_history}` +
  `.benchmarks/score_history.jsonl` (45 runs at plan time). The saturation detector reads
  exactly this; nothing new to persist.
- `dispersion.rs` is feature-free pure math (Plan 090 discipline note); T7 landed the
  `dispersion:` telemetry line in `stats_report` — the detector half's standing input.

## Tasks

- [ ] T0 — Racing pre-check: `git status` clean in riir-clippy; no sibling hunks on
  `score_bench/`, `evolve.rs`, `corpus_gen.rs`, `traj_store.rs`; claim recorded here.
- [ ] T1 — `.heal/misses/` frontier-order wiring (the real-G8 enabler):
  - [ ] T1.1 `evolve.rs`: `read_misses(dir) -> Vec<MissEntry>` (parse `{code}-{ts}.md`
    back into code + captured-ts + file/line/message; tolerate hand-authored entries
    missing fields — the queue is an aid, never a gate) +
    `frontier_order_misses(entries, outcomes) -> Vec<(MissEntry, DispersionVerdict)>`:
    per-code pools from the TrajStore trajectories keyed `"{code}:*"` (strategy
    wildcard), outcome classes mapped Kept=success-verified / Reverted=failure /
    Missed=no-record, ordered by `dispersion::frontier_order` semantics — informative
    first, saturated/regenerate flagged. Determinism pinned (BTreeMap iteration, the
    Plan-090 BT pattern); the cold authoring path is exempt from G4.
  - [ ] T1.2 CLI: `cargo heal --fix-compile --misses [--limit N]` prints the ordered
    authoring queue (code, verdict band, dispersion, path). Without the flag the dump
    path is byte-identical to today (G3 on `dump_misses` behavior).
  - [ ] T1.3 G8 instrument: conversion ledger — when an authored template lands for a
    code (playbook count for `code` increases) and that code's subsequent driver
    outcomes include a Certified admission, record the conversion; `--misses --report`
    prints conversion-per-authored-rule against the PRE-WIRING FIFO history as the
    baseline. Honest note: the FIFO arm is historical, not re-runnable post-wiring —
    the comparison is frontier-vs-its-own-baseline (the T39/Bench-051 discipline), and
    the ledger records the wiring commit so the baseline window is unambiguous.
- [ ] T2 — Composite case generation (M4 generation side):
  - [ ] T2.1 `corpus_gen::generate_composites() -> Vec<CompositeCase>`: compose
    k ∈ {1,2,3,4} atomic cases into one file/scope with order-independent defects
    (same-scope disjoint statements; REJECT composing two mutators that touch the same
    binding — a small compatibility table, pinned by test).
    `CompositeCase { name, members: Vec<GeneratedCase>, broken, gold }` with `gold` =
    composition of the atomic `seed`s (shared module/test-mod scaffolding deduped).
  - [ ] T2.2 G1 bit-identity gate: applying each member's canonical repair in sequence
    to `broken` yields exactly `gold` (text equality); determinism pin (two calls
    byte-identical); count pin (ladder sizes).
  - [ ] T2.3 Difficulty ladder ordered by empirical solve-rate: run the existing
    `--fix-compile` driver over each k once (record-only), order k by solve-rate
    descending. The ladder is DATA (a module table + measured numbers recorded in the
    plan/research note), re-derived only on corpus change. Gate: solve-rate monotone
    decay in k (one tie allowed) with a floor — a non-monotone ladder is a FINDING,
    not a re-pin.
- [ ] T3 — Composite exactness scoring (M4 evaluation side):
  - [ ] T3.1 Per-sub-lint reduction scoring: a driver run on a `CompositeCase` scores
    by per-member reduction — member i counts reduced iff its sub-region matches its
    constructed gold after the fix (region = the member's span in the composite; ONE
    whole-file cargo check per case, the per-member check is textual gold-region
    comparison). Exact by construction; no oracle.
  - [ ] T3.2 Sampled-oracle cross-check vs the 28 real-clippy composite fixtures
    (`eval_fixtures_composite.rs`): the scorer's verdicts agree with the oracle labels
    on ≥95% of a K-sample (the fence-e2e pattern — `#[ignore]`d real-cargo arm +
    always-on arithmetic gates; labeled by the oracle, never by hand).
- [x] T4 — Saturation detector (M5 detector half):
  - [x] T4.1 `score_bench/report.rs`: `saturation(records, epsilon, window) ->
    SaturationVerdict` — marginal heal-rate gain over the last R runs < ε →
    `Saturated`; wired into `print_report` beside the dispersion/vacuous line. Unit
    gates BOTH directions (planted flat history flags; an improving tail does not) +
    ε/R as named consts with derivation comments (start ε = 0.5pp, R = 5; the first
    measured run confirms or re-pins with the numbers recorded).
    **DONE 2026-09-06:** signed comparison (a regression is ALSO `Moving` — this
    detector answers "is the set still measuring", the SVG curve owns regressions);
    window-MEAN reference not prior-run (pinned by
    `saturation_delta_is_against_the_window_mean_not_the_prior_run`); 5 gates +
    clippy 0 under `score_bench`. REAL-history verdict: SATURATED (Δ −0.0pp over
    the last 5 of 45 runs) — the bench has sat at 96.2 since run #3, which is
    exactly the motivation for T5's ratchet.
  - [x] T4.2 Report line `saturation: <verdict> (Δ<eps> over <R>)` — advisory until
    T5 consumes it. (Insufficient-history runs stay quiet; both live verdicts print.)
- [ ] T5 — Generationed bench ratchet (M5 ratchet half):
  - [ ] T5.1 Generation-v2 fixture set: ~30–40 composites (T2 generator, stratified
    over codes × k), construction-labeled, G1-validated (T2.2), oracle-sampled
    (T3.2 ≥95%).
  - [ ] T5.2 Admission gate (non-vacuity): v2 admits ONLY IF the measured heal-rate on
    v2 at current HEAD is LOWER than v1's by a margin — a new generation that doesn't
    lower the rate isn't harder → REFUSE admission, keep v1-only. The refusal path is
    a test.
  - [ ] T5.3 Per-generation floors: `HistoryRecord` gains `generation` (default "v1");
    v1 floors pinned FOREVER (the existing gate untouched); v2 floors pinned at
    admission-time measured − 1 span; the report prints v1 and v2 side by side;
    `score_history.jsonl` stays append-only (records carry their generation).
  - [ ] T5.4 Detector→ratchet loop: when T4 flags `Saturated`, the report names the v2
    set as the resolution (docs line; no automation beyond the print).
- [ ] T6 — GOAT: G1 bit-identity + determinism pins (T2.2); G3 no-regression —
  retrieval floors green, score-bench v1 FLAT-or-better (run recorded), existing
  `dump_misses` byte-identical when unflagged; clippy 0 `--all-targets` in both feature
  states; `ci_feature_guard.sh` all layers; store isolation (temp-dir stores; the live
  `.heal/` store never opened by tests — the Plan-090 rule).
- [ ] T7 — Docs + records: AGENTS.md row (T5/T6 landed + commit hash); issue 072
  T5/T6 checkboxes + status line; queue-snapshot row; a research note ONLY if the
  ladder/verdict produces a finding worth one (else this plan carries it).
- [ ] T8 — Commit + push (`feat: issue 072 T5/T6 — ...`); separate commits for disjoint
  hunk sets (misses wiring vs bench ratchet) so a revert of one never drags the other.

## Non-goals

- No GRPO/policy gradients (riir-train issue 522 owns the training lane); no
  judge-model training; `EvidenceTier` untouched; `clippy_verify` oracle paths untouched.
- v1 floors never move; the composite set does NOT join the v1 denominator (Plan 014's
  frozen-denominator honesty — v2 is a separate reported axis, never a silent widening).
- No automation turning authoring on/off — the frontier order is a listing aid;
  authoring stays human/agent-initiated.

## Discipline notes

- CPU-only unit (no GPU anywhere; safe under any sibling GPU campaign).
- Feature gating: store-adjacent pieces ride `latent_retrieval` (the TrajStore
  convention); `dispersion` / `corpus_gen` / `score_bench` are feature-free.
- Real-cargo arms are `#[ignore]`d (fence/oracle e2e) — the Plan-090 T2 pattern.
- Dispatch reads this plan via ORIGIN (`scripts/plan_read_origin.sh`), not the
  working tree.

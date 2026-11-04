# Issue 206: Probe-authored corpus repair — templated context_notes + demote-execution behind a score-bench referee (Research 649 fusion; ILC leg dropped)

**Status:** open — fusion idea (Research 649 × Latent-Prediction/ILC supergoat candidate; sweep record `katgpt-rs/.research/650_Probe_Grammar_Consumer_Fusion_Sweep.md` @ `e7a746274`), novelty TBD (contested: cleanlab); module-half LANDED 2026-11-04 (`src/corpus_binding.rs`, 9/9 tests, clippy-clean, default lib 2828/0), corpus-half deferred (sibling batch owns the corpus entries files); verdict round 1 REVISE addressed (claim scoped to telemetry-driven writers, `kat_corpus_pr` added as cousin, referee bar pinned to paired LCB95, arm B re-ruled)
**Date:** 2026-11-04
**Repo:** riir-refine (corpus + self_evolve)
**Filing analysis:** 10-analyst fusion sweep, 2026-11-04; the empty-seam claim and every mapped site coordinator-verified (`record_outcome` ×3 sites, `demotion_suggestions` report-only at `src/blueprint.rs` L428/L649, `--active-set-demote` CLI-only)
**Sweep record:** `katgpt-rs/.research/650_Probe_Grammar_Consumer_Fusion_Sweep.md` @ katgpt-rs `e7a746274` (2026-11-04)

## The verified empty seam

**No deterministic TELEMETRY-driven corpus-mutation writer exists** (grepped this
session: `riir-refine/src`, `riir-rethink/src`; katgpt-rs `.research` for prior
framings). Every candidate prior lane ends at a REPORT or a human CLI verb:

- `--gate-calibration` strata verdicts (Supported/Degenerate/Deferred) — report-only.
- blueprint `demotion_suggestions` — report-only, "never auto-executed" printed in its
  own output.
- `--corpus-delta` — gap census report only.
- `--active-set-demote` — explicit CLI only; the marker file is the ONLY demotion
  writer.
- post-mining intake — human/agent-authored from mining batches.

**The nearest shipped cousin, named:** `src/kat_corpus_pr.rs` IS a deterministic
corpus-mutation writer (the devnet→mainnet corpus-PR client; its rebase dedupes/
supersedes by `record_id + blake3(fix)`). It does not close the seam: its input is
OWNER-ATTESTED MINED PAIRS (corpus-PR rows), not runtime verify/score telemetry — the
signal source, not the write mechanics, is the gap. The deterministic writers that DO
exist otherwise are downstream of a human edit (`pins_manifest.py --write`) or an
explicit CLI verb (active_set).

## The idea (what × what)

Research 649's shape — probe → grammar record → behavior-verified discrete emission —
applied to the RULE CORPUS instead of weights:

- **Probe source (already recorded, zero new collection):**
  `LatentFixMemory::record_outcome` / `EvolveRecorder::record_outcome_verified`
  (evidence tiers), score_history strata, `.refine/misses/` queues, the verify-refusal
  tally.
- **Grammar record (NEW — the one new artifact class):** `corpus_binding` =
  `{drive rule × gate stage × failure class}` — BLAKE3-digested, **corpus-epoch-keyed,
  STALE-REFUSES** (a degenerate-cluster diagnosis is stale the moment a batch lands;
  the `sst_certified::verify_against` shape).
- **Params-vocab — RESTRICTED to probe-derivable forms:**
  - (arm A) templated `context_notes` append keyed by failure class, landed via the
    pins-manifest same-commit discipline;
  - (arm B) deterministic execution of the blueprint-named `--active-set-demote`
    command, behind an explicit flag.
  - Template split / span narrowing are **NOT probe-derivable — EXCLUDED**, or the
    loop hallucinates fix content.
- **Referee:** `--score-bench` A/B against the UNEDITED generation N as baseline,
  **paired LCB95 > 0 on the affected strata** (the house bound family — Wilson /
  Beta-LCB / paired-LCB95; no point-estimate bars). Minimum support: n ≥ 30 scored
  cases per affected stratum before the read; below that the arm is UNDECIDED, never
  folded into a verdict.

## Scope restrictions (honest)

- **ILC leg DROPPED:** clustering is commodity k-means over embeddings refine already
  has (`RuleIndex` 8-D + `rule_embed` two-tower); the fusion is really 649-alone →
  refine.
- 649's load-bearing leg (probe-derived edit PARAMETERIZATION — scalar strength window
  + dose sweep) does NOT transfer to text edits: the probe names the broken rule but
  cannot derive fix content. Outside the templated subset this degrades to the existing
  reports — which is why the vocabulary is two arms, not a grammar of arbitrary corpus
  edits.

## Tasks

- [x] Fixture (test-side, synthetic): the seeded degenerate cluster lives in
      `src/corpus_binding.rs` tests (`degenerate_cluster()` — a rule firing with
      systematic compile-gate failures over `RuleOutcomes` counts); the
      real-telemetry fixture (outcomes harvested from actual fix campaigns) is the
      corpus half's job.
- [x] `corpus_binding.rs`: diagnose → record + stale-refusal test — LANDED
      (`GateStage` first-index tie order, `MIN_ATTEMPTS` = 8 floor (UNDECIDED below,
      never flagged), `DEGENERACY_BAR` = 0.5 inclusive, sorted deterministic output,
      digest content-scoped (`recorded_at` deliberately un-hashed — a re-run
      reproduces), `verify_against` refuses `StaleEpoch` with exact bytes; 9 unit
      tests).
- [-] Arm A: templated `context_notes` append (pins-manifest same-commit discipline) —
      DEFERRED 2026-11-04: the sibling batch (Batch 329 voxl distill) owns the corpus
      entries files + `pins_manifest.json` right now; the arm-A edit lands after that
      batch closes (re-open trigger: the batch landing commit).
- [-] Arm B: deterministic demote-execution (the blueprint-named command, run
      programmatically behind an explicit flag) — DEFERRED with arm A (same
      contention; also needs the real-telemetry wiring to have candidates worth
      demoting).
- [-] Referee: score-bench A/B with the pinned paired-LCB95 bar — DEFERRED with arm A
      (the A/B needs a stable corpus generation; the sibling batch is moving it).
- [x] Smallest PoC (module scope) = fixture + binding + stale refusal on the synthetic
      fixture — LANDED. The GOAT question (measured heal-rate gain) stays with the
      deferred referee arm.

## Kill criteria (pre-registered)

Arm A misses the bar AND demote-execution is the only gain ⇒ **downgrade to NOTE**:
fold demote-execution into blueprint as an opt-in flag, PASS the rest, record the
negative. No zombie lane.

## Stationary discipline

Mutates: corpus rows (ONLY the derivable subset — `context_notes` strings + the
active-set marker), `.benchmarks/pins_manifest.json` (same-commit law),
`score_history.jsonl` + generation freeze-tables when the referee runs.
Read-only: rule embeddings (frozen `rule_embed_frozen_v1.bin`), trajectory store,
misses queue, Elo ratings, prior score rows.

## Prior art (novelty contested — the GOAT question is the measured heal-rate gain, not loop mechanics)

- **Confident Learning / cleanlab** (Northcutt et al.) — deterministic class-level
  label-error detection + prune/reassign from observed probabilities; covers the demote
  half. The unpublished remainder: the per-rule drive×gate×gap binding RECORD artifact
  (BLAKE3, corpus-epoch-keyed, stale-refusing) integrated with this pipeline's
  referee/commitment discipline.
- **DRAGO** — arXiv:2610.22568, "Repairing LLM-Generated Specifications with
  Deterministic Diagnosis" — telemetry-constrained (diagnosis-constrained) repair;
  general method, no corpus-record discipline.
- **`kat_corpus_pr`** (shipped cousin, this repo) — deterministic corpus writes keyed
  `record_id + blake3(fix)`, but driven by owner-attested mined pairs over the PR wire,
  never by this pipeline's verify/score telemetry.

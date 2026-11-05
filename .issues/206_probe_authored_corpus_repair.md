# Issue 206: Probe-authored corpus repair — templated context_notes + demote-execution behind a score-bench referee (Research 649 fusion; ILC leg dropped)

**Status:** open — fusion idea (Research 649 × Latent-Prediction/ILC supergoat candidate; sweep record `katgpt-rs/.research/650_Probe_Grammar_Consumer_Fusion_Sweep.md` @ `e7a746274`), novelty TBD (contested: cleanlab); module-half LANDED 2026-11-04 (`src/corpus_binding.rs`, 9/9 tests, clippy-clean, default lib 2828/0); corpus-half **PARTIALLY LANDED 2026-11-04 (live-telemetry diagnose + arm-A writer + first arm-A corpus edit + referee leg measured)** — verdict round 1 REVISE addressed (claim scoped to telemetry-driven writers, `kat_corpus_pr` added as cousin, referee bar pinned to paired LCB95, arm B re-ruled)
**Claimed:** zed-a1ac63f1, 1793890102, expires 2026-11-07T14:48:22+00:00 — the harvest→bind→note CLI verb (the corpus half's next task per the issue's own scope note; re-open trigger: telemetry shows the pipeline exists but no verb wraps the harvest sweep)
**Date:** 2026-11-04
**Repo:** riir-refine (corpus + self_evolve)
**Filing analysis:** 10-analyst fusion sweep, 2026-11-04; the empty-seam claim and every mapped site coordinator-verified (`record_outcome` ×3 sites, `demotion_suggestions` report-only at `src/blueprint.rs` L428/L649, `--active-set-demote` CLI-only)
**Sweep record:** `katgpt-rs/.research/650_Probe_Grammar_Consumer_Fusion_Sweep.md` @ katgpt-rs `e7a746274` (2026-11-04)

## The corpus half — executed 2026-11-04 (this session)

(For the verified empty seam + the idea's provenance, see git history — this section documents the executed corpus half; the sections below it stand as written.)

**Trigger fired:** the re-open condition ("the batch landing commit") — Batch 329 landed
(`f53555e04` + follow-through); the sibling moved to Batch 330 (its WIP,
`entries_57.rs`, stayed untouched — write sets disjoint).

**Live-telemetry harvest (the probe source, zero new collection):**

| source | shape | population |
|---|---|---|
| `.refine/blueprint_history.jsonl` (11 repos with fix campaigns since 2026-10-27) | per-rule `written/reverted/declined` per campaign | 37 campaigns, 23 rules — **0 candidates** (max reverted share 0.00; era total reverts = 1 benign) |
| `.refine/trajectories.kv` durable stores (riir-refine 376 + katgpt-rs 397 + riir-infer 8 + seal-game-editor 315 rows; riir-chain store EMPTY — 0 rows decoded) | cumulative `success_count/fail_count` per (rule, shape) + `component` (E/ρ/W/C) + `last_seen_at` | 46 rules with attempts — **1 formal candidate** |

**Diagnose (the pinned predicate: attempts ≥ 8 ∧ fail-share ≥ 0.5 inclusive):**

- **`hashmap-with-capacity`** — 18 attempts, 9 fails (0.50, at the inclusive bar), ALL W-class (drafter: parse/compile), across 3 stores; last seen **15 d ago** (~5 corpus batches back — the counts' vintage is disclosed in the note itself). The one candidate.
- **Watch (below the pinned bar, not flagged):** `redundant_closure` — 29 attempts, 14 fails (0.48), W-class, `consecutive_fails`=3, last seen 6 d ago. Recorded as watch data; the bar is pinned, no near-miss rescue.
- **Recent-14 d window: ZERO candidates.** The current corpus era is healthy by the pinned predicate.
- **Arm-B population: zero.** C-class (verify-persist / gate) failures: none at the predicate anywhere in the stores — there is nothing to demote; arm B stays deferred WITH this concrete reason (not contention).

**Arm A LANDED (the writer + the first edit):**

- `corpus_binding.rs`: `FailureClass` (W/C, shared tags with `component::ComponentClass`), `degenerate_context_note` (the templated note — probe-derivable values only: class, stage word, counts, share, epoch label; no fix content, per the scope restriction), `CorpusBinding::compose_context_note` (the composition gate: stale-epoch refusal + rule-present check + diagnosis-identity check → the rendered note; the note never carries the binding digest — the digest binds the pre-edit epoch, the landed note would falsify it by construction; the `epoch_short` label is the honest carrier). **16/16 tests** (13 prior-shape + 3 writer), clippy-clean, default lib **2835/0**.
- `entries_03.rs`: `hashmap-with-capacity` carries the FIRST live templated note (`context_notes: None → Some(note)`, epoch `ac280021`), landed under the pins-manifest same-commit discipline (counts unchanged — 296 corpus total; the three count pins re-pinned: annotated 243→244, unannotated 53→52 ×2).
- **Pin-keeper check (the corpus-binding fixture gate):** `floor_corpus_integrity`-class pins all green at default features.

**Referee leg — measured, verdict UNDECIDED (by the pinned support floor, structurally):**

- Affected-stratum support: gen-5 (the rust_perf ladder, `--generation 5`) admits **exactly 4 fixtures** containing the hashmap atom (k=1..k=4 composites, `EXCLUDED_IDS_V5` empty) — 4 < 30 ⇒ **the pre-registered read is UNDECIDED regardless of outcome**; no clippy-oracle generation can exceed this (the rule touches only its own atoms).
- The A/B itself (baseline ac280021 vs note-spliced corpus, one-off probe — dev scratch, never committed): **gen-5 atom tables byte-identical (14 atoms, 0 diffs — atoms are a pure function of `buggy_code`, which the note does not touch); the healed output of `gen5-k1-hashmap-with-capacity` byte-identical both postures** — the gen-5 heal dispatch is compile-time (`FIXABLE_RULES` + `collect_edits`); corpus entry TEXT is not an input. **Structural identity: the bench cannot regress and cannot improve on the note.** The consumer-facing advisory channel (agents reading suggestions; the latent index's corpus text) is the note's real effect surface and is NOT bench-measurable.
- kill-criteria check: "arm A misses the bar" — the arm did NOT miss (there was no measurable bar to miss: structural identity + support 4 < 30 = UNDECIDED, never folded). The NOTE stays landed as an annotated rule (normal corpus practice — advisory metadata, no GOAT gate on annotations); the **telemetry-driven WRITER LANE** stays opt-in/deferred until a real candidate exists in a RECENT window (the current one was 15 d stale). No zombie lane: the writer is 3 small functions over the landed module, inert until wired into a harvest verb.

**Honest scope note (recorded, not hidden):** the v1/v2 clippy-oracle generations were NOT run as full A/B builds (2 × cold release builds for a structurally-identity axis); the gen-5 structural identity argument covers the axis the rule actually touches, and the v1 gen-1 series (35 fixtures) measures clippy-lint vocabulary the rust_perf note cannot move. The harvest→binding→note pipeline exists in-repo; wiring it as a CLI verb is the next corpus-half task when a recent-window candidate exists.

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
      **LANDED 2026-11-04** (the deferred trigger fired: Batch 329 landed; the
      writer + the first live edit on the ONE diagnosed rule,
      `hashmap-with-capacity`; sibling `entries_57.rs` WIP untouched). The
      HARVEST→BIND→NOTE pipeline is in-repo; a CLI verb wrapping the harvest
      (stores sweep → diagnose → compose gate) re-opens when a recent-window
      candidate exists.
- [-] Arm B: deterministic demote-execution (the blueprint-named command, run
      programmatically behind an explicit flag) — DEFERRED with a CONCRETE
      reason (2026-11-04): the live-store diagnose has **zero C-class
      candidates** at the pinned predicate (all flagged failures are W-class;
      there is nothing the note channel cannot address) — demote-execution
      has no population to execute on; re-opens when a C-class candidate
      appears in the stores.
- [-] Referee: score-bench A/B with the pinned paired-LCB95 bar — MEASURED
      2026-11-04, verdict **UNDECIDED** (support floor 4 < 30, structural for
      single-rule arms; gen-5 identity proven byte-level, see the corpus-half
      section). The bar itself may need re-pricing for single-rule edits (a
      per-rule stratum cannot reach 30 on a finite fixture set) — that is a
      referee-design decision for a follow-up, not a silent bar-bend.
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

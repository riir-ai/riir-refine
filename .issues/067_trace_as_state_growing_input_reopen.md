# Issue 067: growing-input conditioning reopen — condition-first placement + same-rule gating (arXiv:2609.02702)

**Status:** OPEN — T1 (the trigger instrumentation) LANDED 2026-09-04; the trigger **fired in practice** (2026-09-05 store read); **T2 (arm G' construction) LANDED 2026-09-05** — same-rule channels, condition-first placement, CLI arm `G2`, degradation byte-contract pinned. T3 (the CF A/B) remains — see its store-selection note below.

## Trigger-state measurement (2026-09-05, no-build read)

`cargo heal --memory-stats` (the installed binary, read-only) over the riir-ai store:
2338 rows, **2281 Certified**, `uninlined_format_args` alone holds **1811 recorded
outcomes** (2405 W / 15 L on the ELO surface) — the K=3 same-rule Certified
threshold is cleared by ~600× for the top rules, and the katgpt-rs / riir-train /
riir-mmorpg-examples stores clear it too (`nan-unsafe-sort-comparator` 128 W,
`uninlined_format_args` 43 W, 378 Certified respectively). Every future
`--fix --write` run querying a top rule logs the T1 fire line; T2/T3/T4 are now
waiting on nothing but a build window.

## T1 record (2026-09-04)

The instrument shipped (`src/traj_store.rs` `TrajStore::certified_density()` + the
`ReopenTrigger067` per-run trigger in `src/bin/heal.rs`):

- **Condition encoded**: `lint_key` holds `>= 3` (K=3) DISTINCT `Certified` rows in the
  durable store. Heuristic (unverified first sight) and Withdrawn (absorbing failure)
  rows do not count.
- **Hooks** (the apply moment = "queried lint"): the clippy-lane `--fix --write`
  apply loops (both `fix_verify` / non-`fix_verify` branches) keyed by rule id, and
  the `--fix-compile` applied loop keyed by `code:strategy` (`traj_key`).
- **Cost**: one lazy full scan of the store per run (first queried lint), then a
  BTreeMap lookup per apply — zero hot-path store traffic after the scan. Logs at
  most once per key per run, to stderr:
  `self-evolve: issue 067 reopen trigger FIRED: '<key>' holds N Certified same-rule
  trajectories (K=3) — same-rule exemplar channels (issue 067 T2) are unblocked for
  this lint`.
- **E2E proven**: a scratch store planted with 3 Certified `identity_op` rows + a
  real `--fix --write` run fired the line exactly once on the apply (and the heal
  itself landed). Unit gate: `certified_density_counts_only_certified_rows_per_lint_key`
  (Certified counts, Heuristic/Withdrawn excluded, multi-key, scan stability).
- **Dev-store baseline (2026-09-04)**: 6942 rows, 2 Certified keys, **0 keys >= K=3**
  — not fired; the watch is now automatic at every durable-store heal run.
- Light builds (no `latent_retrieval`) keep the envelope backend by design and are
  NOT instrumented — the reopen condition is defined over the TrajStore.

## Context

riir-clippy Bench 051 (Issue 049, closed 2026-08-27) measured the growing-input conditioning POC: arm G appended trajectory outcomes + failure patterns AFTER the code module (`cf_workers_ai::prompt_growing`). Verdict `NEGATIVE-no-gain` — strict-keep tied 5/9 across arms A/B/G at 2.8× arm-B tokens — and the channels were **cross-rule** by construction (the reachable lints are off-corpus, so no same-rule history existed anywhere).

katgpt-rs Research 530 (arXiv:2609.02702 "Trace as State", Zou & Tang 2026) adds two falsifiable deltas that recalibrate that negative without overturning the closed verdict:

1. **Placement.** For causal processors, conditioning/derived state placed BEFORE the context dominates placing it after — 26/27 model×task×metric cells, up to +38.8 EM, with a worst-case exponential memory argument (`[z,C]` = `b` bits vs `[C,z]` = `b·2^b` bits). Arm G's channels were condition-LAST relative to the code (the paper's controlled variable). At the healer's short prompt scale the expected effect is small, but the paper's Question-First and Re2 arms show re-anchoring gains even at 256K with far weaker traces than same-rule exemplars would be.
2. **Relevance hazard.** The paper's Random-Trace arm (traces from other problems, prepended) scores BELOW the no-trace first pass (Parents EM 14.2 vs 29.2) — weakly-relevant conditioning content is actively harmful, and its trace-count ablation is monotone only for SAME-problem traces. Bench 051's channels were exactly the weakly-relevant class: the negative may be relevance-poisoned (positive channel × harmful content cancellation), not evidence against the channel design.

Design corollary: the paper's Answer-Feedback arm (45.4 EM) vs full-trace arm (81.8 EM) says process evidence conditions better than outcome evidence — relevant to how future channels should quote trajectories (keep the reasoning/patch narrative, not just before/after pairs), when same-rule content exists.

## Tasks

- [x] T1 — trigger instrumentation: define the reopen condition as "the TrajStore holds ≥K (K=3) Certified same-rule trajectories for a queried lint"; measure store density at heal time (one counter; cheap). Log when the trigger first fires. **LANDED 2026-09-04 — `TrajStore::certified_density()` + the per-run `ReopenTrigger067` (hooks: clippy apply loops + `--fix-compile` applied loop); e2e-proven; dev store not yet fired (2 Certified keys, 0 ≥ 3).**
- [x] T2 — arm G' construction (only after T1 fires): same-rule-only exemplar channels (cap 3, Certified-first), **condition-first placement** — channels before the code module — reusing `prompt_growing`'s splice mechanics and keeping arms A/B byte-identical as controls. Empty-channel degradation must stay byte-identical to arm B (the existing test's contract). **LANDED 2026-09-05** (`src/cf_workers_ai.rs` + `src/bin/fixer_regate.rs`): `prompt_growing_same_rule` (channels between the instruction header and the module — `[z, C]`), `RegateHarness::trajectory_context_same_rule` (boundary-safe `{lint}:` retrieval prefix + the family-scoped pattern fold — a bare prefix would admit `ptr_arg_extra`, pinned by test), CLI arm `G2`, JSONL `growing_prime_channels_empty`, DRY golden `growing2=` column. Arm G's channel rendering extracted byte-preserved (`render_growing_channels`) — arm G's own contract test still passes untouched. Degradation contract pinned two ways: the unit test (`empty ⇒ == prompt_prepass`) and the live DRY run (`growing2 == prepass` hashes on every dev-store sample — this repo's store has 0 keys ≥ K=3, so G2 ≡ B here by construction). Gates: 1455/0/7 feature lib (4 new tests), clippy 0 in touched files, DRY planted controls ALL PASS + decline 10/10 + v2edit EM 16/16.
- [ ] T3 — A/B on the Bench 049 harness (reachable-miss set + strict-keep + never-wrong filters), CF arm. Pre-registered verdict: G' beats B iff strict-keep ≥ B + 1 with zero kept-wrong, at token cost ≤ 2× B. A second negative closes this class for the short-context healer permanently; the placement principle then applies only to long-context surfaces.
      **T3 store note (2026-09-05, from the T2 validation):** the harness opens `TrajStore::open(".")` CWD-relative — the riir-clippy dev store has **0 keys ≥ K=3**, so `--arm G2` from this repo's root measures arm B byte-identically (every row records `growing_prime_channels_empty: true`). The A/B must run where the queried lints hold ≥K Certified same-rule rows: run with CWD set to a repo whose store has fired keys (the 067 T1 read: riir-ai — `uninlined_format_args` 2405W/15L Certified-dense — but note the reachable-miss set is off-corpus lints, so first verify which queried lints actually fire in that store via the T1 `ReopenTrigger067` log lines a `--fix --write` dry pass prints), or plant a scratch store keyed by the reachable set's own lints. The trigger log line IS the per-lint ground truth for sample selection.
- [ ] T4 — record in a new `.benchmarks/{NNN}_*.md` (next free number at write time), cross-linked to Bench 051 and katgpt-rs `.research/530`; close this issue per the noise-reduction rule.

## Refs

- Bench 049 (harness + arms A/B verdict) · Bench 051 (the negative being reopened) · katgpt-rs `.research/530` (paper distillation + signal-diff) · `src/cf_workers_ai.rs` (`prompt_growing`, L404–453) · `src/bin/fixer_regate.rs` (`--arm G`).

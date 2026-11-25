# 260 — the kernel_opt G1 store went UNRE-EMITTED for 6 batches; 4 fixture fails landed UNRECORDED

**Status:** open — surfaced at the Batch 419 landing (the standing G1 full emit found them); repair owned by the next maintained kernel_opt lane.
**Claimed:** amo-idle-i260@amo, 1795639776, expires 2026-11-27T20:49Z

## The finding

The standing G1 full run + `--emit-status` (the annotation channel's refresh) last ran at **Batch 400** (`7276a689`, 2026-11-23). Batches 402, 404, 409, 410, 411, 412, 417 landed kernel_opt entries without re-emitting, so:

- **8 fixture rules fail the compile/run cascade but were never recorded as `fail`** until the Batch-419 emit:
  - `replay-group-eligibility-excludes-mutable-surface` + `patch-validity-against-declared-resource-contract` (entries_gpu_153, Batch 341) — pre-existing at HEAD, **disclosed honestly at B400** ("4 FAIL pre-existing-at-HEAD, stash-run verified identical");
  - `accelerator-backend-first-tile-acceptance-gate` (entries_gpu_154, Batch 345) + `sah-tree-depth-bounded-by-traversal-stack` (entries_gpu_157, Batch 362) — the same B400-disclosed pair;
  - `winograd-admission-budget-floor-routes-to-generalist` + `logical-reduce-width-bitcast-float-carveout` (entries_gpu_166, **Batch 410** `cccb76fb`) — landed green-implied, never emit-recorded;
  - `per-row-bounds-bitmask-hoisted-loader` + `dilation-base-table-tap-odometer` (entries_gpu_167, **Batch 411** `8d9de968`) — same class.
- Their `context_notes` annotation read **`validated: fixture-green`-adjacent nothing / fail-closed Unverified** for 6 batches — the honest fail-closed state, but the fleet could not SEE which rules' fixtures actually fail until an emit ran.

Spot-check of one arm (dilation-base-table-tap-odometer, pair posture): the naive arm's own assert fails (`[2,5,0,3,1,4]` vs `[0,3,1,4,2,5]`) — the fixture's ref-arm expectation disagrees with the modeled odometer ordering, i.e. a fixture bug of the Issue-193 class (the entry discipline validated each fixture manually at landing via the standalone preflight files `b410/b411_*_fixtures.rs`, which PASSED — the divergence is between the standalone preflight arm and the corpus-embedded strings, exactly the drift the standing harness exists to catch).

## The rule being defended

**A landing that moves the corpus re-emits the G1 store in the same commit** (or the batch's landing note discloses the emit debt with a named owner). The emit is cheap (~100 s measured at batch 335; the full run is a campaign lane, not a test target). Two prior misses are on record (B402's census miss repaired at B404, B417's census miss repaired at B419 — see goat_g1.rs's len-pin chain); this is the THIRD instance of the same shape one layer down: the census was repaired, the STORE was not.

## The repair (next kernel_opt lane or a dedicated small unit)

- [ ] Re-run each of the 8 failing rules' fixtures standalone to confirm the ref-arm expectation bug (not a harness artifact) — the Issue-193 triage.
- [ ] Fix the 4 B410/B411 fixtures in their entries files (the embedded ref/opt strings), re-emit, flip the rows to `green`; or, if a fixture encodes a genuinely underdetermined model, re-word the rule's fixture to the honest assertion and record the change.
- [ ] Re-emit the full store; the 4 pre-existing B341/B345/B362 fails get the same triage (they have ridden since B400).
- [ ] Add the landing-row convention note: emit-status rides every corpus-touching kernel_opt landing.

NOTHING INVENTED — every fail row a live full-run read at the Batch-419 landing HEAD (amo, 2026-11-26). Session: amo-idle-i413b@amo.

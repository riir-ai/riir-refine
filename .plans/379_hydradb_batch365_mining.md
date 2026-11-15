# 379 — Batch 365: hydradb distill mining (rust_perf: io/caching/concurrency/measurement-honesty)

**Status:** CLAIMED 2026-11-15 (zed-m3-i379-hydradb@m3, epoch 1794697180, expires 2026-11-18T23:00Z) — mining started; Plan 375's InferenceForge slot stays its holder's (lease to 11-21, honored); this batch runs the NEXT queue slot so the lane advances while amo is offline.

Staged — verdict recorded (Research 411); source pinned `916637477f3f34766b4905b270bd4292109d77e6` (v0.4.0, ELv2 — transcription-only, original fixtures). Queue slot: behind nominal Batch 364 (InferenceForge / Plan 375) — landing order re-derives the number from the score_history tail at landing.

## Pre-registered bars

- GOAT lead `degree-gated-probe-with-doubling-retry`: Pareto validator, integer-exact (probes + neighbor-reads counted over a synthetic degree stream with injected overflow watermarks), two-run stable, wins ≥ 2/3 sizes. Fails ⇒ demote same-commit (the ungoat law).
- Co-lead `bfs-depth-epoch-marker-dedup`: memset-count/alloc axis (N memsets → 0), wins ≥ 2/3 sizes, G1 bit-identical traversal order.
- Measurement-honesty rows land suggestion-only NotBenchable-by-domain (the dial9 class) — no bench bar applies.

- [ ] T0: re-clone `.raw/hydradb` at the pin (the verdict clone was deleted at close); re-verify every quote the batch cites (Protocol A) — the research note carries file:line for all 17 table rows.
- [ ] T1: dedup re-verification at landing HEAD — especially: candidate 14 (`consume-budget-yield-before-ready-await`) vs `adaptive-yield-consecutive-ready` (the owed adjudication — my lane-prompt omission, flagged in Research 411 §Dedup); candidate 3's `witness` grep at authoring HEAD; `one-shared-disk-cache` vs `cache-share-capped-at-measured-distinct-working-set` both-directions in-rule.
- [ ] T2: entry authoring, first tranche (benchable + honesty): `degree-gated-probe-with-doubling-retry` (GOAT lead), `bfs-depth-epoch-marker-dedup`, `spawn-blocking-queue-wait-split-from-compute`, `latency-histogram-split-on-structural-populations` (+ its gap-not-guess + label-from-data sub-laws), `derived-index-write-touches-only-changed-keys`.
- [ ] T3: entry authoring, second tranche (storage-transcription tier, NotBenchable-by-domain): `wal-watermark-pinned-before-snapshot`, `tail-walk-cost-cap-declines-to-rebuild` (carry the in-source two-sided regime — the xlog no-gate counter-example at index_store.rs:219–223), `frozen-artifact-plus-delta-tail-overlay`, `existential-witness-early-exit-scan`, `catalog-memo-scoped-to-the-snapshot-no-invalidation`.
- [ ] T4: faces (count-neutral folds): histogram derived-count/bounds-pinned face; count-before-fault face (measurement-honesty family); align-64 second-instance source row on `align-64-false-sharing` (COLLISION recorded in Research 411 — cite as independent instance, never a new rule).
- [ ] T5: fixtures — ORIGINAL reproductions only (ELv2); every arm standalone-compiled AND executed pre-entry; falsification arms per the 4-arm discipline.
- [ ] T6: standing gates — batch needles, parse gate ×2 domains, clippy ×3 postures, retrieval floors (re-pin only on measured regression), lib count floors, pins_manifest --write (count web one-landing).
- [ ] T7: GOAT cell for the benchable pair at the shared postures; record in `.benchmarks/` per numbering.
- [ ] T8: docs one-landing — AGENTS.md/README/.docs prose heads, category tables if a new family lands (LSM/storage-engine seed), score-bench only if clippy_lints surface touched, release-set binary rebuild only if owed by a maintained-binary lane (corpus-only delta rides the standing precedent).
- [ ] T9: research-note cross-ref (411 → landed batch numbers), queue-snapshot discharge line, commit+push on develop; dual_allocation + numbering gates at push.

## Deferred / watch

- `- [-]` placement rendezvous + provider-mutation second pass (unread remainders, Research 411 §Caveats) — separate verdict if a >10-rule batch is wanted.
- `- [-]` buddy-wal frontier + `generation-switched-index-rebuild` pair — lane-intel notes only (ndb/dapps shapes), not corpus entries yet.

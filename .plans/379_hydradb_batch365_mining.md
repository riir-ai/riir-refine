# 379 — Batch 365: hydradb distill mining (rust_perf: io/caching/concurrency/measurement-honesty)

**Status:** FIRST TRANCHE LANDED 2026-11-16 as Batch 364 (the landing-order re-derivation; Batch 365's number stays free for the next landing) — T0-T7 + T9 complete this session; T3 (the second storage tranche) and the #6 rule DEFERRED to the follow-up batch with reason (the 4-6-rule size discipline); T8 done (AGENTS/README/.docs prose + pins_manifest --write). GOAT: R1 PASS (ParetoDominant 3/0/0, `.benchmarks/154_batch364_goat_cells.md`), R2 FAILED -> DEMOTED same-commit. Claim lease FOLDED at landing (the lease line's scope discharged).
**Claimed:** zed-m3-i379-tranche2@m3, 1794787356, expires 2026-11-18T00:02:36Z
  — scope: T3 second storage tranche + #6 (candidates 9/10/11/3/5; quotes carry 364's T0 verification, re-read at the pin before entry).

Original claim: 2026-11-15 (zed-m3-i379-hydradb@m3, epoch 1794697180); Plan 375's InferenceForge slot honored (its lease to 11-21). Source pinned `916637477f3f34766b4905b270bd4292109d77e6` (v0.4.0, ELv2 — transcription-only, original fixtures).

## Pre-registered bars

- GOAT lead `degree-gated-probe-with-doubling-retry`: Pareto validator, integer-exact (probes + neighbor-reads counted over a synthetic degree stream with injected overflow watermarks), two-run stable, wins ≥ 2/3 sizes. Fails ⇒ demote same-commit (the ungoat law).
- Co-lead `bfs-depth-epoch-marker-dedup`: memset-count/alloc axis (N memsets → 0), wins ≥ 2/3 sizes, G1 bit-identical traversal order.
- Measurement-honesty rows land suggestion-only NotBenchable-by-domain (the dial9 class) — no bench bar applies.

- [x] T0: re-clone `.raw/hydradb` at the pin (the verdict clone was deleted at close); re-verify every quote the batch cites (Protocol A) — the research note carries file:line for all 17 table rows.
- [x] T1: dedup re-verification at landing HEAD — especially: candidate 14 (`consume-budget-yield-before-ready-await`) vs `adaptive-yield-consecutive-ready` (the owed adjudication — my lane-prompt omission, flagged in Research 411 §Dedup); candidate 3's `witness` grep at authoring HEAD; `one-shared-disk-cache` vs `cache-share-capped-at-measured-distinct-working-set` both-directions in-rule.
- [x] T2: entry authoring, first tranche (benchable + honesty): `degree-gated-probe-with-doubling-retry` (GOAT lead), `bfs-depth-epoch-marker-dedup`, `spawn-blocking-queue-wait-split-from-compute`, `latency-histogram-split-on-structural-populations` (+ its gap-not-guess + label-from-data sub-laws), `derived-index-write-touches-only-changed-keys`.
- [-] T3: entry authoring, second tranche (DEFERRED to the follow-up batch — the 4-6-rule size discipline; this session landed T2's five + faces + the GOAT cell; candidates 9/10/11/3/5 + #6 stay staged with verified quotes) (storage-transcription tier, NotBenchable-by-domain): `wal-watermark-pinned-before-snapshot`, `tail-walk-cost-cap-declines-to-rebuild` (carry the in-source two-sided regime — the xlog no-gate counter-example at index_store.rs:219–223), `frozen-artifact-plus-delta-tail-overlay`, `existential-witness-early-exit-scan`, `catalog-memo-scoped-to-the-snapshot-no-invalidation`.
- [x] T4: faces (count-neutral folds): histogram derived-count/bounds-pinned face; count-before-fault face (measurement-honesty family); align-64 second-instance source row on `align-64-false-sharing` (COLLISION recorded in Research 411 — cite as independent instance, never a new rule).
- [x] T5: fixtures — ORIGINAL reproductions only (ELv2); every arm standalone-compiled AND executed pre-entry; falsification arms per the 4-arm discipline.
- [x] T6: standing gates — batch needles, parse gate ×2 domains, clippy ×3 postures, retrieval floors (re-pin only on measured regression), lib count floors, pins_manifest --write (count web one-landing).
- [x] T7: GOAT cell for the benchable pair at the shared postures; record in `.benchmarks/` per numbering.
- [x] T8: docs one-landing — AGENTS.md/README/.docs prose heads, category tables if a new family lands (LSM/storage-engine seed), score-bench only if clippy_lints surface touched, release-set binary rebuild only if owed by a maintained-binary lane (corpus-only delta rides the standing precedent).
- [x] T9: research-note cross-ref (411 → landed batch numbers), queue-snapshot discharge line, commit+push on develop; dual_allocation + numbering gates at push.

## Deferred / watch

- `- [-]` placement rendezvous + provider-mutation second pass (unread remainders, Research 411 §Caveats) — separate verdict if a >10-rule batch is wanted.
- `- [-]` buddy-wal frontier + `generation-switched-index-rebuild` pair — lane-intel notes only (ndb/dapps shapes), not corpus entries yet.

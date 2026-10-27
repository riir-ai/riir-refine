# Plan 289 — Batch 285+ candidate: bbtree ordered-structure-internals rules → rust_perf

**Status:** IN FLIGHT — staged behind the rust_perf slot gate (R134 first per the standing queue law) + behind live claims walk #17 / B279; claim below

Research: `.research/322_bbtree_cache_conscious_ordered_map.md` (source pin `XiangpengHao/bbtree @ 8adae97a0ed97e5c98fa2790312eb0aec9719846`, MIT OR Apache-2.0, v0.1.5). RENUMBERED 288→289 at birth (dual allocation with the burn-cubecl lane's `.plans/288_burn_cubek_*`, their commit `795f2e38` landed first — pushed numbers never reused; the local dual_allocation gate predated their push). B-number note: the burn lane's plan 288 owns the B284 authoring slot; this plan converts at the NEXT batch number after theirs (285+), still behind the rust_perf slot gate. Workspace manual track: 8 sibling issues filed 2026-10-25 citing the research note (those are NOT this batch's tasks — this plan is the CORPUS half only).

**Claimed:** zed-glm-idle-b305/amo, 1793093090, expires 2026-10-29T09:44Z
(the 48 h zed-m3max claim lapsed unlanded at 09:44Z per the standing expiry law; re-claimed at slot open — same yields: walk #19 and plan-283 rider faces for any shared gate; CARGO_TARGET_DIR=/tmp/plan289.)

## Tasks

- [ ] Re-verify the claim lease + queue head at slot open (fetch; the research note's staging row must still be the top bbtree row; re-clone `.raw/bbtree` at the pinned sha and re-verify every quote the entries carry)
- [ ] Dedup re-verification (both vocabularies, ALL FIVE candidates) against rust_perf 270 + kernel_opt — the research note's zero-hit evidence is from 2026-10-25; the corpus moves
- [ ] Author `order-preserving-inline-key-encoding` (caching, GOAT lead) — fixture: ORIGINAL minimal repro (sorted Vec of heap byte-string keys, inline-word compare + tie-full-compare vs `binary_search` with full `Ord`); exhaustive G1 gate per the Batch-59 law: BE order-equivalence matrix (all 2-byte boundary windows × LCG strings × prefix shapes), integer keys incl. i64 sign-flip; regime notes carry the random-prefix penalty (4–23%) + the two-regime WIDTH story
- [ ] Author `count-based-simd-sorted-slot-search` (simd) — fixture: rank-search over a ≤64-slot sorted array, count-of-below/equal vs `binary_search`; sentinel invariant documented in-rule (slots past len = max, or cut at len); G1: exhaustive rank equivalence vs `partition_point` over slot counts 0..=64 × boundary targets
- [ ] Author `batch-miss-interleave-tiny-executor` (concurrency; latency BAR class) — fixture: arena of chained nodes (deterministic layout), batch of 64 interleaved prefetch+resume vs sequential; pair with the `tests/common/ab_timing.rs` interleaved-medians shape (the Issue-723 lesson) + a loud-zero defence (`black_box` both arms — the Issue-855 lesson); regime: working set > ~L2 else serial (the `batches()` heuristic); name the simpler level-by-level-lanes alternative (+10–15 ns/key author-side) in context_notes
- [ ] Author `seqlock-copy-read-unlatched` (concurrency, **NotBenchable** `bench_spec: None`) + `page-self-aligned-hot-node` (allocation/caching) — suggestion-only annotations, contention/TLB-regime notes per the research table
- [ ] Rider faces (count-neutral, in corpus DATA never comments): `typeid-hand-specialization-stable` compile-time face → ride on `order-preserving-inline-key-encoding`; D28 kinship cite → `loop-invariant-dispatch-resolution-hoist` context_notes one-line ("class transfers to type-level dispatch; bbtree D28 instance")
- [ ] Standing gates: batch needle gate + parse; default lib count; `rust_perf_bench` lib count; clippy −D ×3 postures; retrieval floors RE-MEASURED (corpus delta ⇒ re-pin check); freeze gen2+gen3 e2es run-to-confirm (P30 expected no movement for rust_perf-only delta)
- [ ] GOAT for the two A-grade rules: G1 (exhaustive, above) · G2 `--release` latency with `scripts/bench_preflight.sh` PROVENANCE line quoted beside the numbers (M3, verified-quiet box) · G3 no consumer regression · G4 alloc-count axis (encoding + count rules must be alloc-free both arms)
- [ ] Score-bench + frontier snapshot appended; freeze-table regeneration ONLY if the corpus generation moves (gen2/gen3 e2e verdict decides, plan-137 T7.5 law)
- [ ] Docs: AGENTS.md rust_perf count bump + docs ledger row + landing row in `.distill/001_mining_queue_snapshot.md` — ONE commit; research note gets a one-line status update to RECORD (batch landed) pointing at the batch commit
- [ ] Maintained release-set binary rebuilt + installed BEFORE push (rust_perf is release-set surface — §0 law; `cargo refine --version` prints the new stamp)
- [ ] Commit `feat:` on develop, push; `rm -rf .raw/bbtree` in the same unit (the ephemeral-clone law)
- [ ] Post-batch: consumer-residue screen NOT owed (these are additive patterns, no guard landed); report yield vs the 4–5 estimate in the landing row (honest miss ok)

## Notes

- Expected landing: 4–5 own rules + 2 rider faces (research 322 estimate). Honest drops allowed at T1 with reasons — the C+ typeid candidate is the most likely fold.
- The two A-grade rules are the batch's reason to exist; if BOTH falsify at G2 on our box, land the survivors only and record the negative vs the author's numbers (different silicon is a regime difference, not a contradiction — quote both).
- never build in the workspace target dir; `CARGO_TARGET_DIR=/tmp/plan289`, removed when done.

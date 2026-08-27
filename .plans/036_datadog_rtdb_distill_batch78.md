# Plan 036 — Batch 78: Datadog RTDB/Monocle distill (rust_perf)

**Status:** COMPLETE 2026-08-27 — both rules landed (86→88; layout 7, concurrency 8). The bench INVERTED the verdict-time ranking honestly: the GOAT-lead measured ParetoDominated (memory-for-speed trade, annotated) and rule 2 measured ParetoDominant. Record: Research 092 + the AGENTS.md Batch 78 entry.

Source: https://www.datadoghq.com/blog/engineering/rust-timeseries-engine/ (Aug 2025). Verdict: YES B — 2 keep-candidates, lead `narrow-counter-overflow-promotion`. Routing: rust_perf (layout + concurrency) NOT kernel_opt.

## Tasks

- [x] T1 Dedup re-verification at authoring time (the corpus may have moved since Research 092 — re-grep both vocabularies for both keep-candidates; a single hit is a LEAD, read it before calling duplicate)
- [x] T2 Author `narrow-counter-overflow-promotion` (layout): buggy = u64 counter array accumulation sweep; fixed = u32 slots + `checked_add` promote-on-overflow to a u64 sidecar/overflow tier; regime notes (counter ARRAYS, common-case-under-2^32, NOT lone counters, NOT expected-overflow domains); differentiate vs `usize-index-to-u32` (index-field domain-bound narrowing vs value-counter representation + escape hatch); original fixture (no source paste)
- [x] T3 Bench the lead rule on the Pareto validator (rust_perf_bench): memory/bytes axis exact (the halving), throughput axis honest (isolated expectation 1.2-2x on counter-heavy sweeps — the source's +70% is the COMPOUND, never cite as isolated); verdict-stable ×2; alloc-det stable; annotate if Mixed/regime-fragile
- [x] T4 G1 equivalence gate for the lead rule: promoted totals bit-identical to u64 totals at every step (property test over adversarial count sequences incl. sequences crossing 2^32 — the ported-numeric MANDATE shape: exhaustive/boundary where feasible + LCG)
- [x] T5 Author-or-drop `cache-line-block-partitioned-lock` (concurrency): K-items-per-`#[repr(align)]`-block under one mutex vs per-item locks vs one global lock; the B65 differentiation written INTO the rule (false-sharing elimination vs true-sharing partitioning); bench 2-12t; drop honestly if the isolated mechanism measures inside B65's shadow (the adjacency is real — if the fix shape collapses to "use CachePadded", it IS B65)
- [x] T6 Single-flight lead: probe-or-drop (DROPPED — the parse-only detection shape, as predicted) (expect drop — parse-only detection shape is the blocker; record the drop reason in the batch note)
- [x] T7 Standing gates: `batch78_pseudocode_parses` syn gate + fixtures fire/clean-span tests
- [x] T8 Count pins walked (rust_perf header 86→87/88, layout/concurrency category rows, benchable count, annotated count, the serde round-trip + drafter hidden pins per the B75 lesson list)
- [x] T9 GOAT: G1 parse + G2 lib tests default AND bench-feature states + G3 clippy 0 both states `--all-targets` + retrieval floors HELD (re-pin honestly if corpus growth moves prose) + full CI guard
- [x] T10 Docs: `.docs/11_domains/rust_perf.md` row + AGENTS.md batch note + queue-snapshot update (same commit)
- [x] T11 Commit `feat: batch 78 — Datadog RTDB distill (rust_perf NN→NN)` + push

## Non-goals

- The architectural drop set (shard-per-core, CoDel, tiered compaction, series cache design) — recorded in Research 092, not mined.
- The 60x/5x/2x headline numbers — never citable as isolated mechanisms.

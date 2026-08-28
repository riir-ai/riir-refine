# Plan 044 — Batch 83: SIMD bit-slicing predicate distill (rust_perf)

**Status:** IN PROGRESS — Research 097 verdict recorded; batch execution pending quiet box
**Source:** arXiv:2608.26368 (Research 097) — bit-plane transposed zero-decompression predicate evaluation
**Lane:** rust_perf (corpus 88 → ~89-90)

## Tasks

- [x] T1 — Read paper in full + record distill verdict (`.research/097_simd_bitslicing_predicate_distill.md`)
- [x] T2 — Dedup grep at verdict time: `bool-to-bitpack` (flag density), `ternary-bit-plane-u64-as-2x-u32` (GPU weight dequant), `sparse-set-bit-iteration-tz-blsr` (word-internal iteration), `simd-chunk` (vehicle not mechanism), `soa-hot-field` (field layout — differentiate in-rule), `branchless-conditional` (rider target) — no coverage of the transferable mechanisms. Re-verify at entry-authoring time.
- [ ] T3 — Corpus entry: `bitplane-transpose-repeated-predicate-scan` (data-structure or simd; regime gates: amortization 7-25 scans, b≤16, past-cache crossover, point-lookup + IN-list + wide-eq limits; in-rule differentiation vs soa-hot-field/simd-chunk/bool-to-bitpack)
- [ ] T4 — Corpus entry (conditional): `float-order-preserving-bits` (arithmetic; bench = radix-over-transformed-bits vs comparison sort; if the isolated-transform bench cannot separate the transform's contribution, DEMOTE to a context mention inside T3's entry + record why in the research note)
- [ ] T5 — Rider: `branchless-conditional` doc-comment gains the selectivity-constant context (paper scalar 5.4K→1.2K ops/s swing vs bit-plane constant; cite Research 097)
- [ ] T6 — Bench specs through the Pareto validator (`rust_perf_bench`): scans-per-encode axis; top cell sized PAST the L2 boundary (the cache-resident regime is where the paper itself measured 0.5-0.6× regressions — do not hide it, annotate it); G1 mask bit-identity vs scalar reference (integer-exact, no tolerance); alloc-count axis where applicable
- [ ] T7 — Standing gates: batch pseudocode/fixture gates; count-pin walk (88→89/90 across ALL pin sites — header, benchable, annotated, module doc, serde, drafter tests); clippy 0 both states
- [ ] T8 — GOAT: G1 (bit-identity) / G2 (Pareto verdict) / G3 (no-regression: default lib + bench-state lib counts) / G-det (alloc stability) / verdict-stable ×2 / retrieval floors 14/14 HELD; quiet-box check FIRST (`uptime` + no sibling compute consumers)
- [ ] T9 — Docs: `.docs/11_domains/rust_perf.md` table rows + counts; AGENTS.md corpus count + batch entry; `.distill/001_mining_queue_snapshot.md` new batch entry (same commit)
- [ ] T10 — Commit `feat: batch 83 — ...` + push; close plan (all boxes `[x]` or `- [-]` with reasons)

## Notes

- GOAT benches are CPU-timing-sensitive → require the quiet-box gate (Plan 337).
  Verdict + plan landed first as the zero-contention unit while siblings held
  the box at load ~8.3.
- The paper's honest negatives (1M-row sub-1× cells, wide-type equality
  losses, IN-list linear scaling) are REGIME CONTENT for the rule, not
  embarrassments — they pin the annotation.

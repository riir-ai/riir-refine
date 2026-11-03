//! Entries slice - Batch 326 (2026-11-04): one `rust_perf` rule from the
//! rust-lang/rust PR #164484 distill (Research 363 / Plan 330):
//! `adaptive-sparse-blocked-table-encoding` (layout) - an index-keyed
//! table whose payload slots are mostly a default value stops paying
//! full-width for every default slot: at flush time the encoder counts
//! non-defaults (the census only when the presence index could fit in
//! less space than the dense body), picks the SMALLER of dense and
//! sparse by arithmetic (sparse when >25% of entries are default, the
//! value count fits u32, and `index_bytes + non_defaults*width` beats
//! `len*width`), and the sparse shape stores one presence bitmap (u64)
//! plus one cumulative u32 base per 64-entry block - the base counting
//! all preceding blocks' non-defaults so a lookup never scans - followed
//! by the non-default values packed in table order, with the mode flag
//! riding the header word's low bit (`(width << 1) | sparse`). Source:
//! rust-lang/rust #164484 "Use adaptive sparse encoding for metadata
//! tables" (merged 2026-11-01T01:40Z, merge `026faeb1afef7581b2e49b2512d361d09819cb69`,
//! head `de21c98c951e91adb4abf353494ba7739f425ab8`, r=nnethercote,
//! milestone 1.101.0, `METADATA_VERSION` 11 -> 12), re-verified against
//! the files API at that merge this batch. NotBenchable-by-domain (the
//! size-axis annotation class, `box-large-enum-variant` /
//! `narrow-counter-overflow-promotion` precedent): the headline is
//! metadata SIZE (-20.15% avg, PR thread comment 6246665077 - the
//! prompt's 20.18% corrected to the thread figure) and max-RSS
//! (-3.7/-3.8% primary x213-249 benches), neither a latency/alloc
//! bar a criterion-style fixture can pin; `bench_spec: None`, with the
//! accepted instruction-cost trade carried as the anti-regime evidence
//! in `context_notes` (`perf-regression-triaged`: +0.3-0.4% mean
//! primary instructions, "cycles and wall-time were neutral. We decided
//! before merging that this trade-off was reasonable" - never quote the
//! label as a defect; it IS the trade).
//!
//! Fixtures are ORIGINAL machinery (never the rustc source pasted - the
//! metadata table is re-authored as plain-Rust stand-ins with neutral
//! names), each compiled and executed standalone (rustc --edition 2021
//! -O) before this table was assembled; both arms encode the same
//! 4096-slot ~95%-default table, every slot round-trips bit-exact
//! through both decoders, and the size axis is visible standalone:
//! dense 32,768 bytes vs adaptive 2,424 bytes at the fixture's default
//! mix (13.5x; upstream's real metadata mix measured -20.15% - the
//! fixture's default share is the extreme end, not a claimed transfer).

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── layout: an index-keyed table that is mostly-default stops
        // paying full width for every default slot — the flush-time
        // census + smaller-of gate picks dense or per-64-block
        // presence-bitmap encoding, whichever is smaller --------------
        // Batch 326 / rust-lang/rust #164484 @ merge 026faeb1afef
        // (compiler/rustc_metadata/src/rmeta/table.rs +84/-14; the
        // deleted FIXME(eddyb) names the same HAMT popcnt trick
        // michaelwoerister proposed — this PR is that trick, landed
        // and measured).
        PerfRule {
            rule_id: "adaptive-sparse-blocked-table-encoding".to_string(),
            name: "an index-keyed table whose payload slots are mostly one default value serializes dense full-width -> flush-time adaptive encoding: count non-defaults (the census only when a presence index could fit in less space than the dense body), pick the SMALLER of dense and sparse by arithmetic - sparse when >25% of entries are default, the value count fits u32, and index_bytes + non_defaults*width < len*width - and the sparse shape stores one presence bitmap (u64) plus one cumulative u32 base per 64-entry block (the base counts all preceding blocks' non-defaults so a lookup never scans) followed by the non-default values packed in table order, the mode flag riding the header word's low bit ((width << 1) | sparse)"
                .to_string(),
            category: "layout".to_string(),
            buggy_code: r#""" + buggy + """#.to_string(),
            fixed_code: r#""" + fixed + """#.to_string(),
            context_notes: Some(
                "MEASURED upstream, size-domain (NotBenchable HERE by metric class, bench_spec None -- the size-axis precedent: box-large-enum-variant, narrow-counter-overflow-promotion): metadata -20.15% avg (rust-lang/rust #164484 thread, comment 6246665077 -- the commissioning prompt's 20.18% corrected to the thread-verified figure), max-RSS -3.7/-3.8% primary x213-249 benches, binary -0.4/-0.6%; instructions +0.3-0.4% mean primary ACCEPTED (`perf-regression-triaged`, r=nnethercote: 'cycles and wall-time were neutral. We decided before merging that this trade-off was reasonable') -- never quote the label as a defect, it IS the anti-regime evidence. The standalone fixture at the extreme default mix (95% default, 4096 slots, width 8): dense 32,768 bytes vs adaptive 2,424 bytes (13.5x), every slot round-trips bit-exact through both decoders -- the fixture proves the MECHANISM, never the upstream magnitude. SOURCE (rust-lang/rust #164484 'Use adaptive sparse encoding for metadata tables', merged 2026-11-01T01:40Z, merge 026faeb1afef7581b2e49b2512d361d09819cb69, head de21c98c951e91adb4abf353494ba7739f425ab8, r=nnethercote, milestone 1.101.0, METADATA_VERSION 11 -> 12; hunks re-verified against the files API at the merge pin this batch, table.rs +84/-14): the census pre-gate counts non-defaults only when `index_bytes < len * width` could win ('the census pays only when the census can pay' -- counting a 100k-slot table to decide against sparse is its own cost); the sparse gate is `len - non_defaults > len / 4` (>25% defaults -- the occupancy knob, 50% tested upstream and reverted: 'didn't show a clear benefit'; 75% non-default = dense) AND `u32::try_from(non_defaults).is_ok()` AND `index_bytes + non_defaults * width < len * width` (smaller-of, arithmetic, never measured); the per-block index is 12 bytes (u64 bitmap LE, bits from the LSB, + u32 base LE = the cumulative non-default count of ALL preceding blocks, so a set bit's packed position is `base + (bitmap & (bit - 1)).count_ones()` with no bitmap scan); values follow in table order; the header word packs `(width << 1) | sparse` (the low bit is the mode flag, width shifts left -- METADATA_VERSION bumped 11 -> 12 in the same PR: adopting the shape in any persistent format is a version-bump discipline, readers must know the flag bit). The deleted FIXME(eddyb) in table.rs names the origin: 'investigate more compact encodings ... the HAMT popcnt trick (divide things into buckets of 32 or 64 items and then store bit-masks)' per michaelwoerister -- this PR is that trick, landed and measured. REGIME: index-keyed tables whose payload is dominated by one default value, serialized as a persistent blob (metadata, indexes, checkpoint tables, config snapshots), written rarely and read often; the win is SIZE/RSS, never wall-clock. ANTI-REGIMES: read-latency-critical random access over nearly-full tables (the sparse decode adds two loads + a popcount per probe -- upstream's accepted +0.3-0.4% instruction cost is exactly this cost at their dense-majority tables); tables where defaults are rare (the smaller-of gate falls through to dense by arithmetic -- the census pre-gate then skips the count too); mutation-heavy in-memory structures (this encodes a FLUSHED representation, not a live container). DEDUP (greps at authoring over rust_perf + the kernel_opt kin set, the verdict-stage greps re-run plus the verdict-round-1 reviewer's rank-select/succinct/elias-fano/roaring set): minimal-perfect-hash-static-lookup (entries_33) owns LOOKUP over a frozen sparse key-value table (search side) -- THIS owns the STORAGE encoding of an index-keyed default-heavy table (flush side); range-table-page-memoization (entries_33-family) owns lookup memoization -- unrelated; distinct-index-bitmap-ascending-compaction (kernel_opt gpu set) compacts DISTINCT emitted indices via a bitmap -- THIS compacts DEFAULT-heavy TABLE SLOTS via presence bitmaps + cumulative bases (frequency-rank for search vs popcount-rank storage: the reviewer's split, adjudicated distinct); the bit-plane transposition rule (entries_31-family) owns columnar VALUE layout -- unrelated; csr/tile-coo/attention-row-mask kernel_opt kin own GPU sparsity formats -- same philosophy, different subject (blocked presence over a DENSE-DOMAIN table, not sparse-matrix coordinates). FIXTURE PROVENANCE: original machinery (never rustc source) -- both arms encode the same 4096-slot width-8 table at ~95% defaults; the buggy arm pays len*width for every slot; the fixed arm carries the full gate + census + bitmap/base packing + mode-bit header and round-trips all slots bit-exact including defaults. Wave: Batch 326 / Plan 330 / Research 363. Cite: rust-lang/rust #164484 @ merge 026faeb1afef7581b2e49b2512d361d09819cb69 (compiler/rustc_metadata/src/rmeta/table.rs +84/-14; encoder.rs + decoder.rs + mod.rs renames), Batch 326 / Plan 330."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

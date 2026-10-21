//! Entries slice B — Batch 270 (2026-10-21, Plan 274 / Research 306): the
//! splash-wave serving pool, slice B (3 caching rules) — incoai/splash @
//! `18f9c69` + `2712e67` + `716760d`, Apache-2.0; every quote re-verified at
//! the .raw/b270/splash clone this batch.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── caching: the auto share of a hit-only row cache is capped at the MEASURED distinct
        //    working set (the hit-flatness knee, with headroom over the estimated largest working
        //    set) and the streamed experts — whose misses cost decode — take the rest.
        PerfRule {
            rule_id: "cache-share-capped-at-measured-distinct-working-set".to_string(),
            name: "A serving cache whose hits are bounded by the DISTINCT rows its workload reads is handed a static share of the memory budget before the misses-cost consumer takes the rest ('auto planned its row cache at up to 2 GiB before the streamed experts took what was left') → derive the auto share from a measured hit-rate census: 'The cache hits only rows read before, and a request reads few distinct rows'; 'the hit rates did not follow the size' (a cold 32K prompt hit 0.79/0.76 alike at 256 MiB, 512 MiB, 1 GiB and 2 GiB); 'No run filled its cache, even at 256 MiB'; 'The cache above 512 MiB held nothing, and every byte of it was one the streamed experts lacked, whose misses do cost decode' — so the share stops at the measured knee ('Auto's share is now 512 MiB, and the experts take the difference'), which still holds headroom above the estimated largest working set (256 MiB 'holds fewer rows than a 256K prompt's estimated 2.3M, so the share stops at 512 MiB', not lower); an explicit size is unchanged and 'A table that loads whole still does'.".to_string(),
            category: "caching".to_string(),
            buggy_code: r##"#![allow(dead_code)]
// Provenance: incoai/splash @ 18f9c69bacf53d4561bc1cb00ff2e2ad1cd2177e
// (runtime/engine/MemoryPlan.cpp:21-25 - kTableShareBytes 2 * kGiB ->
// 512 * kMiB), Apache-2.0. ORIGINAL machinery: the source's streamed-memory
// planner re-authored as a two-consumer budget partition with a hit model;
// no splash code is copied. BUGGY arm: the auto share is STATIC.

pub const MIB: u64 = 1 << 20;
/// Rows a MiB of row cache holds (the census: 2.12M rows at 256 MiB, about
/// 128 B a row).
pub const ROWS_PER_MIB: u64 = (1 << 20) / 128;
/// Host MiB one streamed expert slot occupies.
pub const MIB_PER_EXPERT_SLOT: u64 = 4;
/// The budget stand-in (the source's 36 GB M5 Max run).
pub const BUDGET_MIB: u64 = 36 * 1024;
/// The estimated largest single working set (a 256K prompt's 2.3M rows).
pub const ESTIMATED_MAX_ROWS: u64 = 2_300_000;

/// The engine's memory plan: what the row cache may hold and what the
/// streamed experts get with the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub cache_rows: u64,
    pub expert_slots: u64,
}

impl Plan {
    pub fn cache_mib(&self) -> u64 {
        self.cache_rows.div_ceil(ROWS_PER_MIB)
    }

    /// The partition covers the budget (to the expert-slot floor).
    pub fn total_mib(&self) -> u64 {
        self.cache_mib() + self.expert_slots * MIB_PER_EXPERT_SLOT
    }
}

/// Split a budget: the row cache takes `share_mib` (capped at the budget),
/// the streamed experts take the rest. The public surface both arms share.
pub fn plan(budget_mib: u64, share_mib: u64) -> Plan {
    let cache_mib = share_mib.min(budget_mib);
    Plan {
        cache_rows: cache_mib * ROWS_PER_MIB,
        expert_slots: (budget_mib - cache_mib) / MIB_PER_EXPERT_SLOT,
    }
}

/// Hits only rows read before: a replayed request's hits are what the
/// capacity covers of its distinct rows.
pub fn hits(distinct_rows: u64, cache_rows: u64) -> u64 {
    distinct_rows.min(cache_rows)
}

/// The run's distinct working set (the census stand-in: 8 rows a prefill
/// token, decode adds none - a 32K prompt reads ~0.26M distinct rows, a
/// 128K prompt ~1.05M; repeats read no new rows).
pub fn distinct_rows_read(prefill_tokens: u64) -> u64 {
    prefill_tokens / 32 * 256
}

/// An explicit --ple-memory replaces auto in both arms, byte-identically.
pub fn plan_explicit(budget_mib: u64, ple_memory_mib: u64) -> Plan {
    plan(budget_mib, ple_memory_mib)
}

/// A table that fits the share loads whole (no streaming share applies).
pub fn loads_whole(table_mib: u64, share_mib: u64) -> bool {
    table_mib <= share_mib
}

pub mod buggy {
    use super::*;

    /// The static share: the row cache takes up to 2 GiB before the
    /// streamed experts take what is left (the pre-18f9c69 constant).
    pub const AUTO_SHARE_MIB: u64 = 2 * 1024;

    pub fn auto_share(_measured_rows: u64, _estimated_max_rows: u64) -> u64 {
        AUTO_SHARE_MIB // static: the census rides in the commit, never read
    }

    pub fn plan_auto(budget_mib: u64, measured_rows: u64, estimated_max_rows: u64) -> Plan {
        plan(budget_mib, auto_share(measured_rows, estimated_max_rows))
    }
}

// -- equivalence: the serving observable both arms share --------------------

#[test]
fn the_cache_still_covers_the_measured_workload() {
    let distinct = distinct_rows_read(128 * 1024); // ~1.05M rows
    let p = buggy::plan_auto(BUDGET_MIB, distinct, ESTIMATED_MAX_ROWS);
    assert_eq!(hits(distinct, p.cache_rows), distinct);
    let distinct32 = distinct_rows_read(32 * 1024); // ~0.26M rows
    let q = buggy::plan_auto(BUDGET_MIB, distinct32, ESTIMATED_MAX_ROWS);
    assert_eq!(hits(distinct32, q.cache_rows), distinct32);
}

#[test]
fn an_explicit_ple_memory_is_honored_byte_identically() {
    let p = plan_explicit(BUDGET_MIB, 2048);
    assert_eq!(p.cache_rows, 16_777_216);
    assert_eq!(p.expert_slots, 8_704);
    assert_eq!(p.total_mib(), BUDGET_MIB);
}

#[test]
fn a_table_that_loads_whole_still_does() {
    assert!(loads_whole(400, buggy::auto_share(1_048_576, ESTIMATED_MAX_ROWS)));
    assert!(!loads_whole(4096, buggy::auto_share(1_048_576, ESTIMATED_MAX_ROWS)));
}

// -- the perf shape: the measurements are carried and never read ------------

#[test]
fn the_static_share_ignores_the_measurements_it_carries() {
    assert_eq!(buggy::auto_share(262_144, ESTIMATED_MAX_ROWS), 2 * 1024);
    assert_eq!(buggy::auto_share(16_777_216, ESTIMATED_MAX_ROWS), 2 * 1024);
}

#[test]
fn the_hit_rate_does_not_follow_the_size_past_the_working_set() {
    // the flatness the census measured: 0.79/0.76 alike at 256 MiB,
    // 512 MiB, 1 GiB and 2 GiB - hits stop at the distinct set
    let distinct = distinct_rows_read(32 * 1024);
    let at = |share_mib: u64| hits(distinct, plan(BUDGET_MIB, share_mib).cache_rows);
    assert_eq!(at(256), at(512));
    assert_eq!(at(512), at(1024));
    assert_eq!(at(1024), at(2048));
}

#[test]
fn the_static_share_starves_the_experts_whose_misses_cost() {
    let distinct = distinct_rows_read(128 * 1024);
    let static_plan = buggy::plan_auto(BUDGET_MIB, distinct, ESTIMATED_MAX_ROWS);
    // a share that merely covers the estimate plus the measured set
    let covering_share = (ESTIMATED_MAX_ROWS + distinct).div_ceil(ROWS_PER_MIB);
    let covered = plan(BUDGET_MIB, covering_share);
    assert!(covered.cache_rows >= distinct + ESTIMATED_MAX_ROWS / 4);
    assert!(
        static_plan.expert_slots < covered.expert_slots,
        "the static share leaves the streamed experts fewer slots"
    );
    // the waste: 2 GiB of capacity over a ~130 MiB census (16x the
    // measured set) - "The cache above 512 MiB held nothing"
    assert!(static_plan.cache_rows >= 16 * distinct);
}"##.to_string(),
            fixed_code: r##"#![allow(dead_code)]
// Provenance: incoai/splash @ 18f9c69bacf53d4561bc1cb00ff2e2ad1cd2177e
// (runtime/engine/MemoryPlan.cpp:21-25 - kTableShareBytes 2 * kGiB ->
// 512 * kMiB), Apache-2.0. ORIGINAL machinery: the source's streamed-memory
// planner re-authored as a two-consumer budget partition with a hit model;
// no splash code is copied. FIXED arm: the auto share is capped at the
// MEASURED distinct working set (the hit-flatness knee) with headroom
// above the estimated largest working set, and the experts take the rest.

pub const MIB: u64 = 1 << 20;
/// Rows a MiB of row cache holds (the census: 2.12M rows at 256 MiB, about
/// 128 B a row).
pub const ROWS_PER_MIB: u64 = (1 << 20) / 128;
/// Host MiB one streamed expert slot occupies.
pub const MIB_PER_EXPERT_SLOT: u64 = 4;
/// The budget stand-in (the source's 36 GB M5 Max run).
pub const BUDGET_MIB: u64 = 36 * 1024;
/// The estimated largest single working set (a 256K prompt's 2.3M rows).
pub const ESTIMATED_MAX_ROWS: u64 = 2_300_000;

/// The engine's memory plan: what the row cache may hold and what the
/// streamed experts get with the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub cache_rows: u64,
    pub expert_slots: u64,
}

impl Plan {
    pub fn cache_mib(&self) -> u64 {
        self.cache_rows.div_ceil(ROWS_PER_MIB)
    }

    /// The partition covers the budget (to the expert-slot floor).
    pub fn total_mib(&self) -> u64 {
        self.cache_mib() + self.expert_slots * MIB_PER_EXPERT_SLOT
    }
}

/// Split a budget: the row cache takes `share_mib` (capped at the budget),
/// the streamed experts take the rest. The public surface both arms share.
pub fn plan(budget_mib: u64, share_mib: u64) -> Plan {
    let cache_mib = share_mib.min(budget_mib);
    Plan {
        cache_rows: cache_mib * ROWS_PER_MIB,
        expert_slots: (budget_mib - cache_mib) / MIB_PER_EXPERT_SLOT,
    }
}

/// Hits only rows read before: a replayed request's hits are what the
/// capacity covers of its distinct rows.
pub fn hits(distinct_rows: u64, cache_rows: u64) -> u64 {
    distinct_rows.min(cache_rows)
}

/// The run's distinct working set (the census stand-in: 8 rows a prefill
/// token, decode adds none - a 32K prompt reads ~0.26M distinct rows, a
/// 128K prompt ~1.05M; repeats read no new rows).
pub fn distinct_rows_read(prefill_tokens: u64) -> u64 {
    prefill_tokens / 32 * 256
}

/// An explicit --ple-memory replaces auto in both arms, byte-identically.
pub fn plan_explicit(budget_mib: u64, ple_memory_mib: u64) -> Plan {
    plan(budget_mib, ple_memory_mib)
}

/// A table that fits the share loads whole (no streaming share applies).
pub fn loads_whole(table_mib: u64, share_mib: u64) -> bool {
    table_mib <= share_mib
}

pub mod fixed {
    use super::*;

    /// 512 MiB: the measured knee. Hit rates did not follow the size from
    /// 256 MiB up; no run filled even 256 MiB; and the share still holds
    /// the estimated largest working set with room.
    pub const KNEE_MIB: u64 = 512;

    /// The auto share: the measured knee, never below what the estimate
    /// needs (256 MiB held fewer rows than a 256K prompt's 2.3M - the 512
    /// MiB knee holds it with room). A bigger estimate can push the share
    /// past the knee; the source's census lands both bounds under it, so
    /// auto is 512.
    pub fn auto_share(measured_rows: u64, estimated_max_rows: u64) -> u64 {
        let hold = |rows: u64| rows.div_ceil(ROWS_PER_MIB);
        KNEE_MIB.max(hold(estimated_max_rows)).max(hold(measured_rows))
    }

    pub fn plan_auto(budget_mib: u64, measured_rows: u64, estimated_max_rows: u64) -> Plan {
        plan(budget_mib, auto_share(measured_rows, estimated_max_rows))
    }
}

// -- equivalence: the serving observable both arms share --------------------

#[test]
fn the_cache_still_covers_the_measured_workload() {
    let distinct = distinct_rows_read(128 * 1024); // ~1.05M rows
    let p = fixed::plan_auto(BUDGET_MIB, distinct, ESTIMATED_MAX_ROWS);
    assert_eq!(hits(distinct, p.cache_rows), distinct);
    let distinct32 = distinct_rows_read(32 * 1024); // ~0.26M rows
    let q = fixed::plan_auto(BUDGET_MIB, distinct32, ESTIMATED_MAX_ROWS);
    assert_eq!(hits(distinct32, q.cache_rows), distinct32);
}

#[test]
fn an_explicit_ple_memory_is_honored_byte_identically() {
    let p = plan_explicit(BUDGET_MIB, 2048);
    assert_eq!(p.cache_rows, 16_777_216);
    assert_eq!(p.expert_slots, 8_704);
    assert_eq!(p.total_mib(), BUDGET_MIB);
}

#[test]
fn a_table_that_loads_whole_still_does() {
    assert!(loads_whole(400, fixed::auto_share(1_048_576, ESTIMATED_MAX_ROWS)));
    assert!(!loads_whole(4096, fixed::auto_share(1_048_576, ESTIMATED_MAX_ROWS)));
}

// -- the perf shape: the share is measured, and the experts take the rest ---

#[test]
fn the_share_is_derived_from_the_census_not_pinned() {
    assert_eq!(fixed::auto_share(1_048_576, ESTIMATED_MAX_ROWS), 512);
    // the knee already covers this estimate: no push
    assert_eq!(fixed::auto_share(1_048_576, 3_500_000), 512);
    // a bigger estimate pushes the share past the knee
    assert_eq!(fixed::auto_share(1_048_576, 4_500_000), 550);
}

#[test]
fn the_cap_keeps_headroom_above_the_estimated_largest_working_set() {
    // 256 MiB is disqualified: 2,097,152 rows < the 2.3M estimate
    assert!(256 * ROWS_PER_MIB < ESTIMATED_MAX_ROWS);
    // the knee holds it with room (over 1.8x the estimate)
    let share = fixed::auto_share(1_048_576, ESTIMATED_MAX_ROWS);
    assert!(share * ROWS_PER_MIB >= ESTIMATED_MAX_ROWS);
    assert!(share * ROWS_PER_MIB >= 3_500_000);
}

#[test]
fn the_experts_take_the_difference() {
    let distinct = distinct_rows_read(128 * 1024);
    let p = fixed::plan_auto(BUDGET_MIB, distinct, ESTIMATED_MAX_ROWS);
    assert_eq!(p.cache_mib(), 512);
    assert_eq!(p.expert_slots, 9_088); // (36864 - 512) / 4
    // the static 2 GiB share would have left 8,704 - the streamed experts
    // gain 384 slots, the M6's +16/+20/+35% decode story
    assert_eq!(p.expert_slots - plan(BUDGET_MIB, 2048).expert_slots, 384);
    assert_eq!(p.total_mib(), BUDGET_MIB);
}

#[test]
fn the_hit_rate_does_not_follow_the_size_past_the_working_set() {
    // the flatness the knee is read off: hits stop at the distinct set
    let distinct = distinct_rows_read(32 * 1024);
    let at = |share_mib: u64| hits(distinct, plan(BUDGET_MIB, share_mib).cache_rows);
    assert_eq!(at(256), at(512));
    assert_eq!(at(512), at(1024));
    assert_eq!(at(1024), at(2048));
}"##.to_string(),
            context_notes: Some("Grade B — NotBenchable decision-content (the win is a budget-PARTITION decision sized from a measured hit-rate census — hit flatness across share sizes is the evidence, and the tok/s deltas are source-reported foreign-box figures, PROVENANCE never bars; a Rust microbench of the stand-in's min/clamp arithmetic would measure the fixture, the B136 class). SOURCE (incoai/splash @ 18f9c69bacf53d4561bc1cb00ff2e2ad1cd2177e 'Give auto's PLE row cache 512 MiB and the streamed experts the rest', Apache-2.0; clone .raw/b270/splash at the pin; every quote read back at the pin THIS batch). THE DEFECT (commit message, verbatim): 'Where the PLE table streams, auto planned its row cache at up to 2 GiB before the streamed experts took what was left.' The removed constant's comment had justified the static share by symmetric hit rates (the pre-commit MemoryPlan.cpp comment, verbatim): 'a prefill's stages and the decode cycles' rows hit it nearly as often as a larger one.' THE CENSUS (commit message, verbatim): 'The cache hits only rows read before, and a request reads few distinct rows: a 128K-token prompt and 512 decoded tokens left 1.14M rows cached (about 130 MiB), scenario 5's lanes (three passes at one, two and four lanes) 0.26M, two 32K prompts and one pass of the lanes 0.68M. No run filled its cache, even at 256 MiB (2.12M rows), and the hit rates did not follow the size: at 256 MiB, 512 MiB, 1 GiB and 2 GiB a cold 32K prompt hit 0.79 on the M5 Max and 0.76 on the M6, the 128K prompt 0.94 and decode 0.35 to 0.49, except in two M5 Max runs where memory pressure took rows back. The cache above 512 MiB held nothing, and every byte of it was one the streamed experts lacked, whose misses do cost decode.' THE LAW (commit message, verbatim): 'Auto's share is now 512 MiB, and the experts take the difference. A table that loads whole still does, and --ple-memory SIZE is unchanged; where every expert loads, the difference goes to the KV.' THE CONSTANT (MemoryPlan.cpp:21-25, verbatim): '// The streamed PLE table's row cache before the experts take the rest. It // hits only rows read before, and a request reads few distinct rows: a // 128K-token prompt about 1.1M, a quarter of what 512 MiB holds. A larger // cache hits no more often and leaves streamed experts less.' + 'constexpr uint64_t kTableShareBytes = 512 * kMiB;' (was 'constexpr uint64_t kTableShareBytes = 2 * kGiB;'). THE HEADER CONTRACT (MemoryPlan.hpp:200-202, verbatim): 'then the routed experts load whole where that leaves KV its allowance, and otherwise stream through all that the allowance and the PLE table's share leave; the table loads whole where the rest holds it, and otherwise streams through its share, 512 MiB at most.' THE HEADROOM CLAUSE (commit message, verbatim): '256 MiB gained nothing more on the M5 Max and stayed within the M6's noise, and holds fewer rows than a 256K prompt's estimated 2.3M, so the share stops at 512 MiB.' — the cap is not the measured set itself: it is the measured knee WITH headroom above the estimated largest single working set (512 MiB holds ~4.2M rows at the census's ~128 B/row; 256 MiB's ~2.1M fails the 2.3M estimate). THE LIVENESS ASYMMETRY (commit message, verbatim): 'The expert store takes its memory when the engine starts, the row cache only as rows arrive, so a plan whose 2 GiB cache stayed mostly empty left about 1.5 GiB of host memory unused that the experts now use; a 2 GiB cache that fills reaches the same total.' MEASURED (source-reported, foreign boxes — M6 24 GB two passes each and M5 Max 36 GB, Flash-Next Q2_0 @ ed59f920 + the DFlash2 draft, auto experts, a fresh server per run; 2 GiB against 512 MiB): M6 expert slots 4,556 (18.5%) against 5,721 (23.3%), the experts' decode hit rate 0.827/0.758/0.726 at one/two/four lanes against 0.892/0.847/0.826, aggregate decode 23.7/27.4/23.4 tok/s against 27.4/32.8/31.6 ('+16%, +20%, +35%'), a cold 32K prompt 650 against 660 tok/s; M5 Max expert slots 11,913 (48.5%) against 13,079 (53.2%), the experts' decode hit rate 0.977 against 0.985, aggregate decode 59.0/93.5/121.1 tok/s against 61.0/96.3/127.0 ('+3%, +3%, +5%; a draft-verify cycle 5% and 8% shorter at one and four lanes'); the tree without memory options plans what --ple-memory 512M did (13,079 slots, 4,427,535 rows) and decodes 61.5/94.9/124.5 tok/s, against 59.4/92.4/118.8 with --ple-memory 2G. Cold-prompt PREFILL moved little (650 against 660; 1,769 against 1,790; the 128K prompt 1,719 against 1,742 and its decode 44.1 against 45.3) — the win is decode-side expert misses, which is the point. TESTS (commit message, verbatim): 'engine-memory-plan pins Flash-Next's table at 512 MiB on the M6, the M5 Max and the M5 Pro, and beside --expert-memory 12G.' REGIME: one memory budget shared between a cache whose hits are bounded by the DISTINCT rows its workload reads (row/prefix/KV-adjacent caches) and a streaming consumer whose misses cost the serving metric; auto/default planning paths (an explicit operator size stays honored, and a part that loads whole never streams). ANTI-REGIME: caches whose hit rate still climbs with size (no flatness — no knee to stop at); unbounded distinct sets (the cap degenerates to the estimate's headroom floor alone); consumers whose misses are cheap (starving them buys nothing); single-consumer budgets. DEDUP ADJUDICATION: distinct from cache-budget-first-sessions-take-remainder-never-reshrink (rust_perf entries_37, B257, TensorFold @ 406b2090 — WHO is sized first under one cap and a never-reshrink check after start; THIS owns HOW BIG the cache-first consumer's auto share may be — they compose: that rule reserves the cache first, this bounds the reservation at the measured distinct working set); distinct from kept-prompt-states-pinned-host-cache-budget (entries_37, @ a739444c — WHERE kept states live under a host-AVAILABLE cap: 8 GiB unasked, at most half of MemAvailable; THIS caps at a MEASURED WORKING SET, a hit-flatness census, not a meminfo reading); distinct from discretionary-cache-growth-keys-on-spare-not-available (entries_26, B237, the SAME repo @ ffc2d4be — the growth axis: WHAT memory the cache may grow into, spare never available; THIS owns the planned SHARE — growth admission vs planned partition, they compose); distinct from slru-probation-protected-promote-on-second-touch (entries_35, B255 — which entries eviction keeps WITHIN a fixed capacity; THIS sizes the capacity itself); distinct from prompt-item-per-consumer-kernel-measured (entries_13, B200 — the measured-cutoff-in-the-rule SHAPE, a per-kernel item width; same 'measured, never guessed' discipline, different axis). Greps 'hit rate|flat|working set|distinct rows|row cache|table share' over rust_perf + kernel_opt: no rule caps an auto cache share at a measured distinct working set; the hit-rate hits are bench-regime prose (paired-probe-layout-present-vs-absent's probe hit-rate gate, sorted-input-previous-row-fast-path's fast-path hit rate). NOVEL: the hit-flatness census as the sizing evidence (hits flat across share sizes => every byte above the distinct set starves the misses-cost consumer) has no corpus owner. Cite: @ 18f9c69bacf53d4561bc1cb00ff2e2ad1cd2177e — runtime/engine/MemoryPlan.cpp:21-25 (+ the removed :21-24 comment, from the diff), runtime/engine/MemoryPlan.hpp:200-202, dev/tests/engine/engine_memory_plan_test.cpp (+8/-8), commit message (all quotes and figures); Batch 270 / Plan 274 / Research 306.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: a cache-reuse assertion in a smoke/real test is gated on the cache's eviction
        //    counter — reuse required unless the count rose, and then exactly one re-encode; a
        //    re-encode the count does not explain still fails, as does a request that did neither.
        PerfRule {
            rule_id: "cache-reuse-assertion-gated-on-eviction-counter".to_string(),
            name: "A smoke check asserts a cache REUSE outright ('the image behind another system prompt comes from the embedding cache without the vision tower') on a cache that gives memory back under pressure → count the evictions and gate the assertion on the counter: 'The runtime counts the entries the embedding cache lets go of, past its bytes or to a reclaim' (/status 'embedding_evictions', /metrics 'splash_image_embedding_evictions_total'), and the smoke 'requires the image behind another system prompt to reuse its rows unless the count rose since, and then to be encoded again, once; a re-encode the count does not explain still fails, as does a request that neither reused nor encoded' — 'rows == (0, 1) or (evicted and rows == (1, 0))'; without the counter 'The check holds only where nothing took the rows, and /status could not tell the two apart', and 'An eviction that should not have happened passes the smoke as well: the engine's tests check which reclaims take the rows'.".to_string(),
            category: "caching".to_string(),
            buggy_code: r##"#![allow(dead_code)]
// Provenance: incoai/splash @ 2712e6768d5c3bf56b320b23df77ce2df7e88807
// (runtime/model/Model.hpp:668-671, runtime/model/Runtime.cpp:616-624,
// runtime/engine/Status.cpp:494-496, dev/tests/smoke_real.py:339-351),
// Apache-2.0. ORIGINAL machinery: a serving embedding cache with a reuse
// oracle; no splash code is copied. BUGGY arm: no eviction counter exists,
// so the smoke's reuse check is unconditional - /status cannot tell an
// eviction-explained re-encode apart from a defect.

use std::collections::HashMap;
use std::collections::VecDeque;

/// The image telemetry /status exposes. The pre-fix surface counts the
/// encodes and the reuses - and nothing else.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImageTelemetry {
    pub encodes: u64,
    pub embedding_reuses: u64,
}

#[derive(Default)]
pub struct EmbeddingCache {
    rows: HashMap<u64, Vec<u8>>,
    order: VecDeque<u64>,
    pub telemetry: ImageTelemetry,
}

impl EmbeddingCache {
    /// The vision tower runs: the rows are placed, the encode counted.
    pub fn place(&mut self, image: u64, rows: Vec<u8>) {
        if !self.rows.contains_key(&image) {
            self.order.push_back(image);
        }
        self.rows.insert(image, rows);
        self.telemetry.encodes += 1;
    }

    /// A placement the cache can serve: the rows come back without the
    /// vision tower, the reuse counted.
    pub fn reuse(&mut self, image: u64) -> Option<&[u8]> {
        if self.rows.contains_key(&image) {
            self.telemetry.embedding_reuses += 1;
            Some(self.rows[&image].as_slice())
        } else {
            None
        }
    }

    /// The paced reclaim: with the host short of its spare reserve, or
    /// under the OS warning, a pass takes the caches the model can rebuild
    /// first. The pre-fix drop records NOTHING - the rows are gone and
    /// /status cannot tell anyone.
    pub fn reclaim(&mut self, entries: usize) -> usize {
        let mut taken = 0;
        for _ in 0..entries {
            match self.order.pop_front() {
                Some(image) => {
                    self.rows.remove(&image);
                    taken += 1;
                }
                None => break,
            }
        }
        taken
    }
}

/// The pre-fix smoke gate: the image behind another system prompt MUST
/// come from the embedding cache - unconditionally.
pub fn check_reuse(before: ImageTelemetry, after: ImageTelemetry) -> Result<(), String> {
    if after.encodes == before.encodes && after.embedding_reuses == before.embedding_reuses + 1 {
        Ok(())
    } else {
        Err(format!(
            "a cached image re-ran the vision tower or was not reused: {:?} -> {:?}",
            before, after
        ))
    }
}

// -- equivalence: the quiet-host observable both arms share -----------------

#[test]
fn a_quiet_host_reuses_and_the_check_passes() {
    let mut c = EmbeddingCache::default();
    c.place(7, vec![1, 2, 3]);
    let before = c.telemetry;
    assert!(c.reuse(7).is_some()); // behind another system prompt
    assert_eq!(check_reuse(before, c.telemetry), Ok(()));
}

// -- the perf shape: policy and defect wear one shape ------------------------

#[test]
fn an_eviction_explained_reencode_fails_the_check_on_a_healthy_engine() {
    // the source's M5 Pro story: warning pressure for the whole smoke, a
    // paced pass takes the image's rows, the image is encoded again - as
    // that policy has it
    let mut c = EmbeddingCache::default();
    c.place(7, vec![1, 2, 3]);
    let before = c.telemetry; // (1, 0)
    assert_eq!(c.reclaim(1), 1); // the pass took the rows; NOTHING recorded
    assert!(c.reuse(7).is_none()); // the cache cannot serve it
    c.place(7, vec![1, 2, 3]); // the vision tower runs again - policy
    // THE DEFECT: a healthy engine fails the smoke
    assert!(check_reuse(before, c.telemetry).is_err());
}

#[test]
fn a_defect_dressed_the_same_is_indistinguishable() {
    // a real defect: the cache HELD the rows and the request re-ran the
    // tower anyway. The telemetry shape is (2, 0) - exactly the eviction
    // story's shape. One error message, two causes.
    let mut c = EmbeddingCache::default();
    c.place(7, vec![1, 2, 3]);
    let before = c.telemetry;
    c.place(7, vec![9, 9]); // re-encoded while its rows sat in the cache
    let defect = check_reuse(before, c.telemetry);
    // ... and the eviction story produced the same Err:
    let mut d = EmbeddingCache::default();
    d.place(7, vec![1, 2, 3]);
    let d_before = d.telemetry;
    d.reclaim(1);
    assert!(d.reuse(7).is_none());
    d.place(7, vec![1, 2, 3]);
    let eviction_story = check_reuse(d_before, d.telemetry);
    assert!(defect.is_err() && eviction_story.is_err());
    assert_eq!(d.telemetry, c.telemetry); // (2, 0) both - cannot tell apart
}"##.to_string(),
            fixed_code: r##"#![allow(dead_code)]
// Provenance: incoai/splash @ 2712e6768d5c3bf56b320b23df77ce2df7e88807
// (runtime/model/Model.hpp:668-671, runtime/model/Runtime.cpp:616-624,
// runtime/engine/Status.cpp:494-496, dev/tests/smoke_real.py:339-351),
// Apache-2.0. ORIGINAL machinery: a serving embedding cache with a reuse
// oracle; no splash code is copied. FIXED arm: the cache counts the
// entries it lets go of (past its bytes or to a reclaim) and the smoke
// gates the reuse assertion on that counter.

use std::collections::HashMap;
use std::collections::VecDeque;

/// The image telemetry /status exposes - the third counter: entries the
/// cache let go of, past its bytes or to a reclaim (Model.hpp:668-671).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImageTelemetry {
    pub encodes: u64,
    pub embedding_reuses: u64,
    pub embedding_evictions: u64,
}

#[derive(Default)]
pub struct EmbeddingCache {
    rows: HashMap<u64, Vec<u8>>,
    order: VecDeque<u64>,
    pub telemetry: ImageTelemetry,
}

impl EmbeddingCache {
    /// The vision tower runs: the rows are placed, the encode counted.
    pub fn place(&mut self, image: u64, rows: Vec<u8>) {
        if !self.rows.contains_key(&image) {
            self.order.push_back(image);
        }
        self.rows.insert(image, rows);
        self.telemetry.encodes += 1;
    }

    /// A placement the cache can serve: the rows come back without the
    /// vision tower, the reuse counted.
    pub fn reuse(&mut self, image: u64) -> Option<&[u8]> {
        if self.rows.contains_key(&image) {
            self.telemetry.embedding_reuses += 1;
            Some(self.rows[&image].as_slice())
        } else {
            None
        }
    }

    /// uncache (Runtime.cpp:616-624): drops one entry - past its bytes or
    /// to a reclaim - and COUNTS it. A paced reclaim takes k entries
    /// through the same counted drop.
    pub fn reclaim(&mut self, entries: usize) -> usize {
        let mut taken = 0;
        for _ in 0..entries {
            match self.order.pop_front() {
                Some(image) => {
                    self.rows.remove(&image);
                    self.telemetry.embedding_evictions += 1; // counted
                    taken += 1;
                }
                None => break,
            }
        }
        taken
    }
}

/// What the smoke requires of the image behind another system prompt.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// rows == (0, 1): the cache served it, the tower never ran.
    Reused,
    /// the count rose and rows == (1, 0): the rows were gone, the tower
    /// ran once - policy, not defect.
    ReencodedAfterEviction,
    /// a re-encode the count does not explain; a request that neither
    /// reused nor encoded; both at once.
    Defect,
}

/// The 2712e67 gate: `rows == (0, 1) or (evicted and rows == (1, 0))`.
/// `start_evictions` is the /status read BEFORE the red image.
pub fn adjudicate(start_evictions: u64, before: ImageTelemetry, after: ImageTelemetry) -> Verdict {
    let rows = (
        after.encodes - before.encodes,
        after.embedding_reuses - before.embedding_reuses,
    );
    let evicted = after.embedding_evictions > start_evictions;
    if rows == (0, 1) {
        Verdict::Reused
    } else if evicted && rows == (1, 0) {
        Verdict::ReencodedAfterEviction
    } else {
        Verdict::Defect
    }
}

// -- equivalence: the quiet-host observable both arms share -----------------

#[test]
fn a_quiet_host_reuses_and_the_check_passes() {
    let mut c = EmbeddingCache::default();
    c.place(7, vec![1, 2, 3]);
    let before = c.telemetry;
    assert!(c.reuse(7).is_some()); // behind another system prompt
    assert_eq!(adjudicate(0, before, c.telemetry), Verdict::Reused);
}

// -- the perf shape: the counter tells the two apart -------------------------

#[test]
fn an_eviction_explained_reencode_is_policy_not_defect() {
    let mut c = EmbeddingCache::default();
    c.place(7, vec![1, 2, 3]);
    let start = c.telemetry; // /status read before the red image: (1, 0, 0)
    let before = c.telemetry; // /status read before the reuse request
    // within the request's decode, a paced pass takes the rows...
    assert_eq!(c.reclaim(1), 1);
    assert!(c.reuse(7).is_none());
    c.place(7, vec![1, 2, 3]); // ...so the tower runs once: policy
    assert_eq!(
        adjudicate(start.embedding_evictions, before, c.telemetry),
        Verdict::ReencodedAfterEviction
    );
}

#[test]
fn the_counter_counts_the_reclaims() {
    // the oracle test's law: reclaim frees are counted like any other
    let mut c = EmbeddingCache::default();
    for i in 0..3 {
        c.place(i, vec![i as u8; 8]);
    }
    let before = c.telemetry.embedding_evictions;
    assert_eq!(c.reclaim(2), 2);
    assert_eq!(c.telemetry.embedding_evictions, before + 2);
}

#[test]
fn the_four_refusals_the_source_pins() {
    // test_smoke_real.py:602-624 - each is a SmokeFailure upstream
    let before = ImageTelemetry { encodes: 1, embedding_reuses: 0, embedding_evictions: 4 };
    let start_evictions = 4;
    // "encoded again": the tower re-ran while the cache kept its rows
    let after = ImageTelemetry { encodes: 2, embedding_reuses: 0, embedding_evictions: 4 };
    assert_eq!(adjudicate(start_evictions, before, after), Verdict::Defect);
    // "neither": no reuse and no encode
    let after = ImageTelemetry { encodes: 1, embedding_reuses: 0, embedding_evictions: 4 };
    assert_eq!(adjudicate(start_evictions, before, after), Verdict::Defect);
    // "evicted, then neither": the rows went and the request did nothing
    let after = ImageTelemetry { encodes: 1, embedding_reuses: 0, embedding_evictions: 5 };
    assert_eq!(adjudicate(start_evictions, before, after), Verdict::Defect);
    // "evicted, then both": reuse and encode at once is a contradiction
    let after = ImageTelemetry { encodes: 2, embedding_reuses: 1, embedding_evictions: 5 };
    assert_eq!(adjudicate(start_evictions, before, after), Verdict::Defect);
}

#[test]
fn a_real_defect_is_still_caught() {
    // the cache HELD the rows and the request re-ran the tower anyway,
    // with no eviction to explain it: Defect, as the pre-fix check erred
    let mut c = EmbeddingCache::default();
    c.place(7, vec![1, 2, 3]);
    let before = c.telemetry;
    c.place(7, vec![9, 9]); // re-encoded while its rows sat in the cache
    assert_eq!(adjudicate(0, before, c.telemetry), Verdict::Defect);
}"##.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (the decision content is the ORACLE law — what makes a serving cache-reuse assertion assertable — which a Rust microbench cannot measure; the M5 Pro smoke story is source-reported provenance, never a bar). SOURCE (incoai/splash @ 2712e6768d5c3bf56b320b23df77ce2df7e88807 'Count the embedding cache's evictions, and expect a re-encode only after one', Apache-2.0; clone .raw/b270/splash at the pin; every quote read back at the pin THIS batch). THE DEFECT (commit message, verbatim): 'test-http-real's image check required the red image, behind another system prompt, to come from the embedding cache.' But the cache gives memory back with the rest (verbatim): 'The cache gives memory back with the rest: with the host short of its spare reserve, or while macOS warns, each paced pass returns up to 1 GiB, the caches the model can rebuild first (Engine::reclaimMemory).' On the M5 Pro (Qwen3.8-Flash-Next UD-IQ3_XXS streaming its routed experts): 'the engine charged 52.8 GB of its 54.5 GB limit, and beside the Mac's other applications macOS reported warning pressure for the whole smoke, the host's spare memory 2.6 GB under its 4.8 GB reserve. Even where no refused allocation takes a cache, a paced pass within the red request's decode takes the vision arena and the image's rows, and the image is encoded again, as that policy has it.' — 'The check holds only where nothing took the rows, and /status could not tell the two apart.' THE LAW (commit message, verbatim): 'The runtime counts the entries the embedding cache lets go of, past its bytes or to a reclaim (ModelTelemetry::imageEmbeddingEvictions, /status images.embedding_evictions, /metrics splash_image_embedding_evictions_total). The smoke reads /status before the red image and requires the image behind another system prompt to reuse its rows unless the count rose since, and then to be encoded again, once; a re-encode the count does not explain still fails, as does a request that neither reused nor encoded. An eviction that should not have happened passes the smoke as well: the engine's tests check which reclaims take the rows.' THE COUNTER (Model.hpp:668-671, verbatim): '// Entries the embedding cache let go of, past its bytes or to a reclaim: // the image of one is encoded again where it is placed next, unless // something else still holds its rows.' + 'uint64_t imageEmbeddingEvictions = 0;'. THE COUNT SITE (Runtime.cpp:616-624, the uncache helper, verbatim header): '// Drops one entry of the embedding cache and returns the bytes it held.' — body '(*entry)->cached.reset(); embeddingCache.erase(entry); embeddingCacheBytes -= bytes; ++counters.imageEmbeddingEvictions;' (:619-622) — past-its-bytes drops AND reclaim drops flow through it, so one counter covers both (the oracle test proves the reclaim half). THE WIRE: Status.cpp:494-496 adds the field beside embedding_reuses inside the images block; server/metrics.py:148 maps 'splash_image_embedding_evictions_total' to ('images', 'embedding_evictions'). THE GATE (dev/tests/smoke_real.py at the pin, verbatim): the comment (:293-302) 'Behind another system prompt the same image comes from the embedding cache without the vision tower, unless the cache let its rows go since the image was encoded (images.embedding_evictions), as it does to give memory back, under macOS's warning for one (DEVELOPMENT.md, Admission and eviction): the image is then encoded again, once. An eviction that should not have happened passes here too: the engine tests check which reclaims take the rows.' — rows is the pair (encodes delta, reuse delta) (:339-342), evicted is 'reused[images][embedding_evictions] > start[images][embedding_evictions]' with start read BEFORE the red image (:274-275, :343-345), and the requirement (:346-351) 'rows == (0, 1) or (evicted and rows == (1, 0))' with the failure 'a cached image re-ran the vision tower while the cache kept its rows, or was not reused.' THE FIVE CASES (dev/tests/tools/test_smoke_real.py:596-624, verbatim names): the acceptance 'test_image_rows_the_cache_let_go_are_encoded_again' feeds reused={'encodes': 2, 'embedding_reuses': 0, 'embedding_evictions': 1}; the refusals in 'test_image_rows_the_cache_kept_are_reused' are 'encoded again': (2, 0, 0), 'neither': (1, 0, 0), 'evicted, then neither': (1, 0, 1), 'evicted, then both': (2, 1, 1) — each asserting a SmokeFailure matching 're-ran the vision tower'. THE RECLAIM HALF (dev/tests/engine/model_runtime_oracle_test.mm:2457-2460 + 2500-2501, verbatim): the comment 'the older entry and the newer, which the telemetry counts as evictions' and the require 'executor.telemetry().imageEmbeddingEvictions == cached.imageEmbeddingEvictions + 2' naming 'the embedding cache did not count the two entries it let go of'. MEASURED: none (source-reported smoke story only; the ISTA Q2_0 contrast: 'whose experts load whole there (44.3 GB charged, 5.1 GB of host headroom, pressure normal), kept and reused its rows'). REGIME: any smoke/real-server test asserting a CACHE HIT on a cache that shares memory with the workload (eviction is correct policy there, so the assertion needs a counter-explained escape); telemetry-driven test oracles generally. ANTI-REGIME: caches that never evict under test (the unconditional check is sound); surfaces with no telemetry (the gate cannot be built — the law says build the counter and keep the check strict, never weaken the check to stop asserting); defects indistinguishable even WITH the counter (out of scope — this law separates exactly eviction-explained from unexplained). DEDUP ADJUDICATION — the THIRD serving-cache oracle law in the corpus lineage: prompt-state-fingerprint-probe-bits-keyed-persistence (entries_20, B219, TensorFold) is an IDENTITY oracle (a startup probe's output bits key persisted learned states, so a stale state is unreachable by construction); persistent-digest-memo-stat-identity-keyed (entries_36, B256) is an identity oracle on-disk (full stat identity, corrupt index refused loud); THIS is a BEHAVIOR oracle — what makes a runtime cache-reuse CLAIM assertable: the claim holds while the eviction counter is flat, and a counter-explained re-encode is policy, not defect. Distinct from discretionary-cache-growth-keys-on-spare-not-available + kept-prompt-states (the eviction POLICY itself — that a reclaim takes the rows is entries_26's law; THIS owns making the smoke TELL THE TWO APART, they compose); distinct from prompt-cache-stale-invalidation-push-borrower-boundary (kernel_opt B249 — production invalidation semantics; THIS is the test-adjudication face). Greps 'eviction counter|reuse assertion|oracle' over both corpora: no rule gates a test assertion on an eviction counter; the oracle owners are fixture oracles (parity, hazard detectors), none telemetry-gated. NOVEL: the three-way adjudication (reuse | eviction-explained re-encode | defect) and the four refusal cells. Cite: @ 2712e6768d5c3bf56b320b23df77ce2df7e88807 — runtime/model/Model.hpp:668-671, runtime/model/Runtime.cpp:616-624, runtime/engine/Status.cpp:494-496, server/metrics.py:147-148, dev/tests/smoke_real.py:274-275+293-302+339-351, dev/tests/tools/test_smoke_real.py:559-624, dev/tests/engine/model_runtime_oracle_test.mm:2457-2460+2500-2501, dev/tests/engine/runtime_status_test.cpp:210+300-302, commit message (all quotes); Batch 270 / Plan 274 / Research 306.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: a publication for a key whose entry is CONDEMNED under a reader takes the
        //    condemned entry over (its copies given up, the reader keeping its own handle) instead
        //    of refusing — the pre-fix logic_error failed the engine and every request in it.
        PerfRule {
            rule_id: "publication-takes-over-condemned-cache-entry".to_string(),
            name: "A cache entry the engine condemned (invalidated while a reader's lease pins it) keeps its copies and its map slot while staying invisible to lookups — and a new publication for its key refused: 'both publications refused a block whose condemned entry kept a RAM copy: publishCompositeState and publishStateToDisk threw std::logic_error ('duplicate composite state key'), which fails the engine and every request in it' → a publication TAKES THE CONDEMNED ENTRY OVER: 'publicationEntry gives up its RAM copy (discardRam, which reclaim now uses to free the RAM copy of a state it wrote) beside its disk copy, publishCompositeState refuses only a valid RAM copy, and publishStateToDisk's refusal of a condemned RAM copy goes. The reader keeps its own handle to the copy it reads; its release finds the entry valid and keeps it'; lookups still never see a condemned entry ('reuseCompositeState and reuseStoredState skip a condemned entry, so the engine publishes there').".to_string(),
            category: "caching".to_string(),
            buggy_code: r##"#![allow(dead_code)]
// Provenance: incoai/splash @ 716760ddc2e00431cae7280c55da3eb036f2f8b3
// (runtime/engine/StateCache.cpp:175-177 + 252-255 + 283-286 + 588-606 +
// 655-663), Apache-2.0. ORIGINAL machinery: a tiered state cache with a
// condemned-under-reader entry and publications; no splash code is copied.
// BUGGY arm: the publication gates read state_resident alone, so a
// condemned entry's RAM copy refuses the publication - the engine fails.

use std::collections::HashMap;
use std::rc::Rc;

/// A tiered state entry. A condemned (invalidated under a reader) entry
/// keeps its copies while its lease pins it; lookups skip it.
#[derive(Default)]
pub struct Entry {
    pub ram: Option<Rc<Vec<u8>>>,
    pub disk: Option<Vec<u8>>,
    pub invalid: bool, // condemned: leaves the lookups, not the map
    pub pins: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PublishErr {
    /// A valid RAM copy holds the block: reuse it through reuse paths.
    DuplicateKey,
}

#[derive(Default)]
pub struct StateCache {
    entries: HashMap<u64, Entry>,
    pub bytes: u64,
    pub disk_bytes: u64,
    pub publications: u64,
}

impl StateCache {
    /// The block's state is reusable: the entry exists and is not
    /// condemned (StateCache.cpp:252-255).
    pub fn contains(&self, block: u64) -> bool {
        self.entries.get(&block).is_some_and(|e| !e.invalid)
    }

    /// A RAM copy exists - a condemned entry keeps its copy while its
    /// readers pin it (StateCache.cpp:283-286).
    pub fn state_resident(&self, block: u64) -> bool {
        self.entries.get(&block).is_some_and(|e| e.ram.is_some())
    }

    /// invalidateState: condemn the entry. It leaves the lookups at once;
    /// its copies stay while its lease pins it (the source's unpinned
    /// condemned entries are removed by the next sweep; the fixture holds
    /// them for the pinned scenario the law is about).
    pub fn invalidate(&mut self, block: u64) {
        if let Some(e) = self.entries.get_mut(&block) {
            e.invalid = true;
        }
    }

    /// A lookup hands the reader its OWN handle (an Rc clone) and pins
    /// the entry; a condemned entry is never found. A read of a disk-only
    /// entry promotes it (the first read lands and promotes it).
    pub fn lookup(&mut self, block: u64) -> Option<Rc<Vec<u8>>> {
        let e = self.entries.get_mut(&block)?;
        if e.invalid {
            return None;
        }
        e.pins += 1;
        let promoted = match (&e.ram, &e.disk) {
            (None, Some(d)) => Some(Rc::new(d.clone())),
            _ => None,
        };
        if let Some(r) = promoted {
            let n = r.len() as u64;
            e.ram = Some(r);
            self.bytes += n;
        }
        e.ram.clone()
    }

    /// The reader lets go: its release finds the entry and keeps it.
    pub fn release(&mut self, block: u64) {
        if let Some(e) = self.entries.get_mut(&block) {
            e.pins = e.pins.saturating_sub(1);
        }
    }

    /// The write lands: the disk copy takes the state and the RAM copy
    /// frees (the give-up the takeover shares with reclaim).
    pub fn persist_to_disk(&mut self, block: u64) {
        if let Some(e) = self.entries.get_mut(&block) {
            if let Some(ram) = e.ram.take() {
                let n = ram.len() as u64;
                self.bytes -= n;
                e.disk = Some((*ram).clone());
                self.disk_bytes += n;
            }
        }
    }

    /// The entry a new copy takes over, made when the block has none.
    /// THE DEFECT: a condemned entry gives up only its DISK copy - its
    /// RAM copy is unreachable here only because both gates refuse it.
    fn publication_entry(&mut self, block: u64) -> &mut Entry {
        if !self.entries.contains_key(&block) {
            self.entries.insert(block, Entry::default());
        }
        let e = self.entries.get_mut(&block).expect("entry just made");
        if e.invalid {
            if let Some(d) = e.disk.take() {
                self.disk_bytes -= d.len() as u64;
            }
            e.invalid = false;
        }
        e
    }

    /// publishCompositeState. THE DEFECT: the gate reads state_resident
    /// alone - a condemned entry's RAM copy (kept while its reader pins
    /// it) refuses the publication, and the logic_error fails the engine
    /// and every request in it.
    pub fn publish_composite(&mut self, block: u64, bytes: Vec<u8>) -> Result<(), PublishErr> {
        if self.state_resident(block) {
            return Err(PublishErr::DuplicateKey);
        }
        let n = bytes.len() as u64;
        let e = self.publication_entry(block);
        e.ram = Some(Rc::new(bytes));
        self.bytes += n;
        self.publications += 1;
        Ok(())
    }

    /// publishStateToDisk. THE DEFECT: the extra gate - a condemned entry
    /// is not contained, but its RAM copy stays while readers pin it, and
    /// the refusal fires.
    pub fn publish_state_to_disk(&mut self, block: u64, bytes: Vec<u8>) -> Result<(), PublishErr> {
        if self.contains(block) {
            return Err(PublishErr::DuplicateKey); // reuse through reuse paths
        }
        if self.state_resident(block) {
            return Err(PublishErr::DuplicateKey);
        }
        let n = bytes.len() as u64;
        let e = self.publication_entry(block);
        e.disk = Some(bytes);
        self.disk_bytes += n;
        self.publications += 1;
        Ok(())
    }

    pub fn entries_len(&self) -> usize {
        self.entries.len()
    }

    pub fn pins_of(&self, block: u64) -> u32 {
        self.entries.get(&block).map_or(0, |e| e.pins)
    }
}

// -- equivalence: every non-condemned path both arms share ------------------

#[test]
fn a_fresh_block_publishes_and_serves() {
    let mut c = StateCache::default();
    assert_eq!(c.publish_composite(9, vec![1, 2, 3]), Ok(()));
    assert_eq!(c.lookup(9).as_deref(), Some(&vec![1u8, 2, 3]));
    assert_eq!(c.publish_composite(9, vec![4, 5]), Err(PublishErr::DuplicateKey));
    assert_eq!(c.publish_state_to_disk(9, vec![4, 5]), Err(PublishErr::DuplicateKey));
    assert_eq!(c.entries_len(), 1);
}

#[test]
fn a_condemned_entry_is_never_found_but_keeps_its_copies() {
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    let _reader = c.lookup(5);
    c.invalidate(5);
    assert!(c.lookup(5).is_none()); // condemned: never found
    assert!(c.state_resident(5)); // its copy stays while pinned
    assert!(!c.contains(5)); // and it is not contained
    assert_eq!(c.bytes, 100); // the copy is not freed under the reader
}

#[test]
fn a_condemned_disk_only_entry_is_taken_over() {
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    c.persist_to_disk(5);
    c.invalidate(5); // condemned, no reader: no RAM copy to hold
    assert!(c.publish_composite(5, vec![9; 100]).is_ok());
    assert_eq!(c.disk_bytes, 0); // the disk copy went with the takeover
    assert_eq!(c.bytes, 100);
    assert!(c.contains(5));
}

// -- the perf shape: the engine-failing refusal ------------------------------

#[test]
fn a_publication_at_a_condemned_resident_entry_fails_the_engine() {
    // two lanes restore one state from disk: the first read lands and
    // promotes it, that lane's first command from it returns an invalid
    // result, and the second lane's lease pins the entry while its read
    // is in flight
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    c.persist_to_disk(5);
    let first = c.lookup(5);
    let reader = c.lookup(5);
    assert!(first.is_some() && reader.is_some());
    c.invalidate(5);
    drop(first); // the first lane lets go; the reader's lease pins on
    // THE DEFECT: a lane reaches the block as a state boundary (the first
    // lane's retry, which matches no condemned state) and publishes
    assert_eq!(c.publish_composite(5, vec![9; 100]), Err(PublishErr::DuplicateKey));
    assert_eq!(c.publish_state_to_disk(5, vec![9; 100]), Err(PublishErr::DuplicateKey));
    // the reader still reads its own copy, but the engine has thrown
    assert_eq!(reader.as_deref(), Some(&vec![7u8; 100]));
}"##.to_string(),
            fixed_code: r##"#![allow(dead_code)]
// Provenance: incoai/splash @ 716760ddc2e00431cae7280c55da3eb036f2f8b3
// (runtime/engine/StateCache.cpp:175-177 + 252-255 + 283-286 + 588-606 +
// 655-663), Apache-2.0. ORIGINAL machinery: a tiered state cache with a
// condemned-under-reader entry and publications; no splash code is copied.
// FIXED arm: a publication takes the condemned entry over with whatever
// copies it holds; publishCompositeState refuses only a valid RAM copy;
// the reader keeps its own handle and its release finds the entry valid.

use std::collections::HashMap;
use std::rc::Rc;

/// A tiered state entry. A condemned (invalidated under a reader) entry
/// keeps its copies while its lease pins it; lookups skip it.
#[derive(Default)]
pub struct Entry {
    pub ram: Option<Rc<Vec<u8>>>,
    pub disk: Option<Vec<u8>>,
    pub invalid: bool, // condemned: leaves the lookups, not the map
    pub pins: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PublishErr {
    /// A valid RAM copy holds the block: reuse it through reuse paths.
    DuplicateKey,
}

#[derive(Default)]
pub struct StateCache {
    entries: HashMap<u64, Entry>,
    pub bytes: u64,
    pub disk_bytes: u64,
    pub publications: u64,
}

impl StateCache {
    /// The block's state is reusable: the entry exists and is not
    /// condemned (StateCache.cpp:252-255).
    pub fn contains(&self, block: u64) -> bool {
        self.entries.get(&block).is_some_and(|e| !e.invalid)
    }

    /// A RAM copy exists - a condemned entry keeps its copy while its
    /// readers pin it (StateCache.cpp:283-286).
    pub fn state_resident(&self, block: u64) -> bool {
        self.entries.get(&block).is_some_and(|e| e.ram.is_some())
    }

    /// invalidateState: condemn the entry. It leaves the lookups at once;
    /// its copies stay while its lease pins it (the source's unpinned
    /// condemned entries are removed by the next sweep; the fixture holds
    /// them for the pinned scenario the law is about).
    pub fn invalidate(&mut self, block: u64) {
        if let Some(e) = self.entries.get_mut(&block) {
            e.invalid = true;
        }
    }

    /// A lookup hands the reader its OWN handle (an Rc clone) and pins
    /// the entry; a condemned entry is never found. A read of a disk-only
    /// entry promotes it (the first read lands and promotes it).
    pub fn lookup(&mut self, block: u64) -> Option<Rc<Vec<u8>>> {
        let e = self.entries.get_mut(&block)?;
        if e.invalid {
            return None;
        }
        e.pins += 1;
        let promoted = match (&e.ram, &e.disk) {
            (None, Some(d)) => Some(Rc::new(d.clone())),
            _ => None,
        };
        if let Some(r) = promoted {
            let n = r.len() as u64;
            e.ram = Some(r);
            self.bytes += n;
        }
        e.ram.clone()
    }

    /// The reader lets go: its release finds the entry (valid again after
    /// a takeover) and keeps it.
    pub fn release(&mut self, block: u64) {
        if let Some(e) = self.entries.get_mut(&block) {
            e.pins = e.pins.saturating_sub(1);
        }
    }

    /// The write lands: the disk copy takes the state and the RAM copy
    /// frees through the same give-up the takeover uses (discardRam,
    /// which reclaim now uses to free the RAM copy of a state it wrote).
    pub fn persist_to_disk(&mut self, block: u64) {
        if let Some(e) = self.entries.get_mut(&block) {
            if let Some(ram) = e.ram.take() {
                let n = ram.len() as u64;
                self.bytes -= n;
                e.disk = Some((*ram).clone());
                self.disk_bytes += n;
            }
        }
    }

    /// The entry a new copy takes over, made when the block has none; a
    /// condemned one gives up whatever copies it still holds; its readers
    /// keep their own handle to them (StateCache.cpp:588-606).
    fn publication_entry(&mut self, block: u64) -> &mut Entry {
        if !self.entries.contains_key(&block) {
            self.entries.insert(block, Entry::default());
        }
        let e = self.entries.get_mut(&block).expect("entry just made");
        if e.invalid {
            if let Some(r) = e.ram.take() {
                self.bytes -= r.len() as u64;
            }
            if let Some(d) = e.disk.take() {
                self.disk_bytes -= d.len() as u64;
            }
            e.invalid = false;
        }
        e
    }

    /// publishCompositeState: a valid RAM copy is reused instead (the
    /// refusal); a condemned one is taken over
    /// (StateCache.cpp:175-177).
    pub fn publish_composite(&mut self, block: u64, bytes: Vec<u8>) -> Result<(), PublishErr> {
        if self.contains(block) && self.state_resident(block) {
            return Err(PublishErr::DuplicateKey);
        }
        let n = bytes.len() as u64;
        let e = self.publication_entry(block);
        e.ram = Some(Rc::new(bytes));
        self.bytes += n;
        self.publications += 1;
        Ok(())
    }

    /// publishStateToDisk: a valid block's state is published once (reuse
    /// through reuse paths); the condemned-RAM refusal is gone
    /// (StateCache.cpp:190-201).
    pub fn publish_state_to_disk(&mut self, block: u64, bytes: Vec<u8>) -> Result<(), PublishErr> {
        if self.contains(block) {
            return Err(PublishErr::DuplicateKey);
        }
        let n = bytes.len() as u64;
        let e = self.publication_entry(block); // the condemned RAM copy goes here
        e.disk = Some(bytes);
        self.disk_bytes += n;
        self.publications += 1;
        Ok(())
    }

    pub fn entries_len(&self) -> usize {
        self.entries.len()
    }

    pub fn pins_of(&self, block: u64) -> u32 {
        self.entries.get(&block).map_or(0, |e| e.pins)
    }
}

// -- equivalence: every non-condemned path both arms share ------------------

#[test]
fn a_fresh_block_publishes_and_serves() {
    let mut c = StateCache::default();
    assert_eq!(c.publish_composite(9, vec![1, 2, 3]), Ok(()));
    assert_eq!(c.lookup(9).as_deref(), Some(&vec![1u8, 2, 3]));
    assert_eq!(c.publish_composite(9, vec![4, 5]), Err(PublishErr::DuplicateKey));
    assert_eq!(c.publish_state_to_disk(9, vec![4, 5]), Err(PublishErr::DuplicateKey));
    assert_eq!(c.entries_len(), 1);
}

#[test]
fn a_condemned_entry_is_never_found_but_keeps_its_copies() {
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    let _reader = c.lookup(5);
    c.invalidate(5);
    assert!(c.lookup(5).is_none()); // condemned: never found
    assert!(c.state_resident(5)); // its copy stays while pinned
    assert!(!c.contains(5)); // and it is not contained
    assert_eq!(c.bytes, 100); // the copy is not freed under the reader
}

#[test]
fn a_condemned_disk_only_entry_is_taken_over() {
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    c.persist_to_disk(5);
    c.invalidate(5); // condemned, no reader: no RAM copy to hold
    assert!(c.publish_composite(5, vec![9; 100]).is_ok());
    assert_eq!(c.disk_bytes, 0); // the disk copy went with the takeover
    assert_eq!(c.bytes, 100);
    assert!(c.contains(5));
}

// -- the perf shape: the takeover, and the reader who outlives it ------------

#[test]
fn a_publication_takes_the_condemned_entry_over_and_outlives_the_reader() {
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    c.persist_to_disk(5);
    let first = c.lookup(5); // two lanes restore it from disk
    let reader = c.lookup(5); // the first read lands and promotes it
    assert!(first.is_some() && reader.is_some());
    c.invalidate(5); // that lane's command from it is not finite
    drop(first); // the first lane lets go; its lease releases
    c.release(5); // the reader's lease pins on
    // condemned: never found, its copies stay under the reader's lease
    assert!(c.lookup(5).is_none());
    assert!(c.state_resident(5) && !c.contains(5));
    assert_eq!(c.bytes, 100);
    assert_eq!(c.disk_bytes, 100);
    // a publication takes the entry over, its copies going
    c.publish_composite(5, vec![9; 100]).unwrap();
    assert_eq!(c.bytes, 100); // the OLD ram copy's bytes went with it
    // the reader keeps its own handle to the copy it read
    assert_eq!(reader.as_deref(), Some(&vec![7u8; 100]));
    // its release finds the entry valid and keeps it
    c.release(5);
    assert_eq!(c.pins_of(5), 0);
    assert!(c.contains(5));
    // a fresh lookup serves the NEW publication
    assert_eq!(c.lookup(5).as_deref(), Some(&vec![9u8; 100]));
    assert_eq!(c.entries_len(), 1);
}

#[test]
fn a_disk_publication_takes_the_condemned_ram_copy_over() {
    let mut c = StateCache::default();
    c.publish_composite(5, vec![7; 100]).unwrap();
    c.persist_to_disk(5);
    let reader = c.lookup(5);
    c.invalidate(5);
    // the pre-fix refusal of a condemned RAM copy is gone
    c.publish_state_to_disk(5, vec![9; 100]).unwrap();
    assert_eq!(c.bytes, 0); // the condemned RAM copy given up
    assert_eq!(c.disk_bytes, 100); // the new disk copy landed
    assert_eq!(reader.as_deref(), Some(&vec![7u8; 100])); // its own handle
    c.release(5);
    assert_eq!(c.pins_of(5), 0);
    assert!(c.contains(5));
    // a fresh lookup promotes the NEW disk copy
    assert_eq!(c.lookup(5).as_deref(), Some(&vec![9u8; 100]));
}"##.to_string(),
            context_notes: Some("Grade B+ — NotBenchable ON THIS BOX (the decision content is a LIFETIME/takeover contract — a condemned entry's copies and its readers' handles — red-before-green injection-tested upstream with 'engine-cache, cache-property and persistent-cache pass, also under ASan and UBSan'; no perf numbers shipped, none applicable). SOURCE (incoai/splash @ 716760ddc2e00431cae7280c55da3eb036f2f8b3 'Let a publication take over a state invalidated under a reader', Apache-2.0; clone .raw/b270/splash at the pin; every quote read back at the pin THIS batch). THE MECHANISM (commit message, verbatim): 'A state the engine invalidates (Cache::invalidateState, when the first command a lane runs from the state it restored returns an invalid result; poisonCopy) leaves once no lease pins it. Until then its entry stays, condemned, with its copies.' — the two predicates split (StateCache.cpp:252-255 + :283-286): contains() is 'found != entries_.end() && !found->second.invalid' (a condemned entry is NOT contained) while stateResident() is 'found != entries_.end() && found->second.ram != nullptr' (its RAM copy still reads resident). THE DEFECT (commit message, verbatim): 'publicationEntry was written to take such an entry over, but gave up only a disk copy, and both publications refused a block whose condemned entry kept a RAM copy: publishCompositeState and publishStateToDisk threw std::logic_error ('duplicate composite state key'), which fails the engine and every request in it. reuseCompositeState and reuseStoredState skip a condemned entry, so the engine publishes there.' The reproduction (verbatim): 'A condemned entry keeps a RAM copy when two lanes restore one state from disk: the first read lands and promotes it, that lane's first command from it returns an invalid result, and the second lane's lease pins the entry while its read is in flight. A lane that reaches the block as a state boundary meanwhile, such as the first lane's retry, which matches no condemned state and prefills through the block, failed the engine.' THE LAW (commit message, verbatim): 'A publication now takes the condemned entry over with whatever copies it holds: publicationEntry gives up its RAM copy (discardRam, which reclaim now uses to free the RAM copy of a state it wrote) beside its disk copy, publishCompositeState refuses only a valid RAM copy, and publishStateToDisk's refusal of a condemned RAM copy goes. The reader keeps its own handle to the copy it reads; its release finds the entry valid and keeps it.' THE CODE, three sites: (1) the RAM-publication gate (StateCache.cpp:175-177, verbatim): '// A valid RAM copy is reused instead; a condemned one is taken over.' over 'if (contains(kvBlock) && stateResident(kvBlock)) throw std::logic_error('duplicate composite state key');' (was the bare 'if (stateResident(kvBlock))'); (2) the disk publication's condemned-RAM refusal DELETED (the removed lines, verbatim): '// A condemned entry is not contained, but its RAM copy stays while readers // pin it.' + its throw; (3) the takeover (StateCache.cpp:588-606): the doc 'The entry a new copy takes over, made when the block has none; a condemned one gives up the copies it still holds.' (StateCache.hpp:316-320) with the body comment '// A condemned entry gives up whatever copies it still holds; its readers // keep their own handle to them.' over 'if (entry.invalid) { discardRam(entry); discardDisk(entry); entry.invalid = false; }'. THE SHARED HELPER (StateCache.cpp:655-663, verbatim): 'void StateCache::discardRam(Entry &target) noexcept { if (!target.ram) return; const uint64_t released = target.ram->bytes(); bytes_ -= released; if (target.checkpoint) checkpointBytes_ -= released; target.ram.reset(); }' — reclaim frees through it too (:379, '// A write reads its own copy, so the RAM copy is free at once.'), so the byte accounting has ONE owner. THE TEST (dev/tests/engine/engine_cache_test.cpp:1617-1655, testInvalidatedStateUnderALookup, verbatim): the header 'A state invalidated while a lookup holds it keeps its copies until the reader lets go: here two lanes restore it from disk, the first read promotes it, and that lane's step from it is not finite. A publication there meanwhile, in RAM or on disk, takes the entry over, its copies going, and outlives the reader.' — the mid-assertion 'the invalidated state did not stay under its reader, or matched a lookup' pins entries == 1, pinned == 1, bytes == 100 AND diskBytes == 100 (the copies stay) with a lookup missing; the final 'the publication did not take over the invalidated state' pins entries == 1, pinned == 0, the bytes split by tier, and a lookup FINDING the new publication. DRIFT (staged framing, disclosed): the staged 'rather than allocating fresh while the condemned one drains' does not match the pin — the source has no allocate-fresh alternative; the slot is REUSED with its copies discarded, and the pre-fix failure was a REFUSAL (logic_error), never a duplicate allocation. The law as landed: a publication takes over the condemned entry; a valid RAM copy still refuses (reuse goes through reuseStoredState). REGIME: shared caches whose entries can be invalidated while a reader still holds a handle (lease/pin + shared copies) and whose write path can publish the same key again (state stores, session caches, snapshot stores with poison-and-republish flows). ANTI-REGIME: single-owner entries (no readers to outlive); keys that can never be republished (the takeover path is dead); caches where condemn implies immediate physical free (no copies to hold). DEDUP ADJUDICATION: distinct from pinned-entry-tombstone-freed-at-last-unpin (entries_36, B256, TensorFold @ 8c6b40a3+3aedf373 — the SAME condemned-under-reader premise, opposite axis: THAT owns the REMOVAL half, dead-to-lookups + the last unpin frees; THIS owns the REPUBLication half — the condemned slot is the new publication's landing, the reader keeps reading the OLD copy through its own handle, and release finds the entry VALID again; they compose: the tombstone makes the old copy invisible and unfreeable, the takeover reuses its slot); distinct from slru-probation-protected-promote-on-second-touch (entries_35, B255 — eviction policy over LIVE entries; a condemned entry is not an eviction candidate, it is a takeover target); distinct from empty-slot-first-victim-order-save-before-overwrite (entries_28, B241 — save-before-overwrite of a LIVE state; THIS overwrites a DEAD one and owes no save: the copies are the condemned state's own, given up); distinct from seq-fork-carries-pending-rollback-record (entries_32, B246 — pending mutations travel WITH a copy; no publication face); distinct from prompt-cache-stale-invalidation-push-borrower-boundary (kernel_opt B249 slice 2 — push invalidation marks expired at the owner and collects at host boundaries; the expired entry is never republished in place, no takeover). Greps 'condemn|invalid entry|takeover|take over|poison|duplicate composite state key' over both corpora: zero owners of the publication-takes-over-condemned-entry face; the poison hits are prose. NOVEL: the reader-keeps-its-handle-across-takeover + release-finds-the-entry-valid pair. Cite: @ 716760ddc2e00431cae7280c55da3eb036f2f8b3 — runtime/engine/StateCache.cpp:175-177+190-201+252-255+283-286+356-382+588-606+655-663, runtime/engine/StateCache.hpp:288-294+316-320, dev/tests/engine/engine_cache_test.cpp:1617-1655, commit message (all quotes); Batch 270 / Plan 274 / Research 306.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

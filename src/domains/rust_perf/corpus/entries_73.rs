//! Batch 355 — the `SGLang` unified-memory `rust_perf` transcription batch
//! (Plan 365 / Research 399): the host-side memory laws behind the one-arena
//! hybrid-state design — co-lifecycle packed entries with strided views (the
//! BENCHABLE GOAT lead), the logical-free/physical-reuse-waits-settle page
//! lifecycle, joint-demand admission over coupled pools, and the boot-ratio
//! sizing-target law.
//!
//! Source: the `LMSYS` blog 'Unified Memory in `SGLang`: One Arena for Every Token
//! Lifetime" (2026-11-11) + `sgl-project/sglang` @ `daa171846f` (the blog's
//! results pin; Apache-2.0; cloned at T0, removed at close — the code-level
//! shapes recorded in-rule). Per the transcription law the corpus consumes
//! DECISION CONTENT with ORIGINAL Rust fixtures
//! (`corpus/fixtures/b355_fixtures.rs`, 4/4 standalone-compiled AND executed
//! before entry), never the blog's Python or their KV structures verbatim.
//! UPSTREAM: present at pin (their landed feature — decision content only,
//! never an outbound-PR lead; the P113 check resolves trivially). Batch 355
//! / Plan 365 / Research 399.

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

#[allow(clippy::too_many_lines)] // the corpus entries ARE the payload
pub fn entries() -> Vec<crate::domains::rust_perf::PerfRule> {
    vec![
        // ── R1 — the GOAT lead: N row-groups with IDENTICAL lifetimes but
        // heterogeneous shapes pack into ONE allocation; per-group strided
        // views preserve every consumer's shape; one alloc/free/move covers
        // all. ─────────────────────────────────────────────────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "co-lifecycle-rows-pack-one-entry-strided-views".to_string(),
            name: "row groups with IDENTICAL lifetimes but heterogeneous shapes pay N allocations, N frees and N move-copies as parallel buffers — pack ONE entry, stride the views".to_string(),
            category: "layout".to_string(),
            buggy_code: r#"// THE BUGGY ARM: one buffer per row-group. The groups' lifetimes are
// identical (every token's rows live and die together) but their shapes
// differ (rows x row_len per group), so each gets its own Vec — N
// allocations, N frees, and a move or growth pays N copies.
fn alloc_groups(geo: &[(usize, usize)]) -> Vec<Vec<u8>> {
    geo.iter()
        .map(|(rows, row_len)| vec![0u8; rows * row_len])
        .collect()
}

fn move_all(bufs: &mut [Vec<u8>]) {
    for b in bufs.iter_mut() {
        // one copy per group, N dispatches
        *b = std::mem::take(b);
    }
}"#.to_string(),
            fixed_code: r#"// THE REMEDY: ONE packed entry — every group back-to-back (word-aligned
// between groups) in a single allocation; a per-group view is
// (offset, rows, row_len) striding over the entry, so every consumer
// keeps its exact (rows, row_len) shape. One alloc, one free, one
// move-copy covers all groups. SHAPE GATE: pack only groups with
// IDENTICAL lifetimes — freeing one group of a packed entry keeps the
// whole entry alive, so heterogeneous lifetimes refuse the pack.
// ANTI-REGIME: packing that would force a readback or break a consumer
// layout (the split-decay pole — see the adjudication); independent
// growth per group.
#[derive(Clone, Copy)]
struct GroupView {
    offset: usize,
    rows: usize,
    row_len: usize,
}

struct PackedEntry {
    data: Vec<u8>, // one allocation
    views: Vec<GroupView>,
}

fn pack_groups(geo: &[(usize, usize)]) -> PackedEntry {
    let mut views = Vec::with_capacity(geo.len());
    let mut off = 0usize;
    for &(rows, row_len) in geo {
        views.push(GroupView { offset: off, rows, row_len });
        off += rows * row_len;
        off = off.div_ceil(8) * 8; // word-align the next group
    }
    PackedEntry { data: vec![0u8; off], views } // ONE allocation
}

impl PackedEntry {
    fn view(&self, g: usize) -> (&[u8], usize, usize) {
        let v = &self.views[g];
        (&self.data[v.offset..v.offset + v.rows * v.row_len], v.rows, v.row_len)
    }
}"#.to_string(),
            context_notes: Some("MEASURED ParetoDominant-on-allocs (rust_perf_bench validator, amo, 2026-11-13): alloc axis 2N->1 EXACT at every size (the parallel arm allocates one buffer per group, the packed arm exactly one; allocs 5->2 incl. the harness at N=4, 9->2 at N=8, 17->2 at N=16), time ~neutral (sub-us at every size, within noise). UPSTREAM: present at pin (sgl-project/sglang @ daa171846f — their landed unified-memory feature; decision content only). Blog: 'Each token's KV rows across all layers are packed into a single entry, allowing them to be allocated, freed, and moved together. Rows of different shapes and dtypes can share an entry, so speculative draft KV can use the same allocation and lifecycle as target KV even when their dtypes differ' + 'a layer's view strides over the entries'. Code shapes at the pin: python/sglang/srt/mem_cache/kv/memory/arena.py:1-9 ('One uint8 region holds every sub-pool of a plan as a band of whole entries ... A band is moved and reset by unit') + move_units (:215) = ONE bulk move per band; plan.py:1-5 ('Parts with the same lifetime and allocation unit share a sub-pool') = the co-lifecycle shape gate in their words; kv/memory/views.py (KVLayoutView/MHAKVView/MLAKVView) = the strided views preserving consumer shapes. MANDATORY ADJUDICATION BOTH DIRECTIONS vs kernel_opt split-decay-outputs-eliminate-readback: THAT rule owns the case where packing is a PESSIMIZATION — consumers reading different shapes off a packed buffer force a device->host->device readback; THIS rule REQUIRES the views to preserve every consumer's shape (the pack is shape-transparent) and wins on LIFECYCLE ops (one alloc/free/move), never on read avoidance — when packing would force a readback or break a consumer shape, split-decay wins and that is THIS rule's anti-regime. Distinct-from combined-kv-buffer-keys-then-values (two same-shape halves behind one binding — the descriptor-pressure axis; THIS is N heterogeneous groups, lifecycle-amortization), from carry-dual-layout-dead-slot (one producer parameterized to the consumer's layout — the layout-adoption axis, not packing), from partial-reset-watermark-pool (pool reset-COST axis, no packing), and from launch-cost-scales-with-allocation-count-pack-one-arena (entries_36 — the DISPATCH-COUNT axis: N launches vs one; THIS is the ALLOCATION/lifecycle axis: N allocs/frees/moves vs one; both are 'pack one arena' laws over different per-unit costs, compose in a serving loop). Instances recorded, never separate rules: the DSA-indexer + MLA-KV fused entry; the draft-KV/target-entry lane (speculative draft KV rides the target entry across dtypes). BENCHABLE: the alloc axis is exact and deterministic (2N->1); the fixture proves byte-identical view indexing including empty groups and odd (unaligned) group lengths. Batch 355 / Plan 365 / Research 399.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(pack_one_entry_bench_spec()),
        },
        // ── R2 — the deferred-reuse/epoch-lifecycle law: logical free is
        // immediate; physical reuse waits for the settle. ──────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "logical-free-immediate-physical-reuse-waits-settle".to_string(),
            name: "an overlapped-execution arena frees pages an in-flight reader may still read — logical free is immediate, physical reuse waits for the settle; a move with an old reader copies + remaps (only SOURCE reuse waits)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// THE BUGGY ARM: freeing a slot makes it immediately reusable. Under
// overlapped execution (schedule stream + compute stream, graph replay,
// an async pipeline) an in-flight reader may still hold the old bytes —
// the new occupant and the old reader share one slot and the read is
// torn or stale.
struct NaiveArena {
    free: Vec<usize>,
}

impl NaiveArena {
    fn free_slot(&mut self, slot: usize) {
        self.free.push(slot); // reusable NOW
    }
    fn alloc(&mut self) -> Option<usize> {
        self.free.pop()
    }
}"#.to_string(),
            fixed_code: r#"// THE REMEDY: a pending set drained by a settle. Logical free is
// IMMEDIATE — capacity counts the freed slot at once (the admission
// math stays honest) — but the slot joins PENDING; physical reuse picks
// only SETTLED slots (the alloc gate refuses pending). A MOVE with an
// old reader does not wait: copy to a fresh slot + remap the handle
// (only SOURCE reuse waits for the settle). A move with an old WRITER
// orders the drain BEFORE the copy (the writer's bytes must be durable
// before anything reuses the source).
#[derive(Clone, Copy, PartialEq)]
enum SlotState {
    Live,
    Pending, // logically free; an old reader may hold the bytes
    Settled, // physical reuse safe
}

struct Arena {
    state: Vec<SlotState>,
    readers: Vec<usize>,
}

impl Arena {
    /// Logical free is immediate: capacity counts it at once.
    fn logical_free(&mut self, slot: usize) {
        self.state[slot] = SlotState::Pending;
    }
    fn capacity(&self) -> usize {
        self.state.iter().filter(|s| **s != SlotState::Live).count()
    }
    /// The settle drain: old readers quiesced, pending -> settled.
    fn settle(&mut self) -> usize {
        let mut settled = 0;
        for s in 0..self.state.len() {
            if self.state[s] == SlotState::Pending && self.readers[s] == 0 {
                self.state[s] = SlotState::Settled;
                settled += 1;
            }
        }
        settled
    }
    /// Physical allocation refuses PENDING slots.
    fn alloc(&mut self) -> Option<usize> {
        let s = self.state.iter().position(|st| *st == SlotState::Settled)?;
        self.state[s] = SlotState::Live;
        Some(s)
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (the capacity-under-overlap metric lives in a serving runtime no validator hosts — the dial9/Batch-136 annotation, not a demotion). UPSTREAM: present at pin (sgl-project/sglang @ daa171846f — decision content only). Blog: 'logical free is immediate; physical reuse waits' + the three-case table (free-with-old-reader; move-with-old-reader = copy + remap, no wait, only source REUSE waits; move-with-old-writer = ordered BEFORE the copy) + the pending/vacated/fenced states. Code shapes at the pin: python/sglang/srt/mem_cache/kv/alloc/page_usage.py (the PageUse ledger: pending_pages set :260/:425, the settle drain difference_update :506, hazards() 'What a fence (a vacated byte range) still guards against' :510) + kv/alloc/band.py:273 settle_pending_for_recovery ('Each band with pending pages settles all of them') + kv/memory/host_backing.py:308 _logical_free_page_count ('sum(end - start for start, end in side.free_logical_extents)') + :352 _can_fit_packed reading logical free. FAMILY SEED: the deferred-reuse/epoch-lifecycle concurrency family — 0 corpus owners before this rule (the greps: epoch/logical-free/pending-settle/deferred-reuse/in-flight-reader hits are papaya cost annotations, barrier batching and holdback, never the law); the famous-lib pool is crossbeam-epoch, and our own house map (papaya) is epoch-based, so the law fires at home conceptually. Distinct-from vanished-entry-tolerant-eviction-panic-else (peer-eviction tolerance, not deferred reuse), from slow-consumer-latest-value-single-slot (coalescing), from deferred-command-flush-hoist-out-of-batch-loop (barrier batching), from partial-reset-watermark-pool (the pool reset-COST axis — and THIS rule's anti-regime: a single-threaded immediate-reuse arena is partial-reset's world, the pending machinery is pure overhead there). Lane feed: riir-infer 742 T4's CUDA-graph-safe cache ops are the graph-replay instance — a page move under replay is exactly 'move with an old reader'. Batch 355 / Plan 365 / Research 399.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R3 — the joint-demand admission law: coupled pools admit on the
        // rounded joint question; per-pool free-byte reads are logs-only. ──
        crate::domains::rust_perf::PerfRule {
            rule_id: "joint-demand-admission-single-pool-free-bytes-logs-only".to_string(),
            name: "multi-pool runtimes with COUPLED demands (one token count, page-rounded per pool) must admit on the joint question — a per-pool free-byte read on a decision path admits demands the rounded joint refuses".to_string(),
            category: "measurement".to_string(),
            buggy_code: r#"// THE BUGGY ARM: the admission path reads per-pool free-BYTE counts.
// Page granularity makes the byte reads lie: a 70-token demand needs
// ceil(70/64) = 2 pages = 128 bytes from pool A, but 96 free BYTES
// 'cover' 70 tokens in an unrounded comparison — the read admits, the
// runtime OOMs.
struct Pools {
    free_bytes_a: usize,
    free_bytes_b: usize,
}

impl Pools {
    fn admits_naive(&self, tokens: usize) -> bool {
        // per-pool byte reads on the DECISION path
        self.free_bytes_a >= tokens && self.free_bytes_b >= tokens
    }
}"#.to_string(),
            fixed_code: r#"// THE REMEDY: one joint question over the COUPLED, page-ROUNDED demand —
// every pool must fit its rounded page count (page rounding is what
// couples the pools: the same token count rounds per pool). Per-pool
// free-byte counts exist for LOGS and METRICS only; a check mode flags
// any decision that reads them. Eviction walks count what each victim
// releases in its own pool — never a global byte pool.
struct Pools {
    free_bytes_a: usize,
    free_bytes_b: usize,
    page_a: usize,
    page_b: usize,
    check_mode: bool,
}

impl Pools {
    fn admits(&self, tokens: usize) -> bool {
        let pages_a = tokens.div_ceil(self.page_a);
        let pages_b = tokens.div_ceil(self.page_b);
        self.free_bytes_a >= pages_a * self.page_a && self.free_bytes_b >= pages_b * self.page_b
    }
    /// The logs-only carve-out, enforced: a free-byte read outside
    /// logging/metrics is flagged in check mode.
    fn free_bytes_for_logs(&self, pool: usize) -> usize {
        debug_assert!(!self.check_mode, "free_bytes read on a decision path");
        if pool == 0 { self.free_bytes_a } else { self.free_bytes_b }
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (serving-admission integrity; no validator hosts a multi-pool KV runtime — the dial9/Batch-136 annotation). UPSTREAM: present at pin (sgl-project/sglang @ daa171846f — decision content only). Blog: 'A free-byte count for a single sub-pool exists only for logs and metrics, and a check mode flags any decision that reads it' + the 512-full-KV-pages + 64-window-pages worked example + eviction walks that 'count what that victim releases in the other sub-pools'. Code shapes at the pin: python/sglang/srt/mem_cache/kv/alloc/chain.py:887 admits(demand: KVDemand) — 'admission must name every window' (a per-pool question is a ValueError) + _admission_units :927 rounding every pool's share of ONE demand (-(-tokens // page) per sub-pool) + state_slots joined into the same question; kv/build.py:339 the 'joint_available=%d' log line (the free count exists at the LOG site). Distinct-from prefault-admission-keyed-to-ram-headroom (B139 — ONE resource vs TOTAL host RAM), from batch-load-admission-sized-from-files-refuses-past-the-hard-budget (B274 — one hard budget sized from files), from reserve-then-commit-capacity-accounting (B324 — reserve/commit WITHIN one pool): every prior admission rule is single-resource/single-pool; THIS is the CROSS-POOL coupling axis (page rounding + eviction release counting across coupled pools on one decision path). ANTI-REGIME: genuinely independent resources — single-resource admission is correct and the joint question degenerates to one rounded read. Batch 355 / Plan 365 / Research 399.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R4 — the boot-ratio sizing-target law: a fixed-ratio partition
        // strands demand when the mix shifts; the ratio is a boot-sizing
        // target, never a run-time wall. ──────────────────────────────────
        crate::domains::rust_perf::PerfRule {
            rule_id: "fixed-ratio-pool-partition-strands-demand".to_string(),
            name: "a fixed-ratio partition over coupled pools strands capacity when the demand mix shifts — the ratio is a boot SIZING target, never a run-time wall; run-time capacity stays fungible".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"// THE BUGGY ARM: the boot guess (demand mix 4:1) hard-partitions the
// arena — and the partition becomes a run-time WALL. When the mix
// shifts, one pool refuses while the other strands bytes behind it.
struct Partitioned {
    pool_a: usize, // 320 of 400 — the 4:1 boot guess, now a wall
    pool_b: usize, // 80
}

impl Partitioned {
    fn admits(&self, need_a: usize, need_b: usize) -> bool {
        // the partition is the refusal boundary
        need_a <= self.pool_a && need_b <= self.pool_b
    }
}"#.to_string(),
            fixed_code: r#"// THE REMEDY: one fungible arena — run-time capacity is the TOTAL, not
// the boot split; the boot ratio sets the initial WATERMARKS (sizing
// targets), never a refusal. A shifted mix admits as long as the joint
// demand fits the whole.
struct Fungible {
    total: usize,      // the arena
    watermark_a: usize, // the boot 4:1 guess as a TARGET, advisory
    watermark_b: usize,
}

impl Fungible {
    fn admits(&self, need_a: usize, need_b: usize) -> bool {
        need_a + need_b <= self.total // the joint question; no walls
    }
}"#.to_string(),
            context_notes: Some("NotBenchable-by-domain (throughput-under-shifted-mix lives in a serving runtime — the dial9/Batch-136 annotation). UPSTREAM: present at pin (sgl-project/sglang @ daa171846f — decision content only). Blog evidence: wrong ratios cost 12%/14% throughput, cap concurrency 24 vs 51, and strand bytes behind a startup guess ('unified memory ran about 1,000 requests at once where separate pools at the default ratio ran 618'); the ratio flags 'remain only as boot-sizing targets, not walls at run time'. Code shapes at the pin: python/sglang/srt/model_executor/pool_configurator.py — the SWA pool sized from --swa-full-tokens-ratio (:746/:921/:959) vs SWARequestCapPoolConfigurator (:1050 'sized from the explicit max_running_requests worst case instead of swa_full_tokens_ratio; the rest goes to the full pool') = the ratio-as-target posture landing as an explicit configurator; kv/memory/arena.py:1-9 the ONE region holding every sub-pool as bands; kv/plan.py:1-5 the sub-pool chain ordered by lifetime rank. Distinct-from unified-memory-page-cache-is-device-allocatable (B207 — host page cache vs device alloc ACROSS MEMORIES, the Apple-hardware axis; THIS is within-device pool partitioning) and from prefault-admission-keyed-to-ram-headroom (B139 — headroom admission). ANTI-REGIME: genuinely independent resources where the partition is exact (a stable mix at the boot guess never strands — the wall never binds); single-resource runtimes. SUGGESTION-ONLY (the arena redesign is architecture-adjacent — the bounded decision a reviewer applies is 'the boot ratio informs watermarks, never a refusal'). Batch 355 / Plan 365 / Research 399.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// The R1 bench spec: the alloc axis is exact (2N->1); time ~neutral.
#[cfg(feature = "rust_perf_bench")]
fn pack_one_entry_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![4, 8, 16],
        harness: r#"
pub struct Input {
    geo: Vec<(usize, usize)>,
    expected_bytes: usize,
}

pub fn make_input(n: usize) -> Input {
    // n row-groups, heterogeneous shapes, identical lifetimes.
    let mut geo = Vec::with_capacity(n);
    let mut total = 0usize;
    for g in 0..n {
        let rows = 2 + (g % 3);
        let row_len = 3 + (g % 7); // odd lengths included
        total += rows * row_len;
        geo.push((rows, row_len));
    }
    Input { geo, expected_bytes: total }
}

// BUGGY: one buffer per group.
pub fn run_buggy(input: &Input) -> usize {
    let bufs: Vec<Vec<u8>> = input
        .geo
        .iter()
        .map(|(rows, row_len)| vec![0u8; rows * row_len])
        .collect();
    bufs.iter().map(|b| b.len()).sum()
}

#[derive(Clone, Copy)]
struct View {
    offset: usize,
    len: usize,
}

// FIXED: one packed entry; views stride over it (word-aligned between groups).
pub fn run_fixed(input: &Input) -> usize {
    let mut views = Vec::with_capacity(input.geo.len());
    let mut off = 0usize;
    for &(rows, row_len) in &input.geo {
        views.push(View {
            offset: off,
            len: rows * row_len,
        });
        off += rows * row_len;
        off = off.div_ceil(8) * 8;
    }
    let entry = vec![0u8; off]; // ONE allocation
    let mut sum = 0usize;
    for v in &views {
        let _ = &entry[v.offset..v.offset + v.len];
        sum += v.len;
    }
    sum
}

pub fn expected(input: &Input) -> usize {
    input.expected_bytes
}
"#
        .to_string(),
    }
}

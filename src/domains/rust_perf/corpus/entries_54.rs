//! Entries slice - Batch 308 (2026-10-30): the 10-29b staged serving
//! pool + `rust_perf` row (Plan 313 / Research 352). TWO `rust_perf`
//! rules, both ANNOTATED `NotBenchable` `bench_spec` None:
//!
//! - `tier-hit-refreshes-lru-recency-at-owning-tier` (caching,
//!   vllm-project/vllm @ `c1b7dffa1457` #62444, Apache-2.0) - the
//!   CROSS-TIER recency-coherence law: a hit served by a faster tier
//!   must still refresh the OWNING tier's recency, or the owning
//!   tier's LRU evicts in the wrong order. The corpus's
//!   touch/promote family (B255 `slru-probation-protected-promote-on-
//!   second-touch`, `entries_35`) is within-one-cache; this is
//!   coherence BETWEEN tiers.
//! - `refusal-quarantines-entry-transient-passes` (caching,
//!   ashhart/tensorfold @ `d1fcb9d93458` #1539, Apache-2.0) - a copy
//!   the receiving family REFUSES (words that differ, a size or a
//!   range that does not fit its blank) is a failed check of the
//!   ENTRY: quarantine it (counted, logged with the file named,
//!   unlinked once, every naming state dropped, the read-ahead entry
//!   taken along) while TRANSIENT errors pass through keeping the
//!   state and its read so retry works. The tombstone/quarantine
//!   cluster (B242/B256/B257) owns lifetime/pin axes; the refusal-
//!   VERDICT clause + the forget-ordering clause were unowned.
//!
//! Count-neutral rider faces landed the same commit: the REMOTE-writer
//! arm on `drain-device-work-before-free` (`kernel_opt` `entries_gpu_111`,
//! vllm #64256 `16777da` - a timed-out P2P load fails at once but its
//! chunks stay out of the free pool until the peer's `AbortAck`) and the
//! CONSUMING-SIZE face on `admission-tests-the-bound-the-admitted-
//! mode-serves` (`entries_45`, llama.cpp #31350 `7df8b87` - the
//! admission tested the derived rollback length where the guarantee is
//! scoped to the consuming size).
//!
//! Every quote is verbatim at the pin (fetched as per-commit .patch at
//! the PINNED SHAs - the #62444/#64256/#31350 PR-bundle SHAs moved vs
//! the staged pins, rebases; the per-commit patches are the
//! verification surface). Fixtures are ORIGINAL machinery (never the
//! source pasted), each compiled and executed standalone (rustc
//! --edition 2021 --test) before this table was assembled; the
//! buggy/fixed arm pair shares the surrounding harness and differs
//! only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── caching: a fast-tier hit must refresh the OWNING tier's recency,
        // or the owning tier's LRU corrupts -------------------------
        // Batch 308 / vllm #62444 @ c1b7dffa1457 (the eager
        // SimpleCPUOffload path: the pending map + the serving-step
        // touch/re-free + the terminal drains).
        PerfRule {
            rule_id: "tier-hit-refreshes-lru-recency-at-owning-tier".to_string(),
            name: "a hit served by a FASTER tier must still refresh the OWNING tier's recency - when a request's prefix hits a tier above the one that owns its copy, and the owning tier's write path only records recency at store time, the owning tier's LRU never sees the touch and evicts in the wrong order (its live, just-used entries evicted before truly cold ones); record the hit at ADMISSION into a pending map (a request waiting for blocks repeats its lookup every step, so one record replaces per-step touches), at the serving step move the owning tier's hit blocks to the MRU end (touch, then re-free there - a cached copy stays free, its RECENCY moves), and drain the pending map on every terminal path (request finished + reset) so a stale record cannot leak into a reused request id".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `tier-hit-refreshes-lru-recency-at-owning-tier`
// (riir-refine rust_perf corpus, Batch 308) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's eager CPU-offload
// serving path (vllm SimpleCPUOffload is cited in context_notes): a
// two-tier block cache. The SLOW tier owns a copy of every stored
// prefix (an LRU-ordered block pool); the FAST tier serves hits. In
// eager mode the fast tier's stores write the owning tier once at
// store time and never again, so the ONLY thing keeping the owning
// tier's recency fresh is the store path -- a lookup that hits on
// the fast tier never tells the owning tier.

use std::collections::HashSet;
use std::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BlockHash(u64);

struct CpuPool {
    capacity: usize,
    // Recency order: front = coldest (evicted first), back = hottest.
    order: Vec<BlockHash>,
}

impl CpuPool {
    fn new(capacity: usize) -> Self {
        Self { capacity, order: Vec::new() }
    }

    fn holds(&self, blk: BlockHash) -> bool {
        self.order.contains(&blk)
    }

    fn evict_coldest(&mut self) -> Option<BlockHash> {
        if self.order.is_empty() { None } else { Some(self.order.remove(0)) }
    }
}

struct TwoTierCache {
    gpu: HashSet<BlockHash>, // the fast tier: prefix hits answer here
    cpu: CpuPool,            // the OWNING tier of every stored copy
}

impl TwoTierCache {
    fn new(cpu_capacity: usize) -> Self {
        Self { gpu: HashSet::new(), cpu: CpuPool::new(cpu_capacity) }
    }

    fn store(&mut self, blocks: &[BlockHash]) {
        // Eager mode: the GPU tier takes the blocks; the CPU copy is
        // written once at store time and its recency never moves again.
        for b in blocks {
            self.gpu.insert(*b);
            if self.cpu.order.len() == self.cpu.capacity {
                self.cpu.evict_coldest();
            }
            self.cpu.order.push(*b);
        }
    }

    /// A request whose whole prefix hits the fast tier. THE DEFECT:
    /// the CPU pool -- which OWNS a copy of that prefix -- is never
    /// told. Its recency for these blocks stays frozen at store time.
    fn lookup_prefix_hit_on_gpu(&mut self, prefix: &[BlockHash]) -> bool {
        prefix.iter().all(|b| self.gpu.contains(b))
    }

    /// A new store that needs room: evicts from the owning tier.
    fn store_and_collect_evictions(&mut self, blocks: &[BlockHash]) -> Vec<BlockHash> {
        let mut evicted = Vec::new();
        for b in blocks {
            if self.cpu.order.len() == self.cpu.capacity {
                if let Some(dead) = self.cpu.evict_coldest() {
                    evicted.push(dead);
                }
            }
            self.cpu.order.push(*b);
            self.gpu.insert(*b);
        }
        evicted
    }
}

fn h(v: u64) -> BlockHash {
    BlockHash(v)
}

#[test]
fn the_just_used_prefix_is_evicted_before_truly_cold_blocks() {
    let mut c = TwoTierCache::new(4);
    let req_a = [h(1), h(2)];
    let req_b = [h(3), h(4)];
    c.store(&req_a); // A stored first -> the coldest end
    c.store(&req_b); // B stored last -> the hot end

    // A's prefix is reused and hits ENTIRELY on the fast tier.
    assert!(c.lookup_prefix_hit_on_gpu(&req_a));

    // A new store needs 2 blocks. THE WRONG EVICTION: the owning
    // tier evicts A -- the blocks that were JUST used -- because
    // the fast-tier hit never refreshed their recency.
    let req_c = [h(5), h(6)];
    let evicted = c.store_and_collect_evictions(&req_c);

    assert_eq!(
        evicted,
        vec![h(1), h(2)],
        "the just-used prefix A is evicted first: the owning tier's recency was frozen at store time"
    );
    assert!(
        !c.cpu.holds(h(1)) && !c.cpu.holds(h(2)),
        "A's owning-tier copy is gone although A is the live working set"
    );
    assert!(
        c.cpu.holds(h(3)) && c.cpu.holds(h(4)),
        "truly cold B survived: the eviction order is inverted"
    );
}"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `tier-hit-refreshes-lru-recency-at-owning-tier`
// (riir-refine rust_perf corpus, Batch 308) -- the LAW arm.
//
// Same harness as the anti-pattern arm; the law added: a fast-tier
// prefix hit is RECORDED at admission into a pending map (a request
// waiting for blocks repeats its lookup every step, so one record
// replaces per-step touches), at the serving step the owning tier's
// hit blocks are moved to the MRU end (touch, then re-free there --
// a cached copy stays free, its RECENCY moves), and the pending map
// is drained on every terminal path (request finished + reset) so a
// stale record cannot leak into a reused request id.

use std::collections::{HashMap, HashSet};
use std::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct BlockHash(u64);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct ReqId(u32);

struct CpuPool {
    capacity: usize,
    // Recency order: front = coldest (evicted first), back = hottest.
    order: Vec<BlockHash>,
}

impl CpuPool {
    fn new(capacity: usize) -> Self {
        Self { capacity, order: Vec::new() }
    }

    fn holds(&self, blk: BlockHash) -> bool {
        self.order.contains(&blk)
    }

    fn evict_coldest(&mut self) -> Option<BlockHash> {
        if self.order.is_empty() { None } else { Some(self.order.remove(0)) }
    }

    /// The owning tier's touch: move the block to the MRU end. The
    /// block stays FREE (it is a cached copy, not an allocation);
    /// only its recency moves.
    fn touch_to_mru(&mut self, blk: BlockHash) {
        if let Some(i) = self.order.iter().position(|b| *b == blk) {
            let moved = self.order.remove(i);
            self.order.push(moved);
        }
    }
}

struct TwoTierCache {
    gpu: HashSet<BlockHash>,
    cpu: CpuPool,
    // Hits recorded at admission (lookup time), keyed by request:
    // consumed ONCE at the request's next serving step.
    pending_local_hits: HashMap<ReqId, Vec<BlockHash>>,
}

impl TwoTierCache {
    fn new(cpu_capacity: usize) -> Self {
        Self {
            gpu: HashSet::new(),
            cpu: CpuPool::new(cpu_capacity),
            pending_local_hits: HashMap::new(),
        }
    }

    fn store(&mut self, blocks: &[BlockHash]) {
        for b in blocks {
            self.gpu.insert(*b);
            if self.cpu.order.len() == self.cpu.capacity {
                self.cpu.evict_coldest();
            }
            self.cpu.order.push(*b);
        }
    }

    fn lookup_prefix_hit_on_gpu(&mut self, req: ReqId, prefix: &[BlockHash]) -> bool {
        // A re-lookup supersedes the stale record.
        self.pending_local_hits.remove(&req);
        if prefix.iter().all(|b| self.gpu.contains(b)) {
            // Refreshed at admission: a request waiting for blocks
            // repeats this lookup every step.
            self.pending_local_hits.insert(req, prefix.to_vec());
            return true;
        }
        false
    }

    /// The serving step: the recorded hit moves the owning tier's
    /// blocks to the MRU end.
    fn update_after_alloc(&mut self, req: ReqId) {
        if let Some(blocks) = self.pending_local_hits.remove(&req) {
            for b in &blocks {
                self.cpu.touch_to_mru(*b);
            }
        }
    }

    /// A terminal path: the request finished; its pending record
    /// must not survive to a reused request id.
    fn request_finished(&mut self, req: ReqId) {
        self.pending_local_hits.remove(&req);
    }

    fn reset(&mut self) {
        self.pending_local_hits.clear();
    }

    fn store_and_collect_evictions(&mut self, blocks: &[BlockHash]) -> Vec<BlockHash> {
        let mut evicted = Vec::new();
        for b in blocks {
            if self.cpu.order.len() == self.cpu.capacity {
                if let Some(dead) = self.cpu.evict_coldest() {
                    evicted.push(dead);
                }
            }
            self.cpu.order.push(*b);
            self.gpu.insert(*b);
        }
        evicted
    }
}

fn h(v: u64) -> BlockHash {
    BlockHash(v)
}

#[test]
fn the_hit_refreshes_the_owning_tier_and_the_eviction_order_holds() {
    let mut c = TwoTierCache::new(4);
    let req_a = [h(1), h(2)];
    let req_b = [h(3), h(4)];
    c.store(&req_a);
    c.store(&req_b);

    // A's prefix reused: hits on the fast tier; the owning tier is
    // told at the serving step.
    let rider = ReqId(7);
    assert!(c.lookup_prefix_hit_on_gpu(rider, &req_a));
    c.update_after_alloc(rider);
    c.request_finished(rider); // the terminal path drains the record

    // The same store: B -- the TRULY cold copy -- is evicted; A's
    // recency was refreshed, so A survives.
    let req_c = [h(5), h(6)];
    let evicted = c.store_and_collect_evictions(&req_c);

    assert_eq!(
        evicted,
        vec![h(3), h(4)],
        "the cold copy B is evicted: the just-used A was refreshed to the MRU end"
    );
    assert!(
        c.cpu.holds(h(1)) && c.cpu.holds(h(2)),
        "the live working set keeps its owning-tier copy"
    );
    assert!(!c.cpu.holds(h(3)) && !c.cpu.holds(h(4)));
}

#[test]
fn a_stale_record_never_survives_to_a_reused_request_id() {
    let mut c = TwoTierCache::new(4);
    let prefix = [h(1), h(2)];
    c.store(&prefix);

    let req = ReqId(9);
    assert!(c.lookup_prefix_hit_on_gpu(req, &prefix));
    // The request dies BEFORE its serving step; the record must go.
    c.request_finished(req);
    assert!(c.pending_local_hits.is_empty(), "the terminal path drained it");

    // A NEW request reuses the id and hits again: exactly one fresh
    // record exists, and the serving step consumes it.
    assert!(c.lookup_prefix_hit_on_gpu(req, &prefix));
    c.update_after_alloc(req);
    assert!(c.pending_local_hits.is_empty(), "the serving step consumed the record");

    // reset() drains everything regardless.
    assert!(c.lookup_prefix_hit_on_gpu(req, &prefix));
    c.reset();
    assert!(c.pending_local_hits.is_empty(), "reset drains the pending map");
}"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is a CROSS-TIER cache-coherence law -- which tier's recency hears about a hit served by another tier -- priced in a serving topology's eviction quality over a workload; an A/B microbench of a stand-in cannot measure eviction-order correctness; source-reported behavior is PROVENANCE, never bars). SOURCE (vllm-project/vllm @ c1b7dffa1457f4ffe7eafb08092eb2505a9651e4 #62444 '[Bugfix][KV Offload] Refresh CPU LRU for GPU-hit prefixes in eager SimpleCPUOffload', Apache-2.0, Hexiang Wang, 2026-10-29; every quote re-verified verbatim at the per-commit .patch this batch -- the PR-bundle SHAs moved vs the staged pin, a rebase; the per-commit patch at the PINNED sha is the verification surface; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT: eager CPU-offload mode -- a GPU-tier prefix hit bypassed the CPU store path (eager stores skip the CPU copy), so the CPU LRU never saw the touch and evicted in the wrong order; the PR title IS the defect statement. THE LAW (vllm/v1/simple_kv_offload/manager.py @ c1b7dff, three surfaces): (1) THE ADMISSION RECORD -- get_num_new_matched_tokens records the local hit into self._pending_local_hits[request.request_id] = num_computed_tokens under the comment 'Refreshed at admission: a request waiting for blocks repeats this lookup every step.' (one record replaces per-step touches; the re-lookup pops the stale record first, mirroring the _pending_cpu_hits stale-pop above it); (2) THE SERVING-STEP TOUCH -- update_state_after_alloc pops the record and runs the prefix_hit lookup + self.cpu_block_pool.touch(prefix_blocks) + self.cpu_block_pool.free_blocks(reversed(prefix_blocks)) under the comment 'Eager CPU is inclusive: a GPU prefix hit also uses the CPU copy, but eager stores skip it, so move it to the MRU end here.' -- touch then re-free AT THE MRU END (the blocks are FREE cached copies; only their recency moves; reversed() so the prefix's own order survives the round trip); (3) THE TERMINAL DRAINS -- request_finished pops the req_id entry and reset() clears the map, so a request that dies between lookup and serving cannot leak its record into a reused id (the stale-entry discipline the _pending_cpu_hits map already practiced beside it). THE UPSTREAM TEST names the shape (tests/v1/simple_kv_offload/test_scheduler.py::test_gpu_prefix_hit_refreshes_cpu_lru, docstring verbatim): 'Eager mode: a prefix that hits on GPU keeps its CPU copy at the MRU end.' -- fill the pool (req_a LRU, req_b MRU), re-serve req_a's prefix as a pure GPU hit, store req_c, and assert cpu_cached(req_a) == [True, True] while cpu_cached(req_b) == [False, False] (the JUST-USED A survives, the truly cold B is evicted). The HYBRID arm (test_gpu_prefix_hit_refreshes_cpu_mamba_boundary, docstring verbatim): 'A hybrid GPU prefix hit refreshes the CPU attention blocks and the Mamba state at the hit boundary, not older Mamba states, once admitted.' -- the refresh covers every group the hit served, to the boundary, never beyond. REGIME: any tiered cache where a FASTER tier can serve a hit over a copy OWNED by a SLOWER tier and the slow tier's recency is only written at store/copy time (eager/offload serving, CPU-offload pools, tiered KV, a RAM front over a disk cache); request-level plumbing where the hit is DISCOVERED in one phase (lookup/admission) and CONSUMED in another (the serving step) -- the pending map is the bridge. ANTI-REGIMES: single-tier caches (no cross-tier hit exists); inclusive hierarchies whose hit path already writes through to the lower tier (the write-through IS the touch); caches whose owning tier observes every read itself (no bypass to correct); adaptive tiers where a hit DEMOTES data (a different decision entirely). KINSHIP/DISTINCT-FROM (read + disclosed): slru-probation-protected-promote-on-second-touch (B255, entries_35) owns WITHIN-one-cache second-touch promotion across probation segments -- THIS is coherence BETWEEN tiers (which tier's LRU hears about a hit another tier served; no promotion/segment axis); cache-reuse-assertion-gated-on-eviction-counter (B270, entries_40) owns the TEST-side honesty law (assert reuse only when the eviction counter moved -- a measurement instrument, not the production touch); persistent-cache-teardown-needs-second-witness (B270) owns teardown witnessing, not recency; the terminal-drain discipline composes with the pending-map laws the source itself pairs (the _pending_cpu_hits free_pending_cpu_hit drains) -- in-use-state-retry-under-serving-mark-keyed-on-refusal-cause (B271, entries_45) keys retry on the refusal CAUSE at serve time, a different axis. NOVEL: grep 'lru|recency|owning tier|tier hit|touch' over the rust_perf corpus rule_ids + notes: the touch/promote family is all within-one-cache; zero cross-tier recency owner (the staged row's own adjudication: 'coherence BETWEEN tiers, vs the corpus's within-one-cache touch/promote family'). Suggestion-only (I/O orchestration; a parse-only proposer cannot see tier ownership or the lookup/serving phase split). FIXTURE PROVENANCE: original machinery (never vllm code) -- a two-tier block cache stand-in (a HashSet fast tier + an LRU-ordered owning pool), the buggy arm's lookup returns the hit without telling the owning tier (the just-used prefix evicted before truly cold blocks, pinned by the eviction list), the fixed arm records the hit at admission into pending_local_hits, consumes it at update_after_alloc as touch_to_mru, drains on request_finished + reset (a stale record never survives to a reused request id, pinned); both arms share the harness and differ only on the law; the eviction-list assertions are the discriminating tests. Cite: vllm-project/vllm @ c1b7dffa1457f4ffe7eafb08092eb2505a9651e4 -- vllm/v1/simple_kv_offload/manager.py (the _pending_local_hits map: __init__, get_num_new_matched_tokens, update_state_after_alloc, request_finished, reset) + tests/v1/simple_kv_offload/test_scheduler.py::test_gpu_prefix_hit_refreshes_cpu_lru + ::test_gpu_prefix_hit_refreshes_cpu_mamba_boundary; Batch 308 / Plan 313 / Research 352.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── caching: a family REFUSAL quarantines the entry; transient
        // passes through keeping the state and its read -------------
        // Batch 308 / TensorFold #1539 @ d1fcb9d93458 (the refused()
        // verdict switch + the scatter() helper + bad()'s
        // forget-ordering).
        PerfRule {
            rule_id: "refusal-quarantines-entry-transient-passes".to_string(),
            name: "a copy the receiving family REFUSES (words that differ, a size or a range that does not fit its blank) is a failed check of the ENTRY, not a transient failure of the system - a refusal must quarantine the entry (counted, logged with the file named, the file unlinked ONCE, every state naming the file dropped, and the state's read-ahead entry dropped AHEAD of the removal so a same-name state written later cannot re-seed the evicted read), while TRANSIENT errors (a failing link, memory) pass through UN-quarantined keeping the state and its read so retry works; both classes route through ONE helper off ONE switch so a family's new refusal class is a single line".to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `refusal-quarantines-entry-transient-passes`
// (riir-refine rust_perf corpus, Batch 308) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's content store
// (TensorFold's rows/prompt-cache is cited in context_notes): stored
// states are copied into a receiving family's ranges. The family can
// REFUSE what a file holds (words that differ, a size or a range that
// does not fit its blank -- a failed check of the FILE) or fail
// transiently (a failing link, memory). The incumbent treats every
// error alike: the bare error propagates and the state STAYS in the
// store, uncounted and unlogged -- every later recall that picks it
// reads its files again, and nothing anywhere says why.

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Refusal {
    SnapshotSize,
    BadKeptRange,
    DiskWords,
    KeptWordsDiffer,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum StoreError {
    Refused(Refusal),        // the family will not take what the file holds
    Transient(&'static str), // a failing link or memory: retry works
}

struct Counts {
    recalls: usize,
    failed: usize,
}

struct Store {
    states: Vec<(String, String)>, // (state name, file it names)
    files: HashMap<String, Vec<u64>>,
    family_blank: usize,    // the size the family accepts
    family_words: Vec<u64>, // the words the family expects
    counts: Counts,
    log: Vec<String>,
    reads: usize, // file reads performed -- the repeated-cost probe
}

impl Store {
    fn recall(&mut self, name: &str) -> Result<(), StoreError> {
        self.counts.recalls += 1;
        let hit = self.states.iter().find(|(n, _)| n == name).cloned();
        let Some((_, file)) = hit else {
            return Err(StoreError::Transient("absent"));
        };
        self.reads += 1; // every recall of a listed state reads its file
        let words = self.files.get(&file).cloned().unwrap_or_default();
        self.scatter(&words)
    }

    // THE DEFECT: a bare try. A refusal takes the same path as a
    // transient failure -- neither counted, nor logged, nor acted on
    // -- so the refused state stays listed forever.
    fn scatter(&mut self, words: &[u64]) -> Result<(), StoreError> {
        if words.len() != self.family_blank {
            return Err(StoreError::Refused(Refusal::SnapshotSize));
        }
        if words != self.family_words.as_slice() {
            return Err(StoreError::Refused(Refusal::DiskWords));
        }
        Ok(())
    }
}

fn h(v: u64) -> u64 {
    v
}

#[test]
fn a_refused_state_is_recalled_again_and_again_with_no_trace() {
    let mut s = Store {
        states: vec![
            ("ghost".to_string(), "ghost.st".to_string()),
            ("live".to_string(), "live.st".to_string()),
        ],
        files: HashMap::from([
            ("ghost.st".to_string(), vec![h(9), h(9), h(9)]), // the family refuses its size
            ("live.st".to_string(), vec![h(1), h(2)]),
        ]),
        family_blank: 2,
        family_words: vec![h(1), h(2)],
        counts: Counts { recalls: 0, failed: 0 },
        log: Vec::new(),
        reads: 0,
    };

    // Every scheduler pass picks the ghost: its file is read, the
    // family refuses, the error surfaces -- and the state STAYS.
    for _ in 0..3 {
        let err = s.recall("ghost").unwrap_err();
        assert_eq!(err, StoreError::Refused(Refusal::SnapshotSize));
    }
    assert_eq!(
        s.reads, 3,
        "the repeated-cost loop: the same bad file is read on EVERY recall"
    );
    assert_eq!(s.counts.failed, 0, "nothing was counted");
    assert!(s.log.is_empty(), "nothing was logged");
    assert!(
        s.states.iter().any(|(n, _)| n == "ghost"),
        "the refused state is still listed"
    );
}"#.to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `refusal-quarantines-entry-transient-passes`
// (riir-refine rust_perf corpus, Batch 308) -- the LAW arm.
//
// Same harness as the anti-pattern arm; the law added: BOTH copy
// paths route through ONE helper; a REFUSAL (the family will not
// take what the file holds -- words that differ, a size or a range
// that does not fit its blank) is a failed check of the FILE and
// quarantines the entry -- counted, logged with the file named, the
// file unlinked ONCE, every state naming the file dropped, and the
// state's read-ahead entry dropped AHEAD of the removal (a
// same-name state written later must not re-seed the evicted read).
// A TRANSIENT error (a failing link, memory) passes through
// UN-quarantined, keeping the state and its read so retry works.

use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Refusal {
    SnapshotSize,
    BadKeptRange,
    DiskWords,
    KeptWordsDiffer,
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum StoreError {
    Refused(Refusal),
    Transient(&'static str),
}

/// Whether the family REFUSED what a file holds. ONE switch: a
/// family's new refusal class is a single line here.
fn refused(err: &StoreError) -> bool {
    matches!(err, StoreError::Refused(_))
}

struct Counts {
    recalls: usize,
    failed: usize,
}

struct Store {
    states: Vec<(String, String)>,
    files: HashMap<String, Vec<u64>>,
    // name -> its listed read-ahead entry (coupled per state)
    read_ahead: HashMap<String, usize>,
    family_blank: usize,
    family_words: Vec<u64>,
    link_down: bool, // the standing transient failure
    counts: Counts,
    log: Vec<String>,
    reads: usize,
}

impl Store {
    fn recall(&mut self, name: &str) -> Result<(), StoreError> {
        self.counts.recalls += 1;
        let hit = self.states.iter().find(|(n, _)| n == name).cloned();
        let Some((_, file)) = hit else {
            return Err(StoreError::Transient("absent"));
        };
        self.reads += 1;
        let words = self.files.get(&file).cloned().unwrap_or_default();
        self.scatter(name, &file, &words)
    }

    /// ONE helper the copy goes through: a refusal of the words is
    /// a failed check of the file.
    fn scatter(&mut self, name: &str, file: &str, words: &[u64]) -> Result<(), StoreError> {
        let result = if self.link_down {
            Err(StoreError::Transient("link"))
        } else if words.len() != self.family_blank {
            Err(StoreError::Refused(Refusal::SnapshotSize))
        } else if words != self.family_words.as_slice() {
            Err(StoreError::Refused(Refusal::DiskWords))
        } else {
            Ok(())
        };
        match result {
            Err(e) if refused(&e) => Err(self.bad(name, file, e)),
            other => other,
        }
    }

    /// A file that failed its checks is deleted, with every state
    /// that names it; the error itself passes on.
    fn bad(&mut self, name: &str, file: &str, err: StoreError) -> StoreError {
        self.counts.failed += 1;
        self.log.push(format!("{name}: refused words in {file}"));
        self.files.remove(file); // unlinked ONCE
        // A state that is gone takes its read ahead with it: a
        // same-name state written later must not re-seed it.
        self.read_ahead.remove(name);
        let mut i = 0;
        while i < self.states.len() {
            if self.states[i].1 == file {
                self.states.remove(i);
            } else {
                i += 1;
            }
        }
        err
    }
}

fn h(v: u64) -> u64 {
    v
}

#[test]
fn a_refusal_quarantines_after_one_read() {
    let mut s = Store {
        states: vec![
            ("ghost".to_string(), "ghost.st".to_string()),
            ("live".to_string(), "live.st".to_string()),
        ],
        files: HashMap::from([
            ("ghost.st".to_string(), vec![h(9), h(9), h(9)]),
            ("live.st".to_string(), vec![h(1), h(2)]),
        ]),
        read_ahead: HashMap::from([("ghost".to_string(), 1)]),
        family_blank: 2,
        family_words: vec![h(1), h(2)],
        link_down: false,
        counts: Counts { recalls: 0, failed: 0 },
        log: Vec::new(),
        reads: 0,
    };

    // First recall reads the file, the family refuses, the
    // quarantine runs: counted, logged with the file named,
    // unlinked, every naming state dropped, the read taken along.
    let err = s.recall("ghost").unwrap_err();
    assert!(matches!(err, StoreError::Refused(_)), "the refusal itself still surfaces");
    assert_eq!(s.reads, 1, "ONE read: the quarantine removed the state");
    assert_eq!(s.counts.failed, 1, "the refusal is counted");
    assert_eq!(
        s.log,
        vec!["ghost: refused words in ghost.st"],
        "the log names the file"
    );
    assert!(!s.files.contains_key("ghost.st"), "the file is unlinked once");
    assert!(
        !s.states.iter().any(|(n, _)| n == "ghost"),
        "every state naming the file is dropped"
    );
    assert!(
        !s.read_ahead.contains_key("ghost"),
        "the read-ahead entry left with the state"
    );

    // Later recalls find nothing: the repeated-cost loop is gone.
    assert!(s.recall("ghost").is_err(), "absent is its own honest error");
    assert_eq!(s.reads, 1, "no second read: the ghost is gone from the store");
    assert_eq!(s.counts.failed, 1, "no double count");

    // The live state still recalls cleanly.
    assert!(s.recall("live").is_ok());
}

#[test]
fn a_transient_error_keeps_the_state_and_the_retry_works() {
    let mut s = Store {
        states: vec![("live".to_string(), "live.st".to_string())],
        files: HashMap::from([("live.st".to_string(), vec![h(1), h(2)])]),
        read_ahead: HashMap::from([("live".to_string(), 1)]),
        family_blank: 2,
        family_words: vec![h(1), h(2)],
        link_down: true, // the link is failing
        counts: Counts { recalls: 0, failed: 0 },
        log: Vec::new(),
        reads: 0,
    };

    // The transient error passes through UN-quarantined.
    assert_eq!(
        s.recall("live").unwrap_err(),
        StoreError::Transient("link"),
        "the transient error surfaces as itself"
    );
    assert_eq!(s.counts.failed, 0, "a transient failure is never a quarantine");
    assert!(s.log.is_empty(), "nothing logged: the state is kept");
    assert!(s.states.iter().any(|(n, _)| n == "live"), "the state stays listed");
    assert!(s.read_ahead.contains_key("live"), "its read stays listed");

    // The link heals: the SAME state recalls cleanly -- retry worked.
    s.link_down = false;
    assert!(s.recall("live").is_ok(), "the kept state serves after the transient heals");
    assert_eq!(s.reads, 2, "the retry did read again -- which is why the state was kept");
}"#.to_string(),
            context_notes: Some("Grade B -- NotBenchable ON THIS BOX (the decision content is a store-hygiene VERDICT law -- which error classes quarantine an entry and which pass through -- priced in a serving store's repeated-read cost and log integrity; an A/B microbench of a stand-in cannot measure eviction-of-bad-entry behavior under a fault-injected family; source-reported behavior is PROVENANCE, never bars). SOURCE (ashhart/tensorfold @ d1fcb9d934584b94789a84cde31d2ed655ab4c71 #1539 'rows: a state its family refuses while it scatters is forgotten after one recall', Apache-2.0, Robert Wojciechowski, 2026-10-28; every quote re-verified verbatim at the per-commit .patch this batch; inner double quotes rendered as single quotes per house string-literal safety). THE DEFECT (commit message, verbatim): 'rows_read.one and the read-ahead branch of read called the family's scatter with a bare try, so a refusal inside it skipped bad: nothing was counted or logged and the state stayed in the store, where every later recall that picked it read its files again. Nemotron's SnapshotSize, Qwen3.5's and FlashNext's KeptWordsDiffer and BadKeptRange, and DiskWords where a family raises it all took that path.' THE LAW, three clauses (zig/src/core/rows_read.zig @ d1fcb9d): (1) THE VERDICT CLAUSE -- ONE classifier over ONE switch, comment verbatim: 'Whether a family refused what a file holds (words that differ, a size or a range that does not fit its blank).' -- fn refused(err) = switch { error.SnapshotSize, error.BadKeptRange, error.DiskWords, error.KeptWordsDiffer => true, else => false }; the commit message verbatim: 'A refusal is words the family will not take (DiskWords, KeptWordsDiffer) or a size or range that does not fit its blank (SnapshotSize, BadKeptRange); they are listed in one switch (refused), so a family's new one is a single line. Other errors, a failing link or memory, pass through as before and keep the state and its read.' (2) THE QUARANTINE -- both copy paths route through ONE helper whose doc comment verbatim: 'A file's payload copied into ranges of saved; a refusal of its words is a failed check of the file.' -- a refusal routes to bad(): counted (counts.failed += 1), logged with the FILE NAMED, the file unlinked ONCE, and EVERY state naming the file removed ('A file that failed its checks deleted, with every state that names it; the error passed on.'); (3) THE FORGET-ORDERING CLAUSE -- bad() drops the state's read-ahead entry AHEAD of the removal, commit message verbatim: 'bad now drops the read ahead of every state it forgets (Prefetch.forget), whichever caller got there: a taken read stayed listed until a sweep, and would have been joined again, with the refused words, by a state of the same name that is written later.' (the call-site comment verbatim: 'a state that is gone takes its read ahead with it'; Prefetch.forget itself: 'name's read, if any, out of the list and freed (engine thread): its state is forgotten.'). THE TRANSIENT ARM stays open: 'Other errors, a failing link or memory, pass through as before and keep the state and its read' -- retry semantics live only on the transient side; the refusal itself still surfaces to the caller (bad returns the error after the quarantine). MEASUREMENT-GRADE TEST: the Fake family gains a scatters fault ('after n copies, every one fails with a given error' -- Fault { after, err }) and rows_refusal_test.zig drives it three ways: on a shared head block (both states going), on the state's own file (only it going), and with a transient error (kept) -- 'synchronously and with both states read ahead, checking the counts, the states, the files and the reads of both.' REGIME: content-addressed/state stores whose payload a receiving family/consumer validates at copy time (words, sizes, ranges); any negative-result path where one error class is DETERMINISTIC (the bytes are wrong for this consumer -- re-reading cannot help) and another is TRANSIENT (link, memory -- retry helps); stores with read-ahead/prefetch entries coupled per state. ANTI-REGIMES: consumers that accept anything (no refusal class exists -- nothing to classify); refusals that are actually retryable (a mid-write torn file heals -- classify it transient, never quarantine it); stores without read-ahead coupling (the forget-ordering clause is inert there; the verdict clause still applies); one-shot reads with no store (nothing persists to quarantine). KINSHIP/DISTINCT-FROM (read + disclosed): batch-local-memoization-negative-cache-tombstone (B242, entries_06) owns the negative cache for BATCH misses (memoizing the miss ITSELF as a tombstone -- the lookup is the problem); THIS quarantines an entry whose PAYLOAD a consumer refused (the entry is the problem, the lookup was correct); pinned-entry-tombstone-freed-at-last-unpin (B256, entries_36) owns the PIN lifetime of tombstoned entries (when a tombstone may be freed); THIS owns the refusal VERDICT that creates the quarantine + the read-ahead coupling; in-use-state-retry-under-serving-mark-keyed-on-refusal-cause (B271, entries_45) keys RETRY on the refusal cause at serve time (the consumer side) -- compose: the cause that rule reads is the class this rule routes; copy-source-quarantine-unproven-free-pinned (kernel_opt B256) owns the async-free lifetime axis (completion proof before free); THIS owns the data-validity axis (the family's own verdict on the bytes); checkpoint-reads-skip-page-cache-with-refusal-fallback (B257, entries_37) owns the read-path fallback on a refusal; THIS owns the post-refusal store hygiene; drain-device-work-before-free (kernel_opt gpu_111) owns the error-vs-empty distinction (a failed read is an ERROR, never an empty result) -- THIS adds the store-side consequence ladder: a refusal specifically kills the ENTRY while a transient failure keeps it. NOVEL: grep 'refus|quarantine|tombstone|negative' over the rust_perf rule_ids + notes: the tombstone/quarantine cluster owns lifetime/pin/cause axes; zero owner of the refusal-vs-transient VERDICT classification or the forget-ordering clause (the staged row's own adjudication: 'none owns the refusal-verdict clause or the forget-ordering clause'). Suggestion-only (the verdict set is family-defined data; a parse-only proposer cannot see which error classes a consumer refuses). FIXTURE PROVENANCE: original machinery (never tensorfold code) -- a content store stand-in (states naming files, an in-memory file map, a family blank + expected words, a link_down flag for the transient class), the buggy arm's bare-try scatter (three recalls read the same bad file three times, counts.failed 0, empty log, the state still listed -- the repeated-cost loop pinned), the fixed arm's refused() classifier + scatter() helper + bad() (ONE read then the quarantine: counted once, the log names the file, unlinked once, naming states dropped, the read-ahead entry taken along; the transient arm keeps the state and its read and the retry succeeds after the link heals); both arms share the harness and differ only on the law; the reads/counts/log assertions are the discriminating tests. Cite: ashhart/tensorfold @ d1fcb9d934584b94789a84cde31d2ed655ab4c71 -- zig/src/core/rows_read.zig (refused() + scatter() + bad() + the two call sites), zig/src/core/rows_prefetch.zig (Prefetch.forget), zig/src/core/rows_fake.zig (the scatters Fault), zig/src/core/rows_refusal_test.zig (the fault-injection battery); Batch 308 / Plan 313 / Research 352.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

//! Entries slice — Batch 356 (2026-11-12): four `rust_perf` rules from
//! the r/rust AI borrow-checker-appeasement case study (Plan 366 /
//! Research 400) — the `mem::take` check-out window (allocation),
//! the clone-before-mutating-loop appeasement (allocation), the
//! thread-spawn-per-call pool-reuse lead (concurrency, BENCHABLE), and
//! the dense-id HashMap→slot-array carve (layout, BENCHABLE) — plus one
//! count-neutral rider face landed on `bounded-batch-buffer-pool`
//! (`entries_06`).
//!
//! THE DROP (recorded, never silent): R4 `cross-thread-buffer-return-ring`
//! was DROPPED AT AUTHORING on the family adjudication —
//! `bounded-batch-buffer-pool` (`entries_06`) OWNS the recycle law
//! (hot alloc/free cycle → bounded recycle pool; the cap IS the
//! decision), and the post's 5-buffer pool remedy is its positive
//! instance; the CROSS-THREAD return ring (producer sends buffers down,
//! the consumer returns them through a second channel instead of
//! Arc-cloning) is the SAME law's concurrent enforcement form, not a new
//! decision axis, and `slow-consumer-latest-value-single-slot`
//! (`entries_70`) already owns the coalescing axis beside it. The face on
//! the pool rule records the concurrent form + the post provenance.
//!
//! Source: a POST-ONLY r/rust case study — a Claude-ported
//! TypeScript→Rust deterministic RTS replay engine (3,091/3,091 bit-exact
//! FNV-1a checkpoint hashes over 30,000+ ticks) whose AI appeasement
//! debt inventory was: 19 OOP classes flattened into 19 `impl Game`
//! blocks on ONE 12,349-line file; 175 clone/collect sites whose only
//! purpose is dropping a borrow before a mutating loop; `std::mem::take`
//! check-out/check-in windows (the field is secretly empty during the
//! nested calls); full pixel buffers cloned into an `Arc` per frame
//! (5.9 GB heap per 900-frame video); a 4-thread `scope` spawned per
//! frame (3,840 OS threads per video). NO repo (the author offered
//! "can provide git if wanted"; not provided) — decision-content-only
//! transcription (the forge-blog precedent): the post contributes
//! PATTERN SHAPES + regime conditions; every fixture is an ORIGINAL Rust
//! reproduction. The author's own refactor (3-crate split, disjoint
//! sub-structs, flat slot arrays, a 5-buffer recycled pool) measured
//! 1.90× sim / 2.6–3.0× draw / 8.5 s→0.77 s incremental check, all
//! hashes matching — PROVENANCE ONLY, never citable as magnitude
//! evidence (single-box anecdote); our fixtures measure their own
//! numbers.
//!
//! KILL (recorded): god-file / N-impl-block flattening — ambient
//! architecture, no bounded fix-space (the decision-content test's first
//! kill class); routes to the skill layer where the file-size rule
//! already owns it.
//!
//! Fixtures: `corpus/fixtures/b356_fixtures.rs` — ORIGINAL plain-Rust
//! reproductions, standalone-compiled AND executed BEFORE entry
//! (16/16 at `--test-threads=1`; the four-arm discipline per law:
//! buggy / fixed / all-safe / tampered-verdict). Each rule's arms embed
//! the same executed code. Batch 356 / Plan 366 / Research 400.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── allocation: the mem::take check-out/check-in window ─────────
        // The field sits at its empty default between take and
        // write-back; every reader entering the window observes empty
        // data. Post-only source (Research 400).
        PerfRule {
            rule_id: "mem-take-field-checkout-empty-window".to_string(),
            name: "std::mem::take field check-out/check-in to appease a nested &mut self call → the field holds its EMPTY DEFAULT inside the window (every reader entering it — a nested audit, a Debug print, a panic path, a re-entrant tick — observes empty data and draws wrong conclusions) → split the owner into disjoint sub-structs so the call takes the other half, or reorder so no reader enters the window; take-with-writeback AROUND a reader is a hazard, not a borrow fix".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"struct GameState { inv: Vec<u64>, score: u64 }

impl GameState {
    // THE APPEASEMENT: the field is checked out so a nested `&mut self`
    // call compiles — `inv` is EMPTY until write-back, and every reader
    // inside the window (this audit, a debug dump, a re-entrant tick)
    // observes the empty default.
    fn appeased_flush(&mut self, seen: &mut Vec<usize>) {
        let mut taken = std::mem::take(&mut self.inv);
        self.flush(seen); // the nested &mut self call the take appeased
        taken.push(self.score);
        self.inv = taken; // check-in
    }
    fn flush(&mut self, seen: &mut Vec<usize>) {
        seen.push(self.inv.len()); // reads INSIDE the window: sees 0
        self.score += 1;
    }
}"#.to_string(),
            fixed_code: r#"struct SubInv { items: Vec<u64> }
struct SubSys { score: u64 }
struct GameState { inv: SubInv, sys: SubSys }

impl GameState {
    // THE REMEDY (architectural): split the owner into disjoint halves
    // so the same work runs on the OTHER half while the inventory stays
    // live — no window ever opens, no take, no clone. An explicit
    // Option<T> + take is the alternative when the empty state is
    // genuinely meaningful: the window becomes VISIBLE in the type.
    fn split_flush(&mut self, seen: &mut Vec<usize>) {
        let GameState { inv, sys } = self;
        sys.score += 1;             // the "nested" write, on the disjoint half
        seen.push(inv.items.len()); // the same reader sees the FULL data
        inv.items.push(sys.score);
    }
}"#.to_string(),
            context_notes: Some(
                "NotBenchable-by-domain (hazard-window class — the decision is WHERE A READER SITS, not a Pareto axis; the dial9/136 lane). THE LAW: mem::take + write-back opens a window in which the field holds its empty default; ANY reader entering the window observes empty data and draws wrong conclusions — type-safe, hash-stable, and wrong (the post's 12,349-line case study carried these as check-out/check-in idioms; the zero-hash streak made them invisible to every functional test). THE REMEDY IS ARCHITECTURAL: disjoint sub-structs so the 'nested' call takes the other half, reorder so the reader runs first, or an explicit Option<T> that makes the window VISIBLE in the type. DEDUP ADJUDICATION (T0 at landing HEAD, 3 vocabularies: mem::take | checkout | check-out window): `mem-take` (entries_02) owns the OPPOSITE POLARITY — there take REPLACES clone+clear for a deliberate move-out (no window reader between take and hand-off; that shape stays the remedy and this rule must not fire on it); THIS owns the check-out/check-in WINDOW as the appeasement itself. The 'checkout' greps in entries_19-22 are file-header prose about git checkouts (read and ruled distinct). REGIME: god-struct methods whose helper needs &mut self while one field is logically held; AI-ported code (the appeasement class is structural POSITION — the code type-checks, the value IS written back, no lint fires). ANTI-REGIMES: move-outs with no window reader (mem-take's shape); fields whose empty default is a MEANINGFUL state (model it as Option, not a lie); single-threaded sites where the reader can simply run before the take. Fixtures: corpus/fixtures/b356_fixtures.rs F1 — the window hazard (the nested reader sees len 0), final-state equality across forms, the all-safe move-out, the tampered missing check-in. Source: a post-only r/rust case study (Research 400; NO repo — decision-content-only transcription). Batch 356 / Plan 366.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: the clone that only ends a borrow ────────────────
        // Clone immediately preceding a mutating loop over the same
        // owner, the clone's sole consumer reading it after the loop.
        PerfRule {
            rule_id: "clone-before-mutating-loop-borrow-appeasement".to_string(),
            name: "a .clone()/.to_vec()/.collect() whose ONLY purpose is ending a borrow before a mutating loop over the same owner (the clone's sole consumer reads it after the loop) → one read-then-mutate pass — the pre-state folds inside the loop — or snapshot only the SCALAR the read needs; the clone is pure appeasement: it type-checks, the value IS used, and nightly redundant_clone passes it".to_string(),
            category: "allocation".to_string(),
            buggy_code: r#"struct World { hp: Vec<u64> }

impl World {
    // THE APPEASEMENT: the clone exists ONLY to end the borrow before
    // the mutating loop over the same owner; the snapshot's sole
    // consumer reads it after the loop.
    fn appeased_round(&mut self) -> u64 {
        let snapshot: Vec<u64> = self.hp.clone(); // the appeasement clone
        for v in &mut self.hp {
            *v = v.wrapping_add(1);
        }
        snapshot.iter().fold(0u64, |a, &b| a.wrapping_add(b)) // pre-round sum
    }
}"#.to_string(),
            fixed_code: r#"impl World {
    // THE REMEDY: one read-then-mutate pass — the pre-round sum falls
    // out of the same loop, zero clones, identical final state.
    fn closed_round(&mut self) -> u64 {
        let mut pre = 0u64;
        for v in &mut self.hp {
            pre = pre.wrapping_add(*v);
            *v = v.wrapping_add(1);
        }
        pre
    }
}"#.to_string(),
            context_notes: Some(
                "NotBenchable-by-domain (the win is the surrounding RESTRUCTURE — an architectural class; the clone itself benches as noise next to the loop it guards). THE LAW: a clone whose ONLY purpose is dropping a borrow before a mutating loop over the same owner — the SIGNAL is structural POSITION (clone immediately preceding a loop that mutates through the same &mut self, the clone's sole consumer reading it after the loop). It type-checks, the value IS used, and nightly clippy redundant_clone passes it — the post's own diagnosis: 'known micro-tricks producing a brand-new macro anti-pattern' (the case study carried 175 such sites; the 31 s/video profile ranked the tick-loop clones first). THE REMEDY: one read-then-mutate pass, or snapshot only the scalar the read needs; when the reader is a helper needing &mut self, split the owner — mem-take-field-checkout-empty-window's remedy — the two rules COMPOSE (the split removes both the clone and the take-window). DEDUP ADJUDICATION (T0 at landing HEAD): `avoid-clone` (entries_02) owns the BY-VALUE SIGNATURE axis (fn(data: Vec<T>) -> &[T]); `collect-avoidance` (entries_01) owns chained-iterator intermediates; neither owns the self-aliasing POSITION. REGIME: AI-ported code appeasing the borrow checker at hot-loop scale. ANTI-REGIMES: clones whose consumer OUTLIVES the call (undo logs — load-bearing, the fixture's all-safe arm); two-phase algorithms genuinely needing divergent copies; cold paths where one clone is a boot-frame. Fixtures: corpus/fixtures/b356_fixtures.rs F2 — same pre-round sum + final state, the alloc census (clone arm allocates per call, the closed arm zero), the all-safe undo log, the tampered fold-after-mutation. Source: a post-only r/rust case study (Research 400). Batch 356 / Plan 366.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: the worker set spawned + joined per call ────────
        // THE BENCH LEAD. A per-frame/per-request spawn+join pays thread
        // stack + packet + handle costs every call; a built-once pool
        // amortizes them to a queue round-trip.
        PerfRule {
            rule_id: "thread-spawn-per-call-pool-reuse".to_string(),
            name: "a worker set spawned + joined PER CALL (per frame, per request — 3,840 OS threads for a 900-frame video) pays thread stack, packet and JoinHandle costs every call → build the pool ONCE (fixed workers looping on a job queue with a completion counter) and submit batches; one-shot calls and tiny per-call work stay spawned, and data-parallel iteration over ~5 µs+ per-task work goes to rayon — the pool is for a STANDING per-call worker set, not for parallel-for".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"const WORKERS: usize = 4;

// THE BUGGY ARM: spawn + join the worker set EVERY call — the thread
// stack, packet and handle cost is paid per frame, forever.
fn frame_naive(data: &[u64], workers: usize) -> u64 {
    let per = data.len().div_ceil(workers).max(1);
    std::thread::scope(|s| {
        let handles: Vec<_> = data
            .chunks(per)
            .map(|chunk| s.spawn(move || chunk.iter().fold(0u64, |a, &b| a.wrapping_add(b))))
            .collect();
        handles
            .into_iter()
            .fold(0u64, |a, h| a.wrapping_add(h.join().unwrap()))
    })
}"#.to_string(),
            fixed_code: r#"const WORKERS: usize = 4;

struct Job { data: Arc<Vec<u64>>, start: usize, end: usize, sink: Arc<Mutex<u64>> }
struct Queue { jobs: Mutex<VecDeque<Job>>, cv: Condvar, pending: Mutex<usize>, pending_cv: Condvar }
struct Pool { queue: Arc<Queue> }

fn build_pool(workers: usize) -> Pool {
    let queue = Arc::new(Queue { jobs: Mutex::new(VecDeque::new()), cv: Condvar::new(), pending: Mutex::new(0), pending_cv: Condvar::new() });
    for _ in 0..workers {
        let q = Arc::clone(&queue);
        // the BUILT-ONCE workers: they serve every later call
        std::thread::spawn(move || loop {
            let job = { let mut j = q.jobs.lock().unwrap(); loop { match j.pop_front() { Some(x) => break x, None => j = q.cv.wait(j).unwrap() } } };
            let sum = job.data[job.start..job.end].iter().fold(0u64, |a, &b| a.wrapping_add(b));
            { let mut s = job.sink.lock().unwrap(); *s = s.wrapping_add(sum); }
            let mut p = q.pending.lock().unwrap(); *p -= 1; q.pending_cv.notify_all();
        });
    }
    Pool { queue }
}

static POOL: OnceLock<Pool> = OnceLock::new();

// THE FIXED ARM: a built-once pool — the same work, the spawn cost
// paid once and amortized over every later call. (The job queue +
// completion-counter pair is the minimal std shape; senders/replies
// add a channel alloc per job this design avoids.)
fn frame_pooled(data: &Arc<Vec<u64>>, workers: usize) -> u64 {
    let pool = POOL.get_or_init(|| build_pool(workers));
    let sink = Arc::new(Mutex::new(0u64));
    let per = data.len().div_ceil(workers).max(1);
    {
        let mut q = pool.queue.jobs.lock().unwrap();
        let mut p = pool.queue.pending.lock().unwrap();
        *p = 0; // no job in flight between calls (the caller waited)
        let mut i = 0;
        while i < data.len() {
            let end = (i + per).min(data.len());
            q.push_back(Job { data: Arc::clone(data), start: i, end, sink: Arc::clone(&sink) });
            *p += 1;
            i = end;
        }
    }
    pool.queue.cv.notify_all();
    let mut p = pool.queue.pending.lock().unwrap();
    while *p > 0 { p = pool.queue.pending_cv.wait(p).unwrap(); }
    let total = *sink.lock().unwrap();
    total
}"#.to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (rust_perf_bench validator, 2026-11-12, m3 = Mac15,8 / M3 Max 16-core / 64 GiB / macOS 27.2, AC power, LOAD ~22-28 — the box carried sibling lanes' builds + two seal-viewer renders during the cell; the win's mechanism is thread-construction cost, load-amortized, not a timing-noise class): 4.51x / 1.96x / 10.81x at n=1024/16384/65536 (4 workers, shared-buffer fold), allocs 14 -> 2 and bytes 648 -> 104 at EVERY size (per-call spawn: closure+packet+JoinHandle+collect vs the pool's one sink Arc + a warm queue; the validator's 100-iteration warmup absorbs the OnceLock pool build, so the alloc pass reads the true steady state). The n=16384 dip (1.96x vs 4.51x/10.81x) is the fold work reaching spawn-cost parity at the mid size under that load — the law holds at every size (all Faster), the magnitude widens as per-call work shrinks toward the spawn floor. THE LAW: a worker set spawned + joined PER CALL (the post's rgba_to_i420 spawned a 4-thread scope per frame = 3,840 OS threads per 900-frame video) pays thread stack + packet + handle costs every call; a built-once pool (fixed workers looping on a job queue + completion counter) amortizes them to a queue round-trip. DEDUP ADJUDICATION (T0 at landing HEAD, 3 vocabularies: thread::spawn | worker pool | spawn-per-call): the corpus's thread::spawn hits are the FIXED arms of partitioning/lock rules (entries_04/05/06 parallelization axes) or REGIME notes inside io rules ('tiny inputs where spawn overhead dominates') — no rule owns the CONSTRUCTION-per-call axis; pre-exec-disables-posix-spawn (entries_58) owns process spawn, online-cost-fit-thread-ladder-reopen-hysteresis (entries_61) owns the ADAPTIVE thread-count ladder — this owns the standing pool. REGIME: per-frame/per-request worker sets whose per-call work is small enough that construction cost is visible (the post's draw path; video encoders; per-request fanouts). ANTI-REGIMES: one-shot calls (build == spawn, the fixture's all-safe arm); tiny per-call work at high parallelism (rayon's ~5 us per-task threshold law — data-parallel iteration goes to rayon, not a hand-rolled pool); already-persistent worker infra (a runtime that owns its threads — this rule is for code that constructs them per call). Fixtures: corpus/fixtures/b356_fixtures.rs F3 — equality across sizes x worker counts, the alloc axis (steady pooled 2 < naive 14), the one-shot all-safe arm, the tampered rebuilt-per-call pool caught by the census; TIMING IS NOT ASSERTED IN FIXTURES (box-load flaky) — this validator cell carries it. Source: a post-only r/rust case study (Research 400; NO repo — decision-content-only transcription; magnitudes PROVENANCE ONLY). Batch 356 / Plan 366.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(thread_spawn_per_call_pool_reuse_bench_spec()),
        },
        // ── layout: dense-id HashMap → slot array ────────────────────────
        // Entity/item ids drawn from a small dense domain pay a full
        // hash + probe per lookup; a slot array indexed by id turns the
        // lookup into a bounds-checked index.
        PerfRule {
            rule_id: "dense-id-domain-hashmap-to-slot-array".to_string(),
            name: "entity/item ids drawn from a small DENSE domain (near-total occupancy over [0, N)) pay a full hash + probe per lookup in HashMap<Id, V> → a slot array indexed by id — Vec<V> at full occupancy, Vec<Option<V>> where absence is answerable — turns every lookup into a bounds-checked index; sparse domains and frozen keysets stay mapped".to_string(),
            category: "layout".to_string(),
            buggy_code: r#"use std::collections::HashMap;

// THE BUGGY ARM: the tick hashes every id — SipHash + probe per
// lookup, every frame, for ids that ARE their own index.
fn tick(map: &mut HashMap<u32, u64>) -> u64 {
    let n = map.len() as u32;
    for id in 0..n {
        if let Some(v) = map.get_mut(&id) {
            *v = v.wrapping_add(w(id as usize));
        }
    }
    map.values().fold(0u64, |a, &b| a.wrapping_add(b))
}"#.to_string(),
            fixed_code: r#"// THE REMEDY: the slot array — the id IS the index; one bounds
// check per lookup, zero hashing. (Full occupancy: Vec<V>. Partial:
// Vec<Option<V>> so absence stays an answer. Sparse domains stay
// mapped — the slot array would be a memory blowup there.)
fn tick(ids: &mut Vec<u64>) -> u64 {
    for (id, v) in ids.iter_mut().enumerate() {
        *v = v.wrapping_add(w(id));
    }
    ids.iter().fold(0u64, |a, &b| a.wrapping_add(b))
}"#.to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (rust_perf_bench validator, 2026-11-12, m3 = Mac15,8 / M3 Max 16-core / 64 GiB / macOS 27.2, AC power, LOAD ~22-28 — sibling builds + two seal-viewer renders live; the ratio is hash-vs-index, load-amortized): 70.21x / 62.23x / 75.57x at n=1024/16384/131072 (std SipHash HashMap get tick vs Vec index tick), allocs 0 -> 0 bytes 0 -> 0 at every size (a read-only tick keeps BOTH arms steady-state alloc-free; the BUILD-side alloc axis lives in the fixtures' census — map growth ~13 allocs vs one Vec::with_capacity). The read-only harness shape is deliberate: the steady-state tick that dominates these workloads is reads; get_mut/insert-in-place add no allocation either, the hash cost is the same order. THE LAW: entity/item ids drawn from a small DENSE domain (near-total occupancy over [0, N)) pay a full hash + probe per lookup for keys that ARE their own index; the slot array (Vec<V> at full occupancy, Vec<Option<V>> where absence is answerable) makes every lookup a bounds-checked index. DEDUP ADJUDICATION (T0 at landing HEAD, 3 vocabularies: dense id | slot array | HashMap domain): fxhashmap (entries_01) owns KEEP hashing, cheaper; minimal-perfect-hash-static-lookup (entries_33) owns FROZEN keysets and its own anti-regime names THIS carve unowned ('runtime-MUTABLE keysets -> a real hash map' — the slot array is the mutable dense middle ground neither rule covers); per-scope-map-scan-hash-crossover (entries_18) owns scan-vs-hash crossover; dense-table-bulk-update-via-contiguous-iter (entries_06) owns bulk updates over an ALREADY-contiguous table; flat-pool-start-len-static-rows (entries_33) owns static-row layout; entries_31's NonMaxUsize owns the Option-slot TYPE representation (compose: the slot array's Vec<Option<V>> shrinks via that type); entries_71's 'slot array' mention is a running-hash prose line (read and ruled distinct). REGIME: entity/monster/item registries, table-style maps keyed by small dense ids — the AI-ported case study's 19-class OOP world flattened into id-keyed maps. ANTI-REGIMES: SPARSE domains (the slot array is a memory blowup — the fixture's numeric regime boundary: 16 live ids over a 1M domain cost 100x+ the map's bytes); huge domains where cold slots thrash cache worse than hashing; FROZEN keysets (minimal-perfect-hash is faster still); ids needing stable handles across compaction (generational slot maps are a different, richer structure). Fixtures: corpus/fixtures/b356_fixtures.rs F4 — equality + the hash census (the map arm hashes every id, the slot arm zero), the build alloc axis, the sparse anti-regime numeric boundary, the tampered map-in-the-fixed-arm caught by the census. Source: a post-only r/rust case study (Research 400; NO repo — decision-content-only transcription; magnitudes PROVENANCE ONLY). Batch 356 / Plan 366.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(dense_id_slot_array_bench_spec()),
        },
    ]
}

/// R3's bench: spawn/join a 4-worker set per call vs a built-once pool
/// over the same fold. The validator's 100-iteration warmup absorbs the
/// pool build (the `OnceLock` initializes in the fixed arm's first warm
/// call); the steady-state alloc pass then reads ONE call per arm.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness string IS the spec
fn thread_spawn_per_call_pool_reuse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1024, 16384, 65536],
        harness: r#"
const WORKERS: usize = 4;

pub struct Input {
    data: std::sync::Arc<Vec<u64>>,
    expected: u64,
}

pub fn make_input(n: usize) -> Input {
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        v.push(s);
    }
    let expected = v.iter().fold(0u64, |a, &b| a.wrapping_add(b));
    Input { data: std::sync::Arc::new(v), expected }
}

struct Job { data: std::sync::Arc<Vec<u64>>, start: usize, end: usize, sink: std::sync::Arc<std::sync::Mutex<u64>> }
struct Queue { jobs: std::sync::Mutex<std::collections::VecDeque<Job>>, cv: std::sync::Condvar, pending: std::sync::Mutex<usize>, pending_cv: std::sync::Condvar }
struct Pool { queue: std::sync::Arc<Queue> }

fn build_pool(workers: usize) -> Pool {
    let queue = std::sync::Arc::new(Queue { jobs: std::sync::Mutex::new(std::collections::VecDeque::new()), cv: std::sync::Condvar::new(), pending: std::sync::Mutex::new(0), pending_cv: std::sync::Condvar::new() });
    for _ in 0..workers {
        let q = std::sync::Arc::clone(&queue);
        std::thread::spawn(move || loop {
            let job = { let mut j = q.jobs.lock().unwrap(); loop { match j.pop_front() { Some(x) => break x, None => j = q.cv.wait(j).unwrap() } } };
            let sum = job.data[job.start..job.end].iter().fold(0u64, |a, &b| a.wrapping_add(b));
            { let mut s = job.sink.lock().unwrap(); *s = s.wrapping_add(sum); }
            let mut p = q.pending.lock().unwrap(); *p -= 1; q.pending_cv.notify_all();
        });
    }
    Pool { queue }
}

fn submit_and_wait(queue: &Queue, data: &std::sync::Arc<Vec<u64>>, workers: usize) -> u64 {
    let sink = std::sync::Arc::new(std::sync::Mutex::new(0u64));
    let per = data.len().div_ceil(workers).max(1);
    { let mut q = queue.jobs.lock().unwrap(); let mut p = queue.pending.lock().unwrap(); *p = 0; let mut i = 0; while i < data.len() { let end = (i + per).min(data.len()); q.push_back(Job { data: std::sync::Arc::clone(data), start: i, end, sink: std::sync::Arc::clone(&sink) }); *p += 1; i = end; } }
    queue.cv.notify_all();
    let mut p = queue.pending.lock().unwrap();
    while *p > 0 { p = queue.pending_cv.wait(p).unwrap(); }
    let total = *sink.lock().unwrap();
    total
}

static POOL: std::sync::OnceLock<Pool> = std::sync::OnceLock::new();

pub fn run_buggy(input: &Input) {
    let data = std::hint::black_box(&*input.data);
    let per = data.len().div_ceil(WORKERS).max(1);
    let got = std::thread::scope(|s| {
        let handles: Vec<_> = data
            .chunks(per)
            .map(|chunk| s.spawn(move || chunk.iter().fold(0u64, |a, &b| a.wrapping_add(b))))
            .collect();
        handles
            .into_iter()
            .fold(0u64, |a, h| a.wrapping_add(h.join().unwrap()))
    });
    debug_assert_eq!(got, input.expected);
    std::hint::black_box(got);
}

pub fn run_fixed(input: &Input) {
    let pool = POOL.get_or_init(|| build_pool(WORKERS));
    let data = std::hint::black_box(&input.data);
    let got = submit_and_wait(&pool.queue, data, WORKERS);
    debug_assert_eq!(got, input.expected);
    std::hint::black_box(got);
}
"#
        .to_string(),
    }
}

/// R5's bench: a read tick over a dense id domain — every id hashed and
/// probed on the map arm, a bounds-checked index on the slot array.
/// Read-only keeps both arms allocation-free at steady state (the
/// build-side alloc axis lives in the fixtures' census, not here).
#[cfg(feature = "rust_perf_bench")]
fn dense_id_slot_array_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1024, 16384, 131_072],
        harness: r#"
pub struct Input {
    map: std::collections::HashMap<u32, u64>,
    vec: Vec<u64>,
    expected: u64,
}

fn w(i: usize) -> u64 {
    (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(0xBF58_476D_1CE4_E5B9) >> 33
}

pub fn make_input(n: usize) -> Input {
    let mut map = std::collections::HashMap::with_capacity(n);
    let mut vec = Vec::with_capacity(n);
    let mut expected = 0u64;
    for i in 0..n as u32 {
        let x = w(i as usize) % 1000 + 1;
        map.insert(i, x);
        vec.push(x);
        expected = expected.wrapping_add(x);
    }
    Input { map, vec, expected }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let n = input.map.len() as u32;
    let mut sum = 0u64;
    for id in 0..n {
        sum = sum.wrapping_add(*input.map.get(&id).unwrap());
    }
    debug_assert_eq!(sum, input.expected);
    std::hint::black_box(sum);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut sum = 0u64;
    for (id, v) in input.vec.iter().enumerate() {
        let _ = id;
        sum = sum.wrapping_add(*v);
    }
    debug_assert_eq!(sum, input.expected);
    std::hint::black_box(sum);
}
"#
        .to_string(),
    }
}

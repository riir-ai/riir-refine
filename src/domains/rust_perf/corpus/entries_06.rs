//! Entries slice 06 of 06 — rules 116–126 of 126 — moved VERBATIM from the
//! former corpus.rs monolith (2026-09-10 split, scripted by
//! `scripts/split_rust_perf_corpus.py`). Append new batches to the
//! LAST slice or open a new one past ~1800 lines.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── Batch 135 (2026-09-09, Plan 114 / Research 155): the Wild-linker
        // "RustForge wild performance tricks" article distill — three rules,
        // all benchable + measured. The article is an unlicensed-format blog
        // post by the Wild linker author — cite-shape only; the fixtures
        // below are original reproductions of the patterns.
        //
        // ── concurrency: permanent atomic storage / transmute → zero-cost
        // representation conversion (the Vec<AtomicT> mirror) ────────────
        // The naive way to get &-aliased random writes into a Vec<T> is to
        // rebuild it as Vec<AtomicT> with Atomic::new per element (fresh
        // allocation + a full copy every conversion), or to transmute, or to
        // keep the data permanently atomic. When size_of + align_of match
        // (AtomicU32 has the layout of u32), consuming the owned vec via
        // into_iter().map(Atomic::new).collect() reuses the SAME heap
        // allocation and the map loop compiles out entirely (the article's
        // x86 asm: a few movups/mov, no loop, no branch) — so switching the
        // representation per phase is free and there is no excuse for either
        // transmute or permanent atomics. Back-conversion must CONSUME via
        // into_inner(), never load per element; core::mem::take moves the
        // vec across the phase boundary without touching the heap (an empty
        // Vec does not allocate).
        PerfRule {
            rule_id: "atomic-vec-zero-cost-convert".to_string(),
            name: "Vec<T> ↔ Vec<AtomicT> conversion by Atomic::new-per-element collect (fresh alloc + full copy) → into_iter().map(Atomic::new).collect() on the OWNED vec — same size+align makes std reuse the buffer and the loop vanish; switch representation per phase instead of paying atomics everywhere or transmuting".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::atomic::AtomicU32;\n\n// Rebuild the atomic mirror from the shared plain vec: the source is only\n// borrowed, so a fresh allocation + an element-by-element copy is forced\n// on EVERY conversion. Teams that need &-aliased writes often keep the\n// data permanently atomic instead — paying atomic costs in every reader\n// — or reach for transmute.\nfn to_atomic(plain: &[u32]) -> Vec<AtomicU32> {\n    plain.iter().map(|&v| AtomicU32::new(v)).collect()\n}".to_string(),
            fixed_code: "use std::sync::atomic::AtomicU32;\n\n// The vec is OWNED, so into_iter() iterates in place; with matching\n// size_of/align_of (AtomicU32 is layout-identical to u32) the collect\n// reuses the SAME heap allocation and the map loop compiles out — the\n// conversion is free, so keep the plain representation for the owned\n// phases and convert only for the shared-write phase. Convert BACK by\n// consuming: into_iter().map(|a| a.into_inner()).collect() — never\n// per-element loads — and move the vec across phases with mem::take.\nfn to_atomic(plain: Vec<u32>) -> Vec<AtomicU32> {\n    plain.into_iter().map(AtomicU32::new).collect()\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0, verdict-stable x2 (M3 Max, release, 2026-09-09 — Batch 135 benches): 1.83x/1.78x/1.57x/1.33x at n=100/1K/10K/100K, alloc axis EXACT 2 → 1 at every size (both arms pay one shared clone; the buggy conversion adds a second malloc + full copy, the fixed one reuses the buffer). REGIME: the conversion is free ONLY when size_of::<T>() == size_of::<AtomicT>() and align matches (std's in-place collect specialization for vec::IntoIter) — u32/i32/u64/i64/usize and their Atomic twins all match; wider/newtype wrappers match when the wrapper is a transparent single-field struct. ANTI-REGIMES: converting a BORROWED source still needs the fresh allocation (the source outlives the result — the fixed form requires ownership); a wrapper with Drop or a different layout falls back to a real copy loop. THE DECISION CONTENT: switch the representation per phase (plain owned storage, atomic only for the shared-write window) instead of keeping data permanently atomic (pays atomic costs in every reader) or transmuting (unsound under noalias). Back-conversion consumes via into_inner() — per-element loads re-copy the buffer for nothing. mem::take moves the vec across the phase boundary without touching the heap. Distinct from mutex-to-atomic (permanent-storage direction: replace a Mutex with one atomic; this switches a WHOLE VEC's representation for a phase, then switches back) and from mem-take (the move idiom this rule uses at the boundary — different mechanism, they compose). Fixtures are original reproductions of the pattern; source: David Lattimore, \"RustForge wild performance tricks\" (davidlattimore.github.io, 2025-09-02) — the Wild linker's symbol-resolution phase conversion, technique 3. Batch 135 / Plan 114 / Research 155.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(atomic_vec_zero_cost_convert_bench_spec()),
        },
        // ── allocation: alloc-per-iteration because the hoisted buffer can't
        // be TYPED → reuse the heap allocation across a lifetime/type change
        // (the reuse_vec pattern) ─────────────────────────────────────────
        // reuse-allocation-clear covers the caller-passed scratch buffer.
        // This rule covers the case that shape CANNOT express: the buffer's
        // element type (or lifetime) differs per iteration — Vec<&'iter T> —
        // so no single borrowed-scratch signature typechecks. The fix keeps
        // the buffer OWNED and retargets it in place: clear() makes the map
        // body unreachable (the loop compiles out), the const size/align
        // assert turns an incompatible T/U into a COMPILE error at the call
        // site, and std's in-place collect preserves the heap allocation.
        PerfRule {
            rule_id: "vec-reuse-across-lifetime-change".to_string(),
            name: "hot loop allocates a fresh Vec<&'iter T> (or any same-layout Vec<U>) every iteration because the hoisted buffer cannot be typed across the element lifetime → own the buffer and retarget it in place with a const size/align assert + clear() + into_iter().map(unreachable).collect() — the allocation is reused, the loop compiles out, incompatible T/U is a compile error".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// Every call allocates: the picked refs live only for this call, so\n// a hoisted Vec<&[u32]> buffer cannot be typed to outlive one call's\n// borrow — the borrow checker rejects every signature that tries.\nfn row_lens(rows: &[Vec<u32>]) -> u64 {\n    let picked: Vec<&[u32]> = rows.iter().map(|r| &r[..]).collect();\n    picked.iter().map(|s| s.len() as u64).sum()\n}".to_string(),
            fixed_code: "// The buffer is OWNED and retargeted per call: the const assert makes\n// a layout-mismatched T/U a compile error; clear() empties the buffer\n// so the map body is unreachable and the loop compiles out; std's\n// in-place collect hands the SAME heap allocation back re-typed.\n// SAFETY CONTRACT (caller-enforced): the stored refs are cleared (or\n// the buffer dropped) before any referent dies — a panic between take\n// and restore only loses the buffer, never reads a dead ref.\nfn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {\n    const {\n        assert!(\n            std::mem::size_of::<T>() == std::mem::size_of::<U>()\n                && std::mem::align_of::<T>() == std::mem::align_of::<U>(),\n        );\n    }\n    v.clear();\n    v.into_iter().map(|_| unreachable!()).collect()\n}\n\nfn row_lens(rows: &[Vec<u32>], buf: &mut Vec<&'static [u32]>) -> u64 {\n    let mut cur: Vec<&[u32]> = reuse_vec(std::mem::take(buf));\n    cur.clear();\n    cur.extend(rows.iter().map(|r| &r[..]));\n    let total: u64 = cur.iter().map(|s| s.len() as u64).sum();\n    *buf = reuse_vec(cur);\n    total\n}".to_string(),
            context_notes: Some("MEASURED (M3 Max, release, 2026-09-09 — Batch 135 benches, verdict-stable x2): **Mixed 2/0/2 timing (1.49x@100 / 1.07x@1K / Tie@10K/100K) + alloc axis EXACT 1 → 0 at every size** — the deterministic alloc axis is the load-bearing cell (the elision is proven by integers, not wall-clock); timing ties once the refill dominates the malloc (the reuse-allocation-clear Mixed shape — its L2 note covers the large-N regime). REGIME: same-layout element types ONLY — the const assert (inline const, stable 1.79) fires at MONOMORPHIZATION, so a mismatched T/U is a compile error at the call site, not a runtime panic; &T → &'static T retargeting is the canonical case (8-byte slices, references, Box-backed handles, NonNull). THE SAFETY CONTRACT is the rule's load-bearing content: the 'static element type is only sound because the buffer is cleared before any referent can die and consumers never observe a cleared slot — the caller owns the contract, the compiler only checks the LAYOUT half; on panic between take and restore the buffer is dropped (freed), never read dead. Distinct from reuse-allocation-clear (a caller-passed &mut Vec<T> scratch of ONE fixed element type — the shape that cannot typecheck when the element lifetime differs per iteration; that rule removes the compiler-can't-hoist obstacle, this one the borrow-checker-can't-type obstacle) and from mem-take (the boundary move this pattern uses). EXTENSION ARM (recorded, not landed): types that mix owned + borrowed fields (Foo<'a> {owned: String, borrowed: &'a str}) cross the same barrier via a MaybeUninit<&'static> shadow-struct field-wise map — sound only while the shadowed field is never observed empty (the article's own panic caveat). Suggestion-only (a parse-only proposer cannot prove the caller contract). Fixtures original; source: David Lattimore, \"RustForge wild performance tricks\" (davidlattimore.github.io, 2025-09-02) — technique 4, the Wild linker's buffer reuse. Batch 135 / Plan 114 / Research 155.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(vec_reuse_across_lifetime_change_bench_spec()),
        },
        // ── concurrency: shared Vec under one Mutex (or atomics everywhere)
        // → partition into disjoint &mut slices, ZERO synchronization ─────
        // When each worker's writes provably land in a DISJOINT contiguous
        // range (the Wild linker's per-object symbol-id ranges), a lock
        // serializes work that cannot conflict. split the buffer once and
        // hand each worker its own &mut [T]: no locks, no atomics, no
        // contention — the writes are ordinary stores.
        PerfRule {
            rule_id: "partitioned-mut-slices-zero-sync".to_string(),
            name: "shared Vec mutated through one Mutex (or atomics) although each worker writes a known DISJOINT contiguous range → partition once (chunks_mut / split_at_mut) and hand each worker its own &mut [T] — zero locks, zero atomics; applies only when the access pattern is provably disjoint".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::Mutex;\n\n// One lock guards the whole array: the workers' index ranges never\n// overlap, but every single bump serializes on the Mutex — the sync\n// buys nothing the partition would not give for free.\nfn bump_all(grid: &Mutex<Vec<u64>>, cells: usize, workers: usize, iters: usize) {\n    std::thread::scope(|s| {\n        for w in 0..workers {\n            s.spawn(move || {\n                for cell_idx in (w..cells).step_by(workers) {\n                    for _ in 0..iters {\n                        let mut g = grid.lock().unwrap();\n                        g[cell_idx] = g[cell_idx].wrapping_add(1);\n                    }\n                }\n            });\n        }\n    });\n}".to_string(),
            fixed_code: "// Partition ONCE into disjoint contiguous ranges and give each\n// worker exclusive &mut ownership of its slice: the writes are plain\n// stores — no lock, no atomic, no contention, and the per-cell result\n// is identical (each cell is bumped by exactly one owner).\nfn bump_all(grid: &mut Vec<u64>, workers: usize, iters: usize) {\n    let per = grid.len().div_ceil(workers);\n    std::thread::scope(|s| {\n        for chunk in grid.chunks_mut(per) {\n            s.spawn(move || {\n                for _ in 0..iters {\n                    for cell in chunk.iter_mut() {\n                        *cell = cell.wrapping_add(1);\n                    }\n                }\n            });\n        }\n    });\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0, verdict-stable x2 (M3 Max, release, 2026-09-09 — Batch 135 benches; sizes = THREAD counts per the align-64 precedent): 6.90x/9.12x/11.81x/6.13x at 1/2/4/8 threads — the per-bump Mutex serializes even UNCONTENDED (a lock/unlock per bump is ~20ns against a plain store), and contention widens it; alloc axis EXEMPT (see bench.rs ALLOC_AXIS_EXEMPT): the contended std Mutex allocates nondeterministically on macOS in the pthread slow path — the Batch-78 probe evidence covers the same platform class. THE DECISION CONTENT is the disjointness proof: the transform is legal exactly when each worker's writes land in a known contiguous range (contiguous id ranges per object — the Wild linker shape; per-thread partitions of an image/row band; per-shard cells). ANTI-REGIMES: scattered/overlapping access patterns (use fine-grained locks or atomics), read-share-write-rare workloads (RwLock), or when the partition itself costs more than the contention (tiny arrays). Composes with vec-with-capacity (reserve before partition) and reuse-allocation-clear (persistent per-worker buffers). Distinct from cache-line-block-partitioned-lock (that REDUCES contention by partitioning LOCK GRANULARITY — still locks, for scattered per-item writes; this ELIMINATES synchronization via ownership partition for provably-ranged writes — the two answer different access patterns and can compose: partition into worker regions, block-partition the remainder) and from parallel-disjoint-pread-hides-cold-page-latency (io positional reads on a shared fd; this is shared-MEMORY mutation). Fixtures original, std-only (thread::scope + chunks_mut — no rayon dep); the article's split_off_mut form is nightly-shaped and the portable split_at_mut/chunks_mut core is what lands. Source: David Lattimore, \"RustForge wild performance tricks\" (davidlattimore.github.io, 2025-09-02) — technique 1. Batch 135 / Plan 114 / Research 155.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(partitioned_mut_slices_zero_sync_bench_spec()),
        },
        // ── Batch 136 (2026-09-09, Plan 115 — the Lattimore "Graph Algorithms
// in Rayon" blog distill): dynamic work (graph traversal — work unknown
// in advance) on a rayon pool. All three rules are NotBenchable — the
// source is a QUALITATIVE approach comparison with NO measured numbers,
// and the author's own closing verdict is the decision content.
// ── concurrency: dynamic-work parallelism ladder ────────────────────
        PerfRule {
            rule_id: "dynamic-work-parallelism-ladder".to_string(),
            name: "parallel work whose amount is UNKNOWN IN ADVANCE (graph traversal from roots) needs a STRUCTURE decision, not a perf decision: scoped spawning is the composition-safe default despite the per-task heap boxing — pick by composition requirements first, allocation second; spawn_broadcast + private job control starves the pool (parked threads are unavailable to rayon), and a channel + par_bridge queue does not compose with borrowed work".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "// The team picked the queue structure for ALLOCATION reasons (rayon\n// documents scoped spawning as 'more expensive' — it heap-boxes each\n// task), then the algorithm grew: two traversal jobs need to combine,\n// and one work item must split an owned value into borrowed sub-work.\n// Neither fits the channel shape: the borrows cannot cross work items,\n// so the queue's WorkItem enum accretes every job's variants and the\n// jobs end up hand-inlined into one monster enum.\nenum WorkItem {\n    Visit(Node),\n    ProcessChunk(/* borrowed from an earlier item? impossible */),\n}".to_string(),
            fixed_code: "// Composition first: scoped spawning keeps rayon's work-stealing\n// available to every task (nested par_iter works), borrows cross\n// freely within the scope, and jobs compose by spawning, not by enum\n// accretion. The per-task heap boxing is the PRICE of that default;\n// pay it until profiling proves the allocations matter.\nuse rayon::Scope;\n\nfn explore_all(roots: &[Node]) {\n    rayon::scope(|s| {\n        for node in roots {\n            s.spawn(|s: &Scope<'_>| explore(node, s));\n        }\n    });\n}\n\nfn explore(node: &Node, s: &Scope<'_>) {\n    // nested par_iter INSIDE the traversal works — the pool is not\n    // privately captured;\n    for child in node.children() {\n        s.spawn(|s| explore(child, s));\n    }\n}".to_string(),
            context_notes: Some("QUALITATIVE LADDER, source-shipped (the author's own measured trade: 'a channel works fine for simple tasks and avoids the heap allocations, but it really doesn't compose at all well' — he RETURNED to scoped spawning; honestly NotBenchable — the decision content is the ladder, not a speed claim). THE LADDER for work-unknown-in-advance parallelism: (1) spawn_broadcast + own job control — COMPLEX and STARVES the pool: the thread that would run a nested par_iter finds all other threads parked in private job control, unavailable to rayon; (2) channel + par_bridge — alloc-light but deadlocks under nested par_iter (see nested-par-inside-channel-bridge-deadlocks) and fails composition (an owned value created in one work item cannot be split into borrowed work items — the borrow checker refuses; two queue jobs needing sequenced borrows cannot combine); (3) SCOPED SPAWNING — the composition-safe default; rayon's 'more expensive' doc warning refers to the per-task heap boxing, which is the price of stealable, composable tasks. THE DECISION CONTENT: choose by composition first, allocations second — the doc warning is the trap. FUTURE axis (recorded, not a rule): async/await detaches tasks from threads, so a steal-then-block would resume elsewhere — the author speculates, has not tried. Related upstream note: rayon try_for_each_init runs init per WORK ITEM, not per thread — per-thread arenas multiply. Distinct from partitioned-mut-slices-zero-sync (KNOWN disjoint work — this is UNKNOWN work) and from channel-send-batching-unpark (producer-side send batching on a FIXED worker set — this is queue STRUCTURE for dynamic work). Fixtures original (rayon + a Node shape). Source: David Lattimore, \"Graph Algorithms in Rayon\" (davidlattimore.github.io, 2025-11-27). Batch 136 / Plan 115.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: the dynamic-queue shutdown idiom ────────────────
        PerfRule {
            rule_id: "sender-per-work-item-terminates-channel".to_string(),
            name: "a dynamic work queue over an unbounded channel terminates only when the LAST sender drops — seed each work item with a CLONE of the send end, drop the ORIGINAL after seeding, and let completion be implicit: when the final item is processed its sender clone drops and the receiver's iterator ends".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::sync::mpsc;

// The seed loop keeps the original sender alive in the spawning
// closure, and work items have NO way to enqueue children — so the
// traversal terminates only because it cannot express discovered
// work. Add child-discovery and the receiver never sees a close:
// the original sender outlives the loop inside the worker's closure
// capture, and the for-iterator blocks forever on the next recv.
fn walk(roots: Vec<Node>) {
    let (tx, rx) = mpsc::channel();
    for r in roots {
        tx.send(r).unwrap();
    }
    for node in rx {                       // blocks: tx never drops
        for child in node.children() {     // ...and cannot enqueue them
            // tx.send(child) — tx is not reachable here
        }
    }
}".to_string(),
            fixed_code: "// Every work item CARRIES a clone of the sender; the original is
// dropped after seeding. Discovering children is just sending from
// the clone the item holds, and when the LAST item finishes, the
// last clone drops — the channel closes and the par_bridge/enumerate
// ends by construction. No sentinel, no count, no close() call.
fn walk(roots: Vec<Node>) {
    let (tx, rx) = crossbeam_channel::unbounded();
    for r in roots {
        tx.send(Work::Visit(r, tx.clone())).unwrap();
    }
    drop(tx); // the seed's copy must go, or nothing ever closes
    for Work::Visit(node, tx) in rx {
        for child in node.children() {
            tx.send(Work::Visit(child, tx.clone())).unwrap();
        }
    } // last sender dropped inside the loop — iteration ends here
}".to_string(),
            context_notes: Some("IDIOM RULE, source-shipped (the working example is the post's channel+par_bridge approach; honestly NotBenchable — it is the CORRECTNESS/termination shape, not a speed claim). THE DECISION CONTENT: termination of a dynamically-discovered work queue must be OWNERSHIP-DRIVEN — 'the channel closes when the last work item drops its sender clone' replaces sentinel values, atomic counters + an explicit close, or a separate done-flag thread. The drop(tx) after seeding is LOAD-BEARING: forgetting it hangs the receiver forever (the classic bug this idiom exists to prevent). Regime: unbounded channels feeding par_bridge or a plain for-over-rx loop where work is DISCOVERED during processing (graph reachability, transitive deps, crawl frontiers). ANTI-REGIMES: fixed work known upfront (just par_iter over the collection — no channel needed); producer/consumer THROUGHPUT shaping (that is bounded-channel-backpressure / channel-send-batching-unpark — different axis entirely); cancellation mid-queue needs an explicit flag, ownership close alone cannot signal abort. Distinct from nested-par-inside-channel-bridge-deadlocks (the FAILURE mode of this same queue shape under nested rayon work — the idiom is correct; the hazard is mixing it with par_bridge's steal behavior). Source: David Lattimore, \"Graph Algorithms in Rayon\" (davidlattimore.github.io, 2025-11-27). Batch 136 / Plan 115.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── concurrency: nested par_iter inside par_bridge work = deadlock ─
        PerfRule {
            rule_id: "nested-par-inside-channel-bridge-deadlocks".to_string(),
            name: "a nested par_iter inside par_bridge-fed work can DEADLOCK: the inner parallel iterator steals a task from another worker, blocks pulling the NEXT item from the same channel, and the work item it interrupted still holds a sender clone — rayon tasks are THREAD-TIED, so the blocked stealer never releases it and the channel never closes".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use crossbeam_channel::unbounded;
use rayon::prelude::*;

// The queue feeds work out; each item ALSO runs a nested parallel
// section. One worker's inner par_iter steals a task from another
// thread; that stolen task's own for_in_rx.next() blocks waiting for
// MORE work — but the interrupted work item holds a sender clone, so
// the channel cannot close. Thread-tied tasks wait on themselves:
// four workers, zero progress, no panic — just a silent hang.
fn drain(rx: crossbeam_channel::Receiver<Item>, tx: crossbeam_channel::Sender<Item>) {
    for item in rx.into_iter().par_bridge() {
        let parts = split(item);
        parts.par_iter().for_each(|p| process(p)); // steals ...
        let _ = tx.clone(); // ... while this clone is still held
    }
}".to_string(),
            fixed_code: "// Keep nested parallel sections OUT of bridge-fed items: either run
// the inner work SEQUENTIALLY inside queue items (the queue already
// parallelizes across items), or hand the nested section its own
// pre-materialized inputs so it cannot block on the queue it feeds
// from. If nested rayon work is unavoidable per item, switch the
// whole job to scoped spawning — stealable scope tasks resume on
// other threads instead of pinning a worker.
fn drain(rx: crossbeam_channel::Receiver<Item>, tx: crossbeam_channel::Sender<Item>) {
    for item in rx {                       // sequential recv loop
        for p in split(&item) {
            process(p);                    // inner work inline, no steal
        }
    }
    drop(tx);
}".to_string(),
            context_notes: Some("CORRECTNESS LEAD, source-explained (the mechanism paragraph is the post's own subtle-case walkthrough; honestly NotBenchable — a deadlock is not an A/B axis). THE DEADLOCK CHAIN, each link load-bearing: (1) par_bridge workers PULL from the channel, and rayon's par_iter may STEAL the pull task from a blocked worker's thread; (2) the stolen pull BLOCKS on rx.recv() for the next item; (3) the channel stays open because the work item that was interrupted still holds its SENDER clone; (4) rayon tasks are thread-TIED — the blocked stealer never resumes far enough to drop anything. Result: silent hang, no panic. REGIME: any queue shape where work items hold channel senders AND their processing runs nested rayon collectors. FIXES in order: serialize the inner section (the queue already parallelizes), pre-materialize inner inputs before the parallel section, or move the whole traversal to scoped spawning (composition-safe default — see dynamic-work-parallelism-ladder). FUTURE axis: async tasks are NOT thread-tied, so steal-then-block would migrate — the author's speculation, unimplemented. Distinct from nested-parallelism-deadlock notes in rayon docs (that warns scoped spawning is expensive; this is the channel-queue interaction the docs do not name) and from sender-per-work-item-terminates-channel (the correct shutdown idiom this hazard breaks). Fixtures original. Source: David Lattimore, \"Graph Algorithms in Rayon\" (davidlattimore.github.io, 2025-11-27). Batch 136 / Plan 115.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── Batch 141 (2026-09-09, Plan 121 — the antirez/ds4 Part-B walk,
// Research 162's staged batch; executed after the Issue-085 dist lane
// landed). Three decision-content rules from the agent/context layer of
// ds4 @ 6289c516 — the same host-side slice shape as Batches 121–125.
// All NotBenchable: the decision content is budget/boundary/stream
// discipline, not an A/B axis.
// ── allocation: reserve measured from the thing actually written ────
        PerfRule {
            rule_id: "compaction-reserve-before-prompt-build".to_string(),
            name: "the reserve for a destructive context rewrite is MEASURED from the real payload, never a constant guess: build the actual prompt, tokenize it, and derive reserve = measured_tokens + slack + output_budget — then lift EVERY free-threshold check to max(threshold, reserve) so no check can fire inside the space the rewrite itself needs".to_string(),
            category: "allocation".to_string(),
            buggy_code: "// The compaction trigger uses a hand-picked constant for how much\n// room the summary exchange needs, while the actual summary prompt is\n// built later from a template that grew over months. When the template\n// crosses the constant, the rewrite fires with less headroom than the\n// exchange itself consumes: the summarizer's own prompt overflows the\n// window mid-rewrite, and the session dies inside the recovery path.\nconst COMPACT_RESERVE_TOKENS: usize = 2_048;\n\nfn should_compact(ctx: &Context) -> bool {\n    ctx.len >= ctx.window - COMPACT_RESERVE_TOKENS\n}\n\nfn compact(ctx: &mut Context) -> Summary {\n    // the REAL cost: prompt template + chat framing + summary budget\n    let prompt = build_summary_prompt(ctx);\n    let cost = prompt.len() + SUMMARY_BUDGET;\n    let _ = cost; // never checked against the threshold above\n    summarize(ctx, &prompt)\n}".to_string(),
            fixed_code: "// The reserve is MEASURED: build the real prompt once, tokenize it,\n// and add the fixed slack + the output budget. Every threshold derives\n// from the same measured figure, so the rewrite can never fire with\n// less headroom than the exchange consumes.\nfn compact_reserve(window: usize) -> usize {\n    // the REAL prompt, not an estimate of it\n    let prompt = build_summary_prompt_for_measure();\n    let measured = tokenize(&prompt).len();\n    measured + 64 /* framing slack */ + summary_budget(window)\n}\n\nfn should_compact(ctx: &Context) -> bool {\n    let reserve = compact_reserve(ctx.window);\n    ctx.len >= ctx.window.saturating_sub(reserve)\n}\n\nfn room_for_generation(ctx: &Context) -> usize {\n    // the SAME reserve bounds every other consumer of the headroom\n    ctx.window.saturating_sub(ctx.len) - compact_reserve(ctx.window)\n}".to_string(),
            context_notes: Some("DECISION-CONTENT, source-shipped (the reserve function BUILDS the real prompt to measure it: 'agent_compact_make_prompt(...)' + chat framing appends, then 'reserve = tokens.len + 64 + agent_compact_summary_budget(ctx)' — and BOTH free-threshold consumers derive from it: 'compact = agent_compact_reserve_tokens(w) + 128' and 'generation_room = room - agent_compact_reserve_tokens(w)'; honestly NotBenchable — control-flow sizing, not an A/B axis). THE DECISION CONTENT: a capacity reserve for a DESTRUCTIVE rewrite must be measured from the thing actually written — tokenizing the real prompt — never a hand-picked constant; the reserve quantity is a tokenization of the actual payload, one level above reserve-before-insert (vec-with-capacity reserves by element COUNT of a KNOWN collection; this reserves by MEASURING an input whose size is only known by building it). The reserve must be the SINGLE figure every headroom consumer reads (trigger, generation room, mid-rewrite re-checks) — two figures for the same headroom is the constant-drift failure this rule exists to kill. Composes with renderer-error-is-not-capacity-overflow (B123, same source file: the tri-state validator decides WHETHER to compact; this decides HOW MUCH room the compaction keeps — the destructive handler is gated by the tri-state, and its budget comes from here). Regime: any context/window manager whose rewrite cost includes a prompt/template that lives in code and drifts — chat compaction, log rotation summaries, cache-rebuild planners. ANTI-REGIME: the payload size is statically known (just size_of/count it — no measurement needed). Distinct from vec-with-capacity (count-known collection; this is measure-the-real-input), from the B123 tri-state rule (classification vs budget, same pipeline), and from length-prefix-validate-before-alloc (sec — that bounds UNTRUSTED input before allocation; this measures OWNED template cost before a threshold). Suggestion-only (a parse-only proposer cannot see that the constant and the measured cost can diverge). Fixtures original. Source: antirez/ds4 @ 6289c516 (MIT) — ds4_agent.c:9354-9364 (the reserve = measured prompt + 64 + summary_budget), :6537/:9273/:9602/:10010 (the four call sites: compact budget, free-threshold, generation room). Batch 141 / Plan 121 / Research 162.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── io: truncation boundaries are semantic, not arithmetic ─────────
        PerfRule {
            rule_id: "compaction-tail-boundary-prefers-complete-turn".to_string(),
            name: "a lossy truncation boundary is SEMANTIC: walk back over a bounded extra budget to the most recent COMPLETE unit (a whole user turn), fall back to the arithmetic target only when no complete unit fits, and NEVER split an atomic span (an image/embedding block snaps the boundary to the span's START) — a slightly longer complete unit beats a fragment that loses the constraints".to_string(),
            category: "io".to_string(),
            buggy_code: "// The tail keeps exactly tail_budget tokens: an arithmetic slice.\n// When the budget lands mid-turn, the surviving fragment has our\n// answer without the user's constraints that prompted it — and when\n// it lands mid-image, the summary consumes a torn token span that\n// neither renders nor embeds.\nfn tail_start(tokens: &[Token], sys_len: usize, tail_budget: usize) -> usize {\n    tokens.len().saturating_sub(tail_budget).max(sys_len)\n}\n\nfn compact(tokens: &[Token], images: &[Span], sys_len: usize) -> Vec<Token> {\n    let start = tail_start(tokens, sys_len, 2_048);\n    let _ = images; // spans are not considered at the boundary\n    tokens[start..].to_vec()\n}".to_string(),
            fixed_code: "// Two semantic gates on the same boundary: (1) walk back up to one\n// extra budget to the latest COMPLETE user turn — a slightly longer\n// complete turn is preferable to losing the user's constraints and\n// keeping only a fragment of our answer; (2) if the chosen position\n// falls strictly inside an atomic span, snap to the span's START.\nfn tail_start(\n    tokens: &[Token], images: &[Span], sys_len: usize, tail_budget: usize,\n) -> usize {\n    let bottom = tokens.len();\n    let mut target = bottom.saturating_sub(tail_budget).max(sys_len);\n    // complete-turn walk: [bottom - 2*budget, bottom) — the floor holds\n    let earliest = bottom.saturating_sub(2 * tail_budget).max(sys_len);\n    for i in (earliest..bottom).rev() {\n        if tokens[i].is_user_turn_start() {\n            target = i;\n            break;\n        }\n    }\n    snap_out_of_spans(target, images) // never sever an atomic span\n}\n\nfn snap_out_of_spans(mut pos: usize, spans: &[Span]) -> usize {\n    for s in spans {\n        if pos > s.start && pos < s.end {\n            pos = s.start; // the span survives whole or not at all\n        }\n    }\n    pos\n}".to_string(),
            context_notes: Some("DECISION-CONTENT, source-shipped (the boundary walk + the verbatim rule: 'A slightly longer complete turn is preferable to losing the user\'s constraints and keeping only a fragment of our answer' — the walk scans [bottom - 2*tail_budget, bottom) floor sys_len for the latest user-turn token and returns the arithmetic target as fallback; the image gate: 'if (pos > start && pos < end) return start' — strictly-inside snaps to the span START; honestly NotBenchable — boundary semantics, not an A/B axis). THE DECISION CONTENT: lossy truncation boundaries are SEMANTIC, not arithmetic — (1) prefer a complete UNIT over an exact budget (the walk-back window is bounded at 2× the budget, the floor at the protected prefix, the fallback the arithmetic target, so the scan terminates and the budget can only be exceeded, never the protected prefix severed); (2) ATOMIC SPANS gate the boundary regardless of how it was chosen — a boundary strictly inside a span moves to the span start (spans that survive whole or not at all). Regime: any lossy truncate-and-summarize over linear content with turn/span structure — chat compaction, log buffers, transcript windows, retrieval chunking. ANTI-REGIME: content with no internal unit structure (a raw byte stream has no complete-turn to prefer — the arithmetic slice IS the semantics). Distinct from tri-state fits validation (B123 — WHETHER to compact; this WHERE the boundary lands), from compaction-reserve-before-prompt-build (Batch 141 — HOW MUCH headroom; this WHICH content survives), from bounded-gap read coalescing (B94 — merges READS by gap; this places a WRITE-side truncation boundary), and from streaming-prefix-holdback (Batch 141 — the OUTPUT side of the same never-tear-a-unit discipline: holdback defers emission, this moves the cut). Suggestion-only (a parse-only proposer cannot see turn/span structure). Fixtures original. Source: antirez/ds4 @ 6289c516 (MIT) — ds4_agent.c:9286-9300 (agent_compact_tail_boundary + the verbatim preference comment), :9367-9377 (agent_compact_image_boundary strictly-inside snap), :9411/:9560 (both consumers gate their boundaries through the snap). Batch 141 / Plan 121 / Research 162.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── io: streaming matchers hold back O(pattern), never O(stream) ───
        PerfRule {
            rule_id: "streaming-prefix-holdback-bounded-buffer".to_string(),
            name: "a streaming output matcher that must recognize a marker BEFORE the marker's bytes reach the wire holds back AT MOST the marker's own length in a fixed buffer, matches only at the gated position (a prose line boundary), and flushes the held bytes verbatim on the first mismatch — ordinary text streams immediately, and no unbounded pending window ever exists".to_string(),
            category: "io".to_string(),
            buggy_code: "// The decorator wants to style hint markers, so it buffers the\n// stream until it KNOWS the line is ordinary prose. A long code line\n// (or a chatty generator) builds a megabyte pending window before\n// the newline that finally resolves it — latency and memory both\n// scale with the LINE, not with the marker.\n#[derive(Default)]\nstruct Decorator {\n    pending: Vec<u8>, // grows until a newline arrives\n}\n\nimpl Decorator {\n    fn feed(&mut self, b: u8, out: &mut Vec<u8>) {\n        self.pending.push(b);\n        if b == b'\\n' {\n            if self.pending.starts_with(b\"> **Hint:**\") {\n                out.extend_from_slice(b\"[hint]\");\n                out.extend_from_slice(&self.pending[10..]);\n            } else {\n                out.extend_from_slice(&self.pending); // held far too long\n            }\n            self.pending.clear();\n        }\n    }\n}".to_string(),
            fixed_code: "// Match the marker INCREMENTALLY: hold at most marker.len() bytes,\n// compare one byte per feed at the gated position (line start, not\n// inside code blocks), and on the first mismatch replay the held\n// bytes verbatim. Ordinary text is delayed by at most marker.len(),\n// and a real marker is recognized at its LAST byte — no line-length\n// window ever exists.\n#[derive(Default)]\nstruct Decorator {\n    held: [u8; 11], // the bound IS the marker: > **Hint:** is 11 bytes\n    held_len: usize,\n}\n\nconst MARKER: &[u8] = b\"> **Hint:**\";\n\nimpl Decorator {\n    fn feed(&mut self, b: u8, out: &mut Vec<u8>) {\n        if self.held_len < MARKER.len() && b == MARKER[self.held_len] {\n            self.held[self.held_len] = b;\n            self.held_len += 1;\n            if self.held_len == MARKER.len() {\n                out.extend_from_slice(b\"[hint]\"); // matched at the last byte\n                self.held_len = 0;\n            }\n            return;\n        }\n        // first mismatch (or the gate declined): flush what we held...\n        out.extend_from_slice(&self.held[..self.held_len]);\n        self.held_len = 0;\n        out.push(b); // ...then the current byte streams too\n    }\n}".to_string(),
            context_notes: Some("DECISION-CONTENT, source-shipped (the bound IS the type: 'char md_hint_prefix[sizeof(\"> **Hint:**\") - 1]' with a parallel len; the gate: 'Recognize only the advertised hint marker at a prose line boundary. Hold at most that short prefix; ordinary text still streams immediately. Explicit quote lines continue the aside. No width-dependent repainting.'; the mismatch path replays the held bytes byte-by-byte before feeding the current one; the match path fires at the marker's LAST byte with no lookahead beyond it; honestly NotBenchable — streaming-latency discipline, not an A/B axis). THE DECISION CONTENT: a stream recognizer's pending window is bounded by the PATTERN, never by the line/stream — the bound lives in the TYPE (a fixed array sized by sizeof(marker)), the match is incremental (one byte per feed against the next pattern byte), the GATE (line boundary / not-in-code-block) decides whether matching is even armed, and mismatch flushes verbatim so ordinary bytes are delayed O(pattern) worst-case and usually O(1). Regime: decorating/transforming a live stream (LLM token output, log tails, protocol framing) where a marker must be caught before its bytes render. ANTI-REGIMES: the whole-line transformation is genuinely needed (buffer deliberately — but then the bound is the documented line cap, not an accident); the marker can be rewritten after the fact (post-pass over a stored buffer — no holdback needed). Distinct from bounded-channel-backpressure (queue DEPTH bounds; this is a PATTERN-sized deferral inside one stream), from compaction-tail-boundary-prefers-complete-turn (Batch 141 — the truncation side of never-tear-a-unit; this is the emission side), and from buffered-io (a fixed throughput window chosen for syscall amortization; the holdback exists for RECOGNITION and is sized by the pattern it matches). Suggestion-only (a parse-only proposer cannot see the marker semantics). Fixtures original. Source: antirez/ds4 @ 6289c516 (MIT) — ds4_agent.c:225-226 (the sized buffer + len), :3294-3299 (the boundary-gate comment), :3301-3311 (incremental match, fires at the last byte), :3317-3325 (mismatch flushes the held prefix verbatim). Batch 141 / Plan 121 / Research 162.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── Batch 143 (2026-09-10, Plan 123 — the nnethercote Dec-2025
// rustc-perf post distill; the PERF-SEARCH intake row staged at cf9e44ae).
// Two rules from the compiler-perf trenches: one classic runtime gate
// (benchable — the win is measured, not asserted), one build-time
// emission discipline (annotated NotBenchable — compile-time content).
// ── complexity: the computation a disabled log call still pays ────
        PerfRule {
            rule_id: "debug-level-gated-computation".to_string(),
            name: "a hot-path value computed into a binding whose ONLY consumer is a debug!/trace! call runs on every iteration even with the level off — the binding is a real statement and the level check is a runtime atomic load LLVM cannot const-fold (without release_max_level_* features), so gate the computation itself with log_enabled! (or move it into the macro's lazy format args) instead of computing first and logging after".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// The tick loop builds a per-event summary for a debug! that is\n// effectively always off in production (no logger configured). The\n// summary binding is a REAL statement: it executes on every event\n// even though the macro drops it — the level check happens inside\n// the macro expansion, AFTER the computation already ran.\nfn tick(events: &[Event], stats: &mut Stats) {\n    for e in events {\n        apply(e, stats);\n        let summary = format!(\"{:?}\", summarize(e)); // paid every event\n        debug!(\"event summary: {summary}\");\n    }\n}".to_string(),
            fixed_code: "// The computation lives INSIDE the level gate: with the level off\n// (or no logger) the summary is never built; with the level on, both\n// arms emit identical records. Moving the computation into the\n// macro's format args is the equivalent lazy form.\nfn tick(events: &[Event], stats: &mut Stats) {\n    for e in events {\n        apply(e, stats);\n        if log_enabled!(log::Level::Debug) {\n            let summary = format!(\"{:?}\", summarize(e));\n            debug!(\"event summary: {summary}\");\n        }\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 verdict-stable (batch143_pareto: 2184-2534x at 100-10K items, alloc axis EXACT 2200/22000/220000 → 0 — with no logger installed the gated arm builds ZERO summaries; the magnitude is the whole log-only computation's share, not a representative tick: real-world speedup = the summary cost ÷ the whole loop body). SOURCE-CITED (nnethercote, \"How to speed up the Rust compiler in December 2025\", 2025-12-05, #147293 verbatim: 'In the query system there was a value computed on a hot path that was only used within a debug! call. In this PR I avoided doing that computation unless necessary, which gave icount reductions across many benchmark results, more than 3% in the best case. This was such a classic micro-optimization that I added it as an example to the Logging and Debugging chapter of the The Rust Performance Book' — the doubled 'The' is in the original). THE MECHANISM: log's max level is a RUNTIME atomic (AtomicUsize), so LLVM cannot const-fold the disabled macro branch — a binding computed on its own line and consumed only by the macro's enabled-arm is still a real statement that executes every iteration; log_enabled! reads the SAME atomic but gates the computation itself, and moving the computation into the macro's format args is the equivalent lazy form (log macros evaluate args only on the enabled path). MEASUREMENT-HONESTY RIDER (same post, #143684): the LLVM-21 bump measured mean icount −1.70% and cycles −0.90% while mean wall-time went UP 0.26% — 'Wall-time is the true metric, because it's what users perceive' — third-party confirmation of this corpus's wall-clock GOAT discipline: an instruction-count A/B can invert on wall; never promote on icount alone. Regime: hot loops (tick handlers, per-request paths, query kernels) whose diagnostic summaries are built unconditionally beside their log calls. ANTI-REGIMES: the value has ANY other consumer (compute once, log freely — the binding is live); the crate enables release_max_level_* (the branch is compiled out either way — the fix is moot); the level is compile-time static (cfg'd-out logging). Distinct from log-write-direct (the ALLOCATION of formatting — push_str(&format!) vs write! into the buffer; this is whether the formatting COMPUTATION runs at all), from debug-stat-full-scan-out-of-hot-path (that diagnostic IS consumed every call and moved out; this one is consumed only when a level is on and gated in place), from debug-assert-length-hoist (assertion ERASURE changes optimization — the opposite direction: there the check must run, here it must not). Fixtures original. Source: nnethercote.github.io Dec-2025 post (#147293 + #143684), the mechanism home the perf-book Logging and Debugging chapter. Batch 143 / Plan 123 / Research 164.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(debug_level_gate_bench_spec()),
        },
        // ── complexity: build-time content the compiler deletes anyway ────
        PerfRule {
            rule_id: "macro-generated-code-optimize-at-generation-point".to_string(),
            name: "code a proc macro emits should be trimmed and deduped AT the generation point — where the emitter's context makes redundancy visible — not left for downstream optimization machinery to reconstruct: measure the emitted volume with -Zmacro-stats, then hoist invariant emission out of per-field loops, drop dead blocks, and dedupe repeated bounds once per impl instead of once per method".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// The derive emitter writes, for EVERY field: a no-op const block,\n// the trait bound repeated per method, and a closure that only\n// forwards to a call. rustc parses, type-checks, and optimizes the\n// redundancy away — every compile, every crate, forever.\nfn emit_reflect_impl(fields: &[Field]) -> String {\n    let mut out = String::new();\n    for f in fields {\n        out.push_str(\"const _: () = ();\\n\");            // dead in every impl\n        out.push_str(&format!(\"fn get_{n}(v: &{t}) -> &{t} where {t}: Reflect, {t}: Reflect {{\n\", n = f.name, t = f.ty));\n        out.push_str(&format!(\"    |v: &{t}| v.field()\\n}}\\n\", t = f.ty)); // closure forwards only\n    }\n    out\n}".to_string(),
            fixed_code: "// The impl's shared parts are emitted ONCE; per field, only what\n// actually varies. Same generated semantics — a fraction of the\n// tokens for rustc to parse, check, and represent in memory.\nfn emit_reflect_impl(fields: &[Field]) -> String {\n    let mut out = String::new();\n    out.push_str(\"fn dispatch(v: &Self, idx: usize) -> &Field { ... }\\n\"); // once\n    for f in fields {\n        // per field: only the accessor table entry\n        out.push_str(&format!(\"    /* {} => slot {} */\\n\", f.name, f.idx));\n    }\n    out\n}".to_string(),
            context_notes: Some("HONESTLY NotBenchable, build-time content (upstream-measured, no runtime A/B axis — the corpus benches RUNTIME loops; a proc-macro emitter's cost is cargo-check wall and peak memory on CRATES that use the macro, not a runnable fixture pair). SOURCE (nnethercote, \"How to speed up the Rust compiler in December 2025\", 2025-12-05 section 'Proc macro wins in Bevy', quotes verbatim): bevy_ui was \"around 16,000 lines and 563,000 bytes of source code\" while its #[derive(Reflect)] output was \"around 27,000 lines and 1,544,000 bytes\" — \"Macro expansion almost quadrupled the size of the code, mostly because of this one macro!\"; after the cleanup PRs, \"[t]he size of the code generated by #[derive(Reflect)] was reduced by 39%, which reduced cargo check wall-time for that crate by 16%, and peak memory usage by 5%\". THE LOAD-BEARING SENTENCE: \"it is more efficient to optimize away redundancies at the generation point, where context-specific and domain-specific information is available, rather than relying on sophisticated optimization machinery further down the compilation pipeline that has to reconstruct information. And it's just less code to parse and represent in memory.\" THE INSTRUMENT: -Zmacro-stats (introduced June 2025 — \"How much code does that proc macro generate?\", same blog; measure BEFORE and AFTER, per crate). DECISION CONTENT: an emitter's per-field redundancy (bounds repeated per method, dead const blocks, forwarding closures) is invisible in source and enormous in expansion; the emitter KNOWS the field set at generation and can hoist/dedupe what downstream must rediscover. Regime: derive-like proc macros over large crates, build-heavy workspaces, compile-time-sensitive CI. ANTI-REGIMES: handwritten code (the compiler's optimizer IS the right tool there — no generation point exists); emission already at the measured floor. Suggestion-only (a parse-only proposer cannot see macro emission volume; -Zmacro-stats is nightly-only). Fixtures original. Batch 143 / Plan 123 / Research 164.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: per-call residency-probe ROUTING (the probe picks the arm) ──
        PerfRule {
            rule_id: "mincore-probe-routes-gather-arm".to_string(),
            name: "a gather over a large memory-mapped table pays warm-pass cost or cold-pass cost depending on page residency the program cannot see — route PER CALL with a sampled mincore(2) probe (fixed-stride row samples, worst-of-all-maps) whose ~0.5 ms cost decides a ~165 ms warm pass: high residency takes the vectorized fancy-index arm with NO warm pass, low residency takes the pooled-pread arm, and any probe error fails safe to the pread arm".to_string(),
            category: "caching".to_string(),
            buggy_code: "// The gather always pays the warm pass: threaded os.pread over every
// unique row to fault the pages in, then fancy-indexes the mmap. On a
// page-warm table the warm pass IS the cost (~165 ms of GIL-contended
// pread per 32,768 rows against a 0.44 ms fancy index) — but the code
// cannot know the table is warm, so it prefaults every call.
fn gather_rows(fd: i32, mmap: &[f32], rows: &[usize], pool: &Pool) -> Vec<f32> {
    // unconditional warm pass — the syscall count IS the cost
    pool.submit_all(rows.iter().map(|&r| pread_row(fd, r)));
    rows.iter().map(|&r| mmap[r]).collect() // 0.44 ms once warm
}".to_string(),
            fixed_code: "// The probe picks the arm PER CALL: ~1.9 us per sampled mincore, 256
// fixed-stride samples (~0.5 ms total) — an answer that must not move
// run to run. Worst-of-all-maps (the gather reads every map, so one
// cold map is a cold gather). Threshold 0.99: at 1% cold the fancy
// index pays ~650 serial faults (~40 ms) with the GIL held, so only
// essentially-full residency earns the vectorized arm. Any probe
// error answers pread — a false cold costs the shipped path, a false
// warm stalls the generation thread.
const RESIDENT_FRACTION_THRESHOLD: f64 = 0.99;

fn gather_rows(fd: i32, maps: &[&[f32]], rows: &[usize], pool: &Pool) -> Vec<f32> {
    let worst = maps.iter().map(|m| mincore_resident_fraction(m))
        .min().unwrap_or(0.0); // probe error -> 0.0 -> pread arm
    if worst < RESIDENT_FRACTION_THRESHOLD {
        pool.submit_all(rows.iter().map(|&r| pread_row(fd, r))); // pooled pread
    }
    // warm arm skips the pass entirely; the fancy index follows either way
    rows.iter().map(|&r| maps[0][r]).collect()
}".to_string(),
            context_notes: Some("SUGGESTION-ONLY, honestly NotBenchable here (host-side Python in the source; an mmap + page-cache-state fixture cannot run hermetic in the bench harness — the decision content is the ROUTING contract, not a Rust rewrite; NotBenchable-with-a-metric-mismatch per Issue 014's vocabulary). MEASURED figures (source-recorded, M5 Max): '~1.9 us per probe... so 256 probes is ~0.5 ms against the 165 ms warm pass it decides'; threshold rationale: 'At 1% cold the fancy index pays ~650 serial faults (~40 ms) with the GIL held; below that the pread pool is strictly better, and taking it is never wrong'; the regime gap is real: 'a prewarmed table probes at 1.00, a cold one at 0.00; there is no realistic middle'. ASYMMETRY: 'A false \"cold\" costs nothing (it takes the shipped pread path); a false \"warm\" is the expensive mistake, so the threshold is high and an unavailable/erroring probe answers pread' — fail-safe direction is load-bearing. SAMPLING DISCIPLINE: 'The sample is a fixed stride through it rather than a random draw: the answer must not move run to run for the same gather, or an A/B could not attribute a delta to the code instead of to the sampler.' PROBE THE WORST MAP: 'Every map is probed, and the WORST answers: the gather reads all three, so one cold map is a cold gather.' SMALL-GATHER INVERSION (anti-regime): below the hot-row threshold the ratio inverts, 'so small gathers keep the shipped path outright' — do not probe when the gather is small. PLATFORM CAVEAT (encoded; from the Batch-139 mining instrument note, NOT in-source): mincore on macOS reports FILE-WIDE page-cache residency of the backing file, not the task's mapped pages — a residency answer can reflect OTHER processes' warmth; treat macOS mincore routing as advisory and re-validate the threshold before shipping there. Distinct from `parallel-disjoint-pread-hides-cold-page-latency` (B118 — OVERLAPS cold page-ins with other work at gather time; here the probe PICKS between arms before any work), from the Batch-139 PLE-prefault admission rule (EAGERLY faults pages at admission gated on RAM headroom — a one-time decision; this is PER-CALL routing of an existing gather; the source's cold-arm bandwidth figures ride its own `mmap-willneed-unwired` measurement label: demand-faulting flat at 1.40 GiB/s vs pooled pread saturating at 12.9 GiB/s — a source measurement, not a corpus rule). Never a prefetch mandate: the pread arm touches exactly the rows the gather needs, ascending ('Reading the rows themselves in ASCENDING order is what the kernel can actually coalesce'). Source: youssofal/MTPLX @ a5e38bb7 (PR #391, Apache-2.0) — mtplx/ple_row_gather.py:93-103 (probe cost + threshold), :122-196 (the ctypes mincore probe + fixed-stride sampling), :199-216 (worst-of-maps arm selection + fail-safe), :34-37 (the asymmetry), mtplx/models/qwen4_exp.py:4888-4900 (the caller routing + counters). Batch 145 / Plan 130 / Research 171.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── Batch 146 (2026-09-16, Plan 132 / Research 173): the accumulated
        // read-lane pack — nnethercote "How to speed up the Rust compiler
        // in July 2026" (2026-07-31, nnethercote.github.io) + bevy 0.19
        // release notes (2026-06-19, bevy.org), both re-verified against
        // the live posts at admission (coordinator reads, not the charter
        // text). All four rules are SUGGESTION-ONLY — a parse-only
        // proposer cannot see handler emptiness, allocation-site heat,
        // engine-owned layout, or shader-side layout. They document the
        // restructure with measured provenance; they never fire
        // mechanically. Fixtures original.

        // ── complexity: multi-handler dispatch pipelines pay for EMPTY handlers ──
        PerfRule {
            rule_id: "dispatch-pass-fusion-skips-empty-handlers".to_string(),
            name: "a visitor pipeline that calls every handler's check_foo on every node pays N virtual dispatches of which the vast majority enter empty bodies (most handlers implement one or two methods) — fuse the passes so only IMPLEMENTED handlers are registered per node kind: the win is the skipped calls plus the un-poisoned branch predictor, not any single call's devirtualization".to_string(),
            category: "complexity".to_string(),
            buggy_code: "// Hundreds of handlers, each implementing a few dozen possible visitor
// methods — but the walk calls EVERY handler's check_foo on EVERY node.
// Each call is a virtual dispatch, and most are no-ops: the callee's
// body is empty and returns immediately. The dispatch itself is the
// cost, and one polymorphic site mixing hundreds of mostly-empty
// targets poisons the branch predictor at every node.
trait NodeVisitor {
    fn check_item(&mut self, _item: &Item) {}
    fn check_stmt(&mut self, _stmt: &Stmt) {}
    fn check_expr(&mut self, _expr: &Expr) {}
}

fn walk(items: &[Item], stmts: &[Stmt], exprs: &[Expr], visitors: &mut [Box<dyn NodeVisitor>]) {
    for item in items {
        for v in visitors.iter_mut() {
            v.check_item(item); // no-op for the vast majority of visitors
        }
    }
    for stmt in stmts {
        for v in visitors.iter_mut() {
            v.check_stmt(stmt); // no-op again
        }
    }
    for expr in exprs {
        for v in visitors.iter_mut() {
            v.check_expr(expr); // expressions dominate node count
        }
    }
}".to_string(),
            fixed_code: "// Registration builds per-node-kind tables of ONLY the implemented
// handlers; the walk dispatches over table entries — an empty
// handler is never called at all, so its vtable slot, its no-op body,
// and its share of the predictor's target mix are all gone.
struct HandlerTables<'a> {
    item: Vec<Box<dyn FnMut(&Item) + 'a>>,
    stmt: Vec<Box<dyn FnMut(&Stmt) + 'a>>,
    expr: Vec<Box<dyn FnMut(&Expr) + 'a>>,
}

fn walk(items: &[Item], stmts: &[Stmt], exprs: &[Expr], t: &mut HandlerTables) {
    for item in items {
        for h in t.item.iter_mut() {
            h(item); // every call enters a real body
        }
    }
    for stmt in stmts {
        for h in t.stmt.iter_mut() {
            h(stmt);
        }
    }
    for expr in exprs {
        for h in t.expr.iter_mut() {
            h(expr);
        }
    }
}".to_string(),
            context_notes: Some("SUGGESTION-ONLY, honestly NotBenchable (a parse-only proposer cannot see which handlers are no-ops — handler emptiness is semantic, not syntactic; the rule documents the restructure, never fires mechanically). SOURCE (nnethercote, \"How to speed up the Rust compiler in July 2026\", 2026-07-31, section Clippy, quotes verbatim): \"Clippy traverses the AST and HIR and for each node it calls the appropriate check_foo method for every lint on that node. Each call is a virtual dispatch. Critically, most of these calls are no-ops because most lints only implement a small number of check_foo methods (often just one).\" The merged fix (xmakro #17124/#17132): \"found a way to combine the passes so that the empty check_foo methods (i.e. the vast majority) are not called\"; the unmerged twin (#157762, functionally equivalent, more detailed measurements): \"in most cases this reduced Clippy's runtime by 10-30%! Branch mispredictions were reduced by 20-80% on real-world examples and by 97% on one stress test. The cost of virtual dispatch adds up if you do it a lot.\" MECHANISM (both halves load-bearing): (1) the skipped CALLS — handlers x node kinds minus the implemented pairs is nearly all waste; (2) the branch predictor — removing the mostly-empty targets un-poisons the sites that remain. Distinct from `dyn-to-enum` (entries_01 — devirtualizes ONE hot dispatch site through an enum, changing the call SHAPE; this restructures a MULTI-HANDLER pipeline so empty handlers are never registered, changing the call COUNT) and from the two-data-passes fusion rule (entries_01 — fuses passes over the same DATA to cut traffic; this fuses DISPATCH passes to cut calls). SHAPE IN OUR STACK: the healer's own syn visitor family (one visitor per domain over dozens of check methods, most empty per domain) and any ECS-style per-entity system dispatch where systems opt into a sparse handler/method matrix. REGIME: many handlers x many node kinds with a sparse implement-count matrix. ANTI-REGIME: a pipeline where every handler implements every method (the tables degenerate to the original calls; registration overhead is pure cost). Fixtures original. Batch 146 / Plan 132 / Research 173.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── layout: single-threaded streaming CAPACITY — fit the hot node under the line class ──
        PerfRule {
            rule_id: "hot-alloc-struct-fits-cache-line-class".to_string(),
            name: "a high-count allocation-site struct whose element size sits just above a cache-line class boundary (72/80 B over 64) streams zero whole elements per line through the hottest loop — shrink/reorder under the boundary, or move the rare arms' cold fields to a side table / box the cold tail; the win is elements-per-line capacity, measured >10% wall and -29% cache misses on rustc's own AST".to_string(),
            category: "layout".to_string(),
            buggy_code: "// The hot node type, streamed millions of times per parse/walk:
// 8 (kind+pad) + 8 (span) + 8 (lhs+rhs) + 8 (payload) + 24 (scratch
// Vec — used by 3 of 20 kinds) + 24 (extra array — used by 2 of 20)
// = 80 bytes: a whole 64-byte line plus 16, so every stream straddles
// lines and no line holds a whole second node.
struct HotNode {
    kind: u8,
    span: (u32, u32),
    lhs: u32,
    rhs: u32,
    payload: u64,
    scratch: Vec<u8>,  // rare-arm only, drags 24 B into every node
    extra: [u8; 24],   // rare-arm only, another 24 B
}".to_string(),
            fixed_code: "// The rare arms' cold fields move to a side table keyed by node
// index, consulted only when the rare kind is hit: the inline node
// drops to 8 + 8 + 8 + 8 = 32 bytes — two whole nodes per line where
// previously zero fit. (The rustc sequence for its AST expression
// nodes: 104 -> 72 -> 64 bytes, \"small enough to fit in a cache
// line\" — the companion PR moved removed data \"temporarily to the
// side\".)
struct HotNode {
    kind: u8,
    span: (u32, u32),
    lhs: u32,
    rhs: u32,
    payload: u64,
}

struct HotNodeSide {
    scratch: Vec<u8>,
    extra: [u8; 24],
}".to_string(),
            context_notes: Some("SUGGESTION-ONLY, honestly NotBenchable (a parse-only proposer cannot see allocation-site counts or loop heat; size_of is computable but the WIN needs the hot-iteration scale — the rule documents the lever and the boundary arithmetic). SOURCE (nnethercote, July 2026 post, section AST, #158720, verbatim): \"I shrank the size of AST expression nodes from 72 bytes to 64 bytes. (Small enough to fit in a cache line.) Expressions are by far the most common AST node kind, and this change reduced wall-time on a small number of AST-heavy benchmarks, some by more than 10%. Cache miss rates on those same benchmarks fell by as much as 29%.\" The paving companion #158942: shrank nodes \"by removing a field. The removed data, when needed, is now stored temporarily to the side\" — the side-table lever this rule generalizes. The trajectory quote worth keeping: \"I remember a few years ago when AST expression nodes were 104 bytes. Good performance often comes from chip, chip, chipping away at things over the long term.\" MECHANISM: single-threaded streaming CAPACITY — how many elements of the hottest allocation site fit per coherence line as the hot loop walks them. Distinct from `adjacent per-thread atomic counter slots -> #[repr(align)]` (entries_02 — prevents FALSE SHARING between cores on adjacent written slots; this packs read-streamed nodes into fewer lines, no contention involved), from `hot-byval-move-over-memcpy-threshold` (entries_03 — the 128-byte LLVM memcpy threshold for MOVED values at call boundaries; this is the 64-byte line class for STREAMED allocations), and from the AoS->SoA split (entries_03 — restructures data ownership into per-field arrays for one hot field; this keeps the struct whole and shrinks it under the boundary). REGIME: allocation-site count x hot-iteration scale in the millions (rustc's AST); a struct just above a line-class boundary with cold rare-arm fields to move or box. ARCH CAVEAT (the memcpy rule's ARM note, same class): Apple Silicon coherence lines are 128 B (`hw.cachelinesize`) — the boundary class moves, the arithmetic does not; measure size_of against the TARGET's line size before claiming the win. ANTI-REGIME: small populations (thousands, not millions — the boundary is irrelevant); structs already under the boundary. Fixtures original. Batch 146 / Plan 132 / Research 173.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── bevy: engine-owned table layout the ordinary iterator hides ──
        PerfRule {
            rule_id: "dense-table-bulk-update-via-contiguous-iter".to_string(),
            name: "a bulk-UNIFORM update through the ordinary bevy Query iterator hands the loop one entity's components at a time even though table storage is a contiguous per-component array — contiguous_iter_mut hands the whole table slice to the loop so LLVM auto-vectorizes (bevy's own 10k-entity table: 5.58 -> 4.88 us scalar, 1.87 us AVX2, 1.58 with change-detection bypass); legal ONLY on dense queries: table storage, no Changed/Added filters, uniform body — and our own view-layer benches measured random-access shapes NEGATIVE".to_string(),
            category: "bevy".to_string(),
            buggy_code: "// A bulk-uniform update through the ordinary iterator: bevy's table
// storage IS a contiguous array per component, but iter_mut() hands
// the loop one entity's components at a time — the compiler cannot
// see the underlying table and cannot vectorize.
fn apply_velocity(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in query.iter_mut() {
        position.x += velocity.x;
        position.y += velocity.y;
        position.z += velocity.z;
    }
}".to_string(),
            fixed_code: "// contiguous_iter_mut hands the whole table slice to the loop: LLVM
// sees dense arrays and auto-vectorizes. ONLY legal when the query is
// DENSE — the three gates are API contract, not advice: (1) default
// table storage for every fetched component (returns Err otherwise);
// (2) NO Changed<T>/Added<T> filters (archetypal With/Without are
// fine) — they force a per-entity check that makes raw table slices
// impossible; (3) a UNIFORM per-entity body — the SIMD premise.
fn apply_velocity(mut query: Query<(&mut Position, &Velocity)>) {
    for (position, velocity) in query.contiguous_iter_mut().unwrap() {
        for (p, v) in position.iter_mut().zip(velocity) {
            p.x += v.x;
            p.y += v.y;
            p.z += v.z;
        }
    }
}".to_string(),
            context_notes: Some("SUGGESTION-ONLY, honestly NotBenchable (+ ENGINE-GATED: bevy_ecs >= 0.19 — the contiguous_iter/_mut API landed there, PRs #21984/#24181; a parse-only proposer cannot see engine version or storage density, and the density contract is what makes the rule safe). SOURCE (bevy 0.19 release notes, Contiguous Query Access, verbatim): \"Table components in Bevy are already laid out flat in memory — all Transform components are stored as values in a contiguous table, exactly what SIMD wants. The Query iterator just wasn't exposing that structure\"; \"contiguous_iter and contiguous_iter_mut hand you the whole table slice at once. LLVM can see the contiguous array and auto-vectorize — or you can reach for explicit SIMD yourself.\" Measured table (10,000-entity position += velocity, bevy's own numbers): normal 5.58 us / 5.51 AVX2; contiguous 4.88 / 1.87; contiguous + bypass_change_detection() 4.40 / 1.58 — and \"While change detection is a generally useful feature, it does incur measurable performance overhead.\" DENSITY GATES (all three from the API contract, all load-bearing): (1) default TABLE storage for every fetched component — the methods return Err otherwise; (2) NO `Changed<T>`/`Added<T>` filters — \"they require a per-entity check that makes it impossible to return raw table slices\" (archetypal With/Without are fine); (3) a UNIFORM per-entity body — the SIMD premise. DUAL-EVIDENCE HONESTY (the anti-regime witness, OUR OWN measurement): riir-ai Bench 930 / Issue 960 measured the view layer's real hot paths NEGATIVE — snapshot-driven get_mut random access does not stream, the one legal contiguous site measured 9-10% SLOWER, best-case 0.13 us/pass at N=1000; the rule fires ONLY on the bulk-uniform-update loop shape (bevy's table regime), never on mixed/random-access sites, and cites 960 as the witness. Distinct from the AoS->SoA split (entries_03 — restructures data OWNERSHIP into per-field arrays; this exploits engine-owned table layout that already exists, zero restructuring, engine-version-gated). REGIME: dense tables, uniform bodies, 10k+ entity bulk passes (physics integrators, transform propagation, decay sweeps). ANTI-REGIME: snapshot/random-access-driven sites (the 960 negative), sparse archetypes, per-entity branching (breaks uniformity AND usually breaks density). Fixtures original. Batch 146 / Plan 132 / Research 173.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── layout: a verified-identical layout makes the per-frame repack the identity transform ──
        PerfRule {
            rule_id: "gpu-upload-repack-skipped-when-layout-matches".to_string(),
            name: "a hot per-frame GPU upload that runs a serialize/repack pass over data whose #[repr(C)] layout already equals the target (WGSL uniform/storage block) pays a no-op transform every frame — when the layout is verified-stable (repr(C), static sizes, a pinned layout test), a direct byte copy (bytemuck::cast_slice / extend_from_slice) is bit-identical and skips the conversion entirely; the verification precondition is the rule's other half, a layout drift is a correctness bug the copy cannot see".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Per-frame upload: a layout-preparing serializer walks every field
// and writes bytes one member at a time — even though the Rust
// struct's layout is byte-identical to the WGSL uniform block it
// targets. The repack is the identity transform, paid every frame.
fn upload_uniforms(device: &Device, params: &WaterParams, dst: &mut UploadBuffer) {
    let mut writer = LayoutWriter::for_uniform_block(); // encase-like
    writer.write_vec4(&params.sun); // serialize per field, per frame
    writer.write_vec4(&params.sun_color);
    writer.write_f32(params.ambient);
    writer.write_f32_array(&params.waves);
    device.write_buffer(dst, 0, writer.as_bytes());
}".to_string(),
            fixed_code: "// The layout is VERIFIED-STABLE: #[repr(C)] + static sizes + a
// const size assertion + a pinned unit test against the WGSL block
// (re-run on struct or shader change). The whole upload is then one
// direct copy — bit-identical bytes, zero conversion work per frame.
#[repr(C)]
struct WaterParams {
    sun: [f32; 4],
    sun_color: [f32; 4],
    ambient: f32,
    waves: [f32; 8],
}

const _: () = assert!(std::mem::size_of::<WaterParams>() == 4 * 4 + 4 * 4 + 4 + 8 * 4);

fn upload_uniforms(device: &Device, params: &WaterParams, dst: &mut UploadBuffer) {
    device.write_buffer(dst, 0, bytemuck::cast_slice(std::slice::from_ref(params)));
}".to_string(),
            context_notes: Some("SUGGESTION-ONLY, honestly NotBenchable (a parse-only proposer cannot verify that the Rust layout equals the shader-side layout — the precondition is semantic; the rule documents the elision AND the precondition, never fires mechanically). SOURCE (bevy 0.19 release notes, Render Big Scenes Faster!, the encase bullet, verbatim): \"Bevy uses the encase library to prepare data for GPU buffers in the layout WGSL expects. In practice, this isn't always needed and incurs unnecessary overhead. We've swapped to cheaper direct memory copies when clustering lights to shave of a bit of time.\" — an engine-internal instance of the general form: a per-frame upload path running a serialize/repack pass over data whose layout already equals the target's. MECHANISM: when the source is #[repr(C)] with static sizes and the target layout is pinned, the repack is the IDENTITY transform — a direct byte copy is bit-identical and removes per-field writing entirely. Distinct from the consumption-ordered repack family (the matvec tile repack, entries_05 — those PAY a one-time repack to make the consumer's reads contiguous; this DELETES a per-frame repack that transforms nothing) and from zero-copy borrow rules (entries_02 — avoiding copies across API boundaries; this avoids a TRANSFORM, the copy remains). REGIME: hot per-frame uploads (uniform pushes, instance tables, staging buffers) at frame-dominating counts. THE PRECONDITION IS THE RULE'S OTHER HALF: a layout drift is a correctness bug the direct copy cannot see — encode the verification (repr(C), size_of const-assert, a pinned layout test against the shader block). SHAPE IN OUR STACK: seal-remake/riir-shader uniform uploads, riir-gpu buffer staging. ANTI-REGIME: cold uploads (the repack is noise, the assertion burden is not worth it); dynamically-laid-out targets (variable-length arrays, runtime offsets — no static layout to match). Fixtures original. Batch 146 / Plan 132 / Research 173.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── Batch 147 (2026-09-16, Plan 131 / Research 172): the staged agave
        // distill — five rules from anza-xyz/agave @ c95d87063 (Apache-2.0),
        // all fixtures original reproductions of the patterns.
        //
        // ── allocation: hot alloc/free cycle around fixed-shape batch ────
        // objects → bounded recycle pool: take/put with a HARD CAP on ────
        // retained capacity ─────────────────────────────────────────────
        // Source: TWO in-source instances of the same decision.
        // (1) scheduler_common.rs:21 `const MAX_RECYCLED_BATCHES: usize =
        // 1024;` + the `recycled_batches` field: take_batch pops a parked
        // triple or allocates fresh; the return path drains ids/
        // transactions and clears max_ages (:262-263, :288), then parks the
        // empty-but-sized triple ONLY below the cap (:289-293) — the 1025th
        // returned batch is dropped. (2) recvmmsg.rs:28
        // `PacketBufferPool(Vec<BytesMut>)`: "Buffers that got used for a
        // packet leave the pool, so it must be replenished periodically";
        // `replenish()` "will allocate only as many buffers as were
        // consumed since last call, so it is idempotent" — and the bound is
        // STRUCTURAL: "we never need to worry about pool being >
        // PACKETS_PER_BATCH since this is the only place where we add
        // elements to it". Decision content: a hot alloc/free cycle around
        // FIXED-SHAPE work objects keeps a recycle pool whose bound is the
        // load-bearing half — unbounded retention is a leak by design (a
        // producer burst grows the pool monotonically and nothing ever
        // shrinks it), so THE CAP IS THE DECISION, enforced either
        // structurally (fixed-width replenish + take-out-only-on-success:
        // "only take it out once it holds a packet, so that no error path
        // can forget to put it back") or explicitly (len check before
        // push). Win = allocator round-trips + header bytes eliminated per
        // cycle. Distinct from kernel_opt B123
        // `batch-pool-read-makes-serial-readahead-redundant` (an IO READ
        // pool whose decision DELETES a redundant read-ahead hint — this is
        // an ALLOCATION recycle pool bounding ALLOCATOR traffic; no
        // absorption either direction), from `reuse-allocation-clear`
        // (entries_02 — ONE caller-owned scratch buffer cleared across
        // calls; no ownership transfer, no retention cap), from
        // `vec-reuse-across-lifetime-change` (ONE buffer hoisted across a
        // lifetime barrier; no pooling, no cap), from `mem-take` (the
        // single-move primitive this pattern composes take/put around), and
        // from `smallvec-stack` (SMALL collections onto the stack; these
        // buffers are page-to-KiB scale heap objects where stack storage is
        // impossible). Suggestion-only (a parse-only proposer cannot prove
        // the alloc/free cycle is hot, the object shape is fixed, or the
        // chosen cap fits the workload's burst profile).
        PerfRule {
            rule_id: "bounded-batch-buffer-pool".to_string(),
            name: "hot alloc/free cycle around fixed-shape batch buffers → bounded recycle pool (take()/put() with a HARD cap on retained capacity; top-up allocates only what left; ownership stays with the pool until success)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "const TARGET: usize = 64;\n\nstruct Work {\n    key: u64,\n    val: u64,\n    stamp: u32,\n}\n\n// every cycle allocates a FRESH buffer triple and drops it at cycle\n// end — 3 malloc/free round-trips per cycle, forever, even though\n// every batch has the same fixed shape\nfn cycle(work: &[Work]) -> u64 {\n    let mut keys: Vec<u64> = Vec::with_capacity(TARGET);\n    let mut vals: Vec<u64> = Vec::with_capacity(TARGET);\n    let mut stamps: Vec<u32> = Vec::with_capacity(TARGET);\n    for w in work {\n        keys.push(w.key);\n        vals.push(w.val);\n        stamps.push(w.stamp);\n    }\n    let sum = keys.iter().sum::<u64>()\n        + vals.iter().sum::<u64>()\n        + stamps.iter().map(|&s| s as u64).sum::<u64>();\n    // buffers dropped here — the allocator pays the round-trip\n    sum\n}".to_string(),
            fixed_code: "const TARGET: usize = 64;\n// the HARD bound on retention — without it the pool is a leak by\n// design: a producer burst grows it monotonically and nothing shrinks it\nconst MAX_RECYCLED: usize = 1024;\n\nstruct Work {\n    key: u64,\n    val: u64,\n    stamp: u32,\n}\n\nstruct BatchPool {\n    recycled: Vec<(Vec<u64>, Vec<u64>, Vec<u32>)>,\n}\n\nimpl BatchPool {\n    // pop a parked triple when one exists; fresh only when empty\n    fn take(&mut self) -> (Vec<u64>, Vec<u64>, Vec<u32>) {\n        self.recycled.pop().unwrap_or_else(|| {\n            (\n                Vec::with_capacity(TARGET),\n                Vec::with_capacity(TARGET),\n                Vec::with_capacity(TARGET),\n            )\n        })\n    }\n    // reset to empty (capacity RETAINED) and park ONLY below the cap —\n    // the 1025th returned batch is dropped\n    fn put(&mut self, mut batch: (Vec<u64>, Vec<u64>, Vec<u32>)) {\n        batch.0.clear();\n        batch.1.clear();\n        batch.2.clear();\n        if self.recycled.len() < MAX_RECYCLED {\n            self.recycled.push(batch);\n        }\n    }\n}\n\nfn cycle(work: &[Work], pool: &mut BatchPool) -> u64 {\n    let (mut keys, mut vals, mut stamps) = pool.take();\n    for w in work {\n        keys.push(w.key);\n        vals.push(w.val);\n        stamps.push(w.stamp);\n    }\n    let sum = keys.iter().sum::<u64>()\n        + vals.iter().sum::<u64>()\n        + stamps.iter().map(|&s| s as u64).sum::<u64>();\n    pool.put((keys, vals, stamps));\n    sum\n}".to_string(),
            context_notes: Some("REGIME: hot alloc/free cycles of fixed-shape buffers — packet batches, scheduler batch triples, request/response objects churned per connection or per tick. THE BOUND IS THE DECISION, two enforcement forms, both in-source: STRUCTURAL (recvmmsg — the pool only ever grows via resize_with to the batch width and shrinks only by consumption; take-out-only-on-success so no error path leaks a buffer) and EXPLICIT (scheduler — `if len < MAX_RECYCLED_BATCHES` before push; the return path drains/clears to empty so capacity is retained without stale contents). TOP-UP SHAPE: replenish allocates only what was consumed ('so it is idempotent') — never resize to a target larger than the working width. ANTI-REGIMES: one-shot allocations (no cycle to amortize — the pool adds a struct for nothing); objects with unstable shape (a one-off huge batch parked in the pool pins its oversized capacity until evicted — the cap bounds the damage, the shape assumption bounds the waste); single-threaded single-buffer scratch (that is reuse-allocation-clear's shape — caller-owned &mut, no pool needed). Suggestion-only (a parse-only proposer cannot prove hotness, fixed shape, or a sane cap). MEASURED ParetoDominant 2/0/1 (rust_perf_bench, Batch 147): allocs 48000 → 4 and bytes 20.5 MB → 1568 at the max size (16K cycles x 3 buffers parked at steady state), timing faster on the majority of sizes. Distinct from kernel_opt B123 batch-pool-read-makes-serial-readahead-redundant (IO read-pool / readahead-hint deletion behind a parallel pool — different axis: disk-I/O dedup vs allocator traffic; no absorption either direction), from reuse-allocation-clear (ONE caller-owned scratch, cleared across calls — no ownership transfer, no retention cap), from vec-reuse-across-lifetime-change (ONE buffer hoisted across a lifetime barrier via layout retyping — no pooling, no cap), from mem-take (the single move primitive this pattern composes take/put around), and from smallvec-stack (stack placement of SMALL collections — these buffers are page-to-KiB scale heap objects where stack storage is impossible; the win is allocator round-trips, not placement). Fixtures original. Source: anza-xyz/agave @ c95d87063 (Apache-2.0) — core/src/banking_stage/transaction_scheduler/scheduler_common.rs:21,27,99-106,262-263,288-293; streamer/src/recvmmsg.rs:21-47,63-70,197-198; perf/src/packet.rs:31 (PACKETS_PER_BATCH = 64). Staged agave batch / Plan 131 / Research 172.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(bounded_batch_buffer_pool_bench_spec()),
        },
        // ── io: reading a LENGTH-PREFIXED record allocates a heap buffer ──
        // per record → fixed STACK page buffer first; the length prefix ──
        // gates the rare heap fallback ──────────────────────────────────
        // Source: accounts-db/src/append_vec.rs
        // get_stored_account_meta_callback (:491): ONE fixed stack page
        // `let mut buf = [MaybeUninit::<u8>::uninit(); PAGE_SIZE];` (with
        // the comment "4096 was just picked to be a single page size");
        // read_into_buffer fills the stack slice (fs/src/file_io.rs:120);
        // parse the fixed header from the valid prefix, then the DECISION
        // at :504-506: `remaining_bytes_for_data >= data_len` — the stack
        // read already suffices, parse the payload as a zero-copy ref INTO
        // the stack buffer; else the rare oversized record takes the heap
        // path (:518-537): a second positioned read into a right-sized
        // heap buffer. The same shape ships twice more in the file: the
        // exact-sized twin at :559 ([MaybeUninit<u8>; STORE_META_OVERHEAD],
        // 136 bytes — no fallback needed, the record is fixed-size) and
        // get_account_shared_data at :594 (page-then-heap via
        // Vec::with_capacity + spare_capacity_mut). Decision content: read
        // ONE page into a STACK buffer first; the record's length prefix
        // decides stack-parse (zero heap on the common path) vs heap
        // fallback; the page may over-read into the next record —
        // harmless, only the record's own bytes are used. Anti-pattern
        // paid by the buggy shape: a heap alloc per record even though the
        // overwhelming majority of records fit in one page. Distinct from
        // `smallvec-stack` (a TYPE choice putting small collections on the
        // stack; this is a READ-PATH buffer strategy gated by a length
        // prefix — io category, and the buffer is a fixed page, not a
        // growable collection), from kernel_opt B123 (deletes a redundant
        // read-ahead HINT behind a parallel read pool — no buffering
        // decision at all), and from `bounded-batch-buffer-pool`
        // (allocation recycle of work objects; this is a read strategy —
        // the two COMPOSE: the oversized fallback could draw its heap
        // buffer from a recycle pool). Suggestion-only default with the
        // SAFE form: the simple `[0u8; PAGE]` fixed arm needs no unsafe;
        // the source's `[MaybeUninit; PAGE]` skips the zero-fill but needs
        // unsafe (source SAFETY comments: "read_into_buffer will only
        // write to uninitialized memory" / "we only read the initialized
        // portion") — document the upgrade, suggest the safe form. HOIST
        // the buffer across records when scanning (a fresh `[0u8; PAGE]`
        // per record pays a 4 KiB fill each time — the fill can dominate
        // small records and eat the win). Regime: many small
        // length-prefixed records — append-only files, packet streams, log
        // replay. ANTI-REGIME: records larger than the buffer are common
        // (the fallback pays DOUBLE — the page read AND the heap read);
        // no length prefix to gate on; single huge-record formats.
        PerfRule {
            rule_id: "stack-read-fast-path-length-prefixed-record".to_string(),
            name: "length-prefixed record read heap-allocates per record → MEASURED-NEGATIVE in-memory: a stack page read copies ~16x the bytes (ParetoDominated 0/3/0); adopt ONLY when reads carry per-call syscall/FFI overhead — read ONE fixed stack page first, the length prefix gates the heap fallback".to_string(),
            category: "io".to_string(),
            buggy_code: "const HEADER: usize = 4;\n\nstruct Blob {\n    bytes: Vec<u8>,\n}\n\nimpl Blob {\n    // stands in for pread: fills buf at `offset`, returns bytes read\n    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {\n        let end = (offset + buf.len()).min(self.bytes.len());\n        if offset >= self.bytes.len() {\n            return 0;\n        }\n        let n = end - offset;\n        buf[..n].copy_from_slice(&self.bytes[offset..end]);\n        n\n    }\n}\n\n// heap buffer PER RECORD: one small header read, then allocate exactly\n// data_len and read again — an allocator round-trip for every record\n// even though the overwhelming majority fit in a single page\nfn record_sum(file: &Blob, offset: usize) -> Option<u64> {\n    let mut hdr = [0u8; HEADER];\n    let n = file.read_at(offset, &mut hdr);\n    if n < HEADER {\n        return None;\n    }\n    let data_len = u32::from_le_bytes(hdr) as usize;\n    let mut data = vec![0u8; data_len];\n    if file.read_at(offset + HEADER, &mut data) < data_len {\n        return None;\n    }\n    Some(data.iter().map(|&b| b as u64).sum())\n}".to_string(),
            fixed_code: "const HEADER: usize = 4;\n// one page — covers the header and the typical payload\nconst PAGE: usize = 4096;\n\nstruct Blob {\n    bytes: Vec<u8>,\n}\n\nimpl Blob {\n    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {\n        let end = (offset + buf.len()).min(self.bytes.len());\n        if offset >= self.bytes.len() {\n            return 0;\n        }\n        let n = end - offset;\n        buf[..n].copy_from_slice(&self.bytes[offset..end]);\n        n\n    }\n}\n\n// ONE page read covers header + typical payload; the length prefix\n// decides: fits → parse straight from the stack slice (zero heap);\n// oversized → the rare heap fallback re-reads exactly data_len\nfn record_sum(file: &Blob, offset: usize) -> Option<u64> {\n    // stack, not heap — upgrade to [MaybeUninit<u8>; PAGE] to also skip\n    // the zero-fill (needs unsafe; the safe form is the default)\n    let mut buf = [0u8; PAGE];\n    let n = file.read_at(offset, &mut buf);\n    if n < HEADER {\n        return None;\n    }\n    let data_len = u32::from_le_bytes(buf[..HEADER].try_into().ok()?) as usize;\n    if HEADER + data_len <= n {\n        // the stack read already suffices — parse in place\n        return Some(buf[HEADER..HEADER + data_len].iter().map(|&b| b as u64).sum());\n    }\n    // rare oversized record: heap + positioned read\n    let mut data = vec![0u8; data_len];\n    if file.read_at(offset + HEADER, &mut data) < data_len {\n        return None;\n    }\n    Some(data.iter().map(|&b| b as u64).sum())\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominated 0/3/0 — DEMOTED TO SUGGESTION-ONLY BY THE MEASUREMENT (rust_perf_bench, Batch 147: allocs 100000 → 802 and bytes 29.5 MB → 3.5 MB at max size — the ALLOCATION axis is a real win — but timing LOSES at every size). THE HONEST MECHANISM: the fixture models reads as memcpy (in-memory blob), so the fixed arm's one 4 KiB page read copies ~16x the bytes of the buggy arm's two small reads — the page-fill cost dominates and the timing loses. The pattern's real-world win requires reads to carry a FIXED PER-CALL OVERHEAD (a syscall per pread, a cross-device fetch) that one big read amortizes and two small reads pay twice — a constant the microbench cannot honestly model (adding an artificial constant to force a win would be fabrication). THE RULE SHIPS AS THE ANNOTATED ANTI-REGIME WITNESS (the Issue-960/batch-146 precedent): adopt the stack-page form ONLY when reads cross a syscall/FFI boundary AND records overwhelmingly fit one page; in a pure in-memory/memcpy regime the heap-per-record form measured FASTER — do not apply. SAFETY LADDER (safe form first): `[0u8; PAGE]` needs no unsafe; `[MaybeUninit<u8>; PAGE]` skips the zero-fill but needs unsafe — the source's two SAFETY obligations are exact ('read_into_buffer will only write to uninitialized memory' on write, 'we only read the initialized portion' on read). HOIST the buffer across records when scanning: a fresh `[0u8; PAGE]` per record re-pays a 4 KiB fill each time. TWIN SITES: the exact-sized record needs NO fallback at all (STORE_META_OVERHEAD = 136 — when the shape is constant the length gate disappears). ALSO TRUE AND MEASURED-WINNING: the ALLOCATION axis (1 heap alloc per record → ~1 per 128 oversized) — the composition candidate is bounded-batch-buffer-pool for the oversized tail, not a standalone win. REGIME (where it CAN win): reads are syscalls/FFI, records overwhelmingly ≪ page. ANTI-REGIME (MEASURED): in-memory/memcpy reads — this measurement. Distinct from smallvec-stack (a TYPE choice for small collections; this is a READ-PATH buffer strategy gated by a length prefix — io), from kernel_opt B123 batch-pool-read-makes-serial-readahead-redundant (readahead-hint deletion behind a read pool — no buffering decision), and from bounded-batch-buffer-pool (allocation recycle; COMPOSES — the oversized fallback's heap buffer could be pooled). Fixtures original. Source: anza-xyz/agave @ c95d87063 (Apache-2.0) — accounts-db/src/append_vec.rs:53,183,491-549,559,594-649; fs/src/file_io.rs:120. Staged agave batch / Plan 131 / Research 172.".to_string()),
            // Measured ParetoDominated (Batch 147) — demoted to suggestion-only;
            // the spec is deliberately withheld so the G5 promotion gate never
            // sees a known-dominated candidate. The negative is the witness.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: batch validation — N==1 bypass, optimistic AGGREGATE ─
        // verify first, per-item fallback ONLY on aggregate failure ───────
        // Source: bls-sigverify/src/bls_vote_sigverify.rs verify_votes
        // (:224-298): the N==1 short-circuit does INDIVIDUAL verification —
        // "no need to do optimistic verification when batch size == 1"
        // (`if let [unverified_vote] = unverified_votes`); otherwise "Try
        // optimistic verification - fast to verify, but cannot identify
        // invalid votes" — the aggregate check covers the whole batch (the
        // in-source cost model at :352: pairings "reduced to (1 + number
        // of distinct messages)"); on aggregate failure the fallback runs
        // verify_individual_votes (:419) to identify the bad votes
        // precisely. Decision content: batch-validating N items where full
        // per-item validation is expensive is a three-tier cost ladder —
        // (1) N==1 skips the batch machinery entirely; (2) N>1 pays the
        // CHEAP AGGREGATE claim first (common case: the whole batch is
        // valid, one check); (3) only on aggregate failure pay the
        // per-item pass to locate offenders. ADMISSION CONDITION (the
        // fixture ships a generic `fn verify_aggregate` stand-in): the
        // aggregate primitive must already exist in the consumer's stack —
        // the source's is BLS aggregate signature verification and BLS is
        // not a workspace dep; a dalek-style verify_batch stand-in + owner
        // sign-off is required before any propose-path use. Distinct from
        // `filter-count-to-any` / the short-circuit family (those skip
        // per-element work on PREDICATES; this is an AGGREGATE-vs-individual
        // VERIFICATION cost ladder with an N==1 bypass) and from kernel_opt
        // B123 (io-pool read-ahead dedup; unrelated axis). Regime:
        // signature/batch verification, merkle batch proofs, bulk integrity
        // checks — anywhere an all-valid batch is the common case. The
        // N==1 short-circuit is its own micro-win: it keeps batch/aggregate
        // overhead off the single-item hot path. ANTI-REGIME (state it
        // honestly): adversarial inputs where the aggregate ALWAYS fails —
        // the ladder then pays aggregate + individual, strictly worse than
        // individual-only; the ladder needs a LOW aggregate-failure-rate
        // assumption (honest senders, transient corruption) and that
        // assumption must hold in the consumer's regime. Suggestion-only
        // (a parse-only proposer can see the per-item map but can neither
        // prove an aggregate primitive exists nor bound the failure rate).
        PerfRule {
            rule_id: "batch-verify-optimistic-aggregate-fallback-individual".to_string(),
            name: "N-item batch always pays N full verifications → N==1 bypass + cheap AGGREGATE verify first, per-item fallback only on aggregate failure".to_string(),
            category: "caching".to_string(),
            buggy_code: "struct SignedTx {\n    sig: [u8; 64],\n    signer: [u8; 32],\n}\nimpl SignedTx {\n    // the EXPENSIVE per-item check — heavy curve math per call\n    fn verify(&self, msg: &[u8; 32]) -> bool {\n        self.signer != [0u8; 32] && self.sig[..32] != [0u8; 32]\n    }\n}\nfn verify_batch(txs: &[SignedTx], msg: &[u8; 32]) -> Vec<bool> {\n    // N items → N full verifications, ALWAYS — even when one cheap\n    // aggregate check over all N would have proven the batch valid\n    txs.iter().map(|tx| tx.verify(msg)).collect()\n}".to_string(),
            fixed_code: "struct SignedTx {\n    sig: [u8; 64],\n    signer: [u8; 32],\n}\nimpl SignedTx {\n    fn verify(&self, msg: &[u8; 32]) -> bool {\n        self.signer != [0u8; 32] && self.sig[..32] != [0u8; 32]\n    }\n}\n// ADMISSION CONDITION: this primitive must already exist in the consumer's\n// stack (the source's is BLS aggregate verification — one check over all\n// signatures). Fast, but CANNOT name which item failed.\nfn verify_aggregate(txs: &[SignedTx], msg: &[u8; 32]) -> bool {\n    txs.iter().all(|tx| tx.signer != [0u8; 32])\n}\nfn verify_batch(txs: &[SignedTx], msg: &[u8; 32]) -> Vec<bool> {\n    // tier 1 — N == 1: skip the batch machinery entirely\n    if let [only] = txs {\n        return vec![only.verify(msg)];\n    }\n    // tier 2 — N > 1: cheap AGGREGATE claim first (common case: all valid)\n    if verify_aggregate(txs, msg) {\n        return vec![true; txs.len()];\n    }\n    // tier 3 — aggregate failed: NOW pay the per-item pass to locate\n    // the invalid items\n    txs.iter().map(|tx| tx.verify(msg)).collect()\n}".to_string(),
            context_notes: Some("CONDITIONAL — ADMISSION CONDITION: the aggregate primitive must exist in the consumer's stack (the source's is BLS aggregate signature verification; BLS is not a workspace dep — a dalek-style verify_batch stand-in + owner sign-off is the staging-plan requirement); the fixture ships a generic `fn verify_aggregate(items) -> bool` stand-in and the rule must not propose where no real aggregate/batch-verify primitive exists. REGIME: signature/batch verification, merkle batch proofs, bulk integrity checks — anywhere an all-valid batch is the common case. The N==1 short-circuit is its own micro-win: it keeps batch/aggregate overhead off the single-item hot path. Cost model (in-source): the aggregate collapses the per-item crypto work to one check over the batch — pairings 'reduced to (1 + number of distinct messages)'. ANTI-REGIME (stated honestly): adversarial inputs where the aggregate ALWAYS fails — the ladder then pays aggregate + individual, strictly worse than individual-only; the ladder needs a LOW aggregate-failure-rate assumption and the consumer's regime must supply it. Distinct from `filter-count-to-any`/short-circuit family (those skip per-element work on predicates; this is an AGGREGATE-vs-individual verification cost ladder with an N==1 bypass) and from `batch-pool-read-makes-serial-readahead-redundant` (kernel_opt B123, io pool; unrelated). SUGGESTION-ONLY, NotBenchable as drafted (a parse-only proposer cannot prove the aggregate primitive exists or bound the failure rate; the honest bench needs a real aggregate primitive a generic stand-in cannot supply). Source: anza-xyz/agave @ c95d87063 (Apache-2.0) — bls-sigverify/src/bls_vote_sigverify.rs verify_votes:224-298, verify_votes_optimistic:315-379 (the :352 cost model), verify_individual_votes:419. Staged agave batch / Plan 131 / Research 172.".to_string()),
            // NotBenchable as drafted — tier 2 needs a real aggregate primitive
            // (BLS-class batch verify) that a generic fixture cannot honestly
            // stand in for; bench spec deferred to the admission-condition landing.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── allocation: fixed-cap pool — O(1) WATERMARK freelist + PARTIAL ─
        // reset of the used prefix on return, never a whole-object clear ──
        // Source: program-runtime/src/mem_pool.rs Pool<T, SIZE>
        // (:24-55): `next_empty` is a WATERMARK over a fixed [Option<T>;
        // SIZE] — get() takes items[next_empty - 1] and decrements;
        // put() resets the value IN PLACE — `value.reset(len)` — then
        // stores it at the watermark and increments. The Reset impl for
        // the pooled AlignedMemory (:57-65) zeroes only the used PREFIX:
        // `let len = len.min(slice.len())` then fill over ..len — the
        // clear is sized by the work done since checkout, never by the
        // buffer's capacity. Decision content: a fixed-capacity freelist
        // where returned objects are reset ONLY up to the length actually
        // used (partial reset at the watermark), and get/put are O(1)
        // watermark moves — no scan, no free-list Vec, no realloc: the
        // pool's steady-state alloc count is SIZE, forever. Buggy shapes:
        // clear the WHOLE object on every return (cost ∝ capacity), or
        // drop-and-reallocate per checkout (re-buying the alloc every
        // time). Distinct from `reuse-allocation-clear` (entries_02 —
        // clear-reuse of ONE scratch buffer handed in by a caller; this is
        // a POOL of N objects the caller checks out from), from `mem-take`
        // (move-the-allocation idiom, no reset semantics), and from
        // `bounded-batch-buffer-pool` (companion rule, same batch —
        // bounded RETENTION cap + replenish-consumed for batch buffers;
        // the boundary: WHERE the bound lives — retention cap vs fixed
        // compile-time SIZE — and WHAT reset costs — full clear vs
        // used-prefix). Regime: execution-memory pools — wasm/VM scratch
        // arenas, fixed-cap object pools in request paths (the source
        // pools VM stack/heap/call-frame memory). Partial-reset cost
        // model: reset cost ∝ the used prefix reported at put() — a
        // checkout that touched 512 bytes of a 64 KiB buffer pays 512
        // bytes of clear, not 64 KiB. ANTI-REGIME (state it honestly):
        // objects where stale data from the UN-reset tail could be
        // OBSERVED — the tail beyond used_len keeps the previous tenant's
        // bytes, so keys/secrets/anything read past the logical length
        // must FULLY clear instead; and the contract requires put() to
        // receive the TRUE used length (a lying length leaves stale bytes
        // inside the logical prefix). Suggestion-only (a parse-only
        // proposer cannot see the pool's size policy or the reset
        // contract).
        PerfRule {
            rule_id: "partial-reset-watermark-pool".to_string(),
            name: "fixed-cap pool clears the WHOLE object on return (or reallocates per checkout) → O(1) watermark freelist + reset(used_len) partial reset".to_string(),
            category: "allocation".to_string(),
            buggy_code: "struct ScratchBuf {\n    data: Vec<u8>, // fixed working size, e.g. 64 KiB\n}\nstruct BufPool {\n    free: Vec<ScratchBuf>,\n}\nimpl BufPool {\n    fn get(&mut self) -> Option<ScratchBuf> {\n        self.free.pop()\n    }\n    // interface cannot express \"only the prefix I touched\" — every\n    // return clears the WHOLE object: cost ∝ capacity, not work done\n    fn put(&mut self, mut buf: ScratchBuf) {\n        buf.data.fill(0);\n        self.free.push(buf);\n    }\n}".to_string(),
            fixed_code: "trait Reset {\n    // reset only up to the length actually used — cost ∝ work, not capacity\n    fn reset(&mut self, used_len: usize);\n}\nstruct ScratchBuf {\n    data: Vec<u8>, // fixed working size, e.g. 64 KiB\n}\nimpl Reset for ScratchBuf {\n    fn reset(&mut self, used_len: usize) {\n        let len = used_len.min(self.data.len());\n        self.data[..len].fill(0);\n    }\n}\n// fixed-capacity freelist: get/put are O(1) watermark moves, no scan\nstruct BufPool<T: Reset, const SIZE: usize> {\n    items: [Option<T>; SIZE],\n    next_empty: usize, // watermark: slots [next_empty..SIZE] are free\n}\nimpl<T: Reset, const SIZE: usize> BufPool<T, SIZE> {\n    fn new(items: [T; SIZE]) -> Self {\n        Self {\n            items: items.map(Some),\n            next_empty: SIZE,\n        }\n    }\n    fn get(&mut self) -> Option<T> {\n        self.next_empty = self.next_empty.checked_sub(1)?;\n        self.items.get_mut(self.next_empty).and_then(|s| s.take())\n    }\n    fn put(&mut self, mut value: T, used_len: usize) -> bool {\n        let Some(slot) = self.items.get_mut(self.next_empty) else {\n            return false; // pool full\n        };\n        value.reset(used_len); // partial reset, in place, AT the watermark\n        slot.replace(value);\n        self.next_empty += 1;\n        true\n    }\n}".to_string(),
            context_notes: Some("REGIME: execution-memory pools — wasm/VM scratch arenas, fixed-cap object pools in request paths (the source pools VM stack/heap/call-frame memory). Partial-reset cost model: reset cost ∝ the used prefix reported at put(), NOT capacity — a checkout that touched 512 bytes of a 64 KiB buffer pays a 512-byte clear; get/put are O(1) watermark moves (no free-list scan, no realloc — steady-state alloc count is the fixed SIZE forever). ANTI-REGIME (stated honestly): security-sensitive buffers where the un-reset TAIL could be observed — keys, secrets, anything read past the logical length must fully clear instead, and the rule requires put() to receive the TRUE used length (a lying length leaves stale bytes inside the logical prefix). Distinct from `reuse-allocation-clear` (entries_02 — clear-reuse of ONE caller-supplied scratch buffer; this is a POOL of N objects with a watermark freelist + partial reset), from `mem-take` (move idiom, no reset semantics), and from `bounded-batch-buffer-pool` (companion rule, same batch — bounded RETENTION cap + replenish-consumed for batch buffers; the boundary: WHERE the bound lives — retention cap vs fixed compile-time SIZE — and WHAT reset costs — full clear vs used-prefix). Suggestion-only, NotBenchable as drafted (a parse-only proposer cannot see the pool's size policy or the reset contract; the allocation microbench — used-prefix vs full-clear at a realistic touched/capacity ratio — is writable and deferred, not claimed). Source: anza-xyz/agave @ c95d87063 (Apache-2.0) — program-runtime/src/mem_pool.rs Pool<T, SIZE>:24-55, the Reset impl for AlignedMemory:57-65. Staged agave batch / Plan 131 / Research 172.".to_string()),
            // Bench spec deliberately deferred to a follow-up: the honest
            // allocation microbench (used-prefix vs full-clear at a
            // realistic touched/capacity ratio) is writable but was not
            // produced this batch; the rule ships suggestion-only.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: batch-local loader caches the MISS too — an Absent ──
        // tombstone so a repeated miss costs a map hit, not a second ────
        // backend query ─────────────────────────────────────────────
        // Source: svm/src/account_loader.rs load_account (:256-274): the
        // batch-local `loaded_accounts` map serves FOUR arms —
        // cache-hit-alive (Some, false) → Some; cache-hit-placeholder
        // (None, false) → None; backend-hit (Some, true) → insert the
        // clone (POSITIVE caching); and the decisive miss arm (None, true)
        // → "Does not exist and has never been seen" → INSERTS
        // AccountSharedData::default() as the tombstone before returning
        // None — so the NEXT ask of the same absent key reads the map
        // (do_load's lamports == 0 path, :279-296) instead of re-querying
        // accounts-db. The tombstone rides in the SAME map as the
        // positives (a default account's zero lamports encodes "absent")
        // — no second structure; and the in-source staleness contract is
        // "We *never* remove accounts, or else we would fetch stale state
        // from accounts-db" (:283) — entries live for the batch's
        // lifetime and the batch's own writes own the state. Decision
        // content: a batch/staged loader that will re-ask the same key
        // must cache NEGATIVE results too — Entry = Present(T) | Absent
        // tombstone — so repeated misses cost a map hit, not a backend
        // query. Buggy shape: the cache stores hits only, Option<&T> None
        // is indistinguishable from not-yet-asked, and every miss walks
        // the store again. Distinct from `sort-key-cache-precompute-out-
        // of-lock` (per-COMPARISON key cache), from the per-insert
        // aggregate-statistic family (`debug-stat-full-scan-out-of-hot-
        // path` — per-operation whole-collection statistics), and from
        // the resume-provenance identity rules (entries_05 — build-level
        // identity pins, not a lookup cache). The decision content is
        // NEGATIVE-result caching within a batch scope, distinguishable by
        // the tombstone insert ON THE MISS PATH. Regime: transaction
        // loading against a store, batch pipelines re-asking absent keys
        // (an absent fee-credit account referenced by several txs in one
        // batch). Tombstone memory is bounded by the batch's key set — one
        // entry per key asked, absent or not. ANTI-REGIME (state it
        // honestly): keys asked exactly once — the tombstone is pure
        // overhead (a map insert for nothing); and a store that MUTATES
        // mid-batch under the loader's feet — a cached Absent can then be
        // stale, which is why the tombstone's scope is the batch, not a
        // global cache. Suggestion-only (a parse-only proposer cannot
        // prove the loader re-asks keys or bound the key set).
        PerfRule {
            rule_id: "batch-local-memoization-negative-cache-tombstone".to_string(),
            name: "batch cache stores hits only — every miss re-queries the backend → cache the miss too (Absent tombstone), repeated misses cost a map hit".to_string(),
            category: "caching".to_string(),
            buggy_code: "use std::collections::HashMap;\nstruct AccountDb;\nimpl AccountDb {\n    // expensive: index lookups + storage reads per call\n    fn fetch(&self, key: u64) -> Option<Vec<u8>> {\n        (key % 3 == 0).then(|| vec![0u8; 32])\n    }\n}\nstruct StagedLoader<'a> {\n    cache: HashMap<u64, Vec<u8>>, // POSITIVE results only\n    backend: &'a AccountDb,\n}\nimpl<'a> StagedLoader<'a> {\n    fn load(&mut self, key: u64) -> Option<Vec<u8>> {\n        if let Some(account) = self.cache.get(&key) {\n            return Some(account.clone());\n        }\n        // the map cannot tell \"asked and absent\" from \"never asked\" —\n        // every miss walks the backend AGAIN (and again, per re-ask)\n        match self.backend.fetch(key) {\n            Some(account) => {\n                self.cache.insert(key, account.clone());\n                Some(account)\n            }\n            None => None,\n        }\n    }\n}".to_string(),
            fixed_code: "use std::collections::HashMap;\nstruct AccountDb;\nimpl AccountDb {\n    fn fetch(&self, key: u64) -> Option<Vec<u8>> {\n        (key % 3 == 0).then(|| vec![0u8; 32])\n    }\n}\nenum Entry {\n    Present(Vec<u8>),\n    Absent, // tombstone: \"known absent\" — do not re-ask this batch\n}\nstruct StagedLoader<'a> {\n    cache: HashMap<u64, Entry>,\n    backend: &'a AccountDb,\n}\nimpl<'a> StagedLoader<'a> {\n    fn load(&mut self, key: u64) -> Option<Vec<u8>> {\n        match self.cache.get(&key) {\n            // both flavors of hit answer WITHOUT a backend query\n            Some(Entry::Present(account)) => return Some(account.clone()),\n            Some(Entry::Absent) => return None,\n            None => {}\n        }\n        match self.backend.fetch(key) {\n            Some(account) => {\n                self.cache.insert(key, Entry::Present(account.clone()));\n                Some(account)\n            }\n            None => {\n                // the tombstone insert — repeated misses are now map hits\n                self.cache.insert(key, Entry::Absent);\n                None\n            }\n        }\n    }\n}".to_string(),
            context_notes: Some("REGIME: transaction loading against a store, batch pipelines that re-ask absent keys (an absent fee-credit account referenced by several txs in one batch). Tombstone memory is bounded by the batch key set — one entry per key asked, absent or not; the source rides the tombstone in the SAME map as positives (a default account's zero lamports encodes absent) so no second structure is needed, and its staleness contract is in-source: entries are never removed within the batch ('We *never* remove accounts, or else we would fetch stale state from accounts-db') — the batch's own writes own the state. ANTI-REGIME (stated honestly): keys asked exactly once — the tombstone is pure overhead; and a store that mutates mid-batch under the loader's feet makes a cached Absent stale — the tombstone's scope is the BATCH, never a global cache. Distinct from `sort-key-cache-precompute-out-of-lock` (per-comparison key cache), from the per-insert aggregate-statistic family (`debug-stat-full-scan-out-of-hot-path`), and from the resume-provenance identity rules (entries_05 — build-level identity): the decision content is NEGATIVE-result caching within a batch scope, distinguishable by the tombstone insert ON THE MISS PATH. Suggestion-only, NotBenchable as drafted (a parse-only proposer cannot prove the loader re-asks keys or bound the key set; the honest bench needs a backend whose miss cost dominates a map hit at a realistic ratio — deferred, not claimed). Source: anza-xyz/agave @ c95d87063 (Apache-2.0) — svm/src/account_loader.rs load_account:256-274 (the (None, true) tombstone arm), do_load:279-296 (the lamports == 0 read-back + the never-remove staleness contract at :283). Staged agave batch / Plan 131 / Research 172.".to_string()),
            // Bench spec deliberately deferred: the honest bench needs a
            // backend whose miss cost dominates a map hit at a realistic
            // ratio; spec deferred to the integration follow-up.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        ]
}

/// Bench spec for `debug-level-gated-computation`: a log-only summary built
/// unconditionally beside its `debug!` call vs the same computation behind
/// `log_enabled!`.
///
/// With no logger configured (the bench crate never installs one), the fixed
/// arm builds ZERO summaries; the buggy arm builds one per item — the binding
/// is a real statement and log's runtime max-level atomic keeps the disabled
/// branch from being const-folded. Expected `ParetoDominant` (the fixed arm
/// does strictly less work; the observable log output is identical: nothing).
/// Default `log` features ONLY — a `release_max_level_*` feature would compile
/// the whole regime away and make both arms tie at zero.
#[cfg(feature = "rust_perf_bench")]
fn debug_level_gate_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000],
        harness: "// [dep: log = \"0.4\"]\n\nconst EVENT_PARTS: usize = 8;\n\nfn summarize(item: &str) -> String {\n    // an expensive, log-only diagnostic: bracket every part, then fold\n    let mut acc = String::new();\n    for part in item.split('/') {\n        acc.push_str(&format!(\"[{part}]\"));\n    }\n    let mut digest = acc.len();\n    for b in acc.bytes() {\n        digest = digest.wrapping_mul(31).wrapping_add(b as usize);\n    }\n    format!(\"{acc} (len={digest})\")\n}\n\nmod buggy {\n    use super::summarize;\n    // The binding is a REAL statement: log's max level is a runtime atomic,\n    // so the macro's enabled branch is never const-folded away — the summary\n    // executes on every item even though the record is never emitted.\n    pub fn tick(items: &[String]) {\n        for it in items {\n            let summary = summarize(it);\n            log::debug!(\"event summary: {summary}\");\n        }\n    }\n}\n\nmod fixed {\n    use super::summarize;\n    // The computation lives INSIDE the level gate. With no logger (the bench\n    // regime) `log_enabled!` is false and the summary is never built.\n    pub fn tick(items: &[String]) {\n        for it in items {\n            if log::log_enabled!(log::Level::Debug) {\n                let summary = summarize(it);\n                log::debug!(\"event summary: {summary}\");\n            }\n        }\n    }\n}\n\npub fn make_input(n: usize) -> Vec<String> {\n    (0..n)\n        .map(|i| {\n            (0..EVENT_PARTS)\n                .map(|p| format!(\"p{}x{}\", p, (i * 31 + p) % 97))\n                .collect::<Vec<_>>()\n                .join(\"/\")\n        })\n        .collect()\n}\n\npub fn run_buggy(input: &[String]) {\n    buggy::tick(std::hint::black_box(input));\n}\n\npub fn run_fixed(input: &[String]) {\n    fixed::tick(std::hint::black_box(input));\n}\n".to_string(),
    }
}

/// Bench spec for `atomic-vec-zero-cost-convert` (Batch 135, Plan 114 —
/// the Wild-linker article distill): both arms pay ONE shared `clone()` so
/// the timing delta is exactly the conversion; the buggy conversion then
/// adds a fresh malloc + element copy while the fixed one consumes the
/// owned clone in place (std's same-layout collect reuse — the alloc axis
/// pins the elision EXACTLY: 2 → 1). G1: both arms read the same values,
/// so the Relaxed-load checksums must match exactly.
#[cfg(feature = "rust_perf_bench")]
fn atomic_vec_zero_cost_convert_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    use std::sync::atomic::{AtomicU32, Ordering};

    // The source is only BORROWED: a fresh allocation + element-by-element
    // copy is forced on every conversion.
    pub fn to_atomic(plain: &[u32]) -> Vec<AtomicU32> {
        plain.iter().map(|&v| AtomicU32::new(v)).collect()
    }

    pub fn convert(owned: Vec<u32>) -> u64 {
        let atomic = to_atomic(&owned);
        std::hint::black_box(&atomic); // keep the allocation observable
        atomic.iter().map(|a| a.load(Ordering::Relaxed) as u64).sum()
    }
}

mod fixed {
    use std::sync::atomic::{AtomicU32, Ordering};

    // The vec is OWNED: into_iter walks in place and the same-layout
    // collect reuses the buffer — no new allocation, no copy loop.
    pub fn to_atomic(plain: Vec<u32>) -> Vec<AtomicU32> {
        plain.into_iter().map(AtomicU32::new).collect()
    }

    pub fn convert(owned: Vec<u32>) -> u64 {
        let atomic = to_atomic(owned);
        std::hint::black_box(&atomic);
        atomic.iter().map(|a| a.load(Ordering::Relaxed) as u64).sum()
    }
}

pub struct Input {
    a: Vec<u32>,
    b: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0x853C_49E6_748F_EA9Bu64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed >> 33
    };
    let mut mk = || (0..n).map(|_| next() as u32 | 1).collect::<Vec<u32>>();
    Input { a: mk(), b: mk() }
}

pub fn run_buggy(input: &Input) {
    let owned = input.a.clone(); // shared ownership gain: both arms pay it
    std::hint::black_box(buggy::convert(owned));
}

pub fn run_fixed(input: &Input) {
    let owned = input.b.clone();
    std::hint::black_box(fixed::convert(owned));
}
"#
        .to_string(),
    }
}

/// Bench spec for `vec-reuse-across-lifetime-change` (Batch 135): the buggy
/// arm builds a fresh `Vec<&[u32]>` per call (malloc + fill + consume); the
/// fixed arm owns a `'static`-typed buffer (`thread_local`), retargets it per
/// call via the `reuse_vec` pattern, and refills — capacity retained, so the
/// steady-state alloc axis is 1 → 0. The rows are `Box::leak`ed so the
/// `'static` element type is sound inside the harness (the rule text carries
/// the real-world caller contract). G1: identical sums.
#[cfg(feature = "rust_perf_bench")]
fn vec_reuse_across_lifetime_change_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100, 1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    // Fresh vec of row refs per call: one malloc + fill + consume.
    pub fn row_lens(rows: &[Vec<u32>]) -> u64 {
        let picked: Vec<&[u32]> = rows.iter().map(|r| &r[..]).collect();
        std::hint::black_box(&picked); // keep the allocation observable
        picked.iter().map(|s| s.len() as u64).sum()
    }
}

mod fixed {
    use std::cell::RefCell;

    // The buffer is OWNED and retargeted per call: the const size/align
    // assert makes a mismatched T/U a compile error; clear() empties the
    // buffer so the map body is unreachable and the loop compiles out;
    // std's in-place collect hands the SAME allocation back re-typed.
    pub fn reuse_vec<T, U>(mut v: Vec<T>) -> Vec<U> {
        const {
            assert!(
                std::mem::size_of::<T>() == std::mem::size_of::<U>()
                    && std::mem::align_of::<T>() == std::mem::align_of::<U>(),
            );
        }
        v.clear();
        v.into_iter().map(|_| unreachable!()).collect()
    }

    thread_local! {
        // 'static-typed BY CONTRACT: rows are leaked for the whole bench,
        // and the buffer is cleared before any stored ref could be read.
        static BUF: RefCell<Vec<&'static [u32]>> = const { RefCell::new(Vec::new()) };
    }

    pub fn row_lens(rows: &[Vec<u32>]) -> u64 {
        BUF.with(|b| {
            let mut buf = b.borrow_mut();
            let mut cur: Vec<&[u32]> = reuse_vec(std::mem::take(&mut buf));
            cur.clear();
            cur.extend(rows.iter().map(|r| &r[..]));
            let total: u64 = cur.iter().map(|s| s.len() as u64).sum();
            let retargeted: Vec<&'static [u32]> = reuse_vec(cur);
            *buf = retargeted;
            total
        })
    }
}

pub struct Input {
    rows: &'static [Vec<u32>],
}

pub fn make_input(n: usize) -> Input {
    let mut seed = 0xA24B_5D3F_6C17_E890u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed >> 33
    };
    let rows: Vec<Vec<u32>> = (0..n)
        .map(|_| (0..4).map(|_| next() as u32).collect())
        .collect();
    Input { rows: Box::leak(rows.into_boxed_slice()) }
}

pub fn run_buggy(input: &Input) {
    std::hint::black_box(buggy::row_lens(input.rows));
}

pub fn run_fixed(input: &Input) {
    std::hint::black_box(fixed::row_lens(input.rows));
}
"#
        .to_string(),
    }
}

/// Bench spec for `partitioned-mut-slices-zero-sync` (Batch 135): the buggy
/// arm locks a single Mutex PER BUMP across all workers (contended); the
/// fixed arm partitions once into disjoint contiguous chunks and runs plain
/// stores with zero synchronization. Sizes = THREAD counts per the align-64
/// precedent. G1: the closed-form checksum (every cell bumped ITERS times)
/// is asserted in-harness. The alloc axis is EXEMPT for this rule (see
/// `ALLOC_AXIS_EXEMPT` — contended macOS pthread mutex slow path).
#[cfg(feature = "rust_perf_bench")]
fn partitioned_mut_slices_zero_sync_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1, 2, 4, 8],
        harness: r#"
use std::sync::Mutex;

const CELLS: usize = 4_096;
const ITERS: usize = 8;

mod buggy {
    use std::sync::Mutex;

    // One lock guards the whole array; every bump serializes even though
    // the workers' index ranges never overlap.
    pub fn bump(grid: &Mutex<Vec<u64>>, workers: usize) {
        std::thread::scope(|s| {
            for w in 0..workers {
                s.spawn(move || {
                    for cell_idx in (w..super::CELLS).step_by(workers) {
                        for _ in 0..super::ITERS {
                            let mut g = grid.lock().unwrap();
                            g[cell_idx] = g[cell_idx].wrapping_add(1);
                        }
                    }
                });
            }
        });
    }
}

mod fixed {
    // Partition ONCE into disjoint contiguous ranges: plain stores, no
    // lock, no atomic — each cell is bumped by exactly one owner.
    pub fn bump(grid: &mut Vec<u64>, workers: usize) {
        let per = grid.len().div_ceil(workers);
        std::thread::scope(|s| {
            for chunk in grid.chunks_mut(per) {
                s.spawn(move || {
                    for _ in 0..super::ITERS {
                        for cell in chunk.iter_mut() {
                            *cell = cell.wrapping_add(1);
                        }
                    }
                });
            }
        });
    }
}

pub struct Input {
    workers: usize,
}

pub fn make_input(n: usize) -> Input {
    // n = THREAD COUNT (the contention axis is the size axis).
    Input { workers: n }
}

fn expected() -> u64 {
    (CELLS * ITERS) as u64
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let grid = Mutex::new(vec![0u64; CELLS]);
    buggy::bump(&grid, input.workers);
    let cs: u64 = grid.lock().unwrap().iter().sum();
    debug_assert_eq!(cs, expected());
    std::hint::black_box(cs);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut grid = vec![0u64; CELLS];
    fixed::bump(&mut grid, input.workers);
    let cs: u64 = grid.iter().sum();
    debug_assert_eq!(cs, expected());
    std::hint::black_box(cs);
}
"#
        .to_string(),
    }
}

/// Bench spec for `bounded-batch-buffer-pool` (Batch 147, Plan 131 / Research
/// 172 — the scheduler `MAX_RECYCLED_BATCHES` + `PacketBufferPool` distill):
/// fresh buffer triple per cycle vs bounded take/put pool.
///
/// sizes = CYCLE COUNT (the churn axis): the buggy arm pays 3 malloc/free
/// per cycle forever; the fixed arm allocates once per parked buffer, then
/// ~0 (pool at steady state). The cap (1024) parks the pool at steady state
/// — retention is bounded, never grown. G1: both arms return the same total
/// (keys+vals+stamps over every record), `debug_assert`'d against a
/// make_input-computed ground truth. Alloc axis: N cycles × 3 allocs → ~3
/// total (warmup) — the deterministic axis; timing axis the malloc/free
/// round-trips.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)] // the harness is a single embedded raw string
fn bounded_batch_buffer_pool_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 4_000, 16_000],
        harness: r#"
pub const TARGET: usize = 64; // records per batch (fixed shape)
pub const CAP: usize = 1024; // MAX_RECYCLED — the pool's hard bound

pub struct Work {
    pub key: u64,
    pub val: u64,
    pub stamp: u32,
}
pub struct Input {
    pub work: Vec<Work>, // n * TARGET records, chunked into n batches
    pub expected: u64,
}
fn next(rng: &mut u64) -> u64 {
    *rng = rng
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *rng >> 16
}
fn checksum(work: &[Work]) -> u64 {
    let mut total = 0u64;
    for chunk in work.chunks(TARGET) {
        total = total
            .wrapping_add(chunk.iter().map(|w| w.key).sum::<u64>())
            .wrapping_add(chunk.iter().map(|w| w.val).sum::<u64>())
            .wrapping_add(chunk.iter().map(|w| w.stamp as u64).sum::<u64>());
    }
    total
}
pub fn make_input(n: usize) -> Input {
    let mut rng: u64 = 0x853C_49E6_748F_EA9B;
    let mut work = Vec::with_capacity(n * TARGET);
    for _ in 0..n * TARGET {
        let key = next(&mut rng) & 0xF;
        let val = next(&mut rng) & 0xF;
        let stamp = (next(&mut rng) & 0xF) as u32;
        work.push(Work { key, val, stamp });
    }
    let expected = checksum(&work);
    Input { work, expected }
}
mod buggy {
    use super::*;
    // fresh triple allocated and dropped EVERY cycle
    pub fn run(work: &[Work]) -> u64 {
        let mut total = 0u64;
        for chunk in work.chunks(TARGET) {
            let mut keys: Vec<u64> = Vec::with_capacity(TARGET);
            let mut vals: Vec<u64> = Vec::with_capacity(TARGET);
            let mut stamps: Vec<u32> = Vec::with_capacity(TARGET);
            for w in chunk {
                keys.push(w.key);
                vals.push(w.val);
                stamps.push(w.stamp);
            }
            total = total
                .wrapping_add(keys.iter().sum::<u64>())
                .wrapping_add(vals.iter().sum::<u64>())
                .wrapping_add(stamps.iter().map(|&s| s as u64).sum::<u64>());
            // dropped here — 3 malloc/free per cycle
        }
        total
    }
}
mod fixed {
    use super::*;
    pub struct BatchPool {
        recycled: Vec<(Vec<u64>, Vec<u64>, Vec<u32>)>,
    }
    impl BatchPool {
        fn new() -> Self {
            Self { recycled: Vec::new() }
        }
        fn take(&mut self) -> (Vec<u64>, Vec<u64>, Vec<u32>) {
            self.recycled.pop().unwrap_or_else(|| {
                (
                    Vec::with_capacity(TARGET),
                    Vec::with_capacity(TARGET),
                    Vec::with_capacity(TARGET),
                )
            })
        }
        fn put(&mut self, mut batch: (Vec<u64>, Vec<u64>, Vec<u32>)) {
            batch.0.clear();
            batch.1.clear();
            batch.2.clear();
            if self.recycled.len() < CAP {
                self.recycled.push(batch);
            } // else dropped — the cap is the decision
        }
    }
    pub fn run(work: &[Work]) -> u64 {
        let mut pool = BatchPool::new();
        let mut total = 0u64;
        for chunk in work.chunks(TARGET) {
            let (mut keys, mut vals, mut stamps) = pool.take();
            for w in chunk {
                keys.push(w.key);
                vals.push(w.val);
                stamps.push(w.stamp);
            }
            total = total
                .wrapping_add(keys.iter().sum::<u64>())
                .wrapping_add(vals.iter().sum::<u64>())
                .wrapping_add(stamps.iter().map(|&s| s as u64).sum::<u64>());
            pool.put((keys, vals, stamps));
        }
        total
    }
}
pub fn run_buggy(input: &Input) {
    let total = buggy::run(&input.work);
    debug_assert_eq!(total, input.expected);
    std::hint::black_box(total);
}
pub fn run_fixed(input: &Input) {
    let total = fixed::run(&input.work);
    debug_assert_eq!(total, input.expected);
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

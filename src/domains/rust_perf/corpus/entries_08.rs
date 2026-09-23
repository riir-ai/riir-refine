//! Entries slice 08 — Batch 174 (2026-09-24, Plan 168 / Research 205): the
//! host-side lock-scope law + the deferred-command barrier batching — both
//! distilled from merged upstream fixes (tracel-ai/cubecl #1678 + #1686,
//! bevyengine/bevy #25851), quotes re-verified against the merged diffs.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── concurrency: lock held across a call that blocks on a thread
        // that re-takes the same lock → decide under the lock, release
        // after ────────────────────────────────────────────────────────
        // Source: tracel-ai/cubecl #1678 "release the probe pool lock
        // before the device call" (merged @ 9d1314c0 = tag v0.11.0-pre.4,
        // closes #1668). PooledProbes::cleanup_unless_held held the global
        // POOLED_PROBES lock ACROSS client.memory_cleanup() — which submits
        // to the device runner thread and waits for drain, and the runner
        // TAKES THE SAME LOCK when it autotunes → mutual wait, process
        // never recovers. Fix: decide under the lock (held_by read inside
        // a scoped guard), drop the guard, THEN issue the release.
        PerfRule {
            rule_id: "host-lock-across-reentrant-blocking-call".to_string(),
            name: "a shared-state lock held across a blocking call that transitively waits on a worker which RE-TAKES the same lock (device runner / executor / thread pool) → decide under the lock in its own scope, drop the guard, issue the side-effect after release — mutual wait by construction, not contention".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "use std::collections::HashSet;\nuse std::sync::Mutex;\n\n// A global probe pool keyed by service id (the CubeCL PooledProbes shape).\nstatic POOLED_PROBES: Mutex<HashSet<u64>> = Mutex::new(HashSet::new());\n\npub struct Client {\n    service_id: u64,\n}\n\nimpl Client {\n    /// Submits cleanup work to the device runner thread and WAITS for\n    /// drain — and the runner takes POOLED_PROBES itself when autotuning.\n    pub fn memory_cleanup(&self) {\n        #![allow(unused)]\n    }\n}\n\nfn cleanup_unless_held(client: &Client) {\n    // DECIDE + ACT under one lock: the guard is held ACROSS\n    // memory_cleanup(), which blocks on a thread that re-takes it —\n    // a mutual wait the process never recovers from.\n    let pooled = POOLED_PROBES.lock().unwrap();\n    if !pooled.contains(&client.service_id) {\n        client.memory_cleanup(); // <- guard still held here\n    }\n}".to_string(),
            fixed_code: "use std::collections::HashSet;\nuse std::sync::Mutex;\n\nstatic POOLED_PROBES: Mutex<HashSet<u64>> = Mutex::new(HashSet::new());\n\npub struct Client {\n    service_id: u64,\n}\n\nimpl Client {\n    pub fn memory_cleanup(&self) {\n        #![allow(unused)]\n    }\n}\n\nfn cleanup_unless_held(client: &Client) {\n    // DECIDE under the lock, in its own scope — the guard drops at the\n    // closing brace. `release` runs with the lock dropped: it blocks on\n    // the device thread, which takes this lock itself when it autotunes\n    // (the upstream doc line, verbatim).\n    let held = {\n        let pooled = POOLED_PROBES.lock().unwrap();\n        pooled.contains(&client.service_id)\n    }; // guard dropped HERE — before the blocking call\n\n    if !held {\n        client.memory_cleanup(); // side-effect issued AFTER the release\n    }\n}".to_string(),
            context_notes: Some("NotBenchable (deadlock/hang class — the O_NONBLOCK head-probe precedent, Batch 172): the buggy arm's cost is a process-wedging mutual wait, not a slow path — a bench either hangs (measuring nothing) or, with the re-entry removed, measures nothing interesting. The decision content is the LOCK SCOPE LAW plus two sub-decisions. THE LAW: a lock guarding shared state must not be held across a call that BLOCKS ON A THREAD THAT RE-TAKES THE SAME LOCK — the call-chain re-entry is invisible at the lock site (memory_cleanup submits to the device runner and waits for drain; the runner takes the same pool lock when it autotunes — mutual wait, DEADLOCK BY CONSTRUCTION, not contention latency). THE FIX SHAPE: decide under the lock (read the decision inputs inside a scoped guard), DROP the guard at the scope end, THEN issue the side-effect — upstream's doc line, verbatim: \"release runs with the lock dropped: it blocks on the device thread, which takes this lock itself when it autotunes.\" SUB-DECISION 1 (skip the no-op side-effect): gate the cleanup on whether a probe actually RAN — \"Only a probe leaves pools with the allocator\"; a cache ANSWER means nothing was allocated, so no release is owed (upstream measured 3 redundant cleanups per roofline_bounds per autotuning thread — the queue-filler that made the latent deadlock reachable at all). SUB-DECISION 2 (assert the drop): the upstream regression test spawns a thread that TAKES THE LOCK while the release runs (a_release_is_issued_with_the_lock_dropped) — the lock-drop is proven by construction, not by reading the code; its sibling a_running_sweep_is_not_released pins the decide-half. DISTINCT FROM tokio-guard-across-await (entries_02): same fix SHAPE (fetch/decide under the guard, drop, then the slow thing), different failure mode and different detection surface — the async rule's hazard is contention (a yielded task parks the guard; clippy's await_holding_lock is a partial oracle for std guards), this one's hazard is re-entrant SELF-DEADLOCK with NO lint oracle (the call-chain re-entry is not statically visible); apply this rule when the blocking call transitively waits on the locked resource's OWN workers (device runners, executors, thread pools the lock's other users run on). kernel_opt's deadlock family (encoder ceilings, co-residency, init races) is GPU-side — this is the HOST-side lock-scope law. Suggestion-only (a parse-only proposer cannot see call-graph re-entry). Fixtures original reproductions; source: tracel-ai/cubecl #1678 (merged @ 9d1314c070b998008959db021abcc23228ab1eb8 = tag v0.11.0-pre.4, closes #1668) — crates/cubecl-std/src/throughput/pooling.rs (cleanup_unless_held_by) + base.rs (the probed flag) + the two regression tests. Batch 174 / Plan 168 / Research 205.".to_string()),
            // Not benchable: the buggy arm is a deadlock, not a slow path.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── concurrency: per-iteration deferred-command barrier → once
        // after the batch loop ──────────────────────────────────────────
        // Source: bevyengine/bevy #25851 "Only flush the world after
        // despawning all entities in despawn_all" (merged @ 1fb979e37).
        // despawn_no_free_with_caller split into a no-flush work fn + a
        // flushing wrapper; the despawn_entities retain loop calls the
        // NO-FLUSH variant per entity and issues ONE world.flush() after
        // the loop. Per-iteration barrier → once-after-batch.
        PerfRule {
            rule_id: "deferred-command-flush-hoist-out-of-batch-loop".to_string(),
            name: "a deferred-command barrier (World::flush / Commands::flush or equivalent) driven INSIDE a batch loop → hoist to ONE flush after the loop — every iteration pays the sync machinery's fixed cost to apply exactly one command; sound only where no observer needs partial mid-batch state (the source's own justification)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: "pub struct CmdQueue {\n    pending: Vec<Cmd>,\n}\n\n#[derive(Clone, Copy)]\npub enum Cmd {\n    Despawn(u32),\n}\n\npub struct World {\n    alive: Vec<bool>,\n}\n\nimpl CmdQueue {\n    pub fn push(&mut self, cmd: Cmd) {\n        self.pending.push(cmd);\n    }\n\n    /// Applies ALL pending commands AND drives the sync machinery\n    /// (hook dispatch, entity-reserve flush) — a fixed cost every\n    /// call pays regardless of pending count.\n    pub fn flush(&mut self, world: &mut World) {\n        drive_sync_machinery();\n        while let Some(cmd) = self.pending.pop() {\n            match cmd {\n                Cmd::Despawn(id) => world.alive[id as usize] = false,\n            }\n        }\n    }\n}\n\nfn drive_sync_machinery() {\n    #![allow(unused)]\n}\n\n// The pre-#25851 despawn_all shape: the barrier is driven PER ENTITY.\npub fn despawn_all(world: &mut World, ids: &[u32]) {\n    let mut q = CmdQueue { pending: Vec::new() };\n    for &id in ids {\n        q.push(Cmd::Despawn(id));\n        q.flush(world); // flush per item — n sync points\n    }\n}".to_string(),
            fixed_code: "pub struct CmdQueue {\n    pending: Vec<Cmd>,\n}\n\n#[derive(Clone, Copy)]\npub enum Cmd {\n    Despawn(u32),\n}\n\npub struct World {\n    alive: Vec<bool>,\n}\n\nimpl CmdQueue {\n    pub fn push(&mut self, cmd: Cmd) {\n        self.pending.push(cmd);\n    }\n\n    /// Applies ALL pending commands AND drives the sync machinery\n    /// (hook dispatch, entity-reserve flush) — a fixed cost every\n    /// call pays regardless of pending count.\n    pub fn flush(&mut self, world: &mut World) {\n        drive_sync_machinery();\n        while let Some(cmd) = self.pending.pop() {\n            match cmd {\n                Cmd::Despawn(id) => world.alive[id as usize] = false,\n            }\n        }\n    }\n}\n\nfn drive_sync_machinery() {\n    #![allow(unused)]\n}\n\n// The #25851 shape: the loop only queues; ONE flush after the batch\n// applies every command and pays the sync machinery exactly once.\n// SEMANTIC CONTRACT: sound only where no observer needs PARTIAL\n// mid-batch state (bevy: \"no observer needs partial mid-despawn\n// state\") — a loop whose iterations READ the flushed state keeps the\n// per-iteration flush.\npub fn despawn_all(world: &mut World, ids: &[u32]) {\n    let mut q = CmdQueue { pending: Vec::new() };\n    for &id in ids {\n        q.push(Cmd::Despawn(id));\n    }\n    q.flush(world); // ONE sync point after the batch\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (4090/Windows release, 2026-09-24 — Batch 174 benches; verdict-stable ×2): 21.3x/13.3x/16.5x at n=1k/10k/100k commands (run 2: 20.7x/18.0x/17.7x — same verdict band). Alloc axis EXACTLY FLAT by construction (3/3 allocs, equal bytes at every size: the drain is in-place and the queue is pre-reserved in both arms — the win is pure timing, the barrier's fixed-cost share). MECHANISM ISOLATION, honestly scoped: the toy's deferred-command queue carries a per-flush FIXED sync cost — a 24-slot hook-dispatch chain (~35ns/flush measured here: branchy dependent loads, unfoldable and unvectorizable by construction) — and identical apply work in both arms, so the ratio IS the barrier's fixed-cost share at toy per-command costs (~1.7ns L1-resident). MODEL CAVEAT (both bounds disclosed): a vectorizable linear hook-sum (~1-2ns/flush) measures 1.1-1.6x on the same shape — the real-world per-flush fixed cost sits between the two models; the bevy PR that is this rule's source measured −2.4%..−10.6% END-TO-END on its own despawn benches because real per-entity despawn work (archetype moves, component drops) dwarfs the barrier — cite the app-layer number for adoption cases, never the toy's magnitude. The non-monotone dip at 10k (13.3x run 1) is sampling noise inside the verdict band, not a regime edge. SEMANTIC CONTRACT (the load-bearing adoption caveat, from the source's own justification): hoisting is sound ONLY where no observer needs PARTIAL mid-batch state — bevy's despawn_all qualifies because no observer needs partial mid-despawn state; a loop whose iterations READ the flushed state (spawn-then-immediately-query, deferred commands consumed mid-batch by other systems) MUST keep the per-iteration flush. FIX SPACE (narrow): World::flush/Commands::flush (or equivalent deferred-command barrier) call sites INSIDE batch loops, hoisted to once-after-batch. Distinct from bufwriter-batched (I/O syscall buffering — the barrier is a write() to the OS) and from channel-send-batching-unpark (wakeup batching — the barrier is a futex unpark): the same batch-the-barrier family over three DIFFERENT barriers; this one is application-level deferred-command application machinery. Suggestion-only (a parse-only proposer cannot prove the no-observer contract). Fixtures original reproductions; source: bevyengine/bevy #25851 (merged @ 1fb979e37365ef6e96fec085830796bd2312679a) — crates/bevy_ecs/src/world/entity_access/world_mut.rs (the despawn_no_free_no_flush_with_caller split) + world/mod.rs (the retain loop + the single post-loop flush). Batch 174 / Plan 168 / Research 205.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(deferred_command_flush_hoist_bench_spec()),
        },
    ]
}

/// Bench spec for `deferred-command-flush-hoist-out-of-batch-loop`: a
/// deferred-command queue whose flush pays a FIXED sync cost (a 24-slot
/// hook-dispatch chain — branchy dependent loads, unfoldable and
/// unvectorizable by construction) plus identical apply work in both arms,
/// so the measured ratio IS the barrier's fixed-cost share.
///
/// Buggy arm: push + flush per command (n sync points — the pre-#25851
/// `despawn_all` shape). Fixed arm: push all, ONE flush after the batch.
/// G1 asserts LIVE in release: identical per-entity world state through
/// both arms + the mechanism witness (`sync_ticks`: buggy pays the chain per
/// flush, fixed exactly once). Alloc axis: flat by construction (the drain
/// is in-place; the queue is pre-reserved in both arms — the win is pure
/// timing).
#[allow(clippy::too_many_lines)] // data table: harness + the live G1 block
#[cfg(feature = "rust_perf_bench")]
fn deferred_command_flush_hoist_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 100_000],
        harness: r#"
mod buggy {
    pub fn despawn_all(world: &mut crate::mach::World, ids: &[u32]) {
        let mut q = crate::mach::CmdQueue::with_capacity(ids.len());
        for &id in ids {
            q.push(crate::mach::Cmd::Despawn(id));
            q.flush(world); // flush per item — n sync points
        }
    }
}

mod fixed {
    pub fn despawn_all(world: &mut crate::mach::World, ids: &[u32]) {
        let mut q = crate::mach::CmdQueue::with_capacity(ids.len());
        for &id in ids {
            q.push(crate::mach::Cmd::Despawn(id));
        }
        q.flush(world); // ONE sync point after the batch
    }
}

mod mach {
    // A deferred-command queue + world at pattern level: push queues a
    // command; flush applies ALL pending commands AND drives the sync
    // machinery — a fixed 24-slot hook-dispatch chain every flush pays
    // regardless of pending count (the World::flush shape: deferred
    // application + the sync pass). The chain is data-DEPENDENT
    // (each slot index derived from the running value): unfoldable and
    // unvectorizable by construction — a linear const-sum would fold.
    pub struct CmdQueue {
        pending: Vec<Cmd>,
    }

    impl CmdQueue {
        pub fn with_capacity(cap: usize) -> Self {
            CmdQueue { pending: Vec::with_capacity(cap) }
        }

        #[inline]
        pub fn push(&mut self, cmd: Cmd) {
            self.pending.push(cmd);
        }

        #[inline]
        pub fn flush(&mut self, world: &mut World) {
            let mut sync = world.sync_hooks.len() as u32;
            for _ in 0..world.sync_hooks.len() {
                sync = world.sync_hooks[(sync as usize) % world.sync_hooks.len()];
            }
            world.sync_ticks = world.sync_ticks.wrapping_add(sync as u64);
            while let Some(cmd) = self.pending.pop() {
                match cmd {
                    Cmd::Despawn(id) => {
                        if world.alive[id as usize] {
                            world.alive[id as usize] = false;
                            world.alive_count -= 1;
                        }
                    }
                }
            }
        }
    }

    #[derive(Clone, Copy)]
    pub enum Cmd {
        Despawn(u32),
    }

    pub struct World {
        pub alive: Vec<bool>,
        pub alive_count: usize,
        pub sync_ticks: u64,      // observable: every flush's dispatch chain lands here
        pub sync_hooks: Vec<u32>, // RUNTIME state — a static const table folds away
    }
}

pub struct Input {
    ids: Vec<u32>,
    hooks: Vec<u32>,
}

pub fn make_input(n: usize) -> Input {
    // Data-dependent ids: descending + duplicated every 8th (despawning an
    // already-dead id is the no-op branch — the result is not statically
    // predictable), every 10th entity PRE-DEAD at init. Hook data through
    // black_box so the chain's operands are opaque to the optimizer.
    let mut ids: Vec<u32> = Vec::with_capacity(n + n / 8);
    let mut i = n as u32;
    while i > 0 {
        i -= 1;
        ids.push(i);
        if i % 8 == 0 && i > 0 {
            ids.push(i); // duplicate
        }
    }
    let hooks: Vec<u32> =
        std::hint::black_box((0..24u32).map(|k| k * 7 + 11).collect());

    // ── G1 — LIVE in release (plain assert, not debug_assert): both arms
    // produce IDENTICAL per-entity world state, and the mechanism witness
    // (sync_ticks: per-flush chain vs once) differs exactly as designed.
    {
        let mk = || mach::World {
            alive: (0..n).map(|k| k % 10 != 0).collect(),
            alive_count: (0..n).filter(|&k| k % 10 != 0).count(),
            sync_ticks: 0,
            sync_hooks: hooks.clone(),
        };
        let mut w_a = mk();
        buggy::despawn_all(&mut w_a, &ids);
        let mut w_b = mk();
        fixed::despawn_all(&mut w_b, &ids);
        assert_eq!(w_a.alive_count, w_b.alive_count, "alive_count must match");
        assert!(w_a.alive == w_b.alive, "per-entity world state must match");
        assert!(w_a.sync_ticks > w_b.sync_ticks && w_b.sync_ticks > 0,
            "mechanism witness: buggy pays the dispatch chain per flush, fixed exactly once");
    }

    Input { ids, hooks }
}

fn fresh_world(n_max: usize, hooks: &[u32]) -> mach::World {
    mach::World {
        alive: (0..n_max).map(|k| k % 10 != 0).collect(),
        alive_count: (0..n_max).filter(|&k| k % 10 != 0).count(),
        sync_ticks: 0,
        sync_hooks: hooks.to_vec(),
    }
}

pub fn run_buggy(input: &Input) {
    let n_max = input.ids.iter().copied().max().map(|m| m as usize + 1).unwrap_or(0);
    let mut world = fresh_world(n_max, &input.hooks);
    buggy::despawn_all(&mut world, &input.ids);
    let ck: u64 = world.alive.iter().map(|&a| a as u64).sum::<u64>()
        + world.alive_count as u64
        + world.sync_ticks;
    std::hint::black_box(ck);
}

pub fn run_fixed(input: &Input) {
    let n_max = input.ids.iter().copied().max().map(|m| m as usize + 1).unwrap_or(0);
    let mut world = fresh_world(n_max, &input.hooks);
    fixed::despawn_all(&mut world, &input.ids);
    let ck: u64 = world.alive.iter().map(|&a| a as u64).sum::<u64>()
        + world.alive_count as u64
        + world.sync_ticks;
    std::hint::black_box(ck);
}
"#
        .to_string(),
    }
}

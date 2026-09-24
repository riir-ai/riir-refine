//! Entries slice 08 — Batch 174 (2026-09-24, Plan 168 / Research 205): the
//! host-side lock-scope law + the deferred-command barrier batching — both
//! distilled from merged upstream fixes (tracel-ai/cubecl #1678 + #1686,
//! bevyengine/bevy #25851), quotes re-verified against the merged diffs.
//! Batch 177 (2026-09-25, Plan 170 / Research 207) appends the two
//! measurement-honesty members from the EXL3/GB10 serving-recipe distill
//! (family members #3 and #4), quotes re-verified at the pin `e0ebee8d`.

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

        // ── complexity (measurement honesty #3): before trusting an
        // output-equality fidelity gate, run the A/A control ────────────
        // Source: vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe
        // @ e0ebee8d — README §"How things were measured" +
        // scripts/probe_greedy.py (marked as superseded in-tree, the
        // honest form). Family: B109 (the work), B115 (the config),
        // this (the comparator).
        PerfRule {
            rule_id: "aa-control-prices-the-fidelity-instrument".to_string(),
            name: "output-text equality as a fidelity gate → run the A/A control first (an engine non-deterministic at temperature 0 makes a text diff measure NOISE) → fall back to a counter-based signal and disclose the prompt-class conditioning".to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"use std::collections::HashMap;

// A fidelity suite that scores A/B by EXACT OUTPUT-TEXT EQUALITY and
// never runs the A/A control. The engine under test is non-deterministic
// at temperature 0 with a fixed seed: 4 distinct outputs from 5 identical
// open-ended requests (1 of 5 constrained). The suite's prompts are all
// tightly constrained, so it reports a clean instrument that has no power.
fn fidelity_gate_text_diff(
    variant_a: &[String],
    variant_b: &[String],
) -> f64 {
    let identical = variant_a
        .iter()
        .zip(variant_b)
        .filter(|(a, b)| a == b)
        .count();
    identical as f64 / variant_a.len() as f64
    // 2/6 identical — but the A/A control (same config vs itself) ALSO
    // scores 2/6: "the control is indistinguishable from the test, so a
    // text diff measures engine noise" — the gate certified nothing
}

fn suite_prompts() -> Vec<String> {
    // all tightly CONSTRAINED prompts — the class that does NOT diverge;
    // the open-ended class (where 4-of-5 distinct was measured) is absent
    vec![
        "Reply with exactly: OK".to_string(),
        "Translate to German: cat".to_string(),
        "2 + 2 = ?".to_string(),
    ]
}"#.to_string(),
            fixed_code: r#"use std::collections::HashMap;

// Fixed: the A/A control is RUN and COMPARED to the A/B before the
// comparator is trusted; a counter-based signal (acceptance counters)
// replaces text equality when the control is indistinguishable; the
// prompt-class conditioning is disclosed beside the number.
struct FidelityReadout {
    ab_identical: f64,
    aa_identical: f64, // the CONTROL — same config against itself
    acceptance_rate: f64, // counter-based: the signal that survives noise
    prompt_class_note: &'static str,
}

fn fidelity_gate_with_aa_control(
    variant_a: &[String],
    variant_b: &[String],
    same_a: &[String],
    same_a2: &[String],
    acceptance: f64,
) -> FidelityReadout {
    let frac = |x: &[String], y: &[String]| {
        x.iter().zip(y).filter(|(p, q)| p == q).count() as f64 / x.len() as f64
    };
    let ab = frac(variant_a, variant_b);
    let aa = frac(same_a, same_a2);
    // THE RULE: an A/A disagreement rate at or above the A/B rate means the
    // instrument has NO POWER — fall back to the counter-based signal.
    let _ = if aa >= ab {
        // text diff measures engine noise here; the acceptance counter is
        // the only signal that survives (spec-on vs spec-off scored the
        // same 2-of-6 as spec-on vs spec-on — measured, at the source)
        acceptance
    } else {
        ab
    };
    FidelityReadout {
        ab_identical: ab,
        aa_identical: aa,
        acceptance_rate: acceptance,
        // near-ties in the logits resolved by varying reduction order:
        // open-ended generations diverge, tightly constrained ones do not —
        // a constrained-only suite reports a clean instrument that has no
        // power, so the class mix is part of the readout
        prompt_class_note: "divergence is prompt-class-conditional; disclose the mix",
    }
}"#.to_string(),
            context_notes: Some("NotBenchable (measurement-honesty #3, the B109 precedent in this exact family — B109 is about the WORK, the 2609.04040 rule about the DESIGN, B115 about the CONFIG; this is the first member about the COMPARATOR): the rule's content is that a fidelity comparator has a NOISE FLOOR which must be measured before the comparator is trusted, so there is no latency quantity to Pareto-ratio. The machine-checkable surrogate is the A/A DISAGREEMENT RATE — a counted quantity with a known direction (A/A at or above A/B ⇒ the instrument has no power; the same shape as B109's zero-baseline assert). MEASURED at the source (README 'How things were measured' + probe_greedy.py, which the author marks as predating the finding and leaves in-tree with the reason — the honest superseded-instrument form): the engine is non-deterministic at temperature 0 with a fixed seed — 4 distinct outputs from 5 identical requests open-ended, 1 of 5 constrained; spec-on vs spec-on scored the SAME 2-of-6 identical as spec-on vs spec-off, 'the control is indistinguishable from the test, so a text diff measures engine noise'; fall back to acceptance counters from /metrics. SECOND CLAUSE: the divergence is PROMPT-CLASS-CONDITIONAL (open-ended diverge, tightly constrained do not — 'which is what near-ties in the logits resolved by varying reduction order would look like'), so a constrained-only fidelity suite reports a clean instrument with NO power — the class mix is part of the disclosure. LOAD-BEARING AT HOME: our G1 bit-identity gates assume a deterministic reference; this rule states the precondition that assumption needs, and names the fallback (counters) when it does not hold. Suggestion-only (a parse-only proposer cannot see which comparator a harness trusts). Fixtures original reproductions; source: vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe @ e0ebee8ddc4391b5e66d86fdce993b0756e00986 (MIT; provenance three-deep — vcruz305/exllamav3 PR #2 → turboderp-org/exllamav3 → QTIP arXiv:2406.11235) — README §'How things were measured' + scripts/probe_greedy.py. Batch 177 / Plan 170 / Research 207.".to_string()),
            // Not benchable: no latency quantity to ratio (the comparator's
            // noise floor IS the content); surrogate = the A/A rate.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },

        // ── complexity (measurement honesty #4): the harness's own console
        // write is a host sync whose bias SCALES WITH CHANGE CLASS ─────
        // Source: tuning/README.md header ("Quote chat.py numbers, not
        // in-process ones") + patch 0003 commit body + README §"How
        // things were measured".
        PerfRule {
            rule_id: "observer-sync-bias-differs-by-change-class".to_string(),
            name: "an in-process harness's per-token console write is a host sync, so its bias SCALES WITH CHANGE CLASS (~4 tok/s for kernel changes, ~10 for host-side) → quote the harness matching the deployed consumer and SAY which one you quoted; a uniform bias licenses the ratio, never the absolute".to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"// A measurement note that quotes IN-PROCESS harness numbers as the
// engine's speed. The harness's per-token console write is itself a host
// sync, so in-process reads run ~4 tok/s high for KERNEL changes and ~10
// for HOST-SIDE ones — the bias is not a constant offset, and the same
// patch measures +12 t/s in-process but inside variance (±2 t/s) through
// the deployed chat.py consumer.
fn report_speed(engine: &str, in_process_tps: f64) -> String {
    format!("{engine}: {in_process_tps} tok/s")
    // quoted WITHOUT the harness class: in-process and deployed-consumer
    // numbers differ by a bias that scales with the change class — the
    // reader cannot tell which world this describes
}"#.to_string(),
            fixed_code: r#"// Fixed: the number is quoted FROM the harness that matches the DEPLOYED
// consumer (chat.py-class), the harness class is NAMED beside it, and any
// known-uniform bias is stated with the direction it licenses (ratios ok,
// absolutes not).
struct SpeedReading {
    tps: f64,
    harness_class: &'static str, // "deployed-consumer (chat.py-class)" | "in-process"
    bias_note: &'static str,
}

fn report_speed_honest(in_process_tps: f64, chat_py_tps: f64, change_class: &str) -> SpeedReading {
    // the bias scales with the change class: ~4 tok/s (kernel changes) vs
    // ~10 (host-side) — it does NOT cancel in a ratio across classes
    let _ = (in_process_tps, change_class);
    SpeedReading {
        tps: chat_py_tps, // "kept because in-process and server (TabbyAPI-class)
                          // callers see the full effect" — quote the class that
                          // matches the deployed consumer
        harness_class: "deployed-consumer (chat.py-class)",
        bias_note: "under speculation TTFT absorbs k+1 tokens that still count in n:
            decode runs ~2% high, UNIFORM across configurations, so comparisons
            (ratios) hold and absolutes do not",
    }
}"#.to_string(),
            context_notes: Some("NotBenchable (measurement-honesty #4, the B109/Batch-136 precedent — a documentation-with-teeth rule; the B109 and 2609.04040 rules are about the work and the design; this is about the OBSERVER's interaction with the deployed consumer). THE NOVEL CONTENT is that the instrument bias is NOT a constant offset — it SCALES WITH THE CLASS OF CHANGE UNDER TEST (an in-process harness's per-token console write is itself a host sync: reads ~4 tok/s high for kernel changes and ~10 for host-side ones), so the same patch measures +12 t/s in-process and inside variance (±2 t/s) through chat.py — a constant offset would cancel in a ratio and this one does not, across classes. THE FIX SHAPE (tuning/README.md header, verbatim): 'Quote `chat.py` numbers, not in-process ones' — quote the harness that matches the DEPLOYED consumer, and say which one you quoted; the patch was 'kept because in-process and server (TabbyAPI-class) callers see the full effect.' COMPANION CLAUSE (same section): under speculation TTFT absorbs k+1 tokens that still count in n, so decode figures run ~2% high — 'uniform across configurations, so comparisons hold': a KNOWN-UNIFORM bias licenses the RATIO and never the absolute. RIDER from the same source (the sampler-pinning discipline, quoted beside it): 'Measure with -topk 1. The default temperature-0.8 sampler moves MTP acceptance 58–75% run to run and the tok/s number with it' — pin the sampler before quoting tok/s at all. Distinct from dead-result-timing-loop-folded-under-lto (B109 — the harness's own work must not vanish; this is the observer's OVERHEAD biasing the observed), from instrumented-times-are-not-production-times (the ds4 profiler rule — instrumented ABSOLUTES can't set budgets; this adds that the bias is change-class-scaled and non-canceling), from aa-control-prices-the-fidelity-instrument (same-batch kin — the comparator's noise floor; this is the harness's direction-dependent bias; compose: run BOTH). Suggestion-only. Fixtures original reproductions; source: vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe @ e0ebee8d (MIT) — tuning/README.md header + patch 0003 commit body + README §'How things were measured'. Batch 177 / Plan 170 / Research 207.".to_string()),
            // Not benchable: the content is the observer's bias, not a
            // faster path; no ratio to measure.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
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

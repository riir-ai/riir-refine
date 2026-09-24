//! Entries slice 08 — Batch 174 (2026-09-24, Plan 168 / Research 205): the
//! host-side lock-scope law + the deferred-command barrier batching — both
//! distilled from merged upstream fixes (tracel-ai/cubecl #1678 + #1686,
//! bevyengine/bevy #25851), quotes re-verified against the merged diffs.
//! Batch 177 (2026-09-25, Plan 170 / Research 207) appends the two
//! measurement-honesty members from the EXL3/GB10 serving-recipe distill
//! (family members #3 and #4), quotes re-verified at the pin `e0ebee8d`.
//! Batch 179 (2026-09-25, Plan 172 / Research 210) added the rsconstmap
//! distill trio: paired-probe-layout-present-vs-absent (layout) +
//! block-hash-before-gather-mlp (iterators) +
//! immutable-static-map-filter-selection (complexity, `[dep: constmap]`).

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
        // ── layout: two parallel arrays probed at the same indices per
        // lookup → zip into 16-byte [value, fingerprint] slots ────────
        // Source: lemire/rsconstmap @ bf7b3341 (Research 210), the
        // PairedVerifiedConstMap layout — VerifiedConstMap keeps values
        // and fingerprints in two separate arrays; the paired variant
        // stores each value next to its fingerprint so the three
        // positions a lookup reads each yield both words from one cache
        // line (three lines instead of six), and each slot XORs as one
        // 128-bit word on the baseline SSE2/NEON ISAs (no feature
        // detection). The doc's own both-sides table is the rule's
        // regime: present keys win ~20%, absent keys LOSE (the split
        // layout's fingerprint array is half the map and early-exits
        // before touching values).
        PerfRule {
            rule_id: "paired-probe-layout-present-vs-absent".to_string(),
            name: "two parallel arrays probed at the same indices per lookup (fingerprint gate + value) → interleave into 16-byte [value, fingerprint] slots — a present-key probe reads 3 cache lines instead of 6 and each slot XORs as one 128-bit word on baseline SSE2/NEON; ABSENT keys regress (the split layout's fingerprint array is half the map and early-exits before the values — pairing doubles the miss working set), so the gate is probe hit rate, not size".to_string(),
            category: "layout".to_string(),
            buggy_code: "// Two PARALLEL arrays probed at the same three indices per lookup: the\n// fingerprint word (membership gate) and the value word live in separate\n// arrays — a present-key probe touches SIX cache lines (three per array)\n// where a zipped layout touches three.\npub struct FingerprintGate {\n    checks: Vec<u64>, // fingerprint word per position\n    data: Vec<u64>,   // value word per position\n}\n\n/// One lookup: the key plus its expected fingerprint XOR (recorded when\n/// the map was built — the construct-then-verify probe shape).\npub struct Query {\n    pub key: u64,\n    pub expect_fp: u64,\n}\n\n#[inline]\nfn mix(mut x: u64) -> u64 {\n    x ^= x >> 30;\n    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);\n    x ^= x >> 27;\n    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);\n    x ^= x >> 31;\n    x\n}\n\n#[inline]\nfn fastrange(h: u64, m: u64) -> usize {\n    // The source's own index form: the high 64 bits of h*m — uniform in\n    // [0, m), one pipelined multiply-high, no division.\n    (((h as u128) * (m as u128)) >> 64) as usize\n}\n\nimpl FingerprintGate {\n    /// Three probe positions derived from one mixed hash.\n    #[inline]\n    fn triple(&self, key: u64) -> [usize; 3] {\n        let m = self.checks.len() as u64;\n        let h = mix(key);\n        [\n            fastrange(h, m),\n            fastrange(h.rotate_left(21), m),\n            fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),\n        ]\n    }\n\n    /// None = fingerprint mismatch (absent); Some = the value word.\n    pub fn probe(&self, q: &Query) -> Option<u64> {\n        let [i0, i1, i2] = self.triple(q.key);\n        let fp = self.checks[i0] ^ self.checks[i1] ^ self.checks[i2];\n        if fp != q.expect_fp {\n            return None; // absent: only the checks array (HALF the map)\n                         // was touched — the value array stays cold\n        }\n        Some(self.data[i0] ^ self.data[i1] ^ self.data[i2])\n    }\n}".to_string(),
            fixed_code: "/// One 16-byte position: the value word and its fingerprint word, side\n/// by side — a probe reads both from ONE cache line, and the pair XORs\n/// as one 128-bit vector.\npub type Slot = [u64; 2]; // [value, fingerprint]\n\npub struct PairedFingerprintGate {\n    slots: Vec<Slot>,\n}\n\npub struct Query {\n    pub key: u64,\n    pub expect_fp: u64,\n}\n\n#[inline]\nfn mix(mut x: u64) -> u64 {\n    x ^= x >> 30;\n    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);\n    x ^= x >> 27;\n    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);\n    x ^= x >> 31;\n    x\n}\n\n#[inline]\nfn fastrange(h: u64, m: u64) -> usize {\n    (((h as u128) * (m as u128)) >> 64) as usize\n}\n\nimpl PairedFingerprintGate {\n    #[inline]\n    fn triple(&self, key: u64) -> [usize; 3] {\n        let m = self.slots.len() as u64;\n        let h = mix(key);\n        [\n            fastrange(h, m),\n            fastrange(h.rotate_left(21), m),\n            fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),\n        ]\n    }\n\n    pub fn probe(&self, q: &Query) -> Option<u64> {\n        let [i0, i1, i2] = self.triple(q.key);\n        let [value, fp] = xor3(&self.slots[i0], &self.slots[i1], &self.slots[i2]);\n        if fp != q.expect_fp {\n            return None; // absent keys DRAG THE VALUE IN alongside the\n                         // fingerprint — the miss working set doubled\n        }\n        Some(value)\n    }\n}\n\n/// XOR of three slots: value in [0], fingerprint in [1]. SSE2/NEON are\n/// part of the x86-64/AArch64 baseline, so the 128-bit form needs NO\n/// runtime feature detection; other targets take the scalar form.\n#[inline(always)]\nfn xor3(a: &Slot, b: &Slot, c: &Slot) -> Slot {\n    #[cfg(target_arch = \"x86_64\")]\n    {\n        use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128, __m128i};\n        // SAFETY: SSE2 is baseline on x86-64; each reference is a live\n        // 16-byte slot, which is all an unaligned 128-bit load reads.\n        unsafe {\n            let v = _mm_loadu_si128(a.as_ptr().cast::<__m128i>());\n            let v = _mm_xor_si128(v, _mm_loadu_si128(b.as_ptr().cast::<__m128i>()));\n            let v = _mm_xor_si128(v, _mm_loadu_si128(c.as_ptr().cast::<__m128i>()));\n            let mut out: Slot = [0; 2];\n            _mm_storeu_si128(out.as_mut_ptr().cast::<__m128i>(), v);\n            out\n        }\n    }\n    #[cfg(target_arch = \"aarch64\")]\n    {\n        use std::arch::aarch64::{veorq_u64, vgetq_lane_u64, vld1q_u64};\n        // SAFETY: NEON is baseline on AArch64; each vld1q_u64 reads one\n        // live 16-byte slot.\n        unsafe {\n            let v = vld1q_u64(a.as_ptr());\n            let v = veorq_u64(v, vld1q_u64(b.as_ptr()));\n            let v = veorq_u64(v, vld1q_u64(c.as_ptr()));\n            [vgetq_lane_u64::<0>(v), vgetq_lane_u64::<1>(v)]\n        }\n    }\n    #[cfg(not(any(target_arch = \"x86_64\", target_arch = \"aarch64\")))]\n    {\n        [a[0] ^ b[0] ^ c[0], a[1] ^ b[1] ^ c[1]]\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max release, 2026-09-25 — Batch 179 benches; verdict-stable ×2: 1.68x/1.45x/1.80x then 1.68x/1.55x/1.82x at n=262_144/4_194_304/16_777_216 table positions; 262_144 present-key probes per timed call, random order, query-order key buffer — the source's own bench-honesty doctrine, adapted: the whole-set walk is not validator-compatible at 256 MB and the fixed random slice stands in, disclosed). Alloc axis exactly flat by construction (16 B/position both arms — the layout moves the same bytes; 0/0 allocs; the G1 pair-equality + absent-None early-exit checks run live in every bench invocation's make_input). THE MECHANISM (lemire/rsconstmap @ bf7b3341 lib.rs:709-795, the PairedVerifiedConstMap doc + xor3_slots): two parallel arrays probed at the same three indices per lookup (fingerprint gate + value) → interleave into 16-byte [value, fingerprint] slots — a present-key probe touches THREE cache lines instead of SIX and each slot XORs as one 128-bit word on SSE2/NEON, both baseline ISAs, so NO runtime feature detection (the runtime-feature-detect-per-chunk adjacency: detection is unnecessary here — folded, not a rule). THE ABSENT-KEY REGRESSION IS THE REGIME CONTENT, quoted from the source's own doc: on an absent key the SPLIT layout reads only the fingerprint array — half the map, far more likely cache-resident — and early-exits before touching values, while the PAIRED layout drags the value in alongside the fingerprint (the miss working set doubles); 'Use Paired when most keys you look up are present, Verified when most are absent.' The source measured ~20% present-key wins on Xeon Gold 6548N (~21% M4 Max in the README — the two figures disagree in-source; cite both) once the map outgrows cache; our cells measure MORE (1.45-1.82x) because the probe-only harness isolates the line-traffic share (6 lines → 3-4 per probe binds on bandwidth once misses pipeline, not on per-probe latency) — never cite ours as the end-to-end number, the source's ~20% is that. REGIME: probe HIT RATE is the gate (mostly-present lookups), secondarily map size past cache; the timed arms carry only the present cell — the absent regression is the adoption gate, not the A/B. Distinct from soa-hot-field BOTH directions (that splits a struct so ONE hot field scans contiguously; this ZIPS two co-probed arrays for random triple probes — the inverse move at the same byte count) and from layout-reorder (field padding inside one struct, not cross-array co-probing). Fixtures ORIGINAL: the recorded-fingerprint stand-in reproduces the access pattern without porting the fuse solve (the construction stays a library, the D1 drop); the fixed arm carries the source's own xor3 shape (baseline-SIMD + scalar fallback). The whole-file contract (struct + ctor + read sites) makes this suggestion-only — the soa-hot-field class. Source: lemire/rsconstmap @ bf7b33419d37c5d141075b9c1642fb69642a7896 (MIT OR Apache-2.0) src/lib.rs + README §Paired layout; upstream lemire/constmap (Go) + fastconstmap (C); the math: Graf & Lemire, Binary Fuse Filters, ACM JEA 2022. Batch 179 / Plan 172 / Research 210.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(paired_probe_layout_bench_spec()),
        },
        // ── iterators: loop of hash-then-dependent-gather → block that
        // hashes the whole block first, THEN issues the gathers (MLP) ──
        // Source: lemire/rsconstmap @ bf7b3341 src/mapmany.rs:11-23 (the
        // BATCH_BLOCK doc) + :52-85 (the two-phase block loop). The
        // in-source law: hashing a whole block first lets every key's
        // array accesses be in flight at once — "That memory-level
        // parallelism is the only reason a batch beats a loop over map."
        // BATCH_BLOCK = 8 is the source's own two-machine adjudication
        // (annotated, never gated).
        PerfRule {
            rule_id: "block-hash-before-gather-mlp".to_string(),
            name: "a lookup loop whose every iteration hashes a key then immediately issues its dependent scattered gathers → block of 8: resolve the whole block's indices first (stack arrays), then gather — all 8 keys' table reads are in flight at once (memory-level parallelism); pays only past last-level cache, where load latency is the regime".to_string(),
            category: "iterators".to_string(),
            buggy_code: "// The per-key shape: each iteration resolves one key's positions and\n// immediately issues its three scattered table reads — the NEXT key's\n// addresses are not computed until this key's chain finishes, so the\n// out-of-order window holds only one key's loads at a time.\n#[inline]\nfn mix(mut x: u64) -> u64 {\n    x ^= x >> 30;\n    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);\n    x ^= x >> 27;\n    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);\n    x ^= x >> 31;\n    x\n}\n\n#[inline]\nfn fastrange(h: u64, m: u64) -> usize {\n    // The source's own index form: the high 64 bits of h*m — uniform in\n    // [0, m), one pipelined multiply-high, no division.\n    (((h as u128) * (m as u128)) >> 64) as usize\n}\n\n#[inline]\nfn triple(key: u64, m: u64) -> [usize; 3] {\n    let h = mix(key);\n    [\n        fastrange(h, m),\n        fastrange(h.rotate_left(21), m),\n        fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),\n    ]\n}\n\npub fn gather_all(table: &[u64], keys: &[u64], out: &mut [u64]) {\n    for (o, &k) in out.iter_mut().zip(keys) {\n        let [i0, i1, i2] = triple(k, table.len() as u64);\n        *o = table[i0] ^ table[i1] ^ table[i2]; // hash chain → gather,\n        // per key: the gathers of key j+1 wait behind key j's hash\n    }\n}".to_string(),
            fixed_code: "// The blocked shape: resolve the WHOLE block's positions first (stack\n// arrays — no table reads yet), then issue every gather — all 8 keys'\n// table reads are independent and in flight at once.\n#[inline]\nfn mix(mut x: u64) -> u64 {\n    x ^= x >> 30;\n    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);\n    x ^= x >> 27;\n    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);\n    x ^= x >> 31;\n    x\n}\n\n#[inline]\nfn fastrange(h: u64, m: u64) -> usize {\n    (((h as u128) * (m as u128)) >> 64) as usize\n}\n\n#[inline]\nfn triple(key: u64, m: u64) -> [usize; 3] {\n    let h = mix(key);\n    [\n        fastrange(h, m),\n        fastrange(h.rotate_left(21), m),\n        fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),\n    ]\n}\n\npub const BLOCK: usize = 8; // the source's adjudicated block: 8 wins or\n// nearly wins everywhere; 16 wins one machine's cold cell and loses its\n// hot one — annotate, never gate.\n\npub fn gather_all(table: &[u64], keys: &[u64], out: &mut [u64]) {\n    let m = table.len() as u64;\n    let mut i0s = [0usize; BLOCK];\n    let mut i1s = [0usize; BLOCK];\n    let mut i2s = [0usize; BLOCK];\n    let blocks = keys.len() / BLOCK * BLOCK;\n    let mut b = 0;\n    while b < blocks {\n        // Phase 1: every address resolved before any table read.\n        for j in 0..BLOCK {\n            let [i0, i1, i2] = triple(keys[b + j], m);\n            i0s[j] = i0;\n            i1s[j] = i1;\n            i2s[j] = i2;\n        }\n        // Phase 2: the block's gathers are independent — in flight at once.\n        for j in 0..BLOCK {\n            out[b + j] = table[i0s[j]] ^ table[i1s[j]] ^ table[i2s[j]];\n        }\n        b += BLOCK;\n    }\n    // Tail: fewer than BLOCK keys left — the plain loop.\n    for (o, &k) in out[blocks..].iter_mut().zip(&keys[blocks..]) {\n        let [i0, i1, i2] = triple(k, m);\n        *o = table[i0] ^ table[i1] ^ table[i2];\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 2/0/1 (M3 Max release, 2026-09-25 — Batch 179 benches; verdict-stable ×2: 1.68x/1.30x/1.02x-tie then 1.25x/1.25x/1.02x-tie at n=262_144/4_194_304/16_777_216 table positions, 4_096 keys per timed call, random order, query-order key buffer; the small-cell ratio moved between invocations — 1.68 vs 1.25, first-execution cache state — the verdict held; allocs 0/0: the output buffer is a 32 KiB stack array in both arms, the phase-1 index arrays are [usize; 8] stack). G1 bit-identity (blocked == loop incl. the tail < BLOCK path) runs live in every bench invocation. THE MECHANISM (mapmany.rs:11-23 + 52-85): a loop whose every iteration hashes a key then immediately issues its three scattered gathers pays the hash/index chain before EVERY load; the block form resolves the whole block's indices first (three stack arrays) and only then gathers — all 8 keys' table reads in flight at once; 'That memory-level parallelism is the only reason a batch beats a loop over map' (in-source). BLOCK=8 is the source's own two-machine adjudication — 'Eight wins or nearly wins everywhere; 16 wins one case and loses another' — annotated, never gated. REGIME, measured honestly and it DIVERGES from the source's telling: on this box the win sits at L2/SLC-resident tables (1.25-1.68x) where the phase split un-overlaps the index-derivation chain, while the 256 MB cell ties (1.02x) — past-DRAM the loop's own out-of-order window already saturates load MLP and the batching has nothing left to hide; the source measured its wins on M4 Max in BOTH cold and hot regimes and a Xeon split (16 about 17% better cold, ~8% worse hot). The Go-port note folds in: Go hand-hoists batched hashing because xxhash.Sum64String reloads primes per call — in Rust the hash inlines into the loop either way, half the Go gain is already free, so the Rust-side expectation is exactly this modest band. FIXTURE LESSON on record: the first draft derived indices with % m and measured all-ties at every size — the divider chain (three non-pipelined divides per key) dominated and masked the regime; the source's own multiply-high index form (the high 64 bits of h*m) is the faithful fixture. Distinct from software-prefetch-storage-bound (removes prefetch HINTS on streaming loads; no hints exist here — this restructures a dependent-gather loop), from dual-mask-fused-block-scan (fuses two PREDICATES per block — compute fusion; this DEFERS dependent gathers — dependency breaking), and from the kernel_opt batch-width family (GPU occupancy; host-side scalar path here). Suggestion-only (a loop restructure; a parse-only proposer cannot prove the table/cache geometry or the hash chain length). Source: lemire/rsconstmap @ bf7b3341 src/mapmany.rs (ConstMap::map_many_into); upstream lemire/constmap v1.1.0. Batch 179 / Plan 172 / Research 210.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(block_hash_before_gather_bench_spec()),
        },
        // ── complexity: the std-map-selection family's missing regime
        // cell — runtime-built, immutable, lookup-only, 10k+ keys → a
        // construct-once binary-fuse static map ─────────────────────
        // Source: lemire/rsconstmap @ bf7b3341 (README §Performance
        // gains/§Scaling; lib.rs ConstMap). Completes the family lattice:
        // keyword-map-to-match (compile-time literals) / fxhashmap
        // (mutable hot) / btreemap-sorted-iteration (ordered reads) /
        // o1-lookup (scan→map) / THIS (runtime-built immutable). The
        // library IS the fix — benchable via [dep: constmap], the
        // papaya dep precedent.
        PerfRule {
            rule_id: "immutable-static-map-filter-selection".to_string(),
            name: "HashMap<String, u64> built ONCE from runtime data, never mutated, lookup-dominated → a construct-once binary-fuse static map (constmap): one hash + three array reads + two XORs per lookup and the key STORAGE DROPPED (~9 vs 41-52 B/key at 1M keys); contracts: absent keys undefined, no enumeration, construction slower (amortize)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::collections::HashMap;\n\n/// An entity table built once at startup from the world snapshot and\n/// only ever READ afterward — every hot lookup pays SipHash over the\n/// full key PLUS the map's bucket walk, and the key material itself is\n/// stored forever.\npub struct EntityTable {\n    values: HashMap<String, u64>,\n}\n\nimpl EntityTable {\n    pub fn build(keys: Vec<String>, values: Vec<u64>) -> Self {\n        Self {\n            values: keys.into_iter().zip(values).collect(),\n        }\n    }\n\n    pub fn value(&self, key: &str) -> Option<u64> {\n        self.values.get(key).copied()\n    }\n}".to_string(),
            fixed_code: "use constmap::ConstMap;\n\n/// The same built-once, read-only table as a construct-once binary-fuse\n/// static map: lookup = one hash + three array reads + two XORs, and the\n/// keys are DROPPED after construction (the fuse solves for the values;\n/// only the ~9 B/key word array remains).\n///\n/// CONTRACTS (the adoption gates): absent keys are UNDEFINED (no\n/// membership — use the +9 B/key Verified variant when misses must\n/// report NOT_FOUND); the keyset is not enumerable; construction is\n/// slower than a HashMap build — amortize over the lookup-dominated\n/// life of the table.\npub struct EntityTable {\n    values: ConstMap,\n}\n\nimpl EntityTable {\n    pub fn build(keys: Vec<String>, values: Vec<u64>) -> Self {\n        let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();\n        Self {\n            values: ConstMap::new(&refs, &values).expect(\"fuse solve\"),\n        }\n    }\n\n    /// Present keys only — see the CONTRACTS above for the absent-key\n    /// posture (an absent lookup here returns garbage, never an error).\n    pub fn value(&self, key: &str) -> u64 {\n        self.values.map(key)\n    }\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 2/0/0 (M3 Max release, 2026-09-25 — Batch 179 benches; verdict-stable ×2: 2.11x/4.27x then 2.09x/4.13x at n=100_000/1_000_000 keys, 100_000 present-key lookups per timed call, random query order, query-order buffer; allocs 0/0 — both maps + the query buffer are built in make_input; G1 — the static map's values equal the HashMap's on present keys — runs live in every bench invocation's first 1_000 probes). THE MECHANISM: a HashMap<String, u64> built once from runtime data, never mutated, lookup-dominated → a construct-once binary-fuse static map (constmap 0.3; the library IS the fix — the [dep: constmap] papaya precedent): lookup = one hash + three array reads + two XORs, and the KEY STORAGE IS DROPPED after construction (~9 B/key vs 41-52 B/key at 1M — the source's README scaling table). The source measured 7.0 ns vs 54.6 ns at 1M keys (M4 Max, ≈7.8x); our 4.1-4.3x at the same size is the honest same-regime cell on shorter fixed-width keys (entity:%012d — SipHash-friendlier than their varied keyset); the margin GROWS with key count on both machines — the scaling story transfers, the absolute ratio does not. FAMILY LATTICE COMPLETION — the std-map-selection family's missing regime cell: keyword-map-to-match (compile-time literals, ≤ a dozen keys), fxhashmap (mutable hot map), btreemap-sorted-iteration (ordered reads), o1-lookup (scan→map), THIS (runtime-built, 10k+, immutable, lookup-only). ADOPTION CONTRACTS (from the source's docs, each a gate): absent keys are UNDEFINED — no membership; the +9 B/key Verified variant reports NOT_FOUND (and PairedVerifiedConstMap zips value+fingerprint per the paired-probe-layout-present-vs-absent rule, same batch); no enumeration — the keyset is not recoverable; construction is SLOWER than a HashMap build — amortize over the lookup-dominated life; the 32-byte serialized header pads both u64 arrays to 8-alignment so an mmapped map can alias as &[u64] (an enabling decision, no perf number — folded). Distinct from hashmap-with-capacity (build tuning of the SAME structure) and from keyword-map-to-match (the literals regime; the oxc negative there — a gperf-class perfect hash could not beat a match — is the small-keyset contrast this rule's 10k+ gate answers). Suggestion-only (a library swap; a parse-only proposer cannot prove immutability or lookup-dominance). Source: lemire/rsconstmap @ bf7b3341 README §Performance gains/§Scaling/§Serialization + lib.rs ConstMap; upstream lemire/constmap (Go); math: Graf & Lemire, Binary Fuse Filters, ACM JEA 2022 (DOI 10.1145/3510449). Batch 179 / Plan 172 / Research 210.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(immutable_static_map_bench_spec()),
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

/// Bench spec for `paired-probe-layout-present-vs-absent`: split
/// (checks + data) vs paired ([u64; 2] slots) fingerprint-gated probes
/// over present keys — the paired regime (the source's own adjudication:
/// "Use Paired when most keys you look up are present"). Sizes are table
/// positions; `262_144` probes per timed call in random order with the
/// query buffer built in query order (the source's bench-honesty
/// doctrine). The small size is the in-cache anti-regime cell; the
/// absent-key regression is the rule's regime content, disclosed in the
/// annotation and exercised by the G1 (both arms answer None on a
/// corrupted fingerprint). G1 pair-equality runs live in `make_input`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)]
#[allow(clippy::too_many_lines)] // bench-spec harness assembler (the corpus convention)
fn paired_probe_layout_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![262_144, 4_194_304, 16_777_216],
        harness: r#"
// Fingerprint-gated probe map in two layouts; present-key probes (the
// paired regime). The whole-set walk is not validator-compatible at
// 256 MB (a multi-second call starves the 100-iteration warmup), so a
// 262_144-key random slice stands in — disclosed in the rule annotation.
const QUERIES: usize = 262_144;

mod common {
    #[derive(Clone, Copy)]
    pub struct Query {
        pub key: u64,
        pub expect_fp: u64,
    }

    #[inline]
    pub fn mix(mut x: u64) -> u64 {
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        x
    }

    /// The source's own index form: the high 64 bits of h*m — uniform in
    /// [0, m), one pipelined multiply-high, no division (a % m divider
    /// chain would dominate the probe and mask the load-latency regime).
    #[inline]
    pub fn fastrange(h: u64, m: u64) -> usize {
        (((h as u128) * (m as u128)) >> 64) as usize
    }

    /// Three probe positions from one mixed hash (the fuse-probe shape).
    #[inline]
    pub fn triple(key: u64, m: u64) -> [usize; 3] {
        let h = mix(key);
        [
            fastrange(h, m),
            fastrange(h.rotate_left(21), m),
            fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),
        ]
    }
}

mod buggy {
    use super::common::{triple, Query};

    pub struct Gate {
        pub checks: Vec<u64>,
        pub data: Vec<u64>,
    }

    impl Gate {
        pub fn probe(&self, q: &Query) -> Option<u64> {
            let [i0, i1, i2] = triple(q.key, self.checks.len() as u64);
            let fp = self.checks[i0] ^ self.checks[i1] ^ self.checks[i2];
            if fp != q.expect_fp {
                return None;
            }
            Some(self.data[i0] ^ self.data[i1] ^ self.data[i2])
        }
    }
}

mod fixed {
    use super::common::{triple, Query};

    pub type Slot = [u64; 2]; // [value, fingerprint] — 16 B

    pub struct Gate {
        pub slots: Vec<Slot>,
    }

    impl Gate {
        pub fn probe(&self, q: &Query) -> Option<u64> {
            let [i0, i1, i2] = triple(q.key, self.slots.len() as u64);
            let [value, fp] = xor3(&self.slots[i0], &self.slots[i1], &self.slots[i2]);
            if fp != q.expect_fp {
                return None;
            }
            Some(value)
        }
    }

    #[inline(always)]
    fn xor3(a: &Slot, b: &Slot, c: &Slot) -> Slot {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128, _mm_xor_si128, __m128i};
            // SAFETY: SSE2 is baseline on x86-64; each reference is a
            // live 16-byte slot, all an unaligned 128-bit load reads.
            unsafe {
                let v = _mm_loadu_si128(a.as_ptr().cast::<__m128i>());
                let v = _mm_xor_si128(v, _mm_loadu_si128(b.as_ptr().cast::<__m128i>()));
                let v = _mm_xor_si128(v, _mm_loadu_si128(c.as_ptr().cast::<__m128i>()));
                let mut out: Slot = [0; 2];
                _mm_storeu_si128(out.as_mut_ptr().cast::<__m128i>(), v);
                out
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            use std::arch::aarch64::{veorq_u64, vgetq_lane_u64, vld1q_u64};
            // SAFETY: NEON is baseline on AArch64; each vld1q_u64 reads
            // one live 16-byte slot.
            unsafe {
                let v = vld1q_u64(a.as_ptr());
                let v = veorq_u64(v, vld1q_u64(b.as_ptr()));
                let v = veorq_u64(v, vld1q_u64(c.as_ptr()));
                [vgetq_lane_u64::<0>(v), vgetq_lane_u64::<1>(v)]
            }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            [a[0] ^ b[0] ^ c[0], a[1] ^ b[1] ^ c[1]]
        }
    }
}

pub struct Input {
    split: buggy::Gate,
    paired: fixed::Gate,
    queries: Vec<common::Query>,
}

pub fn make_input(n: usize) -> Input {
    let mut state = 0x1234_5678_9ABC_DEF0u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };
    let checks: Vec<u64> = (0..n).map(|_| next()).collect();
    let data: Vec<u64> = (0..n).map(|_| next()).collect();
    let slots: Vec<fixed::Slot> = checks.iter().zip(&data).map(|(&c, &d)| [d, c]).collect();
    // Random query order (construction order lets the prefetcher hide
    // real work); the expected fingerprint XOR is RECORDED from the built
    // arrays — the fixtures measure the probe pattern, not the fuse solve.
    let mut queries = Vec::with_capacity(QUERIES);
    for _ in 0..QUERIES {
        let key = next();
        let [i0, i1, i2] = common::triple(key, n as u64);
        let expect_fp = checks[i0] ^ checks[i1] ^ checks[i2];
        queries.push(common::Query { key, expect_fp });
    }
    let split = buggy::Gate { checks, data };
    let paired = fixed::Gate { slots };
    // G1, live in every bench invocation: both layouts answer identically
    // on present keys, and a corrupted fingerprint is None in both (the
    // absent-key early exit).
    for q in queries.iter().take(1_000) {
        assert_eq!(
            split.probe(q),
            paired.probe(q),
            "G1: paired layout must answer identically to the split layout"
        );
    }
    let absent = common::Query { key: next(), expect_fp: u64::MAX };
    assert_eq!(split.probe(&absent), None, "G1: absent must be None (split)");
    assert_eq!(paired.probe(&absent), None, "G1: absent must be None (paired)");
    Input { split, paired, queries }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for q in &input.queries {
        acc = acc.wrapping_add(input.split.probe(q).unwrap_or(0));
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for q in &input.queries {
        acc = acc.wrapping_add(input.paired.probe(q).unwrap_or(0));
    }
    std::hint::black_box(acc);
}
"#
        .to_string(),
    }
}

/// Bench spec for `block-hash-before-gather-mlp`: per-key hash→gather
/// loop vs the two-phase block of 8 (resolve all indices, then gather).
/// Sizes are table positions (the regime axis — the win exists only past
/// last-level cache; the small size is the in-cache "roughly even" cell
/// the source itself reports). `4_096` keys per timed call, random order.
/// G1 bit-identity (blocked == loop, including the tail < 8) runs live
/// in `make_input`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)]
#[allow(clippy::too_many_lines)] // bench-spec harness assembler (the corpus convention)
fn block_hash_before_gather_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![262_144, 4_194_304, 16_777_216],
        harness: r#"
// Batched gathers over one flat table. BLOCK=8 is the source's own
// adjudicated block (8 wins or nearly wins everywhere; 16 splits the two
// machines it was measured on — annotated, never gated).
const BLOCK: usize = 8;
const KEYS_PER_CALL: usize = 4_096;

mod common {
    #[inline]
    pub fn mix(mut x: u64) -> u64 {
        x ^= x >> 30;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        x
    }

    #[inline]
    pub fn fastrange(h: u64, m: u64) -> usize {
        (((h as u128) * (m as u128)) >> 64) as usize
    }

    #[inline]
    pub fn triple(key: u64, m: u64) -> [usize; 3] {
        let h = mix(key);
        [
            fastrange(h, m),
            fastrange(h.rotate_left(21), m),
            fastrange(h ^ (h >> 32).wrapping_mul(0x9E37_79B9_7F4A_7C15), m),
        ]
    }
}

mod buggy {
    use super::common::triple;

    pub fn gather_all(table: &[u64], keys: &[u64], out: &mut [u64]) {
        for (o, &k) in out.iter_mut().zip(keys) {
            let [i0, i1, i2] = triple(k, table.len() as u64);
            *o = table[i0] ^ table[i1] ^ table[i2];
        }
    }
}

mod fixed {
    use super::common::triple;
    use super::BLOCK;

    pub fn gather_all(table: &[u64], keys: &[u64], out: &mut [u64]) {
        let m = table.len() as u64;
        let mut i0s = [0usize; BLOCK];
        let mut i1s = [0usize; BLOCK];
        let mut i2s = [0usize; BLOCK];
        let blocks = keys.len() / BLOCK * BLOCK;
        let mut b = 0;
        while b < blocks {
            // Phase 1: every address resolved before any table read.
            for j in 0..BLOCK {
                let [i0, i1, i2] = triple(keys[b + j], m);
                i0s[j] = i0;
                i1s[j] = i1;
                i2s[j] = i2;
            }
            // Phase 2: the block's gathers are independent — in flight.
            for j in 0..BLOCK {
                out[b + j] = table[i0s[j]] ^ table[i1s[j]] ^ table[i2s[j]];
            }
            b += BLOCK;
        }
        for (o, &k) in out[blocks..].iter_mut().zip(&keys[blocks..]) {
            let [i0, i1, i2] = triple(k, m);
            *o = table[i0] ^ table[i1] ^ table[i2];
        }
    }
}

pub struct Input {
    table: Vec<u64>,
    keys: Vec<u64>,
}

pub fn make_input(n: usize) -> Input {
    let mut state = 0x0DD5_0DD5_0DD5_0DD5u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };
    let table: Vec<u64> = (0..n).map(|_| next()).collect();
    // Random query order; the key buffer is built in query order (the
    // source's bench-honesty doctrine).
    let keys: Vec<u64> = (0..KEYS_PER_CALL).map(|_| next()).collect();
    // G1, live in every bench invocation: the block restructure is
    // bit-identical to the loop, including the tail (< BLOCK) path.
    {
        let mut a = vec![0u64; KEYS_PER_CALL];
        let mut b = vec![0u64; KEYS_PER_CALL];
        buggy::gather_all(&table, &keys, &mut a);
        fixed::gather_all(&table, &keys, &mut b);
        assert_eq!(a, b, "G1: blocked gathers must equal the loop bit-for-bit");
    }
    Input { table, keys }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut out = [0u64; KEYS_PER_CALL];
    buggy::gather_all(std::hint::black_box(&input.table), std::hint::black_box(&input.keys), &mut out);
    std::hint::black_box(out);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut out = [0u64; KEYS_PER_CALL];
    fixed::gather_all(std::hint::black_box(&input.table), std::hint::black_box(&input.keys), &mut out);
    std::hint::black_box(out);
}
"#
        .to_string(),
    }
}

/// Bench spec for `immutable-static-map-filter-selection`: built-once
/// read-only `String`→u64 table — std `HashMap` (`SipHash` + stored keys) vs
/// the construct-once binary-fuse static map (the library IS the fix;
/// `[dep: constmap]`, the papaya dep precedent). Sizes are key counts
/// (100k / 1M); `100_000` present-key lookups per timed call in random
/// order with the query buffer built in query order. G1 (static values
/// == `HashMap` values on present keys) runs live in `make_input`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::items_after_test_module)]
fn immutable_static_map_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![100_000, 1_000_000],
        harness: r#"
// [dep: constmap = "0.3"]
const QUERIES: usize = 100_000;

pub struct Input {
    map: std::collections::HashMap<String, u64>,
    static_map: constmap::ConstMap,
    queries: Vec<String>,
}

pub fn make_input(n: usize) -> Input {
    let mut state = 0x5EED_5EED_5EED_0001u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };
    let keys: Vec<String> = (0..n).map(|i| format!("entity:{i:012}")).collect();
    let values: Vec<u64> = (0..n).map(|_| next()).collect();
    let mut map = std::collections::HashMap::with_capacity(n);
    for (k, &v) in keys.iter().zip(&values) {
        map.insert(k.clone(), v);
    }
    let refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
    let static_map = constmap::ConstMap::new(&refs, &values).expect("fuse solve");
    // Random query order over the whole key set; the query buffer is
    // built in query order (the source's bench-honesty doctrine).
    let mut queries = Vec::with_capacity(QUERIES);
    for _ in 0..QUERIES {
        let idx = ((next() >> 16) as usize) % n;
        queries.push(keys[idx].clone());
    }
    // G1, live in every bench invocation: the static map's values must
    // equal the HashMap's on present keys.
    for q in queries.iter().take(1_000) {
        assert_eq!(
            map[q],
            static_map.map(q),
            "G1: static map must agree with the HashMap on present keys"
        );
    }
    Input { map, static_map, queries }
}

pub fn run_buggy(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for q in &input.queries {
        acc ^= input.map.get(q).copied().unwrap_or(0);
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
    let input = std::hint::black_box(input);
    let mut acc = 0u64;
    for q in &input.queries {
        acc ^= input.static_map.map(q);
    }
    std::hint::black_box(acc);
}
"#
        .to_string(),
    }
}

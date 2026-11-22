//! Batch 400 — the mvm (Magni VM) distill (Plan 400 / Research 435):
//! github.com/Bobliuuu/mvm @ tip `60c42deb5628480d62a50d53296e5f8be77e9030`
//! (main tip 2026-11-19T16:55Z, UNMOVED from the 11-21 verdict pin — the T0
//! re-clone landed HEAD == pin, every quote byte-exact at the same SHA).
//! Magni Community License 1.0 (source-available, free ≤$1M revenue /
//! research tiers) — **transcription-only GO, outbound-PR FAIL-CLOSED**
//! (per-company revenue-gated license; every row decision-content).
//!
//! The controlling verdict: the 11-21 staged-distill row
//! (`.distill/001_mining_queue_snapshot.md`, session zed-m3-mvm-distill) +
//! `../riir-infer/.research/36_*.md` (the verdict's home record). TWO
//! decision-content laws land here (the verdict's R1 lead + R2 after its
//! pre-registered `gpu_50` adjudication survived): the reader→publisher-of-N+1
//! snapshot-isolation edge (exact-N version waits), and park-off-worker
//! waiter custody with the dual in-flight budget. R3 lands as a
//! count-neutral FACE on `rust_perf` `entries_74` (`writeclass-share-partition-
//! inflight-writes`) in the same commit. R4/R5 honestly DROPPED at authoring
//! (thin rows — the research note carries the drops).
//!
//! Fixtures: `corpus/fixtures/b400_fixtures.rs` — standalone-compiled AND
//! executed BEFORE entry (debug AND -O), the four-arm discipline.
//! NOTHING INVENTED — every quote read at the clone this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 400 — the mvm (Magni VM) `kernel_opt` transcription.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the verdict's LEAD (B+): an `exact N` version wait orders
        // the READER before the NEXT publisher with a real edge, so snapshot
        // isolation is a compile-time fact, never a runtime race. ─────────
        KernelRule {
            rule_id: "version-snapshot-reader-before-next-publisher".to_string(),
            gpu_concept: "MECHANISM (versioned-weight publish/read coherence on a persistent-kernel task graph; category A — memory access/lifecycle, the version-coherence axis): a reader that must see an EXACT version N of mutable shared state (published weights) is not protected by acquire/release alone — a reader that sees N can still be overtaken by publisher N+1 whose stores land under it — so snapshot isolation is made a GRAPH FACT: the compiler adds an edge reader → publisher-of-N+1, making `unordered publishers` / a stale base / a wait-nothing-publishes COMPILE ERRORS instead of runtime races. The source's law block, verbatim (FRAMEWORK.md:84-89 @ 60c42deb): 'Publishers of one variable are ordered by program order (graph order, then task order) and get `base+1, base+2, ...`. Consecutive publishers must be connected by a dependency path, else `unordered publishers`.' + '**`exact N`** (snapshot isolation): the same wait, plus an edge reader -> publisher of N+1, so the next update cannot overwrite data while the reader runs. Rejected when `N < base_version` (stale), when N+1 is already ordered before the reader, or when N was never going to be published.' — three refusal classes, all at compile time: a stale N (below base), an N+1 already ordered before the reader (the edge would cycle), and a version nothing publishes (deadlock made static, verbatim :93-94: 'A wait for a version above `base` that nothing publishes, or that is published only after the waiter, is a compile error (deadlock). Version 0 is never waited on.'). THE MEMORY-MODEL HONESTY IS THE PREMISE, verbatim (include/mvm_exec.cuh:27-32): 'Reader R (wait_var=v, wait_ver<=N): the version is read with ld.acquire.gpu ... Reading N synchronizes-with P's release, so P's weight stores happen-before every thread of R's block ... Hence a reader that sees version N sees all of version N's weights (and anything older). Whether it can also see NEWER stores is the graph's job: an \"exact\" reader must be ordered before the next writer by a version variable (reader publishes a \"done\" variable the writer waits on).' — acquire/release buys HAPPENS-BEFORE for version N's own stores; it does NOT buy EXCLUSIVITY against N+1; the edge is what buys exclusivity, and the compiler OWNS the edge so no author can forget it. THE MECHANISM DETAIL: the wait is ONE slot per descriptor — extra waiting reads become materialized `noop` gate tasks that carry the wait and precede the reader (FRAMEWORK.md:90-92: 'One wait slot, one publish slot per descriptor. Extra waiting reads become `noop` tasks `__gate/<graph>/<task>/<k>` (same priority/deadline) that carry the wait and precede the reader; the reader's own slot carries the first. A task that reads and publishes needs no gate.'); the host-side validators are the classification (runtime/compile_workload.py:475 'is stale: version N+1 is already published or admitted (base_version=...)' + :610 'unordered publishers of ...: no dependency path from ...'). COHERENCE IS MEASURED AS A CENSUS, not asserted (results/2026-11-06-demo-tiny/README.md:20: 'version coherence | PASS: 184 version-carrying tasks, 184 exact, 0 violations'; the large trace 2026-11-06-demo-large-trace/README.md:17: 'PASS: 22,935 version-carrying tasks, all exact, 0 violations') — per-reader observed-version census at start and finish, the VM 'checks `exact` reads at start and finish and fails the task on a violation' (FRAMEWORK.md:103-104). THE PARK/PUBLISH LOST-WAKEUP HALF (the executor side of the same coherence claim) is closed with a Dekker pattern, verbatim (include/mvm_exec.cuh:35-36): 'Park/publish race (lost wakeup) is closed with a Dekker pattern: waiter = push; fence.sc; load version. publisher = store version; fence.sc; exchange list. At least one side observes the other; the state CAS makes the release unique.' — and the claim is adversarially litigated (tests/litmus/ 'a litmus that cannot fail proves nothing, so every property has at least one mutant that must be caught'; REVIEW.md controls table: 'SB without fence (19,222), publish-before-stores (2.9M), reader without acquire (55,737), relaxed CAS (244), waiter without recheck (23,020), publisher exchange-first (11,800)' — every mutant detected). ⛔ KINSHIP (adjudicated both directions, the verdict's claim-time diff list): page-reuse-fenced-behind-reader-completion (gpu_138) owns the PAGE-REUSE LIFECYCLE fence — a released page's new WRITER gated behind the in-flight reader's step completion at RUNTIME by a fence sequence; THIS owns the VERSION-COHERENCE edge — the next PUBLISER of a versioned variable ordered behind its exact reader at COMPILE time by a graph edge; both are reader-protection, but the axis is reuse-fencing (storage lifecycle) vs snapshot isolation (version exclusivity) — compose on a paged serving lane (the pages are the buffers, the versions order the publishes); ring-overwrite-gated-k-back-readiness (gpu_61) owns DEVICE-ring distance-K overwrite gating via readiness counters — the same reader-protection shape on a ring; bounded-cross-worker-barrier-protocol-polling-fan-in-ladder (gpu_85) owns barrier-protocol COST pricing (the in-kernel join vs the dispatch boundary) — no version axis; closed-channel-select-arm-gate (rust_perf entries_86) owns the ungated-arm SPIN liveness class — THIS makes the deadlock static at compile time instead of gating an arm; the epoch-tag echo (gpu_148's never-zero per-writer epoch) is cross-domain freshness vocabulary, recorded never landed. REGIME: persistent-kernel / task-graph executors publishing mutable weights or shared state through version variables with long-lived readers (a serving request reads weights a trainer is updating beside it); any system where 'read the exact published version' must hold against concurrent re-publish. ANTI-REGIME: `at_least N` reads (a newer version is fine — no edge, verbatim :86 'No edge is added'); readers whose result tolerates the next version (sampling staleness); single-version state with no second publisher (nothing to order against). NotBenchable here (a coherence/ordering axis — the fixture pins the edge arithmetic and the three refusal classes; no persistent-kernel executor bench in this corpus). The fixture models the graph-side law deterministically: the edge insertion for an exact reader, the three refusals (stale, already-ordered, nothing-publishes), the gate-task materialization for a second wait slot, and the runtime witness (a reader that would have been overtaken without the edge reads its exact version with it). Source: Bobliuuu/mvm @ 60c42deb5628480d62a50d53296e5f8be77e9030 (main tip 2026-11-19T16:55Z, the 11-21 verdict pin UNMOVED; Magni Community License 1.0 — transcription-only, outbound-PR fail-closed) — FRAMEWORK.md:78-97 (the version diagram + the publisher ordering + the exact-N edge + the refusal classes + the gate-slot law), :99-104 (the CPU reference + the exact-read checks), include/mvm_exec.cuh:23-36 (the memory-ordering argument + the Dekker closure), runtime/compile_workload.py:460-615 (the stale + unordered-publisher validators), results/2026-11-06-demo-tiny/README.md:20 + results/2026-11-06-demo-large-trace/README.md:17 (the coherence censuses), tests/litmus/litmus.cu:1-8 (the mutant discipline), REVIEW.md 'What I could NOT break' (the 6.0M-round park/publish zero + the mutant detection counts). Batch 400 / Plan 400 / Research 435."
                .to_string(),
            ref_shape: r#"// ref_shape (rule: version-snapshot-reader-before-next-publisher):
// the reader waits on the version variable, but NOTHING orders the next
// publisher behind it -- acquire/release makes version N's own stores
// visible, yet publisher N+1 can overwrite the weights while the exact
// reader is still running. The wait is also just a soft check: a version
// nothing publishes (or publishes only after the waiter) fails at
// runtime as a TIMEOUT, never at compile time.

#[derive(Clone, Copy, PartialEq)]
struct Weights {
    w: [u32; 4],
}

#[derive(Default)]
struct Runtime {
    version: u32,
    weights: Weights,
}

impl Runtime {
    // publisher: bump the version and store weights (release)
    fn publish(&mut self, n: u32, w: Weights) {
        self.version = n;
        self.weights = w;
    }

    // exact reader: acquire the version, then read -- but the next
    // publisher is free to run CONCURRENTLY (no edge exists), so the
    // read can observe N+1's stores under it.
    fn read_exact(&self, n: u32) -> Option<Weights> {
        if self.version == n {
            Some(self.weights) // NOT exclusive: N+1 may overwrite mid-read
        } else {
            None
        }
    }
}

// runtime deadlock class: a wait for a version nothing publishes is
// only discovered when the executor TIMES OUT.
fn wait_forever_if_unpublished(rt: &Runtime, n: u32) -> Option<Weights> {
    loop {
        if let Some(w) = rt.read_exact(n) {
            return Some(w);
        }
        // no compile-time knowledge that n is never published...
    }
}

let mut rt = Runtime::default();
rt.publish(1, Weights { w: [11, 22, 33, 44] });
let reader = rt.read_exact(1);
rt.publish(2, Weights { w: [99, 99, 99, 99] }); // unordered: runs over the reader
assert!(reader.is_some(), "the reader started on N");
assert_eq!(rt.weights.w, [99, 99, 99, 99], "N+1 overwrote mid-read");
"#.to_string(),
            opt_pattern: r#"// opt_pattern (rule: version-snapshot-reader-before-next-publisher):
// the compiler owns snapshot isolation: an `exact N` read inserts the
// edge reader -> publisher-of-N+1, publishers must be dependency-
// ordered (`unordered publishers` is a compile error), stale waits and
// nothing-publishes waits are compile errors, and a second waiting read
// is materialized as a gate task that precedes the reader. Convention:
// a LOWER task id = EARLIER program order (graph order, then task order).

#[derive(Default)]
struct Graph {
    base_version: u32,
    publishers: Vec<(u32, u32)>, // (task_id, version) ascending in program order
    edges: Vec<(u32, u32)>,
}

impl Graph {
    fn admit_publisher(&mut self, task: u32, version: u32) -> Result<(), &'static str> {
        // versions are base+1, base+2, ... in program order
        let expected = self
            .publishers
            .last()
            .map_or(self.base_version + 1, |&(_, v)| v + 1);
        if version != expected {
            return Err("unordered publishers: versions move out of program order");
        }
        if let Some(&(prev, _)) = self.publishers.last() {
            let connected = self.edges.iter().any(|(a, b)| {
                (*a == prev && *b == task) || (*a == task && *b == prev)
            });
            if !connected {
                return Err("unordered publishers: no dependency path");
            }
        }
        self.publishers.push((task, version));
        Ok(())
    }

    fn admit_exact_reader(&mut self, reader: u32, n: u32) -> Result<(), &'static str> {
        if n < self.base_version {
            return Err("stale: N < base_version");
        }
        // the wait itself must be publishable BEFORE the waiter
        match self.publishers.iter().find(|&&(_, v)| v == n) {
            Some(&(t, _)) if t < reader => {}
            Some(_) => return Err("N is published only after the waiter"),
            None => return Err("deadlock: N was never going to be published"),
        }
        // THE EDGE: reader -> publisher of N+1 (the next update cannot
        // overwrite data while the reader runs); no future N+1 publisher
        // means nothing can overwrite — no edge needed.
        match self.publishers.iter().find(|&&(_, v)| v == n + 1) {
            Some(&(ptask, _)) if ptask < reader => {
                Err("N+1 is already ordered before the reader")
            }
            Some(&(ptask, _)) => {
                self.edges.push((reader, ptask));
                Ok(())
            }
            None => Ok(()),
        }
    }
}

// runtime witness: the exact read sets a HELD done-flag — the edge's
// runtime form — and publish refuses while it is up.
struct Executor {
    version: u32,
    weights: Weights,
    reader_holding: bool,
}

impl Executor {
    fn read_exact(&mut self, n: u32) -> Option<Weights> {
        if self.version == n {
            self.reader_holding = true;
            Some(self.weights)
        } else {
            None
        }
    }
    fn release_reader(&mut self) {
        self.reader_holding = false; // done -> releases publisher N+1
    }
    fn publish(&mut self, n: u32, w: Weights) -> Result<(), &'static str> {
        if self.reader_holding {
            return Err("blocked by the exact reader's edge");
        }
        self.version = n;
        self.weights = w;
        Ok(())
    }
}

let mut g = Graph { base_version: 0, publishers: vec![], edges: vec![] };
g.admit_publisher(1, 1).unwrap();
g.edges.push((1, 3));
g.admit_publisher(3, 2).unwrap();
assert!(g.admit_exact_reader(2, 1).is_ok(), "the edge is inserted");
assert!(g.edges.contains(&(2, 3)), "reader -> publisher-of-N+1");
let mut g2 = Graph { base_version: 1, publishers: vec![(5, 2)], edges: vec![] };
assert_eq!(
    g2.admit_exact_reader(2, 0),
    Err("stale: N < base_version"),
    "stale refused at compile time"
);
let mut ex = Executor { version: 1, weights: Weights { w: [11, 22, 33, 44] }, reader_holding: false };
let r = ex.read_exact(1);
assert_eq!(ex.publish(2, Weights { w: [99, 99, 99, 99] }), Err("blocked by the exact reader's edge"));
ex.release_reader();
assert!(ex.publish(2, Weights { w: [99, 99, 99, 99] }).is_ok(), "N+1 publishes after done");
assert_eq!(r.map(|w| w.w), Some([11, 22, 33, 44]), "the exact read is exclusive");
"#.to_string(),
        },
        // ── R2 — the verdict's R2 (B−, LANDED after the gpu_50 adjudication
        // survived): version-waiters park OFF the runnable queue — no
        // worker block holds them, no queue cell is spent on them — under
        // a SECOND budget so a publisher is never blocked behind waiters.
        // ─────────────────────────────────────────────────────────────────
        KernelRule {
            rule_id: "parked-waiters-release-off-worker".to_string(),
            gpu_concept: "MECHANISM (waiter custody in a persistent-kernel worker pool; category E — parallelism, the admission/custody axis): a task whose dependency (a version variable) is unsatisfied must not occupy a WORKER BLOCK or a READY-QUEUE CELL while it waits — it parks on the dependency variable's lock-free list and the PUBLISHER releases it — so no worker ever holds two tasks and a publisher is never blocked behind the waiters of its own dependency. The source's invariant block, verbatim (include/mvm_exec.cuh:5-8 @ 60c42deb): 'A worker block holds at most ONE task at a time and never waits on anything but the ready queue. A task whose wait_var/wait_ver is unsatisfied is never in the queue: it is PARKED on its variable's lock-free list and released by the publisher (or by its own parker's re-check). So no block can hold two tasks, and a publisher is never blocked behind a waiter: waiters occupy no block and no queue cell.' — the custody claim is what makes the rest of the executor sane (exactly-once enqueue: 'Every task is enqueued exactly once (last indegree decrementer, the admitter for roots, or the single winner of the park CAS) and popped exactly once (head CAS)'). THE SECOND BUDGET IS THE LIVENESS HALF, verbatim (:11-16): 'The ready queue has MVM_NBUCKETS FIFO rings per class (Vyukov bounded MPMC cells), each rcap = 4 * live_cap deep. Two budgets keep every ring from filling: admission waits while (admitted - done - parked + n) > live_cap (the runnable window) or parked > live_cap (version-waiting tasks have their own budget, so a bundle that publishes a version is never blocked behind waiters of an earlier bundle), hence at most 2 * live_cap tasks are ever in flight ... More than live_cap parked at once is the explicit error PARK_STALL.' — ONE budget for runnable in-flight work, a SECOND for parked waiters; without the split, a wave of waiters fills the runnable budget and the bundle that would PUBLISH their version cannot be admitted (the deadlock the review class C2 confirmed pre-fix: 'All of bundle 1 parks, admitted - done == live_cap, bundle 2 never fits'; the post-fix header carries the parked counter — atom_add(&S.cnt->parked, ±1) at park/unpark — and PARK_STALL is the fail-closed over-budget refusal, never a silent hang). THE RELEASE PATH IS THE PUBLISHER'S JOB, verbatim (:527-543 mvm_scan_parked): 'int n = atom_exch(S.park_head + var, -1); while (n >= 0) { int next = S.park_next[n]; ... if (ld_acq(S.version + var) >= need) { if (atom_cas(S.state + n, 1, 2) != 1) mvm_fail(S, MVM_ERR_DOUBLE_EXEC, ...); else { atom_add(&S.cnt->parked, -1); ...; mvm_enqueue(S, n); } }' — detach-all on the variable's head, release exactly the waiters whose version is met, re-park the rest ('A scan re-parks nodes whose version is still too low and re-checks them the same way'); the slot-ownership law binds the recycling: 'EVERY node on a list is owned by whoever detaches it, and a node is removed from the list only by a detach, so a slot is never reachable from a list after its task was enqueued'. THE WAKEUP IS DEKKER-CLOSED AND LITIGATED (the R1 rule's memory-ordering half composes): waiter = push; fence.sc; load version; publisher = store version; fence.sc; detach — 'At least one side observes the other; the state CAS makes the release unique' (:520-522, :35-36); the positive control is the mutant harness, verbatim (:551-553): '#ifndef MVM_BREAK_PARK_RECHECK // negative control for the litmus test: a waiter that never re-checks loses wakeups ... __threadfence(); // seq_cst fence: push -> fence -> load version ... if (ld_acq(S.version + var) >= need) mvm_scan_parked(S, var);' — REVIEW.md measured the mutant DETECTED: 'MVM_BREAK_PARK_RECHECK mutant: 12,834 lost wakeups / 0.3 M (detected)' and the clean property at zero: 'No lost wakeup (park vs publish), single waiter | 6.0 M rounds, random skew, 864 CTAs | 0'. ⛔ KINSHIP (adjudicated both directions, the verdict's pre-registered decision): split-submission-in-flight-cap-overlap (gpu_50) owns the IN-FLIGHT CAP + OVERLAP axis — how much work may be submitted so streams overlap; THIS owns WHERE WAITERS WAIT while under such caps (off-worker custody + the second budget) — compose on a persistent-kernel serving lane (the cap shapes submission, the park law shapes the wait); writeclass-share-partition-inflight-writes (rust_perf entries_74) owns the CLASS-SHARE partition of ONE budget (urgency classes, reserved minimums) — THIS owns the RUNNABLE-VS-PARKED split of the SAME live window (a different partition axis; compose: the admission gate partitions by class, the executor partitions by custody state); singleflight-identical-work-per-content-key (rust_perf entries_83) owns one-elected-leader dedup — no waiting-custody axis; bounded-cross-worker-barrier-protocol-polling-fan-in-ladder (gpu_85) prices barrier protocols and carries the straggler face ('a lost wakeup under replay spins FOREVER') — the liveness HALF of this law's park/re-check discipline, cost axis vs custody axis; page-reuse-fenced-behind-reader-completion (gpu_138) composes via R1. REGIME: persistent-kernel / agent-pool executors with version or event dependencies — waiters whose dependency may be satisfied by work that is itself queued (a publisher behind them in admission order). ANTI-REGIME: spin-waiting inside the worker (burns the block the queue needs); unbounded parked sets without the second budget (the PARK_STALL class); dependencies always satisfied at enqueue (park machinery is pure overhead — take the fast path: 'if (var < 0 || ld_acq(S.version + var) >= need) { mvm_enqueue(S, slot); return; }'). NotBenchable here (a custody/liveness axis — the fixture pins the dual-budget admission arithmetic, the park/release/re-park state machine, and the mutant-detected negative control; no worker-pool executor bench in this corpus). Source: Bobliuuu/mvm @ 60c42deb5628480d62a50d53296e5f8be77e9030 (main tip 2026-11-19T16:55Z, the 11-21 verdict pin UNMOVED; Magni Community License 1.0 — transcription-only, outbound-PR fail-closed) — include/mvm_exec.cuh:5-21 (the roles/invariants header: custody + dual budget + PARK_STALL), :514-555 (mvm_scan_parked + mvm_release_task + the MVM_BREAK_PARK_RECHECK negative control), REVIEW.md C2 (the pre-fix pinned-window deadlock + the real-fix design line), 'What I could NOT break' (the 6.0M-round zero + the 23,020 waiter-without-recheck mutant), results/2026-11-04-a100-sched-integrated/README.md:24 (C2's live 9/9 probe after the parked-budget fix). Batch 400 / Plan 400 / Research 435."
                .to_string(),
            ref_shape: r#"// ref_shape (rule: parked-waiters-release-off-worker): the waiter
// OCCUPIES a worker slot and a ready-queue cell while its version is
// unsatisfied, and there is ONE in-flight budget -- so a wave of
// waiters fills it, the publishing bundle cannot be admitted, and the
// whole pool wedges until a timeout. Spinning in the worker burns the
// block the queue needs.

const LIVE_CAP: usize = 8;

#[derive(Default)]
struct Pool {
    in_flight: Vec<(u32, u32, bool)>, // (task, waits_on, done)
    ready: Vec<u32>,
}

impl Pool {
    // ONE budget for everything: a waiter occupies the same budget a
    // publisher needs. Bundle 2 (the publisher) cannot be admitted
    // while bundle 1's waiters fill the cap.
    fn can_admit(&self, n_new: usize) -> bool {
        self.in_flight.len() + n_new <= LIVE_CAP
    }

    // the worker spins on the version instead of yielding the block
    fn worker_spin(&self, task: u32, version: u32) -> Option<u32> {
        loop {
            if version >= 1 {
                return Some(task);
            }
            // ...burns the worker; the publisher can never run here
            // because it was never admitted (cap full of waiters).
        }
    }
}

let mut pool = Pool::default();
// bundle 1: live_cap waiters on v0 >= 1
for t in 0..LIVE_CAP {
    pool.in_flight.push((t as u32, 0, false));
}
assert!(!pool.can_admit(1), "the publisher bundle never fits: wedged");
"#.to_string(),
            opt_pattern: r#"// opt_pattern (rule: parked-waiters-release-off-worker): waiters park
// OFF the worker and the queue -- on the dependency variable's lock-free
// list -- released by the publisher's detach-all; TWO budgets bound the
// live window (runnable) and the parked set (its own), over-budget
// parking is the explicit PARK_STALL error, and a waiter whose version
// is already met never parks. Admission's window arithmetic EXCLUDES
// parked (admitted - done - parked + n <= live_cap), so a publishing
// bundle is admissible behind a full parked set.

const LIVE_CAP: usize = 8;
const PARK_BUDGET: usize = 8; // the second budget -- waiters never crowd the window

#[derive(Default)]
struct Pool {
    runnable: usize,                   // admitted - done, runnable window
    parked: usize,                     // version-waiting tasks, own budget
    park_lists: Vec<Vec<(u32, u32)>>,  // per variable: parked (task, need)
    version: Vec<u32>,                 // per variable: current version
}

#[derive(Debug, PartialEq)]
enum Admit {
    Admitted,
    Parked,
    ParkStall, // the explicit error, never a silent hang
}

impl Pool {
    /// admission of a NEW bundle: the runnable window only
    fn can_admit(&self, n_new: usize) -> bool {
        self.runnable + n_new <= LIVE_CAP
    }

    // release_task: enqueue now if the version wait is met, else park
    fn release_task(&mut self, task: u32, var: usize, need: u32) -> Admit {
        if self.version[var] >= need {
            self.runnable += 1; // fast path: never parks
            return Admit::Admitted;
        }
        if self.parked + 1 > PARK_BUDGET {
            return Admit::ParkStall;
        }
        self.parked += 1;
        self.park_lists[var].push((task, need));
        Admit::Parked
    }

    // the publisher's scan: detach-all, release the met, re-park the rest
    fn publish(&mut self, var: usize) -> Vec<u32> {
        self.version[var] += 1;
        let list = std::mem::take(&mut self.park_lists[var]);
        let v = self.version[var];
        let mut released = Vec::new();
        let mut repark = Vec::new();
        for (t, need) in list {
            self.parked -= 1;
            if v >= need {
                self.runnable += 1;
                released.push(t);
            } else {
                self.parked += 1;
                repark.push((t, need));
            }
        }
        self.park_lists[var] = repark;
        released
    }
}

let mut pool = Pool { park_lists: vec![vec![]], version: vec![0], ..Pool::default() };
// bundle 1's waiters park -- OFF the runnable window's budget
for t in 0..LIVE_CAP {
    assert_eq!(pool.release_task(t as u32, 0, 1), Admit::Parked);
}
assert_eq!(pool.runnable, 0, "waiters occupy no runnable slots");
assert_eq!(pool.parked, LIVE_CAP);
assert_eq!(pool.release_task(99, 0, 1), Admit::ParkStall, "over-budget is explicit");
// the publisher bundle admits (the runnable window excludes parked) and releases them
assert!(pool.can_admit(1), "the publisher is never blocked behind waiters");
assert_eq!(pool.publish(0).len(), LIVE_CAP, "the scan releases every met waiter");
assert_eq!(pool.parked, 0);
"#.to_string(),
        },
    ]
}

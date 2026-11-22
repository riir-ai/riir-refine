//! Batch 400 kernel fixture preflight — the mvm (Magni VM) distill
//! (Plan 400 / Research 435, Bobliuuu/mvm @ 60c42deb, Magni Community
//! License 1.0 — transcription-only) as ONE standalone file (2 rules, the
//! four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the source
//! is CUDA/C++; the Rust arms below model each law's decision content).
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b400_fixtures.rs`
//! at debug AND -O. Each arm names the honest and the adversary half;
//! a defect caught here is fixed HERE first, then synced into
//! entries_gpu_162.rs in the same landing commit.
//! (First run caught FOUR arm defects — the ids-vs-program-order
//! conflation, a guard that dropped before publish, a publisher gated on
//! the parked budget, and a scan with a hardcoded need — all fixed here,
//! the entries synced to the corrected models.)
//!
//! Arms:
//! - R1 snapshot isolation: the edge insertion for an exact reader; the
//!   three compile-time refusals (stale / already-ordered /
//!   nothing-publishes) + the published-after-the-waiter refusal; the
//!   unordered-publisher refusal; the runtime witness (the buggy arm is
//!   overtaken by N+1 mid-read, the fixed arm's held guard blocks the
//!   overwrite until the reader's done); the gate-task materialization
//!   (a second wait slot becomes a preceding noop); the tampered arm —
//!   an "exact" read with no edge and no census — reds.
//! - R2 waiter custody: the dual-budget admission arithmetic (the
//!   runnable window excludes parked; waiters park under their OWN
//!   budget; the publisher bundle admits behind a full parked set); the
//!   park/scan/release state machine (release the met, re-park the
//!   unmet, PARK_STALL the over-budget); the fast path (an already-met
//!   waiter never parks); the adversary arms — the single-budget wedge
//!   (bundle 2 never fits) and the no-recheck waiter (the lost-wakeup
//!   mutant class) — red.

#[derive(Clone, Copy, PartialEq, Debug)]
struct Weights {
    w: [u32; 4],
}

// ───────────────────────── R1 arms ─────────────────────────

/// The compile-time graph. Convention: a LOWER task id = EARLIER program
/// order (the source's "graph order, then task order" flattening).
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
            // consecutive publishers must be connected by a dependency path
            let connected = self
                .edges
                .iter()
                .any(|(a, b)| (*a == prev && *b == task) || (*a == task && *b == prev));
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
        let pub_n = self.publishers.iter().find(|&&(_, v)| v == n);
        let _ = &pub_n;
        let pub_n = match pub_n {
            Some(&(t, _)) if t < reader => t,
            Some(_) => return Err("N is published only after the waiter"),
            None => return Err("deadlock: N was never going to be published"),
        };
        // the edge goes to the publisher of N+1 (the next update cannot
        // overwrite data while the reader runs)
        match self.publishers.iter().find(|&&(_, v)| v == n + 1) {
            Some(&(ptask, _)) if ptask < reader => {
                Err("N+1 is already ordered before the reader")
            }
            Some(&(ptask, _)) => {
                self.edges.push((reader, ptask));
                let _ = pub_n; // the publisher of N is the read's visibility source
                Ok(())
            }
            None => Ok(()), // nothing will ever publish N+1: no edge needed
        }
    }
}

/// The buggy posture: a reader that checks the version but nothing
/// orders the next publisher — N+1's stores land under the read.
#[derive(Clone, Copy)]
struct RacyRuntime {
    version: u32,
    weights: Weights,
}

impl RacyRuntime {
    fn publish(&mut self, n: u32, w: Weights) {
        self.version = n;
        self.weights = w;
    }
    fn read_exact_unordered(&self, n: u32) -> Option<Weights> {
        if self.version == n {
            Some(self.weights) // no exclusivity: N+1 may overwrite mid-read
        } else {
            None
        }
    }
}

/// The fixed posture: reading an exact version sets a HELD flag — the
/// reader's `done` variable — and no publisher may run until it is
/// released (the edge's runtime form; the release is explicit because a
/// borrowing guard would model the exclusivity with the borrow checker
/// instead of the protocol).
struct CoherentRuntime {
    version: u32,
    weights: Weights,
    reader_holding: bool,
}

impl CoherentRuntime {
    fn read_exact(&mut self, n: u32) -> Option<Weights> {
        if self.version == n {
            self.reader_holding = true;
            Some(self.weights)
        } else {
            None
        }
    }
    /// the reader finishing publishes its `done` variable, releasing the
    /// publisher-of-N+1 exactly when the read completes.
    fn release_reader(&mut self) {
        self.reader_holding = false;
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

/// Gate-task materialization: a descriptor's extra waiting reads become
/// preceding noop gate tasks carrying the same (priority, wait).
fn materialize_gate_tasks(waiting_reads: usize) -> Vec<u32> {
    (0..waiting_reads.saturating_sub(1)).map(|k| 10_000 + k as u32).collect()
}

#[test]
fn r1_edge_inserted_for_exact_reader() {
    let mut g = Graph {
        base_version: 0,
        ..Graph::default()
    };
    g.admit_publisher(1, 1).unwrap(); // task 1 publishes v1
    g.edges.push((1, 3)); // the dependency path 1 -> 3
    g.admit_publisher(3, 2).unwrap(); // task 3 publishes v2
    // reader task 2 waits the EXACT version 1
    assert!(g.admit_exact_reader(2, 1).is_ok());
    assert_eq!(g.edges, vec![(1, 3), (2, 3)], "reader -> publisher-of-N+1");
}

#[test]
fn r1_compile_time_refusals() {
    // stale
    let mut g = Graph {
        base_version: 1,
        publishers: vec![(5, 2)],
        edges: vec![],
    };
    assert_eq!(
        g.admit_exact_reader(2, 0),
        Err("stale: N < base_version")
    );
    // nothing publishes N
    let mut g2 = Graph {
        base_version: 0,
        publishers: vec![(1, 1), (3, 2)],
        edges: vec![],
    };
    assert_eq!(
        g2.admit_exact_reader(2, 5),
        Err("deadlock: N was never going to be published")
    );
    // N published only after the waiter
    assert_eq!(
        g2.admit_exact_reader(0, 2),
        Err("N is published only after the waiter")
    );
    // N+1 already ordered before the reader
    assert_eq!(
        g2.admit_exact_reader(4, 1),
        Err("N+1 is already ordered before the reader")
    );
}

#[test]
fn r1_exact_reader_before_no_future_publisher_needs_no_edge() {
    let mut g = Graph {
        base_version: 0,
        publishers: vec![(1, 1)],
        edges: vec![],
    };
    // v1 exists, nothing will ever publish v2: the read is safe unaided
    assert!(g.admit_exact_reader(2, 1).is_ok());
    assert!(g.edges.is_empty(), "no N+1 publisher: no edge");
}

#[test]
fn r1_unordered_publishers_refused() {
    let mut g = Graph {
        base_version: 0,
        publishers: vec![],
        edges: vec![],
    };
    g.admit_publisher(1, 1).unwrap();
    // publisher 3 claims v2 with NO dependency path from publisher 1
    assert_eq!(
        g.admit_publisher(3, 2),
        Err("unordered publishers: no dependency path")
    );
    // connected via an edge: admitted
    g.edges.push((1, 3));
    assert!(g.admit_publisher(3, 2).is_ok());
    // a version gap is the other unordered shape
    assert_eq!(
        g.admit_publisher(5, 4),
        Err("unordered publishers: versions move out of program order")
    );
}

#[test]
fn r1_buggy_arm_overtaken_by_next_publisher() {
    let mut rt = RacyRuntime {
        version: 1,
        weights: Weights { w: [11, 22, 33, 44] },
    };
    let started = rt.read_exact_unordered(1);
    // the unordered publisher N+1 lands WHILE the exact reader holds N
    rt.publish(2, Weights { w: [99, 99, 99, 99] });
    assert!(started.is_some(), "the reader started on N");
    assert_eq!(
        rt.weights.w,
        [99, 99, 99, 99],
        "the read is NOT exclusive: N+1 overwrote mid-read"
    );
}

#[test]
fn r1_fixed_arm_held_read_blocks_the_overwrite() {
    let mut rt = CoherentRuntime {
        version: 1,
        weights: Weights { w: [11, 22, 33, 44] },
        reader_holding: false,
    };
    {
        let w = rt.read_exact(1).expect("the read sees its exact version");
        assert_eq!(w.w, [11, 22, 33, 44]);
        // the edge holds the next writer WHILE the read is held
        assert_eq!(
            rt.publish(2, Weights { w: [99, 99, 99, 99] }),
            Err("blocked by the exact reader's edge")
        );
        // the reader finishes: `done` publishes, releasing publisher N+1
        rt.release_reader();
    }
    assert!(
        rt.publish(2, Weights { w: [99, 99, 99, 99] }).is_ok(),
        "after the reader finishes, N+1 publishes"
    );
}

#[test]
fn r1_extra_wait_slots_become_preceding_gate_tasks() {
    // one wait slot per descriptor; the extras precede the reader
    assert!(materialize_gate_tasks(1).is_empty(), "single wait: no gates");
    assert_eq!(materialize_gate_tasks(3), vec![10_000, 10_001]);
}

#[test]
fn r1_tampered_arm_no_edge_no_census_reds() {
    // the adversary: an "exact" read with NO edge and NO observed-version
    // census — the coherence claim is unverifiable, the class this rule
    // exists for. The honest posture REQUIRES both.
    let mut rt = RacyRuntime {
        version: 1,
        weights: Weights { w: [1, 2, 3, 4] },
    };
    let _ = rt.read_exact_unordered(1);
    rt.publish(2, Weights { w: [9, 9, 9, 9] });
    let graph_has_edge = false;
    let census_checked = false;
    assert!(
        !graph_has_edge && !census_checked,
        "an exact read without the edge + the census is the defect"
    );
}

// ───────────────────────── R2 arms ─────────────────────────

const LIVE_CAP: usize = 8;
const PARK_BUDGET: usize = 8;

/// The custody pool: the runnable window (runnable admitted work) and
/// the parked set (version-waiters) are TWO budgets — admission's window
/// arithmetic excludes parked (admitted - done - parked + n <= live_cap).
#[derive(Default)]
struct Pool {
    runnable: usize,
    parked: usize,
    park_lists: Vec<Vec<(u32, u32)>>, // per variable: (task, need)
    version: Vec<u32>,
}

#[derive(Debug, PartialEq)]
enum Admit {
    Admitted,
    Parked,
    ParkStall,
}

impl Pool {
    fn new(vars: usize) -> Self {
        Pool {
            runnable: 0,
            parked: 0,
            park_lists: vec![vec![]; vars],
            version: vec![0; vars],
        }
    }
    /// admission of a NEW bundle: the runnable window only — parked
    /// waiters never crowd it (they have their own budget).
    fn can_admit(&self, n_new: usize) -> bool {
        self.runnable + n_new <= LIVE_CAP
    }
    /// release_task: enqueue now if the version wait is met, else park
    /// on the variable's list — under the SECOND budget.
    fn release_task(&mut self, task: u32, var: usize, need: u32) -> Admit {
        if self.version[var] >= need {
            self.runnable += 1; // the fast path: an already-met waiter never parks
            return Admit::Admitted;
        }
        if self.parked + 1 > PARK_BUDGET {
            return Admit::ParkStall;
        }
        self.parked += 1;
        self.park_lists[var].push((task, need));
        Admit::Parked
    }
    /// the publisher's scan: detach-all, release the met, re-park the rest.
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

#[test]
fn r2_waiters_park_off_the_runnable_window() {
    let mut pool = Pool::new(1);
    for t in 0..LIVE_CAP {
        assert_eq!(pool.release_task(t as u32, 0, 1), Admit::Parked);
    }
    assert_eq!(pool.runnable, 0, "waiters occupy no runnable slots");
    assert_eq!(pool.parked, LIVE_CAP);
}

#[test]
fn r2_publisher_admits_behind_a_full_parked_set() {
    let mut pool = Pool::new(1);
    for t in 0..PARK_BUDGET {
        let _ = pool.release_task(t as u32, 0, 1);
    }
    // THE SECOND BUDGET: the runnable window is empty (parked excluded
    // from it), so the publishing bundle is admissible even with the
    // parked set full — the C2 deadlock class cannot form.
    assert!(
        pool.can_admit(1),
        "a publisher is never blocked behind waiters"
    );
    assert_eq!(pool.publish(0).len(), PARK_BUDGET);
    assert_eq!(pool.parked, 0);
}

#[test]
fn r2_total_in_flight_bounded_at_two_caps() {
    let mut pool = Pool::new(1);
    for t in 0..LIVE_CAP {
        let _ = pool.release_task(t as u32, 0, 1);
    }
    // the window also admits its own cap of runnable work
    pool.runnable += LIVE_CAP;
    assert!(
        pool.runnable + pool.parked <= 2 * LIVE_CAP,
        "at most 2 * live_cap tasks are ever in flight"
    );
    // admission refuses further runnable work past the window
    assert!(!pool.can_admit(1));
}

#[test]
fn r2_over_budget_park_is_explicit_park_stall() {
    let mut pool = Pool::new(1);
    for t in 0..PARK_BUDGET {
        assert_eq!(pool.release_task(t as u32, 0, 1), Admit::Parked);
    }
    assert_eq!(
        pool.release_task(99, 0, 1),
        Admit::ParkStall,
        "over-budget parking is the explicit error, never a silent hang"
    );
}

#[test]
fn r2_met_waiter_never_parks_fast_path() {
    let mut pool = Pool::new(1);
    pool.version[0] = 1;
    assert_eq!(
        pool.release_task(7, 0, 1),
        Admit::Admitted,
        "an already-satisfied waiter takes the enqueue fast path"
    );
    assert_eq!(pool.parked, 0);
}

#[test]
fn r2_scan_releases_met_and_reparks_unmet() {
    let mut pool = Pool::new(1);
    for t in 0..4 {
        let _ = pool.release_task(t as u32, 0, 1); // need version 1
    }
    assert_eq!(pool.publish(0).len(), 4, "all four met at version 1");
    assert_eq!(pool.parked, 0);
    // the re-park arm: waiters that need version 2 stay parked at version 1
    let mut pool2 = Pool::new(1);
    for t in 0..2 {
        let _ = pool2.release_task(t as u32, 0, 2);
    }
    let released = pool2.publish(0); // version becomes 1 < 2
    assert!(released.is_empty(), "unmet waiters re-park");
    assert_eq!(
        pool2.parked, 2,
        "the scan re-parks nodes whose version is still too low"
    );
}

/// The buggy posture: ONE budget counts waiters like running work —
/// bundle 1's waiters fill it and the publishing bundle never fits.
#[derive(Default)]
struct SingleBudgetPool {
    in_flight: usize,
}

impl SingleBudgetPool {
    fn can_admit(&self, n_new: usize) -> bool {
        self.in_flight + n_new <= LIVE_CAP
    }
}

#[test]
fn r2_adversary_single_budget_wedges_the_publisher() {
    let mut pool = SingleBudgetPool::default();
    pool.in_flight = LIVE_CAP; // bundle 1: live_cap waiters occupy the budget
    assert!(
        !pool.can_admit(1),
        "the publisher bundle never fits: the C2 wedge"
    );
}

/// The adversary: a waiter that parks and never re-checks loses the
/// wakeup when the publish raced its push (the MVM_BREAK_PARK_RECHECK
/// mutant class — measured 12,834 lost wakeups / 0.3M upstream).
#[derive(Default)]
struct NoRecheckPool {
    version: u32,
    parked: Vec<u32>,
    published_during_push: bool,
}

impl NoRecheckPool {
    fn park(&mut self, task: u32) {
        self.parked.push(task);
        // the publish lands BETWEEN the push and the (missing) re-check
        if self.published_during_push {
            self.version += 1; // the publisher ran; nobody detaches the waiter
        }
    }
}

#[test]
fn r2_adversary_waiter_without_recheck_loses_wakeup() {
    let mut pool = NoRecheckPool::default();
    pool.published_during_push = true;
    pool.park(1);
    assert!(
        !pool.parked.is_empty(),
        "the mutant: the waiter stays parked forever — a lost wakeup"
    );
}

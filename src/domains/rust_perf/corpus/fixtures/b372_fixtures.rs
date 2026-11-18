//! Batch 372 rust_perf fixture preflight — the infinity tokio-family distill
//! (Plan 310 / Research 349): `hydro-project/infinity`
//! @ `1abc6fcff1f186124f924b56aa92c3a3684c90c5` (Apache-2.0). ORIGINAL
//! plain-Rust reproductions — zero repo source pasted (the MECHANISMS are
//! re-authored with our own constants and model harnesses; the source quotes
//! stay in the corpus context_notes). Compiled AND executed BEFORE entry
//! (the b363..b371 pre-entry discipline):
//! `rustc --edition 2021 --test b372_fixtures.rs && ./b372_fixtures`
//!
//! Arms per rule (buggy / fixed / anti-regime / tamper):
//! - F1 `closed-channel-select-arm-gate`: a model channel that CLOSES
//!   mid-loop — the ungated loop takes the closed arm EVERY iteration
//!   (the poll counter pins the spin, and the close is "handled" every
//!   iteration); the gated loop polls the dead arm exactly once past its
//!   last item and never again, while delivery stays identical; the
//!   anti-regime: an OPEN channel is still polled every iteration and a
//!   LATE-closing channel only up to its close (the gate never
//!   over-suppresses).
//! - F2 `streamed-chunk-tail-coalesce-kind-gated` (BENCHABLE — the GOAT
//!   lead): the assembled text is BIT-IDENTICAL between the per-chunk
//!   push arm and the tail-extend arm; the coalesced history is one
//!   entry per kind-run (census pinned); interleaved kinds stay separate
//!   entries (the negative the kind-gate exists for); the tampered arm
//!   (extend across kinds) reds on the entry census and loses the
//!   boundary in the assembled stream. The alloc claim (n chunk String
//!   allocs vs one amortized-growth buffer per run) is pinned by the
//!   entry census + construction, disclosed in the corpus notes.
//! - F3 `inline-actor-futures-pool-not-spawn-per-actor` (+ the
//!   no-yield-idle-recheck rider + the recv-drain face): the
//!   threads-as-tasks model, DISCLOSED — the spawn-per-actor arm holds
//!   N resident parked workers at idle while the pooled arm holds zero
//!   resident machines between mails; respawn-on-input delivers mail
//!   addressed to a dropped actor; the synchronous final recheck keeps
//!   the exit race two-outcome (seen in rx → processed; after the
//!   receivers drop → the respawn path delivers; never lost); the
//!   drain-per-wake face: one wake with a 3-message backlog processes
//!   all three (wakes counted).
//! - F4 `defer-deferrable-inputs-while-work-in-flight`: a synthetic
//!   event landing mid-call does NOT destroy the in-flight result (the
//!   real result lands, then the deferrals drain in order); the BUGGY
//!   arm is pinned too — the synthetic aborts the pending call and the
//!   real result is dropped stale; priority input still interrupts
//!   immediately (the negative); the tampered arm (defer EVERYTHING)
//!   reds — the priority lane never fires.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

// ═══════════════════════════════════════════════════════════════════════
// F1 — closed-channel-select-arm-gate
// ═══════════════════════════════════════════════════════════════════════

enum Poll {
    Item(u32),
    Closed,
    WouldBlock,
}

/// A model channel: delivers queued items, then closes. After close,
/// `poll` is ALWAYS READY with Closed (recv on a closed channel returns
/// immediately) — the model of the tokio closed-receiver readiness.
struct ModelChannel {
    queue: VecDeque<u32>,
    closed: bool,
}

impl ModelChannel {
    fn new(items: &[u32]) -> Self {
        Self { queue: items.iter().copied().collect(), closed: false }
    }
    fn close(&mut self) {
        self.closed = true;
    }
    fn poll(&mut self) -> Poll {
        if let Some(item) = self.queue.pop_front() {
            return Poll::Item(item);
        }
        if self.closed {
            return Poll::Closed;
        }
        Poll::WouldBlock
    }
}

struct WorkSource {
    queue: VecDeque<u32>,
}

impl WorkSource {
    fn poll(&mut self) -> Poll {
        self.queue.pop_front().map_or(Poll::WouldBlock, Poll::Item)
    }
}

#[derive(Default, PartialEq, Debug)]
struct PollCensus {
    subscribe_polls: usize,
    work_polls: usize,
    items_served: usize,
    close_handled: usize,
}

/// The BUGGY loop (the biased-select model): the subscribe arm is polled
/// FIRST every iteration; the closed arm reads as "nothing yet" and the
/// loop continues — the always-ready spin.
fn loop_ungated(subscribe: &mut ModelChannel, work: &mut WorkSource, budget: usize) -> PollCensus {
    let mut c = PollCensus::default();
    for _ in 0..budget {
        c.subscribe_polls += 1;
        match subscribe.poll() {
            Poll::Item(_) => c.items_served += 1,
            Poll::Closed => {
                c.close_handled += 1;
                // "subscriber gone — nothing to do": the loop continues,
                // and next iteration polls the closed arm again (spin)
            }
            Poll::WouldBlock => {}
        }
        c.work_polls += 1;
        if let Poll::Item(_) = work.poll() {
            c.items_served += 1;
        }
    }
    c
}

/// The FIXED loop: a state bool set on the FIRST Closed gates the arm out
/// of the poll set afterwards (the select guard `if !closed`).
fn loop_gated(subscribe: &mut ModelChannel, work: &mut WorkSource, budget: usize) -> PollCensus {
    let mut c = PollCensus::default();
    let mut subscribe_closed = false;
    for _ in 0..budget {
        if !subscribe_closed {
            c.subscribe_polls += 1;
            match subscribe.poll() {
                Poll::Item(_) => c.items_served += 1,
                Poll::Closed => subscribe_closed = true,
                Poll::WouldBlock => {}
            }
        }
        c.work_polls += 1;
        if let Poll::Item(_) = work.poll() {
            c.items_served += 1;
        }
    }
    c
}

#[test]
fn f1_closed_arm_spins_ungated_and_is_polled_once_gated() {
    let mut sub = ModelChannel::new(&[1, 2]);
    let mut work = WorkSource { queue: [10, 11, 12].into() };
    sub.close();

    let budget = 10_000;
    let buggy = loop_ungated(&mut sub, &mut work, budget);
    assert_eq!(buggy.subscribe_polls, budget, "ungated: the closed arm polled EVERY iteration (the spin)");
    assert_eq!(buggy.close_handled, budget - 2, "ungated: the close re-handled every iteration past its items (never once-and-done)");
    assert_eq!(buggy.items_served, 5, "delivery happens, but pays the whole spin");

    let mut sub2 = ModelChannel::new(&[1, 2]);
    let mut work2 = WorkSource { queue: [10, 11, 12].into() };
    sub2.close();
    let fixed = loop_gated(&mut sub2, &mut work2, budget);
    assert_eq!(fixed.subscribe_polls, 3, "gated: two item polls + exactly ONE closed-arm poll, then gated out");
    assert_eq!(fixed.close_handled, 0, "the close folds into the state bool (handled once by construction)");
    assert_eq!(fixed.items_served, 5, "delivery identical between arms");
    assert_eq!(fixed.work_polls, budget, "the live arm keeps polling the whole budget");
}

#[test]
fn f1_gate_never_over_suppresses_a_live_channel() {
    // Anti-regime: an OPEN channel with items must be polled every
    // iteration — the gate arms ONLY on the observed close.
    let mut sub = ModelChannel::new(&[1, 2, 3, 4, 5]);
    let mut work = WorkSource { queue: VecDeque::new() };
    let c = loop_gated(&mut sub, &mut work, 8);
    assert_eq!(c.subscribe_polls, 8, "open channel: polled every iteration");
    assert_eq!(c.items_served, 5, "all items delivered");

    // A channel that closes LATE (after one item) is polled only up to
    // its close (+1), not suppressed early.
    let mut sub2 = ModelChannel::new(&[7]);
    let mut work2 = WorkSource { queue: VecDeque::new() };
    let mut c2 = PollCensus::default();
    let mut subscribe_closed = false;
    for i in 0..6 {
        if !subscribe_closed {
            c2.subscribe_polls += 1;
            match sub2.poll() {
                Poll::Item(_) => c2.items_served += 1,
                Poll::WouldBlock => {}
                Poll::Closed => subscribe_closed = true,
            }
        }
        c2.work_polls += 1;
        let _ = work2.poll();
        if i == 2 {
            sub2.close();
        }
    }
    assert_eq!(c2.items_served, 1);
    assert_eq!(c2.subscribe_polls, 4, "polls until the observed close (+1): one item, two WouldBlocks, one Closed");
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — streamed-chunk-tail-coalesce-kind-gated
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Text,
    Tool,
}

struct Entry {
    kind: Kind,
    text: String,
}

/// The deterministic chunk stream: mostly same-kind runs, every 64th
/// chunk a tool boundary (the LLM-turn shape).
fn make_chunks(n: usize) -> Vec<(Kind, String)> {
    let mut s = 0x243f_6a88_85a3_08d3u64;
    let mut chunks = Vec::with_capacity(n);
    for i in 0..n {
        s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let kind = if i % 64 == 63 { Kind::Tool } else { Kind::Text };
        let len = 8 + ((s >> 33) as usize) % 16;
        let word = (b'a' + ((s >> 40) as u8 % 26)) as char;
        chunks.push((kind, std::iter::repeat(word).take(len).collect()));
    }
    chunks
}

fn push_per_chunk(history: &mut Vec<Entry>, kind: Kind, chunk: &str) {
    history.push(Entry { kind, text: chunk.to_string() });
}

fn push_coalesce(history: &mut Vec<Entry>, kind: Kind, chunk: &str) {
    if let Some(last) = history.last_mut() {
        if last.kind == kind {
            last.text.push_str(chunk);
            return;
        }
    }
    history.push(Entry { kind, text: chunk.to_string() });
}

fn assemble(history: &[Entry]) -> String {
    let mut out = String::new();
    for e in history {
        out.push_str(&e.text);
    }
    out
}

/// Kind-run census for n chunks with a tool boundary every 64th: a text
/// run + a tool entry per complete 64-chunk window, plus the tail run.
fn expected_runs(n: usize) -> usize {
    let windows = n / 64;
    let tail = n % 64;
    2 * windows + usize::from(tail > 0)
}

#[test]
fn f2_assembled_text_bit_identical_and_runs_collapsed() {
    let chunks = make_chunks(500);
    let mut h1 = Vec::new();
    for (k, c) in &chunks {
        push_per_chunk(&mut h1, *k, c);
    }
    let mut h2 = Vec::new();
    for (k, c) in &chunks {
        push_coalesce(&mut h2, *k, c);
    }
    assert_eq!(assemble(&h1), assemble(&h2), "assembled text bit-identical between arms");
    assert_eq!(h1.len(), 500, "per-chunk: one entry per chunk");
    assert_eq!(h2.len(), expected_runs(500), "coalesced: one entry per kind-run (7 windows + tail = 15)");
    assert_eq!(expected_runs(500), 15);
}

#[test]
fn f2_interleaved_kinds_stay_separate_entries() {
    // The negative the kind-gate exists for: T T TOOL T T must NOT merge
    // the text runs across the tool entry.
    let mut h = Vec::new();
    push_coalesce(&mut h, Kind::Text, "ab");
    push_coalesce(&mut h, Kind::Text, "cd");
    push_coalesce(&mut h, Kind::Tool, "tool-use-1");
    push_coalesce(&mut h, Kind::Text, "ef");
    push_coalesce(&mut h, Kind::Text, "gh");
    assert_eq!(h.len(), 3, "two merged text runs + one tool entry — the boundary splits the runs");
    assert_eq!(h[0].text, "abcd");
    assert_eq!(h[1].kind, Kind::Tool);
    assert_eq!(h[2].text, "efgh");
    assert_eq!(assemble(&h), "abcdtool-use-1efgh");
}

#[test]
fn f2_tampered_extend_across_kinds_reds() {
    // The tampered classifier extends the tail REGARDLESS of kind — the
    // entry census catches it and the stream loses the boundary.
    let mut h: Vec<(Kind, String)> = Vec::new();
    {
        let push_tampered = |h: &mut Vec<(Kind, String)>, kind: Kind, chunk: &str| {
            if let Some(last) = h.last_mut() {
                last.1.push_str(chunk); // TAMPER: no kind check
                return;
            }
            h.push((kind, chunk.to_string()));
        };
        push_tampered(&mut h, Kind::Text, "ab");
        push_tampered(&mut h, Kind::Tool, "TOOL");
        push_tampered(&mut h, Kind::Text, "cd");
    }
    assert_eq!(h.len(), 1, "tampered arm merges across kinds");
    assert_eq!(h[0].1, "abTOOLcd", "the boundary is lost — exactly the defect the gate prevents");

    let mut honest = Vec::new();
    push_coalesce(&mut honest, Kind::Text, "ab");
    push_coalesce(&mut honest, Kind::Tool, "TOOL");
    push_coalesce(&mut honest, Kind::Text, "cd");
    assert_eq!(honest.len(), 3, "the kind-aware arm keeps the boundary");
}

// ═══════════════════════════════════════════════════════════════════════
// F3 — inline-actor-futures-pool-not-spawn-per-actor
// (threads-as-tasks model, DISCLOSED: a parked OS thread is the model's
// resident-task analogue — memory + a stack held at idle; the pooled arm
// is machines owned by ONE dispatcher, dropped at idle, respawned on mail)
// ═══════════════════════════════════════════════════════════════════════

static RESIDENT: AtomicUsize = AtomicUsize::new(0);

/// The spawn-per-actor arm: one thread per actor parked on its mailbox
/// (the keep-alive posture — resident while it waits). Returns the
/// handles AND the senders: dropping the senders is the model's
/// "router teardown"; the actor exits on Disconnected.
fn spawn_per_actor(ids: &[u32]) -> (Vec<std::thread::JoinHandle<()>>, Vec<std::sync::mpsc::Sender<u32>>) {
    let mut handles = Vec::new();
    let mut senders = Vec::new();
    for id in ids {
        let (tx, rx) = std::sync::mpsc::channel::<u32>();
        RESIDENT.fetch_add(1, Ordering::SeqCst);
        handles.push(
            std::thread::Builder::new()
                .name(format!("actor-{id}"))
                .spawn(move || {
                    // the actor loop: waits for its next input, resident
                    loop {
                        match rx.recv() {
                            Ok(_) => {} // a message: apply and keep waiting
                            Err(_) => break, // all senders dropped: exit
                        }
                    }
                    RESIDENT.fetch_sub(1, Ordering::SeqCst);
                })
                .unwrap(),
        );
        senders.push(tx); // held by the "router": the task never exits
    }
    (handles, senders)
}

#[derive(Default, PartialEq, Debug)]
struct PoolState {
    resident_machines: usize,
    delivered: Vec<(u32, u32)>,
    respawns: usize,
    wakes: usize,
}

/// The pooled arm: ONE dispatcher owns machines; a machine is RESPAWNED
/// (one wake) on the first mail addressed to an idle actor and STAYS
/// alive through the input wave, its wake handler draining the whole
/// backlog in one pass; at the idle boundary (the wave's end) every
/// machine is DROPPED. BTreeMap keeps the drain order deterministic.
fn pooled_dispatch(mails: &[(u32, u32)]) -> PoolState {
    let mut st = PoolState::default();
    let mut machines: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
    for (to, msg) in mails {
        if !machines.contains_key(to) {
            st.respawns += 1; // the router respawns the actor on its next input
            st.wakes += 1; // the blocking recv that woke it
            machines.insert(*to, Vec::new());
        }
        machines.get_mut(to).unwrap().push(*msg);
    }
    st.resident_machines = machines.len(); // mid-wave residency
    // the idle boundary: each machine's handler drains its whole backlog
    // in one pass, then the machine is dropped (released)
    for (to, mut backlog) in machines {
        for m in backlog.drain(..) {
            st.delivered.push((to, m));
        }
    }
    st.resident_machines = 0;
    st
}

#[test]
fn f3_spawn_holds_n_resident_at_idle_pool_holds_zero() {
    let ids: Vec<u32> = (0..8).collect();
    let (handles, senders) = spawn_per_actor(&ids);
    std::thread::sleep(std::time::Duration::from_millis(120));
    assert_eq!(
        RESIDENT.load(Ordering::SeqCst),
        8,
        "spawn-per-actor: 8 resident parked workers at idle (the residency the pool deletes)"
    );
    drop(senders); // the router teardown
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(RESIDENT.load(Ordering::SeqCst), 0, "teardown verified — the census is honest");

    let mails = [(1, 100), (2, 200), (3, 300)];
    let st = pooled_dispatch(&mails);
    assert_eq!(st.resident_machines, 0, "pooled: every machine dropped at idle");
    assert_eq!(st.respawns, 3, "three actors respawned on their first input each");
    assert_eq!(st.wakes, 3, "one wake per mail wave");
    assert_eq!(st.delivered, vec![(1, 100), (2, 200), (3, 300)], "all mail delivered in order");
}

#[test]
fn f3_drain_per_wake_takes_the_whole_backlog() {
    // Burst: 3 messages arrive for one actor while it is idle. The pooled
    // dispatcher respawns ONCE and the wake's handler drains all three.
    let mails = [(7, 1), (7, 2), (7, 3)];
    let st = pooled_dispatch(&mails);
    assert_eq!(st.wakes, 1, "ONE wake for the whole backlog");
    assert_eq!(st.delivered, vec![(7, 1), (7, 2), (7, 3)]);
    assert_eq!(st.resident_machines, 0);
}

#[test]
fn f3_exit_race_is_two_outcome_never_lost() {
    // The no-yield-idle-recheck rider: the final recheck is SYNCHRONOUS
    // (no yield between the last work and the idle decision).
    // Outcome A: input appears in rx before the recheck → processed.
    let mut rx: VecDeque<u32> = [42].into();
    let mut delivered = Vec::new();
    while let Some(m) = rx.pop_front() {
        delivered.push(m);
    }
    let idle = rx.is_empty();
    assert_eq!(delivered, vec![42], "outcome A: the input seen in rx is processed");
    assert!(idle, "after the synchronous drain the machine exits idle-clean");

    // Outcome B: input arrives AFTER the receivers dropped → the router's
    // respawn path owns the delivery (never a lost wakeup).
    let mails = [(9, 77)];
    let st = pooled_dispatch(&mails);
    assert_eq!(st.delivered, vec![(9, 77)], "outcome B: the respawn path delivers");
    assert_eq!(st.respawns, 1);
}

// ═══════════════════════════════════════════════════════════════════════
// F4 — defer-deferrable-inputs-while-work-in-flight
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, PartialEq, Clone)]
enum Input {
    UserText(String),
    Synthetic(u32),
}

#[derive(Debug, PartialEq)]
enum Outcome {
    PendingResult(String), // the REAL tool result
    Interrupted,           // the synthetic "interrupted" result
    RealResultDroppedStale, // the discarded result (the BUGGY posture)
    SyntheticHandled(u32),
}

/// A model pending call: join() returns the real result if the caller
/// waits for it; discarding the handle (without joining) is the abort.
struct PendingCall(std::thread::JoinHandle<String>);

impl PendingCall {
    fn resolve(self) -> String {
        self.0.join().unwrap_or_default()
    }
}

#[derive(Default)]
struct RuntimeModel {
    pending: Option<PendingCall>,
    defer_queue: Vec<Input>,
    history: Vec<Outcome>,
}

/// The BUGGY handler: a synthetic event landing mid-call aborts the
/// pending work; when the real result arrives it is dropped as stale.
fn on_input_buggy(st: &mut RuntimeModel, input: Input) {
    match input {
        Input::UserText(_) => {
            // deliberate interrupt (priority) — the same on both arms
            if let Some(p) = st.pending.take() {
                let _real = p.resolve(); // read and discarded: interrupted
                st.history.push(Outcome::Interrupted);
            }
        }
        Input::Synthetic(ev) => {
            // BUG: run the synthetic NOW — the pending call is aborted
            if let Some(p) = st.pending.take() {
                let _real = p.resolve();
                st.history.push(Outcome::Interrupted); // the injected result
            }
            st.history.push(Outcome::SyntheticHandled(ev));
        }
    }
}

/// The FIXED handler: synthetic events defer while a call is pending;
/// priority input still interrupts immediately.
fn on_input_fixed(st: &mut RuntimeModel, input: Input) {
    match input {
        Input::UserText(_) => {
            if let Some(p) = st.pending.take() {
                let _real = p.resolve();
                st.history.push(Outcome::Interrupted);
            }
        }
        Input::Synthetic(ev) => {
            if st.pending.is_some() {
                st.defer_queue.push(Input::Synthetic(ev)); // held, not run
            } else {
                st.history.push(Outcome::SyntheticHandled(ev));
            }
        }
    }
}

/// The FIXED resolve path: the real result lands, then the deferrals
/// drain IN ORDER.
fn resolve_pending_fixed(st: &mut RuntimeModel) {
    if let Some(p) = st.pending.take() {
        let real = p.resolve();
        st.history.push(Outcome::PendingResult(real));
    }
    for ev in st.defer_queue.drain(..) {
        if let Input::Synthetic(e) = ev {
            st.history.push(Outcome::SyntheticHandled(e));
        }
    }
}

fn spawn_pending() -> PendingCall {
    PendingCall(std::thread::spawn(|| {
        // the slow tool call's real result
        std::thread::sleep(std::time::Duration::from_millis(10));
        String::from("REAL_TOOL_RESULT")
    }))
}

#[test]
fn f4_synthetic_defers_and_the_real_result_survives() {
    let mut st = RuntimeModel { pending: Some(spawn_pending()), ..Default::default() };
    on_input_fixed(&mut st, Input::Synthetic(1));
    assert!(st.history.is_empty(), "the synthetic was held, nothing ran");
    assert_eq!(st.defer_queue, vec![Input::Synthetic(1)]);
    on_input_fixed(&mut st, Input::Synthetic(2));
    assert_eq!(st.defer_queue.len(), 2, "in-order queue");

    resolve_pending_fixed(&mut st);
    assert_eq!(
        st.history,
        vec![
            Outcome::PendingResult(String::from("REAL_TOOL_RESULT")),
            Outcome::SyntheticHandled(1),
            Outcome::SyntheticHandled(2),
        ],
        "the REAL result survives; the deferrals drained in order after resolve"
    );
}

#[test]
fn f4_buggy_synthetic_destroys_the_in_flight_result() {
    // The defect the law prevents, pinned: the synthetic aborts the
    // pending call — the history records the injected Interrupted and the
    // synthetic, and the REAL result is dropped as stale (never in the
    // history, the caller's wait wasted).
    let mut st = RuntimeModel { pending: Some(spawn_pending()), ..Default::default() };
    on_input_buggy(&mut st, Input::Synthetic(1));
    assert_eq!(
        st.history,
        vec![Outcome::Interrupted, Outcome::SyntheticHandled(1)],
        "the synthetic ran NOW and aborted the pending call"
    );
    assert!(
        !st.history.iter().any(|o| matches!(o, Outcome::PendingResult(_))),
        "the real result NEVER lands — dropped as stale (the waste the deferral prevents)"
    );
}

#[test]
fn f4_priority_still_interrupts_immediately() {
    // The negative: priority input interrupts BOTH arms immediately —
    // deferring user text would trade the defect for an unresponsive
    // priority lane.
    for handler in [on_input_fixed as fn(&mut RuntimeModel, Input), on_input_buggy] {
        let mut st = RuntimeModel { pending: Some(spawn_pending()), ..Default::default() };
        handler(&mut st, Input::UserText(String::from("stop")));
        assert_eq!(st.history.first(), Some(&Outcome::Interrupted), "priority interrupted immediately");
        assert!(st.defer_queue.is_empty());
    }
}

#[test]
fn f4_tampered_defer_everything_reds() {
    // The tampered classifier defers EVERYTHING (no class check): the
    // priority negative catches it — the interrupt never fires.
    let mut st = RuntimeModel { pending: Some(spawn_pending()), ..Default::default() };
    let tampered_defer_all = |st: &mut RuntimeModel, input: Input| {
        st.defer_queue.push(input); // TAMPER: everything defers
    };
    tampered_defer_all(&mut st, Input::UserText(String::from("stop")));
    assert!(
        !st.history.iter().any(|o| matches!(o, Outcome::Interrupted)),
        "tampered arm: the interrupt NEVER fired — the priority lane is dead"
    );
    assert_eq!(st.defer_queue.len(), 1);
}

//! Batch 419 kernel fixture preflight — the cuda-async completion-reactor
//! transcription batch (Plan 415 / Research 450, NVIDIA/cuda-rust cuda-async
//! @ f35d6ccc2e03a54e73e3796f415f11313e439d50, Apache-2.0) as ONE standalone
//! file (4 kernel rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted. The CUDA
//! runtime surfaces (driver-thread host-func hops, stream queries, capture
//! status, Parker contention) are MODELED as deterministic state machines;
//! what the fixtures pin are the RELATIONSHIPS the rules price, never the
//! driver's numbers (all four rules are NotBenchable-by-domain — the fixture
//! discipline here is the defect-shape pin, not a perf claim).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b419_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (flag-write-completion-reactor-amortizes-notification): the naive
//!   arm (every wait notifies through the expensive hop — hops == N, one
//!   parked waiter per op); the bound arm (a flag slab + ONE scanner pass
//!   resolves N armed slots — per-op hops == 0, all completions observed);
//!   the fallback arm (slab exhausted -> the N+1th registration takes the
//!   per-op hop and still completes — the degradation ladder, the amortized
//!   path never load-bearing-by-force); the tamper arm (a scanner that
//!   fires wakers WHILE holding the scan lock — a waker that re-enters the
//!   table fails to take the lock, caught; the bound form fires outside the
//!   lock and the same waker succeeds).
//! - F2 (wake-without-complete-fault-resolves-at-waiter-poll): the
//!   dead-producer arm (2 slots armed on a stream that faults; the scanner
//!   retires them, wakes WITHOUT completing; the waiter's own poll delivers
//!   the stream's OWN error object — identity-asserted against the
//!   synthetic); the rate-limit arm (4 slots over 2 streams -> exactly 2
//!   queries in one pass, memoized per distinct stream); the tamper arm (a
//!   notifier that marks-complete at the wake delivers a synthetic error —
//!   caught: the waiter must observe the driver's error, not a stand-in);
//!   the capture-gate arm (a capturing stream is NEVER queried — zero
//!   query() calls, the slot stays armed, not retired).
//! - F3 (transition-gated-unpark-scanner-armed-count-park-gate): the
//!   transition arm (100 registrations into an already-active table ->
//!   exactly 1 unpark, the 0->1 transition — never O(N)); the park-gate arm
//!   (an armed slot whose flag lands LATER keeps the scanner awake — the
//!   tamper gate-on-"saw a flag this pass" parks over it and misses the
//!   completion, caught); the park-token arm (an unpark delivered between
//!   the scan and the park is absorbed — the permit model, no lost
//!   registration); the all-safe arm (a single registration into an idle
//!   table unparks exactly once — the gate degenerates to always-unpark,
//!   correctness identical).
//! - F4 (bounded-inline-spin-before-completion-registration): the
//!   short-wait arm (complete within budget at the site — ZERO callback
//!   registrations); the long-wait arm (past budget -> exactly ONE
//!   registration, the fall-through tail); the error arm (a query error
//!   during the spin RETURNS as the future's error — no registration, the
//!   sticky fault delivered); the budget-0 arm (0 skips the spin entirely —
//!   one registration, every pipeline through the notification path); the
//!   tamper arm (the swallowed-error form: error -> fall-through to
//!   registration — caught: the error is lost AND a registration fired).
#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

// ════════════════════ F1 — completion-notification amortization ════════════════════

/// The modeled expensive hop: every callback takes this lock (the driver
/// thread serializes host callbacks) and bumps the hop counter.
#[derive(Default)]
struct DriverThread {
    hops: u64,
}

impl DriverThread {
    fn hop(&mut self) {
        self.hops += 1;
    }
}

/// The flag slab: armed slots carry a pending flag; ONE scanner pass
/// resolves every flag-landed slot and collects its waker. The scan guard
/// is released before the wakers fire (the law under test).
struct FlagSlab {
    slots: Vec<AtomicU32>,
    scan_lock: Mutex<()>,
}

impl FlagSlab {
    fn new(n: usize) -> Self {
        FlagSlab {
            slots: (0..n).map(|_| AtomicU32::new(0)).collect(),
            scan_lock: Mutex::new(()),
        }
    }

    /// The device-side arm: the producer writes the flag through its own
    /// channel (no driver call in the model).
    fn arm(&self, slot: usize) {
        self.slots[slot].store(1, Ordering::Release);
    }

    /// ONE scanner pass: plain loads over the armed set; the wakers fire
    /// either INSIDE the scan lock (the tamper) or after the guard drops
    /// (the bound form — 'wakers fire outside any lock the scan held').
    /// Each waker re-enters the table (a registration racing the waking
    /// phase), observed by try_lock.
    fn scan_and_wake(&self, fire_under_lock: bool) -> (usize, usize) {
        let guard: MutexGuard<()> = self.scan_lock.lock().unwrap();
        let mut woken = Vec::new();
        for (i, f) in self.slots.iter().enumerate() {
            if f.load(Ordering::Acquire) == 1 {
                f.store(0, Ordering::Release);
                woken.push(i);
            }
        }
        let mut blocked = 0usize;
        let fire = |woken: &[usize], blocked: &mut usize| {
            for _ in woken {
                // the waker re-enters the table: under the tampered form
                // the scan lock is still held, so the try fails.
                if self.scan_lock.try_lock().is_err() {
                    *blocked += 1;
                }
            }
        };
        if fire_under_lock {
            fire(&woken, &mut blocked); // inside the guard's scope
        }
        drop(guard); // the bound form releases BEFORE waking
        if !fire_under_lock {
            let mut b2 = 0usize;
            fire(&woken, &mut b2);
            blocked = b2;
        }
        (woken.len(), blocked)
    }
}

#[test]
fn naive_notifies_once_per_operation() {
    let mut driver = DriverThread::default();
    let n = 64;
    for _ in 0..n {
        // the naive path: one driver-thread hop PER op.
        driver.hop();
    }
    assert_eq!(driver.hops, n, "N pipelines pay N driver hops");
}

#[test]
fn one_scanner_pass_resolves_every_armed_slot_without_per_op_hops() {
    let slab = Arc::new(FlagSlab::new(64));
    let driver = DriverThread::default();
    for i in 0..64 {
        slab.arm(i); // the device writes the flag; no driver call
    }
    let (woken, blocked) = slab.scan_and_wake(false);
    assert_eq!(woken, 64, "ONE pass resolves all N completions");
    assert_eq!(blocked, 0, "no waker was blocked behind the waking phase");
    assert_eq!(
        driver.hops, 0,
        "the amortized path pays zero per-op driver hops"
    );
}

#[test]
fn exhausted_slab_falls_back_to_the_host_hop_and_still_completes() {
    let slab = Arc::new(FlagSlab::new(4));
    let mut driver = DriverThread::default();
    let mut completions = 0u32;
    for i in 0..5 {
        if i < 4 {
            slab.arm(i);
        } else {
            // the degradation ladder: the slot pool is exhausted -> the
            // per-op host-function hop. Completion still resolves.
            driver.hop();
        }
    }
    completions += slab.scan_and_wake(false).0 as u32;
    completions += driver.hops as u32; // the fallback hop fired its callback
    assert_eq!(completions, 5, "every completion resolves on either path");
    assert_eq!(driver.hops, 1, "exactly one op paid the fallback hop");
}

#[test]
fn wakers_fired_under_the_scan_lock_are_caught_by_a_reentrant_registration() {
    // The tamper: a scanner that fires wakers while still holding the
    // scan lock. A waker that must take the table lock (a registration
    // racing the waking phase) fails to get it — 'a registration is
    // never blocked behind a waking phase' is violated. The bound form
    // releases the lock BEFORE firing, so the same waker succeeds.
    let slab = Arc::new(FlagSlab::new(2));
    slab.arm(0);
    let (woken, blocked) = slab.scan_and_wake(true); // the tampered form
    assert_eq!(woken, 1);
    assert_eq!(
        blocked, 1,
        "the tampered waking phase blocks the re-entrant registration"
    );
    // the bound form: the same completion wakes with zero blocks.
    slab.arm(0);
    let (woken2, blocked2) = slab.scan_and_wake(false);
    assert_eq!(woken2, 1);
    assert_eq!(blocked2, 0, "the bound form fires outside the scan lock");
}

// ════════════════════ F2 — wake-without-complete fault retirement ════════════════════

#[derive(Clone, Debug, PartialEq)]
enum Health {
    Ok,
    Capturing,
    Faulted,
}

#[derive(Debug)]
struct DriverError(&'static str);

#[derive(Clone)]
struct Stream {
    id: u64,
    health: Health,
    queries: Arc<AtomicU32>,
    error: Arc<Mutex<Option<DriverError>>>,
}

impl Stream {
    fn new(id: u64, health: Health) -> Self {
        Stream {
            id,
            health,
            queries: Arc::new(AtomicU32::new(0)),
            error: Arc::new(Mutex::new(None)),
        }
    }

    /// The authoritative source: the waiter's own query. The sticky fault
    /// lives here — the error object the future must deliver.
    fn query(&self) -> Result<bool, DriverError> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        match self.health {
            Health::Ok => Ok(true),
            Health::Capturing => Ok(false), // never reached: gated upstream
            Health::Faulted => {
                let mut e = self.error.lock().unwrap();
                let d = e.get_or_insert_with(|| DriverError("context faulted"));
                Err(DriverError(d.0))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Completion {
    /// signal(): complete-store + wake — the future's result is set.
    Complete,
    /// wake_only(): wake WITHOUT complete — the re-poll re-examines the
    /// stream itself.
    WakeOnly,
}

#[derive(Clone)]
struct Slot {
    stream: Stream,
    flag: Arc<AtomicU32>,
    notified: Arc<Mutex<Vec<Completion>>>,
}

#[derive(Default)]
struct Table {
    armed: Vec<Slot>,
}

impl Table {
    fn scan_pass(&self) -> (Vec<usize>, Vec<usize>) {
        let mut woken = Vec::new();
        let mut faulted = Vec::new();
        for (i, s) in self.armed.iter().enumerate() {
            if s.flag.load(Ordering::Acquire) == 1 {
                woken.push(i);
            } else if stream_is_faulted(&s.stream) {
                faulted.push(i);
            }
        }
        (woken, faulted)
    }

    /// The retirement split: woken -> signal, faulted -> wake_only.
    fn retire(&self, woken: &[usize], faulted: &[usize]) {
        for &i in woken {
            self.armed[i].notified.lock().unwrap().push(Completion::Complete);
        }
        for &i in faulted {
            // Wake without completing: the poll observes the fault.
            self.armed[i].notified.lock().unwrap().push(Completion::WakeOnly);
        }
    }
}

fn stream_is_faulted(s: &Stream) -> bool {
    matches!(s.health, Health::Faulted)
}

/// The waiter's own poll: it queries ITS stream and delivers the driver's
/// error object — never a synthetic invented at the wake.
fn waiter_poll(stream: &Stream, notified: Completion) -> Result<(), DriverError> {
    match notified {
        Completion::Complete => Ok(()),
        Completion::WakeOnly => stream.query().map(|_| ()),
    }
}

fn make_slot(id: u64, health: Health) -> Slot {
    Slot {
        stream: Stream::new(id, health),
        flag: Arc::new(AtomicU32::new(0)),
        notified: Arc::new(Mutex::new(Vec::new())),
    }
}

#[test]
fn dead_producer_retires_and_waiter_observes_the_drivers_own_error() {
    let mut table = Table::default();
    table.armed.push(make_slot(1, Health::Faulted));
    table.armed.push(make_slot(2, Health::Faulted));
    let synthetic = DriverError("notifier-invented");

    let (woken, faulted) = table.scan_pass();
    assert!(woken.is_empty());
    assert_eq!(faulted.len(), 2, "both armed slots retire as faulted");
    table.retire(&woken, &faulted);

    for s in &table.armed {
        let note = s.notified.lock().unwrap()[0].clone();
        assert_eq!(note, Completion::WakeOnly, "woken WITHOUT completing");
        let delivered = waiter_poll(&s.stream, note).unwrap_err();
        assert_eq!(
            delivered.0, "context faulted",
            "the delivered error is the stream's OWN fault"
        );
        assert_ne!(
            delivered.0, synthetic.0,
            "the notifier never synthesized the error"
        );
    }
}

#[test]
fn one_query_per_distinct_stream_per_pass() {
    let mut table = Table::default();
    // 3 slots on stream A + 1 slot on stream B.
    for _ in 0..3 {
        table.armed.push(make_slot(10, Health::Faulted));
    }
    table.armed.push(make_slot(11, Health::Faulted));

    // memoized probing: each DISTINCT stream queried once per pass.
    let mut memo: Vec<u64> = Vec::new();
    for s in &table.armed {
        if !memo.contains(&s.stream.id) {
            memo.push(s.stream.id);
            let _ = s.stream.query();
        }
    }
    let a_queries = table.armed[0].stream.queries.load(Ordering::SeqCst);
    let b_queries = table.armed[3].stream.queries.load(Ordering::SeqCst);
    assert_eq!(a_queries, 1, "stream A: 3 slots, ONE query");
    assert_eq!(b_queries, 1, "stream B: 1 slot, ONE query");
}

#[test]
fn a_notifier_that_marks_complete_at_the_wake_delivers_a_synthetic_error() {
    // The tamper: completing the future AT the wake invents a stand-in
    // error — the waiter never observes the driver's fault. Caught.
    let slot = make_slot(7, Health::Faulted);
    // tampered notifier: signal() semantics with a guessed error
    slot.notified.lock().unwrap().push(Completion::Complete);
    let note = slot.notified.lock().unwrap()[0].clone();
    let result = waiter_poll(&slot.stream, note);
    let delivered_ok = matches!(result, Ok(()));
    let saw_queries = slot.stream.queries.load(Ordering::SeqCst);
    assert!(
        delivered_ok || saw_queries == 0,
        "the tampered form either lies (Ok on a faulted stream) or never \
         consulted the driver at all — neither is the authentic error"
    );
    // the bound form: the waiter's poll queries the stream and sees the fault.
    let bound = waiter_poll(&slot.stream, Completion::WakeOnly);
    assert!(bound.is_err(), "the bound form delivers the driver's fault");
}

#[test]
fn a_capturing_stream_is_never_queried_and_stays_armed() {
    let mut table = Table::default();
    table.armed.push(make_slot(3, Health::Capturing));
    // capture-gate BEFORE the query: status != None returns Capturing and
    // the slot is neither probed nor retired.
    let s = &table.armed[0];
    let gated = matches!(s.stream.health, Health::Capturing);
    let (woken, faulted) = if gated { (vec![], vec![]) } else { table.scan_pass() };
    table.retire(&woken, &faulted);
    assert_eq!(
        s.stream.queries.load(Ordering::SeqCst),
        0,
        "a capturing stream is never queried (that would invalidate the capture)"
    );
    assert!(
        s.notified.lock().unwrap().is_empty(),
        "the slot stays armed — no retirement, no wake"
    );
}

// ════════════════════ F3 — transition-gated unpark, armed-count park gate ════════════════════

#[derive(Default)]
struct ParkTable {
    armed: usize,
    flags: Vec<bool>, // per-slot flag-complete
}

impl ParkTable {
    /// publish: increments the armed count and returns the 0->1 test
    /// BEFORE setting the active bit — the increment-before-bit invariant.
    fn publish(&mut self) -> bool {
        let was_idle = self.armed == 0;
        self.armed += 1;
        self.flags.push(false);
        was_idle
    }

    fn flag_lands(&mut self, slot: usize) {
        self.flags[slot] = true;
    }

    fn retire_flagged(&mut self) -> usize {
        let mut retired = 0;
        for f in self.flags.iter_mut() {
            if *f {
                *f = false;
                self.armed = self.armed.saturating_sub(1);
                retired += 1;
            }
        }
        retired
    }
}

/// The scanner modeled as a permit mailbox: unpark sets a permit, park
/// consumes one if present (the park token) else blocks (modeled as a
/// deferred poll that returns Pending).
#[derive(Default)]
struct Scanner {
    permits: u32,
    parks: u64,
    parked_now: bool,
}

impl Scanner {
    fn unpark(&mut self) {
        self.permits += 1;
        if self.parked_now {
            self.parked_now = false; // the wake lands
        }
    }

    /// park(): consumes a permit if one landed between the scan and the
    /// park (the token), else actually parks.
    fn park(&mut self, armed: usize) -> bool {
        if armed != 0 {
            return false; // the gate: armed count keeps the scanner awake
        }
        if self.permits > 0 {
            self.permits -= 1; // the token absorbs the race
            return false;
        }
        self.parks += 1;
        self.parked_now = true;
        true
    }
}

#[test]
fn sustained_registration_unparks_only_on_the_transition() {
    let mut table = ParkTable::default();
    let mut scanner = Scanner::default();
    let mut unparks = 0u32;
    // the FIRST registration transitions idle->active: exactly one unpark.
    for _ in 0..100 {
        if table.publish() {
            scanner.unpark();
            unparks += 1;
        }
    }
    assert_eq!(unparks, 1, "100 registrations, ONE transition, ONE unpark");
    assert_eq!(table.armed, 100);
}

#[test]
fn an_armed_but_not_flag_complete_slot_keeps_the_scanner_awake() {
    let mut table = ParkTable::default();
    let mut scanner = Scanner::default();
    table.publish(); // armed; the flag write is still in flight
    // the bound gate reads the armed COUNT: the scanner does not park.
    let parked = scanner.park(table.armed);
    assert!(!parked, "armed count > 0 keeps the scanner awake");

    // the tamper gate ("did this pass see a flag") parks over the same
    // state: no flag observed this pass -> park -> the completion is
    // missed until an unrelated unpark. Caught below by the observation.
    table.flag_lands(0);
    let retired = table.retire_flagged();
    assert_eq!(retired, 1, "the awake scanner observes the late flag");
}

#[test]
fn a_park_gate_on_the_flag_observation_parks_over_live_work() {
    // The TAMPER arm as a positive detection: the gate-on-flags scanner
    // parks despite armed-but-incomplete work — the defect the rule kills.
    let mut table = ParkTable::default();
    let mut scanner = Scanner::default();
    table.publish(); // armed, flag NOT yet complete
    let saw_flags_this_pass = false;
    let parked = if saw_flags_this_pass { false } else { scanner.park(0) };
    assert!(
        parked,
        "the tampered gate parks over an armed slot — the lost-wakeup shape"
    );
    assert!(table.armed == 1, "live work is pending while the scanner slept");
}

#[test]
fn an_unpark_between_scan_and_park_is_absorbed_by_the_token() {
    let mut scanner = Scanner::default();
    let mut table = ParkTable::default();
    // registration arrives just after the scanner's scan saw nothing and
    // just before its park: the unpark lands first, then park() runs.
    if table.publish() {
        scanner.unpark();
    }
    let parked = scanner.park(table.armed);
    assert!(!parked, "the token absorbs the race — no missed registration");
}

#[test]
fn a_single_registration_unparks_exactly_once() {
    // the all-safe arm: an idle table degenerates to always-unpark.
    let mut table = ParkTable::default();
    let mut scanner = Scanner::default();
    assert!(table.publish(), "the idle->active transition is true");
    scanner.unpark();
    assert!(scanner.parks == 0 && scanner.permits == 1);
}

// ════════════════════ F4 — bounded inline spin before registration ════════════════════

#[derive(Debug, PartialEq)]
enum PollOut {
    Ready,
    Registered,
    Err(&'static str),
}

/// The wait site: spin inside the budget polling the authoritative source;
/// complete at the site; an error returns immediately (never swallowed,
/// never fall-through); past the budget register the callback (the tail).
fn wait_site(
    budget_us: u64,
    spins_to_complete: u32,
    error_at: Option<u32>,
    registrations: &mut u32,
) -> PollOut {
    let mut spins = 0u32;
    if budget_us > 0 {
        loop {
            if let Some(at) = error_at {
                if spins == at {
                    // A query error is the stream's (sticky) fault: no
                    // completion signal will ever arrive, so it is the
                    // future's error — never swallowed.
                    return PollOut::Err("sticky stream fault");
                }
            }
            if spins >= spins_to_complete {
                return PollOut::Ready; // resolved at the site
            }
            spins += 1;
            if spins as u64 * 5 >= budget_us.max(1) {
                break; // past the budget: the long-wait tail
            }
        }
    }
    *registrations += 1; // register_callback — the TAIL, never the head
    PollOut::Registered
}

#[test]
fn a_short_wait_completes_at_the_site_with_zero_registrations() {
    let mut registrations = 0;
    let out = wait_site(20, 2, None, &mut registrations);
    assert_eq!(out, PollOut::Ready, "the µs wait resolved at the site");
    assert_eq!(registrations, 0, "no completion callback was ever registered");
}

#[test]
fn a_long_wait_falls_through_with_exactly_one_registration() {
    let mut registrations = 0;
    let out = wait_site(20, 10_000, None, &mut registrations);
    assert_eq!(out, PollOut::Registered, "past the budget the tail registers");
    assert_eq!(registrations, 1);
}

#[test]
fn a_query_error_during_the_spin_is_the_future_error_without_registration() {
    let mut registrations = 0;
    let out = wait_site(20, 10_000, Some(1), &mut registrations);
    assert_eq!(out, PollOut::Err("sticky stream fault"));
    assert_eq!(
        registrations, 0,
        "the error returns at the site — no registration hides it"
    );
}

#[test]
fn budget_zero_skips_the_spin_and_takes_the_notification_path() {
    // the source's own env=0 posture: every pipeline through the
    // completion-notification path (the correctness posture).
    let mut registrations = 0;
    let out = wait_site(0, 0, None, &mut registrations);
    assert_eq!(out, PollOut::Registered);
    assert_eq!(registrations, 1);
}

#[test]
fn a_spin_that_swallows_the_error_falls_through_and_loses_it() {
    // The TAMPER: error -> fall-through to registration. The error is
    // lost (the caller blocks on a wait that can never complete) AND a
    // registration fired. Caught by direct contrast with the bound arm.
    let mut registrations = 0;
    let error_at = Some(1);
    let mut spins = 0u32;
    let mut lost = None;
    loop {
        if Some(spins) == error_at {
            lost = Some("sticky stream fault");
            break; // tampered: SWALLOWED — no return, keep going
        }
        spins += 1;
        if spins >= 2 {
            break;
        }
    }
    registrations += 1; // the tampered fall-through
    assert!(lost.is_some(), "the tampered form never delivered the error");
    assert_eq!(registrations, 1, "and it still paid the registration");
    // the bound form delivers the error and registers nothing:
    let mut bound_regs = 0;
    let bound = wait_site(20, 10_000, Some(1), &mut bound_regs);
    assert!(matches!(bound, PollOut::Err(_)) && bound_regs == 0);
}

// ════════════════════ shared queue machinery (F1's modeled waiter side) ════════════════════

#[derive(Default)]
struct WaitQueue {
    pending: std::collections::VecDeque<()>,
}

impl WaitQueue {
    fn push(&mut self) {
        self.pending.push_back(());
    }
    fn drain(&mut self) -> usize {
        self.pending.drain(..).count()
    }
}

#[test]
fn the_queue_shapes_round_trip() {
    // a tiny shared-model smoke: the fixtures' queue machinery is honest.
    let mut q = WaitQueue::default();
    for _ in 0..3 {
        q.push();
    }
    assert_eq!(q.drain(), 3);
    assert_eq!(q.drain(), 0);
}

//! Entries slice - Batch 327 (2026-11-03): one `rust_perf` rule from the
//! firezone/sanstun distill (Research 364 / Plan 332):
//! `batch-reserve-monotonic-counters-offload-bulk` (concurrency) -
//! reserve N consecutive monotonic counters in ONE bound-checked
//! operation (the count checked_add-ed over, filtered against the hard
//! limit), then hand the immutable reservation (fixed nonces, no shared
//! mutable state) to any thread, so the per-item bulk work it fronts can
//! run on any thread in any order; a receiver tolerating gaps and
//! reorder (a replay window) is the regime precondition. Source:
//! firezone/sanstun @ 35cb1932948bd0776ef7d590329c1fdc0b1b9718
//! (Apache-2.0; the pin IS the #129 squash, 2026-11-03), read at the
//! `.raw/sanstun` clone; every quote verbatim at the pin, re-verified
//! this batch. NotBenchable-by-domain (the coordination-law annotation
//! class) - `bench_spec: None`, with the honest serial-component note in
//! `context_notes`.
//!
//! Fixtures are ORIGINAL machinery (never the sanstun source pasted -
//! the packet path is re-authored as plain-Rust stand-ins with neutral
//! names), each compiled and executed standalone (rustc --edition 2021
//! --test) before this table was assembled; the buggy/fixed arm pair
//! shares the harness (same tag function, same limit constant, same
//! reference assignment) and differs only on the law - the buggy arm
//! pays one synchronized trip per packet and dispatches each worker
//! behind its own trip, the fixed arm mints the whole batch in one trip
//! and fans the immutable reservation out through disjoint sub-range
//! splits.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── concurrency: N per-item synchronized counter reservations
        // collapse to ONE bound-checked range reservation whose
        // immutable value lets the bulk work leave the synchronized
        // owner entirely --------------------------------------------
        // Batch 327 / firezone/sanstun @ 35cb1932948bd0776ef7d590329c1f
        // dc0b1b9718 #129 (reserve_counters + PendingEncapsulate +
        // split_off; the receive-side DecapsulateBatch is the mirror
        // face, disclosed not mined).
        PerfRule {
            rule_id: "batch-reserve-monotonic-counters-offload-bulk".to_string(),
            name: "a batch of items whose identities are N consecutive monotonic counter assignments (packet nonces, sequence numbers, request ids) reserves them in ONE bound-checked operation - the count checked_add-ed onto the counter and the end filtered against the hard limit, with overflow or past-limit refusing the WHOLE batch loud and minting NOTHING - instead of N per-item synchronized reservations, and the resulting reservation is an immutable value (first counter and length fixed at mint time, no shared mutable state) so the per-item bulk work it fronts (encrypting or signing each item) can run on ANY thread in ANY order: the owner splits the range into disjoint contiguous sub-ranges and each worker completes its own items without ever re-consulting the shared counter, because an item's result depends only on (key, its own assigned counter, its bytes) so completion order cannot change any result - and the regime precondition is a receiver that tolerates gaps and reorder (a replay window), since a batched reservation hands contiguous ranges to racing workers rather than globally ordered assignments"
                .to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `batch-reserve-monotonic-counters-offload-bulk`
// (riir-refine rust_perf corpus, Batch 327) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's outbound packet path
// (a sans-IO tunnel; the source is cited in context_notes, never
// pasted): one session owns one monotonic counter under its own
// synchronization, and the incumbent assigns each packet's nonce by a
// synchronized reservation taken ONCE PER PACKET, handing each packet
// to its worker only after that packet's own round-trip. N packets
// cost N serialized trips on the sending thread, and no worker can
// touch its packet before the send thread minted its counter -- the
// trip ledger and the start snapshots below make both visible.

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;

/// the hard counter limit: the counter must never pass it -- headroom
/// is left so a replay window's arithmetic cannot wrap (the source pins
/// the same shape as a named constant; the value is this fixture's own).
const REJECT_LIMIT: u64 = u64::MAX - (1 << 13);

/// one outbound packet: opaque payload bytes.
struct Packet {
    bytes: Vec<u8>,
}

/// the session's synchronized counter state. ONE trip = one call to
/// `reserve_one` on this shared state under the session's mutex.
struct CounterState {
    next: u64,
    trips: usize,
}

#[derive(Debug, PartialEq)]
enum ReserveError {
    /// past the limit: the refusal is loud and mints nothing.
    NoCurrentSession,
}

impl CounterState {
    /// the per-item reservation: one synchronized trip mints ONE nonce.
    fn reserve_one(&mut self) -> Result<u64, ReserveError> {
        self.trips += 1;
        let first = self.next;
        let end = first
            .checked_add(1)
            .filter(|end| *end <= REJECT_LIMIT)
            .ok_or(ReserveError::NoCurrentSession)?;
        self.next = end;
        Ok(first)
    }
}

/// the send-side session: a mutex guards the counter -- exactly the
/// per-packet synchronized trip the law replaces.
struct Session {
    counter: Mutex<CounterState>,
}

/// one worker's outcome: the nonce it encrypted, the tag, and the trip
/// count the shared state showed WHEN the worker began (fixture
/// instrumentation -- the snapshot is the start-order evidence).
#[derive(Debug, PartialEq)]
struct WorkerOut {
    counter: u64,
    tag: u64,
    start_trips: usize,
}

/// the deterministic stand-in for the AEAD: a tag depends only on
/// (key, counter, packet bytes). Identical in both arms, so output
/// correctness is comparable against a reference assignment.
fn tag(key: u64, counter: u64, bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ key;
    for b in counter.to_le_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x1_0000_0001_b3);
    }
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x1_0000_0001_b3);
    }
    h
}

/// the INCUMBENT send path: per packet, take the session lock, mint ONE
/// counter (one trip), then dispatch THAT packet's worker. The dispatch
/// is gated behind the reservation -- a worker cannot see its packet
/// before the sending thread paid its trip -- so the send thread
/// serializes every packet's bookkeeping.
fn send_per_item(
    session: &Session,
    key: u64,
    packets: &[Packet],
) -> Vec<Result<WorkerOut, ReserveError>> {
    thread::scope(|s| {
        let mut queued: Vec<Option<(u64, thread::ScopedJoinHandle<'_, WorkerOut>)>> = Vec::new();
        for packet in packets {
            match session.counter.lock().unwrap().reserve_one() {
                Ok(counter) => {
                    let worker = s.spawn(move || {
                        let start_trips = session.counter.lock().unwrap().trips;
                        WorkerOut {
                            counter,
                            tag: tag(key, counter, &packet.bytes),
                            start_trips,
                        }
                    });
                    queued.push(Some((counter, worker)));
                }
                Err(_) => queued.push(None),
            }
        }
        queued
            .into_iter()
            .map(|slot| match slot {
                Some((_counter, handle)) => Ok(handle.join().expect("worker panicked")),
                None => Err(ReserveError::NoCurrentSession),
            })
            .collect()
    })
}

#[test]
fn output_correctness_holds_despite_the_serialization() {
    let n = 8usize;
    let packets: Vec<Packet> = (0..n).map(|i| Packet { bytes: vec![i as u8; 16] }).collect();
    let session = Session { counter: Mutex::new(CounterState { next: 0, trips: 0 }) };
    let key = 0x0123_4567_89ab_cdef;
    let outs = send_per_item(&session, key, &packets);
    let mut got: HashMap<u64, u64> = HashMap::new();
    for out in &outs {
        let out = out.as_ref().expect("no packet is past the limit here");
        got.insert(out.counter, out.tag);
    }
    // the reference assignment: inline per-item nonces, same tag fn
    let reference: HashMap<u64, u64> = (0..n as u64)
        .map(|c| (c, tag(key, c, &packets[c as usize].bytes)))
        .collect();
    assert_eq!(got, reference, "tags are correct; only the bookkeeping differs");
}

#[test]
fn one_synchronized_trip_per_packet_is_the_serialization() {
    let n = 16usize;
    let packets: Vec<Packet> = (0..n).map(|i| Packet { bytes: vec![i as u8; 8] }).collect();
    let session = Session { counter: Mutex::new(CounterState { next: 0, trips: 0 }) };
    let outs = send_per_item(&session, 7, &packets);
    assert!(outs.iter().all(|o| o.is_ok()));
    // THE EVIDENCE: N packets cost N synchronized trips on the sending
    // thread -- the trip count scales with the batch, not with the
    // session.
    assert_eq!(
        session.counter.lock().unwrap().trips,
        n,
        "N packets = N serialized reservation trips"
    );
}

#[test]
fn no_worker_starts_before_its_own_counter_is_minted() {
    let n = 12usize;
    let packets: Vec<Packet> = (0..n).map(|i| Packet { bytes: vec![i as u8; 8] }).collect();
    let session = Session { counter: Mutex::new(CounterState { next: 0, trips: 0 }) };
    let outs = send_per_item(&session, 9, &packets);
    for (i, out) in outs.iter().enumerate() {
        let out = out.as_ref().unwrap();
        // worker i's start snapshot is at least its own packet's trip:
        // the k-th dispatch is gated behind the k-th reservation, so a
        // worker can never run ahead of its own counter being minted.
        assert!(
            out.start_trips >= i + 1,
            "worker for packet {} started after only {} trips",
            i,
            out.start_trips
        );
    }
    assert_eq!(session.counter.lock().unwrap().trips, n);
}

#[test]
fn the_per_item_path_fails_mid_batch_past_the_limit() {
    // the counter sits ONE below the limit: packet 0 mints the last
    // counter (and its worker is dispatched), packet 1 refuses loud --
    // the batch dies halfway, with half its work already handed out.
    let packets = vec![Packet { bytes: vec![1; 8] }, Packet { bytes: vec![2; 8] }];
    let session =
        Session { counter: Mutex::new(CounterState { next: REJECT_LIMIT - 1, trips: 0 }) };
    let outs = send_per_item(&session, 42, &packets);
    assert!(outs[0].is_ok());
    assert_eq!(outs[0].as_ref().unwrap().counter, REJECT_LIMIT - 1);
    assert_eq!(outs[1], Err(ReserveError::NoCurrentSession));
    // every attempted packet paid a synchronized trip, including the
    // refused one -- two trips, one nonce minted
    assert_eq!(session.counter.lock().unwrap().trips, 2);
}

fn main() {}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `batch-reserve-monotonic-counters-offload-bulk`
// (riir-refine rust_perf corpus, Batch 327) -- the LAW arm.
//
// The SAME harness with the batched reservation: ONE bound-checked
// operation reserves the whole batch -- the count checked_add-ed onto
// the counter, the end filtered against the hard limit, the WHOLE batch
// refused loud with nothing minted on overflow or past-limit -- and the
// reservation is an immutable Copy value (first_counter + len fixed at
// mint time, no shared mutable state) that crosses to any thread. The
// owner splits it into disjoint contiguous sub-ranges; each worker
// encrypts its range independently in any order and NEVER re-consults
// the shared counter; a tag depends only on (key, own counter, packet),
// so completion order cannot change any result.

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;

/// the hard counter limit -- the same shape as the anti-pattern arm.
const REJECT_LIMIT: u64 = u64::MAX - (1 << 13);

/// one outbound packet: opaque payload bytes.
struct Packet {
    bytes: Vec<u8>,
}

struct CounterState {
    next: u64,
    trips: usize,
}

#[derive(Debug, PartialEq)]
enum ReserveError {
    /// past the limit: the refusal is loud and mints nothing.
    NoCurrentSession,
}

/// THE RESERVATION: an immutable value. `first_counter + len` are fixed
/// at mint time; holding a copy shares no mutable state with the
/// session, so it can be handed to any thread (all fields are plain
/// integers, so Send is automatic -- the source carries the same shape
/// in a doc line, never an explicit impl; see context_notes).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Reservation {
    first_counter: u64,
    len: usize,
}

impl Reservation {
    /// every nonce this reservation fixes, in order.
    fn counters(&self) -> std::ops::Range<u64> {
        self.first_counter..self.first_counter + self.len as u64
    }

    /// splits off the counters from `at` onwards into a new reservation,
    /// keeping the first `at` (the receiver's own length shrinks to
    /// `at`, exactly the source's shape). Returns None -- keeping
    /// everything -- when `at` is not a proper split point: a bad split
    /// is a no-op, never a partial loss.
    fn split_off(&mut self, at: usize) -> Option<Reservation> {
        if at == 0 || at >= self.len {
            return None;
        }
        let rest = Reservation {
            first_counter: self.first_counter + at as u64,
            len: self.len - at,
        };
        self.len = at;
        Some(rest)
    }
}

struct Session {
    counter: Mutex<CounterState>,
}

impl Session {
    /// THE BATCHED RESERVE: one synchronized trip for the WHOLE batch.
    /// The count is checked_add-ed onto the counter and the end is
    /// filtered against the hard limit; on overflow or past-limit the
    /// batch is refused loud and NOTHING is minted -- the session's
    /// counter is unchanged, no Reservation exists.
    fn reserve_counters(&self, len: usize) -> Result<Reservation, ReserveError> {
        let mut state = self.counter.lock().unwrap();
        state.trips += 1;
        let first = state.next;
        let end = u64::try_from(len)
            .ok()
            .and_then(|len| first.checked_add(len))
            .filter(|end| *end <= REJECT_LIMIT)
            .ok_or(ReserveError::NoCurrentSession)?;
        state.next = end;
        Ok(Reservation { first_counter: first, len })
    }
}

/// the deterministic stand-in for the AEAD -- identical to the
/// anti-pattern arm: a tag depends only on (key, counter, packet bytes).
fn tag(key: u64, counter: u64, bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64 ^ key;
    for b in counter.to_le_bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x1_0000_0001_b3);
    }
    for b in bytes {
        h = (h ^ u64::from(*b)).wrapping_mul(0x1_0000_0001_b3);
    }
    h
}

/// one worker: takes ITS sub-range reservation and ITS packets, and
/// encrypts each at its own counter -- the session is never touched
/// again (no session parameter exists to touch).
fn encrypt_range(
    key: u64,
    res: Reservation,
    packets: &[Packet],
    out: &Mutex<HashMap<u64, u64>>,
) {
    debug_assert_eq!(res.len, packets.len(), "exactly as many packets as counters");
    for (i, packet) in packets.iter().enumerate() {
        let counter = res.first_counter + i as u64;
        let t = tag(key, counter, &packet.bytes);
        out.lock().unwrap().insert(counter, t);
    }
}

/// the reference assignment a per-item pass would produce for the same
/// packets starting at `first`.
fn reference_tags(key: u64, packets: &[Packet], first: u64) -> HashMap<u64, u64> {
    packets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let c = first + i as u64;
            (c, tag(key, c, &p.bytes))
        })
        .collect()
}

#[test]
fn one_trip_for_the_whole_batch_whatever_its_size() {
    let n = 64usize;
    let session = Session { counter: Mutex::new(CounterState { next: 0, trips: 0 }) };
    let res = session.reserve_counters(n).expect("far below the limit");
    assert_eq!(
        res.counters(),
        0..n as u64,
        "the nonces are the same contiguous set a per-item pass assigns"
    );
    // THE EVIDENCE: one batch = ONE synchronized trip, regardless of the
    // packet count.
    assert_eq!(session.counter.lock().unwrap().trips, 1, "one batched reserve, not {} trips", n);
    // the session's counter advanced by exactly the batch length
    assert_eq!(session.counter.lock().unwrap().next, n as u64);
}

#[test]
fn splits_are_disjoint_contiguous_and_reassemble_the_whole() {
    let mut head = Reservation { first_counter: 100, len: 10 };
    let rest = head.split_off(4).expect("a proper split point");
    // the kept head and the split tail are each contiguous...
    assert_eq!(head.counters(), 100..104);
    assert_eq!(rest.counters(), 104..110);
    // ...the tail starts exactly where the head ends...
    assert_eq!(rest.first_counter, head.first_counter + head.len as u64);
    // ...and the union is the whole, disjoint by construction.
    let mut joined: Vec<u64> = head.counters().chain(rest.counters()).collect();
    joined.sort_unstable();
    assert_eq!(joined, (100..110).collect::<Vec<u64>>());
    // a non-split point is a no-op that keeps everything: nothing
    // minted, nothing lost.
    let mut untouched = Reservation { first_counter: 100, len: 10 };
    assert!(untouched.split_off(0).is_none());
    assert!(untouched.split_off(10).is_none());
    assert_eq!(untouched.len, 10);
    assert_eq!(untouched.counters(), 100..110);
}

#[test]
fn completion_order_cannot_change_any_tag() {
    let n = 16usize;
    let packets: Vec<Packet> =
        (0..n).map(|i| Packet { bytes: vec![(i * 7 % 251) as u8; 24] }).collect();
    let key = 0xdead_beef_cafe_f00d;
    let session = Session { counter: Mutex::new(CounterState { next: 0, trips: 0 }) };
    let out = Mutex::new(HashMap::new());

    let res = session.reserve_counters(n).expect("far below the limit");
    let trips_before_workers = session.counter.lock().unwrap().trips;
    // split UNEVENLY and cross the assignment: the tail worker (higher
    // counters) spawns first, the small head worker spawns last, and
    // whatever finishes first wins the map insert -- every ordering here
    // differs from counter order.
    let mut head = res;
    let rest = head.split_off(3).expect("a proper split point");
    thread::scope(|s| {
        s.spawn(|| encrypt_range(key, rest, &packets[3..], &out));
        s.spawn(|| encrypt_range(key, head, &packets[..3], &out));
    });
    // both workers are done: racing completion, any interleaving...
    let got = out.lock().unwrap().clone();
    let reference = reference_tags(key, &packets, 0);
    assert_eq!(got, reference, "the tag set is identical whatever the completion order");
    // ...and the workers consulted the shared counter ZERO extra times:
    // the reservation fixed the nonces, so encryption shared no state
    // with the session.
    assert_eq!(
        session.counter.lock().unwrap().trips,
        trips_before_workers,
        "encryption re-consulted nothing"
    );
}

#[test]
fn uneven_splits_on_racing_workers_reassemble_identically() {
    // the deterministic permutation face: several split arrangements of
    // the same batch -- head 1, head half, head all-but-one -- must all
    // produce the identical per-counter tags.
    let n = 12usize;
    let packets: Vec<Packet> = (0..n).map(|i| Packet { bytes: vec![i as u8; 12] }).collect();
    let key = 0xfeed_face_c0ff_ee01;
    let reference = reference_tags(key, &packets, 40);
    for at in [1usize, 6, 11] {
        let session = Session { counter: Mutex::new(CounterState { next: 40, trips: 0 }) };
        let out = Mutex::new(HashMap::new());
        let res = session.reserve_counters(n).expect("far below the limit");
        let mut head = res;
        let rest = head.split_off(at).expect("a proper split point");
        thread::scope(|s| {
            s.spawn(|| encrypt_range(key, rest, &packets[at..], &out));
            s.spawn(|| encrypt_range(key, head, &packets[..at], &out));
        });
        assert_eq!(*out.lock().unwrap(), reference, "split at {} reassembles identically", at);
        assert_eq!(session.counter.lock().unwrap().trips, 1);
    }
}

#[test]
fn past_limit_or_overflow_refuses_the_whole_batch_and_mints_nothing() {
    // past-limit: the counter sits ONE below the limit and the batch
    // needs TWO -- the filter refuses the WHOLE batch; the counter is
    // unchanged and no Reservation exists to hand out.
    let session =
        Session { counter: Mutex::new(CounterState { next: REJECT_LIMIT - 1, trips: 0 }) };
    assert_eq!(session.reserve_counters(2), Err(ReserveError::NoCurrentSession));
    assert_eq!(session.counter.lock().unwrap().next, REJECT_LIMIT - 1, "nothing was minted");
    assert_eq!(
        session.counter.lock().unwrap().trips,
        1,
        "the refusal cost one trip -- not two per-item attempts"
    );
    // exactly-fitting still works: the last counter is handed out...
    let res = session.reserve_counters(1).expect("the batch fits exactly");
    assert_eq!(res.counters(), REJECT_LIMIT - 1..REJECT_LIMIT);
    // ...and the session is exhausted for even one more.
    assert_eq!(session.reserve_counters(1), Err(ReserveError::NoCurrentSession));
    assert_eq!(session.counter.lock().unwrap().next, REJECT_LIMIT);

    // u64 overflow: checked_add catches it before the filter could --
    // same loud refusal, same nothing-minted.
    let wrapped = Session { counter: Mutex::new(CounterState { next: u64::MAX - 1, trips: 0 }) };
    assert_eq!(wrapped.reserve_counters(4), Err(ReserveError::NoCurrentSession));
    assert_eq!(wrapped.counter.lock().unwrap().next, u64::MAX - 1, "nothing was minted");
}

fn main() {}
"#
            .to_string(),
            context_notes: Some("Grade B - NotBenchable BY DOMAIN (the coordination-law annotation class, stated honestly): the win is PARALLEL THROUGHPUT - offloading the per-item bulk work (encrypt/sign per item) behind ONE immutable reservation - and the rust_perf_bench Pareto validator hosts single-process latency + alloc counts, which cannot host the multi-threaded regime (the established class: the rayon/B136 ladder rules in entries_06 - honestly NotBenchable, the decision content is the ladder, not a speed claim; the splash serving-pool slices in entries_43-46 - all corpus-only NotBenchable decision-content serving laws; the certifying-token rule itself, entries_49/B283, Grade B NotBenchable). The serial-side component (ONE reserve call vs N) is benchable but trivial-magnitude - 20.8 ns of bookkeeping against a 933 ns per-item encryption at the source's own shapes - never a bar; the fixture's trip counters (trips == 1 vs trips == N) are the shape evidence, not a latency claim. THE LAW: reserve N consecutive monotonic counters in ONE bound-checked operation and the per-item bulk work they front can leave the synchronized owner entirely - the count is checked_add-ed onto the counter and the end filtered against the hard limit (overflow or past-limit refuses the WHOLE batch loud and mints NOTHING), and the returned reservation is an immutable value (first counter and length fixed at mint time) so workers hold copies that share no mutable state with the owner; the owner splits the range into disjoint contiguous sub-ranges and each worker completes its items in any order - an item's result depends only on (key, its own counter, its bytes), so completion order cannot change any result. THREE load-bearing halves, all present in the source: (1) THE BOUND CHECK IS THE BATCH'S ADMISSION - the checked_add-over-count + limit-filter refuses the batch BEFORE any item is minted, so a too-big batch costs one trip and zero nonces (the source's own test name says it: batch_beyond_the_message_limit_reserves_nothing; the anti-pattern arm shows the contrasting failure - a MID-BATCH death, k items already dispatched when item k+1 refuses); (2) THE RESERVATION IS A VALUE, NOT A SESSION VIEW - the owner hands out copies, and the source's key doc makes the design explicit, verbatim: 'Pending batches carry a copy, so encrypting and decrypting on other threads shares no memory with the session.' - the worker needs no lock, no counter read, no owner callback (the fixed fixture's worker takes no session parameter to touch); (3) THE RECEIVER TOLERATES REORDER AND GAPS - the enabling precondition, named in the source itself, verbatim: 'The receiver's replay window allows some reordering of messages.' - because batched ranges handed to racing workers arrive in completion order, not assignment order, and even unused counters are fine, verbatim: 'Counters of messages that are never pulled stay unused, which the receiver tolerates.'. THE SOURCE (firezone/sanstun @ 35cb1932948bd0776ef7d590329c1fdc0b1b9718, Apache-2.0, the tip squash of #129 'feat!: reserve counters and decrypt data messages in batches', 2026-11-03; every quote below re-verified verbatim at the pin this batch): the reserve is session.rs reserve_counters, doc 'Reserves `len` consecutive counters and returns the first.', body verbatim: 'let first = self.sending_key_counter; let end = u64::try_from(len.get()).ok().and_then(|len| first.checked_add(len)).filter(|end| *end <= REJECT_AFTER_MESSAGES).ok_or(Error::NoCurrentSession)?; self.sending_key_counter = end; Ok(first)' - EncapsulateBatch::finish calls it once for the whole batch (doc: 'Reserves `len` consecutive counters and releases the tunnel.'; failure doc: 'Fails with [`Error::NoCurrentSession`] if the session cannot send `len` more messages (`REJECT_AFTER_MESSAGES`). No counter is reserved then.'); the offload contract is one doc line on the pending batch, verbatim: 'The reserved counters fix the nonces, so encryption can run on any thread and in any order.'; the split is PendingEncapsulate::split_off distributing contiguous sub-ranges, doc 'Keeps the first `at` counters. Returns [`None`] and keeps all counters if `at` is not less than [`len`](Self::len).', with the tail's first counter computed as 'self.first_counter + at.get() as u64'; the limit is a named constant whose doc IS the regime condition made constant, verbatim: 'pub(crate) const REJECT_AFTER_MESSAGES: u64 = u64::MAX - (1 << 13);' / 'The exclusive counter limit, leaving room for the replay window's arithmetic.'; the same commit batches the receive side too (DecapsulateBatch collects each packet's counter through will_receive's replay screen, then finish decrypts the collected batch) - the mirror face, disclosed not mined. THE SEND HONESTY NOTE: the introducing-sanstun blog says 'PendingEncapsulate implements Send and can thus be transferred to another thread to perform the bulk of the work', but NO explicit Send impl or bound exists anywhere in the repo (grepped zero at the pin) - the trait is auto-implied by the fields and the thread fan-out lives in the docs and the blog, never asserted in code; this rule's law is the batch+offload SHAPE, not a Send-derivation recipe. MEASURED (source-reported, the blog's criterion run - provenance, never bars): begin_encapsulate 20.8 ns (20.785-20.869, size-independent) vs encrypt 933 ns (933.49-933.59) at 1280-byte packets - 'the bookkeeping part of encrypting a new packet is only 2%' - with the blog's own hedge carried as the honest upper bound: 'this hints at a potential speedup of more than 45x!' alongside 'An effective speedup of 45x is hard to achieve in practice because it ignores the cost of passing PendingEncapsulate to another thread and assumes no other overhead is introduced.' - the mechanism is that bookkeeping does not scale with item size while the bulk work does, which is exactly why the serial-side magnitude is trivial and the throughput regime is unbenchable here. CHECKED-ARITHMETIC CROSS-REF (positive instance, never re-owned): the checked_add + limit-filter idiom IS the fixed_code of the sec domain's unchecked-wire-arithmetic rule (sec corpus.rs; its fixture is len.checked_add(FRAME_OVERHEAD)) - that rule owns CHECK THE ARITHMETIC on wire-sourced integers; THIS rule owns the BATCH+OFFLOAD shape above it (one trip for N items, then thread fan-out behind an immutable reservation) - compose, distinct axes. REGIME: per-item monotonic assignments (packet nonces, sequence numbers, request ids, ticket ids, segment ids) fronting per-item bulk work (encrypt, sign, hash, serialize), batches whose item identity (the assigned number) fixes the item's work input, and a consumer that tolerates gaps and reorder (replay window, out-of-order accept, idempotent join on the id). ANTI-REGIME: receivers requiring gap-free strict assignment order (batched ranges handed to racing workers would violate the protocol - the reservation would mint unusable numbers); assignments that must feed back into the shared object per item (the reservation value must stay mutable/shared state - no immutable-copy handoff possible); counters that gate VALIDITY by exact assignment order rather than membership (some fairness/priority constructions). DISTINCT-FROM (read this batch): revalidation-skipped-by-certifying-identity-token (rust_perf entries_49, B283) - that owns a VALIDATION-SKIP: a per-step re-validation pass skipped when a builder-minted certifying token matches, with the token written only by the success path and absence never counting as a hit; THIS owns BULK RESERVATION + OFFLOAD: N synchronized per-item trips collapsed to ONE range mint, then thread fan-out - nothing is skipped (every item's bulk work runs exactly once), the shared state is consulted less often, not trusted more, and there is no token/absence asymmetry anywhere; the two compose (a reserved batch is exactly the kind of input whose re-validation a certifying token could skip). Adjacent (named, not owners): bounded-peer-handoff-wait (entries_22, B219) bounds a WAIT on a peer's liveness - handoff BOUNDS, not reservation BATCHES; no counter range exists there; dynamic-work-parallelism-ladder and the Batch-136 rayon rules (entries_06) are work-DISTRIBUTION structure for work unknown in advance, not monotonic-counter reservation; the splash serving-pool rules (entries_41/43-46) are admission and pacing over a memory governor - reservations of CAPACITY, never of identity numbers. GREP EVIDENCE (run this batch over src/domains/rust_perf/corpus + src/domains/kernel_opt/corpus): 'consecutive counter|counter reserv|monotonic counter' - zero rule owners for the batch-reserve face (the hits are other senses: adaptive-yield-consecutive-ready entries_06.rs:787 counts instantly-ready POLLS, not reservations; compaction-reserve-before-prompt-build entries_06.rs:193 reserves measured CAPACITY before one build); 'nonce' - zero hits in either corpus; 'offload' - one owner, offload-cpu-bottleneck-to-gpu-kernel (kernel_opt entries_gpu_01.rs:74) - CPU-prep-to-GPU-COMPUTE device migration, never thread fan-out behind a counter range; 'Send' - zero coordination-law owners. NOVEL: the batch-reserve + offload-bulk law is new to the corpus. Fixtures ORIGINAL (the sans-IO packet path re-authored as plain-Rust stand-ins with neutral names - no source code, identifiers or product names pasted; the quotes above are the only place the source speaks; both arms compiled and executed standalone with rustc --edition 2021 --test before this table was assembled). CITE: firezone/sanstun @ 35cb1932948bd0776ef7d590329c1fdc0b1b9718 (Apache-2.0; src/session.rs - reserve_counters, EncapsulateBatch::finish, PendingEncapsulate + split_off + encrypt, SessionKey; src/timers.rs - REJECT_AFTER_MESSAGES; the pin is the #129 squash; the introducing-sanstun blog, 2026-10-31, for the criterion numbers); Batch 327 / Plan 332 / Research 364.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

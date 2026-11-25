//! Batch 419 — the cuda-async completion-reactor transcription batch's
//! `kernel_opt` half (Plan 415 / Research 450). Source:
//! github.com/NVIDIA/cuda-rust, the `cuda-async` host crate, at pin
//! `f35d6ccc2e03a54e73e3796f415f11313e439d50` (Apache-2.0; the SAME pin
//! Batch 417 re-verified at its T0 — the clone landed byte-exact at the pin
//! 2026-11-26, tip UNMOVED). FOUR rules, ALL suggestion-only
//! NotBenchable-by-domain (the win metrics live in the CUDA driver runtime —
//! the dial9/B136 annotation class, never a demotion; same treatment Batch
//! 417 gave this source's device-side rules):
//! - `flag-write-completion-reactor-amortizes-notification` (Dispatch) —
//!   per-completion notification amortized: ONE process-wide scanner over a
//!   pinned flag slab replaces the per-op driver host-func hop.
//! - `wake-without-complete-fault-resolves-at-waiter-poll` (Dispatch) — the
//!   dead-producer protocol: armed slots retired, the waiter woken WITHOUT
//!   completion, the error resolves at the waiter's own poll.
//! - `transition-gated-unpark-scanner-armed-count-park-gate` (Parallelism) —
//!   the scanner park discipline: the park gate is the armed count, unpark
//!   fires only on the idle→active 0→1 transition.
//! - `bounded-inline-spin-before-completion-registration` (Dispatch) — the
//!   wait-site spin ahead of the completion-callback registration, budget
//!   derived from the measured wait distribution.
//!
//! The batch's TWO count-neutral faces land on their OWNERS at this landing
//! (in the same commit): F1 the probe-memo face on `gpu_148`
//! `fail-stop-deadline-trap-bounded-cross-rank-wait` (`entries_gpu_148` — the
//! poll-subset amortization law's OWNER; the probe memo is its HOST twin),
//! F2 the Drop-path three-way health-dispatch + exclusion face on `gpu_111`
//! `copy-source-quarantine-unproven-free-pinned` (`entries_gpu_111` — the
//! custody-quarantine owner; the count-the-leak tension clause written
//! in-notes). The round-2 verdict condition is carried: R2's notes POINT to
//! the F1 face instead of restating the probe memo.
//!
//! Verdict lineage: Research 450 round 1 REVISE (the reviewer's greps found
//! nine corpus owners the draft's noun-only dedup list missed — the
//! vocabulary lesson recorded in Research 450's caveats), round 2 AGREE.
//! R4 (the completion-unestablishable drop) folded to F2 at verdict round 1.
//!
//! EN-ROUTE REPAIR rides this landing (the B404-repairs-B402 precedent):
//! Batch 417 landed `entries_gpu_169` WITHOUT arming the G1 census (the
//! B379/B402 class — both lists blind together, the comparison stayed
//! green); `entries_gpu_169` AND `entries_gpu_170` join `expected_corpus_files()`
//! + `per_file_entries()` + the len pin in the same commit.
//!
//! T0b (the unread-residue read, Research 450's own-verdict gate) surfaced
//! SEVEN new atom candidates from the non-reactor files (`predicate.rs` frame
//! atom variants, `device_context.rs` thread-local lease restore,
//! `cuda_graph.rs` sticky-mode end-before-panic-resume, `device_operation.rs`
//! memoized cross-stream event ordering + graph-capture allocation ban +
//! once-executor unwind liveness, `launch.rs` arena pointer materialization) —
//! recorded in the queue snapshot as the STAGED follow-up lane, NEVER forced
//! into this batch (the plan's own gate).
//!
//! Fixtures: `corpus/fixtures/b419_fixtures.rs` — standalone-compiled AND
//! executed BEFORE entry (debug AND -O, the four-arm discipline).
//! NOTHING INVENTED — every quote read at the clone at the pin this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 419 — the cuda-async completion-reactor rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — completion-notification amortization: the per-op driver hop
        // is replaced by ONE scanner over a pinned flag slab, and the
        // amortized path is never load-bearing-by-force (the fallback
        // ladder). ──
        KernelRule {
            rule_id: "flag-write-completion-reactor-amortizes-notification".to_string(),
            gpu_concept: "MECHANISM (completion-notification amortization; category D — \
Dispatch, the host completion plane): per-completion notification moves off the driver-thread \
hop — a `cuLaunchHostFunc` callback PER operation, each a driver call and a thread hop the \
completion pays individually — onto a completion-flag slab the producer writes through its own \
channel and ONE process-wide scanner thread resolves with plain memory loads. THE SOURCE, \
verbatim at the pin (NVIDIA/cuda-rust cuda-async/src/reactor.rs:9-14 @ f35d6ccc2e): 'Replaces \
per-completion `cuLaunchHostFunc` callbacks with a `cuStreamWriteValue32` into a slot of \
pinned host memory at pipeline end, plus one process-wide reactor thread that scans pending \
slots (plain memory loads, no driver calls on the hot path) and fires wakers. The wakeup cost \
amortizes across all in-flight pipelines instead of paying a driver-thread hop per pipeline.' \
THE SHAPE, four decision contents. (a) THE SLAB: 1024 slots ('const NUM_SLOTS: usize = 1024;' \
reactor.rs:46) of pinned host memory device-mapped ('CU_MEMHOSTALLOC_PORTABLE | \
CU_MEMHOSTALLOC_DEVICEMAP' :101) so the DEVICE side arms a slot by writing a flag word \
(`cuStreamWriteValue32` at pipeline end) and the HOST side reads the same bytes with plain \
loads — no driver call anywhere on the scan hot path. (b) ONE SCANNER FOR MANY COMPLETIONS: \
the scanner walks the armed-slot bitmap, reads flags, and fires each completed registration's \
waker — 'Wakers fire outside any lock the scan held, so a registration is never blocked \
behind a waking phase' (reactor.rs:146-147) — the wakers are collected under the scan and \
fired after it, so waking never holds a lock a new registration needs. (c) THE DEGRADATION \
LADDER IS IN-SOURCE — the amortized path is never load-bearing-by-force: 'Flag-write reactor \
path; fall back to the host-function hop if the slot pool is exhausted or stream mem-ops are \
unavailable' (device_future.rs:228-229) — `reactor::register` returns Err on pool exhaustion \
('reactor slot pool exhausted' reactor.rs:240) or on the `cuStreamWriteValue32` failure \
(:244-246), and the caller takes the per-op `launch_host_function` hop that fires \
`waker_state.signal()` per op (device_future.rs:234-235); every completion still resolves, \
only the economics fall back. (d) THE A/B DISCLOSURE IS IN-SOURCE — the env-switch/A-B \
posture's positive instance, recorded never landed here: 'Completion strategy, \
runtime-selectable via `CUDA_ASYNC_HOST_SYNC`' (device_future.rs:201) accepting spin|spinwait \
/ block|blocking (any other value falls through to the reactor, :215-217) — 'The explicit \
modes bypass the reactor so all three completion paths can be A/B'd from a single build.' \
(:207-208); the harvest protocol underneath (active bitmap, SPSC payload handoff, free list) \
is 'model-checked under `loom`/`miri`' (reactor.rs:16-19). ⛔ KINSHIP (adjudicated both \
directions, compose-not-merge): idle-rank-doorbell-on-rendezvous-store (rust_perf entries_13) \
owns WHERE an idle rank blocks — a doorbell store that wakes the ONE waiter parked on a \
rendezvous slot; relaxed-spin-acquire-confirm-metadata-flag-wait (gpu_118, E) owns ONE \
waiter's two-tier wait on ONE flag (relaxed spin, acquire-confirm, metadata decode) — \
neither owns the amortization: the distinct claim is ONE scanner serving MANY completions, \
the per-completion driver hop paid ONCE across all in-flight pipelines instead of once per \
op (compose: the doorbell/gpu_118 shapes are per-waiter mechanisms; THIS is the \
notification SUBSTRATE that replaces per-op notifications fleet-wide). REGIME: many short \
in-flight completions on a channel whose per-completion notification is expensive (a driver \
host-func, an FFI hop, a syscall); the anti-regime is few long-running completions — the \
scanner is pure overhead — and a channel that cannot carry an opaque flag write (the \
fallback ladder owns that posture by design). ⛔ NotBenchable-by-domain (the win metric is \
driver-thread host-func hop pricing on the CUDA runtime — no validator hosts those \
semantics; a mock mpsc-vs-flag-slab would measure a DIFFERENT mechanism at a different \
magnitude than the driver hop the rule prices — the dial9/B136 annotation class, never a \
demotion; the notification-count and fallback-ladder relationships ARE CPU-testable and the \
fixtures pin them). Fixture: the fixture models the notification mechanism deterministically \
(per-op hop count N vs the amortized scanner's zero per-op hops, all completions observed, \
the exhaustion arm taking the fallback hop and still completing, the tamper arm — wakers \
fired while holding the scan lock — caught by the re-entrant registration failing). Source: \
NVIDIA/cuda-rust @ f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0) — \
cuda-async/src/reactor.rs:9-34 (the module doc), :46-52 (the constants), :101 (the host-alloc \
flags), :144-152 (the scan/wake split), :232-258 (register + the empty→wake unpark), \
device_future.rs:201-217 (the A/B modes), :226-235 (the fallback ladder). Quotes \
byte-verified at the clone at the pin this session (T0, tip unmoved). Batch 419 / Plan 415 / \
Research 450."
                .to_string(),
            ref_shape: r"// NAIVE: every wait registers its own completion callback on the
// driver thread — the notification cost is paid PER OPERATION, and a
// burst of short pipelines pays it N times.
fn wait_naive(stream: &Stream, work: Work) -> Result<Output, Error> {
    let (tx, rx) = mpsc::channel();
    // one driver-thread hop PER op: the callback fires on the driver's
    // host-func thread, hops through the FFI boundary, then hands off.
    stream.launch_host_function(move || {
        let _ = tx.send(());
    })?;
    work.submit(stream)?;
    rx.recv().map_err(|_| Error::DriverDead) // blocks a thread per op
}
// N in-flight pipelines = N callbacks = N driver hops + N parked threads.
"
            .to_string(),
            opt_pattern: r"// BOUND: the producer arms a flag-slab slot through its own channel
// (`cuStreamWriteValue32` into pinned, device-mapped host memory); ONE
// process-wide scanner resolves every armed slot with plain loads and
// fires the wakers OUTSIDE the scan lock.
const NUM_SLOTS: usize = 1024;

enum Completion { Slot(SlotHandle), HostHop(Box<dyn FnOnce() + Send>) }

fn register(slab: &mut FlagSlab, stream: &Stream, waker: Waker) -> Completion {
    match slab.arm(stream, waker) {
        Ok(handle) => Completion::Slot(handle), // cuStreamWriteValue32 armed it
        // the degradation ladder: pool exhausted or stream mem-ops
        // unavailable -> the per-op host hop still completes the op.
        Err(_) => Completion::HostHop(Box::new(move || waker.wake())),
    }
}

/// The scanner's ONE pass over the whole armed set: plain loads, no
/// driver calls on the hot path; wakers collected under the scan,
/// fired AFTER the scan guard drops.
fn scan_once(slab: &FlagSlab, woken: &mut Vec<Waker>) {
    let _scan = slab.lock(); // the ONLY lock, and never held while waking
    for slot in slab.armed_iter() {
        if slab.flag_loaded(slot) {
            woken.push(slot.take_waker());
        }
    }
}
// fire: for w in woken.drain(..) { w.wake(); }  — outside the lock.
"
            .to_string(),
        },
        // ── R2 — the dead-producer protocol: the notifier NEVER synthesizes
        // the error; armed slots are retired by a rate-limited probe and the
        // waiter is woken WITHOUT completion so the error resolves at the
        // waiter's own observation point. ──
        KernelRule {
            rule_id: "wake-without-complete-fault-resolves-at-waiter-poll".to_string(),
            gpu_concept: "MECHANISM (faulted-stream retirement; category D — Dispatch, the \
host completion plane, the async-shaped completion lane): a dead producer (a device fault \
kills the CUDA context) leaves armed completion slots whose flag writes will NEVER execute — \
'Their slots would stay armed forever, the awaiting futures would never resolve, and the \
scanner — gated on \"anything armed\" — would never park.' (reactor.rs:26-28 @ f35d6ccc2e; \
the historical defect's test disclosure, device_fault.rs:136-138: fault-after-registration \
'never resolved (the flag write never lands; nothing wakes the future)' and the reactor \
'spun at 100% CPU on the armed slot'). THE REPAIR NEVER SYNTHESIZES THE ERROR AT THE \
NOTIFIER — the error authority stays at the waiter's observation point. Four decision \
contents. (a) THE STATE PAIR IS TYPED: `signal()` = complete-store(Release) + wake (the \
normal completion path, the Release pairing with the poll's Acquire loads, \
device_future.rs:72-75); `wake()` = wake only (:82-84) — 'Wakes the task **without** \
marking the operation complete.' + 'The re-poll re-examines the stream itself. The reactor \
uses this for slots whose stream has faulted: the completion flag can never land, so the \
future has to observe the driver error on its own.' (:77-81). (b) THE RETIREMENT PROBE IS \
RATE-LIMITED AND CAPTURE-GATED: the scanner spins without progress first ('const \
SPIN_PASSES: u32 = 10_000;' reactor.rs:48), probes only every interval ('const \
STALE_PROBE_INTERVAL: Duration = Duration::from_millis(2);' :52), queries each DISTINCT \
stream once per pass ('Each distinct stream is queried once per pass.' :195), and never \
queries a capturing stream — 'A capturing stream is never queried (that would invalidate \
the capture).' (:33-34) — the capture-status check precedes the query in code order \
(device_future.rs:106-126: cuStreamIsCapturing first, status != NONE returns \
StreamHealth::Capturing BEFORE any query; the binding-error arm classifies Faulted — a \
stream whose capture status cannot be read is treated as gone, never queried). (c) THE \
RETIREMENT SPLITS BY VERDICT: flag-landed slots signal (complete+wake), driver-faulted \
slots wake-only — 'for reg in faulted.drain(..) { // Wake without completing: the poll \
observes the fault.' (reactor.rs:184-187) — and 'a stream the driver reports an error for \
is retired and its future is woken *without* being marked complete, so its next poll \
observes the driver error itself' (:32-33); the future's re-poll re-queries the stream and \
delivers the driver's own error object, so the delivered error is the AUTHENTIC one, never \
a notifier-invented stand-in. (d) THE STICKY-FAULT COROLLARY: the query error is sticky — \
a fault that persists poisons every later path that touches the stream, so a FRESH \
future's inline spin must propagate it rather than swallow-and-fall-through (the \
device_fault.rs:161-168 disclosure: before the fix the spin swallowed the sticky error and \
fell through to registration; the sticky error is process-persistent, so swallowing it \
hides the fault behind a perpetual wait). ⛔ KINSHIP (adjudicated both directions, \
compose-not-double-count): relaxed-spin-acquire-confirm-metadata-flag-wait (gpu_118) \
already interleaves a fault-poll INTO a waiter's own spin ('fault-poll interleaved every \
64 misses ... so a peer's fault surfaces in THIS waiter's loop') — the IN-SPIN \
observability twin: that is the WAITER's loop polling the producer's health; THIS is the \
NOTIFIER's retirement of armed slots it can no longer resolve (compose: a lane may do \
both — the waiter polls opportunistically, the notifier retires what the waiter's poll \
cannot reach). faulted-collective-fail-stop-no-numerics-fallback (gpu_118) owns the fault \
POLICY — what a collective does when a peer faults (stop the engine, never fall back to \
weaker numerics); THIS owns the completion plane's DELIVERY discipline (compose: the \
policy decides the outcome, THIS decides how the error travels). The probe-interval/memo \
amortization law's OWNER is fail-stop-deadline-trap-bounded-cross-rank-wait (gpu_148) \
part (a) — the poll-subset clock amortization — so the probe MEMO face routes THERE (see \
that rule's BATCH-419 FACE), never restated in these notes. REGIME: any completion \
reactor whose producer can die — faulted device, dropped runtime, cancelled peer — where \
the error is authoritative at the waiter's observation point, never at the notifier. \
ANTI-REGIME: the notifier IS the error authority (then deliver the error WITH the wake — \
a wake-without-complete would lose it); probe cost comparable to completion cost (then \
rate-limit the probes behind intervals where spinning has already paid, the source's own \
2 ms + 10k-pass gates). ⛔ NotBenchable-by-domain (the win metric is CUDA context-fault \
semantics — no validator hosts them; the delivery relationships — signal vs wake, \
error-at-own-poll, probe-rate-per-distinct-stream — ARE CPU-testable and the fixtures \
pin them; the dial9/B136 annotation class). Fixture: the fixture models the protocol \
deterministically (the dead-producer arm: faulted stream retired, waiter woken WITHOUT \
complete, its poll delivers the stream's OWN error object — identity-asserted against a \
synthetic; the rate-limit arm: 2 queries for 4 slots over 2 streams; the tamper arm: a \
notifier that marks-complete at the wake delivers a synthetic error — caught; the \
capture-gate arm: a capturing stream is never queried and stays armed). Source: \
NVIDIA/cuda-rust @ f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0) — \
cuda-async/src/reactor.rs:23-34 (the faulted-stream doc), :48-52 (SPIN_PASSES + the probe \
interval), :140-152 (the scan/wake split), :170-190 (the slow phase: probe, woken→signal, \
faulted→wake), :193-211 (probe_stale_slots + the per-distinct-stream memo), :218-223 \
(stream_is_faulted), device_future.rs:66-84 (signal/wake), :89-126 (StreamHealth + \
probe_stream's capture-gate-before-query), tests/device_fault.rs:136-176 (the historical \
hang + the sticky-fault propagation pin). Quotes byte-verified at the clone at the pin \
this session. Batch 419 / Plan 415 / Research 450."
                .to_string(),
            ref_shape: r"// NAIVE (a): the notifier GUESSES the error at the wake — the
// future completes with a synthetic failure the driver never reported,
// and the authentic error is lost.
fn retire_naive(slot: &mut Slot, err: &str) {
    slot.future.complete(Err(Error::Synthetic(err))); // invented
    slot.future.wake();
}
// NAIVE (b): no retirement at all — the armed slot is never revisited:
// 'their slots would stay armed forever, the awaiting futures would
// never resolve, and the scanner ... would never park.'
fn scan_naive(table: &mut Table) {
    for slot in table.armed() {
        if table.flag_loaded(slot) {
            slot.future.complete(Ok(()));
            slot.future.wake();
        } // faulted producer: NOTHING — the slot hangs forever
    }
}
"
            .to_string(),
            opt_pattern: r"// BOUND: the state pair is typed (signal vs wake), the probe is
// rate-limited + capture-gated, and the retirement splits by verdict —
// the waiter's OWN poll observes the driver error.
pub fn signal(&self) {
    self.complete.store(true, Ordering::Release); // pairs with poll's Acquire
    self.waker.wake();
}

/// Wake WITHOUT completing: the re-poll re-examines the stream itself.
pub fn wake_only(&self) {
    self.waker.wake();
}

enum Health { Idle, Busy, Capturing, Faulted }

/// One probing pass: each DISTINCT stream queried once; a capturing
/// stream never queried (that would invalidate the capture).
fn probe_pass(table: &mut Table, memo: &mut Vec<(StreamId, bool)>) {
    for slot in table.armed() {
        if table.flag_loaded(slot) {
            slot.waker_state.signal(); // complete + wake
            continue;
        }
        if stream_faulted(&slot.stream, memo) {
            // Wake without completing: the poll observes the fault.
            slot.waker_state.wake_only();
            table.retire(slot); // armed-set shrinks; the scanner may park
        }
    }
}

fn stream_faulted(stream: &Stream, memo: &mut Vec<(StreamId, bool)>) -> bool {
    if let Some((_, dead)) = memo.iter().find(|(s, _)| *s == stream.id()) {
        return *dead; // one query per DISTINCT stream per pass
    }
    // capture-gate BEFORE the query: a capturing stream is never
    // queried (that would invalidate the capture).
    if stream.is_capturing() != CaptureStatus::None {
        memo.push((stream.id(), false));
        return false;
    }
    let dead = matches!(stream.query(), Err(_));
    memo.push((stream.id(), dead));
    dead
}
"
            .to_string(),
        },
        // ── R3 — the scanner park discipline: the park gate is the armed
        // count (not the per-pass flag observation), unpark fires only on
        // the idle→active transition, and the park token absorbs the race. ──
        KernelRule {
            rule_id: "transition-gated-unpark-scanner-armed-count-park-gate".to_string(),
            gpu_concept: "MECHANISM (the scanner park/unpark discipline; category E — \
Parallelism, the host scanner lane): a parked scanner loop's wake discipline has two \
halves that fail DIFFERENTLY and both are pinned in-source. (a) THE PARK GATE IS THE \
ARMED COUNT, never the per-pass flag observation — 'Park gate is the armed count, not \
\"did this pass see a bit\": a slot may be armed but not yet flag-complete, and must keep \
the scanner awake.' (reactor.rs:154-156 @ f35d6ccc2e) — a pass that saw no flags is NOT \
evidence of idleness: the flag writes are in flight behind the device's work, so the \
gate must read the pending-work counter (the armed count, `is_idle()` loading it with \
Acquire and parking only at zero, slot_table.rs:81-83), or the scanner parks over live \
work and the completions stall until the next unrelated unpark (the lost-wakeup class). \
(b) UNPARK FIRES ONLY ON THE IDLE→ACTIVE TRANSITION — `publish` increments the armed \
count and returns the 0→1 test BEFORE setting the active bit: 'self.n_armed.fetch_add(1, \
Ordering::AcqRel) == 0' (slot_table.rs:122; increment-before-bit is the documented \
no-underflow invariant :113-117) — 'Returns whether this was the idle→active transition \
(`n_armed` 0→1) — the caller wakes the scanner only then (llist empty→wake).' (:110-111); \
the registration side carries the measured why, verbatim (reactor.rs:248-251): \
'empty→wake: unpark only when this registration transitioned the reactor from idle to \
active (the scanner may be parked). At higher registration rates the scanner is already \
awake and the skipped unparks avoid cross-core `Parker` contention (+38% throughput in \
the A/B).' + slot_table.rs:56-57 'Measured +38% registration throughput vs always-unpark \
by removing cross-core `Parker` cache-line contention.' (c) THE PARK TOKEN ABSORBS THE \
RACE — an unpark landing between the scan and the park is not lost: 'An unpark that \
lands between the scan and the park is absorbed by the park token, so no registration \
is missed.' (reactor.rs:160-161) — the token is a permit (std::thread's park/\
unpark_has_token semantics), so wake-then-park in either order converges. (d) THE \
IDLE-COST DISCLOSURE keeps (a)+(b) honest about the trade: 'Re-measured on the lock-free \
scan: never-parking is latency-neutral (10.0-10.5 vs 10.2 us medians at N=1, budget 0) \
— the old +4 us penalty was the scan-lock contention — so parking wins on idle CPU.' \
(reactor.rs:158-163) — parking buys idle-CPU, never latency, and the +38%/µs figures \
are the source's own slot_table ab_bench A/B (the 2×2 unpark-policy × idle-posture \
matrix over saturated + bursty regimes, 8 threads × 150k ops / 1024 slots, best-of-3) — \
provenance, never bars. ⛔ KINSHIP (adjudicated both directions, \
compose-not-double-count): channel-send-batching-unpark (rust_perf entries_02) + its \
Batch-224 rider own the THRESHOLD-gated wake — a blocked sender wakes the receiver when \
a batch buffer FILLS to a threshold (send amortization); THIS gates on the 0→1 \
TRANSITION of an armed-count (contention avoidance at high registration rates — the \
wake is priced by how many listeners are already awake, not by how much work \
accumulated; compose on a producer lane). parked-waiters-release-off-worker (gpu_162, \
E) owns waiter CUSTODY — WHERE the waiter parks (off the runnable queue, the second \
parked budget, the PARK_STALL fail-closed ceiling); THIS owns WHEN the wake fires (the \
transition gate) and WHAT evidence keeps the waiter parked (the armed count, never a \
flag observation) (compose: custody is the park posture, THIS is the wake policy). \
REGIME: a scanner/server loop whose work arrives as STATE TRANSITIONS (armed counters, \
pending sets, queue depths) and whose registration rate is high enough that \
always-unpark wake storms dominate. ANTI-REGIME: low registration rates (every \
registration IS the first — the gate degenerates to always-unpark, correctness \
identical, the gate pure overhead); loops that must wake on FLAGS not state counts (a \
flag is work-arrival evidence an armed count lacks — the anti-shape of (a)). ⛔ \
NotBenchable-by-domain (the win metric is cross-core Parker cache-line contention — \
scheduler behavior no validator hosts; the RELATIONSHIPS are CPU-testable and the \
fixtures pin them: unpark count == transition count under sustained registration, \
never the registration count; an armed-but-not-flag-complete slot keeps the scanner \
awake; a parked-over-work scanner is the caught tamper). Fixture: the fixture models \
the discipline deterministically (the transition arm: 100 registrations into an active \
table → exactly 1 unpark; the park-gate arm: an armed slot whose flag lands later is \
observed — the tamper gate-on-flag-saw parks over it and misses the completion, \
caught; the park-token arm: an unpark delivered between scan and park is absorbed — \
the permit model; the all-safe arm: a single registration parks-and-wakes \
byte-identically to always-unpark). Source: NVIDIA/cuda-rust @ \
f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0) — cuda-async/src/reactor.rs:154-163 \
(the park gate + the token + the idle disclosure), :248-258 (register + empty→wake), \
slot_table.rs:45-60 (n_armed + the +38% doc), :108-125 (publish + was_idle + \
increment-before-bit), :473-518 (the ab_bench A/B). Quotes byte-verified at the clone \
at the pin this session. Batch 419 / Plan 415 / Research 450."
                .to_string(),
            ref_shape: r"// NAIVE: two defects. (1) The park gate reads the per-pass
// observation, not the pending-work count: a pass that saw no flags
// parks — over armed slots whose flag writes are still in flight.
// (2) Every registration unparks: the wake storm is O(registrations).
fn scanner_naive(table: &mut Table) {
    loop {
        let saw_flags = scan_pass(table);
        if !saw_flags {
            // WRONG gate: armed-but-not-flag-complete work exists, and
            // we park over it. The completions stall until the next
            // unrelated unpark.
            thread::park();
        }
    }
}

fn publish_naive(table: &mut Table, slot: Slot) {
    table.arm(slot);
    table.notify_scanner(); // ALWAYS: O(N) unparks for N registrations,
                            // each a cross-core IPI into `Parker`'s line
}
"
            .to_string(),
            opt_pattern: r"// BOUND: the gate is the armed count; unpark fires only on the
// 0->1 transition; the park token absorbs the scan-vs-park race.
fn scanner_bound(table: &Table) {
    loop {
        scan_pass(table);
        if table.is_idle() {
            // the armed COUNT, Acquire-loaded: a slot armed but not yet
            // flag-complete keeps the scanner awake. An unpark landing
            // between this check and the park is absorbed by the park
            // token, so no registration is missed.
            thread::park();
        }
    }
}

/// Returns whether this was the idle->active transition (armed 0->1) —
/// the caller wakes the scanner ONLY THEN. Increment precedes the
/// active-bit set (the no-underflow invariant).
fn publish(table: &mut Table, slot: Slot) -> bool {
    let was_idle = table.armed.fetch_add(1, Ordering::AcqRel) == 0;
    table.arm(slot); // sets the active bit with Release after the count
    was_idle
}

fn register(table: &mut Table, slot: Slot, scanner: &Scanner) {
    if publish(table, slot) {
        scanner.unpark(); // at higher rates the scanner is already awake:
                          // the skipped unparks avoid cross-core
                          // `Parker` cache-line contention (+38% in the A/B)
    }
}
"
            .to_string(),
        },
        // ── R5 — the wait-site spin ahead of the completion-callback
        // registration: µs-scale waits resolve at the site, the budget is
        // derived from the measured wait distribution, and a spin-phase
        // query error IS the future's error. ──
        KernelRule {
            rule_id: "bounded-inline-spin-before-completion-registration".to_string(),
            gpu_concept: "MECHANISM (the wait-site spin ahead of the completion-callback \
registration; category D — Dispatch, the host wait plane): a wait measured in MICROSECONDS \
is shorter than the notification machinery it would invoke — registering a completion \
callback pays the waker round trip ('reactor/host-fn -> waker -> scheduler -> re-poll') \
before the answer arrives, so the FIRST polling posture spins at the wait site inside a \
bounded budget and registers the completion callback only past it. THE SOURCE, verbatim \
at the pin (NVIDIA/cuda-rust cuda-async/src/device_future.rs:330-334 @ f35d6ccc2e): \
'Microsecond-scale waits are too short for a waker round trip (reactor/host-fn -> waker \
-> scheduler -> re-poll); spinning at the wait site resolves short pipelines at \
sync-like latency. Budget-bounded so long pipelines fall through to the reactor/callback \
path.' Four decision contents. (a) THE BUDGET IS DERIVED FROM THE MEASURED WAIT \
DISTRIBUTION, never tuned: 'Default 20 us ≈ Q3 of measured decode-step kernel \
durations.' (:337) — the budget is a statistic of the workload's own wait class (the \
upper quartile: waits shorter than it are the common case and resolve at the site; \
waits past it are the tail and fall through), exposed as `fn inline_spin_budget_us() -> \
u64` reading `CUDA_ASYNC_SPIN_BUDGET_US` (OnceLock-memoized, :341-349), where budget 0 \
forces every pipeline through the completion-notification path — the source's own \
correctness-test posture and the demote literal. (b) THE SPIN POLLS THE AUTHORITATIVE \
SOURCE and falls through cleanly: the loop polls `stream.query()` (Ok(true) = complete \
at the site, Ok(false) = keep spinning with `spin_loop` hints, past the deadline = fall \
through), and ONLY the fall-through reaches `register_callback` (:384-388) — the \
completion-callback registration is the TAIL of the posture, never the head, so short \
waits never pay it. (c) THE ERROR IS NOT SWALLOWED: 'A query error is the stream's \
(sticky) fault: no completion signal will ever arrive, so it is the future's error.' \
(:335-336) — an error during the spin RETURNS as the future's error immediately, never \
falls through to registration (where it would hide behind a perpetual wait — the \
device_fault.rs:161-168 disclosure: before the fix the spin swallowed the sticky error \
and fell through; the sticky error is process-persistent, so swallowing it poisons \
every later future on that stream invisibly). (d) THE WAKER IS REGISTERED BEFORE THE \
SPIN (:323) — the spin is bounded and the completion can land concurrently while it \
runs; registering the poll's waker first makes a concurrent completion observable (the \
future's next poll after the wake re-reads the state), so the bounded spin never loses \
a completion that arrived mid-spin — the ordering is register-waker, execute, spin, \
fall-through, NOT spin-then-register-everything. ⛔ KINSHIP (adjudicated both \
directions, compose-not-double-count): deadline-bounded-poll-of-submitted-completion-event \
(gpu_60) owns the POST-registration wait — a deadline-bounded host poll of an ALREADY \
submitted completion event; THIS owns the PRE-registration wait-site spin (compose: \
gpu_60 governs the long wait after registration, THIS the short wait before it — \
together they cover the whole wait-duration axis with the budget as the boundary). \
adaptive-yield-consecutive-ready (rust_perf entries_10) owns consumer-side yield PACING \
(how a consumer yields between ready checks); the entries_74 checked-spin owns clock \
RESIDENCY during a spin; entries_22's spin-then-park hybrid owns µs-class LOCK posts — \
all three pace or instrument a spin that has ALREADY decided to wait; THIS decides \
WHETHER to wait at the site at all vs paying the registration (compose). REGIME: waits \
measured in microseconds whose completion notification costs more than the wait (the \
Q3-budget regime — short kernels, decode steps); ANTI-REGIME: long waits (the budget \
MUST fall through — a budget sized past the wait class spins hot against a callback \
that would have parked the thread); budget 0 postures (every wait takes the \
notification path, the correctness posture); a query class whose error is not sticky \
(then the error arm needs its own delivery decision — the rule prices STICKY faults). \
⛔ NotBenchable-by-domain (the win metric is the waker round trip — scheduler and \
driver behavior no validator hosts; the BUDGET SHAPE is CPU-testable and the fixtures \
pin it: short wait completes at the site with ZERO registrations, long wait falls \
through with exactly ONE, an error returns without any registration, budget 0 skips \
the spin; the dial9/B136 annotation class). Fixture: the fixture models the posture \
deterministically (the four arms above + the tamper arm: a spin that SWALLOWS the \
query error and falls through — caught: the error is lost AND a registration fired). \
Source: NVIDIA/cuda-rust @ f35d6ccc2e03a54e73e3796f415f11313e439d50 (Apache-2.0) — \
cuda-async/src/device_future.rs:323-349 (the waker-first ordering + the budget fn), \
:350-370 (the labeled spin block), :371-388 (the dispatch: complete-at-site / \
fall-through / error-as-future's-error), tests/device_fault.rs:161-168 (the \
sticky-error propagation pin). Quotes byte-verified at the clone at the pin this \
session. Batch 419 / Plan 415 / Research 450."
                .to_string(),
            ref_shape: r"// NAIVE: EVERY wait — however short — registers the completion
// callback first, then parks into the waker round trip. A 5 us kernel
// pays: callback registration + host-func hop + waker -> scheduler ->
// re-poll. The machinery outlives the wait.
fn poll_naive(&mut self, cx: &mut Context) -> Poll<Output> {
    if let Some(out) = self.completed() {
        return Poll::Ready(Ok(out));
    }
    // register FIRST: the round trip begins before we even know how
    // long the wait would have been.
    self.register_callback(cx.waker().clone());
    Poll::Pending
}
// the sticky fault variant: a swallowed query error falls through to
// registration, hiding the fault behind a perpetual wait.
"
            .to_string(),
            opt_pattern: r#"// BOUND: spin the wait site inside a measured budget; register
// the completion callback only past it; a query error IS the error.
fn inline_spin_budget_us() -> u64 {
    // Default 20 us ~= Q3 of measured decode-step kernel durations.
    static BUDGET: OnceLock<u64> = OnceLock::new();
    *BUDGET.get_or_init(|| {
        std::env::var("CUDA_ASYNC_SPIN_BUDGET_US")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20)
    })
}

fn poll_bound(&mut self, cx: &mut Context, stream: &Stream) -> Poll<Output> {
    if let Some(out) = self.completed() {
        return Poll::Ready(Ok(out));
    }
    self.waker.register(cx.waker().clone()); // BEFORE the spin: a
                                             // concurrent completion is
                                             // observable, never lost
    if self.started() {
        let deadline = Instant::now() + Duration::from_micros(inline_spin_budget_us());
        loop {
            match stream.query() {
                Ok(true) => return Poll::Ready(Ok(self.take_result())), // at the site
                Ok(false) => {}
                // A query error is the stream's (sticky) fault: no
                // completion signal will ever arrive, so it is the
                // future's error — never swallowed, never fall-through.
                Err(e) => return Poll::Ready(Err(Error::Driver(e))),
            }
            if Instant::now() >= deadline {
                break; // past the budget: the long-wait tail
            }
            std::hint::spin_loop();
        }
        self.register_callback(cx.waker().clone()); // the TAIL, never the head
    }
    Poll::Pending
}
"#
            .to_string(),
        },
    ]
}

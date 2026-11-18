// Batch 372 — the infinity tokio-family distill
// (hydro-project/infinity @ 1abc6fcff1f186124f924b56aa92c3a3684c90c5,
// Apache-2.0; clone `.raw/b306` read at the pin, T0 2026-11-18, quotes
// byte-verified; the T0 unread-file sweep graded rule R2 UP — three more
// tail-extend instances: terminal.rs / slack_io.rs / lambda event_handler).
//
// rust_perf 365 → 369: R1 `closed-channel-select-arm-gate` (concurrency,
// suggestion-only) + R2 `streamed-chunk-tail-coalesce-kind-gated`
// (allocation, BENCHABLE — the batch's GOAT lead, alloc-count axis) +
// R3 `inline-actor-futures-pool-not-spawn-per-actor` (concurrency,
// suggestion-only; carries the no-yield-idle-recheck rider + the
// recv-one-then-try-recv-drain face folded from the T1 candidate #4) +
// R5 `defer-deferrable-inputs-while-work-in-flight` (concurrency,
// suggestion-only) + 2 count-neutral faces on existing hosts (the
// cancel-sender-registration face on `oneshot-response-send-unwrap-panics`,
// entries_06; the stopped-thread admission face on
// `reject-plan-runs-before-setup-with-outliving-side-effects`, entries_44).
//
// Fixtures `corpus/fixtures/b372_fixtures.rs` F1–F5 are ORIGINAL plain-Rust
// reproductions (zero repo source pasted), standalone-compiled AND executed
// BEFORE entry. NOTHING INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: a closed channel is ALWAYS READY — the select arm must be gated ──
pub fn r1_closed_channel_select_arm_gate() -> PerfRule {
    PerfRule {
        rule_id: "closed-channel-select-arm-gate".to_string(),
        name: "a `tokio::select!` arm polling a channel that CLOSES mid-loop is ALWAYS READY from then on — recv on a closed channel returns `None` immediately, so under `biased` (or an early-ordered arm) every loop iteration takes that arm: the loop spins hot on `None` and starves the live arms → gate the arm with a state bool set on the first `None` (select guard `if !closed`, or swap the receiver to `Option<R>` and `take()` it), keeping the closed-as-signal semantics explicit: the close is handled ONCE, then the select stops polling that arm".to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// NAIVE: the loop treats a closed subscription channel as "nothing
// yet" and keeps polling it. A closed channel is ALWAYS READY — recv
// returns None immediately — so once the subscriber shuts down, EVERY
// iteration of this biased select takes the closed arm: the loop spins
// hot, the work arm starves, the CPU burns.
fn event_loop(
    subscribe_rx: &mut tokio::sync::mpsc::UnboundedReceiver<SubscribeReq>,
    work_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Work>,
) {
    loop {
        tokio::select! {
            biased;
            maybe = subscribe_rx.recv() => {
                match maybe {
                    Some((req, ack)) => {
                        ack.send(()).ok();
                        serve(req);
                    }
                    None => {
                        // "subscriber gone — nothing to do" — the loop
                        // CONTINUES and re-polls the closed arm: the spin
                    }
                }
            }
            item = work_rx.recv() => {
                if let Some(w) = item {
                    process(w);
                }
            }
        }
    }
}"#.to_string(),
        fixed_code: r#"// FIXED: a state bool set on the FIRST None. The close is the
// signal, handled exactly once; after it the select no longer POLLS
// the dead arm — the biased loop blocks on the live arm only. (The
// alternative spelling: keep the receiver in an Option<R> and take()
// it on close — the guard and the swap are the same law.)
fn event_loop(
    subscribe_rx: &mut tokio::sync::mpsc::UnboundedReceiver<SubscribeReq>,
    work_rx: &mut tokio::sync::mpsc::UnboundedReceiver<Work>,
) {
    let mut subscribe_closed = false;
    loop {
        tokio::select! {
            biased;
            maybe = subscribe_rx.recv(), if !subscribe_closed => {
                match maybe {
                    Some((req, ack)) => {
                        ack.send(()).ok();
                        serve(req);
                    }
                    None => subscribe_closed = true, // closed: stop polling
                }
            }
            item = work_rx.recv() => {
                if let Some(w) = item {
                    process(w);
                }
            }
        }
    }
}"#.to_string(),
        context_notes: Some("Grade B+ — suggestion-only, honestly NotBenchable (spin/liveness CPU class: the harm is a hot poll loop's CPU share, not a hot loop's wall or allocs — the adaptive-yield/blocking-sleep annotation family). SOURCE (hydro-project/infinity @ 1abc6fcff1f186124f924b56aa92c3a3684c90c5, Apache-2.0; clone .raw/b306 read at the pin, T0 2026-11-18). THE DEFECT, infinity-agent-core/src/system/local/driver.rs:136-138 at the pin verbatim: 'Set when subscribe_rx is closed, so selects stop polling it (a closed channel is always ready and would otherwise spin).' — the gate bool is set on the first None and consulted by BOTH select loops of the driver (the in-flight loop and the park loop); the None arm stays reachable exactly once and the close keeps its SIGNAL meaning (the subscription ended), it never exits the driver by itself. MECHANISM: a tokio channel receiver that has observed close is permanently ready — the select arm fires every pass; under `biased` (arms polled in written order) an always-ready early arm preempts the loop every iteration. THE FIX is arm REMOVAL, not yield budgeting: the select guard (`if !subscribe_closed`) or the Option<R>-take swap takes the dead arm out of the poll set. REGIME: long-lived select loops over channels whose producer can exit (subscriber/request lifecycles, watch/broadcast consumers, worker shutdown edges); ANY arm whose channel is polled to completion inside a persistent loop. ANTI-REGIMES: the None arm EXITS the loop (the closed-as-signal IS the exit — no spin, nothing to fix); non-biased loops (tokio rotates arm order per poll, so the dead arm burns one poll per iteration instead of starving the rest — the gate is still the fix, the severity varies, disclosed); channels that can never close (a static broadcast receiver — no arm to gate). DISTINCT-FROM (T1 re-greps: 'always ready'/'closed channel'/'stop polling'/'Option<Receiver'/'lost wakeup' ALL zero corpus hits; the select! surface is 4 named files, none an arm-lifecycle owner): `adaptive-yield-consecutive-ready` (entries_02) budgets YIELDS at a live poll site that stays productive — THIS removes a DEAD arm entirely, different fix space, different premise; `blocking-sleep-in-async` (entries_06) is blocking the WORKER thread, not spinning the select loop; `channel-send-batching-unpark` (entries_02) + its B224 consumer-park rider own the crossbeam park/unpark discipline on LIVE channels — a park needs a producer to unpark it, a CLOSED channel has none, only removal stops the spin; `oneshot-response-send-unwrap-panics` (entries_06) is send-side hygiene; `spin-keep-awake-checked-capability-degrade-loud` (entries_74) is accelerator clock residency; `fanout-first-error-leaves-siblings-running` (entries_63) reads FuturesUnordered early-return for CANCELLATION semantics — distinct axis. Fixture: b372_fixtures.rs F1 (the poll-count spin pin: buggy counts a poll per iteration after close, fixed zero; the close handled exactly once; the open-channel negative — the gate never over-suppresses a live channel). Plan 310 / Research 349.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R2: streamed same-kind chunks extend the TAIL entry, other kinds push ──
pub fn r2_streamed_chunk_tail_coalesce_kind_gated() -> PerfRule {
    PerfRule {
        rule_id: "streamed-chunk-tail-coalesce-kind-gated".to_string(),
        name: "streamed chunks of the SAME kind landing in an ordered buffer extend the TAIL entry (`last_mut` + `push_str`) while the last entry's kind matches; any other kind pushes a fresh entry — per-chunk entries (one String + one Vec slot per chunk) collapse into one amortized-growth buffer per kind-run, the assembled history bit-identical, interleaved kinds untouched (the kind-gate is the decision content: extend-only-same-kind)".to_string(),
        category: "allocation".to_string(),
        buggy_code: r#"// NAIVE: every streamed chunk pushes its own entry. A 500-chunk
// text turn = 500 String allocations + 500 Vec slots, and the history
// the UI/persist layer walks is 500 tiny entries instead of a few runs.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    ToolUse,
}

struct Entry {
    kind: Kind,
    text: String,
}

fn push_chunk(history: &mut Vec<Entry>, kind: Kind, chunk: &str) {
    // one String allocation + one Vec slot PER CHUNK
    history.push(Entry {
        kind,
        text: chunk.to_string(),
    });
}"#.to_string(),
        fixed_code: r#"// FIXED: extend the tail entry while the KIND matches; any other
// kind pushes. The coalesce is the ingest side only — the assembled
// text is bit-identical and interleaved kinds stay separate entries
// (the negative the gate exists for).
fn push_chunk(history: &mut Vec<Entry>, kind: Kind, chunk: &str) {
    if let Some(last) = history.last_mut() {
        if last.kind == kind {
            last.text.push_str(chunk); // amortized growth, no new entry
            return;
        }
    }
    history.push(Entry {
        kind,
        text: chunk.to_string(),
    });
}"#.to_string(),
        context_notes: Some("Grade B — the batch's GOAT LEAD, BENCHABLE (the alloc-count axis is exact: one String + one Vec slot per chunk vs one amortized-growth buffer per kind-run; the assembled text is bit-identical). SOURCE (hydro-project/infinity @ 1abc6fcff1f186124f924b56aa92c3a3684c90c5, Apache-2.0; clone .raw/b306 read at the pin). THE DEFECT+LAW, infinity-agent-core/src/event_processor.rs:606-611 at the pin verbatim: 'If the last buffered turn entry is an assistant text message, append `text` to it and return `true`. Otherwise return `false` so the caller pushes a new buffer entry. This coalesces consecutive text chunks within a turn into one message.' — try_merge_buffer_text reads buffer.last_mut(), pattern-matches the entry's KIND (InfinityMessage::Assistant + AssistantContent::Text), and pushes into existing.text only on the match; the source's own pinned negative fixture text_chunks_not_coalesced_across_non_text_item (event_processor.rs:2151) proves the kind-gate is LOAD-BEARING (extend-across-kinds would corrupt the history), so the gate is decision content, never ambient. THE T0 UNREAD-FILE SWEEP graded this rule UP — THREE more positive instances of the same keyed-tail shape: infinity-agent-cli/src/terminal.rs:407-409 (thinking_text_buffer.push_str + thread_buffers.entry(child_tid).or_default().push_str — per-thread keyed tails), infinity-slack-bot/src/slack_io.rs:445-456 (pending_text.entry(thread_ts).or_insert_with + entry.text.push_str — 'This coalesces rapid chunks into one append to avoid Slack throttling.': the RATE-LIMIT flush axis beside the history axis, FLUSH_INTERVAL = Duration::from_secs(3), MissedTickBehavior::Delay, drained on the interval tick + StreamStop), infinity-agent-lambda/src/event_handler.rs:191-193 (per_thread.entry(thread_id).or_default() + entry.accumulated_text.push_str). PRE-REGISTERED BARS (honest, alloc-axis-primary): alloc axis — buggy = 1 String clone + Vec growth class PER CHUNK; fixed = 1 String per kind-RUN + O(log) growth reallocations (the every-64th-chunk tool-boundary pattern gives ~n/32 kind-runs — a text run + a tool entry per 64-chunk window — plus the growth class) — expect a wide alloc reduction at every size; TIME — fixed must never be SLOWER at any size beyond noise (amortized growth keeps total memcpy O(n); a real regression kills the rule); a time Pareto win is a BONUS, not required — the rule's claim is the alloc axis, so a Mixed verdict with alloc dominance lands honestly disclosed (the ≥1.05x-every-size time floor was entries_85's claim shape, not ours). G1: the fixture pins bit-identical assembled text between arms + the interleaved-kind negative (kinds stay separate entries) + the counted alloc delta. REGIME: streamed-token transcript buffers, per-thread/per-key text accumulators, chat/webhook append batching under a rate budget, event-history ingest — anywhere same-kind chunks arrive consecutively and the buffer is consumed as runs. ANTI-REGIMES: chunks already large (the per-chunk entry overhead is noise at KiB-scale chunks); histories consumed per-ENTRY downstream (the coalesce changes the visible shape — the kind-run boundary IS the consumer contract); streams whose kinds interleave per-chunk (no runs — the gate falls through to push, no harm, no win). DISTINCT-FROM (T1, recorded both directions): `coalesce-nearby-reads-bounded-gap-cacheability-gated` (entries_05) shares the last_mut-extend SHAPE but its gate is a byte-GAP budget over syscall spans — THIS gates on kind IDENTITY over history entries; anti-regimes differ (a far gap kills that merge, interleaved kinds fall through this one harmlessly); the entries_31/42 span-normalize family is span algebra, same verdict; `slow-consumer-latest-value-single-slot` (entries_70) coalesces by DROPPING intermediates (lossy latest-wins) — THIS is lossless concatenation; `compaction-tail-boundary-prefers-complete-turn` (entries_06) is lossy truncation PLACEMENT at compaction time — THIS is lossless merge at INGEST; `streaming-prefix-holdback-bounded-buffer` (entries_06) is a holdback posture, not a merge; `ui-event-stream-sampled-gate-terminal-forward` (entries_70) is lossy class sampling; COMPOSE: the entries_01/02 String-building family (format! concat → push_str) governs HOW to extend a buffer, THIS governs WHEN to extend the tail entry vs push a new one. Fixture: b372_fixtures.rs F2. Plan 310 / Research 349.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(streamed_chunk_tail_coalesce_bench_spec()),
    }
}

#[cfg(feature = "rust_perf_bench")]
#[cfg(feature = "rust_perf_bench")]
fn streamed_chunk_tail_coalesce_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
pub struct Input {
    chunks: Vec<(u8, String)>, // 0 = text, 1 = tool
    sink: u64,
}

pub fn make_input(n: usize) -> Input {
    // Deterministic LCG; every 64th chunk is a tool-kind boundary so the
    // stream is mostly same-kind runs (the LLM-turn shape).
    let mut s = 0x243f_6a88_85a3_08d3u64;
    let mut chunks = Vec::with_capacity(n);
    for i in 0..n {
        s = s.wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let kind = if i % 64 == 63 { 1u8 } else { 0u8 };
        let len = 8 + ((s >> 33) as usize) % 16;
        let word = (b'a' + ((s >> 40) as u8 % 26)) as char;
        let text: String = std::iter::repeat(word).take(len).collect();
        chunks.push((kind, text));
    }
    Input { chunks, sink: 0 }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Text,
    Tool,
}

struct Entry {
    kind: Kind,
    text: String,
}

pub fn run_buggy(input: &Input) {
    let mut history: Vec<Entry> = Vec::new();
    for (kind, chunk) in std::hint::black_box(&input.chunks) {
        let k = if *kind == 0 { Kind::Text } else { Kind::Tool };
        history.push(Entry {
            kind: k,
            text: chunk.clone(), // the per-chunk String allocation
        });
    }
    let mut acc = 0u64;
    for e in &history {
        acc = acc.wrapping_add(e.text.len() as u64);
    }
    std::hint::black_box(acc);
}

pub fn run_fixed(input: &Input) {
    let mut history: Vec<Entry> = Vec::new();
    for (kind, chunk) in std::hint::black_box(&input.chunks) {
        let k = if *kind == 0 { Kind::Text } else { Kind::Tool };
        if let Some(last) = history.last_mut() {
            if last.kind == k {
                last.text.push_str(chunk);
                continue;
            }
        }
        history.push(Entry {
            kind: k,
            text: chunk.clone(),
        });
    }
    let mut acc = 0u64;
    for e in &history {
        acc = acc.wrapping_add(e.text.len() as u64);
    }
    std::hint::black_box(acc);
}
"#
        .to_string(),
    }
}

// ── R3: pool the actor futures; respawn-on-input replaces keep-alive tasks ──
pub fn r3_inline_actor_futures_pool_not_spawn_per_actor() -> PerfRule {
    PerfRule {
        rule_id: "inline-actor-futures-pool-not-spawn-per-actor".to_string(),
        name: "an idle-heavy actor fleet holding one RESIDENT task per actor (`tokio::spawn` per actor — the task's whole state stays resident while it waits for its next input) → own the per-actor futures in ONE dispatcher-pooled `FuturesUnordered` (LocalSet/`!Send` posture): a pooled future is DROPPED the moment its actor goes idle and the router respawns it on the next input addressed to it — respawn-on-input replaces keep-alive tasks, memory is released at idle; the final idle decision is made WITHOUT yielding to the dispatcher so the exit race has exactly two outcomes (input appears in rx → processed; input lands after the receivers drop → the respawn path owns it, never a lost wakeup)".to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// NAIVE: one spawned keep-alive task per actor. Each task holds the
// actor's whole state (buffers, channel ends) RESIDENT while it waits
// for its next input — at 10k mostly-idle actors that is 10k resident
// task states + their stacks, just to wait.
async fn spawn_actor(id: ActorId, inbox: mpsc::Receiver<Msg>) {
    tokio::spawn(async move {
        let mut state = ActorState::load(id); // resident forever
        while let Some(msg) = inbox.recv().await {
            state.apply(msg);
        }
    });
}"#.to_string(),
        fixed_code: r#"// FIXED: the router owns the actor futures in ONE pool. An actor's
// future runs only while it has work; at idle it RETURNS (dropped —
// memory released) and the router respawns it on the next input
// addressed to it. No keep-alive tasks. The idle decision is taken
// synchronously (no yield before it), so an input racing the exit is
// EITHER seen in rx (processed) OR lands after the receivers dropped
// and the respawn path delivers it — never lost.
async fn route(
    pool: &mut FuturesUnordered<ActorFut>,
    mut mail: mpsc::Receiver<Envelope>,
) {
    loop {
        tokio::select! {
            maybe = mail.recv() => {
                let Some(env) = maybe else { break };
                if !pool.has_actor(env.to) {
                    pool.push(ActorFut::respawn(env.to)); // wake on demand
                }
                deliver(pool, env);
            }
            Some((id, outcome)) = pool.next() => {
                record(id, outcome); // idle exit: the future is gone
            }
        }
    }
}"#.to_string(),
        context_notes: Some("Grade B− — suggestion-only, honestly NotBenchable (the per-idle-actor MEMORY-FOOTPRINT axis is absent from the Pareto validator — the fixture counts RESIDENT WORKERS, the validator prices a hot loop's wall/allocs; the dial9 annotation class, not a demotion). SOURCE (hydro-project/infinity @ 1abc6fcff1f186124f924b56aa92c3a3684c90c5, Apache-2.0; clone .raw/b306 read at the pin). THE LAW, infinity-agent-core/src/system/local/router.rs:1-5 at the pin verbatim: 'The router owns the driver futures directly (one `FuturesUnordered` pool rather than one spawned task per thread), so a driver's memory is fully released the moment it goes idle.' — the driver lifecycle is Idle → exit; the router respawns the driver on the next event; the hibernation economics (the sleep_until_event_or_input tool, pure-data turn state) are the regime story: an actor's BETWEEN-TURN state is data, so a future is cheap to drop and cheap to respawn. RIDER (the exit-race half, driver.rs check_before_idle doc at the pin verbatim): 'Perform the final idle check without yielding to the router task. Since the router shares this `LocalSet`, an input either appears in `rx` here or is sent after the receivers are dropped, causing the router to spawn a new driver.' — the final drain/recheck runs WITHOUT a yield, so the race has exactly two outcomes and the respawn path is the designated owner; never a lost wakeup. FACE (folded from the T1 candidate #4, the receiver side of the wake-economics triangle — driver.rs:174-176 at the pin: 'let mut batch = vec![first];' / 'while let Ok(item) = rx.try_recv() {' / 'batch.push(item);' / '}' — the drain runs in BOTH select arms and again after the park loop): after a blocking recv returns one item, DRAIN the rest eagerly with try_recv() into the same batch — the unit of work per wake is everything that queued while you were busy, not one message. The triangle: `steal-batch-not-per-item` (entries_06) owns the stealer side of a crossbeam deque, `channel-send-batching-unpark` (entries_02) the sender side, its B224 rider the consumer PARK discipline, THIS the receiver drain-per-wake. REGIME: idle-heavy fleets (agent/thread/session actors, per-connection tasks, per-subscription loops) where resident-per-idle memory is the constraint — density-serving postures. ANTI-REGIMES: few long-lived BUSY actors (spawn-per-actor is the right posture — the pool adds dispatch overhead for nothing); actors whose respawn cost dominates (heavy state reload per wake — hibernation-as-data is the source's answer: keep the between-turn state as data, never inside the resident task); Send-constrained fleets (the pool is the LocalSet/!Send posture — a cross-thread fleet keeps spawned tasks and pays their residency). DISTINCT-FROM: `thread-spawn-per-call-pool-reuse` (entries_72) — COMPOSE: that rule pools the OS THREADS against spawn COST per call, this pools the async task FUTURES against idle RESIDENCY — different resource, different economics; `batch-blocking-pool-segment` (entries_06) is the blocking side's global-resource law, this the async side; `actor-bounded-channel-cycle-deadlock` (entries_06) is backpressure cycles, orthogonal; `fanout-first-error-leaves-siblings-running` (entries_63) reads FuturesUnordered early-return for CANCELLATION semantics — compose (same mechanics, this rule reads it for residency, that for cancellation); `spawn-blocking-queue-wait-split-from-compute` (entries_80) is measurement honesty of a spawned op's timing, distinct. Fixture: b372_fixtures.rs F3 (the threads-as-tasks model, DISCLOSED in the fixture header: resident-thread count at idle, respawn-on-input delivery, the synchronous-recheck race, the drain-per-wake face). Plan 310 / Research 349.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ── R5: synthetic events defer behind the in-flight call; priority still interrupts ──
pub fn r5_defer_deferrable_inputs_while_work_in_flight() -> PerfRule {
    PerfRule {
        rule_id: "defer-deferrable-inputs-while-work-in-flight".to_string(),
        name: "while a long call is in flight, input CLASS decides handling — PRIORITY input (the user's text) interrupts the call deliberately, SYNTHETIC events (subscription events, child reports, parent messages) DEFER behind a queue until the pending call resolves; running a synthetic mid-call would interrupt the pending work: the runtime injects a synthetic \"interrupted\" result so the history stays well-formed and the REAL result — arriving later — is dropped as stale: the work's output is thrown away AND the caller's wait wasted → classify before you act; defer the deferrable, drain it in order after the pending resolves, and never defer the priority lane".to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// NAIVE: every input is handled as it arrives. A synthetic event
// landing mid-call interrupts the pending tool call — the runtime must
// inject an "interrupted" result to keep the history well-formed, and
// when the REAL result finally arrives it is dropped as stale. The
// work is lost and the caller's wait wasted.
enum Input {
    UserText(String),
    Synthetic(SynEvent),
}

async fn on_input(state: &mut Runtime, input: Input) {
    match input {
        Input::UserText(t) => interrupt_pending(state, t), // deliberate
        Input::Synthetic(ev) => {
            // BUG: this cancels the in-flight call to run the event NOW
            abort_pending(state);
            handle_synthetic(state, ev);
        }
    }
}"#.to_string(),
        fixed_code: r#"// FIXED: classify — synthetic events DEFER while a call is pending
// and drain, in order, when it resolves. Priority input still
// interrupts immediately (over-deferral would trade the defect for an
// unresponsive priority lane — that negative is pinned).
async fn on_input(state: &mut Runtime, input: Input) {
    match input {
        Input::UserText(t) => interrupt_pending(state, t),
        Input::Synthetic(ev) => {
            if state.pending_call().is_some() {
                state.defer_queue.push(ev); // held, not run now
            } else {
                handle_synthetic(state, ev);
            }
        }
    }
}

async fn after_pending_resolves(state: &mut Runtime) {
    for ev in state.defer_queue.drain(..) {
        handle_synthetic(state, ev);
    }
}"#.to_string(),
        context_notes: Some("Grade B− — suggestion-only, honestly NotBenchable (work-interruption/latency class: the harm is a DESTROYED in-flight result plus a wasted caller wait, priced only in a live runtime — the dial9/annotation class). SOURCE (hydro-project/infinity @ 1abc6fcff1f186124f924b56aa92c3a3684c90c5, Apache-2.0; clone .raw/b306 read at the pin). THE LAW, infinity-agent-core/src/system/defer.rs:1-10 at the pin verbatim: 'The [`DeferQueue`] trait: holding back synthetic events while a tool call is in flight.' / 'When a thread has dispatched a tool call and is waiting for its result, processing an unrelated synthetic event (a subscription event, a child thread's report, a message from the parent) would *interrupt* the pending call: the runtime injects a synthetic \"interrupted\" result so the history stays well-formed, and the real result is later dropped as stale. To avoid cancelling work that is still running, such events should be deferred until the pending call is answered.' — filter_deferrable classifies and the driver consults defer-readiness before running queued events. The queue TRAIT is part of the law (platform-portable: InMemoryDeferQueue / NoDeferral / transport-level re-enqueue — the holding POINT moves with the platform, the classification does not). REGIME: agent/session runtimes with slow tool calls and mixed input sources; any event loop where side-channel events would cancel in-flight work. ANTI-REGIMES: inputs that MUST run now for correctness — the user text IS the interrupt, and deferring it trades the defect for an unresponsive priority lane (the fixture pins the priority-still-interrupts negative); calls short enough that a deferral queue would grow unbounded (bound the queue or run); deferrals that must order against the pending call's own result (the drain point is AFTER resolve, in order). DISTINCT-FROM (T1): `deferred-command-flush-hoist-out-of-batch-loop` (entries_08) owns flush PLACEMENT inside a batch loop (sync-point amortization) — no input classification, no in-flight result; `tier-write-defers-to-urgent-reads-except-a-read-behind-it` (entries_42) is the SAME GENUS (classify → defer the lower class) in a storage tier keyed on op class — distinct domain and consequence model (a slower write vs a DESTROYED result; compose: both classify-then-defer, the consequence axis differs); `rawvalue-passthrough-defer-parse` (entries_07) defers PARSE, not a runtime event; `nested-par-inside-channel-bridge-deadlocks` (entries_06) says 'interrupted' about a rayon steal, distinct. Fixture: b372_fixtures.rs F5 (the real-result-survives pin, the in-order drain, the priority-still-interrupts negative, the tampered classifier that defers EVERYTHING reds). Plan 310 / Research 349.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b372_rules() -> Vec<PerfRule> {
    vec![
        r1_closed_channel_select_arm_gate(),
        r2_streamed_chunk_tail_coalesce_kind_gated(),
        r3_inline_actor_futures_pool_not_spawn_per_actor(),
        r5_defer_deferrable_inputs_while_work_in_flight(),
    ]
}

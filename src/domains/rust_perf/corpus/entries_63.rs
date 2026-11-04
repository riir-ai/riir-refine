//! Entries slice — Batch 330 (2026-11-04): one `rust_perf` concurrency rule
//! from the vectorize-io/hindsight distill.
//!
//! Source: vectorize-io/hindsight @ `841979dfda4e1bebf7cbf68d0144d5d5454d93df`
//! (MIT code), re-verified at the pin this batch; the verbatim law quotes live
//! in the rule's `context_notes` (the doc-lazy-continuation class: quoted
//! prose in `//!` breaks the paragraph parse). Fixtures are ORIGINAL
//! plain-Rust stand-ins (std threads — the law text names the async twin),
//! standalone rustc-compiled AND executed before landing (buggy arm: 60
//! orphan commits; fixed arm: 1–2, stable across runs). Batch 330 / Plan 338
//! / Research 368.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── concurrency: a task fan-out whose first error propagates while
        // the spawned siblings keep running detached — cancel (or let
        // finish, by the shared-state discriminator) and AWAIT before the
        // error crosses the boundary ───────
        // Batch 330 / vectorize-io/hindsight @ 841979df (consolidator.py
        // _gather_or_cancel).
        PerfRule {
            rule_id: "fanout-first-error-leaves-siblings-running".to_string(),
            name: "a task fan-out whose first error propagates immediately while the spawned siblings keep running detached (spawned tasks are NOT cancelled by an early return — the orchestrator marks the operation failed and re-queues it while orphans keep side-effecting, breaking the serial-per-scope invariant exactly when it matters) -> on the error path CANCEL the outstanding work and AWAIT the cancellations BEFORE propagating (JoinSet::abort_all + drain every join handle; std twin: a cancellation flag set before the early return, then join) — cancel when siblings commit SHARED state, let them finish when their work is standalone and valuable, and never let an error ENVELOPE reclassify the failure (the async-group wrap misroutes retryable-vs-permanent)".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// NAIVE: fan out one worker per scope; the first error returns
// immediately and the other workers keep committing — the operation
// was already marked failed, the retries are queued, and the orphans
// are still writing (the orchestrator never told them to stop).

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

type CommitLog = Arc<Mutex<Vec<usize>>>;

fn scope_worker(base: usize, rows: usize, log: CommitLog, tx: mpsc::Sender<Result<(), String>>) {
    if base == 10 {
        let _ = tx.send(Err("scope 1: integrity violation".into()));
        return;
    }
    for row in base..base + rows {
        thread::sleep(std::time::Duration::from_millis(1)); // LLM-call stand-in
        log.lock().unwrap().push(row);
    }
    let _ = tx.send(Ok(()));
}

pub fn run_scopes(log: CommitLog) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    for base in [0usize, 10, 20] {
        let tx = tx.clone();
        let log = Arc::clone(&log);
        thread::spawn(move || scope_worker(base, 30, log, tx));
    }
    drop(tx);
    for msg in rx {
        if let Err(e) = msg {
            // first error: propagate NOW — healthy siblings unseen,
            // uncancelled, still committing write-groups
            return Err(e);
        }
    }
    Ok(())
}
"#
            .to_string(),
            fixed_code: r#"// GOVERNED: the error path cancels the outstanding work and AWAITS
// the cancellations before propagating — a cancelled worker's writes
// stay invisible and the recovery sweep resolves them, which is the
// same state a crash would leave.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

type CommitLog = Arc<Mutex<Vec<usize>>>;

fn scope_worker(
    base: usize,
    rows: usize,
    cancel: &AtomicBool,
    log: CommitLog,
    tx: mpsc::Sender<Result<(), String>>,
) {
    if base == 10 {
        let _ = tx.send(Err("scope 1: integrity violation".into()));
        return;
    }
    for row in base..base + rows {
        if cancel.load(Ordering::SeqCst) {
            let _ = tx.send(Ok(())); // cancelled: no further commits
            return;
        }
        thread::sleep(std::time::Duration::from_millis(1));
        log.lock().unwrap().push(row);
    }
    let _ = tx.send(Ok(()));
}

pub fn run_scopes(log: CommitLog) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    for base in [0usize, 10, 20] {
        let (tx, cancel, log) = (tx.clone(), Arc::clone(&cancel), Arc::clone(&log));
        handles.push(thread::spawn(move || scope_worker(base, 30, &cancel, log, tx)));
    }
    drop(tx);
    let mut first_err: Option<String> = None;
    for msg in rx {
        if let Err(e) = msg {
            if first_err.is_none() {
                first_err = Some(e);
                // CANCEL FIRST — while the siblings are still running —
                // so they stop at their next commit check.
                cancel.store(true, Ordering::SeqCst);
            }
        }
    }
    // Await the cancellations before propagating: returning while they
    // are still unwinding would reintroduce the very overlap this
    // prevents. (Async twin: JoinSet::abort_all(), then drain every
    // join handle before the `?`.)
    for h in handles {
        let _ = h.join();
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
"#
            .to_string(),
            context_notes: Some(
                "THE LAW: a task fan-out whose FIRST ERROR propagates while the spawned siblings \
keep running detached. Spawned tasks are not cancelled by an early return: the async \
orchestrator marks the operation failed and re-queues it while the orphans keep \
side-effecting — and nothing serialises an orphan against the retry, so the \
serial-per-scope invariant that keeps two workers out of the same observation scope is \
broken exactly when it matters. THE REPAIR IS TWO-STEP and the second step is the \
load-bearing one: cancel the outstanding tasks AND await the cancellations BEFORE \
propagating. SOURCE (vectorize-io/hindsight @ 841979dfda4e1bebf7cbf68d0144d5d5454d93df, \
MIT code; every quote re-verified at the pin this batch): _gather_or_cancel doc, \
verbatim: 'Plain asyncio.gather re-raises the first exception immediately but does NOT \
cancel its siblings — they keep running detached.' + the harm chain, verbatim: 'the \
failure propagates out of run_consolidation_job to the worker, which marks the \
operation failed and re-queues it with a 5s base backoff, while the orphaned tag \
groups are still calling the LLM, stamping mark_consolidated and committing \
write-groups.' + 'The per-scope scope_locks are local to one dispatch, so nothing \
serialises an orphan against the retry, and the \"batches within a group run \
serially\" invariant that keeps two consolidators out of the same observation scope is \
broken exactly when it matters.' THE BODY (consolidator.py L117-124), verbatim: 'So: \
cancel the outstanding tasks and await them before propagating.' + 'Await the \
cancellations before propagating: returning while they are still unwinding would \
reintroduce the very overlap this prevents.' + the crash-equivalence: 'A cancelled \
batch's writes stay invisible (its witness row is never committed) and are resolved by \
the recovery sweep, which is the same state a crash would leave.' THE TWO-SIDED \
DISCRIMINATOR (the sibling file carries the mirror image, fact_extraction.py \
L3592-3595, verbatim): 'return_exceptions=True so a failing item doesn't cancel its \
still-running siblings (which would leave orphaned LLM calls / partial work); we await \
them all, then propagate.' — CANCEL-THEN-AWAIT when siblings commit SHARED state \
(write-groups, stamps, marks — the overlap the retry must not fight); \
LET-FINISH-THEN-PROPAGATE when their work is STANDALONE and valuable (per-item \
extraction — cancelling would orphan real work). A prefetch no longer needed on an \
error path is cancelled in a finally block (reranking.py L482-483, verbatim: 'if \
embeddings is not None and not embeddings.done(): embeddings.cancel()'). THE \
ERROR-ENVELOPE SECONDARY (consolidator.py L109-113, verbatim): 'Deliberately not \
asyncio.TaskGroup: it wraps failures in an ExceptionGroup, and the worker's \
_is_non_retryable_task_error does isinstance checks on the raised exception — a \
wrapped IntegrityConstraintViolationError would be misclassified as retryable and \
retried forever. This helper re-raises the original exception unchanged.' — the Rust \
face is NARROWER, carried as context not law: anyhow's .context() PRESERVES downcast \
through the envelope, so wrapping is not the same hazard here; what remains is the \
envelope that REPLACES the error type (map_err to a coarse enum, a panic-to-string \
wrap, JoinError flattening) — a retry classifier reading `err.is::<Permanent>()` sees \
None through such a wrap and retries a permanent failure forever. THE RUST TWIN \
VOCABULARY (the law names it; the fixture is the std-thread shape): tokio::spawn'd \
tasks are DETACHED — dropping the future never aborts them; try_join_all / \
FuturesUnordered early-return stops POLLING, not the tasks; the bounded spelling is \
JoinSet (spawn into it), on the error path abort_all() + drain every join_next until \
None (checking JoinError::is_cancelled) BEFORE the `?`. THE STAND-IN: a cancellation \
flag (AtomicBool) checked between side-effecting steps, set BEFORE the error \
propagates, then join handles — cancel-before-propagate is the ordering that matters; \
in the fixture the healthy workers stop at their next check (1–2 commits at cancel \
time vs the buggy arm's 60, stable across runs). REGIME: an orchestrator that retries \
or re-queues (a job worker with backoff) over a fan-out whose members commit shared \
state — write batches, DB transactions, mark/stamp rows, files; multi-account sweeps, \
per-scope consolidations, batch migrations. ANTI-REGIME: read-only fan-outs with no \
retry loop (a wasted orphan task is idle CPU, not corruption — the discriminator's \
other side); members that are already cancellation-safe by construction (select! \
branches, scoped threads joined before return); single-task flows. KINSHIPS \
(grep-verified live at authoring): generation-counter-stale-timer-invalidation \
(entries_56 — detached-work cancellation, the timer-staleness axis: THIS is the \
fan-out error path); bounded-fanout-semaphore + oneshot-response-send-unwrap-panics + \
actor-bounded-channel-cycle-deadlock (entries_06 — fan-out bounding + task lifecycle \
liveness, distinct axes); online-cost-fit-thread-ladder-reopen-hysteresis (entries_62, \
Batch 329 — the fan-out coordination vocabulary, MODE SELECTION not error \
propagation). NotBenchable-by-domain (the entries_43-46 concurrency annotation class, \
bench_spec None): the cost is CORRECTNESS — double-writes breaking a serial-per-scope \
invariant under retry — not a Pareto axis. FIXTURE PROVENANCE: original std-thread \
stand-ins (never hindsight source), standalone rustc-compiled AND executed before \
landing. Wave: Batch 330 / Plan 338 / Research 368 (+ addendum). Cite: \
vectorize-io/hindsight @ 841979dfda4e1bebf7cbf68d0144d5d5454d93df — \
hindsight-api-slim/hindsight_api/engine/consolidation/consolidator.py \
(_gather_or_cancel L88-128), Batch 330 / Plan 338."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

//! Entries slice — Batch 331 (2026-11-05): one `rust_perf` concurrency rule
//! from the vectorize-io/hindsight round-2 pool (Plan 339 / Research 368's
//! follow-up pool).
//!
//! Source: vectorize-io/hindsight @ `841979dfda4e1bebf7cbf68d0144d5d5454d93df`
//! (MIT code), quotes re-verified at the pin at plan-339 authoring time; the
//! verbatim law quotes live in the rule's `context_notes` (the
//! doc-lazy-continuation class: quoted prose in `//!` breaks the paragraph
//! parse). Fixtures are ORIGINAL plain-Rust stand-ins (a provider-trait batch
//! runner; the law text names the source's Python face), standalone
//! rustc-compiled AND executed before landing (buggy arm: 3 identical
//! re-sends of a control signal + permanent flags from a transient, 3
//! byte-identical re-sends of a deterministic refusal; fixed arm: 1 call per
//! non-retry class, the ladder serving only the transport class — 5/5
//! census tests). Batch 331 / Plan 339 / Research 368 (+ addendum).

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── concurrency: a batch retry loop that re-sends the rebuilt
        // payload and writes durable failure state on terminal failure
        // WITHOUT classifying the error first — a provider control signal
        // (quota reset carrying retry_at) is a schedule, a deterministic
        // failure has nothing a byte-identical re-send can offer, and only
        // transport-shaped errors belong to the bounded ladder ───────
        // Batch 331 / vectorize-io/hindsight @ 841979df (consolidator.py
        // _classify_batch_failure + retain/fact_extraction.py's
        // permanent-type law).
        PerfRule {
            rule_id: "batch-retry-ladder-classifies-failure-before-permanent-state".to_string(),
            name: "a batch retry loop that catches EVERY failure into one posture — re-sends the rebuilt (possibly byte-identical) payload up the bounded ladder, then writes DURABLE per-item/per-batch failure state on exhaustion — turns a provider CONTROL SIGNAL (a quota/rate-limit reset that names when to resume) into identical re-sends plus a PERMANENT exclusion flag that permanently orphans the pending items, and burns the whole ladder on a DETERMINISTIC failure whose byte-identical replay earns the byte-identical refusal -> CLASSIFY the failure BEFORE acting: PROPAGATE a control signal at first sight (surface retry_at to the caller, the batch stays pending, NOTHING writes failure state), FAIL_FAST a deterministic/input-shaped failure (raise the permanent type immediately — zero re-sends; the recovery that varies input is the CALLER's bisection, never an identical re-send), and give the bounded ladder only the transport-shaped class".to_string(),
            category: "concurrency".to_string(),
            buggy_code: r#"// NAIVE: three failure classes, one posture — catch-all, re-send,
// then flag the items permanently on exhaustion.

use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum ProviderError {
    /// The provider's quota resets at this instant (a control signal).
    QuotaReset { retry_at: Instant },
    /// Deterministic: the same payload earns the same refusal.
    ContentRefused { chunk: usize },
    /// Transient transport noise — a re-send can succeed.
    TransportBlip,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: usize,
    pub payload: String,
    pub failed_at: Option<Instant>, // durable per-item failure state
}

pub struct Provider {
    pub calls: Vec<String>,
    pub fail_with: ProviderError,
    pub succeed_from_call: usize,
}

impl Provider {
    pub fn send(&mut self, payload: &str) -> Result<(), ProviderError> {
        self.calls.push(payload.to_string());
        if self.calls.len() >= self.succeed_from_call {
            return Ok(());
        }
        Err(self.fail_with.clone())
    }
}

/// The catch-all posture: re-send up to `attempts`, then flag.
pub fn run_batch(
    provider: &mut Provider,
    items: &mut [Item],
    attempts: usize,
    _wait: Duration,
) -> Result<(), String> {
    let payload = items.iter().map(|i| i.payload.as_str()).collect::<Vec<_>>().join("|");
    for _ in 0..attempts {
        match provider.send(&payload) {
            Ok(()) => return Ok(()),
            Err(e) => {
                let _ = e; // the classification that never happens
                continue; // identical re-send, EVERY class, same ladder
            }
        }
    }
    let now = Instant::now();
    for item in items.iter_mut() {
        item.failed_at = Some(now); // the PERMANENT exclusion, from ANY class
    }
    Err("batch failed".into())
}
"#
            .to_string(),
            fixed_code: r#"// GOVERNED: classify FIRST — the three-way ladder serves only the
// class a re-send can actually help.

use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum ProviderError {
    QuotaReset { retry_at: Instant },
    ContentRefused { chunk: usize },
    TransportBlip,
}

/// The permanent type a deterministic failure promotes to: the worker
/// fails the operation instead of burning the retry schedule.
#[derive(Debug, PartialEq)]
pub struct PermanentFailure {
    pub reason: String,
}

#[derive(Debug, PartialEq)]
pub enum Posture {
    /// Surface to the caller with WHEN — never write failure state.
    Propagate { retry_at: Instant },
    /// Raise the permanent type now — never re-send.
    FailFast(PermanentFailure),
    /// Transport-shaped: the bounded ladder applies.
    Retry,
}

pub fn classify(e: &ProviderError) -> Posture {
    match e {
        ProviderError::QuotaReset { retry_at } => Posture::Propagate { retry_at: *retry_at },
        ProviderError::ContentRefused { chunk } => Posture::FailFast(PermanentFailure {
            reason: format!("chunk {chunk} refused deterministically"),
        }),
        ProviderError::TransportBlip => Posture::Retry,
    }
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: usize,
    pub payload: String,
    pub failed_at: Option<Instant>,
}

pub struct Provider {
    pub calls: Vec<String>,
    pub fail_with: ProviderError,
    pub succeed_from_call: usize,
}

impl Provider {
    pub fn send(&mut self, payload: &str) -> Result<(), ProviderError> {
        self.calls.push(payload.to_string());
        if self.calls.len() >= self.succeed_from_call {
            return Ok(());
        }
        Err(self.fail_with.clone())
    }
}

#[derive(Debug, PartialEq)]
pub enum BatchOutcome {
    Done,
    /// The batch is pending, resumable at retry_at — no flags written.
    PendingUntil { retry_at: Instant },
    /// The permanent type: the caller bisects the input.
    Permanent(PermanentFailure),
    Exhausted,
}

pub fn run_batch(
    provider: &mut Provider,
    items: &mut [Item],
    attempts: usize,
    _wait: Duration,
) -> BatchOutcome {
    let payload = items.iter().map(|i| i.payload.as_str()).collect::<Vec<_>>().join("|");
    let mut ladders_left = attempts;
    loop {
        match provider.send(&payload) {
            Ok(()) => return BatchOutcome::Done,
            Err(e) => match classify(&e) {
                // PROPAGATE at FIRST SIGHT: no re-send, no flags.
                Posture::Propagate { retry_at } => {
                    return BatchOutcome::PendingUntil { retry_at };
                }
                // FAIL_FAST: the permanent type, no re-send.
                Posture::FailFast(p) => return BatchOutcome::Permanent(p),
                // RETRY: the only class the bounded ladder serves.
                Posture::Retry => {
                    ladders_left -= 1;
                    if ladders_left == 0 {
                        let now = Instant::now();
                        for item in items.iter_mut() {
                            item.failed_at = Some(now);
                        }
                        return BatchOutcome::Exhausted;
                    }
                }
            },
        }
    }
}
"#
            .to_string(),
            context_notes: Some(
                "THE LAW: a batch retry loop that re-sends a rebuilt (possibly byte-identical) \
payload and writes durable per-item failure state on exhaustion MUST classify the \
failure before acting — three classes, three postures. (1) PROPAGATE: a provider \
control signal (quota/rate-limit reset carrying WHEN to resume) is a SCHEDULE, never \
a failure — catching it locally both burns identical re-sends AND, worse, writes the \
permanent exclusion flag that permanently orphans the pending items. (2) FAIL_FAST: \
an input-shaped/deterministic failure reproduces byte-identically (the source's \
measured datum: 'twelve failures at the same character offset') — an identical \
re-send has nothing to offer; the recovery path that varies input is the caller's \
bisection, and deterministic failures raise their PERMANENT type so the worker skips \
the retry schedule ('One refused chunk is enough — the retain cannot succeed while \
it is in the batch'). (3) RETRY: transport-shaped only, bounded ladder. \
NotBenchable-by-domain (bench_spec None; the entries_43-46/entries_63 concurrency \
annotation class): the cost is CORRECTNESS + pipeline completion quality under \
quota churn — permanent orphaning of pending work — never a Pareto latency/alloc \
axis; the fixture's provider-call counts and flag-write censuses are deterministic \
EVIDENCE, not timing bars, and source-reported numbers are provenance, never bars \
(house law). SOURCE (vectorize-io/hindsight @ 841979dfda4e1bebf7cbf68d0144d5d5454d93df, \
MIT; every quote re-verified at the pin at plan-339 authoring time): \
consolidator.py L3773-3783 the _BatchFailureClass taxonomy (PROPAGATE / FAIL_FAST / \
RETRY) + L3786 _classify_batch_failure + the docstring, verbatim: the exclusion \
predicate (`consolidation_failed_at`) 'is cleared only by an explicit bank-wide reset, so a *transient* quota \
exhaustion permanently orphaned those facts' + input-shaped failures at temperature \
0 'the reporter measured twelve failures at the same character offset' — a \
byte-identical rebuild reproduces the byte-identical failure. THE SECOND SITE (the \
same law from the other side — the count-neutral face): \
engine/retain/fact_extraction.py L2787-2796 re-raises ProviderRateLimitResetError \
with 'retry_at = max(err.retry_at ...)' (the control signal carries WHEN) + \
L2799-2806, verbatim: 'A content-policy refusal is deterministic: the offending \
chunk earns the same refusal on every replay ... Re-raise the permanent type \
(rather than a generic RuntimeError) so the worker fails the operation immediately \
instead of burning a full retry schedule on it ... One refused chunk is enough.' \
THE RUST TWIN VOCABULARY: reqwest/ureq backoff wrappers that match err.status() == \
429 into a wait-with-retry_after arm and PERMANENT types for 4xx (never re-sent), \
the bounded ladder reserved for connect/timeout/reset-by-peer; the source's \
Python face names the same shape (rate-limit-reset as a control signal the \
scheduler consumes). REGIME: any caller loop that re-sends a rebuilt (possibly \
byte-identical) payload after catching a failure AND writes durable per-item or \
per-batch failure state (failed_at/exclusion rows, DLQ parking, permanent skip \
lists); upstream providers exposing quota-reset control signals or deterministic \
decode. ANTI-REGIME: all-transient transports with idempotent re-sends (one \
ladder for everything is fine); callers whose every retry already varies input \
(no identical re-send exists to waste); stateless callers writing no durable \
failure state (no orphan harm); temperature > 0 generation (the byte-identical \
premise fails — the FAIL_FAST half demotes, the control-signal half still stands). \
KINSHIPS (grep-verified at authoring): fanout-first-error-leaves-siblings-running \
(entries_63, Batch 330, THE SAME SOURCE REPO — that rule owns fan-out LIVENESS on \
the dispatch error path, cancel-then-await; this rule owns the retry-CLASSIFICATION \
after a failure is observed — composable, distinct halves of one source's error \
discipline); refusal-quarantines-entry-transient-passes (entries_54, B308 — the \
nearest kin and the divergence is sharp: it owns the RECEIVING family's per-entry \
fate (quarantine vs pass-through keeping state for retry); this rule owns the \
CALLER's retry posture at a batch seam plus the permanent-flag-from-transient \
corruption axis entries_54 does not carry); \
in-use-state-retry-under-serving-mark-keyed-on-refusal-cause (entries_45 — keys \
retry on refusal cause at SERVE time, vocabulary-adjacent, different axis); \
batch-miss-interleave-tiny-executor (entries_53 — batch vocabulary, executor \
interleave, unrelated). Dedup (rust_perf rule_ids, retry|ladder|backoff): five \
hits at authoring, none owns the three-way control-signal/fail-fast/retry ladder \
or the permanent-state harm. FIXTURE PROVENANCE: original provider-trait stand-ins \
(never hindsight source), standalone rustc-compiled AND executed before landing \
(buggy arm: 3 identical re-sends of the control signal + permanent flags on every \
item, 3 byte-identical re-sends of the deterministic refusal; fixed arm: 1 call \
per non-retry class with zero flags, the ladder recovering the blip — 5/5 census \
tests). Wave: Batch 331 / Plan 339 / Research 368 (+ addendum). Cite: \
vectorize-io/hindsight @ 841979dfda4e1bebf7cbf68d0144d5d5454d93df — \
hindsight-api-slim/hindsight_api/engine/consolidation/consolidator.py \
(_classify_batch_failure L3786) + engine/retain/fact_extraction.py L2787-2806, \
Batch 331 / Plan 339."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

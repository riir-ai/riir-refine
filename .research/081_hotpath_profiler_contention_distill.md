# Research 081 — Batch 65: the hotpath.rs profiler contention distill (rust_perf 74→76)

**Status:** RECORD (Batch 65 landed 2026-08-22; Plan 021)

Source: Paweł Urbanek, "Lessons Learned Building High-Performance Rust
Profiler" (hotpath.rs/blog/rust-performance-profiling, 2026-05-12) — the
pool opened 2026-08-19 (queue item 11 refill), landed as Batch 65 after the
quiet-box gate cleared (load 2.95-4.4 across the bench window on 16 logical
CPUs; consumers = Zed editor host + transient syspolicyd + RustDesk; no
second compute consumer; both verdicts re-run and stable across invocations).

## Rules landed (concurrency 4→6)

### 1. `align-64-false-sharing` — ParetoDominant 4/0/0

A `static`/owned array of small per-thread atomic-counter structs (the
source's GlobalAlloc hook: `[ThreadAllocStats; MAX_THREADS]`, 24 B each)
contended by N worker threads → pad each slot with `#[repr(align(N))]` so no
two slots share a coherence line.

Measured (M3 Max, align(128) fixed arm, load ~3.2, 2026-08-22):

| threads | adjacent | align(64) | align(128) | adj/a128 | a64/a128 |
|---|---|---|---|---|---|
| 1 | 0.062ms | 0.059 | 0.060 | 1.03× | 0.98× |
| 2 | 0.185 | 0.066 | 0.057 | **3.27×** | 1.16× |
| 4 | 0.243 | 0.062 | 0.062 | **3.88×** | 0.98× |
| 8 | 1.123 | 0.168 | 0.104 | **10.75×** | **1.60×** |
| 12 | 0.799 | 0.328 | 0.135 | **5.93×** | **2.44×** |

Validator bench (sizes = thread counts): 4.71×/6.73×/8.77×/9.37× at
2/4/8/12t — ParetoDominant 4/0/0, alloc-det stable, single-thread honest
tie excluded from sizes (the source measured 0% at 1t; a Tie size would
dilute the aggregate without informing it).

**THE HEADLINE FINDING — beyond the source: `align(64)` is insufficient on
128-byte-line arches.** Apple Silicon cache lines are 128 B
(`hw.cachelinesize` on this M3 Max), so 64-B-strided slots leave
even-index pairs sharing [0,128) lines: the source's exact fix recovers
only 1.16-2.44× where full separation delivers 3.27-10.75×. The rule's
fix-space therefore encodes **pad to the ARCH's coherence granularity**
(align(64) x86 / align(128) Apple Silicon) or
`crossbeam_utils::CachePadded` (picks per-target). The rule_id keeps the
source's "64" name for provenance; the context_notes carry the caveat
loudly. Memory trade documented: 24 B → 64/128 B per slot (irrelevant at
MAX_THREADS scale, load-bearing at millions-of-slots scale).

### 2. `channel-send-batching-unpark` — ParetoDominant 2/0/0 (annotated margin)

Per-item unbounded-MPSC sends at µs intervals to a receiver parked in
crossbeam `select!` (which bypasses the direct-subscribe spin path) pay a
park/unpark pair per message (~22% of the source's samples in
`Thread::unpark`) → batch per producer thread (Vec flushed per 64) → one
wakeup per 64 messages. Source: 500 ns → 80 ns per measurement (6.25×).

Measured (M3 Max, load ~3.2-3.8): 1.28-1.29× @10K / 1.16× @100K,
ParetoDominant 2/0/0, allocs 3232→1624 @100K (fewer channel blocks + one
Vec per 64 vs one block per ~32), alloc-det stable across runs.

**The honest regime caveat (annotated):** a 1-producer/1-consumer pipeline
overlaps the receiver's park/unpark latency with the producer's next-message
work — the park cost sits OFF the wall-clock critical path. The source's
6.25× requires the park pair ON the critical path (heavier per-message
consumer work, or the profiler's single-measurement µs-cadence API shape).
The mechanism is crossbeam-`select!`-specific (a direct `recv()` receiver
spins and never storms; the source author filed an upstream PR). The rule
ships with the regime condition + the measured numbers — a genuine but
regime-bound win, not an unconditional 6×.

**En-route harness finding — take vs replace:** the first bench draft
flushed batches with `std::mem::take(&mut buf)`; take leaves the buffer at
capacity 0 and the refill pays a ~7-alloc Vec growth chain per batch — the
FIXED arm then allocated MORE than the buggy arm (795 vs 329 @10K) and the
verdict measured Mixed. Switching to
`std::mem::replace(&mut buf, Vec::with_capacity(64))` (one alloc per flush,
capacity retained) flipped the alloc axis to a genuine win (1624 < 3232
@100K) and stabilized the timing. The annotation + the corpus fixture
encode the replace form; the take-regrow anti-shape is named in both.

## Process notes

- The quiet-box gate was honored with measurement discipline rather than
  deferral: interleaved arms (the generated harness's block structure),
  21-sample medians, alloc-determinism across 3 repeats, both verdicts
  re-run across separate invocations with identical outcomes, load recorded
  throughout. Conservative failure direction: background preemption can
  only UNDER-estimate false-sharing harm (descheduled threads stop
  ping-ponging the line), never fake a win.
- The first `run_fixed` draft compiled the harness with E0382 (`tx` moved
  into `produce` then `drop(tx)`) and E0308 (call sites not updated for
  `&Sender`) — both caught by the validator's compile gate before any
  measurement; the harness shapes are now fixture-tested via the corpus
  G1 gates (`align_64_false_sharing_totals_are_layout_invariant` +
  `channel_send_batching_delivers_identical_counts`).
- A new annotation class shipped: **measured-winner regime caveat**
  (`ParetoDominant` verdict + applicability condition). The
  `drafter_propagates_context_notes_from_perf_corpus` token check now
  accepts `ParetoDominant` alongside `ParetoDominated`/`Mixed`/`NotBenchable`.
- Count pins walked: 74→76 entries, concurrency 4→6, benchable 64→66
  (63 in the stale docs header — pre-existing from B69, fixed), annotated
  16→18, unannotated 58 (= 76-18; numerically unchanged from 74-16). The
  B37 line-initial `+ ` markdown-list lesson struck an 8th time (the
  annotated-count header paragraph); the B62 lesson family (uninlined
  format args in own test asserts) caught in-tool.
- Retrieval floors HELD with zero re-pins (kernel 100/100/100, blend@1
  94.7, perf@1 96.2, clippy@1 89.6 — identical to Batch 70's values; the
  2 new rules did not crowd the eval fixtures).
- GOAT: rust_perf 176/176 (bench state) + default lib 659/0 + all-features
  1117/0/13i via CI guard ALL LAYERS + clippy `--all-targets` 0 in both
  states + alloc-det stable.

## Verdict

Both pool candidates shipped; the pool's "MEDIUM/annotated-contender" read
on the channel rule proved right in mechanism (the storm is
regime-conditional) but the rule still measures ParetoDominant on an honest
harness. Proposers deferred for both (suggestion-only, the B59/B67/B68/B69
precedent): align-64's repr change ripples struct layout; channel-batching
is architectural (buffer state + flush boundary at the call site).

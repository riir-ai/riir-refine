# Research 180 — Alice Ryhl "Async: What is blocking?" distill verdict (async-perf source pool, source 1 of 3)

**Status:** RECORD — executed 2026-09-18 as Batch 153 ([Plan 143](../.plans/143_batch153_async_blocking_pool.md)): `blocking-sleep-in-async` + `rayon-bridge-oneshot` landed as staged; candidate 3 (`spawn-blocking-cpu-pool-choice`) adjudicated ANNOTATION as predicted (folded into `batch-blocking-pool-segment`'s anti-regimes); candidate 5's 10–100 µs annotation landed on `async-yield-point`; the pool folds (source-4 payload, source-5 nodelay rule + uring cite) composed into the same batch per the accumulation clause. All quotes re-verified live at mine time — every figure carried. Full landing record: HISTORY.md Batch 153.

> **Pool delta 2026-09-18 (post-verdict):** the pool the header names "1 of 3" grew to 5 and is now COMPLETE — source 4 (tokio.rs/blog/2020-04-preemption, MARGINAL C+, annotation payload staged for Batch 153) lives in the `.distill/001` pool row (`c4b5f0b5`), and source 5 (name-corrected: `carllerche/tokio-perf-experiments` @ `f3a6d3b8`, MARGINAL C+, the `tcp-nodelay` B− lead + two annotation-class leads staged for Batch 153 adjudication) closed it (`638f2681`). The header's "of 3" is the write-time count, superseded by the pool row.

## Source

- URL: <https://ryhl.io/blog/async-what-is-blocking/> ("Async: What is blocking?", 2020-12-21, Alice Ryhl — Tokio core maintainer)
- Inspection: 2026-09-18, live full-text fetch.
- Provenance chain: THE canonical upstream of the dial9 "Principles for fast Tokio applications" (Batch 152 / Research 179 records the lineage explicitly). Companion pool sources inspected in the same unit: tokio.rs "Bridging with sync code" topic page (live fetch) + `tokio-rs/mini-redis` @ `3d93b42bc363220f85af4fc9e1bebd35b588a4a3` (MIT, shallow clone via `.raw/`, removed after this note landed).

## Verdict

**YES, B+ — route `rust_perf` / `concurrency` (NOT kernel_opt, NOT clippy_lints).**

- **Lane-intel: NO workspace-lane mapping** (honest zero — the open lanes are CUDA prefill/decode kernel work; nothing here touches them).
- **Product-coverage: fires** — extends the world-facing tokio family (6 → 8 expected), and candidate 2 is the corpus's FIRST tokio×rayon bridge rule, covering the hyper/axum-world CPU-offload path. The famous-lib axis is the reason this grades YES on decision content despite NotBenchable metrics.
- Yield estimate: **2–3 rules after coordinator falsification** (recent batches land 1–3; no extraordinary-evidence claim).

## Ranked candidates

| # | Pattern (proposed rule_id) | Source anchor | Grade | Notes |
|---|---|---|---|---|
| 1 | `blocking-sleep-in-async` — `std::thread::sleep` (or any blocking wait) inside an async fn → `tokio::time::sleep(d).await` | the article's canonical example (`join!` of three 1 s sleepers runs 3 s serially → 1 s concurrent once the sleep suspends) | **B+** | THE canonical blocking bug; statically detectable (a specific call, not a runtime property); bounded fix; zero corpus hits under both vocabularies. The family's missing most-basic member — the corpus has yield-POINT rules but not the blocking-SLEEP replacement. |
| 2 | `rayon-bridge-oneshot` — blocking wait on rayon from async (inline `par_iter().sum()` / `join()` / `block_on(rayon_handle)` in an async body) → `rayon::spawn` + `tokio::sync::oneshot` + `.await` | "The main danger of using rayon is that you must be careful not to block the thread while waiting for rayon to complete" + the worked `parallel_sum` oneshot bridge; reinforced by tokio bridging docs: "you still need the `rayon::spawn` call when you use parallel iterators, because parallel iterators are blocking" | **B−** | First tokio×rayon bridge rule. Distinct from all four Lattimore rules BOTH directions (those are rayon-INTERNAL structure/deadlock; this is the async↔rayon boundary). Two-regime: small work stays inline (the article's own "unless the array is very very large, just computing a sum is probably cheap enough"). |
| 3 | `spawn-blocking-cpu-pool-choice` — heavy CPU-bound closure in `spawn_blocking` → rayon (pool sized #cores vs the ~512-thread blocking pool) | the cheat sheet: spawn_blocking "poorly suited for expensive CPU-bound computations" | **C+** | Two-regime placement decision, but weak static detectability (CPU-boundness unprovable from shape alone). Expected outcome: falsified as a standalone rule, folded as an annotation into `batch-blocking-pool-segment`'s context notes (its anti-regime (a) already borders this). |
| 4 | DROP — running-forever loop on `spawn_blocking` → dedicated thread | "Running forever" cheat-sheet column | kill | Positive instance: already encoded as anti-regime (c) of `batch-blocking-pool-segment` (verbatim: "a very long-lived segment starves one pool slot — a dedicated OS thread is the source's answer there"). |
| 5 | DROP — the 10–100 µs between-`.await` boundary | "a good rule of thumb is no more than 10 to 100 microseconds between each `.await`" | annotation | Not a rule — ANNOTATION upgrade material: `async-yield-point` + `adaptive-yield-consecutive-ready` context notes should carry the canonical quantitative regime boundary + this upstream cite. Fold into the batch's doc-touch task. |

## GOAT assessment

Criterion 1 fails for a DOMAIN reason on all candidates: the win metric is concurrent wall-clock / sibling-task latency under the scheduler (the `join!` 3 s → 1 s demo), which the Pareto validator cannot host — **NotBenchable, suggestion-only** (the Batch-136 precedent; the entire tokio family is graded this way). Criteria 2–4 pass (exact semantics: same duration/same result; magnitude story: scheduler swap-vs-block mechanism; deterministic fixtures). Per the two-class split, a domain-level limitation does NOT demote the source.

## Dedup evidence (both vocabularies, 2026-09-18)

- `sleep` / `thread::sleep` / blocking-wait → **zero** `rule_id` or fixture hits in `rust_perf` corpus (only hit repo-wide: `docker/buildcheck.rs` — the healer's own polling code, not a rule). rustc/clippy ship NO default lint for std sleep in async (clippy #8294 still open) — no clippy_lints collision either.
- `rayon` family → 4 rules, all Lattimore/Batch-136, all rayon-internal (`partitioned-mut-slices-zero-sync`, `dynamic-work-parallelism-ladder`, `sender-per-work-item-terminates-channel`, `nested-par-inside-channel-bridge-deadlocks`). None touches the async boundary. The bridge candidate is novel.
- `spawn_blocking` → only `batch-blocking-pool-segment` (loop-batching axis). Pool-choice and sleep axes are open.
- `async-yield-point` vs candidate 1: distinct trigger (await-less loop vs blocking-sleep call) AND distinct fix (insert yield vs replace sleep) — both directions clear.

## Companion source verdicts (pool sources 2 + 3 — MARGINAL, recorded here so the pool row can close)

- **tokio.rs "Bridging with sync code": MARGINAL C+.** Architectural prescriptions (current_thread runtime for sequential `block_on` wrappers; background-runtime actor + `blocking_send`; `Handle` cloning) — API construction, not bounded buggy→fixed perf shapes. Its one load-bearing line ("parallel iterators are blocking") folds into candidate 2's cite chain. No files owed beyond this paragraph.
- **`tokio-rs/mini-redis` @ `3d93b42b` (MIT): MARGINAL C+ as a mining source.** It is the REFERENCE IMPLEMENTATION of rules the corpus already has: `server.rs` is the canonical accept-loop semaphore idiom (`acquire_owned` BEFORE `accept`, permit moved into the spawned task, dropped at completion — the cleanest "how" for `bounded-fanout-semaphore`'s annotation); `db.rs` is the std-Mutex-short-section anti-regime taught in-source ("no async ops while holding + very small critical sections → std mutex, NOT tokio mutex"); `connection.rs` is check-then-parse incremental decode + BufWriter (buffered-io family). No novel buggy→fixed pairs beyond what dial9 already carried. Value = annotation cites + the fixture-adjacency precedent already in use ("mini-redis-adjacent shape, never verbatim").

## Caveats

- 2020 article: the blocking-pool cap is stated "around 500" threads; dial9 (2026) reports the pool DEFAULTS to ~#cores and grows toward the 512 cap. Carry BOTH as source-reported with dates; neither is our measurement.
- No measured numbers in the article (the 3 s → 1 s demo is illustrative); all magnitude stories are mechanism-level, consistent with the NotBenchable grading.
- Audience is async newcomers — the mechanisms are timeless, but any quantitative claim must be attributed, never adopted as ours.
- mini-redis' `Handler::run` comment notes pipelining is NOT implemented there (the pipelining material lives in dial9, already landed as `adaptive-yield-consecutive-ready`) — no drift risk, recorded to preempt a re-read.

## Queue slot

Staged for the next open batch window (nominal **Batch 153**): lead with `blocking-sleep-in-async` (B+), carry `rayon-bridge-oneshot` (B−), adjudicate `spawn-blocking-cpu-pool-choice` (C+) standalone-vs-annotation at mine time, and fold the 10–100 µs annotation upgrade into the batch's doc-touch task. Follow-up pool continuation named in the queue row: the Tokio blog "Reducing tail latencies with automatic cooperative task yielding" (the article's own closing recommendation — the runtime-side sibling of `adaptive-yield-consecutive-ready`).

# Research 032 — hotpath-rs Profiling Guide Distillation: Deterministic-Signal Bench Axis + 4 rust_perf Rule Candidates

**Date:** 2026-08-14
**Source:** [hotpath.rs/blog/profiling-rust-guide](https://hotpath.rs/blog/profiling-rust-guide) — "Profiling Rust: The Complete Guide - From SQL Queries to CPU Sampling", Paweł Urbanek, published 2026-08-11. A tool-vendor guide (hotpath-rs profiler) walking 7 profiling layers (SQL N+1, HTTP sequential→parallel, I/O throughput, lock wait/acquire split, channel queue depth, allocation counting, CPU sampling), each with a before/after fixture measured by the author's own tool.
**Target:** `riir-clippy` — `rust_perf` domain (corpus + Pareto bench validator). Not a paper; an engineering blog. Classified as rule-distillation + methodology input, not novel research.
**Verdict:** **Gain** → `Issue 014`. Not Super-GOAT (Q1 prior art exists — see §2; Q2/Q3 no new behavior class or product selling point; this is internal tooling).

## 1. What the guide covers (author's measured numbers)

| Layer | Antipattern | Fix | Author's delta |
|---|---|---|---|
| SQL | N+1 per-row SELECT (21 queries) | JOIN + GROUP BY (1 query) | 104.29µs → 70.08µs (~1.5×) + query count 21→1 |
| HTTP | Sequential independent awaits | `tokio::try_join!` | 873.30ms → 394.93ms (~2×, = max not sum) |
| I/O | Brotli encoder misconfig | config tweak | 27.9 KB/s → 1.6 MB/s (~57×), resp 126-230ms → <9ms |
| Locks | Write guard held across HTTP await | fetch before `write().await` | reader P95 1.11s → 9.42µs (>100,000×); write acquire 1.23s → 1.74µs |
| Channels | Unbounded channel, consumer 5× slower | faster consumer (2ms vs 25ms processing) | max queue 73→1; send→recv latency 1.03s → 2.33µs |
| Allocations | `String` clone per message × 5M | `Arc<str>` | 4.8 GB → 25.1 KB alloc traffic; send 98ns → 68ns (~1.3×) |
| CPU | `Regex::new` per line × 10K | `static LazyLock<Regex>` | is_valid 163.17µs → 283ns per call (~555×); runtime 2.15s → 1.13s |

## 2. The transferable primitive: deterministic signals over noisy timing

The guide's core methodology claim (its most valuable line for us):

> "allocated bytes are a deterministic signal: the same code path allocates the same number of bytes on every run, making it a reliable metric for catching regressions" — and conversely, wall-clock shows "deviations of up to 10%-15% on shared GitHub CI runners", so "if a function appears 15% faster but none of the more deterministic signals changed — it may simply reflect run-to-run or hardware variation."

Deterministic counters named: SQL query count, allocation bytes/count, channel max-queue-depth. Timing is explicitly demoted to a *secondary* signal.

**Prior art (checked — NOT novel):** dhat-rs ships heap-usage testing ("write tests and then check that they allocated as much heap memory as you expected"); dev-bench does baseline-gated regression detection. **Our delta is integration, not invention:** adding an alloc-count/bytes axis to the `rust_perf_bench` Pareto validator, which today measures wall-clock only.

**Why it matters here (documented gap it fills):**
- `bench.rs` L12: "microbenchmarks are inherently noisy (mitigated via warmup + dynamic calibration + median-of-samples, but still a measurement, not a proof)".
- Bench 015: 2 rules annotated regime-fragile; Bench 016: 2 Mixed — all timing-verdict fragility.
- Sibling prior art already in the workspace: riir-mmorpg-examples' G4 gates (thread-local `CountingAllocator`, `EVENT_ALLOC_BUDGET`) prove the pattern in production shape. This is a fusion: G4 CountingAllocator ↔ dhat-style counting ↔ our Pareto validator.

## 3. Rule candidates (4)

Per Bench 013 precedent (15/30 candidates empirically rejected BEFORE corpus addition), every candidate below must pass our own bench or be honestly dropped. Delta vs shipped corpus verified by grep — none exist today.

### 3.1 `regex-hoist-lazylock` (caching, benchable — strongest candidate)
`Regex::new` per call in a loop → `static LazyLock<Regex>`. Distinct from `precompute-table` (ParetoDominated 0.20-0.40× — compiler emits POPCNT for that shape); regex compilation is a runtime call the compiler cannot elide — the guide's 555× per-call gap is structural. Harness already supports crate deps via `// [dep: regex = "1"]` markers. Expected: ParetoDominant. No clippy lint exists for this (rust-lang/clippy#587 only ever shipped `trivial_regex`; compile-once remains forum folk knowledge).

### 3.2 `arc-str-payload` (allocation, benchable — needs §4 axis first)
Clone-per-message of an immutable payload across a channel → `Arc<str>` (clone = refcount bump). Timing delta is modest (~1.3×) — the load-bearing signal is the deterministic alloc axis: 4.8 GB → 25.1 KB (~190,000× fewer bytes, bit-reproducible). Distinct from `avoid-clone` (generic elision; this is a type change enabling O(1) clone) and `box-str-immutable` (drops the capacity word; not about multi-clone sharing). **Order issue tasks so the alloc axis lands before this rule's gate** — timing alone may Tie.

### 3.3 `tokio-guard-across-await` (concurrency, NotBenchable candidate)
Guard acquired before a slow await, released after → fetch first, guard only around the state mutation. The victim's latency (other tasks' wait time) is the metric — same class as Bench 013's rejected `park-vs-spin` (metric mismatch: CPU-time vs wall-clock) → NotBenchable with `context_notes`, like `bevy-reactive-loop`. **Scope note:** clippy's `await_holding_lock` already covers **std** guards; the delta is tokio/parking_lot/async-lock guards (Send-by-design, unflagged by clippy; a niche `tokio_mutex_lint` crate covers tokio Mutex only — our pattern adds the RwLock write-guard + fix shape).

### 3.4 `bounded-channel-backpressure` (concurrency, NotBenchable candidate)
`unbounded_channel` + send-in-loop with a slower consumer → bounded channel (backpressure). Metric is queue depth / memory growth (OOM bound), not time. Semantics-changing fix (blocking vs buffering) → `context_notes` mandatory ("prefer bounded when consumer cadence is unknown; bounds memory"). Ships as a pattern rule alongside `async-future-size`.

## 4. Fusion — alloc axis design sketch (for Issue 014 T1)

- Inject a thread-local counting `GlobalAllocator` into the generated bench `main.rs`; buggy/fixed each run under it.
- Report `alloc_count`/`alloc_bytes` per side as a **deterministic second axis** (exact integers, run-invariant — assert equality across samples; any variance = harness bug, not noise).
- Verdict logic unchanged (timing stays the Pareto axis); alloc axis is reported + recorded in the bench doc, and can gate rules whose timing is Tie-class (3.2).
- Precedent shapes: dhat heap-usage testing; riir-mmorpg-examples `g4_*_alloc_free_steady_state` CountingAllocator.

## 5. Non-actionable observations (recorded, no files)

- **Layer ROI ordering** (SQL/HTTP/IO/locks/channels BEFORE CPU micro-opts): validates why a static code healer lives in the last layer — higher-layer wins need runtime context (queries issued, endpoints called) we cannot see in `*.rs` spans. Scope confirmation, not a gap.
- **Exclusive vs inclusive attribution** (CPU samples exclusive vs wall-clock inclusive): our harness benches leaf functions in isolation (already exclusive). Matters only if the validator ever grows call-tree awareness — out of scope.
- **hotpath Diff strategic echo**: the author's hosted product bets on "AI agents need perf constraints + fast feedback" — the same thesis as our `rust_perf_verify` + bench validator. Commercial validation of direction; not actionable.

## 6. Honest caveats

- Vendor numbers: the author's own tool on toy fixtures (20 posts, 3 endpoints, 10K lines). Our bench must independently verify every rule candidate; the guide's ratios are priors, not evidence.
- The two NotBenchable candidates cannot be gated by our harness by construction — they enter the corpus as pattern rules with annotations, the honest class Bench 013 established.

## References

- `Issue 014` — execution
- Bench 013/015/016/017 — Pareto validator methodology + rejection precedent
- dhat-rs (nnethercote) — heap-usage testing prior art
- riir-mmorpg-examples G4 gates — in-workspace CountingAllocator precedent

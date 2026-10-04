# Research 234 — hotpath-rs Repo-Code Distill (3rd hotpath pass): Fast Clock, Random-Draw Sampling, SPSC Drain

**Status:** DISTILLED — pending owner decision (staged behind B201 + nnethercote + the 10-03 fork pool; conversion opens the plan per the PRISMML precedent). §8 conversion-prep addendum (2026-10-04 22:2x +07): C4 drop-overturn adjudicated POSITIVE; C1 clock bench smoke-prevalidated on M3 at 2.83× (official calm-box run pending); B65 cross-reference rider drafted; B201 plan-counter collision re-verified.

**Date:** 2026-10-04
**Source:** [pawurb/hotpath-rs](https://github.com/pawurb/hotpath-rs) @ `ac4096ec8f3c8515bededbeafb0f34a47ed9367b` (MIT, shallow clone, tip = the same day's readme update — pin load-bearing; `.raw/hotpath-rs` deleted at close, every cite below is re-checkable via `git show ac4096e:<path>`).
**Inspected by:** the M3 session (direct reads — coordinator + a 3-round Claude verdict ping-pong, `#Verdict: REVISE → REVISE → AGREE`; the rounds corrected 3 provenance errors + surfaced 2 missed priors, all verified against the tree before acceptance).

## 0. Prior passes on this source (MANDATORY context — the third mining)

This is the **third** hotpath-source distill. The prior-source check initially missed because the August queue rows were folded out of the live snapshot into `.research/108` / the fold archives — the process lesson is recorded in §6.

| Prior | Source form | Landed |
|---|---|---|
| **Research 032** (2026-08-14) | the hotpath.rs *guide* blog ("Profiling Rust: The Complete Guide") | `regex-hoist-lazylock`, `arc-str-payload`, `tokio-guard-across-await`, `bounded-channel-backpressure` + the deterministic-alloc-axis fusion (Issue 014) |
| **Research 081 / Batch 65 / Plan 021** (2026-08-22) | the *profiler blog* ("Lessons Learned Building High-Performance Rust Profiler") | `align-64-false-sharing` (the profiler's own `[ThreadAllocStats; MAX_THREADS]` static) + `channel-send-batching-unpark` (the profiler's own measurement channel, 500ns→80ns) |
| **THIS pass** | the **repo code** at a pinned SHA (post-0.28 engine) | candidates below |

This pass differs in kind: the blog era described v0.1-class internals; the repo has since moved past **one** of the shipped rules' fixes (the channel rule — see C3). `align-64-false-sharing`'s fix is STILL live at the pin (`crates/hotpath-meta/src/lib_on/functions/alloc/core.rs:30-31`, `#[repr(align(64))] pub(crate) struct ThreadAllocStats`, `MAX_THREADS = 256` at :15) — and the source kept align(64), never adopting align(128): supporting our Apple Silicon 128B-line annotation (the source isn't aware of it; not a supersession).

**Product-coverage wording (corrected at verdict round 1):** NOT a family seed — this **extends the hotpath-sourced profiler-internals pair (B65) with the timing, sampling, and transport axes**. The hotpath source family now spans guide → blog → repo.

## 1. What the repo is

~38k-line core: `#[measure]` macros, CPU/alloc/async/lock/channel/SQL/HTTP/axum instrumentation, Prometheus + TUI + MCP. 1.8k★, 1,177 commits, active. The mineable surface is the **observer's own hot path** — the code whose product IS minimal observation overhead. The repo carries an unusually honest `docs/src/profiling_overhead.md` with measured per-op costs (M2 Pro, 100k iters, ±10ns noise): functions ~40ns/call, parking_lot Mutex +29ns … std RwLock write +66ns per cycle, crossbeam +47ns … tokio mpsc +88ns per send/recv, futures ~125ns/poll, streams ~30ns/item; zero-off measured at −0.5ns (noise); methodology: "hammers a single instrumented codepath in a tight, uncontended loop and compares it against an uninstrumented baseline in the same run... ~1 µs of spin work to emulate a realistic function body". These tables are DATA/provenance for the candidates' magnitude stories, not rules themselves.

## 2. Ranked candidates (dedup greps run under both vocabularies — evidence per candidate)

### C1 `timing-hot-path-direct-counter-read` — B+ GOAT-pending (the lead)

`crates/hotpath/src/instant.rs`: a platform-tiered `Instant` — `quanta::Instant` on Linux, a hand-rolled `mach_absolute_time` FFI wrapper on macOS, `std::time::Instant` elsewhere. The doc comment carries the measured claim: "On macOS `std::time::Instant` goes through `clock_gettime(CLOCK_UPTIME_RAW)` (~25ns per call), while reading the counter directly costs ~7ns with the same resolution. Tick-to-nanosecond conversion uses the exact `mach_timebase_info` ratio." The overhead doc's refined figures: **6.1 ns vs 17.2 ns** per `now()`, "most measurements need two of them". Provenance arc: CHANGELOG:859 "Use custom Instant implementation on macos" + :1733 "Always use quanta::Instant on linux". The in-repo `benchmark_instant.rs` is a clean 10M-iteration `black_box(Instant::now())` A/B between both clocks (std as the outer timer).

- **Benchable**: YES — our M3 Pareto validator, macOS arm via `extern "C"` (no new deps); ~2.8× claimed on a ~17ns op. GOAT-pending until the M3 run confirms.
- **Regime**: per-call timing paths where the clock read is material (profilers, metrics, tracing, samplers). **Anti-regime**: ms-scale coarse timing — read cost is noise.
- **MANDATORY caveats (verdict round 2)**: (a) PLATFORM SCOPE — dep-free only on the macOS arm; Linux needs `quanta` or raw TSC, and a TSC read needs an invariant-TSC/VM-stability anti-regime; (b) UNSAFE annotation — the fix arm is an `unsafe extern` FFI call; (c) CLOCK SEMANTICS — `mach_absolute_time` is uptime-class (does not advance during sleep; ~24MHz tick on Apple Silicon): ONLY the call cost changes, not resolution or semantics — the rule must never read as a precision rule. Exact `timebase_info` ratio conversion with a u128 fallback on `checked_mul` overflow (instant.rs `ticks_to_ns`).
- **Dedup**: zero corpus hits for `quanta|mach_absolute|coarse clock|clock read|rdtsc`; the 5 `Instant::now` hits are all bench/measurement-gate code. NOVEL.

### C2 `sample-durations-random-draw-not-stride` — B+ (KEEP)

`crates/hotpath/src/lib_on/sampling.rs` + the guard/state plumbing: counts stay exact for every event; **durations** are sampled by per-call independent random draw — "Every decision is an independent random draw from a per-thread generator, so periodic workloads cannot lock onto a fixed phase" (module doc). Rate 0.0 = count-only mode skipping the clock reads entirely ("the sampler cuts the two `Instant::now()` calls that bracket each measurement - the biggest single cost on the hot path", overhead doc); display reports exact-when-full vs extrapolated (`display_total_poll_duration_ns`: "Exact when every poll was timed, extrapolated (`avg * polls`) under time sampling"); denominator (wrapper) guards never sampled ("their exact total is the % denominator", timing/guard.rs:5); teardown-safe wyrand seed ("No thread-local or OS entropy access, so it is safe during thread teardown" — atomic counter ^ wallclock, sampling.rs `seed()`); resolution once at guard build.

- **Provenance arc**: CHANGELOG:168 "Randomize time sampling logic" — the anti-phase-lock property was a FIX, not the initial design.
- **NotBenchable decision-content** (the fix IS the sampling design; the B136/B177 precedent class).
- **Kinship — a genuine TWO-REGIME PAIR with the MTPLX rule (B145, entries_06.rs:285)**: fixed stride when a *decision* must reproduce run-to-run ("the answer must not move run to run... or an A/B could not attribute a delta"); random draw when a *statistic* must not phase-lock to the workload. The pair composes like seed-warm vs cold-l3 (regime decides the polarity).
- **Dedup**: the only `sampl*` corpus hits are the MTPLX stride clause + incidental bench prose. NOVEL.

### C3 `per-thread-spsc-chunk-drain-single-sweeper` — B− (successor-framed)

`crates/hotpath-drain/src/lib.rs` (299 lines, the extracted drain crate): per-thread chunked SPSC queues (CHUNK_SIZE=64, linked chunks = unbounded), "it appends events into fixed-size chunks with plain stores and publishes them with a single `Release` store of the chunk's `len` — **no mutex, no RMW atomic on the hot path**" (the source's own doc wording — NOT "zero cross-thread coordination": the registry holds a `Mutex<Vec<Arc<EventQueue<M>>>>` at lib.rs:210, taken at thread registration and during sweeps, never on the event hot path). Single consumer per registry "sweeps all registered queues periodically: it `Acquire`-loads each chunk's `len`, reads the published prefix, and frees fully consumed chunks"; "Producer and consumer touch disjoint slot ranges by construction, so a queue is safe to drain at any moment — including queues of threads parked at shutdown, which is what guarantees a complete final report"; `MAX_CHUNKS_PER_SWEEP = 1024` — "so a producer outpacing the consumer cannot pin the sweep in an endless tail chase. Leftovers are picked up on the next tick."

- **HISTORY (corrected at verdict round 2 — the arc is not one-directional)**: lock-free (early) → mutex-default + opt-in `hotpath-lockless` (CHANGELOG:682, filed under **🐛 Bug Fixes** — the demotion was labelled a fix, so the early lock-free presumably had a correctness problem; *inference, not a measurement — keep hedged*) → the extracted `hotpath-drain` crate unconditionally lock-free at the pin (no `hotpath-lockless` feature in any manifest at the pin). Evidence for "became default" = the code at the pin.
- **CONSEQUENCE (the rule's load-bearing form)**: the documented safety invariants — only the owning thread writes its tail chunk's slots/`len`/`next`; the consumer reads only below the published `len` and frees only after full consumption + observing `next`; single consumer per registry — ride as **REQUIRED CONDITIONS of the fix, not commentary**. The demotion-as-bugfix is the evidence the invariants are load-bearing.
- **Successor framing (verdict round 1)**: this SUPERSEDES the fix arm of `channel-send-batching-unpark` (B65) **in the same source** — the blog-era fix was batching sends on the shared measurement channel; the repo's final answer removes the channel per-producer (sweeper polls every 50ms). Lands EITHER as a distinct successor rule with the regime split (per-thread SPSC + periodic sweep when the consumer tolerates ~50ms latency; batched shared-channel when consumers must wake immediately — the llama.cpp quantizer rider's regime) OR as a rider; **either way `channel-send-batching-unpark`'s `context_notes` gains the cross-reference so the corpus never presents the superseded fix as the end state**.
- **Dedup**: `papaya-lockfree` = shared lock-free map (CAS on hot path — this has none); `channel-send-batching-unpark` = the predecessor fix (above). NOVEL as the successor rung.

### C4 `thread-local-attribution-not-across-await` — CONDITIONAL B− (drop-overturn required)

The B65 pool DROPPED this shape with a recorded reason (`.research/108`): "RAII-guard async-fn attribution (correctness-of-measurement, profiler-specific — instrument-at-poll is hotpath's implementation detail, not a general code pattern)". The overturn path (verdict round 1): recast as the GENERAL rule — **"thread-local context set by a guard must not live across `.await`; only compiler-guaranteed await-free sync bodies may scope it"** — covering tracing spans, request-ID middleware, allocator scopes (the interleaved-task context-bleed class), NOT instrument-at-poll. The repo's evidence: `functions.rs:109-113` "Never caller-scoped: a block expression can contain `.await` inside an async function, so its guard may live across suspension points and threads where a thread-local caller scope would leak to interleaved tasks. Only sync function bodies (which the compiler guarantees are await-free) get caller scoping"; cross-thread drop detection (timing/guard.rs Drop: tid compare, skip the caller-stack pop, report tid=None); per-poll `CallerScopeGuard::enter` in futures/wrapper.rs:159/325. **If the conversion session cannot sustain the general framing, it stays dropped (the default).**

### Riders

- **`SuspendAllocTracking` recursion guard** (C+): the observer suspends its own alloc tracking around its event sends (futures.rs `send_future_event`, the poll wrapper's event block) — self-instrumentation recursion prevention. Zero corpus hits.
- **`zero-cost-off pass-through allocator posture`** (C+): `lib_off.rs` compiles every macro to a no-op (`measure_block!` → `{{ $expr }}`) AND ships a pure pass-through `CountingAllocator` so user `#[global_allocator]` declarations compile unchanged with the feature disabled — the measured −0.5ns delta is the discipline. Zero corpus hits; rider-grade (the measured-zero claim is the content).

### Drops (with reasons)

Per-lock/per-channel overhead tables (DATA — provenance for C1/C2 magnitude stories); hdrhistogram consumption (upstream library); wyrand PRNG body (ambient — "one add and one 64x64→128 multiply"); report/TUI/Prometheus/MCP/JSON formatting (ambient structure); `TID_ORPHANED_BASE` synthetic-identity design (correctness nicety, no bounded fix-space); SQL/HTTP attribution machinery generally (the interesting half IS C4).

## 3. Yield estimate

**2-3 rules + 2 riders** after ~50% coordinator falsification: C1 + C2 near-certain; C3 likely (successor framing + the B65 cross-reference rider); C4 conditional on the drop-overturn.

## 4. Lane-intel + product-coverage axes

**Lane-intel: NONE.** Not a serving recipe or kernel source — no league cells move; no open-issue mechanism-keyword matches. A tool-adoption aside only (not a distill artifact): hotpath could profile the healer's own scan/match hot path (~40ns/call overhead, `--all-targets`-aware), and its `hotpath init --agent claude` flow is an interesting distribution pattern for developer tooling.

**Product-coverage: the hotpath family extends** (§0) — the observer-side instrumentation surface (metrics/tracing/profiling code) is a broad real-world class the corpus now covers via 2 rules + these candidates, complementing the bench-side measurement-honesty family (B109/B177/B181/B182/B186).

## 5. Caveats

- Third mining of the source — the easy content is taken; this pass lives entirely in the post-0.28 repo internals the blogs predate.
- The two design rules (C2/C3) are NotBenchable decision-content; only C1 benches.
- C1's magnitude is source-measured on M2 Pro — re-measured on M3 at bench time; platform-conditional (macOS arm is the dep-free one).
- Shallow clone (grafted) — CHANGELOG used for arc provenance, the pin SHA is the record; every cite above is `git show ac4096e:<path>`-recoverable.
- The early-lock-free-had-a-correctness-problem reading is an INFERENCE from the 🐛 Bug Fixes heading, not a measurement — keep hedged in-rule.

## 6. Process lesson (recorded for the distill skill's next source check)

The prior-source check missed because the live queue snapshot's 3-day window (post-09-27 fold) no longer carries the August hotpath rows — they live in `.research/108_mining_queue_snapshot_history.md` and the fold archives. **Prior-source greps must include the fold archives (`.research/108` + `.research/206`/`.research/232`), not just the live snapshot + the corpus.** Caught by the verdict reviewer at round 1; verified against the tree before acceptance.

## 7. Queue slot

Staged behind **B201 (in flight on shikuwa/4090 — the Issue-665 yield)** → nnethercote (R134) → the 10-03 fork pool. **Conversion opens the plan (plan-first Protocol A step 0) at that slot** — no `.plans/` number is allocated now because B201's session holds an uncommitted `.plans/.highwater` bump on the 4090 and a same-file edit from this box is exactly the dual-allocation collision the gate exists to catch. The clock rule benches on M3 at conversion. Nothing invented — every quote read at the pin.

## 8. Conversion-prep addendum (2026-10-04 22:2x +07 — the "do remains" pass; zero-collision prep while B201 holds the plan counter)

### (a) Queue/collision state re-verified (dated)

shikuwa/4090 `E:\git\riir-refine` at 22:0x +07: `.plans/.highwater` = **200, uncommitted**; `.plans/200_batch201_mlxfast_mtp_slice.md` + `src/domains/kernel_opt/corpus/entries_gpu_70.rs` untracked; `.raw/mlxfast-b201` present; zero cargo/rustc/clippy processes live. B201 is unchanged-stalled since 10-04 00:08 — the plan-number allocation stays BLOCKED, and the conversion session allocates its plan number from the then-current `.highwater` AFTER B201 lands (dual_allocation_gate run at allocation, per the standing law).

### (b) C4 drop-overturn — ADJUDICATED POSITIVE (the recast holds; author as the GENERAL rule)

Dedup greps re-run 2026-10-04 against the live corpus (179 rust_perf rules):

- `tokio-guard-across-await` (entries_02.rs:419) — LOCK-scoped: parking_lot/tokio/async-lock `MutexGuard` across `.await`, hazard = **contention** (other lockers blocked); its own note says clippy `await_holding_lock` covers std guards only.
- `host-lock-across-reentrant-blocking-call` (entries_08.rs, B174) — LOCK-scoped, hazard = **re-entrant self-deadlock**; carries its own DISTINCT-FROM against the tokio rule.
- The corpus's ONLY `thread_local` surface is `vec-reuse-across-lifetime-change` (entries_06.rs bench spec, Batch 135 — allocation family, buffer reuse).
- ZERO corpus hits under `task_local` / `task-local` / `instrument()` / `!Send` vocabularies.

The recast (`thread-local context set by a guard must not live across .await`) is a **THIRD failure-mode axis**: WRONG-THREAD context — the guard/binding belongs to the WORKER THREAD, not the task; after migration across `.await` the context is another thread's (or the borrow unsound). No lock is involved; no lint oracle exists for the value-copy shape (the Send bound catches only the guard shape, and only when the future must be `Send` — `block_on`/`LocalSet`/`spawn_local` are silent). Bounded fix-space, 4 shapes: pass the VALUE not the guard / re-enter `with()` after the await / `tokio::task_local!` for task-scoped context / per-poll enter (tracing `instrument()`) instead of holding `enter()`. Ecosystem reach: tracing's `Span::enter()`-across-await is the canonical documented footgun; request-ID middleware; thread-local allocator scopes. The B65-era drop reason ("instrument-at-poll is hotpath's implementation detail") does not touch the general framing — the recast is about the GUARD/CONTEXT shape, not the poll wiring. Source instance stays functions.rs:109-113 (the compiler-guaranteed-await-free-scoping law) + the cross-thread Drop tid-compare (timing/guard.rs). Naming suggestion for the batch: `thread-local-context-not-across-await` (general) with attribution as the named instance — final call at authoring.

### (c) C1 bench harness PREPARED + SMOKE-PREVALIDATED (NOT the official gate number)

`/tmp/hotpath_clock_bench/{main.rs,run.sh}` — zero-dep (`rustc -O --edition 2021`), mirroring the source's `benchmark_instant.rs` (10M `black_box` reads, std outer timer) + this workspace's interleaved position-balanced 7-round discipline with min+median (the sequential_ab_timing law). Full source embedded in §8.1 below (/tmp is ephemeral).

**Smoke run 2026-10-04 22:21 +07 — NON-QUOTABLE (load average 13.99, no preflight; mechanism-confirmation only):** std `Instant::now()` min 18.56 / median 21.63 ns; direct `mach_absolute_time` min 6.57 / median 6.96 ns; **ratio 2.83× (min/min), 3.11× (median/median)**; mach timebase 125/3. The direct arm is load-insensitive (a register read — 6.57..7.36 across rounds under load 14), and the magnitude class REPRODUCES the source's M2 Pro figures (6.1 vs 17.2 = 2.8×). **GOAT-pending → GOAT-prevalidated; the official gate = calm-box re-run (load < ~4) + the riir-reflex `scripts/bench_preflight.sh` PROVENANCE line quoted beside the number, then the in-repo `bench_spec` at authoring.**

### (d) B65 cross-reference rider — DRAFTED (lands with C3 at conversion; placement entries_02.rs:488-489, appended to the `name` field per the B113 rider precedent)

> "SUPERSEDED-IN-SOURCE (Batch NNN, hotpath-rs repo @ ac4096e — the post-0.28 engine's final answer for its OWN measurement channel): per-thread chunked SPSC queues + a single 50ms sweeper (`per-thread-spsc-chunk-drain-single-sweeper`) replace the batched shared channel — no mutex, no RMW atomic on the event hot path. REGIME SPLIT: per-thread SPSC + periodic sweep when the consumer tolerates ~50ms latency; THIS rule (batched shared-channel sends) when consumers must wake immediately — the llama.cpp quantizer rider's regime."

### (e) Conversion checklist as it now stands

Open plan (after B201 frees the counter) → author entries: C1 (benchable + `bench_spec`, carrying the three verdict-round caveats: platform scope / unsafe-FFI annotation / clock-semantics-not-precision clause) + C2 (decision-content, the two-regime pair with the MTPLX stride rule) + C3 (successor-framed + rider (d)) + C4 (the general rule per (b)) + the 2 C+ riders → standing gates + count pins (rust_perf 179→182/183, annotated count walk) → GOAT: G1 parse/G2 lib/G3 clippy ×3 postures + the calm-box C1 run with preflight provenance → research-note status flip + docs/AGENTS/queue-snapshot updates → commit+push. Estimated yield after the (b) upgrade: **3-4 rules + 2 riders + 1 rider-edit**.

### 8.1 Bench source (verbatim, reproducible)

```rust
// (embedded from /tmp/hotpath_clock_bench/main.rs — the runnable copy is
// ephemeral; rebuild from this block)
use std::hint::black_box;
use std::time::Instant;

mod sys {
    #[repr(C)]
    pub struct Timebase { pub numer: u32, pub denom: u32 }
    extern "C" {
        pub fn mach_absolute_time() -> u64;
        pub fn mach_timebase_info(info: *mut Timebase) -> i32;
    }
}

const N: u64 = 10_000_000;
const ROUNDS: usize = 7;

fn time_std() -> f64 {
    let t0 = Instant::now();
    for _ in 0..N { black_box(Instant::now()); }
    t0.elapsed().as_nanos() as f64 / N as f64
}

fn time_mach() -> f64 {
    let t0 = Instant::now();
    for _ in 0..N {
        // SAFETY: mach_absolute_time is a pure counter read (no locks, no
        // allocation); calling it from any thread is always sound.
        black_box(unsafe { sys::mach_absolute_time() });
    }
    t0.elapsed().as_nanos() as f64 / N as f64
}

fn main() {
    unsafe {
        let mut tb = sys::Timebase { numer: 0, denom: 0 };
        assert_eq!(sys::mach_timebase_info(&mut tb), 0);
        println!("# mach timebase numer/denom = {}/{} (ns per tick)", tb.numer, tb.denom);
    }
    println!("# N={} reads/round, {} position-balanced interleaved rounds", N, ROUNDS);
    println!("# source figures (M2 Pro, hotpath overhead doc): mach 6.1ns vs std 17.2ns");
    let mut std_ns: Vec<f64> = Vec::with_capacity(ROUNDS);
    let mut mach_ns: Vec<f64> = Vec::with_capacity(ROUNDS);
    for r in 0..ROUNDS {
        let (a, b) = if r % 2 == 0 {
            (time_std(), time_mach())
        } else {
            let b = time_mach();
            let a = time_std();
            (a, b)
        };
        std_ns.push(a);
        mach_ns.push(b);
        println!("round {}: std {:>6.2} ns/op | direct {:>6.2} ns/op", r, a, b);
    }
    let stat = |v: &Vec<f64>| {
        let mut s = v.clone();
        s.sort_by(|x, y| x.partial_cmp(y).unwrap());
        (s[0], s[s.len() / 2])
    };
    let (smin, smed) = stat(&std_ns);
    let (mmin, mmed) = stat(&mach_ns);
    println!("----");
    println!("std   Instant::now(): min {:>6.2}  median {:>6.2} ns/read", smin, smed);
    println!("direct mach_absolute: min {:>6.2}  median {:>6.2} ns/read", mmin, mmed);
    println!("ratio min/min {:.3}x  median/median {:.3}x", smin / mmin, smed / mmed);
}
```

Run law: `rustc -O --edition 2021 main.rs -o bench && ./bench`, CALM BOX ONLY, with `sh ../riir-reflex/scripts/bench_preflight.sh` provenance quoted beside any published figure.

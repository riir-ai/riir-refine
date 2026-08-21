# Plan 021 — Batch 65: rust_perf contention rules (hotpath.rs distill)

Source: Paweł Urbanek, "Lessons Learned Building High-Performance Rust
Profiler" (hotpath.rs/blog/rust-performance-profiling, 2026-05-12). Pool
opened 2026-08-19 (queue item 11 refill); this batch lands it.

Two rules, both concurrency category:

1. `align-64-false-sharing` (STRONG) — `static`/owned array of small
   per-thread atomic-counter structs (24B each → several per cache line)
   contended by N worker threads → pad each slot with `#[repr(align(N))]`
   so no two slots share a coherence line. Source numbers (x86): 2t 70%,
   4t 72%, 8t 86%, 12t 85%; single-thread 0% (honest).
2. `channel-send-batching-unpark` (MEDIUM, annotated-contender) —
   per-item MPSC sends at µs intervals to a receiver parked in
   crossbeam `select!` pay a park/unpark pair per message (~22% of
   samples in `Thread::unpark`) → batch sends per producer thread
   (per-thread Vec drained on flush). Source: 500ns → 80ns per
   measurement (6.25×). Mechanism is crossbeam-select!-specific;
   pre-authorized to land Mixed/context_notes-annotated if the harness
   cannot reproduce the storm portably.

## Load-bearing box facts (measured at batch open, 2026-08-22)

- `hw.cachelinesize = 128` on this M3 Max. **The source's
  `#[repr(align(64))]` does NOT separate adjacent 24B slots on 128B-line
  arches** (slots 0 and 1 at 64B stride share the [0,128) line). T1
  probes this; the corpus fixture's fixed arm + annotation must encode
  the honest per-arch padding (align(64) x86 / align(128) Apple Silicon,
  or `crossbeam_utils::CachePadded` which handles it per-target).
- Box quietness at batch open: load 2.95/3.79/4.03 on 16 logical CPUs;
  consumers = Zed Dev (editor host, exempt class), syspolicyd (transient),
  RustDesk 30%. No second compute consumer. Multi-threaded contention
  benches run with: interleaved arms (the generated harness measures
  buggy block then fixed block, 21-sample medians), load recorded in the
  research note, conservative failure direction (background preemption
  can only UNDER-estimate false-sharing harm — threads stop ping-ponging
  the line while descheduled — never fake a win). Suspicious results →
  re-run before claiming.

## Tasks

- [x] T1 — 3-arm manual probe (adjacent / align(64) / align(128)) ×
      threads {1,2,4,8,12} on M3: **adjacent/align128 = 3.27×/3.88×/10.75×/5.93×
      at 2/4/8/12t (1t = 1.03× tie, honest); align(64) recovers only
      1.16×/0.98×/1.60×/2.44× of it — the 128B-line finding is MEASURED**
      (slots at 64B stride still share [0,128) lines on Apple Silicon).
      Corpus harness work: ITERS=10K (30K RMW/thread) → adjacent@8t ≈
      1.1ms/call, perfect for the ~1ms calibration target.
- [x] T2 — Corpus entries: both rules appended to the concurrency
      cluster; fixtures mirror the hotpath.rs GlobalAlloc-hook shape;
      fixed arm per T1 verdict (align(128) + the arch caveat);
      context_notes per bench verdicts (both annotated — align-64's
      arch-granularity caveat + channel's pipeline-overlap regime; the
      first measured-winner annotation class).
- [x] T3 — Bench specs: `align_64_false_sharing_bench_spec` (sizes =
      thread counts [2,4,8,12], ITERS=10K × 3 RMW/thread, scoped
      spawn-per-call, closed-form checksum G1) +
      `channel_send_batching_unpark_bench_spec` ([dep: crossbeam-channel
      = "0.5"], receiver parked in select! over 2 channels, per-item vs
      per-64-batch, item-count G1; flush via mem::replace — take-regrow
      measured 795-vs-329 allocs @10K WORSE than buggy, the anti-shape
      named in the rule).
- [x] T4 — Pareto bench run (release, load 2.95-4.4 recorded):
      align-64 **ParetoDominant 4/0/0** (4.71/6.73/8.77/9.37× at
      2/4/8/12t) + channel **ParetoDominant 2/0/0** (1.28/1.16×) — both
      re-run across separate invocations, verdicts stable; alloc-det
      stable 3×.
- [x] T5 — Standing gates: corpus counts 74→76 / concurrency 4→6 /
      benchable 64→66 / annotated 16→18 (unannotated stays 58 = 76−18);
      seventy-four→seventy-six + sixty-four→sixty-six renames; stale
      headers fixed (73→76, the L157 "69 entries", docs 74/63/16 →
      76/66/18); 2 new G1 gates
      (`align_64_false_sharing_totals_are_layout_invariant` +
      `channel_send_batching_delivers_identical_counts`); the drafter
      token check extended to accept `ParetoDominant` (the measured-winner
      annotation class).
- [x] T6 — GOAT: rust_perf 176/176 (bench state) + default lib 659/0 +
      all-features 1117/0/13i via **CI guard ALL LAYERS** + clippy
      `--all-targets` 0 in BOTH states (7 own-edited warnings fixed: the
      B37 line-initial `+ ` lesson struck an 8th time + 2 uninlined
      format args + too_many_lines allow) + retrieval floors HELD zero
      re-pins (kernel 100/100/100, blend@1 94.7, perf@1 96.2, clippy@1
      89.6 — identical to Batch 70's values).
- [x] T7 — Docs: Research 081 note, AGENTS.md (73/16-of-73 stale →
      76/18-of-76 + the Batch 65 record + the feature-row update),
      `.docs/11_domains/rust_perf.md` (counts + 2 table rows + the Batch
      65 history line), queue snapshot (pool → LANDED + progress corpus
      counts), this plan.
- [ ] T8 — Commit `feat: batch 65 — rust_perf contention rules (hotpath
      distill)` + push. Proposers deferred (suggestion-only, the
      B59/B67/B68/B69 precedent — align-64's repr change is
      layout-rippling, channel-batching is architectural).

## Non-goals

- No proposers for either rule (documented deferral above).
- No thread-affinity pinning in the harness (no portable macOS tooling;
  scheduler randomness is symmetric across arms).
- The park-storm's crossbeam-upstream PR context is annotation material,
  not a rule condition.

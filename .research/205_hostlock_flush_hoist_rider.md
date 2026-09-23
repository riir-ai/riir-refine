# 205 — Batch 174: the host-lock release-before-blocking-call + the deferred-command flush-hoist + the CubeCL double-checked-lookup rider

**Status:** RECORD (Batch 174 landed 2026-09-24 ~00:3x +07, Plan 168 COMPLETE).

## Sources (all merged — quotes stable, re-verified against the merged diffs at mine time)

1. **tracel-ai/cubecl #1678** "fix(throughput): release the probe pool lock before the device call" — merged @ `9d1314c070b998008959db021abcc23228ab1eb8` (= tag v0.11.0-pre.4, closes #1668). Diff: `crates/cubecl-std/src/throughput/{base.rs,pooling.rs}`.
2. **tracel-ai/cubecl #1686** "fix(throughput): probe the device with nothing else running on it" — merged @ `8ccbde3f51e5e5e9474ff9fb1d6b75975537cb8e`. Diff: `crates/cubecl-runtime/src/{client.rs,throughput/benchmarker.rs}` + `cubecl-std/src/throughput/base.rs` + the throughput example.
3. **bevyengine/bevy #25851** "Only flush the world after despawning all entities in `despawn_all`" — merged @ `1fb979e37365ef6e96fec085830796bd2312679a`. Diff: `crates/bevy_ecs/src/world/{entity_access/world_mut.rs,mod.rs}`.

## The two rules + the rider

### `host-lock-across-reentrant-blocking-call` (rust_perf, concurrency, NotBenchable)

The mechanism (verified verbatim): `PooledProbes::cleanup_unless_held` held the global `POOLED_PROBES` lock ACROSS `client.memory_cleanup()` — which submits to the runner/device thread and waits for drain, and the runner TAKES THE SAME LOCK when it autotunes → mutual wait, process never recovers. The fix shape: decide under the lock (`held_by` read inside a scoped guard), DROP the guard, THEN issue the release. Upstream doc line, quoted in the entry: "`release` runs with the lock dropped: it blocks on the device thread, which takes this lock itself when it autotunes."

Sub-decisions carried in the annotation:
- **The `probed` flag** — cleanup only when a probe RAN ("Only a probe leaves pools with the allocator"); a cache ANSWER means nothing was allocated to release. Upstream measured 3 redundant cleanups per `roofline_bounds` per autotuning thread (the queue-filler that made the latent deadlock reachable).
- **The lock-drop regression test** — `a_release_is_issued_with_the_lock_dropped` spawns a thread that takes the lock WHILE the release runs (the drop proven by construction); `a_running_sweep_is_not_released` pins the decide-half.

NotBenchable reasoning: deadlock/hang class (the B172 `O_NONBLOCK` head-probe precedent) — the buggy arm's cost is a process-wedging mutual wait; a bench either hangs or measures nothing.

Dedup: distinct-from `tokio-guard-across-await` (entries_02) — same fix SHAPE, different failure mode (await-yield contention, clippy `await_holding_lock` partial oracle) vs re-entrant SELF-DEADLOCK (no lint oracle; the call-chain re-entry is invisible statically). kernel_opt's deadlock family is GPU-side; this fills the host-side gap.

### `deferred-command-flush-hoist-out-of-batch-loop` (rust_perf, concurrency, BENCHABLE)

The mechanism (verified verbatim): `despawn_no_free_with_caller` split into `despawn_no_free_no_flush_with_caller` + a flushing wrapper; the `despawn_entities` retain loop calls the NO-FLUSH variant per entity and issues ONE `world.flush()` after the loop (after `free_many`). Source-measured −2.4%..−10.6% end-to-end on bevy's own benches.

**Measured (Batch 174 benches, 4090/Windows release):** ParetoDominant 3/0/0 — **21.3×/13.3×/16.5× at n=1k/10k/100k** (run 2: 20.7×/18.0×/17.7× — verdict-stable ×2). Alloc axis EXACTLY FLAT (3/3 allocs, equal bytes at every size — the drain is in-place, the queue pre-reserved: pure-timing isolation).

**The bench-honesty arc (recorded for the next harness author):** the toy's fixed-cost model took THREE iterations to get honest —
1. A static-const hook table FOLDED at `-O` (constant-propagated; ratio 0.80× — the dead-result-elimination class the corpus already carries as a rule).
2. A runtime Vec built from a pure map still constant-propagated through inlining (ratio ~1.5× but partly eliminated) — black_box the CONSTRUCTION.
3. The model choice, adjudicated between two bounds: a vectorizable linear hook-sum reads ~1-2ns/flush (ratio 1.1-1.6×); a data-DEPENDENT dispatch chain (~35ns/flush measured, unfoldable + unvectorizable) reads the ratios above. The chained model ships — bevy's real end-to-end (2.4-10.6% on ~µs/entity work) implies a real per-flush fixed cost of tens of ns, which the chain matches and the linear sum does not. BOTH bounds are disclosed in the entry's annotation; the adoption-citation rule: cite the app-layer number, never the toy's magnitude.

The annotation also carries the SEMANTIC CONTRACT (hoisting is sound only where no observer needs partial mid-batch state — the source's own justification) and the FIX SPACE (World::flush/Commands::flush sites inside batch loops). Distinct-from `bufwriter-batched` + `channel-send-batching-unpark`: the batch-the-barrier family over three different barriers (write() / futex-unpark / deferred-command application).

### The rider (kernel_opt B56 `concurrent-context-init-needs-process-wide-lock`)

Appended to `gpu_concept` (KernelRule has no context_notes field): the #1686 second-site instance of the double-checked lookup — `measure_throughput` peeks `cached(key)` first ("A cached answer takes neither"), on a miss takes the device exclusively and asks AGAIN inside ("Asked again inside: another thread may have answered while this one queued"). Plus (a) the probe closure moved inside the exclusive section with its dry-run guard, and (b) the exclusivity rationale ("one sharing it measures its share rather than the peak") — upstream confirmation of this workspace's own GPU-exclusivity discipline (Bench 649's 12.3× swing class), recorded as lane intel.

Lane-intel also recorded, not a rule: our own hand-rolled `riir-gpu` autotune bench (`gemv_autotune`) shares the measure-under-contention exposure class on a multi-tenant box (the 02:4x 09-23 note stands).

## Gates

- G1 parse/count: 102 rust_perf corpus tests green (pins 151→153 / 95→96 / 56→57 / 96→98 ×2 sites / 55×2 sites + the `context_notes_backward_compatible_serde` third annotated site + `batch174_rules_present_and_parse` needles + syn-parse).
- G2 full lib: 2220/0 at default features.
- G3 clippy `-D warnings`: default ✓ / `rust_perf,rust_perf_bench` ✓ / `kernel_heal` ✓ (the rider's quote-escaping compile break caught + fixed at gate time — inner `"` needed `\"` in the Rust literal).
- Pareto: ParetoDominant 3/0/0 verdict-stable ×2 (numbers above); the `batch174_pareto_benches` #[ignore]d gate ran green.
- Kernel retrieval re-measure (Issue 017, kernel corpus touched): 18/18 green, floors hold.
- Score-bench row #162: heal 96.36% / created 1.82% — FLAT vs the 96.2/≤1.9 floors (no decay).
- Freeze tables: UNMOVED by derivation (zero clippy_lints files touched — the B172 law).
- The mod.rs corpus doc count repaired in passing (149→153 + allocation 26→28 + concurrency 21→23 — B173 had left it at 149).

## Drops / non-rules

- #1686's measure-under-contention half — upstream confirmation of our own GPU-exclusivity discipline; lane intel only.
- cubecl #1696 "Remove NoMemoryEffect from sync ops" — compiler-internal IR metadata (the #1687 class), graded DRY pre-batch.

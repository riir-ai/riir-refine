// Batch 413 — the memory-backpressure blog distill's rust_perf half (Plan
// 412 / Research 446): totodore.github.io/memory-backpressure/ ("Memory-
// bounded systems and backpressure in Rust", published 2026-11-22, POST-ONLY
// — the live-fetch URL the pin per the B353/B363/B366 precedent; every quote
// below re-verified BYTE-EXACT against the live page 2026-11-24, and the
// blog's "Allocator trait stabilized in Rust 1.100" claim CONFIRMED upstream
// (Rust 1.100.0, 2026-11-12, the rust-lang blog + releases.rs + rust-lang/
// rust #163811 "Stabilize custom allocators") — carried as verified, with
// the Vec/Box-only detail attributed to the author).
//
// rust_perf 400 → 406: six rules, ALL measurement-honesty, ALL suggestion-
// only NotBenchable-by-domain (the win metric is a correct census / a gate
// firing at the right quantity / a bound that holds — no validator prices
// bounded-memory; the dial9/136 annotation posture, never a demotion):
// - `allocator-census-blind-to-unprefixed-c-deps` (the A− lead) — the
//   tikv-jemallocator default PREFIXED symbols leave every C dep's malloc
//   uninterposed: any allocator-stat-derived bound undercounts exactly the
//   heavy-compression paths; verify the interposition (a canary) before any
//   census-derived number is trusted.
// - `thp-mappings-invisible-to-allocator-resident` — THP `always`
//   consolidates pages behind the allocator: `resident` stops seeing them
//   AND RSS stops falling; MALLOC_CONF=thp:never + the boot check.
// - `sampled-gauge-extremum-between-sparse-exports` — a gauge sampled faster
//   than it exports carries last/min/max per window; each arm's read-and-
//   reset falls back to LAST on an empty window, never the sentinel.
// - `reactive-memory-gate-needs-hard-backstop` — a sampled-metric gate lags
//   TWICE (between-sample growth, slow dirty-page fall); pair it with a hard
//   per-request bound, never stand alone.
// - `jemalloc-narenas-scales-with-host-not-cgroup-quota` — default narenas =
//   4 × host affinity (quota-blind); the epoch walk that refreshes the
//   census scales with the HOST; size explicitly from the quota.
// - `body-limit-before-decompression-bounds-the-wrong-size` — layer order
//   + the inner default shadow + the Content-Length undercount: bound the
//   DECOMPRESSED size, reserve the worst case on a chunked body.
//
// + ONE count-neutral face on `bounded-channel-backpressure` (entries_02):
//   the pull-model corollary (stop pulling while over budget; the stream is
//   the buffer) + the load_shed reject-vs-queue note — rides in its
//   context_notes, no new row.
//
// The two families this seeds: tower·axum admission (0 rules before this
// batch) and jemalloc·allocator-observability (0 rules before this batch).
// Dedup at landing HEAD (the Batch-409 lesson — re-grepped THIS session):
// six axes unowned; `memory-headroom-cgroup-honest-not-host-counter` still
// STAGED (not landed — R5 keeps its own row); docker/rules.rs:886 is the
// musl-allocator CHOICE annotation, not a census law; sec's unbounded_read
// decompression riders own the DECODE-transparency axis, not the admission-
// quantity axis. No workspace repo runs jemallocator — the rules fire at
// consumers' repos (the product-coverage posture, applied in the correct
// direction).
// NOTHING INVENTED — every quote read off the live page this session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 413 — the memory-backpressure blog `rust_perf` rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b413_rules() -> Vec<PerfRule> {
    vec![
        r1_allocator_census_blind_to_unprefixed_c_deps(),
        r2_thp_mappings_invisible_to_allocator_resident(),
        r3_sampled_gauge_extremum_between_sparse_exports(),
        r4_reactive_memory_gate_needs_hard_backstop(),
        r5_jemalloc_narenas_scales_with_host_not_cgroup_quota(),
        r6_body_limit_before_decompression_bounds_the_wrong_size(),
    ]
}

// ---------------------------------------------------------------------------
// Rule 1 — allocator-census-blind-to-unprefixed-c-deps (the A− lead)
// ---------------------------------------------------------------------------
#[must_use]
fn r1_allocator_census_blind_to_unprefixed_c_deps() -> PerfRule {
    let name = "An allocator-stat census left at the allocator crate's DEFAULT (prefixed-symbols) posture never interposes the C deps' malloc family — every C dependency (the source names zstd) keeps the libc allocator, the census is blind to exactly those bytes, and any allocator-stat-derived memory bound undercounts precisely the heavy-compression/foreign-allocator paths; enable the unprefixed-interposition feature the platform supports AND verify the interposition (a canary that allocates through the C dep and asserts the census moved) before any census-derived number is trusted".to_string();
    let notes = "Grade A−, suggestion-only, NotBenchable-by-domain (the win metric is a census that counts everything — no validator hosts it). SOURCE: totodore.github.io/memory-backpressure/ (2026-11-22, POST-ONLY — the URL the pin, quotes re-verified byte-exact at the live page 2026-11-24): 'the jemallocator crate defaults to prefixed symbols, so it won't override the `malloc` family of functions in the resulting binary. If you have any C dependencies like zstd for compression/decompression, it will continue to use the default libc allocator. This is a big issue because zstd's own allocations won't be tracked by jemalloc. Hence the usage estimation will be invalid. To circumvent this, don't forget to enable the `unprefixed_malloc_on_supported_platforms` feature flag in your `Cargo.toml`!' THE LAW: an allocator-stat census sees ONLY what the interposed allocator saw — a bound derived from it is valid exactly over the interposed surface, so the interposition must COVER the tree (or the bound must be derived from a counter that does, e.g. cgroup memory.current) and the coverage must be VERIFIED (the canary), never assumed from a Cargo.toml line. THE CANARY IS LOAD-BEARING: it converts a silent undercount into a boot refusal — the class the loud-verdict doctrine owns. ADJUDICATION (dedup at landing HEAD, vocabularies jemalloc|narenas|allocator+census — the ONLY hits are docker/rules.rs:886,923, the musl-allocator ANNOTATION rule: that rule owns the CHOICE of a small static allocator for the shipped image, THIS owns the census COMPLETENESS of whatever allocator was chosen — compose, never merge; entries_39's counting allocator is FIXTURE INSTRUMENTATION, not a law; token-count-from-authoritative-counter-not-stream-chunks entries_84 owns the UNIT axis of a measurement, THIS owns the COVERAGE axis — compose). REGIME: any service whose memory bound is derived from allocator stats (jemalloc/mimalloc/custom) while ANY C dep links in the tree (compression codecs, TLS, media parsers). ANTI-REGIME: pure-Rust trees (census complete by construction — the canary trivially passes); bounds read from OS/cgroup counters (never allocator-scoped). LANE NOTE (honest fires-at-home): no workspace repo runs an allocator-stat census today — this fires at consumers' services; riir-refine's own escalation-chain memory posture is the named internal consumer of the PAIRING half (reactive-memory-gate-needs-hard-backstop, same batch). Fixtures b413_perf F1 (the census-blind arm counts the undercount exactly; the canary arm DETECTS the prefixed build at boot; the interposed arm passes; the pure-Rust anti-regime). Plan 412 / Research 446. Batch 413.".to_string();
    PerfRule {
        rule_id: "allocator-census-blind-to-unprefixed-c-deps".to_string(),
        name,
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: swap in the allocator crate at its DEFAULT feature set and
// trust its stats as "the whole heap" — the C deps never told you
// they kept their own allocator.
#[global_allocator]
static GLOBAL: AllocatorCrateDefault = AllocatorCrateDefault::default();

fn memory_bound_ok(resident_from_allocator: u64, max_memory: u64) -> bool {
    // resident_from_allocator CANNOT see the C compression library's
    // buffers: its calls went to the libc malloc family, which the
    // prefixed-symbol build never interposed
    resident_from_allocator < max_memory
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the unprefixed-interposition feature ON (the platform gate
// the crate names) + the boot canary that PROVES the interposition
// before any census-derived number is served.
fn interposition_canary(census: &mut Census, c_dep: &mut CompressionDep, probe: usize) -> Result<(), Boot> {
    let before = census.live_bytes();
    c_dep.compress_alloc(probe);
    let saw_it = census.live_bytes() == before + probe as u64;
    c_dep.compress_dealloc(probe);
    if saw_it {
        Ok(())
    } else {
        // the C dep's bytes are invisible to the census: every
        // allocator-stat-derived bound undercounts — refuse at boot
        Err(Boot::CensusBlindToCDeps)
    }
}"#
        .to_string(),
        context_notes: Some(notes),
        // Not benchable: census-coverage honesty; no validator hosts it.
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 2 — thp-mappings-invisible-to-allocator-resident
// ---------------------------------------------------------------------------
#[must_use]
fn r2_thp_mappings_invisible_to_allocator_resident() -> PerfRule {
    let name = "Transparent Huge Pages set to `always` consolidate the allocator's pages behind it: the allocator's own `resident` counter stops seeing the consolidated memory AND the process RSS stops falling even under much smaller traffic (dirty huge pages pin) — check the host setting at boot, refuse or demote it on the allocator's mappings (the allocator's thp:never knob; the unprefixed build reads the _RJEM-prefixed variable), and never calibrate a memory bound from `resident` while the setting is `always`".to_string();
    let notes = "Grade B+, suggestion-only, NotBenchable-by-domain (counter-honesty; no validator hosts it). SOURCE: totodore.github.io/memory-backpressure/ (2026-11-22, POST-ONLY, quotes re-verified byte-exact at the live page 2026-11-24): 'the main cause of our weird memory behavior is THP or Transparent Huge Pages! This Linux feature might be enabled at the OS level by default... it might increase the memory usage drastically (because a small 4 KiB allocation might be moved to a 2 MiB page) and more importantly, it won't be tracked as _resident_ memory by jemalloc. This only happens when the host setting is `always`. You can check it with `cat /sys/kernel/mm/transparent_hugepage/enabled`.' + 'even with traffic being much smaller, the RSS won't reduce because most of it consists of huge pages. The best solution is to disable this feature on the allocator's memory with a `madvise` syscall. Jemalloc will do that for its own mappings if you specify `MALLOC_CONF=thp:never`. Without the unprefixed feature flag, the variable is `_RJEM_MALLOC_CONF`.' THE LAW has TWO teeth: (a) OBSERVATION — a THP-consolidated page is invisible to the allocator's own counters, so any allocator-stat calibration is invalid while the setting is `always`; (b) LIVENESS — dirty huge pages pin RSS, so the service stops RELEASING (the reactive gate's slow-fall arm feeds on this). The PAGE-SIZE axis of the counter-divergence family. ADJUDICATION (dedup at landing HEAD): adjacent-DISTINCT from `macos-pressure-reads-footprint-not-rss` (entries_70 — the OS-COUNTER CHOICE axis: which process counter to read; THIS an allocator-counter VISIBILITY axis created by a kernel remap — compose: both read the honest counter) and from `gpu-residency-summed-beyond-process-and-cgroup-accounting` (entries_84 — the DEVICE-RESIDENCY axis: bytes outside the process the counters cannot see; THIS bytes the process owns that a remap HIDES from the allocator's own model — compose, both directions in-rule). REGIME: Linux hosts with THP `always` (common cloud defaults), allocator-stat-driven memory bounds, RSS-follows-load expectations. ANTI-REGIME: THP off/`madvise` hosts; bounds read from cgroup memory.current (the kernel counts hugepages it charged). Fixtures b413_perf F2 (the consolidated run leaves the counter while the OS view grows; the demoted arm re-aligns; the boot check refuses `always` fail-loud; the no-remapper anti-regime). Plan 412 / Research 446. Batch 413.".to_string();
    PerfRule {
        rule_id: "thp-mappings-invisible-to-allocator-resident".to_string(),
        name,
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: calibrate the memory bound from the allocator's resident
// counter and trust RSS to follow the load — on a THP=always host the
// counter is blind and the RSS is pinned.
fn max_memory(resident: u64, limit: u64, base: u64, factor: f64) -> u64 {
    // calibrated on a bench box; on a THP=always host `resident` never
    // sees the consolidated pages, so the calibration undercounts AND
    // the RSS refuses to fall after the load ends
    ((limit - base) as f64 * factor) as u64 + resident
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the boot check refuses the `always` posture fail-loud; the
// allocator's own mappings are demoted (its thp knob); the bound is
// calibrated only where counter and OS view agree.
fn boot_check_thp(enabled_file: &str) -> Result<(), Boot> {
    if enabled_file.trim() == "always" {
        Err(Boot::TransparentHugepagesAlways)
    } else {
        Ok(())
    }
    // plus: MALLOC_CONF=thp:never (unprefixed builds read the
    // _RJEM-prefixed variable) demotes THP on the allocator's mappings
}"#
        .to_string(),
        context_notes: Some(notes),
        // Not benchable: counter-visibility honesty; no validator hosts it.
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 3 — sampled-gauge-extremum-between-sparse-exports
// ---------------------------------------------------------------------------
#[must_use]
fn r3_sampled_gauge_extremum_between_sparse_exports() -> PerfRule {
    let name = "A gauge sampled MORE often than it is exported loses every inter-export excursion unless the sampler carries last/min/max for the window — and each arm's read-and-reset must fall back to the LAST value when the window recorded NOTHING (min armed with the max sentinel, max armed with the zero sentinel: a naive swap prints a zero trough or a fake peak); the windowed extremum is the honest export, the single sample is the anti-regime".to_string();
    let notes = "Grade B, suggestion-only, NotBenchable-by-domain (measurement-window honesty; ~30-line transcribable shape). SOURCE: totodore.github.io/memory-backpressure/ (2026-11-22, POST-ONLY, quotes re-verified byte-exact at the live page 2026-11-24): 'As the metric export interval is much higher than jemalloc advance calls (60s vs. 1s), we also need to track the minimum and maximum values during the collection interval of each metric' + the shipped Sample shape (atomic last/min/max; the take with the per-arm empty sentinel and the LAST fallback). THE LOAD-BEARING SUBTLETY is the empty-window fallback: read-and-reset must distinguish 'no sample in the window' from 'the value was the sentinel' — the min arm swaps OUT u64::MAX and the max arm swaps OUT 0, each falling back to LAST; a naive swap(0) on min prints a zero trough, a naive swap(u64::MAX) on max prints a fake peak — both are manufactured readings. ADJUDICATION (dedup at landing HEAD, vocabularies fetch_min|fetch_max|extremum over rust_perf — ZERO owners; token-count-from-authoritative-counter-not-stream-chunks entries_84 owns the UNIT axis, THIS the WINDOW axis — compose; benchmark-spread-above-threshold-reports-unstable entries_70 owns the stability VERDICT of a measured session, THIS the retention of excursions between exports — compose). REGIME: any exporter slower than its sampler (OTEL 60 s / scrape 15 s vs per-second sampling), memory gauges, latency gauges, queue-depth gauges. ANTI-REGIME: export == sample cadence (the window is never sparse — one cell suffices). Fixtures b413_perf F3 (the spike survives the sparse export; BOTH empty-window arms fall back to LAST; the sentinel resets after each export; the cadence anti-regime). Plan 412 / Research 446. Batch 413.".to_string();
    PerfRule {
        rule_id: "sampled-gauge-extremum-between-sparse-exports".to_string(),
        name,
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: sample into one cell, export whatever it holds — the spike
// between two exports is gone.
struct Gauge {
    current: AtomicU64,
}

fn sample(g: &Gauge, value: u64) {
    g.current.store(value, Ordering::Relaxed);
}

fn export(g: &Gauge) -> u64 {
    g.current.load(Ordering::Relaxed) // the 9,999 spike at t+1s is
                                      // invisible at the t+60s export
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the window carries last/min/max (atomics; read-and-reset);
// an empty window falls back to LAST, never the sentinel.
struct GaugeWindow {
    last: AtomicU64,
    min: AtomicU64, // empty sentinel: u64::MAX
    max: AtomicU64, // empty sentinel: 0
}

fn sample(g: &GaugeWindow, value: u64) {
    g.last.store(value, Ordering::Relaxed);
    g.min.fetch_min(value, Ordering::Relaxed);
    g.max.fetch_max(value, Ordering::Relaxed);
}

fn take(extremum: &AtomicU64, empty: u64, last: &AtomicU64) -> u64 {
    match extremum.swap(empty, Ordering::Relaxed) {
        v if v == empty => last.load(Ordering::Relaxed), // fall back to LAST
        v => v,
    }
}

fn export(g: &GaugeWindow) -> (u64, u64, u64) {
    (
        g.last.load(Ordering::Relaxed),
        take(&g.min, u64::MAX, &g.last),
        take(&g.max, 0, &g.last),
    )
}"#
        .to_string(),
        context_notes: Some(notes),
        // Not benchable: window-retention honesty; no validator hosts it.
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 4 — reactive-memory-gate-needs-hard-backstop
// ---------------------------------------------------------------------------
#[must_use]
fn r4_reactive_memory_gate_needs_hard_backstop() -> PerfRule {
    let name = "A memory gate driven by a SAMPLED metric lags twice — growth between samples outruns any fixed margin (each burst request is legal against the stale sample), and the signal falls SLOWLY after load ends (freed memory sits in dirty pages, so the gate stays shut while the service starves) — so the reactive global bound is PAIRED with a hard per-request bound (a byte budget or concurrency limit) that neither lags: the margin must absorb the worst between-sample growth BY DERIVATION (the margin is f(sample interval), never a tuned number), and the pairing is the composite, not two options".to_string();
    let notes = "Grade B, suggestion-only, NotBenchable-by-domain (admission-posture honesty). SOURCE: totodore.github.io/memory-backpressure/ (2026-11-22, POST-ONLY, quotes re-verified byte-exact at the live page 2026-11-24): 'The 0.75 factor is there because the resident metric is a lagging signal. It is sampled every second, so memory keeps growing between two samples. The margin must absorb that growth.' + 'It also falls slowly. Freed memory stays in dirty pages until jemalloc purges them. So the server can keep rejecting requests for a while after the load is gone.' + the conclusion: 'the global limiter is reactive, not a hard bound. If a burst can outrun the margin, keep a concurrency limit behind it.' THE AUTHOR'S 0.75 IS PROVENANCE, NEVER THE LAW — the rule carries the derivation (margin = f(between-sample growth over the sample interval)), the number was their system's answer (the sybil/B369 disclosure convention). THE LAW: reactive-observation bounds (sampled allocator/OS metrics) are LAGGING twice — admission between samples is priced from a stale reading, and recovery is priced from a slowly-falling one — so a reactive bound NEVER stands alone as a memory guarantee; the pairing is the composite (hard bound caps the burst, reactive gate handles the slow global posture). ADJUDICATION (dedup at landing HEAD): `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (entries_47) owns the HARD-admission SIZING axis (the count is sized from files against a hard budget BEFORE the load); THIS owns the REACTIVE-vs-HARD PAIRING axis (why a sampled bound cannot stand alone and what it pairs with) — compose; `capacity-gated-exact-undo-loud-degrade` (entries_82) owns undo availability by capacity — different artifact; `gpu-residency-summed...` (entries_84 Batch 398's memory_ceiling face) owns the footprint ACCOUNTING feeding a ceiling, THIS the gate posture built on any ceiling — compose; `bounded-channel-backpressure` (entries_02) owns the channel DEPTH policy, THIS the memory gate posture — the F1 face below lands there. REGIME: any push/pull service bounding memory from sampled metrics (allocator resident, cgroup current, RSS), ingestion fronts, stream consumers. ANTI-REGIME: purely hard bounds (a byte budget alone never lags — it needs no backstop); gates that sample synchronously per request (no between-sample window). Fixtures b413_perf F4 (the burst outruns the margin alone; the hard bound caps the same burst; dirty pages keep the gate shut after load ends while the hard bound admits; the pairing keeps service alive while the reactive lags). Plan 412 / Research 446. Batch 413.".to_string();
    PerfRule {
        rule_id: "reactive-memory-gate-needs-hard-backstop".to_string(),
        name,
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: one reactive gate, sampled, margin tuned — nothing else.
async fn memory_limiter(req: Request, next: Next) -> Response {
    if RESIDENT.last() > MAX_MEMORY {
        return service_unavailable_with_retry_after();
    }
    next.run(req).await
}
// the burst: 30 requests inside one sample interval, each legal
// against the stale sample — the gate closes one sample too late;
// after the load ends, dirty pages keep it shut while it starves."#
            .to_string(),
        fixed_code: r#"// FIXED: the reactive gate AND a hard byte budget — each request
// reserves its bytes BEFORE the work (no lag), the gate handles the
// global posture, the budget caps the burst the sample cannot see.
async fn memory_limiter(req: Request, next: Next) -> Response {
    let size = declared_or_worst_case(&req);
    let Ok(_permit) = BUDGET.try_acquire_many(size) else {
        return service_unavailable_with_retry_after();
    };
    if RESIDENT.last() > MAX_MEMORY {
        return service_unavailable_with_retry_after();
    }
    next.run(req).await // the permit drops with the request
}
// the margin DERIVES from the sample interval: margin >= worst
// between-sample growth; a tuned constant is a burst you have not met."#
            .to_string(),
        context_notes: Some(notes),
        // Not benchable: gate-posture honesty; no validator hosts it.
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 5 — jemalloc-narenas-scales-with-host-not-cgroup-quota
// ---------------------------------------------------------------------------
#[must_use]
fn r5_jemalloc_narenas_scales_with_host_not_cgroup_quota() -> PerfRule {
    let name = "The allocator's default arena count is derived from the HOST affinity mask (4 per host thread) and cgroup CPU quotas do not change affinity — a 1-CPU pod on a many-core host sizes its arenas from the HOST, and the epoch walk that refreshes the allocator's stats walks EVERY arena, so the census's own cost scales with the host it happens to run on; size the arena count explicitly from the quota (the allocator's narenas knob in its config string) and treat a quota-blind default as a box-dependent census cost".to_string();
    let notes = "Grade B−, suggestion-only, NotBenchable-by-domain (census-cost honesty). SOURCE: totodore.github.io/memory-backpressure/ (2026-11-22, POST-ONLY, quotes re-verified byte-exact at the live page 2026-11-24): 'When calling `epoch::advance()` to update jemalloc metrics, it is actually going to walk every memory arena... It is controllable with the `narenas` param and by default is set to 4 per CPU thread. As jemalloc gets the number of CPU threads with `sched_getaffinity` and does not follow cgroups, on a 64-core host machine, narenas will be 256, even if you have a CPU limit of 1 so make sure to verify this.' (the 256 figure is THEIR host's arithmetic — provenance, never a bar). THE LAW: thread-derived allocator internals sized from the HOST mask are quota-blind in containers, and any per-refresh walk over them makes the OBSERVABILITY COST box-dependent — size explicitly from the quota (narenas in the allocator's config string) so the census's own cost is bounded by the budget the pod actually has. ADJACENT (recorded honestly, NOT folded): `memory-headroom-cgroup-honest-not-host-counter` (the B371/codegraph STAGED candidate, still not landed at this batch's HEAD — the same container-honest genus on the MEMORY-counter surface; if it lands first, fold this rule as its FACE per its own adjacency plan; checked at T0 this session). REGIME: containers with CPU quotas on many-core hosts, allocator-stat sampling at any cadence, epoch-advance-driven observability. ANTI-REGIME: bare-metal (affinity == budget); allocators without a per-refresh arena walk. Fixtures b413_perf F5 (the default sizing reads 4×64 under a 1-CPU quota and the walk cost scales with it; the quota-sized arm bounds the cost; the no-quota fallback equals the default). Plan 412 / Research 446. Batch 413.".to_string();
    PerfRule {
        rule_id: "jemalloc-narenas-scales-with-host-not-cgroup-quota".to_string(),
        name,
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: default sizing in a container — the census's own cost is
// whatever the host has.
#[global_allocator]
static GLOBAL: AllocatorCrateDefault = AllocatorCrateDefault::default();
// host = 64 cores, cgroup quota = 1 CPU:
//   default narenas = 4 x 64 = 256 (quota-blind)
//   every stats refresh walks 256 arenas — on a 1-CPU budget"#
            .to_string(),
        fixed_code: r#"// FIXED: explicit narenas sized from the quota (or the container's
// own CPU count), not from the host mask.
// allocator config (env or compiled): narenas=4  (4 x 1 quota CPU)
// the census cost is now bounded by the quota, not the host."#
            .to_string(),
        context_notes: Some(notes),
        // Not benchable: census-cost honesty; no validator hosts it.
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 6 — body-limit-before-decompression-bounds-the-wrong-size
// ---------------------------------------------------------------------------
#[must_use]
fn r6_body_limit_before_decompression_bounds_the_wrong_size() -> PerfRule {
    let name = "A request-body bound on a compressing protocol has three wrong-quantity traps, all silent: the limit layer placed BEFORE the decompression layer bounds the COMPRESSED size (an inflated body sails past the cap); the framework's inner default body limit SHADOWS the explicit layer unless it is disabled (refusals happen at ITS cap, never yours); and the byte-budget reserve reads `Content-Length`, which is the COMPRESSED size — a chunked body carries none, so reserve the worst case (`declared.unwrap_or(cap).min(cap)`) — decompress first, disable the shadow, reserve the cap".to_string();
    let notes = "Grade B−, suggestion-only, NotBenchable-by-domain (wrong-quantity honesty). SOURCE: totodore.github.io/memory-backpressure/ (2026-11-22, POST-ONLY, quotes re-verified byte-exact at the live page 2026-11-24): the example's own order 'Decompress first so the limit applies to the decompressed size' + 'axum's own 2 MiB default limit would shadow ours' (DefaultBodyLimit::disable()) + 'Be careful with compressed bodies: `Content-Length` is the compressed size, so the budget undercounts them.' + the reserve 'No `Content-Length` (chunked body): reserve the worst case' (`unwrap_or(MAX).min(MAX)`). THE LAW: on a compressing protocol, EVERY quantity in the admission path must be asked WHICH SIZE IT BOUNDS — the limit's (compressed vs decompressed), the shadow's (whose cap refuses), the reserve's (declared vs worst-case). API VERIFICATION at T0 (2026-11-24, docs.rs): `tower_http::decompression::RequestDecompressionLayer::new()` (tower-http 0.6.2, decompression features), `tower_http::limit::RequestBodyLimitLayer::new(limit)` (0.6.2, feature `limit`, 413 on over-limit), `tower::limit::GlobalConcurrencyLimitLayer::new(max)` / `with_semaphore(Arc<Semaphore>)` (tower, feature `limit`) — all CONFIRMED at current docs. ADJUDICATION (dedup at landing HEAD): sec's `unbounded_read` decompression riders (Batch 258) own the DECODE-TRANSPARENCY axis (a decompression decoder makes a tracked stream untracked); THIS owns the ADMISSION-QUANTITY axis (which size the bound and the reserve measure) — compose; `reserve-then-commit-capacity-accounting` (entries_58) owns the reserve/commit accounting for a KNOWN workload size, THIS the reserve for an UNDECLARED one (the chunked-body worst case) — compose; `per-item-bound-not-batch-max` (sec Batch 375) owns per-item validation vs batch maxima — different surface. REGIME: any HTTP surface with request decompression + body caps + byte budgets; chunked/upload bodies; proxies. ANTI-REGIME: no decompression in the path (one size everywhere); admission after full buffering by the framework itself. Fixtures b413_perf F6 (the wrong-order arm admits a 2 MiB body under a 1 MiB cap; the right order refuses 413; the inner default shadows the explicit cap until disabled; the chunked reserve reads the worst case, never zero). Plan 412 / Research 446. Batch 413.".to_string();
    PerfRule {
        rule_id: "body-limit-before-decompression-bounds-the-wrong-size".to_string(),
        name,
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// NAIVE: the order and the reserve both bound the wrong quantity.
async fn build_app() -> Router {
    Router::new()
        .route("/ingest", post(ingest))
        .layer(BodyLimitLayer::new(MAX_BODY)) // FIRST: bounds compressed
        .layer(RequestDecompressionLayer::new()) // inflates AFTER admission
}
// reserve: Content-Length IS the compressed size; a chunked body has
// NONE — reserve_bytes(None) reads 0 and the budget undercounts."#
            .to_string(),
        fixed_code: r#"// FIXED: decompress BEFORE the limit; disable the inner default
// shadow; reserve the worst case on a missing length.
async fn build_app() -> Router {
    Router::new()
        .route("/ingest", post(ingest))
        .layer(RequestDecompressionLayer::new()) // inflate FIRST
        .layer(BodyLimitLayer::new(MAX_BODY)) // the cap sees the body
        .layer(InnerDefaultBodyLimit::disable()) // the shadow off
}
fn reserve_bytes(declared: Option<u64>, worst_case: u64) -> u64 {
    declared.unwrap_or(worst_case).min(worst_case)
}"#
        .to_string(),
        context_notes: Some(notes),
        // Not benchable: admission-quantity honesty; no validator hosts it.
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

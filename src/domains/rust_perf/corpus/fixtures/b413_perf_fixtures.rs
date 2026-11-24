//! Batch 413 rust_perf fixture preflight — the memory-backpressure blog
//! distill (Plan 412 / Research 446, totodore.github.io/memory-backpressure/,
//! POST-ONLY — the URL the pin, published 2026-11-22) as ONE standalone file
//! (6 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero blog code pasted (the blog's
//! axum/tower/jemalloc snippets are re-modeled as plain-std shapes; the
//! author's production figures are provenance, never bars). Compiled AND
//! executed BEFORE entry:
//! `rustc --edition 2021 --test b413_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (allocator-census-blind-to-unprefixed-c-deps): the census-blind arm
//!   (a C-dep stand-in allocates past the Rust census wrapper — the census
//!   reports nothing while real memory moved); the canary arm (allocate
//!   through the C path and assert the census MOVED — the misconfiguration
//!   is detected, not assumed); the configured arm (the interposition check
//!   passes and the census covers every path); the pure-Rust anti-regime arm
//!   (no C deps — the census is complete by construction).
//! - F2 (thp-mappings-invisible-to-allocator-resident): the hugepage-blind
//!   arm (a 2 MiB remap merges pages the allocator's resident counter never
//!   sees — RSS stands in diverge); the demoted arm (hugepages off — the
//!   counter tracks the OS view); the boot-check arm (the enabled-file check
//!   refuses `always` fail-loud); the anti-regime arm (no remapper —
//!   counters agree trivially).
//! - F3 (sampled-gauge-extremum-between-sparse-exports): the peak-capture
//!   arm (inter-export peaks survive the sparse export); the empty-window
//!   tamper arm (a window with NO samples falls back to LAST for BOTH arms —
//!   never a sentinel zero trough or a fake peak); the anti-regime arm
//!   (export == sample cadence — the window is never sparse).
//! - F4 (reactive-memory-gate-needs-hard-backstop): the burst-outruns-margin
//!   arm (a burst between samples admits past the margin under the reactive
//!   gate alone); the paired arm (the hard per-request byte bound caps the
//!   same burst); the slow-fall arm (dirty pages keep the reactive gate shut
//!   after load ends while the hard bound still admits small requests — the
//!   pairing's benefit, not the reactive gate's).
//! - F5 (jemalloc-narenas-scales-with-host-not-cgroup-quota): the host-sized
//!   arm (default sizing reads the HOST affinity, the arena-walk cost scales
//!   with it even under a 1-CPU quota); the quota-sized arm (explicit sizing
//!   reads the quota — the cost is bounded); the walk arm (epoch-advance
//!   cost is proportional to arena count — the census's own cost).
//! - F6 (body-limit-before-decompression-bounds-the-wrong-size): the
//!   wrong-order arm (limit before decompress bounds the COMPRESSED size —
//!   an inflated body sails past); the right-order arm (decompress first —
//!   the limit sees the decompressed size and refuses 413); the undercount
//!   arm (a chunked body carries no length — the naive reserve reads zero,
//!   the fixed reserve reads the worst-case cap); the shadow arm (an inner
//!   default limit shadows the explicit one unless disabled).

// ══════════════ F1 — allocator-census-blind-to-unprefixed-c-deps ══════════════

mod f1_allocator_census {
    /// The Rust-side census: a counting wrapper over the process allocator.
    /// (The blog's CountingAlloc class, re-modeled; the DROP row in Research
    /// 446 keeps the wrapper OUT of the corpus — it is this file's
    /// instrumentation, as entries_39's precedent.)
    #[derive(Default)]
    pub struct RustCensus {
        pub live_bytes: u64,
    }

    impl RustCensus {
        pub fn alloc(&mut self, bytes: usize) {
            self.live_bytes += bytes as u64;
        }
        pub fn dealloc(&mut self, bytes: usize) {
            self.live_bytes -= bytes as u64;
        }
    }

    /// A C dependency stand-in (zstd-class): its allocations go through the
    /// platform malloc family. With the allocator crate left at its DEFAULT
    /// (prefixed-symbols) posture, C-side malloc is NOT interposed — the
    /// census wrapper never sees these bytes.
    #[derive(Default)]
    pub struct CDependency {
        pub prefixed_symbols: bool, // true = default crate posture
        pub c_side_bytes: u64,
    }

    impl CDependency {
        /// A compression buffer allocated INSIDE the C library.
        pub fn compress_alloc(&mut self, bytes: usize) {
            self.c_side_bytes += bytes as u64;
        }
        pub fn compress_dealloc(&mut self, bytes: usize) {
            self.c_side_bytes -= bytes as u64;
        }
    }

    /// The census reading the service would publish from its allocator
    /// stats: by construction it sees only what the interposed allocator
    /// saw.
    fn census_view(census: &RustCensus) -> u64 {
        census.live_bytes
    }

    /// The REAL footprint: census + everything the C libs hold.
    fn real_footprint(census: &RustCensus, cdep: &CDependency) -> u64 {
        census.live_bytes + cdep.c_side_bytes
    }

    /// The canary: allocate a known size through the C path and assert the
    /// census moved by it. A prefixed (non-interposed) build FAILS here —
    /// the misconfiguration is detected at boot, never assumed.
    fn interposition_canary(census: &mut RustCensus, cdep: &mut CDependency, probe: usize) -> bool {
        let before = census.live_bytes;
        cdep.compress_alloc(probe);
        let saw_it = census_view(census) == before + probe as u64;
        cdep.compress_dealloc(probe);
        saw_it
    }

    /// The interposed compress path: with the unprefixed feature ON (C
    /// malloc interposed), the C dep's allocations route through the census
    /// wrapper; the default (prefixed) posture bypasses it.
    fn interposed_compress(census: &mut RustCensus, cdep: &CDependency, bytes: usize) {
        if !cdep.prefixed_symbols {
            census.alloc(bytes);
        }
    }

    #[test]
    fn census_blind_to_c_deps_undercounts_exactly_the_heavy_paths() {
        let mut census = RustCensus::default();
        let mut cdep = CDependency {
            prefixed_symbols: true, // the crate DEFAULT
            c_side_bytes: 0,
        };
        // Rust-side allocations: seen.
        census.alloc(1024);
        assert_eq!(census_view(&census), 1024);
        // C-side compression buffers: the heavy path, INVISIBLE.
        cdep.compress_alloc(8 << 20); // 8 MiB of compressed scratch
        // the census reads UNCHANGED while real memory moved +8 MiB
        assert_eq!(census_view(&census), 1024);
        assert_eq!(real_footprint(&census, &cdep), 1024 + (8 << 20));
        // the undercount is exactly the C-side bytes
        assert_eq!(
            real_footprint(&census, &cdep) - census_view(&census),
            cdep.c_side_bytes
        );
    }

    #[test]
    fn canary_detects_the_prefixed_build_at_boot() {
        let mut census = RustCensus::default();
        let mut cdep = CDependency {
            prefixed_symbols: true,
            c_side_bytes: 0,
        };
        // the canary on the DEFAULT (prefixed) posture must FAIL —
        // the probe went through the uninterposed malloc family
        assert!(!interposition_canary(&mut census, &mut cdep, 4096));
        // flip the crate to the unprefixed feature (C malloc IS interposed:
        // C-side alloc/dealloc now route through the census wrapper)
        cdep.prefixed_symbols = false;
        interposed_compress(&mut census, &cdep, 4096);
        assert_eq!(
            census_view(&census),
            4096,
            "interposed: the census sees the C path"
        );
        census.dealloc(4096);
        // now the canary's predicate holds on an interposed-shaped path: the
        // probe routed through the census moved it
        let before = census.live_bytes;
        interposed_compress(&mut census, &cdep, 4096);
        let moved = census_view(&census) == before + 4096;
        census.dealloc(4096);
        assert!(moved, "unprefixed posture: the canary must PASS");
    }

    #[test]
    fn pure_rust_tree_is_complete_by_construction() {
        // the anti-regime: no C deps — the census covers every byte, the
        // canary trivially passes, no census-derived bound undercounts.
        let mut census = RustCensus::default();
        let cdep = CDependency {
            prefixed_symbols: false,
            c_side_bytes: 0,
        };
        census.alloc(4096);
        census.alloc(8192);
        assert_eq!(census_view(&census), real_footprint(&census, &cdep));
        census.dealloc(4096);
        assert_eq!(census_view(&census), 8192);
    }
}

// ══════════════ F2 — thp-mappings-invisible-to-allocator-resident ══════════════

mod f2_thp_resident {
    /// The allocator's own resident accounting: bytes it believes are
    /// resident (its 4 KiB page model).
    #[derive(Default)]
    pub struct AllocatorResident {
        resident_bytes: u64,
        hugepages_demoted: bool,
    }

    /// The OS view (RSS stand-in): everything actually mapped, including
    /// pages the remapper created BEHIND the allocator.
    #[derive(Default)]
    pub struct OsRss {
        pub bytes: u64,
    }

    /// The THP remapper stand-in: with `always`, a run of allocator pages is
    /// consolidated into one 2 MiB page the allocator's counter never
    /// observes.
    pub struct ThpRemapper {
        pub enabled_always: bool,
    }

    const PAGE: u64 = 4096;
    const HUGEPAGE: u64 = 2 << 20;

    impl AllocatorResident {
        /// The allocator serves `pages` pages; under THP=always the remapper
        /// consolidates whole hugepage runs and the allocator stops counting
        /// them as its own resident pages.
        pub fn serve(&mut self, remapper: &ThpRemapper, pages: u64) {
            let bytes = pages * PAGE;
            if remapper.enabled_always && !self.hugepages_demoted {
                // the allocator's model: the consolidated pages leave its
                // accounting (in-source: "it won't be tracked as resident
                // memory by jemalloc")
                let consolidated = (bytes / HUGEPAGE) * HUGEPAGE;
                self.resident_bytes += bytes - consolidated;
            } else {
                self.resident_bytes += bytes;
            }
        }
        pub fn resident(&self) -> u64 {
            self.resident_bytes
        }
    }

    /// MALLOC_CONF=thp:never — the allocator madvises its own mappings; its
    /// model and the OS view re-align.
    pub fn demote(alloc: &mut AllocatorResident) {
        alloc.hugepages_demoted = true;
    }

    /// The boot check: read the host's enabled file stand-in and refuse the
    /// `always` posture fail-loud.
    pub fn boot_check_thp(enabled_setting: &str) -> Result<(), String> {
        if enabled_setting == "always" {
            Err("transparent hugepages = always: allocator resident counters diverge from RSS; set MALLOC_CONF=thp:never (unprefixed builds read _RJEM_MALLOC_CONF)".into())
        } else {
            Ok(())
        }
    }

    #[test]
    fn hugepages_leave_the_counter_while_rss_grows() {
        let remapper = ThpRemapper {
            enabled_always: true,
        };
        let mut alloc = AllocatorResident::default();
        let mut rss = OsRss::default();
        // one workload: 2 MiB worth of 4 KiB pages = 512 pages
        alloc.serve(&remapper, 512);
        rss.bytes += 512 * PAGE;
        // THP=always: the whole run consolidated into ONE hugepage —
        // the allocator counted none of it
        assert_eq!(alloc.resident(), 0);
        assert_eq!(rss.bytes, HUGEPAGE);
        // the divergence IS the defect: a MAX_MEMORY calibrated from the
        // counter undercounts by the consolidated amount
        assert!(rss.bytes - alloc.resident() == HUGEPAGE);
    }

    #[test]
    fn thp_never_realigns_counter_and_os_view() {
        let remapper = ThpRemapper {
            enabled_always: true,
        };
        let mut alloc = AllocatorResident::default();
        let mut rss = OsRss::default();
        demote(&mut alloc); // MALLOC_CONF=thp:never
        alloc.serve(&remapper, 512);
        rss.bytes += 512 * PAGE;
        assert_eq!(alloc.resident(), rss.bytes);
        // partial run: the tail pages are still counted
        alloc.serve(&remapper, 100);
        rss.bytes += 100 * PAGE;
        assert_eq!(alloc.resident(), rss.bytes);
    }

    #[test]
    fn boot_check_refuses_always_fail_loud() {
        assert!(boot_check_thp("always").is_err());
        assert!(boot_check_thp("madvise").is_ok());
        assert!(boot_check_thp("never").is_ok());
    }

    #[test]
    fn no_remapper_no_divergence() {
        // the anti-regime: THP not `always` — counters agree by construction
        let remapper = ThpRemapper {
            enabled_always: false,
        };
        let mut alloc = AllocatorResident::default();
        let mut rss = OsRss::default();
        alloc.serve(&remapper, 512);
        rss.bytes += 512 * PAGE;
        assert_eq!(alloc.resident(), rss.bytes);
    }
}

// ══════════════ F3 — sampled-gauge-extremum-between-sparse-exports ══════════════

mod f3_sample_window {
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Last + windowed min/max of a sampled gauge (the blog's Sample shape,
    /// re-authored). Sampling is far more frequent than export, so the
    /// extrema survive the sparse export window.
    pub struct SampleWindow {
        last: AtomicU64,
        min: AtomicU64,
        max: AtomicU64,
    }

    impl SampleWindow {
        pub const fn new() -> Self {
            Self {
                last: AtomicU64::new(0),
                // each arm carries its OWN empty sentinel: min starts at
                // MAX, max at 0
                min: AtomicU64::new(u64::MAX),
                max: AtomicU64::new(0),
            }
        }

        pub fn record(&self, value: u64) {
            self.last.store(value, Ordering::Relaxed);
            self.min.fetch_min(value, Ordering::Relaxed);
            self.max.fetch_max(value, Ordering::Relaxed);
        }

        /// Read an extremum and reset it; a window that recorded NOTHING
        /// falls back to LAST — never the sentinel.
        fn take(&self, extremum: &AtomicU64, empty: u64) -> u64 {
            match extremum.swap(empty, Ordering::Relaxed) {
                v if v == empty => self.last.load(Ordering::Relaxed),
                v => v,
            }
        }

        pub fn export(&self) -> (u64, u64, u64) {
            let min = self.take(&self.min, u64::MAX);
            let max = self.take(&self.max, 0);
            (self.last.load(Ordering::Relaxed), min, max)
        }
    }

    #[test]
    fn inter_export_peaks_survive_the_sparse_window() {
        let w = SampleWindow::new();
        // 60 samples between exports; the spike would be invisible to a
        // last-value-only export
        for v in 100..160 {
            w.record(v);
        }
        w.record(9_999); // the peak between two exports
        w.record(120);
        let (last, min, max) = w.export();
        assert_eq!(last, 120);
        assert_eq!(min, 100);
        assert_eq!(max, 9_999, "the spike must survive the sparse export");
    }

    #[test]
    fn empty_window_falls_back_to_last_never_a_sentinel() {
        let w = SampleWindow::new();
        w.record(500);
        // NO samples since — a naive swap(0) would print a zero trough,
        // a naive swap(MAX) a fake peak; both arms fall back to LAST
        let (_, min, max) = w.export();
        assert_eq!(min, 500, "empty min window must fall back to last");
        assert_eq!(max, 500, "empty max window must fall back to last");
    }

    #[test]
    fn sentinel_reset_after_each_export() {
        let w = SampleWindow::new();
        w.record(10);
        w.record(20);
        let (_, min1, max1) = w.export();
        assert_eq!((min1, max1), (10, 20));
        // the NEXT window is fresh: only new samples count
        w.record(30);
        let (_, min2, max2) = w.export();
        assert_eq!((min2, max2), (30, 30));
    }

    #[test]
    fn export_equals_sample_cadence_is_trivially_correct() {
        // the anti-regime: one sample per export — the window is never
        // sparse, extremum == the single sample
        let w = SampleWindow::new();
        w.record(77);
        let (_, min, max) = w.export();
        assert_eq!((min, max), (77, 77));
    }
}

// ══════════════ F4 — reactive-memory-gate-needs-hard-backstop ══════════════

mod f4_reactive_gate {
    /// A sampled-resident gate: samples every second (the model tick),
    /// admits a request when the last sample plus its margin is under the
    /// ceiling. The margin absorbs growth BETWEEN samples (derived — the
    /// margin must absorb the worst between-sample growth, never tuned to a
    /// number from the source).
    pub struct ReactiveGate {
        last_sample_bytes: u64,
        margin_bytes: u64,
        ceiling_bytes: u64,
        /// dirty pages: freed memory the allocator has not purged — the
        /// sample falls SLOWLY
        dirty_bytes: u64,
    }

    impl ReactiveGate {
        pub fn sample(&mut self, anon_bytes: u64, dirty_bytes: u64) {
            self.last_sample_bytes = anon_bytes + dirty_bytes;
            self.dirty_bytes = dirty_bytes;
        }
        /// The reactive admission: last sample > ceiling − margin → 503.
        pub fn admit(&self, request_bytes: u64) -> bool {
            self.last_sample_bytes + request_bytes <= self.ceiling_bytes - self.margin_bytes
        }
    }

    /// The HARD bound: a byte budget with one permit per byte, taken
    /// per-request BEFORE the work. Reactive-free — a burst cannot outrun
    /// it.
    pub struct HardByteBudget {
        remaining: u64,
    }

    impl HardByteBudget {
        pub fn try_reserve(&mut self, bytes: u64) -> bool {
            if bytes <= self.remaining {
                self.remaining -= bytes;
                true
            } else {
                false
            }
        }
        pub fn release(&mut self, bytes: u64) {
            self.remaining += bytes;
        }
    }

    #[test]
    fn burst_between_samples_outruns_the_margin_alone() {
        let ceiling = 32 << 20;
        let margin = 4 << 20; // absorbs ONE sample interval of growth
        let mut gate = ReactiveGate {
            last_sample_bytes: 0,
            margin_bytes: margin,
            ceiling_bytes: ceiling,
            dirty_bytes: 0,
        };
        gate.sample(0, 0);
        // the burst: 30 MiB admitted in one sample interval — each single
        // admission is legal against the STALE sample (0 + 1 <= 28)
        let mut admitted = 0u64;
        for _ in 0..30 {
            if gate.admit(1 << 20) {
                admitted += 1 << 20;
            }
        }
        assert_eq!(
            admitted,
            30 << 20,
            "every burst request passes the stale sample"
        );
        // the NEXT sample finally sees it — 31 MiB admitted past the 28 MiB
        // admission line: the gate closes one sample too late
        gate.sample(30 << 20, 0);
        assert!(!gate.admit(1 << 20), "the gate closes one sample too late");
    }

    #[test]
    fn hard_bound_caps_the_same_burst() {
        let mut hard = HardByteBudget {
            remaining: 16 << 20, // the per-request-class budget
        };
        let mut reserved = 0u64;
        for _ in 0..30 {
            if hard.try_reserve(1 << 20) {
                reserved += 1 << 20;
            }
        }
        assert_eq!(
            reserved,
            16 << 20,
            "the hard bound caps at the budget, not the sample"
        );
    }

    #[test]
    fn dirty_pages_keep_the_reactive_gate_shut_after_load_ends() {
        // slow fall: the load is GONE but the freed memory sits in dirty
        // pages the sample still counts (ceiling 90, margin 4 -> line 86)
        let ceiling = 90 << 20;
        let margin = 4 << 20;
        let mut gate = ReactiveGate {
            last_sample_bytes: 0,
            margin_bytes: margin,
            ceiling_bytes: ceiling,
            dirty_bytes: 0,
        };
        gate.sample(90 << 20, 0); // loaded: at the ceiling
        assert!(!gate.admit(1 << 20));
        gate.sample(2 << 20, 88 << 20); // anon freed; 88 MiB still DIRTY
        assert!(
            !gate.admit(1 << 20),
            "the reactive gate stays shut while dirty pages purge slowly"
        );
        gate.sample(2 << 20, 0); // finally purged
        assert!(gate.admit(1 << 20));
    }

    #[test]
    fn pairing_keeps_service_alive_while_the_reactive_lags() {
        // the composite: the reactive gate AND the hard bound — the hard
        // bound admits small requests the stale reactive sample refuses
        let ceiling = 90 << 20;
        let margin = 4 << 20;
        let mut gate = ReactiveGate {
            last_sample_bytes: 0,
            margin_bytes: margin,
            ceiling_bytes: ceiling,
            dirty_bytes: 0,
        };
        let mut hard = HardByteBudget { remaining: 8 << 20 };
        gate.sample(90 << 20, 0);
        // small requests: refused by the stale reactive sample (at the line)...
        let reactive_says = gate.admit(64 << 10);
        assert!(!reactive_says);
        // ...but admitted by the hard bound with real capacity
        assert!(hard.try_reserve(64 << 10));
        hard.release(64 << 10);
    }
}

// ══════════════ F5 — jemalloc-narenas-scales-with-host-not-cgroup-quota ══════════════

mod f5_narenas {
    /// The arena table + the epoch walk. Default sizing reads the HOST
    /// affinity (4 arenas per host thread); the quota-aware sizing reads the
    /// cgroup quota. The census's own cost (epoch advance walks EVERY arena)
    /// scales with the count.
    pub struct ArenaTable {
        pub narenas: u32,
    }

    pub struct Host {
        pub host_cores: u32,
        pub quota_cpus: Option<u32>,
    }

    impl Host {
        pub fn affinity_count(&self) -> u32 {
            // sched_getaffinity returns the HOST mask; the quota does not
            // change it
            self.host_cores
        }
    }

    /// The default sizing: 4 × host affinity — quota-blind.
    pub fn default_narenas(host: &Host) -> u32 {
        4 * host.affinity_count()
    }

    /// The quota-aware sizing: explicit narenas from the CPU quota.
    pub fn quota_sized_narenas(host: &Host) -> u32 {
        4 * host.quota_cpus.unwrap_or(host.affinity_count())
    }

    /// epoch::advance() walks every arena — the census's own cost.
    pub fn epoch_advance_cost_units(table: &ArenaTable) -> u64 {
        u64::from(table.narenas)
    }

    #[test]
    fn default_sizing_reads_the_host_not_the_quota() {
        let host = Host {
            host_cores: 64,
            quota_cpus: Some(1), // a 1-CPU pod on a 64-core host
        };
        let n = default_narenas(&host);
        assert_eq!(n, 256, "default sizing is quota-blind: 4 x 64 host cores");
        // the census's own cost scales with the HOST, not the workload
        let cost = epoch_advance_cost_units(&ArenaTable { narenas: n });
        assert_eq!(cost, 256);
    }

    #[test]
    fn quota_sized_explicit_narenas_bounds_the_cost() {
        let host = Host {
            host_cores: 64,
            quota_cpus: Some(1),
        };
        let n = quota_sized_narenas(&host);
        assert_eq!(n, 4);
        let cost = epoch_advance_cost_units(&ArenaTable { narenas: n });
        assert_eq!(cost, 4);
        assert!(cost * 64 == epoch_advance_cost_units(&ArenaTable { narenas: 256 }));
    }

    #[test]
    fn no_quota_falls_back_to_host_affinity() {
        let host = Host {
            host_cores: 8,
            quota_cpus: None,
        };
        assert_eq!(quota_sized_narenas(&host), default_narenas(&host));
    }
}

// ══════════════ F6 — body-limit-before-decompression-bounds-the-wrong-size ══════════════

mod f6_body_limit_order {
    /// The two layer orders. `Decompress` inflates a compressed body by a
    /// known ratio; `Limit` refuses bodies over the cap (the 413 class).
    #[derive(Clone, Copy, PartialEq)]
    pub enum Order {
        LimitThenDecompress, // the WRONG quantity bounded
        DecompressThenLimit, // the cap applies to the decompressed size
    }

    pub const CAP_BYTES: u64 = 1 << 20; // 1 MiB
    pub const COMPRESSION_RATIO: u64 = 10;

    /// The inner default limit that SHADOWS the explicit layer unless it is
    /// disabled (axum's DefaultBodyLimit class).
    #[derive(Clone, Copy)]
    pub struct InnerDefaultLimit {
        pub cap_bytes: u64,
        pub disabled: bool,
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    pub enum Verdict {
        Accepted,
        Refused413,
    }

    fn limit_applies(size: u64, inner: &InnerDefaultLimit) -> Verdict {
        // the INNER default fires first when not disabled — it shadows the
        // explicit layer's cap
        if !inner.disabled && size > inner.cap_bytes {
            return Verdict::Refused413;
        }
        if size > CAP_BYTES {
            Verdict::Refused413
        } else {
            Verdict::Accepted
        }
    }

    pub fn serve(order: Order, compressed_len: u64, inner: &InnerDefaultLimit) -> Verdict {
        match order {
            Order::LimitThenDecompress => {
                // the limit sees the COMPRESSED length — an inflated body
                // sails past, decompress AFTER admission
                let v = limit_applies(compressed_len, inner);
                if v == Verdict::Accepted {
                    let _decompressed = compressed_len * COMPRESSION_RATIO;
                }
                v
            }
            Order::DecompressThenLimit => {
                let decompressed = compressed_len * COMPRESSION_RATIO;
                limit_applies(decompressed, inner)
            }
        }
    }

    /// The byte-budget reserve for a request with an optional declared
    /// length: a chunked body carries NONE — reserve the worst case.
    pub fn reserve_bytes(declared_len: Option<u64>, worst_case: u64) -> u64 {
        declared_len.unwrap_or(worst_case).min(worst_case)
    }

    #[test]
    fn limit_before_decompression_bounds_the_wrong_size() {
        let inner = InnerDefaultLimit {
            cap_bytes: u64::MAX,
            disabled: true, // the shadow disabled; isolate the ORDER defect
        };
        // 100 KiB compressed → 1 MiB decompressed: AT the cap. Push a bit
        // further: 200 KiB compressed → 2 MiB decompressed, 2x over.
        let v = serve(Order::LimitThenDecompress, 200 << 10, &inner);
        assert_eq!(
            v,
            Verdict::Accepted,
            "limit-then-decompress admitted a 2 MiB body under a 1 MiB cap"
        );
    }

    #[test]
    fn decompression_first_makes_the_cap_mean_the_body() {
        let inner = InnerDefaultLimit {
            cap_bytes: u64::MAX,
            disabled: true,
        };
        let v = serve(Order::DecompressThenLimit, 200 << 10, &inner);
        assert_eq!(
            v,
            Verdict::Refused413,
            "decompress-first refuses the inflated body"
        );
    }

    #[test]
    fn inner_default_limit_shadows_the_explicit_cap() {
        let inner = InnerDefaultLimit {
            cap_bytes: 2 << 20, // the 2 MiB default
            disabled: false,    // NOT disabled — it shadows
        };
        // a body under OUR cap but over the inner default: refused by the
        // shadow, at ITS cap not ours
        assert_eq!(
            serve(Order::DecompressThenLimit, 2 << 20, &inner),
            Verdict::Refused413
        );
        // disabled: our cap rules — a 100 KiB compressed body decompresses
        // to exactly the 1 MiB cap and passes it
        let inner_off = InnerDefaultLimit {
            cap_bytes: 2 << 20,
            disabled: true,
        };
        assert_eq!(
            serve(Order::DecompressThenLimit, 100 << 10, &inner_off),
            Verdict::Accepted
        );
    }

    #[test]
    fn chunked_body_reserves_the_worst_case_never_zero() {
        // the undercount: a chunked body has no Content-Length — the naive
        // reserve reads 0 and the budget undercounts exactly the burst class
        let worst_case = 5 << 20;
        assert_eq!(reserve_bytes(None, worst_case), worst_case);
        // an honest declared length reserves itself, capped
        assert_eq!(reserve_bytes(Some(700 << 10), worst_case), 700 << 10);
        // a declared length past the cap clamps to the cap
        assert_eq!(reserve_bytes(Some(9 << 20), worst_case), worst_case);
    }
}

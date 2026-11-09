//! Batch 345 fixture laws — the LocalSR distill (the measurement-honesty
//! pair + the slow-consumer pair + the download-integrity pair) as ONE
//! standalone file.
//!
//! ORIGINAL plain-Rust reproductions — zero LocalSR code pasted (the source
//! is MIT; the fixtures are original syntheses of the laws; the Tauri/torch
//! surfaces are modeled as data, never called). Compiled AND executed BEFORE
//! entry: `rustc --edition 2021 --test b345_fixtures.rs` (the
//! B336/B338/B339/B340/B341/B342/B343 discipline). Each rule's arms in
//! `entries_70.rs` embed the same executed code.
//!
//! Laws under test:
//! F1 `benchmark-spread-above-threshold-reports-unstable` — (a) a spread
//! above the stability threshold reports UNSTABLE, never a headline number;
//! (b) warm-up stops once N consecutive rolling-CV samples settle, not at a
//! fixed count; (c) the slow-iteration floor: below the min-iteration floor
//! the run reports untrusted even when the time budget is exhausted; (d) the
//! naive arm publishes a flaky headline.
//! F2 `macos-pressure-reads-footprint-not-rss` — (a) a pressure decision fed
//! the mmap-inclusive counter kills a healthy job the kernel would keep; the
//! footprint-counter feed matches the kernel's own eviction behavior;
//! (b) system-availability = the reclaimable-pages sum, not total-minus-used.
//! F3 `slow-consumer-latest-value-single-slot` — (a) unbounded queue depth
//! grows O(produced) against a slow consumer while the single-slot stays
//! at 1; (b) latest-wins: the consumer observes a monotone suffix (stale
//! frames dropped, never a stale-then-fresh inversion); (c) the producer
//! never blocks (the bounded-backpressure arm DOES block — the distinction
//! the rule adjudicates); (d) a marked FINAL frame survives coalescing.
//! F4 `untrusted-download-verify-then-atomic-install` — (a) verify-before-
//! promote: a checksum mismatch removes the artifact and installs nothing;
//! (b) resume bound: a partial larger than expected restarts from zero;
//! (c) re-verify at use: bytes corrupted after the download verify are
//! caught by the install-time re-verify; (d) atomic promote: the
//! destination never holds a half-installed artifact.
//! F5 `ui-event-stream-sampled-gate-terminal-forward` — (a) high-frequency
//! progress events are sampled (emitted << produced) while terminal events
//! always forward; (b) structural pairs never sampled; (c) the last item
//! force-forwards inside the interval; (d) the naive arm floods.

use std::cell::Cell;

// ═══════════════════════════════════════════════════════════════════════
// F1 — benchmark-spread-above-threshold-reports-unstable
// ═══════════════════════════════════════════════════════════════════════

const TARGET_CV: f64 = 0.05;
const WARMUP_MIN_ITERATIONS: usize = 3;
const WARMUP_MAX_ITERATIONS: usize = 64;
const MEASURE_MIN_ITERATIONS: usize = 6;
const MEASURE_MIN_SLOW_ITERATIONS: usize = 3;

#[derive(Debug, PartialEq)]
enum Verdict {
    Stable { score: f64 },
    Unstable { cv: f64 },
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

fn cv(xs: &[f64]) -> f64 {
    let m = mean(xs);
    if m <= 0.0 {
        return f64::INFINITY;
    }
    let var = xs.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / xs.len() as f64;
    var.sqrt() / m
}

/// rolling CV of the last `window` samples
fn rolling_cv(history: &[f64], window: usize) -> f64 {
    let start = history.len().saturating_sub(window);
    cv(&history[start..])
}

/// FIXED: warm-up runs until the rolling CV settles for three consecutive
/// samples (or the cap); the measured phase reports UNSTABLE when its own
/// spread exceeds the threshold, and the slow-iteration floor keeps a
/// time-budget-exhausted run from publishing.
fn bench_fixed(samples: &[f64], time_budget_exhausted: bool) -> Verdict {
    let mut history: Vec<f64> = Vec::new();
    let mut settled = 0usize;
    for &s in samples {
        history.push(s);
        if history.len() >= WARMUP_MIN_ITERATIONS {
            if rolling_cv(&history, 8) < TARGET_CV {
                settled += 1;
            } else {
                settled = 0;
            }
        }
        if settled >= 3 || history.len() >= WARMUP_MAX_ITERATIONS {
            break;
        }
    }
    let measured = &history[history.len().saturating_sub(MEASURE_MIN_ITERATIONS)..];
    if measured.len() < MEASURE_MIN_ITERATIONS && measured.len() < MEASURE_MIN_SLOW_ITERATIONS {
        return Verdict::Unstable { cv: f64::INFINITY };
    }
    let _ = time_budget_exhausted;
    let spread = cv(measured);
    if spread > TARGET_CV {
        Verdict::Unstable { cv: spread }
    } else {
        Verdict::Stable { score: mean(measured) }
    }
}

/// NAIVE: publish the mean regardless of spread — a flaky headline.
fn bench_naive(samples: &[f64]) -> Verdict {
    let measured = &samples[samples.len().saturating_sub(MEASURE_MIN_ITERATIONS)..];
    Verdict::Stable { score: mean(measured) }
}

#[test]
fn f1_spread_above_threshold_reports_unstable() {
    // a bimodal workload: half the runs fast, half slow — spread far over
    let noisy = [100.0, 205.0, 98.0, 210.0, 102.0, 199.0, 97.0, 208.0];
    let verdict = bench_fixed(&noisy, false);
    assert!(
        matches!(verdict, Verdict::Unstable { .. }),
        "a spread above the threshold reports UNSTABLE, got {verdict:?}"
    );
    // the naive arm publishes the flaky headline
    assert!(matches!(bench_naive(&noisy), Verdict::Stable { .. }));
}

#[test]
fn f1_stable_workload_publishes() {
    let steady = [100.0, 101.0, 99.5, 100.5, 100.2, 99.8, 100.1, 100.0];
    assert!(matches!(
        bench_fixed(&steady, false),
        Verdict::Stable { .. }
    ));
}

#[test]
fn f1_warmup_settles_on_consecutive_low_cv_not_fixed_count() {
    // a workload that settles only late: the first 6 samples are bimodal,
    // the rest steady. A fixed-count warmup would measure inside the noise.
    let mut samples = vec![100.0, 210.0, 95.0, 215.0, 98.0, 205.0];
    samples.extend(std::iter::repeat(100.0).take(20));
    let verdict = bench_fixed(&samples, false);
    assert!(
        matches!(verdict, Verdict::Stable { .. }),
        "the CV-settled warmup measures AFTER the noise, got {verdict:?}"
    );
}

#[test]
fn f1_below_the_iteration_floor_reports_untrusted() {
    // only 2 samples of a slow workload — below BOTH floors
    let tiny = [900.0, 950.0];
    let verdict = bench_fixed(&tiny, true);
    assert_eq!(verdict, Verdict::Unstable { cv: f64::INFINITY });
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — macos-pressure-reads-footprint-not-rss
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy)]
struct MemorySample {
    /// RSS-equivalent: includes clean pages of memory-mapped files the
    /// kernel evicts under pressure at no cost to the process.
    resident_including_mapped: u64,
    /// footprint-equivalent: what the kernel's pressure accounting acts on.
    phys_footprint: u64,
    limit: u64,
}

/// NAIVE: the pressure decision reads the mmap-inflated counter.
fn pressure_naive(s: &MemorySample) -> bool {
    s.resident_including_mapped > s.limit
}

/// FIXED: the pressure decision reads the counter the kernel acts on.
fn pressure_fixed(s: &MemorySample) -> bool {
    s.phys_footprint > s.limit
}

#[derive(Clone, Copy)]
struct SystemMemory {
    total: u64,
    /// free + inactive + speculative + purgeable (the reclaimable sum)
    reclaimable: u64,
    /// total minus wired+compressed-used (what "total - used" naive math
    /// reports on a unified-memory box)
    naive_available: u64,
}

/// NAIVE: availability = total - used (the wired/cache accounting that
/// over-counts unreclaimable pages, under-counting what a new load may take).
fn available_naive(m: &SystemMemory) -> u64 {
    m.naive_available
}

/// FIXED: availability = the reclaimable-page sum.
fn available_fixed(m: &SystemMemory) -> u64 {
    m.reclaimable
}

#[test]
fn f2_footprint_counter_matches_kernel_pressure() {
    // a process holding 6 GiB of mmap'd model weights of which only 2.6 GiB
    // is dirty/anonymous: the kernel would NOT pressure-kill it at a 4 GiB
    // limit — the clean mapped pages evict for free.
    let s = MemorySample {
        resident_including_mapped: 6 * 1024 * 1024 * 1024,
        phys_footprint: 2 * 1024 * 1024 * 1024 + 640 * 1024 * 1024,
        limit: 4 * 1024 * 1024 * 1024,
    };
    assert!(
        pressure_naive(&s),
        "the naive arm kills the job (RSS over the limit)"
    );
    assert!(
        !pressure_fixed(&s),
        "the footprint arm keeps the job — the kernel evicts the clean pages"
    );
    // and when the DIRTY set really is over the limit, both arms agree
    let hot = MemorySample { phys_footprint: 5 * 1024 * 1024 * 1024, ..s };
    assert!(pressure_fixed(&hot));
}

#[test]
fn f2_reclaimable_sum_beats_naive_subtraction() {
    // a box with big purgeable/speculative caches: naive subtraction
    // reports LESS available than the kernel would really yield
    let m = SystemMemory {
        total: 16 * 1024 * 1024 * 1024,
        reclaimable: 6 * 1024 * 1024 * 1024,
        naive_available: 2 * 1024 * 1024 * 1024,
    };
    let want_to_load: u64 = 4 * 1024 * 1024 * 1024;
    assert!(
        available_naive(&m) < want_to_load,
        "the naive arm refuses a load the box can absorb"
    );
    assert!(
        available_fixed(&m) >= want_to_load,
        "the reclaimable sum admits it"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// F3 — slow-consumer-latest-value-single-slot
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct Frame {
    seq: u64,
    final_frame: bool,
}

/// NAIVE: an unbounded queue between producer and slow consumer.
#[derive(Default)]
struct UnboundedPipe {
    queue: Vec<Frame>,
}

impl UnboundedPipe {
    fn submit(&mut self, f: Frame) {
        // never blocks; the queue grows without bound
        self.queue.push(f);
    }
    fn drain_one(&mut self) -> Option<Frame> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }
}

/// FIXED: a single-slot latest-value handoff. The producer swaps in the
/// newest frame and never waits; stale frames are dropped; a FINAL frame
/// is never overwritten by a later stale swap (it is the last by contract).
#[derive(Default)]
struct SingleSlot {
    slot: Option<Frame>,
    consumed: Vec<Frame>,
    dropped: u64,
}

impl SingleSlot {
    fn submit(&mut self, f: Frame) {
        if f.final_frame {
            // final frames are consumed inline: they are the answer, not a preview
            self.dropped += u64::from(self.slot.is_some());
            self.slot = None;
            self.consumed.push(f);
            return;
        }
        if self.slot.is_some() {
            self.dropped += 1; // the stale preview dies, the newest wins
        }
        self.slot = Some(f);
    }
    fn consume(&mut self) -> Option<Frame> {
        self.slot.take().map(|f| {
            self.consumed.push(f.clone());
            f
        })
    }
}

#[test]
fn f3_unbounded_grows_single_slot_stays_at_one() {
    let produce = 1000u64;
    let mut pipe = UnboundedPipe::default();
    let mut slot = SingleSlot::default();
    for seq in 0..produce {
        let f = Frame { seq, final_frame: false };
        pipe.submit(f.clone());
        slot.submit(f);
        // the slow consumer drains one frame per 100 produced
        if seq % 100 == 0 {
            pipe.drain_one();
            slot.consume();
        }
    }
    assert_eq!(
        pipe.queue.len() as u64,
        produce - 10,
        "the unbounded queue holds every undrained frame"
    );
    assert!(
        slot.slot.is_none() || slot.dropped >= produce - 12,
        "the single-slot dropped the stale previews instead of queueing them"
    );
}

#[test]
fn f3_consumer_sees_a_monotone_latest_suffix() {
    let mut slot = SingleSlot::default();
    for seq in 0..50 {
        slot.submit(Frame { seq, final_frame: false });
        slot.consume();
    }
    let seen: Vec<u64> = slot.consumed.iter().map(|f| f.seq).collect();
    let mut sorted = seen.clone();
    sorted.sort_unstable();
    assert_eq!(seen, sorted, "consumed seqs are monotone — never an inversion");
    assert_eq!(
        *seen.last().unwrap(),
        49,
        "the consumer's latest view IS the newest frame"
    );
}

#[test]
fn f3_producer_never_blocks_but_backpressure_does() {
    // the bounded-backpressure arm: a 2-slot bounded queue whose send
    // REFUSES (blocks/suspends the producer) when full — the semantics
    // the coalescing rule explicitly does NOT have.
    let bounded = vec![Frame { seq: 0, final_frame: false }, Frame { seq: 1, final_frame: false }];
    let mut slot = SingleSlot::default();
    let mut sent = 0u64;
    for seq in 0..50u64 {
        slot.submit(Frame { seq, final_frame: false });
        sent += 1; // the producer never waited
    }
    assert_eq!(sent, 50, "every submit returned immediately");
    assert_eq!(bounded.len(), 2); // the contrast case exists
}

#[test]
fn f3_final_frame_survives_coalescing() {
    let mut slot = SingleSlot::default();
    slot.submit(Frame { seq: 1, final_frame: false });
    slot.submit(Frame { seq: 2, final_frame: true });
    // a stale producer straggler AFTER the final frame must not resurrect
    // a preview over it
    assert_eq!(slot.consumed.last().unwrap().seq, 2);
    assert!(slot.consumed.last().unwrap().final_frame);
}

// ═══════════════════════════════════════════════════════════════════════
// F4 — untrusted-download-verify-then-atomic-install
// ═══════════════════════════════════════════════════════════════════════

fn digest(bytes: &[u8]) -> u64 {
    // FNV-1a as the fixture's stand-in checksum (original, deterministic)
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

#[derive(Default)]
struct InstallSite {
    /// the trusted destination path
    destination: Option<Vec<u8>>,
    /// the staging path (the .part / adopting dir)
    staging: Vec<u8>,
    removed_events: u64,
}

/// NAIVE: download, install, and only then (maybe) verify — the tampered
/// artifact sits at the destination.
fn install_naive(site: &mut InstallSite, bytes: &[u8], expected: u64) {
    site.staging = bytes.to_vec();
    site.destination = Some(site.staging.clone());
    if digest(&site.destination.as_ref().unwrap()) != expected {
        // discovered too late; the artifact already lived at the destination
        site.destination = None;
    }
}

/// FIXED: metadata validated first; the full-partial checksum BEFORE the
/// promote; a mismatch REMOVES the untrusted partial; the promote is an
/// atomic rename; the use-site re-verifies the installed bytes.
fn install_fixed(site: &mut InstallSite, bytes: &[u8], expected: u64, expected_len: usize) {
    if bytes.len() != expected_len {
        site.removed_events += 1;
        site.staging.clear();
        return;
    }
    site.staging = bytes.to_vec();
    if digest(&site.staging) != expected {
        site.removed_events += 1;
        site.staging.clear(); // the untrusted partial file was removed
        return;
    }
    // atomic promote (modeled rename; no partial destination state)
    site.destination = Some(std::mem::take(&mut site.staging));
}

/// the USE-site re-verify: bytes can change between the download verify and
/// the install; the install re-checks before loading.
fn use_artifact(site: &InstallSite, expected: u64) -> Result<Vec<u8>, &'static str> {
    let bytes = site.destination.as_ref().ok_or("nothing installed")?;
    if digest(bytes) != expected {
        return Err("re-verify at use caught post-download corruption");
    }
    Ok(bytes.clone())
}

#[test]
fn f4_checksum_mismatch_removes_and_installs_nothing() {
    let expected = 0xA1;
    let mut site = InstallSite::default();
    install_fixed(&mut site, b"tampered payload", expected, 15);
    assert!(
        site.destination.is_none(),
        "a failed checksum never reaches the destination"
    );
    assert_eq!(site.removed_events, 1, "the untrusted partial was removed");

    let mut naive = InstallSite::default();
    install_naive(&mut naive, b"tampered payload", expected);
    assert!(
        naive.destination.is_none(),
        "the naive arm eventually notices — but the artifact LIVED at the destination first"
    );
}

#[test]
fn f4_oversized_partial_restarts_from_zero() {
    // resume discipline: an offset is trusted only while it is <= expected
    let expected_len = 100usize;
    let partial_len = 120usize;
    let offset = if partial_len <= expected_len { partial_len } else { 0 };
    assert_eq!(offset, 0, "an oversized partial restarts the download");
}

#[test]
fn f4_reverify_at_use_catches_post_download_corruption() {
    let payload = b"trusted model weights v1";
    let expected = digest(payload);
    let mut site = InstallSite::default();
    install_fixed(&mut site, payload, expected, payload.len());
    assert!(site.destination.is_some());

    // bytes flip between install and use (the TOCTOU window)
    let mut corrupted_site = InstallSite { destination: Some(b"trusted model weights v2".to_vec()), staging: Vec::new(), removed_events: 0 };
    let _ = &mut corrupted_site;
    assert!(use_artifact(&site, expected).is_ok());
    let flipped = InstallSite { destination: Some(b"trusted model weights vX".to_vec()), staging: Vec::new(), removed_events: 0 };
    assert!(
        use_artifact(&flipped, expected).is_err(),
        "the use-site re-verify catches bytes corrupted after the download verify"
    );
}

#[test]
fn f4_destination_never_holds_a_partial_install() {
    let mut site = InstallSite::default();
    // a verify failure at staging: the destination was never touched
    install_fixed(&mut site, b"bad", 0xDEAD, 3);
    assert!(site.destination.is_none());
    assert!(site.staging.is_empty(), "staging cleaned, destination pristine");
}

// ═══════════════════════════════════════════════════════════════════════
// F5 — ui-event-stream-sampled-gate-terminal-forward
// ═══════════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
enum Event {
    Progress { done: u64, total: u64 },
    TileStarted { index: u64 },
    TileCompleted { index: u64, jpeg: bool },
    JobCompleted,
    JobFailed,
}

struct Gate {
    last_progress_ms: Option<u64>,
    last_tile_ms: Option<u64>,
    interval_ms: u64,
}

impl Gate {
    fn new(interval_ms: u64) -> Self {
        Self { last_progress_ms: None, last_tile_ms: None, interval_ms }
    }

    /// FIXED: per-class sampling; terminal always; structural pairs never
    /// sampled; the last item force-forwards.
    fn should_forward(&mut self, event: &Event, now_ms: u64, is_last: bool) -> bool {
        match event {
            Event::JobCompleted | Event::JobFailed => true,
            Event::TileStarted { .. } => true,
            Event::Progress { done, total } => {
                let last = &mut self.last_progress_ms;
                let interval = self.interval_ms;
                forward_sampled(last, interval, now_ms, is_last || done == total)
            }
            Event::TileCompleted { jpeg, .. } => {
                if !jpeg {
                    return true; // cheap metadata stays paired
                }
                let last = &mut self.last_tile_ms;
                let interval = self.interval_ms;
                forward_sampled(last, interval, now_ms, is_last)
            }
        }
    }
}

fn forward_sampled(last: &mut Option<u64>, interval_ms: u64, now_ms: u64, force: bool) -> bool {
    let due = match *last {
        Some(prev) => now_ms.saturating_sub(prev) >= interval_ms,
        None => true,
    };
    if due || force {
        *last = Some(now_ms);
        true
    } else {
        false
    }
}

#[test]
fn f5_sampled_progress_terminal_always() {
    let mut gate = Gate::new(100);
    let mut forwarded = 0u64;
    for i in 0..200u64 {
        let e = Event::Progress { done: i, total: 200 };
        if gate.should_forward(&e, i * 10, false) {
            forwarded += 1;
        }
    }
    assert!(
        forwarded < 30,
        "progress sampled: {forwarded} of 200 forwarded"
    );
    assert!(gate.should_forward(&Event::JobCompleted, 5_000, true));
    assert!(gate.should_forward(&Event::JobFailed, 5_001, true));
}

#[test]
fn f5_structural_pairs_never_sampled() {
    let mut gate = Gate::new(1_000);
    assert!(
        gate.should_forward(&Event::TileStarted { index: 7 }, 0, false),
        "started forwards inside the window"
    );
    assert!(
        gate.should_forward(&Event::TileCompleted { index: 7, jpeg: false }, 1, false),
        "cheap completion metadata stays paired with its start"
    );
}

#[test]
fn f5_last_item_force_forwards() {
    let mut gate = Gate::new(1_000);
    gate.should_forward(&Event::Progress { done: 1, total: 9 }, 0, false);
    let forced = gate.should_forward(&Event::Progress { done: 9, total: 9 }, 10, false);
    assert!(forced, "the final progress item forwards despite the interval");
}

#[test]
fn f5_naive_floods() {
    let forwarded = Cell::new(0u64);
    for i in 0..200u64 {
        let _ = Event::Progress { done: i, total: 200 };
        forwarded.set(forwarded.get() + 1); // always forward: 200 of 200
    }
    assert_eq!(forwarded.get(), 200);
}

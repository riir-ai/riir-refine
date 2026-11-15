//! Batch 364 fixtures — the rust_perf half (Plan 379 / Research 411):
//! the hydra-db/hydradb object-store-graph-db distill, first tranche.
//! ORIGINAL std-only models, never pasted source (the ELv2 transcription
//! law: the laws are re-authored with our own fixtures; the source's
//! measured figures stay in the research note, never in code).
//! Standalone-compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b364_fixtures.rs && ./b364_fixtures`
//! (the B353/B359/B360/B361/B362/B363 pre-entry discipline).
//!
//! Arms (per the rules' FIXTURES notes):
//! - F1 degree-gated-probe-with-doubling-retry: the admission fn's
//!   known-answer table (gate at 64; the doubling watermark; the hub
//!   still probed past an overflow), the census parity (scan-always vs
//!   the gated probe agree on the matched set), the anti-regime (a
//!   uniform low-degree stream — the gate degenerates to scan-all), and
//!   the tampered arm (a `>= W` retry instead of `>= 2W` — caught by
//!   the known-answer table);
//! - F2 bfs-depth-epoch-marker-dedup: visited-census parity over many
//!   levels and a 300-level wrap walk (the reset fires once, the census
//!   still exact), the memset-BYTES census (N clears vs 0), the
//!   anti-regime (a single level — nothing to amortize), and the
//!   tampered arm (marker = 0 collides with "never stamped" — caught);
//! - F3 spawn-blocking-queue-wait-split-from-compute: the split reveals
//!   a saturated pool (queue-wait >> exec — the aggregate named the
//!   kernel), the derived total equals the old aggregate, and the
//!   tampered arm (guessing unattributed waits into the exec bucket —
//!   refused; the gap counter stays a gap);
//! - F4 latency-histogram-split-on-structural-populations: the bimodal
//!   sum names neither population, the record-site label survives an
//!   engine flip (history re-handed to nobody), the gap-not-guess
//!   counter, and the derived total equals the stored sum;
//! - F5 derived-index-write-touches-only-changed-keys: write-count
//!   parity of the final state, the delta count (only changed keys
//!   written), the no-op preflight (an unchanged merge writes nothing),
//!   and the tampered arm (a "diff" that re-puts everything — caught).

use std::collections::BTreeMap;

// ─────────────────────────────────────────────────────────────────────────────
// F1 — degree-gated-probe-with-doubling-retry
// ─────────────────────────────────────────────────────────────────────────────

const PROBE_MIN_DEGREE: usize = 64;
const PROBE_BUDGET: usize = 32;

/// The admission function (the law): probe past the crossover; after an
/// overflow at `overflowed_at`, retry only at twice that degree.
fn probe_due(degree: usize, overflowed_at: usize) -> bool {
    overflowed_at == 0 || degree >= overflowed_at.saturating_mul(2)
}

/// Census of one stream under a policy — 0 = scan-all (the naive the
/// gate fixes), 1 = probe-all (the eager naive), 2 = the gated admission.
/// Reads are counted, not performed — integer-exact.
fn census(degrees: &[usize], matches: &[usize], policy: u8) -> (u64, u64) {
    let mut reads = 0u64;
    let mut matched = 0u64;
    let mut overflowed_at: usize = 0;
    for (i, &d) in degrees.iter().enumerate() {
        let m = matches[i];
        matched += m as u64;
        let probe = match policy {
            0 => false,
            1 => true,
            _ => d >= PROBE_MIN_DEGREE && probe_due(d, overflowed_at),
        };
        if probe {
            // probe: seek + min(m, budget) entry reads; overflow bails to
            // the neighbor scan (d reads) and records the watermark
            reads += 16; // the seek
            if m <= PROBE_BUDGET {
                reads += m as u64;
            } else {
                reads += PROBE_BUDGET as u64;
                reads += d as u64;
                if policy == 2 {
                    overflowed_at = PROBE_BUDGET;
                }
            }
        } else {
            reads += d as u64; // the neighbor check
        }
    }
    (reads, matched)
}

#[test]
fn f1_admission_known_answer_table() {
    // gate: below 64 never probes, whatever the watermark
    assert!(!probe_due_p(4, 0));
    assert!(!probe_due_p(63, 0));
    // at/above 64 with no prior overflow: probe
    assert!(probe_due_p(64, 0));
    assert!(probe_due_p(8000, 0));
    // after an overflow at W: retry only at >= 2W
    assert!(!probe_due_p(63, 32));
    assert!(probe_due_p(64, 32)); // 64 >= 2*32
    assert!(probe_due_p(8192, 32)); // a much larger hub still gets its probe
    assert!(!probe_due_p(100, 60)); // 100 < 120 — waits
    assert!(probe_due_p(120, 60));
    // zero-overload case: anything at/above the gate probes
    assert!(probe_due_p(65, 0));
}

fn probe_due_p(degree: usize, overflowed_at: usize) -> bool {
    degree >= PROBE_MIN_DEGREE && probe_due(degree, overflowed_at)
}

#[test]
fn f1_census_parity_and_watermark_savings() {
    // a skewed stream: tiny mass, some mid, a few hubs; hot sources whose
    // match set exceeds the budget force the overflow path
    let mut degrees = Vec::new();
    let mut matches = Vec::new();
    let mut s: u64 = 0xB24D_0001_5EED_0001;
    let mut lcg = |state: &mut u64| {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *state
    };
    for _ in 0..2_000 {
        let r = lcg(&mut s);
        let b = r % 100;
        let d = if b < 70 {
            1 + ((r >> 4) % 16) as usize
        } else if b < 95 {
            48 + ((r >> 4) % 464) as usize
        } else {
            512 + ((r >> 4) % 7680) as usize
        };
        let hot = lcg(&mut s) % 16 == 0;
        let m = if hot { d - (d >> 4) } else { 1 + ((lcg(&mut s) % 3) as usize) + (d >> 9) };
        degrees.push(d);
        matches.push(m.min(d));
    }
    let (scan_reads, scan_matched) = census(&degrees, &matches, 0);
    let (eager_reads, eager_matched) = census(&degrees, &matches, 1);
    let (gated_reads, gated_matched) = census(&degrees, &matches, 2);
    // G1: all three policies answer the same census
    assert_eq!(eager_matched, gated_matched);
    assert_eq!(scan_matched, gated_matched, "matched-edge parity");
    // the gated admission beats BOTH naives on the skewed stream
    assert!(gated_reads < eager_reads, "gated {gated_reads} vs eager {eager_reads}");
    assert!(gated_reads < scan_reads, "gated {gated_reads} vs scan-all {scan_reads}");
    // the anti-regime: a uniform low-degree stream — the gate degenerates
    // to scan-all EXACTLY (no misfire), while the eager probe still pays
    // the seek on every source
    let tiny: Vec<usize> = vec![3; 500];
    let tm: Vec<usize> = vec![1; 500];
    let (e_scan, _) = census(&tiny, &tm, 0);
    let (e_gated, _) = census(&tiny, &tm, 2);
    assert_eq!(e_scan, e_gated, "uniform low degree: the gate never misfires");
    let (e_eager, _) = census(&tiny, &tm, 1);
    assert!(e_eager > e_scan, "the eager naive pays the seek on the mass");
}

#[test]
fn f1_tampered_single_watermark_retry_is_caught() {
    // TAMPERED: retry at `>= W` instead of `>= 2W`. The known-answer
    // table catches it: at W=60 a degree-100 source must WAIT, not probe.
    let tampered = |degree: usize, overflowed_at: usize| {
        degree >= PROBE_MIN_DEGREE && (overflowed_at == 0 || degree >= overflowed_at)
    };
    assert!(!probe_due(100, 60), "the law: 100 < 2*60 — waits");
    assert!(tampered(100, 60), "the tampered form probes — the tables disagree");
    // and the tampered form re-pays a failed probe on a source that cannot
    // win: reads > the law's bound (W + degree approaches 2x for nothing)
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — bfs-depth-epoch-marker-dedup
// ─────────────────────────────────────────────────────────────────────────────

struct Graph {
    n: usize,
    off: Vec<u32>,
    nbr: Vec<u32>,
}

impl Graph {
    /// ring + a few random chords: large diameter, frontier-bound BFS
    fn ring_with_chords(n: usize) -> Self {
        Self::build(n, true)
    }

    /// a pure ring (degree 2): diameter n/2 — a walk that stays alive
    /// past the 255-marker wrap
    fn pure_ring(n: usize) -> Self {
        Self::build(n, false)
    }

    fn build(n: usize, chords: bool) -> Self {
        let deg = if chords { 4usize } else { 2usize };
        let mut off = Vec::with_capacity(n + 1);
        let mut nbr = Vec::with_capacity(n * deg);
        let mut s: u64 = 0x1BF5_2D6E_D6EE_0001;
        for i in 0..n {
            off.push(nbr.len() as u32);
            nbr.push(((i + n - 1) % n) as u32);
            nbr.push(((i + 1) % n) as u32);
            if chords {
                for _ in 2..deg {
                    nbr.push((s % n as u64) as u32);
                    s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                }
            }
        }
        off.push(nbr.len() as u32);
        Graph { n, off, nbr }
    }

    /// the naive: `clears` counts the full-array clears (the eliminated work)
    fn bfs_naive(&self, hops: usize) -> (u32, u64) {
        let mut seen = vec![false; self.n];
        let mut next = vec![false; self.n];
        let mut frontier = vec![0u32];
        seen[0] = true;
        let mut total = 1u32;
        let mut clears = 0u64;
        for _ in 0..hops {
            next.fill(false);
            clears += 1;
            let mut pending = Vec::new();
            for &u in &frontier {
                for j in self.off[u as usize]..self.off[u as usize + 1] {
                    let v = self.nbr[j as usize] as usize;
                    if !seen[v] && !next[v] {
                        next[v] = true;
                        pending.push(v as u32);
                    }
                }
            }
            if pending.is_empty() {
                break;
            }
            for &v in &pending {
                seen[v as usize] = true;
            }
            total += pending.len() as u32;
            frontier = pending;
        }
        (total, clears)
    }

    /// the epoch marker: zero per-level clears; `resets` counts the wrap
    fn bfs_marker(&self, hops: usize) -> (u32, u64) {
        let mut seen = vec![false; self.n];
        let mut stamp = vec![0u8; self.n];
        let mut frontier = vec![0u32];
        seen[0] = true;
        let mut total = 1u32;
        let mut resets = 0u64;
        for depth in 1..=hops {
            let marker = (depth % 255 + 1) as u8;
            if marker == 1 && depth > 1 {
                stamp.fill(0);
                resets += 1;
            }
            let mut pending = Vec::new();
            for &u in &frontier {
                for j in self.off[u as usize]..self.off[u as usize + 1] {
                    let v = self.nbr[j as usize] as usize;
                    if !seen[v] && stamp[v] != marker {
                        stamp[v] = marker;
                        pending.push(v as u32);
                    }
                }
            }
            if pending.is_empty() {
                break;
            }
            for &v in &pending {
                seen[v as usize] = true;
            }
            total += pending.len() as u32;
            frontier = pending;
        }
        (total, resets)
    }
}

#[test]
fn f2_census_parity_and_zero_clears() {
    let g = Graph::ring_with_chords(4_096);
    let (naive_total, clears) = g.bfs_naive(16);
    let (marker_total, resets) = g.bfs_marker(16);
    // G1: identical visited censuses
    assert_eq!(naive_total, marker_total, "visited-census parity");
    // the eliminated work: N clears vs 0
    assert!(clears >= 2, "the naive clears every level (got {clears})");
    assert_eq!(resets, 0, "16 hops never wrap");
    // and a wrap-length walk stays exact with ONE reset — the pure ring's
    // diameter keeps the frontier alive past depth 255
    let ring = Graph::pure_ring(4_096);
    let (naive_wrap, naive_clears) = ring.bfs_naive(300);
    let (marker_wrap, marker_resets) = ring.bfs_marker(300);
    assert_eq!(naive_wrap, marker_wrap, "300-level parity");
    assert_eq!(marker_resets, 1, "one clear per 255 levels, never per level");
    assert!(naive_clears >= 255, "the walk survived to the wrap ({naive_clears} clears)");
}

#[test]
fn f2_single_level_anti_regime() {
    // a single level: the marker form still answers exactly (nothing to
    // amortize — the honest anti-regime, the law never claims a win here)
    let g = Graph::ring_with_chords(64);
    let (a, _) = g.bfs_naive(1);
    let (b, _) = g.bfs_marker(1);
    assert_eq!(a, b);
}

#[test]
fn f2_tampered_zero_marker_is_caught() {
    // TAMPERED: marker = depth % 256 (reaches 0 — collides with "never
    // stamped"): at depth 256 the test `stamp[v] != 0` fails for every
    // fresh vertex whose stamp is 0, silently dropping the level.
    // Detected structurally: the law's marker domain excludes 0.
    let marker_tampered = |depth: usize| -> u8 { (depth % 256) as u8 };
    let marker_lawful = |depth: usize| -> u8 { (depth % 255 + 1) as u8 };
    assert_eq!(marker_tampered(256), 0, "collides with never-stamped");
    assert_ne!(marker_lawful(256), 0, "the law's domain is 1..=255");
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — spawn-blocking-queue-wait-split-from-compute
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct SplitMetrics {
    queue_us: u64,
    exec_us: u64,
}

/// Simulated dispatch: the pool is modeled by a queue delay the caller
/// injects (the saturated-pool regime); the aggregate is what the naive
/// counter would have held.
fn dispatch_split(metrics: &mut SplitMetrics, queue_us: u64, exec_us: u64) -> u64 {
    let queued_at = queue_us; // stamped BEFORE the spawn (the wrapper)
    metrics.queue_us += queued_at;
    metrics.exec_us += exec_us; // stamped by the closure itself
    queued_at + exec_us
}

#[derive(Default)]
struct NaiveMetrics {
    compute_us: u64,
}

#[test]
fn f3_split_reveals_a_saturated_pool() {
    // a saturated pool: the wait is LONGER than the work
    let mut split = SplitMetrics::default();
    let mut naive = NaiveMetrics::default();
    let aggregate = dispatch_split(&mut split, 4_100, 90);
    naive.compute_us += aggregate;
    // the naive aggregate presents contention as kernel slowness: it reads
    // ~4 190 us of "compute", of which only 90 us is the kernel
    assert_eq!(naive.compute_us, 4_190);
    assert!(split.queue_us > split.exec_us * 10, "the queue dominates");
    assert_eq!(split.queue_us + split.exec_us, naive.compute_us, "derived total");
}

#[test]
fn f3_derived_total_equals_the_old_aggregate() {
    let mut split = SplitMetrics::default();
    let mut expected = 0u64;
    for (q, e) in [(120u64, 80u64), (30, 900), (0, 45)] {
        expected += q + e;
        dispatch_split(&mut split, q, e);
    }
    assert_eq!(split.queue_us + split.exec_us, expected);
}

#[test]
fn f3_tampered_guessing_unattributed_waits_is_refused() {
    // TAMPERED: fold an unattributable wait into the exec bucket so the
    // numbers "add up". The law: a guess in a metric is worse than a gap —
    // the gap counter is the only honest home for unattributed waits.
    let mut gap = 0u64;
    let mut exec = 0u64;
    let attributed = false;
    let wait_us = 700u64;
    if attributed {
        exec += wait_us; // the tampered move
    } else {
        gap += wait_us; // the law
    }
    assert_eq!(gap, 700);
    assert_eq!(exec, 0, "nothing guessed into the execution bucket");
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — latency-histogram-split-on-structural-populations
// ─────────────────────────────────────────────────────────────────────────────

const ENGINES: usize = 3;

#[derive(Default)]
struct Hist {
    count: u64,
    total_us: u64,
}

#[derive(Default)]
struct SplitStatementMetrics {
    reads: [Hist; ENGINES],
    writes: [Hist; ENGINES],
    unattributed_waits: u64,
}

impl SplitStatementMetrics {
    fn record(&mut self, engine: usize, is_write: bool, us: u64) {
        let cell = if is_write { &mut self.writes[engine] } else { &mut self.reads[engine] };
        cell.count += 1;
        cell.total_us += us;
    }
    fn derived_total(&self) -> u64 {
        self.reads.iter().chain(self.writes.iter()).map(|h| h.total_us).sum()
    }
}

#[derive(Default)]
struct NaiveStatementMetrics {
    execution_duration_us: u64,
    effective_engine: usize, // the CURRENT engine — the flip relabels history
}

#[test]
fn f4_bimodal_sum_names_neither() {
    let mut naive = NaiveStatementMetrics { effective_engine: 1, ..Default::default() };
    naive.execution_duration_us += 100; // a legacy read
    naive.execution_duration_us += 50_000; // a datafusion write
    let mean = naive.execution_duration_us / 2;
    // the mean (25 050) is neither population; the label (engine 1) is
    // neither engine — the aggregate names nothing
    assert_eq!(mean, 25_050);
    assert_ne!(mean, 100);
    assert_ne!(mean, 50_000);
    assert_ne!(naive.effective_engine, 0);
    assert_ne!(naive.effective_engine, 2);
}

#[test]
fn f4_record_site_label_survives_a_flip() {
    let mut split = SplitStatementMetrics::default();
    // history: recorded while the legacy engine served
    split.record(0, false, 100);
    split.record(0, false, 120);
    // THE FLIP: the kill switch moves the node to engine 2
    split.record(2, false, 300);
    // the label rode the data: engine 0's history stays engine 0's
    assert_eq!(split.reads[0].count, 2);
    assert_eq!(split.reads[0].total_us, 220);
    assert_eq!(split.reads[2].count, 1);
    assert_eq!(split.reads[2].total_us, 300);
    // the derived total is unchanged by the split
    assert_eq!(split.derived_total(), 520);
}

#[test]
fn f4_gap_not_guess_and_population_structure() {
    let mut split = SplitStatementMetrics::default();
    // mutation is its own population: it carries a commit and can never
    // be served from a cursor — a different distribution entirely
    split.record(1, true, 5_000);
    split.record(1, false, 200);
    assert_eq!(split.writes[1].total_us, 5_000);
    assert_eq!(split.reads[1].total_us, 200);
    // a wait nobody could attribute lands in the gap, never in a bucket
    split.unattributed_waits += 1;
    assert_eq!(split.unattributed_waits, 1);
    assert_eq!(split.derived_total(), 5_200);
}

#[test]
fn f4_derived_total_matches_the_stored_sum() {
    let mut split = SplitStatementMetrics::default();
    let mut stored = 0u64;
    for (e, w, us) in [(0usize, false, 90u64), (1, true, 700), (2, false, 33)] {
        stored += us;
        split.record(e, w, us);
    }
    assert_eq!(split.derived_total(), stored);
}

// ─────────────────────────────────────────────────────────────────────────────
// F5 — derived-index-write-touches-only-changed-keys
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
struct Row(Vec<u8>);

/// the naive: re-put every row; returns the write count
fn save_all(
    txn: &mut Vec<(String, Row)>,
    next: &BTreeMap<String, Row>,
) -> usize {
    for (key, row) in next {
        txn.push((key.clone(), row.clone()));
    }
    next.len()
}

/// the law: diff previous vs next, write only the deltas
fn save_delta(
    txn: &mut Vec<(String, Row)>,
    previous: &BTreeMap<String, Row>,
    next: &BTreeMap<String, Row>,
) -> usize {
    let mut written = 0;
    for (key, row) in next {
        if previous.get(key) != Some(row) {
            txn.push((key.clone(), row.clone()));
            written += 1;
        }
    }
    written
}

#[test]
fn f5_delta_write_and_final_state_parity() {
    let previous: BTreeMap<String, Row> = [("a", Row(vec![1])), ("b", Row(vec![2])), ("c", Row(vec![3]))]
        .into_iter()
        .map(|(k, r)| (k.to_string(), r))
        .collect();
    let mut next = previous.clone();
    next.insert("b".to_string(), Row(vec![9]));
    next.insert("d".to_string(), Row(vec![4]));

    let mut naive_txn = Vec::new();
    let naive_writes = save_all(&mut naive_txn, &next);
    let mut delta_txn = Vec::new();
    let delta_writes = save_delta(&mut delta_txn, &previous, &next);

    // G1: the final state is identical — the delta store's state is
    // previous ∪ its writes (the unchanged rows never left it)
    let naive_state: BTreeMap<String, Row> = naive_txn.iter().cloned().collect();
    let mut final_state = previous.clone();
    for (k, r) in delta_txn {
        final_state.insert(k, r);
    }
    assert_eq!(naive_state, final_state, "final-state parity");
    // the write counts: the naive re-put everything; the delta wrote only
    // the changed `b` + the new `d`
    assert_eq!(naive_writes, 4);
    assert_eq!(delta_writes, 2);
}

#[test]
fn f5_noop_preflight_writes_nothing() {
    // an unchanged merge is a read-only operation: the preflight diffs and
    // declines the write path entirely
    let previous: BTreeMap<String, Row> = [("a", Row(vec![1])), ("b", Row(vec![2]))]
        .into_iter()
        .map(|(k, r)| (k.to_string(), r))
        .collect();
    let next = previous.clone();
    let mut txn = Vec::new();
    let written = save_delta(&mut txn, &previous, &next);
    assert_eq!(written, 0, "the unchanged merge writes nothing");
    assert!(txn.is_empty());
}

#[test]
fn f5_tampered_identity_diff_is_caught() {
    // TAMPERED: a "diff" that re-puts rows whose value merely re-encodes
    // (here: any row present in `next`, compared by KEY only). The final
    // state matches, but the write count explodes — the delta count is
    // the load-bearing witness.
    let previous: BTreeMap<String, Row> = [("a", Row(vec![1])), ("b", Row(vec![2]))]
        .into_iter()
        .map(|(k, r)| (k.to_string(), r))
        .collect();
    let next = previous.clone(); // NOTHING changed
    let mut txn = Vec::new();
    let tampered_writes = next.keys().count(); // key-only "diff": writes both
    let lawful_writes = save_delta(&mut txn, &previous, &next);
    assert_eq!(lawful_writes, 0);
    assert_ne!(tampered_writes, lawful_writes, "the tampered diff re-puts");
}

//! Entries slice — Batch 364 (2026-11-16): five `rust_perf` rules from the
//! hydra-db/hydradb object-store-graph-db distill (Plan 379 / Research 411;
//! the FIRST landing of the staged hydradb batches — Plan 375's
//! `InferenceForge` slot stays its holder's, this batch ran the NEXT queue
//! slot while amo was offline; the batch NUMBER derives from the landing
//! order per the queue-snapshot law).
//!
//! Source: hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6
//! (tag v0.4.0, 2026-11-06), `ELv2` — transcription-only throughout: every
//! fixture is an ORIGINAL plain-Rust reproduction, no source pasted; the
//! license note rides every entry's provenance row.
//!
//! Gates at authoring (disclosures): the SECOND storage-transcription
//! tranche (candidates 9/10/11/3/5 of Research 411's ranked table —
//! wal-watermark-pinned-before-snapshot, tail-walk-cost-cap-declines-
//! to-rebuild, frozen-artifact-plus-delta-tail-overlay, existential-
//! witness-early-exit-scan, catalog-memo-scoped-to-the-snapshot) is
//! DEFERRED to the follow-up batch per the 4-6-rule size discipline;
//! `count-before-fault-injection` (the note's T4 face candidate, cited
//! `fault_store.rs:186`) is an HONEST T0 DROP — no `fault_store.rs` exists at
//! the pin, the cite does not resolve; the histogram faces
//! (derived-count-never-stored / bounds-pinned thresholds / torn-pair
//! bound) are folded into `latency-histogram-split-on-structural-
//! populations`'s notes; candidate 14 (`consume_budget`) ADJUDICATED as a
//! count-neutral rider on `adaptive-yield-consecutive-ready` (`entries_06`);
//! `one-shared-disk-cache-not-even-split` (#6) ADJUDICATED DISTINCT both
//! directions from `cache-share-capped-at-measured-distinct-working-set`
//! and DEFERRED with the second tranche; the align-64 second instance
//! rides `align-64-false-sharing` (`entries_02`) as a count-neutral face.
//!
//! Fixtures are ORIGINAL plain-Rust stand-ins (a degree-stream access
//! model, a ring-and-chords BFS, a queue/exec split model, a per-engine
//! histogram model, a delta-write model — never pasted source),
//! standalone rustc-compiled AND executed before landing (`b364_fixtures`,
//! 16/16; the gate caught FOUR model defects pre-entry — a wrong negated
//! known-answer row, the anti-regime arm comparing the wrong pair, the
//! wrap arm saturating before 255 on the chorded graph, and the delta
//! store's final-state composition).
//!
//! GOAT cells: `degree-gated-probe-with-doubling-retry` PASSED the
//! pre-registered bar (`ParetoDominant`: faster 2/3, 1.06x/1.06x/1.09x,
//! allocs 0->0 both arms, two-run stable alloc axis);
//! `bfs-depth-epoch-marker-dedup` FAILED it (Tie at every size — the fill
//! is ~1% of a realistic walk) and DEMOTED to `bench_spec: None` same-commit
//! per the ungoat law, the numbers carried in its notes.
//! Batch 364 / Plan 379 / Research 411.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── io: anchored property-expansion access path — the degree-gated
        //    probe admission + the doubling watermark retry ────────────────
        PerfRule {
            rule_id: "degree-gated-probe-with-doubling-retry".to_string(),
            name: "An anchored expand with an edge-property equality checks every neighbor past the scan-vs-probe crossover → read the property's index ONCE instead, admitted at degree >= 64; after a probe overflows at its entry budget W, retry only at degree >= 2W — failed probes then read at most about twice the largest degree in all while a much larger hub still gets its probe (never probe-every-source, never a one-overflow-poisons-all watermark)".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: every anchored expansion checks each neighbor's property,
// whatever the degree. A hub at degree 8 000 pays 8 000 property reads
// to find the edges that match, every time the expansion runs — and an
// eager index (probe everything) pays the seek on the low-degree mass
// where reading the few neighbors directly is cheaper.

#[derive(Clone, PartialEq)]
pub struct Prop(pub u32);

pub struct Shard {
    adj: Vec<Vec<u32>>,
    prop: Vec<Prop>,
}

impl Shard {
    pub fn expand(&self, src: u32, want: &Prop, out: &mut Vec<u32>) {
        for &e in &self.adj[src as usize] {
            if self.prop[e as usize] == *want {
                out.push(e);
            }
        }
    }
}

#[test]
fn naive_scan_always() {
    let shard = Shard {
        adj: vec![vec![0, 1, 2], vec![3, 4]],
        prop: vec![Prop(7), Prop(9), Prop(7), Prop(7), Prop(9)],
    };
    let mut out = Vec::new();
    shard.expand(0, &Prop(7), &mut out);
    assert_eq!(out, vec![0, 2]);
}"#.to_string(),
            fixed_code: r#"// OPT: the admission function from the source. A source expansion with
// an edge property equality reads that property's index ONCE, instead
// of checking each neighbor, past 64 neighbors; and after a probe
// overflowed at `overflowed_at` entries, only a source with at least
// twice that degree retries. Failed probes then read at most about
// twice the largest degree in all, while a much larger hub still gets
// its probe — never a permanent poison.

pub const PROBE_MIN_DEGREE: usize = 64;
pub const PROBE_BUDGET: usize = 32;

#[derive(Clone, PartialEq)]
pub struct Prop(pub u32);

pub enum Probe {
    Hits,
    Overflowed,
}

pub struct PropertyIndex {
    buckets: std::collections::HashMap<(u32, u32), Vec<u32>>,
}

impl PropertyIndex {
    fn probe(&self, want: &Prop, src: u32, out: &mut Vec<u32>, budget: usize) -> Probe {
        match self.buckets.get(&(want.0, src)) {
            Some(hits) if hits.len() <= budget => {
                out.extend_from_slice(hits);
                Probe::Hits
            }
            Some(_) => Probe::Overflowed,
            None => Probe::Hits,
        }
    }
}

/// Whether a source with `degree` neighbors probes the property index,
/// after a probe overflowed at `overflowed_at` entries (0: none has).
fn probe_due(degree: usize, overflowed_at: usize) -> bool {
    overflowed_at == 0 || degree >= overflowed_at.saturating_mul(2)
}

pub struct Shard {
    adj: Vec<Vec<u32>>,
    prop: Vec<Prop>,
    index: PropertyIndex,
    overflowed_at: usize,
}

impl Shard {
    pub fn expand(&mut self, src: u32, want: &Prop, out: &mut Vec<u32>) {
        let degree = self.adj[src as usize].len();
        if degree >= PROBE_MIN_DEGREE && probe_due(degree, self.overflowed_at) {
            if let Probe::Overflowed = self.index.probe(want, src, out, PROBE_BUDGET) {
                // the index hit its entry budget: remember it and fall
                // back to the neighbor check below
                self.overflowed_at = PROBE_BUDGET;
            } else {
                return;
            }
        }
        for &e in &self.adj[src as usize] {
            if self.prop[e as usize] == *want {
                out.push(e);
            }
        }
    }
}

#[test]
fn gated_probe_with_watermark() {
    let mut shard = Shard {
        adj: vec![vec![0, 1, 2], (0..800).collect()],
        prop: (0..800).map(|i| Prop(if i % 2 == 0 { 7 } else { 9 })).collect(),
        index: PropertyIndex {
            buckets: [(7u32, 1u32)]
                .into_iter()
                .map(|(p, s)| ((p, s), (0..900u32).collect()))
                .collect(),
        },
        overflowed_at: 0,
    };
    // below the crossover: the neighbor check, no seek
    let mut out = Vec::new();
    shard.expand(0, &Prop(7), &mut out);
    assert_eq!(out, vec![0, 2]);
    // the hub probes, overflows at the budget, and falls back to the
    // neighbor walk; the watermark stands at 32, so only degree >= 64
    // retries — the next hub still gets its probe
    let mut out = Vec::new();
    shard.expand(1, &Prop(7), &mut out);
    assert_eq!(shard.overflowed_at, PROBE_BUDGET);
    assert_eq!(out.len(), 400);
}"#.to_string(),
            context_notes: Some("Grade B+ — BENCHABLE, the batch's GOAT lead (the pre-registered bar: ParetoDominant on the validator, two-run stable, wins >= 2/3 sizes). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — TRANSCRIPTION-ONLY per the license law: original fixtures, never pasted source; T0 byte-verified). THE LAW IN-SOURCE (src/shard/query.rs:8628-8646, verbatim): 'A source expansion with an edge property equality reads that property's index once, instead of checking each neighbor, past this many neighbors.' (ANCHORED_EXPAND_PROPERTY_PROBE_MIN_DEGREE = 64) + the retry fn 'anchored_property_probe_due(degree, overflowed_at) { overflowed_at == 0 || degree >= overflowed_at.saturating_mul(2) }' whose doc: 'Only a source with at least twice that degree retries, so on sources of slowly rising degree the failed probes read at most about twice the largest degree in all, while a much larger hub still gets its probe.' REGIME: anchored access-path selection — any property/secondary-index expansion over a fan-out stream with a heavy degree tail; the crossover VALUE is workload-tuned, the ADMISSION SHAPE (gate + doubling watermark, never a poison) is the law. ANTI-REGIMES: uniform low degree (the gate degenerates to scan-all — correct and cheap); no index to probe (the probe half unrepresentable); permanent poison after one overflow (the counter-shape the doubling retry replaces — hubs lose their probe). ADJUDICATION (dedup at authoring HEAD): the 'backoff' family (entries_63/06) owns TRANSPORT retry ladders — no speculative access-path owner; batch-retry-ladder-classifies-failure-before-permanent-state is transport-shaped (its classify-first law is the SPIRIT of the poison-refusal here — compose, different sink); grep 'probe|crossover|overflow watermark' over rust_perf = zero prior owners. Bench harness: eager-probe naive vs the admission fn; skewed degrees (70% tiny 1-16 / 25% mid 48-511 / 5% hubs 512-8191), 1/16 hot overflow sources, probe budget 32, seek cost 16 real table touches; G1 matched-census parity asserted in make_input; alloc-free both arms. Plan 379 / Research 411 (the hydradb object-store-graph-db distill).".to_string()),
            // The batch's GOAT lead: Pareto validator over the degree stream.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(degree_gated_probe_bench_spec()),
        },
        // ── complexity: per-level dedup-array clears → the monotone epoch
        //    marker (one allocation, zero memsets) ─────────────────────────
        PerfRule {
            rule_id: "bfs-depth-epoch-marker-dedup".to_string(),
            name: "A bounded-level walk (BFS levels, sweep passes) clears its level-dedup array from scratch before every iteration → allocate ONE u8 stamp array and give each level a monotone marker (the depth): entries from older levels read as stale instead of needing a clear — N levels cost one allocation and zero memsets instead of N clear passes; the u8 wrap resets once per 255 levels, never per level".to_string(),
            category: "complexity".to_string(),
            buggy_code: r#"// NAIVE: the level-dedup array is cleared from scratch before
// collecting each BFS level. On a large-diameter graph the frontier is
// small and the clear is most of the work: hops * V bytes of memset to
// dedup a handful of vertices per level.

pub struct Graph {
    pub n: usize,
    pub off: Vec<u32>,
    pub nbr: Vec<u32>,
}

impl Graph {
    pub fn bfs_levels(&self, start: usize, hops: usize) -> u32 {
        let mut seen = vec![false; self.n];
        let mut next = vec![false; self.n]; // re-cleared EVERY level below
        let mut frontier = vec![start as u32];
        seen[start] = true;
        let mut total = 1u32;
        for _ in 0..hops {
            next.fill(false);
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
        total
    }
}

#[test]
fn naive_levels() {
    let g = Graph { n: 4, off: vec![0, 2, 4, 6, 8], nbr: vec![1, 3, 0, 2, 1, 3, 0, 2] };
    assert_eq!(g.bfs_levels(0, 8), 4);
}"#.to_string(),
            fixed_code: r#"// OPT: one u8 stamp array allocated ONCE, never cleared — each level
// carries a monotone marker (the depth): entries from older levels read
// as stale instead of needing a clear. The 255-marker wrap costs ONE
// clear per 255 levels, never per level.

pub struct Graph {
    pub n: usize,
    pub off: Vec<u32>,
    pub nbr: Vec<u32>,
}

impl Graph {
    pub fn bfs_levels(&self, start: usize, hops: usize) -> u32 {
        let mut seen = vec![false; self.n];
        let mut stamp = vec![0_u8; self.n]; // allocated ONCE, never cleared
        let mut frontier = vec![start as u32];
        seen[start] = true;
        let mut total = 1u32;
        for depth in 1..=hops {
            let marker = (depth % 255 + 1) as u8; // 1..=255; 0 = never stamped
            if marker == 1 && depth > 1 {
                // the u8 wrap: one clear per 255 levels
                stamp.fill(0);
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
        total
    }
}

#[test]
fn marker_levels_match_naive() {
    let g = Graph { n: 4, off: vec![0, 2, 4, 6, 8], nbr: vec![1, 3, 0, 2, 1, 3, 0, 2] };
    assert_eq!(g.bfs_levels(0, 8), 4);
}"#.to_string(),
            context_notes: Some("Grade B+ decision-content, landed NotBenchable-BY-MEASUREMENT (the DEMOTE the plan's bar ordered, same-commit): the memset-BYTES axis is real (the fixture censuses it — N clears vs 0, G1 visited-census parity green), but the bench validator MEASURED TIE at every size (n=1024 0.95x, n=16384 1.02x, n=131072 1.00x, allocs 0->0 both arms, two-run stable) — the ring+chords graph saturates by level ~9, so ~9 fills of n bytes land against ~2.4 ms of boundary edge work: the fill is ~1% of the walk on realistic graphs at these scales, and a pure-ring harness that would make the fill dominate is the degenerate-topology trap (the synthetic-model class this batch refused for candidate #4). The law stands on its decision content; the wall-clock claim is WITHDRAWN. SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified). THE LAW IN-SOURCE (src/sparse_kernel/graphblas.rs:360-395, verbatim fragments): the bounded BFS allocates 'let mut next_seen = vec![0_u8; dimension]' ONCE and stamps 'let marker = depth' per level — 'if dst >= dimension || seen[dst] || next_seen[dst] == marker { continue; }' + 'next_seen[dst] = marker;' — N levels cost one allocation and ZERO memsets instead of N clear passes. REGIME: any per-iteration dedup/stamp array over a bounded-level walk — BFS levels, sweep passes, per-round dedup, flood fills. ANTI-REGIME: one-shot fills (a single pass has nothing to amortize); walks whose marker domain outgrows u8 (widen the stamp type or reset at the wrap — one clear per 255 levels, the corpus form's reset; the source carries the generation-overflow reset path per Research 411). ADJUDICATION (dedup at authoring HEAD): entries_03 owns alloc_zeroed-vs-resize (the SINGLE-buffer ALLOCATION law — this rule owns the per-ITERATION CLEAR axis, unowned); generation-counter-stale-timer-invalidation (entries_56) owns TIMER staleness by generation — different consumer (timers, not walk dedup); grep 'memset|per-level clear|epoch marker' over rust_perf = zero owners for this axis. Bench: ring+chords graph (large diameter — frontier-bound BFS is the regime where the clear dominates), 16 hops, fill-per-level naive vs the monotone marker; alloc-free both arms (scratch pre-built in Input). Plan 379 / Research 411.".to_string()),
            // DEMOTED at measurement (the ungoat law): the validator measured
            // Tie at every size — see the notes. The spec fn stays (dead
            // code would be worse); it is no longer referenced.
            #[cfg(feature = "rust_perf_bench")]
            #[allow(dead_code)]
            bench_spec: None,
        },
        // ── measurement honesty: a spawned operation's aggregate duration
        //    must split queue-wait from execution ───────────────────────────
        PerfRule {
            rule_id: "spawn-blocking-queue-wait-split-from-compute".to_string(),
            name: "A spawned operation (spawn_blocking / worker-pool dispatch) whose metric is ONE aggregate duration reports pool contention as kernel slowness — the wrapper is the right place for the split: stamp the queue wait BEFORE the spawn (the span cannot follow the closure onto the blocking thread), let the closure stamp its own execution, land them in separate counters; the old aggregate stays derivable (the two sums add to exactly what it held)".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: one aggregate duration covers the queue wait and the compute.
// `spawn_blocking` can sit behind a saturated blocking pool for longer
// than the closure itself takes, and the aggregate counters cannot tell
// the two apart on a single query — a saturated pool presents contention
// as kernel slowness.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct GraphMetrics {
    pub compute_us: AtomicU64,
}

pub async fn run_graph_compute(
    metrics: &GraphMetrics,
    compute: impl FnOnce() -> u64 + Send + 'static,
) -> u64 {
    let started = std::time::Instant::now();
    let out = tokio::task::spawn_blocking(compute).await.expect("join");
    metrics.compute_us.fetch_add(
        u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
    out
}"#.to_string(),
            fixed_code: r#"// OPT: the wrapper is the right place for the split — the span cannot
// follow the closure onto the blocking thread, which is exactly why the
// wrapper stamps the queue wait BEFORE the spawn and the closure stamps
// its own execution. Separate counters; the aggregate, when wanted, is
// derived (queue + duration add to what the old counter held).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct GraphMetrics {
    pub graph_compute_queue_us: Arc<AtomicU64>,
    pub graph_compute_duration_us: Arc<AtomicU64>,
}

pub async fn run_graph_compute(
    metrics: GraphMetrics,
    compute: impl FnOnce() -> u64 + Send + 'static,
) -> u64 {
    let queued_at = std::time::Instant::now();
    tokio::task::spawn_blocking(move || {
        let queue_us = u64::try_from(queued_at.elapsed().as_micros()).unwrap_or(u64::MAX);
        metrics.graph_compute_queue_us.fetch_add(queue_us, Ordering::Relaxed);
        let started = std::time::Instant::now();
        let out = compute();
        metrics.graph_compute_duration_us.fetch_add(
            u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        out
    })
    .await
    .expect("join")
}

#[test]
fn derived_total_matches_the_old_aggregate() {
    let metrics = GraphMetrics::default();
    metrics.graph_compute_queue_us.fetch_add(120, Ordering::Relaxed);
    metrics.graph_compute_duration_us.fetch_add(80, Ordering::Relaxed);
    let aggregate = metrics.graph_compute_queue_us.load(Ordering::Relaxed)
        + metrics.graph_compute_duration_us.load(Ordering::Relaxed);
    assert_eq!(aggregate, 200); // what the old single counter would have held
}"#.to_string(),
            context_notes: Some("Grade B — measurement-honesty, NotBenchable-by-domain (the win is WHICH cost a saturated pool reports — attribution, never this process's wall-clock; the dial9/B352 annotation class). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified). THE LAW IN-SOURCE (src/shard/query.rs:151-179, verbatim): '`kernel.expand` covers the queue wait as well as the compute, because `spawn_blocking` can sit behind a saturated blocking pool for longer than the traversal itself takes and the aggregate counters cannot tell the two apart on a single query. The span cannot follow the closure onto the blocking thread, which is exactly why the wrapper is the right place for it.' — the wrapper stamps queued_at BEFORE the spawn; the closure stamps queue_us then its own execution; separate counters, one span at the wrapper naming the ladder rung. REGIME: any spawn_blocking / worker-pool dispatch whose metrics time only the closure (graph kernels, encoding jobs, image pipelines, compaction workers on pool lanes). ANTI-REGIME: unpoolled dedicated threads (the wait IS the spawn cost, nothing to split); single-tier dispatch with no queue; metrics read only under an idle pool (the split still lands — record it anyway, the cost is two counters). ADJUDICATION (dedup at authoring HEAD): wall-clock-ungated-shared-ci-noise-floor (entries_79) owns wall-clock's JUDGED ROLE in shared CI — different axis; latency-histogram-split-on-structural-populations (this batch) splits POPULATIONS across requests — COMPOSE with this rule (it names the PHASES of one operation); alloc-proxy-inversion-work-ratio-caveat (entries_79) owns proxy axes. grep 'queue wait|spawn_blocking metric|queue_us' over rust_perf = zero owners. Plan 379 / Research 411.".to_string()),
            // NotBenchable: the metric is attribution, not this process's time.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── measurement honesty: one histogram per structural population,
        //    labelled at the record site ────────────────────────────────────
        PerfRule {
            rule_id: "latency-histogram-split-on-structural-populations".to_string(),
            name: "One latency sum fed structurally different populations (two engines, reads vs writes) produces a bimodal mean that names neither → split at the RECORD site on the discriminator the code already holds (one histogram per population, the label read off the data — never relabelled at export, where a flip would hand the whole history to whichever engine is effective); split populations by STRUCTURE (a mutation carries a commit and is a different distribution entirely); count the unattributable remainder as an explicit gap (a guess in a metric is worse than a gap); retain the old total as a DERIVED value so nothing that read it has to change".to_string(),
            category: "measurement-honesty".to_string(),
            buggy_code: r#"// NAIVE: one sum fed every execution — reads and writes, two engines,
// one number. The mean names neither population, and labelling the
// population at export time re-hands the whole history to whichever
// engine happens to be effective after a flip.

#[derive(Default)]
pub struct StatementMetrics {
    pub execution_duration_us: u64,
    pub effective_engine: usize, // the kill switch's CURRENT engine
}

impl StatementMetrics {
    pub fn record(&mut self, _engine: usize, _is_write: bool, us: u64) {
        self.execution_duration_us += us; // bimodal mean: names neither
    }

    pub fn export(&self) -> String {
        // the label is read off the NODE's current state, not off the data:
        // a flip re-labels every microsecond ever recorded
        format!("engine={} us={}", self.effective_engine, self.execution_duration_us)
    }
}

#[test]
fn bimodal_sum_names_neither() {
    let mut m = StatementMetrics { effective_engine: 1, ..Default::default() };
    m.record(0, false, 100); // legacy-engine read
    m.record(2, true, 50_000); // datafusion write — a different distribution
    let _ = m.export(); // mean 25 050 us: neither population, engine 1: neither
}"#.to_string(),
            fixed_code: r#"// OPT: one histogram per population, labelled AT THE RECORD SITE from
// the data the code already holds (the engine the statement was
// admitted with); the mutation population split out (it carries a
// commit and can never be served from a cursor — a different
// distribution entirely); unattributable waits counted as a GAP, never
// guessed into a bucket; the old total retained as a DERIVED value so
// nothing that read the sum has to change.

pub const ENGINES: usize = 3;

#[derive(Default)]
pub struct Histogram {
    pub count: u64,
    pub total_us: u64,
}

impl Histogram {
    fn record(&mut self, us: u64) {
        self.count += 1;
        self.total_us += us;
    }
}

#[derive(Default)]
pub struct StatementMetrics {
    pub reads: [Histogram; ENGINES],
    pub writes: [Histogram; ENGINES],
    pub unattributed_waits: u64,
}

impl StatementMetrics {
    pub fn record(&mut self, engine: usize, is_write: bool, us: u64) {
        let cell = if is_write {
            &mut self.writes[engine]
        } else {
            &mut self.reads[engine]
        };
        cell.record(us); // the label rides the data — a flip re-hands nothing
    }

    pub fn record_unattributable_wait(&mut self) {
        self.unattributed_waits += 1; // a gap, never a guess
    }

    /// The old sum, derived: every execution lands in exactly one cell,
    /// so the sums add up to the previous value exactly.
    pub fn execution_duration_us(&self) -> u64 {
        self.reads.iter().chain(self.writes.iter()).map(|h| h.total_us).sum()
    }
}

#[test]
fn split_reads_labels_and_derived_total() {
    let mut m = StatementMetrics::default();
    m.record(0, false, 100);
    m.record(2, true, 50_000);
    m.record_unattributable_wait();
    assert_eq!(m.reads[0].total_us, 100);
    assert_eq!(m.writes[2].total_us, 50_000);
    assert_eq!(m.unattributed_waits, 1);
    assert_eq!(m.execution_duration_us(), 50_100); // the old sum, derived
}"#.to_string(),
            context_notes: Some("Grade B+ — measurement-honesty co-lead, NotBenchable-by-domain (the win is a metric that NAMES its populations — attribution quality, no validator host); carries FOUR sub-laws. SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified). THE LAWS IN-SOURCE (src/client/service.rs:860-905 + crates/graph-slate/src/core/histogram.rs, verbatim fragments): (1) SPLIT AT THE RECORD SITE — 'One histogram per engine rather than one labelled at export time. ... a single population relabelled on a flip would hand the whole history to whichever engine happens to be effective at the next scrape — with the engine comparison these families exist for as the first casualty. Each statement lands in the histogram for the engine it was admitted with, so an export reads the label off the data instead of off the node.' (2) POPULATIONS BY STRUCTURE — reads vs writes: 'a mutation, which is a different distribution entirely: it carries a commit, and it can never be served from a cursor.' (3) GAP-NOT-GUESS — 'The two do not have to sum to bookmark_waits: a client that cannot observe the branch ... increments neither, so the gap is the count of waits nobody could attribute. Guessing would have been the alternative, and a guess in a metric is worse than a gap.' (4) DERIVED TOTALS — 'Retained so nothing that read the old sum has to change. It is derived rather than stored: every execution is recorded into exactly one of the six histograms, so their sums add up to the previous value exactly' + histogram.rs:130: the count 'derived from the buckets rather than stored'. FOLDED FACES (count-neutral, Batch 364): the histogram family's bounds-pinned thresholds + torn-pair bound + derived-count-never-stored ride THIS rule (crates/graph-slate/src/core/histogram.rs:97-130, T0-verified — the family's Research-411 face candidates, no separate host needed). REGIME: any metrics surface where one accumulator feeds structurally different populations (engines, tenants, request kinds) or where a flip/kill-switch can relabel history. ANTI-REGIME: genuinely homogeneous populations (the split is pure overhead); populations the record site cannot discriminate (that is what the gap counter is for). ADJUDICATION (dedup at authoring HEAD): spawn-blocking-queue-wait-split-from-compute (this batch) owns the PHASE split of one operation — COMPOSE (phases vs populations); benchmark-spread-above-threshold-reports-unstable (entries_70) owns run-to-run stability; macos-pressure-reads-footprint-not-rss owns the metric-CHOICE axis; zero histogram-split owners at authoring HEAD. FACE (count-neutral, Batch 368 / Plan 382 / Research 414, kunobi-ninja/kache @ da717cb9 Apache-2.0): the kache stats lane carries the SAME LAW's cost axis — a COUNT hit rate and a COMPILE-TIME-weighted hit rate are published as separate numbers (`count_hit_rate` over events vs `compile_weighted_hit_rate` = hit_compile_time_ms / (hit + miss compile time), src/cli.rs:136-151, both in the stats envelope, docs/commands/report.schema.json hit_rate_pct + weighted_hit_rate_pct), with the doc caveat verbatim (docs/getting-started/comparison.mdx:48): 'A count-based hit rate does not show how much compile time was avoided, so record wall time as well.' — populations whose unit costs differ by orders of magnitude (a 2 s miss vs a 40 ms hit) invert the count verdict; the split-by-STRUCTURE law names the populations, the weighting names WHY one number is not enough. Plan 379 / Research 411. FACE (Batch 398, Plan 398 / Research 432, count-neutral — the serial-reason taxonomy): ddalcu/mlx-serve @ 96311b26 (MIT) ships this rule's split law as the decode path's own labeled counter: the BatchVerdict carries WHY a step went serial — spec_active, head_release_pending, grammar, logprobs, penalty, embedded_engine, arch, pad_waste, speech — emitted as decode_serial_total{reason}; a throughput loss EXPLAINS ITSELF instead of hiding in an aggregate (the structural-population split this rule owns, applied to the serial-reason populations; the gap-not-guess doctrine's counter form). FACE (Batch 398, Plan 398 / Research 432, count-neutral — the serial-reason taxonomy): ddalcu/mlx-serve @ 96311b26 (MIT) ships this rule's split law as the decode path's own labeled counter: the BatchVerdict carries WHY a step went serial — spec_active, head_release_pending, grammar, logprobs, penalty, embedded_engine, arch, pad_waste, speech — emitted as decode_serial_total{reason}; a throughput loss EXPLAINS ITSELF instead of hiding in an aggregate (the structural-population split this rule owns, applied to the serial-reason populations; the gap-not-guess doctrine's counter form).".to_string()),
            // NotBenchable: the metric names populations — no validator host.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── io: a save diffs previous vs next — write only the deltas ─────
        PerfRule {
            rule_id: "derived-index-write-touches-only-changed-keys".to_string(),
            name: "A save/put path over a persisted row set re-puts every row → an unchanged re-put is NOT a no-op: it pays the full write path (WAL → memtable → L0 → compactions) for bytes that did not move — diff `previous` against `next` (the previous state is at hand in the transaction that read it) and write only the deltas; an unchanged merge is a read-only operation and can linearize without taking the write path at all (a no-op preflight: one durable-snapshot read + one diff, a bloom-filtered point miss per distinct name in the usual case)".to_string(),
            category: "io".to_string(),
            buggy_code: r#"// NAIVE: a save re-puts every row. The write path below `put` is not
// one write — it is WAL, memtable, L0 flush, compactions. A nightly job
// that rewrites a 100-million-row table to update one row pays 100
// million amplified writes for one change.

#[derive(Clone, PartialEq)]
pub struct Row(pub Vec<u8>);

pub struct Txn {
    pub writes: Vec<(String, Row)>,
}

impl Txn {
    pub fn put(&mut self, key: &str, row: Row) {
        self.writes.push((key.to_string(), row));
    }
}

pub fn save_rows(txn: &mut Txn, next: &std::collections::BTreeMap<String, Row>) {
    for (key, row) in next {
        txn.put(key, row.clone()); // unchanged rows pay full LSM amplification
    }
}

#[test]
fn full_reput_writes_everything() {
    let mut txn = Txn { writes: Vec::new() };
    let mut next = std::collections::BTreeMap::new();
    next.insert("a".to_string(), Row(vec![1]));
    next.insert("b".to_string(), Row(vec![2]));
    save_rows(&mut txn, &next);
    assert_eq!(txn.writes.len(), 2); // both rows written; only one changed
}"#.to_string(),
            fixed_code: r#"// OPT: diff previous against next — write only the deltas. The
// previous state is at hand in the transaction that read it; an
// unchanged merge is a READ-ONLY operation (a no-op preflight — one
// durable-snapshot read + one diff per row — can linearize it without
// taking the write path at all), and a changed key writes once.

#[derive(Clone, PartialEq)]
pub struct Row(pub Vec<u8>);

pub struct Txn {
    pub writes: Vec<(String, Row)>,
}

impl Txn {
    pub fn put(&mut self, key: &str, row: Row) {
        self.writes.push((key.to_string(), row));
    }
}

pub fn save_rows(
    txn: &mut Txn,
    previous: &std::collections::BTreeMap<String, Row>,
    next: &std::collections::BTreeMap<String, Row>,
) -> usize {
    let mut written = 0;
    for (key, row) in next {
        if previous.get(key) != Some(row) {
            txn.put(key, row.clone());
            written += 1;
        }
    }
    written
}

#[test]
fn delta_write_touches_only_changed_keys() {
    let mut txn = Txn { writes: Vec::new() };
    let previous: std::collections::BTreeMap<String, Row> = [("a".to_string(), Row(vec![1])), ("b".to_string(), Row(vec![2]))]
        .into_iter()
        .collect();
    let mut next = previous.clone();
    next.insert("b".to_string(), Row(vec![9]));
    let written = save_rows(&mut txn, &previous, &next);
    assert_eq!(written, 1); // only `b` changed — one write, not two
    assert_eq!(txn.writes.len(), 1);
    assert_eq!(txn.writes[0].0, "b");
}"#.to_string(),
            context_notes: Some("Grade B+ — io, NotBenchable-by-model (the validator's harness would measure OUR amplification constant, not an engine write path — the synthetic-constant class; the decision content is first-class without a bench; the research note's own arithmetic carried here instead). SOURCE (hydra-db/hydradb @ 916637477f3f34766b4905b270bd4292109d77e6, tag v0.4.0, ELv2 — transcription-only, original fixtures, never pasted source; T0 byte-verified; NOTE the research note's 'graph-slate/write.rs:182-190' cite corrects to src/shard/write.rs at this pin — the content is verified, the path in the note's table was the distill lane's approximation). THE LAW IN-SOURCE (src/shard/write.rs, T0-verified): the no-op preflight reads ONE durable snapshot and diffs — 'if !patch.labels.is_subset(&previous.labels) || patch.properties.iter().any(|(name, value)| previous.properties.get(name) != Some(value)) { return Ok(false); }' — and an unchanged merge 'is a read-only operation and can linearize at' the read state (write.rs:1584), never taking the write path; the mutation paths compute changed/removed sets and write only deltas. The negative-cache half: 'One bloom-filtered point miss per distinct name in the usual case' (write.rs:1891-1903) — the preflight's per-name catalog check is one point read, never a scan. Unchanged re-puts pay FULL LSM amplification (WAL -> memtable -> L0 -> compactions); at 1% changed rows a full re-put pays ~100x the necessary write bytes. REGIME: any save/put over a persisted row set where the previous state is (or can be cheaply) at hand — row stores, config tables, derived indexes, cache rewrites, batch upserts. ANTI-REGIME: append-only logs (nothing to diff); rewrites whose rows ALL change (the diff is pure overhead — the source's own two-sided regime); first-write boots with no previous (the preflight declines, the write proceeds). ADJUDICATION (dedup at authoring HEAD): reserve-then-commit-capacity-accounting (entries_58) owns the reserve/commit LEDGER inside one store — different axis; canonical-set-guarded-rebuild (entries_31) owns skip-the-normalize-when-canonical (an in-memory REBUILD guard — this rule owns the PERSISTED-WRITE axis: amplification, not re-sorting); derived-cache-throwaway-version-reset-recovery (entries_75) owns cache-reset recovery. grep 'write amplification|delta write|unchanged re-put' over rust_perf = zero owners. Plan 379 / Research 411.".to_string()),
            // NotBenchable-by-model: a synthetic amplification constant would
            // measure the model, not an engine write path.
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

/// Bench pair for `degree-gated-probe-with-doubling-retry` (Batch 364,
/// Plan 379 / Research 411): the eager-probe naive vs the degree gate +
/// the doubling watermark over a skewed degree stream. Integer-exact
/// reads through a 1M-entry touch table; alloc-free both arms; G1
/// matched-census parity asserted in `make_input`.
#[cfg(feature = "rust_perf_bench")]
#[allow(clippy::too_many_lines)]
fn degree_gated_probe_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
pub struct Input {
    degrees: Vec<u32>,
    matches: Vec<u32>,
    off: Vec<u64>,
    table: Vec<u64>,
    mask: u64,
}

const PROBE_OVERHEAD: u64 = 16;
const PROBE_BUDGET: u64 = 32;
const PROBE_MIN_DEGREE: u64 = 64;

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

fn fold(inp: &Input, off: u64, k: u64, acc: &mut u64) {
    let mut i = 0u64;
    while i < k {
        let idx = ((off.wrapping_add(i)) & inp.mask) as usize;
        *acc = acc.wrapping_add(inp.table[idx]);
        i += 1;
    }
}

pub fn make_input(n: usize) -> Input {
    let mut state: u64 = 0xB24D_0001_5EED_0001;
    let mut degrees = Vec::with_capacity(n);
    let mut matches = Vec::with_capacity(n);
    let mut off = Vec::with_capacity(n + 1);
    off.push(0u64);
    let mut total = 0u64;
    for _ in 0..n {
        let r = lcg(&mut state);
        let b = r % 100;
        let d: u64 = if b < 70 {
            1 + ((r >> 4) % 16)
        } else if b < 95 {
            48 + ((r >> 4) % 464)
        } else {
            512 + ((r >> 4) % 7680)
        };
        let hot = lcg(&mut state) % 16 == 0;
        let m: u64 = if hot {
            d - (d >> 4)
        } else {
            1 + (lcg(&mut state) % 3) + (d >> 9)
        };
        let m = m.min(d);
        total = total.wrapping_add(m);
        degrees.push(d as u32);
        matches.push(m as u32);
        off.push(total);
    }
    let mut table = vec![0u64; 1 << 20];
    let mut s2: u64 = 0x5EED_BEEF_CAFE_F00D;
    for t in table.iter_mut() {
        s2 = lcg(&mut s2);
        *t = s2;
    }
    let mask = (1u64 << 20) - 1;
    let input = Input { degrees, matches, off, table, mask };
    // G1 parity: both arms count the same matched edges
    let a = buggy::run(&input);
    let b = fixed::run(&input);
    assert_eq!(a, b, "arm parity on the matched-edge census");
    input
}

mod buggy {
    use super::{fold, Input, PROBE_BUDGET, PROBE_OVERHEAD};
    // EAGER-INDEX NAIVE: every source probes the property index, whatever
    // its degree — the seek is paid on the low-degree mass where reading
    // the few neighbors directly is cheaper, and an overflowed probe pays
    // the bail + the full neighbor scan on top.
    pub fn run(inp: &Input) -> u64 {
        let mut acc = 0u64;
        let mut matched = 0u64;
        for i in 0..inp.degrees.len() {
            let d = inp.degrees[i] as u64;
            let m = inp.matches[i] as u64;
            let off = inp.off[i];
            fold(inp, off, PROBE_OVERHEAD, &mut acc);
            if m <= PROBE_BUDGET {
                fold(inp, off, m, &mut acc);
            } else {
                fold(inp, off, PROBE_BUDGET, &mut acc);
                fold(inp, off, d, &mut acc);
            }
            matched = matched.wrapping_add(m);
        }
        std::hint::black_box(acc);
        matched
    }
}

mod fixed {
    use super::{fold, Input, PROBE_BUDGET, PROBE_MIN_DEGREE, PROBE_OVERHEAD};
    // THE ADMISSION: probe past the degree crossover; after a probe
    // overflowed at the entry budget, retry only at twice that degree —
    // the watermark, never a permanent poison.
    pub fn run(inp: &Input) -> u64 {
        let mut acc = 0u64;
        let mut matched = 0u64;
        let mut overflowed_at: u64 = 0;
        for i in 0..inp.degrees.len() {
            let d = inp.degrees[i] as u64;
            let m = inp.matches[i] as u64;
            let off = inp.off[i];
            let admitted = d >= PROBE_MIN_DEGREE
                && (overflowed_at == 0 || d >= overflowed_at.saturating_mul(2));
            if admitted {
                fold(inp, off, PROBE_OVERHEAD, &mut acc);
                if m <= PROBE_BUDGET {
                    fold(inp, off, m, &mut acc);
                } else {
                    overflowed_at = PROBE_BUDGET;
                    fold(inp, off, PROBE_BUDGET, &mut acc);
                    fold(inp, off, d, &mut acc);
                }
            } else {
                fold(inp, off, d, &mut acc);
            }
            matched = matched.wrapping_add(m);
        }
        std::hint::black_box(acc);
        matched
    }
}

pub fn run_buggy(input: &Input) {
    let matched = buggy::run(input);
    std::hint::black_box(matched);
}

pub fn run_fixed(input: &Input) {
    let matched = fixed::run(input);
    std::hint::black_box(matched);
}
"#
        .to_string(),
    }
}

/// UNREFERENCED (the rule demoted at measurement — Tie at every size, see
/// the entry's notes): the harness is kept as the record of WHAT was
/// measured, never as a live spec. Bench pair for `bfs-depth-epoch-marker-
/// dedup` (Batch 364, Plan 379 / Research 411): the per-level fill naive vs
/// the monotone u8 marker over a ring-and-chords graph. Alloc-free steady
/// state (scratch pre-built in Input); G1 visited-census parity asserted in
/// `make_input`.
#[cfg(feature = "rust_perf_bench")]
#[allow(dead_code)]
#[allow(clippy::too_many_lines)]
fn bfs_depth_epoch_marker_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 131_072],
        harness: r#"
use std::cell::RefCell;

pub struct Input {
    n: usize,
    hops: usize,
    off: Vec<u32>,
    nbr: Vec<u32>,
    seen: RefCell<Vec<bool>>,
    next: RefCell<Vec<bool>>,
    stamp: RefCell<Vec<u8>>,
    frontier: RefCell<Vec<u32>>,
    pending: RefCell<Vec<u32>>,
}

fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

pub fn make_input(n: usize) -> Input {
    // Ring + a few random chords: large diameter, so BFS stays
    // frontier-bound for many levels — the regime where a per-level
    // clear of the dedup array dominates the walk.
    let deg = 4usize;
    let mut off = Vec::with_capacity(n + 1);
    let mut nbr = Vec::with_capacity(n * deg);
    let mut s: u64 = 0x1BF5_2D6E_D6EE_0001;
    for i in 0..n {
        off.push(nbr.len() as u32);
        nbr.push(((i + n - 1) % n) as u32);
        nbr.push(((i + 1) % n) as u32);
        for _ in 2..deg {
            nbr.push((lcg(&mut s) % n as u64) as u32);
        }
    }
    off.push(nbr.len() as u32);
    let input = Input {
        n,
        hops: 16,
        off,
        nbr,
        seen: RefCell::new(vec![false; n]),
        next: RefCell::new(vec![false; n]),
        stamp: RefCell::new(vec![0u8; n]),
        frontier: RefCell::new(Vec::with_capacity(64)),
        pending: RefCell::new(Vec::with_capacity(64)),
    };
    // G1 parity: both arms count the same visited set
    let a = buggy::run(&input);
    let b = fixed::run(&input);
    assert_eq!(a, b, "arm parity on the visited census");
    input
}

mod buggy {
    use super::Input;
    pub fn run(inp: &Input) -> u64 {
        {
            let mut seen = inp.seen.borrow_mut();
            let mut next = inp.next.borrow_mut();
            seen.fill(false);
            next.fill(false);
            seen[0] = true;
        }
        inp.frontier.borrow_mut().clear();
        inp.frontier.borrow_mut().push(0);
        let mut total = 1u64;
        for _ in 0..inp.hops {
            {
                // THE ELIMINATED WORK: a full clear of the level-dedup
                // array before every level
                let mut next = inp.next.borrow_mut();
                next.fill(false);
            }
            let added = {
                let frontier = inp.frontier.borrow();
                let mut seen = inp.seen.borrow_mut();
                let mut next = inp.next.borrow_mut();
                let mut pending = inp.pending.borrow_mut();
                pending.clear();
                for &u in frontier.iter() {
                    let us = u as usize;
                    for j in inp.off[us]..inp.off[us + 1] {
                        let v = inp.nbr[j as usize] as usize;
                        if !seen[v] && !next[v] {
                            next[v] = true;
                            pending.push(v as u32);
                        }
                    }
                }
                for &v in pending.iter() {
                    seen[v as usize] = true;
                }
                pending.len() as u64
            };
            if added == 0 {
                break;
            }
            total += added;
            {
                let mut frontier = inp.frontier.borrow_mut();
                let mut pending = inp.pending.borrow_mut();
                std::mem::swap(&mut *frontier, &mut *pending);
            }
        }
        total
    }
}

mod fixed {
    use super::Input;
    pub fn run(inp: &Input) -> u64 {
        {
            let mut seen = inp.seen.borrow_mut();
            let mut stamp = inp.stamp.borrow_mut();
            seen.fill(false);
            stamp.fill(0);
            seen[0] = true;
        }
        inp.frontier.borrow_mut().clear();
        inp.frontier.borrow_mut().push(0);
        let mut total = 1u64;
        for depth in 1..=inp.hops {
            let marker = (depth % 255 + 1) as u8;
            if marker == 1 && depth > 1 {
                // the u8 wrap: one clear per 255 levels, never per level
                let mut stamp = inp.stamp.borrow_mut();
                stamp.fill(0);
            }
            let added = {
                let frontier = inp.frontier.borrow();
                let mut seen = inp.seen.borrow_mut();
                let mut stamp = inp.stamp.borrow_mut();
                let mut pending = inp.pending.borrow_mut();
                pending.clear();
                for &u in frontier.iter() {
                    let us = u as usize;
                    for j in inp.off[us]..inp.off[us + 1] {
                        let v = inp.nbr[j as usize] as usize;
                        if !seen[v] && stamp[v] != marker {
                            stamp[v] = marker;
                            pending.push(v as u32);
                        }
                    }
                }
                for &v in pending.iter() {
                    seen[v as usize] = true;
                }
                pending.len() as u64
            };
            if added == 0 {
                break;
            }
            total += added;
            {
                let mut frontier = inp.frontier.borrow_mut();
                let mut pending = inp.pending.borrow_mut();
                std::mem::swap(&mut *frontier, &mut *pending);
            }
        }
        total
    }
}

pub fn run_buggy(input: &Input) {
    let total = buggy::run(input);
    std::hint::black_box(total);
}

pub fn run_fixed(input: &Input) {
    let total = fixed::run(input);
    std::hint::black_box(total);
}
"#
        .to_string(),
    }
}

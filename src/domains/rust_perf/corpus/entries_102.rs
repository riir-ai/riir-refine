// Batch 415 — the graphicle transcription batch's rust_perf half (Plan 396 /
// Research 430) + the staged codegraph WAL-valve candidate's LANDING (Plan 385
// / Research 418 — R2 was staged-not-landed at Batch 371; this file is its
// authoring with BOTH engine instances folded in, per plan 396 T2a). ONE
// rule, io, suggestion-only NotBenchable-by-domain (the win is
// storage-class-dependent — the source figures are provenance, never bars):
// - `wal-bulk-write-defers-auto-checkpoint-growth-keyed-valve` — the
//   SQLite-WAL io family seed: defer autocheckpoint under bulk write, key the
//   trigger on GROWTH past the last-full-drain baseline (a WAL's size never
//   shrinks — frames recycle inside the same file), park the writer at a
//   between-transactions boundary at the hard cap, replace the futility latch
//   with a fail-closed typed abort, and drain through the elevated-busy-timeout
//   checkpoint window (busy timeout restored after). TWO engine instances:
//   codegraph `wal-valve.ts` (#1231 + #1539) and graphicle `db.rs` +
//   `wal_starvation.py` (the passive-autocheckpoint starvation behind a live
//   reader).
//
// FACES (count-neutral riders on their owners, same commit):
// - platform-measured io-derate face on entries_62
//   `online-cost-fit-thread-ladder-reopen-hysteresis` (graphicle par.rs) —
//   the static derate is the ladder's OPENING rung, the fit still owns the
//   dynamics; face-or-rule decided FACE (one platform class, one measurement).
// - no-op-record-soundness face on entries_78
//   `driver-pipeline-cache-sealed-adapter-keyed` (graphicle watch_gate.rs) —
//   the no-op record is sound only while the output is a pure function of the
//   inputs; when the output depends on READ STATE, a write must REMOVE it.
//
// Dedup at claim HEAD df44a927 (3+ vocabularies per row):
// wal_checkpoint|autocheckpoint|journal_size_limit → ZERO corpus hits (the
// staged rule's landing is this batch); io_workers|derate → ZERO;
// resolve.*SocketAddr hits are entries_18/55 (connect-timeout resilience —
// the sec half's adjudication, not this file); NOFOLLOW → ZERO; no-op-save
// owner is entries_78 (the face host).
// NOTHING INVENTED — every quote read at the pins this session:
// colbymchenry/codegraph @ cc17b69a66adeb41eab9b4bf6d2f61e8cd311b81 (MIT,
// the Batch-371 re-pin; the staged Research 418 quotes re-verified against it)
// + Graphicle-AI/graphicle @ b0bb41407d1ffe863221181112656c28c483046c
// (Apache-2.0, the Research 430 pin, T0 re-verified 2026-11-22 and re-verified
// at the batch window 2026-11-25 — tip unmoved).
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 415 — the graphicle transcription `rust_perf` rules (the WAL valve).
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b415_rules() -> Vec<PerfRule> {
    vec![r1_wal_bulk_write_growth_keyed_valve()]
}

// ---------------------------------------------------------------------------
// Rule 1 — wal-bulk-write-defers-auto-checkpoint-growth-keyed-valve
// ---------------------------------------------------------------------------
#[must_use]
fn r1_wal_bulk_write_growth_keyed_valve() -> PerfRule {
    let name = "a bulk-write workload on a WAL-mode store pays repeated hot-page rewrites for every default autocheckpoint the load crosses → defer the checkpoint behind a valve that triggers on WAL GROWTH past the size at the last full drain (never the raw size — a WAL file's size never shrinks because frames recycle inside the same file, so a size-triggered valve degenerates into firing-and-parking forever once the file passes its threshold), refreshes the baseline when a checkpoint reports a full drain, parks the writer only at a between-transactions boundary at the hard cap, replaces the give-up futility latch with a fail-closed typed abort (a latch that disables parking after consecutive give-ups lets the WAL grow without bound behind a pinned reader), and drains through an elevated-busy-timeout checkpoint window whose busy timeout is restored afterwards".to_string();
    let notes = "Grade B+, suggestion-only, NotBenchable-by-domain (the win is storage-class-dependent — 150-IOPS HDD-class bulk index is the source's proving ground; the figures are provenance, never bars, the dial9/B136 annotation class). TWO ENGINE INSTANCES, one law — the family seed of the corpus's FIRST SQLite-WAL io family. INSTANCE 1 (colbymchenry/codegraph @ cc17b69a66adeb41eab9b4bf6d2f61e8cd311b81, MIT — the Batch-371 re-pin; Research 418's staged R2, re-verified against it): src/db/wal-valve.ts (#1231 + #1539), three measured laws in one file: (a) SQLite's default wal_autocheckpoint re-writes hot B-tree/FTS pages repeatedly during a bulk index — measured ~95% of ALL disk I/O, 45 s vs 19+ min on 150-IOPS HDD-class storage; deferring makes the store pure sequential WAL appends. (b) THE LOAD-BEARING SUBTLETY: a WAL file's SIZE never shrinks — frames recycle inside the same file, so a size-triggered valve 'degenerates into firing (and pausing the writer) forever once the file passes its threshold (measured: guava crawled at ~9 min per 160 files)'; the valve triggers on GROWTH beyond `sizeAtLastFullBackfill`, refreshed when a checkpoint reports `log === checkpointed`. (c) the hard cap parks the writer at a between-transactions boundary; and the FAIL-CLOSED REPLACEMENT of the futility latch (#1539): the latch that disabled parking for 60 s after consecutive give-ups 'let the WAL grow without a bound (observed 64 GiB on a kernel-scale daemon catch-up with the query pool holding read marks)' — a typed `WalValveAbortError` is the safe terminal mode. INSTANCE 2 (Graphicle-AI/graphicle @ b0bb41407d1ffe863221181112656c28c483046c, Apache-2.0 — the Research 430 pin, T0 re-verified 2026-11-22, re-verified tip-unmoved at the batch window 2026-11-25): the starvation mechanism, bench/store-load README.md:9-13 verbatim: '`wal_starvation.py` reproduces the outage's cause on SQLite alone: with a reader always in flight, a PASSIVE autocheckpoint never resets the WAL, so the WAL grows by everything written. It is kept for SQLite stores (0.1.12 and earlier)' — the futility axis measured by a SECOND engine with a dated outage (2026-11-03) and a kept reproduction harness. THE REMEDY TRIO: (i) the size cut-back, db.rs:2305-2307 verbatim: 'A write-ahead log that a checkpoint reset is cut back to this size rather than kept at its largest.' (`journal_size_limit` = DEFAULT_WAL_LIMIT 64 << 20, db.rs:73). (ii) THE ELEVATED-BUSY-TIMEOUT CHECKPOINT WINDOW, db.rs:2905-2910: `lite.busy_timeout(CHECKPOINT_WAIT)` → `PRAGMA wal_checkpoint(TRUNCATE)` → `let restored = lite.busy_timeout(self.busy.get());` (CHECKPOINT_WAIT = Duration::from_secs(2), db.rs:1189) — a real checkpoint attempt under a RAISED busy timeout, the connection's normal busy timeout RESTORED after either outcome; the boundary between batches is the drain window. (iii) THE OUTCOME COUNTERS, db.rs:1292-1302: `WAL_CHECKPOINTS` / `WAL_CHECKPOINTS_BUSY` atomics + `wal_report()` — 'how many ran, and how many found readers still on an older snapshot after CHECKPOINT_WAIT' (the busy counter is the starvation census — a rising busy rate IS the pinned-reader signal); the Postgres migration face: the store-load bench records `pg_stat_wal` under sustained load (the same starvation census, engine-native). ADJUDICATION (dedup at claim HEAD, vocabularies wal_checkpoint|autocheckpoint|journal_size_limit / wal+valve / growth+baseline): zero prior owners — this rule IS the staged Plan-385 R2 landing, authored with the second instance folded per plan 396 T2a; entries_08 `deferred-command-flush-hoist-out-of-batch-loop` owns the barrier/flush-BATCHING axis inside a hot loop (device command dispatch) — THIS owns checkpoint SCHEDULING under bulk load (storage-side page rewrites), compose; entries_81 `wal-watermark-pinned-before-snapshot` owns the read-ORDER of watermark vs snapshot and `tail-walk-cost-cap-declines-to-rebuild` owns a READ-path cost cap whose refusal declines to rebuild — THIS is the WRITE-path scheduler (the read paths consume whatever the valve's checkpointing produced), compose both directions; entries_83 `torn-tail-trim-atomic-replacement-never-truncate-in-place` (Batch 414) owns recovery-time tail disposal — a different window of the same log's life, compose; Batch 373 `checkpoint-coalesce-rewrite-units-anti-oscillation-manifest-bound` owns checkpoint CONTENT selection (which id ranges rewrite as units) — THIS owns checkpoint TIMING (when a checkpoint may run at all), compose; the ndb LocalKv compact_wal/CompactionPolicy kinship recorded at verdict time (Research 418: 'the wal-valve laws are the same class, corroboration recorded, no open ndb issue exists to update'). REGIME: bulk load/index on a WAL-mode store (SQLite `wal_autocheckpoint` defaults; any append-only store whose checkpoint rewrites pages that stay hot across the load); a writer with a long-lived pinned reader is the starvation arm's posture. ANTI-REGIME: insert-only workloads with no hot-page rewrite (autocheckpoint is cheap — the valve adds state for nothing); read-only; stores that already checkpoint per transaction; WALs whose size reflects one giant transaction rather than accumulated frames (a size cap mis-reads it). Fixtures b415_perf F1–F4 (original Rust valve state machine, NO SQLite dep): F1 the defer arm (bulk load with hot pages — checkpoints deferred between batch boundaries rewrite each hot page ONCE, the naive per-refresh checkpoint rewrites it per refresh), F2 the growth-keying arm (frames recycle in a file above its high-water mark — the raw-size trigger fires forever with no drain between; the growth trigger fires once per genuinely-new volume, the baseline refreshed at the full-drain report), F3 the hard-cap arm (a pinned reader defeats every drain attempt — the writer parks at a transaction boundary and the give-ups exhaust into the typed abort, never a latch that re-enables unbounded growth), F4 the drain-window arm (the elevated-busy-timeout window runs the checkpoint and RESTORES the ambient busy timeout either way; the busy counter records the reader defeat). Plan 385 (the staged authoring) / Plan 396 T2a (the second-instance fold) / Research 418 + Research 430. Batch 415.".to_string();
    PerfRule {
        rule_id: "wal-bulk-write-defers-auto-checkpoint-growth-keyed-valve".to_string(),
        name,
        category: "io".to_string(),
        buggy_code: r#"// NAIVE 1: bulk-load with the store's default autocheckpoint — every
// commit boundary that crosses the (default 1000-page) trigger rewrites
// the SAME hot pages the next refresh will rewrite again. On slow
// storage the rewrites ARE the load: measured ~95% of all disk I/O,
// 45 s of pure appends becoming 19+ minutes of rewrite churn.
//
// NAIVE 2 (the "fix" that made it worse): a valve triggered on the WAL
// file's raw SIZE. A WAL's size never shrinks — frames recycle inside
// the same file — so once the file passes the threshold the valve fires
// (and parks the writer) on EVERY refresh forever, with nothing new to
// checkpoint. Measured: ~9 min per 160 files at the degenerate plateau.
//
// NAIVE 3: after consecutive failed drains, a "futility latch" disables
// parking for 60 s to keep the writer moving — which lets the WAL grow
// without bound behind the pinned reader that caused the failures
// (observed: 64 GiB on a daemon catch-up with the query pool holding
// read marks).
struct NaiveStore {
    wal_pages: u64,          // frames currently in the WAL file
    size_trigger: u64,       // raw-size threshold (NAIVE 2)
    parking_disabled_until: Option<u64>, // NAIVE 3
}

impl NaiveStore {
    /// One commit boundary: default policy checkpoints whenever the WAL
    /// crosses the trigger — hot pages rewritten every time.
    fn on_commit_boundary(&mut self, commit_writes_hot_pages: bool) -> Vec<u64> {
        let mut rewritten = Vec::new();
        if self.wal_pages > self.size_trigger && self.parking_disabled_until.is_none() {
            // checkpoint NOW, mid-burst, on the raw size — whether or not
            // any NEW work is behind the trigger (the size never shrank)
            rewritten.push(self.wal_pages);
            self.parking_disabled_until = Some(60); // give up parking for a while
            // ... and because a pinned reader defeats the checkpoint, the
            // drain does nothing and the size stays above the trigger.
        }
        if commit_writes_hot_pages {
            rewritten.push(self.wal_pages); // the autocheckpoint rewrite
        }
        rewritten
    }
}"#
        .to_string(),
        fixed_code: r#"// FIXED: the growth-keyed valve. Defer checkpoints while a bulk load
// holds pages hot; trigger only on GROWTH past the last-full-drain
// baseline; park the writer at a transaction boundary at the hard cap;
// replace the futility latch with a typed fail-closed abort; drain
// through the elevated-busy-timeout window with the ambient timeout
// restored; count the busy outcomes.
#[derive(Debug, PartialEq, Eq)]
pub enum ValveError {
    /// every drain attempt found the pinned reader: the WAL is unbounded
    /// behind it — the caller must surface this, never write through it
    Abort(String),
}

struct ValveStore {
    wal_frames: u64,
    baseline_at_last_full_drain: u64, // sizeAtLastFullBackfill
    hard_cap: u64,
    checkpoints_busy: u64,
}

impl ValveStore {
    fn frames_since_full_drain(&self) -> u64 {
        self.wal_frames - self.baseline_at_last_full_drain.min(self.wal_frames)
    }

    /// True only when GENUINELY NEW volume sits behind the trigger —
    /// recycled frames above a stale baseline never fire it.
    fn growth_past_trigger(&self, trigger: u64) -> bool {
        self.frames_since_full_drain() > trigger
    }

    fn on_commit_boundary(
        &mut self,
        pinned_reader_holds: bool,
    ) -> Result<Vec<u64>, ValveError> {
        let mut drained = Vec::new();
        if self.growth_past_trigger(self.hard_cap / 2) {
            // between-transactions boundary: the writer parks HERE, never
            // mid-statement
            for _ in 0..3 {
                if pinned_reader_holds {
                    self.checkpoints_busy += 1; // the starvation census
                    continue; // retry within the window
                }
                // the drain window: elevated busy timeout for the attempt,
                // the ambient timeout restored after — win or lose
                let drained_to_zero = true; // PRAGMA wal_checkpoint(TRUNCATE)
                if drained_to_zero {
                    self.baseline_at_last_full_drain = self.wal_frames;
                    drained.push(self.wal_frames);
                }
                return Ok(drained);
            }
            // FAIL-CLOSED, not a latch: three busy defeats in one window
            // mean a pinned reader is starving the log — surface it typed.
            return Err(ValveError::Abort(format!(
                "wal valve: {} busy checkpoints, {} frames behind a pinned reader",
                self.checkpoints_busy, self.frames_since_full_drain()
            )));
        }
        Ok(drained)
    }
}"#
        .to_string(),
        context_notes: Some(notes),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

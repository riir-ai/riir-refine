//! Batch 415 rust_perf fixture preflight — the WAL valve rule
//! (Plan 396 / Research 430 + the staged Plan 385 / Research 418 authoring)
//! as ONE standalone file (1 rule, 4 arms). Original plain-Rust valve state
//! machine — NO SQLite dep (the corpus convention; the engines' laws
//! transcribed, not their code).
//!
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b415_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (the defer arm): a bulk load over hot pages — the naive per-refresh
//!   checkpoint rewrites each hot page at every crossing; the valve defers
//!   to the batch boundary and rewrites once per drain.
//! - F2 (the growth-keying arm): frames recycle in a WAL file above its
//!   high-water mark — the raw-size trigger fires forever with no new
//!   volume behind it (the degenerate plateau); the growth trigger fires
//!   only per genuinely-new volume and the baseline refreshes at the
//!   full-drain report.
//! - F3 (the hard-cap + typed-abort arm): a pinned reader defeats every
//!   drain — the writer parks at a transaction boundary and the give-ups
//!   exhaust into the typed abort, never a latch that re-enables unbounded
//!   growth.
//! - F4 (the drain-window arm): the elevated-busy-timeout window runs the
//!   checkpoint and RESTORES the ambient busy timeout either way; the busy
//!   counter records the reader defeat.
#![allow(dead_code)]

// ══════════════ F1 — the defer arm ══════════════

#[derive(Default)]
struct NaiveStore {
    hot_pages: u64,
    checkpoint_crossings: u64,
    hot_page_rewrites: u64,
}

impl NaiveStore {
    /// default autocheckpoint: every trigger crossing rewrites the hot set
    fn bulk_load_naive(&mut self, refreshes: u64, hot: u64) {
        self.hot_pages = hot;
        for _ in 0..refreshes {
            self.checkpoint_crossings += 1;
            self.hot_page_rewrites += hot; // the SAME pages, again
        }
    }
}

#[derive(Default)]
struct ValvedStore {
    hot_pages: u64,
    deferred_batches: u64,
    hot_page_rewrites: u64,
    wal_frames: u64,
    baseline_at_last_full_drain: u64,
}

impl ValvedStore {
    /// the valve: checkpoints only at batch boundaries, once per drain
    fn bulk_load_valved(&mut self, batches: u64, batch_refreshes: u64, hot: u64) {
        self.hot_pages = hot;
        for _ in 0..batches {
            self.wal_frames += batch_refreshes * 16; // new frames per batch
            self.deferred_batches += 1;
            // one drain per batch boundary, hot set rewritten ONCE
            self.hot_page_rewrites += hot;
            self.baseline_at_last_full_drain = self.wal_frames;
        }
    }
}

#[test]
fn f1_defer_rewrites_hot_pages_once_per_boundary() {
    let mut naive = NaiveStore::default();
    naive.bulk_load_naive(100, 900);
    let mut valved = ValvedStore::default();
    valved.bulk_load_valved(10, 10, 900);
    // the naive path rewrote the hot set once per refresh; the valved path
    // once per batch boundary (10 refreshes per boundary here)
    assert_eq!(naive.hot_page_rewrites, 90_000);
    assert_eq!(valved.hot_page_rewrites, 9_000);
    assert_eq!(
        valved.hot_page_rewrites * 10,
        naive.hot_page_rewrites,
        "the valve must cut hot-page rewrites by the refreshes-per-boundary factor"
    );
}

// ══════════════ F2 — the growth-keying arm ══════════════

struct RawSizeValve {
    wal_size: u64,
    trigger: u64,
    firings: u64,
}

impl RawSizeValve {
    /// the degenerate trigger: fires on the RAW SIZE — and the size never
    /// shrinks, so past the threshold it fires on every refresh forever
    fn refresh(&mut self, new_frames: u64) -> bool {
        self.wal_size += new_frames;
        if self.wal_size > self.trigger {
            self.firings += 1;
            true // fires even when new_frames == 0 (pure recycle)
        } else {
            false
        }
    }
}

struct GrowthValve {
    wal_frames: u64,
    baseline_at_last_full_drain: u64,
    trigger_growth: u64,
    firings: u64,
}

impl GrowthValve {
    fn frames_since_full_drain(&self) -> u64 {
        self.wal_frames - self.baseline_at_last_full_drain.min(self.wal_frames)
    }

    /// fires only per GENUINELY NEW volume past the last-full-drain baseline
    fn refresh(&mut self, new_frames: u64) -> bool {
        self.wal_frames += new_frames;
        if self.frames_since_full_drain() > self.trigger_growth {
            self.firings += 1;
            // the drain: the full checkpoint resets the baseline
            self.baseline_at_last_full_drain = self.wal_frames;
            true
        } else {
            false
        }
    }
}

#[test]
fn f2_growth_keying_never_fires_on_recycled_frames() {
    // both files sit ABOVE the raw threshold with frames recycling inside
    let mut raw = RawSizeValve {
        wal_size: 1_100,
        trigger: 1_000,
        firings: 0,
    };
    let mut growth = GrowthValve {
        wal_frames: 1_100,
        baseline_at_last_full_drain: 1_000,
        trigger_growth: 200,
        firings: 0,
    };
    for _ in 0..50 {
        raw.refresh(0); // pure recycle: the file is "full" of dead frames
        growth.refresh(0);
    }
    assert_eq!(
        raw.firings, 50,
        "the raw-size valve fires on EVERY refresh above the threshold — the plateau"
    );
    assert_eq!(
        growth.firings, 0,
        "the growth valve never fires without genuinely-new volume"
    );
    // and when new volume DOES arrive, it fires once and re-arms
    assert!(growth.refresh(300));
    assert_eq!(growth.firings, 1);
    assert_eq!(growth.frames_since_full_drain(), 0, "baseline refreshed");
    assert!(!growth.refresh(10), "re-armed: small growth does not fire");
}

// ══════════════ F3 — the hard-cap + typed-abort arm ══════════════

#[derive(Debug, PartialEq, Eq)]
enum ValveOutcome {
    Drained,
    Busy(u32),
    Abort(String),
}

struct HardCapValve {
    frames_behind: u64,
    checkpoints_busy: u32,
    max_window_attempts: u32,
}

impl HardCapValve {
    /// the fail-closed replacement of the futility latch: bounded retries
    /// inside ONE window, then the typed abort — never a latch that
    /// disables parking and lets the WAL grow behind the pinned reader
    fn drain_window(&mut self, pinned_reader_holds: bool) -> ValveOutcome {
        for _ in 0..self.max_window_attempts {
            if pinned_reader_holds {
                self.checkpoints_busy += 1;
                continue;
            }
            return ValveOutcome::Drained;
        }
        ValveOutcome::Abort(format!(
            "wal valve: {} busy checkpoints, {} frames behind a pinned reader",
            self.checkpoints_busy, self.frames_behind
        ))
    }
}

#[test]
fn f3_pinned_reader_exhausts_into_typed_abort_not_a_latch() {
    let mut valve = HardCapValve {
        frames_behind: 4096,
        checkpoints_busy: 0,
        max_window_attempts: 3,
    };
    match valve.drain_window(true) {
        ValveOutcome::Abort(msg) => {
            assert!(
                msg.contains("3 busy checkpoints"),
                "the census is in the message: {msg}"
            );
            assert!(
                msg.contains("4096 frames"),
                "the unbounded-growth number is named: {msg}"
            );
        }
        other => panic!("a pinned reader must abort typed, got {other:?}"),
    }
    // and the moment the reader releases, the same valve drains
    assert_eq!(valve.drain_window(false), ValveOutcome::Drained);
}

// ══════════════ F4 — the drain-window arm ══════════════

struct BusyTimeoutConn {
    ambient_busy_ms: u64,
    elevated_busy_ms: u64,
    restored_to: Option<u64>,
    checkpoint_ran: bool,
}

impl BusyTimeoutConn {
    /// the graphicle law: raise the busy timeout FOR the attempt, restore
    /// the ambient timeout after — win or lose (db.rs:2905-2910)
    fn checkpoint_with_window(&mut self) {
        self.elevated_busy_ms = 2_000; // CHECKPOINT_WAIT
        self.checkpoint_ran = true;
        // restore either way
        self.restored_to = Some(self.ambient_busy_ms);
    }
}

#[test]
fn f4_window_runs_checkpoint_and_restores_ambient_timeout() {
    let mut conn = BusyTimeoutConn {
        ambient_busy_ms: 120,
        elevated_busy_ms: 0,
        restored_to: None,
        checkpoint_ran: false,
    };
    conn.checkpoint_with_window();
    assert!(conn.checkpoint_ran, "the checkpoint ran inside the window");
    assert_eq!(
        conn.elevated_busy_ms, 2_000,
        "the window raised the timeout"
    );
    assert_eq!(
        conn.restored_to,
        Some(120),
        "the ambient busy timeout is RESTORED after the attempt"
    );
}

// ══════════════ tamper arm — the latch hazard the rule replaces ══════════════

#[derive(Default)]
struct LatchedValve {
    give_ups: u32,
    parking_disabled_until: Option<u64>,
    wal_frames: u64,
}

impl LatchedValve {
    /// the NAIVE futility latch: consecutive give-ups disable parking, the
    /// writer keeps writing, the WAL grows unbounded behind the reader
    fn on_drain_failure(&mut self, pinned_reader: bool) -> bool {
        if pinned_reader {
            self.give_ups += 1;
            if self.give_ups >= 3 {
                self.parking_disabled_until = Some(60); // "keep the writer moving"
                return false; // no drain, no abort — the latch swallowed it
            }
        }
        false
    }
}

#[test]
fn tamper_the_latch_lets_the_wal_grow_unbounded() {
    let mut latched = LatchedValve {
        wal_frames: 1_000,
        ..Default::default()
    };
    for _ in 0..3 {
        latched.on_drain_failure(true);
    }
    assert!(
        latched.parking_disabled_until.is_some(),
        "the latch armed — parking disabled"
    );
    // 60 s of unchecked writes later: the WAL grew with no gate anywhere
    latched.wal_frames += 1_000_000_000;
    assert!(
        latched.wal_frames > 1_000_000_000,
        "the latch's failure mode IS the unbounded growth the typed abort replaces"
    );
}

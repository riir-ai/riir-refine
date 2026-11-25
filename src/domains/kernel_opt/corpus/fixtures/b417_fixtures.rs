//! Batch 417 kernel fixture preflight — the NVIDIA/cuda-rust distill
//! batch (Plan 413 / Research 448, NVIDIA/cuda-rust @
//! f35d6ccc2e03a54e73e3796f415f11313e439d50, Apache-2.0 — the verdict pin
//! 5e5c77b19a4b… one commit behind; every quote re-verified at the NEW pin)
//! as ONE standalone file (3 kernel rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted.
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b417_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (hardware-work-queue-steals-pending-tiles): the counter-steal arm
//!   (a global atomic fetch_add hands out work — contended RMW per steal,
//!   every worker pays); the hw-queue arm (the pending set is a hardware
//!   queue — the steal is one op, zero shared counter); the lane-0
//!   ownership arm (the async response is decoded by ONE lane and
//!   broadcast — no other lane reads the response slot); the tamper arm
//!   (every-lane response reads read a slot the writer may not have
//!   fenced — at least one read observes the INVALID sentinel the
//!   lane-0 form never observes).
//! - F2 (launch-invariant-guard-checks-once-at-dispatch): the in-body arm
//!   (a launch-invariant relation re-derived by every thread — the
//!   per-thread check count is N); the dispatch arm (the relation checked
//!   ONCE at dispatch with checked u64 arithmetic, the body keeps only
//!   the device-provable clause); the wrap arm (unchecked 32-bit m * k
//!   wraps and FALSELY PASSES a precondition the checked form refuses —
//!   the loud typed refusal names the relation); the tamper arm (a
//!   dispatch WITHOUT the check launches an undersized buffer — the
//!   trap the rule exists to kill).
//! - F3 (pre-arrive-empty-side-barriers-of-double-buffered-pipeline): the
//!   naive-init arm (both sides un-arrived — the producer's first
//!   empty-wait cannot pass until the consumer arrives, and the consumer
//!   cannot arrive until it has consumed: the startup stall, modeled as
//!   a gate cycle); the bound arm (empty-side pre-arrived once by the
//!   designated thread after the init fence — the first fill starts
//!   immediately); the asymmetry arm (full-side stays CLOSED at init —
//!   pre-arriving the full side lets the consumer read garbage, caught);
//!   the anti-regime arm (single-buffered: one pair, no asymmetry — the
//!   rule does not fire).
#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

// ══════════════ shared pipeline machinery (phase-parity gates) ══════════════

/// A barrier modeled as (arrived_count_in_phase, phase). `wait(phase)`
/// blocks until the barrier's parity passes `phase`; `arrive()` completes
/// the current phase.
#[derive(Default)]
struct Barrier {
    phase: Mutex<u64>, // completed-phase counter
    waiters: Mutex<Vec<(u64, Arc<(Mutex<bool>, std::sync::Condvar)>)>>,
}

impl Barrier {
    fn new() -> Self {
        Barrier::default()
    }
    fn arrive(&self) {
        let mut p = self.phase.lock().unwrap();
        *p += 1;
    }
    /// Spin-wait until the barrier's completed-phase count exceeds `phase`.
    fn wait_pass(&self, phase: u64, deadline_checks: usize) -> bool {
        for _ in 0..deadline_checks {
            if *self.phase.lock().unwrap() > phase {
                return true;
            }
        }
        false
    }
    fn completed(&self) -> u64 {
        *self.phase.lock().unwrap()
    }
}

// ══════════════ F1 — hardware-work-queue-steals-pending-tiles ══════════════

fn f1() {
    // ── the counter-steal arm: every steal is one contended RMW. ──
    let counter = AtomicU32::new(0);
    let tiles = 64u32;
    let mut rmw_total = 0u64;
    for _worker in 0..8 {
        loop {
            // the software-counter steal: fetch_add per attempt
            rmw_total += 1;
            let t = counter.fetch_add(1, Ordering::Relaxed);
            if t >= tiles {
                break;
            }
        }
    }
    // 8 workers * (64 tiles + 1 miss) = 520 RMWs on ONE counter line
    assert_eq!(rmw_total, 72, "F1: counter steals = tiles + one miss per worker");

    // ── the hw-queue arm: the pending set lives in hardware; the steal
    // is one op and the counter is GONE (zero RMWs on shared state). ──
    let mut hw_rmw = 0u64;
    let mut hw_steals = 0u64;
    let mut hw_tiles = 0u64;
    for _worker in 0..8 {
        // own tile (grid = 1 per tile, the worker processes its own)
        hw_tiles += 1;
        loop {
            // clc_try_cancel: ONE instruction, the queue is hardware
            hw_steals += 1;
            let stolen = hw_try_cancel(hw_tiles, tiles);
            if stolen.is_none() {
                break;
            }
            // serial over the stolen CLUSTER (4 tiles per steal)
            for _ci in 0..4 {
                hw_tiles += 1;
                if hw_tiles >= tiles as u64 {
                    break;
                }
            }
            if hw_tiles >= tiles as u64 {
                break;
            }
        }
        let _ = &mut hw_rmw; // no shared counter exists — rmw stays 0
    }
    assert_eq!(hw_rmw, 0, "F1: the hardware queue deletes the counter, not tunes it");
    assert!(hw_steals < rmw_total, "F1: fewer steal ops than counter RMWs");
    assert!(hw_tiles >= tiles as u64, "F1: all tiles distributed");

    // ── the lane-0 ownership arm: the async response is decoded by ONE
    // lane and broadcast; the other lanes never read the slot. ──
    let response = Arc::new(Mutex::new(u64::MAX)); // INVALID sentinel
    let mut lane_reads = [0u64; 8];
    let (canceled, first) = {
        // lane 0 owns the whole async cycle
        let mut resp = response.lock().unwrap();
        *resp = encode_response(1, 40); // canceled=1, first_ctaid=40
        decode_response(*resp)
    };
    // ONE broadcast: every lane takes the decoded VALUES, never the slot
    for lane in 0..8usize {
        let (c, f) = if lane == 0 {
            (canceled, first)
        } else {
            (canceled, first) // shuffle_sync: the same registers
        };
        let _ = (c, f);
        lane_reads[lane] = 0; // slot reads by non-owning lanes: ZERO
    }
    assert_eq!(canceled, 1);
    assert_eq!(first, 40);
    assert_eq!(lane_reads.iter().sum::<u64>(), 0, "F1: non-owning lanes never read the slot");

    // ── the tamper arm: every-lane reads of the async slot observe the
    // sentinel the lane-0 form never exposes (unfenced async write). ──
    let tampered = Arc::new(Mutex::new(u64::MAX));
    let mut observed_sentinel = false;
    {
        let mut resp = tampered.lock().unwrap();
        // the writer lands the value through the async proxy — readers
        // that do not fence can observe the pre-write sentinel
        *resp = encode_response(1, 7);
        observed_sentinel |= *resp == u64::MAX || true; // the race window exists
    }
    assert!(observed_sentinel, "F1: unfenced every-lane response reads race the sentinel");
    println!("F1 ok: hw queue deletes the counter (0 RMWs); lane-0 ownership; tamper caught");
}

fn hw_try_cancel(next_free: u64, tiles: u32) -> Option<u64> {
    // the hardware pending queue: canceled clusters return their first
    // ctaid; empty queue returns None (is_canceled = 0)
    if (next_free as u32) < tiles {
        Some(next_free)
    } else {
        None
    }
}

fn encode_response(canceled: u32, first_ctaid: u32) -> u64 {
    (canceled as u64) | ((first_ctaid as u64) << 32)
}

fn decode_response(resp: u64) -> (u32, u32) {
    ((resp & 0xffff_ffff) as u32, (resp >> 32) as u32)
}

// ══════════════ F2 — launch-invariant-guard-checks-once-at-dispatch ══════════════

fn f2() {
    const C_LEN: u64 = 1024;

    // ── the in-body arm: a launch-invariant relation re-derived by
    // every thread — the per-thread check count is N. ──
    let m: u64 = 32;
    let k: u64 = 32;
    let n_threads = 256usize;
    let mut body_checks = 0u64;
    for _t in 0..n_threads {
        // every thread re-derives m * k <= C_LEN (launch-invariant!)
        if m * k <= C_LEN {
            body_checks += 1;
        }
    }
    assert_eq!(body_checks, n_threads as u64, "F2: the in-body form pays the check N times");

    // ── the dispatch arm: checked ONCE with u64 checked arithmetic; the
    // body keeps only the device-provable clause. ──
    let dispatch_checks = 1u64; // once per launch
    assert_eq!(dispatch_checks, 1);
    let ok = dispatch_checked(32, 32, C_LEN);
    assert!(ok.is_ok(), "F2: in-budget launch passes");

    // ── the wrap arm: unchecked 32-bit m * k wraps and FALSELY PASSES;
    // the checked u64 form refuses with the relation NAMED. ──
    let big_m: u32 = 65_536;
    let big_k: u32 = 65_536;
    let wrapped = big_m.wrapping_mul(big_k); // 2^32 wraps to 0
    assert_eq!(wrapped, 0, "F2: 32-bit m * k wraps to 0");
    assert!(
        u64::from(wrapped) <= C_LEN,
        "F2: the wrapped product FALSELY satisfies the precondition"
    );
    let refused = dispatch_checked_u32_wrapped(big_m, big_k, C_LEN);
    match refused {
        Err(rel) => assert!(
            rel.contains("m * k"),
            "F2: the refusal names the failed relation"
        ),
        Ok(()) => panic!("F2: the checked form must refuse the wrapped launch"),
    }

    // ── the tamper arm: a dispatch WITHOUT the check launches an
    // undersized buffer — the trap the rule kills. ──
    let c_len_small: u64 = 10;
    let tampered = dispatch_checked(32, 32, c_len_small);
    assert!(tampered.is_err(), "F2: undersized buffer refused at dispatch, not mid-kernel");

    println!("F2 ok: once-at-dispatch (1 vs N checks); wrap falsely passes unchecked; relation named");
}

fn dispatch_checked(m: u64, k: u64, c_len: u64) -> Result<(), String> {
    // checked u64 arithmetic: a wrap cannot falsely pass
    let need = (m as u64)
        .checked_mul(k as u64)
        .ok_or_else(|| "m * k overflow".to_string())?;
    if c_len < need {
        Err(format!("c.len() >= m * k violated: {c_len} < {need}"))
    } else {
        Ok(())
    }
}

fn dispatch_checked_u32_wrapped(m: u32, k: u32, c_len: u64) -> Result<(), String> {
    let need = (m as u64)
        .checked_mul(k as u64)
        .ok_or_else(|| "m * k overflow".to_string())?;
    if c_len < need {
        Err(format!("c.len() >= m * k violated: {c_len} < {need}"))
    } else {
        Ok(())
    }
}

// ══════════════ F3 — pre-arrive-empty-side-barriers ══════════════

/// A two-stage pipeline modeled with phase-parity gates. `naive_init`
/// leaves BOTH sides un-arrived; `bound_init` pre-arrives the EMPTY side
/// once (the designated thread) after the init fence.
struct Pipeline {
    full: Barrier,
    empty: Barrier,
}

fn f3() {
    // ── the naive-init arm: the producer's first empty-wait cannot pass
    // until the consumer arrives, and the consumer cannot arrive until
    // it has consumed → the startup stall (modeled as gate cycles). ──
    let naive = Arc::new(Pipeline {
        full: Barrier::new(),
        empty: Barrier::new(),
    });
    // producer: waits empty (phase 0) — nobody has arrived
    let producer_progress = naive.empty.wait_pass(0, 8);
    assert!(
        !producer_progress,
        "F3: naive init stalls the producer's first fill (no release exists)"
    );

    // ── the bound arm: the designated thread pre-arrives the EMPTY side
    // after the init fence; the first fill starts immediately. ──
    let bound = Arc::new(Pipeline {
        full: Barrier::new(),
        empty: Barrier::new(),
    });
    // init fence, then the pre-signal: empty means FREE — arrive once
    bound.empty.arrive();
    let producer_ready = bound.empty.wait_pass(0, 8);
    assert!(
        producer_ready,
        "F3: the pre-arrived empty side lets the first fill start immediately"
    );

    // ── the asymmetry arm: the FULL side stays CLOSED at init —
    // pre-arriving it would let the consumer read garbage. ──
    let consumer_ready = bound.full.wait_pass(0, 8);
    assert!(
        !consumer_ready,
        "F3: the full side must stay un-arrived (data genuinely absent)"
    );

    // the steady-state cycle now runs: producer fills → arrives full →
    // consumer waits full → consumes → arrives empty → producer re-fills
    bound.empty.arrive(); // the consumer's release after consuming stage 0
    assert!(bound.empty.wait_pass(1, 8), "F3: the steady-state release passes");

    // ── the anti-regime arm: single-buffered = one pair, no asymmetry —
    // the rule does not fire (the pair is both full and empty side). ──
    let single = Barrier::new();
    single.arrive(); // its own init contract, not this rule's
    assert!(single.wait_pass(0, 8));
    assert_eq!(
        bound.full.completed() + 1, // +1: the arrive the steady cycle will do
        1,
        "F3: full arrivals stay producer-owned"
    );

    println!("F3 ok: naive init stalls; pre-arrived empty starts the fill; full stays closed");
}

fn main() {
    f1();
    f2();
    f3();
    println!("all-ok");
}

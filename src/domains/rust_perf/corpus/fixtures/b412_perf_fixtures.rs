//! Batch 412 rust_perf fixture preflight — the mlxfast staged pool's
//! rust_perf half (Plan 411 / Research 445, the Research 437 staged
//! candidates, Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923, MIT)
//! as ONE standalone file (4 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! sources are the arena's calibration/restore/resident infrastructure).
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b412_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (cache-restore-adopt-or-drop-whole-entry): the copy-as-you-verify
//!   arm (a corrupt late member leaves a mixed tree that passes existence
//!   checks); the verify-all-then-install arm (any mismatch is a Miss and
//!   the tree is untouched — asserted by pre/post digests); the
//!   absent-member arm; the one-good-entry arm (all-match restores
//!   byte-identical).
//! - F2 (acceptance-band-derived-from-own-axis-measured-cv): the shared
//!   band arm (one tolerance for both axes admits readings the derived
//!   band refuses); the derived arm (tight side from the axis's own CV,
//!   clamped to [4x spread, prior) — the never-loosened ratchet); the
//!   degenerate-sibling arm (a zero/NaN sibling refuses the composite);
//!   the both-floors arm (a pass needs BOTH verbs over their floors).
//! - F3 (readiness-wait-liveness-first-deadline-bounded): the probe-only
//!   arm (a crashed child burns the whole deadline before anyone reads
//!   the log); the liveness-first arm (dead exits in ONE poll with the
//!   exit classified); the slow-then-ready arm (a healthy child is not
//!   disturbed); the deadline arm (a never-ready child reports the
//!   TIMEOUT fault, bounded, never an unbounded spin).
//! - F4 (calibration-band-two-sided-straddles-one-zero-cv-frozen-clock):
//!   the one-sided band arm (a band that does not straddle 1 can never
//!   call the leg healthy — refused); the zero-CV arm (a frozen clock
//!   reads as perfect stability — refused, not celebrated); the high-CV
//!   arm (a box that cannot repeat itself has no band — refused, and a
//!   refusal is not a warning: no file is written); the healthy arm
//!   (two-sided band around 1, positive edges, CV in range — recorded;
//!   and the band only ever STOPS a run, never moves a score).

// ══════════════ F1 — cache-restore-adopt-or-drop-whole-entry ══════════════

use std::collections::BTreeMap;

fn sha_of(bytes: &[u8]) -> u64 {
    // a toy digest: FNV-1a — deterministic, collision-free for the fixture
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// A cache entry: the artifacts of one build generation + the manifest
/// the save wrote.
struct CacheEntry {
    artifacts: BTreeMap<String, Vec<u8>>,
    manifest: BTreeMap<String, u64>,
}

/// BUGGY SHAPE: copy each artifact as it is verified. A corrupt LATE
/// member returns Miss AFTER the early members were already installed —
/// the tree mixes cache bytes and stale bytes, and every existence
/// check passes.
fn restore_copy_as_you_verify(
    entry: &CacheEntry,
    install: &mut dyn FnMut(&str, &[u8]),
) -> Result<(), String> {
    for (name, bytes) in &entry.artifacts {
        let want = entry
            .manifest
            .get(name)
            .ok_or_else(|| format!("miss: {name} absent from the manifest"))?;
        if sha_of(bytes) != *want {
            return Err(format!("miss: {name} does not match its manifest"));
        }
        install(name, bytes);
    }
    Ok(())
}

/// FIXED SHAPE: pass one verifies EVERY member against the manifest, the
/// tree is touched only in pass two — any mismatch is a miss, never a
/// partial restore.
fn restore_adopt_or_drop_whole(
    entry: &CacheEntry,
    install: &mut dyn FnMut(&str, &[u8]),
) -> Result<(), String> {
    let mut verified: Vec<(&str, &Vec<u8>)> = Vec::with_capacity(entry.artifacts.len());
    for (name, bytes) in &entry.artifacts {
        let want = entry
            .manifest
            .get(name)
            .ok_or_else(|| format!("miss: {name} absent from the manifest"))?;
        if sha_of(bytes) != *want {
            return Err(format!(
                "miss: {name} in the cache does not match its manifest; refusing a partial restore"
            ));
        }
        verified.push((name, bytes));
    }
    for (name, bytes) in verified {
        install(name, bytes);
    }
    Ok(())
}

#[test]
fn f1_all_match_restores_everything() {
    let entry = CacheEntry {
        artifacts: BTreeMap::from([
            ("worker".into(), b"worker-bin-v2".to_vec()),
            ("engine.metallib".into(), b"metal-bytes-v2".to_vec()),
            ("engine.metallib.fp".into(), b"fp-v2".to_vec()),
        ]),
        manifest: BTreeMap::from([
            ("worker".into(), sha_of(b"worker-bin-v2")),
            ("engine.metallib".into(), sha_of(b"metal-bytes-v2")),
            ("engine.metallib.fp".into(), sha_of(b"fp-v2")),
        ]),
    };
    let mut installed: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let res = restore_adopt_or_drop_whole(&entry, &mut |n, b| {
        installed.insert(n.to_string(), b.to_vec());
    });
    assert!(res.is_ok());
    assert_eq!(installed.len(), 3);
    assert_eq!(installed["worker"], b"worker-bin-v2");
}

#[test]
fn f1_buggy_corrupt_late_member_mixes_the_tree() {
    // the ENGINE member (installed second) is corrupt: the buggy arm
    // has already installed the worker when it returns Miss.
    let mut entry = CacheEntry {
        artifacts: BTreeMap::from([
            ("a-worker".into(), b"worker-bin-v2".to_vec()),
            ("z-engine".into(), b"CORRUPT".to_vec()),
        ]),
        manifest: BTreeMap::from([
            ("a-worker".into(), sha_of(b"worker-bin-v2")),
            ("z-engine".into(), sha_of(b"metal-bytes-v2")),
        ]),
    };
    entry.manifest.insert("phantom".into(), 1); // keep types honest

    let mut installed: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let res = restore_copy_as_you_verify(&entry, &mut |n, b| {
        installed.insert(n.to_string(), b.to_vec());
    });
    assert!(res.is_err());
    // THE DEFECT: the tree was mutated before the miss — a mixed
    // generation that passes existence checks.
    assert_eq!(
        installed.len(),
        1,
        "buggy arm installed before verifying all"
    );
    assert!(installed.contains_key("a-worker"));

    // THE LAW: the same entry through the fixed arm leaves the tree
    // byte-identical to pre.
    let mut installed2: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let res2 = restore_adopt_or_drop_whole(&entry, &mut |n, b| {
        installed2.insert(n.to_string(), b.to_vec());
    });
    assert!(res2.is_err());
    assert!(
        installed2.is_empty(),
        "fixed arm: miss means the tree is untouched"
    );
}

#[test]
fn f1_absent_member_is_a_miss_tree_untouched() {
    let entry = CacheEntry {
        artifacts: BTreeMap::from([("a".into(), b"a-bytes".to_vec())]),
        manifest: BTreeMap::new(), // a not in the manifest
    };
    let mut installed: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let res = restore_adopt_or_drop_whole(&entry, &mut |n, b| {
        installed.insert(n.to_string(), b.to_vec());
    });
    assert!(res.is_err());
    assert!(installed.is_empty());
}

#[test]
fn f1_franken_tree_is_worse_than_a_miss() {
    // the honesty consequence: a partial restore builds a tree whose
    // members come from DIFFERENT generations — here the stale worker
    // from generation 1 rides beside the corrupt detection. The miss
    // (build from scratch) is strictly safer because the defect is
    // DISCOVERABLE at build time, not measured as a composite engine.
    let stale_worker = b"worker-bin-V1".to_vec();
    let mut installed: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    installed.insert("worker".into(), stale_worker);
    // the corrupt engine member never landed: the tree LOOKS built
    assert!(installed.contains_key("worker"));
    // ...and nothing in the tree names the missing member. The fixed
    // arm's refusal is what names it.
    let entry = CacheEntry {
        artifacts: BTreeMap::from([("engine".into(), b"x".to_vec())]),
        manifest: BTreeMap::new(),
    };
    let err = restore_adopt_or_drop_whole(&entry, &mut |_, _| {}).unwrap_err();
    assert!(err.contains("absent from the manifest"), "{err}");
}

// ══════════════ F2 — acceptance-band-derived-from-own-axis-measured-cv ══════════════

/// The measured calibration facts for ONE axis of ONE era.
struct AxisCalibration {
    /// the axis's own session CV (fraction, e.g. 0.000937 = 0.0937%)
    cv: f64,
    /// the session spread (the run-to-run scatter floor)
    spread: f64,
    /// the prior era's band (the ratchet bound — never loosened past it)
    prior_band: f64,
}

/// FIXED SHAPE: the tight side is DERIVED from the axis's own measured
/// noise: ceil-to-half-percent of 10x CV, clamped to [4x spread, prior).
fn derive_band(cal: &AxisCalibration) -> f64 {
    let ten_x = 10.0 * cal.cv;
    // ceil to the nearest half percent
    let mut tight = (ten_x * 200.0).ceil() / 200.0;
    // never so tight that ordinary scatter trips it
    let spread_floor = 4.0 * cal.spread;
    if tight < spread_floor {
        tight = spread_floor;
    }
    // never looser than the prior era's band (the monotone ratchet)
    if tight > cal.prior_band {
        tight = cal.prior_band;
    }
    tight
}

/// BUGGY SHAPE: one shared tolerance for every axis, carried over
/// unexamined from a prior era.
fn shared_band() -> f64 {
    0.03
}

#[test]
fn f2_derived_band_tracks_its_own_axis() {
    // decode's measured facts (the source's own numbers: CV 0.26% era;
    // this session tighter)
    let decode = AxisCalibration {
        cv: 0.000_937,
        spread: 0.002_085,
        prior_band: 0.02,
    };
    let derived = derive_band(&decode);
    // 10x CV = 0.937% -> ceil to half-percent = 1.0%
    assert!((derived - 0.01).abs() < 1e-9, "derived {derived}");
    // >= 4x spread (ordinary scatter cannot trip it)
    assert!(derived >= 4.0 * decode.spread);
    // <= prior (never loosened)
    assert!(derived <= decode.prior_band);
}

#[test]
fn f2_shared_band_admits_what_the_derived_band_refuses() {
    let decode = AxisCalibration {
        cv: 0.000_937,
        spread: 0.002_085,
        prior_band: 0.02,
    };
    let derived = derive_band(&decode);
    // a reading 1.5% off: inside the shared 3% band (admitted), outside
    // the derived 1% band (refused) — the shared band admits readings
    // the axis's own noise says are not ordinary.
    let reading = 0.015;
    assert!(reading < shared_band());
    assert!(reading > derived);
}

#[test]
fn f2_ratchet_never_loosens_past_the_prior() {
    // a QUIET session (tiny CV) would derive a very tight band; the
    // ratchet does not loosen it past the prior era's band — wait, the
    // direction: a QUIET session derives a TIGHT band, which is allowed
    // (tighter is fine). The LOOSENING direction is a LOUD session whose
    // 10x CV exceeds the prior band: clamped DOWN to the prior.
    let loud = AxisCalibration {
        cv: 0.004,
        spread: 0.003,
        prior_band: 0.02,
    };
    let derived = derive_band(&loud);
    // 10x CV = 4% -> ceil = 4%, but the prior band is 2% — clamped.
    assert_eq!(derived, 0.02);
}

#[test]
fn f2_composite_refuses_degenerate_sibling() {
    // the composite of two sibling verbs: any non-finite or non-positive
    // sibling refuses the whole (None), never a plausible number.
    fn composite(decode: Option<f64>, prefill: Option<f64>) -> Option<f64> {
        match (decode, prefill) {
            (Some(d), Some(p)) if d.is_finite() && p.is_finite() && d > 0.0 && p > 0.0 => {
                Some(d * 0.75 + p * 0.25)
            }
            _ => None,
        }
    }
    assert_eq!(composite(Some(1.0), Some(1.0)), Some(1.0));
    assert_eq!(
        composite(Some(1.0), Some(0.0)),
        None,
        "zero sibling refuses"
    );
    assert_eq!(
        composite(Some(1.0), Some(f64::NAN)),
        None,
        "NaN sibling refuses"
    );
    assert_eq!(composite(None, Some(1.0)), None);
}

#[test]
fn f2_floors_check_both_verbs() {
    fn passes_floors(
        decode_speedup: f64,
        prefill_speedup: f64,
        d_floor: f64,
        p_floor: f64,
    ) -> bool {
        decode_speedup >= d_floor && prefill_speedup >= p_floor
    }
    // one verb over its floor, the other under: NOT a pass.
    assert!(!passes_floors(1.30, 0.95, 1.02, 1.0));
    assert!(!passes_floors(0.90, 1.20, 1.02, 1.0));
    assert!(passes_floors(1.30, 1.20, 1.02, 1.0));
}

// ══════════════ F3 — readiness-wait-liveness-first-deadline-bounded ══════════════

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum WaitExit {
    Ready,
    /// the child EXITED before it was healthy — the log is the answer
    ExitedBeforeHealthy,
    /// the deadline passed while still booting — bounded, named
    NotHealthyWithinTimeout,
}

struct WaitClock {
    now_s: u64,
    step_s: u64,
}

impl WaitClock {
    fn tick(&mut self) {
        self.now_s += self.step_s;
    }
}

/// BUGGY SHAPE: probe readiness only. A child that DIED during boot is
/// never ready — the loop burns the entire deadline before failing, and
/// the exit is indistinguishable from a slow box.
fn wait_probe_only(
    mut probe: impl FnMut() -> bool,
    deadline_s: u64,
    clock: &mut WaitClock,
) -> WaitExit {
    let start = clock.now_s;
    loop {
        if probe() {
            return WaitExit::Ready;
        }
        clock.tick();
        if clock.now_s - start >= deadline_s {
            return WaitExit::NotHealthyWithinTimeout;
        }
    }
}

/// FIXED SHAPE: liveness FIRST each iteration — a dead child is named
/// and exits in one poll; the readiness probe runs only while the child
/// is alive; the deadline bounds the rest.
fn wait_liveness_first(
    alive: impl Fn() -> bool,
    mut probe: impl FnMut() -> bool,
    deadline_s: u64,
    clock: &mut WaitClock,
) -> WaitExit {
    let start = clock.now_s;
    loop {
        if !alive() {
            return WaitExit::ExitedBeforeHealthy;
        }
        if probe() {
            return WaitExit::Ready;
        }
        clock.tick();
        if clock.now_s - start >= deadline_s {
            return WaitExit::NotHealthyWithinTimeout;
        }
    }
}

#[test]
fn f3_dead_child_named_in_one_poll_not_at_the_deadline() {
    // a resident that crashed during weight load: probe-only burns the
    // full budget; liveness-first classifies the exit on poll 1.
    let dead = || false;
    let probe_of_dead = || false;
    let mut clock = WaitClock {
        now_s: 0,
        step_s: 2,
    };

    let slow = wait_probe_only(probe_of_dead, 5400, &mut clock);
    assert_eq!(slow, WaitExit::NotHealthyWithinTimeout);
    assert_eq!(clock.now_s, 5400, "the whole deadline was burned");

    let mut clock2 = WaitClock {
        now_s: 0,
        step_s: 2,
    };
    let fast = wait_liveness_first(dead, probe_of_dead, 5400, &mut clock2);
    assert_eq!(fast, WaitExit::ExitedBeforeHealthy);
    assert_eq!(clock2.now_s, 0, "one poll, zero deadline spent");
}

#[test]
fn f3_healthy_child_not_disturbed() {
    // slow boot then ready at t=30: both shapes eventually return Ready,
    // and liveness-first adds no cost to the healthy path.
    let mut boot_ticks = 15; // ready after 15 polls of 2 s
    let alive = || true;
    let probe = || {
        if boot_ticks == 0 {
            true
        } else {
            boot_ticks -= 1;
            false
        }
    };
    let mut clock = WaitClock {
        now_s: 0,
        step_s: 2,
    };
    assert_eq!(
        wait_liveness_first(alive, probe, 5400, &mut clock),
        WaitExit::Ready
    );
    assert_eq!(clock.now_s, 30);
}

#[test]
fn f3_never_ready_is_bounded_named_timeout() {
    let alive = || true;
    let probe = || false;
    let mut clock = WaitClock {
        now_s: 0,
        step_s: 2,
    };
    let exit = wait_liveness_first(alive, probe, 600, &mut clock);
    assert_eq!(exit, WaitExit::NotHealthyWithinTimeout);
    assert!(clock.now_s >= 600, "the deadline is honored");
    // the two failure classes stay DISTINCT — the caller can name the
    // disease (crashed vs slow) instead of one boolean `false`.
    assert_ne!(
        WaitExit::ExitedBeforeHealthy,
        WaitExit::NotHealthyWithinTimeout
    );
}

#[test]
fn f3_dead_then_probed_does_not_mask_readiness() {
    // the ordering is load-bearing: even if a dead child's socket file
    // lingers (the socket probe could still "succeed" on a stale path
    // in the real world), the liveness check fires FIRST.
    let alive = || false;
    // a stale socket that answers: the probe-only arm would return Ready
    // on a corpse.
    let stale_socket_answers = || true;
    let mut clock = WaitClock {
        now_s: 0,
        step_s: 2,
    };
    assert_eq!(
        wait_liveness_first(alive, stale_socket_answers, 5400, &mut clock),
        WaitExit::ExitedBeforeHealthy,
        "liveness-first refuses to call a corpse ready"
    );
}

// ═════ F4 — calibration-band-two-sided-straddles-one-zero-cv-frozen-clock ═════

#[derive(Debug, Clone)]
struct CalibrationEntry {
    axis_mean: f64,
    axis_cv: f64,
    band_low: f64,
    band_high: f64,
}

#[derive(Debug, PartialEq, Eq)]
enum CalVerdict {
    Recorded,
    /// a refusal, not a warning: NO file is written
    Refused(String),
}

/// The calibration validator: the three refusal laws + the straddle law.
fn validate_calibration(entry: &CalibrationEntry, cv_ceiling: f64) -> CalVerdict {
    // (1) every measured value and every band edge positive — a ZERO CV
    // across repeated passes is a frozen clock, not a stable box.
    let fields = [
        entry.axis_mean,
        entry.axis_cv,
        entry.band_low,
        entry.band_high,
    ];
    for v in fields {
        if !v.is_finite() {
            return CalVerdict::Refused(format!("{v} is not finite"));
        }
        if v <= 0.0 {
            return CalVerdict::Refused(format!(
                "{v}; every measured value and every band edge must be positive \
                 (a zero CV across repeated passes is a frozen clock, not a stable box)"
            ));
        }
    }
    // (2) the band is TWO-SIDED and straddles 1 — a one-sided band can
    // never call the judged leg healthy in one of the two directions.
    if !(entry.band_low < 1.0 && 1.0 < entry.band_high) {
        return CalVerdict::Refused(format!(
            "band [{}, {}]; a band must straddle 1 (low < 1 < high), or the leg it \
             judges can never be healthy",
            entry.band_low, entry.band_high
        ));
    }
    // (3) a CV above the ceiling is equally a refusal: a box that cannot
    // repeat itself has no band worth recording.
    if entry.axis_cv > cv_ceiling {
        return CalVerdict::Refused(format!(
            "CV {} exceeds {}; a CV above the ceiling is a refusal, not a warning — \
             the box did not repeat itself",
            entry.axis_cv, cv_ceiling
        ));
    }
    CalVerdict::Recorded
}

/// The band's USE: it can only STOP a run (leg outside the band), never
/// move a score (the live ratio is the denominator, never this band).
fn band_stops_never_moves(leg_ratio: f64, band: &CalibrationEntry) -> Option<bool> {
    if leg_ratio < band.band_low || leg_ratio > band.band_high {
        Some(false) // kill the run by name
    } else {
        None // inside the band: no opinion, the score stands on its own legs
    }
}

#[test]
fn f4_one_sided_band_refused() {
    // a band [1.0, 1.02] never calls an improvement healthy — refused.
    let e = CalibrationEntry {
        axis_mean: 1.0,
        axis_cv: 0.005,
        band_low: 1.0,
        band_high: 1.02,
    };
    match validate_calibration(&e, 0.01) {
        CalVerdict::Refused(msg) => assert!(msg.contains("straddle 1"), "{msg}"),
        other => panic!("expected refusal, got {other:?}"),
    }
    // a band [0.98, 1.0] never calls a regression dangerous — refused.
    let e2 = CalibrationEntry {
        axis_mean: 1.0,
        axis_cv: 0.005,
        band_low: 0.98,
        band_high: 1.0,
    };
    assert!(matches!(
        validate_calibration(&e2, 0.01),
        CalVerdict::Refused(_)
    ));
}

#[test]
fn f4_zero_cv_is_a_frozen_clock_not_stability() {
    // a submission whose repeated passes read IDENTICAL (CV = 0) is
    // refused: perfect repetition is a frozen clock, not a stable box.
    let e = CalibrationEntry {
        axis_mean: 1.0,
        axis_cv: 0.0,
        band_low: 0.98,
        band_high: 1.02,
    };
    match validate_calibration(&e, 0.01) {
        CalVerdict::Refused(msg) => {
            assert!(msg.contains("frozen clock"), "{msg}");
        }
        other => panic!("zero CV must refuse, got {other:?}"),
    }
}

#[test]
fn f4_high_cv_refused_and_no_file_written() {
    // the calibrator refuses to WRITE when the CV exceeds the ceiling —
    // the refusal is the point of the repeated passes.
    let e = CalibrationEntry {
        axis_mean: 1.0,
        axis_cv: 0.02,
        band_low: 0.97,
        band_high: 1.03,
    };
    let verdict = validate_calibration(&e, 0.01);
    match &verdict {
        CalVerdict::Refused(msg) => assert!(msg.contains("did not repeat itself"), "{msg}"),
        other => panic!("expected refusal, got {other:?}"),
    }
    // the file-existence contract: refused means the artifact is absent.
    let file_written = matches!(verdict, CalVerdict::Recorded);
    assert!(!file_written);
}

#[test]
fn f4_healthy_band_recorded_and_stops_only() {
    let e = CalibrationEntry {
        axis_mean: 1.0,
        axis_cv: 0.003,
        band_low: 0.97,
        band_high: 1.03,
    };
    assert_eq!(validate_calibration(&e, 0.01), CalVerdict::Recorded);
    // inside the band: no opinion (the score's denominator is the LIVE
    // ratio, never the calibration).
    assert_eq!(band_stops_never_moves(1.001, &e), None);
    // outside: the run is killed by name.
    assert_eq!(band_stops_never_moves(1.10, &e), Some(false));
    assert_eq!(band_stops_never_moves(0.90, &e), Some(false));
}

#[test]
fn f4_nonfinite_refused() {
    let e = CalibrationEntry {
        axis_mean: f64::NAN,
        axis_cv: 0.003,
        band_low: 0.97,
        band_high: 1.03,
    };
    assert!(matches!(
        validate_calibration(&e, 0.01),
        CalVerdict::Refused(_)
    ));
}

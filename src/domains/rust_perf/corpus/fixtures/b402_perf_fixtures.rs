//! Batch 402 rust_perf fixture preflight — the mlxfast-27b delta
//! residue's rust_perf half (Plan 402 / Research 437,
//! Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923, MIT) as ONE
//! standalone file (2 rules, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! sources are the arena's docs + shell infra). Compiled AND executed
//! BEFORE entry:
//! `rustc --edition 2021 --test b402_perf_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (lower-median-pair): the outlier arm (a mean drifts with one
//!   unrepresentative prompt, the lower median does not); the
//!   cherry-pick arm (best-pair exceeds the certified statistic); the
//!   parity arms (odd -> middle, even -> LOWER of the two central);
//!   the artifact arm (the scored pair IS a measured pair, the others
//!   stay recorded as measured).
//! - F2 (caller-exclusive-lock): the held arm (try-lock WouldBlock ->
//!   ok); the not-held arm (try-lock succeeds -> refuse BY NAME); the
//!   acquire-and-drop defect arm (the helper's lock outlives nothing —
//!   the resident would outlive it); the poisoned arm (refuse by name).

// ══════════════ F1 — score-is-lower-median-pair-never-mean ══════════════

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pair {
    composite: f64,
}

fn lower_median_pair(mut pairs: Vec<Pair>) -> Option<Pair> {
    if pairs.is_empty() {
        return None;
    }
    pairs.sort_by(|a, b| a.composite.total_cmp(&b.composite));
    let n = pairs.len();
    let idx = if n % 2 == 1 { n / 2 } else { n / 2 - 1 };
    Some(pairs[idx])
}

fn mean_score(pairs: &[Pair]) -> f64 {
    pairs.iter().map(|p| p.composite).sum::<f64>() / pairs.len() as f64
}

fn best_score(pairs: &[Pair]) -> f64 {
    pairs.iter().map(|p| p.composite).fold(f64::MIN, f64::max)
}

struct Artifact {
    scored: Pair,
    paired_legs: Vec<Pair>,
}

#[test]
fn f1_outlier_drifts_mean_not_lower_median() {
    // three honest pairs + one unrepresentative prompt (a huge outlier)
    let pairs = vec![
        Pair { composite: 1.61 },
        Pair { composite: 1.59 },
        Pair { composite: 1.62 },
        Pair { composite: 4.90 }, // the outlier prompt
    ];
    let certified = lower_median_pair(pairs.clone()).unwrap();
    assert!(
        (certified.composite - 1.61).abs() < 1e-12,
        "the certified statistic is the honest central pair, not the outlier's shadow"
    );
    let m = mean_score(&pairs);
    assert!(
        m > 2.0,
        "the mean drifts to {m} — one prompt moved the headline"
    );
    assert_ne!(m, certified.composite);
}

#[test]
fn f1_best_pair_is_not_the_statistic() {
    let pairs = vec![
        Pair { composite: 1.10 },
        Pair { composite: 1.12 },
        Pair { composite: 1.11 },
        Pair { composite: 2.40 },
    ];
    let certified = lower_median_pair(pairs.clone()).unwrap();
    assert!(
        certified.composite < best_score(&pairs),
        "the best pair is a cherry-pick; the lower median leans AGAINST the submitter"
    );
}

#[test]
fn f1_parity_odd_middle_even_lower_central() {
    let odd = vec![
        Pair { composite: 1.5 },
        Pair { composite: 1.2 },
        Pair { composite: 1.9 },
    ];
    assert_eq!(
        lower_median_pair(odd).unwrap().composite,
        1.5,
        "odd: the middle pair"
    );
    let even = vec![
        Pair { composite: 1.1 },
        Pair { composite: 1.3 },
        Pair { composite: 1.5 },
        Pair { composite: 1.7 },
    ];
    assert_eq!(
        lower_median_pair(even).unwrap().composite,
        1.3,
        "even: the LOWER of the two central pairs (never averaged — a mean 1.4 is no measured pair)"
    );
}

#[test]
fn f1_artifact_preserves_paired_legs() {
    let pairs = vec![
        Pair { composite: 1.2 },
        Pair { composite: 1.4 },
        Pair { composite: 1.3 },
    ];
    let scored = lower_median_pair(pairs.clone()).unwrap();
    let art = Artifact {
        scored,
        paired_legs: pairs.clone(),
    };
    assert_eq!(
        art.paired_legs.len(),
        3,
        "every pair stays recorded as measured"
    );
    assert!(
        art.paired_legs.contains(&art.scored),
        "the scored pair IS a measured pair"
    );
}

// ══════════════ F2 — caller-exclusive-lock-asserted-never-taken ══════════════

use std::sync::{Mutex, TryLockError};

struct GpuWindow(Mutex<()>);

impl GpuWindow {
    /// the custody assertion: the caller's lock must be HELD
    fn assert_caller_holds(window: &GpuWindow) -> Result<(), String> {
        match window.0.try_lock() {
            Ok(_would_be_holder) => Err(
                "gpu-window: not held — the caller must own the exclusive GPU window; \
                 this helper never takes the lock itself"
                    .to_string(),
            ),
            Err(TryLockError::WouldBlock) => Ok(()),
            Err(TryLockError::Poisoned(_)) => {
                Err("gpu-window: poisoned — a window holder panicked; fix that first".to_string())
            }
        }
    }
}

struct Resident;

/// the DEFECT arm: the helper acquires-and-drops — the guard's lifetime
/// ends at the helper's exit while the resident it booted keeps running
fn boot_resident_acquire_and_drop(window: &GpuWindow) -> Resident {
    let _guard = window.0.lock().unwrap(); // released HERE
    Resident // the resident outlives the helper's lock — outside the window
}

#[test]
fn f2_held_window_passes() {
    let window = GpuWindow(Mutex::new(()));
    let _caller_guard = window.0.lock().unwrap(); // the caller owns the window
    assert!(
        GpuWindow::assert_caller_holds(&window).is_ok(),
        "try-lock WouldBlock == held == the correct posture"
    );
}

#[test]
fn f2_not_held_refuses_by_name() {
    let window = GpuWindow(Mutex::new(()));
    // nobody holds the window: the probe lock SUCCEEDS -> refuse
    let err = GpuWindow::assert_caller_holds(&window).unwrap_err();
    assert!(
        err.contains("not held"),
        "the refusal names the failure: {err}"
    );
    assert!(err.contains("never takes the lock itself"));
}

#[test]
fn f2_helper_lock_does_not_cover_the_resident() {
    let window = GpuWindow(Mutex::new(()));
    // the acquire-and-drop defect: the helper holds the lock only for
    // its own body; probing AFTER it exits finds the window unheld even
    // though the resident is running — the residency leaked the window
    let _resident = boot_resident_acquire_and_drop(&window);
    drop(_resident);
    let probe = GpuWindow::assert_caller_holds(&window);
    // with no caller guard, the probe lock would SUCCEED — proving no
    // window is held; the fixed path REFUSES here instead of booting
    assert!(
        probe.is_err(),
        "after the helper exits, nothing holds the window"
    );
}

#[test]
fn f2_poisoned_refuses_by_name() {
    let window = std::sync::Arc::new(GpuWindow(Mutex::new(())));
    // a HOLDER panics while holding (a separate thread — same-thread
    // re-lock of a std Mutex would deadlock, not poison)
    let w = window.clone();
    let holder = std::thread::spawn(move || {
        let _g = w.0.lock().unwrap();
        panic!("holder dies while holding");
    });
    assert!(
        holder.join().is_err(),
        "the holder panicked — the mutex is now poisoned"
    );
    let err = GpuWindow::assert_caller_holds(&window).unwrap_err();
    assert!(
        err.contains("poisoned"),
        "the refusal names the poisoned window: {err}"
    );
}

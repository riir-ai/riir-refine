#![cfg(feature = "consumed_window_ledger")]
//! Issue 232 T1 — the G1 fixture (Research 495, the T³ holdout-epistemology
//! extraction): synthetic outcome store + pinned selection replay.
//!
//! Pre-registered predicates (the issue's proof-path ledger):
//! 1. **Own-window flag:** a selection replayed against the SAME store
//!    state twice consults `Fresh` then `Consumed` — the re-judgment is
//!    REPORTED (UNDECIDABLE), never folded into a refusal.
//! 2. **Disjoint window proceeds:** recording a new outcome mutates the
//!    window digest; the evolved window reads `Fresh`.
//! 3. **The k-th-reuse probe + synthetic-truth degradation:** under a
//!    self-confirming append loop (each pass feeds outcomes sampled at the
//!    rules' CURRENT ratings — the adaptive-reuse poison in miniature),
//!    the probe (reuse ≥ 1) fires by the k = 2 pass, and the synthetic
//!    truth metric (mean |rating − true| over rules, truth = the PL
//!    conversion of the seeded win probabilities) DEGRADES as the passes
//!    accumulate — the direction T³ measured (own-half testing inverts
//!    below baseline).
//!
//! Determinism: no RNG — every sample is fixed integer arithmetic; epochs
//! are pinned constants. The digest/replay/every Elo call is the shipped
//! deterministic substrate, so the assertions pin exact measured behavior.
//!
//! Class honesty: reusable-holdout / adaptive data analysis (Dwork et al.,
//! Science 2015) is the KNOWN class; this fixture demonstrates the
//! surface-specific ledger, never a novel mechanism.

use riir_refine::consumed_window::{WindowLedger, WindowRead};
use riir_refine::traj_store::TrajStore;
use std::path::PathBuf;

/// A scratch store root (unique per test process).
fn store_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("cwl_g1_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Seed `rows_per_rule` synthetic outcomes per rule from its TRUE win
/// probability, with fixed integer arithmetic standing in for RNG (the
/// `global_rng_gate` discipline): outcome index k wins iff
/// `(k*53 + i*29) % 100 < p_i*100`.
fn seed_true_outcomes(
    store: &TrajStore,
    rules: &[(String, f64)],
    rows_per_rule: usize,
) -> Result<(), String> {
    for (i, (rule, p)) in rules.iter().enumerate() {
        for k in 0..rows_per_rule {
            let win = (k * 53 + i * 29) % 100 < (p * 100.0) as usize;
            store.record(rule, &format!("shape_{i}"), "fix()", win, win, "g1-seed")?;
        }
    }
    Ok(())
}

/// The PL-conversion true rating for win probability `p` (base 1200):
/// `1200 + 400·log10(p/(1−p))` — the equilibrium [`riir_refine::elo`]'s
/// own gate uses.
fn true_rating(p: f64) -> f64 {
    1200.0 + 400.0 * (p / (1.0 - p)).log10()
}

/// The Elo expectancy of a rule currently rated `r` against the base-rated
/// field: `1/(1+10^(−Δ/400))`.
fn expectancy(rating: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-(rating - 1200.0) / 400.0))
}

#[test]
fn own_window_flags_and_disjoint_window_proceeds() {
    let root = store_root("flag");
    let (store, bad) = TrajStore::open(&root).expect("open store");
    assert_eq!(bad, 0, "fresh store has no corrupt rows");

    let rules = vec![("rule_a".to_string(), 0.75), ("rule_b".to_string(), 0.4)];
    seed_true_outcomes(&store, &rules, 6).expect("seed");

    let mut ledger = WindowLedger::new();

    // Pass 1: first replay of this store state — Fresh.
    let (_, read1, digest1) = store
        .recompute_elo_with_ledger(&mut ledger, "sel", 1_000)
        .expect("pass 1");
    assert_eq!(read1, WindowRead::Fresh, "first replay is fresh");

    // Pass 2: the SAME state re-judged — the reuse probe fires, the ledger
    // REPORTS (UNDECIDABLE), and the replay still runs (report-only).
    let (_, read2, digest2) = store
        .recompute_elo_with_ledger(&mut ledger, "sel", 2_000)
        .expect("pass 2");
    assert_eq!(read2, WindowRead::Consumed { reuse: 1 });
    assert_eq!(digest1, digest2, "unchanged store → identical window");
    let line = ledger.report_line(&digest2, &read2);
    assert!(line.contains("verdict=UNDECIDABLE"), "{line}");
    assert!(line.contains("reuse=1"), "{line}");

    // Pass 3: a NEW outcome lands — the window state evolves, the digest
    // moves, and the evolved window reads Fresh (the disjoint-window
    // predicate).
    store
        .record("rule_a", "shape_0", "fix()", true, true, "g1-evolve")
        .expect("new outcome");
    let (_, read3, digest3) = store
        .recompute_elo_with_ledger(&mut ledger, "sel", 3_000)
        .expect("pass 3");
    assert_eq!(read3, WindowRead::Fresh, "evolved window is fresh");
    assert_ne!(digest3, digest1, "new outcome → new window identity");
    assert_eq!(ledger.reuse_of(&digest1), 2);
    assert_eq!(ledger.reuse_of(&digest3), 1);

    // The ledger answers WHICH windows the selection consumed, in order.
    assert_eq!(
        ledger.windows_of("sel"),
        &[digest1.clone(), digest3.clone()]
    );
}

#[test]
fn kth_reuse_probe_fires_and_truth_degrades_under_self_confirmation() {
    let root = store_root("kth");
    let (store, _bad) = TrajStore::open(&root).expect("open store");

    // Six rules, seeded from true win probabilities (deterministic
    // arithmetic, no RNG).
    let truth: Vec<(String, f64)> = [0.9, 0.75, 0.6, 0.5, 0.4, 0.25]
        .iter()
        .enumerate()
        .map(|(i, p)| (format!("rule_{i}"), *p))
        .collect();
    seed_true_outcomes(&store, &truth, 12).expect("seed");

    let true_ratings: Vec<f64> = truth.iter().map(|(_, p)| true_rating(*p)).collect();
    let mut ledger = WindowLedger::new();

    // divergence = mean |rating − true| over the rules.
    let divergence = |store: &TrajStore| -> f64 {
        let table = store.elo_table();
        truth
            .iter()
            .zip(true_ratings.iter())
            .map(|((rule, _), t)| ((table.rating(rule) as f64) - t).abs())
            .sum::<f64>()
            / truth.len() as f64
    };

    let mut divergences = Vec::new();
    let mut first_reuse_pass = None;
    let mut replay_count = 0u32;
    for pass in 1..=8u64 {
        // Each round replays the same store state TWICE: the report replay
        // (Fresh on round 1) and the RE-JUDGMENT replay on the unchanged
        // window — the poison event the ledger exists to name (the
        // score-bench re-run on the window the selection shaped). The
        // second read of round 1 is the overall k = 2 pass.
        let (_, read_report, _) = store
            .recompute_elo_with_ledger(&mut ledger, "sim", pass * 10)
            .expect("report replay");
        let _ = read_report;
        let (_, read_rejudge, _) = store
            .recompute_elo_with_ledger(&mut ledger, "sim-rejudge", pass * 10 + 1)
            .expect("re-judgment replay");
        replay_count += 2;
        if first_reuse_pass.is_none() {
            if let WindowRead::Consumed { reuse: _ } = read_rejudge {
                first_reuse_pass = Some(replay_count);
            }
        }
        divergences.push(divergence(&store));

        // The self-confirming append: 3 outcomes per rule sampled at the
        // rule's CURRENT rating — the adaptive-reuse poison in miniature
        // (the selection's own output seeds the next window). Fixed
        // integer arithmetic again; a draw index never repeats a (pass,
        // rule, k) triple.
        let table = store.elo_table();
        for (i, (rule, _)) in truth.iter().enumerate() {
            let p = expectancy(table.rating(rule) as f64);
            for k in 0..3u32 {
                let win = (pass as u32 * 31 + i as u32 * 17 + k * 7) % 100 < (p * 100.0) as u32;
                store
                    .record(rule, &format!("shape_{i}"), "fix()", win, win, "g1-sim")
                    .expect("append");
            }
        }
    }

    // Predicate 3a: the probe fires by the k = 2 pass (the first
    // RE-judgment — reuse 1).
    assert_eq!(
        first_reuse_pass,
        Some(2),
        "the reuse probe fires at the first re-consumption (k=2), got {first_reuse_pass:?}"
    );

    // Predicate 3b: the synthetic truth metric degrades under the loop —
    // the mean |rating − true| after six self-confirming rounds exceeds
    // the pre-loop reading. (T³'s direction: self-scoring on consumed
    // data inverts BELOW baseline; the sim's magnitude is its own, never
    // a T³ number.)
    assert!(
        divergences[7] > divergences[0],
        "truth metric must degrade under the self-confirming loop: pass1={:.2} pass8={:.2}",
        divergences[0],
        divergences[7]
    );
    assert!(
        divergences[7] > divergences[1],
        "degradation must hold against the post-first-append reading too: pass2={:.2} pass8={:.2}",
        divergences[1],
        divergences[7]
    );

    // And the ledger saw every replay of the loop (two per round — the
    // report pass + the re-judgment pass — 16 consumptions, one distinct
    // state per round since the store grew every round).
    assert_eq!(ledger.consumptions().len(), 16);
    // Every re-judgment read was Consumed; every report read after round 1
    // was Fresh (the store had evolved). Spot-check the shape:
    let rejudges: Vec<_> = ledger
        .consumptions()
        .iter()
        .filter(|c| c.selection == "sim-rejudge")
        .collect();
    assert_eq!(rejudges.len(), 8);
    assert!(rejudges.iter().all(|c| c.reuse >= 1));
}

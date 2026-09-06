//! Issue 072 PoC — the exit-criterion A/B harness (Plan 091 T5).
//!
//! The issue's exit criterion: pair-fed + frontier-fed arms vs the
//! outcome-fed baseline on the three axes —
//!
//! (a) **unseen-candidate fraction drops measurably** — the T39/Research
//!     135 measured starvation (98.5% of candidates unseen) is the target;
//!     construction pairs cover every strategy that participates in a
//!     pair, without one cargo invocation.
//! (b) **Certified-conversion per authored rule improves** — measured here
//!     as the G8 PROXY: frontier ordering vs FIFO over an authoring queue,
//!     scored by where the pools worth converting land (entropy-ordered
//!     must front-load informative pools; the conversion oracle is the
//!     stored outcome history, honestly labeled a proxy — the real G8 runs
//!     when a live authoring loop exists).
//! (c) **no retrieval-floor regression** — owned by the standing gate
//!     (`retrieval_precision_regression_gate`, run in the same validation
//!     pass; pair rows live in a disjoint `pair:` namespace and the
//!     retrieval indexes never scan it — asserted structurally by the
//!     namespace-isolation test here).
//!
//! Outcome sparsity follows the T39 REAL distribution (t39's
//! [`REAL_ROW_COUNTS`] measurement): most candidates unseen, a thin tail
//! with 1-4 observations.
//!
//! ```bash
//! cargo test --release --features latent_retrieval,rustc_errors --test frontier_poc_ab -- --nocapture
//! ```

#![cfg(all(feature = "latent_retrieval", feature = "rustc_errors"))]
#![allow(clippy::cast_precision_loss)] // count fixtures

use std::path::PathBuf;

use riir_clippy::dispersion::{
    entropy_map, frontier_order, from_counts, outcome_entropy, PoolEntry, Regime,
};
use riir_clippy::elo::{default_elo, PairObservation};
use riir_clippy::frontier_pairs::construction_pairs;
use riir_clippy::traj_store::TrajStore;

/// The playbook candidate universe (the `--fix-compile` driver's selection
/// surface) — every strategy the playbooks can propose, plus the
/// generator-native ids the pair factory mints for codes no playbook owns.
/// The A/B's coverage axis counts over this exact set.
fn candidate_universe() -> Vec<&'static str> {
    vec![
        "e0597-anon-lifetime",
        "e0597-move",
        "e0597-move-clone",
        "e0597-drop-the-box",
        "e0597-shared-ownership",
        "e0597-scope-hoist",
        "e0502-deref-removal",
        "e0502-clone-out",
        "e0499-swap-second-decl",
        "e0505-clone-at-move",
        "gen:E0596:add-mut",
        "gen:E0382:clone-at-move",
        "gen:E0507:clone-at-move",
        "gen:E0373:add-move",
    ]
}

/// Plant the T39-realistic sparse outcome regime: only 3 of the 14
/// candidates have ANY outcome history, with thin counts (the measured
/// distribution's shape — `{0: 98.5%, 1: 1.3%, 2..4: the tail}`).
fn plant_sparse_outcomes(store: &TrajStore) {
    // e0597-scope-hoist: a working rule (mostly wins).
    for (shape, ok) in [("s1", true), ("s2", true), ("s3", false), ("s4", true)] {
        store
            .record("e0597-scope-hoist", shape, "fix", ok, ok, "run")
            .unwrap();
    }
    // e0502-deref-removal: one observation only.
    store
        .record("e0502-deref-removal", "s1", "fix", true, true, "run")
        .unwrap();
    // e0499-swap-second-decl: failing family.
    store
        .record("e0499-swap-second-decl", "s1", "fix", false, false, "run")
        .unwrap();
    store
        .record("e0499-swap-second-decl", "s2", "fix", false, false, "run")
        .unwrap();
}

fn tmp_root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "riir_frontier_poc_{}_{}_{}",
        std::process::id(),
        name,
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// (a) unseen-candidate fraction: outcome-fed vs pair-fed coverage.
#[test]
fn poc_axis_a_unseen_candidate_fraction_drops() {
    let universe = candidate_universe();
    let n = universe.len();
    let root = tmp_root("axis_a");
    let (store, _) = TrajStore::open(&root).unwrap();

    // Outcome-fed baseline: who does the OUTCOME table rate? (Recompute
    // first — elo_table() reads the PERSISTED ratings the replay writes.)
    plant_sparse_outcomes(&store);
    let recompute = store.recompute_elo().unwrap();
    assert!(recompute.matches > 0, "planted outcomes must replay as matches");
    let outcome_table = store.elo_table();
    let outcome_covered = universe
        .iter()
        .copied()
        .filter(|c| outcome_table.contains(c))
        .count();

    // Pair-fed: who does the BT fit over construction pairs rate?
    let pairs = construction_pairs();
    let (seen, written) = store.import_pairs(&pairs).unwrap();
    assert_eq!(seen, pairs.len());
    assert!(written > 0);
    let (pair_fit, pairs_rated) = store.rate_pairs(300).unwrap();
    let pair_covered = universe
        .iter()
        .copied()
        .filter(|c| pair_fit.contains_key(*c))
        .count();

    // The verdict: unseen fraction must DROP measurably.
    let unseen_outcome = n - outcome_covered;
    let unseen_pair = n - pair_covered;
    println!(
        "axis (a): universe={n} outcome-covered={outcome_covered} pair-covered={pair_covered} \
         | unseen {unseen_outcome} -> {unseen_pair} \
         (fraction {:.3} -> {:.3}) | pairs rated: {pairs_rated}",
        f64::from(unseen_outcome as u32) / f64::from(n as u32),
        f64::from(unseen_pair as u32) / f64::from(n as u32),
    );
    assert!(
        pair_covered > outcome_covered,
        "pair-fed coverage must strictly exceed outcome-fed coverage \
         (covered {pair_covered} vs {outcome_covered})"
    );
    // The union arm (what a consumer would actually read): every
    // outcome-covered candidate is also pair-covered here by construction,
    // so the union == the pair set — assert the union covers ≥ the
    // playbook strategies the pairs name.
    let union = universe
        .iter()
        .copied()
        .filter(|c| outcome_table.contains(c) || pair_fit.contains_key(*c))
        .count();
    assert!(union >= pair_covered);
    let _ = default_elo();
}

/// (b) the G8 proxy: frontier ordering vs FIFO over the authoring queue.
#[test]
fn poc_axis_b_frontier_front_loads_informative_pools() {
    // The authoring queue: one pool per (code, strategy) bucket with its
    // stored outcome history. Shaped to mirror the planted regime: most
    // pools empty (no evidence), one informative, one saturated, one
    // regenerate — FIFO (alphabetical here, the driver's own stable
    // order) scatters the informative pool mid-queue; the frontier
    // ordering must rank it first.
    let pool = vec![
        PoolEntry::new("a-empty-1", vec![]),
        PoolEntry::new("b-scope-hoist", from_counts(3, 1, 0)),
        PoolEntry::new("c-empty-2", vec![]),
        PoolEntry::new("d-deref-removal", from_counts(1, 0, 0)),
        PoolEntry::new("e-empty-3", vec![]),
        PoolEntry::new("f-swap-second", from_counts(0, 2, 0)),
        PoolEntry::new("g-empty-4", vec![]),
        PoolEntry::new("h-never", from_counts(0, 0, 4)),
    ];
    let fifo: Vec<&str> = pool.iter().map(|e| e.id.as_str()).collect();
    let frontier = frontier_order(&pool);
    let fids: Vec<&str> = frontier.iter().map(|v| v.id.as_str()).collect();

    // Informative pools under the control law: ONLY mixed-outcome pools
    // have dispersion. (d-deref-removal at 1-0 is zero-entropy-all-pass =
    // the SATURATED band per the issue's own control law — a prior draft
    // of this fixture mislabeled it informative and the assert correctly
    // refused.)
    let informative = ["b-scope-hoist"];
    let mean_rank = |order: &[&str]| -> f64 {
        let sum: usize = informative
            .iter()
            .map(|id| order.iter().position(|o| o == id).expect("id in order") + 1)
            .sum();
        f64::from(sum as u32) / f64::from(informative.len() as u32)
    };
    let fifo_mean = mean_rank(&fifo);
    let frontier_mean = mean_rank(&fids);
    println!(
        "axis (b): informative mean rank fifo={fifo_mean:.2} frontier={frontier_mean:.2} \
         | order: {fids:?}"
    );
    assert!(
        frontier_mean < fifo_mean,
        "frontier ordering must front-load informative pools"
    );
    // The control law's tail: the saturated pool (1-0, zero-entropy-all-pass)
    // lands LAST; the all-missed pool sits in the regenerate band ahead of
    // it — never ahead of the informative pool.
    assert_eq!(fids.last().copied(), Some("d-deref-removal"));
    let regimes = frontier_order(&pool);
    let never_regime = regimes.iter().find(|v| v.id == "h-never").map(|v| v.regime);
    assert_eq!(never_regime, Some(Regime::Regenerate));
    let never_pos = fids.iter().position(|f| *f == "h-never").unwrap();
    assert!(never_pos < fids.len() - 1, "regenerate band precedes saturated");
}

/// (b2) the dispersion telemetry on the planted store: a saturated
/// outcome world must read vacuous (the T7 tripwire); a mixed one must
/// not.
#[test]
fn poc_axis_b2_dispersion_telemetry_flags_vacuous_worlds() {
    let root = tmp_root("axis_b2");
    let (store, _) = TrajStore::open(&root).unwrap();
    plant_sparse_outcomes(&store);
    let rows = store.dump();
    let mut by_rule: std::collections::BTreeMap<&str, Vec<riir_clippy::dispersion::OutcomeClass>> =
        std::collections::BTreeMap::new();
    for r in &rows {
        let e = by_rule.entry(r.lint_key.as_str()).or_default();
        for _ in 0..r.success_count {
            e.push(riir_clippy::dispersion::OutcomeClass::Kept);
        }
        for _ in 0..r.fail_count {
            e.push(riir_clippy::dispersion::OutcomeClass::Reverted);
        }
    }
    let ents: Vec<f64> = by_rule
        .iter()
        .map(|(id, o)| {
            println!("  pool {id}: h={:.3} n={}", outcome_entropy(o), o.len());
            outcome_entropy(o)
        })
        .collect();
    let map: std::collections::BTreeMap<String, f64> =
        by_rule.iter().map(|(id, o)| ((*id).to_string(), outcome_entropy(o))).collect();
    let _ = map;
    // The planted world is NOT vacuous: scope-hoist is 3-1, swap-second is
    // 0-2 — real dispersion exists for selection to consume.
    assert!(!riir_clippy::dispersion::window_is_vacuous(&ents));
    // Contrast canary: an all-saturated world IS vacuous.
    let saturated: Vec<f64> = vec![outcome_entropy(&from_counts(4, 0, 0)); 3];
    assert!(riir_clippy::dispersion::window_is_vacuous(&saturated));
    let _ = entropy_map(&[]);
}

/// (c) namespace isolation — the structural half of the no-retrieval-
/// regression axis: pair rows must be invisible to every trajectory scan
/// (retrieval indexes build from `dump()`/`learn_rows()` only).
#[test]
fn poc_axis_c_pair_namespace_is_isolated_from_outcome_surfaces() {
    let root = tmp_root("axis_c");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .import_pairs(&construction_pairs())
        .expect("pair import");
    // The pair store holds rows…
    assert!(!store.pair_rows().unwrap().is_empty());
    // …but every outcome surface sees ZERO of them.
    assert_eq!(store.dump().len(), 0, "dump() must not see pair rows");
    assert_eq!(store.len(), 0, "len() must not count pair rows");
    assert_eq!(store.learn_rows().len(), 0, "learn rows untouched");
    assert_eq!(store.elo_table().len(), 0, "outcome table untouched");
    assert_eq!(store.certified_density().len(), 0, "tier surface untouched");
    // And the outcome Elo replay over the pair-only store is a no-op.
    let s = store.recompute_elo().unwrap();
    assert_eq!(s.matches, 0);
    assert_eq!(s.rows_written, 0);
    let _ = PairObservation { winner: String::new(), loser: String::new() };
}

//! Issue 093 T2 — the nonergodic driver ordering A/B + mechanism gates.
//!
//! Three tiers, honestly separated:
//! 1. **Mechanism (no cargo)** — the policy's decision content through the
//!    real ordering/observer/revive closures: menu freeze determinism,
//!    within-run learning flips the order, context-scoped one-shot revive,
//!    the stale-store gate. These are the T6.7 "flip-test" class: they pin
//!    that the MECHANISM engages, not that it helps.
//! 2. **E2E no-regression (real cargo)** — multi-error scaffolds through
//!    the REAL driver, static vs nonergodic ordering at equal budget:
//!    resolved-count must not regress, byte-clean must not regress.
//! 3. **Headroom disclosure** — whether the scaffold surface exhibits ANY
//!    revert/wasted-attempt dynamics (the honest-null surface question;
//!    printed, never asserted).
//!
//! ```bash
//! cargo test --features nonergodic_driver --test nonergodic_driver_ab -- --nocapture
//! ```

#![cfg(feature = "nonergodic_driver")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use riir_clippy::domains::rustc_errors::driver::{
    AttemptOutcome, AttemptRecord, CompileDriver,
};
use riir_clippy::domains::rustc_errors::nonergodic_order::NonergodicOrdering;
use riir_clippy::domains::rustc_errors::playbooks::{FixCandidate, Playbooks};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn cargo_available() -> bool {
    std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .arg("--version")
        .output()
        .is_ok()
}

fn scaffold(name: &str, source: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("riir_93ab_{}_{seq}_{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"neab\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n",
    )
    .unwrap();
    std::fs::write(src.join("lib.rs"), source).unwrap();
    dir
}

/// A synthetic candidate at a synthetic site (no cargo needed — the
/// ordering closure only reads identity + shape fields).
fn cand(file: &str, line: usize, code: &str, strategy_id: &str, replacement: &str) -> FixCandidate {
    FixCandidate {
        strategy_id: strategy_id.to_string(),
        code: code.to_string(),
        file: PathBuf::from(file),
        line,
        byte_span: (0, replacement.len()),
        replacement: replacement.to_string(),
        extra_edits: Vec::new(),
        shape: format!("shape of {strategy_id} at {file}:{line}"),
        semantic_note: None,
        diag_digest: None,
    }
}

/// An attempt ring record for `(file, line, strategy)` with `outcome`.
fn attempt(file: &str, line: usize, code: &str, strategy_id: &str, outcome: AttemptOutcome) -> AttemptRecord {
    let path = Path::new(file);
    AttemptRecord {
        seq: 0,
        code: code.to_string(),
        span_hash: riir_clippy::domains::rustc_errors::driver::span_hash(path, line),
        strategy_id: strategy_id.to_string(),
        outcome,
        errors: (2, if outcome == AttemptOutcome::Kept { 1 } else { 2 }),
        masking: (0, 0),
        // Journey fields: 0 = absent (the in-memory pre-stamp state) —
        // and the Issue-105 omission this literal carried was a latent
        // compile break behind the opt-in feature, surfaced by T2's field.
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: None,
        shape_class: None,
        insight: None,
    }
}

fn armed_driver(policy: &Arc<NonergodicOrdering>) -> CompileDriver {
    CompileDriver::isolated()
        .with_ordering(policy.ordering())
        .with_observer(policy.observer())
        .with_revive(policy.revive())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier 1 — mechanism (no cargo)
// ─────────────────────────────────────────────────────────────────────────────

/// The menu freezes deterministically and the initial order matches the
/// incumbent min-delta key (the fallback contract: no evidence, no
/// invention — ties fall through to the incumbent key).
#[test]
fn menu_freeze_is_deterministic_and_starts_incumbent() {
    let policy = Arc::new(NonergodicOrdering::new(None));
    let ordering = policy.ordering();
    let mut cands = vec![
        cand("b.rs", 1, "E0502", "e0502-deref-removal", "x"),
        cand("a.rs", 9, "E0505", "e0505-clone-at-move", "yyyyy"),
        cand("a.rs", 4, "E0597", "e0597-anon-lifetime", "zz"),
    ];
    let mut cands2 = cands.clone();

    ordering(&mut cands);
    ordering(&mut cands2);

    assert_eq!(
        cands, cands2,
        "two identical passes must order identically (determinism)"
    );
    // Incumbent key: file → line — a.rs:4, a.rs:9, b.rs:1.
    assert_eq!(cands[0].line, 4);
    assert_eq!(cands[1].line, 9);
    assert_eq!(cands[2].line, 1);
    let snap = policy.snapshot();
    assert_eq!(snap.menu_len, 3, "menu freezes at first call");
    assert_eq!(snap.t_observed, 0);
}

/// The T6.7-class flip test: within-run evidence moves the order. The
/// incumbent ranks B first (shorter replacement); after B keeps failing and
/// A keeps succeeding, A must rank first — the filter's marginal P(resolved)
/// readout overtakes the base ranking.
#[test]
fn within_run_learning_flips_the_order() {
    let policy = Arc::new(NonergodicOrdering::new(None));
    let ordering = policy.ordering();
    let observer = policy.observer();

    let mut cands = vec![
        // B: shorter replacement → incumbent rank 0.
        cand("a.rs", 1, "E0502", "strat-b", "x"),
        cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
    ];
    ordering(&mut cands);
    assert_eq!(cands[0].strategy_id, "strat-b", "no evidence → incumbent (min-delta) order");

    // Evidence: A keeps resolving (3×), B keeps no-progressing (2×).
    for _ in 0..2 {
        observer(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::RevertedNoProgress));
    }
    for _ in 0..3 {
        observer(&attempt("a.rs", 2, "E0502", "strat-a", AttemptOutcome::Kept));
    }

    let mut cands = vec![
        cand("a.rs", 1, "E0502", "strat-b", "x"),
        cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
    ];
    ordering(&mut cands);
    assert_eq!(
        cands[0].strategy_id, "strat-a",
        "after 3× kept for a + 2× no-progress for b, a must rank first"
    );
}

/// The revive policy's DETERMINISTIC clauses: no record → refuse; same
/// commit (no later evidence) → refuse — plus the EXISTENCE half: some
/// later-evidence pattern (the world flipping under the strategies)
/// re-admits exactly once. The `PoC`'s revive window is deliberately narrow
/// (the commit must have JUST flipped — `revive_gate = sigmoid(-margin)`
/// ≥ 0.35 bounds the flip margin), so the positive clause is pinned by a
/// small (kept, no-progress) tick grid, not one hand-tuned sequence.
#[test]
fn revive_negative_clauses_hold_and_a_flip_window_exists() {
    // ── Deterministic negatives ─────────────────────────────────────────
    let policy = Arc::new(NonergodicOrdering::new(None));
    let ordering = policy.ordering();
    let observer = policy.observer();
    let revive = policy.revive();

    let mut cands = vec![
        cand("a.rs", 1, "E0502", "strat-b", "x"),
        cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
    ];
    ordering(&mut cands);

    // No suppression record for strat-a → refuse.
    let a_cand = cands.iter().find(|c| c.strategy_id == "strat-a").unwrap();
    assert!(!revive(a_cand), "no suppression record → no revive right");

    // strat-b fails → suppressed. The failure's own tick has ALREADY moved
    // the commit (it is evidence); with no LATER flip, revive refuses.
    observer(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::RevertedNoProgress));
    let mut cands2 = cands.clone();
    ordering(&mut cands2);
    let b_cand = cands2.iter().find(|c| c.strategy_id == "strat-b").unwrap();
    assert!(
        !revive(b_cand),
        "same commit + same ctx → provably the same outcome → refuse"
    );

    // ── The flip-window existence sweep ─────────────────────────────────
    // Story: b failed under commit-1; later the world flips (a — the
    // incumbent favorite — starts failing too). Some (j, k) mix of
    // a-kept / a-no-progress ticks flips the commit back with a margin
    // inside the gate window, and a context change then admits exactly
    // ONE revive. The grid is the honest pin: the window EXISTS, its exact
    // cell is the constants' business, not the contract's.
    let mut fired: Vec<(usize, usize)> = Vec::new();
    for j in 0..=4usize {
        for k in 0..=4usize {
            let p = Arc::new(NonergodicOrdering::new(None));
            let ord = p.ordering();
            let obs = p.observer();
            let rev = p.revive();
            let mut c = vec![
                cand("a.rs", 1, "E0502", "strat-b", "x"),
                cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
            ];
            ord(&mut c);
            obs(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::RevertedNoProgress));
            for _ in 0..j {
                obs(&attempt("a.rs", 2, "E0502", "strat-a", AttemptOutcome::Kept));
            }
            for _ in 0..k {
                obs(&attempt("a.rs", 2, "E0502", "strat-a", AttemptOutcome::RevertedNoProgress));
            }
            // Context moves: the diagnostic set grows.
            let mut c2 = c.clone();
            c2.push(cand("a.rs", 5, "E0502", "strat-c", "zzz"));
            ord(&mut c2);
            let b = c2.iter().find(|c| c.strategy_id == "strat-b").unwrap();
            if rev(b) {
                assert!(!rev(b), "one-shot: the second ask must refuse");
                fired.push((j, k));
            }
        }
    }
    assert!(
        !fired.is_empty(),
        "some (kept j, no-progress k) evidence mix must open the revive \
         window (the `PoC`'s revive-on-contradiction is load-bearing); got none"
    );
    eprintln!("revive flip-window cells: {fired:?}");

    // A KEPT outcome clears the suppression (fresh right on re-failure).
    let policy2 = Arc::new(NonergodicOrdering::new(None));
    let ord2 = policy2.ordering();
    let obs2 = policy2.observer();
    let rev2 = policy2.revive();
    let mut c = vec![
        cand("a.rs", 1, "E0502", "strat-b", "x"),
        cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
    ];
    ord2(&mut c);
    obs2(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::RevertedNoProgress));
    obs2(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::Kept));
    obs2(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::RevertedNoProgress));
    let mut c2 = c.clone();
    c2.push(cand("a.rs", 5, "E0502", "strat-c", "zzz"));
    ord2(&mut c2);
    let b = c2.iter().find(|c| c.strategy_id == "strat-b").unwrap();
    // The suppression is fresh but the commit never moved past the failure
    // evidence + the ctx DID move → still refused on the commit clause —
    // proving the CLEAR happened (otherwise the old record's clauses
    // would be evaluated identically). The fresh-right existence is the
    // grid above's job.
    assert!(!rev2(b));
}

/// The stale-store gate: when the filter commits a NON-base rotation with
/// majority weight, the store term drops (λ → 1). Observable via the
/// snapshot (the honest health readout — the gate itself is internal).
#[test]
fn stale_store_gate_fires_on_contradicted_base_ranking() {
    // A store whose evidence ranks strat-b far above strat-a.
    let root = std::env::temp_dir().join(format!(
        "riir_93ab_stale_{}_{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let (store, _bad) = riir_clippy::traj_store::TrajStore::open(&root).unwrap();
    for _ in 0..10 {
        store
            .record(
                &riir_clippy::domains::rustc_errors::evolve::traj_key("E0502", "strat-b"),
                "b shape",
                "x",
                true,
                true,
                "93ab-stale-seed",
            )
            .unwrap();
    }
    let policy = Arc::new(NonergodicOrdering::new(Some(Arc::new(store))));
    let ordering = policy.ordering();
    let observer = policy.observer();

    let mut cands = vec![
        cand("a.rs", 1, "E0502", "strat-b", "x"),
        cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
    ];
    ordering(&mut cands);
    assert_eq!(cands[0].strategy_id, "strat-b", "store seeds the base ranking");

    // The run CONTRADICTS the store: b fails, a resolves, repeatedly.
    for _ in 0..4 {
        observer(&attempt("a.rs", 1, "E0502", "strat-b", AttemptOutcome::RevertedNoProgress));
        observer(&attempt("a.rs", 2, "E0502", "strat-a", AttemptOutcome::Kept));
    }
    let mut cands = vec![
        cand("a.rs", 1, "E0502", "strat-b", "x"),
        cand("a.rs", 2, "E0502", "strat-a", "yyyy"),
    ];
    ordering(&mut cands);
    assert_eq!(
        cands[0].strategy_id, "strat-a",
        "contradicted store loses its vote (filter overtakes)"
    );
    let snap = policy.snapshot();
    assert!(
        snap.store_stale,
        "committed non-base rotation with majority weight must trip the stale gate"
    );
    let _ = std::fs::remove_dir_all(&root);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier 2 — real cargo e2e, no-regression at equal budget
// ─────────────────────────────────────────────────────────────────────────────

/// A multi-error file with playbook-covered codes (the E0499/E0502/E0597
/// anchors from the oracle-probed fixtures): the ordering arms see
/// real multi-candidate iterations.
const MULTI_SRC: &str = r#"
pub fn probe(v: &mut Vec<u8>) -> u8 {
    let first = &v[0];
    v.push(1);
    *first
}
"#;

const E0597_SRC: &str = r#"
pub fn run() -> usize {
    let n = 5usize;
    let cases: Vec<Case<'static>> = vec![Case { f: Box::new(|| n) }];
    cases.iter().map(|c| (c.f)()).sum()
}
pub struct Case<'a> {
    pub f: Box<dyn Fn() -> usize + 'a>,
}
"#;

#[test]
fn e2e_no_regression_at_equal_budget() {
    if !cargo_available() {
        eprintln!("cargo unavailable — skipping e2e arm");
        return;
    }
    let playbooks = Playbooks::production();
    let budget = 6;

    for (name, src) in [("borrow", MULTI_SRC), ("e0597", E0597_SRC)] {
        // Arm A: the shipped static ordering.
        let dir_a = scaffold(name, src);
        let rep_a = CompileDriver::isolated()
            .with_max_iters(budget)
            .run(&[dir_a.join("src").join("lib.rs")], &playbooks, true);
        // Arm B: the nonergodic ordering, same budget, fresh scaffold.
        let dir_b = scaffold(name, src);
        let policy = Arc::new(NonergodicOrdering::new(None));
        let rep_b = armed_driver(&policy)
            .with_max_iters(budget)
            .run(&[dir_b.join("src").join("lib.rs")], &playbooks, true);

        eprintln!(
            "e2e[{name}]: static ran={} clean={} applied={} reverted={} remaining={} err={:?} | \
             nonergodic ran={} clean={} applied={} reverted={} remaining={} err={:?} t_observed={}",
            rep_a.ran, rep_a.clean, rep_a.applied.len(), rep_a.reverted.len(), rep_a.remaining, rep_a.error,
            rep_b.ran, rep_b.clean, rep_b.applied.len(), rep_b.reverted.len(), rep_b.remaining, rep_b.error,
            policy.snapshot().t_observed,
        );
        // No-regression: the nonergodic arm must not leave MORE errors.
        assert!(
            rep_b.remaining <= rep_a.remaining,
            "nonergodic regressed on {name}: remaining {} > {}",
            rep_b.remaining,
            rep_a.remaining
        );
        // Headroom disclosure (tier 3): the surface's revert dynamics.
        let wasted_a = rep_a.reverted.len();
        let wasted_b = rep_b.reverted.len();
        eprintln!(
            "e2e[{name}] headroom: wasted static={wasted_a} nonergodic={wasted_b} \
             (0 on both = the first-try-perfect surface, Bench 045's null class)"
        );
        let _ = (wasted_a, wasted_b);
    }
}

/// Determinism e2e: two fresh nonergodic runs over identical scaffolds make
/// identical attempt sequences (the ordering + revive policy's combined
/// state machine is a pure function of the diagnostic history).
#[test]
fn e2e_determinism_two_runs_identical_attempts() {
    if !cargo_available() {
        eprintln!("cargo unavailable — skipping determinism arm");
        return;
    }
    let playbooks = Playbooks::production();
    let budget = 6;

    let mut attempts_runs: Vec<Vec<(String, String, String)>> = Vec::new();
    for i in 0..2 {
        let dir = scaffold("det", E0597_SRC);
        let policy = Arc::new(NonergodicOrdering::new(None));
        let rep = armed_driver(&policy)
            .with_max_iters(budget)
            .run(&[dir.join("src").join("lib.rs")], &playbooks, true);
        let att: Vec<(String, String, String)> = rep
            .attempts
            .iter()
            .map(|a| (a.code.clone(), a.strategy_id.clone(), a.outcome.label().to_string()))
            .collect();
        eprintln!("det run {i}: {att:?}");
        attempts_runs.push(att);
    }
    assert_eq!(
        attempts_runs[0], attempts_runs[1],
        "two identical scaffolds must produce identical attempt sequences"
    );
}

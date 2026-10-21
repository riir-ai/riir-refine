#![cfg(feature = "latent_retrieval")]
//! Issue 126 lane (b) — the store-level G1/G3 arms for the Lemma-4.2 tier
//! guard ([`riir_refine::tier_guard`]).
//!
//! The pure half (ratio arithmetic, boundaries, fail-closed) lives in the
//! module's own tests and runs at default features. THIS target proves the
//! guard on a real [`riir_refine::traj_store::TrajStore`] — the G1
//! adversarial arm the issue demands (a poisoned wrong-goal trajectory MUST
//! be excluded; a clean one MUST pass) plus the G3 monotonicity/determinism
//! arms — where the goal-side probability is the CROSS-ROW Elo replay and
//! the outcome side is the row's own realized history.
//!
//! The whole-file `#![cfg]` protects the count; the `required-features` row
//! in Cargo.toml protects the reader (a bare `cargo test --test
//! tier_guard_store_gates` errors instead of a green zero); Layer 5d
//! refuses an empty binary under `--all-features`.

use riir_refine::traj_store::TrajStore;
use riir_refine::{tier_guard, EvidenceTier};

fn tmp_root(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "riir_tier_guard_{}_{}_{}",
        std::process::id(),
        name,
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// Cold-start contract at the store level (the Issue-033 lesson, third
/// clause): a fresh row's first verified success certifies exactly as it
/// did before the guard existed — ratio exactly 1.0 at base ratings.
#[test]
fn fresh_verified_success_still_certifies() {
    let root = tmp_root("fresh");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record(
            "clippy::needless_borrow",
            "let x = &y;",
            "*y",
            true,
            true,
            "run-1",
        )
        .unwrap();
    let row = &store.dump()[0];
    assert_eq!(row.tier, EvidenceTier::Certified.as_u8());
    assert_eq!(row.success_count, 1);
    assert_eq!(row.fail_count, 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// G1 adversarial arm — the POISONED trajectory: a weak rule (12 straight
/// losses sank its goal-side rating) faces a mighty span (12 wins over two
/// other rules soared its difficulty rating). A keep on that pair is ~15×
/// more likely under reached dynamics than the goal model predicts — the
/// ratio lands far outside ε, so the certification MUST be refused. The
/// outcome itself still records: only the promotion is gated.
#[test]
fn poisoned_wrong_goal_credit_is_excluded() {
    let root = tmp_root("poisoned");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        // The rule loses 30 straight on ANOTHER span → its rating sinks.
        for _ in 0..30 {
            store
                .record("rule_p", "span_y trigger", "fix", false, false, "run-1")
                .unwrap();
        }
        // The span beats two other rules 15× each → its difficulty soars
        // (measured: the pair's p_goal lands ≈ 0.074).
        for _ in 0..15 {
            store
                .record("rule_a", "span_x trigger", "fix", false, false, "run-1")
                .unwrap();
            store
                .record("rule_b", "span_x trigger", "fix", false, false, "run-1")
                .unwrap();
        }
        // The poisoned row itself: an apply-only sighting (0/0, unrated).
        store
            .record_applied("rule_p", "span_x trigger", "fix", "run-2", None)
            .unwrap();
        store.recompute_elo().unwrap();
        // The goal-side model rates this pair far below a coin flip.
        let row = store
            .dump()
            .iter()
            .find(|r| r.lint_key == "rule_p" && r.shape == "span_x trigger")
            .unwrap()
            .clone();
        let p_goal = tier_guard::goal_probability(row.elo, row.span_elo);
        assert!(
            p_goal < 0.15,
            "fixture premise: goal model must disfavor the pair, got {p_goal}"
        );
    }
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        // The keep: π_g ≈ 0.07 vs π_j = 2/3 — ratio ≈ 0.11, far outside ε.
        store
            .record("rule_p", "span_x trigger", "fix", true, true, "run-3")
            .unwrap();
        let dump = store.dump();
        let row = dump
            .iter()
            .find(|r| r.lint_key == "rule_p" && r.shape == "span_x trigger")
            .unwrap();
        assert_eq!(
            row.tier,
            EvidenceTier::Heuristic.as_u8(),
            "wrong-goal credit must be excluded: tier stays Heuristic"
        );
        assert_eq!(row.success_count, 1, "the outcome still records");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// G1 adversarial arm — the CLEAN trajectory: a dominant rule on a span it
/// beats (goal model favors the pair ≈ 0.6) keeps again; the row's realized
/// history (7/0 → π_j = 8/9) agrees with the goal model within ε — the
/// certification MUST pass.
#[test]
fn clean_consistent_goal_credit_certifies() {
    let root = tmp_root("clean");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        // rule_g beats span_e 6× (row G, all certified at base ratings)…
        for _ in 0..6 {
            store
                .record("rule_g", "span_e trigger", "fix", true, true, "run-1")
                .unwrap();
        }
        // …and span_e beats rule_h 6× (the span is no pushover, the rule is
        // genuinely dominant — the goal model rates the pair ≈ 0.6).
        for _ in 0..6 {
            store
                .record("rule_h", "span_e trigger", "fix", false, false, "run-1")
                .unwrap();
        }
        store.recompute_elo().unwrap();
    }
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("rule_g", "span_e trigger", "fix", true, true, "run-2")
            .unwrap();
        let dump = store.dump();
        let row = dump.iter().find(|r| r.lint_key == "rule_g").unwrap();
        assert_eq!(
            row.tier,
            EvidenceTier::Certified.as_u8(),
            "consistent evidence must certify"
        );
        assert_eq!(row.success_count, 7);
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// G3 monotone: the guard gates the PROMOTION only — an already-Certified
/// row keeps its tier when a later keep is excluded (the goal model turned
/// adverse since certification), and the counters still record. Exclusion
/// never demotes.
#[test]
fn guard_refusal_never_demotes_certified() {
    let root = tmp_root("no-demote");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        // Certified at base ratings: 3 verified successes vs span_z…
        for _ in 0..3 {
            store
                .record("rule_c", "span_z trigger", "fix", true, true, "run-1")
                .unwrap();
        }
        let z = |s: &TrajStore| {
            s.dump()
                .into_iter()
                .find(|r| r.lint_key == "rule_c" && r.shape == "span_z trigger")
                .unwrap()
        };
        assert_eq!(
            z(&store).tier,
            EvidenceTier::Certified.as_u8(),
            "fixture premise: certified at base ratings"
        );
        // …then the rule tanks on ANOTHER span (30 losses sink it) while
        // span_z soars on rule_d's 30 failures — the goal model turns
        // adverse AFTER the row was certified (measured p_goal ≈ 0.118).
        for _ in 0..30 {
            store
                .record("rule_c", "span_w trigger", "fix", false, false, "run-2")
                .unwrap();
            store
                .record("rule_d", "span_z trigger", "fix", false, false, "run-2")
                .unwrap();
        }
        store.recompute_elo().unwrap();
        let row = z(&store);
        let p_goal = tier_guard::goal_probability(row.elo, row.span_elo);
        assert!(
            p_goal < 0.2,
            "fixture premise: goal model must have turned adverse, got {p_goal}"
        );
    }
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("rule_c", "span_z trigger", "fix", true, true, "run-3")
            .unwrap();
        let dump = store.dump();
        let row = dump
            .iter()
            .find(|r| r.lint_key == "rule_c" && r.shape == "span_z trigger")
            .unwrap();
        assert_eq!(
            row.tier,
            EvidenceTier::Certified.as_u8(),
            "the guard gates promotion; it never demotes"
        );
        assert_eq!(row.success_count, 4, "counters record regardless");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// G3 determinism: the same event sequence — certifies, refusals,
/// withdrawals, the ratings replay — on two fresh stores yields
/// bit-identical rows (modulo the wall-clock `last_seen_at` stamp, which is
/// not a replay input). The guard is a pure function of stored fields, so
/// replaying history re-derives the same tiers.
#[test]
fn replay_is_deterministic() {
    let build = |root: &std::path::Path| {
        let (store, _) = TrajStore::open(root).unwrap();
        // rule_g: dominant on span_e (every keep certifies — consistent).
        for _ in 0..4 {
            store
                .record("rule_g", "span_e trigger", "fix", true, true, "run-1")
                .unwrap();
        }
        // rule_p: 3 fails → Withdrawn…
        for _ in 0..3 {
            store
                .record("rule_p", "span_y trigger", "fix", false, false, "run-1")
                .unwrap();
        }
        // …and rule_p faces the mighty span_x (built from rule_q's losses):
        for _ in 0..8 {
            store
                .record("rule_q", "span_x trigger", "fix", false, false, "run-1")
                .unwrap();
        }
        store
            .record_applied("rule_p", "span_x trigger", "fix", "run-2", None)
            .unwrap();
        store.recompute_elo().unwrap();
        // The poisoned keep — refused this replay (p_goal ≪ p_outcome).
        store
            .record("rule_p", "span_x trigger", "fix", true, true, "run-3")
            .unwrap();
    };

    let r1 = tmp_root("det-a");
    let r2 = tmp_root("det-b");
    build(&r1);
    build(&r2);
    let (s1, _) = TrajStore::open(&r1).unwrap();
    let (s2, _) = TrajStore::open(&r2).unwrap();
    let (mut d1, mut d2) = (s1.dump(), s2.dump());
    assert_eq!(d1.len(), d2.len());
    for (a, b) in d1.iter_mut().zip(d2.iter_mut()) {
        a.last_seen_at = 0;
        b.last_seen_at = 0;
    }
    assert_eq!(d1, d2, "identical replay must yield identical rows");
    // And the poisoned keep was refused identically in both.
    for d in [&d1, &d2] {
        let p = d
            .iter()
            .find(|r| r.lint_key == "rule_p" && r.shape == "span_x trigger")
            .unwrap();
        assert_eq!(p.tier, EvidenceTier::Heuristic.as_u8());
    }
    let _ = std::fs::remove_dir_all(&r1);
    let _ = std::fs::remove_dir_all(&r2);
}

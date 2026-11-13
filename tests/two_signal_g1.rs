//! Issue 232 T5 G1 — the two-signal composition's preregistered fixture
//! (Research 495, L5's law). The law: a lone held-out window never
//! reverses an armed lane alone; a corroborated one (fresh window + a
//! FAILED arming premise — T3's own consult, no new threshold) commits
//! through the one write path; probes evaluate on side state.
//!
//! Green-zero fence: this file is `#![cfg]`-gated AND carries the
//! `required-features` row — the whole-file gate pins the COUNT (a run
//! without the features compiles an empty binary and would print
//! `ok. 0 passed`), the required-features row protects the READER (the
//! target is skipped with a named reason instead of running empty).
#![cfg(all(
    feature = "two_signal_composition",
    feature = "multiplicity_gate",
    feature = "consumed_window_ledger",
    feature = "latent_retrieval"
))]

use riir_refine::consumed_window::WindowLedger;
use riir_refine::multiplicity_gate::MultiplicityGate;
use riir_refine::two_signal::{
    ArmingEvidence, Composition, FailureFacts, SideState, UpholdReason, WindowEvidence, commit,
    compose,
};

/// A reuse-dominant histogram ([6,2,1,1]) measuring knee 6 — the fixture's
/// standing gate (same shape as T3's store fixture).
fn measured_gate() -> MultiplicityGate {
    MultiplicityGate::measure(&[6, 2, 1, 1]).expect("[6,2,1,1] is reuse-dominant")
}

#[test]
fn g1_lone_window_reversal_of_an_armed_lane_is_upheld() {
    // THE G1 ROW: the lane is armed (consult(6) — at the measured knee —
    // holds), a FRESH window proposes reversal → UPHOLD (lone-window),
    // and the write path stays shut: the serving artifact is
    // byte-identical, the apply closure never runs.
    let g = measured_gate();
    let arming = ArmingEvidence::from_consult(&g, 6);
    let window = WindowEvidence::Fresh {
        proposes_reversal: true,
    };
    let d = compose(&arming, &window);
    assert_eq!(d, Composition::Uphold(UpholdReason::LoneProposal));
    assert!(d.line().contains("verdict=UPHOLD"), "{}", d.line());
    assert!(d.line().contains("reason=lone-window"), "{}", d.line());

    // The serving artifact: probes ran, the decision was not a commit —
    // nothing moved.
    let mut serving = vec![100u64, 200, 300];
    let applied = commit(&mut serving, &d, |s| {
        s.clear();
        s.push(0);
    });
    assert!(!applied, "a lone-window verdict never opens the write path");
    assert_eq!(serving, vec![100, 200, 300], "serving byte-identical");
}

#[test]
fn g1_corroborated_reversal_proceeds_through_the_one_write_path() {
    // THE G1 ROW (the corroborated half): the same measured gate, the
    // unit's counter BELOW the knee (consult(2) — the premise failed),
    // a fresh window proposes → COMMIT; the artifact mutates exactly
    // through the one path and the decision names both signals.
    let g = measured_gate();
    let arming = ArmingEvidence::from_consult(&g, 2);
    let window = WindowEvidence::Fresh {
        proposes_reversal: true,
    };
    let d = compose(&arming, &window);
    let facts = match &d {
        Composition::Commit(f) => f.clone(),
        other => panic!("expected COMMIT, got {other:?}"),
    };
    assert!(facts.window_fresh);
    match &facts.failure {
        FailureFacts::BelowKnee {
            gate,
            multiplicity,
            knee,
        } => {
            assert_eq!((*multiplicity, *knee), (2, 6));
            assert!(
                *gate < 0.5,
                "below the knee the gate sits under 0.5: {gate}"
            );
        }
        other => panic!("expected below-knee, got {other:?}"),
    }
    assert!(d.line().contains("verdict=COMMIT"), "{}", d.line());
    assert!(d.line().contains("signals=both"), "{}", d.line());
    assert!(d.line().contains("premise=below-knee"), "{}", d.line());

    // The one write path fires; the mutation lands.
    let mut serving = vec![7u64];
    let applied = commit(&mut serving, &d, |s| s.push(8));
    assert!(applied);
    assert_eq!(serving, vec![7, 8]);
}

#[test]
fn g1_side_state_probes_never_touch_the_serving_artifact() {
    // T³'s φ is throwaway: the candidate evaluation runs on the fork,
    // the serving artifact is unreachable from the probe path, and even
    // a non-commit decision handed to `commit` with a closure leaves
    // everything alone.
    let g = measured_gate();
    let arming = ArmingEvidence::from_consult(&g, 6);
    let d = compose(
        &arming,
        &WindowEvidence::Fresh {
            proposes_reversal: true,
        },
    );
    let serving = vec![1u64, 2, 3];
    let mut side = SideState::fork(&serving);
    side.probe(|s| {
        // The candidate's scratch evaluation — mutations land here only.
        s.push(4);
        s[0] = 42;
    });
    assert_eq!(side.state(), &vec![42, 2, 3, 4]);
    assert_eq!(serving, vec![1, 2, 3], "the serving artifact never moved");
    let _ = side.into_inner();

    // The write-path exclusivity, again, from the decision side: the
    // lone-window verdict carries no mutation.
    let mut serving = serving.clone();
    assert!(!commit(&mut serving, &d, |s| s.clear()));
    assert_eq!(serving, vec![1, 2, 3]);
}

#[test]
fn g1_a_consumed_window_decides_nothing_even_corroborated() {
    // The poisoned-window law rides the REAL ledger: the first consult
    // is Fresh (the composition can adjudicate), the second is Consumed
    // (the composition cannot — UNDECIDABLE carried forward), and the
    // ledger's own report line still says UNDECIDABLE, never a refusal.
    let mut ledger = WindowLedger::new();
    let digest = "a1b2c3d4e5f60718293a4b5c6d7e8f90";
    let first = ledger.consult(digest, "g1-selection", 1_000);
    assert_eq!(
        WindowEvidence::from_read(&first, true),
        WindowEvidence::Fresh {
            proposes_reversal: true
        }
    );
    let second = ledger.consult(digest, "g1-selection", 2_000);
    let window = WindowEvidence::from_read(&second, true);
    assert_eq!(
        window,
        WindowEvidence::Consumed {
            reuse: 1,
            proposes_reversal: true
        }
    );

    // Even against a FAILED arming premise the consumed window decides
    // nothing — the poison is independent of the arming side.
    let g = measured_gate();
    let arming = ArmingEvidence::from_consult(&g, 2);
    let d = compose(&arming, &window);
    assert_eq!(d, Composition::WindowUndecidable { reuse: 1 });
    let mut serving = vec![9u64];
    assert!(!commit(&mut serving, &d, |s| s.clear()));
    assert_eq!(serving, vec![9]);
    assert!(d.line().contains("verdict=UNDECIDABLE"), "{}", d.line());
    assert!(
        ledger.report_line(digest, &second).contains("UNDECIDABLE"),
        "{}",
        ledger.report_line(digest, &second)
    );
}

#[test]
fn g1_store_surface_composes_end_to_end() {
    // The REAL surface shape: a trajectory store seeded [6,2,1,1] →
    // measured knee 6; the composition reads the arming consult from the
    // measured gate + the window state from the REAL ledger (READ-ONLY:
    // the report never records) and composes. With the hypothetical
    // proposal disclosed, a holding consult upholds lone-window on the
    // REAL numbers; with no proposal (the standing real-world state —
    // nothing proposes reversal today) the verdict is UPHOLD/no-proposal
    // whatever the halves say.
    let root = std::env::temp_dir().join(format!("two_signal_g1_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root created");

    let (store, bad) = riir_refine::traj_store::TrajStore::open(&root).expect("open store");
    assert_eq!(bad, 0);
    let plan: [(&str, usize); 4] = [("rule_a", 6), ("rule_b", 2), ("rule_c", 1), ("rule_d", 1)];
    for (i, (rule, n)) in plan.iter().enumerate() {
        for k in 0..*n {
            let win = (k + i) % 2 == 0;
            store
                .record(rule, &format!("shape_{i}"), "fix()", win, win, "g1-seed")
                .expect("record");
        }
    }
    let digest = store.consumed_window_digest();
    drop(store);

    // The arming half from the measured store surface.
    let measured =
        riir_refine::multiplicity_gate::measure_store_surface(&root).expect("the store opens");
    let g = measured.verdict.expect("[6,2,1,1] is reuse-dominant");
    assert_eq!(g.knee(), 6);
    let arming = ArmingEvidence::from_consult(&g, g.knee());

    // The window half, READ-ONLY (reuse_of, never consult): the ledger
    // file is absent → reuse 0 → fresh.
    let ledger_path = root.join(".refine").join("window_ledger.json");
    let ledger = WindowLedger::load(&ledger_path);
    let window = WindowEvidence::from_reuse(ledger.reuse_of(&digest), true);
    assert_eq!(
        window,
        WindowEvidence::Fresh {
            proposes_reversal: true
        }
    );

    // The hypothetical proposal against the holding consult → the L5
    // row on the REAL numbers.
    let d = compose(&arming, &window);
    assert_eq!(d, Composition::Uphold(UpholdReason::LoneProposal));
    let text = d.render();
    assert!(text.contains("verdict: UPHOLD"), "{text}");
    assert!(text.contains("lone-window"), "{text}");

    // And the standing state (no proposal — what the real store carries
    // today): UPHOLD/no-proposal, pinned byte-exact.
    let neutral = WindowEvidence::from_reuse(ledger.reuse_of(&digest), false);
    let standing = compose(&arming, &neutral);
    assert_eq!(standing, Composition::Uphold(UpholdReason::NoProposal));
    assert_eq!(
        standing.line(),
        "[composition] verdict=UPHOLD reason=no-proposal — nothing proposed; the lane stands (I232 T5, report-only)"
    );

    // The fixture never recorded a consumption: the ledger file stays
    // absent (the report lane never saves).
    assert!(!ledger_path.exists(), "the report lane never records");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn g1_compose_is_pure_and_threshold_free() {
    // Determinism: the same evidence composes byte-identically. The
    // corroboration boolean IS T3's own consult — flipping ANY single
    // conjunct away from the committing shape (below-knee + fresh +
    // proposal) never commits; the full 2×2×2 table holds exactly one
    // committing row.
    let g = measured_gate();
    let a_hold = ArmingEvidence::from_consult(&g, 6);
    let a_fail = ArmingEvidence::from_consult(&g, 2);
    let w_fresh_yes = WindowEvidence::Fresh {
        proposes_reversal: true,
    };
    let w_fresh_no = WindowEvidence::Fresh {
        proposes_reversal: false,
    };
    let w_consumed_yes = WindowEvidence::Consumed {
        reuse: 2,
        proposes_reversal: true,
    };
    let w_consumed_no = WindowEvidence::Consumed {
        reuse: 2,
        proposes_reversal: false,
    };

    let d1 = compose(&a_fail, &w_fresh_yes);
    let d2 = compose(&a_fail, &w_fresh_yes);
    assert_eq!(d1, d2);
    assert_eq!(d1.line(), d2.line());
    assert_eq!(d1.render(), d2.render());

    let rows = [
        (&a_hold, &w_fresh_yes),
        (&a_hold, &w_fresh_no),
        (&a_hold, &w_consumed_yes),
        (&a_hold, &w_consumed_no),
        (&a_fail, &w_fresh_yes),
        (&a_fail, &w_fresh_no),
        (&a_fail, &w_consumed_yes),
        (&a_fail, &w_consumed_no),
    ];
    let commits = rows
        .iter()
        .filter(|(a, w)| matches!(compose(a, w), Composition::Commit(_)))
        .count();
    assert_eq!(commits, 1, "exactly one committing row: fail×fresh×yes");
}

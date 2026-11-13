#![cfg(feature = "multiplicity_gate")]
//! Issue 232 T3 — the G1 fixture (Research 495, the T³ holdout-epistemology
//! extraction): the multiplicity-gated arming predicate's truth table.
//!
//! Pre-registered predicates (the issue's proof-path ledger):
//! 1. **Truth table:** multiplicity < knee → byte-identical unarmed
//!    behavior (the consult is UNARMED, the gate < 0.5, and the render
//!    carries no armed vocabulary); ≥ knee → armed; AT the knee the gate
//!    is exactly 0.5 (the boolean IS the sigmoid's own half — no second
//!    threshold anywhere).
//! 2. **The knee is measured per surface, never imported:** different
//!    surfaces measure different knees from their own histograms; the
//!    measurement is integer-exact and input-order independent; exact-half
//!    ties arm LATE (the larger knee — conservative).
//! 3. **The fence (the boundary result):** a 1-pass surface (repeat
//!    encounters < first — the chain/quorum shape) NEVER arms, at any
//!    multiplicity; the measurement refuses loud naming the law.
//! 4. **Sigmoid, never softmax:** a unit's gate reads its own counter
//!    against the knee — adding or reshaping OTHER units cannot move it
//!    (the same-knee/different-shape bit-identity row).
//! 5. **Blind + malformed refuse loud** (zero units / zero-count rows) —
//!    never a green zero.
//! 6. **The real surface end to end:** the traj store's per-rule
//!    encounter counts fold into the histogram and measure to the
//!    expected knee; renders carry no refusal vocabulary.

use riir_refine::multiplicity_gate::{Consult, KneeRefusal, MultiplicityGate};

#[test]
fn g1_truth_table_below_knee_unarmed_above_armed_at_knee_exact_half() {
    // [6,2,1,1]: total 10, units 4 — reuse-dominant (10 ≥ 8); the 6-count
    // unit alone carries 6/10 ≥ half → knee 6.
    let g = MultiplicityGate::measure(&[6, 2, 1, 1]).expect("reuse-dominant");
    let knee = g.knee();
    assert_eq!(knee, 6);
    for m in 0..=(knee + 3) {
        let gate = g.gate(m);
        let consult = g.consult(m);
        if m < knee {
            assert_eq!(consult, Consult::Unarmed, "m={m} < knee must be unarmed");
            assert!(gate < 0.5, "unarmed gate must sit below the half at m={m}");
            let line = g.consult_line("s", "u", m);
            assert!(
                !line.contains("consult=ARMED"),
                "byte-identical unarmed behavior: no armed vocabulary below the knee: {line}"
            );
            assert!(line.contains("consult=UNARMED"));
        } else {
            assert_eq!(consult, Consult::Armed, "m={m} ≥ knee must be armed");
            assert!(
                gate > 0.5 || m == knee,
                "armed gate above the half at m={m}"
            );
            assert!(g.consult_line("s", "u", m).contains("consult=ARMED"));
        }
        // The boolean IS the sigmoid's own half — pinned at EVERY m.
        assert_eq!(consult == Consult::Armed, gate >= 0.5, "m={m}");
    }
    // AT the knee: exactly one half — exp(0), bit-exact.
    assert_eq!(g.gate(knee).to_bits(), 0.5f64.to_bits());
}

#[test]
fn g1_knee_is_measured_per_surface_never_imported() {
    // A heavy-tailed surface: only the 10-count unit arms (it alone
    // carries ≥ half the mass).
    let heavy = MultiplicityGate::measure(&[1, 1, 1, 1, 10]).expect("reuse-dominant");
    assert_eq!(heavy.knee(), 10);
    assert_eq!(heavy.consult(9), Consult::Unarmed);
    assert_eq!(heavy.consult(10), Consult::Armed);
    // A uniform 2× surface: knee 2 arms surface-wide (the T³ 4-pass
    // shape's cousin — the measured answer for THAT surface, not an
    // imported constant).
    let uniform = MultiplicityGate::measure(&[2, 2, 2]).expect("reuse-dominant");
    assert_eq!(uniform.knee(), 2);
    assert_eq!(uniform.consult(2), Consult::Armed);
    // Order independence: the histogram is a SET of per-unit counts.
    let a = MultiplicityGate::measure(&[6, 2, 1, 1]).expect("reuse-dominant");
    let b = MultiplicityGate::measure(&[1, 6, 1, 2]).expect("reuse-dominant");
    assert_eq!(a.knee(), b.knee());
}

#[test]
fn g1_exact_half_ties_arm_late() {
    // [2,2,4]: mass_above(3) = 4 and mass_above(4) = 4 — both satisfy
    // 2·mass ≥ total(8). The tie resolves to the LARGER knee (arm late).
    let g = MultiplicityGate::measure(&[2, 2, 4]).expect("reuse-dominant");
    assert_eq!(g.knee(), 4);
    assert_eq!(g.consult(3), Consult::Unarmed);
    assert_eq!(g.consult(4), Consult::Armed);
}

#[test]
fn g1_fence_one_pass_surfaces_never_arm_at_any_multiplicity() {
    // The boundary result: quorum/chain surfaces are 1-pass by
    // construction — L1 predicts holdout machinery buys nothing there,
    // and the fence is the same law that arms reuse-dominant surfaces.
    // Structural: a refused measurement yields NO gate value at all, so
    // there is no bypass constructor that could arm one.
    let big_one_pass: Vec<u64> = vec![1; 500];
    let refusal = MultiplicityGate::measure(&big_one_pass)
        .expect_err("a 1-pass surface must refuse, at any size");
    assert_eq!(
        refusal,
        KneeRefusal::OnePassSurface {
            units: 500,
            total: 500
        }
    );
    let line = refusal.line();
    assert!(line.contains("ONE_PASS"));
    assert!(line.contains("never arm"));
    assert!(
        line.contains("chain/quorum"),
        "the fence names the boundary: {line}"
    );
    // Even a MIXED surface just past the all-first boundary fences:
    // [1,1,1,2] has repeats (1) < firsts (4).
    assert_eq!(
        MultiplicityGate::measure(&[1, 1, 1, 2]),
        Err(KneeRefusal::OnePassSurface { units: 4, total: 5 })
    );
    // The reuse-dominance boundary itself is exact: total == 2·units
    // (repeats == firsts) PASSES the fence.
    assert!(MultiplicityGate::measure(&[1, 3]).is_ok());
    assert!(MultiplicityGate::measure(&[2, 2]).is_ok());
}

#[test]
fn g1_gate_is_a_counter_sigmoid_never_softmax() {
    // Same knee, different surrounding distributions: a unit at 7 reads
    // the same gate BITS under both — other units cannot move it.
    let g1 = MultiplicityGate::measure(&[4, 4, 1, 1, 1]).expect("reuse-dominant");
    let g2 = MultiplicityGate::measure(&[4, 4, 1]).expect("reuse-dominant");
    assert_eq!(g1.knee(), 4);
    assert_eq!(g2.knee(), 4);
    assert_eq!(g1.gate(7).to_bits(), g2.gate(7).to_bits());
    assert_eq!(g1.gate(0).to_bits(), g2.gate(0).to_bits());
    // Monotone non-decreasing in the counter, bounded [0,1].
    let mut prev = 0.0;
    for m in 0..=200u64 {
        let v = g1.gate(m);
        assert!(v >= prev, "monotone at m={m}");
        assert!((0.0..=1.0).contains(&v));
        prev = v;
    }
}

#[test]
fn g1_blind_and_malformed_refuse_loud_never_green_zero() {
    assert_eq!(MultiplicityGate::measure(&[]), Err(KneeRefusal::Blind));
    assert!(!KneeRefusal::Blind.line().is_empty());
    // A zero-count row is MALFORMED, not blind — discriminated, loud.
    assert_eq!(
        MultiplicityGate::measure(&[3, 0, 5]),
        Err(KneeRefusal::MalformedZero { units: 1 })
    );
    assert_eq!(
        MultiplicityGate::measure(&[0, 0]),
        Err(KneeRefusal::MalformedZero { units: 2 })
    );
    assert!(
        KneeRefusal::MalformedZero { units: 2 }
            .line()
            .contains("zero encounters")
    );
}

#[test]
fn g1_store_surface_measures_the_fixer_encounters_end_to_end() {
    // The shared_temp_path law: a per-process temp root — a concurrent
    // process running this same test must never share the directory.
    let root = std::env::temp_dir().join(format!("multiplicity_g1_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root created");

    // A fresh store records zero units: blind, never a green zero.
    let blind =
        riir_refine::multiplicity_gate::measure_store_surface(&root).expect("the store opens");
    assert_eq!(blind.verdict, Err(KneeRefusal::Blind));
    assert_eq!(blind.excluded_zero_outcome_rows, 0);

    // Seed [6,2,1,1]: rule_a ×6 outcomes, rule_b ×2, rule_c ×1,
    // rule_d ×1 — one row per (lint_key, shape), counters incremented on
    // repeat (the store's upsert law). Deterministic win pattern, never 3
    // consecutive fails.
    let (store, bad) = riir_refine::traj_store::TrajStore::open(&root).expect("open store");
    assert_eq!(bad, 0, "fresh store has no corrupt rows");
    let plan: [(&str, usize); 4] = [("rule_a", 6), ("rule_b", 2), ("rule_c", 1), ("rule_d", 1)];
    for (i, (rule, n)) in plan.iter().enumerate() {
        for k in 0..*n {
            let win = (k + i) % 2 == 0;
            store
                .record(rule, &format!("shape_{i}"), "fix()", win, win, "g1-seed")
                .expect("record");
        }
    }
    drop(store);

    let measured =
        riir_refine::multiplicity_gate::measure_store_surface(&root).expect("the store opens");
    assert_eq!(measured.excluded_zero_outcome_rows, 0);
    let g = measured
        .verdict
        .expect("4 units / 10 encounters — reuse-dominant");
    assert_eq!(g.knee(), 6);
    assert_eq!(g.units(), 4);
    assert_eq!(g.total_encounters(), 10);
    assert_eq!(g.consult(6), Consult::Armed);
    assert_eq!(g.consult(2), Consult::Unarmed);

    // The render: the measured verdict, never refusal vocabulary.
    let text = g.render(riir_refine::multiplicity_gate::SURFACE_NAME);
    assert!(text.contains("REUSE_DOMINANT"));
    assert!(text.contains("knee measured 6"));
    assert!(text.contains("units: 4 — encounters: 10"));
    assert!(!text.to_ascii_lowercase().contains("refus"));
    assert!(
        g.consult_line("traj-store", "rule_a", 6)
            .contains("consult=ARMED")
    );

    let _ = std::fs::remove_dir_all(&root);
}

//! Issue 035 T1 — the premise probe as a standing test.
//!
//! Plan 019 was filed on the claim that rustc emits NO machine-applicable
//! suggestions for the borrowck family, and that `cargo fix` is structurally
//! blind there. Measured on rustc 1.93.0 the claim is false on 5 of the 8
//! codes probed (`E0382`, `E0505`, `E0507`, `E0596`, `E0373`).
//!
//! These tests exist so that fact cannot silently rot in either direction:
//!
//! - A toolchain bump that starts auto-fixing `E0597`/`E0499`/`E0502` deletes
//!   part of the product's uncontested head. That must FAIL here, loudly, not
//!   be discovered later by a user.
//! - A fixture that stops provoking the code it claims (the E0597-vs-E0373
//!   trap that put a redundant template at the top of the plan's playbook)
//!   must also fail.
//!
//! The live-toolchain tests are gated behind `--features rustc_errors` and
//! SKIP (not fail) when cargo is unavailable. They spawn `cargo check` +
//! `cargo fix` per case, so they are slow by nature; run them explicitly:
//!
//! ```bash
//! cargo test --features rustc_errors --test premise_probe -- --nocapture
//! ```

#![cfg(feature = "rustc_errors")]

use riir_clippy::domains::rustc_errors::probe::{PremiseProbe, ProbeOutcome, PROBE_CASES};

/// Codes measured to have NO machine-applicable suggestion — the territory
/// Plan 019 can own outright.
const UNCONTESTED: &[&str] = &["E0597", "E0499", "E0502"];

/// Codes rustc already auto-fixes. `E0373` is on this list because the
/// `Box<dyn Fn>` closure shape Plan 019 filed as "E0597 template 1" is
/// diagnosed E0373 and fixed by `cargo fix` unprompted.
const CONTESTED: &[&str] = &["E0382", "E0505", "E0507", "E0596", "E0373"];

fn run_probe() -> Option<Vec<ProbeOutcome>> {
    let probe = PremiseProbe::new();
    if !probe.is_available() {
        eprintln!("SKIP: cargo not on PATH — the premise probe needs the live toolchain");
        return None;
    }
    let mut outcomes = Vec::new();
    for (case, result) in PROBE_CASES.iter().zip(probe.probe_all()) {
        match result {
            Ok(o) => outcomes.push(o),
            Err(e) => panic!("probe {} failed to run: {e}", case.name),
        }
    }
    probe.cleanup();
    Some(outcomes)
}

/// Corpus integrity: every fixture provokes the code it declares.
///
/// This is the test that would have caught the plan's mislabeled flagship
/// template before it was written.
#[test]
fn every_fixture_provokes_its_declared_code() {
    let Some(outcomes) = run_probe() else { return };
    for o in &outcomes {
        assert!(
            o.code_as_declared(),
            "{}: declared {} but rustc emitted {:?} — fixture is mis-labeled",
            o.name,
            o.expect_code,
            o.observed_codes
        );
    }
}

/// The premise itself, pinned per code.
#[test]
fn machine_applicable_matches_the_measured_premise() {
    let Some(outcomes) = run_probe() else { return };
    for (case, o) in PROBE_CASES.iter().zip(&outcomes) {
        let has_auto = o.machine_applicable > 0;
        assert_eq!(
            has_auto,
            case.expect_machine_applicable,
            "{} ({}): expected machine_applicable={} but measured {} suggestion span(s), \
             {} machine-applicable {:?}. \
             If rustc changed, RE-MEASURE and update Issue 035's table — do not \
             relax this assertion; the contested/uncontested split is the product claim.",
            o.name,
            o.expect_code,
            case.expect_machine_applicable,
            o.suggestion_spans,
            o.machine_applicable,
            o.replacements
        );
    }
}

/// The uncontested three: no auto-fix AND `cargo fix` leaves the source alone.
///
/// Both halves are asserted because they can diverge — a `MaybeIncorrect`
/// suggestion appears in the JSON but is never applied, and a code can be
/// incidentally repaired by a suggestion attached to a different diagnostic.
#[test]
fn uncontested_codes_are_untouched_by_cargo_fix() {
    let Some(outcomes) = run_probe() else { return };
    for code in UNCONTESTED {
        let matching: Vec<&ProbeOutcome> =
            outcomes.iter().filter(|o| o.expect_code == *code).collect();
        assert!(!matching.is_empty(), "no fixture covers {code}");
        for o in matching {
            assert!(
                o.is_uncontested(),
                "{code} is no longer uncontested ({}): machine_applicable={}, \
                 cargo_fix_changed={}. rustc gained a fix for a code Plan 019 \
                 counted as its own — the addressable head just shrank.",
                o.name,
                o.machine_applicable,
                o.cargo_fix_changed
            );
        }
    }
}

/// The contested five: `cargo fix` really does repair these unprompted.
#[test]
fn contested_codes_are_repaired_by_cargo_fix_alone() {
    let Some(outcomes) = run_probe() else { return };
    for code in CONTESTED {
        let matching: Vec<&ProbeOutcome> =
            outcomes.iter().filter(|o| o.expect_code == *code).collect();
        assert!(!matching.is_empty(), "no fixture covers {code}");
        for o in matching {
            assert!(
                o.machine_applicable > 0,
                "{code} ({}) was measured contested but now offers no \
                 machine-applicable suggestion",
                o.name
            );
            assert!(
                o.cargo_fix_changed,
                "{code} ({}) offers a machine-applicable suggestion that \
                 `cargo fix --broken-code` did not apply",
                o.name
            );
            assert!(
                !o.codes_after_fix.iter().any(|c| c == code),
                "{code} ({}) survived `cargo fix`: {:?}",
                o.name,
                o.codes_after_fix
            );
        }
    }
}

/// The split is exhaustive and disjoint — no probed code is unclassified.
#[test]
fn every_probed_code_is_classified_exactly_once() {
    for case in PROBE_CASES {
        let in_unc = UNCONTESTED.contains(&case.expect_code);
        let in_con = CONTESTED.contains(&case.expect_code);
        assert!(
            in_unc ^ in_con,
            "{} ({}) must be in exactly one territory list (uncontested={in_unc}, \
             contested={in_con})",
            case.name,
            case.expect_code
        );
        // The territory list and the fixture's own expectation must agree.
        assert_eq!(
            in_con, case.expect_machine_applicable,
            "{}: territory list and fixture expectation disagree",
            case.name
        );
    }
}

/// Plan 019's playbook template 1 (`move` closure) is `cargo fix`'s job.
///
/// Pinned separately from the table because it is the specific finding that
/// demoted the plan's flagship template: the shape is E0373, not E0597.
#[test]
fn closure_outlives_shape_is_e0373_and_rustc_fixes_it() {
    let Some(outcomes) = run_probe() else { return };
    let o = outcomes
        .iter()
        .find(|o| o.name == "e0373_closure_outlives")
        .expect("closure fixture present");
    assert!(
        o.observed_codes.iter().any(|c| c == "E0373"),
        "the Box<dyn Fn> closure shape should be E0373, got {:?}",
        o.observed_codes
    );
    assert!(
        !o.observed_codes.iter().any(|c| c == "E0597"),
        "it is NOT E0597 — that mislabel is what put a redundant template \
         first in the playbook; got {:?}",
        o.observed_codes
    );
    let fixed = o.fixed_source.as_deref().unwrap_or("");
    assert!(
        fixed.contains("move ||"),
        "cargo fix should insert `move`; got {fixed:?}"
    );
}

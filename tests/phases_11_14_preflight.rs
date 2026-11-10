//! Plan 005 Phases 11/12/14 — substrate-first pre-flight.
//!
//! Five plan tasks in this file's scope have already turned out to rest on a
//! premise that no longer held (T3.5.1, T7.1, T7.3, Phase 9's confidence
//! field, T9.4's `clr_vote`). This measures the decisive fact for each
//! remaining phase before any of them is built.
//!
//! - **P11** — Phase 11's rescoped gate is "imagination rejects ≥30% of
//!   *would-fail-L2* fixes". If L2 never rejects, that set is empty and the
//!   gate is undefined. T9.4 already saw L2 accept 3/3 drafts; this checks it
//!   on real proposer output, which is what `--fix` actually feeds L2.
//! - **P12** — Phase 12 classifies fix *trajectories* into archetypes.
//!   `SweTrajectoryFreezer::freeze_attempt` takes `&[&[f32]]`: a SEQUENCE of
//!   states. So the question is whether the healer produces multi-step
//!   attempts at all, or whether every fix is one shot.
//! - **P14** — T14.2 proposes replacing `LatentSkillEvolution`'s
//!   `HashMap<String, Direction>` with `HebbianKernelMemory` for "O(1)
//!   associative recall vs O(N) hash scan". A `HashMap` lookup is already
//!   O(1); this checks the premise empirically rather than by assertion.
//!
//! ```bash
//! cargo test --release --features fix_verify --test phases_11_14_preflight -- --nocapture
//! ```

#![cfg(feature = "clippy_lints")]

use std::time::Instant;

use riir_refine::domain::{HealDomain, HealEntry};
use riir_refine::domains::clippy_lints::ClippyLintDomain;
use riir_refine::domains::clippy_lints::proposers::{FIXABLE_LINTS, propose_fix};
use riir_refine::pruner::RustAnalyzerPruner;

/// P11 — does L2 ever reject what `--fix` actually produces?
#[test]
fn p11_l2_rejection_rate_on_real_proposer_output() {
    println!("\n=== P11 — is Phase 11's \"would-fail-L2\" set non-empty? ===");
    let corpus = ClippyLintDomain::build_corpus();
    let pruner = RustAnalyzerPruner::new();

    let (mut produced, mut accepted) = (0usize, 0usize);
    for entry in &corpus {
        let lint = entry.rule_id();
        if !FIXABLE_LINTS.contains(&lint) {
            continue;
        }
        // The real `--fix` input: a proposer's spliced output.
        let Some(fixed) = propose_fix(lint, entry.buggy_code()) else {
            continue;
        };
        produced += 1;
        if pruner.validate(&fixed, lint).parses {
            accepted += 1;
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize, d: usize| {
        if d == 0 {
            0.0
        } else {
            k as f32 / d as f32 * 100.0
        }
    };
    let rejected = produced - accepted;
    println!("  proposer outputs fed to L2 : {produced}");
    println!(
        "  L2 ACCEPTED                : {accepted} ({:.1}%)",
        pct(accepted, produced)
    );
    println!(
        "  L2 REJECTED                : {rejected} ({:.1}%)  <- Phase 11's target set",
        pct(rejected, produced)
    );
    println!(
        "\n  VERDICT: {}",
        if rejected == 0 {
            "the would-fail-L2 set is EMPTY. Phase 11's G1 (\"reject >=30% of \
             would-fail-L2 fixes\") is a recall over zero positives — undefined, \
             not merely hard. L2 is a syn parse and span-spliced proposer output \
             is valid Rust by construction; the validator that DOES reject is \
             fix_verify's cargo check, which Phase 11 does not target."
        } else {
            "the set is non-empty — Phase 11's G1 is at least measurable"
        }
    );
}

/// P12 — does the healer produce multi-step trajectories to classify?
#[test]
fn p12_fix_attempts_are_multi_step() {
    println!("\n=== P12 — does a fix ATTEMPT have more than one step? ===");
    let corpus = ClippyLintDomain::build_corpus();

    // The `--fix` loop iterates a proposer to a bounded fixpoint. A trajectory
    // exists only if re-applying changes the code again; if pass 2 is always a
    // no-op, every attempt is one step and there is no shape to classify.
    let (mut single, mut multi, mut probed) = (0usize, 0usize, 0usize);
    for entry in &corpus {
        let lint = entry.rule_id();
        if !FIXABLE_LINTS.contains(&lint) {
            continue;
        }
        let Some(pass1) = propose_fix(lint, entry.buggy_code()) else {
            continue;
        };
        probed += 1;
        match propose_fix(lint, &pass1) {
            Some(pass2) if pass2 != pass1 => multi += 1,
            _ => single += 1,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let pct = |k: usize, d: usize| {
        if d == 0 {
            0.0
        } else {
            k as f32 / d as f32 * 100.0
        }
    };
    println!("  fixable rules producing a fix : {probed}");
    println!(
        "  ONE-STEP attempts             : {single} ({:.1}%)",
        pct(single, probed)
    );
    println!(
        "  MULTI-STEP attempts           : {multi} ({:.1}%)  <- classifiable trajectories",
        pct(multi, probed)
    );
    println!(
        "\n  VERDICT: {}",
        if multi == 0 {
            "every fix attempt converges in ONE step. SweTrajectoryFreezer takes \
             a SEQUENCE of states (&[&[f32]]); a one-element sequence has no \
             displacement geometry, so the three archetypes (converged_correct / \
             committed_wrong / oscillation) collapse — `oscillation` in \
             particular cannot occur."
        } else {
            "multi-step attempts exist — archetypes are classifiable"
        }
    );
}

/// P14 — is T14.2's "O(N) hash scan" premise true?
#[test]
fn p14_evolution_lookup_is_already_constant_time() {
    use riir_refine::evolve::{EvolveConfig, LatentSkillEvolution};
    println!("\n=== P14 — T14.2 claims LatentSkillEvolution does an \"O(N) hash scan\" ===");

    let mut ev = LatentSkillEvolution::new(EvolveConfig::default());
    let dir = [0.1_f32; 8];
    // Grow the store and time lookups at two very different sizes. If lookup
    // were O(N), the ratio would track the size ratio (100x).
    for i in 0..40 {
        ev.record_outcome(&format!("k{i}"), &dir, true);
    }
    let t = Instant::now();
    for _ in 0..100_000 {
        std::hint::black_box(ev.get("k39"));
    }
    let small = t.elapsed().as_secs_f64();

    for i in 40..4000 {
        ev.record_outcome(&format!("k{i}"), &dir, true);
    }
    let t = Instant::now();
    for _ in 0..100_000 {
        std::hint::black_box(ev.get("k3999"));
    }
    let large = t.elapsed().as_secs_f64();

    println!("  lookup @ 40 keys   : {:.1} ns", small * 1e9 / 100_000.0);
    println!("  lookup @ 4000 keys : {:.1} ns", large * 1e9 / 100_000.0);
    println!("  size ratio 100x -> time ratio {:.2}x", large / small);
    println!(
        "\n  VERDICT: {}",
        if large / small < 5.0 {
            "REFUTED — lookup is flat under a 100x size increase, i.e. O(1). \
             `LatentSkillEvolution` stores a `HashMap<String, Direction>`; there \
             is no O(N) scan to replace, so T14.2's stated motivation does not \
             exist."
        } else {
            "lookup does scale with N — the premise holds"
        }
    );
}

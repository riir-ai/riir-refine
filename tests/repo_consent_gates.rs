//! Plan 333 T4 gates — the per-repo consent dispatch consult (the hosted
//! fix path's third consent surface). The dispatch half needs the
//! escalation posture (the dispatch modules are compiled under
//! `escalation_manifest` + ANY consumer feature, the code_egress shape);
//! the module's own gates (dark law, memo pin, record round-trip) live
//! ungated in `src/repo_consent.rs`'s test mod. Four holds:
//!
//! 1. **The dark law**: `repo_consent::ENABLED` is `false` and the consult
//!    is side-effect-free at the default posture (no record read, no file
//!    created, no prompt possible).
//! 2. **The memo pin**: the rendered prompt carries the T3 memo item-5
//!    disclosure word-for-word (module test pins the sentences; this test
//!    pins that the dispatch REFUSAL carries the exact refusal vocabulary).
//! 3. **The dispatch consult placement**: a `Refused` decision declines
//!    AFTER the gate's Stand reasons and BEFORE any transport work — the
//!    declined reason IS the refusal vocabulary; a `Disabled` decision is
//!    behaviorally identical to the pre-T4 seam.
//! 4. **The record is per-repo** (two repos, two records; one repo's
//!    answer never leaks to the other).
#![cfg(all(
    feature = "escalation_manifest",
    any(feature = "seat_rerank", feature = "hosted_expert")
))]

use riir_refine::code_egress::EgressConsent;
use riir_refine::escalation_dispatch::{EscalatedFix, EscalationDispatch, ReadinessPolicy};
use riir_refine::escalation_manifest::EscalationManifest;
use riir_refine::repo_consent::{
    REFUSAL_NEVER, REFUSAL_UNSET, RepoAnswer, RepoConsentDecision, read, write,
};

fn manifest_row_armed() -> EscalationManifest {
    // One clippy seat_rerank row: modelless_cap 1.0 = every miss is a
    // gate Proceed under the scores the tests use (0.0 < 1.0), so the
    // ONLY standing reason left after the gate is the consult itself.
    let json = r#"{"domains":{"clippy":{"modelless_cap":1.0,"max_rate":0.9,"target":{"kind":"seat_rerank","endpoint":"http://127.0.0.1:1"}}}}"#;
    let m: EscalationManifest = serde_json::from_str(json).expect("t4 manifest json");
    m.validate().expect("t4 manifest valid");
    m
}

fn armed_dispatch() -> EscalationDispatch {
    let m = manifest_row_armed();
    EscalationDispatch::arm_with(&m, "clippy", None)
        .expect("arm ok")
        .expect("t4 row arms")
}

#[test]
fn the_enable_flip_is_dark_and_side_effect_free() {
    // The deliberate tripwire (the const block satisfies
    // assertions_on_constants): reds LOUD at the flip, forcing the flipper
    // to touch this pin in the same commit as the owner signature.
    const {
        assert!(
            !riir_refine::repo_consent::ENABLED,
            "the ENABLE flip is an owner act (plan 333 Phase B, the memo signature IS the gate) — flip this pin in the same commit as the signature"
        );
    }
    let tmp = std::env::temp_dir().join(format!("t4_dark_{}", std::process::id()));
    let dir = tmp.join("account");
    let repo = tmp.join("repo");
    std::fs::create_dir_all(&dir).unwrap();
    // Pre-existing record: even a REFUSED record is not consulted while
    // dark — the consult is a no-op, the decision is Disabled.
    write(&dir, &repo, RepoAnswer::Refused).unwrap();
    let d = riir_refine::repo_consent::consult(&dir, &repo, 20, false);
    assert_eq!(d, RepoConsentDecision::Disabled);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn refused_decision_declines_with_the_exact_vocabulary() {
    let mut dispatch = armed_dispatch();
    // gate: modelless_cap 1.0, score 0.0 < 1.0 → Proceed, so the ONLY
    // standing reason left is the consult.
    let decision = dispatch.escalate_rerank_checked_with(
        0.0,
        "fn main() {}",
        &["candidate a".to_string(), "candidate b".to_string()],
        std::time::Instant::now(),
        ReadinessPolicy::default(),
        &EgressConsent::parse(Some("snippet")),
        &RepoConsentDecision::Refused(REFUSAL_NEVER),
    );
    match decision {
        EscalatedFix::StoodByModelless {
            reason: riir_refine::escalation_dispatch::StandReason::Declined(msg),
        } => {
            assert!(
                msg.contains(REFUSAL_NEVER),
                "the Declined reason must carry the exact refusal vocabulary, got: {msg}"
            );
        }
        other => panic!("a refused repo consent must stand by modelless, got {other:?}"),
    }
}

#[test]
fn unset_vocabulary_is_the_named_refusal() {
    // The vocabulary constants themselves are the pin surface: they must
    // name the ask-once-per-repo contract and the egress master env.
    assert!(REFUSAL_UNSET.contains("not asked yet"));
    assert!(REFUSAL_UNSET.contains("asks once per repo"));
    assert!(REFUSAL_UNSET.contains("RIIR_REFINE_CODE_EGRESS=snippet"));
    assert!(REFUSAL_NEVER.contains("never send code off-machine"));
    assert!(REFUSAL_NEVER.contains("re-ask"));
}

#[test]
fn disabled_decision_is_pre_t4_identical() {
    // The seam shape check: a Disabled decision returns the SAME verdict
    // class the consent seam produced pre-T4 (the row above has no client
    // installed, so the tail is the transport-not-built decline — the
    // consult must not have changed it).
    let mut dispatch = armed_dispatch();
    let decision = dispatch.escalate_rerank_checked_with(
        0.0,
        "fn main() {}",
        &["candidate a".to_string(), "candidate b".to_string()],
        std::time::Instant::now(),
        ReadinessPolicy::default(),
        &EgressConsent::parse(Some("snippet")),
        &RepoConsentDecision::disabled(),
    );
    match decision {
        EscalatedFix::StoodByModelless {
            reason: riir_refine::escalation_dispatch::StandReason::Declined(msg),
        } => {
            assert!(
                !msg.contains("repo consent"),
                "the dark posture must never surface a repo-consent reason, got: {msg}"
            );
        }
        other => panic!("an unarmed transport stands by modelless, got {other:?}"),
    }
}

#[test]
fn records_are_per_repo() {
    let tmp = std::env::temp_dir().join(format!("t4_per_repo_{}", std::process::id()));
    let dir = tmp.join("account");
    let repo_a = tmp.join("repo_a");
    let repo_b = tmp.join("repo_b");
    std::fs::create_dir_all(&dir).unwrap();
    write(&dir, &repo_a, RepoAnswer::Refused).unwrap();
    // repo_b stays Unset; repo_a's refusal must not leak.
    assert_eq!(read(&dir, &repo_a), RepoAnswer::Refused);
    assert_eq!(read(&dir, &repo_b), RepoAnswer::Unset);
    let _ = std::fs::remove_dir_all(&tmp);
}

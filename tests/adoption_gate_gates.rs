//! Issue 220 / katgpt-rs Plan 633 T1 — the adoption-gate consumer seam's
//! gate battery (required-features = adoption_gate; a default-posture run
//! compiles this to nothing, so the repo-birth `#![cfg]` + the
//! required-features row land in the SAME commit).
//!
//! G1 is the pre-registered bar: a seeded synthetic outcome stream
//! replays through the consumer seam (JSONL rows → replay → the
//! producer's decide) **bit-identically to the producer fixture
//! verdicts** — the exact f64 numbers from
//! `katgpt_core::adoption_gate`'s `degraded_challenger_keeps_incumbent`
//! and `genuine_improvement_adopts_and_ties_go_to_challenger` fixtures.
//! SYNTHETIC gate-mechanics only — never quotable as serving quality
//! (the 976-corpus lesson).

#![cfg(feature = "adoption_gate")]

use std::path::PathBuf;

use katgpt_core::adoption_gate::{AdoptionDecision, DEFAULT_ADOPTION_ALPHA};

use riir_refine::adoption_gate::{AdoptionWindow, GateVerdict, SEAT_TAG, SeatSwapVerdict};

/// A scratch repo root (a pid-scoped temp dir — the shared-temp-path
/// law: another PROCESS running the same test must never share the path).
fn scratch_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "i220_gates_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir creates");
    dir
}

/// Record a synthetic outcome under `tag` with payload `pN` — the digest
/// seam's identity (the same tag + payload = the same outcome).
fn record(window: &mut AdoptionWindow, tag: &str, accepted: bool, payload: &str) {
    window.record(tag, accepted, payload, None);
}

/// G1 — the EXACT producer-fixture replay through the seam: the same
/// ingest sequence the producer fixture uses (accepted outcomes in one
/// order, rejected after; stride-5 audit) replayed as window rows, then
/// the DEMOTE challenger over the corresponding slice. The producer
/// fixture's challenger carried one out-of-window novel output, which a
/// minus-tag challenger cannot express (every seam row is window rows) —
/// disclosed; the NUMBERS that are representable are pinned bit-exactly:
/// retained is the producer fixture's own headline `8/12`, repeated is
/// the seam composition's `1/9`, and the score is the producer scorer's
/// own f64 expression (this seam calls the producer's `decide` — the
/// expression tree cannot drift). Decision Keep, same direction as the
/// producer fixture.
#[test]
fn g1_seam_replays_producer_fixture_numbers() {
    let root = scratch_root("g1_exact");
    let mut w = AdoptionWindow::load(&root).expect("empty window loads");
    // Ingest order pins the audit: 9 core accepted (1-9), 5 leaving
    // accepted (10-14) → the 5th (core) and 10th (leaving) accepted
    // outcomes file to audit → T-accepted: core 8, leaving 4 = 12 (the
    // producer fixture's 12). Rejected: 1 core + 1 leaving → 2 T-rejected
    // (the producer fixture's 2).
    for i in 0..9 {
        record(&mut w, "core", true, &format!("acc:{i}"));
    }
    for i in 0..5 {
        record(&mut w, "leaving", true, &format!("lv:{i}"));
    }
    record(&mut w, "core", false, "rej-core");
    record(&mut w, "leaving", false, "rej-leaving");
    // Reload — the replay path the gate actually takes.
    let w2 = AdoptionWindow::load(&root).expect("reload");
    assert_eq!(w2.row_count(), 16);
    // The demote of `leaving`: challenger = window minus leaving = 8
    // accepted + 1 rejected → retained 8/12 (the producer fixture's
    // headline), repeated 1/9, score the scorer's own expression → Keep.
    let verdict = match w2.demote_verdict("leaving", DEFAULT_ADOPTION_ALPHA) {
        GateVerdict::Refuse(v) => v,
        other => panic!("the leaving demote must refuse, got {other:?}"),
    };
    assert_eq!(verdict.retained, 8.0 / 12.0);
    assert_eq!(verdict.repeated, 1.0 / 9.0);
    let expected_score =
        (1.0 - DEFAULT_ADOPTION_ALPHA) * (8.0 / 12.0) + DEFAULT_ADOPTION_ALPHA * (1.0 - 1.0 / 9.0);
    assert_eq!(verdict.score, expected_score);
    assert_eq!(verdict.decision, AdoptionDecision::Keep);
    // The empty-challenger case: a window where ONE tag owns every row —
    // demoting it leaves no rows: retained 0, repeated 0, score
    // (1-α)·0 + α·1 = 0.5 < the incumbent → Keep.
    let root2 = scratch_root("g1_empty_challenger");
    let mut w3 = AdoptionWindow::load(&root2).expect("empty window loads");
    for i in 0..12 {
        record(&mut w3, "r", true, &format!("acc:{i}"));
    }
    record(&mut w3, "r", false, "rej:0");
    let w4 = AdoptionWindow::load(&root2).expect("reload");
    let all = match w4.demote_verdict("r", DEFAULT_ADOPTION_ALPHA) {
        GateVerdict::Refuse(v) => v,
        other => panic!("the core demote must refuse, got {other:?}"),
    };
    assert_eq!(all.retained, 0.0);
    assert_eq!(all.repeated, 0.0);
    assert_eq!(all.score, 0.5);
    std::fs::remove_dir_all(&root2).ok();
    std::fs::remove_dir_all(&root).ok();
}

/// G1 — the producer's `genuine_improvement_adopts` shape through the
/// demote seam: demoting a tag whose rows are PURE REJECTED outputs
/// strictly RAISES the challenger (removing rejects raises the score) —
/// the gate ADOPTS the demote, pins the tie rule's effect, and the
/// numbers must match the producer scorer's arithmetic exactly.
#[test]
fn g1_demoting_a_pure_reject_adopts() {
    let root = scratch_root("g1_adopt");
    let mut w = AdoptionWindow::load(&root).expect("empty window loads");
    // 8 accepted under `core` (audit holds out the 5th), 2 rejected under
    // `bad`.
    for i in 0..8 {
        record(&mut w, "core", true, &format!("acc:{i}"));
    }
    for j in 0..2 {
        record(&mut w, "bad", false, &format!("rej:{j}"));
    }
    // Demote `bad`: the challenger = the window minus the 2 rejected rows
    // → retained = 1.0 (all accepted kept), repeated = 0 → score 1.0;
    // the incumbent scored (1-α)·1 + α·(1 - 2/…T-rejected share)…
    // Producer arithmetic: incumbent retained = 1.0; incumbent outputs =
    // T-accepted + T-rejected; with 8 accepted at stride 5 → 7th rejected?
    // No: accepted 8 → the 5th is audited → 7 T-accepted; 2 T-rejected.
    // incumbent repeated = 2/9. challenger repeated = 0/7. challenger
    // score = 0.5·1 + 0.5·1 = 1.0 > incumbent — Adopt, numbers pinned.
    let verdict = match w.demote_verdict("bad", DEFAULT_ADOPTION_ALPHA) {
        GateVerdict::Proceed(v) => v,
        other => panic!("demoting a pure-reject rule should PROCEED (adopt), got {other:?}"),
    };
    assert_eq!(verdict.retained, 1.0);
    assert_eq!(verdict.repeated, 0.0);
    assert_eq!(verdict.score, 1.0);
    std::fs::remove_dir_all(&root).ok();
}

/// G2's on-posture half + the demote refusal path: demoting a rule that
/// was CARRYING accepted fixes drops `retained` → Keep → REFUSE. Also
/// pins the abstain posture (an empty window never blocks).
#[test]
fn demote_gate_refuses_a_carrying_rule_and_abstains_on_no_evidence() {
    let root = scratch_root("demote_refuse");
    let mut w = AdoptionWindow::load(&root).expect("empty window loads");
    // Abstain first — the empty window.
    assert_eq!(
        w.demote_verdict("anything", DEFAULT_ADOPTION_ALPHA),
        GateVerdict::Abstain
    );
    // The rule under demotion carries 5 accepted fixes; the rest of the
    // window carries 7 accepted + 1 rejected.
    for i in 0..5 {
        record(&mut w, "carrier", true, &format!("c:{i}"));
    }
    for i in 0..7 {
        record(&mut w, "other", true, &format!("o:{i}"));
    }
    record(&mut w, "other", false, "o-rej:0");
    // Reload — the gate's real read path.
    let w2 = AdoptionWindow::load(&root).expect("reload");
    match w2.demote_verdict("carrier", DEFAULT_ADOPTION_ALPHA) {
        GateVerdict::Refuse(v) => {
            // Challenger = window minus the carrier: retained 7/…T, the
            // exact split is the scorer's; assert the DIRECTION + Keep.
            assert_eq!(v.decision, AdoptionDecision::Keep);
            assert!(
                v.retained < 1.0,
                "dropping the carrier must drop retained, got {}",
                v.retained
            );
        }
        other => panic!("demoting a carrying rule must refuse, got {other:?}"),
    }
    // A foreign rule abstains (no rows) even though evidence exists.
    assert_eq!(
        w2.demote_verdict("ghost", DEFAULT_ADOPTION_ALPHA),
        GateVerdict::Abstain
    );
    std::fs::remove_dir_all(&root).ok();
}

/// The readmit gate: a rule whose history repeats a known-rejected output
/// is REFUSED; a clean-history rule PROCEEDS; a rule with no rows
/// abstains. This is the never-double-file law's consumer face: the same
/// (tag, payload) recorded twice files ONCE (the second record is a
/// no-op), so the repeated metric reads the RING, not the file.
#[test]
fn readmit_gate_refuses_repeats_and_abstains_without_rows() {
    let root = scratch_root("readmit");
    let mut w = AdoptionWindow::load(&root).expect("empty window loads");
    assert_eq!(
        w.readmit_verdict("x", DEFAULT_ADOPTION_ALPHA),
        GateVerdict::Abstain
    );
    // The rule's history: 2 accepted, 1 rejected.
    record(&mut w, "x", true, "x-acc:0");
    record(&mut w, "x", true, "x-acc:1");
    record(&mut w, "x", false, "x-rej:0");
    // Never-double-file: the SAME outcome again is a no-op (one row).
    record(&mut w, "x", false, "x-rej:0");
    assert_eq!(w.row_count(), 3, "the duplicate outcome must not append");
    // The demoted-period window ALSO carries other rules' outcomes so the
    // rings are non-empty (the readmit scores against the current window).
    record(&mut w, "y", true, "y-acc:0");
    let w2 = AdoptionWindow::load(&root).expect("reload");
    match w2.readmit_verdict("x", DEFAULT_ADOPTION_ALPHA) {
        GateVerdict::Refuse(v) => {
            assert!(v.repeated > 0.0, "the rejected row must read as a repeat")
        }
        other => panic!("a repeat-history readmit must refuse, got {other:?}"),
    }
    // A clean-history rule proceeds.
    record(&mut w, "z", true, "z-acc:0");
    let w3 = AdoptionWindow::load(&root).expect("reload");
    match w3.readmit_verdict("z", DEFAULT_ADOPTION_ALPHA) {
        GateVerdict::Proceed(_) => {}
        other => panic!("a clean readmit must proceed, got {other:?}"),
    }
    std::fs::remove_dir_all(&root).ok();
}

/// The seat head-swap watch: rows carry the build fingerprint; a build
/// whose rows repeat known-rejected outputs STANDS DOWN; a clean or
/// unseen build proceeds/no-evidence.
#[test]
fn seat_swap_watch_stands_down_on_regression_evidence() {
    let root = scratch_root("seat_swap");
    let mut w = AdoptionWindow::load(&root).expect("empty window loads");
    // Head A produced: 2 accepted, 1 rejected. Head B produced: 1
    // accepted + a REPEAT of head A's rejected payload (the regression
    // class). The repeat must carry head B's tag-build but a DISTINCT
    // payload (a different outcome identity) — otherwise the dedup eats
    // it. The regression signal for the watch is head B's row sitting in
    // the REJECTED ring: record head B's pick as REJECTED (the verifier
    // reverted it) — that row IS the repeat evidence.
    let build_row = |w: &mut AdoptionWindow, build: &str, accepted: bool, payload: &str| {
        w.record(SEAT_TAG, accepted, payload, Some(build));
    };
    build_row(&mut w, "headA", true, "pickA0");
    build_row(&mut w, "headA", true, "pickA1");
    build_row(&mut w, "headA", false, "pickA2");
    assert_eq!(
        w.seat_swap_verdict("headC"),
        SeatSwapVerdict::NoEvidence,
        "an unseen build abstains"
    );
    // Head B's first life: one accepted pick.
    build_row(&mut w, "headB", true, "pickB0");
    assert_eq!(
        w.seat_swap_verdict("headB"),
        SeatSwapVerdict::Proceed {
            repeated_hits: 0,
            rows: 1
        },
        "a clean build proceeds"
    );
    // Head B later regressed: its pick was reverted (the verifier's
    // verdict lands as a rejected row under head B).
    build_row(&mut w, "headB", false, "pickB1");
    match w.seat_swap_verdict("headB") {
        SeatSwapVerdict::StandDown {
            repeated_hits,
            rows,
        } => {
            assert_eq!(repeated_hits, 1, "the reverted pick is the repeat evidence");
            assert_eq!(rows, 2);
        }
        other => panic!("a regressed build must stand down, got {other:?}"),
    }
    std::fs::remove_dir_all(&root).ok();
}

/// The digest identity: the same (tag, payload) is the SAME outcome
/// regardless of the verdict bit recorded first (the rings keep the first
/// verdict; a later opposite verdict on the same output is a no-op) —
/// pinned so the never-double-file law is explicit about verdict flips.
#[test]
fn digest_identity_and_first_verdict_wins() {
    let root = scratch_root("digest");
    let mut w = AdoptionWindow::load(&root).expect("empty window loads");
    record(&mut w, "r", true, "same");
    record(&mut w, "r", false, "same"); // flip — dedup'd, no row
    assert_eq!(w.row_count(), 1);
    // A different tag with the same payload is a DIFFERENT outcome.
    record(&mut w, "s", true, "same");
    assert_eq!(w.row_count(), 2);
    std::fs::remove_dir_all(&root).ok();
}

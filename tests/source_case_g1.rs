#![cfg(feature = "source_case_gate")]
//! Issue 232 T4 — the G1 fixture (Research 495, the T³ holdout-epistemology
//! extraction): the source-case-first gate's pre-registered predicates.
//!
//! 1. **The NoOp classes decline:** a fix byte-equal to its trigger (the
//!    zero-length step) and an empty/whitespace fix cannot resolve a
//!    finding — declined mechanically, before any embed arithmetic.
//! 2. **Engagement (the source case is live):** the rule's own FIXED
//!    exemplar as the fix IMPROVES; a near-copy of the rule's own BUGGY
//!    exemplar (whitespace-shifted, not byte-equal) lands Uphill through
//!    the embed space.
//! 3. **The uphill comparison is signed + exact:** positive beats negative
//!    outright; the near-identical-exemplar cousin (T2's flip axis) is a
//!    strict tie that passes through — no epsilon, no tolerance band;
//!    same inputs → the same verdict (determinism).
//! 4. **The confusion readout weighs by outcome counts:** a row's verdict
//!    applies to its whole outcome history; no-rule and zero-outcome rows
//!    are disclosed, never folded; the bar verdict is UNDECIDABLE on thin
//!    denominators, never a pass over zero evidence.
//! 5. **The render never refuses** (the T1/T2 verdict-law discipline) and
//!    names the preregistered G2 bar.
//! 6. **The store surface end to end:** seeded rows read back through the
//!    same fold the CLI lane runs, exact counts pinned.

use riir_refine::source_case::{
    ConfusionReadout, DeclineClass, LivedRow, READOUT_VERSION, SourceCaseVerdict,
    confusion_readout, source_case_check,
};

const BUGGY: &str = "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }";
const FIXED: &str = "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }";

#[test]
fn g1_noop_classes_decline_mechanically() {
    assert_eq!(
        source_case_check(BUGGY, BUGGY, BUGGY, FIXED),
        SourceCaseVerdict::Declined(DeclineClass::NoOp),
        "a byte-equal fix is the zero-length step class"
    );
    assert_eq!(
        source_case_check(BUGGY, "", BUGGY, FIXED),
        SourceCaseVerdict::Declined(DeclineClass::NoOp)
    );
    assert_eq!(
        source_case_check(BUGGY, "   \n\t ", BUGGY, FIXED),
        SourceCaseVerdict::Declined(DeclineClass::NoOp)
    );
}

#[test]
fn g1_engagement_own_resolution_improves_own_bug_uphill() {
    // The resolution itself passes the source case.
    assert_eq!(
        source_case_check(BUGGY, FIXED, BUGGY, FIXED),
        SourceCaseVerdict::Improves
    );
    // A whitespace-shifted near-copy of the buggy exemplar (NOT byte-equal
    // — NoOp must not catch it) lands Uphill via the embed space: the
    // candidate resembles the bug, not the resolution.
    let near_bug = BUGGY.replace("out.push(page); }", "out.push(page);  }");
    assert_ne!(near_bug, BUGGY);
    assert_eq!(
        source_case_check(BUGGY, &near_bug, BUGGY, FIXED),
        SourceCaseVerdict::Declined(DeclineClass::Uphill)
    );
    // A fix closer to the resolution's SHAPE improves: the collect() form
    // with extra commentary still names the resolution side.
    let fixed_shaped =
        "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() } // fixed";
    assert_eq!(
        source_case_check(BUGGY, fixed_shaped, BUGGY, FIXED),
        SourceCaseVerdict::Improves
    );
}

#[test]
fn g1_uphill_signed_exact_and_tie_passing() {
    // Determinism: same inputs, same verdict — twice.
    let unrelated = "fn zz(q8: u128) -> bool { q8 == 0 }";
    assert_eq!(
        source_case_check(BUGGY, unrelated, BUGGY, FIXED),
        source_case_check(BUGGY, unrelated, BUGGY, FIXED)
    );
    // The flip-axis cousin: identical exemplars cannot discriminate —
    // cos(fix, buggy) == cos(fix, fixed) exactly, and the STRICT tie rule
    // passes through. No epsilon anywhere.
    assert_eq!(
        source_case_check(BUGGY, FIXED, BUGGY, BUGGY),
        SourceCaseVerdict::Improves
    );
    // Sign quadrant / role semantics: the buggy vs fixed roles are
    // SEMANTIC, not presentational — the same fix text favors whichever
    // side it resembles, so swapping the roles inverts the verdict.
    let unrelated = "fn zz(q8: u128) -> bool { q8 == 0 }";
    // Determinism: same inputs, same verdict — twice.
    assert_eq!(
        source_case_check(BUGGY, unrelated, BUGGY, FIXED),
        source_case_check(BUGGY, unrelated, BUGGY, FIXED)
    );
    // Role inversion: the resolution text passes against (buggy, fixed)
    // and declines against the INVERTED pair (where its own shape is the
    // buggy exemplar and the bug text is the resolution).
    assert_eq!(
        source_case_check(BUGGY, FIXED, BUGGY, FIXED),
        SourceCaseVerdict::Improves
    );
    assert_eq!(
        source_case_check(BUGGY, FIXED, FIXED, BUGGY),
        SourceCaseVerdict::Declined(DeclineClass::Uphill)
    );
}

#[test]
fn g1_confusion_weights_by_counts_and_discloses_skips() {
    let rows = vec![
        LivedRow {
            rule_id: "r_a".into(),
            shape: BUGGY.into(),
            fix: FIXED.into(),
            success_count: 3,
            fail_count: 1,
        },
        LivedRow {
            rule_id: "r_b".into(),
            shape: "let mut v = Vec::new(); for x in xs { v.push(x); } v".into(),
            fix: "let mut v = Vec::new(); for x in xs { v.push(x); } v ".into(),
            success_count: 2,
            fail_count: 8,
        },
        LivedRow {
            rule_id: "r_ghost".into(),
            shape: "a".into(),
            fix: "b".into(),
            success_count: 1,
            fail_count: 0,
        },
        LivedRow {
            rule_id: "r_a".into(),
            shape: "c".into(),
            fix: "d".into(),
            success_count: 0,
            fail_count: 0,
        },
    ];
    let pairs = vec![
        ("r_a".to_string(), BUGGY.to_string(), FIXED.to_string()),
        (
            "r_b".to_string(),
            "let mut v = Vec::new(); for x in xs { v.push(x); } v".to_string(),
            "let v: Vec<_> = xs.into_iter().collect(); v".to_string(),
        ),
    ];
    let rep = confusion_readout(&rows, &pairs);
    assert_eq!(rep.readout_version, READOUT_VERSION);
    assert_eq!(rep.rows, 2, "r_ghost + the zero-outcome row are disclosed");
    assert_eq!(rep.skipped_no_rule, 1);
    assert_eq!(rep.skipped_zero_outcome, 1);
    // Count weighting: the verdict is per row, the cells carry the row's
    // whole outcome history.
    assert_eq!(rep.improves_win, 3);
    assert_eq!(rep.improves_fail, 1);
    assert_eq!(rep.uphill_win, 2);
    assert_eq!(rep.uphill_fail, 8);
    assert_eq!(rep.total_win, 5);
    assert_eq!(rep.total_fail, 9);
    // The bar legs on these numbers: recall 8/9 ≥ 50% holds, false-decline
    // 2/5 > 10% misses → MISS (the bar reads BOTH legs).
    assert_eq!(rep.bar_verdict(), Some("MISS"));
}

#[test]
fn g1_bar_verdict_undecidable_on_thin_denominators() {
    let no_fails = ConfusionReadout {
        readout_version: READOUT_VERSION,
        rows: 1,
        skipped_no_rule: 0,
        skipped_zero_outcome: 0,
        improves_win: 5,
        improves_fail: 0,
        noop_win: 0,
        noop_fail: 0,
        uphill_win: 0,
        uphill_fail: 0,
        mean_max_cos: 0.5,
        total_win: 5,
        total_fail: 0,
    };
    assert_eq!(
        no_fails.bar_verdict(),
        None,
        "a rate over zero evidence is UNDECIDABLE, never a pass"
    );
}

#[test]
fn g1_render_names_the_bar_and_never_refuses() {
    let rep = confusion_readout(
        &[LivedRow {
            rule_id: "r_a".into(),
            shape: BUGGY.into(),
            fix: FIXED.into(),
            success_count: 4,
            fail_count: 2,
        }],
        &[("r_a".to_string(), BUGGY.to_string(), FIXED.to_string())],
    );
    let text = riir_refine::source_case::render(&rep);
    let low = text.to_ascii_lowercase();
    for w in ["refuse", "reject", "deny", "refusal"] {
        assert!(!low.contains(w), "the report must never say {w}");
    }
    assert!(text.contains("preregistered G2 bar"));
    assert!(text.contains("no wiring exists"));
}

#[test]
fn g1_store_surface_reads_the_lived_rows_end_to_end() {
    // The shared_temp_path law: a per-process temp root.
    let root = std::env::temp_dir().join(format!("source_case_g1_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root created");
    let (store, bad) = riir_refine::traj_store::TrajStore::open(&root).expect("open store");
    assert_eq!(bad, 0, "fresh store has no corrupt rows");

    // r_a: the rule's own resolution as the fix — Improves, 18W/2F
    // (deterministic win pattern, never 3 consecutive fails).
    for k in 0..20 {
        let win = !(k == 5 || k == 15);
        store
            .record("r_a", BUGGY, FIXED, win, win, "g1-seed")
            .expect("record");
    }
    // r_b: a whitespace-shifted near-bug fix — Uphill, 1W/9F.
    let shape_b = "let mut v = Vec::new(); for x in xs { v.push(x); } v";
    let near_bug = "let mut v = Vec::new(); for x in xs { v.push(x); } v ";
    for k in 0..10 {
        let win = k == 0;
        store
            .record("r_b", shape_b, near_bug, win, win, "g1-seed")
            .expect("record");
    }
    // r_ghost: no exemplar pair — disclosed, never folded.
    store
        .record("r_ghost", "a", "b", true, true, "g1-seed")
        .expect("record");
    drop(store);

    // Read back through the same fold the CLI lane runs (the pairs table
    // explicit — the compiled-corpus join is the composed posture's
    // business; the fixture pins the arithmetic).
    let (store2, _) = riir_refine::traj_store::TrajStore::open(&root).expect("reopen");
    let rows: Vec<LivedRow> = store2
        .dump()
        .into_iter()
        .map(|r| LivedRow {
            rule_id: r.lint_key,
            shape: r.shape,
            fix: r.fix,
            success_count: r.success_count,
            fail_count: r.fail_count,
        })
        .collect();
    drop(store2);
    let pairs = vec![
        ("r_a".to_string(), BUGGY.to_string(), FIXED.to_string()),
        (
            "r_b".to_string(),
            shape_b.to_string(),
            "let v: Vec<_> = xs.into_iter().collect(); v".to_string(),
        ),
    ];
    let rep = confusion_readout(&rows, &pairs);
    assert_eq!(rep.rows, 2);
    assert_eq!(rep.skipped_no_rule, 1);
    assert_eq!(rep.improves_win, 18);
    assert_eq!(rep.improves_fail, 2);
    assert_eq!(rep.uphill_win, 1);
    assert_eq!(rep.uphill_fail, 9);
    // Rates: recall 9/11 = 81.8% ≥ 50% ✓, false-decline 1/19 = 5.3% ≤ 10%
    // ✓ → PASS — the bar's pass shape demonstrated on a fixture (the
    // module tests demonstrate the MISS shape; the REAL cell reads the
    // lived store).
    assert_eq!(rep.bar_verdict(), Some("PASS"));
    let _ = std::fs::remove_dir_all(&root);
}

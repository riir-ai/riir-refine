//! Issue 256 — the fix-trajectory census gates: the T0 probe's answer
//! shape, the T1 audit's typed verdicts, every census refusal, and the
//! early-fold edge arms. Tests the REAL pipeline
//! (`riir_refine::fix_trajectory_census`), not a copy.

use riir_refine::fix_trajectory_census::{
    CensusRefusal, early_count, early_run, recall_precision_at_median, t0_scores, t1_audits,
};
use riir_refine::horizon_report::{RingLine, RunRing};
use riir_refine::span_embed::span_embed;

/// A synthetic ring line: deterministic embed over the text, honest
/// errors columns.
fn line(seq: usize, strategy: &str, kept: bool, errors: (usize, usize)) -> RingLine {
    RingLine {
        seq,
        code: "E0597".to_string(),
        span_hash: format!("{:016x}", (seq as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        strategy_id: strategy.to_string(),
        outcome: if kept {
            "Kept".to_string()
        } else {
            "RevertedNoProgress".to_string()
        },
        errors,
        masking: (0, 0),
        journey_residual_sum: 0,
        terminal_residual: 0,
        span_embed: Some(span_embed(&format!(
            "{strategy} attempt {seq} trigger text"
        ))),
    }
}

/// A 20-attempt run whose first two attempts GROW errors and whose final
/// attempt reverts — the "doomed run" shape the T0 probe reads.
fn doomed(source: &str) -> RunRing {
    let mut lines: Vec<RingLine> = (0..19)
        .map(|i| {
            let grow = i < 2;
            line(
                i,
                "e0502-clone-out",
                true,
                if grow {
                    (5 + i, 6 + i)
                } else {
                    (30 - i, 29 - i)
                },
            )
        })
        .collect();
    lines.push(line(19, "e0502-clone-out", false, (2, 3)));
    RunRing {
        source: source.to_string(),
        lines,
    }
}

fn clean(source: &str) -> RunRing {
    RunRing {
        source: source.to_string(),
        lines: (0..20)
            .map(|i| line(i, "e0597-move", true, (30 - i, 29 - i)))
            .collect(),
    }
}

#[test]
fn early_count_is_the_max_one_tenth() {
    assert_eq!(early_count(1), 1);
    assert_eq!(early_count(9), 1);
    assert_eq!(early_count(10), 1);
    assert_eq!(early_count(20), 2);
    assert_eq!(early_count(100), 10);
}

#[test]
fn ends_bad_labels_the_final_revert() {
    let dirty = early_run(&doomed("d")).expect("multi-attempt");
    assert!(dirty.ends_bad);
    assert_eq!(dirty.early.len(), 2); // max(1, 20/10)
    let clean_e = early_run(&clean("c")).expect("multi-attempt");
    assert!(!clean_e.ends_bad);
}

#[test]
fn single_attempt_runs_are_skipped() {
    let single = RunRing {
        source: "one".into(),
        lines: vec![line(0, "s", true, (3, 2))],
    };
    assert!(early_run(&single).is_none());
}

#[test]
fn missing_embed_column_skips_the_run() {
    let mut ring = clean("noembed");
    // index 10 is in the LATER half (early = 2 of 20) — the fold's
    // later-state builder hits the missing column and skips the run.
    ring.lines[10].span_embed = None;
    assert!(early_run(&ring).is_none());
}

#[test]
fn t0_refuses_empty_and_scores_the_toy_signal() {
    assert!(t0_scores(&[]).is_none());
    let mut runs = Vec::new();
    for i in 0..6 {
        runs.push(early_run(&clean(&format!("clean{i}"))).expect("run"));
    }
    for i in 0..4 {
        runs.push(early_run(&doomed(&format!("dirty{i}"))).expect("run"));
    }
    let scores = t0_scores(&runs).expect("folds");
    assert_eq!(scores.len(), runs.len());
    let (recall, precision) = recall_precision_at_median(&scores);
    assert!(
        recall.is_some() && precision.is_some(),
        "both classes present"
    );
}

#[test]
fn t1_audits_return_typed_reports() {
    let runs = [
        early_run(&clean("c0")).expect("run"),
        early_run(&doomed("d0")).expect("run"),
    ];
    let audits = t1_audits(&runs);
    assert_eq!(audits.len(), 2);
    for (source, report) in &audits {
        let r = report.as_ref().expect("audit Ok");
        assert!(
            r.support.windows >= 1,
            "{source}: at least the early→later window exists"
        );
    }
}

#[test]
fn refusals_render_with_a_remedy() {
    let r = CensusRefusal::NoRings { roots: 15 };
    let s = r.to_string();
    assert!(s.contains("NO RINGS"), "{s}");
    assert!(s.contains("fix-compile"), "{s} names the capture lane");
    assert!(
        CensusRefusal::NoMultiAttempt { runs: 3 }
            .to_string()
            .contains("NO MULTI-ATTEMPT")
    );
    assert!(
        CensusRefusal::SingleClass { runs: 5 }
            .to_string()
            .contains("SINGLE CLASS")
    );
}

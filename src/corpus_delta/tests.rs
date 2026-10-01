//! Issue 116 T3 — arms for the corpus-truth delta surface.

use super::*;

fn fixable() -> Vec<String> {
    ["needless_return", "manual_map", "collapsible_if"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn verify() -> Vec<String> {
    ["redundant_closure", "manual_is_multiple_of"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn spans(rows: &[Vec<String>]) -> Vec<&[String]> {
    rows.iter().map(Vec::as_slice).collect()
}

fn missing(names: &[&str]) -> Vec<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn a_lint_in_the_fix_set_is_the_freshness_delta_not_a_gap() {
    let per_span = vec![missing(&["needless_return"])];
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert_eq!(d.delta_lints(), 1);
    assert_eq!(d.rows[0].tier, FixTier::Fixable);
    assert_eq!(d.unfixable, 0, "the fix lane carries it — not a coverage gap");
}

#[test]
fn a_lint_in_neither_lane_is_the_coverage_gap() {
    let per_span = vec![missing(&["must_use_candidate"])];
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert!(d.rows.is_empty());
    assert_eq!(d.unfixable, 1);
    assert_eq!(d.unfixable_spans, 1);
}

#[test]
fn the_two_buckets_partition_the_missing_set() {
    let per_span = vec![
        missing(&["needless_return", "must_use_candidate"]),
        missing(&["redundant_closure", "doc_markdown", "needless_return"]),
    ];
    let d = delta(spans(&per_span), &fixable(), &verify());
    // 4 distinct lints, 5 span occurrences.
    assert_eq!(d.missing_lints, 4);
    assert_eq!(d.missing_spans, 5);
    // Every lint is in exactly one bucket, and the spans partition too.
    assert_eq!(d.delta_lints() + d.unfixable, d.missing_lints);
    assert_eq!(d.delta_spans() + d.unfixable_spans, d.missing_spans);
}

#[test]
fn the_verify_tier_is_kept_apart_because_the_remedy_differs() {
    let per_span = vec![missing(&["redundant_closure"])];
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert_eq!(d.rows[0].tier, FixTier::Verify);
    assert_eq!(d.rows[0].tier.remedy(), "cargo refine --fix --verify");
    assert_eq!(FixTier::Fixable.remedy(), "cargo refine --fix");
    assert_ne!(
        FixTier::Fixable.remedy(),
        FixTier::Verify.remedy(),
        "collapsing the tiers would send half the rows to the wrong invocation"
    );
}

#[test]
fn a_lint_in_both_tables_reports_the_looser_remedy() {
    // `redundant_closure` is historically both (demoted to VERIFY by the
    // Bench 019 G5 run, restored under the compile gate) — a build carrying
    // it in both must not send the reader to a flag they do not need.
    let per_span = vec![missing(&["redundant_closure"])];
    let both = vec!["redundant_closure".to_string()];
    let d = delta(spans(&per_span), &both, &verify());
    assert_eq!(d.rows[0].tier, FixTier::Fixable);
}

#[test]
fn span_occurrences_accumulate_and_order_the_rows() {
    let per_span = vec![
        missing(&["needless_return"]),
        missing(&["needless_return", "manual_map"]),
        missing(&["needless_return"]),
    ];
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert_eq!(d.rows.len(), 2);
    assert_eq!(d.rows[0].lint, "needless_return");
    assert_eq!(d.rows[0].spans, 3, "one row, the count is the blast radius");
    assert_eq!(d.rows[1].spans, 1);
}

#[test]
fn the_count_is_over_entries_so_a_producer_change_cannot_be_absorbed() {
    // `push_anchor` dedups per span, so the producer cannot hand us a
    // repeat today. Pin the arithmetic anyway: the count is over ENTRIES,
    // and a producer change that stopped deduping must show up as a count
    // change rather than being silently swallowed here.
    let per_span = vec![missing(&["needless_return", "needless_return"])];
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert_eq!(d.rows[0].spans, 2);
}

#[test]
fn names_are_normalized_before_comparison() {
    // A kebab/tool-prefixed feed row must not read as a coverage gap purely
    // on punctuation — that would be a FALSE mining-intake row.
    let per_span = vec![missing(&["clippy::needless-return"])];
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert_eq!(d.unfixable, 0, "punctuation must not manufacture a gap");
    assert_eq!(d.rows[0].lint, "needless_return");
}

#[test]
fn the_summary_line_is_silent_on_an_empty_delta() {
    let empty: Vec<Vec<String>> = Vec::new();
    assert_eq!(
        delta(spans(&empty), &fixable(), &verify()).summary_line(),
        None
    );

    // ...and silent when every missing lint is a genuine coverage gap: the
    // delta is about FRESHNESS, and a line that fired on gaps would print
    // on almost every real run.
    let gaps = vec![missing(&["must_use_candidate", "doc_markdown"])];
    let d = delta(spans(&gaps), &fixable(), &verify());
    assert_eq!(d.unfixable, 2);
    assert_eq!(
        d.summary_line(),
        None,
        "never noise — the expected state is quiet"
    );
}

#[test]
fn the_summary_line_fires_and_names_the_verify_share() {
    let per_span = vec![missing(&["needless_return"])];
    let line = delta(spans(&per_span), &fixable(), &verify())
        .summary_line()
        .expect("a fix-set-covered missing lint must announce itself");
    assert!(line.contains("1 lint at 1 span"), "{line}");
    assert!(line.contains("the leased corpus lags"), "{line}");
    assert!(!line.contains("verify tier"), "no verify rows — no verify note");

    let mixed = vec![missing(&["needless_return", "redundant_closure"])];
    let line = delta(spans(&mixed), &fixable(), &verify())
        .summary_line()
        .unwrap();
    assert!(line.contains("2 lints at 2 spans"), "{line}");
    assert!(line.contains("(1 --verify tier)"), "{line}");

    let all_v = vec![missing(&["redundant_closure", "manual_is_multiple_of"])];
    let line = delta(spans(&all_v), &fixable(), &verify())
        .summary_line()
        .unwrap();
    assert!(
        line.contains("(all --verify tier)"),
        "a delta reachable ONLY under --verify must say so: {line}"
    );
}

#[test]
fn the_table_names_which_message_no_corpus_rule_is_delivering() {
    let per_span = vec![missing(&["needless_return", "must_use_candidate"])];
    let out = delta(spans(&per_span), &fixable(), &verify()).render();
    assert!(out.contains("FRESHNESS DELTA"));
    assert!(out.contains("not leased yet"));
    assert!(out.contains("COVERAGE GAP"));
    assert!(out.contains("does mean unsupported"));
    assert!(
        out.contains("DIFFERENT BY DESIGN"),
        "the seam must not read as a defect"
    );
    assert!(out.contains("needless_return"));
}

#[test]
fn the_table_still_reports_the_gap_when_the_delta_is_empty() {
    let per_span = vec![missing(&["must_use_candidate"])];
    let out = delta(spans(&per_span), &fixable(), &verify()).render();
    assert!(out.contains("freshness delta: NONE"));
    assert!(out.contains("COVERAGE GAP — 1 lint(s)"));
}

#[test]
fn missing_spans_matches_the_run_summarys_own_count() {
    // The run summary sums `anchored_missing.len()` per span; this must be
    // the same quantity or the two lines contradict each other on screen.
    let per_span = vec![
        missing(&["a_lint", "b_lint"]),
        missing(&["a_lint"]),
        Vec::new(),
    ];
    let hand_count: usize = per_span.iter().map(Vec::len).sum();
    let d = delta(spans(&per_span), &fixable(), &verify());
    assert_eq!(d.missing_spans, hand_count);
}

// ── The producer→consumer arm ───────────────────────────────────────────
//
// Every arm above hands `delta` a hand-built list, so all of them assert the
// ABSTRACTION and leave the production path free to disagree with it. This
// one drives the real producer: a leased overlay that REPLACES clippy's seed
// (exactly what a lagging lease does), a real anchor at a lint the overlay
// does not carry, and the resulting `anchored_missing` fed to `delta`
// against the real compiled `FIXABLE_LINTS`.
//
// It is also the only way to see the freshness bucket fire at all: on a
// lease-less build the compiled fix set is a SUBSET of the rule book
// (measured — `--coverage-report` reports fix-set-only = 0), so the delta is
// 0 by construction and an end-to-end CLI run can never exercise this path.
#[cfg(all(
    feature = "latent_retrieval",
    feature = "clippy_lints",
    feature = "rust_perf"
))]
#[test]
fn a_lagging_lease_produces_a_real_freshness_delta() {
    use crate::draft::clippy_feed::ClippyDiagnostic;
    use crate::draft::latent_matcher::LatentMatcher;
    use crate::DomainId;
    use crate::domains::clippy_lints::proposers::{FIXABLE_LINTS, VERIFY_LINTS};

    const SRC: &str = "pub fn area(w: f64, h: f64) -> f64 {\n    return w * h;\n}\n";

    // A leased epoch carrying ONE rule — `needless_return` is NOT in it,
    // which is precisely the lease-lag state the seam exists to name.
    let overlay = vec![(
        DomainId::Clippy,
        vec![(
            "lease-fixture-rule".to_string(),
            "fn f() -> u8 { return 7; }".to_string(),
            "fn f() -> u8 { 7 }".to_string(),
        )],
    )];
    let mut m = LatentMatcher::from_enabled_domains_with_overlay(&overlay).expect("build");
    assert!(
        m.rule_book("needless_return").is_none(),
        "precondition: the lagging lease must not carry the lint"
    );

    let spans = m.suggest_file_anchored(
        SRC,
        "sample.rs",
        3,
        &[ClippyDiagnostic {
            span: None,
            file: "sample.rs".to_string(),
            line: 2,
            lint: "needless_return".to_string(),
            suggestion: None,
        }],
    );
    let per_span: Vec<&[String]> = spans.iter().map(|s| s.anchored_missing.as_slice()).collect();
    assert!(
        per_span.iter().any(|m| m.iter().any(|l| l == "needless_return")),
        "the producer must mark the lagging lint [no corpus rule]"
    );

    let fixable: Vec<String> = FIXABLE_LINTS.iter().map(|s| (*s).to_string()).collect();
    let verify: Vec<String> = VERIFY_LINTS.iter().map(|s| (*s).to_string()).collect();
    let d = delta(per_span, &fixable, &verify);

    assert_eq!(d.delta_lints(), 1, "the fix lane carries it — this is the seam");
    assert_eq!(d.rows[0].lint, "needless_return");
    assert_eq!(d.rows[0].tier, FixTier::Fixable);
    assert_eq!(d.unfixable, 0, "it is NOT a coverage gap");
    let line = d.summary_line().expect("a real delta must announce itself");
    assert!(line.contains("the leased corpus lags"), "{line}");
}

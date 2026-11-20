//! Plan 392 Phase 1 gate battery — tests the REAL pipeline
//! (`riir_refine::epiplexity_census`), never a copy: the fixture decode
//! (138 rows × 8 domains, accuracy bounds), full-pipeline determinism
//! (bit-identical weights across two runs), the queue's ordering shape, the
//! DEMOTED / SKIP-SELECTION decision functions, and the loader's typed
//! refusal battery.
#![cfg(feature = "epiplexity_census")]

use std::path::{Path, PathBuf};

use riir_refine::epiplexity_census::{
    CensusReport, DOMINANCE_SKIP_WEIGHT, QueueEntry, demoted_domains, dominance_skip, load_panel,
    run_census,
};

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".benchmarks/mining_gain_panel.jsonl")
}

// ── (a) fixture decode ──────────────────────────────────────────────────

#[test]
fn fixture_decodes_138_rows_8_domains_in_accuracy_bounds() {
    let data = load_panel(&fixture_path()).expect("committed fixture must decode");
    assert_eq!(data.domains.len(), 8, "fixture declares 8 domains");
    assert_eq!(data.counts_rows.len(), 138, "fixture holds 138 data rows");
    assert_eq!(data.ts.len(), 138, "timestamps aligned with rows");
    assert_eq!(data.accuracy.len(), 138, "accuracy aligned with rows");
    for (i, row) in data.counts_rows.iter().enumerate() {
        assert_eq!(row.len(), 8, "row {i} width must match the header's 8");
    }
    for &a in &data.accuracy {
        assert!(a > 0.0 && a < 1.0, "accuracy bounds (0, 1): got {a}");
    }
}

// ── (b) + (d) full-pipeline determinism + shape ─────────────────────────

fn assert_reports_bit_identical(a: &CensusReport, b: &CensusReport) {
    assert_eq!(a.domains, b.domains);
    assert_eq!(a.rows, b.rows);
    assert_eq!(a.arm, b.arm);
    assert_eq!(a.census.domains, b.census.domains);
    assert_eq!(a.census.bar, b.census.bar);
    assert_eq!(a.census.eff10, b.census.eff10);
    assert_eq!(a.census.eff1, b.census.eff1);
    assert_eq!(
        a.census.participation_ratio.to_bits(),
        b.census.participation_ratio.to_bits(),
        "participation ratio bits"
    );

    assert_eq!(a.table.len(), b.table.len());
    for (ra, rb) in a.table.iter().zip(b.table.iter()) {
        assert_eq!(ra.name, rb.name);
        assert_eq!(ra.curve, rb.curve);
        assert_eq!(
            ra.counts_now.to_bits(),
            rb.counts_now.to_bits(),
            "counts bits"
        );
        match (ra.epsilon_floor, rb.epsilon_floor) {
            (None, None) => {}
            (Some(x), Some(y)) => assert_eq!(x.to_bits(), y.to_bits(), "eps bits"),
            _ => panic!("eps_floor presence disagrees"),
        }
        match (ra.headroom, rb.headroom) {
            (None, None) => {}
            (Some(x), Some(y)) => assert_eq!(x.to_bits(), y.to_bits(), "headroom bits"),
            _ => panic!("headroom presence disagrees"),
        }
        match (ra.marginal_gain, rb.marginal_gain) {
            (None, None) => {}
            (Some(x), Some(y)) => assert_eq!(x.to_bits(), y.to_bits(), "gain bits"),
            _ => panic!("marginal_gain presence disagrees"),
        }
        assert_eq!(ra.weight.to_bits(), rb.weight.to_bits(), "weight bits");
    }

    assert_eq!(a.queue.len(), b.queue.len());
    for (qa, qb) in a.queue.iter().zip(b.queue.iter()) {
        assert_eq!(qa.name, qb.name);
        assert_eq!(qa.allocable, qb.allocable);
        assert_eq!(
            qa.weight.to_bits(),
            qb.weight.to_bits(),
            "queue weight bits"
        );
    }

    assert_eq!(a.demoted, b.demoted);
    assert_eq!(a.dominance_skip, b.dominance_skip);
    assert_eq!(a.tau.to_bits(), b.tau.to_bits());

    assert_eq!(a.history.len(), b.history.len());
    for (ha, hb) in a.history.iter().zip(b.history.iter()) {
        assert_eq!(ha.idx, hb.idx);
        assert_eq!(ha.ts, hb.ts);
        assert_eq!(ha.top, hb.top);
        assert_eq!(ha.second, hb.second);
        assert_eq!(ha.top_weight.to_bits(), hb.top_weight.to_bits());
        assert_eq!(ha.second_weight.to_bits(), hb.second_weight.to_bits());
    }
}

#[test]
fn full_pipeline_is_bit_identical_across_runs() {
    let a = run_census(&fixture_path(), false).expect("census runs");
    let b = run_census(&fixture_path(), false).expect("census runs");
    assert_reports_bit_identical(&a, &b);

    let ah = run_census(&fixture_path(), true).expect("census with history runs");
    let bh = run_census(&fixture_path(), true).expect("census with history runs");
    assert_reports_bit_identical(&ah, &bh);
    assert!(
        !ah.history.is_empty(),
        "--history produces lines on 138 rows"
    );
    assert_eq!(ah.rows, 138);
    assert_eq!(ah.table.len(), 8);
    assert_eq!(ah.queue.len(), 8);
}

#[test]
fn queue_orders_fitted_by_weight_then_name_with_abstains_last() {
    let r = run_census(&fixture_path(), false).expect("census runs");
    let first_abstain = r.queue.iter().position(|e: &QueueEntry| !e.allocable);
    if let Some(pos) = first_abstain {
        assert!(
            r.queue[pos..].iter().all(|e| !e.allocable),
            "abstains never precede allocable entries"
        );
        for pair in r.queue[..pos].windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            assert!(
                a.weight >= b.weight,
                "fitted queue descends by weight: {} {:.6} then {} {:.6}",
                a.name,
                a.weight,
                b.name,
                b.weight
            );
            if a.weight.to_bits() == b.weight.to_bits() {
                assert!(a.name <= b.name, "weight ties resolve to name order");
            }
        }
        for pair in r.queue[pos..].windows(2) {
            assert!(pair[0].name <= pair[1].name, "abstains sort by name");
        }
    }
    // the table is name-sorted regardless
    for pair in r.table.windows(2) {
        assert!(pair[0].name <= pair[1].name, "table sorts by domain name");
    }
}

// ── the gate decisions as pure functions ────────────────────────────────

#[test]
fn demoted_domains_gates_strictly_under_the_bar() {
    let names = ["b", "a", "c", "d"];
    // at the bar (0.005) is NOT demoted — strictly under is
    let at_bar = [Some(0.001), Some(0.005), Some(0.006), None];
    assert_eq!(demoted_domains(&at_bar, &names), vec!["b".to_string()]);
    // multiple demotions answer in ascending name order
    let two = [Some(0.004), Some(0.001), Some(0.006), None];
    assert_eq!(
        demoted_domains(&two, &names),
        vec!["a".to_string(), "b".to_string()]
    );
    // None headroom (NoCurve) is an abstain, never a demotion
    let nocurve = [None, None, Some(0.001), None];
    assert_eq!(demoted_domains(&nocurve, &names), vec!["c".to_string()]);
    // empty panel → no demotions
    assert!(demoted_domains(&[], &[]).is_empty());
}

#[test]
fn dominance_skip_fires_at_or_above_the_bar_and_breaks_ties_by_name() {
    let names = ["x", "y", "z"];
    // under the bar → None
    assert_eq!(dominance_skip(&[0.5, 0.3, 0.2], &names), None);
    // exactly at the bar fires (the gate is ≥)
    let at_bar: f32 = DOMINANCE_SKIP_WEIGHT as f32;
    assert_eq!(
        dominance_skip(&[at_bar, 1.0 - at_bar, 0.0], &names),
        Some("x".to_string())
    );
    // clearly over the bar
    assert_eq!(
        dominance_skip(&[0.86, 0.1, 0.04], &names),
        Some("x".to_string())
    );
    // a tie at the top resolves to the lexicographically smallest name
    // (unnormalized input is fine — the gate is pure over the arrays it is
    // handed; a normalized pair cannot both sit at ≥ 0.85)
    assert_eq!(
        dominance_skip(&[0.9, 0.9, 0.1], &["z", "a", "m"]),
        Some("a".to_string())
    );
    // empty panel → None
    assert_eq!(dominance_skip(&[], &[]), None);
}

// ── the loader's typed refusal battery ──────────────────────────────────

/// Temp fixtures are pid+tag-suffixed (`scripts/shared_temp_path_gate`'s
/// law: a fixed temp path is safe against sibling tests in one binary, not
/// against another PROCESS running the same test).
fn temp_panel(tag: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "p392_census_gate_{}_{}.jsonl",
        tag,
        std::process::id()
    ));
    std::fs::write(&path, body).expect("write temp fixture");
    path
}

const HEADER: &str = r#"{"format": "mining_gain_panel_v1", "domains": ["a", "b"], "note": "gate"}"#;

#[test]
fn loader_refuses_a_missing_file_typed() {
    let missing = std::env::temp_dir().join(format!(
        "p392_census_gate_missing_{}.jsonl",
        std::process::id()
    ));
    match load_panel(&missing) {
        Err(riir_refine::epiplexity_census::PanelError::Io(_)) => {}
        other => panic!("expected Io refusal, got {other:?}"),
    }
}

#[test]
fn loader_refuses_an_unknown_format_typed() {
    let body = "{\"format\": \"other_v9\", \"domains\": [\"a\"]}\n";
    let path = temp_panel("fmt", body);
    match load_panel(&path) {
        Err(riir_refine::epiplexity_census::PanelError::UnknownFormat { got }) => {
            assert_eq!(got, "other_v9");
        }
        other => panic!("expected UnknownFormat refusal, got {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loader_refuses_a_width_mismatch_typed() {
    let body =
        format!("{HEADER}\n{{\"ts\": \"t0\", \"counts\": [1.0, 2.0, 3.0], \"accuracy\": 0.5}}\n");
    let path = temp_panel("width", &body);
    match load_panel(&path) {
        Err(riir_refine::epiplexity_census::PanelError::WidthMismatch { line, got, want }) => {
            assert_eq!(line, 2);
            assert_eq!(got, 3);
            assert_eq!(want, 2);
        }
        other => panic!("expected WidthMismatch refusal, got {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loader_refuses_an_out_of_range_float_typed() {
    // serde_json's number grammar refuses out-of-range floats itself — JSON
    // cannot carry a non-finite value into the loader at all, so the
    // observable typed refusal is MalformedRow. The loader's own NonFinite
    // check stays as defense-in-depth for a future parser swap.
    let body =
        format!("{HEADER}\n{{\"ts\": \"t0\", \"counts\": [1e400, 2.0], \"accuracy\": 0.5}}\n");
    let path = temp_panel("nonfinite", &body);
    match load_panel(&path) {
        Err(riir_refine::epiplexity_census::PanelError::MalformedRow { line, .. }) => {
            assert_eq!(line, 2);
        }
        other => panic!("expected MalformedRow refusal, got {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loader_refuses_a_header_only_file_typed() {
    let path = temp_panel("norows", &format!("{HEADER}\n"));
    match load_panel(&path) {
        Err(riir_refine::epiplexity_census::PanelError::NoRows) => {}
        other => panic!("expected NoRows refusal, got {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn loader_refuses_a_malformed_row_typed() {
    let body = format!("{HEADER}\nnot json at all\n");
    let path = temp_panel("malformed", &body);
    match load_panel(&path) {
        Err(riir_refine::epiplexity_census::PanelError::MalformedRow { line, .. }) => {
            assert_eq!(line, 2);
        }
        other => panic!("expected MalformedRow refusal, got {other:?}"),
    }
    let _ = std::fs::remove_file(&path);
}

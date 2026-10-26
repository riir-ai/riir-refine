//! Blueprint Phase A tests (proposal 024 / issue 178) — A4 (G1 contract:
//! the append is the sole side-effect, demote = zero writes, the report is
//! deterministic over its inputs) + A5 (the volume-close liveness: the line
//! fires below the thresholds at week 8, prints its explicit not-tripped
//! form above them, and NEVER silently vanishes past the window).
//!
//! The temp-dir shape is pid+name+line scoped — the shared-temp-path rule
//! (a fixed path is safe against sibling tests in one binary, not against
//! another PROCESS running the same tests).

use super::*;
use std::path::PathBuf;

fn tmp_root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "riir_blueprint_{}_{}_{}",
        std::process::id(),
        name,
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn posture(verify: bool) -> BlueprintPosture {
    BlueprintPosture {
        verify,
        verify_args: None,
        scoped_recheck: true,
        repo_posture: false,
        fmt: true,
        domain: "auto".to_string(),
        top_k: 8,
    }
}

fn mk_row(ts: u64, verify: bool) -> BlueprintRow {
    BlueprintRow {
        v: ROW_VERSION,
        ts,
        digest: "d".repeat(32),
        mode: "fix".to_string(),
        arm: "incumbent".to_string(),
        posture: posture(verify),
        findings: BlueprintFindings::default(),
        outcomes: BlueprintOutcomes::default(),
        strata: BTreeMap::new(),
    }
}

// ── G1: the append lane ──────────────────────────────────────────────────

#[test]
fn demote_env_gate_is_demote_only() {
    assert!(lane_enabled_from(None), "unset = on");
    assert!(lane_enabled_from(Some("1")), "non-0 = on");
    assert!(lane_enabled_from(Some("false")), "any non-0 literal = on");
    assert!(
        !lane_enabled_from(Some("0")),
        "the exact literal 0 = demoted"
    );
}

#[test]
fn demoted_gate_never_creates_the_history() {
    let root = tmp_root("demoted");
    assert!(!root.exists(), "test precondition: no root dir at all");
    let written = maybe_append_row_gated(false, &root, &mk_row(1, true)).expect("gate");
    assert!(!written, "demoted = no write");
    assert!(
        !root.join(HISTORY_RELPATH).exists(),
        "G1: demoted ⇒ zero writes — the file is never created"
    );
}

#[test]
fn active_gate_appends_one_bounded_line_and_roundtrips() {
    let root = tmp_root("append");
    let mut row = mk_row(1_700_000_000, true);
    for i in 0..5 {
        row.strata.insert(
            format!("rule_{i}"),
            RuleOutcome {
                written: i,
                reverted: 0,
                declined: 0,
            },
        );
    }
    let written = maybe_append_row_gated(true, &root, &row).expect("append");
    assert!(written);
    let path = root.join(HISTORY_RELPATH);
    let text = std::fs::read_to_string(&path).expect("history exists");
    assert_eq!(text.lines().count(), 1, "ONE line per campaign");
    assert!(
        text.len() < 8 * 1024,
        "bounded row (got {} bytes)",
        text.len()
    );
    let (rows, bad) = load_rows(&path);
    assert_eq!(bad, 0);
    assert_eq!(rows, vec![row], "byte round-trip through the wire");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn malformed_lines_are_skipped_and_counted() {
    let root = tmp_root("malformed");
    let path = root.join(HISTORY_RELPATH);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(
            "not json\n{{\"v\":1}}\n{}\n\n",
            serde_json::to_string(&mk_row(5, true)).unwrap()
        ),
    )
    .unwrap();
    let (rows, bad) = load_rows(&path);
    assert_eq!(bad, 2, "two malformed lines counted");
    assert_eq!(rows.len(), 1, "the good row survives; blank lines ignored");
    let _ = std::fs::remove_dir_all(&root);
}

// ── digest + strata cap ──────────────────────────────────────────────────

#[test]
fn digest_is_stable_and_content_sensitive() {
    let p = posture(true);
    let a = campaign_digest(
        "fix",
        &p,
        &[PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")],
    );
    let b = campaign_digest(
        "fix",
        &p,
        &[PathBuf::from("src/b.rs"), PathBuf::from("src/a.rs")],
    );
    assert_eq!(a, b, "path order must not move the digest");
    assert_eq!(a.len(), 32, "16 truncated bytes hex");
    let c = campaign_digest("fix", &p, &[PathBuf::from("src/other.rs")]);
    assert_ne!(a, c, "a different campaign is a different digest");
    let mut wide = p.clone();
    wide.verify = false;
    assert_ne!(
        a,
        campaign_digest(
            "fix",
            &wide,
            &[PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")]
        ),
        "posture is part of the digest"
    );
}

#[test]
fn strata_cap_keeps_the_top_totals() {
    let mut strata = BTreeMap::new();
    for i in 0..40u32 {
        strata.insert(
            format!("rule_{i:02}"),
            RuleOutcome {
                written: i,
                reverted: 0,
                declined: 0,
            },
        );
    }
    let capped = cap_strata(strata);
    assert_eq!(capped.len(), MAX_STRATA);
    // The top-total rules survive; the smallest do not.
    assert!(capped.contains_key("rule_39"));
    assert!(!capped.contains_key("rule_00"));
    assert!(
        capped.contains_key("rule_08"),
        "the cutoff lands at the top 32"
    );
}

// ── A5: the volume close liveness ────────────────────────────────────────

const WEEK: u64 = 7 * 24 * 3600;

#[test]
fn volume_close_fires_below_both_thresholds_at_week_eight() {
    let first = 1_700_000_000;
    let rows: Vec<_> = (0..10).map(|i| mk_row(first + i, true)).collect();
    let status = volume_status(&rows, first + 8 * WEEK);
    assert_eq!(
        status,
        VolumeStatus::Close {
            total: 10,
            qualified: 0
        },
        "10 campaigns, one incumbent posture: below 16 AND zero qualified classes"
    );
    let line = volume_line(&status).expect("close prints");
    assert!(line.contains("META-VOLUME-CLOSE"), "{line}");
    assert!(line.contains("insufficient volume"), "{line}");
}

#[test]
fn volume_stays_open_with_a_qualified_class_and_enough_campaigns() {
    let first = 1_700_000_000;
    // 8 verify-on + 8 verify-off campaigns: the verify-posture class holds
    // 2 arms × 8 ≥ (MIN_PER_ARM=6, MIN_ARMS=2) — qualified — and total 16.
    let mut rows: Vec<_> = (0..8).map(|i| mk_row(first + i, true)).collect();
    rows.extend((0..8).map(|i| mk_row(first + 100 + i, false)));
    let status = volume_status(&rows, first + 8 * WEEK + 1);
    assert_eq!(
        status,
        VolumeStatus::Open {
            total: 16,
            qualified: 1
        }
    );
    let line = volume_line(&status).expect("open prints past the window");
    assert!(line.contains("META-VOLUME-OPEN"), "{line}");
    assert!(line.contains("not met"), "{line}");
}

#[test]
fn fifteen_campaigns_with_a_qualified_class_still_close() {
    let first = 1_700_000_000;
    let mut rows: Vec<_> = (0..8).map(|i| mk_row(first + i, true)).collect();
    rows.extend((0..7).map(|i| mk_row(first + 100 + i, false)));
    assert_eq!(
        volume_status(&rows, first + 8 * WEEK),
        VolumeStatus::Close {
            total: 15,
            qualified: 1
        },
        "the 16-campaign floor binds even with a qualified class"
    );
}

#[test]
fn no_line_before_the_window_or_on_no_rows() {
    let first = 1_700_000_000;
    let rows: Vec<_> = (0..20).map(|i| mk_row(first + i, true)).collect();
    assert!(matches!(
        volume_status(&rows, first + WEEK),
        VolumeStatus::Window { .. }
    ));
    assert!(volume_line(&volume_status(&rows, first + WEEK)).is_none());
    assert_eq!(volume_status(&[], first + 9 * WEEK), VolumeStatus::NoRows);
    assert!(volume_line(&VolumeStatus::NoRows).is_none());
}

#[test]
fn past_the_window_the_volume_line_can_never_silently_vanish() {
    // The cannot-silently-not-fire assertion: past week 8, EVERY status the
    // arithmetic can produce renders a META-VOLUME line.
    let first = 1_700_000_000;
    let below: Vec<_> = (0..10).map(|i| mk_row(first + i, true)).collect();
    let mut above: Vec<_> = (0..8).map(|i| mk_row(first + i, true)).collect();
    above.extend((0..8).map(|i| mk_row(first + 100 + i, false)));
    for rows in [&below, &above] {
        let status = volume_status(rows, first + 8 * WEEK + 1);
        let rendered = render_report(rows, 0, &[], false, &status);
        assert!(
            rendered.contains("META-VOLUME"),
            "past the window the report must carry the volume line: {status:?}"
        );
    }
}

// ── A2/A3: the report ────────────────────────────────────────────────────

#[test]
fn report_is_deterministic_and_carries_the_audit_split() {
    let first = 1_700_000_000;
    let mut rows: Vec<_> = (0..8).map(|i| mk_row(first + i, true)).collect();
    rows.extend((0..4).map(|i| mk_row(first + 100 + i, false)));
    let sugg = vec![DemotionSuggestion {
        domain: "clippy".to_string(),
        rule: "map_clone".to_string(),
        verdict: "DECORATIVE".to_string(),
        delta_resolved: 0,
    }];
    let a = render_report(&rows, 1, &sugg, false, &volume_status(&rows, first + WEEK));
    let b = render_report(&rows, 1, &sugg, false, &volume_status(&rows, first + WEEK));
    assert_eq!(a, b, "same inputs → byte-identical report");
    assert!(a.contains("blueprint report: 12 campaign(s)"), "{a}");
    assert!(a.contains("1 malformed line(s) skipped"), "{a}");
    assert!(
        a.contains("verify=true"),
        "the audit split names the posture"
    );
    assert!(a.contains("verify=false"));
    assert!(a.contains("arm: incumbent=12"), "{a}");
    assert!(
        !a.contains("META-VOLUME"),
        "inside the window no volume line prints"
    );
    assert!(a.contains("demote-suggest: clippy:map_clone"), "{a}");
    assert!(a.contains("--active-set-demote clippy:map_clone"), "{a}");
    assert!(
        a.contains("REPORT-ONLY"),
        "the join never reads as actionable"
    );
}

#[test]
fn empty_and_missing_histories_render_honest_zeros() {
    let out = render_report(&[], 0, &[], true, &VolumeStatus::NoRows);
    assert!(out.contains("0 campaign(s)"), "{out}");
    assert!(out.contains("no necessity history"), "{out}");
    assert!(!out.contains("META-VOLUME"));
}

#[test]
fn already_demoted_rules_do_not_re_suggest() {
    let wire = vec![NecessityWire {
        rule: "map_clone".to_string(),
        arm: "disabled".to_string(),
        oracle: "clippy-v1".to_string(),
        verdict: "DECORATIVE".to_string(),
        delta_resolved: 0,
    }];
    let mut active = crate::active_set::ActiveSet::empty();
    assert_eq!(demotion_suggestions(&wire, &active).len(), 1);
    active.demote("clippy", "map_clone");
    assert!(
        demotion_suggestions(&wire, &active).is_empty(),
        "a demoted rule never re-suggests"
    );
}

#[test]
fn swap_arms_and_load_bearing_verdicts_never_suggest() {
    let wire = vec![
        NecessityWire {
            rule: "r1".to_string(),
            arm: "swap:donor".to_string(),
            oracle: "clippy-v1".to_string(),
            verdict: "UNMOVED".to_string(),
            delta_resolved: 0,
        },
        NecessityWire {
            rule: "r2".to_string(),
            arm: "disabled".to_string(),
            oracle: "clippy-v1".to_string(),
            verdict: "LOAD-BEARING".to_string(),
            delta_resolved: 5,
        },
        NecessityWire {
            rule: "r3".to_string(),
            arm: "disabled".to_string(),
            oracle: "clippy-v1".to_string(),
            verdict: "NO-EVIDENCE".to_string(),
            delta_resolved: 0,
        },
    ];
    let active = crate::active_set::ActiveSet::empty();
    let out = demotion_suggestions(&wire, &active);
    assert_eq!(out.len(), 1, "only the removable disabled arm survives");
    assert_eq!(out[0].rule, "r3");
    assert_eq!(out[0].domain, "clippy");
}

#[cfg(feature = "score_bench")]
#[test]
fn necessity_row_mirror_tracks_the_authority_wire() {
    // The ungated wire mirror must keep parsing the real module's rows —
    // a renamed/retyped field there is a silent join death here otherwise.
    let real = crate::score_bench::necessity::NecessityRow {
        ts: "2026-10-27T00:00:00Z".to_string(),
        sha: "abc".to_string(),
        rule: "map_clone".to_string(),
        arm: "disabled".to_string(),
        oracle: "clippy-v1".to_string(),
        fixtures: 3,
        solo_fixtures: 3,
        solo_resolved: 2,
        baseline_resolved: 10,
        arm_resolved: 10,
        delta_resolved: 0,
        baseline_created: 0,
        arm_created: 0,
        delta_created: 0,
        baseline_edits: 4,
        arm_edits: 4,
        co_occurrence: 0,
        donor: None,
        verdict: "DECORATIVE".to_string(),
    };
    let line = serde_json::to_string(&real).unwrap();
    let mirror: NecessityWire = serde_json::from_str(&line).expect("mirror parses the real wire");
    assert_eq!(mirror.rule, "map_clone");
    assert_eq!(mirror.arm, "disabled");
    assert_eq!(mirror.verdict, "DECORATIVE");
    assert_eq!(mirror.delta_resolved, 0);
}

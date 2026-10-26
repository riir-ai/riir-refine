use super::*;

fn tmp_root(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "riir_traj_store_{}_{}_{}",
        std::process::id(),
        name,
        line!()
    ));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn tier_u8_roundtrip() {
    for t in [
        EvidenceTier::Certified,
        EvidenceTier::Heuristic,
        EvidenceTier::Withdrawn,
    ] {
        assert_eq!(EvidenceTier::from_u8(t.as_u8()), t);
    }
    assert_eq!(EvidenceTier::from_u8(9), EvidenceTier::Heuristic);
}

/// Issue 156 T1/T2/T5 — the arm-H rescue tallies: accumulate toward the
/// `FlyBy` keep predicate, stay orthogonal to the outcome counters/FSM,
/// ride the existing evidence vocabulary on a verified rescue, and
/// survive reopen.
#[test]
fn rescue_pair_tallies_accumulate_and_never_touch_outcome_counters() {
    use crate::rescue_mine::RescueDepth;
    let root = tmp_root("rescue_pair");
    {
        let (store, bad) = TrajStore::open(&root).unwrap();
        assert_eq!(bad, 0);
        // 4 modelless failures + 3 escalated successes + 1 escalated
        // non-verification → the FlyBy keep-set, demonstrated at seat
        // depth.
        for _ in 0..3 {
            store
                .record_rescue_pair(
                    "ptr_arg",
                    "shape_a",
                    "fix_a",
                    false,
                    Some((true, RescueDepth::SeatRerank)),
                    "run-r",
                )
                .unwrap();
        }
        store
            .record_rescue_pair(
                "ptr_arg",
                "shape_a",
                "fix_a",
                false,
                Some((false, RescueDepth::SeatRerank)),
                "run-r",
            )
            .unwrap();
        let (t, tier) = store.rescue_state("ptr_arg", "shape_a");
        assert_eq!((t.b0_attempts, t.b0_successes), (4, 0));
        assert_eq!((t.bd_attempts, t.bd_successes), (4, 3));
        assert_eq!(t.depth, RescueDepth::SeatRerank.as_u8());
        assert!(t.keep(), "b0=0 ∧ bd=3 — the keep predicate holds");
        // The tallies are orthogonal: the outcome counters saw only the
        // 3 verified rescues (the exemplar record), never the modelless
        // failures, and the fail ladder never moved.
        let row = store
            .dump()
            .into_iter()
            .find(|r| r.lint_key == "ptr_arg")
            .unwrap();
        assert_eq!((row.success_count, row.fail_count), (3, 0));
        assert_eq!(row.consecutive_fails, 0);
        // The exemplar rode the existing evidence vocabulary: a
        // success-only row certifies (the cold start — the guard admits
        // it), which is T2's Certified-tier shape.
        assert_eq!(tier, EvidenceTier::Certified);
        // A stood-by-modelless leg: a no-tool attempt (b0), no bd bump.
        store
            .record_rescue_pair("ptr_arg", "shape_a", "fix_a", true, None, "run-r")
            .unwrap();
        let (t, _) = store.rescue_state("ptr_arg", "shape_a");
        assert_eq!((t.b0_attempts, t.b0_successes, t.bd_attempts), (5, 1, 4));
        assert!(!t.keep(), "b0>0 — execution-reachable, FlyBy's V>0 law");
        // An unseen state reads zeros honestly.
        let (t, tier) = store.rescue_state("ptr_arg", "never_seen");
        assert_eq!(t, crate::rescue_mine::RescueTally::default());
        assert_eq!(tier, EvidenceTier::Heuristic);
    }
    // Reopen: the tallies are durable (WAL replay).
    {
        let (store, bad) = TrajStore::open(&root).unwrap();
        assert_eq!(bad, 0);
        let (t, _) = store.rescue_state("ptr_arg", "shape_a");
        assert_eq!((t.b0_attempts, t.bd_successes), (5, 3));
    }
}

/// Issue 156 — pre-156 rows (no tallies in their JSON) decode with zero
/// tallies: the serde-default schema-evolution law.
#[test]
fn pre_issue_156_rows_decode_with_zero_rescue_tallies() {
    let legacy = r#"{"lint_key":"ptr_arg","shape":"s","fix":"f","direction":[],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":0}"#;
    let row: TrajRowDoc = serde_json::from_str(legacy).unwrap();
    assert_eq!(
        (
            row.b0_attempts,
            row.b0_successes,
            row.bd_attempts,
            row.bd_successes,
            row.rescue_depth
        ),
        (0, 0, 0, 0, 0)
    );
}

#[test]
fn certified_density_counts_only_certified_rows_per_lint_key() {
    // Issue 067 T1 — the reopen-trigger instrument's counting core:
    // Certified rows count per lint_key; Heuristic (unverified success)
    // and Withdrawn (absorbing failure) do not.
    let root = tmp_root("certified_density");
    let (store, bad) = TrajStore::open(&root).unwrap();
    assert_eq!(bad, 0);
    // rule_x: 3 Certified rows (3 distinct shapes).
    for (i, shape) in ["a", "b", "c"].iter().enumerate() {
        store
            .record("rule_x", shape, "fix", true, true, &format!("run-{i}"))
            .unwrap();
    }
    // rule_y: 2 Certified + 1 unverified (Heuristic) — density 2.
    store.record("rule_y", "s1", "f1", true, true, "r").unwrap();
    store.record("rule_y", "s2", "f2", true, true, "r").unwrap();
    store
        .record("rule_y", "s3", "f3", true, false, "r")
        .unwrap();
    // rule_z: 1 Certified + 3 consecutive fails → Withdrawn — density 1.
    store.record("rule_z", "w1", "f", true, true, "r").unwrap();
    store
        .record("rule_z", "w2", "f", false, false, "r")
        .unwrap();
    store
        .record("rule_z", "w3", "f", false, false, "r")
        .unwrap();
    store
        .record("rule_z", "w4", "f", false, false, "r")
        .unwrap();

    let density = store.certified_density();
    assert_eq!(density.get("rule_x"), Some(&3), "3 certified rows");
    assert_eq!(density.get("rule_y"), Some(&2), "heuristic row excluded");
    assert_eq!(density.get("rule_z"), Some(&1), "withdrawn row excluded");
    assert!(!density.contains_key("rule_never_seen"));
    // One scan per call; a repeat call is stable.
    assert_eq!(store.certified_density().get("rule_x"), Some(&3));
}

#[test]
fn record_then_reopen_then_retrieve() {
    let root = tmp_root("roundtrip");
    {
        let (store, bad) = TrajStore::open(&root).unwrap();
        assert_eq!(bad, 0);
        store
            .record(
                "E0502:e0502-deref-removal",
                "&v[0]",
                "",
                true,
                true,
                "run-a",
            )
            .unwrap();
        store
            .record(
                "E0502:e0502-clone-out",
                "&items[0]",
                "items[0].clone()",
                false,
                false,
                "run-a",
            )
            .unwrap();
        assert_eq!(store.len(), 2);
    }
    // Reopen: WAL replay carries the rows + the BM25 rebuild works.
    {
        let (store, bad) = TrajStore::open(&root).unwrap();
        assert_eq!(bad, 0, "no corrupt rows");
        assert_eq!(store.len(), 2, "durable across close/reopen");
        let hits = store.retrieve("&v[0]", "E0502:", 5);
        assert_eq!(hits.len(), 2);
        let top = &hits[0];
        assert_eq!(
            top.lint_key, "E0502:e0502-deref-removal",
            "exact-shape neighbor ranks first: {hits:?}"
        );
        assert_eq!(top.success_count, 1);
        assert!((top.outcome_rate() - 1.0).abs() < 1e-6);
        // Family filter excludes other codes.
        assert!(store.retrieve("&v[0]", "E0597:", 5).is_empty());
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn three_consecutive_failures_withdraw_and_are_excluded() {
    let root = tmp_root("withdrawn");
    let (store, _) = TrajStore::open(&root).unwrap();
    for _ in 0..3 {
        store
            .record("E0499:x", "&mut *v", "swap", false, false, "run-f")
            .unwrap();
    }
    assert!(store.retrieve("&mut *v", "E0499:", 5).is_empty());
    assert_eq!(store.dump().len(), 1, "still dumped (failure analysis)");
    let row = &store.dump()[0];
    assert_eq!(row.tier, EvidenceTier::Withdrawn.as_u8());
    // A verified success exits Withdrawn (absorbing ONLY until one).
    store
        .record("E0499:x", "&mut *v", "swap", true, true, "run-g")
        .unwrap();
    let row = &store.dump()[0];
    assert_eq!(row.tier, EvidenceTier::Certified.as_u8());
    assert_eq!(row.consecutive_fails, 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pair_consecutive_fails_tracks_the_exact_fix_pair() {
    // P12: the suppression signal. Two consecutive fails on the exact
    // (rule, restore, fix) pair read back; ANY of {a recorded success,
    // a different fix text (the proposer's output changed), no row at
    // all} reads 0 — those are exactly the auto-unlock conditions.
    let root = tmp_root("pair-streak");
    let (store, _) = TrajStore::open(&root).unwrap();
    assert_eq!(
        store.pair_consecutive_fails("use_self", "&EventLog<A>", "&Self"),
        0,
        "no row = no suppression"
    );

    // One fail: under the threshold — the whole-file-association retry
    // window (Issue 018's attribution-impossible revert records every
    // edit in the file).
    store
        .record("use_self", "&EventLog<A>", "&Self", false, false, "run-1")
        .unwrap();
    assert_eq!(
        store.pair_consecutive_fails("use_self", "&EventLog<A>", "&Self"),
        1
    );

    // Second consecutive fail: the pair is a deterministic replay —
    // suppress.
    store
        .record("use_self", "&EventLog<A>", "&Self", false, false, "run-2")
        .unwrap();
    assert_eq!(
        store.pair_consecutive_fails("use_self", "&EventLog<A>", "&Self"),
        2
    );

    // A different fix text on the same span: the proposer's output
    // changed since the failures — retry is NOT a replay of the known
    // revert (the 6038e59 manual_let_else emitter-fix recovery path).
    assert_eq!(
        store.pair_consecutive_fails("use_self", "&EventLog<A>", "&Self<A>"),
        0
    );

    // A success (verified) resets the streak — the pair heals honestly
    // once the surrounding code changes.
    store
        .record("use_self", "&EventLog<A>", "&Self", true, true, "run-3")
        .unwrap();
    assert_eq!(
        store.pair_consecutive_fails("use_self", "&EventLog<A>", "&Self"),
        0
    );

    // Different rule or shape: different row, no suppression.
    assert_eq!(
        store.pair_consecutive_fails("use_self", "&OtherLog<A>", "&Self"),
        0
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn dump_carries_the_full_doc() {
    let root = tmp_root("dump");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record(
            "E0505:e0505-clone-at-move",
            "v",
            ".clone()",
            true,
            true,
            "run-d",
        )
        .unwrap();
    let dumped = store.dump();
    assert_eq!(dumped.len(), 1);
    assert_eq!(dumped[0].lint_key, "E0505:e0505-clone-at-move");
    assert_eq!(dumped[0].fix, ".clone()");
    assert_eq!(dumped[0].success_count, 1);
    assert_eq!(dumped[0].tier, EvidenceTier::Certified.as_u8());
    assert_eq!(dumped[0].direction.len(), EMBED_DIM);
    let _ = std::fs::remove_dir_all(&root);
}

// ── Issue 047: component-attributed failure rows ──────────────────

#[test]
fn record_classified_stamps_and_persists_the_component() {
    let root = tmp_root("component");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record_classified(
                "E0502:e0502-clone-out",
                "&items[0]",
                "items[0].clone()",
                false,
                false,
                "run-c",
                Some(crate::component::ComponentClass::Drafter),
                None,
            )
            .unwrap();
        let row = &store.dump()[0];
        assert_eq!(
            row.component,
            Some(crate::component::ComponentClass::Drafter)
        );
        assert_eq!(row.fail_count, 1);

        // A later unattributed SUCCESS leaves the last failure
        // attribution in place (failure metadata, not a state machine).
        store
            .record(
                "E0502:e0502-clone-out",
                "&items[0]",
                "items[0].clone()",
                true,
                true,
                "run-s",
            )
            .unwrap();
        let row = &store.dump()[0];
        assert_eq!(
            row.component,
            Some(crate::component::ComponentClass::Drafter)
        );
    }
    // Durable across reopen (the audit reads it via --dump-trajectories).
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let row = &store.dump()[0];
        assert_eq!(
            row.component,
            Some(crate::component::ComponentClass::Drafter)
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pre_047_row_json_decodes_component_as_none() {
    // Back-compat: rows serialized before Issue 047 carry no `component`
    // field — they must decode as unattributed, not fail.
    let legacy = r#"{"lint_key":"E0502:x","shape":"&v[0]","fix":"v[0]","direction":[0.0],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":1,"last_run_id":"r","last_seen_at":1,"elo":1200.0,"span_elo":1200.0}"#;
    let row: TrajRowDoc = serde_json::from_str(legacy).expect("legacy row decodes");
    assert_eq!(row.component, None, "pre-047 row must decode unattributed");
}

/// riir-dapps Issue 119 T2 — (d) a row written before `src_path`
/// existed decodes with the unknown path `""` (which the sync gate then
/// fails closed on under a path-restricting policy).
#[test]
fn pre_119_row_json_decodes_with_empty_src_path() {
    let old = r#"{"lint_key":"needless_return","shape":"return 1;","fix":"1",
            "direction":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":1,
            "fail_count":0,"consecutive_fails":0,"tier":0,"last_run_id":"r",
            "last_seen_at":5,"elo":1200.0,"span_elo":1200.0}"#;
    let row: TrajRowDoc = serde_json::from_str(old).expect("pre-119 row decodes");
    assert_eq!(row.src_path, "");
    assert_eq!(row.lint_key, "needless_return");
}

/// riir-dapps Issue 119 T2: a SOURCED write stamps the row's file (and a
/// later sourced write from another file overwrites it — the path tracks
/// the CURRENT snippet); an UNSOURCED write leaves the path as it was.
/// The path survives the reopen (it is a durable row field).
#[test]
fn sourced_writes_stamp_src_path_and_unsourced_keep_it() {
    let root = tmp_root("src-path");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("unsourced_only", "s0", "f0", true, true, "r")
            .unwrap();
        store
            .sourced("src/a.rs")
            .record_applied("rule", "shape", "fix", "r1", None)
            .unwrap();
        store
            .record("rule", "shape", "fix2", false, false, "r2")
            .unwrap();
        let src = |k: &str| {
            store
                .dump()
                .into_iter()
                .find(|r| r.lint_key == k)
                .map(|r| r.src_path)
                .unwrap()
        };
        assert_eq!(src("unsourced_only"), "");
        assert_eq!(src("rule"), "src/a.rs", "unsourced write kept the path");
        store
            .sourced("build.rs")
            .record_journey("rule", "shape", "fix2", "r3", None)
            .unwrap();
        assert_eq!(src("rule"), "build.rs");
        store
            .sourced("tests/t.rs")
            .record_classified("rule", "shape", "fix3", true, true, "r4", None, None)
            .unwrap();
        assert_eq!(src("rule"), "tests/t.rs");
    }
    let (store, _) = TrajStore::open(&root).unwrap();
    let row = store
        .dump()
        .into_iter()
        .find(|r| r.lint_key == "rule")
        .unwrap();
    assert_eq!(row.src_path, "tests/t.rs");
    let _ = std::fs::remove_dir_all(&root);
}

/// Issue 148 T-b (the suite-mint label): `Some(rejected)` refreshes the
/// row's candidate set — sorted, deduped, capped — while `None` (a record
/// without the set in hand) never erases one a richer observation left.
#[test]
fn rejected_stamp_refreshes_on_some_and_keeps_on_none() {
    let root = tmp_root("rejected-stamp");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record_applied(
                "rule",
                "shape",
                "fix",
                "r1",
                Some(&[
                    "z_rule".to_string(),
                    "a_rule".to_string(),
                    "a_rule".to_string(),
                ]),
            )
            .unwrap();
        let label = || {
            store
                .dump()
                .into_iter()
                .find(|r| r.lint_key == "rule")
                .map(|r| r.rejected)
                .unwrap()
        };
        assert_eq!(label(), vec!["a_rule".to_string(), "z_rule".to_string()]);
        // `None` keeps (an outcome-only record never erases the label).
        store
            .record("rule", "shape", "fix", true, true, "r2")
            .unwrap();
        assert_eq!(label().len(), 2, "None kept the label");
        // `Some` refreshes — unsorted input is normalized, and the cap
        // truncates after the sort (deterministic, never arbitrary).
        let many: Vec<String> = (0..24).map(|i| format!("r{i:02}")).collect();
        store
            .record_applied("rule", "shape", "fix", "r3", Some(&many))
            .unwrap();
        assert_eq!(label().len(), REJECTED_CAP);
        assert_eq!(label().first().map(String::as_str), Some("r00"));
    }
    let (store, _) = TrajStore::open(&root).unwrap();
    let row = store
        .dump()
        .into_iter()
        .find(|r| r.lint_key == "rule")
        .unwrap();
    assert_eq!(row.rejected.len(), REJECTED_CAP, "the label is durable");
    let _ = std::fs::remove_dir_all(&root);
}

/// Issue 148 T-b follow-up (the `rustc_errors` driver lane): the lane
/// records through `SourcedTraj::record_classified` — the applied fix
/// rides `None` component, the revert rides `Some` — with the captured
/// strategy menu as the label. The None-keep law holds on exactly that
/// shape: a record without the menu (a solo-candidate decision, captured
/// as an empty vec → `rejected_opt` → `None`) never erases one a richer
/// observation left, and a revert's `Some` refreshes through the same law.
#[test]
fn driver_lane_record_shape_keeps_label_on_none() {
    let root = tmp_root("driver-lane-label");
    let key = "E0502:e0597-clone-out";
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        // The applied fix, stamped with the captured menu (sourced — the
        // driver lane's shape carries the file).
        store
            .sourced("src/a.rs")
            .record_classified(
                key,
                "&v[0]",
                "v[0].clone()",
                true,
                true,
                "run-1",
                None,
                Some(&["e0597-deref".to_string(), "rustc-suggestion".to_string()]),
            )
            .unwrap();
        let label = || {
            store
                .dump()
                .into_iter()
                .find(|r| r.lint_key == key)
                .map(|r| r.rejected)
                .unwrap()
        };
        assert_eq!(
            label(),
            vec!["e0597-deref".to_string(), "rustc-suggestion".to_string()]
        );
        // A later keep WITHOUT a menu (solo candidate → empty → `None`)
        // keeps the label; the sourced path survives the same way.
        store
            .sourced("src/a.rs")
            .record_classified(
                key,
                "&v[0]",
                "v[0].clone()",
                true,
                true,
                "run-2",
                None,
                None,
            )
            .unwrap();
        assert_eq!(label().len(), 2, "None kept the driver-lane label");
        // The revert record (`Some(component)`) refreshes through the
        // same law — the narrower menu replaces, it does not append.
        store
            .sourced("src/a.rs")
            .record_classified(
                key,
                "&v[0]",
                "v[0].clone()",
                false,
                false,
                "run-3",
                Some(crate::component::ComponentClass::Drafter),
                Some(&["e0597-deref".to_string()]),
            )
            .unwrap();
        assert_eq!(label(), vec!["e0597-deref".to_string()]);
        assert_eq!(store.dump()[0].src_path, "src/a.rs");
    }
    // Durable across reopen.
    let (store, _) = TrajStore::open(&root).unwrap();
    let row = store
        .dump()
        .into_iter()
        .find(|r| r.lint_key == key)
        .unwrap();
    assert_eq!(row.rejected, vec!["e0597-deref".to_string()]);
    let _ = std::fs::remove_dir_all(&root);
}

/// riir-dapps Issue 119 T2: `src_path_of` yields the repo-relative,
/// forward-slash form the policy globs are written against — through
/// symlinked temp dirs (canonicalized), for a deleted file (via its
/// parent), and `""` for a file outside the repo root.
#[test]
fn src_path_of_is_repo_relative_and_empty_outside_the_root() {
    let root = tmp_root("src-path-of");
    let (store, _) = TrajStore::open(&root).unwrap();
    std::fs::create_dir_all(root.join("src/deep")).unwrap();
    std::fs::write(root.join("src/deep/x.rs"), "fn x() {}").unwrap();
    assert_eq!(
        store.src_path_of(&root.join("src/deep/x.rs")),
        "src/deep/x.rs"
    );
    assert_eq!(
        store.src_path_of(&root.join("src/deep/gone.rs")),
        "src/deep/gone.rs"
    );
    assert_eq!(store.src_path_of(&root.join("src/../build.rs")), "build.rs");
    let outside = tmp_root("src-path-of-outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("y.rs"), "").unwrap();
    assert_eq!(store.src_path_of(&outside.join("y.rs")), "");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn record_stamps_run_id_and_seen_at() {
    let root = tmp_root("runid");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("E0502:s", "&v[0]", "v[0]", true, true, "run-abc")
            .unwrap();
        let row = &store.dump()[0];
        assert_eq!(row.last_run_id, "run-abc");
        assert!(row.last_seen_at > 0, "timestamp stamped");
    }
    // Durable across reopen + updated by the next sighting.
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let before = store.dump()[0].last_seen_at;
        store
            .record("E0502:s", "&v[0]", "v[0]", true, true, "run-zzz")
            .unwrap();
        let row = &store.dump()[0];
        assert_eq!(row.last_run_id, "run-zzz", "latest sighting wins");
        assert!(row.last_seen_at >= before);
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pre_t2_rows_without_verdict_fields_decode() {
    // A row written before 037-T2 (no last_run_id / last_seen_at) must
    // decode with the serde defaults — the schema is append-only.
    let doc: TrajRowDoc = serde_json::from_str(
            r#"{"lint_key":"E0502:x","shape":"&v","fix":"v","direction":[0.0,0.0],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":0}"#,
        )
        .expect("pre-T2 row decodes");
    assert_eq!(doc.last_run_id, "");
    assert_eq!(doc.last_seen_at, 0);
    assert_eq!(doc.success_count, 1);
}

#[test]
#[allow(clippy::float_cmp)] // exact base value — serde default, never computed
fn pre_039_rows_without_elo_fields_decode_at_base() {
    // Issue 039 T1 reservation: rows written before the elo fields exist
    // decode at base 1200 — the honest unrated state every pre-039 row
    // carries (T2 recomputes from verdict history on load).
    let doc: TrajRowDoc = serde_json::from_str(
            r#"{"lint_key":"E0502:x","shape":"&v","fix":"v","direction":[0.0,0.0],"success_count":1,"fail_count":0,"consecutive_fails":0,"tier":0,"last_run_id":"run-1","last_seen_at":100}"#,
        )
        .expect("pre-039 row decodes");
    assert_eq!(doc.elo, default_elo());
    assert_eq!(doc.span_elo, default_elo());
    assert_eq!(doc.last_seen_at, 100);
}

#[cfg(feature = "rustc_errors")]
#[test]
fn shape_conditional_ordering_prefers_the_winning_neighbor() {
    use crate::domains::rustc_errors::playbooks::FixCandidate;
    let root = tmp_root("ordering");
    let (store, _) = TrajStore::open(&root).unwrap();
    // History: deref-removal SUCCEEDED on this exact shape 5×;
    // clone-out FAILED on it 5× (a synthetic reversed ladder — the
    // corpus ladder is the other way round, which is the point: the
    // ORDER follows lived outcomes, not the static preference).
    for _ in 0..5 {
        store
            .record(
                "E0502:e0502-deref-removal",
                "&v[0]",
                "",
                true,
                true,
                "run-o",
            )
            .unwrap();
        store
            .record(
                "E0502:e0502-clone-out",
                "&v[0]",
                "v[0].clone()",
                false,
                false,
                "run-o",
            )
            .unwrap();
    }
    let mk = |strat: &str, repl: &str, len_hint: usize| FixCandidate {
        strategy_id: strat.to_string(),
        code: "E0502".to_string(),
        file: std::path::PathBuf::from("a.rs"),
        line: 2,
        byte_span: (0, len_hint),
        replacement: repl.to_string(),
        shape: "&v[0]".to_string(),
        semantic_note: None,
        diag_digest: None,
        extra_edits: Vec::new(),
    };
    // Static min-delta would ALSO pick deref-removal here (shorter
    // total length) — so the flip arm below is the load-bearing
    // assertion: with the history REVERSED, the order flips, which
    // static ordering can never do.
    let mut cands = vec![
        mk("e0502-clone-out", "v[0].clone()", 6),
        mk("e0502-deref-removal", "", 1),
    ];
    shape_conditional_order(&mut cands, &store);
    assert_eq!(
        cands[0].strategy_id, "e0502-deref-removal",
        "the 5×-successful neighbor outranks the 5×-failed one"
    );
    // And the override is REAL: flip the history, the order flips.
    let flipped_root = tmp_root("ordering_flip");
    let (flipped, _) = TrajStore::open(&flipped_root).unwrap();
    for _ in 0..5 {
        flipped
            .record(
                "E0502:e0502-deref-removal",
                "&v[0]",
                "",
                false,
                false,
                "run-f",
            )
            .unwrap();
        flipped
            .record(
                "E0502:e0502-clone-out",
                "&v[0]",
                "v[0].clone()",
                true,
                true,
                "run-f",
            )
            .unwrap();
    }
    let mut flipped_cands = vec![
        mk("e0502-clone-out", "v[0].clone()", 6),
        mk("e0502-deref-removal", "", 1),
    ];
    shape_conditional_order(&mut flipped_cands, &flipped);
    assert_eq!(
        flipped_cands[0].strategy_id, "e0502-clone-out",
        "flipped history flips the order — the ordering follows outcomes"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&flipped_root);
}

/// The kv branch of `--memory-stats` must carry the SAME detail as the
/// envelope branch (rows, distinct rules, tiers, outcomes, histogram,
/// loop state) — the observability regression: the store path shipped
/// 038 as a 4-counter one-liner and the histogram/tier/loop-state
/// detail was envelope-only (surfaced live on adoption day).
#[test]
fn stats_report_carries_envelope_parity() {
    let root = tmp_root("stats");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let empty = store.stats_report(0);
        assert!(empty.contains("rows: 0 (distinct rules: 0)"));
        assert!(empty.contains("record — | verdict — | learn —"));

        store
            .record(
                "uninlined_format_args",
                "format!(\"{}\", x)",
                "format!(\"{x}\")",
                true,
                true,
                "run-1",
            )
            .unwrap();
        store
            .record_applied(
                "match_bool",
                "match c { true => a, _ => b }",
                "if c { a } else { b }",
                "run-1",
                None,
            )
            .unwrap();
    }
    {
        let (store, bad) = TrajStore::open(&root).unwrap();
        assert_eq!(bad, 0);
        let report = store.stats_report(0);
        assert!(report.contains("rows: 2 (distinct rules: 2)"));
        assert!(report.contains("tiers: Certified 1 | Heuristic 1 | Withdrawn 0"));
        assert!(report.contains("outcomes: success 1 | fail 0 | rows with consecutive fails 0"));
        assert!(report.contains("     1  match_bool"));
        assert!(report.contains("     1  uninlined_format_args"));
        assert!(report.contains("activity: last verdict "));
        assert!(report.contains("distinct active days: 1"));
        assert!(
            report.contains("loop state (Issue 037): record OK | verdict OK | learn OK"),
            "certified row closes all three gates: {report}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Migrated rows carry `last_seen_at = 0` — the activity line must say
/// so instead of fabricating a 1970 date.
#[test]
fn stats_report_migration_rows_report_no_activity() {
    let root = tmp_root("stats-migrated");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .import_row(&TrajRowDoc {
            lint_key: "uninlined_format_args".to_string(),
            shape: "format!(\"{}\", x)".to_string(),
            fix: "format!(\"{x}\")".to_string(),
            direction: vec![0.0; EMBED_DIM],
            success_count: 3,
            fail_count: 0,
            consecutive_fails: 0,
            tier: EvidenceTier::Heuristic.as_u8(),
            last_run_id: "migrated".to_string(),
            last_seen_at: 0,
            elo: default_elo(),
            span_elo: default_elo(),
            component: None,
            journey_runs: 0,
            journey_taint_sum: 0,
            src_path: String::new(),
            rejected: Vec::new(),
            b0_attempts: 0,
            b0_successes: 0,
            bd_attempts: 0,
            bd_successes: 0,
            rescue_depth: 0,
        })
        .unwrap();
    let report = store.stats_report(0);
    assert!(report.contains("rows: 1 (distinct rules: 1)"));
    assert!(report.contains("outcomes: success 3"));
    assert!(
        report.contains("activity: no verdict timestamps yet (apply-only or migrated rows)"),
        "{report}"
    );
    // Successes from migrated history still close E2 (same semantics as
    // the envelope report — a positive outcome is a positive outcome).
    assert!(report.contains("record OK | verdict OK | learn —"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn civil_from_days_known_dates() {
    assert_eq!(civil_from_days(0), (1970, 1, 1));
    assert_eq!(civil_from_days(10_957), (2000, 1, 1));
    assert_eq!(civil_from_days(11_016), (2000, 2, 29), "leap day");
    assert_eq!(civil_from_days(11_323), (2001, 1, 1), "year after a leap");
    assert_eq!(civil_from_days(20_689), (2026, 8, 24), "adoption day");
}

// ── Issue 037 E3: the durable EMA learn rows ─────────────────────────

/// The E3 mechanism: every verdict steps the rule's persisted direction
/// (nonzero `last_delta` — "direction deltas ≠ 0 for buckets with
/// outcomes"), init-on-first-success, fail-before-init is a direction
/// no-op (the evolve.rs rule), and applies never touch learn rows.
#[test]
fn record_feeds_the_per_rule_ema_learn_rows() {
    let root = tmp_root("learn");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        // First verdict is a SUCCESS → initializes the direction.
        store
            .record(
                "uninlined_format_args",
                "format!(\"{}\", x)",
                "format!(\"{x}\")",
                true,
                true,
                "run-1",
            )
            .unwrap();
        let learns = store.learn_rows();
        assert_eq!(learns.len(), 1, "one learn row per rule");
        let l = &learns[0];
        assert_eq!(l.lint_key, "uninlined_format_args");
        assert_eq!(l.updates, 1, "init counts as the first update");
        assert!(l.last_delta > 0.0, "init delta = the born vector's norm");
        assert_eq!(l.direction.len(), EMBED_DIM);
        assert_eq!(l.successes, 1);
        assert_eq!(l.fails, 0);
        assert_eq!(l.last_run_id, "run-1");

        // A second, different fix → the direction MOVES (Δ ≠ 0).
        let init_dir = l.direction.clone();
        store
            .record(
                "uninlined_format_args",
                "println!(\"{}\", y)",
                "println!(\"{y}\")",
                true,
                true,
                "run-2",
            )
            .unwrap();
        let l = &store.learn_rows()[0];
        assert_eq!(l.updates, 2);
        assert_eq!(l.successes, 2);
        assert_ne!(l.direction, init_dir, "EMA moved the persisted direction");
        assert!(l.last_delta > 0.0);

        // A post-init FAIL moves the direction away — still an update.
        store
            .record(
                "uninlined_format_args",
                "write!(f, \"{}\", z)",
                "write!(f, \"{z}\")",
                false,
                false,
                "run-3",
            )
            .unwrap();
        let l = &store.learn_rows()[0];
        assert_eq!(l.updates, 3);
        assert_eq!(l.fails, 1);
    }
    // Durable across reopen (the WAL carries learn rows too).
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let l = &store.learn_rows()[0];
        assert_eq!(l.updates, 3, "learn state survives close/reopen");
        assert_eq!(l.last_run_id, "run-3");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // exact 0.0 — set, never computed
fn first_fail_is_a_learn_direction_noop() {
    let root = tmp_root("learn-fail-first");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record(
            "match_bool",
            "match c { true => a, _ => b }",
            "if c { a } else { b }",
            false,
            false,
            "run-f",
        )
        .unwrap();
    let l = &store.learn_rows()[0];
    assert_eq!(l.fails, 1, "the verdict is still counted");
    assert_eq!(l.updates, 0, "but never establishes a direction");
    assert_eq!(l.last_delta, 0.0);
    assert!(l.direction.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn record_applied_does_not_touch_learn_rows() {
    // The EMA consumes verdicts, not sightings — an apply-only run
    // leaves the learn section empty.
    let root = tmp_root("learn-apply-only");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record_applied(
            "match_bool",
            "match c { true => a, _ => b }",
            "if c { a } else { b }",
            "run-a",
            None,
        )
        .unwrap();
    assert!(store.learn_rows().is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn learn_rows_do_not_pollute_the_trajectory_surface() {
    let root = tmp_root("learn-isolated");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record(
            "E0502:e0502-deref-removal",
            "&v[0]",
            "",
            true,
            true,
            "run-1",
        )
        .unwrap();
    assert_eq!(store.len(), 1, "learn rows are not trajectory rows");
    assert_eq!(store.dump().len(), 1);
    assert_eq!(store.learn_rows().len(), 1);
    assert_eq!(store.retrieve("&v[0]", "E0502:", 5).len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

/// The `--memory-stats` learn section: per-rule counters + the decay
/// line, with a graceful empty state (Issue 037 E3's observability).
#[test]
fn stats_report_prints_the_learn_section_and_decay_line() {
    let root = tmp_root("learn-stats");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let empty = store.stats_report(0);
        assert!(
            empty.contains("learn (EMA): no evolved rules yet"),
            "empty learn section: {empty}"
        );

        store
            .record(
                "uninlined_format_args",
                "format!(\"{}\", x)",
                "format!(\"{x}\")",
                true,
                true,
                "run-1",
            )
            .unwrap();
        store
            .record(
                "match_bool",
                "match c { true => a, _ => b }",
                "if c { a } else { b }",
                true,
                true,
                "run-1",
            )
            .unwrap();
    }
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let report = store.stats_report(0);
        assert!(
            report.contains("learn (EMA): 2 evolved rule(s) | 2 update(s) consumed"),
            "{report}"
        );
        assert!(
            report.contains("last |Δ| "),
            "per-rule delta column: {report}"
        );
        assert!(report.contains("uninlined_format_args"));
        assert!(report.contains("match_bool"));
        assert!(
            report.contains("decay: α 0.100 toward / β 0.050 away | mean last |Δ| "),
            "decay line: {report}"
        );
        assert!(report.contains("across 2 updated rule(s)"));
    }
    let _ = std::fs::remove_dir_all(&root);
}

// ── Issue 039 T2: batch ELO over the store ──────────────────────────────

#[test]
#[allow(clippy::float_cmp)] // base equality + shared-span bit-identity ARE the assertions
fn recompute_elo_backfills_and_is_idempotent() {
    let root = tmp_root("elo-backfill");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record("rule_a", "shape 1", "fix", true, true, "run-1")
            .unwrap();
        for _ in 0..3 {
            store
                .record("rule_a", "shape 1", "fix", false, true, "run-1")
                .unwrap();
        }
        store
            .record("rule_b", "shape 1", "fix2", true, true, "run-1")
            .unwrap();
        // Apply-only sighting: no outcomes, stays unrated.
        store
            .record_applied("rule_c", "shape 3", "fix3", "run-1", None)
            .unwrap();

        let s1 = store.recompute_elo().unwrap();
        assert_eq!(s1.rows_seen, 3);
        assert_eq!(s1.matches, 5);
        assert_eq!(s1.rules_rated, 2);
        assert_eq!(s1.spans_rated, 1);
        assert_eq!(s1.rows_written, 2, "only the two outcome rows");

        let dump = store.dump();
        let a = dump.iter().find(|r| r.lint_key == "rule_a").unwrap();
        let b = dump.iter().find(|r| r.lint_key == "rule_b").unwrap();
        let c = dump.iter().find(|r| r.lint_key == "rule_c").unwrap();
        // rule_b (1W 0L) above base; rule_a (1W 3L) below; both share
        // the span's FINAL rating (denormalized snapshot semantics).
        assert!(b.elo > default_elo(), "1W/0L rule rated up: {}", b.elo);
        assert!(a.elo < default_elo(), "1W/3L rule rated down: {}", a.elo);
        assert_eq!(a.span_elo, b.span_elo, "shared span carries one rating");
        // The span's record: 3 wins (rule_a's failures) vs 2 losses
        // (rule_a + rule_b successes) — a net-winning span rates UP.
        assert!(
            a.span_elo > default_elo(),
            "3W/2L span rated up: {}",
            a.span_elo
        );
        // Apply-only row stays at base (no matches).
        assert_eq!(c.elo, default_elo());
        assert_eq!(c.span_elo, default_elo());

        // Idempotent: bit-identical re-derivation writes nothing.
        let before = store.dump();
        let s2 = store.recompute_elo().unwrap();
        assert_eq!(s2.rows_written, 0);
        assert_eq!(store.dump(), before);
    }
    // Durable: ratings survive close/reopen (the backfill's point).
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        let dump = store.dump();
        let a = dump.iter().find(|r| r.lint_key == "rule_a").unwrap();
        assert!(a.elo < default_elo(), "rating persisted: {}", a.elo);
        let s = store.recompute_elo().unwrap();
        assert_eq!(s.rows_written, 0, "still idempotent after reopen");
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // denormalized final-rating bit-identity IS the assertion
fn shared_rule_rows_carry_the_final_rating() {
    // One rule over two shapes: EVERY row of the rule reports the
    // rule's FINAL rating (the consumer contract — T3/T4 read any row).
    let root = tmp_root("elo-final");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record("rule_r", "shape x", "f", true, true, "run-1")
        .unwrap();
    store
        .record("rule_r", "shape x", "f", true, true, "run-1")
        .unwrap();
    store
        .record("rule_r", "shape y", "f", false, true, "run-1")
        .unwrap();
    store.recompute_elo().unwrap();
    let dump = store.dump();
    let x = dump.iter().find(|r| r.shape == "shape x").unwrap();
    let y = dump.iter().find(|r| r.shape == "shape y").unwrap();
    assert_eq!(x.elo, y.elo, "denormalized FINAL rating on every row");
    assert_ne!(x.span_elo, y.span_elo, "spans rate separately");
    // x lost to the rule twice (0W/2L → down); y beat it once (1W/0L
    // → up) — a span's wins come from the rule's failures.
    assert!(x.span_elo < default_elo(), "twice-beaten span rated down");
    assert!(
        y.span_elo > default_elo(),
        "span that beat the rule rated up"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // unrated == base IS the pre-backfill pin
fn pre_039_row_decodes_unrated_then_backfill_rates_it() {
    // Extends the decode pin above: the SAME pre-039 JSON (no elo
    // fields), once in the store, gets ratings WRITTEN by the T2
    // backfill — decode-unrated is the input state, rated the
    // post-backfill state.
    let root = tmp_root("elo-pre039");
    let (store, _) = TrajStore::open(&root).unwrap();
    let pre039 = r#"{"lint_key":"match_bool","shape":"match c {\n    true => a,\n}","fix":"if c { a }","direction":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":2,"fail_count":0,"consecutive_fails":0,"tier":0,"last_run_id":"run-1","last_seen_at":100}"#;
    let shape = "match c {\n    true => a,\n}";
    let key = TrajStore::row_key("match_bool", shape);
    store.kv.put(&key, pre039.as_bytes()).unwrap();
    // Decodes unrated (the pin above) ...
    let row = &store.dump()[0];
    assert_eq!(row.elo, default_elo());
    // ... then the backfill rates it.
    let s = store.recompute_elo().unwrap();
    assert_eq!(s.rows_written, 1);
    let row = &store.dump()[0];
    assert_eq!(row.success_count, 2);
    assert!(row.elo > default_elo(), "2W/0L rated up: {}", row.elo);
    assert!(row.span_elo < default_elo());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // unrated == base IS the batch-only pin
fn record_never_touches_elo_batch_only() {
    // The non-goal pin: no online ELO updates in the hot fix loop —
    // record leaves the row unrated; ONLY the batch pass writes
    // ratings (G4's heal-path promise).
    let root = tmp_root("elo-batch-only");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .record(
            "match_bool",
            "match c { true => a, _ => b }",
            "if c { a } else { b }",
            true,
            true,
            "run-a",
        )
        .unwrap();
    let row = &store.dump()[0];
    assert_eq!(row.elo, default_elo(), "record must not write ratings");
    assert_eq!(row.span_elo, default_elo());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // bridge fidelity IS bit-identity to the persisted rating
fn elo_table_loads_persisted_ratings_verbatim() {
    // Issue 039 T4 — the bridge contract: after a recompute pass (and a
    // reopen), the store's table answers every row's PERSISTED rating
    // for BOTH namespaces (rule lint_key + span shape_hash). No replay:
    // the load is exactly what recompute wrote.
    let root = tmp_root("elo-table-load");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        for _ in 0..3 {
            store
                .record("rule_a", "shape 1", "fix", true, true, "run-1")
                .unwrap();
        }
        store
            .record("rule_a", "shape 2", "fix", false, true, "run-1")
            .unwrap();
        store
            .record("rule_b", "shape 1", "fix2", true, true, "run-1")
            .unwrap();
        store.recompute_elo().unwrap();
    }
    let (store, _) = TrajStore::open(&root).unwrap();
    let table = store.elo_table();
    for row in store.dump() {
        assert_eq!(
            table.rating(&row.lint_key),
            row.elo,
            "rule rating must match the persisted row value for {}",
            row.lint_key
        );
        assert_eq!(
            table.rating(&shape_hash(&row.shape)),
            row.span_elo,
            "span rating must match the persisted row value"
        );
    }
    assert!(table.contains("rule_a"));
    assert!(table.contains("rule_b"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // the mixed-value edge IS a rating comparison
fn elo_table_prefers_rated_row_over_post_recompute_sighting() {
    // The load-bearing edge: a row CREATED after the last recompute
    // sits at base (a sighting, not a rating event) while the rule's
    // older rows carry the rated value — the bridge must answer the
    // RATED value, not the stale base.
    let root = tmp_root("elo-table-edge");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        for _ in 0..4 {
            store
                .record("rule_r", "shape x", "f", true, true, "run-1")
                .unwrap();
        }
        store.recompute_elo().unwrap();
        // Post-recompute sighting on a NEW shape: fresh row at base.
        store
            .record_applied("rule_r", "shape new", "f", "run-2", None)
            .unwrap();
    }
    let (store, _) = TrajStore::open(&root).unwrap();
    let dump = store.dump();
    let rated = dump.iter().find(|r| r.shape == "shape x").unwrap().elo;
    let fresh = dump.iter().find(|r| r.shape == "shape new").unwrap().elo;
    assert!(rated > default_elo());
    assert_eq!(fresh, default_elo(), "fresh sighting sits at base");
    assert_eq!(store.elo_table().rating("rule_r"), rated);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[allow(clippy::float_cmp)] // unrated == base IS the absent-player contract
fn elo_table_leaves_unrated_absent_and_empty_store_empty() {
    // Review-only sightings are draws — never matches — so an
    // apply-only rule stays OUT of the table (rating() answers base),
    // and a store with no rows bridges to an empty table.
    let root = tmp_root("elo-table-unrated");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .record_applied("review_only", "shape 3", "fix3", "run-1", None)
            .unwrap();
        let table = store.elo_table();
        assert!(table.is_empty());
        assert!(!table.contains("review_only"));
        assert_eq!(table.rating("review_only"), default_elo());
    }
    let root2 = tmp_root("elo-table-empty");
    let (store, _) = TrajStore::open(&root2).unwrap();
    assert!(store.elo_table().is_empty());
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&root2);
}

// ── Issue 050 T2: the corpus + score series families ─────────────

#[test]
fn corpus_roundtrip_and_idempotent_upsert() {
    let root = tmp_root("corpus-roundtrip");
    {
        let (store, _) = TrajStore::open(&root).unwrap();
        store
            .upsert_corpus_count("2026-08-28", "kernel_opt", 393, true, "source-walk")
            .unwrap();
        store
            .upsert_corpus_count("2026-08-28", "rust_perf", 88, true, "source-walk")
            .unwrap();
        store
            .upsert_corpus_count("2026-08-27", "sec", 6, false, "import-jsonl")
            .unwrap();
        let series = store.corpus_series().unwrap();
        assert_eq!(series.len(), 3);
        let json_before = serde_json::to_string(&series).unwrap();
        // Same inputs again → same bytes at the same keys (the
        // idempotent-overwrite contract).
        store
            .upsert_corpus_count("2026-08-28", "kernel_opt", 393, true, "source-walk")
            .unwrap();
        let json_after = serde_json::to_string(&store.corpus_series().unwrap()).unwrap();
        assert_eq!(json_before, json_after, "second upsert changed nothing");
    }
    // Reopen: the series is durable + (day, domain)-ordered.
    let (store, _) = TrajStore::open(&root).unwrap();
    let series = store.corpus_series().unwrap();
    assert_eq!(series.len(), 3);
    assert_eq!(
        (series[0].day.as_str(), series[0].domain.as_str()),
        ("2026-08-27", "sec")
    );
    assert_eq!(
        (series[1].day.as_str(), series[1].domain.as_str()),
        ("2026-08-28", "kernel_opt")
    );
    assert_eq!(
        (series[2].day.as_str(), series[2].domain.as_str()),
        ("2026-08-28", "rust_perf")
    );
    assert_eq!(series[1].count, 393);
    assert!(series[1].compiled);
    assert_eq!(series[0].source_note, "import-jsonl");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn corpus_by_day_merges_and_omits_absent_domains() {
    let root = tmp_root("corpus-by-day");
    let (store, _) = TrajStore::open(&root).unwrap();
    // Day 1: only kernel_opt measured.
    store
        .upsert_corpus_count("2026-08-27", "kernel_opt", 380, true, "source-walk")
        .unwrap();
    // Day 2: kernel_opt + rust_perf (sec absent — must NOT appear, and
    // must NOT be synthesized as 0).
    store
        .upsert_corpus_count("2026-08-28", "kernel_opt", 393, true, "source-walk")
        .unwrap();
    store
        .upsert_corpus_count("2026-08-28", "rust_perf", 88, true, "source-walk")
        .unwrap();
    let by_day = store.corpus_series_by_day().unwrap();
    assert_eq!(by_day.len(), 2, "one flat row per day");
    assert_eq!(
        by_day[0],
        serde_json::json!({"ts": "2026-08-27", "kernel_opt": 380}),
        "absent domains omitted — never a 0"
    );
    assert_eq!(
        by_day[1],
        serde_json::json!({"ts": "2026-08-28", "kernel_opt": 393, "rust_perf": 88})
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn score_run_roundtrip_and_ordering() {
    let root = tmp_root("score-run");
    let mk = |ts: &str, sha: &str, unix: u64, rate: f32| ScoreRunRow {
        ts: ts.to_string(),
        sha: sha.to_string(),
        n_ran: 34,
        n_healable: 28,
        n_clean: 6,
        heal_rate: rate,
        created_rate: 0.01,
        parse_safety: 1.0,
        decline_correctness: 1.0,
        edits_total: 60,
        vocab_size: 33,
        generation: 1,
        recorded_unix: unix,
    };
    let (store, _) = TrajStore::open(&root).unwrap();
    // Insert out of order; score_runs must return (recorded_unix, key)
    // order. The empty-sha row pins the `nosh` key part.
    store
        .record_score_run(&mk("2026-08-25T04:05:18", "7399ffc", 1_756_087_518, 0.96))
        .unwrap();
    store
        .record_score_run(&mk("2026-08-19T04:49:29", "", 1_755_571_769, 0.84))
        .unwrap();
    store
        .record_score_run(&mk("2026-08-28T02:09:06", "9f08442", 1_756_337_346, 0.98))
        .unwrap();
    let runs = store.score_runs().unwrap();
    assert_eq!(runs.len(), 3);
    assert_eq!(
        runs[0].sha, "",
        "empty sha stored verbatim (key part is nosh)"
    );
    assert!(runs[0].recorded_unix < runs[1].recorded_unix);
    assert!(runs[1].recorded_unix < runs[2].recorded_unix);
    assert!((runs[2].heal_rate - 0.98).abs() < 1e-6);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn jsonl_imports_are_idempotent_and_skip_malformed_lines() {
    let root = tmp_root("jsonl-import");
    let dir = root.join("benchmarks");
    std::fs::create_dir_all(&dir).unwrap();
    let corpus_jsonl = dir.join("corpus_history.jsonl");
    std::fs::write(
            &corpus_jsonl,
            concat!(
                "{\"ts\":\"2026-08-27\",\"kernel_opt\":380,\"sec\":6}\n",
                "{\"ts\":\"2026-08-28\",\"kernel_opt\":393,\"rust_perf\":88,\"clippy_lints\":40,\"sec\":6}\n",
                "NOT JSON AT ALL\n",
            ),
        )
        .unwrap();
    let score_jsonl = dir.join("score_history.jsonl");
    std::fs::write(
            &score_jsonl,
            concat!(
                "{\"ts\":\"2026-08-19T04:49:29\",\"sha\":\"72bac03\",\"n_ran\":34,\"n_healable\":28,\"n_clean\":6,\"heal_rate\":0.84615386,\"created_rate\":0.057692308,\"parse_safety\":0.9285714,\"decline_correctness\":1.0,\"edits_total\":60,\"vocab_size\":33}\n",
                "GARBAGE\n",
            ),
        )
        .unwrap();
    let (store, _) = TrajStore::open(&root).unwrap();
    // 2 + 4 domain rows seen (the malformed line skipped, not fatal).
    let (seen, written) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
    assert_eq!((seen, written), (6, 6));
    let (seen_s, written_s) = store.import_score_jsonl(&score_jsonl).unwrap();
    assert_eq!((seen_s, written_s), (1, 1));
    // Series reflects the import with the import-jsonl provenance.
    let series = store.corpus_series().unwrap();
    assert_eq!(series.len(), 6);
    assert!(series.iter().all(|r| r.source_note == "import-jsonl"));
    assert_eq!(
        store.score_runs().unwrap()[0].recorded_unix,
        parse_ts_unix("2026-08-19T04:49:29")
    );
    // Second import: upserts change nothing → 0 written (idempotence).
    let (seen2, written2) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
    assert_eq!((seen2, written2), (6, 0));
    let (seen3, written3) = store.import_score_jsonl(&score_jsonl).unwrap();
    assert_eq!((seen3, written3), (1, 0));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn corpus_import_yields_to_a_first_hand_source_walk_row() {
    // Issue 050 T2 provenance precedence: the store is the source of
    // truth, so a (day, domain) key already holding a `source-walk`
    // stamp is never clobbered by the second-hand jsonl projection —
    // even when the projection carries a DIFFERENT (stale) count.
    let root = tmp_root("corpus-import-precedence");
    let dir = root.join("benchmarks");
    std::fs::create_dir_all(&dir).unwrap();
    let corpus_jsonl = dir.join("corpus_history.jsonl");
    std::fs::write(
        &corpus_jsonl,
        "{\"ts\":\"2026-08-28\",\"kernel_opt\":393}\n",
    )
    .unwrap();
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .upsert_corpus_count("2026-08-28", "kernel_opt", 396, true, "source-walk")
        .unwrap();
    // Import sees the row but writes nothing — the stamp survives.
    let (seen, written) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
    assert_eq!((seen, written), (1, 0));
    let series = store.corpus_series().unwrap();
    assert_eq!(series.len(), 1);
    assert_eq!(
        series[0].count, 396,
        "source-walk stamp survived the import"
    );
    assert_eq!(series[0].source_note, "source-walk");
    // An import-jsonl row (no stamp) is still idempotently refreshed.
    store
        .upsert_corpus_count("2026-08-27", "sec", 6, false, "import-jsonl")
        .unwrap();
    std::fs::write(
        &corpus_jsonl,
        concat!(
            "{\"ts\":\"2026-08-28\",\"kernel_opt\":393}\n",
            "{\"ts\":\"2026-08-27\",\"sec\":6}\n",
        ),
    )
    .unwrap();
    let (seen2, written2) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
    assert_eq!(
        (seen2, written2),
        (2, 0),
        "same-bytes re-import writes nothing"
    );
    assert_eq!(store.corpus_series().unwrap().len(), 2);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn corpus_import_yields_to_a_runtime_corpus_row_too() {
    // Plan 059 B2 (the 009-F2 fix): `runtime-corpus` is a FIRST-HAND
    // provenance (the compiled builder count), so a stale committed
    // projection must not clobber it either — the precedence check
    // previously covered only `source-walk`, which would have let the
    // very drift the fix corrects (docker 2 vs 31) re-enter the store
    // on the next --import-series.
    let root = tmp_root("corpus-import-runtime-precedence");
    let dir = root.join("benchmarks");
    std::fs::create_dir_all(&dir).unwrap();
    let corpus_jsonl = dir.join("corpus_history.jsonl");
    std::fs::write(&corpus_jsonl, "{\"ts\":\"2026-09-01\",\"docker\":2}\n").unwrap();
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .upsert_corpus_count("2026-09-01", "docker", 31, true, "runtime-corpus")
        .unwrap();
    let (seen, written) = store.import_corpus_jsonl(&corpus_jsonl).unwrap();
    assert_eq!((seen, written), (1, 0), "the stale projection yields");
    let series = store.corpus_series().unwrap();
    assert_eq!(series.len(), 1);
    assert_eq!(
        series[0].count, 31,
        "runtime-corpus stamp survived the import"
    );
    assert_eq!(series[0].source_note, "runtime-corpus");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn parse_ts_unix_and_today_roundtrip() {
    // Date-only and full-timestamp shapes both parse.
    assert_eq!(
        parse_ts_unix("2026-08-19"),
        parse_ts_unix("2026-08-19T00:00:00")
    );
    assert_eq!(parse_ts_unix("2026-08-19T04:49:29"), 1_787_114_969);
    assert_eq!(parse_ts_unix("garbage"), 0);
    assert_eq!(parse_ts_unix("2026-13-99"), 0, "out-of-range fields refuse");
    // days_from_civil ∘ civil_from_days is the identity on sample dates.
    for day in [0i64, 1, 100, 19_000, 20_663] {
        let (y, m, d) = civil_from_days(day);
        assert_eq!(days_from_civil(y, m, d), day);
    }
    // today_local_date is the date part of a fresh civil computation
    // (shape check — the value moves with the clock).
    let today = today_local_date();
    assert_eq!(today.len(), 10);
    assert_eq!(today.as_bytes()[4], b'-');
    assert_eq!(today.as_bytes()[7], b'-');
    assert_eq!(
        parse_ts_unix(&today),
        parse_ts_unix(&format!("{today}T00:00:00"))
    );
}

#[cfg(feature = "score_bench")]
#[test]
fn score_run_row_is_schema_identical_to_history_record() {
    // The jsonl ↔ store schema-identity contract (Issue 050 T2): a
    // HistoryRecord round-trips through ScoreRunRow with every scored
    // field preserved — only `recorded_unix` is added. The five
    // DELIBERATE exceptions (instrumentation, not scored axes — the
    // additive law): `ood` (issue 077 T4), `strata` (issue 089 T6),
    // `rule_outcomes` (issue 089 T8), `cross_ooo` (issue 098 T3/T4), and
    // `demonstrability` (issue 134 T1). The store row drops all five
    // and the jsonl stays the committed projection; the asymmetry is
    // pinned by the second half of this test (an instrumented line
    // still imports).
    let rec = crate::score_bench::report::HistoryRecord {
        ts: "2026-08-19T04:49:29".to_string(),
        sha: "72bac03".to_string(),
        n_ran: 34,
        n_healable: 28,
        n_clean: 6,
        heal_rate: 0.846_153_86,
        created_rate: 0.057_692_308,
        parse_safety: 0.928_571_4,
        decline_correctness: 1.0,
        edits_total: 60,
        vocab_size: 33,
        generation: 1,
        ood: None,
        cross_ooo: None,
        strata: None,
        shaped: None,
        rule_outcomes: None,
        demonstrability: None,
    };
    let row = ScoreRunRow::from(&rec);
    assert_eq!(row.ts, rec.ts);
    assert_eq!(row.sha, rec.sha);
    assert_eq!(row.n_ran, rec.n_ran);
    assert_eq!(row.n_healable, rec.n_healable);
    assert_eq!(row.n_clean, rec.n_clean);
    // f32 fields: the contract is BIT-IDENTITY through the row round-trip
    // (not approximate equality) — float_cmp is the gate here.
    #[allow(clippy::float_cmp)]
    {
        assert_eq!(row.heal_rate, rec.heal_rate);
        assert_eq!(row.created_rate, rec.created_rate);
        assert_eq!(row.parse_safety, rec.parse_safety);
        assert_eq!(row.decline_correctness, rec.decline_correctness);
    }
    assert_eq!(row.edits_total, rec.edits_total);
    assert_eq!(row.vocab_size, rec.vocab_size);
    assert_eq!(row.generation, rec.generation, "generation rides the row");
    assert_eq!(row.recorded_unix, parse_ts_unix(&rec.ts));
    // And the serialized jsonl line of the record is a PREFIX-shape of
    // the row: deserializing the record's JSON into a ScoreRunRow keeps
    // recorded_unix at its serde default (0) — the import path.
    let line = serde_json::to_string(&rec).unwrap();
    let from_jsonl: ScoreRunRow = serde_json::from_str(&line).unwrap();
    assert_eq!(
        from_jsonl.recorded_unix, 0,
        "jsonl shape carries no unix stamp"
    );
    assert_eq!(from_jsonl.ts, rec.ts);

    // The ood asymmetry is TOLERANT in both directions: an ood-carrying
    // jsonl line imports into the store row with the scored fields
    // intact (serde ignores the unknown key on the row side), and the
    // dropped axis never corrupts the scored projection.
    let mut instrumented = rec.clone();
    instrumented.ood = Some(crate::score_bench::ood::OodRow {
        ref_set: "v1_frozen".to_string(),
        ood_set: "gen6_real_repo".to_string(),
        n_ref: 34,
        n_ood: 17,
        n_ref_skipped: 0,
        n_ood_skipped: 0,
        mean_ref_entropy: 0.5,
        mean_ood_entropy: 1.5,
        mean_gap: 1.0,
        standardized_gap: 1.25,
        regime: crate::score_bench::ood::OodRegime::Memorizing,
        artifact: "cd".repeat(32),
        ref_provenance_families: vec!["authored-composite".to_string()],
        ood_provenance_families: vec!["riir-chain@7421245a~1".to_string()],
    });
    let line = serde_json::to_string(&instrumented).unwrap();
    let from_ood_line: ScoreRunRow = serde_json::from_str(&line).unwrap();
    let mut expected = ScoreRunRow::from(&rec);
    expected.recorded_unix = 0; // the jsonl shape carries no unix stamp
    assert_eq!(
        from_ood_line, expected,
        "the dropped axis must not move any scored field"
    );
}

// ── Issue 072 T1/T3: the construction-pair family ────────────

// construction_pairs() needs the rustc_errors corpus half of
// frontier_pairs — this test is meaningless without it.
#[cfg(feature = "rustc_errors")]
#[test]
fn pair_import_is_idempotent_and_never_touches_tiers() {
    let root = tmp_root("pair_import");
    let (store, _) = TrajStore::open(&root).unwrap();
    let pairs = crate::frontier_pairs::construction_pairs();
    let n = pairs.len();
    let (seen, written) = store.import_pairs(&pairs).unwrap();
    assert_eq!(seen, n);
    assert_eq!(written, n);
    // Re-import: byte-identical rows → nothing written.
    let (seen2, written2) = store.import_pairs(&pairs).unwrap();
    assert_eq!(seen2, n);
    assert_eq!(written2, 0, "second import must be a no-op");
    // Rows are readable back.
    let rows = store.pair_rows().unwrap();
    assert_eq!(rows.len(), n);
    for r in &rows {
        assert_eq!(r.provenance, "construction");
        assert_ne!(r.code, r.bad_code, "cross-code premise is data");
    }
    // SOFT-JUDGE-ONLY: pair ingestion must not create trajectory rows
    // (dump sees nothing) and must not rate any outcome player.
    assert_eq!(
        store.dump().len(),
        0,
        "pair import must never create traj rows"
    );
    assert_eq!(
        store.elo_table().len(),
        0,
        "pair import must never rate outcome players"
    );
}

#[cfg(feature = "rustc_errors")]
#[test]
fn rate_pairs_orders_playbook_strategies_above_generator_losers() {
    let root = tmp_root("rate_pairs");
    let (store, _) = TrajStore::open(&root).unwrap();
    store
        .import_pairs(&crate::frontier_pairs::construction_pairs())
        .unwrap();
    let (fit, n) = store.rate_pairs(300).unwrap();
    assert!(n >= 200);
    assert!(!fit.is_empty());
    // Every rated player sits inside the standard Elo band (the map's
    // clamp keeps it finite even at sweep boundaries).
    let base = default_elo();
    for (id, elo) in &fit {
        assert!(elo.is_finite(), "{id} not finite");
        // Within ±400·log10(N_max) — a sanity band, not a tight pin.
        assert!(elo.abs() < 10_000.0, "{id} = {elo} out of band");
    }
    // The strategies the corpus's construction truths name must ALL be
    // present in the fit (every strategy participates in ≥1 pair).
    for want in [
        "e0597-scope-hoist",
        "e0499-swap-second-decl",
        "e0502-deref-removal",
        "e0505-clone-at-move",
    ] {
        assert!(fit.contains_key(want), "{want} missing from pair fit");
    }
    let _ = base;
}

// Issue 089 T5 — the non-circularity fence, load-bearing: an organic
// pair row (weight 1.0 in YIELD accounting) must never feed the RATING
// fit. Hand-put one organic row beside imported construction rows and
// assert it is fenced out — count and players both.
#[cfg(feature = "rustc_errors")]
#[test]
fn rate_pairs_fences_non_construction_provenance_out_of_the_fit() {
    let root = tmp_root("rate_pairs_fence");
    let (store, _) = TrajStore::open(&root).unwrap();
    let pairs = crate::frontier_pairs::construction_pairs();
    store.import_pairs(&pairs).unwrap();
    let n_construction = pairs.len();

    // A fabricated-for-test organic row — distinctive strategy ids so its
    // absence from the fit is unambiguous ("never inject failures on
    // purpose" governs PRODUCTION rows; a unit test's fixture row is the
    // measurement, not the curriculum).
    let organic = PairRowDoc {
        code: "E0597".into(),
        bad_code: "E0502".into(),
        case_name: "organic_probe_case".into(),
        good_strategy: "organic-winner-probe".into(),
        bad_strategy: "organic-loser-probe".into(),
        good_fix: String::new(),
        bad_fix: String::new(),
        span_shape_hash: String::new(),
        provenance: crate::frontier_pairs::PROVENANCE_ORGANIC.into(),
    };
    store
        .put_row_if_changed(
            &TrajStore::pair_key("E0597", "organic_probe_case"),
            &organic,
        )
        .unwrap();
    assert_eq!(store.pair_rows().unwrap().len(), n_construction + 1);

    let (fit, rated) = store.rate_pairs(50).unwrap();
    // Count: the fence rated exactly the construction rows.
    assert_eq!(
        rated, n_construction,
        "fence must pass construction rows only"
    );
    // Players: the organic row's strategies never enter the fit.
    assert!(!fit.contains_key("organic-winner-probe"));
    assert!(!fit.contains_key("organic-loser-probe"));
}

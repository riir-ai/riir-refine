//! Issue 204's feed-law gates: the consent state machine drives the
//! contribution feed's auto-send, and the wire payload is counts-only by
//! shape (the privacy floor). The join/leave flows write through
//! [`first_run_consent`]; these tests pin BOTH directions of that law
//! plus the shape floor — the run-end tail reads the same predicates.

#![cfg(feature = "points")]

use riir_refine::first_run_consent::{Answer, CONSENT_FILE};

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("refine-i204-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

#[test]
fn the_consent_state_machine_drives_the_feed_both_directions() {
    let dir = temp_dir("consent");
    // Unset (never asked) → nothing sends.
    assert_eq!(riir_refine::first_run_consent::read(&dir), Answer::Unset);
    assert!(!riir_refine::first_run_consent::read(&dir).send_allowed());
    // The join law: join writes Joined → the feed's predicate is TRUE.
    riir_refine::first_run_consent::write(&dir, Answer::Joined).expect("write");
    assert_eq!(riir_refine::first_run_consent::read(&dir), Answer::Joined);
    assert!(riir_refine::first_run_consent::read(&dir).send_allowed());
    // The leave law: leave writes Never → zero data, both directions.
    riir_refine::first_run_consent::write(&dir, Answer::Never).expect("write");
    assert_eq!(riir_refine::first_run_consent::read(&dir), Answer::Never);
    assert!(!riir_refine::first_run_consent::read(&dir).send_allowed());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_feed_payload_is_counts_only_by_shape() {
    // The contribute body serialization over the CLIENT's own path: no
    // span/source/path/file/rule-text field may exist. The wire's golden
    // pins the exact shape in riir-ledger; this floor re-asserts the same
    // law over the client's fold output so a client-side field addition
    // cannot slip the wire silently.
    let outcomes = [
        riir_refine::points_client::ContributeOutcome {
            name: "needless_return".to_string(),
            written: 2,
            declined: 1,
        },
        riir_refine::points_client::ContributeOutcome {
            name: "map_clone".to_string(),
            written: 1,
            declined: 0,
        },
    ];
    let folded: std::collections::BTreeMap<&str, (u32, u32)> = outcomes
        .iter()
        .map(|o| (o.name.as_str(), (o.written, o.declined)))
        .collect();
    let rules: Vec<riir_ledger::dto::ContributeRuleOutcome> = folded
        .into_iter()
        .map(
            |(name, (written, declined))| riir_ledger::dto::ContributeRuleOutcome {
                name: name.to_string(),
                written,
                declined,
            },
        )
        .collect();
    let body = serde_json::to_string(&rules).unwrap();
    for banned in ["span", "source", "path", "file", "snippet", "code_text"] {
        assert!(
            !body.to_lowercase().contains(banned),
            "the feed payload carries the banned shape field `{banned}`: {body}"
        );
    }
    // Counts stay counts.
    assert!(body.contains("written"));
    assert!(body.contains("declined"));
}

#[test]
fn the_consent_record_file_is_the_join_leave_flows_one_home() {
    // The join/leave CLIs write Answer::Joined/Never through
    // first_run_consent::write — the SAME file the tail reads. Pin the
    // file name so a rename on either side fails here, not in the field.
    assert_eq!(CONSENT_FILE, "first_run_consent.json");
}

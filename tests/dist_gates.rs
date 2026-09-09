//! Issue 085 — the `dist` domain gate tests.
//!
//! The gates a Phase-1 suggest-only domain owes, mechanically: corpus ↔
//! detector agreement, the precision fixtures (fire exactly on the buggy
//! shape, silent on the fixed shape + the negatives), the caller-evidence
//! gate (silent without the parent walk), and the suggest-only contract.
//! The refusal table is pinned here too — advice this domain must never
//! emit ([`refuses`]).
#![cfg(feature = "dist")]

use riir_clippy::domains::dist::rules::{scan_source_with, DIST_RULES, FIXABLE_DIST};
use riir_clippy::domains::dist::{refuses, DistContext, DistDomain};
use riir_clippy::HealDomain;

/// The suggest-only contract: the corpus, the detector table, and the
/// fixable table agree, and Phase 1 ships NO bounded fix.
#[test]
fn corpus_detector_and_fixable_tables_agree() {
    let entries = DistDomain::build_corpus();
    assert_eq!(entries.len(), 4, "Phase 1 ships exactly 4 rules");
    let ids: std::collections::BTreeSet<String> =
        entries.iter().map(|r| r.rule_id.clone()).collect();
    let detectors: std::collections::BTreeSet<String> =
        DIST_RULES.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(ids, detectors, "corpus and detector table drifted");
    assert!(
        FIXABLE_DIST.is_empty(),
        "Phase 1 is suggest-only — a fixable rule joined without the Phase-2 lane"
    );
}

/// Every buggy fixture fires its OWN rule, every fixed fixture loses it.
/// The three ship-size rules share a trigger shape (a bin manifest missing
/// knobs), so a buggy fixture co-firing its SIBLINGS is correct behavior —
/// the gate asserts own-rule presence/absence, not silence.
#[test]
fn fixtures_fire_exactly_their_own_rule() {
    for r in DistDomain::build_corpus() {
        // The member rule needs its caller evidence (the parent-walk flag);
        // every other rule is text-only.
        let ctx = DistContext {
            under_workspace_root: r.rule_id == "member-profile-ignored",
        };
        let buggy = scan_source_with(&r.buggy_code, ctx);
        let fixed = scan_source_with(&r.fixed_code, ctx);
        assert!(
            !buggy.is_empty(),
            "{}: buggy fixture does not fire (context-gated rules are tested with evidence)",
            r.rule_id
        );
        assert!(
            buggy.iter().any(|f| f.rule_id == r.rule_id),
            "{}: buggy fixture does not fire its own rule",
            r.rule_id
        );
        assert!(
            fixed.iter().all(|f| f.rule_id != r.rule_id),
            "{}: fixed fixture still fires its own rule",
            r.rule_id
        );
    }
}

/// The member-profile rule is SILENT without caller evidence — the negative
/// control that keeps the ScanContext pattern honest.
#[test]
fn member_rule_silent_without_evidence() {
    let buggy = "[[bin]]\nname = \"m\"\n\n[profile.release]\nstrip = true\n";
    let findings = scan_source_with(buggy, DistContext::default());
    assert!(
        findings.iter().all(|f| f.rule_id != "member-profile-ignored"),
        "member rule fired without the workspace evidence"
    );
}

/// The canonical precision fixture — a repo-shaped manifest missing every
/// knob fires all three ship-size rules exactly once each; the same manifest
/// with a complete [profile.dist] block is silent.
#[test]
fn canonical_repo_manifest_fires_three_and_completes_to_silence() {
    let before = "[[bin]]\nname = \"tool\"\n\n[dependencies]\nserde = \"1\"\n";
    let findings = scan_source_with(before, DistContext::default());
    let ids: Vec<&str> = findings.iter().map(|f| f.rule_id).collect();
    assert_eq!(ids.len(), 3, "expected 3 ship-size citations, got {ids:?}");
    for rule in [
        "shipped-binary-unstripped",
        "shipped-binary-missing-size-profile",
        "cli-panic-unwind-tables",
    ] {
        assert_eq!(
            ids.iter().filter(|id| **id == rule).count(),
            1,
            "{rule} did not fire exactly once"
        );
    }

    let after = format!("{before}\n[profile.dist]\ninherits = \"release\"\nopt-level = \"s\"\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\n");
    assert!(
        scan_source_with(&after, DistContext::default()).is_empty(),
        "the complete profile must silence every rule"
    );
}

/// Lib-only crates never get ship-size advice (no shipped artifact, no
/// citation).
#[test]
fn lib_only_manifest_is_never_cited() {
    let lib = "[package]\nname = \"lib\"\n\n[lib]\ncrate-type = [\"rlib\"]\n\n[dependencies]\nserde = \"1\"\n";
    assert!(scan_source_with(lib, DistContext::default()).is_empty());
}

/// The refusal table: advice this domain must NEVER emit, pinned by id.
#[test]
fn refusal_table_pinned() {
    let r = refuses();
    let ids: Vec<&str> = r.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec!["edit-release-profile-opt-level", "strip-in-dev-profile"]);
    assert!(r.iter().all(|(_, why)| why.contains("REFUSED")));
}

/// The proposed-fix oracle is Tier-0 structural only — a well-formed
/// manifest validates, and the verdict claims nothing more.
#[test]
fn validate_is_structural() {
    let good = "[[bin]]\nname = \"x\"\n\n[profile.release]\nstrip = true\n";
    assert!(DistDomain::validate("shipped-binary-unstripped", good).scans);
}

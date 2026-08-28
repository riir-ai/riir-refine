//! Plan 030 T0.5(a) + T1.1 — the Class-A agreement gate, measured against
//! the LIVE incumbent.
//!
//! Every Class-A rule claims to overlay a hadolint DL rule: the incumbent
//! detects, we fix. That claim is checkable, and this is the check —
//! hadolint must report the code on our buggy fixture and must NOT report
//! it on our fixed one. A rule that fails here is not an overlay; it is a
//! rule with a citation it has not earned.
//!
//! This is also the T1.1 mine-time dedup re-verification, run against the
//! live binary rather than the Research 086 snapshot (the standing
//! mine-time rule). It SKIPS, loudly, when hadolint is absent.

#![cfg(feature = "docker_verify")]

use riir_clippy::domains::docker::corpus::build_docker_seed_corpus;
use riir_clippy::domains::docker::verify::{available, class_a_agreement, hadolint, Agreement};

#[test]
fn every_class_a_rule_agrees_with_hadolint() {
    if !available() {
        println!("SKIP: hadolint not on PATH — the agreement gate cannot run (NOT a pass)");
        return;
    }
    let corpus = build_docker_seed_corpus();
    let mut checked = 0usize;
    let mut disagreements = Vec::new();
    for rule in &corpus.entries {
        let Some(code) = rule.incumbent.as_deref() else { continue };
        if !code.starts_with("DL") {
            // BuildKit check names belong to `docker build --check` (Tier 2).
            continue;
        }
        checked += 1;
        match class_a_agreement(&rule.rule_id) {
            Agreement::Agrees => {}
            other => disagreements.push((rule.rule_id.clone(), code.to_string(), other)),
        }
    }
    assert!(checked >= 6, "only {checked} DL-coded rules were checkable — the ledger thinned out");
    assert!(
        disagreements.is_empty(),
        "Class-A rules disagree with the live incumbent: {disagreements:?}"
    );
}

/// The other half of the dedup ledger: our **Class-B** rules must be in
/// territory hadolint does NOT cover. If the incumbent already reports a
/// finding on the exact line our Class-B rule fires on, the rule is a
/// duplicate wearing a novel label.
#[test]
fn class_b_rules_are_in_unoccupied_territory() {
    if !available() {
        println!("SKIP: hadolint not on PATH — the dedup check cannot run (NOT a pass)");
        return;
    }
    // Fixtures that trip UNRELATED DL rules are excluded — we are testing
    // TERRITORY overlap, not fixture purity. `DL3009` is in the list for a
    // measured reason, not a convenient one: a bare `apt-get update` RUN
    // necessarily also trips the apt-lists rule, so any fixture for
    // `apt-update-install-same-run` trips it. The distinctness claim is
    // proven directly instead, by `hadolint_is_silent_on_the_apt_split`.
    const IGNORE: &[&str] = &["DL3006", "DL3008", "DL3018", "DL3007", "DL3022", "DL3009"];
    let corpus = build_docker_seed_corpus();
    let mut overlaps = Vec::new();
    for rule in &corpus.entries {
        if rule.incumbent.is_some() {
            continue;
        }
        let Some(found) = hadolint(&rule.buggy_code) else { continue };
        let relevant: Vec<String> = found
            .into_iter()
            .filter(|f| !IGNORE.contains(&f.code.as_str()))
            .map(|f| f.code)
            .collect();
        if !relevant.is_empty() {
            overlaps.push((rule.rule_id.clone(), relevant));
        }
    }
    assert!(
        overlaps.is_empty(),
        "Class-B rules claim novel territory but hadolint already fires there: {overlaps:?}"
    );
}

/// The sharp form of the `apt-update-install-same-run` novelty claim.
///
/// The IGNORE list above excuses DL3009 on that rule's fixture; this test
/// is what earns the excuse. A split where the update RUN ALREADY deletes
/// its lists carries no DL3009 trigger — and hadolint is then completely
/// silent, while our rule still fires. That is unoccupied territory,
/// measured rather than asserted.
#[test]
fn hadolint_is_silent_on_the_apt_split() {
    if !available() {
        println!("SKIP: hadolint not on PATH (NOT a pass)");
        return;
    }
    let split = "FROM debian:bookworm-slim\n\
                 RUN apt-get update && rm -rf /var/lib/apt/lists/*\n\
                 RUN apt-get install --no-install-recommends -y curl=7.88.1-10+deb12u5 \\\n && rm -rf /var/lib/apt/lists/*\n";
    let found = hadolint(split).expect("oracle available");
    assert!(
        found.is_empty(),
        "hadolint is no longer silent on the apt split — re-pin the dedup ledger: {found:?}"
    );
    let ours: Vec<&str> = riir_clippy::domains::docker::rules::scan_source(split)
        .into_iter()
        .map(|f| f.rule_id)
        .collect();
    assert!(
        ours.contains(&"apt-update-install-same-run"),
        "our rule stopped firing on the shape it claims to own: {ours:?}"
    );
}

//! Plan 030 Phase 2 — the TIER-2 agreement gate, measured against the LIVE
//! `docker build --check`.
//!
//! Bench 056's postscript verified four BuildKit-check claims by hand and
//! said so honestly: "a one-shot verification of a documented claim... the
//! standing tier-2 oracle stays Phase 2". This file is that oracle. Every
//! rule whose ledger names a BuildKit check must have that check FIRE on
//! its `buggy_code` and be SILENT on its `fixed_code` — the same contract
//! Tier 1 holds the DL-coded rules to.
//!
//! ## Cost, stated rather than discovered
//!
//! `docker build --check` needs a running daemon, and it resolves source
//! metadata over the network for any base image that is not `scratch`
//! (measured ~1.3-2.2 s per invocation on Docker 29.7.2, and it does NOT
//! warm on repeat). That is why the whole tier is a separate opt-in
//! feature and why nothing here runs in the heal path.
//!
//! It SKIPS, loudly, when the daemon is absent — "not checked" is not
//! "checked and passed" (Issue 018).

#![cfg(feature = "docker_buildcheck")]

use riir_refine::domains::docker::buildcheck::{CheckRun, available, build_check, findings};
use riir_refine::domains::docker::corpus::build_docker_seed_corpus;
use riir_refine::domains::docker::verify::{Agreement, all_tier2_agreements};

/// Best-effort daemon identity for failure output — the issue-071 lesson:
/// both sides of the agreement contract are version-locked to the installed
/// daemon, so a red gate that does not NAME the daemon cannot be diagnosed
/// from the log alone.
fn daemon_version() -> String {
    std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "<unavailable>".to_string())
}

#[test]
fn every_buildkit_rule_agrees_with_docker_build_check() {
    if !available() {
        println!("SKIP: `docker build --check` unusable — the Tier-2 gate cannot run (NOT a pass)");
        return;
    }
    let results = all_tier2_agreements();
    assert!(
        results.len() >= 4,
        "only {} rules name a BuildKit check — the Tier-2 ledger thinned out; \
Bench 056 measured 4",
        results.len()
    );
    let bad: Vec<_> = results
        .iter()
        .filter(|(_, _, v)| *v != Agreement::Agrees)
        .collect();
    assert!(
        bad.is_empty(),
        "Tier-2 disagreements (daemon {daemon}; rule, check, verdict): {bad:#?}\n\
\nA rule here either does not fire the check it cites, or its fix does not \
silence it — OR the oracle could not check it (`Unavailable` = the check phase \
never completed, typically a registry resolve failure; that is NOT a check-set \
drift and NOT a pass — the fixtures for the three check-only rules are \
registry-free, so this arm now means a daemon or scratch-level problem, not \
network). Diagnose the `Unavailable` arm with: `docker build --check` on the \
rule's fixture.\n\nMisdiagnosis guard (Issue 071, 2026-09-06): an `Unavailable` \
was once read as \"the daemon removed the check\" — the check had never stopped \
firing; the oracle never finished.",
        daemon = daemon_version(),
    );
    for (rule, check, _) in &results {
        println!("tier2 OK: {rule} -> {check}");
    }
    println!("tier2 daemon: {}", daemon_version());
}

/// The corpus's own claim about which rules are Tier-2-checkable, pinned so
/// a future rule cannot quietly join or leave the set.
#[test]
fn the_tier2_set_is_the_four_measured_in_bench_056() {
    let mut names: Vec<_> = build_docker_seed_corpus()
        .entries
        .iter()
        .filter_map(|r| r.tier2_check().map(|c| (r.rule_id.clone(), c.to_string())))
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            (
                "cmd-entrypoint-exec-form".to_string(),
                "JSONArgsRecommended".to_string()
            ),
            (
                "instruction-casing".to_string(),
                "ConsistentInstructionCasing".to_string()
            ),
            (
                "legacy-key-value-format".to_string(),
                "LegacyKeyValueFormat".to_string()
            ),
            (
                "no-empty-continuation".to_string(),
                "NoEmptyContinuation".to_string()
            ),
        ]
    );
}

/// The failure mode that motivates [`CheckRun::Errored`]: a base image that
/// cannot be resolved makes BuildKit report a STRICT SUBSET of the real
/// findings and still exit non-zero. Measured 2026-08-28: a two-defect file
/// reported one warning.
///
/// The gate is that we return `Errored` (and `findings()` returns `None`) —
/// never a short list that reads like a clean-ish result.
#[test]
fn a_resolve_failure_is_errored_not_a_short_finding_list() {
    if !available() {
        println!("SKIP: `docker build --check` unusable (NOT a pass)");
        return;
    }
    // Two defects: lowercase `from` and the legacy ENV form.
    let src = "from nonexistent-registry-xyz.invalid/foo:1\nENV a b\n";
    match build_check(src) {
        CheckRun::Errored(msg) => {
            assert!(msg.contains("ERROR:"), "unexpected error text: {msg}");
            assert!(
                findings(src).is_none(),
                "a truncated run must not yield findings"
            );
        }
        // If a future BuildKit stops truncating and reports both, that is
        // strictly better — but it must then not be an error either.
        CheckRun::Ran(f) => {
            let names: Vec<_> = f.iter().map(|x| x.check.as_str()).collect();
            assert!(
                names.contains(&"ConsistentInstructionCasing")
                    && names.contains(&"LegacyKeyValueFormat"),
                "the oracle ran without erroring but reported a SUBSET {names:?} — \
that is the silent-truncation case this test exists to catch"
            );
        }
        CheckRun::Unavailable => panic!("available() said yes, then the run said no"),
    }
}

/// A clean file is `Ran(vec![])` — an empty finding list is a real answer,
/// distinct from `Unavailable`. Uses `scratch` so no registry is touched.
#[test]
fn clean_is_ran_empty_not_unavailable() {
    if !available() {
        println!("SKIP: `docker build --check` unusable (NOT a pass)");
        return;
    }
    assert_eq!(
        build_check("FROM scratch\nCMD [\"/app\"]\n"),
        CheckRun::Ran(vec![])
    );
}

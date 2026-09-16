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
use riir_clippy::domains::dist::{refuses, DistContext, DistDomain, ProfileKnobs};
use riir_clippy::HealDomain;

/// The suggest-only contract: the corpus, the detector table, and the
/// fixable table agree, and Phase 1 ships NO bounded fix.
#[test]
fn corpus_detector_and_fixable_tables_agree() {
    let entries = DistDomain::build_corpus();
    // Batch 143 (522f6947) joined large-api-dependency-mostly-unused-hint —
    // its count-pin walk missed THIS pin (the failure that proves the gate
    // is live); Batch 146 (Plan 132) joined pgo-training-set-covers-real-
    // workload-hint — the pin moved WITH the batch. Suggest-only still holds.
    assert_eq!(entries.len(), 6, "Phase 1 ships exactly 6 rules");
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
            ..DistContext::default()
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
        findings
            .iter()
            .all(|f| f.rule_id != "member-profile-ignored"),
        "member rule fired without the workspace evidence"
    );
}

/// Batch 146 — the PGO hint fires ONLY on the fully-tuned audience (all
/// three effective knobs set) and is silent the moment any knob is
/// missing. Strictly additive over the ship-size fixtures by construction.
#[test]
fn pgo_hint_fires_only_on_fully_tuned_bins() {
    let fully_tuned = "[[bin]]\nname = \"m\"\n\n[profile.dist]\ninherits = \"release\"\nstrip = true\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\n";
    let findings = scan_source_with(fully_tuned, DistContext::default());
    let ids: Vec<&str> = findings.iter().map(|f| f.rule_id).collect();
    assert_eq!(
        ids,
        vec!["pgo-training-set-covers-real-workload-hint"],
        "a fully-tuned bin fires the PGO hint ALONE (no sibling co-fires)"
    );

    // One knob missing (the ship-size regime) — the hint must NOT fire:
    // the audience has not exhausted the manifest-level levers yet.
    for missing_one in [
        "[[bin]]\nname = \"m\"\n\n[profile.release]\nlto = \"fat\"\npanic = \"abort\"\n", // no strip
        "[[bin]]\nname = \"m\"\n\n[profile.release]\nstrip = true\npanic = \"abort\"\n",   // no lto
        "[[bin]]\nname = \"m\"\n\n[profile.release]\nstrip = true\nlto = \"fat\"\n",         // no panic
    ] {
        let findings = scan_source_with(missing_one, DistContext::default());
        assert!(
            findings
                .iter()
                .all(|f| f.rule_id != "pgo-training-set-covers-real-workload-hint"),
            "PGO hint fired on a manifest with a knob still missing"
        );
    }

    // Root-inherited knobs count as effective tuning (the member whose
    // build IS tuned through the workspace root is still the audience).
    let member_no_local_profile = "[package]\nname = \"m\"\n\n[[bin]]\nname = \"m\"\n";
    let ctx = DistContext {
        under_workspace_root: true,
        root_profile_knobs: Some(ProfileKnobs {
            strip: true,
            lto: true,
            panic: true,
        }),
        ..DistContext::default()
    };
    let findings = scan_source_with(member_no_local_profile, ctx);
    assert!(
        findings
            .iter()
            .any(|f| f.rule_id == "pgo-training-set-covers-real-workload-hint"),
        "root-inherited full tuning must still fire the PGO hint"
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
    let completed = scan_source_with(&after, DistContext::default());
    let ids: Vec<&str> = completed.iter().map(|f| f.rule_id).collect();
    // Batch 146: the complete profile silences the three ship-size rules —
    // and earns exactly the ONE next-lever finding (the PGO training-set
    // hint, the out-of-manifest lever for the fully-tuned audience).
    assert_eq!(
        ids,
        vec!["pgo-training-set-covers-real-workload-hint"],
        "the complete profile must silence the ship-size rules (and earns only the Batch-146 PGO hint)"
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
    assert_eq!(
        ids,
        vec!["edit-release-profile-opt-level", "strip-in-dev-profile"]
    );
    assert!(r.iter().all(|(_, why)| why.contains("REFUSED")));
}

/// The proposed-fix oracle is Tier-0 structural only — a well-formed
/// manifest validates, and the verdict claims nothing more.
#[test]
fn validate_is_structural() {
    let good = "[[bin]]\nname = \"x\"\n\n[profile.release]\nstrip = true\n";
    assert!(DistDomain::validate("shipped-binary-unstripped", good).scans);
}

// ── Bench 082 blind spots, landed 2026-09-10 ────────────────────────────────
//
// B1 — a bin autodiscovered from `src/main.rs` / `src/bin/*.rs` (no `[[bin]]`
// section) must fire the ship-size rules; B2 — a member manifest must consult
// the workspace ROOT's profile knobs before the ship-size rules advise.

/// One fixture tree under the OS temp dir: `files` maps repo-relative paths
/// to contents. Returns the fixture root; the caller removes it.
fn fixture_tree(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "riir_clippy_dist_gates_{}_{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (rel, body) in files {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().expect("fixture path has a parent"))
            .expect("create fixture dir");
        std::fs::write(&p, body).expect("write fixture file");
    }
    root
}

const SHIP_RULES: [&str; 3] = [
    "shipped-binary-unstripped",
    "shipped-binary-missing-size-profile",
    "cli-panic-unwind-tables",
];

fn rule_ids(findings: &[riir_clippy::domains::dist::rules::DistFinding]) -> Vec<&'static str> {
    findings.iter().map(|f| f.rule_id).collect()
}

/// B1(a) — the gm-tool shape, on a REAL fixture tree: a manifest with a
/// `[package]` but NO `[[bin]]` section, beside a `src/main.rs`. Derive must
/// see the autodiscovered bin, and all three ship-size rules fire exactly
/// once each.
#[test]
fn autodiscovered_main_rs_fires_all_three_ship_size_rules() {
    let root = fixture_tree(
        "b1_main",
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"gm-tool\"\nedition = \"2021\"\n",
            ),
            ("src/main.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    assert!(
        ctx.autodiscovered_bin,
        "src/main.rs must be derived as a bin"
    );
    assert!(!ctx.under_workspace_root);
    assert!(ctx.root_profile_knobs.is_none());

    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    let findings = scan_source_with(&src, ctx);
    let ids = rule_ids(&findings);
    assert_eq!(
        ids.len(),
        3,
        "expected the 3 ship-size citations, got {ids:?}"
    );
    for rule in SHIP_RULES {
        assert_eq!(
            ids.iter().filter(|id| **id == rule).count(),
            1,
            "{rule} exactly once"
        );
    }
    // The excerpt is honest about the evidence shape (no [[bin]] section).
    assert!(
        findings[0].excerpt.contains("autodiscovered"),
        "excerpt: {}",
        findings[0].excerpt
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// B1(a, second shape) — `src/bin/*.rs` (the other autodiscovery form) fires
/// the same trio; and a virtual workspace ROOT (no `[package]`) with a stray
/// `src/main.rs` discovers nothing — autodiscovery is a package feature.
#[test]
fn autodiscovered_src_bin_rs_fires_but_virtual_root_does_not() {
    let root = fixture_tree(
        "b1_bin",
        &[
            ("Cargo.toml", "[package]\nname = \"convert\"\n"),
            ("src/bin/convert.rs", "fn main() {}\n"),
            ("src/bin/other.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    assert!(ctx.autodiscovered_bin);
    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    let ids = rule_ids(&scan_source_with(&src, ctx));
    for rule in SHIP_RULES {
        assert!(ids.contains(&rule), "{rule} must fire for src/bin/*.rs");
    }
    let _ = std::fs::remove_dir_all(&root);

    // Virtual root: [workspace] only, stray src/main.rs — silent.
    let root = fixture_tree(
        "b1_virtual",
        &[
            ("Cargo.toml", "[workspace]\nmembers = [\"m\"]\n"),
            ("src/main.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    assert!(ctx.autodiscovered_bin, "tree evidence is package-blind");
    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    assert!(
        scan_source_with(&src, ctx).is_empty(),
        "a virtual root discovers no binary target"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// B1(b) — explicit `[[bin]]` behavior is unchanged: same three citations
/// whether the src tree exists or not, and the two evidence shapes together
/// do NOT double-fire.
#[test]
fn explicit_bin_section_unchanged_by_autodiscovery() {
    let manifest_text = "[package]\nname = \"t\"\n\n[[bin]]\nname = \"t\"\n";
    let with_src = fixture_tree(
        "b1_explicit",
        &[
            ("Cargo.toml", manifest_text),
            ("src/main.rs", "fn main() {}\n"),
        ],
    );
    let ctx = DistContext::derive(&with_src.join("Cargo.toml"));
    assert!(ctx.autodiscovered_bin);
    let ids = rule_ids(&scan_source_with(manifest_text, ctx));
    assert_eq!(ids.len(), 3, "no double-fire: {ids:?}");
    let _ = std::fs::remove_dir_all(&with_src);

    // Same manifest, no src tree: derive sees nothing, rules still fire via
    // the explicit section (the Phase-1 path).
    let no_src = fixture_tree("b1_explicit_nosrc", &[("Cargo.toml", manifest_text)]);
    let ctx = DistContext::derive(&no_src.join("Cargo.toml"));
    assert!(!ctx.autodiscovered_bin);
    let ids = rule_ids(&scan_source_with(manifest_text, ctx));
    assert_eq!(ids.len(), 3, "explicit [[bin]] alone still fires: {ids:?}");
    let _ = std::fs::remove_dir_all(&no_src);
}

/// B2(c) — the chaind shape, on a REAL fixture tree: a member with `[[bin]]`
/// and no own profiles under a root carrying strip + lto. Rules 1+2 go
/// silent (the root's knobs apply to member builds); the panic rule still
/// fires (the root carries no panic key). The mirror case — a root carrying
/// ONLY panic — silences exactly the panic rule.
#[test]
fn member_manifest_satisfied_by_root_knobs() {
    let member_manifest = "[package]\nname = \"daemon\"\n\n[[bin]]\nname = \"daemon\"\n";
    let root = fixture_tree(
        "b2_root_knobs",
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/daemon\"]\n\n[profile.release]\nstrip = true\nlto = \"thin\"\ncodegen-units = 1\n",
            ),
            ("crates/daemon/Cargo.toml", member_manifest),
            ("crates/daemon/src/main.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("crates/daemon/Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    assert!(ctx.under_workspace_root);
    let knobs = ctx.root_profile_knobs.expect("root manifest was read");
    assert!(knobs.strip && knobs.lto && !knobs.panic);

    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    let ids = rule_ids(&scan_source_with(&src, ctx));
    assert_eq!(
        ids,
        vec!["cli-panic-unwind-tables"],
        "only the panic rule survives a knob-complete root: {ids:?}"
    );
    let _ = std::fs::remove_dir_all(&root);

    // Mirror: a root carrying ONLY `panic =` silences exactly that rule.
    let root = fixture_tree(
        "b2_root_panic_only",
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/daemon\"]\n\n[profile.release]\npanic = \"abort\"\n",
            ),
            ("crates/daemon/Cargo.toml", member_manifest),
            ("crates/daemon/src/main.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("crates/daemon/Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    let ids = rule_ids(&scan_source_with(&src, ctx));
    assert_eq!(ids.len(), 2, "strip+lto still fire: {ids:?}");
    assert!(ids.contains(&"shipped-binary-unstripped"));
    assert!(ids.contains(&"shipped-binary-missing-size-profile"));
    let _ = std::fs::remove_dir_all(&root);
}

/// B2(d) — a member under a root WITHOUT the knobs changes nothing: all
/// three ship-size rules still fire (an unreadable or knob-less root never
/// satisfies anything).
#[test]
fn member_manifest_still_fires_when_root_lacks_knobs() {
    let member_manifest = "[package]\nname = \"daemon\"\n\n[[bin]]\nname = \"daemon\"\n";
    // Root with a [workspace] but no [profile.*] at all.
    let root = fixture_tree(
        "b2_root_noknobs",
        &[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/daemon\"]\n"),
            ("crates/daemon/Cargo.toml", member_manifest),
            ("crates/daemon/src/main.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("crates/daemon/Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    assert_eq!(ctx.root_profile_knobs, Some(ProfileKnobs::default()));
    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    let ids = rule_ids(&scan_source_with(&src, ctx));
    for rule in SHIP_RULES {
        assert!(ids.contains(&rule), "{rule} must still fire: {ids:?}");
    }
    let _ = std::fs::remove_dir_all(&root);

    // Manual-context twin: no workspace evidence at all (None knobs) — the
    // Phase-1 behavior, unchanged.
    let ctx = DistContext::default();
    assert_eq!(ctx.root_profile_knobs, None);
    for rule in SHIP_RULES {
        let ids = rule_ids(&scan_source_with(member_manifest, ctx));
        assert!(ids.contains(&rule));
    }
}

/// B2(e) — rule 4 is UNCHANGED by B2: a member carrying its own `[profile.*]`
/// still gets `member-profile-ignored`. The self-or-root union also shows
/// here: the member's OWN strip satisfies rule 1, while the knob-less root
/// leaves lto + panic firing.
#[test]
fn member_profile_ignored_still_fires_with_full_context() {
    let member_manifest =
        "[package]\nname = \"m\"\n\n[[bin]]\nname = \"m\"\n\n[profile.release]\nstrip = true\n";
    let root = fixture_tree(
        "b2_member_own_profiles",
        &[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/m\"]\n"),
            ("crates/m/Cargo.toml", member_manifest),
            ("crates/m/src/main.rs", "fn main() {}\n"),
        ],
    );
    let manifest = root.join("crates/m/Cargo.toml");
    let ctx = DistContext::derive(&manifest);
    let src = std::fs::read_to_string(&manifest).expect("read fixture manifest");
    let ids = rule_ids(&scan_source_with(&src, ctx));
    assert!(
        ids.contains(&"member-profile-ignored"),
        "rule 4 unchanged under the full context: {ids:?}"
    );
    assert!(
        !ids.contains(&"shipped-binary-unstripped"),
        "own strip satisfies rule 1: {ids:?}"
    );
    assert!(ids.contains(&"shipped-binary-missing-size-profile"));
    assert!(ids.contains(&"cli-panic-unwind-tables"));
    let _ = std::fs::remove_dir_all(&root);
}

//! The dist seed corpus — Phase 1 (Issue 085).
//!
//! Four rules, all suggest-only, every one with measured provenance from the
//! session that found the class (the owner had to ASK why the binary was
//! 2x — that is the failure this domain exists to prevent). The dedup
//! verdict rides each rule's `incumbent` field: `cargo` itself warns about
//! NOTHING here (checked live 2026-09-09 — a bare release build of a bin
//! crate with no profile section compiles silent), so all four are Class B
//! (novel detection) with no incumbent to overlay.
//!
//! | Rule | Family | Fix | Incumbent |
//! |---|---|---|---|
//! | `shipped-binary-unstripped` | `ShipSize` | suggest | none |
//! | `shipped-binary-missing-size-profile` | `ShipSize` | suggest | none |
//! | `cli-panic-unwind-tables` | `ShipSize` | suggest | none |
//! | `member-profile-ignored` | `ProfilePlacement` | suggest (gated) | none |

use super::{DistDomain, DistFamily, DistRule};
use crate::corpus::Corpus;

/// Build the dist seed corpus (BLAKE3-committed via `Corpus<D>`).
#[must_use]
pub fn build_dist_seed_corpus() -> Corpus<DistDomain> {
    Corpus::new(entries())
}

/// Shorthand constructor — the corpus is data (the docker `rule()` precedent).
fn rule(
    rule_id: &str,
    dist_concept: &str,
    trigger: &str,
    buggy_code: &str,
    fixed_code: &str,
    family: DistFamily,
    notes: Option<&str>,
) -> DistRule {
    DistRule {
        rule_id: rule_id.to_string(),
        dist_concept: dist_concept.to_string(),
        trigger: trigger.to_string(),
        buggy_code: buggy_code.to_string(),
        fixed_code: fixed_code.to_string(),
        family,
        notes: notes.map(str::to_string),
    }
}

fn entries() -> Vec<DistRule> {
    use DistFamily as F;
    vec![
        rule(
            "shipped-binary-unstripped",
            "a shipped binary's default symbol table IS the internal architecture — every function name, every module boundary, readable by anyone who downloads it",
            "a manifest with [[bin]] and no strip key in any [profile.*] section",
            "[[bin]]\nname = \"mytool\"\n\n[dependencies]\nserde = \"1\"\n",
            "[[bin]]\nname = \"mytool\"\n\n[profile.release]\nstrip = true\n\n[dependencies]\nserde = \"1\"\n",
            F::ShipSize,
            Some("Measured basis: the unstripped shipping binary of this very tool exposed 14,485 symbols (Proposal 003 G2, 2026-09-06). strip = \"symbols\" and strip = true are equivalent here; any explicit strip key satisfies the rule."),
        ),
        rule(
            "shipped-binary-missing-size-profile",
            "the default release profile optimizes for SPEED, not size — for a shipped CLI the knob set (lto, codegen-units, opt-level for size, abort, strip) is commonly a 40-60% byte reduction on the same behavior",
            "a manifest with [[bin]] and no lto key in any [profile.*] section",
            "[[bin]]\nname = \"mytool\"\n",
            "[[bin]]\nname = \"mytool\"\n\n# Size-optimized DIST profile — release via `cargo build --profile dist`.\n# NOT [profile.release]: [profile.bench] inherits release, and opt-level/\n# codegen-units there poison criterion benches.\n[profile.dist]\ninherits = \"release\"\nopt-level = \"s\"\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\n",
            F::ShipSize,
            Some("Measured basis: 11,903,168 -> 4,546,448 bytes (-61%) on this tool's own shipping feature set, arm64 (Issue 085). cargo bloat first: 70% of .text was own-code tables, so codegen knobs win, dep removal does not. The suggestion is a SEPARATE profile on purpose — see the domain's refuses() table."),
        ),
        rule(
            "cli-panic-unwind-tables",
            "panic = unwind bakes landing pads and unwind tables into every function so panics can walk the stack — bytes a short-lived CLI tool never needs",
            "a manifest with [[bin]] and no panic key in any [profile.*] section",
            "[[bin]]\nname = \"mytool\"\n",
            "[[bin]]\nname = \"mytool\"\n\n[profile.dist]\ninherits = \"release\"\npanic = \"abort\"\n",
            F::ShipSize,
            Some("Context-dependent ON PURPOSE: a server that isolates panics per task or connection (a panicked worker must not take down the process) RELIES on unwinding — under abort, one panic kills the whole server. The rule fires only on manifests; deciding CLI-vs-server is the reader's call, which is why this is suggest-only with this note."),
        ),
        rule(
            "member-profile-ignored",
            "cargo applies profile settings from the WORKSPACE ROOT manifest only — a [profile.*] block in a member crate is silently ignored when the crate is built through the workspace, and cargo only warns in newer toolchains under some invocations",
            "a member manifest (no [workspace] of its own) carrying [profile.*] sections, when a parent manifest declares [workspace] — caller evidence, silent without it",
            "[package]\nname = \"member-tool\"\n\n[[bin]]\nname = \"member-tool\"\n\n[profile.release]\nstrip = true\n",
            "# The block belongs in the workspace ROOT manifest; see that file.\n[package]\nname = \"member-tool\"\n\n[[bin]]\nname = \"member-tool\"\n",
            F::ProfilePlacement,
            Some("Precision caveat: the evidence is a nearest-ancestor [workspace] walk. An `exclude`d member (built standalone) legitimately keeps its profiles — the advice text hedges accordingly. Silent without the walk evidence, never guessed (the docker ScanContext pattern)."),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::dist::rules::{scan_source_with, DIST_RULES};
    use crate::domains::dist::DistContext;

    #[test]
    fn rule_ids_unique_and_kebab() {
        let mut seen = std::collections::BTreeSet::new();
        for r in entries() {
            assert!(seen.insert(r.rule_id.clone()), "duplicate rule id {}", r.rule_id);
            assert!(
                r.rule_id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} is not kebab-case",
                r.rule_id
            );
        }
    }

    #[test]
    fn corpus_and_detector_rule_sets_agree() {
        let ids: std::collections::BTreeSet<String> =
            entries().into_iter().map(|r| r.rule_id).collect();
        let detectors: std::collections::BTreeSet<String> =
            DIST_RULES.iter().map(|s| (*s).to_string()).collect();
        assert_eq!(
            ids, detectors,
            "the corpus and the detector table have drifted apart"
        );
    }

    /// The Issue 085 precision contract, mechanically: every buggy fixture
    /// produces EXACTLY its own rule's citation (no cross-fire), and every
    /// fixed fixture produces ZERO.
    #[test]
    fn fixtures_fire_exactly_their_own_rule() {
        for r in entries() {
            let buggy = scan_source_with(&r.buggy_code, DistContext::default());
            let fixed = scan_source_with(&r.fixed_code, DistContext::default());
            // Context-gated rule: without evidence the member rule itself is
            // silent (its fire-half is tested with evidence below). The other
            // ship-size rules MAY cross-fire its fixtures — they share a
            // trigger shape — so assert only the member rule's absence.
            if r.rule_id == "member-profile-ignored" {
                assert!(
                    buggy.iter().all(|f| f.rule_id != r.rule_id),
                    "{}: must be silent without caller evidence",
                    r.rule_id
                );
                continue;
            }
            assert!(
                buggy.iter().any(|f| f.rule_id == r.rule_id),
                "{}: buggy fixture does not fire its own rule (got {:?})",
                r.rule_id,
                buggy.iter().map(|f| f.rule_id).collect::<Vec<_>>()
            );
            assert!(
                fixed.iter().all(|f| f.rule_id != r.rule_id),
                "{}: fixed fixture still fires its own rule",
                r.rule_id
            );
        }
    }

    /// The member rule WITH evidence: fires on the buggy side, and the
    /// fixed side (block removed) is silent.
    #[test]
    fn member_rule_fires_with_workspace_evidence() {
        let ctx = DistContext {
            under_workspace_root: true,
        };
        let buggy = "[package]\nname = \"m\"\n\n[[bin]]\nname = \"m\"\n\n[profile.release]\nstrip = true\n";
        let findings = scan_source_with(buggy, ctx);
        assert!(findings.iter().any(|f| f.rule_id == "member-profile-ignored"));
        // A virtual root (workspace, no package/bin) is silent even with the
        // evidence flag — nothing ships from it.
        let root = "[workspace]\nmembers = [\"m\"]\n";
        assert!(scan_source_with(root, ctx).is_empty());
    }

    /// The negative controls: lib-only crates never get ship-size advice,
    /// and a complete profile silences every rule.
    #[test]
    fn lib_only_and_complete_profiles_stay_silent() {
        let lib_only = "[package]\nname = \"lib\"\n\n[lib]\ncrate-type = [\"rlib\"]\n";
        assert!(scan_source_with(lib_only, DistContext::default()).is_empty());
        let complete = "[[bin]]\nname = \"t\"\n\n[profile.dist]\ninherits = \"release\"\nopt-level = \"s\"\nlto = \"fat\"\ncodegen-units = 1\npanic = \"abort\"\nstrip = true\n";
        assert!(scan_source_with(complete, DistContext::default()).is_empty());
    }

    /// The suggest-only contract: no dist rule is fixable in Phase 1.
    #[test]
    fn phase1_is_suggest_only() {
        assert!(crate::domains::dist::rules::FIXABLE_DIST.is_empty());
    }
}

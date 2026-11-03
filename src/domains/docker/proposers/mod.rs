//! Stage 2 proposal mechanism for the `docker` domain (Plan 030 T1.4).
//!
//! Unlike the Rust domains there is no per-rule proposer module: detection
//! and the bounded rewrite are authored together in [`super::rules`], so a
//! rule physically cannot detect one shape and rewrite another. This module
//! is the dispatch + the discipline around it.
//!
//! ## Conservative gates
//!
//! A wrong Dockerfile "fix" breaks a build or, worse, changes runtime
//! signal semantics. Every proposer refuses more than it fires:
//!
//! | Refusal | Why |
//! |---|---|
//! | shell-form `CMD`/`ENTRYPOINT` carrying a metacharacter | the shell is doing real work; exec form would change behavior |
//! | `pipefail` on a non-debian base | `/bin/sh` on alpine/busybox has no `pipefail` — the "fix" would break the build |
//! | `ADD` of a URL or an archive | `ADD` unpacks and fetches; `COPY` does neither |
//! | `--locked` without committed-lockfile evidence | fails the build outright when no lockfile exists |
//! | any `RUN` merge but the adjacent apt update/install pair | the DL3059 refusal ([`super::refuses`]) |
//! | `rm -rf` appended to a heredoc `RUN` | the append would land outside the script |
//! | a target-dir cache mount | the path depends on `WORKDIR`; only the path-independent `CARGO_HOME` mounts are emitted |
//!
//! Every applied fix additionally rides the `--fix --verify` gate, and
//! `docker_verify` adds the hadolint agreement check for Class-A rules.

use super::rules::{self, ScanContext};

// Shared emitter substrate (re-exported like the other domains' proposers).
pub use crate::edit_emitter::{apply_edits, RuleEdit, SourceEdit};

pub use super::rules::FIXABLE_DOCKER;

/// Collect one rule's edits for the source (empty for unknown or
/// suggest-only rules).
#[must_use]
pub fn collect_edits(rule_id: &str, source: &str) -> Vec<RuleEdit> {
    collect_edits_with(rule_id, source, &ScanContext::default())
}

/// [`collect_edits`] with caller-supplied evidence for the gated rules.
#[must_use]
pub fn collect_edits_with(rule_id: &str, source: &str, ctx: &ScanContext) -> Vec<RuleEdit> {
    if FIXABLE_DOCKER.contains(&rule_id) {
        rules::scan_source_with(source, ctx)
            .into_iter()
            .filter(|f| f.rule_id == rule_id)
            .filter_map(|f| {
                f.fix.map(|edit| RuleEdit {
                    rule_id: rule_id.to_string(),
                    edit,
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Apply every bounded fix the source triggers, in one pass.
#[must_use]
pub fn fix_all(source: &str, ctx: &ScanContext) -> String {
    let edits: Vec<SourceEdit> = rules::scan_source_with(source, ctx)
        .into_iter()
        .filter(|f| FIXABLE_DOCKER.contains(&f.rule_id))
        .filter_map(|f| f.fix)
        .collect();
    apply_edits(source, edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::docker::corpus::build_docker_seed_corpus;

    #[test]
    fn fixable_docker_are_corpus_rules() {
        let corpus = build_docker_seed_corpus();
        for id in FIXABLE_DOCKER {
            assert!(
                corpus.entries.iter().any(|r| r.rule_id == *id),
                "FIXABLE_DOCKER lists {id}, which is not a corpus rule"
            );
        }
    }

    #[test]
    fn every_fixable_rule_fires_on_its_fixture() {
        let corpus = build_docker_seed_corpus();
        for id in FIXABLE_DOCKER {
            let rule = corpus
                .entries
                .iter()
                .find(|r| r.rule_id == *id)
                .expect("corpus rule");
            let edits = collect_edits_with(id, &rule.buggy_code, &fixture_ctx());
            assert!(
                !edits.is_empty(),
                "{id} produced no edit on its own fixture"
            );
        }
    }

    #[test]
    fn every_fixable_rule_reproduces_its_fixed_fixture() {
        let corpus = build_docker_seed_corpus();
        for id in FIXABLE_DOCKER {
            let rule = corpus
                .entries
                .iter()
                .find(|r| r.rule_id == *id)
                .expect("corpus rule");
            let edits: Vec<SourceEdit> = collect_edits_with(id, &rule.buggy_code, &fixture_ctx())
                .into_iter()
                .map(|e| e.edit)
                .collect();
            let got = apply_edits(&rule.buggy_code, edits);
            assert_eq!(
                got.trim_end(),
                rule.fixed_code.trim_end(),
                "{id}: proposer output does not reproduce the corpus fixed_code"
            );
        }
    }

    /// Rules whose SUBJECT lives outside the Dockerfile, so their fixed
    /// fixture cannot be clean by construction — the fix is a new file or a
    /// changed `docker build` invocation, not an edit to this text.
    ///
    /// Named explicitly rather than papered over with a doctored fixture: a
    /// fixture that pretended to fix an external defect would be a lie in
    /// the corpus, and the corpus is what the drafter learns from.
    const EXTERNALLY_FIXED: &[&str] = &["dockerignore-missing"];

    #[test]
    fn clean_spans_fire_no_lint() {
        // Every rule's own FIXED fixture must be clean under that rule —
        // otherwise the fix does not resolve what it detected.
        let corpus = build_docker_seed_corpus();
        for rule in &corpus.entries {
            if EXTERNALLY_FIXED.contains(&rule.rule_id.as_str()) {
                continue;
            }
            let findings = rules::scan_source_with(&rule.fixed_code, &fixture_ctx());
            assert!(
                !findings.iter().any(|f| f.rule_id == rule.rule_id),
                "{} still fires on its own fixed_code",
                rule.rule_id
            );
        }
    }

    /// The fixture context supplies every gate, so gated rules are
    /// exercised by their fixtures even though a text-only scan is silent.
    fn fixture_ctx() -> ScanContext {
        ScanContext {
            lockfile_committed: Some(true),
            workspace_multi_member: Some(true),
            dockerignore_present: Some(false),
            multi_platform_build: Some(true),
        }
    }

    #[test]
    fn unknown_rule_yields_no_edits() {
        assert_eq!(
            collect_edits("not-a-rule", "FROM scratch\n"),
            [] as [crate::edit_emitter::RuleEdit; 0]
        );
    }
}

//! The `dist` domain — shipped-artifact build-config healing (Issue 085).
//!
//! Sixth retrieval `HealDomain`. Scope: **Cargo.toml build-configuration
//! patterns whose fix is measured, not guessed** — release-profile size knobs
//! (lto / codegen-units / opt-level / panic / strip) for crates that SHIP a
//! binary, and the workspace-member profile trap. Phase 1 is SUGGEST-ONLY:
//! the domain cites the rule and the advice; it does not write manifests.
//!
//! ## Why this domain exists (the measured delta, Issue 085)
//!
//! The shipped cargo-refine binary rode plain `--release` through v0.1.4:
//! 11,903,168 bytes stripped where the knob set measures 4,546,448 (-61%) —
//! and nobody noticed until the owner asked. `binary_size_gate.sh` watches
//! THIS repo's release lane at PACKAGE time; this domain watches EVERY
//! manifest the healer scans at DEV time, before the bloat ships. Found by
//! `cargo bloat`: 70% of `.text` was own-code corpus tables — codegen knobs
//! win, dep removal does not.
//!
//! ## The oracle question, answered by precedent
//!
//! The domain verifies fixes by SCAN well-formedness (Tier 0, the docker
//! pattern) — never by rebuilding and measuring bytes. That is the house
//! answer, not a compromise: `rust_perf`'s fixes verify by `cargo test`
//! re-run, not by benchmarking each fix. The GAIN is proven once, at class
//! level, by the measurement above; the per-fix oracle only has to prove the
//! edit is well-formed and the trigger is real.
//!
//! ## What this domain is NOT (Issue 085, non-negotiable)
//!
//! - NOT a general TOML linter; NOT dependency auditing (cargo-machete /
//!   cargo-deny own those).
//! - NEVER advice that edits `[profile.release]` directly when a separate
//!   `[profile.dist]` serves: `[profile.bench]` inherits release, and
//!   opt-level/codegen-units edits there poison the criterion benches (the
//!   measured reason this repo keeps release clean). Encoded in
//!   [`refuses`].
//! - NOT profile surgery on library-only crates — every shipped-artifact
//!   rule requires a shipped binary: an explicit `[[bin]]` section OR cargo's
//!   autodiscovery (`src/main.rs` / `src/bin/*.rs` — blind spot B1).
//!   Workspace-aware knob resolution (a member inherits the ROOT manifest's
//!   profiles — blind spot B2) lives in [`context`].

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod context;
pub mod corpus;
pub mod rules;

pub use context::{DistContext, ProfileKnobs};

/// The rule family (parallel to `SecFamily` / `DockerFamily`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistFamily {
    /// Shipped-binary size knobs (strip / size profile / panic).
    ShipSize,
    /// Profiles in the wrong manifest (workspace-member trap).
    ProfilePlacement,
    /// Compile-time/build-cost knobs (dependency-shape advice).
    BuildTime,
}

impl DistFamily {
    /// Stable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::ShipSize => "ship-size",
            Self::ProfilePlacement => "profile-placement",
            Self::BuildTime => "build-time",
        }
    }
}

/// One dist rule: buggy/fixed manifest fixture pair + family.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistRule {
    /// The rule id (kebab-case, the house convention).
    pub rule_id: String,
    /// The concept in one line (the retrieval surface reads this).
    pub dist_concept: String,
    /// What textual shape triggers the rule, in prose.
    pub trigger: String,
    /// The buggy manifest that triggers the rule.
    pub buggy_code: String,
    /// The fixed manifest.
    pub fixed_code: String,
    /// The family.
    pub family: DistFamily,
    /// Context annotation (the `context_notes` channel) — carries the
    /// bench-inheritance and panic-isolation caveats.
    pub notes: Option<String>,
}

impl HealEntry for DistRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.buggy_code
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.fixed_code
    }

    #[inline]
    fn context_notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

/// The Tier-0 verdict: the proposed manifest re-scans cleanly (sections
/// parse, no exotic headers). Deliberately NOT a TOML validation — the
/// oracle question is answered at class level (see the module header).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistVerdict {
    /// Whether the proposed manifest re-scans without structural complaints.
    pub scans: bool,
}

/// The dist healing domain — `HealDomain` impl.
#[derive(Debug, Clone, Copy, Default)]
pub struct DistDomain;

impl HealDomain for DistDomain {
    type Entry = DistRule;
    type Verdict = DistVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_dist_seed_corpus().entries
    }

    fn validate(_rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        let scan = rules::scan(proposed_fix);
        DistVerdict {
            scans: rules::well_formed_enough(&scan),
        }
    }
}

/// The advice this domain REFUSES to emit, and why (the docker `refuses`
/// precedent — encoded as data so the refusal gate can assert it).
#[must_use]
pub fn refuses() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "edit-release-profile-opt-level",
            "opt-level/codegen-units in [profile.release] poison the criterion \
benches: [profile.bench] inherits release, so the GOAT benches would measure \
size-optimized code and every prior bench number goes stale. The advice is \
ALWAYS a separate [profile.dist] block + --profile dist. REFUSED.",
        ),
        (
            "strip-in-dev-profile",
            "strip belongs in release/dist artifacts; a dev profile stripped of \
symbols breaks debuggers and backtraces for zero shipped-byte gain. REFUSED.",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_scans_well_formed_manifest() {
        let v = DistDomain::validate(
            "shipped-binary-unstripped",
            "[[bin]]\nname = \"x\"\n\n[profile.release]\nstrip = true\n",
        );
        assert!(v.scans);
    }

    #[test]
    fn validate_rejects_nothing_structural_yet() {
        // Phase 1: the scanner is line-based and tolerant — a garbage body
        // under clean headers still "scans". The verdict's honesty is that it
        // claims NOTHING beyond structure.
        let v = DistDomain::validate("shipped-binary-unstripped", "[[bin]]\nname =\n");
        assert!(v.scans);
    }

    #[test]
    fn refusal_table_is_stable() {
        let r = refuses();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "edit-release-profile-opt-level");
        assert_eq!(r[1].0, "strip-in-dev-profile");
    }
}

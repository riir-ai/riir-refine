//! The `docker` domain — Dockerfile bounded-fix healing + retrieval
//! (Proposal 006, Plan 030).
//!
//! Fifth retrieval `HealDomain`. Scope: **Dockerfile source patterns with
//! bounded fix-spaces** — build-cache performance, image size, package
//! hygiene, secrets, PID1/signal runtime correctness, and multi-platform
//! cross-builds, Rust-toolchain-first.
//!
//! ## Why this domain exists (the measured delta, Research 086)
//!
//! hadolint (71 DL rules) and `docker build --check` (21 checks) are both
//! **detection-only** — verified live, no fix flag, zero autofix issues.
//! Fixing is unoccupied. The cache/build-speed axis is unoccupied on the
//! DETECTION side too. So the domain splits in two:
//!
//! - **Class A — fix overlay.** hadolint detects, we fix. Our detector must
//!   AGREE with hadolint on the fixture (the `clippy_verify` symmetry);
//!   `docker_verify` runs that agreement gate when the binary is on PATH.
//! - **Class B — novel detection.** Both incumbents are silent; we detect
//!   AND fix (or annotate, when the call is a judgment).
//!
//! ## What this domain is NOT (Proposal 006, non-negotiable)
//!
//! - NOT compose/k8s manifests, NOT GitHub Actions (zizmor owns it), NOT
//!   image scanning (trivy/grype/dockle), NOT base-image version auditing
//!   (renovate/dependabot).
//! - NOT a registry client — digest resolution is suggest-only TEXT, never
//!   a network lookup.
//! - NOT a Cargo.toml/lockfile editor. The healer edits the Dockerfile and
//!   nothing else.
//!
//! ## The refusals are load-bearing (caveat 5)
//!
//! Some incumbent advice is ANTI-MODERN and the healer must refuse to move
//! toward it. DL3059 (consolidate consecutive `RUN`s) fights `BuildKit` layer
//! caching; `apt-get clean` is redundant under the default docker-clean
//! config; LABEL-merging is lore. [`refuses`] names them and
//! `tests/docker_refusal_gate.rs` pins that no proposer ever emits them.
//!
//! ## No `syn`
//!
//! Dockerfiles are scanned by [`scanner`] — a hand-written line scanner,
//! zero new dependencies (no regex either). Rules read stanzas, not tokens;
//! precision guards + the T5 control gate (`tests/docker_t5_control.rs`)
//! carry the weight `syn` carries in the Rust domains.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;
pub mod proposers;
pub mod rules;
pub mod scanner;

#[cfg(feature = "docker_verify")]
pub mod verify;

#[cfg(feature = "docker_buildcheck")]
pub mod buildcheck;

/// The rule family (parallel to `SecFamily` / clippy's `lint_group`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockerFamily {
    /// Build-cache / layer-ordering performance (Class B's core axis).
    CachePerf,
    /// Final image size.
    ImageSize,
    /// Base-image selection (mostly suggest-only — measured trade-offs).
    BaseChoice,
    /// Package-manager hygiene (apt/apk lists, recommends, pinning).
    PkgHygiene,
    /// `RUN` correctness (pipefail, shell semantics, same-run coupling).
    RunCorrectness,
    /// PID1 / signal delivery (exec form, entrypoint scripts).
    SignalPid1,
    /// Secrets hygiene (build args carrying secrets, secret mounts).
    SecretsHygiene,
    /// Multi-platform / cross-build correctness.
    MultiPlatform,
    /// Rust-toolchain idiom (`--locked`, `-p`, chef/sccache, musl).
    RustIdiom,
}

impl DockerFamily {
    /// Stable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::CachePerf => "cache-perf",
            Self::ImageSize => "image-size",
            Self::BaseChoice => "base-choice",
            Self::PkgHygiene => "pkg-hygiene",
            Self::RunCorrectness => "run-correctness",
            Self::SignalPid1 => "signal-pid1",
            Self::SecretsHygiene => "secrets-hygiene",
            Self::MultiPlatform => "multi-platform",
            Self::RustIdiom => "rust-idiom",
        }
    }
}

/// The minimum `# syntax=docker/dockerfile:` frontend a rule's FIX requires.
///
/// Dockerfile semantics are frontend-version-dependent (Proposal 006 caveat
/// 2). A rule whose fix emits a heredoc cannot fire on a file pinned below
/// 1.4; a rule emitting `COPY --exclude` needs 1.19. Rules declare their
/// floor and [`DockerRule::fix_allowed_under`] refuses below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SyntaxVersion {
    /// The builtin frontend — no directive needed (`RUN --mount` is stable
    /// here on any supported daemon).
    Builtin,
    /// `docker/dockerfile:1.4` — heredocs, `COPY --link`.
    V1_4,
    /// `docker/dockerfile:1.19` — `COPY --exclude`.
    V1_19,
}

impl SyntaxVersion {
    /// The `# syntax=` value that satisfies this floor, for the fix's
    /// accompanying advice. `None` for [`SyntaxVersion::Builtin`].
    #[must_use]
    pub fn directive(self) -> Option<&'static str> {
        match self {
            Self::Builtin => None,
            Self::V1_4 => Some("docker/dockerfile:1.4"),
            Self::V1_19 => Some("docker/dockerfile:1.19"),
        }
    }

    /// Parse the numeric floor out of a `# syntax=` value
    /// (`docker/dockerfile:1.7-labs` → `V1_4`-satisfying).
    ///
    /// Conservative by construction: an unparseable or unpinned value
    /// (`docker/dockerfile:1`, a custom frontend) yields `Builtin` — the
    /// scanner never *upgrades* a file's declared capability on a guess.
    #[must_use]
    pub fn from_directive(value: &str) -> Self {
        let Some((_, tag)) = value.rsplit_once(':') else {
            return Self::Builtin;
        };
        let digits: String = tag
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let mut parts = digits.split('.');
        let major: u32 = parts.next().unwrap_or("").parse().unwrap_or(0);
        let minor_tag = parts.next();
        // A tag with NO minor (`docker/dockerfile:1`) is a MOVING label —
        // Docker's own recommended form, always resolving to the latest
        // release of that major. It therefore satisfies every floor in that
        // major, and reading it as `1.0` gets this exactly backwards: the
        // healer would demand `1.4` on a file already carrying something
        // strictly newer, and refuse V1_4-floor fixes the frontend supports.
        //
        // MEASURED 2026-08-29, not inferred from the docs: under
        // `# syntax=docker/dockerfile:1`, `COPY --exclude=` — a 1.19-only
        // feature — passes `docker build --check` and builds green.
        // Found by adopting `copy-link-artifact-rebase` into riir-chain's
        // edge-wallet image, where writing the canonical directive made
        // `missing-syntax-directive` fire on a correct file.
        let Some(minor_tag) = minor_tag else {
            return match major {
                0 => Self::Builtin,
                _ => Self::V1_19,
            };
        };
        let minor: u32 = minor_tag.parse().unwrap_or(0);
        match (major, minor) {
            (1, m) if m >= 19 => Self::V1_19,
            (1, m) if m >= 4 => Self::V1_4,
            (n, _) if n >= 2 => Self::V1_19,
            _ => Self::Builtin,
        }
    }
}

/// One docker rule: buggy/fixed fixture pair + family + syntax floor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerRule {
    /// The rule id (kebab-case, the house convention).
    pub rule_id: String,
    /// The docker concept in one line (parallel to `sec_concept`).
    pub docker_concept: String,
    /// What textual shape triggers the rule (the detector's contract, in
    /// prose — the retrieval surface reads this).
    pub trigger: String,
    /// The buggy Dockerfile that triggers the rule.
    pub buggy_code: String,
    /// The fixed Dockerfile.
    pub fixed_code: String,
    /// The family.
    pub family: DockerFamily,
    /// The minimum frontend the FIX requires.
    pub min_syntax: SyntaxVersion,
    /// The incumbent's id when this is a Class-A overlay (`DL3009`,
    /// `JSONArgsRecommended`), `None` for Class-B novel detection. This is
    /// the Research 086 dedup ledger cross-ref, carried in the corpus so it
    /// cannot drift away from the rule.
    pub incumbent: Option<String>,
    /// Context annotation for rules whose fix is a judgment call rather
    /// than an unconditional win (the `rust_perf` `context_notes` channel).
    pub notes: Option<String>,
    /// The `BuildKit` check name when it differs from [`Self::incumbent`].
    ///
    /// Most Class-A rules have ONE incumbent and it lives in `incumbent`.
    /// A few are detected by BOTH tools under different ids — hadolint's
    /// `DL3025` and `BuildKit`'s `JSONArgsRecommended` cover the same
    /// shell-form `CMD`/`ENTRYPOINT` defect. `incumbent` names the primary
    /// (Tier-1-checkable) id; this names the Tier-2 one so the
    /// `docker_buildcheck` gate can reach a rule whose primary id is a
    /// `DL` code. Set via [`Self::with_buildkit_check`] rather than a 9th
    /// constructor argument: 26 of 27 rules do not need it.
    pub buildkit_check: Option<String>,
}

impl DockerRule {
    /// Whether the rule's fix may be applied to a file whose declared
    /// frontend is `available` (the caveat-2 syntax floor).
    #[must_use]
    pub fn fix_allowed_under(&self, available: SyntaxVersion) -> bool {
        available >= self.min_syntax
    }

    /// Class-A (overlays an incumbent detector) vs Class-B (novel).
    #[must_use]
    pub fn is_class_a(&self) -> bool {
        self.incumbent.is_some()
    }

    /// Declare the Tier-2 (`docker build --check`) id for a rule whose
    /// primary incumbent is a hadolint `DL` code.
    #[must_use]
    pub fn with_buildkit_check(mut self, check: &str) -> Self {
        self.buildkit_check = Some(check.to_string());
        self
    }

    /// The id the Tier-2 oracle should look for, if any: the explicit
    /// [`Self::buildkit_check`], else [`Self::incumbent`] when that is
    /// already a `BuildKit` check name rather than a `DL` code.
    #[must_use]
    pub fn tier2_check(&self) -> Option<&str> {
        match self.buildkit_check.as_deref() {
            Some(c) => Some(c),
            None => self.incumbent.as_deref().filter(|c| !c.starts_with("DL")),
        }
    }
}

impl HealEntry for DockerRule {
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

/// The parse-only verdict (Tier 0 — always available).
///
/// `hadolint_agrees` is `None` unless the `docker_verify` oracle ran and
/// the binary was on PATH. It is never silently `false`: absence of the
/// oracle degrades to `None` with one log line (the Issue 018 oracle-split
/// discipline), because "we could not check" and "the check failed" are
/// different facts and conflating them is how a gate starts lying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerVerdict {
    /// Whether the proposed fix scans as a well-formed Dockerfile.
    pub parses: bool,
    /// Whether every stanza carries a known instruction and the file opens
    /// a stage before any build instruction.
    pub stanza_valid: bool,
    /// Whether hadolint agrees the rule's finding is gone (Class A only).
    pub hadolint_agrees: Option<bool>,
}

/// The docker healing domain — `HealDomain` impl.
#[derive(Debug, Clone, Copy, Default)]
pub struct DockerDomain;

impl HealDomain for DockerDomain {
    type Entry = DockerRule;
    type Verdict = DockerVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_docker_seed_corpus().entries
    }

    fn validate(_rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        let scan = scanner::scan(proposed_fix);
        let parses = !scan.stanzas.is_empty();
        DockerVerdict {
            parses,
            stanza_valid: parses && stanza_valid(&scan, proposed_fix),
            hadolint_agrees: None,
        }
    }
}

/// Structural validity: every stanza is a known instruction (guaranteed by
/// the scanner), the file opens at least one stage, and nothing but `ARG`
/// (or a directive/comment) precedes the first `FROM`.
#[must_use]
pub fn stanza_valid(scan: &scanner::DockerfileScan, source: &str) -> bool {
    if scan.stages.is_empty() {
        false
    } else {
        scan.stanzas
            .iter()
            .filter(|s| s.scope == scanner::Scope::Global)
            .all(|s| s.instruction_of(source) == "ARG")
    }
}

/// The fixes this domain REFUSES to emit, and why (Proposal 006 caveat 5).
///
/// Encoded as data rather than prose so the refusal gate can assert it: a
/// heuristic that "helpfully" merged `RUN`s would be actively harmful, and
/// the only durable defense is a test that fails when one appears.
#[must_use]
pub fn refuses() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "consolidate-consecutive-runs",
            "DL3059 is ANTI-MODERN: separate RUNs are separate cache layers, and \
merging them makes every rebuild redo the merged work. BuildKit's layer cache \
is worth far more than the layer count. REFUSED.",
        ),
        (
            "apt-get-clean",
            "Redundant on debian-derived images since the default \
/etc/apt/apt.conf.d/docker-clean already discards the archives; the useful \
half (rm -rf /var/lib/apt/lists/*) is DL3009, which we DO fix. REFUSED as a \
separate emission.",
        ),
        (
            "merge-label-instructions",
            "Lore from the pre-BuildKit layer-count era. LABELs are metadata, not \
filesystem layers. REFUSED.",
        ),
        (
            "rewrite-qemu-native-to-buildplatform-cross",
            "BOTH are valid (Proposal 006 caveat 6): the in-tree deploy/Dockerfile \
deliberately builds QEMU-native alpine, and cross via $BUILDPLATFORM is 5.2x-7x \
on compile-heavy trees. The choice depends on dep complexity x matrix width. \
Annotated by a decision table, never rewritten.",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntax_floor_ordering() {
        assert!(SyntaxVersion::Builtin < SyntaxVersion::V1_4);
        assert!(SyntaxVersion::V1_4 < SyntaxVersion::V1_19);
    }

    #[test]
    fn syntax_from_directive_is_conservative() {
        assert_eq!(
            SyntaxVersion::from_directive("docker/dockerfile:1.7-labs"),
            SyntaxVersion::V1_4
        );
        assert_eq!(
            SyntaxVersion::from_directive("docker/dockerfile:1.19"),
            SyntaxVersion::V1_19
        );
        // An unpinned or unparseable frontend must NOT be read as capable.
        // A bare major is a MOVING label: latest 1.x, so every 1.x floor is
        // satisfied. Measured against the real frontend (see from_directive).
        assert_eq!(
            SyntaxVersion::from_directive("docker/dockerfile:1"),
            SyntaxVersion::V1_19
        );
        // ...but an explicitly pinned minor is exactly that minor.
        assert_eq!(
            SyntaxVersion::from_directive("docker/dockerfile:1.0"),
            SyntaxVersion::Builtin
        );
        assert_eq!(
            SyntaxVersion::from_directive("docker/dockerfile:1.4"),
            SyntaxVersion::V1_4
        );
        assert_eq!(
            SyntaxVersion::from_directive("mycorp/frontend"),
            SyntaxVersion::Builtin
        );
    }

    #[test]
    fn fix_allowed_respects_the_floor() {
        let r = DockerRule {
            rule_id: "x".into(),
            docker_concept: "c".into(),
            trigger: "t".into(),
            buggy_code: "FROM a\n".into(),
            fixed_code: "FROM a\n".into(),
            family: DockerFamily::CachePerf,
            min_syntax: SyntaxVersion::V1_4,
            incumbent: None,
            notes: None,
            buildkit_check: None,
        };
        assert!(!r.fix_allowed_under(SyntaxVersion::Builtin));
        assert!(r.fix_allowed_under(SyntaxVersion::V1_4));
        assert!(r.fix_allowed_under(SyntaxVersion::V1_19));
    }

    #[test]
    fn validate_reads_structure_not_rust() {
        let v = DockerDomain::validate("any", "FROM rust:1.97.0-alpine AS b\nRUN true\n");
        assert!(v.parses);
        assert!(v.stanza_valid);
        assert_eq!(v.hadolint_agrees, None);
        // A build instruction before any FROM is structurally invalid.
        let bad = DockerDomain::validate("any", "RUN true\nFROM scratch\n");
        assert!(bad.parses);
        assert!(!bad.stanza_valid);
        // Prose is not a Dockerfile.
        let none = DockerDomain::validate("any", "just some text\n");
        assert!(!none.parses);
    }

    #[test]
    fn refusals_are_named_and_reasoned() {
        let r = refuses();
        assert_eq!(r.len(), 4);
        assert!(r
            .iter()
            .any(|(id, _)| *id == "consolidate-consecutive-runs"));
        assert!(r
            .iter()
            .all(|(_, why)| why.contains("REFUSED") || why.contains("never rewritten")));
    }
}

//! Domain modules — pluggable healing domains behind the `HealDomain` trait.
//!
//! Each domain is gated behind its own Cargo feature:
//! - `clippy_lints` (default) — `clippy::*` lint healing (the original domain)
//! - `rust_perf` (default since 2026-08-13, Phase 2) — `rust-optimize` perf
//!   healing (promoted after Bench 013 GOAT G1+G5 PASS)
//! - `kernel_opt` (opt-in, Phase 3) — GPU kernel optimization healing
//! - `docker` (default since 2026-08-28, Plan 030) — Dockerfile bounded-fix
//!   healing + retrieval; the only domain with no `syn` (a hand-written
//!   stanza scanner) and the only one whose oracle is an external binary
//!   (hadolint, when on PATH)
//! - `rustc_errors` (default since 2026-08-29, Plan 019) — rustc
//!   compile-error repair keyed by error code: the E0597 + borrowck
//!   (E0502/E0499/E0505) playbook families behind the `--fix-compile`
//!   driver, plus the premise probe and the `cargo fix` floor that bound
//!   the territory. Driver-shaped: inert until invoked.
//! - `dist` (default since 2026-09-13, Issue 085) — shipped-artifact
//!   build-config advice: release-profile size knobs + the workspace-member
//!   profile trap, over name-matched `Cargo.toml` manifests. Suggest-only in
//!   Phase 1; the second no-`syn` scanner after docker.
//! - `shader_perf` (opt-in, Plan 134 / Proposal 014) — WGSL render-pipeline
//!   (`@vertex`/`@fragment`) bounded-fix healing: loop boundedness (the TDR
//!   class), sample budgets, branch-vs-sample divergence, derivative
//!   safety. `@compute` stays `kernel_opt` (entry-point routing split). The
//!   third no-`syn` scanner; consumers: the upcoming shader-toy editor's
//!   submit/save gate + riir-dapps Proposal 002's Stage-2 mint gate.

// Issue 112: the scratch-probe identity witness, shared by every domain
// whose oracle spawns cargo on a generated crate. Gated on the union of
// those three features rather than living inside one of them — a copy per
// site is a second thing to get wrong, which is the shape Issue 110 was.
#[cfg(any(
    feature = "clippy_verify",
    feature = "rust_perf_verify",
    feature = "rust_perf_bench"
))]
pub mod probe_witness;

// Issue 222 T1 — the batch-intake diffusion-diversity readout
// (REPORT-ONLY; katgpt-rs Plan 634 T1.4's Div_P^τ over the rule-embedding
// kNN graph per mined batch). Never in the release set; feature-off
// byte-identical (the whole module is cfg-gated).
#[cfg(feature = "batch_diversity")]
pub mod batch_diversity;

#[cfg(feature = "clippy_lints")]
pub mod clippy_lints;

#[cfg(feature = "rust_perf")]
pub mod rust_perf;

#[cfg(feature = "kernel_opt")]
pub mod kernel_opt;

#[cfg(feature = "rustc_errors")]
pub mod rustc_errors;

#[cfg(feature = "sec")]
pub mod sec;

#[cfg(feature = "docker")]
pub mod docker;

#[cfg(feature = "dist")]
pub mod dist;

#[cfg(feature = "shader_perf")]
pub mod shader_perf;

#[cfg(feature = "ane")]
pub mod ane;

// ── The domain-wiring registry (plan 329 P3 / issue 197) ───────────────────
//
// ONE static table wiring the three vocabularies a domain ships under: the
// `src/domains/<dir>/` directory name, the cargo feature that compiles it,
// and the two roll-up keys — the draft router's `DomainId` label (the
// routing vocabulary) and the demonstrability canonical domain (the
// `RuleDomains` registry spelling, see `crate::demonstrability::T3_DOMAINS`).
// A new domain directory MUST land in this table (or the exemption list
// below) in the same commit — `domain_registry_tests` at the bottom of this
// file walks the real tree and reds by name when something is missing.

/// One wired domain: the row is data, the registry gates are the enforcement.
pub(crate) struct DomainRow {
    /// The `src/domains/<dir>/` directory name.
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub dir: &'static str,
    /// The cargo feature that compiles it.
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub feature: &'static str,
    /// The draft router's `DomainId::label()` roll-up label (`None`: not a
    /// routed domain — the `--fix-compile` driver shape, inert until
    /// invoked).
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub label: Option<&'static str>,
    /// The demonstrability/registry canonical key (`None`: not a rule
    /// corpus).
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub canonical: Option<&'static str>,
}

/// The six RULE domains (the scan-only `dist`/`docker` live in
/// [`DOMAIN_DIR_EXEMPTIONS`]).
#[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
pub(crate) const DOMAIN_REGISTRY: &[DomainRow] = &[
    DomainRow {
        dir: "clippy_lints",
        feature: "clippy_lints",
        label: Some("clippy"),
        canonical: Some("clippy"),
    },
    DomainRow {
        dir: "rust_perf",
        feature: "rust_perf",
        label: Some("perf"),
        canonical: Some("rust_perf"),
    },
    DomainRow {
        dir: "kernel_opt",
        feature: "kernel_opt",
        label: Some("kernel"),
        canonical: Some("kernel_opt"),
    },
    DomainRow {
        dir: "rustc_errors",
        feature: "rustc_errors",
        label: None, // NOT a DomainId — the `--fix-compile` driver domain
        canonical: Some("rustc_errors"),
    },
    DomainRow {
        dir: "sec",
        feature: "sec",
        label: Some("sec"),
        canonical: Some("sec"),
    },
    DomainRow {
        dir: "shader_perf",
        feature: "shader_perf",
        label: Some("shader"),
        canonical: Some("shader_perf"),
    },
    DomainRow {
        dir: "ane",
        feature: "ane",
        label: Some("ane"),
        canonical: Some("ane"),
    },
];

/// One scan-only domain directory: no rule corpus, nothing to pin — present
/// on disk and routed, but outside [`DOMAIN_REGISTRY`] by charter.
pub(crate) struct DomainDirExemption {
    /// The `src/domains/<dir>/` directory name.
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub dir: &'static str,
    /// WHY the domain is exempt (a charter citation, never "fixes the gate").
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub reason: &'static str,
    /// The `DomainId::label()` the domain rolls up under (also its
    /// demonstrability canonical key — the scan-only domains attribute under
    /// their own name).
    #[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
    pub label: Option<&'static str>,
}

/// THE RATCHET: this list takes DELETIONS ONLY — a scan-only domain that
/// grows a rule corpus graduates into [`DOMAIN_REGISTRY`] and its row is
/// deleted in the same commit. Never add an entry to dodge a red (the
/// `vet`/`fmt_pending` drain-list shape).
#[cfg_attr(not(test), allow(dead_code))] // read by the cfg(test) registry gates
pub(crate) const DOMAIN_DIR_EXEMPTIONS: &[DomainDirExemption] = &[
    DomainDirExemption {
        dir: "dist",
        reason: "scan-only domain (Issue 085) — no rule corpus, nothing to pin",
        label: Some("dist"),
    },
    DomainDirExemption {
        dir: "docker",
        reason: "scan-only domain (Plan 030) — no rule corpus, nothing to pin",
        label: Some("docker"),
    },
];

#[cfg(test)]
mod domain_registry_tests {
    use super::{DOMAIN_DIR_EXEMPTIONS, DOMAIN_REGISTRY};

    /// The pure classifier behind the filesystem gate (also the G1 canary's
    /// arm): directory names that are neither registered nor exempt.
    fn unregistered_dirs<'a>(dirs: &[&'a str]) -> Vec<&'a str> {
        dirs.iter()
            .copied()
            .filter(|d| {
                !DOMAIN_REGISTRY.iter().any(|r| r.dir == *d)
                    && !DOMAIN_DIR_EXEMPTIONS.iter().any(|x| x.dir == *d)
            })
            .collect()
    }

    /// The filesystem gate: every directory under `src/domains/` that is a
    /// real domain (a directory carrying a `mod.rs`) is registered or
    /// exempt. The predicate excludes FILES — `probe_witness.rs` is a file
    /// (the shared scratch-probe witness, Issue 112), not a domain, so the
    /// walk never sees it; a stray subdirectory without a `mod.rs` cannot
    /// compile as a domain either.
    #[test]
    fn every_domain_directory_is_registered_or_exempt() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("domains");
        let mut found: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&root).expect("src/domains exists in this repo") {
            let entry = entry.expect("read_dir entry");
            let is_dir = entry.file_type().expect("entry file type").is_dir();
            if !is_dir || !entry.path().join("mod.rs").exists() {
                continue;
            }
            found.push(entry.file_name().to_string_lossy().into_owned());
        }
        found.sort_unstable();
        let names: Vec<&str> = found.iter().map(String::as_str).collect();
        let offenders = unregistered_dirs(&names);
        assert!(
            offenders.is_empty(),
            "unregistered domain directory(ies) under src/domains: {offenders:?} — add a \
             DomainRow to DOMAIN_REGISTRY (rule domains) or a reasoned row to \
             DOMAIN_DIR_EXEMPTIONS (scan-only), never a silent dir"
        );
        // Exemption hygiene (reads `reason`): a row without a stated reason
        // is exactly the dodge the ratchet forbids — every exemption must
        // cite its charter.
        let reasonless: Vec<&str> = DOMAIN_DIR_EXEMPTIONS
            .iter()
            .filter(|x| x.reason.trim().is_empty())
            .map(|x| x.dir)
            .collect();
        assert!(
            reasonless.is_empty(),
            "exemption row(s) with no stated reason: {reasonless:?} — a dodge without a \
             charter citation is never admitted"
        );
    }

    /// The manifest wiring (reads `feature`): every registry row names a
    /// cargo feature the package actually declares — a row pointing at a
    /// feature the manifest does not carry is drift, caught here at the
    /// registry instead of as a silently-dead module decl.
    #[test]
    fn registry_feature_names_exist_in_the_manifest() {
        let manifest = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        )
        .expect("Cargo.toml exists in this repo");
        let undeclared: Vec<&str> = DOMAIN_REGISTRY
            .iter()
            .map(|r| r.feature)
            .filter(|f| !manifest.lines().any(|l| l.starts_with(*f)))
            .collect();
        assert!(
            undeclared.is_empty(),
            "registry row(s) naming a feature the manifest does not declare: {undeclared:?}"
        );
    }

    /// The roll-up vocabulary: every `DomainId::ALL` label has a home — a
    /// registry row's label or an exemption's label. Gated `latent_retrieval`
    /// because the `draft` module (the `DomainId` source) is — `score_bench`
    /// implies `latent_retrieval`, so the bench posture runs this arm.
    #[cfg(feature = "latent_retrieval")]
    #[test]
    fn rollup_labels_are_registered() {
        let known: std::collections::BTreeSet<&str> = DOMAIN_REGISTRY
            .iter()
            .filter_map(|r| r.label)
            .chain(DOMAIN_DIR_EXEMPTIONS.iter().filter_map(|x| x.label))
            .collect();
        let strays: Vec<&str> = crate::draft::domain_router::DomainId::ALL
            .iter()
            .map(|d| d.label())
            .filter(|l| !known.contains(*l))
            .collect();
        assert!(
            strays.is_empty(),
            "DomainId labels with no registry/exemption row: {strays:?}"
        );
    }

    /// The demonstrability vocabulary: every canonical key the score-bench
    /// column can carry resolves — to a registry row's canonical, or to an
    /// exempt domain whose label IS the key (dist/docker attribute under
    /// their own name; there is no registry row to carry a canonical). For
    /// labeled registry rows the two vocabularies must also AGREE: the
    /// canonical is exactly `canonical_domain(label)` (reads `label` in
    /// every posture — the ungated demonstrability mapping, never a
    /// re-derived copy).
    #[test]
    fn score_bench_domain_keys_are_registered() {
        // The source of truth is `crate::demonstrability::T3_DOMAINS` (pub,
        // ungated) — consumed directly, never replicated.
        let known: std::collections::BTreeSet<&str> = DOMAIN_REGISTRY
            .iter()
            .filter_map(|r| r.canonical)
            .chain(DOMAIN_DIR_EXEMPTIONS.iter().filter_map(|x| x.label))
            .collect();
        let strays: Vec<&str> = crate::demonstrability::T3_DOMAINS
            .iter()
            .copied()
            .filter(|k| !known.contains(*k))
            .collect();
        assert!(
            strays.is_empty(),
            "demonstrability canonical keys with no registry canonical / exemption label: \
             {strays:?}"
        );
        let disagreeing: Vec<&str> = DOMAIN_REGISTRY
            .iter()
            .filter_map(|r| {
                r.label
                    .map(crate::demonstrability::canonical_domain)
                    .filter(|c| Some(*c) != r.canonical)
                    .map(|_| r.dir)
            })
            .collect();
        assert!(
            disagreeing.is_empty(),
            "registry row(s) whose canonical disagrees with canonical_domain(label): \
             {disagreeing:?}"
        );
    }

    /// G1 canary: the classifier REDS an unregistered directory BY NAME.
    /// No filesystem writes — never create dirs inside `src/`; a synthetic
    /// list is the fixture.
    #[test]
    fn unregistered_directory_reds_by_name() {
        let red = unregistered_dirs(&["g1_fixture_domain", "clippy_lints"]);
        assert_eq!(
            red,
            vec!["g1_fixture_domain"],
            "the classifier names the unknown dir and only it"
        );
        assert!(
            unregistered_dirs(&["clippy_lints"]).is_empty(),
            "a registered dir never reds"
        );
    }
}

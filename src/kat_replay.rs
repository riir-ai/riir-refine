//! The replay-endorsement ENGINE (riir-refine Proposal 015 Phase 2 — the
//! `cargo refine replay` command's judgment half; the byte movers live in
//! `riir_kat::kat_replay_client`, the server in riir-dapps Plan 029).
//!
//! # What this module owns
//!
//! 1. **The toolchain fingerprint** (T2.2) — `blake3(rustc -vV stdout)`.
//!    The lease pins the fingerprint every verdict must be computed under;
//!    a mismatch is a LOCAL refusal (skip + report, never submit — the
//!    server refuses skewed verdicts unscored, so submitting one is pure
//!    waste). P015 caveat 2 stands: the fingerprint pins the toolchain
//!    BUILD (version + commit + host), not bit-identical behavior across
//!    platforms — cross-platform disagreement is an expected split, which
//!    is why the pin is per-deployment and the operator sets it to the
//!    fleet majority.
//!
//!    The probe runs with the process's INHERITED cwd on purpose: rustup
//!    resolves the toolchain by the current directory chain, and the
//!    oracle's `cargo clippy` inherits the same cwd — fingerprinting any
//!    other toolchain than the one the replay will actually run under
//!    would be a lie the skew check cannot catch.
//!
//! 2. **The pinned single-item rate rule** (the determinism law) — two
//!    honest verifiers must agree on the FULL 18-byte vector (the milli
//!    rates ride the commitment), so the rule is pinned HERE and every
//!    client run computes the same fields from the same oracle verdicts:
//!
//!    - `compile_ok`      = the FIX compiles (the oracle's `compiles`).
//!    - `oracle_resolved` = the shape compiles AND the fix compiles AND
//!      `lints(shape) \ lints(fix) ≠ ∅` — the fix STOPPED at least one
//!      lint the pre-fix span fired. The wire carries no `lint_key`
//!      (P015's stripped-vector design), so the observable evidence of a
//!      heal is the REMOVED set, not a named target: ambient lints that
//!      fire on both arms (the oracle runs pedantic+nursery —
//!      `must_use_candidate` fires on most `pub fn`s) cancel in the diff
//!      BY CONSTRUCTION, deterministically under the pinned toolchain.
//!      A clean shape + clean fix reads UNRESOLVED (no evidence of any
//!      heal — the honest answer for a vacuous row, never a pass).
//!    - `heal_rate_milli` = 1000 if `oracle_resolved` else 0.
//!    - `created_rate_milli` = 1000 if `lints(fix) \ lints(shape) ≠ ∅`
//!      else 0 (born-closure lints — the score-bench "created" concept,
//!      the gen-8 shaped-verdict half-win: a heal that clears its target
//!      AND births new diagnostics prices as healed + created).
//!
//!    The rates are advisory server-side (P015 caveat 6: the hard checks
//!    are `compile ∧ oracle`) but they are PART OF THE COMMITMENT, so the
//!    pin is load-bearing for agreement — never change this formula
//!    without a wire-version bump on the lease domain. The designed
//!    refinement (a v2 lease carrying `lint_key` so the oracle can name
//!    the target) is a Phase-3 reopen, priced by the econ arm.
//!
//! 3. **Step 0, the canary hash check** — `blake3(fix) == fix_hash` else
//!    the item is a canary and the verdict is `Tamper` (P015's `TrueBit`
//!    adaptation: a canary's pinned hash deliberately mismatches its fix
//!    text; an honest verifier detects it WITHOUT executing anything).
//!
//! # What is deliberately NOT here
//!
//! No transport, no account, no consent — the command orchestration lives
//! in the `cargo-refine` bin (`run_kat_replay`), which gates on
//! `kat_service` + `kat_transport` + `clippy_verify`. This module needs
//! only the oracle (`clippy_verify`) + the protocol wire types (the
//! riir-kat protocol tier, always compiled).
//!
//! # Unmeasurable is not a verdict
//!
//! A verdict is a CLAIM about a replay. When the oracle did not run
//! (cargo missing, temp-crate setup failure) the honest answer is to SKIP
//! the item and say why — never to submit a default-shaped vector that
//! would read as agreement (or disagreement) it did not measure.

use riir_kat::kat_protocol_replay::{ReplayLeaseItemWire, ReplayOutcomeWire};

use crate::draft::clippy_oracle::{ClippyOracle, OracleVerdict};

/// The toolchain fingerprint: `blake3(rustc -vV stdout)` — pins the
/// toolchain BUILD (version + commit hash + host triple). Operator-
/// derivable with `rustc -vV | blake3` on the deployment's fleet box.
///
/// # Errors
///
/// `rustc` missing from PATH, or the probe failed to run — worded for the
/// one `replay:` status line (the command refuses to continue: a verdict
/// under an unpinnable toolchain cannot honestly claim the lease's pin).
pub fn toolchain_fingerprint() -> Result<[u8; 32], String> {
    let out = std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|e| {
            format!("rustc is not reachable on PATH ({e}) — cannot fingerprint the toolchain")
        })?;
    if !out.status.success() {
        return Err("rustc -vV failed — cannot fingerprint the toolchain".to_string());
    }
    Ok(blake3::hash(&out.stdout).into())
}

/// The one-line human rendering of the local toolchain (the first
/// `rustc -vV` line, e.g. `rustc 1.98.1 (… )`) for the skew report.
#[must_use]
pub fn toolchain_display() -> String {
    std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_string))
        .unwrap_or_else(|| "rustc (unknown version)".to_string())
}

/// The pinned single-item judgment — the four wire fields plus the lint
/// sets they came from (the CLI prints those; the wire carries only the
/// four).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayJudgment {
    pub compile_ok: bool,
    pub oracle_resolved: bool,
    pub heal_rate_milli: u64,
    pub created_rate_milli: u64,
    /// What fired on the pre-fix span (sorted; the oracle's `BTreeSet`).
    pub shape_lints: Vec<String>,
    /// What fired on the fixed span.
    pub fix_lints: Vec<String>,
    /// `lints(fix) \ lints(shape)` — the born-closure set.
    pub created_lints: Vec<String>,
}

impl ReplayJudgment {
    /// The wire outcome (what gets signed + submitted).
    #[must_use]
    pub fn to_outcome(&self) -> ReplayOutcomeWire {
        ReplayOutcomeWire::Vector {
            compile_ok: self.compile_ok,
            oracle_resolved: self.oracle_resolved,
            heal_rate_milli: self.heal_rate_milli,
            created_rate_milli: self.created_rate_milli,
        }
    }
}

/// The pinned rate rule (see the module doc — the determinism law). Pure
/// over the two oracle verdicts so it is unit-testable without cargo.
/// The pinned single-item rate rule (see the module doc — the determinism
/// law). Pure over the two oracle verdicts so it is unit-testable without
/// cargo. The judgment axes:
/// - `removed` = `lints(shape) \ lints(fix)` — the observable evidence of
///   a heal (the wire carries no `lint_key`; ambient lints firing on both
///   arms cancel in this diff by construction);
/// - `created` = `lints(fix) \ lints(shape)` — the born-closure set.
#[must_use]
pub fn verdict_from_lints(shape: &OracleVerdict, fix: &OracleVerdict) -> ReplayJudgment {
    let shape_lints: Vec<String> = shape.lints.iter().cloned().collect();
    let fix_lints: Vec<String> = fix.lints.iter().cloned().collect();
    let removed_lints: Vec<String> = shape.lints.difference(&fix.lints).cloned().collect();
    let created_lints: Vec<String> = fix.lints.difference(&shape.lints).cloned().collect();
    // The pinned rule, field by field:
    let compile_ok = fix.compiles;
    let oracle_resolved = shape.compiles && fix.compiles && !removed_lints.is_empty();
    ReplayJudgment {
        compile_ok,
        oracle_resolved,
        heal_rate_milli: u64::from(oracle_resolved) * 1000,
        created_rate_milli: u64::from(!created_lints.is_empty()) * 1000,
        shape_lints,
        fix_lints,
        created_lints,
    }
}

/// One item's replay verdict — a claim, a canary report, or an honest
/// "could not measure" (which the caller turns into a SKIP, never a
/// submission).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayVerdict {
    /// Step 0 caught the hash mismatch — the item is a canary.
    Tamper,
    /// The oracle measured both spans; the vector is the pinned rule over
    /// the two verdicts.
    Judgment(ReplayJudgment),
    /// The oracle did not run (cargo missing / crate setup failed) — the
    /// item must be SKIPPED, never submitted.
    Unmeasurable(String),
}

/// Decode one leased item's pinned hashes and judge it: step-0 hash check,
/// then ONE batched oracle run over `[shape, fix]` (two modules of one
/// scratch crate — a single clippy spawn per item; the oracle's own
/// honesty contract applies: any hard error falls back to per-snippet
/// runs, and a verdict whose `ran` is false is [`ReplayVerdict::Unmeasurable`]).
///
/// The toolchain check is NOT here — it is per-LEASE (every item in a
/// lease shares the pin), so the command checks it once before the loop.
#[must_use]
pub fn judge_item(oracle: &ClippyOracle, item: &ReplayLeaseItemWire) -> ReplayVerdict {
    // Step 0 — the canary check, before any compile (P015: the service
    // never executes untrusted text; the verifier's rig does, and only
    // after the bytes it was pinned to survive the hash comparison).
    let pinned = crate::kat_replay::unhex32(&item.fix_hash);
    let actual = blake3::hash(item.fix.as_bytes());
    match pinned {
        Some(p) if p == *actual.as_bytes() => {}
        Some(_) => return ReplayVerdict::Tamper,
        // A malformed pinned hash is not a canary verdict — it is an
        // unmeasurable lease (the route hex32-gate makes this unreachable
        // for well-formed services; treat it as skip-and-say-why anyway).
        None => {
            return ReplayVerdict::Unmeasurable(format!(
                "pinned fix_hash {:?} is not 64 hex chars",
                &item.fix_hash[..item.fix_hash.len().min(16)]
            ));
        }
    }
    let verdicts = oracle.lints_for_batch(&[&item.shape, &item.fix]);
    if verdicts.len() != 2 {
        return ReplayVerdict::Unmeasurable("oracle returned no verdicts".to_string());
    }
    let (shape, fix) = (&verdicts[0], &verdicts[1]);
    if !shape.ran || !fix.ran {
        let why = shape
            .error
            .clone()
            .or_else(|| fix.error.clone())
            .unwrap_or_else(|| "oracle did not run".to_string());
        return ReplayVerdict::Unmeasurable(why);
    }
    ReplayVerdict::Judgment(verdict_from_lints(shape, fix))
}

/// The protocol tier's hex decoder, re-exposed for the bin's toolchain
/// comparisons (one decoder everywhere — the `kat_protocol_lease` source
/// of truth).
pub use riir_kat::kat_protocol_lease::unhex32;

/// The 16-char short render of a 32-byte hash for status lines (the
/// span-hash display convention the mining surfaces use).
#[must_use]
pub fn hex32_short(bytes: &[u8; 32]) -> String {
    riir_kat::kat_protocol_lease::hex32(bytes)[..16].to_string()
}

/// Test-only assertion helper (keeps the test bodies on the wire shape).
#[cfg(test)]
impl ReplayJudgment {
    fn to_outcome_matches(&self) -> bool {
        let ReplayOutcomeWire::Vector {
            compile_ok,
            oracle_resolved,
            heal_rate_milli,
            created_rate_milli,
        } = self.to_outcome()
        else {
            return false;
        };
        compile_ok == self.compile_ok
            && oracle_resolved == self.oracle_resolved
            && heal_rate_milli == self.heal_rate_milli
            && created_rate_milli == self.created_rate_milli
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn v(compiles: bool, lints: &[&str]) -> OracleVerdict {
        OracleVerdict {
            ran: true,
            compiles,
            lints: lints
                .iter()
                .map(|s| (*s).to_string())
                .collect::<BTreeSet<_>>(),
            error: None,
        }
    }

    fn item(shape: &str, fix: &str) -> ReplayLeaseItemWire {
        ReplayLeaseItemWire {
            span_hash: "0".repeat(64),
            fix_hash: blake3::hash(fix.as_bytes()).to_hex().to_string(),
            shape: shape.to_string(),
            fix: fix.to_string(),
            corpus_version: 1,
            model_version: 1,
            toolchain_fingerprint: "9".repeat(64),
        }
    }

    // ── the pinned rate rule (the determinism law) ─────────────────────

    #[test]
    fn the_pass_vector_is_the_clean_heal() {
        let j = verdict_from_lints(&v(true, &["needless_return"]), &v(true, &[]));
        assert!(j.compile_ok);
        assert!(j.oracle_resolved);
        assert_eq!(j.heal_rate_milli, 1000);
        assert_eq!(j.created_rate_milli, 0);
        assert!(j.to_outcome_matches());
    }

    #[test]
    fn a_persisting_lint_is_a_fail_vector() {
        let j = verdict_from_lints(
            &v(true, &["needless_return"]),
            &v(true, &["needless_return"]),
        );
        assert!(j.compile_ok);
        assert!(!j.oracle_resolved);
        assert_eq!(j.heal_rate_milli, 0);
        assert_eq!(j.created_rate_milli, 0);
    }

    #[test]
    fn born_closure_lints_price_the_half_win() {
        let j = verdict_from_lints(&v(true, &["manual_map"]), &v(true, &["redundant_closure"]));
        assert!(j.oracle_resolved, "the target is gone");
        assert_eq!(j.heal_rate_milli, 1000);
        assert_eq!(
            j.created_rate_milli, 1000,
            "the created axis rides the commitment"
        );
        assert_eq!(j.created_lints, vec!["redundant_closure".to_string()]);
    }

    #[test]
    fn a_broken_fix_fails_compile_and_oracle() {
        let j = verdict_from_lints(&v(true, &["needless_return"]), &v(false, &[]));
        assert!(!j.compile_ok);
        assert!(!j.oracle_resolved);
        assert_eq!(j.heal_rate_milli, 0);
    }

    #[test]
    fn a_non_compiling_shape_never_resolves() {
        // A redaction-broken shape cannot establish a baseline; the honest
        // vector is a FAIL (never a vacuous pass).
        let j = verdict_from_lints(&v(false, &[]), &v(true, &[]));
        assert!(j.compile_ok);
        assert!(!j.oracle_resolved);
    }

    #[test]
    fn a_clean_shape_provides_no_evidence_of_a_heal() {
        // Nothing fired before, nothing fires after — the removed set is
        // empty, so the honest vector is a FAIL (never a vacuous pass;
        // the module doc's determinism-law entry).
        let j = verdict_from_lints(&v(true, &[]), &v(true, &[]));
        assert!(j.compile_ok);
        assert!(!j.oracle_resolved);
        assert_eq!(j.heal_rate_milli, 0);
    }

    #[test]
    fn ambient_lints_firing_on_both_arms_cancel_in_the_diff() {
        // The measured reason the intersection rule died: pedantic
        // `must_use_candidate` fires on most `pub fn`s — under the oracle's
        // pedantic+nursery flags BOTH arms carry it, and a rule demanding an
        // empty intersection would read nearly every honest heal as
        // unresolved. The diff cancels it by construction.
        let j = verdict_from_lints(
            &v(true, &["must_use_candidate", "uninlined_format_args"]),
            &v(true, &["must_use_candidate"]),
        );
        assert!(j.oracle_resolved, "the removed set is non-empty");
        assert_eq!(j.heal_rate_milli, 1000);
        assert_eq!(j.created_rate_milli, 0);
    }

    #[test]
    fn removing_an_ambient_while_keeping_the_target_still_reads_healed() {
        // The honest limit of a lint_key-less wire (documented, priced for
        // the Phase-3 reopen): the removed set is the evidence, and it
        // cannot name WHICH lint was the target. Canaries + K=2 agreement
        // bound the stamp surface this leaves.
        let j = verdict_from_lints(
            &v(true, &["needless_return", "must_use_candidate"]),
            &v(true, &["needless_return"]),
        );
        assert!(j.oracle_resolved, "removed={:?}", j.shape_lints);
    }

    // ── step 0 + the oracle path ────────────────────────────────────────

    #[test]
    fn step0_catches_the_hash_mismatch_without_executing() {
        // A canary: the pinned hash is NOT blake3(fix). No cargo needed —
        // the catch precedes every compile.
        let mut canary = item("fn a() { 1 }", "fn a() { 1 }");
        canary.fix_hash = "ff".repeat(32);
        let oracle = ClippyOracle::new();
        assert_eq!(judge_item(&oracle, &canary), ReplayVerdict::Tamper);
    }

    #[test]
    fn step0_refuses_a_malformed_pinned_hash_as_unmeasurable() {
        let mut bad = item("fn a() { 1 }", "fn a() { 1 }");
        bad.fix_hash = "zz".into();
        let oracle = ClippyOracle::new();
        assert!(matches!(
            judge_item(&oracle, &bad),
            ReplayVerdict::Unmeasurable(_)
        ));
    }

    /// The real-oracle happy path: one spawn, the pass vector. Spawns real
    /// cargo (the `clippy_verify` build always has it on a dev box).
    #[test]
    fn judge_item_measures_a_real_heal() {
        let oracle = ClippyOracle::new();
        if !oracle.is_available() {
            return; // a cargo-less environment cannot judge — skip, don't fail
        }
        let it = item(
            "pub fn greet(name: &str) -> String { format!(\"{}\", name) }",
            "pub fn greet(name: &str) -> String { format!(\"{name}\") }",
        );
        match judge_item(&oracle, &it) {
            ReplayVerdict::Judgment(j) => {
                assert!(j.compile_ok);
                assert!(j.oracle_resolved, "shape lints: {:?}", j.shape_lints);
                assert_eq!(j.created_rate_milli, 0);
            }
            other => panic!("expected a judgment, got {other:?}"),
        }
    }

    /// The toolchain fingerprint is stable across calls and matches the
    /// operator's `rustc -vV | blake3` derivation.
    #[test]
    fn toolchain_fingerprint_is_deterministic() {
        let a = toolchain_fingerprint().expect("rustc on PATH in a dev environment");
        let b = toolchain_fingerprint().unwrap();
        assert_eq!(a, b);
        // And the operator derivation agrees byte-for-byte.
        let out = std::process::Command::new("rustc")
            .arg("-vV")
            .output()
            .unwrap();
        assert_eq!(blake3::hash(&out.stdout).as_bytes(), &a);
    }
}

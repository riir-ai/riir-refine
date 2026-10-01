//! KAT T2.4 (Proposal 005 Phase 2 / Plan 042) — the score-bench promotion
//! gate + `EvidenceTier` wiring for network-learned rules.
//!
//! Corpus-candidate rules arriving from the network (the service plane's
//! proven-mining channel, §Mining #4) must pass the FROZEN score-bench
//! BEFORE entering the live corpus. This module is that gate:
//!
//! 1. **Schema** — a versioned NDJSON candidates file (one
//!    [`CandidateRow`] per line) mirroring the lease `PatternEntry` triple
//!    `(rule_id, buggy_code, fixed_code)` plus versioning + provenance.
//! 2. **Trial** — the frozen fixtures run through
//!    [`crate::score_bench::run_bench_with`] with the candidates as a
//!    textual lane (base + candidates), compared against the no-candidate
//!    baseline on CANONICAL INTEGER totals (`resolved` / `created` — the
//!    denominators are identical by construction, so the comparison is
//!    bit-deterministic, never float).
//! 3. **Verdict** — not-worse on heal AND not-worse on created + every
//!    candidate's `fixed_code` parses (fragment parse) → **Promote** (the
//!    candidate enters the LOCAL overlay as
//!    [`EvidenceTier::Certified`]); any regression or unsafe candidate →
//!    **Reject** (audit row, never enters).
//! 4. **Withdrawn (absorbing)** — a promoted rule whose POST-HOC bench
//!    regresses (corpus/fixtures/toolchain moved under it) transitions to
//!    [`EvidenceTier::Withdrawn`], is removed from the overlay, and can
//!    NEVER return via promotion. This is deliberately STRICTER than the
//!    trajectory tier's re-certification exit (Issue 021): a network rule
//!    already had its quorum chance and still regressed — no second
//!    chance is the §Mining #4 poisoning defense.
//!
//! ## Trial isolation
//!
//! The gate never touches `.benchmarks/score_history.jsonl` — trial runs
//! are not score-history runs (pinned by test). Side effects are exactly:
//! the registry file + the audit NDJSON, both only via explicit
//! [`PromotionRegistry::save`] / [`append_audit`] calls by the caller.
//!
//! ## The network seam (documented, NOT built here)
//!
//! The lease manifest can carry a CANDIDATES segment (the same
//! content-addressed segment scheme the corpus uses); a delivered segment
//! is written to a local candidates file and this gate runs BEFORE the
//! rules reach the overlay. Building that delivery is service-side
//! (T1.5/T2.x) — this module is the consumer half.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};
use crate::draft::clippy_oracle::ClippyOracle;
use crate::memory::EvidenceTier;
use crate::score_bench::{run_bench, run_bench_with, BenchFixture, BenchScore};

/// The candidates-file schema version this build defines (strict match).
pub const CANDIDATE_SCHEMA_VERSION: u32 = 1;

/// The registry file schema version this build defines.
pub const REGISTRY_VERSION: u32 = 1;

/// Registry file name under the `.refine` root.
pub const REGISTRY_FILE: &str = "kat_promotion.json";

/// Append-only audit NDJSON under the `.refine` root (the `score_history`
/// precedent — committed history of every gate decision).
pub const AUDIT_FILE: &str = "kat_promotion_audit.jsonl";

/// Domain tag for clippy-lane candidates (the lease pack domain spelling).
pub const DOMAIN_CLIPPY: &str = "clippy";

/// Domain tag for perf-lane candidates.
pub const DOMAIN_PERF: &str = "perf";

/// One retrieval triple — the same shape the lease/overlay machinery
/// carries (`rule_id`, `buggy`, `fixed`).
pub type Triple = (String, String, String);

// ── §1 the candidate schema ────────────────────────────────────────────────

/// One network-delivered corpus candidate (one NDJSON row of the
/// candidates file). Mirrors the lease `PatternEntry` triple + versioning
/// + provenance — the exact shape the candidate channel will deliver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRow {
    pub schema_version: u32,
    pub rule_id: String,
    /// `clippy` | `perf` (the bench's lanes; anything else is a loud
    /// schema error — kernel candidates need a kernel bench first).
    pub domain: String,
    /// The verbatim match key (textual containment — the L1 verbatim tier).
    pub buggy_code: String,
    /// The verbatim replacement (emitted code — MUST parse as a fragment).
    pub fixed_code: String,
    /// Free-form origin note (`mining-batch-7/quorum-3of3`, …).
    #[serde(default)]
    pub provenance: String,
}

impl CandidateRow {
    /// Canonical bytes for the content hash — the SEMANTIC triple
    /// (domain, buggy, fixed), `\x01`-separated (the corpus commitment
    /// discipline). Deliberately excludes `rule_id`: same content under a
    /// new id is the same rule (hash catches it); same id with new content
    /// is caught by the id blacklist.
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(self.domain.as_bytes());
        out.push(0x01);
        out.extend_from_slice(self.buggy_code.as_bytes());
        out.push(0x01);
        out.extend_from_slice(self.fixed_code.as_bytes());
        out
    }

    /// Content-addressed identity — the absorbing blacklist's key. A
    /// withdrawn rule cannot return under a new `rule_id` (hash catches
    /// same-content resubmission) nor under the same id with new content
    /// (id catches that).
    #[must_use]
    pub fn content_hash_hex(&self) -> String {
        blake3::hash(&self.canonical_bytes()).to_hex().to_string()
    }

    /// The trial-lane pair `(buggy, fixed)`.
    #[must_use]
    pub fn lane(&self) -> (&str, &str) {
        (&self.buggy_code, &self.fixed_code)
    }
}

/// Does `code` parse as a standalone fragment — either a whole file (item
/// fragments: fns, structs, impls) or a brace-wrapped statement/expression
/// sequence (stmt fragments: `return x;`, `let y = …;`, bare expressions)?
/// The bench wrap (`#![allow(...)]` pragmas) is NOT applied — this is the
/// raw validity check for text that will be spliced into real code.
#[must_use]
pub fn parses_as_fragment(code: &str) -> bool {
    if syn::parse_file(code).is_ok() {
        return true;
    }
    let wrapped = format!("{{ {code} }}");
    syn::parse_str::<syn::Block>(&wrapped).is_ok()
}

/// Parse + validate a candidates file (NDJSON, one [`CandidateRow`] per
/// line, empty lines skipped).
///
/// Validation is the FIRST poisoning defense and runs before any bench:
/// strict schema version, non-empty unique `rule_id`, known domain,
/// non-empty `buggy_code` that differs from `fixed_code`, and a
/// `fixed_code` that parses as a fragment (it gets spliced into code —
/// invalid Rust never enters trial).
///
/// # Errors
///
/// A `String` naming the first offending line + reason.
pub fn parse_candidates(bytes: &[u8]) -> Result<Vec<CandidateRow>, String> {
    let text =
        std::str::from_utf8(bytes).map_err(|e| format!("candidates file is not UTF-8: {e}"))?;
    let mut rows = Vec::new();
    let mut seen_ids: BTreeSet<String> = BTreeSet::new();
    let mut seen_content: BTreeSet<String> = BTreeSet::new();
    for (i, line) in text.split('\n').enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row: CandidateRow = serde_json::from_str(line)
            .map_err(|e| format!("candidates line {}: not a valid CandidateRow: {e}", i + 1))?;
        let ln = i + 1;
        if row.schema_version != CANDIDATE_SCHEMA_VERSION {
            return Err(format!(
                "candidates line {ln}: schema version {} (this client defines {CANDIDATE_SCHEMA_VERSION})",
                row.schema_version
            ));
        }
        if row.rule_id.trim().is_empty() {
            return Err(format!("candidates line {ln}: empty rule_id"));
        }
        if !seen_ids.insert(row.rule_id.clone()) {
            return Err(format!(
                "candidates line {ln}: duplicate rule_id {:?}",
                row.rule_id
            ));
        }
        if row.domain != DOMAIN_CLIPPY && row.domain != DOMAIN_PERF {
            return Err(format!(
                "candidates line {ln}: unknown domain {:?} (want {DOMAIN_CLIPPY}|{DOMAIN_PERF} — kernel candidates need a kernel bench)",
                row.domain
            ));
        }
        if row.buggy_code.is_empty() {
            return Err(format!(
                "candidates line {ln}: empty buggy_code (the match key)"
            ));
        }
        if row.buggy_code == row.fixed_code {
            return Err(format!(
                "candidates line {ln}: buggy_code == fixed_code (a no-op rule)",
            ));
        }
        if !parses_as_fragment(&row.fixed_code) {
            return Err(format!(
                "candidates line {ln}: fixed_code does not parse as a fragment (ParseUnsafe — never trialed)",
            ));
        }
        // Exact-duplicate content under a different id is schema noise (the
        // absorbing blacklist keys content — a resubmission of the SAME rule
        // should carry ONE id).
        if !seen_content.insert(row.content_hash_hex()) {
            return Err(format!(
                "candidates line {ln}: duplicate content (same domain/buggy/fixed as an earlier row)",
            ));
        }
        rows.push(row);
    }
    Ok(rows)
}

// ── §2 the registry (EvidenceTier wiring) ─────────────────────────────────

/// Why a promotion attempt was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromoteError {
    /// Absorbing blacklist: this content (hash) or this `rule_id` was
    /// Withdrawn — it can NEVER return via promotion.
    WithdrawnAbsorbing,
    /// A different rule already lives at this `rule_id`.
    DuplicateRuleId,
}

/// One promoted (Certified) rule in the local overlay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotedEntry {
    pub row: CandidateRow,
    /// [`EvidenceTier::Certified`] at promote; transitions to
    /// `Withdrawn` (absorbing) on post-hoc regression.
    pub tier: EvidenceTier,
    /// Monotone sequence — the audit's promotion ordinal.
    pub promoted_seq: u64,
}

/// The local promotion state: the live overlay + the ABSORBING withdrawn
/// blacklists. Persisted at `<heal-root>/kat_promotion.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionRegistry {
    pub registry_version: u32,
    /// Live overlay (Certified only — withdrawal removes entries).
    pub promoted: BTreeMap<String, PromotedEntry>,
    /// Withdrawn CONTENT hashes (hex) — same-content resubmission bar.
    pub withdrawn_hashes: BTreeSet<String>,
    /// Withdrawn `rule_id`s — same-id resubmission bar.
    pub withdrawn_ids: BTreeSet<String>,
    /// Next promotion ordinal.
    pub next_seq: u64,
}

impl Default for PromotionRegistry {
    fn default() -> Self {
        Self {
            registry_version: REGISTRY_VERSION,
            promoted: BTreeMap::new(),
            withdrawn_hashes: BTreeSet::new(),
            withdrawn_ids: BTreeSet::new(),
            next_seq: 1,
        }
    }
}

impl PromotionRegistry {
    /// Is this candidate's content or id on the absorbing blacklist?
    #[must_use]
    pub fn is_withdrawn(&self, row: &CandidateRow) -> bool {
        self.withdrawn_hashes.contains(&row.content_hash_hex())
            || self.withdrawn_ids.contains(&row.rule_id)
    }

    /// Promote a gate-passing candidate: Certified + live in the overlay.
    ///
    /// # Errors
    ///
    /// [`PromoteError::WithdrawnAbsorbing`] — the poisoning defense: a
    /// withdrawn rule NEVER returns via promotion (no re-certification
    /// exit here, unlike the trajectory tier — see the module doc).
    /// [`PromoteError::DuplicateRuleId`] — a different rule holds the id.
    pub fn promote(&mut self, row: CandidateRow) -> Result<(), PromoteError> {
        if self.is_withdrawn(&row) {
            return Err(PromoteError::WithdrawnAbsorbing);
        }
        if let Some(existing) = self.promoted.get(&row.rule_id) {
            if existing.row == row {
                // Idempotent re-promote of the SAME row: already live.
                return Ok(());
            }
            return Err(PromoteError::DuplicateRuleId);
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        self.promoted.insert(
            row.rule_id.clone(),
            PromotedEntry {
                row,
                tier: EvidenceTier::Certified,
                promoted_seq: seq,
            },
        );
        Ok(())
    }

    /// Withdraw `rule_id`: tier → Withdrawn, REMOVED from the overlay, and
    /// its content hash + id land on the absorbing blacklists. Returns
    /// `false` when the id was never promoted.
    pub fn withdraw(&mut self, rule_id: &str) -> bool {
        let Some(entry) = self.promoted.remove(rule_id) else {
            return false;
        };
        let _ = entry.tier; // (the tier travels with history only — the
                            // blacklists below are what enforce absorbing)
        self.withdrawn_hashes.insert(entry.row.content_hash_hex());
        self.withdrawn_ids.insert(rule_id.to_string());
        true
    }

    /// The live overlay as retrieval triples (`rule_id` order — `BTreeMap`).
    #[must_use]
    pub fn overlay_triples(&self) -> Vec<Triple> {
        self.promoted
            .values()
            .map(|e| {
                (
                    e.row.rule_id.clone(),
                    e.row.buggy_code.clone(),
                    e.row.fixed_code.clone(),
                )
            })
            .collect()
    }

    /// The live overlay as trial-lane pairs (for the post-hoc sweep).
    #[must_use]
    pub fn lanes(&self) -> Vec<(String, String)> {
        self.promoted
            .values()
            .map(|e| (e.row.buggy_code.clone(), e.row.fixed_code.clone()))
            .collect()
    }

    /// Load from disk. Missing file → fresh registry. CORRUPT file →
    /// `Err` — the gate REFUSES to operate on an unreadable blacklist
    /// (promoting around a broken blacklist is the poisoning vector).
    ///
    /// # Errors
    ///
    /// `String` when the file exists but is not a readable v1 registry.
    pub fn load(path: &Path) -> Result<Self, String> {
        let Ok(bytes) = std::fs::read(path) else {
            return Ok(Self::default());
        };
        let reg: Self = serde_json::from_slice(&bytes)
            .map_err(|e| format!("promotion registry unreadable ({}): {e}", path.display()))?;
        if reg.registry_version != REGISTRY_VERSION {
            return Err(format!(
                "promotion registry carries version {} (this client defines {REGISTRY_VERSION}): {}",
                reg.registry_version,
                path.display()
            ));
        }
        Ok(reg)
    }

    /// Persist (canonical via serde on `BTree` collections — deterministic).
    ///
    /// # Errors
    ///
    /// Propagates filesystem errors.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?,
        )
    }
}

// ── §3 the verdict (canonical integer comparison) ─────────────────────────

/// Integer totals from a bench run — the comparison basis. Derived from
/// the per-fixture outcomes (deterministic by construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Totals {
    /// Σ vocabulary lints resolved (the heal axis, higher better).
    pub resolved: usize,
    /// Σ created lints, any group (the false-positive axis, lower better).
    pub created: usize,
    /// Σ vocabulary lints before (the shared denominator).
    pub vocab_before: usize,
    /// Fixtures the oracle scored (0 ⇒ the gate cannot judge — fail closed).
    pub n_ran: usize,
}

/// Derive [`Totals`] from a bench score.
#[must_use]
pub fn totals(score: &BenchScore) -> Totals {
    let mut t = Totals {
        n_ran: score.n_ran,
        ..Totals::default()
    };
    for o in &score.outcomes {
        t.resolved += o.resolved();
        t.created += o.created.len();
        t.vocab_before += o.vocab_before.len();
    }
    t
}

/// Why a trial was rejected (the pre-registered regression bar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// `fixed_code` does not parse standalone — rejected at the schema,
    /// never trialed.
    ParseUnsafe,
    /// The candidate's content/id is on the absorbing blacklist.
    WithdrawnAbsorbing,
    /// Another candidate in the same file carries the SAME `buggy_code`
    /// match key with a DIFFERENT `fixed_code` — only the first can fire,
    /// the others would ride the batch trial undetected (shadowing).
    /// Ambiguous rules are rejected wholesale, deterministically, before
    /// any bench.
    ConflictingMatchKey,
    /// Trial resolved FEWER vocabulary lints than baseline (includes
    /// applied-breakage: a non-compiling after-span scores as nothing
    /// resolved — the bench's own no-reward-for-breakage rule).
    HealRateRegression { base: usize, trial: usize },
    /// Trial CREATED more lints than baseline — the false-positive /
    /// poisoning axis.
    CreatedRateRegression { base: usize, trial: usize },
}

impl RejectReason {
    /// The one-word audit tag.
    #[must_use]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::ParseUnsafe => "parse_unsafe",
            Self::WithdrawnAbsorbing => "withdrawn_absorbing",
            Self::ConflictingMatchKey => "conflicting_match_key",
            Self::HealRateRegression { .. } => "heal_regression",
            Self::CreatedRateRegression { .. } => "created_regression",
        }
    }
}

/// The regression bar: `None` = not-worse on BOTH axes. Canonical INTEGER
/// comparison — same fixtures ⇒ same denominators ⇒ bit-deterministic.
/// A trial with `n_ran == 0` is judged a heal regression (fail closed: no
/// evidence = no promotion).
#[must_use]
pub fn not_worse(base: &Totals, trial: &Totals) -> Option<RejectReason> {
    if trial.n_ran == 0 {
        return Some(RejectReason::HealRateRegression {
            base: base.resolved,
            trial: 0,
        });
    }
    if trial.resolved < base.resolved {
        return Some(RejectReason::HealRateRegression {
            base: base.resolved,
            trial: trial.resolved,
        });
    }
    if trial.created > base.created {
        return Some(RejectReason::CreatedRateRegression {
            base: base.created,
            trial: trial.created,
        });
    }
    None
}

// ── §4 the gate + the post-hoc sweep ──────────────────────────────────────

/// One candidate's gate verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Cleared the bar + promoted into the local overlay (Certified).
    /// `fired=false` companions mean "promoted on no-regression evidence
    /// only" — the frozen fixtures did not exercise the rule.
    Promoted,
    Rejected(RejectReason),
}

/// One candidate's gate verdict row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictRow {
    pub rule_id: String,
    pub verdict: Verdict,
    /// Did the candidate's `buggy_code` textually occur in any fixture?
    /// `false` = promoted on no-regression evidence ONLY (the frozen
    /// fixtures cover sweep-history shapes — reported, honestly).
    pub fired: bool,
}

/// The gate's full report (deterministic render; no side effects).
#[derive(Debug, Clone, Default)]
pub struct GateReport {
    pub base: Totals,
    pub batch: Option<Totals>,
    pub verdicts: Vec<VerdictRow>,
}

impl GateReport {
    /// Deterministic one-line-per-verdict render for the CLI.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "promotion gate: base resolved={} created={} ({} fixture(s) scored)",
            self.base.resolved, self.base.created, self.base.n_ran
        );
        if let Some(b) = &self.batch {
            let _ = writeln!(
                out,
                "  batch trial: resolved={} created={}",
                b.resolved, b.created
            );
        }
        for v in &self.verdicts {
            match &v.verdict {
                Verdict::Promoted => {
                    let note = if v.fired {
                        String::new()
                    } else {
                        " (fired=false — not exercised by the frozen fixtures)".to_string()
                    };
                    let _ = writeln!(out, "  PROMOTE  {}{note}", v.rule_id);
                }
                Verdict::Rejected(r) => {
                    let _ = writeln!(
                        out,
                        "  REJECT   {} — {} (fired={})",
                        v.rule_id,
                        r.tag(),
                        v.fired
                    );
                }
            }
        }
        out
    }
}

/// The pre-filter pass: parse-safety (defense-in-depth for direct
/// callers), the absorbing blacklist, and CONFLICTING MATCH KEYS (same
/// buggy, different fixed — the shadowing hole: in a batch only the first
/// substitution can fire, the rest would ride the trial undetected).
/// Appends a verdict row for every rejected candidate; returns the triable
/// remainder.
fn pre_filter(
    candidates: &[CandidateRow],
    fixtures: &[BenchFixture],
    registry: &PromotionRegistry,
    verdicts: &mut Vec<VerdictRow>,
) -> Vec<CandidateRow> {
    let mut key_conflicts: BTreeSet<String> = BTreeSet::new();
    {
        let mut by_key: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for row in candidates {
            by_key
                .entry(row.buggy_code.as_str())
                .or_default()
                .insert(row.fixed_code.as_str());
        }
        for (key, fixeds) in &by_key {
            if fixeds.len() > 1 {
                let n = fixeds.len();
                eprintln!(
                    "promotion gate: CONFLICT — {n} different fixed_code values share one match key ({:<40}…)",
                    key.chars().take(40).collect::<String>()
                );
                key_conflicts.insert((*key).to_string());
            }
        }
    }
    let mut triable = Vec::new();
    for row in candidates {
        let fired = fixtures
            .iter()
            .any(|f| f.code.contains(row.buggy_code.as_str()));
        let reject = |reason, verdicts: &mut Vec<VerdictRow>| {
            verdicts.push(VerdictRow {
                rule_id: row.rule_id.clone(),
                verdict: Verdict::Rejected(reason),
                fired,
            });
        };
        if !parses_as_fragment(&row.fixed_code) {
            reject(RejectReason::ParseUnsafe, verdicts);
            continue;
        }
        if registry.is_withdrawn(row) {
            reject(RejectReason::WithdrawnAbsorbing, verdicts);
            continue;
        }
        if key_conflicts.contains(row.buggy_code.as_str()) {
            reject(RejectReason::ConflictingMatchKey, verdicts);
            continue;
        }
        triable.push(row.clone());
    }
    triable
}

/// Run the promotion gate over `candidates` (already schema-validated by
/// [`parse_candidates`]; re-checked here for direct callers). Mutates
/// `registry` exactly by promoting the passers. NEVER touches the score
/// history. Batch-first; per-candidate attribution only on batch failure
/// (the standard promotion-gate shape — the happy path costs two benches).
pub fn run_promotion_gate(
    oracle: &ClippyOracle,
    fixtures: &[BenchFixture],
    candidates: &[CandidateRow],
    registry: &mut PromotionRegistry,
) -> GateReport {
    let mut report = GateReport::default();
    let triable = pre_filter(candidates, fixtures, registry, &mut report.verdicts);

    let base_score = run_bench(oracle, fixtures);
    let base = totals(&base_score);
    report.base = base;

    if triable.is_empty() {
        return report;
    }

    // Fail closed: an oracle that scored nothing cannot clear anyone.
    if base.n_ran == 0 {
        for row in triable {
            report.verdicts.push(VerdictRow {
                rule_id: row.rule_id.clone(),
                verdict: Verdict::Rejected(RejectReason::HealRateRegression { base: 0, trial: 0 }),
                fired: fixtures
                    .iter()
                    .any(|f| f.code.contains(row.buggy_code.as_str())),
            });
        }
        return report;
    }

    let lanes: Vec<(&str, &str)> = triable.iter().map(CandidateRow::lane).collect();
    let batch_score = run_bench_with(oracle, fixtures, &lanes);
    let batch = totals(&batch_score);
    report.batch = Some(batch);

    if not_worse(&base, &batch).is_none() {
        // Batch passes ⇒ every candidate cleared the bar.
        for row in triable {
            let fired = fixtures
                .iter()
                .any(|f| f.code.contains(row.buggy_code.as_str()));
            if matches!(
                registry.promote(row.clone()),
                Err(PromoteError::DuplicateRuleId)
            ) {
                report.verdicts.push(VerdictRow {
                    rule_id: row.rule_id.clone(),
                    verdict: Verdict::Rejected(RejectReason::WithdrawnAbsorbing), // unreachable via parse (dedup); defensive
                    fired,
                });
                continue;
            }
            report.verdicts.push(VerdictRow {
                rule_id: row.rule_id.clone(),
                verdict: Verdict::Promoted,
                fired,
            });
        }
        return report;
    }

    // Batch failed ⇒ per-candidate attribution (base already measured).
    for row in triable {
        let fired = fixtures
            .iter()
            .any(|f| f.code.contains(row.buggy_code.as_str()));
        let trial = totals(&run_bench_with(oracle, fixtures, &[row.lane()]));
        let verdict = match not_worse(&base, &trial) {
            None => {
                let _ = registry.promote(row.clone());
                Verdict::Promoted
            }
            Some(reason) => Verdict::Rejected(reason),
        };
        report.verdicts.push(VerdictRow {
            rule_id: row.rule_id.clone(),
            verdict,
            fired,
        });
    }
    report
}

/// The post-hoc sweep report.
#[derive(Debug, Clone, Default)]
pub struct SweepReport {
    pub ran: bool,
    pub base: Totals,
    pub overlay: Totals,
    /// `rule_id`s withdrawn this sweep (absorbing — they can never re-promote).
    pub withdrawn: Vec<String>,
}

impl SweepReport {
    /// Deterministic render for the CLI.
    #[must_use]
    pub fn render(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        if !self.ran {
            let _ = writeln!(
                out,
                "post-hoc sweep: no live promoted rules — nothing to verify"
            );
            return out;
        }
        let _ = writeln!(
            out,
            "post-hoc sweep: base resolved={} created={} | overlay resolved={} created={}",
            self.base.resolved, self.base.created, self.overlay.resolved, self.overlay.created
        );
        if self.withdrawn.is_empty() {
            let _ = writeln!(out, "  clean — no withdrawals");
        } else {
            for id in &self.withdrawn {
                let _ = writeln!(out, "  WITHDRAWN {id} (absorbing — can never re-promote)");
            }
        }
        out
    }
}

/// Re-verify the live overlay against the current bench: base vs
/// base+overlay. A regression attributes per-candidate (bisect-free —
/// each promoted rule is trialed alone) and every regressor is WITHDRAWN
/// (absorbing). Never touches the score history.
pub fn run_posthoc_sweep(
    oracle: &ClippyOracle,
    fixtures: &[BenchFixture],
    registry: &mut PromotionRegistry,
) -> SweepReport {
    if registry.promoted.is_empty() {
        return SweepReport::default();
    }
    let base = totals(&run_bench(oracle, fixtures));
    let lanes: Vec<(&str, &str)> = registry.promoted.values().map(|e| e.row.lane()).collect();
    let overlay = totals(&run_bench_with(oracle, fixtures, &lanes));
    let mut report = SweepReport {
        ran: true,
        base,
        overlay,
        withdrawn: Vec::new(),
    };
    if not_worse(&base, &overlay).is_none() {
        return report;
    }
    // Attribute: trial each promoted rule alone; withdraw the regressors.
    let ids: Vec<String> = registry.promoted.keys().cloned().collect();
    for id in ids {
        let Some(entry) = registry.promoted.get(&id) else {
            continue;
        };
        let lane = entry.row.lane();
        let trial = totals(&run_bench_with(oracle, fixtures, &[lane]));
        if not_worse(&base, &trial).is_some() {
            registry.withdraw(&id);
            report.withdrawn.push(id);
        }
    }
    report.withdrawn.sort();
    report
}

// ── §5 audit + overlay compose ────────────────────────────────────────────

/// One append-only audit row (the `score_history` precedent; `ts` makes it
/// a log — VERDICT determinism is unaffected, audit is history).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub ts: String,
    /// `promote` | `reject` | `withdraw`.
    pub kind: String,
    pub rule_id: String,
    pub content_hash_hex: String,
    /// Reject tag / withdrawal cause; empty for promote.
    pub reason: String,
    pub fired: bool,
    pub base_resolved: usize,
    pub trial_resolved: usize,
    pub base_created: usize,
    pub trial_created: usize,
}

/// Append audit rows to the audit NDJSON (creates parents on first write).
///
/// # Errors
///
/// Propagates filesystem errors.
pub fn append_audit(path: &Path, records: &[AuditRecord]) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for r in records {
        let mut line = serde_json::to_string(r).map_err(std::io::Error::other)?;
        line.push('\n');
        f.write_all(line.as_bytes())?;
    }
    Ok(())
}

/// Deterministically merge promoted triples into a domain's base triples
/// (leased or compiled-in): base order first, promoted appended, dedup by
/// `rule_id` with the base winning. Promotion never REPLACES base rules —
/// and never mutates the base (a new Vec is returned).
#[must_use]
pub fn merge_triples(base: &[Triple], promoted: &[Triple]) -> Vec<Triple> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::with_capacity(base.len() + promoted.len());
    for t in base.iter().chain(promoted) {
        if seen.insert(t.0.as_str()) {
            out.push(t.clone());
        }
    }
    out.shrink_to_fit();
    out
}

/// The compiled-in seed corpus as triples (the merge base when no lease
/// covers the domain). Unknown domains → empty (no statement).
#[must_use]
pub fn compiled_in_triples(domain: &str) -> Vec<Triple> {
    fn to_triples<E: HealEntry>(entries: Vec<E>) -> Vec<Triple> {
        entries
            .into_iter()
            .map(|e| {
                (
                    e.rule_id().to_string(),
                    e.buggy_code().to_string(),
                    e.fixed_code().to_string(),
                )
            })
            .collect()
    }
    match domain {
        #[cfg(feature = "clippy_lints")]
        DOMAIN_CLIPPY => to_triples(crate::domains::clippy_lints::ClippyLintDomain::build_corpus()),
        #[cfg(feature = "rust_perf")]
        DOMAIN_PERF => to_triples(crate::domains::rust_perf::RustPerfDomain::build_corpus()),
        _ => Vec::new(),
    }
}

/// The promotion-aware run overlay for the matcher: the leased overlay (if
/// any) with promoted triples MERGED into each covered domain, plus
/// domains covered ONLY by promotions (base = compiled-in for those).
/// Returns `None` when there is neither a lease nor any promoted rule.
///
/// `pin` (`--corpus-epoch`) suppresses the promotion merge — a pinned run
/// reproduces the exact leased/compiled-in state (CI reproducibility).
#[must_use]
pub fn promotion_run_overlay(
    leased: Option<&[(String, Vec<Triple>)]>,
    registry: &PromotionRegistry,
    pin: bool,
) -> Option<Vec<(String, Vec<Triple>)>> {
    let promoted = registry.overlay_triples();
    if pin || promoted.is_empty() {
        return leased.map(<[_]>::to_vec);
    }
    // Split promotions by their declared domain.
    let mut by_domain: BTreeMap<String, Vec<Triple>> = BTreeMap::new();
    for t in promoted {
        let domain = registry
            .promoted
            .get(&t.0)
            .map_or_else(|| DOMAIN_CLIPPY.to_string(), |e| e.row.domain.clone());
        by_domain.entry(domain).or_default().push(t);
    }
    let mut out: Vec<(String, Vec<Triple>)> = Vec::new();
    // Leased domains first (lease order), promotions merged in.
    if let Some(leased) = leased {
        for (domain, base) in leased {
            let extra = by_domain.remove(domain.as_str()).unwrap_or_default();
            out.push((domain.clone(), merge_triples(base, &extra)));
        }
    }
    // Domains covered only by promotions: base = compiled-in.
    for (domain, extra) in by_domain {
        let base = compiled_in_triples(&domain);
        out.push((domain, merge_triples(&base, &extra)));
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn row(rule_id: &str, buggy: &str, fixed: &str) -> CandidateRow {
        CandidateRow {
            schema_version: CANDIDATE_SCHEMA_VERSION,
            rule_id: rule_id.to_string(),
            domain: DOMAIN_CLIPPY.to_string(),
            buggy_code: buggy.to_string(),
            fixed_code: fixed.to_string(),
            provenance: String::new(),
        }
    }

    #[test]
    fn schema_happy_path_and_every_rejection() {
        let good = r#"{"schema_version":1,"rule_id":"r1","domain":"clippy","buggy_code":"return 1;","fixed_code":"1"}"#;
        let rows = parse_candidates(good.as_bytes()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rule_id, "r1");

        // version mismatch
        let bad = r#"{"schema_version":2,"rule_id":"r1","domain":"clippy","buggy_code":"a","fixed_code":"b"}"#;
        assert!(parse_candidates(bad.as_bytes()).is_err());
        // empty file is fine
        assert!(parse_candidates(b"\n\n").unwrap().is_empty());
        // duplicate id
        let dup = format!("{good}\n{good}");
        let err = parse_candidates(dup.as_bytes()).unwrap_err();
        assert!(err.contains("duplicate rule_id"), "{err}");
        // unknown domain
        let kern = r#"{"schema_version":1,"rule_id":"r2","domain":"kernel","buggy_code":"a","fixed_code":"b"}"#;
        let err = parse_candidates(kern.as_bytes()).unwrap_err();
        assert!(err.contains("kernel candidates"), "{err}");
        // empty buggy
        let empty = r#"{"schema_version":1,"rule_id":"r3","domain":"perf","buggy_code":"","fixed_code":"b"}"#;
        assert!(parse_candidates(empty.as_bytes()).is_err());
        // no-op rule
        let noop = r#"{"schema_version":1,"rule_id":"r4","domain":"perf","buggy_code":"x","fixed_code":"x"}"#;
        assert!(parse_candidates(noop.as_bytes()).is_err());
        // non-parsing fixed
        let broken = r#"{"schema_version":1,"rule_id":"r5","domain":"clippy","buggy_code":"return 1;","fixed_code":"fn ("}"#;
        let err = parse_candidates(broken.as_bytes()).unwrap_err();
        assert!(err.contains("ParseUnsafe"), "{err}");
    }

    #[test]
    fn fragment_parse_accepts_items_and_stmts() {
        assert!(parses_as_fragment("fn f() -> u32 { 1 }"));
        assert!(parses_as_fragment("return 1;"));
        assert!(parses_as_fragment("let x = 1; x + 1"));
        assert!(parses_as_fragment("|x| x + 1"));
        assert!(!parses_as_fragment("fn ("));
        assert!(!parses_as_fragment("let ="));
    }

    #[test]
    fn registry_promote_duplicate_and_idempotent() {
        let mut reg = PromotionRegistry::default();
        let r = row("r1", "return 1;", "1");
        reg.promote(r.clone()).unwrap();
        assert_eq!(reg.promoted["r1"].tier, EvidenceTier::Certified);
        // same row again: idempotent
        reg.promote(r.clone()).unwrap();
        // same id, different content: duplicate
        let err = reg.promote(row("r1", "return 2;", "2")).unwrap_err();
        assert_eq!(err, PromoteError::DuplicateRuleId);
        assert_eq!(reg.overlay_triples().len(), 1);
    }

    #[test]
    fn withdrawn_is_absorbing_on_every_return_path() {
        let mut reg = PromotionRegistry::default();
        let r = row("r1", "return 1;", "1");
        reg.promote(r.clone()).unwrap();
        assert!(reg.withdraw("r1"));
        assert!(reg.overlay_triples().is_empty());

        // same content, same id
        assert_eq!(
            reg.promote(r.clone()).unwrap_err(),
            PromoteError::WithdrawnAbsorbing
        );
        // same content, NEW id
        let r2 = CandidateRow {
            rule_id: "r2".into(),
            ..r.clone()
        };
        assert_eq!(
            reg.promote(r2).unwrap_err(),
            PromoteError::WithdrawnAbsorbing
        );
        // same id, NEW content
        let r3 = row("r1", "return 3;", "3");
        assert_eq!(
            reg.promote(r3).unwrap_err(),
            PromoteError::WithdrawnAbsorbing
        );
        // withdraw of unknown id is false
        assert!(!reg.withdraw("nope"));
    }

    #[test]
    fn not_worse_is_canonical_integer_comparison() {
        let base = Totals {
            resolved: 10,
            created: 2,
            vocab_before: 12,
            n_ran: 5,
        };
        let same = Totals {
            resolved: 10,
            created: 2,
            vocab_before: 12,
            n_ran: 5,
        };
        let better = Totals {
            resolved: 11,
            created: 1,
            vocab_before: 12,
            n_ran: 5,
        };
        let heal_worse = Totals {
            resolved: 9,
            created: 2,
            vocab_before: 12,
            n_ran: 5,
        };
        let created_worse = Totals {
            resolved: 10,
            created: 3,
            vocab_before: 12,
            n_ran: 5,
        };
        let no_evidence = Totals {
            resolved: 99,
            created: 0,
            vocab_before: 12,
            n_ran: 0,
        };
        assert!(not_worse(&base, &same).is_none());
        assert!(not_worse(&base, &better).is_none());
        assert!(matches!(
            not_worse(&base, &heal_worse),
            Some(RejectReason::HealRateRegression { base: 10, trial: 9 })
        ));
        assert!(matches!(
            not_worse(&base, &created_worse),
            Some(RejectReason::CreatedRateRegression { base: 2, trial: 3 })
        ));
        // fail closed: no evidence = no promotion
        assert!(matches!(
            not_worse(&base, &no_evidence),
            Some(RejectReason::HealRateRegression { .. })
        ));
    }

    #[test]
    fn merge_triples_base_wins_and_is_deterministic() {
        let base = vec![
            ("a".to_string(), "ba".to_string(), "fa".to_string()),
            ("b".to_string(), "bb".to_string(), "fb".to_string()),
        ];
        let promoted = vec![
            ("b".to_string(), "BROKEN".to_string(), "x".to_string()), // dup id — base wins
            ("c".to_string(), "bc".to_string(), "fc".to_string()),
        ];
        let merged = merge_triples(&base, &promoted);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[1].1, "bb"); // base's version
        assert_eq!(merged[2].0, "c");
        assert_eq!(merge_triples(&base, &promoted), merged); // deterministic
                                                             // base untouched
        assert_eq!(base.len(), 2);
    }

    #[test]
    fn registry_and_audit_roundtrip() {
        // (tmpdir-free variant: save/load under the std temp root)
        let dir = std::env::temp_dir().join(format!("kat_promo_rt_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(REGISTRY_FILE);
        let mut reg = PromotionRegistry::default();
        reg.promote(row("r1", "return 1;", "1")).unwrap();
        reg.save(&path).unwrap();
        let loaded = PromotionRegistry::load(&path).unwrap();
        assert_eq!(loaded, reg);
        // save is deterministic (canonical bytes)
        let p2 = dir.join("again.json");
        reg.save(&p2).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), std::fs::read(&p2).unwrap());
        // corrupt registry refused
        std::fs::write(&path, b"{not json").unwrap();
        assert!(PromotionRegistry::load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn promotion_never_mutates_the_frozen_base_corpus() {
        // The compiled-in clippy corpus bytes before/after promote+withdraw
        // cycles must be identical — the overlay is ADDITIVE data only.
        let before = crate::domains::clippy_lints::ClippyLintDomain::build_corpus();
        let frozen_before = serde_json::to_vec(&before).unwrap();

        let mut reg = PromotionRegistry::default();
        for i in 0..3 {
            let r = row(&format!("net-{i}"), "return 1;", "1");
            reg.promote(r).unwrap();
        }
        assert!(!reg.overlay_triples().is_empty());
        reg.withdraw("net-1");
        assert_eq!(reg.overlay_triples().len(), 2);

        let after = crate::domains::clippy_lints::ClippyLintDomain::build_corpus();
        let frozen_after = serde_json::to_vec(&after).unwrap();
        assert_eq!(
            frozen_before, frozen_after,
            "the frozen base corpus must be byte-identical"
        );
    }

    #[test]
    fn promotion_run_overlay_merges_and_respects_pin() {
        let mut reg = PromotionRegistry::default();
        let mut net = row("net-1", "return 1;", "1");
        net.domain = DOMAIN_PERF.to_string();
        reg.promote(net).unwrap();

        // No lease: perf covered only by promotion → compiled-in + promoted.
        let overlay = promotion_run_overlay(None, &reg, false).unwrap();
        assert_eq!(overlay.len(), 1);
        assert_eq!(overlay[0].0, DOMAIN_PERF);
        assert!(overlay[0].1.iter().any(|t| t.0 == "net-1"));
        // The compiled-in seed survived the merge (base wins).
        assert!(overlay[0].1.len() > 1);

        // Pin suppresses the merge entirely.
        assert!(promotion_run_overlay(None, &reg, true).is_none());

        // Lease-covered domain: promotions merge INTO the leased triples.
        let leased = vec![(
            DOMAIN_CLIPPY.to_string(),
            vec![("seed".to_string(), "b".to_string(), "f".to_string())],
        )];
        let mut clippy_net = row("net-clippy", "return 2;", "2");
        clippy_net.domain = DOMAIN_CLIPPY.to_string();
        reg.promote(clippy_net).unwrap();
        let overlay = promotion_run_overlay(Some(&leased), &reg, false).unwrap();
        assert_eq!(overlay.len(), 2); // clippy (leased+net-clippy) + perf (compiled-in+net-1)
        let clippy = &overlay.iter().find(|(d, _)| d == DOMAIN_CLIPPY).unwrap().1;
        assert!(clippy.iter().any(|t| t.0 == "seed"));
        assert!(clippy.iter().any(|t| t.0 == "net-clippy"));

        // Empty registry + no lease → None (the plain non-promotion path).
        let empty = PromotionRegistry::default();
        assert!(promotion_run_overlay(None, &empty, false).is_none());
    }

    #[test]
    fn conflicting_match_keys_are_rejected_wholesale_without_a_bench() {
        // Two candidates share one match key with different fixeds: only the
        // first can fire in a batch — the rest would ride the pass
        // undetected. Both are rejected, deterministically, pre-bench.
        let mut reg = PromotionRegistry::default();
        let fixtures = Vec::new(); // NO fixtures — the conflict check must not need a bench
        let a = row("a", "return 1;", "1");
        let b = row("b", "return 1;", "2");
        let report = run_promotion_gate(&ClippyOracle::new(), &fixtures, &[a, b], &mut reg);
        assert_eq!(report.verdicts.len(), 2);
        for v in &report.verdicts {
            assert_eq!(
                v.verdict,
                Verdict::Rejected(RejectReason::ConflictingMatchKey),
                "{v:?}"
            );
        }
        assert!(reg.promoted.is_empty());
        // Identical key + identical fixed is a parse-level duplicate, not a
        // conflict — parse_candidates rejects it before the gate.
    }

    #[test]
    fn candidate_lane_determinism_x3() {
        let src = "pub fn scale_ticks(ticks: u32) -> u32 {\n    return ticks * 1;\n}\n";
        let cands = [("return ticks * 1;", "ticks")];
        let mut runs: Vec<(String, usize, usize)> = Vec::new();
        for _ in 0..3 {
            runs.push(crate::score_bench::heal_fixpoint_with(src, &cands));
        }
        assert_eq!(runs[0], runs[1]);
        assert_eq!(runs[1], runs[2]);
        assert!(runs[0].0.contains("ticks"));
        assert!(!runs[0].0.contains("return"));
    }
}
